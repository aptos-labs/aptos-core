-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace integer_cast where
  fun cast_values(unsigned : u32, signed : i16) -> (u8, u16, i32) :=
    (unsigned as u8, signed as u16, signed as i32)
