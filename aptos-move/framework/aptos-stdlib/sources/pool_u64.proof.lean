-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "pool_u64.move"
verex proofs where

verify balance by
  on ensures 1, aborts_if 1 {
    apply lemma_mul_nonneg(num_shares, self.total_coins)
  }
  on _ { }

verify buy_in by
  on aborts_if 1 when aborts at call 1 of add_shares {
    split spec_contains(self, shareholder)
    assert coins_amount > 0
  }
  on aborts_if 1, aborts_if 2, aborts_if 3, aborts_if 4 when returns {
    split spec_contains(self, shareholder)
  }
  on ensures 1 {
    assert self'.shares == spec_set(self.shares, shareholder,
      spec_get(self.shares, shareholder) +
        spec_amount_to_shares_with_total_coins(self, coins_amount, self.total_coins)) by congr
  }
  on _ { }
verify redeem_shares by
  after call 1 of deduct_shares {
    assert self'.total_coins == self.total_coins - redeemed_coins by congr
    assert self'.total_shares == self.total_shares - shares_to_redeem by congr
  }
  on _ { }
