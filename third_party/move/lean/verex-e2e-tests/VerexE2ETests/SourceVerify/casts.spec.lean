-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec truncate where
  ensures result == a % 256

spec to_unsigned where
  requires a >= 0
  ensures result == a

spec sign_extend where
  ensures result == a

spec zero_extend where
  ensures result == a

spec to_usize where
  ensures result == a

spec from_usize where
  ensures result == a

spec to_isize where
  ensures result == a
