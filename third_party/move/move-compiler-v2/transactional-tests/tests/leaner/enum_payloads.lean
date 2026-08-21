--# publish

import Move

open scoped Move

move_module LeanerEnumPayloads where

  @[move_enum]
  inductive Choice where
    | left (value : U64)
    | right (value : U64)
    deriving Copy, Drop, Store

  @[move_enum]
  inductive Batch where
    | empty
    | items (values : Move.Vector U64)
    deriving Copy, Drop, Store

  @[move_enum]
  inductive Positional where
    | pair : U64 → U64 → Positional
    deriving Copy, Drop, Store

  @[move_enum]
  inductive Wrapper where
    | wrap (value : U64)
    deriving Copy, Drop, Store

  /-! ## Functions -/

  fun choose (right : Bool) (value : U64) : Choice :=
    if right then .right value else .left value

  fun score (choice : Choice) : U64 :=
    match choice with
    | .left value => value + 1
    | .right value => value + 2

  fun chooseAndScore (right : Bool) (value : U64) : U64 :=
    score (choose right value)

  fun isRight (choice : Choice) : U64 :=
    match choice with
    | .right _ => 1
    | _ => 0

  fun batchLength (batch : Batch) : U64 :=
    match batch with
    | .empty => 0
    | .items values => Move.Vector.length values

  fun populatedBatch : U64 :=
    batchLength (.items vector![4, 5, 6, 7])

  fun emptyBatch : U64 := batchLength .empty

  fun positionalTotal (value : Positional) : U64 :=
    match value with
    | .pair left right => left + right

  fun wrappedValue (value : Wrapper) : U64 :=
    match value with
    | .wrap inner => inner

  fun leftScore (value : U64) : U64 := score (.left value)

  fun rightScore (value : U64) : U64 := score (.right value)

  fun leftIsRight (value : U64) : U64 := isRight (.left value)

  fun rightIsRight (value : U64) : U64 := isRight (.right value)

  fun makePositional (left right : U64) : U64 :=
    positionalTotal (.pair left right)

  fun makeWrapper (value : U64) : U64 :=
    wrappedValue (.wrap value)

  fun vectorOfEnums : U64 :=
    let values : Move.Vector Choice := vector![.left 4, .right 5]
    score (Move.Vector.get values 1)

  fun replaceEnumElement : U64 :=
    let values : Move.Vector Choice := vector![.left 1]
    let values := Move.Vector.set values 0 (.right 7)
    score (Move.Vector.get values 0)

/-! ## Tests -/

--# run 0x0::LeanerEnumPayloads::chooseAndScore --args false 10u64

--# run 0x0::LeanerEnumPayloads::chooseAndScore --args true 10u64

--# run 0x0::LeanerEnumPayloads::leftScore --args 10u64

--# run 0x0::LeanerEnumPayloads::rightScore --args 10u64

--# run 0x0::LeanerEnumPayloads::leftIsRight --args 10u64

--# run 0x0::LeanerEnumPayloads::rightIsRight --args 10u64

--# run 0x0::LeanerEnumPayloads::populatedBatch

--# run 0x0::LeanerEnumPayloads::emptyBatch

--# run 0x0::LeanerEnumPayloads::makePositional --args 8u64 9u64

--# run 0x0::LeanerEnumPayloads::makeWrapper --args 23u64

--# run 0x0::LeanerEnumPayloads::vectorOfEnums

--# run 0x0::LeanerEnumPayloads::replaceEnumElement
