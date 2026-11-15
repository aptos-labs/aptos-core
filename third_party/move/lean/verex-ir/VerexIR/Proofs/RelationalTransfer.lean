-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Contract

/-!
# Contract transfer between related interpretations

The verification and concrete interpretations may have different arguments,
results, failures, and states. A concrete computation is covered by the
verification computation when each concrete outcome has a related verification
outcome. Concrete undefinedness must remain undefined: dropping a failed
certification cannot establish a concrete weakest precondition.

The refinement below is an explicit, contract-independent premise. This module
proves how to use it; it does not assert refinement of compiled functions,
native meanings, or a change of carrier interpretation.
-/

namespace VerexIR.Proofs

/-- Outcome preservation at one pair of initial states. A normal outcome
carries a related result and final state; an abort carries a related failure.
Undefinedness is preserved even when no normal or abort outcome exists. -/
structure Spec.RelationalRefinesAt
    (stateRel : ConcreteState → GivenState → Prop)
    (resultRel : ConcreteResult → GivenResult → Prop)
    (errorRel : ConcreteError → GivenError → Prop)
    (concrete : Spec ConcreteState ConcreteError ConcreteResult)
    (given : Spec GivenState GivenError GivenResult)
    (concreteInitial : ConcreteState) (givenInitial : GivenState) : Prop where
  ok : ∀ result final, concrete.ok concreteInitial result final →
    ∃ givenResult givenFinal, given.ok givenInitial givenResult givenFinal ∧
      resultRel result givenResult ∧ stateRel final givenFinal
  aborts : ∀ error, concrete.aborts concreteInitial error →
    ∃ givenError, given.aborts givenInitial givenError ∧ errorRel error givenError
  undefined : concrete.undefined concreteInitial → given.undefined givenInitial

/-- Function refinement on related arguments and admissible concrete starts.
Admissibility is supplied separately from any contract: it may state the
source, loan, storage, or external-meaning conditions needed by a simulation.
The actual relation is an explicit theorem premise, never an instance or axiom. -/
def FunctionRelationalRefines
    (argsRel : ConcreteArgs → GivenArgs → Prop)
    (stateRel : ConcreteState → GivenState → Prop)
    (resultRel : ConcreteResult → GivenResult → Prop)
    (errorRel : ConcreteError → GivenError → Prop)
    (admissible : ConcreteArgs → ConcreteState → Prop)
    (concrete : ConcreteArgs → Spec ConcreteState ConcreteError ConcreteResult)
    (given : GivenArgs → Spec GivenState GivenError GivenResult) : Prop :=
  ∀ args givenArgs initial givenInitial,
    argsRel args givenArgs → stateRel initial givenInitial → admissible args initial →
      Spec.RelationalRefinesAt stateRel resultRel errorRel
        (concrete args) (given givenArgs) initial givenInitial

/-- A verified weakest precondition transfers along an explicit outcome
refinement when its normal and failure predicates transfer along the relations. -/
theorem wp_of_relationalRefinesAt
    {stateRel : ConcreteState → GivenState → Prop}
    {resultRel : ConcreteResult → GivenResult → Prop}
    {errorRel : ConcreteError → GivenError → Prop}
    {concrete : Spec ConcreteState ConcreteError ConcreteResult}
    {given : Spec GivenState GivenError GivenResult}
    {initial : ConcreteState} {givenInitial : GivenState}
    (refines : Spec.RelationalRefinesAt stateRel resultRel errorRel
      concrete given initial givenInitial)
    {ensures : ConcreteResult → ConcreteState → Prop}
    {givenEnsures : GivenResult → GivenState → Prop}
    {aborts : ConcreteError → Prop} {givenAborts : GivenError → Prop}
    (normal : ∀ result givenResult final givenFinal,
      resultRel result givenResult → stateRel final givenFinal →
      givenEnsures givenResult givenFinal → ensures result final)
    (failure : ∀ error givenError, errorRel error givenError →
      givenAborts givenError → aborts error)
    (verified : wp given givenEnsures givenAborts givenInitial) :
    wp concrete ensures aborts initial := by
  refine ⟨?_, ?_, ?_⟩
  · intro result final execution
    obtain ⟨givenResult, givenFinal, givenExecution, relatedResult, relatedFinal⟩ :=
      refines.ok result final execution
    exact normal result givenResult final givenFinal relatedResult relatedFinal
      (verified.1 givenResult givenFinal givenExecution)
  · intro error execution
    obtain ⟨givenError, givenExecution, relatedError⟩ := refines.aborts error execution
    exact failure error givenError relatedError (verified.2.1 givenError givenExecution)
  · intro undefined
    exact verified.2.2 (refines.undefined undefined)

/-- Compatibility of two contracts under the relations. Preconditions and
assumptions pass to the verification interpretation; guarantees pass back.
A verification-side excuse for the postcondition must also excuse the concrete
postcondition. Conversely, a concrete mandatory abort must be mandatory on the
verification side, so a verified normal outcome refutes it. -/
structure Contract.RelationalTransfer
    (argsRel : ConcreteArgs → GivenArgs → Prop)
    (stateRel : ConcreteState → GivenState → Prop)
    (resultRel : ConcreteResult → GivenResult → Prop)
    (errorRel : ConcreteError → GivenError → Prop)
    (concrete : Contract ConcreteState ConcreteError ConcreteArgs ConcreteResult)
    (given : Contract GivenState GivenError GivenArgs GivenResult) : Prop where
  assumes : ∀ args givenArgs initial givenInitial,
    argsRel args givenArgs → stateRel initial givenInitial →
    concrete.assumes args initial → given.assumes givenArgs givenInitial
  requires : ∀ args givenArgs initial givenInitial,
    argsRel args givenArgs → stateRel initial givenInitial →
    concrete.requires args initial → given.requires givenArgs givenInitial
  ensures : ∀ args givenArgs initial givenInitial result givenResult final givenFinal,
    argsRel args givenArgs → stateRel initial givenInitial →
    resultRel result givenResult → stateRel final givenFinal →
    given.ensures givenArgs givenInitial givenResult givenFinal →
    concrete.ensures args initial result final
  frame : ∀ args givenArgs initial givenInitial result givenResult final givenFinal,
    argsRel args givenArgs → stateRel initial givenInitial →
    resultRel result givenResult → stateRel final givenFinal →
    given.frame givenArgs givenInitial givenResult givenFinal →
    concrete.frame args initial result final
  aborts : ∀ args givenArgs initial givenInitial error givenError,
    argsRel args givenArgs → stateRel initial givenInitial → errorRel error givenError →
    given.aborts givenArgs givenInitial givenError → concrete.aborts args initial error
  mayAbort : ∀ args givenArgs initial givenInitial,
    argsRel args givenArgs → stateRel initial givenInitial →
    given.mayAbort givenArgs givenInitial → concrete.mayAbort args initial
  mustAbort : ∀ args givenArgs initial givenInitial,
    argsRel args givenArgs → stateRel initial givenInitial →
    concrete.mustAbort args initial → given.mustAbort givenArgs givenInitial

/-- A given contract establishes a concrete contract only with an explicit
refinement, contract compatibility, and coverage of every permitted concrete
start. Neither coverage nor admissibility is inferred from the function body.
In particular, this does not require a total bijection of plain mutable views. -/
theorem Contract.satisfies_of_relationalRefines
    {argsRel : ConcreteArgs → GivenArgs → Prop}
    {stateRel : ConcreteState → GivenState → Prop}
    {resultRel : ConcreteResult → GivenResult → Prop}
    {errorRel : ConcreteError → GivenError → Prop}
    {admissible : ConcreteArgs → ConcreteState → Prop}
    {concrete : ConcreteArgs → Spec ConcreteState ConcreteError ConcreteResult}
    {given : GivenArgs → Spec GivenState GivenError GivenResult}
    {concreteContract : Contract ConcreteState ConcreteError ConcreteArgs ConcreteResult}
    {givenContract : Contract GivenState GivenError GivenArgs GivenResult}
    (refines : FunctionRelationalRefines argsRel stateRel resultRel errorRel
      admissible concrete given)
    (compatible : Contract.RelationalTransfer argsRel stateRel resultRel errorRel
      concreteContract givenContract)
    (admissibleStarts : ∀ args initial, concreteContract.assumes args initial →
      concreteContract.requires args initial → admissible args initial)
    (relatedStarts : ∀ args initial, concreteContract.assumes args initial →
      concreteContract.requires args initial →
      ∃ givenArgs givenInitial, argsRel args givenArgs ∧ stateRel initial givenInitial)
    (verified : Satisfies given givenContract) : Satisfies concrete concreteContract := by
  intro args initial assumed permitted
  obtain ⟨givenArgs, givenInitial, relatedArgs, relatedInitial⟩ :=
    relatedStarts args initial assumed permitted
  have refined := refines args givenArgs initial givenInitial relatedArgs relatedInitial
    (admissibleStarts args initial assumed permitted)
  have established := verified givenArgs givenInitial
    (compatible.assumes args givenArgs initial givenInitial relatedArgs relatedInitial assumed)
    (compatible.requires args givenArgs initial givenInitial relatedArgs relatedInitial permitted)
  refine wp_of_relationalRefinesAt refined ?_ ?_ established
  · intro result givenResult final givenFinal relatedResult relatedFinal supplied
    refine ⟨?_, ?_, ?_⟩
    · intro noAbort
      apply compatible.ensures args givenArgs initial givenInitial result givenResult final givenFinal
        relatedArgs relatedInitial relatedResult relatedFinal
      apply supplied.1
      intro excuse
      exact noAbort (compatible.mayAbort args givenArgs initial givenInitial
        relatedArgs relatedInitial excuse)
    · exact compatible.frame args givenArgs initial givenInitial result givenResult final givenFinal
        relatedArgs relatedInitial relatedResult relatedFinal supplied.2.1
    · intro mustAbort
      exact supplied.2.2 (compatible.mustAbort args givenArgs initial givenInitial
        relatedArgs relatedInitial mustAbort)
  · intro error givenError relatedError supplied
    exact compatible.aborts args givenArgs initial givenInitial error givenError
      relatedArgs relatedInitial relatedError supplied

/-- Explicit evidence for exporting one given proof. The relations and the
admissible starts are part of the evidence, so a generated theorem need not
identify the two interpretations' representations. The refinement field is
contract-independent; the other fields establish that this particular contract
can be exported. There is deliberately no concrete `Satisfies` field. -/
structure Contract.RelationalCertificate
    (concrete : ConcreteArgs → Spec ConcreteState ConcreteError ConcreteResult)
    (given : GivenArgs → Spec GivenState GivenError GivenResult)
    (concreteContract : Contract ConcreteState ConcreteError ConcreteArgs ConcreteResult)
    (givenContract : Contract GivenState GivenError GivenArgs GivenResult) where
  argsRel : ConcreteArgs → GivenArgs → Prop
  stateRel : ConcreteState → GivenState → Prop
  resultRel : ConcreteResult → GivenResult → Prop
  errorRel : ConcreteError → GivenError → Prop
  admissible : ConcreteArgs → ConcreteState → Prop
  refines : FunctionRelationalRefines argsRel stateRel resultRel errorRel
    admissible concrete given
  compatible : Contract.RelationalTransfer argsRel stateRel resultRel errorRel
    concreteContract givenContract
  admissibleStarts : ∀ args initial, concreteContract.assumes args initial →
    concreteContract.requires args initial → admissible args initial
  relatedStarts : ∀ args initial, concreteContract.assumes args initial →
    concreteContract.requires args initial →
    ∃ givenArgs givenInitial, argsRel args givenArgs ∧ stateRel initial givenInitial

/-- Apply explicit relational evidence to a separately established given proof.
Generated exports may take `Nonempty (RelationalCertificate ...)` as a named
propositional hypothesis; no instance supplies this evidence automatically. -/
theorem Contract.RelationalCertificate.transfer
    {concrete : ConcreteArgs → Spec ConcreteState ConcreteError ConcreteResult}
    {given : GivenArgs → Spec GivenState GivenError GivenResult}
    {concreteContract : Contract ConcreteState ConcreteError ConcreteArgs ConcreteResult}
    {givenContract : Contract GivenState GivenError GivenArgs GivenResult}
    (certificate : Nonempty (Contract.RelationalCertificate
      concrete given concreteContract givenContract))
    (verified : Satisfies given givenContract) : Satisfies concrete concreteContract := by
  obtain ⟨certificate⟩ := certificate
  exact Contract.satisfies_of_relationalRefines certificate.refines certificate.compatible
    certificate.admissibleStarts certificate.relatedStarts verified

end VerexIR.Proofs
