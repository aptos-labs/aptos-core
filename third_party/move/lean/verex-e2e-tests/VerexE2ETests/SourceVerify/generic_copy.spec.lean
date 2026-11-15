-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec read where
  ensures result == m

spec read_mut where
  ensures result == old(m)
  ensures m == old(m)

spec pair where
  ensures spec.result[0] == x
  ensures spec.result[1] == x

spec field where
  ensures result == m[0]

spec option where
  ensures result == m

spec through_call where
  ensures result == m

spec upvar where
  ensures result == x

spec at_u8 where
  ensures result == x % 100 + x % 100

spec consumed where
  ensures result == x

spec consumed_tuple where
  ensures result == x
