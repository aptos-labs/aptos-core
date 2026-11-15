-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Each contract misses a panic or states another value.

spec add_usize where
  ensures result == a + b

spec add_isize where
  requires -9223372036854775808 <= a + b && a + b <= 9223372036854775807
  ensures result == a + b + 1

spec div_u32 where
  ensures result == a / b

spec not_u8 where
  ensures result == 254 - a

spec shl_u8 where
  ensures true
