-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# Place borrows no local holds

A mutable borrow of a place that dies where it is used, dereferenced, read
through a field, or discarded, writes nothing: it reads the place, which
keeps its value. Checked as the mutable borrow it is, it is read as a shared
borrow of its place (validation's `foldPlaceBorrows`). The interpreter runs
and the verifier proves the same results.
-/

namespace VerexSL.Tests.Check.References.PlaceReads

verex module 0x42::place_reads where
  struct Cell has Copy, Drop where
    value : u64

  fun wildcard_local(x : u64) -> u64 := do
    let mut y := x
    let _ := &mut y
    y
  spec wildcard_local where
    aborts_if false
    ensures result == x

  fun discarded_local(x : u64) -> u64 := do
    let mut y := x
    &mut y
    y
  spec discarded_local where
    aborts_if false
    ensures result == x

  fun read_through(c : Cell) -> u64 := do
    let mut d := c
    let v := (&mut d).value
    v + d.value
  spec read_through where
    aborts_if c.value + c.value > MAX_U64
    ensures result == c.value + c.value

  fun deref_through(x : u64) -> u64 := do
    let mut y := x
    let z := *(&mut y)
    z + y
  spec deref_through where
    aborts_if x + x > MAX_U64
    ensures result == x + x

  fun wildcard_field(c : &mut Cell) -> u64 := do
    let _ := &mut c.value
    c.value
  spec wildcard_field where
    aborts_if false
    ensures result == c.value && c == old(c)

  fun compared(x : u64, y : u64) -> Bool := do
    let mut a := x
    let mut b := y
    let same := &mut a == &mut b
    same && a == x
  spec compared where
    pragma verify = false

  fun deref_unchanged(x : u64) -> u64 := do
    let mut y := x
    let z := *(&mut y)
    z + y
  spec deref_unchanged where
    ensures result == x -- error: both reads see the place's value

  fun run_all() -> u64 := do
    let mut c := new Cell { value := 3 }
    wildcard_local(1) + discarded_local(2) * 10 + read_through(c) * 100 +
      deref_through(4) * 1000 + wildcard_field(&mut c) * 10000
  spec run_all where
    pragma verify = false

  fun run_compared() -> Bool := compared(5, 5)
  spec run_compared where
    pragma verify = false

-- The lenders keep their values: no loan was left with a hole.
open VerexIR VerexE2ETests.CheckSupport in
run_cmd assertRuns `«0x42».place_reads #[
  ⟨"run_all", #[], .returned #[.integer 38621], {}⟩,
  ⟨"run_compared", #[], .returned #[.bool true], {}⟩]

end VerexSL.Tests.Check.References.PlaceReads
