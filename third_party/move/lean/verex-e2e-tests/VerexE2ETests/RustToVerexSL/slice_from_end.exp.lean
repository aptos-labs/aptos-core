-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace slice_from_end where
  fun last_or_zero(values : &Slice<u32>) -> u32 := do
    let mut _0 : u32
    if values.length >= 1usize then
      let last := &values[values.length - 1usize]
      _0 := *last
    else _0 := 0u32
    _0
