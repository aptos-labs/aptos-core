--# publish

import Move

open scoped Move

module LeanerLoops where

  /-! ## Functions -/

  fun count_down (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      n := n - 1
    n

  fun count_down_loop (n : U64) : U64 := do
    let mut n := n
    loop
      if n < 1 then break
      n := n - 1
    n

  fun skip_evens (n acc : U64) : U64 := do
    let mut n := n
    let mut acc := acc
    while 0 < n do
      n := n - 1
      if n % 2 == 0 then continue
      acc := acc + 1
    acc

  fun labeled_count_down (n : U64) : U64 := do
    let mut n := n
    loop@outer
      loop
        if n < 1 then break@outer
        n := n - 1
        continue@outer
    n

  fun return_in_loop (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      if n == 3 then return 1
      n := n - 1
    n

/-! ## Tests -/

--# run 0x0::LeanerLoops::count_down --args 5u64

--# run 0x0::LeanerLoops::count_down_loop --args 5u64

--# run 0x0::LeanerLoops::skip_evens --args 5u64 0u64

--# run 0x0::LeanerLoops::labeled_count_down --args 5u64

--# run 0x0::LeanerLoops::return_in_loop --args 5u64

--# run 0x0::LeanerLoops::return_in_loop --args 2u64
