-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace «struct» where
  struct Pair where
    first : u32
    second : u32

  fun make(first : u32, second : u32) -> Pair := new Pair { first, second }

  fun second(pair : Pair) -> u32 := pair.second
