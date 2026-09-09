-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_index where
  fun middle() -> u64 := do
    let values := vector<u64>[10, 20, 30]
    let view := &values[1]
    return *view
  spec middle where
    ensures result == 20
    aborts_if false
  verify middle

  fun dynamic(values : Vector<u64>, index : u64) -> u64 := do
    let view := &values[index]
    return *view
  spec dynamic where
    requires index < values.length
    ensures result == values[index]
    aborts_if false
  verify dynamic

  fun bad() -> u64 := do
    let values := vector<u64>[1]
    let view := &values[1]
    return *view
  spec bad where
    ensures false
    aborts_if true
  verify bad

  fun flag(value : Bool) -> Bool := do
    let values := vector<Bool>[value]
    let view := &values[0]
    return *view
  spec flag where
    ensures result == value
    aborts_if false
  verify flag

  fun shared(values : &Vector<u64>, index : u64) -> u64 := do
    let view := &values[index]
    return *view
  spec shared where
    requires index < values.length
    ensures result == values[index]
    aborts_if false
  verify shared

  fun nested() -> u64 := do
    let values := vector<Vector<u64> >[vector<u64>[1, 2], vector<u64>[3, 4]]
    let row_view := &values[1]
    let row := *row_view
    let view := &row[0]
    return *view
  spec nested where
    ensures result == 3
    aborts_if false
  verify nested

  fun computed(index : u64) -> u64 := do
    let values := vector<u64>[10, 20, 30]
    let view := &values[index + 1]
    return *view
  spec computed where
    requires index == 1
    ensures result == 30
    aborts_if false
  verify computed

  fun any_index(values : Vector<u64>, index : u64) -> u64 := do
    let view := &values[index]
    return *view
  spec any_index where
    ensures result == values[index]
    aborts_if index >= values.length
  verify any_index

  fun forward(values : Vector<u64>, index : u64) -> u64 := dynamic(values, index)
  spec forward where
    requires index < values.length
    ensures result == values[index]
    aborts_if false
  verify forward

  fun make() -> Vector<u64> := vector<u64>[10, 20, 30]
  spec make where
    ensures result == vector<u64>[10, 20, 30]
    aborts_if false
  verify make

  fun from_callee() -> u64 := do
    let values := make()
    let view := &values[1]
    return *view
  spec from_callee where
    ensures result == 20
    aborts_if false
  verify from_callee

  fun byte(values : Vector<u8>, index : u64) -> u8 := do
    let view := &values[index]
    return *view
  spec byte where
    requires index < values.length
    ensures result == values[index]
    aborts_if false
  verify byte

  fun byte_computed(index : u64) -> u8 := do
    let values := vector<u8>[10, 20, 30]
    let view := &values[index + 1]
    return *view
  spec byte_computed where
    requires index == 1
    ensures result == 30
    aborts_if false
  verify byte_computed

  fun wrong_result() -> u64 := do
    let values := vector<u64>[10, 20, 30]
    let view := &values[1]
    return *view
  spec wrong_result where
    ensures result == 10
    aborts_if false

  fun wrong_error_kind() -> u64 := do
    let values := vector<u64>[1]
    let view := &values[1]
    return *view
  spec wrong_error_kind where
    ensures false
    aborts_if true with 1

  fun wrong_bounds(values : Vector<u64>, index : u64) -> u64 := do
    let view := &values[index]
    return *view
  spec wrong_bounds where
    ensures result == values[index]
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::native_index::wrong_result
#guard_msgs (drop error) in
#leaner_verify 0x42::native_index::wrong_error_kind
#guard_msgs (drop error) in
#leaner_verify 0x42::native_index::wrong_bounds

#leaner_require_native_all

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["middle", "dynamic", "bad", "flag", "shared", "nested", "computed",
      "any_index", "forward", "make", "from_callee", "byte", "byte_computed"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_index::{function} ")
    unless measured.size == 2 do throwError "missing index stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    let objects : Nat := measured.foldl (fun cost sample => cost + sample.objects) 0
    logInfo m!"{function}: {total} heartbeats / {objects} objects (all generated stages)"
    unless total ≤ 10000000 do throwError "native index {function} exceeds aggregate 10M budget"
    unless objects ≤ 15000 do throwError "native index {function} exceeds aggregate term-size budget"
  let some proof := (← getEnv).find? `«0x42».native_index.forward.nativeSummary
      |>.bind (·.value? (allowOpaque := true))
    | throwError "missing native index caller summary"
  unless proof.getUsedConstants.contains `«0x42».native_index.dynamic.nativeSummary do
    throwError "native index caller did not reuse the callee summary"
  let some producerProof := (← getEnv).find? `«0x42».native_index.from_callee.nativeSummary
      |>.bind (·.value? (allowOpaque := true))
    | throwError "missing native vector producer caller summary"
  unless producerProof.getUsedConstants.contains `«0x42».native_index.make.nativeSummary do
    throwError "native indexed caller did not reuse the vector producer summary"
  for rejected in [`wrong_result, `wrong_error_kind, `wrong_bounds] do
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified, `verified] do
      if (← getEnv).contains (`«0x42».native_index ++ rejected ++ suffix) then
        throwError "rejected index contract leaked {rejected}.{suffix}"

set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_index in
example (state : RuntimeState) :
    (middle.computation ⟨⟩).ok state ⟨20, by decide⟩ state := by
  have executed : middle.computation ⟨⟩ = Spec.pure ⟨20, by decide⟩ := by
    simp [middle.computation, NativeVector.get]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_index in
example (state : RuntimeState) (error : Failure) :
    (bad.computation ⟨⟩).aborts state error ↔
      error = (.profile { profile := .move, tag := "runtime.vector_error" }, #[.integer 1]) := by
  have executed : bad.computation ⟨⟩ =
      Spec.abort (.profile { profile := .move, tag := "runtime.vector_error" }, #[.integer 1]) := by
    simp [bad.computation, NativeVector.get, NativeVector.indexFailure]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_index in
example (state : RuntimeState) :
    (nested.computation ⟨⟩).ok state ⟨3, by decide⟩ state := by
  have executed : nested.computation ⟨⟩ = Spec.pure ⟨3, by decide⟩ := by
    simp [nested.computation, NativeVector.get]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_index in
example (state : RuntimeState) (error : Failure) :
    (computed.computation ⟨⟨18446744073709551615, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 18446744073709551616]) := by
  have executed : computed.computation ⟨⟨18446744073709551615, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 18446744073709551616]) := by
    simp [computed.computation, NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_index in
example (state : RuntimeState) (error : Failure) :
    (computed.computation ⟨⟨2, by decide⟩⟩).aborts state error ↔
      error = (.profile { profile := .move, tag := "runtime.vector_error" }, #[.integer 1]) := by
  have executed : computed.computation ⟨⟨2, by decide⟩⟩ =
      Spec.abort (.profile { profile := .move, tag := "runtime.vector_error" }, #[.integer 1]) := by
    simp [computed.computation, NativeArithmetic.checkedInteger, IntegerValueFits,
      Ty.integerValueFits?, Ty.integerBounds?, NativeVector.get, NativeVector.indexFailure]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_index in
example (state : RuntimeState) :
    (any_index.computation ⟨⟨#[⟨5, by decide⟩, ⟨7, by decide⟩], by decide⟩, ⟨1, by decide⟩⟩).ok
      state ⟨7, by decide⟩ state := by
  have executed : any_index.computation
      ⟨⟨#[⟨5, by decide⟩, ⟨7, by decide⟩], by decide⟩, ⟨1, by decide⟩⟩ = Spec.pure ⟨7, by decide⟩ := by
    simp [any_index.computation, NativeVector.get]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_index in
example (state : RuntimeState) (error : Failure) :
    (any_index.computation ⟨⟨#[⟨5, by decide⟩, ⟨7, by decide⟩], by decide⟩, ⟨2, by decide⟩⟩).aborts
      state error ↔
      error = (.profile { profile := .move, tag := "runtime.vector_error" }, #[.integer 1]) := by
  have executed : any_index.computation
      ⟨⟨#[⟨5, by decide⟩, ⟨7, by decide⟩], by decide⟩, ⟨2, by decide⟩⟩ =
      Spec.abort (.profile { profile := .move, tag := "runtime.vector_error" }, #[.integer 1]) := by
    simp [any_index.computation, NativeVector.get, NativeVector.indexFailure]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_index in
example (state : RuntimeState) :
    (byte.computation ⟨⟨#[⟨5, by decide⟩, ⟨9, by decide⟩], by decide⟩, ⟨1, by decide⟩⟩).ok
      state ⟨9, by decide⟩ state := by
  have executed : byte.computation
      ⟨⟨#[⟨5, by decide⟩, ⟨9, by decide⟩], by decide⟩, ⟨1, by decide⟩⟩ = Spec.pure ⟨9, by decide⟩ := by
    simp [byte.computation, NativeVector.get]
  rw [executed]
  exact ⟨rfl, rfl⟩
