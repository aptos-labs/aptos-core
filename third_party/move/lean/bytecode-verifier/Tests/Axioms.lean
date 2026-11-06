-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs

/-!
# Axiom audit (plan §9.4)

Every theorem of the proof layer may depend only on Lean's standard axioms
(`propext`, `Classical.choice`, `Quot.sound`). Native contracts, entry
premises and acyclicity are hypotheses in theorem statements, never axioms.
Each main theorem is listed here with its exact axiom set, so a new
dependency fails the test suite.
-/

/--
info: 'MoveBytecode.Proofs.progress' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.progress

/--
info: 'MoveBytecode.Proofs.stackDiscipline_of_verifyFunction' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.stackDiscipline_of_verifyFunction

/--
info: 'MoveBytecode.Proofs.noWrong_of_certified' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.noWrong_of_certified

/--
info: 'MoveBytecode.Proofs.constants_decode' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.constants_decode

/--
info: 'MoveBytecode.Proofs.typeCertificate' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.typeCertificate



/--
info: 'MoveBytecode.Proofs.soundness_byValue' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.soundness_byValue

/--
info: 'MoveBytecode.Proofs.certificatesHold_byValue' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.certificatesHold_byValue

/--
info: 'MoveBytecode.Proofs.soundness_M3' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.soundness_M3

/--
info: 'MoveBytecode.Proofs.soundness_M2' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.soundness_M2

/--
info: 'MoveBytecode.Proofs.soundness_M1' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.soundness_M1

/--
info: 'MoveBytecode.Proofs.certified_of_wellFormed' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.certified_of_wellFormed

/--
info: 'MoveBytecode.Proofs.refCert' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.refCert

/--
info: 'MoveBytecode.Proofs.indexIntegrity_of_accepted' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.indexIntegrity_of_accepted

/--
info: 'MoveBytecode.Proofs.uniqueness_of_duplication' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.uniqueness_of_duplication

/--
info: 'MoveBytecode.Proofs.stackDiscipline_of_accepted' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.stackDiscipline_of_accepted

/--
info: 'MoveBytecode.Proofs.constantsWellFormed_of_accepted' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.constantsWellFormed_of_accepted

/--
info: 'MoveBytecode.Proofs.instructionConsistency_of_accepted' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.instructionConsistency_of_accepted

/--
info: 'MoveBytecode.Proofs.entriesAdmissible' depends on axioms: [propext, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.entriesAdmissible

/--
info: 'MoveBytecode.Proofs.transactionEntriesAdmissible' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.transactionEntriesAdmissible

/--
info: 'MoveBytecode.Proofs.nativeContractsSatisfiable' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.nativeContractsSatisfiable

/--
info: 'MoveBytecode.Proofs.back_dominated' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.back_dominated

/--
info: 'MoveBytecode.Proofs.summary_facts' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.summary_facts

/--
info: 'MoveBytecode.Proofs.reducible_of_verified' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.reducible_of_verified


/--
info: 'MoveBytecode.Proofs.structsAgree' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.structsAgree


/--
info: 'MoveBytecode.Proofs.typeFormation_of_accepted' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.typeFormation_of_accepted

/--
info: 'MoveBytecode.Proofs.structDefinitionsWellFormed_of_accepted' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.structDefinitionsWellFormed_of_accepted

/--
info: 'MoveBytecode.Proofs.friendsWellFormed_of_accepted' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.friendsWellFormed_of_accepted

/--
info: 'MoveBytecode.Proofs.localProperties_of_phantom' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.localProperties_of_phantom

/--
info: 'MoveBytecode.Proofs.phantomUnderReferences_of_switch' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.phantomUnderReferences_of_switch

/--
info: 'MoveBytecode.Proofs.localProperties_of_switch' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.localProperties_of_switch

/--
info: 'MoveBytecode.Proofs.certificatesConsistent' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.certificatesConsistent

/--
info: 'MoveBytecode.Proofs.variantFieldsShared_of_accepted' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.variantFieldsShared_of_accepted

/--
info: 'MoveBytecode.Proofs.exec_annot' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.exec_annot

/--
info: 'MoveBytecode.Proofs.coreStep_annot' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.coreStep_annot

/--
info: 'MoveBytecode.Proofs.typed_core' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.typed_core

/--
info: 'MoveBytecode.Proofs.typed_native' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.typed_native

/--
info: 'MoveBytecode.Proofs.nativeAnnotated' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.nativeAnnotated

/--
info: 'MoveBytecode.Proofs.entry_inv' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.entry_inv

/--
info: 'MoveBytecode.Proofs.ownershipAccounting' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.ownershipAccounting

/--
info: 'MoveBytecode.Proofs.encapsulation' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.encapsulation

/--
info: 'MoveBytecode.Proofs.noDanglingFromContainment' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.noDanglingFromContainment

/--
info: 'MoveBytecode.Proofs.mutableContained_of_steps' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.mutableContained_of_steps

/--
info: 'MoveBytecode.Proofs.mutableContained_M3' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.mutableContained_M3

/--
info: 'MoveBytecode.Proofs.noDanglingUse_M3' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.noDanglingUse_M3

/--
info: 'MoveBytecode.Proofs.immutableStable_M3' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.immutableStable_M3

/--
info: 'MoveBytecode.Proofs.deadSlotsUnborrowed_M3' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.deadSlotsUnborrowed_M3

/--
info: 'MoveBytecode.Proofs.unnested_M3' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.unnested_M3

/--
info: 'MoveBytecode.Proofs.unnestedAfter_M3' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.unnestedAfter_M3

/--
info: 'MoveBytecode.Proofs.noEscape_M3' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.noEscape_M3

/--
info: 'MoveBytecode.Proofs.parametersExclusive_M3' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.parametersExclusive_M3

/--
info: 'MoveBytecode.Proofs.referenceSafety_M3' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.referenceSafety_M3

/--
info: 'MoveBytecode.Proofs.soundness_nonGeneric' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.soundness_nonGeneric

/--
info: 'MoveBytecode.Proofs.referenceSafety_nonGeneric' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.referenceSafety_nonGeneric

/--
info: 'MoveBytecode.Proofs.noDanglingUse_all' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.noDanglingUse_all

/--
info: 'MoveBytecode.Proofs.noEscape_all' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.noEscape_all

/--
info: 'MoveBytecode.Proofs.parametersExclusive_all' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.parametersExclusive_all

/--
info: 'MoveBytecode.Proofs.instantiationsFinite_of' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.instantiationsFinite_of

/--
info: 'MoveBytecode.Proofs.loopDepthAtMost_of' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.loopDepthAtMost_of

/--
info: 'MoveBytecode.Proofs.withinLimits_of' depends on axioms: [propext, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.withinLimits_of

/--
info: 'MoveBytecode.Proofs.localPropertiesNonGeneric_of_phantom' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.localPropertiesNonGeneric_of_phantom

/--
info: 'MoveBytecode.Proofs.linkedProperties' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.linkedProperties

/--
info: 'MoveBytecode.Proofs.translate_wf' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.translate_wf

/--
info: 'MoveBytecode.Proofs.abilities_sub_gen' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.abilities_sub_gen

/--
info: 'MoveBytecode.Proofs.ofTokenWith_instantiate' depends on axioms: [propext, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.ofTokenWith_instantiate

/--
info: 'MoveBytecode.Proofs.soundness_byValue_gen' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.soundness_byValue_gen

/--
info: 'MoveBytecode.Proofs.certificatesHold_byValue_gen' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.certificatesHold_byValue_gen

/--
info: 'MoveBytecode.Proofs.abilitiesSound_gen' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.abilitiesSound_gen

/--
info: 'MoveBytecode.Proofs.modelTypes_consistent_gen' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.modelTypes_consistent_gen

/--
info: 'MoveBytecode.Proofs.soundness' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.soundness

/--
info: 'MoveBytecode.Proofs.certificatesHold' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.certificatesHold

/--
info: 'MoveBytecode.Proofs.referenceSafety' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.referenceSafety

/--
info: 'MoveBytecode.Proofs.modelAvail_drops_gen' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.modelAvail_drops_gen

/--
info: 'MoveBytecode.Proofs.functionValueSafety' depends on axioms: [propext, Classical.choice, Quot.sound]
-/
#guard_msgs in
#print axioms MoveBytecode.Proofs.functionValueSafety
