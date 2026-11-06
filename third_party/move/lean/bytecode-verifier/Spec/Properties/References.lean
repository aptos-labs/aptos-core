-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Properties.Ownership

/-!
# Reference safety D4

TRUSTED. D4 (plan §7) over annotated executions (`Trace`). The ghost layer
(`Spec.Semantics.Trace`) gives every reference an identity, records each
reference's creation and the reference it derives from (`borrowed`), each use
of a reference (`used`), each access to a location (`accessed`), each local
becoming unavailable (`vacated`), and each frame's reference parameters and
returned references (`entered`, `frameEnded`).

**Formulation** (plan §16, question 1). The statements are trace definitions
over these events, written as the plan words them; there is no borrow graph
and no permission state. Each is an executable check on a list of events
(`RefSafety`), evaluated on every annotated execution prefix.

- **Derivation** (plan §7, "Definitions used by D4") is transitive: `d`
  derives from `r` when `d` is `r` or was borrowed through a reference that
  derives from `r` (`derivesFrom`). A reference keeps its identity when it is
  moved, copied, passed to a callee or returned, so a reference "returned by
  a call that received `r` and whose result provenance includes `r`" derives
  from `r` by the same chain, and the copies of a reference are that
  reference. (Were each copy a new reference, the field references compiled
  code borrows through separate copies of one parent would conflict, though
  they designate disjoint fields.)
- **Use**: `ReadRef`, `WriteRef`, `FreezeRef`, deriving a field, variant
  field or element borrow, copying or moving the reference, passing it as an
  argument, returning it, comparing it, and the instructions that act through
  it (`VecLen`, the vector updates, `TestVariant`, `MoveTo`'s `&signer`).
  Dropping a reference (`Pop`, a local left at `Ret`) is not a use.
- **Liveness**: "`r` is live from its creation to the last use of `r` or of
  any reference derived from it; a live child keeps its parent's borrow
  alive". In the events of an execution, `r` is live at an event that comes
  after `r`'s creation and before some use of `r` or of a reference derived
  from it (`usedAfter`). A child keeps its parent's borrow alive *on the
  child's location*: D4b″ and D4c compare an access with the locations of
  the later uses of `r` and of the references derived from it
  (`liveLocations`), not with `r`'s whole location once only a child of `r`
  is used. Compiled code borrows `&mut s.f` and `&mut s.g` through two
  borrows of `s` (`MutBorrowLoc`, then `MutBorrowField`); the plan's wording,
  read as comparing with all of `s`, would make a write through `&mut s.f`
  conflict with the borrow of `s` behind the live `&mut s.g`, though the
  fields are disjoint. D4b′ asks less: a `&mut` reference constrains its own
  location only, until the last use of the reference itself
  (`usedItselfAfter`). The ghost layer lists a step's accesses before its
  uses, so an access in the step of a reference's last use counts, and a
  step's created references after its accesses, so a reference is not live
  during the step that creates it.
- **Overlap**: a read or write of location `ℓ'` overlaps a reference to `ℓ`
  when they have the same root and one path extends the other; an inspection
  of the node at `ℓ'` (a variant, a length) overlaps it when `ℓ'` is at or
  inside `ℓ`: a reference strictly inside the node cannot change what is
  inspected (`Access.overlaps`). `ℓ'` *strictly encloses* `ℓ` when `ℓ` is
  strictly inside the value at `ℓ'` (`Location.strictlyInside`).
- **Active frame**: a frame is active from its entry to its return, while
  its callees run too (`whileActive`).

The properties:

- **D4a no dangling.** Every use of a reference targets a valid location: in
  the state before the step, the reference's location resolves and nothing
  recorded since its creation invalidated it (`State.deref?`). This is
  stronger than D1, which checks validity only where a location is accessed.
- **D4b′ containment.** For every `&mut` reference `r`, at every access
  after `r`'s creation and before a later use of `r` itself, no direct read
  or write (an instruction on a local or a resource) overlaps `r`'s
  location, and no read or write through any reference is of a location
  strictly enclosing `r`'s location. Reads and writes at or inside `r`'s
  location through other references are allowed, and so are inspections:
  one strictly above `r` cannot conflict with it.
- **D4b″ exclusivity at calls.** For every frame and every `&mut` reference
  parameter `p` of it, every access while the frame is active to a location
  overlapping the location of a later use of `p` or of a reference derived
  from it is through `p` or a reference derived from it. The entry frame's
  parameters are references the transaction lent.
- **D4c immutable stability.** While a `&` reference `r` is live, no step
  writes, through any reference or directly, a location overlapping the
  location of a later use of `r` or of a reference derived from it.
- **D4d no escape.** No reference is stored in a field, a vector element, a
  closure capture or global storage, nor in a value the transaction holds:
  references occur only at the top of locals and operand-stack slots. A
  closure's captured values are its node's children (`Value.children`), so
  "no reference in a capture" is the same check as for fields and elements.
  A returned reference derives from a reference parameter of the
  returning frame; for the entry frame, from a reference the transaction lent.
- **D4e dead slots are unborrowed.** When a local becomes unavailable or its
  frame returns, no live reference points into it.

**Mutable references** (spec-owner decision, 2026-10-05). The language does
not make every live `&mut` exclusive: two `&mut` references a function
borrows itself may designate one location, or one a location inside the
other's. D4b′ and D4b″ are what it guarantees, for three reasons.

1. *Type safety.* A reference must never be used after its location was
   invalidated (`Invalidation`, `State.deref?`): by a write strictly above
   its target, which can change an enum variant or a vector length under
   it; by removing its root (moving a local out, `MoveFrom`); or by the end
   of its frame. Aliasing at or below a target cannot invalidate it: writes
   there are well typed and leave it valid. Such aliasing costs reasoning
   (the holder of a `&mut` may see another reference's writes), not safety.
2. *Coherent whole values.* No value is read or written as a whole while a
   part of it is lent out through a live `&mut`: that would observe, or
   clobber, a part another reference is in charge of. An instruction on a
   local or a resource takes the whole value there, so it may not overlap a
   live `&mut` at all.
3. *Modular reasoning.* A function may assume its `&mut` parameters
   exclusive for the whole call: reasoning about it in isolation relies on
   that. Aliasing among references a function borrows itself is visible in
   its own code, so it does not break modularity.

D4b′ is the first two, which each `&mut` asks on its own location, from its
creation to its own last use. A `&mut` derived from `r` asks the same on its
own location. An `&` derived from `r` is D4c's: reading a value that
contains an `&` target is harmless. The locations of `r`'s derived
references play no part in D4b′: a write before a derived reference is
created does not affect it. D4b″ is the third: full exclusivity, for the
references a frame receives, while the frame is active.

**D4b′, D4c and D4e imply D4a** (`NoDanglingFromContainment`,
`Spec.Properties.Theorems`). A reference becomes invalid only through an
invalidation recorded since its creation, or the end of its frame, and each
is excluded before the reference's last use: a write strictly above a `&mut`
by D4b′, and a write overlapping an `&` by D4c; a removal is a direct write,
excluded by D4b′ or D4c, and for a local also by D4e; a frame end by D4e.
D4b′ is the part of full exclusivity this argument uses: full exclusivity
implies it, since an access through a reference derived from `r`, which full
exclusivity allows, is at or inside `r`'s location.

The cases involving global resources depend on R3 and R4 (plan §7, D4): the
semantics fails a reentrant global storage operation (R3) and a reentrant
call under the module lock (R4), and failures roll back.

**Generic code.** The generic forms of the borrows and of the global storage
operations are annotated as their non-generic forms (`Spec.Semantics.Trace`),
at the instance they name, so every statement here covers generic code
unchanged: locations do not mention types, and a resource's root is its
address and its type with its type arguments (`Root.global`).

**Function values.** `CallClosure` is annotated as `Call` is: the references
among the provided arguments are used, and the target's frame is entered with
them as its reference parameters, so D4b″ protects them while it is active
and D4d asks that the references it returns derive from them. A closure
captures no reference, so a closure call passes no reference it did not
receive on the stack.

**Natives with references** (language-owner rulings, 2026-11-03 and
2026-11-04). A native call is annotated with the events of what it does
through its reference arguments (`Shadow.nativeStep`): it reads the location
of each (it sees the whole value there), writes the location of each mutable
one it writes back, and uses each; a reference it returns for a place is
created, derived from the reference argument the place is inside. So every
statement here covers native calls unchanged: D4b′, D4b″ and D4c judge a
native's reads and writes as those of an instruction acting through the same
reference, D4d asks nothing new (a native has no frame, writes back no
reference and returns references derived from its arguments), and D4a checks
its uses. A native's mutable reference arguments overlap no other reference
argument, immutable ones included, or the call is `wrong`
(`overlappingReferences`; ruling of 2026-11-04). A native sees whole values,
so its reads are whole reads: `vector::length` and `vector::borrow` read the
vector, where `VecLen` and `VecImmBorrow` only inspect it, and D4b′ forbids
them while a `&mut` into an element is still to be used. This stricter
reading is intended (ruling of 2026-11-04): natives have no access kind of
their own and record no inspection, and a borrow-shape native, as
`signer::borrow_address`, reads its arguments' locations too.

**Accounting is total** (`ShadowAccounts`), as for D2 and D3: a step the
ghost layer cannot follow falsifies D4 rather than ending the trace.
**Rollback**: a failing step is not annotated, so D4 is judged on the steps
before termination.
-/

namespace MoveBytecode

namespace Event

/-- The reference an event creates: its identity, value and parent. -/
def borrowed? : Event → Option (Nat × Reference × Option Nat)
  | .borrowed i r parent => some (i, r, parent)
  | .created .. | .consumed .. | .used .. | .accessed .. | .vacated .. | .entered ..
  | .frameEnded .. | .privileged .. => none

/-- The reference an event uses: its identity and value. -/
def used? : Event → Option (Nat × Reference)
  | .used i r => some (i, r)
  | .created .. | .consumed .. | .borrowed .. | .accessed .. | .vacated .. | .entered ..
  | .frameEnded .. | .privileged .. => none

/-- The access an event records: its kind, the location, and the reference
it is through, if any. -/
def accessed? : Event → Option (Access × Location × Option Nat)
  | .accessed a l via => some (a, l, via)
  | .created .. | .consumed .. | .borrowed .. | .used .. | .vacated .. | .entered ..
  | .frameEnded .. | .privileged .. => none

/-- The roots an event ends: a local made unavailable, or every local of a
frame that returned. -/
def ends? : Event → Option (Root → Bool)
  | .vacated root => some (· == root)
  | .frameEnded f _ => some fun
    | .local_ g _ => g == f
    | .global _ _ => false
    | .held _ => false
  | .created .. | .consumed .. | .borrowed .. | .used .. | .accessed .. | .entered ..
  | .privileged .. => none

/-- The frame an event enters, with its reference parameters. -/
def entered? : Event → Option (FrameId × List Nat)
  | .entered f ps => some (f, ps)
  | .created .. | .consumed .. | .borrowed .. | .used .. | .accessed .. | .vacated ..
  | .frameEnded .. | .privileged .. => none

/-- The frame an event returns from. -/
def returned? : Event → Option FrameId
  | .frameEnded f _ => some f
  | .created .. | .consumed .. | .borrowed .. | .used .. | .accessed .. | .vacated ..
  | .entered .. | .privileged .. => none

end Event

/-- Whether an access of kind `a` to location `l` overlaps a reference to
location `m` (module docstring, "Overlap"). -/
def Access.overlaps : Access → Location → Location → Bool
  | .read, l, m | .write, l, m =>
    l.root == m.root && (l.path.isPrefixOf m.path || m.path.isPrefixOf l.path)
  | .inspect, l, m => l.root == m.root && m.path.isPrefixOf l.path

/-- Whether location `l` is strictly inside the value at location `m`. -/
def Location.strictlyInside (l m : Location) : Bool :=
  l.root == m.root && m.path.isPrefixOf l.path && m.path != l.path

/-- Whether an access of kind `a` to location `l`, through reference `via` or
directly (`via = none`), reaches over a `&mut` reference to location `m`
(module docstring, D4b′): a direct read or write that overlaps `m`, or a read
or write through a reference of a location strictly enclosing `m`. -/
def Access.reachesOver (a : Access) (l : Location) (via : Option Nat) (m : Location) : Bool :=
  match a, via with
  | .read, none | .write, none => a.overlaps l m
  | .read, some _ | .write, some _ => m.strictlyInside l
  | .inspect, none | .inspect, some _ => false

/-- A value holds no reference strictly inside it: a reference may be a whole
slot value, never part of one. -/
def Value.noInnerReference (v : Value) : Bool :=
  v.nodes.all fun (p, w) => p.isEmpty || !w.isRef

/-- A value holds no reference at all. -/
def Value.referenceFree (v : Value) : Bool :=
  v.nodes.all fun (_, w) => !w.isRef

namespace RefSafety

/-- How reference `id` was created: its value and the reference it derives
from. -/
def creation? (evs : List Event) (id : Nat) : Option (Reference × Option Nat) :=
  evs.findSome? fun e => e.borrowed?.bind fun (i, r, parent) =>
    if i == id then some (r, parent) else none

/-- Reference `d` derives from reference `r`: `d` is `r`, or the reference it
was derived from derives from `r`. A reference is created after the one it
derives from, so a chain has fewer links than there are events. -/
def derivesFrom (evs : List Event) (d r : Nat) : Bool :=
  go evs.length d
where
  go : Nat → Nat → Bool
    | 0, _ => false
    | n + 1, x =>
      x == r || match creation? evs x with
        | some (_, some parent) => go n parent
        | some (_, none) => false
        | none => false

/-- Reference `r` is used, itself or through a reference derived from it,
after position `p` of `evs`: `r` is live at `p` (module docstring,
"Liveness"). -/
def usedAfter (evs : List Event) (r p : Nat) : Bool :=
  (evs.drop (p + 1)).any fun e => match e.used? with
    | some (d, _) => derivesFrom evs d r
    | none => false

/-- The locations on which reference `r` is live after position `p` of `evs`:
those of the uses, after `p`, of `r` and of the references derived from it. -/
def liveLocations (evs : List Event) (r p : Nat) : List Location :=
  (evs.drop (p + 1)).filterMap fun e => match e.used? with
    | some (d, dr) => if derivesFrom evs d r then some dr.location else none
    | none => none

/-- **D4a** for the events `evs` of one step from state `s`: every reference
the step uses designates a valid location in `s`. -/
def usesValid (s : State) (evs : List Event) : Bool :=
  evs.all fun e => match e.used? with
    | some (_, r) => (s.deref? r.location r.since).isSome
    | none => true

/-- Reference `r` itself is used after position `p` of `evs`; a copy of `r`
is `r`. -/
def usedItselfAfter (evs : List Event) (r p : Nat) : Bool :=
  (evs.drop (p + 1)).any fun e => match e.used? with
    | some (d, _) => d == r
    | none => false

/-- **D4b′** on events `evs`: after the creation of a `&mut` reference `r`, no
access before a later use of `r` itself reaches over `r`'s location. -/
def contained (evs : List Event) : Bool :=
  evs.zipIdx.all fun (e, p) => match e.borrowed? with
    | some (r, ref, _) =>
      !ref.mutable || (evs.zipIdx.drop (p + 1)).all fun (e', q) => match e'.accessed? with
        | some (a, l, via) => !a.reachesOver l via ref.location || !usedItselfAfter evs r q
        | none => true
    | none => true

/-- Whether reference `id` is a `&mut` reference. -/
def isMutable (evs : List Event) (id : Nat) : Bool :=
  match creation? evs id with
  | some (r, _) => r.mutable
  | none => false

/-- The events of `evs`, with their positions, while the frame `f` entered at
position `p` is active: after its entry and before its return, or to the end
of `evs` if it has not returned. -/
def whileActive (evs : List Event) (f : FrameId) (p : Nat) : List (Event × Nat) :=
  (evs.zipIdx.drop (p + 1)).takeWhile fun (e, _) => e.returned? != some f

/-- **D4b″** on events `evs`: while a frame is active, every access that
overlaps a location on which one of its `&mut` reference parameters `r` is
live is through `r` or a reference derived from it. -/
def paramsExclusive (evs : List Event) : Bool :=
  evs.zipIdx.all fun (e, p) => match e.entered? with
    | some (f, params) => params.all fun r =>
      !isMutable evs r || (whileActive evs f p).all fun (e', q) => match e'.accessed? with
        | some (a, l, via) =>
          via.any (derivesFrom evs · r) || (liveLocations evs r q).all (!a.overlaps l ·)
        | none => true
    | none => true

/-- **D4c** on events `evs`: after the creation of a `&` reference `r`, no
write overlaps a location on which `r` is live. -/
def stable (evs : List Event) : Bool :=
  evs.zipIdx.all fun (e, p) => match e.borrowed? with
    | some (r, ref, _) =>
      ref.mutable || (evs.zipIdx.drop (p + 1)).all fun (e', q) => match e'.accessed? with
        | some (.write, l, _) => (liveLocations evs r q).all (!Access.write.overlaps l ·)
        | some (.read, _, _) => true
        | some (.inspect, _, _) => true
        | none => true
    | none => true

/-- **D4e** on events `evs`: when a local becomes unavailable or its frame
returns, no reference created before into that local or frame is live. -/
def deadSlotsUnborrowed (evs : List Event) : Bool :=
  evs.zipIdx.all fun (e, q) => match e.ends? with
    | some ended => (evs.take q).all fun e' => match e'.borrowed? with
      | some (r, ref, _) => !ended ref.location.root || !usedAfter evs r q
      | none => true
    | none => true

/-- The reference parameters of frame `f`. -/
def params? (evs : List Event) (f : FrameId) : Option (List Nat) :=
  evs.findSome? fun e => match e with
    | .entered g ps => if g == f then some ps else none
    | .created .. | .consumed .. | .borrowed .. | .used .. | .accessed .. | .vacated ..
    | .frameEnded .. | .privileged .. => none

/-- **D4d**, returns, on events `evs`: every reference a frame returns derives
from one of the frame's reference parameters. -/
def returnsDerived (evs : List Event) : Bool :=
  evs.all fun e => match e with
    | .frameEnded f rets => match params? evs f with
      | some ps => rets.all fun d => ps.any (derivesFrom evs d ·)
      | none => rets.isEmpty
    | .created .. | .consumed .. | .borrowed .. | .used .. | .accessed .. | .vacated ..
    | .entered .. | .privileged .. => true

/-- **D4d**, storage, on state `s`: references occur only as whole values of
locals and operand-stack slots, never inside a value, in global storage, or
among the values the transaction holds. -/
def unnested (s : State) : Bool :=
  s.frames.all (fun fr => fr.locals.toList.all (fun slot => slot.all Value.noInnerReference) &&
    fr.stack.all Value.noInnerReference) &&
  s.storage.all (·.value.referenceFree) && s.held.all Value.referenceFree

/-- **D4d**, storage, after a step: in the next state, or in the returned
values and the committed global storage. -/
def unnestedAfter : Outcome → Bool
  | .running s => unnested s
  | .returned vs st => vs.all Value.noInnerReference && st.all (·.value.referenceFree)
  | .failed _ => true
  | .wrong _ => true

end RefSafety

/-- **D4a.** Every step from an annotated state uses only references to valid
locations. -/
def NoDanglingUse (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs → ∀ o post evs', TracedStep P N ν s h o post evs' →
    RefSafety.usesValid s evs' = true

/-- **D4b′.** Along every annotated execution, and in its last step, no access
reaches over a `&mut` reference that is still to be used. -/
def MutableContained (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs → RefSafety.contained evs = true ∧
    ∀ o post evs', TracedStep P N ν s h o post evs' → RefSafety.contained (evs ++ evs') = true

/-- **D4b″.** Along every annotated execution, and in its last step, no access
while a frame is active conflicts with a live `&mut` reference parameter of
that frame. -/
def ParametersExclusive (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs → RefSafety.paramsExclusive evs = true ∧
    ∀ o post evs', TracedStep P N ν s h o post evs' →
      RefSafety.paramsExclusive (evs ++ evs') = true

/-- **D4c.** Along every annotated execution, and in its last step, no write
overlaps a live `&` reference. -/
def ImmutableStable (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs → RefSafety.stable evs = true ∧
    ∀ o post evs', TracedStep P N ν s h o post evs' → RefSafety.stable (evs ++ evs') = true

/-- **D4d.** In every annotated state and after every step, no reference is
inside a value, in global storage or among the held values; and every
returned reference derives from a reference parameter. -/
def NoEscape (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs →
    RefSafety.unnested s = true ∧ RefSafety.returnsDerived evs = true ∧
    ∀ o post evs', TracedStep P N ν s h o post evs' →
      RefSafety.unnestedAfter o = true ∧ RefSafety.returnsDerived (evs ++ evs') = true

/-- **D4e.** Along every annotated execution, and in its last step, no local
becomes unavailable, and no frame returns, while a live reference points into
it. -/
def DeadSlotsUnborrowed (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs → RefSafety.deadSlotsUnborrowed evs = true ∧
    ∀ o post evs', TracedStep P N ν s h o post evs' →
      RefSafety.deadSlotsUnborrowed (evs ++ evs') = true

/-- **D4**, reference safety, for executions from `s₀` with natives `N` and
their flows `ν`. Accounting must be total (`ShadowAccounts`) for D4 to cover
every step. -/
def ReferencesSafe (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State) : Prop :=
  ShadowAccounts P N ν s₀ ∧ NoDanglingUse P N ν s₀ ∧ MutableContained P N ν s₀ ∧
    ParametersExclusive P N ν s₀ ∧ ImmutableStable P N ν s₀ ∧ NoEscape P N ν s₀ ∧
    DeadSlotsUnborrowed P N ν s₀

end MoveBytecode
