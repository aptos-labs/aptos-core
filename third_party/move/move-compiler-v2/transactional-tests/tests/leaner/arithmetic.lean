--# publish

import Move

move_module LeanerArithmetic where

  /-! ## Functions -/

  fun calculate (left right : U64) : U64 :=
    ((left + right) * 3 - right) / 2 % 100

  fun addOverflow (value : U64) : U64 :=
    value + 1

  fun subtractUnderflow (value : U64) : U64 :=
    value - 1

  fun divideByZero (value : U64) : U64 :=
    value / 0

/-! ## Tests -/

--# run 0x0::LeanerArithmetic::calculate --args 8u64 2u64

--# run 0x0::LeanerArithmetic::calculate --args 81u64 21u64

--# run 0x0::LeanerArithmetic::addOverflow --args 18446744073709551615u64

--# run 0x0::LeanerArithmetic::subtractUnderflow --args 0u64

--# run 0x0::LeanerArithmetic::divideByZero --args 9u64
