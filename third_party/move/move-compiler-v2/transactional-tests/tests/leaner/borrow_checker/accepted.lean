-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowAccepted where
  struct Pair has Copy, Drop, Store where
    left : u64
    right : u64

  fun multiple_immutable() -> u64 := do
    let owner : u64 := 7
    let first := &owner
    let second := &owner
    let firstValue := *first
    let secondValue := *second
    firstValue + secondValue

  fun disjoint_siblings() -> u64 := do
    let mut pair := new Pair { left := 1, right := 2 }
    let pairRef := &mut pair
    let left := &mut pairRef.left
    let right := &mut pairRef.right
    *left := 10
    *right := 20
    let leftValue := *left
    let rightValue := *right
    leftValue + rightValue

  fun child_then_parent() -> u64 := do
    let mut pair := new Pair { left := 1, right := 2 }
    let parent := &mut pair
    let child := &mut parent.left
    *child := 8
    let _childValue := *child
    let result := *parent
    result.left + result.right

--# run 0x0::LeanerBorrowAccepted::multiple_immutable

--# run 0x0::LeanerBorrowAccepted::disjoint_siblings

--# run 0x0::LeanerBorrowAccepted::child_then_parent
