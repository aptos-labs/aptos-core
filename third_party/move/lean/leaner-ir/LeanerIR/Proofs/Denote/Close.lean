-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Agreement
import LeanerIR.Proofs.Order
import LeanerIR.Proofs.Denote.SimpAll
import LeanerIR.Proofs.Maps
import LeanerIR.Proofs.Denote.BitLift

/-!
# Closing the verification condition of a denotation

After the `lir_denote` normalization a goal is a tree of conjunctions,
implications, and conditionals whose leaves are arithmetic over the
native values and their certificates, or decidable propositions.  The
closer splits the tree structurally and decides each leaf: certificates
become bounds, then `omega`; finite leaves go to `decide`.  A leaf that
no decision procedure closes is reported at the authored clause its
`Obligation` marker names.  Nothing here selects a route or matches a
goal shape.
-/

namespace LeanerIR.Proofs.Denote

open Lean Elab Tactic Meta

/-- Show the residual goal of a clause the closer cannot establish. -/
register_option leaner.certifyDebug : Bool := {
  defValue := false
  descr := "show the residual goal of a specification clause the closer cannot establish"
}

/-- Marks a function's arguments and state at its start, what the `old` of
its loop invariants reads: the verified function's, and each inlined
callee's at its call. A hypothesis rather than an equation, so that
substitution rewrites it instead of eliminating it. -/
def FunctionStart {α : Type} (_function : LeanerIR.FunctionHandle) (_arguments : α)
    (_state : LeanerIR.RuntimeState) : Prop := True

theorem FunctionStart.intro {α : Type} (function : LeanerIR.FunctionHandle) (arguments : α)
    (state : LeanerIR.RuntimeState) : FunctionStart function arguments state := trivial

/-- The range an `Obligation` marker names, when the expression is one. -/
def obligationRange? (e : Lean.Expr) : Option ObligationRange := do
  guard <| e.isAppOfArity ``Obligation 4
  let .lit (.strVal file) := (e.getArg! 0).consumeMData | none
  let startByte ← (e.getArg! 1).nat? <|> (e.getArg! 1).rawNatLit?
  let endByte ← (e.getArg! 2).nat? <|> (e.getArg! 2).rawNatLit?
  pure { file, startByte, endByte }

/-- The ranges of the `Obligation` markers in an expression. -/
partial def obligationRanges (expression : Lean.Expr) : Array ObligationRange :=
  let rec walk (e : Lean.Expr) (found : Array ObligationRange) : Array ObligationRange :=
    let found := match obligationRange? e with
      | some range => found.push range
      | none => found
    match e with
    | .app function argument => walk argument (walk function found)
    | .lam _ type body _ => walk body (walk type found)
    | .forallE _ type body _ => walk body (walk type found)
    | .letE _ type value body _ => walk body (walk value (walk type found))
    | .mdata _ body => walk body found
    | .proj _ _ body => walk body found
    | _ => found
  walk expression #[]

/-- Where a goal the closer splits off comes from, when no clause marker
locates it. -/
inductive Provenance where
  | precondition (callee : String)
  | continuation (callee : String)
  | loopEntry
  | loopIteration

def Provenance.describe : Provenance → String
  | .precondition callee => s!"the precondition of `{callee}`"
  | .continuation callee => s!"the continuation after `{callee}`"
  | .loopEntry => "a loop invariant at entry"
  | .loopIteration => "a loop invariant at an iteration"

/-- What an obligation located at an authored clause says: a loop's
invariant is owed at entry and after each iteration, any other clause by the
function. -/
private def clauseFailure (origin : Option Provenance) (snippet : String) : MessageData :=
  match origin with
  | some .loopEntry => m!"the loop invariant `{snippet}` is not established at entry"
  | some .loopIteration => m!"the loop invariant `{snippet}` is not preserved by an iteration"
  | _ => m!"the specification clause `{snippet}` is not established"

/-- Report a verification condition that was not established, at every
authored clause its `Obligation` markers — those in it and the `clauses` its
closing stripped — locate not yet reported with the same message, or, with
no marker, as `origin` or a residual obligation with its goal. Returns the
reports so far. -/
def reportObligation (goal : MVarId) (origin : Option Provenance)
    (clauses : Array ObligationRange) (reported : Array (ObligationRange × String)) :
    TacticM (Array (ObligationRange × String)) := do
  let fileMap ← getFileMap
  let fileName ← getFileName
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let ranges := (clauses ++ obligationRanges target).toList.eraseDups.filter
      fun range => range.startByte < range.endByte
    if ranges.isEmpty then
      match origin with
      | some origin => logError m!"{origin.describe} is not established"
      | none => logError m!"verification failed with a residual obligation:\n{(← Lean.Meta.ppGoal goal)}"
    let mut reported := reported
    for range in ranges do
      -- A clause of another module's file is reported in that file.
      let source? ← if range.file == fileName then pure (some fileMap.source) else
        try some <$> IO.FS.readFile range.file catch _ => pure none
      let snippet := match source? with
        | some source => Substring.Raw.toString ⟨source, ⟨range.startByte⟩, ⟨range.endByte⟩⟩
        | none => s!"{range.file}:{range.startByte}"
      let message := clauseFailure origin snippet
      let key := (range, ← message.toString)
      if reported.contains key then continue
      reported := reported.push key
      match source? with
      | some _ =>
          if range.file == fileName then
            logErrorAt (Syntax.atom (.synthetic ⟨range.startByte⟩ ⟨range.endByte⟩) snippet) message
          else
            let foreign := FileMap.ofString (source?.getD "")
            logMessage {
              fileName := range.file
              pos := foreign.toPosition ⟨range.startByte⟩
              endPos := some (foreign.toPosition ⟨range.endByte⟩)
              severity := .error
              data := ← addMessageContext message }
      | none => logError message
    if leaner.certifyDebug.get (← getOptions) && !(ranges.isEmpty && origin.isNone) then
      logError m!"residual obligation:\n{(← Lean.Meta.ppGoal goal)}"
    return reported

register_option leaner.denoteDebug : Bool := {
  defValue := false
  descr := "show the normalized verification condition of a denotation before closing"
}

/-- Run a step and, under `leaner.denoteDebug`, report its heartbeats. -/
elab "leaner_denote_timed " label:str step:tactic : tactic => do
  let debug := leaner.denoteDebug.get (← getOptions)
  let start ← IO.getNumHeartbeats
  tryCatchRuntimeEx
    (do
      evalTactic step
      if debug then
        IO.println s!"  {label.getString}: {((← IO.getNumHeartbeats) - start) / 1000}k, \
          {(← getGoals).length} goals")
    fun failure => do
      if debug then
        IO.println s!"  {label.getString}: {((← IO.getNumHeartbeats) - start) / 1000}k, failed\
          {if failure.isRuntime then " (runtime)" else ""}: \
          {(← failure.toMessageData.toString).take 200}"
      throw failure

/-- The unsigned certified integer a `.val` projection reads, with its width. -/
private def unsignedValue? (e : Lean.Expr) : Option (Lean.Expr × Lean.Expr) :=
  if e.isAppOfArity ``LeanerIR.SpecInt.val 3 && (e.getArg! 1).isConstOf ``Bool.false &&
      (e.getArg! 0).isAppOfArity ``LeanerIR.IntWidth.bits 1 && !e.hasLooseBVars then
    some ((e.getArg! 0).getArg! 0, e.getArg! 2)
  else none
/-- The signed certified integer a `.val` projection reads, with its width. -/
private def signedValue? (e : Lean.Expr) : Option (Lean.Expr × Lean.Expr) :=
  if e.isAppOfArity ``LeanerIR.SpecInt.val 3 && (e.getArg! 1).isConstOf ``Bool.true &&
      (e.getArg! 0).isAppOfArity ``LeanerIR.IntWidth.bits 1 && !e.hasLooseBVars then
    some ((e.getArg! 0).getArg! 0, e.getArg! 2)
  else none

/-- A proof that an integer term is nonnegative, from its structure: a
certified unsigned value, a nonnegative literal, and sums, products,
shifts, conjunctions, and remainders of nonnegative terms. -/
private partial def nonnegative? (e : Lean.Expr) : MetaM (Option Lean.Expr) := do
  if let some (_, value) := unsignedValue? e then
    return some (← mkAppM ``And.left #[← mkAppM ``LeanerIR.SpecInt.unsigned_bounds #[value]])
  if let some literal := e.int? <|> (e.nat?.map Int.ofNat) then
    if 0 ≤ literal then
      return some (← mkDecideProof (← mkAppM ``LE.le #[mkIntLit 0, e]))
    return none
  let both (lemma : Name) : MetaM (Option Lean.Expr) := do
    match ← nonnegative? (e.getArg! 4), ← nonnegative? (e.getArg! 5) with
    | some left, some right => some <$> mkAppM lemma #[left, right]
    | _, _ => pure none
  if e.isAppOfArity ``HAdd.hAdd 6 then return ← both ``Int.add_nonneg
  if e.isAppOfArity ``HMul.hMul 6 then return ← both ``Int.mul_nonneg
  if e.isAppOfArity ``Int.shiftLeft 2 then
    let some value ← nonnegative? (e.getArg! 0) | return none
    return some (← mkAppM ``LeanerIR.Proofs.Denote.shiftLeft_nonneg #[e.getArg! 0, e.getArg! 1, value])
  if e.isAppOfArity ``LeanerIR.Proofs.IntegerArithmetic.bitwiseAnd 2 then
    let some left ← nonnegative? (e.getArg! 0) | return none
    let some right ← nonnegative? (e.getArg! 1) | return none
    return some (← mkAppM ``LeanerIR.Proofs.IntegerArithmetic.bitwiseAnd_nonneg #[left, right])
  if e.isAppOfArity ``Int.tmod 2 then
    let some value ← nonnegative? (e.getArg! 0) | return none
    return some (← mkAppM ``Int.tmod_nonneg #[e.getArg! 1, value])
  if e.isAppOfArity ``HMod.hMod 6 then
    let modulus := e.getArg! 5
    let some literal := modulus.int? <|> (modulus.nat?.map Int.ofNat) | return none
    unless literal != 0 do return none
    let nonzero ← mkDecideProof (← mkAppM ``Ne #[modulus, mkIntLit 0])
    return some (← mkAppM ``Int.emod_nonneg #[e.getArg! 4, nonzero])
  return none

/-- Both facts, either, or none. -/
private def conjoin? : Option Lean.Expr → Option Lean.Expr → MetaM (Option Lean.Expr)
  | some left, some right => some <$> mkAppM ``And.intro #[left, right]
  | some fact, none | none, some fact => pure (some fact)
  | none, none => pure none

/-- The bound of one bit operation, when its operands are certified. -/
private def operationFact? (sub : Lean.Expr) : MetaM (Option Lean.Expr) := do
  if sub.hasLooseBVars then return none
  -- A map's size is a count, and a key's position lies within it.
  if sub.isAppOfArity ``LeanerIR.Maps.size 1 then
    let map := sub.getArg! 0
    if map.isAppOfArity ``LeanerIR.RuntimeValue.nominal 3 then
      if let some #[field] := (map.getArg! 2).arrayLit?.map (·.2.toArray) then
        if field.isAppOfArity ``LeanerIR.RuntimeValue.vector 1 then
          return some (← mkAppM ``LeanerIR.Maps.size_nominal_vector_bounds
            #[map.getArg! 0, map.getArg! 1, field.getArg! 0])
    return some (← mkAppM ``LeanerIR.Maps.size_nonneg #[map])
  if sub.isAppOfArity ``LeanerIR.Maps.rank 2 then
    return some (← mkAppM ``LeanerIR.Maps.rank_bounds #[sub.getArg! 0, sub.getArg! 1])
  if sub.isAppOfArity ``LeanerIR.Proofs.IntegerArithmetic.bitwiseAnd 2 then
    match unsignedValue? (sub.getArg! 0), unsignedValue? (sub.getArg! 1) with
    | some (_, left), some (_, right) =>
        return some (← mkAppM ``LeanerIR.Proofs.Denote.BitOp.eval_bounds
          #[mkConst ``LeanerIR.Proofs.Denote.BitOp.and, left, right])
    | _, _ =>
        match ← nonnegative? (sub.getArg! 0), ← nonnegative? (sub.getArg! 1) with
        | some left, some right =>
            return some (← mkAppM ``LeanerIR.Proofs.IntegerArithmetic.bitwiseAnd_nonneg
              #[left, right])
        | _, _ => return none
  -- A shift by a literal distance is a multiplication or division by a
  -- literal, which `omega` reads.
  let literalDistance := (sub.getArg! 1).nat?.isSome || (sub.getArg! 1).rawNatLit?.isSome
  if sub.isAppOfArity ``Int.shiftRight 2 then
    let division ← if literalDistance then
        some <$> mkAppM ``LeanerIR.Proofs.Denote.shiftRight_eq_div #[sub.getArg! 0, sub.getArg! 1]
      else pure none
    let bounds ← match unsignedValue? (sub.getArg! 0) with
      | some (_, value) =>
          some <$> mkAppM ``LeanerIR.Proofs.Denote.shiftRight_bounds #[value, sub.getArg! 1]
      | none => pure none
    return ← conjoin? bounds division
  if sub.isAppOfArity ``HMul.hMul 6 && (sub.getArg! 0).isConstOf ``Int then
    match unsignedValue? (sub.getArg! 4), unsignedValue? (sub.getArg! 5) with
    | some (_, left), some (_, right) =>
        return some (← mkAppM ``LeanerIR.Proofs.Denote.mul_unsigned_bounds #[left, right])
    | _, _ => return none
  if sub.isAppOfArity ``Int.tdiv 2 || sub.isAppOfArity ``Int.tmod 2 then
    match ← nonnegative? (sub.getArg! 0), ← nonnegative? (sub.getArg! 1) with
    | some leftNonnegative, some rightNonnegative =>
        let lemma := if sub.isAppOfArity ``Int.tdiv 2
          then ``LeanerIR.Proofs.Denote.tdiv_bounds_of_nonneg
          else ``LeanerIR.Proofs.Denote.tmod_bounds_of_nonneg
        return some (← mkAppM lemma #[leftNonnegative, rightNonnegative])
    | _, _ =>
        match signedValue? (sub.getArg! 0), signedValue? (sub.getArg! 1) with
        | some (width, left), some (_, right) =>
            let lemma := if sub.isAppOfArity ``Int.tdiv 2
              then ``LeanerIR.Proofs.Denote.tdiv_signed_range
              else ``LeanerIR.Proofs.Denote.tmod_signed_range
            return some (mkApp3 (mkConst lemma) width left right)
        | _, _ =>
            let lemma := if sub.isAppOfArity ``Int.tdiv 2
              then ``LeanerIR.Proofs.Denote.tdiv_facts
              else ``LeanerIR.Proofs.Denote.tmod_facts
            return some (mkApp2 (mkConst lemma) (sub.getArg! 0) (sub.getArg! 1))
  if sub.isAppOfArity ``Int.shiftLeft 2 then
    let product ← if literalDistance then
        some <$> mkAppM ``LeanerIR.Proofs.Denote.shiftLeft_eq_mul #[sub.getArg! 0, sub.getArg! 1]
      else pure none
    let nonnegative ← nonnegative? sub
    return ← conjoin? nonnegative product
  return none

/-- The distinct subterms of some terms a predicate accepts, in the order
first met, each shared subterm visited once: a term built by repeated updates
shares its parts, and is exponentially larger read as a tree. -/
private partial def sitesWhere (accept : Lean.Expr → Bool) (terms : Array Lean.Expr) :
    Array Lean.Expr :=
  (terms.forM go |>.run ({}, #[])).2.2
where
  go (e : Lean.Expr) : StateM (Std.HashSet Lean.Expr × Array Lean.Expr) Unit := do
    if (← get).1.contains e then return
    modify fun (visited, found) => (visited.insert e, if accept e then found.push e else found)
    match e with
    | .app f a => go f; go a
    | .lam _ t b _ | .forallE _ t b _ => go t; go b
    | .letE _ t v b _ => go t; go v; go b
    | .mdata _ b | .proj _ _ b => go b
    | _ => pure ()

/-- The operations propositions apply whose results have bounds. -/
private def operationSites (terms : Array Lean.Expr) : Array Lean.Expr :=
  sitesWhere (fun e => e.isAppOfArity ``LeanerIR.Proofs.IntegerArithmetic.bitwiseAnd 2 ||
    e.isAppOfArity ``Int.shiftRight 2 || e.isAppOfArity ``Int.shiftLeft 2 ||
    e.isAppOfArity ``Int.tdiv 2 || e.isAppOfArity ``Int.tmod 2 ||
    e.isAppOfArity ``HMul.hMul 6 || e.isAppOfArity ``LeanerIR.Maps.size 1 ||
    e.isAppOfArity ``LeanerIR.Maps.rank 2) terms

/-- The closed applications of structure projections propositions mention. -/
private def projectionSites (env : Environment) (terms : Array Lean.Expr) : Array Lean.Expr :=
  sitesWhere (fun e => match e.getAppFn with
    | .const name _ => e.isApp && !e.hasLooseBVars && (env.getProjectionFnInfo? name).isSome
    | _ => false) terms

/-- The `Skolems` family an encoder encodes at: the instance a lemma about
the encoding must be applied at, never one synthesized from the context,
which is the runtime family while a trusted generic callee's values live at
its instantiated one. -/
def codecInstance? (encoder : Lean.Expr) : Option Lean.Expr := do
  if encoder.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.encode 2 then return encoder.getArg! 0
  if encoder.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.encode 3 then return encoder.getArg! 0
  guard (encoder.isAppOfArity ``LeanerIR.Proofs.Codec.encode 3)
  let codec := encoder.getArg! 2
  guard (codec.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.codec 2)
  return codec.getArg! 0

/-- The native type an encoding function encodes, from its spelling: the
typed `NTy.encode τ`, or the codec of a type. -/
partial def codecType? (encoder : Lean.Expr) : Option Lean.Expr := do
  if encoder.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.encode 2 then return encoder.getArg! 1
  guard (encoder.isAppOfArity ``LeanerIR.Proofs.Codec.encode 3)
  codecOf (encoder.getArg! 2)
where
  codecOf (codec : Lean.Expr) : Option Lean.Expr := do
    let scalar (name : Name) (τ : Name) : Option Lean.Expr :=
      if codec.isConstOf name then some (mkConst τ) else none
    if codec.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.codec 2 then return codec.getArg! 1
    if codec.isAppOfArity ``LeanerIR.Proofs.Codec.specInt 2 then
      let width := codec.getArg! 0
      guard (width.isAppOfArity ``LeanerIR.IntWidth.bits 1)
      return mkAppN (mkConst ``LeanerIR.Proofs.Denote.NTy.int) #[width.getArg! 0, codec.getArg! 1]
    if codec.isAppOfArity ``LeanerIR.Proofs.Codec.boundedVector 2 then
      return mkApp (mkConst ``LeanerIR.Proofs.Denote.NTy.vector) (← codecOf (codec.getArg! 1))
    if codec.isAppOfArity ``LeanerIR.Proofs.Codec.tuple 2 then
      let row := codec.getArg! 1
      guard (row.isAppOfArity ``LeanerIR.Proofs.Denote.rowCodec 2)
      return mkApp (mkConst ``LeanerIR.Proofs.Denote.NTy.tuple) (row.getArg! 1)
    if codec.isAppOfArity ``LeanerIR.Proofs.Denote.Codec.nominalRow 4 then
      guard ((codec.getArg! 2).isAppOfArity ``Option.none 1)
      let row := codec.getArg! 3
      guard (row.isAppOfArity ``LeanerIR.Proofs.Denote.rowCodec 2)
      return mkAppN (mkConst ``LeanerIR.Proofs.Denote.NTy.struct) #[codec.getArg! 1, row.getArg! 1]
    scalar ``LeanerIR.Proofs.Codec.bool ``LeanerIR.Proofs.Denote.NTy.bool <|>
      scalar ``LeanerIR.Proofs.Codec.address ``LeanerIR.Proofs.Denote.NTy.address <|>
      scalar ``LeanerIR.Proofs.Codec.signer ``LeanerIR.Proofs.Denote.NTy.signer <|>
      scalar ``LeanerIR.Proofs.Codec.string ``LeanerIR.Proofs.Denote.NTy.string <|>
      scalar ``LeanerIR.Proofs.Codec.bytes ``LeanerIR.Proofs.Denote.NTy.bytes <|>
      scalar ``LeanerIR.Proofs.Codec.unit ``LeanerIR.Proofs.Denote.NTy.unit

/-- Assert the facts a leaf needs beside its hypotheses: the bounds of every
certified integer in context, and the bounds of every bit operation on
them that the goal or a hypothesis mentions. A fact whose statement is in
`skip` is not asserted. The statements asserted are returned. -/
def assertBounds (skip : Array Lean.Expr := #[]) : TacticM (Array Lean.Expr) := do
  if (← getGoals).isEmpty then return #[]
  let goal ← getMainGoal
  let facts ← goal.withContext do
    let mut facts : Array Lean.Expr := #[]
    let mut expressions := #[← instantiateMVars (← goal.getType)]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← whnfR (← instantiateMVars decl.type)
      expressions := expressions.push (← instantiateMVars decl.type)
      if ty.isAppOfArity ``LeanerIR.SpecVector 1 then
        facts := facts.push (← mkAppM ``LeanerIR.SpecVector.bounded #[decl.toExpr])
      if ty.isAppOfArity ``LeanerIR.SpecInt 2 then
        let width := ty.getArg! 0
        if width.isAppOfArity ``LeanerIR.IntWidth.bits 1 then
          let lemma := if (ty.getArg! 1).isConstOf ``Bool.true
            then ``LeanerIR.SpecInt.signed_bounds else ``LeanerIR.SpecInt.unsigned_bounds
          facts := facts.push (mkApp2 (mkConst lemma) (width.getArg! 0) decl.toExpr)
    for site in operationSites expressions do
      if let some fact ← operationFact? site then facts := facts.push fact
    -- A proof that does not assemble is not a fact.
    let attempt (fact : MetaM Lean.Expr) : MetaM (Option Lean.Expr) :=
      try pure (some (← fact)) catch _ => pure none
    -- The family the elements are encoded at, when the encoder names one; a
    -- scalar codec's does not, and any family serves it.
    let mapped? (e : Lean.Expr) : Option (Option Lean.Expr × Lean.Expr) :=
      if e.isAppOfArity ``Array.map 4 then do
        pure (codecInstance? (e.getArg! 2), ← codecType? (e.getArg! 2))
      else none
    -- Range certificates and their negations in context yield bounds as
    -- separate facts; rewriting them would leave casts in the terms that
    -- depend on them.
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      -- An encoding equated to a runtime value names the native value it
      -- decodes to, and one distinguished from it excludes that value.
      let equation := if ty.isAppOfArity ``LeanerIR.Proofs.Obligation 4 then ty.getArg! 3 else ty
      if equation.isAppOfArity ``Not 1 && (equation.getArg! 0).isAppOfArity ``Eq 3 then
        let inner := equation.getArg! 0
        let lhs := inner.getArg! 1
        let rhs := inner.getArg! 2
        -- Distinct certified values differ in their fields.
        let carrier ← whnfR (inner.getArg! 0)
        if carrier.isAppOfArity ``LeanerIR.SpecInt 2 then
          facts := facts.push (← mkAppM ``LeanerIR.Proofs.Denote.SpecInt.val_ne_of_ne #[decl.toExpr])
        else if carrier.isAppOfArity ``LeanerIR.SpecVector 1 then
          facts := facts.push
            (← mkAppM ``LeanerIR.Proofs.Denote.SpecVector.values_ne_of_ne #[decl.toExpr])
        let fact ← if lhs.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.encode 3 then
            attempt (mkAppOptM ``LeanerIR.Proofs.Denote.NTy.decode?_ne_of_encode_ne
              #[lhs.getArg! 0, none, none, none, decl.toExpr])
          else if let some (inst, τ) := mapped? lhs then
            attempt (mkAppOptM ``LeanerIR.Proofs.Denote.NTy.decode?_vector_ne_of_map_ne
              #[inst, τ, none, none, decl.toExpr])
          else pure none
        if let some fact := fact then facts := facts.push fact
      if equation.isAppOfArity ``Eq 3 then
        let proof ← if ty.isAppOfArity ``LeanerIR.Proofs.Obligation 4 then
            mkAppM ``Iff.mp #[← mkAppOptM ``LeanerIR.Proofs.Obligation_iff
              #[ty.getArg! 0, ty.getArg! 1, ty.getArg! 2, equation], decl.toExpr]
          else pure decl.toExpr
        let lhs := equation.getArg! 1
        let rhs := equation.getArg! 2
        let isRuntime (e : Lean.Expr) := match e.getAppFn with
          | .const name _ => name.getPrefix == ``LeanerIR.RuntimeValue
          | _ => false
        -- A map observation is opaque, so its scalar reading needs the name
        -- the equation gives it.
        let observesMap (e : Lean.Expr) := match e.getAppFn with
          | .const name _ => name.getPrefix == `LeanerIR.Maps
          | _ => false
        let scalarReading? (scalar observation : Lean.Expr) : Option Name :=
          if !observesMap observation then none
          else if scalar.isAppOfArity ``LeanerIR.RuntimeValue.integer 1 then
            some ``LeanerIR.Proofs.Denote.asInt_of_integer_eq
          else if scalar.isAppOfArity ``LeanerIR.RuntimeValue.bool 1 then
            some ``LeanerIR.Proofs.Denote.asBool_of_bool_eq
          else none
        let decodeFact ← if lhs.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.encode 3 && isRuntime rhs then
            attempt (mkAppOptM ``LeanerIR.Proofs.Denote.NTy.decode?_of_encode
              #[lhs.getArg! 0, none, none, none, proof])
          else if rhs.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.encode 3 && isRuntime lhs then
            attempt (mkAppOptM ``LeanerIR.Proofs.Denote.NTy.decode?_of_encode
              #[rhs.getArg! 0, none, none, none, ← mkEqSymm proof])
          else if let some (inst, τ) := mapped? lhs then
            attempt (mkAppOptM ``LeanerIR.Proofs.Denote.NTy.decode?_vector_of_map
              #[inst, τ, none, none, proof])
          else if let some (inst, τ) := mapped? rhs then
            attempt (mkAppOptM ``LeanerIR.Proofs.Denote.NTy.decode?_vector_of_map
              #[inst, τ, none, none, ← mkEqSymm proof])
          -- A scalar equated to a map's observation, such as its value at a
          -- key, is what the observation reads as.
          else if let some lemma := scalarReading? lhs rhs then
            attempt (mkAppM lemma #[proof])
          else if let some lemma := scalarReading? rhs lhs then
            attempt (mkAppM lemma #[← mkEqSymm proof])
          else pure none
        if let some fact := decodeFact then facts := facts.push fact
        -- An encoded enum value equated to a variant holds that variant.
        let variantOf (encoded : Lean.Expr) (proof : Lean.Expr) : MetaM (Option Lean.Expr) :=
          attempt (mkAppOptM ``LeanerIR.Proofs.Denote.NTy.variantName_of_encode_enum
            #[encoded.getArg! 0, none, none, none, none, none, none, none, none, proof])
        -- One equated to a conditional value holds the variant it names.
        let variantOfConditional (encoded : Lean.Expr) (proof : Lean.Expr) :
            MetaM (Option Lean.Expr) :=
          attempt (mkAppOptM ``LeanerIR.Proofs.Denote.NTy.variantName_of_encode_eq
            #[encoded.getArg! 0, none, none, none, none, none, none, proof])
        let variantFact ← if lhs.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.encode 3 &&
              rhs.isAppOfArity ``LeanerIR.RuntimeValue.nominal 3 then
            variantOf lhs proof
          else if rhs.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.encode 3 &&
              lhs.isAppOfArity ``LeanerIR.RuntimeValue.nominal 3 then
            variantOf rhs (← mkEqSymm proof)
          else if lhs.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.encode 3 &&
              rhs.isAppOfArity ``ite 5 then
            variantOfConditional lhs proof
          else if rhs.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.encode 3 &&
              lhs.isAppOfArity ``ite 5 then
            variantOfConditional rhs (← mkEqSymm proof)
          else pure none
        if let some fact := variantFact then facts := facts.push fact
      let (negated, fits) := if ty.isAppOfArity ``Not 1 then (true, ty.getArg! 0) else (false, ty)
      unless fits.isAppOfArity ``LeanerIR.IntegerValueFits 3 do continue
      let width := fits.getArg! 0
      unless width.isAppOfArity ``LeanerIR.IntWidth.bits 1 do continue
      let signed := (fits.getArg! 1).isConstOf ``Bool.true
      if negated then
        let some bits := (width.getArg! 0).nat? | continue
        if bits == 0 then continue
        let nonzero ← mkDecideProof (← mkAppM ``Ne #[width.getArg! 0, mkNatLit 0])
        let lemma := if signed then ``LeanerIR.Proofs.Denote.bounds_of_not_fits_signed
          else ``LeanerIR.Proofs.Denote.bounds_of_not_fits_unsigned
        facts := facts.push (← mkAppM lemma #[nonzero, decl.toExpr])
      else
        let lemma := if signed then ``LeanerIR.Proofs.Denote.bounds_of_fits_signed
          else ``LeanerIR.Proofs.Denote.bounds_of_fits_unsigned
        facts := facts.push (← mkAppM lemma #[decl.toExpr])
    -- Certified integers read through a structure projection, such as a
    -- twin's field, carry their bounds too.
    for site in projectionSites (← getEnv) expressions do
      let ty ← whnfR (← inferType site)
      if ty.isAppOfArity ``LeanerIR.SpecInt 2 then
        let width := ty.getArg! 0
        if width.isAppOfArity ``LeanerIR.IntWidth.bits 1 then
          let lemma := if (ty.getArg! 1).isConstOf ``Bool.true
            then ``LeanerIR.SpecInt.signed_bounds else ``LeanerIR.SpecInt.unsigned_bounds
          facts := facts.push (mkApp2 (mkConst lemma) (width.getArg! 0) site)
    pure facts
  -- A fact already in context is not added again.
  let mut goal := goal
  let mut known ← goal.withContext do
    (← getLCtx).foldlM (init := skip) fun known decl => do
      if decl.isImplementationDetail then pure known
      else pure (known.push (← instantiateMVars decl.type))
  let mut added := #[]
  for proof in facts do
    let type ← goal.withContext (instantiateMVars (← inferType proof))
    if known.contains type then continue
    known := known.push type
    added := added.push type
    let asserted ← goal.assert `bounds type proof
    let (_, next) ← asserted.intro1P
    goal := next
    -- The projections of literal values in the fact reduce, so that a
    -- decision procedure reads the value, not the projection.
    setGoals [goal]
    let name := mkIdent `bounds
    evalTactic (← `(tactic| try dsimp only at $name:ident))
    goal ← getMainGoal
  replaceMainGoal [goal]
  return added

/-- Assert the bounds a leaf needs beside its hypotheses (`assertBounds`). -/
elab "leaner_denote_bounds" : tactic => discard assertBounds

/-- A structure projection applied to a constructor application, reduced. -/
private def reduceProjection? (e : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let .const name _ := e.getAppFn | return none
  let some info ← getProjectionFnInfo? name | return none
  let arguments := e.getAppArgs
  let some value := arguments[info.numParams]? | return none
  let .const constructor _ := value.getAppFn | return none
  unless constructor == info.ctorName do return none
  let some field := value.getAppArgs[info.numParams + info.i]? | return none
  return some (mkAppN field (arguments.extract (info.numParams + 1) arguments.size))

/-- Reduce every projection of a constructor application, in the goal and in
every hypothesis. A substituted value leaves `{ val := e, … }.val` behind,
which arithmetic cannot see through; the reduction is a definitional change,
so it reaches the hypotheses other terms depend on, which `simp at *`
leaves alone. -/
elab "leaner_denote_reduce_projections" : tactic => do
  let mut goal ← getMainGoal
  let reduce (e : Lean.Expr) : MetaM Lean.Expr :=
    Meta.transform e (post := fun e => do
      match ← reduceProjection? e with
      | some reduced => return .visit reduced
      | none => return .continue)
  for decl in ← goal.withContext getLCtx do
    if decl.isImplementationDetail || decl.isLet then continue
    let type ← goal.withContext (instantiateMVars decl.type)
    let reduced ← goal.withContext (reduce type)
    if reduced != type then
      goal ← goal.replaceLocalDeclDefEq decl.fvarId reduced
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  let reduced ← goal.withContext (reduce target)
  if reduced != target then goal ← goal.replaceTargetDefEq reduced
  replaceMainGoal [goal]

/-- Clear every hypothesis that speaks about computations rather than
values: the loop hypotheses and recursive iterations a leaf never needs. -/
elab "leaner_denote_clear_computations" : tactic => do
  if (← getGoals).isEmpty then return
  let goal ← getMainGoal
  let victims ← goal.withContext do
    let mut found := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      let constants := ty.getUsedConstants
      if constants.contains ``LeanerIR.Proofs.wp || constants.contains ``LeanerIR.Proofs.Spec ||
          constants.contains ``LeanerIR.Validation.prepareExecution ||
          constants.contains ``LeanerIR.Validation.ExecutableUnit ||
          constants.contains ``LeanerIR.Validation.SemanticsRegistry then
        found := found.push decl.fvarId
    pure found
  let mut goal := goal
  for fvarId in victims.reverse do
    goal ← goal.tryClear fvarId
  replaceMainGoal [goal]

/-- The heartbeats a leaf's attempt at an arbitrary instance may spend. The
attempt is extra work on a leaf its deciders already failed, so it must
neither exhaust the target's budget nor turn the leaf's report into a
timeout; a decided instance costs a few million. -/
def instanceAttemptHeartbeats : Nat := 20000000

/-- Run a tactic within `instanceAttemptHeartbeats` (and within what is left
of the enclosing budget); exhausting it is an ordinary failure, which the
enclosing `first` recovers from. -/
elab "leaner_denote_budgeted " step:tactic : tactic => do
  let context ← readThe Core.Context
  let now ← IO.getNumHeartbeats
  let remaining := if context.maxHeartbeats == 0 then instanceAttemptHeartbeats
    else context.initHeartbeats + context.maxHeartbeats - now
  let budget := max 1 (min remaining instanceAttemptHeartbeats)
  tryCatchRuntimeEx
    (withTheReader Core.Context
      (fun context => { context with initHeartbeats := now, maxHeartbeats := budget })
      (evalTactic step))
    fun failure => do
      if failure.isMaxHeartbeat then throwError "the attempt exceeded its heartbeat budget"
      throw failure

/-- Introduce the binders and premises a goal states, through the
`Obligation` markers heading them; whether any was introduced. -/
private def introduceThroughMarkers (goal : MVarId) : MetaM (MVarId × Bool) := do
  let mut goal := goal
  let mut introduced := false
  repeat
    let target ← goal.withContext (instantiateMVars (← goal.getType))
    if target.isAppOfArity ``LeanerIR.Proofs.Obligation 4 then
      goal ← goal.replaceTargetDefEq (target.getArg! 3)
    else if target.isForall then
      let (_, next) ← goal.intro1P
      goal := next
      introduced := true
    else break
  return (goal, introduced)

/-- Introduce the binders and premises a leaf's goal states, through the
`Obligation` markers heading them: a quantified clause is decided at an
arbitrary instance, whose premises the deciders then use as hypotheses.
Fails when there is nothing to introduce. A leaf takes this only after its
deciders failed on the goal as stated, since `decide` needs a closed goal
and every other leaf keeps its path. -/
elab "leaner_denote_intro" : tactic => do
  if (← getGoals).isEmpty then return
  let (goal, introduced) ← introduceThroughMarkers (← getMainGoal)
  unless introduced do throwError "the goal states no binder or premise"
  replaceMainGoal [goal]

/-- `leaner_denote_intro` where the goal may state nothing to introduce. -/
elab "leaner_denote_intro_any" : tactic => do
  if (← getGoals).isEmpty then return
  let (goal, _) ← introduceThroughMarkers (← getMainGoal)
  replaceMainGoal [goal]

/-- Introduce the binders and premises of the goal and, where it is a
negation, the negated statement as a hypothesis toward `False`: the
deciders that refute a context by instantiation take the goal this way. -/
elab "leaner_denote_intro_negation" : tactic => do
  if (← getGoals).isEmpty then return
  let (goal, _) ← introduceThroughMarkers (← getMainGoal)
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  if target.isAppOfArity ``Not 1 then
    let (_, next) ← (← goal.replaceTargetDefEq
      (.forallE `negated (target.getArg! 0) (mkConst ``False) .default)).intro1P
    replaceMainGoal [next]
  else replaceMainGoal [goal]

/-- Consume the hypotheses a call leaves behind before a computation goal
is normalized again: an implication whose premise is decided by omega is
specialized, existentials and conjunctions are destructured, and the
equations on state components and on witnesses rewrite the goal. -/
elab "leaner_denote_consume" : tactic => do
  if (← getGoals).isEmpty then return
  let mut goal ← getMainGoal
  -- Specialize decided implications.
  let mut progress := true
  while progress do
    progress := false
    let candidate ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail then return none
        let ty ← instantiateMVars decl.type
        unless ty.isArrow do return none
        let premise := ty.bindingDomain!
        if premise.hasLooseBVars then return none
        unless ← isProp premise do return none
        if premise.getUsedConstants.contains ``LeanerIR.Proofs.wp then return none
        return some (decl.fvarId, premise, ty.bindingBody!)
    if let some (fvarId, premise, conclusion) := candidate then
      let saved ← saveState
      try
        let premiseProof ← goal.withContext do
          let syntactic ← (← getLCtx).findDeclM? fun decl => do
            if decl.isImplementationDetail then return none
            return if (← instantiateMVars decl.type) == premise then some decl.toExpr else none
          match syntactic with
          | some proof => pure proof
          | none =>
              let proof ← mkFreshExprMVar premise
              setGoals [proof.mvarId!]
              evalTactic (← `(tactic| omega))
              pure proof
        let specialized ← goal.withContext do
          instantiateMVars (mkApp (mkFVar fvarId) premiseProof)
        let asserted ← goal.assert `consumed conclusion specialized
        let (_, next) ← asserted.intro1P
        goal ← next.tryClear fvarId
        progress := true
      catch _ =>
        restoreState saved
        -- Leave it; a later leaf may still use it.
        pure ()
      if !progress then break
  -- Destructure existentials and conjunctions.
  progress := true
  while progress do
    progress := false
    let compound ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail then return none
        let ty ← instantiateMVars decl.type
        return if ty.isAppOfArity ``And 2 || ty.isAppOfArity ``Exists 2 then some decl.fvarId else none
    if let some fvarId := compound then
      match ← goal.cases fvarId with
      | #[subgoal] =>
          goal := subgoal.mvarId
          progress := true
      | _ => break
  -- Substitute witness equations: a variable that is not a state.
  progress := true
  while progress do
    progress := false
    let witness ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail then return none
        let ty ← instantiateMVars decl.type
        unless ty.isAppOfArity ``Eq 3 do return none
        let isWitness (e : Lean.Expr) : MetaM Bool := do
          unless e.isFVar do return false
          let type ← whnfR (← inferType e)
          return !type.isConstOf ``LeanerIR.RuntimeState
        if ← isWitness (ty.getArg! 2) then return some decl.fvarId
        if ← isWitness (ty.getArg! 1) then return some decl.fvarId
        return none
    if let some fvarId := witness then
      try
        goal ← Lean.Meta.subst goal fvarId
        progress := true
      catch _ => pure ()
  -- Rewrite with state-component equations.
  let equations ← goal.withContext do
    let mut found := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      if ty.isAppOfArity ``Eq 3 then
        let lhs := ty.getArg! 1
        if lhs.isApp && lhs.appArg!.isFVar && lhs.isAppOfArity ``LeanerIR.RuntimeState.globals 1 then
          found := found.push decl.fvarId
    pure found
  setGoals [goal]
  if !equations.isEmpty then
    let idents ← equations.mapM fun fvarId => do
      let name := (← goal.withContext (fvarId.getDecl)).userName
      `(Lean.Parser.Tactic.simpLemma| $(mkIdent name):ident)
    evalTactic (← `(tactic| try simp only [$idents,*]))

/-- The constructor a term applies, when it is a full application of a
constructor with fields. -/
private def fieldConstructor? (e : Lean.Expr) : MetaM (Option Name) := do
  let .const name _ := e.getAppFn | return none
  match (← getEnv).find? name with
  | some (.ctorInfo info) =>
      return if info.numFields > 0 && e.getAppNumArgs == info.numParams + info.numFields then
        some name else none
  | _ => return none

/-- A loop invariant's elimination over a slot it asserts defined: the
slot and the invariant at its value. -/
private def slotElimination? (ty : Lean.Expr) : MetaM (Option (FVarId × Lean.Expr)) := do
  if ty.isAppOfArity ``Option.elim 5 && (ty.getArg! 3).isConstOf ``False then
    -- The slot is a projection of the row until the row is destructured.
    match ← whnfR (ty.getArg! 2) with
    | .fvar slot => return some (slot, ty.getArg! 4)
    | _ => return none
  else return none

/-- Split every conjunctive or existential hypothesis into its parts,
every row-valued variable into its components, and every slot an
invariant asserts defined into its value, so that a leaf is over
scalars. -/
elab "leaner_denote_split_hypotheses" : tactic => do
  if (← getGoals).isEmpty then return
  let mut goal ← getMainGoal
  -- An existential eliminates into a proposition only; a data goal keeps it.
  let propositional ← goal.withContext do isProp (← goal.getType)
  let mut progress := true
  while progress do
    progress := false
    let conjunction ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail then return none
        let ty ← instantiateMVars decl.type
        if ty.isAppOfArity ``And 2 || (propositional && ty.isAppOfArity ``Exists 2) then
          return some decl.fvarId
        if (← whnfD ty).isAppOfArity ``Prod 2 then return some decl.fvarId
        if (← slotElimination? ty).isSome then return some decl.fvarId
        -- Two constructor applications equated, whatever type the equation
        -- is stated at: a pair splits into its components, a variant into
        -- its payload, and different variants refute the context.
        if ty.isAppOfArity ``Eq 3 then
          if (← fieldConstructor? (ty.getArg! 1)).isSome then
            if (← fieldConstructor? (ty.getArg! 2)).isSome then return some decl.fvarId
        return none
    if let some fvarId := conjunction then
      let isConstructorEquation ← goal.withContext do
        let ty ← instantiateMVars (← fvarId.getType)
        if ty.isAppOfArity ``Eq 3 then return (← fieldConstructor? (ty.getArg! 1)).isSome
        return false
      if isConstructorEquation then
        match ← Lean.Meta.injection goal fvarId with
        | Lean.Meta.InjectionResult.subgoal subgoal _ _ =>
            goal := subgoal
            progress := true
        | Lean.Meta.InjectionResult.solved =>
            replaceMainGoal []
            return
      else if let some (slot, _) ← goal.withContext do
          slotElimination? (← instantiateMVars (← fvarId.getType)) then
        -- A slot a loop invariant asserts defined: the empty case refutes
        -- the context, and the defined case reads the invariant at the
        -- value, by definition.
        let mut defined := none
        for subgoal in ← goal.cases slot do
          let hypothesis := subgoal.subst.get fvarId
          match subgoal.ctorName with
          | some ``Option.none =>
              subgoal.mvarId.withContext do
                subgoal.mvarId.assign (← mkFalseElim (← subgoal.mvarId.getType) hypothesis)
          | some ``Option.some =>
              defined := some (← subgoal.mvarId.withContext do
                let ty ← instantiateMVars (← inferType hypothesis)
                let value := (← whnfR (ty.getArg! 2)).appArg!
                let read := (ty.getArg! 4).beta #[value]
                -- The value takes the name the invariant binds the slot by.
                let named ← subgoal.mvarId.rename value.fvarId! (ty.getArg! 4).bindingName!
                named.replaceLocalDeclDefEq hypothesis.fvarId! read)
          | _ => throwError "a slot has a case besides its definedness"
        let some next := defined | throwError "a slot has no defined case"
        goal := next
        progress := true
      else
      let subgoals ← goal.cases fvarId
      match subgoals with
      | #[subgoal] =>
          goal := subgoal.mvarId
          progress := true
      | _ => break
    else
      -- An array equated through its list is substituted as the array of
      -- that list.
      let listEquation ← goal.withContext do
        (← getLCtx).findDeclM? fun decl => do
          if decl.isImplementationDetail then return none
          let ty ← instantiateMVars decl.type
          unless ty.isAppOfArity ``Eq 3 do return none
          let lhs := ty.getArg! 1
          let rhs := ty.getArg! 2
          let asList (side other : Lean.Expr) (symm : Bool) : Option (Lean.LocalDecl × Bool) :=
            if side.isAppOfArity ``Array.toList 2 then
              match side.getArg! 1 with
              | .fvar xs => if other.containsFVar xs then none else some (decl, symm)
              | _ => none
            else none
          return asList lhs rhs false <|> asList rhs lhs true
      match listEquation with
      | some (decl, symm) =>
          let (equation, asserted) ← goal.withContext do
            let h ← if symm then mkEqSymm decl.toExpr else pure decl.toExpr
            let proof ← mkAppM ``LeanerIR.Proofs.Denote.array_eq_of_toList_eq #[h]
            let asserted ← goal.assert `arrayEq (← inferType proof) proof
            asserted.intro1P
          goal ← asserted.withContext (Lean.Meta.subst asserted equation)
          progress := true
      | none => pure ()
    -- Once nothing else destructures, a twin-typed local splits: a struct
    -- twin into its fields, an enum twin into its variants, so that its
    -- native view and erasure reduce to constructor forms. The tag on the
    -- twin's declaration licenses the split; an enum's variants become
    -- separate goals, each continued by the next round.
    if !progress then
      let twin ← goal.withContext do
        (← getLCtx).findDeclM? fun decl => do
          if decl.isImplementationDetail then return none
          let ty ← instantiateMVars decl.type
          let .const name _ := ty.getAppFn | return none
          return if Proofs.leanerTwinAttribute.hasTag (← getEnv) name then some decl.fvarId
            else none
      if let some fvarId := twin then
        let subgoals ← goal.cases fvarId
        match subgoals with
        | #[subgoal] =>
            goal := subgoal.mvarId
            progress := true
        | _ =>
            replaceMainGoal (subgoals.map (·.mvarId)).toList
            return
  replaceMainGoal [goal]

/-- The proof arguments of an expression that mention a local, each with
the proposition its position expects, when the local occurs nowhere else.
A proof whose proposition itself mentions the local, through the proofs
of an earlier argument, is left for a later pass, once those are
detached. -/
private partial def proofsMentioning (x : FVarId) (e : Lean.Expr) :
    MetaM (Option (Array (Lean.Expr × Lean.Expr))) := do
  let rec visit (e : Lean.Expr) (found : Array (Lean.Expr × Lean.Expr)) :
      MetaM (Option (Array (Lean.Expr × Lean.Expr))) := do
    if !e.containsFVar x then return some found
    match e with
    | .app function argument => do
        let some found ← visit function found | return none
        if argument.containsFVar x && !argument.hasLooseBVars && (← Meta.isProof argument) then
          let .forallE _ proposition _ _ ← whnf (← inferType function) | return none
          let proposition ← instantiateMVars proposition
          if proposition.containsFVar x || proposition.hasLooseBVars then return some found
          return some (found.push (argument, proposition))
        visit argument found
    | .mdata _ body => visit body found
    | _ => return none
  visit e #[]

/-- An equation between a local and a value that mentions the local only
inside proofs, restated so that it does not: each such proof becomes a
hypothesis of its proposition, innermost first.  By proof irrelevance the
restated equation is the same up to definitional equality, and the local
can be substituted. -/
private partial def detachProofs (goal : MVarId) (equation : FVarId) (x : FVarId) :
    MetaM (Option MVarId) := goal.withContext do
  let type ← instantiateMVars (← equation.getType)
  unless type.isAppOfArity ``Eq 3 do return none
  let (value, valueOnLeft) :=
    if type.getArg! 2 == .fvar x then (type.getArg! 1, true) else (type.getArg! 2, false)
  let some proofs ← proofsMentioning x value | return none
  if proofs.isEmpty then return none
  let hypotheses := proofs.map fun (proof, proposition) =>
    ({ userName := `detached, type := proposition, value := proof } : Hypothesis)
  let (detached, goal) ← goal.assertHypotheses hypotheses
  goal.withContext do
    let value := (proofs.zip detached).foldl (fun value ((proof, _), hypothesis) =>
      value.replace fun sub => if sub == proof then some (.fvar hypothesis) else none) value
    let restated := if valueOnLeft then mkApp3 type.getAppFn (type.getArg! 0) value (.fvar x)
      else mkApp3 type.getAppFn (type.getArg! 0) (.fvar x) value
    let goal ← goal.replaceLocalDeclDefEq equation restated
    -- The proofs the detached ones exposed, until the local is gone.
    if value.containsFVar x then detachProofs goal equation x else return some goal

/-- Whether a runtime value is a constructor that fixes its variant: a
nominal value's variant is a literal. -/
private def fixesVariant (value : Lean.Expr) : Bool :=
  match value.getAppFn.constName? with
  | some ``LeanerIR.RuntimeValue.nominal =>
      value.getAppNumArgs == 3 && match (value.getArg! 1).consumeMData with
        | .app (.app (.const ``Option.some _) _) (.lit (.strVal _)) => true
        | .app (.const ``Option.none _) _ => true
        | _ => false
  | some name => name.getPrefix == ``LeanerIR.RuntimeValue
  | none => false

/-- A hypothesis fixing the encoding of a local, `encode τ x = e` with `x`
not in `e` and `e` a constructor fixing its variant, that `rewritten` does
not list. -/
private def encodingEquation? (goal : MVarId) (rewritten : Array FVarId) :
    MetaM (Option FVarId) := goal.withContext do
  (← getLCtx).findDeclM? fun decl => do
    if decl.isImplementationDetail || rewritten.contains decl.fvarId then return none
    let ty ← instantiateMVars decl.type
    unless ty.isAppOfArity ``Eq 3 do return none
    let encodes (side other : Lean.Expr) : Bool :=
      side.isAppOfArity ``NTy.encode 3 && fixesVariant other && match side.appArg! with
        | .fvar x => !other.containsFVar x
        | _ => false
    if encodes (ty.getArg! 1) (ty.getArg! 2) || encodes (ty.getArg! 2) (ty.getArg! 1) then
      return some decl.fvarId
    return none

/-- Rewrite the target and the other hypotheses by an equation fixing a
local's encoding, each where the rewrite is well typed. -/
private def rewriteByEncoding (goal : MVarId) (equation : FVarId) : MetaM MVarId :=
  goal.withContext do
    let type ← instantiateMVars (← equation.getType)
    let symm := !(type.getArg! 1).isAppOfArity ``NTy.encode 3
    let mut goal := goal
    for decl in ← getLCtx do
      if decl.isImplementationDetail || decl.fvarId == equation then continue
      let rewritten ← observing? do
        let result ← goal.rewrite decl.type (Lean.mkFVar equation) symm
        unless result.mvarIds.isEmpty do failure
        pure result
      if let some result := rewritten then
        goal := (← goal.replaceLocalDecl decl.fvarId result.eNew result.eqProof).mvarId
    let rewritten ← observing? do
      let result ← goal.rewrite (← goal.getType) (Lean.mkFVar equation) symm
      unless result.mvarIds.isEmpty do failure
      pure result
    if let some result := rewritten then
      goal ← goal.replaceTargetEq result.eNew result.eqProof
    return goal

/-- Close a goal by a case hypothesis `∀ xs, a = b → … → False` that an
instance refutes: the fall-through case of a match whose discriminant is
known to be a pattern's constructor, as a split states it or as the
normalizer leaves it, its equation taken apart. -/
private def refuteFallthrough (goal : MVarId) : MetaM Bool := goal.withContext do
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    unless type.isForall do continue
    let refuted ← observing? do
      let (arguments, _, body) ← forallMetaTelescopeReducing type
      unless body.isConstOf ``False do failure
      let mut equations := 0
      for argument in arguments do
        let argumentType ← instantiateMVars (← inferType argument)
        let_expr Eq _ left right := argumentType | continue
        unless ← isDefEq left right do failure
        argument.mvarId!.assign (← mkEqRefl left)
        equations := equations + 1
      if equations == 0 then failure
      let proof ← instantiateMVars (mkAppN decl.toExpr arguments)
      if proof.hasExprMVar then failure
      goal.assign (← mkFalseElim (← goal.getType) proof)
    if refuted.isSome then return true
  return false

/-- The normalization of `leaner_denote_normalize`, as a simp context and its
simprocs. It reads no hypothesis, so one serves every goal of a target. -/
def normalization : TacticM (Simp.Context × Simp.SimprocsArray) := do
  let stx ← `(tactic| simp only [lir_denote, lir_denote_norm, lir_denote_eval, Prod.fst, Prod.snd])
  let { ctx, simprocs, .. } ← mkSimpContext stx (eraseLocal := false)
  return (ctx, simprocs)

/-- Destructure a local along its native carrier: a variant sum into its
alternatives, a row into its fields, until the parts are scalars or
parameters. -/
private partial def destructureCarrier (goal : MVarId) (value : FVarId) :
    MetaM (Array MVarId) := goal.withContext do
  let type ← whnfD (← instantiateMVars (← value.getType))
  unless type.isAppOf ``Sum || type.isAppOf ``Prod || type.isAppOf ``PUnit ||
      type.isAppOf ``Empty do
    return #[goal]
  let cases ← goal.cases value
  cases.foldlM (init := #[]) fun found case => do
    let mut goals := #[case.mvarId]
    for field in case.fields do
      let .fvar part := field | continue
      goals ← goals.flatMapM fun goal => destructureCarrier goal part
    return found ++ goals

/-- Substitute every hypothesis equating a local variable to a term, by
syntactic shape alone: no hypothesis is unfolded to find one. -/
elab "leaner_denote_subst_vars" : tactic => do
  if (← getGoals).isEmpty then return
  let mut goal ← getMainGoal
  let mut progress := true
  let mut attempted : Array FVarId := #[]
  let mut encoded : Array FVarId := #[]
  while progress do
    progress := false
    let equation ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail then return none
        let ty ← instantiateMVars decl.type
        unless ty.isAppOfArity ``Eq 3 do return none
        let lhs := ty.getArg! 1
        let rhs := ty.getArg! 2
        let substitutable (x : Lean.Expr) (other : Lean.Expr) : Bool :=
          match x with
          | .fvar id => !other.containsFVar id
          | _ => false
        if substitutable lhs rhs || substitutable rhs lhs then return some (decl.fvarId, none)
        -- The local occurs on the other side, perhaps only inside proofs.
        if attempted.contains decl.fvarId then return none
        match rhs, lhs with
        | .fvar x, _ => return some (decl.fvarId, some x)
        | _, .fvar x => return some (decl.fvarId, some x)
        | _, _ => return none
    match equation with
    | some (fvarId, none) =>
        goal ← goal.withContext (Lean.Meta.subst goal fvarId)
        progress := true
    | some (fvarId, some x) =>
        attempted := attempted.push fvarId
        if let some detached ← detachProofs goal fvarId x then
          goal := detached
        progress := true
    | none =>
        -- A local whose encoding is fixed, such as a generic callee's
        -- result, is known wherever that encoding occurs.
        if let some fvarId ← encodingEquation? goal encoded then
          encoded := encoded.push fvarId
          goal ← rewriteByEncoding goal fvarId
          progress := true
  -- A match's fall-through case whose discriminant the substitutions have
  -- fixed to a constructor is refuted.
  if ← refuteFallthrough goal then replaceMainGoal []; return
  -- The substitutions replaced the main goal.
  replaceMainGoal [goal]
  -- At a leaf, a local whose variant is tested holds one of its variants;
  -- in each case the test computes. A goal still executing keeps its
  -- locals, which its steps take apart as they reach them.
  let executing ← goal.withContext do
    return ((← instantiateMVars (← goal.getType)).find? (·.isConstOf ``LeanerIR.Proofs.wp)).isSome
  let tested ← if executing then pure #[] else goal.withContext do
    let found ← IO.mkRef (#[] : Array FVarId)
    let mut expressions := #[← instantiateMVars (← goal.getType)]
    for decl in ← getLCtx do
      unless decl.isImplementationDetail do
        expressions := expressions.push (← instantiateMVars decl.type)
    for expression in expressions do
      Lean.Meta.forEachExpr expression fun e => do
        if e.isAppOfArity ``LeanerIR.Proofs.Denote.variantName 4 then
          if let .fvar x := e.appArg! then
            found.modify fun found => if found.contains x then found else found.push x
    found.get
  -- Built while the goal is open: destructuring assigns it.
  let normalization? ← if tested.isEmpty then pure none else some <$> normalization
  -- A local's own structure follows from its encoding once it is
  -- destructured; the alternatives an equation excludes compute to
  -- contradictions.
  let mut goals := #[goal]
  for equation in encoded do
    goals ← goals.flatMapM fun goal => goal.withContext do
      let some decl := (← getLCtx).find? equation | return #[goal]
      let type ← instantiateMVars decl.type
      let side := if (type.getArg! 1).isAppOfArity ``NTy.encode 3 then type.getArg! 1
        else type.getArg! 2
      let .fvar encodedLocal := side.appArg! | return #[goal]
      destructureCarrier goal encodedLocal
  let split := goals.size
  for local_ in tested do
    goals ← goals.flatMapM fun goal => goal.withContext do
      if (← getLCtx).contains local_ then destructureCarrier goal local_ else pure #[goal]
  let some (ctx, simprocs) := normalization? | replaceMainGoal goals.toList
  if goals.size == split then replaceMainGoal goals.toList; return
  -- The cases rewrite the hypotheses that mention a destructured local,
  -- where its tests now compute; the others are as they were.
  let before ← goal.withContext do
    return (← getLCtx).foldl (init := ({} : FVarIdSet)) fun known decl => known.insert decl.fvarId
  let mut remaining := #[]
  for case in goals do
    let rewritten := (← case.getNondepPropHyps).filter (!before.contains ·)
    let saved ← saveState
    try
      match ← simpGoal case ctx simprocs (fvarIdsToSimp := rewritten) with
      | (none, _) => pure ()
      | (some (_, next), _) => remaining := remaining.push next
    catch _ =>
      saved.restore
      remaining := remaining.push case
  replaceMainGoal remaining.toList

/-- Split a leaf goal into its cases: a conjunction into its parts, and a
binder, implication, or negation into a hypothesis, until the goal is
atomic.  Each case then carries the equations of its branch, which a
rewriting pass turns into closed arithmetic. -/
partial def splitGoal (goal : MVarId) : MetaM (List MVarId) := goal.withContext do
  let target ← whnfR (← instantiateMVars (← goal.getType))
  if target.isAppOfArity ``And 2 then
    let subgoals ← goal.apply (mkConst ``And.intro)
    return (← subgoals.mapM splitGoal).flatten
  if target.isAppOfArity ``Not 1 then
    let (_, next) ← (← goal.change (mkForall `h .default (target.getArg! 0) (mkConst ``False))).intro1
    return ← splitGoal next
  if target.isForall then
    -- A quantified variable keeps its name, for an authored proof.
    let (_, next) ← goal.intro1P
    return ← splitGoal next
  return [goal]

elab "leaner_denote_split_goal" : tactic => do
  if (← getGoals).isEmpty then return
  let goals ← (← getMainGoal).withContext (splitGoal (← getMainGoal))
  replaceMainGoal goals

/-- What a saturation round can change of a goal: its target and its
hypotheses. The hypotheses are compared as a collection: the rewriting
pass restates the hypotheses it changes after the others, so a round that
proves nothing new may still reorder them. -/
private def goalContent (goal : MVarId) : MetaM (Lean.Expr × Array Lean.Expr) := goal.withContext do
  let mut hypotheses := #[]
  for decl in ← getLCtx do
    unless decl.isImplementationDetail do
      hypotheses := hypotheses.push (← instantiateMVars decl.type)
  return (← instantiateMVars (← goal.getType), hypotheses.qsort (·.lt ·))

/-- The bounds asserted on a goal's ancestors, by the goal's content: the
rewriting pass restates each bound, or drops it as known, so asserting it
again would only be undone by the next pass. Emptied for every target. -/
initialize assertedBounds :
    IO.Ref (Std.HashMap (Lean.Expr × Array Lean.Expr) (Array Lean.Expr)) ← IO.mkRef {}

/-- A goal some hypothesis states, up to reducible unfolding. Plain
`assumption` compares at default transparency, which unfolds large
verification conditions at length before failing on the leaves nothing
closes. -/
elab "leaner_denote_assumption" : tactic => do
  let goal ← getMainGoal
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      if ← withReducible (isDefEq target (← instantiateMVars decl.type)) then
        goal.assign decl.toExpr
        replaceMainGoal []
        return
    throwError "no hypothesis states the goal"

/-- The hypothesis types a normalization pass of a prepared leaf left
normal in this target, up to the names of their free variables: the leaves
share their context, and restate it over the variables each destructures,
so a hypothesis normal at one leaf is not normalized again at the next,
where the pass would traverse it for nothing. Emptied for every target. -/
initialize normalHypotheses : IO.Ref (Std.HashSet (Lean.Expr × Array Lean.Expr)) ← IO.mkRef {}

/-- A type up to the names of its free variables: the variables abstracted
in order of occurrence, with their types, which the normalization may
read, abstracted the same way. -/
private def normalKey (type : Lean.Expr) : MetaM (Lean.Expr × Array Lean.Expr) := do
  let variables := (collectFVars {} type).fvarIds.map mkFVar
  let types ← variables.mapM fun fvar => do
    return (← instantiateMVars (← inferType fvar)).abstract variables
  return (type.abstract variables, types)

/-- Whether a hypothesis's normal form depends on its context: the
conditional rules of the pass (a conditional's branch, a truncating
division's form) read the context, so such a hypothesis may normalize
further where more is known. -/
private def contextDependent (type : Lean.Expr) : Bool :=
  (type.find? fun e => e.isAppOf ``ite || e.isAppOf ``dite || e.isAppOf ``Int.tmod ||
    e.isAppOf ``Int.tdiv).isSome

/-- Under `leaner.denoteDebug`: the discharger's calls, their heartbeats,
and the conditions asked. -/
private initialize dischargeLog : IO.Ref (Nat × Nat × Array String) ← IO.mkRef (0, 0, #[])

private def contextDischargeCore (context : LocalContext) (condition : Lean.Expr)
    (atoms : FVarIdSet) : MetaM (Option Lean.Expr) := do
  for decl in context do
    if decl.isImplementationDetail then continue
    if ← withReducible (isDefEq decl.type condition) then return some decl.toExpr
  -- A nonnegativity condition of a certified unsigned value, structurally.
  if condition.isAppOfArity ``LE.le 4 && (condition.getArg! 2).int? == some 0 then
    if let some proof ← nonnegative? (condition.getArg! 3) then return some proof
  let proof ← mkFreshExprMVar condition
  try
    let some refutation ← proof.mvarId!.falseOrByContra | return some (← instantiateMVars proof)
    refutation.withContext do
      let relevant ← (← getLCtx).foldlM (init := #[]) fun found decl => do
        if decl.isImplementationDetail then return found
        unless ← isProp decl.type do return found
        let shared := (decl.type.find? fun e => e.isFVar && atoms.contains e.fvarId!).isSome
        return if shared then found.push decl.toExpr else found
      Lean.Elab.Tactic.Omega.omega relevant.toList refutation {}
    return some (← instantiateMVars proof)
  catch _ => return none

/-- The discharger of the normalization's side conditions, over a leaf's
context: a condition about a variable simp introduced under a binder has no
fact in the context and is not attempted; otherwise a hypothesis states it
at reducible transparency, or omega proves it from the hypotheses sharing a
variable with it, not from the whole context, whose size the condition's
cost would otherwise follow. -/
private def contextDischarge (context : LocalContext) (decided : IO.Ref (Std.HashMap Lean.Expr (Option Lean.Expr))) :
    Simp.Discharge := fun condition => do
  let condition ← instantiateMVars condition
  let atoms := (collectFVars {} condition).fvarSet
  if atoms.isEmpty then return none
  if atoms.any fun fvarId => !context.contains fvarId then return none
  -- A condition asked before in this pass, decided or not, is answered again.
  if let some known := (← decided.get)[condition]? then return known
  let started ← IO.getNumHeartbeats
  let result ← contextDischargeCore context condition atoms
  decided.modify (·.insert condition result)
  if leaner.denoteDebug.get (← getOptions) then
    let finished ← IO.getNumHeartbeats
    let shown ← if result.isSome then pure "" else do
      pure ((toString (← ppExpr condition)).replace "\n" " " |>.take 120).toString
    dischargeLog.modify fun (calls, cost, failed) =>
      (calls + 1, cost + (finished - started), if result.isSome then failed else failed.push shown)
  return result

/-- The hypotheses a stage's `simp at *` normalization already left normal,
by shape: the stage set differs from the leaf set, so this is a cache of
its own. -/
private initialize stageNormalHypotheses : IO.Ref (Std.HashSet (Lean.Expr × Array Lean.Expr)) ←
  IO.mkRef {}

/-- Normalize the goal and the hypotheses a cache does not know normal, and
record every hypothesis of the result normal. Returns the hypotheses
normalized. -/
private def normalizeUncached (cache : IO.Ref (Std.HashSet (Lean.Expr × Array Lean.Expr)))
    (stx : Lean.TSyntax `tactic) (discharge? : Option Simp.Discharge) :
    TacticM (Array FVarId) := do
  let { ctx, simprocs, .. } ← mkSimpContext stx (eraseLocal := false)
  let known ← cache.get
  let goal ← getMainGoal
  let mut targets := #[]
  for fvarId in ← goal.getNondepPropHyps do
    let type ← instantiateMVars (← fvarId.getType)
    unless known.contains (← normalKey type) do targets := targets.push fvarId
  -- A context already normal is no failure.
  let ctx ← Simp.mkContext { ctx.config with failIfUnchanged := false } ctx.simpTheorems
    ctx.congrTheorems
  match ← simpGoal goal ctx simprocs discharge? (simplifyTarget := true)
      (fvarIdsToSimp := targets) with
  | (none, _) => replaceMainGoal []
  | (some (_, next), _) =>
      next.withContext do
        for decl in ← getLCtx do
          if decl.isImplementationDetail || decl.isLet then continue
          let type ← instantiateMVars decl.type
          if (← isProp type) && !contextDependent type then
            cache.modify (·.insert (← normalKey type))
      replaceMainGoal [next]
  return targets

/-- `leaner_denote_normalize at *`, skipping the hypotheses a stage already
left normal, as a loop step normalizes the whole context several times
while most hypotheses do not change. -/
elab "leaner_denote_normalize_context_stage" : tactic => withMainContext do
  discard <| normalizeUncached stageNormalHypotheses
    (← `(tactic| simp only [lir_denote, lir_denote_norm, lir_denote_eval, Prod.fst, Prod.snd]))
    none

/-- Normalize the goal and the hypotheses not known normal, as `simp at *`
would normalize the whole context: a conditional whose condition omega
decides, such as a vector's bound after an update, takes its branch, and a
condition a hypothesis states is proved by that hypothesis, so that the
branch's proof term mentions nothing a substitution would then have to
keep. Every hypothesis of the result is recorded normal. -/
elab "leaner_denote_normalize_context" : tactic => withMainContext do
  -- A side condition a hypothesis states is proved by it at reducible
  -- transparency: plain `assumption` would unfold the arithmetic of every
  -- hypothesis it compares against.
  let stx ← `(tactic| simp only
    [LeanerIR.Proofs.Obligation_iff, Nat.reduceAdd, Int.reducePow, Int.reduceSub, Nat.reducePow,
    Nat.reduceSub, Int.tmod_eq_emod_of_nonneg, Int.tdiv_eq_ediv_of_nonneg, lir_denote_norm, dif_pos,
    dif_neg, if_pos, if_neg, Prod.fst, Prod.snd])
  let goal ← getMainGoal
  let started ← IO.getNumHeartbeats
  -- A side condition about a variable simp introduced under a binder
  -- (`fun decoded => …`) has no fact in the context: the discharger is not
  -- run on it, which would try omega over the whole context for nothing.
  let context ← goal.withContext getLCtx
  let targets ← normalizeUncached normalHypotheses stx
    (some (contextDischarge context (← IO.mkRef {})))
  if leaner.denoteDebug.get (← getOptions) then
    let hypotheses ← goal.withContext goal.getNondepPropHyps
    let (calls, cost, failed) ← dischargeLog.get
    dischargeLog.set (0, 0, #[])
    IO.println s!"    normalize: {targets.size} of {hypotheses.size} hypotheses uncached, {((← IO.getNumHeartbeats) - started) / 1000}k; discharger {calls} calls {cost / 1000}k, {failed.size} failed"
    for c in failed.toList.take 4 do IO.println s!"      undecided: {c}"
    -- Conditional hypotheses the pass could not touch: dependent ones.
    goal.withContext do
      for decl in ← getLCtx do
        if decl.isImplementationDetail || decl.isLet then continue
        let type ← instantiateMVars decl.type
        unless ← isProp type do continue
        if hypotheses.contains decl.fvarId then continue
        if contextDependent type then
          IO.println s!"      dependent conditional hypothesis {decl.userName}: {((toString (← ppExpr type)).replace "\n" " " |>.take 160).toString}"

/-- Take each goal through `step` on its own, after asserting the bounds it
needs (`assert`), and record the bounds asserted on the goals `step` leaves
and their ancestors: those are not asserted on them again. -/
private def withBounds (assert : Bool) (step : Syntax) : TacticM Unit := do
  let mut remaining := []
  for goal in ← getGoals do
    let known := (← assertedBounds.get)[← goalContent goal]?.getD #[]
    setGoals [goal]
    let asserted := known ++ (← if assert then assertBounds known else pure #[])
    evalTactic step
    let results ← getGoals
    unless asserted.isEmpty do
      for result in results do
        let content ← goalContent result
        assertedBounds.modify (·.insert content asserted)
    remaining := remaining ++ results
  setGoals remaining

/-- Assert the bounds each goal needs, then run a tactic on it. -/
elab "leaner_denote_bounded " step:tactic : tactic => withBounds true step

/-- Run a tactic on each goal; the goals it leaves have their ancestor's
bounds. -/
elab "leaner_denote_inheriting " step:tactic : tactic => withBounds false step

/-- Run a `simp only` call at the given hypotheses and, when asked, the
goal, with the given hypotheses as rewrite rules besides its lemmas. The
hypotheses are addressed by identity: a name a hypothesis has lost cannot
be spelled in a location. Returns the goal left, `none` when closed. -/
private def simpAt (goal : MVarId) (call : Lean.TSyntax `tactic) (rules : Array FVarId)
    (targets : Array FVarId) (simplifyTarget : Bool) : TacticM (Option MVarId) :=
  goal.withContext do
    let { ctx, simprocs, dischargeWrapper, .. } ← mkSimpContext call (eraseLocal := false)
    let mut theorems := ctx.simpTheorems
    unless rules.isEmpty do
      let mut extra : SimpTheorems := {}
      for fvarId in rules do
        extra ← extra.add (.fvar fvarId) #[] (mkFVar fvarId)
      theorems := theorems.push extra
    let ctx ← Simp.mkContext { ctx.config with failIfUnchanged := false } theorems ctx.congrTheorems
    let targets := targets.filter (!rules.contains ·)
    let (result, _) ← dischargeWrapper.with fun discharge? =>
      simpGoal goal ctx simprocs discharge? (simplifyTarget := simplifyTarget)
        (fvarIdsToSimp := targets)
    return result.map (·.2)

/-- `simpAt` as a tactic step over the main goal. -/
private def simpMainAt (call : Lean.TSyntax `tactic) (rules : Array FVarId) (targets : Array FVarId)
    (simplifyTarget : Bool) : TacticM Unit := do
  match ← simpAt (← getMainGoal) call rules targets simplifyTarget with
  | none => replaceMainGoal []
  | some goal => replaceMainGoal [goal]

/-- The hypotheses whose statement satisfies a predicate. -/
private def hypothesesWhere (goal : MVarId) (predicate : Lean.Expr → Bool) : MetaM (Array FVarId) :=
  goal.withContext do
    (← goal.getNondepPropHyps).filterM fun fvarId => do
      return predicate (← instantiateMVars (← fvarId.getType))

/-- Read positions of written arrays: a lookup at a position omega tells
apart from the written one reads the array before the write, and a lookup
at the written position within bounds reads the written value; a lookup
into an array with an element removed or inserted reads the array before
where omega places the position against the removed or inserted one. Runs
only where the context holds a write, so other leaves pay nothing. -/
elab "leaner_denote_lookups_after_writes" : tactic => do
  if (← getGoals).isEmpty then return
  let goal ← getMainGoal
  let isWrite (e : Lean.Expr) := (e.find? fun e => e.isConstOf ``Array.setIfInBounds ||
    e.isConstOf ``Array.eraseIdx || e.isConstOf ``Array.insertIdx ||
    e.isConstOf ``Array.eraseIdxIfInBounds || e.isConstOf ``Array.insertIdxIfInBounds).isSome
  let (inTarget, written) ← goal.withContext do
    let inTarget := isWrite (← instantiateMVars (← goal.getType))
    let written ← (← goal.getNondepPropHyps).filterM fun fvarId => do
      return isWrite (← instantiateMVars (← fvarId.getType))
    pure (inTarget, written)
  if !inTarget && written.isEmpty then return
  -- Only where a write is read: the other hypotheses would be traversed
  -- for nothing.
  simpMainAt (← `(tactic| simp (disch := omega) only [Array.getElem?_setIfInBounds_ne,
    Array.getElem?_setIfInBounds_self_of_lt, Array.getElem_setIfInBounds_ne,
    Array.getElem_setIfInBounds_self, LeanerIR.Proofs.Denote.eraseIdxIfInBounds_of_lt,
    LeanerIR.Proofs.Denote.insertIdxIfInBounds_of_le, Array.size_eraseIdx, Array.size_insertIdx,
    LeanerIR.Proofs.Denote.getElem?_eraseIdx_toNat_of_lt,
    LeanerIR.Proofs.Denote.getElem?_eraseIdx_toNat_of_ge,
    LeanerIR.Proofs.Denote.getElem?_insertIdx_toNat_of_lt,
    LeanerIR.Proofs.Denote.getElem?_insertIdx_toNat_of_gt, Array.getElem?_insertIdx_self]))
    #[] written inTarget

/-- The keys of a map held as the vector of entries of a single-variant enum,
encoded, are the encodings of their first fields. -/
theorem keysRead_encode_enum [Skolems] {α : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (source : StructHandle) (name : String) (entry : StructHandle)
    (key : NTy) (rest : NRow) (distinct : [name].Nodup)
    (value : variantCarrier [name] (.cons (.cons (NTy.vector (NTy.struct entry (.cons key rest))) .nil) .nil))
    (roundTrip : ∀ element : key.carrier, write (read (key.encode element)) = key.encode element) :
    Maps.KeysRead read write
      (NTy.encode (.enum source [name] (.cons (.cons (NTy.vector (NTy.struct entry (.cons key rest))) .nil) .nil) distinct) value) := by
  cases value with
  | inl fields =>
      simp only [NTy.encode_enum_inl, HList.encode_cons, HList.encode_nil, NTy.encode_vector]
      apply Maps.keysRead_map
      intro element
      simp only [NTy.codec_encode, NTy.encode_struct, HList.encode_cons]
      exact roundTrip element.1
  | inr empty => exact empty.elim

/-- The same for a map held as the vector of entries of a struct. -/
theorem keysRead_encode_struct [Skolems] {α : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (source : StructHandle) (entry : StructHandle)
    (key : NTy) (rest : NRow)
    (value : HList (.cons (NTy.vector (NTy.struct entry (.cons key rest))) .nil))
    (roundTrip : ∀ element : key.carrier, write (read (key.encode element)) = key.encode element) :
    Maps.KeysRead read write
      (NTy.encode (.struct source (.cons (NTy.vector (NTy.struct entry (.cons key rest))) .nil)) value) := by
  simp only [NTy.encode_struct, HList.encode_cons, HList.encode_nil, NTy.encode_vector]
  apply Maps.keysRead_map
  intro element
  simp only [NTy.codec_encode, NTy.encode_struct, HList.encode_cons]
  exact roundTrip element.1

/-- The ground positions a map is read at: the positions of its keys the
context reads, and the ranks of keys in it. -/
private def mapPositions (terms : Array Lean.Expr) (map : Lean.Expr) : MetaM (Array Lean.Expr) := do
  let reads := sitesWhere (fun e => !e.hasLooseBVars &&
    (e.isAppOfArity ``LeanerIR.Maps.keyAt 2 || e.isAppOfArity ``LeanerIR.Maps.rank 2)) terms
  let mut positions := #[]
  for read in reads do
    unless ← isDefEq (read.getArg! 0) map do continue
    let position := if read.isAppOfArity ``LeanerIR.Maps.keyAt 2 then read.getArg! 1 else read
    unless positions.contains position do positions := positions.push position
  return positions

/-- Close a goal by omega over its hypotheses, as the tactic does: by
contradiction, over the local context. -/
private def decideByOmega (goal : MVarId) : MetaM Unit := do
  let some goal ← goal.falseOrByContra | return
  goal.withContext do
    Lean.Elab.Tactic.Omega.omega (← getLocalHyps).toList goal {}

/-- A proof of a statement in a goal's context by omega, if omega finds one. -/
private def omegaProof? (goal : MVarId) (statement : Lean.Expr) : MetaM (Option Lean.Expr) :=
  goal.withContext do
    let proof ← mkFreshExprMVar statement
    try decideByOmega proof.mvarId!; some <$> instantiateMVars proof
    catch _ => pure none

/-- A proof that a map's keys are integers read back, by the lemmas over the
entries layouts, if they give one. -/
private def integerKeysProof? (goal : MVarId) (map : Lean.Expr) : TacticM (Option Lean.Expr) := do
  let saved ← getGoals
  try
    let proof ← goal.withContext (mkFreshExprMVar (← mkAppM ``LeanerIR.Maps.KeysRead
      #[mkConst ``LeanerIR.RuntimeValue.asInt, mkConst ``LeanerIR.RuntimeValue.integer, map]))
    setGoals [proof.mvarId!]
    evalTactic (← `(tactic| simp only [LeanerIR.Maps.keysRead_map, keysRead_encode_enum,
      keysRead_encode_struct, lir_denote, lir_denote_norm, lir_denote_eval]))
    if (← getGoals).isEmpty then some <$> instantiateMVars proof else pure none
  catch _ => pure none
  finally setGoals saved

/-- The facts the positions a map is read at give, as an SMT solver's map
axioms would: a key at a position is a key of the map, so it differs from
each integer the map lacks; the keys of a valid ordered map ascend with their
positions; and a hypothesis quantified over the positions of a map holds at
each position the context reads it at. -/
private def mapPositionFacts (goal : MVarId) : TacticM (Array (Lean.Expr × Lean.Expr)) := do
  let (terms, absent, present, ordered, quantified) ← goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    let mut absent : Array (Lean.Expr × Lean.Expr × Lean.Expr) := #[]
    let mut present : Array (Lean.Expr × Lean.Expr × Lean.Expr) := #[]
    let mut ordered : Array (Lean.Expr × Lean.Expr) := #[]
    let mut quantified : Array (Lean.Expr × Lean.Expr) := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      unless ← isProp ty do continue
      terms := terms.push ty
      if let some (_, lhs, rhs) := ty.eq? then
        if rhs.isConstOf ``Bool.false && lhs.isAppOfArity ``LeanerIR.Maps.hasKey 2 then
          let key := lhs.getArg! 1
          if key.isAppOfArity ``LeanerIR.RuntimeValue.integer 1 then
            absent := absent.push (lhs.getArg! 0, key.getArg! 0, decl.toExpr)
        if rhs.isConstOf ``Bool.true && lhs.isAppOfArity ``LeanerIR.Maps.hasKey 2 then
          present := present.push (lhs.getArg! 0, lhs.getArg! 1, decl.toExpr)
      if ty.isAppOfArity ``LeanerIR.Maps.Valid 2 &&
          (ty.getArg! 0).isAppOfArity ``LeanerIR.Maps.Discipline.ordered 1 then
        ordered := ordered.push (ty.getArg! 1, decl.toExpr)
      if ty.isForall && (ty.find? (·.isConstOf ``LeanerIR.Maps.keyAt)).isSome then
        quantified := quantified.push (decl.toExpr, ty)
    pure (terms, absent, present, ordered, quantified)
  let mut facts : Array (Lean.Expr × Lean.Expr) := #[]
  -- A key the map has sits at its rank, within the map.
  for (map, key, found) in present do
    let located ← goal.withContext (mkAppM ``LeanerIR.Maps.keyAt_rank #[found])
    let below ← goal.withContext (mkAppM ``LeanerIR.Maps.rank_lt_size #[found])
    let nonneg ← goal.withContext (mkAppM ``LeanerIR.Maps.rank_nonneg #[map, key])
    for fact in #[located, below, nonneg] do
      facts := facts.push (← goal.withContext (inferType fact), fact)
  -- The positions each map is read at, and the ranks of the keys it has;
  -- and whether its keys are integers read back. Once per map.
  let mut positions : Std.HashMap Lean.Expr (Array Lean.Expr) := {}
  let mut keys : Std.HashMap Lean.Expr (Option Lean.Expr) := {}
  for map in absent.map (·.1) ++ ordered.map (·.1) do
    if positions.contains map then continue
    let read ← goal.withContext do
      let mut read ← mapPositions terms map
      for (other, key, _) in present do
        if ← isDefEq other map then
          let rank ← mkAppM ``LeanerIR.Maps.rank #[map, key]
          unless read.contains rank do read := read.push rank
      return read
    positions := positions.insert map read
    keys := keys.insert map (← integerKeysProof? goal map)
  for (map, _, missing) in absent do
    let some keysRead := keys.getD map none | continue
    for index in positions.getD map #[] do
      if index.isAppOfArity ``LeanerIR.Maps.rank 2 then continue
      let fact? ← goal.withContext do
        let some low ← omegaProof? goal (← mkAppM ``LE.le #[mkIntLit 0, index]) | return none
        let some high ← omegaProof? goal
            (← mkAppM ``LT.lt #[index, ← mkAppM ``LeanerIR.Maps.size #[map]]) | return none
        let fact ← mkAppM ``LeanerIR.Maps.keyAt_asInt_ne_of_absent #[keysRead, low, high, missing]
        return some (← inferType fact, fact)
      if let some fact := fact? then facts := facts.push fact
  for (map, valid) in ordered do
    let read := positions.getD map #[]
    if read.size < 2 || read.size > 6 then continue
    let some keysRead := keys.getD map none | continue
    for first in read do
      for second in read do
        if first == second then continue
        for fact? in #[mkAppOptM ``LeanerIR.Maps.keyAt_asInt_lt
              #[none, none, valid, keysRead, first, second],
            -- Equal positions hold equal keys.
            mkAppM ``LeanerIR.Maps.keyAt_asInt_congr #[map, first, second]] do
          let fact? ← goal.withContext do
            try let fact ← fact?; pure (some (← inferType fact, fact)) catch _ => pure none
          if let some fact := fact? then facts := facts.push fact
  for (hypothesis, ty) in quantified do
    let .forallE _ binder _ _ := ty | continue
    unless binder.isConstOf ``Int do continue
    for (map, _) in ordered do
      for position in positions.getD map #[] do
        facts := facts.push (ty.bindingBody!.instantiate1 position, mkApp hypothesis position)
  return facts

/-- Read the keys of maps at positions within them: a map whose keys are
scalar encodings holds, at such a position, the encoding of the key's
reading, so a key read at its scalar type and written back is the key, and
an order on it is the order on its reading. Runs only where the context
reads a key at a position; omega decides the positions. -/
elab "leaner_denote_map_positions" : tactic => do
  if (← getGoals).isEmpty then return
  let reads (e : Lean.Expr) := (e.find? (·.isConstOf ``LeanerIR.Maps.keyAt)).isSome
  let read ← (← getMainGoal).withContext do
    if reads (← instantiateMVars (← (← getMainGoal).getType)) then return true
    (← getLCtx).anyM fun decl => do
      return !decl.isImplementationDetail && reads (← instantiateMVars decl.type)
  unless read do return
  -- The facts the positions are decided by, out of their obligations.
  evalTactic (← `(tactic| try simp only [LeanerIR.Proofs.Obligation_iff] at *))
  if (← getGoals).isEmpty then return
  let mut goal ← getMainGoal
  -- A fact the context states is not asserted again.
  let mut stated : Std.HashSet Lean.Expr ← goal.withContext do
    (← goal.getNondepPropHyps).foldlM (init := {}) fun stated fvarId => do
      return stated.insert (← instantiateMVars (← fvarId.getType))
  for (statement, proof) in ← mapPositionFacts goal do
    if stated.contains statement then continue
    stated := stated.insert statement
    goal ← (← goal.assert `mapFact statement proof).intro1P <&> (·.2)
  replaceMainGoal [goal]
  let (inTarget, reading, context) ← goal.withContext do
    let inTarget := reads (← instantiateMVars (← goal.getType))
    let hypotheses ← goal.getNondepPropHyps
    let reading ← hypotheses.filterM fun fvarId => do
      return reads (← instantiateMVars (← fvarId.getType))
    -- Facts without binders only: a quantified one is rewritten, never used.
    let context ← hypotheses.filterM fun fvarId => do
      if reading.contains fvarId then return false
      let ty ← instantiateMVars (← fvarId.getType)
      return !(ty.find? (·.isForall)).isSome
    pure (inTarget, reading, context)
  -- The other hypotheses rewrite along, so that a case a position decides,
  -- such as a lookup at another position than the written one, reduces in
  -- the same pass. Contextually, so that the premises bounding a quantified
  -- position decide it.
  simpMainAt (← `(tactic| simp (config := { contextual := true }) (disch := first
      | omega
      | assumption
      | (simp (disch := omega) only [LeanerIR.Maps.hasKey_keyAt]; done)
      | (simp only [LeanerIR.Maps.keysRead_map, keysRead_encode_enum, keysRead_encode_struct,
        lir_denote, lir_denote_norm, lir_denote_eval]; done))
    only [LeanerIR.Maps.integer_asInt_keyAt, LeanerIR.Maps.bool_asBool_keyAt,
      LeanerIR.Maps.address_asString_keyAt, LeanerIR.Maps.order_keyAt_integer,
      LeanerIR.Maps.order_integer_keyAt, LeanerIR.Maps.order_keyAt_keyAt,
      LeanerIR.RuntimeValue.order_integer,
      LeanerIR.Maps.hasKey_keyAt, LeanerIR.Maps.rank_keyAt, LeanerIR.Maps.keyAt_rank,
      LeanerIR.Maps.keyAt_update_present, LeanerIR.Maps.keyAt_eq_keyAt_iff,
      LeanerIR.Maps.size_update, LeanerIR.Maps.valueAt_update, LeanerIR.Maps.asInt_integer,
      Int.compare_eq_lt, Int.compare_eq_gt, Int.compare_eq_eq, true_implies, implies_true,
      ↓reduceIte]))
    context reading inTarget
/-- One saturation round: bound the integers, destructure the hypotheses,
substitute witnesses, and rewrite with every hypothesis. -/
macro "leaner_denote_saturate_round" : tactic => `(tactic|
  leaner_denote_bounded (
    all_goals leaner_denote_split_hypotheses
    all_goals leaner_denote_subst_vars
    all_goals (try leaner_simp_all [lir_denote_norm])
    -- A goal about vectors is decided at their values; a hypothesis keeps
    -- the vector, which substitution reads.
    all_goals (try simp only [LeanerIR.SpecVector.eq_iff_values, lir_denote_norm])
    all_goals leaner_denote_lookups_after_writes
    all_goals leaner_denote_map_positions))

/-- Saturate a leaf's context: rounds of rewriting while new hypotheses
appear, after the round the leaf has already run, at most four. A round
that leaves the goals as they were has reached the fixed point, and a
further round would repeat it. -/
elab "leaner_denote_saturate" : tactic => do
  for _ in [0:4] do
    let before ← (← getGoals).mapM fun goal => goalContent goal
    evalTactic (← `(tactic| leaner_denote_saturate_round))
    if (← (← getGoals).mapM fun goal => goalContent goal) == before then return

/-- `decide` on a closed goal only: on an open one the kernel evaluates
the instance for as long as the goal is large, then fails anyway. -/
elab "leaner_denote_decide" : tactic => do
  let target ← instantiateMVars (← (← getMainGoal).getType)
  if target.hasFVar then throwError "the goal is not closed"
  evalTactic (← `(tactic| decide))

/-- Identify the elements two lookups of one position found: from
`e = some a` and `e = some b`, `a = b`, which injection then splits. -/
elab "leaner_denote_merge_lookups" : tactic => do
  let goal ← getMainGoal
  let found ← goal.withContext do
    let decls := (← getLCtx).foldl (init := #[]) fun found decl =>
      if decl.isImplementationDetail then found else found.push decl
    let mut pairs : Array (Lean.Expr × Lean.Expr) := #[]
    for i in [:decls.size] do
      let ti ← instantiateMVars decls[i]!.type
      unless ti.isAppOfArity ``Eq 3 && (ti.getArg! 2).isAppOfArity ``Option.some 2 do continue
      for j in [i + 1:decls.size] do
        let tj ← instantiateMVars decls[j]!.type
        unless tj.isAppOfArity ``Eq 3 && (tj.getArg! 2).isAppOfArity ``Option.some 2 do continue
        if (ti.getArg! 2) != (tj.getArg! 2) then
          if ← withReducible (isDefEq (ti.getArg! 1) (tj.getArg! 1)) then
            pairs := pairs.push (decls[i]!.toExpr, decls[j]!.toExpr)
    return pairs
  let mut goal := goal
  for (left, right) in found do
    let proof ← goal.withContext do
      mkAppM ``Option.some.inj #[← mkEqTrans (← mkEqSymm left) right]
    let (_, next) ← (← goal.assert `sameElement (← goal.withContext (inferType proof)) proof).intro1P
    goal := next
  replaceMainGoal [goal]

/-- Split on the result of a search the goal or a hypothesis reads: the
search finds nothing, or a position, characterized by the elements at and
before it. Fails when nothing reads a search. -/
elab "leaner_denote_split_search" : tactic => do
  let goal ← getMainGoal
  -- A search over a literal vector computes instead.
  let isSearch (e : Lean.Expr) := e.isAppOfArity ``findIndex? 6 && !e.hasLooseBVars && e.hasFVar
  let (search?, reading) ← goal.withContext do
    let mut found := (← instantiateMVars (← goal.getType)).find? isSearch
    let mut reading : Array FVarId := #[]
    for fvarId in ← goal.getNondepPropHyps do
      let type ← instantiateMVars (← fvarId.getType)
      if let some search := type.find? isSearch then
        reading := reading.push fvarId
        if found.isNone then found := some search
    pure (found, reading)
  let some search := search? | throwError "nothing reads a search"
  -- The result is `none` or `some found`; the equation rewrites every
  -- reader, including a removal's bound on the position.
  let cases ← goal.withContext (mkAppM ``Option.eq_none_or_eq_some #[search])
  let (disjunction, goal) ← (← goal.assert `search (← goal.withContext (inferType cases)) cases).intro1P
  let mut goals := #[]
  for subgoal in ← goal.cases disjunction do
    match subgoal.ctorName with
    | some ``Or.inl =>
        let some equation := subgoal.fields[0]? | throwError "a search with no result"
        let branch ← subgoal.mvarId.rename equation.fvarId! `search
        setGoals [branch]
        simpMainAt (← `(tactic| simp only [Option.getD_none, Option.isSome_none,
          Bool.false_eq_true])) #[equation.fvarId!] reading true
        evalTactic (← `(tactic| try rw [LeanerIR.Proofs.Denote.findIndex?_eq_none_iff]
          at $(mkIdent `search):ident))
    | some ``Or.inr =>
        let some existential := subgoal.fields[0]? | throwError "a search with a result"
        let some found := (← subgoal.mvarId.cases existential.fvarId!)[0]?
          | throwError "a found position"
        let some position := found.fields[0]? | throwError "a found position"
        let some equation := found.fields[1]? | throwError "a found position's equation"
        let branch ← found.mvarId.rename position.fvarId! `found
        let branch ← branch.rename equation.fvarId! `search
        setGoals [branch]
        simpMainAt (← `(tactic| simp only [Option.getD_some, Option.isSome_some]))
          #[equation.fvarId!] reading true
        evalTactic (← `(tactic| try rw [LeanerIR.Proofs.Denote.findIndex?_eq_some_iff]
          at $(mkIdent `search):ident))
    | _ => throwError "a search has a case besides its result"
    goals := goals ++ (← getGoals)
  setGoals goals.toList

/-- Canonical family spellings (`canonicalFamilies`). -/
syntax "leaner_denote_canonical_families" : tactic

/-- The normalization every leaf receives before a decision: the leaf an
authored proof takes over is left in this form. -/
macro "leaner_denote_prepare" : tactic => `(tactic| (
  all_goals leaner_denote_canonical_families
  leaner_denote_timed "p-clear" leaner_denote_clear_computations
  try leaner_denote_split_search
  all_goals leaner_denote_timed "p-split" leaner_denote_split_hypotheses
  all_goals leaner_denote_timed "p-subst" leaner_denote_subst_vars
  all_goals leaner_denote_timed "p-proj" leaner_denote_reduce_projections
  all_goals leaner_denote_timed "p-bounds" leaner_denote_bounds
  all_goals leaner_denote_timed "normalize" leaner_denote_normalize_context
  all_goals leaner_denote_timed "p-split2" leaner_denote_split_hypotheses
  all_goals leaner_denote_timed "p-subst2" leaner_denote_subst_vars
  all_goals leaner_denote_timed "p-proj2" leaner_denote_reduce_projections
  -- Lookups of one position, once substitution has exposed them.
  all_goals leaner_denote_timed "p-merge" leaner_denote_merge_lookups
  all_goals leaner_denote_timed "p-split3" leaner_denote_split_hypotheses
  all_goals leaner_denote_timed "p-subst3" leaner_denote_subst_vars
  -- The terms a substitution brought in, such as a written vector, in
  -- normal form too, and the lookups after writes read.
  all_goals leaner_denote_timed "renormalize" leaner_denote_normalize_context
  all_goals leaner_denote_lookups_after_writes
  all_goals leaner_denote_timed "p-positions" leaner_denote_map_positions))

/-- Split the goal's first conditional or match. A match's fall-through
case that its computed discriminant refutes is closed at once. -/
elab "leaner_denote_split" : tactic => do
  -- A discriminant a hypothesis equates to a term is split as that term.
  let original ← getMainGoal
  evalTactic (← `(tactic| leaner_denote_subst_vars))
  -- A refuted or destructured goal is progress enough.
  if (← getGoals).length != 1 then return
  -- The context stays normalized for the deciders that rewrite with it.
  if (← getMainGoal) != original then
    evalTactic (← `(tactic| try simp only [lir_denote, lir_denote_norm, lir_denote_eval,
      Prod.fst, Prod.snd] at *))
  evalTactic (← `(tactic| split))
  let goals ← (← getGoals).filterM fun goal => do return !(← refuteFallthrough goal)
  setGoals goals

/-- A premise of an instance: a hypothesis, either way round, arithmetic,
or reflexivity. -/
macro "leaner_denote_instance_premise" : tactic => `(tactic|
  first
  | leaner_denote_assumption
  | omega
  | with_reducible rfl
  | (symm; leaner_denote_assumption))

/-- A position as an integer: the integer it is `Int.toNat` of, or its cast. -/
private def intPosition (position : Lean.Expr) : MetaM Lean.Expr :=
  if position.isAppOfArity ``Int.toNat 1 then pure (position.getArg! 0)
  else mkAppOptM ``Nat.cast #[mkConst ``Int, none, position]

/-- Decide the goal by the context: the goal alone in normal form
rewritten by the hypotheses, then by arithmetic. -/
syntax "leaner_denote_decide_by_context" : tactic

/-- The positions written in a term, as integers: a push writes at the
array's size, a `setIfInBounds` at its position. -/
private def writtenPositions (e : Lean.Expr) : MetaM (Array Lean.Expr) :=
  let writes := sitesWhere (fun e => !e.hasLooseBVars && (e.isAppOfArity ``Array.push 3 ||
    e.isAppOfArity ``Array.setIfInBounds 4 && (e.getArg! 2).isAppOfArity ``Int.toNat 1)) #[e]
  writes.mapM fun write => do
    if write.isAppOfArity ``Array.push 3 then
      mkAppOptM ``Nat.cast #[mkConst ``Int, none, ← mkAppM ``Array.size #[write.getArg! 1]]
    else pure ((write.getArg! 2).getArg! 0)

/-- A premise of a witness at a position the context reads or writes: a
premise of an instance, the lookup at the position read, or the goal
rewritten by the context. -/
macro "leaner_denote_witness_premise" : tactic => `(tactic|
  first
  | leaner_denote_instance_premise
  | (simp (disch := omega) only [Int.toNat_natCast, Array.getElem?_push_size, LeanerIR.Proofs.Denote.getElem?_push_of_lt,
      Array.getElem?_setIfInBounds_self_of_lt, Array.getElem?_setIfInBounds_ne]; done)
  | leaner_denote_decide_by_context)

/-- An existential goal whose body some hypothesis states at a witness, or
that a variable of the context or a written position witnesses. -/
elab "leaner_denote_witness" : tactic => do
  let goal ← getMainGoal
  let others := (← getGoals).filter (· != goal)
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``Exists 2 do throwError "not an existential"
    let body := target.getArg! 1
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let witness ← mkFreshExprMVar (target.getArg! 0)
      let saved ← saveState
      if ← isDefEq (body.beta #[witness]) (← instantiateMVars decl.type) then
        let proof ← mkAppOptM ``Exists.intro #[target.getArg! 0, body, ← instantiateMVars witness,
          decl.toExpr]
        goal.assign proof
        setGoals others
        return
      restoreState saved
    -- A candidate witness, the body decided conjunct by conjunct.
    let attempt (witness : Lean.Expr) (premise : Lean.TSyntax `tactic) : TacticM Bool := do
      unless ← isDefEq (← inferType witness) (target.getArg! 0) do return false
      let saved ← saveState
      try
        let instantiated ← mkFreshExprMVar (body.beta #[witness])
        setGoals [instantiated.mvarId!]
        evalTactic (← `(tactic| repeat' apply And.intro))
        -- A disjunctive premise by one of its sides.
        evalTactic (← `(tactic| all_goals first
          | $premise:tactic
          | (apply Or.inl; repeat' apply And.intro; all_goals $premise:tactic)
          | (apply Or.inr; repeat' apply And.intro; all_goals $premise:tactic)))
        if (← getGoals).isEmpty then
          goal.assign (← mkAppOptM ``Exists.intro
            #[target.getArg! 0, body, witness, ← instantiateMVars instantiated])
          setGoals others
          return true
      catch _ => pure ()
      restoreState saved
      return false
    -- A variable of the binder's type in the context, or the value of a
    -- certified integer for an integer binder.
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      if ← attempt decl.toExpr (← `(tactic| leaner_denote_instance_premise)) then return
    if (target.getArg! 0).isConstOf ``Int then
      for decl in ← getLCtx do
        if decl.isImplementationDetail || decl.isLet then continue
        let some value ← (try some <$> mkAppM ``LeanerIR.SpecInt.val #[decl.toExpr]
            catch _ => pure none) | continue
        if ← attempt value (← `(tactic| leaner_denote_witness_premise)) then return
    -- A position the body reads an array written at, or one a lookup
    -- fact of the context reads.
    for position in ← writtenPositions body do
      if ← attempt position (← `(tactic| leaner_denote_witness_premise)) then return
    let mut read : Array Lean.Expr := #[]
    for fvarId in ← goal.getNondepPropHyps do
      let fact ← instantiateMVars (← fvarId.getType)
      unless fact.isAppOfArity ``Eq 3 && (fact.getArg! 2).isAppOfArity ``Option.some 2 do continue
      let lookup := fact.getArg! 1
      unless lookup.isAppOfArity ``GetElem?.getElem? 7 do continue
      let position ← intPosition (lookup.getArg! 6)
      unless read.contains position do read := read.push position
    for position in read do
      if ← attempt position (← `(tactic| leaner_denote_witness_premise)) then return
    throwError "no hypothesis states the existential at a witness"

/-- The cheap deciders of a leaf: those that never rewrite the context. -/
syntax "leaner_denote_decide_cheap" : tactic

macro_rules
  | `(tactic| leaner_denote_decide_cheap) => `(tactic|
  first
  | leaner_denote_assumption
  | leaner_denote_witness
  | (simp only [LeanerIR.Proofs.Obligation_iff, Nat.reduceAdd, Int.reducePow, Int.reduceSub,
      Nat.reducePow, Nat.reduceSub]
     first
     | done
     | leaner_denote_assumption
     | (leaner_denote_bounds; omega)
     | leaner_denote_decide
     | leaner_denote_witness
     -- A conjunction, such as a clause with an abort code, conjunct by
     -- conjunct.
     | (apply And.intro <;> leaner_denote_decide_cheap)))

/-- `trivial` at reducible transparency and without `decide` and
`contradiction`: an open goal's decision and a search for contradicting
hypotheses belong to the deciders, which bound them. -/
syntax "leaner_denote_trivial" : tactic

macro_rules
  | `(tactic| leaner_denote_trivial) => `(tactic|
      first
      | leaner_denote_assumption
      | with_reducible rfl
      | exact True.intro
      | (apply And.intro <;> leaner_denote_trivial))

/-- A quantified statement with its negated conclusion spelled as the
implication it is, so that `apply` at reducible transparency sees a `False`
goal as its instance. -/
private def unfoldNegatedConclusion : Lean.Expr → Option Lean.Expr
  | .forallE name domain body info =>
      (unfoldNegatedConclusion body).map (.forallE name domain · info)
  | e => if e.isAppOfArity ``Not 1 then some (.forallE `negated (e.getArg! 0) (mkConst ``False) .default)
    else none

/-- A goal that is an instance of a quantified hypothesis: the hypothesis
applied at reducible transparency, its premises closed by omega or by a
hypothesis. -/
elab "leaner_denote_instance" : tactic => do
  let initial ← saveState
  let (goal, _) ← introduceThroughMarkers (← getMainGoal)
  let refuting ← goal.withContext do return (← instantiateMVars (← goal.getType)).isConstOf ``False
  let candidates ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let ty ← instantiateMVars decl.type
      unless ty.isForall do return found
      if refuting then
        if let some unfolded := unfoldNegatedConclusion ty then
          return found.push (← mkExpectedTypeHint decl.toExpr unfolded)
      return found.push decl.toExpr
  for candidate in candidates.reverse do
    let saved ← saveState
    try
      let premises ← withReducible (goal.apply candidate)
      -- The premises that pin the instance, such as a lookup, before the
      -- bounds a hypothesis of any position would satisfy.
      let (bounds, pinning) ← goal.withContext do premises.partitionM fun premise => do
        let ty ← instantiateMVars (← premise.getType)
        return ty.isAppOfArity ``LE.le 4 || ty.isAppOfArity ``LT.lt 4 ||
          ty.isAppOfArity ``GE.ge 4 || ty.isAppOfArity ``GT.gt 4
      setGoals (pinning ++ bounds)
      evalTactic (← `(tactic| all_goals leaner_denote_instance_premise))
      if (← getGoals).isEmpty then return
    catch _ => pure ()
    saved.restore
  initial.restore
  throwError "no quantified hypothesis has the goal as an instance"

/-- The goal, or the given hypotheses, in normal form rewritten by the
other hypotheses, but for the equations that would rewrite a term into one
containing it. An element a hypothesis fixes (`a[i] = v`) is read through
the lookup the leaves state (`a[i]? = some v`). -/
private def simpByContext (goal : MVarId) (targets : Array FVarId) (simplifyTarget : Bool)
    (failIfUnchanged : Bool) : TacticM (Option MVarId) := goal.withContext do
  let stx ← `(tactic| simp only [LeanerIR.Proofs.Obligation_iff, Nat.reduceAdd, Int.reducePow,
    Int.reduceSub, Nat.reducePow, Nat.reduceSub, lir_denote_norm])
  let { ctx, simprocs, .. } ← mkSimpContext stx (eraseLocal := false)
  let mut hypotheses : SimpTheorems := {}
  for decl in ← getLCtx do
    if decl.isImplementationDetail || targets.contains decl.fvarId then continue
    let type ← instantiateMVars decl.type
    unless ← isProp type do continue
    if ← selfReferential type then continue
    hypotheses ← hypotheses.add (.fvar decl.fvarId) #[] decl.toExpr
    if let some (_, lhs, _) := type.eq? then
      if lhs.isAppOfArity ``GetElem.getElem 8 && (lhs.getArg! 0).isAppOfArity ``Array 1 then
        let lookup ← mkAppM ``Array.getElem?_eq_getElem #[lhs.getArg! 7]
        let proof ← mkEqTrans lookup
          (← mkCongrArg (← mkAppOptM ``Option.some #[lhs.getArg! 2]) decl.toExpr)
        hypotheses ← hypotheses.add (.fvar decl.fvarId) #[] proof
  let ctx ← Simp.mkContext { ctx.config with failIfUnchanged } (ctx.simpTheorems.push hypotheses)
    (withoutBranchConditions ctx.congrTheorems)
  match ← simpGoal goal ctx simprocs (simplifyTarget := simplifyTarget) (fvarIdsToSimp := targets) with
  | (none, _) => return none
  | (some (_, goal), _) => return some goal

elab "leaner_denote_simp_by_context" : tactic => do
  match ← simpByContext (← getMainGoal) #[] true true with
  | none => replaceMainGoal []
  | some goal => replaceMainGoal [goal]

/-- Rewrite the goal once by an instance of a quantified equation (or
equivalence) among the hypotheses, in either direction, at a subterm the
equation's side matches, its premises proved by a hypothesis or omega; the
rewritten goal must then be decided by the context. Each hypothesis and
direction is tried once, so the search is bounded, unlike rewriting with
the equations as rules, which may cycle. -/
elab "leaner_denote_rewrite_instance" : tactic => do
  let initial ← saveState
  let goal ← getMainGoal
  let candidates ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let ty ← instantiateMVars decl.type
      return if ty.isForall then found.push decl.toExpr else found
  let closeByContext ← `(tactic| first
    | done
    | leaner_denote_assumption
    | omega
    | (leaner_denote_simp_by_context
       first | done | leaner_denote_assumption | omega))
  for candidate in candidates.reverse do
    for symm in [false, true] do
      let saved ← saveState
      try
        let (proof, rewritten) ← goal.withContext do
          let (arguments, _, conclusion) ← forallMetaTelescopeReducing (← inferType candidate)
          let (lhs, rhs) ← match conclusion.eq?, conclusion.iff? with
            | some (_, lhs, rhs), _ => pure (lhs, rhs)
            | _, some (lhs, rhs) => pure (lhs, rhs)
            | _, _ => throwError "not an equation"
          let side := if symm then rhs else lhs
          let target ← instantiateMVars (← goal.getType)
          let abstracted ← kabstract target side
          unless abstracted.hasLooseBVars do throwError "no instance in the goal"
          -- The equation's own arguments are fixed by the match; its
          -- premises are proved.
          for argument in arguments do
            let argument ← instantiateMVars argument
            let .mvar id := argument | continue
            if ← id.isAssigned then continue
            let premise ← instantiateMVars (← id.getType)
            unless ← isProp premise do throwError "an argument is not determined"
            setGoals [id]
            evalTactic (← `(tactic| first | leaner_denote_assumption | omega))
          let proof ← instantiateMVars (mkAppN candidate arguments)
          unless (← instantiateMVars (← inferType proof)).hasMVar == false do
            throwError "the instance is not determined"
          let result ← goal.rewrite target proof symm
          let rewritten ← goal.replaceTargetEq result.eNew result.eqProof
          pure (proof, ← pure rewritten)
        let _ := proof
        setGoals [rewritten]
        evalTactic closeByContext
        if (← getGoals).isEmpty then return
      catch _ => pure ()
      saved.restore
  initial.restore
  throwError "no quantified equation rewrites the goal to a decided one"

/-- The equations that pin a position to the bound of a range when the
range's premise fails and omega knows the position is within one of it:
`¬ x < y` pins `x` to `y`, and `¬ x ≤ y` pins `x` to `y + 1`, each read
with either side as the position. -/
private def boundEquations (premise : Lean.Expr) : MetaM (Array (Lean.Expr × Lean.Expr)) := do
  let premise ← whnfR premise
  match_expr premise with
  | LT.lt _ _ x y => return #[(x, y), (y, x)]
  | LE.le _ _ x y => do
      let one ← mkNumeral (← inferType x) 1
      return #[(x, ← mkAppM ``HAdd.hAdd #[y, one]), (y, ← mkAppM ``HSub.hSub #[x, one])]
  | _ => return #[]

/-- A goal at a position one past the range of a quantified hypothesis,
the step that extends a range invariant by one iteration: the hypothesis
states the goal for the positions its range premises admit, and one
premise, a comparison, is not proved at the goal's position. The goal
splits on that premise: where it holds, the goal is the hypothesis's
instance; where it fails, omega pins the position to the range's bound,
and the goal there is decided by the context. -/
elab "leaner_denote_range_instance" : tactic => do
  let initial ← saveState
  let (goal, _) ← introduceThroughMarkers (← getMainGoal)
  let candidates ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let ty ← instantiateMVars decl.type
      return if ty.isForall then found.push decl.toExpr else found
  -- The context at the bound is normalized first: the facts there come
  -- from the iteration, such as a tested condition, as they were stated.
  let closeByContext ← `(tactic| first
    | leaner_denote_assumption
    | omega
    | ((try simp only [lir_denote_norm] at *)
       leaner_denote_simp_by_context
       first | done | omega | leaner_denote_assumption))
  for candidate in candidates.reverse do
    let saved ← saveState
    try
      let premises ← withReducible (goal.apply candidate)
      let mut failing : Array MVarId := #[]
      for premise in premises do
        if ← premise.isAssigned then continue
        setGoals [premise]
        try evalTactic (← `(tactic| first | leaner_denote_assumption | omega))
        catch _ => failing := failing.push premise
      let #[openPremise] := failing | throwError "not one open premise"
      let premise ← instantiateMVars (← openPremise.getType)
      if premise.hasExprMVar then throwError "the open premise is not determined"
      saved.restore
      let equations ← goal.withContext (boundEquations premise)
      if equations.isEmpty then throwError "the open premise is not a comparison"
      let (holds, fails) ← goal.byCases premise
      -- Where the premise holds, the goal is the hypothesis's instance.
      let premises ← holds.mvarId.withContext (withReducible (holds.mvarId.apply candidate))
      setGoals premises
      evalTactic (← `(tactic| all_goals first | leaner_denote_assumption | omega))
      unless (← getGoals).isEmpty do throwError "the instance's premises are open"
      -- Where it fails, the position is the bound.
      let mut closed := false
      for (position, bound) in equations do
        let .fvar positionId := position | continue
        let attempt ← saveState
        try
          let equation ← fails.mvarId.withContext (mkEq position bound)
          let proof ← fails.mvarId.withContext do
            let proofGoal ← mkFreshExprMVar equation
            setGoals [proofGoal.mvarId!]
            evalTactic (← `(tactic| omega))
            instantiateMVars proofGoal
          let (pinned, atBound) ← (← fails.mvarId.assert `pinned equation proof).intro1P
          let atBound ← subst atBound pinned
          let _ := positionId
          setGoals [atBound]
          try evalTactic closeByContext
          catch failure =>
            if leaner.denoteDebug.get (← getOptions) then
              IO.println s!"  range at the bound not decided: {← failure.toMessageData.toString}\n{← ppGoal (← getMainGoal)}"
            throw failure
          if (← getGoals).isEmpty then
            closed := true
            break
        catch _ => pure ()
        attempt.restore
      if closed then
        setGoals []
        return
    catch _ => pure ()
    saved.restore
  initial.restore
  throwError "no quantified hypothesis has the goal as an instance at or one past its range"

/-- The writes a lookup may read past. -/
private inductive WriteKind where
  | set | push | erase | insert

/-- A lookup into a written array: the array, the written and the read
position, both as integers, and the kind of write. -/
private structure WrittenLookup where
  array : Lean.Expr
  written : Lean.Expr
  read : Lean.Expr
  kind : WriteKind

/-- A lookup in a term into an array with an element written, removed, or
inserted. -/
private def writtenLookup? (target : Lean.Expr) : MetaM (Option WrittenLookup) := do
  let some lookup := target.find? fun e =>
      let array? := if e.isAppOfArity ``GetElem?.getElem? 7 then some (e.getArg! 5)
        else if e.isAppOfArity ``GetElem.getElem 8 then some (e.getArg! 5) else none
      array?.any fun array => array.isAppOfArity ``Array.setIfInBounds 4 ||
        array.isAppOfArity ``Array.push 3 || array.isAppOfArity ``Array.eraseIdx 4 ||
        array.isAppOfArity ``Array.insertIdx 5
    | return none
  let written := lookup.getArg! 5
  let read ← intPosition (lookup.getArg! 6)
  let array := written.getArg! 1
  if written.isAppOfArity ``Array.push 3 then
    let size ← mkAppOptM ``Nat.cast #[mkConst ``Int, none, ← mkAppM ``Array.size #[array]]
    return some { array, written := size, read, kind := .push }
  let kind := if written.isAppOfArity ``Array.setIfInBounds 4 then WriteKind.set
    else if written.isAppOfArity ``Array.eraseIdx 4 then .erase else .insert
  return some { array, written := ← intPosition (written.getArg! 2), read, kind }

/-- The variable a position is an offset of, and its value at a given
position: `x`, `x + t`, `t + x`, and `x - t`. -/
private def positionVariable? (position : Lean.Expr) :
    Option (FVarId × (Lean.Expr → MetaM Lean.Expr)) :=
  match position with
  | .fvar x => some (x, pure)
  | _ =>
    match_expr position with
    | HAdd.hAdd _ _ _ _ a b =>
        if let .fvar x := a then some (x, fun at_ => mkAppM ``HSub.hSub #[at_, b])
        else if let .fvar x := b then some (x, fun at_ => mkAppM ``HSub.hSub #[at_, a])
        else none
    | HSub.hSub _ _ _ _ a b =>
        if let .fvar x := a then some (x, fun at_ => mkAppM ``HAdd.hAdd #[at_, b]) else none
    | _ => none

/-- Whether the context already states or refutes a proposition, at
reducible transparency. -/
private def contextDecides (goal : MVarId) (proposition : Lean.Expr) : MetaM Bool :=
  goal.withContext do
    (← getLCtx).anyM fun decl => do
      if decl.isImplementationDetail then return false
      let ty ← instantiateMVars decl.type
      let stated := if ty.isAppOfArity ``Not 1 then ty.getArg! 0 else ty
      withReducible (isDefEq stated proposition)

/-- Read a lookup into a written array whose position the context does not
tell apart from the written one: the goal splits on the positions being
equal, or for a removal on the read position being before the removed one.
Where a written position is read, the variable the read position offsets
is pinned to the written one and the lookup reads the written element;
elsewhere it reads the array before the write, at the position that is
there. Fails when nothing reads a written array, or the context already
tells the positions apart. -/
elab "leaner_denote_split_write" : tactic => do
  let goal ← getMainGoal
  let isWrite (e : Lean.Expr) := (e.find? fun e =>
    e.isConstOf ``Array.setIfInBounds || e.isConstOf ``Array.push ||
    e.isConstOf ``Array.eraseIdx || e.isConstOf ``Array.insertIdx).isSome
  let found ← goal.withContext do
    let mut found ← writtenLookup? (← instantiateMVars (← goal.getType))
    for fvarId in ← goal.getNondepPropHyps do
      if found.isSome then break
      let type ← instantiateMVars (← fvarId.getType)
      if isWrite type then found ← writtenLookup? type
    pure found
  let some lookup := found | throwError "the goal reads no written array"
  -- The readers of a write, in the branch at hand.
  let readers (call : Lean.TSyntax `tactic) : TacticM Unit := do
    let goal ← getMainGoal
    simpMainAt call #[] (← hypothesesWhere goal isWrite) true
  let before ← goal.withContext (mkAppM ``LT.lt #[lookup.read, lookup.written])
  let equation ← goal.withContext (mkEq lookup.read lookup.written)
  match lookup.kind with
  | .erase =>
      if ← contextDecides goal before then throwError "the positions are already told apart"
      let (first, second) ← goal.byCases before `written_position
      setGoals [first.mvarId]
      readers (← `(tactic| simp (disch := omega) only
        [LeanerIR.Proofs.Denote.getElem?_eraseIdx_toNat_of_lt]))
      let firstGoals ← getGoals
      setGoals [second.mvarId]
      readers (← `(tactic| simp (disch := omega) only
        [LeanerIR.Proofs.Denote.getElem?_eraseIdx_toNat_of_ge]))
      setGoals (firstGoals ++ (← getGoals))
  | .set | .push | .insert =>
  if ← contextDecides goal equation then throwError "the positions are already told apart"
  let (same, apart) ← goal.byCases equation `written_position
  -- Where the positions are the same.
  let mut sameGoal := same.mvarId
  let mut pinned := false
  if let some (pinnedVariable, valueAt) := positionVariable? lookup.read then
    let attempt ← saveState
    try
      let value ← sameGoal.withContext (valueAt lookup.written)
      let pin ← sameGoal.withContext (mkEq (mkFVar pinnedVariable) value)
      let proof ← sameGoal.withContext do
        let proofGoal ← mkFreshExprMVar pin
        decideByOmega proofGoal.mvarId!
        instantiateMVars proofGoal
      let (pinEquation, next) ← (← sameGoal.assert `pinned pin proof).intro1P
      sameGoal ← subst next pinEquation
      pinned := true
    catch _ => attempt.restore
  setGoals [sameGoal]
  -- Pinned, the positions coincide; otherwise the equation rewrites the read one.
  let call ← `(tactic| simp (disch := omega) only [Int.add_sub_cancel, Int.sub_add_cancel,
    Int.toNat_natCast, Array.getElem?_setIfInBounds_self_of_lt, Array.getElem?_setIfInBounds_ne,
    Array.getElem_setIfInBounds_self, Array.getElem_setIfInBounds_ne, Array.getElem?_push_size,
    LeanerIR.Proofs.Denote.getElem?_push_of_lt, Array.getElem?_insertIdx_self, lir_denote_norm])
  if pinned then readers call
  else
    let equationFVar := same.fvarId
    simpMainAt call #[equationFVar] (← hypothesesWhere sameGoal isWrite) true
  let sameGoals ← getGoals
  -- Where they are apart.
  match lookup.kind with
  | .insert =>
      -- Before or past the inserted position.
      let (first, second) ← apart.mvarId.byCases before `written_side
      setGoals [first.mvarId]
      readers (← `(tactic| simp (disch := omega) only
        [LeanerIR.Proofs.Denote.getElem?_insertIdx_toNat_of_lt]))
      let firstGoals ← getGoals
      setGoals [second.mvarId]
      readers (← `(tactic| simp (disch := omega) only
        [LeanerIR.Proofs.Denote.getElem?_insertIdx_toNat_of_gt]))
      setGoals (sameGoals ++ firstGoals ++ (← getGoals))
  | .set | .push | .erase =>
      setGoals [apart.mvarId]
      readers (← `(tactic| simp (disch := omega) only [Array.getElem?_setIfInBounds_ne,
        Array.getElem_setIfInBounds_ne, LeanerIR.Proofs.Denote.getElem?_push_of_lt]))
      setGoals (sameGoals ++ (← getGoals))

/-- The closed array lookups in terms. -/
private def arrayLookups (terms : Array Lean.Expr) : Array Lean.Expr :=
  sitesWhere (fun e => e.isAppOfArity ``GetElem?.getElem? 7 && !e.hasLooseBVars &&
    (e.getArg! 0).isAppOfArity ``Array 1) terms

/-- Name the element at each lookup in the goal and the hypotheses whose
position the context proves in bounds and whose result it states nothing
about: the lookup reads `some` of the element, and a fact about the lookup
becomes one about the element. Fails when no lookup is named. -/
elab "leaner_denote_name_lookups" : tactic => do
  let mut goal ← getMainGoal
  let lookups ← goal.withContext do
    let hypotheses ← (← goal.getNondepPropHyps).mapM fun fvarId => do
      instantiateMVars (← fvarId.getType)
    pure (arrayLookups (#[← instantiateMVars (← goal.getType)] ++ hypotheses))
  let mut named := false
  for lookup in lookups do
    let array := lookup.getArg! 5
    let index := lookup.getArg! 6
    let element := lookup.getArg! 2
    -- A lookup the context already reads is not named again.
    let stated ← goal.withContext do
      (← goal.getNondepPropHyps).anyM fun fvarId => do
        let type ← instantiateMVars (← fvarId.getType)
        return type.isAppOfArity ``Eq 3 && type.getArg! 1 == lookup &&
          (type.getArg! 2).isAppOfArity ``Option.some 2
    if stated then continue
    -- A lookup the context reads at a position omega equates reads what
    -- that one reads.
    let equated ← goal.withContext do
      for fvarId in ← goal.getNondepPropHyps do
        let type ← instantiateMVars (← fvarId.getType)
        let some (_, read, value) := type.eq? | continue
        unless value.isAppOfArity ``Option.some 2 && read.isAppOfArity ``GetElem?.getElem? 7 &&
            read.getAppFn == lookup.getAppFn &&
            read.getAppArgs.extract 0 6 == lookup.getAppArgs.extract 0 6 do continue
        let position := read.getArg! 6
        let attempt ← saveState
        try
          let equal ← mkFreshExprMVar (← mkEq index position)
          decideByOmega equal.mvarId!
          let at_ := mkLambda `position .default (← inferType index)
            (mkAppN lookup.getAppFn ((lookup.getAppArgs.extract 0 6).push (.bvar 0)))
          let reads ← mkEqTrans (← mkCongrArg at_ (← instantiateMVars equal)) (mkFVar fvarId)
          return some reads
        catch _ => attempt.restore
      return none
    if let some reads := equated then
      let (fact, next) ← goal.withContext do
        let (fact, next) ← (← goal.assert `lookup (← inferType reads) reads).intro1P
        pure (fact, next)
      let mentioning ← hypothesesWhere next fun type => (type.find? (· == lookup)).isSome
      setGoals [next]
      simpMainAt (← `(tactic| simp only [Option.map_some, Option.getD_some, lir_denote_norm]))
        #[fact] mentioning true
      if (← getGoals).isEmpty then return
      goal ← getMainGoal
      named := true
      continue
    let attempt ← saveState
    try
      let (fact, next) ← goal.withContext do
        let inBounds ← mkFreshExprMVar (← mkAppM ``LT.lt #[index, ← mkAppM ``Array.size #[array]])
        decideByOmega inBounds.mvarId!
        let read ← mkAppOptM ``Array.getElem?_eq_getElem #[none, array, index, ← instantiateMVars inBounds]
        let statement ← withLocalDeclD `element element fun v => do
          mkLambdaFVars #[v] (← mkEq lookup (← mkAppM ``Option.some #[v]))
        let witness := ((← inferType read).getArg! 2).appArg!
        let existential ← mkAppOptM ``Exists.intro #[element, statement, witness, read]
        let (fact, next) ← (← goal.assert `named (← mkAppM ``Exists #[statement]) existential).intro1P
        pure (fact, next)
      let subgoals ← next.cases fact
      let some subgoal := subgoals[0]? | throwError "naming an element gives one case"
      let some elementVar := subgoal.fields[0]? | throwError "a named element"
      let some factVar := subgoal.fields[1]? | throwError "a named lookup"
      goal ← subgoal.mvarId.rename elementVar.fvarId! (← mkFreshUserName `element)
      goal ← goal.rename factVar.fvarId! (← mkFreshUserName `lookup)
      let mentioning ← hypothesesWhere goal fun type => (type.find? (· == lookup)).isSome
      setGoals [goal]
      simpMainAt (← `(tactic| simp only [Option.map_some, Option.getD_some, lir_denote_norm]))
        #[factVar.fvarId!] mentioning true
      goal ← getMainGoal
      named := true
    catch ex =>
      if leaner.denoteDebug.get (← getOptions) then
        IO.println s!"naming failed: {← ex.toMessageData.toString}"
      attempt.restore
  unless named do throwError "no lookup at a position in bounds to name"
  replaceMainGoal [goal]

/-- A quantified hypothesis applied to its telescope, the premises left
undecided bound as the premises of the instance. -/
private partial def bindOpenPremises (candidate : Lean.Expr) (mvars : Array Lean.Expr) (i : Nat)
    (arguments : Array Lean.Expr) : MetaM Lean.Expr := do
  if i = mvars.size then return ← instantiateMVars (mkAppN candidate arguments)
  let m := mvars[i]!
  if ← m.mvarId!.isAssigned then
    return ← bindOpenPremises candidate mvars (i + 1) (arguments.push (← instantiateMVars m))
  withLocalDeclD `premise (← instantiateMVars (← m.mvarId!.getType)) fun h => do
    m.mvarId!.assign h
    mkLambdaFVars #[h] (← bindOpenPremises candidate mvars (i + 1) (arguments.push h))

/-- Instantiate each hypothesis quantified over integer positions that reads
an array at every combination of the positions the context reads arrays
at, its other premises decided by a hypothesis or by omega, and the
instances rewritten by the context. Fails when nothing is instantiated. -/
elab "leaner_denote_instantiate_positions" : tactic => do
  let mut goal ← getMainGoal
  let (positions, candidates) ← goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    let mut candidates := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      unless ← isProp ty do continue
      if ty.isForall then
        if (ty.find? fun e => e.isAppOfArity ``GetElem?.getElem? 7).isSome then
          candidates := candidates.push decl.toExpr
      else terms := terms.push ty
    let mut positions : Array Lean.Expr := #[]
    for lookup in arrayLookups terms do
      let position ← intPosition (lookup.getArg! 6)
      unless positions.contains position do positions := positions.push position
    pure (positions, candidates)
  let debug := leaner.denoteDebug.get (← getOptions)
  if debug then
    IO.println s!"positions: {positions.size} at {candidates.size} quantified hypotheses"
  if positions.isEmpty || candidates.isEmpty then throwError "no position to instantiate at"
  let mut instances : Array FVarId := #[]
  let started ← IO.getNumHeartbeats
  let mut omegaCost := 0
  -- The facts a premise or an instance is compared against, by statement.
  let facts : Std.HashMap Lean.Expr FVarId ← goal.withContext do
    (← goal.getNondepPropHyps).foldlM (init := {}) fun facts fvarId => do
      let type ← instantiateMVars (← fvarId.getType)
      return if facts.contains type then facts else facts.insert type fvarId
  let mut stated : Std.HashSet Lean.Expr := facts.fold (init := {}) fun stated type _ =>
    stated.insert type
  for candidate in candidates do
    let ty ← goal.withContext (inferType candidate)
    let slots ← goal.withContext do
      let (mvars, _, _) ← forallMetaTelescope ty
      mvars.filterM fun m => do return (← whnfR (← m.mvarId!.getType)).isConstOf ``Int
    if slots.isEmpty then continue
    let total := positions.size ^ slots.size
    if debug then IO.println s!"  {slots.size} position binders, {total} instances"
    if total > 64 then continue
    for tuple in [0:total] do
      let saved ← saveState
      try
        let (mvars, _, conclusion) ← goal.withContext (forallMetaTelescope ty)
        let mut rest := tuple
        for m in mvars do
          let binder ← goal.withContext (whnfR (← instantiateMVars (← m.mvarId!.getType)))
          if binder.isConstOf ``Int then
            m.mvarId!.assign positions[rest % positions.size]!
            rest := rest / positions.size
        -- A premise a hypothesis or omega decides is closed; the others,
        -- such as the equation that triggers the instance, stay premises.
        for m in mvars do
          if ← m.mvarId!.isAssigned then continue
          let cost ← goal.withContext do
            let premise ← instantiateMVars (← m.mvarId!.getType)
            unless ← isProp premise do throwError "a binder besides a position"
            match facts[premise]? with
            | some fvarId => m.mvarId!.assign (mkFVar fvarId); pure 0
            | none =>
                let before ← IO.getNumHeartbeats
                try decideByOmega m.mvarId! catch _ => pure ()
                pure ((← IO.getNumHeartbeats) - before)
          omegaCost := omegaCost + cost
        let proof ← goal.withContext (bindOpenPremises candidate mvars 0 #[])
        let statement ← goal.withContext do instantiateMVars (← inferType proof)
        if stated.contains statement then throwError "an instance the context states"
        stated := stated.insert statement
        let (instance_, next) ← (← goal.assert `instance statement proof).intro1P
        goal := next
        instances := instances.push instance_
      catch ex =>
        if leaner.denoteDebug.get (← getOptions) then
          IO.println s!"instance skipped: {← ex.toMessageData.toString}"
        saved.restore
  let asserted ← IO.getNumHeartbeats
  if debug then
    IO.println s!"  {instances.size} instances asserted in {(asserted - started) / 1000}k, omega {omegaCost / 1000}k"
  if instances.isEmpty then throwError "no instance at a read position"
  setGoals [goal]
  let rewritten ← simpByContext goal instances false false
  if debug then
    IO.println s!"  instances rewritten in {((← IO.getNumHeartbeats) - asserted) / 1000}k, goal {if rewritten.isNone then "closed" else "open"}"
  match rewritten with
  | none => replaceMainGoal []
  | some rewritten => replaceMainGoal [rewritten]

/-- Split the first disjunctive hypothesis into its cases. -/
elab "leaner_denote_split_disjunction" : tactic => do
  let goal ← getMainGoal
  let disjunction ← goal.withContext do
    (← getLCtx).findDeclM? fun decl => do
      if decl.isImplementationDetail then return none
      let ty ← instantiateMVars decl.type
      return if ty.isAppOfArity ``Or 2 then some decl.fvarId else none
  let some fvarId := disjunction | throwError "no disjunctive hypothesis"
  replaceMainGoal ((← goal.cases fvarId).map (·.mvarId)).toList

/-- Decide a branch of a written position: as an instance of the context,
a refuted hypothesis, in each case of another written position, or with
the elements at positions in bounds named. -/
syntax "leaner_denote_decide_split" : tactic

macro_rules
  | `(tactic| leaner_denote_decide_split) => `(tactic|
  first
  | done
  | leaner_denote_timed "s-omega" omega
  | leaner_denote_timed "s-instance" leaner_denote_instance
  | leaner_denote_timed "s-range" leaner_denote_range_instance
  | leaner_denote_timed "s-rewrite" leaner_denote_rewrite_instance
  | leaner_denote_timed "s-refute" (exfalso; leaner_denote_instance)
  | (leaner_denote_timed "s-split" leaner_denote_split_write
     all_goals leaner_denote_decide_split)
  | (leaner_denote_timed "s-name" leaner_denote_name_lookups
     leaner_denote_intro_negation
     leaner_denote_subst_vars
     first
     | done
     | omega
     | leaner_denote_instance
     | (exfalso; leaner_denote_instance))
  | (leaner_denote_timed "s-positions" (leaner_denote_intro_negation; leaner_denote_instantiate_positions)
     first
     | done
     | leaner_denote_timed "s-positions-omega" omega
     | leaner_denote_instance))

/-- The general leaf: one pipeline, each stage over the previous stage's
goals, so that a rewriting pass is never repeated for a later alternative. -/
macro "leaner_denote_pipeline" : tactic => `(tactic| (
  leaner_denote_subst_vars
  try leaner_denote_split_search
  all_goals leaner_denote_reduce_projections
  all_goals leaner_denote_bounded (try simp (disch := omega) only [LeanerIR.Proofs.Obligation_iff,
    Nat.reduceAdd, Int.reducePow, Int.reduceSub, Nat.reducePow, Nat.reduceSub,
    Int.tmod_eq_emod_of_nonneg, Int.tdiv_eq_ediv_of_nonneg, lir_denote_norm] at *)
  all_goals first
  | done
  | leaner_denote_assumption
  | omega
  | (leaner_denote_saturate_round
     all_goals (first
       | done
       | (leaner_denote_bounds; omega)
       -- An instance of a quantified hypothesis once the lookups after
       -- writes are read, before the context is saturated.
       | leaner_denote_instance
       | leaner_denote_range_instance
       | leaner_denote_witness
       -- A lookup at a position the context does not tell apart from a
       -- written one, in each case.
       | (leaner_denote_split_write
          all_goals leaner_denote_decide_split)
       | (leaner_denote_saturate
          leaner_denote_inheriting
            (all_goals (try (leaner_denote_split <;> (try leaner_simp_all [lir_denote_norm]))))
          leaner_denote_inheriting
            (all_goals (try (leaner_denote_split <;> (try leaner_simp_all [lir_denote_norm]))))
          leaner_denote_inheriting (all_goals leaner_denote_split_goal)
          leaner_denote_saturate_round
          all_goals (first | (leaner_denote_bounds; omega) | leaner_denote_bv))))))

/-- A value the arithmetic rules named, with its definition, if a hypothesis
defines one: `named.val = e` for a local of type `SpecInt` itself. -/
private def namedDefinition? (decl : Lean.LocalDecl) : MetaM (Option (FVarId × Lean.Expr)) := do
  if decl.isImplementationDetail then return none
  let type ← instantiateMVars decl.type
  let some (_, lhs, rhs) := type.eq? | return none
  unless lhs.isAppOfArity ``LeanerIR.SpecInt.val 3 do return none
  let .fvar result := lhs.appArg! | return none
  unless (← instantiateMVars (← result.getType)).isAppOfArity ``LeanerIR.SpecInt 2 do
    return none
  if rhs.containsFVar result then return none
  return some (result, rhs)

/-- The named values of a goal's context, with their definitions. -/
private def namedDefinitions (goal : MVarId) : MetaM (Array (FVarId × Lean.Expr)) :=
  goal.withContext do (← getLCtx).decls.toArray.filterMapM fun
    | some decl => namedDefinition? decl
    | none => pure none

/-- A term with every named value it reads replaced by its definition, read
the same way; names are defined by earlier ones only. -/
private partial def unnamed (definitions : Array (FVarId × Lean.Expr)) (e : Lean.Expr) :
    Lean.Expr :=
  e.replace fun sub =>
    if sub.isAppOfArity ``LeanerIR.SpecInt.val 3 then
      match sub.appArg! with
      | .fvar x => (definitions.find? (·.1 == x)).map fun (_, d) => unnamed definitions d
      | _ => none
    else none

/-- The positions arrays are read or written at in terms. -/
private def arrayPositions (terms : Array Lean.Expr) : Array Lean.Expr :=
  let reads (e : Lean.Expr) := e.isAppOfArity ``GetElem?.getElem? 7 ||
    e.isAppOfArity ``GetElem.getElem 8
  sitesWhere (fun e => reads e || e.isAppOfArity ``Array.setIfInBounds 4 ||
      e.isAppOfArity ``Array.eraseIdx 4 || e.isAppOfArity ``Array.insertIdx 5 ||
      e.isAppOfArity ``Array.eraseIdxIfInBounds 3 || e.isAppOfArity ``Array.insertIdxIfInBounds 4)
    terms |>.map fun site => site.getArg! (if reads site then 6 else 2)

/-- Array positions by value: a position reading a named value is replaced
by the position its definition gives, where the arithmetic context proves
them equal, so a write at a named position and a read at the computed one,
or a clause's position, are compared as the same term. -/
elab "leaner_denote_unname_positions" : tactic => do
  let goal ← getMainGoal
  let definitions ← namedDefinitions goal
  if definitions.isEmpty then throwError "no named value"
  let positions ← goal.withContext do
    let hypotheses ← (← goal.getNondepPropHyps).mapM fun fvarId => do
      instantiateMVars (← fvarId.getType)
    return arrayPositions (#[← instantiateMVars (← goal.getType)] ++ hypotheses)
  let mut equated := goal
  let mut rules := #[]
  let mut seen : Array Lean.Expr := #[]
  for position in positions do
    if seen.contains position then continue
    seen := seen.push position
    unless definitions.any (position.containsFVar ·.1) do continue
    let replaced := unnamed definitions position
    if replaced == position then continue
    let attempt ← saveState
    try
      let (rule, next) ← equated.withContext do
        let equation ← mkEq position replaced
        let proof ← mkFreshExprMVar equation
        decideByOmega proof.mvarId!
        let (rule, next) ← (← equated.assert `position equation (← instantiateMVars proof)).intro1P
        pure (rule, next)
      equated := next
      rules := rules.push rule
    catch _ => attempt.restore
  if rules.isEmpty then throwError "no named position"
  let targets ← equated.withContext do
    return (← equated.getNondepPropHyps).filter (!rules.contains ·)
  setGoals [equated]
  match ← simpAt equated (← `(tactic| simp only [])) rules targets true with
  | none => replaceMainGoal []
  | some next => replaceMainGoal [← next.tryClearMany rules]

/-- A residual leaf in source form: each value the arithmetic rules named
(`named.val = e`, a local of type `SpecInt` itself, where parameters and
results have carrier types) replaced by what it names, earliest first, as an
authored proof states it. -/
elab "leaner_denote_unname" : tactic => do
  let mut goal ← getMainGoal
  repeat
    let named ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        return (← namedDefinition? decl).map fun (result, _) => (decl.fvarId, result)
    let some (equation, result) := named | break
    let targets ← goal.withContext do
      return (← goal.getNondepPropHyps).filter (· != equation)
    setGoals [goal]
    match ← simpAt goal (← `(tactic| simp only [])) #[equation] targets true with
    | none => replaceMainGoal []; return
    | some next => goal ← (← next.clear equation).tryClear result
  replaceMainGoal [goal]

/-- The deciders of a leaf an authored proof may take over: the cheap ones,
and the goal alone in normal form rewritten by the hypotheses, which is
cheap where the context is large and decides a leaf whose facts are
already in the context. -/
macro "leaner_denote_decide_residual" : tactic => `(tactic|
  first
  | done
  | omega
  | leaner_denote_decide_cheap
  | leaner_denote_instance
  | leaner_denote_range_instance
  | (leaner_denote_simp_by_context
     first | done | (leaner_denote_bounds; omega))
  | leaner_denote_trivial)

macro_rules
  | `(tactic| leaner_denote_decide_by_context) => `(tactic|
  (leaner_denote_simp_by_context
   first | done | (leaner_denote_bounds; omega)))

/-- The deciders over a prepared leaf: the goal as a decided instance of the
context, with the lookups after writes read, in each case of a position not
told apart from a written one. -/
syntax "leaner_denote_decide_written" : tactic

macro_rules
  | `(tactic| leaner_denote_decide_written) => `(tactic|
  first
  | done
  | leaner_denote_timed "w-omega" omega
  | leaner_denote_timed "w-cheap" leaner_denote_decide_cheap
  | leaner_denote_timed "w-instance" leaner_denote_instance
  | leaner_denote_timed "w-refute" (exfalso; leaner_denote_instance)
  | leaner_denote_timed "w-range" leaner_denote_range_instance
  | leaner_denote_timed "w-context" leaner_denote_decide_by_context
  | leaner_denote_timed "w-trivial" leaner_denote_trivial
  -- A conjunction, such as a clause with an abort code, conjunct by
  -- conjunct.
  | (apply And.intro <;> leaner_denote_decide_written)
  | (leaner_denote_timed "w-lookups" leaner_denote_lookups_after_writes
     first
     | done
     | leaner_denote_timed "wl-omega" omega
     | leaner_denote_timed "wl-instance" leaner_denote_instance
     | leaner_denote_timed "wl-range" leaner_denote_range_instance)
  | (leaner_denote_timed "w-split" leaner_denote_split_write
     all_goals leaner_denote_timed "ws" leaner_denote_decide_split)
  | (leaner_denote_timed "w-positions" (leaner_denote_intro_negation; leaner_denote_instantiate_positions)
     first
     | done
     | omega
     | leaner_denote_instance))

/-- Whether the prepared deciders can decide the leaf at all: they decide
instances of hypotheses quantified over integer positions and lookups
after writes, so a leaf with neither (a storage leaf, whose quantifiers
range over keys) is left to the pipeline, whose normalization would only
be paid twice. -/
elab "leaner_denote_prepared_applies" : tactic => do
  let goal ← getMainGoal
  let applies ← goal.withContext do
    let isWrite (e : Lean.Expr) : Bool :=
      e.isConstOf ``Array.setIfInBounds || e.isConstOf ``LeanerIR.SpecVector.set ||
      e.isConstOf ``Array.eraseIdxIfInBounds || e.isConstOf ``Array.insertIdxIfInBounds
    if ((← instantiateMVars (← goal.getType)).find? isWrite).isSome then return true
    (← getLCtx).anyM fun decl => do
      if decl.isImplementationDetail then return false
      let ty ← instantiateMVars decl.type
      if ty.isForall && (← isProp ty) then
        if (← whnfR ty.bindingDomain!).isConstOf ``Int then return true
      return (ty.find? isWrite).isSome
  unless applies do
    throwError "the leaf has no hypothesis quantified over positions and no written vector"

/-- Under `leaner.denoteDebug`, print the goal and the propositional
hypotheses of every open goal, each on one truncated line. -/
elab "leaner_denote_trace_state " label:str : tactic => do
  unless leaner.denoteDebug.get (← getOptions) do return
  for goal in ← getGoals do
    goal.withContext do
      let line (e : Lean.Expr) : MetaM String := do
        return ((toString (← ppExpr e)).replace "\n" " " |>.take 220).toString
      IO.println s!"    {label.getString} ⊢ {← line (← instantiateMVars (← goal.getType))}"
      for fvarId in ← goal.getNondepPropHyps do
        let decl ← fvarId.getDecl
        IO.println s!"      {decl.userName} : {← line (← instantiateMVars decl.type)}"

/-- Decide a leaf in the normal form an authored proof receives: the
normalization is one pass over the context, where the saturation of the
pipeline rewrites the hypotheses with each other for rounds. -/
macro "leaner_denote_decide_prepared" : tactic => `(tactic| (
  leaner_denote_prepared_applies
  leaner_denote_timed "intro" leaner_denote_intro_any
  leaner_denote_timed "prepare" leaner_denote_prepare
  all_goals (try leaner_denote_timed "positions" leaner_denote_unname_positions)
  all_goals (first
    | leaner_denote_timed "written" leaner_denote_decide_written
    | (leaner_denote_trace_state "undecided prepared"; fail "the prepared leaf is undecided"))))

/-- Decide one leaf cheaply, without rewriting its context. -/
macro "leaner_denote_leaf_cheap" : tactic => `(tactic|
  (leaner_denote_canonical_families
   leaner_denote_clear_computations
   first
   | leaner_denote_assumption
   | (leaner_denote_split_hypotheses
      leaner_denote_decide_residual)
   | leaner_denote_budgeted
      (leaner_denote_intro
       leaner_denote_split_hypotheses
       leaner_denote_decide_residual)))

/-- Decide one leaf. -/
syntax "leaner_denote_leaf" : tactic

macro_rules
  | `(tactic| leaner_denote_leaf) => `(tactic|
      (leaner_denote_canonical_families
       leaner_denote_timed "clear" leaner_denote_clear_computations
       first
       -- A goal a hypothesis states, before splitting takes it apart.
       | leaner_denote_timed "assumption" leaner_denote_assumption
       -- Atomic hypotheses first: each is rewritten, used, and dropped on
       -- its own. A goal whose binders a loop step introduced is an
       -- instance of a quantified hypothesis as much as a quantified one.
       | (leaner_denote_timed "split" leaner_denote_split_hypotheses
          leaner_denote_timed "map positions" leaner_denote_map_positions
          first
          | leaner_denote_timed "cheap" leaner_denote_decide_cheap
          | leaner_denote_timed "instance" leaner_denote_instance
          | leaner_denote_timed "range" leaner_denote_range_instance
          | leaner_denote_timed "prepared" leaner_denote_decide_prepared
          | leaner_denote_timed "pipeline" leaner_denote_pipeline
          -- A disjunctive hypothesis, case by case.
          | (leaner_denote_timed "cases" leaner_denote_split_disjunction
             all_goals leaner_denote_leaf)
          | leaner_denote_trivial)
       -- A quantified or conditional goal, at an arbitrary instance.
       | leaner_denote_budgeted
          (leaner_denote_intro
           leaner_denote_split_hypotheses
           leaner_denote_timed "map positions3" leaner_denote_map_positions
           first
           | leaner_denote_timed "cheap3" leaner_denote_decide_cheap
           -- An instance of a quantified hypothesis, such as a loop
           -- invariant, at the position or one past its range.
           | leaner_denote_timed "instance3" leaner_denote_instance
           | leaner_denote_timed "range3" leaner_denote_range_instance
           | leaner_denote_timed "prepared3" leaner_denote_decide_prepared
           | leaner_denote_timed "pipeline3" leaner_denote_pipeline
           | leaner_denote_trivial)))

/-- A structural update of an array literal at literal positions: the
positions move, the elements are not inspected. -/
private def reduceArrayLiteral (array : Lean.Expr) (positions : Array Lean.Expr)
    (update : List Lean.Expr → Array Nat → Option (List Lean.Expr)) : MetaM Simp.DStep := do
  let some (type, elements) := array.arrayLit? | return .continue
  let some positions := positions.mapM (·.nat?) | return .continue
  let some updated := update elements positions | return .continue
  return .visit (← mkArrayLit type updated)

/-- An in-bounds write into an array literal. -/
dsimproc [lir_denote_norm] reduceSetIfInBounds (Array.setIfInBounds _ _ _) := fun e => do
  let_expr Array.setIfInBounds _ array index value := e | return .continue
  reduceArrayLiteral array #[index] fun elements positions =>
    let index := positions[0]!
    some (if index < elements.length then elements.set index value else elements)

/-- An in-bounds exchange in an array literal. -/
dsimproc [lir_denote_norm] reduceSwapIfInBounds (Array.swapIfInBounds _ _ _) := fun e => do
  let_expr Array.swapIfInBounds _ array left right := e | return .continue
  reduceArrayLiteral array #[left, right] fun elements positions =>
    match elements[positions[0]!]?, elements[positions[1]!]? with
    | some first, some second =>
        some ((elements.set positions[0]! second).set positions[1]! first)
    | _, _ => some elements

/-- A slice of an array literal, its end clamped to the size. -/
dsimproc [lir_denote_norm] reduceExtract (Array.extract _ _ _) := fun e => do
  let_expr Array.extract _ array start stop := e | return .continue
  reduceArrayLiteral array #[start, stop] fun elements positions =>
    some ((elements.take positions[1]!).drop positions[0]!)

/-- A range reversal of an array literal. -/
dsimproc [lir_denote_norm] reduceReverseRange (LeanerIR.Proofs.Denote.reverseRange _ _ _ _) :=
  fun e => do
    let_expr LeanerIR.Proofs.Denote.reverseRange _ count left right array := e | return .continue
    reduceArrayLiteral array #[count, left, right] fun elements positions => Id.run do
      let mut elements := elements
      let mut left := positions[1]!
      let mut right := positions[2]!
      for _ in [0:positions[0]!] do
        elements := match elements[left]?, elements[right]? with
          | some first, some second => (elements.set left second).set right first
          | _, _ => elements
        left := left + 1
        right := right - 1
      return some elements

/-- Normalize only a bind's action. Its continuation is the rest of the
program: the closer reaches it through a folded continuation when the action
completes, so the normalizer never traverses it while a head is processed.
An action that completed hands the bind back to the rewrite rules that
reduce it. -/
private def bindActionStep (e : Lean.Expr) : Simp.SimpM Simp.Step := do
  let arguments := e.getAppArgs
  if arguments.size < 2 then return .continue
  let action := arguments[arguments.size - 2]!
  let next := arguments[arguments.size - 1]!
  let result ← Simp.simp action
  let head := mkAppN e.getAppFn (arguments.extract 0 (arguments.size - 2))
  let expr := mkApp2 head result.expr next
  let proof? ← result.proof?.mapM fun proof => do mkCongrFun (← mkCongrArg head proof) next
  let completed := match result.expr.getAppFn with
    | .const name _ => name == ``LeanerIR.Proofs.Spec.pure || name == ``LeanerIR.Proofs.Spec.abort
    | _ => false
  if completed then return .continue (some { expr, proof? })
  return .done { expr, proof? }

simproc ↓ [lir_denote_norm] bindAction (LeanerIR.Proofs.Spec.bind _ _) := bindActionStep

simproc ↓ [lir_denote_norm] flowBindAction (@LeanerIR.Proofs.Denote.Flow.bind ?skolems _ _ _ _ _ _) :=
  bindActionStep


/-- An equation between values of a type parameter under the family a
call's type arguments induce is the equation of their encodings at the
argument's type, which the normalizer reduces to the runtime structure a
caller's clauses speak about. -/
simproc ↓ [lir_denote_norm] instantiatedEqAsEncoding (@Eq _ _ _) :=
  fun e => do
    let_expr Eq carrier left right := e | return .continue
    let some (outer, argument) ← (do
        match carrier.getAppFn.constName?, carrier.getAppArgs with
        | some ``LeanerIR.Proofs.Denote.Skolems.carrier, #[family, index] =>
            let_expr LeanerIR.Proofs.Denote.Skolems.instantiate θ outer := family | pure none
            pure (some (outer, mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.subst)
              (← mkAppM ``Subtype.val #[θ])
              (mkApp (mkConst ``LeanerIR.Proofs.Denote.NTy.param) index)))
        | some ``LeanerIR.Proofs.Denote.NTy.carrier, #[family, τ] =>
            let_expr LeanerIR.Proofs.Denote.Skolems.instantiate θ outer := family | pure none
            pure (some (outer, mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.subst)
              (← mkAppM ``Subtype.val #[θ]) τ))
        | _, _ => pure none : MetaM (Option (Lean.Expr × Lean.Expr)))
      | return .continue
    let encode (value : Lean.Expr) :=
      mkApp3 (mkConst ``LeanerIR.Proofs.Denote.NTy.encode) outer argument value
    let equation ← mkEq (encode left) (encode right)
    let injective := mkAppN (mkConst ``LeanerIR.Proofs.Denote.NTy.encode_inj) #[outer, argument, left, right]
    let proof ← mkAppM ``propext #[← mkAppM ``Iff.symm #[injective]]
    return .visit { expr := equation, proof? := some proof }

mutual
/-- Whether a native type is spelled with constructors and literal type
parameters only, so that its substitution evaluates. -/
private partial def spelledNTy (τ : Lean.Expr) : Bool :=
  match τ.getAppFn.constName?, τ.getAppArgs with
  | some ``LeanerIR.Proofs.Denote.NTy.unit, #[] | some ``LeanerIR.Proofs.Denote.NTy.bool, #[]
  | some ``LeanerIR.Proofs.Denote.NTy.address, #[] | some ``LeanerIR.Proofs.Denote.NTy.signer, #[]
  | some ``LeanerIR.Proofs.Denote.NTy.string, #[] | some ``LeanerIR.Proofs.Denote.NTy.bytes, #[] => true
  | some ``LeanerIR.Proofs.Denote.NTy.int, #[_, _] => true
  | some ``LeanerIR.Proofs.Denote.NTy.param, #[index] => index.nat?.isSome || index.rawNatLit?.isSome
  | some ``LeanerIR.Proofs.Denote.NTy.tuple, #[row] => spelledNRow row
  | some ``LeanerIR.Proofs.Denote.NTy.struct, #[_, row] => spelledNRow row
  | some ``LeanerIR.Proofs.Denote.NTy.enum, #[_, _, rows, _] => spelledNRows rows
  | some ``LeanerIR.Proofs.Denote.NTy.vector, #[element] => spelledNTy element
  | some ``LeanerIR.Proofs.Denote.NTy.ref, #[referent] => spelledNTy referent
  | _, _ => false

private partial def spelledNRow (row : Lean.Expr) : Bool :=
  match row.getAppFn.constName?, row.getAppArgs with
  | some ``LeanerIR.Proofs.Denote.NRow.nil, #[] => true
  | some ``LeanerIR.Proofs.Denote.NRow.cons, #[τ, rest] => spelledNTy τ && spelledNRow rest
  | _, _ => false

private partial def spelledNRows (rows : Lean.Expr) : Bool :=
  match rows.getAppFn.constName?, rows.getAppArgs with
  | some ``LeanerIR.Proofs.Denote.NRows.nil, #[] => true
  | some ``LeanerIR.Proofs.Denote.NRows.cons, #[row, rest] => spelledNRow row && spelledNRows rest
  | _, _ => false
end

/-- The row a type-argument transport substitutes, when it is spelled. -/
private def substitutionRow? (θ : Lean.Expr) : Option Lean.Expr :=
  if θ.isAppOfArity ``Subtype.mk 4 then some (θ.getArg! 2) else none

/-- The entry of a spelled row at a position, as `NRow.getD` reads it. -/
private partial def rowEntry? (row : Lean.Expr) (index : Nat) : Option Lean.Expr :=
  if row.isAppOfArity ``LeanerIR.Proofs.Denote.NRow.cons 2 then
    if index == 0 then some (row.getArg! 0) else rowEntry? (row.getArg! 1) (index - 1)
  else none

mutual
/-- The substitution of a spelled type by a spelled row, evaluated to a
spelling, as `NTy.subst` defines it. -/
private partial def substituteNTy (θ : Lean.Expr) (τ : Lean.Expr) : Option Lean.Expr := do
  let args := τ.getAppArgs
  match τ.getAppFn.constName? with
  | some ``LeanerIR.Proofs.Denote.NTy.param =>
      let index ← (args[0]?).bind fun index => index.nat? <|> index.rawNatLit?
      pure ((rowEntry? θ index).getD τ)
  | some ``LeanerIR.Proofs.Denote.NTy.tuple =>
      pure (mkApp τ.getAppFn (← substituteNRow θ (← args[0]?)))
  | some ``LeanerIR.Proofs.Denote.NTy.struct =>
      pure (mkApp2 τ.getAppFn (← args[0]?) (← substituteNRow θ (← args[1]?)))
  | some ``LeanerIR.Proofs.Denote.NTy.enum =>
      pure (mkApp4 τ.getAppFn (← args[0]?) (← args[1]?) (← substituteNRows θ (← args[2]?)) (← args[3]?))
  | some ``LeanerIR.Proofs.Denote.NTy.vector | some ``LeanerIR.Proofs.Denote.NTy.ref =>
      pure (mkApp τ.getAppFn (← substituteNTy θ (← args[0]?)))
  | _ => if spelledNTy τ then some τ else none

private partial def substituteNRow (θ : Lean.Expr) (row : Lean.Expr) : Option Lean.Expr := do
  if row.isAppOfArity ``LeanerIR.Proofs.Denote.NRow.cons 2 then
    pure (mkApp2 row.getAppFn (← substituteNTy θ (row.getArg! 0)) (← substituteNRow θ (row.getArg! 1)))
  else if row.isConstOf ``LeanerIR.Proofs.Denote.NRow.nil then some row
  else none

private partial def substituteNRows (θ : Lean.Expr) (rows : Lean.Expr) : Option Lean.Expr := do
  if rows.isAppOfArity ``LeanerIR.Proofs.Denote.NRows.cons 2 then
    pure (mkApp2 rows.getAppFn (← substituteNRow θ (rows.getArg! 0))
      (← substituteNRows θ (rows.getArg! 1)))
  else if rows.isConstOf ``LeanerIR.Proofs.Denote.NRows.nil then some rows
  else none
end

/-- The row a substitution is by, as spelled. -/
private def spelledRow (row : Lean.Expr) : MetaM Lean.Expr := do
  if row.isAppOfArity ``Subtype.val 3 then
    let inner ← whnfR (row.getArg! 2)
    if let some spelled := substitutionRow? inner then return spelled
  return row

/-- A substitution by a literal row, evaluated by the given evaluator or,
failing it, by reduction. -/
private def reduceSubst (substitute : Lean.Expr → Lean.Expr → Option Lean.Expr)
    (e row subject : Lean.Expr) : MetaM Simp.DStep := do
  if let some substituted := substitute (← spelledRow row) subject then
    if substituted != e then return .visit substituted
  let reduced ← whnfR e
  if reduced == e then return .continue
  return .visit reduced

/-- A substitution by a literal row evaluates, the whole type at once, so no
substitution is left in an argument the normalizer does not visit. -/
dsimproc ↓ [lir_denote, lir_denote_norm] reduceParameterSubst
    (LeanerIR.Proofs.Denote.NTy.subst _ _) := fun e => do
  let_expr LeanerIR.Proofs.Denote.NTy.subst row τ := e | return .continue
  reduceSubst substituteNTy e row τ

/-- The substitution of a literal row evaluates (`reduceParameterSubst`). -/
dsimproc ↓ [lir_denote, lir_denote_norm] reduceRowSubst
    (LeanerIR.Proofs.Denote.NRow.subst _ _) := fun e => do
  let_expr LeanerIR.Proofs.Denote.NRow.subst row fields := e | return .continue
  reduceSubst substituteNRow e row fields

/-- The substitution of literal variant rows evaluates (`reduceParameterSubst`). -/
dsimproc ↓ [lir_denote, lir_denote_norm] reduceRowsSubst
    (LeanerIR.Proofs.Denote.NRows.subst _ _) := fun e => do
  let_expr LeanerIR.Proofs.Denote.NRows.subst row rows := e | return .continue
  reduceSubst substituteNRows e row rows

/-- A spelled type at an induced family, as the outer family spells it: the
family and the substituted type. The carrier, codec, and encoding of a type
at an induced family are those of its substitution at the outer family, by
definition, so a callee's and a caller's spellings of one value become one
term. -/
private def outerSpelling? (family τ : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  unless family.isAppOfArity ``LeanerIR.Proofs.Denote.Skolems.instantiate 2 do return none
  let some row := substitutionRow? (family.getArg! 0) | return none
  let some substituted := substituteNTy row τ | return none
  return some (family.getArg! 1, substituted)

/-- The codec of a type at an induced family (`outerSpelling?`). -/
dsimproc ↓ [lir_denote, lir_denote_norm] codecOuterFamily
    (@LeanerIR.Proofs.Denote.NTy.codec ?skolems _) := fun e => do
  let_expr LeanerIR.Proofs.Denote.NTy.codec family τ := e | return .continue
  let some (outer, substituted) ← outerSpelling? family τ | return .continue
  return .visit (mkApp2 e.getAppFn outer substituted)

/-- A term with every family-dependent spelling of a spelled type at an
induced family canonical: the carrier, row, codec, and encoding of a type at
a family a call's type arguments induce are, by definition, those of its
substitution at the outer family, so that a callee's view of a value and
the caller's read alike, also in the implicit arguments rewriting does not
reach. -/
private def canonicalFamilies (e : Lean.Expr) : MetaM Lean.Expr := do
  unless (e.find? (·.isConstOf ``LeanerIR.Proofs.Denote.Skolems.instantiate)).isSome do return e
  Meta.transform e (post := fun t => do
    let args := t.getAppArgs
    let some head := t.getAppFn.constName? | return .continue
    let induced (family : Lean.Expr) : Option (Lean.Expr × Lean.Expr) :=
      if family.isAppOfArity ``LeanerIR.Proofs.Denote.Skolems.instantiate 2 then
        (substitutionRow? (family.getArg! 0)).map (family.getArg! 1, ·)
      else none
    let rebuilt : Option Lean.Expr := do
      let family ← args[0]?
      let (outer, row) ← induced family
      if head == ``LeanerIR.Proofs.Denote.NTy.carrier || head == ``LeanerIR.Proofs.Denote.NTy.codec then
        let τ ← args[1]?
        let τ' ← substituteNTy row τ
        pure (mkAppN t.getAppFn (#[outer, τ'] ++ args.extract 2 args.size))
      else if head == ``LeanerIR.Proofs.Denote.HList || head == ``LeanerIR.Proofs.Denote.rowCodec then
        let fields ← args[1]?
        let fields' ← substituteNRow row fields
        pure (mkAppN t.getAppFn (#[outer, fields'] ++ args.extract 2 args.size))
      else if head == ``LeanerIR.Proofs.Denote.variantCarrier then
        let names ← args[1]?
        let rows ← args[2]?
        let rows' ← substituteNRows row rows
        pure (mkAppN t.getAppFn (#[outer, names, rows'] ++ args.extract 3 args.size))
      else none
    match rebuilt with
    | some t' => return .done t'
    | none => return .continue)

/-- Make the family spellings of a goal and its hypotheses canonical
(`canonicalFamilies`), by definitional replacement. -/
elab_rules : tactic
  | `(tactic| leaner_denote_canonical_families) => do
  let goals ← getGoals
  let mut result := #[]
  for goal in goals do
    if ← goal.isAssigned then continue
    let mut goal := goal
    let target ← instantiateMVars (← goal.getType)
    let canonical ← canonicalFamilies target
    if canonical != target then goal ← goal.replaceTargetDefEq canonical
    for decl in ← goal.withContext getLCtx do
      if decl.isImplementationDetail then continue
      let type ← instantiateMVars decl.type
      let canonical ← goal.withContext (canonicalFamilies type)
      if canonical != type then goal ← goal.replaceLocalDeclDefEq decl.fvarId canonical
    result := result.push goal
  setGoals result.toList

/-- A codec's encoding function of a type at an induced family, with its
carrier, as the outer family spells both (`outerSpelling?`), so that a
callee's and a caller's mapped encodings are one term. -/
dsimproc ↓ [lir_denote, lir_denote_norm] codecEncodeOuterFamily
    (@LeanerIR.Proofs.Codec.encode _ _ _) := fun e => do
  let_expr LeanerIR.Proofs.Codec.encode _ target codec := e | return .continue
  let_expr LeanerIR.Proofs.Denote.NTy.codec family τ := codec | return .continue
  let some (outer, substituted) ← outerSpelling? family τ | return .continue
  let carrier := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.carrier) outer substituted
  return .visit (mkApp3 e.getAppFn carrier target (mkApp2 codec.getAppFn outer substituted))

/-- Whether a term rebuilds a value from its components: the value itself,
or a pair of its first component and a rebuild of its second, down to the
unit a row ends with. -/
private partial def rebuilds (term value : Lean.Expr) : Bool :=
  term == value ||
    (term.isAppOfArity ``Prod.mk 4 &&
      (let first := term.getArg! 2
       first.isAppOfArity ``Prod.fst 3 && first.getArg! 2 == value &&
         (let rest := term.getArg! 3
          let tail := mkApp3 (mkConst ``Prod.snd first.getAppFn.constLevels!) (first.getArg! 0)
            (first.getArg! 1) value
          rebuilds rest tail ||
            ((rest.isConstOf ``Unit.unit || rest.isAppOfArity ``PUnit.unit 0) &&
              let rowType := first.getArg! 1
              rowType.isConstOf ``Unit || rowType.isAppOfArity ``PUnit 0 ||
                (rowType.isAppOfArity ``LeanerIR.Proofs.Denote.HList 2 &&
                  (rowType.getArg! 1).isConstOf ``LeanerIR.Proofs.Denote.NRow.nil)))))

/-- Mapping the identity over an array is the array: a function that
returns its argument, or rebuilds it component by component as the
transport of a row whose types need no substitution does, also where the
element type is spelled differently on the two sides. -/
simproc [lir_denote, lir_denote_norm] mapIdentity (Array.map _ _) := fun e => do
  let_expr Array.map element _ function array := e | return .continue
  let .lam name domain body info := function | return .continue
  unless rebuilds body (.bvar 0) do return .continue
  let levels := [e.getAppFn.constLevels!.head!]
  let identity := mkApp (mkConst ``Array.map_id'' levels) element
  let proof ← withLocalDecl name info domain fun x => do
    let reflexive ← mkEqRefl x
    let pointwise ← mkLambdaFVars #[x] reflexive
    return mkApp3 identity function pointwise array
  let proof ← mkExpectedTypeHint proof (← mkEq e array)
  return .done { expr := array, proof? := some proof }

/-- A caller's value in the callee's view, as far as projections reach into
a transported row or reference: the caller's family, the transport, whether
the value is a row, its type (or row type) in the callee, and the caller's
value. -/
private partial def peelTransport? (value : Lean.Expr) :
    MetaM (Option (Lean.Expr × Lean.Expr × Bool × Lean.Expr × Lean.Expr)) := do
  if value.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.toSkolem 4 then
    return some (value.getArg! 0, value.getArg! 1, false, value.getArg! 2, value.getArg! 3)
  if value.isAppOfArity ``LeanerIR.Proofs.Denote.HList.toSkolem 4 then
    return some (value.getArg! 0, value.getArg! 1, true, value.getArg! 2, value.getArg! 3)
  let first := value.isAppOfArity ``Prod.fst 3
  unless first || value.isAppOfArity ``Prod.snd 3 do return none
  -- A projection of a pair already built.
  let pair := value.getArg! 2
  if pair.isAppOfArity ``Prod.mk 4 then
    return ← peelTransport? (pair.getArg! (if first then 2 else 3))
  let some (family, θ, isRow, ty, v) ← peelTransport? pair | return none
  -- A signature's row may be its published constant.
  let ty ← whnfD ty
  let projected ← mkAppM (if first then ``Prod.fst else ``Prod.snd) #[v]
  if isRow then
    unless ty.isAppOfArity ``LeanerIR.Proofs.Denote.NRow.cons 2 do return none
    return if first then some (family, θ, false, ty.getArg! 0, projected)
      else some (family, θ, true, ty.getArg! 1, projected)
  unless ty.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.ref 1 do return none
  return some (family, θ, false, ty.getArg! 0, projected)

/-- A caller's value in the callee's view: the caller's family, the
transport, the callee's type, and the caller's value. A transported row or
variant is the value of the tuple, struct, or enum type the encoding names,
over the callee's rows. -/
private def componentTransport? (value τ : Lean.Expr) :
    MetaM (Option (Lean.Expr × Lean.Expr × Lean.Expr × Lean.Expr)) := do
  let τ ← whnfR τ
  if value.isAppOfArity ``LeanerIR.Proofs.Denote.variantCarrier.toSkolem 5 then
    unless τ.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.enum 4 do return none
    return some (value.getArg! 0, value.getArg! 1,
      mkApp4 τ.getAppFn (τ.getArg! 0) (value.getArg! 2) (value.getArg! 3) (τ.getArg! 3),
      value.getArg! 4)
  let some (family, θ, isRow, ty, v) ← peelTransport? value | return none
  unless isRow do return some (family, θ, ty, v)
  if τ.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.struct 2 then
    return some (family, θ, mkApp2 τ.getAppFn (τ.getArg! 0) ty, v)
  if τ.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.tuple 1 then
    return some (family, θ, mkApp τ.getAppFn ty, v)
  return none

/-- A callee's value in the caller's view encodes as it does at the callee's
family (`NTy.encode_ofSkolem`), so a callee's contract about the encoding
speaks about the caller's value. The caller's type is the callee's under the
substitution, which the normalizer has already reduced, so the callee's type
is read off the transport and the caller's type together. -/
simproc ↓ [lir_denote, lir_denote_norm] encodeTransport
    (@LeanerIR.Proofs.Denote.NTy.encode ?skolems _ _) :=
  fun e => do
    let_expr LeanerIR.Proofs.Denote.NTy.encode outer callerType value := e | return .continue
    -- A caller's value in the callee's view encodes as it does in the caller's,
    -- also a component of a transported row, which a call's argument is. The
    -- encoding may be spelled at the induced family and the callee's type, or
    -- at the outer family and the substituted type.
    if let some (family, θ, calleeType, v) ←
        (try componentTransport? value callerType catch _ => pure none) then
      let some substituted := (substitutionRow? θ).bind (substituteNTy · calleeType)
        | return .continue
      let proof := mkAppN (mkConst ``LeanerIR.Proofs.Denote.NTy.encode_toSkolem)
        #[family, θ, calleeType, v]
      let encoded := mkApp3 e.getAppFn family substituted v
      let proof ← mkExpectedTypeHint proof (← mkEq e encoded)
      return .visit { expr := encoded, proof? := some proof }
    let callee? : Option (Lean.Expr × Lean.Expr × Lean.Expr) ← do
      match value.getAppFn.constName?, value.getAppArgs with
      | some ``LeanerIR.Proofs.Denote.NTy.ofSkolem, #[_, θ, τ, v] => pure (some (θ, τ, v))
      | some ``LeanerIR.Proofs.Denote.HList.ofSkolem, #[_, θ, row, v] =>
          match (← whnfR callerType).getAppFnArgs with
          | (``LeanerIR.Proofs.Denote.NTy.tuple, #[_]) =>
              pure (some (θ, mkApp (mkConst ``LeanerIR.Proofs.Denote.NTy.tuple) row, v))
          | (``LeanerIR.Proofs.Denote.NTy.struct, #[source, _]) =>
              pure (some (θ, mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.struct) source row, v))
          | _ => pure none
      | some ``LeanerIR.Proofs.Denote.variantCarrier.ofSkolem, #[_, θ, names, rows, v] =>
          match (← whnfR callerType).getAppFnArgs with
          | (``LeanerIR.Proofs.Denote.NTy.enum, #[source, _, _, distinct]) =>
              pure (some (θ, mkApp4 (mkConst ``LeanerIR.Proofs.Denote.NTy.enum) source names rows
                distinct, v))
          | _ => pure none
      | _, _ => pure none
    let some (θ, τ, v) := callee? | return .continue
    let proof := mkAppN (mkConst ``LeanerIR.Proofs.Denote.NTy.encode_ofSkolem) #[outer, θ, τ, v]
    let some (_, _, encoded) := (← inferType proof).eq? | return .continue
    return .visit { expr := encoded, proof? := some proof }

/-- A callee's value in the caller's view holds the variant it holds at the
callee's family (`variantName_ofSkolem`). -/
simproc ↓ [lir_denote, lir_denote_norm] variantNameTransport
    (@LeanerIR.Proofs.Denote.variantName ?skolems _ _ _) :=
  fun e => do
    let_expr LeanerIR.Proofs.Denote.variantName outer _ _ value := e | return .continue
    let_expr LeanerIR.Proofs.Denote.variantCarrier.ofSkolem _ θ names rows v := value
      | return .continue
    let proof := mkAppN (mkConst ``LeanerIR.Proofs.Denote.variantName_ofSkolem)
      #[outer, θ, names, rows, v]
    let some (_, _, name) := (← inferType proof).eq? | return .continue
    return .visit { expr := name, proof? := some proof }

/-- Decoding a literal integer at a literal width is decided outright: the
certificate is a kernel decision, so no conditional is left for a split. -/
simproc ↓ [lir_denote_norm] decodeIntegerLiteral
    (LeanerIR.Proofs.Codec.decode? (LeanerIR.Proofs.Codec.specInt _ _) (LeanerIR.RuntimeValue.integer _)) :=
  fun e => do
    unless e.isAppOfArity ``LeanerIR.Proofs.Codec.decode? 4 do return .continue
    let codec := e.getArg! 2
    let raw := e.getArg! 3
    unless codec.isAppOfArity ``LeanerIR.Proofs.Codec.specInt 2 do return .continue
    unless raw.isAppOfArity ``LeanerIR.RuntimeValue.integer 1 do return .continue
    let width := codec.getArg! 0
    let signed := codec.getArg! 1
    let value := raw.getArg! 0
    unless width.isAppOfArity ``LeanerIR.IntWidth.bits 1 && (width.getArg! 0).nat?.isSome do
      return .continue
    unless signed.isConstOf ``Bool.true || signed.isConstOf ``Bool.false do return .continue
    let some _ := value.int? | return .continue
    let fits ← mkAppM ``LeanerIR.IntegerValueFits #[width, signed, value]
    -- The equation is closed and decidable: its certificate is a decision.
    let fitsProof? ← try some <$> mkDecideProof fits catch _ => pure none
    let result ← match fitsProof? with
      | some proof =>
          mkAppM ``Option.some #[mkAppN (mkConst ``LeanerIR.SpecInt.mk) #[width, signed, value, proof]]
      | none => mkAppOptM ``Option.none #[← mkAppM ``LeanerIR.SpecInt #[width, signed]]
    let equation ← mkEq e result
    let proof? ← try some <$> mkDecideProof equation catch _ => pure none
    let some proof := proof? | return .continue
    return .done { expr := result, proof? := some proof }

/- The normalization of a verification condition: the denotation's rules,
the weakest-precondition rules, and the propositional normal forms a leaf
is decided in. -/
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.NTy.encode_enum_inl
  LeanerIR.Proofs.Denote.NTy.encode_enum_inr LeanerIR.Proofs.Denote.variantName_inl
  LeanerIR.Proofs.Denote.variantName_inr LeanerIR.Proofs.Denote.HList.encode_cons
  LeanerIR.Proofs.Denote.HList.encode_nil LeanerIR.Proofs.Denote.NTy.encode_int
attribute [lir_denote_norm] LeanerIR.Proofs.wp_choose LeanerIR.Proofs.wp_assume
-- A clause's Boolean literal, decided, is the runtime's; a Boolean's truth,
-- decided, is the Boolean.
attribute [lir_denote_norm] decide_false decide_true Bool.decide_eq_true Bool.decide_eq_false
-- A clause's encoded vectors: equal to another encoding exactly when the
-- arrays are, equal to a literal exactly when the literal decodes to the
-- values.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.map_encode_eq_map_encode_iff
  LeanerIR.Proofs.Denote.map_specInt_encode_eq_toArray_iff
  LeanerIR.Proofs.Denote.toArray_eq_map_specInt_encode_iff
  LeanerIR.Proofs.Denote.map_bool_encode_eq_toArray_iff
  LeanerIR.Proofs.Denote.map_address_encode_eq_toArray_iff
  LeanerIR.Proofs.Denote.map_codec_encode_eq_toArray_iff
  LeanerIR.Proofs.Denote.toArray_eq_map_codec_encode_iff

/-- A quantifier over a range with literal bounds is its instances, one per
position, when the range is short (at most 64 positions): the first position
is split off until the range is empty, each step decided by the kernel. -/
simproc [lir_denote_norm] expandLiteralRange (∀ _ : Int, _ ≤ _ → _ < _ → _) :=
  fun e => do
    let .forallE binder (.const ``Int []) body _ := e | return .continue
    let .forallE _ lowerBound rest _ := body | return .continue
    let .forallE _ upperBound predicate _ := rest | return .continue
    -- The premises' proofs are unused by the clause.
    if predicate.hasLooseBVar 0 || predicate.hasLooseBVar 1 then return .continue
    let_expr LE.le _ _ low lowVariable := lowerBound | return .continue
    let_expr LT.lt _ _ highVariable high := upperBound | return .continue
    unless lowVariable == .bvar 0 && highVariable == .bvar 1 do return .continue
    let some lowValue := low.int? | return .continue
    let some highValue := high.int? | return .continue
    unless lowValue < highValue && highValue ≤ lowValue + 64 do return .continue
    let intType : Lean.Expr := .const ``Int []
    -- The clause with the range's variable free at `#0`, and as a predicate.
    let clause := predicate.lowerLooseBVars 2 2
    let clauseAt (position : Lean.Expr) : Lean.Expr := clause.instantiate1 position
    let asPredicate : Lean.Expr := .lam binder intType clause .default
    -- From the last position back: `(∀ i, k ≤ i → i < high → P i) ↔ conjunction`.
    let mut conjunction : Lean.Expr := mkConst ``True
    let mut proof ← mkAppOptM ``LeanerIR.Proofs.Denote.forall_int_range_empty
      #[asPredicate, high, high, ← mkDecideProof (← mkAppM ``LE.le #[high, high])]
    let mut position := highValue
    while position > lowValue do
      position := position - 1
      let here : Lean.Expr := toExpr position
      let short ← mkAppM ``LE.le #[high, ← mkAppM ``HAdd.hAdd #[here, (toExpr (64 : Int) : Lean.Expr)]]
      let split ← mkAppOptM ``LeanerIR.Proofs.Denote.forall_int_range_split
        #[asPredicate, here, high, ← mkDecideProof (← mkAppM ``LT.lt #[here, high]),
          ← mkDecideProof short]
      let restCongruent ← mkAppM ``and_congr #[← mkAppM ``Iff.refl #[clauseAt here], proof]
      proof ← mkAppM ``Iff.trans #[split, restCongruent]
      conjunction := mkAnd (clauseAt here) conjunction
    return .visit { expr := conjunction, proof? := some (← mkAppM ``propext #[proof]) }
-- A literal instantiation keys a family as the runtime does.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.instantiatedTypeId_cons
  LeanerIR.Proofs.Denote.instantiatedTypeId_nil LeanerIR.TypeId.mk.injEq
-- A type parameter's value is encoded by its family's codec, and defaults
-- under an induced family to its argument's value.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.Skolems.default_instantiate
  LeanerIR.Proofs.Denote.NTy.inhabitant LeanerIR.Proofs.Denote.HList.inhabitant
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.NTy.codec_param
  LeanerIR.Proofs.Denote.NTy.encode_param LeanerIR.Proofs.Denote.Skolems.codec_instantiate
-- The type arguments of an opaque specification function, at an
-- instantiated family the caller's, at the public family themselves.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.Skolems.type_instantiate
  LeanerIR.Proofs.Denote.Skolems.type_runtime LeanerIR.Proofs.Denote.NTy.substWith
  LeanerIR.Proofs.Denote.NRow.substWith LeanerIR.Proofs.Denote.NRows.substWith
attribute [lir_denote_norm high] LeanerIR.Proofs.Denote.NTy.substWith_type_runtime
-- A callee's value in the caller's view reduces constructor by constructor.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.NTy.ofSkolem_unit
  LeanerIR.Proofs.Denote.NTy.ofSkolem_bool LeanerIR.Proofs.Denote.NTy.ofSkolem_int
  LeanerIR.Proofs.Denote.NTy.ofSkolem_address LeanerIR.Proofs.Denote.NTy.ofSkolem_signer
  LeanerIR.Proofs.Denote.NTy.ofSkolem_string LeanerIR.Proofs.Denote.NTy.ofSkolem_bytes
  LeanerIR.Proofs.Denote.NTy.ofSkolem_param LeanerIR.Proofs.Denote.NTy.ofSkolem_tuple
  LeanerIR.Proofs.Denote.NTy.ofSkolem_struct LeanerIR.Proofs.Denote.NTy.ofSkolem_enum
  LeanerIR.Proofs.Denote.NTy.ofSkolem_vector LeanerIR.Proofs.Denote.NTy.ofSkolem_ref
  LeanerIR.Proofs.Denote.HList.ofSkolem_nil LeanerIR.Proofs.Denote.HList.ofSkolem_cons
  LeanerIR.Proofs.Denote.variantCarrier.ofSkolem_inl
  LeanerIR.Proofs.Denote.variantCarrier.ofSkolem_inr
  LeanerIR.Proofs.Denote.ResultShape.ofSkolem_none LeanerIR.Proofs.Denote.ResultShape.ofSkolem_one
-- A function calling itself, or a member of a cycle of calls: the calls
-- routed to the cycle are `self`, every other call its callee's meaning.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.routeMeaning_self
  LeanerIR.Proofs.Denote.routeMeaning_other LeanerIR.Proofs.Denote.routeGeneric_self
  LeanerIR.Proofs.Denote.routeGeneric_other LeanerIR.FunctionHandle.mk.injEq
  LeanerIR.NamespaceId.mk.injEq LeanerIR.FunctionId.mk.injEq
-- The structural order, as the ordering it computes.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.compareResult_val LeanerIR.orderValue_lt_zero
  LeanerIR.orderValue_eq_zero LeanerIR.zero_lt_orderValue LeanerIR.orderValue_le_zero
  LeanerIR.zero_le_orderValue LeanerIR.orderValue_eq_neg_one LeanerIR.orderValue_eq_one
  LeanerIR.RuntimeValue.order_integer LeanerIR.RuntimeValue.order_bool Int.compare_eq_lt
  Int.compare_eq_gt Int.compare_eq_eq
-- A search over a whole vector, as membership.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.getD_map_field
-- A vector construction whose representability is decided, and the sizes
-- of in-bounds insertion and removal. Whether an insertion or removal is
-- in bounds is an arithmetic side condition, decided at a leaf by omega
-- (`leaner_denote_lookups_after_writes`), not by the normalization, whose
-- default discharger would run it over the whole set.
attribute [lir_denote_norm] Option.dite_none_right_eq_some
  Array.size_insertIdx Array.size_eraseIdx
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.findIndex?_eq_none_iff
  LeanerIR.Proofs.Denote.findIndex?_isSome_iff LeanerIR.Proofs.Denote.forall_imp_ne_iff
  LeanerIR.Proofs.Denote.forall_imp_ne_iff' LeanerIR.Proofs.Denote.exists_some_val_eq_iff
  LeanerIR.Proofs.Denote.forall_some_val_ne_iff LeanerIR.Proofs.Denote.findIndex?_zero
  LeanerIR.Proofs.Denote.findIndex?_succ LeanerIR.Proofs.Denote.NTy.eqb_int
  LeanerIR.Proofs.Denote.NTy.eqb_bool LeanerIR.Proofs.Denote.NTy.eqb_address
  LeanerIR.Proofs.Denote.NTy.eqb_unit LeanerIR.Proofs.Denote.NTy.eqb_param
-- A wrapping shift whose value cannot reach the modulus.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.shiftLeft_emod_of_fits
  LeanerIR.Proofs.Denote.shiftLeft_tmod_of_fits
-- A resolved prophecy brings an operation's result into a leaf.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.wrapInt_unsigned
  LeanerIR.Proofs.Denote.wrapInt_signed LeanerIR.Proofs.Denote.ModularOp.run_val
  LeanerIR.Proofs.Denote.BitOp.run_val
attribute [lir_denote_norm] LeanerIR.Proofs.wp_pure
  LeanerIR.Proofs.wp_abort LeanerIR.Proofs.Spec.pure_bind LeanerIR.Proofs.Spec.abort_bind
  LeanerIR.Proofs.Denote.ResultShape.bodyType Bool.not_eq_true decide_eq_true_eq Bool.and_eq_true
  Bool.or_eq_true and_assoc exists_and_left exists_and_right exists_eq_left exists_eq_left'
  and_true true_and and_imp forall_and forall_eq forall_eq' Prod.mk.injEq exists_eq exists_eq'
  and_false false_and not_false_eq_true not_true_eq_false true_implies false_implies implies_true
  imp_self eq_self_iff_true ite_true ite_false Bool.false_eq_true Bool.true_eq_false
  Bool.not_eq_false decide_eq_false_iff_not Bool.and_eq_false_imp Bool.not_true Bool.not_false
  Bool.not_not Bool.not_eq_false' Bool.not_eq_true' Option.some.injEq Option.elim_some Option.elim_none LeanerIR.RuntimeValue.field
  LeanerIR.RuntimeValue.asInt LeanerIR.RuntimeValue.asBool LeanerIR.RuntimeValue.asString
  LeanerIR.RuntimeValue.variant? List.getElem?_toArray List.getElem?_cons_zero
  List.getElem?_cons_succ Option.getD_some LeanerIR.RuntimeValue.nominal.injEq
  LeanerIR.RuntimeValue.tuple.injEq LeanerIR.RuntimeValue.integer.injEq
  LeanerIR.RuntimeValue.bool.injEq LeanerIR.RuntimeValue.address.injEq List.cons.injEq List.nil_eq
  beq_self_eq_true LeanerIR.Proofs.Denote.Array.push_eq_push_iff
  LeanerIR.Proofs.Denote.specInt_encode LeanerIR.Proofs.Denote.bool_encode
  LeanerIR.Proofs.Denote.address_encode Nat.lt_succ_self Nat.lt_add_one Nat.le_refl
  Option.map_some Option.map_none LeanerIR.Proofs.Denote.specInt_decode_integer
  LeanerIR.Proofs.Denote.bool_decode_bool LeanerIR.Proofs.Denote.address_decode_address
  LeanerIR.Proofs.Denote.signer_decode_signer LeanerIR.Proofs.Denote.NTy.codec_int
  LeanerIR.Proofs.Denote.NTy.codec_bool LeanerIR.Proofs.Denote.NTy.codec_address
  LeanerIR.Proofs.Denote.NTy.codec_unit LeanerIR.Proofs.Denote.nat_eq_add_succ_iff
  LeanerIR.Proofs.Denote.nat_add_succ_eq_iff LeanerIR.Proofs.Denote.nat_eq_succ_iff
  LeanerIR.Proofs.Denote.nat_succ_eq_iff LeanerIR.Proofs.Denote.Family.key_eq
  LeanerIR.Proofs.Denote.NTy.codec_encode Option.bind_some Option.bind_none
  LeanerIR.FamilyRepresentation Option.getD_none LeanerIR.Proofs.Denote.HList.encode_inj
  LeanerIR.Proofs.Denote.NTy.decode?_struct_literal
  LeanerIR.Proofs.Denote.NTy.decode?_struct_nominal LeanerIR.Proofs.Denote.rowCodec_decode?_cons
  LeanerIR.Proofs.Denote.rowCodec_decode?_nil LeanerIR.Proofs.Denote.NTy.decode?_tuple_literal
  LeanerIR.Proofs.Denote.NTy.encode_inj LeanerIR.Proofs.Denote.NTy.decode?_encode
  LeanerIR.Proofs.Denote.storageKey_address LeanerIR.Proofs.Denote.storageKey_signer
  LeanerIR.SemanticOperations.globalKey LeanerIR.GlobalMap.lookup_insert_self
  LeanerIR.GlobalMap.lookup_erase_self LeanerIR.SemanticOperations.instantiatedTypeId_empty
  Nat.add_assoc LeanerIR.Proofs.Denote.nat_self_eq_add_iff
  LeanerIR.Proofs.Denote.nat_add_eq_self_iff LeanerIR.Proofs.Denote.NTy.codec_vector
  LeanerIR.Proofs.Denote.decodeElements?_nil LeanerIR.Proofs.Denote.decodeElements?_cons
  LeanerIR.Proofs.Denote.decodeElements?_map_encode
  LeanerIR.Proofs.Denote.boundedVector_decode?_map_encode
  LeanerIR.Proofs.Denote.boundedVector_decode?_vector LeanerIR.Proofs.Codec.boundedVector_encode
  LeanerIR.SpecVector.ofArray?_eq LeanerIR.SpecVector.values_set LeanerIR.SpecVector.values_mk
  Array.toList_map LeanerIR.Proofs.Denote.exists_mem_map_iff Array.mem_toList_iff
  exists_eq_right exists_eq_right' LeanerIR.Proofs.Denote.exists_range_eq_iff
  LeanerIR.Proofs.Denote.mem_iff_exists_int_index
  LeanerIR.Proofs.Denote.not_exists_range_iff LeanerIR.Proofs.Denote.not_forall_range_iff
  LeanerIR.Proofs.Denote.forall_exists_range_iff
  Option.isSome_eq_false_iff Option.isNone_iff_eq_none true_iff false_iff iff_true iff_false
  LeanerIR.Proofs.Denote.not_eq_none_iff_isSome
  LeanerIR.decodeBool?_bool LeanerIR.decodeString?_string LeanerIR.decodeAddress?_address
  LeanerIR.decodeSigner?_signer LeanerIR.decodeBytes?_bytes LeanerIR.decodeInt?_val
  Int.toNat_natCast Array.toArray_toList List.toList_toArray Array.length_toList Array.size_set!
  Array.size_push Array.size_map LeanerIR.Proofs.Denote.toArray_inj_iff
  LeanerIR.RuntimeValue.vector.injEq List.map_toArray List.map_cons List.map_nil List.push_toArray
  List.size_toArray List.length_cons List.length_nil List.getElem?_eq_some_iff
  Array.getElem?_eq_none_iff Array.size_setIfInBounds Array.getElem?_map
  -- The transport of a value at the identity instantiation.
  Option.map_id' Array.map_id' Int.sub_add_cancel
  Option.map_map Function.comp_def Array.set!_eq_setIfInBounds List.setIfInBounds_toArray
  List.set_cons_zero List.set_cons_succ List.insertIdxIfInBounds_toArray List.insertIdx_zero
  List.insertIdx_succ_cons List.eraseIdxIfInBounds_toArray List.eraseIdx_cons_zero
  List.eraseIdx_cons_succ List.toList_toArray Option.bind_eq_some_iff Option.map_eq_some_iff
  Int.toNat_natCast LeanerIR.Proofs.Denote.vectorLength_val
  LeanerIR.Proofs.Denote.NTy.encode_vector LeanerIR.Proofs.Denote.NTy.eqb_vector_decide
  LeanerIR.Proofs.Denote.NTy.refFree LeanerIR.Proofs.Denote.NRow.refFree
  LeanerIR.Proofs.Denote.NRows.refFree Option.isSome_some Option.isSome_none Option.isSome_map
  LeanerIR.GlobalKey.mk.injEq LeanerIR.StorageKey.address.injEq Option.map_eq_none_iff
  LeanerIR.StructHandle.mk.injEq LeanerIR.Proofs.Contract.typed LeanerIR.Proofs.Denote.hlistCodec
  LeanerIR.Proofs.Denote.resultCodec LeanerIR.Proofs.Denote.NTy.encode_bool
  LeanerIR.Proofs.Denote.NTy.encode_unit LeanerIR.Proofs.Denote.NTy.encode_address
  LeanerIR.Proofs.Denote.NTy.encode_tuple LeanerIR.Proofs.Denote.NTy.encode_struct
  List.toArray_eq_iff exists_prop exists_const Int.not_lt Int.not_le ne_eq Decidable.not_not
  Bool.true_or Bool.false_or Bool.or_true Bool.or_false


-- Variant names are literals, decided where they are compared.
attribute [lir_denote_norm] String.reduceBEq String.reduceEq String.reduceBNe String.reduceNe

attribute [lir_denote_eval] reduceCtorEq Nat.reduceAdd Nat.reduceSub Nat.reduceDiv Nat.reducePow
  Nat.reduceLT Nat.reduceEqDiff Int.reduceAdd Int.reduceSub Int.reducePow Int.reduceLT Int.reduceLE
  Int.reduceNatCast' Int.reduceToNat String.reduceBEq String.reduceEq String.reduceBNe
  String.reduceNe dite_true dite_false

macro "leaner_denote_normalize" location:(Lean.Parser.Tactic.location)? : tactic =>
  `(tactic| simp only [lir_denote, lir_denote_norm, lir_denote_eval, Prod.fst, Prod.snd]
    $[$location]?)

/-- Normalize the main goal's target as `leaner_denote_normalize` does, taking
the subterms in `normal` as they are. The closer keeps every goal it holds
normalized, so the parts of a goal a rule only rearranges, or the values a
continuation is applied to, are normal already; traversing them again would
cost as much as normalizing them, while only the structure the step created
changes. A target nothing rewrites is left as it is. -/
def normalizeAround (normalization : Simp.Context × Simp.SimprocsArray)
    (normal : Array Lean.Expr) : TacticM Unit := do
  let goal ← getMainGoal
  let saved ← saveState
  try
    goal.withContext do
      let (ctx, simprocs) := normalization
      let known : Std.HashSet Lean.Expr := normal.foldl (·.insert ·) {}
      let base := Simp.mkDefaultMethodsCore simprocs
      let methods := { base with
        pre := fun e => if known.contains e then return .done { expr := e } else base.pre e
        dpre := fun e => if known.contains e then return .done e else base.dpre e }
      let target ← instantiateMVars (← goal.getType)
      let (result, _) ← Simp.main target ctx (methods := methods)
      if result.expr == target then return
      if result.expr.isTrue then
        goal.assign (← match result.proof? with
          | some proof => mkOfEqTrue proof
          | none => pure (mkConst ``True.intro))
        replaceMainGoal []
      else
        replaceMainGoal [← applySimpResultToTarget goal target result]
  catch _ =>
    saved.restore

/-- The loop a goal's weakest precondition is over, if any: its site, and
the arguments of `wp_loopAt` besides the invariant. -/
private def loopGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``LeanerIR.Proofs.Denote.loopAt 6 do return none
    let some site ← (evalNat (action.getArg! 3)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- One structural split of a goal, on its syntax alone: a binder, a
conjunction, or a conditional.  Nothing is unfolded to find one.  Only a
conditional's branches can hold new redexes: a binder or conjunct of a
normalized goal is normalized already. -/
private def splitOnce (goal : MVarId) : TacticM (Option (List MVarId × Bool)) := do
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  if target.isForall then
    -- A quantified variable keeps its name, for an authored proof.
    let (_, next) ← goal.intro1P
    return some ([next], false)
  if target.isAppOfArity ``And 2 then
    let subgoals ← goal.apply (mkConst ``And.intro)
    return some (subgoals, false)
  let saved ← saveState
  try
    setGoals [goal]
    evalTactic (← `(tactic| leaner_denote_split))
    return some (← getGoals, true)
  catch _ =>
    saved.restore
    return none

/-- Close a goal `∃ a, slot = some a ∧ P a` (or `∃ a, slot = some a`) whose
slot reduces to `some v` at reducible transparency, with `v` as the
witness; the goal `P v` is returned when there is one. -/
private def slotWitness? (goal : MVarId) (type : Lean.Expr) : MetaM (Option MVarId) := do
  unless type.isAppOfArity ``Exists 2 do return none
  let .lam _ _ body _ := type.getArg! 1 | return none
  let (equation, rest?) := if body.isAppOfArity ``And 2 then (body.getArg! 0, some (body.getArg! 1))
    else (body, none)
  unless equation.isAppOfArity ``Eq 3 do return none
  let slot := equation.getArg! 1
  let held := equation.getArg! 2
  unless !slot.hasLooseBVars && held.isAppOfArity ``Option.some 2 && held.appArg! == .bvar 0 do
    return none
  let reduced ← whnfR slot
  unless reduced.isAppOfArity ``Option.some 2 do return none
  let value := reduced.appArg!
  let equationProof ← mkExpectedTypeHint (← mkEqRefl slot) (equation.instantiate1 value)
  match rest? with
  | none =>
      goal.assign (← mkAppOptM ``Exists.intro #[type.getArg! 0, type.getArg! 1, value, equationProof])
      return none
  | some rest =>
      let restGoal ← mkFreshExprMVar (rest.instantiate1 value) (kind := .syntheticOpaque)
        (userName := ← goal.getTag)
      goal.assign (← mkAppOptM ``Exists.intro #[type.getArg! 0, type.getArg! 1, value,
        ← mkAppM ``And.intro #[equationProof, restGoal]])
      return some restGoal.mvarId!

/-- The goal applied to the loop hypothesis it is an instance of, if any:
the premises that remain. -/
private def recursiveHypothesis? (goal : MVarId) : TacticM (Option (List MVarId)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 && (target.getArg! 3).getAppFn.isFVar do
      return none
    (← getLCtx).findDeclM? fun decl => do
      if decl.isImplementationDetail then return none
      let ty ← instantiateMVars decl.type
      unless ty.isForall && ty.bindingBody!.isForall do return none
      let saved ← saveState
      try
        let subgoals ← goal.apply decl.toExpr
        if subgoals.isEmpty then
          saved.restore
          return none
        -- A value the hypothesis binds and states by an equation on the
        -- row (the value a slot holds) is what the row holds there: the
        -- equation is reflexive at reducible transparency and fixes it.
        let mut remaining := #[]
        for subgoal in subgoals do
          let subtype ← subgoal.withContext do instantiateMVars (← subgoal.getType)
          if subtype.isAppOfArity ``Eq 3 && subtype.hasExprMVar then
            try withReducible (subgoal.refl) catch _ => pure ()
          -- The premise `∃ a, slot = some a ∧ P a` over a slot the row holds
          -- a value at is that value's: `P` at it remains.
          else if let some rest ← subgoal.withContext (slotWitness? subgoal subtype) then
            remaining := remaining.push rest
            continue
          unless ← subgoal.isAssigned do remaining := remaining.push subgoal
        return some remaining.toList
      catch _ =>
        saved.restore
        return none

/-- The callee a goal's weakest precondition is over, if any, with the
action: the handle of a `propheticMeaning` application, or a callee given
as a local function, as a function calling itself is. -/
private def callGoal? (goal : MVarId) (callees : Array (Lean.Expr × String × Lean.Expr)) :
    MetaM (Option (Lean.Expr × Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    if action.isAppOfArity ``LeanerIR.Proofs.Denote.propheticMeaning 7 then
      return some (action.getArg! 3, action)
    let head := action.getAppFn
    if head.isFVar && callees.any (·.1 == head) then return some (head, action)
    return none

/-- The contract constants a callee theorem speaks about, to unfold at a
call site: the typed contract and the raw contract it wraps. -/
private def contractConstants (theoremType : Lean.Expr) : MetaM (Array Name) := do
  let type ← instantiateMVars theoremType
  let type := type.getForallBody
  unless type.isAppOfArity ``LeanerIR.Proofs.Satisfies 6 do return #[]
  let contract := type.getArg! 5
  let mut names := #[]
  if let .const name _ := contract.getAppFn then
    names := names.push name
    if let some info := (← getEnv).find? name then
      if let some value := info.value? then
        for dependency in value.getUsedConstants do
          if name.getPrefix.isPrefixOf dependency then names := names.push dependency
  return names

/-- The size of an expression as a tree, counting shared subterms at each
occurrence, up to `limit`: what a traversal that does not cache visits. -/
partial def treeSize (e : Lean.Expr) (limit : Nat) : Nat :=
  go e 0
where
  go (e : Lean.Expr) (count : Nat) : Nat :=
    if count ≥ limit then count else
    match e with
    | .app f a => go a (go f (count + 1))
    | .lam _ t b _ | .forallE _ t b _ => go b (go t (count + 1))
    | .letE _ t v b _ => go b (go v (go t (count + 1)))
    | .mdata _ b | .proj _ _ b => go b (count + 1)
    | _ => count + 1

/-- The name of a continuation — a loop's, or a bind's — bound as a local
definition while the action before it is traversed. Substitutions
re-introduce locals, so the closer recognizes it by name rather than by
identity. -/
def continuationName : Name := `leanerContinuation

/-- Whether an action can branch, so that its continuation would be reached
on several paths. -/
private def mayBranch (action : Lean.Expr) : MetaM Bool := do
  let env ← getEnv
  return (action.find? fun sub => match sub.getAppFn with
    | .const name _ =>
        name == ``ite || name == ``dite || name == ``LeanerIR.Proofs.Denote.loopAt ||
          Lean.Meta.isMatcherCore env name
    | _ => false).isSome

/-- A bind at the head of a goal, taken by its rule: the goal with the
action at the head, whether the action can branch, and what the normalizer
takes as it is: the action, the continuation the rule builds (normalized
where it is applied), the two conditions, and the state, in this order. -/
private def bindRule? (goal : MVarId) : MetaM (Option (MVarId × Bool × Array Lean.Expr)) :=
    goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
  let action := (target.getArg! 3).consumeMData.headBeta
  let conditions := #[target.getArg! 4, target.getArg! 5, target.getArg! 6]
  let rule ← if action.isAppOfArity ``LeanerIR.Proofs.Spec.bind 6 then
      mkAppM ``LeanerIR.Proofs.wp_bind (#[action.getArg! 4, action.getArg! 5] ++ conditions)
    else if action.isAppOfArity ``LeanerIR.Proofs.Denote.Flow.bind 7 then
      let inner := (action.getArg! 5).consumeMData.headBeta
      if inner.isAppOfArity ``LeanerIR.Proofs.Denote.Flow.bind 7 then
        -- Flows associate to the right, so that the goal's continuation
        -- stays out of the inner bind's arms.
        pure (mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_flowBind_flowBind)
          (action.getAppArgs.extract 0 3 ++
            #[inner.getArg! 3, inner.getArg! 4, action.getArg! 4, inner.getArg! 5,
              inner.getArg! 6, action.getArg! 6] ++ conditions))
      else if inner.isAppOfArity ``LeanerIR.Proofs.Spec.bind 6 then
        -- A flow after a bind binds inside the bind's continuation, where the
        -- goal's continuation appears once.
        pure (mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_flowBind_specBind)
          ((action.getAppArgs.extract 0 5).push (inner.getArg! 2) ++
            #[inner.getArg! 4, inner.getArg! 5, action.getArg! 6] ++ conditions))
      else
      -- The flow's own skolem family and types instantiate the rule.
      pure (mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_flowBind) (action.getAppArgs ++ conditions))
    else return none
  -- `Iff.mpr` at the rule's own sides: inferring them by unification
  -- assigns the whole goal to a metavariable, whose check walks it.
  let_expr Iff lhs rhs := ← whnfR (← inferType rule)
    | throwError "a bind rule is not an equivalence"
  let [unfolded] ← goal.apply (mkApp3 (mkConst ``Iff.mpr) lhs rhs rule)
    | throwError "the bind rule did not leave one goal"
  -- The action and the continuation the rule builds; the continuation is
  -- normalized where it is applied, to the action's result.
  let (first, continuation) ← unfolded.withContext do
    let unfoldedTarget ← instantiateMVars (← unfolded.getType)
    pure (unfoldedTarget.getArg! 3, unfoldedTarget.getArg! 4)
  return some (unfolded, ← mayBranch first, #[first, continuation] ++ conditions)

/-- Bind a goal's continuation as a local definition, which the normalizer
leaves folded until the action completes: a branching action reaches it on
several paths, and no branch renormalizes it. -/
private def foldContinuation (goal : MVarId) : MetaM (MVarId × Option FVarId) :=
    goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return (goal, none)
  let ensures := target.getArg! 4
  if ensures.isFVar then return (goal, none)
  let defined ← goal.define continuationName (← inferType ensures) ensures
  let (continuation, folded) ← defined.intro1P
  folded.withContext do
    let target ← instantiateMVars (← folded.getType)
    let folded ← folded.replaceTargetDefEq
      (mkAppN target.getAppFn (target.getAppArgs.set! 4 (mkFVar continuation)))
    return (folded, some continuation)

/-- Strip the `Obligation` markers heading a goal, returning the goal and
the clause ranges they carried, kept aside for reporting. A contract clause
is decided whole under its marker; a loop invariant is established as the
closer takes it apart, its quantifiers introduced by name for an authored
proof and its existentials left to a witness, so an invariant goal's
markers are set aside rather than sealing its clauses. -/
private partial def stripObligations (goal : MVarId) (clauses : Array ObligationRange) :
    MetaM (MVarId × Array ObligationRange) := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``LeanerIR.Proofs.Obligation 4 do return (goal, clauses)
  let clauses := match obligationRange? target with
    | some range => clauses.push range
    | none => clauses
  stripObligations (← goal.replaceTargetDefEq (target.getArg! 3)) clauses

/-- Close every goal: a loop by its invariant, a recursive-call leaf by the
loop hypothesis, a call by the callee's theorem, a structural node by
splitting, and a leaf by decision. -/
partial def closeGoals (invariants : Array (Nat × Lean.Expr × Lean.Expr))
    (callees : Array (Lean.Expr × String × Lean.Expr)) (equations : Array Lean.Expr := #[])
    (residual : Bool := false) :
    TacticM Unit := do
  assertedBounds.set {}
  normalHypotheses.set {}
  stageNormalHypotheses.set {}
  if leaner.denoteDebug.get (← getOptions) then
    IO.println s!"=== closer start {(← Lean.Elab.Term.getDeclName?).getD .anonymous}"
  -- In residual mode an undecided leaf is left for an authored proof.
  let mut residuals : Array MVarId := #[]
  -- The target's closed equations, such as its generic calls' frame
  -- instantiations, apply wherever a body brings such a term into a goal.
  let rewriteEquations (goals : List MVarId) : TacticM (List MVarId) := do
    if equations.isEmpty then return goals
    let mut rewritten := []
    for goal in goals do
      setGoals [goal]
      let lemmas ← goal.withContext <| equations.mapM fun equation => do
        `(Lean.Parser.Tactic.simpLemma| $(← Lean.Elab.Term.exprToSyntax equation):term)
      evalTactic (← `(tactic| try simp only [$lemmas,*]))
      rewritten := rewritten ++ (← getGoals)
    return rewritten
  setGoals (← rewriteEquations (← getGoals))
  -- The structural matchers read a target's head: drop the metadata an
  -- introduction may leave around it.
  setGoals (← (← getGoals).mapM fun goal => do
    let target ← instantiateMVars (← goal.getType)
    if target.isMData then goal.replaceTargetDefEq target.consumeMData else pure goal)
  -- The function's arguments and state at its start, what a loop
  -- invariant's `old` reads, from the goal's own context.
  -- A function's latest start on the path: an inlined callee's calls are
  -- sequential, and a loop runs inside the call that reached it.
  let start? (goal : MVarId) (function : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) :=
    goal.withContext do
      let mut found := none
      for declaration in ← getLCtx do
        let type ← instantiateMVars declaration.type
        if type.isAppOfArity ``FunctionStart 4 then
          if ← isDefEq (type.getArg! 1) function then
            found := some (type.getArg! 2, type.getArg! 3)
      return found
  let debug := leaner.denoteDebug.get (← getOptions)
  let startHeartbeats ← IO.getNumHeartbeats
  -- The normalization, built when a step first needs it.
  let mut normalization? : Option (Simp.Context × Simp.SimprocsArray) := none
  -- The normal conditions of each continuation folded into a local
  -- definition.
  let mut foldedNormal : Std.HashMap FVarId (Array Lean.Expr) := {}
  let mut leaves := 0
  let mut stageCost : Array (String × Nat) := #[]
  let mut reported : Array (ObligationRange × String) := #[]
  let mut pending : Array (MVarId × Option Provenance × Array ObligationRange) :=
    (← getGoals).toArray.map fun g => (g, none, #[])
  while let some (goal, provenance, clauses) := pending.back? do
    pending := pending.pop
    let (goal, clauses) ← match provenance with
      | some .loopEntry | some .loopIteration => stripObligations goal clauses
      | _ => pure (goal, clauses)
    if ← goal.isAssigned then continue
    setGoals [goal]
    let stageStart ← IO.getNumHeartbeats
    if debug then
      let head ← goal.withContext do
        let target ← instantiateMVars (← goal.getType)
        let rec heads (e : Lean.Expr) (depth : Nat) : MetaM String := do
          let e := e.consumeMData.headBeta
          let name ← match e.getAppFn with
            | .const name _ => pure (toString name)
            | .fvar id => do return s!"local {(← id.getDecl).userName}"
            | _ => pure "?"
          let name := if e.isAppOfArity ``LeanerIR.Proofs.Denote.propheticMeaning 6 then
              s!"{name} {e.getArg! 2}" else name
          if depth == 0 then return name
          let inner := if e.isAppOfArity ``LeanerIR.Proofs.wp 7 then some (e.getArg! 3)
            else if e.getAppNumArgs ≥ 1 && (e.getAppFn.isConstOf ``LeanerIR.Proofs.Spec.bind ||
                e.getAppFn.isConstOf ``LeanerIR.Proofs.Denote.Flow.bind) then
              some (e.getArg! (e.getAppNumArgs - 2))
            else none
          match inner with
          | some inner => return s!"{name} ({← heads inner (depth - 1)})"
          | none => return name
        heads target 4
      IO.println s!"→ {head}"
    -- An exit of a loop reaches its continuation, which is unfolded here, at
    -- the one place the rest of the function is needed.
    let exit? ← goal.withContext do
      let target ← instantiateMVars (← goal.getType)
      let .fvar id := target.getAppFn | pure none
      let declaration ← id.getDecl
      if declaration.isLet && declaration.userName == continuationName then
        pure (some ((← zetaDeltaFVars target #[id]).headBeta,
          target.getAppArgs ++ (foldedNormal.get? id).getD #[]))
      else pure none
    if let some (unfolded, values) := exit? then
      setGoals [← goal.replaceTargetDefEq unfolded]
      let normalization ← normalization?.getDM normalization
      normalization? := some normalization
      -- The values the continuation is applied to, and the conditions it
      -- passes on, are normal already.
      normalizeAround normalization values
      if debug then
        let head := match unfolded.getAppFn with
          | .const name _ => toString name
          | _ => "?"
        let action := if unfolded.isAppOfArity ``LeanerIR.Proofs.wp 7 then
            match (unfolded.getArg! 3).consumeMData.headBeta.getAppFn with
            | .const name _ => s!" of {name}"
            | _ => " of ?"
          else ""
        logInfo m!"exit into {head}{action}: {((← IO.getNumHeartbeats) - stageStart) / 1000}k"
      pending := pending ++ (← getGoals).toArray.map fun g => (g, provenance, clauses)
      stageCost := stageCost.push ("exit", (← IO.getNumHeartbeats) - stageStart)
      continue
    if let some (unfolded, branches, normal) ← bindRule? goal then
      -- A straight-line action keeps its continuation in place: it reaches
      -- it on one path.
      if branches then
        let (folded, continuation?) ← foldContinuation unfolded
        setGoals [folded]
        -- The conditions the continuation passes on are normal, where it
        -- is unfolded too.
        if let some continuation := continuation? then
          foldedNormal := foldedNormal.insert continuation (normal.extract 2 4)
      else setGoals [unfolded]
      let normalization ← normalization?.getDM normalization
      normalization? := some normalization
      normalizeAround normalization normal
      pending := pending ++ (← getGoals).toArray.map fun g => (g, provenance, clauses)
      stageCost := stageCost.push ("bind", (← IO.getNumHeartbeats) - stageStart)
      continue
    if let some (site, arguments) ← loopGoal? goal then
      let some (_, function, invariant) := invariants.find? (·.1 == site)
        | throwError m!"no invariant for the loop at site {site}"
      -- The invariant is stated over the loop's skolem instance: its frame's
      -- for an inlined generic callee.
      let invariant := mkApp invariant arguments[0]!
      -- Without a recorded start, the invariant does not read `old`, and
      -- its start arguments vanish on reduction.
      let (start, startState) ← match ← start? goal function with
        | some start => pure start
        | none => goal.withContext do
            let type ← inferType invariant
            let .forallE _ argumentsType rest _ := type
              | throwError m!"the invariant at site {site} takes no function start"
            let .forallE _ stateType _ _ := rest
              | throwError m!"the invariant at site {site} takes no function start"
            pure (← mkFreshExprMVar argumentsType, ← mkFreshExprMVar stateType)
      let invariant ← goal.withContext
        (whnfR (mkAppN invariant #[start, startState, arguments[5]!, arguments[8]!]))
      if (← instantiateMVars invariant).hasExprMVar then
        throwError m!"the invariant at site {site} reads `old` without a recorded function start"
      -- The loop's continuation is bound once, as a local definition the
      -- normalizer does not unfold: the loop hypothesis and every back edge
      -- carry it folded.
      let ensures := arguments[6]!
      let (continuation, goal) ← goal.withContext do
        let target ← instantiateMVars (← goal.getType)
        let defined ← goal.define continuationName (← inferType ensures) ensures
        let (continuation, goal) ← defined.intro1P
        let abstracted := target.replace fun e =>
          if e == ensures then some (mkFVar continuation) else none
        pure (continuation, ← goal.replaceTargetDefEq abstracted)
      let arguments := arguments.set! 6 (mkFVar continuation)
      let rule := mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_loopAt)
        (arguments.extract 0 6 ++ #[invariant] ++ arguments.extract 6 9)
      let subgoals ← goal.withContext (goal.apply rule)
      match subgoals with
      | [entryHolds, step] =>
          let stepStart ← IO.getNumHeartbeats
          let mut marks : Array (String × Nat) := #[]
          setGoals [step]
          evalTactic (← `(tactic| intro recursive loopHypothesis env state loopInvariant))
          -- Assumed, the invariant's clauses are facts rather than obligations.
          evalTactic (← `(tactic| try simp only [LeanerIR.Proofs.Obligation_iff] at loopInvariant))
          -- The invariant is applied to the entry row: its slots are the row's
          -- projections, reduced before the normalization reads them, which
          -- would otherwise derive the row's type at each.
          evalTactic (← `(tactic| leaner_denote_reduce_projections))
          evalTactic (← `(tactic| leaner_denote_normalize at loopHypothesis loopInvariant ⊢))
          marks := marks.push ("normalize 1", ← IO.getNumHeartbeats)
          -- The invariant's equations on the locals are substituted before
          -- the iteration is traversed, so the traversal sees their values.
          evalTactic (← `(tactic| all_goals leaner_denote_split_hypotheses))
          evalTactic (← `(tactic| all_goals leaner_denote_subst_vars))
          marks := marks.push ("split/subst 1", ← IO.getNumHeartbeats)
          -- The whole context once, so that every leaf of the iteration
          -- inherits normal hypotheses, with the equations the
          -- normalization exposes substituted.
          evalTactic (← `(tactic| all_goals (try leaner_denote_normalize_context_stage)))
          marks := marks.push ("normalize 2", ← IO.getNumHeartbeats)
          evalTactic (← `(tactic| all_goals leaner_denote_split_hypotheses))
          evalTactic (← `(tactic| all_goals leaner_denote_subst_vars))
          marks := marks.push ("split/subst 2", ← IO.getNumHeartbeats)
          evalTactic (← `(tactic| all_goals (try leaner_denote_normalize_context_stage)))
          marks := marks.push ("normalize 3", ← IO.getNumHeartbeats)
          if debug then
            let mut previous := stepStart
            let mut report := m!"loop step at site {site}:"
            for (label, mark) in marks do
              report := report ++ m!" {label} {(mark - previous) / 1000}k;"
              previous := mark
            logInfo m!"{report} {(← getGoals).length} goals"
          pending := pending ++ (← getGoals).toArray.map fun g => (g, provenance, clauses)
          setGoals [entryHolds]
          evalTactic (← `(tactic| try leaner_denote_normalize))
          pending := pending ++ (← getGoals).toArray.map fun g => (g, some .loopEntry, #[])
      | _ => throwError "the loop rule did not produce its two obligations"
      stageCost := stageCost.push ("loop", (← IO.getNumHeartbeats) - stageStart)
      continue
    if let some (handle, _) ← callGoal? goal callees then
      let some (_, calleeName, theoremProof) ← callees.findM? fun (candidate, _, _) =>
          goal.withContext (isDefEq candidate handle)
        | throwError m!"no verified callee for {handle}"
      -- A generic callee's theorem holds at every skolem family and type
      -- instantiation; the call's own are found by unification.
      let (theoremProof, theoremArguments) ← goal.withContext do
        let (arguments, _, _) ← forallMetaTelescope (← inferType theoremProof)
        pure (mkAppN theoremProof arguments, arguments)
      let proofSyntax ← goal.withContext (Lean.Elab.Term.exprToSyntax theoremProof)
      setGoals [goal]
      let proofType ← goal.withContext (instantiateMVars (← inferType theoremProof))
      if proofType.isAppOfArity ``Eq 3 then
        -- An unspecified callee: its compiled body replaces the call, by
        -- the agreement theorem, and the traversal continues into it.
        let compiled := (proofType.getArg! 2).appArg!
        let unfoldNames := match compiled with
          | .const name _ =>
              [name, name.getPrefix ++ `body, name.getPrefix ++ `mutables, name.getPrefix ++ `params,
                name.getPrefix ++ `locals, name.getPrefix ++ `shape, name.getPrefix ++ `row]
          | _ => []
        let lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ← unfoldNames.toArray.mapM
          fun name => `(Lean.Parser.Tactic.simpLemma| $(mkIdent (rootNamespace ++ name)):ident)
        let target ← goal.withContext (instantiateMVars (← goal.getType))
        let action := target.getArg! 3
        -- The rule at the call's own arguments; its left side is the goal up
        -- to unfolding the compiled callee's signature, which the kernel
        -- checks — unifying the two here would unfold them in `isDefEq`.
        let equivalence := mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_propheticMeaning_of_compiled)
          #[action.getArg! 0, action.getArg! 1, action.getArg! 2, action.getArg! 3, compiled,
            theoremProof, action.getArg! 6, target.getArg! 4, target.getArg! 5, target.getArg! 6]
        let next ← goal.withContext do
          let_expr Iff lhs rhs := ← whnfR (← inferType equivalence)
            | throwError "the agreement rule is not an equivalence"
          let next ← mkFreshExprSyntheticOpaqueMVar rhs (← goal.getTag)
          goal.assign (mkApp4 (mkConst ``Iff.mpr) lhs rhs equivalence next)
          pure next.mvarId!
        setGoals [next]
        -- The inlined callee starts here: its loops' invariants read `old`
        -- from its arguments at the call.
        if ← invariants.anyM fun (_, function, _) => goal.withContext (isDefEq function handle) then
          let inlined ← getMainGoal
          let started ← inlined.withContext do
            let proof ← mkAppM ``FunctionStart.intro
              #[action.getArg! 3, action.getArg! 6, target.getArg! 6]
            let (_, started) ← (← inlined.assert `leanerStart (← inferType proof) proof).intro1P
            pure started
          replaceMainGoal [started]
        let sizeBefore ← if debug then do
            let goal ← getMainGoal
            pure (← goal.withContext do pure (treeSize (← instantiateMVars (← goal.getType)) 100000000))
          else pure 0
        let unfoldStart ← IO.getNumHeartbeats
        evalTactic (← `(tactic| try simp only [$lemmas,*, LeanerIR.Proofs.Denote.Function.denote,
          LeanerIR.Proofs.Denote.NRow.nil_append, LeanerIR.Proofs.Denote.NRow.cons_append]))
        let normalizeStart ← IO.getNumHeartbeats
        evalTactic (← `(tactic| try leaner_denote_normalize))
        if debug then
          let sizeAfter ← (← getGoals).foldlM (init := 0) fun total goal => do
            pure (total + (← goal.withContext do
              pure (treeSize (← instantiateMVars (← goal.getType)) 100000000)))
          logInfo m!"inline {calleeName}: goal tree {sizeBefore} → {sizeAfter}; unfold \
            {(normalizeStart - unfoldStart) / 1000}k, normalize \
            {((← IO.getNumHeartbeats) - normalizeStart) / 1000}k"
        pending := pending ++
          (← rewriteEquations (← getGoals)).toArray.map fun g => (g, provenance, clauses)
        stageCost := stageCost.push ("inline", (← IO.getNumHeartbeats) - stageStart)
        continue
      evalTactic (← `(tactic| refine LeanerIR.Proofs.Denote.wp_call $proofSyntax ?_ ?_ ?_))
      let subgoals ← getGoals
      -- A callee's theorem assumes the natives it reaches, a generic one's
      -- at every family and instantiation; the caller assumes them too.
      for argument in theoremArguments do
        if let .mvar id ← instantiateMVars argument then
          if ← id.isAssigned then continue
          -- A generic callee runs in a frame: the empty one, the call's, or
          -- one a frame equation of the target gives.
          if (← instantiateMVars (← id.getType)).isAppOfArity ``LeanerIR.Proofs.Denote.FrameOf 4 then
            let mut candidates : Array (TSyntax `tactic) :=
              #[← `(tactic| exact LeanerIR.Proofs.Denote.FrameOf.empty),
                ← `(tactic| exact LeanerIR.Proofs.Denote.FrameOf.frame rfl),
                ← `(tactic| assumption)]
            for equation in equations do
              let term ← goal.withContext (Lean.Elab.Term.exprToSyntax equation)
              candidates := candidates.push (← `(tactic| exact LeanerIR.Proofs.Denote.FrameOf.rewrite
                (LeanerIR.Proofs.Denote.FrameOf.frame rfl) $term))
            let saved ← getGoals
            let mut discharged := false
            for candidate in candidates do
              let before ← saveState
              try
                setGoals [id]
                evalTactic candidate
                if (← getGoals).isEmpty then
                  discharged := true
                  break
              catch _ => pure ()
              before.restore
            setGoals saved
            unless discharged do
              throwError m!"the frame of the generic call of {calleeName} is not established"
            continue
          let native ← forallTelescope (← instantiateMVars (← id.getType)) fun _ body =>
            pure (body.isAppOf ``LeanerIR.Proofs.Satisfies)
          if native then id.assumption
      let constants ← goal.withContext (contractConstants (← inferType theoremProof))
      let lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ← constants.mapM fun name => do
        let ident := mkIdent (rootNamespace ++ name)
        `(Lean.Parser.Tactic.simpLemma| $ident:ident)
      for (subgoal, index) in subgoals.toArray.zipIdx do
        setGoals [subgoal]
        evalTactic (← `(tactic| try simp only [$lemmas,*, lir_denote_norm]))
        evalTactic (← `(tactic| try leaner_denote_normalize))
        let origin := if index == 0 then some (Provenance.precondition calleeName)
          else if index == 1 then some (.continuation calleeName) else none
        pending := pending ++ (← getGoals).toArray.map fun g =>
          (g, origin, if index == 0 then #[] else clauses)
      stageCost := stageCost.push ("call", (← IO.getNumHeartbeats) - stageStart)
      continue
    if let some subgoals ← recursiveHypothesis? goal then
      -- The invariant at the iteration's end, normalized so that its
      -- conjunction splits into leaves.
      setGoals subgoals
      evalTactic (← `(tactic| all_goals (try leaner_denote_normalize)))
      pending := pending ++ (← getGoals).toArray.map fun g => (g, some .loopIteration, #[])
      stageCost := stageCost.push ("recursive", (← IO.getNumHeartbeats) - stageStart)
      continue
    let splitTarget ← if debug then goal.withContext do
        pure (some (← instantiateMVars (← goal.getType))) else pure none
    if let some (subgoals, conditional) ← splitOnce goal then
      let splitDone ← IO.getNumHeartbeats
      for subgoal in subgoals do
        setGoals [subgoal]
        let target ← subgoal.withContext (instantiateMVars (← subgoal.getType))
        if target.isAppOf ``LeanerIR.Proofs.wp then
          let consumed ← if provenance matches some (.continuation _) then do
              evalTactic (← `(tactic| try leaner_denote_consume))
              pure ((← getGoals) != [subgoal])
            else pure false
          if conditional || consumed then
            evalTactic (← `(tactic| try leaner_denote_normalize))
        pending := pending ++ (← getGoals).toArray.map fun g => (g, provenance, clauses)
      let finished ← IO.getNumHeartbeats
      if let some target := splitTarget then
        let head := match target.getAppFn with
          | .const name _ => toString name
          | _ => "?"
        let action := if target.isAppOfArity ``LeanerIR.Proofs.wp 7 then
            match (target.getArg! 3).getAppFn with
            | .const name _ => s!" of {name}"
            | _ => ""
          else ""
        logInfo m!"split {head}{action}: {subgoals.length} goals; split {(splitDone - stageStart) / 1000}k, normalize {(finished - splitDone) / 1000}k"
      stageCost := stageCost.push ("split", (← IO.getNumHeartbeats) - stageStart)
      continue
    setGoals [goal]
    leaves := leaves + 1
    if residual then
      -- An authored proof takes over what the cheap deciders leave, on the
      -- normalized leaf: the pipeline is not run for a leaf the proof
      -- will prove anyway, and a leaf the cheap deciders close costs no
      -- normalization at all.
      let before ← IO.getNumHeartbeats
      let beforeCheap ← saveState
      let closed ← try
          Lean.Elab.Tactic.withoutRecover (evalTactic (← `(tactic| leaner_denote_leaf_cheap)))
          let remaining ← getGoals
          if debug && !remaining.isEmpty then
            let types ← remaining.mapM fun g => g.withContext do instantiateMVars (← g.getType)
            logInfo m!"leaf {leaves}: the cheap deciders leave {remaining.length} goals: {types}"
          pure remaining.isEmpty
        catch failure =>
          if debug then logInfo m!"leaf {leaves} not decided cheaply: {failure.toMessageData}"
          pure false
      if closed then
        if debug then
          logInfo m!"leaf {leaves} closed cheaply: {((← IO.getNumHeartbeats) - before) / 1000}k heartbeats"
        stageCost := stageCost.push ("residual", (← IO.getNumHeartbeats) - before)
        continue
      -- The failed attempt's partial work on the goal is undone.
      beforeCheap.restore (restoreInfo := true)
      setGoals [goal]
      if debug then
        logInfo m!"leaf {leaves} before prepare:\n{goal}"
      let cheap ← IO.getNumHeartbeats
      -- The authored proof sees the leaf in source form, normalized as it.
      evalTactic (← `(tactic| try leaner_denote_unname))
      let beforePrepare ← saveState
      try evalTactic (← `(tactic| leaner_denote_prepare))
      catch failure =>
        let report ← failure.toMessageData.toString
        beforePrepare.restore
        if debug then logInfo m!"leaf {leaves}: prepare failed: {report}"
      let prepared ← IO.getNumHeartbeats
      evalTactic (← `(tactic| all_goals (try leaner_denote_decide_residual)))
      evalTactic (← `(tactic| all_goals (try simp only [LeanerIR.Proofs.Obligation_iff])))
      if debug then
        logInfo m!"leaf {leaves}: cheap {(cheap - before) / 1000}k, prepare {(prepared - cheap) / 1000}k, \
          deciders {((← IO.getNumHeartbeats) - prepared) / 1000}k heartbeats; \
          {(← getGoals).length} residual"
        for g in ← getGoals do logInfo m!"leaf {leaves} after prepare:\n{g}"
      for remaining in ← getGoals do
        remaining.setTag (Name.mkSimple s!"leaf_{residuals.size + 1}")
        residuals := residuals.push remaining
      stageCost := stageCost.push ("residual", (← IO.getNumHeartbeats) - before)
      continue
    let leafStart ← IO.getNumHeartbeats
    let closed ← tryCatchRuntimeEx
        (do
          -- Without recovery: a failing alternative raises at once, so the
          -- next alternative runs instead of an aborted goal list.
          Lean.Elab.Tactic.withoutRecover (evalTactic (← `(tactic| leaner_denote_leaf)))
          pure (← getGoals).isEmpty)
        fun failure => do
          -- The target's own heartbeat budget stays the target's failure; a
          -- leaf exhausting the recursion depth is that leaf's.
          if failure.isMaxHeartbeat then throw failure
          if debug then logInfo m!"leaf not decided: {failure.toMessageData}\n{goal}"
          pure false
    stageCost := stageCost.push (if closed then "leaf" else "undecided",
      (← IO.getNumHeartbeats) - leafStart)
    if debug then
      IO.println s!"leaf {leaves}: {if closed then "closed" else "undecided"} in \
        {((← IO.getNumHeartbeats) - leafStart) / 1000}k heartbeats"
    unless closed do
      setGoals [goal]
      reported ← reportObligation goal provenance clauses reported
      admitGoal goal
  if debug then
    let mut totals : Array (String × Nat × Nat) := #[]
    for (stage, cost) in stageCost do
      match totals.findIdx? (·.1 == stage) with
      | some index => totals := totals.modify index fun (name, count, sum) => (name, count + 1, sum + cost)
      | none => totals := totals.push (stage, 1, cost)
    let mut report := m!""
    for (stage, count, sum) in totals do
      report := report ++ m!" {stage} ×{count} {sum / 1000}k;"
    logInfo m!"closer {(← Lean.Elab.Term.getDeclName?).getD .anonymous}: \
      {((← IO.getNumHeartbeats) - startHeartbeats) / 1000}k heartbeats, \
      {leaves} leaves, {residuals.size} residual;{report}"
    IO.println "=== closer end"
  setGoals residuals.toList

/-- Split the normalized goal into leaves and decide each; report the
clause of every leaf that is not decided.  Loops are handled by the
invariants given as `(site, function, invariant)` triples, calls by the callees' theorems,
and the `using` equations rewrite wherever a body brings their terms in. -/
syntax "leaner_denote_close" &" residual"? (" [" term,* "]")? (" with" " [" term,* "]")?
  (" using" " [" term,* "]")? : tactic

elab_rules : tactic
  | `(tactic| leaner_denote_close $[residual%$residualToken]? $[[$loops:term,*]]?
      $[with [$calls:term,*]]? $[using [$equations:term,*]]?) => do
      if leaner.denoteDebug.get (← getOptions) then
        logInfo m!"normalized verification condition:\n{← getMainGoal}"
      let mut invariants : Array (Nat × Lean.Expr × Lean.Expr) := #[]
      for loop in (loops.map (·.getElems)).getD #[] do
        let triple ← Lean.Elab.Tactic.elabTerm loop none
        let triple ← whnf (← instantiateMVars triple)
        let rest ← whnf (triple.getArg! 3)
        unless triple.isAppOfArity ``Prod.mk 4 && rest.isAppOfArity ``Prod.mk 4 do
          throwError m!"a loop invariant must be a `(site, function, invariant)` triple, \
            not {triple}"
        let some site ← (evalNat (triple.getArg! 2)).run
          | throwError m!"a loop site must be a numeral, not {triple.getArg! 2}"
        invariants := invariants.push (site, rest.getArg! 2, rest.getArg! 3)
      let mut callees : Array (Lean.Expr × String × Lean.Expr) := #[]
      for call in (calls.map (·.getElems)).getD #[] do
        let pair ← Lean.Elab.Tactic.elabTerm call none
        let pair ← instantiateMVars pair
        let pair ← whnf pair
        unless pair.isAppOfArity ``PProd.mk 4 && (pair.getArg! 3).isAppOfArity ``PProd.mk 4 do
          throwError m!"a callee must be `PProd.mk handle (PProd.mk name theorem)`, not {pair}"
        let inner := pair.getArg! 3
        let some name := (match inner.getArg! 2 with
            | .lit (.strVal name) => some name
            | _ => none)
          | throwError m!"a callee name must be a string literal, not {inner.getArg! 2}"
        callees := callees.push (pair.getArg! 2, name, inner.getArg! 3)
      let equations ← ((equations.map (·.getElems)).getD #[]).mapM fun equation => do
        instantiateMVars (← Lean.Elab.Tactic.elabTerm equation none)
      closeGoals invariants callees equations residualToken.isSome

end LeanerIR.Proofs.Denote
