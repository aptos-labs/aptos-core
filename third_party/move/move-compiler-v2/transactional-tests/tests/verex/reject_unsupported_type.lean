-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexRejectUnsupportedType where
  fun unsupported(_value : Nat) -> u64 := 1
