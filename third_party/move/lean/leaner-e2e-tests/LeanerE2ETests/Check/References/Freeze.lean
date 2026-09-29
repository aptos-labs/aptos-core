-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Freezing a mutable reference

Passing `&mut` where `&` is expected freezes the loan into a shared reborrow:
the frozen reference observes the current value, and the mutable loan stays
live until its holder dies, so writes after the freeze reach the lender.
-/

namespace LeanerLang.Tests.Check.References.Freeze

leaner module 0x42::freeze where
  fun peek(r : &u64) -> u64 := *r
  spec peek where
    ensures result == r
    aborts_if false

  -- Writes through `x` after freezing it into `peek` are the caller's.
  fun bump(x : &mut u64) -> u64 := do
    let before := peek(x)
    if before < 100 then *x := before + 1
    before
  spec bump where
    ensures result == old(x)
    ensures old(x) < 100 ==> x == old(x) + 1
    aborts_if false

  fun write_after() -> u64 := do
    let mut v : u64 := 10
    let r := &mut v
    let a := peek(r)
    *r := a + 1
    v
  spec write_after where
    ensures result == 11
    aborts_if false

  fun freeze_last() -> u64 := do
    let mut v : u64 := 10
    let r := &mut v
    *r := 5
    let s := peek(r)
    v + s
  spec freeze_last where
    ensures result == 10
    aborts_if false

  fun run() -> u64 := do
    let mut v : u64 := 10
    let r := &mut v
    let b := bump(r)
    v + b

-- Run the functions on concrete inputs in the interpreter and compare the
-- outcomes: writes after a freeze reach the lender.
open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».freeze #[
    ⟨"run", #[], .returned #[.integer 21], {}⟩,
    ⟨"write_after", #[], .returned #[.integer 11], {}⟩,
    ⟨"freeze_last", #[], .returned #[.integer 10], {}⟩]

end LeanerLang.Tests.Check.References.Freeze
