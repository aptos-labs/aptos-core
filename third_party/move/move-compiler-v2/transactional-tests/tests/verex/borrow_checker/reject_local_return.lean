-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexBorrowRejectReturn where
  fun run() -> &u64 := do
    let owner : u64 := 7
    let result := &owner
    result
