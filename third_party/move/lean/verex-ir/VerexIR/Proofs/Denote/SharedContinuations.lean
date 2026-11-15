-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import VerexIR.Proofs.Denote.Join

/-!
# Checked shared continuations

A continuation has a closed definition and explicit captured parameters.
Its constant identifies the program cut across substitution of branch locals.
Incoming goals are retained until all immediate predecessors have arrived.
Their contexts form alternatives in an entry relation, and a single theorem
proves the continuation under that relation. Only normal value flows join.
-/

namespace VerexIR.Proofs.Denote.SharedContinuations

open Lean Meta Elab Tactic

initialize registerTraceClass `verex.joinPruning

/-- A constructor reconstructed over abstracted fields does not have the
expected type (`withConstructorFields`). -/
initialize mismatchedShape : InternalExceptionId ←
  registerInternalExceptionId `verexMismatchedShape

/-- Count visible normal exits, capped at two. Matchers used by reads and
checked operations often have just one successful arm; their failure arms
do not reach a value continuation. Only inspect computation structure, not
conditionals inside its types, values or captured specifications. Hidden
actions are left to the existing walker, which exposes their structure. -/
private partial def normalExits (action : Lean.Expr) (flow : Bool) : MetaM Nat := do
  let action := action.consumeMData.headBeta
  if action.isLambda then return ← normalExits action.getLambdaBody flow
  if let .letE _ _ value body _ := action then
    return ← normalExits (body.instantiate1 value) flow
  if action.isAppOf ``Spec.abort || action.isAppOf ``abortAt || action.isAppOf ``Spec.stuck then
    return 0
  if action.isAppOfArity ``Spec.pure 4 then
    let value := (action.getArg! 3).consumeMData.headBeta
    return if flow && (value.isAppOf ``Flow.return_ || value.isAppOf ``Flow.break_ ||
      value.isAppOf ``Flow.continue_) then 0 else 1
  if action.isAppOfArity ``Flow.bind 8 then
    return min 2 ((← normalExits (action.getArg! 6) true) *
      (← normalExits (action.getArg! 7) flow))
  if action.isAppOfArity ``Spec.bind 6 then
    return min 2 ((← normalExits (action.getArg! 4) false) *
      (← normalExits (action.getArg! 5) flow))
  if action.isAppOfArity ``ite 5 || action.isAppOfArity ``dite 5 then
    return min 2 ((← normalExits (action.getArg! 3) flow) +
      (← normalExits (action.getArg! 4) flow))
  if let some matcher ← matchMatcherApp? action then
    let mut exits := 0
    for alternative in matcher.alts do
      exits := min 2 (exits + (← normalExits alternative flow))
      if exits == 2 then break
    return exits
  if (action.isAppOf ``branchAt || action.isAppOf ``armAt) && action.getAppNumArgs ≥ 1 then
    return ← normalExits action.appArg! flow
  if action.isAppOf ``loopAt then return 2
  return 1

/-- Whether visible program branching warrants a shared node. A negative
answer retains ordinary folded traversal; it never discards a proof path. -/
def mayJoin (action : Lean.Expr) : MetaM Bool := do
  return (← normalExits action true) > 1

/-- Whether an action is an exit: `wp` reduces it to its postcondition or
abort condition in one step, with no further program rule. -/
private def exitAction (action : Lean.Expr) : Bool :=
  match action.consumeMData.headBeta.getAppFn with
  | .const name _ =>
      name == ``ResultShape.finish || name == ``VerexIR.Proofs.Spec.pure ||
        name == ``VerexIR.Proofs.Spec.bottom || name == ``Mutables.resolve
  | _ => false

/-- Whether a continuation still runs program (an action under `wp` that is
not an exit), itself or through a local or shared continuation it applies.
A shared class amortizes only the rule applications of its suffix, never its
leaves: its entry relation is a disjunction over the arrivals, so the leaf
work stays per arrival. One that only exits and states the postcondition has
nothing to share: substituting it per path is cheaper than a node. -/
def carriesProgram (shared : Array Name) (continuation : Lean.Expr) : MetaM Bool := do
  let mut pending := #[← instantiateMVars continuation]
  let mut seenConstants : Std.HashSet Name := {}
  let mut seenLocals : Std.HashSet FVarId := {}
  while let some next := pending.back? do
    pending := pending.pop
    if (next.find? fun e => e.isAppOfArity ``wp 7 && !exitAction (e.getArg! 3)).isSome then
      return true
    let constants := next.getUsedConstants
    for name in constants do
      unless shared.contains name && !seenConstants.contains name do continue
      seenConstants := seenConstants.insert name
      if let some value := (← getConstInfo name).value? then pending := pending.push value
    for id in (collectFVars {} next).fvarIds do
      if seenLocals.contains id then continue
      seenLocals := seenLocals.insert id
      if let some value := (← id.getDecl).value? then
        pending := pending.push (← instantiateMVars value)
  return false

structure Arrival where
  goal : MVarId
  arguments : Array Lean.Expr
  /-- Filled when the node is sealed: a single arrival never closes them. -/
  locals : Array Lean.LocalDecl := #[]
  /-- The arrival came from inside a sealed class's suffix: a continuation
  folded on the tree's own path can be reached from shared work. -/
  summarized : Bool := false
  /-- The arriving step, as the walk recorded it: the arrival that resumes
  alone continues its own path. -/
  step : Nat := 0
  /-- Filled when the node is sealed: the locals its arm closes. -/
  armLocals : Array Lean.LocalDecl := #[]
  /-- Filled when the node is sealed: its proofs of the node's `commonFacts`. -/
  factProofs : Array Lean.Expr := #[]
  deriving Inhabited

/-- A summary parameter supplied by one incoming argument or one of its
constructor fields. Field positions index application arguments in the
constructors checked while sealing. -/
structure EntryArgument where
  parameter : Nat
  fields : Array Nat := #[]
  deriving Inhabited

private def EntryArgument.select (source : EntryArgument) (incoming : Array Lean.Expr) : Lean.Expr :=
  source.fields.foldl (fun value field => value.consumeMData.headBeta.getArg! field)
    incoming[source.parameter]!

structure Node where
  continuation : Lean.Expr
  captures : Nat
  arrivals : Array Arrival := #[]
  arms : Array Lean.Expr := #[]
  /-- The propositions every arrival holds over the enclosing context, stated
  once before the arms. -/
  commonFacts : Array Lean.Expr := #[]
  enclosing : Array FVarId := #[]
  relation? : Option Lean.Expr := none
  summary? : Option MVarId := none
  summaryArguments : Array EntryArgument := #[]
  /-- Per arrival, the proof of the stored invariants of its memory, when
  every arrival proved them: the summary then assumes them of its memory
  row (`sealNode`). Empty otherwise. -/
  invariantProofs : Array Lean.Expr := #[]
  sealed : Bool := false
  finished : Bool := false
  /-- Only a closed proof without reporting admissions is certified. -/
  certified : Bool := false
  /-- The locals of the context the continuation was folded in, by type. -/
  origin : Std.HashMap FVarId Lean.Expr := {}
  /-- The locals the closed body abstracts, in its telescope's order: the
  captures as lambdas, the local definitions among them as lets. -/
  abstracted : Array FVarId := #[]
  deriving Inhabited

def Node.name (node : Node) : Name := node.continuation.getAppFn.constName!

/-- The hypotheses the summary states before its entry relation. -/
def Node.hypotheses (node : Node) : Nat :=
  if node.invariantProofs.isEmpty then 0 else 1

/-- A closed telescope (`defineClosed`) at its arguments: the lambdas take
the arguments, and the local definitions between them are expanded. -/
partial def instantiateTelescope (value : Lean.Expr) (arguments : Array Lean.Expr) : Lean.Expr :=
  match value with
  | .lam _ _ body _ =>
      if arguments.isEmpty then value else instantiateTelescope (body.instantiate1 arguments[0]!) (arguments.extract 1)
  | .letE _ _ definition body _ => instantiateTelescope (body.instantiate1 definition) arguments
  | .mdata _ e => instantiateTelescope e arguments
  | _ => mkAppN value arguments

/-- The closed applications of the shared continuations in a term, each
subterm visited once: the term is a DAG, and its tree can be exponentially
larger. -/
private def sharedApplications (shared : Array Name) (e : Lean.Expr) : Array Lean.Expr :=
  let rec walk (e : Lean.Expr) : StateM (Std.HashSet Lean.Expr × Array Lean.Expr) Unit := do
    if (← get).1.contains e then return
    modify fun (seen, found) => (seen.insert e, found)
    if let .const name _ := e.getAppFn then
      if shared.contains name && !e.hasLooseBVars then
        modify fun (seen, found) => (seen, found.push e)
    match e with
    | .app function argument => walk function; walk argument
    | .lam _ type body _ | .forallE _ type body _ => walk type; walk body
    | .letE _ type value body _ => walk type; walk value; walk body
    | .mdata _ body | .proj _ _ body => walk body
    | _ => pure ()
  ((walk e).run ({}, #[])).2.2

/-- The stored invariants the continuation owes of a memory, as the loop cut
states them (`DataInvariants inv m₀ → DataInvariants inv m`): the
predicate `DataInvariants inv` and the memory `m₀` they are assumed of, at
the given capture arguments. A continuation that reaches them only through
an enclosing shared continuation it applies states what that one states, at
the arguments it applies it to. None where no such implication is stated.
Whether a continuation states one does not depend on its arguments, so each
one is explored once, at the arguments of its first application found. -/
partial def storedInvariantIn? (value : Lean.Expr) (arguments : Array Lean.Expr)
    (shared : Array Name) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let rec explore (value : Lean.Expr) (arguments : Array Lean.Expr) :
      StateT (Std.HashSet Name) MetaM (Option (Lean.Expr × Lean.Expr)) := do
    let body := instantiateTelescope value arguments
    let invariants (e : Lean.Expr) :=
      e.consumeMData.isAppOfArity ``VerexIR.Proofs.Denote.DataInvariants 4
    let implication? := body.find? fun e => match e with
      | .forallE _ domain conclusion _ =>
          invariants domain && invariants conclusion && !conclusion.hasLooseBVar 0 && !domain.hasLooseBVars
      | _ => false
    if let some implication := implication? then
      let premise := implication.bindingDomain!.consumeMData
      return some (mkAppN premise.getAppFn (premise.getAppArgs.extract 0 3), premise.getArg! 3)
    for application in sharedApplications shared body do
      let name := application.getAppFn.constName!
      if (← get).contains name then continue
      modify (·.insert name)
      let some inner := (← getConstInfo name).value? | continue
      if let some found ← explore inner application.getAppArgs then
        return some found
    return none
  (explore value arguments).run' {}

def storedInvariantOf? (node : Node) (arguments : Array Lean.Expr) (shared : Array Name) :
    MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let some value := (← getConstInfo node.name).value? | return none
  storedInvariantIn? value (arguments.extract 0 node.captures) shared

/-- The stored invariants of `memory` under those of the memory the
continuation assumes them of (`storedInvariantOf?`). -/
def storedInvariantStatement (invariant : Lean.Expr × Lean.Expr) (memory : Lean.Expr) : MetaM Lean.Expr :=
  mkArrow (mkApp invariant.1 invariant.2) (mkApp invariant.1 memory)

/-- Whether an incoming context states no fact beyond the context its node
was folded in: only fresh values, whose ranges are satisfiable. Refuting it
would refute the fold point, which the walk continued from. -/
def Node.unchanged (node : Node) (goal : MVarId) : MetaM Bool := goal.withContext do
  (← getLCtx).allM fun decl => do
    if decl.isImplementationDetail then return true
    let type ← instantiateMVars decl.type
    match node.origin[decl.fvarId]? with
    | some original => return original == type
    | none => return !decl.isLet && !(← isProp type)

def unfold (node : Node) (goal : MVarId) : MetaM MVarId := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  withTransparency .all <| goal.replaceTargetDefEq (← deltaExpand target (· == node.name))

/-- Unfold the continuation at an arrival that resumes alone, in the form
the default walk's exit has: the body's lambdas take the arrival's
arguments, and a local definition of the fold context the arrival still
holds, with the same value, stays that local rather than its value. Delta
expansion would leave the definitions as `let` terms, which normalization
reduces and traverses where the default walk keeps an atom with its facts. -/
def unfoldArrival (node : Node) (arrival : Arrival) : MetaM MVarId := arrival.goal.withContext do
  let target ← instantiateMVars (← arrival.goal.getType)
  unless target.isAppOf node.name do throwError "an arrival lost its shared continuation"
  let arguments := target.getAppArgs
  let info ← getConstInfo node.name
  let mut body := info.instantiateValueLevelParams! node.continuation.constLevels!
  let mut next := 0
  let mut relinked : Nat := 0
  let mut expanded : Nat := 0
  let lctx ← getLCtx
  for id in node.abstracted do
    match body with
    | .lam _ _ inner _ =>
        let some argument := arguments[next]? | throwError "an arrival lost a captured argument"
        body := inner.instantiate1 argument
        next := next + 1
    | .letE _ _ value inner _ =>
        let same ← match lctx.find? id with
          | some decl => do
              let some original := decl.value? | pure false
              let original ← instantiateMVars original
              unless original == value do
                trace[verex.joinPruning] "{node.name} alone: definition {mkFVar id} differs: context {original}; body {value}"
              pure (original == value)
          | none => do
              trace[verex.joinPruning] "{node.name} alone: definition {id.name} is gone"
              pure false
        if same then relinked := relinked + 1 else expanded := expanded + 1
        body := inner.instantiate1 (if same then mkFVar id else value)
    | .mdata _ inner => body := inner
    | _ => throwError "a shared continuation's body lost its telescope"
  let unfolded := (mkAppN body (arguments.extract next arguments.size)).headBeta
  trace[verex.joinPruning] "{node.name} alone: {relinked} definitions relinked, {expanded} expanded"
  withTransparency .all <| arrival.goal.replaceTargetDefEq unfolded

/-- Rewriting can make an older hypothesis depend on a newer local. Close
the telescope in dependency order, retaining definitions as definitions. -/
private def orderedLocals : MetaM (Array Lean.LocalDecl) := do
  let mut remaining := ((← getLCtx).decls.toList.filterMap id).toArray.filter (!·.isAuxDecl)
  let mut placed : Std.HashSet FVarId := {}
  let mut ordered := #[]
  while !remaining.isEmpty do
    let some index := remaining.findIdx? (fun decl =>
        let dependencies := collectFVars
          (decl.value?.map (collectFVars {}) |>.getD {}) decl.type
        dependencies.fvarIds.all placed.contains)
      | throwError "cannot close the dependent context of a shared continuation"
    let decl := remaining[index]!
    ordered := ordered.push decl
    placed := placed.insert decl.fvarId
    remaining := remaining.eraseIdx! index
  return ordered

/-- The value is closed, so there is no telescope to abstract: declare it as
it is. `mkAuxDefinition` would `Meta.check` the whole body to classify
dependent lets, a depth-bounded traversal a long program suffix exceeds. -/
private def defineClosed (kind : Name) (value : Lean.Expr) : MetaM Lean.Expr := do
  let value ← instantiateMVars value
  unless !value.hasFVar && !value.hasExprMVar && !value.hasLevelMVar do
    throwError "a shared continuation definition has unresolved dependencies"
  let name ← mkAuxDeclName (kind := kind)
  let type := (← instantiateMVars (← inferType value)).headBeta
  let levelParams := (collectLevelParams (collectLevelParams {} type) value).params.toList
  let hints := ReducibilityHints.regular (getMaxHeight (← getEnv) value + 1)
  addDecl <| .defnDecl (← mkDefinitionValInferringUnsafe name levelParams type value hints)
  setReducibilityStatus name .irreducible
  return mkConst name (levelParams.map mkLevelParam)

/-- Close the postcondition before its action branches. The capture
arguments remain visible to substitution; the continuation body stays named.
A local definition in `opaqueDefinitions` is one the walk matches by identity (a loop's
continuation, which its hypothesis and back edges carry folded): it is a
capture, so the suffix still reads that local where the walk resumes. -/
def fold? (goal : MVarId) (opaqueDefinitions : Std.HashSet FVarId := {}) :
    MetaM (Option (MVarId × Node)) := goal.withContext do
  let target := (← instantiateMVars (← goal.getType)).consumeMData
  unless target.isAppOfArity ``wp 7 do return none
  let resultType ← whnfR (target.getArg! 2)
  unless resultType.isAppOfArity ``Flow 4 do return none
  let ensures := target.getArg! 4
  if ensures.hasExprMVar then return none
  let allLocals ← orderedLocals
  -- The body captures only what it reads, including transitive type and
  -- definition dependencies. Surrounding assumptions and observations are
  -- retained by the incoming contexts and the summary's enclosing context;
  -- they need not be arguments of every candidate continuation.
  let mut needed := (collectFVars {} ensures).fvarIds.foldl (·.insert ·) ({} : Std.HashSet FVarId)
  for decl in allLocals.reverse do
    unless needed.contains decl.fvarId do continue
    let dependencies := collectFVars
      (if decl.isLet then decl.value?.map (collectFVars {}) |>.getD {} else {}) decl.type
    needed := dependencies.fvarIds.foldl (·.insert ·) needed
  let locals := allLocals.filter (needed.contains ·.fvarId)
  let captures := (locals.filter (fun decl => !decl.isLet || opaqueDefinitions.contains decl.fvarId)).map
    (·.toExpr)
  -- `isLet` excludes nondependent lets (for example facts introduced by
  -- `have`): their values are hidden and they are explicit capture arguments.
  -- Generalize those lets while retaining ordinary let-bound observations;
  -- an opaque definition is abstracted as an assumption, its value hidden.
  let lctx := locals.foldl (init := ← getLCtx) fun lctx decl =>
    if decl.isLet && opaqueDefinitions.contains decl.fvarId then
      lctx.modifyLocalDecl decl.fvarId fun decl =>
        .cdecl decl.index decl.fvarId decl.userName decl.type .default decl.kind
    else lctx
  let body ← withLCtx lctx (← getLocalInstances) do
    mkLambdaFVars (locals.map (·.toExpr)) ensures (generalizeNondepLet := true)
  let continuation ← defineClosed `_verexContinuation body
  let post := mkAppN continuation captures
  let folded ← withTransparency .all <| goal.replaceTargetDefEq
    (mkAppN target.getAppFn (target.getAppArgs.set! 4 post))
  let origin ← allLocals.foldlM (init := {}) fun found decl => do
    return found.insert decl.fvarId (← instantiateMVars decl.type)
  let node : Node := { continuation, captures := captures.size, origin,
                       abstracted := locals.map (·.fvarId) }
  return some (folded, node)

/-- Saved program continuations that the named suffix no longer reads need
not become existential witnesses in its entry relation. Retain all facts,
and let `tryClear` protect definitions used by the suffix or other locals. -/
private def clearSavedComputations (goal : MVarId) : MetaM MVarId := goal.withContext do
  let mut candidates : Array FVarId := #[]
  for decl in ← getLCtx do
    unless decl.isLet do continue
    let some value := decl.value? | continue
    let constants := value.getUsedConstants
    let alias := value.isFVar && candidates.contains value.fvarId!
    unless alias || constants.contains ``wp || constants.contains ``Spec do continue
    if ← isProp decl.type then continue
    candidates := candidates.push decl.fvarId
  let mut goal := goal
  for id in candidates.reverse do
    goal ← goal.tryClear id
  return goal

/-- The arguments of a normal arrival at the node: its captures, the value
and locals of its flow, and its memory. None for an abrupt flow. -/
private def arrivalArguments? (node : Node) (goal : MVarId) : MetaM (Option (Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOf node.name do return none
    let arguments := target.getAppArgs
    unless arguments.size == node.captures + 2 do return none
    let flow := arguments[node.captures]!.consumeMData.headBeta
    unless flow.isAppOfArity ``Flow.value 6 do return none
    return some (arguments.extract 0 node.captures ++ #[flow.getArg! 4, flow.getArg! 5, arguments.back!])

/-- Abrupt flows keep their original destinations; only a value carries
locals into the shared suffix. -/
def arrival? (node : Node) (goal : MVarId) : MetaM (Option Arrival) := do
  let some arguments ← arrivalArguments? node goal | return none
  return some { goal, arguments }

/-- State the arrival's arguments over the oldest terms its context equates
them to, opportunistically: a branch-local variable (one the node's fold did
not see) that an equation of the context, in either orientation, equates to
a term without it is eliminated by that equation, newest first, while such a
variable occurs in the arguments; chains of equations are followed as each
elimination exposes the next variable. An equation whose dependent
elimination fails is skipped and the name stays. The arrivals then agree
where they differed only by such a name, and anti-unification keeps the
common term instead of abstracting a row. An equation the walk keeps on
purpose (`kept`, a call's contract frame) is never used: the suffix and its
leaves read it as the default walk leaves it. The arrival's goal is
substituted, so its hole is assigned the proof of the substituted goal. -/
private def substituteArguments (node : Node) (kept : FVarIdSet) (arrival : Arrival) :
    MetaM Arrival := do
  let mut goal := arrival.goal
  let mut arguments := arrival.arguments
  let mut attempted : Std.HashSet FVarId := {}
  repeat
    let occurring := arguments.foldl (fun found argument => collectFVars found argument) {}
    let equation? ← goal.withContext do
      let lctx ← getLCtx
      -- The variable to eliminate, with the equation and its direction
      -- (`symm` eliminates the right side), the newest variable first.
      let mut best : Option (Nat × FVarId × Bool) := none
      for decl in lctx do
        if decl.isImplementationDetail || decl.isLet || attempted.contains decl.fvarId ||
            kept.contains decl.fvarId then continue
        let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | continue
        let eliminable (side other : Lean.Expr) : Option (Nat × FVarId × Bool) := do
          let .fvar id := side | none
          guard (occurring.fvarSet.contains id && !node.origin.contains id)
          let eliminated ← lctx.find? id
          guard (!eliminated.isLet && !other.containsFVar id)
          pure (eliminated.index, decl.fvarId, side == rhs)
        let candidate := match eliminable lhs rhs, eliminable rhs lhs with
          | some left, some right => if left.1 ≥ right.1 then some left else some right
          | left, right => left <|> right
        if let some candidate := candidate then
          if best.all (·.1 < candidate.1) then best := some candidate
      return best
    let some (_, equation, symm) := equation? | break
    attempted := attempted.insert equation
    -- Dependent elimination can fail on a dependent context; the arrival
    -- then keeps that name.
    let some (_, substituted) ← observing? (substCore goal equation (symm := symm)) | continue
    let some next ← arrivalArguments? node substituted
      | throwError "substituting an arrival's arguments lost its continuation"
    goal := substituted
    arguments := next
  return { arrival with goal, arguments }

/-- Common context is independent of the body's capture telescope. Preserve
only declarations whose identities, types, definitions and dependencies
agree at every predecessor, including facts used only by proof automation. -/
private def commonLocals (node : Node) : Array Lean.LocalDecl := Id.run do
  let mut common := #[]
  let mut retained : Std.HashSet FVarId := {}
  for decl in node.arrivals[0]!.locals do
    let dependencies := collectFVars
      (decl.value?.map (collectFVars {}) |>.getD {}) decl.type
    unless dependencies.fvarIds.all retained.contains do continue
    unless node.arrivals.all (fun arrival => arrival.locals.any fun other =>
        other.fvarId == decl.fvarId && other.type == decl.type && other.value? == decl.value? &&
          other.binderInfo == decl.binderInfo && other.kind == decl.kind) do continue
    common := common.push decl
    retained := retained.insert decl.fvarId
  return common

/-- Captured arguments supplied by the exact same dominating local. -/
private def commonCaptures (node : Node) (enclosing : Array FVarId) : Array (Nat × FVarId) := Id.run do
  let mut common := #[]
  let first := node.arrivals[0]!
  for index in [:node.captures] do
    let argument := first.arguments[index]!
    unless argument.isFVar do continue
    unless node.arrivals.all (·.arguments[index]! == argument) do continue
    unless enclosing.contains argument.fvarId! do continue
    common := common.push (index, argument.fvarId!)
  return common

private def isArmFact (enclosing : Array FVarId) (decl : Lean.LocalDecl) : Bool :=
  !decl.isLet && !decl.isImplementationDetail && !enclosing.contains decl.fvarId

/-- The propositions over the enclosing context that every arrival holds as a
local of its own: stated once, before the arms. -/
private def commonFacts (node : Node) (context : LocalContext) (enclosing : Array FVarId) :
    MetaM (Array Lean.Expr) := do
  let mut held : Array (Std.HashSet Lean.Expr) := #[]
  for arrival in node.arrivals do
    let mut types : Std.HashSet Lean.Expr := {}
    for decl in arrival.locals do
      if isArmFact enclosing decl then types := types.insert (← instantiateMVars decl.type)
    held := held.push types
  let mut facts := #[]
  for decl in node.arrivals[0]!.locals do
    unless isArmFact enclosing decl do continue
    let type ← instantiateMVars decl.type
    unless (collectFVars {} type).fvarIds.all context.contains do continue
    if facts.contains type || !held.all (·.contains type) then continue
    if ← isProp type then facts := facts.push type
  return facts

/-- An arrival with the part of its arm the seal fixes: the locals the arm
closes, without those stated as common facts that nothing in the arm reads, and
its proofs of the common facts. -/
private def armOf (facts : Array Lean.Expr) (enclosing : Array FVarId)
    (sources : Array EntryArgument) (arrival : Arrival) : MetaM Arrival := do
  let locals := arrival.locals.filter (!enclosing.contains ·.fvarId)
  let mut read : CollectFVars.State := {}
  for source in sources do
    read := collectFVars read (← instantiateMVars (source.select arrival.arguments))
  for decl in locals do
    read := collectFVars read (← instantiateMVars decl.type)
    if let some value := decl.value? then read := collectFVars read (← instantiateMVars value)
  let mut armLocals := #[]
  let mut factProofs : Array (Option Lean.Expr) := facts.map fun _ => none
  for decl in locals do
    let type ← instantiateMVars decl.type
    let index? := if isArmFact enclosing decl then facts.findIdx? (· == type) else none
    if let some index := index? then
      if factProofs[index]!.isNone then factProofs := factProofs.set! index (some decl.toExpr)
      unless read.fvarSet.contains decl.fvarId do continue
    armLocals := armLocals.push decl
  return { arrival with armLocals, factProofs := factProofs.filterMap id }

/-- An arm's equation for a binder, at the binder's type: heterogeneous unless
the types agree reducibly. Default transparency could unfold the carriers of
large struct types here, and an `Eq` stated at a type the leaf sees differently
defeats its substitution. -/
private def binderEquation (binder value : Lean.Expr) : MetaM Lean.Expr := do
  let binderType ← inferType binder
  let valueType ← inferType value
  if binderType == valueType || (← withReducible (isDefEq binderType valueType)) then
    mkEq binder value
  else mkHEq binder value

/-- An edge existentially closes branch-local context and states, per binder of
the summary, the value this arrival gives it. -/
private def edgeRelation (binders : Array Lean.Expr) (sources : Array EntryArgument)
    (arrival : Arrival) : MetaM Lean.Expr := do
  let lctx := arrival.armLocals.foldl (·.addDecl ·) (← getLCtx)
  withLCtx lctx (← getLocalInstances) do
    let mut body := mkConst ``True
    for binder in binders.reverse, source in sources.reverse do
      body := mkAnd (← binderEquation binder (source.select arrival.arguments)) body
    for original in arrival.armLocals.reverse do
      let decl ← original.fvarId.getDecl
      if decl.isLet then
        body ← mkLetFVars #[decl.toExpr] body (generalizeNondepLet := false)
      else
        body ← mkAppM ``Exists #[← mkLambdaFVars #[decl.toExpr] body]
    -- Common parameters stay in the enclosing context: abstracting one
    -- here would change the types of other common locals left free in the
    -- arm (for example an environment depending on the captured row).
    mkLambdaFVars binders body

/-- Keep common dominating locals in the summary's enclosing context.
In particular, callee rules and model metadata still refer to these exact
locals. Only changed captures and the incoming state get new binders. -/
private partial def withCaptures (type : Lean.Expr) (count : Nat)
    (common : Array (Nat × FVarId)) (captures : Array Lean.Expr := #[])
    (act : Array Lean.Expr → Lean.Expr → MetaM α) : MetaM α := do
  if captures.size == count then return ← act captures type
  let .forallE name domain body info ← whnfR type
    | throwError "a shared continuation lost a captured parameter"
  if let some (_, id) := common.find? (·.1 == captures.size) then
    let parameter := mkFVar id
    unless ← isDefEq (← inferType parameter) domain do
      throwError "a dominating join parameter changed its type"
    withCaptures (body.instantiate1 parameter) count common (captures.push parameter) act
  else
    withLocalDecl name info domain fun parameter =>
      withCaptures (body.instantiate1 parameter) count common (captures.push parameter) act

mutual
/-- Preserve the common structure of a state component. A changed field
must not turn every unchanged local, or every known-defined slot, into an
abstract row. An application of one constructor at every arrival is
decomposed field by field (`withConstructorFields`); every varying field
gets its own parameter, and incoming edges supply that field from the
constructors observed here. A position where the arrivals differ in a
constructor is abstracted whole, as is a constructor of an indexed family
whose reconstruction at abstracted fields does not have the expected type. -/
private partial def withEntryArgument (context : LocalContext) (source : EntryArgument)
    (type : Lean.Expr) (incoming : Array Lean.Expr)
    (binders : Array Lean.Expr) (sources : Array EntryArgument)
    (act : Lean.Expr → Array Lean.Expr → Array EntryArgument → MetaM α) : MetaM α := do
  let incoming := incoming.map (·.consumeMData.headBeta)
  let candidate := incoming[0]!
  if !candidate.hasExprMVar && incoming.all (· == candidate) &&
      (collectFVars {} candidate).fvarIds.all context.contains then
    let candidateType ← inferType candidate
    if (collectFVars {} candidateType).fvarIds.all context.contains &&
        (← isDefEq candidateType type) then
      return ← act candidate binders sources
  if let some constructor ← isConstructorAppCore? candidate then
    let arity := constructor.numParams + constructor.numFields
    if incoming.all (·.isAppOfArity constructor.name arity) then
      -- The parameters as the expected type spells them. Inferring them
      -- from a shaped payload exposes its product representation,
      -- preventing reducible simp matching at an elimination over the
      -- nominal carrier.
      let reduced ← whnfD type
      let family ← getConstInfoInduct constructor.induct
      if reduced.isAppOfArity constructor.induct (family.numParams + family.numIndices) then
        let parameters := reduced.getAppArgs.extract 0 constructor.numParams
        let levels := reduced.getAppFn.constLevels!
        let head := mkAppN (mkConst constructor.name levels) parameters
        let telescope ← instantiateForall
          ((ConstantInfo.ctorInfo constructor).instantiateTypeLevelParams levels) parameters
        let saved ← saveState
        try
          return ← withConstructorFields context source telescope constructor.numParams arity
            incoming head type binders sources act
        catch error =>
          let mismatched := match error with
            | .internal id _ => id == mismatchedShape
            | _ => false
          unless mismatched do throw error
          saved.restore
  withLocalDeclD `entry type fun value => act value (binders.push value) (sources.push source)

/-- The fields of a constructor application from `field` on, each shaped at
the domain of the constructor's telescope at the shaped fields before it.
The reconstruction must have the expected type (`mismatchedShape` otherwise,
before the continuation runs): an abstracted field of an indexed family can
change the index. -/
private partial def withConstructorFields (context : LocalContext) (source : EntryArgument)
    (telescope : Lean.Expr) (field arity : Nat) (incoming : Array Lean.Expr) (shaped type : Lean.Expr)
    (binders : Array Lean.Expr) (sources : Array EntryArgument)
    (act : Lean.Expr → Array Lean.Expr → Array EntryArgument → MetaM α) : MetaM α := do
  if field == arity then
    unless ← isDefEq (← inferType shaped) type do
      throw (.internal mismatchedShape)
    return ← act shaped binders sources
  let .forallE _ domain body _ ← whnfD telescope
    | throwError "a constructor of a shared continuation's argument lost a field"
  withEntryArgument context { source with fields := source.fields.push field } domain
    (incoming.map (·.getArg! field)) binders sources fun value binders sources =>
    withConstructorFields context source (body.instantiate1 value) (field + 1) arity incoming
      (mkApp shaped value) type binders sources act
end

/-- Specialize value, locals and memory together, retaining the captured
telescope before their varying fields. The entry predicate still connects
complete states; these parameters only describe their shared shape. -/
private partial def withEntryArguments (context : LocalContext) (parameters : Array Lean.Expr)
    (arrivals : Array Arrival) (index : Nat) (shaped : Array Lean.Expr)
    (binders : Array Lean.Expr) (sources : Array EntryArgument)
    (act : Array Lean.Expr → Array Lean.Expr → Array EntryArgument → MetaM α) : MetaM α := do
  if index == parameters.size then return ← act shaped binders sources
  withEntryArgument context { parameter := index } (← inferType parameters[index]!)
    (arrivals.map (·.arguments[index]!)) binders sources fun value binders sources =>
    withEntryArguments context parameters arrivals (index + 1) (shaped.push value) binders sources act

/-- The arrivals with their arguments over the oldest terms
(`substituteArguments`, except through the `kept` facts) and their contexts
in dependency order, ready to be partitioned. A single arrival is compared
with nothing: it keeps the context of the default walk's exit. -/
def prepareArrivals (node : Node) (kept : FVarIdSet) : MetaM Node := do
  let single := node.arrivals.size < 2
  return { node with arrivals := ← node.arrivals.mapM fun arrival => do
    let arrival ← if single then pure arrival else substituteArguments node kept arrival
    arrival.goal.withContext do
      trace[verex.joinPruning] "{node.name} arrival: memory {arrival.arguments.back!}; equations {
        ← (← getLCtx).foldlM (init := #[]) fun found decl => do
          let type ← instantiateMVars decl.type
          return if decl.isImplementationDetail || !type.isEq then found else found.push type}"
    return { arrival with locals := ← arrival.goal.withContext orderedLocals } }

/-- The arrivals of a class that seals, as the seal closes them: without
saved computations. Facts proved at an arrival before the seal are proved in
this context. An arrival resuming alone keeps them: its suffix reads the
definitions as the default walk's exit does (`unfoldArrival`). -/
def closeArrivals (node : Node) : MetaM Node := do
  return { node with arrivals := ← node.arrivals.mapM fun arrival => do
    let goal ← clearSavedComputations arrival.goal
    return { arrival with goal, locals := ← goal.withContext orderedLocals } }

/-- The arrivals partitioned by `key` (what their shared suffix owes beside
the arguments), in order of first arrival. Arrivals may differ in a constructor:
the summary abstracts that position, its entry relation states the disjunction
of what they bring, and a match or leaf that reads the position opens it. A class
of one shares nothing and resumes as a single predecessor. -/
def partitionArrivals [BEq κ] (key : Arrival → κ) (arrivals : Array Arrival) :
    MetaM (Array (Array Arrival)) := do
  let mut classes : Array (κ × Array Arrival) := #[]
  for arrival in arrivals do
    let shape := key arrival
    match classes.findIdx? (·.1 == shape) with
    | some index => classes := classes.modify index fun (shape, members) => (shape, members.push arrival)
    | none => classes := classes.push (shape, #[arrival])
  return classes.map (·.2)

/-- Seal once every immediate incoming edge is known (`prepareArrivals`
first). Earlier entry relations remain named inside these arms; no path
product is distributed. With a proof per arrival of the stored invariants of
its memory, the summary assumes them of its memory row: a leaf then closes
them over the row as the default walk closes them over the concrete memory,
and the edge proofs carry them inductively through nested joins. -/
def sealNode (node : Node) (invariantProofs : Array Lean.Expr := #[]) (shared : Array Name := #[]) :
    MetaM Node := do
  if node.arrivals.isEmpty then return { node with sealed := true, finished := true }
  unless invariantProofs.isEmpty || invariantProofs.size == node.arrivals.size do
    throwError "a join's stored invariants were not proved at every arrival"
  let enclosingLocals := commonLocals node
  let enclosing := enclosingLocals.map (·.fvarId)
  let common := commonCaptures node enclosing
  let first := node.arrivals[0]!
  let context := enclosingLocals.foldl (·.addDecl ·) ({} : LocalContext)
  let instances := (← first.goal.getDecl).localInstances.filter fun inst =>
    context.contains inst.fvar.fvarId!
  withLCtx context instances do
  let type ← inferType node.continuation
  withCaptures type node.captures common #[] fun captures rest => do
    let rest ← whnfR rest
    let .forallE _ flowType _ _ := rest
      | throwError "a shared continuation does not accept its flow"
    unless flowType.isAppOfArity ``Flow 4 do
      throwError "a shared continuation lost its flow type"
    withLocalDeclD `value (flowType.getArg! 3) fun value => do
    let constructor := mkAppN (mkConst ``Flow.value) (flowType.getAppArgs.push value)
    let .forallE _ envType _ _ ← whnfR (← inferType constructor)
      | throwError "a shared continuation value does not accept its locals"
    withLocalDeclD `locals envType fun locals => do
    let flow := mkApp constructor locals
    let applied := mkAppN node.continuation (captures.push flow)
    let .forallE _ memoryType _ _ ← whnfR (← inferType applied)
      | throwError "a shared continuation does not accept its memory"
    withLocalDeclD `memory memoryType fun memory => do
      let parameters := captures ++ #[value, locals, memory]
      let captureIndices := (List.range node.captures).toArray.filter fun index =>
        !common.any (·.1 == index)
      let captureSources := captureIndices.map fun index => ({ parameter := index } : EntryArgument)
      withEntryArguments context parameters node.arrivals node.captures captures
        (captureIndices.map (parameters[·]!)) captureSources fun shaped binders sources => do
        let facts ← commonFacts node context enclosing
        let arrivals ← node.arrivals.mapM (armOf facts enclosing sources)
        let arms ← arrivals.mapM (edgeRelation binders sources)
        let body := arms.foldr (fun arm rest => mkOr (mkAppN arm binders) rest) (mkConst ``False)
        let body := facts.foldr mkAnd body
        let relation ← defineClosed `_verexJoinEntry
          (← mkLambdaFVars (enclosingLocals.map (·.toExpr) ++ binders) body)
        let entry := mkAppN relation ((enclosingLocals.filter (!·.isLet)).map (·.toExpr) ++ binders)
        let mut conclusion ← mkArrow entry (mkApp applied memory)
        unless invariantProofs.isEmpty do
          let some invariant ← storedInvariantOf? node captures shared
            | throwError "a join proved stored invariants its continuation does not state"
          conclusion ← mkArrow (← storedInvariantStatement invariant memory) conclusion
        let statement ← mkForallFVars binders (conclusion.replaceFVars parameters shaped)
        let summary ← mkFreshExprMVarAt context instances statement .syntheticOpaque
        return { node with
          arrivals := arrivals
          arms := arms
          commonFacts := facts
          enclosing := enclosing
          relation? := some relation
          summary? := some summary.mvarId!
          summaryArguments := sources
          invariantProofs := invariantProofs
          sealed := true }

private def edgeProof (arm : Lean.Expr) (values : Array Lean.Expr) (arrival : Arrival) :
    MetaM Lean.Expr := arrival.goal.withContext do
  let mut type := mkAppN arm values
  let mut witnesses : Array (Lean.Expr × Lean.Expr × Lean.Expr) := #[]
  for decl in arrival.armLocals do
    if decl.isLet then continue
    let type' ← whnfR type
    unless type'.isAppOfArity ``Exists 2 do
      throwError "a join edge lost an existential witness"
    let domain := type'.getArg! 0
    let predicate := type'.getArg! 1
    witnesses := witnesses.push (domain, predicate, decl.toExpr)
    type := (mkApp predicate decl.toExpr).headBeta
  let mut equations := #[]
  for _ in values do
    let conjunction ← whnfR type
    unless conjunction.isAppOfArity ``And 2 do
      throwError "a join edge lost a binder's equation"
    let equation := conjunction.getArg! 0
    equations := equations.push <| ← if equation.isAppOfArity ``Eq 3 then
        mkEqRefl (equation.getArg! 1)
      else mkHEqRefl (equation.getArg! 1)
    type := conjunction.getArg! 1
  let mut proof := mkConst ``True.intro
  for equation in equations.reverse do
    proof ← mkAppM ``And.intro #[equation, proof]
  for (domain, predicate, witness) in witnesses.reverse do
    proof := mkAppN (mkConst ``Exists.intro [← getLevel domain]) #[domain, predicate, witness, proof]
  return proof

/-- The summary already has a dependency-ordered enclosing telescope. Close
it explicitly, retaining local definitions as lets. The auxiliary theorem
helper then only abstracts universes: it need not recheck the entire proof
to discover which local definitions are safe to generalize. The resulting
closed theorem still goes through the kernel's ordinary declaration check. -/
private def certifyProof (locals : Array FVarId) (type proof : Lean.Expr) (kind : Name) : MetaM Lean.Expr := do
  let enclosing := locals.map mkFVar
  let closedType ← mkForallFVars enclosing type (usedLetOnly := false)
  let closedProof ← mkLambdaFVars enclosing proof (usedLetOnly := false)
  if closedType.hasFVar || closedProof.hasFVar then
    throwError "a shared summary has dependencies outside its enclosing context"
  let certificate ← mkAuxTheorem closedType closedProof (zetaDelta := true)
    (kind? := some kind) (cache := false)
  let arguments ← enclosing.filterM fun parameter => do
    return !(← parameter.fvarId!.getDecl).isLet
  return mkAppN certificate arguments

private def certifySummary (node : Node) (type proof : Lean.Expr) : MetaM Lean.Expr :=
  certifyProof node.enclosing type proof `_verexJoinProof

/-- Close the dependencies of a shared proof in telescope order. Definitions
remain definitions, including those required by the proof's expected type. -/
private def sharedProofLocals (type proof : Lean.Expr) : MetaM (Array FVarId) := do
  let ordered ← orderedLocals
  let mut required := collectFVars (collectFVars {} type) proof
  for decl in ordered.reverse do
    if required.fvarSet.contains decl.fvarId then
      required := collectFVars required (← instantiateMVars decl.type)
      if let some value := decl.value? then
        required := collectFVars required (← instantiateMVars value)
  return (ordered.filter (fun decl => required.fvarSet.contains decl.fvarId)).map (·.fvarId)

/-- Mark one proof used by several consumers. The fresh goal keeps its tag
and context; the marker is part of the assignment, so ordinary tactic
rollback removes discarded attempts without a persistent registry. -/
def markSharedProof (goal : MVarId) : MetaM MVarId := goal.withContext do
  let next ← mkFreshExprMVar (← goal.getType) .syntheticOpaque (← goal.getTag)
  goal.assign (mkMData (KVMap.empty.setBool `verexSharedProof true) next)
  return next.mvarId!

def sharedProof? (value : Lean.Expr) : Option Lean.Expr := do
  let .mdata metadata body := value | none
  guard (metadata.getBool `verexSharedProof)
  return body

/-- Name a completed shared proof after its children are finalized. Every
consumer then uses this constant instead of abstracting another body copy. -/
def certifySharedProof (type proof : Lean.Expr) : MetaM Lean.Expr := do
  certifyProof (← sharedProofLocals type proof) type proof `_verexSharedProof

/-- Close predecessor holes only after the shared proof is complete. A
successful summary is a theorem constant, so its proof is retained once.
Reporting admissions propagate to the final artifact audit without entering
the successful theorem cache. -/
def finish? (node : Node) : MetaM (Option Node) := do
  let some summary := node.summary? | return none
  summary.withContext do
  let proof ← instantiateMVars (mkMVar summary)
  if proof.hasExprMVar then return none
  let type ← instantiateMVars (← summary.getType)
  let certificate ← if proof.hasSorry then pure proof
    else certifySummary node type proof
  for arrival in node.arrivals, index in [:node.arrivals.size] do
    arrival.goal.withContext do
      let values := node.summaryArguments.map (·.select arrival.arguments)
      let armTypes := node.arms.map (fun arm => mkAppN arm values)
      let mut tail := mkConst ``False
      for arm in (armTypes.extract (index + 1) armTypes.size).reverse do
        tail := mkOr arm tail
      let mut reached := mkApp3 (mkConst ``Or.inl) armTypes[index]! tail
        (← edgeProof node.arms[index]! values arrival)
      let mut joined := mkOr armTypes[index]! tail
      for arm in (armTypes.extract 0 index).reverse do
        reached := mkApp3 (mkConst ``Or.inr) arm joined reached
        joined := mkOr arm joined
      for fact in node.commonFacts.reverse, factProof in arrival.factProofs.reverse do
        reached := mkApp4 (mkConst ``And.intro) fact joined factProof reached
        joined := mkAnd fact joined
      let hypotheses := (node.invariantProofs[index]?).toArray
      arrival.goal.assign (mkAppN certificate (values ++ hypotheses.push reached))
  return some { node with finished := true, certified := !proof.hasSorry }

/-- Reduce only row projections and local definitions. General `whnf` on an
unrelated equality can execute the preparation certificate for the whole unit.
The feasibility check needs only the constructors already present in a row. -/
private partial def entryValue (expression : Lean.Expr) : MetaM Lean.Expr := do
  let e := expression.consumeMData.headBeta
  if let .letE _ _ value body _ := e then return ← entryValue (body.instantiate1 value)
  if let .fvar id := e then
    if let some value := (← id.getDecl).value? then return ← entryValue value
  if let .proj _ index value := e then
    let value ← entryValue value
    if let some constructor ← isConstructorAppCore? value then
      if let some field := value.getAppArgs[constructor.numParams + index]? then
        return ← entryValue field
  if let .const name _ := e.getAppFn then
    if let some projection ← getProjectionFnInfo? name then
      let arguments := e.getAppArgs
      if let some value := arguments[projection.numParams]? then
        let value ← entryValue value
        if value.isAppOf projection.ctorName then
          if let some field := value.getAppArgs[projection.numParams + projection.i]? then
            return ← entryValue (mkAppN field (arguments.extract (projection.numParams + 1)))
  return e

/-- An incoming row can contradict the suffix's constructor cases. Check
that after linking the row and before expanding earlier entry relations:
distributing those predecessors first multiplies impossible alternatives.
Only constructor conflicts are inspected; no contextual rewriting or theory
search changes a surviving context. -/
private def closeImpossibleEntry (goal : MVarId) : MetaM Bool := goal.withContext do
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    let some (_, lhs, rhs) := type.eq? | continue
    let some left ← isConstructorAppCore? (← entryValue lhs) | continue
    let some right ← isConstructorAppCore? (← entryValue rhs) | continue
    if left.name != right.name then
      goal.assign (← mkNoConfusion (← goal.getType) decl.toExpr)
      return true
  return false

/-- What an arm's equation asks of the opener, read off its two sides. -/
inductive EquationStep where
  /-- The sides are one term. -/
  | reflexive
  /-- A variable the other side does not contain; `symm` when it is the right
  side. As `unifyEq?`, of two variables the younger goes. -/
  | substitute (symm : Bool)
  /-- Two applications of different constructors. -/
  | conflict
  /-- Two applications of one constructor. -/
  | inject
  /-- Anything else stays a hypothesis. -/
  | fact

private def classifyEquation (lhs rhs : Lean.Expr) : MetaM EquationStep := do
  if lhs == rhs then return .reflexive
  let eliminable (side other : Lean.Expr) : MetaM Bool := do
    let .fvar id := side | return false
    if (← id.getDecl).isLet then return false
    return !(← exprDependsOn other id)
  let left ← eliminable lhs rhs
  let right ← eliminable rhs lhs
  if left && right then
    let older := (← lhs.fvarId!.getDecl).index < (← rhs.fvarId!.getDecl).index
    return .substitute older
  if left then return .substitute false
  if right then return .substitute true
  let constructor (side : Lean.Expr) : MetaM (Option ConstructorVal) := do
    let some value ← isConstructorAppCore? side | return none
    return if side.isAppOfArity value.name (value.numParams + value.numFields) then some value
      else none
  let some leftConstructor ← constructor lhs | return .fact
  let some rightConstructor ← constructor rhs | return .fact
  return if leftConstructor.name == rightConstructor.name then .inject else .conflict

/-- `goal` without the hypothesis `id`, where nothing states it. The context is
searched for a mention, not checked through dependent elimination. -/
private def eraseUnused (goal : MVarId) (id : FVarId) : MetaM MVarId := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  if target.containsFVar id then return goal
  let lctx ← getLCtx
  for decl in lctx do
    if decl.fvarId == id then continue
    let type ← instantiateMVars decl.type
    if type.containsFVar id || (decl.value?.any (·.containsFVar id)) then return goal
  let next ← mkFreshExprMVarAt (lctx.erase id) (← getLocalInstances) target .syntheticOpaque
    (← goal.getTag)
  goal.assign next
  return next.mvarId!

/-- The two cases of `h : a ∨ b`, each with its side introduced. -/
private def eliminateOr (goal : MVarId) (h type : Lean.Expr) :
    MetaM ((MVarId × Lean.Expr) × (MVarId × Lean.Expr)) := goal.withContext do
  let target ← goal.getType
  let left := type.getArg! 0
  let right := type.getArg! 1
  let tag ← goal.getTag
  let leftCase ← mkFreshExprSyntheticOpaqueMVar (← mkArrow left target) tag
  let rightCase ← mkFreshExprSyntheticOpaqueMVar (← mkArrow right target) tag
  goal.assign (mkAppN (mkConst ``Or.elim) #[left, right, target, h, leftCase, rightCase])
  let introduce (case : Lean.Expr) := do
    let (id, next) ← (← eraseUnused case.mvarId! h.fvarId!).intro1
    return (next, mkFVar id)
  return (← introduce leftCase, ← introduce rightCase)

/-- A conjunction or existential opened by one term into one goal under its
witnesses and conjuncts, whatever its depth: `Exists.elim` per witness and
`And.left`/`And.right` per conjunct, with the goal's continuation stated once
over all of them and introduced at once. With the conjuncts to read on, in
order: all of them but a disjunction on the left of a conjunction, which is a
fact stated beside the arms and not a choice. -/
private partial def openConjunction (goal : MVarId) (h type : Lean.Expr) :
    MetaM (MVarId × Array Lean.Expr) := goal.withContext do
  let target ← goal.getType
  let tag ← goal.getTag
  let base ← eraseUnused goal h.fvarId!
  let (lctx, instances) ← base.withContext do return (← getLCtx, ← getLocalInstances)
  let opened ← IO.mkRef (#[] : Array Bool)
  let continuation ← IO.mkRef (none : Option MVarId)
  let rec go (items : List (Lean.Expr × Lean.Expr × Bool)) (binders arguments : Array Lean.Expr) :
      MetaM Lean.Expr := do
    match items with
    | [] =>
        let statement ← mkForallFVars binders target
        let next ← mkFreshExprMVarAt lctx instances statement .syntheticOpaque tag
        continuation.set next.mvarId!
        return mkAppN next arguments
    | (proof, stated, read) :: rest =>
        let stated ← whnfR stated
        if stated.isConstOf ``True then return ← go rest binders arguments
        if stated.isAppOfArity ``And 2 then
          let left := stated.getArg! 0
          let right := stated.getArg! 1
          let leftProof := mkApp3 (mkConst ``And.left) left right proof
          let rightProof := mkApp3 (mkConst ``And.right) left right proof
          let keepsLeft := !(← whnfR left).isAppOfArity ``Or 2
          return ← go ((leftProof, left, keepsLeft) :: (rightProof, right, read) :: rest) binders arguments
        if stated.isAppOfArity ``Exists 2 then
          let domain := stated.getArg! 0
          let predicate := stated.getArg! 1
          let level ← getLevel domain
          return ← withLocalDeclD (← mkFreshUserName `w) domain fun witness => do
            let body := (mkApp predicate witness).headBeta
            withLocalDeclD (← mkFreshUserName `h) body fun held => do
              opened.modify (·.push false)
              let inner ← go ((held, body, read) :: rest) (binders.push witness) (arguments.push witness)
              return mkApp5 (mkConst ``Exists.elim [level]) domain predicate target proof
                (← mkLambdaFVars #[witness, held] inner)
        withLocalDeclD (← mkFreshUserName `h) stated fun fact => do
          opened.modify (·.push read)
          go rest (binders.push fact) (arguments.push proof)
  let term ← go [(h, type, true)] #[] #[]
  goal.assign term
  let some next ← continuation.get | throwError "an opened conjunction lost its continuation"
  let (introduced, next) ← next.introNP (← opened.get).size
  let reads ← opened.get
  return (next, ((introduced.zip reads).filter (·.2)).map (mkFVar ·.1))

/-- What a caller of `exposeEntries` observes of it: its parts, each run through
`timed` under its label, and each entry relation opened. -/
structure ExposeObserver where
  timed : {α : Type} → String → MetaM α → MetaM α := fun _ step => step
  woke : MetaM Unit := pure ()
  keptEquation : MVarId → Lean.Expr → MetaM Unit := fun _ _ => pure ()

/-- Open entry alternatives only after program traversal, inside the
automatic leaf's allowance. Every case retains a proof destination, and
constructor contradictions close before earlier predecessors are expanded.
The caller may also close a linked goal without reading earlier entries;
that decision consumes the same enclosing allowance as the decomposition. -/
def exposeEntries (goal : MVarId) (relations : Array Name)
    (close : MVarId → MetaM Bool := fun _ => pure false)
    (observer : ExposeObserver := {}) : MetaM (List MVarId) := do
  if relations.isEmpty then return [goal]
  let close goal := observer.timed "expose/frame" (close goal)
  if ← close goal then return []
  let findEntries (goal : MVarId) := goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      let type ← instantiateMVars decl.type
      return if (type.getAppFn.constName?).any relations.contains then found.push decl.toExpr else found
  let mut pending := #[(goal, ← findEntries goal)]
  let mut leaves := []
  while let some (goal, todo) := pending.back? do
    pending := pending.pop
    if ← goal.isAssigned then continue
    let some hypothesis := todo.back? | do
      -- Link this edge's parameters before opening earlier entry predicates
      -- introduced as existential witnesses. Their correlations stay intact.
      if ← observer.timed "expose/refute" (closeImpossibleEntry goal) then continue
      if ← close goal then continue
      let earlier ← findEntries goal
      if earlier.isEmpty then leaves := goal :: leaves
      else pending := pending.push (goal, earlier)
      continue
    let todo := todo.pop
    unless hypothesis.isFVar && (← goal.getDecl).lctx.contains hypothesis.fvarId! do
      pending := pending.push (goal, todo)
      continue
    let (goal, type) ← goal.withContext do
      let mut type ← whnfR (← instantiateMVars (← inferType hypothesis))
      let mut goal := goal
      if (type.getAppFn.constName?).any relations.contains then
        -- Unfold this node only. Earlier entries can occur in witness types;
        -- keeping their names lets us open them after this edge is linked.
        let name := type.getAppFn.constName!
        observer.woke
        type ← deltaExpand type (· == name)
        goal ← goal.replaceLocalDeclDefEq hypothesis.fvarId! type
      return (goal, ← whnfR type)
    -- A homogeneous HEq is an ordinary equality; a dependent one stays a fact.
    let (goal, hypothesis, type) ← if type.isAppOf ``HEq then do
        let (id, next) ← heqToEq goal hypothesis.fvarId!
        let type ← next.withContext (inferType (mkFVar id))
        pure (next, mkFVar id, type)
      else pure (goal, hypothesis, type)
    if let some (domain, lhs, rhs) := type.eq? then
      let (goal, step) ← goal.withContext do
        let lhs ← whnfR (← instantiateMVars lhs)
        let rhs ← whnfR (← instantiateMVars rhs)
        let goal ← goal.replaceLocalDeclDefEq hypothesis.fvarId!
          (mkApp3 (mkConst ``Eq type.getAppFn.constLevels!) domain lhs rhs)
        return (goal, ← goal.withContext (classifyEquation lhs rhs))
      match step with
      | .reflexive =>
          let goal ← observer.timed "expose/cases" (goal.tryClear hypothesis.fvarId!)
          pending := pending.push (goal, todo)
      | .substitute symm =>
          let (subst, goal) ← observer.timed "expose/cases"
            (substCore goal hypothesis.fvarId! (symm := symm) (tryToSkip := true))
          pending := pending.push (goal, todo.map subst.apply)
      | .conflict =>
          observer.timed "expose/refute" do
            goal.withContext do
              goal.assign (← mkNoConfusion (← goal.getType) hypothesis)
      | .inject =>
          match ← observer.timed "expose/injection" (injection goal hypothesis.fvarId!) with
          | .solved => pure ()
          | .subgoal next fields _ =>
              pending := pending.push (next, todo ++ fields.reverse.map mkFVar)
      | .fact =>
          observer.keptEquation goal (mkApp3 (mkConst ``Eq type.getAppFn.constLevels!) domain lhs rhs)
          pending := pending.push (goal, todo)
      continue
    if type.isConstOf ``False then
      observer.timed "expose/arms" do
        goal.withContext do goal.assign (← mkFalseElim (← goal.getType) hypothesis)
    else if type.isConstOf ``True then
      pending := pending.push (← eraseUnused goal hypothesis.fvarId!, todo)
    else if type.isAppOfArity ``Or 2 then
      let (left, right) ← observer.timed "expose/arms" (eliminateOr goal hypothesis type)
      pending := pending.push (right.1, todo.push right.2)
      pending := pending.push (left.1, todo.push left.2)
    else if type.isAppOf ``And || type.isAppOf ``Exists then
      let (next, opened) ← observer.timed "expose/arms" (openConjunction goal hypothesis type)
      pending := pending.push (next, todo ++ opened.reverse)
    else
      pending := pending.push (goal, todo)
  return leaves.reverse

end VerexIR.Proofs.Denote.SharedContinuations
