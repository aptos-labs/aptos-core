-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectGlobalOwner where
  struct Counter has Key where
    value : u64

  fun run(addr : Address) -> u64 := do
    let observation := &Counter[addr].value
    let Counter { value := removed } := move_from<Counter>(addr)
    let _observed := *observation
    removed
