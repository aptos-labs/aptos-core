-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Semantics.Run

/-!
# The ghost layer: identities, events and traces

TRUSTED. The ghost events of the reference semantics (plan §5.2, "Event
trace") and the annotated executions that D2, D3 and D4 are stated over
(plan §7, "Definitions used by D2" and "Definitions used by D4").

**Design.** The ghost layer is a *shadow* of the concrete state, kept beside
it, never inside it:

- `step` stays the single source of truth for outcomes. Nothing here changes
  a value, a state, `step`, `run` or `StepRel`, so D1 and every earlier
  statement keep their meaning, and `Eq` keeps comparing values structurally
  (identities are not part of values).
- An `IdTree` gives an identity to every node of a value, in the value's
  shape. A `Shadow` holds the identity trees of a state's locals and operand
  stacks, frame by frame, of the resources in global storage, and of the
  values the transaction holds. Pairing a shadow with its state gives the
  *occurrences*: each node of each value with its identity.
- `Shadow.coreStep` and `Shadow.nativeStep` annotate one concrete step: given
  the step's pre-state and its outcome, they say where every identity of the
  pre-state went, which identities the step created, and which it consumed,
  and they record the step's references and accesses. They follow the
  instruction's meaning (below), not `step`'s code, and read the concrete
  states only for what the instruction produced or acted on (the value it
  pushed, the callee's frame, the reference it used). The result must fit
  the concrete outcome node for node, or the step is unaccounted; D2, D3 and
  D4 require every step to be accounted (`ShadowAccounts`), so a ghost that
  drifts from the semantics falsifies them instead of making them vacuous.

Steps that end in `failed` are not annotated: the payload rolls back (plan
§7, "Rollback"), so D2, D3 and D4 judge only the states before termination
and `returned` executions. A limit stops an execution at any state, which a
trace prefix already models.

**Identities** (plan §7, D2). An identity is created by an instruction that
makes a new value (`Pack`, `PackVariant`, `VecPack`, `PackClosure`, loading
a constant or a literal, and the result of an arithmetic, comparison, cast, `VecLen`,
`TestVariant` or `Exists`), by a native, or by the transaction for the entry
function's arguments and the values it holds. A value already in global
storage when the transaction starts was created before it
(`Creator.storage`). Copying a value is not creation: the copy has the
original's identities, so a value of a type with `copy` may occur several
times, and D2a ("for types without `copy`, each identity has at most one
occurrence") is exactly the claim that only such values do. Every node has
an identity, primitives included; their abilities exempt them from every D2
constraint. A signer's address is a node inside the signer, with an identity
of its own, so that a reference to it (`signer::borrow_address`) reads its
identity as a reference to a field does. A table is a node whose children are
its fields, then its entries (language-owner ruling 1, 2026-11-04: "its
entries are part of the value: for ownership"); its keys are not nodes: they
are values of a type with `copy` and `drop`, which no reference designates
and no native passes back out.

**Transfers** need no event: moving a value (`MoveLoc`, `StLoc`, argument
passing, `Ret`), packing it into a container, pushing it onto a vector,
popping or swapping it through a reference, writing it through a reference,
publishing it (`MoveTo`) or moving it out of storage (`MoveFrom`), or a
native passing it on, into a result or into a value it writes back through a
mutable reference, keeps its identities, now at another position of the
shadow. Reading through a reference (`ReadRef`) copies the identities at the
location. The module whose code acts is recorded in each creation and
consumption.

**Consumption** is an event: `Unpack` and `UnpackVariant` consume the struct
or enum value and transfer its fields; `VecUnpack` consumes the vector and
transfers its elements; `CallClosure` consumes the closure and transfers its
captured values to the target's parameters (plan §7, "Legitimate
consumption"); a native consumes every node it does not pass on of its value
arguments and of the values its mutable reference arguments designated
(Natives, below). Discarding (`Pop`, `Eq`/`Neq`, overwriting a local, writing through
a reference over the old value, locals left at `Ret`) is not consumption:
the identities just disappear, which D2b allows only for types with `drop`.

**References** (plan §7, "Definitions used by D4"). A reference value is a
node without children; its identity is the reference's identity. A reference
is created by a borrow of a local, a field, a variant field, an element or a
resource, by `FreezeRef` (a new, immutable reference), by a borrow-shape
native (a reference to a place inside what a reference argument designates),
or by the transaction (a reference to a held value, `&signer`); each creation
is a `borrowed` event naming the reference it derives from, if any. Moving a reference (`MoveLoc`,
`StLoc`, argument passing, `Ret`) keeps its identity, and so does copying it,
as for any copied value: the copy is the same reference, used once more.
Compiled code borrows each field through its own copy of the parent
reference, so a field reference derives from the parent itself, and two
field references of one parent do not conflict unless their fields overlap.

The other D4 events: every *use* of a reference (`ReadRef`, `WriteRef`,
`FreezeRef`, a borrow derived from it, copying or moving it, passing it,
returning it, comparing it, and the vector operations, `TestVariant` and
`MoveTo` that act through it, and a native it is passed to); every *access*
to a location: a read, a write (which includes removal) or an inspection of
the node only (an enum value's variant, a vector's length), through a
reference or directly by an instruction on a local or a resource, and a
native's reads and writes through its reference arguments; a local becoming unavailable; frames
entered (with their reference parameters) and returning (with the references
they return). For D3a, the operations at a struct type that create or
consume nothing (field and variant field borrows, `TestVariant`, the global
storage operations) record the type and the module whose code ran them.

**Generic code** (plan Phase 4, stage A). A generic form is annotated as its
non-generic form: `CallGeneric` as `Call`, `PackGeneric` as `Pack`, and so on.
The values it creates and consumes are of the instance the instruction names,
and the struct types its D3a events record are instances too, so D3a's check
(the type's module ran the code) holds "under every instantiation" (plan §7,
D3a): the type arguments play no part in who may act on a type. The abilities
that decide D2 (`copy`, `drop`) are the instances' (`RuntimeType.abilities`).

**Function values** (plan Phase 4, stage B). A closure is a node whose
children are its captured values: "a closure capture" is an occurrence (plan
§7, "Definitions used by D2"). `PackClosure` creates the closure around the
captured values it moves in, as `Pack` creates a struct around its fields,
and records the creation by the code of the current module (D3c).
`CallClosure` consumes the closure and moves its captured values and the
provided arguments into the target's parameters; the provided references are
used, and the target's frame is entered with them as its reference
parameters, as for `Call`.

Within a step the events come in this order: accesses, uses, created
references, the ends of locals and frames, and the D2 and D3 events. D4's
liveness runs "from its creation to the last use", so an access counts while
a reference used in the same step is live, and a reference created by a step
is not live during that step's accesses.
-/

namespace MoveBytecode

/-! ## Paths and nodes of a value -/

/-- A position inside a value: the indices followed from its root, each into
a vector's elements, a struct value's fields, or an enum value's fields. -/
abbrev Path := List Nat

namespace Value

/-- The values a value directly contains, in order: a vector's elements, a
struct value's fields, an enum value's fields, a closure's captured values,
the address a signer holds ("VM Runtime representation is equivalent to ...
`Master { account: address }`", `signer.move`; `PathStep.signerAddress`), and
a table's fields, then its entries (`PathStep.tableEntry`), but not its
keys. -/
def children : Value → List Value
  | .vector _ vs => vs
  | .struct _ fs => fs
  | .variant _ _ fs => fs
  | .closure _ _ cs _ _ _ => cs
  | .table _ fs _ es => fs ++ es
  | .bool _ => []
  | .int _ _ => []
  | .address _ => []
  | .signer a => [.address a]
  | .ref _ => []

/-- The node at `path` inside a value. -/
def at? : Value → Path → Option Value
  | v, [] => some v
  | v, i :: p =>
    match v.children[i]? with
    | some c => c.at? p
    | none => none

mutual
/-- Every node of a value with its path: the value itself, then the nodes of
each child in order (pre-order). -/
def nodes : Value → List (Path × Value)
  | v@(.vector _ vs) => ([], v) :: Value.nodesFrom 0 vs
  | v@(.struct _ fs) => ([], v) :: Value.nodesFrom 0 fs
  | v@(.variant _ _ fs) => ([], v) :: Value.nodesFrom 0 fs
  | v@(.closure _ _ cs _ _ _) => ([], v) :: Value.nodesFrom 0 cs
  | v@(.table _ fs _ es) => ([], v) :: (Value.nodesFrom 0 fs ++ Value.nodesFrom fs.length es)
  | v@(.bool _) => [([], v)]
  | v@(.int _ _) => [([], v)]
  | v@(.address _) => [([], v)]
  | v@(.signer a) => [([], v), ([0], .address a)]
  | v@(.ref _) => [([], v)]

/-- The nodes of the children `vs`, the first of which has index `i`. -/
def nodesFrom : Nat → List Value → List (Path × Value)
  | _, [] => []
  | i, v :: vs => (v.nodes.map fun (p, w) => (i :: p, w)) ++ Value.nodesFrom (i + 1) vs
end

/-- Whether a value is a reference. -/
def isRef : Value → Bool
  | .ref _ => true
  | .bool _ => false
  | .int _ _ => false
  | .address _ => false
  | .signer _ => false
  | .vector _ _ => false
  | .struct _ _ => false
  | .variant _ _ _ => false
  | .closure .. => false
  | .table .. => false

end Value

/-- The nodes of a list of values, each with the index of its value in the
list and its path inside that value. -/
def nodesOf (vs : List Value) : List (Nat × Path × Value) :=
  vs.zipIdx.flatMap fun (v, j) => v.nodes.map fun (p, w) => (j, p, w)

/-- The index of the child a path step goes to (`Value.children`): a signer's
address is its only child, and entry `i` of a table is its child `i` after the
table's fields (`Value.childIndex`). -/
def PathStep.index : PathStep → Nat
  | .field i => i
  | .variantField _ i => i
  | .element i => i
  | .signerAddress => 0
  | .tableEntry i => i

/-- The index of the child of value `v` that path step `step` goes to
(`Value.children`): the step's own index (`PathStep.index`), except that a
table's entries come after its fields. -/
def Value.childIndex : Value → PathStep → Nat
  | .table _ fields _ _, .tableEntry i => fields.length + i
  | _, step => step.index

/-- The node indices of path `path` inside value `v`, step by step
(`Value.childIndex`); past a step that `v` does not have, each step's own
index. -/
def Value.indicesOf (v : Value) : List PathStep → Path
  | [] => []
  | step :: path =>
    v.childIndex step ::
      match v.child? step with
      | some c => c.indicesOf path
      | none => path.map PathStep.index

/-- The node indices of a location's path, inside `v`, the value at its root. -/
def Location.indicesIn (l : Location) (v : Value) : Path :=
  v.indicesOf l.path

/-! ## Events -/

/-- What created an identity. -/
inductive Creator where
  /-- The transaction, for an argument of the entry function and a value it
  holds (E1, E2). -/
  | transaction
  /-- Global storage, for a value stored there when the transaction starts:
  an earlier transaction created it. -/
  | storage
  /-- Instruction `instr`, executed by code of module `module`. -/
  | instruction (module : ModuleId) (instr : Bytecode)
  /-- Native function `f` of module `module`, called with `args` as it saw them
  (`State.nativeView`: a reference argument as the value it designates). -/
  | native (f : FunctionRef) (module : ModuleId) (args : List Value)
  deriving DecidableEq, Repr

/-- What consumed an identity. -/
inductive Consumer where
  /-- `Unpack` or `UnpackVariant`, executed by code of module `module`. -/
  | unpack (module : ModuleId)
  /-- `VecUnpack`. -/
  | vecUnpack
  /-- `CallClosure`, executed by code of module `module`: the closure is
  consumed and its captured values move to the target's parameters. -/
  | callClosure (module : ModuleId)
  /-- Native function `f` of module `module`, which did not pass the node on;
  `whole` when it passed on nothing inside the node either. -/
  | native (f : FunctionRef) (module : ModuleId) (whole : Bool)
  deriving DecidableEq, Repr

/-- How a step accesses the value at a location (plan §5.2, "local and global
reads and writes"). -/
inductive Access where
  /-- The whole value at the location is observed. -/
  | read
  /-- The value at the location is changed or removed. -/
  | write
  /-- Only the node at the location is observed: an enum value's variant
  (`TestVariant`, a variant field borrow) or a vector's length (`VecLen`, an
  element borrow's bounds check), which no write through a reference strictly
  inside the value can change. -/
  | inspect
  deriving DecidableEq, Repr

/-- A ghost event. `v` is the concrete value of the node when the event
happened; `r` is the concrete reference. -/
inductive Event where
  | created (id : Nat) (v : Value) (creator : Creator)
  | consumed (id : Nat) (v : Value) (consumer : Consumer)
  /-- Reference `id` was created: borrowed from the root of its location
  (`parent = none`), or derived from reference `parent`. -/
  | borrowed (id : Nat) (r : Reference) (parent : Option Nat)
  /-- Reference `id` was used. -/
  | used (id : Nat) (r : Reference)
  /-- Location `loc` was accessed, through reference `via`, or directly by an
  instruction on a local or a resource (`via = none`). -/
  | accessed (access : Access) (loc : Location) (via : Option Nat)
  /-- The local at `root` became unavailable (`MoveLoc`). -/
  | vacated (root : Root)
  /-- Frame `frame` was entered, with the references `params` as its
  parameters. -/
  | entered (frame : FrameId) (params : List Nat)
  /-- Frame `frame` returned, returning the references `results`. -/
  | frameEnded (frame : FrameId) (results : List Nat)
  /-- Code of module `module` ran `instr` at struct type `type`: a field or
  variant field borrow, `TestVariant`, or a global storage operation (D3a). -/
  | privileged (module : ModuleId) (type : StructId) (instr : Bytecode)
  deriving DecidableEq, Repr

/-! ## Identity trees -/

/-- The identities of a value: one node per node of the value, with the
value's shape. -/
inductive IdTree where
  | node (id : Nat) (children : List IdTree)
  deriving Repr, Inhabited

namespace IdTree

def id : IdTree → Nat
  | .node i _ => i

def children : IdTree → List IdTree
  | .node _ cs => cs

/-- The node at `path`. -/
def at? : IdTree → Path → Option IdTree
  | t, [] => some t
  | t, i :: p =>
    match t.children[i]? with
    | some c => c.at? p
    | none => none

/-- The tree with the node at `path` replaced by `t`. -/
def set? (u : IdTree) : Path → IdTree → Option IdTree
  | [], t => some t
  | i :: p, t => do
    let c ← u.children[i]?
    pure (.node u.id (u.children.set i (← c.set? p t)))

mutual
/-- An identity tree fits a value when it has the value's shape. -/
def fits : IdTree → Value → Bool
  | .node _ cs, v => IdTree.fitsAll cs v.children

def fitsAll : List IdTree → List Value → Bool
  | [], [] => true
  | t :: ts, v :: vs => t.fits v && IdTree.fitsAll ts vs
  | [], _ :: _ => false
  | _ :: _, [] => false
end

mutual
/-- The occurrences in a value: each node's identity with the node, in
pre-order. Meaningful when the tree fits the value. -/
def occurrences : IdTree → Value → List (Nat × Value)
  | .node i cs, v => (i, v) :: IdTree.occurrencesAll cs v.children

def occurrencesAll : List IdTree → List Value → List (Nat × Value)
  | t :: ts, v :: vs => t.occurrences v ++ IdTree.occurrencesAll ts vs
  | [], _ => []
  | _ :: _, [] => []
end

/-- The identity a node keeps or gets: `idOf p = some i` keeps identity `i`;
`none` creates the next fresh identity, by `c`. -/
def rootId (idOf : Path → Option Nat) (c : Creator) (p : Path) (v : Value) (n : Nat) :
    Nat × Nat × List Event :=
  match idOf p with
  | some i => (i, n, [])
  | none => (n, n + 1, [.created n v c])

mutual
/-- The identity tree of value `v` at path `p`: each node keeps the identity
`idOf` gives its path, or is created by `c` with the next fresh identity,
counting from `n`. Returns the next fresh identity and the creation events,
in pre-order. -/
def build (idOf : Path → Option Nat) (c : Creator) (p : Path) : Value → Nat →
    IdTree × Nat × List Event
  | v@(.vector _ vs), n =>
    let (i, n, e) := rootId idOf c p v n
    let (cs, n, es) := IdTree.buildAll idOf c p 0 vs n
    (.node i cs, n, e ++ es)
  | v@(.struct _ fs), n =>
    let (i, n, e) := rootId idOf c p v n
    let (cs, n, es) := IdTree.buildAll idOf c p 0 fs n
    (.node i cs, n, e ++ es)
  | v@(.variant _ _ fs), n =>
    let (i, n, e) := rootId idOf c p v n
    let (cs, n, es) := IdTree.buildAll idOf c p 0 fs n
    (.node i cs, n, e ++ es)
  | v@(.closure _ _ captured _ _ _), n =>
    let (i, n, e) := rootId idOf c p v n
    let (cs, n, es) := IdTree.buildAll idOf c p 0 captured n
    (.node i cs, n, e ++ es)
  | v@(.table _ fs _ entries), n =>
    let (i, n, e) := rootId idOf c p v n
    let (cs, n, es) := IdTree.buildAll idOf c p 0 fs n
    let (cs', n, es') := IdTree.buildAll idOf c p fs.length entries n
    (.node i (cs ++ cs'), n, e ++ es ++ es')
  | v@(.bool _), n => let (i, n, e) := rootId idOf c p v n; (.node i [], n, e)
  | v@(.int _ _), n => let (i, n, e) := rootId idOf c p v n; (.node i [], n, e)
  | v@(.address _), n => let (i, n, e) := rootId idOf c p v n; (.node i [], n, e)
  | v@(.signer a), n =>
    let (i, n, e) := rootId idOf c p v n
    let (j, n, e') := rootId idOf c (p ++ [0]) (.address a) n
    (.node i [.node j []], n, e ++ e')
  | v@(.ref _), n => let (i, n, e) := rootId idOf c p v n; (.node i [], n, e)

/-- The identity trees of the children `vs` of the node at path `p`, the
first of which has index `i`. -/
def buildAll (idOf : Path → Option Nat) (c : Creator) (p : Path) : Nat → List Value → Nat →
    List IdTree × Nat × List Event
  | _, [], n => ([], n, [])
  | i, v :: vs, n =>
    let (t, n, e) := IdTree.build idOf c (p ++ [i]) v n
    let (ts, n, es) := IdTree.buildAll idOf c p (i + 1) vs n
    (t :: ts, n, e ++ es)
end

/-- A value created whole by `c`: every node gets a fresh identity. -/
def fresh (c : Creator) (v : Value) (n : Nat) : IdTree × Nat × List Event :=
  IdTree.build (fun _ => none) c [] v n

/-- Values created whole by `c`, in order. -/
def freshAll (c : Creator) (vs : List Value) (n : Nat) : List IdTree × Nat × List Event :=
  vs.foldl (fun (ts, n, evs) v => let (t, n, e) := IdTree.fresh c v n; (ts ++ [t], n, evs ++ e))
    ([], n, [])

end IdTree

/-! ## The shadow of a state -/

/-- The identities of a frame's locals and operand-stack segment, slot for
slot. -/
structure ShadowFrame where
  locals : Array (Option IdTree)
  stack : List IdTree
  deriving Repr, Inhabited

/-- The identities of a state's values, frame for frame, resource for
resource in global storage, and held value for held value, and the next
fresh identity. -/
structure Shadow where
  frames : List ShadowFrame
  storage : List IdTree
  held : List IdTree
  fresh : Nat
  deriving Repr, Inhabited

namespace ShadowFrame

def slotFits : Option IdTree → Option Value → Bool
  | some t, some v => t.fits v
  | none, none => true
  | some _, none => false
  | none, some _ => false

def fits (g : ShadowFrame) (fr : Frame) : Bool :=
  g.locals.size == fr.locals.size &&
    (g.locals.toList.zip fr.locals.toList).all (fun (t, v) => slotFits t v) &&
    IdTree.fitsAll g.stack fr.stack

def slotOccurrences : Option IdTree → Option Value → List (Nat × Value)
  | some t, some v => t.occurrences v
  | some _, none => []
  | none, _ => []

/-- The occurrences in a frame: its locals, then its operand-stack segment. -/
def occurrences (g : ShadowFrame) (fr : Frame) : List (Nat × Value) :=
  (g.locals.toList.zip fr.locals.toList).flatMap (fun (t, v) => slotOccurrences t v) ++
    IdTree.occurrencesAll g.stack fr.stack

end ShadowFrame

namespace Shadow

/-- A shadow fits a state when it has the state's frames, and each frame's
locals and segment, its global storage and its held values, node for node. -/
def fits (h : Shadow) (s : State) : Bool :=
  h.frames.length == s.frames.length && (h.frames.zip s.frames).all (fun (g, fr) => g.fits fr) &&
    IdTree.fitsAll h.storage (s.storage.map (·.value)) && IdTree.fitsAll h.held s.held

/-- **Occurrences** (plan §7): every position of the state holding an
identity, nested positions included, as the identity with the value there:
locals and operand stacks, global storage, and the values the transaction
holds. -/
def occurrences (h : Shadow) (s : State) : List (Nat × Value) :=
  (h.frames.zip s.frames).flatMap (fun (g, fr) => g.occurrences fr) ++
    IdTree.occurrencesAll h.storage (s.storage.map (·.value)) ++
    IdTree.occurrencesAll h.held s.held

/-- The shadow of an initial state: every value in its frames and every value
the transaction holds is created by the transaction (E1, E2); every value in
global storage was created before the transaction (`Creator.storage`). The
reference arguments are references the transaction borrowed from the values
it holds, and the entry frame is entered with them as its reference
parameters. -/
def initial (s : State) : Shadow × List Event :=
  let frame (fr : Frame) (n : Nat) : ShadowFrame × Nat × List Event :=
    let (locals, n, e₁) := fr.locals.toList.foldl (fun (ls, n, evs) slot =>
      match slot with
      | some v => let (t, n, e) := IdTree.fresh .transaction v n; (ls ++ [some t], n, evs ++ e)
      | none => (ls ++ [none], n, evs)) ([], n, [])
    let (stack, n, e₂) := IdTree.freshAll .transaction fr.stack n
    ({ locals := locals.toArray, stack }, n, e₁ ++ e₂)
  let (frames, n, evs) := s.frames.foldl (fun (gs, n, evs) fr =>
    let (g, n, e) := frame fr n; (gs ++ [g], n, evs ++ e)) ([], 0, [])
  let (storage, n, e₁) := IdTree.freshAll .storage (s.storage.map (·.value)) n
  let (held, n, e₂) := IdTree.freshAll .transaction s.held n
  let refs := (frames.zip s.frames).flatMap fun (g, fr) =>
    let params := (g.locals.toList.zip fr.locals.toList).filterMap fun (t, v) =>
      match t, v with
      | some t, some (.ref r) => some (t.id, r)
      | _, _ => none
    params.map (fun (i, r) => Event.borrowed i r none) ++ [.entered fr.id (params.map (·.1))]
  (⟨frames, storage, held, n⟩, evs ++ e₁ ++ e₂ ++ refs)

/-! ### Locations in a shadow -/

/-- The position of frame `f` in the call stack of `s`. -/
def framePos (s : State) (f : FrameId) : Option Nat :=
  s.frames.findIdx? (·.id == f)

/-- The position of the resource of type `t` under `a` in the global storage
of `s`. -/
def resourcePos (s : State) (a : Address) (t : StructId) : Option Nat :=
  s.storage.findIdx? fun r => r.address == a && r.type == t

/-- The identity tree of the value at `root` in state `s`, whose shadow is
`h`. -/
def rootTree? (h : Shadow) (s : State) : Root → Option IdTree
  | .local_ f i => do (← (← h.frames[← framePos s f]?).locals[i]?)
  | .global a t => do h.storage[← resourcePos s a t]?
  | .held k => h.held[k]?

/-- The shadow with the tree of the value at `root` replaced by `t`. -/
def withRootTree? (h : Shadow) (s : State) (root : Root) (t : IdTree) : Option Shadow :=
  match root with
  | .local_ f i => do
    let k ← framePos s f
    let g ← h.frames[k]?
    let _ ← g.locals[i]?
    pure { h with frames := h.frames.set k { g with locals := g.locals.setIfInBounds i (some t) } }
  | .global a ty => do
    let k ← resourcePos s a ty
    let _ ← h.storage[k]?
    pure { h with storage := h.storage.set k t }
  | .held k => do
    let _ ← h.held[k]?
    pure { h with held := h.held.set k t }

/-- The identity tree of the value at location `l`, its path followed in the
value at its root (`Location.indicesIn`). -/
def tree? (h : Shadow) (s : State) (l : Location) : Option IdTree := do
  (← h.rootTree? s l.root).at? (l.indicesIn (← s.root? l.root))

/-- The shadow with the tree of the value at location `l` replaced by `t`. -/
def withTree? (h : Shadow) (s : State) (l : Location) (t : IdTree) : Option Shadow := do
  h.withRootTree? s l.root (← (← h.rootTree? s l.root).set? (l.indicesIn (← s.root? l.root)) t)

end Shadow

/-- The shadow after a step: of the next state, or of the returned values and
the committed global storage. -/
inductive ShadowPost where
  | running (h : Shadow)
  | returned (ids : List IdTree) (storage : List IdTree)
  deriving Repr, Inhabited

namespace ShadowPost

def fits : ShadowPost → Outcome → Bool
  | .running h, .running s => h.fits s
  | .returned ts sts, .returned vs st =>
    IdTree.fitsAll ts vs && IdTree.fitsAll sts (st.map (·.value))
  | .running _, .returned _ _ => false
  | .running _, .failed _ => false
  | .running _, .wrong _ => false
  | .returned _ _, .running _ => false
  | .returned _ _, .failed _ => false
  | .returned _ _, .wrong _ => false

/-- The occurrences after a step: in the next state, or in the returned
values and the committed global storage. -/
def occurrences : ShadowPost → Outcome → List (Nat × Value)
  | .running h, .running s => h.occurrences s
  | .returned ts sts, .returned vs st =>
    IdTree.occurrencesAll ts vs ++ IdTree.occurrencesAll sts (st.map (·.value))
  | .running _, _ => []
  | .returned _ _, _ => []

end ShadowPost

/-! ## Annotating a step -/

namespace Shadow

/-- The value on top of the current frame's segment after a step. -/
def topAfter : Outcome → Option Value
  | .running s => do (← s.frames.head?).stack.head?
  | .returned _ _ => none
  | .failed _ => none
  | .wrong _ => none

/-- The ghost effect of instruction `instr` of module `m`, executed from state
`s` in its current frame `fr`, whose shadow is `h`, with outcome `o`:

- `Pop`, `BrTrue`, `BrFalse`: discard the top value (dropping a reference is
  not a use).
- `Branch`, `Nop`: nothing.
- `Ret`: the segment's values move to the caller's segment, or become the
  returned values; values left in locals are discarded. The returned
  references are used, and the frame ends returning them.
- Literal and constant loads: push a value created whole by the instruction.
- Casts, `Not`, `Negate`; arithmetic, bitwise, comparison, `Eq`, `Neq`,
  `Or`, `And`, shifts: discard the one or two operands and push a created
  result. `Eq` and `Neq` on references read both locations and use both
  references.
- `CopyLoc`: read the local, and push its identities again (a copy is not a
  new value); copying a reference uses it.
- `MoveLoc`, `StLoc`: move the identities between the local and the segment
  (a reference moved is used); `MoveLoc` removes the local's value and makes
  the local unavailable, `StLoc` writes the local and discards what it held.
- `Call`, `CallGeneric`: the arguments move from the caller's segment to the
  callee's first locals; reference arguments are used, and the callee's frame
  is entered with them as its reference parameters. A native call is
  `nativeStep`'s.
- `Pack`, `PackVariant`, `VecPack`, `PackClosure`: the operands move into a
  container, which the instruction creates; a closure's container holds its
  captured values.
- `Unpack`, `UnpackVariant`, `VecUnpack`: the instruction consumes the
  container and moves its contents onto the segment.
- `ReadRef`: read the location, use the reference, and push the location's
  identities again.
- `WriteRef`: write the location, use the reference, move the value into
  the location, discarding the old value.
- `FreezeRef`: use the reference, and push a new one derived from it.
- The borrows of a local or a resource: push a new reference borrowed from
  the root. Field, variant field and element borrows: use the reference, and
  push a new one derived from it; a variant field or element borrow inspects
  the location (its variant, its length).
- `TestVariant`, `VecLen`: inspect the location, use the reference, and push
  a created result.
- `VecPushBack`, `VecPopBack`, `VecSwap`: write the vector's location and use
  the reference; the pushed value moves in, the popped element moves out,
  the swapped elements move.
- `Exists`: push a created result. `MoveFrom`: write (remove) the resource's
  location and move its value onto the segment. `MoveTo`: read the signer
  through the reference, use it, write the resource's location, and move the
  value into global storage.

Field and variant field borrows, `TestVariant` and the global storage
operations record the struct type and the module that ran them (D3a).

- `CallClosure`: as `Call`, with the closure's captured values among the
  arguments, mixed with the provided ones by its mask; the instruction
  consumes the closure.

A generic form is annotated as its non-generic form; `ExistsGeneric` records
its instance, translated under the frame's type arguments as the step does.
Every other instruction (`Abort`, `AbortMsg`) is not accounted: failures roll
back. -/
def exec (P : Program) (m : ModuleId) (instr : Bytecode) (s : State) (fr : Frame) (o : Outcome)
    (h : Shadow) : Option (ShadowPost × List Event) := do
  let g :: rest := h.frames | none
  let creator := Creator.instruction m instr
  let local_ (i : Nat) : Location := ⟨.local_ fr.id i, []⟩
  let cont (g : ShadowFrame) (fresh : Nat) : ShadowPost := .running { h with frames := g :: rest, fresh }
  let discard (n : Nat) : Option (ShadowPost × List Event) := do
    guard (n ≤ g.stack.length)
    pure (cont { g with stack := g.stack.drop n } h.fresh, [])
  let replace (n : Nat) : Option (ShadowPost × List Event) := do
    guard (n ≤ g.stack.length)
    let v ← topAfter o
    let (t, fresh, evs) := IdTree.fresh creator v h.fresh
    pure (cont { g with stack := t :: g.stack.drop n } fresh, evs)
  let pack : Option (ShadowPost × List Event) := do
    let v ← topAfter o
    let n := v.children.length
    guard (n ≤ g.stack.length)
    pure (cont { g with stack := .node h.fresh (g.stack.take n).reverse :: g.stack.drop n }
      (h.fresh + 1), [.created h.fresh v creator])
  let unpack (c : Consumer) : Option (ShadowPost × List Event) := do
    let t :: st := g.stack | none
    let v ← fr.stack.head?
    pure (cont { g with stack := t.children.reverse ++ st } h.fresh, [.consumed t.id v c])
  -- The reference at depth `k` of the segment, with its identity.
  let refAt (k : Nat) : Option (Nat × Reference) := do
    let some (.ref r) := fr.stack[k]? | none
    pure ((← g.stack[k]?).id, r)
  -- Pop `n` slots and push the reference the step created, derived from
  -- `parent`, with the events `before` and `after` around its creation.
  let borrowNew (n : Nat) (parent : Option Nat) (before after : List Event) :
      Option (ShadowPost × List Event) := do
    guard (n ≤ g.stack.length)
    let some (.ref r) := topAfter o | none
    pure (cont { g with stack := .node h.fresh [] :: g.stack.drop n } (h.fresh + 1),
      before ++ [.borrowed h.fresh r parent] ++ after)
  -- The shadow after popping `n` slots and changing the tree at location `l`
  -- with `f`.
  let change (n : Nat) (l : Location) (f : IdTree → Option IdTree) : Option Shadow := do
    guard (n ≤ g.stack.length)
    let h₁ := { h with frames := { g with stack := g.stack.drop n } :: rest }
    h₁.withTree? s l (← f (← h₁.tree? s l))
  match instr with
  | .pop | .brTrue _ | .brFalse _ => discard 1
  | .branch _ | .nop => some (cont g h.fresh, [])
  | .ret =>
    let refs := (g.stack.reverse.zip fr.stack.reverse).filterMap fun (t, v) =>
      match v with
      | .ref r => some (t.id, r)
      | _ => none
    let evs := refs.map (fun (i, r) => Event.used i r) ++ [.frameEnded fr.id (refs.map (·.1))]
    match rest with
    | [] => some (.returned g.stack.reverse h.storage, evs)
    | caller :: rest' =>
      some (.running { h with frames := { caller with stack := g.stack ++ caller.stack } :: rest' },
        evs)
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => replace 0
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => replace 0
  | .ldTrue | .ldFalse | .ldConst _ => replace 0
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => replace 1
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => replace 1
  | .not | .negate => replace 1
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .shl | .shr => replace 2
  | .or | .and | .lt | .gt | .le | .ge => replace 2
  | .eq | .neq => do
    let (post, evs) ← replace 2
    match refAt 1, refAt 0 with
    | some (ta, ra), some (tb, rb) =>
      pure (post, [.accessed .read ra.location (some ta), .accessed .read rb.location (some tb),
        .used ta ra, .used tb rb] ++ evs)
    | _, _ => pure (post, evs)
  | .copyLoc i => do
    let some (some t) := g.locals[i]? | none
    let uses := match fr.locals[i]? with
      | some (some (.ref r)) => [Event.used t.id r]
      | _ => []
    pure (cont { g with stack := t :: g.stack } h.fresh, [.accessed .read (local_ i) none] ++ uses)
  | .moveLoc i => do
    let some (some t) := g.locals[i]? | none
    let uses := match fr.locals[i]? with
      | some (some (.ref r)) => [Event.used t.id r]
      | _ => []
    pure (cont { g with locals := g.locals.setIfInBounds i none, stack := t :: g.stack } h.fresh,
      [.accessed .write (local_ i) none] ++ uses ++ [.vacated (.local_ fr.id i)])
  | .stLoc i => do
    let t :: st := g.stack | none
    let uses := match fr.stack.head? with
      | some (.ref r) => [Event.used t.id r]
      | _ => []
    pure (cont { g with locals := g.locals.setIfInBounds i (some t), stack := st } h.fresh,
      [.accessed .write (local_ i) none] ++ uses)
  | .call _ | .callGeneric _ => do
    let .running s' := o | none
    let callee :: caller :: _ := s'.frames | none
    let k := fr.stack.length - caller.stack.length
    guard (k ≤ g.stack.length)
    let args := (g.stack.take k).reverse
    let refs := (args.zip (fr.stack.take k).reverse).filterMap fun (t, v) =>
      match v with
      | .ref r => some (t.id, r)
      | _ => none
    let locals := (args.map some ++ List.replicate (callee.locals.size - k) none).toArray
    pure (.running { h with frames := { locals, stack := [] } :: { g with stack := g.stack.drop k } :: rest },
      refs.map (fun (i, r) => Event.used i r) ++ [.entered callee.id (refs.map (·.1))])
  | .pack _ | .packVariant _ | .vecPack _ _ | .packGeneric _ | .packVariantGeneric _ => pack
  | .packClosure _ _ | .packClosureGeneric _ _ => pack
  | .callClosure _ => do
    let .running s' := o | none
    let callee :: caller :: _ := s'.frames | none
    let some c@(.closure _ mask _ _ _ _) := fr.stack.head? | none
    let ct :: st := g.stack | none
    let k := fr.stack.length - 1 - caller.stack.length
    guard (k ≤ st.length)
    let provided := (st.take k).reverse
    let refs := (provided.zip ((fr.stack.drop 1).take k).reverse).filterMap fun (t, v) =>
      match v with
      | .ref r => some (t.id, r)
      | _ => none
    let args := maskCompose mask ct.children provided
    let locals := (args.map some ++ List.replicate (callee.locals.size - args.length) none).toArray
    pure (.running { h with frames := { locals, stack := [] } :: { g with stack := st.drop k } :: rest },
      refs.map (fun (i, r) => Event.used i r) ++ [.entered callee.id (refs.map (·.1)),
        .consumed ct.id c (.callClosure m)])
  | .unpack _ | .unpackVariant _ | .unpackGeneric _ | .unpackVariantGeneric _ => unpack (.unpack m)
  | .vecUnpack _ _ => unpack .vecUnpack
  -- References.
  | .readRef => do
    let (t, r) ← refAt 0
    let tv ← h.tree? s r.location
    pure (cont { g with stack := tv :: g.stack.drop 1 } h.fresh,
      [.accessed .read r.location (some t), .used t r])
  | .writeRef => do
    let (t, r) ← refAt 0
    let tv ← g.stack[1]?
    pure (.running (← change 2 r.location fun _ => some tv),
      [.accessed .write r.location (some t), .used t r])
  | .freezeRef => do
    let (t, r) ← refAt 0
    borrowNew 1 (some t) [.used t r] []
  | .mutBorrowLoc _ | .immBorrowLoc _ => borrowNew 0 none [] []
  | .mutBorrowField _ | .immBorrowField _ | .mutBorrowFieldGeneric _ | .immBorrowFieldGeneric _ => do
    let (t, r) ← refAt 0
    let .struct sid := r.target | none
    borrowNew 1 (some t) [.used t r] [.privileged m sid instr]
  | .mutBorrowVariantField _ | .immBorrowVariantField _ | .mutBorrowVariantFieldGeneric _
  | .immBorrowVariantFieldGeneric _ => do
    let (t, r) ← refAt 0
    let .struct sid := r.target | none
    borrowNew 1 (some t) [.accessed .inspect r.location (some t), .used t r]
      [.privileged m sid instr]
  | .testVariant _ | .testVariantGeneric _ => do
    let (t, r) ← refAt 0
    let .struct sid := r.target | none
    let (post, evs) ← replace 1
    pure (post, [.accessed .inspect r.location (some t), .used t r] ++ evs ++
      [.privileged m sid instr])
  -- Vectors through references.
  | .vecLen _ => do
    let (t, r) ← refAt 0
    let (post, evs) ← replace 1
    pure (post, [.accessed .inspect r.location (some t), .used t r] ++ evs)
  | .vecImmBorrow _ | .vecMutBorrow _ => do
    let (t, r) ← refAt 1
    borrowNew 2 (some t) [.accessed .inspect r.location (some t), .used t r] []
  | .vecPushBack _ => do
    let tv ← g.stack.head?
    let (t, r) ← refAt 1
    pure (.running (← change 2 r.location fun vt => some (.node vt.id (vt.children ++ [tv]))),
      [.accessed .write r.location (some t), .used t r])
  | .vecPopBack _ => do
    let (t, r) ← refAt 0
    let last ← (← h.tree? s r.location).children.getLast?
    let h₂ ← change 1 r.location fun vt => some (.node vt.id vt.children.dropLast)
    let g₂ :: rest₂ := h₂.frames | none
    pure (.running { h₂ with frames := { g₂ with stack := last :: g₂.stack } :: rest₂ },
      [.accessed .write r.location (some t), .used t r])
  | .vecSwap _ => do
    let some (Value.int _ j) := fr.stack[0]? | none
    let some (Value.int _ i) := fr.stack[1]? | none
    let (t, r) ← refAt 2
    let swap : IdTree → Option IdTree := fun vt => do
      let x ← vt.children[i.toNat]?
      let y ← vt.children[j.toNat]?
      pure (.node vt.id ((vt.children.set i.toNat y).set j.toNat x))
    pure (.running (← change 3 r.location swap), [.accessed .write r.location (some t), .used t r])
  -- Global storage.
  | .exists_ d => do
    let sid ← P.definedStruct? fr.function.module d
    let (post, evs) ← replace 1
    pure (post, evs ++ [.privileged m sid instr])
  | .existsGeneric i => do
    let cm ← P.modules[fr.function.module]?
    let x ← cm.structDefInstantiations[i]?
    let sid ← P.definedStruct? fr.function.module x.def_
    let typeArgs ← cm.typeArgsWith fr.function.typeArgs x.typeParameters
    let (post, evs) ← replace 1
    pure (post, evs ++ [.privileged m { sid with typeArgs } instr])
  | .moveFrom _ | .moveFromGeneric _ => do
    let some (.address a) := fr.stack.head? | none
    let v ← topAfter o
    let .struct sid := v.type | none
    let k ← resourcePos s a sid
    let t ← h.storage[k]?
    pure (.running { h with frames := { g with stack := t :: g.stack.drop 1 } :: rest,
                            storage := h.storage.eraseIdx k },
      [.accessed .write ⟨.global a sid, []⟩ none, .privileged m sid instr])
  | .moveTo _ | .moveToGeneric _ => do
    let tv ← g.stack.head?
    let v ← fr.stack.head?
    let .struct sid := v.type | none
    let (t, r) ← refAt 1
    let some (.signer a) := s.deref? r.location r.since | none
    pure (.running { h with frames := { g with stack := g.stack.drop 2 } :: rest,
                            storage := h.storage ++ [tv] },
      [.accessed .read r.location (some t), .accessed .write ⟨.global a sid, []⟩ none, .used t r,
       .privileged m sid instr])
  | .mutBorrowGlobal _ | .immBorrowGlobal _ | .mutBorrowGlobalGeneric _
  | .immBorrowGlobalGeneric _ => do
    let some (.ref r) := topAfter o | none
    let .struct sid := r.target | none
    borrowNew 1 none [] [.privileged m sid instr]
  -- Failures roll back; nothing to account.
  | .abort | .abortMsg => none

/-- Annotate a step of the deterministic core from `s`, whose shadow is `h`,
with outcome `o`. `none` when the step is not accounted: it does not
continue (`failed`, `wrong`), or the shadow after it does not fit `o`. -/
def coreStep (P : Program) (s : State) (o : Outcome) (h : Shadow) :
    Option (ShadowPost × List Event) := do
  let fr :: _ := s.frames | none
  let info ← P.function? fr.function
  let code ← info.definition.code
  let instr ← code.code[fr.pc]?
  let (post, evs) ← exec P info.moduleId instr s fr o h
  if post.fits o then some (post, evs) else none

end Shadow

/-! ## Natives

A native is given as a relation on values (`NativeRel`), which says nothing
about identities. Which nodes of its outputs are nodes of what it saw passed
on, and which are new, is a *flow*: part of the native's ghost behavior,
supplied with the natives and constrained by their contracts
(`NativesRespectOwnership`).

**Natives with references** (language-owner rulings, 2026-11-03). A native
sees its arguments as `State.nativeView` gives them: a reference argument as
the value it designates. Its *outputs* are its value results, then the values
it writes back (update shape), and a flow says where each of their nodes comes
from: a node of a value argument, or of a value a reference argument
designates. What happens to the nodes it saw depends on the argument:

- a value argument is the native's: a node it does not pass on is consumed by
  it;
- the value a mutable reference argument of an update-shape native designates
  is replaced by the value written back: a node it does not pass on, into a
  result or into a value written back, is consumed by it;
- the value any other reference argument designates stays where it is
  (`Program.nativeKeeps`): a node passed on from it is a copy, and none is
  consumed.

Its reference events: it reads the location of every reference argument (it
sees the whole value there), writes the location of every mutable one it
writes back, and uses every reference argument; each reference it returns for
a place (borrow shape) is created, derived from the reference argument the
place is inside. A native's reads are of whole values, where an instruction
may only inspect: `vector::length` reads the vector, while `VecLen` inspects
its length. This stricter reading is intended (language-owner ruling,
2026-11-04): natives have no access kind of their own and record no
inspection, and a borrow-shape native reads its arguments' locations too. -/

/-- Where a node of a native's output comes from. -/
inductive Origin where
  /-- The node at `path` of argument `index`, as the native sees it (the value
  a reference argument designates), which the native passes on. -/
  | arg (index : Nat) (path : Path)
  /-- A node the native creates. -/
  | fresh
  deriving DecidableEq, Repr

/-- The flows of natives: for native `f` that saw its arguments as `args` and
produced the outputs `vals` (its value results, then the values it writes
back), the origin of the node at path `p` of output `j`. -/
abbrev NativeFlows := FunctionRef → List Value → List Value → Nat → Path → Origin

/-- What an argument of native `f` of declared type `t` designates stays where it
is during the call: the argument is a reference, and `f` writes nothing back
through it. Only an update-shape native writes back, through every mutable
reference argument (`Program.nativeResultFits`). -/
def Program.nativeKeeps (P : Program) (f : FunctionRef) : RuntimeType → Bool
  | .ref mutable _ => !(mutable && P.nativeShape? f == some .update)
  | _ => false

/-- The argument nodes a flow passes on, once per result node that takes its
identity. -/
def passedOn (φ : Nat → Path → Origin) (vals : List Value) : List (Nat × Path) :=
  (nodesOf vals).filterMap fun (j, p, _) =>
    match φ j p with
    | .arg k q => some (k, q)
    | .fresh => none

/-- Whether no node strictly inside argument node `(k, q)` is passed on. -/
def keepsNothingInside (origins : List (Nat × Path)) (k : Nat) (q : Path) : Bool :=
  !origins.any fun (k', q') => k' == k && q.isPrefixOf q' && q != q'

namespace Shadow

/-- Annotate a call of native `f` from state `s`, whose shadow is `h` (the
arguments `args` still on the caller's segment), that saw them as `views`,
returned the values `vals`, wrote back `writes` and returned `places`, with
outcome `o` (Natives, above). Each output node takes the identity of the node
its flow names, or a fresh one created by the native; each node the native saw
of a value argument or through a reference argument it writes back that no
output node takes is consumed by it; the place each such reference argument
designates gets the identities of the value written back; each returned place
gets a new reference, derived from the reference argument it is inside. The
events: reads of every reference argument's location, writes of those written
back, uses of every reference argument, the new references, then
consumptions and creations. `none` when the flow names a node the native did
not see, or the shadow after the call does not fit `o`. -/
def nativeStep (P : Program) (ν : NativeFlows) (s : State) (f : FunctionRef)
    (args views vals writes : List Value) (places : List NativePlace) (o : Outcome) (h : Shadow) :
    Option (ShadowPost × List Event) := do
  let g :: rest := h.frames | none
  let info ← P.function? f
  let tys ← P.returnTypes f
  let n := args.length
  guard (n ≤ g.stack.length)
  let argIds := (g.stack.take n).reverse
  -- The identities of what the native saw of each argument.
  let seen ← (args.zip argIds).mapM fun (a, t) => match a with
    | .ref r => h.tree? s r.location
    | _ => some t
  guard (seen.length == views.length)
  let outs := vals ++ writes
  let φ := ν f views outs
  let idAt (k : Nat) (q : Path) : Option Nat := (seen[k]?.bind (·.at? q)).map (·.id)
  guard ((nodesOf outs).all fun (j, p, _) =>
    match φ j p with
    | .arg k q => (idAt k q).isSome
    | .fresh => true)
  let creator := Creator.native f info.moduleId views
  let (outTrees, fresh, created) := outs.zipIdx.foldl (fun (ts, n, evs) (v, j) =>
      let idOf (p : Path) : Option Nat :=
        match φ j p with
        | .arg k q => idAt k q
        | .fresh => none
      let (t, n, e) := IdTree.build idOf creator [] v n
      (ts ++ [t], n, evs ++ e)) ([], h.fresh, [])
  let origins := passedOn φ outs
  -- What a reference argument designates stays where it is, unless the native
  -- writes it back.
  let ptys ← P.paramTypes f
  let shared (k : Nat) : Bool := (ptys[k]?).any (P.nativeKeeps f)
  let consumed := (seen.zip views).zipIdx.flatMap fun ((t, a), k) =>
    if shared k then [] else
      (a.nodes.zip (t.occurrences a)).filterMap fun ((q, w), (i, _)) =>
        if origins.contains (k, q) then none
        else some (Event.consumed i w
          (.native f info.moduleId (keepsNothingInside origins k q)))
  -- The reference arguments with their identities, and the mutable ones whose
  -- places are written back.
  let refArgs := (argIds.zip args).filterMap fun (t, a) => match a with
    | .ref r => some (t.id, r)
    | _ => none
  let written := refArgs.filter fun (_, r) => r.mutable && P.nativeShape? f == some .update
  guard (written.length == writes.length)
  -- The references made for the places, read from the outcome, each with a
  -- fresh identity and the identity of the reference argument it derives from.
  let .running s' := o | none
  let fr' :: _ := s'.frames | none
  let results := (fr'.stack.take tys.length).reverse
  let refResults := (tys.zip results).filterMap fun (t, v) => match v with
    | .ref r => if t.isValue then none else some r
    | _ => none
  guard (refResults.length == places.length)
  let newRefs ← (refResults.zip places).zipIdx.mapM fun ((r, p), i) => do
    pure (fresh + i, r, (← argIds[p.arg]?).id)
  let resultTrees ← nativeResultsIn tys (outTrees.take vals.length)
    (newRefs.map fun (i, _, _) => IdTree.node i [])
  let h₁ : Shadow :=
    { h with frames := { g with stack := resultTrees.reverse ++ g.stack.drop n } :: rest,
             fresh := fresh + newRefs.length }
  let h₂ ← (written.zip (outTrees.drop vals.length)).foldlM
    (fun h ((_, r), t) => h.withTree? s r.location t) h₁
  let post := ShadowPost.running h₂
  guard (post.fits o)
  pure (post,
    refArgs.map (fun (i, r) => Event.accessed .read r.location (some i)) ++
      written.map (fun (i, r) => Event.accessed .write r.location (some i)) ++
      refArgs.map (fun (i, r) => Event.used i r) ++
      newRefs.map (fun (i, r, parent) => Event.borrowed i r (some parent)) ++
      consumed ++ created)

end Shadow

/-! ## Annotated executions -/

/-- Whether an outcome continues the execution or commits it: `running` or
`returned`. `failed` rolls back and `wrong` is excluded by D1. -/
def Outcome.continues : Outcome → Bool
  | .running _ => true
  | .returned _ _ => true
  | .failed _ => false
  | .wrong _ => false

/-- An annotated step: a step of `StepRel` from `s` with outcome `o`, and its
ghost effect on shadow `h`. A native that fails is not annotated (rollback). -/
inductive TracedStep (P : Program) (N : NativeRel) (ν : NativeFlows) :
    State → Shadow → Outcome → ShadowPost → List Event → Prop where
  | core {s : State} {h : Shadow} {o : Outcome} {post : ShadowPost} {evs : List Event} :
      step P s = .done o → Shadow.coreStep P s o h = some (post, evs) →
      TracedStep P N ν s h o post evs
  | native {s s' : State} {h : Shadow} {f : FunctionRef} {args views vals writes : List Value}
      {places : List NativePlace} {post : ShadowPost} {evs : List Event} :
      step P s = .callNative s' f args views → N f views (.returned vals writes places) →
      Shadow.nativeStep P ν s f args views vals writes places
          (resumeNative P s' f args (.returned vals writes places)) h =
        some (post, evs) →
      TracedStep P N ν s h (resumeNative P s' f args (.returned vals writes places)) post evs

/-- **Trace.** An annotated execution from `s₀`, through running steps: the
state reached, its shadow, and every event so far, starting with the
creation of the initial values by the transaction. -/
inductive Trace (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State) :
    State → Shadow → List Event → Prop where
  | start : Trace P N ν s₀ s₀ (Shadow.initial s₀).1 (Shadow.initial s₀).2
  | step {s s' : State} {h h' : Shadow} {evs evs' : List Event} :
      Trace P N ν s₀ s h evs → TracedStep P N ν s h (.running s') (.running h') evs' →
      Trace P N ν s₀ s' h' (evs ++ evs')

/-! ## Executable runner -/

/-- The result of an annotated run, with every event. -/
inductive TracedResult where
  | returned (values : List Value) (storage : GlobalStorage) (ids : List IdTree) (events : List Event)
  | failed (f : Failure) (events : List Event)
  | wrong (r : WrongReason) (events : List Event)
  | limit (events : List Event)
  /-- A step continued but the shadow could not account for it: a defect of
  the ghost layer, which D2, D3 and D4 exclude. -/
  | unaccounted (events : List Event)
  deriving Repr, Inhabited

/-- The concrete result, `none` for an unaccounted step. -/
def TracedResult.erase : TracedResult → Option RunResult
  | .returned vs st _ _ => some (.returned vs st)
  | .failed f _ => some (.failed f)
  | .wrong r _ => some (.wrong r)
  | .limit _ => some .limit
  | .unaccounted _ => none

/-- The events of a run. -/
def TracedResult.events : TracedResult → List Event
  | .returned _ _ _ evs | .failed _ evs | .wrong _ evs | .limit evs | .unaccounted evs => evs

/-- One step of `run` from `s`, whose shadow is `h`: its outcome, and its
annotation when it has one. -/
def traceStep (P : Program) (natives : NativeImpl) (ν : NativeFlows) (s : State) (h : Shadow) :
    Outcome × Option (ShadowPost × List Event) :=
  match step P s with
  | .done o => (o, Shadow.coreStep P s o h)
  | .callNative s' f args views =>
    match natives f views with
    | .returned vals writes places =>
      let o := resumeNative P s' f args (.returned vals writes places)
      (o, Shadow.nativeStep P ν s f args views vals writes places o h)
    | .failed e => (.failed e, none)

/-- `run` with the ghost layer: the same steps and outcomes, annotated. -/
def runTraced (P : Program) (natives : NativeImpl) (ν : NativeFlows) :
    Nat → State → Shadow → List Event → TracedResult
  | 0, _, _, evs => .limit evs
  | fuel + 1, s, h, evs =>
    match traceStep P natives ν s h with
    | (.running s', some (.running h', e)) => runTraced P natives ν fuel s' h' (evs ++ e)
    | (.returned vals st, some (.returned ids _, e)) => .returned vals st ids (evs ++ e)
    | (.failed f, _) => .failed f evs
    | (.wrong r, _) => .wrong r evs
    | (.running _, _) => .unaccounted evs
    | (.returned _ _, _) => .unaccounted evs

end MoveBytecode
