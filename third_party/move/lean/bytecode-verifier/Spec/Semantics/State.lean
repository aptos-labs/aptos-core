-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Semantics.Program

/-!
# Machine state and outcomes

TRUSTED. The state of the reference semantics and the complete classification
of step outcomes (plan §5.2):

- `running`: a normal step;
- `returned`: the top-level function returned; payload effects commit;
- `failed`: a language-defined failure (a Move abort, or a check of the R
  catalogue); payload effects roll back;
- `wrong`: the bytecode broke the language's rules. This is the only bad
  outcome.

`limit` is not a step outcome: any execution may stop with a limit at any
state (the stopping relation), which `Run` makes explicit.

Each frame owns its operand-stack segment: a callee can neither see nor pop
its caller's values. A VM with one shared stack refines this on verified code.

**Global storage** maps an address and a struct type to a value (plan §5.2,
"State"; book, `global-storage.md`: "`resources: Map<(address, ResourceType),
ResourceValue>`"). It is an input of the transaction (`initialState`), and
the payload's effects on it are what `returned` commits; `failed` and limits
roll them back, so they expose no storage.

**Held values.** The transaction holds some values for the whole execution
and lends them to the entry function by reference: the signers of `&signer`
parameters ("Entry functions can accept parameters of the following types:
primitive types, references to a `signer`, ...", book, `functions.md`). They
are the roots `Root.held` of those references (`State.held`).

**Locations and their validity** (plan §5.2, "References"). Frames have
identities, never reused (`Frame.id`). A location stops being valid when what
it designates is gone or may have moved; the state records each such event as
an invalidation (`Invalidation`), and a reference is valid when its location
resolves and no invalidation recorded since the reference was created covers
it (`State.deref?`). The language does not say where a VM keeps a value, so
the invalidations are the conservative ones: a reference survives only what
it survives in every representation.

- The value at a root is removed: a local is moved out (`MoveLoc`), a
  resource is moved out of global storage (`MoveFrom`). Every location under
  the root becomes invalid, for good: assigning the local again or publishing
  the resource again makes new locations, not the old ones.
- A frame returns. Its identity is never used again, so no location in its
  locals resolves ever after.
- The value at a location is changed in place: a write through a reference
  (`WriteRef`), overwriting a local (`StLoc`), or a vector push, pop or swap.
  The location itself stays valid, holding the new value, but every location
  strictly inside the old value becomes invalid: the old value is discarded
  ("A write `*x = v` discards the value previously stored in `x`", book,
  `references.md`; `StLoc`: "If the local contains an old value, then that
  value is dropped", `file_format.rs`), and a vector's elements may move when
  it grows or shrinks. This covers a popped or swapped element and an enum
  value whose variant changes.

Reading through a reference, borrowing, and copying or freezing a reference
invalidate nothing.
-/

namespace MoveBytecode

structure Frame where
  /-- The function instance the frame runs: the definition and its closed type
  arguments ("`new_frame_from_func(func, ty_args, ...)`", `file_format.rs`,
  `Call`). Every token the frame's code names is translated under them. -/
  function : FunctionRef
  /-- The frame instance's identity, which no other frame of the execution
  has (`State.nextFrame`). Locations in its locals name it (`Root.local_`). -/
  id : FrameId
  pc : Nat
  /-- `some v` for an available local, `none` for an unavailable one (never
  assigned, or moved out). -/
  locals : Array (Option Value)
  /-- The frame's operand-stack segment; the head is the top. -/
  stack : List Value
  /-- Whether entering this frame counted its module as active (R3, R4): a
  cross-module call, the transaction's entry, and every closure dispatch,
  "even within one module" (plan §6, R3). -/
  countedActive : Bool
  /-- Whether entering this frame entered the module lock (R4). -/
  lockEntered : Bool
  deriving Repr, Inhabited

/-! ## Global storage -/

/-- A resource in global storage: the value of struct type `type` stored under
`address`. The type includes its type arguments: one address may hold a
`Container<u64>` and a `Container<bool>` ("the ability to index into global
storage via a type parameter chosen at runtime", book, `global-storage.md`,
"Global storage operators with generics"). -/
structure Resource where
  address : Address
  type : StructId
  value : Value
  deriving DecidableEq, Repr, Inhabited

/-- Global storage: at most one resource for each address and type ("each
`address` can store at most one resource value of a given type", book,
`global-storage.md`). Kept as a list; two storages are the same storage when
they hold the same resources (`get?`), in whatever order. -/
abbrev GlobalStorage := List Resource

namespace GlobalStorage

/-- The value of type `t` stored under `a`. -/
def get? (st : GlobalStorage) (a : Address) (t : StructId) : Option Value :=
  (st.find? fun r => r.address == a && r.type == t).map (·.value)

/-- Publish `v` of type `t` under `a`, where nothing of type `t` is. -/
def insert (st : GlobalStorage) (a : Address) (t : StructId) (v : Value) : GlobalStorage :=
  st ++ [⟨a, t, v⟩]

/-- Remove the resource of type `t` under `a`. -/
def remove (st : GlobalStorage) (a : Address) (t : StructId) : GlobalStorage :=
  st.filter fun r => !(r.address == a && r.type == t)

/-- Replace the value of the resource of type `t` under `a`. -/
def update (st : GlobalStorage) (a : Address) (t : StructId) (v : Value) : GlobalStorage :=
  st.map fun r => if r.address == a && r.type == t then { r with value := v } else r

/-- Well-typed global storage (plan §7, D: "global values conform to their
declared types"): one resource per address and type, each value conforming to
its type, and having `key` all the way down. Only a type with `key` can be
stored ("in order for a type to be used with `move_to`, ... the type must
have the `key` ability"), and "if a value has `key`, all values contained
inside of that value have `store`" (book, `generics-and-abilities.md`). -/
def wellTyped (P : Program) (st : GlobalStorage) : Bool :=
  decide (st.map fun r => (r.address, r.type)).Nodup &&
    st.all fun r => r.value.conforms P (.struct r.type) && (r.value.abilities P).key

end GlobalStorage

/-! ## Invalidations -/

/-- An event that ends the validity of locations (module docstring). -/
inductive Invalidation where
  /-- The value at `root` was removed: moved out of a local or out of global
  storage. Every location under `root` becomes invalid. -/
  | removed (root : Root)
  /-- The value at `loc` was changed in place. Every location strictly inside
  it becomes invalid; `loc` itself stays valid. -/
  | mutated (loc : Location)
  deriving DecidableEq, Repr, Inhabited

/-- Whether invalidation `e` ends the validity of location `l`. -/
def Invalidation.covers : Invalidation → Location → Bool
  | .removed root, l => l.root == root
  | .mutated m, l => l.root == m.root && m.path.isPrefixOf l.path && m.path != l.path

structure State where
  /-- The call stack; the head is the current frame. -/
  frames : List Frame
  /-- Active modules with their activation counts (R3, R4). -/
  active : List (ModuleId × Nat)
  /-- Nesting depth of the module lock (R4). -/
  lockDepth : Nat
  /-- Global storage. -/
  storage : GlobalStorage
  /-- The identity of the next frame to be created: frames created so far
  have smaller identities. -/
  nextFrame : FrameId
  /-- The invalidations so far, the latest first. -/
  invalidations : List Invalidation
  /-- The values the transaction holds and lends by reference (module
  docstring, "Held values"). -/
  held : List Value
  deriving Repr, Inhabited

namespace State

def activeCount (s : State) (id : ModuleId) : Nat :=
  match s.active.find? (fun e => e.1 == id) with
  | some e => e.2
  | none => 0

def incActive (s : State) (id : ModuleId) : State :=
  let n := s.activeCount id
  { s with active := (id, n + 1) :: s.active.filter (fun e => e.1 != id) }

def decActive (s : State) (id : ModuleId) : State :=
  let n := s.activeCount id
  let rest := s.active.filter (fun e => e.1 != id)
  { s with active := if n ≤ 1 then rest else (id, n - 1) :: rest }

/-! ### Locations in a state -/

/-- The frame with identity `f`, if it has not returned. -/
def frame? (s : State) (f : FrameId) : Option Frame :=
  s.frames.find? (·.id == f)

/-- The value stored at a root: an available local of a frame that has not
returned, a resource in global storage, or a held value. -/
def root? (s : State) : Root → Option Value
  | .local_ f i => do (← (← s.frame? f).locals[i]?)
  | .global a t => s.storage.get? a t
  | .held k => s.held[k]?

/-- The value at location `l`, if the location resolves. -/
def read? (s : State) (l : Location) : Option Value := do
  (← s.root? l.root).get? l.path

/-- Replace the value at an existing root. -/
def setRoot (s : State) (root : Root) (v : Value) : State :=
  match root with
  | .local_ f i =>
    { s with frames := s.frames.map fun fr =>
        if fr.id == f then { fr with locals := fr.locals.setIfInBounds i (some v) } else fr }
  | .global a t => { s with storage := s.storage.update a t v }
  | .held k => { s with held := s.held.set k v }

/-- Replace the value at location `l`, if it resolves. -/
def write? (s : State) (l : Location) (v : Value) : Option State := do
  pure (s.setRoot l.root (← (← s.root? l.root).set? l.path v))

/-- Record an invalidation. -/
def invalidate (s : State) (e : Invalidation) : State :=
  { s with invalidations := e :: s.invalidations }

/-- The invalidations recorded after the first `since`. -/
def invalidatedSince (s : State) (since : Nat) : List Invalidation :=
  s.invalidations.take (s.invalidations.length - since)

/-- The value a reference to `l`, created when `since` invalidations had been
recorded, designates: `none` when the reference is not valid, because `l` does
not resolve or an invalidation recorded since covers it. -/
def deref? (s : State) (l : Location) (since : Nat) : Option Value :=
  if (s.invalidatedSince since).any (·.covers l) then none else s.read? l

/-- What a native sees of its arguments `args` in state `s`: each value
argument as it is, and each reference argument replaced by the value it
designates; `none` if a reference argument is not valid. A native "sees what
its reference arguments designate, not the references themselves"
(language-owner ruling, 2026-11-03). For a native that takes values only, it is
`args`. -/
def nativeView (s : State) (args : List Value) : Option (List Value) :=
  args.mapM fun v => match v with
    | .ref r => s.deref? r.location r.since
    | v => some v

end State

/-- Language-defined failures of verified code: a Move abort, or a check of the
R catalogue (plan §6). The static verifier cannot exclude these, so they are
legitimate outcomes, not violations. On any failure, and on a limit, the
transaction payload's effects on global storage roll back; only `returned`
commits them.

R7, bounded layout and specialization, is not a failure: it is the stopping
relation, which may end any execution with a limit (`Spec.Semantics.Run`). -/
inductive Failure where
  /-- `Abort` with its code. -/
  | abort (code : Nat)
  /-- R6: overflow, division by zero, a shift by at least the width, a cast out
  of range. -/
  | arithmetic
  /-- R6: a vector index out of range (`VecImmBorrow`, `VecMutBorrow`,
  `VecSwap`: "Abort the execution if the index is out of bounds",
  `file_format.rs`). -/
  | vectorIndex
  /-- R1: unpacking an enum value, or borrowing a variant field, whose runtime
  variant differs. -/
  | variantMismatch
  /-- R2: `VecUnpack n` on a vector whose length is not `n`. -/
  | vectorLength
  /-- R3: a global-storage operation on a type of a module that is active more
  than once (the resource lock on reentrancy), including through a closure
  dispatched within the module: "dispatching a function value to a concrete
  function in the same module is also considered to be reentrancy" (book,
  `functions.md`, "Reentrancy Check"). -/
  | resourceLocked
  /-- R4: re-entering an active module while the module lock is held, by a
  cross-module call or by a closure dispatch. -/
  | moduleLocked
  /-- R5: `MoveFrom` or `BorrowGlobal` of a resource that does not exist. -/
  | missingResource
  /-- R5: `MoveTo` of a resource that already exists. -/
  | resourceExists
  /-- `AbortMsg` with its code and its message. The message is valid UTF-8 of
  at most `maxAbortMessageLength` bytes (book, `abort-and-assert.md`, "Abort
  messages"). -/
  | abortMessage (code : Nat) (message : List UInt8)
  /-- `AbortMsg` with a message that is not valid UTF-8, or is longer than
  `maxAbortMessageLength` bytes: "the transaction fails with a VM error
  instead of the user-supplied message" (book, "Abort messages";
  `INVALID_ABORT_MESSAGE`, `ABORT_MESSAGE_LIMIT_EXCEEDED`). -/
  | invalidAbortMessage
  /-- R6: `VecPopBack` on an empty vector: "Aborts if the vector is empty"
  (`file_format.rs`); `pop_back` aborts "If `self` is empty" (book,
  `vector.md`). -/
  | emptyVector
  deriving DecidableEq, Repr, Inhabited

/-- The longest abort message, in bytes: "The message must be at most 1024
bytes long" (book, `abort-and-assert.md`, "Abort messages"). -/
def maxAbortMessageLength : Nat := 1024

/-- Why a step went wrong: each is a violation of the language's rules. -/
inductive WrongReason where
  | noFrame
  | badFunctionRef
  | nativeFrame
  | fellOffEnd
  | invalidJump
  | stackUnderflow
  | stackNotEmptyAtReturn
  | typeMismatch
  | localOutOfRange
  | unavailableLocal
  | copyWithoutAbility
  | dropWithoutAbility
  | badIndex
  | badConstant
  /-- An instruction names an entity of the wrong kind for its form (L9):
  `Pack` or `Unpack` of an enum or a native struct, a vector instruction whose
  signature is not a single type, a non-generic form naming a generic entity,
  or a generic form naming a non-generic one or supplying another number of
  type arguments than the entity has type parameters. -/
  | badOperand
  | unresolvedFunction
  /-- A struct or enum type that does not resolve to a unique definition. -/
  | unresolvedStruct
  /-- The caller's function handle declares other types than the callee's
  definition, or another number of type parameters. -/
  | linkMismatch
  | visibility
  | nativeContract
  /-- An instruction or type outside the supported subset. The `Supported`
  premise of every theorem rules it out. -/
  | unsupported (what : String)
  /-- An access through a reference whose location is not valid (plan §5.2:
  "Using an invalid location is `wrong`"): the local was moved out or its
  frame returned, the resource was moved out of global storage, or a value
  enclosing the location was changed in place (`State`'s module docstring). -/
  | invalidReference
  /-- An instruction that writes through a reference, or borrows mutably, or
  freezes, given an immutable reference: "Only mutable references can be
  written" (book, `references.md`); the mutable borrows and `FreezeRef`
  require `&mut` (`file_format.rs`). -/
  | immutableReference
  /-- A global storage operation at a type without `key` ("in order for a type
  to be used with `move_to`, `borrow_global`, `move_from`, etc., the type must
  have the `key` ability", book, `generics-and-abilities.md`), or `MoveTo` of a
  value without `key` all the way down: some value inside lacks `store` ("if a
  value has `key`, all values contained inside of that value have `store`"). -/
  | globalWithoutKey
  /-- An instruction instantiates a generic function, struct or enum with type
  arguments that are not types of the program satisfying the type parameters'
  constraints (`RuntimeType.argumentsWellFormed`): "constraints are checked at
  call sites", and a type argument without an ability its constraint requires
  is an error (book, `generics-and-abilities.md`, "Verifying Constraints"). -/
  | constraintViolation
  /-- `PackClosure` captures a parameter of reference type: "type ti [of a
  captured parameter] is not a reference" (`file_format.rs`, `PackClosure`);
  "it is not possible to _capture_ reference values" (book, `functions.md`,
  "Lambda Expressions and Closures"). -/
  | capturedReference
  /-- A native is called with a mutable reference argument whose location
  overlaps that of another reference argument (`Location.overlaps`): "Move
  prevents from having two mutable references to the same value, so `left` and
  `right` references are always distinct" (`mem.move`, `swap`; likewise
  `vector.move`, `move_range`), and Move is "strict in ensuring unique
  ownership of mutable references before writes" (book, `references.md`,
  "Ownership"). A native's writes through its mutable arguments are defined
  only on places that do not overlap. The other reference argument may be
  immutable (language-owner ruling, 2026-11-04). -/
  | overlappingReferences
  deriving DecidableEq, Repr, Inhabited

inductive Outcome where
  | running (s : State)
  /-- The top-level function returned `values`; the payload's effects commit,
  leaving global storage as `storage`. -/
  | returned (values : List Value) (storage : GlobalStorage)
  | failed (f : Failure)
  | wrong (r : WrongReason)
  deriving Repr, Inhabited

/-- The result of the deterministic core step. A call to a native function
suspends with the state after popping its arguments `args`, and what the native
sees of them, `views` (`State.nativeView`: a reference argument as the value it
designates); the native relation supplies the result (see `Run`). -/
inductive StepResult where
  | done (o : Outcome)
  | callNative (s : State) (f : FunctionRef) (args views : List Value)
  deriving Repr, Inhabited

/-- A place a native returns a reference to: the path `path` inside the value
that its argument `arg`, a reference, designates. -/
structure NativePlace where
  arg : Nat
  path : List PathStep
  deriving DecidableEq, Repr, Inhabited

/-- The result of a native function. A native that returns gives:

- `values`: its results that are values, in order;
- `writes`: for a native of the update shape, the new values of the places its
  mutable reference arguments designate, one for each, in argument order;
  otherwise none;
- `places`: for a native of the borrow shape, for each of its results that is
  a reference, in order, the place it designates, inside what one of its
  reference arguments designates; otherwise none.

The shapes are the language-owner rulings of 2026-11-03
(`Spec.Semantics.ModeledNatives`); a native that takes and returns values only
returns `values` alone. -/
inductive NativeResult where
  | returned (values writes : List Value) (places : List NativePlace)
  | failed (f : Failure)
  deriving Repr, Inhabited

end MoveBytecode
