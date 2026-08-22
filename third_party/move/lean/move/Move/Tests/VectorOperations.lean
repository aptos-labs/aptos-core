-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import MoveModel.Tests.Common

/-! Broader vector operation and boundary coverage. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

module VectorOperations where

  /-! ## Functions -/

  fun empty_length : U64 :=
    (Move.Vector.empty : Vector U64).length

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
    aborts_if True with Semantics.Resource.executionFailure

  fun write_out_of_bounds : Action U64 := do
    let values : Vector U64 := vector![1]
    let value ← &mut values[1]
    value := 9
    pure values.length

  spec write_out_of_bounds where
    ensures False;
    aborts_if True with Semantics.Resource.executionFailure

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

  verify read_out_of_bounds

  verify write_out_of_bounds

  verify conditional_writes

  verify write_then_read_owner

  /-! ## Tests -/

  def compiled : MModule := module% "VectorOperationsTest"

  private def run := Tests.run compiled

  #test run "empty_length" [] [] = Tests.okU64 0
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
  #test run "insert_out_of_bounds" [] [] = Tests.aborted 0x20000
  #test run "remove_out_of_bounds" [] [] = Tests.aborted 0x20000
  #test run "read_out_of_bounds" [] [] = Tests.aborted 0
  #test run "write_out_of_bounds" [] [] = Tests.aborted 0
  #test run "conditional_writes" [] [.bool true] = Tests.okU64 2
  #test run "conditional_writes" [] [.bool false] = Tests.okU64 2
  #test run "write_then_read_owner" [] [] = Tests.okU64 2

end Tests.MovePrograms
