-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexBorrowRejectImmutable where
  fun run() -> u64 := do
    let mut owner : u64 := 1
    let observation := &owner
    let writer := &mut owner
    *writer := 3
    let result := *observation
    result
