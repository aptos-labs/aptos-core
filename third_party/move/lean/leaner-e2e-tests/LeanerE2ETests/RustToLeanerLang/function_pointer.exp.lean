-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace function_pointer where
  fun increment(value : u32) -> u32 := value ^ 3u32

  fun apply(value : u32) -> u32 := do
    let «function» := function[Fn(u32) -> u32 has Copy, Drop](increment)
    return invoke(«function», value)
