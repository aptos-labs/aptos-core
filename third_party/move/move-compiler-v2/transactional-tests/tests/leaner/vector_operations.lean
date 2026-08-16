--# publish

import Move

open scoped Move

move_module LeanerVectorOperations where

  fun emptyLength : U64 :=
    Move.Vector.length (Move.Vector.empty : Move.Vector U64)

  fun pushed : U64 :=
    Move.Vector.get (Move.Vector.push vector![3, 4] 9) 2

  fun setEdges : U64 :=
    let values := Move.Vector.set vector![1, 2, 3] 0 10
    let values := Move.Vector.set values 2 30
    Move.Vector.get values 0 + Move.Vector.get values 2

  fun nested : U64 :=
    let values : Move.Vector (Move.Vector U64) :=
      vector![vector![1, 2], vector![3, 4]]
    Move.Vector.get (Move.Vector.get values 1) 0

  fun boolRoundTrip (value : Bool) : Bool :=
    Move.Vector.get vector![value] 0

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
    Move.Vector.insert valuesRef 1 20
    let updated ← *valuesRef
    let middle ← &updated[1]
    (*middle)

  fun insertEdges : Action U64 := do
    let values : Move.Vector U64 := vector![20]
    let valuesRef ← &mut values
    Move.Vector.insert valuesRef 0 10
    Move.Vector.insert valuesRef 2 30
    let updated ← *valuesRef
    let first ← &updated[0]
    let left ← *first
    let last ← &updated[2]
    let right ← *last
    pure (left + right + Move.Vector.length updated)

  fun removeMiddle : Action U64 := do
    let values : Move.Vector U64 := vector![10, 20, 30]
    let valuesRef ← &mut values
    let removed ← Move.Vector.remove valuesRef 1
    let updated ← *valuesRef
    let shifted ← &updated[1]
    let shiftedValue ← *shifted
    pure (removed + shiftedValue + Move.Vector.length updated)

  fun getOutOfBounds : U64 := Move.Vector.get vector![1] 1

  fun setOutOfBounds : U64 :=
    Move.Vector.length
      (Move.Vector.set (vector![1] : Move.Vector U64) 1 9)

  fun borrowOutOfBounds : Action U64 := do
    let values : Move.Vector U64 := vector![1]
    let value ← &values[1]
    (*value)

  fun insertOutOfBounds : Action U64 := do
    let values : Move.Vector U64 := vector![1]
    let valuesRef ← &mut values
    Move.Vector.insert valuesRef 2 9
    pure 0

  fun removeOutOfBounds : Action U64 := do
    let values : Move.Vector U64 := vector![1]
    let valuesRef ← &mut values
    Move.Vector.remove valuesRef 1

--# run 0x0::LeanerVectorOperations::emptyLength

--# run 0x0::LeanerVectorOperations::pushed

--# run 0x0::LeanerVectorOperations::setEdges

--# run 0x0::LeanerVectorOperations::nested

--# run 0x0::LeanerVectorOperations::boolRoundTrip --args true

--# run 0x0::LeanerVectorOperations::boolRoundTrip --args false

--# run 0x0::LeanerVectorOperations::mutateAndRead

--# run 0x0::LeanerVectorOperations::mutateThenBorrowOther

--# run 0x0::LeanerVectorOperations::freezeElement

--# run 0x0::LeanerVectorOperations::insertMiddle

--# run 0x0::LeanerVectorOperations::insertEdges

--# run 0x0::LeanerVectorOperations::removeMiddle

--# run 0x0::LeanerVectorOperations::getOutOfBounds

--# run 0x0::LeanerVectorOperations::setOutOfBounds

--# run 0x0::LeanerVectorOperations::borrowOutOfBounds

--# run 0x0::LeanerVectorOperations::insertOutOfBounds

--# run 0x0::LeanerVectorOperations::removeOutOfBounds
