-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectPoisonedWrite where
  fun run() -> Unit := do
    let mut owner : u64 := 0
    let selected := &mut owner
    let poisoned := &mut owner
    *selected := 1
    *poisoned := 2
