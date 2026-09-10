--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectImmutable where

  fun run : Action U64 := do
    let owner : U64 := 1
    let observation ← &owner
    let writer ← &mut owner
    writer := 3
    let result ← *observation
    pure result

  spec run where
    ensures True
