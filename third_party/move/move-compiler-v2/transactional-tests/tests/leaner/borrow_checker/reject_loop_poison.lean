-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectLoop where
  fun run() -> u64 := do
    let mut owner : u64 := 0
    let selected := &mut owner
    let poisoned := &mut owner
    let mut count : u64 := 0
    while count < 1 do
      *selected := 1
      count := count + 1
    let result := *poisoned
    result
