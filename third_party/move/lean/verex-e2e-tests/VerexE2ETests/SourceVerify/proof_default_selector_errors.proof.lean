-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_default_selector_errors.move"
verex proofs where

verify pick by
  on _ when if 2 then { }
  on _ { }
