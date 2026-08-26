--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectOwner where

  fun run : Action U64 := do
    let mut owner : U64 := 0
    let observation ← &owner
    owner := 1
    let result ← *observation
    pure result

  spec run where
    ensures True
