--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectVectorSwap where

  fun run : Action U64 := do
    let values : Vector U64 := vector![1, 2]
    let vectorRef ← &mut values
    let observation ← &vectorRef[0]
    vectorRef.swap 0 1
    let result ← *observation
    pure result

  spec run where
    ensures True
