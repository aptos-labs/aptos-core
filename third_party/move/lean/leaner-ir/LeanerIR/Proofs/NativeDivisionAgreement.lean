-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeResultAgreement
import LeanerIR.Proofs.NativeDivision
import LeanerIR.Proofs.IntegerEvaluation

namespace LeanerIR.Proofs.ComputationAgreement
open LeanerIR.SemanticOperations LeanerIR.BigStep Denotation

theorem checkedInteger_result (width : Nat) (signed : Bool) (kind : ThrowKind)
    (value lower upper : Int)
    (bounds : (Ty.integer (.bits width) signed).integerBounds? = some (lower, upper)) :
    checkedInteger kind (.integer (.bits width) signed) value =
      (match NativeArithmetic.checkedResult (.bits width) signed (NativeArithmetic.runtimeFailure kind) value with
      | .ok result => .ok (.integer result.val)
      | .error error => .error error) := by
  have fits_eq : IntegerValueFits (.bits width) signed value ↔ lower ≤ value ∧ value ≤ upper := by
    simp [IntegerValueFits, Ty.integerValueFits?, bounds]
  by_cases fits : lower ≤ value ∧ value ≤ upper <;>
    simp [checkedInteger, bounds, NativeArithmetic.checkedResult,
      NativeArithmetic.runtimeFailure, fits_eq, fits]

theorem checkedDivideIntegers_nonzero (kind : ThrowKind) (ty : Ty) (left right : Int)
    (nonzero : right ≠ 0) :
    checkedDivideIntegers? kind ty #[.integer left, .integer right] =
      some (checkedInteger kind ty (left.tdiv right)) := by
  unfold checkedDivideIntegers?
  split
  · rename_i h
    simp at h
    exact absurd h.2 nonzero
  · rename_i h
    simp only [List.cons.injEq, RuntimeValue.integer.injEq, and_true] at h
    obtain ⟨rfl, rfl⟩ := h
    simp [truncatingQuotient?_eq, nonzero]
    try (cases checkedInteger kind ty (left.tdiv right) <;> rfl)
  · simp_all

theorem checkedModuloIntegers_nonzero (kind : ThrowKind) (ty : Ty) (left right : Int)
    (nonzero : right ≠ 0) :
    checkedModuloIntegers? kind ty #[.integer left, .integer right] =
      some (match checkedInteger kind ty (left.tdiv right) with
      | .error error => .error error
      | .ok _ => checkedInteger kind ty (left.tmod right)) := by
  unfold checkedModuloIntegers?
  split
  · rename_i h
    simp at h
    exact absurd h.2 nonzero
  · rename_i h
    simp only [List.cons.injEq, RuntimeValue.integer.injEq, and_true] at h
    obtain ⟨rfl, rfl⟩ := h
    simp [truncatingQuotient?_eq, nonzero]
    rw [Int.tmod_def, Int.mul_comm right]
    cases checkedInteger kind ty (left.tdiv right) <;> rfl
  · simp_all

theorem division_evaluate_result (width : Nat) (signed : Bool) (kind : ThrowKind)
    (left right lower upper : Int) (frame : RuntimeFrame) (state : RuntimeState)
    (bounds : (Ty.integer (.bits width) signed).integerBounds? = some (lower, upper)) :
    (PrimitiveLocationOperation.checkedDivide kind (.integer (.bits width) signed)).evaluate?
      #[.integer left, .integer right] frame state =
      some (resultControl (fun value : SpecInt (.bits width) signed => .integer value.val) frame state
        (NativeArithmetic.divisionResult (.bits width) signed (NativeArithmetic.emptyFailure kind)
          (NativeArithmetic.runtimeFailure kind) left right)) := by
  by_cases nonzero : right = 0
  · subst right
    simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
      checkedDivideIntegers?, NativeArithmetic.divisionResult, NativeArithmetic.emptyFailure, resultControl]
  · simp only [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
      checkedDivideIntegers_nonzero _ _ _ _ nonzero,
      NativeArithmetic.divisionResult, if_neg nonzero,
      checkedInteger_result _ _ _ _ _ _ bounds]
    cases NativeArithmetic.checkedResult (.bits width) signed (NativeArithmetic.runtimeFailure kind)
        (left.tdiv right) <;> rfl

theorem remainder_evaluate_result (width : Nat) (signed : Bool) (kind : ThrowKind)
    (left right lower upper : Int) (frame : RuntimeFrame) (state : RuntimeState)
    (bounds : (Ty.integer (.bits width) signed).integerBounds? = some (lower, upper)) :
    (PrimitiveLocationOperation.checkedModulo kind (.integer (.bits width) signed)).evaluate?
      #[.integer left, .integer right] frame state =
      some (resultControl (fun value : SpecInt (.bits width) signed => .integer value.val) frame state
        (NativeArithmetic.remainderResult (.bits width) signed (NativeArithmetic.emptyFailure kind)
          (NativeArithmetic.runtimeFailure kind) left right)) := by
  by_cases nonzero : right = 0
  · subst right
    simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
      checkedModuloIntegers?, NativeArithmetic.remainderResult, NativeArithmetic.emptyFailure, resultControl]
  · simp only [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
      checkedModuloIntegers_nonzero _ _ _ _ nonzero,
      NativeArithmetic.remainderResult, if_neg nonzero,
      checkedInteger_result _ _ _ _ _ _ bounds]
    cases NativeArithmetic.checkedResult (.bits width) signed (NativeArithmetic.runtimeFailure kind)
        (left.tdiv right) <;>
      cases NativeArithmetic.checkedResult (.bits width) signed (NativeArithmetic.runtimeFailure kind)
        (left.tmod right) <;> rfl

end LeanerIR.Proofs.ComputationAgreement
