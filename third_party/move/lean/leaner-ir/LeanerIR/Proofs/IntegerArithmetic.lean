-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Representation

/-! Integer bounds shared by native verification and execution-boundary
proofs. These are mathematical facts, independent of frames or evaluators. -/

namespace LeanerIR.Proofs.IntegerArithmetic

/-- Mathematical bitwise conjunction on infinite two's-complement integers.
For nonnegative inputs this is exactly the natural-number operation used by
the VM's unsigned bit patterns. -/
def bitwiseAnd : Int → Int → Int
  | .ofNat left, .ofNat right => .ofNat (left &&& right)
  | .ofNat left, .negSucc right => .ofNat (left - (left &&& right))
  | .negSucc left, .ofNat right => .ofNat (right - (left &&& right))
  | .negSucc left, .negSucc right => .negSucc (left ||| right)

def bitwiseOr (left right : Int) : Int :=
  ~~~(bitwiseAnd (~~~left) (~~~right))

def bitwiseXor (left right : Int) : Int :=
  bitwiseAnd (bitwiseOr left right) (~~~(bitwiseAnd left right))

theorem bitwiseAnd_nonnegative (left right : Int)
    (leftNonnegative : 0 ≤ left) (rightNonnegative : 0 ≤ right) :
    bitwiseAnd left right = Int.ofNat (left.toNat &&& right.toNat) := by
  rw [← Int.toNat_of_nonneg leftNonnegative, ← Int.toNat_of_nonneg rightNonnegative]
  rfl

theorem bitwiseAnd_mod (left right modulus : Int)
    (leftLower : 0 ≤ left) (leftUpper : left < modulus)
    (rightLower : 0 ≤ right) (rightUpper : right < modulus) :
    (Int.ofNat ((left % modulus).toNat &&& (right % modulus).toNat)) % modulus =
      bitwiseAnd left right := by
  rw [Int.emod_eq_of_lt leftLower leftUpper,
    Int.emod_eq_of_lt rightLower rightUpper,
    bitwiseAnd_nonnegative left right leftLower rightLower]
  apply Int.emod_eq_of_lt (Int.natCast_nonneg _)
  have bounded := Nat.and_le_left (n := left.toNat) (m := right.toNat)
  omega

theorem shiftLeft_mod (value modulus : Int) (distance : Nat)
    (lower : 0 ≤ value) (upper : value < modulus) :
    (max (value % modulus) 0 <<< distance) % modulus =
      (value.shiftLeft distance).tmod modulus := by
  have shiftedNonnegative : 0 ≤ value <<< distance := by
    rw [Int.shiftLeft_eq]
    exact Int.mul_nonneg lower (Int.pow_nonneg (by decide))
  rw [Int.emod_eq_of_lt lower upper, Int.max_eq_left lower]
  exact (Int.tmod_eq_emod_of_nonneg shiftedNonnegative).symm

theorem shiftRight_mod (value modulus : Int) (distance : Nat)
    (lower : 0 ≤ value) (upper : value < modulus) :
    (max (value % modulus) 0 >>> distance) % modulus = value.shiftRight distance := by
  rw [Int.emod_eq_of_lt lower upper, Int.max_eq_left lower]
  exact Int.emod_eq_of_lt (Int.le_shiftRight_of_nonneg lower)
    (Int.lt_of_le_of_lt (Int.shiftRight_le_of_nonneg lower) upper)

/-- Negating a closed interval produces its two open complements. Kept as a
small reusable proof so generated closers need not normalize large concrete
powers throughout an execution context. -/
theorem outside_of_failed_range {lower upper value : Int}
    (failed : lower ≤ value → upper < value) :
    value < lower ∨ upper < value := by
  omega

/-- The i64 instance is kept opaque to generated storage contexts. It avoids
normalizing the large power in every unrelated codec and loan hypothesis. -/
theorem outside_i64_of_failed_range {value : Int}
    (failed : -2 ^ (64 - 1) ≤ value → 2 ^ (64 - 1) - 1 < value) :
    value < -9223372036854775808 ∨ 9223372036854775807 < value := by
  simp only [Nat.reduceSub, Int.reduceNeg] at failed ⊢
  exact outside_of_failed_range failed

/-- Unsigned truncating division cannot leave the dividend's fixed-width
range. The zero divisor is handled by the operation before this fact is used. -/
theorem unsigned_quotient_bounds {width : Nat} {left right : Int}
    (leftFits : IntegerValueFits (.bits width) false left)
    (rightFits : IntegerValueFits (.bits width) false right) :
    0 ≤ left.tdiv right ∧ left.tdiv right ≤ 2 ^ width - 1 := by
  have leftBounds := leftFits.unsigned_bounds
  have rightBounds := rightFits.unsigned_bounds
  exact ⟨Int.tdiv_nonneg leftBounds.1 rightBounds.1,
    Int.le_trans (Int.tdiv_le_self right leftBounds.1) leftBounds.2⟩

/-- Unsigned truncating remainder is nonnegative and smaller than its
nonzero in-range divisor. -/
theorem unsigned_remainder_bounds {width : Nat} {left right : Int}
    (leftFits : IntegerValueFits (.bits width) false left)
    (rightFits : IntegerValueFits (.bits width) false right)
    (nonzero : right ≠ 0) :
    0 ≤ left.tmod right ∧ left.tmod right ≤ 2 ^ width - 1 := by
  have leftBounds := leftFits.unsigned_bounds
  have rightBounds := rightFits.unsigned_bounds
  have positive : 0 < right := by omega
  exact ⟨Int.tmod_nonneg right leftBounds.1,
    by have := Int.tmod_lt_of_pos left positive; omega⟩

/-- A checked product that failed its lower-or-upper range test crosses the
upper endpoint when both operands are nonnegative. -/
theorem product_upper_of_failed_range {left right upper : Int}
    (leftNonnegative : 0 ≤ left) (rightNonnegative : 0 ≤ right)
    (failed : 0 ≤ left * right → upper < left * right) :
    upper < left * right :=
  failed (Int.mul_nonneg leftNonnegative rightNonnegative)

/-- Truncating division in an asymmetric signed range overflows exactly at
the minimum integer divided by minus one. A zero divisor is handled by the
operation's separate abort check. -/
theorem signed_quotient_bounds (limit : Nat)
    (left right : Int) (lower : -(limit : Int) ≤ left) (upper : left < limit)
    (nonzero : right ≠ 0) :
    (-(limit : Int) ≤ left.tdiv right ∧ left.tdiv right < limit) ↔
      ¬ (left = -(limit : Int) ∧ right = -1) := by
  by_cases one : right = 1
  · subst right
    simp [lower, upper]
  by_cases negativeOne : right = -1
  · subst right
    simp only [Int.tdiv_neg, Int.tdiv_one, and_true]
    omega
  have divisor : 1 < right.natAbs := by omega
  have dividend : left.natAbs ≤ limit := by omega
  have bounded : (left.tdiv right).natAbs < limit := by
    by_cases zero : left.natAbs = 0
    · have := Int.natAbs_tdiv_le_natAbs left right
      omega
    · rw [Int.natAbs_tdiv]
      exact Nat.lt_of_lt_of_le
        (Nat.div_lt_self (show 0 < left.natAbs by omega) divisor) dividend
  constructor <;> intro h <;> omega

/-- A truncating remainder by an in-range nonzero signed divisor stays in
range. This does not eliminate the VM's earlier quotient-overflow check. -/
theorem signed_remainder_bounds (limit : Nat) (left right : Int)
    (lower : -(limit : Int) ≤ right) (upper : right < limit)
    (nonzero : right ≠ 0) :
    -(limit : Int) ≤ left.tmod right ∧ left.tmod right < limit := by
  have bounded : (left.tmod right).natAbs < right.natAbs := by
    rw [Int.natAbs_tmod]
    exact Nat.mod_lt _ (Int.natAbs_pos.mpr nonzero)
  omega

end LeanerIR.Proofs.IntegerArithmetic
