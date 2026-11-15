-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- The bounds with contracts their code does not meet: each failure is
-- reported at the clause in this file.

spec max where
  ensures result >= left && result >= right
  ensures result == left || result == right

spec min where
  ensures result <= left && result <= right
  ensures result == left || result == right

-- Without `requires low <= high`, a low bound above the high one wins.
spec clamp where
  ensures low <= result && result <= high

spec raise where
  ensures slot >= floor
  ensures slot == floor
