--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectImmutableAfterMutation where

  fun run : Action U64 := do
    let owner : U64 := 0
    let writer ← &mut owner
    writer := 1
    let observation ← &owner
    let result ← *observation
    let _writerValue ← *writer
    pure result

  spec run where
    ensures True
