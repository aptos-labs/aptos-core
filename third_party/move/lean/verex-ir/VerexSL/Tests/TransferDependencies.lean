-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexIR.Tests.GivenExecution

/-! Explicit concrete-transfer dependencies propagate separately from trusted
lemma assumptions. The artifact audit rejects misplaced transfer premises
and Given proof metadata that does not describe a completed body proof. -/

open Lean Meta Elab Command
open VerexSL.Verify
namespace TransferPlumbingProbe

-- Alias the actual private producers for this isolated mechanics test.
run_cmd do
  for (user, aliasName) in [(`VerexSL.Verify.addTheoremDependencies, `collectDependencies),
      (`VerexSL.Verify.theoremPair, `calleePair),
      (`VerexSL.Verify.transferBinderOf, `transferBinder)] do
    let some (actual, _) := (← getEnv).constants.toList.find? fun (name, _) =>
      privateToUserName name == user
      | throwError "missing producer {user}"
    elabCommand (← `(command| abbrev $(mkIdent aliasName) := @$(mkIdent actual)))

def obligation : Prop := True
def otherObligation : Prop := True

theorem exported.typedSemanticsVerified {unit executable : Nat} (_prepared : unit = executable)
    {α : Type} (_value : α)
    (native_probe : ∀ {β : Type} (value : β), value = value)
    (assumptions_probe : ∀ {β : Type} (value : β), value = value)
    (trusted_TransferPlumbingProbe_otherObligation : otherObligation)
    (transfer_TransferPlumbingProbe_obligation : obligation) : True := by
  have := native_probe _value
  have := assumptions_probe _value
  have := trusted_TransferPlumbingProbe_otherObligation
  exact transfer_TransferPlumbingProbe_obligation

run_cmd do
  modifyEnv fun env => transferDependencies.addEntry env
    (`TransferPlumbingProbe.exported, #[``obligation])
  modifyEnv fun env => lemmaTrustDependencies.addEntry env
    (`TransferPlumbingProbe.exported, #[``otherObligation])
  let actual ← collectDependencies ``exported.typedSemanticsVerified #[] #[] #[] #[``obligation]
  unless actual.2.2.1 == #[``otherObligation] && actual.2.2.2 == #[``obligation] do
    throwError "dependency classes mixed or duplicates retained"
  modifyEnv fun env => nativeDependencies.addEntry env
    (`TransferPlumbingProbe.exported, #[(⟨⟨0⟩, ⟨0⟩⟩, "probe")])
  modifyEnv fun env => assumptionDependencies.addEntry env
    (`TransferPlumbingProbe.exported, #[(⟨⟨0⟩, ⟨0⟩⟩, "probe")])
  let binder ← transferBinder ``obligation
  let pair ← calleePair (← `(term| ())) "exported" ``exported.typedSemanticsVerified #[]
  let nativeIdent := mkIdent `native_probe
  let assumed := mkIdent `assumptions_probe
  let trusted := mkIdent (trustBinderName ``otherObligation)
  elabCommand (← `(command|
    theorem $(mkIdent `forwards_transfer) {unit executable : Nat} (prepared : unit = executable)
        ($nativeIdent:ident : ∀ {β : Type} (value : β), value = value)
        ($assumed:ident : ∀ {β : Type} (value : β), value = value)
        ($trusted:ident : otherObligation)
        $binder:bracketedBinder : ∀ (α : Type), α → True := ($pair).2.2))

#print axioms forwards_transfer
end TransferPlumbingProbe


set_option Elab.async false
open Lean Elab Command
open VerexSL.Verify

namespace TransferAuditProbe
verex module 0x42::transfer_audit where
  fun identity(value : u64) -> u64 := value
  spec identity where
    ensures result == value
    aborts_if false

set_option linter.defProp false in
def nonTheorem : True := True.intro
theorem wrongConclusion : True := True.intro
def obligation : Prop := True
def parameterized (_ : Nat) : Prop := True

open VerexIR.Proofs VerexIR.Proofs.Denote in
theorem conditionalBody [C : Carriers] {State : Type}
    (unit : VerexIR.Validation.ValidatedUnit) (ops : InterpreterOps unit C State)
    (_transfer : obligation) :
    VerexIR.Proofs.Satisfies
      ((VerexIR.Tests.GivenExecution.identity unit).denoteIn ops)
      (VerexIR.Tests.GivenExecution.identityContract State) :=
  VerexIR.Tests.GivenExecution.identity_verified unit ops

run_cmd do
  let base := `TransferAuditProbe.«0x42».transfer_audit.identity
  let baseline ← getEnv
  let reject (edit : Environment → Environment) (expected : String) : CommandElabM Unit := do
    setEnv (edit baseline)
    let message ← try
      requireNativeArtifacts base false
      pure "audit incorrectly accepted invalid metadata"
    catch e => pure (← e.toMessageData.toString)
    setEnv baseline
    unless (message.splitOn expected).length > 1 do
      throwError "expected '{expected}', received '{message}'"
  reject (fun env => givenProofArtifacts.addEntry env (base, ``nonTheorem))
    "has no completed Given body theorem"
  reject (fun env => givenProofArtifacts.addEntry env (base, ``wrongConclusion))
    "does not state satisfaction of the shared Given interpreter"
  reject (fun env => transferDependencies.addEntry env (base, #[``parameterized]))
    "must declare a parameterless Prop"
  reject (fun env => transferDependencies.addEntry env (base, #[``obligation]))
    "invalid public verification certificate"
  reject (fun env => givenProofArtifacts.addEntry
      (transferDependencies.addEntry env (base, #[``obligation])) (base, ``conditionalBody))
    "assumes a concrete transfer in its Given body proof"
  reject (fun env => givenProofArtifacts.addEntry env
      (base, ``VerexIR.Tests.GivenExecution.identity_verified))
    "does not use its registered Given body theorem"
  requireNativeArtifacts base false
end TransferAuditProbe

-- A reference in a separate compilation certificate cannot stand in for a
-- dependency of the concrete adapter's proof on the Given body theorem.
namespace MisleadingTransferArtifact
open VerexIR.Proofs.Denote
theorem typedVerified : @Term.denote = @Term.denote := rfl
theorem compiled : True := True.intro
theorem compiled_eq :
    @VerexIR.Tests.GivenExecution.identity_verified =
      @VerexIR.Tests.GivenExecution.identity_verified := rfl
end MisleadingTransferArtifact

/-- error: the concrete adapter for `MisleadingTransferArtifact` does not use its registered Given body theorem -/
#guard_msgs in
run_cmd do
  modifyEnv fun env => givenProofArtifacts.addEntry env
    (`MisleadingTransferArtifact, ``VerexIR.Tests.GivenExecution.identity_verified)
  requireNativeArtifacts `MisleadingTransferArtifact false
