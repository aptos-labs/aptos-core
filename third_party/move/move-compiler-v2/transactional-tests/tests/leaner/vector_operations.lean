-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerVectorOperations where
  fun empty_length() -> u64 := vector<u64>[].length

  fun pushed() -> u64 := do
    let values := core.prim.pushVector(vector<u64>[3, 4], 9)
    let value := &values[2]
    *value

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
    left + right

  fun nested() -> u64 := do
    let values := vector<Vector<u64> >[vector<u64>[1, 2], vector<u64>[3, 4]]
    let row_ref := &values[1]
    let row := *row_ref
    let value := &row[0]
    *value

  fun bool_round_trip(value : Bool) -> Bool := do
    let values := vector<Bool>[value]
    let result := &values[0]
    *result

  fun mutate_and_read() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    let middle := &mut values[1]
    *middle := *middle + 7
    *middle

  fun mutate_then_borrow_other() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    let first := &mut values[0]
    *first := 99
    let last := &values[2]
    *last

  fun freeze_element() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    let middle := &mut values[1]
    *middle := 55
    let immutable := core.ref.freezeExplicit(middle)
    *immutable

  fun insert_middle() -> u64 := do
    let mut values := vector<u64>[10, 30]
    let values_ref := &mut values
    *values_ref := core.prim.insertVector(*values_ref, 1, 20)
    let updated := *values_ref
    let middle := &updated[1]
    *middle

  fun insert_edges() -> u64 := do
    let mut values := vector<u64>[20]
    let values_ref := &mut values
    *values_ref := core.prim.insertVector(*values_ref, 0, 10)
    *values_ref := core.prim.insertVector(*values_ref, 2, 30)
    let updated := *values_ref
    let first := &updated[0]
    let left := *first
    let last := &updated[2]
    let right := *last
    left + right + updated.length

  fun remove_middle() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    let values_ref := &mut values
    let (removed, rest) := core.prim.removeVector(*values_ref, 1)
    *values_ref := rest
    let updated := *values_ref
    let shifted := &updated[1]
    removed + *shifted + updated.length

  fun get_out_of_bounds() -> u64 := do
    let values := vector<u64>[1]
    values[1]

  fun set_out_of_bounds() -> u64 := do
    let mut values := vector<u64>[1]
    let value := &mut values[1]
    *value := 9
    values.length

  fun borrow_out_of_bounds() -> u64 := do
    let values := vector<u64>[1]
    let value := &values[1]
    *value

  fun insert_out_of_bounds() -> u64 := do
    let mut values := vector<u64>[1]
    let values_ref := &mut values
    *values_ref := core.prim.insertVector(*values_ref, 2, 9)
    0

  fun remove_out_of_bounds() -> u64 := do
    let mut values := vector<u64>[1]
    let values_ref := &mut values
    let (removed, rest) := core.prim.removeVector(*values_ref, 1)
    *values_ref := rest
    removed

--# run 0x0::LeanerVectorOperations::empty_length

--# run 0x0::LeanerVectorOperations::pushed

--# run 0x0::LeanerVectorOperations::set_edges

--# run 0x0::LeanerVectorOperations::nested

--# run 0x0::LeanerVectorOperations::bool_round_trip --args true

--# run 0x0::LeanerVectorOperations::bool_round_trip --args false

--# run 0x0::LeanerVectorOperations::mutate_and_read

--# run 0x0::LeanerVectorOperations::mutate_then_borrow_other

--# run 0x0::LeanerVectorOperations::freeze_element

--# run 0x0::LeanerVectorOperations::insert_middle

--# run 0x0::LeanerVectorOperations::insert_edges

--# run 0x0::LeanerVectorOperations::remove_middle

--# run 0x0::LeanerVectorOperations::get_out_of_bounds

--# run 0x0::LeanerVectorOperations::set_out_of_bounds

--# run 0x0::LeanerVectorOperations::borrow_out_of_bounds

--# run 0x0::LeanerVectorOperations::insert_out_of_bounds

--# run 0x0::LeanerVectorOperations::remove_out_of_bounds
