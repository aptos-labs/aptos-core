-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Import.Raw
import VerexIR.Validation.Validated

/-!
# Place borrows no local holds

A mutable borrow of a place that no local holds dies where it is used: it is
dereferenced, frozen, read through a field or variant, compared, or
discarded. Nothing is written through it, so it reads the place, but its
loan dies without a local the semantics could settle from (`settleLoans`),
and the lender would keep the loan's hole. After the borrow analysis has
checked it as the mutable borrow it is, keeping its conflicts with other
loans, such a borrow is read as a shared borrow of its place: the place's
value, which the semantics reads without minting a loan. Its loan fact
leaves the certificate. The shared reference types these borrows take are
interned before validation (`internSharedViews`), so the unit's tables are
fixed once validation starts.
-/

namespace VerexIR.Validation

open VerexIR.Import

/-- The shared reference to a mutable reference's referent. -/
private def sharedView? : Ty → Option Ty
  | .reference reference =>
      if reference.kind == .mutable then some (.reference { reference with kind := .shared })
      else none
  | _ => none

/-- Whether a node reads through one operand that is a mutable place borrow
(`readsThroughReference`), or discards it as a statement. -/
private def readsThroughOperand (types : Array Ty) (parent : Expr) (operand : ExprId) : Bool :=
  match parent.kind with
  | .operation operation _ arguments _ =>
      let selected := match operation with
        | .reference _ | .data _ => arguments[0]? == some operand
        | _ => arguments.contains operand
      selected && readsThroughReference types operation parent.typeId
  | .block statements _ => statements.contains operand
  | _ => false

/-- One mutable place borrow no local holds: its expression, the shared
view its type takes, and the wildcard discarding it, if one does. -/
private structure UnheldBorrow where
  expression : Nat
  view : Ty
  wildcard : Option PatternId

/-- The mutable place borrows of an arena that no local holds: each has one
use, by a node that reads through it or discards it. -/
private def unheldBorrows (types : Array Ty) (expressions : Array Expr) (patterns : Array Pattern)
    (places : Array Place) : Array UnheldBorrow := Id.run do
  let mut uses : Array Nat := Array.replicate expressions.size 0
  let mut parent : Array (Option Nat) := Array.replicate expressions.size none
  for (expression, index) in expressions.zipIdx do
    for child in expressionChildren expression.kind do
      if h : child.index < uses.size then
        uses := uses.set child.index (uses[child.index] + 1)
        parent := parent.set! child.index (some index)
  for place in places do
    if let .index _ index := place then
      if h : index.index < uses.size then uses := uses.set index.index (uses[index.index] + 1)
  let wildcard (pattern : PatternId) := patterns[pattern.index]? matches some { kind := .wildcard, .. }
  let mut found := #[]
  for (expression, index) in expressions.zipIdx do
    let .operation (.borrow .mutable _) #[] #[] _ := expression.kind | continue
    let some view := (types[expression.typeId.index]?).bind sharedView? | continue
    unless uses[index]? == some 1 do continue
    let some (some parentIndex) := parent[index]? | continue
    let some parentExpression := expressions[parentIndex]? | continue
    let discarding := match parentExpression.kind with
      | .letDecl pattern (some initializer) _ =>
          if initializer.index == index && wildcard pattern then some pattern else none
      | .assignPattern pattern value =>
          if value.index == index && wildcard pattern then some pattern else none
      | _ => none
    if discarding.isSome || readsThroughOperand types parentExpression ⟨index⟩ then
      found := found.push { expression := index, view, wildcard := discarding }
  return found

/-- The tables with the shared view each unheld mutable place borrow takes. -/
def internSharedViews (unit : RawUnit) : RawUnit :=
  let types := unit.namespaces.foldl (init := unit.tables.types) fun types ns =>
    (unheldBorrows unit.tables.types ns.expressions ns.patterns ns.places).foldl
      (init := types) fun types borrow =>
        if types.contains borrow.view then types else types.push borrow.view
  { unit with tables := { unit.tables with types } }

/-- A namespace's mutable place borrows no local holds read as shared borrows
of their places, with the wildcards that discard them retyped; and the
expressions of the borrows so read. -/
def foldPlaceBorrows (ns : ValidatedNamespace) : ValidatedNamespace × Array ExprId := Id.run do
  let types := ns.tables.types
  let mut expressions := ns.expressions
  let mut patterns := ns.patterns
  let mut folded := #[]
  for borrow in unheldBorrows types ns.expressions ns.patterns ns.places do
    let some viewId := types.findIdx? (· == borrow.view) | continue
    let some expression := ns.expressions[borrow.expression]? | continue
    let .operation (.borrow .mutable place) _ _ surface := expression.kind | continue
    expressions := expressions.set! borrow.expression { expression with
      typeId := ⟨viewId⟩, kind := .operation (.borrow .immutable place) #[] #[] surface }
    if let some pattern := borrow.wildcard then
      patterns := patterns.modify pattern.index ({ · with typeId := ⟨viewId⟩ })
    folded := folded.push ⟨borrow.expression⟩
  return ({ ns with expressions, patterns }, folded)

/-- The certificate without the loans of the borrows read as shared ones. -/
def BorrowCertificate.withoutLoans (certificate : BorrowCertificate) (folded : Array ExprId) :
    BorrowCertificate :=
  { certificate with loans := certificate.loans.filter (!folded.contains ·.expression) }

end VerexIR.Validation
