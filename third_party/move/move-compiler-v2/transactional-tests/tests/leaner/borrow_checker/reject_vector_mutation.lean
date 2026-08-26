--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectVectorMutation where

  fun run : Action U64 := do
    let values : Vector U64 := vector![1, 2]
    let vectorRef ← &mut values
    let observation ← &vectorRef[0]
    Move.Vector.insert vectorRef 2 3
    let result ← *observation
    pure result

  spec run where
    ensures True
