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
  if action.isAppOfArity ``Flow.bind 9 then
    return min 2 ((← normalExits (action.getArg! 7) true) *
      (← normalExits (action.getArg! 8) flow))
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

structure Arrival where
  goal : MVarId
  arguments : Array Lean.Expr
  locals : Array Lean.LocalDecl
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
  common : Array (Nat × FVarId) := #[]
  enclosing : Array FVarId := #[]
  relation? : Option Lean.Expr := none
  summary? : Option MVarId := none
  summaryArguments : Array EntryArgument := #[]
  sealed : Bool := false
  finished : Bool := false
  /-- Only a closed proof without reporting admissions is certified. -/
  certified : Bool := false
  deriving Inhabited

def Node.name (node : Node) : Name := node.continuation.getAppFn.constName!

def unfold (node : Node) (goal : MVarId) : MetaM MVarId := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  withTransparency .all <| goal.replaceTargetDefEq (← deltaExpand target (· == node.name))

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

private def defineClosed (kind : Name) (value : Lean.Expr) : MetaM Lean.Expr := do
  unless !value.hasFVar && !value.hasExprMVar do
    throwError "a shared continuation definition has unresolved dependencies"
  let name ← mkAuxDeclName (kind := kind)
  let defined ← mkAuxDefinitionFor name value (compile := false)
  setReducibilityStatus name .irreducible
  return defined

/-- Close the postcondition before its action branches. The capture
arguments remain visible to substitution; the continuation body stays named. -/
def fold? (goal : MVarId) : MetaM (Option (MVarId × Node)) := goal.withContext do
  let target := (← instantiateMVars (← goal.getType)).consumeMData
  unless target.isAppOfArity ``wp 7 do return none
  let resultType ← whnfR (target.getArg! 2)
  unless resultType.isAppOfArity ``Flow 6 do return none
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
  let captures := (locals.filter (!·.isLet)).map (·.toExpr)
  -- `isLet` excludes nondependent lets (for example facts introduced by
  -- `have`): their values are hidden and they are explicit capture arguments.
  -- Generalize those lets while retaining ordinary let-bound observations.
  let body ← mkLambdaFVars (locals.map (·.toExpr)) ensures (generalizeNondepLet := true)
  let continuation ← defineClosed `_verexContinuation body
  let post := mkAppN continuation captures
  let folded ← withTransparency .all <| goal.replaceTargetDefEq
    (mkAppN target.getAppFn (target.getAppArgs.set! 4 post))
  return some (folded, { continuation, captures := captures.size })

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

/-- Abrupt flows keep their original destinations; only a value carries
locals into the shared suffix. -/
def arrival? (node : Node) (goal : MVarId) : MetaM (Option Arrival) := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOf node.name do return none
  let arguments := target.getAppArgs
  unless arguments.size == node.captures + 2 do return none
  let flow := arguments[node.captures]!.consumeMData.headBeta
  unless flow.isAppOfArity ``Flow.value 8 do return none
  let goal ← clearSavedComputations goal
  return some {
    goal
    arguments := arguments.extract 0 node.captures ++
      #[flow.getArg! 6, flow.getArg! 7, arguments.back!]
    locals := ← goal.withContext orderedLocals }

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

private def varyingArguments (arguments : Array Lean.Expr) (common : Array (Nat × FVarId)) :
    Array Lean.Expr :=
  ((List.range arguments.size).toArray.filter fun index => !common.any (·.1 == index)).map
    (arguments[·]!)

/-- An edge existentially closes branch-local context. Common dominating
captures use the summary parameters directly. Heterogeneous equalities retain
correlations between all other captures, values, locals and memory. -/
private def edgeRelation (parameters : Array Lean.Expr) (common : Array (Nat × FVarId))
    (enclosing : Array FVarId) (arrival : Arrival) : MetaM Lean.Expr := do
  let originals := common.map (mkFVar ·.2)
  let shared := common.map (parameters[·.1]!)
  let replace (value : Lean.Expr) := value.replaceFVars originals shared
  let locals := arrival.locals.filter (!enclosing.contains ·.fvarId)
  let lctx := locals.foldl (fun ctx decl =>
    let decl := decl.setType (replace decl.type)
    let decl := decl.value?.map (fun value => decl.setValue (replace value)) |>.getD decl
    ctx.addDecl decl) (← getLCtx)
  withLCtx lctx (← getLocalInstances) do
    let mut body := mkConst ``True
    for index in (List.range parameters.size).reverse do
      if common.any (·.1 == index) then continue
      body := mkAnd (← mkHEq parameters[index]! (replace arrival.arguments[index]!)) body
    for original in locals.reverse do
      let decl ← original.fvarId.getDecl
      if decl.isLet then
        body ← mkLetFVars #[decl.toExpr] body (generalizeNondepLet := false)
      else
        body ← mkAppM ``Exists #[← mkLambdaFVars #[decl.toExpr] body]
    -- Common parameters stay in the enclosing context: abstracting one
    -- here would change the types of other common locals left free in the
    -- arm (for example an environment depending on the captured row).
    mkLambdaFVars (varyingArguments parameters common) body

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

/-- Preserve the common structure of a state component. A changed field
must not turn every unchanged local, or every known-defined slot, into an
abstract row. Only pairs and present slots are decomposed; their field types
are nondependent. Every varying field gets its own parameter, and incoming
edges supply that field from the constructors observed here. -/
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
  if incoming.all (·.isAppOfArity ``Prod.mk 4) then
    let reduced ← whnfD type
    if reduced.isAppOfArity ``Prod 2 then
      return ← withEntryArgument context { source with fields := source.fields.push 2 }
        (reduced.getArg! 0) (incoming.map (·.getArg! 2)) binders sources fun left binders sources =>
        withEntryArgument context { source with fields := source.fields.push 3 }
          (reduced.getArg! 1) (incoming.map (·.getArg! 3)) binders sources fun right binders sources => do
          let constructor := mkConst ``Prod.mk reduced.getAppFn.constLevels!
          act (mkAppN constructor (reduced.getAppArgs ++ #[left, right])) binders sources
  if incoming.all (·.isAppOfArity ``Option.some 2) then
    let reduced ← whnfD type
    if reduced.isAppOfArity ``Option 1 then
      return ← withEntryArgument context { source with fields := source.fields.push 1 }
        (reduced.getArg! 0) (incoming.map (·.getArg! 1)) binders sources fun value binders sources => do
        -- Retain the expected carrier spelling. Inferring it from a shaped
        -- payload exposes its product representation, preventing reducible
        -- simp matching at an elimination over the nominal carrier.
        let constructor := mkConst ``Option.some reduced.getAppFn.constLevels!
        act (mkApp2 constructor (reduced.getArg! 0) value) binders sources
  withLocalDeclD `entry type fun value => act value (binders.push value) (sources.push source)

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

/-- Seal once every immediate incoming edge is known. Earlier entry
relations remain named inside these arms; no path product is distributed. -/
def sealNode (node : Node) : MetaM Node := do
  if node.arrivals.isEmpty then return { node with sealed := true, finished := true }
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
    unless flowType.isAppOfArity ``Flow 6 do
      throwError "a shared continuation lost its flow type"
    withLocalDeclD `value (flowType.getArg! 5) fun value => do
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
      let arms ← node.arrivals.mapM (edgeRelation parameters common enclosing)
      let dynamic := varyingArguments parameters common
      let body := arms.foldr (fun arm rest => mkOr (mkAppN arm dynamic) rest) (mkConst ``False)
      let relation ← defineClosed `_verexJoinEntry
        (← mkLambdaFVars (enclosingLocals.map (·.toExpr) ++ dynamic) body)
      let entry := mkAppN relation ((enclosingLocals.filter (!·.isLet)).map (·.toExpr) ++ dynamic)
      let captureIndices := (List.range node.captures).toArray.filter fun index =>
        !common.any (·.1 == index)
      let captureSources := captureIndices.map fun index => ({ parameter := index } : EntryArgument)
      withEntryArguments context parameters node.arrivals node.captures captures
        (captureIndices.map (parameters[·]!)) captureSources fun shaped binders sources => do
        let statement ← mkForallFVars binders
          ((← mkArrow entry (mkApp applied memory)).replaceFVars parameters shaped)
        let summary ← mkFreshExprMVarAt context instances statement .syntheticOpaque
        return { node with
          arms := arms
          common := common
          enclosing := enclosing
          relation? := some relation
          summary? := some summary.mvarId!
          summaryArguments := sources
          sealed := true }

private def edgeProof (arm : Lean.Expr) (common : Array (Nat × FVarId))
    (enclosing : Array FVarId) (arrival : Arrival) : MetaM Lean.Expr := arrival.goal.withContext do
  let mut type := mkAppN arm (varyingArguments arrival.arguments common)
  let mut witnesses : Array (Lean.Expr × Lean.Expr × Lean.Expr) := #[]
  for decl in arrival.locals do
    if decl.isLet || enclosing.contains decl.fvarId then continue
    let type' ← whnfR type
    unless type'.isAppOfArity ``Exists 2 do
      throwError "a join edge lost an existential witness"
    let domain := type'.getArg! 0
    let predicate := type'.getArg! 1
    witnesses := witnesses.push (domain, predicate, decl.toExpr)
    type := (mkApp predicate decl.toExpr).headBeta
  let mut proof := mkConst ``True.intro
  for index in (List.range arrival.arguments.size).reverse do
    if common.any (·.1 == index) then continue
    proof ← mkAppM ``And.intro #[← mkHEqRefl arrival.arguments[index]!, proof]
  for (domain, predicate, witness) in witnesses.reverse do
    proof := mkAppN (mkConst ``Exists.intro [← getLevel domain]) #[domain, predicate, witness, proof]
  return proof

/-- The summary already has a dependency-ordered enclosing telescope. Close
it explicitly, retaining local definitions as lets. The auxiliary theorem
helper then only abstracts universes: it need not recheck the entire proof
to discover which local definitions are safe to generalize. The resulting
closed theorem still goes through the kernel's ordinary declaration check. -/
private def certifySummary (node : Node) (type proof : Lean.Expr) : MetaM Lean.Expr := do
  let enclosing := node.enclosing.map mkFVar
  let closedType ← mkForallFVars enclosing type (usedLetOnly := false)
  let closedProof ← mkLambdaFVars enclosing proof (usedLetOnly := false)
  if closedType.hasFVar || closedProof.hasFVar then
    throwError "a shared summary has dependencies outside its enclosing context"
  let certificate ← mkAuxTheorem closedType closedProof (zetaDelta := true)
    (kind? := some `_verexJoinProof) (cache := false)
  let arguments ← enclosing.filterM fun parameter => do
    return !(← parameter.fvarId!.getDecl).isLet
  return mkAppN certificate arguments

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
      let armTypes := node.arms.map (fun arm => mkAppN arm (varyingArguments arrival.arguments node.common))
      let mut tail := mkConst ``False
      for arm in (armTypes.extract (index + 1) armTypes.size).reverse do
        tail := mkOr arm tail
      let mut reached := mkApp3 (mkConst ``Or.inl) armTypes[index]! tail
        (← edgeProof node.arms[index]! node.common node.enclosing arrival)
      let mut joined := mkOr armTypes[index]! tail
      for arm in (armTypes.extract 0 index).reverse do
        reached := mkApp3 (mkConst ``Or.inr) arm joined reached
        joined := mkOr arm joined
      arrival.goal.assign (mkApp (mkAppN certificate (node.summaryArguments.map (·.select arrival.arguments))) reached)
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

/-- Open entry alternatives only after program traversal, inside the
automatic leaf's allowance. Every case retains a proof destination, and
constructor contradictions close before earlier predecessors are expanded.
The caller may also close a linked goal without reading earlier entries;
that decision consumes the same enclosing allowance as the decomposition. -/
def exposeEntries (goal : MVarId) (relations : Array Name)
    (close : MVarId → MetaM Bool := fun _ => pure false) : MetaM (List MVarId) := do
  if relations.isEmpty then return [goal]
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
      if ← closeImpossibleEntry goal then continue
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
        type ← deltaExpand type (· == name)
        goal ← goal.replaceLocalDeclDefEq hypothesis.fvarId! type
      return (goal, ← whnfR type)
    -- Ordinary equalities avoid generalizing a fresh type index for each
    -- homogeneous HEq. Dependent equalities still use checked HEq cases.
    let (goal, hypothesis, type) ← if type.isAppOf ``HEq then do
        let (id, next) ← heqToEq goal hypothesis.fvarId!
        let type ← next.withContext (inferType (mkFVar id))
        pure (next, mkFVar id, type)
      else pure (goal, hypothesis, type)
    if let some (_, lhs, rhs) := type.eq? then
      -- Ordinary row substitution is substantially cheaper than a chain
      -- of injections when dependent elimination can solve every field.
      let saved ← saveState
      let alternatives? ← try
          some <$> goal.withContext (goal.cases hypothesis.fvarId!)
        catch error => do
          saved.restore
          if error.isMaxHeartbeat then throw error
          pure none
      if let some alternatives := alternatives? then
        for child in alternatives do
          pending := pending.push (child.mvarId, todo.map child.subst.apply ++ child.fields.reverse)
        continue
      let lhs := lhs.consumeMData.headBeta
      let rhs := rhs.consumeMData.headBeta
      -- Shaped rows can equate opaque computations inside their fields.
      -- Decompose those rows one constructor at a time, retaining equations
      -- that dependent elimination could not solve for ordinary reasoning.
      if (lhs.isAppOf ``Prod.mk && rhs.isAppOf ``Prod.mk) ||
          (lhs.isAppOf ``Option.some && rhs.isAppOf ``Option.some) then
        match ← injection goal hypothesis.fvarId! with
        | .solved => pure ()
        | .subgoal next fields _ =>
            pending := pending.push (next, todo ++ fields.reverse.map mkFVar)
      else
        pending := pending.push (goal, todo)
      continue
    if type.isAppOf ``And || type.isAppOf ``Or || type.isAppOf ``Exists ||
        type.isAppOf ``HEq || type.isAppOf ``Eq ||
        type.isConstOf ``False || type.isConstOf ``True then
      let cases ← goal.withContext do
        try goal.cases hypothesis.fvarId!
        catch error => throwError "opening shared entry {hypothesis} : {type}\n{error.toMessageData}\n{goal}"
      for child in cases do
        let remaining := todo.map child.subst.apply
        let fields := if type.isAppOf ``Exists then child.fields.extract 1 child.fields.size
          else child.fields
        pending := pending.push (child.mvarId, remaining ++ fields.reverse)
    else
      pending := pending.push (goal, todo)
  if (← getOptions).getBool `verex.solverDebug false then
    for leaf in leaves do
      IO.println s!"join alternative:\n{← ppGoal leaf}"
  return leaves.reverse

end VerexIR.Proofs.Denote.SharedContinuations
