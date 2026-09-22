-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_mutable_generated where
  fun set_seven(slot : &mut u64) -> Unit := *slot := 7
  spec set_seven where
    ensures *slot == 7
    aborts_if false
  verify set_seven

  fun byte_max(slot : &mut u8) -> Unit := *slot := 255
  spec byte_max where
    ensures *slot == 255
    aborts_if false
  verify byte_max

  fun signed_zero(slot : &mut i64) -> Unit := *slot := 0
  spec signed_zero where
    ensures *slot == 0
    aborts_if false
  verify signed_zero

  fun grows(slot : &mut u64) -> Unit := *slot := 7
  spec grows where
    requires *slot < 7
    ensures *slot > old(slot)
    aborts_if false
  verify grows

  fun unspecified(slot : &mut u64) -> Unit := *slot := 1
  spec unspecified where
    ensures *slot == 1
  verify unspecified

  fun strict(slot : &mut u64) -> Unit := *slot := 1
  spec strict where
    pragma aborts_if_is_strict
    ensures *slot == 1
  verify strict

  fun declared(slot : &mut u64) -> Unit := *slot := 7
  spec declared where
    requires *slot < 7
    ensures *slot > old(slot)
    aborts_if *slot > 100 with 9
  verify declared

  fun partial_abort(slot : &mut u64) -> Unit := *slot := 7
  spec partial_abort where
    pragma aborts_if_is_partial
    requires *slot < 7
    ensures *slot > old(slot)
    aborts_if *slot > 100 with 9
  verify partial_abort

  fun no_post(slot : &mut u64) -> Unit := *slot := 7
  spec no_post where
    aborts_if false
  verify no_post

  fun bump(slot : &mut u64) -> Unit := *slot := *slot + 1
  spec bump where
    ensures *slot == old(slot) + 1
    aborts_if old(slot) == 18446744073709551615
  verify bump

  fun bump_safe(slot : &mut u64) -> Unit := *slot := *slot + 1
  spec bump_safe where
    requires *slot < 100
    ensures *slot == old(slot) + 1
    aborts_if false
  verify bump_safe

  fun halve(slot : &mut u64) -> Unit := *slot := *slot / 2
  spec halve where
    ensures *slot == old(slot) / 2
    aborts_if false
  verify halve

  fun square(slot : &mut u8) -> Unit := *slot := *slot * *slot
  spec square where
    requires *slot < 16
    ensures *slot == old(slot) * old(slot)
    aborts_if false
  verify square

  fun nested(slot : &mut u64) -> Unit := *slot := (*slot + 1) + (*slot + 2)
  spec nested where
    requires *slot < 100
    ensures *slot == 2 * old(slot) + 3
    aborts_if false
  verify nested

  fun wrong_value(slot : &mut u64) -> Unit := *slot := 7
  spec wrong_value where
    ensures *slot == 8
    aborts_if false

  fun wrong_old(slot : &mut u64) -> Unit := *slot := 7
  spec wrong_old where
    requires *slot == 0
    ensures *slot == old(slot)
    aborts_if false

  fun wrong_abort(slot : &mut u64) -> Unit := *slot := 7
  spec wrong_abort where
    aborts_if true

  fun missing_overflow(slot : &mut u64) -> Unit := *slot := *slot + 1
  spec missing_overflow where
    ensures *slot == old(slot) + 1
    aborts_if false

#leaner_require_native_all

#guard_msgs (drop error) in
#leaner_verify 0x42::native_mutable_generated::wrong_value
#guard_msgs (drop error) in
#leaner_verify 0x42::native_mutable_generated::wrong_old
#guard_msgs (drop error) in
#leaner_verify 0x42::native_mutable_generated::wrong_abort
#guard_msgs (drop error) in
#leaner_verify 0x42::native_mutable_generated::missing_overflow

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["set_seven", "byte_max", "signed_zero", "grows", "unspecified",
      "strict", "declared", "partial_abort", "no_post", "bump", "bump_safe",
      "halve", "square", "nested"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_mutable_generated::{function} ")
    unless measured.size == 2 do throwError "missing mutable generation stages for {function}"
    let total := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    let objects := measured.foldl (fun cost sample => cost + sample.objects) 0
    logInfo m!"{function}: {total} heartbeats / {objects} objects (all generated stages)"
    if total > 10000000 || objects > 15000 then
      throwError "native mutable generation exceeds reference-test 10M/15k ceiling: {function}"
    let base := `«0x42».native_mutable_generated ++ Name.mkSimple function
    for suffix in [`computationVerified, `computationBoundary, `computationRepresents, `verified] do
      if (← collectAxioms (base ++ suffix)).contains ``sorryAx then
        throwError "admitted native mutable artifact: {base ++ suffix}"
    let some computation := (← getEnv).find? (base ++ `computation) |>.bind (·.value?)
      | throwError "missing native mutable computation {function}"
    let some contract := (← getEnv).find? (base ++ `nativeContract) |>.bind (·.value?)
      | throwError "missing authored native owner contract {function}"
    unless contract.isAppOfArity ``LeanerIR.Proofs.Contract.mk 10 do
      throwError "unexpected native owner contract shape {function}"
    let post := contract.getArg! 5
    for retired in [``LeanerIR.RuntimeFrame, ``LeanerIR.RuntimeValue,
        ``LeanerIR.Proofs.Codec.encode, ``LeanerIR.Proofs.Codec.decode?,
        ``LeanerIR.Proofs.NativeBoundary.finish] do
      if computation.getUsedConstants.contains retired then
        throwError "native mutable body retains {retired}"
      if post.getUsedConstants.contains retired then
        throwError "native mutable postcondition retains {retired}"
  for function in [`wrong_value, `wrong_old, `wrong_abort, `missing_overflow] do
    for suffix in [`nativeContract, `computation, `computationVerified, `computationRepresents,
        `computationBoundary, `typedVerified, `verified] do
      let name := `«0x42».native_mutable_generated ++ function ++ suffix
      if (← getEnv).contains name then throwError "failed mutable verification leaked {name}"

set_option maxHeartbeats 1000
open LeanerIR LeanerIR.Proofs SemanticOperations «0x42».native_mutable_generated

example (loan : Nat) (initial : RuntimeState) :
    (byte_max.computation ⟨⟨loan, ⟨0, by decide⟩⟩⟩).ok initial ⟨loan, ⟨255, by decide⟩⟩ initial :=
  ⟨rfl, rfl⟩

example (loan : Nat) (initial : RuntimeState) :
    ¬(set_seven.computation ⟨⟨loan, ⟨0, by decide⟩⟩⟩).ok initial
      ⟨loan, ⟨8, by decide⟩⟩ initial := by
  intro wrong
  have value := congrArg (fun owner => owner.value.val) wrong.1
  change (8 : Int) = 7 at value
  contradiction

example (executable : Validation.ExecutableUnit) (args : set_seven.Arguments)
    (initial : RuntimeState) (noGlobal : globalLoanKey? initial args.slot.loan = none) :
    (_denotation_dependencies.namespace0.function0.set_seven.denotation executable
      (set_seven.argumentsCodec.encode args)).ok initial #[]
        { initial with pending := initial.pending.push (args.slot.loan, .integer 7) } := by
  apply ((set_seven.computationRepresents executable args).ok _ _ _).mpr
  refine ⟨(), ⟨NativeMutation.write args.slot ⟨7, by decide⟩, initial,
    ⟨rfl, rfl⟩, rfl, ?_⟩, rfl⟩
  exact (set_seven.commit_eq (NativeMutation.write args.slot ⟨7, by decide⟩) initial noGlobal).symm

example (loan : Nat) (initial : RuntimeState) :
    ¬(set_seven.computation ⟨⟨loan, ⟨0, by decide⟩⟩⟩).ok initial
      ⟨loan + 1, ⟨7, by decide⟩⟩ initial := by
  intro wrong
  have identity := congrArg (fun owner => owner.loan) wrong.1
  change loan + 1 = loan at identity
  omega

example (executable : Validation.ExecutableUnit) (args : set_seven.Arguments)
    (initial : RuntimeState) (error : Failure) :
    ¬(_denotation_dependencies.namespace0.function0.set_seven.denotation executable
      (set_seven.argumentsCodec.encode args)).aborts initial error := by
  intro failed
  exact ((set_seven.computationRepresents executable args).aborts _ _).mp failed
