-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace shift where
  fun shifts(value : u32, distance : u8) -> (u32, u32) := do
    let _3 := core.prim.checkedShiftLeftPanic(value, distance)
    (_3, core.prim.checkedShiftRightPanic(value, distance))
