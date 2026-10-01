-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowFreeze where
  fun run() -> u64 := do
    let mut owner : u64 := 3
    let writer := &mut owner
    *writer := 8
    let observation : &u64 := writer
    let result := *observation
    result

--# run 0x0::LeanerBorrowFreeze::run
