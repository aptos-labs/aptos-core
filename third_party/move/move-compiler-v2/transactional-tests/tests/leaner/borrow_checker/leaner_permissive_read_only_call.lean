--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowPermissiveCall where

  fun observe_two (left : &mut U64) (right : &mut U64) : Action U64 := do
    let leftValue ← *left
    let rightValue ← *right
    pure (leftValue + rightValue)

  spec observe_two (_left : &mut U64) (_right : &mut U64) where
    ensures True

  /-- The callee summary is read-only, so Leaner does not activate either
  overlapping mutable argument. -/
  fun run : Action U64 := do
    let owner : U64 := 9
    let first ← &mut owner
    let second ← &mut owner
    observe_two first second

  spec run where
    ensures True

--# run 0x0::LeanerBorrowPermissiveCall::run
