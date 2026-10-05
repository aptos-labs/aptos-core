-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRecursion where
  fun drain(slot : &mut u64) -> Unit := do
    let current := *slot
    if current == 0 then return ()
    *slot := current - 1
    drain(slot)

  fun ping(slot : &mut u64, remaining : u64) -> Unit :=
    if remaining == 0 then *slot := 21 else pong(slot, remaining - 1)

  fun pong(slot : &mut u64, remaining : u64) -> Unit :=
    if remaining == 0 then *slot := 22 else ping(slot, remaining - 1)

  fun run() -> u64 := do
    let mut owner : u64 := 4
    let writer := &mut owner
    drain(writer)
    let result := *writer
    result

  fun run_mutual() -> u64 := do
    let mut owner : u64 := 0
    let writer := &mut owner
    ping(writer, 3)
    let result := *writer
    result

--# run 0x0::LeanerBorrowRecursion::run

--# run 0x0::LeanerBorrowRecursion::run_mutual
