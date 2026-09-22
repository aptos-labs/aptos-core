-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace overflowing where
  fun add(left : u8, right : u8) -> (u8, Bool) := overflowing_add(left, right)

  fun subtract(left : i8, right : i8) -> (i8, Bool) :=
    overflowing_subtract(left, right)

  fun multiply(left : u16, right : u16) -> (u16, Bool) :=
    overflowing_multiply(left, right)
