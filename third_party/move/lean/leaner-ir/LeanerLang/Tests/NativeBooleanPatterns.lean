-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

-- Guard normalization must terminate in both directions and reuse a callee
-- equivalence without unfolding its computation.
set_option maxHeartbeats 1000 in
example (flag : Bool) (branchFalse : ¬ (flag == true) = true) : ¬ flag = true := by
  leaner_native_guard branchFalse
  assumption

set_option maxHeartbeats 1000 in
example (flag value : Bool) (summary : value = true ↔ flag = true)
    (branchTrue : (value == true) = true) : flag = true := by
  leaner_native_guard branchTrue
  assumption

set_option maxHeartbeats 1000 in
example (flag : Bool) (valueEquation : flag = true) (branchFalse : ¬ flag = true) : False := by
  leaner_native_guard branchFalse

#leaner_measure

leaner module 0x42::native_boolean_patterns where
  struct Flag has Copy, Drop, Store where
    value : Bool

  fun match_effect(flag : Bool, value : u8) -> u8 := match flag with
    | true => value + 1
    | false => 0
  spec match_effect where
    ensures result == if flag then value + 1 else 0
    aborts_if flag && value == 255
  verify match_effect

  fun echo(flag : Bool) -> Bool := flag
  spec echo where
    ensures result == flag
    aborts_if false
  verify echo

  fun after_call(flag : Bool) -> u8 := match echo(flag) with
    | true => 1
    | _ => 0
  spec after_call where
    ensures result == if flag then 1 else 0
    aborts_if false
  verify after_call

  fun equal(left : Bool, right : Bool) -> Bool := left == right
  spec equal where
    ensures result == (left == right)
    aborts_if false
  verify equal

  fun not_equal(left : Bool, right : Bool) -> Bool := left != right
  spec not_equal where
    ensures result == (left != right)
    aborts_if false
  verify not_equal

  fun local_equal(left : Bool, right : Bool) -> u8 := do
    let same := left == right
    return if same then 1 else 0
  spec local_equal where
    ensures result == if left == right then 1 else 0
    aborts_if false
  verify local_equal

  fun false_first(flag : Bool) -> u8 := match flag with
    | false => 0
    | true => 1
  spec false_first where
    ensures result == if flag then 1 else 0
    aborts_if false
  verify false_first

  fun literal_first(flag : Bool) -> Bool := true == flag
  spec literal_first where
    ensures result == flag
    aborts_if false
  verify literal_first

  fun projection_equal(flag : Flag, other : Bool) -> Bool := flag.value == other
  spec projection_equal where
    ensures result == (flag.value == other)
    aborts_if false
  verify projection_equal

  fun bad(flag : Bool, value : u8) -> u8 := match flag with
    | true => value + 1
    | false => 0
  spec bad where
    ensures result == if flag then value + 1 else 0
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::native_boolean_patterns::bad

#leaner_require_native 0x42::native_boolean_patterns::match_effect
#leaner_require_native 0x42::native_boolean_patterns::echo
#leaner_require_native 0x42::native_boolean_patterns::after_call
#leaner_require_native 0x42::native_boolean_patterns::equal
#leaner_require_native 0x42::native_boolean_patterns::not_equal
#leaner_require_native 0x42::native_boolean_patterns::local_equal
#leaner_require_native 0x42::native_boolean_patterns::false_first
#leaner_require_native 0x42::native_boolean_patterns::literal_first
#leaner_require_native 0x42::native_boolean_patterns::projection_equal

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["match_effect", "echo", "after_call", "equal", "not_equal", "local_equal",
      "false_first", "literal_first", "projection_equal"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_boolean_patterns::{function} ")
    unless measured.size == 2 do throwError "missing Boolean pattern stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    logInfo m!"{function}: {total} heartbeats (all generated stages)"
    unless total ≤ 50000000 do throwError "native Boolean pattern {function} exceeds aggregate 50M budget"
  for suffix in [`computation, `nativeSummary, `computationVerified, `computationRepresents] do
    if (← getEnv).contains (`«0x42».native_boolean_patterns.bad ++ suffix) then
      throwError "rejected Boolean pattern leaked {suffix}"
  let summary := `«0x42».native_boolean_patterns.after_call.nativeSummary
  let some proof := (← getEnv).find? summary |>.bind (·.value? (allowOpaque := true))
    | throwError "missing Boolean caller summary"
  unless proof.getUsedConstants.contains `«0x42».native_boolean_patterns.echo.nativeSummary do
    throwError "Boolean caller does not reuse the callee's contract"

open LeanerIR LeanerIR.Proofs «0x42».native_boolean_patterns in
example (state : RuntimeState) :
    (match_effect.computation ⟨false, ⟨255, by decide⟩⟩).ok state ⟨0, by decide⟩ state := by
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_boolean_patterns in
example (state : RuntimeState) (error : Failure) :
    (match_effect.computation ⟨true, ⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  simp [match_effect.computation, NativeArithmetic.checkedInteger,
    NativeArithmetic.runtimeFailure, Spec.abort,
    IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]

open LeanerIR LeanerIR.Proofs «0x42».native_boolean_patterns in
example (state : RuntimeState) :
    (after_call.computation ⟨true⟩).ok state ⟨1, by decide⟩ state := by
  exact ⟨true, state, ⟨rfl, rfl⟩, rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_boolean_patterns in
example (left right : Bool) (state : RuntimeState) :
    (not_equal.computation ⟨left, right⟩).ok state (left != right) state := by
  exact ⟨rfl, rfl⟩
