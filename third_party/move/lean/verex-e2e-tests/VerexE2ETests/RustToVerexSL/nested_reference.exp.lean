-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace nested_reference where
  fun read_nested(value : & &u32) -> u32 := do
    let _2 := copy(*value)
    *_2
