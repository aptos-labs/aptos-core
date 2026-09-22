-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_aggregates where
  enum Atom has Copy, Drop, Store where
    | None
    | Number (value : u8)

  struct Pair has Copy, Drop, Store where
    first : Atom
    second : Atom

  fun number(value : u8) -> Atom := new Atom::Number { value }
  spec number where
    ensures result == new Atom::Number { value }
    aborts_if false
  verify number

  fun empty() -> Atom := new Atom::None {}
  spec empty where
    ensures result == new Atom::None {}
    aborts_if false
  verify empty

  fun nested(value : u8) -> Pair := new Pair {
    first := new Atom::Number { value }, second := new Atom::None {}
  }
  spec nested where
    ensures result == new Pair {
      first := new Atom::Number { value }, second := new Atom::None {}
    }
    aborts_if false
  verify nested

  fun local(value : u8) -> Atom := do
    let atom := new Atom::Number { value }
    return atom
  spec local where
    ensures result == new Atom::Number { value }
    aborts_if false
  verify local

  fun choose(flag : Bool, left : Atom, right : Atom) -> Atom :=
    if flag then left else right
  spec choose where
    ensures result == if flag then left else right
    aborts_if false
  verify choose

  fun constructed_branch(flag : Bool, value : u8) -> Atom :=
    if flag then new Atom::Number { value } else new Atom::None {}
  spec constructed_branch where
    ensures result == if flag then new Atom::Number { value } else new Atom::None {}
    aborts_if false
  verify constructed_branch

  fun is_number(atom : Atom) -> Bool := atom is Number
  spec is_number where
    ensures result == match atom with
      | Atom::Number { value := _ } => true
      | _ => false
    aborts_if false
  verify is_number

  fun classify(atom : Atom) -> u8 :=
    match atom with
      | Atom::None {} => 0
      | Atom::Number { value := _ } => 1
  spec classify where
    ensures result == match atom with
      | Atom::None {} => 0
      | Atom::Number { value := _ } => 1
    aborts_if false
  verify classify

  fun local_test(value : u8) -> Bool := do
    let atom := new Atom::Number { value }
    return atom is Number
  spec local_test where
    ensures result
    aborts_if false
  verify local_test

  fun bad(value : u8) -> Atom := new Atom::Number { value }
  spec bad where
    ensures result == new Atom::None {}
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::native_aggregates::bad

#leaner_require_native 0x42::native_aggregates::number
#leaner_require_native 0x42::native_aggregates::empty
#leaner_require_native 0x42::native_aggregates::nested
#leaner_require_native 0x42::native_aggregates::local
#leaner_require_native 0x42::native_aggregates::choose
#leaner_require_native 0x42::native_aggregates::constructed_branch
#leaner_require_native 0x42::native_aggregates::is_number
#leaner_require_native 0x42::native_aggregates::classify
#leaner_require_native 0x42::native_aggregates::local_test

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["number", "empty", "nested", "local", "choose", "constructed_branch",
      "is_number", "classify", "local_test"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_aggregates::{function} ")
    unless measured.size == 2 do throwError "missing native aggregate cost stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    logInfo m!"{function}: {total} heartbeats (all generated stages)"
    unless total ≤ 50000000 do throwError "native aggregate {function} exceeds aggregate 50M budget"
  for suffix in [`computation, `nativeSummary, `computationVerified, `computationRepresents] do
    if (← getEnv).contains (`«0x42».native_aggregates.bad ++ suffix) then
      throwError "rejected aggregate leaked {suffix}"

open LeanerIR «0x42».native_aggregates in
example (state : RuntimeState) :
    (choose.computation ⟨true, .Number ⟨7, by decide⟩, .None⟩).ok
      state (.Number ⟨7, by decide⟩) state := by
  exact ⟨rfl, rfl⟩

open LeanerIR «0x42».native_aggregates in
example (state : RuntimeState) :
    (nested.computation ⟨⟨8, by decide⟩⟩).ok
      state ⟨.Number ⟨8, by decide⟩, .None⟩ state := by
  exact ⟨rfl, rfl⟩

open LeanerIR «0x42».native_aggregates in
example (state : RuntimeState) :
    (is_number.computation ⟨.None⟩).ok state false state := by
  exact ⟨rfl, rfl⟩

open LeanerIR «0x42».native_aggregates in
example (state : RuntimeState) :
    (classify.computation ⟨.Number ⟨255, by decide⟩⟩).ok state ⟨1, by decide⟩ state := by
  exact ⟨rfl, rfl⟩
