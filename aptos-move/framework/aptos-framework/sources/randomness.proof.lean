-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "randomness.move"
verex proofs where

-- The input the hash is stated of is the input the body builds, one concatenation under the
-- bound the body's own concatenations give.
verify next_32_bytes by
  on ensures 2 {
    let input := b"APTOS_RANDOMNESS"
    let seed := borrow(PerBlockRandomness[@0x1].seed)
    let txn_hash := spec_get_txn_hash()
    let txn_counter := spec_fetch_and_increment_txn_counter()
    assert len(input) + len(seed) + len(txn_hash) + len(txn_counter) <= MAX_U64
  }
  on ensures 1 { }
  on _ {
    split PerBlockRandomness[@0x1].seed
  }
