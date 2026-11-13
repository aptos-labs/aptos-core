-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec keep where
  ensures result != a -- error: the result is a

spec first_of where
  ensures result != pair[0] -- error: the result is the first component

spec between where
  ensures result != b -- error: the result is b

spec named_beside where
  ensures result != x -- error: the result is x

spec at_two_types where
  ensures result == a -- error: each half rounds down
