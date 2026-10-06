-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Agreement
import LeanerIR.Proofs.Denote.Closures
import LeanerIR.Proofs.Behavior
import LeanerIR.Proofs.ClosureTyping
import LeanerIR.Proofs.Order
import LeanerIR.Proofs.Denote.SimpAll
import LeanerIR.Proofs.Maps
import LeanerIR.Proofs.Denote.BitLift
import LeanerIR.Proofs.Denote.LemmaSteps

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
def FunctionStart {α σ : Type} (_function : LeanerIR.FunctionHandle) (_arguments : α)
    (_state : σ) : Prop := True

theorem FunctionStart.intro {α σ : Type} (function : LeanerIR.FunctionHandle) (arguments : α)
    (state : σ) : FunctionStart function arguments state := trivial

/-- Marks the locals and state a state anchor at a site records, what the
in-body assertions reading it see. -/
def AnchorSaved {α σ : Type} (_site : Nat) (_env : α) (_state : σ) : Prop :=
  True

theorem AnchorSaved.intro {α σ : Type} (site : Nat) (env : α) (state : σ) :
    AnchorSaved site env state := trivial

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
  | assertion
  | mutationEnd
  | construction
  | memoryWritten
  | callReturned
  | lemmaRequirement

def Provenance.describe : Provenance → String
  | .precondition callee => s!"the precondition of `{callee}`"
  | .continuation callee => s!"the continuation after `{callee}`"
  | .loopEntry => "a loop invariant at entry"
  | .loopIteration => "a loop invariant at an iteration"
  | .assertion => "an in-body assertion"
  | .mutationEnd => "a data invariant where a mutation ends"
  | .construction => "a data invariant where a value is constructed"
  | .memoryWritten => "a module invariant where a write of memory ends"
  | .callReturned => "a module invariant where a call returns"
  | .lemmaRequirement => "the requirement of an applied lemma"

/-- What an obligation located at an authored clause says: a loop's
invariant is owed at entry and after each iteration, any other clause by the
function. -/
private def clauseFailure (origin : Option Provenance) (snippet : String) : MessageData :=
  match origin with
  | some .loopEntry => m!"the loop invariant `{snippet}` is not established at entry"
  | some .loopIteration => m!"the loop invariant `{snippet}` is not preserved by an iteration"
  | some .assertion => m!"the assertion `{snippet}` does not hold"
  | some .mutationEnd => m!"the data invariant `{snippet}` does not hold where a mutation ends"
  | some .construction =>
      m!"the data invariant `{snippet}` does not hold where a value is constructed"
  | some .memoryWritten =>
      m!"the module invariant `{snippet}` does not hold after a write of memory it reads"
  | some .callReturned =>
      m!"the module invariant `{snippet}` does not hold after a call that writes memory it reads"
  | some .lemmaRequirement => m!"the requirement of the lemma applied at `{snippet}` does not hold"
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

register_option leaner.denoteProfile : Bool := {
  defValue := false
  descr := "report, per verified function, the heartbeats each closer step spends, \
    succeeding and failing"
}

/-- Per closer step label: runs, failures, heartbeats in succeeding runs, and
heartbeats in failing runs, under `leaner.denoteProfile`. -/
initialize stepProfile : IO.Ref (Std.HashMap String (Nat × Nat × Nat × Nat)) ← IO.mkRef {}

/-- A program point of the closer's run: the value and state it handed a
continuation at a call boundary. A hypothesis of every leaf below the
point, in program order, which the context's rewrites carry along. A state
label's existential is witnessed at one: the state for its memory, the
values of the locals for its copies of the mutable parameters. -/
def ProgramPoint {unit : Validation.ValidatedUnit} [Skolems unit] {ρ : ResultShape} {Γ : NRow}
    {α : Type} (value : Flow ρ Γ α) (state : Memory unit) : Prop := True

theorem ProgramPoint.intro {unit : Validation.ValidatedUnit} [Skolems unit] {ρ : ResultShape}
    {Γ : NRow} {α : Type} (value : Flow ρ Γ α) (state : Memory unit) :
    ProgramPoint value state := trivial

/-- The goal with the program point `(value, state)` as an implementation
detail of its context: only the witness stage reads it, and the deciders of
a leaf skip it. The goal unchanged where `value` is not a flow. -/
private def noteProgramPoint (goal : MVarId) (value state : Lean.Expr) : MetaM MVarId :=
  goal.withContext do
    let flow ← instantiateMVars (← inferType value)
    unless flow.isAppOfArity ``Flow 5 do return goal
    -- The point's implicit arguments are the flow's; its `Skolems` instance
    -- is a hypothesis, which instance synthesis would not find.
    let arguments := flow.getAppArgs ++ #[value, state]
    let proof := mkAppN (mkConst ``ProgramPoint.intro) arguments
    let fvarId ← mkFreshFVarId
    let lctx := (← getLCtx).mkLocalDecl fvarId `programPoint
      (mkAppN (mkConst ``ProgramPoint) arguments) .default .implDetail
    let next ← mkFreshExprMVarAt lctx (← getLocalInstances) (← goal.getType) .syntheticOpaque
      (← goal.getTag)
    let abstracted ← withLCtx lctx (← getLocalInstances) (mkLambdaFVars #[mkFVar fvarId] next)
    goal.assign (mkApp abstracted proof)
    pure next.mvarId!

private def recordStep (label : String) (failed : Bool) (cost : Nat) : IO Unit :=
  stepProfile.modify fun profile =>
    let (runs, failures, succeeding, failing) := profile.getD label (0, 0, 0, 0)
    profile.insert label (if failed then (runs + 1, failures + 1, succeeding, failing + cost)
      else (runs + 1, failures, succeeding + cost, failing))

/-- Run a step and, under `leaner.denoteDebug`, report its heartbeats; under
`leaner.denoteProfile`, add them to its label's totals. -/
elab "leaner_denote_timed " label:str step:tactic : tactic => do
  let debug := leaner.denoteDebug.get (← getOptions)
  let profile := leaner.denoteProfile.get (← getOptions)
  let start ← IO.getNumHeartbeats
  tryCatchRuntimeEx
    (do
      evalTactic step
      let cost := (← IO.getNumHeartbeats) - start
      if profile then recordStep label.getString false cost
      if debug then
        IO.println s!"  {label.getString}: {cost / 1000}k, \
          {(← getGoals).length} goals")
    fun failure => do
      let cost := (← IO.getNumHeartbeats) - start
      if profile then recordStep label.getString true cost
      if debug then
        IO.println s!"  {label.getString}: {cost / 1000}k, failed\
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

/-- An unsigned element a vector read finds, as an integer: the optional
element of `((o.map (specInt (bits w) false).encode).getD unit).asInt`. -/
private def unsignedElementRead? (e : Lean.Expr) : Option Lean.Expr := do
  guard (e.isAppOfArity ``RuntimeValue.asInt 1 && !e.hasLooseBVars)
  let read := e.appArg!
  guard (read.isAppOfArity ``Option.getD 3 && (read.getArg! 2).isConstOf ``RuntimeValue.unit)
  let mapped := read.getArg! 1
  guard (mapped.isAppOfArity ``Option.map 4)
  let encoder := mapped.getArg! 2
  guard (encoder.isAppOfArity ``LeanerIR.Proofs.Codec.encode 3)
  let codec := encoder.getArg! 2
  guard (codec.isAppOfArity ``LeanerIR.Proofs.Codec.specInt 2 &&
    (codec.getArg! 1).isConstOf ``Bool.false && (codec.getArg! 0).isAppOfArity ``IntWidth.bits 1)
  return mapped.getArg! 3

/-- The bounds of an unsigned integer an entry a read finds holds, stated at
the read's own spelling: `((entry.map fun v => integer (f v).val).getD
unit).asInt` for an unsigned certified `f v`. -/
private def unsignedProjectionRead? (e : Lean.Expr) : MetaM (Option Lean.Expr) := do
  unless e.isAppOfArity ``RuntimeValue.asInt 1 && !e.hasLooseBVars do return none
  let read := e.appArg!
  unless read.isAppOfArity ``Option.getD 3 && (read.getArg! 2).isConstOf ``RuntimeValue.unit do
    return none
  let mapped := read.getArg! 1
  unless mapped.isAppOfArity ``Option.map 4 do return none
  let .lam name domain body info := mapped.getArg! 2 | return none
  unless body.isAppOfArity ``RuntimeValue.integer 1 do return none
  let value := body.appArg!
  unless value.isAppOfArity ``LeanerIR.SpecInt.val 3 do return none
  let width := value.getArg! 0
  unless width.isAppOfArity ``IntWidth.bits 1 && (value.getArg! 1).isConstOf ``Bool.false do
    return none
  let projection := Lean.Expr.lam name domain (value.getArg! 2) info
  let proof ← mkAppM ``LeanerIR.Proofs.Denote.asInt_getD_map_integer_unsigned
    #[projection, mapped.getArg! 3]
  return some (← mkExpectedTypeHint proof (← Core.betaReduce (← inferType proof)))

/-- The range of a truncating quotient or remainder: from its operands'
signs, their signed widths, or its operands' magnitudes. -/
private def tdivBounds? (sub : Lean.Expr) : MetaM (Option Lean.Expr) := do
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

/-- What a leaf reads of the division sites it holds, collected once: the
remainders it states, and the operands of its products. -/
private structure DivisionReads where
  remainders : Array Lean.Expr
  factors : Array Lean.Expr
  /-- The equalities between integers the leaf states, as `(lhs, rhs)`. -/
  equalities : Array (Lean.Expr × Lean.Expr)

/-- Whether a product the leaf reads has `term` as a factor, up to the leaf's
equalities: a quotient an opaque callee returns is multiplied through the
variable its contract equates with it. -/
private def DivisionReads.readsFactor (reads : DivisionReads) (term : Lean.Expr) : Bool :=
  Id.run do
    let mut related := #[term]
    let mut grown := true
    while grown do
      grown := false
      for (lhs, rhs) in reads.equalities do
        if related.contains lhs && !related.contains rhs then
          related := related.push rhs; grown := true
        else if related.contains rhs && !related.contains lhs then
          related := related.push lhs; grown := true
    return reads.factors.any related.contains

/-- The bound of one bit operation, when its operands are certified. A
division site's facts are chosen by what the leaf reads (`reads`), and
its remainder's bound by the divisor's positivity, which omega decides over
the leaf's hypotheses (`goal`). -/
private def operationFact? (goal : MVarId) (reads : DivisionReads) (sub : Lean.Expr) :
    MetaM (Option Lean.Expr) := do
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
            return some (← mkAppM ``LeanerIR.Proofs.IntegerArithmetic.bitwiseAnd_bounds
              #[left, right])
        | _, _ => return none
  -- A disjunction or exclusive disjunction of certified unsigned values, as
  -- the runtime computes it, lies within their width.
  if sub.isAppOfArity ``Int.ofNat 1 then
    let bits := sub.appArg!
    let op? := if bits.isAppOfArity ``HOr.hOr 6 then some ``LeanerIR.Proofs.Denote.BitOp.or
      else if bits.isAppOfArity ``HXor.hXor 6 then some ``LeanerIR.Proofs.Denote.BitOp.xor
      else none
    let operand? (e : Lean.Expr) : Option Lean.Expr :=
      if e.isAppOfArity ``Int.toNat 1 then (unsignedValue? e.appArg!).map (·.2) else none
    -- Exclusive disjunction with the ones of a width at least the value's:
    -- the value's complement there.
    if bits.isAppOfArity ``HXor.hXor 6 then
      for (valueSide, onesSide, swapped) in
          [(bits.getArg! 4, bits.getArg! 5, false), (bits.getArg! 5, bits.getArg! 4, true)] do
        let some ones := onesSide.nat? <|> onesSide.rawNatLit? | continue
        let k := (ones + 1).log2
        unless 2 ^ k == ones + 1 && valueSide.isAppOfArity ``Int.toNat 1 do continue
        let some (width, value) := unsignedValue? valueSide.appArg! | continue
        let some w := width.nat? <|> width.rawNatLit? | continue
        unless w ≤ k do continue
        let wide ← mkDecideProof (← mkAppM ``LE.le #[width, mkNatLit k])
        let complement ← mkAppM ``LeanerIR.Proofs.Denote.SpecInt.toNat_xor_ones #[value, wide]
        let proof ← if swapped then
            mkEqTrans (← mkAppM ``Nat.xor_comm #[onesSide, valueSide]) complement
          else pure complement
        return some (← mkExpectedTypeHint proof
          (← mkEq bits (← mkAppM ``HSub.hSub #[onesSide, valueSide])))
    let some op := op? | return none
    let some left := operand? (bits.getArg! 4) | return none
    let some right := operand? (bits.getArg! 5) | return none
    let bounds ← mkAppM ``LeanerIR.Proofs.Denote.BitOp.eval_bounds #[mkConst op, left, right]
    -- The operation's `eval` is the site, by definition.
    let evaluated := mkApp3 (mkConst ``LeanerIR.Proofs.Denote.BitOp.eval) (mkConst op)
      (bits.getArg! 4).appArg! (bits.getArg! 5).appArg!
    let stated := (← instantiateMVars (← inferType bounds)).replace fun e =>
      if e == evaluated then some sub else none
    return some (← mkExpectedTypeHint bounds stated)
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
    let bounds ← tdivBounds? sub
    let (dividend, divisor) := (sub.getArg! 0, sub.getArg! 1)
    if (divisor.int? <|> (divisor.nat?.map Int.ofNat)).isSome then return bounds
    -- At a divisor that is not a literal: the division algorithm of
    -- nonnegative operands, where the leaf reads the remainder or a product
    -- with the divisor or the quotient, up to its equalities (the identity
    -- constrains nothing else), with the remainder below a divisor the hypotheses make
    -- positive, and conditionally on positivity only where the remainder is
    -- read; and the quotient of a dividend the divisor divides.
    let quotient := mkApp2 (mkConst ``Int.tdiv) dividend divisor
    let readsRemainder := reads.remainders.contains (mkApp2 (mkConst ``Int.tmod) dividend divisor)
    let needsIdentity := readsRemainder ||
      reads.readsFactor divisor || reads.readsFactor quotient
    let algorithm ← if !needsIdentity then pure none else
      match ← nonnegative? dividend, ← nonnegative? divisor with
      | some dividendNonnegative, some divisorNonnegative =>
          let identity ← mkAppM ``Int.tdiv_mul_add_tmod #[dividend, divisor]
          let nonnegative ← mkAppM ``Int.tmod_nonneg #[divisor, dividendNonnegative]
          let positive ← mkAppM ``LT.lt #[mkIntLit 0, divisor]
          let positivity ← omegaProof? goal
            (← mkArrow (← inferType divisorNonnegative) positive)
          let below ← match positivity with
            | some proof =>
                some <$> mkAppM ``Int.tmod_lt_of_pos #[dividend, proof.app divisorNonnegative]
            | none =>
                if !readsRemainder then pure none else
                  some <$> withLocalDeclD `positive positive fun h => do
                    mkLambdaFVars #[h] (← mkAppM ``Int.tmod_lt_of_pos #[dividend, h])
          conjoin? (some identity) (← conjoin? (some nonnegative) below)
      | _, _ => pure none
    let cancellation ← if !sub.isAppOfArity ``Int.tdiv 2 then pure none
      else if dividend == divisor then some <$> mkAppOptM ``Int.tdiv_self #[dividend]
      else if dividend.isAppOfArity ``HMul.hMul 6 && dividend.getArg! 5 == divisor then
        some <$> mkAppOptM ``Int.mul_tdiv_cancel #[dividend.getArg! 4, divisor]
      else if dividend.isAppOfArity ``HMul.hMul 6 && dividend.getArg! 4 == divisor then
        some <$> mkAppOptM ``Int.mul_tdiv_cancel_left #[divisor, dividend.getArg! 5]
      else pure none
    return ← conjoin? (← conjoin? bounds algorithm) cancellation
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
    (e.isAppOfArity ``Int.ofNat 1 &&
      (e.appArg!.isAppOfArity ``HOr.hOr 6 || e.appArg!.isAppOfArity ``HXor.hXor 6)) ||
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
      -- A codec states no type arguments, and a structure's codec is the same
      -- at every one.
      return mkAppN (mkConst ``LeanerIR.Proofs.Denote.NTy.struct)
        #[codec.getArg! 1, mkConst ``LeanerIR.Proofs.Denote.NRow.nil, row.getArg! 1]
    scalar ``LeanerIR.Proofs.Codec.bool ``LeanerIR.Proofs.Denote.NTy.bool <|>
      scalar ``LeanerIR.Proofs.Codec.address ``LeanerIR.Proofs.Denote.NTy.address <|>
      scalar ``LeanerIR.Proofs.Codec.signer ``LeanerIR.Proofs.Denote.NTy.signer <|>
      scalar ``LeanerIR.Proofs.Codec.string ``LeanerIR.Proofs.Denote.NTy.string <|>
      scalar ``LeanerIR.Proofs.Codec.bytes ``LeanerIR.Proofs.Denote.NTy.bytes <|>
      scalar ``LeanerIR.Proofs.Codec.unit ``LeanerIR.Proofs.Denote.NTy.unit

/-- The bounds of the unsigned elements some reads find, each once. -/
private def elementBoundsOf (reads : Array Lean.Expr) : MetaM (Array Lean.Expr) := do
  let mut elements : Array Lean.Expr := #[]
  for read in reads do
    if let some element := unsignedElementRead? read then
      unless elements.contains element do elements := elements.push element
  elements.mapM fun element => mkAppM ``LeanerIR.Proofs.Denote.asInt_getD_map_unsigned #[element]

/-- The bounds of the unsigned elements some terms read anywhere. -/
private def elementBounds (terms : Array Lean.Expr) : MetaM (Array Lean.Expr) :=
  elementBoundsOf (sitesWhere (fun e => (unsignedElementRead? e).isSome) terms)

/-- The distinct terms a linear reading of some terms reaches: through sums,
differences, products, and negations, which `omega` reads, and not into any
other operation, whose result it reads as an atom. -/
private partial def linearOperands (terms : Array Lean.Expr) : Array Lean.Expr :=
  (terms.forM go |>.run ({}, #[])).2.2
where
  go (e : Lean.Expr) : StateM (Std.HashSet Lean.Expr × Array Lean.Expr) Unit := do
    if (← get).1.contains e then return
    modify fun (visited, found) => (visited.insert e, found)
    if e.isAppOfArity ``HAdd.hAdd 6 || e.isAppOfArity ``HSub.hSub 6 ||
        e.isAppOfArity ``HMul.hMul 6 then
      go (e.getArg! 4); go (e.getArg! 5)
    else if e.isAppOfArity ``Neg.neg 3 then go (e.getArg! 2)
    else modify fun (visited, found) => (visited, found.push e)

/-- The divisions a leaf's goal and hypotheses read. -/
private def divisionReads (terms : Array Lean.Expr) : DivisionReads :=
  let products := sitesWhere (·.isAppOfArity ``HMul.hMul 6) terms
  let equalities := sitesWhere
    (fun e => e.isAppOfArity ``Eq 3 && (e.getArg! 0).isConstOf ``Int) terms
  { remainders := sitesWhere (·.isAppOfArity ``Int.tmod 2) terms
    factors := products.flatMap fun product => #[product.getArg! 4, product.getArg! 5]
    equalities := equalities.map fun equality => (equality.getArg! 1, equality.getArg! 2) }

/-- Assert the facts a leaf needs beside its hypotheses: the bounds of every
certified integer in context, and the bounds of every bit operation on
them that the goal or a hypothesis mentions. A fact whose statement is in
`skip` is not asserted. The statements asserted are returned. -/
def assertBounds (skip : Array Lean.Expr := #[]) : TacticM (Array Lean.Expr) := do
  if (← getGoals).isEmpty then return #[]
  let goal ← getMainGoal
  let facts ← goal.withContext do
    let mut facts : Array Lean.Expr := #[]
    let target ← instantiateMVars (← goal.getType)
    let mut expressions := #[target]
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
    let reads := divisionReads expressions
    for site in operationSites expressions do
      if let some fact ← operationFact? goal reads site then facts := facts.push fact
    -- The bounds of the unsigned elements the goal compares by order, as
    -- its linear reading reaches them.
    let comparisons := sitesWhere (fun e => e.isAppOfArity ``LE.le 4 || e.isAppOfArity ``LT.lt 4)
      #[target]
    let operands := comparisons.flatMap fun comparison => #[comparison.getArg! 2, comparison.getArg! 3]
    facts := facts ++ (← elementBoundsOf (linearOperands operands))
    -- And of the unsigned fields of the entries it reads.
    let mut projected : Array Lean.Expr := #[]
    for comparison in comparisons do
      for operand in #[comparison.getArg! 2, comparison.getArg! 3] do
        unless projected.contains operand do
          if let some fact ← (try unsignedProjectionRead? operand catch _ => pure none) then
            projected := projected.push operand
            facts := facts.push fact
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
        -- A position an array reads a value at lies within it, where the
        -- goal reads the array's size.
        if lhs.isAppOfArity ``GetElem?.getElem? 7 && rhs.isAppOfArity ``Option.some 2 then
          let array := lhs.getArg! 5
          if (target.find? fun e => e.isAppOfArity ``Array.size 2 && e.appArg! == array).isSome then
            if let some fact ← attempt (mkAppM
                ``LeanerIR.Proofs.Denote.lt_size_of_getElem?_eq_some #[proof]) then
              facts := facts.push fact
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
      -- Saved continuations have a plain function type; their bodies carry
      -- the computation. Leaving unused ones here makes simp unfold the
      -- rest of the program again on an already extracted leaf.
      let savedComputation := decl.value?.any fun value =>
        let constants := value.getUsedConstants
        constants.contains ``LeanerIR.Proofs.wp || constants.contains ``LeanerIR.Proofs.Spec
      if savedComputation || constants.contains ``LeanerIR.Proofs.wp || constants.contains ``LeanerIR.Proofs.Spec ||
          constants.contains ``LeanerIR.Validation.prepareExecution ||
          constants.contains ``LeanerIR.Validation.ExecutableUnit ||
          constants.contains ``LeanerIR.Validation.SemanticsRegistry then
        found := found.push decl.fvarId
    pure found
  let mut goal := goal
  for fvarId in victims.reverse do
    goal ← goal.tryClear fvarId
  replaceMainGoal [goal]

/-- Read memory as the leaf's case splits fixed it: a slot a hypothesis
equates to a value (`memory r k = some v`, from a program read's split) is
that value wherever the goal reads it, in normal form, so that a clause's
read meets the program's before the arithmetic is normalized. -/
elab "leaner_denote_memory_reads" : tactic => do
  if (← getGoals).isEmpty then return
  let goal ← getMainGoal
  let reads ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let mut found : Array FVarId := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      let some (_, lhs, _) := ty.eq? | continue
      let head := lhs.getAppFn
      unless head.isFVar && lhs.getAppNumArgs == 2 do continue
      unless (target.find? (· == lhs)).isSome do continue
      if (← whnfR (← inferType head)).isAppOfArity ``LeanerIR.Proofs.Denote.Memory 1 then
        found := found.push decl.fvarId
    pure found
  if reads.isEmpty then throwError "the goal reads no memory the leaf fixes"
  let lemmas ← goal.withContext do
    reads.mapM fun fvarId => do
      `(Lean.Parser.Tactic.simpLemma| $(← Lean.Elab.Term.exprToSyntax (.fvar fvarId)):term)
  evalTactic (← `(tactic| simp only [$lemmas,*, lir_denote_norm]))

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

/-- Facts the arithmetic fast path can use without inspecting stored-state
invariants, closure contracts, or equalities between encoded values. -/
private partial def arithmeticProposition (type : Lean.Expr) : Bool :=
  if type.isAppOfArity ``Eq 3 then
    (type.getArg! 0).isConstOf ``Int || (type.getArg! 0).isConstOf ``Nat
  else if type.isAppOfArity ``LE.le 4 || type.isAppOfArity ``LT.lt 4 then
    (type.getArg! 0).isConstOf ``Int || (type.getArg! 0).isConstOf ``Nat
  else if type.isAppOfArity ``Not 1 then arithmeticProposition (type.getArg! 0)
  else if type.isAppOfArity ``And 2 || type.isAppOfArity ``Or 2 then
    arithmeticProposition (type.getArg! 0) && arithmeticProposition (type.getArg! 1)
  else type.isConstOf ``False

/-- Try only arithmetic hypotheses first. Even checked integer bounds become
expensive when omega preprocesses unrelated resource and function facts.
This speculative attempt is small; the caller retains the full-context solver
when its proof needs facts outside this syntactic subset. -/
elab "leaner_denote_arithmetic_only" : tactic => do
  let context ← readThe Core.Context
  let now ← IO.getNumHeartbeats
  let remaining := if context.maxHeartbeats == 0 then 500000
    else context.initHeartbeats + context.maxHeartbeats - now
  let budget := max 1 (min remaining 500000)
  tryCatchRuntimeEx
    (withTheReader Core.Context
      (fun context => { context with initHeartbeats := now, maxHeartbeats := budget }) do
      let original ← getMainGoal
      let some goal ← original.falseOrByContra | replaceMainGoal []; return
      goal.withContext do
        let facts ← (← getLCtx).foldlM (init := []) fun facts decl => do
          if decl.isImplementationDetail then return facts
          let type ← instantiateMVars decl.type
          if arithmeticProposition type then return decl.toExpr :: facts else return facts
        Lean.Elab.Tactic.Omega.omega facts goal {}
      replaceMainGoal [])
    fun failure => do
      if failure.isMaxHeartbeat then throwError "the arithmetic fast path exceeded its budget"
      throw failure

/-- Filtering is useful only where memory-dependent facts would burden the
arithmetic solver. Pure arithmetic keeps its original proof construction,
whose terms are smaller than the speculative refutation's on simple bounds. -/
elab "leaner_denote_has_memory_facts" : tactic => do
  let goal ← getMainGoal
  let applies ← goal.withContext do
    let mut memories : FVarIdSet := {}
    for decl in ← getLCtx do
      if decl.type.isAppOfArity ``LeanerIR.Proofs.Denote.Memory 1 then
        memories := memories.insert decl.fvarId
    if memories.isEmpty then return false
    (← getLCtx).anyM fun decl => do
      if decl.isImplementationDetail || decl.isLet then return false
      unless ← isProp decl.type do return false
      return (decl.type.find? fun e => e.isFVar && memories.contains e.fvarId!).isSome
  unless applies do throwError "the leaf has no memory-dependent hypothesis"

macro "leaner_denote_omega" : tactic => `(tactic|
  first | (leaner_denote_has_memory_facts; leaner_denote_arithmetic_only) | omega)

/-- Abstract the rank tables of the structural order (`valueRanks` of a
literal) to variables: no lemma reads inside one, and as literals they make
up much of a map term. Every occurrence of a table is abstracted at once; a
table in the type of a local stays. -/
private def abstractRankTables (goal : MVarId) : MetaM MVarId := do
  let tables ← goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    for decl in ← getLCtx do
      unless decl.isImplementationDetail do terms := terms.push (← instantiateMVars decl.type)
    return sitesWhere (fun e => e.isAppOfArity ``LeanerIR.SemanticOperations.valueRanks 1 &&
      !e.hasFVar && !e.hasMVar && !e.hasLooseBVars) terms
  let mut goal := goal
  for table in tables do
    let hypotheses? ← goal.withContext do
      let mut hypotheses := #[]
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let type ← instantiateMVars decl.type
        if table.occurs type then
          unless ← isProp type do return none
          hypotheses := hypotheses.push decl.fvarId
      return some hypotheses
    let some hypotheses := hypotheses? | continue
    try
      let (_, _, next) ← goal.withContext <|
        goal.generalizeHyp #[{ expr := table, xName? := some `ranks }] hypotheses
      goal := next
    catch _ => pure ()
  return goal

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

/-- Reads of one array at positions `omega` proves equal, spelled alike:
`grind` relates its arithmetic to its congruence closure only through the
terms it is given, so `xs[i.toNat]` and `xs[n - 1 - (↑n - 1 - i).toNat]?`
stay apart for it. A position is rewritten, in the goal and the
hypotheses, to the shallowest one equal to it; the goal's binders are
introduced first. -/
private def identifyReads (goal : MVarId) : TacticM MVarId := do
  let (goal, _) ← introduceThroughMarkers goal
  let isRead (e : Lean.Expr) : Bool :=
    (e.isAppOfArity ``GetElem?.getElem? 7 || e.isAppOfArity ``GetElem.getElem 8) &&
      (e.getArg! 1).isConstOf ``Nat && !e.hasLooseBVars && !e.hasMVar
  let reads ← goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    for decl in ← getLCtx do
      unless decl.isImplementationDetail do
        let type ← instantiateMVars decl.type
        if ← isProp type then terms := terms.push type
    return sitesWhere isRead terms
  let mut groups : Array (Lean.Expr × Array Lean.Expr) := #[]
  for read in reads do
    let collection := read.getArg! 5
    let position := read.getArg! 6
    match groups.findIdx? (·.1 == collection) with
    | some index =>
        unless groups[index]!.2.contains position do
          groups := groups.modify index fun (collection, positions) =>
            (collection, positions.push position)
    | none => groups := groups.push (collection, #[position])
  let mut goal := goal
  for (_, positions) in groups do
    let ordered := positions.qsort (·.approxDepth < ·.approxDepth)
    for deeper in [1:ordered.size] do
      for shallower in [0:deeper] do
        let proof? ← goal.withContext do
          let proof ← mkFreshExprMVar (← mkEq ordered[deeper]! ordered[shallower]!)
          try
            let remaining ← Lean.Elab.Tactic.run proof.mvarId! (evalTactic (← `(tactic| omega)))
            if remaining.isEmpty then return some (← instantiateMVars proof)
            return none
          catch _ => return none
        let some proof := proof? | continue
        goal ← goal.withContext do
          let mut goal := goal
          for decl in ← getLCtx do
            if decl.isImplementationDetail then continue
            let type ← instantiateMVars decl.type
            unless ordered[deeper]!.occurs type && (← isProp type) do continue
            try
              let rewritten ← goal.rewrite type proof
              goal := (← goal.replaceLocalDecl decl.fvarId rewritten.eNew rewritten.eqProof).mvarId
            catch _ => pure ()
          try
            let rewritten ← goal.rewrite (← goal.getType) proof
            goal ← goal.replaceTargetEq rewritten.eNew rewritten.eqProof
          catch _ => pure ()
          return goal
        break
  return goal

/-- An equation between arrays, element by element: the reads of its two
sides, by the element equations of the operations that built them. `grind`
finds the extensionality itself only at many times the cost. `none` when
the equations decide it. -/
private def arraysByElements (goal : MVarId) : TacticM (Option MVarId) := do
  let (goal, _) ← introduceThroughMarkers goal
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let some (type, _, _) := target.eq? | return some goal
  unless type.isAppOfArity ``Array 1 do return some goal
  let saved ← saveState
  try
    let [next] ← goal.apply (← mkConstWithFreshMVarLevels ``Array.ext_getElem?)
      | saved.restore; return some goal
    let (_, next) ← next.intro1P
    setGoals [next]
    evalTactic (← `(tactic| try simp only [lir_denote_norm, Array.getElem?_eraseIdx,
      Array.getElem?_insertIdx, Array.getElem?_setIfInBounds, Array.getElem?_push,
      Array.getElem?_append]))
    return (← getGoals).head?
  catch _ =>
    saved.restore
    return some goal

/-- The products of two non-literal integer factors a term holds. -/
private def integerProducts (e : Lean.Expr) : Array (Lean.Expr × Lean.Expr) :=
  sitesWhere (fun e => e.isAppOfArity ``HMul.hMul 6 && (e.getArg! 0).isConstOf ``Int &&
    !e.hasLooseBVars && (e.getArg! 4).int?.isNone && (e.getArg! 5).int?.isNone &&
    (e.getArg! 4).nat?.isNone && (e.getArg! 5).nat?.isNone) #[e]
  |>.map fun product => (product.getArg! 4, product.getArg! 5)

/-- Whether a linear integer term reads a factor as one of its summands,
possibly scaled by a literal. -/
private partial def readsLinearly (factor : Lean.Expr) (term : Lean.Expr) : Bool :=
  term == factor ||
    if term.isAppOfArity ``HAdd.hAdd 6 || term.isAppOfArity ``HSub.hSub 6 then
      readsLinearly factor (term.getArg! 4) || readsLinearly factor (term.getArg! 5)
    else if term.isAppOfArity ``Neg.neg 3 then readsLinearly factor (term.getArg! 2)
    else if term.isAppOfArity ``HMul.hMul 6 then
      ((term.getArg! 4).int?.isSome && readsLinearly factor (term.getArg! 5)) ||
        ((term.getArg! 5).int?.isSome && readsLinearly factor (term.getArg! 4))
    else false

/-- Where a leaf multiplies integers, the products of the bounds of one
factor with the bounds of the other, as nonnegative facts: for `0 ≤ x` and
`y ≤ u`, `0 ≤ x * (u - y)`, which bounds `x * y` linearly in the product as
an atom (the McCormick bounds of a product). A bound of a factor is an
integer comparison that reads it linearly. -/
private def assertProductBounds (goal : MVarId) : MetaM MVarId := goal.withContext do
  let mut products := integerProducts (← instantiateMVars (← goal.getType))
  let mut bounds : Array (Lean.Expr × Lean.Expr) := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    products := products ++ integerProducts type
    -- `x ≤ y` gives `0 ≤ y - x`, `x < y` gives `0 ≤ y - (x + 1)`.
    let isInt (e : Lean.Expr) := (e.getArg! 0).isConstOf ``Int
    if type.isAppOfArity ``LE.le 4 && isInt type then
      let difference ← mkAppM ``HSub.hSub #[type.getArg! 3, type.getArg! 2]
      bounds := bounds.push (difference, ← mkAppM ``Int.sub_nonneg_of_le #[decl.toExpr])
    else if type.isAppOfArity ``LT.lt 4 && isInt type then
      let lower ← mkAppM ``HAdd.hAdd #[type.getArg! 2, mkIntLit 1]
      let difference ← mkAppM ``HSub.hSub #[type.getArg! 3, lower]
      bounds := bounds.push (difference,
        ← mkAppM ``Int.sub_nonneg_of_le #[← mkAppM ``Int.add_one_le_of_lt #[decl.toExpr]])
  let boundsOf (factor : Lean.Expr) := bounds.filter fun (difference, _) =>
    readsLinearly factor difference
  let mut goal := goal
  let mut seen : Array (Lean.Expr × Lean.Expr) := #[]
  for (left, right) in products do
    if seen.contains (left, right) then continue
    seen := seen.push (left, right)
    for (_, leftProof) in boundsOf left do
      for (_, rightProof) in boundsOf right do
        let proof ← mkAppM ``Int.mul_nonneg #[leftProof, rightProof]
        let (_, next) ← (← goal.assert `product (← inferType proof) proof).intro1P
        goal := next
  return goal

/-- `grind` over a leaf's own facts, within an attempt's budget. The callees'
contracts, which the call rule has consumed, and the continuations bound in
the context are cleared first; `grind` would instantiate the contracts at
every term they match. An equation between arrays is taken element by
element (`arraysByElements`), and reads at equal positions are spelled alike
(`identifyReads`). -/
elab "leaner_denote_grind" : tactic => do
  let goal ← getMainGoal
  let victims ← goal.withContext do
    let mut found := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      if decl.isLet ||
          (← instantiateMVars decl.type).getUsedConstants.contains ``LeanerIR.Proofs.Satisfies then
        found := found.push decl.fvarId
    pure found
  let mut goal := goal
  for fvarId in victims.reverse do
    goal ← goal.tryClear fvarId
  let some elements ← arraysByElements (← abstractRankTables goal) | replaceMainGoal []
  replaceMainGoal [← assertProductBounds (← identifyReads elements)]
  evalTactic (← `(tactic| leaner_denote_budgeted grind))

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
  -- Substitute witness equations, of locals and of memory alike.
  progress := true
  while progress do
    progress := false
    let witness ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail then return none
        let ty ← instantiateMVars decl.type
        unless ty.isAppOfArity ``Eq 3 do return none
        if (ty.getArg! 2).isFVar then return some decl.fvarId
        if (ty.getArg! 1).isFVar then return some decl.fvarId
        return none
    if let some fvarId := witness then
      try
        goal ← Lean.Meta.subst goal fvarId
        progress := true
      catch _ => pure ()
  setGoals [goal]

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

/-- Each hypothesis `¬a = b` also as `¬b = a`, unless the context states
it: a rewrite with a hypothesis matches its equation only as stated. -/
private def symmetricDisequalities (goal : MVarId) : MetaM MVarId := do
  let facts ← goal.withContext do
    let mut stated : Array Lean.Expr := #[]
    for decl in ← getLCtx do
      unless decl.isImplementationDetail do stated := stated.push (← instantiateMVars decl.type)
    let mut facts : Array (Lean.Expr × Lean.Expr) := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some proposition := (← instantiateMVars decl.type).not? | continue
      let some (_, lhs, rhs) := proposition.eq? | continue
      if lhs == rhs then continue
      let symmetric := mkNot (← mkEq rhs lhs)
      if stated.contains symmetric then continue
      stated := stated.push symmetric
      facts := facts.push (symmetric, ← mkAppM ``Ne.symm #[decl.toExpr])
    pure facts
  let mut goal := goal
  for (type, proof) in facts do
    goal ← goal.withContext do
      let (_, next) ← (← goal.assert `symmetric type proof).intro1P
      pure next
  return goal

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
  replaceMainGoal [← symmetricDisequalities goal]

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

/-- An equation to a caller's value in a generic callee's view,
`x = toSkolem θ τ v`, restated in the caller's view, `ofSkolem θ τ x = v`
(`NTy.eq_toSkolem_iff`), where the caller's value is a local to substitute.
The equation is typed at the caller's family, which no lemma keyed on the
transport's family matches, so the rule is applied to its arguments. -/
private def transportEquation? (goal : MVarId) : MetaM (Option MVarId) := goal.withContext do
  let rule? (side : Lean.Expr) : Option (Name × Array Lean.Expr) :=
    match side.getAppFn.constName?, side.getAppArgs with
    | some ``LeanerIR.Proofs.Denote.NTy.toSkolem, #[family, θ, τ, v] =>
        some (``LeanerIR.Proofs.Denote.NTy.eq_toSkolem_iff, #[family, θ, τ, v])
    | some ``LeanerIR.Proofs.Denote.HList.toSkolem, #[family, θ, row, v] =>
        some (``LeanerIR.Proofs.Denote.HList.eq_toSkolem_iff, #[family, θ, row, v])
    | some ``LeanerIR.Proofs.Denote.variantCarrier.toSkolem, #[family, θ, names, rows, v] =>
        some (``LeanerIR.Proofs.Denote.variantCarrier.eq_toSkolem_iff, #[family, θ, names, rows, v])
    | _, _ => none
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let ty ← instantiateMVars decl.type
    let some (_, lhs, rhs) := ty.eq? | continue
    let (proof, rule) ← match rule? rhs, rule? lhs with
      | some rule, _ => pure (decl.toExpr, rule)
      | none, some rule => pure (← mkEqSymm decl.toExpr, rule)
      | none, none => continue
    let (name, arguments) := rule
    let other := if (rule? rhs).isSome then lhs else rhs
    let caller := arguments.back!
    let iff := mkAppN (mkConst name) (arguments.pop.push other |>.push caller)
    let restated ← mkAppM ``Iff.mp #[iff, proof]
    let (_, next) ← (← goal.assert decl.userName (← inferType restated) restated).intro1P
    return some (← next.tryClear decl.fvarId)
  return none

/-- An integer literal, also as a natural number cast. -/
private def intLiteral? (e : Lean.Expr) : Option Int :=
  e.int? <|> if e.isAppOfArity ``Nat.cast 3 then (e.appArg!.nat?).map Int.ofNat else none

/-- The integer local whose value a term reads, `x.val`. -/
private def valuedLocal? (e : Lean.Expr) : Option FVarId :=
  if e.isAppOfArity ``LeanerIR.SpecInt.val 3 then e.appArg!.fvarId? else none

/-- The tightest literal bounds the context states of each integer local,
strictly or not, with the hypotheses stating them. -/
private def integerBounds (goal : MVarId) :
    MetaM (Std.HashMap FVarId ((Int × Lean.Expr) × (Int × Lean.Expr))) := goal.withContext do
  let mut lower : Std.HashMap FVarId (Int × Lean.Expr) := {}
  let mut upper : Std.HashMap FVarId (Int × Lean.Expr) := {}
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    let strict := type.isAppOfArity ``LT.lt 4
    unless strict || type.isAppOfArity ``LE.le 4 do continue
    let (left, right) := (type.getArg! 2, type.getArg! 3)
    if let (some x, some c) := (valuedLocal? right, intLiteral? left) then
      let c := if strict then c + 1 else c
      unless (lower[x]?).any (c ≤ ·.1) do lower := lower.insert x (c, decl.toExpr)
    if let (some x, some c) := (valuedLocal? left, intLiteral? right) then
      let c := if strict then c - 1 else c
      unless (upper[x]?).any (·.1 ≤ c) do upper := upper.insert x (c, decl.toExpr)
  return lower.fold (init := {}) fun bounds x below =>
    match upper[x]? with
    | some above => bounds.insert x (below, above)
    | none => bounds

/-- An integer local the context pins to one value, bounded below and above
by literals, strictly or not, that leave one value: the local, the value,
and the two bounds. -/
private def pinnedInteger? (goal : MVarId) (tried : Array FVarId) :
    MetaM (Option (FVarId × Int × Lean.Expr × Lean.Expr)) := do
  for (x, ((c, below), (c', above))) in ← integerBounds goal do
    if !tried.contains x && c == c' then return some (x, c, below, above)
  return none

/-- Destructure an integer local and substitute its value, which an equation
`x.val = c` states. -/
private def substituteValue (goal : MVarId) (x equation : FVarId) : MetaM MVarId := do
  let #[case] ← goal.cases x | throwError "an integer has one constructor"
  let some (Lean.Expr.fvar val) := case.fields[0]? | throwError "an integer's value"
  let Lean.Expr.fvar equation := case.subst.get equation | throwError "the value's equation"
  case.mvarId.withContext do
    let type ← instantiateMVars (← equation.getType)
    let reduced ← mkEq (mkFVar val) (type.getArg! 2)
    let goal ← case.mvarId.replaceLocalDeclDefEq equation reduced
    Lean.Meta.subst goal equation

/-- The integer locals substituted by their pinned values so far, which a
step reads to see whether it fixed one. -/
private initialize pinnedCount : IO.Ref Nat ← IO.mkRef 0

/-- Destructure an integer local the context pins to a value and substitute
the value, so that the positions and lengths computed from it evaluate. -/
private def substitutePinned (goal : MVarId) (x : FVarId) (value : Int) (below above : Lean.Expr) :
    MetaM MVarId := do
  let proof ← goal.withContext do
    let read ← mkAppM ``LeanerIR.SpecInt.val #[mkFVar x]
    let statement ← mkEq read (toExpr value)
    let proof ← mkFreshExprMVar statement
    let some refutation ← proof.mvarId!.falseOrByContra | throwError "a pinned value"
    refutation.withContext do
      let negated := (← getLCtx).lastDecl.map (·.toExpr)
      Lean.Elab.Tactic.Omega.omega ([below, above] ++ negated.toList) refutation {}
    instantiateMVars proof
  let (equation, goal) ← (← goal.assert `pinned (← goal.withContext (inferType proof)) proof).intro1P
  let goal ← substituteValue goal x equation
  pinnedCount.modify (· + 1)
  return goal

/-- An equation fixing a vector local's elements, `x.values = e` or, as the
normalization states a literal, `l = x.values.toList`, with `x` not on the
other side: the equation and the local. -/
private def vectorEquation? (goal : MVarId) : MetaM (Option (FVarId × FVarId)) :=
  goal.withContext do
    let fixed (side other : Lean.Expr) : Option FVarId :=
      let values := if side.isAppOfArity ``Array.toList 2 then side.appArg! else side
      if values.isAppOfArity ``LeanerIR.SpecVector.values 2 then
        match values.appArg! with
        | .fvar x => if other.containsFVar x then none else some x
        | _ => none
      else none
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      unless ty.isAppOfArity ``Eq 3 do continue
      if let some x := fixed (ty.getArg! 1) (ty.getArg! 2) |>.orElse
          fun _ => fixed (ty.getArg! 2) (ty.getArg! 1) then
        return some (decl.fvarId, x)
    return none

/-- Destructure a vector local whose elements an equation fixes, and
substitute the elements: arrays have no decision procedure, so what is
known of them is known only where they are spelled alike. -/
private def substituteVector (goal : MVarId) (equation x : FVarId) : MetaM MVarId := do
  let #[case] ← goal.cases x | throwError "a vector has one constructor"
  let goal := case.mvarId
  let some (Lean.Expr.fvar values) := case.fields[0]? | throwError "a vector's elements"
  let Lean.Expr.fvar equation := case.subst.get equation | throwError "the vector's equation"
  goal.withContext do
    let type ← instantiateMVars (← equation.getType)
    let lhs ← whnfR (type.getArg! 1)
    let rhs ← whnfR (type.getArg! 2)
    let proof := Lean.mkFVar equation
    -- `values = e`, from whichever side and spelling the equation has.
    let (proof, other) ← if lhs == Lean.mkFVar values then pure (proof, type.getArg! 2)
      else if rhs == Lean.mkFVar values then pure (← mkEqSymm proof, type.getArg! 1)
      else if rhs.isAppOfArity ``Array.toList 2 then
        let literal := type.getArg! 1
        pure (← mkEqSymm (← mkAppM ``Iff.mpr #[← mkAppOptM ``List.toArray_eq_iff
          #[none, literal, Lean.mkFVar values], proof]), ← mkAppM ``List.toArray #[literal])
      else
        let literal := type.getArg! 2
        pure (← mkEqSymm (← mkAppM ``Iff.mpr #[← mkAppOptM ``List.toArray_eq_iff
          #[none, literal, Lean.mkFVar values], ← mkEqSymm proof]),
          ← mkAppM ``List.toArray #[literal])
    let statement ← mkEq (Lean.mkFVar values) other
    let (fixed, goal) ← (← goal.assert `elements statement proof).intro1P
    let goal ← goal.tryClear equation
    Lean.Meta.subst goal fixed

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
  let mut pinned : Array FVarId := #[]
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
        -- A vector local whose elements are fixed, a caller's value fixed in
        -- a callee's view, an integer pinned by its bounds, and a local whose
        -- encoding is fixed, such as a generic callee's result, are known
        -- wherever they occur.
        if let some (equation, vector) ← vectorEquation? goal then
          goal ← substituteVector goal equation vector
          progress := true
        else if let some restated ← transportEquation? goal then
          goal := restated
          progress := true
        else if let some (x, value, below, above) ← pinnedInteger? goal pinned then
          pinned := pinned.push x
          let saved ← saveState
          try goal ← substitutePinned goal x value below above
          catch _ => saved.restore
          progress := true
        else if let some fvarId ← encodingEquation? goal encoded then
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
        if e.isAppOfArity ``LeanerIR.Proofs.Denote.variantName 5 then
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
      -- The facts connected to the condition through shared variables; the
      -- negated condition is stated through the assigned metavariable.
      let facts ← (← getLCtx).foldlM (init := #[]) fun found decl => do
        if decl.isImplementationDetail then return found
        let type ← instantiateMVars decl.type
        unless ← isProp type do return found
        return found.push (decl.toExpr, (collectFVars {} type).fvarSet)
      let mut reached := atoms
      let mut relevant : Array Lean.Expr := #[]
      let mut remaining := facts
      let mut grown := true
      while grown do
        grown := false
        let mut rest := #[]
        for (fact, variables) in remaining do
          if variables.any fun x => reached.contains x then
            relevant := relevant.push fact
            reached := variables.foldl (fun found x => found.insert x) reached
            grown := true
          else rest := rest.push (fact, variables)
        remaining := rest
      Lean.Elab.Tactic.Omega.omega relevant.toList refutation {}
    return some (← instantiateMVars proof)
  catch _ => return none

/-- The discharger of the normalization's side conditions, over a leaf's
context: a condition about a variable simp introduced under a binder has no
fact in the context and is not attempted; otherwise a hypothesis states it
at reducible transparency, or omega proves it from the hypotheses connected
to it through shared variables, not from the whole context, whose size the
condition's cost would otherwise follow. -/
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
    (stx : Lean.TSyntax `tactic) (discharge? : Option Simp.Discharge) (target : Bool := true) :
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
  match ← simpGoal goal ctx simprocs discharge? (simplifyTarget := target)
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
private def normalizeContext (target : Bool) : TacticM Unit := withMainContext do
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
    (some (contextDischarge context (← IO.mkRef {}))) target
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

elab "leaner_denote_normalize_context" : tactic => normalizeContext true

/-- `leaner_denote_normalize_context` over the hypotheses only, the goal left
as it is. -/
elab "leaner_denote_normalize_hypotheses" : tactic => normalizeContext false

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

/-- Rewrite the hypotheses that read memory the leaf fixes (`m r k = v`)
with those reads, as `leaner_denote_memory_reads` does for the goal: a fact
the contract states over memory then speaks of the value the program read,
such as the variant a precondition fixes. -/
elab "leaner_denote_context_memory_reads" : tactic => do
  if (← getGoals).isEmpty then return
  let goal ← getMainGoal
  let (reads, targets) ← goal.withContext do
    let mut reads : Array (FVarId × Lean.Expr) := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      let some (_, lhs, _) := ty.eq? | continue
      let head := lhs.getAppFn
      unless head.isFVar && lhs.getAppNumArgs == 2 do continue
      if (← whnfR (← inferType head)).isAppOfArity ``LeanerIR.Proofs.Denote.Memory 1 then
        reads := reads.push (decl.fvarId, lhs)
    if reads.isEmpty then return (#[], #[])
    let mut targets := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail || reads.any (·.1 == decl.fvarId) then continue
      let ty ← instantiateMVars decl.type
      unless ← isProp ty do continue
      if reads.any fun (_, lhs) => (ty.find? (· == lhs)).isSome then
        targets := targets.push decl.fvarId
    return (reads.map (·.1), targets)
  if targets.isEmpty then throwError "no hypothesis reads memory the leaf fixes"
  simpMainAt (← `(tactic| simp only [lir_denote_norm])) reads targets false

/-- The hypotheses whose statement satisfies a predicate. -/
private def hypothesesWhere (goal : MVarId) (predicate : Lean.Expr → Bool) : MetaM (Array FVarId) :=
  goal.withContext do
    (← goal.getNondepPropHyps).filterM fun fvarId => do
      return predicate (← instantiateMVars (← fvarId.getType))

/-- `requires_of` of the closures whose target, weave, and captures a leaf's
goal sees: the target's entry in the table of declared preconditions, at the
arguments woven by the closure's mask. -/
macro "leaner_denote_behavior" : tactic => `(tactic|
  simp (disch := simp only [List.length_cons, List.length_nil, NRow.length]) only
    [NTy.encode_function, requiresOf_closureOf, Weave.composeList, HList.encode_cons,
      HList.encode_nil, Array.getD_eq_getD_getElem?, List.getElem?_toArray,
      List.getElem?_cons_succ, List.getElem?_cons_zero, Option.getD_some, lir_denote_norm])

/-- Whether a leaf's tactic closes a goal, leaving the goal list as it was. -/
private def closesBy (goal : MVarId) (tactic : TSyntax `tactic) : TacticM Bool := do
  let saved ← getGoals
  let state ← saveState
  try
    setGoals [goal]
    Lean.Elab.Tactic.withoutRecover (evalTactic tactic)
    let closed := (← getGoals).isEmpty
    unless closed do state.restore
    setGoals saved
    return closed
  catch _ =>
    state.restore
    setGoals saved
    return false

/-- The unfolding theorem (`f.spec.unfold`) of a full application of a
recursive specification function: the definition's leading parameters
(what its body reads besides its arguments), then the bundled arguments. -/
private def specUnfolding? (env : Environment) (e : Lean.Expr) : Option Name :=
  match e.getAppFn with
  | .const name _ =>
      if name matches .str _ "spec" then
        match env.find? (Name.str name "unfold") with
        | some info => if arity info.type == e.getAppNumArgs && e.getAppNumArgs > 0
            then some (Name.str name "unfold") else none
        | none => none
      else none
  | _ => none
where
  arity : Lean.Expr → Nat
    | .forallE _ _ body _ => arity body + 1
    | _ => 0

/-- The bounds of the elements the unfoldings in context read: those of
earlier rounds too, as a substitution since has spelled them. -/
private def boundUnfoldings (goal : MVarId) : TacticM MVarId := goal.withContext do
  let mut goal := goal
  let mut known : Array Lean.Expr := #[]
  let mut unfoldings : Array Lean.Expr := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    known := known.push type
    if decl.userName == `unfolded then unfoldings := unfoldings.push type
  for proof in ← elementBounds unfoldings do
    let type ← goal.withContext (instantiateMVars (← inferType proof))
    if known.contains type then continue
    known := known.push type
    (_, goal) ← (← goal.assert `bounds type proof).intro1P
  return goal

/-- Instances of the unfolding theorems of the recursive specification
functions (`f.spec.unfold`) at the applications a leaf holds, once, kept
where the context decides the guards of the unfolding: what a solver's
definitional axiom for the function gives at its trigger terms. None where
the leaf applies none; else the goal left, if any, and how many instances
were kept. -/
private def unfoldSpecsOnce (goal : MVarId) (groundOnly := false) :
    TacticM (Option (Option MVarId × Nat)) := do
  let unfolding? := specUnfolding? (← getEnv)
  let (applications, previous) ← goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    -- An application already unfolded is not unfolded again.
    let mut unfolded := #[]
    let mut previous := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let type ← instantiateMVars decl.type
      terms := terms.push type
      if decl.userName == `unfolded then
        previous := previous.push decl.fvarId
        if let some (_, lhs, _) := type.eq? then unfolded := unfolded.push lhs
    return ((sitesWhere (fun e => (unfolding? e).isSome && !e.hasLooseBVars &&
        !(groundOnly && e.hasFVar)) terms).filter (!unfolded.contains ·), previous)
  if applications.isEmpty then return none
  let mut goal := goal
  let mut facts := #[]
  for application in applications do
    let some unfold := unfolding? application | continue
    let proof := mkAppN (mkConst unfold) application.getAppArgs
    let (fact, next) ← goal.withContext do
      (← goal.assert `unfolded (← inferType proof) proof).intro1P
    goal := next
    facts := facts.push fact
  -- The guard of each instance, the condition its body branches on first,
  -- is decided over the facts connected to it by shared variables, and the
  -- instance rewritten to the branch the guard takes. An instance whose
  -- guard the context leaves open is dropped: rewriting by it would only
  -- unfold the function again.
  let debug := leaner.denoteDebug.get (← getOptions)
  let decisions ← goal.withContext do
    let candidates ← (← getLCtx).decls.toArray.filterMapM fun
      | some other => do
          if other.isImplementationDetail || facts.contains other.fvarId then return none
          let otherType ← instantiateMVars other.type
          return some (other.fvarId, (Lean.collectFVars {} otherType).fvarIds)
      | none => pure none
    facts.mapM fun fact => do
      let type ← instantiateMVars (← fact.getType)
      let some (_, _, rhs) := type.eq? | return (fact, some none)
      unless rhs.isAppOfArity ``ite 5 || rhs.isAppOfArity ``dite 5 do return (fact, some none)
      let guard := rhs.getArg! 1
      let mut variables := (Lean.collectFVars {} guard).fvarIds
      let mut related : Array FVarId := #[]
      let mut changed := true
      while changed do
        changed := false
        for (id, uses) in candidates do
          if related.contains id then continue
          if variables.contains id || uses.any variables.contains then
            related := related.push id
            for use in uses.push id do
              unless variables.contains use do variables := variables.push use
            changed := true
      let unrelated := facts ++ candidates.filterMap fun (id, _) =>
        if related.contains id then none else some id
      for (statement, holds) in #[(guard, true), (mkNot guard, false)] do
        let proof ← mkFreshExprMVar statement
        let some reduced ← observing? (proof.mvarId!.tryClearMany unrelated) | continue
        if ← closesBy reduced (← `(tactic|
            (try simp only [lir_denote_norm, beq_iff_eq, decide_eq_true_eq]) <;> omega)) then
          return (fact, some (some (← instantiateMVars proof, holds)))
      if debug then
        IO.println s!"    unfolded (dropped): {((toString (← ppExpr type)).replace "\n" " ").take 300}"
      return (fact, none)
  let mut kept := #[]
  for (fact, decision) in decisions do
    match decision with
    | none => goal ← goal.clear fact
    | some none => kept := kept.push fact
    | some (some (decided, holds)) =>
        let (branched, next) ← goal.withContext do
          let type ← instantiateMVars (← fact.getType)
          let some (_, lhs, rhs) := type.eq? | throwError "an unfolding is an equation"
          let arguments := rhs.getAppArgs
          let rule := if rhs.isAppOf ``ite then (if holds then ``if_pos else ``if_neg)
            else (if holds then ``dif_pos else ``dif_neg)
          let branch ← mkAppOptM rule #[arguments[1]!, arguments[2]!, decided, arguments[0]!,
            arguments[3]!, arguments[4]!]
          let proof ← mkEqTrans (.fvar fact) branch
          let some (_, _, taken) := (← instantiateMVars (← inferType proof)).eq?
            | throwError "a branch is an equation"
          (← goal.assert `unfolded (← mkEq lhs taken.headBeta) proof).intro1P
        goal ← next.clear fact
        kept := kept.push branched
  if kept.isEmpty then
    let bounded ← boundUnfoldings goal
    return some (some bounded, 0)
  setGoals [goal]
  -- The conditions of the taken branch the context decides, such as the
  -- decrease a recursive application checks, are decided as well.
  let some simplified ← simpAt goal (← `(tactic| simp (disch := omega) only
      [lir_denote_norm, dif_pos, dif_neg, if_pos, if_neg, Int.add_sub_cancel, Int.reduceSub,
        Int.reduceAdd, Int.reduceToNat])) #[] kept false
    | return some (none, kept.size)
  if debug then
    simplified.withContext do
      for decl in ← getLCtx do
        if decl.userName == `unfolded && !previous.contains decl.fvarId then
          let shown := toString (← ppExpr (← instantiateMVars decl.type))
          IO.println s!"    unfolded (kept): {(shown.replace "\n" " ").take 300}"
  return some (some (← boundUnfoldings simplified), kept.size)

/-- One round of `unfoldSpecsOnce`, then the applications at closed
arguments unfolded down to their base cases: evaluation, whose guards the
values decide. -/
elab "leaner_denote_unfold_specs" : tactic => do
  let some (goal?, _) ← unfoldSpecsOnce (← getMainGoal)
    | throwError "the leaf applies no recursive specification function"
  let mut goal? := goal?
  for _ in [0:32] do
    let some goal := goal? | break
    let some (next, kept) ← unfoldSpecsOnce goal (groundOnly := true) | break
    goal? := next
    if kept == 0 then break
  replaceMainGoal goal?.toList

/-- The recursive specification functions a leaf applies, unfolded round by
round while a round keeps an instance, as at arguments the leaf fixes, each
application unfolds to further ones down to its base case. -/
elab "leaner_denote_specs_to_base" : tactic => do
  let mut goal ← getMainGoal
  let mut progress := false
  for _ in [0:32] do
    let some (goal?, kept) ← unfoldSpecsOnce goal | break
    let some next := goal? | replaceMainGoal []; return
    goal := next
    if kept == 0 then break
    progress := true
  unless progress do throwError "no recursive specification function unfolds at the leaf's values"
  replaceMainGoal [goal]

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
theorem keysRead_encode_enum [Carriers] {α : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (source : StructHandle) (arguments : NRow) (name : String)
    (entry : StructHandle) (entryArguments : NRow) (key : NTy) (rest : NRow)
    (distinct : [name].Nodup)
    (value : variantCarrier [name]
      (.cons (.cons (NTy.vector (NTy.struct entry entryArguments (.cons key rest))) .nil) .nil))
    (roundTrip : ∀ element : key.carrier, write (read (key.encode element)) = key.encode element) :
    Maps.KeysRead read write
      (NTy.encode (.enum source arguments [name]
        (.cons (.cons (NTy.vector (NTy.struct entry entryArguments (.cons key rest))) .nil) .nil)
        distinct) value) := by
  cases value with
  | inl fields =>
      simp only [NTy.encode_enum_inl, HList.encode_cons, HList.encode_nil, NTy.encode_vector]
      apply Maps.keysRead_map
      intro element
      simp only [NTy.codec_encode, NTy.encode_struct, HList.encode_cons]
      exact roundTrip element.1
  | inr empty => exact empty.elim

/-- The same for a map held as the vector of entries of a struct. -/
theorem keysRead_encode_struct [Carriers] {α : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (source : StructHandle) (arguments : NRow) (entry : StructHandle)
    (entryArguments : NRow) (key : NTy) (rest : NRow)
    (value : HList (.cons (NTy.vector (NTy.struct entry entryArguments (.cons key rest))) .nil))
    (roundTrip : ∀ element : key.carrier, write (read (key.encode element)) = key.encode element) :
    Maps.KeysRead read write
      (NTy.encode (.struct source arguments
        (.cons (NTy.vector (NTy.struct entry entryArguments (.cons key rest))) .nil)) value) := by
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
  -- A constructor equality can make these reads computational. Use only
  -- literal construction equations here, before generating rank/order facts;
  -- those facts are unnecessary (and costly) for a map whose entries are known.
  let concrete ← (← getMainGoal).withContext do
    let mut equations := #[]
    for fvarId in ← (← getMainGoal).getNondepPropHyps do
      let type ← instantiateMVars (← fvarId.getType)
      let some (_, _, rhs) := type.eq? | continue
      if rhs.isAppOfArity ``LeanerIR.Maps.Layout.build 2 &&
          (rhs.getArg! 1).listLit?.isSome then
        equations := equations.push fvarId
    return equations
  unless concrete.isEmpty do
    let goal ← getMainGoal
    let hypotheses ← (← goal.getNondepPropHyps).filterM fun fvarId => do
      let type ← instantiateMVars (← fvarId.getType)
      return !type.isForall && (type.find? fun e =>
        e.isAppOf ``LeanerIR.Maps.keyAt || e.isAppOf ``LeanerIR.Maps.rank ||
        e.isAppOf ``LeanerIR.Maps.valueAt || e.isAppOf ``LeanerIR.Maps.size ||
        e.isAppOf ``LeanerIR.Maps.remove).isSome
    simpMainAt (← `(tactic| simp only [lir_denote_norm, Prod.fst, Prod.snd]))
      concrete hypotheses true
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
the instance for as long as the goal is large, then fails anyway. The
kernel also evaluates what elaboration leaves stuck, such as string
comparisons and well-founded definitions like the structural order. -/
elab "leaner_denote_decide" : tactic => do
  let target ← instantiateMVars (← (← getMainGoal).getType)
  if target.hasFVar then throwError "the goal is not closed"
  evalTactic (← `(tactic| first | decide | decide +kernel))

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

/-- The instances of a hypothesis quantified over integers whose premises
bound its binders by literals to a few values: the binders range over the
box of the literal bounds, an instance whose premise omega refutes is
vacuous and dropped, and the premises omega does not decide stay premises.
None when a binder is not an integer or the box is not small. -/
private def literalRangeInstances (goal : MVarId) (hypothesis : Lean.Expr) :
    MetaM (Option (Array Lean.Expr)) := goal.withContext do
  let type ← instantiateMVars (← inferType hypothesis)
  let (mvars, _, _) ← forallMetaTelescope type
  let mut binders := #[]
  let mut premises := #[]
  for m in mvars do
    let binderType ← whnfR (← instantiateMVars (← m.mvarId!.getType))
    if binderType.isConstOf ``Int then binders := binders.push m
    else if ← isProp binderType then premises := premises.push binderType
    else return none
  if binders.isEmpty || binders.size > 2 then return none
  -- The literal bounds the premises state of the binders.
  let bound (e : Lean.Expr) := binders.contains e
  let mut lower : Option Int := none
  let mut upper : Option Int := none
  for premise in premises do
    let strict := premise.isAppOfArity ``LT.lt 4
    unless strict || premise.isAppOfArity ``LE.le 4 do continue
    let (left, right) := (premise.getArg! 2, premise.getArg! 3)
    if let (true, some c) := (bound right, intLiteral? left) then
      let c := if strict then c + 1 else c
      lower := some (lower.elim c (min c))
    if let (true, some c) := (bound left, intLiteral? right) then
      let c := if strict then c - 1 else c
      upper := some (upper.elim c (max c))
  let (some lo, some hi) := (lower, upper) | return none
  if hi < lo then return some #[]
  let width := (hi - lo + 1).toNat
  if width > 4 then return none
  let mut instances := #[]
  for tuple in [0:width ^ binders.size] do
    let saved ← saveState
    let (mvars, _, _) ← forallMetaTelescope type
    let mut rest := tuple
    let mut vacuous := false
    for m in mvars do
      let binderType ← whnfR (← instantiateMVars (← m.mvarId!.getType))
      if binderType.isConstOf ``Int then
        m.mvarId!.assign (toExpr (lo + Int.ofNat (rest % width)))
        rest := rest / width
      else
        let premise ← instantiateMVars binderType
        unless premise.hasFVar || premise.hasMVar do
          let negated ← mkFreshExprMVar (mkNot premise)
          if (← observing? (decideByOmega negated.mvarId!)).isSome then vacuous := true
          else try decideByOmega m.mvarId! catch _ => pure ()
    if vacuous then saved.restore; continue
    let proof ← bindOpenPremises hypothesis mvars 0 #[]
    saved.restore
    instances := instances.push proof
  return some instances

/-- Expand each hypothesis quantified over integers in a small literal range
into its instances, which the normalization then computes: a quantifier
over a literal vector's positions becomes facts about its elements. -/
elab "leaner_denote_expand_ranges" : tactic => do
  let mut goal ← getMainGoal
  let mut expanded := false
  for decl in ← goal.withContext getLCtx do
    if decl.isImplementationDetail then continue
    let type ← goal.withContext (instantiateMVars decl.type)
    unless type.isForall do continue
    unless ← goal.withContext (isProp type) do continue
    let some instances ← literalRangeInstances goal decl.toExpr | continue
    for proof in instances do
      let statement ← goal.withContext (inferType proof)
      let (_, next) ← (← goal.assert `instance statement proof).intro1P
      goal := next
    goal ← goal.tryClear decl.fvarId
    expanded := true
  unless expanded do throwError "no quantifier over a small literal range"
  replaceMainGoal [goal]

/-- The integer positions of a goal without a value certificate, such as the
binders a quantified goal introduced (`Int.toNat x`), each with the range
the context bounds it to: the tightest of the literal bounds the context
states of its integers that omega proves of it. None unless every position
is bounded and the ranges hold few tuples. -/
private def positionBox? (goal : MVarId) : MetaM (Option (Array (FVarId × Int × Int))) :=
  goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    for decl in ← getLCtx do
      unless decl.isImplementationDetail do terms := terms.push (← instantiateMVars decl.type)
    let mut positions : Array FVarId := #[]
    for site in sitesWhere (·.isAppOfArity ``Int.toNat 1) terms do
      if let .fvar x := site.appArg! then
        unless positions.contains x do positions := positions.push x
    if positions.isEmpty || positions.size > 2 then return none
    -- The literal bounds stated of any integer local, as candidates.
    let mut lowers : Array Int := #[]
    let mut uppers : Array Int := #[]
    for term in terms do
      let strict := term.isAppOfArity ``LT.lt 4
      unless strict || term.isAppOfArity ``LE.le 4 do continue
      let (left, right) := (term.getArg! 2, term.getArg! 3)
      if let (some _, some c) := (right.fvarId?, intLiteral? left) then
        lowers := lowers.push (if strict then c + 1 else c)
      if let (some _, some c) := (left.fvarId?, intLiteral? right) then
        uppers := uppers.push (if strict then c - 1 else c)
    let proves (statement : Lean.Expr) : MetaM Bool := do
      let proof ← mkFreshExprMVar statement
      return (← observing? (decideByOmega proof.mvarId!)).isSome
    let mut ranges := #[]
    for x in positions do
      let mut below := none
      for c in lowers.qsort (· > ·) do
        if ← proves (← mkAppM ``LE.le #[toExpr c, mkFVar x]) then below := some c; break
      let mut above := none
      for c in uppers.qsort (· < ·) do
        if ← proves (← mkAppM ``LE.le #[mkFVar x, toExpr c]) then above := some c; break
      let (some lo, some hi) := (below, above) | return none
      ranges := ranges.push (x, lo, hi)
    let tuples := ranges.foldl (fun n (_, lo, hi) => n * (hi - lo + 1).toNat) 1
    if tuples == 0 || tuples > 16 then return none
    return some ranges

/-- The cases of a disjunction of equations of a local to values, each with
the equation substituted: a certified integer's by `substituteValue`, a
plain integer's by substitution. -/
private partial def splitValueCases (goal : MVarId) (cases : FVarId) (certified : Option FVarId) :
    MetaM (List MVarId) := do
  let type ← goal.withContext do instantiateMVars (← cases.getType)
  let substitute (goal : MVarId) (equation : FVarId) : MetaM MVarId :=
    match certified with
    | some x => substituteValue goal x equation
    | none => Lean.Meta.subst goal equation
  if type.isAppOfArity ``Or 2 then
    let #[left, right] ← goal.cases cases | throwError "a disjunction has two cases"
    let some (Lean.Expr.fvar equation) := left.fields[0]? | throwError "a case's equation"
    let some (Lean.Expr.fvar rest) := right.fields[0]? | throwError "a case's rest"
    return (← substitute left.mvarId equation) :: (← splitValueCases right.mvarId rest certified)
  return [← substitute goal cases]

/-- Split on the values of integers that range over a few literals and give
positions: an integer local the context bounds by literals, where it gives
a position (`x.val.toNat`) or bounds a quantified hypothesis, or else the
plain integer positions in a small literal box, such as a quantified
goal's binders. In each case the integers are those values, and the
positions and ranges they give are literals. -/
elab "leaner_denote_split_range" : tactic => do
  let goal ← getMainGoal
  let bounds ← integerBounds goal
  let certified? ← goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    for decl in ← getLCtx do
      unless decl.isImplementationDetail do terms := terms.push (← instantiateMVars decl.type)
    let positions (x : FVarId) : Bool := terms.any fun term =>
      (term.find? fun e => e.isAppOfArity ``Int.toNat 1 && valuedLocal? e.appArg! == some x).isSome ||
        (term.isForall && (term.find? fun e => valuedLocal? e == some x).isSome)
    -- An argument of a recursive specification function, which unfolds
    -- to its base case at a literal.
    let env ← getEnv
    let specArgument (x : FVarId) : Bool := terms.any fun term =>
      (term.find? fun e => match e.getAppFn with
        | .const name _ => name matches .str _ "spec" && (env.find? (Name.str name "unfold")).isSome &&
            (e.find? fun e => valuedLocal? e == some x).isSome
        | _ => false).isSome
    -- A position is split over a few values; a specification function's
    -- argument, whose cases evaluate, over as many as a box of positions.
    for (x, ((lo, _), (hi, _))) in bounds do
      if hi - lo < 4 && lo < hi && positions x || hi - lo < 16 && lo ≤ hi && specArgument x then
        return some (x, lo, hi)
    return none
  let (disjuncts, certified) ← goal.withContext do
    match certified? with
    | some (x, lo, hi) =>
        let read ← mkAppM ``LeanerIR.SpecInt.val #[mkFVar x]
        let disjuncts ← (List.range ((hi - lo).toNat + 1)).mapM fun k =>
          mkEq read (toExpr (lo + Int.ofNat k))
        pure (disjuncts, some x)
    | none =>
        -- One position at a time; a case splits on the next.
        let some ranges ← positionBox? goal
          | throwError "no integer in a small literal range gives a position"
        let some (x, lo, hi) := ranges[0]? | throwError "no position"
        let disjuncts ← (List.range ((hi - lo).toNat + 1)).mapM fun k =>
          mkEq (mkFVar x) (toExpr (lo + Int.ofNat k))
        pure (disjuncts, none)
  let some last := disjuncts.getLast? | throwError "an empty range"
  let disjunction := disjuncts.dropLast.foldr (fun e rest => mkApp2 (mkConst ``Or) e rest) last
  let proof ← goal.withContext do
    let proof ← mkFreshExprMVar disjunction
    decideByOmega proof.mvarId!
    instantiateMVars proof
  let (cases, goal) ← (← goal.assert `range disjunction proof).intro1P
  replaceMainGoal (← splitValueCases goal cases certified)

/-- Decide a callee's integer result where the call returns, when its
contract bounds it by literals to a few values that give positions: the
continuation is split on the value, and a case the contract's facts refute
at its value, instance by instance, is closed. What follows the call then
reads the result as a literal, in every leaf once. Fails when there is no
such result. -/
elab "leaner_denote_call_result_cases" : tactic => withoutRecover do
  let pinnedBefore ← pinnedCount.get
  let (_, goal) ← (← getMainGoal).intros
  -- The facts the continuation's proof terms depend on, conjunct by
  -- conjunct, as copies the normalization may rewrite.
  let mut goal := goal
  let mut pending ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    (← getLCtx).decls.toArray.filterMapM fun
      | some decl => do
          if decl.isImplementationDetail || !target.containsFVar decl.fvarId then return none
          return if ← isProp decl.type then some decl.toExpr else none
      | none => pure none
  while let some fact := pending.back? do
    pending := pending.pop
    let type ← goal.withContext do whnfR (← instantiateMVars (← inferType fact))
    unless ← goal.withContext (isProp type) do continue
    if type.isAppOfArity ``And 2 then
      pending := pending.push (← goal.withContext (mkAppM ``And.left #[fact]))
      pending := pending.push (← goal.withContext (mkAppM ``And.right #[fact]))
    else
      let (_, next) ← (← goal.assert `fact type fact).intro1P
      goal := next
  setGoals [goal]
  -- Two rounds: the first round's normalization exposes what the second
  -- substitutes, such as the parts of a transported value, whose types the
  -- callee spells at its family.
  for _ in [0:2] do
    evalTactic (← `(tactic| all_goals leaner_denote_split_hypotheses))
    evalTactic (← `(tactic| all_goals leaner_denote_subst_vars))
    evalTactic (← `(tactic| all_goals leaner_denote_reduce_projections))
    evalTactic (← `(tactic| leaner_denote_canonical_families))
    evalTactic (← `(tactic| all_goals leaner_denote_normalize_hypotheses))
  let [normal] ← getGoals | return
  -- The results the contract pins are substituted; one in a few values is
  -- split on.
  let mut normal := normal
  let mut tried := #[]
  while let some (x, value, below, above) ← pinnedInteger? normal tried do
    tried := tried.push x
    let saved ← saveState
    try normal ← substitutePinned normal x value below above
    catch _ => saved.restore
  setGoals [normal]
  -- Without a pinned result, one in a few values is split on, or the step
  -- leaves the continuation as it was.
  if (← pinnedCount.get) == pinnedBefore then
    evalTactic (← `(tactic| leaner_denote_split_range))
  let cases ← getGoals
  let mut feasible := #[]
  for case in cases do
    setGoals [case]
    evalTactic (← `(tactic| leaner_denote_reduce_projections))
    evalTactic (← `(tactic| leaner_denote_normalize_hypotheses))
    evalTactic (← `(tactic|
      all_goals (try (leaner_denote_expand_ranges; leaner_denote_normalize_hypotheses))))
    feasible := feasible ++ (← getGoals).toArray
  setGoals feasible.toList

/-- Whether a leaf holds a value in a generic callee's view: a transport,
or a type spelled at a family a call's type arguments induce. Fails when it
holds none. -/
elab "leaner_denote_holds_transport" : tactic => do
  let goal ← getMainGoal
  let held ← goal.withContext do
    let transports (e : Lean.Expr) : Bool := (e.find? fun t =>
      t.isConstOf ``LeanerIR.Proofs.Denote.NTy.toSkolem ||
      t.isConstOf ``LeanerIR.Proofs.Denote.HList.toSkolem ||
      t.isConstOf ``LeanerIR.Proofs.Denote.variantCarrier.toSkolem ||
      t.isConstOf ``LeanerIR.Proofs.Denote.NTy.ofSkolem ||
      t.isConstOf ``LeanerIR.Proofs.Denote.HList.ofSkolem ||
      t.isConstOf ``LeanerIR.Proofs.Denote.Carriers.instantiate).isSome
    if transports (← instantiateMVars (← goal.getType)) then return true
    (← getLCtx).anyM fun decl => do
      if decl.isImplementationDetail then return false
      return transports (← instantiateMVars decl.type)
  unless held do throwError "the leaf holds no value in a callee's view"

/-- Substitute and renormalize until a substitution leaves the goal as it
was: a renormalization exposes equations to substitute, such as the parts
of a caller's value a transport equation fixes. At most three rounds. -/
elab "leaner_denote_settle" : tactic => do
  for _ in [0:3] do
    if (← getGoals).isEmpty then return
    let before ← (← getGoals).mapM fun goal => goalContent goal
    -- The normalization spells a transported value's types at the callee's
    -- family; canonical, they meet the caller's spellings.
    evalTactic (← `(tactic| leaner_denote_canonical_families))
    evalTactic (← `(tactic| all_goals leaner_denote_split_hypotheses))
    evalTactic (← `(tactic| all_goals leaner_denote_subst_vars))
    if (← (← getGoals).mapM fun goal => goalContent goal) == before then return
    evalTactic (← `(tactic| all_goals leaner_denote_normalize_context))

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
  all_goals try leaner_denote_timed "p-reads" leaner_denote_context_memory_reads
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
  all_goals leaner_denote_timed "p-settle" leaner_denote_settle
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
  -- A read of memory the context fixes is that value, so that a match on
  -- it is decided, or is the program's; resolving one is progress enough.
  let beforeReads ← getMainGoal
  evalTactic (← `(tactic| try leaner_denote_memory_reads))
  if (← getGoals).length != 1 || (← getMainGoal) != beforeReads then return
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

/-- A witness of a type along its pairs: a pair of witnesses, the unit, or
an unknown. -/
private partial def assembleWitness (type : Lean.Expr) : MetaM Lean.Expr := do
  let type ← whnfD type
  if type.isAppOfArity ``Prod 2 then
    mkAppM ``Prod.mk #[← assembleWitness (type.getArg! 0), ← assembleWitness (type.getArg! 1)]
  else if type.isConstOf ``Unit || type.isAppOfArity ``PUnit 0 then
    pure (mkConst ``Unit.unit)
  else mkFreshExprMVar type

/-- The conjuncts of a proposition. -/
private partial def conjunctsOf (e : Lean.Expr) : Array Lean.Expr :=
  if e.isAppOfArity ``And 2 then conjunctsOf (e.getArg! 0) ++ conjunctsOf (e.getArg! 1)
  else #[e]

/-- A goal stating, perhaps among other conjuncts, an existential over a
value of pairs whose body equates the parts of the value to terms, as a
value's typing states it field by field: each such witness assembled from
those terms by unifying each equation. What the bodies and the other
conjuncts state is left to prove. -/
partial def assembleWitnesses (goal : MVarId) : MetaM (Array MVarId × Bool) := goal.withContext do
  let target ← whnfR (← instantiateMVars (← goal.getType))
  if target.isAppOfArity ``LeanerIR.Proofs.Obligation 4 then
    return ← assembleWitnesses (← goal.replaceTargetDefEq (target.getArg! 3))
  if target.isAppOfArity ``And 2 then
    let [left, right] ← goal.apply (mkConst ``And.intro) | return (#[goal], false)
    let (lefts, assembledLeft) ← assembleWitnesses left
    let (rights, assembledRight) ← assembleWitnesses right
    return (lefts ++ rights, assembledLeft || assembledRight)
  let some (domain, body) := target.app2? ``Exists | return (#[goal], false)
  let witness ← assembleWitness domain
  let stated ← whnfR (mkApp body witness).headBeta
  for conjunct in conjunctsOf stated do
    if let some (_, left, right) := conjunct.eq? then
      discard <| isDefEq left right
  let witness ← instantiateMVars witness
  if witness.hasExprMVar then return (#[goal], false)
  return (#[← goal.existsIntro witness], true)

elab "leaner_denote_assembled_witness" : tactic => do
  let saved ← saveState
  let (goals, assembled) ← assembleWitnesses (← getMainGoal)
  unless assembled do
    saved.restore
    throwError "no existential's body determines its witness"
  replaceMainGoal goals.toList

/-- The initialized locals of a program point's value, each with its type:
the entries of the environment a `Flow.value` carries, and the carrier
family they live in. -/
private partial def pointLocals (value : Lean.Expr) :
    TacticM (Option (Lean.Expr × Array (Lean.Expr × Lean.Expr))) := do
  let flow ← instantiateMVars (← inferType value)
  unless flow.isAppOfArity ``Flow 5 do return none
  let carriers := mkApp2 (mkConst ``Skolems.toCarriers) (flow.getArg! 0) (flow.getArg! 1)
  let value ← whnfR value
  unless value.isAppOf ``LeanerIR.Proofs.Denote.Flow.value do return none
  let rec walk (row env : Lean.Expr) (found : Array (Lean.Expr × Lean.Expr)) :
      TacticM (Array (Lean.Expr × Lean.Expr)) := do
    let row ← whnfR row
    unless row.isAppOfArity ``LeanerIR.Proofs.Denote.NRow.cons 2 do return found
    let env ← whnfR env
    unless env.isAppOfArity ``Prod.mk 4 do return found
    let entry ← whnfR (env.getArg! 2)
    let found := if entry.isAppOfArity ``Option.some 2 then
        found.push (row.getArg! 0, entry.getArg! 1) else found
    walk (row.getArg! 1) (env.getArg! 3) found
  return some (carriers, ← walk (flow.getArg! 3) value.appArg! #[])

/-- The accessor reading a label's copy of a mutable parameter in a binder's
domain: none for the runtime values themselves; no accessor for a domain no
label binds. -/
private def domainAccessor? (domain : Lean.Expr) : Option (Option Name) :=
  if domain.isConstOf ``LeanerIR.RuntimeValue then some none
  else if domain.isConstOf ``Int then some (some ``LeanerIR.RuntimeValue.asInt)
  else if domain.isConstOf ``Bool then some (some ``LeanerIR.RuntimeValue.asBool)
  else if domain.isConstOf ``String then some (some ``LeanerIR.RuntimeValue.asString)
  else none

/-- The value of a local as a contract reads it in a binder's domain: its
runtime encoding — a reference's by its referent's current value — through
the accessor of the domain. `none` for a domain no label binds. -/
private def localInDomain (carriers τ value domain : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let some accessor := domainAccessor? domain | return none
  let τ ← whnfR τ
  let (τ, value) ← if τ.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.ref 1 then
      let value ← whnfR value
      let current := if value.isAppOfArity ``Prod.mk 4 then value.getArg! 2
        else mkProj ``Prod 0 value
      pure (τ.getArg! 0, current)
    else pure (τ, value)
  let encoded := mkApp3 (mkConst ``LeanerIR.Proofs.Denote.NTy.encode) carriers τ value
  return some (accessor.elim encoded fun name => mkApp (mkConst name) encoded)

/-- The program points of the context, each a value and a state. -/
private def programPoints : MetaM (Array (Lean.Expr × Lean.Expr)) := do
  let mut points := #[]
  for decl in ← getLCtx do
    if decl.type.isAppOfArity ``ProgramPoint 7 then
      let type ← instantiateMVars decl.type
      points := points.push (type.getArg! 5, type.getArg! 6)
  return points

/-- An existential goal whose body some hypothesis states at a witness, or
that a bound of the body, a variable of the context, or a written position
witnesses. The conjuncts reading the binder are decided at the witness
first, which rejects a candidate cheaply, a nested existential witnessed in
turn; then the conjuncts not reading it. -/
syntax "leaner_denote_witness" : tactic


/-- The body of an existential at a witness, proved: split into conjuncts,
those marked in `first` first (a candidate fails fast on the conjuncts
reading its binder), each by a nested witness, by `premise`, or by one side
of a disjunction; normalized before, where asked. The proof, or `none` with
the state restored. -/
private def decideInstance (stated : Lean.Expr) (premise : Lean.TSyntax `tactic)
    (first : Array Bool := #[]) (normalize : Bool := false) : TacticM (Option Lean.Expr) := do
  let saved ← saveState
  try
    let instantiated ← mkFreshExprMVar stated
    setGoals [instantiated.mvarId!]
    if normalize then
      evalTactic (← `(tactic| try simp only [lir_denote, lir_denote_norm, lir_denote_eval,
        Prod.fst, Prod.snd]))
    evalTactic (← `(tactic| all_goals repeat' apply And.intro))
    let split := (← getGoals).toArray
    let ordered := if split.size == first.size then
        ((split.zip first).qsort fun (_, a) (_, b) => a && !b).map (·.1)
      else split
    let decide ← `(tactic| first
      | leaner_denote_witness
      | $premise:tactic
      | (apply Or.inl; repeat' apply And.intro; all_goals $premise:tactic)
      | (apply Or.inr; repeat' apply And.intro; all_goals $premise:tactic))
    for conjunct in ordered do
      setGoals [conjunct]
      evalTactic decide
      unless (← getGoals).isEmpty do throwError "a conjunct is left"
    return some (← instantiateMVars instantiated)
  catch _ =>
    restoreState saved
    return none

/-- The goal closed by `proof` of its body at the witnesses `chosen`, the
outermost binder first. -/
private def assignWitnessed (goal : MVarId) (others : List MVarId)
    (chosen : Array (Lean.Expr × Lean.Expr × Lean.Expr)) (proof : Lean.Expr) : TacticM Unit := do
  let mut proof := proof
  for (domain, predicate, witness) in chosen.reverse do
    proof ← mkAppOptM ``Exists.intro #[domain, predicate, witness, proof]
  goal.assign proof
  setGoals others

/-- An existential goal over a state label — a memory and copies of the
mutable parameters, as nested existentials — witnessed at one program point:
the point's state for a `Memory` binder, the values of its locals in the
binder's domain for the others; the instantiated body normalized and decided
conjunct by conjunct. -/
private partial def ledgerWitness (goal : MVarId) (others : List MVarId)
    (domain target : Lean.Expr) : TacticM Bool := do
  -- Only a label's binders: a memory, or a copy of a mutable parameter in
  -- the domain its contract reads it in.
  unless domain.isAppOf ``LeanerIR.Proofs.Denote.Memory || (domainAccessor? domain).isSome do
    return false
  for (value, state) in ← programPoints do
    let stateType ← inferType state
    let locals ← pointLocals value
    -- Each binder in turn, from this point.
    let rec choose (body : Lean.Expr) (used : Array Lean.Expr)
        (chosen : Array (Lean.Expr × Lean.Expr × Lean.Expr)) :
        TacticM (Option (Array (Lean.Expr × Lean.Expr × Lean.Expr) × Lean.Expr)) := do
      let body ← whnfR body
      let some (domain, predicate) := body.app2? ``Exists | return some (chosen, body)
      let memory ← try isDefEq stateType domain catch _ => pure false
      let candidates ← if memory then pure #[state] else
        match locals with
        | some (carriers, locals) =>
            locals.filterMapM fun (τ, entry) => do
              let some candidate ← localInDomain carriers τ entry domain | return none
              return if used.contains candidate then none else some candidate
        | none => pure #[]
      for candidate in candidates do
        if let some found ← choose (predicate.beta #[candidate]) (used.push candidate)
            (chosen.push (domain, predicate, candidate)) then
          return some found
      return none
    let some (chosen, body) ← choose target #[] #[] | continue
    if chosen.isEmpty then continue
    let some proof ← decideInstance body (← `(tactic| leaner_denote_instance_premise))
      (normalize := true) | continue
    assignWitnessed goal others chosen proof
    return true
  return false

/-- The integers a body bounds its binder by: one past a strict bound, a
bound itself, for a bound not reading the binder. -/
private def boundWitnesses (body : Lean.Expr) : MetaM (Array Lean.Expr) :=
  withLocalDeclD `k (mkConst ``Int) fun k => do
    let atoms := sitesWhere (fun e => e.isAppOfArity ``LT.lt 4 || e.isAppOfArity ``LE.le 4 ||
      e.isAppOfArity ``Eq 3) #[body.beta #[k]]
    let mut candidates : Array Lean.Expr := #[]
    for atom in atoms do
      let (left, right) := (atom.getArg! (atom.getAppNumArgs - 2), atom.appArg!)
      let other? := if left == k && !right.hasLooseBVars && !right.containsFVar k.fvarId! then
          some (right, true)
        else if right == k && !left.hasLooseBVars && !left.containsFVar k.fvarId! then
          some (left, false)
        else none
      let some (other, above) := other? | continue
      let candidate ← if atom.isAppOfArity ``LT.lt 4 then
          mkAppM (if above then ``HSub.hSub else ``HAdd.hAdd) #[other, toExpr (1 : Int)]
        else pure other
      unless candidates.contains candidate do candidates := candidates.push candidate
    pure candidates

/-- The one field index a body reads a runtime-value binder at, through
`field`, if it reads it so and nowhere else. -/
private def singleFieldRead? (body : Lean.Expr) : MetaM (Option Nat) :=
  withLocalDeclD `S (mkConst ``LeanerIR.RuntimeValue) fun S => do
    let stated := body.beta #[S]
    let reads := sitesWhere (fun e => e.isAppOfArity ``LeanerIR.RuntimeValue.field 2 &&
      e.getArg! 0 == S) #[stated]
    let some index := reads[0]? >>= fun e => (e.getArg! 1).nat? | return none
    unless reads.all fun e => (e.getArg! 1).nat? == some index do return none
    -- Every occurrence of the binder is such a read.
    let masked := stated.replace fun e =>
      if e.isAppOfArity ``LeanerIR.RuntimeValue.field 2 && e.getArg! 0 == S then
        some (mkConst ``LeanerIR.RuntimeValue.unit) else none
    if masked.containsFVar S.fvarId! then return none
    return some index

elab_rules : tactic
  | `(tactic| leaner_denote_witness) => do
    let goal ← getMainGoal
    let others := (← getGoals).filter (· != goal)
    goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``Exists 2 do throwError "not an existential"
    let domain := target.getArg! 0
    let body := target.getArg! 1
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let witness ← mkFreshExprMVar domain
      let saved ← saveState
      if ← isDefEq (body.beta #[witness]) (← instantiateMVars decl.type) then
        let proof ← mkAppOptM ``Exists.intro #[domain, body, ← instantiateMVars witness,
          decl.toExpr]
        goal.assign proof
        setGoals others
        return
      restoreState saved
    if ← ledgerWitness goal others domain target then return
    -- The conjuncts of the body in the order `And.intro` splits them, and
    -- whether each reads the binder.
    let conjuncts := if body.isLambda then conjunctsOf body.bindingBody! else #[]
    let reads := conjuncts.map (·.hasLooseBVars)
    -- The conjuncts not reading the binder hold or no witness does: decided
    -- once, before any candidate.
    for conjunct in conjuncts, read in reads do
      if read then continue
      let saved ← saveState
      let holds ← mkFreshExprMVar conjunct
      setGoals [holds.mvarId!]
      let decided ← try
          evalTactic (← `(tactic| leaner_denote_witness_premise))
          pure (← getGoals).isEmpty
        catch _ => pure false
      restoreState saved
      unless decided do
        throwError "a conjunct of the existential not reading its binder is not decided"
    -- A candidate witness.
    let attempt (witness : Lean.Expr) (premise : Lean.TSyntax `tactic) : TacticM Bool := do
      unless ← isDefEq (← inferType witness) domain do return false
      let some proof ← decideInstance (body.beta #[witness]) premise reads | return false
      assignWitnessed goal others #[(domain, body, witness)] proof
      return true
    -- An integer the body bounds, such as a mutable parameter's value between
    -- two increments.
    if domain.isConstOf ``Int then
      for value in ← boundWitnesses body do
        if ← attempt value (← `(tactic| leaner_denote_witness_premise)) then return
    -- A runtime value the body reads at one integer field: a nominal value
    -- holding an integer there, the integer witnessed in turn.
    if domain.isConstOf ``LeanerIR.RuntimeValue then
      if let some index ← singleFieldRead? body then
        let saved ← saveState
        let reduced ← try
            -- `∃ k : Int, body (nominal k)`, eliminated into the goal.
            let (statement, elimination) ← withLocalDeclD `k (mkConst ``Int) fun k => do
              let elements := (Array.range index).toList.map
                (fun _ => mkConst ``LeanerIR.RuntimeValue.unit) ++
                [mkApp (mkConst ``LeanerIR.RuntimeValue.integer) k]
              let source := mkApp2 (mkConst ``LeanerIR.StructHandle.mk)
                (mkApp (mkConst ``LeanerIR.NamespaceId.mk) (mkNatLit 0)) (mkNatLit 0)
              let witness := mkApp3 (mkConst ``LeanerIR.RuntimeValue.nominal) source
                (mkApp (mkConst ``Option.none [.zero]) (mkConst ``String))
                (← mkAppM ``List.toArray
                  #[← mkListLit (mkConst ``LeanerIR.RuntimeValue) elements])
              let stated := body.beta #[witness]
              let elimination ← withLocalDeclD `holds stated fun holds => do
                mkLambdaFVars #[k, holds]
                  (← mkAppOptM ``Exists.intro #[domain, body, witness, holds])
              pure (mkApp2 (mkConst ``Exists [.one]) (mkConst ``Int)
                (← mkLambdaFVars #[k] stated), elimination)
            let reduced ← mkFreshExprMVar statement
            goal.assign (← mkAppM ``Exists.elim #[reduced, elimination])
            setGoals [reduced.mvarId!]
            evalTactic (← `(tactic| try simp only [lir_denote_norm]))
            evalTactic (← `(tactic| leaner_denote_witness))
            pure (some (← getGoals))
          catch _ => pure none
        if reduced matches some [] then
          setGoals others
          return
        restoreState saved
    -- A variable of the binder's type in the context, or the value of a
    -- certified integer for an integer binder.
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      if ← attempt decl.toExpr (← `(tactic| leaner_denote_instance_premise)) then return
    if domain.isConstOf ``Int then
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
  | ((try simp only [LeanerIR.Proofs.Obligation_iff, Nat.reduceAdd, Int.reducePow,
      Int.reduceSub, Nat.reducePow, Nat.reduceSub])
     first
     | done
     | leaner_denote_assumption
     | (leaner_denote_bounds; leaner_denote_omega)
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

/-- The goal with each fact a hypothesis, named `name`. -/
private def assertFacts (goal : MVarId) (name : Name) (facts : Array Lean.Expr) :
    MetaM MVarId := do
  let mut goal := goal
  for fact in facts do
    goal ← goal.withContext do
      let (_, next) ← (← goal.assert name (← inferType fact) fact).intro1P
      pure next
  return goal

/-- The arguments of the storage keys at addresses a term builds. -/
private partial def storageAddresses (e : Lean.Expr) : Array Lean.Expr :=
  go e #[]
where
  go (e : Lean.Expr) (found : Array Lean.Expr) : Array Lean.Expr :=
    if e.isAppOfArity ``LeanerIR.StorageKey.address 1 then
      let argument := e.appArg!
      if found.contains argument then found else found.push argument
    else match e with
      | .app function argument => go argument (go function found)
      | .lam _ domain body _ | .forallE _ domain body _ => go body (go domain found)
      | .letE _ type value body _ => go body (go value (go type found))
      | .mdata _ inner | .proj _ _ inner => go inner found
      | _ => found

/-- Instances of the hypotheses quantified over an address at the addresses
the leaf reads memory at: a global invariant assumed at entry speaks about
every address, a proof about the ones it reads, as the Move Prover's
triggers select them. -/
elab "leaner_denote_key_instances" : tactic => do
  let goal ← getMainGoal
  let facts ← goal.withContext do
    let mut types : Array Lean.Expr := #[← instantiateMVars (← goal.getType)]
    let mut quantified : Array Lean.LocalDecl := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      let type ← instantiateMVars declaration.type
      types := types.push type
      if type.isForall && !type.isArrow then quantified := quantified.push declaration
    if quantified.isEmpty then throwError "the leaf has no quantified hypothesis"
    -- The addresses memory is read at.
    let mut addresses : Array Lean.Expr := #[]
    for type in types do
      let context ← getLCtx
      for found in storageAddresses type do
        unless found.hasLooseBVars || found.hasAnyFVar (!context.contains ·) ||
            addresses.contains found do
          addresses := addresses.push found
    if addresses.isEmpty then throwError "the leaf reads memory at no address"
    let mut facts : Array Lean.Expr := #[]
    for declaration in quantified do
      let type ← instantiateMVars declaration.type
      let .forallE _ domain body _ := type | continue
      -- The binder must be an address memory is read at in the body.
      unless (storageAddresses body).any (· == .bvar 0) do continue
      for address in addresses do
        if (← isDefEq (← inferType address) domain) then
          facts := facts.push (mkApp declaration.toExpr address)
    pure facts
  if facts.isEmpty then throwError "no quantified hypothesis reads memory at an address of the leaf"
  replaceMainGoal [← assertFacts goal `instance facts]

/-- Whether omega decides a proposition: a comparison of integers or
naturals, or a conjunction, disjunction, or negation of such. -/
private partial def isArithmetic (p : Lean.Expr) : MetaM Bool := do
  let p := (← instantiateMVars p).consumeMData
  let numeric (type : Lean.Expr) := type.isConstOf ``Int || type.isConstOf ``Nat
  if p.isAppOfArity ``And 2 || p.isAppOfArity ``Or 2 || p.isAppOfArity ``Iff 2 then
    return (← isArithmetic (p.getArg! 0)) && (← isArithmetic (p.getArg! 1))
  if p.isAppOfArity ``Not 1 then return ← isArithmetic (p.getArg! 0)
  if p.isAppOfArity ``Eq 3 || p.isAppOfArity ``Ne 3 then return numeric (p.getArg! 0)
  if p.isAppOfArity ``LE.le 4 || p.isAppOfArity ``LT.lt 4 || p.isAppOfArity ``GE.ge 4 ||
      p.isAppOfArity ``GT.gt 4 then
    return numeric (p.getArg! 0)
  return false

/-- A proof of a premise by a hypothesis stating it, either way round, or by
reflexivity. Both are normal, so they are compared as terms: a comparison
up to unfolding, at every hypothesis of every leaf, costs more than the
deciders that follow. -/
private def premiseByContext? (premise : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let premise ← instantiateMVars premise
  let flipped? ← match premise.eq? with
    | some (_, left, right) =>
        if left == right then return some (← mkEqRefl left)
        pure (some (← mkEq right left))
    | none => pure none
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    if type == premise then return some decl.toExpr
    if flipped? == some type then return some (← mkEqSymm decl.toExpr)
  return none

/-- Each hypothesis `p₁ → … → pₙ → q` whose premises omega or a hypothesis
proves is replaced by its conclusion `q`: the conclusion of a lemma under the
conditions its statement or a proof's path puts on it. Fails when none is. -/
elab "leaner_denote_modus_ponens" : tactic => do
  let initial ← saveState
  let others := (← getGoals).drop 1
  -- The goal's own premises are what may establish an implication's.
  let (goal, _) ← introduceThroughMarkers (← getMainGoal)
  let mut used : Array FVarId := #[]
  let mut facts : Array Lean.Expr := #[]
  let candidates ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let type ← instantiateMVars decl.type
      unless type.isArrow && (← isProp type) do return found
      return found.push decl
  for decl in candidates do
    -- A candidate whose premises do not all hold leaves no trace; one whose
    -- premises hold keeps what proving them added.
    let saved ← saveState
    try
      let (premises, conclusion) ← goal.withContext do
        forallTelescope (← instantiateMVars decl.type) fun binders conclusion => do
          if conclusion.hasAnyFVar (binders.contains <| .fvar ·) then
            throwError "a premise is a dependency of the conclusion"
          let types ← binders.mapM fun binder => inferType binder
          unless ← types.allM (fun type => Meta.isProp type) do
            throwError "a premise is not a proposition"
          pure (types, conclusion)
      let mut proofs : Array Lean.Expr := #[]
      for premise in premises do
        if let some proof ← goal.withContext (premiseByContext? premise) then
          proofs := proofs.push proof
          continue
        -- Omega only where it decides the premise.
        unless ← goal.withContext (isArithmetic premise) do
          throwError "a premise is not proved"
        let proof ← goal.withContext (mkFreshExprSyntheticOpaqueMVar premise)
        setGoals [proof.mvarId!]
        evalTactic (← `(tactic| omega))
        unless (← getGoals).isEmpty do throwError "a premise is not proved"
        proofs := proofs.push (← instantiateMVars proof)
      used := used.push decl.fvarId
      facts := facts.push (← goal.withContext (mkExpectedTypeHint (mkAppN decl.toExpr proofs)
        conclusion))
    catch _ => saved.restore
  if facts.isEmpty then
    initial.restore
    throwError "no implication's premises hold"
  let goal ← assertFacts goal `implied facts
  setGoals ((← goal.tryClearMany used) :: others)

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
containing it, and an equation whose reverse is already a rule. An element
a hypothesis fixes (`a[i] = v`) is read through the lookup the leaves state
(`a[i]? = some v`). -/
private def simpByContext (goal : MVarId) (targets : Array FVarId) (simplifyTarget : Bool)
    (failIfUnchanged : Bool) : TacticM (Option MVarId) := goal.withContext do
  let stx ← `(tactic| simp only [LeanerIR.Proofs.Obligation_iff, Nat.reduceAdd, Int.reducePow,
    Int.reduceSub, Nat.reducePow, Nat.reduceSub, lir_denote_norm])
  let { ctx, simprocs, .. } ← mkSimpContext stx (eraseLocal := false)
  let mut hypotheses : SimpTheorems := {}
  let mut equations : Array (Lean.Expr × Lean.Expr) := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail || targets.contains decl.fvarId then continue
    let type ← instantiateMVars decl.type
    unless ← isProp type do continue
    if ← selfReferential type then continue
    if let some (_, lhs, rhs) := type.eq? then
      if equations.contains (rhs, lhs) then continue
      equations := equations.push (lhs, rhs)
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
private def rangeInstance (normalize : Bool) : TacticM Unit := do
  let initial ← saveState
  let (goal, _) ← introduceThroughMarkers (← getMainGoal)
  let candidates ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let ty ← instantiateMVars decl.type
      return if ty.isForall then found.push decl.toExpr else found
  -- The context at the bound is normalized first, where asked: the facts
  -- there come from the iteration, such as a tested condition, as they
  -- were stated.
  let closeByContext ← if normalize then `(tactic| first
    | leaner_denote_assumption
    | omega
    | ((try simp only [lir_denote_norm] at *)
       leaner_denote_simp_by_context
       first | done | omega | leaner_denote_assumption))
    else `(tactic| first | leaner_denote_assumption | omega)
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

elab "leaner_denote_range_instance" : tactic => rangeInstance true

/-- `leaner_denote_range_instance` deciding the bound by the context as
stated: a leaf the prepared deciders normalize first pays the
normalization once, there. -/
elab "leaner_denote_range_instance_stated" : tactic => rangeInstance false

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

/-- Decide a leaf case by case over its disjunctive hypotheses, as a read
after a write normalizes, each case by the cheap deciders, splitting at
most `depth` disjunctions. -/
syntax "leaner_denote_cheap_cases " num : tactic

macro_rules
  | `(tactic| leaner_denote_cheap_cases $depth) => do
      let depth := depth.getNat
      if depth == 0 then `(tactic| fail "no cases left")
      else
        let rest := Lean.Syntax.mkNumLit (toString (depth - 1))
        `(tactic| (
          leaner_denote_split_disjunction
          all_goals (first
            | leaner_denote_assumption
            | leaner_denote_decide_cheap
            | leaner_denote_instance
            | leaner_denote_cheap_cases $rest)
          done))

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
     | leaner_denote_instance)
  | leaner_denote_timed "s-grind" leaner_denote_grind)

/-- Decide a normalized leaf case by case over the values of an integer that
ranges over a few literals (`leaner_denote_split_range`), each case settled
and decided as the prepared leaf is. -/
syntax "leaner_denote_decide_ranges" : tactic

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
  -- Positions that range over a few literal values, case by case.
  | leaner_denote_decide_ranges
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
          -- Last, products and congruence, which `omega` reads as atoms.
          all_goals (first | (leaner_denote_bounds; omega) | leaner_denote_bv | leaner_denote_grind))))))

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

/-- The components of a bundled argument. -/
private partial def tupleComponents (e : Lean.Expr) : List Lean.Expr :=
  if e.isAppOfArity ``Prod.mk 4 then e.getArg! 2 :: tupleComponents (e.getArg! 3) else [e]

/-- The integer arguments of the recursive specification functions terms
apply. -/
private def specIntegerArguments (terms : Array Lean.Expr) : MetaM (Array Lean.Expr) := do
  let env ← getEnv
  let mut arguments := #[]
  for application in sitesWhere (fun e => (specUnfolding? env e).isSome && !e.hasLooseBVars) terms do
    for argument in application.getAppArgs do
      for component in tupleComponents argument do
        if (← whnfR (← inferType component)).isConstOf ``Int then
          arguments := arguments.push component
  return arguments

/-- Array positions and the integer arguments of recursive specification
functions by value: a position reading a named value is replaced by the
position its definition gives, where the arithmetic context proves them
equal, so a write at a named position and a read at the computed one, or a
clause's position, are compared as the same term, as are applications at a
named argument and at the computed one. -/
elab "leaner_denote_unname_positions" : tactic => do
  let goal ← getMainGoal
  let definitions ← namedDefinitions goal
  if definitions.isEmpty then throwError "no named value"
  let positions ← goal.withContext do
    let hypotheses ← (← goal.getNondepPropHyps).mapM fun fvarId => do
      instantiateMVars (← fvarId.getType)
    let terms := #[← instantiateMVars (← goal.getType)] ++ hypotheses
    return arrayPositions terms ++ (← specIntegerArguments terms)
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
  match ← simpAt equated (← `(tactic| simp only [Int.add_sub_cancel])) rules targets true with
  | none => replaceMainGoal []
  | some next => replaceMainGoal [← next.tryClearMany rules]

/-- A residual leaf in source form: each value the arithmetic rules named
(`named.val = e`, a local of type `SpecInt` itself, where parameters and
results have carrier types) replaced by what it names, earliest first, as an
authored proof states it. -/
elab "leaner_denote_unname" : tactic => do
  let mut goal ← getMainGoal
  let mut visited : Array FVarId := #[]
  repeat
    let named ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if visited.contains decl.fvarId then return none
        return (← namedDefinition? decl).map fun (result, _) => (decl.fvarId, result)
    let some (equation, result) := named | break
    visited := visited.push equation
    let targets ← goal.withContext do
      return (← goal.getNondepPropHyps).filter (· != equation)
    setGoals [goal]
    match ← simpAt goal (← `(tactic| simp only [])) #[equation] targets true with
    | none => replaceMainGoal []; return
    | some next =>
        -- simpAt rewrites propositions and the goal, but a saved continuation
        -- can still mention the name in its local definition. Keep the equation
        -- until that use disappears; otherwise later unfolding loses the value.
        let used ← next.withContext do
          if (← next.getType).containsFVar result then return true
          return (← getLCtx).any fun decl =>
            decl.fvarId != equation && decl.fvarId != result &&
              (decl.type.containsFVar result || decl.value?.any (·.containsFVar result))
        goal ← if used then pure next else (← next.clear equation).tryClear result
  replaceMainGoal [goal]

/-- The deciders of a leaf an authored proof may take over. Bound the attempt:
even omega can branch heavily on the conditional facts of a prepared map
leaf. Automatic search must leave budget for the authored proof. -/
macro "leaner_denote_decide_residual" : tactic => `(tactic| leaner_denote_budgeted (
  first
  | done
  | leaner_denote_timed "residual omega" omega
  | leaner_denote_timed "residual cheap" leaner_denote_decide_cheap
  | leaner_denote_timed "residual instance" leaner_denote_instance
  | leaner_denote_timed "residual range" leaner_denote_range_instance
  | leaner_denote_timed "residual context" (
      leaner_denote_simp_by_context
      first | done | (leaner_denote_bounds; omega))
  | leaner_denote_trivial))

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

/-- Whether the written deciders can decide the leaf at all: they decide
instances of hypotheses quantified over integer positions and lookups
after writes, so a leaf with neither (a storage leaf, whose quantifiers
range over keys) is prepared only when it holds atoms `grind` relates
(`leaner_denote_has_atoms`), and is otherwise left to the pipeline, whose normalization would only be paid
twice. -/
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

/-- Whether a leaf holds terms omega reads as atoms that `grind` relates:
observations of a map value (`hasKey`, `valueAt`, `keyAt`, `size`, `rank`),
whose laws are its lemmas, or applications of a recursive specification
function (`f.spec`), equal at equal arguments. -/
elab "leaner_denote_has_atoms" : tactic => do
  let goal ← getMainGoal
  let env ← getEnv
  let found ← goal.withContext do
    let isAtom (e : Lean.Expr) : Bool :=
      e.isConstOf ``LeanerIR.Maps.hasKey || e.isConstOf ``LeanerIR.Maps.valueAt ||
      e.isConstOf ``LeanerIR.Maps.keyAt || e.isConstOf ``LeanerIR.Maps.size ||
      e.isConstOf ``LeanerIR.Maps.rank ||
      (match e with
        | .const name _ => name matches .str _ "spec" && env.contains (Name.str name "unfold")
        | _ => false)
    if ((← instantiateMVars (← goal.getType)).find? isAtom).isSome then return true
    (← getLCtx).anyM fun decl => do
      if decl.isImplementationDetail then return false
      return ((← instantiateMVars decl.type).find? isAtom).isSome
  unless found do throwError "the leaf observes no map and applies no recursive specification function"

/-- Whether a hypothesis is a frame of a call: global memory reads the same
after the call as before, at every key or resource type it ranges over, or
everywhere. -/
private def isFrame (type : Lean.Expr) : MetaM Bool :=
  forallTelescope type fun _ body => do
    let_expr Eq _ after before := body | return false
    let afterHead := after.getAppFn
    let beforeHead := before.getAppFn
    unless afterHead.isFVar && beforeHead.isFVar && afterHead != beforeHead &&
        after.getAppArgs == before.getAppArgs do
      return false
    return (← whnfR (← inferType afterHead)).isAppOfArity ``LeanerIR.Proofs.Denote.Memory 1

/-- Decide that two memory slots differ, from the context. -/
macro "leaner_denote_keys_differ" : tactic => `(tactic| (
  simp only [ne_eq, LeanerIR.Proofs.Denote.ResourceType.mk.injEq,
    LeanerIR.Proofs.Denote.NTy.struct.injEq, LeanerIR.Proofs.Denote.NTy.enum.injEq,
    LeanerIR.Proofs.Denote.NRow.cons.injEq, LeanerIR.StructHandle.mk.injEq,
    LeanerIR.NamespaceId.mk.injEq, LeanerIR.StorageKey.bool.injEq,
    LeanerIR.StorageKey.character.injEq, LeanerIR.StorageKey.integer.injEq,
    LeanerIR.StorageKey.address.injEq, LeanerIR.StorageKey.signer.injEq,
    LeanerIR.StorageKey.string.injEq, LeanerIR.StorageKey.bytes.injEq, reduceCtorEq, true_and,
    and_true, false_and, and_false, not_false_eq_true]
  first
  | done
  | assumption
  -- A discharger runs at reducible transparency, which does not unfold `¬`.
  | with_unfolding_all (intro equal; exact absurd equal.symm ‹_›)
  | omega))

/-- Read each global a call leaves alone at the state before the call: the
frames of the calls rewrite the goal where the context decides that a key
is not one the callee modifies. -/
elab "leaner_denote_frames" : tactic => do
  let goal ← getMainGoal
  let lemmas ← goal.withContext do
    let mut lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      if ← isFrame (← instantiateMVars declaration.type) then
        lemmas := lemmas.push
          (← `(Lean.Parser.Tactic.simpLemma| $(← Lean.Elab.Term.exprToSyntax declaration.toExpr):term))
    pure lemmas
  if lemmas.isEmpty then throwError "the leaf has no frame of a call"
  evalTactic (← `(tactic| simp (disch := leaner_denote_keys_differ) only [$lemmas,*]))

/-- A proof of the stored invariants of a memory from the context: a
hypothesis or a conjunct of one stating them, or an implication concluding
them whose premise has such a proof, as a function preserving them states. -/
private partial def storedInvariantsProof? (memory : Lean.Expr) (fuel : Nat := 8) :
    MetaM (Option Lean.Expr) := do
  if fuel == 0 then return none
  let rec conjunct (proof type : Lean.Expr) : Nat → MetaM (Option Lean.Expr)
    | 0 => pure none
    | depth + 1 => do
      if type.isAppOfArity ``LeanerIR.Proofs.Denote.MemoryInvariants 3 &&
          type.getArg! 2 == memory then
        return some proof
      if let some (left, right) := type.and? then
        if let some found ← conjunct (mkApp3 (mkConst ``And.left) left right proof) left depth then
          return some found
        return ← conjunct (mkApp3 (mkConst ``And.right) left right proof) right depth
      if type.isArrow && type.bindingBody!.isAppOfArity ``LeanerIR.Proofs.Denote.MemoryInvariants 3 &&
          type.bindingBody!.getArg! 2 == memory then
        let premise := type.bindingDomain!
        if premise.isAppOfArity ``LeanerIR.Proofs.Denote.MemoryInvariants 3 &&
            premise.getArg! 2 != memory then
          if let some held ← storedInvariantsProof? (premise.getArg! 2) (fuel - 1) then
            return some (mkApp proof held)
      return none
  for declaration in ← getLCtx do
    if declaration.isImplementationDetail then continue
    if let some proof ← conjunct declaration.toExpr (← instantiateMVars declaration.type) 32 then
      return some proof
  return none

/-- Establish the data invariants of stored resources (`MemoryInvariants`)
of a memory the leaf's writes made from one the context has them of:
through each write, which owes the value written its own invariant. A goal
preserving them takes them of the earlier memory first. -/
elab "leaner_denote_memory_invariants" : tactic => do
  let rec establish (goal : MVarId) : Nat → TacticM (List MVarId)
    | 0 => throwError "too many writes to establish the stored invariants through"
    | fuel + 1 => goal.withContext do
      let target ← instantiateMVars (← goal.getType)
      if target.isArrow && target.bindingBody!.isAppOfArity
          ``LeanerIR.Proofs.Denote.MemoryInvariants 3 then
        let (_, goal) ← goal.intro1P
        return ← establish goal fuel
      unless target.isAppOfArity ``LeanerIR.Proofs.Denote.MemoryInvariants 3 do
        throwError "the leaf owes no stored invariants"
      if let some proof ← storedInvariantsProof? (target.getArg! 2) then
        goal.assign proof
        return []
      let written := target.getArg! 2
      unless written.isAppOfArity ``LeanerIR.Proofs.Denote.Memory.set 5 do
        throwError "no hypothesis states the stored invariants of the memory written"
      let value := written.getArg! 4
      if value.isAppOfArity ``Option.none 1 then
        let [holds] ← goal.apply (mkConst ``LeanerIR.Proofs.Denote.MemoryInvariants.set_none)
          | throwError "the removal law did not leave its obligation"
        return ← establish holds fuel
      let law := if value.isAppOfArity ``Option.some 2 then
          ``LeanerIR.Proofs.Denote.MemoryInvariants.set_some
        else ``LeanerIR.Proofs.Denote.MemoryInvariants.set
      let [holds, stored] ← goal.apply (mkConst law)
        | throwError "the write law did not leave its two obligations"
      return (← establish holds fuel) ++ [stored]
  replaceMainGoal (← establish (← getMainGoal) 256)
  -- Select the invariant of each written resource before running the leaf
  -- pipeline over the surrounding context. Constructor comparisons in the
  -- unit's dispatch table are computations, not new proof obligations.
  evalTactic (← `(tactic| all_goals (try simp only
    [lir_denote, lir_denote_norm, lir_denote_eval, Prod.fst, Prod.snd])))

/-- The data invariant of each stored value the leaf reads, from the stored
invariants of the memory it reads (`MemoryInvariants.read`). -/
elab "leaner_denote_stored_reads" : tactic => do
  let goal ← getMainGoal
  let (ctx, simprocs) ← normalization
  let facts ← goal.withContext do
    let mut facts : Array Lean.Expr := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      let_expr Eq _ read held := ← instantiateMVars declaration.type | continue
      unless held.isAppOfArity ``Option.some 2 && read.getAppNumArgs == 2 do continue
      let memory := read.appFn!.appFn!
      unless (← whnfR (← inferType memory)).isAppOfArity ``LeanerIR.Proofs.Denote.Memory 1 do continue
      let some holds ← storedInvariantsProof? memory | continue
      let fact ← mkAppM ``LeanerIR.Proofs.Denote.MemoryInvariants.read #[holds, declaration.toExpr]
      let (result, _) ← Simp.main (← inferType fact) ctx
        (methods := Simp.mkDefaultMethodsCore simprocs)
      facts := facts.push (← match result.proof? with
        | some equality => mkEqMP equality fact
        | none => pure fact)
    pure facts
  if facts.isEmpty then throwError "the leaf reads no stored value with stored invariants"
  replaceMainGoal [← assertFacts goal `stored facts]

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

-- A leaf holding values in a generic callee's view reads its ranges once the
-- substitutions settled; each case is decided by the pipeline, which splits
-- on a further position.
macro_rules
  | `(tactic| leaner_denote_decide_ranges) => `(tactic| (
      first
      | leaner_denote_timed "range split" leaner_denote_split_range
      | (leaner_denote_holds_transport
         leaner_denote_settle
         leaner_denote_timed "range split" leaner_denote_split_range)
      all_goals leaner_denote_reduce_projections
      all_goals leaner_denote_normalize_context
      -- The quantifiers the case's value bounds by literals, instance by
      -- instance.
      all_goals (try (leaner_denote_expand_ranges; leaner_denote_normalize_context))
      -- The specification functions the case's values fix, unfolded to
      -- their base cases.
      all_goals (try (leaner_denote_timed "range unfold" leaner_denote_specs_to_base
                      leaner_denote_normalize_context))
      all_goals leaner_denote_settle
      all_goals (first
        | done
        | leaner_denote_timed "range omega" omega
        -- A range invariant extended by the case's iteration.
        | leaner_denote_timed "range instance" leaner_denote_range_instance
        | (leaner_denote_prepared_applies
           leaner_denote_timed "written" leaner_denote_decide_written)
        | leaner_denote_timed "range pipeline" leaner_denote_pipeline)))

/-- Stored-field invariants and an invocation can spell the same unknown
closure through its native encoding and its raw value, respectively. Align
those facts only within this closing attempt: known-closure dispatch and
authored proofs still need the unreduced encoding elsewhere. -/
elab "leaner_denote_function_facts" : tactic => do
  let goal ← getMainGoal
  let applies ← goal.withContext do
    (← getLCtx).anyM fun decl => do
      if decl.isImplementationDetail then return false
      let type ← instantiateMVars decl.type
      return (type.find? (·.isConstOf ``NTy.encode)).isSome &&
        (type.find? (·.isConstOf ``LeanerIR.Proofs.EnsuresOf)).isSome
  unless applies do throwError "the leaf has no encoded function variable"
  evalTactic (← `(tactic| leaner_denote_budgeted (
    leaner_denote_prepare
    all_goals simp only [NTy.encode_function] at *
    all_goals leaner_denote_grind
    done)))

/-- Decide a leaf in the normal form an authored proof receives: the
normalization is one pass over the context, where the saturation of the
pipeline rewrites the hypotheses with each other for rounds. -/
macro "leaner_denote_decide_prepared" : tactic => `(tactic| (
  (first | leaner_denote_prepared_applies | leaner_denote_has_atoms)
  leaner_denote_timed "intro" leaner_denote_intro_any
  leaner_denote_timed "prepare" leaner_denote_prepare
  all_goals (try leaner_denote_timed "positions" leaner_denote_unname_positions)
  -- The facts a solver's map axioms give hold at the positions the
  -- unfolded definitions read as well.
  all_goals (try (leaner_denote_timed "p-unfold specs" leaner_denote_unfold_specs
                  leaner_denote_timed "p-map positions" leaner_denote_map_positions))
  all_goals (first
    | (leaner_denote_prepared_applies
       leaner_denote_timed "written" leaner_denote_decide_written)
    -- Linear arithmetic over the terms the normalization left.
    | leaner_denote_timed "p-omega" omega
    -- An integer that ranges over a few literal values, case by case.
    | leaner_denote_decide_ranges
    -- Congruence over the terms omega reads as atoms, such as products.
    | leaner_denote_timed "p-grind" leaner_denote_grind
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

/-- An opaque call can describe a stored value through its codec and a
non-aborting memory read. Expose the present value before reducing the codec.
Keep this closing attempt bounded: unrelated leaves and failed attempts retain
their original context. -/
elab "leaner_denote_decoded_reads" : tactic => do
  let goal ← getMainGoal
  let applies ← goal.withContext do
    let mentionsDecode (e : Lean.Expr) :=
      (e.find? (·.isConstOf ``LeanerIR.Proofs.Codec.decode?)).isSome
    if mentionsDecode (← instantiateMVars (← goal.getType)) then return true
    (← getLCtx).anyM fun decl => do
      if decl.isImplementationDetail then return false
      return mentionsDecode (← instantiateMVars decl.type)
  unless applies do throwError "the leaf has no decoded memory read"
  evalTactic (← `(tactic| leaner_denote_budgeted (
    try leaner_denote_unname
    leaner_denote_prepare
    all_goals simp only [not_or, Option.ne_none_iff_exists] at *
    all_goals leaner_denote_split_hypotheses
    all_goals simp [lir_denote_norm] at *
    all_goals grind
    done)))

/-- Decide one leaf whose recursive specification functions are unfolded. -/
syntax "leaner_denote_leaf_unfolded" : tactic

macro_rules
  | `(tactic| leaner_denote_leaf) => `(tactic|
      (leaner_denote_canonical_families
       leaner_denote_timed "clear" leaner_denote_clear_computations
       try leaner_denote_timed "memory reads" leaner_denote_memory_reads
       try leaner_denote_timed "context reads" leaner_denote_context_memory_reads
       first
       -- A goal a hypothesis states, before unfolding or splitting.
       | leaner_denote_timed "assumption" leaner_denote_assumption
       | (try leaner_denote_timed "unfold specs" leaner_denote_unfold_specs
          leaner_denote_leaf_unfolded)))

macro_rules
  | `(tactic| leaner_denote_leaf_unfolded) => `(tactic|
      (first
       -- Stored invariants, through the writes to their memory.
       | (leaner_denote_timed "stored invariants" leaner_denote_memory_invariants
          all_goals leaner_denote_leaf)
       -- A goal a hypothesis states, as the unfolding gives it.
       | leaner_denote_timed "assumption" leaner_denote_assumption
       | leaner_denote_timed "decoded reads" leaner_denote_decoded_reads
       | leaner_denote_timed "function facts" leaner_denote_function_facts
       -- Atomic hypotheses first: each is rewritten, used, and dropped on
       -- its own. A goal whose binders a loop step introduced is an
       -- instance of a quantified hypothesis as much as a quantified one.
       | (try leaner_denote_timed "stored reads" leaner_denote_stored_reads
          leaner_denote_timed "split" leaner_denote_split_hypotheses
          -- The variables the destructuring equates, such as a resolved
          -- prophecy and the value written, are one to the deciders, and the
          -- components it projects off literal pairs are the components.
          all_goals leaner_denote_timed "subst" leaner_denote_subst_vars
          all_goals leaner_denote_timed "projections" leaner_denote_reduce_projections
          try leaner_denote_timed "key instances" leaner_denote_key_instances
          try leaner_denote_timed "frames" leaner_denote_frames
          leaner_denote_timed "map positions" leaner_denote_map_positions
          -- What an implication concludes where its premises hold.
          try leaner_denote_timed "modus ponens" leaner_denote_modus_ponens
          first
          | leaner_denote_timed "cheap" leaner_denote_decide_cheap
          | leaner_denote_timed "instance" leaner_denote_instance
          | (leaner_denote_timed "assembled witness" leaner_denote_assembled_witness
             all_goals leaner_denote_leaf)
          | leaner_denote_timed "range" leaner_denote_range_instance_stated
          | leaner_denote_timed "cheap cases" (leaner_denote_cheap_cases 3)
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
           try leaner_denote_timed "frames3" leaner_denote_frames
           leaner_denote_timed "map positions3" leaner_denote_map_positions
           first
           | leaner_denote_timed "cheap3" leaner_denote_decide_cheap
           -- An instance of a quantified hypothesis, such as a loop
           -- invariant, at the position or one past its range.
           | leaner_denote_timed "instance3" leaner_denote_instance
           | leaner_denote_timed "range3" leaner_denote_range_instance_stated
           | leaner_denote_timed "prepared3" leaner_denote_decide_prepared
           | leaner_denote_timed "pipeline3" leaner_denote_pipeline
           | (leaner_denote_timed "cases3" leaner_denote_split_disjunction
              all_goals leaner_denote_leaf)
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
dsimproc [lir_denote_norm] reduceReverseSlice (LeanerIR.Proofs.Denote.reverseSlice _ _ _) :=
  fun e => do
    let_expr LeanerIR.Proofs.Denote.reverseSlice _ array left right := e | return .continue
    reduceArrayLiteral array #[left, right] fun elements positions =>
      let left := positions[0]!
      let right := positions[1]!
      if right ≤ elements.length then
        some (elements.take left ++ ((elements.take right).drop left).reverse ++
          elements.drop right)
      else some elements

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

simproc ↓ [lir_denote_norm] flowBindAction (@LeanerIR.Proofs.Denote.Flow.bind _ ?skolems _ _ _ _ _ _) :=
  bindActionStep


/-- The type arguments and outer carriers of the carriers a call's type
arguments induce, in either spelling: as carriers, or as the carriers of an
induced frame. -/
private def inducedCarriers? (family : Lean.Expr) : Option (Lean.Expr × Lean.Expr) :=
  if family.isAppOfArity ``LeanerIR.Proofs.Denote.Carriers.instantiate 2 then
    some (family.getArg! 0, family.getArg! 1)
  else if family.isAppOfArity ``LeanerIR.Proofs.Denote.Skolems.toCarriers 2 then
    let frame := family.getArg! 1
    if frame.isAppOfArity ``LeanerIR.Proofs.Denote.Skolems.instantiate 3 then
      some (frame.getArg! 1,
        mkApp2 (mkConst ``LeanerIR.Proofs.Denote.Skolems.toCarriers) (frame.getArg! 0)
          (frame.getArg! 2))
    else none
  else none

/-- An equation between values of a type parameter under the family a
call's type arguments induce is the equation of their encodings at the
argument's type, which the normalizer reduces to the runtime structure a
caller's clauses speak about. -/
simproc ↓ [lir_denote_norm] instantiatedEqAsEncoding (@Eq _ _ _) :=
  fun e => do
    let_expr Eq carrier left right := e | return .continue
    let some (outer, argument) ← (do
        match carrier.getAppFn.constName?, carrier.getAppArgs with
        | some ``LeanerIR.Proofs.Denote.Carriers.carrier, #[family, index] =>
            let some (θ, outer) := inducedCarriers? family | pure none
            pure (some (outer, mkApp2 (mkConst ``LeanerIR.Proofs.Denote.NTy.subst)
              (← mkAppM ``Subtype.val #[θ])
              (mkApp (mkConst ``LeanerIR.Proofs.Denote.NTy.param) index)))
        | some ``LeanerIR.Proofs.Denote.NTy.carrier, #[family, τ] =>
            let some (θ, outer) := inducedCarriers? family | pure none
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
  let τ := τ.consumeMData
  match τ.getAppFn.constName?, τ.getAppArgs with
  | some ``LeanerIR.Proofs.Denote.NTy.unit, #[] | some ``LeanerIR.Proofs.Denote.NTy.bool, #[]
  | some ``LeanerIR.Proofs.Denote.NTy.address, #[] | some ``LeanerIR.Proofs.Denote.NTy.signer, #[]
  | some ``LeanerIR.Proofs.Denote.NTy.string, #[] | some ``LeanerIR.Proofs.Denote.NTy.bytes, #[] => true
  | some ``LeanerIR.Proofs.Denote.NTy.int, #[_, _] => true
  | some ``LeanerIR.Proofs.Denote.NTy.param, #[index] => index.nat?.isSome || index.rawNatLit?.isSome
  | some ``LeanerIR.Proofs.Denote.NTy.tuple, #[row] => spelledNRow row
  | some ``LeanerIR.Proofs.Denote.NTy.struct, #[_, arguments, row] =>
      spelledNRow arguments && spelledNRow row
  | some ``LeanerIR.Proofs.Denote.NTy.enum, #[_, arguments, _, rows, _] =>
      spelledNRow arguments && spelledNRows rows
  | some ``LeanerIR.Proofs.Denote.NTy.vector, #[element] => spelledNTy element
  | some ``LeanerIR.Proofs.Denote.NTy.ref, #[referent] => spelledNTy referent
  | _, _ => false

private partial def spelledNRow (row : Lean.Expr) : Bool :=
  let row := row.consumeMData
  match row.getAppFn.constName?, row.getAppArgs with
  | some ``LeanerIR.Proofs.Denote.NRow.nil, #[] => true
  | some ``LeanerIR.Proofs.Denote.NRow.cons, #[τ, rest] => spelledNTy τ && spelledNRow rest
  | _, _ => false

private partial def spelledNRows (rows : Lean.Expr) : Bool :=
  let rows := rows.consumeMData
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
  let τ := τ.consumeMData
  let args := τ.getAppArgs
  match τ.getAppFn.constName? with
  | some ``LeanerIR.Proofs.Denote.NTy.param =>
      let index ← (args[0]?).bind fun index => index.nat? <|> index.rawNatLit?
      pure ((rowEntry? θ index).getD τ)
  | some ``LeanerIR.Proofs.Denote.NTy.tuple =>
      pure (mkApp τ.getAppFn (← substituteNRow θ (← args[0]?)))
  | some ``LeanerIR.Proofs.Denote.NTy.struct =>
      pure (mkApp3 τ.getAppFn (← args[0]?) (← substituteNRow θ (← args[1]?))
        (← substituteNRow θ (← args[2]?)))
  | some ``LeanerIR.Proofs.Denote.NTy.enum =>
      pure (mkApp5 τ.getAppFn (← args[0]?) (← substituteNRow θ (← args[1]?)) (← args[2]?)
        (← substituteNRows θ (← args[3]?)) (← args[4]?))
  | some ``LeanerIR.Proofs.Denote.NTy.vector | some ``LeanerIR.Proofs.Denote.NTy.ref =>
      pure (mkApp τ.getAppFn (← substituteNTy θ (← args[0]?)))
  | _ => if spelledNTy τ then some τ else none

private partial def substituteNRow (θ : Lean.Expr) (row : Lean.Expr) : Option Lean.Expr := do
  let row := row.consumeMData
  if row.isAppOfArity ``LeanerIR.Proofs.Denote.NRow.cons 2 then
    pure (mkApp2 row.getAppFn (← substituteNTy θ (row.getArg! 0)) (← substituteNRow θ (row.getArg! 1)))
  else if row.isConstOf ``LeanerIR.Proofs.Denote.NRow.nil then some row
  else none

private partial def substituteNRows (θ : Lean.Expr) (rows : Lean.Expr) : Option Lean.Expr := do
  let rows := rows.consumeMData
  if rows.isAppOfArity ``LeanerIR.Proofs.Denote.NRows.cons 2 then
    pure (mkApp2 rows.getAppFn (← substituteNRow θ (rows.getArg! 0))
      (← substituteNRows θ (rows.getArg! 1)))
  else if rows.isConstOf ``LeanerIR.Proofs.Denote.NRows.nil then some rows
  else none
end

/-- Whether a row is spelled to its end, so that a position past it is known
to be past it. -/
private partial def spelledSpine (row : Lean.Expr) : Bool :=
  row.isConstOf ``LeanerIR.Proofs.Denote.NRow.nil ||
    (row.isAppOfArity ``LeanerIR.Proofs.Denote.NRow.cons 2 && spelledSpine (row.getArg! 1))

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
  let some (θ, outer) := inducedCarriers? family | return none
  let some row := substitutionRow? θ | return none
  let some substituted := substituteNTy row τ | return none
  return some (outer, substituted)

/-- The codec of a type at an induced family (`outerSpelling?`). -/
dsimproc ↓ [lir_denote, lir_denote_norm] codecOuterFamily
    (@LeanerIR.Proofs.Denote.NTy.codec ?skolems _) := fun e => do
  let_expr LeanerIR.Proofs.Denote.NTy.codec family τ := e | return .continue
  let some (outer, substituted) ← outerSpelling? family τ | return .continue
  return .visit (mkApp2 e.getAppFn outer substituted)

/-- Opaque map encodings exposed after preparation use the caller's family.
Restrict this to the enumeration-backed entry-vector layout: control-flow
variants retain the carrier spelling their constructor rules match. -/
dsimproc ↓ [lir_denote, lir_denote_norm] encodeOuterFamily
    (@LeanerIR.Proofs.Denote.NTy.encode ?skolems _ _) := fun e => do
  let_expr LeanerIR.Proofs.Denote.NTy.encode family τ value := e | return .continue
  unless value.isFVar do return .continue
  let type := τ
  unless type.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.enum 5 do return .continue
  let rows := type.getArg! 3
  unless rows.isAppOfArity ``LeanerIR.Proofs.Denote.NRows.cons 2 &&
      (rows.getArg! 1).isConstOf ``LeanerIR.Proofs.Denote.NRows.nil do return .continue
  let fields := rows.getArg! 0
  unless fields.isAppOfArity ``LeanerIR.Proofs.Denote.NRow.cons 2 &&
      (fields.getArg! 1).isConstOf ``LeanerIR.Proofs.Denote.NRow.nil do return .continue
  let vector := fields.getArg! 0
  unless vector.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.vector 1 &&
      vector.appArg!.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.struct 3 do return .continue
  let some (outer, substituted) ← outerSpelling? family τ | return .continue
  return .visit (mkApp3 e.getAppFn outer substituted value)

/-- A term with every family-dependent spelling of a spelled type at an
induced family canonical: the carrier, row, codec, and encoding of a type at
a family a call's type arguments induce are, by definition, those of its
substitution at the outer family, so that a callee's view of a value and
the caller's read alike, also in the implicit arguments rewriting does not
reach. -/
private def canonicalFamilies (e : Lean.Expr) : MetaM Lean.Expr := do
  unless (e.find? fun t => t.isConstOf ``LeanerIR.Proofs.Denote.Skolems.instantiate ||
      t.isConstOf ``LeanerIR.Proofs.Denote.Carriers.instantiate ||
      t.isConstOf ``LeanerIR.Proofs.Denote.NTy.subst || t.isConstOf ``LeanerIR.Proofs.Denote.NRow.subst ||
      t.isConstOf ``LeanerIR.Proofs.Denote.NRows.subst).isSome do return e
  Meta.transform e (post := fun t => do
    let args := t.getAppArgs
    let some head := t.getAppFn.constName? | return .continue
    -- A substitution by a row spelled to its end, such as a transport's
    -- result type, evaluated as the caller spells it.
    if head == ``LeanerIR.Proofs.Denote.NTy.subst || head == ``LeanerIR.Proofs.Denote.NRow.subst ||
        head == ``LeanerIR.Proofs.Denote.NRows.subst then
      let #[θ, operand] := args | return .continue
      let row ← spelledRow θ
      unless spelledSpine row do return .continue
      let evaluated := if head == ``LeanerIR.Proofs.Denote.NTy.subst then substituteNTy row operand
        else if head == ``LeanerIR.Proofs.Denote.NRow.subst then substituteNRow row operand
        else substituteNRows row operand
      return match evaluated with
        | some evaluated => .done evaluated
        | none => .continue
    let induced (family : Lean.Expr) : Option (Lean.Expr × Lean.Expr) := do
      let (θ, outer) ← inducedCarriers? family
      (substitutionRow? θ).map (outer, ·)
    let rebuilt : Option Lean.Expr := do
      let family ← args[0]?
      let (outer, row) ← induced family
      -- Encodings must use the same family as their values as well: a concrete
      -- enum field returned by a generic callee otherwise keeps an induced
      -- family that prevents the row-encoding simp rules from matching.
      if head == ``LeanerIR.Proofs.Denote.NTy.carrier || head == ``LeanerIR.Proofs.Denote.NTy.codec ||
          head == ``LeanerIR.Proofs.Denote.NTy.encode then
        let τ ← args[1]?
        let τ' ← substituteNTy row τ
        pure (mkAppN t.getAppFn (#[outer, τ'] ++ args.extract 2 args.size))
      else if head == ``LeanerIR.Proofs.Denote.HList || head == ``LeanerIR.Proofs.Denote.rowCodec ||
          head == ``LeanerIR.Proofs.Denote.HList.encode then
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

/-- The outer frame, the type arguments, and their spelled row of a frame a
call's type arguments induce. -/
private def inducedFrame? (frame : Lean.Expr) : Option (Lean.Expr × Lean.Expr × Lean.Expr) := do
  let frame := frame.consumeMData
  guard (frame.isAppOfArity ``LeanerIR.Proofs.Denote.Skolems.instantiate 3)
  let θ := frame.getArg! 1
  pure (θ, frame.getArg! 2, ← substitutionRow? θ)

/-- A value carried from the runtime family to an induced frame, as the
outer frame carries it at the type's substitution, evaluated
(`Skolems.ofRuntime_instantiate`): it then meets the outer frame's
transports, also where the value's type needs the substitution evaluated. -/
dsimproc ↓ [lir_denote, lir_denote_norm] ofRuntimeOuterFrame
    (@LeanerIR.Proofs.Denote.Skolems.ofRuntime _ ?frame _ _) := fun e => do
  let_expr LeanerIR.Proofs.Denote.Skolems.ofRuntime unit frame τ value := e | return .continue
  let some (θ, outer, row) := inducedFrame? frame | return .continue
  let some substituted := substituteNTy row τ | return .continue
  let carriers := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.Skolems.toCarriers) unit outer
  return .visit (mkApp4 (mkConst ``LeanerIR.Proofs.Denote.NTy.toSkolem) carriers θ τ
    (mkApp4 e.getAppFn unit outer substituted value))

/-- A value carried from an induced frame to the runtime family, as the
outer frame carries it (`Skolems.toRuntime_instantiate`, `ofRuntimeOuterFrame`). -/
dsimproc ↓ [lir_denote, lir_denote_norm] toRuntimeOuterFrame
    (@LeanerIR.Proofs.Denote.Skolems.toRuntime _ ?frame _ _) := fun e => do
  let_expr LeanerIR.Proofs.Denote.Skolems.toRuntime unit frame τ value := e | return .continue
  let some (θ, outer, row) := inducedFrame? frame | return .continue
  let some substituted := substituteNTy row τ | return .continue
  let carriers := mkApp2 (mkConst ``LeanerIR.Proofs.Denote.Skolems.toCarriers) unit outer
  return .visit (mkApp4 e.getAppFn unit outer substituted
    (mkApp4 (mkConst ``LeanerIR.Proofs.Denote.NTy.ofSkolem) carriers θ τ value))

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
    unless τ.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.enum 5 do return none
    return some (value.getArg! 0, value.getArg! 1,
      mkApp5 τ.getAppFn (τ.getArg! 0) (τ.getArg! 1) (value.getArg! 2) (value.getArg! 3)
        (τ.getArg! 4),
      value.getArg! 4)
  let some (family, θ, isRow, ty, v) ← peelTransport? value | return none
  unless isRow do return some (family, θ, ty, v)
  if τ.isAppOfArity ``LeanerIR.Proofs.Denote.NTy.struct 3 then
    return some (family, θ, mkApp3 τ.getAppFn (τ.getArg! 0) (τ.getArg! 1) ty, v)
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
          -- A transported row states no type arguments, and encoding ignores
          -- them: the callee's are spelled as the caller's.
          | (``LeanerIR.Proofs.Denote.NTy.struct, #[source, arguments, _]) =>
              pure (some (θ, mkApp3 (mkConst ``LeanerIR.Proofs.Denote.NTy.struct) source arguments
                row, v))
          | _ => pure none
      | some ``LeanerIR.Proofs.Denote.variantCarrier.ofSkolem, #[_, θ, names, rows, v] =>
          match (← whnfR callerType).getAppFnArgs with
          | (``LeanerIR.Proofs.Denote.NTy.enum, #[source, arguments, _, _, distinct]) =>
              pure (some (θ, mkApp5 (mkConst ``LeanerIR.Proofs.Denote.NTy.enum) source arguments
                names rows distinct, v))
          | _ => pure none
      | _, _ => pure none
    let some (θ, τ, v) := callee? | return .continue
    let proof := mkAppN (mkConst ``LeanerIR.Proofs.Denote.NTy.encode_ofSkolem) #[outer, θ, τ, v]
    let some (_, _, encoded) := (← inferType proof).eq? | return .continue
    let proof ← mkExpectedTypeHint proof (← mkEq e encoded)
    return .visit { expr := encoded, proof? := some proof }

/-- A callee's value in the caller's view holds the variant it holds at the
callee's family (`variantName_ofSkolem`). -/
simproc ↓ [lir_denote, lir_denote_norm] variantNameTransport
    (@LeanerIR.Proofs.Denote.variantName _ ?skolems _ _ _) :=
  fun e => do
    let_expr LeanerIR.Proofs.Denote.variantName unit outer _ _ value := e | return .continue
    let_expr LeanerIR.Proofs.Denote.variantCarrier.ofSkolem _ θ names rows v := value
      | return .continue
    let proof := mkAppN (mkConst ``LeanerIR.Proofs.Denote.variantName_ofSkolem)
      #[unit, outer, θ, names, rows, v]
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
  LeanerIR.Proofs.Denote.variantName_inr LeanerIR.Proofs.Denote.variantPayload_inl
  LeanerIR.Proofs.Denote.variantPayload_inr LeanerIR.Proofs.Denote.HList.encode_cons
  LeanerIR.Proofs.Denote.HList.encode_nil LeanerIR.Proofs.Denote.NTy.encode_int
/-- Comparing a constructed closure with a specification literal compares its
runtime captures. Keep the encoding intact elsewhere for invocation dispatch. -/
theorem typedClosure_eq_literal {unit : Validation.ValidatedUnit} [Skolems unit]
    (parameters : NRow) (shared : List Bool) (results : NRow)
    (handle : FunctionHandle) (mask : Nat) (inst : Array (TypeId × TypeId))
    {captured : NRow} (captures : HList captured)
    (typed : Carriers.closureTyped parameters shared results (closureOf handle mask inst captures))
    (other : FunctionHandle) (otherMask : Nat) (otherInst : Array (TypeId × TypeId))
    (otherCaptures : Array RuntimeValue) :
    ((NTy.function parameters shared results).encode
        (typedClosure parameters shared results (closureOf handle mask inst captures) typed) =
      RuntimeValue.closure other otherMask otherInst otherCaptures) ↔
    (RuntimeValue.closure handle mask inst (HList.encode captures).toArray =
      RuntimeValue.closure other otherMask otherInst otherCaptures) := Iff.rfl

attribute [lir_denote_norm] typedClosure_eq_literal RuntimeValue.closure.injEq
attribute [lir_denote_norm] LeanerIR.Proofs.wp_choose LeanerIR.Proofs.wp_assume
attribute [lir_denote_norm] LeanerIR.Proofs.encodedKeepsMemory_encode
  LeanerIR.Proofs.encodedKeepsMemory_function
  LeanerIR.Proofs.encodedFramed_encode LeanerIR.Proofs.encodedFramed_function
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.typedClosure_val
-- A clause's Boolean literal, decided, is the runtime's; a Boolean's truth,
-- decided, is the Boolean.

-- A value an encoding states encodes a value of its native type: before
-- the encodings' equation is taken apart.
attribute [lir_denote_norm ↓] exists_apply_eq_apply

attribute [lir_denote_norm] decide_false decide_true Bool.decide_eq_true Bool.decide_eq_false
-- A branch on a condition neither arm reads, as `omega` splits it.
attribute [lir_denote_norm] dite_eq_ite
-- A clause's encoded vectors: equal to another encoding exactly when the
-- arrays are, equal to a literal exactly when the literal decodes to the
-- values.
/-- The sequence of integers literal elements hold, if each element is
one. -/
private def literalIntegers? (elements : Lean.Expr) (value? : Lean.Expr → Option Lean.Expr) :
    MetaM (Option Lean.Expr) := do
  let some (_, elements) := elements.listLit? | return none
  let some values := elements.mapM value? | return none
  return some (← mkListLit (mkConst ``Int) values)

/-- `[a, …][k]?.getD 0`, the read of a literal sequence of integers. -/
private def literalIntegerRead (values index : Lean.Expr) : MetaM Lean.Expr := do
  mkAppM ``Option.getD #[← mkAppM ``GetElem?.getElem? #[values, index], mkIntLit 0]

/-- A specification function's read of a literal sequence of integers at an
arbitrary position, as the read of the sequence of their values: the
encoding of a certified element (`((xs[k]?.map encode).getD unit).asInt`)
or an encoded element (`([integer a, …][k]?.getD unit).asInt`) becomes
`[a, …][k]?.getD 0`, as the program's read does. -/
simproc [lir_denote_norm] literalEncodedRead (RuntimeValue.asInt _) := fun e => do
  if e.hasLooseBVars then return .continue
  let read := e.appArg!
  unless read.isAppOfArity ``Option.getD 3 && (read.getArg! 2).isConstOf ``RuntimeValue.unit do
    return .continue
  let lookup := read.getArg! 1
  if lookup.isAppOfArity ``Option.map 4 then
    let encoder := lookup.getArg! 2
    unless encoder.isAppOfArity ``LeanerIR.Proofs.Codec.encode 3 &&
        (encoder.getArg! 2).isAppOfArity ``LeanerIR.Proofs.Codec.specInt 2 do
      return .continue
    let inner := lookup.getArg! 3
    unless inner.isAppOfArity ``GetElem?.getElem? 7 do return .continue
    let some values ← literalIntegers? (inner.getArg! 5) fun element =>
        if element.isAppOfArity ``LeanerIR.SpecInt.mk 4 then some (element.getArg! 2) else none
      | return .continue
    let index := inner.getArg! 6
    let rhs ← literalIntegerRead values index
    let codec := encoder.getArg! 2
    let proof := mkAppN (mkConst ``asInt_getD_map_encode_getElem?)
      #[codec.getArg! 0, codec.getArg! 1, inner.getArg! 5, index]
    return .done { expr := rhs, proof? := some (← mkExpectedTypeHint proof (← mkEq e rhs)) }
  unless lookup.isAppOfArity ``GetElem?.getElem? 7 do return .continue
  let some values ← literalIntegers? (lookup.getArg! 5) fun element =>
      if element.isAppOfArity ``RuntimeValue.integer 1 then some element.appArg! else none
    | return .continue
  let index := lookup.getArg! 6
  let rhs ← literalIntegerRead values index
  let proof ← mkAppM ``asInt_getD_getElem?_map_integer #[values, index]
  return .done { expr := rhs, proof? := some (← mkExpectedTypeHint proof (← mkEq e rhs)) }

/-- The program's read of a literal sequence of certified integers at an
arbitrary position (`[⟨a, _⟩, …][k].val`), as the read of the sequence of
their values, `[a, …][k]?.getD 0`. -/
simproc [lir_denote_norm] literalCertifiedRead (LeanerIR.SpecInt.val _) := fun e => do
  if e.hasLooseBVars || !e.isAppOfArity ``LeanerIR.SpecInt.val 3 then return .continue
  let element := e.getArg! 2
  unless element.isAppOfArity ``GetElem.getElem 8 do return .continue
  let some values ← literalIntegers? (element.getArg! 5) fun element =>
      if element.isAppOfArity ``LeanerIR.SpecInt.mk 4 then some (element.getArg! 2) else none
    | return .continue
  let index := element.getArg! 6
  let rhs ← literalIntegerRead values index
  -- The read's bound may state the length as a literal.
  let proof := mkAppN (mkConst ``val_getElem_eq_getD_map_val)
    #[e.getArg! 0, e.getArg! 1, element.getArg! 5, index, element.getArg! 7]
  return .done { expr := rhs, proof? := some (← mkExpectedTypeHint proof (← mkEq e rhs)) }

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
-- Typed memory: a slot read after a write, a frame's resource types at the
-- runtime frame, and the values it carries as they are.
attribute [lir_denote, lir_denote_norm] LeanerIR.Proofs.Denote.Memory.set_same
  LeanerIR.Proofs.Denote.ite_some_some LeanerIR.Proofs.Denote.isSome_ite
  Bool.ite_eq_true_distrib if_false_left LeanerIR.Proofs.Denote.ite_true_or
-- A bitwise operation of a value with itself.
attribute [lir_denote_norm] LeanerIR.Proofs.IntegerArithmetic.bitwiseAnd_self
  LeanerIR.Proofs.IntegerArithmetic.bitwiseOr_self Nat.and_self Nat.or_self Nat.xor_self
-- A specification's bitwise operation on nonnegative operands, as the runtime computes it.
attribute [lir_denote_norm] LeanerIR.Proofs.IntegerArithmetic.bitwiseOr_nonnegative
  LeanerIR.Proofs.IntegerArithmetic.bitwiseXor_nonnegative
-- An empty vector, by its size.
attribute [lir_denote_norm] Array.isEmpty_iff_size_eq_zero
  LeanerIR.Proofs.Denote.array_isEmpty_eq_false
-- The data invariant a stored value's resource type selects.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.NTy.handle?_struct
  LeanerIR.Proofs.Denote.NTy.handle?_enum LeanerIR.Proofs.Denote.NTy.handle?_resolve_struct LeanerIR.Proofs.Denote.NTy.handle?_resolve_enum
  LeanerIR.Proofs.Denote.Skolems.encode_runtime
  LeanerIR.Proofs.Denote.Memory.set_other LeanerIR.Proofs.Denote.Memory.set_other_resource
  LeanerIR.Proofs.Denote.Skolems.resource LeanerIR.Proofs.Denote.NRow.map
  LeanerIR.Proofs.Denote.Skolems.resolve_runtime LeanerIR.Proofs.Denote.Skolems.ofRuntime_runtime
  LeanerIR.Proofs.Denote.Skolems.toRuntime_runtime LeanerIR.Proofs.Denote.ResourceType.mk.injEq
  LeanerIR.Proofs.Denote.Skolems.resolve_instantiate LeanerIR.Proofs.Denote.Skolems.ofRuntime_instantiate
  LeanerIR.Proofs.Denote.Skolems.toRuntime_instantiate
  LeanerIR.Proofs.Denote.NTy.struct.injEq LeanerIR.Proofs.Denote.NTy.enum.injEq
  LeanerIR.Proofs.Denote.NRow.cons.injEq LeanerIR.StructHandle.mk.injEq
  LeanerIR.Proofs.Denote.NTy.int.injEq LeanerIR.Proofs.Denote.NTy.vector.injEq
  LeanerIR.StorageKey.address.injEq LeanerIR.Proofs.Denote.NTy.encode_signer
  LeanerIR.Proofs.Denote.NTy.encode_address
-- A literal instantiation keys a family as the runtime does.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.instantiatedTypeId_cons

-- The function value naming a target spells its weave as a closure literal does.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.Weave.supplying_nil
  LeanerIR.Proofs.Denote.Weave.supplying_cons
  LeanerIR.Proofs.Denote.instantiatedTypeId_nil LeanerIR.TypeId.mk.injEq
-- A type parameter's value is encoded by its family's codec, and defaults
-- under an induced family to its argument's value.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.Carriers.default_instantiate
  LeanerIR.Proofs.Denote.NTy.inhabitant LeanerIR.Proofs.Denote.HList.inhabitant
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.NTy.codec_param
  LeanerIR.Proofs.Denote.NTy.encode_param LeanerIR.Proofs.Denote.Carriers.codec_instantiate
-- The type arguments of an opaque specification function, at an
-- instantiated family the caller's, at the public family themselves.
-- Before `resolve_instantiate`, which a parameter's type also matches.
attribute [lir_denote_norm high] LeanerIR.Proofs.Denote.Skolems.type_instantiate
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.Skolems.type_runtime LeanerIR.Proofs.Denote.NTy.substWith
  LeanerIR.Proofs.Denote.NRow.substWith LeanerIR.Proofs.Denote.NRows.substWith
-- A resource type at an induced frame is its substitution, evaluated, so
-- that a callee's slot and the caller's are one term.
attribute [lir_denote, lir_denote_norm] LeanerIR.Proofs.Denote.NTy.subst
  LeanerIR.Proofs.Denote.NRow.subst LeanerIR.Proofs.Denote.NRows.subst
  LeanerIR.Proofs.Denote.NRow.getD
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
  Int.compare_eq_gt Int.compare_eq_eq LeanerIR.RuntimeValue.order_vector
  LeanerIR.RuntimeValue.order_nominal LeanerIR.RuntimeValue.orderList_nil_nil
  LeanerIR.RuntimeValue.orderList_nil_cons LeanerIR.RuntimeValue.orderList_cons_nil
  LeanerIR.RuntimeValue.orderList_cons_cons List.toList_toArray
-- A search over a whole vector, as membership.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.getD_map_field
-- A vector construction whose representability is decided, and the sizes
-- of in-bounds insertion and removal. Whether an insertion or removal is
-- in bounds is an arithmetic side condition, decided at a leaf by omega
-- (`leaner_denote_lookups_after_writes`), not by the normalization, whose
-- default discharger would run it over the whole set.
attribute [lir_denote_norm] Option.dite_none_right_eq_some
  Array.size_insertIdx Array.size_eraseIdx
theorem self_eq_extract_iff {α : Type} {as : Array α} {i j : Nat} :
    as = as.extract i j ↔ as.size = 0 ∨ i = 0 ∧ as.size ≤ j := by
  rw [eq_comm]; exact Array.extract_eq_self_iff

-- A slice, read by its size and elements.
attribute [lir_denote_norm] Array.extract_size Array.size_extract Array.getElem?_extract
  Array.extract_eq_self_iff self_eq_extract_iff Array.extract_size_left
-- An operation on encoded arrays, as the encoding of its result.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.setIfInBounds_map
  LeanerIR.Proofs.Denote.extract_map LeanerIR.Proofs.Denote.push_map
  LeanerIR.Proofs.Denote.append_map
-- A range reversal, read by its size and elements.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.size_reverseSlice
  LeanerIR.Proofs.Denote.getElem?_reverseSlice LeanerIR.Proofs.Denote.reverseSlice_reverseSlice
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
  LeanerIR.Proofs.wp_abort LeanerIR.Proofs.wp_given LeanerIR.Proofs.Spec.pure_bind
  LeanerIR.Proofs.Spec.abort_bind
  LeanerIR.Proofs.Denote.ResultShape.bodyType Bool.not_eq_true decide_eq_true_eq Bool.and_eq_true
  Bool.or_eq_true and_assoc exists_and_left exists_and_right exists_eq_left exists_eq_left'
  and_true true_and and_imp forall_and forall_eq forall_eq' Prod.mk.injEq exists_eq exists_eq'
  and_false false_and not_false_eq_true not_true_eq_false true_implies false_implies implies_true
  true_or or_true false_or or_false
  imp_self eq_self_iff_true ite_true ite_false Bool.false_eq_true Bool.true_eq_false
  Bool.not_eq_false decide_eq_false_iff_not Bool.and_eq_false_imp Bool.not_true Bool.not_false
  Bool.not_not Bool.not_eq_false' Bool.not_eq_true' Option.some.injEq Option.elim_some Option.elim_none LeanerIR.RuntimeValue.field
  LeanerIR.RuntimeValue.asInt LeanerIR.RuntimeValue.asBool LeanerIR.RuntimeValue.asString
  LeanerIR.packResults_nil LeanerIR.packResults_single LeanerIR.packResults_many
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
  LeanerIR.Proofs.Denote.nat_succ_eq_iff
  LeanerIR.Proofs.Denote.NTy.codec_encode Option.bind_some Option.bind_none
  Option.getD_none LeanerIR.Proofs.Denote.HList.encode_inj
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
  -- A literal vector updated at a position not known literally, and a
  -- literal vector transported elementwise.
  List.length_insertIdx List.getElem?_insertIdx List.length_eraseIdx List.getElem?_eraseIdx
  List.length_map List.getElem?_map List.getElem_cons_zero List.getElem_cons_succ
  List.getElem?_nil List.getElem?_cons_zero List.getElem?_cons_succ
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
-- A value transported into a callee's view and back is the value, also
-- element by element.
attribute [lir_denote_norm] LeanerIR.Proofs.Denote.NTy.ofSkolem_toSkolem
  LeanerIR.Proofs.Denote.NTy.toSkolem_ofSkolem LeanerIR.Proofs.Denote.HList.ofSkolem_toSkolem
  LeanerIR.Proofs.Denote.HList.toSkolem_ofSkolem
  LeanerIR.Proofs.Denote.variantCarrier.ofSkolem_toSkolem
  LeanerIR.Proofs.Denote.variantCarrier.toSkolem_ofSkolem Array.map_map List.map_map
-- The order of integers and of booleans is their comparison, computed on
-- boolean literals.
attribute [lir_denote_norm] LeanerIR.RuntimeValue.order_integer Int.compare_eq_lt
  Int.compare_eq_gt Int.compare_eq_eq LeanerIR.RuntimeValue.order_bool
@[lir_denote_norm] theorem compare_false_false : compare false false = Ordering.eq := rfl
@[lir_denote_norm] theorem compare_false_true : compare false true = Ordering.lt := rfl
@[lir_denote_norm] theorem compare_true_false : compare true false = Ordering.gt := rfl
@[lir_denote_norm] theorem compare_true_true : compare true true = Ordering.eq := rfl
-- A position a pinned integer gives is a literal, and so are the
-- comparisons of the lengths it computes.
attribute [lir_denote_norm] Int.reduceToNat Nat.reduceLeDiff Nat.reduceLT Int.reduceLT
  Int.reduceLE

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
    unless action.isAppOfArity ``LeanerIR.Proofs.Denote.loopAt 7 do return none
    let some site ← (evalNat (action.getArg! 4)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The in-body assertion a goal's weakest precondition is over, if any: its
site, and the arguments of `wp_assertAt` besides the condition. -/
private def assertionGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``LeanerIR.Proofs.Denote.assertAt 6 do return none
    let some site ← (evalNat (action.getArg! 4)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The end of a mutation a goal's weakest precondition is over, if any: its
site, and the arguments of `wp_mutationEndAt` besides the invariant. -/
private def mutationEndGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``LeanerIR.Proofs.Denote.mutationEndAt 6 do return none
    let some site ← (evalNat (action.getArg! 4)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The end of a write of global memory a goal's weakest precondition is
over, if any: its site, and the arguments of `wp_memoryWrittenAt` besides the
invariant. -/
private def memoryWrittenGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``LeanerIR.Proofs.Denote.memoryWrittenAt 6 do return none
    let some site ← (evalNat (action.getArg! 4)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The call a goal's weakest precondition is over, if any: its site, and
the arguments of `wp_callAt`. -/
private def callAtGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``LeanerIR.Proofs.Denote.callAt 5 do return none
    let some site ← (evalNat (action.getArg! 3)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The in-body assumption a goal's weakest precondition is over, if any:
the arguments of `wp_assumeAt` before its continuation. -/
private def assumeGoal? (goal : MVarId) : MetaM (Option (Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``LeanerIR.Proofs.Denote.assumeAt 7 do return none
    return some (action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The invariants owed where a call returns and what follows, if a goal
is over them (`CallChecked`). -/
private def callCheckedGoal? (goal : MVarId) : MetaM (Option (Lean.Expr × Lean.Expr)) :=
  goal.withContext do
    let target := (← instantiateMVars (← goal.getType)).headBeta
    unless target.isAppOfArity ``LeanerIR.Proofs.Denote.CallChecked 2 do return none
    return some (target.getArg! 0, target.getArg! 1)

/-- The construction a goal's weakest precondition is over, if any: its
site, and the arguments of `wp_constructedAt` besides the invariant. -/
private def constructionGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``LeanerIR.Proofs.Denote.constructedAt 8 do return none
    let some site ← (evalNat (action.getArg! 5)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The state anchor a goal's weakest precondition is over, if any: its
site, and the arguments of `wp_anchorAt`. -/
private def anchorGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``LeanerIR.Proofs.Denote.anchorAt 6 do return none
    let some site ← (evalNat (action.getArg! 4)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- A predicate over the locals and state of state anchors, applied to those
the anchors recorded last on the path. -/
private partial def resolveAnchors (goal : MVarId) (predicate : Lean.Expr) :
    MetaM Lean.Expr := goal.withContext do
  let .lam _ type _ _ := predicate | return predicate
  unless type.isAppOfArity ``LeanerIR.Proofs.Denote.AnchorOf 4 do return predicate
  let some site ← (evalNat (type.getArg! 2)).run
    | throwError m!"an anchor site must be a numeral, not {type.getArg! 2}"
  let mut saved := none
  for declaration in ← getLCtx do
    let recorded ← instantiateMVars declaration.type
    if recorded.isAppOfArity ``AnchorSaved 5 then
      if (← (evalNat (recorded.getArg! 2)).run) == some site then
        saved := some (recorded.getArg! 3, recorded.getArg! 4)
  let some (env, state) := saved
    | throwError m!"the state anchor at site {site} is read where it is not recorded"
  resolveAnchors goal (← whnfR (mkApp predicate (← mkAppM ``Prod.mk #[env, state])))

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
the premises that remain.  The hypothesis concludes the iteration's weakest
precondition; it binds the row, and the state unless the invariant fixed
it. -/
private def recursiveHypothesis? (goal : MVarId) : TacticM (Option (List MVarId)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
    let recursive := (target.getArg! 3).getAppFn
    unless recursive.isFVar do return none
    -- A call in the loop can leave a source marker on the abort continuation,
    -- while normalization has removed it from the induction hypothesis.
    -- `apply` need not unfold that semireducible marker during unification.
    -- The marker is definitionally its proposition; keep the back edge in
    -- the same form as the hypothesis before matching it.
    let stripAbortMarkers (type : Lean.Expr) := type.replace fun e =>
      if e.isAppOfArity ``LeanerIR.Proofs.wp 7 then
        let args := e.getAppArgs
        let abort := args[5]!.replace fun marker =>
          if marker.isAppOfArity ``LeanerIR.Proofs.Obligation 4 then some (marker.getArg! 3) else none
        some (mkAppN e.getAppFn (args.set! 5 abort))
      else none
    let unmarked := stripAbortMarkers target
    (← getLCtx).findDeclM? fun decl => do
      if decl.isImplementationDetail then return none
      let ty ← instantiateMVars decl.type
      unless ty.isForall do return none
      let conclusion := ty.getForallBody
      unless conclusion.isAppOfArity ``LeanerIR.Proofs.wp 7 &&
          (conclusion.getArg! 3).getAppFn == recursive do return none
      let saved ← saveState
      try
        -- Replacing a target assigns the old metavariable. Keep that mutation
        -- inside the saved attempt, so a mismatch leaves the original goal live.
        let goal ← if unmarked == target then pure goal else goal.replaceTargetDefEq unmarked
        -- Preserve invariant-premise markers: they identify a failed step's
        -- exact source clause. Only the final abort continuation must agree.
        let hypothesisType := stripAbortMarkers ty
        let hypothesis := mkExpectedPropHint decl.toExpr hypothesisType
        let subgoals ← goal.apply hypothesis
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
      catch failure =>
        saved.restore
        if leaner.denoteDebug.get (← getOptions) then
          IO.println s!"recursive hypothesis mismatch: {← failure.toMessageData.toString}"
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
    if action.isAppOfArity ``LeanerIR.Proofs.Denote.propheticMeaning 8 then
      return some (action.getArg! 4, action)
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

/-- A lemma over a frame applied at the unit, executable, and frame a term
states, its other implicit arguments inferred: no frame is synthesized from a
goal's context, which holds none. -/
private def mkAppAtFrame (name : Name) (unit executable frame : Lean.Expr)
    (arguments : Array Lean.Expr) : MetaM Lean.Expr :=
  mkAppM' (mkApp3 (mkConst name) unit executable frame) arguments

/-- The native value a runtime value encodes at a type: a certified
integer's value read back, or certified by its bounds; a boolean, an
address, or a signer as itself. -/
private def nativeOf? (τ value : Lean.Expr) : TacticM (Option Lean.Expr) := do
  if value.isAppOfArity ``NTy.encode 3 then
    return some value.appArg!
  match value.getAppFn.constName?, τ.getAppFn.constName? with
  | some ``RuntimeValue.integer, some ``NTy.int =>
      let integer := value.appArg!
      let width ← mkAppM ``LeanerIR.IntWidth.bits #[τ.getArg! 0]
      let signed := τ.getArg! 1
      if integer.isAppOfArity ``SpecInt.val 3 then
        let native := integer.appArg!
        if ← isDefEq (← inferType native) (mkApp2 (mkConst ``SpecInt) width signed) then
          return some native
      let fits := mkApp3 (mkConst ``LeanerIR.IntegerValueFits) width signed integer
      let certificate ← mkFreshExprSyntheticOpaqueMVar fits
      let certified ← closesBy certificate.mvarId! (← `(tactic|
        (simp only [LeanerIR.Proofs.Denote.IntegerValueFits_unsigned_succ,
          LeanerIR.Proofs.Denote.IntegerValueFits_signed_succ]; omega)))
      unless certified do return none
      return some (mkApp4 (mkConst ``SpecInt.mk) width signed integer
        (← instantiateMVars certificate))
  | some ``RuntimeValue.bool, some ``NTy.bool
  | some ``RuntimeValue.address, some ``NTy.address
  | some ``RuntimeValue.signer, some ``NTy.signer => return some value.appArg!
  | _, _ => return none

/-- The native arguments the runtime values of a row encode. -/
private partial def nativeRow? (row : Lean.Expr) :
    List Lean.Expr → TacticM (Option Lean.Expr)
  | [] => return if row.isConstOf ``NRow.nil then some (mkConst ``Unit.unit) else none
  | value :: values => do
      unless row.isAppOfArity ``NRow.cons 2 do return none
      let some head ← nativeOf? (row.getArg! 0) value | return none
      let some tail ← nativeRow? (row.getArg! 1) values | return none
      return some (← mkAppM ``Prod.mk #[head, tail])

/-- The elements of a list or array literal. -/
private def literalElements? (literal : Lean.Expr) : Option (List Lean.Expr) :=
  if literal.isAppOfArity ``List.toArray 2 then
    literal.appArg!.listLit?.map (·.2)
  else literal.listLit?.map (·.2)

/-- A statement without its obligation marker. -/
private def unmarked (statement : Lean.Expr) : Lean.Expr :=
  if statement.isAppOfArity ``LeanerIR.Proofs.Obligation 4 then statement.appArg! else statement

/-- Whether a statement is `ensures_of` or `aborts_of` under premises. -/
private partial def states (statement : Lean.Expr) : Bool :=
  if statement.isArrow then states statement.bindingBody!
  else
    let statement := unmarked statement
    statement.isAppOf ``LeanerIR.Proofs.EnsuresOf || statement.isAppOf ``LeanerIR.Proofs.AbortsOf

/-- Behavioral contract derivation only handles literal closures. An abstract
function value has no target theorem to re-derive, so trying to avoid that
work by preparing the whole context would add only speculative cost. -/
private partial def statesLiteralBehavior (statement : Lean.Expr) : Bool :=
  if statement.isArrow then statesLiteralBehavior statement.bindingBody!
  else
    let statement := unmarked statement
    (statement.isAppOfArity ``LeanerIR.Proofs.EnsuresOf 7 ||
      statement.isAppOfArity ``LeanerIR.Proofs.AbortsOf 5) &&
      ((statement.getArg! 2).find? (·.isAppOfArity ``closureOf 7)).isSome

/-- Whether a premise of a goal's binders states `ensures_of` or `aborts_of`. -/
private partial def spineStates : Lean.Expr → Bool
  | .forallE _ domain body _ => states domain || spineStates body
  | _ => false

/-- A goal's `ensures_of` and `aborts_of` as hypotheses: the premises of its
spine that state them, and a negated one, whose clause keeps its marker on
`False`. -/
private def introduceBehaviors (goal : MVarId) : TacticM MVarId := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let goal ← if spineStates target then pure (← introduceThroughMarkers goal).1 else pure goal
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let negated := unmarked target
  unless negated.isAppOfArity ``Not 1 && states negated.appArg! do return goal
  let goal ← if target.isAppOfArity ``LeanerIR.Proofs.Obligation 4 then
      let marked := mkAppN target.getAppFn (target.getAppArgs.pop.push (mkConst ``False))
      goal.replaceTargetDefEq (.forallE `leanerAborts negated.appArg! marked .default)
    else pure goal
  return (← goal.intro1P).2

/-- What a leaf learns from `ensures_of` and `aborts_of` of a closure whose
target, weave, and captures it sees, and whose target is a verified
callee: where the leaf establishes the target's precondition at the woven
arguments, what the target's contract ensures of the run, or the failures it
permits (`ensuresOf_closureOf_verified`, `abortsOf_closureOf_verified`). -/
private def dispatchBehavior (goal : MVarId) (callees : Array (Lean.Expr × String × Lean.Expr)) :
    TacticM MVarId := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  -- `requires_of`: the target's entry in the table.
  if (unmarked target).isAppOf ``LeanerIR.Proofs.RequiresOf then
    let state ← saveState
    try
      setGoals [goal]
      evalTactic (← `(tactic| leaner_denote_behavior))
      return ← getMainGoal
    catch _ => state.restore
  let goal ← introduceBehaviors goal
  goal.withContext do
  let mut current := goal
  for hypothesis in ← goal.getNondepPropHyps do
    -- A predicate under premises the leaf holds, as a caller's contract
    -- states it under the absence of the failures it declares.
    let mut type ← instantiateMVars (← hypothesis.getType)
    unless states type do continue
    let mut stated := Lean.Expr.fvar hypothesis
    let mut held := true
    while type.isArrow do
      let some premise ← findLocalDeclWithType? type.bindingDomain! | held := false; break
      stated := mkApp stated (.fvar premise)
      type := type.bindingBody!
    unless held do continue
    -- A clause's marker is its statement.
    if type.isAppOfArity ``LeanerIR.Proofs.Obligation 4 then
      stated := mkApp4 (mkConst ``Iff.mp) type type.appArg!
        (mkAppN (mkConst ``LeanerIR.Proofs.Obligation_iff) type.getAppArgs) stated
      type := type.appArg!
    let ensures := type.isAppOfArity ``LeanerIR.Proofs.EnsuresOf 7
    unless ensures || type.isAppOfArity ``LeanerIR.Proofs.AbortsOf 5 do continue
    let some closure := (type.getArg! 2).find? (·.isAppOfArity ``closureOf 7) | continue
    let mask := closure.getArg! 3
    unless mask.isAppOfArity ``Weave.mask 4 do continue
    let weave := mask.appArg!
    let rows := (← whnfR (← inferType weave)).getAppArgs
    let some captured := rows[1]? | continue
    let some supplied := rows[2]? | continue
    let handle := closure.getArg! 2
    -- A target the call rule inlines is listed by its theorem as well.
    let some (_, _, theoremProof) ← callees.findM? fun (candidate, _, proof) => do
        unless ← isDefEq candidate handle do return false
        forallTelescope (← inferType proof) fun _ statement =>
          pure (statement.isAppOfArity ``LeanerIR.Proofs.Satisfies 6)
      | continue
    let (theoremArguments, _, statement) ← forallMetaTelescope (← inferType theoremProof)
    unless statement.isAppOfArity ``LeanerIR.Proofs.Satisfies 6 do continue
    let meaning := statement.getArg! 4
    unless meaning.isAppOf ``propheticMeaning do continue
    let shape ← whnfD meaning.appArg!
    -- Fix the target theorem's family before unifying its dependent contract.
    -- Scalar argument carriers alone do not determine this implicit family.
    unless ← isDefEq (meaning.getArg! 2) (closure.getArg! 1) do continue
    let verified ← instantiateMVars (mkAppN theoremProof theoremArguments)
    let some arguments := literalElements? (type.getArg! 3) | continue
    let some natives ← nativeRow? supplied arguments | continue
    -- Reference-freedom of a row or type, decided by the kernel.
    let free := fun (statement : Lean.Expr) => do
      mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true)) (← mkEq statement (mkConst ``Bool.true))
    -- A closure of a target without type arguments runs at the empty
    -- instantiation, which the runtime frame is coherent with.
    let coherent ← mkAppM ``coherent_runtime #[meaning.getArg! 0, handle]
    let implication? ← try
        if ensures && shape.isConstOf ``ResultShape.none then
          -- A run that returns nothing.
          let some [] := literalElements? (type.getArg! 4) | continue
          some <$> mkAppAtFrame ``LeanerIR.Proofs.ensuresOf_closureOf_verified_shape
              (type.getArg! 0) (type.getArg! 1) (closure.getArg! 1)
            #[weave, ← free (mkApp (mkConst ``NRow.refFree) captured),
              ← free (mkApp (mkConst ``NRow.refFree) supplied),
              ← free (mkApp (mkConst ``NRow.refFree) (← mkAppM ``ResultShape.row #[shape])),
              closure.getArg! 4, coherent, closure.appArg!, natives, mkConst ``Unit.unit,
              verified, stated]
        else if ensures then
          unless shape.isAppOfArity ``ResultShape.one 1 do continue
          let τ := shape.appArg!
          let some [result] := literalElements? (type.getArg! 4) | continue
          let some native ← nativeOf? τ result | continue
          some <$> mkAppAtFrame ``LeanerIR.Proofs.ensuresOf_closureOf_verified
              (type.getArg! 0) (type.getArg! 1) (closure.getArg! 1)
            #[weave, ← free (mkApp (mkConst ``NRow.refFree) captured),
              ← free (mkApp (mkConst ``NRow.refFree) supplied), ← free (mkApp (mkConst ``NTy.refFree) τ),
              closure.getArg! 4, coherent, closure.appArg!, natives, native, verified, stated]
        else
          some <$> mkAppAtFrame ``LeanerIR.Proofs.abortsOf_closureOf_verified
              (type.getArg! 0) (type.getArg! 1) (closure.getArg! 1)
            #[weave, ← free (mkApp (mkConst ``NRow.refFree) captured),
              ← free (mkApp (mkConst ``NRow.refFree) supplied), closure.getArg! 4, coherent,
              closure.appArg!, natives, verified, stated]
      catch ex =>
        if leaner.denoteDebug.get (← getOptions) then
          IO.println s!"behavior dispatch skipped: {← ex.toMessageData.toString}"
        pure none
    let some implication := implication? | continue
    -- The natives the target's theorem assumes, the caller assumes too.
    let mut assumed := true
    for argument in theoremArguments do
      if let .mvar id ← instantiateMVars argument then
        unless ← id.isAssigned do
          try id.assumption catch _ => assumed := false
    unless assumed do
      if leaner.denoteDebug.get (← getOptions) then IO.println "behavior dispatch: theorem assumptions missing"
      continue
    let constants ← contractConstants (← inferType theoremProof)
    let lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ← constants.mapM fun name =>
      `(Lean.Parser.Tactic.simpLemma| $(mkIdent (rootNamespace ++ name)):ident)
    -- What the target's theorem assumes, then its precondition.
    let mut fact ← instantiateMVars implication
    let mut established := true
    for _ in [0:2] do
      let .forallE _ premiseType _ _ ← whnfR (← inferType fact) | established := false; break
      let premise ← mkFreshExprSyntheticOpaqueMVar premiseType
      unless ← closesBy premise.mvarId! (← `(tactic|
          ((try simp only [$lemmas,*, Weave.compose, lir_denote_norm]) <;> leaner_denote_leaf))) do
        established := false
        break
      fact := mkApp fact (← instantiateMVars premise)
    unless established do
      if leaner.denoteDebug.get (← getOptions) then IO.println "behavior dispatch: contract premises unproved"
      continue
    let (_, next) ← (← current.assert `leanerBehavior (← inferType fact) fact).intro1P
    current := next
    let state ← saveState
    try
      setGoals [current]
      evalTactic (← `(tactic| simp only [$lemmas,*, Weave.compose, lir_denote_norm,
        not_false_eq_true, true_implies] at $(mkIdent `leanerBehavior):ident))
      current ← getMainGoal
    catch _ => state.restore
  return current

/-- The facts a proof states: its conjuncts, recursively. -/
private partial def conjuncts (proof : Lean.Expr) : MetaM (Array Lean.Expr) := do
  let type ← whnfR (← instantiateMVars (← inferType proof))
  if type.isAppOfArity ``And 2 then
    return (← conjuncts (← mkAppM ``And.left #[proof])) ++
      (← conjuncts (← mkAppM ``And.right #[proof]))
  return #[proof]

/-- A runtime value built of encodings typed at a closed semantic type:
scalars, integers by their certified bounds or a literal's, and structures
whose declaration the unit gives, by evaluation over it. -/
syntax "leaner_has_type" : tactic
/-- A row of such values typed at a row of types. -/
syntax "leaner_has_types" : tactic
macro_rules
  | `(tactic| leaner_has_type) => `(tactic| first
      | exact LeanerIR.HasType.bool _
      | exact LeanerIR.HasType.address _
      | exact LeanerIR.HasType.signer _
      | exact LeanerIR.HasType.string _
      | exact LeanerIR.HasType.bytes _
      | exact LeanerIR.HasType.unit
      | (refine LeanerIR.HasType.integer _ _ _ ?_
         first | exact LeanerIR.Proofs.holdsAt_val _ _ | decide)
      | (refine LeanerIR.HasType.nominal _ _ _ _ [] _ _ _ rfl rfl
           (LeanerIR.Proofs.ResolvesAll.ofResolveIn rfl) ?_
         leaner_has_types))
macro_rules
  | `(tactic| leaner_has_types) => `(tactic| first
      | exact LeanerIR.HasTypes.nil
      | (refine LeanerIR.HasTypes.cons ?_ ?_
         · leaner_has_type
         · leaner_has_types))

/-- A memory's runtime encodings typed: every memory's are, for a unit whose
resource types correspond to their keys' types, which a hypothesis states
(`memoryTyped`). -/
private def proveMemoryTyped (facts : Array Lean.Expr) (goal : MVarId) : TacticM Bool := do
  let type ← goal.withContext do instantiateMVars (← goal.getType)
  let_expr LeanerIR.Proofs.MemoryTyped unit memory := type | return false
  for fact in facts do
    let factType ← goal.withContext do instantiateMVars (← inferType fact)
    let_expr LeanerIR.Proofs.ResourcesTyped factUnit := factType | continue
    if ← goal.withContext (isDefEq factUnit unit) then
      goal.assign (mkApp3 (mkConst ``LeanerIR.Proofs.memoryTyped) unit fact memory)
      return true
  return false

/-- Inspect a frame under a source clause's diagnostic marker. -/
private partial def frameFactType (type : Lean.Expr) : MetaM Lean.Expr := do
  let type ← instantiateMVars type
  if type.isAppOfArity ``LeanerIR.Proofs.Obligation 4 then
    frameFactType type.appArg!
  else pure type

/-- Remove diagnostic markers from a frame proof before applying it. -/
private partial def frameFactProof (fact : Lean.Expr) : MetaM Lean.Expr := do
  let type ← instantiateMVars (← inferType fact)
  if type.isAppOfArity ``LeanerIR.Proofs.Obligation 4 then
    frameFactProof (mkApp4 (mkConst ``Iff.mp) type type.appArg!
      (mkAppN (mkConst ``LeanerIR.Proofs.Obligation_iff) type.getAppArgs) fact)
  else pure fact

/-- The parameter row and function value of a stored frame hypothesis. -/
private def framedFact? (type : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let type ← frameFactType type
  if type.isAppOfArity ``LeanerIR.Proofs.KeepsMemoryAt 5 then
    return some (type.getArg! 3, type.getArg! 4)
  if type.isAppOfArity ``LeanerIR.Proofs.FramedAt 6 then
    return some (type.getArg! 3, type.getArg! 5)
  return none

/-- A run of a function value a hypothesis frames (`KeepsMemoryAt`,
`FramedAt`) keeps the frame: of a value that keeps memory, the run's final
memory, `leanerFinal`, is substituted by its initial one; of another, its
frame at the run's arguments is a hypothesis. -/
private def keptMemory (goal : MVarId) (action : Lean.Expr) (facts : Array Lean.Expr) :
    TacticM MVarId := do
  let kept? ← goal.withContext do
    let some ensures := (← getLCtx).findFromUserName? `leanerEnsures | return none
    let ensuresType ← instantiateMVars ensures.type
    unless ensuresType.isAppOfArity ``LeanerIR.Proofs.EnsuresOf 7 do return none
    for fact in facts do
      let factType ← frameFactType (← inferType fact)
      let some (row, closure) ← framedFact? factType | continue
      unless ← isDefEq closure (action.getArg! 3) do
        if leaner.denoteDebug.get (← getOptions) then logInfo m!"frame closure mismatch: {closure} vs {action.getArg! 3}"
        continue
      unless ← isDefEq (factType.getArg! 1) (ensuresType.getArg! 1) do continue
      let typed ← mkFreshExprMVar (← mkAppM ``LeanerIR.Proofs.MemoryTyped
        #[ensuresType.getArg! 0, ensuresType.getArg! 5])
      unless ← proveMemoryTyped facts typed.mvarId! do continue
      let refFree := mkApp (mkConst ``NRow.refFree) row
      unless ← isDefEq refFree (mkConst ``Bool.true) do continue
      let free ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq refFree (mkConst ``Bool.true))
      let kept := mkAppN (← frameFactProof fact) #[free, action.getArg! 6, ensuresType.getArg! 4,
        ensuresType.getArg! 5, ensuresType.getArg! 6, ← instantiateMVars typed, ensures.toExpr]
      try check kept catch error =>
        if leaner.denoteDebug.get (← getOptions) then logInfo m!"frame application: {error.toMessageData}"
        continue
      return some (kept, factType.isAppOfArity ``LeanerIR.Proofs.KeepsMemoryAt 5)
    return none
  let some (kept, keeps) := kept? | return goal
  let (equation, next) ← goal.withContext do
    (← goal.assert `leanerKept (← whnfR (← inferType kept)).headBeta kept).intro1P
  -- Substitute using the frame equation itself. Substituting `leanerFinal`
  -- by name can select the earlier StateOf equation instead, leaving the
  -- memory hidden behind the invocation's state-label expression.
  if keeps then next.withContext (Lean.Meta.subst next equation) else pure next

/-- A row the frame resolves to scalars (`ScalarAt`): a hypothesis of the
theorem, or by evaluation where the frame is the runtime frame or one a
call's type arguments induce. -/
private def scalarAt? (facts : Array Lean.Expr) (goal : MVarId) : TacticM Bool := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``LeanerIR.Proofs.ScalarAt 3 do return false
  for fact in facts do
    let factType ← goal.withContext do instantiateMVars (← inferType fact)
    unless factType.isAppOfArity ``LeanerIR.Proofs.ScalarAt 3 do continue
    if ← goal.withContext do isDefEq factType target then
      goal.assign fact
      return true
  let state ← saveState
  try
    setGoals [goal]
    evalTactic (← `(tactic|
      (simp only [LeanerIR.Proofs.ScalarAt, LeanerIR.Proofs.Denote.NRow.resolved,
          LeanerIR.Proofs.Denote.Skolems.resolve_instantiate,
          LeanerIR.Proofs.Denote.Skolems.resolve_runtime,
          LeanerIR.Proofs.Denote.Skolems.resolve_paramFree, LeanerIR.Proofs.Denote.NTy.paramFree,
          LeanerIR.Proofs.Denote.NRow.paramFree, LeanerIR.Proofs.Denote.NRows.paramFree,
          LeanerIR.Proofs.Denote.NTy.subst, LeanerIR.Proofs.Denote.NRow.subst,
          LeanerIR.Proofs.Denote.NRows.subst, LeanerIR.Proofs.Denote.NRow.getD,
          Bool.and_true, Bool.true_and]
       first | rfl | decide)))
    unless (← getGoals).isEmpty do throwError "a row is not scalar"
    return true
  catch _ =>
    state.restore
    return false

/-- A function value a carrier holds, at rows the frame resolves to scalars:
typed by the carrier where the unit's readings agree, which a hypothesis
states (`ClosureTypedAt.ofCarrier`). -/
private def carrierClosureTyped? (facts : Array Lean.Expr) (goal : MVarId) : TacticM Bool := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``LeanerIR.Proofs.ClosureTypedAt 5 do return false
  unless (target.getArg! 4).consumeMData.isAppOfArity ``Subtype.val 3 do return false
  let state ← saveState
  try
    let premises ← goal.withContext do
      let rule ← mkConstWithFreshMVarLevels ``LeanerIR.Proofs.ClosureTypedAt.ofCarrier
      let (arguments, _, conclusion) ← forallMetaTelescope (← inferType rule)
      arguments[1]!.mvarId!.assign (target.getArg! 1)
      unless ← isDefEq conclusion target do throwError "not a carrier's value"
      let agreement ← instantiateMVars (← inferType arguments[2]!)
      let some fact ← facts.findM? fun fact => do
          let factType ← instantiateMVars (← inferType fact)
          pure (factType.isAppOfArity ``LeanerIR.Proofs.TypesAgree 1) <&&>
            isDefEq factType agreement
        | throwError "the unit's readings are not known to agree"
      arguments[2]!.mvarId!.assign fact
      goal.assign (mkAppN rule arguments)
      pure (arguments.extract (arguments.size - 3) arguments.size)
    for premise in premises do
      let .mvar id ← instantiateMVars premise | continue
      if ← id.isAssigned then continue
      let type ← id.getType
      if type.isAppOfArity ``Eq 3 then
        unless ← id.withContext (isDefEq premise (← mkEqRefl (type.getArg! 1))) do
          throwError "a row holds a reference"
      else unless ← scalarAt? facts id do throwError "a row is not scalar at the frame"
    return true
  catch _ =>
    state.restore
    return false

/-- The literal closure a function value is: `closureOf …`, or the value of
the typed closure built of it. -/
private def literalClosure? (e : Lean.Expr) : Option Lean.Expr :=
  let e := e.consumeMData
  if e.isAppOfArity ``closureOf 7 then some e
  else if e.isAppOfArity ``Subtype.val 3 && (e.getArg! 2).isAppOfArity ``typedClosure 7 &&
      ((e.getArg! 2).getArg! 5).isAppOfArity ``closureOf 7 then some ((e.getArg! 2).getArg! 5)
  else none

/-- An invocation of a function value the proof does not see: the rule with
the theorem's typing assumptions (`wp_closureMeaning_unseen_at`). Its
runs and aborts remain, stated by the behavioral predicates. -/
private def unseenInvocation? (goal : MVarId) : TacticM (Option (List MVarId)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
    let action := (target.getArg! 3).consumeMData
    unless action.isAppOfArity ``closureMeaning 7 do return none
    if (literalClosure? (action.getArg! 3)).isSome then return none
    let original ← saveState
    let result ← do
      -- A closure read from a resource gets its field frame from that value's
      -- stored invariant. Recover it before recording the invocation's final
      -- memory, so the call can use the frame just as a parameter call does.
      let goal ← do
        let saved ← saveState
        try
          setGoals [goal]
          evalTactic (← `(tactic| leaner_denote_split_hypotheses))
          evalTactic (← `(tactic| leaner_denote_stored_reads))
          evalTactic (← `(tactic| simp_all only [Which.project?_eq_some_iff,
            Which.inject_here, Which.inject_there, lir_denote_norm]))
          -- Tuple/enum matches expose another payload equation after the
          -- first normalization. Substitute it before matching the frame to
          -- the closure being invoked.
          for _ in [0:2] do
            evalTactic (← `(tactic| leaner_denote_split_hypotheses))
            evalTactic (← `(tactic| leaner_denote_subst_vars))
            evalTactic (← `(tactic| leaner_denote_reduce_projections))
            evalTactic (← `(tactic| leaner_denote_normalize_hypotheses))
          getMainGoal
        catch error =>
          if leaner.denoteDebug.get (← getOptions) then
            IO.println s!"stored closure frame: {← error.toMessageData.toString}"
          saved.restore
          pure goal
      goal.withContext do
        let target ← instantiateMVars (← goal.getType)
        unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
        let action := (target.getArg! 3).consumeMData
        unless action.isAppOfArity ``closureMeaning 7 do return none
        let mut facts := #[]
        for declaration in ← getLCtx do
          if declaration.isImplementationDetail then continue
          if ← isProp declaration.type then
            facts := facts ++ (← conjuncts declaration.toExpr)
        let state ← saveState
        let rule ← mkConstWithFreshMVarLevels ``LeanerIR.Proofs.wp_closureMeaning_unseen_at
        let (arguments, _, conclusion) ← forallMetaTelescope (← inferType rule)
        unless ← isDefEq conclusion target do
          state.restore
          return none
        -- The premises before the runs and the aborts: the row facts by
        -- evaluation, the rest from the hypotheses.
        for argument in arguments.extract 0 (arguments.size - 2) do
          let .mvar id ← instantiateMVars argument | continue
          if ← id.isAssigned then continue
          let type ← instantiateMVars (← id.getType)
          unless ← isProp type do continue
          let mut found := false
          if type.isAppOfArity ``Eq 3 then
            found ← isDefEq argument (← mkEqRefl (type.getArg! 1))
          else if type.isAppOfArity ``LeanerIR.Proofs.MemoryTyped 2 then
            found ← proveMemoryTyped facts id
          else if type.isAppOfArity ``LeanerIR.Proofs.ClosureTypedAt 5 then
            found ← carrierClosureTyped? facts id
          else
            for fact in facts do
              if ← isDefEq (← inferType fact) type then
                id.assign fact
                found := true
                break
          unless found do
            state.restore
            return none
        goal.assign (mkAppN rule arguments)
        let some (.mvar returns) := arguments[arguments.size - 2]? | return none
        let some (.mvar fails) := arguments[arguments.size - 1]? | return none
        let (_, returns) ← returns.introN 6
          [`leanerResult, `leanerFinal, `leanerEnsures, `leanerUnaborted, `leanerResultOf,
            `leanerStateOf]
        let returns ← keptMemory returns action facts
        return some [returns, fails]
    -- Preparation can assign the original goal. A failed match must undo
    -- that preparation as well as the attempted invocation rule.
    if result.isNone then original.restore
    return result

/-- A literal closure of a non-generic target at rows its frame resolves to
scalars: the check, evaluated by the kernel over the goal's unit
(`ClosureTypedAt.ofClosureOf`). -/
private def literalClosureTyped? (goal : MVarId) : TacticM Bool := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``LeanerIR.Proofs.ClosureTypedAt 5 do return false
  unless (literalClosure? (target.getArg! 4)).isSome do return false
  let state ← saveState
  try
    let check ← goal.withContext do
      let rule ← mkConstWithFreshMVarLevels ``LeanerIR.Proofs.ClosureTypedAt.ofClosureOf
      let (arguments, _, conclusion) ← forallMetaTelescope (← inferType rule)
      -- The frame the typing is stated at: the goal's.
      arguments[1]!.mvarId!.assign (target.getArg! 1)
      unless ← isDefEq conclusion target do throwError "not a literal closure"
      let premises := arguments.extract (arguments.size - 5) arguments.size
      -- The rows as the frames resolve them: a frame evaluates where it is
      -- the runtime frame or one a call's type arguments induce, and
      -- resolves a type without parameters to itself.
      for premise in premises.extract 0 4 do
        let .mvar id := premise | throwError "a row premise is not open"
        setGoals [id]
        evalTactic (← `(tactic| first
          | rfl
          | (simp only [LeanerIR.Proofs.Denote.NRow.resolved,
              LeanerIR.Proofs.Denote.Skolems.resolve_instantiate,
              LeanerIR.Proofs.Denote.Skolems.resolve_runtime,
              LeanerIR.Proofs.Denote.Skolems.resolve_paramFree, LeanerIR.Proofs.Denote.NTy.paramFree,
              LeanerIR.Proofs.Denote.NRow.paramFree, LeanerIR.Proofs.Denote.NRows.paramFree,
              LeanerIR.Proofs.Denote.NTy.subst, LeanerIR.Proofs.Denote.NRow.subst,
              LeanerIR.Proofs.Denote.NRows.subst, LeanerIR.Proofs.Denote.NRow.getD,
              Bool.and_true, Bool.true_and]
             rfl)))
        unless (← getGoals).isEmpty do throwError "a row is not scalar"
      goal.assign (mkAppN rule arguments)
      let some (Lean.Expr.mvar check) := premises[4]? | throwError "no check"
      pure check
    setGoals [check]
    evalTactic (← `(tactic| decide +kernel))
    return true
  catch _ =>
    state.restore
    return false

/-- A literal closure keeps a frame (`FramedAt`, `KeepsMemoryAt`) where its
target's theorem applies at every typed memory and its frame lies within it
(`FramedAt.ofVerified`), or else where its target's body keeps it, an
obligation over every argument and typed memory (`FramedAt.ofBody`); any
function value does at a row with references, where the frame is vacuous.
The goals left, if the closure's frame is decided. -/
private def literalClosureKeepsMemory? (goal : MVarId)
    (callees : Array (Lean.Expr × String × Lean.Expr)) : TacticM (Option (List MVarId)) := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let target := if target.isAppOfArity ``LeanerIR.Proofs.Obligation 4 then target.appArg! else target
  let target ← if target.isAppOfArity ``LeanerIR.Proofs.KeepsMemoryAt 5 then
      goal.withContext (whnfR target) else pure target
  unless target.isAppOfArity ``LeanerIR.Proofs.FramedAt 6 do return none
  if leaner.denoteDebug.get (← getOptions) then logInfo "frame: inspecting closure"
  let references ← goal.withContext do
    let refFree := mkApp (mkConst ``NRow.refFree) (target.getArg! 3)
    if ← isDefEq refFree (mkConst ``Bool.false) then
      some <$> mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.false))
        (← mkEq refFree (mkConst ``Bool.false))
    else pure none
  if let some references := references then
    goal.assign (← goal.withContext do
      mkAppOptM ``LeanerIR.Proofs.FramedAt.of_refFree_false
        #[target.getArg! 0, target.getArg! 1, target.getArg! 2, target.getArg! 3,
          target.getArg! 4, target.getArg! 5, references])
    return some []
  -- A frame a hypothesis states of the same function value, within it.
  if leaner.denoteDebug.get (← getOptions) then logInfo "frame: checking stated frames"
  let stated ← try goal.withContext do
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some (_, closure) ← framedFact? decl.type | continue
      unless ← isDefEq closure (target.getArg! 5) do continue
      let narrow ← whnfR (← frameFactType decl.type)
      unless narrow.isAppOfArity ``LeanerIR.Proofs.FramedAt 6 do continue
      let .forallE _ argumentsType _ _ ← whnfR (← inferType (target.getArg! 4)) | continue
      let memoryType := mkApp (mkConst ``LeanerIR.Proofs.Denote.Memory) (target.getArg! 0)
      let withinType ← withLocalDeclD `leanerArguments argumentsType fun args =>
        withLocalDeclD `pre memoryType fun pre =>
          withLocalDeclD `post memoryType fun post => do
            mkForallFVars #[args, pre, post] (← mkArrow
              (mkApp3 (narrow.getArg! 4) args pre post).headBeta
              (mkApp3 (target.getArg! 4) args pre post).headBeta)
      let within ← mkFreshExprSyntheticOpaqueMVar withinType
      unless ← closesBy within.mvarId! (← `(tactic| (
          intro _ _ _ $(mkIdent `leanerFrame):ident
          first
          | exact $(mkIdent `leanerFrame)
          | leaner_denote_bounded
              (simp only [lir_denote_norm, lir_denote_eval] at $(mkIdent `leanerFrame):ident ⊢
               grind)))) do continue
      return some (← mkAppAtFrame ``LeanerIR.Proofs.FramedAt.mono
          (target.getArg! 0) (target.getArg! 1) (target.getArg! 2)
        #[← mkExpectedTypeHint (← frameFactProof decl.toExpr) narrow, ← instantiateMVars within])
    return none
    catch _ => pure none
  if let some proof := stated then
    if leaner.denoteDebug.get (← getOptions) then logInfo "frame: applying stated frame"
    goal.assign proof
    return some []
  if leaner.denoteDebug.get (← getOptions) then logInfo "frame: checking literal target"
  let some closure := literalClosure? (target.getArg! 5) | return none
  let mask := closure.getArg! 3
  unless mask.isAppOfArity ``Weave.mask 4 do return none
  goal.withContext do
  let weave := mask.appArg!
  let rows := (← whnfR (← inferType weave)).getAppArgs
  let some captured := rows[1]? | return none
  let some supplied := rows[2]? | return none
  let handle := closure.getArg! 2
  let free := fun (statement : Lean.Expr) => do
    mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true)) (← mkEq statement (mkConst ``Bool.true))
  let coherent ← mkAppM ``coherent_runtime
    #[target.getArg! 0, handle]
  let state ← saveState
  try
    let some (_, _, theoremProof) ← callees.findM? fun (candidate, _, proof) => do
        unless ← isDefEq candidate handle do return false
        forallTelescope (← inferType proof) fun _ statement =>
          pure (statement.isAppOfArity ``LeanerIR.Proofs.Satisfies 6)
      | throwError "the target has no theorem"
    let (theoremArguments, _, statement) ← forallMetaTelescope (← inferType theoremProof)
    let meaning := statement.getArg! 4
    unless meaning.isAppOf ``propheticMeaning do throwError "not a function's meaning"
    unless ← isDefEq (meaning.getArg! 2) (target.getArg! 2) do
      throwError "the closure and its target theorem use different type families"
    let rule ← mkAppAtFrame ``LeanerIR.Proofs.FramedAt.ofVerified
        (target.getArg! 0) (target.getArg! 1) (target.getArg! 2)
      #[weave, ← free (mkApp (mkConst ``NRow.refFree) captured),
        ← free (mkApp (mkConst ``NRow.refFree) supplied),
        ← free (mkApp (mkConst ``NRow.refFree) (← mkAppM ``ResultShape.row #[← whnfD meaning.appArg!])),
        closure.getArg! 4, coherent, closure.appArg!, ← instantiateMVars (mkAppN theoremProof theoremArguments)]
    let rule := mkApp rule (target.getArg! 4)
    -- The natives the target's theorem assumes, the caller assumes too.
    for argument in theoremArguments do
      if let .mvar id ← instantiateMVars argument then
        unless ← id.isAssigned do id.assumption
    let constants ← contractConstants (← inferType theoremProof)
    let lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ← constants.mapM fun name =>
      `(Lean.Parser.Tactic.simpLemma| $(mkIdent (rootNamespace ++ name)):ident)
    let .forallE _ heldType rest _ ← whnfR (← inferType rule) | throwError "no premises"
    let .forallE _ framedType _ _ ← whnfR rest | throwError "no frame premise"
    -- What the target's theorem assumes, then its precondition, at every
    -- argument and typed memory.
    let held ← mkFreshExprSyntheticOpaqueMVar heldType
    unless ← closesBy held.mvarId! (← `(tactic| (
        intro _ _ $(mkIdent `leanerTyped):ident
        (try simp only [$lemmas,*, Weave.compose, lir_denote_norm])
        all_goals (try constructor)
        all_goals leaner_denote_leaf))) do
      throwError "the target's theorem does not apply everywhere"
    -- The target's frame lies within the closure's.
    let framed ← mkFreshExprSyntheticOpaqueMVar framedType
    unless ← closesBy framed.mvarId! (← `(tactic| (
        intro _ _ _ $(mkIdent `leanerFrame):ident
        (try simp only [$lemmas,*, Weave.compose, lir_denote_norm]
          at $(mkIdent `leanerFrame):ident ⊢)
        first
        | exact $(mkIdent `leanerFrame)
        | leaner_denote_leaf))) do
      throwError "the target's frame exceeds the closure's"
    let proof ← instantiateMVars (mkApp2 rule held framed)
    unless ← isDefEq (← inferType proof) target do throwError "not the closure"
    goal.assign proof
    return some []
  catch error =>
    if leaner.denoteDebug.get (← getOptions) then
      IO.println s!"closure frame: {← error.toMessageData.toString}"
    state.restore
  -- The target's body, where the proof inlines it.
  let mut compiled? := none
  for (candidate, _, proof) in callees do
    unless ← isDefEq candidate handle do continue
    let statement ← forallTelescope (← inferType proof) fun _ statement => pure statement
    if statement.isAppOfArity ``Eq 3 && (statement.getArg! 2).isAppOfArity ``Except.ok 3 then
      compiled? := some (statement.getArg! 2).appArg!
  let some compiled := compiled? | return none
  try
    let shape ← whnfD (← mkAppM ``Function.result #[compiled])
    let rule ← mkAppAtFrame ``LeanerIR.Proofs.FramedAt.ofBody (target.getArg! 0) (target.getArg! 1) (target.getArg! 2)
      #[weave, ← free (mkApp (mkConst ``NRow.refFree) captured),
        ← free (mkApp (mkConst ``NRow.refFree) supplied),
        ← free (mkApp (mkConst ``NRow.refFree) (← mkAppM ``ResultShape.row #[shape])),
        closure.getArg! 4, coherent, closure.appArg!]
    let rule := mkApp rule (target.getArg! 4)
    let .forallE _ keptType _ _ ← whnfR (← inferType rule) | throwError "no body premise"
    let kept ← mkFreshExprSyntheticOpaqueMVar keptType
    let proof ← instantiateMVars (mkApp rule kept)
    unless ← isDefEq (← inferType proof) target do throwError "not the closure"
    goal.assign proof
    let (_, kept) ← kept.mvarId!.introN 3 [`leanerArguments, `leanerStart, `leanerTyped]
    return some [kept]
  catch _ =>
    state.restore
    return none

/-- A negated disjunction's negated disjuncts, as proofs, or the negation
itself. -/
private partial def negatedDisjuncts (proof : Lean.Expr) : MetaM (Array Lean.Expr) := do
  let type ← instantiateMVars (← inferType proof)
  let some negated := type.not? | return #[]
  unless negated.isAppOfArity ``Or 2 do return #[proof]
  let parts ← mkAppM ``Iff.mp
    #[← mkAppOptM ``not_or #[negated.getArg! 0, negated.getArg! 1], proof]
  return (← negatedDisjuncts (← mkAppM ``And.left #[parts])) ++
    (← negatedDisjuncts (← mkAppM ``And.right #[parts]))

/-- The propositions a goal's context states, conjunctions split. -/
private def contextFacts (goal : MVarId) : MetaM (Array Lean.Expr) := goal.withContext do
  let mut facts := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    if ← isProp decl.type then facts := facts ++ (← conjuncts decl.toExpr)
  return facts

/-- The run of one literal closure's invocation a hypothesis `hypothesis`
states not to abort (`aborts` its `aborts_of`), where runs end: asserted at
the leaf's own spelling, normalized, and its `result_of` rewritten in the
other hypotheses. -/
private def terminatingRun (goal : MVarId) (facts : Array Lean.Expr)
    (hypothesis aborts : Lean.Expr) : TacticM MVarId := do
  let proof ← goal.withContext do
    -- The invocation's rows, shape, and native arguments, as the
    -- predicate's literal arguments spell them.
    let some function := (aborts.getArg! 2).find? (·.isAppOfArity ``NTy.function 3)
      | throwError "the callable has no function type"
    let parameterRow := function.getArg! 0
    let resultRow := function.getArg! 2
    let shape ← if resultRow.isConstOf ``NRow.nil then pure (mkConst ``ResultShape.none)
      else if resultRow.isAppOfArity ``NRow.cons 2 && (resultRow.getArg! 1).isConstOf ``NRow.nil
      then mkAppM ``ResultShape.one #[resultRow.getArg! 0]
      else throwError "the results are not one shape"
    let some elements := literalElements? (aborts.getArg! 3)
      | throwError "the arguments are not literal"
    -- An argument another invocation's single result gave is that result:
    -- `(packResults #[integer e]).asInt` is `e`, by definition.
    let elements := elements.map fun element => Id.run do
      unless element.isAppOfArity ``RuntimeValue.integer 1 do return element
      let read := element.appArg!
      unless read.isAppOfArity ``RuntimeValue.asInt 1 &&
          read.appArg!.isAppOfArity ``SemanticOperations.packResults 1 do return element
      let some (_, [single]) := read.appArg!.appArg!.arrayLit? | return element
      return if single.isAppOfArity ``RuntimeValue.integer 1 then single else element
    let some args ← nativeRow? parameterRow elements
      | throwError m!"the arguments {elements} are not native at {parameterRow}"
    let rule ← mkConstWithFreshMVarLevels ``LeanerIR.Proofs.returns_of_terminating
    let ruleType ← inferType rule
    let (arguments, _, _) ← forallMetaTelescope ruleType
    let names := ruleType.getForallBinderNames
    for (name, value) in [(`σs, parameterRow), (`shape, shape), (`args, args)] do
      let some index := names.idxOf? name | throwError "the rule has no {name}"
      unless ← isDefEq arguments[index]! value do throwError "{name} does not fit"
    let some premise := arguments.back? | throwError "the rule has no premise"
    let premiseType ← instantiateMVars (← inferType premise)
    unless ← isDefEq premiseType (← inferType hypothesis) do
      throwError "another invocation"
    premise.mvarId!.assign hypothesis
    for argument in arguments.pop do
      let .mvar id ← instantiateMVars argument | continue
      if ← id.isAssigned then continue
      let type ← instantiateMVars (← id.getType)
      unless ← isProp type do continue
      if type.isAppOfArity ``Eq 3 then
        unless ← isDefEq argument (← mkEqRefl (type.getArg! 1)) do
          throwError "a row is not scalar"
      else if type.isAppOfArity ``LeanerIR.Proofs.MemoryTyped 2 then
        unless ← proveMemoryTyped facts id do throwError "memory is not typed"
      else if type.isAppOfArity ``LeanerIR.Proofs.ClosureTypedAt 5 then
        unless ← (literalClosureTyped? id) <||> (carrierClosureTyped? facts id) do
          throwError "the closure is not typed"
      else
        let some fact ← facts.findM? fun fact => do isDefEq (← inferType fact) type
          | throwError "a premise is not held"
        id.assign fact
    let proof ← instantiateMVars (mkAppN rule arguments)
    if proof.hasExprMVar then throwError "the run leaves premises open"
    -- The run is stated at the leaf's own spelling of the invocation, read
    -- once the premises fix every argument of the rule.
    let some stated := (← instantiateMVars premiseType).not? | throwError "no invocation"
    let spelled := fun (e : Lean.Expr) =>
      if e == stated.getArg! 2 then some (aborts.getArg! 2)
      else if e == stated.getArg! 3 then some (aborts.getArg! 3) else none
    mkExpectedTypeHint proof ((← instantiateMVars (← inferType proof)).replace spelled)
  let (_, next) ← goal.withContext do
    (← goal.assert `leanerReturns (← inferType proof) proof).intro1P
  setGoals [next]
  evalTactic (← `(tactic| obtain ⟨_, _, $(mkIdent `leanerEnsures), $(mkIdent `leanerResult),
    $(mkIdent `leanerStateOf)⟩ :=
    $(mkIdent `leanerReturns)))
  -- The results as the leaf spells them: a literal array.
  evalTactic (← `(tactic| try leaner_denote_normalize at $(mkIdent `leanerEnsures):ident))
  evalTactic (← `(tactic| try leaner_denote_normalize at $(mkIdent `leanerResult):ident))
  let [obtained] ← getGoals | throwError "the run leaves one goal"
  obtained.withContext do
    let some result := (← getLCtx).findFromUserName? `leanerResult
      | throwError "no result equation"
    let mut current := obtained
    for decl in ← getLCtx do
      if decl.isImplementationDetail || decl.fvarId == result.fvarId then continue
      let type ← instantiateMVars decl.type
      unless (type.find? (·.isAppOf ``LeanerIR.Proofs.ResultOf)).isSome do continue
      try
        current ← current.withContext do
          let rewritten ← current.rewrite type result.toExpr
          pure (← current.replaceLocalDecl decl.fvarId rewritten.eNew rewritten.eqProof).mvarId
      catch _ => pure ()
    return current

/-- The `result_of` reads of a term at arguments a leaf names: those of
function values a closure literal builds, outside binders. -/
private def resultOfReads (terms : Array Lean.Expr) : Array Lean.Expr :=
  sitesWhere (fun e =>
    (e.isAppOfArity ``LeanerIR.Proofs.ResultOf 5 ||
      e.isAppOfArity ``LeanerIR.Proofs.StateOf 5) && !e.hasLooseBVars &&
      ((e.getArg! 2).find? (·.isAppOf ``closureOf)).isSome &&
      (literalElements? (e.getArg! 3)).isSome) terms

/-- A leaf that reads `result_of` of a literal closure where it knows
neither that the invocation aborts nor that it does not, case by case on
the abort: where it aborts, the target's body or theorem decides the leaf;
where it does not, the run `result_of` reads (`terminatingRuns`), as the
Move Prover reads `result_of` under `!aborts_of`. -/
private def abortCases? (goal : MVarId) : TacticM (Option (List MVarId)) := do
  let some aborts ← goal.withContext do
      let facts ← contextFacts goal
      unless ← facts.anyM (fun fact => do
          return (← instantiateMVars (← inferType fact)).isAppOfArity
            ``LeanerIR.Proofs.Terminating 2) do
        return none
      let target ← instantiateMVars (← goal.getType)
      let mut statements := #[target]
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let type ← instantiateMVars decl.type
        statements := statements.push type
      for read in resultOfReads statements do
        let aborts := mkAppN (mkConst ``LeanerIR.Proofs.AbortsOf) read.getAppArgs
        if statements.any (fun statement => (statement.find? (· == aborts)).isSome) then continue
        -- Arguments a run is read at: natives of the function value's row.
        let some function := (read.getArg! 2).find? (·.isAppOfArity ``NTy.function 3) | continue
        let some elements := literalElements? (read.getArg! 3) | continue
        let state ← saveState
        let native ← try pure (← nativeRow? (function.getArg! 0) elements).isSome
          catch _ => pure false
        state.restore
        if native then return some aborts
      return none
    | return none
  let (aborted, unaborted) ← goal.byCases aborts `leanerAborted
  return some [aborted.mvarId, unaborted.mvarId]

/-- Where the theorem holds that the unit's runs end (`Terminating`), each
literal closure's invocation the leaf knows not to abort returns: a run
`ensures_of` names, and the result `result_of` reads, which replaces it in
the hypotheses (`returns_of_terminating`). One invocation a round, so that
an invocation at another's result is met once that result is read. -/
private def terminatingRuns (goal : MVarId) : TacticM MVarId := do
  let saved ← getGoals
  let terminating ← goal.withContext do (← contextFacts goal).anyM fun fact => do
    return (← instantiateMVars (← inferType fact)).isAppOfArity ``LeanerIR.Proofs.Terminating 2
  unless terminating do return goal
  let mut current := goal
  let mut handled : Array Lean.Expr := #[]
  let debug := leaner.denoteDebug.get (← getOptions)
  for _ in [0:16] do
    let facts ← contextFacts current
    let candidates ← current.withContext do
      -- The results the runs found so far, read where an invocation's
      -- arguments name them.
      let mut results := #[]
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let type ← instantiateMVars decl.type
        if type.isAppOfArity ``Eq 3 && (type.getArg! 1).isAppOf ``LeanerIR.Proofs.ResultOf then
          results := results.push decl.toExpr
      let mut found : Array (Lean.Expr × Lean.Expr) := #[]
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        for negation in ← negatedDisjuncts decl.toExpr do
          let mut negation := negation
          for result in results do
            try
              let rewritten ← current.rewrite (← instantiateMVars (← inferType negation)) result
              negation ← mkEqMP rewritten.eqProof negation
            catch _ => pure ()
          let some aborts := (← instantiateMVars (← inferType negation)).not? | continue
          unless aborts.isAppOfArity ``LeanerIR.Proofs.AbortsOf 5 &&
              ((aborts.getArg! 2).find? (·.isAppOf ``closureOf)).isSome do continue
          unless handled.contains aborts || found.any (·.2 == aborts) do
            found := found.push (negation, aborts)
      pure found
    let mut progress := false
    for (hypothesis, aborts) in candidates do
      let state ← saveState
      try
        current ← terminatingRun current facts hypothesis aborts
        handled := handled.push aborts
        progress := true
        break
      catch ex =>
        if debug then IO.println s!"terminating run skipped: {← ex.toMessageData.toString}"
        state.restore
    unless progress do break
  if debug && !handled.isEmpty then
    logInfo m!"after terminating runs:{indentD (MessageData.ofGoal current)}"
  setGoals saved
  return current

/-- The run or abort of a literal closure a hypothesis states, of a target
the proof inlines — compiled, for a run without a theorem, as a function
without a specification is: the target's prophetic meaning runs or aborts
(`ensuresOf_closureOf`, `abortsOf_closureOf`). The leaf becomes the
weakest precondition of that meaning under which it holds of each run or
abort (`forall_ok_of_wp`, `of_aborts_of_wp`), which the call rule inlines;
the hypothesis stays, marked read (`Denoted`). -/
private def denotedRun (goal : MVarId) (callees : Array (Lean.Expr × String × Lean.Expr))
    (hypothesis : FVarId) (stated type : Lean.Expr) : TacticM (Option MVarId) := do
  let ensures := type.isAppOfArity ``LeanerIR.Proofs.EnsuresOf 7
  let some closure := (type.getArg! 2).find? (·.isAppOfArity ``closureOf 7) | return none
  let mask := closure.getArg! 3
  unless mask.isAppOfArity ``Weave.mask 4 do throwError "no weave"
  let weave := mask.appArg!
  let rows := (← whnfR (← inferType weave)).getAppArgs
  let some captured := rows[1]? | return none
  let some supplied := rows[2]? | return none
  let handle := closure.getArg! 2
  -- The runs of a target with a theorem are read by its contract
  -- (`dispatchBehavior`); its aborts by its body, which states them where
  -- the contract need not, as the Move Prover derives them.
  let mut compiled? := none
  for (candidate, _, proof) in callees do
    unless ← isDefEq candidate handle do continue
    let statement ← forallTelescope (← inferType proof) fun _ statement => pure statement
    if ensures && statement.isAppOfArity ``LeanerIR.Proofs.Satisfies 6 then
      throwError "a theorem"
    if statement.isAppOfArity ``Eq 3 && (statement.getArg! 2).isAppOfArity ``Except.ok 3 then
      compiled? := some (statement.getArg! 2).appArg!
  let some compiled := compiled? | throwError "not compiled"
  let shape ← whnfD (← mkAppM ``Function.result #[compiled])
  let some arguments := literalElements? (type.getArg! 3) | throwError "arguments not literal"
  let some natives ← nativeRow? supplied arguments | throwError "arguments not native"
  let free := fun (statement : Lean.Expr) => do
    mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true)) (← mkEq statement (mkConst ``Bool.true))
  let coherent ← mkAppM ``coherent_runtime
    #[type.getArg! 0, handle]
  let capturedFree ← free (mkApp (mkConst ``NRow.refFree) captured)
  let suppliedFree ← free (mkApp (mkConst ``NRow.refFree) supplied)
  if ensures && shape.isConstOf ``ResultShape.none then
    -- A run that returns nothing.
    let some [] := literalElements? (type.getArg! 4) | throwError m!"results not literal {type.getArg! 4}"
    let .fvar finalVar := type.getArg! 6 | throwError "final not a variable"
    let run ← mkAppAtFrame ``LeanerIR.Proofs.ensuresOf_closureOf_shape
        (type.getArg! 0) (type.getArg! 1) (closure.getArg! 1)
      #[weave, capturedFree, suppliedFree,
        ← free (mkApp (mkConst ``NRow.refFree) (← mkAppM ``ResultShape.row #[shape])),
        closure.getArg! 4, coherent, closure.appArg!, natives, mkConst ``Unit.unit, stated]
    let runType ← inferType run
    let goal ← goal.replaceLocalDeclDefEq hypothesis
      (mkApp (mkConst ``LeanerIR.Proofs.Denoted) (← instantiateMVars (← hypothesis.getType)))
    let (runVar, goal) ← goal.withContext do (← goal.assert `leanerRun runType run).intro1P
    let dependents ← goal.withContext do
      (← getLCtx).foldlM (init := #[]) fun found decl => do
        if decl.isImplementationDetail || decl.fvarId == runVar || decl.fvarId == finalVar then
          return found
        if (← instantiateMVars decl.type).containsFVar finalVar then return found.push decl.fvarId
        return found
    let (_, goal) ← goal.revert dependents (preserveOrder := true)
    let (_, goal) ← goal.revert #[runVar]
    let (_, goal) ← goal.revert #[finalVar]
    let [next] ← goal.apply (← mkConstWithFreshMVarLevels ``LeanerIR.Proofs.forall_ok_unit_of_wp)
      | return none
    return some next
  else if ensures then
    unless shape.isAppOfArity ``ResultShape.one 1 do throwError m!"not one result {shape}"
    let τ := shape.appArg!
    let some [result] := literalElements? (type.getArg! 4) | throwError m!"results not literal {type.getArg! 4}"
    let some native ← nativeOf? τ result | throwError "result not native"
    let .fvar resultVar := native | throwError m!"result not a variable {native}"
    let .fvar finalVar := type.getArg! 6 | throwError "final not a variable"
    let run ← mkAppAtFrame ``LeanerIR.Proofs.ensuresOf_closureOf
        (type.getArg! 0) (type.getArg! 1) (closure.getArg! 1)
      #[weave, capturedFree, suppliedFree, ← free (mkApp (mkConst ``NTy.refFree) τ),
        closure.getArg! 4, coherent, closure.appArg!, natives, native, stated]
    let runType ← inferType run
    let goal ← goal.replaceLocalDeclDefEq hypothesis
      (mkApp (mkConst ``LeanerIR.Proofs.Denoted) (← instantiateMVars (← hypothesis.getType)))
    let (runVar, goal) ← goal.withContext do (← goal.assert `leanerRun runType run).intro1P
    -- What depends on the run's result and final state goes into the goal,
    -- the run first, so that the leaf is a fact about every run.
    let dependents ← goal.withContext do
      (← getLCtx).foldlM (init := #[]) fun found decl => do
        if decl.isImplementationDetail || decl.fvarId == runVar || decl.fvarId == resultVar ||
            decl.fvarId == finalVar then return found
        let declType ← instantiateMVars decl.type
        if declType.containsFVar resultVar || declType.containsFVar finalVar then
          return found.push decl.fvarId
        return found
    let (_, goal) ← goal.revert dependents (preserveOrder := true)
    let (_, goal) ← goal.revert #[runVar]
    let (_, goal) ← goal.revert #[resultVar, finalVar] (preserveOrder := true)
    let [next] ← goal.apply (← mkConstWithFreshMVarLevels ``LeanerIR.Proofs.forall_ok_of_wp)
      | return none
    return some next
  else
    let run ← mkAppAtFrame ``LeanerIR.Proofs.abortsOf_closureOf (type.getArg! 0) (type.getArg! 1)
        (closure.getArg! 1)
      #[weave, capturedFree, suppliedFree, shape, closure.getArg! 4, coherent, closure.appArg!,
        natives, stated]
    let goal ← goal.replaceLocalDeclDefEq hypothesis
      (mkApp (mkConst ``LeanerIR.Proofs.Denoted) (← instantiateMVars (← hypothesis.getType)))
    goal.withContext do
      -- The abort's action and start, read off `∃ failure, action.aborts start failure`.
      let runType ← instantiateMVars (← inferType run)
      let .lam _ _ aborts _ := runType.appArg! | throwError "the abort is not an existential"
      unless aborts.isAppOfArity ``LeanerIR.Proofs.Spec.aborts 6 do
        throwError "the abort is not of an action"
      let (σ, ε, α) := (aborts.getArg! 0, aborts.getArg! 1, aborts.getArg! 2)
      let (action, initial) := (aborts.getArg! 3, aborts.getArg! 4)
      let leaf ← goal.getType
      let returns ← withLocalDeclD `result α fun result => withLocalDeclD `final σ fun final =>
        mkLambdaFVars #[result, final] (mkConst ``True)
      let fails ← withLocalDeclD `error ε fun error => mkLambdaFVars #[error] leaf
      let established ← mkFreshExprSyntheticOpaqueMVar
        (mkAppN (mkConst ``LeanerIR.Proofs.wp) #[σ, ε, α, action, returns, fails, initial])
      goal.assign (mkAppN (mkConst ``LeanerIR.Proofs.of_aborts_of_wp)
        #[σ, ε, α, action, initial, leaf, established, run])
      return some established.mvarId!

/-- A leaf's first run or abort of a literal closure of an inlined target
(`denotedRun`), as a hypothesis states it under premises the leaf holds
and its clause's marker. -/
private def denotedRun? (goal : MVarId) (callees : Array (Lean.Expr × String × Lean.Expr)) :
    TacticM (Option MVarId) := do
  let candidates ← goal.withContext do
    let mut found := #[]
    for hypothesis in ← goal.getNondepPropHyps do
      let mut type ← instantiateMVars (← hypothesis.getType)
      if type.isAppOfArity ``LeanerIR.Proofs.Denoted 1 then continue
      unless (type.find? fun e => e.isAppOf ``LeanerIR.Proofs.EnsuresOf ||
          e.isAppOf ``LeanerIR.Proofs.AbortsOf).isSome do continue
      let mut stated := Lean.Expr.fvar hypothesis
      let mut held := true
      while type.isArrow do
        let some premise ← findLocalDeclWithType? type.bindingDomain! | held := false; break
        stated := mkApp stated (.fvar premise)
        type := type.bindingBody!
      unless held do continue
      if type.isAppOfArity ``LeanerIR.Proofs.Obligation 4 then
        stated := mkApp4 (mkConst ``Iff.mp) type type.appArg!
          (mkAppN (mkConst ``LeanerIR.Proofs.Obligation_iff) type.getAppArgs) stated
        type := type.appArg!
      unless type.isAppOfArity ``LeanerIR.Proofs.EnsuresOf 7 ||
          type.isAppOfArity ``LeanerIR.Proofs.AbortsOf 5 do continue
      if ((type.getArg! 2).find? (·.isAppOfArity ``closureOf 7)).isSome then
        found := found.push (hypothesis, stated, type)
    pure found
  for (hypothesis, stated, type) in candidates do
    let state ← saveState
    try
      if let some next ← goal.withContext (denotedRun goal callees hypothesis stated type) then
        return some next
      state.restore
    catch ex =>
      if leaner.denoteDebug.get (← getOptions) then
        IO.println s!"denoted run skipped: {← ex.toMessageData.toString}"
      state.restore
  return none

/-- Rewrite a goal's reads of `result_of` by what its hypotheses state the
invocation returned. -/
private def rewriteResultOf (goal : MVarId) : MetaM MVarId := goal.withContext do
  let mut goal := goal
  for declaration in ← getLCtx do
    let type ← instantiateMVars declaration.type
    unless type.isAppOfArity ``Eq 3 &&
        ((type.getArg! 1).isAppOfArity ``LeanerIR.Proofs.ResultOf 5 ||
          (type.getArg! 1).isAppOfArity ``LeanerIR.Proofs.StateOf 5) do
      continue
    try
      let rewritten ← goal.rewrite (← goal.getType) declaration.toExpr
      goal ← goal.replaceTargetEq rewritten.eNew rewritten.eqProof
    catch _ => pure ()
    if (type.getArg! 1).isAppOfArity ``LeanerIR.Proofs.StateOf 5 then
      for other in ← getLCtx do
        if other.isImplementationDetail || other.fvarId == declaration.fvarId then continue
        unless (other.type.find? (·.isAppOf ``LeanerIR.Proofs.StateOf)).isSome do continue
        try
          goal ← goal.withContext do
            let rewritten ← goal.rewrite other.type declaration.toExpr
            pure (← goal.replaceLocalDecl other.fvarId rewritten.eNew rewritten.eqProof).mvarId
        catch _ => pure ()
  return goal

/-- A call of a target taking and returning no reference, where the goal
states a behavioral predicate: its continuations assume `ensures_of` and
`aborts_of` of the function value naming the target (`wp_named`) and, where
the natives keep loans apart, the absence of its aborts and the result
`result_of` reads (`wp_named_typed`). Other calls stay as they are. -/
private def namedCall (goal : MVarId) : TacticM MVarId := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  unless (target.find? fun e =>
      e.isConstOf ``LeanerIR.Proofs.EnsuresOf || e.isConstOf ``LeanerIR.Proofs.AbortsOf ||
        e.isConstOf ``LeanerIR.Proofs.ResultOf || e.isConstOf ``LeanerIR.Proofs.StateOf).isSome do
    return goal
  -- The rules at the call's executable and frame, which no instance search
  -- finds.
  unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return goal
  let meaning := target.getArg! 3
  unless meaning.isAppOfArity ``propheticMeaning 8 do return goal
  let atFrame := fun (rule : Name) => goal.withContext do
    Lean.Elab.Term.exprToSyntax (mkApp3 (mkConst rule) (meaning.getArg! 0) (meaning.getArg! 1)
      (meaning.getArg! 2))
  let saved ← saveState
  let shifts? ← goal.withContext do
    (← getLCtx).findDeclM? fun declaration => do
      if declaration.isImplementationDetail then return none
      let type ← instantiateMVars declaration.type
      return if type.isAppOfArity ``LeanerIR.NativesShift 2 then some declaration.toExpr else none
  if let some shifts := shifts? then
    try
      setGoals [goal]
      let shiftsSyntax ← goal.withContext (Lean.Elab.Term.exprToSyntax shifts)
      let rule ← atFrame ``LeanerIR.Proofs.wp_named_typed
      evalTactic (← `(tactic| refine $rule $shiftsSyntax (by decide) (by decide) ?_))
      let [named] ← getGoals | throwError "the typed named call leaves one goal"
      return named
    catch failure =>
      saved.restore
      if leaner.denoteDebug.get (← getOptions) then
        logInfo m!"typed named call not taken: {failure.toMessageData}"
  try
    setGoals [goal]
    let rule ← atFrame ``LeanerIR.Proofs.wp_named
    evalTactic (← `(tactic| refine $rule (by decide) (by decide) ?_))
    let [named] ← getGoals | throwError "the named call leaves one goal"
    return named
  catch _ =>
    saved.restore
    return goal

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
    else if action.isAppOfArity ``LeanerIR.Proofs.Denote.Flow.bind 8 then
      let inner := (action.getArg! 6).consumeMData.headBeta
      if inner.isAppOfArity ``LeanerIR.Proofs.Denote.Flow.bind 8 then
        -- Flows associate to the right, so that the goal's continuation
        -- stays out of the inner bind's arms.
        pure (mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_flowBind_flowBind)
          (action.getAppArgs.extract 0 4 ++
            #[inner.getArg! 4, inner.getArg! 5, action.getArg! 5, inner.getArg! 6,
              inner.getArg! 7, action.getArg! 7] ++ conditions))
      else if inner.isAppOfArity ``LeanerIR.Proofs.Spec.bind 6 then
        -- A flow after a bind binds inside the bind's continuation, where the
        -- goal's continuation appears once.
        pure (mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_flowBind_specBind)
          ((action.getAppArgs.extract 0 6).push (inner.getArg! 2) ++
            #[inner.getArg! 4, inner.getArg! 5, action.getArg! 7] ++ conditions))
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
    (residual : Bool := false) (labeled : Bool := false) :
    TacticM Unit := do
  assertedBounds.set {}
  normalHypotheses.set {}
  stageNormalHypotheses.set {}
  stepProfile.set {}
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
        if type.isAppOfArity ``FunctionStart 5 then
          if ← isDefEq (type.getArg! 2) function then
            found := some (type.getArg! 3, type.getArg! 4)
      return found
  -- A predicate over a function's start, applied there: without a recorded
  -- start, it does not read `old`, and its start arguments vanish on
  -- reduction.
  let atStart (goal : MVarId) (function predicate : Lean.Expr) (site : Nat)
      (rest : Array Lean.Expr) : MetaM Lean.Expr := do
    let (start, startState) ← match ← start? goal function with
      | some start => pure start
      | none => goal.withContext do
          let type ← inferType predicate
          let .forallE _ argumentsType rest _ := type
            | throwError m!"the predicate at site {site} takes no function start"
          let .forallE _ stateType _ _ := rest
            | throwError m!"the predicate at site {site} takes no function start"
          pure (← mkFreshExprMVar argumentsType, ← mkFreshExprMVar stateType)
    let applied ← goal.withContext (whnfR (mkAppN predicate (#[start, startState] ++ rest)))
    if (← instantiateMVars applied).hasExprMVar then
      throwError m!"the predicate at site {site} reads `old` without a recorded function start"
    return applied
  let debug := leaner.denoteDebug.get (← getOptions)
  let startHeartbeats ← IO.getNumHeartbeats
  -- The normalization, built when a step first needs it.
  let mut normalization? : Option (Simp.Context × Simp.SimprocsArray) := none
  -- The normal conditions of each continuation folded into a local
  -- definition.
  let mut foldedNormal : Std.HashMap FVarId (Array Lean.Expr) := {}
  let mut leaves := 0
  -- A successful preparation pays for itself by closing its leaf. Bound
  -- failed speculation across the target, while allowing one unrelated bad
  -- clause without disabling preparation for all its siblings.
  let mut behaviorFailures := 0
  let mut stageCost : Array (String × Nat) := #[]
  let mut reported : Array (ObligationRange × String) := #[]
  let mut pending : Array (MVarId × Option Provenance × Array ObligationRange) :=
    (← getGoals).toArray.map fun g => (g, none, #[])
  -- The runs `denotedRun` reads, each the target's body the call rule
  -- inlines: no call of the function, so naming no function value.
  let mut denoted : Array MVarId := #[]
  while let some (goal, provenance, clauses) := pending.back? do
    pending := pending.pop
    let isDenoted := denoted.contains goal
    let (goal, clauses) ← match provenance with
      | some .loopEntry | some .loopIteration | some .assertion | some .mutationEnd
      | some .construction | some .memoryWritten | some .callReturned =>
          stripObligations goal clauses
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
          let name := if e.isAppOfArity ``LeanerIR.Proofs.Denote.propheticMeaning 7 then
              s!"{name} {e.getArg! 3}" else name
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
      let goal ← goal.replaceTargetDefEq unfolded
      -- The continuation's value and state are a program point (the folded
      -- conditions of a branching action follow them), kept for a target
      -- whose contract binds a state label.
      let goal ← match labeled, values[0]?, values[1]? with
        | true, some value, some state => noteProgramPoint goal value state
        | _, _, _ => pure goal
      setGoals [goal]
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
    if let some arguments ← assumeGoal? goal then
      -- An in-body assumption: what follows assumes the condition the
      -- meanings read for its site.
      let next ← goal.withContext (goal.apply
        (mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_assumeAt) arguments))
      let [continues] := next | throwError "the assumption rule did not leave its continuation"
      let (assumed, continues) ← continues.intro `assumptionHolds
      setGoals [continues]
      let holdsIdent := mkIdent `assumptionHolds
      -- The table the meanings read the condition from, at the site.
      let table? ← continues.withContext do
        let holds ← instantiateMVars (← assumed.getType)
        let entry := holds.getAppFn.constName? |>.bind fun name =>
          if name == ``LeanerIR.Proofs.Denote.assumptionOf then holds.getAppArgs[3]? else none
        pure (entry.bind (·.getAppFn.constName?))
      if let some table := table? then
        unless table == ``LeanerIR.Proofs.Denote.Meanings.assumption do
          evalTactic (← `(tactic| try simp only [$(mkIdent table):ident] at $holdsIdent:ident))
      evalTactic (← `(tactic| try simp only [LeanerIR.Proofs.Denote.closedMeanings,
        LeanerIR.Proofs.Denote.assumptionOf_none, LeanerIR.Proofs.Denote.assumptionOf_same,
        ↓reduceIte, Nat.reduceEqDiff] at $holdsIdent:ident))
      evalTactic (← `(tactic| try leaner_denote_normalize at $holdsIdent:ident))
      evalTactic (← `(tactic| all_goals leaner_denote_split_hypotheses))
      evalTactic (← `(tactic| all_goals leaner_denote_subst_vars))
      pending := pending ++ (← getGoals).toArray.map fun g => (g, provenance, clauses)
      stageCost := stageCost.push ("assume", (← IO.getNumHeartbeats) - stageStart)
      continue
    if let some (site, arguments) ← callAtGoal? goal then
      -- A call owing invariants where it returns is a cut there; any other
      -- is its meaning.
      let rule ← match invariants.find? (·.1 == site) with
        | some (_, function, condition) => do
            let owed ← atStart goal function (mkApp2 condition arguments[0]! arguments[1]!) site #[]
            pure (mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_callAt_checked)
              (arguments.extract 0 5 ++ #[owed] ++ arguments.extract 5 8))
        | none => pure (mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_callAt) arguments)
      let next ← goal.withContext (goal.apply rule)
      pending := pending ++ next.toArray.map fun g => (g, provenance, clauses)
      stageCost := stageCost.push ("call site", (← IO.getNumHeartbeats) - stageStart)
      continue
    if let some (owed, rest) ← callCheckedGoal? goal then
      match ← goal.withContext (goal.apply
          (mkApp2 (mkConst ``LeanerIR.Proofs.Denote.CallChecked.intro) owed rest)) with
      | [holds, continues] =>
          setGoals [continues]
          evalTactic (← `(tactic| intro invariantHolds))
          evalTactic (← `(tactic| try simp only [LeanerIR.Proofs.Obligation_iff] at invariantHolds))
          evalTactic (← `(tactic| try leaner_denote_normalize at invariantHolds))
          evalTactic (← `(tactic| all_goals leaner_denote_split_hypotheses))
          evalTactic (← `(tactic| all_goals leaner_denote_subst_vars))
          pending := pending ++ (← getGoals).toArray.map fun g => (g, provenance, clauses)
          setGoals [holds]
          evalTactic (← `(tactic| try leaner_denote_normalize))
          pending := pending ++ (← getGoals).toArray.map fun g => (g, some .callReturned, #[])
      | _ => throwError "the call check did not produce its two obligations"
      stageCost := stageCost.push ("call returned", (← IO.getNumHeartbeats) - stageStart)
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
      let invariant ← atStart goal function (mkApp2 invariant arguments[0]! arguments[1]!) site
        #[arguments[6]!, arguments[9]!]
      -- The loop's continuation is bound once, as a local definition the
      -- normalizer does not unfold: the loop hypothesis and every back edge
      -- carry it folded.
      let ensures := arguments[7]!
      let (continuation, goal) ← goal.withContext do
        let target ← instantiateMVars (← goal.getType)
        let defined ← goal.define continuationName (← inferType ensures) ensures
        let (continuation, goal) ← defined.intro1P
        let abstracted := target.replace fun e =>
          if e == ensures then some (mkFVar continuation) else none
        pure (continuation, ← goal.replaceTargetDefEq abstracted)
      let arguments := arguments.set! 7 (mkFVar continuation)
      let rule := mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_loopAt)
        (arguments.extract 0 7 ++ #[invariant] ++ arguments.extract 7 10)
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
    if let some (site, arguments) ← assertionGoal? goal then
      -- A cut: the condition is owed at its site, and assumed after it.
      let some (_, function, condition) := invariants.find? (·.1 == site)
        | throwError m!"no condition for the assertion at site {site}"
      let condition ← resolveAnchors goal
        (← atStart goal function (mkApp2 condition arguments[0]! arguments[1]!) site #[])
      let rule := mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_assertAt)
        (arguments.extract 0 6 ++ #[condition] ++ arguments.extract 6 9)
      match ← goal.withContext (goal.apply rule) with
      | [holds, continues] =>
          setGoals [continues]
          let (_, continued) ← (← getMainGoal).intro `assertionHolds
          let holdsIdent := mkIdent `assertionHolds
          replaceMainGoal [continued]
          evalTactic (← `(tactic| try simp only [LeanerIR.Proofs.Obligation_iff] at $holdsIdent:ident))
          evalTactic (← `(tactic| try leaner_denote_normalize at $holdsIdent:ident))
          -- The lemmas a proof step applies give their conclusions.
          unless (← getGoals).isEmpty do
            let continued ← getMainGoal
            let holdsHypothesis ← continued.withContext do
              pure ((← getLCtx).findFromUserName? `assertionHolds |>.map (·.fvarId))
            if let some hypothesis := holdsHypothesis then
              replaceMainGoal [← replaceByLemmaFacts continued hypothesis]
              evalTactic (← `(tactic| try leaner_denote_normalize at $holdsIdent:ident))
          -- An assertion that normalizes to `False` closes what follows.
          unless (← getGoals).isEmpty do
            -- The continuation is normalized where it is applied, as after
            -- a step; the row and state it is applied to are normal.
            let normalization ← normalization?.getDM normalization
            normalization? := some normalization
            normalizeAround normalization #[arguments[5]!, arguments[8]!]
            evalTactic (← `(tactic| all_goals leaner_denote_split_hypotheses))
            evalTactic (← `(tactic| all_goals leaner_denote_subst_vars))
            -- A split step continues once per case, each normalized under
            -- its case.
            let before ← getGoals
            let cases ← before.flatMapM splitCases
            setGoals cases
            if cases.length != before.length then
              evalTactic (← `(tactic| all_goals (try leaner_denote_normalize at *)))
            pending := pending ++ (← getGoals).toArray.map fun g => (g, provenance, clauses)
          -- What a proof step owes: a lemma's premise for each application.
          setGoals [holds]
          evalTactic (← `(tactic| try leaner_denote_normalize))
          let owed ← (← getGoals).toArray.flatMapM dischargeLemmaSteps
          for (part, premise) in owed do
            setGoals [part]
            -- A premise is unfolded, so normalized again.
            if premise then evalTactic (← `(tactic| try leaner_denote_normalize))
            pending := pending ++ (← getGoals).toArray.map fun g =>
              (g, some (if premise then .lemmaRequirement else .assertion), #[])
      | _ => throwError "the assertion rule did not produce its two obligations"
      stageCost := stageCost.push ("assertion", (← IO.getNumHeartbeats) - stageStart)
      continue
    -- A data invariant where a mutation of a local ends, and of a value
    -- where it is constructed: cuts, as an assertion is.
    if let some (site, arguments) ← memoryWrittenGoal? goal then
      -- A cut over the invariants the write owes, the memory before it read
      -- from its anchor; a write whose function owes none there passes.
      let condition ← match invariants.find? (·.1 == site) with
        | some (_, function, condition) =>
            resolveAnchors goal (← atStart goal function (mkApp2 condition arguments[0]! arguments[1]!) site #[])
        | none => goal.withContext do
            withLocalDeclD `env (← inferType arguments[5]!) fun env =>
            withLocalDeclD `state (mkApp (mkConst ``LeanerIR.Proofs.Denote.Memory) arguments[0]!)
              fun state =>
              mkLambdaFVars #[env, state] (mkConst ``True)
      let rule := mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_memoryWrittenAt)
        (arguments.extract 0 6 ++ #[condition] ++ arguments.extract 6 9)
      match ← goal.withContext (goal.apply rule) with
      | [holds, continues] =>
          setGoals [continues]
          evalTactic (← `(tactic| intro invariantHolds))
          evalTactic (← `(tactic| try simp only [LeanerIR.Proofs.Obligation_iff] at invariantHolds))
          evalTactic (← `(tactic| try leaner_denote_normalize at invariantHolds))
          unless (← getGoals).isEmpty do
            let normalization ← normalization?.getDM normalization
            normalization? := some normalization
            normalizeAround normalization #[arguments[5]!, arguments[8]!]
            evalTactic (← `(tactic| all_goals leaner_denote_split_hypotheses))
            evalTactic (← `(tactic| all_goals leaner_denote_subst_vars))
            pending := pending ++ (← getGoals).toArray.map fun g => (g, provenance, clauses)
          setGoals [holds]
          evalTactic (← `(tactic| try leaner_denote_normalize))
          pending := pending ++ (← getGoals).toArray.map fun g => (g, some .memoryWritten, #[])
      | _ => throwError "the memory write rule did not produce its two obligations"
      stageCost := stageCost.push ("memory", (← IO.getNumHeartbeats) - stageStart)
      continue
    let check ← match ← mutationEndGoal? goal with
      | some (site, arguments) => pure (some (site, arguments, false))
      | none => match ← constructionGoal? goal with
        | some (site, arguments) => pure (some (site, arguments, true))
        | none => pure none
    if let some (site, arguments, constructed) := check then
      let some (_, function, condition) := invariants.find? (·.1 == site)
        | throwError m!"no data invariant for the site {site}"
      let condition ← atStart goal function (mkApp2 condition arguments[0]! arguments[1]!) site #[]
      let (rule, entry, initial, origin) := if constructed then
          (mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_constructedAt)
            (arguments.extract 0 8 ++ #[condition] ++ arguments.extract 8 11),
            arguments[7]!, arguments[10]!, Provenance.construction)
        else
          (mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_mutationEndAt)
            (arguments.extract 0 6 ++ #[condition] ++ arguments.extract 6 9),
            arguments[5]!, arguments[8]!, Provenance.mutationEnd)
      match ← goal.withContext (goal.apply rule) with
      | [holds, continues] =>
          setGoals [continues]
          evalTactic (← `(tactic| intro invariantHolds))
          evalTactic (← `(tactic| try simp only [LeanerIR.Proofs.Obligation_iff] at invariantHolds))
          evalTactic (← `(tactic| try leaner_denote_normalize at invariantHolds))
          unless (← getGoals).isEmpty do
            let normalization ← normalization?.getDM normalization
            normalization? := some normalization
            normalizeAround normalization #[entry, initial]
            evalTactic (← `(tactic| all_goals leaner_denote_split_hypotheses))
            evalTactic (← `(tactic| all_goals leaner_denote_subst_vars))
            pending := pending ++ (← getGoals).toArray.map fun g => (g, provenance, clauses)
          setGoals [holds]
          evalTactic (← `(tactic| try leaner_denote_normalize))
          pending := pending ++ (← getGoals).toArray.map fun g => (g, some origin, #[])
      | _ => throwError "the data invariant rule did not produce its two obligations"
      stageCost := stageCost.push ("invariant", (← IO.getNumHeartbeats) - stageStart)
      continue
    if let some (site, arguments) ← anchorGoal? goal then
      -- The locals and state here are recorded for the assertions reading
      -- the anchor.
      let rule := mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_anchorAt) (arguments.extract 0 9)
      let [continues] ← goal.withContext (goal.apply rule)
        | throwError "the anchor rule did not produce its one obligation"
      let recorded ← continues.withContext do
        let proof ← mkAppM ``AnchorSaved.intro #[Lean.toExpr site, arguments[5]!, arguments[8]!]
        let (_, recorded) ← (← continues.assert `leanerAnchor (← inferType proof) proof).intro1P
        pure recorded
      setGoals [recorded]
      let normalization ← normalization?.getDM normalization
      normalization? := some normalization
      normalizeAround normalization #[arguments[5]!, arguments[8]!]
      pending := pending ++ (← getGoals).toArray.map fun g => (g, provenance, clauses)
      stageCost := stageCost.push ("anchor", (← IO.getNumHeartbeats) - stageStart)
      continue
    if ← scalarAt? (← contextFacts goal) goal then
      stageCost := stageCost.push ("scalar rows", (← IO.getNumHeartbeats) - stageStart)
      continue
    if ← literalClosureTyped? goal then
      stageCost := stageCost.push ("closure typing", (← IO.getNumHeartbeats) - stageStart)
      continue
    if let some remaining ← literalClosureKeepsMemory? goal callees then
      let clauses ← goal.withContext do
        let target ← instantiateMVars (← goal.getType)
        pure <| match obligationRange? target with
          | some range => if clauses.contains range then clauses else clauses.push range
          | none => clauses
      pending := pending ++ remaining.toArray.map fun g => (g, provenance, clauses)
      stageCost := stageCost.push ("closure frame", (← IO.getNumHeartbeats) - stageStart)
      continue
    if let some remaining ← unseenInvocation? goal then
      setGoals remaining
      evalTactic (← `(tactic|
        all_goals (try dsimp only [LeanerIR.Proofs.Denote.ResultShape.row] at *)))
      evalTactic (← `(tactic|
        all_goals (try leaner_denote_normalize at $(mkIdent `leanerResultOf):ident)))
      evalTactic (← `(tactic| all_goals (try leaner_denote_normalize)))
      setGoals (← (← getGoals).mapM fun g => liftM (rewriteResultOf g))
      evalTactic (← `(tactic| all_goals (try leaner_denote_normalize)))
      pending := pending ++ (← getGoals).toArray.map fun g => (g, provenance, clauses)
      stageCost := stageCost.push ("invoke", (← IO.getNumHeartbeats) - stageStart)
      continue
    if let some (handle, _) ← callGoal? goal callees then
      let some (_, calleeName, theoremProof) ← callees.findM? fun (candidate, _, _) =>
          goal.withContext (isDefEq candidate handle)
        | throwError m!"no verified callee for {handle}"
      -- Where the proof states a behavioral predicate, the call shows one
      -- of the function value naming its target (`wp_named`).
      let goal ← if isDenoted then pure goal else namedCall goal
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
        let rule := mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_propheticMeaning_of_compiled)
          #[action.getArg! 0, action.getArg! 1, action.getArg! 2, action.getArg! 3, action.getArg! 4,
            compiled, theoremProof, action.getArg! 7, target.getArg! 4, target.getArg! 5,
            target.getArg! 6]
        let next ← goal.withContext do
          let .forallE _ premise _ _ := ← whnfR (← inferType rule)
            | throwError "the agreement rule does not take the callee's denotation"
          let next ← mkFreshExprSyntheticOpaqueMVar premise (← goal.getTag)
          goal.assign (mkApp rule next)
          pure next.mvarId!
        setGoals [next]
        -- The inlined callee starts here: its loops' invariants read `old`
        -- from its arguments at the call.
        if ← invariants.anyM fun (_, function, _) => goal.withContext (isDefEq function handle) then
          let inlined ← getMainGoal
          let started ← inlined.withContext do
            let proof ← mkAppM ``FunctionStart.intro
              #[action.getArg! 4, action.getArg! 7, target.getArg! 6]
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
      -- The theorem's meaning is the call's: its frame and instantiation are
      -- the call's, found before the rule's types are unified.
      goal.withContext do
        let target ← instantiateMVars (← goal.getType)
        let_expr LeanerIR.Proofs.Satisfies _ _ _ _ meaning _ := ← whnfR proofType | pure ()
        if target.isAppOfArity ``LeanerIR.Proofs.wp 7 then
          let action := (target.getArg! 3).consumeMData
          if action.isApp then discard <| isDefEq meaning action.appFn!
      evalTactic (← `(tactic| refine LeanerIR.Proofs.Denote.wp_call $proofSyntax ?_ ?_ ?_ ?_))
      let subgoals ← getGoals
      -- A callee's theorem assumes the natives it reaches, a generic one's
      -- at every family and instantiation, and that the in-body assumptions
      -- of the functions it relies on hold; the caller assumes them too.
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
            pure (body.isAppOf ``LeanerIR.Proofs.Satisfies ||
              body.isAppOf ``LeanerIR.NativesTyped || body.isAppOf ``LeanerIR.NativesShift ||
              body.isAppOf ``LeanerIR.Proofs.Terminating ||
              body.isAppOf ``LeanerIR.Proofs.AssumptionsHold ||
              -- An assumed step of a lemma the callee applies.
              (match body.getAppFn with
                | .const (.str _ part) _ => part.startsWith "lemmaTrusted_"
                | _ => false))
          if native then id.assumption
      let constants ← goal.withContext (contractConstants (← inferType theoremProof))
      let lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ← constants.mapM fun name => do
        let ident := mkIdent (rootNamespace ++ name)
        `(Lean.Parser.Tactic.simpLemma| $ident:ident)
      for (subgoal, index) in subgoals.toArray.zipIdx do
        setGoals [subgoal]
        evalTactic (← `(tactic| try simp only [$lemmas,*, lir_denote_norm]))
        evalTactic (← `(tactic| try leaner_denote_normalize))
        -- A result the contract fixes to a few literal values, decided here.
        if index == 2 then
          evalTactic (← `(tactic| all_goals (try leaner_denote_call_result_cases)))
        -- What the callee's theorem assumes and its precondition, then
        -- the continuation and the failures.
        let origin := if index ≤ 1 then some (Provenance.precondition calleeName)
          else if index == 2 then some (.continuation calleeName) else none
        pending := pending ++ (← getGoals).toArray.map fun g =>
          (g, origin, if index ≤ 1 then #[] else clauses)
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
    -- A literal closure's run, by its target's body where the target has
    -- no theorem, and its abort, by its body where the proof inlines it.
    let goal ← introduceBehaviors goal
    if let some cases ← abortCases? goal then
      pending := pending ++ cases.toArray.map fun g => (g, provenance, clauses)
      stageCost := stageCost.push ("abort cases", (← IO.getNumHeartbeats) - stageStart)
      continue
    if let some run ← denotedRun? goal callees then
      pending := pending.push (run, provenance, clauses)
      denoted := denoted.push run
      stageCost := stageCost.push ("denoted run", (← IO.getNumHeartbeats) - stageStart)
      continue
    -- A literal closure's behavior, by its target's theorem.
    let behaviorStart ← IO.getNumHeartbeats
    let goal ← terminatingRuns goal
    let terminatingDone ← IO.getNumHeartbeats
    let goal ← rewriteResultOf goal
    let rewritingDone ← IO.getNumHeartbeats
    -- An authored proof may need only the facts the ordinary call rule
    -- already supplied. Try those before re-deriving behavioral contracts;
    -- retain the original path when higher-order facts are still needed.
    let hasBehavior ← if residual && behaviorFailures < 2 then goal.withContext do
        (← getLCtx).anyM fun decl => do
          if decl.isImplementationDetail then return false
          return statesLiteralBehavior (← instantiateMVars decl.type)
      else pure false
    if hasBehavior then
      let saved ← saveState
      let context ← readThe Core.Context
      let now ← IO.getNumHeartbeats
      let remaining := if context.maxHeartbeats == 0 then 2000000
        else context.initHeartbeats + context.maxHeartbeats - now
      let budget := max 1 (min remaining 2000000)
      let closed ← tryCatchRuntimeEx
        (withTheReader Core.Context
          (fun context => { context with initHeartbeats := now, maxHeartbeats := budget }) do
          closesBy goal (← `(tactic|
            (try leaner_denote_unname
             leaner_denote_prepare
             all_goals (first
               | leaner_denote_decide_residual
               | (solve | simp_all))))))
        (fun _ => pure false)
      let spent := (← IO.getNumHeartbeats) - now
      if closed then
        stageCost := stageCost.push ("existing behavior facts", spent)
        continue
      saved.restore
      behaviorFailures := behaviorFailures + 1
      stageCost := stageCost.push ("existing behavior attempt", spent)
    let derivationStart ← IO.getNumHeartbeats
    let goal ← dispatchBehavior goal callees
    stageCost := stageCost.push ("terminating facts", terminatingDone - behaviorStart)
      |>.push ("result/state rewriting", rewritingDone - terminatingDone)
      |>.push ("behavior facts", (← IO.getNumHeartbeats) - derivationStart)
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
  if leaner.denoteProfile.get (← getOptions) then
    let mut lines := #[s!"profile {(← Lean.Elab.Term.getDeclName?).getD .anonymous}: \
      {((← IO.getNumHeartbeats) - startHeartbeats) / 1000}k; stages"]
    let mut stages : Array (String × Nat × Nat) := #[]
    for (stage, cost) in stageCost do
      match stages.findIdx? (·.1 == stage) with
      | some index => stages := stages.modify index fun (name, count, sum) => (name, count + 1, sum + cost)
      | none => stages := stages.push (stage, 1, cost)
    for (stage, count, sum) in stages do
      lines := lines.push s!"  stage {stage} ×{count} {sum / 1000}k"
    let steps := (← stepProfile.get).toArray.qsort fun a b =>
      a.2.2.2.1 + a.2.2.2.2 > b.2.2.2.1 + b.2.2.2.2
    for (label, runs, failures, succeeding, failing) in steps do
      lines := lines.push s!"  step {label} ×{runs} ok {succeeding / 1000}k, \
        failed ×{failures} {failing / 1000}k"
    IO.println ("\n".intercalate lines.toList)
  setGoals residuals.toList

/-- Split the normalized goal into leaves and decide each; report the
clause of every leaf that is not decided.  Loops and in-body assertions are
handled by the predicates given as `(site, function, predicate)` triples, calls by the
callees' theorems,
and the `using` equations rewrite wherever a body brings their terms in. -/
declare_syntax_cat closeFlag (behavior := symbol)
/-- Leave the obligations the deciders do not close to an authored script. -/
syntax (name := residualFlag) &"residual" : closeFlag
/-- Keep the program points: the contract binds a state label. -/
syntax (name := labeledFlag) &"labeled" : closeFlag
syntax "leaner_denote_close" closeFlag* (" [" term,* "]")?
  (" with" " [" term,* "]")? (" using" " [" term,* "]")? : tactic

elab_rules : tactic
  | `(tactic| leaner_denote_close $flags* $[[$loops:term,*]]? $[with [$calls:term,*]]?
      $[using [$equations:term,*]]?) => do
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
      let flagged (kind : Name) := flags.any (·.raw.isOfKind kind)
      closeGoals invariants callees equations (flagged ``residualFlag) (flagged ``labeledFlag)

/-- Prove a step a lemma's proof owes: what its lemma applications require,
its assertions, and its case splits. -/
elab "leaner_denote_lemma_owed" : tactic => do
  let parts ← dischargeLemmaSteps (← getMainGoal)
  let mut residual := #[]
  for (part, _) in parts do
    setGoals [part]
    evalTactic (← `(tactic| try leaner_denote_normalize))
    evalTactic (← `(tactic| all_goals leaner_denote_close))
    residual := residual ++ (← getGoals).toArray
  setGoals residual.toList

/-- What a step of a lemma's proof gives once proved, in the hypothesis
`h`: each lemma application's conclusion, and one goal per case of a split. -/
elab "leaner_denote_lemma_holds " h:ident : tactic => do
  let goal ← getMainGoal
  let some decl := (← goal.withContext getLCtx).findFromUserName? h.getId
    | throwError m!"no hypothesis `{h.getId}`"
  replaceMainGoal [← replaceByLemmaFacts goal decl.fvarId]
  evalTactic (← `(tactic| try simp only [LeanerIR.Proofs.Obligation_iff] at $h:ident))
  evalTactic (← `(tactic| try leaner_denote_normalize at $h:ident))
  evalTactic (← `(tactic| leaner_denote_split_hypotheses))
  let before ← getGoals
  let cases ← before.flatMapM splitCases
  setGoals cases
  if cases.length != before.length then
    evalTactic (← `(tactic| all_goals (try leaner_denote_normalize at *)))

/-- A recursive application's measure descends: componentwise as the Move
Prover's condition states it, decided as an obligation. -/
macro "leaner_denote_lemma_decreasing" : tactic => `(tactic|
  ((try simp only [Prod.lex_def, LeanerIR.Proofs.lemmaMeasure])
   first
   | omega
   | ((try leaner_denote_normalize)
      leaner_denote_close residual
      done)
   | fail "a recursive application of the lemma does not decrease its measure"))

end LeanerIR.Proofs.Denote
