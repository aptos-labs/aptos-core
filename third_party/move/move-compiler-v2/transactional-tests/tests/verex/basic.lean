-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexTxn where
  entry fun fail(code : u64) -> Unit := abort(code)

--# run 0x0::VerexTxn::fail --args 7u64
