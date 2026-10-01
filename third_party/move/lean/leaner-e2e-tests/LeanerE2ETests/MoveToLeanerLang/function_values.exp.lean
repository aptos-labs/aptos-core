-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x42::function_values where
  -- unsupported Move declaration `add_one`: in function `function_values::add_one`: closures are not supported by XAST (function values are out of scope)
  -- unsupported Move declaration `add_two`: in function `function_values::add_two`: closures are not supported by XAST (function values are out of scope)
  fun __lambda__1__add_two(x : u64) -> u64 := x + 1

  fun __lambda__2__add_two(x : u64) -> u64 := x + 1

  fun __lambda__1__add_one(x : u64) -> u64 := x + 1
