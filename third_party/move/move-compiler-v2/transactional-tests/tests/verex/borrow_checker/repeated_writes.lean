-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexBorrowRepeatedWrites where
  fun run() -> u64 := do
    let mut owner : u64 := 1
    let writer := &mut owner
    *writer := 2
    *writer := *writer + 3
    let result := *writer
    result

--# run 0x0::VerexBorrowRepeatedWrites::run
