-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeBitwise
import LeanerIR.Proofs.NativeResult

/-! Typed checked shifts, including fixed-width wrapping and signed extension. -/

namespace LeanerIR.Proofs.NativeArithmetic

def shiftValue (width : Nat) (positive : 0 < width) (signed left : Bool)
    (value : SpecInt (.bits width) signed) (distance : Int) : SpecInt (.bits width) signed :=
  let amount := distance.toNat
  let bits := (value.val % (2 : Int) ^ width).toNat
  let shifted := if left then bits <<< amount
    else if signed && (2 : Nat) ^ (width - 1) <= bits && amount != 0 then
      (bits >>> amount) ||| (((2 : Nat) ^ amount - 1) <<< (width - amount))
    else bits >>> amount
  wrap width positive signed (Int.ofNat shifted)

def shiftResult (width : Nat) (positive : 0 < width) (signed left : Bool)
    (failure : Int → Error) (value : SpecInt (.bits width) signed) (distance : Int) :
    Except Error (SpecInt (.bits width) signed) :=
  if distance < 0 ∨ width ≤ distance.toNat then .error (failure distance)
  else .ok (shiftValue width positive signed left value distance)

theorem wp_shiftResult (width : Nat) (positive : 0 < width) (signed left : Bool)
    (failure : Int → Error) (value : SpecInt (.bits width) signed) (distance : Int)
    (ensures : SpecInt (.bits width) signed → State → Prop) (aborts : Error → Prop) (initial : State) :
    wp (Spec.ofExcept (shiftResult width positive signed left failure value distance)) ensures aborts initial ↔
      ((distance < 0 ∨ width ≤ distance.toNat) → aborts (failure distance)) ∧
      (¬(distance < 0 ∨ width ≤ distance.toNat) →
        ensures (shiftValue width positive signed left value distance) initial) := by
  by_cases invalid : distance < 0 ∨ width ≤ distance.toNat <;>
    simp [shiftResult, invalid, Spec.ofExcept, wp_pure, wp_abort]

/-- Bind a shift result once at a local/call boundary. The continuation works
with its native type and a value equation, not a substituted bit-pattern tree. -/
theorem wp_shiftResult_value (width : Nat) (positive : 0 < width) (signed left : Bool)
    (failure : Int → Error) (value : SpecInt (.bits width) signed) (distance : Int)
    (ensures : SpecInt (.bits width) signed → State → Prop) (aborts : Error → Prop) (initial : State) :
    wp (Spec.ofExcept (shiftResult width positive signed left failure value distance)) ensures aborts initial ↔
      ((distance < 0 ∨ width ≤ distance.toNat) → aborts (failure distance)) ∧
      (¬(distance < 0 ∨ width ≤ distance.toNat) → ∀ result,
        result = shiftValue width positive signed left value distance → ensures result initial) := by
  rw [wp_shiftResult]
  simp

theorem shiftValue_left_unsigned (width : Nat) (positive : 0 < width)
    (value : SpecInt (.bits width) false) (distance : Int) :
    (shiftValue width positive false true value distance).val =
      (value.val.shiftLeft distance.toNat).tmod ((2 : Int) ^ width) := by
  have bounds := value.unsigned_bounds
  have modulusPositive : (0 : Int) < 2 ^ width := Int.pow_pos (by decide)
  have residueNonnegative := Int.emod_nonneg value.val (by omega : (2 : Int) ^ width ≠ 0)
  simp only [shiftValue, wrap, wrapValue, Bool.false_and, Bool.false_eq_true, ite_false,
    ite_true, Int.ofNat_eq_natCast, Int.natCast_shiftLeft, Int.toNat_of_nonneg residueNonnegative]
  rw [Int.emod_eq_of_lt bounds.1 (by omega)]
  have shiftedNonnegative : 0 ≤ value.val.shiftLeft distance.toNat := by
    change 0 ≤ value.val <<< distance.toNat
    rw [Int.shiftLeft_eq]
    exact Int.mul_nonneg bounds.1 (Int.pow_nonneg (by decide))
  exact (Int.tmod_eq_emod_of_nonneg shiftedNonnegative).symm

theorem shiftValue_right_unsigned (width : Nat) (positive : 0 < width)
    (value : SpecInt (.bits width) false) (distance : Int) :
    (shiftValue width positive false false value distance).val = value.val.shiftRight distance.toNat := by
  have bounds := value.unsigned_bounds
  have modulusPositive : (0 : Int) < 2 ^ width := Int.pow_pos (by decide)
  have residueNonnegative := Int.emod_nonneg value.val (by omega : (2 : Int) ^ width ≠ 0)
  simp only [shiftValue, wrap, wrapValue, Bool.false_and, Bool.false_eq_true, ite_false,
    Int.ofNat_eq_natCast, Int.natCast_shiftRight, Int.toNat_of_nonneg residueNonnegative]
  rw [Int.emod_eq_of_lt bounds.1 (by omega)]
  exact Int.emod_eq_of_lt (Int.le_shiftRight_of_nonneg bounds.1)
    (Int.lt_of_le_of_lt (Int.shiftRight_le_of_nonneg bounds.1) (by omega))

/-- Supply an arithmetic equation for continuations such as a checked addition
after a right shift, without unfolding bit patterns in their proof contexts. -/
theorem shiftValue_right_unsigned_div (width : Nat) (positive : 0 < width)
    (value : SpecInt (.bits width) false) (distance : Int) :
    (shiftValue width positive false false value distance).val =
      value.val / (2 : Int) ^ distance.toNat := by
  rw [shiftValue_right_unsigned]
  change value.val >>> distance.toNat = _
  simpa using Int.shiftRight_eq_div_pow value.val distance.toNat

attribute [irreducible] shiftValue

theorem shiftValue_right_unsigned_upper (width : Nat) (positive : 0 < width)
    (value : SpecInt (.bits width) false) (distance : Int)
    (valid : distance.toNat ≤ width) :
    (shiftValue width positive false false value distance).val ≤
      2 ^ (width - distance.toNat) - 1 := by
  rw [shiftValue_right_unsigned_div]
  apply Int.le_sub_one_of_lt
  apply (Int.ediv_lt_iff_lt_mul (Int.pow_pos (by decide))).mpr
  rw [← Int.pow_add, Nat.sub_add_cancel valid]
  have bounds := value.unsigned_bounds
  omega

end LeanerIR.Proofs.NativeArithmetic
