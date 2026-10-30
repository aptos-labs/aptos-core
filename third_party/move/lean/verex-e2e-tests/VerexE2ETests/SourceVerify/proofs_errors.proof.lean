-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proofs_errors.move"
verex proofs where

-- The proof of `product` does not establish its clause: `omega` decides
-- linear arithmetic only. `square_of_sum` and `twice` have no proof here.
verify product by lean =>
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  omega
