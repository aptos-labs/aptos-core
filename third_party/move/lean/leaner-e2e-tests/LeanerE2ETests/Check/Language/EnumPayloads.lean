-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

set_option leaner.route "native"

-- Port of v0 Language/EnumPayloads: duplicate payload names, vectors,
-- positional payloads, wrappers, nested calls, and indexed enum updates.
leaner module 0x42::enum_payloads where
  enum Choice has Copy, Drop, Store where
    | Left (value : u64)
    | Right (value : u64)
  enum Batch has Copy, Drop, Store where
    | Empty
    | Items (values : Vector<u64>)
  enum Positional has Copy, Drop, Store where
    | Pair (_0 : u64, _1 : u64)
  enum Wrapper has Copy, Drop, Store where
    | Wrap (value : u64)

  fun choose(right : Bool, value : u64) -> Choice :=
    if right then new Choice::Right { value } else new Choice::Left { value }
  spec choose where
    ensures result == if right then new Choice::Right { value } else new Choice::Left { value }
  verify choose

  fun score(choice : Choice) -> u64 :=
    match choice with
      | Choice::Left { value := value } => value + 1
      | Choice::Right { value := value } => value + 2
  spec score where
    ensures result == match choice with
      | Choice::Left { value := value } => value + 1
      | Choice::Right { value := value } => value + 2
  verify score

  fun choose_and_score(right : Bool, value : u64) -> u64 :=
    score(choose(right, value))
  spec choose_and_score where
    ensures result == if right then value + 2 else value + 1
  verify choose_and_score

  fun is_right(choice : Choice) -> u64 :=
    match choice with
      | Choice::Right { value := _ } => 1
      | _ => 0
  spec is_right where
    ensures result == match choice with
      | Choice::Right { value := _ } => 1
      | _ => 0
  verify is_right

  fun batch_length(batch : Batch) -> u64 :=
    match batch with
      | Batch::Empty {} => 0
      | Batch::Items { values := values } => values.length
  spec batch_length where
    ensures result == match batch with
      | Batch::Empty {} => 0
      | Batch::Items { values := values } => values.length
  verify batch_length

  fun populated_batch() -> u64 := batch_length(new Batch::Items { values := vector<u64>[4, 5, 6, 7] })
  spec populated_batch where
    ensures result == 4
  verify populated_batch

  fun empty_batch() -> u64 := batch_length(new Batch::Empty {})
  spec empty_batch where
    ensures result == 0
  verify empty_batch

  fun positional_total(value : Positional) -> u64 :=
    match value with
      | Positional::Pair { _0 := left, _1 := right } => left + right
  spec positional_total where
    ensures match value with
      | Positional::Pair { _0 := left, _1 := right } => result == left + right
  verify positional_total

  fun wrapped_value(value : Wrapper) -> u64 :=
    match value with
      | Wrapper::Wrap { value := inner } => inner
  spec wrapped_value where
    ensures match value with
      | Wrapper::Wrap { value := inner } => result == inner
  verify wrapped_value

  fun make_positional(left : u64, right : u64) -> u64 :=
    positional_total(new Positional::Pair { _0 := left, _1 := right })
  spec make_positional where
    ensures result == left + right
  verify make_positional

  fun make_wrapper(value : u64) -> u64 := wrapped_value(new Wrapper::Wrap { value })
  spec make_wrapper where
    ensures result == value
  verify make_wrapper

  -- These two functions were execution-only in v0 too.
  fun vector_of_enums() -> u64 := do
    let values := vector<Choice>[new Choice::Left { value := 4 }, new Choice::Right { value := 5 }]
    let selected := &values[1]
    let choice := *selected
    return score(choice)

  fun replace_enum_element() -> u64 := do
    let mut values := vector<Choice>[new Choice::Left { value := 1 }]
    let selected := &mut values[0]
    *selected := new Choice::Right { value := 7 }
    let choice := *selected
    return score(choice)

open Lean Elab Command in
run_cmd do
  for name in #[`«0x42».enum_payloads.Batch.outermostBorrows_erase,
      `«0x42».enum_payloads.Batch.plain_erase,
      `«0x42».enum_payloads.Batch.parameterLoanLocations_erase,
      `«0x42».enum_payloads.batch_length.verified,
      `«0x42».enum_payloads.choose_and_score.verified] do
    unless (← getEnv).contains name do
      throwError "missing generated vector/enum proof: {name}"
    if (← collectAxioms name).contains ``sorryAx then
      throwError "generated vector/enum proof contains an admission: {name}"

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let choice (variant : String) (value : Int) : RuntimeValue :=
    .nominal ⟨⟨0⟩, 0⟩ (some variant) #[.integer value]
  assertRuns `«0x42».enum_payloads #[
    ⟨"choose_and_score", #[.bool false, .integer 10], .returned #[.integer 11], {}⟩,
    ⟨"choose_and_score", #[.bool true, .integer 10], .returned #[.integer 12], {}⟩,
    ⟨"is_right", #[choice "Left" 10], .returned #[.integer 0], {}⟩,
    ⟨"is_right", #[choice "Right" 10], .returned #[.integer 1], {}⟩,
    ⟨"populated_batch", #[], .returned #[.integer 4], {}⟩,
    ⟨"empty_batch", #[], .returned #[.integer 0], {}⟩,
    ⟨"make_positional", #[.integer 8, .integer 9], .returned #[.integer 17], {}⟩,
    ⟨"make_wrapper", #[.integer 23], .returned #[.integer 23], {}⟩,
    ⟨"vector_of_enums", #[], .returned #[.integer 7], {}⟩,
    ⟨"replace_enum_element", #[], .returned #[.integer 9], {}⟩]
