-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Semantics.Spec

/-!
# Ownership-passing reference semantics

Immutable references are observations. Mutable references carry their current
value and a ghost prophecy for the value at loan death, following RustHorn's
encoding. Neither representation requires a reference store in `Txn`.
-/

namespace Move.Semantics

/-- Logical representation of an immutable reference. Borrow legality, rather
than this alias, prevents an observation from becoming an illegal owning copy. -/
abbrev ImmRef (α : Type) := α

namespace ImmRef

def deref (reference : ImmRef α) : α := reference

end ImmRef

/-- A mutable loan's checked-out current value and ghost final value. -/
structure Mutation (α : Type) where
  current : α
  prophecy : α
  deriving Repr

namespace Mutation

def read (reference : Mutation α) : α := reference.current

/-- Functional reference assignment. The prophecy remains fixed. -/
def write (reference : Mutation α) (value : α) : Mutation α :=
  { reference with current := value }

/-- A loan is reconciled when its computed current value is the value named by
its prophecy. -/
def Finished (reference : Mutation α) : Prop :=
  reference.current = reference.prophecy

@[simp] theorem read_write (reference : Mutation α) (value : α) :
    (reference.write value).read = value := rfl

@[simp] theorem current_write (reference : Mutation α) (value : α) :
    (reference.write value).current = value := rfl

@[simp] theorem read_mk (current prophecy : α) :
    (Mutation.read { current := current, prophecy := prophecy }) = current := rfl

@[simp] theorem prophecy_write (reference : Mutation α) (value : α) :
    (reference.write value).prophecy = reference.prophecy := rfl

end Mutation

/-- Create a fresh returned loan for a focused value. The second result is the
fresh future which the borrow site installs into its enclosing owner. Keeping
that installation local is what prevents a field or index path from crossing
the function boundary. -/
def reborrowMutation (current : α) : Spec σ (Mutation α × α) where
  ok := fun initial output final =>
    output.1.current = current ∧
    output.2 = output.1.prophecy ∧
    final = initial
  aborts := fun _ _ => False

/-- Move a mutation across a returned-reference boundary.  The returned loan
gets a fresh prophecy, while the poisoned lender keeps its enclosing prophecy
and is suspended at the returned loan's future value.  Resolving the returned
loan therefore revives the lender without prematurely fixing the lender's
eventual value at its enclosing scope. -/
def transferMutation (reference : Mutation α) :
    Spec σ (Mutation α × Mutation α) where
  ok := fun initial output final =>
    output.1.current = reference.current ∧
    output.2.current = output.1.prophecy ∧
    output.2.prophecy = reference.prophecy ∧
    final = initial
  aborts := fun _ _ => False

/-- Resolve an existing mutation without allocating a fresh prophecy.  This
is the operation used when a mutation returned by a function reaches its last
use in the caller. -/
def resolveMutation (reference : Mutation α) : Spec σ Unit where
  ok := fun initial result final =>
    reference.Finished ∧ result = () ∧ final = initial
  aborts := fun _ _ => False

/-- Scope a mutation whose prophecy was chosen before this scope, notably a
mutable reference returned by a call.  In contrast with `withMutation`, this
does not mint a prophecy and does not return an owner value: it only requires
the transferred mutation to resolve when its caller-side live range ends. -/
def withTransferredMutation (reference : Mutation α)
    (body : Mutation α → Spec σ (β × Mutation α)) : Spec σ β :=
  Spec.bind (body reference) fun output =>
    Spec.bind (resolveMutation output.2) fun _ =>
      Spec.pure output.1

/-- A scoped mutable computation written against the prophecy representation. -/
abbrev MutationBody (Owner Result : Type) :=
  Mutation Owner → Result × Mutation Owner

/-- Relational meaning of a mutable loan. `future` is fresh ghost data. The
body determines it by finishing with that current value, and the suspended
owner resumes with it. -/
def ProphecyLoan (body : MutationBody α β)
    (initial : α) (result : β) (final : α) : Prop :=
  ∃ future,
    body { current := initial, prophecy := future } =
      (result, { current := future, prophecy := future }) ∧
    final = future

/-- Concrete ownership passing has no prophecy: it returns the computed final
owner value directly. -/
abbrev ConcreteLoan (Owner Result : Type) := Owner → Result × Owner

/-- Interpret a concrete ownership-passing body without allowing it to inspect
or modify the prophecy. -/
def liftConcrete (body : ConcreteLoan α β) : MutationBody α β := fun reference =>
  let (result, final) := body reference.current
  (result, { current := final, prophecy := reference.prophecy })

/-- Hiding the prophecy recovers ordinary ownership passing exactly. -/
theorem prophecyLoan_liftConcrete_iff (body : ConcreteLoan α β)
    (initial : α) (result : β) (final : α) :
    ProphecyLoan (liftConcrete body) initial result final ↔
      body initial = (result, final) := by
  constructor
  · rintro ⟨future, hbody, rfl⟩
    rcases hconcrete : body initial with ⟨bodyResult, bodyFinal⟩
    simp [liftConcrete, hconcrete] at hbody
    obtain ⟨rfl, rfl⟩ := hbody
    rfl
  · intro hbody
    refine ⟨final, ?_, rfl⟩
    simp [liftConcrete, hbody]

/-- Relational scoped mutable loan. The future value is an existential ghost
on both paths. On normal return it is constrained by the final mutation and
returned as the resumed owner's value. -/
def withMutation (initial : α)
    (body : Mutation α → Spec σ (β × Mutation α)) : Spec σ (β × α) where
  ok := fun initialState output finalState =>
    ∃ (future : α) (reference : Mutation α),
      (body { current := initial, prophecy := future }).ok
        initialState (output.1, reference) finalState ∧
      reference.current = future ∧ output.2 = future
  aborts := fun initialState code =>
    ∃ (future : α),
      (body { current := initial, prophecy := future }).aborts initialState code
  undefined := fun initialState =>
    ∃ (future : α),
      (body { current := initial, prophecy := future }).undefined initialState

/-- Two independent mutable parameters opened together. The result retains
the parameter order, while each mutation has its own prophecy. -/
def withMutations2 (first : α) (second : β)
    (body : Mutation α → Mutation β →
      Spec σ (γ × (Mutation α × Mutation β))) :
    Spec σ (γ × (α × β)) where
  ok := fun initialState output finalState =>
    ∃ (firstFuture : α) (secondFuture : β)
      (firstReference : Mutation α) (secondReference : Mutation β),
      (body { current := first, prophecy := firstFuture }
        { current := second, prophecy := secondFuture }).ok initialState
          (output.1, (firstReference, secondReference)) finalState ∧
      firstReference.current = firstFuture ∧
      secondReference.current = secondFuture ∧
      output.2 = (firstFuture, secondFuture)
  aborts := fun initialState code =>
    ∃ (firstFuture : α) (secondFuture : β),
      (body { current := first, prophecy := firstFuture }
        { current := second, prophecy := secondFuture }).aborts initialState code
  undefined := fun initialState =>
    ∃ (firstFuture : α) (secondFuture : β),
      (body { current := first, prophecy := firstFuture }
        { current := second, prophecy := secondFuture }).undefined initialState

/-- Three independent mutable parameters opened together.  Products use the
same right-nested order as source argument and result tuples. -/
def withMutations3 (first : α) (second : β) (third : γ)
    (body : Mutation α → Mutation β → Mutation γ →
      Spec σ (δ × (Mutation α × (Mutation β × Mutation γ)))) :
    Spec σ (δ × (α × (β × γ))) where
  ok := fun initialState output finalState =>
    ∃ (firstFuture : α) (secondFuture : β) (thirdFuture : γ)
      (firstReference : Mutation α) (secondReference : Mutation β)
      (thirdReference : Mutation γ),
      (body { current := first, prophecy := firstFuture }
        { current := second, prophecy := secondFuture }
        { current := third, prophecy := thirdFuture }).ok initialState
          (output.1, (firstReference, (secondReference, thirdReference))) finalState ∧
      firstReference.current = firstFuture ∧
      secondReference.current = secondFuture ∧
      thirdReference.current = thirdFuture ∧
      output.2 = (firstFuture, (secondFuture, thirdFuture))
  aborts := fun initialState code =>
    ∃ (firstFuture : α) (secondFuture : β) (thirdFuture : γ),
      (body { current := first, prophecy := firstFuture }
        { current := second, prophecy := secondFuture }
        { current := third, prophecy := thirdFuture }).aborts initialState code
  undefined := fun initialState =>
    ∃ (firstFuture : α) (secondFuture : β) (thirdFuture : γ),
      (body { current := first, prophecy := firstFuture }
        { current := second, prophecy := secondFuture }
        { current := third, prophecy := thirdFuture }).undefined initialState

/-! Arbitrary heterogeneous mutable-parameter bundles. `Tuple` deliberately
reduces to the source translator's existing right-nested product convention,
including the singleton case without a wrapper. -/

abbrev Tuple : List Type → Type
  | [] => Unit
  | [type] => type
  | type :: next :: rest => type × Tuple (next :: rest)

abbrev MutationTuple : List Type → Type
  | [] => Unit
  | [type] => Mutation type
  | type :: next :: rest => Mutation type × MutationTuple (next :: rest)

@[simp] def openMutations : (types : List Type) →
    Tuple types → Tuple types → MutationTuple types
  | [], (), () => ()
  | [_], current, future => { current, prophecy := future }
  | _ :: next :: rest, (current, currents), (future, futures) =>
      ({ current, prophecy := future }, openMutations (next :: rest) currents futures)

@[simp] def mutationCurrents : (types : List Type) → MutationTuple types → Tuple types
  | [], () => ()
  | [_], reference => reference.current
  | _ :: next :: rest, (reference, references) =>
      (reference.current, mutationCurrents (next :: rest) references)

/-- Open any number of heterogeneous mutable parameters together. The two
layout maps keep the externally visible owner product independent of the
recursive type family. Besides preserving the source tuple convention, this
lets WP theorem matching instantiate `Owners` without unfolding `Tuple` at a
restricted transparency level. Generated uses supply the identity maps. -/
def withMutations {types : List Type} {Owners : Type}
    (toTuple : Owners → Tuple types) (fromTuple : Tuple types → Owners)
    (owners : Owners)
    (body : MutationTuple types → Spec σ (β × MutationTuple types)) :
    Spec σ (β × Owners) where
  ok := fun initialState output finalState =>
    ∃ futures references,
      (body (openMutations types (toTuple owners) futures)).ok initialState
        (output.1, references) finalState ∧
      mutationCurrents types references = futures ∧
      output.2 = fromTuple futures
  aborts := fun initialState code =>
    ∃ futures,
      (body (openMutations types (toTuple owners) futures)).aborts initialState code
  undefined := fun initialState =>
    ∃ futures,
      (body (openMutations types (toTuple owners) futures)).undefined initialState

@[simp] theorem withMutation_ok (initial : α)
    (body : Mutation α → Spec σ (β × Mutation α)) :
    (withMutation initial body).ok state output finalState ↔
      ∃ (future : α) (reference : Mutation α),
        (body { current := initial, prophecy := future }).ok
          state (output.1, reference) finalState ∧
        reference.current = future ∧ output.2 = future := Iff.rfl

@[simp] theorem withMutation_aborts (initial : α)
    (body : Mutation α → Spec σ (β × Mutation α)) :
    (withMutation initial body).aborts state code ↔
      ∃ (future : α),
        (body { current := initial, prophecy := future }).aborts state code :=
  Iff.rfl

/-- Pointwise well-definedness of a scoped mutation. -/
theorem withMutation_defined {initial : α} {state : σ}
    {body : Mutation α → Spec σ (β × Mutation α)}
    (scope : ∀ reference, ¬(body reference).undefined state) :
    ¬(withMutation initial body).undefined state := by
  rintro ⟨future, obligation⟩
  exact scope _ obligation

theorem withMutation_total {initial : α}
    {body : Mutation α → Spec σ (β × Mutation α)}
    (total : ∀ reference, Spec.Total (body reference)) :
    Spec.Total (withMutation initial body) := by
  rintro state ⟨future, obligation⟩
  exact total _ state obligation

theorem withMutation_undefined (initial : α)
    (body : Mutation α → Spec σ (β × Mutation α)) (state : σ) :
    (withMutation initial body).undefined state ↔
      ∃ future, (body { current := initial, prophecy := future }).undefined state :=
  Iff.rfl

/-- A total functional lens used for field and already-bounds-checked vector
element reborrows. The laws make nested prophecy reconciliation provable. -/
structure Lens (Owner Focus : Type) where
  get : Owner → Focus
  set : Owner → Focus → Owner
  get_set : ∀ owner value, get (set owner value) = value
  set_get : ∀ owner, set owner (get owner) = owner
  set_set : ∀ owner first second, set (set owner first) second = set owner second

namespace Mutation

/-- The two values created by a prophecy reborrow: the active child and the
parent suspended with the child's future value already installed. -/
structure Split (Owner Focus : Type) where
  child : Mutation Focus
  suspendedParent : Mutation Owner
  deriving Repr

def split (lens : Lens α β) (parent : Mutation α) (childFuture : β) : Split α β :=
  { child := { current := lens.get parent.current, prophecy := childFuture }
    suspendedParent := {
      current := lens.set parent.current childFuture
      prophecy := parent.prophecy } }

/-- Concrete child write-back into the checked-out parent value. -/
def closeChild (lens : Lens α β) (parent : Mutation α)
    (child : Mutation β) : Mutation α :=
  { current := lens.set parent.current child.current
    prophecy := parent.prophecy }

/-- Once the child satisfies its prophecy, concrete write-back equals the
parent value which was suspended at reborrow creation. -/
theorem closeChild_eq_suspended (lens : Lens α β) (parent : Mutation α)
    (childFuture : β) (child : Mutation β)
    (hcurrent : child.current = childFuture) :
    closeChild lens parent child =
      (split lens parent childFuture).suspendedParent := by
  cases parent
  cases child
  simp_all [closeChild, split]

end Mutation

end Move.Semantics
