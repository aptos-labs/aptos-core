-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeVector
import LeanerIR.Proofs.Denotation
import LeanerIR.Proofs.NativeValueAgreement

namespace LeanerIR.Proofs.NativeVector

open LeanerIR.SemanticOperations Denotation ComputationAgreement

theorem length_evaluate (codec : Codec α RuntimeValue) (values : SpecVector α)
    (frame : RuntimeFrame) (state : RuntimeState) :
    (Denotation.PrimitiveLocationOperation.length (.integer (.bits 64) false)).evaluate?
      #[(Codec.boundedVector codec).encode values] frame state =
      some (.value frame state (.integer (length values).val)) := by
  have bound := values.bounded
  have residue : ((values.values.size : Int) % 18446744073709551616) = values.values.size :=
    Int.emod_eq_of_lt (by omega) (by omega)
  simp [Denotation.PrimitiveLocationOperation.evaluate?, Denotation.liftPrimitiveEvaluator,
    Codec.boundedVector_encode, SemanticOperations.modularInteger, length_val, residue]

theorem check_index_success (codec : Codec α RuntimeValue) (values : SpecVector α)
    (index : Int) (kind : ThrowKind) (frame : RuntimeFrame) (state : RuntimeState)
    (valid : 0 ≤ index ∧ index < (values.values.size : Int)) :
    (PrimitiveLocationOperation.checkVectorIndex kind).evaluate?
      #[(Codec.boundedVector codec).encode values, .integer index] frame state =
      some (.value frame state .unit) := by
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator, checkVectorIndex?,
    Codec.boundedVector_encode, valid]

/-- A specification projection of a represented vector reads the same native
element. The explicit bounds premise prevents default-valued invalid reads. -/
theorem field_encode (codec : Codec α RuntimeValue) (values : SpecVector α)
    (index : Int) (valid : 0 ≤ index ∧ index < (values.values.size : Int)) :
    ((Codec.boundedVector codec).encode values).field index.toNat =
      codec.encode (values.values[index.toNat]'(by omega)) := by
  have bound : index.toNat < values.values.size := by omega
  simp [Codec.boundedVector_encode, RuntimeValue.field, bound]

theorem check_index_failure (codec : Codec α RuntimeValue) (values : SpecVector α)
    (index : Int) (kind : ThrowKind) (frame : RuntimeFrame) (state : RuntimeState)
    (invalid : ¬(0 ≤ index ∧ index < (values.values.size : Int))) :
    (PrimitiveLocationOperation.checkVectorIndex kind).evaluate?
      #[(Codec.boundedVector codec).encode values, .integer index] frame state =
      some (.throw_ frame state kind #[.integer 1]) := by
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator, checkVectorIndex?,
    Codec.boundedVector_encode, invalid]

/-- Exact write-back through the interpreter's resolved vector-element path.
Only the representation theorem maps native elements to runtime values. -/
theorem replaceAt_write (codec : Codec α RuntimeValue) (values : SpecVector α)
    (index : Nat) (replacement : α) (valid : index < values.values.size) :
    writeProjections? ((Codec.boundedVector codec).encode values) [.index index]
      (codec.encode replacement) =
      some ((Codec.boundedVector codec).encode (replaceAt values index replacement valid)) := by
  simp [writeProjections?, Codec.boundedVector_encode, replaceAt, valid,
    Array.set!, Array.setIfInBounds]

/-- The checked update has exactly the profile's element-borrow failures,
including signed negative indices. No invalid native write becomes a no-op. -/
theorem set_aborts (values : SpecVector α) (index : Int) (replacement : α)
    (failure : Error) (initial : State) (error : Error) :
    (set values index replacement failure).aborts initial error ↔
      ¬(0 ≤ index ∧ index < (values.values.size : Int)) ∧ error = failure := by
  by_cases valid : 0 ≤ index ∧ index < (values.values.size : Int) <;>
    simp [set, valid, Spec.pure, Spec.abort]

/-- Successful native assignment is exactly the existing projection writer,
with the same vector bounds check. The final store is unchanged because the
updated owner is returned explicitly, ready for local/loan reconciliation. -/
theorem set_ok (codec : Codec α RuntimeValue) (values : SpecVector α)
    (index : Int) (replacement : α) (failure : Error)
    (initial final : State) (result : SpecVector α) :
    (set values index replacement failure).ok initial result final ↔
      (0 ≤ index ∧ index < (values.values.size : Int)) ∧
      writeProjections? ((Codec.boundedVector codec).encode values) [.index index.toNat]
        (codec.encode replacement) = some ((Codec.boundedVector codec).encode result) ∧
      final = initial := by
  by_cases valid : 0 ≤ index ∧ index < (values.values.size : Int)
  · rw [show (set values index replacement failure : Spec State Error (SpecVector α)) =
        Spec.pure (replaceAt values index.toNat replacement (by omega)) from dif_pos valid]
    rw [replaceAt_write codec values index.toNat replacement (by omega)]
    have injective := (Codec.boundedVector codec).encode_injective
    simp only [Spec.pure, valid, true_and, Option.some.injEq, injective.eq_iff]
    exact and_congr_left (fun _ => eq_comm)
  · simp [set, valid, Spec.abort]

theorem indexed_dynamic_shared (codec : Codec α RuntimeValue) (values : SpecVector α)
    (frame : RuntimeFrame) (localId indexId : LocalId) (index : Int)
    (referenceType : ReferenceType) (lexical : Nat) (unusedIndex : Nat)
    (shared : referenceType.kind = .shared)
    (present : readLocal? frame localId = some ((Codec.boundedVector codec).encode values))
    (indexPresent : readLocal? frame indexId = some (.integer index))
    (inBounds : localId.index < frame.locals.size)
    (valid : 0 ≤ index ∧ index < (values.values.size : Int)) :
    Returns (nativeIndexedLocalBorrowOperation
      ⟨⟨localId⟩, false, unusedIndex, referenceType, .immutable, lexical, some indexId⟩ valuesNil)
      frame frame (codec.encode (values.values[index.toNat]'(by omega))) := by
  apply nativeOperation_value
  · exact nil frame
  · intro state
    have nonnegative : ¬index < 0 := by omega
    have elementBound : index.toNat < values.values.size := by omega
    simp [IndexedLocalBorrowOperation.evaluate?, IndexedLocalBorrowOperation.resolve?,
      resolveLocalIndex?, liftPlaceEvaluator, Nat.not_le_of_lt inBounds,
      borrowRuntimePlaceAt?, shared, readRuntimePlace?, readRoot?, readProjections?, present,
      indexPresent, nonnegative, Codec.boundedVector_encode,
      show (ReferenceKind.shared != ReferenceKind.shared) = false from rfl, elementBound]

theorem local_read (frame : RuntimeFrame) (localId : LocalId) (value : RuntimeValue)
    (present : readLocal? frame localId = some value)
    (inBounds : localId.index < frame.locals.size) :
    Returns (nativeLocalOperation (.read ⟨localId⟩) valuesNil) frame frame value := by
  apply nativeOperation_value
  · exact nil frame
  · intro state
    simp [LocalLocationOperation.evaluate?, inBounds, readRuntimePlace?, readRoot?,
      readProjections?, present]

theorem indexed_shared (codec : Codec α RuntimeValue) (values : SpecVector α)
    (frame : RuntimeFrame) (localId : LocalId) (index : Nat)
    (referenceType : ReferenceType) (lexical : Nat)
    (shared : referenceType.kind = .shared)
    (present : readLocal? frame localId = some ((Codec.boundedVector codec).encode values))
    (inBounds : localId.index < frame.locals.size) (elementBound : index < values.values.size) :
    Returns (nativeIndexedLocalBorrowOperation
      ⟨⟨localId⟩, false, index, referenceType, .immutable, lexical, none⟩ valuesNil)
      frame frame (codec.encode values.values[index]) := by
  apply nativeOperation_value
  · exact nil frame
  · intro state
    simp [IndexedLocalBorrowOperation.evaluate?, IndexedLocalBorrowOperation.resolve?,
      resolveLocalLiteralIndex?, liftPlaceEvaluator, Nat.not_le_of_lt inBounds,
      borrowRuntimePlaceAt?, shared, readRuntimePlace?, readRoot?, readProjections?, present,
      Codec.boundedVector_encode,
      show (ReferenceKind.shared != ReferenceKind.shared) = false from rfl, elementBound]

end LeanerIR.Proofs.NativeVector
