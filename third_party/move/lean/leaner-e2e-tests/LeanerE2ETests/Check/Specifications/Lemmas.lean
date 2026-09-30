-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Lemmas and the steps that apply them. A lemma's `requires` imply its
`ensures`, established by the steps of its proof; a recursive one applies
itself at a smaller measure (`decreases`, or its integer parameters in
order). An `apply` step owes the lemma's premise and gives its conclusion;
under a quantifier it gives the lemma's implication. A `split` step
continues once per case. A lemma's integer parameters are bounded by their
types, which an application owes; a parameter of a vector type ranges
over vectors of its element type, whose elements are bounded so. -/

leaner module 0x42::lemmas where
  spec fun sum(n : Int) : Int decreases n := if n <= 0 then 0 else n + sum(n - 1)

  spec lemma add_zero_right(x : u64) where
    ensures x + 0 == x

  spec lemma small_product(x : u64, y : u64) where
    requires x <= 1000
    requires y <= 1000
    ensures x * y <= 1000000

  spec lemma monotonicity(x : Int, y : Int) where
    requires 0 <= x
    requires x <= y
    ensures sum(x) <= sum(y)
    proof
      assert x < y ==> sum(y - 1) <= sum(y)
      apply x < y ==> monotonicity(x, y - 1)

  spec lemma sum_bound(n : Int) decreases n where
    requires 0 <= n
    ensures sum(n) >= n
    proof
      split n > 0
      apply n > 0 ==> sum_bound(n - 1)

  spec fun prefix_sum(v : Vector<u64>, n : Int) : Int :=
    if n == 0 then 0 else prefix_sum(v, n - 1) + v[n - 1]

  spec lemma prefix_step(v : Vector<u64>, i : u64, n : u64) where
    requires i < n && v.length >= n
    ensures prefix_sum(v, i) + v[i] <= prefix_sum(v, n)
    proof
      split i + 1 < n
      apply i + 1 < n ==> prefix_step(v, i, n - 1)

  fun step_bound(v : Vector<u64>, i : u64) -> u64 := do
    spec apply prefix_step(v, i, v.length)
    0
  spec step_bound where
    requires i < v.length
    ensures prefix_sum(v, i) + v[i] <= prefix_sum(v, v.length)

  fun identity_via_add(x : u64) -> u64 := do
    spec apply add_zero_right(x)
    x + 0
  spec identity_via_add where
    ensures result == x

  fun product(x : u64, y : u64) -> u64 := do
    spec apply small_product(x, y)
    x * y
  spec product where
    requires x <= 1000
    requires y <= 1000
    ensures result <= 1000000

  fun sum_up_to(n : u64) -> u64 := do
    spec apply ∀ (x : Int; y : Int), monotonicity(x, y)
    if n == 0 then 0 else n + sum_up_to(n - 1)
  spec sum_up_to where
    aborts_if sum(n) > MAX_U64
    ensures result == sum(n)

  fun double(x : u64) -> u64 := do
    spec assert x + x == 2 * x
    let doubled := x + x
    spec assert doubled == x + x
    doubled
  spec double where
    requires x + x <= MAX_U64
    ensures result == 2 * x

  fun abs_diff(a : u64, b : u64) -> u64 := do
    spec split a >= b
    if a >= b then a - b else b - a
  spec abs_diff where
    ensures result == (if a >= b then a - b else b - a)

  -- ## Steps that do not hold

  spec lemma bad_claim(x : u64) where
    ensures x + 1 == x -- error: false for every `x`

  spec lemma upper_bound(n : Int) where
    requires 0 <= n
    ensures sum(n) <= n -- error: false from 3 on
    proof
      apply n > 0 ==> upper_bound(n - 1)

  spec lemma needs_positive(x : u64) where
    requires x > 0
    ensures x >= 1

  fun might_be_zero(x : u64) -> u64 := do
    spec apply needs_positive(x) -- error: `x` may be 0
    x
  spec might_be_zero where
    ensures result == x

  fun out_of_range(x : u128) -> Bool := do
    spec apply x >= 0 ==> add_zero_right(x) -- error: `x` may exceed `MAX_U64`
    true
  spec out_of_range where
    ensures result

  fun step_bound_unapplied(v : Vector<u64>, i : u64) -> u64 := 0
  spec step_bound_unapplied where
    requires i < v.length
    ensures prefix_sum(v, i) + v[i] <= prefix_sum(v, v.length) -- error: needs the induction `prefix_step` proves

  -- A lemma not established gives no fact: a function that needs none
  -- verifies, one that needs its conclusion does not.
  fun applies_unestablished(x : u64) -> u64 := do
    spec apply bad_claim(x)
    x
  spec applies_unestablished where
    ensures result == x

  fun needs_unestablished(x : u64) -> u64 := do
    spec apply bad_claim(x)
    x + 1
  spec needs_unestablished where
    requires x < MAX_U64
    ensures result == x -- error: `bad_claim` is not established
