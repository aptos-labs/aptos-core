--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectLoop where

  fun run : Action U64 := do
    let owner : U64 := 0
    let selected ← &mut owner
    let poisoned ← &mut owner
    let mut count : U64 := 0
    while count < 1 do
      selected := 1
      count := count + 1
    let result ← *poisoned
    pure result

  spec run where
    ensures True
