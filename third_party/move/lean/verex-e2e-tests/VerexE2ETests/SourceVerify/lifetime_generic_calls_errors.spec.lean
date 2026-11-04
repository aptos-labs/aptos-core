-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

impl Generic where
  spec inner where
    ensures result == self.r

  spec again where
    ensures result != self.r

spec use_inner where
  ensures result != h.r

spec use_again where
  ensures result == h.r

spec use_unspecified where
  ensures result != h.r
