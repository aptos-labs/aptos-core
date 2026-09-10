-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Computation
import LeanerIR.Proofs.Meaning

/-! Native Move vectors retain their length bound and element type. Length
and checked indexing compute directly on the native array, without encoding
elements as runtime values. -/

namespace LeanerIR.Proofs.NativeVector

def length (values : SpecVector α) : SpecInt (.bits 64) false :=
  ⟨values.values.size, by
    have bound := values.bounded
    simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
    omega⟩

theorem length_val (values : SpecVector α) :
    (length values).val = values.values.size := rfl

/-- Checked native indexing. The error is supplied by the source profile;
successful reads retain the actual element type and its range certificates. -/
def get (values : SpecVector α) (index : Int) (failure : Error) : Spec State Error α :=
  if valid : 0 ≤ index ∧ index < (values.values.size : Int) then
    Spec.pure (values.values[index.toNat]'(by omega))
  else Spec.abort failure

theorem get_state (values : SpecVector α) (index : Int) (failure : Error) :
    StatePreserving (get values index failure : Spec State Error α) := by
  unfold get
  split
  · exact StatePreserving.pure _
  · intro initial result final impossible
    exact impossible.elim

theorem wp_get (values : SpecVector α) (index : Int) (failure : Error)
    (ensures : α → State → Prop) (aborts : Error → Prop) (initial : State) :
    wp (get values index failure) ensures aborts initial ↔
      (∀ valid : 0 ≤ index ∧ index < (values.values.size : Int),
        ensures (values.values[index.toNat]'(by omega)) initial) ∧
      (¬(0 ≤ index ∧ index < (values.values.size : Int)) → aborts failure) := by
  by_cases valid : 0 ≤ index ∧ index < (values.values.size : Int) <;> simp [get, valid]

theorem wp_get_value (values : SpecVector α) (index : Int) (failure : Error)
    (ensures : α → State → Prop) (aborts : Error → Prop) (initial : State) :
    wp (get values index failure) ensures aborts initial ↔
      (∀ valid : 0 ≤ index ∧ index < (values.values.size : Int), ∀ result,
        result = (values.values[index.toNat]'(by omega)) → ensures result initial) ∧
      (¬(0 ≤ index ∧ index < (values.values.size : Int)) → aborts failure) := by
  rw [wp_get]
  simp

/-- Functional write-back after an element borrow's bounds check. The owner
and replacement retain their native types; no loan/frame encoding is needed
to compute the updated vector. -/
def replaceAt (values : SpecVector α) (index : Nat) (replacement : α)
    (valid : index < values.values.size) : SpecVector α :=
  ⟨values.values.set index replacement valid, by simpa using values.bounded⟩

@[simp] theorem replaceAt_size (values : SpecVector α) (index : Nat) (replacement : α)
    (valid : index < values.values.size) :
    (replaceAt values index replacement valid).values.size = values.values.size := by
  simp [replaceAt]

@[simp] theorem replaceAt_length (values : SpecVector α) (index : Nat) (replacement : α)
    (valid : index < values.values.size) :
    length (replaceAt values index replacement valid) = length values := by
  apply SpecInt.ext
  simp [length_val]

@[simp] theorem replaceAt_get_self (values : SpecVector α) (index : Nat) (replacement : α)
    (valid : index < values.values.size) :
    (replaceAt values index replacement valid).values[index]'(by simpa using valid) =
      replacement := by
  simp [replaceAt]

theorem replaceAt_get_other (values : SpecVector α) (index other : Nat) (replacement : α)
    (valid : index < values.values.size) (otherValid : other < values.values.size)
    (distinct : index ≠ other) :
    (replaceAt values index replacement valid).values[other]'(by simpa using otherValid) =
      values.values[other] := by
  simp [replaceAt, Array.getElem_set, distinct]

/-- Checked element assignment, returning the updated native owner. Failure
uses the same profile-selected error as checked element borrowing. In
particular a negative index must not be truncated to a successful index zero. -/
def set (values : SpecVector α) (index : Int) (replacement : α) (failure : Error) :
    Spec State Error (SpecVector α) :=
  if valid : 0 ≤ index ∧ index < (values.values.size : Int) then
    Spec.pure (replaceAt values index.toNat replacement (by omega))
  else Spec.abort failure

theorem set_state (values : SpecVector α) (index : Int) (replacement : α) (failure : Error) :
    StatePreserving (set values index replacement failure : Spec State Error (SpecVector α)) := by
  unfold set
  split
  · exact StatePreserving.pure _
  · intro initial result final impossible
    exact impossible.elim

theorem set_defined (values : SpecVector α) (index : Int) (replacement : α) (failure : Error)
    (initial : State) : ¬(set values index replacement failure).undefined initial := by
  unfold set
  split <;> exact id

theorem wp_set (values : SpecVector α) (index : Int) (replacement : α) (failure : Error)
    (ensures : SpecVector α → State → Prop) (aborts : Error → Prop) (initial : State) :
    wp (set values index replacement failure) ensures aborts initial ↔
      (∀ valid : 0 ≤ index ∧ index < (values.values.size : Int),
        ensures (replaceAt values index.toNat replacement (by omega)) initial) ∧
      (¬(0 ≤ index ∧ index < (values.values.size : Int)) → aborts failure) := by
  by_cases valid : 0 ≤ index ∧ index < (values.values.size : Int) <;> simp [set, valid]

/-- The Move native vector error payload, distinct from library abort codes. -/
def indexFailure (kind : ThrowKind) : Failure := (kind, #[.integer 1])

end LeanerIR.Proofs.NativeVector
