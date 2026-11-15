-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexRejectVerification where
  fun wrong_increment(value : u64) -> u64 := value + 1
  spec wrong_increment where
    ensures result == value
    aborts_if value + 1 > MAX_U64
