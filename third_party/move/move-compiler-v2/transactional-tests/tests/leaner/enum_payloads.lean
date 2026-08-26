--# publish

import Move

open scoped Move

module LeanerEnumPayloads where

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

  fun choose_and_score (right : Bool) (value : U64) : U64 :=
    score (choose right value)

  fun is_right (choice : Choice) : U64 :=
    match choice with
    | .right _ => 1
    | _ => 0

  fun batch_length (batch : Batch) : U64 :=
    match batch with
    | .empty => 0
    | .items values => Move.Vector.length values

  fun populated_batch : U64 :=
    batch_length (.items vector![4, 5, 6, 7])

  fun empty_batch : U64 := batch_length .empty

  fun positional_total (value : Positional) : U64 :=
    match value with
    | .pair left right => left + right

  fun wrapped_value (value : Wrapper) : U64 :=
    match value with
    | .wrap inner => inner

  fun left_score (value : U64) : U64 := score (.left value)

  fun right_score (value : U64) : U64 := score (.right value)

  fun left_is_right (value : U64) : U64 := is_right (.left value)

  fun right_is_right (value : U64) : U64 := is_right (.right value)

  fun make_positional (left right : U64) : U64 :=
    positional_total (.pair left right)

  fun make_wrapper (value : U64) : U64 :=
    wrapped_value (.wrap value)

  fun vector_of_enums : U64 :=
    let values : Move.Vector Choice := vector![.left 4, .right 5]
    score (Move.Vector.get values 1)

  fun replace_enum_element : U64 :=
    let values : Move.Vector Choice := vector![.left 1]
    let values := Move.Vector.set values 0 (.right 7)
    score (Move.Vector.get values 0)

/-! ## Tests -/

--# run 0x0::LeanerEnumPayloads::choose_and_score --args false 10u64

--# run 0x0::LeanerEnumPayloads::choose_and_score --args true 10u64

--# run 0x0::LeanerEnumPayloads::left_score --args 10u64

--# run 0x0::LeanerEnumPayloads::right_score --args 10u64

--# run 0x0::LeanerEnumPayloads::left_is_right --args 10u64

--# run 0x0::LeanerEnumPayloads::right_is_right --args 10u64

--# run 0x0::LeanerEnumPayloads::populated_batch

--# run 0x0::LeanerEnumPayloads::empty_batch

--# run 0x0::LeanerEnumPayloads::make_positional --args 8u64 9u64

--# run 0x0::LeanerEnumPayloads::make_wrapper --args 23u64

--# run 0x0::LeanerEnumPayloads::vector_of_enums

--# run 0x0::LeanerEnumPayloads::replace_enum_element
