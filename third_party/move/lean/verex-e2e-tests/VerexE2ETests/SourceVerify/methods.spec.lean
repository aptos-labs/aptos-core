-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

impl Counter where
  spec start where
    ensures result.value == value

  spec add where
    requires self.value + amount <= 4294967295
    ensures result == self.value + amount

impl Gauge where
  spec add where
    requires self.value + amount + 1 <= 4294967295
    ensures result == self.value + amount + 1

spec call_method where
  requires value + amount <= 4294967295
  ensures result == value + amount

spec call_gauge where
  requires value + 2 <= 4294967295
  ensures result == value + 2
