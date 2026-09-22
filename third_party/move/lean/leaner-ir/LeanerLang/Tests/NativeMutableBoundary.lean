-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
import LeanerIR.Proofs.NativeBoundaryAgreement
import LeanerIR.Proofs.NativeMutationAgreement
import Lean.Util.CollectAxioms

/-! Source-level pilot for typed ownership outputs. The native body/contract
and the runtime loan-export proof are deliberately separate artifacts. -/

set_option Elab.async false
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

open Lean Elab Command in
elab "boundary_cost " name:ident proof:command : command => do
  let qualified := (← getCurrNamespace) ++ name.getId
  LeanerLang.Perf.measure qualified.toString qualified (elabCommand proof)

leaner module 0x42::native_mutable_boundary where
  fun set_seven(slot : &mut u64) -> Unit := *slot := 7
  spec set_seven where
    ensures *slot == 7
    aborts_if false

  fun bad_boundary(value : u64) -> u64 := value
  spec bad_boundary where
    ensures result == value
    aborts_if false

#leaner_prepare 0x42::native_mutable_boundary::set_seven

namespace «0x42».native_mutable_boundary

open LeanerIR LeanerIR.Proofs SemanticOperations Denotation ComputationAgreement

abbrev Owner := MutableArgument (SpecInt (.bits 64) false)

def set_seven.computation (args : set_seven.Arguments) : Spec RuntimeState Failure Owner :=
  Spec.pure (NativeMutation.write args.slot ⟨7, by decide⟩)

def set_seven.nativeContract : Contract RuntimeState Failure set_seven.Arguments Owner where
  requires := set_seven.typedContract.requires
  ensures := fun args _ output _ => output.loan = args.slot.loan ∧ output.value.val = 7
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False

boundary_cost set_seven.computationVerified
theorem set_seven.computationVerified : Satisfies set_seven.computation set_seven.nativeContract := by
  apply satisfies_of_wp
  intro args initial permitted
  simp only [computation, nativeContract, wp_pure, NativeMutation.write]
  simp

-- Runtime frames and export are confined to the boundary/agreement artifacts.
def set_seven.exitFrame (output : Owner) : RuntimeFrame := {
  locals := #[some (.borrow output.loan (.integer output.value.val))]
  loanLocations := #[(output.loan, ⟨.local ⟨0⟩, #[], true⟩)] }

def set_seven.project (_ : Owner) : Unit := ()

def set_seven.commit (output : Owner) (state : RuntimeState) : RuntimeState :=
  exportReturnedFrameLoans #[] (set_seven.exitFrame output) state

boundary_cost set_seven.commit_eq
theorem set_seven.commit_eq (output : Owner) (state : RuntimeState)
    (noGlobal : globalLoanKey? state output.loan = none) :
    set_seven.commit output state =
      { state with pending := state.pending.push (output.loan, .integer output.value.val) } := by
  rw [commit, exportReturnedFrameLoans_noReturnedBorrows _ _ _ rfl]
  exact exportFrameLoans_singleInteger_state state output.loan output.value.val _ _ noGlobal

boundary_cost set_seven.computationBoundary
theorem set_seven.computationBoundary :
    NativeBoundary.Transports set_seven.nativeContract set_seven.typedContract
      set_seven.project set_seven.commit := by
  constructor
  · exact fun _ _ permitted => permitted
  · intro args initial output middle permitted post frame _
    have sameState : middle = initial := frame
    subst middle
    obtain ⟨sameLoan, seven⟩ := post (fun impossible => impossible)
    have noGlobal : globalLoanKey? initial args.slot.loan = none := by
      obtain ⟨loan, value, ⟨⟨⟨equal, _⟩, _⟩, _⟩, noGlobal⟩ := permitted
      have same : args.slot.loan = loan ∧ args.slot.value.val = value := by
        simpa [argumentsCodec, Codec.mutable, Codec.specInt] using equal
      rw [same.1]
      exact noGlobal
    rw [commit_eq output initial (sameLoan ▸ noGlobal)]
    constructor
    · intro _
      refine ⟨args.slot.loan, args.slot.value.val, 7, .integer 7,
        ⟨⟨⟨rfl, rfl⟩, ?_, (resolveReturnedBorrows_empty _).symm⟩, by decide⟩,
        Obligation_iff.mpr rfl⟩
      rw [sameLoan, seven]
    · exact ⟨⟨rfl, LoanDiscipline.of_eq rfl (Nat.le_refl _)⟩, by
        rintro ⟨_, _, _, impossible⟩; exact impossible⟩
  · exact fun _ _ _ _ impossible => False.elim impossible

boundary_cost set_seven.computationRepresents
theorem set_seven.computationRepresents (executable : Validation.ExecutableUnit) :
    NativeBoundary.Represents set_seven.argumentsCodec set_seven.resultsCodec
      set_seven.project set_seven.commit set_seven.computation
      (_denotation_dependencies.namespace0.function0.set_seven.denotation executable) := by
  intro args
  apply Spec.Equiv.trans
    (function_fromFrame _ _ _ _ _ (set_seven.exitFrame args.slot) ?_) ?_
  · simp [nativeInitialFrame?, SemanticOperations.initialLocals,
      set_seven.argumentsCodec, Codec.mutable, Codec.specInt,
      _denotation_dependencies.namespace0.function0.set_seven.denotationShape,
      set_seven.exitFrame, parameterLoanLocations_singleBorrow]
    rfl
  · apply fromFrame_boundary (exit := set_seven.exitFrame)
      (control := fun _ => .value .unit)
    · apply controlled_pure
      · apply nativeOperation_update
        · exact cons (read _ _ _ rfl) (cons (literal _ _) (nil _))
        · intro state
          exact NativeMutation.write_registered (Codec.specInt (.bits 64) false)
              args.slot ⟨7, by decide⟩ ⟨0⟩ (set_seven.exitFrame args.slot) state
              (by simp [localLoanPlace?, set_seven.exitFrame]) rfl
      · exact fun _ => trivial
    · exact fun _ => rfl
    · exact fun _ _ => rfl

-- Exercise the generated source denotation, not just the native owner helper.
theorem set_seven.execution (executable : Validation.ExecutableUnit)
    (args : set_seven.Arguments) (initial : RuntimeState)
    (noGlobal : globalLoanKey? initial args.slot.loan = none) :
    (_denotation_dependencies.namespace0.function0.set_seven.denotation executable
      (set_seven.argumentsCodec.encode args)).ok initial #[]
        { initial with pending := initial.pending.push (args.slot.loan, .integer 7) } := by
  apply ((set_seven.computationRepresents executable args).ok _ _ _).mpr
  refine ⟨(), ⟨NativeMutation.write args.slot ⟨7, by decide⟩, initial,
    ⟨rfl, rfl⟩, rfl, ?_⟩, rfl⟩
  exact (set_seven.commit_eq (NativeMutation.write args.slot ⟨7, by decide⟩) initial noGlobal).symm

example (executable : Validation.ExecutableUnit) (args : set_seven.Arguments)
    (initial : RuntimeState) (noGlobal : globalLoanKey? initial args.slot.loan = none) :
    ¬(_denotation_dependencies.namespace0.function0.set_seven.denotation executable
      (set_seven.argumentsCodec.encode args)).ok initial #[] initial := by
  intro ran
  obtain ⟨_, ⟨output, middle, executed, _, final⟩, _⟩ :=
    ((set_seven.computationRepresents executable args).ok _ _ _).mp ran
  have sameOutput : output = NativeMutation.write args.slot ⟨7, by decide⟩ := executed.1
  have sameState : middle = initial := executed.2
  subst output middle
  have sizes := congrArg (fun state => state.pending.size) final
  rw [set_seven.commit_eq (NativeMutation.write args.slot ⟨7, by decide⟩) initial noGlobal] at sizes
  simp only [Array.size_push] at sizes
  omega

example (executable : Validation.ExecutableUnit) (args : set_seven.Arguments)
    (initial : RuntimeState) (error : Failure) :
    ¬(_denotation_dependencies.namespace0.function0.set_seven.denotation executable
      (set_seven.argumentsCodec.encode args)).aborts initial error := by
  intro failed
  exact ((set_seven.computationRepresents executable args).aborts _ _).mp failed

end «0x42».native_mutable_boundary

#leaner_verify 0x42::native_mutable_boundary::set_seven
#leaner_require_native_all

-- Cached reuse must retain the same checked boundary transport.
#leaner_verify 0x42::native_mutable_boundary::set_seven

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for sample in samples do
    logInfo m!"{sample.target}: {sample.heartbeats} heartbeats / {sample.objects} objects"
  unless samples.size == 6 do throwError "missing native boundary proof-cost stages: {samples.size}"
  let heartbeats := samples.foldl (fun total sample => total + sample.heartbeats) 0
  let objects := samples.foldl (fun total sample => total + sample.objects) 0
  logInfo m!"set_seven: all native stages {heartbeats} heartbeats / {objects} objects"
  -- The complete source pipeline uses the same ceiling as NativeReferences;
  -- shared kernel lemmas retain their separate 1M per-declaration cap.
  if heartbeats > 10000000 || objects > 15000 then
    throwError "native mutable boundary exceeds 10M heartbeats / 15k objects"

open Lean Elab Command in
run_cmd do
  for name in [
      ``«0x42».native_mutable_boundary.set_seven.computationVerified,
      ``«0x42».native_mutable_boundary.set_seven.computationBoundary,
      ``«0x42».native_mutable_boundary.set_seven.computationRepresents,
      ``«0x42».native_mutable_boundary.set_seven.typedVerified,
      ``«0x42».native_mutable_boundary.set_seven.verified,
      ``LeanerIR.Proofs.NativeBoundary.wp_finish,
      ``LeanerIR.Proofs.NativeBoundary.finish_bind,
      ``LeanerIR.Proofs.NativeBoundary.Transports.satisfies,
      ``LeanerIR.Proofs.ComputationAgreement.fromFrame_boundary,
      ``LeanerIR.Proofs.ComputationAgreement.nativeOperation_update] do
    let axioms ← Lean.collectAxioms name
    if axioms.contains ``sorryAx then throwError "admission in {name}"

-- A named but ill-typed bridge must fail closed, even for an otherwise
-- supported identity body. It cannot select the state-preserving transport
-- instead or leave a published source theorem after the failure.
theorem «0x42».native_mutable_boundary.bad_boundary.computationBoundary : True := True.intro

#guard_msgs (drop error) in
#leaner_verify 0x42::native_mutable_boundary::bad_boundary

open Lean Elab Command in
run_cmd do
  for suffix in [`computation, `computationVerified, `computationRepresents,
      `nativeSummary, `typedVerified, `verified] do
    let name := `«0x42».native_mutable_boundary.bad_boundary ++ suffix
    if (← getEnv).contains name then throwError "invalid boundary leaked {name}"
