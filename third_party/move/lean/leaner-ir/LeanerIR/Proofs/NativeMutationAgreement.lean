-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeMutation
import LeanerIR.Proofs.NativeVectorAgreement

namespace LeanerIR.Proofs.NativeMutation

set_option maxHeartbeats 1000

open SemanticOperations

/-- Replacing a live reference's current value is exactly a runtime dereference
write, preserving the loan identity. Encoding appears only in this proof. -/
theorem write_projection (codec : Codec α RuntimeValue) (reference : MutableArgument α)
    (replacement : α) :
    writeProjections? (codec.mutable.encode reference) [.deref] (codec.encode replacement) =
      some (codec.mutable.encode (write reference replacement)) := by
  rfl

theorem read_projection (codec : Codec α RuntimeValue) (reference : MutableArgument α) :
    readProjections? (codec.mutable.encode reference) [.deref] =
      some (codec.encode (read reference)) := by
  rfl

/-- Agreement with the actual reference-mutation operation, for a registered
resting reference. The cache and occupied slot are checked, not assumed from
the argument's value. The rule is independent of scalar/aggregate layout. -/
theorem write_registered (codec : Codec α RuntimeValue) (reference : MutableArgument α)
    (replacement : α) (slot : LocalId) (frame : RuntimeFrame) (state : RuntimeState)
    (location : localLoanPlace? frame reference.loan = some ⟨.local slot, #[], true⟩)
    (present : readLocal? frame slot = some (codec.mutable.encode reference)) :
    Denotation.ReferenceLocationOperation.mutate.evaluate?
      #[codec.mutable.encode reference, codec.encode replacement] frame state =
      some (.value { frame with
        locals := frame.locals.set! slot.index
          (some (codec.mutable.encode (write reference replacement))) } state .unit) := by
  have bound : slot.index < frame.locals.size := by
    by_cases bound : slot.index < frame.locals.size
    · exact bound
    · simp [readLocal?, getElem?_neg frame.locals slot.index bound] at present
  have replaced : rewriteFirst (borrowRewrite? reference.loan (codec.encode replacement))
      (.borrow reference.loan (codec.encode reference.value)) =
      some (.borrow reference.loan (codec.encode replacement)) := by
    rw [rewriteFirst.eq_def]
    simp [borrowRewrite?]
  simp [Denotation.ReferenceLocationOperation.evaluate?, Denotation.liftPlaceEvaluator,
    mutateBorrow?, updateBorrowValue?, updateLocalBorrowValue?, location,
    readRuntimePlace?, readRoot?, present, readProjections?, Codec.mutable, write,
    replaced, writeRuntimePlace?, writeRoot?, Nat.not_le.mpr bound]

/-- Owner reconciliation is a native map of the checked vector update. This
equivalence retains both the updated value and the original loan identity. -/
theorem setElement_ok (reference : MutableArgument (SpecVector α)) (index : Int)
    (replacement : α) (failure : Error) (initial final : State)
    (result : MutableArgument (SpecVector α)) :
    (setElement reference index replacement failure).ok initial result final ↔
      (NativeVector.set reference.value index replacement failure).ok initial result.value final ∧
      result.loan = reference.loan := by
  constructor
  · rintro ⟨updated, middle, executed, rfl, rfl⟩
    exact ⟨executed, rfl⟩
  · rintro ⟨executed, sameLoan⟩
    refine ⟨result.value, final, executed, ?_, rfl⟩
    cases result
    cases reference
    simp_all [write]

theorem setElement_aborts (reference : MutableArgument (SpecVector α)) (index : Int)
    (replacement : α) (failure : Error) (initial : State) (error : Error) :
    (setElement reference index replacement failure).aborts initial error ↔
      ¬(0 ≤ index ∧ index < (reference.value.values.size : Int)) ∧ error = failure := by
  simpa only [setElement, Spec.bind, Spec.pure, and_false, exists_false, or_false] using
    NativeVector.set_aborts reference.value index replacement failure initial error

theorem setElement_defined (reference : MutableArgument (SpecVector α)) (index : Int)
    (replacement : α) (failure : Error) (initial : State) :
    ¬(setElement reference index replacement failure).undefined initial := by
  simpa only [setElement, Spec.bind, Spec.pure, and_false, exists_false, or_false] using
    NativeVector.set_defined reference.value index replacement failure initial

/-- The updated borrowed vector agrees with the interpreter's composed
dereference/index path. The bounds check and store equality are explicit;
neither is inferred from a postcondition. -/
theorem setElement_write (codec : Codec α RuntimeValue)
    (reference : MutableArgument (SpecVector α)) (index : Int) (replacement : α)
    (failure : Error) (initial final : State) (result : MutableArgument (SpecVector α)) :
    (setElement reference index replacement failure).ok initial result final ↔
      (0 ≤ index ∧ index < (reference.value.values.size : Int)) ∧
      writeProjections? ((Codec.boundedVector codec).mutable.encode reference)
        [.deref, .index index.toNat] (codec.encode replacement) =
        some ((Codec.boundedVector codec).mutable.encode result) ∧ final = initial := by
  rw [setElement_ok, NativeVector.set_ok codec]
  change (_ ∧ _ ∧ _) ∧ result.loan = reference.loan ↔
    _ ∧ (writeProjections? ((Codec.boundedVector codec).encode reference.value)
      [.index index.toNat] (codec.encode replacement)).bind
        (fun updated => some (RuntimeValue.borrow reference.loan updated)) =
      some (.borrow result.loan ((Codec.boundedVector codec).encode result.value)) ∧ _
  cases written : writeProjections? ((Codec.boundedVector codec).encode reference.value)
      [.index index.toNat] (codec.encode replacement) with
  | none => simp
  | some updated =>
    simp only [Option.bind_some, Option.some.injEq, RuntimeValue.borrow.injEq]
    constructor
    · rintro ⟨⟨valid, sameValue, sameState⟩, sameLoan⟩
      exact ⟨valid, ⟨sameLoan.symm, sameValue⟩, sameState⟩
    · rintro ⟨valid, ⟨sameLoan, sameValue⟩, sameState⟩
      exact ⟨⟨valid, sameValue, sameState⟩, sameLoan.symm⟩

end LeanerIR.Proofs.NativeMutation

namespace LeanerIR.Proofs.ComputationAgreement

open Denotation

set_option maxHeartbeats 1000

/-- A live owner supplies its typed observed value directly. Dereference
does not allocate a loan, reconcile pending writes, or change the state. -/
theorem scalar_reference_read (codec : Codec α RuntimeValue) (owner : MutableArgument α)
    (reference : ExprDenotation) (entry : RuntimeFrame)
    (present : Returns reference entry entry (codec.mutable.encode owner)) :
    Scalar (nativeReferenceOperation .dereference (valuesCons reference valuesNil))
      entry codec.encode (Spec.pure owner.value) := by
  apply scalar_pure codec.encode owner.value
  apply nativeOperation_value
  · exact cons present (nil entry)
  · exact fun _ => rfl

/-- Operand evaluation and a frame-changing primitive share one exact
certificate. The mutation's final frame is distinct from the operand frame;
it never becomes a native computation parameter. -/
theorem nativeOperation_update {operands : ValuesDenotation}
    {entry middle exit : RuntimeFrame} {values : List RuntimeValue}
    (evaluate : NativeEvaluator) (value : RuntimeValue)
    (operandsAgreement : ValuesReturn operands entry middle values)
    (evaluates : ∀ state, evaluate values.toArray middle state =
      some (.value exit state value)) :
    Returns (nativeOperation evaluate operands) entry exit value := by
  intro state finalFrame finalState control
  constructor
  · rintro (⟨frame, store, flow, propagated, _⟩ |
      ⟨frame, store, arguments, prepared, executed⟩)
    · have impossible := (operandsAgreement state _).mp propagated
      cases impossible
    · have same := (operandsAgreement state _).mp prepared
      cases same
      rcases executed with ⟨result, evaluated, equal⟩ | ⟨kind, arguments, evaluated, _⟩
      · rw [evaluates] at evaluated
        cases evaluated
        exact ⟨rfl, rfl, equal⟩
      · rw [evaluates] at evaluated
        cases evaluated
  · rintro ⟨sameFrame, sameState, sameControl⟩
    subst finalFrame finalState control
    exact Or.inr ⟨middle, state, values, (operandsAgreement state _).mpr rfl,
      Or.inl ⟨value, evaluates state, rfl⟩⟩

end LeanerIR.Proofs.ComputationAgreement
