-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "no_abort.move"
verex proofs where

verify routed by
  on no abort when aborts at op 1 {
    assert b <= 10
  }
  on no abort when aborts at op 2 {
    assert a <= 10
  }
