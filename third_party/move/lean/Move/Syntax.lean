-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import Move.Action
import Move.Attributes

/-!
# Leaner surface syntax

One scoped borrow parser covers reference types and Move 2 places. Assignment
uses Lean's existing `doReassign` syntax and dispatches by the left-hand local's
type: `MutRef α` writes through the reference, while ordinary `let mut` locals
fall back to Lean's built-in reassignment elaborator. `continue f args...`
marks one direct self-call for checked tail-call-to-loop lowering.
-/

namespace Move

open Lean Meta Elab Term

scoped syntax:max (name := borrowTerm) "&" term:40 : term
scoped syntax:max (name := borrowMutTerm) "&mut " term:40 : term
scoped syntax:max (name := borrowIndexTerm) (priority := high)
  "&" ident "[" term "]" : term
scoped syntax:max (name := borrowMutIndexTerm) (priority := high)
  "&mut " ident "[" term "]" : term
scoped syntax:max (name := derefTerm) "*" term:max : term
scoped syntax:max (name := vectorLiteralTerm) "vector![" term,* "]" : term
syntax:max (name := abortTerm) "abort " term:max : term
syntax:max (name := continueCallTerm)
  "continue " term:max term:max* : term

macro_rules
  | `(abort $code:term) => `(Move.abort $code)

macro_rules
  | `(continue $fn:term $args:term*) =>
      `(Move.continueMarker ($fn $args*))

macro_rules
  | `(vector![$values:term,*]) => do
      let values := values.getElems.toList
      let mut result ← `(Move.Vector.empty)
      for value in values do
        result ← `(Move.Vector.push $result $value)
      return result

private def expectedIsType (expectedType? : Option Expr) : TermElabM Bool := do
  let some expected ← pure expectedType? | return false
  return (← whnf expected).isSort

private def ownerType? (refType : Expr) (ctor : Name) : MetaM (Option Expr) := do
  let refType ← whnf refType
  if refType.isAppOfArity ctor 1 then
    return some refType.appArg!
  return none

private def projectionName? (owner : Expr) (field : Ident) : MetaM (Option Name) := do
  let .const ownerName _ := owner.getAppFn | return none
  let candidate := ownerName ++ field.getId
  return if (← getEnv).contains candidate then some candidate else none

private def elabFieldBorrow (mutable : Bool) (refStx : Term)
    (field : Ident) (expectedType? : Option Expr) : TermElabM Expr := do
  let refExpr ← elabTerm refStx none
  let refType ← inferType refExpr
  let refType ← whnf refType
  let (owner, refExpr) ←
    if mutable then
      let some owner ← ownerType? refType ``MutRef
        | throwErrorAt refStx "expected a mutable Move reference"
      pure (owner, refExpr)
    else if let some owner ← ownerType? refType ``Ref then
      pure (owner, refExpr)
    else if let some owner ← ownerType? refType ``MutRef then
      pure (owner, ← mkAppM ``freezeRef #[refExpr])
    else
      throwErrorAt refStx "expected a Move reference"
  let some projectionName ← projectionName? owner field
    | throwErrorAt field "`{field.getId}` is not a field of {owner}"
  -- Structure projections carry the structure's type parameters before the
  -- owner argument. Building a lambda lets elaboration infer those parameters
  -- from the concrete referent (`Map K V`, for example) instead of passing the
  -- still-polymorphic projection to `fieldOfProjection`.
  let projection ← withLocalDeclD `_moveFieldOwner owner fun ownerVar => do
    let projected ← mkAppM projectionName #[ownerVar]
    mkLambdaFVars #[ownerVar] projected
  let descriptor ← mkAppM ``fieldOfProjection #[projection]
  let primitive := if mutable then ``borrowFieldMut else ``borrowField
  let result ← mkAppM primitive #[refExpr, descriptor]
  ensureHasType expectedType? result

private def fieldParts : Name → List String
  | .anonymous => []
  | .str baseName part => fieldParts baseName ++ [part]
  | .num _ _ => []

/-- Elaborate an argument to a Move function, inserting Move's implicit
`&mut T` to `&T` freeze before ordinary coercion elaboration gets stuck on
generic referent types. -/
private def elabMoveArgument (argument : Term) (expectedType : Expr) : TermElabM Expr := do
  if argument.raw.isIdent then
    let decl? ←
      try
        pure (some (← getLocalDeclFromUserName argument.raw.getId))
      catch _ => pure none
    if let some decl := decl? then
      let actualType ← whnf decl.type
      let expectedType ← whnf expectedType
      if actualType.isAppOfArity ``MutRef 1 && expectedType.isAppOfArity ``Ref 1 then
        unless ← isDefEq actualType.appArg! expectedType.appArg! do
          throwErrorAt argument "mutable reference has incompatible referent type"
        let value ← elabTerm argument (some actualType)
        return ← mkAppM ``freezeRef #[value]
  elabTerm argument (some expectedType)

/-- Move calls infer immutable views of mutable-reference arguments.  Lean's
standard coercion elaborator cannot always infer this conversion when a
generic parameter occurs only under `Ref`, so tagged Move functions use this
small application elaborator. Other applications fall through unchanged. -/
@[term_elab Lean.Parser.Term.app]
def elabMoveFunctionApplication : TermElab := fun stx expectedType? => do
  let `($function:ident $arguments:term*) := stx
    | throwUnsupportedSyntax
  let functionName ←
    try resolveGlobalConstNoOverload function
    catch _ => throwUnsupportedSyntax
  let env ← getEnv
  unless moveFunAttr.hasTag env functionName || movePublicAttr.hasTag env functionName ||
      moveEntryAttr.hasTag env functionName do
    throwUnsupportedSyntax
  let functionExpr ← elabTerm function none
  let (parameters, binderInfos, _) ← forallMetaTelescope (← inferType functionExpr)
  let explicitParameters := (parameters.zip binderInfos).filter (·.2.isExplicit)
  unless explicitParameters.size == arguments.size do
    throwUnsupportedSyntax
  for ((parameter, _), argument) in explicitParameters.zip arguments do
    let parameterType ← instantiateMVars (← inferType parameter)
    let value ← elabMoveArgument argument parameterType
    unless ← isDefEq parameter value do
      throwErrorAt argument "argument has type {← inferType value}, expected {parameterType}"
  synthesizeSyntheticMVars
  ensureHasType expectedType? (mkAppN functionExpr parameters)

/-- Turn a chained place into nested `do` expressions.  Each generated borrow
is elaborated normally, so owner and field types are checked at every edge. -/
private partial def chainBorrowSyntax (mutable : Bool) (origin : Syntax)
    (effect : Term) : List String → TermElabM Term
  | [] => pure effect
  | fieldName :: rest => do
      let ref := mkIdentFrom origin (Name.mkSimple "_leanerRef")
      let field := mkIdentFrom origin (Name.mkSimple fieldName)
      let primitive := mkIdent (if mutable then ``borrowFieldMut else ``borrowField)
      let next ← `($primitive $ref (fieldOfProjection (fun owner => owner.$field)))
      let tail ← chainBorrowSyntax mutable origin next rest
      `($effect >>= fun $ref => $tail)

private partial def elabBorrow (mutable : Bool) (place : Term)
    (expectedType? : Option Expr) : TermElabM Expr := do
  if ← expectedIsType expectedType? then
    let ctor := if mutable then ``MutRef else ``Ref
    return ← elabTermEnsuringType (← `($(mkIdent ctor) $place)) expectedType?
  if place.raw.isOfKind ``Lean.Parser.Term.paren then
    return ← elabBorrow mutable ⟨place.raw[1]⟩ expectedType?
  match place with
  | `($base[$index]) =>
      let local? ← if base.raw.isIdent then
        try
          pure (some (← getLocalDeclFromUserName base.raw.getId))
        catch _ => pure none
      else pure none
      match local? with
      | some decl =>
          let type ← whnf decl.type
          if type.isAppOfArity ``Ref 1 then
            let referent ← whnf type.appArg!
            unless referent.isAppOfArity ``Move.Vector 1 do
              throwErrorAt base "indexed borrow expects a Move vector reference"
            if mutable then
              throwErrorAt base "cannot mutably borrow through immutable reference `{base.raw.getId}`"
            elabTerm (← `(borrowElem $base $index)) expectedType?
          else if type.isAppOfArity ``MutRef 1 then
            let referent ← whnf type.appArg!
            unless referent.isAppOfArity ``Move.Vector 1 do
              throwErrorAt base "indexed borrow expects a Move vector reference"
            if mutable then
              elabTerm (← `(borrowElemMut $base $index)) expectedType?
            else
              elabTerm (← `(do
                let _leanerVectorRef ← freeze $base
                borrowElem _leanerVectorRef $index)) expectedType?
          else if type.isAppOfArity ``Move.Vector 1 then
            let localBorrow := if mutable then ``borrowLocalMut else ``borrowLocal
            let elemBorrow := if mutable then ``borrowElemMut else ``borrowElem
            let ref := mkIdentFrom base `_leanerVectorRef
            let localCall ← `($(mkIdent localBorrow) $base)
            let elemCall ← `($(mkIdent elemBorrow) $ref $index)
            elabTerm (← `($localCall >>= fun $ref => $elemCall)) expectedType?
          else
            throwErrorAt base "indexed borrow expects a Move vector or vector reference"
      | none =>
          let primitive := if mutable then ``borrowGlobalMut else ``borrowGlobal
          elabTerm (← `($(mkIdent primitive) $base $index)) expectedType?
  | _ =>
      if place.raw.isIdent then
        try
          let decl ← getLocalDeclFromUserName place.raw.getId
          let type ← whnf decl.type
          -- An identifier in value position is a local borrow. Reference type
          -- notation was already handled above through `expectedIsType`.
          unless type.isSort do
            let primitive := if mutable then ``borrowLocalMut else ``borrowLocal
            return ← elabTerm (← `($(mkIdent primitive) $place)) expectedType?
        catch _ => pure ()
      if place.raw.isOfKind ``Lean.Parser.Term.proj then
        let args := place.raw.getArgs
        if let some field := args[2]? then
          if field.isIdent then
            let base : Term := ⟨args[0]!⟩
            let root ← if mutable then `(&mut $base) else `(& $base)
            let chained ← chainBorrowSyntax mutable place.raw root (fieldParts field.getId)
            return ← elabTerm chained expectedType?
      if place.raw.isIdent then
        match place.raw.getId with
        | .str baseName fieldName =>
            let ref := mkIdentFrom place baseName
            let field := mkIdentFrom place (Name.mkSimple fieldName)
            return ← elabFieldBorrow mutable ref field expectedType?
        | _ => pure ()
      -- With no expected type (for example under `#check`), the remaining
      -- unambiguous form is a reference type.
      let ctor := if mutable then ``MutRef else ``Ref
      elabTerm (← `($(mkIdent ctor) $place)) expectedType?

@[term_elab borrowTerm]
def elabBorrowTerm : TermElab := fun stx expectedType? => do
  let `(& $place) := stx | throwUnsupportedSyntax
  elabBorrow false place expectedType?

@[term_elab borrowMutTerm]
def elabBorrowMutTerm : TermElab := fun stx expectedType? => do
  let `(&mut $place) := stx | throwUnsupportedSyntax
  elabBorrow true place expectedType?

@[term_elab borrowIndexTerm]
def elabBorrowIndexTerm : TermElab := fun stx expectedType? => do
  let `(& $base:ident[$index]) := stx | throwUnsupportedSyntax
  elabBorrow false (← `($base[$index])) expectedType?

@[term_elab borrowMutIndexTerm]
def elabBorrowMutIndexTerm : TermElab := fun stx expectedType? => do
  let `(&mut $base:ident[$index]) := stx | throwUnsupportedSyntax
  elabBorrow true (← `($base[$index])) expectedType?

/-- Read through either kind of Move reference.  The operation remains in
`Action`, so it composes with native `do` notation as `let x ← *ref`. -/
@[term_elab derefTerm]
def elabDerefTerm : TermElab := fun stx expectedType? => do
  let `(* $ref) := stx | throwUnsupportedSyntax
  let refExpr ← elabTerm ref none
  let type ← whnf (← inferType refExpr)
  let primitive ←
    if type.isAppOfArity ``MutRef 1 then pure ``read
    else if type.isAppOfArity ``Ref 1 then pure ``readImm
    else throwErrorAt ref "expected a Move reference after `*`"
  ensureHasType expectedType? (← mkAppM primitive #[refExpr])

open Lean.Elab.Do Lean.Parser.Term in
@[doElem_elab Lean.Parser.Term.doReassign]
def elabMoveReassign : DoElab := fun stx cont => do
  match stx with
  | `(doReassign| $x:ident $[: $_]? :=%$_ $rhs) =>
      let decl ← getLocalDeclFromUserName x.getId
      let type ← whnf decl.type
      if type.isAppOfArity ``MutRef 1 then
        match rhs with
        | `(* $ref - $amount) =>
            let old := mkIdentFrom rhs `_leanerOld
            elabDoElems1
              #[(← `(doElem| let $old ← read $ref)),
                (← `(doElem| write $x ($old - $amount)))] cont
        | `(* $ref + $amount) =>
            let old := mkIdentFrom rhs `_leanerOld
            elabDoElems1
              #[(← `(doElem| let $old ← read $ref)),
                (← `(doElem| write $x ($old + $amount)))] cont
        | _ => elabDoElem (← `(doElem| write $x $rhs)) cont
      else if type.isAppOfArity ``Ref 1 then
        throwErrorAt x "cannot write through immutable reference `{x.getId}`"
      else
        throwUnsupportedSyntax
  | _ => throwUnsupportedSyntax

end Move
