-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectPoisonedCall where
  fun observe(input : &mut u64) -> u64 := do
    let result := *input
    result

  fun run() -> u64 := do
    let mut owner : u64 := 0
    let selected := &mut owner
    let poisoned := &mut owner
    *selected := 1
    observe(poisoned)
