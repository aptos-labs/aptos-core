--# publish

import Move

move_module LeanerCalls where

  /-! ## Functions -/

  fun twice (value : U64) : U64 :=
    value + value

  fun increment (value : U64) : Action U64 := do
    pure (value + 1)

  fun composed (value : U64) : Action U64 := do
    let doubled := twice value
    increment doubled

  fun boundCall (value : U64) : Action U64 := do
    let incremented ← increment value
    pure (twice incremented)

  partial fun sumDown (value : U64) : U64 :=
    if value < 1 then 0 else value + sumDown (value - 1)

  mutual
    partial fun evenFlag (value : U64) : U64 :=
      if value < 1 then 1 else oddFlag (value - 1)

    partial fun oddFlag (value : U64) : U64 :=
      if value < 1 then 0 else evenFlag (value - 1)
  end

/-! ## Tests -/

--# run 0x0::LeanerCalls::composed --args 7u64

--# run 0x0::LeanerCalls::boundCall --args 7u64

--# run 0x0::LeanerCalls::sumDown --args 5u64

--# run 0x0::LeanerCalls::evenFlag --args 6u64

--# run 0x0::LeanerCalls::evenFlag --args 7u64
