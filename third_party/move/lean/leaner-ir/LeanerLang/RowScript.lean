-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Denotation
import LeanerLang.SpecTypes
import LeanerIR.Proofs.Native
import LeanerIR.Proofs.Certify
import LeanerIR.Proofs.NativeCall
import LeanerIR.Proofs.NativeStore

/-!
# Generated frame-free proof scripts

The resolution a frame-free proof performs is completely determined by the
body's combinator tree, and the generator holds that tree.  So the script
is emitted, not searched for: for each operation the tree contains, the
exact closed evaluation, the exact split, and the exact injection chain —
precisely the script `LeanerLang/Tests/NativeRow.lean` was written by
hand, produced mechanically.

The governing rule is that automation must always be fast.  A body this
generator does not model is not an occasion for a cleverer tactic; it is
either the frame route for now, or a proof someone — or some AI — writes
and checks in.  Eligibility is therefore decided here, at generation time,
so an ineligible function pays nothing.
-/

/-- Run an eligible body's generated row script alone, without the frame
fallback, so a generation gap surfaces as the script's own error instead of
being masked by the fallback's.  The generated script is also logged. -/
register_option leaner.rowDebug : Bool := {
  defValue := false
  descr := "surface generated row-script failures instead of falling back"
}

/-- The heartbeat budget of one generated verification theorem, in the
units of `maxHeartbeats`.  A checked-in check suite lowers it so a proof
that collapses into search fails instead of running long. -/
register_option leaner.verifyHeartbeats : Nat := {
  defValue := 400000000
  descr := "heartbeat budget of a generated verification theorem (thousands)"
}

namespace LeanerLang.RowScript

open Lean Elab Command
open LeanerIR
open LeanerIR.Proofs.Denotation

/-- An operand of a comparison, as the guard's proposition names it: a
parameter, or a literal. -/
inductive Operand where
  | local (index : Nat)
  | literal (value : Int)
  deriving Repr, BEq

/-- A guard: an integer comparison one of whose arms throws a literal code
while the other is empty.  `if c then abort(code)` throws on its then-arm;
`assert!(c, code)` throws on its else-arm. -/
structure Guard where
  /-- The comparison's closed evaluation row. -/
  row : Name
  relation : String
  left : Operand
  right : Operand
  /-- Whether the throwing arm is the then-arm. -/
  throwsWhenTrue : Bool
  deriving Repr, BEq

/-- What a value-producing branch splits on: a `Bool` parameter read into
the condition, or an integer comparison. -/
inductive BranchCondition where
  | boolLocal (index : Nat)
  | comparison (relation : String) (left right : Operand)
  deriving Repr, BEq

/-- One resolution event, in evaluation order. -/
inductive Event where
  /-- A local read: the drive introduces a value and its read equation. -/
  | read
  /-- A dereference through a mutable parameter. -/
  | dereference
  /-- Checked integer arithmetic at the given width and signedness;
  `subtract` distinguishes the two operations the templates model. -/
  | checked (width : Nat) (signed : Bool) (subtract : Bool)
  /-- Modular unsigned integer arithmetic at the given width (the Rust
  profile's `+` and `-`): a closed row, never a throw. -/
  | modular (width : Nat) (subtract : Bool)
  /-- A write through a mutable scalar parameter, by the local it sits
  in. -/
  | mutate (slot : Nat)
  /-- A resolved nominal constructor over its operand values. -/
  | construct
  /-- A `let` binding its initializer's value into a local. -/
  | bind (slot : Nat) (fuel : Nat)
  /-- A copy of a borrowed value. -/
  | copy
  /-- A field selection through a twin's erasure, by its generated row. -/
  | select (row : Name)
  /-- A field selection from a value the path spelled as a constructor:
  the selector computes. -/
  | selectConstructed
  /-- An integer comparison, by its closed row: the value is the decided
  relation. -/
  | compare (row : Name)
  /-- A guard on the comparison the events before it produced: the path
  splits on the relation, throwing on the arm the guard throws. -/
  | guardThrow (guard : Guard)
  /-- Checked division by a nonzero literal, at the given width. -/
  | checkedDivide (width : Nat) (signed : Bool)
  /-- A branch whose arms both produce the path's value: the path splits on
  the condition, each arm's events continuing to the same finish. -/
  | branch (condition : BranchCondition) (thenEvents elseEvents : List Event)
  /-- A loop at its site, with the events of one iteration: the path
  enters by the site's invariant, and each iteration re-establishes it
  or leaves. -/
  | loop (site : Nat) (body : List Event)
  /-- A `break` out of the enclosing loop: the path continues after it. -/
  | break_
  /-- A `continue` of the enclosing loop: the iteration ends here. -/
  | continue_
  deriving Repr, BEq

/-- The parameter rows the scalar route's closed entry and exit cover. -/
inductive ScalarShape where
  /-- One mutable scalar parameter, one local. -/
  | singleBorrow
  /-- Plain integers only, one to three of them, with up to one more
  local. -/
  | plainIntegers (count : Nat)
  /-- A mutable scalar parameter, a plain integer, and up to one more
  integer local. -/
  | borrowIntegers
  /-- Two mutable scalar parameters, with independent prophecies. -/
  | twoBorrows
  deriving Repr, BEq

/-- What a body's proof needs, read off its combinator tree. -/
structure Plan where
  events : List Event
  shape : ScalarShape
  parameterCount : Nat
  localCount : Nat
  /-- The plain parameters' kinds, `true` for an integer and `false` for a
  `Bool`; empty for the borrow shapes. -/
  kinds : Array Bool := #[]
  deriving Repr

/-- The literal a chain of single-argument constructors wraps. -/
private partial def natLeaf? (e : Lean.Expr) : Option Nat :=
  e.nat? <|> e.rawNatLit? <|>
    (if e.getAppNumArgs == 1 then natLeaf? (e.getArg! 0) else none)

private def integerTypeOf? (ty : Lean.Expr) : Option (Nat × Bool) := do
  guard (ty.isAppOfArity ``Ty.integer 2)
  let width := ty.getArg! 0
  guard (width.isAppOfArity ``IntWidth.bits 1)
  let widthValue ← (width.getArg! 0).nat?
  let signed ← match ty.getArg! 1 with
    | .const ``Bool.true _ => some true
    | .const ``Bool.false _ => some false
    | _ => none
  pure (widthValue, signed)

/-- The closed evaluation row of an integer comparison. -/
private def comparisonRow? (head : Name) : Option Name :=
  if head == ``PrimitiveLocationOperation.less then
    some ``LeanerIR.Proofs.Denotation.less_evaluate
  else if head == ``PrimitiveLocationOperation.lessEqual then
    some ``LeanerIR.Proofs.Denotation.lessEqual_evaluate
  else if head == ``PrimitiveLocationOperation.greater then
    some ``LeanerIR.Proofs.Denotation.greater_evaluate
  else if head == ``PrimitiveLocationOperation.greaterEqual then
    some ``LeanerIR.Proofs.Denotation.greaterEqual_evaluate
  else if head == ``PrimitiveLocationOperation.equal then
    some ``LeanerIR.Proofs.Denotation.equal_evaluate
  else if head == ``PrimitiveLocationOperation.notEqual then
    some ``LeanerIR.Proofs.Denotation.notEqual_evaluate
  else none

/-- The relation a comparison decides, as the guard states it. -/
private def comparisonRelation? (head : Name) : Option String :=
  if head == ``PrimitiveLocationOperation.less then some "<"
  else if head == ``PrimitiveLocationOperation.lessEqual then some "≤"
  else if head == ``PrimitiveLocationOperation.greater then some ">"
  else if head == ``PrimitiveLocationOperation.greaterEqual then some "≥"
  else none

/-- An operand a guard can name: a local read, or an integer literal. -/
private def operand? (e : Lean.Expr) : Option Operand := do
  if e.isAppOfArity ``localVar 1 then
    return .local (← natLeaf? (e.getArg! 0))
  else if e.isAppOfArity ``value 1 then
    let literal := e.getArg! 0
    guard (literal.isAppOfArity ``RuntimeValue.integer 1)
    return .literal (← (literal.getArg! 0).int?)
  else none

/-- The one operand of a row `values [head]`. -/
private def singleOperandOf? (e : Lean.Expr) : Option Lean.Expr := do
  guard (e.isAppOfArity ``valuesCons 2)
  guard ((e.getArg! 1).isConstOf ``valuesNil)
  pure (e.getArg! 0)

/-- The two operands of a row `values [left, right]`. -/
private def twoOperands? (e : Lean.Expr) : Option (Operand × Operand) := do
  guard (e.isAppOfArity ``valuesCons 2)
  let rest := e.getArg! 1
  guard (rest.isAppOfArity ``valuesCons 2)
  guard ((rest.getArg! 1).isConstOf ``valuesNil)
  pure (← operand? (e.getArg! 0), ← operand? (rest.getArg! 0))

/-- Recognize a guard: its shape, its comparison, and the thrown code's
operand — a `value` in a scalar body, a loan-ending marker over one inside
a bracket. -/
private def guardBranch? (e : Lean.Expr) : Option (Guard × Lean.Expr × Lean.Expr) := do
  guard (e.isAppOfArity ``nativeBranch 3)
  let condition := e.getArg! 0
  guard (condition.isAppOfArity ``nativePrimitiveOperation 2)
  let operationHead ← (condition.getArg! 0).getAppFn.constName?
  let row ← comparisonRow? operationHead
  let relation ← comparisonRelation? operationHead
  let (left, right) ← twoOperands? (condition.getArg! 1)
  let thenArm := e.getArg! 1
  let elseArm := e.getArg! 2
  let (throwsWhenTrue, thrown) ←
    if elseArm.isAppOf ``Option.none then some (true, thenArm)
    else if thenArm.isAppOfArity ``value 1 &&
        (thenArm.getArg! 0).isConstOf ``RuntimeValue.unit &&
        elseArm.isAppOfArity ``Option.some 2 then
      some (false, elseArm.getArg! 1)
    else none
  guard (thrown.isAppOfArity ``nativeThrow 2)
  let code ← singleOperandOf? (thrown.getArg! 1)
  pure ({ row, relation, left, right, throwsWhenTrue }, condition, code)

/-- Linearize a body into events, or refuse.  The traversal mirrors the
drive exactly: operand rows left to right, an operation after its
operands, statements in order. -/
private partial def linearize (e : Lean.Expr) : Option (List Event) := do
  let head ← e.getAppFn.constName?
  if head == ``blockUnit then
    linearize (e.getArg! 0)
  else if head == ``blockResult then
    return (← linearize (e.getArg! 0)) ++ (← linearize (e.getArg! 1))
  else if head == ``statementsNil || head == ``valuesNil then
    return []
  else if head == ``statementsCons || head == ``valuesCons then
    return (← linearize (e.getArg! 0)) ++ (← linearize (e.getArg! 1))
  else if head == ``value then
    return []
  else if head == ``localVar then
    return [.read]
  else if head == ``nativeAssignLocal then
    /- The assignment step is the drive's: its bounds side condition is
    decided at the literal row. -/
    linearize (e.getArg! 1)
  else if head == ``nativeSpec then
    return []
  else if head == ``nativeLoop then
    let site ← natLeaf? ((e.getArg! 0).getArg! 0)
    return [.loop site (← linearize (e.getArg! 1))]
  else if head == ``nativeBreak then
    /- A unit break of the enclosing loop; a value or an outer level is
    outside the subset. -/
    guard (natLeaf? (e.getArg! 0) == some 0)
    guard ((e.getArg! 1).isAppOf ``Option.none)
    return [.break_]
  else if head == ``nativeContinue then
    guard (natLeaf? (e.getArg! 0) == some 0)
    return [.continue_]
  else if head == ``letNativeValue then
    let binder := e.getArg! 0
    guard (binder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
    let fuel ← binder.getArg! 0 |>.nat?
    guard (fuel > 0)
    let pattern := binder.getArg! 1
    guard (pattern.isAppOfArity ``LeanerIR.SemanticOperations.NativePattern.variable 1)
    let local_ ← natLeaf? (pattern.getArg! 0)
    return (← linearize (e.getArg! 1)) ++ [.bind local_ fuel] ++ (← linearize (e.getArg! 2))
  else if head == ``nativePrimitiveOperation then
    let operation := e.getArg! 0
    let operands ← linearize (e.getArg! 1)
    let operationHead ← operation.getAppFn.constName?
    if operationHead == ``PrimitiveLocationOperation.checkedAdd ||
        operationHead == ``PrimitiveLocationOperation.checkedSubtract then
      let (width, signed) ← integerTypeOf? (operation.getArg! 1)
      return operands ++ [.checked width signed
        (operationHead == ``PrimitiveLocationOperation.checkedSubtract)]
    else if operationHead == ``PrimitiveLocationOperation.add ||
        operationHead == ``PrimitiveLocationOperation.subtract then
      /- The modular row is stated for unsigned widths; a signed wrap has
      no row yet. -/
      let (width, signed) ← integerTypeOf? (operation.getArg! 0)
      guard (!signed && width > 0)
      return operands ++ [.modular width
        (operationHead == ``PrimitiveLocationOperation.subtract)]
    else if let some row := comparisonRow? operationHead then
      return operands ++ [.compare row]
    else if operationHead == ``PrimitiveLocationOperation.copyValue then
      /- A read through a shared reference parameter: the parameter is its
      value at the boundary, and the read copies it. -/
      return operands ++ [.copy]
    else if operationHead == ``PrimitiveLocationOperation.checkedDivide then
      /- The divisor must be a nonzero literal: the row's side condition
      is decided, and a variable divisor would need an abort case. -/
      let (width, signed) ← integerTypeOf? (operation.getArg! 1)
      let (_, divisor) ← twoOperands? (e.getArg! 1)
      match divisor with
      | .literal value => if value == 0 then none else
          return operands ++ [.checkedDivide width signed]
      | .local _ => none
    else
      none
  else if head == ``nativeBranch then
    match guardBranch? e with
    | some (guardShape, condition, code) =>
        /- A guard on a literal code: the comparison's events, then the split. -/
        guard (code.isAppOfArity ``value 1)
        return (← linearize condition) ++ [.guardThrow guardShape]
    | none =>
        /- A branch producing the path's value on both arms, or a
        statement branch with no else arm. -/
        let condition := e.getArg! 0
        let elseArm := e.getArg! 2
        let thenEvents ← linearize (e.getArg! 1)
        let elseEvents ← if elseArm.isAppOfArity ``Option.some 2 then
            linearize (elseArm.getArg! 1)
          else if elseArm.isAppOf ``Option.none then pure []
          else none
        if condition.isAppOfArity ``localVar 1 then
          let index ← natLeaf? (condition.getArg! 0)
          return [.read, .branch (.boolLocal index) thenEvents elseEvents]
        else
          guard (condition.isAppOfArity ``nativePrimitiveOperation 2)
          let operationHead ← (condition.getArg! 0).getAppFn.constName?
          let relation ← comparisonRelation? operationHead
          let (left, right) ← twoOperands? (condition.getArg! 1)
          return (← linearize condition) ++
            [.branch (.comparison relation left right) thenEvents elseEvents]
  else if head == ``nativeOperation then
    let evaluate := e.getArg! 0
    let operands ← linearize (e.getArg! 1)
    if evaluate.isAppOfArity ``NominalConstructor.evaluate? 1 then
      return operands ++ [.construct]
    else if evaluate.isAppOfArity ``NominalFieldLocation.evaluateSelect? 1 then
      return operands ++ [.selectConstructed]
    else
      none
  else if head == ``nativeReferenceOperation then
    let operation := e.getArg! 0
    let operands ← linearize (e.getArg! 1)
    if operation.isConstOf ``ReferenceLocationOperation.dereference then
      return operands ++ [.dereference]
    else if operation.isConstOf ``ReferenceLocationOperation.mutate then
      /- The slot written through: the reference operand is a local read. -/
      let operandList := e.getArg! 1
      guard (operandList.isAppOfArity ``valuesCons 2)
      let reference := operandList.getArg! 0
      guard (reference.isAppOfArity ``localVar 1)
      let slot ← natLeaf? (reference.getArg! 0)
      return operands ++ [.mutate slot]
    else
      none
  else
    none

/-- Extract a plan from a generated body, or refuse.

The parameter shape is read off the tree, not the signature: a reference
event can only arise from a borrow parameter in this subset, so the events
determine which entry and export rows apply.  Eligibility is exactly the
proved rows — a single borrow, a borrow in front of an integer, or plain
integers; anything else (including a borrow parameter the body never
dereferences) has no route. -/
def plan? (body : Lean.Expr) (parameterCount localCount : Nat)
    (tailKinds : Option (Array Bool)) (plainKinds : Option (Array Bool))
    (secondBorrow : Bool := false) : Option Plan := do
  /- The stored body is `fun executable => tree`; a tree that mentions the
  executable (a call, a global read) is outside this subset. -/
  let tree := body.bindingBody!
  guard !tree.hasLooseBVars
  let events ← linearize tree
  /- A reference event anywhere on the path, branches included. -/
  let rec touchesReference : List Event → Bool
    | [] => false
    | .dereference :: _ | .mutate _ :: _ => true
    | .branch _ thenEvents elseEvents :: rest =>
        touchesReference thenEvents || touchesReference elseEvents || touchesReference rest
    | .loop _ body :: rest => touchesReference body || touchesReference rest
    | _ :: rest => touchesReference rest
  let borrowed := touchesReference events
  /- The rows are read off the signature's kinds, not the tree: a second
  mutable parameter also reads and writes through a local. -/
  let shape ← if borrowed then
    if parameterCount == 1 && localCount == 1 then some .singleBorrow
    else if parameterCount == 2 && localCount == 2 && secondBorrow then
      some .twoBorrows
    else if parameterCount == 2 && (localCount == 2 || localCount == 3) &&
        tailKinds.isSome then
      some .borrowIntegers
    else none
  else
    if plainKinds.isSome && 1 ≤ parameterCount && parameterCount ≤ 3 &&
        (localCount == parameterCount || localCount == parameterCount + 1) then
      some (.plainIntegers parameterCount)
    else none
  let kinds := match shape with
    | .plainIntegers _ => plainKinds.getD #[]
    | .borrowIntegers => #[true] ++ tailKinds.getD #[]
    | .singleBorrow | .twoBorrows => #[]
  pure { events, shape, parameterCount, localCount, kinds }

/-! ## Emission -/

/-- A raw (hygiene-free) identifier.  Every hypothesis name the script
introduces is spliced through this: the emitters build the script from many
separate quotations, and a literal name would carry a different macro scope
in each of them, so an `intro` in one quotation could never be referenced by
an `injection` in the next. -/
private def n (s : String) : Ident :=
  mkIdent (Name.mkSimple s)

private def named (base : String) (index : Nat) : Ident :=
  mkIdent (Name.mkSimple s!"{base}{index}")

/-- The bounds equation for one checked operation's type. -/
private def boundsHave (index width : Nat) (signed : Bool) :
    CommandElabM (TSyntax `tactic) := do
  let widthLit := Syntax.mkNatLit width
  let bounds ← if signed then
    `(term| some (-2 ^ ($widthLit - 1), 2 ^ ($widthLit - 1) - 1))
  else
    `(term| some ((0 : Int), 2 ^ $widthLit - 1))
  let signedLit := if signed then mkIdent ``Bool.true else mkIdent ``Bool.false
  `(tactic| have $(named "rowBounds" index):ident :
      (LeanerIR.Ty.integer (LeanerIR.IntWidth.bits $widthLit)
        $signedLit:ident).integerBounds? = $bounds := rfl)

/-- The injection-and-substitute chain for a value outcome. -/
private def invertValue (equation : Ident) : CommandElabM (Array (TSyntax `tactic)) := do
  pure #[
    ← `(tactic| injection $equation:ident with $(n "rowInner"):ident),
    ← `(tactic| injection $(n "rowInner"):ident with $(n "rowFrameEq"):ident $(n "rowStateEq"):ident $(n "rowValueEq"):ident),
    ← `(tactic| injection $(n "rowFrameEq"):ident with $(n "rowRowEq"):ident $(n "rowActiveEq"):ident $(n "rowLocationsEq"):ident),
    ← `(tactic| subst $(n "rowRowEq"):ident $(n "rowStateEq"):ident $(n "rowValueEq"):ident)]

/-- The injection-and-substitute chain for a throw outcome. -/
private def invertThrow (equation : Ident) : CommandElabM (Array (TSyntax `tactic)) := do
  pure #[
    ← `(tactic| injection $equation:ident with $(n "rowInner"):ident),
    ← `(tactic| injection $(n "rowInner"):ident with $(n "rowFrameEq"):ident $(n "rowStateEq"):ident $(n "rowKindEq"):ident $(n "rowThrownEq"):ident),
    ← `(tactic| injection $(n "rowFrameEq"):ident with $(n "rowRowEq"):ident $(n "rowActiveEq"):ident $(n "rowLocationsEq"):ident),
    ← `(tactic| subst $(n "rowRowEq"):ident $(n "rowStateEq"):ident $(n "rowKindEq"):ident $(n "rowThrownEq"):ident)]

/-- The contradiction close for an impossible outcome. -/
private def invertImpossible (equation : Ident) : CommandElabM (Array (TSyntax `tactic)) := do
  pure #[
    ← `(tactic| injection $equation:ident with $(n "rowInner"):ident),
    ← `(tactic| injection $(n "rowInner"):ident)]

/-- Consume a batch of local-read equations the drive introduced. -/
private def readBatch (count : Nat) : CommandElabM (Array (TSyntax `tactic)) := do
  if count == 0 then return #[]
  let mut binders : Array Ident := #[]
  let mut equations : Array Ident := #[]
  for index in [0:count] do
    binders := binders.push (named "rowRead" index)
    equations := equations.push (named "rowReadEq" index)
  let mut renameArgs : Array Ident := #[]
  for index in [0:count] do
    renameArgs := renameArgs.push binders[index]!
    renameArgs := renameArgs.push equations[index]!
  pure #[
    ← `(tactic| leaner_name_reads),
    ← `(tactic| simp only [LeanerIR.Proofs.Denotation.readLocal?_rowFrame,
        List.getElem?_toArray, List.getElem?_cons_zero,
        List.getElem?_cons_succ, Option.join_some,
        Option.some.injEq] at $[$equations:ident]*),
    ← `(tactic| subst $[$equations:ident]*)]

/-- The names the caller's own final closing consumes. -/
structure CallerNames where
  rawContract : Term
  resultsCodec : Term
  argumentsCodec : Term
  shape : Term
  /-- The declared parameter names, which the precondition's facts are
  named after (`<p>`, `<p>_loan`, `<p>_fits`, `<p>_loan_keyFree`, …). -/
  parameters : Array String := #[]
  /-- How many `requires` clauses the contract declares: their facts
  arrive as `requires_<i>`. -/
  requiresCount : Nat := 0
  /-- The generated invariant of the loop at a site. -/
  loopInvariant : Nat → Ident := fun _ => mkIdent .anonymous
  /-- Inside a recursive function's own theorem, its relation constant:
  the recursive call's contract fact is the induction hypothesis already
  in context, not a transported theorem. -/
  selfRelation : Option Name := none

/-- How a resolved path ends, and which write row a mutate resolves with:
the route (scalar, call, storage bracket) fixes all three. -/
structure Finish where
  /-- The tactics closing a value leaf, from the goal the last resolution
  step leaves. -/
  returned : Array (TSyntax `tactic)
  /-- The tactics closing a throw leaf. -/
  thrown : Array (TSyntax `tactic)
  /-- The closed evaluation of a write through a mutable borrow. -/
  mutateRow : Ident
  /-- The closed evaluation of a write through a second mutable
  parameter, when the row holds two. -/
  mutateRowSecond : Option Ident := none
  /-- The declared parameter names, which a guard's proposition uses for
  the locals it compares. -/
  parameters : Array String := #[]
  /-- The generated invariant of the loop at a site. -/
  loopInvariant : Nat → Ident := fun _ => mkIdent .anonymous
  /-- The row's length: the invariant quantifies a value per slot. -/
  localCount : Nat := 0
  /-- Inside a loop, the tactics continuing after a `break`. -/
  loopExit : Array (TSyntax `tactic) := #[]

/-- The finish for a returning path: compute the outcome, finalize through
the row export, close the contract by construction. -/
private def finishReturned (finish : Finish) :
    CommandElabM (Array (TSyntax `tactic)) :=
  pure finish.returned

/-- The tactics shared by every scalar-shaped value leaf up to the export
rewrite. -/
private def scalarReturnedPrefix : CommandElabM (Array (TSyntax `tactic)) := do
  pure #[
    ← `(tactic| leaner_row_drive),
    ← `(tactic| rename_i $(n "rowOutcome"):ident $(n "rowFinished"):ident),
    ← `(tactic| simp only [LeanerIR.SemanticOperations.finishControl?,
        LeanerIR.SemanticOperations.unpackFallthrough,
        Option.map_eq_map, Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident),
    ← `(tactic| subst $(n "rowFinished"):ident),
    ← `(tactic| simp only [LeanerIR.SemanticOperations.finalizeFunctionState])]

/-- The tactics shared by every throw leaf up to the closing. -/
private def scalarThrownPrefix : CommandElabM (Array (TSyntax `tactic)) := do
  pure #[
    ← `(tactic| leaner_row_drive),
    ← `(tactic| rename_i $(n "rowOutcome"):ident $(n "rowFinished"):ident),
    ← `(tactic| simp only [LeanerIR.SemanticOperations.finishControl?,
        Option.some.injEq] at $(n "rowFinished"):ident),
    ← `(tactic| subst $(n "rowFinished"):ident)]

/-- The scalar route's finish, selected by the parameter shape. -/
private def scalarFinish (caller : CallerNames) (plan : Plan) : CommandElabM Finish := do
  let export_ ← match plan.shape, plan.localCount with
    | .singleBorrow, _ =>
      `(tactic| rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_singleInteger
          _ _ _ _ _ (by assumption)])
    | .twoBorrows, _ =>
      `(tactic| rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_twoIntegers
          _ _ _ _ _ _ _ (by assumption) (by assumption)])
    | .plainIntegers _, _ =>
      /- Nothing in a plain row is a borrow, whatever its length. -/
      let slots ← match plan.localCount with
        | 1 => `(tactic| subst mem)
        | 2 => `(tactic| rcases mem with $(n "rfl"):ident | $(n "rfl"):ident)
        | 3 => `(tactic| rcases mem with $(n "rfl"):ident | $(n "rfl"):ident
            | $(n "rfl"):ident)
        | _ => `(tactic| rcases mem with $(n "rfl"):ident | $(n "rfl"):ident
            | $(n "rfl"):ident | $(n "rfl"):ident)
      `(tactic|
        (rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowFree _ _ _
           ?borrowFree]
         case borrowFree =>
           intro slot mem value eq
           simp only [List.mem_toArray, List.mem_cons, List.mem_nil_iff, or_false] at mem
           $slots:tactic <;> cases eq <;>
             simp [LeanerIR.SemanticOperations.outermostBorrows,
               LeanerIR.SemanticOperations.borrowEntry?,
               LeanerIR.SemanticOperations.collectPruned]))
    | .borrowIntegers, 2 =>
      if plan.kinds.getD 1 true then
        `(tactic| rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowInteger
            _ _ _ _ _ _ (by assumption)])
      else
        `(tactic| rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowBool
            _ _ _ _ _ _ (by assumption)])
    | .borrowIntegers, _ =>
      `(tactic| rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowTwoIntegers
          _ _ _ _ _ _ _ (by assumption)])
  pure {
    returned := (← scalarReturnedPrefix) ++ #[export_,
      ← `(tactic| leaner_certified_close!)]
    thrown := (← scalarThrownPrefix) ++ #[← `(tactic| leaner_certified_close!)]
    mutateRow := mkIdent <| match plan.shape with
      | .singleBorrow => ``LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_singleBorrow
      | .twoBorrows => ``LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_twoBorrows_left
      | _ => ``LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_borrowFirst
    mutateRowSecond := match plan.shape with
      | .twoBorrows =>
          some (mkIdent ``LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_twoBorrows_right)
      | _ => none
    /- The borrow route binds its own vocabulary for the leading two
    parameters, so events naming a parameter must use those names. -/
    loopInvariant := caller.loopInvariant
    localCount := plan.localCount
    parameters := match plan.shape with
      | .borrowIntegers =>
          #["valVar", "amountVar"] ++ caller.parameters.extract 2 caller.parameters.size
      | _ => caller.parameters }

/-- The finish for an aborting path. -/
private def finishThrown (finish : Finish) :
    CommandElabM (Array (TSyntax `tactic)) :=
  pure finish.thrown

private def seq (tactics : Array (TSyntax `tactic)) :
    CommandElabM (TSyntax `tactic) := do
  `(tactic| ($[$tactics:tactic]*))

/-- The guard's proposition, its operands named by the row's binders. -/
private def guardProposition (guardShape : Guard) (nameOf : Nat → String) :
    CommandElabM Term := do
  let operandTerm : Operand → CommandElabM Term
    | .local index => `(term| $(mkIdent (Name.mkSimple (nameOf index))):ident)
    | .literal value =>
        if value < 0 then `(term| -$(Syntax.mkNumLit (toString (-value))))
        else `(term| $(Syntax.mkNumLit (toString value)))
  let leftTerm ← operandTerm guardShape.left
  let rightTerm ← operandTerm guardShape.right
  match guardShape.relation with
  | "<" => `(term| $leftTerm < $rightTerm)
  | "≤" => `(term| $leftTerm ≤ $rightTerm)
  | ">" => `(term| $leftTerm > $rightTerm)
  | _ => `(term| $leftTerm ≥ $rightTerm)

/-- Split on a guard: the decided relation reduces the branch to the arm it
selects, the throwing arm on the side the guard throws. -/
private def guardSplit (guardShape : Guard) (nameOf : Nat → String)
    (throwArm continueArm : TSyntax `tactic) :
    CommandElabM (Array (TSyntax `tactic)) := do
  let proposition ← guardProposition guardShape nameOf
  let (onTrue, onFalse) :=
    if guardShape.throwsWhenTrue then (throwArm, continueArm)
    else (continueArm, throwArm)
  pure #[
    ← `(tactic| by_cases $(n "rowGuard"):ident : $proposition),
    ← `(tactic| · (simp only [$(n "rowGuard"):ident, decide_true]
                   $onTrue:tactic)),
    ← `(tactic| · (simp only [$(n "rowGuard"):ident, decide_false]
                   $onFalse:tactic))]

/-- Bind a `let`'s value at the literal row: the drive has introduced the
bound frame and its binding equation, and the body continues from the
row with the local set. -/
private def bindStep (slot fuel : Nat) : CommandElabM (Array (TSyntax `tactic)) := do
  let fuelLit := Syntax.mkNatLit (fuel - 1)
  let localLit := Syntax.mkNatLit slot
  pure #[
    ← `(tactic| rename_i $(n "boundFrame"):ident $(n "boundEq"):ident),
    ← `(tactic| rw [LeanerIR.Proofs.Denotation.bindVariable_rowFrame $fuelLit ⟨$localLit⟩
        _ _ _ (by simp)] at $(n "boundEq"):ident),
    ← `(tactic| cases Option.some.inj $(n "boundEq"):ident),
    ← `(tactic| refine ⟨_, rfl, ?_⟩),
    ← `(tactic| simp only [Array.set!_eq_setIfInBounds, List.setIfInBounds_toArray,
        List.set_cons_succ, List.set_cons_zero])]

/-- The name an iteration knows a slot's value by: the parameter's own
name, or `loop_local_<i>` for a local. -/
private def slotName (parameters : Array String) (index : Nat) : String :=
  parameters.getD index s!"loop_local_{index}"

/-- One binder per slot of the row, named as the iteration knows them. -/
private def witnessPatterns (parameters : Array String) (localCount : Nat) :
    CommandElabM (Array (TSyntax `rcasesPat)) := do
  (List.range localCount).toArray.mapM fun index =>
    `(rcasesPat| $(mkIdent (Name.mkSimple (slotName parameters index))):ident)

/-- Close a loop invariant at the row the path reached: the witnesses are
goals the frame shape assigns by reduction, the equations hold by
reduction, and the ranges and the authored clauses are arithmetic. -/
private def invariantCloser (_localCount : Nat) :
    CommandElabM (Array (TSyntax `tactic)) := do
  pure #[
    ← `(tactic| repeat' (apply Exists.intro)),
    ← `(tactic| repeat' (refine And.intro ?_ ?_)),
    ← `(tactic| all_goals first
        | exact rfl
        | omega
        | (simp only [LeanerIR.Proofs.Denotation.readLocal?_rowFrame,
             List.getElem?_toArray, List.getElem?_cons_zero, List.getElem?_cons_succ,
             Option.join_some, Option.some.injEq, LeanerIR.RuntimeValue.integer.injEq]
           omega))]

/-- Emit the resolution for the events after the first drive call. -/
private partial def emitEvents (finish : Finish) (events : List Event)
    (boundsIndex : Nat) : CommandElabM (Array (TSyntax `tactic)) := do
  match events with
  | [] => finishReturned finish
  | .read :: _ =>
      let reads := events.takeWhile (· == .read) |>.length
      let rest := events.drop reads
      pure ((← readBatch reads) ++ (← emitEvents finish rest boundsIndex))
  | .dereference :: rest =>
      let valueBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowValueOut"):ident $(n "rowEq"):ident),
        ← `(tactic| simp only [
            LeanerIR.Proofs.Denotation.ReferenceLocationOperation.evaluate?,
            LeanerIR.Proofs.Denotation.liftPlaceEvaluator,
            LeanerIR.SemanticOperations.dereferenceBorrow?] at $(n "rowEq"):ident)]
        ++ (← invertValue (mkIdent `rowEq))
        ++ #[← `(tactic| leaner_row_drive)]
        ++ (← emitEvents finish rest boundsIndex)
      let throwBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident),
        ← `(tactic| simp only [
            LeanerIR.Proofs.Denotation.ReferenceLocationOperation.evaluate?,
            LeanerIR.Proofs.Denotation.liftPlaceEvaluator,
            LeanerIR.SemanticOperations.dereferenceBorrow?] at $(n "rowEq"):ident)]
        ++ (← invertImpossible (mkIdent `rowEq))
      pure #[← `(tactic| constructor), ← `(tactic| case' left => $valueBranch:tactic),
        ← `(tactic| case' right => $throwBranch:tactic)]
  | .checked width signed _ :: rest =>
      let bounds := named "rowBounds" boundsIndex
      let computeEq ← `(tactic| simp only [
          LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.evaluate?,
          LeanerIR.SemanticOperations.checkedBinaryInteger,
          LeanerIR.SemanticOperations.checkedInteger, $bounds:ident] at $(n "rowEq"):ident)
      let inRangeTail ← seq <| #[
          ← `(tactic| simp only [Bool.and_eq_true, decide_eq_true_eq] at $(n "rowInRange"):ident),
          ← `(tactic| obtain ⟨rowLower, rowUpper⟩ := $(n "rowInRange"):ident)]
        ++ (← invertValue (mkIdent `rowEq))
        ++ #[← `(tactic| leaner_row_drive)]
        ++ (← emitEvents finish rest (boundsIndex + 1))
      let valueBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowValueOut"):ident $(n "rowEq"):ident),
        ← boundsHave boundsIndex width signed,
        computeEq,
        ← `(tactic| split at $(n "rowEq"):ident),
        ← `(tactic| next $(n "rowInRange"):ident => $inRangeTail:tactic),
        ← `(tactic| next $(n "rowOutOfRange"):ident =>
            $(← seq (← invertImpossible (mkIdent `rowEq))):tactic)]
      let outOfRangeTail ← seq <| #[
          ← `(tactic| simp only [Bool.and_eq_true, decide_eq_true_eq] at $(n "rowOutOfRange"):ident)]
        ++ (← invertThrow (mkIdent `rowEq))
        ++ (← finishThrown finish)
      let throwBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident),
        ← boundsHave boundsIndex width signed,
        computeEq,
        ← `(tactic| split at $(n "rowEq"):ident),
        ← `(tactic| next $(n "rowInRange"):ident =>
            $(← seq (← invertImpossible (mkIdent `rowEq))):tactic),
        ← `(tactic| next $(n "rowOutOfRange"):ident => $outOfRangeTail:tactic)]
      pure #[← `(tactic| constructor), ← `(tactic| case' left => $valueBranch:tactic),
        ← `(tactic| case' right => $throwBranch:tactic)]
  | .bind slot fuel :: rest =>
      /- The drive introduced the binding step; bind at the literal row
      and continue the body from it. -/
      pure <| (← bindStep slot fuel) ++ #[← `(tactic| leaner_row_drive)]
        ++ (← emitEvents finish rest boundsIndex)
  | .construct :: rest =>
      let computeEq ← `(tactic| simp only [
          LeanerIR.Proofs.Denotation.NominalConstructor.evaluate?,
          List.size_toArray, List.length_cons, List.length_nil, Nat.zero_add,
          bne_self_eq_false, Bool.false_eq_true, ↓reduceIte] at $(n "rowEq"):ident)
      let valueBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowValueOut"):ident $(n "rowEq"):ident),
        computeEq]
        ++ (← invertValue (mkIdent `rowEq))
        ++ #[← `(tactic| leaner_row_drive)]
        ++ (← emitEvents finish rest boundsIndex)
      let throwBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident),
        computeEq]
        ++ (← invertImpossible (mkIdent `rowEq))
      pure #[← `(tactic| constructor), ← `(tactic| case' left => $valueBranch:tactic),
        ← `(tactic| case' right => $throwBranch:tactic)]
  | .copy :: rest =>
      closedRow (mkIdent ``LeanerIR.Proofs.Denotation.copyValue_evaluate) rest
  | .modular _ subtract :: rest =>
      let row := mkIdent <| if subtract
        then ``LeanerIR.Proofs.Denotation.subtract_evaluate_unsigned
        else ``LeanerIR.Proofs.Denotation.add_evaluate_unsigned
      let rewrite ← `(tactic| rw [$row:ident _ _ _ _ _ (by decide)] at $(n "rowEq"):ident)
      let valueBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowValueOut"):ident $(n "rowEq"):ident),
        rewrite]
        ++ (← invertValue (mkIdent `rowEq))
        ++ #[← `(tactic| leaner_row_drive)]
        ++ (← emitEvents finish rest boundsIndex)
      let throwBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident),
        rewrite]
        ++ (← invertImpossible (mkIdent `rowEq))
      pure #[← `(tactic| constructor), ← `(tactic| case' left => $valueBranch:tactic),
        ← `(tactic| case' right => $throwBranch:tactic)]
  | .compare row :: rest =>
      closedRow (mkIdent row) rest
  | .guardThrow guardShape :: rest =>
      let thrown ← seq finish.thrown
      let continued ← seq <| #[← `(tactic| leaner_row_drive)]
        ++ (← emitEvents finish rest boundsIndex)
      guardSplit guardShape
        (fun index => finish.parameters.getD index s!"argument{index}") thrown continued
  | .branch condition thenEvents elseEvents :: rest =>
      let nameOf (index : Nat) := finish.parameters.getD index s!"argument{index}"
      let thenArm ← seq <| #[← `(tactic| leaner_row_drive)]
        ++ (← emitEvents finish (thenEvents ++ rest) boundsIndex)
      let elseArm ← seq <| #[← `(tactic| leaner_row_drive)]
        ++ (← emitEvents finish (elseEvents ++ rest) boundsIndex)
      match condition with
      | .boolLocal index =>
          let flag := mkIdent (Name.mkSimple (nameOf index))
          pure #[
            ← `(tactic| by_cases $(n "rowGuard"):ident : $flag:ident = true),
            ← `(tactic| · (simp only [$(n "rowGuard"):ident]
                           $thenArm:tactic)),
            ← `(tactic| · (simp only [Bool.not_eq_true] at $(n "rowGuard"):ident
                           simp only [$(n "rowGuard"):ident]
                           $elseArm:tactic))]
      | .comparison relation left right =>
          let guardShape : Guard :=
            { row := .anonymous, relation, left, right, throwsWhenTrue := true }
          guardSplit guardShape nameOf thenArm elseArm
  | .checkedDivide width signed :: rest =>
      let bounds := named "rowBounds" boundsIndex
      let computeEq ← `(tactic| (
          rw [LeanerIR.Proofs.Denotation.checkedDivide_evaluate _ _ _ _ _ _ (by decide)]
            at $(n "rowEq"):ident
          simp only [$bounds:ident] at $(n "rowEq"):ident))
      let inRangeTail ← seq <| #[
          ← `(tactic| simp only [Bool.and_eq_true, decide_eq_true_eq] at $(n "rowInRange"):ident),
          ← `(tactic| obtain ⟨rowLower, rowUpper⟩ := $(n "rowInRange"):ident)]
        ++ (← invertValue (mkIdent `rowEq))
        ++ #[← `(tactic| leaner_row_drive)]
        ++ (← emitEvents finish rest (boundsIndex + 1))
      let valueBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowValueOut"):ident $(n "rowEq"):ident),
        ← boundsHave boundsIndex width signed,
        computeEq,
        ← `(tactic| split at $(n "rowEq"):ident),
        ← `(tactic| next $(n "rowInRange"):ident => $inRangeTail:tactic),
        ← `(tactic| next $(n "rowOutOfRange"):ident =>
            $(← seq (← invertImpossible (mkIdent `rowEq))):tactic)]
      let outOfRangeTail ← seq <| #[
          ← `(tactic| simp only [Bool.and_eq_true, decide_eq_true_eq] at $(n "rowOutOfRange"):ident)]
        ++ (← invertThrow (mkIdent `rowEq))
        ++ (← finishThrown finish)
      let throwBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident),
        ← boundsHave boundsIndex width signed,
        computeEq,
        ← `(tactic| split at $(n "rowEq"):ident),
        ← `(tactic| next $(n "rowInRange"):ident =>
            $(← seq (← invertImpossible (mkIdent `rowEq))):tactic),
        ← `(tactic| next $(n "rowOutOfRange"):ident => $outOfRangeTail:tactic)]
      pure #[← `(tactic| constructor), ← `(tactic| case' left => $valueBranch:tactic),
        ← `(tactic| case' right => $throwBranch:tactic)]
  | .select row :: rest =>
      closedRow (mkIdent row) rest
  | .loop site body :: rest =>
      /- The loop by its invariant: the entry establishes it at the row
      the path reached; an iteration assumes it at an arbitrary row,
      recovers that row's literal from its frame shape and the immutable
      locals from the entry frame, and runs the body's events, which end
      by re-establishing the invariant or leave through the exit. -/
      let invariant := finish.loopInvariant site
      let exit ← emitEvents finish rest boundsIndex
      let closeInvariant ← invariantCloser finish.localCount
      let witnesses ← witnessPatterns finish.parameters finish.localCount
      let iteration : Finish := { finish with
        returned := closeInvariant, loopExit := #[← `(tactic| leaner_row_drive)] ++ exit
        parameters := (List.range finish.localCount).toArray.map (slotName finish.parameters) }
      let iterate ← seq <| #[
        ← `(tactic| intro $(n "iterationRow"):ident $(n "iterationState"):ident
            $(n "invariantHolds"):ident),
        ← `(tactic| simp only [$invariant:ident, and_assoc] at $(n "invariantHolds"):ident),
        ← `(tactic| obtain ⟨$[$witnesses],*,
            $(n "iterationStateEq"):ident, $(n "iterationRowEq"):ident,
            $(n "invariantHolds"):ident⟩ := $(n "invariantHolds"):ident),
        ← `(tactic| subst $(n "iterationStateEq"):ident),
        ← `(tactic| simp only [LeanerIR.Proofs.Denotation.rowFrame] at $(n "iterationRowEq"):ident),
        ← `(tactic| subst $(n "iterationRowEq"):ident),
        ← `(tactic| simp only [LeanerIR.Proofs.Denotation.readLocal?_rowFrame,
            List.getElem?_toArray, List.getElem?_cons_zero, List.getElem?_cons_succ,
            Option.join_some, Option.some.injEq, LeanerIR.RuntimeValue.integer.injEq,
            true_and, and_true] at $(n "invariantHolds"):ident),
        ← `(tactic| repeat' (obtain ⟨$(n "immutableEq"):ident, $(n "invariantHolds"):ident⟩ :=
                                $(n "invariantHolds"):ident; subst $(n "immutableEq"):ident)),
        ← `(tactic| leaner_row_drive)]
        ++ (← emitEvents iteration body boundsIndex)
      let entry ← seq closeInvariant
      pure #[
        ← `(tactic| refine LeanerIR.Proofs.Denotation.wpRow_nativeLoop_of_invariant _ _
            $invariant:ident _ _ _ _ ?_ ?_ ?_),
        ← `(tactic| · leaner_row_stable),
        ← `(tactic| · $entry:tactic),
        ← `(tactic| · $iterate:tactic)]
  | .break_ :: _ =>
      pure finish.loopExit
  | .continue_ :: _ =>
      finishReturned finish
  | .selectConstructed :: rest =>
      let computeEq ← `(tactic| simp only [
          LeanerIR.Proofs.Denotation.NominalFieldLocation.evaluateSelect?,
          LeanerIR.Proofs.Denotation.liftConstructorEvaluator,
          LeanerIR.SemanticOperations.selectNominalFieldAt?,
          List.toList_toArray, bne_self_eq_false, Bool.false_or, Bool.false_eq_true,
          ↓reduceIte, List.getElem?_toArray, List.getElem?_cons_zero,
          List.getElem?_cons_succ, Option.bind_some] at $(n "rowEq"):ident)
      let valueBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowValueOut"):ident $(n "rowEq"):ident),
        computeEq]
        ++ (← invertValue (mkIdent `rowEq))
        ++ #[← `(tactic| leaner_row_drive)]
        ++ (← emitEvents finish rest boundsIndex)
      let throwBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident),
        computeEq]
        ++ (← invertImpossible (mkIdent `rowEq))
      pure #[← `(tactic| constructor), ← `(tactic| case' left => $valueBranch:tactic),
        ← `(tactic| case' right => $throwBranch:tactic)]
  | .mutate slot :: rest =>
      let mutateRow := if slot == 1 then finish.mutateRowSecond.getD finish.mutateRow
        else finish.mutateRow
      let valueBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowValueOut"):ident $(n "rowEq"):ident),
        ← `(tactic| rw [$mutateRow:ident] at $(n "rowEq"):ident),
        /- Two parameters' loans are separate by the requirement. -/
        ← `(tactic| try case separate => assumption)]
        ++ (← invertValue (mkIdent `rowEq))
        ++ #[← `(tactic| leaner_row_drive)]
        ++ (← emitEvents finish rest boundsIndex)
      let throwBranch ← seq <| #[
        ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident),
        ← `(tactic| rw [$mutateRow:ident] at $(n "rowEq"):ident),
        ← `(tactic| try case separate => assumption)]
        ++ (← invertImpossible (mkIdent `rowEq))
      pure #[← `(tactic| constructor), ← `(tactic| case' left => $valueBranch:tactic),
        ← `(tactic| case' right => $throwBranch:tactic)]
where
  /-- An operation with a closed evaluation row: the value branch resolves
  through the row and continues, the throw branch is impossible. -/
  closedRow (row : Ident) (rest : List Event) :
      CommandElabM (Array (TSyntax `tactic)) := do
    let valueBranch ← seq <| #[
      ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowValueOut"):ident $(n "rowEq"):ident),
      ← `(tactic| rw [$row:ident] at $(n "rowEq"):ident)]
      ++ (← invertValue (mkIdent `rowEq))
      ++ #[← `(tactic| leaner_row_drive)]
      ++ (← emitEvents finish rest boundsIndex)
    let throwBranch ← seq <| #[
      ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident),
      ← `(tactic| rw [$row:ident] at $(n "rowEq"):ident)]
      ++ (← invertImpossible (mkIdent `rowEq))
    pure #[← `(tactic| constructor), ← `(tactic| case' left => $valueBranch:tactic),
      ← `(tactic| case' right => $throwBranch:tactic)]

/-- Bind a route's vocabulary to the contract-derived hypothesis names:
`leaner_native_cases` names the precondition's witnesses and facts from
the contract, so a route never depends on their positions. -/
private def bindVocabulary (pairs : Array (String × String)) :
    CommandElabM (TSyntax `tactic) := do
  let mut tactics : Array (TSyntax `tactic) := #[]
  for (source, target) in pairs do
    unless source == target do
      tactics := tactics.push
        (← `(tactic| leaner_rename $(mkIdent (Name.mkSimple source)):ident =>
            $(mkIdent (Name.mkSimple target)):ident))
  `(tactic| ($[$tactics:tactic]*))

/-- The single-`&mut`-scalar caller's vocabulary. -/
private def borrowVocabulary (parameter : String) : Array (String × String) :=
  #[(s!"{parameter}_loan", "loanVar"), (parameter, "valVar"),
    (s!"{parameter}_fits", "fitsVar"), (s!"{parameter}_loan_keyFree", "keyFree"),
    (s!"{parameter}_loan_bound", "loanBound"), (s!"{parameter}_nonNeg", "valNonNeg"),
    (s!"{parameter}_max", "valMax")]

/-- The complete row-route tactic block, from the goal `wp_nativeFunction`
leaves.  Names are the caller's generated constants. -/
def script (caller : CallerNames) (plan : Plan) :
    CommandElabM (TSyntax `tactic) := do
  let argumentsCodec := caller.argumentsCodec
  let shape := caller.shape
  let slot := caller.parameters.getD 0 "slot"
  let amount := caller.parameters.getD 1 "amount"
  /- The entry row and registered loans, closed per parameter shape. -/
  let entry ← match plan.shape with
    | .singleBorrow => `(tactic| simp only [$argumentsCodec:term, $shape:term,
        LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.mutable,
        LeanerIR.Proofs.Denotation.initialLocals_one,
        LeanerIR.SemanticOperations.parameterLoanLocations_singleBorrow])
    | .twoBorrows => `(tactic| simp only [$argumentsCodec:term, $shape:term,
        LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.mutable,
        LeanerIR.Proofs.Denotation.initialLocals_two,
        LeanerIR.SemanticOperations.parameterLoanLocations_twoBorrows])
    | .plainIntegers count =>
        /- The row literal: each integer parameter under its own name, and
        `none` for a local the body binds later. -/
        let names := (List.range count).map fun index =>
          mkIdent (Name.mkSimple (caller.parameters.getD index s!"argument{index}"))
        let valueOf (index : Nat) (name : Ident) : CommandElabM Term :=
          if plan.kinds.getD index true then `(term| .integer $name:ident)
          else `(term| .bool $name:ident)
        let arguments ← names.zipIdx.mapM fun (name, index) => valueOf index name
        let present ← names.zipIdx.mapM fun (name, index) => do
          let value ← valueOf index name
          `(term| some $value)
        let absent ← (List.replicate (plan.localCount - count) ()).mapM fun _ =>
          `(term| none)
        let slotsTerm := present ++ absent
        let localsLit := Syntax.mkNatLit plan.localCount
        `(tactic|
          (simp only [$argumentsCodec:term, $shape:term,
             LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool,
             LeanerIR.Proofs.Codec.mutable]
           rw [show LeanerIR.SemanticOperations.initialLocals $localsLit
               #[$(arguments.toArray),*] = #[$(slotsTerm.toArray),*] by
               simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
             show LeanerIR.SemanticOperations.parameterLoanLocations
               #[$(arguments.toArray),*] = #[] by
               simp [LeanerIR.SemanticOperations.parameterLoanLocations]]))
    | .borrowIntegers =>
        let localsLit := Syntax.mkNatLit plan.localCount
        let integerTail := plan.kinds.getD 1 true
        let tail ← if integerTail then `(term| .integer $(n "amountVar"):ident)
          else `(term| .bool $(n "amountVar"):ident)
        let entryLoans ← if integerTail then
          `(term| LeanerIR.SemanticOperations.parameterLoanLocations_borrowInteger)
        else
          `(term| LeanerIR.SemanticOperations.parameterLoanLocations_borrowBool)
        let row ← if plan.localCount == 2 then
          `(term| #[some (.borrow $(n "loanVar"):ident (.integer $(n "valVar"):ident)),
            some $tail])
        else
          `(term| #[some (.borrow $(n "loanVar"):ident (.integer $(n "valVar"):ident)),
            some $tail, none])
        `(tactic|
          (simp only [$argumentsCodec:term, $shape:term,
             LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool,
             LeanerIR.Proofs.Codec.mutable]
           rw [show LeanerIR.SemanticOperations.initialLocals $localsLit
               #[.borrow $(n "loanVar"):ident (.integer $(n "valVar"):ident),
                 $tail] = $row by
               simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
             $entryLoans:term]))
  let renames ← match plan.shape with
    | .borrowIntegers =>
        if plan.kinds.getD 1 true then
          bindVocabulary (borrowVocabulary slot ++
            #[(amount, "amountVar"), (s!"{amount}_fits", "amountFits"),
              (s!"{amount}_nonNeg", "amountNonNeg"), (s!"{amount}_max", "amountMax"),
              (s!"{amount}_range", "amountFitsConj")])
        else bindVocabulary (borrowVocabulary slot ++ #[(amount, "amountVar")])
    | _ => `(tactic| skip)
  let reads := plan.events.takeWhile (· == .read) |>.length
  let rest := plan.events.drop reads
  let resolution ← seq <|
    (← readBatch reads) ++ (← emitEvents (← scalarFinish caller plan) rest 0)
  `(tactic|
    ($renames:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntry_rowFrame
     case arity => rfl
     case declared => simp [$argumentsCodec:term, $shape:term]
     case stable => leaner_row_stable
     $entry:tactic
     leaner_row_drive
     $resolution:tactic))

/-! ## Call bodies

A body whose statements are all `core.call f(&mut *slot)` — the
reborrow-call-endLoan pattern — is resolved modularly: the emitted script
consumes each callee's proved contract through
`wpStatementsRowThrow_consCallReborrow` instead of stepping the callee.
The supported shape is a single-`&mut`-scalar caller whose callees have
the same signature, which is the whole generated contract grammar the
templates below destructure. -/

/-- The relation constant of a callee term.  A callee is either a generated
relation at the executable unit, `<f>.denotationRelation executable`, or —
inside a recursive function's own body, planned with its recursive calls
resolved to its fixed point — that relation unfolded,
`nativeFunctionRelation executable <f>.denotationShape (<f>.denotationBody
executable)`; both name the same callee. -/
private def calleeRelation? (callee : Lean.Expr) : Option Name := do
  if callee.isApp && callee.appArg! == .bvar 0 then
    callee.appFn!.constName?
  else
    guard (callee.isAppOfArity ``nativeFunctionRelation 3)
    guard (callee.getArg! 0 == .bvar 0)
    let shape ← (callee.getArg! 1).constName?
    let body := callee.getArg! 2
    guard (body.isApp && body.appArg! == .bvar 0)
    let bodyConst ← body.appFn!.constName?
    let .str functionPath "denotationShape" := shape | none
    guard (bodyConst == Name.str functionPath "denotationBody")
    pure (Name.str functionPath "denotationRelation")

/-- The names a call site consumes, all derived from the callee's relation
constant. -/
structure CalleeNames where
  relation : Name
  verified : Name
  agreement : Name
  contract : Name
  typedContract : Name
  rawContract : Name
  argumentsCodec : Name
  resultsCodec : Name
  /-- The callee's lowered shape and body, in the dependency namespace:
  what an unspecified callee is run from, inline. -/
  shape : Name
  body : Name

/-- Derive the callee's public and dependency-namespace constant names
from its relation constant.  The relation lives at
`<module>._denotation_dependencies.namespaceN.functionM.<f>.denotationRelation`
and the public constants at `<module>.<f>.*`. -/
def calleeNames? (relation : Name) : Option CalleeNames := do
  let functionPath := relation.getPrefix
  let .str _ functionName := functionPath | none
  let modulePath :=
    functionPath.getPrefix.getPrefix.getPrefix.getPrefix
  let publicPath := Name.str modulePath functionName
  pure {
    relation
    verified := Name.str publicPath "verified"
    agreement := Name.str functionPath "denotation_agrees"
    contract := Name.str publicPath "contract"
    typedContract := Name.str publicPath "typedContract"
    rawContract := Name.str publicPath "rawContract"
    argumentsCodec := Name.str publicPath "argumentsCodec"
    resultsCodec := Name.str publicPath "resultsCodec"
    shape := Name.str functionPath "denotationShape"
    body := Name.str functionPath "denotationBody" }

/-- One call statement: the callee, and how many of the caller's mutable
parameters it takes by reborrow — the first alone, or both. -/
structure CallSite where
  callee : CalleeNames
  reborrows : Nat
  /-- The callee's `ensures` clauses: each is one `Obligation` conjunct of
  its summary, nested after the row and range facts. -/
  clauses : Nat := 1
  /-- Whether the call statement is an arm block's result expression rather
  than one of its statements. -/
  resultPosition : Bool := false

/-- A whole-parameter mutable reborrow of local `slot`. -/
private def reborrowOfLocal? (e : Lean.Expr) (slot : Nat) : Option Unit := do
  guard (e.isAppOfArity ``nativeDerefLocalBorrowOperation 2)
  guard ((e.getArg! 1).isConstOf ``valuesNil)
  let record := e.getArg! 0
  guard (record.isAppOfArity ``DerefLocalBorrowOperation.mk 5)
  guard ((record.getArg! 3).isConstOf ``BorrowKind.mutable)
  guard ((record.getArg! 1).isAppOf ``List.nil)
  let location := record.getArg! 0
  guard (location.isAppOfArity ``LocalLocation.mk 1)
  guard ((← natLeaf? ((location.getArg! 0).getArg! 0)) == slot)

/-- Recognize one `endLoan (call f (reborrow local0, …))` statement and
return the callee's relation constant with the reborrow count. -/
private def callStatement? (e : Lean.Expr) (slot : Nat := 0) :
    Option (Name × Nat × (Nat × Nat)) := do
  guard (e.isAppOfArity ``nativeReferenceOperation 2)
  let operation := e.getArg! 0
  guard (operation.isAppOfArity ``ReferenceLocationOperation.endLoan 1)
  let operands := e.getArg! 1
  guard (operands.isAppOfArity ``valuesCons 2)
  guard ((operands.getArg! 1).isConstOf ``valuesNil)
  let callExpr := operands.getArg! 0
  guard (callExpr.isAppOfArity ``nativeCall 4)
  guard ((callExpr.getArg! 1).isAppOf ``Option.none)
  let callee := callExpr.getArg! 2
  let relation ← calleeRelation? callee
  let handle := callExpr.getArg! 0
  guard (handle.isAppOfArity ``LeanerIR.FunctionHandle.mk 2)
  let namespaceIndex ← natLeaf? ((handle.getArg! 0).getArg! 0)
  let functionIndex ← natLeaf? ((handle.getArg! 1).getArg! 0)
  let inner := callExpr.getArg! 3
  guard (inner.isAppOfArity ``valuesCons 2)
  reborrowOfLocal? (inner.getArg! 0) slot
  let tail := inner.getArg! 1
  if tail.isConstOf ``valuesNil then
    pure (relation, 1, (namespaceIndex, functionIndex))
  else
    guard (tail.isAppOfArity ``valuesCons 2)
    guard ((tail.getArg! 1).isConstOf ``valuesNil)
    reborrowOfLocal? (tail.getArg! 0) (slot + 1)
    pure (relation, 2, (namespaceIndex, functionIndex))

private partial def callStatements? (e : Lean.Expr) (clauseCountOf : Nat × Nat → Nat) :
    Option (List CallSite) := do
  if e.isConstOf ``statementsNil then
    pure []
  else do
    guard (e.isAppOfArity ``statementsCons 2)
    let (relation, reborrows, handle) ← callStatement? (e.getArg! 0)
    let callee ← calleeNames? relation
    let rest ← callStatements? (e.getArg! 1) clauseCountOf
    pure ({ callee, reborrows, clauses := clauseCountOf handle } :: rest)

/-- The symbolic value of a scalar expression over a single-borrow row:
the borrow's entry value, an integer literal, and their sums and
differences — what a write before a call site leaves in the borrow. -/
inductive Symbolic where
  | borrowed
  | literal (value : Int)
  | add (left right : Symbolic)
  | sub (left right : Symbolic)
  deriving Repr, BEq

/-- The symbolic value of an expression, given the locals' values. -/
private partial def symbolic? (env : Nat → Option Symbolic) (e : Lean.Expr) :
    Option Symbolic := do
  if e.isAppOfArity ``localVar 1 then
    env (← natLeaf? (e.getArg! 0))
  else if e.isAppOfArity ``value 1 then
    let literal := e.getArg! 0
    guard (literal.isAppOfArity ``RuntimeValue.integer 1)
    return .literal (← (literal.getArg! 0).int?)
  else if e.isAppOfArity ``nativeReferenceOperation 2 then
    guard ((e.getArg! 0).isConstOf ``ReferenceLocationOperation.dereference)
    let operand ← singleOperandOf? (e.getArg! 1)
    guard (operand.isAppOfArity ``localVar 1)
    guard (natLeaf? (operand.getArg! 0) == some 0)
    return .borrowed
  else if e.isAppOfArity ``nativePrimitiveOperation 2 then
    let operationHead ← (e.getArg! 0).getAppFn.constName?
    let operands := e.getArg! 1
    guard (operands.isAppOfArity ``valuesCons 2)
    let rest := operands.getArg! 1
    guard (rest.isAppOfArity ``valuesCons 2)
    guard ((rest.getArg! 1).isConstOf ``valuesNil)
    let left ← symbolic? env (operands.getArg! 0)
    let right ← symbolic? env (rest.getArg! 0)
    if operationHead == ``PrimitiveLocationOperation.checkedAdd ||
        operationHead == ``PrimitiveLocationOperation.add then
      return .add left right
    else if operationHead == ``PrimitiveLocationOperation.checkedSubtract ||
        operationHead == ``PrimitiveLocationOperation.subtract then
      return .sub left right
    else none
  else none

/-- A symbolic value as a term over the borrow's entry value. -/
private partial def symbolicTerm (borrowed : Term) : Symbolic → CommandElabM Term
  | .borrowed => pure borrowed
  | .literal value =>
      if value < 0 then `(term| -$(Syntax.mkNumLit (toString (-value))))
      else `(term| $(Syntax.mkNumLit (toString value)))
  | .add left right => do
      `(term| ($(← symbolicTerm borrowed left) + $(← symbolicTerm borrowed right)))
  | .sub left right => do
      `(term| ($(← symbolicTerm borrowed left) - $(← symbolicTerm borrowed right)))

/-- A guard around the call statements: `if c then { s…; f(&mut *slot) }`
closing the body, with no else arm.  The statements before the first site
are scalar, and the value they write through the borrow is what the site
is entered with. -/
structure GuardedCalls where
  /-- The condition's events, on the scalar templates. -/
  condition : List Event
  /-- The statements before the first call site, each one's events. -/
  prelude : List (List Event)
  /-- The borrow's value those statements leave for the first site. -/
  written : Symbolic
  /-- Whether the arm is a `blockResult` whose result is the last call. -/
  resultBlock : Bool
  deriving Repr

/-- A body that opens with call statements through the mutable parameter
in local 0, optionally preceded by scalar `let`s and followed by a scalar
result expression. -/
structure CallPlan where
  /-- The body is the one call statement itself, not a block around it:
  its value is the callee's packed results. -/
  bare : Bool := false
  sites : List CallSite
  /-- Whether the caller's two parameters are both mutable scalars, every
  site reborrowing both. -/
  twoBorrows : Bool
  /-- The events of a `blockResult`'s result expression; `none` for a unit
  block. -/
  result : Option (List Event)
  /-- The scalar `let`s before the block: each initializer's events, then
  its binding. -/
  lets : List Event
  /-- Plain scalar locals after the borrow — parameters and `let`-bound
  alike: zero or one. -/
  plainLocals : Nat
  /-- Plain scalar parameters after the borrow: zero or one. -/
  plainParameters : Nat
  /-- Per parameter, whether it is an integer (`true`) or a `Bool`; the
  borrow leads. -/
  kinds : Array Bool
  /-- A guard around the call statements, with the statements before them. -/
  guarded : Option GuardedCalls := none

/-- Peel the scalar `let`s wrapping a block: each binds one local from an
initializer inside the scalar subset. -/
private partial def peelLets (tree : Lean.Expr) (lets : List Event) (binds : Nat)
    (initializers : Array (Nat × Lean.Expr) := #[]) :
    Option (Lean.Expr × List Event × Nat × Array (Nat × Lean.Expr)) := do
  if tree.isAppOfArity ``letNativeValue 3 then
    let binder := tree.getArg! 0
    guard (binder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
    let fuel ← binder.getArg! 0 |>.nat?
    guard (fuel > 0)
    let pattern := binder.getArg! 1
    guard (pattern.isAppOfArity ``LeanerIR.SemanticOperations.NativePattern.variable 1)
    let local_ ← natLeaf? (pattern.getArg! 0)
    let initializer := tree.getArg! 1
    guard !initializer.hasLooseBVars
    let events ← linearize initializer
    peelLets (tree.getArg! 2) (lets ++ events ++ [.bind local_ fuel]) (binds + 1)
      (initializers.push (local_, initializer))
  else pure (tree, lets, binds, initializers)

/-- Split a guarded arm's statements: the stable statements before the
first call site, tracking the value they write through the borrow, then
the call sites. -/
private partial def guardedStatements? (e : Lean.Expr) (env : Nat → Option Symbolic)
    (written : Symbolic) (clauseCountOf : Nat × Nat → Nat)
    (prelude : List (List Event)) :
    Option (List (List Event) × Symbolic × List CallSite) := do
  if e.isConstOf ``statementsNil then
    return (prelude, written, [])
  guard (e.isAppOfArity ``statementsCons 2)
  let head := e.getArg! 0
  if (callStatement? head).isSome then
    return (prelude, written, ← callStatements? e clauseCountOf)
  guard !head.hasLooseBVars
  let events ← linearize head
  /- A write through the borrow: its operand is the value the next site is
  entered with. -/
  let written' ← if head.isAppOfArity ``nativeReferenceOperation 2 &&
      (head.getArg! 0).isConstOf ``ReferenceLocationOperation.mutate then
    let operands := head.getArg! 1
    guard (operands.isAppOfArity ``valuesCons 2)
    let rest := operands.getArg! 1
    guard (rest.isAppOfArity ``valuesCons 2)
    symbolic? env (rest.getArg! 0)
  else pure written
  guardedStatements? (e.getArg! 1) env written' clauseCountOf (prelude ++ [events])

/-- A unit block whose one statement is a guarded arm with no else:
`if c then { … }` closing a `do` body. -/
private def guardedStatement? (tree : Lean.Expr) : Option (Lean.Expr × Lean.Expr) := do
  guard (tree.isAppOfArity ``blockUnit 1)
  let statements := tree.getArg! 0
  guard (statements.isAppOfArity ``statementsCons 2)
  guard ((statements.getArg! 1).isConstOf ``statementsNil)
  let branch := statements.getArg! 0
  guard (branch.isAppOfArity ``nativeBranch 3)
  guard ((branch.getArg! 2).isAppOf ``Option.none)
  pure (branch.getArg! 0, branch.getArg! 1)

/-- The guarded arm: its statements before the first call site, the value
they write, and the sites — in statement position, or closing with the
block's result. -/
private def guardedArm? (arm : Lean.Expr) (env : Nat → Option Symbolic)
    (clauseCountOf : Nat × Nat → Nat) :
    Option (List (List Event) × Symbolic × List CallSite × Bool) := do
  if arm.isAppOfArity ``blockUnit 1 then
    let (prelude, written, sites) ←
      guardedStatements? (arm.getArg! 0) env .borrowed clauseCountOf []
    /- The sites follow a statement only in a result block: after a
    statement's events the drive has opened the statement row, and only
    the empty row is re-closed from that state. -/
    guard prelude.isEmpty
    pure (prelude, written, sites, false)
  else if arm.isAppOfArity ``blockResult 2 then
    let (prelude, written, sites) ←
      guardedStatements? (arm.getArg! 0) env .borrowed clauseCountOf []
    let (relation, reborrows, handle) ← callStatement? (arm.getArg! 1)
    let callee ← calleeNames? relation
    pure (prelude, written,
      sites ++ [{ callee, reborrows, clauses := clauseCountOf handle,
                  resultPosition := true }],
      true)
  else none

/-- Recognize a call body: scalar `let`s around a `blockUnit` or
`blockResult` over a row of call statements, all through the mutable
parameter in local 0, with any result expression inside the scalar
subset. -/
def callPlan? (body : Lean.Expr) (parameterCount localCount : Nat)
    (tailKinds : Option (Array Bool)) (clauseCountOf : Nat × Nat → Nat)
    (secondBorrow : Bool := false) : Option CallPlan := do
  guard (parameterCount == 1 || parameterCount == 2)
  let twoBorrows := parameterCount == 2 && secondBorrow
  let kinds := if twoBorrows then #[] else #[true] ++ tailKinds.getD #[]
  let (tree, lets, binds, initializers) ← peelLets body.bindingBody! [] 0
  guard (localCount == parameterCount + binds)
  /- Two borrows are exactly the row; otherwise the proved exit rows hold a
  borrow beside at most one plain local. -/
  let plainLocals := if twoBorrows then 0 else localCount - 1
  guard (if twoBorrows then binds == 0 else plainLocals ≤ 1)
  let plainParameters := if twoBorrows then 0 else parameterCount - 1
  /- Every site reborrows what the row holds. -/
  let expected := if twoBorrows then 2 else 1
  if let some (conditionTree, arm) := guardedStatement? tree then
    /- A guarded arm of call statements closing the body. -/
    guard !twoBorrows
    guard !conditionTree.hasLooseBVars
    let condition ← linearize conditionTree
    /- The locals' values: the borrow's entry value, and each `let`'s
    initializer over it. -/
    let env : Nat → Option Symbolic := fun slot =>
      if slot == 0 then some .borrowed
      else (initializers.find? (·.1 == slot)).bind fun (_, initializer) =>
        symbolic? (fun inner => if inner == 0 then some .borrowed else none) initializer
    let (prelude, written, sites, resultBlock) ← guardedArm? arm env clauseCountOf
    guard !sites.isEmpty
    guard (sites.all (·.reborrows == expected))
    pure { sites, twoBorrows, result := none, lets, plainLocals, plainParameters, kinds,
           guarded := some { condition, prelude, written, resultBlock } }
  else if tree.isAppOfArity ``blockUnit 1 then
    let sites ← callStatements? (tree.getArg! 0) clauseCountOf
    guard !sites.isEmpty
    guard (sites.all (·.reborrows == expected))
    pure { sites, twoBorrows, result := none, lets, plainLocals, plainParameters, kinds }
  else if let some (relation, reborrows, handle) := callStatement? tree then
    /- An expression-bodied caller: the call statement is the body. -/
    guard (twoBorrows && binds == 0 && reborrows == expected)
    let callee ← calleeNames? relation
    pure { bare := true, sites := [{ callee, reborrows, clauses := clauseCountOf handle }],
           twoBorrows, result := none, lets, plainLocals, plainParameters, kinds }
  else do
    guard (tree.isAppOfArity ``blockResult 2)
    let sites ← callStatements? (tree.getArg! 0) clauseCountOf
    guard !sites.isEmpty
    guard (sites.all (·.reborrows == expected))
    let resultTree := tree.getArg! 1
    guard !resultTree.hasLooseBVars
    let events ← linearize resultTree
    pure { sites, twoBorrows, result := some events, lets, plainLocals, plainParameters, kinds }


/-- The hypothesis holding one callee's transported `Satisfies` fact,
bound once at the start of the script. -/
def calleeSatIdent (callee : CalleeNames) : Ident :=
  mkIdent (Name.mkSimple s!"calleeSat_{callee.relation.getPrefix.toString.replace "." "_"}")

/-- Bring a callee's contract fact into context under `calleeSatIdent`:
the verified theorem transported across the callee's agreement, or, for
the recursive call inside its own theorem, nothing — the induction
hypothesis is already there under that name. -/
private def calleeSatFact (caller : CallerNames) (callee : CalleeNames)
    (generic : Bool := false) : CommandElabM (TSyntax `tactic) := do
  if caller.selfRelation == some callee.relation then
    `(tactic| skip)
  else if generic then
    `(tactic|
      have $(calleeSatIdent callee):ident :=
        (LeanerIR.Proofs.satisfies_congr
          ($(mkIdent callee.agreement) (Carrier := fun _ => PUnit) $(n "hu"):ident)
          $(mkIdent callee.contract)).mpr
          ($(mkIdent callee.verified) $(n "prepared"):ident))
  else
    `(tactic|
      have $(calleeSatIdent callee):ident :=
        (LeanerIR.Proofs.satisfies_congr
          ($(mkIdent callee.agreement) $(n "hu"):ident)
          $(mkIdent callee.contract)).mpr
          ($(mkIdent callee.verified) $(n "prepared"):ident))

/-- The `Obligation` conjuncts of a callee's summary, nested after its
row and range facts: `((core ∧ o₁) ∧ o₂) …`, one per `ensures` clause. -/
private def obligationPattern (core : TSyntax `rcasesPat) (site : Nat) (clauses : Nat) :
    CommandElabM (TSyntax `rcasesPat × Array Ident) := do
  let mut pattern := core
  let mut idents : Array Ident := #[]
  for clause in [0:max clauses 1] do
    let ident := if clause == 0 then named "obligation" site
      else mkIdent (Name.mkSimple s!"obligation{site}_{clause}")
    idents := idents.push ident
    pattern ← `(rcasesPat| ⟨$pattern:rcasesPat, $ident:ident⟩)
  pure (pattern, idents)

/-- Emit the modular script for a calls-only body, mirroring
`LeanerLang/Tests/NativeCallRow.lean` mechanically.  Per-site hypothesis
names carry the site index; the current symbolic value of the borrowed
slot is threaded as a term — the parameter's value at entry, then the
value each callee's contract names for its exit, with the callee's
clauses kept as the site's facts. -/
private partial def emitCallSites (caller : CallerNames) (plan : CallPlan)
    (sites : List CallSite) (index : Nat) (currentValue : Term)
    (siteCount : Nat) (unwrap : Array (TSyntax `tactic) := #[]) :
    CommandElabM (TSyntax `tactic) := do
  match sites with
  | [] =>
      /- All calls returned: finish the block, export the borrow, close
      the caller's contract. -/
      let mut keyChain : Term ← `(term| $(n "keyFree"):ident)
      for i in List.range siteCount do
        let prevNext : Term ← if i == 0 then
          `(term| ($(n "initial"):ident).nextLoan)
        else
          `(term| ($(named "final" (i - 1))).nextLoan)
        keyChain ← `(term| ($(named "stableLookups" i) $(n "loanVar"):ident
          (show $(n "loanVar"):ident < $prevNext + 1 by omega)).trans
            $keyChain)
      let bumpedState (base : Term) : CommandElabM Term :=
        `(term| { globals := ($base).globals
                  globalLoans := ($base).globalLoans
                  nextLoan := ($base).nextLoan + 1
                  pending := ($(n "initial"):ident).pending })
      let mut disciplineChain : Term ← do
        let entry ← `(term| $(n "initial"):ident)
        `(term| LeanerIR.SemanticOperations.LoanDiscipline.of_eq
          (initial := $entry) (final := $(← bumpedState entry)) rfl
          (Nat.le_succ _))
      for i in List.range siteCount do
        disciplineChain ← `(term|
          ($disciplineChain).trans $(named "discipline" i))
        if i + 1 < siteCount then
          let base ← `(term| $(named "final" i))
          disciplineChain ← `(term| ($disciplineChain).trans
            (LeanerIR.SemanticOperations.LoanDiscipline.of_eq
              (initial := $base) (final := $(← bumpedState base)) rfl
              (Nat.le_succ _)))
      let lastFinal ← `(term| $(named "final" (siteCount - 1)))
      let exported ← `(term|
        { globals := ($lastFinal).globals
          globalLoans := ($lastFinal).globalLoans
          nextLoan := ($lastFinal).nextLoan
          pending := ($(n "initial"):ident).pending.push
            ($(n "loanVar"):ident,
              LeanerIR.RuntimeValue.integer $currentValue) })
      disciplineChain ← `(term| ($disciplineChain).trans
        (LeanerIR.SemanticOperations.LoanDiscipline.of_eq
          (initial := $lastFinal) (final := $exported) rfl
          (Nat.le_refl _)))
      let mut globalsChain : Term ← `(term| $(named "globalsEq" 0))
      for i in List.range (siteCount - 1) do
        globalsChain ← `(term|
          ($(named "globalsEq" (i + 1))).trans $globalsChain)
      let exportLemma := mkIdent <| if plan.plainLocals == 0 then
        ``LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_singleInteger
      else if plan.kinds.getD 1 true then
        ``LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowInteger
      else
        ``LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowBool
      /- The hole and its `case` share one quotation, so the export
      rewrite unifies its state from the goal and the freshness chain is
      supplied afterward at the spelling the goal chose. -/
      let exportRow ← `(tactic|
        (rw [$exportLemma:ident (noGlobal := ?rowKeyFree)]
         case rowKeyFree => exact $keyChain))
      if let some events := plan.result then
        /- A result expression follows the calls: continue with the
        scalar drive from the moved row, and close each leaf through the
        export and the constructive closing, with the loan-discipline
        chain in context. -/
        let finish : Finish := {
          returned := (← scalarReturnedPrefix) ++ #[exportRow,
            ← `(tactic| have $(n "rowDiscipline"):ident := $disciplineChain),
            ← `(tactic| leaner_certified_close!)]
          thrown := (← scalarThrownPrefix) ++
            #[← `(tactic| leaner_certified_close!)]
          mutateRow := mkIdent
            ``LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_singleBorrow }
        let reads := events.takeWhile (· == .read) |>.length
        let restEvents := events.drop reads
        let resolution ← seq <|
          (← readBatch reads) ++ (← emitEvents finish restEvents 0)
        return ← `(tactic|
          (apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_nil
           apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow
           case stable => leaner_row_stable
           leaner_row_drive
           $resolution:tactic))
      let unwrapTactic ← if unwrap.isEmpty then `(tactic| skip) else seq unwrap
      `(tactic|
        (apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_nil
         $unwrapTactic:tactic
         intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finishControl?,
           LeanerIR.SemanticOperations.unpackFallthrough,
           Option.map_eq_map, Option.map_some, Option.some.injEq]
           at $(n "rowFinished"):ident
         subst $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
         $exportRow:tactic
         simp only [$(caller.rawContract):term, $(caller.resultsCodec):term]
         refine ⟨PUnit.unit, trivial,
           fun _ => ⟨$(n "loanVar"):ident, $(n "valVar"):ident,
             $currentValue,
             .integer $currentValue,
             ⟨⟨⟨rfl, trivial⟩, rfl,
               (LeanerIR.SemanticOperations.resolveReturnedBorrows_empty
                 _).symm⟩,
               by omega, by omega⟩, ?_⟩,
           ⟨$globalsChain, $disciplineChain⟩, ?_⟩
         · simp only [LeanerIR.Proofs.Obligation_iff]
           omega
         · rintro ⟨$(n "rowLoan"):ident, $(n "rowSlot"):ident,
             $(n "rowArgsEq"):ident, $(n "rowOverflow"):ident⟩
           simp only [Array.mk.injEq, List.cons.injEq,
             LeanerIR.RuntimeValue.borrow.injEq,
             LeanerIR.RuntimeValue.integer.injEq, and_true]
             at $(n "rowArgsEq"):ident
           obtain ⟨-, $(n "rowValEq"):ident⟩ := $(n "rowArgsEq"):ident
           omega))
  | site :: rest =>
      let callee := site.callee
      let i := index
      let calleeContract := mkIdent callee.contract
      let calleeTyped := mkIdent callee.typedContract
      let calleeRaw := mkIdent callee.rawContract
      let calleeCodec := mkIdent callee.argumentsCodec
      let stateNext : Term ← if i == 0 then
        `(term| ($(n "initial"):ident).nextLoan)
      else
        `(term| ($(named "final" (i - 1))).nextLoan)
      let freshSource : Term ← if i == 0 then
        `(term| $(n "freshLoans"):ident)
      else
        `(term| $(named "freshFinal" (i - 1)))
      let priorLoanTactic ← if i == 0 then
        `(tactic| exact $(n "loanBound"):ident)
      else
        `(tactic|
          (show $(n "loanVar"):ident < $(named "final" (i - 1)).nextLoan
           omega))
      let core ← `(rcasesPat| ⟨⟨⟨$(n "rowArgsEq2"):ident, $(n "rowResultsEq"):ident⟩,
        $(named "pendingEq" i):ident, $(n "rowPendResolve"):ident⟩,
        $(named "slotNonNeg" i):ident, $(named "slotMax" i):ident⟩)
      let (obligations, obligationIdents) ← obligationPattern core i site.clauses
      let continuation ← emitCallSites caller plan rest (i + 1)
        (← `(term| $(named "slot" i):ident)) siteCount unwrap
      /- A site in result position is read off the expression: the value
      it leaves is the arm's, and the enclosing statement row continues
      after a head reduction. -/
      let law := mkIdent <| if site.resultPosition then
          ``LeanerIR.Proofs.Denotation.wpRowThrow_callReborrowStatement
        else ``LeanerIR.Proofs.Denotation.wpStatementsRowThrow_consCallReborrow
      let continuation ← if site.resultPosition then
          `(tactic| ((try leaner_row_head)
                     $continuation:tactic))
        else pure continuation
      `(tactic|
        (apply $law:ident
         case mutableKind => rfl
         case priorLoan => $priorLoanTactic:tactic
         case calleeWp =>
           apply LeanerIR.Proofs.Denotation.wpFunction_of_satisfies
             $(calleeSatIdent callee)
           case permitted =>
             simp only [$calleeContract:term,
               LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
               LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
               $calleeCodec:term,
               LeanerIR.Proofs.Codec.mutable, LeanerIR.Proofs.Codec.specInt]
             refine ⟨⟨⟨$stateNext, ⟨$currentValue, ?_⟩⟩⟩, rfl,
               $stateNext, $currentValue,
               ⟨⟨⟨rfl, by omega, by omega⟩, ?_⟩, by omega⟩, ?_⟩
             · simp [LeanerIR.IntegerValueFits,
                 LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
               omega
             · intro $(n "rowLoan"):ident $(n "rowBound"):ident
               have $(n "rowAdvanced"):ident :
                   $stateNext + 1 ≤ $(n "rowLoan"):ident :=
                 $(n "rowBound"):ident
               exact $freshSource $(n "rowLoan"):ident (by omega)
             · simp only [LeanerIR.SemanticOperations.globalLoanKey?]
               exact $freshSource $stateNext (by omega)
           case onReturn =>
             intro $(named "results" i) $(named "final" i)
               $(named "ensuresFn" i) $(named "frameFn" i)
               $(named "notMust" i)
             have $(named "ensures" i):ident := $(named "ensuresFn" i)
               (by exact $(named "notMust" i))
             simp only [$calleeContract:term,
               LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
               LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
               at $(named "ensures" i):ident
             simp only [$calleeContract:term,
               LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
               LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
               at $(named "frameFn" i):ident
             obtain ⟨$(n "rowArgs"):ident, $(n "rowResult"):ident,
               $(n "rowArgsEq"):ident, $(n "rowDecodeEq"):ident,
               $(n "rowSlotLoan"):ident, $(n "rowSlotEntry"):ident,
               $(named "slot" i):ident, $(n "rowSlotPending"):ident,
               $obligations:rcasesPat⟩ := $(named "ensures" i):ident
             rw [$(n "rowArgsEq"):ident] at $(n "rowArgsEq2"):ident
             simp only [Array.mk.injEq, List.cons.injEq,
               LeanerIR.RuntimeValue.borrow.injEq,
               LeanerIR.RuntimeValue.integer.injEq, and_true]
               at $(n "rowArgsEq2"):ident
             obtain ⟨$(n "rowLoanEq"):ident, $(n "rowEntryEq"):ident⟩ :=
               $(n "rowArgsEq2"):ident
             subst $(n "rowLoanEq"):ident $(n "rowEntryEq"):ident
             rw [LeanerIR.SemanticOperations.resolveReturnedBorrows_empty]
               at $(n "rowPendResolve"):ident
             subst $(n "rowPendResolve"):ident
             simp only [LeanerIR.Proofs.Obligation_iff] at $obligationIdents*
             obtain ⟨$(n "rowArgs2"):ident, $(n "rowArgsEq3"):ident,
               $(named "globalsEq" i):ident, $(named "discipline" i):ident⟩ :=
               $(named "frameFn" i):ident
             have $(named "monotone" i):ident :
                 $stateNext + 1 ≤ $(named "final" i).nextLoan :=
               $(named "discipline" i):ident.2.2
             have $(named "freshFinal" i):ident :
                 LeanerIR.SemanticOperations.FreshGlobalLoanIds
                   $(named "final" i) :=
               $(named "discipline" i):ident.1
                 (fun loan bound => $freshSource loan
                   (by exact Nat.le_trans (Nat.le_succ _) bound))
             have $(named "stableLookups" i):ident :=
               $(named "discipline" i):ident.2.1
             refine ⟨$(named "slot" i):ident, $(named "pendingEq" i):ident, ?_⟩
             $continuation:tactic
           case onThrow =>
             intro $(n "rowKind"):ident $(n "rowThrown"):ident
               $(named "final" i) $(named "aborts" i)
             intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
               $(n "rowOutcome"):ident $(n "rowFinished"):ident
             simp only [LeanerIR.SemanticOperations.finishControl?,
               Option.some.injEq] at $(n "rowFinished"):ident
             subst $(n "rowFinished"):ident
             simp only [$calleeContract:term,
               LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
               LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
               at $(named "aborts" i):ident
             obtain ⟨$(n "rowArgs"):ident, $(n "rowArgsEq"):ident,
               $(n "rowSlotLoan"):ident, $(n "rowSlot"):ident,
               $(n "rowArgsEq2"):ident, $(n "rowObligation"):ident⟩ :=
               $(named "aborts" i):ident
             rw [$(n "rowArgsEq"):ident] at $(n "rowArgsEq2"):ident
             simp only [Array.mk.injEq, List.cons.injEq,
               LeanerIR.RuntimeValue.borrow.injEq,
               LeanerIR.RuntimeValue.integer.injEq, and_true]
               at $(n "rowArgsEq2"):ident
             obtain ⟨-, $(n "rowSlotEq"):ident⟩ := $(n "rowArgsEq2"):ident
             subst $(n "rowSlotEq"):ident
             simp only [$(caller.rawContract):term]
             simp only [LeanerIR.Proofs.Obligation_iff]
               at $(n "rowObligation"):ident
             leaner_certified_close!))

/-- The modular script for a calls-only two-borrow body: each site reborrows
both parameters, and the callee's contract names both exit values. -/
private partial def emitTwoBorrowSites (caller : CallerNames) (plan : CallPlan)
    (sites : List CallSite) (index : Nat) (leftValue rightValue : Term)
    (siteCount : Nat) : CommandElabM (TSyntax `tactic) := do
  match sites with
  | [] =>
      let mut leftChain : Term ← `(term| $(n "leftKeyFree"):ident)
      let mut rightChain : Term ← `(term| $(n "rightKeyFree"):ident)
      for i in List.range siteCount do
        let prevNext : Term ← if i == 0 then
          `(term| ($(n "initial"):ident).nextLoan)
        else
          `(term| ($(named "final" (i - 1))).nextLoan)
        leftChain ← `(term| ($(named "stableLookups" i) $(n "leftLoan"):ident
          (show $(n "leftLoan"):ident < $prevNext + 1 + 1 by omega)).trans $leftChain)
        rightChain ← `(term| ($(named "stableLookups" i) $(n "rightLoan"):ident
          (show $(n "rightLoan"):ident < $prevNext + 1 + 1 by omega)).trans $rightChain)
      let bumpedState (base : Term) : CommandElabM Term :=
        `(term| { globals := ($base).globals
                  globalLoans := ($base).globalLoans
                  nextLoan := ($base).nextLoan + 1 + 1
                  pending := ($(n "initial"):ident).pending })
      let mut disciplineChain : Term ← do
        let entry ← `(term| $(n "initial"):ident)
        `(term| LeanerIR.SemanticOperations.LoanDiscipline.of_eq
          (initial := $entry) (final := $(← bumpedState entry)) rfl
          (Nat.le_succ_of_le (Nat.le_succ _)))
      for i in List.range siteCount do
        disciplineChain ← `(term| ($disciplineChain).trans $(named "discipline" i))
        if i + 1 < siteCount then
          let base ← `(term| $(named "final" i))
          disciplineChain ← `(term| ($disciplineChain).trans
            (LeanerIR.SemanticOperations.LoanDiscipline.of_eq
              (initial := $base) (final := $(← bumpedState base)) rfl
              (Nat.le_succ_of_le (Nat.le_succ _))))
      let lastFinal ← `(term| $(named "final" (siteCount - 1)))
      let exported ← `(term|
        { globals := ($lastFinal).globals
          globalLoans := ($lastFinal).globalLoans
          nextLoan := ($lastFinal).nextLoan
          pending := (($(n "initial"):ident).pending.push
            ($(n "leftLoan"):ident, LeanerIR.RuntimeValue.integer $leftValue)).push
            ($(n "rightLoan"):ident, LeanerIR.RuntimeValue.integer $rightValue) })
      disciplineChain ← `(term| ($disciplineChain).trans
        (LeanerIR.SemanticOperations.LoanDiscipline.of_eq
          (initial := $lastFinal) (final := $exported) rfl (Nat.le_refl _)))
      let exportRow ← `(tactic|
        (rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_twoIntegers
            (noLeftGlobal := ?rowLeftKeyFree) (noRightGlobal := ?rowRightKeyFree)]
         case rowLeftKeyFree => exact $leftChain
         case rowRightKeyFree => exact $rightChain))
      let finish : Finish := {
        returned := (← scalarReturnedPrefix) ++ #[exportRow,
          ← `(tactic| have $(n "rowDiscipline"):ident := $disciplineChain),
          ← `(tactic| leaner_certified_close!)]
        thrown := (← scalarThrownPrefix) ++ #[← `(tactic| leaner_certified_close!)]
        mutateRow := mkIdent ``LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_twoBorrows_left
        mutateRowSecond :=
          some (mkIdent ``LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_twoBorrows_right) }
      let events := plan.result.getD []
      let reads := events.takeWhile (· == .read) |>.length
      let restEvents := events.drop reads
      let resolution ← seq <| (← readBatch reads) ++ (← emitEvents finish restEvents 0)
      let value ← if plan.result.isSome then
          `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow)
        else `(tactic| skip)
      let stable ← if plan.result.isSome then
          `(tactic| case stable => leaner_row_stable)
        else `(tactic| skip)
      /- The statement spine ends at its marker; the bare body's value is
      the callee's packed results, no results at all. -/
      let spineEnd ← if plan.bare then
          `(tactic| rw [LeanerIR.Proofs.Denotation.packResults_nil])
        else `(tactic| apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_nil)
      return ← `(tactic|
        ($spineEnd:tactic
         $value:tactic
         $stable:tactic
         leaner_row_drive
         $resolution:tactic))
  | site :: rest =>
      let callee := site.callee
      let i := index
      let calleeContract := mkIdent callee.contract
      let calleeTyped := mkIdent callee.typedContract
      let calleeRaw := mkIdent callee.rawContract
      let calleeCodec := mkIdent callee.argumentsCodec
      let calleeResults := mkIdent callee.resultsCodec
      let stateNext : Term ← if i == 0 then
        `(term| ($(n "initial"):ident).nextLoan)
      else
        `(term| ($(named "final" (i - 1))).nextLoan)
      let freshSource : Term ← if i == 0 then
        `(term| $(n "freshLoans"):ident)
      else
        `(term| $(named "freshFinal" (i - 1)))
      let priorTactic (loan bound : Ident) : CommandElabM (TSyntax `tactic) :=
        if i == 0 then `(tactic| exact $bound:ident)
        else `(tactic| (show $loan:ident < $(named "final" (i - 1)).nextLoan; omega))
      let leftPriorTactic ← priorTactic (n "leftLoan") (n "leftBound")
      let rightPriorTactic ← priorTactic (n "rightLoan") (n "rightBound")
      let core ← `(rcasesPat| ⟨⟨⟨⟨$(n "rowArgsEq2"):ident, $(n "rowResultsEq"):ident⟩,
        ⟨$(named "pendingEq" i):ident, $(n "rowLeftResolve"):ident⟩,
        $(n "rowRightResolve"):ident⟩,
        $(named "leftNonNeg" i):ident, $(named "leftMax" i):ident⟩,
        $(named "rightNonNeg" i):ident, $(named "rightMax" i):ident⟩)
      let (obligations, obligationIdents) ← obligationPattern core i site.clauses
      let continuation ← emitTwoBorrowSites caller plan rest (i + 1)
        (← `(term| $(named "leftSlot" i):ident)) (← `(term| $(named "rightSlot" i):ident))
        siteCount
      /- A bare body's callee returns no results: its result row is
      recovered as empty from the decoded unit, so the value it leaves
      is known. -/
      let continuation ← if plan.bare then `(tactic|
          (simp only [$calleeResults:term] at $(n "rowDecodeEq"):ident
           split at $(n "rowDecodeEq"):ident
           · rename_i $(n "listEq"):ident
             have $(n "resultsNil"):ident : $(named "results" i) = #[] := by
               have $(n "listArray"):ident := congrArg List.toArray $(n "listEq"):ident
               simpa using $(n "listArray"):ident
             subst $(n "resultsNil"):ident
             refine ⟨$(named "leftSlot" i):ident, $(named "rightSlot" i):ident,
               $(named "pendingEq" i):ident, ?_⟩
             $continuation:tactic
           · exact absurd $(n "rowDecodeEq"):ident (by simp)))
        else `(tactic|
          (refine ⟨$(named "leftSlot" i):ident, $(named "rightSlot" i):ident,
             $(named "pendingEq" i):ident, ?_⟩
           $continuation:tactic))
      let siteLaw ← if plan.bare then
          `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_callTwoReborrows)
        else
          `(tactic| apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_consCallTwoReborrows)
      `(tactic|
        ($siteLaw:tactic
         case mutableKind => rfl
         case distinctLexical => decide
         case leftPrior => $leftPriorTactic:tactic
         case rightPrior => $rightPriorTactic:tactic
         case calleeWp =>
           apply LeanerIR.Proofs.Denotation.wpFunction_of_satisfies
             $(calleeSatIdent callee)
           case permitted =>
             simp only [$calleeContract:term,
               LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
               LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
               $calleeCodec:term,
               LeanerIR.Proofs.Codec.mutable, LeanerIR.Proofs.Codec.specInt]
             refine ⟨⟨⟨$stateNext, ⟨$leftValue, ?_⟩⟩, ⟨$stateNext + 1, ⟨$rightValue, ?_⟩⟩⟩, rfl,
               $stateNext, $leftValue, $stateNext + 1, $rightValue,
               ⟨⟨⟨⟨⟨⟨⟨rfl, by omega, by omega⟩, by omega, by omega⟩, ?_⟩, by omega⟩, ?_⟩,
                 by omega⟩, ?_⟩, by omega⟩
             · simp [LeanerIR.IntegerValueFits,
                 LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
               omega
             · simp [LeanerIR.IntegerValueFits,
                 LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
               omega
             · intro $(n "rowLoan"):ident $(n "rowBound"):ident
               have $(n "rowAdvanced"):ident :
                   $stateNext + 1 + 1 ≤ $(n "rowLoan"):ident :=
                 $(n "rowBound"):ident
               exact $freshSource $(n "rowLoan"):ident (by omega)
             · simp only [LeanerIR.SemanticOperations.globalLoanKey?]
               exact $freshSource $stateNext (by omega)
             · simp only [LeanerIR.SemanticOperations.globalLoanKey?]
               exact $freshSource ($stateNext + 1) (by omega)
           case onReturn =>
             intro $(named "results" i) $(named "final" i)
               $(named "ensuresFn" i) $(named "frameFn" i)
               $(named "notMust" i)
             have $(named "ensures" i):ident := $(named "ensuresFn" i)
               (by exact $(named "notMust" i))
             simp only [$calleeContract:term,
               LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
               LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
               at $(named "ensures" i):ident
             simp only [$calleeContract:term,
               LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
               LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
               at $(named "frameFn" i):ident
             obtain ⟨$(n "rowArgs"):ident, $(n "rowResult"):ident,
               $(n "rowArgsEq"):ident, $(n "rowDecodeEq"):ident,
               $(n "rowLeftLoan"):ident, $(n "rowLeftEntry"):ident,
               $(named "leftSlot" i):ident, $(n "rowLeftPending"):ident,
               $(n "rowRightLoan"):ident, $(n "rowRightEntry"):ident,
               $(named "rightSlot" i):ident, $(n "rowRightPending"):ident,
               $obligations:rcasesPat⟩ := $(named "ensures" i):ident
             rw [$(n "rowArgsEq"):ident] at $(n "rowArgsEq2"):ident
             simp only [Array.mk.injEq, List.cons.injEq,
               LeanerIR.RuntimeValue.borrow.injEq,
               LeanerIR.RuntimeValue.integer.injEq, and_true]
               at $(n "rowArgsEq2"):ident
             obtain ⟨⟨$(n "rowLeftLoanEq"):ident, $(n "rowLeftEntryEq"):ident⟩,
               $(n "rowRightLoanEq"):ident, $(n "rowRightEntryEq"):ident⟩ :=
               $(n "rowArgsEq2"):ident
             subst $(n "rowLeftLoanEq"):ident $(n "rowLeftEntryEq"):ident
               $(n "rowRightLoanEq"):ident $(n "rowRightEntryEq"):ident
             rw [LeanerIR.SemanticOperations.resolveReturnedBorrows_empty]
               at $(n "rowLeftResolve"):ident $(n "rowRightResolve"):ident
             subst $(n "rowLeftResolve"):ident $(n "rowRightResolve"):ident
             simp only [LeanerIR.Proofs.Obligation_iff] at $obligationIdents*
             obtain ⟨$(n "rowArgs2"):ident, $(n "rowArgsEq3"):ident,
               $(named "globalsEq" i):ident, $(named "discipline" i):ident⟩ :=
               $(named "frameFn" i):ident
             have $(named "monotone" i):ident :
                 $stateNext + 1 + 1 ≤ $(named "final" i).nextLoan :=
               $(named "discipline" i):ident.2.2
             have $(named "freshFinal" i):ident :
                 LeanerIR.SemanticOperations.FreshGlobalLoanIds
                   $(named "final" i) :=
               $(named "discipline" i):ident.1
                 (fun loan bound => $freshSource loan
                   (Nat.le_trans (Nat.le_succ_of_le (Nat.le_succ _)) bound))
             have $(named "stableLookups" i):ident :=
               $(named "discipline" i):ident.2.1
             $continuation:tactic
           case onThrow =>
             intro $(n "rowKind"):ident $(n "rowThrown"):ident
               $(named "final" i) $(named "aborts" i)
             intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
               $(n "rowOutcome"):ident $(n "rowFinished"):ident
             simp only [LeanerIR.SemanticOperations.finishControl?,
               Option.some.injEq] at $(n "rowFinished"):ident
             subst $(n "rowFinished"):ident
             simp only [$calleeContract:term,
               LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
               LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
               at $(named "aborts" i):ident
             obtain ⟨$(n "rowArgs"):ident, $(n "rowArgsEq"):ident,
               $(n "rowLeftLoan"):ident, $(n "rowLeft"):ident,
               $(n "rowRightLoan"):ident, $(n "rowRight"):ident,
               $(n "rowArgsEq2"):ident, $(n "rowObligation"):ident⟩ :=
               $(named "aborts" i):ident
             rw [$(n "rowArgsEq"):ident] at $(n "rowArgsEq2"):ident
             simp only [Array.mk.injEq, List.cons.injEq,
               LeanerIR.RuntimeValue.borrow.injEq,
               LeanerIR.RuntimeValue.integer.injEq, and_true]
               at $(n "rowArgsEq2"):ident
             obtain ⟨⟨-, $(n "rowLeftEq"):ident⟩, -, $(n "rowRightEq"):ident⟩ :=
               $(n "rowArgsEq2"):ident
             subst $(n "rowLeftEq"):ident $(n "rowRightEq"):ident
             simp only [$(caller.rawContract):term]
             simp only [LeanerIR.Proofs.Obligation_iff]
               at $(n "rowObligation"):ident
             leaner_certified_close!))

/-- The complete modular script for a calls-only single-borrow body, from
the goal `wp_nativeFunction` leaves. -/
def callScript (caller : CallerNames) (plan : CallPlan) :
    CommandElabM (TSyntax `tactic) := do
  let sites := plan.sites
  let resolution ← if plan.twoBorrows then
      emitTwoBorrowSites caller plan sites 0
        (← `(term| $(n "leftVal"):ident)) (← `(term| $(n "rightVal"):ident)) sites.length
    else
      emitCallSites caller plan sites 0
        (← `(term| $(n "valVar"):ident)) sites.length
  /- The entry row: the parameters, then a slot per `let`-bound local. -/
  let borrowArg ← `(term| .borrow $(n "loanVar"):ident (.integer $(n "valVar"):ident))
  let mut arguments : Array Term := #[borrowArg]
  if plan.plainParameters == 1 then
    arguments := arguments.push (← if plan.kinds.getD 1 true then
      `(term| .integer $(n "amountVar"):ident) else `(term| .bool $(n "amountVar"):ident))
  let mut slots : Array Term := #[]
  for argument in arguments do
    slots := slots.push (← `(term| some $argument))
  for _ in [0:plan.plainLocals - plan.plainParameters] do
    slots := slots.push (← `(term| none))
  let localsLit := Syntax.mkNatLit (plan.plainLocals + 1)
  let entryLoans ← if plan.twoBorrows then
    `(term| LeanerIR.SemanticOperations.parameterLoanLocations_twoBorrows)
  else if plan.plainParameters == 0 then
    `(term| LeanerIR.SemanticOperations.parameterLoanLocations_singleBorrow)
  else if plan.kinds.getD 1 true then
    `(term| LeanerIR.SemanticOperations.parameterLoanLocations_borrowInteger)
  else
    `(term| LeanerIR.SemanticOperations.parameterLoanLocations_borrowBool)
  let entry ← if plan.twoBorrows then
      `(tactic| simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
         LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.mutable,
         LeanerIR.Proofs.Denotation.initialLocals_two, $entryLoans:term])
    else if plan.plainLocals == plan.plainParameters then
      /- Parameters only: the closed entry rows, in one pass. -/
      let entryLocals ← if plan.plainLocals == 0 then
          `(term| LeanerIR.Proofs.Denotation.initialLocals_one)
        else `(term| LeanerIR.Proofs.Denotation.initialLocals_two)
      `(tactic| simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
         LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.mutable,
         LeanerIR.Proofs.Codec.bool, $entryLocals:term, $entryLoans:term])
    else
      `(tactic|
        (simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
           LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.mutable,
           LeanerIR.Proofs.Codec.bool]
         rw [show LeanerIR.SemanticOperations.initialLocals $localsLit
             #[$arguments,*] = #[$slots,*] by
             simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
           $entryLoans:term]))
  /- Each distinct callee's contract fact is transported once; elaborating
  the transport inline at every site re-unifies whole contract bodies. -/
  let mut seen : Array Name := #[]
  let mut calleeFacts : Array (TSyntax `tactic) := #[]
  for site in sites do
    let callee := site.callee
    unless seen.contains callee.relation do
      seen := seen.push callee.relation
      calleeFacts := calleeFacts.push (← calleeSatFact caller callee)
  let calleeSetup ← seq calleeFacts
  /- The caller's requires-side facts arrive inaccessible, in the order
  the generated preamble destructures them; the list is read off the real
  context for each supported parameter shape. -/
  let slot := caller.parameters.getD 0 "slot"
  let renames ← if plan.twoBorrows then
    let left := caller.parameters.getD 0 "left"
    let right := caller.parameters.getD 1 "right"
    bindVocabulary #[(s!"{left}_loan", "leftLoan"), (left, "leftVal"),
      (s!"{left}_loan_keyFree", "leftKeyFree"), (s!"{left}_loan_bound", "leftBound"),
      (s!"{right}_loan", "rightLoan"), (right, "rightVal"),
      (s!"{right}_loan_keyFree", "rightKeyFree"), (s!"{right}_loan_bound", "rightBound")]
  else if plan.plainParameters == 0 then
    bindVocabulary (borrowVocabulary slot)
  else
    let amount := caller.parameters.getD 1 "amount"
    if plan.kinds.getD 1 true then
      bindVocabulary (borrowVocabulary slot ++
        #[(amount, "amountVar"), (s!"{amount}_fits", "amountFits"),
          (s!"{amount}_nonNeg", "amountNonNeg"), (s!"{amount}_max", "amountMax"),
          (s!"{amount}_range", "amountFitsConj")])
    else bindVocabulary (borrowVocabulary slot ++ #[(amount, "amountVar")])
  let blockBridge ← if plan.bare then `(tactic| skip)
  else if plan.result.isSome then
    `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_blockResult)
  else
    `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_blockUnit)
  /- A guard around the block: the condition runs on the scalar templates
  and the path splits.  The then-arm runs the statements before the first
  site on the templates too — a write through the borrow changes the
  value the site is entered with — and then the sites; the else-arm
  exports the borrow as it came. -/
  let mutateRow := mkIdent <| if plan.plainLocals == 0 then
      ``LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_singleBorrow
    else ``LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_borrowFirst
  let exportRow ← if plan.plainLocals == 0 then
      `(tactic| rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_singleInteger
          _ _ _ _ _ (by assumption)])
    else if plan.kinds.getD 1 true then
      `(tactic| rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowInteger
          _ _ _ _ _ _ (by assumption)])
    else
      `(tactic| rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowBool
          _ _ _ _ _ _ (by assumption)])
  let guardedBlock ← match plan.guarded with
    | none => pure #[blockBridge, resolution]
    | some guarded =>
        let written ← symbolicTerm (← `(term| $(n "valVar"):ident)) guarded.written
        /- After the sites: the arm's statement row ends, then the body's. -/
        let unwrap : Array (TSyntax `tactic) ← if guarded.resultBlock then pure #[]
          else pure #[← `(tactic| try leaner_row_head),
            ← `(tactic| apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_nil)]
        let sitesResolution ← emitCallSites caller plan sites 0 written sites.length unwrap
        /- After the statements, the empty row before the result: closed
        from the opened form the drive leaves after a statement's events,
        or by the rule when no statement ran. -/
        let mut armBody : Array (TSyntax `tactic) ← if guarded.resultBlock then
            if guarded.prelude.isEmpty then
              pure #[← `(tactic| try leaner_row_head),
                ← `(tactic| apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_nil),
                sitesResolution]
            else
              pure #[← `(tactic| rename_i $(n "nilResult"):ident $(n "nilStep"):ident),
                ← `(tactic| simp only [LeanerIR.Proofs.Denotation.statementsNil]
                    at $(n "nilStep"):ident),
                ← `(tactic| subst $(n "nilStep"):ident),
                ← `(tactic| refine ⟨_, _, rfl, ?_⟩),
                sitesResolution]
          else pure #[← `(tactic| try leaner_row_head), sitesResolution]
        /- The statements before the sites, each a throw-aware step on the
        scalar templates, nesting from the last to the first. -/
        for events in guarded.prelude.reverse do
          let finish : Finish := {
            returned := armBody
            thrown := #[← `(tactic| leaner_certified_close!)]
            mutateRow, parameters := caller.parameters }
          let reads := events.takeWhile (· == .read) |>.length
          let rest := events.drop reads
          armBody := #[
              ← `(tactic| try leaner_row_head),
              ← `(tactic| apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_cons),
              ← `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow),
              ← `(tactic| case stable => leaner_row_stable),
              ← `(tactic| leaner_row_drive)]
            ++ (← readBatch reads) ++ (← emitEvents finish rest 0)
        let armEntry ← if guarded.resultBlock then
            `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_blockResult)
          else `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_blockUnit)
        let thenArm ← seq (#[armEntry] ++ armBody)
        let elseArm ← seq #[
            ← `(tactic| try leaner_row_head),
            ← `(tactic| apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_nil),
            ← `(tactic| intro $(n "rowOutcome"):ident $(n "rowFinished"):ident),
            ← `(tactic| simp only [LeanerIR.SemanticOperations.finishControl?,
                LeanerIR.SemanticOperations.unpackFallthrough,
                Option.map_eq_map, Option.map_some, Option.some.injEq]
                at $(n "rowFinished"):ident),
            ← `(tactic| subst $(n "rowFinished"):ident),
            ← `(tactic| simp only [LeanerIR.SemanticOperations.finalizeFunctionState]),
            exportRow,
            ← `(tactic| simp only [$(caller.rawContract):term, $(caller.resultsCodec):term]),
            ← `(tactic| leaner_certified_close!)]
        let split : Array (TSyntax `tactic) := #[
          ← `(tactic| leaner_row_drive),
          ← `(tactic| refine ⟨fun $(n "rowGuard"):ident => ?_, fun $(n "rowGuard"):ident => ?_⟩),
          ← `(tactic| · ((try simp only [decide_eq_true_eq] at $(n "rowGuard"):ident)
                         $thenArm:tactic)),
          ← `(tactic| · ((try simp only [decide_eq_false_iff_not] at $(n "rowGuard"):ident)
                         $elseArm:tactic))]
        let conditionFinish : Finish := {
          returned := split
          thrown := #[← `(tactic| leaner_certified_close!)]
          mutateRow, parameters := caller.parameters }
        let conditionReads := guarded.condition.takeWhile (· == .read) |>.length
        let conditionRest := guarded.condition.drop conditionReads
        pure <| #[
            ← `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_blockUnit),
            ← `(tactic| apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_cons),
            ← `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_nativeBranchThrow),
            ← `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow),
            ← `(tactic| case stable => leaner_row_stable),
            ← `(tactic| leaner_row_drive)]
          ++ (← readBatch conditionReads) ++ (← emitEvents conditionFinish conditionRest 0)
  /- Each `let` before the block is a throw-aware step whose initializer
  runs on the scalar templates and whose binding hands the block the row
  with its local set; the lets nest from the last to the first. -/
  let mut block : Array (TSyntax `tactic) := guardedBlock
  let mut segments : Array (List Event × Nat × Nat) := #[]
  let mut pending : List Event := []
  for event in plan.lets do
    match event with
    | .bind local_ fuel =>
        segments := segments.push (pending, local_, fuel)
        pending := []
    | other => pending := pending ++ [other]
  for (events, local_, fuel) in segments.reverse do
    let finish : Finish := {
      returned := (← bindStep local_ fuel) ++ block
      thrown := #[← `(tactic| leaner_certified_close!)]
      mutateRow := mkIdent ``LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_singleBorrow
      parameters := caller.parameters }
    let reads := events.takeWhile (· == .read) |>.length
    let rest := events.drop reads
    block := #[
      ← `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_letNativeValue),
      ← `(tactic| case initializerStable => leaner_row_stable),
      ← `(tactic| leaner_row_drive)]
      ++ (← readBatch reads) ++ (← emitEvents finish rest 0)
  let body ← seq block
  `(tactic|
    ($calleeSetup:tactic
     $renames:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term,
       $(caller.shape):term]
     $entry:tactic
     $body:tactic))


/-! ## Storage bodies: the field bracket

`deposit`'s shape — a mutable global borrow of the resource, a two-step
field reborrow through it, an inner scalar block, the paired death marker
— is resolved through `wpRowThrow_focusedFieldBracket`, with the inner
block on the scalar event templates and both exits closed by the
storage-aware certified closer through the family representation. -/

/-- One step of a bracket's focused path, as the twin metadata spells it:
the struct's twin, its runtime identity, its field row, and the focused
field. -/
structure PathStep where
  twin : Name
  structIndex : Nat
  fields : Array (String × SpecTypes.FieldRep)
  focus : Nat
  deriving Repr, BEq

/-- The resource twin a bracket writes through, and the path it focuses:
every struct from the resource down to the integer the bracket reads and
writes. -/
structure StoreTwin where
  twin : Name
  namespaceIndex : Nat
  typeIndex : Nat
  path : List PathStep

/-- The two bracket shapes the laws cover. -/
inductive StoreShape where
  /-- A field reborrow through the resource borrow (`deposit`). -/
  | field
  /-- The resource borrow written whole (`replace`). -/
  | whole
  /-- A field reborrow with the read saved in a local, guarding a throw
  that retires the loans itself (`withdraw`). -/
  | saved
  deriving Repr, BEq

/-- A bracket body, read off its tree. -/
structure StorePlan where
  shape : StoreShape
  twin : StoreTwin
  fuelOuter : Nat
  /-- The field reborrow's binding fuel; unused by the whole shape. -/
  fuelInner : Nat
  /-- The saved read's binding fuel; used by the saved shape alone. -/
  fuelSaved : Nat := 0
  /-- Events of the inner block, after the two bindings. -/
  inner : List Event
  /-- The saved shape's guard. -/
  guard : Option Guard := none

private def natLitOf? (e : Lean.Expr) : Option Nat := e.nat?

/-- What the field bracket's head — the marker, the two lets, the global
borrow site, the two-step reborrow — says about the body. -/
private structure BracketHead where
  fuelOuter : Nat
  fuelInner : Nat
  namespaceIndex : Nat
  typeIndex : Nat
  /-- The field path, each step's struct and field index. -/
  path : List (Nat × Nat)
  /-- The local the resource borrow is bound to. -/
  holder : Nat
  /-- The inner block, after the two bindings. -/
  inner : Lean.Expr

/-- A closed row of nominal field steps, as (struct, field) pairs. -/
private partial def fieldSteps? (e : Lean.Expr) : Option (List (Nat × Nat)) := do
  if e.isAppOf ``List.nil then return []
  guard (e.isAppOfArity ``List.cons 3)
  let step := e.getArg! 1
  guard (step.isAppOfArity ``LeanerIR.SemanticOperations.NominalFieldStep.mk 3)
  guard ((step.getArg! 1).isAppOf ``Option.none)
  let structIndex ← natLitOf? ((step.getArg! 0).getArg! 1)
  let index ← natLitOf? (step.getArg! 2)
  let rest ← fieldSteps? (e.getArg! 2)
  pure ((structIndex, index) :: rest)

private def isInteger : SpecTypes.FieldRep → Bool
  | .int _ _ => true
  | _ => false

private def isNominal : SpecTypes.FieldRep → Bool
  | .nominal _ => true
  | _ => false

/-- The twin metadata of the structs along a field path: each step's twin
and field row, its focused field a nested twin until the last, which is
the integer the bracket reads and writes. -/
private def pathSteps? (namespaceIndex : Nat) (raw : List (Nat × Nat))
    (twinInfoOf : Nat → Nat → Option (Name × Array (String × SpecTypes.FieldRep))) :
    Option (List PathStep) := do
  let steps ← raw.mapM fun (structIndex, focus) => do
    let (twin, fields) ← twinInfoOf namespaceIndex structIndex
    guard (focus < fields.size)
    pure ({ twin, structIndex, fields, focus } : PathStep)
  let leaf ← steps.getLast?
  guard (isInteger leaf.fields[leaf.focus]!.2)
  guard (steps.dropLast.all fun step => isNominal step.fields[step.focus]!.2)
  pure steps

/-- Recognize `endLoan [0,1] (values [let v2 := (let vH := borrow<T>(local0);
reborrow localH.[f*]) inner])` and read off its indices. -/
private def bracketHead? (tree : Lean.Expr) : Option BracketHead := do
  guard (tree.isAppOfArity ``nativeReferenceOperation 2)
  let operation := tree.getArg! 0
  guard (operation.isAppOfArity ``ReferenceLocationOperation.endLoan 1)
  let operands := tree.getArg! 1
  guard (operands.isAppOfArity ``valuesCons 2)
  guard ((operands.getArg! 1).isConstOf ``valuesNil)
  let outerLet := operands.getArg! 0
  guard (outerLet.isAppOfArity ``letNativeValue 3)
  let outerBinder := outerLet.getArg! 0
  guard (outerBinder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
  let fuelOuter ← natLitOf? (outerBinder.getArg! 0)
  guard (fuelOuter > 0)
  let innerLet := outerLet.getArg! 1
  guard (innerLet.isAppOfArity ``letNativeValue 3)
  let innerBinder := innerLet.getArg! 0
  guard (innerBinder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
  let fuelInner ← natLitOf? (innerBinder.getArg! 0)
  guard (fuelInner > 0)
  let borrow := innerLet.getArg! 1
  guard (borrow.isAppOfArity ``nativeGlobalOperation 2)
  let site := borrow.getArg! 0
  guard (site.isAppOfArity ``GlobalLocationOperation.borrow 1)
  let siteRecord := site.getArg! 0
  guard (siteRecord.isAppOfArity ``BorrowLocation.mk 4)
  let resource := siteRecord.getArg! 0
  guard (resource.isAppOfArity ``ResourceLocation.mk 2)
  let namespaceIndex ← natLitOf? ((resource.getArg! 0).getArg! 0)
  let typeIndex ← natLitOf? ((resource.getArg! 1).getArg! 0)
  let reborrow := innerLet.getArg! 2
  guard (reborrow.isAppOfArity ``nativeDerefLocalBorrowOperation 2)
  let reborrowRecord := reborrow.getArg! 0
  guard (reborrowRecord.isAppOfArity ``DerefLocalBorrowOperation.mk 5)
  let holder ← natLeaf? (reborrowRecord.getArg! 0)
  let path ← fieldSteps? (reborrowRecord.getArg! 1)
  guard !path.isEmpty
  pure {
    fuelOuter := fuelOuter
    fuelInner := fuelInner
    namespaceIndex := namespaceIndex
    typeIndex := typeIndex
    path := path
    holder := holder
    inner := outerLet.getArg! 2 }

/-- The field bracket over four locals: the holder in local 3, the inner
block a scalar segment. -/
private def fieldPlan? (body : Lean.Expr) (parameterCount localCount : Nat)
    (twinOf : Nat → Nat → Option (Lean.Name × Nat))
    (twinInfoOf : Nat → Nat → Option (Name × Array (String × SpecTypes.FieldRep))) :
    Option StorePlan := do
  guard (parameterCount == 2 && localCount == 4)
  let tree := body.bindingBody!
  guard !tree.hasLooseBVars
  let head ← bracketHead? tree
  guard (head.holder == 3)
  let (twinName, familyStruct) ← twinOf head.namespaceIndex head.typeIndex
  guard (head.path.head?.map (·.1) == some familyStruct)
  let path ← pathSteps? head.namespaceIndex head.path twinInfoOf
  let inner ← linearize head.inner
  pure {
    shape := .field
    twin := {
      twin := twinName
      namespaceIndex := head.namespaceIndex
      typeIndex := head.typeIndex
      path := path }
    fuelOuter := head.fuelOuter
    fuelInner := head.fuelInner
    inner := inner }

/-- A local read `localVar ⟨index⟩`. -/
private def isLocalRead (e : Lean.Expr) (index : Nat) : Bool :=
  e.isAppOfArity ``localVar 1 && natLeaf? (e.getArg! 0) == some index

/-- The one-element operand row `values [head]`. -/
private def singleOperand? (e : Lean.Expr) : Option Lean.Expr := do
  guard (e.isAppOfArity ``valuesCons 2)
  guard ((e.getArg! 1).isConstOf ``valuesNil)
  pure (e.getArg! 0)

/-- The saved bracket over five locals: the holder in local 4, the read
saved in local 3, a branch on it guarding a throw that retires the loans,
then the checked write. -/
private def savedPlan? (body : Lean.Expr) (parameterCount localCount : Nat)
    (twinOf : Nat → Nat → Option (Lean.Name × Nat))
    (twinInfoOf : Nat → Nat → Option (Name × Array (String × SpecTypes.FieldRep))) :
    Option StorePlan := do
  guard (parameterCount == 2 && localCount == 5)
  let tree := body.bindingBody!
  guard !tree.hasLooseBVars
  let head ← bracketHead? tree
  guard (head.holder == 4)
  let (twinName, familyStruct) ← twinOf head.namespaceIndex head.typeIndex
  guard (head.path.head?.map (·.1) == some familyStruct)
  let path ← pathSteps? head.namespaceIndex head.path twinInfoOf
  /- `let v3 := *v2; { if v3 < v1 then throw(endLoan …); *v2 := *v2 - v1 }` -/
  let savedLet := head.inner
  guard (savedLet.isAppOfArity ``letNativeValue 3)
  let savedBinder := savedLet.getArg! 0
  guard (savedBinder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
  let fuelSaved ← natLitOf? (savedBinder.getArg! 0)
  guard (fuelSaved > 0)
  guard (natLeaf? (savedBinder.getArg! 1) == some 3)
  let savedRead := savedLet.getArg! 1
  guard (savedRead.isAppOfArity ``nativeReferenceOperation 2)
  guard ((savedRead.getArg! 0).isConstOf ``ReferenceLocationOperation.dereference)
  guard (isLocalRead (← singleOperand? (savedRead.getArg! 1)) 2)
  let block := savedLet.getArg! 2
  guard (block.isAppOfArity ``blockUnit 1)
  let statements := block.getArg! 0
  guard (statements.isAppOfArity ``statementsCons 2)
  let branch := statements.getArg! 0
  let restStatements := statements.getArg! 1
  guard (restStatements.isAppOfArity ``statementsCons 2)
  guard ((restStatements.getArg! 1).isConstOf ``statementsNil)
  let write := restStatements.getArg! 0
  let (guardShape, _, marker) ← guardBranch? branch
  let savedOperand : Operand → Bool
    | .local index => index == 1 || index == 3
    | .literal _ => true
  guard (savedOperand guardShape.left && savedOperand guardShape.right)
  guard (marker.isAppOfArity ``nativeReferenceOperation 2)
  guard ((marker.getArg! 0).isAppOfArity ``ReferenceLocationOperation.endLoan 1)
  guard ((← singleOperand? (marker.getArg! 1)).isAppOfArity ``value 1)
  let inner ← linearize write
  guard (inner == [.read, .read, .dereference, .read, .checked 64 false true, .mutate 2])
  pure {
    shape := .saved
    twin := {
      twin := twinName
      namespaceIndex := head.namespaceIndex
      typeIndex := head.typeIndex
      path := path }
    fuelOuter := head.fuelOuter
    fuelInner := head.fuelInner
    fuelSaved := fuelSaved
    inner := inner
    guard := some guardShape }

/-- The struct handle a resolved constructor builds. -/
private def constructorStruct? (e : Lean.Expr) : Option Nat := do
  guard (e.isAppOfArity ``nativeOperation 2)
  let evaluate := e.getArg! 0
  guard (evaluate.isAppOfArity ``NominalConstructor.evaluate? 1)
  let constructor := evaluate.getArg! 0
  guard (constructor.isAppOfArity ``NominalConstructor.mk 3)
  natLitOf? ((constructor.getArg! 0).getArg! 1)

/-- The selections `local2.[f*]` down the given structs, innermost struct
first, over a dereference of local 2. -/
private def dereferencedPath? (e : Lean.Expr) (structs : List Nat) : Option Unit := do
  match structs with
  | [] =>
      guard (e.isAppOfArity ``nativeReferenceOperation 2)
      guard ((e.getArg! 0).isConstOf ``ReferenceLocationOperation.dereference)
      let operand ← singleOperandOf? (e.getArg! 1)
      guard (operand.isAppOfArity ``localVar 1)
      guard (((operand.getArg! 0).getArg! 0).nat? == some 2)
  | innermost :: outer =>
      guard (e.isAppOfArity ``nativeOperation 2)
      let evaluator := e.getArg! 0
      guard (evaluator.isAppOfArity ``NominalFieldLocation.evaluateSelect? 1)
      let location := evaluator.getArg! 0
      guard (location.isAppOfArity ``NominalFieldLocation.mk 3)
      guard (natLitOf? ((location.getArg! 0).getArg! 1) == some innermost)
      guard ((location.getArg! 1).isAppOf ``Option.none)
      guard (natLitOf? (location.getArg! 2) == some 0)
      let operand ← singleOperandOf? (e.getArg! 1)
      dereferencedPath? operand outer

/-- Recognize `endLoan [0] (values [let v2 := borrow<T>(local0);
[*v2 := Outer(Inner(leaf))]])`: the whole resource rebuilt around a leaf
and written through the resource borrow.  The leaf is the integer
argument in local 1, or a checked operation of the resource's own leaf,
read back through the borrow, with that argument. -/
private def wholePlan? (body : Lean.Expr) (parameterCount localCount : Nat)
    (twinOf : Nat → Nat → Option (Lean.Name × Nat))
    (twinInfoOf : Nat → Nat → Option (Name × Array (String × SpecTypes.FieldRep))) :
    Option StorePlan := do
  guard (parameterCount == 2 && localCount == 3)
  let tree := body.bindingBody!
  guard !tree.hasLooseBVars
  guard (tree.isAppOfArity ``nativeReferenceOperation 2)
  let operation := tree.getArg! 0
  guard (operation.isAppOfArity ``ReferenceLocationOperation.endLoan 1)
  let operands := tree.getArg! 1
  guard (operands.isAppOfArity ``valuesCons 2)
  guard ((operands.getArg! 1).isConstOf ``valuesNil)
  let outerLet := operands.getArg! 0
  guard (outerLet.isAppOfArity ``letNativeValue 3)
  let outerBinder := outerLet.getArg! 0
  guard (outerBinder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
  let fuelOuter ← natLitOf? (outerBinder.getArg! 0)
  guard (fuelOuter > 0)
  let borrow := outerLet.getArg! 1
  guard (borrow.isAppOfArity ``nativeGlobalOperation 2)
  let site := borrow.getArg! 0
  guard (site.isAppOfArity ``GlobalLocationOperation.borrow 1)
  let siteRecord := site.getArg! 0
  guard (siteRecord.isAppOfArity ``BorrowLocation.mk 4)
  let resource := siteRecord.getArg! 0
  guard (resource.isAppOfArity ``ResourceLocation.mk 2)
  let namespaceIndex ← natLitOf? ((resource.getArg! 0).getArg! 0)
  let typeIndex ← natLitOf? ((resource.getArg! 1).getArg! 0)
  /- The inner block is exactly the write of `Outer(Inner(local1))` through
  local 2, so the written leaf is the argument. -/
  let block := outerLet.getArg! 2
  guard (block.isAppOfArity ``blockUnit 1)
  let statements := block.getArg! 0
  guard (statements.isAppOfArity ``statementsCons 2)
  guard ((statements.getArg! 1).isConstOf ``statementsNil)
  let write := statements.getArg! 0
  guard (write.isAppOfArity ``nativeReferenceOperation 2)
  guard ((write.getArg! 0).isConstOf ``ReferenceLocationOperation.mutate)
  let writeOperands := write.getArg! 1
  guard (writeOperands.isAppOfArity ``valuesCons 2)
  let target := writeOperands.getArg! 0
  guard (target.isAppOfArity ``localVar 1)
  guard (((target.getArg! 0).getArg! 0).nat? == some 2)
  let valueOperands := writeOperands.getArg! 1
  guard (valueOperands.isAppOfArity ``valuesCons 2)
  guard ((valueOperands.getArg! 1).isConstOf ``valuesNil)
  /- The constructed value is a chain of single-field structs down to the
  argument in local 1: a path with no siblings, of any depth. -/
  let rec chain (construct : Lean.Expr) (fuel : Nat) : Option (List Nat) := do
    match fuel with
    | 0 => none
    | fuel + 1 =>
      let structIndex ← constructorStruct? construct
      let arguments := construct.getArg! 1
      guard (arguments.isAppOfArity ``valuesCons 2)
      guard ((arguments.getArg! 1).isConstOf ``valuesNil)
      let argument := arguments.getArg! 0
      if argument.isAppOfArity ``localVar 1 then
        guard (((argument.getArg! 0).getArg! 0).nat? == some 1)
        pure [structIndex]
      else if argument.isAppOfArity ``nativePrimitiveOperation 2 then
        let operationHead ← (argument.getArg! 0).getAppFn.constName?
        guard (operationHead == ``PrimitiveLocationOperation.checkedAdd ||
          operationHead == ``PrimitiveLocationOperation.checkedSubtract)
        pure [structIndex]
      else
        pure (structIndex :: (← chain argument fuel))
  let structs ← chain (valueOperands.getArg! 0) 8
  /- A computed leaf reads the resource's leaf back down the same path. -/
  let rec leafOf (construct : Lean.Expr) (fuel : Nat) : Option Lean.Expr := do
    match fuel with
    | 0 => none
    | fuel + 1 =>
      let argument := (construct.getArg! 1).getArg! 0
      if argument.isAppOfArity ``nativeOperation 2 then leafOf argument fuel
      else pure argument
  let leaf ← leafOf (valueOperands.getArg! 0) 8
  if leaf.isAppOfArity ``nativePrimitiveOperation 2 then
    let operands := leaf.getArg! 1
    guard (operands.isAppOfArity ``valuesCons 2)
    let rest := operands.getArg! 1
    guard (rest.isAppOfArity ``valuesCons 2)
    guard ((rest.getArg! 1).isConstOf ``valuesNil)
    let argument := rest.getArg! 0
    guard (argument.isAppOfArity ``localVar 1)
    guard (((argument.getArg! 0).getArg! 0).nat? == some 1)
    dereferencedPath? (operands.getArg! 0) structs.reverse
  let outerStruct ← structs.head?
  let (twinName, familyStruct) ← twinOf namespaceIndex typeIndex
  guard (familyStruct == outerStruct)
  let path ← pathSteps? namespaceIndex (structs.map fun structIndex => (structIndex, 0))
    twinInfoOf
  guard (path.all fun step => step.fields.size == 1)
  let inner ← linearize block
  pure {
    shape := .whole
    twin := {
      twin := twinName
      namespaceIndex := namespaceIndex
      typeIndex := typeIndex
      path := path }
    fuelOuter := fuelOuter
    fuelInner := 0
    inner := inner }

/-- Recognize either bracket shape. -/
def storePlan? (body : Lean.Expr) (parameterCount localCount : Nat)
    (twinOf : Nat → Nat → Option (Lean.Name × Nat))
    (twinInfoOf : Nat → Nat → Option (Name × Array (String × SpecTypes.FieldRep))) :
    Option StorePlan :=
  fieldPlan? body parameterCount localCount twinOf twinInfoOf <|>
    wholePlan? body parameterCount localCount twinOf twinInfoOf <|>
    savedPlan? body parameterCount localCount twinOf twinInfoOf

/-! ### The path, spelled

The script names the resource's parts by destructuring it down the path:
siblings get names, the focused integer its value and certificate.  The
runtime image of the path is then a literal list of focus steps, each
sibling erased from its name, and every row the bracket laws state over
`focusValue` is read at that list. -/

/-- The sibling at a field of a step, as the destructuring names it. -/
private def siblingIdent (depth index : Nat) : Ident :=
  mkIdent (Name.mkSimple s!"sibling{depth}_{index}")

/-- The path's focus steps, siblings erased from their names. -/
private def stepsTerm (namespaceIndex : Nat) (path : List PathStep) :
    CommandElabM Term := do
  let steps ← path.toArray.mapIdxM fun depth step => do
    let erased ← step.fields.mapIdxM fun index (_, rep) =>
      rep.eraseSyntax (siblingIdent depth index)
    let before := erased.extract 0 step.focus
    let after := erased.extract (step.focus + 1) erased.size
    `(term| (⟨⟨⟨$(Syntax.mkNatLit namespaceIndex)⟩,
        $(Syntax.mkNatLit step.structIndex)⟩, #[$before,*], #[$after,*]⟩ :
      LeanerIR.SemanticOperations.FocusStep))
  `(term| ([$steps,*] : List LeanerIR.SemanticOperations.FocusStep))

private def patternLo (pattern : TSyntax `rcasesPat) :
    CommandElabM (TSyntax ``Lean.Parser.Tactic.rcasesPatLo) :=
  `(Lean.Parser.Tactic.rcasesPatLo| $pattern:rcasesPat)

/-- The destructuring of the resource down the path. -/
private partial def resourcePattern (path : List PathStep) (depth : Nat := 0) :
    CommandElabM (TSyntax `rcasesPat) := do
  match path with
  | [] =>
      let value ← `(rcasesPat| $(n "fieldVal"):ident)
      let fits ← `(rcasesPat| $(n "fieldFits"):ident)
      `(rcasesPat| ⟨$(← patternLo value), $(← patternLo fits)⟩)
  | step :: rest =>
      let inner ← resourcePattern rest (depth + 1)
      let parts ← step.fields.mapIdxM fun index _ =>
        if index == step.focus then patternLo inner
        else do patternLo (← `(rcasesPat| $(siblingIdent depth index):ident))
      `(rcasesPat| ⟨$parts,*⟩)

/-- The typed resource down the path, the focused integer replaced by
`leaf`. -/
private partial def typedTerm (path : List PathStep) (leaf : Term) (depth : Nat := 0) :
    CommandElabM Term := do
  match path with
  | [] => pure leaf
  | step :: rest =>
      let inner ← typedTerm rest leaf (depth + 1)
      let parts ← step.fields.mapIdxM fun index _ =>
        if index == step.focus then pure inner
        else pure (siblingIdent depth index : Term)
      `(term| ⟨$parts,*⟩)

/-- The runtime image of a sibling-free path around `leaf`, as a literal:
what a constructor chain evaluates to. -/
private def nominalTerm (namespaceIndex : Nat) (path : List PathStep) (leaf : Term) :
    CommandElabM Term :=
  path.foldrM (init := leaf) fun step inner =>
    `(term| (.nominal ⟨⟨$(Syntax.mkNatLit namespaceIndex)⟩,
        $(Syntax.mkNatLit step.structIndex)⟩ none #[$inner]))

/-- A one-parameter field bracket whose inner block is one call through
the field borrow (`bump_counter`): `let value := &mut R[addr].f; f(value)`. -/
structure FieldCallPlan where
  twin : StoreTwin
  fuelOuter : Nat
  fuelInner : Nat
  site : CallSite

def fieldCallPlan? (body : Lean.Expr) (parameterCount localCount : Nat)
    (twinOf : Nat → Nat → Option (Lean.Name × Nat))
    (twinInfoOf : Nat → Nat → Option (Name × Array (String × SpecTypes.FieldRep))) :
    Option FieldCallPlan := do
  guard (parameterCount == 1 && localCount == 3)
  let head ← bracketHead? body.bindingBody!
  guard (head.holder == 2)
  let (twinName, familyStruct) ← twinOf head.namespaceIndex head.typeIndex
  guard (head.path.head?.map (·.1) == some familyStruct)
  let path ← pathSteps? head.namespaceIndex head.path twinInfoOf
  guard (head.inner.isAppOfArity ``blockUnit 1)
  let statements := head.inner.getArg! 0
  guard (statements.isAppOfArity ``statementsCons 2)
  guard ((statements.getArg! 1).isConstOf ``statementsNil)
  let (relation, reborrows, _) ← callStatement? (statements.getArg! 0) 1
  guard (reborrows == 1)
  let callee ← calleeNames? relation
  pure {
    twin := {
      twin := twinName
      namespaceIndex := head.namespaceIndex
      typeIndex := head.typeIndex
      path := path }
    fuelOuter := head.fuelOuter
    fuelInner := head.fuelInner
    site := { callee, reborrows } }

/-- The script for the one-parameter field bracket around a call: the
bracket law with a throw-aware inner block, the focused call statement
consuming the callee's contract, and the exit over the callee's abstract
loan registry with the lookups its discipline preserves. -/
def fieldCallScript (caller : CallerNames) (plan : FieldCallPlan) :
    CommandElabM (TSyntax `tactic) := do
  let twin := plan.twin
  let nsLit := Syntax.mkNatLit twin.namespaceIndex
  let tyLit := Syntax.mkNatLit twin.typeIndex
  let steps ← stepsTerm twin.namespaceIndex twin.path
  let callee := plan.site.callee
  let calleeContract := mkIdent callee.contract
  let calleeTyped := mkIdent callee.typedContract
  let calleeRaw := mkIdent callee.rawContract
  let calleeCodec := mkIdent callee.argumentsCodec
  let eraseName := mkIdent (twin.twin ++ `erase)
  let getName := mkIdent (twin.twin ++ `get)
  let readName := mkIdent (twin.twin ++ `read)
  let keyName := mkIdent (twin.twin ++ `key)
  let family := twin.twin.getString!
  let address := caller.parameters.getD 0 "addr"
  let pattern ← resourcePattern twin.path
  let leafPresent ← `(term| .integer $(n "fieldVal"):ident)
  let present ← `(term| LeanerIR.SemanticOperations.focusValue $steps $leafPresent)
  let keyTerm ← `(term| LeanerIR.SemanticOperations.globalKey ⟨$nsLit⟩ ⟨$tyLit⟩
    (.address $(n "addr"):ident))
  let keyEq ← `(tactic| rw [show $keyTerm = ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩ from rfl,
    $(n "lookupEq"):ident, $(n "resourceEq"):ident])
  let stateNext ← `(term| ($(n "initial"):ident).nextLoan + 1 + 1)
  let leafWritten ← `(term| .integer $(n "slot0"):ident)
  let resource ← `(term| LeanerIR.SemanticOperations.focusValue $steps $leafWritten)
  let leafTyped ← `(term| ⟨$(n "slot0"):ident, $(n "fitsSum"):ident⟩)
  let typed ← typedTerm twin.path leafTyped
  let closingPrefix ← `(tactic|
    simp only [$(caller.rawContract):term, $(caller.resultsCodec):term,
      $getName:ident, $readName:ident, $keyName:ident])
  let thrownTail ← seq <| (← scalarThrownPrefix) ++ #[closingPrefix,
    ← `(tactic| leaner_certified_close!)]
  let calleeSetup ← calleeSatFact caller callee
  let entry ← `(tactic|
    (apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.address]
     rw [show LeanerIR.SemanticOperations.initialLocals 3 #[.address $(n "addr"):ident] =
         #[some (.address $(n "addr"):ident), none, none] by
         simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
       show LeanerIR.SemanticOperations.parameterLoanLocations
         #[.address $(n "addr"):ident] = #[] by
         simp [LeanerIR.SemanticOperations.parameterLoanLocations]]))
  let site ← `(tactic|
    (apply LeanerIR.Proofs.Denotation.wpRowThrow_blockUnit
     apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_consCallFocused
     case distinctZero => decide
     case distinctOne => decide
     case mutableKind => rfl
     case innerPrior => exact Nat.lt_succ_self _
     case calleeWp =>
       apply LeanerIR.Proofs.Denotation.wpFunction_of_satisfies
         $(calleeSatIdent callee)
       case permitted =>
         simp only [$calleeContract:term,
           LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
           LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
           $calleeCodec:term,
           LeanerIR.Proofs.Codec.mutable, LeanerIR.Proofs.Codec.specInt]
         have $(n "fieldRange"):ident : 0 ≤ $(n "fieldVal"):ident ∧
             $(n "fieldVal"):ident ≤ 18446744073709551615 := by
           have $(n "h"):ident := $(n "fieldFits"):ident
           simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?,
             LeanerIR.Ty.integerBounds?] at $(n "h"):ident
           omega
         refine ⟨⟨⟨$stateNext, ⟨$(n "fieldVal"):ident, ?_⟩⟩⟩, rfl,
           $stateNext, $(n "fieldVal"):ident,
           ⟨⟨⟨rfl, by omega, by omega⟩, ?_⟩, Nat.lt_succ_self _⟩, ?_⟩
         · simp [LeanerIR.IntegerValueFits,
             LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
           omega
         · intro $(n "rowLoan"):ident $(n "rowBound"):ident
           have $(n "rowAdvanced"):ident :
               $stateNext + 1 ≤ $(n "rowLoan"):ident := $(n "rowBound"):ident
           have $(n "headNe"):ident :
               (($(n "initial"):ident).nextLoan == $(n "rowLoan"):ident) = false := by
             simp only [beq_eq_false_iff_ne]
             omega
           simp only [LeanerIR.SemanticOperations.globalLoanKey?,
             LeanerIR.SemanticOperations.globalLoanKeyIn?, List.find?_cons,
             $(n "headNe"):ident]
           exact $(n "freshLoans"):ident $(n "rowLoan"):ident (by omega)
         · have $(n "headNe"):ident :
               (($(n "initial"):ident).nextLoan == $stateNext) = false := by
             simp only [beq_eq_false_iff_ne]
             omega
           simp only [LeanerIR.SemanticOperations.globalLoanKey?,
             LeanerIR.SemanticOperations.globalLoanKeyIn?, List.find?_cons,
             $(n "headNe"):ident]
           exact $(n "freshLoans"):ident $stateNext (by omega)
       case onReturn =>
         intro $(n "results0"):ident $(n "final0"):ident
           $(n "ensuresFn0"):ident $(n "frameFn0"):ident $(n "notMust0"):ident
         have $(n "ensures0"):ident := $(n "ensuresFn0"):ident
           (by exact $(n "notMust0"):ident)
         simp only [$calleeContract:term,
           LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
           LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
           at $(n "ensures0"):ident $(n "frameFn0"):ident
         obtain ⟨$(n "rowArgs"):ident, $(n "rowResult"):ident,
           $(n "rowArgsEq"):ident, $(n "rowDecodeEq"):ident,
           $(n "rowSlotLoan"):ident, $(n "rowSlotEntry"):ident,
           $(n "slot0"):ident, $(n "rowSlotPending"):ident,
           ⟨⟨⟨$(n "rowArgsEq2"):ident, $(n "rowResultsEq"):ident⟩,
             $(n "pendingEq0"):ident, $(n "rowPendResolve"):ident⟩,
             $(n "slotNonNeg0"):ident, $(n "slotMax0"):ident⟩,
           $(n "obligation0"):ident⟩ := $(n "ensures0"):ident
         rw [$(n "rowArgsEq"):ident] at $(n "rowArgsEq2"):ident
         simp only [Array.mk.injEq, List.cons.injEq,
           LeanerIR.RuntimeValue.borrow.injEq,
           LeanerIR.RuntimeValue.integer.injEq, and_true]
           at $(n "rowArgsEq2"):ident
         obtain ⟨$(n "rowLoanEq"):ident, $(n "rowEntryEq"):ident⟩ :=
           $(n "rowArgsEq2"):ident
         subst $(n "rowLoanEq"):ident $(n "rowEntryEq"):ident
         rw [LeanerIR.SemanticOperations.resolveReturnedBorrows_empty]
           at $(n "rowPendResolve"):ident
         subst $(n "rowPendResolve"):ident
         simp only [LeanerIR.Proofs.Obligation_iff] at $(n "obligation0"):ident
         obtain ⟨$(n "rowArgs2"):ident, $(n "rowArgsEq3"):ident,
           $(n "globalsEq0"):ident, $(n "discipline0"):ident⟩ := $(n "frameFn0"):ident
         refine ⟨$(n "slot0"):ident, $(n "pendingEq0"):ident, ?_⟩
         apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_nil
         leaner_row_drive
         rename_i $(n "exitFrame"):ident $(n "exitState"):ident
           $(n "retired"):ident $(n "exited"):ident
         have $(n "plainSteps"):ident :
             LeanerIR.SemanticOperations.PlainSteps $steps := by leaner_plain
         have $(n "stable0"):ident := ($(n "discipline0"):ident).2.1
         have $(n "lookupOuter"):ident :
             LeanerIR.SemanticOperations.globalLoanKeyIn?
               ($(n "final0"):ident).globalLoans ($(n "initial"):ident).nextLoan =
               some $keyTerm := by
           rw [$(n "stable0"):ident ($(n "initial"):ident).nextLoan
             (by show ($(n "initial"):ident).nextLoan < $stateNext + 1; omega)]
           exact LeanerIR.SemanticOperations.globalLoanKeyIn?_head _ _ _
         have $(n "lookupInner"):ident :
             LeanerIR.SemanticOperations.globalLoanKeyIn?
               ($(n "final0"):ident).globalLoans (($(n "initial"):ident).nextLoan + 1) =
               none := by
           rw [$(n "stable0"):ident (($(n "initial"):ident).nextLoan + 1)
             (by show ($(n "initial"):ident).nextLoan + 1 < $stateNext + 1; omega)]
           have $(n "headNe"):ident :
               (($(n "initial"):ident).nextLoan == ($(n "initial"):ident).nextLoan + 1) =
                 false := by
             simp only [beq_eq_false_iff_ne]
             omega
           simp only [LeanerIR.SemanticOperations.globalLoanKeyIn?, List.find?_cons,
             $(n "headNe"):ident]
           exact $(n "freshLoans"):ident _ (by omega)
         simp only [$(n "globalsEq0"):ident] at $(n "exited"):ident
         rw [LeanerIR.Proofs.Denotation.endLoan_evaluate_focusedFieldOne
           (steps := $steps) (plainSteps := $(n "plainSteps"):ident)
           (lookupOuter := $(n "lookupOuter"):ident)
           (lookupInner := $(n "lookupInner"):ident)] at $(n "exited"):ident
         injection $(n "exited"):ident with $(n "exitInner"):ident
         injection $(n "exitInner"):ident with $(n "exitFrameEq"):ident
           $(n "exitStateEq"):ident $(n "retiredEq"):ident
         subst $(n "exitFrame"):ident $(n "exitState"):ident $(n "retired"):ident
         refine ⟨_, _, rfl, ?_⟩
         intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finishControl?,
           LeanerIR.SemanticOperations.unpackFallthrough,
           Option.map_eq_map, Option.map_some, Option.some.injEq]
           at $(n "rowFinished"):ident
         subst $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
         rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowFree _ _ _
           ?borrowFree]
         case borrowFree =>
           intro slot mem value eq
           simp only [List.mem_toArray, List.mem_cons, List.mem_nil_iff,
             or_false] at mem
           rcases mem with $(n "rfl"):ident | $(n "rfl"):ident | $(n "rfl"):ident <;>
             cases eq <;>
             simp [LeanerIR.SemanticOperations.outermostBorrows,
               LeanerIR.SemanticOperations.borrowEntry?,
               LeanerIR.SemanticOperations.collectPruned]
         rw [show $keyTerm = ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩ from rfl]
         have $(n "fitsSum"):ident : LeanerIR.IntegerValueFits
             (.bits 64) false $(n "slot0"):ident := by
           simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?,
             LeanerIR.Ty.integerBounds?]
           omega
         have $(n "represented'"):ident : LeanerIR.FamilyRepresentation
             $eraseName ⟨$nsLit⟩ ⟨$tyLit⟩
             (LeanerIR.updateContents $(n "contents"):ident
               (.address $(n "addr"):ident)
               (some $typed))
             ((($(n "initial"):ident).globals.insert
                 ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩
                 (.loanHole ($(n "initial"):ident).nextLoan)).insert
               ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩
               $resource) :=
           LeanerIR.FamilyRepresentation.insert_over_hole (erase := $eraseName)
             $(n "represented"):ident (.address $(n "addr"):ident)
             (.loanHole ($(n "initial"):ident).nextLoan)
             $typed
         have $(n "rowDiscipline"):ident :=
           LeanerIR.Proofs.Denotation.LoanDiscipline.throughBracketCall
             $(n "initial"):ident $(n "final0"):ident
             ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩ _ _
             ((($(n "initial"):ident).globals.insert
                 ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩
                 (.loanHole ($(n "initial"):ident).nextLoan)).insert
               ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩
               $resource)
             ($(n "initial"):ident).pending
             $(n "freshLoans"):ident $(n "discipline0"):ident
         $closingPrefix:tactic
         leaner_certified_close!
       case onThrow =>
         intro $(n "rowKind"):ident $(n "rowThrown"):ident
           $(n "final0"):ident $(n "aborts0"):ident
         simp only [$calleeContract:term,
           LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
           LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
           at $(n "aborts0"):ident
         obtain ⟨$(n "rowArgs"):ident, $(n "rowArgsEq"):ident,
           $(n "rowSlotLoan"):ident, $(n "rowSlot"):ident,
           $(n "rowArgsEq2"):ident, $(n "rowObligation"):ident⟩ :=
           $(n "aborts0"):ident
         rw [$(n "rowArgsEq"):ident] at $(n "rowArgsEq2"):ident
         simp only [Array.mk.injEq, List.cons.injEq,
           LeanerIR.RuntimeValue.borrow.injEq,
           LeanerIR.RuntimeValue.integer.injEq, and_true]
           at $(n "rowArgsEq2"):ident
         obtain ⟨-, $(n "rowSlotEq"):ident⟩ := $(n "rowArgsEq2"):ident
         subst $(n "rowSlotEq"):ident
         simp only [LeanerIR.Proofs.Obligation_iff] at $(n "rowObligation"):ident
         $thrownTail:tactic))
  let law ← `(tactic|
    (apply LeanerIR.Proofs.Denotation.wpRowThrow_focusedFieldBracketOneThrow
      (steps := $steps)
     case borrowMutable => rfl
     case fieldMutable => rfl
     case present => exact $(n "presentValue"):ident))
  let vocabulary := #[
    (s!"{family}_contents", "contents"), (s!"{family}_represented", "represented"),
    (address, "addr"), ("requires_0", "present")]
  let renames ← bindVocabulary vocabulary
  `(tactic|
    ($calleeSetup:tactic
     $renames:tactic
     have $(n "lookupEq"):ident :=
       $(n "represented"):ident (.address $(n "addr"):ident)
     rw [$(n "lookupEq"):ident] at $(n "present"):ident
     obtain ⟨$(n "resource"):ident, $(n "resourceEq"):ident⟩ :=
       Option.isSome_iff_exists.mp (by simpa using $(n "present"):ident)
     obtain $pattern:rcasesPat := $(n "resource"):ident
     have $(n "presentValue"):ident : ($(n "initial"):ident).globals.lookup $keyTerm =
         some $present := by
       $keyEq:tactic
       rfl
     $entry:tactic
     $law:tactic
     $site:tactic))

/-! ## A returned reborrow chosen dynamically

`if flag then &mut *left else &mut *right`: a `Bool` selector in front of
two mutable parameters, each arm a returned reborrow of one of them.  The
selector's contract has fixed its value on each side of the preamble, so
the branch is decided by rewriting, and the chosen arm is the returned
reborrow at its local. -/

/-- What a parameter is, for the routes that read the whole signature. -/
inductive ParamKind where
  | integer
  | bool
  | borrow
  | other
  deriving Repr, BEq

structure ChoosePlan where
  /-- The lexical loans of the two arms' reborrows. -/
  lexLeft : Nat
  lexRight : Nat

def choosePlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (kinds : Array ParamKind) : Option ChoosePlan := do
  guard (parameterCount == 3 && localCount == 3 && resultCount == 1)
  guard (kinds == #[.bool, .borrow, .borrow])
  let tree := body.bindingBody!
  guard !tree.hasLooseBVars
  guard (tree.isAppOfArity ``nativeBranch 3)
  guard (isLocalRead (tree.getArg! 0) 0)
  let thenArm := tree.getArg! 1
  let elseArm := tree.getArg! 2
  guard (elseArm.isAppOfArity ``Option.some 2)
  let elseArm := elseArm.getArg! 1
  reborrowOfLocal? thenArm 1
  reborrowOfLocal? elseArm 2
  let lexLeft ← natLitOf? ((thenArm.getArg! 0).getArg! 4)
  let lexRight ← natLitOf? ((elseArm.getArg! 0).getArg! 4)
  pure { lexLeft, lexRight }

/-- The complete script for the chosen returned reborrow, from the goal
`wp_nativeFunction` leaves; the preamble has fixed the selector. -/
def chooseScript (caller : CallerNames) (_plan : ChoosePlan) :
    CommandElabM (TSyntax `tactic) := do
  let flag := mkIdent (Name.mkSimple (caller.parameters.getD 0 "flag"))
  let left := caller.parameters.getD 1 "left"
  let right := caller.parameters.getD 2 "right"
  let renames ← bindVocabulary #[(s!"{left}_loan", "leftLoan"), (left, "leftVal"),
    (s!"{left}_loan_keyFree", "leftKeyFree"), (s!"{left}_loan_bound", "leftBound"),
    (s!"{right}_loan", "rightLoan"), (right, "rightVal"),
    (s!"{right}_loan_keyFree", "rightKeyFree"), (s!"{right}_loan_bound", "rightBound")]
  let rowOf (flagLit : Term) : CommandElabM Term := `(term| (#[some (.bool $flagLit),
    some (.borrow $(n "leftLoan"):ident (.integer $(n "leftVal"):ident)),
    some (.borrow $(n "rightLoan"):ident (.integer $(n "rightVal"):ident))] :
    LeanerIR.Proofs.Denotation.Row))
  let row ← rowOf flag
  let hole ← `(term| .loanHole ($(n "initial"):ident).nextLoan)
  /- On each side the preamble has rewritten the selector to its literal
  before the arm's rows are spelled. -/
  let leftLent ← `(term| (#[some (.bool true),
    some (.borrow $(n "leftLoan"):ident $hole),
    some (.borrow $(n "rightLoan"):ident (.integer $(n "rightVal"):ident))] :
    LeanerIR.Proofs.Denotation.Row))
  let rightLent ← `(term| (#[some (.bool false),
    some (.borrow $(n "leftLoan"):ident (.integer $(n "leftVal"):ident)),
    some (.borrow $(n "rightLoan"):ident $hole)] :
    LeanerIR.Proofs.Denotation.Row))
  let arm (localLit : Nat) (flagLit : Term) (lent : Term) (holeValue : Term)
      (export_ : Ident) : CommandElabM (TSyntax `tactic) := do
    let localId := Syntax.mkNatLit localLit
    let armRow ← rowOf flagLit
    `(tactic|
      (apply LeanerIR.Proofs.Denotation.wpRowThrow_returnedReborrowAt
         (localId := ⟨$localId⟩)
       case mutableKind => rfl
       case slot => rfl
       rw [show ($armRow).set! $localId (some (.borrow _ $holeValue)) = $lent from rfl]
       intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
       simp only [LeanerIR.SemanticOperations.finishControl?,
         LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
         Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
       subst $(n "rowFinished"):ident
       simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
       rw [$export_:ident _ _ _ _ _ _ _ _ ?separateLeft ?separateRight
         ?noLeftGlobal ?noRightGlobal]
       case separateLeft => omega
       case separateRight => omega
       case noLeftGlobal => exact $(n "leftKeyFree"):ident
       case noRightGlobal => exact $(n "rightKeyFree"):ident
       simp only [$(caller.rawContract):term, $(caller.resultsCodec):term]
       leaner_certified_close!))
  let leftArm ← arm 1 (← `(term| true)) leftLent hole
    (mkIdent ``LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_returnedChoiceLeft)
  let rightArm ← arm 2 (← `(term| false)) rightLent hole
    (mkIdent ``LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_returnedChoiceRight)
  `(tactic|
    ($renames:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.mutable,
       LeanerIR.Proofs.Codec.bool]
     rw [show LeanerIR.SemanticOperations.initialLocals 3
         #[.bool $flag:ident,
           .borrow $(n "leftLoan"):ident (.integer $(n "leftVal"):ident),
           .borrow $(n "rightLoan"):ident (.integer $(n "rightVal"):ident)] = $row by
         simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
       LeanerIR.SemanticOperations.parameterLoanLocations_boolTwoBorrows]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_nativeBranch
     case conditionStable => leaner_row_stable
     leaner_row_drive
     $(← seq (← readBatch 1)):tactic
     /- The preamble fixed the selector on this side. -/
     simp only [$(n "requires_0"):ident]
     first
     | $leftArm:tactic
     | $rightArm:tactic))

/-- The natural a non-negative integer literal spells, in either of the
spellings a generated tree carries. -/
private def intLitOf? (e : Lean.Expr) : Option Nat :=
  if e.isAppOfArity ``Int.ofNat 1 then natLeaf? (e.getArg! 0)
  else if e.isAppOfArity ``OfNat.ofNat 3 then natLeaf? (e.getArg! 1)
  else none

/-! ## A caller consuming a returned pair of reborrows

`let (a, b) := f(&mut *left, &mut *right); *a := m; *b := n`: the callee's
summary names both returned loans and states that each lender's export is
its loan's hole, so the caller settles both returned borrows into its
parameters with one marker. -/

structure SetPairPlan where
  callee : CalleeNames
  /-- The tuple binding's fuel. -/
  fuel : Nat
  /-- The literals written through the two returned references. -/
  leftWritten : Nat
  rightWritten : Nat

/-- Recognize `mutate localN (value (integer n))` as one statement. -/
private def writeLocalN? (write : Lean.Expr) (slot : Nat) : Option Nat := do
  guard (write.isAppOfArity ``nativeReferenceOperation 2)
  guard ((write.getArg! 0).isConstOf ``ReferenceLocationOperation.mutate)
  let operands := write.getArg! 1
  guard (operands.isAppOfArity ``valuesCons 2)
  guard (isLocalRead (operands.getArg! 0) slot)
  let valueOperand ← singleOperand? (operands.getArg! 1)
  guard (valueOperand.isAppOfArity ``value 1)
  let literal := valueOperand.getArg! 0
  guard (literal.isAppOfArity ``LeanerIR.RuntimeValue.integer 1)
  intLitOf? (literal.getArg! 0)

/-- Recognize the pair-consuming body: the marker over a tuple-bound call
whose block writes through both returned references. -/
def setPairPlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (kinds : Array ParamKind) (statesTransfer : Name → Bool) : Option SetPairPlan := do
  guard (parameterCount == 2 && localCount == 4 && resultCount == 0)
  guard (kinds == #[.borrow, .borrow])
  let tree := body.bindingBody!
  guard (tree.isAppOfArity ``nativeReferenceOperation 2)
  guard ((tree.getArg! 0).isAppOfArity ``ReferenceLocationOperation.endLoan 1)
  let operand ← singleOperand? (tree.getArg! 1)
  guard (operand.isAppOfArity ``letNativeValue 3)
  let binder := operand.getArg! 0
  guard (binder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
  let fuel ← natLitOf? (binder.getArg! 0)
  guard (fuel > 0)
  let pattern := binder.getArg! 1
  guard (pattern.isAppOfArity ``LeanerIR.SemanticOperations.NativePattern.tuple 1)
  let elements := pattern.getArg! 0
  guard (elements.isAppOfArity ``List.cons 3)
  let second := elements.getArg! 2
  guard (second.isAppOfArity ``List.cons 3)
  guard ((second.getArg! 2).isAppOf ``List.nil)
  let variableSlot (e : Lean.Expr) : Option Nat := do
    guard (e.isAppOfArity ``LeanerIR.SemanticOperations.NativePattern.variable 1)
    natLeaf? (e.getArg! 0)
  guard ((← variableSlot (elements.getArg! 1)) == 2)
  guard ((← variableSlot (second.getArg! 1)) == 3)
  let call := operand.getArg! 1
  guard (call.isAppOfArity ``nativeCall 4)
  guard ((call.getArg! 1).isAppOf ``Option.none)
  let calleeExpr := call.getArg! 2
  let relation ← calleeRelation? calleeExpr
  let arguments := call.getArg! 3
  guard (arguments.isAppOfArity ``valuesCons 2)
  reborrowOfLocal? (arguments.getArg! 0) 0
  let argumentTail := arguments.getArg! 1
  guard (argumentTail.isAppOfArity ``valuesCons 2)
  guard ((argumentTail.getArg! 1).isConstOf ``valuesNil)
  reborrowOfLocal? (argumentTail.getArg! 0) 1
  let callee ← calleeNames? relation
  /- The callee's summary must state the transfer of both prophecies. -/
  guard (statesTransfer callee.rawContract)
  let block := operand.getArg! 2
  guard (block.isAppOfArity ``blockUnit 1)
  let statements := block.getArg! 0
  guard (statements.isAppOfArity ``statementsCons 2)
  let rest := statements.getArg! 1
  guard (rest.isAppOfArity ``statementsCons 2)
  guard ((rest.getArg! 1).isConstOf ``statementsNil)
  let leftWritten ← writeLocalN? (statements.getArg! 0) 2
  let rightWritten ← writeLocalN? (rest.getArg! 0) 3
  pure { callee, fuel, leftWritten, rightWritten }

/-- The complete script for a caller consuming a returned pair. -/
def setPairScript (caller : CallerNames) (plan : SetPairPlan) :
    CommandElabM (TSyntax `tactic) := do
  let callee := plan.callee
  let calleeContract := mkIdent callee.contract
  let calleeTyped := mkIdent callee.typedContract
  let calleeRaw := mkIdent callee.rawContract
  let calleeCodec := mkIdent callee.argumentsCodec
  let calleeResults := mkIdent callee.resultsCodec
  let stateNext ← `(term| ($(n "initial"):ident).nextLoan)
  let left := caller.parameters.getD 0 "left"
  let right := caller.parameters.getD 1 "right"
  let renames ← bindVocabulary #[(s!"{left}_loan", "leftLoan"), (left, "leftVal"),
    (s!"{left}_loan_keyFree", "leftKeyFree"), (s!"{left}_loan_bound", "leftBound"),
    (s!"{right}_loan", "rightLoan"), (right, "rightVal"),
    (s!"{right}_loan_keyFree", "rightKeyFree"), (s!"{right}_loan_bound", "rightBound")]
  let closingPrefix ← `(tactic|
    simp only [$(caller.rawContract):term, $(caller.resultsCodec):term])
  let thrownTail ← seq <| (← scalarThrownPrefix) ++ #[closingPrefix,
    ← `(tactic| leaner_certified_close!)]
  let calleeSetup ← calleeSatFact caller callee
  /- The caller after the call: the two returned loans are the callee's
  decoded result components, resting in locals 2 and 3 once bound. -/
  let fuelLit := Syntax.mkNatLit (plan.fuel - 2)
  let leftLit := Syntax.mkNumLit (toString plan.leftWritten)
  let rightLit := Syntax.mkNumLit (toString plan.rightWritten)
  let leftReturned ← `(term| ($(n "d0"):ident).loan)
  let rightReturned ← `(term| ($(n "d1"):ident).loan)
  let leftValue ← `(term| ($(n "d0"):ident).value.val)
  let rightValue ← `(term| ($(n "d1"):ident).value.val)
  let lenders ← `(term| (#[some (.borrow $(n "leftLoan"):ident (.loanHole $leftReturned)),
    some (.borrow $(n "rightLoan"):ident (.loanHole $rightReturned)), none, none] :
    LeanerIR.Proofs.Denotation.Row))
  let bound ← `(term| (#[some (.borrow $(n "leftLoan"):ident (.loanHole $leftReturned)),
    some (.borrow $(n "rightLoan"):ident (.loanHole $rightReturned)),
    some (.borrow $leftReturned (.integer $leftValue)),
    some (.borrow $rightReturned (.integer $rightValue))] :
    LeanerIR.Proofs.Denotation.Row))
  let exported ← `(term|
    { globals := ($(n "final"):ident).globals
      globalLoans := ($(n "final"):ident).globalLoans
      nextLoan := ($(n "final"):ident).nextLoan
      pending := (($(n "initial"):ident).pending.push
        ($(n "leftLoan"):ident, LeanerIR.RuntimeValue.integer $leftLit)).push
        ($(n "rightLoan"):ident, LeanerIR.RuntimeValue.integer $rightLit) })
  let write (lemma : Ident) : CommandElabM (TSyntax `tactic) :=
    `(tactic|
      (leaner_row_drive
       rename_i $(n "rowRead0"):ident $(n "rowReadEq0"):ident
       simp only [List.getElem?_toArray, List.getElem?_cons_zero,
         List.getElem?_cons_succ, Option.join_some, Option.some.injEq]
         at $(n "rowReadEq0"):ident
       subst $(n "rowReadEq0"):ident
       constructor
       case' right =>
         intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
           $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident
         rw [$lemma:ident _ _ _ _ _ _ _ _ (by omega) (by omega) (by omega)]
           at $(n "rowEq"):ident
         injection $(n "rowEq"):ident with $(n "rowInner"):ident
         injection $(n "rowInner"):ident
       case' left =>
         intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
           $(n "rowValueOut"):ident $(n "rowEq"):ident
         rw [$lemma:ident _ _ _ _ _ _ _ _ (by omega) (by omega) (by omega)]
           at $(n "rowEq"):ident
         injection $(n "rowEq"):ident with $(n "rowInner"):ident
         injection $(n "rowInner"):ident with $(n "rowFrameEq"):ident
           $(n "rowStateEq"):ident $(n "rowValueEq"):ident
         injection $(n "rowFrameEq"):ident with $(n "rowRowEq"):ident
           $(n "rowActiveEq"):ident $(n "rowLocationsEq"):ident
         subst $(n "rowRowEq"):ident $(n "rowStateEq"):ident $(n "rowValueEq"):ident))
  let leftWrite ← write (mkIdent ``LeanerIR.Proofs.Denotation.mutate_evaluate_returnedPairLeft)
  let rightWrite ← write (mkIdent ``LeanerIR.Proofs.Denotation.mutate_evaluate_returnedPairRight)
  let continuation ← `(tactic|
    (leaner_row_drive
     rename_i $(n "boundFrame"):ident $(n "boundEq"):ident
     rw [LeanerIR.Proofs.Denotation.bindTuple_rowFrame $fuelLit _ _ _ _ (by simp)]
       at $(n "boundEq"):ident
     cases Option.some.inj $(n "boundEq"):ident
     refine ⟨_, rfl, ?_⟩
     rw [show (($lenders).set! 2 (some (.borrow $leftReturned (.integer $leftValue)))).set! 3
         (some (.borrow $rightReturned (.integer $rightValue))) = $bound from rfl]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_blockUnit
     apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_cons
     apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow
     case stable => leaner_row_stable
     $leftWrite:tactic
     leaner_row_head
     apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_cons
     apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow
     case stable => leaner_row_stable
     $rightWrite:tactic
     leaner_row_drive
     rename_i $(n "nilResult"):ident $(n "nilStep"):ident
     simp only [LeanerIR.Proofs.Denotation.statementsNil] at $(n "nilStep"):ident
     subst $(n "nilStep"):ident
     refine ⟨_, _, rfl, ?_⟩
     intro $(n "exitFrame"):ident $(n "exitState"):ident
       $(n "retired"):ident $(n "exited"):ident
     rw [LeanerIR.Proofs.Denotation.endLoan_evaluate_returnedPair _ _ _ _ _ _ _ _
       (by omega) (by omega) (by omega) (by omega) (by omega)] at $(n "exited"):ident
     injection $(n "exited"):ident with $(n "exitInner"):ident
     injection $(n "exitInner"):ident with $(n "exitFrameEq"):ident
       $(n "exitStateEq"):ident $(n "retiredEq"):ident
     subst $(n "exitFrame"):ident $(n "exitState"):ident $(n "retired"):ident
     refine ⟨_, _, rfl, ?_⟩
     intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finishControl?,
       LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
       Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
     subst $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
     rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_twoIntegersUnits
       _ _ _ _ _ _ _ ?noLeftGlobal ?noRightGlobal]
     case noLeftGlobal =>
       rw [($(n "discipline"):ident).2.1 $(n "leftLoan"):ident
         (by show $(n "leftLoan"):ident < ($(n "initial"):ident).nextLoan + 1 + 1; omega)]
       exact $(n "leftKeyFree"):ident
     case noRightGlobal =>
       rw [($(n "discipline"):ident).2.1 $(n "rightLoan"):ident
         (by show $(n "rightLoan"):ident < ($(n "initial"):ident).nextLoan + 1 + 1; omega)]
       exact $(n "rightKeyFree"):ident
     $closingPrefix:tactic
     have $(n "rowDiscipline"):ident :=
       (LeanerIR.SemanticOperations.LoanDiscipline.of_eq
         (initial := $(n "initial"):ident)
         (final := { $(n "initial"):ident with
           nextLoan := ($(n "initial"):ident).nextLoan + 1 + 1 }) rfl
         (Nat.le_succ_of_le (Nat.le_succ _))).trans
       (($(n "discipline"):ident).trans
         (LeanerIR.SemanticOperations.LoanDiscipline.of_eq
           (final := $exported) rfl (Nat.le_refl _)))
     leaner_certified_close!))
  `(tactic|
    ($calleeSetup:tactic
     $renames:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.mutable]
     rw [show LeanerIR.SemanticOperations.initialLocals 4
         #[.borrow $(n "leftLoan"):ident (.integer $(n "leftVal"):ident),
           .borrow $(n "rightLoan"):ident (.integer $(n "rightVal"):ident)] =
         #[some (.borrow $(n "leftLoan"):ident (.integer $(n "leftVal"):ident)),
           some (.borrow $(n "rightLoan"):ident (.integer $(n "rightVal"):ident)),
           none, none] by
         simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
       LeanerIR.SemanticOperations.parameterLoanLocations_twoBorrows]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_endLoanOver
     apply LeanerIR.Proofs.Denotation.wpRowThrow_letNativeValueThrow
     apply LeanerIR.Proofs.Denotation.wpRowThrow_callReturnedPair
     case mutableKind => rfl
     case distinctLexical => decide
     case zeroFirst => rfl
     case oneSecond => rfl
     case leftPrior => exact $(n "leftBound"):ident
     case rightPrior => exact $(n "rightBound"):ident
     case calleeWp =>
       apply LeanerIR.Proofs.Denotation.wpFunction_of_satisfies
         $(calleeSatIdent callee)
       case permitted =>
         simp only [$calleeContract:term,
           LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
           LeanerIR.Proofs.Contract.typed, $calleeRaw:term, $calleeCodec:term,
           LeanerIR.Proofs.Codec.mutable, LeanerIR.Proofs.Codec.specInt]
         refine ⟨⟨⟨$stateNext, ⟨$(n "leftVal"):ident, ?_⟩⟩,
             ⟨$stateNext + 1, ⟨$(n "rightVal"):ident, ?_⟩⟩⟩, rfl,
           $stateNext, $(n "leftVal"):ident, $stateNext + 1, $(n "rightVal"):ident,
           ⟨⟨⟨⟨⟨⟨⟨rfl, by omega, by omega⟩, by omega, by omega⟩, ?_⟩, by omega⟩, ?_⟩,
             by omega⟩, ?_⟩, by omega⟩
         · simp [LeanerIR.IntegerValueFits,
             LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
           omega
         · simp [LeanerIR.IntegerValueFits,
             LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
           omega
         · intro $(n "rowLoan"):ident $(n "rowBound"):ident
           have $(n "rowAdvanced"):ident :
               $stateNext + 1 + 1 ≤ $(n "rowLoan"):ident := $(n "rowBound"):ident
           exact $(n "freshLoans"):ident $(n "rowLoan"):ident (by omega)
         · simp only [LeanerIR.SemanticOperations.globalLoanKey?]
           exact $(n "freshLoans"):ident $stateNext (by omega)
         · simp only [LeanerIR.SemanticOperations.globalLoanKey?]
           exact $(n "freshLoans"):ident ($stateNext + 1) (by omega)
       case onReturn =>
         intro $(n "results"):ident $(n "final"):ident $(n "ensuresFn"):ident
           $(n "frameFn"):ident $(n "notMust"):ident
         have $(n "ensures"):ident := $(n "ensuresFn"):ident
           (by exact $(n "notMust"):ident)
         simp only [$calleeContract:term,
           LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
           LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
           at $(n "ensures"):ident $(n "frameFn"):ident
         obtain ⟨$(n "rowArgs"):ident, $(n "rowResult"):ident,
           $(n "rowArgsEq"):ident, $(n "rowDecodeEq"):ident,
           $(n "rowLeftLoan"):ident, $(n "rowLeftEntry"):ident,
           $(n "leftSlot"):ident, $(n "rowLeftPending"):ident,
           $(n "rowRightLoan"):ident, $(n "rowRightEntry"):ident,
           $(n "rightSlot"):ident, $(n "rowRightPending"):ident,
           $(n "tupleValue"):ident, $(n "leftReturned"):ident, $(n "leftValue"):ident,
           $(n "rightReturned"):ident, $(n "rightValue"):ident,
           ⟨⟨⟨⟨⟨⟨⟨$(n "rowArgsEq2"):ident, $(n "rowResultsEq"):ident⟩,
             $(n "tupleEq"):ident⟩,
             ⟨⟨⟨⟨⟨⟨⟨⟨⟨$(n "pendingEq"):ident, $(n "leftResolve"):ident⟩,
               $(n "leftTransfer"):ident⟩, $(n "leftFreshLo"):ident⟩,
               $(n "leftFreshHi"):ident⟩, $(n "rightResolve"):ident⟩,
               $(n "rightTransfer"):ident⟩, $(n "rightFreshLo"):ident⟩,
               $(n "rightFreshHi"):ident⟩, $(n "returnedSeparate"):ident⟩⟩,
             $(n "leftValueNonNeg"):ident, $(n "leftValueMax"):ident⟩,
             $(n "rightValueNonNeg"):ident, $(n "rightValueMax"):ident⟩,
             $(n "leftNonNeg"):ident, $(n "leftMax"):ident⟩,
             $(n "rightNonNeg"):ident, $(n "rightMax"):ident⟩,
           $(n "obligation"):ident⟩ := $(n "ensures"):ident
         simp only [$calleeResults:term] at $(n "rowDecodeEq"):ident $(n "rowResultsEq"):ident
         split at $(n "rowDecodeEq"):ident
         · rename_i $(n "packed"):ident $(n "listEq"):ident
           split at $(n "rowDecodeEq"):ident
           · rename_i $(n "r0"):ident $(n "r1"):ident $(n "packedEq"):ident
             simp only [Option.bind_eq_some_iff, Option.some.injEq]
               at $(n "rowDecodeEq"):ident
             obtain ⟨$(n "d0"):ident, $(n "r0Eq"):ident, $(n "d1"):ident,
               $(n "r1Eq"):ident, $(n "rowResultEq"):ident⟩ := $(n "rowDecodeEq"):ident
             obtain ⟨$(n "c0"):ident, $(n "r0Shape"):ident, $(n "c0Eq"):ident⟩ :=
               LeanerIR.Proofs.Codec.mutable_decode?_eq_some $(n "r0Eq"):ident
             obtain ⟨$(n "c1"):ident, $(n "r1Shape"):ident, $(n "c1Eq"):ident⟩ :=
               LeanerIR.Proofs.Codec.mutable_decode?_eq_some $(n "r1Eq"):ident
             have $(n "c0Int"):ident :=
               LeanerIR.Proofs.Codec.specInt_decode?_eq_some $(n "c0Eq"):ident
             have $(n "c1Int"):ident :=
               LeanerIR.Proofs.Codec.specInt_decode?_eq_some $(n "c1Eq"):ident
             subst $(n "c0Int"):ident $(n "c1Int"):ident $(n "r0Shape"):ident
               $(n "r1Shape"):ident $(n "rowResultEq"):ident
             have $(n "packedShape"):ident : $(n "packed"):ident =
                 #[.borrow ($(n "d0"):ident).loan (.integer ($(n "d0"):ident).value.val),
                   .borrow ($(n "d1"):ident).loan (.integer ($(n "d1"):ident).value.val)] := by
               have $(n "packedArray"):ident := congrArg List.toArray $(n "packedEq"):ident
               simpa using $(n "packedArray"):ident
             subst $(n "packedShape"):ident
             have $(n "resultsShape"):ident : $(n "results"):ident =
                 #[.tuple #[.borrow ($(n "d0"):ident).loan (.integer ($(n "d0"):ident).value.val),
                   .borrow ($(n "d1"):ident).loan (.integer ($(n "d1"):ident).value.val)]] := by
               have $(n "listArray"):ident := congrArg List.toArray $(n "listEq"):ident
               simpa using $(n "listArray"):ident
             subst $(n "resultsShape"):ident
             simp only [LeanerIR.Proofs.Codec.mutable, LeanerIR.Proofs.Codec.specInt,
               Array.mk.injEq, List.cons.injEq, and_true] at $(n "rowResultsEq"):ident
             subst $(n "rowResultsEq"):ident
             simp only [LeanerIR.RuntimeValue.tuple.injEq, Array.mk.injEq, List.cons.injEq,
               LeanerIR.RuntimeValue.borrow.injEq, LeanerIR.RuntimeValue.integer.injEq,
               and_true] at $(n "tupleEq"):ident
             obtain ⟨⟨$(n "leftLoanEq"):ident, $(n "leftValueEq"):ident⟩,
               $(n "rightLoanEq"):ident, $(n "rightValueEq"):ident⟩ := $(n "tupleEq"):ident
             subst $(n "leftLoanEq"):ident $(n "leftValueEq"):ident
               $(n "rightLoanEq"):ident $(n "rightValueEq"):ident
             rw [$(n "rowArgsEq"):ident] at $(n "rowArgsEq2"):ident
             simp only [Array.mk.injEq, List.cons.injEq,
               LeanerIR.RuntimeValue.borrow.injEq,
               LeanerIR.RuntimeValue.integer.injEq, and_true]
               at $(n "rowArgsEq2"):ident
             obtain ⟨⟨$(n "rowLeftLoanEq"):ident, $(n "rowLeftEntryEq"):ident⟩,
               $(n "rowRightLoanEq"):ident, $(n "rowRightEntryEq"):ident⟩ :=
               $(n "rowArgsEq2"):ident
             subst $(n "rowLeftLoanEq"):ident $(n "rowLeftEntryEq"):ident
               $(n "rowRightLoanEq"):ident $(n "rowRightEntryEq"):ident
             subst $(n "leftTransfer"):ident $(n "rightTransfer"):ident
             simp only [LeanerIR.Proofs.Obligation_iff] at $(n "obligation"):ident
             obtain ⟨$(n "rowArgs2"):ident, $(n "rowArgsEq3"):ident,
               $(n "globalsEq"):ident, $(n "discipline"):ident⟩ := $(n "frameFn"):ident
             have $(n "leftFreshLoLoan"):ident :
                 ($(n "initial"):ident).nextLoan + 1 + 1 ≤ ($(n "d0"):ident).loan :=
               $(n "leftFreshLo"):ident
             have $(n "rightFreshLoLoan"):ident :
                 ($(n "initial"):ident).nextLoan + 1 + 1 ≤ ($(n "d1"):ident).loan :=
               $(n "rightFreshLo"):ident
             have $(n "monotone"):ident :
                 ($(n "initial"):ident).nextLoan + 1 + 1 ≤ ($(n "final"):ident).nextLoan :=
               ($(n "discipline"):ident).2.2
             refine ⟨_, _, _, _, rfl, $(n "returnedSeparate"):ident, by omega,
               $(n "pendingEq"):ident, ?_⟩
             $continuation:tactic
           · exact absurd $(n "rowDecodeEq"):ident (by simp)
         · exact absurd $(n "rowDecodeEq"):ident (by simp)
       case onThrow =>
         intro $(n "rowKind"):ident $(n "rowThrown"):ident
           $(n "final"):ident $(n "aborts"):ident
         simp only [$calleeContract:term,
           LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
           LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
           at $(n "aborts"):ident
         obtain ⟨$(n "rowArgs"):ident, $(n "rowArgsEq"):ident,
           $(n "rowLeftLoan"):ident, $(n "rowLeft"):ident,
           $(n "rowRightLoan"):ident, $(n "rowRight"):ident,
           $(n "rowArgsEq2"):ident, $(n "rowObligation"):ident⟩ := $(n "aborts"):ident
         rw [$(n "rowArgsEq"):ident] at $(n "rowArgsEq2"):ident
         simp only [Array.mk.injEq, List.cons.injEq,
           LeanerIR.RuntimeValue.borrow.injEq,
           LeanerIR.RuntimeValue.integer.injEq, and_true]
           at $(n "rowArgsEq2"):ident
         obtain ⟨⟨-, $(n "rowLeftEq"):ident⟩, -, $(n "rowRightEq"):ident⟩ :=
           $(n "rowArgsEq2"):ident
         subst $(n "rowLeftEq"):ident $(n "rowRightEq"):ident
         /- A callee that cannot abort closes this goal at the simp. -/
         simp only [LeanerIR.Proofs.Obligation_iff] at $(n "rowObligation"):ident
         all_goals ($thrownTail:tactic)))

/-! ## A returned pair of reborrows -/

structure ReturnPairPlan where
  lexLeft : Nat
  lexRight : Nat

/-- Recognize `(&mut *left, &mut *right)` as the whole body of a
two-borrow function. -/
def returnPairPlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (kinds : Array ParamKind) : Option ReturnPairPlan := do
  guard (parameterCount == 2 && localCount == 2 && resultCount == 1)
  guard (kinds == #[.borrow, .borrow])
  let tree := body.bindingBody!
  guard !tree.hasLooseBVars
  guard (tree.isAppOfArity ``nativePrimitiveOperation 2)
  guard ((tree.getArg! 0).isConstOf ``PrimitiveLocationOperation.tuple)
  let operands := tree.getArg! 1
  guard (operands.isAppOfArity ``valuesCons 2)
  let first := operands.getArg! 0
  let tail := operands.getArg! 1
  guard (tail.isAppOfArity ``valuesCons 2)
  guard ((tail.getArg! 1).isConstOf ``valuesNil)
  let second := tail.getArg! 0
  reborrowOfLocal? first 0
  reborrowOfLocal? second 1
  let lexLeft ← natLitOf? ((first.getArg! 0).getArg! 4)
  let lexRight ← natLitOf? ((second.getArg! 0).getArg! 4)
  pure { lexLeft, lexRight }

/-- The complete script for the returned pair: both mints, the tuple, the
export of both holes. -/
def returnPairScript (caller : CallerNames) (_plan : ReturnPairPlan) :
    CommandElabM (TSyntax `tactic) := do
  let left := caller.parameters.getD 0 "left"
  let right := caller.parameters.getD 1 "right"
  let renames ← bindVocabulary #[(s!"{left}_loan", "leftLoan"), (left, "leftVal"),
    (s!"{left}_loan_keyFree", "leftKeyFree"), (s!"{left}_loan_bound", "leftBound"),
    (s!"{right}_loan", "rightLoan"), (right, "rightVal"),
    (s!"{right}_loan_keyFree", "rightKeyFree"), (s!"{right}_loan_bound", "rightBound")]
  `(tactic|
    ($renames:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.mutable,
       LeanerIR.Proofs.Denotation.initialLocals_two,
       LeanerIR.SemanticOperations.parameterLoanLocations_twoBorrows]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_returnedPair
     case mutableKind => rfl
     case distinctLexical => decide
     intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finishControl?,
       LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
       Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
     subst $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
     rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_returnedPair
       _ _ _ _ _ _ _ ?sepLL ?sepLR ?sepRL ?sepRR ?noLeftGlobal ?noRightGlobal]
     case sepLL => omega
     case sepLR => omega
     case sepRL => omega
     case sepRR => omega
     case noLeftGlobal => exact $(n "leftKeyFree"):ident
     case noRightGlobal => exact $(n "rightKeyFree"):ident
     simp only [$(caller.rawContract):term, $(caller.resultsCodec):term]
     leaner_certified_close!))

/-- The storage bracket's finish: the death marker's exit row, the
borrow-free export, the updated representation at the map's own
spelling, then the storage-aware closing. -/
private def storeFinish (caller : CallerNames) (plan : StorePlan)
    (written : Term) (width : Nat) (computed : Bool) : CommandElabM Finish := do
  let twin := plan.twin
  let eraseName := mkIdent (twin.twin ++ `erase)
  let getName := mkIdent (twin.twin ++ `get)
  let readName := mkIdent (twin.twin ++ `read)
  let keyName := mkIdent (twin.twin ++ `key)
  let nsLit := Syntax.mkNatLit twin.namespaceIndex
  let tyLit := Syntax.mkNatLit twin.typeIndex
  let widthLit := Syntax.mkNatLit width
  let steps ← stepsTerm twin.namespaceIndex twin.path
  let leafWritten ← `(term| .integer ($written))
  let resource ← match plan.shape with
    | .whole => nominalTerm twin.namespaceIndex twin.path leafWritten
    | _ => `(term| LeanerIR.SemanticOperations.focusValue $steps $leafWritten)
  /- The exit rows take the path and its siblings' loan-freedom, decided
  before the row is used so a failure names the value. -/
  let plainHave ← match plan.shape with
    | .whole => `(tactic| have $(n "plainResource"):ident :
        LeanerIR.SemanticOperations.Plain $resource := by leaner_plain)
    | _ => `(tactic| have $(n "plainSteps"):ident :
        LeanerIR.SemanticOperations.PlainSteps $steps := by leaner_plain)
  let exitRow ← match plan.shape with
    | .field => `(term| LeanerIR.Proofs.Denotation.endLoan_evaluate_focusedField
        (steps := $steps) (plainSteps := $(n "plainSteps"):ident))
    | .whole => `(term| LeanerIR.Proofs.Denotation.endLoan_evaluate_wholeResource
        (plain := $(n "plainResource"):ident))
    | .saved => `(term| LeanerIR.Proofs.Denotation.endLoan_evaluate_focusedFieldSaved
        (steps := $steps) (plainSteps := $(n "plainSteps"):ident))
  let leafTyped ← `(term| ⟨$written, $(n "fitsSum"):ident⟩)
  let typed ← typedTerm twin.path leafTyped
  let borrowFreeCases ← match plan.shape with
    | .field => `(tactic| rcases mem with $(n "rfl"):ident | $(n "rfl"):ident
        | $(n "rfl"):ident | $(n "rfl"):ident)
    | .whole => `(tactic| rcases mem with $(n "rfl"):ident | $(n "rfl"):ident
        | $(n "rfl"):ident)
    | .saved => `(tactic| rcases mem with $(n "rfl"):ident | $(n "rfl"):ident
        | $(n "rfl"):ident | $(n "rfl"):ident | $(n "rfl"):ident)
  /- The exit is reached from the last statement: the scalar shapes end
  their block on the row rules, the saved shape on the throw-aware
  spine. -/
  let exitEntry : Array (TSyntax `tactic) ← match plan.shape with
    | .saved => pure #[
        ← `(tactic| rename_i $(n "nilResult"):ident $(n "nilStep"):ident),
        ← `(tactic| simp only [LeanerIR.Proofs.Denotation.statementsNil]
            at $(n "nilStep"):ident),
        ← `(tactic| subst $(n "nilStep"):ident),
        ← `(tactic| refine ⟨_, _, rfl, ?_⟩),
        ← `(tactic| intro $(n "exitFrame"):ident $(n "exitState"):ident
            $(n "retired"):ident $(n "exited"):ident)]
    | _ => pure #[
        ← `(tactic| leaner_row_drive),
        ← `(tactic| rename_i $(n "exitFrame"):ident $(n "exitState"):ident
            $(n "retired"):ident $(n "exited"):ident)]
  /- The written value's certificate: the checked result's bounds where
  the leaf is computed, the argument's own certificate where it is the
  argument. -/
  let fitsHave ← if computed then
      `(tactic| have $(n "fitsSum"):ident : LeanerIR.IntegerValueFits
        (.bits $widthLit) false ($written) := by
          simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?,
            LeanerIR.Ty.integerBounds?]
          omega)
    else `(tactic| have $(n "fitsSum"):ident := $(n "fitsVar"):ident)
  let mutateRow := mkIdent <| match plan.shape with
    | .field => ``LeanerIR.Proofs.Denotation.mutate_evaluate_focusedField
    | .whole => ``LeanerIR.Proofs.Denotation.mutate_evaluate_globalBorrow
    | .saved => ``LeanerIR.Proofs.Denotation.mutate_evaluate_focusedFieldSaved
  let closingPrefix ← `(tactic|
    simp only [$(caller.rawContract):term, $(caller.resultsCodec):term,
      $getName:ident, $readName:ident, $keyName:ident])
  let returned ← do
    let exitRow ← `(tactic|
      ($plainHave:tactic
       rw [$exitRow:term] at $(n "exited"):ident
       injection $(n "exited"):ident with $(n "exitInner"):ident
       injection $(n "exitInner"):ident with $(n "exitFrameEq"):ident
         $(n "exitStateEq"):ident $(n "retiredEq"):ident
       subst $(n "exitFrame"):ident $(n "exitState"):ident $(n "retired"):ident
       refine ⟨_, _, rfl, ?_⟩
       intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
       simp only [LeanerIR.SemanticOperations.finishControl?,
         LeanerIR.SemanticOperations.unpackFallthrough,
         Option.map_eq_map, Option.map_some, Option.some.injEq]
         at $(n "rowFinished"):ident
       subst $(n "rowFinished"):ident
       simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
       rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowFree _ _ _
         ?borrowFree]
       case borrowFree =>
         intro slot mem value eq
         simp only [List.mem_toArray, List.mem_cons, List.mem_nil_iff,
           or_false] at mem
         $borrowFreeCases:tactic <;> cases eq <;>
           simp [LeanerIR.SemanticOperations.outermostBorrows,
             LeanerIR.SemanticOperations.borrowEntry?,
             LeanerIR.SemanticOperations.collectPruned]
       rw [show LeanerIR.SemanticOperations.globalKey ⟨$nsLit⟩ ⟨$tyLit⟩
           (.address $(n "addr"):ident) =
           ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩ from rfl]
       $fitsHave:tactic
       have $(n "represented'"):ident : LeanerIR.FamilyRepresentation
           $eraseName ⟨$nsLit⟩ ⟨$tyLit⟩
           (LeanerIR.updateContents $(n "contents"):ident
             (.address $(n "addr"):ident)
             (some $typed))
           ((($(n "initial"):ident).globals.insert
               ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩
               (.loanHole ($(n "initial"):ident).nextLoan)).insert
             ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩
             $resource) :=
         LeanerIR.FamilyRepresentation.insert_over_hole (erase := $eraseName)
           $(n "represented"):ident (.address $(n "addr"):ident)
           (.loanHole ($(n "initial"):ident).nextLoan)
           $typed
       $closingPrefix:tactic
       leaner_certified_close!))
    pure (exitEntry.push exitRow)
  let thrown := (← scalarThrownPrefix) ++ #[closingPrefix,
    ← `(tactic| leaner_certified_close!)]
  pure { returned := returned, thrown := thrown, mutateRow := mutateRow }

/-- The saved bracket's inner block on the throw-aware spine: the saved
read bound as a step at the literal row, the branch's comparison computed
and split, the throwing arm closed through the marker's exit row, the
writing arm resolved as scalar events, and the block's exit as the
bracket's. -/
private def savedInner (plan : StorePlan) (finish : Finish) (steps : Term) :
    CommandElabM (TSyntax `tactic) := do
  let fuelLit := Syntax.mkNatLit (plan.fuelSaved - 1)
  /- The row the bracket law hands the inner block, with the saved slot
  empty, and the same row once the read is bound. -/
  let holder ← `(term| some (.borrow ($(n "initial"):ident).nextLoan
    (LeanerIR.SemanticOperations.focusValue $steps
      (.loanHole (($(n "initial"):ident).nextLoan + 1)))))
  let field ← `(term| some (.borrow (($(n "initial"):ident).nextLoan + 1)
    (.integer $(n "fieldVal"):ident)))
  let rowIn ← `(term| (#[some (.address $(n "addr"):ident),
    some (.integer $(n "val"):ident), $field, none, $holder] :
      LeanerIR.Proofs.Denotation.Row))
  let rowBound ← `(term| (#[some (.address $(n "addr"):ident),
    some (.integer $(n "val"):ident), $field,
    some (.integer $(n "fieldVal"):ident), $holder] :
      LeanerIR.Proofs.Denotation.Row))
  let some guardShape := plan.guard
    | throwError "the saved bracket's plan carries no guard"
  let savedName : Nat → String
    | 1 => "val"
    | 3 => "fieldVal"
    | index => s!"local{index}"
  let armThrown ← seq finish.thrown
  let armThrow ← `(tactic|
    (apply LeanerIR.Proofs.Denotation.wpRowThrow_nativeThrowEndLoan
     intro $(n "exitFrame"):ident $(n "exitState"):ident
       $(n "retired"):ident $(n "exited"):ident
     rw [LeanerIR.Proofs.Denotation.endLoan_evaluate_focusedFieldSaved
       (steps := $steps) (plainSteps := by leaner_plain)] at $(n "exited"):ident
     injection $(n "exited"):ident with $(n "exitInner"):ident
     injection $(n "exitInner"):ident with $(n "exitFrameEq"):ident
       $(n "exitStateEq"):ident $(n "retiredEq"):ident
     subst $(n "exitFrame"):ident $(n "exitState"):ident $(n "retired"):ident
     $armThrown:tactic))
  let writeEvents ← seq <| (← readBatch 2) ++ (← emitEvents finish (plan.inner.drop 2) 0)
  let writeSpine ← `(tactic|
    (apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_cons
     apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow
     case stable => leaner_row_stable
     leaner_row_drive
     $writeEvents:tactic))
  /- An `assert!` selects an empty arm before the write: step it. -/
  let armWrite ← if guardShape.throwsWhenTrue then pure writeSpine else
    `(tactic|
      (apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow
       case stable => leaner_row_stable
       refine (LeanerIR.Proofs.Denotation.wpRow_value _ _ _ _ _).mpr ?_
       $writeSpine:tactic))
  let compareValue ← seq <| #[
      ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
          $(n "rowValueOut"):ident $(n "rowEq"):ident),
      ← `(tactic| rw [$(mkIdent guardShape.row):ident] at $(n "rowEq"):ident)]
    ++ (← invertValue (mkIdent `rowEq))
    ++ (← guardSplit guardShape savedName armThrow armWrite)
  let compareThrow ← seq <| #[
      ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
          $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident),
      ← `(tactic| rw [$(mkIdent guardShape.row):ident] at $(n "rowEq"):ident)]
    ++ (← invertImpossible (mkIdent `rowEq))
  let bound ← `(tactic|
    (rename_i $(n "boundFrame"):ident $(n "boundEq"):ident
     rw [LeanerIR.Proofs.Denotation.bindVariable_rowFrame $fuelLit ⟨3⟩ _ _ _
       (by simp)] at $(n "boundEq"):ident
     cases Option.some.inj $(n "boundEq"):ident
     refine ⟨_, rfl, ?_⟩
     rw [show ($rowIn).set! 3 (some (.integer $(n "fieldVal"):ident)) = $rowBound
       from rfl]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_blockUnit
     apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_cons
     apply LeanerIR.Proofs.Denotation.wpRowThrow_nativeBranch
     case conditionStable => leaner_row_stable
     leaner_row_drive
     $(← seq (← readBatch 2)):tactic
     constructor
     case' left => $compareValue:tactic
     case' right => $compareThrow:tactic))
  /- The saved read: a local read and a dereference, resolved by the
  scalar events, then the binding. -/
  let bindFinish : Finish := { finish with returned := #[bound] }
  let savedRead ← seq <| (← readBatch 1) ++ (← emitEvents bindFinish [.dereference] 0)
  `(tactic|
    (apply LeanerIR.Proofs.Denotation.wpRowThrow_letNativeValue
     case initializerStable => leaner_row_stable
     leaner_row_drive
     $savedRead:tactic))

/-- The complete script for a bracket body, from the goal
`wp_nativeFunction` leaves. -/
def storeScript (caller : CallerNames) (plan : StorePlan) :
    CommandElabM (TSyntax `tactic) := do
  let twin := plan.twin
  let nsLit := Syntax.mkNatLit twin.namespaceIndex
  let tyLit := Syntax.mkNatLit twin.typeIndex
  let steps ← stepsTerm twin.namespaceIndex twin.path
  /- The written value is the checked operation's result over the field's
  entry value and the argument; its width is the operation's. -/
  let checked? := plan.inner.find? fun
    | .checked .. => true
    | _ => false
  let (width, subtract) := match checked? with
    | some (.checked width _ subtract) => (width, subtract)
    | _ => (64, false)
  let written ← match plan.shape, checked?.isSome, subtract with
    | .whole, false, _ => `(term| $(n "val"):ident)
    | _, _, true => `(term| $(n "fieldVal"):ident - $(n "val"):ident)
    | _, _, false => `(term| $(n "fieldVal"):ident + $(n "val"):ident)
  let finish ← storeFinish caller plan written width checked?.isSome
  let entryRow ← match plan.shape with
    | .field => `(tactic| rw [show LeanerIR.SemanticOperations.initialLocals 4
         #[.address $(n "addr"):ident, .integer $(n "val"):ident] =
         #[some (.address $(n "addr"):ident), some (.integer $(n "val"):ident),
           none, none] by
           simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
       show LeanerIR.SemanticOperations.parameterLoanLocations
         #[.address $(n "addr"):ident, .integer $(n "val"):ident] = #[] by
           simp [LeanerIR.SemanticOperations.parameterLoanLocations]])
    | .whole => `(tactic| rw [show LeanerIR.SemanticOperations.initialLocals 3
         #[.address $(n "addr"):ident, .integer $(n "val"):ident] =
         #[some (.address $(n "addr"):ident), some (.integer $(n "val"):ident),
           none] by
           simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
       show LeanerIR.SemanticOperations.parameterLoanLocations
         #[.address $(n "addr"):ident, .integer $(n "val"):ident] = #[] by
           simp [LeanerIR.SemanticOperations.parameterLoanLocations]])
    | .saved => `(tactic| rw [show LeanerIR.SemanticOperations.initialLocals 5
         #[.address $(n "addr"):ident, .integer $(n "val"):ident] =
         #[some (.address $(n "addr"):ident), some (.integer $(n "val"):ident),
           none, none, none] by
           simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
       show LeanerIR.SemanticOperations.parameterLoanLocations
         #[.address $(n "addr"):ident, .integer $(n "val"):ident] = #[] by
           simp [LeanerIR.SemanticOperations.parameterLoanLocations]])
  let law ← match plan.shape with
    | .field => `(tactic|
        (apply LeanerIR.Proofs.Denotation.wpRowThrow_focusedFieldBracket
          (steps := $steps)
         case borrowMutable => rfl
         case fieldMutable => rfl
         case present => exact $(n "presentValue"):ident
         case innerStable => leaner_row_stable))
    | .whole => `(tactic|
        (apply LeanerIR.Proofs.Denotation.wpRowThrow_wholeResourceBracket
         case borrowMutable => rfl
         case present => exact $(n "presentValue"):ident
         case innerStable => leaner_row_stable))
    | .saved => `(tactic|
        (apply LeanerIR.Proofs.Denotation.wpRowThrow_focusedFieldBracketSaved
          (steps := $steps)
         case borrowMutable => rfl
         case fieldMutable => rfl
         case present => exact $(n "presentValue"):ident))
  let reads := plan.inner.takeWhile (· == .read) |>.length
  let rest := plan.inner.drop reads
  let resolution ← match plan.shape with
    | .saved => savedInner plan finish steps
    | _ => seq <| #[← `(tactic| leaner_row_drive)] ++ (← readBatch reads)
        ++ (← emitEvents finish rest 0)
  let family := twin.twin.getString!
  let address := caller.parameters.getD 0 "addr"
  let amount := caller.parameters.getD 1 "amount"
  let pattern ← resourcePattern twin.path
  let leafPresent ← `(term| .integer $(n "fieldVal"):ident)
  let present ← match plan.shape with
    | .whole => nominalTerm twin.namespaceIndex twin.path leafPresent
    | _ => `(term| LeanerIR.SemanticOperations.focusValue $steps $leafPresent)
  let vocabulary := #[
    (s!"{family}_contents", "contents"), (s!"{family}_represented", "represented"),
    (address, "addr"), (amount, "val"), (s!"{amount}_fits", "fitsVar"),
    (s!"{amount}_nonNeg", "valNonNeg"), (s!"{amount}_max", "valMax")]
  let keyEq ← `(tactic| rw [show LeanerIR.SemanticOperations.globalKey ⟨$nsLit⟩ ⟨$tyLit⟩
      (.address $(n "addr"):ident) =
      ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩ from rfl,
    $(n "lookupEq"):ident, $(n "resourceEq"):ident])
  let entry ← `(tactic|
    (apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.address]
     $entryRow:tactic))
  /- The present side: the resource destructured down the path, its
  runtime image named, the bracket law, and the inner block. -/
  let presentSide ← `(tactic|
    (obtain $pattern:rcasesPat := $(n "resource"):ident
     have $(n "presentValue"):ident : ($(n "initial"):ident).globals.lookup
         (LeanerIR.SemanticOperations.globalKey ⟨$nsLit⟩ ⟨$tyLit⟩
           (.address $(n "addr"):ident)) =
         some $present := by
       $keyEq:tactic
       rfl
     $entry:tactic
     $law:tactic
     $resolution:tactic))
  if caller.requiresCount > 0 then
    /- The contract requires the resource: its fact is the present side. -/
    let renames ← bindVocabulary (vocabulary.push ("requires_0", "present"))
    `(tactic|
      ($renames:tactic
       have $(n "lookupEq"):ident :=
         $(n "represented"):ident (.address $(n "addr"):ident)
       rw [$(n "lookupEq"):ident] at $(n "present"):ident
       obtain ⟨$(n "resource"):ident, $(n "resourceEq"):ident⟩ :=
         Option.isSome_iff_exists.mp (by simpa using $(n "present"):ident)
       $presentSide:tactic))
  else
    /- The contract declares the missing resource as an abort direction:
    the key splits, and the absent side throws at the borrow. -/
    let renames ← bindVocabulary vocabulary
    let absentLaw := mkIdent <| match plan.shape with
      | .whole => ``LeanerIR.Proofs.Denotation.wpRowThrow_wholeBracketAbsent
      | .field | .saved => ``LeanerIR.Proofs.Denotation.wpRowThrow_fieldBracketAbsent
    let thrownSide ← seq finish.thrown
    `(tactic|
      ($renames:tactic
       have $(n "lookupEq"):ident :=
         $(n "represented"):ident (.address $(n "addr"):ident)
       cases $(n "resourceEq"):ident : $(n "contents"):ident (.address $(n "addr"):ident) with
       | none =>
           have $(n "absentValue"):ident : ($(n "initial"):ident).globals.lookup
               (LeanerIR.SemanticOperations.globalKey ⟨$nsLit⟩ ⟨$tyLit⟩
                 (.address $(n "addr"):ident)) = none := by
             $keyEq:tactic
             rfl
           $entry:tactic
           apply $absentLaw:ident (absent := $(n "absentValue"):ident)
           $thrownSide:tactic
       | some $(n "resource"):ident => $presentSide:tactic))

/-! ## Storage reads

A body that reads through a shared global borrow: `Coin[addr].amount.value`
copies the borrowed resource and selects fields from it, and
`exists<Coin>(addr)` tests the key.  The contract's family binder decides
the presence split.  At a present key the borrow's value is the erasure of
the typed contents and the selections read fields through that erasure —
the twin stays folded, so the closing meets it with its roundtrips — and
at an absent key the borrow aborts. -/

/-- A read body, read off its tree. -/
structure ReadPlan where
  /-- The family's twin, root-qualified. -/
  twin : Name
  namespaceIndex : Nat
  typeIndex : Nat
  /-- The events after the borrow — the copy and the selections, in
  evaluation order; none for an existence test. -/
  events : List Event
  /-- The existence test rather than a borrow. -/
  contains : Bool
  /-- The locals: the address, and one per `let` of the bound form. -/
  locals : Nat := 1
  deriving Repr

/-- The indices of a `ResourceLocation.mk ⟨ns⟩ ⟨ty⟩` literal. -/
private def resourceIndices? (resource : Lean.Expr) : Option (Nat × Nat) := do
  guard (resource.isAppOfArity ``ResourceLocation.mk 2)
  let namespaceIndex ← natLitOf? ((resource.getArg! 0).getArg! 0)
  let typeIndex ← natLitOf? ((resource.getArg! 1).getArg! 0)
  pure (namespaceIndex, typeIndex)

/-- Peel the field selections wrapping `tree`, outermost first, as the
generated selection rows they resolve with. -/
private partial def peelSelections
    (structOf : Nat → Nat → Option (Name × Array String))
    (tree : Lean.Expr) (outer : Array Event) : Option (Lean.Expr × Array Event) := do
  if tree.isAppOfArity ``nativeOperation 2 then
    let evaluator := tree.getArg! 0
    guard (evaluator.isAppOfArity ``NominalFieldLocation.evaluateSelect? 1)
    let location := evaluator.getArg! 0
    guard (location.isAppOfArity ``NominalFieldLocation.mk 3)
    let source := location.getArg! 0
    guard (source.isAppOfArity ``StructHandle.mk 2)
    let namespaceIndex ← natLitOf? ((source.getArg! 0).getArg! 0)
    let structIndex ← natLitOf? (source.getArg! 1)
    guard ((location.getArg! 1).isAppOf ``Option.none)
    let index ← natLitOf? (location.getArg! 2)
    let (twin, fields) ← structOf namespaceIndex structIndex
    let field ← fields[index]?
    let operand ← singleOperand? (tree.getArg! 1)
    peelSelections structOf operand
      (outer.push (.select (twin ++ Name.mkSimple s!"select_{field}")))
  else pure (tree, outer)

/-- Recognize the whole body of a one-parameter, one-local, one-result
function as selections over a copy of the shared borrow `T[local0]`, or as
the existence test `exists<T>(local0)`. -/
def readPlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (twinOf : Nat → Nat → Option (Name × Nat))
    (structOf : Nat → Nat → Option (Name × Array String)) : Option ReadPlan := do
  guard (parameterCount == 1 && resultCount == 1)
  let tree := body.bindingBody!
  guard !tree.hasLooseBVars
  /- The bound form: the shared borrow in local 1, its copy in local 2,
  the selections over local 2. -/
  let boundForm? : Option ReadPlan := do
    guard (localCount == 3)
    guard (tree.isAppOfArity ``letNativeValue 3)
    let outerBinder := tree.getArg! 0
    guard (outerBinder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
    let fuelOuter ← natLitOf? (outerBinder.getArg! 0)
    guard (fuelOuter > 0 && natLeaf? (outerBinder.getArg! 1) == some 1)
    let borrow := tree.getArg! 1
    guard (borrow.isAppOfArity ``nativeGlobalOperation 2)
    let site := borrow.getArg! 0
    guard (site.isAppOfArity ``GlobalLocationOperation.borrow 1)
    let record := site.getArg! 0
    guard (record.isAppOfArity ``BorrowLocation.mk 4)
    let (namespaceIndex, typeIndex) ← resourceIndices? (record.getArg! 0)
    let referenceType := record.getArg! 1
    guard (referenceType.isAppOfArity ``ReferenceType.mk 4)
    guard ((referenceType.getArg! 1).isConstOf ``ReferenceKind.shared)
    guard ((record.getArg! 2).isConstOf ``BorrowKind.immutable)
    guard (isLocalRead (← singleOperand? (borrow.getArg! 1)) 0)
    let innerLet := tree.getArg! 2
    guard (innerLet.isAppOfArity ``letNativeValue 3)
    let innerBinder := innerLet.getArg! 0
    guard (innerBinder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
    let fuelInner ← natLitOf? (innerBinder.getArg! 0)
    guard (fuelInner > 0 && natLeaf? (innerBinder.getArg! 1) == some 2)
    let copy := innerLet.getArg! 1
    guard (copy.isAppOfArity ``nativePrimitiveOperation 2)
    guard ((copy.getArg! 0).isAppOfArity ``PrimitiveLocationOperation.copyValue 1)
    guard (isLocalRead (← singleOperand? (copy.getArg! 1)) 1)
    let (inner, selections) ← peelSelections structOf (innerLet.getArg! 2) #[]
    guard (isLocalRead inner 2)
    let (twin, _) ← twinOf namespaceIndex typeIndex
    pure {
      twin := twin
      namespaceIndex := namespaceIndex
      typeIndex := typeIndex
      events := [.bind 1 fuelOuter, .read, .copy, .bind 2 fuelInner, .read]
        ++ selections.reverse.toList
      contains := false
      locals := 3 }
  if let some plan := boundForm? then return plan
  guard (localCount == 1)
  if tree.isAppOfArity ``nativeGlobalOperation 2 then
    let operation := tree.getArg! 0
    guard (operation.isAppOfArity ``GlobalLocationOperation.contains 1)
    let (namespaceIndex, typeIndex) ← resourceIndices? (operation.getArg! 0)
    let operand ← singleOperand? (tree.getArg! 1)
    guard (isLocalRead operand 0)
    let (twin, _) ← twinOf namespaceIndex typeIndex
    pure {
      twin := twin
      namespaceIndex := namespaceIndex
      typeIndex := typeIndex
      events := []
      contains := true }
  else
    let (inner, selections) ← peelSelections structOf tree #[]
    guard (inner.isAppOfArity ``nativePrimitiveOperation 2)
    guard ((inner.getArg! 0).isAppOfArity ``PrimitiveLocationOperation.copyValue 1)
    let borrow ← singleOperand? (inner.getArg! 1)
    guard (borrow.isAppOfArity ``nativeGlobalOperation 2)
    let site := borrow.getArg! 0
    guard (site.isAppOfArity ``GlobalLocationOperation.borrow 1)
    let record := site.getArg! 0
    guard (record.isAppOfArity ``BorrowLocation.mk 4)
    let (namespaceIndex, typeIndex) ← resourceIndices? (record.getArg! 0)
    let referenceType := record.getArg! 1
    guard (referenceType.isAppOfArity ``ReferenceType.mk 4)
    guard ((referenceType.getArg! 1).isConstOf ``ReferenceKind.shared)
    guard ((record.getArg! 2).isConstOf ``BorrowKind.immutable)
    let operand ← singleOperand? (borrow.getArg! 1)
    guard (isLocalRead operand 0)
    let (twin, _) ← twinOf namespaceIndex typeIndex
    /- Evaluation runs inside out: the copy, then the innermost selection. -/
    pure {
      twin := twin
      namespaceIndex := namespaceIndex
      typeIndex := typeIndex
      events := .copy :: selections.reverse.toList
      contains := false }

/-- The read route's tactic block, from the goal `wp_nativeFunction`
leaves.  The parameter row is the address alone; the frame stays
borrow-free, so the exit exports nothing. -/
def readScript (caller : CallerNames) (plan : ReadPlan) :
    CommandElabM (TSyntax `tactic) := do
  let nsLit := Syntax.mkNatLit plan.namespaceIndex
  let tyLit := Syntax.mkNatLit plan.typeIndex
  let getName := mkIdent (plan.twin ++ `get)
  let readName := mkIdent (plan.twin ++ `read)
  let keyName := mkIdent (plan.twin ++ `key)
  let containsName := mkIdent (plan.twin ++ `contains)
  let noBorrowsName := mkIdent (plan.twin ++ `outermostBorrows_erase)
  let closingPrefix ← `(tactic|
    simp only [$(caller.rawContract):term, $(caller.resultsCodec):term,
      $getName:ident, $readName:ident, $keyName:ident, $containsName:ident])
  let keyEq ← `(tactic| rw [show LeanerIR.SemanticOperations.globalKey ⟨$nsLit⟩ ⟨$tyLit⟩
      (.address $(n "addr"):ident) =
      ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩ from rfl])
  /- One alternative per slot of the exit row, each substituted. -/
  let slotPatterns ← (List.range plan.locals).toArray.mapM fun _ =>
    `(rcasesPat| $(n "rfl"):ident)
  let slotCases ← `(tactic| rcases mem with $[$slotPatterns]|*)
  let returned := (← scalarReturnedPrefix) ++ #[
    ← `(tactic| rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowFree _ _ _
        ?borrowFree]),
    ← `(tactic| case borrowFree =>
        intro slot mem value eq
        simp only [List.mem_toArray, List.mem_cons, List.mem_nil_iff, or_false] at mem
        $slotCases:tactic <;> cases eq <;>
          first
          | simp only [$noBorrowsName:ident]
          | simp [LeanerIR.SemanticOperations.outermostBorrows,
              LeanerIR.SemanticOperations.borrowEntry?,
              LeanerIR.SemanticOperations.collectPruned]),
    closingPrefix, ← `(tactic| leaner_certified_close!)]
  let thrown := (← scalarThrownPrefix) ++ #[closingPrefix,
    ← `(tactic| leaner_certified_close!)]
  let finish : Finish := {
    returned := returned
    thrown := thrown
    mutateRow := mkIdent ``LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_singleBorrow }
  let family := plan.twin.getString!
  let address := caller.parameters.getD 0 "addr"
  let renames ← bindVocabulary #[
    (s!"{family}_contents", "contents"), (s!"{family}_represented", "represented"),
    (address, "addr")]
  let localsLit := Syntax.mkNatLit plan.locals
  let mut entrySlots : Array Term := #[← `(term| some (.address $(n "addr"):ident))]
  for _ in [1:plan.locals] do
    entrySlots := entrySlots.push (← `(term| none))
  let entry ← `(tactic|
    (have $(n "lookupEq"):ident := $(n "represented"):ident (.address $(n "addr"):ident)
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.address]
     rw [show LeanerIR.SemanticOperations.initialLocals $localsLit
         #[.address $(n "addr"):ident] = #[$entrySlots,*] by
           simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
       show LeanerIR.SemanticOperations.parameterLoanLocations
         #[.address $(n "addr"):ident] = #[] by
           simp [LeanerIR.SemanticOperations.parameterLoanLocations]]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow
     case stable => leaner_row_stable
     leaner_row_drive))
  let reads ← seq (← readBatch 1)
  let valueIntro ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
    $(n "rowValueOut"):ident $(n "rowEq"):ident)
  let throwIntro ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
    $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident)
  if plan.contains then
    let containsRow ← `(tactic| rw [LeanerIR.Proofs.Denotation.contains_evaluate]
      at $(n "rowEq"):ident)
    let valueBranch ← seq <| #[valueIntro, containsRow]
      ++ (← invertValue (mkIdent `rowEq)) ++ #[keyEq] ++ finish.returned
    let throwBranch ← seq <| #[throwIntro, containsRow]
      ++ (← invertImpossible (mkIdent `rowEq))
    return ← `(tactic|
      ($renames:tactic
       $entry:tactic
       $reads:tactic
       constructor
       case' left => $valueBranch:tactic
       case' right => $throwBranch:tactic))
  let presentRow ← `(tactic|
    rw [LeanerIR.Proofs.Denotation.globalBorrowShared_evaluate_present _ _ _ _ rfl _ _ _ _
      (by rw [show LeanerIR.SemanticOperations.globalKey ⟨$nsLit⟩ ⟨$tyLit⟩
            (.address $(n "addr"):ident) =
            ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩ from rfl]
          exact $(n "lookupEq"):ident)] at $(n "rowEq"):ident)
  let absentRow ← `(tactic|
    rw [LeanerIR.Proofs.Denotation.globalBorrowShared_evaluate_absent _ _ _ _ _ _ _
      (by rw [show LeanerIR.SemanticOperations.globalKey ⟨$nsLit⟩ ⟨$tyLit⟩
            (.address $(n "addr"):ident) =
            ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩ from rfl]
          exact $(n "lookupEq"):ident)] at $(n "rowEq"):ident)
  let someLookup ← `(tactic|
    (rw [$(n "contentsEq"):ident] at $(n "lookupEq"):ident
     simp only [Option.map_some] at $(n "lookupEq"):ident))
  let noneLookup ← `(tactic|
    (rw [$(n "contentsEq"):ident] at $(n "lookupEq"):ident
     simp only [Option.map_none] at $(n "lookupEq"):ident))
  let valueSome ← seq <| #[someLookup, presentRow]
    ++ (← invertValue (mkIdent `rowEq)) ++ #[← `(tactic| leaner_row_drive)]
    ++ (← emitEvents finish plan.events 0)
  let valueNone ← seq <| #[noneLookup, absentRow] ++ (← invertImpossible (mkIdent `rowEq))
  let throwSome ← seq <| #[someLookup, presentRow] ++ (← invertImpossible (mkIdent `rowEq))
  /- The absent key throws; a contract that requires the resource refutes
  the branch by its requirement, one that declares the abort direction
  meets the throw's obligation. -/
  let throwNone ← if caller.requiresCount > 0 then
    seq <| #[noneLookup, ← `(tactic|
      (rw [$(n "lookupEq"):ident] at $(n "requires_0"):ident
       simp at $(n "requires_0"):ident))]
  else
    seq <| #[noneLookup, absentRow] ++ (← invertThrow (mkIdent `rowEq)) ++ finish.thrown
  let valueBranch ← `(tactic|
    ($valueIntro:tactic
     cases $(n "contentsEq"):ident : $(n "contents"):ident (.address $(n "addr"):ident) with
     | none => $valueNone:tactic
     | some $(n "c"):ident => $valueSome:tactic))
  let throwBranch ← `(tactic|
    ($throwIntro:tactic
     cases $(n "contentsEq"):ident : $(n "contents"):ident (.address $(n "addr"):ident) with
     | none => $throwNone:tactic
     | some $(n "c"):ident => $throwSome:tactic))
  `(tactic|
    ($renames:tactic
     $entry:tactic
     $reads:tactic
     constructor
     case' left => $valueBranch:tactic
     case' right => $throwBranch:tactic))

/-! ## Returned reborrows

A body that is the reborrow of its sole `&mut` parameter, returned.  The
mint leaves the loan's hole in the parameter and the borrow is the value;
the exit exports the hole as the parameter's prophecy, and the contract's
`resolveReturnedBorrows` clause fills it with the returned current. -/

/-- A returned-reborrow body, read off its tree. -/
structure ReturnPlan where
  /-- The reborrow's lexical loan. -/
  lex : Nat
  deriving Repr

/-- Recognize `reborrow local0` as the whole body of a one-parameter,
one-result function. -/
def returnPlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat) :
    Option ReturnPlan := do
  guard (parameterCount == 1 && localCount == 1 && resultCount == 1)
  let tree := body.bindingBody!
  guard !tree.hasLooseBVars
  guard (tree.isAppOfArity ``nativeDerefLocalBorrowOperation 2)
  guard ((tree.getArg! 1).isConstOf ``valuesNil)
  let record := tree.getArg! 0
  guard (record.isAppOfArity ``DerefLocalBorrowOperation.mk 5)
  guard (natLeaf? (record.getArg! 0) == some 0)
  guard ((record.getArg! 1).isAppOf ``List.nil)
  let lex ← natLitOf? (record.getArg! 4)
  pure { lex }

/-- The complete script for a returned-reborrow body, from the goal
`wp_nativeFunction` leaves. -/
def returnScript (caller : CallerNames) (_plan : ReturnPlan) :
    CommandElabM (TSyntax `tactic) := do
  let renames ← bindVocabulary (borrowVocabulary (caller.parameters.getD 0 "slot"))
  `(tactic|
    ($renames:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.mutable,
       LeanerIR.Proofs.Denotation.initialLocals_one,
       LeanerIR.SemanticOperations.parameterLoanLocations_singleBorrow]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_returnedReborrow (rest := [])
     case mutableKind => rfl
     intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finishControl?,
       LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
       Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
     subst $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
     rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_returnedReborrow
       _ _ _ _ _ ?separate ?noGlobal]
     case separate => omega
     case noGlobal => exact $(n "keyFree"):ident
     simp only [$(caller.rawContract):term, $(caller.resultsCodec):term]
     leaner_certified_close!))

/-! ## A returned reborrow projected from a struct parameter

`&mut pair.left`: the parameter is a mutable borrow of a struct of
integers, and the body reborrows one of its fields.  The preamble
destructures the struct into one `Int` per field; the lender exports its
struct with the returned loan's hole at the focus, and the callee's own
resolution fills it. -/

structure ProjectPlan where
  twin : Name
  namespaceIndex : Nat
  structIndex : Nat
  /-- The struct's field count; every field is an integer. -/
  fieldCount : Nat
  /-- The reborrowed field. -/
  focus : Nat
  deriving Repr

/-- Recognize `reborrow local0.[f]` as the whole body of a one-parameter,
one-result function whose parameter borrows a struct of integers. -/
def projectPlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (twinInfoOf : Nat → Nat → Option (Name × Array (String × SpecTypes.FieldRep))) :
    Option ProjectPlan := do
  guard (parameterCount == 1 && localCount == 1 && resultCount == 1)
  let tree := body.bindingBody!
  guard !tree.hasLooseBVars
  guard (tree.isAppOfArity ``nativeDerefLocalBorrowOperation 2)
  guard ((tree.getArg! 1).isConstOf ``valuesNil)
  let record := tree.getArg! 0
  guard (record.isAppOfArity ``DerefLocalBorrowOperation.mk 5)
  guard (natLeaf? (record.getArg! 0) == some 0)
  guard ((record.getArg! 3).isConstOf ``BorrowKind.mutable)
  let fields := record.getArg! 1
  guard (fields.isAppOfArity ``List.cons 3)
  guard ((fields.getArg! 2).isAppOf ``List.nil)
  let step := fields.getArg! 1
  guard (step.isAppOfArity ``LeanerIR.SemanticOperations.NominalFieldStep.mk 3)
  guard ((step.getArg! 1).isAppOf ``Option.none)
  let source := step.getArg! 0
  let namespaceIndex ← natLitOf? ((source.getArg! 0).getArg! 0)
  let structIndex ← natLitOf? (source.getArg! 1)
  let focus ← natLitOf? (step.getArg! 2)
  let (twin, fieldReps) ← twinInfoOf namespaceIndex structIndex
  guard (focus < fieldReps.size)
  guard (fieldReps.all fun (_, rep) => isInteger rep)
  pure { twin, namespaceIndex, structIndex, fieldCount := fieldReps.size, focus }

/-- The complete script for a projected returned reborrow, from the goal
`wp_nativeFunction` leaves. -/
def projectScript (caller : CallerNames) (plan : ProjectPlan) :
    CommandElabM (TSyntax `tactic) := do
  let parameter := caller.parameters.getD 0 "slot"
  let renames ← bindVocabulary #[(s!"{parameter}_loan", "loanVar"),
    (s!"{parameter}_loan_keyFree", "keyFree"), (s!"{parameter}_loan_bound", "loanBound")]
  /- The preamble's field binders are inaccessible; they are named in
  declaration order. -/
  let fieldIdent (index : Nat) : Ident := mkIdent (Name.mkSimple s!"field{index}")
  let fieldIdents := (Array.range plan.fieldCount).map fieldIdent
  let fieldBinders ← fieldIdents.mapM fun field => `(Lean.binderIdent| $field:ident)
  let erased ← fieldIdents.mapM fun field => `(term| LeanerIR.RuntimeValue.integer $field:ident)
  let before := erased.extract 0 plan.focus
  let after := erased.extract (plan.focus + 1) erased.size
  let nsLit := Syntax.mkNatLit plan.namespaceIndex
  let structLit := Syntax.mkNatLit plan.structIndex
  let steps ← `(term| ([⟨⟨⟨$nsLit⟩, $structLit⟩, #[$before,*], #[$after,*]⟩] :
    List LeanerIR.SemanticOperations.FocusStep))
  let leaf ← `(term| LeanerIR.RuntimeValue.integer $(fieldIdent plan.focus):ident)
  let hole ← `(term| LeanerIR.RuntimeValue.loanHole ($(n "initial"):ident).nextLoan)
  let entryRow ← `(term| (#[some (.borrow $(n "loanVar"):ident
    (.nominal ⟨⟨$nsLit⟩, $structLit⟩ none #[$erased,*]))] :
    LeanerIR.Proofs.Denotation.Row))
  let lentRow ← `(term| (#[some (.borrow $(n "loanVar"):ident
    (LeanerIR.SemanticOperations.focusValue $steps $hole))] :
    LeanerIR.Proofs.Denotation.Row))
  let codecName := mkIdent (plan.twin ++ `codec)
  let eraseName := mkIdent (plan.twin ++ `erase)
  `(tactic|
    (rename_i $fieldBinders*
     $renames:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.mutable, $codecName:ident, $eraseName:ident,
       LeanerIR.Proofs.Denotation.initialLocals_one,
       LeanerIR.SemanticOperations.parameterLoanLocations_singleBorrow]
     have $(n "plainSteps"):ident : LeanerIR.SemanticOperations.PlainSteps $steps := by
       leaner_plain
     apply LeanerIR.Proofs.Denotation.wpRowThrow_returnedReborrowPath (localId := ⟨0⟩)
       (steps := $steps) (leaf := $leaf)
     case mutableKind => rfl
     case slot => rfl
     intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finishControl?,
       LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
       Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
     subst $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
     rw [show ($entryRow).set! 0 (some (.borrow $(n "loanVar"):ident
       (LeanerIR.SemanticOperations.focusValue $steps $hole))) = $lentRow from rfl]
     rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_returnedProjected
       _ _ _ _ $(n "plainSteps"):ident _ _ ?separate ?noGlobal]
     case separate => omega
     case noGlobal => exact $(n "keyFree"):ident
     simp only [$(caller.rawContract):term, $(caller.resultsCodec):term]
     leaner_certified_close!))

/-! ## Callers of a reborrow-returning callee

The callee's summary states the transfer — its export is the returned
loan's hole — so a caller fills its own hole with it and continues with
the returned borrow as an ordinary row value.  The callee's runtime result
row is recovered from the decoded value through the codec inversions. -/

/-- The reborrow-returning call `f(&mut *slot)` at the body's top, its
value returned. -/
structure ForwardPlan where
  callee : CalleeNames
  /-- The reborrow's lexical loan. -/
  lex : Nat

/-- Recognize `nativeCall f (values [reborrow local0])` and name the
callee. -/
private def returnedCall? (e : Lean.Expr) : Option (CalleeNames × Nat) := do
  guard (e.isAppOfArity ``nativeCall 4)
  guard ((e.getArg! 1).isAppOf ``Option.none)
  let callee := e.getArg! 2
  let relation ← calleeRelation? callee
  let inner := e.getArg! 3
  guard (inner.isAppOfArity ``valuesCons 2)
  guard ((inner.getArg! 1).isConstOf ``valuesNil)
  let reborrow := inner.getArg! 0
  guard (reborrow.isAppOfArity ``nativeDerefLocalBorrowOperation 2)
  guard ((reborrow.getArg! 1).isConstOf ``valuesNil)
  let record := reborrow.getArg! 0
  guard (record.isAppOfArity ``DerefLocalBorrowOperation.mk 5)
  guard (natLeaf? (record.getArg! 0) == some 0)
  guard ((record.getArg! 1).isAppOf ``List.nil)
  guard ((record.getArg! 3).isConstOf ``BorrowKind.mutable)
  let lex ← natLitOf? (record.getArg! 4)
  let names ← calleeNames? relation
  pure (names, lex)

/-- Recognize a body that is exactly the returned call. -/
def forwardPlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (statesTransfer : Name → Bool) : Option ForwardPlan := do
  guard (parameterCount == 1 && localCount == 1 && resultCount == 1)
  let (callee, lex) ← returnedCall? body.bindingBody!
  /- The route reads the transfer off the callee's summary; a callee whose
  result is not a whole reborrow of its parameter states none. -/
  guard (statesTransfer callee.rawContract)
  pure { callee, lex }

/-- The callee-side facts of one reborrow-returning call: the contract's
permission, then on return the summary destructured — the runtime result
row recovered through the codec inversions, the transfer substituted, the
lender's value resolved — and on throw the callee's (absent) abort clause
refuted.  `continuation` closes the caller from the destructured facts. -/
private def returnedCallSite (callee : CalleeNames) (continuation : TSyntax `tactic) :
    CommandElabM (TSyntax `tactic) := do
  let calleeContract := mkIdent callee.contract
  let calleeTyped := mkIdent callee.typedContract
  let calleeRaw := mkIdent callee.rawContract
  let calleeCodec := mkIdent callee.argumentsCodec
  let calleeResults := mkIdent callee.resultsCodec
  `(tactic|
    (case mutableKind => rfl
     case priorLoan => exact $(n "loanBound"):ident
     case calleeWp =>
       apply LeanerIR.Proofs.Denotation.wpFunction_of_satisfies
         $(calleeSatIdent callee)
       case permitted =>
         simp only [$calleeContract:term,
           LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
           LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
           $calleeCodec:term,
           LeanerIR.Proofs.Codec.mutable, LeanerIR.Proofs.Codec.specInt]
         refine ⟨⟨⟨($(n "initial"):ident).nextLoan, ⟨$(n "valVar"):ident, ?_⟩⟩⟩, rfl,
           ($(n "initial"):ident).nextLoan, $(n "valVar"):ident,
           ⟨⟨⟨rfl, by omega, by omega⟩, ?_⟩, by omega⟩, ?_⟩
         · simp [LeanerIR.IntegerValueFits,
             LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
           omega
         · intro $(n "rowLoan"):ident $(n "rowBound"):ident
           have $(n "rowAdvanced"):ident :
               ($(n "initial"):ident).nextLoan + 1 ≤ $(n "rowLoan"):ident :=
             $(n "rowBound"):ident
           exact $(n "freshLoans"):ident $(n "rowLoan"):ident (by omega)
         · simp only [LeanerIR.SemanticOperations.globalLoanKey?]
           exact $(n "freshLoans"):ident ($(n "initial"):ident).nextLoan (by omega)
       case onReturn =>
         intro $(n "results"):ident $(n "final"):ident $(n "ensuresFn"):ident
           $(n "frameFn"):ident $(n "notMust"):ident
         have $(n "ensures"):ident := $(n "ensuresFn"):ident
           (by exact $(n "notMust"):ident)
         simp only [$calleeContract:term,
           LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
           LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
           at $(n "ensures"):ident $(n "frameFn"):ident
         obtain ⟨$(n "rowArgs"):ident, $(n "rowResult"):ident,
           $(n "rowArgsEq"):ident, $(n "rowDecodeEq"):ident,
           $(n "slotLoan"):ident, $(n "slotEntry"):ident, $(n "slot"):ident,
           $(n "slotPending"):ident, $(n "resultLoan"):ident, $(n "result"):ident,
           ⟨⟨⟨⟨$(n "argsEq2"):ident, $(n "resultsEq"):ident⟩,
             ⟨⟨⟨⟨$(n "pendingEq"):ident, $(n "resolveEq"):ident⟩,
               $(n "transferEq"):ident⟩, $(n "freshLo"):ident⟩, $(n "freshHi"):ident⟩⟩,
             ⟨$(n "resNonNeg"):ident, $(n "resMax"):ident⟩⟩,
             ⟨$(n "slotNonNeg"):ident, $(n "slotMax"):ident⟩⟩,
           $(n "obligation"):ident⟩ := $(n "ensures"):ident
         simp only [$calleeResults:term] at $(n "rowDecodeEq"):ident $(n "resultsEq"):ident
         split at $(n "rowDecodeEq"):ident
         · rename_i $(n "r0"):ident $(n "listEq"):ident
           simp only [Option.bind_eq_some_iff, Option.some.injEq, exists_eq_right]
             at $(n "rowDecodeEq"):ident
           obtain ⟨$(n "current"):ident, $(n "r0Eq"):ident, $(n "currentEq"):ident⟩ :=
             LeanerIR.Proofs.Codec.mutable_decode?_eq_some $(n "rowDecodeEq"):ident
           have $(n "currentInt"):ident :=
             LeanerIR.Proofs.Codec.specInt_decode?_eq_some $(n "currentEq"):ident
           subst $(n "currentInt"):ident $(n "r0Eq"):ident
           have $(n "resultsShape"):ident : $(n "results"):ident =
               #[.borrow ($(n "rowResult"):ident).loan
                 (.integer ($(n "rowResult"):ident).value.val)] := by
             have $(n "listArray"):ident := congrArg List.toArray $(n "listEq"):ident
             simpa using $(n "listArray"):ident
           subst $(n "resultsShape"):ident
           simp only [LeanerIR.Proofs.Codec.mutable, LeanerIR.Proofs.Codec.specInt,
             Array.mk.injEq, List.cons.injEq, LeanerIR.RuntimeValue.borrow.injEq,
             LeanerIR.RuntimeValue.integer.injEq, and_true] at $(n "resultsEq"):ident
           obtain ⟨$(n "loanEq"):ident, $(n "valueEq"):ident⟩ := $(n "resultsEq"):ident
           rw [$(n "rowArgsEq"):ident] at $(n "argsEq2"):ident
           simp only [Array.mk.injEq, List.cons.injEq, LeanerIR.RuntimeValue.borrow.injEq,
             LeanerIR.RuntimeValue.integer.injEq, and_true] at $(n "argsEq2"):ident
           obtain ⟨$(n "slotLoanEq"):ident, $(n "slotEntryEq"):ident⟩ := $(n "argsEq2"):ident
           subst $(n "slotLoanEq"):ident $(n "slotEntryEq"):ident
           subst $(n "transferEq"):ident
           rw [LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedBorrow]
             at $(n "resolveEq"):ident
           injection $(n "resolveEq"):ident with $(n "slotEq"):ident
           subst $(n "slotEq"):ident
           simp only [LeanerIR.Proofs.Obligation_iff] at $(n "obligation"):ident
           obtain ⟨$(n "resultEntry"):ident, -⟩ := $(n "obligation"):ident
           subst $(n "resultEntry"):ident
           obtain ⟨$(n "rowArgs2"):ident, $(n "rowArgsEq3"):ident,
             $(n "globalsEq"):ident, $(n "discipline"):ident⟩ := $(n "frameFn"):ident
           have $(n "freshLoLoan"):ident :
               ($(n "initial"):ident).nextLoan + 1 ≤ $(n "resultLoan"):ident :=
             $(n "freshLo"):ident
           have $(n "freshHiLoan"):ident :
               $(n "resultLoan"):ident < ($(n "final"):ident).nextLoan :=
             $(n "freshHi"):ident
           have $(n "monotone"):ident :
               ($(n "initial"):ident).nextLoan + 1 ≤ ($(n "final"):ident).nextLoan :=
             ($(n "discipline"):ident).2.2
           have $(n "stableKey"):ident :=
             ($(n "discipline"):ident).2.1 $(n "loanVar"):ident (by
               show $(n "loanVar"):ident < ($(n "initial"):ident).nextLoan + 1
               omega)
           have $(n "resultFits"):ident := ($(n "rowResult"):ident).value.fits
           refine ⟨$(n "resultLoan"):ident, ($(n "rowResult"):ident).value.val,
             by rw [$(n "loanEq"):ident], $(n "pendingEq"):ident, ?_⟩
           $continuation:tactic
         · exact absurd $(n "rowDecodeEq"):ident (by simp)
       case onThrow =>
         intro $(n "rowKind"):ident $(n "rowThrown"):ident $(n "final"):ident
           $(n "aborts"):ident
         intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
           $(n "rowOutcome"):ident $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finishControl?,
           Option.some.injEq] at $(n "rowFinished"):ident
         subst $(n "rowFinished"):ident
         simp only [$calleeContract:term,
           LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
           LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
           at $(n "aborts"):ident
         obtain ⟨$(n "rowArgs"):ident, $(n "rowArgsEq"):ident,
           $(n "rowSlotLoan"):ident, $(n "rowSlot"):ident,
           $(n "rowArgsEq2"):ident, $(n "rowObligation"):ident⟩ :=
           $(n "aborts"):ident
         simp only [LeanerIR.Proofs.Obligation_iff] at $(n "rowObligation"):ident))

/-- The loan-discipline chain a caller's exit establishes: the reborrow's
mint, the callee, and the export. -/
private def forwardDiscipline (exported : Term) : CommandElabM Term :=
  `(term| (LeanerIR.SemanticOperations.LoanDiscipline.of_eq
      (initial := $(n "initial"):ident)
      (final := { $(n "initial"):ident with
        nextLoan := ($(n "initial"):ident).nextLoan + 1 }) rfl (Nat.le_succ _)).trans
      (($(n "discipline"):ident).trans
        (LeanerIR.SemanticOperations.LoanDiscipline.of_eq
          (final := $exported) rfl (Nat.le_refl _))))

/-- The complete script for a body that returns its callee's reborrow. -/
def forwardScript (caller : CallerNames) (plan : ForwardPlan) :
    CommandElabM (TSyntax `tactic) := do
  let callee := plan.callee
  let exported ← `(term|
    { globals := ($(n "final"):ident).globals
      globalLoans := ($(n "final"):ident).globalLoans
      nextLoan := ($(n "final"):ident).nextLoan
      pending := ($(n "initial"):ident).pending.push
        ($(n "loanVar"):ident, LeanerIR.RuntimeValue.loanHole $(n "resultLoan"):ident) })
  let exit ← `(tactic|
    (intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finishControl?,
       LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
       Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
     subst $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
     rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_returnedReborrow
       _ _ _ _ _ ?separate ?noGlobal]
     case separate => omega
     case noGlobal =>
       rw [$(n "stableKey"):ident]
       exact $(n "keyFree"):ident
     simp only [$(caller.rawContract):term, $(caller.resultsCodec):term]
     have $(n "rowDiscipline"):ident := $(← forwardDiscipline exported)
     leaner_certified_close!))
  let site ← returnedCallSite callee exit
  let renames ← bindVocabulary (borrowVocabulary (caller.parameters.getD 0 "slot"))
  let calleeFact ← calleeSatFact caller callee (generic := false)
  `(tactic|
    ($calleeFact:tactic
     $renames:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.mutable,
       LeanerIR.Proofs.Denotation.initialLocals_one,
       LeanerIR.SemanticOperations.parameterLoanLocations_singleBorrow]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_callReturnedReborrow (rest := [])
     $site:tactic))

/-! ## Writing through a callee's returned reborrow

`set_then_read` binds the returned reborrow, writes through it, retires it
with its own death marker (the write reaches the parameter), and reads the
parameter; `set_through_forward` writes through it under a death marker
wrapping the binding.  Both are the call law, the throw-aware binding, the
scalar write, and the settling exit row. -/

/-- Which of the two writing shapes a body is. -/
inductive SetShape where
  /-- `let r := f(&mut *slot); *r := v; endLoan; *slot`, returning the read. -/
  | thenRead
  /-- `endLoan (let r := f(&mut *slot); *r := v)`, returning unit. -/
  | throughForward
  deriving Repr, BEq

/-- A writing caller, read off its tree. -/
structure SetPlan where
  shape : SetShape
  callee : CalleeNames
  /-- The binding's fuel. -/
  fuel : Nat
  /-- The literal written through the returned reference. -/
  written : Nat

/-- Recognize `mutate local1 (value (integer n))` as a whole block. -/
private def writeLocal1? (block : Lean.Expr) : Option Nat := do
  guard (block.isAppOfArity ``statementsCons 2)
  guard ((block.getArg! 1).isConstOf ``statementsNil)
  let write := block.getArg! 0
  guard (write.isAppOfArity ``nativeReferenceOperation 2)
  guard ((write.getArg! 0).isConstOf ``ReferenceLocationOperation.mutate)
  let operands := write.getArg! 1
  guard (operands.isAppOfArity ``valuesCons 2)
  guard (isLocalRead (operands.getArg! 0) 1)
  let valueOperand ← singleOperand? (operands.getArg! 1)
  guard (valueOperand.isAppOfArity ``value 1)
  let literal := valueOperand.getArg! 0
  guard (literal.isAppOfArity ``LeanerIR.RuntimeValue.integer 1)
  intLitOf? (literal.getArg! 0)

/-- Recognize `let r := f(&mut *slot) in body` and hand back the binding's
fuel, the callee, and the body. -/
private def boundCall? (e : Lean.Expr) : Option (Nat × CalleeNames × Nat × Lean.Expr) := do
  guard (e.isAppOfArity ``letNativeValue 3)
  let binder := e.getArg! 0
  guard (binder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
  let fuel ← natLitOf? (binder.getArg! 0)
  guard (fuel > 0)
  guard (natLeaf? (binder.getArg! 1) == some 1)
  let (callee, lex) ← returnedCall? (e.getArg! 1)
  guard (lex == 0)
  pure (fuel, callee, lex, e.getArg! 2)

/-- Recognize either writing shape. -/
def setPlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (statesTransfer : Name → Bool) : Option SetPlan := do
  guard (parameterCount == 1 && localCount == 2)
  let tree := body.bindingBody!
  if resultCount == 1 then
    let (fuel, callee, _, rest) ← boundCall? tree
    guard (statesTransfer callee.rawContract)
    guard (rest.isAppOfArity ``blockResult 2)
    let written ← writeLocal1? (rest.getArg! 0)
    let inner := rest.getArg! 1
    guard (inner.isAppOfArity ``blockResult 2)
    let marker := inner.getArg! 0
    guard (marker.isAppOfArity ``statementsCons 2)
    guard ((marker.getArg! 1).isConstOf ``statementsNil)
    let death := marker.getArg! 0
    guard (death.isAppOfArity ``nativeReferenceOperation 2)
    guard ((death.getArg! 0).isAppOfArity ``ReferenceLocationOperation.endLoan 1)
    guard ((death.getArg! 1).isConstOf ``valuesNil)
    let read := inner.getArg! 1
    guard (read.isAppOfArity ``nativeReferenceOperation 2)
    guard ((read.getArg! 0).isConstOf ``ReferenceLocationOperation.dereference)
    guard (isLocalRead (← singleOperand? (read.getArg! 1)) 0)
    pure { shape := .thenRead, callee, fuel, written }
  else
    guard (resultCount == 0)
    guard (tree.isAppOfArity ``nativeReferenceOperation 2)
    guard ((tree.getArg! 0).isAppOfArity ``ReferenceLocationOperation.endLoan 1)
    let bound ← singleOperand? (tree.getArg! 1)
    let (fuel, callee, _, rest) ← boundCall? bound
    guard (statesTransfer callee.rawContract)
    guard (rest.isAppOfArity ``blockUnit 1)
    let written ← writeLocal1? (rest.getArg! 0)
    pure { shape := .throughForward, callee, fuel, written }

/-- The tactics resolving the write through the returned reborrow in local
1, from the drive's read of that local to the row it leaves. -/
private def returnedWrite : CommandElabM (Array (TSyntax `tactic)) := do
  pure #[
    ← `(tactic| leaner_row_drive),
    ← `(tactic| rename_i $(n "rowRead0"):ident $(n "rowReadEq0"):ident),
    ← `(tactic| simp only [List.getElem?_toArray, List.getElem?_cons_zero,
        List.getElem?_cons_succ, Option.join_some, Option.some.injEq]
        at $(n "rowReadEq0"):ident),
    ← `(tactic| subst $(n "rowReadEq0"):ident),
    ← `(tactic| constructor),
    ← `(tactic| case' right =>
        (intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
           $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident
         rw [LeanerIR.Proofs.Denotation.mutate_evaluate_returnedReborrowLocal1
           _ _ _ _ _ _ (by omega)] at $(n "rowEq"):ident
         injection $(n "rowEq"):ident with $(n "rowInner"):ident
         injection $(n "rowInner"):ident)),
    ← `(tactic| case' left =>
        (intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
           $(n "rowValueOut"):ident $(n "rowEq"):ident
         rw [LeanerIR.Proofs.Denotation.mutate_evaluate_returnedReborrowLocal1
           _ _ _ _ _ _ (by omega)] at $(n "rowEq"):ident
         injection $(n "rowEq"):ident with $(n "rowInner"):ident
         injection $(n "rowInner"):ident with $(n "rowFrameEq"):ident
           $(n "rowStateEq"):ident $(n "rowValueEq"):ident
         injection $(n "rowFrameEq"):ident with $(n "rowRowEq"):ident
           $(n "rowActiveEq"):ident $(n "rowLocationsEq"):ident
         subst $(n "rowRowEq"):ident $(n "rowStateEq"):ident $(n "rowValueEq"):ident
         leaner_row_drive
         rename_i $(n "nilResult"):ident $(n "nilStep"):ident
         simp only [LeanerIR.Proofs.Denotation.statementsNil] at $(n "nilStep"):ident
         subst $(n "nilStep"):ident
         refine ⟨_, _, rfl, ?_⟩))]

/-- The caller's exit once its parameter holds the written value beside
the cleared returned slot: the export, then the closing. -/
private def writtenExit (caller : CallerNames) (written : Nat) :
    CommandElabM (TSyntax `tactic) := do
  let writtenLit := Syntax.mkNumLit (toString written)
  let exported ← `(term|
    { globals := ($(n "final"):ident).globals
      globalLoans := ($(n "final"):ident).globalLoans
      nextLoan := ($(n "final"):ident).nextLoan
      pending := ($(n "initial"):ident).pending.push
        ($(n "loanVar"):ident, LeanerIR.RuntimeValue.integer $writtenLit) })
  `(tactic|
    (rename_i $(n "rowOutcome"):ident $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finishControl?,
       LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
       Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
     subst $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
     rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowUnit
       _ _ _ _ _ ?noGlobal]
     case noGlobal =>
       rw [$(n "stableKey"):ident]
       exact $(n "keyFree"):ident
     simp only [$(caller.rawContract):term, $(caller.resultsCodec):term]
     have $(n "rowDiscipline"):ident := $(← forwardDiscipline exported)
     leaner_certified_close!))

/-- The complete script for a writing caller. -/
def setScript (caller : CallerNames) (plan : SetPlan) :
    CommandElabM (TSyntax `tactic) := do
  let callee := plan.callee
  let fuelLit := Syntax.mkNatLit (plan.fuel - 1)
  let exit ← writtenExit caller plan.written
  /- After the write: the settling marker and, for the reading shape, the
  read of the parameter; for the wrapping shape, the marker's exit row over
  the block's unit. -/
  let afterWrite ← match plan.shape with
    | .thenRead => `(tactic|
        (apply LeanerIR.Proofs.Denotation.wpRowThrow_blockResult
         apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_cons
         apply LeanerIR.Proofs.Denotation.wpRowThrow_endLoanStatement
         intro $(n "exitFrame"):ident $(n "exitState"):ident
           $(n "retired"):ident $(n "exited"):ident
         rw [LeanerIR.Proofs.Denotation.endLoan_evaluate_returnedReborrow_nil
           _ _ _ _ _ (by omega)] at $(n "exited"):ident
         injection $(n "exited"):ident with $(n "exitInner"):ident
         injection $(n "exitInner"):ident with $(n "exitFrameEq"):ident
           $(n "exitStateEq"):ident $(n "retiredEq"):ident
         subst $(n "exitFrame"):ident $(n "exitState"):ident $(n "retired"):ident
         refine ⟨_, _, rfl, ?_⟩
         apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_nil
         apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow
         case stable => leaner_row_stable
         leaner_row_drive
         rename_i $(n "rowRead1"):ident $(n "rowReadEq1"):ident
         simp only [List.getElem?_toArray, List.getElem?_cons_zero,
           List.getElem?_cons_succ, Option.join_some, Option.some.injEq]
           at $(n "rowReadEq1"):ident
         subst $(n "rowReadEq1"):ident
         constructor
         case' right =>
           intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
             $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident
           simp only [LeanerIR.Proofs.Denotation.ReferenceLocationOperation.evaluate?,
             LeanerIR.Proofs.Denotation.liftPlaceEvaluator,
             LeanerIR.SemanticOperations.dereferenceBorrow?] at $(n "rowEq"):ident
           injection $(n "rowEq"):ident with $(n "rowInner"):ident
           injection $(n "rowInner"):ident
         case' left =>
           intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
             $(n "rowValueOut"):ident $(n "rowEq"):ident
           simp only [LeanerIR.Proofs.Denotation.ReferenceLocationOperation.evaluate?,
             LeanerIR.Proofs.Denotation.liftPlaceEvaluator,
             LeanerIR.SemanticOperations.dereferenceBorrow?] at $(n "rowEq"):ident
           injection $(n "rowEq"):ident with $(n "rowInner"):ident
           injection $(n "rowInner"):ident with $(n "rowFrameEq"):ident
             $(n "rowStateEq"):ident $(n "rowValueEq"):ident
           injection $(n "rowFrameEq"):ident with $(n "rowRowEq"):ident
             $(n "rowActiveEq"):ident $(n "rowLocationsEq"):ident
           subst $(n "rowRowEq"):ident $(n "rowStateEq"):ident $(n "rowValueEq"):ident
           leaner_row_drive
           $exit:tactic))
    | .throughForward => `(tactic|
        (intro $(n "exitFrame"):ident $(n "exitState"):ident
           $(n "retired"):ident $(n "exited"):ident
         rw [LeanerIR.Proofs.Denotation.endLoan_evaluate_returnedReborrow_value
           _ _ _ _ _ _ (by omega)] at $(n "exited"):ident
         injection $(n "exited"):ident with $(n "exitInner"):ident
         injection $(n "exitInner"):ident with $(n "exitFrameEq"):ident
           $(n "exitStateEq"):ident $(n "retiredEq"):ident
         subst $(n "exitFrame"):ident $(n "exitState"):ident $(n "retired"):ident
         refine ⟨_, _, rfl, ?_⟩
         intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
         $exit:tactic))
  let blockBridge ← match plan.shape with
    | .thenRead => `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_blockResult)
    | .throughForward => `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_blockUnit)
  let write ← seq (← returnedWrite)
  let bound ← `(tactic|
    (intro $(n "boundFrame"):ident $(n "boundEq"):ident
     rw [LeanerIR.Proofs.Denotation.bindVariable_rowFrame $fuelLit ⟨1⟩ _ _ _
       (by simp)] at $(n "boundEq"):ident
     cases Option.some.inj $(n "boundEq"):ident
     refine ⟨_, rfl, ?_⟩
     rw [show (#[some (.borrow $(n "loanVar"):ident (.loanHole $(n "resultLoan"):ident)),
         none] : LeanerIR.Proofs.Denotation.Row).set! 1
         (some (.borrow $(n "resultLoan"):ident
           (.integer ($(n "rowResult"):ident).value.val))) =
         #[some (.borrow $(n "loanVar"):ident (.loanHole $(n "resultLoan"):ident)),
           some (.borrow $(n "resultLoan"):ident
             (.integer ($(n "rowResult"):ident).value.val))] from rfl]
     $blockBridge:tactic
     apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_cons
     apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow
     case stable => leaner_row_stable
     $write:tactic
     $afterWrite:tactic))
  let site ← returnedCallSite callee bound
  let renames ← bindVocabulary (borrowVocabulary (caller.parameters.getD 0 "slot"))
  let opening ← match plan.shape with
    | .thenRead => `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_letNativeValueThrow)
    | .throughForward => `(tactic|
        (apply LeanerIR.Proofs.Denotation.wpRowThrow_endLoanOver
         apply LeanerIR.Proofs.Denotation.wpRowThrow_letNativeValueThrow))
  let calleeFact ← calleeSatFact caller callee (generic := false)
  `(tactic|
    ($calleeFact:tactic
     $renames:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.mutable,
       LeanerIR.SemanticOperations.parameterLoanLocations_singleBorrow]
     rw [show LeanerIR.SemanticOperations.initialLocals 2
         #[.borrow $(n "loanVar"):ident (.integer $(n "valVar"):ident)] =
         #[some (.borrow $(n "loanVar"):ident (.integer $(n "valVar"):ident)), none] by
         simp [LeanerIR.SemanticOperations.initialLocals]; rfl]
     $opening:tactic
     apply LeanerIR.Proofs.Denotation.wpRowThrow_callReturnedReborrow (rest := [none])
     $site:tactic))

/-! ## Writing through a projected returned reborrow

`set_projected` binds the reborrow a callee projects from its struct
parameter, writes through it, and retires it under a death marker
wrapping the binding: the call law with the callee's focus, the write and
the marker beside the focused hole, and the export of the focused struct.
The callee's summary names the siblings existentially; its clauses,
normalized to arithmetic over the fields, are what pin them. -/

structure ProjectedSetPlan where
  callee : CalleeNames
  /-- The binding's fuel. -/
  fuel : Nat
  /-- The literal written through the returned reference. -/
  written : Nat
  /-- The parameter's twin, its struct handle, and its field count. -/
  twin : Name
  namespaceIndex : Nat
  structIndex : Nat
  fieldCount : Nat
  /-- The field the callee's result projects. -/
  focus : Nat

/-- The callee handle of `nativeCall handle …`, as namespace and function
indices. -/
private def callHandle? (e : Lean.Expr) : Option (Nat × Nat) := do
  guard (e.isAppOfArity ``nativeCall 4)
  let handle := e.getArg! 0
  guard (handle.isAppOfArity ``LeanerIR.FunctionHandle.mk 2)
  let namespaceIndex ← natLitOf? ((handle.getArg! 0).getArg! 0)
  let functionIndex ← natLitOf? ((handle.getArg! 1).getArg! 0)
  pure (namespaceIndex, functionIndex)

/-- Recognize the wrapping writing shape whose parameter borrows a struct
of integers and whose callee returns a reborrow of one of its fields. -/
def projectedSetPlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (parameterTwin? : Option (Name × Nat × Nat × Nat))
    (calleeFocus? : Nat × Nat → Option Nat) : Option ProjectedSetPlan := do
  guard (parameterCount == 1 && localCount == 2 && resultCount == 0)
  let (twin, namespaceIndex, structIndex, fieldCount) ← parameterTwin?
  let tree := body.bindingBody!
  guard (tree.isAppOfArity ``nativeReferenceOperation 2)
  guard ((tree.getArg! 0).isAppOfArity ``ReferenceLocationOperation.endLoan 1)
  let bound ← singleOperand? (tree.getArg! 1)
  let (fuel, callee, _, rest) ← boundCall? bound
  let focus ← calleeFocus? (← callHandle? (bound.getArg! 1))
  guard (focus < fieldCount)
  guard (rest.isAppOfArity ``blockUnit 1)
  let written ← writeLocal1? (rest.getArg! 0)
  pure { callee, fuel, written, twin, namespaceIndex, structIndex, fieldCount, focus }

/-- The complete script for a caller writing through a projected returned
reborrow. -/
def projectedSetScript (caller : CallerNames) (plan : ProjectedSetPlan) :
    CommandElabM (TSyntax `tactic) := do
  let callee := plan.callee
  let parameter := caller.parameters.getD 0 "slot"
  let renames ← bindVocabulary #[(s!"{parameter}_loan", "loanVar"),
    (s!"{parameter}_loan_keyFree", "keyFree"), (s!"{parameter}_loan_bound", "loanBound")]
  let fieldIdent (index : Nat) : Ident := mkIdent (Name.mkSimple s!"field{index}")
  let siblingIdent (index : Nat) : Ident := mkIdent (Name.mkSimple s!"sibling{index}")
  let indices := Array.range plan.fieldCount
  let fieldBinders ← indices.mapM fun index => `(Lean.binderIdent| $(fieldIdent index):ident)
  let entryFields ← indices.mapM fun index =>
    `(term| LeanerIR.RuntimeValue.integer $(fieldIdent index):ident)
  let typedFields ← indices.mapM fun index =>
    `(term| ⟨$(fieldIdent index):ident, by assumption⟩)
  let siblings := indices.filter (· != plan.focus)
  /- The transfer names each sibling existentially, its equation before
  its range: `∃ s₀, (∃ s₁, eq ∧ range₁) ∧ range₀`. -/
  let transferPattern ← siblings.foldrM (init := ← `(rcasesPat| $(n "transferEq"):ident))
    fun index inner => `(rcasesPat| ⟨$(siblingIdent index):ident, $inner:rcasesPat, -⟩)
  let siblingValues ← siblings.mapM fun index =>
    `(term| LeanerIR.RuntimeValue.integer $(siblingIdent index):ident)
  let before := siblingValues.extract 0 plan.focus
  let after := siblingValues.extract plan.focus siblingValues.size
  let nsLit := Syntax.mkNatLit plan.namespaceIndex
  let structLit := Syntax.mkNatLit plan.structIndex
  let handle ← `(term| (⟨⟨$nsLit⟩, $structLit⟩ : LeanerIR.StructHandle))
  let entryValue ← `(term| LeanerIR.RuntimeValue.nominal $handle none #[$entryFields,*])
  let steps ← `(term| ([⟨$handle, #[$before,*], #[$after,*]⟩] :
    List LeanerIR.SemanticOperations.FocusStep))
  let lent ← `(term| LeanerIR.SemanticOperations.focusValue $steps
    (.loanHole $(n "resultLoan"):ident))
  let writtenLit := Syntax.mkNumLit (toString plan.written)
  let fuelLit := Syntax.mkNatLit (plan.fuel - 1)
  let codecName := mkIdent (plan.twin ++ `codec)
  let eraseName := mkIdent (plan.twin ++ `erase)
  let calleeContract := mkIdent callee.contract
  let calleeTyped := mkIdent callee.typedContract
  let calleeRaw := mkIdent callee.rawContract
  let calleeCodec := mkIdent callee.argumentsCodec
  let calleeResults := mkIdent callee.resultsCodec
  let exported ← `(term|
    { globals := ($(n "final"):ident).globals
      globalLoans := ($(n "final"):ident).globalLoans
      nextLoan := ($(n "final"):ident).nextLoan
      pending := ($(n "initial"):ident).pending.push
        ($(n "loanVar"):ident, LeanerIR.SemanticOperations.focusValue $steps
          (.integer $writtenLit)) })
  let calleeFact ← calleeSatFact caller callee (generic := false)
  `(tactic|
    ($calleeFact:tactic
     rename_i $fieldBinders*
     $renames:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.mutable, $codecName:ident, $eraseName:ident,
       LeanerIR.SemanticOperations.parameterLoanLocations_singleBorrow]
     rw [show LeanerIR.SemanticOperations.initialLocals 2
         #[.borrow $(n "loanVar"):ident $entryValue] =
         #[some (.borrow $(n "loanVar"):ident $entryValue), none]
       by simp [LeanerIR.SemanticOperations.initialLocals]; rfl]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_endLoanOver
     apply LeanerIR.Proofs.Denotation.wpRowThrow_letNativeValueThrow
     apply LeanerIR.Proofs.Denotation.wpRowThrow_callReturnedProjected (rest := [none])
     case mutableKind => rfl
     case priorLoan => exact $(n "loanBound"):ident
     case calleeWp =>
       apply LeanerIR.Proofs.Denotation.wpFunction_of_satisfies
         $(calleeSatIdent callee)
       case permitted =>
         simp only [$calleeContract:term,
           LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
           LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
           $calleeCodec:term, LeanerIR.Proofs.Codec.mutable,
           $codecName:ident, $eraseName:ident]
         refine ⟨⟨⟨($(n "initial"):ident).nextLoan, ⟨$typedFields,*⟩⟩⟩, rfl,
           ($(n "initial"):ident).nextLoan, _, ⟨⟨rfl, ?_⟩, by omega⟩, ?_⟩
         · intro $(n "rowLoan"):ident $(n "rowBound"):ident
           have $(n "rowAdvanced"):ident :
               ($(n "initial"):ident).nextLoan + 1 ≤ $(n "rowLoan"):ident :=
             $(n "rowBound"):ident
           exact $(n "freshLoans"):ident $(n "rowLoan"):ident (by omega)
         · simp only [LeanerIR.SemanticOperations.globalLoanKey?]
           exact $(n "freshLoans"):ident ($(n "initial"):ident).nextLoan (by omega)
       case onReturn =>
         intro $(n "results"):ident $(n "final"):ident $(n "ensuresFn"):ident
           $(n "frameFn"):ident $(n "notMust"):ident
         have $(n "ensures"):ident := $(n "ensuresFn"):ident
           (by exact $(n "notMust"):ident)
         simp only [$calleeContract:term,
           LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
           LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
           at $(n "ensures"):ident $(n "frameFn"):ident
         obtain ⟨$(n "rowArgs"):ident, $(n "rowResult"):ident,
           $(n "rowArgsEq"):ident, $(n "rowDecodeEq"):ident,
           $(n "slotLoan"):ident, $(n "slotEntry"):ident, $(n "slot"):ident,
           $(n "slotPending"):ident, $(n "resultLoan"):ident, $(n "result"):ident,
           ⟨⟨⟨$(n "argsEq2"):ident, $(n "resultsEq"):ident⟩,
             ⟨⟨⟨⟨$(n "pendingEq"):ident, $(n "resolveEq"):ident⟩,
               $transferPattern:rcasesPat⟩,
               $(n "freshLo"):ident⟩, $(n "freshHi"):ident⟩⟩,
             ⟨$(n "resNonNeg"):ident, $(n "resMax"):ident⟩⟩,
           $(n "obligation"):ident⟩ := $(n "ensures"):ident
         simp only [$calleeResults:term] at $(n "rowDecodeEq"):ident $(n "resultsEq"):ident
         split at $(n "rowDecodeEq"):ident
         · rename_i $(n "r0"):ident $(n "listEq"):ident
           simp only [Option.bind_eq_some_iff, Option.some.injEq, exists_eq_right]
             at $(n "rowDecodeEq"):ident
           obtain ⟨$(n "current"):ident, $(n "r0Eq"):ident, $(n "currentEq"):ident⟩ :=
             LeanerIR.Proofs.Codec.mutable_decode?_eq_some $(n "rowDecodeEq"):ident
           have $(n "currentInt"):ident :=
             LeanerIR.Proofs.Codec.specInt_decode?_eq_some $(n "currentEq"):ident
           subst $(n "currentInt"):ident $(n "r0Eq"):ident
           have $(n "resultsShape"):ident : $(n "results"):ident =
               #[.borrow ($(n "rowResult"):ident).loan
                 (.integer ($(n "rowResult"):ident).value.val)] := by
             have $(n "listArray"):ident := congrArg List.toArray $(n "listEq"):ident
             simpa using $(n "listArray"):ident
           subst $(n "resultsShape"):ident
           simp only [LeanerIR.Proofs.Codec.mutable, LeanerIR.Proofs.Codec.specInt,
             Array.mk.injEq, List.cons.injEq, LeanerIR.RuntimeValue.borrow.injEq,
             LeanerIR.RuntimeValue.integer.injEq, and_true] at $(n "resultsEq"):ident
           obtain ⟨$(n "loanEq"):ident, $(n "valueEq"):ident⟩ := $(n "resultsEq"):ident
           rw [$(n "rowArgsEq"):ident] at $(n "argsEq2"):ident
           simp only [Array.mk.injEq, List.cons.injEq, LeanerIR.RuntimeValue.borrow.injEq,
             and_true] at $(n "argsEq2"):ident
           obtain ⟨$(n "slotLoanEq"):ident, $(n "slotEntryEq"):ident⟩ := $(n "argsEq2"):ident
           subst $(n "slotLoanEq"):ident $(n "slotEntryEq"):ident
           subst $(n "transferEq"):ident
           rw [LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedBorrow_focus _ _ _
               (by leaner_plain)] at $(n "resolveEq"):ident
           subst $(n "resolveEq"):ident
           have $(n "plainSteps"):ident : LeanerIR.SemanticOperations.PlainSteps $steps := by
             leaner_plain
           obtain ⟨$(n "rowArgs2"):ident, $(n "rowArgsEq3"):ident,
             $(n "globalsEq"):ident, $(n "discipline"):ident⟩ := $(n "frameFn"):ident
           have $(n "freshLoLoan"):ident :
               ($(n "initial"):ident).nextLoan + 1 ≤ $(n "resultLoan"):ident :=
             $(n "freshLo"):ident
           have $(n "freshHiLoan"):ident :
               $(n "resultLoan"):ident < ($(n "final"):ident).nextLoan :=
             $(n "freshHi"):ident
           have $(n "monotone"):ident :
               ($(n "initial"):ident).nextLoan + 1 ≤ ($(n "final"):ident).nextLoan :=
             ($(n "discipline"):ident).2.2
           have $(n "stableKey"):ident :=
             ($(n "discipline"):ident).2.1 $(n "loanVar"):ident (by
               show $(n "loanVar"):ident < ($(n "initial"):ident).nextLoan + 1
               omega)
           have $(n "resultFits"):ident := ($(n "rowResult"):ident).value.fits
           refine ⟨$(n "resultLoan"):ident, ($(n "rowResult"):ident).value.val, _,
             $(n "plainSteps"):ident, by rw [$(n "loanEq"):ident], $(n "pendingEq"):ident, ?_⟩
           intro $(n "boundFrame"):ident $(n "boundEq"):ident
           rw [LeanerIR.Proofs.Denotation.bindVariable_rowFrame $fuelLit ⟨1⟩ _ _ _ (by simp)]
             at $(n "boundEq"):ident
           cases Option.some.inj $(n "boundEq"):ident
           refine ⟨_, rfl, ?_⟩
           rw [show (#[some (.borrow $(n "loanVar"):ident $lent), none] :
                 LeanerIR.Proofs.Denotation.Row).set! 1
                 (some (.borrow $(n "resultLoan"):ident
                   (.integer ($(n "rowResult"):ident).value.val))) =
               #[some (.borrow $(n "loanVar"):ident $lent),
                 some (.borrow $(n "resultLoan"):ident
                   (.integer ($(n "rowResult"):ident).value.val))] from rfl]
           apply LeanerIR.Proofs.Denotation.wpRowThrow_blockUnit
           apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_cons
           apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow
           case stable => leaner_row_stable
           leaner_row_drive
           rename_i $(n "rowRead0"):ident $(n "rowReadEq0"):ident
           simp only [List.getElem?_toArray, List.getElem?_cons_zero,
             List.getElem?_cons_succ, Option.join_some, Option.some.injEq]
             at $(n "rowReadEq0"):ident
           subst $(n "rowReadEq0"):ident
           constructor
           case' right =>
             (intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
                $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident
              rw [LeanerIR.Proofs.Denotation.mutate_evaluate_returnedProjectedLocal1
                _ _ _ _ $(n "plainSteps"):ident _ _ _ (by omega)] at $(n "rowEq"):ident
              injection $(n "rowEq"):ident with $(n "rowInner"):ident
              injection $(n "rowInner"):ident)
           case' left =>
             (intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
                $(n "rowValueOut"):ident $(n "rowEq"):ident
              rw [LeanerIR.Proofs.Denotation.mutate_evaluate_returnedProjectedLocal1
                _ _ _ _ $(n "plainSteps"):ident _ _ _ (by omega)] at $(n "rowEq"):ident
              injection $(n "rowEq"):ident with $(n "rowInner"):ident
              injection $(n "rowInner"):ident with $(n "rowFrameEq"):ident
                $(n "rowStateEq"):ident $(n "rowValueEq"):ident
              injection $(n "rowFrameEq"):ident with $(n "rowRowEq"):ident
                $(n "rowActiveEq"):ident $(n "rowLocationsEq"):ident
              subst $(n "rowRowEq"):ident $(n "rowStateEq"):ident $(n "rowValueEq"):ident
              leaner_row_drive
              rename_i $(n "nilResult"):ident $(n "nilStep"):ident
              simp only [LeanerIR.Proofs.Denotation.statementsNil] at $(n "nilStep"):ident
              subst $(n "nilStep"):ident
              refine ⟨_, _, rfl, ?_⟩)
           intro $(n "exitFrame"):ident $(n "exitState"):ident
             $(n "retired"):ident $(n "exited"):ident
           rw [LeanerIR.Proofs.Denotation.endLoan_evaluate_returnedProjected_value
             _ _ _ _ $(n "plainSteps"):ident _ _ _ (by omega)] at $(n "exited"):ident
           injection $(n "exited"):ident with $(n "exitInner"):ident
           injection $(n "exitInner"):ident with $(n "exitFrameEq"):ident
             $(n "exitStateEq"):ident $(n "retiredEq"):ident
           subst $(n "exitFrame"):ident $(n "exitState"):ident $(n "retired"):ident
           refine ⟨_, _, rfl, ?_⟩
           intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
           simp only [LeanerIR.SemanticOperations.finishControl?,
             LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
             Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
           subst $(n "rowFinished"):ident
           simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
           rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_focusedUnit
             _ _ _ $(n "plainSteps"):ident _ _ _ ?noGlobal]
           case noGlobal =>
             rw [$(n "stableKey"):ident]
             exact $(n "keyFree"):ident
           simp only [$(caller.rawContract):term, $(caller.resultsCodec):term]
           have $(n "rowDiscipline"):ident := $(← forwardDiscipline exported)
           /- The callee's clauses, as arithmetic over the fields: they pin
           the siblings its summary named existentially. -/
           simp only [LeanerIR.Proofs.Obligation_iff,
             LeanerIR.SemanticOperations.focusValue_cons,
             LeanerIR.SemanticOperations.focusValue_nil,
             LeanerIR.SemanticOperations.FocusStep.fill,
             List.push_toArray, List.append_toArray, List.cons_append, List.nil_append,
             LeanerIR.RuntimeValue.field, LeanerIR.RuntimeValue.asInt, List.getElem?_toArray,
             List.getElem?_cons_succ, List.getElem?_cons_zero, Option.getD_some]
             at $(n "obligation"):ident
           leaner_cases $(n "obligation"):ident
           leaner_certified_close!
         · exact absurd $(n "rowDecodeEq"):ident (by simp)
       case onThrow =>
         intro $(n "rowKind"):ident $(n "rowThrown"):ident $(n "final"):ident
           $(n "aborts"):ident
         intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
           $(n "rowOutcome"):ident $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finishControl?,
           Option.some.injEq] at $(n "rowFinished"):ident
         subst $(n "rowFinished"):ident
         simp only [$calleeContract:term,
           LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
           LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
           at $(n "aborts"):ident
         obtain ⟨$(n "rowArgs"):ident, $(n "rowArgsEq"):ident,
           $(n "rowSlotLoan"):ident, $(n "rowSlot"):ident,
           $(n "rowArgsEq2"):ident, $(n "rowObligation"):ident⟩ :=
           $(n "aborts"):ident
         simp only [LeanerIR.Proofs.Obligation_iff] at $(n "rowObligation"):ident))

/-! ## A returned reborrow projected from a global borrow

`&mut Pair[address].left`: the resource is borrowed into a local and a
reborrow of one of its fields is returned.  The global borrow is the
`let`'s initializer, the reborrow is the path law at the holder, and the
exit writes the resource back with the returned loan's hole at the focus,
its key transferring to the returned loan. -/

structure GlobalReturnPlan where
  twin : StoreTwin
  /-- The binding's fuel. -/
  fuel : Nat

/-- Recognize `let holder := &mut R[local0] in reborrow holder.[f*]` as the
whole body of a one-parameter, one-result function. -/
def globalReturnPlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (twinOf : Nat → Nat → Option (Lean.Name × Nat))
    (twinInfoOf : Nat → Nat → Option (Name × Array (String × SpecTypes.FieldRep))) :
    Option GlobalReturnPlan := do
  guard (parameterCount == 1 && localCount == 2 && resultCount == 1)
  let outerLet := body.bindingBody!
  guard !outerLet.hasLooseBVars
  guard (outerLet.isAppOfArity ``letNativeValue 3)
  let binder := outerLet.getArg! 0
  guard (binder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
  let fuel ← natLitOf? (binder.getArg! 0)
  guard (fuel > 0)
  guard (natLeaf? (binder.getArg! 1) == some 1)
  let borrow := outerLet.getArg! 1
  guard (borrow.isAppOfArity ``nativeGlobalOperation 2)
  let site := borrow.getArg! 0
  guard (site.isAppOfArity ``GlobalLocationOperation.borrow 1)
  let siteRecord := site.getArg! 0
  guard (siteRecord.isAppOfArity ``BorrowLocation.mk 4)
  let resource := siteRecord.getArg! 0
  guard (resource.isAppOfArity ``ResourceLocation.mk 2)
  let namespaceIndex ← natLitOf? ((resource.getArg! 0).getArg! 0)
  let typeIndex ← natLitOf? ((resource.getArg! 1).getArg! 0)
  guard ((siteRecord.getArg! 2).isConstOf ``BorrowKind.mutable)
  guard (natLitOf? (siteRecord.getArg! 3) == some 0)
  let operands := borrow.getArg! 1
  guard (operands.isAppOfArity ``valuesCons 2)
  guard (isLocalRead (operands.getArg! 0) 0)
  guard ((operands.getArg! 1).isConstOf ``valuesNil)
  let reborrow := outerLet.getArg! 2
  guard (reborrow.isAppOfArity ``nativeDerefLocalBorrowOperation 2)
  guard ((reborrow.getArg! 1).isConstOf ``valuesNil)
  let record := reborrow.getArg! 0
  guard (record.isAppOfArity ``DerefLocalBorrowOperation.mk 5)
  guard (natLeaf? (record.getArg! 0) == some 1)
  guard ((record.getArg! 3).isConstOf ``BorrowKind.mutable)
  guard (natLitOf? (record.getArg! 4) == some 1)
  let raw ← fieldSteps? (record.getArg! 1)
  guard (raw.length == 1)
  let (twinName, familyStruct) ← twinOf namespaceIndex typeIndex
  guard (raw.head?.map (·.1) == some familyStruct)
  let path ← pathSteps? namespaceIndex raw twinInfoOf
  pure { twin := { twin := twinName, namespaceIndex, typeIndex, path }, fuel }

/-- The complete script for a returned reborrow projected from a global
borrow, from the goal `wp_nativeFunction` leaves. -/
def globalReturnScript (caller : CallerNames) (plan : GlobalReturnPlan) :
    CommandElabM (TSyntax `tactic) := do
  let twin := plan.twin
  let nsLit := Syntax.mkNatLit twin.namespaceIndex
  let tyLit := Syntax.mkNatLit twin.typeIndex
  let steps ← stepsTerm twin.namespaceIndex twin.path
  let family := twin.twin.getString!
  let address := caller.parameters.getD 0 "addr"
  let pattern ← resourcePattern twin.path
  let leafPresent ← `(term| .integer $(n "fieldVal"):ident)
  let present ← `(term| LeanerIR.SemanticOperations.focusValue $steps $leafPresent)
  let entryValue ← nominalTerm twin.namespaceIndex twin.path leafPresent
  let entryValue ← match twin.path with
    | [step] =>
        let erased ← step.fields.mapIdxM fun index (_, rep) =>
          if index == step.focus then pure leafPresent
          else rep.eraseSyntax (siblingIdent 0 index)
        `(term| (.nominal ⟨⟨$nsLit⟩, $(Syntax.mkNatLit step.structIndex)⟩ none #[$erased,*] :
          LeanerIR.RuntimeValue))
    | _ => pure entryValue
  let hole ← `(term| .loanHole (($(n "initial"):ident).nextLoan + 1))
  let lent ← `(term| LeanerIR.SemanticOperations.focusValue $steps $hole)
  let fuelLit := Syntax.mkNatLit (plan.fuel - 1)
  let leafTyped ← `(term| ⟨$(n "fieldVal"):ident, $(n "fieldFits"):ident⟩)
  let typed ← typedTerm twin.path leafTyped
  let eraseName := mkIdent (twin.twin ++ `erase)
  let key ← `(term| (⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩ : LeanerIR.GlobalKey))
  let getName := mkIdent (twin.twin ++ `get)
  let readName := mkIdent (twin.twin ++ `read)
  let keyName := mkIdent (twin.twin ++ `key)
  let renames ← bindVocabulary #[
    (s!"{family}_contents", "contents"), (s!"{family}_represented", "represented"),
    (address, "addr"), ("requires_0", "present")]
  `(tactic|
    ($renames:tactic
     have $(n "lookupEq"):ident := $(n "represented"):ident (.address $(n "addr"):ident)
     rw [$(n "lookupEq"):ident] at $(n "present"):ident
     obtain ⟨$(n "resource"):ident, $(n "resourceEq"):ident⟩ :=
       Option.isSome_iff_exists.mp (by simpa using $(n "present"):ident)
     obtain $pattern:rcasesPat := $(n "resource"):ident
     have $(n "presentValue"):ident : ($(n "initial"):ident).globals.lookup
         (LeanerIR.SemanticOperations.globalKey ⟨$nsLit⟩ ⟨$tyLit⟩
           (.address $(n "addr"):ident)) =
         some $present := by
       rw [show LeanerIR.SemanticOperations.globalKey ⟨$nsLit⟩ ⟨$tyLit⟩
           (.address $(n "addr"):ident) =
           ⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩ from rfl,
         $(n "lookupEq"):ident, $(n "resourceEq"):ident]
       rfl
     have $(n "plainSteps"):ident : LeanerIR.SemanticOperations.PlainSteps $steps := by
       leaner_plain
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.address]
     rw [show LeanerIR.SemanticOperations.initialLocals 2 #[.address $(n "addr"):ident] =
         #[some (.address $(n "addr"):ident), none] by
         simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
       show LeanerIR.SemanticOperations.parameterLoanLocations
         #[.address $(n "addr"):ident] = #[] by
         simp [LeanerIR.SemanticOperations.parameterLoanLocations]]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_letNativeValueThrow
     apply LeanerIR.Proofs.Denotation.wpRowThrow_globalBorrowLocal0 (rest := [none])
     case borrowMutable => rfl
     case present => exact $(n "presentValue"):ident
     intro $(n "boundFrame"):ident $(n "boundEq"):ident
     rw [LeanerIR.Proofs.Denotation.bindVariable_rowFrame $fuelLit ⟨1⟩ _ _ _ (by simp)]
       at $(n "boundEq"):ident
     cases Option.some.inj $(n "boundEq"):ident
     refine ⟨_, rfl, ?_⟩
     rw [show (#[some (.address $(n "addr"):ident), none] :
           LeanerIR.Proofs.Denotation.Row).set! 1
           (some (.borrow ($(n "initial"):ident).nextLoan $present)) =
         #[some (.address $(n "addr"):ident),
           some (.borrow ($(n "initial"):ident).nextLoan $entryValue)] from rfl]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_returnedReborrowPath (localId := ⟨1⟩)
       (steps := $steps) (leaf := $leafPresent)
     case mutableKind => rfl
     case slot => rfl
     intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finishControl?,
       LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
       Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
     subst $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
     rw [show (#[some (.address $(n "addr"):ident),
           some (.borrow ($(n "initial"):ident).nextLoan $entryValue)] :
           LeanerIR.Proofs.Denotation.Row).set! 1
           (some (.borrow ($(n "initial"):ident).nextLoan $lent)) =
         #[some (.address $(n "addr"):ident),
           some (.borrow ($(n "initial"):ident).nextLoan $lent)] from rfl]
     rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_returnedGlobal
       $(n "plainSteps"):ident (by omega) ($(n "freshLoans"):ident _ (Nat.le_refl _))]
     /- A clause reads the final map through the export's resolution: the
     resource back at its key over the two intermediate values, which the
     typed contents represent. -/
     have $(n "represented'"):ident :
         LeanerIR.FamilyRepresentation $eraseName:ident ⟨$nsLit⟩ ⟨$tyLit⟩
           (LeanerIR.updateContents $(n "contents"):ident (.address $(n "addr"):ident)
             (some $typed))
           (((($(n "initial"):ident).globals.insert $key
               (.loanHole ($(n "initial"):ident).nextLoan)).insert $key $lent).insert $key
             (LeanerIR.SemanticOperations.focusValue $steps $leafPresent)) :=
       LeanerIR.FamilyRepresentation.insert_over_two (erase := $eraseName:ident)
         $(n "represented"):ident (.address $(n "addr"):ident) _ _ $typed
     simp only [$(caller.rawContract):term, $(caller.resultsCodec):term,
       $getName:ident, $readName:ident, $keyName:ident]
     leaner_certified_close!))

/-! ## Writing through a returned global reborrow

`set_global_left` binds the reborrow a callee returns into storage, writes
through it, and retires it under a death marker wrapping the binding.
The callee's summary states its storage transfer and the registry; the
call registers the returned loan under the caller's lexical loan, the
write finds the borrow by scanning, the marker fills the hole in storage
and retires the key, and the exit exports nothing.  The typed resource
around the write is rebuilt from the callee's frame clause, and the
siblings the callee named existentially are pinned by refuting their
difference from the entry's through the callee's clauses. -/

structure GlobalSetPlan where
  callee : CalleeNames
  /-- The binding's fuel. -/
  fuel : Nat
  /-- The literal written through the returned reference. -/
  written : Nat
  twin : StoreTwin
  /-- The bit width of every field of the resource, all integers. -/
  widths : Array Nat

/-- Recognize `let r := f(local0) in body`, a call carrying its lexical
loan, and hand back the binding's fuel, the callee, its handle, and the
body. -/
private def boundGlobalCall? (e : Lean.Expr) :
    Option (Nat × CalleeNames × Nat × Nat × Lean.Expr) := do
  guard (e.isAppOfArity ``letNativeValue 3)
  let binder := e.getArg! 0
  guard (binder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
  let fuel ← natLitOf? (binder.getArg! 0)
  guard (fuel > 0)
  guard (natLeaf? (binder.getArg! 1) == some 1)
  let call := e.getArg! 1
  guard (call.isAppOfArity ``nativeCall 4)
  let lexical := call.getArg! 1
  guard (lexical.isAppOfArity ``Option.some 2)
  guard (natLitOf? (lexical.getArg! 1) == some 0)
  let callee := call.getArg! 2
  let relation ← calleeRelation? callee
  let operands := call.getArg! 3
  guard (operands.isAppOfArity ``valuesCons 2)
  guard (isLocalRead (operands.getArg! 0) 0)
  guard ((operands.getArg! 1).isConstOf ``valuesNil)
  let names ← calleeNames? relation
  let (namespaceIndex, functionIndex) ← callHandle? call
  pure (fuel, names, namespaceIndex, functionIndex, e.getArg! 2)

/-- Recognize the wrapping writing shape whose callee returns a reborrow
into storage. -/
def globalSetPlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (calleeGlobalLender? : Nat × Nat → Option (Nat × Nat × Nat × Nat))
    (twinOf : Nat → Nat → Option (Lean.Name × Nat))
    (twinInfoOf : Nat → Nat → Option (Name × Array (String × SpecTypes.FieldRep))) :
    Option GlobalSetPlan := do
  guard (parameterCount == 1 && localCount == 2 && resultCount == 0)
  let tree := body.bindingBody!
  guard (tree.isAppOfArity ``nativeReferenceOperation 2)
  guard ((tree.getArg! 0).isAppOfArity ``ReferenceLocationOperation.endLoan 1)
  let bound ← singleOperand? (tree.getArg! 1)
  let (fuel, callee, namespaceIndex, functionIndex, rest) ← boundGlobalCall? bound
  let (familyNamespace, typeIndex, structIndex, focus) ←
    calleeGlobalLender? (namespaceIndex, functionIndex)
  let (twinName, familyStruct) ← twinOf familyNamespace typeIndex
  guard (familyStruct == structIndex)
  let path ← pathSteps? familyNamespace [(structIndex, focus)] twinInfoOf
  let [step] := path | none
  let widths ← step.fields.mapM fun (_, rep) => match rep with
    | .int (.bits width) false => some width
    | _ => none
  guard (rest.isAppOfArity ``blockUnit 1)
  let written ← writeLocal1? (rest.getArg! 0)
  pure { callee, fuel, written, widths
         twin := { twin := twinName, namespaceIndex := familyNamespace, typeIndex, path } }

/-- The complete script for a caller writing through a returned global
reborrow. -/
def globalSetScript (caller : CallerNames) (plan : GlobalSetPlan) :
    CommandElabM (TSyntax `tactic) := do
  let callee := plan.callee
  let twin := plan.twin
  let some step := twin.path.head? | throwError "the global writing plan has no path"
  let nsLit := Syntax.mkNatLit twin.namespaceIndex
  let tyLit := Syntax.mkNatLit twin.typeIndex
  let structLit := Syntax.mkNatLit step.structIndex
  let family := twin.twin.getString!
  let address := caller.parameters.getD 0 "addr"
  let pattern ← resourcePattern twin.path
  let handle ← `(term| (⟨⟨$nsLit⟩, $structLit⟩ : LeanerIR.StructHandle))
  let key ← `(term| (⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩ : LeanerIR.GlobalKey))
  let indices := Array.range step.fields.size
  let siblings := indices.filter (· != step.focus)
  let siblingIdent' (index : Nat) : Ident := mkIdent (Name.mkSimple s!"sibling{index}")
  let siblingFitsIdent (index : Nat) : Ident := mkIdent (Name.mkSimple s!"siblingFits{index}")
  let siblingEqIdent (index : Nat) : Ident := mkIdent (Name.mkSimple s!"siblingEq{index}")
  let siblingRangeIdent (index : Nat) : Ident := mkIdent (Name.mkSimple s!"siblingRange{index}")
  let siblingValues ← siblings.mapM fun index =>
    `(term| LeanerIR.RuntimeValue.integer $(siblingIdent' index):ident)
  let before := siblingValues.extract 0 step.focus
  let after := siblingValues.extract step.focus siblingValues.size
  let steps ← `(term| ([⟨$handle, #[$before,*], #[$after,*]⟩] :
    List LeanerIR.SemanticOperations.FocusStep))
  /- The transfer names each sibling existentially, its equation before
  its range: `∃ s₀, (∃ s₁, eq ∧ range₁) ∧ range₀`. -/
  let transferPattern ← siblings.foldrM (init := ← `(rcasesPat| $(n "transferEq"):ident))
    fun index inner => `(rcasesPat| ⟨$(siblingIdent' index):ident, $inner:rcasesPat,
      $(siblingRangeIdent index):ident⟩)
  let writtenLit := Syntax.mkNumLit (toString plan.written)
  let fuelLit := Syntax.mkNatLit (plan.fuel - 1)
  let focusWidth := Syntax.mkNatLit (plan.widths.getD step.focus 64)
  let typedOf (focusTerm : Term) : CommandElabM Term := do
    let fields ← indices.mapM fun index =>
      if index == step.focus then pure focusTerm
      else `(term| ⟨$(siblingIdent' index):ident, $(siblingFitsIdent index):ident⟩)
    `(term| ⟨$fields,*⟩)
  let typedCallee ← typedOf (← `(term| ($(n "rowResult"):ident).value))
  let typedWritten ← typedOf (← `(term| ⟨$writtenLit, $(n "fitsWritten"):ident⟩))
  let eraseName := mkIdent (twin.twin ++ `erase)
  let getName := mkIdent (twin.twin ++ `get)
  let readName := mkIdent (twin.twin ++ `read)
  let keyName := mkIdent (twin.twin ++ `key)
  let containsName := mkIdent (twin.twin ++ `contains)
  let calleeContract := mkIdent callee.contract
  let calleeTyped := mkIdent callee.typedContract
  let calleeRaw := mkIdent callee.rawContract
  let calleeCodec := mkIdent callee.argumentsCodec
  let calleeResults := mkIdent callee.resultsCodec
  let renames ← bindVocabulary #[
    (s!"{family}_contents", "contents"), (s!"{family}_represented", "represented"),
    (address, "addr"), ("requires_0", "present")]
  let siblingFacts ← siblings.mapM fun index => do
    let width := Syntax.mkNatLit (plan.widths.getD index 64)
    `(tactic|
      (have $(siblingFitsIdent index):ident :
           LeanerIR.IntegerValueFits (.bits $width) false $(siblingIdent' index):ident := by
         simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?,
           LeanerIR.Ty.integerBounds?]
         omega))
  /- The siblings the callee named are pinned by its clauses: their
  difference from the entry's is refuted, the clauses in context. -/
  let siblingPins ← siblings.mapM fun index => do
    `(tactic|
      (have $(siblingEqIdent index):ident :
           $(siblingIdent' index):ident = ($(siblingIdent 0 index):ident).val := by
         refine Classical.byContradiction fun $(n "distinct"):ident => ?_
         revert $(n "obligation"):ident
         leaner_certified_close!))
  let calleeFact ← calleeSatFact caller callee (generic := false)
  `(tactic|
    ($calleeFact:tactic
     $renames:tactic
     have $(n "lookupEq"):ident := $(n "represented"):ident (.address $(n "addr"):ident)
     rw [$(n "lookupEq"):ident] at $(n "present"):ident
     obtain ⟨$(n "resource"):ident, $(n "resourceEq"):ident⟩ :=
       Option.isSome_iff_exists.mp (by simpa using $(n "present"):ident)
     obtain $pattern:rcasesPat := $(n "resource"):ident
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.address]
     rw [show LeanerIR.SemanticOperations.initialLocals 2 #[.address $(n "addr"):ident] =
         #[some (.address $(n "addr"):ident), none] by
         simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
       show LeanerIR.SemanticOperations.parameterLoanLocations
         #[.address $(n "addr"):ident] = #[] by
         simp [LeanerIR.SemanticOperations.parameterLoanLocations]]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_endLoanOver
     apply LeanerIR.Proofs.Denotation.wpRowThrow_letNativeValueThrow
     apply LeanerIR.Proofs.Denotation.wpRowThrow_callReturnedGlobal (rest := [none])
     apply LeanerIR.Proofs.Denotation.wpFunction_of_satisfies $(calleeSatIdent callee)
     case permitted =>
       simp only [$calleeContract:term,
         LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
         LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
         $calleeCodec:term, LeanerIR.Proofs.Codec.address]
       refine ⟨⟨$(n "addr"):ident⟩, rfl, $(n "contents"):ident, $(n "addr"):ident,
         ⟨⟨$(n "represented"):ident, rfl⟩, $(n "freshLoans"):ident⟩, ?_⟩
       simp only [$containsName:ident, $keyName:ident, LeanerIR.RuntimeValue.storageKey,
         LeanerIR.RuntimeValue.storageKey?, Option.getD_some, $(n "lookupEq"):ident]
       exact $(n "present"):ident
     case onReturn =>
       intro $(n "results"):ident $(n "final"):ident $(n "ensuresFn"):ident
         $(n "frameFn"):ident $(n "notMust"):ident
       have $(n "ensures"):ident := $(n "ensuresFn"):ident
         (by exact $(n "notMust"):ident)
       simp only [$calleeContract:term,
         LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
         LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
         at $(n "ensures"):ident $(n "frameFn"):ident
       obtain ⟨$(n "rowArgs"):ident, $(n "rowResult"):ident,
         $(n "rowArgsEq"):ident, $(n "rowDecodeEq"):ident,
         $(n "address'"):ident, $(n "resultLoan"):ident, $(n "result"):ident,
         $(n "pending"):ident,
         ⟨⟨⟨⟨⟨⟨⟨⟨$(n "argsEq2"):ident, $(n "resultsEq"):ident⟩, $(n "pendingEq"):ident⟩,
           $(n "storedEq"):ident⟩, $transferPattern:rcasesPat⟩, $(n "registryEq"):ident⟩,
           $(n "freshLo"):ident⟩, $(n "freshHi"):ident⟩,
           ⟨$(n "resNonNeg"):ident, $(n "resMax"):ident⟩⟩,
         $(n "obligation"):ident⟩ := $(n "ensures"):ident
       simp only [$calleeResults:term] at $(n "rowDecodeEq"):ident $(n "resultsEq"):ident
       split at $(n "rowDecodeEq"):ident
       · rename_i $(n "r0"):ident $(n "listEq"):ident
         simp only [Option.bind_eq_some_iff, Option.some.injEq, exists_eq_right]
           at $(n "rowDecodeEq"):ident
         obtain ⟨$(n "current"):ident, $(n "r0Eq"):ident, $(n "currentEq"):ident⟩ :=
           LeanerIR.Proofs.Codec.mutable_decode?_eq_some $(n "rowDecodeEq"):ident
         have $(n "currentInt"):ident :=
           LeanerIR.Proofs.Codec.specInt_decode?_eq_some $(n "currentEq"):ident
         subst $(n "currentInt"):ident $(n "r0Eq"):ident
         have $(n "resultsShape"):ident : $(n "results"):ident =
             #[.borrow ($(n "rowResult"):ident).loan
               (.integer ($(n "rowResult"):ident).value.val)] := by
           have $(n "listArray"):ident := congrArg List.toArray $(n "listEq"):ident
           simpa using $(n "listArray"):ident
         subst $(n "resultsShape"):ident
         simp only [LeanerIR.Proofs.Codec.mutable, LeanerIR.Proofs.Codec.specInt,
           Array.mk.injEq, List.cons.injEq, LeanerIR.RuntimeValue.borrow.injEq,
           LeanerIR.RuntimeValue.integer.injEq, and_true] at $(n "resultsEq"):ident
         obtain ⟨$(n "loanEq"):ident, $(n "valueEq"):ident⟩ := $(n "resultsEq"):ident
         rw [$(n "rowArgsEq"):ident] at $(n "argsEq2"):ident
         simp only [Array.mk.injEq, List.cons.injEq, LeanerIR.RuntimeValue.address.injEq,
           and_true] at $(n "argsEq2"):ident
         subst $(n "argsEq2"):ident
         subst $(n "transferEq"):ident
         obtain ⟨$(n "rowArgs2"):ident, $(n "rowArgsEq3"):ident,
           ⟨$(n "address''"):ident, $(n "argsEq4"):ident, $(n "frameOther"):ident⟩,
           $(n "discipline"):ident⟩ := $(n "frameFn"):ident
         rw [$(n "rowArgsEq3"):ident] at $(n "argsEq4"):ident
         simp only [Array.mk.injEq, List.cons.injEq, LeanerIR.RuntimeValue.address.injEq,
           and_true] at $(n "argsEq4"):ident
         subst $(n "argsEq4"):ident
         have $(n "plainSteps"):ident : LeanerIR.SemanticOperations.PlainSteps $steps := by
           leaner_plain
         $[$siblingFacts:tactic]*
         have $(n "monotone"):ident :
             ($(n "initial"):ident).nextLoan ≤ ($(n "final"):ident).nextLoan :=
           ($(n "discipline"):ident).2.2
         refine ⟨$(n "resultLoan"):ident, ($(n "rowResult"):ident).value.val,
           by rw [$(n "loanEq"):ident], $(n "pendingEq"):ident, ?_⟩
         intro $(n "boundFrame"):ident $(n "boundEq"):ident
         rw [LeanerIR.Proofs.Denotation.bindVariable_rowFrame $fuelLit ⟨1⟩ _ _ _ (by simp)]
           at $(n "boundEq"):ident
         cases Option.some.inj $(n "boundEq"):ident
         refine ⟨_, rfl, ?_⟩
         rw [show (#[some (.address $(n "addr"):ident), none] :
               LeanerIR.Proofs.Denotation.Row).set! 1
               (some (.borrow $(n "resultLoan"):ident
                 (.integer ($(n "rowResult"):ident).value.val))) =
             #[some (.address $(n "addr"):ident),
               some (.borrow $(n "resultLoan"):ident
                 (.integer ($(n "rowResult"):ident).value.val))] from rfl]
         apply LeanerIR.Proofs.Denotation.wpRowThrow_blockUnit
         apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_cons
         apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow
         case stable => leaner_row_stable
         leaner_row_drive
         rename_i $(n "rowRead0"):ident $(n "rowReadEq0"):ident
         simp only [List.getElem?_toArray, List.getElem?_cons_zero,
           List.getElem?_cons_succ, Option.join_some, Option.some.injEq]
           at $(n "rowReadEq0"):ident
         subst $(n "rowReadEq0"):ident
         constructor
         case' right =>
           (intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
              $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident
            rw [LeanerIR.Proofs.Denotation.mutate_evaluate_returnedGlobalLocal1]
              at $(n "rowEq"):ident
            injection $(n "rowEq"):ident with $(n "rowInner"):ident
            injection $(n "rowInner"):ident)
         case' left =>
           (intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
              $(n "rowValueOut"):ident $(n "rowEq"):ident
            rw [LeanerIR.Proofs.Denotation.mutate_evaluate_returnedGlobalLocal1]
              at $(n "rowEq"):ident
            injection $(n "rowEq"):ident with $(n "rowInner"):ident
            injection $(n "rowInner"):ident with $(n "rowFrameEq"):ident
              $(n "rowStateEq"):ident $(n "rowValueEq"):ident
            injection $(n "rowFrameEq"):ident with $(n "rowRowEq"):ident
              $(n "rowActiveEq"):ident $(n "rowLocationsEq"):ident
            subst $(n "rowRowEq"):ident $(n "rowStateEq"):ident $(n "rowValueEq"):ident
            leaner_row_drive
            rename_i $(n "nilResult"):ident $(n "nilStep"):ident
            simp only [LeanerIR.Proofs.Denotation.statementsNil] at $(n "nilStep"):ident
            subst $(n "nilStep"):ident
            refine ⟨_, _, rfl, ?_⟩)
         intro $(n "exitFrame"):ident $(n "exitState"):ident
           $(n "retired"):ident $(n "exited"):ident
         rw [$(n "registryEq"):ident] at $(n "exited"):ident
         rw [LeanerIR.Proofs.Denotation.endLoan_evaluate_returnedGlobal _ _ _ _ _ _ _ _ _
           $(n "plainSteps"):ident $(n "storedEq"):ident] at $(n "exited"):ident
         injection $(n "exited"):ident with $(n "exitInner"):ident
         injection $(n "exitInner"):ident with $(n "exitFrameEq"):ident
           $(n "exitStateEq"):ident $(n "retiredEq"):ident
         subst $(n "exitFrame"):ident $(n "exitState"):ident $(n "retired"):ident
         refine ⟨_, _, rfl, ?_⟩
         intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finishControl?,
           LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
           Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
         subst $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
         rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowFree _ _ _ ?borrowFree]
         case borrowFree =>
           intro $(n "slot"):ident $(n "mem"):ident $(n "value"):ident $(n "eq"):ident
           simp only [List.mem_toArray, List.mem_cons, List.mem_nil_iff, or_false]
             at $(n "mem"):ident
           rcases $(n "mem"):ident with $(n "rfl"):ident | $(n "rfl"):ident <;>
             cases $(n "eq"):ident <;>
             simp [LeanerIR.SemanticOperations.outermostBorrows,
               LeanerIR.SemanticOperations.borrowEntry?,
               LeanerIR.SemanticOperations.collectPruned]
         subst $(n "valueEq"):ident
         have $(n "fitsWritten"):ident :
             LeanerIR.IntegerValueFits (.bits $focusWidth) false $writtenLit := by
           simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?,
             LeanerIR.Ty.integerBounds?]
         /- The callee's final map, as its frame clause and export describe
         it, represented around the returned reborrow's current; then
         the same map around the written value. -/
         have $(n "representedCallee"):ident :
             LeanerIR.FamilyRepresentation $eraseName:ident ⟨$nsLit⟩ ⟨$tyLit⟩
               (LeanerIR.updateContents $(n "contents"):ident (.address $(n "addr"):ident)
                 (some $typedCallee))
               (($(n "final"):ident).globals.insert $key
                 (LeanerIR.SemanticOperations.focusValue $steps
                   (.integer ($(n "rowResult"):ident).value.val))) :=
           LeanerIR.FamilyRepresentation.insert_agreeing (erase := $eraseName:ident)
             $(n "represented"):ident (.address $(n "addr"):ident) $(n "frameOther"):ident
             $typedCallee
         have $(n "represented'"):ident :
             LeanerIR.FamilyRepresentation $eraseName:ident ⟨$nsLit⟩ ⟨$tyLit⟩
               (LeanerIR.updateContents $(n "contents"):ident (.address $(n "addr"):ident)
                 (some $typedWritten))
               (($(n "final"):ident).globals.insert $key
                 (LeanerIR.SemanticOperations.focusValue $steps (.integer $writtenLit))) :=
           LeanerIR.FamilyRepresentation.insert_agreeing (erase := $eraseName:ident)
             $(n "represented"):ident (.address $(n "addr"):ident) $(n "frameOther"):ident
             $typedWritten
         $[$siblingPins:tactic]*
         simp only [$(caller.rawContract):term, $(caller.resultsCodec):term,
           $getName:ident, $readName:ident, $keyName:ident]
         leaner_certified_close!
       · exact absurd $(n "rowDecodeEq"):ident (by simp)
     case onThrow =>
       intro $(n "rowKind"):ident $(n "rowThrown"):ident $(n "final"):ident
         $(n "aborts"):ident
       intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
         $(n "rowOutcome"):ident $(n "rowFinished"):ident
       simp only [LeanerIR.SemanticOperations.finishControl?,
         Option.some.injEq] at $(n "rowFinished"):ident
       subst $(n "rowFinished"):ident
       simp only [$calleeContract:term,
         LeanerIR.Proofs.Contract.runtime, $calleeTyped:term,
         LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
         at $(n "aborts"):ident
       obtain ⟨$(n "rowArgs"):ident, $(n "rowArgsEq"):ident,
         $(n "rowAddress"):ident, $(n "rowArgsEq2"):ident, $(n "rowObligation"):ident⟩ :=
         $(n "aborts"):ident
       simp only [LeanerIR.Proofs.Obligation_iff] at $(n "rowObligation"):ident))

/-! ## Taking a resource out of storage

`remove` takes the resource with `move_from`, destructures it into its
one field, and returns the field.  The take is a row-stable evaluator;
the present side erases the key and binds the field, the absent side
aborts, which the contract either refutes by its requirement or meets as
a declared abort direction. -/

structure TakePlan where
  twin : StoreTwin
  /-- The binding's fuel. -/
  fuel : Nat
  /-- The selections after the destructuring, in evaluation order. -/
  events : List Event

/-- The struct and field of each selection in a nest, innermost first —
the order the selections evaluate in. -/
private partial def selectionChain? (tree : Lean.Expr) (outer : List (Nat × Nat)) :
    Option (Lean.Expr × List (Nat × Nat)) := do
  if tree.isAppOfArity ``nativeOperation 2 then
    let evaluator := tree.getArg! 0
    guard (evaluator.isAppOfArity ``NominalFieldLocation.evaluateSelect? 1)
    let location := evaluator.getArg! 0
    guard (location.isAppOfArity ``NominalFieldLocation.mk 3)
    let source := location.getArg! 0
    guard (source.isAppOfArity ``StructHandle.mk 2)
    let structIndex ← natLitOf? (source.getArg! 1)
    let index ← natLitOf? (location.getArg! 2)
    let operand ← singleOperand? (tree.getArg! 1)
    selectionChain? operand ((structIndex, index) :: outer)
  else pure (tree, outer)

/-- Recognize `let T { f := local1 } := move_from<T>(local0) in local1.g…`,
the field's selections down a chain of one-field structs. -/
def takePlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (twinOf : Nat → Nat → Option (Lean.Name × Nat))
    (twinInfoOf : Nat → Nat → Option (Name × Array (String × SpecTypes.FieldRep)))
    (structOf : Nat → Nat → Option (Name × Array String)) :
    Option TakePlan := do
  guard (parameterCount == 1 && localCount == 2 && resultCount == 1)
  let tree := body.bindingBody!
  guard !tree.hasLooseBVars
  guard (tree.isAppOfArity ``letNativeValue 3)
  let binder := tree.getArg! 0
  guard (binder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
  let fuel ← natLitOf? (binder.getArg! 0)
  guard (fuel ≥ 2)
  let pattern := binder.getArg! 1
  guard (pattern.isAppOfArity ``LeanerIR.SemanticOperations.NativePattern.constructor 3)
  let source := pattern.getArg! 0
  guard (source.isAppOfArity ``LeanerIR.StructHandle.mk 2)
  let structIndex ← natLitOf? (source.getArg! 1)
  guard ((pattern.getArg! 1).isAppOf ``Option.none)
  let fields := pattern.getArg! 2
  guard (fields.isAppOfArity ``List.cons 3)
  guard ((fields.getArg! 2).isAppOf ``List.nil)
  let field := fields.getArg! 1
  guard (field.isAppOfArity ``LeanerIR.SemanticOperations.NativePattern.variable 1)
  guard (natLeaf? (field.getArg! 0) == some 1)
  let take := tree.getArg! 1
  guard (take.isAppOfArity ``nativeGlobalOperation 2)
  let site := take.getArg! 0
  guard (site.isAppOfArity ``GlobalLocationOperation.take 1)
  let (namespaceIndex, typeIndex) ← resourceIndices? (site.getArg! 0)
  guard (isLocalRead (← singleOperand? (take.getArg! 1)) 0)
  let (inner, selections) ← peelSelections structOf (tree.getArg! 2) #[]
  guard (isLocalRead inner 1)
  let (_, chain) ← selectionChain? (tree.getArg! 2) []
  let (twinName, familyStruct) ← twinOf namespaceIndex typeIndex
  guard (familyStruct == structIndex)
  let path ← pathSteps? namespaceIndex ((structIndex, 0) :: chain) twinInfoOf
  guard (path.all fun step => step.fields.size == 1)
  pure { twin := { twin := twinName, namespaceIndex, typeIndex, path }, fuel,
         events := selections.reverse.toList }

/-- The complete script for a body taking a resource out of storage. -/
def takeScript (caller : CallerNames) (plan : TakePlan) :
    CommandElabM (TSyntax `tactic) := do
  let twin := plan.twin
  let nsLit := Syntax.mkNatLit twin.namespaceIndex
  let tyLit := Syntax.mkNatLit twin.typeIndex
  let family := twin.twin.getString!
  let address := caller.parameters.getD 0 "addr"
  let getName := mkIdent (twin.twin ++ `get)
  let readName := mkIdent (twin.twin ++ `read)
  let keyName := mkIdent (twin.twin ++ `key)
  let containsName := mkIdent (twin.twin ++ `contains)
  let eraseName := mkIdent (twin.twin ++ `erase)
  let fuelLit := Syntax.mkNatLit (plan.fuel - 2)
  let key ← `(term| (⟨⟨$nsLit⟩, ⟨$tyLit⟩, .address $(n "addr"):ident⟩ : LeanerIR.GlobalKey))
  let keyEq ← `(tactic| rw [show LeanerIR.SemanticOperations.globalKey ⟨$nsLit⟩ ⟨$tyLit⟩
      (.address $(n "addr"):ident) = $key from rfl])
  let closingPrefix ← `(tactic|
    simp only [$(caller.rawContract):term, $(caller.resultsCodec):term,
      $getName:ident, $readName:ident, $keyName:ident, $containsName:ident])
  /- A slot holding a twin's erasure is borrow-free by that twin's lemma. -/
  let noBorrowsNames := twin.path.toArray.map fun step =>
    mkIdent (step.twin ++ `outermostBorrows_erase)
  let renames ← bindVocabulary #[
    (s!"{family}_contents", "contents"), (s!"{family}_represented", "represented"),
    (address, "addr")]
  let entry ← `(tactic|
    (have $(n "lookupEq"):ident := $(n "represented"):ident (.address $(n "addr"):ident)
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.address]
     rw [show LeanerIR.SemanticOperations.initialLocals 2 #[.address $(n "addr"):ident] =
         #[some (.address $(n "addr"):ident), none] by
           simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
       show LeanerIR.SemanticOperations.parameterLoanLocations
         #[.address $(n "addr"):ident] = #[] by
           simp [LeanerIR.SemanticOperations.parameterLoanLocations]]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow
     case stable => leaner_row_stable
     leaner_row_drive))
  /- The exit: the frame holds the address and the taken field, so it
  exports nothing; the erased map is represented by the typed contents
  without the key. -/
  let returned := (← scalarReturnedPrefix) ++ #[
    ← `(tactic| rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowFree _ _ _
        ?borrowFree]),
    ← `(tactic| case borrowFree =>
        intro slot mem value eq
        simp only [List.mem_toArray, List.mem_cons, List.mem_nil_iff, or_false] at mem
        rcases mem with $(n "rfl"):ident | $(n "rfl"):ident <;> cases eq <;>
          first
          $[| simp only [$noBorrowsNames:ident]]*
          | simp [LeanerIR.SemanticOperations.outermostBorrows,
              LeanerIR.SemanticOperations.borrowEntry?,
              LeanerIR.SemanticOperations.collectPruned]),
    keyEq,
    ← `(tactic| have $(n "represented'"):ident :=
        LeanerIR.FamilyRepresentation.erase_self (erase := $eraseName:ident)
          $(n "represented"):ident (.address $(n "addr"):ident)),
    closingPrefix, ← `(tactic| leaner_certified_close!)]
  let thrown := (← scalarThrownPrefix) ++ #[closingPrefix,
    ← `(tactic| leaner_certified_close!)]
  let finish : Finish := {
    returned := returned
    thrown := thrown
    mutateRow := mkIdent ``LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_singleBorrow }
  let pattern ← resourcePattern twin.path
  let leafTyped ← `(term| ⟨$(n "fieldVal"):ident, $(n "fieldFits"):ident⟩)
  let typed ← typedTerm twin.path leafTyped
  /- The taken resource, its outer constructor exposed for the
  destructuring and the inner value kept as the inner twin's erasure, the
  spelling the selection rows read. -/
  let some outerStep := twin.path.head? | throwError "the take plan has no path"
  let outerLit := Syntax.mkNatLit outerStep.structIndex
  let innerValue ← match twin.path.drop 1 with
    | [] => `(term| LeanerIR.RuntimeValue.integer $(n "fieldVal"):ident)
    | innerStep :: _ =>
        let innerTyped ← typedTerm (twin.path.drop 1) leafTyped (depth := 1)
        `(term| $(mkIdent (innerStep.twin ++ `erase)) $innerTyped)
  let taken ← `(term| LeanerIR.RuntimeValue.nominal ⟨⟨$nsLit⟩, $outerLit⟩ none #[$innerValue])
  let valueIntro ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
    $(n "rowValueOut"):ident $(n "rowEq"):ident)
  let throwIntro ← `(tactic| intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident
    $(n "rowKindOut"):ident $(n "rowThrownOut"):ident $(n "rowEq"):ident)
  let presentRow ← `(tactic|
    rw [LeanerIR.Proofs.Denotation.globalTake_evaluate_present _ _ _ _ _ _
      (by $keyEq:tactic
          exact $(n "lookupEq"):ident)] at $(n "rowEq"):ident)
  let absentRow ← `(tactic|
    rw [LeanerIR.Proofs.Denotation.globalTake_evaluate_absent _ _ _ _ _
      (by $keyEq:tactic
          exact $(n "lookupEq"):ident)] at $(n "rowEq"):ident)
  let someLookup ← `(tactic|
    (rw [$(n "contentsEq"):ident] at $(n "lookupEq"):ident
     simp only [Option.map_some] at $(n "lookupEq"):ident))
  let noneLookup ← `(tactic|
    (rw [$(n "contentsEq"):ident] at $(n "lookupEq"):ident
     simp only [Option.map_none] at $(n "lookupEq"):ident))
  /- The taken resource is the typed value's erasure: one field, bound to
  local 1 by the destructuring, then read as the result. -/
  let bound ← `(tactic|
    (obtain $pattern:rcasesPat := $(n "c"):ident
     leaner_row_drive
     rename_i $(n "boundFrame"):ident $(n "boundEq"):ident
     rw [show $eraseName:ident $typed = $taken from rfl,
       LeanerIR.Proofs.Denotation.bindConstructorOne_rowFrame $fuelLit _ _ _ _ (by simp)]
       at $(n "boundEq"):ident
     cases Option.some.inj $(n "boundEq"):ident
     refine ⟨_, rfl, ?_⟩
     simp only [Array.set!_eq_setIfInBounds, List.setIfInBounds_toArray,
       List.set_cons_succ, List.set_cons_zero]
     leaner_row_drive
     $(← seq ((← readBatch 1) ++ (← emitEvents finish plan.events 0))):tactic))
  let valueSome ← seq <| #[someLookup, presentRow]
    ++ (← invertValue (mkIdent `rowEq)) ++ #[bound]
  let valueNone ← seq <| #[noneLookup, absentRow] ++ (← invertImpossible (mkIdent `rowEq))
  let throwSome ← seq <| #[someLookup, presentRow] ++ (← invertImpossible (mkIdent `rowEq))
  let throwNone ← if caller.requiresCount > 0 then
    seq <| #[noneLookup, ← `(tactic|
      (rw [$(n "lookupEq"):ident] at $(n "requires_0"):ident
       simp at $(n "requires_0"):ident))]
  else
    seq <| #[noneLookup, absentRow] ++ (← invertThrow (mkIdent `rowEq)) ++ thrown
  let valueBranch ← `(tactic|
    ($valueIntro:tactic
     cases $(n "contentsEq"):ident : $(n "contents"):ident (.address $(n "addr"):ident) with
     | none => $valueNone:tactic
     | some $(n "c"):ident => $valueSome:tactic))
  let throwBranch ← `(tactic|
    ($throwIntro:tactic
     cases $(n "contentsEq"):ident : $(n "contents"):ident (.address $(n "addr"):ident) with
     | none => $throwNone:tactic
     | some $(n "c"):ident => $throwSome:tactic))
  `(tactic|
    ($renames:tactic
     $entry:tactic
     $(← seq (← readBatch 1)):tactic
     constructor
     case' left => $valueBranch:tactic
     case' right => $throwBranch:tactic))

/-! ## A value passed to a callee

`carry_u64` calls a generic callee at a concrete instantiation with one
plain integer.  The callee's summary — generic in its carrier, taken at
`PUnit` for the agreement and at the runtime identity codecs for the
contract — says it exports no write-back, so the caller's row is untouched
and its result is the callee's. -/

/-- The value call `f(value)` at the body's top, its value returned. -/
structure ValuePlan where
  callee : CalleeNames
  /-- Whether the callee's agreement quantifies a carrier: a generic callee
  is consumed at the unit carrier and the identity codecs, a monomorphic
  one at its certified codecs. -/
  generic : Bool := true
  /-- Whether the one parameter is an integer (`true`) or a `Bool`. -/
  integerParameter : Bool := true
  /-- The callee's handle — namespace and function indices — by which
  its declaration is found when it has no contract to consume. -/
  handle : Nat × Nat := (0, 0)

/-- The namespace and function indices of a call's handle literal. -/
private def handleIndices? (handle : Lean.Expr) : Option (Nat × Nat) := do
  guard (handle.isAppOfArity ``LeanerIR.FunctionHandle.mk 2)
  let namespaceId := handle.getArg! 0
  let functionId := handle.getArg! 1
  guard (namespaceId.isAppOfArity ``LeanerIR.NamespaceId.mk 1)
  guard (functionId.isAppOfArity ``LeanerIR.FunctionId.mk 1)
  pure (← natLeaf? (namespaceId.getArg! 0), ← natLeaf? (functionId.getArg! 0))

/-- Recognize `nativeCall f (values [localVar 0])` as the whole body of a
one-integer, one-result function. -/
def valuePlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (plainKinds : Option (Array Bool)) : Option ValuePlan := do
  guard (parameterCount == 1 && localCount == 1 && resultCount == 1)
  let some kinds := plainKinds | none
  let integerParameter ← kinds[0]?
  let tree := body.bindingBody!
  guard (tree.isAppOfArity ``nativeCall 4)
  guard ((tree.getArg! 1).isAppOf ``Option.none)
  let callee := tree.getArg! 2
  let relation ← calleeRelation? callee
  guard (isLocalRead (← singleOperand? (tree.getArg! 3)) 0)
  let names ← calleeNames? relation
  let handle ← handleIndices? (tree.getArg! 0)
  pure { callee := names, integerParameter, handle }

/-- The events of an unspecified callee's body, for running it inline:
the body must be inside the scalar subset and mention no callee of its
own. -/
def calleeEvents? (body : Lean.Expr) : Option (List Event) := do
  let tree := body.bindingBody!
  guard !tree.hasLooseBVars
  linearize tree

/-- A callee without a contract, run inline where it is called. -/
structure InlineCallee where
  names : CalleeNames
  /-- The callee's body events, on the scalar templates. -/
  events : List Event
  /-- Whether its one parameter is an integer (`true`) or a `Bool`. -/
  integerParameter : Bool

/-- A callee with a contract, consumed modularly at a spine call: one
integer or `Bool` parameter, one integer result, one `ensures` clause —
the contract grammar `modularCallee` destructures. -/
structure ModularCallee where
  names : CalleeNames
  /-- Whether its one parameter is an integer (`true`) or a `Bool`. -/
  integerParameter : Bool

/-- How a spine call is resolved: an unspecified callee runs inline, a
specified one is consumed through its contract. -/
inductive SpineCallee where
  | inline (callee : InlineCallee)
  | modular (callee : ModularCallee)

/-- A throw-aware body: the spine nodes a call can sit under, with the
subtrees between them — row-stable, inside the scalar subset — driven on
the scalar templates. -/
inductive Spine where
  | stable (events : List Event)
  | call (relation : Name) (handle : Nat × Nat) (operand : Spine)
  | letIn (slot fuel : Nat) (initializer body : Spine)
  | branch (condition thenArm : Spine) (elseArm : Option Spine)

/-- Recognize a body's spine.  A subtree that mentions no callee is
stable; a one-operand call, a `let`, and a branch are spine nodes. -/
partial def spine? (tree : Lean.Expr) : Option Spine := do
  if !tree.hasLooseBVars then
    return .stable (← linearize tree)
  let head ← tree.getAppFn.constName?
  if head == ``nativeCall then
    guard (tree.isAppOfArity ``nativeCall 4)
    guard ((tree.getArg! 1).isAppOf ``Option.none)
    let callee := tree.getArg! 2
    let relation ← calleeRelation? callee
    let handle ← handleIndices? (tree.getArg! 0)
    let operands := tree.getArg! 3
    guard (operands.isAppOfArity ``valuesCons 2)
    guard ((operands.getArg! 1).isConstOf ``valuesNil)
    return .call relation handle (← spine? (operands.getArg! 0))
  else if head == ``letNativeValue then
    guard (tree.isAppOfArity ``letNativeValue 3)
    let binder := tree.getArg! 0
    guard (binder.isAppOfArity ``LeanerIR.SemanticOperations.NativePatternBinder.mk 2)
    let fuel ← binder.getArg! 0 |>.nat?
    guard (fuel > 0)
    let pattern := binder.getArg! 1
    guard (pattern.isAppOfArity ``LeanerIR.SemanticOperations.NativePattern.variable 1)
    let slot ← natLeaf? (pattern.getArg! 0)
    return .letIn slot fuel (← spine? (tree.getArg! 1)) (← spine? (tree.getArg! 2))
  else if head == ``nativeBranch then
    guard (tree.isAppOfArity ``nativeBranch 3)
    let elseArm := tree.getArg! 2
    let elseSpine ← if elseArm.isAppOfArity ``Option.some 2 then
        (some <$> spine? (elseArm.getArg! 1) : Option (Option Spine))
      else if elseArm.isAppOf ``Option.none then pure none
      else none
    return .branch (← spine? (tree.getArg! 0)) (← spine? (tree.getArg! 1)) elseSpine
  else none

/-- The calls a spine makes, with their handles. -/
partial def Spine.calls : Spine → List (Name × (Nat × Nat))
  | .stable _ => []
  | .call relation handle operand => (relation, handle) :: operand.calls
  | .letIn _ _ initializer body => initializer.calls ++ body.calls
  | .branch condition thenArm elseArm =>
      condition.calls ++ thenArm.calls ++ (elseArm.map (·.calls)).getD []

/-- A throw-aware body with plain parameters and its callees. -/
structure SpinePlan where
  spine : Spine
  parameterCount : Nat
  localCount : Nat
  /-- Per parameter, whether it is an integer (`true`) or a `Bool`. -/
  kinds : Array Bool
  callees : Array (Name × SpineCallee)

/-- What the spine emitter carries: the caller's finish, and the callees
by relation. -/
private structure SpineContext where
  finish : Finish
  callees : Array (Name × SpineCallee)

/-- Run an unspecified callee inline from the goal its call left — the
callee's relation at the argument — and continue the caller with `k` on
the value it returned.  The callee's throw is the caller's throw. -/
private def inlineCallee (ctx : SpineContext) (callee : InlineCallee)
    (k : Array (TSyntax `tactic)) : CommandElabM (Array (TSyntax `tactic)) := do
  let shape := mkIdent callee.names.shape
  let loans ← if callee.integerParameter then
      `(term| LeanerIR.SemanticOperations.parameterLoanLocations_singleInteger)
    else `(term| LeanerIR.SemanticOperations.parameterLoanLocations_singleBool)
  let returned : Array (TSyntax `tactic) := #[
    ← `(tactic| leaner_row_drive),
    ← `(tactic| rename_i $(n "rowOutcome"):ident $(n "rowFinished"):ident),
    ← `(tactic| simp only [$shape:term, LeanerIR.SemanticOperations.finishControl?,
        LeanerIR.SemanticOperations.unpackFallthrough,
        Option.map_eq_map, Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident),
    ← `(tactic| subst $(n "rowFinished"):ident),
    ← `(tactic| simp only [LeanerIR.SemanticOperations.finalizeFunctionState]),
    ← `(tactic|
        (rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowFree _ _ _
           ?calleeBorrowFree]
         case calleeBorrowFree =>
           intro slot mem value eq
           simp only [List.mem_toArray, List.mem_cons, List.mem_nil_iff, or_false] at mem
           subst mem
           cases eq
           simp [LeanerIR.SemanticOperations.outermostBorrows,
             LeanerIR.SemanticOperations.borrowEntry?,
             LeanerIR.SemanticOperations.collectPruned])),
    ← `(tactic| refine ⟨rfl, ?_⟩),
    ← `(tactic| rw [LeanerIR.Proofs.Denotation.packResults_singleton])] ++ k
  let thrown : Array (TSyntax `tactic) := #[
    ← `(tactic| leaner_row_drive),
    ← `(tactic| rename_i $(n "rowOutcome"):ident $(n "rowFinished"):ident),
    ← `(tactic| simp only [$shape:term, LeanerIR.SemanticOperations.finishControl?,
        Option.some.injEq] at $(n "rowFinished"):ident),
    ← `(tactic| subst $(n "rowFinished"):ident)] ++ ctx.finish.thrown
  let calleeFinish : Finish := {
    returned, thrown, mutateRow := ctx.finish.mutateRow, parameters := #[] }
  let reads := callee.events.takeWhile (· == .read) |>.length
  let rest := callee.events.drop reads
  pure <| #[
    ← `(tactic| leaner_row_drive),
    ← `(tactic| rw [LeanerIR.Proofs.Denotation.wpFunction_nativeFunctionRelation]),
    ← `(tactic| apply LeanerIR.Proofs.Denotation.nativeEntry_rowFrame),
    ← `(tactic| case arity => rfl),
    ← `(tactic| case declared => simp [$shape:term]),
    ← `(tactic| case stable => leaner_row_stable),
    ← `(tactic| simp only [$shape:term, LeanerIR.Proofs.Denotation.initialLocals_one,
        $loans:term]),
    ← `(tactic| leaner_row_drive)]
    ++ (← readBatch reads) ++ (← emitEvents calleeFinish rest 0)

/-- Consume a specified callee at a spine call from the goal its argument
leaves — `wpFunction` of the callee's relation at the argument, under the
`match` the call law states — and continue the caller with `k` on the
result the callee's clauses name.  The callee's contract is the grammar
`valueScriptMono` destructures: the argument record at the callee's
codec, one integer result, one `ensures` clause; its abort clause is the
caller's throw. -/
private def modularCallee (ctx : SpineContext) (callee : ModularCallee)
    (k : Array (TSyntax `tactic)) : CommandElabM (Array (TSyntax `tactic)) := do
  let names := callee.names
  let calleeContract := mkIdent names.contract
  let calleeTyped := mkIdent names.typedContract
  let calleeRaw := mkIdent names.rawContract
  let calleeCodec := mkIdent names.argumentsCodec
  let calleeResults := mkIdent names.resultsCodec
  /- The argument is whatever value the spine left: its record is built
  by unification, its range facts closed from the context. -/
  let permitted ← if callee.integerParameter then
      `(tactic| refine ⟨⟨⟨_, ?_⟩⟩, rfl, _, ⟨rfl, ?_, ?_⟩, ?_⟩)
    else `(tactic| refine ⟨⟨_⟩, rfl, _, rfl, ?_⟩)
  let injection := mkIdent <| if callee.integerParameter then
      ``LeanerIR.RuntimeValue.integer.injEq else ``LeanerIR.RuntimeValue.bool.injEq
  let continued ← seq k
  let thrown ← seq ctx.finish.thrown
  pure #[
    ← `(tactic| leaner_row_drive),
    ← `(tactic|
      (apply LeanerIR.Proofs.Denotation.wpFunction_of_satisfies
         $(calleeSatIdent names)
       case permitted =>
         simp only [$calleeContract:term, LeanerIR.Proofs.Contract.runtime,
           $calleeTyped:term, LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
           $calleeCodec:term, LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool]
         $permitted:tactic
         all_goals first
           | assumption
           | exact $(n "freshLoans"):ident
           | omega
           | decide
           | (simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?,
                LeanerIR.Ty.integerBounds?]; omega)
       case onReturn =>
         intro $(n "results"):ident $(n "final"):ident $(n "ensuresFn"):ident
           $(n "frameFn"):ident $(n "notMust"):ident
         have $(n "ensures"):ident := $(n "ensuresFn"):ident (by exact $(n "notMust"):ident)
         simp only [$calleeContract:term, LeanerIR.Proofs.Contract.runtime,
           $calleeTyped:term, LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
           $calleeResults:term, $calleeCodec:term, LeanerIR.Proofs.Codec.specInt,
           LeanerIR.Proofs.Codec.bool]
           at $(n "ensures"):ident $(n "frameFn"):ident
         obtain ⟨$(n "rowArgs"):ident, $(n "rowResult"):ident, $(n "rowArgsEq"):ident,
           $(n "rowDecodeEq"):ident, $(n "value"):ident, $(n "result"):ident,
           ⟨⟨⟨$(n "argsEq2"):ident, $(n "resultsEq"):ident⟩, $(n "pendingEq"):ident⟩,
             $(n "resNonNeg"):ident, $(n "resMax"):ident⟩,
           $(n "obligation"):ident⟩ := $(n "ensures"):ident
         split at $(n "rowDecodeEq"):ident
         · rename_i $(n "r0"):ident $(n "listEq"):ident
           simp only [Option.bind_eq_some_iff, Option.some.injEq, exists_eq_right]
             at $(n "rowDecodeEq"):ident
           cases $(n "r0"):ident <;> simp [LeanerIR.decodeInt?] at $(n "rowDecodeEq"):ident
           rename_i $(n "resultVal"):ident
           obtain ⟨$(n "resultFits"):ident, $(n "rfl"):ident⟩ := $(n "rowDecodeEq"):ident
           have $(n "resultsShape"):ident :
               $(n "results"):ident = #[.integer $(n "resultVal"):ident] := by
             have $(n "listArray"):ident := congrArg List.toArray $(n "listEq"):ident
             simpa using $(n "listArray"):ident
           subst $(n "resultsShape"):ident
           simp only [Array.mk.injEq, List.cons.injEq, and_true,
             LeanerIR.RuntimeValue.integer.injEq] at $(n "resultsEq"):ident
           subst $(n "resultsEq"):ident
           rw [$(n "rowArgsEq"):ident] at $(n "argsEq2"):ident
           simp only [Array.mk.injEq, List.cons.injEq, and_true, $injection:ident]
             at $(n "argsEq2"):ident
           subst $(n "argsEq2"):ident
           simp only [LeanerIR.Proofs.Obligation_iff] at $(n "obligation"):ident
           subst $(n "obligation"):ident
           obtain ⟨$(n "rowArgs2"):ident, $(n "rowArgsEq3"):ident, $(n "globalsEq"):ident,
             $(n "discipline"):ident⟩ := $(n "frameFn"):ident
           refine ⟨$(n "pendingEq"):ident, ?_⟩
           rw [LeanerIR.Proofs.Denotation.packResults_singleton]
           $continued:tactic
         · exact absurd $(n "rowDecodeEq"):ident (by simp)
       case onThrow =>
         intro $(n "rowKind"):ident $(n "rowThrown"):ident $(n "final"):ident
           $(n "aborts"):ident
         simp only [$calleeContract:term, LeanerIR.Proofs.Contract.runtime,
           $calleeTyped:term, LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
           $calleeCodec:term, LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool]
           at $(n "aborts"):ident
         obtain ⟨$(n "rowArgs"):ident, $(n "rowArgsEq"):ident, $(n "value"):ident,
           $(n "argsEq2"):ident, $(n "calleeAborts"):ident⟩ := $(n "aborts"):ident
         rw [$(n "rowArgsEq"):ident] at $(n "argsEq2"):ident
         simp only [Array.mk.injEq, List.cons.injEq, and_true, $injection:ident]
           at $(n "argsEq2"):ident
         subst $(n "argsEq2"):ident
         simp only [LeanerIR.Proofs.Obligation_iff] at $(n "calleeAborts"):ident
         all_goals $thrown:tactic))]

/-- Emit a spine from a `wpRowThrow` goal over it: `k` continues the
caller on the value the spine produces, from the goal that value leaves
before any head reduction. -/
private partial def emitSpine (ctx : SpineContext) (spine : Spine)
    (k : Array (TSyntax `tactic)) : CommandElabM (Array (TSyntax `tactic)) := do
  match spine with
  | .stable events =>
      let finish : Finish := { ctx.finish with returned := k }
      let reads := events.takeWhile (· == .read) |>.length
      let rest := events.drop reads
      pure <| #[
        ← `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_of_wpRow),
        ← `(tactic| case stable => leaner_row_stable),
        ← `(tactic| leaner_row_drive)]
        ++ (← readBatch reads) ++ (← emitEvents finish rest 0)
  | .call relation _ operand =>
      let some (_, callee) := ctx.callees.find? (·.1 == relation)
        | throwError "no callee for {relation}"
      let consumed ← match callee with
        | .inline callee => inlineCallee ctx callee k
        | .modular callee => modularCallee ctx callee k
      pure <| #[← `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_callValueOperand)]
        ++ (← emitSpine ctx operand consumed)
  | .letIn slot fuel initializer body =>
      let continued ← emitSpine ctx body k
      let bound := #[← `(tactic| leaner_row_drive)] ++ (← bindStep slot fuel) ++ continued
      pure <| #[← `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_letNativeValueThrow)]
        ++ (← emitSpine ctx initializer bound)
  | .branch condition thenArm elseArm =>
      let thenTactics ← seq (← emitSpine ctx thenArm k)
      let elseTactics ← match elseArm with
        | some arm => seq (← emitSpine ctx arm k)
        | none => seq k
      let split : Array (TSyntax `tactic) := #[
        ← `(tactic| leaner_row_drive),
        ← `(tactic| refine ⟨fun $(n "rowGuard"):ident => ?_, fun $(n "rowGuard"):ident => ?_⟩),
        ← `(tactic| · $thenTactics:tactic),
        ← `(tactic| · $elseTactics:tactic)]
      pure <| #[← `(tactic| apply LeanerIR.Proofs.Denotation.wpRowThrow_nativeBranchThrow)]
        ++ (← emitSpine ctx condition split)

/-- The complete script for a plain-parameter body with calls to
unspecified callees: the throw-aware entry, the spine, the scalar finish. -/
def spineScript (caller : CallerNames) (plan : SpinePlan) :
    CommandElabM (TSyntax `tactic) := do
  let scalarPlan : Plan := {
    events := [], shape := .plainIntegers plan.parameterCount,
    parameterCount := plan.parameterCount, localCount := plan.localCount,
    kinds := plan.kinds }
  let finish ← scalarFinish caller scalarPlan
  let ctx : SpineContext := { finish, callees := plan.callees }
  let body ← seq (← emitSpine ctx plan.spine finish.returned)
  let names := (List.range plan.parameterCount).map fun index =>
    mkIdent (Name.mkSimple (caller.parameters.getD index s!"argument{index}"))
  let valueOf (index : Nat) (name : Ident) : CommandElabM Term :=
    if plan.kinds.getD index true then `(term| .integer $name:ident)
    else `(term| .bool $name:ident)
  let arguments ← names.zipIdx.mapM fun (name, index) => valueOf index name
  let present ← names.zipIdx.mapM fun (name, index) => do
    let value ← valueOf index name
    `(term| some $value)
  let absent ← (List.replicate (plan.localCount - plan.parameterCount) ()).mapM fun _ =>
    `(term| none)
  let slotsTerm := present ++ absent
  let localsLit := Syntax.mkNatLit plan.localCount
  /- Every inline callee's relation and body are opened once, at entry,
  to the native relation and tree the call laws and the drive read; every
  specified callee's contract fact is brought into context once. -/
  let mut relations : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) := #[]
  let mut calleeFacts : Array (TSyntax `tactic) := #[]
  for (_, callee) in plan.callees do
    match callee with
    | .inline callee =>
        relations := relations.push
          (← `(Lean.Parser.Tactic.simpLemma| $(mkIdent callee.names.relation):ident))
        relations := relations.push
          (← `(Lean.Parser.Tactic.simpLemma| $(mkIdent callee.names.body):ident))
    | .modular callee =>
        calleeFacts := calleeFacts.push (← calleeSatFact caller callee.names)
  let calleeSetup ← seq calleeFacts
  `(tactic|
    ($calleeSetup:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool,
       LeanerIR.Proofs.Codec.mutable, $relations,*]
     rw [show LeanerIR.SemanticOperations.initialLocals $localsLit
         #[$(arguments.toArray),*] = #[$(slotsTerm.toArray),*] by
         simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
       show LeanerIR.SemanticOperations.parameterLoanLocations
         #[$(arguments.toArray),*] = #[] by
         simp [LeanerIR.SemanticOperations.parameterLoanLocations]]
     $body:tactic))

/-- The complete script for a body that returns a value callee's result.
The callee is generic: its agreement is taken at the unit carrier and its
contract is the one `verified` states, at the runtime identity codecs. -/
def valueScript (caller : CallerNames) (plan : ValuePlan) :
    CommandElabM (TSyntax `tactic) := do
  let callee := plan.callee
  let value := caller.parameters.getD 0 "value"
  /- The parameter's kind fixes its facts and its runtime value; the
  generic callee's identity codecs take either as they are. -/
  let renames ← if plan.integerParameter then
      bindVocabulary #[(value, "valVar"), (s!"{value}_fits", "fitsVar"),
        (s!"{value}_nonNeg", "valNonNeg"), (s!"{value}_max", "valMax")]
    else bindVocabulary #[(value, "valVar")]
  let argument ← if plan.integerParameter then `(term| .integer $(n "valVar"):ident)
    else `(term| .bool $(n "valVar"):ident)
  let calleeContract := mkIdent callee.contract
  let calleeTyped := mkIdent callee.typedContract
  let calleeRaw := mkIdent callee.rawContract
  let calleeCodec := mkIdent callee.argumentsCodec
  let calleeResults := mkIdent callee.resultsCodec
  let calleeFact ← calleeSatFact caller callee (generic := true)
  `(tactic|
    ($calleeFact:tactic
     $renames:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool]
     rw [show LeanerIR.SemanticOperations.initialLocals 1
         #[$argument] = #[some $argument] by
         simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
       show LeanerIR.SemanticOperations.parameterLoanLocations
         #[$argument] = #[] by
         simp [LeanerIR.SemanticOperations.parameterLoanLocations]]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_callValue (rest := [])
     apply LeanerIR.Proofs.Denotation.wpFunction_of_satisfies
       $(calleeSatIdent callee)
     case permitted =>
       simp only [$calleeContract:term, LeanerIR.Proofs.Contract.runtime,
         $calleeTyped:term, LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
         $calleeCodec:term, LeanerIR.Proofs.Codec.identity]
       exact ⟨⟨$argument⟩, rfl, $argument, rfl, $(n "freshLoans"):ident⟩
     case onReturn =>
       intro $(n "results"):ident $(n "final"):ident $(n "ensuresFn"):ident
         $(n "frameFn"):ident $(n "notMust"):ident
       have $(n "ensures"):ident := $(n "ensuresFn"):ident (by exact $(n "notMust"):ident)
       simp only [$calleeContract:term, LeanerIR.Proofs.Contract.runtime,
         $calleeTyped:term, LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
         at $(n "ensures"):ident $(n "frameFn"):ident
       obtain ⟨$(n "rowArgs"):ident, $(n "rowResult"):ident, $(n "rowArgsEq"):ident,
         $(n "rowDecodeEq"):ident, $(n "value"):ident, $(n "result"):ident,
         ⟨⟨$(n "argsEq2"):ident, $(n "resultsEq"):ident⟩, $(n "pendingEq"):ident⟩,
         $(n "obligation"):ident⟩ := $(n "ensures"):ident
       simp only [$calleeResults:term, LeanerIR.Proofs.Codec.identity]
         at $(n "rowDecodeEq"):ident $(n "resultsEq"):ident
       split at $(n "rowDecodeEq"):ident
       · rename_i $(n "r0"):ident $(n "listEq"):ident
         simp only [Option.bind_eq_some_iff, Option.some.injEq, exists_eq_right]
           at $(n "rowDecodeEq"):ident
         subst $(n "rowDecodeEq"):ident
         have $(n "resultsShape"):ident : $(n "results"):ident = #[$(n "r0"):ident] := by
           have $(n "listArray"):ident := congrArg List.toArray $(n "listEq"):ident
           simpa using $(n "listArray"):ident
         subst $(n "resultsShape"):ident
         simp only [Array.mk.injEq, List.cons.injEq, and_true] at $(n "resultsEq"):ident
         subst $(n "resultsEq"):ident
         rw [$(n "rowArgsEq"):ident] at $(n "argsEq2"):ident
         simp only [Array.mk.injEq, List.cons.injEq, and_true] at $(n "argsEq2"):ident
         subst $(n "argsEq2"):ident
         simp only [LeanerIR.Proofs.Obligation_iff] at $(n "obligation"):ident
         subst $(n "obligation"):ident
         obtain ⟨$(n "rowArgs2"):ident, $(n "rowArgsEq3"):ident, $(n "globalsEq"):ident,
           $(n "discipline"):ident⟩ := $(n "frameFn"):ident
         refine ⟨$(n "pendingEq"):ident, ?_⟩
         intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
         have $(n "packOne"):ident : LeanerIR.SemanticOperations.packResults
             #[$argument] = $argument := rfl
         rw [$(n "packOne"):ident] at $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finishControl?,
           LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
           Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
         subst $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
         rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowFree _ _ _
           ?borrowFree]
         case borrowFree =>
           intro slot mem value eq
           simp only [List.mem_toArray, List.mem_cons, List.mem_nil_iff, or_false] at mem
           rcases mem with $(n "rfl"):ident <;> cases eq <;>
             simp [LeanerIR.SemanticOperations.outermostBorrows,
               LeanerIR.SemanticOperations.borrowEntry?,
               LeanerIR.SemanticOperations.collectPruned]
         simp only [$(caller.rawContract):term, $(caller.resultsCodec):term]
         have $(n "rowDiscipline"):ident : LeanerIR.SemanticOperations.LoanDiscipline
             $(n "initial"):ident
             { globals := ($(n "final"):ident).globals
               globalLoans := ($(n "final"):ident).globalLoans
               nextLoan := ($(n "final"):ident).nextLoan
               pending := ($(n "initial"):ident).pending } :=
           ($(n "discipline"):ident).trans
             (LeanerIR.SemanticOperations.LoanDiscipline.of_eq rfl (Nat.le_refl _))
         leaner_certified_close!
       · exact absurd $(n "rowDecodeEq"):ident (by simp)
     case onThrow =>
       intro $(n "rowKind"):ident $(n "rowThrown"):ident $(n "final"):ident
         $(n "aborts"):ident
       simp only [$calleeContract:term, LeanerIR.Proofs.Contract.runtime,
         $calleeTyped:term, LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
         at $(n "aborts"):ident
       obtain ⟨-, -, $(n "absurdAbort"):ident⟩ := $(n "aborts"):ident
       exact ($(n "absurdAbort"):ident).elim))

/-- The complete script for a body that returns a monomorphic integer
callee's result: the callee's contract is consumed at its certified integer
codecs — the argument record is the caller's integer with its certificate,
the result decodes to the integer the callee's clauses name — and the
callee's abort clause is the caller's. -/
def valueScriptMono (caller : CallerNames) (plan : ValuePlan) :
    CommandElabM (TSyntax `tactic) := do
  let callee := plan.callee
  let value := caller.parameters.getD 0 "value"
  /- The parameter's kind fixes its facts, its runtime value, and the
  argument record the callee's `requires` receives. -/
  let renames ← if plan.integerParameter then
      bindVocabulary #[(value, "valVar"), (s!"{value}_fits", "fitsVar"),
        (s!"{value}_nonNeg", "valNonNeg"), (s!"{value}_max", "valMax")]
    else bindVocabulary #[(value, "valVar")]
  let argument ← if plan.integerParameter then `(term| .integer $(n "valVar"):ident)
    else `(term| .bool $(n "valVar"):ident)
  let permittedTerm ← if plan.integerParameter then
      `(term| ⟨⟨⟨$(n "valVar"):ident, $(n "fitsVar"):ident⟩⟩, rfl, $(n "valVar"):ident,
        ⟨rfl, $(n "valNonNeg"):ident, $(n "valMax"):ident⟩, $(n "freshLoans"):ident⟩)
    else
      `(term| ⟨⟨$(n "valVar"):ident⟩, rfl, $(n "valVar"):ident, rfl, $(n "freshLoans"):ident⟩)
  let injection := mkIdent <| if plan.integerParameter then
      ``LeanerIR.RuntimeValue.integer.injEq else ``LeanerIR.RuntimeValue.bool.injEq
  let calleeContract := mkIdent callee.contract
  let calleeTyped := mkIdent callee.typedContract
  let calleeRaw := mkIdent callee.rawContract
  let calleeCodec := mkIdent callee.argumentsCodec
  let calleeResults := mkIdent callee.resultsCodec
  let calleeFact ← calleeSatFact caller callee (generic := false)
  `(tactic|
    ($calleeFact:tactic
     $renames:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => rfl
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term,
       LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool]
     rw [show LeanerIR.SemanticOperations.initialLocals 1
         #[$argument] = #[some $argument] by
         simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
       show LeanerIR.SemanticOperations.parameterLoanLocations
         #[$argument] = #[] by
         simp [LeanerIR.SemanticOperations.parameterLoanLocations]]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_callValue (rest := [])
     apply LeanerIR.Proofs.Denotation.wpFunction_of_satisfies
       $(calleeSatIdent callee)
     case permitted =>
       simp only [$calleeContract:term, LeanerIR.Proofs.Contract.runtime,
         $calleeTyped:term, LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
         $calleeCodec:term, LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool]
       exact $permittedTerm
     case onReturn =>
       intro $(n "results"):ident $(n "final"):ident $(n "ensuresFn"):ident
         $(n "frameFn"):ident $(n "notMust"):ident
       have $(n "ensures"):ident := $(n "ensuresFn"):ident (by exact $(n "notMust"):ident)
       simp only [$calleeContract:term, LeanerIR.Proofs.Contract.runtime,
         $calleeTyped:term, LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
         $calleeResults:term, $calleeCodec:term, LeanerIR.Proofs.Codec.specInt,
         LeanerIR.Proofs.Codec.bool]
         at $(n "ensures"):ident $(n "frameFn"):ident
       obtain ⟨$(n "rowArgs"):ident, $(n "rowResult"):ident, $(n "rowArgsEq"):ident,
         $(n "rowDecodeEq"):ident, $(n "value"):ident, $(n "result"):ident,
         ⟨⟨⟨$(n "argsEq2"):ident, $(n "resultsEq"):ident⟩, $(n "pendingEq"):ident⟩,
           $(n "resNonNeg"):ident, $(n "resMax"):ident⟩,
         $(n "obligation"):ident⟩ := $(n "ensures"):ident
       split at $(n "rowDecodeEq"):ident
       · rename_i $(n "r0"):ident $(n "listEq"):ident
         simp only [Option.bind_eq_some_iff, Option.some.injEq, exists_eq_right]
           at $(n "rowDecodeEq"):ident
         cases $(n "r0"):ident <;> simp [LeanerIR.decodeInt?] at $(n "rowDecodeEq"):ident
         rename_i $(n "resultVal"):ident
         obtain ⟨$(n "resultFits"):ident, $(n "rfl"):ident⟩ := $(n "rowDecodeEq"):ident
         have $(n "resultsShape"):ident :
             $(n "results"):ident = #[.integer $(n "resultVal"):ident] := by
           have $(n "listArray"):ident := congrArg List.toArray $(n "listEq"):ident
           simpa using $(n "listArray"):ident
         subst $(n "resultsShape"):ident
         simp only [Array.mk.injEq, List.cons.injEq, and_true,
           LeanerIR.RuntimeValue.integer.injEq] at $(n "resultsEq"):ident
         subst $(n "resultsEq"):ident
         rw [$(n "rowArgsEq"):ident] at $(n "argsEq2"):ident
         simp only [Array.mk.injEq, List.cons.injEq, and_true, $injection:ident]
           at $(n "argsEq2"):ident
         subst $(n "argsEq2"):ident
         simp only [LeanerIR.Proofs.Obligation_iff] at $(n "obligation"):ident
         subst $(n "obligation"):ident
         obtain ⟨$(n "rowArgs2"):ident, $(n "rowArgsEq3"):ident, $(n "globalsEq"):ident,
           $(n "discipline"):ident⟩ := $(n "frameFn"):ident
         refine ⟨$(n "pendingEq"):ident, ?_⟩
         intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
         rw [LeanerIR.Proofs.Denotation.packResults_singleton] at $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finishControl?,
           LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
           Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
         subst $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
         rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowFree _ _ _
           ?borrowFree]
         case borrowFree =>
           intro slot mem value eq
           simp only [List.mem_toArray, List.mem_cons, List.mem_nil_iff, or_false] at mem
           rcases mem with $(n "rfl"):ident <;> cases eq <;>
             simp [LeanerIR.SemanticOperations.outermostBorrows,
               LeanerIR.SemanticOperations.borrowEntry?,
               LeanerIR.SemanticOperations.collectPruned]
         simp only [$(caller.rawContract):term, $(caller.resultsCodec):term]
         have $(n "rowDiscipline"):ident : LeanerIR.SemanticOperations.LoanDiscipline
             $(n "initial"):ident
             { globals := ($(n "final"):ident).globals
               globalLoans := ($(n "final"):ident).globalLoans
               nextLoan := ($(n "final"):ident).nextLoan
               pending := ($(n "initial"):ident).pending } :=
           ($(n "discipline"):ident).trans
             (LeanerIR.SemanticOperations.LoanDiscipline.of_eq rfl (Nat.le_refl _))
         leaner_certified_close!
       · exact absurd $(n "rowDecodeEq"):ident (by simp)
     case onThrow =>
       intro $(n "rowKind"):ident $(n "rowThrown"):ident $(n "final"):ident
         $(n "aborts"):ident
       simp only [$calleeContract:term, LeanerIR.Proofs.Contract.runtime,
         $calleeTyped:term, LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
         $calleeCodec:term, LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool]
         at $(n "aborts"):ident
       obtain ⟨$(n "rowArgs"):ident, $(n "rowArgsEq"):ident, $(n "value"):ident,
         $(n "argsEq2"):ident, $(n "calleeAborts"):ident⟩ := $(n "aborts"):ident
       rw [$(n "rowArgsEq"):ident] at $(n "argsEq2"):ident
       simp only [Array.mk.injEq, List.cons.injEq, and_true, $injection:ident]
         at $(n "argsEq2"):ident
       subst $(n "argsEq2"):ident
       /- A callee that cannot abort refutes the branch here. -/
       simp only [LeanerIR.Proofs.Obligation_iff] at $(n "calleeAborts"):ident
       all_goals
         (intro $(n "rowFrameOut"):ident $(n "rowStateOut"):ident $(n "rowOutcome"):ident
            $(n "rowFinished"):ident
          simp only [LeanerIR.SemanticOperations.finishControl?, Option.some.injEq]
            at $(n "rowFinished"):ident
          subst $(n "rowFinished"):ident
          simp only [$(caller.rawContract):term]
          leaner_certified_close!)))

/-! ## Generic bodies

A generic function's theorem quantifies over its carrier and codecs; its
values are abstract encodings, and so are its registries (an abstract
value may be a borrow).  `carry {T}(value : T) -> T := value` moves its
parameter out of local 0 and returns it: the row empties, the export is
borrow-free, and the codec's own left inverse decodes the result. -/

/-- The move-out body, read off its tree. -/
structure GenericPlan where
  parameter : String

/-- Recognize `move local0` as the whole body of a one-parameter,
one-result function. -/
def genericPlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (parameter : String) : Option GenericPlan := do
  guard (parameterCount == 1 && localCount == 1 && resultCount == 1)
  let tree := body.bindingBody!
  guard !tree.hasLooseBVars
  guard (tree.isAppOfArity ``nativeLocalOperation 2)
  guard ((tree.getArg! 1).isConstOf ``valuesNil)
  let operation := tree.getArg! 0
  guard (operation.isAppOfArity ``LocalLocationOperation.move 1)
  guard (natLeaf? (operation.getArg! 0) == some 0)
  pure { parameter }

/-- The complete script for a generic move-out body. -/
def genericScript (caller : CallerNames) (plan : GenericPlan) :
    CommandElabM (TSyntax `tactic) := do
  let value := mkIdent (Name.mkSimple plan.parameter)
  `(tactic|
    (apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term]
     rw [show LeanerIR.SemanticOperations.initialLocals 1
         #[($(n "codecs"):ident 0).encode $value] =
         #[some (($(n "codecs"):ident 0).encode $value)] by
         simp [LeanerIR.SemanticOperations.initialLocals]; rfl]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_moveLocal0 (rest := [])
     intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finishControl?,
       LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
       Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
     subst $(n "rowFinished"):ident
     simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
     rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowFree _ _ _
       ?borrowFree]
     case borrowFree =>
       intro slot mem value eq
       simp only [List.mem_toArray, List.mem_cons, List.mem_nil_iff, or_false] at mem
       subst mem
       cases eq
     simp only [$(caller.rawContract):term, $(caller.resultsCodec):term,
       LeanerIR.Proofs.Codec.decode_encode_apply]
     leaner_certified_close!))

/-! ## A struct returned as it came

A one-parameter function whose body is its struct parameter, by copy or
by move.  The parameter's twin encodes as its erasure, which is loan-free
by the twin's own lemmas, so the row is the erasure alone and the result
decodes by the codec roundtrip. -/

structure NominalReturnPlan where
  /-- The parameter's twin, root-qualified. -/
  twin : Name
  /-- Whether the body moves the parameter out (else copies it). -/
  move : Bool

/-- Recognize `local0` or `move local0` as the whole body of a
one-parameter, one-result function whose parameter is a struct. -/
def nominalReturnPlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (twin : Option Name) : Option NominalReturnPlan := do
  let twin ← twin
  guard (parameterCount == 1 && localCount == 1 && resultCount == 1)
  let tree := body.bindingBody!
  guard !tree.hasLooseBVars
  if tree.isAppOfArity ``localVar 1 then
    guard (natLeaf? (tree.getArg! 0) == some 0)
    pure { twin, move := false }
  else
    guard (tree.isAppOfArity ``nativeLocalOperation 2)
    guard ((tree.getArg! 1).isConstOf ``valuesNil)
    let operation := tree.getArg! 0
    guard (operation.isAppOfArity ``LocalLocationOperation.move 1)
    guard (natLeaf? (operation.getArg! 0) == some 0)
    pure { twin, move := true }

/-- The complete script for a struct returned as it came. -/
def nominalReturnScript (caller : CallerNames) (plan : NominalReturnPlan) :
    CommandElabM (TSyntax `tactic) := do
  let codec := mkIdent (plan.twin ++ `codec)
  let noBorrows := mkIdent (plan.twin ++ `outermostBorrows_erase)
  let noParameterLoans := mkIdent (plan.twin ++ `parameterLoanLocations_erase)
  let roundtrip := mkIdent (plan.twin ++ `decode?_erase)
  let closing ← `(tactic|
    (rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowFree _ _ _
       ?borrowFree]
     case borrowFree =>
       intro slot mem value eq
       simp only [List.mem_toArray, List.mem_cons, List.mem_nil_iff, or_false] at mem
       subst mem
       cases eq
       simp only [$noBorrows:ident]
     simp only [$(caller.rawContract):term, $(caller.resultsCodec):term, $codec:ident,
       $roundtrip:ident, Option.bind_some]
     leaner_certified_close!))
  if plan.move then
    `(tactic|
      (apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
       case arity => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
       case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
       simp only [$(caller.argumentsCodec):term, $(caller.shape):term, $codec:ident,
         LeanerIR.Proofs.Denotation.initialLocals_one, $noParameterLoans:ident]
       apply LeanerIR.Proofs.Denotation.wpRowThrow_moveLocal0 (rest := [])
       intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
       simp only [LeanerIR.SemanticOperations.finishControl?,
         LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
         Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
       subst $(n "rowFinished"):ident
       simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
       $closing:tactic))
  else
    let reads ← readBatch 1
    let finish ← scalarReturnedPrefix
    `(tactic|
      (apply LeanerIR.Proofs.Denotation.nativeEntry_rowFrame
       case arity => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
       case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
       case stable => leaner_row_stable
       simp only [$(caller.argumentsCodec):term, $(caller.shape):term, $codec:ident,
         LeanerIR.Proofs.Denotation.initialLocals_one, $noParameterLoans:ident]
       leaner_row_drive
       $[$reads:tactic]*
       $[$finish:tactic]*
       $closing:tactic))

/-! ## A generic value call

A generic function whose body returns a generic callee's result on its
moved parameter (`forward`).  The theorem is over abstract codecs: the
row is the parameter's encoding, moved out into the call, and the callee's
contract is consumed at the unit carrier and the identity codecs as the
monomorphic value call consumes it. -/

structure GenericCallPlan where
  parameter : String
  callee : CalleeNames
  handle : Nat × Nat

/-- Recognize `nativeCall f (values [move local0])` as the whole body of a
one-parameter, one-result generic function. -/
def genericCallPlan? (body : Lean.Expr) (parameterCount localCount resultCount : Nat)
    (parameter : String) : Option GenericCallPlan := do
  guard (parameterCount == 1 && localCount == 1 && resultCount == 1)
  let tree := body.bindingBody!
  guard (tree.isAppOfArity ``nativeCall 4)
  guard ((tree.getArg! 1).isAppOf ``Option.none)
  let callee := tree.getArg! 2
  let relation ← calleeRelation? callee
  let operand ← singleOperand? (tree.getArg! 3)
  guard (operand.isAppOfArity ``nativeLocalOperation 2)
  guard ((operand.getArg! 1).isConstOf ``valuesNil)
  let operation := operand.getArg! 0
  guard (operation.isAppOfArity ``LocalLocationOperation.move 1)
  guard (natLeaf? (operation.getArg! 0) == some 0)
  let names ← calleeNames? relation
  let handle ← handleIndices? (tree.getArg! 0)
  pure { parameter, callee := names, handle }

/-- The complete script for a generic value call. -/
def genericCallScript (caller : CallerNames) (plan : GenericCallPlan) :
    CommandElabM (TSyntax `tactic) := do
  let callee := plan.callee
  let value := mkIdent (Name.mkSimple plan.parameter)
  let argument ← `(term| ($(n "codecs"):ident 0).encode $value)
  let calleeContract := mkIdent callee.contract
  let calleeTyped := mkIdent callee.typedContract
  let calleeRaw := mkIdent callee.rawContract
  let calleeCodec := mkIdent callee.argumentsCodec
  let calleeResults := mkIdent callee.resultsCodec
  /- The generic theorem names the execution facts as its requirements. -/
  let renames ← bindVocabulary #[("requires_0", "prepared"), ("requires_1", "hu")]
  let calleeFact ← calleeSatFact caller callee (generic := true)
  `(tactic|
    ($renames:tactic
     $calleeFact:tactic
     apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
     case arity => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     case declared => simp [$(caller.argumentsCodec):term, $(caller.shape):term]
     simp only [$(caller.argumentsCodec):term, $(caller.shape):term]
     rw [show LeanerIR.SemanticOperations.initialLocals 1
         #[$argument] = #[some $argument] by
         simp [LeanerIR.SemanticOperations.initialLocals]; rfl]
     apply LeanerIR.Proofs.Denotation.wpRowThrow_callValueOperand
     apply LeanerIR.Proofs.Denotation.wpRowThrow_moveLocal0 (rest := [])
     leaner_row_head
     apply LeanerIR.Proofs.Denotation.wpFunction_of_satisfies
       $(calleeSatIdent callee)
     case permitted =>
       simp only [$calleeContract:term, LeanerIR.Proofs.Contract.runtime,
         $calleeTyped:term, LeanerIR.Proofs.Contract.typed, $calleeRaw:term,
         $calleeCodec:term, LeanerIR.Proofs.Codec.identity]
       exact ⟨⟨$argument⟩, rfl, $argument, rfl, $(n "freshLoans"):ident⟩
     case onReturn =>
       intro $(n "results"):ident $(n "final"):ident $(n "ensuresFn"):ident
         $(n "frameFn"):ident $(n "notMust"):ident
       have $(n "ensures"):ident := $(n "ensuresFn"):ident (by exact $(n "notMust"):ident)
       simp only [$calleeContract:term, LeanerIR.Proofs.Contract.runtime,
         $calleeTyped:term, LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
         at $(n "ensures"):ident $(n "frameFn"):ident
       obtain ⟨$(n "rowArgs"):ident, $(n "rowResult"):ident, $(n "rowArgsEq"):ident,
         $(n "rowDecodeEq"):ident, $(n "value"):ident, $(n "result"):ident,
         ⟨⟨$(n "argsEq2"):ident, $(n "resultsEq"):ident⟩, $(n "pendingEq"):ident⟩,
         $(n "obligation"):ident⟩ := $(n "ensures"):ident
       simp only [$calleeResults:term, LeanerIR.Proofs.Codec.identity]
         at $(n "rowDecodeEq"):ident $(n "resultsEq"):ident
       split at $(n "rowDecodeEq"):ident
       · rename_i $(n "r0"):ident $(n "listEq"):ident
         simp only [Option.bind_eq_some_iff, Option.some.injEq, exists_eq_right]
           at $(n "rowDecodeEq"):ident
         subst $(n "rowDecodeEq"):ident
         have $(n "resultsShape"):ident : $(n "results"):ident = #[$(n "r0"):ident] := by
           have $(n "listArray"):ident := congrArg List.toArray $(n "listEq"):ident
           simpa using $(n "listArray"):ident
         subst $(n "resultsShape"):ident
         simp only [Array.mk.injEq, List.cons.injEq, and_true] at $(n "resultsEq"):ident
         subst $(n "resultsEq"):ident
         rw [$(n "rowArgsEq"):ident] at $(n "argsEq2"):ident
         simp only [Array.mk.injEq, List.cons.injEq, and_true] at $(n "argsEq2"):ident
         subst $(n "argsEq2"):ident
         simp only [LeanerIR.Proofs.Obligation_iff] at $(n "obligation"):ident
         subst $(n "obligation"):ident
         obtain ⟨$(n "rowArgs2"):ident, $(n "rowArgsEq3"):ident, $(n "globalsEq"):ident,
           $(n "discipline"):ident⟩ := $(n "frameFn"):ident
         refine ⟨$(n "pendingEq"):ident, ?_⟩
         intro $(n "rowOutcome"):ident $(n "rowFinished"):ident
         have $(n "packOne"):ident : LeanerIR.SemanticOperations.packResults
             #[$argument] = $argument := rfl
         rw [$(n "packOne"):ident] at $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finishControl?,
           LeanerIR.SemanticOperations.unpackFallthrough, Option.map_eq_map,
           Option.map_some, Option.some.injEq] at $(n "rowFinished"):ident
         subst $(n "rowFinished"):ident
         simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
         rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowFree _ _ _
           ?borrowFree]
         case borrowFree =>
           intro slot mem value eq
           simp only [List.mem_toArray, List.mem_cons, List.mem_nil_iff, or_false] at mem
           subst mem
           cases eq
         simp only [$(caller.rawContract):term, $(caller.resultsCodec):term,
           LeanerIR.Proofs.Codec.decode_encode_apply]
         have $(n "rowDiscipline"):ident : LeanerIR.SemanticOperations.LoanDiscipline
             $(n "initial"):ident
             { globals := ($(n "final"):ident).globals
               globalLoans := ($(n "final"):ident).globalLoans
               nextLoan := ($(n "final"):ident).nextLoan
               pending := ($(n "initial"):ident).pending } :=
           ($(n "discipline"):ident).trans
             (LeanerIR.SemanticOperations.LoanDiscipline.of_eq rfl (Nat.le_refl _))
         leaner_certified_close!
       · exact absurd $(n "rowDecodeEq"):ident (by simp)
     case onThrow =>
       intro $(n "rowKind"):ident $(n "rowThrown"):ident $(n "final"):ident
         $(n "aborts"):ident
       simp only [$calleeContract:term, LeanerIR.Proofs.Contract.runtime,
         $calleeTyped:term, LeanerIR.Proofs.Contract.typed, $calleeRaw:term]
         at $(n "aborts"):ident
       obtain ⟨-, -, $(n "absurdAbort"):ident⟩ := $(n "aborts"):ident
       exact ($(n "absurdAbort"):ident).elim))

end LeanerLang.RowScript
