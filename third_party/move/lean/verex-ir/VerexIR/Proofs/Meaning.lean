-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Contract
import VerexIR.Proofs.Interpreter
import VerexIR.Proofs.Completeness
import VerexIR.Semantics.Typing
import VerexIR.Semantics.MapViews
import VerexIR.Semantics.StateTyping

/-!
# Relational function meaning

The bridge from VIR execution to the contract calculus: a validated
function's relational `Spec` is read directly off the fuel-free big-step
judgment.  Successful executions are `returned` outcomes; failures are
`threw` outcomes with the final state quantified away, matching the
transaction boundary's rollback.  Nothing in the M1 executable subset is
`undefined` — unchecked proof obligations enter later with data invariants.
-/

namespace VerexIR.Proofs

open VerexIR.Validation

/-- A runtime failure outcome: the throw kind with its payload. -/
abbrev RuntimeFailure := ThrowKind × Array RuntimeValue

/-- A failure outcome of the denotation: the throw kind with its integer
payload, an abort's code or the failed operation's operands. -/
abbrev Failure := ThrowKind × Array Int

/-- A failure of a throw kind with integer arguments. -/
def Failure.ofInts (kind : ThrowKind) (values : Array Int) : Failure := (kind, values)

/-- The runtime failure a failure is: its integers as runtime integers. -/
def Failure.encode (failure : Failure) : RuntimeFailure :=
  (failure.1, failure.2.map RuntimeValue.integer)

theorem Failure.encode_injective : Function.Injective Failure.encode := by
  intro left right same
  obtain ⟨kind, values⟩ := left
  obtain ⟨kind', values'⟩ := right
  simp only [Failure.encode, Prod.mk.injEq] at same
  obtain ⟨rfl, same⟩ := same
  refine Prod.ext rfl ?_
  exact (Array.map_inj_right fun _ _ equal => RuntimeValue.integer.inj equal).mp same

/-- The contract shape of an VIR function: runtime state, throw failures,
argument and result value rows. -/
abbrev FunctionContract :=
  Contract RuntimeState RuntimeFailure (Array RuntimeValue) (Array RuntimeValue)

/-- A typed representation of a function's inputs at an instantiation: a
typed start and arguments typed at the function's parameter types under
every environment faithful to the instantiation, normalizing to the inputs
(`SemanticOperations.normalize`). The runtime runs the Move representation
of every value; the function meaning reads its runs at the abstract values
the representation normalizes to (`doc/designs/intrinsic-maps.md`, "Abstract
representation"). -/
structure Represents (unit : ValidatedUnit) (function : FunctionHandle)
    (typeInstantiation : Array (TypeId × TypeId)) (start : RuntimeState)
    (arguments : Array RuntimeValue) (initial : RuntimeState) (inputs : Array RuntimeValue) :
    Prop where
  start_eq : start.normalize unit = initial
  arguments_eq : arguments.map (SemanticOperations.normalize unit) = inputs
  typed : ∃ (loans : LoanTypes) (inert : Nat), TypedState unit loans inert start ∧
    ∀ (ns : ValidatedNamespace) (declaration : FunctionDecl FunctionBody) (semantic : Array SemArg)
      (parameters results : List SemTy),
      unit.namespaces[function.namespaceId.index]? = some ns →
      ns.functions[function.functionId.index]? = some declaration →
      FrameInstantiation ns (requiredAt unit function) typeInstantiation
        (frameEnv declaration.signature.generics semantic) →
      StaticTyping.signatureTypes? ns declaration (frameEnv declaration.signature.generics semantic) =
        some (parameters, results) →
      HasTypes unit loans arguments.toList parameters

/-- A run of a function at an instantiation, read up to normalization: a
run from a typed representation of the start on a typed representation of
the arguments, its exit and outcome normalized. -/
def functionRuns {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (function : FunctionHandle) (typeInstantiation : Array (TypeId × TypeId))
    (initial : RuntimeState) (inputs : Array RuntimeValue) (final : RuntimeState)
    (outcome : Outcome) : Prop :=
  ∃ start arguments exit returned,
    Represents unit function typeInstantiation start arguments initial inputs ∧
    exit.normalize unit = final ∧ returned.normalize unit = outcome ∧
    BigStep.EvalFunction executable function typeInstantiation start arguments exit returned

/-- Relational meaning of one VIR function invoked under a type
instantiation: its runs (`functionRuns`), the returning ones as successes,
the throwing ones as failures with the final state quantified away. -/
def functionSpecAt {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (function : FunctionHandle)
    (typeInstantiation : Array (TypeId × TypeId)) (arguments : Array RuntimeValue) :
    Spec RuntimeState RuntimeFailure (Array RuntimeValue) where
  ok := fun initial results final =>
    functionRuns executable function typeInstantiation initial arguments final (.returned results)
  aborts := fun initial failure =>
    ∃ final, functionRuns executable function typeInstantiation initial arguments final
      (.threw failure.1 failure.2)

/-- Relational meaning of one VIR function, derived from the big-step
judgment. -/
def functionSpec {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (function : FunctionHandle)
    (arguments : Array RuntimeValue) :
    Spec RuntimeState RuntimeFailure (Array RuntimeValue) :=
  functionSpecAt executable function #[] arguments

/-- The M1 executable subset owes no unchecked proofs. -/
@[simp] theorem functionSpec_total {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (function : FunctionHandle) (arguments : Array RuntimeValue) :
    Spec.Total (functionSpec executable function arguments) :=
  fun _ obligation => obligation.elim

/-- A function satisfies a contract when its big-step meaning does.  The
well-formedness of global memory is not a separate side condition: a
generated contract's `requires` carries the typed representation of every
storable family, so the claim quantifies over well-typed stores through the
representation binders rather than through a typing judgment. -/
def SatisfiesFunction {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (function : FunctionHandle)
    (contract : FunctionContract) : Prop :=
  Satisfies (functionSpec executable function) contract

/-- A function satisfies a contract at a runtime type instantiation when its
big-step meaning there does: the runs of a generic function at the
instantiation a call gives it. -/
def SatisfiesFunctionAt {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (function : FunctionHandle) (typeInstantiation : Array (TypeId × TypeId))
    (contract : FunctionContract) : Prop :=
  Satisfies (functionSpecAt executable function typeInstantiation) contract

/-- A returning interpreter run from a typed representation witnesses the
relational success at the inputs it represents. -/
theorem functionSpec_ok_of_run {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {fuel : Nat}
    {function : FunctionHandle} {arguments inputs results : Array RuntimeValue}
    {state initial finalState : RuntimeState} {outcome : LocatedOutcome}
    (represents : Represents unit function #[] state arguments initial inputs)
    (execution : VerexIR.Interpreter.run executable fuel function arguments state
      = .ok (finalState, outcome))
    (returned : outcome.value = .returned results) :
    (functionSpec executable function inputs).ok initial
      (results.map (SemanticOperations.normalize unit)) (finalState.normalize unit) := by
  have sound := VerexIR.Proofs.Interpreter.run_sound executable fuel function arguments state
    finalState outcome execution
  rw [returned] at sound
  exact ⟨state, arguments, finalState, .returned results, represents, rfl, rfl, sound⟩

/-- A throwing interpreter run from a typed representation witnesses the
relational failure at the inputs it represents. -/
theorem functionSpec_aborts_of_run {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {fuel : Nat}
    {function : FunctionHandle} {arguments inputs : Array RuntimeValue}
    {state initial finalState : RuntimeState} {outcome : LocatedOutcome}
    {kind : ThrowKind} {values : Array RuntimeValue}
    (represents : Represents unit function #[] state arguments initial inputs)
    (execution : VerexIR.Interpreter.run executable fuel function arguments state
      = .ok (finalState, outcome))
    (threw : outcome.value = .threw kind values) :
    (functionSpec executable function inputs).aborts initial
      (kind, values.map (SemanticOperations.normalize unit)) := by
  have sound := VerexIR.Proofs.Interpreter.run_sound executable fuel function arguments state
    finalState outcome execution
  rw [threw] at sound
  exact ⟨finalState.normalize unit, state, arguments, finalState, .threw kind values, represents, rfl,
    rfl, sound⟩

/-- Every relational success is reached by the interpreter at some fuel,
from a typed representation of the start on one of the arguments. -/
theorem run_of_functionSpec_ok {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {function : FunctionHandle} {inputs results : Array RuntimeValue}
    {initial final : RuntimeState}
    (h : (functionSpec executable function inputs).ok initial results final) :
    ∃ start arguments exit returned fuel outcome,
      Represents unit function #[] start arguments initial inputs ∧
      exit.normalize unit = final ∧
      returned.map (SemanticOperations.normalize unit) = results ∧
      VerexIR.Interpreter.run executable fuel function arguments start = .ok (exit, outcome) ∧
      outcome.value = .returned returned := by
  obtain ⟨start, arguments, exit, outcome, represents, exitEq, outcomeEq, step⟩ := h
  obtain ⟨returned, rfl, resultsEq⟩ := Outcome.normalize_eq_returned outcomeEq
  obtain ⟨fuel, located, execution, value⟩ := Completeness.run_complete step
  exact ⟨start, arguments, exit, returned, fuel, located, represents, exitEq, resultsEq, execution,
    value⟩

/-- Every relational failure is reached by the interpreter at some fuel,
from a typed representation of the start on one of the arguments. -/
theorem run_of_functionSpec_aborts {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {function : FunctionHandle} {inputs : Array RuntimeValue}
    {initial : RuntimeState} {failure : RuntimeFailure}
    (h : (functionSpec executable function inputs).aborts initial failure) :
    ∃ start arguments payload fuel finalState outcome,
      Represents unit function #[] start arguments initial inputs ∧
      payload.map (SemanticOperations.normalize unit) = failure.2 ∧
      VerexIR.Interpreter.run executable fuel function arguments start = .ok (finalState, outcome) ∧
      outcome.value = .threw failure.1 payload := by
  obtain ⟨final, start, arguments, finalState, outcome, represents, -, outcomeEq, step⟩ := h
  obtain ⟨payload, rfl, payloadEq⟩ := Outcome.normalize_eq_threw outcomeEq
  obtain ⟨fuel, located, execution, value⟩ := Completeness.run_complete step
  exact ⟨start, arguments, payload, fuel, finalState, located, represents, payloadEq, execution,
    value⟩

end VerexIR.Proofs
