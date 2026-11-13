-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Term
import VerexIR.Proofs.RelationalTransfer

/-!
Generic ownership-preserving execution through the shared interpreter, proved
simulation and concrete contract transfer, and rejection of a false ADT
invariant. The external operations are explicit throughout; these value-moving
bodies require no assumptions about them.
-/

open VerexIR.Proofs VerexIR.Proofs.Denote
namespace VerexIR.Tests.GivenExecution

-- Specification views and their ordering use the single abstract sort.
example (frame : GivenFrame) (i : Nat)
    (value : @NTy.carrier frame.toCarriers (.param i)) :
    @NTy.read frame.toCarriers (.param i) value = value := rfl
example (frame : GivenFrame) (i : Nat)
    [@CarriersNonempty frame.toCarriers]
    (value : @NTy.readCarrier frame.toCarriers (.param i)) :
    @NTy.unread frame.toCarriers _ (.param i) rfl value = value := rfl
example (frame : GivenFrame) (i : Nat) (rank : ValueRanks)
    (left right : @NTy.carrier frame.toCarriers (.param i)) :
    (@NTy.readOrder frame.toCarriers rank (.param i)).compare left right =
      @NTy.plainOrder frame.toCarriers rank (.param i) left right := rfl

-- Given parameters retain both API roles with one definitional representation.
example (frame : GivenFrame) (index : Nat) :
    @NTy.plain frame.toCarriers (.param index) =
      @NTy.carrier frame.toCarriers (.param index) := rfl

example (frame : GivenFrame) (index : Nat)
    (value : @NTy.carrier frame.toCarriers (.param index)) :
    @NTy.forget frame.toCarriers (.param index) value = value := rfl

example (frame : GivenFrame) (index : Nat)
    (value : @NTy.plain frame.toCarriers (.param index)) :
    @NTy.ofDeepHolds frame.toCarriers (.param index) value trivial = value := rfl

def identity (unit : VerexIR.Validation.ValidatedUnit) : Function unit where
  params := .cons (.param 0) .nil
  locals := .nil
  frame := .cons (.param 0) .nil
  frame_eq := rfl
  result := .one (.param 0)
  body := .take .here
  mutables := .nil
  exitSite := 0

def swapped (unit : VerexIR.Validation.ValidatedUnit) : Function unit where
  params := .cons (.param 0) (.cons (.param 1) .nil)
  locals := .nil
  frame := .cons (.param 0) (.cons (.param 1) .nil)
  frame_eq := rfl
  result := .one (.tuple (.cons (.param 1) (.cons (.param 0) .nil)))
  body := .tuple (.cons (.take (.there .here)) (.cons (.take .here) .nil))
  mutables := .nil
  exitSite := 0

set_option maxRecDepth 2000

theorem identity_denote [C : Carriers] {State : Type}
    (unit : VerexIR.Validation.ValidatedUnit) (ops : InterpreterOps unit C State)
    (value : NTy.carrier (.param 0)) :
    (identity unit).denoteIn ops (value, ()) = Spec.pure value := by
  simp [identity, Function.denoteIn, Term.denoteIn, initialEnv, Var.get, Var.clear,
    ResultShape.finish, Mutables.resolve, ResultShape.ofBody]

theorem swapped_denote [C : Carriers] {State : Type}
    (unit : VerexIR.Validation.ValidatedUnit) (ops : InterpreterOps unit C State)
    (left : NTy.carrier (.param 0)) (right : NTy.carrier (.param 1)) :
    (swapped unit).denoteIn ops (left, (right, ())) = Spec.pure (right, (left, ())) := by
  simp [swapped, Function.denoteIn, Term.denoteIn, Args.denoteIn, Flow.bind,
    initialEnv, Var.get, Var.clear, ResultShape.finish, Mutables.resolve, ResultShape.ofBody]
  rfl

-- The concrete adapter is the exact entry point used by existing verification.
theorem concrete_adapter {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    {executable : VerexIR.Validation.ExecutableUnit unit} [Skolems unit]
    (meanings : Meanings executable) (f : Function unit) (args : HList f.params) :
    f.denoteWith meanings args = f.denoteIn meanings.interpreterOps args := rfl

/-- A source-level identity contract at arbitrary certified parameter carriers. -/
def identityContract [Carriers] (State : Type) :
    Contract State Failure (HList (.cons (.param 0) .nil)) (NTy.carrier (.param 0)) where
  requires := fun _ _ => True
  ensures := fun args _ result _ => result = args.1
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False

theorem identity_verified [C : Carriers] {State : Type}
    (unit : VerexIR.Validation.ValidatedUnit) (ops : InterpreterOps unit C State) :
    Satisfies ((identity unit).denoteIn ops) (identityContract State) := by
  intro args initial _ _
  obtain ⟨value, tail⟩ := args
  cases tail
  rw [identity_denote]
  exact ⟨fun _ _ execution => ⟨fun _ => execution.1, execution.2, id⟩,
    fun _ impossible => impossible, fun impossible => impossible⟩

/-- The whole-value swap has no precondition about the representations of
its abstract parameters. The certified values travel intact. -/
def swappedContract [Carriers] (State : Type) :
    Contract State Failure (HList (.cons (.param 0) (.cons (.param 1) .nil)))
      (NTy.carrier (.tuple (.cons (.param 1) (.cons (.param 0) .nil)))) where
  requires := fun _ _ => True
  ensures := fun args _ result _ => result = (args.2.1, (args.1, ()))
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False

theorem swapped_verified [C : Carriers] {State : Type}
    (unit : VerexIR.Validation.ValidatedUnit) (ops : InterpreterOps unit C State) :
    Satisfies ((swapped unit).denoteIn ops) (swappedContract State) := by
  intro args initial _ _
  obtain ⟨left, right, tail⟩ := args
  cases tail
  rw [swapped_denote]
  exact ⟨fun _ _ execution => ⟨fun _ => execution.1, execution.2, id⟩,
    fun _ impossible => impossible, fun impossible => impossible⟩

/-- Concrete identity execution is related to actual Given execution by any
relation on parameter values, at any related input and output stores. There
is no admitted compiler-wide transfer premise. -/
theorem identity_refines {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    {executable : VerexIR.Validation.ExecutableUnit unit} [Θ : Skolems unit]
    (meanings : Meanings executable) (G : GivenFrame) {State : Type}
    (givenOps : InterpreterOps unit G.toCarriers State)
    (values : @NTy.carrier Θ.toCarriers (.param 0) → @NTy.carrier G.toCarriers (.param 0) → Prop)
    (stores : Memory unit → State → Prop) :
    FunctionRelationalRefines (fun args givenArgs => values args.1 givenArgs.1) stores values Eq
      (fun _ _ => True) ((identity unit).denoteWith meanings)
      (@Function.denoteIn unit G.toCarriers State givenOps (identity unit)) := by
  intro args givenArgs initial givenInitial related relatedStores _
  obtain ⟨value, tail⟩ := args
  obtain ⟨givenValue, givenTail⟩ := givenArgs
  cases tail
  cases givenTail
  rw [concrete_adapter, identity_denote]
  rw [@identity_denote G.toCarriers State unit givenOps givenValue]
  refine ⟨?_, ?_, ?_⟩
  · intro result final execution
    obtain ⟨rfl, rfl⟩ := execution
    exact ⟨givenValue, givenInitial, ⟨rfl, rfl⟩, related, relatedStores⟩
  · intro _ impossible
    exact False.elim impossible
  · intro impossible
    exact False.elim impossible

/-- Swap preserves both abstract value relations in reverse order, and
preserves the store relation. -/
theorem swapped_refines {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
    {executable : VerexIR.Validation.ExecutableUnit unit} [Θ : Skolems unit]
    (meanings : Meanings executable) (G : GivenFrame) {State : Type}
    (givenOps : InterpreterOps unit G.toCarriers State)
    (lefts : @NTy.carrier Θ.toCarriers (.param 0) → @NTy.carrier G.toCarriers (.param 0) → Prop)
    (rights : @NTy.carrier Θ.toCarriers (.param 1) → @NTy.carrier G.toCarriers (.param 1) → Prop)
    (stores : Memory unit → State → Prop) :
    FunctionRelationalRefines
      (fun args givenArgs => lefts args.1 givenArgs.1 ∧ rights args.2.1 givenArgs.2.1)
      stores (fun result givenResult => rights result.1 givenResult.1 ∧ lefts result.2.1 givenResult.2.1)
      Eq (fun _ _ => True) ((swapped unit).denoteWith meanings)
      (@Function.denoteIn unit G.toCarriers State givenOps (swapped unit)) := by
  intro args givenArgs initial givenInitial related relatedStores _
  obtain ⟨left, right, tail⟩ := args
  obtain ⟨givenLeft, givenRight, givenTail⟩ := givenArgs
  cases tail
  cases givenTail
  rw [concrete_adapter, swapped_denote]
  rw [@swapped_denote G.toCarriers State unit givenOps givenLeft givenRight]
  refine ⟨?_, ?_, ?_⟩
  · intro result final execution
    obtain ⟨rfl, rfl⟩ := execution
    exact ⟨(givenRight, (givenLeft, ())), givenInitial, ⟨rfl, rfl⟩,
      ⟨related.2, related.1⟩, relatedStores⟩
  · intro _ impossible
    exact False.elim impossible
  · intro impossible
    exact False.elim impossible

def emptyPack (unit : VerexIR.Validation.ValidatedUnit) (source : VerexIR.StructHandle) :
    Function unit where
  params := .nil
  locals := .nil
  frame := .nil
  frame_eq := rfl
  result := .one (.struct source .nil .nil)
  body := .pack 0 source .nil .nil .nil
  mutables := .nil
  exitSite := 0

/-- Trivial abstract parameter certification does not erase a declared
ADT invariant. A deliberately false nominal invariant still fails at pack. -/
theorem false_pack_undefined [C : Carriers] {State : Type}
    (unit : VerexIR.Validation.ValidatedUnit) (ops : InterpreterOps unit C State)
    (source : VerexIR.StructHandle)
    (broken : ¬ C.own source .nil (NTy.plainEncode (.struct source .nil .nil) ()))
    (initial : State) :
    ((emptyPack unit source).denoteIn ops ()).undefined initial := by
  simp only [emptyPack, Function.denoteIn, Term.denoteIn, Args.denoteIn,
    initialEnv, Flow.bind_pure_value, GhostInits.initialize, Spec.pure_bind]
  exact Or.inl (Or.inl (fun held => broken held.1))

theorem concrete_false_pack_undefined {unit : VerexIR.Validation.ValidatedUnit}
    [FunctionCarriers unit] {executable : VerexIR.Validation.ExecutableUnit unit}
    [Θ : Skolems unit] (meanings : Meanings executable) (source : VerexIR.StructHandle)
    (broken : ¬ Θ.toCarriers.own source .nil (NTy.plainEncode (.struct source .nil .nil) ()))
    (initial : Memory unit) :
    ((emptyPack unit source).denoteWith meanings ()).undefined initial := by
  rw [concrete_adapter]
  exact false_pack_undefined unit meanings.interpreterOps source broken initial

/-- The given parameter sort is the concrete certified sort for these
closed examples. Concrete temporary views remain on the concrete side of the
explicit relational transfer. -/
private def abstractParameters (C : Carriers) : GivenFrame where
  operations := C

private theorem same_contract_transfer {State Arg Result : Type}
    (contract : Contract State Failure Arg Result) :
    Contract.RelationalTransfer Eq Eq Eq Eq contract contract := by
  constructor <;> intros <;> subst_vars <;> assumption

/-- A concrete contract obtained from the verified Given identity through
the explicit, proved simulation. The external interface is arbitrary because
this body calls no external operation. -/
theorem identity_concrete_from_given {unit : VerexIR.Validation.ValidatedUnit}
    [FunctionCarriers unit] {executable : VerexIR.Validation.ExecutableUnit unit}
    [Θ : Skolems unit] (meanings : Meanings executable)
    (givenOps : InterpreterOps unit (abstractParameters Θ.toCarriers).toCarriers (Memory unit)) :
    Satisfies ((identity unit).denoteWith meanings) (identityContract (Memory unit)) := by
  have refines : FunctionRelationalRefines Eq Eq Eq Eq (fun _ _ => True)
      ((identity unit).denoteWith meanings)
      (@Function.denoteIn unit (abstractParameters Θ.toCarriers).toCarriers (Memory unit)
        givenOps (identity unit)) := by
    intro args givenArgs initial givenInitial related relatedStores admissible
    exact identity_refines meanings (abstractParameters Θ.toCarriers) givenOps Eq Eq
      args givenArgs initial givenInitial (congrArg Prod.fst related) relatedStores admissible
  apply Contract.satisfies_of_relationalRefines refines
    (same_contract_transfer (identityContract (Memory unit)))
    (fun _ _ _ _ => trivial) (fun args initial _ _ => ⟨args, initial, rfl, rfl⟩)
  exact @identity_verified (abstractParameters Θ.toCarriers).toCarriers (Memory unit) unit givenOps

/-- The same concrete export for whole-value swap, using the two separately
proved component relations. -/
theorem swapped_concrete_from_given {unit : VerexIR.Validation.ValidatedUnit}
    [FunctionCarriers unit] {executable : VerexIR.Validation.ExecutableUnit unit}
    [Θ : Skolems unit] (meanings : Meanings executable)
    (givenOps : InterpreterOps unit (abstractParameters Θ.toCarriers).toCarriers (Memory unit)) :
    Satisfies ((swapped unit).denoteWith meanings) (swappedContract (Memory unit)) := by
  have refines : FunctionRelationalRefines Eq Eq Eq Eq (fun _ _ => True)
      ((swapped unit).denoteWith meanings)
      (@Function.denoteIn unit (abstractParameters Θ.toCarriers).toCarriers (Memory unit)
        givenOps (swapped unit)) := by
    intro args givenArgs initial givenInitial related relatedStores admissible
    have proof := swapped_refines meanings (abstractParameters Θ.toCarriers) givenOps Eq Eq Eq
      args givenArgs initial givenInitial
      ⟨congrArg Prod.fst related, congrArg (fun pair => pair.2.1) related⟩ relatedStores admissible
    refine ⟨?_, proof.aborts, proof.undefined⟩
    intro result final execution
    obtain ⟨givenResult, givenFinal, runs, components, state⟩ := proof.ok result final execution
    refine ⟨givenResult, givenFinal, runs, ?_, state⟩
    exact Prod.ext components.1 (Prod.ext components.2 (@Subsingleton.elim Unit _ _ _))
  apply Contract.satisfies_of_relationalRefines refines
    (same_contract_transfer (swappedContract (Memory unit)))
    (fun _ _ _ _ => trivial) (fun args initial _ _ => ⟨args, initial, rfl, rfl⟩)
  exact @swapped_verified (abstractParameters Θ.toCarriers).toCarriers (Memory unit) unit givenOps

namespace ConcreteCompatibility
open VerexIR

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
  {executable : Validation.ExecutableUnit unit} [Θ : Skolems unit]
  (meanings : Meanings executable)

/-- The refactored generic-call boundary has exactly the previous concrete
transport on arguments and results, including all failure/undefined outcomes. -/
theorem generic_call {ρ : ResultShape} {Γ σs σs' : NRow}
    (site : Nat) (handle : FunctionHandle) (typeArgs : Array GenericArgument) (θ : TypeArgs)
    (shape shape' : ResultShape)
    (resolved : (NRow.subst θ.1 σs == σs' && shape.subst θ.1 == shape') = true)
    (arguments : Args unit ρ Γ σs') (env : HEnv Γ) :
    (Term.callGeneric site handle typeArgs θ shape shape' resolved arguments).denote meanings env =
      Flow.bind (arguments.denote meanings env) fun values env =>
        Spec.bind (callAt site (meanings.generic handle typeArgs θ σs shape
          (HList.toSkolem θ σs (cast (congrArg HList (resolvedCall resolved).1.symm) values))))
          fun result => Spec.pure (.value
            (cast ((congrArg ResultShape.carrier (resolvedCall resolved).2).trans
              shape'.carrier_eq_bodyType) (ResultShape.ofSkolem θ shape result)) env) := by
  simp only [Term.denote, Term.denoteIn, Args.denote, Meanings.interpreterOps,
    callAt, Spec.bind_assoc, Spec.pure_bind]

/-- A concrete global read still produces the same certified runtime view,
and aborts precisely when the resource is absent. -/
theorem global_read {ρ : ResultShape} {Γ : NRow} {κ τ : NTy}
    (site : Nat) (arguments : NRow) (key : Term unit ρ Γ κ) (env : HEnv Γ) :
    (Term.globalRead (τ := τ) site arguments key).denote meanings env =
      Flow.bind (key.denote meanings env) fun k env =>
        Spec.bind Spec.get fun memory =>
          match memory (Skolems.resource τ arguments) (κ.storageKey k) with
          | none => abortAt site (Failure.ofInts .abort #[])
          | some value => Spec.pure (.value (Skolems.ofRuntime τ value) env) := by
  simp only [Term.denote, Term.denoteIn, Meanings.interpreterOps]
  congr 1
  funext k env
  congr 1
  funext memory
  cases memory (Skolems.resource τ arguments) (κ.storageKey k) <;> rfl

end ConcreteCompatibility

-- This gate rejects a new global transfer axiom or an admitted test proof.
open Lean Elab Command in
run_cmd do
  for name in [``identity_verified, ``swapped_verified, ``identity_refines, ``swapped_refines,
      ``identity_concrete_from_given, ``swapped_concrete_from_given,
      ``false_pack_undefined, ``concrete_false_pack_undefined,
      ``ConcreteCompatibility.generic_call, ``ConcreteCompatibility.global_read] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end VerexIR.Tests.GivenExecution
