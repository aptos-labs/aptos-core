-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

-- Incoming references keep their provenance through aliases, tuple patterns,
-- branch results, calls, and the end of an inner holder's scope.
verex module 0x42::caller_provenance where
  struct Pair has Copy, Drop where
    first : u64
    second : u64

  fun select(p : &Pair, choice : Bool) -> &u64 := do
    let result := do
      let (alias, branch) := (p, choice)
      if branch then &alias.first else &alias.second
    &*result

  fun identity(p : &u64) -> &u64 := p

  fun through_call(p : &Pair) -> &u64 := do
    let selected := identity(select(p, true))
    &*selected

  fun mutable_identity(p : &mut u64) -> &mut u64 := p

  fun update(p : &mut u64) -> Unit := do
    let alias := &mut *p
    let selected := mutable_identity(alias)
    *selected := 9

  -- A mutable reference copied by name into a local, or out of a branch, is
  -- a reborrow of it: no two locals hold one loan, so the write through the
  -- copy reaches the caller.
  fun update_copy(p : &mut u64) -> Unit := do
    let alias := p
    let selected := mutable_identity(alias)
    *selected := 9

  fun update_branch(p : &mut u64, q : &mut u64, first : Bool) -> Unit := do
    let selected := if first then p else q
    let chosen := mutable_identity(selected)
    *chosen := 9

  fun read_selected(choice : Bool) -> u64 := do
    let pair := new Pair { first := 11, second := 22 }
    *select(&pair, choice)

  fun read_call() -> u64 := do
    let pair := new Pair { first := 11, second := 22 }
    *through_call(&pair)

  fun read_updated() -> u64 := do
    let mut value : u64 := 7
    update(&mut value)
    value

  fun read_copied() -> u64 := do
    let mut value : u64 := 7
    update_copy(&mut value)
    value

  fun read_branch(first : Bool) -> u64 := do
    let mut left : u64 := 7
    let mut right : u64 := 7
    update_branch(&mut left, &mut right, first)
    left * 10 + right

open VerexIR VerexE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».caller_provenance #[
    ⟨"read_selected", #[.bool true], .returned #[.integer 11], {}⟩,
    ⟨"read_selected", #[.bool false], .returned #[.integer 22], {}⟩,
    ⟨"read_call", #[], .returned #[.integer 11], {}⟩,
    ⟨"read_updated", #[], .returned #[.integer 9], {}⟩,
    ⟨"read_copied", #[], .returned #[.integer 9], {}⟩,
    ⟨"read_branch", #[.bool true], .returned #[.integer 97], {}⟩,
    ⟨"read_branch", #[.bool false], .returned #[.integer 79], {}⟩]

-- A caller origin on one path does not justify a frame origin on another.
-- Reassigning a reference parameter also replaces its original provenance.
verex module 0x42::mixed_provenance where
  fun branch(p : &u64, choose : Bool) -> &u64 := do
    let owner : u64 := 7
    let selected := if choose then p else &owner
    &*selected

  fun replace(mut p : &u64) -> &u64 := do
    let owner : u64 := 7
    p := &owner
    &*p

open Lean Elab Command VerexIR in
run_cmd do
  let some unit := VerexSL.registeredUnit? (← getEnv) `«0x42».mixed_provenance
    | throwError "missing mixed-provenance fixture"
  match Validation.prepareExecution #[Move.semantics] unit with
  | .ok _ => throwError "a frame-owned alternative escaped"
  | .error diagnostics =>
      unless (diagnostics.filter (·.code == "VIR-SEMANTIC-BORROW-ESCAPE")).size == 2 do
        throwError "expected both frame escapes to be rejected: {repr diagnostics}"
