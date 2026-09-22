-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeBoundary
import LeanerIR.Proofs.NativeFlowAgreement

namespace LeanerIR.Proofs.ComputationAgreement

open SemanticOperations BigStep Denotation

set_option maxHeartbeats 1000

private theorem boundary_throw (count : Nat) (control : Control) (error : Failure)
    (ended : finishControl? count control = some (.threw error.1 error.2)) :
    control = .throw_ error.1 error.2 := by
  cases control with
  | value value =>
    cases equal : unpackFallthrough count value <;> simp [finishControl?, equal] at ended
  | return_ values =>
    simp only [finishControl?] at ended
    split at ended <;> cases ended
  | throw_ kind values => cases error; cases ended; rfl
  | break_ depth value => cases ended
  | continue_ depth => cases ended

/-- The native body returns its updated owners, and the execution agreement
relates those owners to the actual final frame. Loan export is proved here,
once, without becoming a native body operation or a native VC premise. -/
theorem fromFrame_boundary (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (body : ExprDenotation) (entry : RuntimeFrame)
    (exit : Output → RuntimeFrame) (control : Output → Control)
    (computation : Spec RuntimeState Failure Output)
    (project : Output → Result) (commit : Output → RuntimeState → RuntimeState)
    (codec : Codec Result (Array RuntimeValue))
    (agreement : Controlled body entry exit control computation)
    (finished : ∀ output, finishControl? shape.resultCount (control output) =
      some (.returned (codec.encode (project output))))
    (exported : ∀ output state,
      exportReturnedFrameLoans (codec.encode (project output)) (exit output) state = commit output state) :
    Spec.Equiv (fromFrame unit shape body entry)
      (encodeSpec codec (NativeBoundary.finish project commit computation)) := by
  constructor
  · intro initial result final
    constructor
    · rintro ⟨finalFrame, state, flow, executed, ended, finalState⟩
      have valid : NonThrow flow := by
        cases flow <;> try trivial
        cases ended
      obtain ⟨output, ran, sameFrame, sameControl⟩ :=
        (agreement.normal initial finalFrame state flow valid).mp executed
      subst finalFrame
      subst flow
      rw [finished] at ended
      cases ended
      have sameState : commit output state = final := by
        simpa only [finalizeFunctionState, exported] using finalState
      exact ⟨project output, ⟨output, state, ran, rfl, sameState.symm⟩, rfl⟩
    · rintro ⟨result, ⟨output, state, ran, rfl, rfl⟩, rfl⟩
      refine ⟨exit output, state, control output, ?_, finished output, ?_⟩
      · exact (agreement.normal _ _ _ _ (agreement.valid output)).mpr ⟨output, ran, rfl, rfl⟩
      · exact exported output state
  · intro initial error
    constructor
    · rintro ⟨frame, state, flow, executed, ended⟩
      have same := boundary_throw shape.resultCount flow error ended
      subst flow
      exact (agreement.aborts initial error).mp ⟨frame, state, executed⟩
    · intro aborted
      obtain ⟨frame, state, executed⟩ := (agreement.aborts initial error).mpr aborted
      exact ⟨frame, state, .throw_ error.1 error.2, executed, rfl⟩
  · intro initial
    constructor
    · exact False.elim
    · exact agreement.defined initial

end LeanerIR.Proofs.ComputationAgreement
