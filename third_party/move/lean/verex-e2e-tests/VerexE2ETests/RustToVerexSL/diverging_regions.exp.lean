-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace diverging_regions where
  enum Color where
    | Red = 0
    | Green = 1
    | Blue = 2

  fun spin(_value : u64) -> Never := loop continue

  fun die() -> Never := panic()

  fun direct(c : u64) -> u64 :=
    if c == 0u64 then panic() else c

  fun explicit(c : u64) -> u64 :=
    if c == 0u64 then panic() else c

  -- The region computes before its call, including a check of its own.
  fun computes(c : u64, x : u64) -> u64 :=
    if c == 0u64 then panic() else c

  -- Both ways through the entry of the region reach a call.
  fun both_ways(c : Bool) -> u64 := panic()

  fun nested(a : Bool, b : Bool) -> u64 := do
    let mut x := 0u64
    if a then
      if b then panic() else x := 1u64
    x

  -- The impossible default of the switch stays impossible.
  fun arm(color : Color) -> u64 :=
    match core.readDiscriminant[isize](color) with
      | 0 => panic()
      | 1 => 1u64
      | 2 => 2u64

  -- Every arm diverges, so the switch goes with its impossible default.
  fun every_arm(color : Color) -> u64 := panic()

  -- No region: nothing but the impossible default fails.
  fun no_region(color : Color) -> u64 :=
    match core.readDiscriminant[isize](color) with
      | 0 => 0u64
      | 1 => 1u64
      | 2 => 2u64

  -- The loop has no way out but the call and is not in the region.
  fun loop_exit(limit : u64) -> u64 := do
    let mut count := 0u64
    loop if count > limit then break
    else
      count := core.prim.checkedAddPanic(count, 1u64)
      continue
    panic()

  fun before_a_loop(limit : u64) -> u64 := do
    let mut count := 0u64
    while count < limit do
      count := core.prim.checkedAddPanic(count, 1u64)
      continue
    panic()

verex namespace std where
  pragma verify = false

  mod process where
    native fun exit(_1 : i32) -> Never
