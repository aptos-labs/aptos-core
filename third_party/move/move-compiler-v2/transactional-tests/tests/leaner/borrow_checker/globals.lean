-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowGlobals where
  struct Counter has Key where
    value : u64

  entry fun publish(account : &Signer, value : u64) -> Unit :=
    move_to<Counter>(account, new Counter { value := value })

  fun read_twice(addr : Address) -> u64 := do
    let first := &Counter[addr].value
    let second := &Counter[addr].value
    let left := *first
    let right := *second
    left + right

  fun increment(addr : Address) -> Unit := do
    let writer := &mut Counter[addr].value
    *writer := *writer + 1

  fun abort_after_write(addr : Address) -> Unit := do
    let writer := &mut Counter[addr].value
    *writer := 99
    abort(7)

  fun read(addr : Address) -> u64 := do
    let observation := &Counter[addr].value
    let result := *observation
    result

--# run --args 5u64 --signers 0x42 -- 0x0::LeanerBorrowGlobals::publish

--# run 0x0::LeanerBorrowGlobals::read_twice --args @0x42

--# run 0x0::LeanerBorrowGlobals::increment --args @0x42

--# run 0x0::LeanerBorrowGlobals::read --args @0x42

--# run 0x0::LeanerBorrowGlobals::abort_after_write --args @0x42

--# run 0x0::LeanerBorrowGlobals::read --args @0x42
