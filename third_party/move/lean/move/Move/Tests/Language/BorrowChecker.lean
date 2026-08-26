-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Verify.BorrowChecker

/-!
# Poison-aware source borrow policy

Executable policy and certificate tests, independent of compiler lowering.
-/

namespace Move.Tests.BorrowChecker

open Move.Verify.Borrow

private def localPlace (name : String) (path : Array Step := #[]) : Place :=
  { root := .local name, path }

private def sequence (events : Array Event) : Block :=
  events.zipIdx.foldr (init := .done) fun (event, point) tail =>
    .event point event tail

/-- Overlapping handles may coexist.  Activating one poisons the other, but
dropping the poisoned handle without using it is safe. -/
def dropPoisoned : Program := {
  declaration := "dropPoisoned"
  body := sequence #[
    .borrowMut "left" (localPlace "owner"),
    .borrowMut "right" (localPlace "owner"),
    .write "left",
    .drop "right",
    .drop "left"] }

#guard (analyze dropPoisoned).isOk

def usePoisoned : Program := {
  declaration := "usePoisoned"
  body := sequence #[
    .borrowMut "left" (localPlace "owner"),
    .borrowMut "right" (localPlace "owner"),
    .write "left",
    .read "right"] }

#guard analyze usePoisoned == .error {
  point := 3
  kind := .poisonedUse
  reference? := some "right"
  conflicting? := some "left" }

def immutableBlocksActivation : Program := {
  declaration := "immutableBlocksActivation"
  body := sequence #[
    .borrowImm "observation" (localPlace "owner" #[.field "right"]),
    .borrowMut "writer" (localPlace "owner" #[.field "left"]),
    .write "writer"] }

#guard analyze immutableBlocksActivation == .error {
  point := 2
  kind := .mutationDuringImmutable
  reference? := some "writer"
  conflicting? := some "observation" }

def disjointSiblings : Program := {
  declaration := "disjointSiblings"
  body := sequence #[
    .borrowMut "left" (localPlace "owner" #[.field "left"]),
    .borrowMut "right" (localPlace "owner" #[.field "right"]),
    .write "left",
    .write "right",
    .drop "left",
    .drop "right"] }

#guard (analyze disjointSiblings).isOk

def callPoisonsAlias : Program := {
  declaration := "callPoisonsAlias"
  body := sequence #[
    .borrowMut "left" (localPlace "owner"),
    .borrowMut "right" (localPlace "owner"),
    .call "callee" #[{ reference := "left", effect := .write }],
    .read "right"] }

#guard !(analyze callPoisonsAlias).isOk

def loopCarriesMutation : Program := {
  declaration := "loopCarriesMutation"
  body := .event 0 (.borrowMut "writer" (localPlace "owner")) <|
    .loop 1 (.event 2 (.write "writer") .done) <|
    .event 3 (.drop "writer") .done }

#guard (analyze loopCarriesMutation).isOk

def poisonAcrossBranch : Program := {
  declaration := "poisonAcrossBranch"
  body := .event 0 (.borrowMut "left" (localPlace "owner")) <|
    .event 1 (.borrowMut "right" (localPlace "owner")) <|
    .branch 2 (.event 3 (.write "left") .done) .done <|
    .event 4 (.read "right") .done }

#guard analyze poisonAcrossBranch == .error {
  point := 4
  kind := .poisonedUse
  reference? := some "right"
  conflicting? := some "left" }

def poisonAcrossIteration : Program := {
  declaration := "poisonAcrossIteration"
  body := .event 0 (.borrowMut "left" (localPlace "owner")) <|
    .event 1 (.borrowMut "right" (localPlace "owner")) <|
    .loop 2 (.event 3 (.write "left") .done) <|
    .event 4 (.read "right") .done }

#guard !(analyze poisonAcrossIteration).isOk

def childReconcilesIntoParent : Program := {
  declaration := "childReconcilesIntoParent"
  parameters := #[{ name := "parent", kind := .mutable }]
  body := sequence #[
    .borrowMut "child" { root := .parameter 0, path := #[.field "value"] }
      (some "parent"),
    .write "child",
    .drop "child",
    .write "parent"] }

#guard (analyze childReconcilesIntoParent).isOk

def conditionalCallSeparation : Program := {
  declaration := "conditionalCallSeparation"
  parameters := #[
    { name := "left", kind := .mutable },
    { name := "right", kind := .mutable }]
  body := sequence #[.call "callee" #[
    { reference := "left", parameter := 0, effect := .write },
    { reference := "right", parameter := 1, effect := .read }]
    #[Separation.normalized 0 1]] }

#guard (analyze conditionalCallSeparation).toOption.get!.finalState.requiredSeparations ==
  #[Separation.normalized 0 1]

def concreteCallConflict : Program := {
  declaration := "concreteCallConflict"
  body := sequence #[
    .borrowMut "left" (localPlace "owner"),
    .borrowMut "right" (localPlace "owner"),
    .call "callee" #[
      { reference := "left", parameter := 0, effect := .write },
      { reference := "right", parameter := 1, effect := .read }]
      #[Separation.normalized 0 1]] }

#guard analyze concreteCallConflict == .error {
  point := 2
  kind := .invalidCallEffect
  reference? := some "left"
  conflicting? := some "right" }

def returnedReferenceThroughCall : Program := {
  declaration := "returnedReferenceThroughCall"
  parameters := #[{ name := "input", kind := .immutable }]
  body := sequence #[
    .call "callee" #[{ reference := "input", parameter := 0, effect := .read }]
      #[] #[{ destination := "result", derivation := {
        parameter := 0, path := #[.field "value"], kind := .immutable } }],
    .read "result",
    .returnRef "result"] }

#guard (analyze returnedReferenceThroughCall).toOption.get!.finalState.returns == #[{
  parameter := 0, path := #[.field "value"], kind := .immutable }]

def returnedReferenceCertificate : Certificate :=
  (makeCertificate { returnedReferenceThroughCall with summary := {
    parameterEffects := #[.read]
    returns := #[{
      parameter := 0, path := #[.field "value"], kind := .immutable }] } }).toOption.get!

#guard (returnedReferenceCertificate.check returnedReferenceCertificate.program).isOk

def omittedReturnSummary : Certificate :=
  (makeCertificate returnedReferenceThroughCall).toOption.get!

#guard omittedReturnSummary.check returnedReferenceThroughCall == .error {
  point := 0, kind := .invalidSummary }

def corruptedAnalysis : Certificate :=
  let certificate := (makeCertificate dropPoisoned).toOption.get!
  { certificate with analysis := {
      certificate.analysis with finalState := {
        returns := #[{ parameter := 0, kind := .immutable }] } } }

#guard corruptedAnalysis.check dropPoisoned == .error {
  point := 0, kind := .certificateMismatch }

def validCertificate : Certificate :=
  (makeCertificate dropPoisoned).toOption.get!

#guard (validCertificate.check dropPoisoned).isOk

def corruptedCertificate : Certificate :=
  { validCertificate with version := 2 }

#guard corruptedCertificate.check dropPoisoned == .error {
  point := 0, kind := .certificateMismatch }

theorem certificate_is_proof_evidence : WellBorrowed dropPoisoned :=
  soundChecked (certificate := validCertificate) (by native_decide)

/-! ## Ports from `move-vm/transactional-tests/runtime_ref_checks`

The dynamic monitor permits some bad operations and fails on later poisoned
use.  The static checker rejects at the operation when every continuation is
unsafe (immutable/owner cases), while retaining lazy failure for competing
mutable handles.
-/

def multipleImmutableReferences : Program := {
  declaration := "multiple_immut_refs_allowed"
  body := sequence #[
    .borrowImm "first" (localPlace "owner"),
    .borrowImm "second" (localPlace "owner"),
    .borrowImm "third" (localPlace "owner"),
    .call "read_only" #[
      { reference := "first", parameter := 0, effect := .read },
      { reference := "second", parameter := 1, effect := .read },
      { reference := "third", parameter := 2, effect := .read }]] }

#guard (analyze multipleImmutableReferences).isOk

/-- Runtime poisoning fails on the eventual read.  Static analysis can reject
at activation because the immutable observation is definitely live. -/
def immutableThenWrite : Program := {
  declaration := "poisoned_ref_after_write"
  body := sequence #[
    .borrowImm "observation" (localPlace "owner" #[.field "value"]),
    .borrowMut "writer" (localPlace "owner" #[.field "value"]),
    .write "writer",
    .read "observation"] }

#guard analyze immutableThenWrite == .error {
  point := 2
  kind := .mutationDuringImmutable
  reference? := some "writer"
  conflicting? := some "observation" }

/-- Unlike the runtime's exclusive call lock, a read-only summary does not
activate either overlapping mutable handle. -/
def readOnlyOverlappingCall : Program := {
  declaration := "call_with_overlapping_mut_refs"
  body := sequence #[
    .borrowMut "first" (localPlace "owner"),
    .borrowMut "second" (localPlace "owner"),
    .call "read_only" #[
      { reference := "first", parameter := 0, effect := .ignore },
      { reference := "second", parameter := 1, effect := .ignore }],
    .drop "first",
    .drop "second"] }

#guard (analyze readOnlyOverlappingCall).isOk

def freezePoisoned : Program := {
  declaration := "freeze_ref_poisoned"
  body := sequence #[
    .borrowMut "field" (localPlace "owner" #[.field "value"]),
    .borrowMut "parent" (localPlace "owner"),
    .write "parent",
    .freeze "field" "immutable"] }

#guard !(analyze freezePoisoned).isOk

/-- Runtime `st_loc` poisons and fails on later use; source verification
rejects invalidating the owner immediately because prophecy write-back would
otherwise have no owner. -/
def overwriteBorrowedOwner : Program := {
  declaration := "st_loc_overwrite_poisons"
  body := sequence #[
    .borrowImm "reference" (localPlace "owner"),
    .ownerWrite (localPlace "owner"),
    .read "reference"] }

#guard analyze overwriteBorrowedOwner == .error {
  point := 1
  kind := .ownerInvalidation
  conflicting? := some "reference" }

def returnLocalReference : Program := {
  declaration := "return_local_ref"
  body := sequence #[
    .borrowImm "result" (localPlace "owner"),
    .returnRef "result"] }

#guard !(analyze returnLocalReference).isOk

def returnDerivedReference : Program := {
  declaration := "return_derived_ref_valid"
  parameters := #[{ name := "input", kind := .immutable }]
  body := sequence #[.returnRef "input"] }

#guard (analyze returnDerivedReference).isOk

def vectorIndexAbstraction : Program := {
  declaration := "vector_multiple_elem_refs"
  body := sequence #[
    .borrowMut "zero" (localPlace "values" #[.anyIndex]),
    .borrowMut "one" (localPlace "values" #[.anyIndex]),
    .write "zero",
    .read "one"] }

#guard !(analyze vectorIndexAbstraction).isOk

end Move.Tests.BorrowChecker
