-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.Validated

/-!
# Loan-death materialization

Turns the death records of accepted borrow certificates into explicit
`endLoan` markers in the expression arenas, per
[`designs/prophetic-references.md`](../../../designs/prophetic-references.md).

Marking runs at semantic preparation, not at validation: the validated unit
stays the marker-free authority the printers and exchange consumers read,
while everything that executes or verifies a body receives the marked copy.
Two marker shapes cover the two death flavors:

- an after-death at anchor `a` wraps the anchor: the slot becomes
  `endLoan loans (a')` with the original node moved to a fresh index `a'`,
  so the write-back runs right after the anchor's value is produced;
- a before-death becomes a zero-operand `endLoan` statement in a synthesized
  block around the anchor, so the write-back runs first.

Both rewrites are value-transparent, and markers are conditional at run
time, so the recorded over-approximation of branch-dependent deaths is
sound. Marked arenas contain parent-to-appended-child forward references;
every arena consumer is fuel- or relation-based, and the raw-stage
children-precede-parents invariant is deliberately not re-established.
-/

namespace LeanerIR.Validation

/-- Death records of one namespace, grouped per anchor. -/
private structure AnchorMarks where
  anchor : ExprId
  before : Array LoanId := #[]
  after : Array LoanId := #[]

private def pushLoan (loans : Array LoanId) (loan : LoanId) : Array LoanId :=
  if loans.contains loan then loans else loans.push loan

private def addMark (marks : Array AnchorMarks) (anchor : ExprId)
    (before : Bool) (loan : LoanId) : Array AnchorMarks :=
  match marks.findIdx? (·.anchor == anchor) with
  | some index =>
      marks.modify index fun mark =>
        if before then { mark with before := pushLoan mark.before loan }
        else { mark with after := pushLoan mark.after loan }
  | none =>
      marks.push <|
        if before then { anchor, before := #[loan] }
        else { anchor, after := #[loan] }

/-- Expression ids consumed as place indexes; their arena slots must keep
their literal or local shape, so they may never be wrapped. Death anchors
are operation, branch, or statement nodes, so a collision is an internal
error rather than an expected case. -/
private def placeIndexIds (ns : ValidatedNamespace) : Array ExprId :=
  ns.places.foldl (init := #[]) fun ids place =>
    match place with
    | .index _ index => ids.push index
    | _ => ids

/-- Wrap one anchor slot with its markers. `unitType` types the synthesized
zero-operand marker statements. -/
private def markAnchor (expressions : Array Expr) (unitType : TypeId)
    (mark : AnchorMarks) : Array Expr × Bool :=
  match expressions[mark.anchor.index]? with
  | none => (expressions, false)
  | some node =>
      let expressions := expressions
      -- After-deaths: the anchor moves to a fresh slot and the marker takes
      -- its place, forwarding the value.
      let (expressions, current) :=
        if mark.after.isEmpty then (expressions, node) else
          let moved : ExprId := ⟨expressions.size⟩
          let wrapper := { node with
            kind := .operation (.reference (.endLoan mark.after)) #[] #[moved] }
          (expressions.push node |>.set! mark.anchor.index wrapper, wrapper)
      -- Before-deaths: a marker statement precedes the (possibly wrapped)
      -- anchor inside a synthesized block.
      if mark.before.isEmpty then (expressions, true) else
        let moved : ExprId := ⟨expressions.size⟩
        let marker : Expr := {
          loc := node.loc
          typeId := unitType
          kind := .operation (.reference (.endLoan mark.before)) #[] #[] }
        let markerId : ExprId := ⟨expressions.size + 1⟩
        let wrapper := { node with kind := .block #[markerId] (some moved) }
        (expressions.push current |>.push marker |>.set! mark.anchor.index wrapper,
          true)

/-- Structural scan for the `unit` type: the nested `Ty` inductive's
derived equality does not reduce under whnf, and semantic preparation must
stay a computable value for symbolic execution. -/
private def unitTypeIndex? : List Ty → Nat → Option Nat
  | [], _ => none
  | .unit :: _, index => some index
  | _ :: rest, index => unitTypeIndex? rest (index + 1)

/-- Ensure the tables contain the `unit` type, appending it if absent.
Appending follows the stated tables-append invariant: existing ids stay
stable, and every namespace's cached copy is refreshed. -/
private def ensureUnitType (tables : Tables) : Tables × TypeId :=
  match unitTypeIndex? tables.types.toList 0 with
  | some index => (tables, ⟨index⟩)
  | none => ({ tables with types := tables.types.push .unit }, ⟨tables.types.size⟩)

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

private def replacePlaceCopies : Nat → List Place → List (Nat × Place) → List Place
  | _, [], _ => []
  | index, node :: rest, patches =>
      match patches with
      | (anchor, replacement) :: tail =>
          if index == anchor then
            replacement :: replacePlaceCopies (index + 1) rest
              (tail.dropWhile fun entry => entry.1 == anchor)
          else node :: replacePlaceCopies (index + 1) rest patches
      | [] => node :: rest

/-- Apply ordered copies between place slots, reading each source from the
state established by preceding copies. Keep a sparse overlay until the final
arena merge, so kernel reduction never replays a chain of full-array updates.
The changed-slot bits avoid searching the overlay for untouched source slots.
Stable sorting retains the newest replacement at a repeatedly written slot. -/
def applyPlaceCopies (original : Array Place) (copies : List (Nat × PlaceId)) : Array Place :=
  let (_, patches) := copies.foldl (init := (0, ([] : List (Nat × Place))))
    fun (changed, patches) (index, base) =>
      if index < original.size then
        let value := if base.index < original.size && changed / 2 ^ base.index % 2 != 0 then
            ((patches.find? fun entry => entry.1 == base.index).map (·.2)).getD
              original[base.index]!
          else original[base.index]!
        let changed := if changed / 2 ^ index % 2 == 0 then changed + 2 ^ index else changed
        (changed, (index, value) :: patches)
      else (changed, patches)
  (replacePlaceCopies 0 original.toList (sortByIndex patches)).toArray

/-- Merge unique, sorted replacement slots in one traversal of the original
arena. Repeated `Array.set!` on the growing arena duplicates the computation
of previous updates during kernel reduction. -/
private def replaceAnchors : Nat → List Expr → List (Nat × Expr) → List Expr
  | _, [], _ => []
  | index, node :: rest, patches =>
      match patches with
      | (anchor, replacement) :: tail =>
          if index == anchor then replacement :: replaceAnchors (index + 1) rest tail
          else node :: replaceAnchors (index + 1) rest patches
      | [] => node :: rest

private def anchorNodeNative (original : Array Expr) (index : Nat) : Expr :=
  original[index]!

@[implemented_by anchorNodeNative]
private def anchorNode (original : Array Expr) (index : Nat) : Expr :=
  original.toList[index]!

private theorem anchorNode_eq (original : Array Expr) (index : Nat) :
    anchorNode original index = anchorNodeNative original index := by
  exact Array.getElem!_toList

/-- Batch valid, grouped anchors while preserving the original append order
and therefore every synthesized expression id. Only the final merge touches
the full original arena; each marker inspects its original slot once. -/
private def markAnchorsBatched (original : Array Expr) (unitType : TypeId)
    (reserved : Array ExprId) (marks : Array AnchorMarks)
    (diagnostics : Array Diagnostic) : Array Expr × Array Diagnostic :=
  let (_, appended, patches, diagnostics) := marks.foldl
    (init := (original.size, ([] : List Expr), ([] : List (Nat × Expr)), diagnostics))
    fun (next, appended, patches, diagnostics) mark =>
      if reserved.contains mark.anchor then
        (next, appended, patches, diagnostics.push (.error "LIR-SEMANTIC-LOAN-MARKER"
          s!"internal: loan-death anchor {mark.anchor.index} is a place index" none))
      else
        let node := anchorNode original mark.anchor.index
        let (next, appended, current) := if mark.after.isEmpty then
            (next, appended, node)
          else
            (next + 1, node :: appended, { node with
              kind := .operation (.reference (.endLoan mark.after)) #[] #[⟨next⟩] })
        let (next, appended, current) := if mark.before.isEmpty then
            (next, appended, current)
          else
            let marker : Expr := {
              loc := node.loc
              typeId := unitType
              kind := .operation (.reference (.endLoan mark.before)) #[] #[] }
            (next + 2, marker :: current :: appended,
              { node with kind := .block #[⟨next + 1⟩] (some ⟨next⟩) })
        (next, appended, (mark.anchor.index, current) :: patches, diagnostics)
  let sorted := sortByIndex patches
  ((replaceAnchors 0 original.toList sorted ++ appended.reverse).toArray, diagnostics)

/-- Materialize the death records of every certificate as `endLoan` markers.
The result is the semantic view of the unit; the input stays the marker-free
surface authority. -/
def markLoanDeaths (unit : ValidatedUnit) : ValidatedUnit × Array Diagnostic :=
  let (tables, unitType) := ensureUnitType unit.tables
  let (namespaces, diagnostics) :=
    unit.namespaces.zipIdx.foldl (init := (#[], #[])) fun (namespaces, diagnostics) (ns, index) =>
      let namespaceId : NamespaceId := ⟨index⟩
      let marks := unit.borrowCertificates.foldl (init := #[]) fun marks certificate =>
        if certificate.namespaceId != namespaceId then marks else
          certificate.loans.zipIdx.foldl (init := marks) fun marks (loan, loanIndex) =>
            loan.deaths.foldl (init := marks) fun marks death =>
              addMark marks death.anchor death.before ⟨loanIndex⟩
      let reserved := placeIndexIds ns
      let (expressions, diagnostics) :=
        if marks.all (fun mark => mark.anchor.index < ns.expressions.size) then
          markAnchorsBatched ns.expressions unitType reserved marks diagnostics
        else
        -- Retain the established malformed-certificate diagnostics, including
        -- their ordering, on the uncommon out-of-range path.
        marks.foldl (init := (ns.expressions, diagnostics)) fun (expressions, diagnostics) mark =>
          if reserved.contains mark.anchor then
            (expressions, diagnostics.push (.error "LIR-SEMANTIC-LOAN-MARKER"
              s!"internal: loan-death anchor {mark.anchor.index} is a place index"
              none))
          else
            let (expressions, applied) := markAnchor expressions unitType mark
            if applied then (expressions, diagnostics)
            else (expressions, diagnostics.push (.error "LIR-SEMANTIC-LOAN-MARKER"
              s!"internal: loan-death anchor {mark.anchor.index} is out of range"
              none))
      (namespaces.push { ns with expressions, tables }, diagnostics)
  let marked := Internal.mkValidatedUnit tables unit.profiles namespaces
    unit.dependencies unit.evidence unit.indexes unit.structurizationWitnesses
    unit.resolution unit.initializationCertificates unit.borrowCertificates
    unit.borrowDiagnostics
  (marked, diagnostics)

end LeanerIR.Validation
