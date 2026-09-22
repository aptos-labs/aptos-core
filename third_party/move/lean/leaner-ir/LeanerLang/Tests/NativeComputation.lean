-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
import LeanerIR.Proofs.ComputationAgreement

/-!
# Native-computation pilot: a generic body and its concrete caller

The computations and operation certificates remain explicit in this rule
pilot; `NativeGenerated` tests automatic generation. `#leaner_prepare` only
generates the execution denotations and authored contracts. Neither function
is first verified by the old route. The final `native` verifications are
fail-closed and transport these independently checked native proofs.

Every new proof stage is measured, including representation agreement.
The aggregate gate uses the existing generic performance ceilings, not a
newly relaxed baseline. The caller consumes the callee's native theorem and
its separate agreement certificate; it does not re-prove the callee body.
-/

open LeanerIR LeanerIR.Proofs LeanerIR.SemanticOperations
open LeanerIR.Proofs.Denotation
set_option Elab.async false
set_option maxHeartbeats 50000

#leaner_measure

open Lean Elab Command in
elab "native_cost " name:ident proof:command : command => do
  let qualified := (← getCurrNamespace) ++ name.getId
  LeanerLang.Perf.measure qualified.toString qualified (elabCommand proof)

leaner module 0x42::native_pilot where
  fun carry {T : type}(value : T) -> T := value
  spec carry where
    ensures result == value
    aborts_if false
  fun carry_u64(value : u64) -> u64 := core.call carry::<u64>(value)
  spec carry_u64 where
    ensures result == value
    aborts_if false

#leaner_prepare 0x42::native_pilot::carry
#leaner_prepare 0x42::native_pilot::carry_u64

namespace «0x42».native_pilot

def carry.computation {Carrier : Nat → Type} (args : carry.Arguments Carrier) :
    Spec RuntimeState Failure (Carrier 0) := Spec.pure args.value

native_cost carry.pureVerified
theorem carry.pureVerified {Carrier : Nat → Type}
    [carrierInhabited : ∀ i, Inhabited (Carrier i)]
    (codecs : ∀ i, Codec (Carrier i) RuntimeValue) (types : Array (TypeId × TypeId)) :
    Satisfies carry.computation (carry.typedContract types codecs).withStateFrame := by
  apply satisfies_of_wp
  intro arguments initial permitted
  simp only [Contract.withStateFrame, carry.computation, wp_pure, carry.typedContract, Contract.typed,
    carry.rawContract, carry.argumentsCodec, carry.resultsCodec] at permitted ⊢
  leaner_certified_close!

native_cost carry.computationVerified
theorem carry.computationVerified {Carrier : Nat → Type}
    [carrierInhabited : ∀ i, Inhabited (Carrier i)]
    (codecs : ∀ i, Codec (Carrier i) RuntimeValue) (types : Array (TypeId × TypeId)) :
    Satisfies carry.computation (carry.typedContract types codecs) :=
  satisfies_of_stateFrame (carry.pureVerified codecs types)

native_cost carry.computationRepresents
theorem carry.computationRepresents {Carrier : Nat → Type}
    (codecs : ∀ i, Codec (Carrier i) RuntimeValue) (types : Array (TypeId × TypeId))
    (executable : Validation.ExecutableUnit) :
    Represents (carry.argumentsCodec codecs) (carry.resultsCodec codecs)
      carry.computation
      (_denotation_dependencies.namespace0.function0.carry.denotation executable types) := by
  intro args
  apply Spec.Equiv.trans ?_ (encodeSpec_pure (carry.resultsCodec codecs) args.value).symm
  apply ComputationAgreement.function (entry := {
    locals := #[some ((codecs 0).encode args.value)]
    loanLocations := parameterLoanLocations #[(codecs 0).encode args.value]
    typeInstantiation := types })
  · simp [nativeInitialFrame?, SemanticOperations.initialLocals, carry.argumentsCodec,
      _denotation_dependencies.namespace0.function0.carry.denotationShape]
    rfl
  · apply ComputationAgreement.move
    · rfl
    · simp
  · rfl
  · simp [frameBorrows]

def carry_u64.computation (args : carry_u64.Arguments) :
    Spec RuntimeState Failure (SpecInt (.bits 64) false) :=
  carry.computation (Carrier := fun _ => SpecInt (.bits 64) false) ⟨args.value⟩

native_cost carry_u64.computationVerified
theorem carry_u64.computationVerified :
    Satisfies carry_u64.computation carry_u64.typedContract := by
  apply satisfies_of_wp
  intro arguments initial permitted
  simp only [carry_u64.typedContract, Contract.typed, carry_u64.rawContract,
    carry_u64.argumentsCodec, Codec.specInt] at permitted
  obtain ⟨input, inputFacts, fresh⟩ := permitted
  have verified := carry.pureVerified (Carrier := fun _ => SpecInt (.bits 64) false)
    (carrierInhabited := fun _ => ⟨⟨0, by decide⟩⟩)
    (fun _ => Codec.specInt (.bits 64) false) #[]
  have call := wp_of_satisfies (args := ⟨arguments.value⟩) (initial := initial) verified
    ⟨arguments.value, ⟨rfl, fresh⟩, by dsimp only [Codec.specInt]; leaner_plain⟩
    (by rintro ⟨_, _, impossible⟩; exact impossible)
  apply wp_mono call
  · intro result final established
    clear call verified
    obtain ⟨ensures, same, frame⟩ := established
    subst final
    simp only [Contract.withStateFrame, carry.typedContract, Contract.typed, carry.rawContract,
      carry.argumentsCodec, carry.resultsCodec, carry_u64.typedContract,
      carry_u64.rawContract, carry_u64.argumentsCodec, carry_u64.resultsCodec,
      Codec.specInt] at ensures frame ⊢
    simp only [Obligation_iff] at ensures
    obtain ⟨prior, out, ⟨⟨argEq, resultEq⟩, _⟩, equal⟩ := ensures
    have sameInput : out = prior := SpecInt.ext (RuntimeValue.integer.inj equal)
    subst out
    have same : result.val = arguments.value.val := by
      simp only [Array.mk.injEq, List.cons.injEq, RuntimeValue.integer.injEq,
        and_true] at argEq resultEq
      exact resultEq.trans argEq.symm
    have sameNative : result = arguments.value := SpecInt.ext same
    subst result
    leaner_certified_close!
  · intro error impossible
    clear call verified
    simp [Contract.withStateFrame, carry.typedContract, Contract.typed, carry.rawContract,
      Obligation_iff] at impossible

native_cost carry_u64.computationRepresents
theorem carry_u64.computationRepresents (executable : Validation.ExecutableUnit) :
    Represents carry_u64.argumentsCodec carry_u64.resultsCodec
      carry_u64.computation
      (_denotation_dependencies.namespace0.function1.carry_u64.denotation executable) := by
  intro args
  apply Spec.Equiv.trans ?_ (encodeSpec_pure carry_u64.resultsCodec args.value).symm
  apply ComputationAgreement.function (entry := {
    locals := #[some (.integer args.value.val)]
    loanLocations := parameterLoanLocations #[.integer args.value.val] })
  · simp [nativeInitialFrame?, SemanticOperations.initialLocals, carry_u64.argumentsCodec, Codec.specInt,
      _denotation_dependencies.namespace0.function1.carry_u64.denotationShape]
    rfl
  · apply ComputationAgreement.call
    · exact ComputationAgreement.cons (ComputationAgreement.read _ _ _ rfl)
        (ComputationAgreement.nil _)
    · intro initial final outcome
      apply ComputationAgreement.relation
      exact (carry.computationRepresents (Carrier := fun _ => SpecInt (.bits 64) false)
        (fun _ => Codec.specInt (.bits 64) false) _ executable ⟨args.value⟩).trans
        (encodeSpec_pure _ args.value)
  · rfl
  · simp [frameBorrows, outermostBorrows, collectPruned, borrowEntry?]

end «0x42».native_pilot

set_option leaner.route "native" in
#leaner_verify 0x42::native_pilot::carry

set_option leaner.route "native" in
#leaner_verify 0x42::native_pilot::carry_u64

-- Repeating a native verification is allowed; reusing another route is not
-- (the negative route fixture checks that distinction).
set_option leaner.route "native" in
#leaner_verify 0x42::native_pilot::carry_u64

-- Audit the locally generated dependency closure, not just the top-level
-- theorem. Runtime representation is allowed in the authored contract
-- adapter, but execution/frame agreement must not leak into native VCs.
open Lean Elab Command in
run_cmd do
  let artifactRoot := `«0x42».native_pilot
  for (root, nativeValues) in [
      (artifactRoot ++ `carry.computation, true),
      (artifactRoot ++ `carry_u64.computation, true),
      (artifactRoot ++ `carry.pureVerified, false),
      (artifactRoot ++ `carry.computationVerified, false),
      (artifactRoot ++ `carry_u64.computationVerified, false)] do
    let mut pending := #[root]
    let mut visited : Array Name := #[]
    while let some name := pending.back? do
      pending := pending.pop
      if visited.contains name then continue
      visited := visited.push name
      let some declaration := (← getEnv).find? name | throwError "missing native artifact {name}"
      let constants := declaration.type.getUsedConstants ++
        ((declaration.value? (allowOpaque := true)).map (·.getUsedConstants)).getD #[]
      for dependency in constants do
        if dependency == ``LeanerIR.RuntimeFrame ||
            dependency == ``LeanerIR.Proofs.typedFunction ||
            dependency == ``LeanerIR.Proofs.decodeSpec ||
            (`LeanerIR.Proofs.Denotation).isPrefixOf dependency ||
            dependency.getString! == "computationRepresents" ||
            (nativeValues && dependency == ``LeanerIR.RuntimeValue) then
          throwError "native artifact {root} depends on execution representation through {dependency}"
        if artifactRoot.isPrefixOf dependency then pending := pending.push dependency

-- Include the native proof, exact agreement, typed adapter and public
-- transport. Measuring just the adapter would conceal the real cost.
open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for sample in samples do
    logInfo m!"{sample.target}: {sample.heartbeats} heartbeats / {sample.objects} objects"
  let baseline := LeanerLang.Perf.parseBaseline
    (← IO.FS.readFile "LeanerLang/Tests/Performance.exp")
  for function in ["carry", "carry_u64"] do
    let proofPrefix := s!"«0x42».native_pilot.{function}."
    let routePrefix := s!"«0x42».native_pilot::{function} "
    let selected := samples.filter fun sample =>
      sample.target.startsWith proofPrefix || sample.target.startsWith routePrefix
    let expected := if function == "carry" then 5 else 4
    unless selected.size == expected do
      throwError "missing cost stages for {function}: expected {expected}, got {selected.size}"
    let heartbeats := selected.foldl (fun total sample => total + sample.heartbeats) 0
    let objects := selected.foldl (fun total sample => total + sample.objects) 0
    let previous := baseline.filter (·.1.startsWith s!"«0x42».perf_generics::{function} ")
    unless previous.size == 2 do throwError "missing original performance ceilings for {function}"
    let heartbeatLimit := previous.foldl (fun total entry => total + entry.2.1) 0
    let objectLimit := previous.foldl (fun total entry => total + entry.2.2) 0
    logInfo m!"{function}: all native stages {heartbeats} heartbeats / {objects} objects; \
      original ceiling {heartbeatLimit} / {objectLimit}"
    if heartbeats > heartbeatLimit || objects > objectLimit then
      throwError "native computation costs exceed the original ceiling for {function}"
