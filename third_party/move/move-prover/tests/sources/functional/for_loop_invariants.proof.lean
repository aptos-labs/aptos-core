-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Triangular sums use Move integer division (truncating toward zero).
-- Nonnegative loop indices let us use Euclidean division identities.
theorem triangular_nonneg (i : Int) (lo : 0 ≤ i) : 0 ≤ i * (i - 1) := by
  by_cases zero : i = 0
  · simp [zero]
  · exact Int.mul_nonneg lo (by omega)

theorem triangular_step (i : Int) (lo : 0 ≤ i) :
    (i * (i - 1)).tdiv 2 + i = ((i + 1) * (i + 1 - 1)).tdiv 2 := by
  simp only [Int.add_sub_cancel]
  rw [Int.tdiv_eq_ediv_of_nonneg (triangular_nonneg i lo),
    Int.tdiv_eq_ediv_of_nonneg (Int.mul_nonneg (by omega) lo)]
  have h : (i + 1) * i = i * (i - 1) + i * 2 := by grind
  rw [h, Int.add_mul_ediv_right _ _ (by decide)]

theorem triangular_bounds (i : Int) (lo : 0 ≤ i) (hi : i ≤ 1000) :
    0 ≤ (i * (i - 1)).tdiv 2 ∧ (i * (i - 1)).tdiv 2 ≤ 500000 := by
  have nonnegative := triangular_nonneg i lo
  rw [Int.tdiv_eq_ediv_of_nonneg nonnegative]
  have square : i * i ≤ 1000000 := by
    simpa using Int.mul_le_mul hi hi lo (by decide)
  have product : i * (i - 1) ≤ i * i :=
    Int.mul_le_mul_of_nonneg_left (by omega) lo
  omega

@[lir_denote_norm] theorem triangular_div (i : Int) :
    (i * (i - 1)).tdiv 2 = (i * (i - 1)) / 2 := by
  apply Int.tdiv_eq_ediv_of_nonneg
  by_cases h : 0 ≤ i
  · exact triangular_nonneg i h
  · exact Int.mul_nonneg_of_nonpos_of_nonpos (by omega) (by omega)

verify sum_range by
  all_goals first | omega |
    have bounded := triangular_bounds i.val (by omega) (by omega)
    have step := triangular_step i.val (by omega)
    simp only [triangular_div] at *
    first | omega | grind
verify sum_from by
  all_goals first | omega |
    have bounded := triangular_bounds i.val (by omega) (by omega)
    have lower := triangular_bounds a.val (by omega) (by omega)
    have step := triangular_step i.val (by omega)
    simp only [triangular_div] at *
    first | omega | grind
verify sum_skip_first by
  all_goals first | omega |
    have bounded := triangular_bounds i.val (by omega) (by omega)
    have step := triangular_step i.val (by omega)
    simp only [triangular_div] at *
    first | omega | grind
