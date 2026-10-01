-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerRejectUnsupportedType where
  fun unsupported(_value : Nat) -> u64 := 1
