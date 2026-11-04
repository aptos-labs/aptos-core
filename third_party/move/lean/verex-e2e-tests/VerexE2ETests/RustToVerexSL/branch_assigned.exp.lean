-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace branch_assigned where
  fun larger(left : &mut u32, right : &mut u32) -> &mut u32 := do
    let mut _3 : &mut u32
    if *left >= *right then _3 := left else _3 := right
    &mut *_3
