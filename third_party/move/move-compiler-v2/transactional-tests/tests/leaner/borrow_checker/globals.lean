--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowGlobals where

  struct Counter has Key where
    value : U64

  fun publish (account : &Signer) (value : U64) : Action Unit :=
    moveTo account ({ value } : Counter)

  fun read_twice (address : Address) : Action U64 := do
    let first ← &Counter[address].value
    let second ← &Counter[address].value
    let left ← *first
    let right ← *second
    pure (left + right)

  spec read_twice (_address : Address) where
    ensures True

  fun increment (address : Address) : Action Unit := do
    let writer ← &mut Counter[address].value
    writer := *writer + 1

  spec increment (_address : Address) where
    ensures True

  fun abort_after_write (address : Address) : Action Unit := do
    let writer ← &mut Counter[address].value
    writer := 99
    abort 7

  spec abort_after_write (_address : Address) where
    ensures True

  fun read (address : Address) : Action U64 := do
    let observation ← &Counter[address].value
    let result ← *observation
    pure result

  spec read (_address : Address) where
    ensures True

--# run --args 5u64 --signers 0x42 -- 0x0::LeanerBorrowGlobals::publish

--# run 0x0::LeanerBorrowGlobals::read_twice --args @0x42

--# run 0x0::LeanerBorrowGlobals::increment --args @0x42

--# run 0x0::LeanerBorrowGlobals::read --args @0x42

--# run 0x0::LeanerBorrowGlobals::abort_after_write --args @0x42

--# run 0x0::LeanerBorrowGlobals::read --args @0x42
