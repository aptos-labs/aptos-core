-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerCalls where
  fun twice(value : u64) -> u64 := value + value

  fun increment(value : u64) -> u64 := value + 1

  fun composed(value : u64) -> u64 := do
    let doubled := twice(value)
    increment(doubled)

  fun bound_call(value : u64) -> u64 := twice(increment(value))

  fun sum_down(value : u64) -> u64 :=
    if value < 1 then 0 else value + sum_down(value - 1)

  fun even_flag(value : u64) -> u64 :=
    if value < 1 then 1 else odd_flag(value - 1)

  fun odd_flag(value : u64) -> u64 :=
    if value < 1 then 0 else even_flag(value - 1)

--# run 0x0::LeanerCalls::composed --args 7u64

--# run 0x0::LeanerCalls::bound_call --args 7u64

--# run 0x0::LeanerCalls::sum_down --args 5u64

--# run 0x0::LeanerCalls::even_flag --args 6u64

--# run 0x0::LeanerCalls::even_flag --args 7u64
