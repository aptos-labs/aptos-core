-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectPoisonedReborrow where
  struct Box has Copy, Drop, Store where
    value : u64

  fun run() -> u64 := do
    let mut owner := new Box { value := 0 }
    let selected := &mut owner
    let poisoned := &mut owner
    *selected := new Box { value := 1 }
    let child := &mut poisoned.value
    let result := *child
    result
