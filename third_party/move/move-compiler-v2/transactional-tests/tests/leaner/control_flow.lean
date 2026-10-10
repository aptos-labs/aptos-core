-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerControlFlow where
  fun classify(value : u64) -> u64 :=
    if value < 10 then 1
    else if value <= 20 then 2
    else 3

  fun compare(left : u64, right : u64) -> u64 :=
    if left == right then 10
    else if left < right then 20
    else 30

  fun choose(flag : Bool) -> u64 := if flag then 4 else 5

  fun both(left : Bool, right : Bool) -> u64 := if left && right then 1 else 0

  fun either(left : Bool, right : Bool) -> u64 := if left || !right then 1 else 0

  fun countdown(value : u64, accumulator : u64) -> u64 := do
    let mut remaining := value
    let mut total := accumulator
    while remaining > 0 do
      remaining := remaining - 1
      total := total + 1
    total

--# run 0x0::LeanerControlFlow::classify --args 9u64

--# run 0x0::LeanerControlFlow::classify --args 10u64

--# run 0x0::LeanerControlFlow::classify --args 21u64

--# run 0x0::LeanerControlFlow::compare --args 7u64 7u64

--# run 0x0::LeanerControlFlow::compare --args 6u64 7u64

--# run 0x0::LeanerControlFlow::compare --args 8u64 7u64

--# run 0x0::LeanerControlFlow::choose --args true

--# run 0x0::LeanerControlFlow::choose --args false

--# run 0x0::LeanerControlFlow::both --args true false

--# run 0x0::LeanerControlFlow::either --args false false

--# run 0x0::LeanerControlFlow::countdown --args 5u64 40u64
