-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Incorrect pure and effectful
contracts must fail at their source clauses, without exporting a proof. -/

leaner module 0x42::negative_verification where
  fun wrong_increment(value : u64) -> u64 := value + 1
  spec wrong_increment where
    ensures result == value

  fun wrong_action(value : u64) -> u64 := value
  spec wrong_action where
    ensures result == value + 1
    aborts_if false

  fun peek(r : &u64) -> u64 := *r
  spec peek where
    ensures result == r
    aborts_if false

  -- Freezing `x` into `peek` does not end its loan: the write is visible.
  fun frozen_write(x : &mut u64) -> Unit := do
    let before := peek(x)
    if before < 100 then *x := before + 1
  spec frozen_write where
    ensures x == old(x)
    aborts_if false

  fun identity(slot : &mut u64) -> &mut u64 := slot
  spec identity where
    ensures result == old(slot) && slot == final(result)
    aborts_if false

  -- A write through the returned reference reaches its lender.
  fun write_returned(slot : &mut u64) -> Unit := do
    let returned := identity(slot)
    *returned := 7
  spec write_returned where
    ensures slot == old(slot)
    aborts_if false

  -- The lender ends at the returned reference's final value, not one more.
  fun wrong_final(slot : &mut u64) -> &mut u64 := slot
  spec wrong_final where
    ensures slot == final(result) + 1
    aborts_if false

-- The members of a cycle of calls are verified together: one incorrect
-- contract fails them all, and every member is used through its contract.
leaner module 0x42::negative_cycles where
  fun even(n : u64) -> Bool := if n == 0 then true else odd(n - 1)
  spec even where
    ensures result == (n % 2 == 0)
    aborts_if false

  fun odd(n : u64) -> Bool := if n == 0 then true else even(n - 1)
  spec odd where
    ensures result == (n % 2 == 1)
    aborts_if false

  fun ping(n : u64) -> u64 := if n == 0 then 0 else pong(n - 1)
  spec ping where
    ensures result == 0
    aborts_if false

  fun pong(n : u64) -> u64 := ping(n)

-- An incorrect contract exports no verification theorem.
open Lean Elab Command in
run_cmd do
  let failed := [(`«0x42».negative_verification, ["wrong_increment", "wrong_action",
      "frozen_write", "write_returned", "wrong_final"]),
    (`«0x42».negative_cycles, ["even", "odd", "ping"])]
  for (module, functions) in failed do
    for function in functions do
      let name := (module.str function).str "verified"
      if (← getEnv).contains name then
        throwError "incorrect contract exported a verification theorem: {name}"
