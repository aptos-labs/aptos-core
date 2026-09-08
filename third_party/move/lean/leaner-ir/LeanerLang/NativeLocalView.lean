-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.NativeFlowBody
import LeanerIR.Proofs.NativeObservedLoop

namespace LeanerLang.NativeLocalView

open Lean Meta Elab Command
open LeanerIR.Proofs.Denotation
open NativeFlowBody (Slots)

set_option quotPrecheck false

private partial def index? (expression : Lean.Expr) : Option Nat :=
  (expression.nat? <|> expression.rawNatLit?).orElse fun _ =>
    if expression.getAppNumArgs == 1 then index? (expression.getArg! 0) else none

partial def boundLocals (expression : Lean.Expr) (found : Array LeanerIR.LocalId := #[]) :
    Array LeanerIR.LocalId := Id.run do
  let mut found := found
  let slot := if expression.isAppOfArity ``letNativeValue 3 then
      let pattern := (expression.getArg! 0).getArg! 1
      if pattern.isAppOfArity ``LeanerIR.SemanticOperations.NativePattern.variable 1 then
        index? (pattern.getArg! 0) else none
    else if expression.isAppOfArity ``nativeAssignLocal 2 then index? (expression.getArg! 0)
    else none
  if let some index := slot then
    let slot : LeanerIR.LocalId := { index }
    unless found.contains slot do found := found.push slot
  for argument in expression.getAppArgs do
    found := boundLocals argument found
  return found

/-- Typed optional cells used only in agreement witnesses. Native loop and
statement-join arguments contain their live locals, not this residual state. -/
structure View where
  locals : Array Typed.LocalInfo
  dead : Array LeanerIR.LocalId
  frame : Array (Option Term) → Slots → CommandElabM Term

namespace View

def ghostType (view : View) : CommandElabM Term := do
  let mut result ← ``(Unit)
  for slot in view.dead.reverse do
    let type ← view.locals[slot.index]!.rep.typeSyntax (mkIdent `Carrier)
    result ← ``(Option $type × $result)
  return result

def unpackGhosts (view : View) (value : Term) : CommandElabM (Array (Option Term)) := do
  let mut ghosts := Array.replicate view.locals.size none
  let mut rest := value
  for slot in view.dead do
    ghosts := ghosts.set! slot.index (some (← ``(($rest).1)))
    rest ← ``(($rest).2)
  return ghosts

def packGhosts (view : View) (ghosts : Array (Option Term)) (slots : Slots) : CommandElabM Term := do
  let mut packed ← ``(())
  for slot in view.dead.reverse do
    let value ← match slots[slot.index]!, ghosts[slot.index]? |>.join with
      | some value, _ => ``(some $value)
      | none, some value => pure value
      | none, none => ``(none)
    packed ← ``(($value, $packed))
  return packed

def slots (view : View) (info : NativeLoopInfo.Loop) (value : Term) : CommandElabM Slots :=
  NativeFlowBody.unpack info (Array.replicate view.locals.size none) value

def frames (view : View) (info : NativeLoopInfo.Loop) : CommandElabM Term := do
  let type ← NativeFlowBody.headerType info
  let ghostType ← ghostType view
  let locals := mkIdent `observed_locals
  let dead := mkIdent `observed_dead
  let values ← slots view info locals
  let ghosts ← unpackGhosts view dead
  let frame ← view.frame ghosts values
  ``(fun $locals : $type => fun observed_frame : LeanerIR.RuntimeFrame =>
    Exists (fun $dead : $ghostType => observed_frame = $frame))

/-- Each ordinary join retains the locals available there. Abrupt control
instead carries the enclosing loop header, dropping lexical body locals. -/
def joinInfo (view : View) (loop : NativeLoopInfo.Loop) (slots : Slots) : NativeLoopInfo.Loop :=
  let indices := slots.zipIdx.filterMap fun (value, index) =>
    if value.isSome then some ({ index } : LeanerIR.LocalId) else none
  { loop with slots := indices, representations := indices.map (fun slot => view.locals[slot.index]!.rep) }

end View

end LeanerLang.NativeLocalView
