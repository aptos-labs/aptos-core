--# publish

import Move

move_module LeanerTailRecursion where

  /-! ## Functions -/

  partial fun countdown (remaining accumulator : U64) : U64 :=
    if remaining < 1 then
      accumulator
    else
      continue countdown (remaining - 1) (accumulator + 1)

  partial fun alternate (remaining left right : U64) : U64 :=
    if remaining < 1 then
      left
    else
      continue alternate (remaining - 1) right left

  partial fun effectCountdown (remaining accumulator : U64) : Action U64 := do
    if remaining < 1 then
      pure accumulator
    else
      continue effectCountdown (remaining - 1) (accumulator + 1)

  partial fun mixedCountdown (remaining accumulator : U64) : U64 :=
    if remaining < 1 then
      accumulator
    else if remaining < 2 then
      mixedCountdown (remaining - 1) (accumulator + 1)
    else
      continue mixedCountdown (remaining - 1) (accumulator + 1)

  partial fun sumDown (value : U64) : U64 :=
    if value < 1 then 0 else value + sumDown (value - 1)

/-! ## Tests -/

--# run 0x0::LeanerTailRecursion::countdown --args 2000u64 40u64

--# run 0x0::LeanerTailRecursion::alternate --args 2001u64 10u64 20u64

--# run 0x0::LeanerTailRecursion::effectCountdown --args 2000u64 40u64

--# run 0x0::LeanerTailRecursion::mixedCountdown --args 2000u64 40u64

--# run 0x0::LeanerTailRecursion::sumDown --args 10u64
