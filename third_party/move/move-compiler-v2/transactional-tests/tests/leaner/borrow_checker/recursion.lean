--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRecursion where

  partial fun drain (_slot : &mut U64) : Action Unit := do
    let current ← *_slot
    if current == 0 then return ()
    _slot := current - 1
    continue drain _slot

  spec drain (_slot : &mut U64) where
    ensures True

  mutual
    partial fun ping (_slot : &mut U64) (remaining : U64) : Action Unit := do
      if remaining == 0 then
        _slot := 21
      else
        pong _slot (remaining - 1)

    partial fun pong (_slot : &mut U64) (remaining : U64) : Action Unit := do
      if remaining == 0 then
        _slot := 22
      else
        ping _slot (remaining - 1)
  end

  spec ping (_slot : &mut U64) (_remaining : U64) where
    ensures True

  spec pong (_slot : &mut U64) (_remaining : U64) where
    ensures True

  fun run : Action U64 := do
    let owner : U64 := 4
    let writer ← &mut owner
    drain writer
    let result ← *writer
    pure result

  spec run where
    ensures True

  fun run_mutual : Action U64 := do
    let owner : U64 := 0
    let writer ← &mut owner
    ping writer 3
    let result ← *writer
    pure result

  spec run_mutual where
    ensures True

--# run 0x0::LeanerBorrowRecursion::run

--# run 0x0::LeanerBorrowRecursion::run_mutual
