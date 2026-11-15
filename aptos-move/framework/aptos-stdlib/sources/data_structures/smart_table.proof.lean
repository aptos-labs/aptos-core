-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "smart_table.move"
verex proofs where

verify contains by
  on invariant 3 step {
    split j == i
  }
  on assert 1 {
    if i < bucket.length && bucket[i].hash == hash && bucket[i].key == key {
      assert (∃ (j : Int), 0 <= j && j < bucket.length &&
        bucket[j].hash == hash && (bucket[j].hash == hash && bucket[j].key == key)) by witness i
    }
  }
  on _ { }
