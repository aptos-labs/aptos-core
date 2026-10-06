-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- `mul_div`: the product's quotient by 2^32, shifted back, is at most the
-- product, and the quotient by `y` at most `x`.
verify mul_div by
  case leaf_1 =>
    have fits := ‹¬((_ : Int) = 0 ∨ MAX_U64 < _)›
    have quotient := ‹¬((_ : Int) = 0 ∨ MAX_U64 < _) → _ = _› fits
    rw [quotient, ‹result.val.shiftLeft 32 = result.val * 4294967296›]
    apply Int.ediv_le_of_le_mul (by omega)
    grind
