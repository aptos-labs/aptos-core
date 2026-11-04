-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

impl Map where
  spec find where
    pragma opaque
    ensures result is Some ==> result.0 < self.entries.length

  spec get_or_zero where
    ensures true

  spec next_or_zero where
    ensures true -- error: the index may be out of bounds

  spec value_is_key where
    ensures result == key -- error: the value is not the key
