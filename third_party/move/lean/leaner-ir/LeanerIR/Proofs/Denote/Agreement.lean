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

/-- The prophetic meanings of the compiled members of a cycle of calls, at
every slot (the runtime family, or a skolem family and type instantiation),
are the least fixed point of their bodies over all slots, with every call to
a member routed to the argument at the slot it reaches (at the runtime
family, calls with type arguments stay closed): every outcome is one of a finite unfolding, since a big-step
derivation nests finitely many calls. A function calling itself is a cycle
of one. Assumed with the agreement above (`designs/denotation.md`, D6). -/
axiom compileFunction_least_cycle (unit : ExecutableUnit)
    (members : List (FunctionHandle × Function))
    (compiled : ∀ index : CycleIndex members,
      compileFunction unit.unit index.member.1 = .ok index.member.2)
    (typeInstantiation : Array (TypeId × TypeId)) (slot : CycleSlot members)
    (args : @HList (familySkolems slot.family) slot.position.member.2.params) :
    Spec.Refines
      (@propheticMeaning (familySkolems slot.family) unit
        (familyInstantiation typeInstantiation slot.family) slot.position.member.1
        slot.position.member.2.params slot.position.member.2.result args)
      (Spec.fixFamily (Index := CycleSlot members)
        (Args := fun slot => @HList (familySkolems slot.family) slot.position.member.2.params)
        (Result := fun slot =>
          @ResultShape.carrier (familySkolems slot.family) slot.position.member.2.result)
        (fun self slot => @Function.denoteWith (familySkolems slot.family)
          (cycleMeanings unit members typeInstantiation self slot) slot.position.member.2)
        slot args)

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

/-- Contracts established over the bodies of a cycle's members at every
slot, each assuming every slot's contract of the calls to the members, hold
of their prophetic meanings. -/
theorem satisfies_cycle (unit : ExecutableUnit) (typeInstantiation : Array (TypeId × TypeId))
    (members : List (FunctionHandle × Function))
    (compiled : ∀ index : CycleIndex members,
      compileFunction unit.unit index.member.1 = .ok index.member.2)
    (contracts : (slot : CycleSlot members) →
      Contract RuntimeState Failure (@HList (familySkolems slot.family) slot.position.member.2.params)
        (@ResultShape.carrier (familySkolems slot.family) slot.position.member.2.result))
    (verified : ∀ self : CycleFamilySelves members,
      (∀ slot, Satisfies (self slot) (contracts slot)) →
      ∀ slot, Satisfies
        (@Function.denoteWith (familySkolems slot.family)
          (cycleMeanings unit members typeInstantiation self slot) slot.position.member.2)
        (contracts slot))
    (slot : CycleSlot members) :
    Satisfies
      (@propheticMeaning (familySkolems slot.family) unit
        (familyInstantiation typeInstantiation slot.family) slot.position.member.1
        slot.position.member.2.params slot.position.member.2.result)
      (contracts slot) :=
  satisfies_of_refines (compileFunction_least_cycle unit members compiled typeInstantiation slot)
    (satisfies_fixFamily _ contracts verified slot)

/-- `satisfies_cycle` at the runtime family: contracts established over the
members' bodies at the runtime slots, assuming them of the calls to members
there, hold of their prophetic meanings. The other slots stand vacuous. -/
theorem satisfies_cycle_runtime (unit : ExecutableUnit)
    (typeInstantiation : Array (TypeId × TypeId)) (members : List (FunctionHandle × Function))
    (compiled : ∀ index : CycleIndex members,
      compileFunction unit.unit index.member.1 = .ok index.member.2)
    (contracts : (index : CycleIndex members) →
      Contract RuntimeState Failure (HList index.member.2.params) index.member.2.result.carrier)
    (verified : ∀ self : CycleFamilySelves members,
      (∀ index, Satisfies (self ⟨index, none⟩) (contracts index)) →
      ∀ index, Satisfies
        (index.member.2.denoteWith (cycleMeanings unit members typeInstantiation self ⟨index, none⟩))
        (contracts index))
    (index : CycleIndex members) :
    Satisfies
      (propheticMeaning unit typeInstantiation index.member.1 index.member.2.params
        index.member.2.result)
      (contracts index) :=
  satisfies_cycle unit typeInstantiation members compiled
    (fun | ⟨index, none⟩ => contracts index | ⟨_, some _⟩ => Contract.vacuous)
    (fun self hypothesis => fun
      | ⟨index, none⟩ => verified self (fun index => hypothesis ⟨index, none⟩) index
      | ⟨_, some _⟩ => satisfies_vacuous _)
    ⟨index, none⟩

omit [Skolems] in
/-- `satisfies_cycle` at every skolem family and type instantiation:
contracts established over the members' bodies at every family, assuming
them of the calls to members at every family, hold of their prophetic
meanings. The runtime slots stand vacuous. -/
theorem satisfies_cycle_family [root : Skolems] (unit : ExecutableUnit)
    (typeInstantiation : Array (TypeId × TypeId)) (members : List (FunctionHandle × Function))
    (compiled : ∀ index : CycleIndex members,
      compileFunction unit.unit index.member.1 = .ok index.member.2)
    (contracts : (index : CycleIndex members) → (Θ : Skolems) → Array (TypeId × TypeId) →
      Contract RuntimeState Failure (@HList Θ index.member.2.params)
        (@ResultShape.carrier Θ index.member.2.result))
    (verified : ∀ self : @CycleFamilySelves root members,
      (∀ index Θ instantiation,
        Satisfies (self ⟨index, some (Θ, instantiation)⟩) (contracts index Θ instantiation)) →
      ∀ index Θ instantiation, Satisfies
        (@Function.denoteWith Θ
          (@cycleMeanings root unit members typeInstantiation self ⟨index, some (Θ, instantiation)⟩)
          index.member.2)
        (contracts index Θ instantiation))
    (index : CycleIndex members) (Θ : Skolems) (instantiation : Array (TypeId × TypeId)) :
    Satisfies
      (@propheticMeaning Θ unit instantiation index.member.1 index.member.2.params
        index.member.2.result)
      (contracts index Θ instantiation) :=
  @satisfies_cycle root unit typeInstantiation members compiled
    (fun | ⟨_, none⟩ => Contract.vacuous
         | ⟨index, some (Θ, instantiation)⟩ => contracts index Θ instantiation)
    (fun self hypothesis => fun
      | ⟨_, none⟩ => satisfies_vacuous _
      | ⟨index, some (Θ, instantiation)⟩ =>
          verified self (fun index Θ instantiation => hypothesis ⟨index, some (Θ, instantiation)⟩)
            index Θ instantiation)
    ⟨index, some (Θ, instantiation)⟩

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
