-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-! # The model's frame step at an induced frame leaves no trace where it declines

A frame over a model's invocation meaning stated at the frame a call's type
arguments induce is carried to the outer frame by the meaning's coherence
(`FramedAtOf.of_instantiate`) and decided there. The attempt stands under one
saved state: where the coherence is not in the context, where the type
arguments are not reference-free, or where the continuation at the outer
frame decides nothing, the step declines and what it was asked about is left
as it was — the goal unassigned, the goals unchanged, and no metavariable it
found assigned — so the step that follows sees what it would have seen.

Each control states its goal in the form the step reads, and the command
checks that form before it runs the step: a frame over a meaning, at a frame
`Skolems.instantiate` builds, of a meaning taken at that frame. Those are the
step's own conditions for entering the attempt, so a control that passes the
check has reached it. The goal is a metavariable the command inspects; no
theorem is admitted and the conclusion is not among the context's facts, so
the step cannot answer by reading it back.
-/

namespace VerexIR.Tests.ModelFramePeel

open Lean Elab Command Term Meta Tactic

/-- What the step left: its answer and the goals before and after it ran. -/
private structure Ran where
  answer : Option (List MVarId) := none
  before : List MVarId := []
  after : List MVarId := []
  deriving Inhabited

/-- `#guard_model_frame declines in σ` states that the model's frame step
declines the goal `σ` concludes, in the context `σ` binds, leaving the goal
unassigned, the goals unchanged and every metavariable it found unassigned;
`applies` states that it answers and proves the goal. The goal must be one
the step reads, which the command checks first. -/
elab "#guard_model_frame " outcome:ident " in " statement:term : command => liftTermElabM do
  let declines ← match outcome.getId with
    | `declines => pure true
    | `applies => pure false
    | other => throwError "a control expects `declines` or `applies`, not `{other}`"
  let statement ← Term.elabType statement
  Term.synthesizeSyntheticMVarsNoPostponing
  forallTelescope statement fun _ conclusion => do
    -- The step's own conditions for entering the induced attempt
    -- (`routedClosureFrame?`): a frame over a meaning, at an induced frame,
    -- of a meaning taken there.
    let target := conclusion.consumeMData
    let framed ← if target.isAppOfArity ``VerexIR.Proofs.KeepsMemoryAtOf 8 then whnfR target
      else pure target
    unless framed.isAppOfArity ``VerexIR.Proofs.FramedAtOf 9 do
      throwError "the control states no frame over a meaning:{indentExpr target}"
    unless (framed.getArg! 2).consumeMData.isAppOfArity ``Skolems.instantiate 4 do
      throwError "the control's frame is not an induced one:{indentExpr (framed.getArg! 2)}"
    unless (framed.getArg! 3).consumeMData.isApp do
      throwError "the control's meaning is not taken at the frame:{indentExpr (framed.getArg! 3)}"
    let goal ← mkFreshExprSyntheticOpaqueMVar conclusion
    let id := goal.mvarId!
    let opened ← getMCtx
    let found := opened.decls.toList.filterMap fun (mvar, _) =>
      if (opened.eAssignment.find? mvar).isSome then none else some mvar
    let ran ← IO.mkRef (default : Ran)
    let _ ← Tactic.run id do
      let before ← getGoals
      let answer ← routedClosureFrame? (← getMainGoal)
      ran.set { answer, before, after := ← getGoals }
    let { answer, before, after } ← ran.get
    if declines then
      if let some remaining := answer then
        throwError "the step did not decline; it answered with {remaining.length} goal(s)"
      if ← id.isAssigned then
        throwError "the step assigned the goal it declined"
      unless after == before do
        throwError "the step left {after.length} goal(s) instead of the {before.length} it found"
      let assigned := (← getMCtx).eAssignment
      for mvar in found do
        if (assigned.find? mvar).isSome then
          throwError "the step assigned the metavariable `{mvar.name}` it found"
    else
      unless answer.isSome do throwError "the step declined"
      unless ← id.isAssigned do throwError "the step answered without proving the goal"

/-- Type arguments of one scalar: reference-free, so the coherence applies. -/
abbrev scalarArgs : TypeArgs := ⟨.cons (.int 64 false) .nil, rfl⟩

/-- Type arguments holding a reference: the coherence does not apply there. -/
abbrev borrowedArgs : TypeArgs := ⟨.cons (.ref (.int 64 false)) .nil, rfl⟩

/-- The row a value of one scalar takes. -/
abbrev scalarRow : NRow := .cons (.int 64 false) .nil

/-- One parameter passed by value, one result. -/
abbrev sharing : Sharing := ⟨[false], [false]⟩

/-- A result of one scalar. -/
abbrev scalarShape : ResultShape := .one (.int 64 false)

/-! ## Without the coherence

The law's premise has no hypothesis to take it from, so the step declines
before it builds anything. -/

#guard_model_frame declines in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (invoke : (Θ : Skolems unit) → @InvokeMeaning unit _ Θ) (Θ : Skolems unit)
    (value : @NTy.carrier (Skolems.instantiate scalarArgs Θ).toCarriers
      (.function scalarRow sharing scalarShape.row)),
    @KeepsMemoryAtOf unit _ (Skolems.instantiate scalarArgs Θ)
      (invoke (Skolems.instantiate scalarArgs Θ)) scalarRow sharing scalarShape value

/-! ## At type arguments that hold a reference

The coherence relates the frames at reference-free arguments. The premise is
decided in the step, so a row it does not cover declines there and no witness
of it is built. -/

#guard_model_frame declines in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (invoke : (Θ : Skolems unit) → @InvokeMeaning unit _ Θ) (Θ : Skolems unit)
    (_coherent : InvokeMeaning.Coherent invoke)
    (value : @NTy.carrier (Skolems.instantiate borrowedArgs Θ).toCarriers
      (.function scalarRow sharing scalarShape.row)),
    @KeepsMemoryAtOf unit _ (Skolems.instantiate borrowedArgs Θ)
      (invoke (Skolems.instantiate borrowedArgs Θ)) scalarRow sharing scalarShape value

/-! ## Without a routing at the outer frame

The peel applies and the continuation decides nothing of a value no site
constructs: the step restores what the peel changed and declines. -/

#guard_model_frame declines in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (invoke : (Θ : Skolems unit) → @InvokeMeaning unit _ Θ) (Θ : Skolems unit)
    (_coherent : InvokeMeaning.Coherent invoke)
    (value : @NTy.carrier (Skolems.instantiate scalarArgs Θ).toCarriers
      (.function scalarRow sharing scalarShape.row)),
    @KeepsMemoryAtOf unit _ (Skolems.instantiate scalarArgs Θ)
      (invoke (Skolems.instantiate scalarArgs Θ)) scalarRow sharing scalarShape value

end VerexIR.Tests.ModelFramePeel
