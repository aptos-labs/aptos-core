-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec truncate where
  ensures result == a % 256

spec truncate_signed where
  ensures a % 256 <= 127 ==> result == a % 256
  ensures a % 256 > 127 ==> result == a % 256 - 256

spec narrow_signed where
  requires a >= -32768 && a <= 32767
  ensures result == a

spec to_unsigned where
  ensures a >= 0 ==> result == a
  ensures a < 0 ==> result == a + 65536

spec sign_extend where
  ensures result == a

spec sign_extend_unsigned where
  ensures a >= 0 ==> result == a
  ensures a < 0 ==> result == a + 4294967296

spec low_byte_is_zero where
  ensures result == (a % 256 == 0)

spec branch_on_truncated where
  ensures result == 1 ==> a % 256 > 100
  ensures a % 256 > 100 ==> result == 1

spec branch_on_sign where
  ensures result == (a % 256 > 127)

spec round_trip where
  ensures result == a

spec panic_on_truncation where
  panics_if a > 255
  ensures result == a
