-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowVectors where
  fun insert_without_element_borrow() -> u64 := do
    let mut values := vector<u64>[1, 3]
    let writer := &mut values
    *writer := core.prim.insertVector(*writer, 1, 2)
    let updated := *writer
    updated.length

  fun freeze_element() -> u64 := do
    let mut values := vector<u64>[2, 4]
    let writer := &mut values[1]
    *writer := 6
    let observation : &u64 := writer
    let result := *observation
    result

  fun pop_without_element_borrow() -> u64 := do
    let mut values := vector<u64>[2, 4]
    let writer := &mut values
    let (removed, rest) := core.prim.removeVector(*writer, 1)
    *writer := rest
    removed

  fun swap_without_element_borrow() -> u64 := do
    let mut values := vector<u64>[1, 2]
    let writer := &mut values
    *writer := core.prim.swapVector(*writer, 0, 1)
    let updated := *writer
    let first := &updated[0]
    let result := *first
    result

--# run 0x0::LeanerBorrowVectors::insert_without_element_borrow

--# run 0x0::LeanerBorrowVectors::freeze_element

--# run 0x0::LeanerBorrowVectors::pop_without_element_borrow

--# run 0x0::LeanerBorrowVectors::swap_without_element_borrow
