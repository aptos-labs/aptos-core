--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRepeatedWrites where

  fun run : Action U64 := do
    let owner : U64 := 1
    let writer ← &mut owner
    writer := 2
    writer := *writer + 3
    let result ← *writer
    pure result

  spec run where
    ensures True

--# run 0x0::LeanerBorrowRepeatedWrites::run
