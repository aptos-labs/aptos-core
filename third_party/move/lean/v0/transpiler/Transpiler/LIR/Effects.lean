-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.Validated
import Transpiler.Effects
import Transpiler.LIR.MoveNames

/-!
# Effect inference over validated LIR

The Leaner source backend needs two package-wide facts: whether a function
prints as an `Action`, and whether its transitive call graph writes global
storage. This analysis is defined over `ValidatedUnit`; the XAST-shaped table
returned to the established printer is only a name adapter at the end.
-/

namespace Transpiler.LIR.Effects

open LeanerIR LeanerIR.Validation

abbrev Facts := Transpiler.Effects.Facts
abbrev Table := List (QualifiedRef × Facts)

private def merge (left right : Facts) : Facts :=
  { isAction := left.isAction || right.isAction
    writesGlobal := left.writesGlobal || right.writesGlobal }

def Table.get (table : Table) (name : QualifiedRef) : Facts :=
  (table.find? fun (candidate, _) => candidate == name).map (·.2) |>.getD {}

private def referencedName? (unit : ValidatedUnit) (reference : QualifiedRef) :
    Option (NamespaceRef × String) :=
  MoveNames.resolvedName? unit.tables reference

private def vectorCall? (unit : ValidatedUnit) (reference : QualifiedRef) :
    Option Transpiler.Names.VecOp := do
  let (namespaceRef, name) ← referencedName? unit reference
  match namespaceRef.segments with
  | #[address, _, moduleName] =>
      if address == "0x1" && moduleName == "vector" then Transpiler.Names.vecOp name else none
  | _ => none

private def certified (unit : ValidatedUnit) (reference : QualifiedRef) : Bool :=
  unit.namespaces.any fun ns => ns.identity == reference.namespaceId &&
    ns.structs.any fun declaration => declaration.name == reference.name &&
      declaration.contract.conditions.any (·.kind == .structInvariant)

private def expression? (ns : ValidatedNamespace) (id : ExprId) : Option Expr :=
  ns.expressions[id.index]?

private def expressionHasReferenceType (ns : ValidatedNamespace) (id : ExprId) : Bool :=
  (expression? ns id).any fun expression =>
    (ns.tables.types[expression.typeId.index]?).any fun
      | .reference _ => true
      | _ => false

private def localPlace (ns : ValidatedNamespace) (id : PlaceId) : Bool :=
  (ns.places[id.index]?).any fun
    | .localVar _ => true
    | _ => false

/-- Whether an expression is a borrow of a local or parameter, the precise
shape for which the transitional vector printer absorbs the borrow. -/
private def isLocalBorrow (ns : ValidatedNamespace) (id : ExprId) : Bool :=
  match expression? ns id with
  | some expression => match expression.kind with
      | .operation (.reference (.borrow _)) _ arguments _ =>
          match arguments.toList with
          | [argument] => (expression? ns argument).any fun candidate =>
              match candidate.kind with | .localVar _ => true | _ => false
          | _ => false
      | .operation (.borrow _ place) _ arguments _ =>
          arguments.isEmpty && localPlace ns place
      | _ => false
  | _ => false

private def operationFacts (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (table : Table) (operation : Operation) (arguments : Array ExprId) : Facts :=
  match operation with
  | .call (.function callee) =>
      match vectorCall? unit callee with
      | some vectorOperation =>
          let pureOperand := arguments[0]?.any (isLocalBorrow ns)
          let pureOperation := match vectorOperation with
            | .pushBack | .length | .isEmpty => pureOperand
            | .empty | .singleton | .destroyEmpty | .contains | .indexOf => true
            | _ => false
          if pureOperation then {} else { isAction := true }
      | none => table.get callee
  | .call .invoke =>
      -- Function types carry no effect row, so invocation remains
      -- conservative, matching the established Leaner interpretation.
      { isAction := true, writesGlobal := true }
  | .call (.constructor constructor none) =>
      if certified unit constructor then { isAction := true } else {}
  | .global (.borrow .mutable) | .global .take | .global .publish =>
      { isAction := true, writesGlobal := true }
  | .global (.borrow _) | .global .contains => { isAction := true }
  | .borrow _ _ | .read _ | .write _ => { isAction := true }
  | .reference _ | .assert => { isAction := true }
  | .data (.select ..) | .data (.selectVariants ..) | .data (.testVariants ..) =>
      if arguments.any (expressionHasReferenceType ns ·) then { isAction := true } else {}
  | _ => {}

/-- Syntactic effect facts of one structured LIR expression. -/
partial def expressionFacts (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (table : Table) (id : ExprId) : Facts :=
  let fold (expressions : Array ExprId) : Facts :=
    expressions.foldl (fun facts expression =>
      merge facts (expressionFacts unit ns table expression)) {}
  match expression? ns id with
  | none => {}
  | some expression => match expression.kind with
    | .value .. | .constant _ | .localVar _ | .break_ .. | .continue_ _ | .spec _ => {}
    | .operation operation _ arguments _ =>
        let argumentFacts := fold arguments
        let argumentFacts := match operation, vectorCallForOperation operation with
          | .call (.function _), some vectorOperation =>
              match vectorOperation, arguments.toList with
              | .pushBack, first :: rest | .length, first :: rest | .isEmpty, first :: rest
              | .borrow, first :: rest =>
                  if isLocalBorrow ns first then fold rest.toArray else argumentFacts
              | _, _ => argumentFacts
          | _, _ => argumentFacts
        merge (operationFacts unit ns table operation arguments) argumentFacts
    | .block statements result => fold (statements ++ result.toArray)
    | .letDecl _ value body =>
        merge (value.map (expressionFacts unit ns table) |>.getD {})
          (expressionFacts unit ns table body)
    | .ifElse condition thenBranch elseBranch =>
        fold (#[condition, thenBranch] ++ elseBranch.toArray)
    | .match_ scrutinee arms =>
        let scrutineeRead : Facts :=
          if expressionHasReferenceType ns scrutinee then { isAction := true } else {}
        let armFacts := arms.foldl (fun facts arm =>
          merge facts <| merge (arm.guard.map (expressionFacts unit ns table) |>.getD {})
            (expressionFacts unit ns table arm.body)) {}
        merge scrutineeRead <| merge (expressionFacts unit ns table scrutinee) armFacts
    | .loop _ body => expressionFacts unit ns table body
    | .return_ values => fold values
    | .throw_ _ arguments => merge { isAction := true } (fold arguments)
    | .assign _ value | .assignPattern _ value => expressionFacts unit ns table value
    | .quantifier .. => {}
where
  vectorCallForOperation : Operation → Option Transpiler.Names.VecOp
    | .call (.function callee) => vectorCall? unit callee
    | _ => none

private def isMutableReference (ns : ValidatedNamespace) (typeUse : TypeUse) : Bool :=
  (ns.tables.types[typeUse.typeId.index]?).any fun
    | .reference reference => reference.kind == .mutable
    | _ => false

private def isReference (ns : ValidatedNamespace) (typeUse : TypeUse) : Bool :=
  (ns.tables.types[typeUse.typeId.index]?).any fun
    | .reference _ => true
    | _ => false

private def isEntry (function : FunctionDecl FunctionBody) : Bool :=
  function.profileData.any (·.tag == "function.entry")

/-- Effect facts of one checked function under the current call table. -/
def functionFacts (unit : ValidatedUnit) (ns : ValidatedNamespace) (table : Table)
    (function : FunctionDecl FunctionBody) : Facts :=
  match function.body with
  | .absent =>
      let mutatesParameter := function.signature.parameters.any
        (isMutableReference ns ·.typeUse)
      let returnsReference := function.signature.results.any (isReference ns)
      { isAction := isEntry function || mutatesParameter || returnsReference }
  | .structured root =>
      let facts := expressionFacts unit ns table root
      { facts with isAction := facts.isAction || isEntry function }

/-- Least fixed point of effect facts over every checked function in the unit. -/
def compute (unit : ValidatedUnit) : Table :=
  let functions : List (QualifiedRef × ValidatedNamespace × FunctionDecl FunctionBody) :=
    unit.namespaces.toList.flatMap fun ns => ns.functions.toList.map fun function =>
      ({ namespaceId := ns.identity, name := function.name }, ns, function)
  let step (table : Table) : Table := functions.map fun (name, ns, function) =>
    (name, functionFacts unit ns table function)
  let rec iterate (table : Table) (fuel : Nat) : Table :=
    match fuel with
    | 0 => table
    | fuel + 1 =>
        let next := step table
        if next == table then table else iterate next fuel
  iterate (functions.map fun (name, _, _) => (name, {})) (functions.length + 2)

/-- Adapts neutral qualified references to the transitional printer table. No
expression or declaration analysis occurs in this conversion. -/
def toPrinterTable (unit : ValidatedUnit) (table : Table) :
    Except String Transpiler.Effects.Table :=
  table.mapM fun (name, facts) => return (← MoveNames.qualifiedRef unit.tables name, facts)

def computePrinterTable (unit : ValidatedUnit) : Except String Transpiler.Effects.Table :=
  toPrinterTable unit (compute unit)

end Transpiler.LIR.Effects
