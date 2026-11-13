-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Verify.GivenTransfer

open Lean Meta Elab Command VerexIR.Proofs VerexSL.Verify

namespace VerexSL.Tests.GivenTransferAdapter

def valueContract (State : Type) (value : Nat) : Contract State Nat Unit Nat where
  requires := fun _ _ => True
  ensures := fun _ _ result _ => result = value
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False

theorem givenPure (State : Type) (value : Nat) :
    Satisfies (fun _ : Unit => (Spec.pure value : Spec State Nat Nat))
      (valueContract State value) := by
  intro _ initial _ _
  refine ⟨?_, ?_, ?_⟩
  · intro result final execution
    exact ⟨fun _ => execution.1, execution.2, fun impossible => impossible⟩
  · intro _ impossible
    exact impossible.elim
  · intro impossible
    exact impossible

-- Operation premises are witnessed along with the interpretation, then passed
-- to the actual given proof. They are not synthesized from concrete validity.
theorem givenWithOperation (State : Type) (value : Nat)
    (operation : Unit → Spec State Nat Nat)
    (operationVerified : Satisfies operation (valueContract State value)) :
    Satisfies (fun args => Spec.bind (operation args) Spec.pure)
      (valueContract State value) := by
  simpa only [Spec.bind_pure] using operationVerified

run_cmd liftTermElabM do
  let concreteType ← Term.elabType (← `(term|
    ∀ (value : Nat) (concrete : Unit → Spec Unit Nat Nat),
      Satisfies concrete (valueContract Unit value)))
  declareGivenTransferAdapter `VerexSL.Tests.GivenTransferAdapter.givenTransfer
    `VerexSL.Tests.GivenTransferAdapter.concreteVerified concreteType 1
    (fun _ => pure (mkConst ``givenPure))
  declareGivenTransferAdapter `VerexSL.Tests.GivenTransferAdapter.operationTransfer
    `VerexSL.Tests.GivenTransferAdapter.operationConcreteVerified concreteType 1
    (fun _ => pure (mkConst ``givenWithOperation))

run_cmd do
  let definition ← getConstInfo `VerexSL.Tests.GivenTransferAdapter.givenTransfer
  unless definition.type == mkSort .zero do throwError "transfer assumption is not Prop"
  let some body := definition.value? | throwError "missing transfer assumption definition"
  unless body.getUsedConstants.contains ``Contract.RelationalCertificate do
    throwError "missing explicit relational certificate"
  if body.getUsedConstants.contains ``Satisfies then
    throwError "pure function export must not assume a target Satisfies proof"
  let theoremInfo ← getConstInfo `VerexSL.Tests.GivenTransferAdapter.concreteVerified
  liftTermElabM <| forallTelescope theoremInfo.type fun binders _ => do
    unless binders.size == 3 do throwError "wrong adapter binder count"
    unless (← inferType binders[1]!).isConstOf `VerexSL.Tests.GivenTransferAdapter.givenTransfer do
      throwError "transfer premise is not in requested position"
  for name in [`VerexSL.Tests.GivenTransferAdapter.concreteVerified,
      `VerexSL.Tests.GivenTransferAdapter.operationConcreteVerified] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected adapter axioms: {axioms}"
end VerexSL.Tests.GivenTransferAdapter
