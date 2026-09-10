-- Copyright © Aptos Foundation

--# publish

import Move

module LeanerTxn where

  /-! ## Functions -/

  @[entry]
  fun fail (code : U64) : Action Unit := do
    abort code

/-! ## Tests -/

--# run 0x0::LeanerTxn::fail --args 7u64
