-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec keep where
  ensures result == a

spec first_of where
  ensures result == pair[0]

spec between where
  ensures result == b

spec named_beside where
  ensures result == x

spec at_two_types where
  ensures result == a / 2 + a / 2
