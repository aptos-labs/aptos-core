-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_payloads where
  enum Atom has Copy, Drop, Store where
    | None
    | Number (value : u8)

  enum Envelope has Copy, Drop, Store where
    | Empty
    | One (value : Atom)
    | Two (left : Atom, right : Atom)

  struct Payload has Copy, Drop, Store where
    number : u8
    enabled : Bool

  struct Nested has Copy, Drop, Store where
    inner : Payload
    ignored : u8

  fun destructure(value : Nested) -> u8 := do
    let Nested { inner := Payload { number := number, enabled := enabled }, ignored := _ } := value
    return if enabled then number else 0
  spec destructure where
    ensures result == if value.inner.enabled then value.inner.number else 0
    aborts_if false
  verify destructure

  fun read(atom : Atom) -> u8 :=
    match atom with
      | Atom::None {} => 0
      | Atom::Number { value := number } => number
  spec read where
    ensures result == match atom with
      | Atom::None {} => 0
      | Atom::Number { value := number } => number
    aborts_if false
  verify read

  fun add(left : Atom, right : Atom) -> u8 :=
    match left with
      | Atom::None {} => 0
      | Atom::Number { value := a } =>
        match right with
          | Atom::None {} => 0
          | Atom::Number { value := b } => a + b
  spec add where
    pragma aborts_if_is_partial
    ensures result == match left with
      | Atom::None {} => 0
      | Atom::Number { value := a } =>
        match right with
          | Atom::None {} => 0
          | Atom::Number { value := b } => a + b
  verify add

  fun nested(envelope : Envelope) -> u8 :=
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
  spec nested where
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
  verify nested

  fun one(value : u8) -> u8 := nested(new Envelope::One { value := new Atom::Number { value } })
  spec one where
    ensures result == value
  verify one

  fun empty() -> u8 := nested(new Envelope::Empty {})
  spec empty where
    ensures result == 0
  verify empty

  fun forward(atom : Atom) -> u8 := read(atom)
  spec forward where
    ensures result == match atom with
      | Atom::None {} => 0
      | Atom::Number { value := value } => value
    aborts_if false
  verify forward

  fun guarded(atom : Atom) -> u8 :=
    match atom with
      | Atom::None {} => 0
      | Atom::Number { value := value } => if value < 255 then value + 1 else value
  spec guarded where
    ensures result == match atom with
      | Atom::None {} => 0
      | Atom::Number { value := value } => if value < 255 then value + 1 else value
    aborts_if false
  verify guarded

  fun bounded(atom : Atom) -> u8 :=
    match atom with
      | Atom::None {} => 0
      | Atom::Number { value := value } => value + 1
  spec bounded where
    requires match atom with
      | Atom::None {} => true
      | Atom::Number { value := value } => value < 255
    ensures result == match atom with
      | Atom::None {} => 0
      | Atom::Number { value := value } => value + 1
    aborts_if false
  verify bounded

  fun bounded_call(value : u8) -> u8 := bounded(new Atom::Number { value })
  spec bounded_call where
    requires value < 255
    ensures result == value + 1
    aborts_if false
  verify bounded_call

  fun bad_call(value : u8) -> u8 := bounded(new Atom::Number { value })
  spec bad_call where
    ensures result == value + 1
    aborts_if false

  fun bad(atom : Atom) -> u8 :=
    match atom with
      | Atom::None {} => 0
      | Atom::Number { value := value } => value
  spec bad where
    ensures result == 0
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::native_payloads::bad_call
#guard_msgs (drop error) in
#leaner_verify 0x42::native_payloads::bad

#leaner_require_native 0x42::native_payloads::read
#leaner_require_native 0x42::native_payloads::destructure
#leaner_require_native 0x42::native_payloads::add
#leaner_require_native 0x42::native_payloads::nested
#leaner_require_native 0x42::native_payloads::one
#leaner_require_native 0x42::native_payloads::empty
#leaner_require_native 0x42::native_payloads::forward
#leaner_require_native 0x42::native_payloads::guarded
#leaner_require_native 0x42::native_payloads::bounded
#leaner_require_native 0x42::native_payloads::bounded_call

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["destructure", "read", "add", "nested", "one", "empty", "forward", "guarded",
      "bounded", "bounded_call"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_payloads::{function} ")
    unless measured.size == 2 do throwError "missing native payload cost stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    logInfo m!"{function}: {total} heartbeats (all generated stages)"
    unless total ≤ 50000000 do throwError "native payload {function} exceeds aggregate 50M budget"
  for function in [`bad_call, `bad] do
    for suffix in [`computation, `nativeSummary, `computationVerified, `computationRepresents] do
      if (← getEnv).contains (`«0x42».native_payloads ++ function ++ suffix) then
        throwError "rejected payload leaked {function}.{suffix}"

open LeanerIR LeanerIR.Proofs «0x42».native_payloads in
example (state : RuntimeState) :
    (nested.computation ⟨.Two (.Number ⟨255, by decide⟩) (.Number ⟨1, by decide⟩)⟩).aborts
      state (.abort, #[.integer 256]) := by
  simp [nested.computation, NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
    IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?, Spec.abort]

open LeanerIR «0x42».native_payloads in
example (state : RuntimeState) :
    (nested.computation ⟨.Two .None (.Number ⟨255, by decide⟩)⟩).ok state ⟨0, by decide⟩ state := by
  exact ⟨rfl, rfl⟩

open LeanerIR «0x42».native_payloads in
example (state : RuntimeState) :
    (destructure.computation ⟨⟨⟨⟨255, by decide⟩, true⟩, ⟨0, by decide⟩⟩⟩).ok
      state ⟨255, by decide⟩ state := by
  exact ⟨rfl, rfl⟩

open LeanerIR «0x42».native_payloads in
example (state : RuntimeState) :
    (guarded.computation ⟨.Number ⟨255, by decide⟩⟩).ok state ⟨255, by decide⟩ state := by
  exact ⟨rfl, rfl⟩
