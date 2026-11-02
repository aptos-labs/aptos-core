-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexBorrowReturns where
  struct Pair has Copy, Drop, Store where
    left : u64
    right : u64

  fun identity(input : &u64) -> &u64 := input

  fun borrow_left(input : &Pair) -> &u64 := &input.left

  fun direct() -> u64 := do
    let owner : u64 := 13
    let observation := &owner
    let returned := identity(observation)
    let result := *returned
    result

  fun nested() -> u64 := do
    let owner := new Pair { left := 17, right := 19 }
    let observation := &owner
    let returned := borrow_left(observation)
    let result := *returned
    result

--# run 0x0::VerexBorrowReturns::direct

--# run 0x0::VerexBorrowReturns::nested
