-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectBranch where
  fun run(activate : Bool) -> u64 := do
    let mut owner : u64 := 0
    let selected := &mut owner
    let poisoned := &mut owner
    if activate then
      *selected := 1
    let result := *poisoned
    result
