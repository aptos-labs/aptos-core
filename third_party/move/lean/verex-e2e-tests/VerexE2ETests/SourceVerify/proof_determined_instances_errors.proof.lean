-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_determined_instances_errors.move"
verex proofs where

verify not_an_instance by
  on ensures 1 {
    assert v[k] < v[k + 1]
  }
  on _ { }
