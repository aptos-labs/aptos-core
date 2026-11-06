-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Semantics.Step

/-!
# Executions

TRUSTED. The semantics has two sources of nondeterminism beyond the
deterministic core `step` (plan §5.2):

- **natives**, given as a relation: a native call may return any result the
  relation allows; native contracts (`Spec.Properties.Safety`) constrain it;
- **stopping**: an execution may stop with a limit at any state. Statements
  quantify over reachable states, so a stopped execution is simply a prefix;
  the executable runner models it with fuel.

**R7** (plan §6): "layout construction, value nesting and type specialization
are bounded and stop with `limit` instead of misbehaving". The semantics
builds no layouts, its values are finite trees of any depth, and the type
arguments of a frame are closed types of any size (`Spec.Semantics.Step`,
"R7, type specialization"): cyclic module dependencies (plan §4.3) admit
recursive types and generic calls whose type arguments grow without end. A VM
bounds all three, and stopping at its bound is a step of the stopping
relation, which every state has. So R7 is not a check of the semantics but an
obligation on a VM: when it cannot build a type or a value, it must stop with
a limit, never continue differently.

**Natives** receive their type arguments: a native call is a call of a
function instance (`FunctionRef.typeArgs`), as "`call_native(func.name,
ty_args, args)`" (`file_format.rs`, `Call`), and its results conform to its
return types at that instance.

**Natives with references** (language-owner rulings, 2026-11-03;
`Spec.Semantics.ModeledNatives`). A native of the read, update or borrow shape
is a relation too, which sees what its reference arguments designate
(`State.nativeView`), never the references. Its result is checked against its
shape (`Program.nativeResultFits`), defensively, as a value native's results
are checked against its return types: a result outside the shape, a value or a
written-back value that does not conform, or a returned place that does not
exist, has the wrong type, or is more mutable than its argument, is `wrong
.nativeContract`. The semantics then writes the new values back through the
mutable reference arguments (`writeBack`), each place changing in place
(`Invalidation.mutated`, as for `WriteRef`), and makes a new reference for each
returned place (`placeReferences`), as a borrow does.

**Table natives** (language-owner rulings, 2026-11-04). `add_box` and
`remove_box` write back the whole table, as any update-shape native writes
back what its mutable reference designates: every reference into the table,
to an entry or to its field, becomes invalid, as a vector's elements do when
it grows or shrinks. `borrow_box` and `borrow_box_mut` return the place of an
entry (`PathStep.tableEntry`), whose declared type is the table's entry type
(`RuntimeType.placeType?`).
-/

namespace MoveBytecode

/-- The declared type of the place at `path` inside a value of declared type
`t`: a struct field's type at the struct's instance, an enum variant field's,
a vector's element type, a signer's address, or a table's entry type, `Box<V>`
in a `Table<K, V>` (`tableEntryType`). -/
def RuntimeType.placeType? (P : Program) (t : RuntimeType) : List PathStep → Option RuntimeType
  | [] => some t
  | step :: path =>
    let inner : Option RuntimeType := match t, step with
      | .struct s, .field i => (P.fieldTypes s).bind (·[i]?)
      | .struct s, .variantField tag i => (P.variantFieldTypes s tag).bind (·[i]?)
      | .vector elem, .element _ => some elem
      | .signer, .signerAddress => some .address
      | .struct s, .tableEntry _ =>
        match s.isTable, s.typeArgs with
        | true, [_, v] => some (tableEntryType v)
        | _, _ => none
      | _, _ => none
    inner.bind (RuntimeType.placeType? P · path)

/-- The targets of the mutable reference types among `tys`, in order: the
declared types of the places an update-shape native writes back. -/
def RuntimeType.mutableTargets (tys : List RuntimeType) : List RuntimeType :=
  tys.filterMap fun t => match t with
    | .ref true target => some target
    | _ => none

/-- A place returned by a borrow-shape native with parameter types `ptys`,
seen as `views`, for a result of type `rty`, fits (language-owner ruling,
2026-11-03: "returns references, each to a place inside what one of its
reference parameters designates, never more mutable than that parameter"):
the result is a reference, the place names a reference parameter at least as
mutable, the path exists in the value that argument designates, and the
declared type there is the result's target, or for an immutable result one
assignable to it (`&` is covariant, `RuntimeType.assignable`). -/
def nativePlaceFits (P : Program) (ptys : List RuntimeType) (views : List Value)
    (rty : RuntimeType) (place : NativePlace) : Bool :=
  match rty, ptys[place.arg]?, views[place.arg]? with
  | .ref mutable target, some (.ref argMutable argTarget), some view =>
    (!mutable || argMutable) && (view.get? place.path).isSome &&
      match argTarget.placeType? P place.path with
      | some t => RuntimeType.assignable (.ref mutable t) (.ref mutable target)
      | none => false
  | _, _, _ => false

/-- **The result of a native call fits its shape** (language-owner rulings,
2026-11-03). Native `f`, with return types `tys`, saw its arguments as `views`
and returned `values`, `writes` and `places` (`NativeResult.returned`):

- update shape: the values conform to the return types; the writes conform,
  one for one, to the targets of its mutable reference parameters; no places;
- borrow shape: no writes; the values conform to the return types that are not
  references, and each reference result has a place that fits
  (`nativePlaceFits`);
- read shape, and a native that takes and returns values only: the values
  conform to the return types; no writes and no places. -/
def Program.nativeResultFits (P : Program) (f : FunctionRef) (tys : List RuntimeType)
    (views vals writes : List Value) (places : List NativePlace) : Bool :=
  match P.nativeShape? f with
  | some .update =>
    match P.paramTypes f with
    | some ptys =>
      conformsAll P vals tys && conformsAll P writes (RuntimeType.mutableTargets ptys) &&
        places.isEmpty
    | none => false
  | some .borrow =>
    match P.paramTypes f with
    | some ptys =>
      let refTys := tys.filter (!·.isValue)
      writes.isEmpty && conformsAll P vals (tys.filter (·.isValue)) &&
        places.length == refTys.length &&
        (refTys.zip places).all fun (t, p) => nativePlaceFits P ptys views t p
    | none => false
  | some .read | none =>
    conformsAll P vals tys && writes.isEmpty && places.isEmpty

/-- Write `writes` back through the mutable references `refs`, one for one:
each place changes in place, every location strictly inside its old value
becoming invalid (`Invalidation.mutated`, as `WriteRef` does). -/
def writeBack (s : State) : List Reference → List Value → Option State
  | [], [] => some s
  | r :: refs, w :: writes => do
    writeBack ((← s.write? r.location w).invalidate (.mutated r.location)) refs writes
  | [], _ :: _ => none
  | _ :: _, [] => none

/-- The new references a borrow-shape native's places designate, in state
`s`, for its reference results among `tys`: each at the place's path inside the
location of the reference argument it names, of the result's mutability and
target, created now (`Step.borrow`). -/
def placeReferences (s : State) (tys : List RuntimeType) (args : List Value)
    (places : List NativePlace) : Option (List Value) :=
  ((tys.filter (!·.isValue)).zip places).mapM fun (t, p) =>
    match t, args[p.arg]? with
    | .ref mutable target, some (.ref r) =>
      some (Step.borrow s mutable target (r.location.descend p.path))
    | _, _ => none

/-- The results of a native call in order, for return types `tys`: the next of
`refs` for a reference type, the next of `vals` otherwise; `none` unless both
are used up. -/
def nativeResultsIn {α : Type} : List RuntimeType → List α → List α → Option (List α)
  | [], vals, refs => if vals.isEmpty && refs.isEmpty then some [] else none
  | t :: tys, vals, refs =>
    if t.isValue then
      match vals with
      | v :: vals => (v :: ·) <$> nativeResultsIn tys vals refs
      | [] => none
    else
      match refs with
      | r :: refs => (r :: ·) <$> nativeResultsIn tys vals refs
      | [] => none

/-- The references native `f` writes back through, among its arguments `args`,
in order: every mutable reference argument of an update-shape native, and none
of another native, which writes nothing (`Program.nativeResultFits`). -/
def Program.writtenReferences (P : Program) (f : FunctionRef) (args : List Value) : List Reference :=
  if P.nativeShape? f == some .update then
    args.filterMap fun v => match v with
      | .ref r => if r.mutable then some r else none
      | _ => none
  else []

/-- Resume after a call of native `f` with arguments `args`, from state `s`
whose current frame is the caller's: the result must fit the native's shape
(`Program.nativeResultFits`), the values an update-shape native writes back are
written through its mutable reference arguments (`writeBack`,
`Program.writtenReferences`), the references its places designate
are made (`placeReferences`), and its results are pushed onto the caller's
segment. A native that takes and returns values only has results that conform,
deeply (`Value.wellFormed`), to its return types, and nothing else. -/
def resumeNative (P : Program) (s : State) (f : FunctionRef) (args : List Value) :
    NativeResult → Outcome
  | .failed e => .failed e
  | .returned vals writes places =>
    match s.frames with
    | [] => .wrong .noFrame
    | _ :: _ =>
      match P.returnTypes f with
      | none => .wrong (.unsupported "return types")
      | some tys =>
        match s.nativeView args with
        | none => .wrong .invalidReference
        | some views =>
          if !P.nativeResultFits f tys views vals writes places then .wrong .nativeContract else
          match writeBack s (P.writtenReferences f args) writes with
          | none => .wrong .invalidReference
          | some s =>
            match (placeReferences s tys args places).bind (nativeResultsIn tys vals) with
            | none => .wrong .nativeContract
            | some results =>
              match s.frames with
              | [] => .wrong .noFrame
              | fr :: rest =>
                .running
                  { s with frames := { fr with stack := results.reverse ++ fr.stack, pc := fr.pc + 1 } :: rest }

/-- The behavior of natives, as a relation from function and arguments to
results. A native's arguments are what it sees of them (`State.nativeView`):
value arguments as they are, reference arguments as the values they
designate. -/
abbrev NativeRel := FunctionRef → List Value → NativeResult → Prop

/-- One step, including native resumption: the native relation is applied to
what the native sees of its arguments. -/
inductive StepRel (P : Program) (N : NativeRel) : State → Outcome → Prop where
  | core {s : State} {o : Outcome} : step P s = .done o → StepRel P N s o
  | native {s s' : State} {f : FunctionRef} {args views : List Value} {r : NativeResult} :
      step P s = .callNative s' f args views → N f views r →
      StepRel P N s (resumeNative P s' f args r)

/-- States reachable from `s₀` through running steps. -/
inductive Reachable (P : Program) (N : NativeRel) (s₀ : State) : State → Prop where
  | refl : Reachable P N s₀ s₀
  | step {s s' : State} : Reachable P N s₀ s → StepRel P N s (.running s') → Reachable P N s₀ s'

/-- The arguments of a call from the transaction, given the values `held` it
holds: each is a value, or a reference to a whole held value of the
reference's target type that no other argument refers to (reference
parameters are distinct roots), taken before anything was invalidated. The
held values are well-formed values of value types (`Value.wellFormed`), so
no reference is anywhere inside them. -/
def heldArguments (P : Program) (held : List Value) (args : List Value) : Bool :=
  let refs := args.filterMap fun v => match v with
    | .ref r => some r
    | _ => none
  held.all (fun v => v.type.isValue && v.wellFormed P) &&
  args.all (fun v => match v with
    | .ref _ => true
    | _ => v.type.isValue) &&
  refs.all (fun r => r.since == 0 && match r.location.root, r.location.path with
    | .held k, [] => match held[k]? with
      | some v => v.conforms P r.target
      | none => false
    | _, _ => false) &&
  decide (refs.map (·.location)).Nodup

/-- The state that calls function instance `f` from the transaction with
`args`, over global storage `storage`, the transaction's input, and with the
values `held` the transaction lends by reference. Top-level entry counts the
module as active (R3, R4). The entry frame is the execution's first frame.

The arguments are values, or references to held values (`heldArguments`): no
other location exists before the call for a reference to designate. The type
arguments of `f` are closed types of the program, one per type parameter,
satisfying its constraints (`Program.typeArgsOk`): the transaction supplies
them as a call site does, and "constraints are checked at call sites" (book,
`generics-and-abilities.md`, "Verifying Constraints"). A non-generic `f`
takes none. -/
def initialState (P : Program) (f : FunctionRef) (args : List Value) (storage : GlobalStorage := [])
    (held : List Value := []) : Option State := do
  let info ← P.function? f
  let _ ← info.definition.code
  let localTys ← P.localTypes f
  if !P.typeArgsOk f then none else
  if !heldArguments P held args then none else
  let locking := info.handle.attributes.contains .moduleLock
  let locals := (args.map some ++ List.replicate (localTys.length - args.length) none).toArray
  pure {
    frames := [{ function := f, id := 0, pc := 0, locals, stack := [], countedActive := true,
                 lockEntered := locking }]
    active := [(info.moduleId, 1)]
    lockDepth := if locking then 1 else 0
    storage
    nextFrame := 1
    invalidations := []
    held }

/-! ## Executable runner -/

/-- A deterministic native implementation, for running the semantics. -/
abbrev NativeImpl := FunctionRef → List Value → NativeResult

inductive RunResult where
  /-- The function returned `values` and committed `storage`. The default,
  empty storage, is for tests that use none; a pattern must name the
  storage. -/
  | returned (values : List Value) (storage : GlobalStorage := [])
  | failed (f : Failure)
  | wrong (r : WrongReason)
  /-- Out of fuel: the stopping relation. -/
  | limit
  deriving DecidableEq, Repr, Inhabited

def run (P : Program) (natives : NativeImpl) : Nat → State → RunResult
  | 0, _ => .limit
  | fuel + 1, s =>
    let o : Outcome :=
      match step P s with
      | .done o => o
      | .callNative s' f args views => resumeNative P s' f args (natives f views)
    match o with
    | .running s' => run P natives fuel s'
    | .returned vs st => .returned vs st
    | .failed f => .failed f
    | .wrong r => .wrong r

/-- Global storage after a transaction that started with storage `initial`
and ended with `r`: only a returning execution commits its effects; a
failure, a limit, or a wrong step roll them back (plan §5.2, "Outcomes"). -/
def RunResult.committed (initial : GlobalStorage) : RunResult → GlobalStorage
  | .returned _ st => st
  | .failed _ => initial
  | .wrong _ => initial
  | .limit => initial

/-- A native implementation for programs without natives, or that fails every
native call. -/
def noNatives : NativeImpl := fun _ _ => .failed (.abort 0)

end MoveBytecode
