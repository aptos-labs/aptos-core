--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectReturn where

  fun run : Action (&U64) := do
    let owner : U64 := 7
    let result ← &owner
    pure result

  spec run where
    ensures True
