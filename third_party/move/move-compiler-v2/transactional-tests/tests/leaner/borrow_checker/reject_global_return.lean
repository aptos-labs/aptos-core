-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectGlobalReturn where
  struct Counter has Key where
    value : u64

  fun run(addr : Address) -> &u64 := &Counter[addr].value
