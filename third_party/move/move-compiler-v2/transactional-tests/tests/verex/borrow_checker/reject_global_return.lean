-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexBorrowRejectGlobalReturn where
  struct Counter has Key where
    value : u64

  fun run(addr : Address) -> &u64 := &Counter[addr].value
