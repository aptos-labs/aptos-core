--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectPoisonedCall where

  fun observe (input : &mut U64) : Action U64 := do
    let result ← *input
    pure result

  spec observe (_input : &mut U64) where
    ensures True

  fun run : Action U64 := do
    let owner : U64 := 0
    let selected ← &mut owner
    let poisoned ← &mut owner
    selected := 1
    observe poisoned

  spec run where
    ensures True
