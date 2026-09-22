-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of v0 Language/EnumPatterns: nested enum patterns, wildcard
fallbacks, constructed nested values, and calls through the total matcher. -/

leaner module 0x42::enum_patterns where
  enum Atom has Copy, Drop, Store where
    | None
    | Number (value : u64)

  enum Envelope has Copy, Drop, Store where
    | Empty
    | One (value : Atom)
    | Two (left : Atom, right : Atom)

  fun nested_total(envelope : Envelope) -> u64 :=
    match envelope with
      | Envelope::Empty {} => 0
      | Envelope::One { value := atom } =>
          match atom with
            | Atom::None {} => 0
            | Atom::Number { value := number } => number
      | Envelope::Two { left := left_atom, right := right_atom } =>
          match left_atom with
            | Atom::None {} => 0
            | Atom::Number { value := left_number } =>
                match right_atom with
                  | Atom::None {} => 0
                  | Atom::Number { value := right_number } => left_number + right_number
  spec nested_total where
    pragma aborts_if_is_partial
    ensures result == match envelope with
      | Envelope::Empty {} => 0
      | Envelope::One { value := atom } =>
          match atom with
            | Atom::None {} => 0
            | Atom::Number { value := number } => number
      | Envelope::Two { left := left_atom, right := right_atom } =>
          match left_atom with
            | Atom::None {} => 0
            | Atom::Number { value := left_number } =>
                match right_atom with
                  | Atom::None {} => 0
                  | Atom::Number { value := right_number } => left_number + right_number

  fun one_number(value : u64) -> u64 :=
    nested_total(new Envelope::One { value := new Atom::Number { value } })
  spec one_number where
    ensures result == value

  fun one_none() -> u64 :=
    nested_total(new Envelope::One { value := new Atom::None {} })
  spec one_none where
    ensures result == 0

  fun two_numbers(left : u64, right : u64) -> u64 :=
    nested_total(new Envelope::Two {
      left := new Atom::Number { value := left },
      right := new Atom::Number { value := right }
    })
  spec two_numbers where
    pragma aborts_if_is_partial
    ensures result == left + right

  fun left_missing(right : u64) -> u64 :=
    nested_total(new Envelope::Two {
      left := new Atom::None {},
      right := new Atom::Number { value := right }
    })
  spec left_missing where
    ensures result == 0

  fun right_missing(left : u64) -> u64 :=
    nested_total(new Envelope::Two {
      left := new Atom::Number { value := left },
      right := new Atom::None {}
    })
  spec right_missing where
    ensures result == 0

set_option leaner.route "native" in
#leaner_verify 0x42::enum_patterns::nested_total
#leaner_require_native 0x42::enum_patterns::nested_total
set_option leaner.route "native" in
#leaner_verify 0x42::enum_patterns::one_number
#leaner_require_native 0x42::enum_patterns::one_number
set_option leaner.route "native" in
#leaner_verify 0x42::enum_patterns::one_none
#leaner_require_native 0x42::enum_patterns::one_none
set_option leaner.route "native" in
#leaner_verify 0x42::enum_patterns::two_numbers
#leaner_require_native 0x42::enum_patterns::two_numbers
set_option leaner.route "native" in
#leaner_verify 0x42::enum_patterns::left_missing
#leaner_require_native 0x42::enum_patterns::left_missing
set_option leaner.route "native" in
#leaner_verify 0x42::enum_patterns::right_missing
#leaner_require_native 0x42::enum_patterns::right_missing

open Lean Elab Command in
run_cmd do
  for function in ["nested_total", "one_number", "one_none", "two_numbers",
      "left_missing", "right_missing"] do
    let name := ((`«0x42».enum_patterns).str function).str "verified"
    unless (← getEnv).contains name do
      throwError "missing generated nested-enum proof: {name}"
    if (← collectAxioms name).contains ``sorryAx then
      throwError "generated nested-enum proof contains an admission: {name}"

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let atom (variant : String) (fields : Array RuntimeValue := #[]) : RuntimeValue :=
    .nominal ⟨⟨0⟩, 0⟩ (some variant) fields
  let envelope (variant : String) (fields : Array RuntimeValue := #[]) : RuntimeValue :=
    .nominal ⟨⟨0⟩, 1⟩ (some variant) fields
  assertRuns `«0x42».enum_patterns #[
    ⟨"one_number", #[.integer 7], .returned #[.integer 7], {}⟩,
    ⟨"one_none", #[], .returned #[.integer 0], {}⟩,
    ⟨"two_numbers", #[.integer 4, .integer 5], .returned #[.integer 9], {}⟩,
    ⟨"left_missing", #[.integer 5], .returned #[.integer 0], {}⟩,
    ⟨"right_missing", #[.integer 4], .returned #[.integer 0], {}⟩,
    ⟨"nested_total", #[envelope "Empty"], .returned #[.integer 0], {}⟩,
    ⟨"nested_total", #[envelope "One" #[atom "Number" #[.integer 8]]],
      .returned #[.integer 8], {}⟩,
    ⟨"nested_total", #[envelope "Two" #[atom "None", atom "Number" #[.integer 8]]],
      .returned #[.integer 0], {}⟩]

#leaner_require_native_all
