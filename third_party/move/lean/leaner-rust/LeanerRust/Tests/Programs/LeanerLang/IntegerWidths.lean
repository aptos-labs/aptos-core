-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace integer_widths where
  fun widths(
    _u8 : u8, _u16 : u16, _u32 : u32, _u64 : u64, _u128 : u128, _i8 : i8,
    _i16 : i16, _i32 : i32, _i64 : i64, _i128 : i128
  ) -> i64 := -7i64
