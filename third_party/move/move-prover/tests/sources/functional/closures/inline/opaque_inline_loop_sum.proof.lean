-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- The loop uses truncating division; its consecutive product is nonnegative.
theorem consecutive_nonneg (x : Int) (nonnegative : 0 ≤ x) : 0 ≤ x * (x - 1) := by
  by_cases zero : x = 0
  · simp [zero]
  · exact Int.mul_nonneg nonnegative (by omega)

theorem triangular_step (x : Int) (nonnegative : 0 ≤ x) :
    (x * (x - 1)).tdiv 2 + x = ((x + 1) * (x + 1 - 1)).tdiv 2 := by
  rw [Int.tdiv_eq_ediv_of_nonneg (consecutive_nonneg x nonnegative),
    Int.tdiv_eq_ediv_of_nonneg (consecutive_nonneg (x + 1) (by omega))]
  have polynomial : (x + 1) * (x + 1 - 1) = x * (x - 1) + x * 2 := by grind
  rw [polynomial, Int.add_mul_ediv_right _ _ (by decide)]

verify sum by
  all_goals expose_names
  case leaf_1 =>
    have at_exit : i.val = n.val + 1 := by omega
    rw [right, at_exit, Int.add_sub_cancel, Int.mul_comm]
    exact Int.tdiv_eq_ediv_of_nonneg (Int.mul_nonneg (by omega) (by omega))
  case leaf_2 =>
    have nonnegative := consecutive_nonneg i.val (by omega)
    have value_eq : s.val = i.val * (i.val - 1) / 2 := by
      simpa only [Int.tdiv_eq_ediv_of_nonneg nonnegative] using right
    have divided := Int.ediv_le_self 2 nonnegative
    by_cases zero : i.val = 0
    · simp only [zero, Int.zero_mul, Int.zero_ediv] at value_eq
      omega
    · have upper := Int.mul_le_mul
        (a := i.val) (b := i.val - 1) (c := 4294967295) (d := 4294967294)
        (by omega) (by omega) (by omega) (by decide)
      omega
  case leaf_3 =>
    rw [right]
    exact triangular_step i.val (by omega)

-- Two calls recover the complete consecutive product because it is even.
theorem double_triangular (n : Int) :
    n * (n + 1) / 2 + n * (n + 1) / 2 = n * (n + 1) := by
  have even : n * (n + 1) % 2 = 0 := by
    rw [Int.mul_emod, Int.add_emod]
    rcases Int.emod_two_eq n with zero | one
    · simp [zero]
    · simp [one]
  have division := Int.emod_add_mul_ediv (n * (n + 1)) 2
  omega

verify test_sum_twice by
  case leaf_1 =>
    have twice := double_triangular n.val
    omega
  all_goals change n.val < 4294967296
  all_goals omega
