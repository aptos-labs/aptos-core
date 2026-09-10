-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Reject invariants that fail on entry, are not preserved, or do not establish the exit
contract. These proofs must not export a public verification theorem. -/

leaner module 0x42::negative_loop_invariants where
  fun bad_entry(n : u64) -> u64 := do
    let mut i : u64 := 0
    loop do
      if !(i < n) then break
      i := i + 1
    spec do
      invariant i < n
    return i
  spec bad_entry where
    ensures result == n
    aborts_if false
  verify bad_entry

  fun weak_exit(n : u64) -> u64 := do
    let mut i : u64 := 0
    while i < n do
      i := i + 1
    where
      invariant true
    return i
  spec weak_exit where
    ensures result == n
    aborts_if false
  verify weak_exit

  fun bad_step() -> u64 := do
    let mut i : u64 := 0
    loop do
      if i == 1 then break
      i := i + 1
    spec do
      invariant i == 0
    return i
  spec bad_step where
    ensures result == 0
    aborts_if false
  verify bad_step

  -- A default typed invariant must not assume the function's postcondition.
  fun bad_default(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      remaining := remaining - 1
    return remaining
  spec bad_default where
    ensures result == 1
    aborts_if false
  verify bad_default

open Lean Elab Command in
run_cmd do
  for function in ["bad_entry", "weak_exit", "bad_step", "bad_default"] do
    let name := ((`«0x42».negative_loop_invariants).str function).str "verified"
    if (← getEnv).contains name then
      throwError "incorrect loop invariant exported a theorem: {name}"
