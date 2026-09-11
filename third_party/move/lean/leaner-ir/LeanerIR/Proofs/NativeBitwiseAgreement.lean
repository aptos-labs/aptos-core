-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeBitwise
import LeanerIR.Proofs.NativeValueAgreement

/-! Exact execution agreement for fixed-width native bit patterns. -/

namespace LeanerIR.Proofs.ComputationAgreement
open SemanticOperations

theorem modularInteger_wrap (width : Nat) (positive : 0 < width) (signed : Bool) (value : Int) :
    modularInteger (.integer (.bits width) signed) value =
      some (.integer (NativeArithmetic.wrap width positive signed value).val) := by
  simp [modularInteger, NativeArithmetic.wrap, NativeArithmetic.wrapValue,
    Nat.ne_of_gt positive]

theorem bitwiseBinaryInteger_native (width : Nat) (positive : 0 < width) (signed : Bool)
    (operation : Nat → Nat → Nat) (left right : SpecInt (.bits width) signed) :
    bitwiseBinaryInteger (.integer (.bits width) signed)
        #[.integer left.val, .integer right.val] operation =
      some (.ok (.integer (NativeArithmetic.bitwise width positive signed operation left right).val)) := by
  simp [bitwiseBinaryInteger, integerBitPattern?, Nat.ne_of_gt positive,
    modularInteger_wrap width positive, NativeArithmetic.bitwise]

end LeanerIR.Proofs.ComputationAgreement
