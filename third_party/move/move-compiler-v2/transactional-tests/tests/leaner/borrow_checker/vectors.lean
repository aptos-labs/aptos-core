--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowVectors where

  fun insert_without_element_borrow : Action U64 := do
    let values : Vector U64 := vector![1, 3]
    let writer ← &mut values
    Move.Vector.insert writer 1 2
    let updated ← *writer
    pure updated.length

  spec insert_without_element_borrow where
    ensures True

  fun freeze_element : Action U64 := do
    let values : Vector U64 := vector![2, 4]
    let writer ← &mut values[1]
    writer := 6
    let observation ← freeze writer
    let result ← *observation
    pure result

  spec freeze_element where
    ensures True

  fun pop_without_element_borrow : Action U64 := do
    let values : Vector U64 := vector![2, 4]
    let writer ← &mut values
    writer.popBack

  spec pop_without_element_borrow where
    ensures True

  fun swap_without_element_borrow : Action U64 := do
    let values : Vector U64 := vector![1, 2]
    let writer ← &mut values
    writer.swap 0 1
    let updated ← *writer
    let first ← &updated[0]
    let result ← *first
    pure result

  spec swap_without_element_borrow where
    ensures True

--# run 0x0::LeanerBorrowVectors::insert_without_element_borrow

--# run 0x0::LeanerBorrowVectors::freeze_element

--# run 0x0::LeanerBorrowVectors::pop_without_element_borrow

--# run 0x0::LeanerBorrowVectors::swap_without_element_borrow
