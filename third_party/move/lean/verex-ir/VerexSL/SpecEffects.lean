-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR

/-!
# What a specification reading may discard

A specification reads a block statement that assigns no local by dropping it, so
the statement must have no effect on what the rest of the block reads. The nodes
that may be dropped are listed; anything else fails closed. The lowering checks a
derived body against this list, and the clause translator checks what reaches it.
-/

namespace VerexSL

open VerexIR

/-- What an operation does that a dropped statement would lose, if anything. A
`functionCall` names a function the lowering has found pure. -/
private def operationEffect : Operation → Option String
  | .primitive _ | .data _ | .specification _ | .assert | .read _ | .readDiscriminant _ | .copy _ | .move _ |
      .borrow .immutable _ | .reference (.borrow .immutable) | .reference .dereference |
      .reference (.freeze _) | .global .contains | .global (.borrow .immutable) |
      .call (.constructor ..) => none
  | .write _ => some "a write of a place"
  | .drop _ => some "a drop"
  | .borrow .mutable _ | .reference (.borrow .mutable) => some "a `&mut` borrow"
  | .reference .mutate => some "a mutation through a reference"
  | .global .take | .global .publish | .global (.borrow .mutable) =>
      some "a write of global memory"
  | .call (.function _) => some "an executable call"
  | .call (.closure ..) => some "a closure"
  | .call .invoke => some "a call of a function value"
  | .call (.extension ..) | .call (.destructor ..) | .profile .. | .borrow (.profile _) _ |
      .reference (.borrow (.profile _)) | .global (.borrow (.profile _)) =>
      some "a profile operation"

/-- The first effect of an expression a specification would drop. A failure is
none: it imposes no condition, the reading being partial. -/
partial def droppedEffect? (expressions : Array Expr) (id : ExprId) : Option String :=
  let recur := droppedEffect? expressions
  match expressions[id.index]? with
  | none => some "a missing expression"
  | some expression => match expression.kind with
    | .value .. | .constant _ | .localVar _ | .quantifier .. | .spec _ => none
    | .operation operation _ arguments _ =>
        (operationEffect operation).orElse fun _ => arguments.findSome? recur
    | .throw_ _ arguments => arguments.findSome? recur
    | .block statements result =>
        (statements ++ result.toArray).findSome? recur
    | .letDecl _ initializer body =>
        (initializer.toArray.push body).findSome? recur
    | .ifElse condition thenBranch elseBranch =>
        (#[condition, thenBranch] ++ elseBranch.toArray).findSome? recur
    | .match_ scrutinee arms =>
        (#[scrutinee] ++ arms.flatMap fun arm => arm.guard.toArray.push arm.body).findSome? recur
    | .loop .. => some "a loop"
    | .break_ .. | .continue_ _ | .return_ _ => some "a control transfer"
    | .assign .. | .assignPattern .. => some "an assignment"

/-- The first node under a dropped statement that the impurity scan should have
reported. An assignment and a control transfer in a dropped statement are not:
the reading of the statement fails there rather than losing them. -/
private partial def scanMissed? (expressions : Array Expr) (id : ExprId) : Option String :=
  let recur := scanMissed? expressions
  match expressions[id.index]? with
  | none => none
  | some expression => match expression.kind with
    | .quantifier .. | .spec _ => none
    | .operation (.call (.closure ..)) _ arguments _ => arguments.findSome? recur
    | .operation operation _ arguments _ =>
        (operationEffect operation).orElse fun _ => arguments.findSome? recur
    | kind => (VerexIR.Validation.expressionChildren kind).findSome? recur

/-- What the derived reading of a body would drop with an effect. A statement
assigning a local is what the reading threads. -/
partial def bodyDroppedEffect? (expressions : Array Expr) (id : ExprId) : Option String :=
  let recur := bodyDroppedEffect? expressions
  match expressions[id.index]? with
  | none => none
  | some expression => match expression.kind with
    | .block statements result =>
        (statements.findSome? fun (statement : ExprId) =>
          let node : Option Expr := expressions[statement.index]?
          match node.map (·.kind) with
          | some (ExprKind.assign _ value) => recur value
          | _ => scanMissed? expressions statement).orElse fun _ => result.bind recur
    | kind => (VerexIR.Validation.expressionChildren kind).findSome? recur

end VerexSL
