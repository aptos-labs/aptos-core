-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_owned_calls where
  enum Atom has Copy, Drop, Store where
    | None
    | Number (value : u8)

  fun read(atom : Atom) -> u8 := match atom with
    | Atom::None {} => 0
    | Atom::Number { value := number } => number
  spec read where
    ensures result == match atom with
      | Atom::None {} => 0
      | Atom::Number { value := number } => number
    aborts_if false
  verify read

  fun local_call(value : u8) -> u8 := do
    let number := read(new Atom::Number { value })
    return number + 1
  spec local_call where
    requires value < 255
    ensures result == value + 1
    aborts_if false
  verify local_call

  fun identity(atom : Atom) -> Atom := atom
  spec identity where
    ensures result == atom
    aborts_if false
  verify identity

  fun nominal_local(value : u8) -> u8 := do
    let atom := identity(new Atom::Number { value })
    return read(atom)
  spec nominal_local where
    ensures result == value
    aborts_if false
  verify nominal_local

  fun match_result(value : u8) -> u8 := do
    let atom := identity(new Atom::Number { value })
    return match atom with
      | Atom::None {} => 0
      | Atom::Number { value := number } => number
  spec match_result where
    ensures result == value
    aborts_if false
  verify match_result

  fun two_calls(value : u8) -> u8 := do
    let first := identity(new Atom::Number { value })
    let second := identity(first)
    return read(second)
  spec two_calls where
    ensures result == value
    aborts_if false
  verify two_calls

  fun four_calls(value : u8) -> u8 := do
    let first := identity(new Atom::Number { value })
    let second := identity(first)
    let third := identity(second)
    let fourth := identity(third)
    return read(fourth)
  spec four_calls where
    ensures result == value
    aborts_if false
  verify four_calls

  fun weak_identity(atom : Atom) -> Atom := atom
  spec weak_identity where
    ensures true
    aborts_if false
  verify weak_identity

  -- The implementation preserves this value, but its summary does not say so.
  fun hidden_result(value : u8) -> u8 := do
    let atom := weak_identity(new Atom::Number { value })
    return read(atom)
  spec hidden_result where
    ensures result == value
    aborts_if false

  fun bad_local(value : u8) -> u8 := do
    let number := read(new Atom::Number { value })
    return number + 1
  spec bad_local where
    ensures result == value + 1
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::native_owned_calls::bad_local
#guard_msgs (drop error) in
#leaner_verify 0x42::native_owned_calls::hidden_result

#leaner_require_native 0x42::native_owned_calls::read
#leaner_require_native 0x42::native_owned_calls::local_call
#leaner_require_native 0x42::native_owned_calls::identity
#leaner_require_native 0x42::native_owned_calls::nominal_local
#leaner_require_native 0x42::native_owned_calls::match_result
#leaner_require_native 0x42::native_owned_calls::weak_identity
#leaner_require_native 0x42::native_owned_calls::two_calls
#leaner_require_native 0x42::native_owned_calls::four_calls

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  let mut totals : Array (String × Nat) := #[]
  for function in ["read", "local_call", "identity", "nominal_local", "match_result",
      "weak_identity", "two_calls", "four_calls"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_owned_calls::{function} ")
    unless measured.size == 2 do throwError "missing native owned-call stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    logInfo m!"{function}: {total} heartbeats (all generated stages)"
    unless total ≤ 50000000 do throwError "native owned call {function} exceeds aggregate 50M budget"
    totals := totals.push (function, total)
  let some (_, two) := totals.find? (·.1 == "two_calls") | throwError "missing two-call cost"
  let some (_, four) := totals.find? (·.1 == "four_calls") | throwError "missing four-call cost"
  unless four ≤ 2 * two do throwError "repeated owned-call scaling regressed"
  for function in [`bad_local, `hidden_result] do
    for suffix in [`computation, `nativeSummary, `computationVerified, `computationRepresents] do
      if (← getEnv).contains (`«0x42».native_owned_calls ++ function ++ suffix) then
        throwError "rejected owned call leaked {function}.{suffix}"
  for (function, callee) in [("local_call", "read"), ("nominal_local", "identity"),
      ("nominal_local", "read"), ("match_result", "identity")] do
    let summary := Name.str `«0x42».native_owned_calls function ++ `nativeSummary
    let some proof := (← getEnv).find? summary |>.bind (·.value? (allowOpaque := true))
      | throwError "missing owned-call summary {summary}"
    unless proof.getUsedConstants.contains (Name.str `«0x42».native_owned_calls callee ++ `nativeSummary) do
      throwError "{summary} does not reuse the callee's contract"

open LeanerIR «0x42».native_owned_calls in
example (state : RuntimeState) :
    (identity.computation ⟨.Number ⟨255, by decide⟩⟩).ok
      state (.Number ⟨255, by decide⟩) state := by
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_owned_calls in
example (state : RuntimeState) (error : Failure) :
    (local_call.computation ⟨⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  simp [local_call.computation, read.computation, Spec.bind, Spec.pure, Spec.abort,
    NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
    IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]

open LeanerIR LeanerIR.Proofs «0x42».native_owned_calls in
example (state : RuntimeState) :
    (nominal_local.computation ⟨⟨255, by decide⟩⟩).ok state ⟨255, by decide⟩ state := by
  exact ⟨Atom.Number ⟨255, by decide⟩, state, ⟨rfl, rfl⟩, rfl, rfl⟩
