-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proofs.move"
verex proofs where

-- The expansion of the square proves the result; the abort conditions need
-- the square's sign where the deciders do not establish them.
verify square_of_sum by
  verex_obligations
  verex_on
  | ensures_1 =>
    verex_prepare
    rw [Int.add_mul, Int.mul_add, Int.mul_add, Int.mul_comm b.val a.val, Int.mul_assoc]
    omega
  | _ =>
    first
    | verex_decide
    | (verex_prepare
       all_goals first
       | verex_decide
       | (have := Int.mul_nonneg ‹0 ≤ a.val + b.val› ‹0 ≤ a.val + b.val›
          omega))
