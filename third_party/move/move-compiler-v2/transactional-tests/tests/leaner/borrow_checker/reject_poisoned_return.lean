--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectPoisonedReturn where

  fun run : Action (&mut U64) := do
    let owner : U64 := 0
    let selected ← &mut owner
    let poisoned ← &mut owner
    selected := 1
    pure poisoned

  spec run where
    ensures True
