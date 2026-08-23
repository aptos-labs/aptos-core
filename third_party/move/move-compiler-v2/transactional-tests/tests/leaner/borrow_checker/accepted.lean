--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowAccepted where

  struct Pair has Copy, Drop, Store where
    left : U64
    right : U64

  fun multiple_immutable : Action U64 := do
    let owner : U64 := 7
    let first ← &owner
    let second ← &owner
    let firstValue ← *first
    let secondValue ← *second
    pure (firstValue + secondValue)

  spec multiple_immutable where
    ensures True

  fun disjoint_siblings : Action U64 := do
    let pair : Pair := { left := 1, right := 2 }
    let pairRef ← &mut pair
    let left ← &mut pairRef.left
    let right ← &mut pairRef.right
    left := 10
    right := 20
    let leftValue ← *left
    let rightValue ← *right
    pure (leftValue + rightValue)

  spec disjoint_siblings where
    ensures True

  fun child_then_parent : Action U64 := do
    let pair : Pair := { left := 1, right := 2 }
    let parent ← &mut pair
    let child ← &mut parent.left
    child := 8
    let _childValue ← *child
    let result ← *parent
    pure (result.left + result.right)

  spec child_then_parent where
    ensures True

--# run 0x0::LeanerBorrowAccepted::multiple_immutable

--# run 0x0::LeanerBorrowAccepted::disjoint_siblings

--# run 0x0::LeanerBorrowAccepted::child_then_parent
