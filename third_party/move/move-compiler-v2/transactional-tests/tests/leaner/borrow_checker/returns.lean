--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowReturns where

  struct Pair has Copy, Drop, Store where
    left : U64
    right : U64

  fun identity (input : &U64) : Action (&U64) := do
    pure input

  spec identity (_input : U64) where
    ensures True

  fun borrow_left (input : &Pair) : Action (&U64) := do
    &input.left

  spec borrow_left (_input : Pair) where
    ensures True

  fun direct : Action U64 := do
    let owner : U64 := 13
    let observation ← &owner
    let returned ← identity observation
    let result ← *returned
    pure result

  spec direct where
    ensures True

  fun nested : Action U64 := do
    let owner : Pair := { left := 17, right := 19 }
    let observation ← &owner
    let returned ← borrow_left observation
    let result ← *returned
    pure result

  spec nested where
    ensures True

--# run 0x0::LeanerBorrowReturns::direct

--# run 0x0::LeanerBorrowReturns::nested
