-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of v0 Language/VectorOperations. Shared value-level operations
retain explicit writes at their borrowed-place boundary.
Move library bounds assertions retain their codes; native vector failures
are VM errors, not v0's blanket 0x20000 abort. -/

set_option leaner.route "native"

leaner module 0x42::vector_operations where
  fun empty_length() -> u64 := vector<u64>[].length
  spec empty_length where
    ensures result == 0
  verify empty_length

  fun singleton_value() -> u64 := do
    let values := vector<u64>[7]
    let value := &values[0]
    return *value

  fun emptiness(flag : Bool) -> Bool :=
    if flag then vector<u64>[].length == 0 else vector<u64>[1].length == 0

  fun pushed() -> u64 := do
    let values := core.prim.pushVector(vector<u64>[3, 4], 9)
    let value := &values[2]
    return *value
  spec pushed where
    ensures result == 9
    aborts_if false
  verify pushed

  fun set_edges() -> u64 := do
    let mut values := vector<u64>[1, 2, 3]
    let first := &mut values[0]
    *first := 10
    let last := &mut values[2]
    *last := 30
    let first_value := &values[0]
    let left := *first_value
    let last_value := &values[2]
    let right := *last_value
    return left + right
  spec set_edges where
    ensures result == 40
    aborts_if false

  fun nested() -> u64 := do
    let values := vector<Vector<u64> >[vector<u64>[1, 2], vector<u64>[3, 4]]
    let row_ref := &values[1]
    let row := *row_ref
    let value := &row[0]
    return *value
  spec nested where
    ensures result == 3
    aborts_if false
  verify nested

  fun borrowed_length() -> u64 := do
    let values := vector<u64>[1, 2, 3]
    let values_ref := &values
    return (*values_ref).length
  spec borrowed_length where
    ensures result == 3
    aborts_if false
  verify borrowed_length

  fun bool_round_trip(value : Bool) -> Bool := do
    let values := vector<Bool>[value]
    let result := &values[0]
    return *result
  spec bool_round_trip where
    ensures result == value
    aborts_if false
  verify bool_round_trip

  fun mutate_and_read() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    let middle := &mut values[1]
    *middle := *middle + 7
    return *middle
  spec mutate_and_read where
    ensures result == 27
    aborts_if false
  verify mutate_and_read

  fun mutate_then_borrow_other() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    let first := &mut values[0]
    *first := 99
    let last := &values[2]
    return *last
  spec mutate_then_borrow_other where
    ensures result == 30
    aborts_if false

  fun freeze_element() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    let middle := &mut values[1]
    *middle := 55
    let immutable := core.ref.freezeExplicit(middle)
    return *immutable

  fun insert_middle() -> u64 := do
    let mut values := vector<u64>[10, 30]
    let values_ref := &mut values
    assert(1 <= (*values_ref).length, 131072)
    *values_ref := core.prim.insertVector(*values_ref, 1, 20)
    let updated := *values_ref
    let middle := &updated[1]
    return *middle
  spec insert_middle where
    ensures result == 20
    aborts_if false
  verify insert_middle

  fun insert_edges() -> u64 := do
    let mut values := vector<u64>[20]
    let values_ref := &mut values
    assert(0 <= (*values_ref).length, 131072)
    *values_ref := core.prim.insertVector(*values_ref, 0, 10)
    assert(2 <= (*values_ref).length, 131072)
    *values_ref := core.prim.insertVector(*values_ref, 2, 30)
    let updated := *values_ref
    let first := &updated[0]
    let left := *first
    let last := &updated[2]
    let right := *last
    return left + right + updated.length
  spec insert_edges where
    ensures result == 43
    aborts_if false

  fun remove_middle() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    let values_ref := &mut values
    assert(1 < (*values_ref).length, 131072)
    let (removed, rest) := core.prim.removeVector(*values_ref, 1)
    *values_ref := rest
    let updated := *values_ref
    let shifted := &updated[1]
    let shifted_value := *shifted
    return removed + shifted_value + updated.length
  spec remove_middle where
    ensures result == 52
    aborts_if false

  fun swap_values() -> u64 := do
    let mut values := vector<u64>[1, 2, 3]
    let values_ref := &mut values
    *values_ref := core.prim.swapVector(*values_ref, 0, 2)
    let updated := *values_ref
    let first := &updated[0]
    let last := &updated[2]
    let first_value := *first
    let last_value := *last
    return first_value * 10 + last_value
  spec swap_values where
    ensures result == 31
    aborts_if false
  verify swap_values

  fun pop_back() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    let values_ref := &mut values
    if (*values_ref).length == 0 then
      moveVectorError(2)
    let (removed, rest) := core.prim.removeVector(*values_ref, (*values_ref).length - 1)
    *values_ref := rest
    return removed
  spec pop_back where
    ensures result == 30
    aborts_if false
  verify pop_back

  fun pop_empty() -> u64 := do
    let mut values := vector<u64>[]
    let values_ref := &mut values
    if (*values_ref).length == 0 then
      moveVectorError(2)
    let (removed, rest) := core.prim.removeVector(*values_ref, (*values_ref).length - 1)
    *values_ref := rest
    return removed
  spec pop_empty where
    ensures false
    aborts_if true
  verify pop_empty

  fun swap_remove_value() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    let values_ref := &mut values
    assert((*values_ref).length != 0, 131072)
    let last := (*values_ref).length - 1
    *values_ref := core.prim.swapVector(*values_ref, 0, last)
    let (removed, rest) := core.prim.removeVector(*values_ref, last)
    *values_ref := rest
    let updated := *values_ref
    let first := &updated[0]
    let first_value := *first
    return removed + first_value + updated.length
  spec swap_remove_value where
    ensures result == 42
    aborts_if false
  verify swap_remove_value

  fun append_values() -> u64 := do
    let mut values := vector<u64>[1, 2]
    let values_ref := &mut values
    *values_ref := core.prim.concatVector(*values_ref, vector<u64>[3, 4])
    let updated := *values_ref
    let last := &updated[3]
    let last_value := *last
    return last_value + updated.length
  spec append_values where
    ensures result == 8
    aborts_if false
  verify append_values

  fun trim_values() -> u64 := do
    let mut values := vector<u64>[1, 2, 3, 4]
    let values_ref := &mut values
    assert(2 <= (*values_ref).length, 131072)
    let evicted := core.prim.slice(*values_ref, 2, (*values_ref).length)
    *values_ref := core.prim.slice(*values_ref, 0, 2)
    let updated := *values_ref
    let first_evicted := &evicted[0]
    let first_evicted_value := *first_evicted
    return updated.length + evicted.length + first_evicted_value
  spec trim_values where
    ensures result == 7
    aborts_if false
  verify trim_values

  fun reverse_slice_values() -> u64 := do
    let mut values := vector<u64>[1, 2, 3, 4]
    let values_ref := &mut values
    *values_ref := core.prim.reverseSliceVector(*values_ref, 1, 4)
    let updated := *values_ref
    let middle := &updated[1]
    return *middle
  spec reverse_slice_values where
    ensures result == 4
    aborts_if false
  verify reverse_slice_values

  fun trim_reverse_values() -> u64 := do
    let mut values := vector<u64>[1, 2, 3, 4]
    let values_ref := &mut values
    assert(2 <= (*values_ref).length, 131072)
    let evicted := core.prim.slice(*values_ref, 2, (*values_ref).length)
    *values_ref := core.prim.slice(*values_ref, 0, 2)
    let reversed := core.prim.reverseSliceVector(evicted, 0, evicted.length)
    let first_evicted := &reversed[0]
    return *first_evicted
  spec trim_reverse_values where
    ensures result == 4
    aborts_if false
  verify trim_reverse_values

  fun rotate_values() -> u64 := do
    let mut values := vector<u64>[1, 2, 3, 4]
    let values_ref := &mut values
    let length := (*values_ref).length
    *values_ref := core.prim.reverseSliceVector(*values_ref, 0, 1)
    *values_ref := core.prim.reverseSliceVector(*values_ref, 1, length)
    *values_ref := core.prim.reverseSliceVector(*values_ref, 0, length)
    let split := length - 1
    let updated := *values_ref
    let first := &updated[0]
    return *first + split
  spec rotate_values where
    ensures result == 5
    aborts_if false
  verify rotate_values

  fun rotate_slice_values() -> u64 := do
    let mut values := vector<u64>[0, 1, 2, 3, 4]
    let values_ref := &mut values
    *values_ref := core.prim.reverseSliceVector(*values_ref, 1, 2)
    *values_ref := core.prim.reverseSliceVector(*values_ref, 2, 5)
    *values_ref := core.prim.reverseSliceVector(*values_ref, 1, 5)
    let split : u64 := 1 + (5 - 2)
    let updated := *values_ref
    let first := &updated[1]
    return *first + split
  spec rotate_slice_values where
    ensures result == 6
    aborts_if false
  verify rotate_slice_values

  fun destroy_empty() -> Unit := do
    let values := vector<u64>[]
    if values.length != 0 then
      moveVectorError(3)
    core.prim.destroyEmptyVector(move(values))
  spec destroy_empty where
    ensures true
    aborts_if false
  verify destroy_empty

  fun contains_value() -> Bool := do
    let values := vector<u64>[1, 2, 3]
    let needle : u64 := 2
    let values_ref := &values
    let needle_ref := &needle
    return core.prim.containsVector(*values_ref, *needle_ref)
  spec contains_value where
    ensures result
    aborts_if false
  verify contains_value

  fun index_of_value() -> u64 := do
    let values := vector<u64>[4, 5, 6]
    let needle : u64 := 5
    let values_ref := &values
    let needle_ref := &needle
    let (found, index) := core.prim.indexOfVector(*values_ref, *needle_ref)
    return if found then index else 99
  spec index_of_value where
    ensures result == 1
    aborts_if false
  verify index_of_value

  fun insert_out_of_bounds() -> u64 := do
    let mut values := vector<u64>[1]
    let values_ref := &mut values
    assert(2 <= (*values_ref).length, 131072)
    *values_ref := core.prim.insertVector(*values_ref, 2, 9)
    return 0
  spec insert_out_of_bounds where
    ensures false
    aborts_if true with 131072
  verify insert_out_of_bounds

  fun remove_out_of_bounds() -> u64 := do
    let mut values := vector<u64>[1]
    let values_ref := &mut values
    assert(1 < (*values_ref).length, 131072)
    let (removed, rest) := core.prim.removeVector(*values_ref, 1)
    *values_ref := rest
    return removed

  fun read_out_of_bounds() -> u64 := do
    let values := vector<u64>[1]
    let value := &values[1]
    return *value
  spec read_out_of_bounds where
    ensures false
    aborts_if true
  verify read_out_of_bounds

  fun write_out_of_bounds() -> u64 := do
    let mut values := vector<u64>[1]
    let value := &mut values[1]
    *value := 9
    return values.length
  spec write_out_of_bounds where
    ensures false
    aborts_if true
  verify write_out_of_bounds

  fun conditional_writes(flag : Bool) -> u64 := do
    let mut value : u64 := 0
    let value_ref := &mut value
    if flag then
      *value_ref := 1
    *value_ref := 2
    return *value_ref
  spec conditional_writes where
    ensures result == 2
    aborts_if false
  verify conditional_writes

  fun write_then_read_owner() -> u64 := do
    let mut value : u64 := 1
    let value_ref := &mut value
    *value_ref := 2
    return value
  spec write_then_read_owner where
    ensures result == 2
    aborts_if false
  verify write_then_read_owner

#leaner_require_native 0x42::vector_operations::borrowed_length
#leaner_require_native 0x42::vector_operations::nested
#leaner_require_native 0x42::vector_operations::bool_round_trip
#leaner_require_native 0x42::vector_operations::read_out_of_bounds

open Lean Elab Command in
run_cmd do
  let env ← getEnv
  for function in ["empty_length", "pushed", "nested", "borrowed_length",
      "bool_round_trip", "mutate_and_read", "insert_middle", "swap_values",
      "pop_back", "pop_empty", "swap_remove_value", "append_values", "trim_values",
      "reverse_slice_values", "trim_reverse_values", "rotate_values", "rotate_slice_values",
      "destroy_empty", "contains_value", "index_of_value", "insert_out_of_bounds",
      "read_out_of_bounds", "write_out_of_bounds", "conditional_writes",
      "write_then_read_owner"] do
    let name := ((`«0x42».vector_operations).str function).str "verified"
    unless env.contains name do throwError "missing vector proof: {name}"
    if (← collectAxioms name).contains ``sorryAx then
      throwError "vector proof contains an admission: {name}"
  let some unit := LeanerLang.registeredUnit? env `«0x42».vector_operations
    | throwError "missing vector source"
  unless unit.namespaces[0]!.functions.size == 33 do
    throwError "vector port must retain all 33 original functions"
  let .ok printed := LeanerLang.Print.render env unit
    | throwError "vector printing failed"
  let .ok formatted := LeanerLang.Print.formatSource env printed
    | throwError "vector reimport failed: {printed}"
  unless printed == formatted do
    throwError "vector print/reimport is not a fixed point: {printed}\n{formatted}"

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».vector_operations #[
    ⟨"empty_length", #[], .returned #[.integer 0], {}⟩,
    ⟨"singleton_value", #[], .returned #[.integer 7], {}⟩,
    ⟨"emptiness", #[.bool true], .returned #[.bool true], {}⟩,
    ⟨"emptiness", #[.bool false], .returned #[.bool false], {}⟩,
    ⟨"pushed", #[], .returned #[.integer 9], {}⟩,
    ⟨"set_edges", #[], .returned #[.integer 40], {}⟩,
    ⟨"nested", #[], .returned #[.integer 3], {}⟩,
    ⟨"borrowed_length", #[], .returned #[.integer 3], {}⟩,
    ⟨"bool_round_trip", #[.bool true], .returned #[.bool true], {}⟩,
    ⟨"bool_round_trip", #[.bool false], .returned #[.bool false], {}⟩,
    ⟨"mutate_and_read", #[], .returned #[.integer 27], {}⟩,
    ⟨"mutate_then_borrow_other", #[], .returned #[.integer 30], {}⟩,
    ⟨"freeze_element", #[], .returned #[.integer 55], {}⟩,
    ⟨"insert_middle", #[], .returned #[.integer 20], {}⟩,
    ⟨"insert_edges", #[], .returned #[.integer 43], {}⟩,
    ⟨"remove_middle", #[], .returned #[.integer 52], {}⟩,
    ⟨"swap_values", #[], .returned #[.integer 31], {}⟩,
    ⟨"pop_back", #[], .returned #[.integer 30], {}⟩,
    ⟨"swap_remove_value", #[], .returned #[.integer 42], {}⟩,
    ⟨"append_values", #[], .returned #[.integer 8], {}⟩,
    ⟨"trim_values", #[], .returned #[.integer 7], {}⟩,
    ⟨"reverse_slice_values", #[], .returned #[.integer 4], {}⟩,
    ⟨"trim_reverse_values", #[], .returned #[.integer 4], {}⟩,
    ⟨"rotate_values", #[], .returned #[.integer 5], {}⟩,
    ⟨"rotate_slice_values", #[], .returned #[.integer 6], {}⟩,
    ⟨"destroy_empty", #[], .returned #[], {}⟩,
    ⟨"contains_value", #[], .returned #[.bool true], {}⟩,
    ⟨"index_of_value", #[], .returned #[.integer 1], {}⟩,
    ⟨"insert_out_of_bounds", #[], .threw .abort #[.integer 131072], {}⟩,
    ⟨"remove_out_of_bounds", #[], .threw .abort #[.integer 131072], {}⟩,
    ⟨"conditional_writes", #[.bool true], .returned #[.integer 2], {}⟩,
    ⟨"conditional_writes", #[.bool false], .returned #[.integer 2], {}⟩,
    ⟨"write_then_read_owner", #[], .returned #[.integer 2], {}⟩]
  let bounds : ThrowKind := .profile { profile := .move, tag := "runtime.vector_error" }
  assertRuns `«0x42».vector_operations #[
    ⟨"pop_empty", #[], .threw bounds #[.integer 2], {}⟩,
    ⟨"read_out_of_bounds", #[], .threw bounds #[.integer 1], {}⟩,
    ⟨"write_out_of_bounds", #[], .threw bounds #[.integer 1], {}⟩]
