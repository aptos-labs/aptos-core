-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectPoisonedReturn where
  fun run(owner : &mut u64) -> &mut u64 := do
    let selected := &mut *owner
    let poisoned := &mut *owner
    *selected := 1
    poisoned
