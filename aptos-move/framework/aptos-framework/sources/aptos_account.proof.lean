-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "aptos_account.move"
verex proofs where

-- `exists_at` returned, so the account exists, and `is_account_registered` needed the coin info.
verify assert_account_is_registered_for_apt by
  on aborts_if 1, aborts_if 2, requires of canonical, requires of is_account_registered, requires of exists_at { }
  on _ { }
