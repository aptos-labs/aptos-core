-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Control and expression forms: early exits, conditions, branches and
matches with effects, compound locals, and ranges. Arithmetic failures carry
the VM's computed payload. -/

leaner module 0x42::control_forms where
  struct Box has Copy, Drop, Store where
    value : u64

  -- ## Early exits

  fun return_in_loop(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      if remaining == 3 then return 1
      remaining := remaining - 1
    remaining
  spec return_in_loop where
    ensures result <= 1
    aborts_if false

  fun clamp(value : u64) -> u64 := do
    let mut result := value
    if value < 10 then
      result := value
    else
      result := 10
    result
  spec clamp where
    ensures result <= 10
    aborts_if false

  -- ## Conditions and operators

  fun then_else(flag : Bool) -> u64 := do
    let mut value : u64 := 0
    if flag then
      value := 1
    else
      value := 2
    value + 1
  spec then_else where
    ensures result == if flag then 2 else 3
    aborts_if false

  fun arithmetic_condition(value : u64) -> u64 :=
    if value + 1 < 2 then 1 else 0
  spec arithmetic_condition where
    ensures result == if value == 0 then 1 else 0
    aborts_if value + 1 > MAX_U64 with value + 1

  fun explicit_arithmetic_condition(value : u64) -> u64 :=
    if core.prim.checkedAddAbort(value, 1) < 2 then 1 else 0
  spec explicit_arithmetic_condition where
    ensures result == if value == 0 then 1 else 0
    aborts_if value + 1 > MAX_U64 with value + 1

  fun index_arithmetic(base : u64) -> u64 := do
    let values := vector<u64>[10, 20, 30]
    let value := &values[base + 1]
    *value
  spec index_arithmetic where
    requires base == 1
    ensures result == 30
    aborts_if false

  fun embedded(value : u64) -> Box := new Box { value := value + 1 }
  spec embedded where
    ensures result.value == value + 1
    aborts_if value + 1 > MAX_U64 with value + 1

  fun short_circuit_and(value : u64) -> u64 :=
    if value == 0 && value + 1 == 2 then 1 else 0
  spec short_circuit_and where
    ensures result == 0
    aborts_if false

  fun short_circuit_or(value : u64) -> Bool :=
    value == 18446744073709551615 || value + 1 == 2
  spec short_circuit_or where
    ensures result == (value == MAX_U64 || value == 1)
    aborts_if false

  fun eager_core_and(value : u64) -> Bool :=
    core.prim.logicalAnd(false, value + 1 == 0)
  spec eager_core_and where
    ensures result == false
    aborts_if value + 1 > MAX_U64 with value + 1

  -- ## Branches and matches with effects

  fun branch_effect(flag : Bool, value : u64) -> u64 :=
    if flag then value + 1 else 0
  spec branch_effect where
    ensures result == if flag then value + 1 else 0
    aborts_if flag && value + 1 > MAX_U64 with value + 1

  fun match_effect(flag : Bool, value : u64) -> u64 :=
    match flag with
      | true => value + 1
      | false => 0
  spec match_effect where
    ensures result == if flag then value + 1 else 0
    aborts_if flag && value + 1 > MAX_U64 with value + 1

  fun match_two(left : Bool, right : Bool) -> u64 :=
    match (left, right) with
      | (true, true) => 2
      | (true, false) => 1
      | (false, true) => 0
      | (false, false) => 0
  spec match_two where
    ensures result == if left then if right then 2 else 1 else 0
    aborts_if false

  fun echo_flag(flag : Bool) -> Bool := flag
  spec echo_flag where
    ensures result == flag
    aborts_if false

  fun if_let_action(flag : Bool) -> u64 :=
    match echo_flag(flag) with
      | true => 1
      | _ => 0
  spec if_let_action where
    ensures result == if flag then 1 else 0
    aborts_if false

  -- ## Loops, assertions, and compound locals

  fun dependent_while(value : u64) -> u64 := do
    while value < 1 do
      break
    value
  spec dependent_while where
    ensures result == value
    aborts_if false

  fun checked_assert(flag : Bool) -> u64 := do
    assert(flag, 17)
    1
  spec checked_assert where
    ensures result == 1
    aborts_if !flag with 17

  fun checked_assert_syntax(flag : Bool) -> u64 := do
    assert!(flag, 18)
    10

  fun checked_assert_eq(left : u64, right : u64) -> Unit := assert!(left == right, 19)

  fun checked_assert_ne(left : u64, right : u64) -> Unit := assert!(left != right, 20)

  fun compound_local() -> u64 := do
    let mut value : u64 := 4
    value := value + 2
    value := value * 3
    value
  spec compound_local where
    ensures result == 18

  fun compound_reference() -> u64 := do
    let mut value : u64 := 20
    let value_ref := &mut value
    *value_ref := *value_ref - 5
    *value_ref := *value_ref / 3
    *value_ref
  spec compound_reference where
    ensures result == 5
    aborts_if false

  -- ## Ranges

  fun range_empty() -> u64 := do
    let mut value : u64 := 0
    for index in 4..4 do
      value := value + index
    value
  spec range_empty where
    ensures result == 0
    aborts_if false

  fun range_once_runtime() -> u64 := do
    let mut value : u64 := 0
    for index in 4..5 do
      value := value + index
    value

-- Printing the registered unit and re-importing it gives the same source: the
-- printer's output is a canonical fixed point.
open Lean Elab Command in
run_cmd do
  let env ← getEnv
  let some unit := LeanerLang.registeredUnit? env `«0x42».control_forms
    | throwError "missing control forms"
  let .ok printed := LeanerLang.Print.render env unit
    | throwError "control forms did not render"
  let .ok formatted := LeanerLang.Print.formatSource env printed
    | throwError "control forms did not re-import:\n{printed}"
  unless formatted == printed do
    throwError "control forms are not a canonical fixed point:\n{printed}\n{formatted}"

-- Run the functions on concrete inputs in the interpreter and compare the
-- outcomes on both arms of each branch.
open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let box : StructHandle := { namespaceId := ⟨0⟩, structId := 0 }
  assertRuns `«0x42».control_forms #[
    ⟨"return_in_loop", #[.integer 5], .returned #[.integer 1], {}⟩,
    ⟨"return_in_loop", #[.integer 2], .returned #[.integer 0], {}⟩,
    ⟨"clamp", #[.integer 50], .returned #[.integer 10], {}⟩,
    ⟨"clamp", #[.integer 4], .returned #[.integer 4], {}⟩,
    ⟨"then_else", #[.bool true], .returned #[.integer 2], {}⟩,
    ⟨"then_else", #[.bool false], .returned #[.integer 3], {}⟩,
    ⟨"arithmetic_condition", #[.integer 0], .returned #[.integer 1], {}⟩,
    ⟨"arithmetic_condition", #[.integer 5], .returned #[.integer 0], {}⟩,
    ⟨"explicit_arithmetic_condition", #[.integer 0], .returned #[.integer 1], {}⟩,
    ⟨"index_arithmetic", #[.integer 1], .returned #[.integer 30], {}⟩,
    ⟨"embedded", #[.integer 4], .returned #[.nominal box none #[.integer 5]], {}⟩,
    ⟨"short_circuit_and", #[.integer 0], .returned #[.integer 0], {}⟩,
    ⟨"short_circuit_and", #[.integer 18446744073709551615], .returned #[.integer 0], {}⟩,
    ⟨"branch_effect", #[.bool false, .integer 18446744073709551615], .returned #[.integer 0], {}⟩,
    ⟨"branch_effect", #[.bool true, .integer 5], .returned #[.integer 6], {}⟩,
    ⟨"branch_effect", #[.bool true, .integer 18446744073709551615], .threw .abort #[.integer 18446744073709551616], {}⟩,
    ⟨"match_effect", #[.bool false, .integer 18446744073709551615], .returned #[.integer 0], {}⟩,
    ⟨"match_effect", #[.bool true, .integer 5], .returned #[.integer 6], {}⟩,
    ⟨"match_two", #[.bool true, .bool true], .returned #[.integer 2], {}⟩,
    ⟨"match_two", #[.bool true, .bool false], .returned #[.integer 1], {}⟩,
    ⟨"match_two", #[.bool false, .bool true], .returned #[.integer 0], {}⟩,
    ⟨"if_let_action", #[.bool true], .returned #[.integer 1], {}⟩,
    ⟨"if_let_action", #[.bool false], .returned #[.integer 0], {}⟩,
    ⟨"dependent_while", #[.integer 0], .returned #[.integer 0], {}⟩,
    ⟨"dependent_while", #[.integer 7], .returned #[.integer 7], {}⟩,
    ⟨"checked_assert", #[.bool true], .returned #[.integer 1], {}⟩,
    ⟨"checked_assert", #[.bool false], .threw .abort #[.integer 17], {}⟩,
    ⟨"checked_assert_syntax", #[.bool true], .returned #[.integer 10], {}⟩,
    ⟨"checked_assert_syntax", #[.bool false], .threw .abort #[.integer 18], {}⟩,
    ⟨"checked_assert_eq", #[.integer 3, .integer 3], .returned #[], {}⟩,
    ⟨"checked_assert_eq", #[.integer 3, .integer 4], .threw .abort #[.integer 19], {}⟩,
    ⟨"checked_assert_ne", #[.integer 3, .integer 4], .returned #[], {}⟩,
    ⟨"checked_assert_ne", #[.integer 3, .integer 3], .threw .abort #[.integer 20], {}⟩,
    ⟨"compound_local", #[], .returned #[.integer 18], {}⟩,
    ⟨"compound_reference", #[], .returned #[.integer 5], {}⟩,
    ⟨"range_empty", #[], .returned #[.integer 0], {}⟩,
    ⟨"range_once_runtime", #[], .returned #[.integer 4], {}⟩,
    ⟨"index_arithmetic", #[.integer 18446744073709551615], .threw .abort #[.integer 18446744073709551616], {}⟩,
    ⟨"short_circuit_or", #[.integer 18446744073709551615], .returned #[.bool true], {}⟩,
    ⟨"short_circuit_or", #[.integer 1], .returned #[.bool true], {}⟩,
    ⟨"short_circuit_or", #[.integer 0], .returned #[.bool false], {}⟩,
    ⟨"eager_core_and", #[.integer 0], .returned #[.bool false], {}⟩,
    ⟨"eager_core_and", #[.integer 18446744073709551615], .threw .abort #[.integer 18446744073709551616], {}⟩]
