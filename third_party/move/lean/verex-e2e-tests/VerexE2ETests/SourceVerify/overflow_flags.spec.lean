-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec add_flag where
  ensures result == (a + b > 255)

spec subtract_flag where
  ensures result == (a < b)

spec multiply_flag where
  ensures result == (a * b > 65535)

spec signed_add_flag where
  ensures result == (a + b < -128 || a + b > 127)

spec signed_subtract_flag where
  ensures result == (a - b < -2147483648 || a - b > 2147483647)

spec wrapped_value where
  ensures a + b <= 255 ==> result == a + b
  ensures a + b > 255 ==> result == a + b - 256

spec saturating where
  ensures a + b <= 255 ==> result == a + b
  ensures a + b > 255 ==> result == 255

spec checked where
  panics_if a + b > 4294967295
  ensures result == a + b

spec checked_multiply where
  panics_if a * b > 255
  ensures result == a * b

spec pointer_flag where
  ensures result == (a + b > 18446744073709551615)

spec wide_flag where
  ensures result == (a * b > 340282366920938463463374607431768211455)
