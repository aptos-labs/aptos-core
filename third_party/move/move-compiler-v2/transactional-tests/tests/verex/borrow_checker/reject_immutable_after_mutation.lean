-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexBorrowRejectImmutableAfterMutation where
  fun run() -> u64 := do
    let mut owner : u64 := 0
    let writer := &mut owner
    *writer := 1
    let observation := &owner
    let result := *observation
    let _writerValue := *writer
    result
