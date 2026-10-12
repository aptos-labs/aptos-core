-- Copyright © Aptos Foundation

--# publish

import VerexMove

verex module 0x0::VerexBorrowRejectReadOnlyCall where
  fun observe_two(left : &mut u64, right : &mut u64) -> u64 := do
    let leftValue := *left
    let rightValue := *right
    leftValue + rightValue

  fun run() -> u64 := do
    let mut owner : u64 := 9
    let first := &mut owner
    let second := &mut owner
    observe_two(first, second)
