-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Import.Raw
import LeanerIR.Validation.Diagnostic
import LeanerIR.Validation.Validated

/-!
# Reducible CFG structurization

Raw graph bodies are an import form, not a backend language.  This pass finds
natural loops from dominance backedges and recursively builds the common LIR
tree language.  Edges to active headers/continuation exits become
`continue`/`break`; exits which immediately return or raise remain structured
terminal branches inside the loop. Acyclic diamonds are joined at their
nearest common reachable block.

The implementation intentionally rejects graphs for which those regions are
not unique.  It never leaves a `goto` in validated LIR and never falls back to
a program-counter state machine.
-/

namespace LeanerIR.Import.Structurize

open LeanerIR.Validation

private def successors (graph : RawCfg) (block : Nat) : Array Nat :=
  match graph.blocks[block]? with
  | none => #[]
  | some block => match block.terminator with
      | .goto target => #[target.index]
      | .branch _ thenTarget elseTarget => #[thenTarget.index, elseTarget.index]
      | .switch _ cases defaultTarget => cases.map (·.2.index) |>.push defaultTarget.index
      | .call _ destination unwind =>
          destination.toArray.map (·.target.index) ++ match unwind with
            | .cleanup target => #[target.index]
            | _ => #[]
      | .drop _ target unwind | .assert _ _ _ target unwind =>
          #[target.index] ++ match unwind with
            | .cleanup cleanup => #[cleanup.index]
            | _ => #[]
      | .return_ _ | .throw_ _ _ | .unreachable | .resume | .abort => #[]

private def predecessors (graph : RawCfg) : Array (Array Nat) := Id.run do
  let mut result := Array.replicate graph.blocks.size #[]
  for source in Array.range graph.blocks.size do
    for target in successors graph source do
      if target < result.size then
        result := result.modify target (·.push source)
  return result

private def intersectAll (allNodes : Array Nat) (sets : Array (Array Nat)) : Array Nat :=
  if sets.isEmpty then #[]
  else allNodes.filter fun candidate => sets.all (·.contains candidate)

/-- A deliberately small fixed-point dominator computation.  Imported
functions are small enough that determinism and auditability matter more than
an asymptotically clever implementation here. -/
private def dominators (graph : RawCfg) : Array (Array Nat) := Id.run do
  let count := graph.blocks.size
  let allNodes := Array.range count
  let preds := predecessors graph
  let mut result := allNodes.map fun index =>
    if index == graph.entry.index then #[index] else allNodes
  for _ in Array.range (count * count + 1) do
    let mut changed := false
    let mut next := result
    for index in allNodes do
      if index == graph.entry.index then continue
      let inherited := intersectAll allNodes <| preds[index]!.filterMap fun predecessor =>
        result[predecessor]?
      let value := (inherited.filter (· != index)).push index
      if value != result[index]! then
        changed := true
        next := next.set! index value
    result := next
    unless changed do break
  return result

private structure LoopInfo where
  header : Nat
  nodes : Array Nat
  exit : Option Nat
  deriving Inhabited

private def naturalLoop (preds : Array (Array Nat)) (header latch : Nat) : Array Nat := Id.run do
  let mut nodes := #[header]
  if latch != header then nodes := nodes.push latch
  let mut work := if latch == header then #[] else #[latch]
  let mut cursor := 0
  while cursor < work.size do
    let current := work[cursor]!
    cursor := cursor + 1
    for predecessor in preds[current]! do
      unless nodes.contains predecessor do
        nodes := nodes.push predecessor
        if predecessor != header then work := work.push predecessor
  return nodes

private def directlyTerminates (graph : RawCfg) (block : Nat) : Bool :=
  match graph.blocks[block]? with
  | some { terminator := .return_ .., .. }
  | some { terminator := .throw_ .., .. }
  | some { terminator := .abort, .. } => true
  | _ => false

private def directlyUnreachable (graph : RawCfg) (block : Nat) : Bool :=
  match graph.blocks[block]? with
  | some { terminator := .unreachable, .. } => true
  | _ => false

/-- MIR administration which carries source/compiler lifetime information but
has no executable effect in the initial safe profile. Destructive state and
provenance statements remain explicit rejection boundaries. -/
private def inertStatement : RawStatement → Bool
  | .storageLive _ | .storageDead _ | .placeMention _ | .ascribeUserType _ _ => true
  | .execute _ | .deinit _ | .setDiscriminant _ _ | .retag _ | .profile _ => false

/-- Assertions from ordinary safe Rust whose message is unobservable under
`panic=abort`. Pointer-alignment and extension assertions remain behind their
own semantic-profile gates. -/
private def panicAssertKind : RawAssertKind → Bool
  | .boundsCheck | .overflow | .divisionByZero | .remainderByZero => true
  | .misalignedPointerDereference | .profile _ => false

private def loopInfos (graph : RawCfg) : Except Diagnostic (Array LoopInfo) := do
  let dom := dominators graph
  let preds := predecessors graph
  let mut headers : Array (Nat × Array Nat) := #[]
  for source in Array.range graph.blocks.size do
    for target in successors graph source do
      if (dom[source]?).any (·.contains target) then
        let nodes := naturalLoop preds target source
        match headers.findIdx? (·.1 == target) with
        | some index =>
            headers := headers.modify index fun (header, previous) =>
              (header, nodes.foldl (fun all node =>
                if all.contains node then all else all.push node) previous)
        | none => headers := headers.push (target, nodes)
  let mut loops := #[]
  for (header, nodes) in headers do
    let mut exits := #[]
    for node in nodes do
      for target in successors graph node do
        if !nodes.contains target && !exits.contains target then exits := exits.push target
    -- A return/exceptional block is not a continuation choice: it can stay in
    -- the loop tree as that branch's terminator. Only distinct nonterminal
    -- targets require a post-loop dispatch, which this structured language
    -- intentionally does not synthesize.
    let continuationExits := exits.filter (!directlyTerminates graph ·)
    if continuationExits.size > 1 then
      let loc := graph.blocks[header]?.map (·.loc)
      throw <| .error "LIR-CFG-MULTI-EXIT"
        s!"natural loop at block {header} has {continuationExits.size} distinct continuation exits" loc
    loops := loops.push { header, nodes, exit := continuationExits[0]? }
  return loops

private structure ActiveLoop where
  header : Nat
  exit : Option Nat

private structure BuildContext where
  graph : RawCfg
  loops : Array LoopInfo
  originalBlocks : Array BlockId
  resultTypeId : TypeId
  unitTypeId : Option TypeId

private structure BuildState where
  expressions : Array Expr
  patterns : Array Pattern
  blockExpressions : Array (BlockId × ExprId) := #[]
  regions : Array StructurizedRegion := #[]

private abbrev BuildM := ReaderT BuildContext (StateT BuildState (Except Diagnostic))

private def originalBlock (index : Nat) : BuildM BlockId := do
  let context ← read
  let some block := context.originalBlocks[index]?
    | throw <| .error "LIR-CFG-WITNESS" s!"missing original identity for block {index}"
  pure block

private def recordBlock (block : Nat) (expression : ExprId) : BuildM Unit := do
  let original ← originalBlock block
  let state ← get
  unless state.blockExpressions.contains (original, expression) do
    set { state with blockExpressions := state.blockExpressions.push (original, expression) }

private def recordRegion (kind : StructurizedRegionKind) (header : Nat)
    (expression : ExprId) (members : Array Nat := #[]) (exit : Option Nat := none) : BuildM Unit := do
  let originalHeader ← originalBlock header
  let originalMembers ← members.mapM originalBlock
  let originalExit ← exit.mapM originalBlock
  modify fun state => { state with regions := state.regions.push {
    kind, header := originalHeader, expression,
    members := originalMembers, exit := originalExit } }

private def addTypedExpr (loc : LocId) (typeId : TypeId) (kind : ExprKind) : BuildM ExprId := do
  let state ← get
  let id : ExprId := ⟨state.expressions.size⟩
  let expr : Expr := { loc, typeId, kind }
  set { state with expressions := state.expressions.push expr }
  return id

private def addExpr (loc : LocId) (kind : ExprKind) : BuildM ExprId := do
  addTypedExpr loc (← read).resultTypeId kind

private def addUnitExpr (loc : LocId) (kind : ExprKind) : BuildM ExprId := do
  let context ← read
  let some typeId := context.unitTypeId
    | throw <| .at "LIR-CFG-UNIT-TYPE"
        "structurizing this CFG requires Unit in the compilation-unit type table" loc
  addTypedExpr loc typeId kind

private def addPattern (loc : LocId) (typeId : TypeId) (kind : PatternKind) :
    BuildM PatternId := do
  let state ← get
  let id : PatternId := ⟨state.patterns.size⟩
  let pattern : Pattern := { loc, typeId, kind }
  set { state with patterns := state.patterns.push pattern }
  return id

private def empty (loc : LocId) : BuildM ExprId := addUnitExpr loc (.block #[] none)

private def sequence (loc : LocId) (statements : Array ExprId) (tail : Option ExprId := none) : BuildM ExprId :=
  if statements.isEmpty then match tail with
    | some tail => pure tail
    | none => empty loc
  else do
    let typeId ← match tail with
      | some tail => do
          let state ← get
          let some expression := state.expressions[tail.index]?
            | throw <| .at "LIR-ID-BOUNDS"
                s!"invalid structured sequence tail {tail.index}" loc
          pure expression.typeId
      | none =>
          let some typeId := (← read).unitTypeId
            | throw <| .at "LIR-CFG-UNIT-TYPE"
                "structurizing this CFG requires Unit in the compilation-unit type table" loc
          pure typeId
    addTypedExpr loc typeId (.block statements tail)

private def distances (graph : RawCfg) (start : Nat) (blocked : Array Nat) : Array (Option Nat) := Id.run do
  let mut result := Array.replicate graph.blocks.size none
  if start >= graph.blocks.size then return result
  result := result.set! start (some 0)
  let mut work := #[start]
  let mut cursor := 0
  while cursor < work.size do
    let current := work[cursor]!
    cursor := cursor + 1
    let distance := result[current]!.getD 0
    for target in successors graph current do
      if target < result.size && !blocked.contains target && result[target]!.isNone then
        result := result.set! target (some (distance + 1))
        work := work.push target
  return result

/-- Remove blocks which cannot be reached from the graph entry and rewrite
block IDs densely. Frontends commonly receive such blocks from ordinary
compiler cleanup after a return or abort; they have no semantic contribution
to the imported function and must not make an otherwise structured body
unprintable. -/
private def pruneUnreachable (graph : RawCfg) : RawCfg × Array BlockId := Id.run do
  let reachable := distances graph graph.entry.index #[]
  let mut remap : Array (Option Nat) := Array.replicate graph.blocks.size none
  let mut originals : Array BlockId := #[]
  let mut next := 0
  for old in Array.range graph.blocks.size do
    if reachable[old]!.isSome then
      remap := remap.set! old (some next)
      originals := originals.push ⟨old⟩
      next := next + 1
  let mapId (id : BlockId) : BlockId := ⟨remap[id.index]!.get!⟩
  let blocks := graph.blocks.zipIdx.filterMap fun (block, old) =>
    if reachable[old]!.isNone then none else
    let terminator := match block.terminator with
      | .goto target => RawTerminator.goto (mapId target)
      | .branch condition thenTarget elseTarget =>
          .branch condition (mapId thenTarget) (mapId elseTarget)
      | .switch scrutinee cases defaultTarget =>
          .switch scrutinee (cases.map fun (value, target) => (value, mapId target))
            (mapId defaultTarget)
      | .call call destination unwind => .call call
          (destination.map fun value => { value with target := mapId value.target })
          (match unwind with | .cleanup target => .cleanup (mapId target) | value => value)
      | .drop place target unwind => .drop place (mapId target)
          (match unwind with | .cleanup cleanup => .cleanup (mapId cleanup) | value => value)
      | .assert condition expected kind target unwind =>
          .assert condition expected kind (mapId target)
            (match unwind with | .cleanup cleanup => .cleanup (mapId cleanup) | value => value)
      | .return_ values => .return_ values
      | .throw_ kind arguments => .throw_ kind arguments
      | .unreachable => .unreachable
      | .resume => .resume
      | .abort => .abort
    some { block with terminator }
  return ({ entry := mapId graph.entry, blocks }, originals)

private def nearestJoin (graph : RawCfg) (starts : Array Nat)
    (active : Array ActiveLoop) : Option Nat :=
  if starts.isEmpty then none else
  let blocked := active.flatMap fun loop => #[loop.header] ++ loop.exit.toArray
  let allDistances := starts.map fun start => distances graph start blocked
  let candidates : Array (Nat × Nat) := (Array.range graph.blocks.size).filterMap fun index =>
    let distances := allDistances.map (·[index]!)
    if distances.all (Option.isSome ·) then
      some (index, distances.foldl (fun total distance => total + distance.getD 0) 0)
    else none
  let best : Option (Nat × Nat) := candidates.foldl (init := none) fun best candidate =>
    match best, candidate with
    | none, candidate => some candidate
    | some previous@(_, previousDistance), candidate@(_, candidateDistance) =>
        if candidateDistance < previousDistance then some candidate else some previous
  best.map fun (index, _) => index

private def transfer? (target : Nat) (active : Array ActiveLoop) : Option (Nat × Bool) :=
  active.zipIdx.findSome? fun (loop, nest) =>
    if target == loop.header then some (nest, true)
    else if loop.exit == some target then some (nest, false)
    else none

private partial def emit (current : Nat) (stop : Option Nat) (active : Array ActiveLoop)
    (fuel : Nat) : BuildM ExprId := do
  let context ← read
  let some block := context.graph.blocks[current]?
    | throw <| .error "LIR-ID-BOUNDS" s!"invalid basic block {current}"
  if fuel == 0 then
    throw <| .at "LIR-CFG-NOT-STRUCTURABLE"
      "control-flow recursion did not reach a structured boundary" block.loc
  if let some loop := context.loops.find? fun loop =>
      loop.header == current && !active.any (·.header == current) then
    let body ← emitCore current none (#[{ header := loop.header, exit := loop.exit }] ++ active)
      (fuel - 1)
    let loopExpr ← match loop.exit with
      | some _ => addUnitExpr block.loc (.loop none body)
      | none => addExpr block.loc (.loop none body)
    recordRegion .loop loop.header loopExpr loop.nodes loop.exit
    match loop.exit with
    | none => sequence block.loc #[loopExpr]
    | some exit =>
        if stop == some exit then sequence block.loc #[loopExpr]
        else sequence block.loc #[loopExpr] (some (← emit exit stop active (fuel - 1)))
  else
    emitCore current stop active fuel
where
  emitCore (current : Nat) (stop : Option Nat) (active : Array ActiveLoop)
      (fuel : Nat) : BuildM ExprId := do
    let context ← read
    let block := context.graph.blocks[current]!
    let tail ← match block.terminator with
      | .goto target => transferOrContinue block.loc target.index stop active fuel
      | .branch condition thenTarget elseTarget => do
          let thenIndex := thenTarget.index
          let elseIndex := elseTarget.index
          if thenIndex == elseIndex then
            transferOrContinue block.loc thenIndex stop active fuel
          else
            let join := nearestJoin context.graph #[thenIndex, elseIndex] active
            let thenExpr ← emitBranch thenIndex join active fuel block.loc
            let elseExpr ← emitBranch elseIndex join active fuel block.loc
            let branch ← match join with
              | some _ => addUnitExpr block.loc (.ifElse condition thenExpr (some elseExpr))
              | none => addExpr block.loc (.ifElse condition thenExpr (some elseExpr))
            recordRegion .branch current branch
            match join with
            | some join =>
                if stop == some join then pure (some branch)
                else pure (some (← sequence block.loc #[branch]
                  (some (← emit join stop active (fuel - 1)))))
            | none => pure (some branch)
      | .switch scrutinee cases defaultTarget => do
          let includeDefault := !directlyUnreachable context.graph defaultTarget.index
          let targets := cases.map (·.2.index) ++
            (if includeDefault then #[defaultTarget.index] else #[])
          if includeDefault && targets.all (· == targets[0]!) then
            transferOrContinue block.loc targets[0]! stop active fuel
          else
            let state ← get
            let some scrutineeExpr := state.expressions[scrutinee.index]?
              | throw <| .at "LIR-ID-BOUNDS"
                  s!"invalid switch scrutinee expression {scrutinee.index}" block.loc
            let join := nearestJoin context.graph targets active
            let mut arms := #[]
            for (value, target) in cases do
              let pattern ← addPattern block.loc scrutineeExpr.typeId (.literal value)
              let body ← emitBranch target.index join active fuel block.loc
              arms := arms.push { pattern, body }
            if includeDefault then
              let defaultPattern ← addPattern block.loc scrutineeExpr.typeId .wildcard
              let defaultBody ← emitBranch defaultTarget.index join active fuel block.loc
              arms := arms.push { pattern := defaultPattern, body := defaultBody }
            let branch ← match join with
              | some _ => addUnitExpr block.loc (.match_ scrutinee arms)
              | none => addExpr block.loc (.match_ scrutinee arms)
            recordRegion .switch current branch
            match join with
            | some join =>
                if stop == some join then pure (some branch)
                else pure (some (← sequence block.loc #[branch]
                  (some (← emit join stop active (fuel - 1)))))
            | none => pure (some branch)
      | .call call (some destination) .unreachable => do
          let assignment ← addUnitExpr block.loc (.assign destination.place call)
          recordRegion .callContinuation current assignment
          let continuation ← transferOrContinue block.loc destination.target.index stop active fuel
          some <$> sequence block.loc #[assignment] continuation
      | .drop place target .unreachable => do
          -- The Rust import profile fixes panic behavior to `panic=abort`.
          -- A drop with no cleanup successor is therefore an ordinary
          -- explicit destruction followed by its normal continuation.
          let destruction ← addUnitExpr block.loc <|
            .operation (.drop place) #[] #[] none
          let continuation ← transferOrContinue block.loc target.index stop active fuel
          some <$> sequence block.loc #[destruction] continuation
      | .assert condition expected kind target .unreachable => do
          unless panicAssertKind kind do
            throw <| .at "LIR-CFG-MIR-TERMINATOR"
              "this raw MIR assertion kind is preserved but not yet structurizable" block.loc
          -- Under the checked initial Rust profile (`panic=abort`), the MIR
          -- assertion payload is not observable by the program. Preserve its
          -- control meaning: the expected condition continues normally and
          -- the other branch terminates with the shared panic outcome.
          let success ← emitBranch target.index stop active fuel block.loc
          let state ← get
          let some successExpr := state.expressions[success.index]?
            | throw <| .at "LIR-ID-BOUNDS"
                s!"invalid structured assertion successor {success.index}" block.loc
          let failure ← addTypedExpr block.loc successExpr.typeId (.throw_ .panic #[])
          let assertion ← if expected then
            addTypedExpr block.loc successExpr.typeId (.ifElse condition success (some failure))
          else
            addTypedExpr block.loc successExpr.typeId (.ifElse condition failure (some success))
          pure (some assertion)
      | .abort => some <$> addExpr block.loc (.throw_ .panic #[])
      | .call .. | .drop .. | .assert .. | .unreachable | .resume =>
          throw <| .at "LIR-CFG-MIR-TERMINATOR"
            "this raw MIR terminator is preserved but not yet structurizable" block.loc
      | .return_ values => some <$> addExpr block.loc (.return_ values)
      | .throw_ kind arguments => some <$> addExpr block.loc (.throw_ kind arguments)
    let statements := block.statements.filterMap fun
      | .execute expression => some expression
      | _ => none
    let result ← sequence block.loc statements tail
    recordBlock current result
    pure result

  emitBranch (target : Nat) (join : Option Nat) (active : Array ActiveLoop)
      (fuel : Nat) (loc : LocId) : BuildM ExprId := do
    if let some transfer := transfer? target active then
      if transfer.2 then addExpr loc (.continue_ transfer.1)
      else addExpr loc (.break_ transfer.1 none)
    else if join == some target then empty loc
    else emit target join active (fuel - 1)

  transferOrContinue (loc : LocId) (target : Nat) (stop : Option Nat)
      (active : Array ActiveLoop) (fuel : Nat) : BuildM (Option ExprId) := do
    if let some transfer := transfer? target active then
      if transfer.2 then some <$> addExpr loc (.continue_ transfer.1)
      else some <$> addExpr loc (.break_ transfer.1 none)
    else if stop == some target then
      pure none
    else
      some <$> emit target stop active (fuel - 1)

private structure GraphResult where
  expressions : Array Expr
  patterns : Array Pattern
  root : ExprId
  blocks : Array StructurizedBlock
  edges : Array StructurizedEdge
  regions : Array StructurizedRegion

private def graphWithWitness (expressions : Array Expr) (patterns : Array Pattern) (resultTypeId : TypeId)
    (unitTypeId : Option TypeId) (cfg : RawCfg) :
    Except Diagnostic GraphResult := do
  if cfg.entry.index >= cfg.blocks.size then
    throw <| .error "LIR-ID-BOUNDS"
      s!"invalid entry block {cfg.entry.index}; graph has {cfg.blocks.size} blocks"
  for block in cfg.blocks do
    if block.statements.any fun statement =>
        !(statement matches .execute _) && !inertStatement statement then
      throw <| .at "LIR-CFG-MIR-STATEMENT"
        "this destructive or provenance-sensitive raw MIR statement is preserved but not yet structurizable"
        block.loc
  let (cfg, originalBlocks) := pruneUnreachable cfg
  let loops ← loopInfos cfg
  let context := { graph := cfg, loops, originalBlocks, resultTypeId, unitTypeId }
  let initial := { expressions, patterns }
  let (root, state) ← (emit cfg.entry.index none #[] (cfg.blocks.size * cfg.blocks.size * 4 + 1)).run context |>.run initial
  let blocks := originalBlocks.map fun block => {
    block
    expressions := state.blockExpressions.filterMap fun (source, expression) =>
      if source == block then some expression else none }
  let edges := (Array.range cfg.blocks.size).flatMap fun source =>
    (successors cfg source).map fun target => {
      source := originalBlocks[source]!
      target := originalBlocks[target]! }
  return {
    expressions := state.expressions
    patterns := state.patterns
    root := root
    blocks := blocks
    edges := edges
    regions := state.regions }

/-- Convert one checked-bounds raw graph to a structured root, appending only
new control nodes to the namespace expression arena. -/
def graph (expressions : Array Expr) (patterns : Array Pattern) (resultTypeId : TypeId)
    (unitTypeId : Option TypeId) (cfg : RawCfg) :
    Except Diagnostic (Array Expr × Array Pattern × ExprId) := do
  let result ← graphWithWitness expressions patterns resultTypeId unitTypeId cfg
  pure (result.expressions, result.patterns, result.root)

private def witnessFailure (message : String) : Except Diagnostic α :=
  .error <| .error "LIR-CFG-WITNESS" message

private def impossibleSwitchDefault (cfg : RawCfg) (block : BlockId) : Bool :=
  directlyUnreachable cfg block.index &&
    let incoming := (predecessors cfg)[block.index]?.getD #[]
    !incoming.isEmpty && incoming.all fun source =>
      match cfg.blocks[source]?.map (·.terminator) with
      | some (RawTerminator.switch _ _ defaultTarget) => defaultTarget == block
      | _ => false

/-- Independently check the durable correspondence emitted for one graph.
This is public so mutation tests and alignment tooling can audit a witness
without gaining access to the private `ValidatedUnit` constructor. -/
def checkStructurizationWitness (cfg : RawCfg) (expressions : Array Expr)
    (witness : StructurizationWitness) : Except Diagnostic Unit := do
  let (pruned, originalBlocks) := pruneUnreachable cfg
  unless witness.entry == cfg.entry do
    witnessFailure "witness entry does not match the raw CFG entry"
  unless witness.root.index < expressions.size do
    witnessFailure s!"witness root expression {witness.root.index} is out of bounds"
  unless witness.blocks.map (·.block) == originalBlocks do
    witnessFailure "witness block list does not exactly cover reachable raw CFG blocks"
  for block in witness.blocks do
    if block.expressions.isEmpty then
      unless impossibleSwitchDefault cfg block.block do
        witnessFailure s!"block {block.block.index} has no structured correspondence"
    else for expression in block.expressions do
      unless expression.index < expressions.size do
        witnessFailure s!"block {block.block.index} maps to an out-of-bounds expression"
  let expectedEdges := (Array.range pruned.blocks.size).flatMap fun source =>
    (successors pruned source).map fun target => {
      source := originalBlocks[source]!
      target := originalBlocks[target]! }
  unless witness.edges == expectedEdges do
    witnessFailure "witness edges do not exactly match reachable raw CFG edges"
  let loops ← loopInfos pruned
  let mut expected : Array
      (StructurizedRegionKind × BlockId × Array BlockId × Option BlockId) := #[]
  for (block, index) in pruned.blocks.zipIdx do
    let original := originalBlocks[index]!
    match block.terminator with
    | .branch _ thenTarget elseTarget =>
        if thenTarget != elseTarget then
          expected := expected.push (.branch, original, #[], none)
    | .switch _ cases defaultTarget =>
        let includeDefault := !directlyUnreachable pruned defaultTarget.index
        let targets := cases.map (·.2.index) ++
          (if includeDefault then #[defaultTarget.index] else #[])
        unless includeDefault && targets.all (· == targets[0]!) do
          expected := expected.push (.switch, original, #[], none)
    | .call _ (some _) .unreachable =>
        expected := expected.push (.callContinuation, original, #[], none)
    | _ => pure ()
  for loop in loops do
    expected := expected.push (.loop, originalBlocks[loop.header]!,
      loop.nodes.map (originalBlocks[·]!), loop.exit.map (originalBlocks[·]!))
  for (kind, header, _, _) in expected do
    let matchingRegions := witness.regions.filter fun region =>
      region.kind == kind && region.header == header
    if matchingRegions.isEmpty then
      witnessFailure s!"witness omits region at block {header.index}"
  for region in witness.regions do
    let some (_, _, members, exit) := expected.find? fun (kind, header, _, _) =>
        region.kind == kind && region.header == header
      | witnessFailure s!"witness has an unexpected region at block {region.header.index}"
    unless region.members == members && region.exit == exit do
      witnessFailure s!"witness has incorrect loop membership or exit at block {region.header.index}"
    let some expression := expressions[region.expression.index]?
      | witnessFailure s!"region at block {region.header.index} maps to an out-of-bounds expression"
    let correctKind := match region.kind, expression.kind with
      | .branch, .ifElse .. => true
      | .switch, .match_ .. => true
      | .loop, .loop .. => true
      | .callContinuation, .assign .. => true
      | _, _ => false
    unless correctKind do
      witnessFailure s!"region at block {region.header.index} maps to the wrong expression kind"

/-- Structurize every graph body in a namespace and retain an independently
checked correspondence witness for each converted CFG function.  The result is
the backend-stage namespace: every function body is `.structured` or
`.absent`, so no raw graph survives this boundary. -/
def structurizeNamespace (tables : Tables) (namespaceId : NamespaceId) (ns : RawNamespace) :
    Except (Array Diagnostic) (Namespace FunctionBody × Array StructurizationWitness) := do
  let mut expressions := ns.expressions
  let mut patterns := ns.patterns
  let mut functions : Array (FunctionDecl FunctionBody) := #[]
  let mut witnesses := #[]
  let unitTypeId := tables.types.findIdx? (fun type => type == .unit) |>.map (⟨·⟩)
  for (function, functionIndex) in ns.functions.zipIdx do
    match function.body with
    | .cfg cfg =>
        let typeId := function.signature.results[0]?.map (·.typeId) |>.getD ⟨0⟩
        match graphWithWitness expressions patterns typeId unitTypeId cfg with
        | .ok result =>
            expressions := result.expressions
            patterns := result.patterns
            let parameterIds := Array.range function.signature.parameters.size |>.map (⟨·⟩)
            let mut root := result.root
            for localDecl in function.locals.reverse do
              unless parameterIds.contains localDecl.id do
                let patternId : PatternId := ⟨patterns.size⟩
                patterns := patterns.push {
                  loc := localDecl.loc, typeId := localDecl.type.typeId,
                  kind := .variable localDecl.id }
                let exprId : ExprId := ⟨expressions.size⟩
                expressions := expressions.push {
                  loc := localDecl.loc, typeId, kind := .letDecl patternId none root }
                root := exprId
            let witness : StructurizationWitness := {
              namespaceId
              functionId := ⟨functionIndex⟩
              alignment := function.alignment
              entry := cfg.entry
              root
              blocks := result.blocks
              edges := result.edges
              regions := result.regions }
            match checkStructurizationWitness cfg expressions witness with
            | .ok () => witnesses := witnesses.push witness
            | .error diagnostic => throw #[diagnostic]
            functions := functions.push (function.withBody (.structured root))
        | .error diagnostic => throw #[diagnostic]
    | .structured root => functions := functions.push (function.withBody (.structured root))
    | .absent => functions := functions.push (function.withBody .absent)
  return (ns.withStage expressions patterns functions, witnesses)

end LeanerIR.Import.Structurize
