-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Contract
import LeanerIR.Proofs.Interpreter
import LeanerIR.Proofs.Completeness
import LeanerIR.Semantics.Typing

/-!
# Relational function meaning

The bridge from LIR execution to the contract calculus: a validated
function's relational `Spec` is read directly off the fuel-free big-step
judgment.  Successful executions are `returned` outcomes; failures are
`threw` outcomes with the final state quantified away, matching the
transaction boundary's rollback.  Nothing in the M1 executable subset is
`undefined` — unchecked proof obligations enter later with data invariants.
-/

namespace LeanerIR.Proofs

open LeanerIR.Validation

/-- An LIR failure outcome: the throw kind with its payload. -/
abbrev Failure := ThrowKind × Array RuntimeValue

/-- The contract shape of an LIR function: runtime state, throw failures,
argument and result value rows. -/
abbrev FunctionContract :=
  Contract RuntimeState Failure (Array RuntimeValue) (Array RuntimeValue)

/-- Relational meaning of one LIR function, derived from the big-step
judgment. -/
def functionSpec (unit : ExecutableUnit) (function : FunctionHandle)
    (arguments : Array RuntimeValue) :
    Spec RuntimeState Failure (Array RuntimeValue) where
  ok := fun initial results final =>
    BigStep.EvalFunction unit function #[] initial arguments final (.returned results)
  aborts := fun initial failure =>
    ∃ final,
      BigStep.EvalFunction unit function #[] initial arguments final
        (.threw failure.1 failure.2)

/-- The M1 executable subset owes no unchecked proofs. -/
@[simp] theorem functionSpec_total (unit : ExecutableUnit)
    (function : FunctionHandle) (arguments : Array RuntimeValue) :
    Spec.Total (functionSpec unit function arguments) :=
  fun _ obligation => obligation.elim

/-- A function satisfies a contract when its big-step meaning does.  The
well-formedness of global memory is not a separate side condition: a
generated contract's `requires` carries the typed representation of every
storable family, so the claim quantifies over well-typed stores through the
representation binders rather than through a typing judgment. -/
def SatisfiesFunction (unit : ExecutableUnit) (function : FunctionHandle)
    (contract : FunctionContract) : Prop :=
  Satisfies (functionSpec unit function) contract

/-- A returning interpreter run witnesses the relational success. -/
theorem functionSpec_ok_of_run {unit : ExecutableUnit} {fuel : Nat}
    {function : FunctionHandle} {arguments results : Array RuntimeValue}
    {state finalState : RuntimeState} {outcome : LocatedOutcome}
    (execution : LeanerIR.Interpreter.run unit fuel function arguments state
      = .ok (finalState, outcome))
    (returned : outcome.value = .returned results) :
    (functionSpec unit function arguments).ok state results finalState := by
  have sound := LeanerIR.Proofs.Interpreter.run_sound unit fuel function arguments state
    finalState outcome execution
  rw [returned] at sound
  exact sound

/-- A throwing interpreter run witnesses the relational failure. -/
theorem functionSpec_aborts_of_run {unit : ExecutableUnit} {fuel : Nat}
    {function : FunctionHandle} {arguments : Array RuntimeValue}
    {state finalState : RuntimeState} {outcome : LocatedOutcome}
    {kind : ThrowKind} {values : Array RuntimeValue}
    (execution : LeanerIR.Interpreter.run unit fuel function arguments state
      = .ok (finalState, outcome))
    (threw : outcome.value = .threw kind values) :
    (functionSpec unit function arguments).aborts state (kind, values) := by
  have sound := LeanerIR.Proofs.Interpreter.run_sound unit fuel function arguments state
    finalState outcome execution
  rw [threw] at sound
  exact ⟨finalState, sound⟩

/-- Every relational success is reached by the interpreter at some fuel. -/
theorem run_of_functionSpec_ok {unit : ExecutableUnit}
    {function : FunctionHandle} {arguments results : Array RuntimeValue}
    {state finalState : RuntimeState}
    (h : (functionSpec unit function arguments).ok state results finalState) :
    ∃ fuel outcome,
      LeanerIR.Interpreter.run unit fuel function arguments state
        = .ok (finalState, outcome) ∧
      outcome.value = .returned results :=
  Completeness.run_complete h

/-- Every relational failure is reached by the interpreter at some fuel. -/
theorem run_of_functionSpec_aborts {unit : ExecutableUnit}
    {function : FunctionHandle} {arguments : Array RuntimeValue}
    {state : RuntimeState} {failure : Failure}
    (h : (functionSpec unit function arguments).aborts state failure) :
    ∃ fuel finalState outcome,
      LeanerIR.Interpreter.run unit fuel function arguments state
        = .ok (finalState, outcome) ∧
      outcome.value = .threw failure.1 failure.2 := by
  obtain ⟨finalState, step⟩ := h
  obtain ⟨fuel, outcome, execution, value⟩ := Completeness.run_complete step
  exact ⟨fuel, finalState, outcome, execution, value⟩

/-- The relational meaning cannot both return and throw from one state:
big-step evaluation is deterministic. -/
theorem functionSpec_ok_aborts_exclusive {unit : ExecutableUnit}
    {function : FunctionHandle} {arguments results : Array RuntimeValue}
    {state finalState : RuntimeState} {failure : Failure}
    (ok : (functionSpec unit function arguments).ok state results finalState)
    (aborts : (functionSpec unit function arguments).aborts state failure) :
    False := by
  obtain ⟨threwState, threw⟩ := aborts
  obtain ⟨-, outcome_eq⟩ := Completeness.evalFunction_deterministic ok threw
  cases outcome_eq

end LeanerIR.Proofs
