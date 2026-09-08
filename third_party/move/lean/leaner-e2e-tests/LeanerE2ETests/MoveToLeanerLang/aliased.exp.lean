-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! A module declared under a named address, with a receiver-style function. -/
leaner module 0x1::counter where
  struct Counter has Drop, Key where
    value : u64

  /--
  Reads the counter's value (receiver style: `c.value()`).
  -/
  public fun value(self : &Counter) -> u64 := self.value

  public fun bump(self : &mut Counter) -> Unit := self.value := self.value + 1

  public fun fresh() -> Counter := new Counter { value := 0 }

  public fun bumped_twice() -> u64 := do
    let mut c := fresh()
    c.bump()
    c.bump()
    return c.value()

  spec bumped_twice where
    ensures result == 2
