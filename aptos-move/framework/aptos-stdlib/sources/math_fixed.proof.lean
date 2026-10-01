-- `math128::sqrt` of a value below 2^96 fits in 64 bits: its square is at
-- most the value. Nonlinear, so not decided by `omega` alone.
theorem sqrt_fits_u64 {r s : Int} (zero : s = 0 → r = 0) (square : 0 < s → r * r ≤ s)
    (nonnegative : 0 ≤ s) (bound : s < 79228162514264337593543950336) (root : 0 ≤ r) :
    r ≤ 18446744073709551615 := by
  by_cases positive : 0 < s
  · have := square positive
    refine Int.not_lt.mp fun large => ?_
    have wide : (18446744073709551616 : Int) ≤ r := by omega
    have := Int.mul_le_mul wide wide (by omega) root
    omega
  · have := zero (by omega)
    omega

verify sqrt by
  case leaf_1 =>
    have := sqrt_fits_u64 (r := result.val) ‹_› ‹_› (by omega) (by omega) (by omega)
    omega
