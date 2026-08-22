--# publish

import Move

open scoped Move

module LeanerVectors where

  /-! ## Functions -/

  fun length : U64 :=
    Move.Vector.length (vector![10, 20, 30] : Move.Vector U64)

  fun middle : U64 := Move.Vector.get vector![10, 20, 30] 1

  fun replaced : U64 :=
    Move.Vector.get (Move.Vector.set vector![10, 20, 30] 1 42) 1

  fun borrowed : Action U64 := do
    let values : Move.Vector U64 := vector![10, 20, 30]
    let value ← &values[1]
    (*value)

  fun borrowed_mut : Action U64 := do
    let values : Move.Vector U64 := vector![10, 20, 30]
    let value ← &mut values[1]
    value := 42
    (*value)

/-! ## Tests -/

--# run 0x0::LeanerVectors::length

--# run 0x0::LeanerVectors::middle

--# run 0x0::LeanerVectors::replaced

--# run 0x0::LeanerVectors::borrowed

--# run 0x0::LeanerVectors::borrowed_mut
