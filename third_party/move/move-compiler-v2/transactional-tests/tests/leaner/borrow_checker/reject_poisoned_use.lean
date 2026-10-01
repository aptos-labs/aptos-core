-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectPoisoned where
  fun run() -> u64 := do
    let mut owner : u64 := 0
    let selected := &mut owner
    let poisoned := &mut owner
    *selected := 1
    let result := *poisoned
    result
