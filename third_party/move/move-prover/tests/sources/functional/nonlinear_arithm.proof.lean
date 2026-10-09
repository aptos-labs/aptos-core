-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Five increasing unsigned integers have product zero, or at least
-- 1 * 2 * 3 * 4 * 5 = 120. In either case their product cannot be 72.
verify mul5 by
  by_cases zero : a.val = 0
  · simp [zero]
  have ha : 1 ≤ a.val := by omega
  have hb : 2 ≤ b.val := by omega
  have hc : 3 ≤ c.val := by omega
  have hd : 4 ≤ d.val := by omega
  have he : 5 ≤ e.val := by omega
  have hab : (2 : Int) ≤ a.val * b.val := by
    simpa using Int.mul_le_mul ha hb (by decide) (by omega)
  have habc : (6 : Int) ≤ a.val * b.val * c.val := by
    simpa using Int.mul_le_mul hab hc (by decide) (by omega)
  have habcd : (24 : Int) ≤ a.val * b.val * c.val * d.val := by
    simpa using Int.mul_le_mul habc hd (by decide) (by omega)
  have habcde : (120 : Int) ≤ a.val * b.val * c.val * d.val * e.val := by
    simpa using Int.mul_le_mul habcd he (by decide) (by omega)
  omega
