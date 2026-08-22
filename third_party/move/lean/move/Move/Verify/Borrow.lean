-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Semantics.Vector
import Move.Verify.Contract

/-!
# Prophecy proof rules

Small derived rules for proving scoped mutable computations without exposing
the existential prophecy in routine source proofs.
-/

namespace Move.Verify

open Move.Semantics

/-- Weakest precondition of a relational prophecy loan. -/
def mutationWP (initial : α) (body : MutationBody α β)
    (post : β → α → Prop) : Prop :=
  ∀ result final, ProphecyLoan body initial result final → post result final

/-- Assignment through a mutable reference as a scoped body. -/
def assignBody (value : α) : MutationBody α Unit := fun reference =>
  ((), reference.write value)

/-- Reading without writing as a scoped body. -/
def readBody : MutationBody α α := fun reference =>
  (reference.read, reference)

/-- Relational assignment body used by the authoritative `Spec` semantics. -/
def assignSpecBody (value : α) (reference : Mutation α) :
    Spec σ (Unit × Mutation α) :=
  Spec.pure ((), reference.write value)

/-- Relational read body used by the authoritative `Spec` semantics. -/
def readSpecBody (reference : Mutation α) : Spec σ (α × Mutation α) :=
  Spec.pure (reference.read, reference)

/-- Derived proof rule for `vector::insert` through a scoped mutable borrow. -/
theorem withMutation_insertSpec_eq_pure (values : Move.Vector α)
    (index : Move.U64) (value : α)
    (room : index.toNat ≤ values.toList.length ∧
      values.toList.length + 1 < Move.U64.size) :
    withMutation values
        (fun reference =>
          Spec.bind (Move.Semantics.Vector.insertSpec reference index value)
            (fun output => Spec.pure ((), output.2))) =
      (Spec.pure
        ((), Move.Vector.ofList
          (values.toList.take index.toNat ++
            value :: values.toList.drop index.toNat)
          (by
            have inBounds := room.1
            have hasRoom : values.toList.length + 1 <
                MoveModel.IR.IntWidth.size .w64 := room.2
            simp only [List.length_append, List.length_cons,
              List.length_take, List.length_drop]
            omega)) :
        Spec σ (Unit × Move.Vector α)) := by
  apply Spec.extensionality
  · funext state output finalState
    apply propext
    simp [withMutation, Move.Semantics.Vector.insertSpec, room.1, room.2,
      Mutation.read, Mutation.write, Spec.bind, Spec.pure]
    constructor
    · rintro ⟨future, reference, ⟨rfl, stateEq⟩,
          currentEq, outputEq⟩
      subst future
      exact ⟨by ext; simp_all, stateEq⟩
    · rintro ⟨rfl, rfl⟩
      refine ⟨_, _, ⟨rfl, rfl⟩, rfl, rfl⟩
  · funext state code
    apply propext
    simp [withMutation, Move.Semantics.Vector.insertSpec, room.1, room.2,
      Mutation.read, Mutation.write, Spec.bind, Spec.pure]

  · funext state
    apply propext
    constructor
    · intro obligation
      exact absurd obligation (by
        apply Move.Semantics.withMutation_total
        intro reference
        apply Move.Semantics.Spec.Total.bind
        · first
            | apply Move.Semantics.Vector.insertSpec_total
            | apply Move.Semantics.Vector.removeSpec_total
        · exact fun _ _ contradiction => contradiction.elim)
    · exact fun contradiction => contradiction.elim
/-- Abort-side proof rule for `vector::insert` through a scoped mutable
borrow: out of bounds or out of the `u64` length domain. -/
theorem withMutation_insertSpec_eq_abort (values : Move.Vector α)
    (index : Move.U64) (value : α)
    (noRoom : ¬(index.toNat ≤ values.toList.length ∧
      values.toList.length + 1 < Move.U64.size)) :
    withMutation values
        (fun reference =>
          Spec.bind (Move.Semantics.Vector.insertSpec reference index value)
            (fun output => Spec.pure ((), output.2))) =
      (Spec.abort Move.Semantics.Vector.indexOutOfBounds :
        Spec σ (Unit × Move.Vector α)) := by
  apply Spec.extensionality
  · funext state output finalState
    apply propext
    simp [withMutation, Move.Semantics.Vector.insertSpec, noRoom,
      Mutation.read, Spec.bind, Spec.pure, Spec.abort]
  · funext state code
    apply propext
    simp [withMutation, Move.Semantics.Vector.insertSpec, noRoom,
      Mutation.read, Spec.bind, Spec.pure, Spec.abort]

  · funext state
    apply propext
    constructor
    · intro obligation
      exact absurd obligation (by
        apply Move.Semantics.withMutation_total
        intro reference
        apply Move.Semantics.Spec.Total.bind
        · first
            | apply Move.Semantics.Vector.insertSpec_total
            | apply Move.Semantics.Vector.removeSpec_total
        · exact fun _ _ contradiction => contradiction.elim)
    · exact fun contradiction => contradiction.elim
/-- Derived proof rule for `vector::remove` through a scoped mutable borrow. -/
theorem withMutation_removeSpec_eq_pure (values : Move.Vector α)
    (index : Move.U64) (removed : α) (project : α → β)
    (present : values.toList[index.toNat]? = some removed) :
    withMutation values
        (fun reference =>
          Spec.bind (Move.Semantics.Vector.removeSpec reference index)
            (fun output => Spec.pure (project output.1, output.2))) =
      (Spec.pure
        (project removed, Move.Vector.ofList
          (values.toList.take index.toNat ++
            values.toList.drop (index.toNat + 1))
          (by
            have bounded : values.toList.length <
                MoveModel.IR.IntWidth.size .w64 := values.toList_length_lt
            simp only [List.length_append, List.length_take,
              List.length_drop]
            omega)) :
        Spec σ (β × Move.Vector α)) := by
  apply Spec.extensionality
  · funext state output finalState
    apply propext
    simp [withMutation, Move.Semantics.Vector.removeSpec, present,
      Mutation.read, Mutation.write, Spec.bind, Spec.pure]
    grind
  · funext state code
    apply propext
    simp [withMutation, Move.Semantics.Vector.removeSpec, present,
      Mutation.read, Mutation.write, Spec.bind, Spec.pure]

  · funext state
    apply propext
    constructor
    · intro obligation
      exact absurd obligation (by
        apply Move.Semantics.withMutation_total
        intro reference
        apply Move.Semantics.Spec.Total.bind
        · first
            | apply Move.Semantics.Vector.insertSpec_total
            | apply Move.Semantics.Vector.removeSpec_total
        · exact fun _ _ contradiction => contradiction.elim)
    · exact fun contradiction => contradiction.elim
/-- Abort-side proof rule for `vector::remove` through a scoped mutable
borrow. -/
theorem withMutation_removeSpec_eq_abort (values : Move.Vector α)
    (index : Move.U64) (project : α → β)
    (missing : values.toList[index.toNat]? = none) :
    withMutation values
        (fun reference =>
          Spec.bind (Move.Semantics.Vector.removeSpec reference index)
            (fun output => Spec.pure (project output.1, output.2))) =
      (Spec.abort Move.Semantics.Vector.indexOutOfBounds :
        Spec σ (β × Move.Vector α)) := by
  apply Spec.extensionality
  · funext state output finalState
    apply propext
    simp [withMutation, Move.Semantics.Vector.removeSpec, missing,
      Mutation.read, Spec.bind, Spec.pure, Spec.abort]
  · funext state code
    apply propext
    simp [withMutation, Move.Semantics.Vector.removeSpec, missing,
      Mutation.read, Spec.bind, Spec.pure, Spec.abort]

  · funext state
    apply propext
    constructor
    · intro obligation
      exact absurd obligation (by
        apply Move.Semantics.withMutation_total
        intro reference
        apply Move.Semantics.Spec.Total.bind
        · first
            | apply Move.Semantics.Vector.insertSpec_total
            | apply Move.Semantics.Vector.removeSpec_total
        · exact fun _ _ contradiction => contradiction.elim)
    · exact fun contradiction => contradiction.elim
/-- Weakest precondition of `vector::insert` through a scoped mutable borrow:
the same two-branch shape as the checked arithmetic rules, so one tactic
splits every checked operation. -/
@[wp_norm high] theorem wp_withMutation_insertSpec (values : Move.Vector α)
    (index : Move.U64) (value : α)
    (ensures : (Unit × Move.Vector α) → σ → Prop) (aborts : Nat → Prop)
    (state : σ) :
    wp (withMutation values
        (fun reference =>
          Spec.bind (Move.Semantics.Vector.insertSpec reference index value)
            (fun output => Spec.pure ((), output.2))))
        ensures aborts state ↔
      (∀ room : index.toNat ≤ values.toList.length ∧
          values.toList.length + 1 < Move.U64.size,
        ensures
          ((), Move.Vector.ofList
            (values.toList.take index.toNat ++
              value :: values.toList.drop index.toNat)
            (by
              have inBounds := room.1
              have hasRoom : values.toList.length + 1 <
                  MoveModel.IR.IntWidth.size .w64 := room.2
              simp only [List.length_append, List.length_cons,
                List.length_take, List.length_drop]
              omega))
          state) ∧
      (¬(index.toNat ≤ values.toList.length ∧
          values.toList.length + 1 < Move.U64.size) →
        aborts Move.Semantics.Vector.indexOutOfBounds) := by
  by_cases room : index.toNat ≤ values.toList.length ∧
      values.toList.length + 1 < Move.U64.size
  · rw [withMutation_insertSpec_eq_pure values index value room]
    simp only [wp_pure]
    exact ⟨fun holds => ⟨fun _ => holds, fun noRoom => absurd room noRoom⟩,
      fun holds => holds.1 room⟩
  · rw [withMutation_insertSpec_eq_abort values index value room]
    simp only [wp_abort]
    exact ⟨fun holds =>
        ⟨fun contradiction => absurd contradiction room, fun _ => holds⟩,
      fun holds => holds.2 room⟩

/-- Weakest precondition of `vector::remove` through a scoped mutable borrow.
The removed element is produced by the success branch. -/
@[wp_norm high] theorem wp_withMutation_removeSpec (values : Move.Vector α)
    (index : Move.U64) (project : α → β)
    (ensures : (β × Move.Vector α) → σ → Prop) (aborts : Nat → Prop)
    (state : σ) :
    wp (withMutation values
        (fun reference =>
          Spec.bind (Move.Semantics.Vector.removeSpec reference index)
            (fun output => Spec.pure (project output.1, output.2))))
        ensures aborts state ↔
      (∀ removed, values.toList[index.toNat]? = some removed →
        ensures
          (project removed, Move.Vector.ofList
            (values.toList.take index.toNat ++
              values.toList.drop (index.toNat + 1))
            (by
              have bounded : values.toList.length <
                  MoveModel.IR.IntWidth.size .w64 := values.toList_length_lt
              simp only [List.length_append, List.length_take,
                List.length_drop]
              omega))
          state) ∧
      (values.toList[index.toNat]? = none →
        aborts Move.Semantics.Vector.indexOutOfBounds) := by
  cases present : values.toList[index.toNat]? with
  | none =>
      rw [withMutation_removeSpec_eq_abort values index project present]
      simp only [wp_abort]
      exact ⟨fun holds => ⟨fun actual equal => by simp at equal, fun _ => holds⟩,
        fun holds => holds.2 trivial⟩
  | some removed =>
      rw [withMutation_removeSpec_eq_pure values index removed project present]
      simp only [wp_pure]
      constructor
      · refine fun holds =>
          ⟨fun actual equal => ?_, fun missing => by simp at missing⟩
        injection equal with same
        subst same
        exact holds
      · intro holds
        exact holds.1 removed rfl

theorem prophecyLoan_assign_iff (initial value final : α) :
    ProphecyLoan (assignBody value) initial () final ↔ final = value := by
  constructor
  · rintro ⟨future, hbody, rfl⟩
    simp [assignBody, Mutation.write] at hbody
    exact hbody.symm
  · intro hfinal
    subst final
    exact ⟨value, by simp [assignBody, Mutation.write]⟩

theorem prophecyLoan_read_iff (initial result final : α) :
    ProphecyLoan readBody initial result final ↔
      result = initial ∧ final = initial := by
  constructor
  · rintro ⟨future, hbody, rfl⟩
    simp [readBody, Mutation.read] at hbody
    exact ⟨hbody.1.symm, hbody.2.symm⟩
  · rintro ⟨hresult, hfinal⟩
    subst result
    subst final
    exact ⟨initial, by simp [readBody, Mutation.read]⟩

@[simp] theorem mutationWP_assign (initial value : α) (post : Unit → α → Prop) :
    mutationWP initial (assignBody value) post ↔ post () value := by
  constructor
  · intro h
    apply h () value
    exact (prophecyLoan_assign_iff initial value value).2 rfl
  · intro h _ final hloan
    have : final = value := (prophecyLoan_assign_iff initial value final).1 hloan
    simpa [this] using h

@[simp] theorem mutationWP_read (initial : α) (post : α → α → Prop) :
    mutationWP initial readBody post ↔ post initial initial := by
  constructor
  · intro h
    apply h initial initial
    exact (prophecyLoan_read_iff initial initial initial).2 ⟨rfl, rfl⟩
  · intro h result final hloan
    obtain ⟨rfl, rfl⟩ := (prophecyLoan_read_iff initial result final).1 hloan
    exact h

@[simp] theorem withMutation_assign_ok_iff (initial value : α)
    (state finalState : σ) (output : Unit × α) :
    (withMutation initial (assignSpecBody value)).ok state output finalState ↔
      output = ((), value) ∧ finalState = state := by
  simp only [withMutation, assignSpecBody, Spec.pure, Mutation.write,
    Prod.mk.injEq, true_and]
  constructor
  · rintro ⟨future, reference, ⟨href, hstate⟩, hcurrent, houtput⟩
    subst reference
    simp only at hcurrent
    subst future
    constructor
    · apply Prod.ext <;> simp_all
    · exact hstate
  · rintro ⟨rfl, rfl⟩
    exact ⟨value, { current := value, prophecy := value }, ⟨rfl, rfl⟩, rfl, rfl⟩

@[simp] theorem withMutation_assign_not_aborts (initial value : α)
    (state : σ) (code : Nat) :
    ¬(withMutation initial (assignSpecBody value)).aborts state code := by
  simp [withMutation, assignSpecBody, Spec.pure]

/-- Assignment through a mutable vector-element borrow is a checked functional
update. The body shape is the one generated for `let r ← &mut v[i]; r := x`. -/
theorem withBorrowElemMutSpec_write_eq_pure {values : Move.Vector α}
    {index : Move.U64} {value old : α}
    (present : values.toList[index.toNat]? = some old) :
    (Move.Semantics.Vector.withBorrowElemMutSpec values index
        (fun reference => Spec.pure ((), reference.write value)) :
      Spec σ (Unit × Move.Vector α)) =
      Spec.pure ((), Move.Vector.set values index value) := by
  have bodyEq :
      (fun reference =>
        (Spec.pure ((), reference.write value) : Spec σ (Unit × Mutation α))) =
        assignSpecBody value := rfl
  rw [bodyEq]
  apply Spec.extensionality
  · funext initial output final
    apply propext
    simp only [Move.Semantics.Vector.withBorrowElemMutSpec, present]
    change (Spec.bind (withMutation old (assignSpecBody value)) fun output =>
      Spec.pure (output.1, Move.Vector.set values index output.2)).ok
        initial output final ↔ _
    simp only [Spec.bind, Spec.pure, withMutation_assign_ok_iff]
    constructor
    · rintro ⟨bodyOutput, middle, ⟨rfl, rfl⟩, rfl, rfl⟩
      exact ⟨rfl, rfl⟩
    · rintro ⟨rfl, rfl⟩
      exact ⟨((), value), _, ⟨rfl, rfl⟩, rfl, rfl⟩
  · funext initial code
    apply propext
    simp only [Move.Semantics.Vector.withBorrowElemMutSpec, present]
    change (Spec.bind (withMutation old (assignSpecBody value)) fun output =>
      Spec.pure (output.1, Move.Vector.set values index output.2)).aborts
        initial code ↔ _
    simp only [Spec.bind, Spec.pure, withMutation_assign_ok_iff,
      withMutation_assign_not_aborts]
    constructor
    · rintro (impossible | ⟨bodyOutput, middle, ⟨rfl, rfl⟩, impossible⟩) <;>
        exact impossible.elim
    · exact fun impossible => impossible.elim
  · funext state
    apply propext
    simp only [Move.Semantics.Vector.withBorrowElemMutSpec, present]
    change (Spec.bind (withMutation old (assignSpecBody value)) fun output =>
      Spec.pure (output.1, Move.Vector.set values index output.2)).undefined
        state ↔ _
    constructor
    · intro obligation
      refine absurd obligation (Spec.Total.bind ?_ ?_ state)
      · exact Move.Semantics.withMutation_total fun _ _ h => h.elim
      · exact fun _ _ h => h.elim
    · exact fun contradiction => contradiction.elim

@[simp] theorem withMutation_read_ok_iff (initial : α)
    (state finalState : σ) (output : α × α) :
    (withMutation initial readSpecBody).ok state output finalState ↔
      output = (initial, initial) ∧ finalState = state := by
  simp only [withMutation, readSpecBody, Spec.pure, Mutation.read,
    Prod.mk.injEq]
  constructor
  · rintro ⟨future, reference, ⟨⟨hresult, href⟩, hstate⟩, hcurrent, houtput⟩
    subst reference
    simp only at hcurrent
    subst future
    constructor
    · apply Prod.ext <;> simp_all
    · exact hstate
  · rintro ⟨rfl, rfl⟩
    exact ⟨initial, { current := initial, prophecy := initial },
      ⟨⟨rfl, rfl⟩, rfl⟩, rfl, rfl⟩

@[simp] theorem withMutation_read_not_aborts (initial : α)
    (state : σ) (code : Nat) :
    ¬(withMutation initial readSpecBody).aborts state code := by
  simp [withMutation, readSpecBody, Spec.pure]

@[simp] theorem wp_withMutation_assign (initial value : α)
    (state : σ) (ensures : (Unit × α) → σ → Prop) (aborts : Nat → Prop) :
    wp (withMutation initial (assignSpecBody value)) ensures aborts state ↔
      ensures ((), value) state := by
  simp only [wp, withMutation_assign_ok_iff, withMutation_assign_not_aborts]
  constructor
  · exact fun holds => holds.1 ((), value) state ⟨rfl, rfl⟩
  · refine fun holds => ⟨?_, fun code absurd => absurd.elim,
      Move.Semantics.withMutation_total
        (body := assignSpecBody value) (fun _ _ h => h.elim) state⟩
    rintro output final ⟨rfl, rfl⟩
    exact holds

@[simp] theorem wp_withMutation_read (initial : α)
    (state : σ) (ensures : (α × α) → σ → Prop) (aborts : Nat → Prop) :
    wp (withMutation initial readSpecBody) ensures aborts state ↔
      ensures (initial, initial) state := by
  simp only [wp, withMutation_read_ok_iff]
  constructor
  · exact fun holds => holds.1 (initial, initial) state ⟨rfl, rfl⟩
  · refine fun holds => ⟨?_, ?_,
      Move.Semantics.withMutation_total
        (body := readSpecBody) (fun _ _ h => h.elim) state⟩
    · rintro output final ⟨rfl, rfl⟩
      exact holds
    · rintro code ⟨future, absurd⟩
      exact absurd.elim

end Move.Verify
