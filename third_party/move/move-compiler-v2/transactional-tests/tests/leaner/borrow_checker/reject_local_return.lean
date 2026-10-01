-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectReturn where
  fun run() -> &u64 := do
    let owner : u64 := 7
    let result := &owner
    result
