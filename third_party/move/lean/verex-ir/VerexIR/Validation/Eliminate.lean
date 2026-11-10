-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Validation.IndexedArena
import VerexIR.Validation.Validated

/-!
# Loan deaths

The borrow certificates record where each mutable loan dies: before an
anchor expression runs, or after it produced a value, per
[`doc/designs/prophetic-references.md`](../../../designs/prophetic-references.md).
The semantics reads them at the anchor (`loanDeathsAt`), so the validated
unit is the one every consumer reads: its expression indexes are the ones
the interpreter, the big-step relation, and compilation see.

Deaths are conditional at run time: ending a loan the path did not create is
a no-op, so the recorded over-approximation of branch-dependent deaths is
sound.
-/

namespace VerexIR.Validation

/-- The loans that end at one anchor: `before` ones before its node runs,
`after` ones once it produced a value, each in minting order. -/
structure AnchorDeaths where
  before : Array LoanId := #[]
  after : Array LoanId := #[]
  deriving Repr, BEq, Inhabited

private def pushLoan (loans : Array LoanId) (loan : LoanId) : Array LoanId :=
  if loans.contains loan then loans else loans.push loan

/-- Record a loan's death at an anchor. -/
def AnchorDeaths.add (deaths : AnchorDeaths) (before : Bool) (loan : LoanId) : AnchorDeaths :=
  if before then { deaths with before := pushLoan deaths.before loan }
  else { deaths with after := pushLoan deaths.after loan }

/-- The deaths a function's loans record at an anchor. -/
def loanDeathsIn (loans : Array CheckedLoanFact) (anchor : ExprId) (deaths : AnchorDeaths := {}) :
    AnchorDeaths :=
  loans.toList.zipIdx.foldl (init := deaths) fun deaths (loan, index) =>
    loan.deaths.toList.foldl (init := deaths) fun deaths death =>
      if death.anchor == anchor && !death.alone then deaths.add death.before ⟨index⟩ else deaths

/-- The places dying alone at an anchor, the loans they hold going on: a local and the
path from it to the reference it holds. -/
structure DeadPlaces where
  before : Array (LocalId × List LoanProjection) := #[]
  after : Array (LocalId × List LoanProjection) := #[]
  deriving Inhabited

/-- The concrete places named by one carrier death. Explicit nested paths
win over the holder's root, even when that root is itself a reference. -/
def LoanDeath.places (death : LoanDeath) : Array (LocalId × List LoanProjection) :=
  death.holders.toList.flatMap (fun holder =>
    match death.holderPaths.toList.filter (·.1 == holder) with
    | [] => [(holder, [])]
    | paths => paths.map fun path => (holder, path.2.toList)) |>.toArray

/-- Add one carrier death to its side of an anchor. -/
def DeadPlaces.add (dead : DeadPlaces) (death : LoanDeath) : DeadPlaces :=
  if death.before then { dead with before := dead.before ++ death.places }
  else { dead with after := dead.after ++ death.places }

/-- The places a function's loans record dying alone at an anchor. -/
def deadPlacesIn (loans : Array CheckedLoanFact) (anchor : ExprId) (dead : DeadPlaces := {}) :
    DeadPlaces :=
  loans.foldl (init := dead) fun dead loan =>
    loan.deaths.foldl (init := dead) fun dead death =>
      if death.anchor != anchor || !death.alone then dead else dead.add death

/-- Carrier retirements do not need a locally minted loan identity. -/
def carrierDeathsIn (deaths : Array LoanDeath) (anchor : ExprId) (dead : DeadPlaces := {}) :
    DeadPlaces :=
  deaths.foldl (init := dead) fun dead death =>
    if death.anchor == anchor then dead.add death else dead

/-- The places dying alone at an expression of a namespace. -/
def deadPlacesAt (unit : ValidatedUnit) (namespaceId : NamespaceId) (anchor : ExprId) :
    DeadPlaces :=
  unit.borrowCertificates.foldl (init := {}) fun dead certificate =>
    if certificate.namespaceId == namespaceId then
      carrierDeathsIn certificate.carrierDeaths anchor (deadPlacesIn certificate.loans anchor dead)
    else dead

/-- The loan deaths anchored at an expression of a namespace, recorded by
the borrow certificates of its functions. -/
def loanDeathsAt (unit : ValidatedUnit) (namespaceId : NamespaceId) (anchor : ExprId) :
    AnchorDeaths :=
  match unit.namespaces[namespaceId.index]? with
  | some ns => ns.borrowCertificates.foldl (init := {}) fun deaths certificate =>
      loanDeathsIn certificate.loans anchor deaths
  | none => {}

/-- Expression ids consumed as place indexes. A place reads its index in
place, so a death anchored there would never run. -/
private def placeIndexIds (ns : ValidatedNamespace) : List ExprId :=
  ns.places.toList.foldl (init := []) fun ids place =>
    match place with
    | .index _ index => index :: ids
    | _ => ids

/-- Deaths the semantics would never reach: anchored outside their
namespace's arena, or at a place index. Borrow analysis anchors deaths at
operation, branch, or statement nodes, so either is an internal error. -/
def loanDeathDiagnostics (unit : ValidatedUnit) : Array Diagnostic :=
  unit.namespaces.foldl (init := #[]) fun diagnostics ns =>
    let reserved := placeIndexIds ns
    ns.borrowCertificates.foldl (init := diagnostics) fun diagnostics certificate =>
      let deaths := certificate.carrierDeaths ++ certificate.loans.flatMap (·.deaths)
      deaths.foldl (init := diagnostics) fun diagnostics death =>
        if ns.expressions.size ≤ death.anchor.index then
          diagnostics.push (.error "VIR-SEMANTIC-LOAN-DEATH"
            s!"internal: loan-death anchor {death.anchor.index} is out of range" none)
        else if reserved.contains death.anchor then
          diagnostics.push (.error "VIR-SEMANTIC-LOAN-DEATH"
            s!"internal: loan-death anchor {death.anchor.index} is a place index" none)
        else diagnostics

private def mergeByIndex {α : Type} :
    Nat → List (Nat × α) → List (Nat × α) → List (Nat × α)
  | 0, left, right => left ++ right
  | _ + 1, [], right => right
  | _ + 1, left, [] => left
  | fuel + 1, (i, a) :: left, (j, b) :: right =>
      if i ≤ j then (i, a) :: mergeByIndex fuel left ((j, b) :: right)
      else (j, b) :: mergeByIndex fuel ((i, a) :: left) right

private def sortByIndexFuel {α : Type} : Nat → List (Nat × α) → List (Nat × α)
  | 0, entries => entries
  | _ + 1, [] => []
  | _ + 1, [entry] => [entry]
  | fuel + 1, entries =>
      let size := entries.length
      let middle := size / 2
      mergeByIndex size
        (sortByIndexFuel fuel (entries.take middle))
        (sortByIndexFuel fuel (entries.drop middle))

/-- Stable O(n log n) index sorting with structural recursion throughout.
Unlike the library's well-founded merge sort, this reduces directly in
kernel-checked preparation certificates without an accessibility proof. -/
def sortByIndex {α : Type} (entries : List (Nat × α)) : List (Nat × α) :=
  sortByIndexFuel entries.length entries

/-- The loan deaths of a namespace indexed by anchor, answering what
`loanDeathsAt` does at each, and the deaths of places alone, which only a compilation
reads: it looks an anchor up in logarithmic time instead of scanning every certificate. -/
def deathIndex (unit : ValidatedUnit) (namespaceId : NamespaceId) : KeyTree AnchorDeaths :=
  let certificates := ((unit.namespaces[namespaceId.index]?).map (·.borrowCertificates)).getD #[]
  let entries := certificates.toList.foldl (init := []) fun entries certificate =>
    certificate.loans.toList.zipIdx.foldl (init := entries) fun entries (loan, index) =>
      loan.deaths.toList.foldl (init := entries) fun entries death =>
        (death.anchor.index, (death.before, (⟨index⟩ : LoanId))) :: entries
  -- The sort is stable: an anchor's loans stay in the order `loanDeathsAt`
  -- visits them.
  let groups := (sortByIndex entries.reverse).foldl (init := []) fun groups (anchor, before, loan) =>
    match groups with
    | (last, deaths) :: rest =>
        if last == anchor then (last, deaths.add before loan) :: rest
        else (anchor, AnchorDeaths.add {} before loan) :: groups
    | [] => [(anchor, AnchorDeaths.add {} before loan)]
  KeyTree.ofSorted groups.reverse

/-- Caller-lent carrier retirements indexed without synthetic local loan ids. -/
def carrierDeathIndex (unit : ValidatedUnit) (namespaceId : NamespaceId) : KeyTree DeadPlaces :=
  let certificates := ((unit.namespaces[namespaceId.index]?).map (·.borrowCertificates)).getD #[]
  let entries := certificates.toList.flatMap fun certificate =>
    certificate.carrierDeaths.toList.map fun death => (death.anchor.index, death)
  let groups := (sortByIndex entries).foldl (init := []) fun groups (anchor, death) =>
    match groups with
    | (last, dead) :: rest =>
        if last == anchor then (last, dead.add death) :: rest
        else (anchor, DeadPlaces.add {} death) :: groups
    | [] => [(anchor, DeadPlaces.add {} death)]
  KeyTree.ofSorted groups.reverse

end VerexIR.Validation
