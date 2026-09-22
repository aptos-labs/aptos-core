-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeArithmetic
import LeanerIR.Proofs.NativeResult

/-! Truncating division/remainder, including the VM's quotient overflow check.
Only the supplied error constructors cross the runtime compatibility boundary. -/

namespace LeanerIR.Proofs
namespace NativeArithmetic

/-- Compatibility boundary for a VM failure with no payload. -/
def emptyFailure (kind : ThrowKind) : Failure := (kind, #[])

def checkedResult (width : IntWidth) (signed : Bool) (failure : Int → Error)
    (value : Int) : Except Error (SpecInt width signed) :=
  if fits : IntegerValueFits width signed value then .ok ⟨value, fits⟩
  else .error (failure value)

theorem ofExcept_checkedResult (width : IntWidth) (signed : Bool) (failure : Int → Error)
    (value : Int) :
    Spec.ofExcept (State := State) (checkedResult width signed failure value) =
      checkedInteger width signed failure value := by
  unfold checkedResult checkedInteger
  split <;> rfl

def divisionResult (width : IntWidth) (signed : Bool) (zero : Error)
    (failure : Int → Error) (left right : Int) : Except Error (SpecInt width signed) :=
  if right = 0 then .error zero else checkedResult width signed failure (left.tdiv right)

def remainderResult (width : IntWidth) (signed : Bool) (zero : Error)
    (failure : Int → Error) (left right : Int) : Except Error (SpecInt width signed) :=
  if right = 0 then .error zero else
    match checkedResult width signed failure (left.tdiv right) with
    | .error error => .error error
    | .ok _ => checkedResult width signed failure (left.tmod right)

theorem wp_divisionResult (width : IntWidth) (signed : Bool) (zero : Error)
    (failure : Int → Error) (left right : Int)
    (ensures : SpecInt width signed → State → Prop) (aborts : Error → Prop) (initial : State) :
    wp (Spec.ofExcept (divisionResult width signed zero failure left right)) ensures aborts initial ↔
      (right = 0 → aborts zero) ∧
      (right ≠ 0 → wp (checkedInteger width signed failure (left.tdiv right)) ensures aborts initial) := by
  by_cases nonzero : right = 0
  · simp [divisionResult, nonzero, Spec.ofExcept]
  · simp only [divisionResult, if_neg nonzero, ofExcept_checkedResult]
    simp [nonzero]

theorem wp_remainderResult (width : IntWidth) (signed : Bool) (zero : Error)
    (failure : Int → Error) (left right : Int)
    (ensures : SpecInt width signed → State → Prop) (aborts : Error → Prop) (initial : State) :
    wp (Spec.ofExcept (remainderResult width signed zero failure left right)) ensures aborts initial ↔
      (right = 0 → aborts zero) ∧
      (right ≠ 0 → wp (Spec.bind (checkedInteger width signed failure (left.tdiv right))
        (fun _ => checkedInteger width signed failure (left.tmod right))) ensures aborts initial) := by
  by_cases nonzero : right = 0
  · simp [remainderResult, nonzero, Spec.ofExcept]
  · by_cases fits : IntegerValueFits width signed (left.tdiv right) <;>
      by_cases remainderFits : IntegerValueFits width signed (left.tmod right) <;>
      simp [remainderResult, nonzero, checkedResult, checkedInteger, fits, remainderFits, Spec.ofExcept]

end NativeArithmetic

end LeanerIR.Proofs
