-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_items_errors.move"
verex proofs where

-- A specification belongs to the module, not to its proof file.
spec double where
  ensures result >= a
