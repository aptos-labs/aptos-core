--# publish

import Move

move_module LeanerRejectNonSelfContinue where

  @[move_fun]
  fun twice (value : U64) : U64 := value + value

  @[move_fun]
  fun invalid (value : U64) : U64 := continue twice value
