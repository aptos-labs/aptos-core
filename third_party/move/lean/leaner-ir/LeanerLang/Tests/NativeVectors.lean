-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_vectors where
  fun make() -> Vector<u64> := vector<u64>[10, 20, 30]
  spec make where
    ensures result == vector<u64>[10, 20, 30]
    aborts_if false
  verify make

  fun empty() -> Vector<u8> := vector<u8>[]
  spec empty where
    ensures result == vector<u8>[]
    aborts_if false
  verify empty

  fun echo(values : Vector<u64>) -> Vector<u64> := values
  spec echo where
    ensures result == values
    aborts_if false
  verify echo

  fun local_values() -> Vector<u8> := do
    let values := vector<u8>[3, 4]
    return values
  spec local_values where
    ensures result == vector<u8>[3, 4]
    aborts_if false
  verify local_values

  fun nested() -> Vector<Vector<u8> > := vector<Vector<u8> >[vector<u8>[1, 2], vector<u8>[]]
  spec nested where
    ensures result == vector<Vector<u8> >[vector<u8>[1, 2], vector<u8>[]]
    aborts_if false
  verify nested

  fun effectful(value : u8) -> Vector<u8> := vector<u8>[value + 1, value + 2]
  spec effectful where
    ensures result == vector<u8>[value + 1, value + 2]
    aborts_if value >= 254 with if value == 255 then value + 1 else value + 2
  verify effectful

  fun size(values : Vector<u64>) -> u64 := values.length
  spec size where
    ensures result == values.length
    aborts_if false
  verify size

  fun length() -> u64 := make().length
  spec length where
    ensures result == 3
    aborts_if false
  verify length

  fun call_echo(values : Vector<u64>) -> Vector<u64> := echo(values)
  spec call_echo where
    ensures result == values
    aborts_if false
  verify call_echo

  fun call_literal() -> Vector<u64> := echo(vector<u64>[1, 2])
  spec call_literal where
    ensures result == vector<u64>[1, 2]
    aborts_if false
  verify call_literal

  fun local_length(values : Vector<u64>) -> u64 := do
    let count := values.length
    return count
  spec local_length where
    ensures result == values.length
    aborts_if false
  verify local_length

  fun increment(value : u8) -> u8 := value + 1
  spec increment where
    ensures result == value + 1
    aborts_if value == 255 with value + 1
  verify increment

  fun from_calls(value : u8) -> Vector<u8> := vector<u8>[increment(value), increment(value)]
  spec from_calls where
    ensures result == vector<u8>[value + 1, value + 1]
    aborts_if value == 255 with value + 1
  verify from_calls

  fun mixed_calls(value : u8) -> Vector<u8> := vector<u8>[increment(value), value + 2]
  spec mixed_calls where
    ensures result == vector<u8>[value + 1, value + 2]
    aborts_if value >= 254 with if value == 255 then value + 1 else value + 2
  verify mixed_calls

  fun nested_calls() -> Vector<Vector<u64> > := vector<Vector<u64> >[make(), make()]
  spec nested_calls where
    ensures result == vector<Vector<u64> >[vector<u64>[10, 20, 30], vector<u64>[10, 20, 30]]
    aborts_if false
  verify nested_calls

  struct Batch has Copy, Drop, Store where
    values : Vector<u8>

  fun pack(value : u8) -> Batch := new Batch { values := vector<u8>[value] }
  spec pack where
    ensures result == new Batch { values := vector<u8>[value] }
    aborts_if false
  verify pack

  fun batch_length(batch : Batch) -> u64 := batch.values.length
  spec batch_length where
    ensures result == batch.values.length
    aborts_if false
  verify batch_length

  fun wrong_length(values : Vector<u64>) -> u64 := values.length
  spec wrong_length where
    ensures result == 0
    aborts_if false

  fun wrong_order(value : u8) -> Vector<u8> := vector<u8>[increment(value), value + 2]
  spec wrong_order where
    ensures true
    aborts_if value >= 254 with value + 2

#guard_msgs (drop error) in
#leaner_verify 0x42::native_vectors::wrong_length
#guard_msgs (drop error) in
#leaner_verify 0x42::native_vectors::wrong_order

#leaner_require_native_all

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["make", "empty", "echo", "local_values", "nested", "effectful", "size",
      "length", "call_echo", "call_literal", "local_length", "increment", "from_calls",
      "mixed_calls", "nested_calls", "pack", "batch_length"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_vectors::{function} ")
    unless measured.size == 2 do throwError "missing vector stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    let objects : Nat := measured.foldl (fun cost sample => cost + sample.objects) 0
    logInfo m!"{function}: {total} heartbeats / {objects} objects (all generated stages)"
    unless total ≤ 10000000 do throwError "native vector {function} exceeds aggregate 10M budget"
    unless objects ≤ 15000 do throwError "native vector {function} exceeds aggregate term-size budget"
  for (caller, callee) in [(`length, `make), (`call_echo, `echo), (`call_literal, `echo),
      (`from_calls, `increment), (`mixed_calls, `increment), (`nested_calls, `make)] do
    let some proof := (← getEnv).find? (`«0x42».native_vectors ++ caller ++ `nativeSummary)
        |>.bind (·.value? (allowOpaque := true))
      | throwError "missing native vector caller {caller}"
    unless proof.getUsedConstants.contains (`«0x42».native_vectors ++ callee ++ `nativeSummary) do
      throwError "native vector caller {caller} did not reuse {callee}"
  for rejected in [`wrong_length, `wrong_order] do
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified] do
      if (← getEnv).contains (`«0x42».native_vectors ++ rejected ++ suffix) then
        throwError "rejected vector contract leaked {rejected}.{suffix}"

set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_vectors in
example (state : RuntimeState) :
    (make.computation ⟨⟩).ok state
      ⟨#[⟨10, by decide⟩, ⟨20, by decide⟩, ⟨30, by decide⟩], by decide⟩ state := by
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_vectors in
example (state : RuntimeState) :
    (empty.computation ⟨⟩).ok state ⟨#[], by decide⟩ state := by
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_vectors in
example (state : RuntimeState) :
    (length.computation ⟨⟩).ok state ⟨3, by decide⟩ state := by
  have executed : length.computation ⟨⟩ = Spec.pure ⟨3, by decide⟩ := by
    simp [length.computation, make.computation, NativeVector.length]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_vectors in
example (state : RuntimeState) :
    (mixed_calls.computation ⟨⟨253, by decide⟩⟩).ok state
      ⟨#[⟨254, by decide⟩, ⟨255, by decide⟩], by decide⟩ state := by
  have executed : mixed_calls.computation ⟨⟨253, by decide⟩⟩ =
      Spec.pure ⟨#[⟨254, by decide⟩, ⟨255, by decide⟩], by decide⟩ := by
    simp [mixed_calls.computation, increment.computation, NativeArithmetic.checkedInteger,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_vectors in
example (state : RuntimeState) (error : Failure) :
    (mixed_calls.computation ⟨⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : mixed_calls.computation ⟨⟨255, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [mixed_calls.computation, increment.computation, NativeArithmetic.checkedInteger,
      NativeArithmetic.runtimeFailure, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_vectors in
example (state : RuntimeState) (error : Failure) :
    (mixed_calls.computation ⟨⟨254, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : mixed_calls.computation ⟨⟨254, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [mixed_calls.computation, increment.computation, NativeArithmetic.checkedInteger,
      NativeArithmetic.runtimeFailure, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_vectors in
example (state : RuntimeState) :
    (nested_calls.computation ⟨⟩).ok state
      ⟨#[⟨#[⟨10, by decide⟩, ⟨20, by decide⟩, ⟨30, by decide⟩], by decide⟩,
          ⟨#[⟨10, by decide⟩, ⟨20, by decide⟩, ⟨30, by decide⟩], by decide⟩], by decide⟩ state := by
  have executed : nested_calls.computation ⟨⟩ = Spec.pure
      ⟨#[⟨#[⟨10, by decide⟩, ⟨20, by decide⟩, ⟨30, by decide⟩], by decide⟩,
          ⟨#[⟨10, by decide⟩, ⟨20, by decide⟩, ⟨30, by decide⟩], by decide⟩], by decide⟩ := by
    simp [nested_calls.computation, make.computation]
  rw [executed]
  exact ⟨rfl, rfl⟩
