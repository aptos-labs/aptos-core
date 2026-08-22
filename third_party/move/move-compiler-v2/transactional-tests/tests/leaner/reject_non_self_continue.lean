--# publish

import Move

module LeanerRejectNonSelfContinue where

  /-! ## Functions -/

  @[move_fun]
  fun twice (value : U64) : U64 := value + value

  @[move_fun]
  fun invalid (value : U64) : U64 := continue twice value
