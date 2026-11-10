-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Properties.Natives

/-!
# Ownership accounting D2

TRUSTED. D2 (plan §7) over annotated executions (`Trace`): identities,
occurrences, transfers and consumption are the ghost layer's
(`Spec.Semantics.Trace`); legitimate consumption is `Consumer.Legitimate`.

- **D2a uniqueness.** For types without `copy`, each identity has at most one
  occurrence at any time. A value with `copy` may be copied, and a copy has
  the original's identities ("Any value with the `copy` ability can be copied",
  book, `variables.md`), so D2a does not apply to it.
- **D2b no implicit destruction.** For types without `drop`, an occurrence
  disappears only through a legitimate transfer or consumption. `drop`
  "allows values of types with this ability to be popped/dropped", which is
  "not using the value in a local variable", "not using the value in a
  sequence", and "overwriting values in variables" (book,
  `generics-and-abilities.md`, "`drop`"); `Eq` and `Neq` "consume the value"
  and require `drop` too (book, `equality-and-comparison.md`). Without
  `drop`, a value must be moved on or taken apart. Writing through a
  reference discards the old value, which needs `drop` ("writing to the
  reference will discard (or "drop") the old value", book, `references.md`);
  reading through one copies the value, which needs `copy`.
- **D2c conservation.** After `returned`, every identity of a type without
  `drop` that existed initially (in the transaction's arguments, in the
  values it holds, or in global storage) or was created is in global storage,
  in the return values (none for a transaction: its entry function returns
  nothing, E1), or was consumed legitimately. The storage is the one the
  execution commits (`Outcome.returned`).

**Legitimate transfers** keep identities (`Spec.Semantics.Trace`): moves,
argument passing and returns, packing into a container (a closure's captures
included), vector push, pop and swap, writing a value through a reference,
`MoveTo` and `MoveFrom`, and natives passing a value on, into a result or into
a value they write back through a mutable reference argument (`mem::swap`,
`vector::move_range`, and `table::add_box` and `table::remove_box` moving an
entry into or out of a table). `CallClosure`
consumes the closure and transfers its captured values to the target's
parameters (`Consumer.callClosure`), a legitimate consumption (plan §7). A
native may also destroy, whole, a value the native policy lets it destroy
(`NativePolicy.destroys`): `table::drop_unchecked_box` consumes a table with
its entries (language-owner ruling 3, 2026-11-04).

"Types without `copy`/`drop`" are judged node by node, by each node's own
type (`RuntimeType.abilities`), at its instance: `Box<u64>` may have `copy`
where `Box<R>` does not; nested occurrences count, so copying or
dropping a container copies or drops everything inside it ("If a value has
`copy`, all values contained inside of that value have `copy`", and likewise
`drop`; book, "The Four Abilities").

A type may have `copy` without `drop`: D2a does not apply to it, but D2b and
D2c do. D2b counts occurrences, so discarding one of two copies is an
implicit destruction.

**Rollback.** A step that fails is not annotated: the payload rolls back,
so D2b and D2c are judged on the steps before termination and on `returned`
executions only.

**Accounting is total** (`ShadowAccounts`): every step that continues from
an annotated state is annotated. Without it a ghost that could not follow
some step would make D2 vacuous from that step on.
-/

namespace MoveBytecode

namespace Ownership

/-- How many of `occ` have identity `i`. -/
def count (occ : List (Nat × Value)) (i : Nat) : Nat :=
  occ.countP (·.1 == i)

/-- How many events of `evs` consume identity `i`. -/
def consumptions (evs : List Event) (i : Nat) : Nat :=
  evs.countP fun
    | .consumed j _ _ => j == i
    | .created .. | .borrowed .. | .used .. | .accessed .. | .vacated .. | .entered ..
    | .frameEnded .. | .privileged .. => false

/-- **D2a** on a set of occurrences: an identity occurring at a node whose
type lacks `copy` occurs once. -/
def unique (P : Program) (occ : List (Nat × Value)) : Bool :=
  occ.all fun (i, v) => (v.type.abilities P).copy || count occ i ≤ 1

/-- **D2b** for one step, from occurrences `pre` to `post`, with events
`evs`: every identity occurring at a node whose type lacks `drop` occurs
after the step as often as before, except for the occurrences the step
consumed; and every consumption of a value without `drop` is legitimate. -/
def destroysNothing (P : Program) (π : NativePolicy) (pre post : List (Nat × Value))
    (evs : List Event) : Bool :=
  pre.all (fun (i, v) => (v.type.abilities P).drop || count pre i ≤ count post i + consumptions evs i) &&
    evs.all fun
      | .consumed _ v c => (v.type.abilities P).drop || c.Legitimate π v
      | .created .. | .borrowed .. | .used .. | .accessed .. | .vacated .. | .entered ..
      | .frameEnded .. | .privileged .. => true

/-- **D2c** for an execution with events `evs` that ended with the occurrences
`final` (the returned values and the committed global storage): every
identity created (by the transaction, an earlier transaction for a value in
the initial global storage, an instruction or a native) with a value whose
type lacks `drop` is among them, or was consumed legitimately. -/
def conserved (P : Program) (π : NativePolicy) (evs : List Event) (final : List (Nat × Value)) :
    Bool :=
  evs.all fun
    | .created i v _ =>
      (v.type.abilities P).drop || final.any (·.1 == i) ||
        evs.any fun
          | .consumed j w c => j == i && c.Legitimate π w
          | .created .. | .borrowed .. | .used .. | .accessed .. | .vacated .. | .entered ..
          | .frameEnded .. | .privileged .. => false
    | .consumed .. | .borrowed .. | .used .. | .accessed .. | .vacated .. | .entered ..
    | .frameEnded .. | .privileged .. => true

end Ownership

/-- The ghost layer accounts for every step that continues an annotated
execution from `s₀`: running or returned, core or native. -/
def ShadowAccounts (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs → ∀ o, StepRel P N s o → o.continues = true →
    ∃ post evs', TracedStep P N ν s h o post evs'

/-- **D2a.** In every annotated state, and in the returned values, no
identity of a node whose type lacks `copy` occurs twice. -/
def IdentitiesUnique (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs →
    Ownership.unique P (h.occurrences s) = true ∧
    ∀ o post evs', TracedStep P N ν s h o post evs' →
      Ownership.unique P (post.occurrences o) = true

/-- **D2b.** No step from an annotated state, the returning step included,
destroys an occurrence of a node whose type lacks `drop` other than by a
legitimate consumption. -/
def NoImplicitDestruction (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy)
    (s₀ : State) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs → ∀ o post evs', TracedStep P N ν s h o post evs' →
    Ownership.destroysNothing P π (h.occurrences s) (post.occurrences o) evs' = true

/-- **D2c.** Every annotated execution that returns conserves the identities
of types without `drop`: in the returned values or in the global storage it
commits. -/
def Conservation (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy)
    (s₀ : State) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs → ∀ vals storage post evs',
    TracedStep P N ν s h (.returned vals storage) post evs' →
    Ownership.conserved P π (evs ++ evs') (post.occurrences (.returned vals storage)) = true

/-- **D2**, ownership accounting, for executions from `s₀` with natives `N`,
their flows `ν`, and native policy `π`. -/
def OwnershipAccounted (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy)
    (s₀ : State) : Prop :=
  ShadowAccounts P N ν s₀ ∧ IdentitiesUnique P N ν s₀ ∧ NoImplicitDestruction P N ν π s₀ ∧
    Conservation P N ν π s₀

end MoveBytecode
