-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import LeanerIR.Relocate.Class

/-!
# Deriving `Remap`

`deriving instance Remap for T` generates the relocation of `T`: each
constructor rebuilt from the relocations of its fields, a proof field kept
as it is. Recursive and nested types go through local instances, as Lean's
own `BEq` derivation does.
-/

namespace LeanerIR.Relocate.Deriving

open Lean Elab Command Term Meta Deriving Parser.Term

private def mkAlts (indVal : InductiveVal) (functions : Ident) :
    TermElabM (Array (TSyntax ``matchAlt)) :=
  indVal.ctors.toArray.mapM fun ctorName => do
    let ctorInfo ← getConstInfoCtor ctorName
    forallTelescopeReducing ctorInfo.type fun xs _ => do
      let mut patterns : Array Term := #[]
      for _ in [:indVal.numIndices] do
        patterns := patterns.push (← `(_))
      let mut ctorArgs : Array Term := #[]
      let mut rhsArgs : Array Term := #[]
      for _ in [:indVal.numParams] do
        ctorArgs := ctorArgs.push (← `(_))
        rhsArgs := rhsArgs.push (← `(_))
      for i in [:ctorInfo.numFields] do
        let x := xs[indVal.numParams + i]!
        let a := mkIdent (← mkFreshUserName `a)
        ctorArgs := ctorArgs.push a
        if ← isProp (← inferType x) then
          rhsArgs := rhsArgs.push a
        else
          rhsArgs := rhsArgs.push (← `((← LeanerIR.Remap.remap $a $functions)))
      patterns := patterns.push (← `(@$(mkIdent ctorName):ident $ctorArgs:term*))
      `(matchAltExpr| | $[$patterns:term],* => do return @$(mkIdent ctorName):ident $rhsArgs:term*)

private def mkAuxFunction (ctx : Deriving.Context) (i : Nat) : TermElabM Command := do
  let auxFunName := ctx.auxFunNames[i]!
  let indVal := ctx.typeInfos[i]!
  let header ← mkHeader ``Remap 1 indVal
  let functions := mkIdent `functions
  let monad := mkIdent `m
  let discrs ← mkDiscrs header indVal
  let alts ← mkAlts indVal functions
  let mut body : Term ← `(match $[$discrs],* with $alts:matchAlt*)
  if ctx.usePartial then
    let letDecls ← mkLocalInstanceLetDecls ctx ``Remap header.argNames
    body ← mkLet letDecls body
  let binders := header.binders
  if ctx.usePartial then
    `(partial def $(mkIdent auxFunName):ident $binders:bracketedBinder*
        {$monad : Type → Type} [Monad $monad] ($functions : LeanerIR.IdFns $monad) :
        $monad $(header.targetType) := $body:term)
  else
    `(def $(mkIdent auxFunName):ident $binders:bracketedBinder*
        {$monad : Type → Type} [Monad $monad] ($functions : LeanerIR.IdFns $monad) :
        $monad $(header.targetType) := $body:term)

private def mkRemapInstanceCmds (declName : Name) : TermElabM (Array Syntax) := do
  let ctx ← mkContext ``Remap "remap" declName (supportsRec := false)
  let auxDefs ← (List.range ctx.typeInfos.size).toArray.mapM (mkAuxFunction ctx)
  let block ← `(mutual $auxDefs:command* end)
  return #[block] ++ (← mkInstanceCmds ctx ``Remap #[declName])

def mkRemapInstanceHandler (declNames : Array Name) : CommandElabM Bool := do
  unless ← declNames.allM isInductive do return false
  for declName in declNames do
    let cmds ← liftTermElabM <| mkRemapInstanceCmds declName
    cmds.forM elabCommand
  return true

initialize registerDerivingHandler ``Remap mkRemapInstanceHandler

end LeanerIR.Relocate.Deriving
