-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common

/-! Broader vector operation and boundary coverage. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module VectorOperations where

  /-! ## Functions -/

  fun empty_length : U64 :=
    (Move.Vector.empty : Vector U64).length

  fun singleton_value : Action U64 := do
    let values := Move.Vector.singleton (7 : U64)
    let value ← &values[0]
    (*value)

  fun emptiness (flag : Bool) : Bool :=
    if flag then (Move.Vector.empty : Vector U64).isEmpty
    else (Move.Vector.singleton (1 : U64)).isEmpty

  spec empty_length where
    ensures result = 0

  fun pushed : Action U64 := do
    let values := vector![3, 4].push 9
    let value ← &values[2]
    (*value)

  spec pushed where
    ensures result = 9;
    aborts_if False

  fun set_edges : Action U64 := do
    let values : Vector U64 := vector![1, 2, 3]
    let first ← &mut values[0]
    first := 10
    let last ← &mut values[2]
    last := 30
    let firstValue ← &values[0]
    let left ← *firstValue
    let lastValue ← &values[2]
    let right ← *lastValue
    pure (left + right)

  spec set_edges where
    ensures result = 40;
    aborts_if False

  fun nested : Action U64 := do
    let values : Vector (Vector U64) :=
      vector![vector![1, 2], vector![3, 4]]
    let rowRef ← &values[1]
    let row ← *rowRef
    let value ← &row[0]
    (*value)

  spec nested where
    ensures result = 3;
    aborts_if False

  /-- Native vector length observes a borrowed vector through an explicit
  reference read in the normalized IR. -/
  fun borrowed_length : Action U64 := do
    let values : Vector U64 := vector![1, 2, 3]
    let valuesRef ← &values
    pure valuesRef.length

  spec borrowed_length where
    ensures result = 3;
    aborts_if False

  fun bool_round_trip (value : Bool) : Action Bool := do
    let values := vector![value]
    let result ← &values[0]
    (*result)

  spec bool_round_trip (value : Bool) where
    ensures result = value;
    aborts_if False

  fun mutate_and_read : Action U64 := do
    let values : Vector U64 := vector![10, 20, 30]
    let middle ← &mut values[1]
    middle := *middle + 7
    (*middle)

  spec mutate_and_read where
    ensures result = 27;
    aborts_if False

  fun mutate_then_borrow_other : Action U64 := do
    let values : Vector U64 := vector![10, 20, 30]
    let first ← &mut values[0]
    first := 99
    let last ← &values[2]
    (*last)

  spec mutate_then_borrow_other where
    ensures result = 30;
    aborts_if False

  fun freeze_element : Action U64 := do
    let values : Vector U64 := vector![10, 20, 30]
    let middle ← &mut values[1]
    middle := 55
    let immutable ← freeze middle
    (*immutable)

  fun insert_middle : Action U64 := do
    let values : Vector U64 := vector![10, 30]
    let valuesRef ← &mut values
    Move.Vector.insert valuesRef 1 20
    let updated ← *valuesRef
    let middle ← &updated[1]
    (*middle)

  spec insert_middle where
    ensures result = 20;
    aborts_if False

  fun insert_edges : Action U64 := do
    let values : Vector U64 := vector![20]
    let valuesRef ← &mut values
    Move.Vector.insert valuesRef 0 10
    Move.Vector.insert valuesRef 2 30
    let updated ← *valuesRef
    let first ← &updated[0]
    let left ← *first
    let last ← &updated[2]
    let right ← *last
    pure (left + right + updated.length)

  spec insert_edges where
    ensures result = 43;
    aborts_if False

  fun remove_middle : Action U64 := do
    let values : Vector U64 := vector![10, 20, 30]
    let valuesRef ← &mut values
    let removed ← Move.Vector.remove valuesRef 1
    let updated ← *valuesRef
    let shifted ← &updated[1]
    let shiftedValue ← *shifted
    pure (removed + shiftedValue + updated.length)

  spec remove_middle where
    ensures result = 52;
    aborts_if False

  fun pop_back : Action U64 := do
    let values : Vector U64 := vector![10, 20, 30]
    let valuesRef ← &mut values
    let removed ← valuesRef.popBack
    pure removed

  spec pop_back where
    ensures result = 30;
    aborts_if False

  fun pop_empty : Action U64 := do
    let values : Vector U64 := vector![]
    let valuesRef ← &mut values
    valuesRef.popBack

  fun swap_values : Action U64 := do
    let values := vector![1, 2, 3]
    let valuesRef ← &mut values
    valuesRef.swap 0 2
    let updated ← *valuesRef
    let first ← &updated[0]
    let last ← &updated[2]
    let firstValue ← *first
    let lastValue ← *last
    pure (firstValue * 10 + lastValue)

  fun swap_remove_value : Action U64 := do
    let values := vector![10, 20, 30]
    let valuesRef ← &mut values
    let removed ← valuesRef.swapRemove 0
    let updated ← *valuesRef
    let first ← &updated[0]
    let firstValue ← *first
    pure (removed + firstValue + updated.length)

  fun append_values : Action U64 := do
    let values := vector![1, 2]
    let valuesRef ← &mut values
    valuesRef.append vector![3, 4]
    let updated ← *valuesRef
    let last ← &updated[3]
    let lastValue ← *last
    pure (lastValue + updated.length)

  fun reverse_slice_values : Action U64 := do
    let values := vector![1, 2, 3, 4]
    let valuesRef ← &mut values
    valuesRef.reverseSlice 1 4
    let updated ← *valuesRef
    let middle ← &updated[1]
    (*middle)

  fun trim_values : Action U64 := do
    let values := vector![1, 2, 3, 4]
    let valuesRef ← &mut values
    let evicted ← valuesRef.trim 2
    let updated ← *valuesRef
    let firstEvicted ← &evicted[0]
    let firstEvictedValue ← *firstEvicted
    pure (updated.length + evicted.length + firstEvictedValue)

  fun trim_reverse_values : Action U64 := do
    let values := vector![1, 2, 3, 4]
    let valuesRef ← &mut values
    let evicted ← valuesRef.trimReverse 2
    let firstEvicted ← &evicted[0]
    (*firstEvicted)

  fun rotate_values : Action U64 := do
    let values := vector![1, 2, 3, 4]
    let valuesRef ← &mut values
    let split ← valuesRef.rotate 1
    let updated ← *valuesRef
    let first ← &updated[0]
    let firstValue ← *first
    pure (split + firstValue)

  fun rotate_slice_values : Action U64 := do
    let values := vector![0, 1, 2, 3, 4]
    let valuesRef ← &mut values
    let split ← valuesRef.rotateSlice 1 2 5
    let updated ← *valuesRef
    let middle ← &updated[1]
    let middleValue ← *middle
    pure (split + middleValue)

  fun destroy_empty : Action Unit := do
    Move.Vector.destroyEmpty (Move.Vector.empty : Vector U64)

  fun contains_value : Action Bool := do
    let values := vector![1, 2, 3]
    let needle : U64 := 2
    let valuesRef ← &values
    let needleRef ← &needle
    pure (Move.Vector.contains valuesRef needleRef)

  fun index_of_value : Action U64 := do
    let values := vector![4, 5, 6]
    let needle : U64 := 5
    let valuesRef ← &values
    let needleRef ← &needle
    let (found, index) := Move.Vector.indexOf valuesRef needleRef
    if found then pure index else pure 99

  spec swap_values where
    ensures result = 31;
    aborts_if False

  spec swap_remove_value where
    ensures result = 42;
    aborts_if False

  spec append_values where
    ensures result = 8;
    aborts_if False

  spec reverse_slice_values where
    ensures result = 4;
    aborts_if False

  spec trim_values where
    ensures result = 7;
    aborts_if False

  spec trim_reverse_values where
    ensures result = 4;
    aborts_if False

  spec rotate_values where
    ensures result = 5;
    aborts_if False

  spec rotate_slice_values where
    ensures result = 6;
    aborts_if False

  spec destroy_empty where
    ensures True;
    aborts_if False

  spec contains_value where
    ensures result = true;
    aborts_if False

  spec index_of_value where
    ensures result = 1;
    aborts_if False

  spec pop_empty where
    ensures False;
    aborts_if True with Semantics.Vector.indexOutOfBounds

  fun insert_out_of_bounds : Action U64 := do
    let values : Vector U64 := vector![1]
    let valuesRef ← &mut values
    Move.Vector.insert valuesRef 2 9
    pure 0

  spec insert_out_of_bounds where
    ensures False;
    aborts_if True with Semantics.Vector.indexOutOfBounds

  fun remove_out_of_bounds : Action U64 := do
    let values : Vector U64 := vector![1]
    let valuesRef ← &mut values
    Move.Vector.remove valuesRef 1

  fun read_out_of_bounds : Action U64 := do
    let values : Vector U64 := vector![1]
    let value ← &values[1]
    (*value)

  spec read_out_of_bounds where
    ensures False;
    aborts_if True with Semantics.Vector.indexOutOfBounds

  fun write_out_of_bounds : Action U64 := do
    let values : Vector U64 := vector![1]
    let value ← &mut values[1]
    value := 9
    pure values.length

  spec write_out_of_bounds where
    ensures False;
    aborts_if True with Semantics.Vector.indexOutOfBounds

  /-- A normally completing `then` branch must continue with the statements
  following the conditional in the generated source specification. -/
  fun conditional_writes (flag : Bool) : Action U64 := do
    let value : U64 := 0
    let valueRef ← &mut value
    if flag then
      valueRef := 1
    valueRef := 2
    (*valueRef)

  spec conditional_writes (flag : Bool) where
    ensures result = 2;
    aborts_if False

  /-- Reading a local after the last use of its mutable reference observes the
  value reconciled from that reference. -/
  fun write_then_read_owner : Action U64 := do
    let value : U64 := 1
    let valueRef ← &mut value
    valueRef := 2
    pure value

  spec write_then_read_owner where
    ensures result = 2;
    aborts_if False

  /-! ## Proofs -/

  verify empty_length

  verify pushed

  verify nested

  verify borrowed_length by
    contract_intro
    rw [Move.Verify.wp_pure]
    exact ⟨Move.UInt.ext rfl, rfl⟩

  verify bool_round_trip

  verify mutate_and_read by
    contract_intro
    simp [wp_norm, Move.Semantics.Mutation.read, Move.Semantics.Mutation.write,
      Move.Vector.empty, Move.Vector.push, Move.Vector.toList, move_norm]

  verify insert_middle

  verify insert_out_of_bounds

  verify pop_back

  verify pop_empty

  verify swap_values by
    contract_intro
    simp [wp_norm, move_norm, Nat.reducePow,
      Move.Semantics.Mutation.read, Move.Semantics.Mutation.write,
      Move.Vector.empty, Move.Vector.push, Move.Vector.set, Move.Vector.ofList,
      Move.Vector.toList] <;> grind

  verify swap_remove_value by
    contract_intro
    simp [wp_norm, move_norm, Nat.reducePow,
      Move.Semantics.Mutation.read, Move.Semantics.Mutation.write,
      Move.Vector.empty, Move.Vector.push, Move.Vector.set, Move.Vector.ofList,
      Move.Vector.toList] <;> grind

  verify append_values by
    contract_intro
    simp [wp_norm, move_norm, Nat.reducePow,
      Move.Semantics.Mutation.read, Move.Semantics.Mutation.write,
      Move.Vector.empty, Move.Vector.push, Move.Vector.set, Move.Vector.ofList,
      Move.Vector.toList] <;> grind

  verify reverse_slice_values by
    contract_intro
    simp [wp_norm, move_norm, Nat.reducePow,
      Move.Semantics.Mutation.read, Move.Semantics.Mutation.write,
      Move.Semantics.Vector.rotateSliceValues, Move.Vector.empty,
      Move.Vector.push, Move.Vector.set, Move.Vector.ofList,
      Move.Vector.toList] <;> grind

  verify trim_values by
    contract_intro
    simp [wp_norm, move_norm, Nat.reducePow,
      Move.Semantics.Mutation.read, Move.Semantics.Mutation.write,
      Move.Vector.empty, Move.Vector.push, Move.Vector.set, Move.Vector.ofList,
      Move.Vector.toList] <;> grind

  verify trim_reverse_values by
    contract_intro
    simp [wp_norm, move_norm, Nat.reducePow,
      Move.Semantics.Mutation.read, Move.Semantics.Mutation.write,
      Move.Vector.empty, Move.Vector.push, Move.Vector.set, Move.Vector.ofList,
      Move.Vector.toList] <;> grind

  verify rotate_values by
    contract_intro
    simp [wp_norm, move_norm, Nat.reducePow,
      Move.Semantics.Mutation.read, Move.Semantics.Mutation.write,
      Move.Semantics.Vector.rotateSliceValues, Move.Vector.empty,
      Move.Vector.push, Move.Vector.set, Move.Vector.ofList,
      Move.Vector.toList] <;> grind

  verify rotate_slice_values by
    contract_intro
    simp [wp_norm, move_norm, Nat.reducePow,
      Move.Semantics.Mutation.read, Move.Semantics.Mutation.write,
      Move.Semantics.Vector.rotateSliceValues, Move.Vector.empty,
      Move.Vector.push, Move.Vector.set, Move.Vector.ofList,
      Move.Vector.toList] <;> grind

  verify destroy_empty

  verify contains_value by
    contract_intro
    simp [Move.Verify.Source.vectorContains, Move.Vector.empty, Move.Vector.push,
      Move.Vector.toList, wp_norm, move_norm]

  verify index_of_value by
    contract_intro
    simp [Move.Verify.Source.vectorIndexOf, Move.Verify.Source.vectorFindIndex,
      Move.Vector.empty,
      Move.Vector.push, Move.Vector.toList, wp_norm, move_norm]

  verify read_out_of_bounds

  verify write_out_of_bounds

  verify conditional_writes

  verify write_then_read_owner


  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.VectorOperations

  private def run := Tests.run compiled

  #test run "empty_length" [] [] = Tests.okU64 0
  #test run "singleton_value" [] [] = Tests.okU64 7
  #test run "emptiness" [] [.bool true] = Tests.okBool true
  #test run "emptiness" [] [.bool false] = Tests.okBool false
  #test run "pushed" [] [] = Tests.okU64 9
  #test run "set_edges" [] [] = Tests.okU64 40
  #test run "nested" [] [] = Tests.okU64 3
  #test run "borrowed_length" [] [] = Tests.okU64 3
  #test run "bool_round_trip" [] [.bool true] = Tests.okBool true
  #test run "bool_round_trip" [] [.bool false] = Tests.okBool false
  #test run "mutate_and_read" [] [] = Tests.okU64 27
  #test run "mutate_then_borrow_other" [] [] = Tests.okU64 30
  #test run "freeze_element" [] [] = Tests.okU64 55
  #test run "insert_middle" [] [] = Tests.okU64 20
  #test run "insert_edges" [] [] = Tests.okU64 43
  #test run "remove_middle" [] [] = Tests.okU64 52
  #test run "pop_back" [] [] = Tests.okU64 30
  #test run "pop_empty" [] [] = Tests.aborted 0x20000
  #test run "swap_values" [] [] = Tests.okU64 31
  #test run "swap_remove_value" [] [] = Tests.okU64 42
  #test run "append_values" [] [] = Tests.okU64 8
  #test run "reverse_slice_values" [] [] = Tests.okU64 4
  #test run "trim_values" [] [] = Tests.okU64 7
  #test run "trim_reverse_values" [] [] = Tests.okU64 4
  #test run "rotate_values" [] [] = Tests.okU64 5
  #test run "rotate_slice_values" [] [] = Tests.okU64 6
  #test run "destroy_empty" [] [] = Tests.okUnit
  #test run "contains_value" [] [] = Tests.okBool true
  #test run "index_of_value" [] [] = Tests.okU64 1
  #test run "insert_out_of_bounds" [] [] = Tests.aborted 0x20000
  #test run "remove_out_of_bounds" [] [] = Tests.aborted 0x20000
  #test run "read_out_of_bounds" [] [] = Tests.aborted 0x20000
  #test run "write_out_of_bounds" [] [] = Tests.aborted 0x20000
  #test run "conditional_writes" [] [.bool true] = Tests.okU64 2
  #test run "conditional_writes" [] [.bool false] = Tests.okU64 2
  #test run "write_then_read_owner" [] [] = Tests.okU64 2

end Tests.MovePrograms
