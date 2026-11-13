-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexRejectContinueOutsideLoop where
  fun countdown(value : u64) -> u64 := do
    if value == 0 then return 0
    continue
