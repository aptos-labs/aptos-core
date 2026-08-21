--# publish

import Move

open scoped Move

move_module LeanerLoops where

  /-! ## Functions -/

  fun countDown (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      n := n - 1
    n

  fun countDownLoop (n : U64) : U64 := do
    let mut n := n
    loop
      if n < 1 then break
      n := n - 1
    n

  fun skipEvens (n acc : U64) : U64 := do
    let mut n := n
    let mut acc := acc
    while 0 < n do
      n := n - 1
      if n % 2 == 0 then continue
      acc := acc + 1
    acc

  fun labeledCountDown (n : U64) : U64 := do
    let mut n := n
    loop@outer
      loop
        if n < 1 then break@outer
        n := n - 1
        continue@outer
    n

  fun returnInLoop (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      if n == 3 then return 1
      n := n - 1
    n

/-! ## Tests -/

--# run 0x0::LeanerLoops::countDown --args 5u64

--# run 0x0::LeanerLoops::countDownLoop --args 5u64

--# run 0x0::LeanerLoops::skipEvens --args 5u64 0u64

--# run 0x0::LeanerLoops::labeledCountDown --args 5u64

--# run 0x0::LeanerLoops::returnInLoop --args 5u64

--# run 0x0::LeanerLoops::returnInLoop --args 2u64
