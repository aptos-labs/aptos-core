-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec read where
  ensures result != m

spec read_mut where
  ensures m != old(m)

spec pair where
  ensures spec.result[0] != spec.result[1]

spec field where
  ensures result != m[0]

spec option where
  ensures result != m

spec through_call where
  ensures result != m

spec upvar where
  ensures result != x

spec at_u8 where
  ensures result == x

spec consumed where
  ensures result != x

spec consumed_tuple where
  ensures result != x
