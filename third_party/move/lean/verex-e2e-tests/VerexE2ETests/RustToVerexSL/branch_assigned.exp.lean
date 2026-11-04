-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace branch_assigned where
  fun larger {a : lifetime}(
    left : &[a] mut u32, right : &[a] mut u32
  ) -> &[a] mut u32 := do
    let mut _3 : &mut u32
    if *left >= *right then _3 := left else _3 := right
    &mut *_3
