-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace aggregate where
  fun array(left : u32, right : u32) -> Vector<u32, const 2> := #[left, right]

  fun first(pair : (u32, Bool)) -> u32 := pair[0u32]

  fun tuple(value : u32, flag : Bool) -> (u32, Bool) := (value, flag)
