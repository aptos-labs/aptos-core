-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerArithmetic where
  fun calculate(left : u64, right : u64) -> u64 :=
    ((left + right) * 3 - right) / 2 % 100

  fun add_overflow(value : u64) -> u64 := value + 1

  fun subtract_underflow(value : u64) -> u64 := value - 1

  fun divide_by_zero(value : u64) -> u64 := value / 0

--# run 0x0::LeanerArithmetic::calculate --args 8u64 2u64

--# run 0x0::LeanerArithmetic::calculate --args 81u64 21u64

--# run 0x0::LeanerArithmetic::add_overflow --args 18446744073709551615u64

--# run 0x0::LeanerArithmetic::subtract_underflow --args 0u64

--# run 0x0::LeanerArithmetic::divide_by_zero --args 9u64
