-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerRejectVerification where
  fun wrong_increment(value : u64) -> u64 := value + 1
  spec wrong_increment where
    ensures result == value
    aborts_if value + 1 > MAX_U64
