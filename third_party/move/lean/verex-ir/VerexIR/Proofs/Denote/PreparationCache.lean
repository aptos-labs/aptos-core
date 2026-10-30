-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-! Scoped results of preparation requests. A proved fact survives context
extension when all of its dependencies still denote the same declarations.
A no-progress result is reusable only at the identical context revision. -/

namespace VerexIR.Proofs.Denote.PreparationCache

open Lean Meta

private structure Declaration where
  id : FVarId
  name : Name
  type : Lean.Expr
  value? : Option Lean.Expr
  binderInfo : BinderInfo
  kind : LocalDeclKind
  deriving BEq

private def declaration? (decl : Lean.LocalDecl) : MetaM (Option Declaration) := do
  let type ← instantiateMVars decl.type
  let value? ← decl.value?.mapM instantiateMVars
  if type.hasMVar || type.hasLooseBVars ||
      value?.any (fun value => value.hasMVar || value.hasLooseBVars) then return none
  return some {
    id := decl.fvarId, name := decl.userName, type, value?, binderInfo := decl.binderInfo, kind := decl.kind }

private def dependencies? (terms : Array Lean.Expr) : MetaM (Option (Array Declaration)) := do
  let mut pending := (terms.foldl collectFVars {}).fvarIds
  let mut seen : Std.HashSet FVarId := {}
  let mut declarations := #[]
  while let some id := pending.back? do
    pending := pending.pop
    if seen.contains id then continue
    seen := seen.insert id
    let some decl := (← getLCtx).find? id | return none
    let some snapshot ← declaration? decl | return none
    declarations := declarations.push snapshot
    let dependencies := collectFVars
      (snapshot.value?.map (collectFVars {}) |>.getD {}) snapshot.type
    pending := pending ++ dependencies.fvarIds
  return some declarations

private def revision? : MetaM (Option (Array Declaration)) := do
  let mut declarations := #[]
  for decl in ← getLCtx do
    let some snapshot ← declaration? decl | return none
    declarations := declarations.push snapshot
  return some declarations

private def instances : MetaM (Array Lean.Expr) := do
  return (← getLocalInstances).map (·.fvar)

private structure Entry where
  proof? : Option Lean.Expr
  declarations : Array Declaration
  instances : Array Lean.Expr
  options : Options
  /-- Only owners charged for this request may inherit its result. -/
  owners : Array Nat

structure Cache where
  private entries : Std.HashMap (Name × Lean.Expr) (Array Entry) := {}
  deriving Inhabited

inductive Lookup where
  | missing
  | found (proof? : Option Lean.Expr)

/-- Reuse stays within the group charged for the original work. Unowned
generation and authored work have a separate scope from automatic owners. -/
def paidBy (paying consuming : Array Nat) : Bool :=
  paying.isEmpty == consuming.isEmpty && consuming.all paying.contains

/-- Proof dependencies include the proof term, not just its statement.
Changed definitions, replaced hypotheses and new local instances invalidate
reuse. Failed requests additionally depend on the whole context and options. -/
def lookup (cache : Cache) (statement : Lean.Expr) (stage : Name := .anonymous)
    (owners : Array Nat := #[]) : MetaM Lookup := do
  let statement ← instantiateMVars statement
  if statement.hasMVar || statement.hasLooseBVars then return .missing
  let currentInstances ← instances
  for entry in cache.entries.getD (stage, statement) #[] do
    unless paidBy entry.owners owners do continue
    if entry.instances != currentInstances then continue
    if entry.proof?.isNone then
      if entry.options == (← getOptions) && (← revision?) == some entry.declarations then
        return .found none
    else
      let unchanged ← entry.declarations.allM fun previous => do
        let some current := (← getLCtx).find? previous.id | return false
        return (← declaration? current) == some previous
      if unchanged then return .found entry.proof?
  return .missing

/-- Retain closed proofs or an exact no-progress revision. Unresolved
metavariables and reporting admissions never enter the reusable cache. -/
def remember (cache : Cache) (statement : Lean.Expr) (proof? : Option Lean.Expr)
    (stage : Name := .anonymous) (owners : Array Nat := #[]) : MetaM Cache := do
  let statement ← instantiateMVars statement
  if statement.hasMVar || statement.hasLooseBVars then return cache
  let proof? ← proof?.mapM instantiateMVars
  if proof?.any (fun proof => proof.hasMVar || proof.hasLooseBVars || proof.hasSorry) then return cache
  if let some proof := proof? then
    unless ← isDefEq (← inferType proof) statement do
      throwError "a preparation result does not prove its request"
  let localInstances ← instances
  let declarations? ← match proof? with
    | some proof => dependencies? (#[statement, proof] ++ localInstances)
    | none => revision?
  let some declarations := declarations? | return cache
  let entry : Entry := { proof?, declarations, instances := localInstances, options := ← getOptions, owners }
  return { cache with
    entries := cache.entries.insert (stage, statement) ((cache.entries.getD (stage, statement) #[]).push entry) }

/-- Run a request once for its valid inputs. Exceptions propagate without
recording a no-progress result; the caller owns transaction rollback and
the automatic allowance. The cache itself must share that transaction. -/
def run {m : Type → Type} [Monad m] [MonadLiftT MetaM m] (cache : IO.Ref Cache)
    (statement : Lean.Expr) (action : m (Option Lean.Expr)) (stage : Name := .anonymous)
    (owners : Array Nat := #[]) :
    m (Option Lean.Expr) := do
  match ← liftM (do lookup (← cache.get) statement stage owners : MetaM Lookup) with
  | .found proof? => return proof?
  | .missing =>
      let proof? ← action
      liftM (show MetaM Unit from do cache.set (← remember (← cache.get) statement proof? stage owners))
      return proof?

end VerexIR.Proofs.Denote.PreparationCache
