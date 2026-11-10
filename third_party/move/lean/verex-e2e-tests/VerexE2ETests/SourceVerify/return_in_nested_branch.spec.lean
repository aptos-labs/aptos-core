-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec in_while where
  ensures a && !b && c ==> result == 1
  loop 1 where
    invariant x <= 2

spec after_a_branch where
  ensures a && !b && c && d ==> result == 1
  ensures a && !b && c && !d ==> result == 0
  loop 1 where
    invariant a ==> x == 0
