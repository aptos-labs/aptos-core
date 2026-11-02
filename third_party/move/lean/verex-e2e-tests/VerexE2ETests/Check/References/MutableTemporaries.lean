-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# Mutable references no local holds

A mutable reference a call returns passes its current value back to its
lender when it dies. Discarded, matched by a wildcard, read through, or
borrowed through, it is held by a local of its own until then (validation's
`holdMutableReferences`), so the lender sees what the callee wrote. The interpreter runs and the verifier proves the same results.
-/

namespace VerexSL.Tests.Check.References.MutableTemporaries

verex module 0x42::mutable_temporaries where
  struct Cell has Copy, Drop where
    value : u64

  fun increment(c : &mut Cell) -> &mut Cell := do
    c.value := c.value + 1
    c
  spec increment where
    aborts_if c.value == MAX_U64
    ensures c.value == old(c).value + 1

  fun bump_pair(a : &mut u64, b : &mut u64) -> (&mut u64, &mut u64) := do
    *a := *a + 1
    *b := *b + 1
    (a, b)
  spec bump_pair where
    aborts_if a == MAX_U64 || b == MAX_U64
    ensures a == old(a) + 1 && b == old(b) + 1

  -- ## Discarded results

  fun discarded(c : &mut Cell) -> Unit := do
    increment(c)
  spec discarded where
    aborts_if c.value == MAX_U64
    ensures c.value == old(c).value + 1

  fun discarded_twice(c : &mut Cell) -> Unit := do
    increment(c)
    increment(c)
  spec discarded_twice where
    aborts_if c.value + 2 > MAX_U64
    ensures c.value == old(c).value + 2

  fun discarded_unchanged(c : &mut Cell) -> Unit := do
    increment(c)
  spec discarded_unchanged where
    ensures c.value == old(c).value -- error: the discarded result writes back

  fun wildcard(c : &mut Cell) -> Unit := do
    let _ := increment(c)
  spec wildcard where
    aborts_if c.value == MAX_U64
    ensures c.value == old(c).value + 1

  fun discarded_pair(a : &mut u64, b : &mut u64) -> Unit := do
    bump_pair(a, b)
  spec discarded_pair where
    aborts_if a == MAX_U64 || b == MAX_U64
    ensures a == old(a) + 1 && b == old(b) + 1

  fun pair_wildcard(a : &mut u64, b : &mut u64) -> Unit := do
    let (_, second) := bump_pair(a, b)
    *second := *second + 1
  spec pair_wildcard where
    aborts_if a == MAX_U64 || b + 1 >= MAX_U64
    ensures a == old(a) + 1 && b == old(b) + 2

  -- ## Read and borrowed through

  fun read_field(c : &mut Cell) -> u64 := do
    increment(c).value
  spec read_field where
    aborts_if c.value == MAX_U64
    ensures c.value == old(c).value + 1 && result == c.value

  fun read_deref(c : &mut Cell) -> Cell := do
    *increment(c)
  spec read_deref where
    aborts_if c.value == MAX_U64
    ensures c.value == old(c).value + 1 && result == c

  fun borrow_field(c : &mut Cell) -> Unit := do
    let r := &mut increment(c).value
    *r := *r + 10
  spec borrow_field where
    aborts_if c.value + 11 > MAX_U64
    ensures c.value == old(c).value + 11

  -- ## Runs

  fun run_discarded() -> u64 := do
    let mut c := new Cell { value := 3 }
    discarded_twice(&mut c)
    c.value

  fun run_read() -> u64 := do
    let mut c := new Cell { value := 3 }
    let x := read_field(&mut c)
    let y := read_deref(&mut c)
    c.value * 100 + x * 10 + y.value

  fun run_borrowed() -> u64 := do
    let mut c := new Cell { value := 3 }
    borrow_field(&mut c)
    c.value

  fun run_pair() -> u64 := do
    let mut a := 1
    let mut b := 2
    discarded_pair(&mut a, &mut b)
    pair_wildcard(&mut a, &mut b)
    a * 10 + b
  spec run_pair where
    pragma verify = false

  -- A mutable borrow of a computed value borrows a temporary holding it.
  fun bump_temporary(x : u64) -> u64 := do
    let r := &mut (x + 1)
    *r := *r + 1
    *r
  spec bump_temporary where
    aborts_if x + 2 > MAX_U64
    ensures result == x + 2

-- The interpreter writes the results back as the verifier proves them.
open VerexIR VerexE2ETests.CheckSupport in
run_cmd assertRuns `«0x42».mutable_temporaries #[
  ⟨"run_discarded", #[], .returned #[.integer 5], {}⟩,
  ⟨"run_read", #[], .returned #[.integer 545], {}⟩,
  ⟨"run_borrowed", #[], .returned #[.integer 14], {}⟩,
  ⟨"run_pair", #[], .returned #[.integer 35], {}⟩,
  ⟨"bump_temporary", #[.integer 4], .returned #[.integer 6], {}⟩]

end VerexSL.Tests.Check.References.MutableTemporaries
