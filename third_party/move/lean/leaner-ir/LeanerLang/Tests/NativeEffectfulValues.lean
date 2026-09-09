-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_effectful_values where
  struct Box has Copy, Drop, Store where
    value : u8
  struct Pair has Copy, Drop, Store where
    left : Box
    right : Box
  struct Packet has Copy, Drop, Store where
    flag : Bool
    value : u8
  enum Choice has Copy, Drop, Store where
    | Empty
    | Value(value : u8)

  fun embedded(value : u8) -> Box := new Box { value := value + 1 }
  spec embedded where
    ensures result.value == value + 1
    aborts_if value == 255 with value + 1
  verify embedded

  fun nested(value : u8) -> Pair := new Pair {
    left := new Box { value := value + 1 },
    right := new Box { value := value + 2 } }
  spec nested where
    ensures result.left.value == value + 1
    ensures result.right.value == value + 2
    aborts_if value >= 254 with if value == 255 then value + 1 else value + 2
  verify nested

  fun local_value(value : u8) -> u8 := do
    let boxed := new Box { value := value + 1 }
    return boxed.value
  spec local_value where
    ensures result == value + 1
    aborts_if value == 255 with value + 1
  verify local_value

  fun variant(value : u8) -> Choice := new Choice::Value { value := value + 1 }
  spec variant where
    ensures result == new Choice::Value { value := value + 1 }
    aborts_if value == 255 with value + 1
  verify variant

  fun read(boxed : Box) -> u8 := boxed.value
  spec read where
    ensures result == boxed.value
    aborts_if false
  verify read

  fun local_call(value : u8) -> u8 := do
    let boxed := new Box { value := value + 1 }
    return read(boxed)
  spec local_call where
    ensures result == value + 1
    aborts_if value == 255 with value + 1
  verify local_call

  fun mixed(flag : Bool, value : u8) -> Packet := new Packet { flag, value := value + 1 }
  spec mixed where
    ensures result.flag == flag
    ensures result.value == value + 1
    aborts_if value == 255 with value + 1
  verify mixed

  fun bad(value : u8) -> Box := new Box { value := value + 1 }
  spec bad where
    ensures result.value == value + 1
    aborts_if false

  fun wrong_order(value : u8) -> Pair := new Pair {
    left := new Box { value := value + 1 },
    right := new Box { value := value + 2 } }
  spec wrong_order where
    ensures result.left.value == value + 1
    ensures result.right.value == value + 2
    aborts_if value >= 254 with value + 2

#guard_msgs (drop error) in
#leaner_verify 0x42::native_effectful_values::bad
#guard_msgs (drop error) in
#leaner_verify 0x42::native_effectful_values::wrong_order

#leaner_require_native 0x42::native_effectful_values::embedded
#leaner_require_native 0x42::native_effectful_values::nested
#leaner_require_native 0x42::native_effectful_values::local_value
#leaner_require_native 0x42::native_effectful_values::variant
#leaner_require_native 0x42::native_effectful_values::read
#leaner_require_native 0x42::native_effectful_values::local_call
#leaner_require_native 0x42::native_effectful_values::mixed

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["embedded", "nested", "local_value", "variant", "read", "local_call", "mixed"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_effectful_values::{function} ")
    unless measured.size == 2 do throwError "missing effectful value stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    logInfo m!"{function}: {total} heartbeats (all generated stages)"
    unless total ≤ 50000000 do throwError "native effectful value {function} exceeds aggregate 50M budget"
  for function in [`bad, `wrong_order] do
    for suffix in [`computation, `nativeSummary, `computationVerified, `computationRepresents] do
      if (← getEnv).contains (`«0x42».native_effectful_values ++ function ++ suffix) then
        throwError "rejected effectful value leaked {function}.{suffix}"
  let summary := `«0x42».native_effectful_values.local_call.nativeSummary
  let some proof := (← getEnv).find? summary |>.bind (·.value? (allowOpaque := true))
    | throwError "missing effectful local caller summary"
  unless proof.getUsedConstants.contains `«0x42».native_effectful_values.read.nativeSummary do
    throwError "effectful local caller does not reuse the callee's contract"

set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_effectful_values in
example (state : RuntimeState) :
    (embedded.computation ⟨⟨254, by decide⟩⟩).ok state ⟨⟨255, by decide⟩⟩ state := by
  have executed : embedded.computation ⟨⟨254, by decide⟩⟩ =
      Spec.pure (Box.mk ⟨255, by decide⟩) := by
    simp [embedded.computation, NativeArithmetic.checkedInteger,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_effectful_values in
example (state : RuntimeState) (error : Failure) :
    (nested.computation ⟨⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : nested.computation ⟨⟨255, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [nested.computation, NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_effectful_values in
example (state : RuntimeState) (error : Failure) :
    (nested.computation ⟨⟨254, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : nested.computation ⟨⟨254, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [nested.computation, NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_effectful_values in
example (state : RuntimeState) :
    (mixed.computation ⟨true, ⟨0, by decide⟩⟩).ok state ⟨true, ⟨1, by decide⟩⟩ state := by
  have executed : mixed.computation ⟨true, ⟨0, by decide⟩⟩ =
      Spec.pure (Packet.mk true ⟨1, by decide⟩) := by
    simp [mixed.computation, NativeArithmetic.checkedInteger,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  exact ⟨rfl, rfl⟩
