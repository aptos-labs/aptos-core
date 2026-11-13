-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexBorrowRejectPoisonedFreeze where
  fun run() -> u64 := do
    let mut owner : u64 := 0
    let selected := &mut owner
    let poisoned := &mut owner
    *selected := 1
    let observation : &u64 := poisoned
    let result := *observation
    result
