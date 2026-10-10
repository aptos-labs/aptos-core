-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.IndexedArena
import LeanerIR.Validation.Validated

/-!
# Loan deaths

The borrow certificates record where each mutable loan dies: before an
anchor expression runs, or after it produced a value, per
[`designs/prophetic-references.md`](../../../designs/prophetic-references.md).
The semantics reads them at the anchor (`loanDeathsAt`), so the validated
unit is the one every consumer reads: its expression indexes are the ones
the interpreter, the big-step relation, and compilation see.

Deaths are conditional at run time: ending a loan the path did not create is
a no-op, so the recorded over-approximation of branch-dependent deaths is
sound.
-/

namespace LeanerIR.Validation

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
      if death.anchor == anchor then deaths.add death.before ⟨index⟩ else deaths

/-- The loan deaths anchored at an expression of a namespace, recorded by
the borrow certificates of its functions. -/
def loanDeathsAt (unit : ValidatedUnit) (namespaceId : NamespaceId) (anchor : ExprId) :
    AnchorDeaths :=
  unit.borrowCertificates.foldl (init := {}) fun deaths certificate =>
    if certificate.namespaceId == namespaceId then loanDeathsIn certificate.loans anchor deaths
    else deaths

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
  unit.borrowCertificates.foldl (init := #[]) fun diagnostics certificate =>
    match unit.namespaces[certificate.namespaceId.index]? with
    | none => diagnostics
    | some ns =>
        let reserved := placeIndexIds ns
        certificate.loans.foldl (init := diagnostics) fun diagnostics loan =>
          loan.deaths.foldl (init := diagnostics) fun diagnostics death =>
            if ns.expressions.size ≤ death.anchor.index then
              diagnostics.push (.error "LIR-SEMANTIC-LOAN-DEATH"
                s!"internal: loan-death anchor {death.anchor.index} is out of range" none)
            else if reserved.contains death.anchor then
              diagnostics.push (.error "LIR-SEMANTIC-LOAN-DEATH"
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
`loanDeathsAt` does at each: a compilation the kernel evaluates looks an
anchor up in logarithmic time instead of scanning every certificate. -/
def deathIndex (unit : ValidatedUnit) (namespaceId : NamespaceId) : KeyTree AnchorDeaths :=
  let entries := unit.borrowCertificates.toList.foldl (init := []) fun entries certificate =>
    if certificate.namespaceId != namespaceId then entries else
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

end LeanerIR.Validation
