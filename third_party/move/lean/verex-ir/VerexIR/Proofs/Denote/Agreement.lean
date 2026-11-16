-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Closures
import VerexIR.Proofs.Denote.Compile

/-!
# Agreement of the denotation with the big-step semantics

The one theorem of `doc/designs/denotation.md`: for every function the compiler
accepts, the denotation of its term is exactly its prophetic big-step
meaning, with references as `(current, prophecy)`.  It is stated over the compiler, so it holds for every
unit at once; a verified function transports its native theorem through it
to the authoritative semantics without any proof of its own.

**Status: assumed.**  By the user's decision of 2026-09-08 the induction
over the compiler's fuel is postponed and the statement is an explicit,
named axiom until then, so that `#print axioms` names it on every theorem
that depends on it.  Nothing else in the denotation is admitted.
-/

namespace VerexIR.Proofs.Denote

open VerexIR.Validation

section Frames
variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit] [Skolems unit]

/-- Agreement of a compiled function's prophetic big-step meaning
(`propheticMeaning`) with its denotation: every outcome of the big-step run
from an admissible start whose globals encode the memory, with the argument
references lent under loans and each reference's prophecy its resolved
export, is an outcome of the denotation, at every frame and type
instantiation.  Where the frame and the instantiation disagree on a type
the callee reads the prophetic meaning has no outcome; at a coherent frame
the compiler's types agree with the runtime's, so a coherent caller's calls
and closures have coherent frames.  A function compiled at a substitution
agrees at the same frames, over its substituted rows (`propheticMeaningAt`);
without one the statement is `compileFunction`'s and `propheticMeaning`'s.
To be proved by induction on the fuel of `compileExpr`, one case per term
constructor.

The refinement relates the outcomes of a start only where the denotation
is defined there, and its checking steps are undefined where a data
invariant fails: a run that packs a value breaking its invariant, or that
ends a mutable loan over one, keeps its big-step outcomes while the
denotation is undefined at its start, so nothing is asserted of that start,
and `Satisfies` excludes it, so the invariant is owed in the function that
breaks it (`doc/designs/data-invariants.md`). -/
axiom compileFunction_agrees {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit)
    [Skolems unit] (handle : FunctionHandle) (substitution : Option TypeArgs)
    (f : Function unit) (compiled : compileFunctionAt unit handle substitution = .ok f)
    (typeInstantiation : Array (TypeId × TypeId)) (args : HList f.params) :
    Spec.Refines
      (propheticMeaningAt executable substitution typeInstantiation handle f.params f.result args)
      (f.denote executable typeInstantiation args)

/-- The prophetic meanings of the compiled members of a cycle of calls, at
every slot (the runtime family, or a skolem family and type instantiation),
are the least fixed point of their bodies over all slots, with every call to
a member routed to the argument at the slot it reaches (at the runtime
family, calls with type arguments stay closed): every outcome is one of a finite unfolding, since a big-step
derivation nests finitely many calls. A function calling itself is a cycle
of one. Assumed with the agreement above (`doc/designs/denotation.md`, D6). -/
axiom compileFunction_least_cycle {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit)
    [Skolems unit]
    (members : List (FunctionHandle × Function unit))
    (compiled : ∀ index : CycleIndex members,
      compileFunction unit index.member.1 = .ok index.member.2)
    (typeInstantiation : Array (TypeId × TypeId)) (slot : CycleSlot unit members)
    (args : @HList (familySkolems slot.family).toCarriers slot.position.member.2.params) :
    Spec.Refines
      (@propheticMeaning _ _ executable (familySkolems slot.family)
        (familyInstantiation typeInstantiation slot.family) slot.position.member.1
        slot.position.member.2.params slot.position.member.2.result args)
      (Spec.fixFamily (Index := CycleSlot unit members)
        (Args := fun slot => @HList (familySkolems slot.family).toCarriers slot.position.member.2.params)
        (Result := fun slot =>
          @ResultShape.carrier (familySkolems slot.family).toCarriers slot.position.member.2.result)
        (fun self slot => @Function.denoteWith _ _ _ (familySkolems slot.family)
          (cycleMeanings executable members typeInstantiation self slot) slot.position.member.2)
        slot args)

omit [Skolems unit] in
/-- A refinement keeps every weakest precondition of what it refines. -/
theorem wp_of_refines {σ ε α : Type} {a b : Spec σ ε α} (refines : Spec.Refines a b)
    {ensures : α → σ → Prop} {aborts : ε → Prop} {initial : σ}
    (established : wp b ensures aborts initial) : wp a ensures aborts initial :=
  ⟨fun result final step =>
      (refines.ok _ _ _ step).elim (established.1 result final) (absurd · established.2.2),
    fun error step => (refines.aborts _ _ step).elim (established.2.1 error) (absurd · established.2.2),
    fun undefined => established.2.2 (refines.undefined _ undefined)⟩

/-- A call to a compiled callee reasons over the callee's denotation: an
unspecified callee, or one returning a reference, is inlined by the
agreement theorem. -/
theorem wp_propheticMeaning_of_compiled {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit)
    [Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) (handle : FunctionHandle)
    (f : Function unit) (compiled : compileFunction unit handle = .ok f) (values : HList f.params)
    (ensures : f.result.carrier → Memory unit → Prop) (aborts : Failure → Prop)
    (initial : Memory unit)
    (established : wp (f.denote executable typeInstantiation values) ensures aborts initial) :
    wp (propheticMeaning executable typeInstantiation handle f.params f.result values) ensures aborts
      initial :=
  wp_of_refines (compileFunction_agrees executable handle none f compiled typeInstantiation values)
    established

/-- A call at type arguments holding a mutable reference: inlined, the callee's body at
them replacing the call by the agreement theorem. -/
theorem wp_propheticMeaningAt_of_compiled {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit)
    [Skolems unit]
    (substitution : Option TypeArgs) (typeInstantiation : Array (TypeId × TypeId))
    (handle : FunctionHandle)
    (f : Function unit) (compiled : compileFunctionAt unit handle substitution = .ok f)
    (values : HList f.params)
    (ensures : f.result.carrier → Memory unit → Prop) (aborts : Failure → Prop)
    (initial : Memory unit)
    (established : wp (f.denote executable typeInstantiation values) ensures aborts initial) :
    wp (propheticMeaningAt executable substitution typeInstantiation handle f.params f.result values)
      ensures aborts initial :=
  wp_of_refines (compileFunction_agrees executable handle substitution f compiled typeInstantiation
    values) established

/-- A contract established over the denotation holds of the prophetic
meaning, which is what a caller's call denotes. -/
theorem satisfies_propheticMeaning {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit)
    [Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) (handle : FunctionHandle)
    (f : Function unit) (compiled : compileFunction unit handle = .ok f)
    (contract : Contract (Memory unit) Failure (HList f.params) f.result.carrier)
    (verified : Satisfies (f.denote executable typeInstantiation) contract) :
    Satisfies (propheticMeaning executable typeInstantiation handle f.params f.result) contract :=
  satisfies_of_refines (compileFunction_agrees executable handle none f compiled typeInstantiation)
    verified

/-- A contract proved independently over a specialized body holds at that
same frame; callers may use its contract without inlining the body. -/
theorem satisfies_propheticMeaningAt {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit) [Skolems unit]
    (substitution : Option TypeArgs) (typeInstantiation : Array (TypeId × TypeId))
    (handle : FunctionHandle) (f : Function unit)
    (compiled : compileFunctionAt unit handle substitution = .ok f)
    (contract : Contract (Memory unit) Failure (HList f.params) f.result.carrier)
    (verified : Satisfies (f.denote executable typeInstantiation) contract) :
    Satisfies (propheticMeaningAt executable substitution typeInstantiation handle
      f.params f.result) contract :=
  satisfies_of_refines
    (compileFunction_agrees executable handle substitution f compiled typeInstantiation) verified

/-- Contracts established over the bodies of a cycle's members at every
slot, each assuming every slot's contract of the calls to the members, hold
of their prophetic meanings. -/
theorem satisfies_cycle {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit) [Skolems unit]
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
        (@Function.denoteWith _ _ _ (familySkolems slot.family)
          (cycleMeanings executable members typeInstantiation self slot) slot.position.member.2)
        (contracts slot))
    (slot : CycleSlot unit members) :
    Satisfies
      (@propheticMeaning _ _ executable (familySkolems slot.family)
        (familyInstantiation typeInstantiation slot.family) slot.position.member.1
        slot.position.member.2.params slot.position.member.2.result)
      (contracts slot) :=
  satisfies_of_refines (compileFunction_least_cycle executable members compiled typeInstantiation slot)
    (satisfies_fixFamily _ contracts verified slot)

/-- `satisfies_cycle` at the runtime family: contracts established over the
members' bodies at the runtime slots, assuming them of the calls to members
there, hold of their prophetic meanings. The other slots stand vacuous. -/
theorem satisfies_cycle_runtime {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit)
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
theorem satisfies_cycle_family {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit)
    [root : Skolems unit]
    (typeInstantiation : Array (TypeId × TypeId)) (members : List (FunctionHandle × Function unit))
    (compiled : ∀ index : CycleIndex members,
      compileFunction unit index.member.1 = .ok index.member.2)
    (contracts : (index : CycleIndex members) → (Θ : Skolems unit) → Array (TypeId × TypeId) →
      Contract (Memory unit) Failure (@HList Θ.toCarriers index.member.2.params)
        (@ResultShape.carrier Θ.toCarriers index.member.2.result))
    (verified : ∀ self : @CycleFamilySelves _ _ root members,
      (∀ index Θ instantiation,
        Satisfies (self ⟨index, some (Θ, instantiation)⟩) (contracts index Θ instantiation)) →
      ∀ index Θ instantiation, Satisfies
        (@Function.denoteWith _ _ _ Θ
          (@cycleMeanings _ _ executable root members typeInstantiation self ⟨index, some (Θ, instantiation)⟩)
          index.member.2)
        (contracts index Θ instantiation))
    (index : CycleIndex members) (Θ : Skolems unit) (instantiation : Array (TypeId × TypeId)) :
    Satisfies
      (@propheticMeaning _ _ executable Θ instantiation index.member.1 index.member.2.params
        index.member.2.result)
      (contracts index Θ instantiation) :=
  @satisfies_cycle _ _ executable root typeInstantiation members compiled
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
semantics that static typing establishes (`doc/designs/static-memory.md`); the
public theorem states it as a hypothesis. -/
def GlobalsPreserved {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit) : Prop :=
  ∀ handle typeInstantiation start arguments results exit memory,
    StorageEncodes unit memory start →
    (functionSpecAt executable handle typeInstantiation arguments).ok start results exit →
    ∃ final, StorageEncodesReturned unit final exit results ∧ AgreeUnnamed unit final memory

/-- Runs of a unit fail with integers: every failure of a run is a failure of
the denotation encoded (`Failure.encode`), an abort code or the failed
operation's operands. An assumption the public theorem states as a
hypothesis, beside `GlobalsPreserved`; nothing establishes it yet, as
validation does not constrain a throw's operands
(`doc/status/assumptions.md`, "`FailuresIntegral`"). -/
def FailuresIntegral {unit : ValidatedUnit} (executable : ExecutableUnit unit) : Prop :=
  ∀ handle typeInstantiation start arguments failure,
    (functionSpecAt executable handle typeInstantiation arguments).aborts start failure →
    ∃ error : Failure, error.encode = failure

end Frames

section Public

/-- The runtime form of a contract over native arguments and typed memory.
A runtime call lends the argument references under admissible loans from a
state whose stores encode a memory, and the contract holds for every valid
prophecy of theirs and every memory the stores encode. A successful
execution supplies one loan correspondence spanning arguments, results, and
exports. The correspondence follows replacement references by their actual
runtime identity and resolves the final memory consistently. -/
def _root_.VerexIR.Proofs.Contract.prophetic (unit : ValidatedUnit) [FunctionCarriers unit]
    [Skolems unit] (σs : NRow)
    (shape : ResultShape) (contract : Contract (Memory unit) Failure (HList σs) shape.carrier) :
    FunctionContract where
  requires := fun arguments initial =>
    (∃ loans args, Admissible initial loans ∧ contract.validArgs args ∧
      lendArguments σs args loans = some arguments) ∧
      (∃ memory, StorageEncodes unit memory initial) ∧
      ∀ memory, StorageEncodes unit memory initial →
        ∀ loans args, contract.validArgs args →
          lendArguments σs args loans = some arguments → contract.requires args memory
  assumes := fun arguments initial =>
    ∀ memory, StorageEncodes unit memory initial →
      ∀ loans args, contract.validArgs args →
        lendArguments σs args loans = some arguments → contract.assumes args memory
  ensures := fun arguments initial results final =>
    ∃ memory memory' loans args result correspondence,
      StorageEncodes unit memory initial ∧
      StorageEncodesReturned unit memory' final (correspondence.returnedRow results) ∧
      contract.validArgs args ∧ contract.validResult result ∧
      lendArguments σs args loans = some arguments ∧
      BoundaryCorrespondence σs args shape result arguments results loans
        (exportsAfter initial.pending final.pending) correspondence ∧
      contract.ensures args memory result memory'
  aborts := fun arguments initial error =>
    ∃ memory loans args failure, StorageEncodes unit memory initial ∧
      contract.validArgs args ∧
      lendArguments σs args loans = some arguments ∧ failure.encode = error ∧
      contract.aborts args memory failure
  mayAbort := fun arguments initial =>
    ∃ memory loans args, StorageEncodes unit memory initial ∧
      contract.validArgs args ∧
      lendArguments σs args loans = some arguments ∧ contract.mayAbort args memory
  mustAbort := fun arguments initial =>
    ∀ memory, StorageEncodes unit memory initial →
      ∀ loans args, contract.validArgs args →
        lendArguments σs args loans = some arguments → contract.mustAbort args memory
  frame := fun arguments initial results final =>
    ∃ memory memory' loans args result correspondence, StorageEncodes unit memory initial ∧
      StorageEncodesReturned unit memory' final (correspondence.returnedRow results) ∧
      contract.validArgs args ∧ contract.validResult result ∧
      lendArguments σs args loans = some arguments ∧
      BoundaryCorrespondence σs args shape result arguments results loans
        (exportsAfter initial.pending final.pending) correspondence ∧
      contract.frame args memory result memory'

private theorem resolvedHolds_apply [Carriers] : (σs : NRow) → {args resolved : HList σs} →
    HList.Holds σs args → HList.ResolvedHolds σs args resolved → HList.Holds σs resolved
  | .nil, _, _, _, _ => trivial
  | .cons _ rest, _, _, holds, resolved =>
      ⟨resolved.1 holds.1, resolvedHolds_apply rest holds.2 resolved.2⟩

private theorem resolvedHolds_toSkolem {Θ : Carriers} (θ : TypeArgs) :
    (σs : NRow) → {args resolved : @HList Θ (NRow.subst θ.1 σs)} →
    @HList.ResolvedHolds Θ (NRow.subst θ.1 σs) args resolved →
    @HList.ResolvedHolds (Carriers.instantiate θ Θ) σs
      (HList.toSkolem θ σs args) (HList.toSkolem θ σs resolved)
  | .nil, _, _, _ => trivial
  | .cons τ rest, _, _, resolved =>
      ⟨fun holds => (NTy.holds_toSkolem_iff (Θ := Θ) θ τ _).mpr
          (resolved.1 ((NTy.holds_toSkolem_iff (Θ := Θ) θ τ _).mp holds)),
        resolvedHolds_toSkolem θ rest resolved.2⟩

/-- Automatic discharge that `contract.validArgs` is preserved across
`HList.ResolvedHolds` (`HList.Holds σs` on generated contracts and
`fun _ => True` on vacuous or unguarded contracts). -/
class PreservedValidArgs [Carriers] (σs : NRow) (validArgs : HList σs → Prop) : Prop where
  preserved : ∀ args resolved, validArgs args → HList.ResolvedHolds σs args resolved →
    validArgs resolved

instance [Carriers] (σs : NRow) : PreservedValidArgs σs (HList.Holds σs) :=
  ⟨fun _ _ => resolvedHolds_apply σs⟩

instance [Carriers] (σs : NRow) : PreservedValidArgs σs (fun _ => True) :=
  ⟨fun _ _ _ _ => True.intro⟩

/-- A contract satisfied by the prophetic run at a runtime type instantiation
holds of the big-step meaning there in its runtime form, for a unit whose
runs keep global memory typed.  Every runtime execution from an admissible
start is a prophetic outcome with a shared loan correspondence and final
memory, whose existence the contract's definedness guarantees. -/
theorem satisfies_run_at {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit)
    (preserved : GlobalsPreserved executable) (integral : FailuresIntegral executable)
    (handle : FunctionHandle) (typeInstantiation : Array (TypeId × TypeId)) (σs : NRow)
    (shape : ResultShape)
    (contract : Contract (Memory unit) Failure (@HList (Carriers.runtime unit) σs)
      (@ResultShape.carrier (Carriers.runtime unit) shape))
    (verified : letI : Skolems unit := Skolems.runtime unit
      Satisfies (propheticRun executable typeInstantiation handle σs shape) contract)
    (preservedValid : letI : Skolems unit := Skolems.runtime unit
      ∀ args resolved, contract.validArgs args → HList.ResolvedHolds σs args resolved →
        contract.validArgs resolved) :
    letI : Skolems unit := Skolems.runtime unit
    SatisfiesFunctionAt executable handle typeInstantiation
      (contract.prophetic unit σs shape) := by
  letI : Skolems unit := Skolems.runtime unit
  intro arguments initial _ assumed permitted
  obtain ⟨⟨loans, args, admissible, validArgs0, lent⟩, ⟨memory, encoded⟩, required⟩ := permitted
  have established := verified args memory validArgs0
    (assumed memory encoded loans args validArgs0 lent)
    (required memory encoded loans args validArgs0 lent)
  refine ⟨?_, ?_, fun obligation => obligation⟩
  · intro results final execution
    obtain ⟨memory', encoded', agree⟩ :=
      preserved handle typeInstantiation initial arguments results final memory encoded execution
    have view : ∃ result, ∃ resolved : HList σs, ∃ correspondence,
        HList.ResolvedHolds σs args resolved ∧
        lendArguments σs resolved loans = some arguments ∧
        BoundaryCorrespondence σs resolved shape result arguments results loans
          (exportsAfter initial.pending final.pending) correspondence ∧
        correspondence.ReturnsStorage results final := by
      refine Classical.byContradiction fun none => established.2.2 ?_
      exact ⟨initial, loans, arguments, results, final, encoded, admissible, lent, execution,
        none⟩
    obtain ⟨result, resolved, correspondence, resolvedHolds, lentResolved, boundary, storage⟩ := view
    have validResolved : contract.validArgs resolved :=
      preservedValid args resolved validArgs0 resolvedHolds
    have encodedCorrespondence :
        StorageEncodesReturned unit memory' final (correspondence.returnedRow results) := by
      change StorageEncodes unit memory' (final.resolveReturned (correspondence.returnedRow results))
      rw [storage]
      exact encoded'
    have outcome :
        (propheticRun executable typeInstantiation handle σs shape resolved).ok memory result
          memory' :=
      ⟨initial, loans, arguments, results, final, correspondence, encoded, admissible,
        lentResolved, execution, boundary, encodedCorrespondence, agree⟩
    obtain ⟨validRes, ensured, framed, notMust⟩ :=
      (verified resolved memory validResolved
        (assumed memory encoded loans resolved validResolved lentResolved)
        (required memory encoded loans resolved validResolved lentResolved)).1 result _ outcome
    refine ⟨trivial, fun notMay => ?_,
      ⟨memory, memory', loans, resolved, result, correspondence, encoded, encodedCorrespondence,
        validResolved, validRes, lentResolved, boundary, framed⟩,
      fun must => notMust (must memory encoded loans resolved validResolved lentResolved)⟩
    exact ⟨memory, memory', loans, resolved, result, correspondence, encoded, encodedCorrespondence,
      validResolved, validRes, lentResolved, boundary,
      ensured fun may => notMay ⟨memory, loans, resolved, encoded, validResolved, lentResolved, may⟩⟩
  · intro error failed
    obtain ⟨failure, rfl⟩ := integral handle typeInstantiation initial arguments error failed
    exact ⟨memory, loans, args, failure, encoded, validArgs0, lent, rfl,
      established.2.1 failure ⟨initial, loans, arguments, encoded, admissible, lent, failed⟩⟩

theorem satisfies_prophetic_valid {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit)
    (preserved : GlobalsPreserved executable) (integral : FailuresIntegral executable)
    (handle : FunctionHandle) (σs : NRow) (shape : ResultShape)
    (contract : Contract (Memory unit) Failure (@HList (Carriers.runtime unit) σs)
      (@ResultShape.carrier (Carriers.runtime unit) shape))
    (verified : letI : Skolems unit := Skolems.runtime unit
      Satisfies (propheticMeaning executable #[] handle σs shape) contract)
    (preservedValid : letI : Skolems unit := Skolems.runtime unit
      ∀ args resolved, contract.validArgs args → HList.ResolvedHolds σs args resolved →
        contract.validArgs resolved) :
    letI : Skolems unit := Skolems.runtime unit
    SatisfiesFunction executable handle (contract.prophetic unit σs shape) := by
  letI : Skolems unit := Skolems.runtime unit
  refine satisfies_run_at executable preserved integral handle #[] σs shape contract ?_
    (preservedValid := preservedValid)
  have meaning : propheticMeaning executable #[] handle σs shape =
      propheticRun executable #[] handle σs shape :=
    funext (propheticMeaning_of_coherent (coherent_runtime unit handle) σs shape)
  rwa [meaning] at verified

/-- A contract satisfied by the prophetic meaning at the runtime frame and
the empty instantiation holds of the big-step meaning in its runtime form,
for a unit whose runs keep global memory typed. -/
theorem satisfies_prophetic {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit)
    (preserved : GlobalsPreserved executable) (integral : FailuresIntegral executable)
    (handle : FunctionHandle) (σs : NRow) (shape : ResultShape)
    (contract : Contract (Memory unit) Failure (@HList (Carriers.runtime unit) σs)
      (@ResultShape.carrier (Carriers.runtime unit) shape))
    (verified : letI : Skolems unit := Skolems.runtime unit
      Satisfies (propheticMeaning executable #[] handle σs shape) contract)
    [validArgs : @PreservedValidArgs (Carriers.runtime unit) σs contract.validArgs] :
    letI : Skolems unit := Skolems.runtime unit
    SatisfiesFunction executable handle (contract.prophetic unit σs shape) :=
  satisfies_prophetic_valid executable preserved integral handle σs shape contract verified
    validArgs.preserved

theorem satisfies_generic_at_valid {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit)
    (preserved : GlobalsPreserved executable) (integral : FailuresIntegral executable)
    (handle : FunctionHandle) (θ : TypeArgs)
    (free : θ.1.refFree = true) (typeInstantiation : Array (TypeId × TypeId)) (σs : NRow)
    (shape : ResultShape)
    (contract : letI : Skolems unit := Skolems.instantiate θ (Skolems.runtime unit)
      Contract (Memory unit) Failure (HList σs) shape.carrier)
    (coherent : @Coherent unit _ (Skolems.instantiate θ (Skolems.runtime unit)) handle
      typeInstantiation)
    (verified : letI : Skolems unit := Skolems.instantiate θ (Skolems.runtime unit)
      Satisfies (propheticMeaning executable typeInstantiation handle σs shape) contract)
    (preservedValid : letI : Skolems unit := Skolems.instantiate θ (Skolems.runtime unit)
      ∀ args resolved, contract.validArgs args → HList.ResolvedHolds σs args resolved →
        contract.validArgs resolved) :
    letI : Skolems unit := Skolems.runtime unit
    SatisfiesFunctionAt executable handle typeInstantiation
      ((contract.ofSkolem θ).prophetic unit (NRow.subst θ.1 σs) (shape.subst θ.1)) := by
  letI : Skolems unit := Skolems.runtime unit
  refine satisfies_run_at executable preserved integral handle typeInstantiation _ _ _ ?_
    (preservedValid := fun args resolved valid resolvedHolds =>
      preservedValid (HList.toSkolem θ σs args) (HList.toSkolem θ σs resolved) valid
        (resolvedHolds_toSkolem (Θ := Carriers.runtime unit) θ σs resolvedHolds))
  have meaning : @propheticMeaning _ _ executable (Skolems.instantiate θ (Skolems.runtime unit))
        typeInstantiation handle σs shape =
      @propheticRun _ _ executable (Skolems.instantiate θ (Skolems.runtime unit)) typeInstantiation
        handle σs shape :=
    funext (@propheticMeaning_of_coherent _ _ _ (Skolems.instantiate θ (Skolems.runtime unit)) _ _
      coherent σs shape)
  rw [meaning] at verified
  exact satisfies_run_ofSkolem executable θ free typeInstantiation handle σs shape contract
    verified

/-- A generic function's contract, satisfied by its prophetic meaning at the
frame type arguments induce over the runtime family, holds, read at the
arguments' types, of its big-step meaning at every runtime type
instantiation coherent with that frame. -/
theorem satisfies_generic_at {unit : ValidatedUnit} [FunctionCarriers unit]
    (executable : ExecutableUnit unit)
    (preserved : GlobalsPreserved executable) (integral : FailuresIntegral executable)
    (handle : FunctionHandle) (θ : TypeArgs)
    (free : θ.1.refFree = true) (typeInstantiation : Array (TypeId × TypeId)) (σs : NRow)
    (shape : ResultShape)
    (contract : letI : Skolems unit := Skolems.instantiate θ (Skolems.runtime unit)
      Contract (Memory unit) Failure (HList σs) shape.carrier)
    (coherent : @Coherent unit _ (Skolems.instantiate θ (Skolems.runtime unit)) handle
      typeInstantiation)
    (verified : letI : Skolems unit := Skolems.instantiate θ (Skolems.runtime unit)
      Satisfies (propheticMeaning executable typeInstantiation handle σs shape) contract)
    [validArgs : letI : Skolems unit := Skolems.instantiate θ (Skolems.runtime unit)
      PreservedValidArgs σs contract.validArgs] :
    letI : Skolems unit := Skolems.runtime unit
    SatisfiesFunctionAt executable handle typeInstantiation
      ((contract.ofSkolem θ).prophetic unit (NRow.subst θ.1 σs) (shape.subst θ.1)) :=
  satisfies_generic_at_valid executable preserved integral handle θ free typeInstantiation σs shape
    contract coherent verified validArgs.preserved

end Public

end VerexIR.Proofs.Denote
