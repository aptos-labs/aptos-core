-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeShift
import LeanerIR.Proofs.NativeBitwiseAgreement
import LeanerIR.Proofs.NativeResultAgreement

namespace LeanerIR.Proofs.ComputationAgreement
open SemanticOperations Denotation

theorem checkedShiftInteger_native (width : Nat) (positive : 0 < width) (signed left : Bool)
    (kind : ThrowKind) (value : SpecInt (.bits width) signed) (distance : Int) :
    checkedShiftInteger kind left (.integer (.bits width) signed)
        #[.integer value.val, .integer distance] =
      some (match NativeArithmetic.shiftResult width positive signed left
          (fun distance => (kind, #[.integer distance])) value distance with
        | .ok result => .ok (.integer result.val)
        | .error failure => .error failure) := by
  by_cases invalid : distance < 0 ∨ width ≤ distance.toNat <;>
    simp [checkedShiftInteger, NativeArithmetic.shiftResult, NativeArithmetic.shiftValue, integerBitPattern?,
      Nat.ne_of_gt positive, Bool.or_eq_true, invalid, modularInteger_wrap width positive]

theorem shift_evaluate_result (width : Nat) (positive : 0 < width) (signed left : Bool)
    (kind : ThrowKind) (value : SpecInt (.bits width) signed) (distance : Int)
    (frame : RuntimeFrame) (state : RuntimeState) :
    (if left then PrimitiveLocationOperation.checkedShiftLeft kind (.integer (.bits width) signed)
      else PrimitiveLocationOperation.checkedShiftRight kind (.integer (.bits width) signed)).evaluate?
        #[.integer value.val, .integer distance] frame state =
      some (resultControl (fun value : SpecInt (.bits width) signed => .integer value.val) frame state
        (NativeArithmetic.shiftResult width positive signed left
          (fun distance => (kind, #[.integer distance])) value distance)) := by
  cases left <;>
    simp only [Bool.false_eq_true, ite_false, ite_true, PrimitiveLocationOperation.evaluate?,
      liftPrimitiveEvaluator, checkedShiftInteger_native width positive] <;>
    split <;> simp_all [resultControl]

end LeanerIR.Proofs.ComputationAgreement
