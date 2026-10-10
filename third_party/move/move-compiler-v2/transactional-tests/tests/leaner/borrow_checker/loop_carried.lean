-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowLoop where
  -- One mutable handle remains the unique mutation lineage across every loop
  -- iteration.
  fun run() -> u64 := do
    let mut owner : u64 := 0
    let writer := &mut owner
    let mut count : u64 := 0
    while count < 3 do
      *writer := *writer + 2
      count := count + 1
    let result := *writer
    result

--# run 0x0::LeanerBorrowLoop::run
