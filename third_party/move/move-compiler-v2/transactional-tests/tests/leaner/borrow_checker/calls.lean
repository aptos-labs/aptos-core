--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowCalls where

  struct Pair has Copy, Drop, Store where
    left : U64
    right : U64

  fun replace (_slot : &mut U64) (value : U64) : Action Unit := do
    _slot := value

  spec replace (_slot : &mut U64) (_value : U64) where
    ensures True

  fun write_and_read (_writer : &mut U64) (reader : &mut U64) : Action U64 := do
    _writer := 11
    let result ← *reader
    pure result

  spec write_and_read (_writer : &mut U64) (_reader : &mut U64) where
    ensures True

  fun write_capable_call : Action U64 := do
    let owner : U64 := 1
    let writer ← &mut owner
    replace writer 9
    let result ← *writer
    pure result

  spec write_capable_call where
    ensures True

  fun separated_call : Action U64 := do
    let pair : Pair := { left := 1, right := 7 }
    let pairRef ← &mut pair
    let writer ← &mut pairRef.left
    let reader ← &mut pairRef.right
    write_and_read writer reader

  spec separated_call where
    ensures True

--# run 0x0::LeanerBorrowCalls::write_capable_call

--# run 0x0::LeanerBorrowCalls::separated_call
