--# publish

import Move

module LeanerControlFlow where

  /-! ## Functions -/

  fun classify (value : U64) : U64 :=
    if value < 10 then
      1
    else if UInt.lessEq value 20 then
      2
    else
      3

  fun compare (left right : U64) : U64 :=
    if UInt.equal left right then
      10
    else if left < right then
      20
    else
      30

  fun choose (flag : Bool) : U64 :=
    if flag then 4 else 5

  partial fun countdown (value accumulator : U64) : U64 :=
    if value < 1 then
      accumulator
    else
      continue countdown (value - 1) (accumulator + 1)

/-! ## Tests -/

--# run 0x0::LeanerControlFlow::classify --args 9u64

--# run 0x0::LeanerControlFlow::classify --args 10u64

--# run 0x0::LeanerControlFlow::classify --args 21u64

--# run 0x0::LeanerControlFlow::compare --args 7u64 7u64

--# run 0x0::LeanerControlFlow::compare --args 6u64 7u64

--# run 0x0::LeanerControlFlow::compare --args 8u64 7u64

--# run 0x0::LeanerControlFlow::choose --args true

--# run 0x0::LeanerControlFlow::choose --args false

--# run 0x0::LeanerControlFlow::countdown --args 5u64 40u64
