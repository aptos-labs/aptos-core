-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport
set_option leaner.route "native"

leaner module 0x42::vector_bounds where
  fun read_bad() -> u64 := do
    let values := vector<u64>[1]
    let value := &values[1]
    return *value
  spec read_bad where
    ensures false
    aborts_if true
  verify read_bad

  fun write_bad() -> u64 := do
    let mut values := vector<u64>[1]
    let value := &mut values[1]
    *value := 9
    return values.length
  spec write_bad where
    ensures false
    aborts_if true
  verify write_bad

  fun direct_write_bad() -> Unit := do
    let mut values := vector<u64>[1]
    values[1] := 9
  spec direct_write_bad where
    ensures false
    aborts_if true
  verify direct_write_bad

  fun read_reference(values : &Vector<u64>) -> u64 := do
    let value := &values[1]
    return *value
  spec read_reference where
    requires values.length <= 1
    ensures false
    aborts_if true
  verify read_reference

  fun read_mutable_reference(values : &mut Vector<u64>) -> u64 := do
    let value := &values[1]
    return *value
  spec read_mutable_reference where
    requires values.length <= 1
    ensures false
    aborts_if true
  verify read_mutable_reference

  fun bad_mutable_reference() -> u64 := do
    let mut values := vector<u64>[1]
    return read_mutable_reference(&mut values)

  struct Counter has Key where
    value : u64

  fun rollback(addr : Address) -> u64 := do
    let counter := &mut Counter[addr].value
    *counter := 9
    let values := vector<u64>[]
    let value := &values[0]
    return *value

  fun next_index(addr : Address) -> u64 := do
    let counter := &mut Counter[addr].value
    *counter := *counter + 1
    return *counter

  fun indexed_once(addr : Address) -> u64 := do
    let values := vector<u64>[10, 20, 30]
    let value := &values[next_index(addr)]
    return *value

  fun nested_indexed_once(addr : Address) -> u64 := do
    let values := vector<Vector<u64> >[vector<u64>[10], vector<u64>[20], vector<u64>[30]]
    let value := &values[next_index(addr)][0]
    return *value

  fun direct_write_once(addr : Address) -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    values[next_index(addr)] := 99
    let value := &values[1]
    return *value

  fun nested_direct_write_once(addr : Address) -> u64 := do
    let mut values := vector<Vector<u64> >[vector<u64>[10], vector<u64>[20]]
    values[next_index(addr)][0] := 99
    let value := &values[1][0]
    return *value

  fun ordered_direct_write(addr : Address) -> u64 := do
    let mut values := vector<u64>[0, 0, 0]
    values[next_index(addr)] := next_index(addr)
    let value := &values[2]
    return *value

  fun rhs_abort_before_bounds() -> Unit := do
    let mut values := vector<u64>[]
    values[0] := abort(7)
  spec rhs_abort_before_bounds where
    ensures false
    aborts_if true with 7
  verify rhs_abort_before_bounds

  struct Owned has Drop, Store where
    value : u64

  fun replace_owned() -> u64 := do
    let mut values := vector<Owned>[new Owned { value := 1 }]
    values[0] := new Owned { value := 9 }
    let value := &values[0].value
    return *value

#leaner_require_native 0x42::vector_bounds::read_bad
#leaner_require_native 0x42::vector_bounds::read_reference

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let bounds : ThrowKind := .profile { profile := .move, tag := "runtime.vector_error" }
  assertRuns `«0x42».vector_bounds #[
    ⟨"read_bad", #[], .threw bounds #[.integer 1], {}⟩,
    ⟨"write_bad", #[], .threw bounds #[.integer 1], {}⟩,
    ⟨"direct_write_bad", #[], .threw bounds #[.integer 1], {}⟩,
    ⟨"read_reference", #[.vector #[.integer 1]], .threw bounds #[.integer 1], {}⟩,
    ⟨"read_reference", #[.vector #[.integer 1, .integer 7]], .returned #[.integer 7], {}⟩,
    ⟨"bad_mutable_reference", #[], .threw bounds #[.integer 1], {}⟩,
    ⟨"rhs_abort_before_bounds", #[], .threw .abort #[.integer 7], {}⟩,
    ⟨"replace_owned", #[], .returned #[.integer 9], {}⟩]
  let initial ← singleResourceState `«0x42».vector_bounds "Counter" "0x2" #[.integer 0]
  let incremented ← singleResourceState `«0x42».vector_bounds "Counter" "0x2" #[.integer 1] 2
  let twice ← singleResourceState `«0x42».vector_bounds "Counter" "0x2" #[.integer 2] 4
  assertRunsState `«0x42».vector_bounds #[
    ⟨"rollback", #[.address "0x2"], .threw bounds #[.integer 1], initial, initial⟩,
    ⟨"indexed_once", #[.address "0x2"], .returned #[.integer 20], initial, incremented⟩,
    ⟨"nested_indexed_once", #[.address "0x2"], .returned #[.integer 20], initial, incremented⟩,
    ⟨"direct_write_once", #[.address "0x2"], .returned #[.integer 99], initial, incremented⟩,
    ⟨"nested_direct_write_once", #[.address "0x2"], .returned #[.integer 99], initial, incremented⟩,
    ⟨"ordered_direct_write", #[.address "0x2"], .returned #[.integer 1], initial, twice⟩]

open Lean Elab Command in
run_cmd do
  let env ← getEnv
  let some unit := LeanerLang.registeredUnit? env `«0x42».vector_bounds
    | throwError "missing source"
  let .ok printed := LeanerLang.Print.render env unit
    | throwError "printing failed"
  let formatted ← match LeanerLang.Print.formatSource env printed with
    | .ok value => pure value
    | .error reason => throwError "reimport failed: {repr reason}\n{printed}"
  unless printed == formatted do
    throwError "not fixed point: {printed}\n{formatted}"
  for function in ["read_bad", "write_bad", "direct_write_bad", "read_reference",
      "read_mutable_reference", "rhs_abort_before_bounds"] do
    let name := ((`«0x42».vector_bounds).str function).str "verified"
    unless env.contains name do throwError "missing bounds proof {name}"
    if (← collectAxioms name).contains ``sorryAx then
      throwError "bounds proof contains an admission: {name}"
