-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

/-! Broader vector operation and boundary coverage. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module VectorOperations where

  fun emptyLength : U64 :=
    (Move.Vector.empty : Move.Vector U64).length

  fun pushed : Action U64 := do
    let values := vector![3, 4].push 9
    let value ← &values[2]
    (*value)

  fun setEdges : Action U64 := do
    let values : Move.Vector U64 := vector![1, 2, 3]
    let first ← &mut values[0]
    first := 10
    let last ← &mut values[2]
    last := 30
    let firstValue ← &values[0]
    let left ← *firstValue
    let lastValue ← &values[2]
    let right ← *lastValue
    pure (left + right)

  fun nested : Action U64 := do
    let values : Move.Vector (Move.Vector U64) :=
      vector![vector![1, 2], vector![3, 4]]
    let rowRef ← &values[1]
    let row ← *rowRef
    let value ← &row[0]
    (*value)

  fun boolRoundTrip (value : Bool) : Action Bool := do
    let values := vector![value]
    let result ← &values[0]
    (*result)

  fun mutateAndRead : Action U64 := do
    let values : Move.Vector U64 := vector![10, 20, 30]
    let middle ← &mut values[1]
    middle := *middle + 7
    (*middle)

  fun mutateThenBorrowOther : Action U64 := do
    let values : Move.Vector U64 := vector![10, 20, 30]
    let first ← &mut values[0]
    first := 99
    let last ← &values[2]
    (*last)

  fun freezeElement : Action U64 := do
    let values : Move.Vector U64 := vector![10, 20, 30]
    let middle ← &mut values[1]
    middle := 55
    let immutable ← freeze middle
    (*immutable)

  fun insertMiddle : Action U64 := do
    let values : Move.Vector U64 := vector![10, 30]
    let valuesRef ← &mut values
    valuesRef.insert 1 20
    let updated ← *valuesRef
    let middle ← &updated[1]
    (*middle)

  fun insertEdges : Action U64 := do
    let values : Move.Vector U64 := vector![20]
    let valuesRef ← &mut values
    valuesRef.insert 0 10
    valuesRef.insert 2 30
    let updated ← *valuesRef
    let first ← &updated[0]
    let left ← *first
    let last ← &updated[2]
    let right ← *last
    pure (left + right + updated.length)

  fun removeMiddle : Action U64 := do
    let values : Move.Vector U64 := vector![10, 20, 30]
    let valuesRef ← &mut values
    let removed ← valuesRef.remove 1
    let updated ← *valuesRef
    let shifted ← &updated[1]
    let shiftedValue ← *shifted
    pure (removed + shiftedValue + updated.length)

  fun insertOutOfBounds : Action U64 := do
    let values : Move.Vector U64 := vector![1]
    let valuesRef ← &mut values
    valuesRef.insert 2 9
    pure 0

  fun removeOutOfBounds : Action U64 := do
    let values : Move.Vector U64 := vector![1]
    let valuesRef ← &mut values
    valuesRef.remove 1

  fun readOutOfBounds : Action U64 := do
    let values : Move.Vector U64 := vector![1]
    let value ← &values[1]
    (*value)

  fun writeOutOfBounds : Action U64 := do
    let values : Move.Vector U64 := vector![1]
    let value ← &mut values[1]
    value := 9
    pure values.length

  /-- A normally completing `then` branch must continue with the statements
  following the conditional in the generated source specification. -/
  fun conditionalWrites (flag : Bool) : Action U64 := do
    let value : U64 := 0
    let valueRef ← &mut value
    if flag then
      valueRef := 1
    valueRef := 2
    (*valueRef)

  spec conditionalWrites (flag : Bool) where
    requires True;
    ensures result = 2;
    aborts_if False

  verify conditionalWrites

  /-- Reading a local after the last use of its mutable reference observes the
  value reconciled from that reference. -/
  fun writeThenReadOwner : Action U64 := do
    let value : U64 := 1
    let valueRef ← &mut value
    valueRef := 2
    pure value

  spec writeThenReadOwner where
    requires True;
    ensures result = 2;
    aborts_if False

  verify writeThenReadOwner

  def compiled : MModule := move_module% "VectorOperationsTest"

  private def run := Tests.run compiled

  #test run "emptyLength" [] [] = Tests.okU64 0
  #test run "pushed" [] [] = Tests.okU64 9
  #test run "setEdges" [] [] = Tests.okU64 40
  #test run "nested" [] [] = Tests.okU64 3
  #test run "boolRoundTrip" [] [.bool true] = Tests.okBool true
  #test run "boolRoundTrip" [] [.bool false] = Tests.okBool false
  #test run "mutateAndRead" [] [] = Tests.okU64 27
  #test run "mutateThenBorrowOther" [] [] = Tests.okU64 30
  #test run "freezeElement" [] [] = Tests.okU64 55
  #test run "insertMiddle" [] [] = Tests.okU64 20
  #test run "insertEdges" [] [] = Tests.okU64 43
  #test run "removeMiddle" [] [] = Tests.okU64 52
  #test run "insertOutOfBounds" [] [] = Tests.aborted 0x20000
  #test run "removeOutOfBounds" [] [] = Tests.aborted 0x20000
  #test run "readOutOfBounds" [] [] = Tests.aborted 0
  #test run "writeOutOfBounds" [] [] = Tests.aborted 0
  #test run "conditionalWrites" [] [.bool true] = Tests.okU64 2
  #test run "conditionalWrites" [] [.bool false] = Tests.okU64 2
  #test run "writeThenReadOwner" [] [] = Tests.okU64 2

end Tests.MovePrograms
