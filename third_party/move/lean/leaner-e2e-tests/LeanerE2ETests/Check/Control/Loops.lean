-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Structured loops. Loop typing supplies the default invariant; the second
phase of `two_phases` needs a mathematical bound. -/

leaner module 0x42::loops where
  -- ## Loop forms

  fun count_down(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      remaining := remaining - 1
    remaining
  spec count_down where
    ensures result == 0
    aborts_if false

  fun count_down_loop(n : u64) -> u64 := do
    let mut remaining := n
    loop do
      if remaining < 1 then break
      remaining := remaining - 1
    remaining
  spec count_down_loop where
    ensures result == 0
    aborts_if false

  fun skip_evens(n : u64, acc : u64) -> u64 := do
    let mut remaining := n
    let mut total := acc
    while 0 < remaining do
      remaining := remaining - 1
      if remaining % 2 == 0 then continue
      total := total + 1
    total

  fun two_phases(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      remaining := remaining - 1
    while remaining < 3 do
      remaining := remaining + 1
    where
      invariant remaining <= 3
    remaining
  spec two_phases where
    ensures result == 3
    aborts_if false

  fun nested(x : u64) -> u64 := do
    let mut remaining := x
    while 0 < remaining do
      while 10 < remaining do
        remaining := remaining - 10
      remaining := remaining - 1
    remaining

  fun shadowed_loop_state(n : u64) -> u64 := do
    loop do
      let mut n : u64 := 2
      n := 1
      break
    n
  spec shadowed_loop_state where
    ensures result == n
    aborts_if false

  -- LeanerLang has no separate Action/pure binding layer. Both binding
  -- spellings lower to ordinary locals and assignments here.
  fun shadowed_loop_arrow(n : u64) -> u64 := do
    loop do
      let mut n : u64 := 2
      n := 1
      break
    n
  spec shadowed_loop_arrow where
    ensures result == n
    aborts_if false

  fun arrow_reassign_loop(n : u64) -> u64 := do
    let mut remaining := n
    loop do
      remaining := 0
      break
    remaining

  -- ## Loops over storage

  struct Counter has Key where
    value : u64

  fun drain(addr : Address) -> u64 := do
    let value := &mut Counter[addr].value
    let mut remaining := *value
    while 0 < remaining do
      remaining := remaining - 1
    *value := remaining
    remaining
  spec drain where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures global<Counter>(addr).value == 0
    aborts_if false

  -- ## Tail recurrence, labels, and early exits

  -- A tail recurrence is a loop over the parameter row; retain
  -- that structure without introducing a recursive call or helper function.
  fun countdown_tail(value : u64, accumulator : u64) -> u64 := do
    let mut remaining := value
    let mut total := accumulator
    loop do
      if remaining < 1 then break
      remaining := remaining - 1
      total := total + 1
    total

  fun labeled_exit(n : u64) -> u64 := do
    let mut remaining := n
    loop@outer do
      loop do
        if remaining < 1 then break@outer
        remaining := remaining - 1
        break
    remaining
  spec labeled_exit where
    ensures result == 0
    aborts_if false

  fun labeled_continue(n : u64) -> u64 := do
    let mut remaining := n
    loop@outer do
      loop do
        if remaining < 1 then break@outer
        remaining := remaining - 1
        continue@outer
    remaining
  spec labeled_continue where
    ensures result == 0
    aborts_if false

  fun labeled_proof() -> u64 := do
    loop@outer do
      loop do
        break@outer
    7
  spec labeled_proof where
    ensures result == 7
    aborts_if false

  fun early(flag : Bool) -> u64 := do
    if flag then return 7
    8
  spec early where
    ensures result == if flag then 7 else 8
    aborts_if false

  fun return_in_loop(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      if remaining == 3 then return 1
      remaining := remaining - 1
    remaining
  spec return_in_loop where
    ensures result <= 1
    aborts_if false

-- Run the functions on concrete inputs in the interpreter and compare the
-- outcomes, including zero iterations and early exits.
open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».loops #[
    ⟨"count_down", #[.integer 5], .returned #[.integer 0], {}⟩,
    ⟨"count_down", #[.integer 0], .returned #[.integer 0], {}⟩,
    ⟨"count_down_loop", #[.integer 5], .returned #[.integer 0], {}⟩,
    ⟨"skip_evens", #[.integer 5, .integer 0], .returned #[.integer 2], {}⟩,
    ⟨"two_phases", #[.integer 2], .returned #[.integer 3], {}⟩,
    ⟨"nested", #[.integer 25], .returned #[.integer 0], {}⟩,
    ⟨"shadowed_loop_state", #[.integer 7], .returned #[.integer 7], {}⟩,
    ⟨"shadowed_loop_arrow", #[.integer 7], .returned #[.integer 7], {}⟩,
    ⟨"arrow_reassign_loop", #[.integer 3], .returned #[.integer 0], {}⟩,
    ⟨"labeled_exit", #[.integer 5], .returned #[.integer 0], {}⟩,
    ⟨"labeled_continue", #[.integer 5], .returned #[.integer 0], {}⟩,
    ⟨"labeled_proof", #[], .returned #[.integer 7], {}⟩,
    ⟨"early", #[.bool true], .returned #[.integer 7], {}⟩,
    ⟨"early", #[.bool false], .returned #[.integer 8], {}⟩,
    ⟨"return_in_loop", #[.integer 5], .returned #[.integer 1], {}⟩,
    ⟨"return_in_loop", #[.integer 2], .returned #[.integer 0], {}⟩,
    ⟨"countdown_tail", #[.integer 100, .integer 40], .returned #[.integer 140], {}⟩]
  let initial ← singleResourceState `«0x42».loops "Counter" "0x3" #[.integer 4]
  let final ← singleResourceState `«0x42».loops "Counter" "0x3" #[.integer 0] 2
  assertRunsState `«0x42».loops #[
    ⟨"drain", #[.address "0x3"], .returned #[.integer 0], initial, final⟩]

-- The loops lowered in place, without helper functions.
open Lean Elab Command LeanerIR in
run_cmd do
  let env ← getEnv
  let some unit := LeanerLang.registeredUnit? env `«0x42».loops
    | throwError "missing loop module"
  let ns := unit.namespaces[0]!
  unless ns.functions.size == 15 do
    throwError "loops introduced helper functions: {ns.functions.size}"
  -- Loops stay structured, without helper or self calls. The shared
  -- IR retains structured loops; check the corresponding representation.
  for (function, expectedLoops) in [
      ("count_down", 1), ("count_down_loop", 1), ("two_phases", 2),
      ("nested", 2), ("labeled_exit", 2), ("labeled_continue", 2),
      ("labeled_proof", 2), ("countdown_tail", 1), ("return_in_loop", 1)] do
    let some (_, _, _, declaration) := LeanerLang.Contract.findFunction? unit function
      | throwError "missing function: {function}"
    let .structured root := declaration.body
      | throwError "loop function has no structured body: {function}"
    let mut pending := #[root]
    let mut seen : Array ExprId := #[]
    let mut loops := 0
    let mut returns := 0
    while !pending.isEmpty do
      let current := pending.back!
      pending := pending.pop
      if seen.contains current then continue
      seen := seen.push current
      let kind := ns.expressions[current.index]!.kind
      match kind with
      | .loop .. => loops := loops + 1
      | .return_ .. => returns := returns + 1
      | .operation (.call _) .. => throwError "loop lowered to a call: {function}"
      | _ => pure ()
      pending := pending ++ Validation.expressionChildren kind
    unless loops == expectedLoops do
      throwError "wrong structured loop count in {function}: {loops}"
    -- The normal exit is the structured body's value; only the early exit
    -- remains an explicit return node, unlike the two CFG ret terminators.
    if function == "return_in_loop" && returns == 0 then
      throwError "early loop return was lost"
  let .ok printed := LeanerLang.Print.render env unit
    | throwError "loop module did not render"
  let .ok formatted := LeanerLang.Print.formatSource env printed
    | throwError "loop module did not re-import:\n{printed}"
  unless printed == formatted do
    throwError "loop module is not a canonical fixed point:\n{printed}\n{formatted}"

-- A block-tail return is only a fallthrough value at the function tail.
-- These regressions execute the source IR as well as checking native proofs.
leaner module 0x42::return_scopes where
  fun loop_local(n : u64) -> u64 := do
    loop do
      let next := n + 1
      return next
    0
  spec loop_local where
    requires n < 100
    ensures result == n + 1
    aborts_if false

  fun unit_operand(flag : Bool) -> Unit := do
    if flag then return assert(!flag, 19)
    ()
  spec unit_operand where
    aborts_if flag with 19

-- Run the functions on concrete inputs in the interpreter and compare the
-- outcomes: a `return` inside a loop or a branch leaves the function, not just
-- the block.
open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».return_scopes #[
    ⟨"loop_local", #[.integer 4], .returned #[.integer 5], {}⟩,
    ⟨"unit_operand", #[.bool true], .threw .abort #[.integer 19], {}⟩,
    ⟨"unit_operand", #[.bool false], .returned #[], {}⟩]
