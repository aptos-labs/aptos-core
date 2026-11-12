-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_determined_instances.move"
verex proofs where

verify shifted by
  on ensures 1 {
    assert v[k] < v[k + 1]
  }
  on _ { }

verify offset by
  on ensures 1 {
    assert v[k] <= v[k + 1]
  }
  on _ { }

verify first_x by
  on ensures 1 { }
  on _ { }

verify at_y by
  on ensures 1 { }
  on _ { }
