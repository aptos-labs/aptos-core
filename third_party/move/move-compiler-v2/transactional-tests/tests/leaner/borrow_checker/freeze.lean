--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowFreeze where

  fun run : Action U64 := do
    let owner : U64 := 3
    let writer ← &mut owner
    writer := 8
    let observation ← freeze writer
    let result ← *observation
    pure result

  spec run where
    ensures True

--# run 0x0::LeanerBorrowFreeze::run
