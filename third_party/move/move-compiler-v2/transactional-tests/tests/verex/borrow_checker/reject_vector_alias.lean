-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexBorrowRejectVector where
  fun run() -> u64 := do
    let mut values := vector<u64>[1, 2]
    let first := &mut values[0]
    let second := &mut values[1]
    *first := 8
    let result := *second
    result
