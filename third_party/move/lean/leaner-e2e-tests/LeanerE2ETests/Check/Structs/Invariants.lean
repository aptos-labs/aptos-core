-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Data invariants

Data invariants of structs, enums, and resources.
-/

namespace LeanerLang.Tests.Check.Structs.Invariants

leaner module 0x42::invariants where
  struct Percent has Copy, Drop, Store where
    value : u64

  spec Percent where
    invariant value <= 100

  struct Bounds has Copy, Drop, Store where
    low : u64
    high : u64

  spec Bounds where
    invariant low <= high
    invariant high <= 1000

  enum Payment has Copy, Drop, Store where
    | None
    | Direct (amount : u64)
    | Split (left : u64, right : u64)

  spec Payment where
    invariant match this with
      | Payment::None {} => true
      | Payment::Direct { amount := amount } => amount > 0
      | Payment::Split { left := left, right := right } =>
          left > 0 && right > 0

  struct Gauge has Key where
    level : u64

  spec Gauge where
    invariant level <= 100

  fun empty() -> Percent := new Percent { value := 0 }

  fun half() -> Percent := new Percent { value := 50 }

  fun unit_bounds() -> Bounds := new Bounds { low := 0, high := 0 }

  fun span(bounds : Bounds) -> u64 := bounds.high - bounds.low
  spec span where
    ensures result <= bounds.high && result <= 1000
    aborts_if false

  fun direct(amount : u64) -> Payment :=
    if 0 < amount then
      new Payment::Direct { amount }
    else
      new Payment::None {}

  fun first_part(payment : Payment) -> u64 :=
    match payment with
      | Payment::None {} => 0
      | Payment::Direct { amount := amount } => amount
      | Payment::Split { left := left, right := right } => left
  spec first_part where
    ensures match payment with
      | Payment::None {} => result == 0
      | Payment::Direct { amount := _ } => result > 0
      | Payment::Split { left := _, right := _ } => result > 0
    aborts_if false

  public entry fun set_level(addr : Address, amount : u64) -> Unit := do
    let level := &mut Gauge[addr].level
    if amount < 101 then
      *level := amount
    else
      abort(1)
  spec set_level where
    requires exists<Gauge>(addr)
    modifies global<Gauge>(addr)
    ensures global<Gauge>(addr).level == amount
    aborts_if !(amount < 101) with 1

  fun reading(percent : Percent) -> u64 := percent.value
  spec reading where
    ensures result <= 100
    aborts_if false

  fun clamp(amount : u64) -> Percent := do
    if amount > 100 then abort(1)
    new Percent { value := amount }
  spec clamp where
    ensures result.value == amount
    aborts_if 100 < amount with 1

-- Run the functions in the interpreter: creating a value
-- checks nothing at run time, and `set_level` writes the gauge or aborts,
-- leaving storage unchanged.
open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let percent (value : Int) : RuntimeValue := .nominal ⟨⟨0⟩, 0⟩ none #[.integer value]
  let payment (variant : String) (fields : Array RuntimeValue) : RuntimeValue :=
    .nominal ⟨⟨0⟩, 2⟩ (some variant) fields
  assertRuns `«0x42».invariants #[
    ⟨"reading", #[percent 50], .returned #[.integer 50], {}⟩,
    ⟨"clamp", #[.integer 50], .returned #[percent 50], {}⟩,
    ⟨"clamp", #[.integer 500], .threw .abort #[.integer 1], {}⟩,
    ⟨"first_part", #[payment "Direct" #[.integer 7]], .returned #[.integer 7], {}⟩,
    ⟨"direct", #[.integer 0], .returned #[payment "None" #[]], {}⟩]
  let gauge (level : Int) (nextLoan : Nat) :=
    singleResourceState `«0x42».invariants "Gauge" "0x4" #[.integer level] nextLoan
  assertRunsState `«0x42».invariants #[
    ⟨"set_level", #[.address "0x4", .integer 42], .returned #[], ← gauge 10 0, ← gauge 42 2⟩,
    ⟨"set_level", #[.address "0x4", .integer 500], .threw .abort #[.integer 1], ← gauge 10 0,
      ← gauge 10 0⟩]

end LeanerLang.Tests.Check.Structs.Invariants
