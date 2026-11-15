-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace modules where
  struct Pair where
    left : u32
    right : u32

  impl Pair where
    fun first(self : Pair) -> u32 := inner::nested::id(self.left)

  mod inner where
    fun second(pair : Pair) -> u32 := pair.right

    mod nested where
      struct Wrapper where
        value : u32

      fun id(value : u32) -> u32 := do
        let _2 := new Wrapper { value }
        _2.value

      fun through_root(value : u32) -> u32 := root(value)

  fun root(value : u32) -> u32 := helper(value)

  fun helper(value : u32) -> u32 := value
