-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerRejectContinueOutsideLoop where
  fun countdown(value : u64) -> u64 := do
    if value == 0 then return 0
    continue
