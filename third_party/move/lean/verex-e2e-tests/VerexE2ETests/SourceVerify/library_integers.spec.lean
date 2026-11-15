-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec shift_past_width where
  ensures result == (value * 2) % 4294967296

spec shift_right_past_width where
  ensures result == value / 8

spec signed_shift_past_width where
  ensures -1073741824 <= value && value <= 1073741823 ==> result == value * 2

spec signed_shift_right_past_width where
  ensures result == value >> 1

spec wide_shift where
  ensures distance == 64 ==> result == value

spec saturate_up where
  ensures left + right <= 255 ==> result == left + right
  ensures left + right > 255 ==> result == 255

spec saturate_down where
  ensures left - right < -128 ==> result == -128
  ensures left - right > 127 ==> result == 127
  ensures -128 <= left - right && left - right <= 127 ==> result == left - right

spec multiply where
  ensures left * right <= MAX_U64 ==> result is Some && result.0 == left * right
  ensures left * right > MAX_U64 ==> result is None

spec subtract where
  ensures left >= right ==> result is Some && result.0 == left - right
  ensures left < right ==> result is None

spec signed_add where
  ensures left + right > 32767 ==> result is None
  ensures -32768 <= left + right && left + right <= 32767 ==> result is Some

spec wrapped where
  ensures result == (left + right) % 4294967296
