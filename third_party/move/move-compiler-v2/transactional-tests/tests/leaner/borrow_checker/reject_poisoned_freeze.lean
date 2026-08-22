--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectPoisonedFreeze where

  fun run : Action U64 := do
    let owner : U64 := 0
    let selected ← &mut owner
    let poisoned ← &mut owner
    selected := 1
    let observation ← freeze poisoned
    let result ← *observation
    pure result

  spec run where
    ensures True
