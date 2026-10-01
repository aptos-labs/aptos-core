-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerLoops where
  fun count_down(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      remaining := remaining - 1
    remaining

  fun count_down_loop(n : u64) -> u64 := do
    let mut remaining := n
    loop do
      if remaining < 1 then break
      remaining := remaining - 1
    remaining

  fun skip_evens(n : u64, acc : u64) -> u64 := do
    let mut remaining := n
    let mut total := acc
    while 0 < remaining do
      remaining := remaining - 1
      if remaining % 2 == 0 then continue
      total := total + 1
    total

  fun labeled_count_down(n : u64) -> u64 := do
    let mut remaining := n
    loop@outer do
      loop do
        if remaining < 1 then break@outer
        remaining := remaining - 1
        continue@outer
    remaining

  fun return_in_loop(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      if remaining == 3 then return 1
      remaining := remaining - 1
    remaining

--# run 0x0::LeanerLoops::count_down --args 5u64

--# run 0x0::LeanerLoops::count_down_loop --args 5u64

--# run 0x0::LeanerLoops::skip_evens --args 5u64 0u64

--# run 0x0::LeanerLoops::labeled_count_down --args 5u64

--# run 0x0::LeanerLoops::return_in_loop --args 5u64

--# run 0x0::LeanerLoops::return_in_loop --args 2u64
