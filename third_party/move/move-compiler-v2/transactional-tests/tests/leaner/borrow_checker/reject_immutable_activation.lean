-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectImmutable where
  fun run() -> u64 := do
    let mut owner : u64 := 1
    let observation := &owner
    let writer := &mut owner
    *writer := 3
    let result := *observation
    result
