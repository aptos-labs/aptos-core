-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace partial_move where
  struct Leaf where
    value : u32

  struct Pair where
    first : Leaf
    second : Leaf

  fun take_first(pair : Pair) -> Leaf := move(pair.first)
