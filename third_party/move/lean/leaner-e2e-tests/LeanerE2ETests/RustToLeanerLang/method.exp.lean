-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace method where
  struct Counter where
    value : u32

  fun add(self : Counter, amount : u32) -> u32 :=
    core.prim.checkedAddPanic(self.value, amount)

  fun call_method(value : u32, amount : u32) -> u32 :=
    new Counter { value }.add(amount)
