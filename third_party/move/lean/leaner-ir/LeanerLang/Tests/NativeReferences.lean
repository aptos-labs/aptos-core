-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_references where
  fun read(value : &u64) -> u64 := *value
  spec read where
    ensures result == value
    aborts_if false
  verify read

  fun flag(value : &Bool) -> Bool := *value
  spec flag where
    ensures result == value
    aborts_if false
  verify flag

  fun increment(value : &u8) -> u8 := *value + 1
  spec increment where
    ensures result == value + 1
    aborts_if value == 255 with value + 1
  verify increment

  fun nested(value : &u8) -> u8 := (*value + 1) + 1
  spec nested where
    ensures result == value + 2
    aborts_if value >= 254 with if value == 255 then value + 1 else value + 2
  verify nested

  fun local_read(value : &u64) -> u64 := do
    let current := *value
    return current
  spec local_read where
    ensures result == value
    aborts_if false
  verify local_read

  fun choose(flag : &Bool) -> u8 := if *flag then 1 else 2
  spec choose where
    ensures result == if flag then 1 else 2
    aborts_if false
  verify choose

  fun equal(left : &Bool, right : &Bool) -> Bool := *left == *right
  spec equal where
    ensures result == (left == right)
    aborts_if false
  verify equal

  fun forward(value : &u64) -> u64 := read(value)
  spec forward where
    ensures result == value
    aborts_if false
  verify forward

  fun values(source : &Vector<u8>) -> Vector<u8> := *source
  spec values where
    ensures result == source
    aborts_if false
  verify values

  fun length(source : &Vector<u8>) -> u64 := (*source).length
  spec length where
    ensures result == source.length
    aborts_if false
  verify length

  struct Pair has Copy, Drop, Store where
    first : u64
    second : u64

  fun pair(source : &Pair) -> Pair := *source
  spec pair where
    ensures result == source
    aborts_if false
  verify pair

  fun field(source : &Pair) -> u64 := (*source).first
  spec field where
    ensures result == source.first
    aborts_if false
  verify field

  fun shared_local(value : u64) -> u64 := do
    let view := &value
    return *view
  spec shared_local where
    ensures result == value
    aborts_if false
  verify shared_local

  fun shared_vector() -> u64 := do
    let source := vector<u8>[1, 2, 3]
    let view := &source
    return (*view).length
  spec shared_vector where
    ensures result == 3
    aborts_if false
  verify shared_vector

  fun forward_increment(value : &u8) -> u8 := increment(value)
  spec forward_increment where
    ensures result == value + 1
    aborts_if value == 255 with value + 1
  verify forward_increment

  fun wrong_read(value : &u64) -> u64 := *value
  spec wrong_read where
    ensures result == 0
    aborts_if false

  fun wrong_flag(value : &Bool) -> Bool := *value
  spec wrong_flag where
    ensures result == !value
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::native_references::wrong_read
#guard_msgs (drop error) in
#leaner_verify 0x42::native_references::wrong_flag

#leaner_require_native_all

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["read", "flag", "increment", "nested", "local_read", "choose", "equal",
      "forward", "values", "length", "pair", "field", "shared_local", "shared_vector",
      "forward_increment"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_references::{function} ")
    unless measured.size == 2 do throwError "missing reference stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    let objects : Nat := measured.foldl (fun cost sample => cost + sample.objects) 0
    logInfo m!"{function}: {total} heartbeats / {objects} objects (all generated stages)"
    unless total ≤ 10000000 do throwError "native reference {function} exceeds aggregate 10M budget"
    unless objects ≤ 15000 do throwError "native reference {function} exceeds aggregate term-size budget"
  for (caller, callee) in [(`forward, `read), (`forward_increment, `increment)] do
    let some proof := (← getEnv).find? (`«0x42».native_references ++ caller ++ `nativeSummary)
        |>.bind (·.value? (allowOpaque := true))
      | throwError "missing shared caller summary {caller}"
    unless proof.getUsedConstants.contains (`«0x42».native_references ++ callee ++ `nativeSummary) do
      throwError "shared caller {caller} did not reuse the callee summary {callee}"
  for rejected in [`wrong_read, `wrong_flag] do
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified, `verified] do
      if (← getEnv).contains (`«0x42».native_references ++ rejected ++ suffix) then
        throwError "rejected reference contract leaked {rejected}.{suffix}"

set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_references in
example (args : read.Arguments) : read.computation args = Spec.pure args.value := rfl

open LeanerIR LeanerIR.Proofs «0x42».native_references in
example (args : flag.Arguments) : flag.computation args = Spec.pure args.value := rfl

open LeanerIR LeanerIR.Proofs «0x42».native_references in
example (args : values.Arguments) : values.computation args = Spec.pure args.source := rfl

open LeanerIR LeanerIR.Proofs «0x42».native_references in
example (args : pair.Arguments) : pair.computation args = Spec.pure args.source := rfl

open LeanerIR LeanerIR.Proofs «0x42».native_references in
example (state : RuntimeState) (error : Failure) :
    (increment.computation ⟨⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : increment.computation ⟨⟨255, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [increment.computation, NativeArithmetic.checkedInteger,
      NativeArithmetic.runtimeFailure, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_references in
example (state : RuntimeState) (error : Failure) :
    (nested.computation ⟨⟨254, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : nested.computation ⟨⟨254, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [nested.computation, NativeArithmetic.checkedInteger,
      NativeArithmetic.runtimeFailure, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_references in
example (state : RuntimeState) (error : Failure) :
    (nested.computation ⟨⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : nested.computation ⟨⟨255, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [nested.computation, NativeArithmetic.checkedInteger,
      NativeArithmetic.runtimeFailure, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl
