--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowLoop where

  /-- One activated handle remains the unique mutation lineage across every
  loop iteration.  Leaner's loop fixpoint accepts it. -/
  fun run : Action U64 := do
    let owner : U64 := 0
    let _writer ← &mut owner
    let mut count : U64 := 0
    while count < 3 do
      _writer := *_writer + 2
      count := count + 1
    let result ← *_writer
    pure result

  spec run where
    ensures True

--# run 0x0::LeanerBorrowLoop::run
