-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Borrow policy

Source programs checked by the LIR borrow checker at execution preparation:
the accepted programs share one module and verify; every rejected program has
its own module, since preparation rejects a whole unit. Borrow certificates
are covered by `References/BorrowCertificates`.

An immutable observation of one field does not block a mutable borrow of a
sibling field. Two overlapping mutable handles passed to a callee that
ignores them are rejected, as the Move VM's exclusive call lock rejects
them.
-/

leaner module 0x42::borrow_accepted where
  struct Pair has Copy, Drop where
    left : u64
    right : u64

  struct Cell has Copy, Drop where
    value : u64

  -- ## Coexisting handles

  -- Overlapping handles coexist; the unused poisoned one is dropped.
  fun drop_poisoned() -> u64 := do
    let mut owner : u64 := 0
    let left := &mut owner
    let _right := &mut owner
    *left := 1
    owner
  spec drop_poisoned where
    ensures result == 1
    aborts_if false

  -- An immutable observation of one field does not block a sibling write.
  fun observe_sibling() -> u64 := do
    let mut owner := new Pair { left := 1, right := 2 }
    let observation := &owner.right
    let writer := &mut owner.left
    *writer := 3
    *observation
  spec observe_sibling where
    ensures result == 2
    aborts_if false

  fun disjoint_siblings() -> u64 := do
    let mut owner := new Pair { left := 0, right := 0 }
    let left := &mut owner.left
    let right := &mut owner.right
    *left := 1
    *right := 2
    owner.left + owner.right
  spec disjoint_siblings where
    ensures result == 3
    aborts_if false

  -- ## Loans across control flow

  fun loop_carries_mutation() -> u64 := do
    let mut owner : u64 := 0
    let writer := &mut owner
    let mut remaining : u64 := 3
    while 0 < remaining do
      *writer := *writer + 1
      remaining := remaining - 1
    where
      invariant remaining <= 3 && *writer + remaining == 3
    owner
  spec loop_carries_mutation where
    ensures result == 3
    aborts_if false

  fun break_unwinds_loan(parent : &mut Cell) -> Unit := do
    loop do
      let child := &mut (*parent).value
      *child := 1
      break
    *parent := new Cell { value := 2 }
  spec break_unwinds_loan where
    ensures parent.value == 2
    aborts_if false

  fun continue_unwinds_loan(parent : &mut Cell) -> Unit := do
    let mut remaining : u64 := 2
    while 0 < remaining do
      remaining := remaining - 1
      let child := &mut (*parent).value
      *child := remaining
      continue
    *parent := new Cell { value := 2 }
  spec continue_unwinds_loan where
    ensures parent.value == 2
    aborts_if false

  fun labeled_break_unwinds_loan(parent : &mut Cell) -> Unit := do
    loop @outer do
      loop do
        let child := &mut (*parent).value
        *child := 1
        break @outer
    *parent := new Cell { value := 2 }
  spec labeled_break_unwinds_loan where
    ensures parent.value == 2
    aborts_if false

  fun return_stops_control_flow(flag : Bool) -> u64 := do
    let mut first : u64 := 1
    let mut second : u64 := 2
    if flag then
      let loan := &mut first
      *loan := 3
      return first
    let loan := &mut second
    *loan := 4
    second
  spec return_stops_control_flow where
    ensures result == (if flag then 3 else 4)
    aborts_if false

  -- ## Children, calls, and returned references

  fun child_reconciles_into_parent(parent : &mut Cell) -> Unit := do
    let child := &mut (*parent).value
    *child := 1
    *parent := new Cell { value := (*parent).value + 1 }
  spec child_reconciles_into_parent where
    ensures parent.value == 2
    aborts_if false

  fun write_first(left : &mut u64, right : &u64) -> Unit := *left := *right

  fun separated_call(left : &mut u64, right : &mut u64) -> Unit := write_first(left, right)
  spec separated_call where
    ensures left == old(right) && right == old(right)
    aborts_if false

  fun field_of(input : &Cell) -> &u64 := &(*input).value

  fun returned_reference_through_call(input : &Cell) -> &u64 := field_of(input)

  fun choose(flag : Bool, left : &mut u64, right : &mut u64) -> &mut u64 :=
    if flag then left else right

  fun dropping_result_revives_inputs(flag : Bool) -> u64 := do
    let mut left : u64 := 1
    let mut right : u64 := 2
    let result := choose(flag, &mut left, &mut right)
    *result := 5
    left + right
  spec dropping_result_revives_inputs where
    ensures result == (if flag then 7 else 6)
    aborts_if false

  fun read_three(first : &u64, second : &u64, third : &u64) -> u64 :=
    *first + *second + *third

  fun multiple_immutable_references() -> u64 := do
    let owner : u64 := 1
    let first := &owner
    let second := &owner
    let third := &owner
    read_three(first, second, third)
  spec multiple_immutable_references where
    ensures result == 3
    aborts_if false

  fun return_derived_reference(input : &u64) -> &u64 := input

/-! ## Rejected programs -/

leaner module 0x42::borrow_poisoned_use where
  fun poisoned_use() -> u64 := do
    let mut owner : u64 := 0
    let left := &mut owner
    let right := &mut owner
    *left := 1
    *right

leaner module 0x42::borrow_call_poisons_alias where
  fun write_one(target : &mut u64) -> Unit := *target := 1

  fun call_poisons_alias() -> u64 := do
    let mut owner : u64 := 0
    let left := &mut owner
    let right := &mut owner
    write_one(left)
    *right

leaner module 0x42::borrow_poison_across_branch where
  fun poison_across_branch(flag : Bool) -> u64 := do
    let mut owner : u64 := 0
    let left := &mut owner
    let right := &mut owner
    if flag then
      *left := 1
    *right

leaner module 0x42::borrow_poison_across_iteration where
  fun poison_across_iteration() -> u64 := do
    let mut owner : u64 := 0
    let left := &mut owner
    let right := &mut owner
    let mut remaining : u64 := 2
    while 0 < remaining do
      *left := remaining
      remaining := remaining - 1
    *right

leaner module 0x42::borrow_concrete_call_conflict where
  fun write_first(left : &mut u64, right : &mut u64) -> Unit := *left := *right

  fun concrete_call_conflict() -> Unit := do
    let mut owner : u64 := 0
    let left := &mut owner
    let right := &mut owner
    write_first(left, right)

leaner module 0x42::borrow_returned_mutation_suspends where
  fun choose(flag : Bool, left : &mut u64, right : &mut u64) -> &mut u64 :=
    if flag then left else right

  fun returned_mutation_suspends(flag : Bool, left : &mut u64, right : &mut u64) -> u64 := do
    let result := choose(flag, left, right)
    let observed := *left
    *result := 5
    observed

leaner module 0x42::borrow_immutable_then_write where
  struct Cell has Copy, Drop where
    value : u64

  fun immutable_then_write() -> u64 := do
    let mut owner := new Cell { value := 0 }
    let observation := &owner.value
    let writer := &mut owner.value
    *writer := 1
    *observation

leaner module 0x42::borrow_overlapping_call where
  fun ignore_both(first : &mut u64, second : &mut u64) -> Unit := ()

  fun overlapping_call() -> Unit := do
    let mut owner : u64 := 0
    let first := &mut owner
    let second := &mut owner
    ignore_both(first, second)

leaner module 0x42::borrow_freeze_poisoned where
  struct Cell has Copy, Drop where
    value : u64

  fun read_one(value : &u64) -> u64 := *value

  fun freeze_poisoned() -> u64 := do
    let mut owner := new Cell { value := 0 }
    let field := &mut owner.value
    let parent := &mut owner
    *parent := new Cell { value := 1 }
    read_one(field)

leaner module 0x42::borrow_overwrite_owner where
  fun overwrite_owner() -> u64 := do
    let mut owner : u64 := 0
    let reference := &owner
    owner := 5
    *reference

leaner module 0x42::borrow_return_local where
  fun return_local() -> &u64 := do
    let owner : u64 := 1
    &owner

leaner module 0x42::borrow_vector_elements where
  fun vector_elements() -> u64 := do
    let mut values := vector<u64>[1, 2]
    let zero := &mut values[0]
    let one := &mut values[1]
    *zero := 3
    *one

-- Execution preparation rejects each program above with the expected borrow
-- diagnostic; each has its own module because preparation rejects a whole unit.
open Lean Elab Command LeanerIR in
run_cmd do
  let expectations : List (Name × String) := [
    (`«0x42».borrow_poisoned_use, "LIR-SEMANTIC-BORROW-CONFLICT"),
    (`«0x42».borrow_call_poisons_alias, "LIR-SEMANTIC-BORROW-CONFLICT"),
    (`«0x42».borrow_poison_across_branch, "LIR-SEMANTIC-BORROW-CONFLICT"),
    (`«0x42».borrow_poison_across_iteration, "LIR-SEMANTIC-BORROW-CONFLICT"),
    (`«0x42».borrow_concrete_call_conflict, "LIR-SEMANTIC-BORROW-CONFLICT"),
    (`«0x42».borrow_returned_mutation_suspends, "LIR-SEMANTIC-BORROW-CONFLICT"),
    (`«0x42».borrow_immutable_then_write, "LIR-SEMANTIC-BORROW-CONFLICT"),
    (`«0x42».borrow_overlapping_call, "LIR-SEMANTIC-BORROW-CONFLICT"),
    (`«0x42».borrow_freeze_poisoned, "LIR-SEMANTIC-BORROW-CONFLICT"),
    (`«0x42».borrow_overwrite_owner, "LIR-SEMANTIC-BORROW-CONFLICT"),
    (`«0x42».borrow_return_local, "LIR-SEMANTIC-BORROW-ESCAPE"),
    (`«0x42».borrow_vector_elements, "LIR-SEMANTIC-BORROW-CONFLICT")]
  for (name, code) in expectations do
    let some unit := LeanerLang.registeredUnit? (← getEnv) name
      | throwError "missing borrow fixture {name}"
    match Validation.prepareExecution #[Move.semantics] unit with
    | .ok _ => throwError "borrow violation accepted: {name}"
    | .error diagnostics =>
      unless diagnostics.size == 1 && diagnostics.all (fun diagnostic =>
          diagnostic.code == code && diagnostic.primary.isSome) do
        throwError "wrong borrow rejection for {name}: {repr diagnostics}"
  let some unit := LeanerLang.registeredUnit? (← getEnv) `«0x42».borrow_accepted
    | throwError "missing accepted borrow fixture"
  match Validation.prepareExecution #[Move.semantics] unit with
  | .ok _ => pure ()
  | .error diagnostics => throwError "accepted borrow fixture rejected: {repr diagnostics}"
