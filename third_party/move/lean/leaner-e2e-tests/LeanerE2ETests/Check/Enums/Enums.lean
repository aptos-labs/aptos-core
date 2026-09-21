-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

-- Enum construction, exhaustive matching, wildcard
-- payloads, variant tests, and a mixed enum/Boolean match. The last case
-- spells out the Boolean branch and enum decision tree explicitly.
leaner module 0x42::language_enums where
  enum Action has Copy, Drop, Store where
    | Idle
    | Transfer (amount : u64)
    | Split (left : u64, right : u64)

  fun make_transfer(amount : u64) -> Action := new Action::Transfer { amount }
  spec make_transfer where
    ensures result == new Action::Transfer { amount }

  fun total(action : Action) -> u64 :=
    match action with
      | Action::Idle {} => 0
      | Action::Transfer { amount := amount } => amount
      | Action::Split { left := left, right := right } => left + right
  spec total where
    ensures result == match action with
      | Action::Idle {} => 0
      | Action::Transfer { amount := amount } => amount
      | Action::Split { left := left, right := right } => left + right

  fun classify(action : Action) -> u64 :=
    match action with
      | Action::Idle {} => 0
      | Action::Transfer { amount := _ } => 1
      | Action::Split { left := _, right := _ } => 2
  spec classify where
    ensures result == match action with
      | Action::Idle {} => 0
      | Action::Transfer { amount := _ } => 1
      | Action::Split { left := _, right := _ } => 2

  fun is_transfer(action : Action) -> Bool := action is Transfer
  spec is_transfer where
    ensures result == match action with
      | Action::Transfer { amount := _ } => true
      | _ => false

  fun mixed_match(action : Action, flag : Bool) -> u64 :=
    if flag then
      match action with
        | Action::Idle {} => 1
        | Action::Transfer { amount := amount } => amount
        | Action::Split { left := left, right := right } => left + right
    else 0
  spec mixed_match where
    ensures result == if flag then
      match action with
        | Action::Idle {} => 1
        | Action::Transfer { amount := amount } => amount
        | Action::Split { left := left, right := right } => left + right
      else 0
    aborts_if match action with
      | Action::Split { left := left, right := right } => flag && left + right > MAX_U64
      | _ => false

-- Run the functions on concrete inputs in the interpreter and compare the
-- outcomes for each variant.
open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let action (variant : String) (fields : Array RuntimeValue) : RuntimeValue :=
    .nominal ⟨⟨0⟩, 0⟩ (some variant) fields
  assertRuns `«0x42».language_enums #[
    ⟨"make_transfer", #[.integer 7], .returned #[action "Transfer" #[.integer 7]], {}⟩,
    ⟨"total", #[action "Idle" #[]], .returned #[.integer 0], {}⟩,
    ⟨"total", #[action "Transfer" #[.integer 9]], .returned #[.integer 9], {}⟩,
    ⟨"total", #[action "Split" #[.integer 4, .integer 5]], .returned #[.integer 9], {}⟩,
    ⟨"classify", #[action "Split" #[.integer 4, .integer 5]], .returned #[.integer 2], {}⟩,
    ⟨"is_transfer", #[action "Transfer" #[.integer 8]], .returned #[.bool true], {}⟩,
    ⟨"mixed_match", #[action "Transfer" #[.integer 8], .bool true], .returned #[.integer 8], {}⟩,
    ⟨"mixed_match", #[action "Split" #[.integer 3, .integer 4], .bool true], .returned #[.integer 7], {}⟩,
    ⟨"mixed_match", #[action "Idle" #[], .bool false], .returned #[.integer 0], {}⟩]

