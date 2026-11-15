-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod m where
  spec one where
    ensures result == 2 -- error: `one` returns 1

impl Counter where
  spec get where
    ensures result == self.value + 1 -- error: `get` returns the value
