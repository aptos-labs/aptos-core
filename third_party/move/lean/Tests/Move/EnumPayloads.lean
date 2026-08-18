-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

/-! Enum payload types, duplicate field names, wildcards, and calls. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module EnumPayloads where

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

  spec choose (right : Bool) (value : U64) where
    ensures
      result = if right then .right value else .left value

  fun score (choice : Choice) : U64 :=
    match choice with
    | .left value => value + 1
    | .right value => value + 2

  spec score (choice : Choice) where
    ensures
      result =
        match choice with
        | .left value => value + 1
        | .right value => value + 2

  fun chooseAndScore (right : Bool) (value : U64) : U64 :=
    score (choose right value)

  spec chooseAndScore (right : Bool) (value : U64) where
    ensures result = if right then value + 2 else value + 1

  fun isRight (choice : Choice) : U64 :=
    match choice with
    | .right _ => 1
    | _ => 0

  spec isRight (choice : Choice) where
    ensures
      result =
        match choice with
        | .right _ => 1
        | _ => 0

  fun batchLength (batch : Batch) : U64 :=
    match batch with
    | .empty => 0
    | .items values => Move.Vector.length values

  spec batchLength (batch : Batch) where
    ensures
      result =
        match batch with
        | .empty => 0
        | .items values => Move.Vector.length values

  fun populatedBatch : U64 :=
    batchLength (.items vector![4, 5, 6, 7])

  spec populatedBatch where
    ensures result = 4

  fun emptyBatch : U64 := batchLength .empty

  spec emptyBatch where
    ensures result = 0

  fun positionalTotal (value : Positional) : U64 :=
    match value with
    | .pair left right => left + right

  spec positionalTotal (value : Positional) where
    ensures
      match value with
      | .pair left right => result = left + right

  fun wrappedValue (value : Wrapper) : U64 :=
    match value with
    | .wrap inner => inner

  spec wrappedValue (value : Wrapper) where
    ensures
      match value with
      | .wrap inner => result = inner

  fun makePositional (left right : U64) : U64 :=
    positionalTotal (.pair left right)

  spec makePositional (left : U64) (right : U64) where
    ensures result = left + right

  fun makeWrapper (value : U64) : U64 :=
    wrappedValue (.wrap value)

  spec makeWrapper (value : U64) where
    ensures result = value

  -- Automatic source specifications for effectful functions do not yet model
  -- calls to pure Move helpers such as `score`, so these remain unspecced.
  fun vectorOfEnums : Action U64 := do
    let values : Move.Vector Choice := vector![.left 4, .right 5]
    let selected ← &values[1]
    let choice ← *selected
    pure (score choice)

  fun replaceEnumElement : Action U64 := do
    let values : Move.Vector Choice := vector![.left 1]
    let selected ← &mut values[0]
    selected := .right 7
    let choice ← *selected
    pure (score choice)

  /-! ## Proofs -/

  verify choose
  verify score
  verify chooseAndScore
  verify isRight
  verify batchLength

  @[grind .] private theorem fourElementVectorLength (a b c d : U64) :
      Move.Vector.length vector![a, b, c, d] = 4 := by
    apply U64.ext
    rfl

  verify populatedBatch
  verify emptyBatch
  verify positionalTotal
  verify wrappedValue
  verify makePositional
  verify makeWrapper

  /-! ## Tests -/

  def compiled : MModule := move_module% "EnumPayloadsTest"

  private def run := Tests.run compiled

  #test run "chooseAndScore" [] [.bool false, .u64 10] = Tests.okU64 11
  #test run "chooseAndScore" [] [.bool true, .u64 10] = Tests.okU64 12
  #test run "isRight" [] [.variant 0 [.u64 10]] = Tests.okU64 0
  #test run "isRight" [] [.variant 1 [.u64 10]] = Tests.okU64 1
  #test run "populatedBatch" [] [] = Tests.okU64 4
  #test run "emptyBatch" [] [] = Tests.okU64 0
  #test run "makePositional" [] [.u64 8, .u64 9] = Tests.okU64 17
  #test run "makeWrapper" [] [.u64 23] = Tests.okU64 23
  #test run "vectorOfEnums" [] [] = Tests.okU64 7
  #test run "replaceEnumElement" [] [] = Tests.okU64 9

end Tests.MovePrograms
