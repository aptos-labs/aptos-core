-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Compiler.Normalize

/-!
# Module elaboration

`lowerToIR ``Namespace` performs compilation during elaboration through
`Move.Compiler.LIR` into `MoveModel.IR`, then embeds semantic IR directly as
an ordinary Lean value. The quoted namespace is a registered Move module
identity, not a Lean value.
-/

namespace Move.Compiler

open Lean Meta Elab Term
open MoveModel.IR

/-! ## Quotation of finite semantic IR -/

deriving instance ToExpr for IntWidth
deriving instance ToExpr for NumType
deriving instance ToExpr for Ty
deriving instance ToExpr for TypeTagToken
deriving instance ToExpr for ResourceKey
deriving instance ToExpr for RefRoot
deriving instance ToExpr for RefTarget
deriving instance ToExpr for Value
deriving instance ToExpr for QuantKind
deriving instance ToExpr for SpecBinop
deriving instance ToExpr for SpecExp
deriving instance ToExpr for Oper
deriving instance ToExpr for Instr
deriving instance ToExpr for MoveModel.IR.Term
deriving instance ToExpr for Block
deriving instance ToExpr for AbilitySet
deriving instance ToExpr for TypeParamDecl
deriving instance ToExpr for MoveModel.IR.StructDecl
deriving instance ToExpr for Contract
deriving instance ToExpr for MoveModel.IR.Visibility
deriving instance ToExpr for Dialect
deriving instance ToExpr for AttributeArg
deriving instance ToExpr for MoveModel.IR.Attribute
deriving instance ToExpr for SourceSpan
deriving instance ToExpr for BlockSourceMap
deriving instance ToExpr for FunSourceMap
deriving instance ToExpr for StructMeta
deriving instance ToExpr for FunMeta
deriving instance ToExpr for ExternalFunRef
deriving instance ToExpr for ExternalModuleRef

private def materialize (kind : String) (count : Nat) (get : Nat → Option α) :
    TermElabM (List α) :=
  (List.range count).mapM fun i =>
    match get i with
    | some value => pure value
    | none => throwError "missing {kind} at in-range index {i}"

private def mkListExpr (type : Expr) : List Expr → MetaM Expr
  | [] => pure (mkApp (mkConst ``List.nil [0]) type)
  | head :: tail => do
    let tail ← mkListExpr type tail
    mkAppM ``List.cons #[head, tail]

private def quoteFunDecl (funDecl : MoveModel.IR.FunDecl) : TermElabM Expr := do
  for block in List.range funDecl.body.size do
    if (funDecl.loopSpecs block).isSome then
      throwError "source compiler produced unsupported loop metadata at block {block}"
  let locals ← materialize "local" funDecl.numLocals funDecl.locals
  let blocks ← materialize "block" funDecl.body.size funDecl.body.blocks
  mkAppM ``MoveModel.IR.FunDecl.ofLists #[
    toExpr funDecl.typeParams,
    toExpr funDecl.numParams,
    toExpr locals,
    toExpr funDecl.returns,
    toExpr blocks,
    toExpr funDecl.body.entry,
    toExpr funDecl.contract,
    toExpr funDecl.native]

private def quoteModule (module : MoveModel.IR.Module) : TermElabM Expr := do
  let structs ← materialize "struct declaration" module.numStructs module.program.structs
  let funs ← materialize "function declaration" module.numFuns module.program.funs
  let structMeta ← materialize "struct metadata" module.numStructs module.structMeta
  let funMeta ← materialize "function metadata" module.numFuns module.funMeta
  let funExprs ← funs.mapM quoteFunDecl
  let funsExpr ← mkListExpr (mkConst ``MoveModel.IR.FunDecl) funExprs
  mkAppM ``MoveModel.IR.Module.ofLists #[
    toExpr module.address,
    toExpr module.name,
    toExpr structs,
    funsExpr,
    toExpr structMeta,
    toExpr funMeta,
    toExpr module.externalFuns,
    toExpr module.dialect,
    toExpr module.friends]

private def resolveNames (idents : Array Syntax) : TermElabM (Array Name) :=
  idents.mapM resolveGlobalConstNoOverload

private def identifiers (listNode : Syntax) : Array Syntax :=
  listNode.getArgs.filter (·.isIdent)

private def taggedNamesInNamespace (ns : Name) (attrs : Array TagAttribute) :
    TermElabM (Array Name) := do
  let env ← getEnv
  let mut names : NameSet := {}
  for attr in attrs do
    let entries := (attr.ext.exportEntriesFn env (attr.ext.getState env)).private
    for name in entries do
      if ns.isPrefixOf name then
        names := names.insert name
  -- A tag extension's current state only records declarations elaborated in
  -- this module. Imported entries remain queryable through `hasTag`, so scan
  -- the environment to discover the tagged declarations of an imported Move
  -- namespace as well.
  for (name, _) in env.constants do
    if ns.isPrefixOf name && attrs.any (·.hasTag env name) then
      names := names.insert name
  let mut result := #[]
  for name in names do
    result := result.push name
  return result

private def discoverModuleDecls (ns : Name) : TermElabM (Array Name × Array Name) := do
  let structs ← taggedNamesInNamespace ns #[moveStructAttr, moveEnumAttr]
  let functions ← taggedNamesInNamespace ns
    #[moveFunAttr, movePublicAttr, moveFriendAttr, movePackageAttr, moveEntryAttr,
      moveNativeAttr]
  return (structs, functions)

scoped syntax (name := moveModuleTerm)
  "module%" str " structs " "[" ident,* "]" " functions " "[" ident,* "]" : term

/-- Compatibility syntax for the old compilation form. Its name makes the
implicit selection of declarations in the current namespace explicit. -/
scoped syntax (name := moveModuleFromContextTerm)
  "module_from_context%" str : term

/-- Lower one registered Move module namespace. ` ``Namespace` is deliberately
used instead of a `Name` value: a Move module is a namespace plus persistent
compiler metadata, not a first-class Lean value. -/
scoped syntax (name := lowerToIRTerm)
  "lowerToIR " Lean.Parser.Term.doubleQuotedName : term

open scoped Move.Compiler

private def elaborateModule (module : Move.ModuleRef) (structNames funNames : Array Name)
    (expectedType? : Option Expr) : TermElabM Expr := do
  let named ← compileModule module structNames funNames
  let ir ← match named.toIR with
    | .ok ir => pure ir
    | .error message => throwError message
  ensureHasType expectedType? (← quoteModule ir)

@[term_elab moveModuleTerm]
def elabMoveModule : TermElab := fun stx expectedType? => do
  let some name := stx[1].isStrLit? | throwErrorAt stx[1] "expected module name"
  let structNames ← resolveNames (identifiers stx[4])
  let funNames ← resolveNames (identifiers stx[8])
  elaborateModule { name } structNames funNames expectedType?

@[term_elab moveModuleFromContextTerm]
def elabMoveModuleFromContext : TermElab := fun stx expectedType? => do
  let some name := stx[1].isStrLit? | throwErrorAt stx[1] "expected module name"
  let ns ← getCurrNamespace
  let (structNames, funNames) ← discoverModuleDecls ns
  elaborateModule { name } structNames funNames expectedType?

@[term_elab lowerToIRTerm]
def elabLowerToIR : TermElab := fun stx expectedType? => do
  let namespaceIdent : TSyntax `ident := ⟨stx[1][2]⟩
  let ns ← resolveUniqueNamespace namespaceIdent
  let env ← getEnv
  let some module := Move.moduleForNamespace? env ns
    | throwErrorAt stx[1] "`{ns}` is not a registered Move module namespace"
  let (structNames, funNames) ← discoverModuleDecls ns
  elaborateModule module structNames funNames expectedType?

syntax (name := printLeanerIR)
  "#print_leaner_ir" str "structs" "[" ident,* "]" "functions" "[" ident,* "]" : command

@[command_elab printLeanerIR]
def elabPrintLeanerIR : Elab.Command.CommandElab := fun stx => do
  let moduleName := stx[1]
  let some name := moduleName.isStrLit? | throwErrorAt moduleName "expected module name"
  let structNames ← Elab.Command.liftTermElabM <| resolveNames (identifiers stx[4])
  let funNames ← Elab.Command.liftTermElabM <| resolveNames (identifiers stx[8])
  let named ← Elab.Command.liftCoreM <| compileModule { name } structNames funNames
  logInfo (repr named)

end Move.Compiler
