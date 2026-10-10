-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectCall where
  fun write_and_observe(writer : &mut u64, reader : &mut u64) -> u64 := do
    *writer := 4
    let result := *reader
    result

  fun run() -> u64 := do
    let mut owner : u64 := 0
    let writer := &mut owner
    let reader := &mut owner
    write_and_observe(writer, reader)
