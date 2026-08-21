-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Compiler.Normalize
import MoveModel.Frontend.Elab
import MoveModel.Frontend.XIR.FromIR

/-!
# Module elaboration

`move_module%` performs compilation during elaboration through
`Move.Compiler.LIR`, `MoveModel.IR`, and `MoveModel.Frontend.XIR`, then embeds the first-order
`MModule` as an ordinary Lean value.
-/

namespace Move.Compiler

open Lean Elab Term
open MoveModel.Frontend.XIR

private def resolveNames (idents : Array Syntax) : TermElabM (Array Name) :=
  idents.mapM resolveGlobalConstNoOverload

private def identifiers (listNode : Syntax) : Array Syntax :=
  listNode.getArgs.filter (·.isIdent)

private def localTaggedNames (ns : Name) (attrs : Array TagAttribute) : TermElabM (Array Name) := do
  let env ← getEnv
  let mut names : NameSet := {}
  for attr in attrs do
    let entries := (attr.ext.exportEntriesFn env (attr.ext.getState env)).private
    for name in entries do
      if ns.isPrefixOf name then
        names := names.insert name
  let mut result := #[]
  for name in names do
    result := result.push name
  return result

private def discoverModuleDecls : TermElabM (Array Name × Array Name) := do
  let ns ← getCurrNamespace
  let structs ← localTaggedNames ns #[moveStructAttr, moveEnumAttr]
  let functions ← localTaggedNames ns #[moveFunAttr, movePublicAttr, moveEntryAttr]
  return (structs, functions)

scoped syntax (name := moveModuleTerm)
  "move_module%" str
    (" structs " "[" ident,* "]" " functions " "[" ident,* "]")? : term

open scoped Move.Compiler

private def elaborateModule (moduleName : Syntax) (structNames funNames : Array Name)
    (expectedType? : Option Expr) : TermElabM Expr := do
  let some name := moduleName.isStrLit? | throwErrorAt moduleName "expected module name"
  let named ← compileModule name structNames funNames
  let ir ← match named.toIR with
    | .ok ir => pure ir
    | .error message => throwError message
  let xir ← match MModule.ofIR ir with
    | .ok xir => pure xir
    | .error message => throwError message
  ensureHasType expectedType? (toExpr xir)

@[term_elab moveModuleTerm]
def elabMoveModule : TermElab := fun stx expectedType? => do
  let (structNames, funNames) ←
    if stx[2].isNone then
      discoverModuleDecls
    else
      let selection := stx[2]
      pure (← resolveNames (identifiers selection[2]),
        ← resolveNames (identifiers selection[6]))
  elaborateModule stx[1] structNames funNames expectedType?

syntax (name := printLeanerIR)
  "#print_leaner_ir" str "structs" "[" ident,* "]" "functions" "[" ident,* "]" : command

@[command_elab printLeanerIR]
def elabPrintLeanerIR : Elab.Command.CommandElab := fun stx => do
  let moduleName := stx[1]
  let some name := moduleName.isStrLit? | throwErrorAt moduleName "expected module name"
  let structNames ← Elab.Command.liftTermElabM <| resolveNames (identifiers stx[4])
  let funNames ← Elab.Command.liftTermElabM <| resolveNames (identifiers stx[8])
  let named ← Elab.Command.liftCoreM <| compileModule name structNames funNames
  logInfo (repr named)

end Move.Compiler
