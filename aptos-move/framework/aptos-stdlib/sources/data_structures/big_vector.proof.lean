-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "big_vector.move"
verex proofs where

-- The position of `i` lies in a present bucket: the quotient is below the bucket count, the
-- remainder below the bucket's length, which is `bucket_size` except in the last bucket, whose
-- length is what `end_index` leaves over. The arithmetic holds on every path.
verify borrow by
  at entry {
    let s := self.bucket_size
    let n := spec_table_len(self.buckets)
    apply lemma_div_nonneg(i, s)
    apply lemma_mul_mono(i, i, 1, s)
    apply lemma_div_le(i, i, s)
    apply i < self.end_index ==> lemma_div_lt(i, n, s)
    apply lemma_div_mod(i, s)
  }
  on aborts_if 1 {
    let s := self.bucket_size
    let n := spec_table_len(self.buckets)
    assert spec_table_contains(self.buckets, i / s)
    assert n != 0
    split i / s == n - 1
    if i / s < n - 1 {
      assert len(table_with_length::spec_get(self.buckets, i / s)) == s
    } else { }
  }
  on ensures 1 { }
  on _ { }
