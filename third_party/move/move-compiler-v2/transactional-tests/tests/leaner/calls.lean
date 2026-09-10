--# publish

import Move

module LeanerCalls where

  /-! ## Functions -/

  fun twice (value : U64) : U64 :=
    value + value

  fun increment (value : U64) : Action U64 := do
    pure (value + 1)

  fun composed (value : U64) : Action U64 := do
    let doubled := twice value
    increment doubled

  fun bound_call (value : U64) : Action U64 := do
    let incremented ← increment value
    pure (twice incremented)

  partial fun sum_down (value : U64) : U64 :=
    if value < 1 then 0 else value + sum_down (value - 1)

  mutual
    partial fun even_flag (value : U64) : U64 :=
      if value < 1 then 1 else odd_flag (value - 1)

    partial fun odd_flag (value : U64) : U64 :=
      if value < 1 then 0 else even_flag (value - 1)
  end

/-! ## Tests -/

--# run 0x0::LeanerCalls::composed --args 7u64

--# run 0x0::LeanerCalls::bound_call --args 7u64

--# run 0x0::LeanerCalls::sum_down --args 5u64

--# run 0x0::LeanerCalls::even_flag --args 6u64

--# run 0x0::LeanerCalls::even_flag --args 7u64
