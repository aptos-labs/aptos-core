-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Closures
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

section Frames
variable {unit : Validation.ValidatedUnit} [Skolems unit]

/-- Agreement of a compiled function's prophetic big-step meaning
(`propheticMeaning`) with its denotation: every outcome of the big-step run
from an admissible start whose globals encode the memory, with the argument
references lent under loans and each reference's prophecy its resolved
export, is an outcome of the denotation, at every frame and type
instantiation.  Where the frame and the instantiation disagree on a type
the callee reads the prophetic meaning has no outcome; at a coherent frame
the compiler's types agree with the runtime's, so a coherent caller's calls
and closures have coherent frames.  To be proved by induction on the fuel of
`compileExpr`, one case per term constructor. -/
axiom compileFunction_agrees {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit] (handle : FunctionHandle)
    (f : Function unit) (compiled : compileFunction unit handle = .ok f)
    (typeInstantiation : Array (TypeId × TypeId)) (args : HList f.params) :
    Spec.Refines (propheticMeaning executable typeInstantiation handle f.params f.result args)
      (f.denote executable typeInstantiation args)

/-- The prophetic meanings of the compiled members of a cycle of calls, at
every slot (the runtime family, or a skolem family and type instantiation),
are the least fixed point of their bodies over all slots, with every call to
a member routed to the argument at the slot it reaches (at the runtime
family, calls with type arguments stay closed): every outcome is one of a finite unfolding, since a big-step
derivation nests finitely many calls. A function calling itself is a cycle
of one. Assumed with the agreement above (`designs/denotation.md`, D6). -/
axiom compileFunction_least_cycle {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit]
    (members : List (FunctionHandle × Function unit))
    (compiled : ∀ index : CycleIndex members,
      compileFunction unit index.member.1 = .ok index.member.2)
    (typeInstantiation : Array (TypeId × TypeId)) (slot : CycleSlot unit members)
    (args : @HList (familySkolems slot.family).toCarriers slot.position.member.2.params) :
    Spec.Refines
      (@propheticMeaning _ executable (familySkolems slot.family)
        (familyInstantiation typeInstantiation slot.family) slot.position.member.1
        slot.position.member.2.params slot.position.member.2.result args)
      (Spec.fixFamily (Index := CycleSlot unit members)
        (Args := fun slot => @HList (familySkolems slot.family).toCarriers slot.position.member.2.params)
        (Result := fun slot =>
          @ResultShape.carrier (familySkolems slot.family).toCarriers slot.position.member.2.result)
        (fun self slot => @Function.denoteWith _ _ (familySkolems slot.family)
          (cycleMeanings executable members typeInstantiation self slot) slot.position.member.2)
        slot args)

omit [Skolems unit] in
/-- A refinement keeps every weakest precondition of what it refines. -/
theorem wp_of_refines {σ ε α : Type} {a b : Spec σ ε α} (refines : Spec.Refines a b)
    {ensures : α → σ → Prop} {aborts : ε → Prop} {initial : σ}
    (established : wp b ensures aborts initial) : wp a ensures aborts initial :=
  ⟨fun result final step => established.1 result final (refines.ok _ _ _ step),
    fun error step => established.2.1 error (refines.aborts _ _ step),
    fun undefined => established.2.2 (refines.undefined _ undefined)⟩

/-- A call to a compiled callee reasons over the callee's denotation: an
unspecified callee, or one returning a reference, is inlined by the
agreement theorem. -/
theorem wp_propheticMeaning_of_compiled {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) (handle : FunctionHandle)
    (f : Function unit) (compiled : compileFunction unit handle = .ok f) (values : HList f.params)
    (ensures : f.result.carrier → Memory unit → Prop) (aborts : Failure → Prop)
    (initial : Memory unit)
    (established : wp (f.denote executable typeInstantiation values) ensures aborts initial) :
    wp (propheticMeaning executable typeInstantiation handle f.params f.result values) ensures aborts
      initial :=
  wp_of_refines (compileFunction_agrees executable handle f compiled typeInstantiation values) established

/-- A contract established over the denotation holds of the prophetic
meaning, which is what a caller's call denotes. -/
theorem satisfies_propheticMeaning {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) (handle : FunctionHandle)
    (f : Function unit) (compiled : compileFunction unit handle = .ok f)
    (contract : Contract (Memory unit) Failure (HList f.params) f.result.carrier)
    (verified : Satisfies (f.denote executable typeInstantiation) contract) :
    Satisfies (propheticMeaning executable typeInstantiation handle f.params f.result) contract :=
  satisfies_of_refines (compileFunction_agrees executable handle f compiled typeInstantiation) verified

/-- Contracts established over the bodies of a cycle's members at every
slot, each assuming every slot's contract of the calls to the members, hold
of their prophetic meanings. -/
theorem satisfies_cycle {unit : ValidatedUnit} (executable : ExecutableUnit unit) [Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId))
    (members : List (FunctionHandle × Function unit))
    (compiled : ∀ index : CycleIndex members,
      compileFunction unit index.member.1 = .ok index.member.2)
    (contracts : (slot : CycleSlot unit members) →
      Contract (Memory unit) Failure (@HList (familySkolems slot.family).toCarriers slot.position.member.2.params)
        (@ResultShape.carrier (familySkolems slot.family).toCarriers slot.position.member.2.result))
    (verified : ∀ self : CycleFamilySelves members,
      (∀ slot, Satisfies (self slot) (contracts slot)) →
      ∀ slot, Satisfies
        (@Function.denoteWith _ _ (familySkolems slot.family)
          (cycleMeanings executable members typeInstantiation self slot) slot.position.member.2)
        (contracts slot))
    (slot : CycleSlot unit members) :
    Satisfies
      (@propheticMeaning _ executable (familySkolems slot.family)
        (familyInstantiation typeInstantiation slot.family) slot.position.member.1
        slot.position.member.2.params slot.position.member.2.result)
      (contracts slot) :=
  satisfies_of_refines (compileFunction_least_cycle executable members compiled typeInstantiation slot)
    (satisfies_fixFamily _ contracts verified slot)

/-- `satisfies_cycle` at the runtime family: contracts established over the
members' bodies at the runtime slots, assuming them of the calls to members
there, hold of their prophetic meanings. The other slots stand vacuous. -/
theorem satisfies_cycle_runtime {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) (members : List (FunctionHandle × Function unit))
    (compiled : ∀ index : CycleIndex members,
      compileFunction unit index.member.1 = .ok index.member.2)
    (contracts : (index : CycleIndex members) →
      Contract (Memory unit) Failure (HList index.member.2.params) index.member.2.result.carrier)
    (verified : ∀ self : CycleFamilySelves members,
      (∀ index, Satisfies (self ⟨index, none⟩) (contracts index)) →
      ∀ index, Satisfies
        (index.member.2.denoteWith (cycleMeanings executable members typeInstantiation self ⟨index, none⟩))
        (contracts index))
    (index : CycleIndex members) :
    Satisfies
      (propheticMeaning executable typeInstantiation index.member.1 index.member.2.params
        index.member.2.result)
      (contracts index) :=
  satisfies_cycle executable typeInstantiation members compiled
    (fun | ⟨index, none⟩ => contracts index | ⟨_, some _⟩ => Contract.vacuous)
    (fun self hypothesis => fun
      | ⟨index, none⟩ => verified self (fun index => hypothesis ⟨index, none⟩) index
      | ⟨_, some _⟩ => satisfies_vacuous _)
    ⟨index, none⟩

omit [Skolems unit] in
/-- `satisfies_cycle` at every skolem family and type instantiation:
contracts established over the members' bodies at every family, assuming
them of the calls to members at every family, hold of their prophetic
meanings. The runtime slots stand vacuous. -/
theorem satisfies_cycle_family {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    [root : Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) (members : List (FunctionHandle × Function unit))
    (compiled : ∀ index : CycleIndex members,
      compileFunction unit index.member.1 = .ok index.member.2)
    (contracts : (index : CycleIndex members) → (Θ : Skolems unit) → Array (TypeId × TypeId) →
      Contract (Memory unit) Failure (@HList Θ.toCarriers index.member.2.params)
        (@ResultShape.carrier Θ.toCarriers index.member.2.result))
    (verified : ∀ self : @CycleFamilySelves _ root members,
      (∀ index Θ instantiation,
        Satisfies (self ⟨index, some (Θ, instantiation)⟩) (contracts index Θ instantiation)) →
      ∀ index Θ instantiation, Satisfies
        (@Function.denoteWith _ _ Θ
          (@cycleMeanings _ executable root members typeInstantiation self ⟨index, some (Θ, instantiation)⟩)
          index.member.2)
        (contracts index Θ instantiation))
    (index : CycleIndex members) (Θ : Skolems unit) (instantiation : Array (TypeId × TypeId)) :
    Satisfies
      (@propheticMeaning _ executable Θ instantiation index.member.1 index.member.2.params
        index.member.2.result)
      (contracts index Θ instantiation) :=
  @satisfies_cycle _ executable root typeInstantiation members compiled
    (fun | ⟨_, none⟩ => Contract.vacuous
         | ⟨index, some (Θ, instantiation)⟩ => contracts index Θ instantiation)
    (fun self hypothesis => fun
      | ⟨_, none⟩ => satisfies_vacuous _
      | ⟨index, some (Θ, instantiation)⟩ =>
          verified self (fun index Θ instantiation => hypothesis ⟨index, some (Θ, instantiation)⟩)
            index Θ instantiation)
    ⟨index, some (Θ, instantiation)⟩

omit [Skolems unit] in
/-- Runs of a unit keep global and Table storage encoding a typed memory: from
stores encoding a memory, every successful run ends in stores encoding one that
agrees with it where no runtime key reaches.  A property of the unit's
semantics that static typing establishes (`designs/static-memory.md`); the
public theorem states it as a hypothesis. -/
def GlobalsPreserved {unit : ValidatedUnit} (executable : ExecutableUnit unit) : Prop :=
  ∀ handle typeInstantiation start arguments results exit memory,
    StorageEncodes unit memory start →
    (functionSpecAt executable handle typeInstantiation arguments).ok start results exit →
    ∃ final, StorageEncodesReturned unit final exit results ∧ AgreeUnnamed unit final memory

end Frames

section Public

/-- The runtime form of a contract over native arguments and typed memory.
A runtime call lends the argument references under admissible loans from a
state whose stores encode a memory, and the contract holds for every
prophecy of theirs and every memory the stores encode.  A successful
execution satisfies it at the value view: each returned reference's
prophecy is its current value, and each argument reference's prophecy is
its export with the returned references' holes filled by their current
values. -/
def _root_.LeanerIR.Proofs.Contract.prophetic (unit : ValidatedUnit) [Skolems unit] (σs : NRow)
    (shape : ResultShape) (contract : Contract (Memory unit) Failure (HList σs) shape.carrier) :
    FunctionContract where
  requires := fun arguments initial =>
    (∃ loans args, Admissible initial loans ∧ lendArguments σs args loans = some arguments) ∧
      (∃ memory, StorageEncodes unit memory initial) ∧
      ∀ memory, StorageEncodes unit memory initial →
        ∀ loans args, lendArguments σs args loans = some arguments → contract.requires args memory
  assumes := fun arguments initial =>
    ∀ memory, StorageEncodes unit memory initial →
      ∀ loans args, lendArguments σs args loans = some arguments → contract.assumes args memory
  ensures := fun arguments initial results final =>
    ∃ memory memory' loans args result returnedLoans,
      StorageEncodes unit memory initial ∧ StorageEncodesReturned unit memory' final results ∧
      lendArguments σs args loans = some arguments ∧
      shape.lend false result returnedLoans = some results ∧
      shape.lend true result returnedLoans = some results ∧
      argumentsResolve σs args loans results (exportsAfter initial.pending final.pending) ∧
      contract.ensures args memory result memory'
  aborts := fun arguments initial error =>
    ∃ memory loans args, StorageEncodes unit memory initial ∧
      lendArguments σs args loans = some arguments ∧ contract.aborts args memory error
  mayAbort := fun arguments initial =>
    ∃ memory loans args, StorageEncodes unit memory initial ∧
      lendArguments σs args loans = some arguments ∧ contract.mayAbort args memory
  mustAbort := fun arguments initial =>
    ∀ memory, StorageEncodes unit memory initial →
      ∀ loans args, lendArguments σs args loans = some arguments → contract.mustAbort args memory
  frame := fun arguments initial results final =>
    ∃ memory memory' loans args result returnedLoans, StorageEncodes unit memory initial ∧
      StorageEncodesReturned unit memory' final results ∧
      lendArguments σs args loans = some arguments ∧
      shape.lend false result returnedLoans = some results ∧
      shape.lend true result returnedLoans = some results ∧
      contract.frame args memory result memory'

/-- A contract satisfied by the prophetic run at a runtime type instantiation
holds of the big-step meaning there in its runtime form, for a unit whose
runs keep global memory typed.  Every runtime execution from an admissible
start is a prophetic outcome at the value view from a memory its stores
encode, whose existence the contract's definedness guarantees. -/
theorem satisfies_run_at {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (preserved : GlobalsPreserved executable)
    (handle : FunctionHandle) (typeInstantiation : Array (TypeId × TypeId)) (σs : NRow)
    (shape : ResultShape)
    (contract : Contract (Memory unit) Failure (@HList (Carriers.runtime unit) σs)
      (@ResultShape.carrier (Carriers.runtime unit) shape))
    (verified : letI : Skolems unit := Skolems.runtime unit
      Satisfies (propheticRun executable typeInstantiation handle σs shape) contract) :
    letI : Skolems unit := Skolems.runtime unit
    SatisfiesFunctionAt executable handle typeInstantiation
      (contract.prophetic unit σs shape) := by
  letI : Skolems unit := Skolems.runtime unit
  intro arguments initial assumed permitted
  obtain ⟨⟨loans, args, admissible, lent⟩, ⟨memory, encoded⟩, required⟩ := permitted
  have established := verified args memory (assumed memory encoded loans args lent)
    (required memory encoded loans args lent)
  refine ⟨?_, ?_, fun obligation => obligation⟩
  · intro results final execution
    obtain ⟨memory', encoded', agree⟩ :=
      preserved handle typeInstantiation initial arguments results final memory encoded execution
    have view : ∃ result returnedLoans, ∃ resolved : HList σs,
        shape.lend false result returnedLoans = some results ∧
        shape.lend true result returnedLoans = some results ∧
        lendArguments σs resolved loans = some arguments ∧
        argumentsResolve σs resolved loans results (exportsAfter initial.pending final.pending) := by
      refine Classical.byContradiction fun none => established.2.2 ?_
      exact ⟨initial, loans, arguments, results, final, encoded, admissible, lent, execution,
        none⟩
    obtain ⟨result, returnedLoans, resolved, lentCurrent, lentProphecy, lentResolved, resolves⟩ :=
      view
    have outcome :
        (propheticRun executable typeInstantiation handle σs shape resolved).ok memory result
          memory' :=
      ⟨initial, loans, arguments, results, final, returnedLoans, results, encoded, admissible,
        lentResolved, execution, lentCurrent, lentProphecy, resolves, encoded', agree⟩
    obtain ⟨ensured, framed, notMust⟩ :=
      (verified resolved memory (assumed memory encoded loans resolved lentResolved)
        (required memory encoded loans resolved lentResolved)).1 result _ outcome
    refine ⟨fun notMay => ?_, ⟨memory, memory', loans, resolved, result, returnedLoans, encoded, encoded', lentResolved,
        lentCurrent, lentProphecy, framed⟩, fun must => notMust (must memory encoded loans resolved lentResolved)⟩
    exact ⟨memory, memory', loans, resolved, result, returnedLoans, encoded, encoded', lentResolved,
      lentCurrent, lentProphecy, resolves,
      ensured fun may => notMay ⟨memory, loans, resolved, encoded, lentResolved, may⟩⟩
  · intro error failed
    exact ⟨memory, loans, args, encoded, lent,
      established.2.1 error ⟨initial, loans, arguments, encoded, admissible, lent, failed⟩⟩

/-- A contract satisfied by the prophetic meaning at the runtime frame and
the empty instantiation holds of the big-step meaning in its runtime form,
for a unit whose runs keep global memory typed. -/
theorem satisfies_prophetic {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (preserved : GlobalsPreserved executable)
    (handle : FunctionHandle) (σs : NRow) (shape : ResultShape)
    (contract : Contract (Memory unit) Failure (@HList (Carriers.runtime unit) σs)
      (@ResultShape.carrier (Carriers.runtime unit) shape))
    (verified : letI : Skolems unit := Skolems.runtime unit
      Satisfies (propheticMeaning executable #[] handle σs shape) contract) :
    letI : Skolems unit := Skolems.runtime unit
    SatisfiesFunction executable handle (contract.prophetic unit σs shape) := by
  letI : Skolems unit := Skolems.runtime unit
  refine satisfies_run_at executable preserved handle #[] σs shape contract ?_
  have meaning : propheticMeaning executable #[] handle σs shape =
      propheticRun executable #[] handle σs shape :=
    funext (propheticMeaning_of_coherent (coherent_runtime unit handle) σs shape)
  rwa [meaning] at verified

/-- A generic function's contract, satisfied by its prophetic meaning at the
frame type arguments induce over the runtime family, holds, read at the
arguments' types, of its big-step meaning at every runtime type
instantiation coherent with that frame. -/
theorem satisfies_generic_at {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (preserved : GlobalsPreserved executable) (handle : FunctionHandle) (θ : TypeArgs)
    (free : θ.1.refFree = true) (typeInstantiation : Array (TypeId × TypeId)) (σs : NRow)
    (shape : ResultShape)
    (contract : letI : Skolems unit := Skolems.instantiate θ (Skolems.runtime unit)
      Contract (Memory unit) Failure (HList σs) shape.carrier)
    (coherent : @Coherent unit (Skolems.instantiate θ (Skolems.runtime unit)) handle
      typeInstantiation)
    (verified : letI : Skolems unit := Skolems.instantiate θ (Skolems.runtime unit)
      Satisfies (propheticMeaning executable typeInstantiation handle σs shape) contract) :
    letI : Skolems unit := Skolems.runtime unit
    SatisfiesFunctionAt executable handle typeInstantiation
      ((contract.ofSkolem θ).prophetic unit (NRow.subst θ.1 σs) (shape.subst θ.1)) := by
  letI : Skolems unit := Skolems.runtime unit
  refine satisfies_run_at executable preserved handle typeInstantiation _ _ _ ?_
  have meaning : @propheticMeaning _ executable (Skolems.instantiate θ (Skolems.runtime unit))
        typeInstantiation handle σs shape =
      @propheticRun _ executable (Skolems.instantiate θ (Skolems.runtime unit)) typeInstantiation
        handle σs shape :=
    funext (@propheticMeaning_of_coherent _ _ (Skolems.instantiate θ (Skolems.runtime unit)) _ _
      coherent σs shape)
  rw [meaning] at verified
  exact satisfies_run_ofSkolem executable θ free typeInstantiation handle σs shape contract
    verified

end Public

end LeanerIR.Proofs.Denote
