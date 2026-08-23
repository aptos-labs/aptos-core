--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectCall where

  fun write_and_observe (writer : &mut U64) (reader : &mut U64) : Action U64 := do
    writer := 4
    let result ← *reader
    pure result

  spec write_and_observe (writer : &mut U64) (reader : &mut U64) where
    ensures True

  fun run : Action U64 := do
    let owner : U64 := 0
    let writer ← &mut owner
    let reader ← &mut owner
    write_and_observe writer reader

  spec run where
    ensures True
