--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectVector where

  fun run : Action U64 := do
    let values : Vector U64 := vector![1, 2]
    let first ← &mut values[0]
    let second ← &mut values[1]
    first := 8
    let result ← *second
    pure result

  spec run where
    ensures True
