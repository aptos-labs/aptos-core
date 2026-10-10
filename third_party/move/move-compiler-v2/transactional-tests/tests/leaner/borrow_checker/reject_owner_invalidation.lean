-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectOwner where
  fun run() -> u64 := do
    let mut owner : u64 := 0
    let observation := &owner
    owner := 1
    let result := *observation
    result
