-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Computation
import LeanerIR.Proofs.IntegerArithmetic

/-! Fixed-width native bit patterns. Wrapping returns a range-carrying integer;
no runtime values, frames, or semantic evaluators appear in the computation. -/

namespace LeanerIR.Proofs.NativeArithmetic

def wrapValue (width : Nat) (signed : Bool) (value : Int) : Int :=
  let residue := value % (2 : Int) ^ width
  if signed && residue >= (2 : Int) ^ (width - 1) then
    residue - (2 : Int) ^ width
  else residue

theorem wrapValue_fits (width : Nat) (positive : 0 < width) (signed : Bool) (value : Int) :
    IntegerValueFits (.bits width) signed (wrapValue width signed value) := by
  have modulusPositive : (0 : Int) < 2 ^ width := Int.pow_pos (by decide)
  have lower := Int.emod_nonneg value (by omega : (2 : Int) ^ width ≠ 0)
  have upper := Int.emod_lt_of_pos value modulusPositive
  have twice : (2 : Int) ^ width = 2 ^ (width - 1) * 2 := by
    calc
      (2 : Int) ^ width = 2 ^ ((width - 1) + 1) := congrArg (fun n => (2 : Int) ^ n) (by omega)
      _ = _ := Int.pow_succ 2 (width - 1)
  cases signed <;>
    simp [wrapValue, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?, Nat.ne_of_gt positive]
  · omega
  · split <;> omega

def wrap (width : Nat) (positive : 0 < width) (signed : Bool) (value : Int) :
    SpecInt (.bits width) signed :=
  ⟨wrapValue width signed value, wrapValue_fits width positive signed value⟩

def bitwise (width : Nat) (positive : 0 < width) (signed : Bool)
    (operation : Nat → Nat → Nat) (left right : SpecInt (.bits width) signed) :
    SpecInt (.bits width) signed :=
  wrap width positive signed (Int.ofNat
    (operation (left.val % (2 : Int) ^ width).toNat (right.val % (2 : Int) ^ width).toNat))

/-- Keep nested masks in their mathematical spelling while their native range
certificates are still available. No bound search through a flattened VC. -/
theorem bitwise_and_unsigned (width : Nat) (positive : 0 < width)
    (left right : SpecInt (.bits width) false) :
    (bitwise width positive false (fun a b => a &&& b) left right).val =
      IntegerArithmetic.bitwiseAnd left.val right.val := by
  have leftBounds := left.unsigned_bounds
  have rightBounds := right.unsigned_bounds
  simp only [bitwise, wrap, wrapValue, Bool.false_and, Bool.false_eq_true, ite_false]
  apply IntegerArithmetic.bitwiseAnd_mod <;> omega

-- Preserve the certified result projection until its mathematical rewrite
-- fires. Unfolding the subtype while choosing contract witnesses would lose
-- both that rewrite and its matching range facts. Explicit execution and
-- agreement proofs still unfold this definition.
attribute [irreducible] bitwise

end LeanerIR.Proofs.NativeArithmetic
