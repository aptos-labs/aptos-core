-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common

/-! Enum payload types, duplicate field names, wildcards, and calls. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module EnumPayloads where

  enum Choice has Copy, Drop, Store where
    | Left (value : U64)
    | Right (value : U64)

  enum Batch has Copy, Drop, Store where
    | Empty
    | Items (values : Vector U64)

  enum Positional has Copy, Drop, Store where
    | Pair : U64 → U64 → Positional

  enum Wrapper has Copy, Drop, Store where
    | Wrap (value : U64)

  /-! ## Functions -/

  fun choose (right : Bool) (value : U64) : Choice :=
    if right then .Right value else .Left value

  spec choose (right : Bool) (value : U64) where
    ensures
      result = if right then .Right value else .Left value

  fun score (choice : Choice) : U64 :=
    match choice with
    | .Left value => value + 1
    | .Right value => value + 2

  spec score (choice : Choice) where
    ensures
      result =
        match choice with
        | .Left value => value + 1
        | .Right value => value + 2

  fun choose_and_score (right : Bool) (value : U64) : U64 :=
    score (choose right value)

  spec choose_and_score (right : Bool) (value : U64) where
    ensures result = if right then value + 2 else value + 1

  fun is_right (choice : Choice) : U64 :=
    match choice with
    | .Right _ => 1
    | _ => 0

  spec is_right (choice : Choice) where
    ensures
      result =
        match choice with
        | .Right _ => 1
        | _ => 0

  fun batch_length (batch : Batch) : U64 :=
    match batch with
    | .Empty => 0
    | .Items values => Move.Vector.length values

  spec batch_length (batch : Batch) where
    ensures
      result =
        match batch with
        | .Empty => 0
        | .Items values => Move.Vector.length values

  fun populated_batch : U64 :=
    batch_length (.Items vector![4, 5, 6, 7])

  spec populated_batch where
    ensures result = 4

  fun empty_batch : U64 := batch_length .Empty

  spec empty_batch where
    ensures result = 0

  fun positional_total (value : Positional) : U64 :=
    match value with
    | .Pair left right => left + right

  spec positional_total (value : Positional) where
    ensures
      match value with
      | .Pair left right => result = left + right

  fun wrapped_value (value : Wrapper) : U64 :=
    match value with
    | .Wrap inner => inner

  spec wrapped_value (value : Wrapper) where
    ensures
      match value with
      | .Wrap inner => result = inner

  fun make_positional (left right : U64) : U64 :=
    positional_total (.Pair left right)

  spec make_positional (left : U64) (right : U64) where
    ensures result = left + right

  fun make_wrapper (value : U64) : U64 :=
    wrapped_value (.Wrap value)

  spec make_wrapper (value : U64) where
    ensures result = value

  -- Automatic source specifications for effectful functions do not yet model
  -- calls to pure Move helpers such as `score`, so these remain unspecced.
  fun vector_of_enums : Action U64 := do
    let values : Vector Choice := vector![.Left 4, .Right 5]
    let selected ← &values[1]
    let choice ← *selected
    pure (score choice)

  fun replace_enum_element : Action U64 := do
    let values : Vector Choice := vector![.Left 1]
    let selected ← &mut values[0]
    selected := .Right 7
    let choice ← *selected
    pure (score choice)

  /-! ## Proofs -/

  verify choose
  verify score
  verify choose_and_score
  verify is_right
  verify batch_length

  verify populated_batch
  verify empty_batch
  verify positional_total
  verify wrapped_value
  verify make_positional
  verify make_wrapper

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.EnumPayloads

  private def run := Tests.run compiled

  #test run "choose_and_score" [] [.bool false, .u64 10] = Tests.okU64 11
  #test run "choose_and_score" [] [.bool true, .u64 10] = Tests.okU64 12
  #test run "is_right" [] [.variant 0 [.u64 10]] = Tests.okU64 0
  #test run "is_right" [] [.variant 1 [.u64 10]] = Tests.okU64 1
  #test run "populated_batch" [] [] = Tests.okU64 4
  #test run "empty_batch" [] [] = Tests.okU64 0
  #test run "make_positional" [] [.u64 8, .u64 9] = Tests.okU64 17
  #test run "make_wrapper" [] [.u64 23] = Tests.okU64 23
  #test run "vector_of_enums" [] [] = Tests.okU64 7
  #test run "replace_enum_element" [] [] = Tests.okU64 9

end Tests.MovePrograms
