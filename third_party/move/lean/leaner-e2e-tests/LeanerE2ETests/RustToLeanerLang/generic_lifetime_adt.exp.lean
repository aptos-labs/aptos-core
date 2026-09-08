-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace generic_lifetime_adt where
  struct Borrowed {a : lifetime} {T} where
    value : &[a] T

  fun read_u32(value : Borrowed<lifetime _, u32>) -> u32 := do
    let _2 := copy(value.value)
    return *_2
