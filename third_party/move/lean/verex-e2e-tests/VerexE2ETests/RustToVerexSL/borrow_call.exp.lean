-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace borrow_call where
  fun read(value : &u32) -> u32 := *value

  fun borrow_then_read(value : u32) -> u32 := read(&value)
