--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerRejectVerification where

  fun wrong_increment (value : U64) : U64 := value + 1

  spec wrong_increment (value : U64) where
    ensures result = value

  verify wrong_increment
