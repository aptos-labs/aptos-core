-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Computation

/-! A native body may return updated owners in addition to its source result.
Commit those owners only at the representation boundary. The native body and
its contract remain independent of the runtime loan/export representation. -/

namespace LeanerIR.Proofs.NativeBoundary

set_option maxHeartbeats 1000

/-- Successful completion projects the source result and commits the returned
owners to the boundary state. Aborts and undefined behavior are unchanged:
neither is allowed to execute a successful-return finalizer. -/
def finish (project : Output → Result) (commit : Output → State → State)
    (body : Spec State Error Output) : Spec State Error Result where
  ok initial result final := ∃ output middle,
    body.ok initial output middle ∧ result = project output ∧ final = commit output middle
  aborts := body.aborts
  undefined := body.undefined

theorem wp_finish (project : Output → Result) (commit : Output → State → State)
    (body : Spec State Error Output) (ensures : Result → State → Prop)
    (aborts : Error → Prop) (initial : State) :
    wp (finish project commit body) ensures aborts initial ↔
      wp body (fun output middle => ensures (project output) (commit output middle)) aborts initial := by
  constructor
  · rintro ⟨normal, failing, defined⟩
    exact ⟨fun output middle executed => normal _ _ ⟨output, middle, executed, rfl, rfl⟩,
      failing, defined⟩
  · rintro ⟨normal, failing, defined⟩
    refine ⟨?_, failing, defined⟩
    rintro result final ⟨output, middle, executed, rfl, rfl⟩
    exact normal output middle executed

/-- Finalization occurs once, after the final native continuation. It is not
inserted after each statement, iteration, or native owner update. -/
theorem finish_bind (project : Output → Result) (commit : Output → State → State)
    (first : Spec State Error Local) (next : Local → Spec State Error Output) :
    Spec.Equiv (finish project commit (Spec.bind first next))
      (Spec.bind first (fun value => finish project commit (next value))) := by
  constructor
  · intro initial result final
    constructor
    · rintro ⟨output, middle, ⟨value, earlier, one, two⟩, projected, committed⟩
      exact ⟨value, earlier, one, output, middle, two, projected, committed⟩
    · rintro ⟨value, earlier, one, output, middle, two, projected, committed⟩
      exact ⟨output, middle, ⟨value, earlier, one, two⟩, projected, committed⟩
  · intro initial error; rfl
  · intro initial; rfl

/-- A separately proved contract bridge. Native contracts speak about typed
outputs; only this agreement refers to boundary finalization. Frame and
must-abort obligations remain unconditional on successful executions. -/
structure Transports (native : Contract State Error Args Output)
    (target : Contract State Error Args Result)
    (project : Output → Result) (commit : Output → State → State) : Prop where
  requires : ∀ args initial, target.requires args initial → native.requires args initial
  normal : ∀ args initial output middle, target.requires args initial →
    (¬native.mayAbort args initial → native.ensures args initial output middle) →
    native.frame args initial middle → ¬native.mustAbort args initial →
    (¬target.mayAbort args initial → target.ensures args initial (project output) (commit output middle)) ∧
    target.frame args initial (commit output middle) ∧ ¬target.mustAbort args initial
  aborts : ∀ args initial error, target.requires args initial →
    native.aborts args initial error → target.aborts args initial error

theorem Transports.satisfies
    {native : Contract State Error Args Output} {target : Contract State Error Args Result}
    {project : Output → Result} {commit : Output → State → State}
    (bridge : Transports native target project commit)
    {body : Args → Spec State Error Output} (verified : Satisfies body native) :
    Satisfies (fun args => finish project commit (body args)) target := by
  intro args initial permitted
  obtain ⟨normal, failing, defined⟩ := verified args initial (bridge.requires args initial permitted)
  refine ⟨?_, ?_, defined⟩
  · rintro result final ⟨output, middle, executed, rfl, rfl⟩
    obtain ⟨post, frame, noAbort⟩ := normal output middle executed
    exact bridge.normal args initial output middle permitted post frame noAbort
  · intro error executed
    exact bridge.aborts args initial error permitted (failing error executed)

/-- Exact execution agreement, with updated owners retained by the native
body and committed only in the boundary adapter. This is not inferred from
contract satisfaction or from a successful result decoder. -/
def Represents (arguments : Codec Args RuntimeArgs) (results : Codec Result RuntimeResult)
    (project : Output → Result) (commit : Output → State → State)
    (body : Args → Spec State Error Output)
    (execution : RuntimeArgs → Spec State Error RuntimeResult) : Prop :=
  LeanerIR.Proofs.Represents arguments results
    (fun args => finish project commit (body args)) execution

theorem Represents.typed
    {arguments : Codec Args RuntimeArgs} {results : Codec Result RuntimeResult}
    {project : Output → Result} {commit : Output → State → State}
    {body : Args → Spec State Error Output}
    {execution : RuntimeArgs → Spec State Error RuntimeResult}
    (agreement : Represents arguments results project commit body execution) (args : Args) :
    Spec.Equiv (typedFunction arguments results execution args)
      (finish project commit (body args)) :=
  LeanerIR.Proofs.Represents.typed agreement args

theorem Represents.satisfies
    {arguments : Codec Args RuntimeArgs} {results : Codec Result RuntimeResult}
    {project : Output → Result} {commit : Output → State → State}
    {body : Args → Spec State Error Output}
    {execution : RuntimeArgs → Spec State Error RuntimeResult}
    (agreement : Represents arguments results project commit body execution)
    {native : Contract State Error Args Output} {target : Contract State Error Args Result}
    (bridge : Transports native target project commit) (verified : Satisfies body native) :
    Satisfies execution (target.runtime arguments results) :=
  LeanerIR.Proofs.Represents.satisfies agreement (bridge.satisfies verified)

end LeanerIR.Proofs.NativeBoundary
