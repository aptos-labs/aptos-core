-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Compile

/-!
# Agreement of the denotation with the big-step semantics

The one theorem of `designs/denotation.md`: for every function the compiler
accepts, the denotation of its term is exactly its prophetic big-step
meaning, with references as `(current, prophecy)`.  It is stated over the compiler, so it holds for every
unit at once; a verified function transports its native theorem through it
to the authoritative semantics without any proof of its own.

**Status: assumed.**  By the user's decision of 2026-09-08 the induction
over the compiler's fuel is postponed and the statement is an explicit,
named axiom until then, so that `#print axioms` names it on every theorem
that depends on it.  Nothing else in the denotation is admitted.
-/

namespace LeanerIR.Proofs.Denote

open LeanerIR.Validation

variable [Skolems]

/-- Exact agreement of a compiled function's denotation with its prophetic
big-step meaning (`propheticMeaning`): the big-step run from any admissible
start with the same globals, the argument references lent under loans, and
each reference's prophecy its resolved export, at every skolem family and
type instantiation.  To be proved by induction on the fuel of
`compileExpr`, one case per term constructor. -/
axiom compileFunction_agrees (unit : ExecutableUnit) (handle : FunctionHandle)
    (f : Function) (compiled : compileFunction unit.unit handle = .ok f)
    (typeInstantiation : Array (TypeId × TypeId)) (args : HList f.params) :
    Spec.Equiv (f.denote unit typeInstantiation args)
      (propheticMeaning unit typeInstantiation handle f.params f.result args)

/-- The prophetic meanings of the compiled members of a cycle of calls are
the least fixed point of their bodies, with the calls to the members routed
to the argument: every outcome is one of a finite unfolding, since a
big-step derivation nests finitely many calls.  Assumed with the agreement
above (`designs/denotation.md`, D6). -/
axiom compileFunction_least_cycle (unit : ExecutableUnit)
    (members : List (FunctionHandle × Function))
    (compiled : ∀ index : CycleIndex members,
      compileFunction unit.unit index.member.1 = .ok index.member.2)
    (typeInstantiation : Array (TypeId × TypeId)) (index : CycleIndex members)
    (args : HList index.member.2.params) :
    Spec.Refines
      (propheticMeaning unit typeInstantiation index.member.1 index.member.2.params
        index.member.2.result args)
      (Spec.fixFamily (Index := CycleIndex members)
        (Args := fun index => HList index.member.2.params)
        (Result := fun index => index.member.2.result.carrier)
        (fun self index => index.member.2.denoteWith
          ⟨cycleMeaning unit members self, closedGeneric unit typeInstantiation,
            typeInstantiation⟩)
        index args)

/-- The prophetic meaning of a compiled generic function, at every skolem
family and type instantiation, is the least fixed point of its body over
all of them, with its own calls, with or without type arguments, routed
to the argument at the family and instantiation the call induces.
Assumed with the agreement above (`designs/denotation.md`, D6). -/
axiom compileFunction_least_generic (unit : ExecutableUnit) (handle : FunctionHandle)
    (f : Function) (compiled : compileFunction unit.unit handle = .ok f)
    (typeInstantiation : Array (TypeId × TypeId)) (args : HList f.params) :
    Spec.Refines (propheticMeaning unit typeInstantiation handle f.params f.result args)
      (Spec.fixFamily (Index := Skolems × Array (TypeId × TypeId))
        (Args := fun index => @HList index.1 f.params)
        (Result := fun index => @ResultShape.carrier index.1 f.result)
        (fun self index => @Function.denoteWith index.1
          (@Meanings.mk index.1 (@recursiveMeaning index.1 unit handle f.params f.result (self index))
            (@recursiveGeneric index.1 unit handle f.params f.result
              (fun Θ instantiation => self (Θ, instantiation)) index.2)
            index.2) f)
        (‹Skolems›, typeInstantiation) args)

omit [Skolems] in
/-- Equivalent computations have the same weakest precondition. -/
theorem wp_congr_equiv {σ ε α : Type} {a b : Spec σ ε α} (equiv : Spec.Equiv a b)
    (ensures : α → σ → Prop) (aborts : ε → Prop) (initial : σ) :
    wp a ensures aborts initial ↔ wp b ensures aborts initial := by
  simp only [wp]
  rw [(equiv.undefined initial)]
  constructor
  · rintro ⟨normal, failing, defined⟩
    exact ⟨fun r f step => normal r f ((equiv.ok _ _ _).mpr step),
      fun e step => failing e ((equiv.aborts _ _).mpr step), defined⟩
  · rintro ⟨normal, failing, defined⟩
    exact ⟨fun r f step => normal r f ((equiv.ok _ _ _).mp step),
      fun e step => failing e ((equiv.aborts _ _).mp step), defined⟩

/-- A call to a compiled callee reasons over the callee's denotation: an
unspecified callee, or one returning a reference, is inlined by the
agreement theorem. -/
theorem wp_propheticMeaning_of_compiled (unit : ExecutableUnit)
    (typeInstantiation : Array (TypeId × TypeId)) (handle : FunctionHandle)
    (f : Function) (compiled : compileFunction unit.unit handle = .ok f) (values : HList f.params)
    (ensures : f.result.carrier → RuntimeState → Prop) (aborts : Failure → Prop)
    (initial : RuntimeState) :
    wp (propheticMeaning unit typeInstantiation handle f.params f.result values) ensures aborts
        initial ↔
      wp (f.denote unit typeInstantiation values) ensures aborts initial :=
  (wp_congr_equiv (compileFunction_agrees unit handle f compiled typeInstantiation values)
    ensures aborts initial).symm

/-- A contract established over the denotation holds of the prophetic
meaning, which is what a caller's call denotes. -/
theorem satisfies_propheticMeaning (unit : ExecutableUnit)
    (typeInstantiation : Array (TypeId × TypeId)) (handle : FunctionHandle)
    (f : Function) (compiled : compileFunction unit.unit handle = .ok f)
    (contract : Contract RuntimeState Failure (HList f.params) f.result.carrier)
    (verified : Satisfies (f.denote unit typeInstantiation) contract) :
    Satisfies (propheticMeaning unit typeInstantiation handle f.params f.result) contract :=
  (satisfies_congr (compileFunction_agrees unit handle f compiled typeInstantiation) contract).mp
    verified

/-- Contracts established over the bodies of a cycle's members, each
assuming every member's contract of the calls to the members, hold of their
prophetic meanings. -/
theorem satisfies_cycle (unit : ExecutableUnit) (typeInstantiation : Array (TypeId × TypeId))
    (members : List (FunctionHandle × Function))
    (compiled : ∀ index : CycleIndex members,
      compileFunction unit.unit index.member.1 = .ok index.member.2)
    (contracts : (index : CycleIndex members) →
      Contract RuntimeState Failure (HList index.member.2.params) index.member.2.result.carrier)
    (verified : ∀ self : CycleSelves members, (∀ index, Satisfies (self index) (contracts index)) →
      ∀ index, Satisfies
        (index.member.2.denoteWith
          ⟨cycleMeaning unit members self, closedGeneric unit typeInstantiation,
            typeInstantiation⟩)
        (contracts index))
    (index : CycleIndex members) :
    Satisfies
      (propheticMeaning unit typeInstantiation index.member.1 index.member.2.params
        index.member.2.result)
      (contracts index) :=
  satisfies_of_refines (compileFunction_least_cycle unit members compiled typeInstantiation index)
    (satisfies_fixFamily _ contracts verified index)

/-- A contract established over the body of a generic function calling
itself, at every family and instantiation, assuming it of the calls to
itself at the family and instantiation each induces, holds of its
prophetic meaning. -/
theorem satisfies_recursive_generic (unit : ExecutableUnit) (handle : FunctionHandle)
    (f : Function) (compiled : compileFunction unit.unit handle = .ok f)
    (contract : (Θ : Skolems) → (instantiation : Array (TypeId × TypeId)) →
      Contract RuntimeState Failure (@HList Θ f.params) (@ResultShape.carrier Θ f.result))
    (verified : ∀ self : SelfFamily f.params f.result,
      (∀ Θ instantiation, Satisfies (self Θ instantiation) (contract Θ instantiation)) →
      ∀ (Θ : Skolems) (instantiation : Array (TypeId × TypeId)),
        Satisfies (@Function.denoteWith Θ
          (@Meanings.mk Θ (@recursiveMeaning Θ unit handle f.params f.result (self Θ instantiation))
            (@recursiveGeneric Θ unit handle f.params f.result self instantiation)
            instantiation) f)
          (contract Θ instantiation))
    (typeInstantiation : Array (TypeId × TypeId)) :
    Satisfies (propheticMeaning unit typeInstantiation handle f.params f.result)
      (contract ‹Skolems› typeInstantiation) :=
  satisfies_of_refines (compileFunction_least_generic unit handle f compiled typeInstantiation)
    (satisfies_fixFamily _ (fun index => contract index.1 index.2)
      (fun recursive hypothesis index =>
        verified (fun Θ instantiation => recursive (Θ, instantiation))
          (fun Θ instantiation => hypothesis (Θ, instantiation)) index.1 index.2)
      (‹Skolems›, typeInstantiation))

/-- The runtime form of a contract over native arguments.  A runtime call
lends the argument references under admissible loans, and the contract
holds for every prophecy of theirs.  A successful execution satisfies it at
the value view: each returned reference's prophecy is its current value,
and each argument reference's prophecy is its export with the returned
references' holes filled by their current values. -/
def _root_.LeanerIR.Proofs.Contract.prophetic (σs : NRow) (shape : ResultShape)
    (contract : Contract RuntimeState Failure (HList σs) shape.carrier) : FunctionContract where
  requires := fun arguments initial =>
    (∃ loans args, Admissible initial loans ∧ lendArguments σs args loans = some arguments) ∧
      ∀ loans args, lendArguments σs args loans = some arguments → contract.requires args initial
  ensures := fun arguments initial results final =>
    ∃ loans args result returnedLoans,
      lendArguments σs args loans = some arguments ∧
      shape.lend false result returnedLoans = some results ∧
      shape.lend true result returnedLoans = some results ∧
      argumentsResolve σs args loans results (exportsAfter initial.pending final.pending) ∧
      contract.ensures args initial result (final.withLoansOf initial)
  aborts := fun arguments initial error =>
    ∃ loans args, lendArguments σs args loans = some arguments ∧ contract.aborts args initial error
  mayAbort := fun arguments initial =>
    ∃ loans args, lendArguments σs args loans = some arguments ∧ contract.mayAbort args initial
  mustAbort := fun arguments initial =>
    ∀ loans args, lendArguments σs args loans = some arguments → contract.mustAbort args initial
  frame := fun arguments initial final =>
    ∃ loans args, lendArguments σs args loans = some arguments ∧
      contract.frame args initial (final.withLoansOf initial)

/-- A contract satisfied by the prophetic meaning holds of the big-step
meaning in its runtime form.  Every runtime execution from an admissible
start is a prophetic outcome at the value view, whose existence the
contract's definedness guarantees. -/
theorem satisfies_prophetic (unit : ExecutableUnit) (handle : FunctionHandle) (σs : NRow)
    (shape : ResultShape) (contract : Contract RuntimeState Failure (HList σs) shape.carrier)
    (verified : Satisfies (propheticMeaning unit #[] handle σs shape) contract) :
    SatisfiesFunction unit handle (contract.prophetic σs shape) := by
  intro arguments initial permitted
  obtain ⟨⟨loans, args, admissible, lent⟩, required⟩ := permitted
  have established := verified args initial (required loans args lent)
  refine ⟨?_, ?_, fun obligation => obligation⟩
  · intro results final execution
    have view : ∃ result returnedLoans, ∃ resolved : HList σs,
        shape.lend false result returnedLoans = some results ∧
        shape.lend true result returnedLoans = some results ∧
        lendArguments σs resolved loans = some arguments ∧
        argumentsResolve σs resolved loans results (exportsAfter initial.pending final.pending) := by
      refine Classical.byContradiction fun none => established.2.2 ?_
      exact ⟨initial, loans, arguments, results, final, rfl, admissible, lent, execution, none⟩
    obtain ⟨result, returnedLoans, resolved, lentCurrent, lentProphecy, lentResolved, resolves⟩ :=
      view
    have outcome : (propheticMeaning unit #[] handle σs shape resolved).ok initial result
        (final.withLoansOf initial) :=
      ⟨initial, loans, arguments, results, final, returnedLoans, results, rfl, admissible,
        lentResolved, execution, lentCurrent, lentProphecy, resolves, rfl⟩
    obtain ⟨ensured, framed, notMust⟩ :=
      (verified resolved initial (required loans resolved lentResolved)).1 result _ outcome
    refine ⟨fun notMay => ?_, ⟨loans, resolved, lentResolved, framed⟩,
      fun must => notMust (must loans resolved lentResolved)⟩
    exact ⟨loans, resolved, result, returnedLoans, lentResolved, lentCurrent, lentProphecy,
      resolves, ensured fun may => notMay ⟨loans, resolved, lentResolved, may⟩⟩
  · intro error failed
    exact ⟨loans, args, lent,
      established.2.1 error ⟨initial, loans, arguments, rfl, admissible, lent, failed⟩⟩

end LeanerIR.Proofs.Denote
