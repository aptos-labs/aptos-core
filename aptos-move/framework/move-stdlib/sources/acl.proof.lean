-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "acl.move"
verex proofs where
verify remove by
  on data invariant 1 {
    split i < index
    split j < index
    compute
    let a := if i < index then i else i + 1
    let b := if j < index then j else j + 1
    assert self.list[a] == self.list[b]
    assert a == b
  }
  on ensures 1 {
    apply lemma_vector_index_of(self.list, addr)
    apply lemma_vector_contains_remove(self.list, index, addr)
  }
  on _ {
    if spec_contains(self, addr) {
      apply lemma_vector_index_of(self.list, addr)
      assert index < self.list.length
    }
  }
