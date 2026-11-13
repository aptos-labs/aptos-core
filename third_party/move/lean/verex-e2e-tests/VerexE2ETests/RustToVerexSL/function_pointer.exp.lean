-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace function_pointer where
  fun increment(value : u32) -> u32 := value ^ 3u32

  fun apply(value : u32) -> u32 := do
    let «function» := function[Fn(u32) -> u32 has Copy, Drop](increment)
    invoke(«function», value)
