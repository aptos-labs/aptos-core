-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Decide
import VerexIR.Proofs.Denote.Solve.Normal
import VerexIR.Proofs.Denote.Solve.Invoke
import Std.Sync.Mutex

/-!
# Closing the verification condition of a denotation

After the `vir_denote` normalization a goal is a tree of conjunctions,
implications, and conditionals whose leaves are arithmetic over the
native values and their certificates, or decidable propositions.  The
solver splits the tree structurally and decides each leaf: certificates
become bounds, then `omega`; finite leaves go to `decide`.  A leaf that
no decision procedure closes is reported at the authored clause its
`Obligation` marker names.  Nothing here selects a route or matches a
goal shape.

This module holds the walker (`solveGoals`) and the tactics of authored
proofs. The parts before it, split for build time: `Solve.Base` (options,
accounting, leaf facts, caches), `Solve.Decide` (the leaf deciders),
`Solve.Normal` (the normal form's simprocs and lemmas), and
`Solve.Invoke` (splits, behaviors, invocations).
-/

namespace VerexIR.Proofs.Denote

open Lean Elab Tactic Meta

/-- The last segment of a qualified name, as an identifier. -/
private def identifierPart (name : String) : String :=
  let last := (name.splitOn "::").getLast?.getD name
  String.map (fun c => if c.isAlphanum then c else '_') last

/-- The key a leaf's origin gives it, where no clause names it. -/
private def Provenance.key : Provenance → ObligationKey
  | .precondition callee call => { kind := "requires", callee := identifierPart callee, call }
  | .continuation callee call => { kind := "after", callee := identifierPart callee, call }
  | .loopEntry loop => { kind := "invariant", phase := "entry", loop }
  | .loopIteration loop => { kind := "invariant", phase := "step", loop }
  | .assertion => { kind := "assert" }
  | .mutationEnd | .construction | .copyOut | .exitMutationEnd | .handOver =>
      { kind := "data invariant" }
  | .memoryWritten | .callReturned => { kind := "module invariant" }
  | .lemmaRequirement => { kind := "lemma requires" }
  | .unrollBound _ => { kind := "unroll" }
  | .storedWrite => { kind := "stored invariant" }
  | .ghostWrite => { kind := "ghost domain" }
  | .pointAssert point index => { kind := "assert", index, point }
  | .returns => { kind := "aborts_if" }

/-- The key of a clause by its name, `kind_index` as the generator names a
function's clauses (`ensures_2`, `aborts_if_1`), or the name itself. -/
private def clauseKey (name : String) : ObligationKey :=
  match name.splitOn "_" |>.reverse with
  | last :: kind@(_ :: _) => match last.toNat? with
      | some index => { kind := "_".intercalate kind.reverse, index }
      | none => { kind := name }
  | _ => { kind := name }

/-- The numbers of the target's abort conditions, as `names` names its clauses. -/
private def abortClauses (names : Array (ObligationRange × String)) : Array Nat :=
  (names.filterMap fun (_, name) => do
    let key := clauseKey name
    guard (key.kind == "aborts_if")
    key.index).qsort (· < ·)

/-- The key of a returning path's leaf: every abort condition of the target. -/
private def returnsKey (names : Array (ObligationRange × String)) : ObligationKey :=
  let numbers := abortClauses names
  { kind := "aborts_if", index := numbers[0]?, also := numbers.extract 1 numbers.size }

/-- A leaf's key: the clause it owes, as `names` names the function's
clauses and its callees', at entry or after an iteration for a loop
invariant, at the call for a callee's precondition; else its origin. -/
private def leafKey (goal : MVarId) (provenance : Option Provenance)
    (clauses : Array ObligationRange) (names : Array (ObligationRange × String)) :
    MetaM ObligationKey := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  let ranges := obligationRanges target ++ clauses
  let named := ranges.findSome? fun range => (names.find? (·.1 == range)).map (·.2)
  let numbers ← (← targetState).preconditionNumbers.get
  let numbered := ranges.findSome? fun range => (numbers.find? (·.1 == range)).map (·.2)
  let invariants ← (← targetState).invariantKeys.get
  let invariant := ranges.findSome? fun range => (invariants.find? (·.1 == range)).map (·.2)
  -- A stored invariant is keyed by its clause, a data invariant by its struct, only
  -- where a proof names one.
  let routed := match ← (← targetState).routedKeys.get with
    | some (patterns, _) => patterns
    | none => #[]
  let storedClauses := routed.any fun key => key.kind == "stored invariant" && key.index.isSome
  let invariantKey (name : String) : ObligationKey :=
    match name.splitOn "@" with
    | [clause, owner] =>
        { clauseKey clause with owner := if routed.any (·.owner.isSome) then some owner else none }
    | _ => clauseKey name
  return match provenance, named with
    | some .returns, _ => returnsKey names
    | some (.precondition callee call), _ =>
        { (Provenance.precondition callee call).key with index := numbered }
    | some (.loopEntry loop), some name => { clauseKey name with phase := "entry", loop }
    | some (.loopIteration loop), some name => { clauseKey name with phase := "step", loop }
    | _, some name => clauseKey name
    -- An invariant a clause states, where the step owing it names no
    -- invariant of its own (a stored or data invariant where a write or a
    -- mutation ends).
    | _, none => match invariant.map invariantKey, provenance with
      | some key, some origin =>
          if origin.key.kind.endsWith "invariant" then
            if origin.key.kind == key.kind then key
            else if storedClauses && origin.key.kind == "stored invariant" then
              { origin.key with index := key.index }
            else origin.key
          else key
      | some key, none => key
      | none, some origin => origin.key
      -- The stored invariants a memory owes, where no write names them:
      -- after a call of a function value, at the exit.
      | none, none =>
        if target.consumeMData.isAppOf ``DataInvariants then { kind := "stored invariant" }
        else { kind := "leaf" }

/-- The keys of a path's clauses, as its leaves will have them (`leafKey`):
each clause a range of its target or a stripped clause names, and the key
its origin gives it. -/
private def pathKeys (goal : MVarId) (provenance : Option Provenance)
    (clauses : Array ObligationRange) (names : Array (ObligationRange × String)) :
    MetaM (Array ObligationKey) := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  let named := (obligationRanges target ++ clauses).filterMap fun range =>
    (names.find? (·.1 == range)).map fun (_, name) => match provenance with
      | some (.loopEntry loop) => { clauseKey name with phase := "entry", loop }
      | some (.loopIteration loop) => { clauseKey name with phase := "step", loop }
      | _ => clauseKey name
  -- A returning path owes every abort condition.
  let returning := if (target.find? (·.isAppOfArity ``NoAbort 1)).isSome then
      (abortClauses names).map fun index => { kind := "aborts_if", index := some index }
    else #[]
  return (named ++ returning).push (← leafKey goal provenance clauses names)

/-- Restore the context-dependent caches along with a failed automatic
attempt. Measurements and budget debits deliberately survive rollback. -/
private def runAutomatic (ledger : IO.Ref SharedBudgets.Ledger)
    (owners : Array SharedBudgets.OwnerId) (work : SharedBudgets.Work)
    (action : TacticM α) (cacheOwners : Option (Array SharedBudgets.OwnerId) := none) :
    TacticM (SharedBudgets.Outcome α × Nat) := do
  let restore ← IO.mkRef (pure () : IO Unit)
  let previousOwners ← (← targetState).preparationOwners.get
  let mut committed := false
  try
    (← targetState).preparationOwners.set (cacheOwners.getD owners)
    let result ← SharedBudgets.run ledger owners work do
      restore.set (← saveWalkCaches)
      action
    if let .done _ := result.1 then committed := true
    return result
  finally
    (← targetState).preparationOwners.set previousOwners
    unless committed do (← restore.get)

private def countWoken : MetaM Unit :=
  countStructure fun counts => { counts with woken := counts.woken + 1 }

private def countAlternatives (alternatives : List MVarId) : MetaM Unit :=
  countStructure fun counts =>
    { counts with alternatives := counts.alternatives + alternatives.length }

/-- The observer of an exposure, collecting the equations it keeps as facts. Printing them is left to `reportExposure`, outside the exposure's parts. -/
private def exposeObserver (timed : Bool) (failures : IO.Ref (Array (MVarId × Lean.Expr))) :
    SharedContinuations.ExposeObserver where
  timed := fun label step => if timed then timedPart label step else step
  woke := countWoken
  keptEquation := fun goal equation => failures.modify (·.push (goal, equation))

private def reportExposure (failures : Array (MVarId × Lean.Expr)) (alternatives : List MVarId) :
    MetaM Unit := do
  unless verex.solverDebug.get (← getOptions) do return
  for (goal, equation) in failures do
    let printed ← goal.withContext (ppExpr equation)
    IO.println s!"  expose kept equation: {(toString printed).take 4000}"
  for alternative in alternatives do
    IO.println s!"join alternative:\n{← ppGoal alternative}"

/-- The incoming alternatives of a goal under entry relations, each in the
walk normal form. Linking an entry row substitutes the arrival's values into
a target normalized over abstract ones; normalizing it again gives the
alternative what `exitStep` gives the same path's concrete arrival, where a
clause every value settles closes to `True` before any decider. -/
private def exposeNormalEntries (normalization : Simp.Context × Simp.SimprocsArray)
    (goal : MVarId) (entries : Array Name) : TacticM (List MVarId) := do
  -- A target the summary's walk settled without its rows, or one its
  -- context states over the abstract rows, needs no arrival. Stated means
  -- up to reducible unfolding (`verex_assumption`): a comparison at default
  -- transparency unfolds the clauses and relations of the goal, and what it
  -- then unifies is not what the context states.
  if (← goal.withContext (instantiateMVars (← goal.getType))).isConstOf ``True then
    goal.assign (mkConst ``True.intro)
    return []
  -- A goal its context states, as the tree walk's leaf closes it first
  -- (`decideOne`): by a hypothesis, or by the stated facts, such as a
  -- `False` the callee's abort clause states under its marker.
  if ← closesBy goal (← `(tactic| first | verex_assumption | verex_stated)) then return []
  -- The stored invariants of a row the summary assumes (`sealNode`), under
  -- the premise the walk introduced.
  if (← goal.withContext (instantiateMVars (← goal.getType))).isAppOfArity
      ``VerexIR.Proofs.Denote.DataInvariants 4 then
    if ← closesBy goal (← `(tactic| verex_data_invariants)) then return []
  let exposed ← IO.mkRef [goal]
  let failures ← IO.mkRef #[]
  let observer := exposeObserver true failures
  tryCatchRuntimeEx
    (timedStep "join exposure" do
      exposed.set (← SharedContinuations.exposeEntries goal entries closeFrameWrites observer))
    fun failure => do
      withTheReader Core.Context (fun context => { context with maxHeartbeats := 0 })
        (reportExposure (← failures.get) [])
      throw failure
  let alternatives ← exposed.get
  countAlternatives alternatives
  reportExposure (← failures.get) alternatives
  if alternatives == [goal] then return alternatives
  alternatives.filterMapM fun alternative => do
    setGoals [alternative]
    normalizeAround (fixedPoints := walkFixedPoints) normalization #[]
    return (← getGoals).head?

/-- The arrivals a shared suffix's goals answer for: the suffix was queued
with `provenance` and the union `clauses` of its arrivals, and each arrival
owes its own provenance and clauses. A failed leaf of the suffix is reported
for each arrival, with the clauses the suffix's walk added since
(`attributed`). -/
structure OwedAttribution where
  provenance : Option Provenance
  clauses : Array ObligationRange
  arrivals : Array (Option Provenance × Array ObligationRange)
  deriving Inhabited

/-- Whom a goal with `provenance` and `clauses` answers for under
`attribution`: each arrival, with the clauses added since the suffix was
queued, while the goal still carries the suffix's provenance; else the goal
itself, its obligation being the suffix's own. -/
private def attributed (attribution : Option OwedAttribution) (provenance : Option Provenance)
    (clauses : Array ObligationRange) : Array (Option Provenance × Array ObligationRange) :=
  match attribution with
  | some owed =>
      if provenance == owed.provenance then
        let added := clauses.filter (!owed.clauses.contains ·)
        owed.arrivals.foldl (init := #[]) fun found (origin, owing) =>
          let entry := (origin, added.foldl (fun c r => if c.contains r then c else c.push r) owing)
          if found.contains entry then found else found.push entry
      else #[(provenance, clauses)]
  | none => #[(provenance, clauses)]

/-- Decide a leaf within one budget covering call observations and the
leaf pipeline, or report the clause it owes and admit it. Whether it was
decided, and the cost of the complete attempt. -/
private def solveLeaf (normalization : Simp.Context × Simp.SimprocsArray)
    (goal : MVarId) (provenance : Option Provenance)
    (clauses : Array ObligationRange) (entries : Array Name := #[])
    (owners? : Option (Array SharedBudgets.OwnerId) := none)
    (entryOpened : Bool := false) (owed : Option OwedAttribution := none) :
    TacticM (Bool × Array (String × Nat)) := do
  let debug := verex.solverDebug.get (← getOptions)
  let mut stageCost : Array (String × Nat) := #[]
  setGoals [goal]
  let leafStart ← IO.getNumHeartbeats
  let ranges ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    pure <| (clauses ++ obligationRanges target).toList.eraseDups.filter
      fun range => range.startByte < range.endByte
  let (reported, rejectedClauses) ← (← targetState).leafReports.get
  let repeated := !ranges.isEmpty && ranges.all fun range =>
    rejectedClauses.contains (provenance, range)
  let leafBudget := verex.leafHeartbeats.get (← getOptions) * 1000
  let exhausted ← IO.mkRef false
  let saved ← saveState
  let closed ← if repeated then pure false else do
    tryCatchRuntimeEx
      (do
        -- Without recovery: a failing alternative raises at once, so the
        -- next alternative runs instead of an aborted goal list. Under a
        -- leaf budget, exhausting it is the leaf's failure.
        -- An explicit bit-vector request goes to its decision procedure
        -- after normalization, without trying the general search first.
        let bitVectors ← if owners?.isSome then
            `(tactic| first | verex_bv_target | verex_bv)
          else `(tactic| verex_bv)
        let fallback ← if (← getOptions).getBool `verex.bitVectors false then
            `(tactic| first
              | (verex_prepare_leaf
                 all_goals verex_saturate
                 all_goals first | omega | $bitVectors:tactic)
              | verex_leaf_attempt)
          else `(tactic| verex_leaf_attempt)
        let decideOne (goal : MVarId) : TacticM Unit := do
          -- A goal its context states closes before any preparation,
          -- observation or decider spends the allowance.
          setGoals [goal]
          if ← (do evalTactic (← `(tactic| first | verex_assumption | verex_stated)); pure true)
              <|> pure false then
            if (← getGoals).isEmpty then return
          setGoals [goal]
          -- The clauses' defined integers, at the locals the path named.
          evalTactic (← `(tactic| verex_defined_values))
          -- The data invariants the leaf's certified values carry: a read
          -- assumes nothing beyond its value's certificate, so the clauses
          -- are a fact of the leaf and not of its preparation
          -- (`assertCertifiedClauses`).
          evalTactic (← `(tactic| verex_certified_clauses))
          let some goal := (← getGoals).head? | return
          -- The call rule already supplies its result and contract. Many
          -- leaves need only those observations, within the same budget
          -- as the remaining deciders rather than before the cap starts.
          let hasObservation ← goal.withContext do
            (← getLCtx).anyM fun decl => do
              if decl.isImplementationDetail then return false
              let type := unmarked (← instantiateMVars decl.type)
              -- Hypothetical observations inside quantified invariants
              -- are not observed invocations; a call's result is stated at
              -- every default (`∀ default, result_of … = r`).
              let stated := if type.isForall then type.bindingBody! else type
              if let some (_, lhs, rhs) := stated.eq? then
                return lhs.isAppOf ``VerexIR.Proofs.ResultOf || rhs.isAppOf ``VerexIR.Proofs.ResultOf
              return (unmarked (type.not?.getD type)).isAppOf ``VerexIR.Proofs.AbortsOf
          if hasObservation then
            if ← closesBy goal (← `(tactic| verex_timed "call observations" verex_call_observations)) then
              setGoals []
              return
          setGoals [goal]
          -- The automatic solver already starts with cheap deciders. A
          -- separate residual pass repeats those attempts and consumes the
          -- shared leaf budget before preparation can make progress.
          -- A leaf the deciders do not close goes through the prepared
          -- pipeline too, rather than failing here: a leaf whose locals
          -- test no variant is never opened for preparation
          -- (`verex_open_prepared`), and two obligations are closed only in
          -- source form — an arithmetic refutation whose facts the walk
          -- states of a prepared row, and an obligation over certified
          -- values, whose closer substitutes rather than rewrites
          -- (`verex_certified_congr`).  The goal is restored as the
          -- deciders received it and its binders introduced, as the
          -- preparation's own step does; where even that does not apply,
          -- the deciders' failure stands.
          let received ← saveState
          try
            Lean.Elab.Tactic.withoutRecover (evalTactic fallback)
          catch undecided =>
            received.restore
            -- The prepared form is for what the source form settles: a
            -- refutation, whose facts the walk states of a prepared row,
            -- and an equation of certified values, whose closer substitutes
            -- rather than rewrites (`verex_certified_congr`).  A leaf of
            -- any other shape fails here, as it did: the source form would
            -- cost it the target's budget and settle nothing.
            let needsSource ← goal.withContext do
              let target ← instantiateMVars (← goal.getType)
              pure (target.isConstOf ``False ||
                (target.find? (·.isAppOfArity ``Subtype.mk 4)).isSome)
            unless needsSource do
              received.restore
              throw undecided
            setGoals [goal]
            let tagged ← (try
                Lean.Elab.Tactic.withoutRecover (evalTactic
                  (← `(tactic| (verex_intro_any; verex_tag_preparation))))
                pure true
              catch _ => pure false)
            unless tagged do
              received.restore
              throw undecided
        -- Each attempt at the leaf within its own budget: the cases of a
        -- prepared split an attempt leaves open are attempts of their own.
        let budgeted (act : TacticM Unit) : TacticM Unit := do
          if leafBudget == 0 then return ← act
          let start ← IO.getNumHeartbeats
          try withBudget leafBudget act
          catch failure =>
            if (← IO.getNumHeartbeats) - start >= leafBudget then exhausted.set true
            throw failure
        let decide : TacticM Unit := do
          -- The existing path has no entry relations or owned residual queue.
          if owners?.isNone then
            timedStep "leaf decide" (budgeted (decideOne goal))
            setGoals (← decidePreparedCases (← getUnsolvedGoals) budgeted)
            return
          -- Each attempt within its own allowance, as on the existing path:
          -- every incoming alternative and every case of its prepared split.
          let alternatives ← exposeNormalEntries normalization goal entries
          let opened := entryOpened || alternatives != [goal]
          for alternative in alternatives do
            let mut pending := [alternative]
            if opened then
              let saved ← saveState
              let restoreCaches ← saveWalkCaches
              pending ← tryCatchRuntimeEx (do
                setGoals pending
                Lean.Elab.Tactic.withoutRecover <| evalTactic
                  (← `(tactic| verex_timed "join entry" verex_shared_entry))
                getUnsolvedGoals) fun _ => do
                saved.restore
                restoreCaches
                pure pending
            -- Substitution can expose proof-only variant cases. The leaf
            -- pipeline focuses one case at a time; retain its siblings in
            -- this same attempt instead of dropping them or renewing caps.
            while let next :: remaining := pending do
              let before ← goalContent next
              timedStep "join decide" (budgeted (decideOne next))
              let residual ← decidePreparedCases (← getUnsolvedGoals) budgeted
              for child in residual do
                if (← goalContent child) == before then
                  throwError "a shared continuation alternative made no progress"
              pending := residual ++ remaining
          setGoals []
        -- The leaf's deciders read the preparation its owners shared.
        if let some owners := owners? then
          let previous ← (← targetState).preparationOwners.get
          try
            (← targetState).preparationOwners.set owners
            decide
          finally (← targetState).preparationOwners.set previous
        else decide
        pure (← getGoals).isEmpty)
      fun failure => do
        -- The target's own heartbeat budget stays the target's failure; a
        -- leaf exhausting the recursion depth is that leaf's.
        if failure.isMaxHeartbeat then throw failure
        if debug then
          IO.println (← (m!"leaf not decided: {failure.toMessageData}").toString)
          -- The leaf as its attempt received it.
          let failed ← saveState
          saved.restore
          try
            setGoals [goal]
            evalTactic (← `(tactic| verex_trace_state "undecided leaf"))
          catch _ => pure ()
          failed.restore
        pure false
  let failure := if ← exhausted.get then "exhausted" else "undecided"
  stageCost := stageCost.push (if repeated then "rejected clause" else if closed then "leaf" else failure,
    (← IO.getNumHeartbeats) - leafStart)
  unless closed || repeated do
    let state ← targetState
    (if ← exhausted.get then state.exhaustedLeaves else state.undecidedLeaves).modify (· + 1)
  unless closed do
    saved.restore
    setGoals [goal]
    -- A shared suffix's leaf is reported for each arrival it answers for.
    let mut reported := reported
    for (origin, owing) in attributed owed provenance clauses do
      let name := (← leafKey goal origin owing (← (← targetState).clauseRanges.get)).tag.toString
      reported ← recordAutomaticReports name <| reportObligation goal origin owing reported
    let mut rejectedClauses := rejectedClauses
    for range in ranges do
      unless rejectedClauses.contains (provenance, range) do
        rejectedClauses := rejectedClauses.push (provenance, range)
    (← targetState).leafReports.set (reported, rejectedClauses)
    admitGoal goal
  -- Outside the states the leaf's attempts restored.
  reportRuntimeMention
  return (closed, stageCost)

/-- What every step of `solveGoals` reads: the target's data and the
solver's settings. -/
structure SolveContext where
  invariants : Array (Nat × Lean.Expr × Lean.Expr)
  callees : Array (Lean.Expr × String × Lean.Expr)
  equations : Array Lean.Expr
  /-- The equivalences of the callees' contracts specialized at the type
  arguments of the generic calls (`doc/designs/native-verification.md`,
  "Specialized contracts"). -/
  specializations : Array Lean.Expr := #[]
  handOver : Bool
  preparing? : Option (TSyntax ``Lean.Parser.Tactic.tacticSeq) := none
  /-- Whether the automatic walk prepares a completed path once, before its
  clauses and their cases split, for its leaves to share. -/
  preparesPaths : Bool := false
  /-- The hooks of an authored proof at program points (`verex_obligations
  at …`): each point's pattern, the hook's syntax, and its tactics. -/
  points : Array (ProgramPointKey × Syntax × TSyntax ``Lean.Parser.Tactic.tacticSeq) := #[]
  labeled : Bool
  unrolls : Array (Nat × Nat)
  preconditions : Array (Lean.Expr × Lean.Expr)
  /-- Under a model, the function handle of each inlined callee's meaning. -/
  inlinedHandles : Array (Lean.Expr × Lean.Expr) := #[]
  names : Array (ObligationRange × String)
  /-- The target's calls: each site, the callee's name, and the call's
  number among the calls of the callee and among all calls, in source
  order. -/
  callSites : Array (Nat × String × Nat × Nat) := #[]
  /-- The target's other sites a path's decisions name: each site, its kind
  (`abort`, `op`, `if`, …), its number among the sites of the kind, in
  source order, and a label (a `match` arm's variant). -/
  sites : Array (Nat × String × Nat × String) := #[]
  ghostWrites : Array (Nat × ObligationRange)
  debug : Bool
  profile : Bool
  shared : Bool
  budgets : IO.Ref SharedBudgets.Ledger
  normalization : Simp.Context × Simp.SimprocsArray
  startHeartbeats : Nat
  /-- The context the walk started in, which a derived summary's walk
  starts in too. -/
  base? : Option (LocalContext × LocalInstances) := none
  /-- The compiled callees whose summaries are being derived, innermost
  last: such a walk hands its leaves over as the summary's conjuncts. -/
  summarizing : Array Name := #[]

/-- The number of a call among the target's calls of its callee, by its
site. -/
private def SolveContext.callNumber (ctx : SolveContext) (site : Option Nat) : Option Nat :=
  site.bind fun site => (ctx.callSites.find? (·.1 == site)).map (·.2.2.1)

/-- A site's number among the target's sites of a kind. -/
private def SolveContext.siteNumber (ctx : SolveContext) (kind : String) (site : Nat) :
    Option Nat :=
  (ctx.sites.find? fun (other, otherKind, _) => other == site && otherKind == kind).map (·.2.2.1)

/-- A loop's number in a key (`invariant 2 of loop 1 step`): its number among
the target's loops, where it has several. -/
private def SolveContext.loopKey (ctx : SolveContext) (site : Nat) : Option Nat :=
  if (ctx.sites.filter (·.2.1 == "loop")).size > 1 then ctx.siteNumber "loop" site else none

/-- A site's number among the target's sites of a kind, and its label. -/
private def SolveContext.siteEntry (ctx : SolveContext) (kind : String) (site : Nat) :
    Option (Nat × String) :=
  (ctx.sites.find? fun (other, otherKind, _) => other == site && otherKind == kind).map (·.2.2)

/-- Where a path aborting in a call aborts, by the call's site. -/
private def SolveContext.callAbortSite (ctx : SolveContext) (site : Nat) : Option AbortSite :=
  (ctx.callSites.find? (·.1 == site)).map fun (_, callee, number, overall) =>
    .call number callee overall

/-- `verex_normalize` on each goal, by the walk's normalization, around the
`normal` terms. -/
private def normalizeGoals (ctx : SolveContext) (normal : Array Lean.Expr := #[]) :
    TacticM Unit := do
  let mut normalized := #[]
  for goal in ← getGoals do
    setGoals [goal]
    normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization normal
    normalized := normalized ++ (← getGoals).toArray
  setGoals normalized.toList

/-- An inlined callee's body unfolded in the main goal (`compiledUnfoldLemmas`,
the hypotheses, `denote`), around the `normal` terms. -/
private def unfoldInlined (lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma))
    (denote : Ident) (normal : Array Lean.Expr) : TacticM Unit := withMainContext do
  let { ctx, simprocs, .. } ← mkSimpContext
    (← `(tactic| simp only [$lemmas,*, $denote:ident, VerexIR.Proofs.Denote.NRow.nil_append,
      VerexIR.Proofs.Denote.NRow.cons_append]))
    (eraseLocal := false)
  normalizeAround (ctx, simprocs) normal

/-- Invocation-local measurements of the continuations the walker actually
folds. These are dynamic instances, not static CFG sites: a later diamond
can be folded separately on earlier paths. Reintroduced locals that no longer
have their original identity are reported as untracked arrivals, rather than
mistaken for new program sites. Profiling never annotates or rewrites goals. -/
structure ContinuationProfile where
  branchFolds : Nat := 0
  loopFolds : Nat := 0
  arrivals : Nat := 0
  revisits : Nat := 0
  untracked : Nat := 0
  visits : Std.HashMap FVarId Nat := {}

private def ContinuationProfile.folded (p : ContinuationProfile) (id : FVarId)
    (loop : Bool) : ContinuationProfile :=
  { p with
    branchFolds := p.branchFolds + (if loop then 0 else 1)
    loopFolds := p.loopFolds + (if loop then 1 else 0)
    visits := p.visits.insert id 0 }

private def ContinuationProfile.arrived (p : ContinuationProfile) (id : FVarId) :
    ContinuationProfile :=
  match p.visits[id]? with
  | some previous =>
      { p with
        arrivals := p.arrivals + 1
        revisits := p.revisits + (if previous == 0 then 0 else 1)
        visits := p.visits.insert id (previous + 1) }
  | none => { p with arrivals := p.arrivals + 1, untracked := p.untracked + 1 }

private def ContinuationProfile.report (p : ContinuationProfile) : String :=
  s!"  continuations: branch folds {p.branchFolds}, loop folds {p.loopFolds}, \
    arrivals {p.arrivals}, revisits {p.revisits}, untracked {p.untracked}"

/-- What an arriving step's path queues its successors with: an arrival
that resumes alone continues that path (`resumeShared`), as the exit rule
continues it (`exitStep`). -/
structure ArrivalStep where
  provenance : Option Provenance
  clauses : Array ObligationRange
  owed : Option OwedAttribution := none
  decisions : Array Decision
  points : Array ProgramPointKey
  owners : Array (List Bool × SharedBudgets.OwnerId) := #[]
  deriving Inhabited

/-- A structured region ends at its named continuation. Its summary resumes
in the enclosing region, after all of its incoming goals have arrived. -/
structure SharedRegion where
  node : SharedContinuations.Node
  parent : Option Name
  provenance : Option Provenance
  clauses : Array ObligationRange
  owed : Option OwedAttribution := none
  owners : Array (List Bool × SharedBudgets.OwnerId) := #[]
  debits : Array SharedBudgets.DebitId := #[]
  /-- The decisions and pending points of the path at the fold: the common
  prefix of every arrival's. -/
  decisions : Array Decision := #[]
  points : Array ProgramPointKey := #[]
  /-- The locals of the fold context the node keeps until its arrivals are
  in (`foldPins`). -/
  pins : FVarIdSet := {}
  /-- The normal conditions the folding bind passes on (`foldedNormal`). -/
  conditions : Array Lean.Expr := #[]
  /-- The arriving steps, indexed by `Arrival.step`. -/
  arrivalSteps : Array ArrivalStep := #[]
  deriving Inhabited

/-- The state of the traversal `solveGoals` runs: the goals still to solve,
each with its provenance and the clauses it owes; the runs `denotedRun`
reads, each the target's body the call rule inlines (no call of the
function, so naming no function value); the normal conditions of each
continuation folded into a local definition; the leaves handed over to an
authored proof, with their names; the number of leaves; and the cost of
each stage. -/
structure SolveState where
  shared : Bool := false
  pending : Array (MVarId × Option Provenance × Array ObligationRange × Option Nat) := #[]
  /-- Shared preparation has already run on these goals and their logical children. -/
  prepared : Std.HashSet MVarId := {}
  denoted : Array MVarId := #[]
  foldedNormal : Std.HashMap FVarId (Array Lean.Expr) := {}
  continuations : ContinuationProfile := {}
  regions : Array SharedRegion := #[]
  activeRegion : Option Name := none
  goalRegions : Std.HashMap MVarId Name := {}
  /-- Routes through the region's conjunction, stopping at source clauses.
  Empty routes belong to one atomic obligation, even after it splits. -/
  activeOwners : Array (List Bool × SharedBudgets.OwnerId) := #[]
  goalOwners : Std.HashMap MVarId (Array (List Bool × SharedBudgets.OwnerId)) := {}
  /-- Pruning work paid before clauses exist. Distinct identities survive
  program branches and are combined when those branches reach a join. -/
  activeDebits : Array SharedBudgets.DebitId := #[]
  goalDebits : Std.HashMap MVarId (Array SharedBudgets.DebitId) := {}
  /-- A group opened its entry before clause routing. Each clause still
  needs the ordinary post-entry preparation under its own allowance. -/
  activeEntryOpened : Bool := false
  openedEntries : Std.HashSet MVarId := {}
  /-- The arrivals the goal being stepped answers for (`OwedAttribution`). -/
  activeOwed : Option OwedAttribution := none
  goalOwed : Std.HashMap MVarId OwedAttribution := {}
  /-- The goal is the suffix of a sealed class of several arrivals, walked
  once over the class's relation: its clauses share owners and their
  preparation, and its leaves link the relation's rows. A goal under no
  summary is the tree's own path and is prepared and decided as without
  shared continuations. -/
  activeSummary : Bool := false
  goalSummaries : Std.HashSet MVarId := {}
  prepaidOwners : Std.HashSet SharedBudgets.OwnerId := {}
  pruningChecks : Nat := 0
  prunedEdges : Nat := 0
  /-- One common proof case may precede a group's clause decisions.
  Further cases stay in the leaf pipeline, avoiding an eager product
  of proof cases before any clause is decided. -/
  caseScheduled : Std.HashSet SharedBudgets.OwnerId := {}
  handed : Array (MVarId × ObligationKey) := #[]
  /-- The source decisions on the path of the goal being stepped, which the
  goals it queues take, each split adding its own (`queueDeciding`). -/
  activeDecisions : Array Decision := #[]
  goalDecisions : Std.HashMap MVarId (Array Decision) := {}
  /-- The loop of each iteration's recursive hypothesis, as its keys name it
  (`SolveContext.loopKey`). -/
  recursiveLoops : Std.HashMap FVarId (Option Nat) := {}
  /-- Local definitions the walk matches by identity, which a shared fold
  takes as captures rather than closing into its body (`fold?`). -/
  opaqueDefinitions : Std.HashSet FVarId := {}
  /-- The hooks at program points some path reached, by index. -/
  pointsUsed : Array Nat := #[]
  /-- The points a goal's path passed whose hooks wait for the walker to
  introduce the continuation's results and premises, which the goals it
  queues take (`goalPoints`). -/
  activePoints : Array ProgramPointKey := #[]
  goalPoints : Std.HashMap MVarId (Array ProgramPointKey) := {}
  /-- Whether the function's entry was reached (`entryStep`). -/
  entryReached : Bool := false
  /-- The binding index after the scope entries the walk has placed in
  goals (`scopedBinder`), not yet in a context. -/
  scopeIndex : Nat := 0
  /-- The goals a loop's entry hooks already ran on. -/
  pointsDone : Array (MVarId × ProgramPointKey) := #[]
  leaves : Nat := 0
  stageCost : Array (String × Nat) := #[]
  /-- The call site of the step most recently popped from `pending`, for the
  budget-stop report when `isMaxHeartbeat` fires inside that step. -/
  stepCallSite : Option Nat := none
  /-- The heartbeats when that step began, and the stages charged before it:
  what the step spent beyond its charged stages is the stage the budget
  interrupts (`traverse`). -/
  stepStart : Nat := 0
  stepStages : Nat := 0

/-- The traversal's monad: its state over the tactic monad. -/
abbrev SolveM := StateRefT SolveState TacticM

/-- A goal taken from the queue, with its provenance and clauses, and the
heartbeats when its step began. -/
structure SolveStep where
  goal : MVarId
  provenance : Option Provenance
  clauses : Array ObligationRange
  stageStart : Nat
  /-- The site of the call the goal's action is, where its marker was
  taken off (`callAtStep`). -/
  callSite : Option Nat := none

private def mergeDebits (left right : Array SharedBudgets.DebitId) :
    Array SharedBudgets.DebitId :=
  right.foldl (fun found id => if found.contains id then found else found.push id) left

/-- Queues `goals`, each with `provenance` and `clauses`. -/
private def queue (goals : List MVarId) (provenance : Option Provenance)
    (clauses : Array ObligationRange) (prepared : Bool := false) (callSite : Option Nat := none) :
    SolveM Unit :=
  modify fun s =>
    let s := { s with
      pending := s.pending ++ goals.toArray.map fun g => (g, provenance, clauses, callSite)
      goalDecisions := goals.foldl (fun decisions goal =>
        if s.activeDecisions.isEmpty then decisions.erase goal
        else decisions.insert goal s.activeDecisions) s.goalDecisions
      goalPoints := goals.foldl (fun points goal =>
        if s.activePoints.isEmpty then points.erase goal
        else points.insert goal s.activePoints) s.goalPoints
      prepared := if prepared then goals.foldl (·.insert ·) s.prepared else s.prepared }
    if !s.shared then s else { s with
      goalRegions := goals.foldl (fun regions goal => match s.activeRegion with
        | some region => regions.insert goal region
        | none => regions.erase goal) s.goalRegions
      goalOwners := goals.foldl (fun owners goal =>
        if s.activeOwners.isEmpty then owners.erase goal
        else owners.insert goal s.activeOwners) s.goalOwners
      goalDebits := goals.foldl (fun debits goal =>
        if s.activeDebits.isEmpty then debits.erase goal
        else debits.insert goal s.activeDebits) s.goalDebits
      openedEntries := goals.foldl (fun opened goal =>
        if s.activeEntryOpened then opened.insert goal else opened.erase goal) s.openedEntries
      goalOwed := goals.foldl (fun owed goal => match s.activeOwed with
        | some attribution => owed.insert goal attribution
        | none => owed.erase goal) s.goalOwed
      goalSummaries := goals.foldl (fun under goal =>
        if s.activeSummary then under.insert goal else under.erase goal) s.goalSummaries }

/-- Queues `goals` as `queue` does, their path taking `decision` beside the
decisions of the goal being stepped. -/
private def queueDeciding (decision : Decision) (goals : List MVarId)
    (provenance : Option Provenance) (clauses : Array ObligationRange) : SolveM Unit := do
  let active := (← get).activeDecisions
  modify fun s => { s with activeDecisions := active.push decision }
  queue goals provenance clauses
  modify fun s => { s with activeDecisions := active }

/-- Records the cost of `stage` since `start`. -/
private def charge (stage : String) (start : Nat) : SolveM Unit := do
  let now ← IO.getNumHeartbeats
  modify fun s => { s with stageCost := s.stageCost.push (stage, now - start) }

/-- Queue the obligations a point's asserts owe among `goals`, each a leaf of
its own (`.pointAssert`) at the point the path reached; the other goals. -/
private def queueOwed (point : ProgramPointKey) (goals : List MVarId) (prepared : Bool := false) :
    SolveM (List MVarId) := do
  let owed ← (← targetState).pointOwed.get
  let mut rest := []
  for goal in goals do
    match owed[goal]? with
    | some (_, index) =>
        -- Queued once: the leaf passes the end of its path later.
        (← targetState).pointOwed.modify (·.erase goal)
        queue [goal] (some (.pointAssert point index)) #[] prepared
    | none => rest := rest ++ [goal]
  return rest

/-- Run the hooks an authored proof places at `point` (`verex_obligations
at …`) on the current goals: what they establish there holds on every path
through the point and is proved once for all of them. -/
private def atPoint (ctx : SolveContext) (point : ProgramPointKey) : SolveM Unit := do
  let hooks := ctx.points.zipIdx.filter (·.1.1.selects point)
  if hooks.isEmpty then return
  let started ← IO.getNumHeartbeats
  for ((_, _, hook), index) in hooks do
    modify fun s => { s with
      pointsUsed := if s.pointsUsed.contains index then s.pointsUsed else s.pointsUsed.push index }
    let mut next := #[]
    for goal in ← getGoals do
      setGoals [goal]
      withOptions (verex.keepObligationMarkers.set · true) do evalTactic hook
      next := next ++ (← getUnsolvedGoals).toArray
    setGoals next.toList
  setGoals (← queueOwed point (← getGoals))
  -- The structural matchers read the logical head, as at solver entry.
  setGoals (← (← getGoals).mapM fun next => next.withContext do
    let target ← instantiateMVars (← next.getType)
    if target.isMData then next.replaceTargetDefEq target.consumeMData else pure next)
  charge "point" started

/-- The target's closed equations, such as its generic calls' frame
instantiations, apply wherever a body brings such a term into a goal. -/
private def rewriteEquations (equations : Array Lean.Expr) (goals : List MVarId) :
    TacticM (List MVarId) := do
  if equations.isEmpty then return goals
  let mut rewritten := []
  for goal in goals do
    setGoals [goal]
    let lemmas ← goal.withContext <| equations.mapM fun equation => do
      `(Lean.Parser.Tactic.simpLemma| $(← Lean.Elab.Term.exprToSyntax equation):term)
    evalTactic (← `(tactic| try simp only [$lemmas,*]))
    rewritten := rewritten ++ (← getGoals)
  return rewritten

/-- The function's arguments and state at its start, what a loop
invariant's `old` reads, from the goal's own context. A function's latest
start on the path: an inlined callee's calls are sequential, and a loop runs
inside the call that reached it. -/
private def start? (goal : MVarId) (function : Lean.Expr) :
    MetaM (Option (Lean.Expr × Lean.Expr)) :=
  goal.withContext do
    let mut found := none
    for declaration in ← getLCtx do
      let type ← instantiateMVars declaration.type
      if type.isAppOfArity ``FunctionStart 5 then
        if ← isDefEq (type.getArg! 2) function then
          found := some (type.getArg! 3, type.getArg! 4)
    return found

/-- A concrete condition together with the frame used by its producer.
The stored frame handles state-only markers. An inlined concrete body may
select its own explicitly represented frame; Given predicates need no adapter. -/
def concreteCondition {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
    {Predicate : Skolems unit → Sort u} (frame : Skolems unit)
    (condition : (current : Skolems unit) → Predicate current) : Predicate frame :=
  condition frame

/-- Find a concrete frame only in a carrier expression explicitly produced
by the concrete adapter. No inverse from arbitrary carriers is assumed. -/
private partial def concreteConditionFrame? (unit functions carriers : Lean.Expr) :
    MetaM (Option Lean.Expr) := do
  let carriers := carriers.consumeMData.headBeta
  if carriers.isAppOfArity ``Skolems.toCarriers 3 then
    unless ← isDefEq unit (carriers.getArg! 0) do return none
    unless ← isDefEq functions (carriers.getArg! 1) do return none
    return some (carriers.getArg! 2)
  if carriers.isAppOfArity ``Carriers.instantiate 3 then
    let some outer ← concreteConditionFrame? unit functions (carriers.getArg! 1) | return none
    let frame := mkAppN (mkConst ``Skolems.instantiate)
      #[unit, functions, carriers.getArg! 0, outer]
    let candidate := mkApp3 (mkConst ``Skolems.toCarriers) unit functions frame
    unless ← withTransparency .default (isDefEq candidate carriers) do return none
    return some frame
  return none

/-- Read a condition in the producer's interpretation. Ready predicates,
including Given predicates, pass unchanged. Only the explicit concrete
adapter permits selecting an inlined body's concrete frame. -/
private def conditionAt (goal : MVarId) (predicate : Lean.Expr)
    (carriers? : Option Lean.Expr) : MetaM Lean.Expr := goal.withContext do
  let predicate := predicate.consumeMData.headBeta
  unless predicate.isAppOfArity ``concreteCondition 5 do return predicate
  let arguments := predicate.getAppArgs
  let frame ← match carriers? with
    | none => pure arguments[3]!
    | some carriers =>
        match ← concreteConditionFrame? arguments[0]! arguments[1]! carriers with
        | some frame => pure frame
        | none => throwError "a concrete condition requires an explicitly concrete carrier frame"
  return mkApp arguments[4]! frame

/-- A predicate over a function's start, applied there: without a recorded
start, it does not read `old`, and its start arguments vanish on
reduction. -/
private def atStart (goal : MVarId) (function predicate : Lean.Expr) (site : Nat)
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

/-- A model's structural hypotheses (`doc/designs/ghost-fields.md`): the
definedness of its invocation meaning and the routing of literal closures
are read by the invocation steps by name, and hidden from the leaf deciders,
which have no use for them. -/
private def hideModelBinders (goal : MVarId) : TacticM MVarId := goal.withContext do
  let hidden := (← getLCtx).foldl (init := #[]) fun found decl =>
    if decl.isImplementationDetail then found
    else if decl.userName == `verexInvokeDefined || decl.userName == `verexInvokeCoherent ||
        decl.userName.toString.startsWith "invokeRoutes_" then found.push (decl.fvarId, true)
    else found
  if hidden.isEmpty then return goal
  markDetails goal hidden

/-- The head of a goal's target, with the actions below it, for the debug
trace. -/
private partial def debugHead (goal : MVarId) : MetaM String := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  let rec heads (e : Lean.Expr) (depth : Nat) : MetaM String := do
    let e := e.consumeMData.headBeta
    let name ← match e.getAppFn with
      | .const name _ => pure (toString name)
      | .fvar id => do return s!"local {(← id.getDecl).userName}"
      | _ => pure "?"
    let name := if e.isAppOfArity ``VerexIR.Proofs.Denote.propheticMeaning 8 then
        s!"{name} {e.getArg! 4}" else name
    if depth == 0 then return name
    let inner := if e.isAppOfArity ``VerexIR.Proofs.wp 7 then some (e.getArg! 3)
      else if e.getAppNumArgs ≥ 1 && (e.getAppFn.isConstOf ``VerexIR.Proofs.Spec.bind ||
          e.getAppFn.isConstOf ``VerexIR.Proofs.Denote.Flow.bind) then
        some (e.getArg! (e.getAppNumArgs - 2))
      else none
    match inner with
    | some inner => return s!"{name} ({← heads inner (depth - 1)})"
    | none => return name
  heads target 4

/-- Retain normal arrivals as proof holes. Abrupt flows unfold their
original postcondition, so they cannot become normal join predecessors. -/
private def sharedExitStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  unless ctx.shared do return false
  let { goal, provenance, clauses, stageStart, .. } := step
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  -- The continuation bound at the fold (`bindStep`) is read through its
  -- definition, one level, as the exit rule reads it (`exitStep`).
  let regions := (← get).regions
  let named? ← goal.withContext do
    let .fvar id := target.getAppFn | pure none
    let some value ← id.getValue? | pure none
    unless regions.any ((← instantiateMVars value).isAppOf ·.node.name) do return none
    pure (some (← zetaDeltaFVars target #[id]).headBeta)
  let (goal, target) ← match named? with
    | some unfolded => pure (← goal.replaceTargetDefEq unfolded, unfolded)
    | none => pure (goal, target)
  let some index := regions.findIdx? (target.isAppOf ·.node.name) | return false
  let region := (← get).regions[index]!
  -- State-label witnesses need the same program points as the original
  -- exit rule. Branch-local points travel in the entry relation, together
  -- with the locals and memory at which they were recorded.
  let arguments := target.getAppArgs
  let goal ← if ctx.labeled && arguments.size == region.node.captures + 2 then
      noteProgramPoint goal arguments[region.node.captures]! arguments.back! true
    else pure goal
  if let some arrival ← SharedContinuations.arrival? region.node goal then
    if region.node.sealed then throwError "an incoming edge reached a sealed continuation"
    modify fun s => { s with regions := s.regions.modify index fun region =>
      { region with
        debits := mergeDebits region.debits s.activeDebits
        arrivalSteps := region.arrivalSteps.push
          { provenance, clauses, owed := s.activeOwed, decisions := s.activeDecisions,
            points := s.activePoints, owners := s.activeOwners }
        node := { region.node with arrivals :=
          region.node.arrivals.push { arrival with summarized := s.activeSummary,
                                                   step := region.arrivalSteps.size } } } }
    charge "join arrival" stageStart
    return true
  let goal ← SharedContinuations.unfold region.node goal
  setGoals [goal]
  normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization #[]
  queue (← getGoals) provenance clauses
  charge "abrupt join exit" stageStart
  return true

/-- Pruning proves only inconsistency of the incoming context. The suffix
is never unfolded or used as a premise, and no failed proof is admitted. -/
private def proveUnreachable (goal : MVarId) : TacticM Unit := goal.withContext do
  let saved ← getGoals
  let restoreCaches ← saveWalkCaches
  try
    let contradiction ← mkFreshExprMVar (mkConst ``False)
    setGoals [contradiction.mvarId!]
    Lean.Elab.Tactic.withoutRecover do
      evalTactic (← `(tactic| (try verex_context_memory_reads) <;> (verex_bounds; omega)))
    unless (← getUnsolvedGoals).isEmpty do throwError "the incoming branch may be reachable"
    let proof ← instantiateMVars contradiction
    if proof.hasMVar || proof.hasSorry then throwError "an unreachable branch needs a closed proof"
    goal.assign (← mkFalseElim (← goal.getType) proof)
  finally
    setGoals saved
    -- The derived facts stayed inside a discarded branch, even on success.
    restoreCaches

/-- An inner join's checks also serve the enclosing suffix. Consult the
live ancestor debits before every attempt: sibling regions must not each
receive another allowance for that same downstream suffix. -/
private def enclosingDebits (regions : Array SharedRegion) (region : SharedRegion) :
    Array SharedBudgets.DebitId := Id.run do
  let mut debits := region.debits
  let mut parent := region.parent
  for _ in [:regions.size] do
    let some name := parent | break
    let some outer := regions.find? (·.node.name == name) | break
    debits := mergeDebits debits outer.debits
    parent := outer.parent
  return debits

private def recordRegionDebit (index : Nat) (debit : SharedBudgets.DebitId) : SolveM Unit := do
  let mut next := some index
  for _ in [: (← get).regions.size] do
    let some current := next | break
    let parent := (← get).regions[current]!.parent
    modify fun s => { s with regions := s.regions.modify current fun region =>
      { region with debits := mergeDebits region.debits #[debit] } }
    let regions := (← get).regions
    next := parent.bind fun name => regions.findIdx? (·.node.name == name)

/-- Optional incoming-edge proofs draw on the downstream leaf allowance.
Before clause allocation, run against the already inherited debit and record
the new physical cost once. Every later clause inherits it, on success or
failure. The temporary ledger is only an accounting view, not a fresh cap. -/
private def underEdgeAllowance (ctx : SolveContext) (index : Nat) (action : TacticM α) :
    SolveM (SharedBudgets.Outcome α × Nat) := do
  let region := (← get).regions[index]!
  if !region.owners.isEmpty then
    return ← runAutomatic ctx.budgets (region.owners.map (·.2)) .preparation action
  let regions := (← get).regions
  let debits := enclosingDebits regions regions[index]!
  let limit := verex.leafHeartbeats.get (← getOptions) * 1000
  let some allowance := (← ctx.budgets.get).afterDebits limit debits
    | throwError "a join lost its earlier preparation debit"
  let pending ← IO.mkRef ({ owners := #[allowance] } : SharedBudgets.Ledger)
  try
    runAutomatic pending #[0] .preparation action (cacheOwners := some #[])
  finally
    let cost := (← pending.get).physical
    if cost > 0 then
      let (ledger, debit) := (← ctx.budgets.get).defer .preparation cost
      ctx.budgets.set ledger
      recordRegionDebit index debit

/-- The points whose hooks wait at an arrival for the suffix's results
(`activePoints`). They run on the suffix, so arrivals waiting at different
points do not share it (`partitionArrivals`). -/
private def arrivalPoints (region : SharedRegion) (arrival : SharedContinuations.Arrival) :
    Array ProgramPointKey :=
  (region.arrivalSteps[arrival.step]?.map (·.points)).getD #[]

/-- What a class's shared suffix owes: the union of the clauses its arrivals
owe, in order of arrival, and their provenance where they agree, else the
region's; and, where the arrivals owe differently, whom its leaves answer
for (`OwedAttribution`). -/
private def classOwed (region : SharedRegion) (arrivals : Array SharedContinuations.Arrival) :
    Option Provenance × Array ObligationRange × Option OwedAttribution :=
  let steps := arrivals.filterMap (region.arrivalSteps[·.step]?)
  if steps.isEmpty then (region.provenance, region.clauses, region.owed) else
  let owing := steps.foldl (init := #[]) fun found step =>
    (attributed step.owed step.provenance step.clauses).foldl
      (fun found entry => if found.contains entry then found else found.push entry) found
  let clauses := owing.foldl (init := #[]) fun found (_, clauses) =>
    clauses.foldl (fun found range => if found.contains range then found else found.push range) found
  let provenance := match owing[0]? with
    | some (first, _) => if owing.all (·.1 == first) then first else region.provenance
    | none => region.provenance
  let attribution := if owing.size == 1 then none
    else some { provenance, clauses, arrivals := owing }
  (provenance, clauses, attribution)

/-- Prepare the region's arrivals for their seal and prune the incoming
edges of the classes that will share (`partitionArrivals`): an arrival that
resumes alone walks its suffix as the default walk would, which refutes
nothing before it. -/
private def pruneShared (ctx : SolveContext) (index : Nat) : SolveM Unit := do
  let region := (← get).regions[index]!
  let start ← IO.getNumHeartbeats
  let node ← SharedContinuations.prepareArrivals region.node (← keptFacts)
  modify fun s => { s with regions := s.regions.modify index ({ · with node }) }
  charge "join preparation" start
  if node.arrivals.size < 2 then return
  let classes ← SharedContinuations.partitionArrivals (arrivalPoints region) node.arrivals
  trace[verex.joinPruning] "{node.name}: {node.arrivals.size} arrivals in classes of {classes.map (·.size)}"
  let shared := (classes.filter (·.size ≥ 2)).flatten
  if shared.isEmpty then return
  let mut retained := #[]
  for arrival in node.arrivals do
    -- Nothing new to refute since the fold.
    if !shared.any (·.goal == arrival.goal) || (← node.unchanged arrival.goal) then
      retained := retained.push arrival
      continue
    let start ← IO.getNumHeartbeats
    let (outcome, cost) ← underEdgeAllowance ctx index (proveUnreachable arrival.goal)
    if cost > 0 then
      modify fun s => { s with pruningChecks := s.pruningChecks + 1 }
    match outcome with
    | .done _ =>
        modify fun s => { s with prunedEdges := s.prunedEdges + 1 }
        trace[verex.joinPruning] "removed incoming edge"
    | .exhausted =>
        retained := retained.push arrival
        trace[verex.joinPruning] "kept incoming edge: allowance exhausted after {cost / 1000}k"
    | .failed failure =>
        retained := retained.push arrival
        trace[verex.joinPruning] "kept incoming edge after {cost / 1000}k: {failure.toMessageData}"
    charge "join feasibility" start
  modify fun s => { s with regions := s.regions.modify index fun current =>
    { current with node := { current.node with arrivals := retained } } }

/-- The stored invariants of an arrival's memory, proved in its context by
`verex_data_invariants`: from the invariants of the memory the arrival
wrote from, which the precondition states of the initial memory and an
enclosing summary states of its memory row (the summary's hypothesis of
`sealNode`), through the writes. A closed proof per arrival, or none when
one arrival does not prove them: the summary then states nothing of its
memory row, as before. -/
-- Retired with doc/plan/data-and-global-invariants.md, "The stored-invariant layer
-- still exists for the declarations the carriers do not state": the carrier replaces it.
private def proveStoredInvariants (ctx : SolveContext) (index : Nat)
    (node : SharedContinuations.Node) : SolveM (Array Lean.Expr) := do
  let shared := (← get).regions.map (·.node.name)
  let mut proofs := #[]
  for arrival in node.arrivals do
    let some invariant ← SharedContinuations.storedInvariantOf? node arrival.arguments shared
      | return #[]
    let start ← IO.getNumHeartbeats
    let (outcome, _) ← underEdgeAllowance ctx index <| arrival.goal.withContext do
      let saved ← getGoals
      let restoreCaches ← saveWalkCaches
      try
        let statement ← SharedContinuations.storedInvariantStatement invariant arrival.arguments.back!
        let held ← mkFreshExprSyntheticOpaqueMVar statement
        setGoals [held.mvarId!]
        Lean.Elab.Tactic.withoutRecover do
          evalTactic (← `(tactic| verex_data_invariants))
          evalTactic (← `(tactic| all_goals verex_assumption))
        unless (← getUnsolvedGoals).isEmpty do throwError "the stored invariants are not established"
        let proof ← instantiateMVars held
        if proof.hasMVar || proof.hasSorry then throwError "the stored invariants need a closed proof"
        pure proof
      finally
        setGoals saved
        restoreCaches
    charge "join invariants" start
    match outcome with
    | .done proof => proofs := proofs.push proof
    | .exhausted => return #[]
    | .failed failure =>
        trace[verex.joinPruning] "stored invariants not proved at an arrival: {failure.toMessageData}"
        return #[]
  return proofs

/-- The names of the regions enclosing `region`, innermost first. -/
private def regionAncestors (regions : Array SharedRegion) (region : SharedRegion) :
    Array Name := Id.run do
  let mut ancestors := #[]
  let mut parent := region.parent
  for _ in [:regions.size] do
    let some name := parent | break
    ancestors := ancestors.push name
    parent := (regions.find? (·.node.name == name)).bind (·.parent)
  return ancestors

/-- The innermost unsealed region no pending goal lies in, its own or a
descendant's: all its arrivals are in. -/
private def completeRegion? (ctx : SolveContext) : SolveM (Option Nat) := do
  let s ← get
  unless ctx.shared && s.regions.any (!·.node.sealed) do return none
  let parents : Std.HashMap Name (Option Name) :=
    s.regions.foldl (fun parents region => parents.insert region.node.name region.parent) {}
  let mut busy : Std.HashSet Name := {}
  for (goal, _) in s.pending do
    let mut current := s.goalRegions[goal]?
    for _ in [:s.regions.size] do
      let some name := current | break
      -- Its ancestors are busy already.
      if busy.contains name then break
      busy := busy.insert name
      current := parents[name]?.join
  let reverseIndex? := s.regions.reverse.findIdx? fun region =>
    !region.node.sealed && !busy.contains region.node.name
  return reverseIndex?.map (s.regions.size - · - 1)

/-- The region at `index` is complete: no pending goal lies in it
(`completeRegion?`), and `pruneShared` prepared its arrivals. Each class of
arrivals owing the same suffix (`partitionArrivals`) becomes a
region of its own: a class of several proves the continuation once, under
the disjunction of its incoming contexts; a single normal predecessor needs
no relation or universal proof, and retaining its concrete state also avoids
artificial joins at reads. -/
private def resumeShared (ctx : SolveContext) (index : Nat) : SolveM Unit := do
  let region := (← get).regions[index]!
  let classes ← SharedContinuations.partitionArrivals (arrivalPoints region) region.node.arrivals
  let classes := if classes.isEmpty then #[#[]] else classes
  modify fun s => { s with
    regions := s.regions.extract 0 index ++
      classes.map (fun arrivals => { region with node := { region.node with arrivals } }) ++
      s.regions.extract (index + 1) s.regions.size
    activeRegion := region.parent
    activeOwners := region.owners
    activeDebits := region.debits
    activeEntryOpened := false
    activeOwed := region.owed }
  -- The enclosing continuation also benefits if this region has no normal
  -- exit. Retain its debit even when there will be no arrival to carry it.
  if let some parent := region.parent then
    modify fun s => { s with regions := s.regions.map fun outer =>
      if outer.node.name == parent then
        { outer with debits := mergeDebits outer.debits region.debits }
      else outer }
  for position in [index : index + classes.size] do
    let start ← IO.getNumHeartbeats
    let node := (← get).regions[position]!.node
    let single := node.arrivals.size == 1
    let node ← if single then pure { node with sealed := true, finished := true }
      else do
        let node ← SharedContinuations.closeArrivals node
        let proofs ← proveStoredInvariants ctx position node
        SharedContinuations.sealNode node proofs ((← get).regions.map (·.node.name))
    -- The invariant attempts' deferred debits reach the suffix's goals.
    modify fun s => { s with
      regions := s.regions.modify position ({ · with node })
      activeDebits := s.regions[position]!.debits }
    if let some summary := node.summary? then
      let (_, goal) ← summary.introNP (node.summaryArguments.size + node.hypotheses + 1)
      let goal ← SharedContinuations.unfold node goal
      setGoals [goal]
      normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization #[]
      -- The suffix of a class of several is shared work: its clauses take
      -- owners and link the relation's rows at their leaves. It continues
      -- what every arrival decided before the fold, and owes what its
      -- arrivals owe, which they share.
      let points := node.arrivals[0]?.map (arrivalPoints region) |>.getD #[]
      let (provenance, clauses, owed) := classOwed region node.arrivals
      modify fun s => { s with activeSummary := true, activeOwners := region.owners,
                               activeDecisions := region.decisions, activePoints := points,
                               activeOwed := owed }
      queue (← getGoals) provenance clauses
    else if single then
      -- The exit rule's form: the arguments and the conditions passed on
      -- are normal already (`exitStep`).
      let arrival := node.arrivals[0]!
      let goal ← SharedContinuations.unfoldArrival node arrival
      let goal ← bindStateEntry goal arrival.arguments.back!
      setGoals [goal]
      normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization
        (arrival.arguments ++ region.conditions)
      -- The node's pins were just lifted: the equations they held
      -- substitute once for the suffix, as the default walk substituted
      -- them where they arose.
      timedStep "single predecessor/substitute"
        (evalTactic (← `(tactic| all_goals verex_subst_equations)))
      -- The predecessor's own path continues: under a summary where the
      -- arrival came from inside one, with what it owes and decided.
      let some step := region.arrivalSteps[arrival.step]?
        | throwError "an arrival lost its step"
      modify fun s => { s with activeSummary := arrival.summarized, activeOwners := step.owners,
                               activeDecisions := step.decisions, activePoints := step.points,
                               activeOwed := step.owed }
      queue (← getGoals) step.provenance step.clauses
    charge (if single then "single predecessor" else "join summary") start

/-- Successor certificates discharge predecessor holes. No open summary may
be silently dropped, including one containing a reported failed obligation. -/
private def finishShared : SolveM Unit := do
  let start ← IO.getNumHeartbeats
  while (← get).regions.any (!·.node.finished) do
    let mut progress := false
    for index in [: (← get).regions.size] do
      let region := (← get).regions[index]!
      if region.node.finished then continue
      if let some node ← SharedContinuations.finish? region.node then
        modify fun s => { s with regions := s.regions.set! index { region with node := node } }
        progress := true
    unless progress do throwError "shared continuation proofs have unresolved dependencies"
  charge "join proofs" start

/-- An exit of a loop reaches its continuation, which is unfolded here, at
the one place the rest of the function is needed. -/
private def exitStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let foldedNormal := (← get).foldedNormal
  let exit? ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let .fvar id := target.getAppFn | pure none
    let declaration ← id.getDecl
    if declaration.isLet && declaration.userName == continuationName then
      pure (some ((← zetaDeltaFVars target #[id]).headBeta,
        target.getAppArgs ++ (foldedNormal.get? id).getD #[], id))
    else pure none
  let some (unfolded, values, continuation) := exit? | return false
  if ctx.profile then
    modify fun s => { s with continuations := s.continuations.arrived continuation }
  let goal ← goal.replaceTargetDefEq unfolded
  -- The continuation's value and state are a program point (the folded
  -- conditions of a branching action follow them), kept for a target
  -- whose contract binds a state label.
  let goal ← match ctx.labeled, values[0]?, values[1]? with
    | true, some value, some state => noteProgramPoint goal value state ctx.shared
    | _, _, _ => pure goal
  -- In a step proof, the memory the path continues with is the state of the
  -- steps after it.
  let goal ← match values[1]? with
    | some state => bindStateEntry goal state
    | none => pure goal
  setGoals [goal]
  -- The values the continuation is applied to, and the conditions it
  -- passes on, are normal already.
  normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization values
  if ctx.debug then
    let head := match unfolded.getAppFn with
      | .const name _ => toString name
      | _ => "?"
    let action := if unfolded.isAppOfArity ``VerexIR.Proofs.wp 7 then
        match (unfolded.getArg! 3).consumeMData.headBeta.getAppFn with
        | .const name _ => s!" of {name}"
        | _ => " of ?"
      else ""
    logInfo m!"exit into {head}{action}: {((← IO.getNumHeartbeats) - stageStart) / 1000}k"
  queue (← getGoals) provenance clauses
  charge "exit" stageStart
  return true

/-- An in-body assumption: what follows assumes the condition the meanings
read for its site. -/
private def assumeStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let some arguments ← assumeGoal? goal | return false
  let next ← goal.withContext (goal.apply
    (mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_assumeAt) arguments))
  let [continues] := next | throwError "the assumption rule did not leave its continuation"
  let (assumed, continues) ← continues.intro `assumptionHolds
  setGoals [continues]
  let holdsIdent := mkIdent `assumptionHolds
  -- The table the meanings read the condition from, at the site.
  let table? ← continues.withContext do
    let holds ← instantiateMVars (← assumed.getType)
    let entry := holds.getAppFn.constName? |>.bind fun name =>
      if name == ``VerexIR.Proofs.Denote.assumptionOf then holds.getAppArgs[3]? else none
    pure (entry.bind (·.getAppFn.constName?))
  if let some table := table? then
    unless table == ``VerexIR.Proofs.Denote.Meanings.assumption ||
        table == ``VerexIR.Proofs.Denote.InterpreterOps.assumption do
      evalTactic (← `(tactic| try simp only [$(mkIdent table):ident] at $holdsIdent:ident))
  evalTactic (← `(tactic| try simp only [VerexIR.Proofs.Denote.closedMeanings,
    VerexIR.Proofs.Denote.assumptionOf_none, VerexIR.Proofs.Denote.assumptionOf_same,
    ↓reduceIte, Nat.reduceEqDiff] at $holdsIdent:ident))
  evalTactic (← `(tactic| try verex_normalize at $holdsIdent:ident))
  let cut ← cutHypothesis? `assumptionHolds
  evalTactic (← `(tactic| all_goals verex_split_hypotheses))
  evalTactic (← `(tactic| all_goals verex_subst_vars))
  renormalizeCut ctx.normalization cut
  queue (← getGoals) provenance clauses
  charge "assume" stageStart
  return true

/-- A call owing invariants where it returns is a cut there; any other is
its meaning. -/
private def callAtStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let some (site, arguments) ← callAtGoal? goal | return false
  let arguments := arguments.modify 3 (·.consumeMData.headBeta)
  let rule ← match ctx.invariants.find? (·.1 == site) with
    | some (_, function, condition) => do
        let owed ← atStart goal function (← conditionAt goal condition none) site #[]
        pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_callAt_checked)
          (arguments.extract 0 4 ++ #[owed] ++ arguments.extract 4 7))
    | none => pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_callAt) arguments)
  let next ← goal.withContext (goal.apply rule)
  setGoals next
  -- Removing the marker exposes the interpreter adapter's action. Normalize
  -- that computation before the bind walker treats it as already normal,
  -- keeping the caller's conditions and their source markers intact.
  normalizeGoals ctx (arguments.extract 4 7)
  queue (← getGoals) provenance clauses (callSite := site)
  charge "call site" stageStart
  return true

/-- An abort at its site the normal form has not reached: the abort condition
at the failure, marked with the site (`wp_abortAt`), which the walker reads
where it steps the goal next (`abortSite?`). -/
private def abortStep (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let some _ ← abortAtGoal? goal | return false
  setGoals [goal]
  evalTactic (← `(tactic| rw [VerexIR.Proofs.Denote.wp_abortAt]))
  queue (← getGoals) provenance clauses
  charge "abort" stageStart
  return true

/-- A marker on what a path owes (`doc/designs/proof-language.md`, "Obligation
addressing and provenance"): an abort at a site, a branch of a conditional, an
arm of a `match`, or the continuation after an operation a proof names. -/
private inductive PathMark where
  | abort (site : Nat)
  | branch (site : Nat) (taken : Bool)
  | arm (site : Nat) (variant : String)
  | after (site : Nat)
  | dispatch (site : Nat) (target : String)

/-- A goal's conclusion through branches and arms, at aborts and after
operations (`Taken`, `Matched`, `AbortsAt`, `AfterAt`): its markers,
outermost first, and the conclusion without them. The premises on the way to
a decision's marker are the path's decisions (`isPathDecision`). -/
private partial def pathMarkers? (e : Lean.Expr) :
    MetaM (Option (Array PathMark × Lean.Expr)) := do
  match e with
  | .forallE name domain body info =>
      let some (marks, body) ← pathMarkers? body | return none
      let decides := !body.hasLooseBVar 0 && marks.any fun
        | .after _ => false
        | _ => true
      let name ← if !decides then pure name
        else if name.hasMacroScopes then mkFreshUserName pathDecisionName
        else do
          (← targetState).decisionNames.modify (·.insert name)
          pure name
      return some (marks, .forallE name domain body info)
  | .mdata data body =>
      return (← pathMarkers? body).map fun (marks, body) => (marks, .mdata data body)
  | _ =>
    let (mark, inner) ← if e.isAppOfArity ``AbortsAt 2 then
        pure ((← siteOf? (e.getArg! 0)).map PathMark.abort, e.getArg! 1)
      else if e.isAppOfArity ``Taken 3 then
        let taken := (e.getArg! 1).consumeMData.isConstOf ``Bool.true
        pure ((← siteOf? (e.getArg! 0)).map (PathMark.branch · taken), e.getArg! 2)
      else if e.isAppOfArity ``Matched 3 then
        let variant := match (e.getArg! 1).consumeMData with
          | .lit (.strVal variant) => variant
          | _ => "_"
        pure ((← siteOf? (e.getArg! 0)).map (PathMark.arm · variant), e.getArg! 2)
      else if e.isAppOfArity ``AfterAt 2 then
        pure ((← siteOf? (e.getArg! 0)).map PathMark.after, e.getArg! 1)
      else if e.isAppOfArity ``Dispatched 3 then
        let target := match (e.getArg! 1).consumeMData with
          | .lit (.strVal target) => target
          | _ => "_"
        pure ((← siteOf? (e.getArg! 0)).map (PathMark.dispatch · target), e.getArg! 2)
      else return none
    let some mark := mark | return none
    return match ← pathMarkers? inner with
      | some (marks, body) => some (#[mark] ++ marks, body)
      | none => some (#[mark], inner)

/-- The decision a path through a conditional takes: the source's `if`, the
heading of a `while` loop, whose `then` continues it, or the test of an arm
of a lowered `match`, whose `then` takes the arm and whose `else` may be
the match's last arm. -/
private def SolveContext.branchDecision (ctx : SolveContext) (site : Nat) (taken : Bool) :
    Option Decision :=
  match ctx.siteNumber "if" site, ctx.siteNumber "loopHeader" site,
      ctx.siteEntry (if taken then "matchArm" else "matchElse") site with
  | some number, _, _ => some (.branch number taken)
  | none, some number, _ => some (.loop number !taken)
  | none, none, some (number, variant) => some (.variant number variant)
  | none, none, none => none

/-- Where a path owing an abort condition at a site aborts: at a throw, an
operation, an invocation, or a call of the target's own body. -/
private def SolveContext.abortDecision (ctx : SolveContext) (site : Nat) : Option Decision :=
  match ctx.siteNumber "abort" site, ctx.siteNumber "op" site, ctx.siteNumber "invoke" site,
      ctx.callAbortSite site with
  | some number, _, _, _ => some (.aborts (.abort number))
  | none, some number, _, _ => some (.aborts (.op number))
  | none, none, some number, _ => some (.aborts (.invoke number))
  | none, none, none, some call => some (.aborts call)
  | none, none, none, none => none

/-- The invariants a call owes where it returns, as a cut. -/
private def callCheckedStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let some (owed, rest) ← callCheckedGoal? goal | return false
  match ← goal.withContext (goal.apply
      (mkApp2 (mkConst ``VerexIR.Proofs.Denote.CallChecked.intro) owed rest)) with
  | [holds, continues] =>
      setGoals [continues]
      evalTactic (← `(tactic| intro invariantHolds))
      evalTactic (← `(tactic| try simp only [VerexIR.Proofs.Obligation_eq] at invariantHolds))
      evalTactic (← `(tactic| try verex_normalize at invariantHolds))
      let cut ← cutHypothesis? `invariantHolds
      evalTactic (← `(tactic| all_goals verex_split_hypotheses))
      evalTactic (← `(tactic| all_goals verex_subst_vars))
      renormalizeCut ctx.normalization cut
      queue (← getGoals) provenance clauses (callSite := step.callSite)
      setGoals [holds]
      normalizeGoals ctx
      queue (← getGoals) (some .callReturned) #[]
  | _ => throwError "the call check did not produce its two obligations"
  charge "call returned" stageStart
  return true

/-- The subterms of a normal target that a split leaves normal in a case:
those reading no local a hypothesis the case added mentions, as the
split's condition (`splitStep`). Each shared subterm is visited once. -/
private def unaffectedParts (target : Lean.Expr) (affected : FVarIdSet) : Array Lean.Expr := Id.run do
  -- Post-order with memo: whether a subterm reads an affected local.
  let mut reads : Std.HashMap Lean.Expr Bool := {}
  let mut parts : Array Lean.Expr := #[]
  let mut pending : Array (Lean.Expr × Bool) := #[(target, false)]
  while let some (e, expanded) := pending.back? do
    pending := pending.pop
    if reads.contains e then continue
    let children : Array Lean.Expr := match e with
      | .app f a => #[f, a]
      | .lam _ t b _ | .forallE _ t b _ => #[t, b]
      | .letE _ t v b _ => #[t, v, b]
      | .mdata _ b | .proj _ _ b => #[b]
      | _ => #[]
    if !expanded && children.any (!reads.contains ·) then
      pending := pending.push (e, true)
      for child in children do
        unless reads.contains child do pending := pending.push (child, false)
      continue
    let own := match e with
      | .fvar id => affected.contains id
      | _ => false
    let read := own || children.any fun child => reads.getD child false
    reads := reads.insert e read
    if !read && !children.isEmpty && !e.hasLooseBVars then parts := parts.push e
  return parts

/-- The locals of a fold context that sharing reads, which the walk keeps
until the node's arrivals are in: the captures the arrivals are compared at
(`commonCaptures`), and the locals of the facts over a capture or a local
definition matched by identity, which the seal keeps as common context
(`commonLocals`). A frame equation between other locals substitutes on the
walk as without sharing. -/
private def foldPins (node : SharedContinuations.Node) (definitions : Std.HashSet FVarId) :
    MetaM FVarIdSet := do
  let lctx ← getLCtx
  let read (id : FVarId) : Bool := node.abstracted.contains id || definitions.contains id
  let mut pins : FVarIdSet := {}
  for id in node.abstracted do
    if let some decl := lctx.find? id then
      unless decl.isLet do pins := pins.insert id
  for decl in lctx do
    if decl.isImplementationDetail || decl.isLet then continue
    unless ← isProp decl.type do continue
    let type ← instantiateMVars decl.type
    let mentioned := (collectFVars {} type).fvarIds
    unless mentioned.any read do continue
    for id in mentioned do
      if let some other := lctx.find? id then
        unless other.isLet do pins := pins.insert id
  return pins
/-- A bind rule: a straight-line action keeps its continuation in place,
since it reaches it on one path; a branching one has it folded. -/
private def bindStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let some (unfolded, branches, normal) ← bindRule? goal | return false
  let ruled ← IO.getNumHeartbeats
  if branches then
    -- Reassociation can expose another branching bind with the very same
    -- postcondition. Its existing node already owns that continuation;
    -- wrapping it again duplicates telescopes and program-point captures.
    let regions := (← get).regions
    let named ← if ctx.shared then unfolded.withContext do
        let post := (← instantiateMVars (← unfolded.getType)).getArg! 4
        return post.isFVar || regions.any (post.isAppOf ·.node.name)
      else pure false
    -- A node needs two normal exits to join, and a suffix to share.
    let shared ← if ctx.shared && !named then do
        if ← SharedContinuations.mayJoin normal[0]! <&&> unfolded.withContext
            (SharedContinuations.carriesProgram (regions.map (·.node.name)) normal[1]!) then
          SharedContinuations.fold? unfolded (← get).opaqueDefinitions
        else pure none
      else pure none
    let (folded, continuation?) ← match shared with
      | some (folded, node) => do
          -- The application is bound as a local definition, as the default
          -- walk binds its continuation (`foldContinuation`): normalization
          -- does not enter it at each step until an arrival reads it.
          let (folded, named?) ← foldContinuation folded
          let node ← match named? with
            | some id => folded.withContext do
                pure { node with origin := node.origin.insert id (← instantiateMVars (← id.getType)) }
            | none => pure node
          -- A branch on the way to the node does not rewrite the context
          -- the node is folded in: its arrivals then share that context,
          -- with what each branch decided as an equation.
          let pins ← unfolded.withContext (foldPins node (← get).opaqueDefinitions)
          (← targetState).sharedPins.modify fun pinned => pins.foldl (·.insert ·) pinned
          modify fun s => { s with
            regions := s.regions.push
              { node, parent := s.activeRegion, provenance, clauses, owed := s.activeOwed,
                owners := s.activeOwners, debits := s.activeDebits,
                decisions := s.activeDecisions, points := s.activePoints, pins,
                conditions := normal.extract 2 4 }
            activeRegion := some node.name }
          pure (folded, none)
      | none => if named then pure (unfolded, none) else foldContinuation unfolded
    setGoals [folded]
    -- The conditions the continuation passes on are normal, where it
    -- is unfolded too.
    if let some continuation := continuation? then
      modify fun s =>
        { s with foldedNormal := s.foldedNormal.insert continuation (normal.extract 2 4) }
      if ctx.profile then
        modify fun s => { s with continuations := s.continuations.folded continuation false }
  else setGoals [unfolded]
  let folded ← IO.getNumHeartbeats
  let beforeNormal ← getGoals
  -- The newly named postcondition is as normal as the lambda it replaced.
  -- Preserve its capture prefix, just as the old walker preserves a local
  -- continuation, instead of renormalizing every capture at each bind.
  let normal ← if ctx.shared then do
      let goal ← getMainGoal
      let post ← goal.withContext do return (← instantiateMVars (← goal.getType)).getArg! 4
      pure (normal.push post)
    else pure normal
  normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization normal
  let changed := (← getGoals) != beforeNormal
  if ctx.debug then
    let target ← instantiateMVars (← goal.getType)
    let action := if target.isAppOfArity ``VerexIR.Proofs.wp 7 then
        match (target.getArg! 3).consumeMData.headBeta.getAppArgs.back?.map
            (·.consumeMData.headBeta.getAppFn) with
        | some (.const name _) => toString name
        | _ => "?"
      else "?"
    logInfo m!"bind {action}{if branches then " (branching)" else ""}: rule \
      {(ruled - stageStart) / 1000}k, fold {(folded - ruled) / 1000}k, normalize \
      {((← IO.getNumHeartbeats) - folded) / 1000}k{if changed then "" else " (unchanged)"}"
  queue (← getGoals) provenance clauses
  charge "bind" stageStart
  return true

/-- A step of an unrolled loop: its next iteration while its allowance
lasts, else the bound's leaf. -/
private def unrollStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let unrolling ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``loopUnroll 9 do return none
    let some remaining ← (evalNat (action.getArg! 5)).run
      | throwError "loop unrolling requires a concrete allowance"
    return some (action.getAppArgs, remaining,
      #[target.getArg! 4, target.getArg! 5, target.getArg! 6])
  let some (arguments, remaining, conditions) := unrolling | return false
  let rule := if remaining == 0 then
      mkAppN (mkConst ``wp_loopUnroll_exhausted)
        (arguments.extract 0 5 ++ arguments.extract 6 9 ++ conditions)
    else
      mkAppN (mkConst ``wp_loopUnroll_step)
        (arguments.extract 0 5 ++ #[toExpr (remaining - 1)] ++
          arguments.extract 6 9 ++ conditions)
  let next ← goal.withContext (goal.apply rule)
  setGoals next
  if remaining > 0 then
    evalTactic (← `(tactic| intro verexLoopFuel))
    normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization (#[arguments[8]!] ++ conditions)
    queue (← getGoals) provenance clauses
  else
    let site ← siteOf? arguments[4]!
    let origin := match ctx.unrolls.find? (some ·.1 == site) with
      | some (_, bound) => some (Provenance.unrollBound bound)
      | none => provenance
    queue (← getGoals) origin #[]
  charge "unroll step" stageStart
  return true

/-- The propositions of a goal's hypotheses. -/
private def propositionsOf (goal : MVarId) : MetaM (Std.HashSet Lean.Expr) := goal.withContext do
  let mut propositions := {}
  for fvarId in ← goal.getNondepPropHyps do
    propositions := propositions.insert (← instantiateMVars (← fvarId.getType))
  return propositions

/-- `normalizeContextStage` on each goal, over its hypotheses whose
proposition is not among `before`; a goal it fails on stays as it was. -/
private def normalizeNewHypotheses (before : Std.HashSet Lean.Expr) : TacticM Unit := do
  let mut normalized := #[]
  for goal in ← getGoals do
    setGoals [goal]
    let current ← goal.withContext do
      let mut current : FVarIdSet := {}
      for fvarId in ← goal.getNondepPropHyps do
        unless before.contains (← instantiateMVars (← fvarId.getType)) do
          current := current.insert fvarId
      pure current
    let saved ← saveState
    try normalizeContextStage current.contains
    catch _ => saved.restore
    normalized := normalized ++ (← getGoals).toArray
  setGoals normalized.toList

/-- A loop: unrolled where its site has a bound, else by its invariant,
with the loop hypothesis stating it at each back edge and the iteration
assuming it. -/
private def loopStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let some (site, arguments) ← loopGoal? goal | return false
  -- The hooks at the loop's entry run on the goal the loop is applied to.
  if let some loop := ctx.siteNumber "loop" site then
    let point := ProgramPointKey.loopEntry loop
    if ctx.points.any (·.1.selects point) && !(← get).pointsDone.contains (goal, point) then
      setGoals [goal]
      atPoint ctx point
      let goals ← getGoals
      modify fun s => { s with pointsDone := s.pointsDone ++ goals.toArray.map (·, point) }
      queue goals provenance clauses (callSite := step.callSite)
      return true
  if let some (_, bound) := ctx.unrolls.find? (·.1 == site) then
    let rule := mkAppN (mkConst ``wp_loopAt_unroll)
      (arguments.extract 0 5 ++ #[toExpr bound] ++ arguments.extract 5 10)
    setGoals (← goal.withContext (goal.apply rule))
    evalTactic (← `(tactic| intro verexLoopFuel))
    queue (← getGoals) provenance clauses
    charge "unroll entry" stageStart
    return true
  let some (_, function, invariant) := ctx.invariants.find? (·.1 == site)
    | throwError m!"no invariant for the loop at site {site}"
  -- The predicate uses the loop's interpretation; an explicit concrete
  -- adapter selects the frame of an inlined concrete callee.
  let invariant ← atStart goal function (← conditionAt goal invariant (some arguments[0]!)) site
    #[arguments[6]!, arguments[9]!]
  -- The invariant normalized once, as a predicate: the loop hypothesis
  -- states it at each back edge and the iteration assumes it, and the
  -- rule holds of any predicate.
  let invariant ← goal.withContext do
    let (simpCtx, simprocs) := ctx.normalization
    let predicate ← forallTelescope (← whnfR (← inferType invariant)) fun binders _ => do
      mkLambdaFVars binders (mkAppN invariant binders).headBeta
    let (result, _) ← Simp.main predicate simpCtx (methods := Simp.mkDefaultMethodsCore simprocs)
    pure result.expr
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
  modify fun s => { s with opaqueDefinitions := s.opaqueDefinitions.insert continuation }
  if ctx.profile then
    modify fun s => { s with continuations := s.continuations.folded continuation true }
  let before ← propositionsOf goal
  let rule := mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_loopAt)
    (arguments.extract 0 7 ++ #[invariant] ++ arguments.extract 7 10)
  let subgoals ← goal.withContext (goal.apply rule)
  match subgoals with
  | [entryHolds, step] =>
      let stepStart ← IO.getNumHeartbeats
      let mut marks : Array (String × Nat) := #[]
      setGoals [step]
      evalTactic (← `(tactic| intro recursive loopHypothesis env state loopInvariant))
      -- The loop hypothesis and the invariant assumed are instances of the
      -- invariant, normalized once; the assumed one's clauses become facts
      -- under their names when the hypotheses are split. Each replacement
      -- leaves the other hypotheses in place.
      let mut step ← getMainGoal
      let introduced ← step.withContext do
        let locals := (← getLCtx).getFVarIds
        return locals.extract (locals.size - 5)
      -- The back edges of the iteration are keyed by the loop.
      modify fun s => { s with recursiveLoops := s.recursiveLoops.insert introduced[0]! (ctx.loopKey site) }
      let mut closed := false
      for (hypothesis, unwrap) in [(introduced[4]!, true), (introduced[1]!, false)] do
        match ← normalizeAroundClauses ctx.normalization step hypothesis false with
        | some next =>
            step := next
            if unwrap then
              -- The clauses, normal with their markers, are recorded under
              -- their names before the markers go.
              recordAssumedClauses ctx.normalization step hypothesis
              match ← normalizeAroundClauses ctx.normalization step hypothesis true with
              | some next => step := next
              | none => closed := true; break
        | none => closed := true; break
      setGoals (if closed then [] else [step])
      unless closed do
        -- The iteration is applied to the entry row: its slots are the
        -- row's projections, reduced before the normalization reads them,
        -- which would otherwise derive the row's type at each.
        evalTactic (← `(tactic| verex_reduce_projections))
        normalizeGoals ctx
      marks := marks.push ("normalize 1", ← IO.getNumHeartbeats)
      -- The invariant's equations on the locals are substituted before
      -- the iteration is traversed, so the traversal sees their values.
      evalTactic (← `(tactic| all_goals verex_split_hypotheses))
      evalTactic (← `(tactic| all_goals verex_subst_vars))
      marks := marks.push ("split/subst 1", ← IO.getNumHeartbeats)
      -- The rule's part is charged before the context stage, which an
      -- exhausted budget would otherwise leave unattributed.
      charge "loop" stageStart
      let contextStart ← IO.getNumHeartbeats
      -- The hypotheses the rule introduced or rewrote, so that every leaf of
      -- the iteration inherits them normal, with the equations the
      -- normalization exposes substituted. The others are in the walk's
      -- form already.
      let contextMarks ← IO.mkRef marks
      let mark (label : String) : TacticM Unit := do
        contextMarks.modify (·.push (label, ← IO.getNumHeartbeats))
      timedStep "loop context" do
        normalizeNewHypotheses before
        mark "normalize 2"
        evalTactic (← `(tactic| all_goals verex_split_hypotheses))
        evalTactic (← `(tactic| all_goals verex_subst_vars))
        mark "split/subst 2"
        normalizeNewHypotheses before
        mark "normalize 3"
      marks ← contextMarks.get
      if ctx.debug then
        let mut previous := stepStart
        let mut report := m!"loop step at site {site}:"
        for (label, mark) in marks do
          report := report ++ m!" {label} {(mark - previous) / 1000}k;"
          previous := mark
        logInfo m!"{report} {(← getGoals).length} goals"
      -- The hooks at the start of an iteration, once for all its paths.
      if let some loop := ctx.siteNumber "loop" site then atPoint ctx (.loopStep loop)
      queue (← getGoals) provenance clauses
      setGoals [entryHolds]
      normalizeGoals ctx
      queue (← getGoals) (some (.loopEntry (ctx.loopKey site))) #[]
      charge "loop context" contextStart
  | _ => throwError "the loop rule did not produce its two obligations"
  return true

/-- An assertion, a cut: the condition is owed at its site, and assumed
after it. -/
private def assertionStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let some (site, arguments) ← assertionGoal? goal | return false
  let some (_, function, condition) := ctx.invariants.find? (·.1 == site)
    | throwError m!"no condition for the assertion at site {site}"
  let condition ← resolveAnchors goal
    (← atStart goal function (← conditionAt goal condition (some arguments[0]!)) site #[])
  let rule := mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_assertAt)
    (arguments.extract 0 6 ++ #[condition] ++ arguments.extract 6 9)
  match ← goal.withContext (goal.apply rule) with
  | [holds, continues] =>
      setGoals [continues]
      let (_, continued) ← (← getMainGoal).intro `assertionHolds
      let holdsIdent := mkIdent `assertionHolds
      replaceMainGoal [continued]
      evalTactic (← `(tactic| try simp only [VerexIR.Proofs.Obligation_eq] at $holdsIdent:ident))
      evalTactic (← `(tactic| try verex_normalize at $holdsIdent:ident))
      -- The lemmas a proof step applies give their conclusions.
      unless (← getGoals).isEmpty do
        let continued ← getMainGoal
        let holdsHypothesis ← continued.withContext do
          pure ((← getLCtx).findFromUserName? `assertionHolds |>.map (·.fvarId))
        if let some hypothesis := holdsHypothesis then
          replaceMainGoal [← replaceByLemmaFacts continued hypothesis]
          evalTactic (← `(tactic| try verex_normalize at $holdsIdent:ident))
      -- An assertion that normalizes to `False` closes what follows.
      unless (← getGoals).isEmpty do
        -- The continuation is normalized where it is applied, as after
        -- a step; the row and state it is applied to are normal.
        let cut ← cutHypothesis? `assertionHolds
        normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization #[arguments[5]!, arguments[8]!]
        evalTactic (← `(tactic| all_goals verex_split_hypotheses))
        evalTactic (← `(tactic| all_goals verex_subst_vars))
        renormalizeCut ctx.normalization cut
        -- A split step continues once per case, each normalized under
        -- its case.
        let before ← getGoals
        let cases ← before.flatMapM fun goal => liftM (splitCases goal)
        setGoals cases
        if cases.length != before.length then
          evalTactic (← `(tactic| all_goals (try verex_normalize at *)))
        queue (← getGoals) provenance clauses
      -- What a proof step owes: a lemma's premise for each application.
      setGoals [holds]
      normalizeGoals ctx
      let owed ← (← getGoals).toArray.flatMapM fun goal => liftM (dischargeLemmaSteps goal)
      for (part, premise) in owed do
        setGoals [part]
        -- A premise is unfolded, so normalized again.
        if premise then normalizeGoals ctx
        queue (← getGoals) (some (if premise then .lemmaRequirement else .assertion)) #[]
  | _ => throwError "the assertion rule did not produce its two obligations"
  charge "assertion" stageStart
  return true

/-- A write of memory: a cut over the invariants the write owes, the memory
before it read from its anchor; a write whose function owes none there
passes, and so does a loan's death after a branch that did not borrow,
which ends nothing on this path (no anchor of the write is recorded). -/
private def memoryWrittenStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let some (site, arguments) ← memoryWrittenGoal? goal | return false
  let nothingOwed : SolveM Lean.Expr := goal.withContext do
    withLocalDeclD `env (← inferType arguments[5]!) fun env =>
    withLocalDeclD `state arguments[1]!
      fun state =>
      mkLambdaFVars #[env, state] (mkConst ``True)
  -- A write of an inlined callee owes nothing where it ends.
  let (condition, owed) ← match ctx.invariants.find? (·.1 == site) with
    | some (_, function, condition) =>
        if ← anchorRecorded goal site then
          pure (← resolveAnchors goal
            (← atStart goal function (← conditionAt goal condition (some arguments[0]!)) site #[]), true)
        else pure (← nothingOwed, false)
    | none => pure (← nothingOwed, false)
  let rule := mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_memoryWrittenAt)
    (arguments.extract 0 6 ++ #[condition] ++ arguments.extract 6 9)
  match ← goal.withContext (goal.apply rule) with
  | [holds, continues] =>
      setGoals [continues]
      evalTactic (← `(tactic| intro invariantHolds))
      evalTactic (← `(tactic| try simp only [VerexIR.Proofs.Obligation_eq] at invariantHolds))
      evalTactic (← `(tactic| try verex_normalize at invariantHolds))
      unless (← getGoals).isEmpty do
        let cut ← cutHypothesis? `invariantHolds
        normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization #[arguments[5]!, arguments[8]!]
        evalTactic (← `(tactic| all_goals verex_split_hypotheses))
        evalTactic (← `(tactic| all_goals verex_subst_vars))
        renormalizeCut ctx.normalization cut
        queue (← getGoals) provenance clauses
      if owed then
        setGoals [holds]
        normalizeGoals ctx
        queue (← getGoals) (some .memoryWritten) #[]
      else holds.assign (mkConst ``True.intro)
  | _ => throwError "the memory write rule did not produce its two obligations"
  charge "memory" stageStart
  return true

/-- A ghost write, a cut: the value is owed in the field's domain at the
update, and what follows takes it. -/
private def ghostWriteStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let some (site, arguments) ← ghostWriteGoal? goal | return false
  let rule := mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_ghostAt) arguments
  match ← goal.withContext (goal.apply rule) with
  | [defined, continues] =>
      setGoals [continues]
      evalTactic (← `(tactic| intro ghostValue ghostChosen))
      evalTactic (← `(tactic| try verex_normalize at ghostChosen))
      unless (← getGoals).isEmpty do
        normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization #[arguments[7]!, arguments[10]!]
        evalTactic (← `(tactic| all_goals verex_split_hypotheses))
        evalTactic (← `(tactic| all_goals verex_subst_vars))
        queue (← getGoals) provenance clauses
      setGoals [defined]
      normalizeGoals ctx
      let located := (ctx.ghostWrites.filter (·.1 == site)).map (·.2)
      queue (← getGoals) (some .ghostWrite) located
  | _ => throwError "the ghost write rule did not produce its two obligations"
  charge "ghost" stageStart
  return true

/-- A data invariant where a mutation of a local ends, and of a value where
it is constructed: cuts, as an assertion is. -/
private def dataInvariantStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  if let some opened ← decodedOwnGoal? goal then
    setGoals [opened]
    normalizeGoals ctx
    queue (← getGoals) provenance clauses
    charge "invariant" stageStart
    return true
  -- A declaration's data invariant of an encoding, wherever the walk meets
  -- one: the goal a cut leaves, and the conjunct an outer declaration's
  -- clauses leave of a nested value, which is a goal of its own by then.
  -- Its premises introduced, what is owed is the declaration's clauses,
  -- which the normalization reduces (`verex_own_encoded`).
  if ← goal.withContext do
      pure ((← instantiateMVars (← goal.getType)).isAppOf
        ``VerexIR.Proofs.Denote.OwnAtArguments) then
    setGoals [goal]
    evalTactic (← `(tactic| verex_own_encoded))
    normalizeGoals ctx
    queue (← getGoals) provenance clauses
    charge "invariant" stageStart
    return true
  -- A certifying step: the obligation is the value's data invariants at
  -- every depth, and the site and the kind are the step's own
  -- (`certifyingAt`, `doc/designs/data-invariants.md`).
  let certifying? ← certifyingGoal? goal
  let check ← match certifying? with
    | some _ => pure none
    | none => match ← mutationEndGoal? goal with
      | some (site, arguments) => pure (some (site, arguments, false))
      | none => match ← constructionGoal? goal with
        | some (site, arguments) => pure (some (site, arguments, true))
        | none => pure none
  if check.isNone && certifying?.isNone then return false
  let (rule, marks, origin, site) ← match certifying? with
    | some (site, kind, arguments) =>
        let origin ←
          if kind.isConstOf ``VerexIR.Proofs.Denote.CertifyKind.construction then
            pure Provenance.construction
          else if kind.isConstOf ``VerexIR.Proofs.Denote.CertifyKind.mutationEnd then
            pure Provenance.mutationEnd
          else if kind.isConstOf ``VerexIR.Proofs.Denote.CertifyKind.copyOut then
            pure Provenance.copyOut
          else if kind.isConstOf ``VerexIR.Proofs.Denote.CertifyKind.handOver then
            pure Provenance.handOver
          else if kind.isConstOf ``VerexIR.Proofs.Denote.CertifyKind.exit then
            pure Provenance.exitMutationEnd
          else throwError m!"a certifying step at the site {site} with an unknown kind {kind}"
        pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_certifyingAt_intro) arguments,
          #[arguments[arguments.size - 1]!], origin, site)
    | none =>
      let some (site, arguments, constructed) := check | return false
      -- The predicate a `constructedAt` or `mutationEndAt` marker checks.
      -- Where the unit states none the marker checks nothing: the obligation
      -- is the certifying step's, and these two markers retire with the open
      -- problem `doc/plan/data-and-global-invariants.md`, "`constructedAt` and
      -- `mutationEndAt` are emitted where they check nothing".  The marker's
      -- own terms are read in the goal's context: their types mention the
      -- goal's locals, a reference local of a reborrow among them.
      let condition ← match ctx.invariants.find? (·.1 == site) with
        | some (_, function, condition) =>
            atStart goal function (← conditionAt goal condition (some arguments[0]!)) site #[]
        | none => goal.withContext do
            -- A construction's marker checks its value, a mutation end's
            -- the locals it ends with (`constructedAt`, `mutationEndAt`).
            let value := if constructed then arguments[6]! else arguments[5]!
            let valueType ← inferType value
            let memory ← inferType (if constructed then arguments[10]! else arguments[8]!)
            pure (.lam `value valueType (.lam `memory memory (mkConst ``True) .default) .default)
      if constructed then
        pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_constructedAt)
            (arguments.extract 0 8 ++ #[condition] ++ arguments.extract 8 11),
          #[arguments[7]!, arguments[10]!], Provenance.construction, site)
      else
        pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_mutationEndAt)
            (arguments.extract 0 6 ++ #[condition] ++ arguments.extract 6 9),
          #[arguments[5]!, arguments[8]!], Provenance.mutationEnd, site)
  match ← goal.withContext (goal.apply rule) with
  | [holds, continues] =>
      setGoals [continues]
      evalTactic (← `(tactic| intro invariantHolds))
      evalTactic (← `(tactic| try simp only [VerexIR.Proofs.Obligation_eq] at invariantHolds))
      evalTactic (← `(tactic| try verex_normalize at invariantHolds))
      unless (← getGoals).isEmpty do
        let cut ← cutHypothesis? `invariantHolds
        normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization marks
        evalTactic (← `(tactic| all_goals verex_split_hypotheses))
        evalTactic (← `(tactic| all_goals verex_subst_vars))
        renormalizeCut ctx.normalization cut
        queue (← getGoals) provenance clauses
      setGoals [holds]
      -- The certificate of a value whose variant is not known is the
      -- certificate of each of its variants: split it, so that the clauses
      -- of an arm are its fields' (`variantPlain.deepHolds_inl`).
      if let some certifying := certifying? then
        if let .fvar value := certifying.2.2[5]!.consumeMData then
          let abstractVariant ← holds.withContext do
            let type ← whnfD (← instantiateMVars (← value.getType))
            pure (type.isAppOf ``Sum)
          if abstractVariant then
            setGoals (← destructureOuterVariant holds value).toList
      normalizeGoals ctx
      -- A declaration's data invariant of the encoding of the value packed:
      -- the normal form of `Carriers.own` at a unit that states invariants
      -- (`<path>.ownEq`, `OwnAtArguments`).  Its premises introduced, what
      -- is owed is the declaration's clauses of the row, which one more
      -- normalization reduces (`verex_own_encoded`).
      let owed ← getGoals
      let mut encoded : Array MVarId := #[]
      let mut introduced := false
      for goal in owed do
        -- The goal's own context reads its type; the tactic runs on the goal
        -- itself, never inside another goal's context, so that the terms it
        -- elaborates are the goal's.
        let isEncoded ← goal.withContext do
          pure ((← instantiateMVars (← goal.getType)).isAppOf
            ``VerexIR.Proofs.Denote.OwnAtArguments)
        unless isEncoded do
          encoded := encoded.push goal
          continue
        setGoals [goal]
        evalTactic (← `(tactic| verex_own_encoded))
        match ← getGoals with
        | [next] =>
            encoded := encoded.push next
            introduced := true
        | goals => throwError
            "the data invariant of an encoding left {goals.length} obligations"
      setGoals encoded.toList
      if introduced then normalizeGoals ctx
      queue (← getGoals) (some origin) #[]
  | _ => throwError "the data invariant rule did not produce its two obligations"
  charge "invariant" stageStart
  return true

/-- An anchor: the locals and state here are recorded for the assertions
reading it. -/
private def anchorStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let some (site, arguments) ← anchorGoal? goal | return false
  let rule := mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_anchorAt) (arguments.extract 0 9)
  let [continues] ← goal.withContext (goal.apply rule)
    | throwError "the anchor rule did not produce its one obligation"
  let recorded ← continues.withContext do
    let proof ← mkAppM ``AnchorSaved.intro #[Lean.toExpr site, arguments[5]!, arguments[8]!]
    let (_, recorded) ← (← continues.assert `verexAnchor (← inferType proof) proof).intro1P
    pure recorded
  setGoals [recorded]
  normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization #[arguments[5]!, arguments[8]!]
  queue (← getGoals) provenance clauses
  charge "anchor" stageStart
  return true

/-- The clauses of a goal with the obligation range of its target added. -/
private def withTargetClause (goal : MVarId) (clauses : Array ObligationRange) :
    MetaM (Array ObligationRange) := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  pure <| match obligationRange? target with
    | some range => if clauses.contains range then clauses else clauses.push range
    | none => clauses

/-- The scalar, closure and invocation steps, each a rule of its own. -/
private def invocationSteps (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  if ← scalarAt? (← contextFacts goal) goal then
    charge "scalar rows" stageStart
    return true
  if let some remaining ← routedClosureFrame? goal then
    queue remaining provenance (← withTargetClause goal clauses)
    charge "closure frame" stageStart
    return true
  if let some remaining ← literalClosureKeepsMemory? goal ctx.callees then
    queue remaining provenance (← withTargetClause goal clauses)
    charge "closure frame" stageStart
    return true
  if let some remaining ← routedInvocation? goal then
    queue remaining provenance clauses
    charge "invoke" stageStart
    return true
  if let some remaining ← unseenModelInvocation? goal then
    setGoals remaining
    evalTactic (← `(tactic| all_goals (try verex_normalize)))
    queue (← getGoals) provenance clauses
    charge "invoke" stageStart
    return true
  if let some remaining ← unseenInvocation? goal then
    setGoals remaining
    evalTactic (← `(tactic|
      all_goals (try dsimp only [VerexIR.Proofs.Denote.ResultShape.row] at *)))
    evalTactic (← `(tactic|
      all_goals (try verex_normalize at $(mkIdent `verexResultOf):ident)))
    evalTactic (← `(tactic| all_goals (try verex_normalize)))
    setGoals ((← (← getGoals).mapM fun g => liftM (rewriteResultOf g)).filterMap id)
    evalTactic (← `(tactic| all_goals (try verex_normalize)))
    queue (← getGoals) provenance clauses
    charge "invoke" stageStart
    return true
  return false

/-- The simp lemmas unfolding a compiled callee: its definition and the
parts of its signature. -/
private def compiledUnfoldLemmas (compiled : Lean.Expr) :
    TacticM (Array (TSyntax ``Lean.Parser.Tactic.simpLemma)) := do
  let unfoldNames := match compiled with
    | .const name _ =>
        [name, name.getPrefix ++ `body, name.getPrefix ++ `mutables, name.getPrefix ++ `params,
          name.getPrefix ++ `locals, name.getPrefix ++ `shape, name.getPrefix ++ `row]
    | _ => []
  unfoldNames.toArray.mapM fun name =>
    `(Lean.Parser.Tactic.simpLemma| $(mkIdent (rootNamespace ++ name)):ident)

/-- An inlined callee starts here, on the main goal: its loops' invariants
read `old` from its arguments at the call, and its precondition is read
there. The caller owes the precondition where the callee starts, as the Move
Prover asserts it at the call; the body may assume it. False where a
precondition that does not hold closes what follows. -/
private def enterCallee (ctx : SolveContext) (step : SolveStep) (handle : Lean.Expr)
    (calleeName : String) (unit frame arguments state : Lean.Expr) : SolveM Bool := do
  let goal := step.goal
  let precondition? ← ctx.preconditions.findM? fun (function, _) =>
    goal.withContext (isDefEq function handle)
  if precondition?.isSome ||
      (← ctx.invariants.anyM fun (_, function, _) => goal.withContext (isDefEq function handle)) then
    let inlined ← getMainGoal
    let started ← inlined.withContext do
      let proof ← mkAppM ``FunctionStart.intro #[handle, arguments, state]
      let (_, started) ← (← inlined.assert `verexStart (← inferType proof) proof).intro1P
      pure started
    replaceMainGoal [started]
  if let some (_, condition) := precondition? then
    let started ← getMainGoal
    let owed ← atStart started handle (mkApp2 condition unit frame) 0 #[]
    let holds ← started.withContext (mkFreshExprSyntheticOpaqueMVar owed)
    let (_, continues) ← (← started.assert `calleeRequires owed holds).intro1P
    replaceMainGoal [continues]
    -- The clauses the precondition states, as the obligation's
    -- locations; the leaves decide what they state.
    let (holdsGoal, required) ← stripObligations holds.mvarId! #[]
    setGoals [holdsGoal]
    evalTactic (← `(tactic| try verex_normalize))
    queue (← getGoals) (some (.precondition calleeName (ctx.callNumber step.callSite))) required
    setGoals [continues]
    evalTactic (← `(tactic| try simp only [VerexIR.Proofs.Obligation_eq] at calleeRequires))
    if (← getGoals).isEmpty then return false
  return true

/-- The summary of an inlined callee of a model, where one applies to the goal
over its body (`applyModelSummary`), set where the summaries are defined. -/
initialize modelSummaryCall :
    IO.Ref (SolveContext → SolveStep → Lean.Expr → String → SolveM Bool) ←
  IO.mkRef fun _ _ _ _ => pure false

/-- Walk into a model's inlined callee on the goal over its body under its
meanings: its start, then the body unfolded. -/
private def inlineModelBody (ctx : SolveContext) (step : SolveStep) (handle? : Option Lean.Expr)
    (calleeName : String) : SolveM Unit := do
  let { goal := next, provenance, clauses, stageStart, .. } := step
  let target ← next.withContext (instantiateMVars (← next.getType))
  let body := (target.getArg! 3).consumeMData
  let compiled := body.getArg! 5
  let lemmas ← compiledUnfoldLemmas compiled
  if let some handle := handle? then
    unless ← enterCallee ctx step handle calleeName (body.getArg! 0) (body.getArg! 3)
        (body.getArg! 6) (target.getArg! 6) do
      charge "inline" stageStart
      return
  -- The continuation, state, frame and arguments of the call are normal: only
  -- the body is new.
  let normal ← next.withContext do
    #[target.getArg! 4, target.getArg! 5, target.getArg! 6, body.getArg! 6].mapM instantiateMVars
  unfoldInlined lemmas (mkIdent ``VerexIR.Proofs.Denote.Function.denoteWith) normal
  normalizeGoals ctx normal
  queue (← rewriteEquations ctx.equations (← getGoals)) provenance clauses
  charge "inline" stageStart

/-- Under a model an inlined callee's meaning refines its body under the
same meanings (`doc/designs/ghost-fields.md`): the body replaces the call. -/
private def inlineRefinement (ctx : SolveContext) (step : SolveStep) (refinesProof : Lean.Expr)
    (handle? : Option Lean.Expr) (calleeName : String) : SolveM Unit := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  let action := (target.getArg! 3).consumeMData
  let next ← goal.withContext do
    -- The refinement at the call: its family, instantiation and
    -- arguments, where it is stated over them, are the call's.
    let (binders, _, statement) ←
      forallMetaTelescope (← whnfR (← zetaReduce (← inferType refinesProof)))
    let .app (.app _ refined) body ← whnfR statement
      | throwError "an inlined callee's refinement does not name its body"
    unless ← isDefEq refined action do
      throwError m!"an inlined callee's refinement does not meet its call {action}"
    let refinement ← instantiateMVars (mkAppN refinesProof binders)
    let body ← instantiateMVars body
    let premise ← mkAppM ``VerexIR.Proofs.wp
      #[body, target.getArg! 4, target.getArg! 5, target.getArg! 6]
    let next ← mkFreshExprSyntheticOpaqueMVar premise (← goal.getTag)
    goal.assign (← mkAppM ``VerexIR.Proofs.Denote.wp_of_refines #[refinement, next])
    pure next.mvarId!
  setGoals [next]
  if let some handle := handle? then
    if ← (← modelSummaryCall.get) ctx { step with goal := next } handle calleeName then return
    setGoals [next]
  inlineModelBody ctx { step with goal := next } handle? calleeName

/-- An unspecified callee: its compiled body replaces the call, by the
agreement theorem, and the traversal continues into it. -/
private def inlineCompiled (ctx : SolveContext) (step : SolveStep) (handle : Lean.Expr)
    (calleeName : String) (theoremProof proofType : Lean.Expr) : SolveM Unit := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let compiled := (proofType.getArg! 2).appArg!
  let lemmas ← compiledUnfoldLemmas compiled
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  let action := target.getArg! 3
  -- The rule at the call's own arguments; its left side is the goal up
  -- to unfolding the compiled callee's signature, which the kernel
  -- checks — unifying the two here would unfold them in `isDefEq`.
  let rule := mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_propheticMeaning_of_compiled)
    #[action.getArg! 0, action.getArg! 1, action.getArg! 2, action.getArg! 3, action.getArg! 4,
      action.getArg! 5, compiled, theoremProof, action.getArg! 8, target.getArg! 4,
      target.getArg! 5, target.getArg! 6]
  let next ← goal.withContext do
    let .forallE _ premise _ _ := ← whnfR (← inferType rule)
      | throwError "the agreement rule does not take the callee's denotation"
    let next ← mkFreshExprSyntheticOpaqueMVar premise (← goal.getTag)
    goal.assign (mkApp rule next)
    pure next.mvarId!
  setGoals [next]
  unless ← enterCallee ctx step handle calleeName (action.getArg! 0) (action.getArg! 3)
      (action.getArg! 8) (target.getArg! 6) do
    charge "call" stageStart
    return
  let sizeBefore ← if ctx.debug then do
      let goal ← getMainGoal
      pure (← goal.withContext do pure (treeSize (← instantiateMVars (← goal.getType)) 100000000))
    else pure 0
  let unfoldStart ← IO.getNumHeartbeats
  -- The continuation, state, frame and arguments of the call are normal: only
  -- the body is new.
  let normal ← goal.withContext do
    #[target.getArg! 4, target.getArg! 5, target.getArg! 6, action.getArg! 8].mapM instantiateMVars
  unfoldInlined lemmas (mkIdent ``VerexIR.Proofs.Denote.Function.denote) normal
  let normalizeStart ← IO.getNumHeartbeats
  normalizeGoals ctx normal
  if ctx.debug then
    let sizeAfter ← (← getGoals).foldlM (init := 0) fun total goal => do
      pure (total + (← goal.withContext do
        pure (treeSize (← instantiateMVars (← goal.getType)) 100000000)))
    logInfo m!"inline {calleeName}: goal tree {sizeBefore} → {sizeAfter}; unfold \
      {(normalizeStart - unfoldStart) / 1000}k, normalize \
      {((← IO.getNumHeartbeats) - normalizeStart) / 1000}k"
  queue (← rewriteEquations ctx.equations (← getGoals)) provenance clauses
  charge "inline" stageStart

/-- The locals a term depends on, with those their types depend on, in
context order; none when a local definition is among them. -/
private def closingLocals (e : Lean.Expr) : MetaM (Option (Array Lean.Expr)) := do
  let mut pending := (collectFVars {} e).fvarIds
  let mut seen : Std.HashSet FVarId := {}
  while let some id := pending.back? do
    pending := pending.pop
    if seen.contains id then continue
    seen := seen.insert id
    let decl ← id.getDecl
    if decl.isLet then return none
    pending := pending ++ (collectFVars {} (← instantiateMVars decl.type)).fvarIds
  return some ((← getLCtx).sortFVarsByContextOrder seen.toArray |>.map mkFVar)

/-- The family transports of values, whose last argument is the value. -/
private def familyTransports : Array Name := #[``VerexIR.Proofs.Denote.NTy.toSkolem,
  ``VerexIR.Proofs.Denote.NTy.ofSkolem, ``VerexIR.Proofs.Denote.HList.toSkolem,
  ``VerexIR.Proofs.Denote.HList.ofSkolem, ``VerexIR.Proofs.Denote.variantCarrier.toSkolem,
  ``VerexIR.Proofs.Denote.variantCarrier.ofSkolem,
  ``VerexIR.Proofs.Denote.NTy.plainToSkolem, ``VerexIR.Proofs.Denote.NTy.plainOfSkolem,
  ``VerexIR.Proofs.Denote.HListPlain.toSkolem, ``VerexIR.Proofs.Denote.HListPlain.ofSkolem,
  ``VerexIR.Proofs.Denote.variantPlain.toSkolem, ``VerexIR.Proofs.Denote.variantPlain.ofSkolem,
  ``VerexIR.Proofs.Denote.ResultShape.toSkolem,
  ``VerexIR.Proofs.Denote.ResultShape.ofSkolem]

/-- Whether a call's argument is structure its contract's normal form keeps:
a tuple, a variant's injection, the unit, or a family transport of a
component. -/
private def argumentStructure (e : Lean.Expr) : Bool :=
  e.isAppOfArity ``Prod.mk 4 || e.isConstOf ``Unit.unit || e.isAppOf ``PUnit.unit ||
    e.isAppOfArity ``Sum.inl 3 || e.isAppOfArity ``Sum.inr 3 ||
    familyTransports.any e.isAppOf

/-- The components of a call's arguments: the maximal subterms below their
structure, in order, each once. -/
private partial def argumentComponents (e : Lean.Expr) (found : Array Lean.Expr) :
    Array Lean.Expr :=
  if e.isAppOfArity ``Prod.mk 4 then
    argumentComponents (e.getArg! 3) (argumentComponents (e.getArg! 2) found)
  else if e.isConstOf ``Unit.unit || e.isAppOf ``PUnit.unit then found
  else if argumentStructure e then argumentComponents e.appArg! found
  else if found.contains e then found else found.push e

/-- A call's arguments with each component replaced by its variable. -/
private partial def argumentPattern (e : Lean.Expr) (components variables : Array Lean.Expr) :
    Lean.Expr :=
  if e.isAppOfArity ``Prod.mk 4 then
    mkAppN e.getAppFn #[e.getArg! 0, e.getArg! 1,
      argumentPattern (e.getArg! 2) components variables,
      argumentPattern (e.getArg! 3) components variables]
  else if e.isConstOf ``Unit.unit || e.isAppOf ``PUnit.unit then e
  else if argumentStructure e then
    mkApp e.appFn! (argumentPattern e.appArg! components variables)
  else match components.idxOf? e with
    | some index => variables[index]!
    | none => e

/-- The conditions of a contract the call rules take as equations: those of
`wp_call_equations` in its order; `wp_call_normal` takes all but the
permitted failures. -/
private def contractConditions : Array Name := #[``VerexIR.Proofs.Contract.assumes,
  ``VerexIR.Proofs.Contract.requires, ``VerexIR.Proofs.Contract.mayAbort,
  ``VerexIR.Proofs.Contract.mustAbort, ``VerexIR.Proofs.Contract.ensures,
  ``VerexIR.Proofs.Contract.frame, ``VerexIR.Proofs.Contract.aborts]

/-- The equations giving the normal forms of a callee's contract at a call,
in the order of `contractConditions`: instances of proofs found once for the
contract with the call's instantiations, argument components, and state
generalized. None where the contract or arguments mention a local
definition. -/
private def contractEquations? (contract args initial : Lean.Expr)
    (lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma)) : TacticM (Option (Array Lean.Expr)) := do
  let instantiationType ← mkAppM ``Array
    #[← mkAppM ``Prod #[mkConst ``VerexIR.TypeId, mkConst ``VerexIR.TypeId]]
  let instantiations ← contract.getAppArgs.filterM fun argument => do
    withReducible (isDefEq (← inferType argument) instantiationType)
  let components := argumentComponents args #[]
  let mut declarations : Array (Name × BinderInfo × (Array Lean.Expr → TacticM Lean.Expr)) := #[]
  for _ in instantiations do
    declarations := declarations.push (`instantiation, .default, fun _ => pure instantiationType)
  for component in components do
    let type ← inferType component
    declarations := declarations.push (`argument, .default, fun _ => pure type)
  let stateType ← inferType initial
  declarations := declarations.push (`initial, .default, fun _ => pure stateType)
  withLocalDecls declarations fun variables => do
    let instantiationVariables := variables.extract 0 instantiations.size
    let componentVariables := variables.extract instantiations.size
      (instantiations.size + components.size)
    let state := variables.back!
    let contract' := mkAppN contract.getAppFn (contract.getAppArgs.map fun argument =>
      match instantiations.idxOf? argument with
      | some index => instantiationVariables[index]!
      | none => argument)
    let args' := argumentPattern args components componentVariables
    let marker := mkApp2 (mkConst `verexContractPattern) contract' args'
    let some closing ← closingLocals marker | return none
    let locals := closing.filter fun local_ => !variables.contains local_
    let binders := locals ++ variables
    let key ← mkLambdaFVars binders marker
    let actual := locals ++ instantiations ++ components ++ #[initial]
    if let some (proofs, constants) := (← (← targetState).contractNormalForms.get)[key]? then
      let env ← getEnv
      if constants.all env.contains then
        return some (proofs.map fun proof => (mkAppN proof actual).headBeta)
    -- A pattern met for the first time is normalized at its call.
    unless (← (← targetState).contractPatternsMet.get).contains key do
      (← targetState).contractPatternsMet.modify (·.insert key)
      return none
    let { ctx, simprocs, .. } ← mkSimpContext
      (← `(tactic| simp only [$lemmas,*, vir_denote, vir_denote_norm, vir_denote_eval,
        Prod.fst, Prod.snd]))
      (eraseLocal := false)
    let ctx ← Simp.mkContext { ctx.config with failIfUnchanged := false } ctx.simpTheorems
      ctx.congrTheorems
    let mut proofs := #[]
    for condition in contractConditions do
      let applied := mkApp2 (← mkAppM condition #[contract']) args' state
      let proof ← forallTelescope (← whnfR (← inferType applied)) fun rest _ => do
        let value := mkAppN applied rest
        let (result, _) ← Simp.main value ctx (methods := Simp.mkDefaultMethodsCore simprocs)
        let proof ← match result.proof? with
          | some proof => pure proof
          | none => mkEqRefl value
        mkLambdaFVars (binders ++ rest) proof
      let proof ← instantiateMVars proof
      if proof.hasFVar || proof.hasMVar then return none
      proofs := proofs.push proof
    let constants := proofs.foldl (fun found proof => found ++ proof.getUsedConstants) #[]
    (← targetState).contractNormalForms.modify (·.insert key (proofs, constants))
    return some (proofs.map fun proof => (mkAppN proof actual).headBeta)

/-- Apply the call rule taking the callee's contract normal forms
(`contractEquations?`, `wp_call_normal`). False, the state restored, where
the normal forms do not apply. -/
private def callThroughNormalForms (goal : MVarId) (proofType : Lean.Expr)
    (proofSyntax : TSyntax `term) (lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma)) :
    TacticM Bool := do
  let saved ← saveState
  try
    let found ← goal.withContext do
      let target ← instantiateMVars (← goal.getType)
      let_expr VerexIR.Proofs.Satisfies _ _ _ _ _ contract :=
          ← whnfR (← instantiateMVars proofType) | pure none
      unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
      let action := (target.getArg! 3).consumeMData
      unless action.isApp do return none
      let some equations ← contractEquations? contract action.appArg! (target.getArg! 6) lemmas
        | return none
      some <$> equations.mapM fun equation => do Lean.Elab.Term.exprToSyntax equation
    let some terms := found | return false
    evalTactic (← `(tactic| refine VerexIR.Proofs.Denote.wp_call_normal $proofSyntax rfl
      $(terms[0]!) $(terms[1]!) $(terms[3]!) $(terms[4]!) $(terms[5]!) $(terms[6]!)
      ?_ ?_ ?_ ?_))
    return true
  catch _ =>
    saved.restore
    return false

/-- The type arguments inducing the frame a callee's satisfaction is at, and
the caller's frame, where that frame is `Skolems.instantiate θ Θ`. -/
private def inducedArguments? (proofType : Lean.Expr) :
    MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let_expr VerexIR.Proofs.Satisfies args _ _ _ _ _ := ← whnfR proofType | return none
  let_expr VerexIR.Proofs.Denote.HList carriers _ := args.consumeMData | return none
  let_expr VerexIR.Proofs.Denote.Skolems.toCarriers _ _ frame := carriers.consumeMData
    | return none
  let_expr VerexIR.Proofs.Denote.Skolems.instantiate _ _ θ outer := frame.consumeMData
    | return none
  return some (θ, outer)

/-- The callee's satisfaction at the caller's types where its contract is
specialized at the call's type arguments (`satisfies_specialized`), with
those arguments: the equivalence whose generic contract is the
satisfaction's. -/
private def specializedCall? (specializations : Array Lean.Expr) (theoremProof proofType : Lean.Expr) :
    MetaM (Option (Lean.Expr × Lean.Expr × Lean.Expr)) := do
  let some (θ, outer) ← inducedArguments? proofType | return none
  let_expr VerexIR.Proofs.Satisfies _ _ _ _ _ contract := ← whnfR proofType | return none
  let contract ← instantiateMVars contract
  let some head := contract.getAppFn.constName? | return none
  for equivalence in specializations do
    let saved ← saveState
    let (arguments, _, statement) ← forallMetaTelescope (← inferType equivalence)
    let_expr VerexIR.Proofs.Contract.Equivalent _ _ _ _ _ read := statement | continue
    let_expr VerexIR.Proofs.Contract.ofSkolem _ _ _ induced _ _ generic := read | continue
    unless generic.getAppFn.constName? == some head do continue
    if ← (isDefEq induced θ <&&> isDefEq generic contract) then
      let equivalent ← instantiateMVars (mkAppN equivalence arguments)
      let proof ← mkAppOptM ``VerexIR.Proofs.Denote.satisfies_specialized
        #[none, none, some outer, some θ, none, none, none, none, none, some theoremProof,
          some equivalent]
      return some (θ, outer, proof)
    saved.restore
  return none

/-- A statement's conclusion, under its binders and premises, marked as
what a path owes after the site (`AfterAt`). -/
private partial def markConclusion (site : Nat) : Lean.Expr → Lean.Expr
  | .forallE name domain body info => .forallE name domain (markConclusion site body) info
  | .mdata data body => .mdata data (markConclusion site body)
  | conclusion => mkApp2 (mkConst ``AfterAt) (mkNatLit site) conclusion

/-- The goal with the premises after its first `skip` binders named, in
order, as far as the names reach. -/
-- Only propositions are premises; the values and states between them
-- keep their names.
private partial def namePropositions (e : Lean.Expr) : List Name → MetaM Lean.Expr
  | [] => pure e
  | names@(name :: rest) => match e with
    | .forallE n d b i =>
        withLocalDecl n i d fun x => do
          let (n, rest) := if ← isProp d then (name, rest) else (n, names)
          pure (.forallE n d ((← namePropositions (b.instantiate1 x) rest).abstract #[x]) i)
    | _ => pure e

private def namePremises (goal : MVarId) (names : List Name) : MetaM MVarId :=
  goal.withContext do
    goal.replaceTargetDefEq (← namePropositions (← instantiateMVars (← goal.getType)) names)

/-- A call by the callee's theorem: what the theorem assumes and its
precondition, then the continuation and the failures. A generic call meets
the callee's contract specialized at its type arguments, where one is
given: its arguments and result at the caller's types. -/
private def applyCallTheorem (ctx : SolveContext) (step : SolveStep) (calleeName : String)
    (theoremProof proofType : Lean.Expr) (theoremArguments : Array Lean.Expr)
    (proofSyntax : TSyntax `term) : SolveM Unit := do
  let { goal, clauses, stageStart, .. } := step
  -- The theorem's meaning is the call's: its frame and instantiation are
  -- the call's, found before the rule's types are unified.
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let_expr VerexIR.Proofs.Satisfies _ _ _ _ meaning _ := ← whnfR proofType | pure ()
    if target.isAppOfArity ``VerexIR.Proofs.wp 7 then
      let action := (target.getArg! 3).consumeMData
      if action.isApp then discard <| isDefEq meaning action.appFn!
  -- The call's continuations are normal; the rules apply them to the
  -- callee's fresh result, error and final state, a specialized rule
  -- through the result's transport, so held wherever they occur, the
  -- contract's normalization walks the contract and that transport only.
  -- The call's arguments and state are normal too; the contract's
  -- normalization takes them as they are where its forms are instantiated.
  let (parts, values) ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    pure (if target.isAppOfArity ``VerexIR.Proofs.wp 7 then
        (#[target.getArg! 4, target.getArg! 5],
          (target.getArg! 3).consumeMData.getAppArgs.push (target.getArg! 6))
      else (#[], #[]))
  let heldRef ← IO.mkRef #[]
  timedStep "call-hold" do heldRef.set (← holdContinuations parts)
  let held ← heldRef.get
  let goal ← getMainGoal
  let specialized? ← goal.withContext do
    specializedCall? ctx.specializations theoremProof (← instantiateMVars proofType)
  let (theoremProof, proofType, proofSyntax) ← match specialized? with
    | some (θ, outer, proof) => goal.withContext do
        -- The call's meaning and arguments as the goal states them: its
        -- arguments are the caller's values, at the carriers of the frame.
        let target ← instantiateMVars (← goal.getType)
        let action := (target.getArg! 3).consumeMData
        let rule ← mkAppOptM ``VerexIR.Proofs.Denote.wp_specialized
          #[none, none, some outer, some θ, none, none, some action.appFn!, some action.appArg!,
            some (target.getArg! 4), some (target.getArg! 5), some (target.getArg! 6)]
        let [next] ← goal.apply rule { newGoals := .nonDependentOnly }
          | throwError m!"the generic call of {calleeName} does not meet its specialized contract"
        setGoals [next]
        pure (proof, ← instantiateMVars (← inferType proof), ← Lean.Elab.Term.exprToSyntax proof)
    | none => pure (theoremProof, proofType, proofSyntax)
  let goal ← getMainGoal
  let constants ← goal.withContext (do contractConstants (← inferType theoremProof))
  let lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ← constants.mapM fun name => do
    let ident := mkIdent (rootNamespace ++ name)
    `(Lean.Parser.Tactic.simpLemma| $ident:ident)
  -- The callee's contract normalized once for its instance, its normal
  -- forms instantiated at the call (`doc/designs/native-verification.md`, "Specialized contracts").
  let contractForms ← if verex.contractLemmas.get (← getOptions) then
      callThroughNormalForms goal proofType proofSyntax lemmas
    else pure false
  -- The equation rule uses the target's normalization below. Build the
  -- contract's unfolding context only when a call rule still needs it.
  let contractNormalization ← if contractForms then pure ctx.normalization else
    goal.withContext do
      simpContextOf (← `(tactic|
        simp only [$lemmas,*, vir_denote, vir_denote_norm, vir_denote_eval, Prod.fst, Prod.snd]))
  -- A contract whose failure conditions coincide gives its postcondition
  -- outright after a successful run.
  let strict ← if !contractForms then do
      let saved ← (Tactic.saveState : TacticM Tactic.SavedState)
      try
        evalTactic (← `(tactic|
          refine VerexIR.Proofs.Denote.wp_call_strict $proofSyntax rfl ?_ ?_ ?_ ?_))
        pure true
      catch _ =>
        saved.restore
        pure false
    else pure false
  unless contractForms || strict do
    evalTactic (← `(tactic| refine VerexIR.Proofs.Denote.wp_call $proofSyntax ?_ ?_ ?_ ?_))
  let subgoals ← getGoals
  -- A callee's theorem assumes the natives it reaches, a generic one's
  -- at every family and instantiation, and that the in-body assumptions
  -- of the functions it relies on hold; the caller assumes them too.
  for argument in theoremArguments do
    if let .mvar id ← instantiateMVars argument then
      if ← id.isAssigned then continue
      -- A generic callee runs in a frame: the empty one, the call's, or
      -- one a frame equation of the target gives.
      if (← instantiateMVars (← id.getType)).isAppOfArity ``VerexIR.Proofs.Denote.FrameOf 4 then
        let mut candidates : Array (TSyntax `tactic) :=
          #[← `(tactic| exact VerexIR.Proofs.Denote.FrameOf.empty),
            ← `(tactic| exact VerexIR.Proofs.Denote.FrameOf.frame rfl),
            ← `(tactic| assumption)]
        for equation in ctx.equations do
          let term ← goal.withContext (Lean.Elab.Term.exprToSyntax equation)
          candidates := candidates.push (← `(tactic| exact VerexIR.Proofs.Denote.FrameOf.rewrite
            (VerexIR.Proofs.Denote.FrameOf.frame rfl) $term))
        let saved ← getGoals
        let mut discharged := false
        for candidate in candidates do
          let before ← Tactic.saveState
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
        pure (body.isAppOf ``VerexIR.Proofs.Satisfies ||
          body.isAppOf ``VerexIR.NativesTyped || body.isAppOf ``VerexIR.NativesShift ||
          body.isAppOf ``VerexIR.Proofs.RunsAgree ||
          body.isAppOf ``VerexIR.Proofs.OwnersRepresentable ||
          body.isAppOf ``VerexIR.Proofs.Terminating ||
          body.isAppOf ``VerexIR.Proofs.AssumptionsHold ||
          -- An assumed step of a lemma the callee applies.
          (match body.getAppFn with
            | .const (.str _ part) _ => part.startsWith "lemmaTrusted_"
            | _ => false))
      if native then id.assumption
  for (subgoal, index) in subgoals.toArray.zipIdx do
    -- The callee's result is named after the callee in the continuation.
    let subgoal ← if index == 2 then subgoal.withContext do
        let target ← instantiateMVars (← subgoal.getType)
        let .forallE _ domain body info := target | pure subgoal
        let name := Name.mkSimple s!"{identifierPart calleeName}_result"
        -- Keep successive results addressable in authored proofs. The latest
        -- call keeps the ordinary name; earlier results get a fresh old name.
        let subgoal ← match (← getLCtx).findFromUserName? name with
          | some previous => renameScoped subgoal previous.fvarId (← oldUserName name)
          | none => renameLatestEntry subgoal name.toString none (← oldUserName name)
        -- A call a proof places facts after: its continuation's conclusion
        -- carries the call's site (`AfterAt`), met on every path after it.
        let body := match step.callSite with
          | some site =>
              let named := ctx.callSites.find? (·.1 == site) |>.any fun (_, callee, number, overall) =>
                ctx.points.any (·.1.selects (.after (.call number (identifierPart callee) overall)))
              if named then markConclusion site body else body
          | none => body
        -- In a step proof, the result's scope entry comes with it.
        let index := max (← nextScopeIndex) (← get).scopeIndex
        modify fun s => { s with scopeIndex := index + 1 }
        -- A call result enters the context at the native type's canonical
        -- spelling, so its construction certificate is immediately visible.
        let domain := (← canonicalFamilyResult domain).expr
        let subgoal ← scopedBinderGoal subgoal name domain body info index
        -- The call's premises are named after the callee, as its clauses
        -- are: the ensures, the frame, and that the call returned.
        let part := identifierPart calleeName
        let premises := [s!"{part}_ensures", s!"{part}_frame", s!"{part}_returns"].map Name.mkSimple
        -- The frame stays a fact (`TargetState.keptFacts`).
        noteKeptPremiseNames [premises[1]!]
        namePremises subgoal (if strict || contractForms then
          [premises[2]!, premises[0]!, premises[1]!] else premises)
      else pure subgoal
    setGoals [subgoal]
    -- Unfold and normalize the contract in one pass, around the continuations.
    -- Normal forms of the contract are normalized again where the call's
    -- values instantiate them.
    let values ← values.mapM instantiateMVars
    if contractForms then
      timedStep "call-contract" (normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization values)
    else
      timedStep "call-contract" (normalizeAround contractNormalization values)
    timedStep "call-release" (releaseContinuations ctx.normalization held walkFixedPoints)
    -- Speculative range search belongs to automatic verification. An authored
    -- proof receives the contract directly instead of paying for normalization
    -- and failed case enumeration before its first tactic runs.
    if index == 2 && !ctx.handOver then
      evalTactic (← `(tactic| all_goals (try verex_timed "call-cases" verex_call_result_cases)))
    -- What the callee's theorem assumes and its precondition, then
    -- the continuation and the failures.
    let call := ctx.callNumber step.callSite
    let origin := if index ≤ 1 then some (Provenance.precondition calleeName call)
      else if index == 2 then some (.continuation calleeName call) else none
    queue (← getGoals) origin (if index ≤ 1 then #[] else clauses)
  charge "call" stageStart

/-! ## Derived summaries

A contract-free callee's summary is its weakest-precondition characterisation:
`∀ args ensures aborts initial, Ψ → wp (call args) ensures aborts initial`,
where `args` are the scalars the arguments split into and `Ψ` conjoins the
leaves the walk leaves on the callee's body with the continuations opaque,
one per path. The walk's proof term proves it, checked
by the kernel once; a call applies it as `applyCallTheorem` applies a
contract, and its conjuncts, instantiated at the call, are the leaves an
inlined body would give. -/

/-- A derived summary: its theorem, declared before the module's proofs run
(`deriveModuleSummary`), and its number of conjuncts. -/
structure DerivedSummary where
  name : Name
  conjuncts : Nat
  /-- Whether the statement takes the callee's precondition, which a call owes
  once, before its conjuncts. -/
  takesRequires : Bool := false

/-- The summaries derived in this process, by compiled callee; none where the
callee's walk does not qualify. Written before any proof of the module runs,
read by every call. -/
initialize derivedSummaries : Std.Mutex (Std.HashMap Name (Option DerivedSummary)) ←
  Std.Mutex.new {}

/-- Record what a callee's summary is. -/
def recordDerivedSummary (compiled : Name) (summary : Option DerivedSummary) : BaseIO Unit :=
  derivedSummaries.atomically (modify (·.insert compiled summary))

/-- The traversal a summary's walk runs from a state to its final state, set
where `traverse` is defined. -/
initialize summaryTraversal : IO.Ref (SolveContext → SolveState → TacticM SolveState) ←
  IO.mkRef fun _ state => pure state

/-- Whether a compiled callee's form admits a summary: not generic, without
a loop, not returning a reference. -/
def summaryForm (compiled : Name) : CoreM Bool := do
  let env ← getEnv
  let value (part : Name) := (env.find? (compiled.getPrefix ++ part)).bind (·.value?)
  let some row := value `row | return false
  let some shape := value `shape | return false
  let some body := value `body | return false
  let mentions (e : Lean.Expr) (name : Name) := (e.find? (·.isConstOf name)).isSome
  -- A body may mention a type parameter without being generic: a generic
  -- call carries its callee's signature.
  return ![row, shape].any (mentions · ``VerexIR.Proofs.Denote.NTy.param) &&
    !mentions body ``VerexIR.Proofs.Denote.Term.loop &&
    !mentions shape ``VerexIR.Proofs.Denote.NTy.ref

/-- Assign the argument variables of a summary's statement as the components
of a call's argument: where the argument is no tuple literal, as projections of
it. Unifying a variable tuple with a value at default transparency unfolds the
value, which the walk's inlining never does, and spells the summary's facts
differently from the clauses over the same value. -/
private partial def assignSummaryArguments (pattern actual : Lean.Expr) : MetaM Bool := do
  let pattern := (← instantiateMVars pattern).consumeMData
  let actual := actual.consumeMData
  if let .mvar id := pattern then
    id.assign actual
    return true
  if pattern.isAppOfArity ``Prod.mk 4 then
    let (head, tail) ← if actual.isAppOfArity ``Prod.mk 4 then
        pure (actual.getArg! 2, actual.getArg! 3)
      else
        let levels := pattern.getAppFn.constLevels!
        let first := pattern.getArg! 0
        let second := pattern.getArg! 1
        pure (mkApp3 (mkConst ``Prod.fst levels) first second actual,
          mkApp3 (mkConst ``Prod.snd levels) first second actual)
    return (← assignSummaryArguments (pattern.getArg! 2) head) &&
      (← assignSummaryArguments (pattern.getArg! 3) tail)
  if pattern.isAppOf ``PUnit.unit || pattern.isConstOf ``Unit.unit then return true
  withReducible (isDefEq pattern actual)

/-- Whether a leaf is one a summary states: what the callee's continuations
owe, on the paths after its calls too, and a callee's precondition its callers
owe, with no clause of its own. -/
private def summaryStates (provenance : Option Provenance) (clauses : Array ObligationRange) : Bool :=
  clauses.isEmpty &&
    (provenance matches none | some .returns | some (.continuation ..) | some (.precondition ..))

/-- The leading binders of a goal, `∀` and `let`. -/
private partial def leadingBinders : Lean.Expr → Nat
  | .forallE _ _ b _ | .letE _ _ _ b _ => leadingBinders b + 1
  | .mdata _ b => leadingBinders b
  | _ => 0

/-- The scalar locals a callee's arguments are, as the walk splits a
row-valued local (`verex_split_hypotheses`): a native pair into its
components, named by position as the walk names them, a unit tail its one
value, any other value one local; with the arguments they make up. A
summary is stated over them, so that its conjuncts speak of the arguments
a call instantiates them with, where a local the walk splits would leave
its components unrelated to the call. -/
private partial def withScalarArguments (name : Name) (index : Nat) (type : Lean.Expr)
    (k : Array Lean.Expr → Lean.Expr → TacticM α) : TacticM α := do
  let carrier ← whnfD type
  match carrier with
  | .const ``PUnit levels => k #[] (mkConst ``PUnit.unit levels)
  | .app (.app (.const ``Prod levels) head) tail =>
      unless (runtimeValueMentions carrier).isEmpty do
        return ← withLocalDeclD name type fun x => k #[x] x
      let tailUnit := (← whnfD tail).isConstOf ``PUnit
      let headName := if index == 0 && tailUnit then name else name.appendIndexAfter index
      withScalarArguments headName 0 head fun headLocals headValue =>
        withScalarArguments name (index + 1) tail fun tailLocals tailValue =>
          k (headLocals ++ tailLocals) (mkApp4 (mkConst ``Prod.mk levels) head tail headValue tailValue)
  | _ => withLocalDeclD name type fun x => k #[x] x

/-- The walk deriving the summary of the compiled callee `compiled` in the
current context, the root: its body with fresh arguments
(`withScalarArguments`), continuations and state, the call's shape being
`wpHead wpImplicit (meaning args) ensures aborts initial` at the
`types` of the arguments, continuations and state. The summary's closed
statement and proof with its number of conjuncts, or why the walk refuses
one. -/
private def summaryWalk (ctx : SolveContext) (compiled : Name) (handle : Lean.Expr)
    (calleeName : String) (theoremProof proofType : Lean.Expr) (types : Array Lean.Expr)
    (wpHead : Lean.Expr) (wpImplicit : Array Lean.Expr) (meaning : Lean.Expr)
    (requires? : Option Lean.Expr) :
    TacticM (Except String (Lean.Expr × Lean.Expr × Nat)) := do
  withScalarArguments `args 0 types[0]! fun scalars args =>
  withLocalDeclD `ensures types[1]! fun ensures =>
  withLocalDeclD `aborts types[2]! fun aborts => withLocalDeclD `initial types[3]! fun initial => do
  -- The callee's precondition, in the form a target's `permitted` has: a
  -- premise of the statement, from which the preconditions of the callee's
  -- own calls follow as they do in a target.
  let requires? ← requires?.mapM fun requires => do
    let reading := requires.beta #[args, ensures, aborts, initial]
    let contract := (reading.getArg! 4).getAppFn
    let some contractName := contract.constName? | return reading
    let mut names := #[contractName]
    if let some value := ((← getEnv).find? contractName).bind (·.value?) then
      for dependency in value.getUsedConstants do
        if contractName.getPrefix.isPrefixOf dependency && !names.contains dependency then
          names := names.push dependency
    let lemmas ← names.mapM fun name =>
      `(Lean.Parser.Tactic.simpLemma| $(mkIdent (rootNamespace ++ name)):ident)
    let { ctx := simpContext, simprocs, .. } ← mkSimpContext
      (← `(tactic| simp only [$lemmas,*, vir_denote, vir_denote_norm, vir_denote_eval,
        Prod.fst, Prod.snd])) (eraseLocal := false)
    let simpContext ← Simp.mkContext { simpContext.config with failIfUnchanged := false }
      simpContext.simpTheorems simpContext.congrTheorems
    let (result, _) ← Simp.main reading simpContext (methods := Simp.mkDefaultMethodsCore simprocs)
    return result.expr
  withLocalDeclD `permitted (requires?.getD (mkConst ``True)) fun permitted => do
    let parameters := scalars ++ #[ensures, aborts, initial] ++
      (if requires?.isSome then #[permitted] else #[])
    (← targetState).summaryParameters.modify
      (parameters.foldl (fun found x => found.insert x.fvarId!) ·)
    let statement := mkAppN wpHead (wpImplicit ++ #[mkApp meaning args, ensures, aborts, initial])
    let root ← mkFreshExprSyntheticOpaqueMVar statement
    let rootContext ← getLCtx
    setGoals [root.mvarId!]
    let first : SolveStep :=
      { goal := root.mvarId!, provenance := none, clauses := #[], stageStart := ← IO.getNumHeartbeats }
    let enter : SolveM Unit :=
      if (meaning.isAppOf ``VerexIR.Proofs.Denote.Function.denoteWith) then
        inlineModelBody ctx first (some handle) calleeName
      else inlineCompiled ctx first handle calleeName theoremProof proofType
    let ((), entered) ← enter.run ({ pending := #[] } : SolveState)
    let final ← (← summaryTraversal.get) ctx entered
    let mut leaves := #[]
    let origins ← (← targetState).leafOrigins.get
    for (leaf, _) in final.handed do
      if (← leaf.isAssigned) || leaves.contains leaf then continue
      -- An open obligation of the callee's own (a precondition of a call it
      -- makes, a module invariant after its write, an assertion, a loop
      -- invariant, ...) is named by its provenance and clauses, which the
      -- call's do not carry: the callee inlines, and the failure keeps its
      -- name. A summary states only what the callee's return and abort
      -- continuations owe, on the paths after its calls too.
      if let some (provenance, clauses) := origins[leaf]? then
        unless summaryStates provenance clauses do
          let shown := match provenance with
            | some p => s!"{p.key.kind} {p.key.callee}"
            | none => "none"
          return .error s!"a path owes an obligation of the callee's own: {shown}, {clauses.size} clauses"
      leaves := leaves.push leaf
    if leaves.size > verex.summaryLeaves.get (← getOptions) then
      return .error s!"{leaves.size} paths"
    -- Each leaf over the locals its path introduced.
    let mut conjuncts := #[]
    let mut introduced := #[]
    for leaf in leaves do
      let (conjunct, locals) ← leaf.withContext do
        let leafTarget ← instantiateMVars (← leaf.getType)
        if (leafTarget.find? (·.isAppOf ``VerexIR.Proofs.wp)).isSome then
          throwError "a summary's path does not end"
        let locals := (← getLCtx).foldl (init := #[]) fun found decl =>
          if rootContext.contains decl.fvarId || decl.isImplementationDetail then found
          else found.push decl.toExpr
        pure (← instantiateMVars (← mkForallFVars locals leafTarget), locals)
      unless !conjunct.hasMVar && (collectFVars {} conjunct).fvarIds.all rootContext.contains do
        return .error "a path's leaf is not over the root context"
      -- A conjunct stating a function value's behavior, or reading a
      -- call's target (`behaviorPredicates`), says nothing a call can use:
      -- there, the value may be a literal closure whose body the walk
      -- follows, and a call site does not fall back to inlining.
      if (conjunct.find? fun e => behaviorPredicates.any e.isConstOf).isSome then
        return .error "a path states a function value's behavior"
      -- A value reflected at the skolem family does not cancel against a
      -- caller's instantiation; inlined, the read normalizes at the caller's.
      if (conjunct.find? (·.isConstOf ``VerexIR.Proofs.Denote.reflectedTypeInfo)).isSome then
        return .error "a path reads a reflected type"
      conjuncts := conjuncts.push conjunct
      introduced := introduced.push locals
    let summary := if conjuncts.isEmpty then mkConst ``True
      else conjuncts.pop.foldr mkAnd conjuncts.back!
    withLocalDeclD `summary summary fun premise => do
      -- Each leaf is its conjunct at the locals of its path.
      let mut rest := premise
      for (leaf, index) in leaves.zipIdx do
        let proof ← if index + 1 == leaves.size then pure rest
          else mkAppM ``And.left #[rest]
        if index + 1 < leaves.size then rest ← mkAppM ``And.right #[rest]
        let arguments ← leaf.withContext do
          introduced[index]!.filterM fun local_ => do return !(← local_.fvarId!.getDecl).isLet
        leaf.assign (mkAppN proof arguments)
      let proof ← instantiateMVars root
      if proof.hasMVar || proof.hasSorry then return .error "a path's proof is open"
      let statement ← mkForallFVars parameters (← mkArrow summary statement)
      let value ← mkLambdaFVars (parameters.push premise) proof
      let some locals ← closingLocals (mkApp statement value)
        | return .error "the statement depends on a local definition"
      -- The root's binders are instantiated here: a theorem's are at its end.
      let type ← instantiateMVars (← mkForallFVars locals statement)
      let value ← instantiateMVars (← mkLambdaFVars locals value)
      if type.hasFVar || value.hasFVar || type.hasLevelParam || value.hasLevelParam then
        return .error "the statement is not closed"
      if type.hasMVar || value.hasMVar then return .error "the statement is open"
      return .ok (type, value, conjuncts.size)

/-- A call of a contract-free callee by its derived summary, where one
qualifies: the summary's conjuncts at the call's arguments and
continuations, normalized around them and walked as the caller's goals.
False where the callee is inlined. -/
private def applySummaryAt (ctx : SolveContext) (step : SolveStep) (handle : Lean.Expr)
    (calleeName : String) (compiled : Name) (actionIsCall : Lean.Expr → Bool) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let skip (reason : String) : SolveM Bool := do
    if ctx.profile then IO.println s!"summary skipped {calleeName}: {reason}"
    return false
  let some base := ctx.base? | skip "no base"
  if ctx.handOver then return ← skip "hand over"
  if (← get).activeSummary then return ← skip "activeSummary"
  if !ctx.unrolls.isEmpty then return ← skip "unrolls"
  if !ctx.points.isEmpty then return ← skip "points"
  if ctx.summarizing.contains compiled then return false
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return false
  let action := (target.getArg! 3).consumeMData
  unless actionIsCall action do return false
  -- The meaning at the call reads only the starting context.
  let meaning := action.appFn!
  unless (collectFVars {} meaning).fvarIds.all base.1.contains do return ← skip "base"
  if ← goal.withContext (ctx.preconditions.anyM fun (function, _) => isDefEq function handle) then
    return ← skip "precondition"
  if ← goal.withContext (ctx.invariants.anyM fun (_, function, _) => isDefEq function handle) then
    return ← skip "invariant"
  -- Derived before the module's proofs (`deriveModuleSummary`); a callee
  -- without one inlines.
  let some (some summary) := (← derivedSummaries.atomically get)[compiled]? | skip "no summary"
  -- The summary at the call: its context's locals by unification with the
  -- call, or the caller's facts.
  let instantiate (summary : DerivedSummary) : SolveM (Except MessageData (MVarId × Option MVarId)) := do
    let saved ← Tactic.saveState
    let goal ← getMainGoal
    let premise? ← goal.withContext do
      let (arguments, _, conclusion) ← forallMetaTelescope (← inferType (mkConst summary.name))
      let target ← instantiateMVars (← goal.getType)
      unless conclusion.isAppOfArity ``VerexIR.Proofs.wp 7 do
        return .error m!"the summary of `{calleeName}` does not state a call: {conclusion}"
      -- The call's meaning, continuations and state as they are stated: a call
      -- at other meanings is found without unfolding them.
      let stated := (conclusion.getArg! 3).consumeMData
      let called := (target.getArg! 3).consumeMData
      let same ← withReducible do
        isDefEq stated.appFn! called.appFn! <&&> isDefEq (conclusion.getArg! 4) (target.getArg! 4) <&&>
          isDefEq (conclusion.getArg! 5) (target.getArg! 5) <&&>
          isDefEq (conclusion.getArg! 6) (target.getArg! 6)
      unless same do
        return .error m!"the summary of `{calleeName}` does not match the call"
      -- Its arguments as projections of the call's, unfolding no value.
      unless ← assignSummaryArguments stated.appArg! called.appArg! do
        return .error m!"the summary of `{calleeName}` does not match the arguments of the call"
      -- The precondition is the call's to establish.
      let owed ← if summary.takesRequires then do
          let owed ← mkFreshExprSyntheticOpaqueMVar
            (← instantiateMVars (← inferType arguments[arguments.size - 2]!))
          arguments[arguments.size - 2]!.mvarId!.assign owed
          pure (some owed.mvarId!)
        else pure none
      for argument in arguments.pop do
        let .mvar id ← instantiateMVars argument | continue
        let some local_ ← findLocalDeclWithType? (← instantiateMVars (← id.getType))
          | return .error m!"the summary of `{calleeName}` needs a hypothesis the call lacks: \
              {← instantiateMVars (← id.getType)}"
        id.assign (mkFVar local_)
      let premise ← mkFreshExprSyntheticOpaqueMVar
        (← instantiateMVars (← inferType arguments.back!))
      arguments.back!.mvarId!.assign premise
      goal.assign (← instantiateMVars (mkAppN (mkConst summary.name) arguments))
      pure (.ok (premise.mvarId!, owed))
    if premise?.isOk then return premise?
    saved.restore
    return premise?
  setGoals [goal]
  let values := action.getAppArgs.push (target.getArg! 6)
  let held ← holdContinuations #[target.getArg! 4, target.getArg! 5]
  -- A call whose telescope lacks a hypothesis of the summary, or whose
  -- shape is not the statement's, inlines.
  let (premise, owedRequires) ← match ← instantiate summary with
    | .ok instantiated => pure instantiated
    | .error reason =>
        if ctx.profile then
          IO.println s!"summary skipped {calleeName}: {((← reason.toString).replace "\n" " ").take 300}"
        return false
  -- One goal per conjunct.
  let mut conjuncts := #[]
  let mut rest := premise
  for _ in [1:summary.conjuncts] do
    let type ← rest.withContext do whnfR (← instantiateMVars (← rest.getType))
    let some (left, right) := type.and? | throwError "a summary's premise is not its conjunction"
    let (leftGoal, rightGoal) ← rest.withContext do
      let leftGoal ← mkFreshExprSyntheticOpaqueMVar left
      let rightGoal ← mkFreshExprSyntheticOpaqueMVar right
      rest.assign (mkApp4 (mkConst ``And.intro) left right leftGoal rightGoal)
      pure (leftGoal.mvarId!, rightGoal.mvarId!)
    conjuncts := conjuncts.push leftGoal
    rest := rightGoal
  if summary.conjuncts == 0 then rest.assign (mkConst ``True.intro)
  else conjuncts := conjuncts.push rest
  let mut next := #[]
  for conjunct in conjuncts do
    setGoals [conjunct]
    normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization values
    let mut introduced := #[]
    for goal in ← getGoals do
      let binders ← goal.withContext do
        pure (leadingBinders (← instantiateMVars (← goal.getType)))
      introduced := introduced.push (← goal.introNP binders).2
    setGoals introduced.toList
    releaseContinuations ctx.normalization held walkFixedPoints
    next := next ++ (← getGoals).toArray
  if let some owed := owedRequires then
    setGoals [owed]
    evalTactic (← `(tactic| try verex_normalize))
    queue (← getGoals) (some (.precondition calleeName (ctx.callNumber step.callSite))) #[]
  queue (← rewriteEquations ctx.equations next.toList) provenance clauses
  if ctx.profile then
    IO.println s!"summary applied {calleeName} {((← IO.getNumHeartbeats) - stageStart) / 1000}k"
  charge "summary" stageStart
  return true

private def applyCallSummary (ctx : SolveContext) (step : SolveStep) (handle : Lean.Expr)
    (calleeName : String) (proofType : Lean.Expr) : SolveM Bool := do
  let .const compiled _ := (proofType.getArg! 2).appArg! | return false
  applySummaryAt ctx step handle calleeName compiled
    (·.isAppOfArity ``VerexIR.Proofs.Denote.propheticMeaning 9)

/-- An inlined callee of a model by its summary, on the goal over its body
under its meanings; the state as it was where it does not apply. -/
private def applyModelSummary (ctx : SolveContext) (step : SolveStep) (handle : Lean.Expr)
    (calleeName : String) : SolveM Bool := do
  let target ← step.goal.withContext (instantiateMVars (← step.goal.getType))
  let .const compiled _ := ((target.getArg! 3).consumeMData.getArg! 5) | return false
  let saved ← Tactic.saveState
  if ← applySummaryAt ctx step handle calleeName compiled
      (·.isAppOfArity ``VerexIR.Proofs.Denote.Function.denoteWith 7) then
    return true
  saved.restore
  return false

initialize modelSummaryCall.set applyModelSummary

/-- A call of a verified callee: inlined by its refinement under a model,
inlined by the agreement theorem where it is unspecified, else by its
theorem. `isDenoted` says the goal is a run `denotedRun` reads. -/
private def callStep (ctx : SolveContext) (step : SolveStep) (isDenoted : Bool) : SolveM Bool := do
  let goal := step.goal
  let some (handle, _) ← callGoal? goal ctx.callees | return false
  let some (_, calleeName, theoremProof) ← ctx.callees.findM? fun (candidate, _, _) =>
      goal.withContext (isDefEq candidate handle)
    | throwError m!"no verified callee for {handle}"
  -- Where the proof states a behavioral predicate, the call shows one
  -- of the function value naming its target (`wp_named`).
  let throughContract ← goal.withContext do
    forallTelescope (← inferType theoremProof) fun _ statement =>
      pure (statement.isAppOf ``VerexIR.Proofs.Satisfies)
  let refinement? ← goal.withContext do
    forallTelescope (← whnfR (← zetaReduce (← inferType theoremProof))) fun _ statement =>
      pure (if statement.isAppOf ``VerexIR.Proofs.Spec.Refines then some theoremProof else none)
  let continuations := (← get).regions.map (·.node.name)
  -- The frame of a call a clause reads directly, as its certificate states it.
  let goal ← if ← pure !isDenoted <&&> owesDirectCall goal continuations then
      match ← rewriteEquations ctx.equations [goal] with
      | [rewritten] => pure rewritten
      | _ => pure goal
    else pure goal
  let goal ← if isDenoted then pure goal else namedCall goal throughContract continuations
  let step := { step with goal := goal }
  -- A generic callee's theorem holds at every skolem family and type
  -- instantiation; the call's own are found by unification.
  let (theoremProof, theoremArguments) ← goal.withContext do
    let (arguments, _, _) ← forallMetaTelescope (← inferType theoremProof)
    pure (mkAppN theoremProof arguments, arguments)
  let proofSyntax ← goal.withContext (Lean.Elab.Term.exprToSyntax theoremProof)
  setGoals [goal]
  let proofType ← goal.withContext (do instantiateMVars (← inferType theoremProof))
  if let some refinesProof := refinement? then
    inlineRefinement ctx step refinesProof ((ctx.inlinedHandles.find? (·.1 == handle)).map (·.2))
      calleeName
  else if proofType.isAppOfArity ``Eq 3 then
    unless ← applyCallSummary ctx step handle calleeName proofType do
      setGoals [goal]
      inlineCompiled ctx step handle calleeName theoremProof proofType
  else
    applyCallTheorem ctx step calleeName theoremProof proofType theoremArguments proofSyntax
  return true

/-- The recursive-call leaf of a loop: the invariant at the iteration's
end, normalized so that its conjunction splits into leaves. -/
private def recursiveStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, stageStart, .. } := step
  -- The loop the back edge is of, by its recursive hypothesis.
  let loop ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let .fvar recursive := (target.getArg! 3).getAppFn | return none
    return ((← get).recursiveLoops[recursive]?).getD none
  let some subgoals ← recursiveHypothesis? goal | return false
  setGoals subgoals
  normalizeGoals ctx
  queue (← getGoals) (some (.loopIteration loop)) #[]
  charge "recursive" stageStart
  return true

/-- The stored invariants a write owes: the invariant of each value
written, one leaf per clause. -/
-- Retired with doc/plan/data-and-global-invariants.md, "The stored-invariant layer
-- still exists for the declarations the carriers do not state": the carrier replaces it.
private def storedInvariantsStep (step : SolveStep) : SolveM Bool := do
  let { goal, stageStart, .. } := step
  unless ← writesStoredInvariants goal do return false
  let saved ← Tactic.saveState
  setGoals [goal]
  -- Each write's own obligation is the declaration's invariant of the value
  -- written, which the cut at the mutation's end has already proved of it:
  -- state the case at the resource's type arguments (`verex_stored_cases`)
  -- so that the leaf reads the fact in the context instead of proving the
  -- invariant again.
  let applied ← tryCatch (do
      evalTactic (← `(tactic| verex_data_invariants))
      evalTactic (← `(tactic| all_goals (try verex_stored_cases)))
      pure true)
    fun _ => pure false
  if applied then
    queue (← getGoals) (some .storedWrite) #[]
    charge "stored invariants" stageStart
    return true
  saved.restore
  return false

/-- A literal closure's run, queued as a run `denotedRun` reads. -/
private def denotedRunStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let some run ← denotedRun? goal ctx.callees | return false
  queue [run] provenance clauses
  modify fun s => { s with denoted := s.denoted.push run }
  charge "denoted run" stageStart
  return true

/-- Independent obligations at a region boundary. Markers and quantified
clauses are atomic here; their proof-only decomposition inherits one owner. -/
private theorem optionElim_and (slot : Option α) (left right : α → Prop) :
    slot.elim False (fun value => left value ∧ right value) ↔
      slot.elim False left ∧ slot.elim False right := by
  cases slot <;> simp

private partial def obligationRoutes (target : Lean.Expr) : Array (List Bool) :=
  let target := target.consumeMData
  if target.isAppOfArity ``And 2 then
    (obligationRoutes (target.getArg! 0)).map (false :: ·) ++
      (obligationRoutes (target.getArg! 1)).map (true :: ·)
  else #[[]]

private def allocateOwners (ctx : SolveContext) (goal : MVarId)
    (routes : Array (List Bool)) : SolveM Unit := do
  let limit := verex.leafHeartbeats.get (← getOptions) * 1000
  let mut ledger ← ctx.budgets.get
  let mut owners := #[]
  let debits := (← get).activeDebits
  for route in routes do
    let some (next, owner) := ledger.allocateAfter limit debits
      | throwError "an automatic clause lost its earlier preparation debit"
    ledger := next
    owners := owners.push (route, owner)
  ctx.budgets.set ledger
  modify fun s => { s with
    activeOwners := owners
    goalOwners := s.goalOwners.insert goal owners
    prepaidOwners := if debits.isEmpty then s.prepaidOwners
      else owners.foldl (fun found owner => found.insert owner.2) s.prepaidOwners }

/-- A structural node, split once: a leaf handed over is one per path and
clause, since the case analysis of its own conditionals belongs to the
proof. -/
private def splitStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  let prepared := ctx.preparing?.isSome || ctx.preparesPaths
  let prepared := prepared && (← get).prepared.contains goal
  let owners := (← get).activeOwners
  if !owners.isEmpty then
    let target ← goal.withContext (instantiateMVars (← goal.getType))
    -- Independent source clauses have already been routed. Keep logical
    -- conjuncts of one clause together for preparation and cheap decisions;
    -- the leaf can decompose them later under their existing owner. A
    -- conjunction guarding folded continuations still executes the program,
    -- including an inlined callback whose frame a precondition must prove.
    if target.isAppOfArity ``And 2 then
      let folded ← goal.withContext do
        (collectFVars {} target).fvarIds.anyM fun id => do
          let declaration ← id.getDecl
          return declaration.isLet && declaration.userName == continuationName
      let regions := (← get).regions
      let pending := folded || (target.find? fun term =>
        term.isAppOf ``VerexIR.Proofs.wp || regions.any (term.isAppOf ·.node.name)).isSome
      unless pending do return false
    -- A successful path must refute each alternative of the aggregate
    -- abort condition. Expose these proof destinations before deciding
    -- them, retaining the same owner for all of their attempts.
    if target.isAppOfArity ``Not 1 && (target.getArg! 0).isAppOfArity ``Or 2 then
      setGoals [goal]
      evalTactic (← `(tactic| simp only [not_or]))
      queue (← getGoals) provenance clauses prepared
      charge "negative alternatives" stageStart
      return true
    -- Independent source clauses have already been routed by
    -- `partitionOwners`. Conjunctions here are logical children of the
    -- same occurrence, and `queue` retains its existing allowance.
  let splitTarget ← if ctx.debug then goal.withContext do
      pure (some (← instantiateMVars (← goal.getType))) else pure none
  let entries := (← get).regions.filterMap (fun region => region.node.relation?.map (·.constName!))
  let shared := (← get).activeSummary
  let hasEntry ← if !shared || owners.isEmpty then pure false
    else if (← get).activeEntryOpened then pure true else goal.withContext do
    (← getLCtx).anyM fun decl => do
      let type ← instantiateMVars decl.type
      return type.getAppFn.constName?.any entries.contains
  let readsMemory ← if !shared || owners.isEmpty || hasEntry then pure false
    else goal.withContext do
      let target ← instantiateMVars (← goal.getType)
      return !(storageAddresses target).isEmpty
  let readsMap ← if !shared || owners.isEmpty || hasEntry then pure false
    else readsConditionalMap goal
  let (parentTarget, parentLocals) ← goal.withContext do
    return (← instantiateMVars (← goal.getType), (← getLCtx).getFVarIds)
  let some (subgoals, conditional) ← splitOnce goal
    -- Resolve a read across a memory write before general normalization.
    -- Its address cases remain within the existing clause allowance.
    -- Abstract joined values are linked to their incoming rows in
    -- `solveLeaf` before deciding their variants and conditionals. An
    -- obligation the context states, such as a callee's assumed axiom,
    -- closes whole under a summary as on the tree walk.
    (obligations := !ctx.handOver)
    (cases := !ctx.handOver && (!shared || readsMemory || readsMap))
    (prepare := !shared || !owners.isEmpty) | return false
  let splitDone ← IO.getNumHeartbeats
  -- Unrolling takes a branch's first case first, a loop's next
  -- iteration, so a path outrunning the bound is met early.
  let subgoals := if ctx.unrolls.isEmpty then subgoals else subgoals.reverse
  for subgoal in subgoals do
    setGoals [subgoal]
    let target ← subgoal.withContext (instantiateMVars (← subgoal.getType))
    if target.isAppOf ``VerexIR.Proofs.wp then
      if subgoals.length > 1 then
        countStructure fun counts => { counts with walks := counts.walks + 1 }
      let consumed ← if provenance matches some (.continuation _ _) then do
          timedStep "split/consume" (evalTactic (← `(tactic| try verex_consume)))
          pure ((← getGoals) != [subgoal])
        else pure false
      if conditional || consumed then
        -- The parts of the target the split's facts do not read are normal
        -- as the target was.
        let normal ← (← getMainGoal).withContext do
          let mut affected : FVarIdSet := {}
          for decl in ← getLCtx do
            if decl.isImplementationDetail || parentLocals.contains decl.fvarId then continue
            affected := affected.insert decl.fvarId
            affected := (collectFVars { fvarSet := affected } (← instantiateMVars decl.type)).fvarSet
          return unaffectedParts parentTarget affected
        normalizeAround (fixedPoints := walkFixedPoints) ctx.normalization normal
    queue (← getGoals) provenance clauses prepared
  let finished ← IO.getNumHeartbeats
  if let some target := splitTarget then
    let head := match target.getAppFn with
      | .const name _ => toString name
      | _ => "?"
    let action := if target.isAppOfArity ``VerexIR.Proofs.wp 7 then
        match (target.getArg! 3).getAppFn with
        | .const name _ => s!" of {name}"
        | _ => ""
      else ""
    logInfo m!"split {head}{action}: {subgoals.length} goals; split {(splitDone - stageStart) / 1000}k, normalize {(finished - splitDone) / 1000}k"
  charge "split" stageStart
  return true

/-- A leaf: a literal closure's run by its target's body where the target
has no theorem and its abort by its body where the proof inlines it, its
behavior by its target's theorem, then `solveLeaf`, or, with `handOver`,
the leaf of an authored proof, named by what it owes (`leafName`). -/
private def prepareLeafStep (ctx : SolveContext) (step : SolveStep) : SolveM (Option MVarId) := do
  let pins ← sharedPins
  withSharedPinsLifted do
  let goal ← substitutePinEquations step.goal pins
  let step := { step with goal }
  let { goal, provenance, clauses, stageStart, .. } := step
  let shared := (← get).activeSummary
  let observesBehavior ← if !shared then pure false else goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    return (← statesBehavior #[target]) &&
      (target.find? (·.isAppOf ``VerexIR.Proofs.wp)).isNone
  if ctx.labeled || shared then
    let entries := (← get).regions.filterMap (fun region => region.node.relation?.map (·.constName!))
    let hasEntry ← if (← get).activeEntryOpened then pure true else goal.withContext do
      (← getLCtx).anyM fun decl => do
        let type ← instantiateMVars decl.type
        return type.getAppFn.constName?.any entries.contains
    let needsEntry ← if !hasEntry || ctx.labeled || observesBehavior then pure hasEntry
      else goal.withContext do
        -- Structural introductions may already have moved the invocation's
        -- observations from the target into its premises. Their concrete
        -- closure is still supplied by this entry relation.
        let statements ← (← getLCtx).foldlM (init := #[]) fun found decl => do
          if decl.isImplementationDetail then return found
          return found.push (← instantiateMVars decl.type)
        statesBehavior statements
    if needsEntry then
      -- A witness may be a point inside one incoming branch, and a joined
      -- closure's behavior depends on the selected implementation. Expose
      -- these alternatives before deriving behavior or choosing witnesses,
      -- under the same owner used by all of this clause's later decisions.
      let alternatives ← exposeNormalEntries ctx.normalization goal entries
      let alternatives ← alternatives.mapM fun goal => goal.withContext do
        let points := (← getLCtx).foldl (init := #[]) fun points decl =>
          if decl.type.isAppOf ``ProgramPoint && !decl.isImplementationDetail then
            points.push (decl.fvarId, true)
          else points
        if points.isEmpty then pure goal else markDetails goal points
      setGoals alternatives
      -- The suffix was generated over an abstract row. Linking that row
      -- leaves previously introduced slot equalities with concrete tuple
      -- projections; expose and substitute them before choosing a witness.
      evalTactic (← `(tactic| all_goals verex_reduce_projections))
      evalTactic (← `(tactic| all_goals verex_split_hypotheses))
      evalTactic (← `(tactic| all_goals verex_subst_vars))
      evalTactic (← `(tactic| all_goals verex_reduce_projections))
      evalTactic (← `(tactic| all_goals (try verex_normalize_context_stage)))
      modify fun s => { s with activeEntryOpened := false }
      queue (← getGoals) provenance clauses
      charge "entry observations" stageStart
      return none
  -- Decide completed observations before introducing hypothetical aborts
  -- and runs. In particular, an early return can already fix a conditional
  -- result without observing any invocation on its unreachable other arm.
  -- This attempt and any later preparation share the current owner's cap.
  if observesBehavior then
    let started ← IO.getNumHeartbeats
    let restoreCaches ← saveWalkCaches
    let closed ← closesBy goal (← `(tactic| verex_shared_call_observations))
    unless closed do restoreCaches
    charge "early call observations" started
    if closed then return none
  let goal ← introduceBehaviors goal
  if let some cases ← behaviorAlternatives? goal then
    queue cases provenance clauses
    charge "behavior alternatives" stageStart
    return none
  if let some cases ← abortCases? goal then
    queue cases provenance clauses
    charge "abort cases" stageStart
    return none
  if ← denotedRunStep ctx { step with goal := goal } then return none
  -- A literal closure's behavior, by its target's theorem.
  let behaviorStart ← IO.getNumHeartbeats
  let goal ← terminatingRuns goal
  let terminatingDone ← IO.getNumHeartbeats
  let some goal ← rewriteResultOf goal
    | charge "result/state rewriting" terminatingDone; return none
  let rewritingDone ← IO.getNumHeartbeats
  let derivationStart ← IO.getNumHeartbeats
  if ← behaviorWitness goal ctx.callees then
    charge "behavior witness" derivationStart
    return none
  let dispatched ← dispatchBehavior goal ctx.callees
  let now ← IO.getNumHeartbeats
  let costs := #[("terminating facts", terminatingDone - behaviorStart),
    ("result/state rewriting", rewritingDone - terminatingDone),
    ("behavior facts", now - derivationStart)]
  modify fun s => { s with stageCost := s.stageCost ++ costs }
  return dispatched

/-- Reporting and the final decision are separate from preparation: the
decision already charges the same owner, without double-debiting its work. -/
private def decideLeaf (ctx : SolveContext) (step : SolveStep) : SolveM Unit := do
  let { goal, provenance, clauses, .. } := step
  setGoals [goal]
  modify fun s => { s with leaves := s.leaves + 1 }
  if !ctx.handOver && !ctx.summarizing.isEmpty then
    -- A leaf a summary cannot state ends the derivation here, not after the
    -- rest of the body's walk.
    unless summaryStates provenance clauses do
      throwError "a path owes an obligation of the callee's own: \
        {(provenance.map fun p => s!"{p.key.kind} {p.key.callee}").getD "none"}, {clauses.size} clauses"
    -- A callee's precondition holds, or is what its callers owe.
    if provenance matches some (.precondition ..) then
      let saved ← Tactic.saveState
      let (closed, _) ← solveLeaf ctx.normalization goal provenance clauses #[]
      if closed then return
      saved.restore
      setGoals [goal]
  if ctx.handOver || !ctx.summarizing.isEmpty then
    (← targetState).leafOrigins.modify (·.insert goal (provenance, clauses))
    let key ← leafKey goal provenance clauses ctx.names
    -- A path returns where it aborts nowhere and owes no abort's condition.
    let decisions := (← get).activeDecisions
    let aborting := decisions.any (· matches .aborts _) ||
      (!(provenance matches some .returns) && (key.kind == "aborts_if" || key.kind == "aborts_with"))
    let decisions := if aborting then decisions else decisions.push .returns
    (← targetState).leafDecisions.modify (·.insert goal decisions)
    modify fun s => { s with handed := s.handed.push (goal, key) }
    if !ctx.handOver then
      let handed := (← get).handed.size
      if handed > verex.summaryLeaves.get (← getOptions) then throwError "{handed} paths"
    return
  -- Verification is native (`doc/designs/native-verification.md`).
  let pins ← sharedPins
  withSharedPinsLifted do
  let goal ← substitutePinEquations goal pins
  setGoals [goal]
  if (← (← targetState).runtimeMention.get) matches .none then
    recordRuntimeValues m!"the leaf `{(← leafKey goal provenance clauses ctx.names).text}`" goal
  let entries := (← get).regions.filterMap (fun region => region.node.relation?.map (·.constName!))
  let owners := (← get).activeOwners.map (·.2)
  let (closed, costs) ← solveLeaf ctx.normalization goal provenance clauses entries
    (if (← get).activeSummary then some owners else none)
    (← get).activeEntryOpened (← get).activeOwed
  modify fun s => { s with stageCost := s.stageCost ++ costs }
  if ctx.debug then
    IO.println s!"leaf {(← get).leaves}: {if closed then "closed" else "undecided"}"
  -- An unrolled proof cannot succeed once a path outruns its bound; the
  -- remaining paths would only spend the budget.
  if !closed && provenance matches some (.unrollBound _) then
    for (other, _, _, _) in (← get).pending do
      unless ← other.isAssigned do admitGoal other
    -- Joins also retain proof destinations outside the ordinary queue.
    -- A known unroll failure stops all of them without sealing a partial
    -- entry relation or caching an admitted summary as a successful proof.
    for region in (← get).regions do
      for arrival in region.node.arrivals do
        unless ← arrival.goal.isAssigned do admitGoal arrival.goal
      if let some summary := region.node.summary? then
        unless ← summary.isAssigned do admitGoal summary
    modify fun s => { s with
      pending := #[]
      regions := s.regions.map fun region =>
        { region with node := { region.node with sealed := true, finished := true } } }

/-- The premise every clause of a conjunction states first, such as the
continuation's result equation, or `none`. -/
private partial def sharedPremise? (target : Lean.Expr) : Option Lean.Expr :=
  let target := target.consumeMData
  if target.isAppOfArity ``And 2 then do
    let left ← sharedPremise? (target.getArg! 0)
    let right ← sharedPremise? (target.getArg! 1)
    if left == right then some left else none
  else if target.isArrow then some target.bindingDomain!
  else none

/-- The conjunction with the shared premise dropped from each clause. -/
private partial def withoutPremise (target : Lean.Expr) : Lean.Expr :=
  let target := target.consumeMData
  if target.isAppOfArity ``And 2 then
    mkAnd (withoutPremise (target.getArg! 0)) (withoutPremise (target.getArg! 1))
  else target.bindingBody!

/-- A proof of the clauses from `proof`, a proof of the clauses without
their shared premise under it. -/
private partial def underPremise (premise target proof : Lean.Expr) : MetaM Lean.Expr := do
  let target := target.consumeMData
  unless target.isAppOfArity ``And 2 do return proof
  let part (projection : Name) (clause : Lean.Expr) : MetaM Lean.Expr := do
    let inner ← withLocalDeclD `h premise fun h => do
      mkLambdaFVars #[h] (← mkAppM projection #[mkApp proof h])
    underPremise premise clause inner
  mkAppM ``And.intro #[← part ``And.left (target.getArg! 0), ← part ``And.right (target.getArg! 1)]

/-- The premises every clause of a path states, introduced once into the
path's context, so that its preparation reads them as each leaf would. -/
private partial def introSharedPremises (goal : MVarId) : MetaM MVarId := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.consumeMData.isAppOfArity ``And 2 do return goal
  let some premise := sharedPremise? target | return goal
  let name := (target.find? (·.isArrow)).map (·.bindingName!) |>.getD `h
  let hoisted ← mkFreshExprSyntheticOpaqueMVar
    (.forallE name premise (withoutPremise target) .default) (← goal.getTag)
  goal.assign (← underPremise premise target hoisted)
  let (_, next) ← hoisted.mvarId!.intro1P
  introSharedPremises next

/-- `verex_prepare_path` on `goal` within one leaf allowance, with its
clauses' markers, or `none`, the state restored, where it fails. -/
private def preparedPath? (goal : MVarId) : TacticM (Option (List MVarId)) := do
  let leafBudget := verex.leafHeartbeats.get (← getOptions) * 1000
  let saved ← saveState
  let restoreCaches ← saveWalkCaches
  tryCatchRuntimeEx
    (do
      setGoals [← introSharedPremises goal]
      let prepare : TacticM Unit :=
        withOptions (fun options => verex.keepTargetLocals.set
            (verex.keepObligationMarkers.set options true) true) do
          Lean.Elab.Tactic.withoutRecover (evalTactic (← `(tactic| verex_prepare_path)))
      if leafBudget == 0 then prepare else withBudget leafBudget prepare
      pure (some (← getUnsolvedGoals)))
    fun failure => do
      if failure.isMaxHeartbeat then throw failure
      saved.restore
      restoreCaches
      pure none

/-- An obligation a hypothesis states closes as it is, before preparation
splits that hypothesis (`splitOnce`). -/
private def closesStated (goal : MVarId) : TacticM Bool := do
  return (← StatedFacts.close goal (exact := true)) ||
    (← closesBy goal (← `(tactic| verex_assumption)))

/-- Prepare one completed region before its independent clauses split. The
common work is the path's preparation (`preparedPath?`); bounds and map
positions stay each leaf's, as on the default walk (`verex_prepare_path`). -/
private def prepareAutomatic (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  unless (← get).activeSummary do return false
  -- Owners that served the walk before its clauses existed are atomic: a
  -- completed group splits them, each clause keeping what they spent.
  let inherited := (← get).activeOwners
  unless inherited.all (·.1.isEmpty) do return false
  let { goal, provenance, clauses, stageStart, .. } := step
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  -- A write's aggregate expands into independent source invariants. Let
  -- its structural rule expose them before allocating their owners; a false
  -- invariant must not exhaust the allowance of the other stored values.
  if (← writesStoredInvariants goal) ||
      (target.isArrow && target.bindingBody!.isAppOfArity
        ``VerexIR.Proofs.Denote.DataInvariants 4) then return false
  if (target.find? (·.isAppOf ``VerexIR.Proofs.wp)).isSome then return false
  let regions := (← get).regions
  if (target.find? fun term => regions.any (term.isAppOf ·.node.name)).isSome then return false
  if ← goal.withContext do
      (collectFVars {} target).fvarIds.anyM fun id => do
        let decl ← id.getDecl
        return decl.isLet && decl.userName == continuationName
    then return false
  -- A call's result binder can precede several source clauses. Introduce
  -- that structural binder before assigning their independent owners. A
  -- quantifier inside a marker remains in its clause's single leaf attempt.
  if clauses.isEmpty && target.isForall &&
      (target.find? (·.isAppOf ``VerexIR.Proofs.Obligation)).isSome then return false
  if !inherited.isEmpty && !(clauses.isEmpty && (obligationRanges target).size > 1 &&
      (obligationRoutes target).size > 1) then
    return false
  -- A joined row is abstract: decoding its slots can wrap several source
  -- clauses in Option.elim. Distribute that structural wrapper over their
  -- conjunction so a false clause cannot consume its siblings' allowance
  -- or report them as failed. This does not enumerate incoming paths.
  let mut goal := goal
  if clauses.isEmpty && (obligationRanges target).size > 1 then
    setGoals [goal]
    evalTactic (← `(tactic| try simp only [optionElim_and]))
    let some next := (← getGoals).head? | return true
    goal := next
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  let routes := if clauses.isEmpty then obligationRoutes target
    else Array.replicate clauses.toList.eraseDups.length []
  if inherited.isEmpty then allocateOwners ctx goal routes
  else
    let (ledger, ids) := (← ctx.budgets.get).split (inherited.map (·.2)) routes.size
    ctx.budgets.set ledger
    let owners := routes.zip ids
    modify fun s => { s with activeOwners := owners, goalOwners := s.goalOwners.insert goal owners }
  if routes.size < 2 then
    queue [goal] provenance clauses
    return true
  -- The path's preparation precedes the split of its clauses, as on the
  -- existing path, keeping their routes.
  if ctx.preparesPaths && !(← get).prepared.contains goal then
    if ← closesStated goal then
      charge "stated path" stageStart
      return true
    let saved ← (saveState : TacticM Tactic.SavedState)
    let restoreCaches ← saveWalkCaches
    let discard : SolveM Unit := do (saved.restore : TacticM Unit); restoreCaches
    -- The group's common preparation, debited once to each clause's owner.
    let (outcome, _) ← runAutomatic ctx.budgets ((← get).activeOwners.map (·.2)) .preparation
      (preparedPath? goal)
    let prepared := match outcome with
      | .done prepared => prepared
      | .exhausted | .failed _ => none
    let prepared ← prepared.mapM (·.mapM fun next => next.withContext do
      let target ← instantiateMVars (← next.getType)
      if target.isMData then next.replaceTargetDefEq target.consumeMData else pure next)
    let keepsRoutes ← (prepared.getD []).allM fun next => do
      return !clauses.isEmpty || obligationRoutes (← instantiateMVars (← next.getType)) == routes
    match prepared with
    | some [next] =>
        if keepsRoutes then goal := next else discard
    | some goals =>
        if keepsRoutes then
          queue goals provenance clauses true
          charge "path preparation" stageStart
          return true
        discard
    | none => pure ()
    modify fun s => { s with prepared := s.prepared.insert goal }
  queue [goal] provenance clauses
  charge "automatic preparation" stageStart
  return true

/-- The automatic preparation of a completed path (`preparedPath?`), once,
before its clauses or a clause's cases split (`splitStep`), after the
structural steps that read its form; the leaves of the clauses and cases start
from it. A failed preparation leaves the path as it was. -/
private def preparePath (ctx : SolveContext) (step : SolveStep) (target : Lean.Expr) :
    SolveM Bool := do
  let { goal, provenance, clauses, stageStart, .. } := step
  unless ctx.preparesPaths && !(← get).prepared.contains goal do return false
  let target := target.consumeMData
  -- Only where the split leaves siblings: clauses, or a clause's cases.
  unless target.isAppOfArity ``And 2 || (target.isAppOfArity ``VerexIR.Proofs.Obligation 4 &&
      (← goal.withContext (findSplit? target)).isSome) do
    return false
  if (target.find? (·.isAppOf ``VerexIR.Proofs.wp)).isSome then return false
  -- An arrival hole at a shared continuation is proved by its summary.
  let regions := (← get).regions
  if (target.find? fun term => regions.any (term.isAppOf ·.node.name)).isSome then return false
  if ← goal.withContext do
      (collectFVars {} target).fvarIds.anyM fun id => do
        let declaration ← id.getDecl
        return declaration.isLet && declaration.userName == continuationName
    then return false
  -- A completed path is compared with no arrival: its leaves' form, without
  -- the shared fold pins (`substitutePinEquations`).
  let pins ← sharedPins
  withSharedPinsLifted do
  let goal ← substitutePinEquations goal pins
  if ← closesStated goal then
    charge "stated path" stageStart
    return true
  let prepared ← preparedPath? goal
  let goals ← (prepared.getD [goal]).mapM fun next => next.withContext do
    let target ← instantiateMVars (← next.getType)
    if target.isMData then next.replaceTargetDefEq target.consumeMData else pure next
  queue goals provenance clauses true
  charge (if prepared.isSome then "path preparation" else "failed path preparation") stageStart
  return true

/-- Prepare a completed path before splitting its proof-only conjunctions and
binders. Program steps must still be traversed first. Logical children inherit
the preparation; a newly exposed computation can reach another preparation site. -/
private def prepareObligations (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let some preparing := ctx.preparing? | return false
  let { goal, provenance, clauses, stageStart, .. } := step
  if (← get).prepared.contains goal then return false
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  -- These are path binders, including a callee's results and assumptions.
  -- Clause-specific binders remain inside their conjunction or marker.
  if target.isForall then return false
  -- A binding site the walk names values at comes first.
  if target.isAppOf ``LetIn || target.isAppOf ``LetRowIn || !(← resultSites target).isEmpty then
    return false
  if (target.find? (·.isAppOf ``VerexIR.Proofs.wp)).isSome then return false
  -- A folded continuation still contains program, even when no `wp` is
  -- syntactically visible. Its values must reach `exitStep` before a proof
  -- script can generalize or clear the computation's representation.
  if ← goal.withContext do
      (collectFVars {} target).fvarIds.anyM fun id => do
        let declaration ← id.getDecl
        return declaration.isLet && declaration.userName == continuationName
    then return false
  setGoals [goal]
  -- Whether a proof's `on` steps take several of the path's clauses, which
  -- then share the path's preparation; a path with one such clause leaves
  -- it to its leaf.
  let routed ← match ← (← targetState).routedKeys.get with
    | none => pure true
    | some (patterns, all) => do
        let keys ← pathKeys goal provenance clauses ctx.names
        let taken := if all then keys else keys.filter fun key => patterns.any (·.selects key)
        pure (taken.size ≥ 2)
  (← targetState).pathRouted.set routed
  withOptions (verex.keepObligationMarkers.set · true) do
    evalTactic preparing
  -- Substitution and generalization can leave target annotations. The
  -- structural matchers read the logical head, just as at solver entry.
  let goals ← queueOwed .exit (← getUnsolvedGoals) true
  let goals ← goals.mapM fun next => next.withContext do
    let target ← instantiateMVars (← next.getType)
    if target.isMData then next.replaceTargetDefEq target.consumeMData else pure next
  queue goals provenance clauses true
  charge "authored preparation" stageStart
  return true

/-- The operations and invocations a hook names (`after op k`), marked so
that every path continuing after one carries its site (`opAfterAt`). -/
private def markPointSites (ctx : SolveContext) (goal : MVarId) : MetaM MVarId := do
  let named := ctx.sites.filterMap fun (site, kind, number, _) =>
    if (kind == "op" && ctx.points.any (·.1 == .after (.op number))) ||
        (kind == "invoke" && ctx.points.any (·.1 == .after (.invoke number))) then some site
    else none
  if named.isEmpty then return goal
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless (target.find? (·.isAppOf ``opAt)).isSome do return goal
    let marked := target.replace fun e =>
      if e.isAppOf ``opAt && e.getAppNumArgs ≥ 2 then
        let site := e.getArg! (e.getAppNumArgs - 2)
        if site.rawNatLit?.any named.contains || site.nat?.any named.contains then
          some (mkAppN (.const ``opAfterAt e.getAppFn.constLevels!) e.getAppArgs)
        else none
      else none
    if marked == target then return goal
    goal.replaceTargetDefEq marked

/-- The function's entry: the first goal of the root path that is a step of
its body, where the hooks `at entry` run. -/
private def entryStep (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  if (← get).entryReached || !ctx.points.any (·.1 == .entry) then return false
  let target ← step.goal.withContext (instantiateMVars (← step.goal.getType))
  -- A source `let` the body starts with is a step of it (`LetIn`).
  unless target.isAppOf ``VerexIR.Proofs.wp || target.isAppOf ``LetIn ||
      target.isAppOf ``LetRowIn do return false
  modify fun s => { s with entryReached := true }
  setGoals [step.goal]
  atPoint ctx .entry
  queue (← getGoals) step.provenance step.clauses (callSite := step.callSite)
  return true

/-- A returning path's obligation that no abort condition holds: the marker
comes off, the goal is the contract's, and the leaf records the abort clauses'
ranges. -/
private def takeOffNoAbort (ctx : SolveContext) (step : SolveStep) : SolveM SolveStep := do
  let target ← step.goal.withContext do instantiateMVars (← step.goal.getType)
  unless target.isAppOfArity ``NoAbort 1 do return step
  let goal ← step.goal.replaceTargetDefEq target.appArg!
  let ranges := ctx.names.filterMap fun (range, name) =>
    if (clauseKey name).kind == "aborts_if" then some range else none
  return { step with goal, provenance := some .returns, clauses := step.clauses ++ ranges }

/-- Advance the walker until it queues successors or reaches a decision.
Once an automatic owner exists, every transformation here belongs to its
allowance, including newly exposed computation and behavior alternatives. -/
private def advance (ctx : SolveContext) (step : SolveStep) : SolveM (Option SolveStep) := do
  let step ← takeOffNoAbort ctx step
  let { goal, provenance, clauses, .. } := step
  if ctx.preparing?.isSome && (← prepareObligations ctx step) then return none
  let isDenoted := (← get).denoted.contains goal
  let prepared := (ctx.preparing?.isSome || ctx.preparesPaths) && (← get).prepared.contains goal
  let (goal, clauses) ← match provenance with
    | some (.loopEntry _) | some (.loopIteration _) | some .assertion | some .mutationEnd
    | some .construction | some .memoryWritten | some .callReturned
    | some (.precondition _ _) =>
        stripObligations goal clauses
    | _ => pure (goal, clauses)
  let goal ← markPointSites ctx goal
  -- A path through a branch or an arm takes it, and one owing the abort
  -- condition at a site aborts there: the markers come off, and the path
  -- takes the decisions.
  let (goal, after) ← match ← goal.withContext do pathMarkers? (← instantiateMVars (← goal.getType)) with
    | some (marks, stripped) =>
        let decisions := marks.filterMap fun
          | .branch site taken => ctx.branchDecision site taken
          | .arm site variant => (ctx.siteNumber "match" site).map (.variant · variant)
          | .abort site => ctx.abortDecision site
          | .dispatch site target => (ctx.siteNumber "invoke" site).map (.invoke · target)
          | .after _ => none
        modify fun s => { s with activeDecisions := s.activeDecisions ++ decisions }
        let after := marks.filterMap fun
          | .after site => match ctx.callSites.find? (·.1 == site), ctx.siteNumber "op" site,
                ctx.siteNumber "invoke" site with
              | some (_, callee, number, overall), _, _ =>
                  some (ProgramPointKey.after (.call number (identifierPart callee) overall))
              | none, some number, _ => some (.after (.op number))
              | none, none, some number => some (.after (.invoke number))
              | none, none, none => none
          | _ => none
        pure (← goal.replaceTargetDefEq stripped, after)
    | none => pure (goal, #[])
  -- Where a path continues after a call or an operation a proof names, the
  -- point's hooks wait until the walker has introduced the call's results
  -- and clauses or the operation's range conditions, under their names.
  modify fun s => { s with activePoints := s.activePoints ++ after }
  let pending := (← get).activePoints
  unless pending.isEmpty do
    let target ← goal.withContext do instantiateMVars (← goal.getType)
    unless target.consumeMData.isForall do
      modify fun s => { s with activePoints := #[] }
      setGoals [goal]
      for point in pending do atPoint ctx point
      queue (← getGoals) provenance clauses (callSite := step.callSite)
      return none
  if prepared then modify fun s => { s with prepared := s.prepared.insert goal }
  if ← goal.isAssigned then return none
  setGoals [goal]
  let step := { step with goal, clauses }
  if ctx.debug then IO.println s!"→ {← debugHead goal}"
  if ← entryStep ctx step then return none
  if ← prepareAutomatic ctx step then return none
  if ← sharedExitStep ctx step then return none
  if ← exitStep ctx step then return none
  if ← assumeStep ctx step then return none
  if ← callAtStep ctx step then return none
  if ← abortStep step then return none
  if ← callCheckedStep ctx step then return none
  if ← bindStep ctx step then return none
  if ← unrollStep ctx step then return none
  if ← loopStep ctx step then return none
  if ← assertionStep ctx step then return none
  if ← memoryWrittenStep ctx step then return none
  if ← ghostWriteStep ctx step then return none
  if ← dataInvariantStep ctx step then return none
  if ← anchorStep ctx step then return none
  if ← invocationSteps ctx step then return none
  if ← callStep ctx step isDenoted then return none
  if ← recursiveStep ctx step then return none
  if ← storedInvariantsStep step then return none
  -- A literal closure's run is read once for all conjuncts. For completed
  -- obligations, allocation above already includes every benefiting clause.
  -- The clauses of a proof's obligations split first: their keys route them.
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  if target.isAppOfArity ``And 2 && !(ctx.handOver && !(obligationRanges target).isEmpty) then
    if ← denotedRunStep ctx step then return none
  if ← preparePath ctx step target then return none
  if ← splitStep ctx step then return none
  if (← get).activeSummary && (← get).activeOwners.isEmpty then
    -- A stalled computation may need proof preparation to expose its next
    -- step. Allocate before trying that preparation, then resume bounded.
    allocateOwners ctx goal #[[]]
    queue [goal] provenance clauses
    return none
  let some goal ← prepareLeafStep ctx step | return none
  return some { step with goal }

/-- Tactic and scheduler state commit together. Failed work still debits
its owners, but cannot retain queued goals or half-built summaries. -/
private def runOwned (ctx : SolveContext) (owners : Array (List Bool × SharedBudgets.OwnerId))
    (action : SolveM α) : SolveM (SharedBudgets.Outcome α) := do
  if owners.isEmpty then return .done (← action)
  let state ← get
  let (outcome, _) ← runAutomatic ctx.budgets (owners.map (·.2)) .leaf (action.run state)
  match outcome with
  | .done (value, state) => set state; return .done value
  | .exhausted => return .exhausted
  | .failed failure => return .failed failure

/-- Share one scoped discriminator before separating source clauses. Every
case keeps every proof destination and the same owners; no allowance is
renewed. Deeper proof search remains with the ordinary leaf pipeline. -/
private def splitSharedCase (step : SolveStep) : SolveM Bool := do
  let state ← get
  let owners := state.activeOwners
  if owners.size < 2 || owners.any (·.1.isEmpty) then return false
  if ← closesBy step.goal (← `(tactic| verex_assumption)) then
    charge "common immediate" step.stageStart
    return true
  let entries := state.regions.filterMap (fun region => region.node.relation?.map (·.constName!))
  let hasEntry ← step.goal.withContext do
    (← getLCtx).anyM fun decl => do
      let type ← instantiateMVars decl.type
      return type.getAppFn.constName?.any entries.contains
  if hasEntry then
    -- The program suffix has already reached its proof group. Open its
    -- incoming relation once before routing the source clauses, so their
    -- proofs share the same row substitutions and case eliminators. The
    -- enclosing `runOwned` debits this proof decomposition to every owner.
    let failures ← IO.mkRef #[]
    let goals ← SharedContinuations.exposeEntries step.goal entries closeFrameWrites
      (exposeObserver false failures)
    countAlternatives goals
    reportExposure (← failures.get) goals
    modify fun s => { s with activeEntryOpened := true }
    queue goals step.provenance step.clauses
    charge "common entry" step.stageStart
    return true
  -- The newly linked context needs the same native/behavior preparation
  -- as an entry opened by a leaf, before deciding its proof variants.
  if state.activeEntryOpened then return false
  if owners.any (fun owner => state.caseScheduled.contains owner.2) then return false
  let some candidate ← SharedCases.find? step.goal | return false
  let (goals, stage) ← step.goal.withContext do
    match candidate with
    | .condition proposition decision =>
        let (positive, negative) ← step.goal.byCasesDec proposition decision `conditionHolds
        pure ([negative.mvarId, positive.mvarId], "common conditional")
    | .variant value =>
        let cases ← step.goal.cases value.fvarId!
        pure ((cases.map (·.mvarId)).toList.reverse, "common variant")
  modify fun s => { s with caseScheduled := owners.foldl (fun found owner => found.insert owner.2) s.caseScheduled }
  -- As a split's cases (`splitStep`), they keep the path's preparation.
  queue goals step.provenance step.clauses ((← get).prepared.contains step.goal)
  charge stage step.stageStart
  return true

/-- Routing independent clauses is proof bookkeeping, even when one owner
is exhausted. It must still expose the other clauses and their diagnostics;
no normalization or decision work runs while partitioning their owners. -/
private def partitionOwners (ctx : SolveContext) (step : SolveStep) : SolveM Bool := do
  let owners := (← get).activeOwners
  unless owners.any (!·.1.isEmpty) do return false
  let start ← IO.getNumHeartbeats
  let target ← step.goal.withContext (instantiateMVars (← step.goal.getType))
  unless target.isAppOfArity ``And 2 && owners.all (!·.1.isEmpty) do
    throwError "an automatic obligation group lost its clause routes"
  let subgoals ← step.goal.apply (mkConst ``And.intro)
  unless subgoals.length == 2 do throwError "an obligation conjunction did not have two children"
  -- The clauses keep the context their path's preparation left.
  let prepared := (← get).prepared.contains step.goal
  for (goal, index) in subgoals.zipIdx do
    let selected := (owners.filter (·.1.head? == some (index != 0))).map fun (route, owner) =>
      (route.drop 1, owner)
    modify fun s => { s with activeOwners := selected }
    queue [goal] step.provenance step.clauses prepared
  let cost := (← IO.getNumHeartbeats) - start
  ctx.budgets.modify (·.charge (owners.map (·.2)) .leaf cost)
  charge "clause routing" start
  return true

/-- Under `verex.solverProfile`, the stages and steps of a run, whole or as far
as the target's budget let it go. -/
private def reportProfile (ctx : SolveContext) (state : SolveState) (stopped : Bool := false) :
    TacticM Unit := do
  let init := (← readThe Core.Context).initHeartbeats
  let mut lines := #[s!"profile {(← Lean.Elab.Term.getDeclName?).getD .anonymous}: \
    {((← IO.getNumHeartbeats) - ctx.startHeartbeats) / 1000}k, \
    before the solver {(ctx.startHeartbeats - init) / 1000}k, {state.leaves} leaves\
    {if stopped then ", stopped by the budget" else ""}; stages",
    state.continuations.report]
  if ctx.shared then
    -- A node's arrival classes are regions of one name.
    let nodes := state.regions.foldl (fun names region =>
      if names.contains region.node.name then names else names.push region.node.name) #[]
    lines := lines.push s!"  joins: nodes {nodes.size}, \
      edges {state.regions.foldl (fun count r => count + r.node.arrivals.size) 0}, \
      summaries {(state.regions.filter (·.node.summary?.isSome)).size}, \
      proofs {(state.regions.filter (·.node.certified)).size}"
    lines := lines.push s!"  join facts: invariants \
      {(state.regions.filter (!·.node.invariantProofs.isEmpty)).size}"
    let ledger ← ctx.budgets.get
    lines := lines.push s!"  automatic owners: {ledger.owners.size}, \
      prepared {(ledger.owners.filter (·.preparationSpent > 0)).size}, \
      exhausted {(ledger.owners.filter (·.remaining == some 0)).size}"
    lines := lines.push s!"  join pruning: checks {state.pruningChecks}, \
      pruned {state.prunedEdges}, debited owners {state.prepaidOwners.size}"
    let counts ← (← targetState).structureCounts.get
    lines := lines.push s!"  join structure: woken {counts.woken}, \
      alternatives {counts.alternatives}, walks {counts.walks}, \
      variant splits {counts.variantSplits}"
  let mut stages : Array (String × Nat × Nat) := #[]
  for (stage, cost) in state.stageCost do
    match stages.findIdx? (·.1 == stage) with
    | some index => stages := stages.modify index fun (name, count, sum) => (name, count + 1, sum + cost)
    | none => stages := stages.push (stage, 1, cost)
  for (stage, count, sum) in stages do
    lines := lines.push s!"  stage {stage} ×{count} {sum / 1000}k"
  let counts ← (← targetState).fixedPointCounts.get
  lines := lines.push s!"  fixed points: hits {counts[0]!}, misses {counts[1]!}, recorded {counts[2]!}; \
    dsimp hits {counts[3]!}, misses {counts[4]!}, recorded {counts[5]!}"
  let steps := (← (← targetState).stepProfile.get).toArray.qsort fun a b =>
    a.2.2.2.1 + a.2.2.2.2 > b.2.2.2.1 + b.2.2.2.2
  for (label, runs, failures, succeeding, failing) in steps do
    lines := lines.push s!"  step {label} ×{runs} ok {succeeding / 1000}k, \
      failed ×{failures} {failing / 1000}k"
  IO.println ("\n".intercalate lines.toList)

/-- The traversal: each pending goal through the first step that takes
it, down to its leaf. -/
private partial def traverseSteps (ctx : SolveContext) : SolveM Unit := do
  repeat
    -- A complete region resumes before any older pending goal, as the exit
    -- rule continues a path at its exit: its suffix's goals go on top.
    if let some index ← completeRegion? ctx then
      -- Every arrival is in: the node's fold context need no longer stay
      -- (`bindStep`), except where an enclosing node still collects.
      let regions := (← get).regions
      let ancestors := regionAncestors regions regions[index]!
      (← targetState).sharedPins.modify fun pinned =>
        regions[index]!.pins.foldl (init := pinned) fun pinned id =>
          if regions.any fun region => !region.node.sealed &&
              ancestors.contains region.node.name && region.pins.contains id then pinned
          else pinned.erase id
      pruneShared ctx index
      let region := (← get).regions[index]!
      match ← runOwned ctx region.owners (resumeShared ctx index) with
      | .done _ => pure ()
      | .failed failure => throw failure
      | .exhausted =>
          -- Rollback retained the original incoming holes. Report them
          -- under their exhausted owner; no summary certificate is cached.
          modify fun s => { s with activeOwners := region.owners }
          for arrival in region.node.arrivals do
            modify fun s => { s with activeSummary := arrival.summarized }
            decideLeaf ctx {
              goal := arrival.goal
              provenance := region.provenance
              clauses := region.clauses
              stageStart := ← IO.getNumHeartbeats }
          modify fun s => { s with regions := s.regions.modify index fun region =>
            { region with node := { region.node with sealed := true, finished := true } } }
      continue
    let some (goal, provenance, clauses, callSite) := (← get).pending.back? | do
      if ctx.shared then finishShared
      break
    modify fun s => if !ctx.shared then { s with
      pending := s.pending.pop
      activeDecisions := s.goalDecisions[goal]?.getD #[]
      activePoints := s.goalPoints[goal]?.getD #[] } else { s with
      pending := s.pending.pop
      activeDecisions := s.goalDecisions[goal]?.getD #[]
      activePoints := s.goalPoints[goal]?.getD #[]
      activeRegion := s.goalRegions[goal]?
      activeOwners := s.goalOwners[goal]?.getD #[]
      activeDebits := s.goalDebits[goal]?.getD #[]
      activeEntryOpened := s.openedEntries.contains goal
      activeOwed := s.goalOwed[goal]?
      activeSummary := s.goalSummaries.contains goal }
    if ← goal.isAssigned then continue
    let step : SolveStep := { goal, provenance, clauses, stageStart := ← IO.getNumHeartbeats, callSite }
    modify fun s => { s with stepCallSite := callSite, stepStart := step.stageStart,
                             stepStages := s.stageCost.size }
    if !ctx.shared then
      if let some leaf ← advance ctx step then decideLeaf ctx leaf
      continue
    let owners := (← get).activeOwners
    if owners.size > 1 && owners.all (!·.1.isEmpty) then
      match ← runOwned ctx owners (splitSharedCase step) with
      | .done true => continue
      | .done false | .exhausted | .failed _ => pure ()
    if ← partitionOwners ctx step then continue
    match ← runOwned ctx (← get).activeOwners (advance ctx step) with
    | .done none => pure ()
    | .done (some leaf) => decideLeaf ctx leaf
    | .exhausted => decideLeaf ctx step
    | .failed failure => throw failure

/-- The traversal, with what it spent reported where the target's budget stops
it. On `isMaxHeartbeat`, stores the current position in `targetState.budgetStop`
so the failure message can name it. -/
private def traverse (ctx : SolveContext) : SolveM Unit :=
  -- A goal the walk steps nests as deep as the program it runs, and the
  -- traversals over it -- normalization, the splits, a leaf's preparation --
  -- nest with it.  The elaborator's default recursion depth bounds the
  -- metaprogram's own stack, not the verification's work, which the target's
  -- heartbeats bound; left at the default, a walk over a deep enough program
  -- reports a recursion depth where its budget is what decides
  -- (`TypeReflection.evaluate` scopes it the same way).
  withTheReader Core.Context (fun context => { context with maxRecDepth := 1 <<< 20 }) <|
  tryCatchRuntimeEx (traverseSteps ctx) fun failure => do
    if failure.isMaxHeartbeat then
      -- The stage the budget interrupted never reaches its `charge`: the
      -- step's cost beyond the stages it charged is that stage's.
      let state ← get
      let charged := (state.stageCost.extract state.stepStages).foldl (· + ·.2) 0
      charge "interrupted" (state.stepStart + charged)
    if ctx.profile && failure.isMaxHeartbeat then
      reportProfile ctx (← get) true
      let now ← IO.getNumHeartbeats
      let steps := (← (← targetState).interruptedSteps.get).map fun (label, start) =>
        s!"{label} {(now - start) / 1000}k"
      IO.println s!"  stopped in: {if steps.isEmpty then "no timed step" else " > ".intercalate steps.toList}"
    if failure.isMaxHeartbeat then
      let state ← get
      let callEntry := state.stepCallSite.bind fun site => ctx.callSites.find? (·.1 == site)
      (← targetState).budgetStop.set (some {
        leavesDecided := state.leaves
        callee := callEntry.map (·.2.1)
        callNumber := callEntry.map (·.2.2.1)
        lastStage := state.stageCost.back?.map (·.1) })
    throw failure

initialize summaryTraversal.set fun ctx state => do
  let ((), final) ← (traverse ctx).run state
  return final

/-- Solve every goal: a loop by its invariant, a recursive-call leaf by the
loop hypothesis, a call by the callee's theorem, a structural node by
splitting, and a leaf by `solveLeaf`, or, with `handOver`, as the leaf of
an authored proof, named by what it owes (`leafName`). -/
partial def solveGoals (invariants : Array (Nat × Lean.Expr × Lean.Expr))
    (callees : Array (Lean.Expr × String × Lean.Expr)) (equations : Array Lean.Expr := #[])
    (handOver : Bool := false) (labeled : Bool := false)
    (unrolls : Array (Nat × Nat) := #[])
    (preconditions : Array (Lean.Expr × Lean.Expr) := #[])
    (inlinedHandles : Array (Lean.Expr × Lean.Expr) := #[])
    (names : Array (ObligationRange × String) := #[])
    (ghostWrites : Array (Nat × ObligationRange) := #[])
    (preparing? : Option (TSyntax ``Lean.Parser.Tactic.tacticSeq) := none)
    (specializations : Array Lean.Expr := #[])
    (callSites : Array (Nat × String × Nat × Nat) := #[])
    (sites : Array (Nat × String × Nat × String) := #[])
    (points : Array (ProgramPointKey × Syntax × TSyntax ``Lean.Parser.Tactic.tacticSeq) := #[]) :
    TacticM Unit := do
  (← targetState).assertedBounds.set {}
  (← targetState).leafReports.set (#[], #[])
  (← targetState).leafOrigins.set {}
  (← targetState).leafKeys.set {}
  (← targetState).leafDecisions.set {}
  (← targetState).decisionNames.set {}
  (← targetState).leafPaths.set {}
  (← targetState).clauseRanges.set names
  (← targetState).failedLeaves.set #[]
  (← targetState).exhaustedLeaves.set 0
  (← targetState).undecidedLeaves.set 0
  (← targetState).budgetStop.set none
  reportRuntimeMention
  (← targetState).runtimeMention.set .none
  (← targetState).nativeStatements.set {}
  (← targetState).assumedClauses.set #[]
  (← targetState).normalHypotheses.set {}
  (← targetState).normalForms.set {}
  (← targetState).readForms.set {}
  (← targetState).readMentions.set {}
  (← targetState).summaryParameters.set {}
  (← targetState).sharedPins.set {}
  (← targetState).keptFacts.set {}
  (← targetState).keptPremises.set {}
  (← targetState).fixedPoints.set {}
  (← targetState).dsimpResults.set {}
  (← targetState).fixedPointCounts.set #[0, 0, 0, 0, 0, 0]
  (← targetState).familyProofs.set {}
  (← targetState).contextRules.set {}
  (← targetState).stageNormalHypotheses.set {}
  (← targetState).familyNormalHypotheses.set {}
  (← targetState).projectionFreeTypes.set {}
  (← targetState).reducedBounds.set {}
  (← targetState).undecidedPremises.set {}
  -- Registered call theorems are solver evidence. Their expressions are
  -- retained in `SolveContext`, so incidental context normalization must
  -- not replace their local declarations with fresh proof variables.
  let calleeFacts := callees.foldl (fun found (_, _, proof) => collectFVars found proof) {}
  for goal in ← getGoals do
    goal.withContext do
      for fact in calleeFacts.fvarIds do
        let some declaration := (← getLCtx).find? fact | continue
        if ← isProp declaration.type then goal.setFVarKind fact .implDetail
  for goal in ← getGoals do recordModuleFactsNormal goal
  let debug := verex.solverDebug.get (← getOptions)
  if debug then
    IO.println s!"=== solver start {(← Lean.Elab.Term.getDeclName?).getD .anonymous}"
  setGoals (← rewriteEquations equations (← getGoals))
  -- The structural matchers read a target's head: drop the metadata an
  -- introduction may leave around it.
  setGoals (← (← getGoals).mapM fun goal => do
    let target ← instantiateMVars (← goal.getType)
    if target.isMData then goal.replaceTargetDefEq target.consumeMData else pure goal)
  -- A step proof's postcondition reads the returned value as `result`.
  if handOver then
    setGoals (← (← getGoals).mapM fun goal => (enterResult goal : MetaM MVarId))
  let startHeartbeats ← IO.getNumHeartbeats
  let normalization ← normalization
  let pending := (← (← getGoals).toArray.mapM hideModelBinders).map fun g => (g, none, #[], none)
  let base? ← pending[0]?.mapM fun (goal, _) => goal.withContext do
    return (← getLCtx, ← getLocalInstances)
  let ctx : SolveContext := {
    invariants := invariants
    callees := callees
    equations := equations
    specializations := specializations
    handOver := handOver
    preparing? := if handOver then preparing? else none
    preparesPaths := !handOver
    points := if handOver then points else #[]
    labeled := labeled
    unrolls := unrolls
    preconditions := preconditions
    inlinedHandles := inlinedHandles
    names := names
    callSites := callSites
    sites := sites
    ghostWrites := ghostWrites
    debug := debug
    profile := verex.solverProfile.get (← getOptions)
    shared := verex.sharedContinuations.get (← getOptions) && !handOver
    budgets := ← IO.mkRef {}
    normalization := normalization
    startHeartbeats := startHeartbeats
    base? := base? }
  let previousPreparation ← (← targetState).preparationProofs.get
  let previousOwners ← (← targetState).preparationOwners.get
  let messagesBefore := (← Core.getMessageLog).unreported.size
  let ((), final) ← try
      let active ← if ctx.shared then some <$> IO.mkRef {} else pure none
      (← targetState).preparationProofs.set active
      (← targetState).preparationOwners.set #[]
      (traverse ctx).run { pending := pending, shared := ctx.shared }
    finally
      (← targetState).preparationProofs.set previousPreparation
      (← targetState).preparationOwners.set previousOwners
      unless handOver do orderAutomaticReports messagesBefore
  -- A hook at a point no path reaches is an error of the proof.
  for ((point, hook, _), index) in ctx.points.zipIdx do
    unless final.pointsUsed.contains index do
      logErrorAt hook m!"no path reaches the point `{point.text}`"
  let stageCost := final.stageCost
  let handed := final.handed
  let leaves := final.leaves
  if debug then
    let mut totals : Array (String × Nat × Nat) := #[]
    for (stage, cost) in stageCost do
      match totals.findIdx? (·.1 == stage) with
      | some index => totals := totals.modify index fun (name, count, sum) => (name, count + 1, sum + cost)
      | none => totals := totals.push (stage, 1, cost)
    let mut report := m!""
    for (stage, count, sum) in totals do
      report := report ++ m!" {stage} ×{count} {sum / 1000}k;"
    logInfo m!"solver {(← Lean.Elab.Term.getDeclName?).getD .anonymous}: \
      {((← IO.getNumHeartbeats) - startHeartbeats) / 1000}k heartbeats, \
      {leaves} leaves, {handed.size} handed over;{report}"
    IO.println "=== solver end"
  if ctx.profile then reportProfile ctx final
  -- The leaves owing one clause, as one goal over their paths
  -- (`combineLeaves`), which `verex_paths` takes apart again.
  let mut groups : Array (ObligationKey × Array MVarId) := #[]
  for (goal, key) in handed do
    match groups.findIdx? (·.1 == key) with
    | some index => groups := groups.modify index fun (k, gs) => (k, gs.push goal)
    | none => groups := groups.push (key, #[goal])
  let mut goals := #[]
  for (key, members) in groups do
    let tag := key.tag
    if members.size == 1 then
      members[0]!.setTag tag
      (← targetState).leafKeys.modify (·.insert tag key)
      goals := goals.push members[0]!
      continue
    match ← (if verex.combine.get (← getOptions) then combineLeaves tag members
        else pure none) with
    | some goal =>
        (← targetState).leafKeys.modify (·.insert tag key)
        goals := goals.push goal
    | none =>
        for member in members, index in [0:members.size] do
          let numbered := tag.appendIndexAfter (index + 1)
          member.setTag numbered
          (← targetState).leafKeys.modify (·.insert numbered key)
          goals := goals.push member
  setGoals goals.toList

declare_syntax_cat solveFlag (behavior := symbol)
/-- Keep the program points: the contract binds a state label. -/
syntax (name := labeledFlag) &"labeled" : solveFlag
/-- A clause of the function, by its source range: the name of the leaves
that owe it. -/
declare_syntax_cat obligationName
syntax "(" str ", " str ", " num ", " num ")" : obligationName
/-- A ghost write's site, by its update's source range. -/
declare_syntax_cat ghostWriteSite
syntax "(" num ", " str ", " num ", " num ")" : ghostWriteSite
/-- A call of the target: its site, the callee's name, and the call's number
among the calls of the callee and among all calls. -/
declare_syntax_cat callSite
syntax "(" num ", " str ", " num ", " num ")" : callSite
/-- A precondition's number among its function's preconditions, by its
source range. -/
declare_syntax_cat preconditionNumber
syntax "(" num ", " str ", " num ", " num ")" : preconditionNumber
/-- A site of the target a path's decision names: the site, its kind, its
number among the target's sites of the kind, and a label. -/
declare_syntax_cat sourceSite
syntax "(" num ", " str ", " num ", " str ")" : sourceSite
/-- An invariant clause by its key and its source range. -/
declare_syntax_cat invariantClause
syntax "(" str ", " str ", " num ", " num ")" : invariantClause

/-- The arguments of a function's obligations: loops and in-body assertions
by the predicates given as `(site, function, predicate)` triples, calls by
the callees' theorems; the `using` equations rewrite wherever a body brings
their terms in. -/
syntax obligationArguments := solveFlag* " [" term,* "]"
  (" with" " [" term,* "]")? (" using" " [" term,* "]")? (" specializing" " [" term,* "]")?
  (" unrolling" " [" term,* "]")? (" requiring" " [" term,* "]")?
  (" writing" " [" ghostWriteSite,* "]")? (" naming" " [" obligationName,* "]")?
  (" calling" " [" callSite,* "]")? (" numbering" " [" preconditionNumber,* "]")?
  (" siting" " [" sourceSite,* "]")? (" keying" " [" invariantClause,* "]")?

/-- Where a selector says a path aborts (`AbortSite`): `call 1 of g`, `call
2` among all calls, `abort 1`, `op 3`, `invoke 1`. -/
declare_syntax_cat verexSite (behavior := symbol)
syntax (name := siteCall) &"call" num (&"of" ident)? : verexSite
syntax (name := siteAbort) &"abort" num : verexSite
syntax (name := siteOp) &"op" num : verexSite
syntax (name := siteInvoke) &"invoke" num : verexSite

/-- A program point of a hook (`ProgramPointKey`): the function's entry, a
loop's entry or the start of its iteration. -/
declare_syntax_cat verexPoint (behavior := symbol)
syntax (name := pointEntry) &"entry" : verexPoint
syntax (name := pointExit) &"exit" : verexPoint
syntax (name := pointLoop) &"loop" num (&"entry" <|> &"step") : verexPoint

/-- Tactics run where the walk reaches a point, `at` one or `after` a site,
once for every path through it. -/
syntax verexPointHook := ppLine ((&"at" verexPoint) <|> (&"after" verexSite)) " => " tacticSeq

/-- Split a function's goal into its leaves, each named by what it owes, and
leave them to the proof (`verex_solve`, `verex_decide`, …). In a `verify f
by …` item, plain `verex_obligations` is the obligations of `f`. Hooks at
points run on the paths through them; `preparing` runs on each path's
end. -/
syntax (name := verexObligations) "verex_obligations" (obligationArguments)?
  withPosition((colGe verexPointHook)*) (&"preparing" tacticSeq)? : tactic

/-- The point a hook's syntax names, a call without its callee counted among
all calls. -/
def pointPattern (stx : Syntax) : TacticM ProgramPointKey :=
  match stx with
  | `(verexPointHook| at entry => $_) => pure .entry
  | `(verexPointHook| at exit => $_) =>
      throwErrorAt stx "`at exit` is the end of every path: `preparing` runs there"
  | `(verexPointHook| at loop $k:num entry => $_) => pure (.loopEntry k.getNat)
  | `(verexPointHook| at loop $k:num step => $_) => pure (.loopStep k.getNat)
  | `(verexPointHook| after call $k:num of $callee:ident => $_) =>
      pure (.after (.call k.getNat callee.getId.toString 0))
  | `(verexPointHook| after call $k:num => $_) => pure (.after (.call k.getNat "" 0))
  | `(verexPointHook| after op $k:num => $_) => pure (.after (.op k.getNat))
  | `(verexPointHook| after invoke $k:num => $_) => pure (.after (.invoke k.getNat))
  | `(verexPointHook| after abort $_:num => $_) =>
      throwErrorAt stx "an abort has no continuation: `after abort` names no point"
  | _ => throwErrorAt stx "unexpected point"

/-- A hook's point, its syntax and its tactics. -/
def pointHook (stx : TSyntax ``verexPointHook) :
    TacticM (ProgramPointKey × Syntax × TSyntax ``Lean.Parser.Tactic.tacticSeq) := do
  let point ← pointPattern stx
  return (point, stx, ⟨stx.raw[stx.raw.getNumArgs - 1]⟩)
/-- Split a function's goal into its leaves and decide each as it is
reached, or report the clause it owes: `verex_obligations` followed by
`verex_solve` on every leaf. -/
syntax "verex_solve_goals" (obligationArguments)? : tactic

/-- What a function's obligation arguments state, elaborated. -/
private structure ObligationData where
  invariants : Array (Nat × Lean.Expr × Lean.Expr)
  callees : Array (Lean.Expr × String × Lean.Expr)
  equations : Array Lean.Expr
  specializations : Array Lean.Expr
  labeled : Bool
  unrolls : Array (Nat × Nat)
  preconditions : Array (Lean.Expr × Lean.Expr)
  inlinedHandles : Array (Lean.Expr × Lean.Expr)
  names : Array (ObligationRange × String)
  ghostWrites : Array (Nat × ObligationRange)
  callSites : Array (Nat × String × Nat × Nat)
  sites : Array (Nat × String × Nat × String)

/-- Elaborate a function's obligation arguments; the precondition numbers
and invariant keys go to the target's state. -/
private def elabObligationArguments (arguments : TSyntax ``obligationArguments) :
    TacticM ObligationData := do
  match arguments with
  | `(obligationArguments| $flags* [$loops:term,*] $[with [$calls:term,*]]?
      $[using [$equations:term,*]]? $[specializing [$specialized:term,*]]?
      $[unrolling [$bounds:term,*]]?
      $[requiring [$required:term,*]]? $[writing [$writes:ghostWriteSite,*]]?
      $[naming [$named:obligationName,*]]? $[calling [$calls':callSite,*]]?
      $[numbering [$numbered:preconditionNumber,*]]? $[siting [$sited:sourceSite,*]]?
      $[keying [$keyed:invariantClause,*]]?) => do
      let loops := some loops
      if verex.solverDebug.get (← getOptions) then
        logInfo m!"normalized verification condition:\n{← getMainGoal}"
      let mut invariants : Array (Nat × Lean.Expr × Lean.Expr) := #[]
      for loop in (loops.map (·.getElems)).getD #[] do
        let triple ← Lean.Elab.Tactic.elabTerm loop none
        let triple ← whnf (← instantiateMVars triple)
        let rest ← whnf (triple.getArg! 3)
        unless triple.isAppOfArity ``Prod.mk 4 && rest.isAppOfArity ``Prod.mk 4 do
          throwError m!"a loop invariant must be a `(site, function, invariant)` triple, \
            not {triple}"
        let some site ← siteOf? (triple.getArg! 2)
          | throwError m!"a loop site must be a numeral, not {triple.getArg! 2}"
        invariants := invariants.push (site, rest.getArg! 2, rest.getArg! 3)
      let mut callees : Array (Lean.Expr × String × Lean.Expr) := #[]
      let mut inlinedHandles : Array (Lean.Expr × Lean.Expr) := #[]
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
        -- An inlined callee's meaning names its function beside its refinement.
        let proved := inner.getArg! 3
        if proved.isAppOfArity ``PProd.mk 4 then
          inlinedHandles := inlinedHandles.push (pair.getArg! 2, proved.getArg! 2)
          callees := callees.push (pair.getArg! 2, name, proved.getArg! 3)
        else callees := callees.push (pair.getArg! 2, name, proved)
      let equations ← ((equations.map (·.getElems)).getD #[]).mapM fun equation => do
        instantiateMVars (← Lean.Elab.Tactic.elabTerm equation none)
      let specializations ← ((specialized.map (·.getElems)).getD #[]).mapM fun equivalence => do
        instantiateMVars (← Lean.Elab.Tactic.elabTerm equivalence none)
      let unrolls ← ((bounds.map (·.getElems)).getD #[]).mapM fun bound => do
        let pairType ← mkAppM ``Prod #[mkConst ``Nat, mkConst ``Nat]
        let pair ← Lean.Elab.Tactic.elabTerm bound (some pairType)
        let pair ← whnf (← instantiateMVars pair)
        unless pair.isAppOfArity ``Prod.mk 4 do throwError "expected (loop site, unroll bound)"
        let some site ← siteOf? (pair.getArg! 2)
          | throwError "unroll site must be a numeral"
        let some bound ← (evalNat (pair.getArg! 3)).run
          | throwError "unroll bound must be a numeral"
        pure (site, bound)
      let mut preconditions : Array (Lean.Expr × Lean.Expr) := #[]
      for entry in (required.map (·.getElems)).getD #[] do
        let pair ← whnf (← instantiateMVars (← Lean.Elab.Tactic.elabTerm entry none))
        unless pair.isAppOfArity ``Prod.mk 4 do
          throwError m!"a precondition must be a `(function, predicate)` pair, not {pair}"
        preconditions := preconditions.push (pair.getArg! 2, pair.getArg! 3)
      let names ← ((named.map (·.getElems)).getD #[]).mapM fun
        | `(obligationName| ($name:str, $file:str, $start:num, $stop:num)) =>
            pure ({ file := file.getString, startByte := start.getNat, endByte := stop.getNat },
              name.getString)
        | _ => throwError "a clause name must be `(name, file, start, end)`"
      let ghostWrites ← ((writes.map (·.getElems)).getD #[]).mapM fun
        | `(ghostWriteSite| ($site:num, $file:str, $start:num, $stop:num)) =>
            pure (site.getNat,
              ({ file := file.getString, startByte := start.getNat, endByte := stop.getNat } :
                ObligationRange))
        | _ => throwError "a ghost write site must be `(site, file, start, end)`"
      let callSites ← ((calls'.map (·.getElems)).getD #[]).mapM fun
        | `(callSite| ($site:num, $callee:str, $number:num, $overall:num)) =>
            pure (site.getNat, callee.getString, number.getNat, overall.getNat)
        | _ => throwError "a call site must be `(site, callee, number, overall number)`"
      let preconditionNumbers ← ((numbered.map (·.getElems)).getD #[]).mapM fun
        | `(preconditionNumber| ($number:num, $file:str, $start:num, $stop:num)) =>
            pure ({ file := file.getString, startByte := start.getNat, endByte := stop.getNat },
              number.getNat)
        | _ => throwError "a precondition number must be `(number, file, start, end)`"
      (← targetState).preconditionNumbers.set preconditionNumbers
      let invariantKeys ← ((keyed.map (·.getElems)).getD #[]).mapM fun
        | `(invariantClause| ($kind:str, $file:str, $start:num, $stop:num)) =>
            pure ({ file := file.getString, startByte := start.getNat, endByte := stop.getNat },
              kind.getString)
        | _ => throwError "an invariant clause must be `(key, file, start, end)`"
      (← targetState).invariantKeys.set invariantKeys
      let sites ← ((sited.map (·.getElems)).getD #[]).mapM fun
        | `(sourceSite| ($site:num, $kind:str, $number:num, $label:str)) =>
            pure (site.getNat, kind.getString, number.getNat, label.getString)
        | _ => throwError "a site must be `(site, kind, number, label)`"
      let labeled := flags.any (·.raw.isOfKind ``labeledFlag)
      return {
        invariants := invariants
        callees := callees
        equations := equations
        specializations := specializations
        labeled := labeled
        unrolls := unrolls
        preconditions := preconditions
        inlinedHandles := inlinedHandles
        names := names
        ghostWrites := ghostWrites
        callSites := callSites
        sites := sites }
  | _ => throwError "unexpected obligation arguments"

/-- Generate the obligations, handing them over or solving each. -/
private def obligations (arguments : Option (TSyntax ``obligationArguments)) (handOver : Bool)
    (preparing? : Option (TSyntax ``Lean.Parser.Tactic.tacticSeq) := none)
    (points : Array (ProgramPointKey × Syntax × TSyntax ``Lean.Parser.Tactic.tacticSeq) := #[]) :
    TacticM Unit := do
  match arguments with
  | none =>
      if handOver then
        throwError "`verex_obligations` names the obligations of the function a `verify` item proves"
      solveGoals #[] #[]
  | some arguments =>
      let data ← elabObligationArguments arguments
      solveGoals data.invariants data.callees data.equations handOver data.labeled data.unrolls
        data.preconditions data.inlinedHandles data.names data.ghostWrites preparing?
        data.specializations data.callSites data.sites points

/-- Derive the summary of the compiled callee `callee` names (a callee pair,
`PProd.mk handle (PProd.mk name certificate)`) and declare it as the theorem
`name`, before any proof of the module runs: the main goal states the
call's shape in the callee's own telescope,
`∀ args ensures aborts initial, wp (call args) ensures aborts initial`, and
the walk of the callee's body runs in that context (`summaryWalk`) with the
`arguments` of the callee's own obligations, within
`verex.summaryHeartbeats`, under a target state of its own. The number of
conjuncts, or why no summary is derived; `derivedSummaries` records
either. A theorem the kernel rejects is an error. -/
def deriveModuleSummary (name : Name) (arguments : TSyntax ``obligationArguments)
    (callee : Lean.Expr) : TacticM (Except String Nat) := do
  let pair ← whnf (← instantiateMVars callee)
  unless pair.isAppOfArity ``PProd.mk 4 && (pair.getArg! 3).isAppOfArity ``PProd.mk 4 do
    throwError m!"a callee must be `PProd.mk handle (PProd.mk name theorem)`, not {pair}"
  let handle := pair.getArg! 2
  let inner := pair.getArg! 3
  let .lit (.strVal calleeName) := inner.getArg! 2
    | throwError m!"a callee name must be a string literal, not {inner.getArg! 2}"
  let theoremProof := inner.getArg! 3
  let proofType ← instantiateMVars (← inferType theoremProof)
  -- A model's callee names its compiled body itself.
  let some compiled := (do
      if let .const compiled _ := theoremProof.consumeMData then
        unless proofType.isAppOfArity ``Eq 3 do return compiled
      guard (proofType.isAppOfArity ``Eq 3)
      let .const compiled _ := (proofType.getArg! 2).appArg! | none
      pure compiled)
    | throwError m!"a summarized callee's certificate must name its compiled body, not {proofType}"
  let refuse (reason : String) : TacticM (Except String Nat) := do
    recordDerivedSummary compiled none
    return .error reason
  unless ← summaryForm compiled do return ← refuse "its form admits none"
  let goal ← getMainGoal
  -- The call's shape, read off the statement.
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  let shape? ← goal.withContext <| forallBoundedTelescope target (some 4) fun xs body => do
    -- A model's statement takes the callee's precondition first.
    let (requires?, body) ← match body with
      | .forallE _ domain inner _ =>
          if inner.hasLooseBVars then pure (none, body)
          else pure (some (← mkLambdaFVars xs domain), inner)
      | _ => pure (none, body)
    unless xs.size == 4 && body.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := (body.getArg! 3).consumeMData
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.propheticMeaning 9 ||
        action.isAppOfArity ``VerexIR.Proofs.Denote.Function.denoteWith 7 do return none
    let types ← xs.mapM fun x => do instantiateMVars (← inferType x)
    let implicit := body.getAppArgs.extract 0 3
    let meaning := action.appFn!
    let bound := xs.map (·.fvarId!)
    if (types ++ implicit ++ #[meaning]).any (·.hasAnyFVar bound.contains) then return none
    return some (types, body.getAppFn, implicit, meaning, requires?)
  let some (types, wpHead, wpImplicit, meaning, requires?) := shape?
    | throwError m!"a summary's statement must be the shape of a call, not {target}"
  let data ← elabObligationArguments arguments
  -- The walk's own target state, named by the theorem for its hidden
  -- theorems; the command's is restored after it.
  let env ← getEnv
  let key := env.asyncPrefix?.getD .anonymous
  let previous := targetStateExt.getState env
  let fresh ← TargetState.new
  fresh.proofName.set name
  modifyEnv (targetStateExt.setState · (some (key, fresh)))
  try
    let result ← goal.withContext do
      (← targetState).moduleFacts.set ((← goal.getNondepPropHyps).foldl (·.insert ·) {})
      recordModuleFactsNormal goal
      let ctx : SolveContext := {
        invariants := data.invariants
        callees := data.callees
        equations := data.equations
        specializations := data.specializations
        handOver := false
        preparesPaths := false
        labeled := data.labeled
        unrolls := data.unrolls
        preconditions := data.preconditions
        inlinedHandles := data.inlinedHandles
        names := data.names
        callSites := data.callSites
        sites := data.sites
        ghostWrites := data.ghostWrites
        debug := verex.solverDebug.get (← getOptions)
        profile := false
        -- A summary is collected from open leaves over the callee's
        -- parameters; a sealed class leaves its leaves over abstract entry
        -- rows and needs them closed, so the derivation is a tree walk.
        shared := false
        budgets := ← IO.mkRef {}
        normalization := ← normalization
        startHeartbeats := ← IO.getNumHeartbeats
        base? := some (← getLCtx, ← getLocalInstances)
        summarizing := #[compiled] }
      let options ← getOptions
      let derivationBudget := if proofType.isAppOfArity ``Eq 3 then verex.summaryHeartbeats.get options
        else 7 * verex.summaryHeartbeats.get options
      tryCatchRuntimeEx
        -- A model's callee stands for walks of its body in every target.
        (withBudget (derivationBudget * 1000)
          (summaryWalk ctx compiled handle calleeName theoremProof proofType types wpHead
            wpImplicit meaning requires?))
        fun failure => do
          if failure.isMaxHeartbeat then return .error "the walk exceeded its heartbeat budget"
          return .error s!"the walk failed: {← failure.toMessageData.toString}"
    match result with
    | .error reason => refuse reason
    | .ok (type, value, conjuncts) =>
        -- Declared once, in the command's environment, for every proof.
        declareHiddenTheorems value
        withOptions (Elab.async.set · false) do
          addDecl (.thmDecl { name, levelParams := [], type, value })
        let summary : DerivedSummary :=
          { name, conjuncts, takesRequires := requires?.isSome }
        recordDerivedSummary compiled (some summary)
        return .ok conjuncts
  finally
    modifyEnv (targetStateExt.setState · previous)

elab_rules : tactic
  | `(tactic| verex_obligations $[$arguments]? $hooks:verexPointHook* $[preparing $preparing?]?) => do
      obligations arguments true preparing? (← hooks.mapM pointHook)
  | `(tactic| verex_solve_goals $[$arguments]?) => obligations arguments false

/-- An authored proof's goal with its obligation that no abort condition holds
marked (`NoAbort`), which the walk takes off where it owes it: the contract stays
as it is, and an automatic goal has no marker. -/
elab "verex_mark_returns" : tactic => focusGoal do
  let goal ← getMainGoal
  let target ← instantiateMVars (← goal.getType)
  let marked := target.replace fun e =>
    if e.isAppOfArity ``Not 1 && e.appArg!.isAppOf ``VerexIR.Proofs.Contract.mustAbort then
      some (mkApp (mkConst ``NoAbort) e)
    else none
  replaceMainGoal [← goal.replaceTargetDefEq marked]

/-! ## Tactics of authored proofs

`verify f by …` proves `f`'s goal; `verex_obligations` splits it into the
leaves it owes, which these or any other tactics decide. -/

/-- The paths of a clause's goal, one goal each, tagged `clause_k`: the
conjunction split, and each conjunct's locals and hypotheses introduced as
its path had them. A goal that is one leaf stays as it is. -/
def splitPaths (goal : MVarId) : MetaM (Array MVarId) := do
  let some paths := (← (← targetState).leafPaths.get)[goal]? | return #[goal]
  let tag ← goal.getTag
  let key := ((← (← targetState).leafKeys.get)[tag]?).getD { kind := toString tag }
  let mut goals := #[]
  let mut current := goal
  for (name, provenance, clauses, details, decisions) in paths, index in [0:paths.size] do
    -- The conjunction taken apart by its statement: unification would
    -- reduce a path's `let` binders away.
    let conjunct ← if index + 1 < paths.size then do
        let type ← current.withContext (instantiateMVars (← current.getType))
        let some (left, right) := type.and? | throwError "a clause's paths are a conjunction"
        let leftGoal ← current.withContext (mkFreshExprSyntheticOpaqueMVar left)
        let rightGoal ← current.withContext (mkFreshExprSyntheticOpaqueMVar right)
        current.assign (mkApp4 (mkConst ``And.intro) left right leftGoal rightGoal)
        current := rightGoal.mvarId!
        pure leftGoal.mvarId!
      else pure current
    let (fvars, next) ← conjunct.introNP details.size
    let next ← markDetails next (fvars.zip details)
    next.setTag name
    (← targetState).leafOrigins.modify (·.insert next (provenance, clauses))
    (← targetState).leafDecisions.modify (·.insert next decisions)
    (← targetState).leafKeys.modify (·.insert name key)
    goals := goals.push next
  return goals

@[inherit_doc splitPaths]
elab "verex_paths" : tactic => focusGoal do
  replaceMainGoal (← splitPaths (← getMainGoal)).toList

/-- A goal's local context, declaration by declaration: goals of one path
share it. -/
def goalContextKey (goal : MVarId) : MetaM (Array (FVarId × Lean.Expr × Option Lean.Expr)) :=
  goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      return found.push (decl.fvarId, ← instantiateMVars decl.type,
        ← decl.value?.mapM instantiateMVars)

/-- Prove selected clauses with the same context as one conjunction. Their
preparation, case analysis, and intermediate facts can then be shared. Every
original goal is assigned its own projection of the conjunction. -/
elab "verex_group " name:ident " := " clauses:ident,+ : tactic => do
  evalTactic (← `(tactic| all_goals verex_paths))
  let requested := clauses.getElems.map (·.getId.toString)
  let mut selected := #[]
  let mut others := #[]
  let known ← (← targetState).leafKeys.get
  let mut found := #[]
  for goal in ← getGoals do
    let tag ← goal.getTag
    let clause := ((known[tag]?).map (·.tag.toString)).getD tag.toString
    if requested.contains clause then
      selected := selected.push goal
      found := found.push clause
    else others := others.push goal
  for clause in requested do
    unless found.contains clause do throwError "no obligation named `{clause}`"
  let first := selected[0]!
  let context ← goalContextKey first
  for goal in selected do
    unless (← goalContextKey goal) == context do
      throwError "the selected obligations have different contexts; group clauses on the same path"
  let statements ← selected.mapM fun goal => goal.withContext do
    instantiateMVars (← goal.getType)
  let mut statement := statements.back!
  for part in statements.pop.reverse do statement := mkAnd part statement
  let decl ← first.getDecl
  let combined ← mkFreshExprMVarAt decl.lctx decl.localInstances statement .syntheticOpaque name.getId
  for goal in selected, index in [0:selected.size] do
    let mut part := combined
    for _ in [0:index] do part := .proj ``And 1 part
    if index + 1 < selected.size then part := .proj ``And 0 part
    goal.assign part
  (← targetState).leafKeys.modify (·.insert name.getId { kind := name.getId.toString })
  setGoals (combined.mvarId! :: others.toList)

/-- Group obligations whose contexts agree up to local variable names. Prove
one conjunction per context, sharing preparation and case analysis. Each original
goal receives its projection, instantiated at its own locals. This is opt-in for
authored proofs with a common argument; split conjunctions after preparation.
Clause-specific routing with `verex_on` must precede this grouping. -/
elab "verex_group_contexts" : tactic => do
  evalTactic (← `(tactic| all_goals verex_paths))
  let contextKey (goal : MVarId) : MetaM (Array (Lean.Expr × Option Lean.Expr × BinderInfo)) := goal.withContext do
    let mut vars := #[]
    let mut key := #[]
    for decl in ← getLCtx do
      key := key.push ((← instantiateMVars decl.type).abstract vars,
        ← decl.value?.mapM fun value => return (← instantiateMVars value).abstract vars,
        decl.binderInfo)
      vars := vars.push decl.toExpr
    return key
  let mut groups : Array (Array (Lean.Expr × Option Lean.Expr × BinderInfo) × Array MVarId) := #[]
  for goal in ← getGoals do
    let key ← contextKey goal
    match groups.findIdx? (·.1 == key) with
    | some i => groups := groups.modify i fun (k, gs) => (k, gs.push goal)
    | none => groups := groups.push (key, #[goal])
  let mut goals := #[]
  for (_, selected) in groups do
    if selected.size == 1 then
      goals := goals.push selected[0]!
      continue
    let first := selected[0]!
    let decls ← first.withContext do (← getLCtx).foldlM (init := #[]) fun acc decl => pure (acc.push decl)
    let firstVars := decls.map (·.toExpr)
    let vars ← selected.mapM fun goal => goal.withContext do
      (← getLCtx).foldlM (init := #[]) fun acc decl => pure (acc.push decl.toExpr)
    let statements ← selected.mapIdxM fun index goal => goal.withContext do
      return (← instantiateMVars (← goal.getType)).replaceFVars vars[index]! firstVars
    let mut statement := statements.back!
    for part in statements.pop.reverse do statement := mkAnd part statement
    -- Close over the context so each original goal can instantiate its own locals.
    statement := statement.abstract firstVars
    for decl in decls.reverse, i in [0:decls.size] do
      let prior := firstVars.extract 0 (decls.size - i - 1)
      let type := (← instantiateMVars decl.type).abstract prior
      let value? ← decl.value?.mapM instantiateMVars
      statement := match value? with
      | some value => Lean.Expr.letE decl.userName type (value.abstract prior) statement false
      | none => Lean.Expr.forallE decl.userName type statement decl.binderInfo
    let combined ← mkFreshExprMVarAt {} #[] statement .syntheticOpaque (← first.getTag)
    for goal in selected, index in [0:selected.size] do
      let arguments := vars[index]!.zip decls |>.filterMap fun (var, decl) => if decl.isLet then none else some var
      let mut part := mkAppN combined arguments
      for _ in [0:index] do part := .proj ``And 1 part
      if index + 1 < selected.size then part := .proj ``And 0 part
      goal.assign part
    -- Restore metadata used by preparation and by Lean's auxiliary declarations.
    let auxiliaryNames := (← first.getDecl).lctx.auxDeclToFullName
    let (introduced, next) ← combined.mvarId!.introNP decls.size
    let next ← next.withContext do
      let mut lctx ← getLCtx
      for id in introduced, decl in decls do
        lctx := lctx.setKind id decl.kind
        if let some fullName := auxiliaryNames.get? decl.fvarId then
          lctx := { lctx with auxDeclToFullName := lctx.auxDeclToFullName.insert id fullName }
      let fresh ← mkFreshExprMVarAt lctx (← getLocalInstances) (← next.getType) .syntheticOpaque (← first.getTag)
      next.assign fresh
      return fresh.mvarId!
    goals := goals.push next
  setGoals goals.toList

/-- The typed maps a term reads, with the source owner of each table or
projected table field. -/
private def typedMaps (e : Lean.Expr) (found : Array (Lean.Expr × FVarId) := #[]) :
    Array (Lean.Expr × FVarId) :=
  let rec walk (e : Lean.Expr) : StateM (Std.HashSet Lean.Expr × Array (Lean.Expr × FVarId)) Unit := do
    if (← get).1.contains e then return
    modify fun (seen, found) => (seen.insert e, found)
    if e.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 && !e.hasLooseBVars then
      if let some owner := tableViewOwner? e.appArg! then
        unless (← get).2.any (·.1 == e) do
          modify fun (seen, found) => (seen, found.push (e, owner))
    match e with
    | .app f a => walk f; walk a
    | .lam _ d b _ | .forallE _ d b _ => walk d; walk b
    | .letE _ ty v b _ => walk ty; walk v; walk b
    | .mdata _ b | .proj _ _ b => walk b
    | _ => pure ()
  ((walk e).run ({}, found)).2.2

/-- Name the typed maps of a prepared leaf over source-named table carriers
or projected fields. Bare carriers are the named maps written back
(`writeBackCarrier`); projected tables retain only their observed read
bridge, without reconstructing their owners. -/
def nameTypedMaps : TacticM Unit := focusGoal do
  -- Observe keys-only validity at the typed views before naming and
  -- write-back replace their encodings with source map names.
  let goal ← getMainGoal
  let validities ← goal.withContext do
    let mut types := #[]
    let mut valids := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let type ← instantiateMVars decl.type
      types := types.push type
      if type.isAppOfArity ``VerexIR.SpecMap.Valid 4 then
        valids := valids.push (type.getArg! 3, decl.toExpr)
    if valids.isEmpty then return #[]
    let mut observed := closedApplications (← instantiateMVars (← goal.getType))
      ``VerexIR.SpecMap.ofEntries 7
    for type in types do
      for map in closedApplications type ``VerexIR.SpecMap.ofEntries 7 do
        unless observed.contains map do observed := observed.push map
    observedMapValidities observed valids
  let mut observedGoal := goal
  for (_, proof) in validities do
    let type ← observedGoal.withContext (inferType proof)
    let (_, next) ← (← observedGoal.assert `valid_read type proof).intro1P
    observedGoal := next
  replaceMainGoal [observedGoal]
  let isFact (ty : Lean.Expr) : MetaM Bool := do
    try isProp ty catch _ => pure false
  -- Substituting equal carriers can bring two previously named views
  -- together. Reuse one witness, retaining a summary parameter if present.
  for _ in [:16] do
    let goal ← getMainGoal
    let duplicate? ← goal.withContext do
      let mut reads : Array (Lean.Expr × Lean.Expr × FVarId) := #[]
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | continue
        unless (typedMap? lhs).isSome && rhs.isFVar do continue
        for (view, named, witness) in reads do
          unless lhs.appArg! == view.appArg! && rhs != named do continue
          if (← isSummaryParameter named.fvarId!) && (← isSummaryParameter rhs.fvarId!) then continue
          if ← withDefault (isDefEq lhs view) then
            return some (witness, decl.fvarId)
        reads := reads.push (lhs, rhs, decl.fvarId)
      return none
    let some (first, later) := duplicate? | break
    let (equation, next) ← goal.withContext do
      let proof ← mkEqTrans (← mkEqSymm (mkFVar first)) (mkFVar later)
      (← goal.assert `map_alias (← inferType proof) proof).intro1P
    let next ← substEquation next equation
    replaceMainGoal [next]
  -- A map named earlier reads back as its name where a specification
  -- function unfolded since spells its write-back again (`m_read`).
  let reads ← (← getMainGoal).withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | return found
      unless lhs.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 && rhs.isFVar do return found
      let entries := lhs.appArg!.consumeMData
      let written := if entries.isAppOfArity ``NTy.forget 3 then
          (entries.getArg! 2).consumeMData else entries
      return if (tableViewOwner? entries).isSome ||
          written.isAppOfArity ``VerexIR.SpecMap.writeBack 6 then
        found.push decl.fvarId else found
  unless reads.isEmpty do
    let goal ← getMainGoal
    let targets ← goal.withContext do
      (← getLCtx).foldlM (init := #[]) fun found decl => do
        if decl.isImplementationDetail || reads.contains decl.fvarId then return found
        return if ← isFact (← instantiateMVars decl.type) then found.push decl.fvarId else found
    if let some goal ← simpAt goal (← `(tactic| simp only [])) reads targets true then
      replaceMainGoal [goal]
    else replaceMainGoal []
  -- A key or value of a typed map cut back to its integer carrier is the
  -- key or value, by the carrier's form the naming abstracts: substitution
  -- after normalization leaves such cuts.
  unless (← getGoals).isEmpty do
    let goal ← getMainGoal
    let (targets, cut) ← goal.withContext do
      let mut targets := #[]
      let mut cut := (← instantiateMVars (← goal.getType)).find?
        (·.isAppOf ``VerexIR.SpecInt.ofInt) |>.isSome
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let ty ← instantiateMVars decl.type
        unless ← isFact ty do continue
        targets := targets.push decl.fvarId
        cut := cut || (ty.find? (·.isAppOf ``VerexIR.SpecInt.ofInt)).isSome
      pure (targets, cut)
    if cut then
      if let some goal ← simpAt goal (← `(tactic| simp only [VerexIR.SpecInt.ofInt_eq_iff,
          VerexIR.SpecInt.eq_ofInt_iff, VerexIR.SpecMap.val_ofInt_keyAt_ofEntries,
          VerexIR.SpecMap.val_ofInt_valueAt_ofEntries])) #[] targets true then
        replaceMainGoal [goal]
      else replaceMainGoal []
  for _ in [:16] do
    if (← getGoals).isEmpty then return
    let goal ← getMainGoal
    let next? ← goal.withContext do
      let mut found := typedMaps (← instantiateMVars (← goal.getType))
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let ty ← instantiateMVars decl.type
        -- A named map's read names nothing again. Canonical native
        -- validity alone does not request another model value view.
        if let some (_, lhs, rhs) := ty.eq? then
          if lhs.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 && rhs.isFVar then continue
        if (StatedFacts.unmarked ty).isAppOfArity ``VerexIR.SpecMap.Valid 4 then continue
        if ← isFact ty then found := typedMaps ty found
      found.findM? fun (_, carrier) => do
        let some decl := (← getLCtx).find? carrier | return false
        return !decl.userName.isAnonymous && !decl.userName.hasMacroScopes
    let some (map, carrier) := next? | break
    -- Reuse the witness of this exact typed view. Naming a second alias
    -- would separate its validity facts from the map read in the goal.
    let read? ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail then return none
        let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | return none
        unless rhs.isFVar && lhs.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 &&
            lhs.appArg! == map.appArg! do return none
        return if ← withDefault (isDefEq lhs map) then some decl else none
    if let some read := read? then
      -- The view may differ only in a definitionally equal carrier family.
      -- State the existing witness at the requested view before rewriting.
      let goal ← goal.withContext do
        let some (_, _, named) := (← instantiateMVars read.type).eq?
          | throwError "the map's read witness is not an equation"
        goal.replaceLocalDeclDefEq read.fvarId (← mkEq map named)
      replaceMainGoal [goal]
      let targets ← goal.withContext do
        (← getLCtx).foldlM (init := #[]) fun found decl => do
          if decl.isImplementationDetail || decl.fvarId == read.fvarId then return found
          return if ← isFact (← instantiateMVars decl.type) then found.push decl.fvarId else found
      if let some goal ← simpAt goal (← `(tactic| simp only [])) #[read.fvarId] targets true then
        replaceMainGoal [goal]
      else replaceMainGoal []
      continue
    let name ← goal.withContext do
      let owner := (← carrier.getDecl).userName
      if (typedMap? map).isSome then return owner
      let mut name := owner.appendAfter "_map"
      while ((← getLCtx).findFromUserName? name).isSome do name := name.appendAfter "'"
      return name
    let sizeBound ← goal.withContext do
      mkAppM ``VerexIR.SpecMap.size_ofEntries_bounds #[map.getArg! 4, map.getArg! 5, map.getArg! 6]
    let (_, bounded) ← (← goal.assert (Name.mkSimple s!"{name}_size_bounds")
      (← goal.withContext (inferType sizeBound)) sizeBound).intro1P
    -- Integer keys and values fit their carriers' range.
    let bounded ← if (map.getArg! 4).isAppOfArity ``VerexIR.SpecInt.val 2 then do
        let fit ← bounded.withContext do
          mkAppM ``VerexIR.SpecMap.keys_ofEntries_fit #[map.getArg! 5, map.getArg! 6]
        let (_, next) ← (← bounded.assert (Name.mkSimple s!"{name}_keys")
          (← bounded.withContext (inferType fit)) fit).intro1P
        pure next
      else pure bounded
    let bounded ← if (map.getArg! 5).isAppOfArity ``VerexIR.SpecInt.val 2 then do
        let fit ← bounded.withContext do
          mkAppM ``VerexIR.SpecMap.values_ofEntries_fit #[map.getArg! 4, map.getArg! 6]
        let (_, next) ← (← bounded.assert (Name.mkSimple s!"{name}_values")
          (← bounded.withContext (inferType fit)) fit).intro1P
        pure next
      else pure bounded
    -- The carrier's own bound (`c.entries.length < 2 ^ 64`) restates the
    -- map's size bounds; a fact reading the carrier otherwise, such as a
    -- specification function applied to it, stays.
    let stale ← bounded.withContext do
      (← getLCtx).foldlM (init := #[]) fun found decl => do
        if decl.isImplementationDetail then return found
        let ty ← instantiateMVars decl.type
        unless ty.isAppOfArity ``LT.lt 4 do return found
        let length := (ty.getArg! 2).consumeMData
        unless length.isAppOfArity ``List.length 2 do return found
        let entries := (length.getArg! 1).consumeMData
        if entries.isAppOfArity ``VerexIR.SpecEntries.entries 2 && entries.getArg! 1 == mkFVar carrier then
          return found.push decl.fvarId
        return found
    let goal ← bounded.tryClearMany stale
    let hyps ← goal.withContext do
      (← getLCtx).foldlM (init := #[]) fun found decl => do
        if decl.isImplementationDetail then return found
        let ty ← instantiateMVars decl.type
        return if (← isFact ty) && (typedMaps ty).any (·.1 == map) then found.push decl.fvarId
          else found
    let (reverted, goal) ← goal.revert hyps (preserveOrder := true)
    let saved ← saveState
    -- A bare carrier is the named map written back. For a projected
    -- field, writeBackCarrier declines and the read bridge alone remains.
    let goal ← try
      let (introduced, goal) ← goal.generalize
        #[{ expr := map, xName? := some name, hName? := some (Name.mkSimple s!"{name}_read") }]
      let some definition := introduced[1]? | throwError "the map's definition is not introduced"
      pure ((← writeBackCarrier goal definition).getD goal)
    catch _ =>
      saved.restore
      let (localMap, named) ← (← goal.define name (← goal.withContext (inferType map)) map).intro1P
      let target ← named.withContext (instantiateMVars (← named.getType))
      named.replaceTargetDefEq (target.replace fun term =>
        if term == map then some (mkFVar localMap) else none)
    let (_, goal) ← goal.introNP reverted.size
    replaceMainGoal [goal]

/-- Entries compared entry by entry have the same keys, where the comparison
compares keys by value. -/
theorem keys_of_listEqual {K V : Type} (equal : K × V → K × V → Bool)
    (sameKey : ∀ l r, equal l r = true → l.1 = r.1) :
    (left right : List (K × V)) → listEqual equal left right = true →
      left.map Prod.fst = right.map Prod.fst
  | [], [], _ => rfl
  | _ :: _, [], same => by simp [listEqual] at same
  | [], _ :: _, same => by simp [listEqual] at same
  | a :: as, b :: bs, same => by
      simp only [listEqual, Bool.and_eq_true] at same
      simp [sameKey a b same.1, keys_of_listEqual equal sameKey as bs same.2]

/-- A map's validity reads only its key sequence. -/
theorem valid_of_same_keys {K V : Type} (discipline : SpecMap.Discipline K) {m n : SpecMap K V}
    (keys : m.map Prod.fst = n.map Prod.fst) (valid : SpecMap.Valid discipline n) :
    SpecMap.Valid discipline m := by
  cases discipline with
  | sequence => simpa [SpecMap.Valid, SpecMap.Distinct, keys] using valid
  | ordered order =>
      have pairwise : ∀ (a b : List (K × V)), a.map Prod.fst = b.map Prod.fst →
          SpecMap.Ascending order b → SpecMap.Ascending order a := by
        intro a b same held
        have read : ∀ c : List (K × V), SpecMap.Ascending order c ↔
            (c.map Prod.fst).Pairwise fun x y => order.compare x y = .lt := by
          intro c
          simp [SpecMap.Ascending, List.pairwise_map]
        rw [read] at held ⊢
        rwa [same]
      exact pairwise m n keys valid

/-- The validity of a table's map equated entry by entry to a valid map's
table (`doc/designs/proof-language.md`, "Normal-form fixes"). -/
theorem valid_of_listEqual {K V K' V' : Type} (key : K → K') (value : V → V')
    (discipline : SpecMap.Discipline K') (equal : K × V → K × V → Bool)
    (sameKey : ∀ l r, equal l r = true → l.1 = r.1) {left right : SpecEntries (K × V)}
    {m n : SpecMap K' V'} (readLeft : SpecMap.ofEntries key value left = m)
    (readRight : SpecMap.ofEntries key value right = n)
    (same : listEqual equal left.entries right.entries = true)
    (valid : SpecMap.Valid discipline n) : SpecMap.Valid discipline m := by
  subst readLeft readRight
  refine valid_of_same_keys discipline ?_ valid
  have mapped := congrArg (List.map key) (keys_of_listEqual equal sameKey _ _ same)
  simpa [SpecMap.ofEntries, List.map_map, Function.comp_def] using mapped

/-- Preparation's facts of a map equated entry by entry to a valid map: for a
hypothesis `listEqual eq left right = true` whose `eq` compares keys by value,
and the maps two bridges read `left` and `right` as, `Valid r m` from `Valid r
n`, either way round. -/
private def validityOfEqualTables (goal : MVarId) : TacticM MVarId := goal.withContext do
  let mut goal := goal
  let decls := (← getLCtx).foldl (init := #[]) (·.push ·)
  let types ← decls.mapM fun decl => return (decl, (← instantiateMVars decl.type).consumeMData)
  -- The bridges: `ofEntries key value table = m`.
  let bridges := types.filterMap fun (decl, type) => do
    let (_, lhs, rhs) ← type.eq?
    guard (lhs.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 && rhs.isFVar)
    pure (decl, lhs, rhs)
  if bridges.isEmpty then return goal
  let valids := types.filterMap fun (decl, type) =>
    if type.isAppOf ``VerexIR.SpecMap.Valid then some (decl, type) else none
  let bridgeOf (entries : Lean.Expr) : Option (Lean.LocalDecl × Lean.Expr × Lean.Expr) :=
    bridges.find? fun (_, lhs, _) =>
      entries.isAppOfArity ``SpecEntries.entries 2 && lhs.appArg! == entries.appArg!
  for (decl, type) in types do
    let some (_, compared, literal) := type.eq? | continue
    unless literal.isConstOf ``Bool.true && compared.isAppOfArity ``listEqual 4 do continue
    let equal := compared.getArg! 1
    let some (readLeft, lhs, m) := bridgeOf (compared.getArg! 2) | continue
    let some (readRight, rhsRead, n) := bridgeOf (compared.getArg! 3) | continue
    unless lhs.getArg! 4 == rhsRead.getArg! 4 && lhs.getArg! 5 == rhsRead.getArg! 5 do continue
    let some (validDecl, validType) := valids.find? fun (_, valid) => valid.appArg! == n | continue
    if valids.any fun (_, valid) => valid.appArg! == m then continue
    let saved ← saveState
    try
      let args := compared.getAppArgs
      let some pair := (← whnfR args[0]!).app2? ``Prod | throwError "not a pair"
      let sameKey ← mkFreshExprSyntheticOpaqueMVar (← withLocalDeclD `l args[0]! fun l =>
        withLocalDeclD `r args[0]! fun r => do
          mkForallFVars #[l, r] (← mkArrow (← mkEq (mkApp2 equal l r) literal)
            (← mkEq (mkApp3 (mkConst ``Prod.fst [0, 0]) pair.1 pair.2 l)
              (mkApp3 (mkConst ``Prod.fst [0, 0]) pair.1 pair.2 r))))
      setGoals [sameKey.mvarId!]
      evalTactic (← `(tactic| (
        intro _ _ equal
        simp only [Bool.and_eq_true] at equal
        simpa [VerexIR.SpecInt.eq_iff_val] using equal.1)))
      unless (← getUnsolvedGoals).isEmpty do throwError "the comparison reads no key"
      let proof ← mkAppM ``valid_of_listEqual #[lhs.getArg! 4, lhs.getArg! 5, validType.getArg! 2,
        equal, ← instantiateMVars sameKey, readLeft.toExpr, readRight.toExpr, decl.toExpr,
        validDecl.toExpr]
      let (_, next) ← (← goal.assert `valid_equal (← inferType proof) proof).intro1P
      goal := next
    catch _ => saved.restore
  return goal

/-- Name the maps of a prepared leaf (`nameTypedMaps`), after abstracting
the rank tables they are ordered by, with the bounds of the values read from
the named maps (`typeBounds`) and the validity of maps equated to valid ones
(`validityOfEqualTables`). -/
elab "verex_name_maps" : tactic => focusGoal do
  nameTypedMaps
  if (← getGoals).isEmpty then return
  replaceMainGoal [← abstractRankTables (← getMainGoal)]
  typeBounds
  if (← getGoals).isEmpty then return
  let next ← validityOfEqualTables (← getMainGoal)
  setGoals [next]

/-- Finish authored preparation, retaining source markers while the structural
walker still needs them to name the clauses of a shared path. -/
elab "verex_finish_prepare" : tactic => focusGoal do
  if verex.keepObligationMarkers.get (← getOptions) then
    evalTactic (← `(tactic| try simp only [Int.add_zero, Int.zero_add]))
  else
    evalTactic (← `(tactic| try simp only
      [VerexIR.Proofs.Obligation_eq, Int.add_zero, Int.zero_add]))

/-- A leaf in source form: the intermediate values the solver names replaced
by the expressions they stand for, normalized as the solver normalizes a
leaf, a case per variant of each local it tests, its clauses unmarked. -/
macro "verex_prepare" : tactic => `(tactic| verex_focus (
  verex_paths
  all_goals (
    verex_defined_values
    (try verex_unname)
    (try verex_prepare_leaf)
    all_goals (try (verex_split_variants; all_goals (try verex_prepare_leaf)))
    all_goals verex_finish_prepare
    all_goals (try verex_name_maps))))

/-- Prepare values for an authored argument without deriving speculative map
position facts or running the automatic solver's saturation passes. -/
macro "verex_prepare" " only" : tactic => `(tactic| (
  verex_paths
  all_goals verex_defined_values
  all_goals verex_canonical_families
  all_goals verex_clear_computations
  all_goals try verex_unname
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals verex_reduce_projections
  all_goals try verex_trivial
  all_goals verex_certified_clauses
  all_goals verex_normalize_context
  all_goals verex_spec_forms
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals verex_reduce_projections
  all_goals verex_intro_any
  all_goals verex_type_bounds
  all_goals verex_finish_prepare
  all_goals try verex_trivial
  all_goals verex_name_maps))

/-- Decide a leaf by bounded deciders over its context as it stands, its
hypotheses split and its premises introduced: assumption, `omega`, ground
evaluation, an instance of a quantified hypothesis, rewriting by the
context. An attempt that exhausts its bound fails as any other. -/
elab "verex_decide" : tactic => focusGoal <| withoutRecover do
  evalTactic (← `(tactic| verex_paths))
  evalTactic (← `(tactic| all_goals verex_defined_values))
  evalTactic (← `(tactic| all_goals verex_leaf_decide))

/-- Decide a leaf as the automatic verification does, or report the clause
it owes and admit it. -/
elab "verex_solve" : tactic => focusGoal do
  evalTactic (← `(tactic| verex_paths))
  let normalization ← normalization
  for goal in ← getGoals do
    let (provenance, clauses) := ((← (← targetState).leafOrigins.get)[goal]?).getD (none, #[])
    discard <| solveLeaf normalization goal provenance clauses
  setGoals []

/-- An obligation's key (`ObligationKey`), as a proof addresses it:
`ensures 1`, `invariant 2 step`, `requires 1 of iter_next at call 2`,
`data invariant`. A key of one word is written as a tag. A callee's
precondition no clause of it states, such as a data invariant of an
argument, has no number (`requires of borrow at call 1`). -/
declare_syntax_cat verexKey (behavior := symbol)
syntax (name := keyEnsures) &"ensures" num : verexKey
syntax (name := keyAbortsIf) &"aborts_if" num : verexKey
syntax (name := keyAbortsWith) &"aborts_with" num : verexKey
syntax (name := keyAssert) &"assert" num
  (&"of" ((&"at" verexPoint) <|> (&"after" verexSite)))? : verexKey
syntax (name := keyInvariant) &"invariant" num (&"of" &"loop" num)? (&"entry" <|> &"step") : verexKey
syntax (name := keyRequires) &"requires" (num)? &"of" ident (&"at" &"call" num)? : verexKey
syntax (name := keyDataInvariant) &"data" &"invariant" (num)? (&"of" ident)? : verexKey
syntax (name := keyModuleInvariant) &"module" &"invariant" : verexKey
syntax (name := keyStoredInvariant) &"stored" &"invariant" (num)? : verexKey
syntax (name := keyLemmaRequires) &"lemma" &"requires" : verexKey
syntax (name := keyGhostDomain) &"ghost" &"domain" : verexKey
syntax (name := keyAfterCall) &"after" verexSite : verexKey

/-- A point as a key or a step spells it: `at` and a point (`entry`, `exit`,
`loop k entry|step`), or `after` and a site. -/
def pointOfSyntax {m : Type → Type} [Monad m] [MonadError m] (keyword point : Syntax) :
    m ProgramPointKey := do
  let number (stx : Syntax) : Nat := stx.isNatLit?.getD 0
  if keyword.isAtom && keyword.getAtomVal == "at" then
    if point.isOfKind ``pointEntry then return .entry
    if point.isOfKind ``pointExit then return .exit
    if point.isOfKind ``pointLoop then
      let phase := point[2]
      let phase := if phase.isAtom then phase.getAtomVal else phase[0].getAtomVal
      return if phase == "entry" then .loopEntry (number point[1]) else .loopStep (number point[1])
  else
    if point.isOfKind ``siteCall then
      let callee := if point[2].getNumArgs == 2 then point[2][1].getId.toString else ""
      return .after (.call (number point[1]) callee 0)
    if point.isOfKind ``siteOp then return .after (.op (number point[1]))
    if point.isOfKind ``siteInvoke then return .after (.invoke (number point[1]))
    if point.isOfKind ``siteAbort then
      throwErrorAt point "an abort has no continuation: `after abort` names no point"
  throwErrorAt point "unexpected point"

/-- The key a key's syntax states, its omitted parts any. -/
def keyPattern {m : Type → Type} [Monad m] [MonadError m] (stx : Syntax) : m ObligationKey := do
  -- `assert k of at p` or `assert k of after s`: an assert of a point step.
  if stx.isOfKind ``keyAssert && stx[2].getNumArgs > 0 then
    let parts := stx[2][1]
    let point ← pointOfSyntax parts[0] parts[1]
    return { kind := "assert", index := stx[1].isNatLit?.getD 0, point }
  match stx with
  | `(verexKey| ensures $k:num) => pure { kind := "ensures", index := k.getNat }
  | `(verexKey| aborts_if $k:num) => pure { kind := "aborts_if", index := k.getNat }
  | `(verexKey| aborts_with $k:num) => pure { kind := "aborts_with", index := k.getNat }
  | `(verexKey| assert $k:num) => pure { kind := "assert", index := k.getNat }
  | `(verexKey| invariant $k:num $[of loop $loop?:num]? entry) =>
      pure { kind := "invariant", index := k.getNat, phase := "entry", loop := loop?.map (·.getNat) }
  | `(verexKey| invariant $k:num $[of loop $loop?:num]? step) =>
      pure { kind := "invariant", index := k.getNat, phase := "step", loop := loop?.map (·.getNat) }
  | `(verexKey| requires $[$k?:num]? of $callee:ident $[at call $call?:num]?) =>
      pure { kind := "requires", index := k?.map (·.getNat), callee := callee.getId.toString,
             call := call?.map (·.getNat) }
  | `(verexKey| data invariant $[$k?]? $[of $owner?:ident]?) =>
      pure { kind := "data invariant", index := k?.map (·.getNat),
             owner := owner?.map (·.getId.toString) }
  | `(verexKey| module invariant) => pure { kind := "module invariant" }
  | `(verexKey| stored invariant $[$k?]?) =>
      pure { kind := "stored invariant", index := k?.map (·.getNat) }
  | `(verexKey| lemma requires) => pure { kind := "lemma requires" }
  | `(verexKey| ghost domain) => pure { kind := "ghost domain" }
  | `(verexKey| after $site:verexSite) => do
      -- A continuation is keyed by its call: no other site owes one.
      unless site.raw.isOfKind ``siteCall do
        throwErrorAt site "an obligation is keyed after a call (`after call k of g`)"
      let number := site.raw[1].isNatLit?.getD 0
      unless site.raw[2].getNumArgs == 2 do
        throwErrorAt site "an obligation after a call names its callee (`after call k of g`)"
      pure { kind := "after", callee := site.raw[2][1].getId.toString, call := number }
  | _ => throwErrorAt stx "unexpected obligation key"

/-- `verex_step_routing [all] k, …`: the obligations a proof's `on` steps
take, and with `all` an `on _` taking the rest (`TargetState.routedKeys`). -/
syntax (name := verexStepRouting) "verex_step_routing" (&"all")? verexKey,* : tactic

@[tactic verexStepRouting] def evalStepRouting : Tactic := fun stx => do
  let keys ← stx[2].getSepArgs.mapM keyPattern
  (← targetState).routedKeys.set (some (keys, stx[1].getNumArgs > 0))

/-- Report each selector of the proof's `verex_on` alternatives that took no
path, with the decisions of the paths of the obligations it names. -/
def reportUnusedSelectors : TacticM Unit := do
  for (stx, stated, used, paths) in ← (← targetState).selectorUses.get do
    unless used do
      logErrorAt stx m!"no path takes `when {stated}`; the paths' decisions: \
        {if paths.isEmpty then "no path" else "; ".intercalate (paths.map (s!"[{·}]")).toList}"
  (← targetState).selectorUses.set #[]


/-- A source decision a selector states (`Decision`): `if 1 then`, `match 2
Some`, `loop 1 exits`, `aborts at call 1 of add_shares`. -/
declare_syntax_cat verexDecision (behavior := symbol)
syntax (name := decisionThen) &"if" num &"then" : verexDecision
syntax (name := decisionElse) &"if" num &"else" : verexDecision
syntax (name := decisionVariant) &"match" num ident : verexDecision
syntax (name := decisionExits) &"loop" num &"exits" : verexDecision
syntax (name := decisionContinues) &"loop" num &"continues" : verexDecision
syntax (name := decisionAborts) &"aborts" &"at" verexSite : verexDecision
syntax (name := decisionInvoke) &"invoke" num ident : verexDecision
syntax (name := decisionReturns) &"returns" : verexDecision

/-- The decision a selector's syntax states, a call without its callee
counted among all calls. -/
def decisionPattern (stx : Syntax) : TacticM Decision :=
  match stx with
  | `(verexDecision| if $k:num then) => pure (.branch k.getNat true)
  | `(verexDecision| if $k:num else) => pure (.branch k.getNat false)
  | `(verexDecision| match $k:num $variant:ident) => pure (.variant k.getNat variant.getId.toString)
  | `(verexDecision| loop $k:num exits) => pure (.loop k.getNat true)
  | `(verexDecision| loop $k:num continues) => pure (.loop k.getNat false)
  | `(verexDecision| aborts at call $k:num of $callee:ident) =>
      pure (.aborts (.call k.getNat callee.getId.toString 0))
  | `(verexDecision| aborts at call $k:num) => pure (.aborts (.call k.getNat "" 0))
  | `(verexDecision| aborts at abort $k:num) => pure (.aborts (.abort k.getNat))
  | `(verexDecision| aborts at op $k:num) => pure (.aborts (.op k.getNat))
  | `(verexDecision| aborts at invoke $k:num) => pure (.aborts (.invoke k.getNat))
  | `(verexDecision| invoke $k:num $target:ident) => pure (.invoke k.getNat target.getId.toString)
  | `(verexDecision| returns) => pure .returns
  | _ => throwErrorAt stx "unexpected decision"

/-- An obligation an alternative of `verex_on` takes: by its key, or by its
tag. -/
syntax verexOnItem := verexKey <|> ident

declare_syntax_cat verexOnPattern
syntax (name := verexOnClauses) verexOnItem,+ (&"when" verexDecision,+)? : verexOnPattern
syntax (name := verexOnDefault) "_" : verexOnPattern

/-- The leaves an alternative of `verex_on` takes: those owing one of the
obligations it names, on paths its selector takes, or every leaf no earlier
alternative takes. -/
syntax verexOnAlternative := "| " verexOnPattern " => " tacticSeq

/-- Prove each leaf by the first alternative naming the obligation it owes,
or by the `_` alternative: `verex_on | ensures 1, requires 1 of g at call 2
=> tac | _ => tac'`. An alternative names an obligation by its key, any part
the key omits matching, or by its tag (`ensures_1`), where several leaves
owe one obligation the tag with its number. A selector, `| aborts_if 1 when
aborts at abort 1 => tac`, takes the paths whose source decisions it states
out of their obligation's goal; the paths it does not take go to the later
alternatives, and a selector taking no path is an error listing the
decisions of the obligation's paths. A leaf no alternative takes is
attempted by `verex_decide` only; if undecided it stays a goal, as do the
goals an alternative leaves. Named alternatives run directly, without
attempting automation first. -/
syntax (name := verexOn) "verex_on" withPosition((ppDedent(ppLine) colGe verexOnAlternative)+) :
  tactic

/-- An alternative of a routing (`routeLeaves`): the obligations it names,
each a key or a tag (none for the default), its selector, the syntax a
selector taking no path is reported at, and its tactics. -/
structure RouteAlternative where
  items : Option (Array (Sum ObligationKey Name))
  selector : Array Decision
  reference : Syntax
  steps : Syntax
  deriving Inhabited

/-- A leaf's obligation key, by its tag, and the tag's leaf name. -/
def leafKeyOf (goal : MVarId) : TacticM (ObligationKey × Name) := do
  let leaf := (← goal.getTag).components.head?.getD .anonymous
  return (((← (← targetState).leafKeys.get)[leaf]?).getD { kind := toString leaf }, leaf)

/-- Whether an alternative's items name a leaf's obligation; a tag omitting
the loop of an invariant (`invariant_2_step`) names it as a key does. -/
def namesLeaf (items : Array (Sum ObligationKey Name)) (key : ObligationKey) (leaf : Name) : Bool :=
  items.any fun
    | .inl pattern => pattern.selects key
    | .inr name => name == key.tag || name == { key with loop := none }.tag || name == leaf

/-- Whether an alternative names a leaf's obligation: a default names all. -/
def RouteAlternative.names (alternative : RouteAlternative) (key : ObligationKey) (leaf : Name) :
    Bool :=
  alternative.items.all (namesLeaf · key leaf)

/-- The alternative a leaf goes to: the first naming its obligation, on a
path its selector takes, or the default. -/
def chosenAlternative (alternatives : Array RouteAlternative) (goal : MVarId) :
    TacticM (Option Nat) := do
  let (key, leaf) ← leafKeyOf goal
  let decisions := ((← (← targetState).leafDecisions.get)[goal]?).getD #[]
  return alternatives.findIdx? fun alternative =>
    alternative.names key leaf && alternative.selector.all fun pattern =>
      decisions.any (pattern.selects ·)

/-- The disjuncts of a disjunction, in order. -/
private partial def disjuncts (e : Lean.Expr) : Array Lean.Expr :=
  let e := e.consumeMData
  if e.isAppOfArity ``Or 2 then disjuncts (e.getArg! 0) ++ disjuncts (e.getArg! 1) else #[e]

/-- The negation of a disjunction, by cases over its tree, from each disjunct's. -/
private partial def refuteDisjunction (refuted : Array (Lean.Expr × Lean.Expr)) (e : Lean.Expr) :
    MetaM Lean.Expr := do
  let e := e.consumeMData
  if e.isAppOfArity ``Or 2 then
    let (left, right) := (e.getArg! 0, e.getArg! 1)
    let proofLeft ← refuteDisjunction refuted left
    let proofRight ← refuteDisjunction refuted right
    return .lam `held e (mkApp6 (mkConst ``Or.elim) left right (mkConst ``False) (.bvar 0)
      proofLeft proofRight) .default
  let some (_, proof) := refuted.find? (·.1 == e)
    | throwError "a clause of the abort condition has no refutation"
  return proof

/-- A returning path's leaf taken apart at the clauses in `named`, once: `¬(A ∨ B
∨ …)` as `¬A` for each named clause, keyed by it, and the conjunction of the
others' negations as one goal, keyed by them. A leaf whose disjuncts the normal
form no longer counts one per clause stays whole. -/
private def takeClauses (goal : MVarId) (named : Array Nat) : TacticM (Array MVarId) := do
  let (key, _) ← leafKeyOf goal
  let some first := key.index | return #[goal]
  let indices := #[first] ++ key.also
  goal.withContext do
    let target ← whnfR (← instantiateMVars (← goal.getType))
    let some condition := target.not? | return #[goal]
    let parts := disjuncts condition
    unless parts.size == indices.size do return #[goal]
    let origin := ((← (← targetState).leafOrigins.get)[goal]?).getD (none, #[])
    let decisions := ((← (← targetState).leafDecisions.get)[goal]?).getD #[]
    let opened (statement : Lean.Expr) (key : ObligationKey) : TacticM Lean.Expr := do
      let tag := key.tag
      let owed ← mkFreshExprSyntheticOpaqueMVar statement tag
      (← targetState).leafKeys.modify (·.insert tag key)
      (← targetState).leafOrigins.modify (·.insert owed.mvarId! origin)
      (← targetState).leafDecisions.modify (·.insert owed.mvarId! decisions)
      return owed
    let mut refuted : Array (Lean.Expr × Lean.Expr) := #[]
    let mut goals := #[]
    for part in parts, index in indices do
      if named.contains index then
        let owed ← opened (mkNot part) { key with index := some index, also := #[] }
        refuted := refuted.push (part, owed)
        goals := goals.push owed.mvarId!
    let others := (parts.zip indices).filter fun (_, index) => !named.contains index
    if let some last := others.back? then
      let statement := others.pop.foldr (fun (part, _) rest => mkAnd (mkNot part) rest) (mkNot last.1)
      let numbers := others.map (·.2)
      let owed ← opened statement
        { key with index := numbers[0]?, also := numbers.extract 1 numbers.size }
      goals := goals.push owed.mvarId!
      -- Each other's negation, its projection of the conjunction.
      let mut rest := owed
      for (part, _) in others, position in [0:others.size] do
        if position + 1 < others.size then
          refuted := refuted.push (part, .proj ``And 0 rest)
          rest := .proj ``And 1 rest
        else refuted := refuted.push (part, rest)
    goal.assign (← mkExpectedTypeHint (← refuteDisjunction refuted condition) target)
    return goals

/-- Prove each leaf by the first alternative naming the obligation it owes,
on a path its selector takes, or by the default alternative; a leaf no
alternative takes is attempted by `verex_decide` only. -/
def routeLeaves (alternatives : Array RouteAlternative) : TacticM Unit := do
  let keyOf := leafKeyOf
  let names := namesLeaf
  -- A goal an alternative with a selector names is taken apart into its
  -- paths, which the alternatives then take one by one; a default with a
  -- selector names it unless an alternative before it takes the whole goal.
  let takesPaths (key : ObligationKey) (leaf : Name) : Bool := Id.run do
    for alternative in alternatives do
      if alternative.names key leaf then
        if !alternative.selector.isEmpty then return true
        if alternative.items.isNone then return false
    return alternatives.any fun alternative =>
      !alternative.selector.isEmpty && alternative.items.any (names · key leaf)
  -- A leaf owing several clauses at once is taken apart only at the clauses an
  -- alternative names by number.
  let namedClauses (key : ObligationKey) : Array Nat :=
    (alternatives.flatMap fun alternative => (alternative.items.getD #[]).filterMap fun
      | .inl pattern => if pattern.selects key then pattern.index else none
      | .inr _ => none)
  let mut work := #[]
  for goal in ← getGoals do
    if ← goal.isAssigned then continue
    let (key, leaf) ← keyOf goal
    let named := if key.also.isEmpty then #[] else namedClauses key
    if takesPaths key leaf || !named.isEmpty then
      for path in ← splitPaths goal do
        if named.isEmpty then work := work.push path
        else work := work ++ (← takeClauses path named)
    else work := work.push goal
  let decisionsOf ← (← targetState).leafDecisions.get
  let mut taken := alternatives.map fun _ => false
  let mut remaining := #[]
  let mut aborted := false
  for goal in work do
    let (key, _) ← keyOf goal
    let chosen ← chosenAlternative alternatives goal
    let steps ← match chosen with
      | some index =>
          taken := taken.set! index true
          pure alternatives[index]!.steps
      | none => pure (← `(tactic| try verex_decide)).raw
    setGoals [goal]
    let saved ← saveState
    let errorsBefore := (← Core.getMessageLog).hasErrors
    try
      -- Without recovery: a failing step raises its own error, which is
      -- reported at the leaf; recovering, `all_goals` over a `first` drops
      -- the error of the failing alternative and aborts without a report.
      withoutRecover (evalTactic steps)
      remaining := remaining ++ (← getUnsolvedGoals).toArray
      -- A bullet whose goal stays open admits it without a report when not
      -- recovering: the admission is reported here.
      if (← instantiateMVars (mkMVar goal)).hasSorry && !errorsBefore &&
          !(← Core.getMessageLog).hasErrors then
        logError m!"the proof of `{key.text}` leaves a goal open under a bullet; the bullet's \
          steps outside it report what is left"
        aborted := true
    catch failure =>
      unless (← read).recover do throw failure
      logException failure
      unless errorsBefore || (← Core.getMessageLog).hasErrors do
        logError m!"the proof of `{key.text}` fails without a report"
      let log ← Core.getMessageLog
      saved.restore
      Core.setMessageLog log
      admitGoal goal
      aborted := true
  -- What each selector took, for the proof's report of a selector taking no
  -- path (`reportUnusedSelectors`).
  for alternative in alternatives, used in taken do
    let selector := alternative.selector
    if selector.isEmpty then continue
    let mut paths := #[]
    for goal in work do
      let (key, leaf) ← keyOf goal
      if alternative.names key leaf then
        let decisions := decisionsOf[goal]?.getD #[]
        paths := paths.push (", ".intercalate (decisions.map (·.text)).toList)
    let stated := ", ".intercalate (selector.map (·.text)).toList
    (← targetState).selectorUses.modify fun uses =>
      match uses.findIdx? (·.1 == alternative.reference) with
      | some index => uses.modify index fun (stx, text, before, seen) =>
          (stx, text, before || used, seen ++ paths)
      | none => uses.push (alternative.reference, stated, used, paths)
  -- Outside a target's proof (`verex_proof`), this is the whole proof.
  if (← (← targetState).proofName.get).isAnonymous then reportUnusedSelectors
  if aborted then throwAbortTactic
  setGoals remaining.toList

@[tactic verexOn] def evalVerexOn : Tactic := fun stx => do
  routeLeaves (← stx[1].getArgs.mapM fun alternative => do
    let pattern := alternative[1]
    if pattern.isOfKind ``verexOnDefault then
      return { items := none, selector := #[], reference := pattern, steps := alternative[3] }
    let items ← pattern[0].getSepArgs.mapM fun item => do
      let item := item[0]
      if item.isIdent then pure (Sum.inr item.getId) else Sum.inl <$> keyPattern item
    -- The optional `when …` holds the keyword and the decisions.
    let selector ← if pattern[1].getNumArgs == 2 then pattern[1][1].getSepArgs.mapM decisionPattern
      else pure #[]
    return { items := some items, selector, reference := pattern, steps := alternative[3] })

/-- Prove a step a lemma's proof owes: what its lemma applications require,
its assertions, and its case splits. -/
elab "verex_lemma_owed" : tactic => focusGoal do
  let parts ← dischargeLemmaSteps (← getMainGoal)
  let mut residual := #[]
  for (part, _) in parts do
    setGoals [part]
    evalTactic (← `(tactic| try verex_normalize))
    evalTactic (← `(tactic| all_goals verex_solve_goals))
    residual := residual ++ (← getGoals).toArray
  setGoals residual.toList

/-- What a step of a lemma's proof gives once proved, in the hypothesis
`h`: each lemma application's conclusion, and one goal per case of a split. -/
elab "verex_lemma_holds " h:ident : tactic => focusGoal do
  let goal ← getMainGoal
  let some decl := (← goal.withContext getLCtx).findFromUserName? h.getId
    | throwError m!"no hypothesis `{h.getId}`"
  replaceMainGoal [← replaceByLemmaFacts goal decl.fvarId]
  evalTactic (← `(tactic| try simp only [VerexIR.Proofs.Obligation_eq] at $h:ident))
  evalTactic (← `(tactic| try verex_normalize at $h:ident))
  evalTactic (← `(tactic| verex_split_hypotheses))
  let before ← getGoals
  let cases ← before.flatMapM splitCases
  setGoals cases
  if cases.length != before.length then
    evalTactic (← `(tactic| all_goals (try verex_normalize at *)))

/-- A recursive application's measure descends: componentwise as the Move
Prover's condition states it, decided as an obligation. -/
macro "verex_lemma_decreasing" : tactic => `(tactic| verex_focus (
  ((try simp only [Prod.lex_def, VerexIR.Proofs.lemmaMeasure])
   first
   | omega
   | ((try verex_normalize)
      verex_obligations []
      all_goals (first | verex_decide | (verex_prepare; all_goals verex_decide))
      done)
   | fail "a recursive application of the lemma does not decrease its measure")))

end VerexIR.Proofs.Denote
