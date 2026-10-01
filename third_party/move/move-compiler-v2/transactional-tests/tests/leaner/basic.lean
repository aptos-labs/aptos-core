-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerTxn where
  entry fun fail(code : u64) -> Unit := abort(code)

--# run 0x0::LeanerTxn::fail --args 7u64
