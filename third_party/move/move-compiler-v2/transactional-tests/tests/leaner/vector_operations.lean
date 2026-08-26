--# publish

import Move

open scoped Move

module LeanerVectorOperations where

  /-! ## Functions -/

  fun empty_length : U64 :=
    Move.Vector.length (Move.Vector.empty : Move.Vector U64)

  fun pushed : U64 :=
    Move.Vector.get (Move.Vector.push vector![3, 4] 9) 2

  fun set_edges : U64 :=
    let values := Move.Vector.set vector![1, 2, 3] 0 10
    let values := Move.Vector.set values 2 30
    Move.Vector.get values 0 + Move.Vector.get values 2

  fun nested : U64 :=
    let values : Move.Vector (Move.Vector U64) :=
      vector![vector![1, 2], vector![3, 4]]
    Move.Vector.get (Move.Vector.get values 1) 0

  fun bool_round_trip (value : Bool) : Bool :=
    Move.Vector.get vector![value] 0

  fun mutate_and_read : Action U64 := do
    let values : Move.Vector U64 := vector![10, 20, 30]
    let middle ← &mut values[1]
    middle := *middle + 7
    (*middle)

  fun mutate_then_borrow_other : Action U64 := do
    let values : Move.Vector U64 := vector![10, 20, 30]
    let first ← &mut values[0]
    first := 99
    let last ← &values[2]
    (*last)

  fun freeze_element : Action U64 := do
    let values : Move.Vector U64 := vector![10, 20, 30]
    let middle ← &mut values[1]
    middle := 55
    let immutable ← freeze middle
    (*immutable)

  fun insert_middle : Action U64 := do
    let values : Move.Vector U64 := vector![10, 30]
    let valuesRef ← &mut values
    Move.Vector.insert valuesRef 1 20
    let updated ← *valuesRef
    let middle ← &updated[1]
    (*middle)

  fun insert_edges : Action U64 := do
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

  fun remove_middle : Action U64 := do
    let values : Move.Vector U64 := vector![10, 20, 30]
    let valuesRef ← &mut values
    let removed ← Move.Vector.remove valuesRef 1
    let updated ← *valuesRef
    let shifted ← &updated[1]
    let shiftedValue ← *shifted
    pure (removed + shiftedValue + Move.Vector.length updated)

  fun get_out_of_bounds : U64 := Move.Vector.get vector![1] 1

  fun set_out_of_bounds : U64 :=
    Move.Vector.length
      (Move.Vector.set (vector![1] : Move.Vector U64) 1 9)

  fun borrow_out_of_bounds : Action U64 := do
    let values : Move.Vector U64 := vector![1]
    let value ← &values[1]
    (*value)

  fun insert_out_of_bounds : Action U64 := do
    let values : Move.Vector U64 := vector![1]
    let valuesRef ← &mut values
    Move.Vector.insert valuesRef 2 9
    pure 0

  fun remove_out_of_bounds : Action U64 := do
    let values : Move.Vector U64 := vector![1]
    let valuesRef ← &mut values
    Move.Vector.remove valuesRef 1

/-! ## Tests -/

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
