-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace const_static where
  struct Point where
    x : u32

  fun count() -> u64 := do
    let _1 := &7u64
    *_1

  fun origin() -> u32 := do
    let _1 := &new Point { x := 3u32 }
    (*_1).x

  fun shared() -> u64 := do
    let _1 := & &7u64
    let _2 := copy(*_1)
    *_2
