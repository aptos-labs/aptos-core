-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowCalls where
  struct Pair has Copy, Drop, Store where
    left : u64
    right : u64

  fun replace(slot : &mut u64, value : u64) -> Unit := *slot := value

  fun write_and_read(writer : &mut u64, reader : &mut u64) -> u64 := do
    *writer := 11
    let result := *reader
    result

  fun write_capable_call() -> u64 := do
    let mut owner : u64 := 1
    let writer := &mut owner
    replace(writer, 9)
    let result := *writer
    result

  fun separated_call() -> u64 := do
    let mut pair := new Pair { left := 1, right := 7 }
    let pairRef := &mut pair
    let writer := &mut pairRef.left
    let reader := &mut pairRef.right
    write_and_read(writer, reader)

--# run 0x0::LeanerBorrowCalls::write_capable_call

--# run 0x0::LeanerBorrowCalls::separated_call
