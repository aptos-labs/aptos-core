-- Copyright © Aptos Foundation

--# publish

import LeanerMove

-- Tail recursions written as loops over their parameter rows. The large
-- inputs check that none of them consumes call stack per iteration.
leaner module 0x0::LeanerTailRecursion where
  fun countdown(remaining : u64, accumulator : u64) -> u64 := do
    let mut remaining := remaining
    let mut total := accumulator
    loop do
      if remaining < 1 then break
      remaining := remaining - 1
      total := total + 1
    total

  -- Both loop parameters are updated in parallel: the pair is swapped.
  fun alternate(remaining : u64, left : u64, right : u64) -> u64 := do
    let mut remaining := remaining
    let mut left := left
    let mut right := right
    while 0 < remaining do
      let previous := left
      left := right
      right := previous
      remaining := remaining - 1
    left

  fun effect_countdown(remaining : u64, accumulator : u64) -> u64 := do
    let mut remaining := remaining
    let mut total := accumulator
    loop do
      if remaining < 1 then return total
      remaining := remaining - 1
      total := total + 1

  -- The last step is an ordinary recursive call from inside the loop.
  fun mixed_countdown(remaining : u64, accumulator : u64) -> u64 := do
    let mut remaining := remaining
    let mut total := accumulator
    loop do
      if remaining < 1 then break
      if remaining < 2 then return mixed_countdown(remaining - 1, total + 1)
      remaining := remaining - 1
      total := total + 1
    total

  fun sum_down(value : u64) -> u64 :=
    if value < 1 then 0 else value + sum_down(value - 1)

--# run 0x0::LeanerTailRecursion::countdown --args 2000u64 40u64

--# run 0x0::LeanerTailRecursion::alternate --args 2001u64 10u64 20u64

--# run 0x0::LeanerTailRecursion::effect_countdown --args 2000u64 40u64

--# run 0x0::LeanerTailRecursion::mixed_countdown --args 2000u64 40u64

--# run 0x0::LeanerTailRecursion::sum_down --args 10u64
