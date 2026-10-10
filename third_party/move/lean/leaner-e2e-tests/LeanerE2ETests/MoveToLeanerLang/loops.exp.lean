-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x42::loops where
  fun count_down(n : u64) -> u64 := do
    let n := n
    while n > 0 do
      n := n - 1
    n

  spec count_down where
    ensures result == 0

  fun sum_to(n : u64) -> u64 := do
    let i := 0
    let total := 0
    while i < n do
      total := total + i
      i := i + 1
    total

  fun labeled_exit(n : u64) -> u64 := do
    let n := n
    loop@l0 loop do
      if n < 1 then break@l0
      n := n - 1
      break
    n

  fun first_even(v : &Vector<u64>) -> u64 := do
    let i := 0
    let len := v.length
    while i < len do
      let x := v[i]
      if x % 2 == 0 then return x;
      i := i + 1
    0

  fun build(n : u64) -> Vector<u64> := do
    let mut v := vector<u64>[]
    let i := 0
    while i < n do
      v := core.prim.pushVector(v, i)
      i := i + 1
    v

  fun for_sum(n : u64) -> u64 := do
    let total := 0
    for i in 0..n do
      total := total + i
    total
