-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Port of v0 Negative/Verification: incorrect pure and effectful
contracts must fail at their source clauses, without exporting a proof. -/

leaner module 0x42::negative_verification where
  fun wrong_increment(value : u64) -> u64 := value + 1
  spec wrong_increment where
    ensures result == value

  fun wrong_action(value : u64) -> u64 := value
  spec wrong_action where
    ensures result == value + 1
    aborts_if false

  verify wrong_increment
  verify wrong_action

open Lean Elab Command in
run_cmd do
  for function in ["wrong_increment", "wrong_action"] do
    let name := ((`«0x42».negative_verification).str function).str "verified"
    if (← getEnv).contains name then
      throwError "incorrect contract exported a verification theorem: {name}"
