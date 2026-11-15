-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace subslice where
  fun tail_len(values : &Slice<u32>) -> usize :=
    if values.length >= 1usize then
      (&core.subsliceFromEnd[1, 0](*values)).length
    else 0usize
