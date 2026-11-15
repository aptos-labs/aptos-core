-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec add_u8 where
  panics_if a + b > 255
  ensures result == a + b

spec add_u16 where
  requires a + b <= 65535
  ensures result == a + b

spec add_u32 where
  requires a + b <= 4294967295
  ensures result == a + b

spec add_u64 where
  requires a + b <= 18446744073709551615
  ensures result == a + b

spec add_u128 where
  requires a + b <= 340282366920938463463374607431768211455
  ensures result == a + b

spec add_usize where
  requires a + b <= 18446744073709551615
  ensures result == a + b

spec add_i8 where
  requires -128 <= a + b && a + b <= 127
  ensures result == a + b

spec add_i16 where
  requires -32768 <= a + b && a + b <= 32767
  ensures result == a + b

spec add_i32 where
  requires -2147483648 <= a + b && a + b <= 2147483647
  ensures result == a + b

spec add_i64 where
  requires -9223372036854775808 <= a + b && a + b <= 9223372036854775807
  ensures result == a + b

spec add_i128 where
  requires -170141183460469231731687303715884105728 <= a + b && a + b <= 170141183460469231731687303715884105727
  ensures result == a + b

spec add_isize where
  requires -9223372036854775808 <= a + b && a + b <= 9223372036854775807
  ensures result == a + b

spec sub_usize where
  panics_if a < b
  ensures result == a - b

spec mul_u32 where
  requires a * b <= 4294967295
  ensures result == a * b

spec div_u32 where
  panics_if b == 0
  ensures result == a / b

spec rem_usize where
  panics_if b == 0
  ensures result == a % b

spec div_i32 where
  requires b > 0
  requires a >= 0
  ensures result == a / b

spec half where
  ensures result <= a

spec neg_i32 where
  requires a > -2147483648
  ensures result == -a

spec not_u8 where
  ensures result == 255 - a

spec not_i8 where
  ensures result == -1 - a

spec shl_u8 where
  panics_if n >= 8

spec shr_i32 where
  panics_if n >= 32

spec bool_and where
  ensures result == (a && b)

spec bool_or where
  ensures result == (a || b)

spec bool_xor where
  ensures result == (a != b)

spec bool_less where
  ensures result == (!a && b)

spec bool_at_least where
  ensures result == (a || !b)
