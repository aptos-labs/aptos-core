-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex module 0x42::cast_comparison where
  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  public fun direct(a : u64, b : u8) -> Bool := (a as u8) < b

  public fun nested(a : u8, b : u64, c : u8) -> Bool := (a + b as u8) < c

  public fun at_most(a : u64, b : u8) -> Bool := a as u8 <= b
