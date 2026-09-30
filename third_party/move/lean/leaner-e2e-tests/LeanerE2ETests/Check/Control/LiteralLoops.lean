-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
import LeanerE2ETests.CheckSupport

/-! Loops over literal vectors whose invariants state recursive
specification functions, and a nested loop with a nonlinear invariant. A
program's read of a literal vector and a specification function's read of
its encoding are the same term; a step's invariant follows from one
unfolding of the function at the computed argument, and the exit's value
from unfolding it down to its base case. A product of bounded integers is
bounded by the products of its factors' bounds. -/

namespace LeanerLang.Tests.Check.Control.LiteralLoops

leaner module 0x42::literal_loops where
  spec fun maximum(v : Vector<u64>, init : Int, end : Int) : Int :=
    if end == 0 then init
    else
      if v[end - 1] > maximum(v, init, end - 1) then v[end - 1]
      else maximum(v, init, end - 1)

  spec fun total(v : Vector<u64>, end : Int) : Int :=
    if end == 0 then 0 else total(v, end - 1) + v[end - 1]

  fun max_three() -> u64 := do
    let v := vector<u64>[3, 7, 2]
    let mut acc : u64 := 0
    let mut i : u64 := 0
    let n := v.length
    while i < n do
      if v[i] > acc then acc := v[i]
      i := i + 1
    where
      invariant i <= n
      invariant n == v.length
      invariant acc == maximum(v, 0, i)
    acc
  spec max_three where
    ensures result == 7

  fun max_three_incorrect() -> u64 := do
    let v := vector<u64>[3, 7, 2]
    let mut acc : u64 := 0
    let mut i : u64 := 0
    let n := v.length
    while i < n do
      if v[i] > acc then acc := v[i]
      i := i + 1
    where
      invariant i <= n
      invariant n == v.length
      invariant acc == maximum(v, 0, i)
    acc
  spec max_three_incorrect where
    ensures result == 8 -- error: the maximum is 7

  fun sum_three() -> u64 := do
    let v := vector<u64>[1, 2, 3]
    let mut acc : u64 := 0
    let mut i : u64 := 0
    let n := v.length
    while i < n do
      acc := acc + v[i]
      i := i + 1
    where
      invariant i <= n
      invariant n == v.length
      invariant acc == total(v, i)
    acc
  spec sum_three where
    aborts_if false
    ensures result == 6

  fun sum_grid(n : u64, m : u64) -> u64 := do
    let mut sum : u64 := 0
    let mut i : u64 := 0
    while i < n do
      let mut j : u64 := 0
      while j < m do
        sum := sum + 1
        j := j + 1
      where
        invariant j <= m
        invariant sum == i * m + j
      i := i + 1
    where
      invariant i <= n
      invariant sum == i * m
    sum
  spec sum_grid where
    requires n < 1000 && m < 1000
    aborts_if false
    ensures result == n * m

end LeanerLang.Tests.Check.Control.LiteralLoops
