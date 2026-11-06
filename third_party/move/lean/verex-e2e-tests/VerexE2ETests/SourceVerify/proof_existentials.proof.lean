-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_existentials.move"
verex proofs where

verify even by
  on ensures 1 {
    let half : u64 where x == 2 * half
  }
  on _ { }

verify evens by
  on ensures 1 {
    let half : u64 where v[i] == 2 * half
    assert half + half == v[i]
  }
  on _ { }

verify stated by
  on ensures 1 {
    assert ∃ (half : u64), v[i] == 2 * half
  }
  on _ { }
