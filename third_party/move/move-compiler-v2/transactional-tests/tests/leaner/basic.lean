-- Copyright © Aptos Foundation

--# publish

import Move

move_module LeanerTxn where

  @[entry]
  fun fail (code : U64) : Action Unit := do
    abort code

--# run 0x0::LeanerTxn::fail --args 7u64
