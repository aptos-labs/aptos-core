--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectPoisonedWrite where

  fun run : Action Unit := do
    let owner : U64 := 0
    let selected ← &mut owner
    let poisoned ← &mut owner
    selected := 1
    poisoned := 2

  spec run where
    ensures True
