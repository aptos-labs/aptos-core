-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

impl Token where
  spec value where
    ensures result == 7

  spec consume where
    ensures result == 8

spec borrowed where
  ensures result == 7

spec borrowed_false where
  ensures result == 8 -- error: `value` gives 7

spec moved where
  ensures result == 8

spec moved_false where
  ensures result == 7 -- error: `consume` gives 8

spec aggregate where
  ensures result == 15

spec aggregate_false where
  ensures result == 14 -- error: 7 + 8

spec through_return where
  ensures result == 7

spec through_return_false where
  ensures result == 8 -- error: `value` gives 7
