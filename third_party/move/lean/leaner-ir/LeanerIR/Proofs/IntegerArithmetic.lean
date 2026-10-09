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

/-- Bits added to a value that holds none of them: their disjunction. -/
private theorem xor_add_of_and_eq {a c : Nat} (within : a &&& c = c) : (a ^^^ c) + c = a := by
  have aBelow : a < 2 ^ a := Nat.lt_two_pow_self
  have cBelow : c < 2 ^ a := by
    have : c ≤ a := within ▸ Nat.and_le_left
    omega
  have xBelow : a ^^^ c < 2 ^ a := Nat.xor_lt_two_pow aBelow cBelow
  have disjoint : BitVec.ofNat a (a ^^^ c) &&& BitVec.ofNat a c = 0#a := by
    apply BitVec.eq_of_toNat_eq
    rw [BitVec.toNat_and, BitVec.toNat_ofNat, BitVec.toNat_ofNat, Nat.mod_eq_of_lt xBelow,
      Nat.mod_eq_of_lt cBelow, BitVec.toNat_ofNat, Nat.zero_mod]
    apply Nat.eq_of_testBit_eq
    intro i
    have bit := congrArg (Nat.testBit · i) within
    simp only [Nat.testBit_and] at bit
    simp only [Nat.testBit_and, Nat.testBit_xor, Nat.zero_testBit]
    cases ha : a.testBit i <;> cases hc : c.testBit i <;> simp_all
  have sum := BitVec.toNat_add_of_and_eq_zero disjoint
  rw [BitVec.add_eq_or_of_and_eq_zero _ _ disjoint, BitVec.toNat_or, BitVec.toNat_ofNat,
    BitVec.toNat_ofNat, Nat.mod_eq_of_lt xBelow, Nat.mod_eq_of_lt cBelow] at sum
  have union : (a ^^^ c) ||| c = a := by
    apply Nat.eq_of_testBit_eq
    intro i
    have bit := congrArg (Nat.testBit · i) within
    simp only [Nat.testBit_and] at bit
    simp only [Nat.testBit_or, Nat.testBit_xor]
    cases ha : a.testBit i <;> cases hc : c.testBit i <;> simp_all
  omega

theorem bitwiseOr_nonnegative (left right : Int)
    (leftNonnegative : 0 ≤ left) (rightNonnegative : 0 ≤ right) :
    bitwiseOr left right = Int.ofNat (left.toNat ||| right.toNat) := by
  rw [← Int.toNat_of_nonneg leftNonnegative, ← Int.toNat_of_nonneg rightNonnegative]
  rfl

theorem bitwiseXor_nonnegative (left right : Int)
    (leftNonnegative : 0 ≤ left) (rightNonnegative : 0 ≤ right) :
    bitwiseXor left right = Int.ofNat (left.toNat ^^^ right.toNat) := by
  rw [← Int.toNat_of_nonneg leftNonnegative, ← Int.toNat_of_nonneg rightNonnegative]
  generalize left.toNat = m
  generalize right.toNat = n
  have within : (m ||| n) &&& (m &&& n) = m &&& n := by
    apply Nat.eq_of_testBit_eq
    intro i
    simp only [Nat.testBit_and, Nat.testBit_or]
    cases m.testBit i <;> cases n.testBit i <;> rfl
  have parts : (m ||| n) ^^^ (m &&& n) = m ^^^ n := by
    apply Nat.eq_of_testBit_eq
    intro i
    simp only [Nat.testBit_and, Nat.testBit_or, Nat.testBit_xor]
    cases m.testBit i <;> cases n.testBit i <;> rfl
  have sum := xor_add_of_and_eq within
  rw [parts] at sum
  have difference : (m ||| n) - (m &&& n) = m ^^^ n := by omega
  show Int.ofNat ((m ||| n) - ((m ||| n) &&& (m &&& n))) = Int.ofNat ((m : Int).toNat ^^^ (n : Int).toNat)
  rw [within, difference, Int.toNat_natCast, Int.toNat_natCast]

/-- Exclusive disjunction with the ones below a width complements a value
below it. -/
theorem xor_two_pow_sub_one {n k : Nat} (below : n < 2 ^ k) :
    n ^^^ (2 ^ k - 1) = 2 ^ k - 1 - n := by
  have := BitVec.toNat_xor (BitVec.ofNat k n) (BitVec.allOnes k)
  rw [BitVec.xor_allOnes, BitVec.toNat_not, BitVec.toNat_ofNat, BitVec.toNat_allOnes,
    Nat.mod_eq_of_lt below] at this
  omega

theorem bitwiseAnd_comm (left right : Int) : bitwiseAnd left right = bitwiseAnd right left := by
  cases left <;> cases right <;> simp only [bitwiseAnd, Nat.and_comm, Nat.or_comm]

theorem bitwiseAnd_self (value : Int) : bitwiseAnd value value = value := by
  cases value <;> simp only [bitwiseAnd, Nat.and_self, Nat.or_self]

theorem bitwiseOr_self (value : Int) : bitwiseOr value value = value := by
  cases value <;> simp only [bitwiseOr, bitwiseAnd_self] <;> rfl

theorem complement_complement (value : Int) : ~~~~~~value = value := by
  cases value <;> rfl

/-- A conjunction with all ones, `-1`, keeps a value. -/
theorem bitwiseAnd_negOne (value : Int) : bitwiseAnd value (-1) = value := by
  show bitwiseAnd value (Int.negSucc 0) = value
  cases value <;> simp [bitwiseAnd]

theorem bitwiseAnd_zero (value : Int) : bitwiseAnd value 0 = 0 := by
  show bitwiseAnd value (Int.ofNat 0) = Int.ofNat 0
  cases value <;> simp [bitwiseAnd]

theorem zero_bitwiseAnd (value : Int) : bitwiseAnd 0 value = 0 := by
  rw [bitwiseAnd_comm, bitwiseAnd_zero]

theorem bitwiseOr_zero (value : Int) : bitwiseOr value 0 = value := by
  unfold bitwiseOr
  rw [show (~~~(0 : Int)) = -1 from rfl, bitwiseAnd_negOne, complement_complement]

theorem zero_bitwiseOr (value : Int) : bitwiseOr 0 value = value := by
  unfold bitwiseOr
  rw [show (~~~(0 : Int)) = -1 from rfl, bitwiseAnd_comm, bitwiseAnd_negOne, complement_complement]

theorem bitwiseXor_zero (value : Int) : bitwiseXor value 0 = value := by
  unfold bitwiseXor
  rw [bitwiseOr_zero, bitwiseAnd_zero, show (~~~(0 : Int)) = -1 from rfl, bitwiseAnd_negOne]

theorem zero_bitwiseXor (value : Int) : bitwiseXor 0 value = value := by
  unfold bitwiseXor
  rw [zero_bitwiseOr, zero_bitwiseAnd, show (~~~(0 : Int)) = -1 from rfl, bitwiseAnd_negOne]

theorem bitwiseAnd_nonneg {left right : Int} (leftNonnegative : 0 ≤ left)
    (rightNonnegative : 0 ≤ right) : 0 ≤ bitwiseAnd left right := by
  rw [bitwiseAnd_nonnegative left right leftNonnegative rightNonnegative]
  exact Int.natCast_nonneg _

/-- A conjunction of nonnegative values is bounded by each. -/
theorem bitwiseAnd_bounds {left right : Int} (leftNonnegative : 0 ≤ left)
    (rightNonnegative : 0 ≤ right) :
    0 ≤ bitwiseAnd left right ∧ bitwiseAnd left right ≤ left ∧
      bitwiseAnd left right ≤ right := by
  rw [bitwiseAnd_nonnegative left right leftNonnegative rightNonnegative, Int.ofNat_eq_coe]
  have leftBound := Nat.and_le_left (n := left.toNat) (m := right.toNat)
  have rightBound := Nat.and_le_right (n := left.toNat) (m := right.toNat)
  have leftCast := Int.toNat_of_nonneg leftNonnegative
  have rightCast := Int.toNat_of_nonneg rightNonnegative
  refine ⟨Int.natCast_nonneg _, ?_, ?_⟩ <;> omega

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
