--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectBranch where

  fun run (activate : Bool) : Action U64 := do
    let owner : U64 := 0
    let selected ← &mut owner
    let poisoned ← &mut owner
    if activate then
      selected := 1
    let result ← *poisoned
    pure result

  spec run (activate : Bool) where
    ensures True
