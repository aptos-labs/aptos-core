-- `math128::sqrt` returns r with r² ≤ y < (r + 1)², facts `omega` does not
-- combine: the root of a nonzero u128 is between 1 and 2^64 - 1, so the
-- scaled root is a nonzero divisor, and the quotient of one Newton step
-- from it stays below (r + 3) 2^32.
theorem sqrt_root_range {y r : Int} (square : 0 < y → r * r ≤ y)
    (above : 0 < y → y < (r + 1) * (r + 1)) (positive : 0 < y)
    (small : y ≤ 340282366920938463463374607431768211455) (root : 0 ≤ r) :
    1 ≤ r ∧ r ≤ 18446744073709551615 := by
  have square := square positive
  have above := above positive
  constructor
  · refine Int.not_lt.mp fun zero => ?_
    have : r = 0 := by omega
    subst this
    omega
  · refine Int.not_lt.mp fun large => ?_
    have wide : (18446744073709551616 : Int) ≤ r := by omega
    have := Int.mul_le_mul wide wide (by omega) root
    omega

theorem newton_quotient {y r : Int} (square : 0 < y → r * r ≤ y)
    (above : 0 < y → y < (r + 1) * (r + 1)) (positive : 0 < y)
    (small : y ≤ 340282366920938463463374607431768211455) (root : 0 ≤ r) :
    y.shiftLeft 64 / (r.shiftLeft 32 % 340282366920938463463374607431768211456) <
      (r + 3) * 4294967296 := by
  obtain ⟨one, bounded⟩ := sqrt_root_range square above positive small root
  have above := above positive
  have dividend : y.shiftLeft 64 = y * 18446744073709551616 := Int.shiftLeft_eq y 64
  have divisor : r.shiftLeft 32 = r * 4294967296 := Int.shiftLeft_eq r 32
  rw [dividend, divisor, Int.emod_eq_of_lt (by omega) (by omega)]
  have expand : (r + 1) * (r + 1) + (r - 1) = (r + 3) * r := by
    simp only [Int.add_mul, Int.mul_add, Int.mul_one, Int.one_mul]; omega
  apply Int.ediv_lt_of_lt_mul (by omega)
  rw [show (r + 3) * 4294967296 * (r * 4294967296) = (r + 3) * r * 18446744073709551616 by
    rw [Int.mul_assoc, Int.mul_comm 4294967296, Int.mul_assoc, Int.mul_assoc]; rfl]
  exact Int.mul_lt_mul_of_pos_right (by omega) (by decide)

verify sqrt by
  case leaf_1 =>
    have := sqrt_root_range ‹_› ‹_› (by omega) (by omega) (by omega)
    have := newton_quotient ‹_› ‹_› (by omega) (by omega) (by omega)
    omega
  case leaf_2 =>
    have := sqrt_root_range ‹_› ‹_› (by omega) (by omega) (by omega)
    omega
