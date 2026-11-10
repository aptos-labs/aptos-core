-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Normal

/-!
# The solver: leaf deciders

Specification unfolding, map positions, witnesses and instances, and the
deciders a leaf runs (`verex_leaf` and the tactics it is built from).
-/

namespace VerexIR.Proofs.Denote

open Lean Elab Tactic Meta

/-- The definition of a full application of a recursive specification
function, as the verifier recorded it (`SpecDefinition`): the definition's
leading parameters (what its body reads besides its arguments), then the
bundled arguments, which is how its unfolding theorem and its measure are
stated. -/
def specDefinitionAt? (env : Environment) (e : Lean.Expr) : Option SpecDefinition := do
  let .const name _ := e.getAppFn | none
  let definition ← specDefinition? env name
  let some info := env.find? definition.unfold | none
  guard (arity info.type == e.getAppNumArgs && e.getAppNumArgs > 0)
  definition
where
  arity : Lean.Expr → Nat
    | .forallE _ _ body _ => arity body + 1
    | _ => 0

/-- The unfolding theorem (`f.spec.unfold`) of a full application of a
recursive specification function. -/
def specUnfolding? (env : Environment) (e : Lean.Expr) : Option Name :=
  (specDefinitionAt? env e).map (·.unfold)

/-- The measure of a full application of a recursive specification
function, where it evaluates to a literal: the recursion descends on the
measure, so the literal bounds how often the application unfolds
(`evaluateSpecApplication`). -/
def specMeasure? (e : Lean.Expr) : MetaM (Option Nat) := do
  let some definition := specDefinitionAt? (← getEnv) e | return none
  (Lean.Meta.evalNat (← whnf (mkAppN definition.measure e.getAppArgs))).run

/-- The unfolding theorem at an application, and its statement: the instance
reads its arguments, not projections of their bundle, a definitional reduction
that keeps each argument once per use. -/
def specUnfoldingInstance (unfold : Name) (application : Lean.Expr) :
    MetaM (Lean.Expr × Lean.Expr) := do
  let proof := mkAppN (mkConst unfold) application.getAppArgs
  let type ← Meta.transform (← inferType proof) (post := fun e => do
    match ← reduceProjection? e with
    | some reduced => return .visit reduced
    | none => return .continue)
  return (proof, type)

/-- An unfolding `f a = if c then t else e` rewritten to the branch `decided`
takes: the proof and the statement. -/
def unfoldingBranch (equation decided : Lean.Expr) (holds : Bool) :
    MetaM (Lean.Expr × Lean.Expr) := do
  let type ← instantiateMVars (← inferType equation)
  let some (_, lhs, rhs) := type.eq? | throwError "an unfolding is an equation"
  let arguments := rhs.getAppArgs
  let rule := if rhs.isAppOf ``ite then (if holds then ``if_pos else ``if_neg)
    else (if holds then ``dif_pos else ``dif_neg)
  let branch ← mkAppOptM rule #[arguments[1]!, arguments[2]!, decided, arguments[0]!,
    arguments[3]!, arguments[4]!]
  let proof ← mkEqTrans equation branch
  let some (_, _, taken) := (← instantiateMVars (← inferType proof)).eq?
    | throwError "a branch is an equation"
  return (proof, ← mkEq lhs taken.headBeta)

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
    let (fact, next) ← goal.withContext do
      let (proof, type) ← specUnfoldingInstance unfold application
      (← goal.assert `unfolded type proof).intro1P
    goal := next
    facts := facts.push fact
  -- The guard of each instance, the condition its body branches on first,
  -- is decided over the facts connected to it by shared variables, and the
  -- instance rewritten to the branch the guard takes. An instance whose
  -- guard the context leaves open is dropped: rewriting by it would only
  -- unfold the function again.
  let debug := verex.solverDebug.get (← getOptions)
  let decisions ← goal.withContext do
    let candidates ← (← getLCtx).decls.toArray.filterMapM fun
      | some other => do
          if other.isImplementationDetail || facts.contains other.fvarId then return none
          let otherType ← instantiateMVars other.type
          return some (other.fvarId, (Lean.collectFVars {} otherType).fvarIds)
      | none => pure none
    -- A proof of the guard or of its negation, over the facts connected to
    -- it by shared variables and the bounds of the values they relate.
    let decide (guard : Lean.Expr) : TacticM (Option (Lean.Expr × Bool)) := do
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
        -- The values the guard relates carry their types' bounds, which a
        -- definition's domain guard states.
        let reduced ← reduced.withContext do
          let mut reduced := reduced
          for decl in ← getLCtx do
            if decl.isImplementationDetail then continue
            let some fact ← typedBound? decl.toExpr (← boundType decl.type)
              | continue
            (_, reduced) ← (← reduced.assert `bounds (← inferType fact) fact).intro1P
          pure reduced
        -- A side decision (`sideDecisionHeartbeats`): undecided, the
        -- instance is dropped.
        let saved ← saveState
        let closed ← try withBudget sideDecisionHeartbeats (closesBy reduced (← `(tactic|
            (try simp only [vir_denote_norm, beq_iff_eq, decide_eq_true_eq]) <;> omega)))
          catch _ => saved.restore; pure false
        if closed then
          return some (← instantiateMVars proof, holds)
      return none
    let branches (e : Lean.Expr) := e.isAppOfArity ``ite 5 || e.isAppOfArity ``dite 5
    facts.mapM fun fact => do
      let type ← instantiateMVars (← fact.getType)
      let dropped : TacticM (FVarId × Option (Array (Lean.Expr × Bool))) := do
        if debug then
          IO.println s!"    unfolded (dropped): {((toString (← ppExpr type)).replace "\n" " ").take 300}"
        return (fact, none)
      let some (_, _, rhs) := type.eq? | return (fact, some #[])
      unless branches rhs do return (fact, some #[])
      let some first ← decide (rhs.getArg! 1) | return ← dropped
      -- A definition's domain guard (`bounds`) does not tell whether its
      -- instance is worth keeping; the condition its body branches on first
      -- does, as it does for a function without a domain.
      let domain := rhs.isAppOfArity ``dite 5 &&
        (match rhs.getArg! 3 with | .lam name .. => name == `bounds | _ => false)
      if domain && first.2 then
        let body := (rhs.getArg! 3).bindingBody!.instantiate1 first.1
        if branches body then
          let some second ← decide (body.getArg! 1) | return ← dropped
          return (fact, some #[first, second])
      return (fact, some #[first])
  let mut kept := #[]
  for (fact, decision) in decisions do
    match decision with
    | none => goal ← goal.clear fact
    | some decisions =>
        -- Each decision rewrites the instance to the branch it takes.
        let mut current := fact
        for (decided, holds) in decisions do
          let (branched, next) ← goal.withContext do
            let (proof, statement) ← unfoldingBranch (.fvar current) decided holds
            (← goal.assert `unfolded statement proof).intro1P
          goal ← next.clear current
          current := branched
        kept := kept.push current
  if kept.isEmpty then return some (some goal, 0)
  setGoals [goal]
  -- The conditions of the taken branch the context decides, such as the
  -- decrease a recursive application checks, are decided as well.
  let some simplified ← simpAt goal (← `(tactic| simp (disch := omega) only
      [vir_denote_norm, dif_pos, dif_neg, if_pos, if_neg, Int.add_sub_cancel, Int.reduceSub,
        Int.reduceAdd, Int.reduceToNat])) #[] kept false
    | return some (none, kept.size)
  if debug then
    simplified.withContext do
      for decl in ← getLCtx do
        if decl.userName == `unfolded && !previous.contains decl.fvarId then
          let shown := toString (← ppExpr (← instantiateMVars decl.type))
          IO.println s!"    unfolded (kept): {(shown.replace "\n" " ").take 300}"
  return some (some simplified, kept.size)

/-- The applications of recursive specification functions the context
fixes to values: a hypothesis equating an application to a value headed by
a constructor or an encoding, such as a loop invariant stating a native
value's encoding is the fold so far, reads the application as the value, in
the goal and in the unfolding instances, where its unfolding then meets the
value directly. -/
private def specValues (goal : MVarId) : TacticM (Option MVarId) := do
  -- The arguments of the instances of earlier rounds, as a substitution
  -- since has spelled them: the arithmetic of a position's successor, which
  -- a later value then matches syntactically.
  let unfoldings ← goal.withContext do
    return (← getLCtx).decls.toArray.filterMap fun
      | some decl => if !decl.isImplementationDetail && decl.userName == `unfolded
          then some decl.fvarId else none
      | none => none
  let goal ← if unfoldings.isEmpty then pure goal else do
    -- The simp context reads the main goal, which the asserts above replaced.
    setGoals [goal]
    match ← simpAt goal (← `(tactic| simp only [Int.add_sub_cancel, Int.reduceSub, Int.reduceAdd,
        Int.reduceToNat])) #[] unfoldings false with
    | some goal => pure goal
    | none => return none
  some <$> goal.withContext do
  let env ← getEnv
  let application (e : Lean.Expr) : Bool := (specUnfolding? env e).isSome && !e.hasLooseBVars
  let value (e : Lean.Expr) : Bool :=
    match e.getAppFn with
    | .const name _ => env.isConstructor name
    | _ => false
  let mut rules : Array Lean.Expr := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail || decl.userName == `unfolded then continue
    let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | continue
    if application rhs && value lhs then rules := rules.push (← mkEqSymm decl.toExpr)
    else if application lhs && value rhs then rules := rules.push decl.toExpr
  if rules.isEmpty then return goal
  let mut goal := goal
  for rule in rules do
    try
      let rewritten ← goal.rewrite (← goal.getType) rule
      goal ← goal.replaceTargetEq rewritten.eNew rewritten.eqProof
    catch _ => pure ()
    for other in ← getLCtx do
      if other.isImplementationDetail || other.userName != `unfolded then continue
      try
        goal ← goal.withContext do
          let rewritten ← goal.rewrite other.type rule
          pure (← goal.replaceLocalDecl other.fvarId rewritten.eNew rewritten.eqProof).mvarId
      catch _ => pure ()
  return goal

/-- A built vector equated to a term is its values equated to the term's. -/
private theorem specVector_mk_eq {α : Type} (values : Array α) (bounded) (other : VerexIR.SpecVector α) :
    (({ values, bounded } : VerexIR.SpecVector α) = other) = (values = other.values) :=
  propext ⟨fun same => same ▸ rfl, fun same => VerexIR.SpecVector.ext same⟩

/-- A term equated to a built vector is its values equated to the vector's. -/
private theorem eq_specVector_mk {α : Type} (values : Array α) (bounded) (other : VerexIR.SpecVector α) :
    (other = ({ values, bounded } : VerexIR.SpecVector α)) = (other.values = values) :=
  propext ⟨fun same => same ▸ rfl, fun same => VerexIR.SpecVector.ext same⟩

/-- The rules a bounded unfolding reads its result by: the normal form, the
ground evaluation of literal arithmetic and decided conditionals, and the
arithmetic a measure's descent states. -/
private def specEvaluationRules : TacticM (Lean.TSyntax `tactic) :=
  `(tactic| simp (disch := omega) only
      [vir_denote_norm, vir_denote_eval, dif_pos, dif_neg, if_pos, if_neg,
        Int.add_sub_cancel, Int.reduceSub, Int.reduceAdd, Int.reduceToNat,
        Array.map_push, Array.append_singleton, Option.getD_some, Option.some.injEq,
        specVector_mk_eq, eq_specVector_mk])

/-- An application of a recursive specification function at a literal
measure, evaluated: the unfolding theorem at the application, its body read
by the rules above so that the guards the arguments decide take their
branches, then the applications the branches taken still recurse into
evaluated and the body read again.  Reading before descending matters: a
branch a guard excludes applies the definition at an argument its own
descent never reaches, which has no smaller measure to evaluate by.  The
equation `application = value`, `none` where the measure is no literal,
where an application reached is not below it, or where the reading leaves
an application.  `memo` holds the equations taken, so an application met at
two levels is evaluated once. -/
private partial def evaluateSpecApplication
    (rules : Simp.Context × Simp.SimprocsArray × Simp.DischargeWrapper)
    (memo : IO.Ref (Std.HashMap Lean.Expr (Option Lean.Expr)))
    (application : Lean.Expr) : TacticM (Option Lean.Expr) := do
  if let some equation := (← memo.get)[application]? then return equation
  -- Taken as unevaluated while it is being evaluated: an application that
  -- reaches itself has no descent to evaluate by.
  memo.modify (·.insert application none)
  let equation ← evaluate
  memo.modify (·.insert application equation)
  return equation
where
  evaluate : TacticM (Option Lean.Expr) := do
    let env ← getEnv
    let some definition := specDefinitionAt? env application | return none
    let some measure ← specMeasure? application | return none
    let (proof, statement) ← specUnfoldingInstance definition.unfold application
    let some (_, _, body) := statement.eq? | return none
    let applies (e : Lean.Expr) : Bool :=
      !e.hasLooseBVars && (specDefinitionAt? env e).isSome
    let (context, simprocs, dischargeWrapper) := rules
    -- The body read with the equations of the applications descended into so
    -- far, which grow by a round.  A round either leaves no application or
    -- descends into one, and the measure bounds the levels below, so the
    -- rounds end.
    let mut descended : Array Lean.Expr := #[]
    let mut equations : Array Lean.Expr := #[]
    let mut read : Simp.Result := { expr := body }
    for _ in [0:measure + 2] do
      let mut theorems := context.simpTheorems
      unless equations.isEmpty do
        let mut taken : SimpTheorems := {}
        for equation in equations do taken ← taken.add (.other `unfolded) #[] equation
        theorems := theorems.push taken
      let stated ← Simp.mkContext context.config theorems context.congrTheorems
      (read, _) ← dischargeWrapper.with fun discharge? =>
        Lean.Meta.simp body stated simprocs discharge?
      let remaining := (sitesWhere applies #[read.expr]).filter (!descended.contains ·)
      if remaining.isEmpty then break
      for inner in remaining do
        if inner == application then return none
        let some smaller ← specMeasure? inner | return none
        unless smaller < measure do return none
        let some equation ← evaluateSpecApplication rules memo inner | return none
        descended := descended.push inner
        equations := equations.push equation
    -- A result still applying a definition is no value: the equation would
    -- restate the chain of instances the evaluation is there to replace.
    unless (sitesWhere applies #[read.expr]).isEmpty do return none
    let equation ← mkExpectedTypeHint proof statement
    match read.proof? with
    | some rewritten => return some (← mkEqTrans equation rewritten)
    | none => return some equation

/-- The applications of recursive specification functions a leaf holds at a
literal measure, each evaluated and stated as an `unfolded` equation
(`evaluateSpecApplication`): the value of such an application, which decides
the conditionals the levels above it state, where a single instance per
round leaves each level reading the one below.  The goal left, and whether
an application evaluated. -/
private def evaluateSpecApplications (goal : MVarId) : TacticM (MVarId × Bool) := do
  let env ← getEnv
  let applies (e : Lean.Expr) : Bool :=
    !e.hasLooseBVars && (specDefinitionAt? env e).isSome
  let applications ← goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    -- An application the leaf already states an equation of is left alone.
    let mut unfolded := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let type ← instantiateMVars decl.type
      terms := terms.push type
      if decl.userName == `unfolded then
        if let some (_, lhs, _) := type.eq? then unfolded := unfolded.push lhs
    (sitesWhere applies terms).filter (!unfolded.contains ·) |>.filterM fun application =>
      return (← specMeasure? application).isSome
  if applications.isEmpty then return (goal, false)
  let rules ← goal.withContext <| rulesSimpContext (← specEvaluationRules) #[]
  let memo ← IO.mkRef {}
  let debug := verex.solverDebug.get (← getOptions)
  let mut goal := goal
  let mut evaluated := false
  for application in applications do
    let some equation ← goal.withContext (evaluateSpecApplication rules memo application)
      | continue
    let statement ← goal.withContext (instantiateMVars (← inferType equation))
    (_, goal) ← (← goal.assert `unfolded statement equation).intro1P
    evaluated := true
    if debug then
      goal.withContext do
        let shown := toString (← ppExpr statement)
        IO.println s!"    evaluated: {(shown.replace "\n" " ").take 300}"
  return (goal, evaluated)

/-- Whether the unfolding instances close the goal: the goal's
applications read as their unfoldings, and the lookups the context fixes
read, normalize to a truth. A closer only: a goal the unfoldings do not
settle keeps its applications, which a quantified hypothesis may state. -/
private def closesByUnfoldings (goal : MVarId) : TacticM Bool := do
  let saved ← saveState
  let (unfoldings, lookups) ← goal.withContext do
    let mut unfoldings := #[]
    let mut lookups := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let type ← instantiateMVars decl.type
      if decl.userName == `unfolded then unfoldings := unfoldings.push decl.fvarId
      else if type.isAppOfArity ``Eq 3 && (type.getArg! 2).isAppOfArity ``Option.some 2 then
        lookups := lookups.push decl.fvarId
    pure (unfoldings, lookups)
  if unfoldings.isEmpty then return false
  setGoals [goal]
  let closed ← try
      let result ← simpAt goal (← `(tactic| simp (disch := omega) only
          [vir_denote_norm, dif_pos, dif_neg, if_pos, if_neg, Int.add_sub_cancel, Int.reduceSub,
            Int.reduceAdd, Int.reduceToNat, Array.map_push, Array.append_singleton,
            Option.getD_some, Option.some.injEq, specVector_mk_eq, eq_specVector_mk]))
        (unfoldings ++ lookups) #[] true
      pure result.isNone
    catch _ => pure false
  unless closed do saved.restore
  return closed

/-- The applications at a literal measure evaluated, then one round of
`unfoldSpecsOnce`, then the applications at closed arguments unfolded down
to their base cases: evaluation, whose guards the values decide. -/
elab "verex_unfold_specs" : tactic => focusGoal do
  let debug := verex.solverDebug.get (← getOptions)
  let (evaluatedGoal, evaluated) ← evaluateSpecApplications (← getMainGoal)
  replaceMainGoal [evaluatedGoal]
  let mut goal? : Option MVarId := some evaluatedGoal
  match ← unfoldSpecsOnce evaluatedGoal with
  | none => unless evaluated do
      throwError "the leaf applies no recursive specification function"
  | some (next, _) => goal? := next
  goal? ← try goal?.bindM specValues
    catch failure =>
      if debug then IO.println s!"    spec values failed: {← failure.toMessageData.toString}"
      throw failure
  if let some goal := goal? then
    let closed ← try closesByUnfoldings goal
      catch failure =>
        if debug then IO.println s!"    unfolding closer failed: {← failure.toMessageData.toString}"
        throw failure
    if closed then
      if debug then IO.println "    the unfoldings close the goal"
      replaceMainGoal []
      return
  for _ in [0:32] do
    let some goal := goal? | break
    let some (next, kept) ← unfoldSpecsOnce goal (groundOnly := true) | break
    goal? := next
    if kept == 0 then break
  replaceMainGoal goal?.toList

/-- The recursive specification functions a leaf applies, unfolded round by
round while a round keeps an instance, as at arguments the leaf fixes, each
application unfolds to further ones down to its base case. -/
elab "verex_specs_to_base" : tactic => focusGoal do
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
elab "verex_lookups_after_writes" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let goal ← getMainGoal
  let isWrite (e : Lean.Expr) := (e.find? fun e => e.isConstOf ``Array.setIfInBounds ||
    e.isConstOf ``Array.eraseIdx || e.isConstOf ``Array.insertIdx ||
    e.isConstOf ``VerexIR.Proofs.Denote.eraseBounded ||
    e.isConstOf ``Array.eraseIdxIfInBounds || e.isConstOf ``Array.insertIdxIfInBounds).isSome
  let (inTarget, written) ← goal.withContext do
    let inTarget := isWrite (← instantiateMVars (← goal.getType))
    let written ← (← goal.getNondepPropHyps).filterM fun fvarId => do
      return isWrite (← instantiateMVars (← fvarId.getType))
    pure (inTarget, written)
  if !inTarget && written.isEmpty then return
  -- Only where a write is read: the other hypotheses would be traversed
  -- for nothing. The side conditions go to the leaf's bounded discharger:
  -- normalization only asks for cheap side conditions.
  let result ← goal.withContext do
    let vectorSize := mkIdent `VerexIR.Proofs.Denote.vectorCertificateSize
    let (ctx, simprocs, _) ← rulesSimpContext (← `(tactic| simp only
      [Array.size_swapIfInBounds, $vectorSize:ident,
      VerexIR.Proofs.Denote.eraseBounded_values, Array.getElem?_setIfInBounds_ne, Array.getElem?_setIfInBounds_self_of_lt,
      Array.getElem_setIfInBounds_ne, Array.getElem_setIfInBounds_self,
      VerexIR.Proofs.Denote.eraseIdxIfInBounds_of_lt,
      VerexIR.Proofs.Denote.insertIdxIfInBounds_of_le, Array.size_eraseIdx, Array.size_insertIdx,
      VerexIR.Proofs.Denote.getElem?_eraseIdx_toNat_of_lt,
      VerexIR.Proofs.Denote.getElem?_eraseIdx_toNat_of_ge,
      VerexIR.Proofs.Denote.getElem?_insertIdx_toNat_of_lt,
      VerexIR.Proofs.Denote.getElem?_insertIdx_toNat_of_gt, Array.getElem?_insertIdx_self])) #[]
    let discharge ← goalDischarge goal
    let (sizeContext, sizeSimprocs, _) ← rulesSimpContext (← `(tactic| simp only
      [Array.size_swapIfInBounds, $vectorSize:ident])) #[]
    let dischargeSizes : Simp.Discharge := fun condition => do
      if let some proof ← discharge condition then return some proof
      -- A guarded write can ask for a size before that observation exists
      -- in the goal. Read it through certification and prior swaps using
      -- the same size rules as ordinary normalization, then use the leaf's
      -- existing bounds. Keep the vector itself in its canonical form.
      let (normal, _) ← simp condition sizeContext sizeSimprocs
      if normal.expr == condition then return none
      let some proof ← discharge normal.expr | return none
      return some (← normal.mkEqMPR proof)
    let (result, _) ← simpGoal goal ctx simprocs (some dischargeSizes)
      (simplifyTarget := inTarget) (fvarIdsToSimp := written)
    pure result
  match result with
  | none => replaceMainGoal []
  | some (_, next) => replaceMainGoal [next]

/-- The applications of a constant in a term, closed. -/
def closedApplications (e : Lean.Expr) (name : Name) (arity : Nat) : Array Lean.Expr :=
  let rec walk (e : Lean.Expr) : StateM (Std.HashSet Lean.Expr × Array Lean.Expr) Unit := do
    if (← get).1.contains e then return
    modify fun (seen, found) => (seen.insert e, found)
    if e.isAppOfArity name arity && !e.hasLooseBVars then
      modify fun (seen, found) => (seen, found.push e)
    match e with
    | .app f a => walk f; walk a
    | .lam _ d b _ | .forallE _ d b _ => walk d; walk b
    | .letE _ t v b _ => walk t; walk v; walk b
    | .mdata _ b | .proj _ _ b => walk b
    | _ => pure ()
  ((walk e).run ({}, #[])).2.2

/-- A map discipline observes its keys, independently of the reader chosen
for its values. Used only when a map observation asks for that view. -/
private theorem validity_sameKeys {K V K' V' V'' : Type}
    (keys : K → K') (values : V → V') (otherValues : V → V'')
    (discipline : VerexIR.SpecMap.Discipline K') (entries : VerexIR.SpecEntries (K × V)) :
    VerexIR.SpecMap.Valid discipline (VerexIR.SpecMap.ofEntries keys values entries) ↔
      VerexIR.SpecMap.Valid discipline (VerexIR.SpecMap.ofEntries keys otherValues entries) := by
  cases discipline <;>
    simp only [VerexIR.SpecMap.Valid, VerexIR.SpecMap.Distinct, VerexIR.SpecMap.Ascending,
      VerexIR.SpecMap.ofEntries, List.Nodup, List.pairwise_map]

/-- Index validity by entries, key reader and discipline. Instantiate it at
only the value views the leaf observes, leaving the canonical producer's
native-value predicate and the typed operation maps as they stand. -/
def observedMapValidities (observed : Array Lean.Expr)
    (valids : Array (Lean.Expr × Lean.Expr)) : MetaM (Array (Lean.Expr × Lean.Expr)) := do
  let mut index : Array ((Lean.Expr × Lean.Expr × Lean.Expr) × Lean.Expr × Lean.Expr) := #[]
  for (map, proof) in valids do
    unless map.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 do continue
    let type ← instantiateMVars (← inferType proof)
    unless type.isAppOfArity ``VerexIR.SpecMap.Valid 4 do continue
    index := index.push ((map.getArg! 6, map.getArg! 4, type.getArg! 2), map, proof)
  if index.isEmpty then return #[]
  let mut found : Array (Lean.Expr × Lean.Expr) := #[]
  let mut seen : Array (Lean.Expr × Lean.Expr) := #[]
  for map in observed do
    for ((entries, keys, discipline), source, proof) in index do
      unless map.getArg! 6 == entries && map.getArg! 4 == keys do continue
      if seen.contains (map, discipline) || index.any (fun (key, known, _) =>
          known == map && key.2.2 == discipline) then continue
      if let some viewed ← observing? do
        -- Keep the requested map's native type spellings in the emitted
        -- predicate; inferring them from a scalar reader can reduce them.
        let arguments := #[map.getArg! 0, map.getArg! 1, map.getArg! 2,
          source.getArg! 3, map.getArg! 3, keys, source.getArg! 5, map.getArg! 5,
          discipline, entries]
        let equivalence ← mkAppOptM ``validity_sameKeys (arguments.map some)
        mkAppM ``Iff.mp #[equivalence, proof] then
          found := found.push (map, viewed)
          seen := seen.push (map, discipline)
  return found

/-- The facts the positions a typed map (`SpecMap`) is read at give, as an
SMT solver's map axioms would: a key the map has sits at its rank, within
the map, and a position within the map holds a key of the map, at that
rank; the keys of a valid ordered map ascend with their positions; and a
hypothesis quantified over the positions of a map holds at each position the
context reads it at. A map whose entries are known (a literal, or one of
`known`, which the context defines as one) needs none: its reads evaluate. -/
private def mapPositionFacts (goal : MVarId) (known : Array Lean.Expr := #[]) :
    MetaM (Array (Lean.Expr × Lean.Expr)) :=
  goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    let mut present : Array (Lean.Expr × Lean.Expr × Lean.Expr) := #[]
    let mut valids : Array (Lean.Expr × Lean.Expr) := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      unless ← isProp ty do continue
      terms := terms.push ty
      if let some (_, lhs, rhs) := ty.eq? then
        if rhs.isConstOf ``Bool.true && lhs.isAppOfArity ``VerexIR.SpecMap.hasKey 5 then
          present := present.push (lhs.getArg! 3, lhs.getArg! 4, decl.toExpr)
      if ty.isAppOfArity ``VerexIR.SpecMap.Valid 4 then valids := valids.push (ty.getArg! 3, decl.toExpr)
    let mut observedMaps : Array Lean.Expr := #[]
    unless valids.isEmpty do
      for term in terms do
        for map in closedApplications term ``VerexIR.SpecMap.ofEntries 7 do
          unless observedMaps.contains map do observedMaps := observedMaps.push map
    -- The bulk roles' abort conditions the leaf has ruled out.
    let mut permits : Array (Lean.Expr × Lean.Expr) := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      if ty.isAppOfArity ``Not 1 then permits := permits.push (ty.appArg!, decl.toExpr)
    -- A construction's validity from its parts', as the normal form decides
    -- it (`valid_updateAll`, `valid_update`, `valid_remove`, `valid_takeEntries`,
    -- `valid_dropEntries`, `valid_nil`).
    let inner? (construction : Lean.Expr) : Option Lean.Expr :=
      if construction.isAppOfArity ``VerexIR.SpecMap.updateAll 7 ||
          construction.isAppOfArity ``VerexIR.SpecMap.update 7 ||
          construction.isAppOfArity ``VerexIR.SpecMap.remove 6 then some (construction.getArg! 4)
      else if construction.isAppOfArity ``VerexIR.SpecMap.takeEntries 4 ||
          construction.isAppOfArity ``VerexIR.SpecMap.dropEntries 4 then some (construction.getArg! 2)
      else none
    let validity? (valids : Array (Lean.Expr × Lean.Expr)) (map : Lean.Expr) :
        MetaM (Option Lean.Expr) := do
      let mut chain : Array Lean.Expr := #[]
      let mut current := map
      let mut base : Option Lean.Expr := none
      for _ in [0:16] do
        if let some (_, valid) := valids.find? (·.1 == current) then
          base := some valid
          break
        if current.isAppOfArity ``List.nil 1 then
          -- The empty map is valid under the discipline its construction names.
          if let some outer := chain.back? then
            if outer.getAppNumArgs == 7 || outer.getAppNumArgs == 6 then
              base := some (← mkAppOptM ``VerexIR.SpecMap.valid_nil
                #[outer.getArg! 0, outer.getArg! 1, outer.getArg! 3])
          break
        let some inner := inner? current | break
        chain := chain.push current
        current := inner
      let some valid := base | return none
      some <$> chain.foldrM (init := valid) fun (construction valid : Lean.Expr) => do
        let parts := construction.getAppArgs
        if construction.isAppOfArity ``VerexIR.SpecMap.takeEntries 4 ||
            construction.isAppOfArity ``VerexIR.SpecMap.dropEntries 4 then
          let lemma := if construction.isAppOfArity ``VerexIR.SpecMap.takeEntries 4 then
              ``VerexIR.SpecMap.valid_takeEntries
            else ``VerexIR.SpecMap.valid_dropEntries
          return ← mkAppOptM lemma #[parts[0]!, parts[1]!, none, parts[2]!, valid, parts[3]!]
        let lemma := if construction.isAppOfArity ``VerexIR.SpecMap.updateAll 7 then
            ``VerexIR.SpecMap.valid_updateAll
          else if construction.isAppOfArity ``VerexIR.SpecMap.update 7 then
            ``VerexIR.SpecMap.valid_update
          else ``VerexIR.SpecMap.valid_remove
        let explicit : Array Lean.Expr := (parts.extract 0 5).push valid ++ parts.extract 5 parts.size
        mkAppOptM lemma (explicit.map some)
    -- A map an equation relates to a valid one: the validity carries across,
    -- in either direction. The valid side is read through `validity?`, so an
    -- operation's result equated to a construction over a valid map — the
    -- form an intrinsic map operation hands back — carries it too.
    let mut equations : Array (Lean.Expr × Lean.Expr × Lean.Expr) := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      if let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? then
        equations := equations.push (lhs, rhs, decl.toExpr)
    let mut carried : Array (Lean.Expr × Lean.Expr) := #[]
    for _ in [0:4] do
      let observed ← observedMapValidities observedMaps valids
      valids := valids ++ observed
      carried := carried ++ observed
      let mut grew := !observed.isEmpty
      for (lhs, rhs, equation) in equations do
        for flipped in [false, true] do
          let (source, target) := if flipped then (rhs, lhs) else (lhs, rhs)
          if valids.any (·.1 == target) then continue
          let some valid ← validity? valids source | continue
          let validity ← inferType valid
          let motive ← withLocalDeclD `map (← inferType target) fun m =>
            mkLambdaFVars #[m] (mkAppN validity.getAppFn (validity.getAppArgs.set! 3 m))
          let oriented ← if flipped then mkEqSymm equation else pure equation
          let carriedValid ← mkEqMP (← mkCongrArg motive oriented) valid
          valids := valids.push (target, carriedValid)
          carried := carried.push (target, carriedValid)
          grew := true
      unless grew do break
    -- The instances the leaf's observations of a map carry: a native key
    -- type has no instance resolution finds, so a fact takes the leaf's.
    let mut decEqs : Array (Lean.Expr × Lean.Expr) := #[]
    let mut inhabiteds : Array (Lean.Expr × Lean.Expr) := #[]
    for term in terms do
      for observed in closedApplications term ``VerexIR.SpecMap.hasKey 5 do
        decEqs := decEqs.push (observed.getArg! 3, observed.getArg! 2)
      for observed in closedApplications term ``VerexIR.SpecMap.rank 5 do
        decEqs := decEqs.push (observed.getArg! 3, observed.getArg! 2)
      for observed in closedApplications term ``VerexIR.SpecMap.keyAt 5 do
        inhabiteds := inhabiteds.push (observed.getArg! 3, observed.getArg! 2)
    let instanceOf (known : Array (Lean.Expr × Lean.Expr)) (map : Lean.Expr) (cls : Name) :
        MetaM (Option Lean.Expr) := do
      if let some (_, inst) := known.find? (·.1 == map) then return some inst
      -- The map's type as stated: `SpecMap` unfolds at reducible transparency.
      let type ← instantiateMVars (← inferType map)
      let some (K, _) := (type.app2? ``VerexIR.SpecMap).orElse
          fun _ => (type.app1? ``List).bind (·.app2? ``Prod) | return none
      try pure (some (← synthInstance (← mkAppM cls #[K]))) catch _ => pure none
    let mut facts := #[]
    -- A fact's premises decided by the integers alone: one a premise of
    -- which fails is left out, those that hold are discharged.
    let push (facts : Array (Lean.Expr × Lean.Expr)) (fact : MetaM Lean.Expr) :
        MetaM (Array (Lean.Expr × Lean.Expr)) := do
      try
        let some fact ← decidePremises (← fact) | return facts
        return facts.push (← inferType fact, fact)
      catch _ => return facts
    -- A literal map's reads evaluate; its positions need no facts.
    let literal (map : Lean.Expr) : Bool := map.listLit?.isSome || known.contains map
    -- The validity carried across an equation is a fact in its own right: a
    -- map's data invariant at a loan's end asks for exactly this.
    for (map, valid) in carried do
      if literal map then continue
      facts ← push facts (pure valid)
    for (map, key, found) in present do
      if literal map then continue
      let some decEq ← instanceOf decEqs map ``DecidableEq | continue
      facts ← push facts (mkAppOptM ``VerexIR.SpecMap.rank_lt_size
        #[none, none, decEq, map, key, found])
      facts ← push facts (mkAppOptM ``VerexIR.SpecMap.rank_nonneg #[none, none, decEq, map, key])
      if let some inhabited ← instanceOf inhabiteds map ``Inhabited then
        facts ← push facts (mkAppOptM ``VerexIR.SpecMap.keyAt_rank
          #[none, none, decEq, inhabited, map, key, found])
    -- A key's neighbour the context states: none, the key is last (first);
    -- one, it is present a position on (back): `SpecMap.nextKey_none`,
    -- `nextKey_some`, `prevKey_none`, `prevKey_some`. The key's presence is a
    -- premise where the context does not state it.
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let term ← instantiateMVars decl.type
      let some (_, lhs, rhs) := term.eq? | continue
      let neighbour (e : Lean.Expr) := e.isAppOf ``VerexIR.SpecMap.nextKey? ||
        e.isAppOf ``VerexIR.SpecMap.prevKey?
      let flipped := !neighbour lhs
      let (call, result) := if flipped then (rhs, lhs) else (lhs, rhs)
      unless neighbour call && call.getAppNumArgs ≥ 3 do continue
      let next := call.isAppOf ``VerexIR.SpecMap.nextKey?
      unless result.isAppOf ``Option.none || result.isAppOf ``Option.some do continue
      let map := call.getArg! (call.getAppNumArgs - 2)
      let key := call.appArg!
      if literal map then continue
      let some (_, valid) := valids.find? (·.1 == map) | continue
      let proof ← if flipped then mkEqSymm decl.toExpr else pure decl.toExpr
      let neighbourFacts (present : Lean.Expr) : MetaM (Array Lean.Expr) := do
        if result.isAppOf ``Option.none then
          return #[← mkAppM (if next then ``VerexIR.SpecMap.nextKey_none
            else ``VerexIR.SpecMap.prevKey_none) #[valid, present, proof]]
        let found ← mkAppM (if next then ``VerexIR.SpecMap.nextKey_some
          else ``VerexIR.SpecMap.prevKey_some) #[valid, present, proof]
        return #[← mkAppM ``And.left #[found], ← mkAppM ``And.right #[found]]
      let stated := present.find? fun (other, presentKey, _) => other == map && presentKey == key
      let derived ← try
          match stated with
          | some (_, _, found) => neighbourFacts found
          | none =>
              let presence ← mkEq (← mkAppM ``VerexIR.SpecMap.hasKey #[map, key]) (mkConst ``Bool.true)
              withLocalDeclD `present presence fun premise => do
                (← neighbourFacts premise).mapM (mkLambdaFVars #[premise])
        catch _ => pure #[]
      for fact in derived do facts ← push facts (pure fact)
    -- A typed map's size is a count; a table's map fits its carrier.
    let mut sized : Array Lean.Expr := #[]
    for term in terms do
      for size in closedApplications term ``VerexIR.SpecMap.size 3 do
        if literal (size.getArg! 2) then continue
        unless sized.contains size do
          sized := sized.push size
          let map := size.getArg! 2
          facts ← push facts (mkAppM ``VerexIR.SpecMap.size_nonneg #[map])
          if map.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 then
            facts ← push facts (do mkAppM ``And.right #[← mkAppM ``VerexIR.SpecMap.size_ofEntries_bounds
              #[map.getArg! 4, map.getArg! 5, map.getArg! 6]])
    -- Several entries set keep every entry and add at most one per key.
    let mut bulk : Array Lean.Expr := #[]
    for term in terms do
      for updated in closedApplications term ``VerexIR.SpecMap.updateAll 7 do
        unless bulk.contains updated do
          bulk := bulk.push updated
          let bounds ← mkAppOptM ``VerexIR.SpecMap.size_updateAll_bounds
            (updated.getAppArgs.map some)
          facts ← push facts (mkAppM ``And.left #[bounds])
          facts ← push facts (mkAppM ``And.right #[bounds])
          -- The size a permitted bulk role fixes: one entry per key added.
          let (map, keys, values) := (updated.getArg! 4, updated.getArg! 5, updated.getArg! 6)
          for (condition, permit) in permits do
            if condition.isAppOfArity ``VerexIR.SpecMap.AbortsAddAll 6 &&
                condition.getArg! 3 == map && condition.getArg! 4 == keys &&
                condition.getArg! 5 == values then
              for (other, valid) in valids do
                if other == map then
                  facts ← push facts (mkAppOptM ``VerexIR.SpecMap.size_updateAll_add_all
                    #[none, none, none, none, none, valid, keys, values, permit])
            if condition.isAppOfArity ``VerexIR.SpecMap.AbortsNewFrom 4 &&
                condition.getArg! 2 == keys && condition.getArg! 3 == values &&
                map.isAppOfArity ``List.nil 1 then
              facts ← push facts (mkAppOptM ``VerexIR.SpecMap.size_updateAll_new_from
                #[none, none, none, updated.getArg! 3, keys, values, permit])
          -- An appended map: bounds by its size, and at least its entries.
          let (keys, values) := (updated.getArg! 5, updated.getArg! 6)
          if keys.isAppOfArity ``VerexIR.SpecMap.keys 3 &&
              values.isAppOfArity ``VerexIR.SpecMap.values 3 && keys.getArg! 2 == values.getArg! 2 then
            let other := keys.getArg! 2
            let bounds ← mkAppOptM ``VerexIR.SpecMap.size_updateAll_append_bounds
              #[updated.getArg! 0, updated.getArg! 1, updated.getArg! 2, updated.getArg! 3,
                updated.getArg! 4, other]
            facts ← push facts (mkAppM ``And.left #[bounds])
            facts ← push facts (mkAppM ``And.right #[bounds])
            for (condition, permit) in permits do
              if condition.isAppOfArity ``VerexIR.SpecMap.AbortsAppendDisjoint 5 &&
                  condition.getArg! 3 == map && condition.getArg! 4 == other then
                for (left, valid) in valids do
                  for (right, validOther) in valids do
                    if left == map && right == other then
                      facts ← push facts (mkAppOptM
                        ``VerexIR.SpecMap.size_updateAll_append_disjoint
                        #[none, none, none, none, none, none, valid, validOther, permit])
            for (map, valid) in valids do
              if map == other then
                facts ← push facts (mkAppOptM ``VerexIR.SpecMap.size_other_le_updateAll_append
                  #[none, none, none, none, updated.getArg! 4, none, valid])
    let mut reads : Array (Lean.Expr × Lean.Expr) := #[]
    for term in terms do
      for read in closedApplications term ``VerexIR.SpecMap.keyAt 5 do
        let position := (read.getArg! 3, read.getArg! 4)
        unless reads.contains position do reads := reads.push position
    for (map, index) in reads do
      if literal map then continue
      -- A position the context bounds below; its bound above is a premise
      -- where the context does not decide it, as the map's size often
      -- follows from facts the normal form derives later.
      let some low ← omegaProof? goal (← mkAppM ``LE.le #[mkIntLit 0, index]) | continue
      let high ← omegaProof? goal
          (← mkAppM ``LT.lt #[index, ← mkAppM ``VerexIR.SpecMap.size #[map]])
      let some decEq ← instanceOf decEqs map ``DecidableEq | continue
      let some inhabited ← instanceOf inhabiteds map ``Inhabited | continue
      let bounds : Array (Option Lean.Expr) := (#[low] ++ high.toArray).map some
      facts ← push facts (mkAppOptM ``VerexIR.SpecMap.hasKey_keyAt
        ((#[none, none, some decEq, some inhabited, some map, some index] : Array (Option Lean.Expr)) ++
          bounds))
      if let some valid ← validity? valids map then
        facts ← push facts (mkAppOptM ``VerexIR.SpecMap.rank_keyAt
          ((#[none, none, some decEq, some inhabited, none, some map, some valid, some index] :
            Array (Option Lean.Expr)) ++ bounds))
    -- The keys of a valid ordered map of integers ascend with their
    -- positions, and a hypothesis quantified over the positions holds at
    -- each position the context reads.
    let mut ordered : Array (Lean.Expr × Lean.Expr) := #[]
    for (map, valid) in valids do
      let ty ← instantiateMVars (← inferType valid)
      let discipline := ty.getArg! 2
      if discipline.isAppOfArity ``VerexIR.SpecMap.Discipline.ordered 2 &&
          discipline.appArg!.isConstOf ``VerexIR.SpecMap.intOrder && !literal map then
        ordered := ordered.push (map, valid)
    for (map, valid) in ordered do
      let mut positions : Array Lean.Expr :=
        reads.filterMap fun (other, index) => if other == map then some index else none
      for (other, key, _) in present do
        if other == map then
          if let some rank ← observing? (mkAppM ``VerexIR.SpecMap.rank #[map, key]) then
            unless positions.contains rank do positions := positions.push rank
      if positions.size ≥ 2 && positions.size ≤ 6 then
        for first in positions do
          for second in positions do
            if first == second then continue
            facts ← push facts (mkAppOptM ``VerexIR.SpecMap.keyAt_lt_of_lt
              #[none, none, valid, first, second])
            facts ← push facts (mkAppM ``VerexIR.SpecMap.keyAt_congr #[map, first, second])
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let ty ← instantiateMVars decl.type
        let .forallE _ binder body _ := ty | continue
        unless binder.isConstOf ``Int &&
          (body.find? (·.isConstOf ``VerexIR.SpecMap.keyAt)).isSome do continue
        for position in positions do
          facts ← push facts (pure (mkApp decl.toExpr position))
    return facts

/-- Read the keys of typed maps at positions within them (`mapPositionFacts`).
Runs only where the context reads a key at a position, a key's position, or
a map's size; omega decides the positions. -/
elab "verex_map_positions" : tactic => focusGoal do
  if (← getGoals).isEmpty then return
  let reads (e : Lean.Expr) := (e.find? fun sub =>
    sub.isConstOf ``VerexIR.SpecMap.keyAt || sub.isConstOf ``VerexIR.SpecMap.rank ||
      sub.isConstOf ``VerexIR.SpecMap.size || sub.isConstOf ``VerexIR.SpecMap.nextKey? ||
      sub.isConstOf ``VerexIR.SpecMap.prevKey?).isSome
  let read ← (← getMainGoal).withContext do
    if reads (← instantiateMVars (← (← getMainGoal).getType)) then return true
    (← getLCtx).anyM fun decl => do
      return !decl.isImplementationDetail && reads (← instantiateMVars decl.type)
  unless read do return
  -- The facts the positions are decided by, out of their obligations.
  if verex.keepObligationMarkers.get (← getOptions) then
    let goal ← getMainGoal
    let hypotheses ← goal.withContext goal.getNondepPropHyps
    simpMainAt (← `(tactic| simp only [VerexIR.Proofs.Obligation_eq])) #[] hypotheses false
  else
    evalTactic (← `(tactic| try simp only [VerexIR.Proofs.Obligation_eq] at *))
  if (← getGoals).isEmpty then return
  -- A constructor equality can make these reads computational. Use only
  -- literal construction equations here, before generating rank/order facts;
  -- those facts are unnecessary (and costly) for a map whose entries are known.
  let (concrete, known) ← (← getMainGoal).withContext do
    let mut equations := #[]
    let mut known := #[]
    for fvarId in ← (← getMainGoal).getNondepPropHyps do
      let type ← instantiateMVars (← fvarId.getType)
      let some (_, lhs, rhs) := type.eq? | continue
      -- A typed map that is a literal list, or a table carrier holding one.
      let literal := rhs.listLit?.isSome ||
        (rhs.isAppOfArity ``VerexIR.SpecEntries.mk 3 && (rhs.getArg! 1).listLit?.isSome)
      if literal then
        equations := equations.push fvarId
        known := known.push lhs
    return (equations, known)
  unless concrete.isEmpty do
    let goal ← getMainGoal
    let hypotheses ← goal.withContext do (← goal.getNondepPropHyps).filterM fun fvarId => do
      let type ← instantiateMVars (← fvarId.getType)
      return !type.isForall && !concrete.contains fvarId && (type.find? fun e =>
        e.isAppOf ``VerexIR.SpecMap.keyAt || e.isAppOf ``VerexIR.SpecMap.rank ||
        e.isAppOf ``VerexIR.SpecMap.valueAt || e.isAppOf ``VerexIR.SpecMap.size ||
        e.isAppOf ``VerexIR.SpecMap.hasKey || e.isAppOf ``VerexIR.SpecMap.update ||
        e.isAppOf ``VerexIR.SpecMap.remove).isSome
    -- With evaluation: a literal map's size is a numeral, which the
    -- positions read at it are decided by.
    simpMainAt (← `(tactic| simp only [vir_denote_norm, vir_denote_eval, Prod.fst, Prod.snd]))
      concrete hypotheses true
    if (← getGoals).isEmpty then return
  let mut goal ← getMainGoal
  let debug := verex.solverDebug.get (← getOptions)
  let mark ← IO.getNumHeartbeats
  -- A fact the context states is not asserted again.
  let mut stated : Std.HashSet Lean.Expr ← goal.withContext do
    (← goal.getNondepPropHyps).foldlM (init := {}) fun stated fvarId => do
      return stated.insert (← instantiateMVars (← fvarId.getType))
  let facts ← mapPositionFacts goal known
  for (statement, proof) in facts do
    if stated.contains statement then continue
    stated := stated.insert statement
    goal ← (← goal.assert `mapFact statement proof).intro1P <&> (·.2)
  replaceMainGoal [goal]
  if debug then
    IO.println s!"    positions: {facts.size} facts in {((← IO.getNumHeartbeats) - mark) / 1000}k (concrete {concrete.size})"
/-- A certified integer is its value. -/
theorem specInt_eq_iff_val {width : VerexIR.IntWidth} {signed : Bool}
    (left right : VerexIR.SpecInt width signed) : left = right ↔ left.val = right.val :=
  ⟨fun same => same ▸ rfl, VerexIR.SpecInt.ext⟩

/-- An equation of native values at their reduced carrier: of rows, the
equations of their components; of certified integers, of their values; of
units, true. -/
private def nativeComponentStep : Simp.Simproc := fun e => withDefault do
  -- At default transparency: a value's carrier is a row (`HList`), which
  -- reduces to its pairs only there, and the projections are typed at it.
  let_expr Eq type left right := e | return .continue
  -- Tables, as their typed maps (`NTy.table_eq_iff`).
  let spelled := type.consumeMData
  if spelled.isAppOfArity ``VerexIR.Proofs.Denote.NTy.carrier 2 &&
      (spelled.getArg! 1).isAppOfArity ``VerexIR.Proofs.Denote.NTy.table 2 then
    let τ := spelled.getArg! 1
    let iff ← mkAppOptM ``VerexIR.Proofs.Denote.NativeMaps.NTy.table_eq_iff
      #[spelled.getArg! 0, τ.getArg! 0, τ.getArg! 1, left, right]
    let some (_, typed) := (← inferType iff).iff? | return .continue
    return .visit { expr := typed, proof? := some (← mkAppM ``propext #[iff]) }
  let carrier ← whnfD type
  if carrier.isAppOfArity ``Prod 2 then
    let components ← mkAppM ``And
      #[← mkEq (← mkAppM ``Prod.fst #[left]) (← mkAppM ``Prod.fst #[right]),
        ← mkEq (← mkAppM ``Prod.snd #[left]) (← mkAppM ``Prod.snd #[right])]
    let proof ← mkAppM ``propext #[← mkAppOptM ``Prod.ext_iff
      #[carrier.getArg! 0, carrier.getArg! 1, left, right]]
    return .visit { expr := components,
                    proof? := some (← mkExpectedTypeHint proof (← mkEq e components)) }
  if carrier.isAppOfArity ``VerexIR.SpecInt 2 then
    let values ← mkEq (← mkAppM ``VerexIR.SpecInt.val #[left]) (← mkAppM ``VerexIR.SpecInt.val #[right])
    let proof ← mkAppM ``propext #[← mkAppM ``specInt_eq_iff_val #[left, right]]
    return .visit { expr := values, proof? := some proof }
  if carrier.isConstOf ``Unit || carrier.isAppOf ``PUnit then
    let same ← mkAppOptM ``Subsingleton.elim #[carrier, none, left, right]
    return .done { expr := mkConst ``True, proof? := some (← mkAppM ``eq_true #[same]) }
  -- A struct's or a variant's value is its row under the data invariants
  -- (`NTy.carrier`, `doc/designs/data-invariants.md`): two values
  -- are equal exactly when their rows are, the certificate being a
  -- proposition.  The row is a plain row, which this simproc then splits
  -- component by component as it splits a tuple's.
  if carrier.isAppOfArity ``Subtype 2 &&
      spelled.isAppOfArity ``VerexIR.Proofs.Denote.NTy.carrier 2 then
    -- The type the carrier is of, which a definition or a frame's
    -- resolution may stand in front of.
    let τ ← whnfD (spelled.getArg! 1)
    if τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.struct 3 ||
        τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.enum 5 then
      let rows ← mkEq (← mkAppM ``Subtype.val #[left]) (← mkAppM ``Subtype.val #[right])
      let proof ← mkAppM ``propext #[← mkAppOptM ``Subtype.ext_iff
        #[carrier.getArg! 0, carrier.getArg! 1, left, right]]
      return .visit { expr := rows,
                      proof? := some (← mkExpectedTypeHint proof (← mkEq e rows)) }
  return .continue

/-- The reads of the components of a native value at its reduced carrier
that a leaf's deciders compare: through nested pairs to each component, a
certified integer by its value; none for a unit component. -/
private partial def nativeComponentReads (carrier : Lean.Expr) (read : Lean.Expr → MetaM Lean.Expr) :
    MetaM (Array (Lean.Expr → MetaM Lean.Expr)) := do
  let carrier ← whnfD carrier
  if carrier.isAppOfArity ``Prod 2 then
    let first ← nativeComponentReads (carrier.getArg! 0) fun value => do
      mkAppM ``Prod.fst #[← read value]
    let second ← nativeComponentReads (carrier.getArg! 1) fun value => do
      mkAppM ``Prod.snd #[← read value]
    return first ++ second
  if carrier.isAppOfArity ``VerexIR.SpecInt 2 then
    return #[fun value => do mkAppM ``VerexIR.SpecInt.val #[← read value]]
  -- A certified value reads through its row (`NTy.carrier` at a struct or a
  -- variant): the certificate is a proposition and nothing reads it.
  if carrier.isAppOfArity ``Subtype 2 then
    return ← nativeComponentReads (carrier.getArg! 0) fun value => do
      mkAppM ``Subtype.val #[← read value]
  if carrier.isConstOf ``Unit || carrier.isAppOf ``PUnit then return #[]
  return #[read]

/-- An equation of native rows or certified integers is stated at their
components and values: the goal is rewritten; for a hypothesis neither of whose sides
is a local the other does not mention, the component equations the context
does not state yet are added beside it (`nativeComponentReads`), so that
the hypothesis keeps its place for the steps reading it. -/
elab "verex_native_components" : tactic => do
  let mut goal ← getMainGoal
  let ctx ← Simp.mkContext {} (simpTheorems := #[]) (congrTheorems := ← getSimpCongrTheorems)
  let substitutable (side other : Lean.Expr) : Bool := match side with
    | .fvar x => !other.containsFVar x
    | _ => false
  for hypothesis in ← goal.getNondepPropHyps do
    let type ← goal.withContext do instantiateMVars (← hypothesis.getType)
    let some (carrier, left, right) := type.eq? | continue
    if substitutable left right || substitutable right left then continue
    let spelled := carrier.consumeMData
    unless spelled.isAppOf ``VerexIR.Proofs.Denote.NTy.carrier ||
        spelled.isAppOf ``VerexIR.Proofs.Denote.HList do continue
    let reads ← goal.withContext (nativeComponentReads carrier pure)
    if reads.size == 1 then
      -- One component read as itself: the hypothesis already.
      let same ← goal.withContext do return (← reads[0]! left) == left
      if same then continue
    for read in reads do
      let (equation, proof) ← goal.withContext do
        let equation ← mkEq (← read left) (← read right)
        let projection ← withLocalDeclD `value carrier fun value => do
          mkLambdaFVars #[value] (← read value)
        pure (equation, ← mkCongrArg projection (Lean.mkFVar hypothesis))
      let stated ← goal.withContext do
        (← getLCtx).anyM fun decl => do
          if decl.isImplementationDetail then return false
          return (← instantiateMVars decl.type) == equation
      if stated then continue
      goal := (← (← goal.assert `component equation proof).intro1P).2
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let (result, _) ← goal.withContext (Simp.main target ctx (methods := { post := nativeComponentStep }))
  if result.expr == target then replaceMainGoal [goal]
  else replaceMainGoal [← applySimpResultToTarget goal target result]

/-- The component equations of the context (`verex_native_components`),
split, with the locals they fix substituted. -/
elab "verex_native_decompose" : tactic => do
  evalTactic (← `(tactic| (
    verex_native_components
    all_goals (try verex_split_hypotheses)
    all_goals (try verex_subst_vars))))

/-- Maps written back into tables read back as themselves
(`SpecMap.ofEntries_writeBack`), where their length and the read-back of
their entries are decided: a specification's map value, read. -/
syntax "verex_written_back" : tactic

/-- One saturation round: bound the integers, destructure the hypotheses,
substitute witnesses, and rewrite with every hypothesis. -/
macro "verex_saturate_round" : tactic => `(tactic|
  verex_bounded (
    all_goals verex_split_hypotheses
    all_goals verex_subst_vars
    all_goals (try verex_simp_all [vir_denote_norm])
    -- A goal about vectors is decided at their values; a hypothesis keeps
    -- the vector, which substitution reads.
    all_goals (try simp only [VerexIR.SpecVector.eq_iff_values, vir_denote_norm])
    all_goals verex_native_components
    all_goals verex_lookups_after_writes
    all_goals verex_map_positions))

/-- Saturate a leaf's context: rounds of rewriting while new hypotheses
appear, after the round the leaf has already run, at most four. A round
that leaves the goals as they were has reached the fixed point, and a
further round would repeat it. -/
elab "verex_saturate" : tactic => focusGoal do
  for _ in [0:4] do
    let before ← (← getGoals).mapM fun goal => goalContent goal
    evalTactic (← `(tactic| verex_saturate_round))
    if (← (← getGoals).mapM fun goal => goalContent goal) == before then return

/-- `decide` on a closed goal only: on an open one the kernel evaluates
the instance for as long as the goal is large, then fails anyway. The
kernel also evaluates what elaboration leaves stuck, such as string
comparisons and well-founded definitions like the structural order. -/
elab "verex_decide" : tactic => focusGoal do
  let target ← instantiateMVars (← (← getMainGoal).getType)
  if target.hasFVar then throwError "the goal is not closed"
  evalTactic (← `(tactic| first | decide | decide +kernel))

/-- The definitions of typed maps the context fixes to literals through each
other: `m = [(1, 2)]`, then `m' = update … m 2 3`. Rewriting by them takes a
read of the last to a literal the ground evaluation decides. -/
private def groundMapDefinitions : MetaM (Array FVarId) := do
  let mut definitions : Array (FVarId × FVarId × Lean.Expr) := #[]
  for decl in ← getLCtx do
    -- A definition the preparation hid (`m_def`) defines its map as well.
    if decl.isImplementationDetail && !(decl.userName.toString.endsWith "_def") then continue
    let some (type, left, right) := (← instantiateMVars decl.type).eq? | continue
    -- A map, named or a carrier's typed map, or a table carrier itself.
    let isMap := type.consumeMData.isAppOfArity ``VerexIR.SpecMap 2
    let isCarrier ← if isMap then pure false else do
      pure ((← whnfR type).isAppOfArity ``VerexIR.SpecEntries 1)
    unless isMap || isCarrier do continue
    let map := if left.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 then left.appArg! else left
    let .fvar var := map | continue
    if right.hasMVar || right.hasAnyFVar (· == var) then continue
    definitions := definitions.push (decl.fvarId, var, right)
  -- The variables a definition reads, its proofs aside: a literal's bound
  -- may cite a hypothesis.
  let valueFVars (e : Lean.Expr) : MetaM (Array FVarId) := do
    let found ← IO.mkRef (#[] : Array FVarId)
    Lean.Meta.forEachExpr e fun e => do
      if let .fvar x := e then
        unless ← isProof e do found.modify fun found => if found.contains x then found else found.push x
    found.get
  -- A map is ground when the maps its definition reads are, down to a
  -- literal; the keys and values written may be symbolic.
  let mapVars := definitions.foldl (fun vars (_, var, _) => vars.insert var) ({} : FVarIdSet)
  let mut ground : FVarIdSet := {}
  let mut grown := true
  while grown do
    grown := false
    for (_, var, rhs) in definitions do
      if ground.contains var then continue
      if (← valueFVars rhs).all fun x => !mapVars.contains x || ground.contains x then
        ground := ground.insert var
        grown := true
  return definitions.filterMap fun (hyp, var, _) => if ground.contains var then some hyp else none

/-- Whether `verex_ground_arithmetic` may apply, before the literals it reads
are reduced: an integer equation of a variable's term to a closed one, or a
map the context defines by a literal (`groundMapDefinitions`). -/
elab "verex_ground_inputs" : tactic => focusGoal do
  let applies ← (← getMainGoal).withContext do
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some (type, left, right) := (← instantiateMVars decl.type).eq? | continue
      unless type.isConstOf ``Int || type.isConstOf ``Nat do continue
      let closed (e : Lean.Expr) := !e.hasFVar && !e.hasMVar
      if (left.hasFVar && closed right) || (right.hasFVar && closed left) then return true
    return !(← groundMapDefinitions).isEmpty
  unless applies do throwError "no scalar input is fixed to a literal"

/-- Evaluate arithmetic whose inputs the context fixes to literals, and the
reads of typed maps it fixes to literals (`groundMapDefinitions`).
Substituting those equalities in the target before omega avoids introducing
a quotient and remainder constraint for every nested modular conversion.
Only literal equalities are rewrite rules; unrelated hypotheses are not
simplified. -/
private def groundArithmetic : TacticM Unit := do
  let goal ← getMainGoal
  let maps ← goal.withContext do
    let maps ← groundMapDefinitions
    -- Only a goal reading one of the maps is decided by their literals: by
    -- its variables, or through the definitions the context states of
    -- them (`x = e` or `x.val = e`, an equation of a variable or of a
    -- projection of it), what the defining expressions read.
    let target ← instantiateMVars (← goal.getType)
    let goalVars := (collectFVars {} target).fvarSet
    let env ← getEnv
    let defined : Lean.Expr → Option FVarId := fun side =>
      let side := side.consumeMData
      let core := match side with
        | .proj _ _ inner => inner
        | _ =>
          match side.getAppFn with
          | .const name _ => if env.isProjectionFn name && side.getAppNumArgs > 0 then side.appArg! else side
          | _ => side
      core.fvarId?
    let mut reading := goalVars
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some (_, left, right) := (← instantiateMVars decl.type).eq? | continue
      if (defined left).any goalVars.contains then
        reading := (collectFVars { fvarSet := reading } right).fvarSet
      if (defined right).any goalVars.contains then
        reading := (collectFVars { fvarSet := reading } left).fvarSet
    let vars ← maps.mapM fun hyp => do
      let some (_, left, _) := (← instantiateMVars (← hyp.getType)).eq? | return hyp
      pure (if left.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 then left.appArg!.fvarId! else left.fvarId!)
    pure (if vars.any reading.contains then maps else #[])
  let candidates ← goal.withContext do
    let mut rules := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some (type, left, right) := (← instantiateMVars decl.type).eq? | continue
      unless type.isConstOf ``Int || type.isConstOf ``Nat do continue
      let literal (e : Lean.Expr) := e.int?.isSome || e.nat?.isSome
      if left.hasFVar && literal right then
        rules := rules.push (decl.fvarId, false)
      else if right.hasFVar && literal left then
        rules := rules.push (decl.fvarId, true)
    pure rules
  if candidates.isEmpty && maps.isEmpty then throwError "no scalar input is fixed to a literal"
  let mut goal := goal
  let mut rules := #[]
  for (id, reverse) in candidates do
    if reverse then
      let (id, next) ← goal.withContext do
        let proof ← mkEqSymm (mkFVar id)
        (← goal.assert `literalValue (← inferType proof) proof).intro1P
      goal := next
      rules := rules.push id
    else rules := rules.push id
  replaceMainGoal [goal]
  if maps.isEmpty then
    simpMainAt (← `(tactic| simp only [VerexIR.Proofs.Obligation_eq])) rules #[] true
  else
    -- The reads of the literal maps, in the goal and the facts, evaluate;
    -- the leaf is then arithmetic over the values read.
    let targets ← goal.withContext do
      let vars ← maps.mapM fun hyp => do
        let some (_, left, _) := (← instantiateMVars (← hyp.getType)).eq? | return hyp
        pure (if left.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 then left.appArg!.fvarId! else left.fvarId!)
      (← goal.getNondepPropHyps).filterM fun hyp => do
        if maps.contains hyp then return false
        let type ← instantiateMVars (← hyp.getType)
        return vars.any fun var => type.hasAnyFVar (· == var)
    -- The lemmas of the reads of a literal map and the equality of literal
    -- values; the whole normal form would cost a leaf's budget here.
    let pairs := mkIdent `VerexIR.Proofs.Denote.pairInjectionRows
    simpMainAt (← `(tactic| simp only [VerexIR.Proofs.Obligation_eq, vir_denote_eval,
      VerexIR.SpecMap.valueAt_update, VerexIR.SpecMap.hasKey_update,
      VerexIR.SpecMap.size_update_sequence, VerexIR.SpecMap.valueAt_cons, VerexIR.SpecMap.valueAt_nil,
      VerexIR.SpecMap.hasKey_cons, VerexIR.SpecMap.hasKey_nil, VerexIR.SpecMap.size_cons,
      VerexIR.SpecMap.size_nil, VerexIR.SpecMap.ofEntries_mk_nil, VerexIR.SpecMap.ofEntries_mk_cons,
      $pairs:ident, Prod.mk.injEq, VerexIR.SpecVector.mk.injEq, VerexIR.SpecInt.mk.injEq,
      VerexIR.Proofs.Denote.NativeMaps.toArray_eq_iff, VerexIR.Proofs.Denote.NativeMaps.array_mk_eq_toArray,
      List.cons.injEq, VerexIR.Proofs.Denote.HList.cons, VerexIR.Proofs.Denote.HList.nil,
      Int.reduceEq, Int.reduceNe, decide_eq_true_eq, decide_eq_false_iff_not, decide_true, decide_false,
      Bool.or_eq_true, Bool.or_false, Bool.false_or, Bool.or_true, Bool.true_or, Bool.false_eq_true,
      Bool.true_eq_false, ite_true, ite_false, and_true, true_and, and_false, false_and,
      eq_self_iff_true, not_false_eq_true, not_true_eq_false]))
      (rules ++ maps) targets true
    if verex.solverDebug.get (← getOptions) then
      for open_ in ← getGoals do
        open_.withContext do
          IO.println s!"    ground maps ({maps.size} definitions, {targets.size} facts) ⊢ {(toString (← ppExpr (← instantiateMVars (← open_.getType)))).replace "\n" " " |>.take 400}"
          for hyp in targets do
            if (← getLCtx).contains hyp then
              IO.println s!"      {← hyp.getUserName} : {(toString (← ppExpr (← instantiateMVars (← hyp.getType)))).replace "\n" " " |>.take 300}"
    evalTactic (← `(tactic| all_goals first
      | verex_assumption
      | (verex_bounds; verex_omega)
      | verex_decide
      -- Native values read: their components' equations, normal under the
      -- leaf's bounds (an integer cut to its width where it fits).
      | (verex_native_components
         verex_normalize_context
         first | done | verex_assumption | (verex_bounds; verex_omega))
      -- A conditional on a membership the context decides.
      | (simp (config := { failIfUnchanged := false }) only [*, ite_true, ite_false,
          Bool.false_eq_true, Bool.true_eq_false, vir_denote_eval]
         first | done | verex_assumption | (verex_bounds; verex_omega) | verex_decide)))
  evalTactic (← `(tactic| all_goals verex_decide))

/-- What the ground evaluation reads of a goal: its content and the map
definitions the preparation hid (`groundMapDefinitions`). -/
private def groundContent (goal : MVarId) : MetaM (Lean.Expr × Array Lean.Expr) := goal.withContext do
  let (target, hypotheses) ← goalContent goal
  let mut hidden := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail && decl.userName.toString.endsWith "_def" then
      hidden := hidden.push (← instantiateMVars decl.type)
  return (target, hypotheses ++ hidden.qsort (·.lt ·))

/-- The heartbeats the enclosing budget leaves, `none` when unbounded. -/
private def heartbeatsLeft : CoreM (Option Nat) := do
  let context ← readThe Core.Context
  if context.maxHeartbeats == 0 then return none
  return some (context.initHeartbeats + context.maxHeartbeats - (← IO.getNumHeartbeats))

/-- The ground evaluation (`groundArithmetic`), not repeated on a goal it
failed on. -/
elab "verex_ground_arithmetic" : tactic => focusGoal do
  let key ← groundContent (← getMainGoal)
  let left ← heartbeatsLeft
  let failures := (← targetState).groundFailures
  -- A goal it failed on with at least as many heartbeats left fails again.
  if let some before := (← failures.get)[key]? then
    let noMore := match left, before with
      | _, none => true
      | none, some _ => false
      | some now, some before => now ≤ before
    if noMore then throwError "the ground evaluation failed on this goal before"
  try groundArithmetic
  catch failure =>
    -- When ground evaluation proves the goal reduces to False under the
    -- substituted literals, the path is arithmetically infeasible: omega
    -- can close any goal from a contradictory arithmetic context (e.g. the
    -- original context holds `value.val = 2` and `4 ≤ value.val`, which
    -- together imply `4 ≤ 2`).  Try before recording the failure.
    -- The state is restored by `try groundArithmetic`, so the goal here is
    -- the original goal and omega operates on the original context.
    try evalTactic (← `(tactic| omega))
    catch _ => pure ()
    if (← getGoals).isEmpty then return
    -- Runtime failures, such as an exhausted budget, propagate unrecorded.
    failures.modify fun known => match known[key]? with
      | some none => known
      | some (some before) => known.insert key (left.map (max before))
      | none => known.insert key left
    throw failure

/-- Identify the elements two lookups of one position found: from
`e = some a` and `e = some b`, `a = b`, which injection then splits. -/
elab "verex_merge_lookups" : tactic => focusGoal do
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

/-- Characterize a full-array search after its length has been normalized.
Carrier conversion can expose an `Array.map` while its size has already
reduced to the original array's size. Normalize the theorem the same way
before applying it; rewriting with its unnormalized statement misses that
case. Searches with a different fuel retain their equation. -/
private def characterizeSearch (search : Lean.Expr) (equation : FVarId)
    (position? : Option Lean.Expr) : TacticM Bool := do
  if (← getGoals).isEmpty then return true
  let saved ← saveState
  try
    let goal ← getMainGoal
    let fact ← goal.withContext do
      let lemma := if position?.isSome then ``findIndex?_eq_some_iff else ``findIndex?_eq_none_iff
      let arguments := search.getAppArgs.extract 1 4 ++ position?.toArray
      let characterization ← mkAppM lemma arguments
      let { ctx, simprocs, .. } ← mkSimpContext
        (← `(tactic| simp only [Array.size_map])) (eraseLocal := false)
      let (result, _) ← simp (← inferType characterization) ctx simprocs
      let characterization ← result.mkEqMP characterization
      mkAppM ``Iff.mp #[characterization, mkFVar equation]
    let (_, next) ← (← goal.assert `search (← goal.withContext (inferType fact)) fact).intro1P
    replaceMainGoal [← next.tryClear equation]
    return true
  catch error =>
    saved.restore
    if error.isMaxHeartbeat then throw error
    return false

/-- Split on the result of a search the goal or a hypothesis reads: the
search finds nothing, or a position, characterized by the elements at and
before it. Fails when nothing reads a search. -/
elab "verex_split_search" : tactic => focusGoal do
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
        discard <| characterizeSearch search equation.fvarId! none
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
        unless ← characterizeSearch search equation.fvarId! (some position) do
          -- A successful search is in bounds even when its fuel is not
          -- the full array size, so this does not need a characterization.
          let goal ← getMainGoal
          let bound ← goal.withContext do
            mkAppM ``findIndex?_lt
              (search.getAppArgs.extract 1 6 ++ #[position, equation])
          let (_, next) ←
            (← goal.assert `searchBound (← goal.withContext (inferType bound)) bound).intro1P
          replaceMainGoal [next]
    | _ => throwError "a search has a case besides its result"
    goals := goals ++ (← getGoals)
  setGoals goals.toList

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
elab "verex_expand_ranges" : tactic => focusGoal do
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

/-- The literal bounds of an existential over an integer in the shape a
range quantifier and a vector's membership take, `lo ≤ i ∧ i < hi ∧ _`. -/
private def literalRangeExistential? (e : Lean.Expr) : Option (Int × Int) := do
  guard (e.isAppOfArity ``Exists 2 && (e.getArg! 0).isConstOf ``Int)
  let .lam _ _ body _ := e.getArg! 1 | none
  guard (body.isAppOfArity ``And 2)
  let low := body.getArg! 0
  guard (low.isAppOfArity ``LE.le 4 && low.getArg! 3 == .bvar 0)
  let rest := body.getArg! 1
  guard (rest.isAppOfArity ``And 2)
  let high := rest.getArg! 0
  guard (high.isAppOfArity ``LT.lt 4 && high.getArg! 2 == .bvar 0)
  pure (← intLiteral? (low.getArg! 2), ← intLiteral? (high.getArg! 3))

/-- The existentials over literal ranges a term states, counted at each of
their occurrences up to `cap`, and whether each range spans at most `width`. -/
private partial def literalRangeExistentials (e : Lean.Expr) (cap : Nat) (width : Int) :
    StateM (Std.HashMap Lean.Expr (Nat × Bool)) (Nat × Bool) := do
  if let some known := (← get)[e]? then return known
  let own := match literalRangeExistential? e with
    | some (lo, hi) => (1, decide (hi - lo ≤ width))
    | none => (0, true)
  let children := match e with
    | .app f a => #[f, a]
    | .lam _ type body _ | .forallE _ type body _ => #[type, body]
    | .letE _ type value body _ => #[type, value, body]
    | .mdata _ body | .proj _ _ body => #[body]
    | _ => #[]
  let mut count := own.1
  let mut narrow := own.2
  for child in children do
    let (childCount, childNarrow) ← literalRangeExistentials child cap width
    count := min cap (count + childCount)
    narrow := narrow && childNarrow
  modify (·.insert e (count, narrow))
  return (count, narrow)

/-- An existential goal over integers in small literal ranges, as the
disjunction of its instances, which the normalization computes: a literal
vector's membership, stated over its positions, holds at one of them. Fails
where the goal states no such existential or one over a wide range. -/
elab "verex_expand_goal_ranges" : tactic => focusGoal do
  let goal ← getMainGoal
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  if (target.find? (literalRangeExistential? · |>.isSome)).isNone then
    throwError "the goal states no existential over a literal range"
  let ((count, narrow), _) := (literalRangeExistentials target 5 8).run {}
  unless count ≤ 4 && narrow do
    throwError "the goal states an existential over a wide literal range"
  -- Each instance a case of its own: the conjunctions and existentials
  -- around an expanded existential distribute over its instances.
  evalTactic (← `(tactic| simp only [VerexIR.Proofs.Denote.exists_range_step,
    VerexIR.Proofs.Denote.exists_range_empty, Int.reduceLT, Int.reduceLE, Int.reduceAdd,
    or_false, or_and_right, and_or_left, exists_or]))

/-- An existential goal over a few literal positions, decided at each of
them: a literal vector's membership. -/
macro "verex_goal_witnesses" : tactic => `(tactic| (
  verex_expand_goal_ranges
  all_goals (try simp only [vir_denote, vir_denote_norm, vir_denote_eval, Prod.fst, Prod.snd])
  all_goals (first | omega | verex_assumption)))

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

/-- Rewrite scalar equalities from opaque call summaries by hypothesis
identity. Several summaries use the same binder name (`consumed`), so a
name-based rewrite can select an unrelated equation. Carrier-typed results
are included even when the source-form unname pass keeps their names. -/
elab "verex_scalar_equalities" : tactic => focusGoal do
  let goal ← getMainGoal
  let rules ← goal.withContext do
    let mut rules := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | continue
      unless lhs.isAppOfArity ``VerexIR.SpecInt.val 3 do continue
      let .fvar value := lhs.appArg! | continue
      unless rhs.containsFVar value do rules := rules.push decl.fvarId
    pure rules
  if rules.isEmpty then throwError "no scalar equality"
  let targets ← goal.withContext do
    return (← goal.getNondepPropHyps).filter (!rules.contains ·)
  let next ← simpAt goal (← `(tactic| simp only [])) rules targets true
  replaceMainGoal next.toList

/-- A bounded certified integer used by a position or recursive specification. -/
private def boundedPosition? (goal : MVarId) (throughMarkers := false) :
    MetaM (Option (FVarId × Int × Int)) := do
  let bounds ← integerBounds goal
  goal.withContext do
    let mut terms := #[← instantiateMVars (← goal.getType)]
    for decl in ← getLCtx do
      unless decl.isImplementationDetail do terms := terms.push (← instantiateMVars decl.type)
    let positions (x : FVarId) : Bool := terms.any fun term =>
      (term.find? fun e => e.isAppOfArity ``Int.toNat 1 &&
        (e.appArg!.find? fun part => valuedLocal? part == some x).isSome).isSome ||
        ((if throughMarkers then StatedFacts.unmarked term else term).isForall &&
          (term.find? fun e => valuedLocal? e == some x).isSome)
    -- An argument of a recursive specification function, which unfolds
    -- to its base case at a literal.
    let env ← getEnv
    let specArgument (x : FVarId) : Bool := terms.any fun term =>
      (term.find? fun e => (specDefinitionAt? env e).isSome &&
        (e.find? fun e => valuedLocal? e == some x).isSome).isSome
    -- A position is split over a few values; a specification function's
    -- argument, whose cases evaluate, over as many as a box of positions.
    for (x, ((lo, _), (hi, _))) in bounds do
      if hi - lo < 4 && lo < hi && positions x || hi - lo < 16 && lo ≤ hi && specArgument x then
        return some (x, lo, hi)
    return none

/-- Split a bounded integer's alternatives while retaining each value equation.
The caller decides when to substitute the value through the continuation. -/
private partial def finiteIntegerCases (goal : MVarId) (read : Lean.Expr) (lo hi : Int) :
    MetaM (List (MVarId × FVarId)) := do
  let (statement, proof) ← goal.withContext do
    let disjuncts ← (List.range ((hi - lo).toNat + 1)).mapM fun k =>
      mkEq read (toExpr (lo + Int.ofNat k))
    let some last := disjuncts.getLast? | throwError "an empty range"
    let statement := disjuncts.dropLast.foldr (fun e rest => mkApp2 (mkConst ``Or) e rest) last
    let proof ← mkFreshExprMVar statement
    decideByOmega proof.mvarId!
    return (statement, ← instantiateMVars proof)
  let (cases, goal) ← (← goal.assert `range statement proof).intro1P
  splitCases goal cases
where
  splitCases (goal : MVarId) (cases : FVarId) : MetaM (List (MVarId × FVarId)) := do
    let type ← goal.withContext do instantiateMVars (← cases.getType)
    if type.isAppOfArity ``Or 2 then
      let #[left, right] ← goal.cases cases | throwError "a disjunction has two cases"
      let some (Lean.Expr.fvar equation) := left.fields[0]? | throwError "a case's equation"
      let some (Lean.Expr.fvar rest) := right.fields[0]? | throwError "a case's rest"
      return (left.mvarId, equation) :: (← splitCases right.mvarId rest)
    return [(goal, cases)]

private def certifiedValueCases (goal : MVarId) (x : FVarId) (lo hi : Int) :
    MetaM (List (MVarId × FVarId)) := goal.withContext do
  finiteIntegerCases goal (← mkAppM ``VerexIR.SpecInt.val #[mkFVar x]) lo hi

/-- Assign every position in the already bounded finite box before normalizing
its cases. Substitution carries dependent locals along, so subsequent positions
are addressed through the substitution's map rather than by their old names. -/
private partial def plainPositionCases (goal : MVarId) :
    List (FVarId × Int × Int) → MetaM (List MVarId)
  | [] => pure [goal]
  | (x, lo, hi) :: remaining => do
      let mut goals := []
      for (case, equation) in ← finiteIntegerCases goal (mkFVar x) lo hi do
        let (substitution, case) ← Lean.Meta.substEq case equation
        let remaining ← remaining.mapM fun (position, lo, hi) => do
          let .fvar position := substitution.get position
            | throwError "a remaining position is not a local"
          return (position, lo, hi)
        goals := goals ++ (← plainPositionCases case remaining)
      return goals

/-- Split on the values of integers that range over a few literals and give
positions: an integer local the context bounds by literals, where it gives
a position (possibly computed from `x.val`) or bounds a quantified hypothesis, or else the
plain integer positions in a small literal box, such as a quantified
goal's binders. In each case the integers are those values, and the
positions and ranges they give are literals. -/
elab "verex_split_range" : tactic => focusGoal do
  let goal ← getMainGoal
  let bounds ← integerBounds goal
  -- Arithmetic introduces certified temporaries, for example
  -- `index.val = result.val - 10`. Expose this dependency before looking
  -- for a bounded result used as a position. With no literal bounds there
  -- is no candidate, so avoid traversing the context for scalar rewrites.
  unless bounds.isEmpty do
    evalTactic (← `(tactic| try verex_scalar_equalities))
  if (← getGoals).isEmpty then return
  let goal ← getMainGoal
  let certified? ← boundedPosition? goal
  let plain? ← if certified?.isNone then positionBox? goal else pure none
  if let some ranges := plain? then
    -- Independent plain positions admit a complete tuple assignment. A
    -- dependent type or local definition keeps the existing single-position
    -- path, which can normalize what its first substitution exposes.
    let independent ← goal.withContext do
      ranges.allM fun (x, _, _) => do
        let declaration ← x.getDecl
        return !declaration.isLet &&
          !ranges.any (fun (other, _, _) => declaration.type.containsFVar other)
    if independent then
      replaceMainGoal (← plainPositionCases goal ranges.toList)
      return
  let (disjuncts, certified) ← goal.withContext do
    match certified? with
    | some (x, lo, hi) =>
        let read ← mkAppM ``VerexIR.SpecInt.val #[mkFVar x]
        let disjuncts ← (List.range ((hi - lo).toNat + 1)).mapM fun k =>
          mkEq read (toExpr (lo + Int.ofNat k))
        pure (disjuncts, some x)
    | none =>
        let some ranges := plain?
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

/-- A transport between a value's spelling and a generic callee's view. -/
private def isTransport (t : Lean.Expr) : Bool :=
  t.isConstOf ``NTy.toSkolem || t.isConstOf ``NTy.ofSkolem ||
    t.isConstOf ``HList.toSkolem || t.isConstOf ``HList.ofSkolem ||
    t.isConstOf ``variantCarrier.toSkolem || t.isConstOf ``variantCarrier.ofSkolem ||
    t.isConstOf ``NTy.plainToSkolem || t.isConstOf ``NTy.plainOfSkolem ||
    t.isConstOf ``HListPlain.toSkolem || t.isConstOf ``HListPlain.ofSkolem ||
    t.isConstOf ``variantPlain.toSkolem || t.isConstOf ``variantPlain.ofSkolem

/-- The comparisons (`<`, `≤`) among a term's subterms, each once. -/
private def comparisons (e : Lean.Expr) : Array Lean.Expr :=
  let rec walk (e : Lean.Expr) : StateM (Std.HashSet Lean.Expr × Array Lean.Expr) Unit := do
    if (← get).1.contains e then return
    modify fun (seen, found) => (seen.insert e, found)
    if e.isAppOfArity ``LT.lt 4 || e.isAppOfArity ``LE.le 4 then
      modify fun (seen, found) => (seen, found.push e)
    match e with
    | .app function argument => walk function; walk argument
    | .lam _ type body _ | .forallE _ type body _ => walk type; walk body
    | .letE _ type value body _ => walk type; walk value; walk body
    | .mdata _ body | .proj _ _ body => walk body
    | _ => pure ()
  ((walk e).run ({}, #[])).2.2

/-- A scalar bound with no variable computational input. Type and proof
arguments do not make a literal depend on the context: an empty generic
array's size, for example, is closed even though its element type is not.
This only selects a call for the existing normalization and proved range
substitution; it does not establish the bound or evaluate named definitions. -/
private def closedScalarBound (expression : Lean.Expr) : MetaM Bool := do
  if expression.hasMVar || expression.hasLooseBVars then return false
  if !expression.hasFVar then return true
  let closed ← IO.mkRef true
  Lean.Meta.forEachExpr' expression fun part => do
    if !(← closed.get) || !part.hasFVar then return false
    if (← isType part) || (← isProof part) then return false
    if part.isFVar then
      closed.set false
      return false
    return true
  closed.get

/-- A direct bound on one of the call's results, beneath clause markers and
conjunctions. Quantified clauses and certificates are not scalar bounds. -/
private partial def callResultBound (results : Array FVarId) (type : Lean.Expr) : Bool :=
  if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
    callResultBound results type.appArg!
  else if type.isAppOfArity ``And 2 then
    callResultBound results (type.getArg! 0) || callResultBound results (type.getArg! 1)
  else if type.isAppOfArity ``LE.le 4 || type.isAppOfArity ``LT.lt 4 then
    ((valuedLocal? (type.getArg! 2)).any results.contains) ||
      ((valuedLocal? (type.getArg! 3)).any results.contains)
  else false

/-- Look for literal bounds on the call's result before speculative
normalization at a call, in the premises the call just introduced
(`introduced`, with its result). A transport of the result and a closed
bound on it, such as a literal vector's size, stay eligible: normalization
can reveal their values. Otherwise some literal lower/upper pair on the
result must be close enough for the range splitter. -/
private def callHasLiteralRange (goal : MVarId) (introduced : Array FVarId) : MetaM Bool :=
    goal.withContext do
  let mut terms := #[]
  let mut results := #[]
  for fvarId in introduced do
    let declaration ← fvarId.getDecl
    if declaration.isImplementationDetail then continue
    let type ← instantiateMVars declaration.type
    if ← isProp type then terms := terms.push type else results := results.push fvarId
  let ofResult (expression : Lean.Expr) : Bool := expression.hasAnyFVar results.contains
  let mut lowers : Array Int := #[]
  let mut uppers : Array Int := #[]
  let read (expression : Lean.Expr) : Bool :=
    (expression.isFVar || expression.isAppOfArity ``VerexIR.SpecInt.val 3) && ofResult expression
  for term in terms do
    if (term.find? fun e => isTransport e.getAppFn && ofResult e).isSome then return true
    let bounds ← (comparisons term).foldlM
        (init := ((#[], #[], false) : Array Int × Array Int × Bool))
        fun (lowers, uppers, closedBound) expression => do
      let strict := expression.isAppOfArity ``LT.lt 4
      let left := expression.getArg! 2
      let right := expression.getArg! 3
      let lowers := if read right then
          match intLiteral? left with
          | some value => lowers.push (if strict then value + 1 else value)
          | none => lowers
        else lowers
      let uppers := if read left then
          match intLiteral? right with
          | some value => uppers.push (if strict then value - 1 else value)
          | none => uppers
        else uppers
      let closedBound := closedBound ||
        (read right && (intLiteral? left).isNone && (← closedScalarBound left)) ||
        (read left && (intLiteral? right).isNone && (← closedScalarBound right))
      return (lowers, uppers, closedBound)
    if bounds.2.2 then return true
    lowers := lowers ++ bounds.1
    uppers := uppers ++ bounds.2.1
  return lowers.any fun low => uppers.any fun high => low ≤ high && high - low < 16

/-- Save the caches whose entries depend on a leaf's context or the walk: an
attempt that fails, or leaves its goal as it was, restores them with the
goal. The caches keyed by a hypothesis's type (normal forms, read forms,
projection-free types) hold whatever goal computed them, and are kept. -/
def saveWalkCaches : CoreM (IO Unit) := do
  let state ← targetState
  let bounds ← state.assertedBounds.get
  let families ← state.familyProofs.get
  let rules ← state.contextRules.get
  let assumed ← state.assumedClauses.get
  let slots ← state.assumedSlotSizes.get
  let preparation? ← (← state.preparationProofs.get).mapM fun cache => return (cache, ← cache.get)
  return do
    state.assertedBounds.set bounds
    state.familyProofs.set families
    state.contextRules.set rules
    state.assumedClauses.set assumed
    state.assumedSlotSizes.set slots
    if let some (cache, previous) := preparation? then cache.set previous

/-- Decide a callee's integer result where the call returns, when its
contract bounds it by literals to a few values that give positions: the
continuation is split on the value, and a case the contract's facts refute
at its value, instance by instance, is closed. What follows the call then
reads the result as a literal, in every leaf once. Without a candidate
range, leave the continuation untouched. -/
elab "verex_call_result_cases" : tactic => focusGoal <| withoutRecover do
  let before ← saveState
  let pinnedBefore ← (← targetState).pinnedCount.get
  let earlier ← withMainContext getLCtx
  let (introduced, goal) ← (← getMainGoal).intros
  unless ← callHasLiteralRange goal introduced do
    before.restore
    return
  -- What the step introduces or rewrites; the rest of the context is as the
  -- walk left it.
  let fresh (fvarId : FVarId) : Bool := !earlier.contains fvarId
  let normalizeFresh : TacticM Unit := do
    let goals ← getGoals
    let mut next := #[]
    for goal in goals do
      setGoals [goal]
      normalizeContext false fresh
      next := next ++ (← getGoals).toArray
    setGoals next.toList
  let normalizeBounds : TacticM Unit := do
    let goals ← getGoals
    let mut next := #[]
    let selected (goal : MVarId) : MetaM (Array FVarId) := goal.withContext do
      (← getLCtx).decls.toArray.filterMapM fun
        | some decl => do
            if !fresh decl.fvarId || decl.isImplementationDetail then return none
            return if callResultBound introduced (← instantiateMVars decl.type) then
              some decl.fvarId else none
        | none => pure none
    for goal in goals do
      let bounds ← selected goal
      setGoals [goal]
      unless bounds.isEmpty do
        let facts ← goal.withContext <| bounds.mapM fun id => Lean.Elab.Term.exprToSyntax (mkFVar id)
        evalTactic (← `(tactic| verex_canonical_hypotheses [$facts,*]))
      for canonical in ← getGoals do
        setGoals [canonical]
        let bounds ← selected canonical
        unless bounds.isEmpty do normalizeContext false bounds.contains
        next := next ++ (← getGoals).toArray
    setGoals next.toList
  let restoreCaches ← saveWalkCaches
  try withBudget (← residualAllowance) do
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
      evalTactic (← `(tactic| all_goals verex_split_hypotheses))
      evalTactic (← `(tactic| all_goals verex_subst_vars))
      evalTactic (← `(tactic| all_goals verex_reduce_projections))
      -- The bound's implicit type arguments need their caller spelling before
      -- its literal computation. Prepare only these statements before
      -- traversing the other clauses' family spellings;
      -- the copies above keep dependent construction evidence available.
      normalizeBounds
      evalTactic (← `(tactic| all_goals verex_split_hypotheses))
      evalTactic (← `(tactic| all_goals verex_subst_vars))
      if (← (← targetState).pinnedCount.get) != pinnedBefore then return
      -- Once the result's finite positions are known, normalize the rest
      -- of the contract at its candidate value, not first at an unknown
      -- value. Clause markers are diagnostic metadata, not missing bounds.
      if let [bounded] ← getGoals then
        if let some (x, _, _) ← boundedPosition? bounded (throughMarkers := true) then
          if fresh x then
            break
      -- The facts' family spellings: the continuation keeps its own until
      -- its leaves are prepared.
      evalTactic (← `(tactic| verex_canonical_hypotheses))
      normalizeFresh
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
    if (← (← targetState).pinnedCount.get) == pinnedBefore then
      match ← boundedPosition? normal (throughMarkers := true) with
      | some (x, lo, hi) =>
          if fresh x then
            let cases ← certifiedValueCases normal x lo hi
            setGoals (← cases.mapM fun (case, equation) => substituteValue case x equation)
          else evalTactic (← `(tactic| verex_split_range))
      | none => evalTactic (← `(tactic| verex_split_range))
      if (← getGoals).length ≤ 1 then
        restoreCaches
        before.restore
        return
    let cases ← getGoals
    let mut feasible := #[]
    for case in cases do
      setGoals [case]
      evalTactic (← `(tactic| verex_reduce_projections))
      normalizeFresh
      for split in ← getGoals do
        setGoals [split]
        let saved ← saveState
        try
          evalTactic (← `(tactic| verex_expand_ranges))
          normalizeFresh
        catch _ =>
          saved.restore
        feasible := feasible ++ (← getGoals).toArray
    setGoals feasible.toList
  catch failure =>
    restoreCaches
    throw failure

/-- Whether a leaf holds a value in a generic callee's view: a transport,
or a type spelled at a family a call's type arguments induce. Fails when it
holds none. -/
elab "verex_holds_transport" : tactic => focusGoal do
  let held ← mentionsAnywhere (← getMainGoal) fun t =>
    isTransport t || t.isConstOf ``VerexIR.Proofs.Denote.Carriers.instantiate
  unless held do throwError "the leaf holds no value in a callee's view"

/-- Substitute and renormalize until a substitution leaves the goal as it
was: a renormalization exposes equations to substitute, such as the parts
of a caller's value a transport equation fixes. At most three rounds, with
`normalize` the normalization. -/
syntax "verex_settle " tactic : tactic

elab_rules : tactic
  | `(tactic| verex_settle $normalize) => do
    for _ in [0:3] do
      if (← getGoals).isEmpty then return
      let before ← (← getGoals).mapM fun goal => goalContent goal
      -- The normalization spells a transported value's types at the callee's
      -- family; canonical, they meet the caller's spellings.
      evalTactic (← `(tactic| verex_canonical_families))
      evalTactic (← `(tactic| all_goals verex_split_hypotheses))
      evalTactic (← `(tactic| all_goals verex_subst_vars))
      if (← (← getGoals).mapM fun goal => goalContent goal) == before then return
      let mut left := []
      for goal in ← getGoals do
        if ← goal.isAssigned then continue
        setGoals [goal]
        evalTactic normalize
        left := left ++ (← getGoals)
      setGoals left

/-- The first runtime value a prepared goal states, recorded for the
target's report (`recordRuntimeValues`): the normalization must not
introduce one. -/
elab "verex_check_native " what:str : tactic => focusGoal do
  recordRuntimeValues what.getString (← getMainGoal)

/-- The preparation steps, with `normalize` the normalization of the context,
`bounds` the bounds of the obligation and `derive` the facts its terms read. -/
syntax "verex_prepare_steps " tactic ppSpace tactic ppSpace tactic : tactic

/-- The normalization every leaf receives before a decision: the leaf an
authored proof takes over is left in this form. -/
macro "verex_prepare_leaf" : tactic => `(tactic| verex_prepare_steps verex_normalize_context
  (verex_timed "p-bounds" verex_bounds)
  (verex_timed "p-lookups" verex_lookups_after_writes; verex_timed "p-positions" verex_map_positions))

/-- The preparation a completed path's clauses share: its structural steps,
which substitute only locals no clause reads (`verex.keepTargetLocals`). The
rest is each leaf's: the normalization consumes the forms the leaf's cheap
deciders match syntactically, such as a validity a stored invariant leaves
that a hypothesis states, and the bounds, lookups and map positions an
obligation's terms seed would, seeded by all clauses at once, give every leaf
its siblings' facts. -/
macro "verex_prepare_path" : tactic => `(tactic| verex_prepare_steps skip skip skip)

macro_rules
  | `(tactic| verex_prepare_steps $normalize $bounds $derive) => `(tactic| verex_focus ( (
  all_goals verex_canonical_families
  verex_timed "p-clear" verex_clear_computations
  all_goals verex_timed "p-split" verex_split_hypotheses
  all_goals verex_timed "p-subst" verex_subst_vars
  all_goals try verex_timed "p-variants" verex_determined_variants
  all_goals verex_timed "p-proj" verex_reduce_projections
  all_goals try verex_timed "p-reads" verex_context_memory_reads
  -- The data invariants the leaf's certified values carry, before the
  -- normalization settles them (`assertCertifiedClauses`).
  all_goals verex_timed "p-invariants" verex_certified_clauses
  all_goals verex_timed "normalize" $normalize
  -- A full-array search is characterized after its values and size are in
  -- the same canonical form, including a vector's certification round trip.
  all_goals try verex_split_search
  all_goals verex_timed "p-split2" verex_split_hypotheses
  all_goals verex_timed "p-subst2" verex_subst_vars
  all_goals verex_timed "p-maps" verex_subst_maps
  all_goals verex_timed "p-written" verex_written_back
  all_goals verex_timed "p-proj2" verex_reduce_projections
  -- Lookups of one position, once substitution has exposed them.
  all_goals verex_timed "p-merge" verex_merge_lookups
  -- The bounds of the obligation over its settled terms, normalized once.
  all_goals $bounds:tactic
  all_goals verex_timed "p-split3" verex_split_hypotheses
  all_goals verex_timed "p-subst3" verex_subst_vars
  -- The terms a substitution brought in, such as a written vector, in
  -- normal form too, and the lookups after writes read.
  all_goals verex_timed "renormalize" $normalize
  all_goals verex_timed "p-settle" (verex_settle $normalize)
  -- Write-backs the renormalization exposed, folded before their lookups.
  all_goals verex_timed "p-maps2" verex_subst_maps
  all_goals verex_timed "p-written2" verex_written_back
  all_goals $derive:tactic
  all_goals verex_check_native "a prepared leaf")))

/-- The cases of a split, its refuted fall-throughs dropped. A match on a
computed discriminant states the pattern it took (`d = some x`): in normal
form, the value it fixes is substituted. A lookup is not computed: it is the
read leaves state (`a[i]? = some v`). `known` are the locals before the split. -/
def statePatterns (known : Array FVarId) (cases : List MVarId) : TacticM (List MVarId) := do
  let goals ← cases.filterM fun goal => do return !(← refuteFallthrough goal)
  let mut solved := #[]
  for goal in goals do
    let equations ← goal.withContext do
      (← getLCtx).foldlM (init := #[]) fun found decl => do
        if decl.isImplementationDetail || known.contains decl.fvarId then return found
        let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | return found
        if lhs.isFVar || lhs.isAppOf ``GetElem?.getElem? then return found
        return if ← isConstructorApp rhs then found.push decl.fvarId else found
    if equations.isEmpty then
      solved := solved.push goal
      continue
    -- The simp context elaborates in the main goal's context.
    setGoals [goal]
    let some next ← simpAt goal (← `(tactic| simp only [vir_denote_norm, Option.some.injEq]))
        #[] equations false | continue
    setGoals [next]
    evalTactic (← `(tactic| (verex_split_hypotheses; all_goals verex_subst_vars)))
    solved := solved ++ (← getGoals)
  return solved.toList

/-- The goal with every closed instance of a conditional on `condition`
read as the branch `decision` takes, as `rw [if_pos h]` does, instead of
`split`'s simp over the whole goal: one rewrite of one occurrence per
instance, so the goal's other parts are not restated.  A branch taken may
hold further instances, so the reading repeats. -/
def takeConditionalBranch (goal : MVarId) (condition decision : Lean.Expr) (positive : Bool) :
    MetaM MVarId := do
  let onCondition (e : Lean.Expr) :=
    (e.isAppOfArity ``ite 5 || e.isAppOfArity ``dite 5) && !e.hasLooseBVars &&
      e.getArg! 1 == condition
  let mut next := goal
  repeat
    let target ← instantiateMVars (← next.getType)
    let some occurrence := target.find? onCondition | break
    let dependent := occurrence.isAppOfArity ``dite 5
    let rule := match dependent, positive with
      | false, true => ``if_pos | false, false => ``if_neg
      | true, true => ``dif_pos | true, false => ``dif_neg
    let arguments := occurrence.getAppArgs
    let equation := mkAppN (mkConst rule occurrence.getAppFn.constLevels!)
      #[condition, arguments[2]!, decision, arguments[0]!, arguments[3]!, arguments[4]!]
    let branch := if positive then arguments[3]! else arguments[4]!
    let branch := if dependent then (mkApp branch decision).headBeta else branch
    -- Syntactic occurrences: `kabstract` would unify every conditional.
    let motive ← next.withContext do
      withLocalDeclD `x arguments[0]! fun x =>
        mkLambdaFVars #[x] (target.replace fun e => if e == occurrence then some x else none)
    unless ← next.withContext (isTypeCorrect motive) do
      throwError "a conditional's instance cannot be abstracted"
    next ← next.replaceTargetEq (motive.beta #[branch])
      (← next.withContext (mkCongrArg motive equation))
  return next

/-- The cases of a goal on the condition of `conditional`, an `ite` or
`dite` in it: each case assumes its decision and takes the branch at every
closed instance of a conditional on that condition
(`takeConditionalBranch`). `none` where no instance is closed. -/
def splitConditionInstances? (goal : MVarId) (conditional : Lean.Expr) :
    MetaM (Option (List MVarId)) := goal.withContext do
  unless conditional.isAppOfArity ``ite 5 || conditional.isAppOfArity ``dite 5 do return none
  let condition := conditional.getArg! 1
  if condition.hasLooseBVars then return none
  let onCondition (e : Lean.Expr) :=
    (e.isAppOfArity ``ite 5 || e.isAppOfArity ``dite 5) && !e.hasLooseBVars &&
      e.getArg! 1 == condition
  if ((← instantiateMVars (← goal.getType)).find? onCondition).isNone then return none
  let (yes, no) ← goal.byCasesDec condition (conditional.getArg! 2) (← mkFreshUserName `h)
  [(yes, true), (no, false)].mapM fun (case, positive) =>
    takeConditionalBranch case.mvarId condition (Lean.mkFVar case.fvarId) positive

/-- The conditions of the branches (`if`) among a term's subterms, each once. -/
private def branchConditions (e : Lean.Expr) : Array Lean.Expr :=
  let rec walk (e : Lean.Expr) : StateM (Std.HashSet Lean.Expr × Array Lean.Expr) Unit := do
    if (← get).1.contains e then return
    modify fun (seen, found) => (seen.insert e, found)
    if (e.isAppOfArity ``ite 5 || e.isAppOfArity ``dite 5) && !(e.getArg! 1).hasLooseBVars then
      modify fun (seen, found) => (seen, found.push (e.getArg! 1))
    match e with
    | .app function argument => walk function; walk argument
    | .lam _ type body _ | .forallE _ type body _ => walk type; walk body
    | .letE _ type value body _ => walk type; walk value; walk body
    | .mdata _ body | .proj _ _ body => walk body
    | _ => pure ()
  ((walk e).run ({}, #[])).2.2

/-- Whether omega decides a proposition: a comparison of integers or
naturals, or a conjunction, disjunction, or negation of such. -/
private partial def isArithmetic (p : Lean.Expr) : MetaM Bool := do
  let p := (← instantiateMVars p).consumeMData
  -- A term branching on a condition omega does not read is an atom to it.
  for condition in branchConditions p do
    unless ← isArithmetic condition do return false
  let numeric (type : Lean.Expr) := type.isConstOf ``Int || type.isConstOf ``Nat
  if p.isAppOfArity ``And 2 || p.isAppOfArity ``Or 2 || p.isAppOfArity ``Iff 2 then
    return ← (isArithmetic (p.getArg! 0) <&&> isArithmetic (p.getArg! 1))
  if p.isAppOfArity ``Not 1 then return ← isArithmetic (p.getArg! 0)
  if p.isAppOfArity ``Eq 3 || p.isAppOfArity ``Ne 3 then return numeric (p.getArg! 0)
  if p.isAppOfArity ``LE.le 4 || p.isAppOfArity ``LT.lt 4 || p.isAppOfArity ``GE.ge 4 ||
      p.isAppOfArity ``GT.gt 4 then
    return numeric (p.getArg! 0)
  return false

/-- A proof of `condition` or of its negation over the goal's context,
within a side decision's allowance (`sideDecisionHeartbeats`), and whether
it is of the condition.  `none` where the context leaves the condition
open.  A condition omega already reads (`isArithmetic`) is given to it as
it stands: normalizing it first would run the whole normal form over a
comparison of integers for nothing, once for each polarity of each
condition the goal reads. -/
private def decidedByContext (goal : MVarId) (condition : Lean.Expr) :
    TacticM (Option (Lean.Expr × Bool)) := goal.withContext do
  let arithmetic ← isArithmetic condition
  let decider ← if arithmetic then `(tactic| omega)
    else `(tactic| ((try simp only [vir_denote_norm, beq_iff_eq, decide_eq_true_eq]) <;> omega))
  for (statement, positive) in #[(condition, true), (mkNot condition, false)] do
    let proof ← mkFreshExprMVar statement
    let saved ← saveState
    let closed ← try withBudget sideDecisionHeartbeats (closesBy proof.mvarId! decider)
      catch _ => saved.restore; pure false
    if closed then return some (← instantiateMVars proof, positive)
    saved.restore
  return none

/-- The readings of a `Bool` decided either way as the proposition it
decides, with the readings of the Boolean connectives a decision is
combined by. -/
private def booleanDecisionReadings : Array Name :=
  #[``Bool.not_eq_false', ``Bool.not_eq_true', ``decide_eq_true_eq,
    ``decide_eq_false_iff_not, ``Bool.and_eq_true, ``Bool.or_eq_true,
    ``Bool.and_eq_false_iff, ``Bool.or_eq_false_iff, ``beq_iff_eq,
    ``beq_eq_false_iff_ne]

/-- Whether the readings (`booleanDecisionReadings`) give a `Bool` term a
proposition: it decides one, or combines decisions by the Boolean
connectives. -/
private partial def decidesProposition (e : Lean.Expr) : Bool :=
  if e.isAppOfArity ``Decidable.decide 2 || e.isAppOfArity ``BEq.beq 4 then true
  else if e.isAppOfArity ``Bool.not 1 then decidesProposition e.appArg!
  else if e.isAppOfArity ``Bool.and 2 || e.isAppOfArity ``Bool.or 2 then
    decidesProposition (e.getArg! 0) || decidesProposition (e.getArg! 1)
  else false

/-- An observation's index ignores only the evidence certifying a plain
value. The family, native type, and value stay in the key. Independently
transported certificates of the same value need not have the same syntax;
proof irrelevance identifies them. This key is never used in a proof, and
matching keys are checked for definitional equality before closing a path. -/
private partial def certifiedObservationKey (term : Lean.Expr) : Lean.Expr :=
  term.replace fun sub =>
    if sub.isAppOfArity ``NTy.ofDeepHolds 4 then
      some (mkAppN sub.getAppFn #[certifiedObservationKey (sub.getArg! 0),
        certifiedObservationKey (sub.getArg! 1), certifiedObservationKey (sub.getArg! 2),
        mkConst ``True.intro])
    else none

/-- The path's decisions on a `Bool` the program computed, read as the
propositions they decide: a program branches on a `Bool`, a clause states
the proposition, and every decider after this step — the conditionals the
context decides, the implications whose premises hold, and arithmetic —
reads propositions only. Opposite literal observations of the same canonical
expression close the path directly. A decision left in its Boolean spelling is an
atom to them, so the clause restating the program's conditional stays
undecided at the leaf and its proof is searched for instead.  Fails where
the context decides no computed `Bool`. -/
elab "verex_boolean_decisions" : tactic => focusGoal do
  let goal ← getMainGoal
  let (decisions, conflict?) ← goal.withContext do
    let mut found : Array FVarId := #[]
    let mut observed : Std.HashMap Lean.Expr (Lean.Expr × Lean.Expr × Lean.Expr) := {}
    let literal (e : Lean.Expr) := e.isConstOf ``Bool.true || e.isConstOf ``Bool.false
    for decl in ← getLCtx do
      if decl.isImplementationDetail || decl.isLet then continue
      let some (ty, left, right) := (← instantiateMVars decl.type).eq? | continue
      unless ty.isConstOf ``Bool do continue
      let reading? := if literal right then some (left, right, false)
        else if literal left then some (right, left, true) else none
      let some (term, value, reversed) := reading? | continue
      let term := term.consumeMData
      let proof ← if reversed then mkEqSymm decl.toExpr else pure decl.toExpr
      let key := certifiedObservationKey term
      if let some (previousTerm, previous, previousProof) := observed.get? key then
        if value != previous then
          unless ← withNewMCtxDepth (withReducible (isDefEq previousTerm term)) do continue
          -- The same canonical expression cannot return opposite literals.
          -- Cancel it directly, without unfolding its surrounding context.
          let equation ← mkEqTrans (← mkEqSymm previousProof) proof
          return (found, some (← mkNoConfusion (← goal.getType) equation))
      else
        observed := observed.insert key (term, value, proof)
      if decidesProposition term then found := found.push decl.fvarId
    return (found, none)
  if let some proof := conflict? then
    goal.assign proof
    replaceMainGoal []
    return
  if decisions.isEmpty then throwError "the context decides no computed Boolean"
  let mut theorems : SimpTheorems := {}
  for reading in booleanDecisionReadings do theorems ← theorems.addConst reading
  let ctx ← Simp.mkContext {} #[theorems] (← getSimpCongrTheorems)
  let (result, _) ← simpGoal goal ctx #[] none (simplifyTarget := false)
    (fvarIdsToSimp := decisions)
  match result with
  | none => replaceMainGoal []
  | some (_, next) => replaceMainGoal [next]

/-- The conditionals the goal reads whose condition the leaf's context
decides, each read as the branch it takes (`takeConditionalBranch`): a
clause restating the program's conditional reads the atoms the path's
decisions already fix, and the branch taken is all the clause says there.
The condition is decided rather than split, so the leaf keeps one case and
each condition is decided once however many instances read it; a split
would carry the conditionals the branch does not take into a case of their
own, and leaving them to the pipeline's normalization restates the whole
obligation in its proof. -/
elab "verex_decided_conditionals" : tactic => focusGoal do
  let mut goal ← getMainGoal
  let conditionals ← goal.withContext do
    pure (sitesWhere (fun e => (e.isAppOfArity ``ite 5 || e.isAppOfArity ``dite 5) &&
      !e.hasLooseBVars && !(e.getArg! 1).hasLooseBVars)
      #[← instantiateMVars (← goal.getType)])
  if conditionals.isEmpty then throwError "the goal reads no conditional"
  let mut decided : Array Lean.Expr := #[]
  let mut taken := 0
  for conditional in conditionals do
    let condition := conditional.getArg! 1
    if decided.contains condition then continue
    decided := decided.push condition
    let some (decision, positive) ← decidedByContext goal condition | continue
    goal ← takeConditionalBranch goal condition decision positive
    taken := taken + 1
  if taken == 0 then throwError "the context decides no condition the goal reads"
  replaceMainGoal [goal]

/-- Split the goal's first conditional or match, after the variants of the
locals it tests where `variants` holds. A match's fall-through case that its
computed discriminant refutes is closed at once. -/
private def splitFirst (variants : Bool) : TacticM Unit := focusGoal do
  -- A discriminant a hypothesis equates to a term is split as that term.
  let original ← getMainGoal
  let originalContent? ← if (← (← targetState).preparationOwners.get).isEmpty then pure none
    else some <$> goalContent original
  evalTactic (← `(tactic| verex_subst_vars))
  if variants then evalTactic (← `(tactic| all_goals try verex_split_goal_variants))
  -- A refuted or destructured goal is progress enough.
  if (← getGoals).length != 1 then return
  -- The context stays normalized for the deciders that rewrite with it,
  -- where the goal, normalized, has a conditional or match to split or
  -- reads memory the context may fix.
  if (← getMainGoal) != original then
    evalTactic (← `(tactic| try simp only [vir_denote, vir_denote_norm, vir_denote_eval,
      Prod.fst, Prod.snd]))
    let some goal := (← getGoals).head? | do
      if originalContent?.isSome then return
      throwError "the normalization closed the goal"
    let splittable ← goal.withContext do
      let target ← instantiateMVars (← goal.getType)
      if (← findSplit? target).isSome then return true
      -- The variables applied to two arguments, as a memory is read.
      let collect : StateT FVarIdSet MetaM Unit := target.forEach fun e => do
        if e.getAppNumArgs == 2 && e.getAppFn.isFVar then modify (·.insert e.getAppFn.fvarId!)
      let (_, applied) ← collect.run {}
      applied.toList.anyM fun id => do
        return (← whnfR (← inferType (.fvar id))).isAppOfArity ``VerexIR.Proofs.Denote.Memory 2
    unless splittable do
      -- A single feasible constructor is useful progress too. Discarding
      -- it would repeat the same preparation in this owner's leaf attempt.
      if let some previous := originalContent? then
        if (← goalContent goal) != previous then return
      throwError "the goal has no conditional, match, or memory read"
    evalTactic (← `(tactic| verex_normalize_context_stage))
  -- A read of memory the context fixes is that value, so that a match on
  -- it is decided, or is the program's; resolving one is progress enough.
  let beforeReads ← getMainGoal
  evalTactic (← `(tactic| try verex_memory_reads))
  if (← getGoals).length != 1 || (← getMainGoal) != beforeReads then return
  let known ← (← getMainGoal).withContext do return (← getLCtx).getFVarIds
  let goal ← getMainGoal
  let found ← goal.withContext do findSplit? (← instantiateMVars (← goal.getType))
  -- A conditional splits at its instances; a match by `split`.
  if let some cases ← found.bindM fun e => splitConditionInstances? goal e then setGoals cases
  else evalTactic (← `(tactic| split))
  setGoals (← statePatterns known (← getGoals))

/-- Split the goal's first conditional or match, the variants of the locals
it tests first. -/
elab "verex_split" : tactic => splitFirst true

/-- Split the goal's first conditional or match only: the pipeline's split,
which a leaf reaches after it tried the variants of its tested locals. -/
elab "verex_split_conditional" : tactic => splitFirst false

/-- A memory read fixed by a program step may still be conditional on a
write's address aliasing the read's. Split only a read equation whose
conditional branch is a memory lookup the goal actually uses. -/
elab "verex_memory_aliases" : tactic => focusGoal do
  let goal ← getMainGoal
  let candidate ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let reads ← (sitesWhere (fun e => e.getAppNumArgs == 2 && e.getAppFn.isFVar)
      #[target]).filterM fun read => do
        return (← whnfR (← inferType read.getAppFn)).isAppOfArity
          ``VerexIR.Proofs.Denote.Memory 2
    if reads.isEmpty then return none
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some (_, left, right) := (← instantiateMVars decl.type).eq? | continue
      for conditional in #[left, right] do
        unless conditional.isAppOfArity ``ite 5 do continue
        unless (conditional.getArg! 1).isAppOfArity ``Eq 3 do continue
        unless ← reads.anyM (fun read => withNewMCtxDepth <| withDefault do
            return (← isDefEq read (conditional.getArg! 3)) ||
              (← isDefEq read (conditional.getArg! 4))) do continue
        return some (decl.fvarId, conditional)
    return none
  let some (hypothesis, conditional) := candidate
    | throwError "the goal reads no unresolved memory alias"
  let (yes, no) ← goal.byCasesDec (conditional.getArg! 1) (conditional.getArg! 2)
    (← mkFreshUserName `alias)
  let mut goals := []
  for branch in [yes.mvarId, no.mvarId] do
    setGoals [branch]
    normalizeContext (target := false) (select := fun id => id == hypothesis)
    goals := goals ++ (← getGoals)
  setGoals goals

/-- The behaviors a clause states of a function value at the entry values of
its arguments, each with the behavior of an arguments row it is the reading
of, and the theorem that reads it (`Behavior.lean`). -/
private def entryBehaviorReadings : Array (Name × Name × Name) :=
  #[(``VerexIR.Proofs.EnsuresOfEntries, ``VerexIR.Proofs.EnsuresOf,
      ``VerexIR.Proofs.ensuresOfEntries_of_ensuresOf),
    (``VerexIR.Proofs.AbortsOfEntries, ``VerexIR.Proofs.AbortsOf,
      ``VerexIR.Proofs.abortsOfEntries_of_abortsOf)]

/-- A premise stating a behavior of a function value at the entry values of
its arguments, from a hypothesis stating that behavior of an arguments row.
A clause names values and no certificate, so a data invariant of a
function-valued field states `ensures_of` and `aborts_of` at entry values
(`EnsuresOfEntries`), while the leaf of an invocation holds the run itself,
at the certified row the call passes (`EnsuresOf`): the two meet by reading
the run at its row's entry values, and the invariant's quantified value and
result are the row's and the run's. -/
elab "verex_entry_behavior" : tactic => focusGoal do
  let goal ← getMainGoal
  goal.withContext do
  let target := StatedFacts.unmarked (← instantiateMVars (← goal.getType))
  let some (_, behavior, reading) := entryBehaviorReadings.find? fun (entries, _, _) =>
      target.isAppOf entries
    | throwError "the goal states no behavior at entry values"
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let statement := StatedFacts.unmarked (← instantiateMVars decl.type)
    unless statement.isAppOf behavior do continue
    let saved ← saveState
    let rule ← mkConstWithFreshMVarLevels reading
    let (arguments, _, conclusion) ← forallMetaTelescope (← inferType rule)
    let some run := arguments.back? | continue
    if ← isDefEq run decl.toExpr then
      if ← isDefEq conclusion target then
        let proof ← instantiateMVars (mkAppN rule arguments)
        unless proof.hasExprMVar do
          goal.assign proof
          replaceMainGoal []
          return
    saved.restore
  throwError "no hypothesis states the behavior of an arguments row"

/-- A premise of an instance: a hypothesis, either way round, a behavior at
entry values, arithmetic, or reflexivity. -/
macro "verex_instance_premise" : tactic => `(tactic| verex_focus (
  first
  | verex_assumption
  -- A behavior the instance states of entry values, from the leaf's run.
  | verex_entry_behavior
  | omega
  | with_reducible rfl
  | (symm; verex_assumption)
  -- The bounds of what the instance brings in.
  | (verex_bounds; omega)))

/-- A position as an integer: the integer it is `Int.toNat` of, or its cast. -/
private def intPosition (position : Lean.Expr) : MetaM Lean.Expr :=
  if position.isAppOfArity ``Int.toNat 1 then pure (position.getArg! 0)
  else mkAppOptM ``Nat.cast #[mkConst ``Int, none, position]

/-- Decide the goal by the context: the goal alone in normal form
rewritten by the hypotheses, then by arithmetic. -/
syntax "verex_decide_by_context" : tactic

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
macro "verex_witness_premise" : tactic => `(tactic| verex_focus (
  first
  | verex_timed "wp-instance" verex_instance_premise
  | verex_timed "wp-simp" (simp (disch := omega) only [Int.toNat_natCast, Array.getElem?_push_size, VerexIR.Proofs.Denote.getElem?_push_of_lt,
      Array.getElem?_setIfInBounds_self_of_lt, Array.getElem?_setIfInBounds_ne]; done)
  | verex_timed "wp-context" verex_decide_by_context))

/-- A witness of a type along its pairs: a pair of witnesses, the unit, or
an unknown. -/
private partial def assembleWitness (type : Lean.Expr) : MetaM Lean.Expr := do
  let type ← whnfD type
  if type.isAppOfArity ``Prod 2 then
    mkAppM ``Prod.mk #[← assembleWitness (type.getArg! 0), ← assembleWitness (type.getArg! 1)]
  else if type.isConstOf ``Unit || type.isAppOfArity ``PUnit 0 then
    pure (mkConst ``Unit.unit)
  else mkFreshExprMVar type

/-- The parts of a type of pairs that are not the unit, each spelled as the
pair spells it. -/
private partial def pairParts (type : Lean.Expr) : MetaM (Array Lean.Expr) := do
  let unfolded ← whnfD type
  if unfolded.isAppOfArity ``Prod 2 then
    return (← pairParts (unfolded.getArg! 0)) ++ (← pairParts (unfolded.getArg! 1))
  else if unfolded.isConstOf ``Unit || unfolded.isAppOfArity ``PUnit 0 then return #[]
  else return #[type]

/-- A value of a type of pairs with one part not the unit: the pairs of
that part, given, and units. -/
private partial def pairOfPart (type part : Lean.Expr) : MetaM Lean.Expr := do
  let unfolded ← whnfD type
  if unfolded.isAppOfArity ``Prod 2 then
    mkAppM ``Prod.mk #[← pairOfPart (unfolded.getArg! 0) part,
      ← pairOfPart (unfolded.getArg! 1) part]
  else if unfolded.isConstOf ``Unit || unfolded.isAppOfArity ``PUnit 0 then
    pure (mkConst ``Unit.unit)
  else pure part

/-- An existential over a value of pairs with one part not the unit, such
as a structure of one field, from an existential over that part: the
witness search reads the context's values of the part, as
`verex_native_components` splits the context's values of pairs. -/
def pairPartExistential? (goal : MVarId) (domain body : Lean.Expr) : MetaM (Option MVarId) :=
  goal.withContext do
    unless (← whnfD domain).isAppOfArity ``Prod 2 do return none
    let #[partType] ← pairParts domain | return none
    let partBody ← withLocalDeclD `part partType fun part => do
      mkLambdaFVars #[part] (body.beta #[← pairOfPart domain part]).headBeta
    let split ← mkFreshExprSyntheticOpaqueMVar (← mkAppM ``Exists #[partBody])
    let introduced ← withLocalDeclD `part partType fun part =>
      withLocalDeclD `held (partBody.beta #[part]) fun held => do
        mkLambdaFVars #[part, held]
          (← mkAppOptM ``Exists.intro #[domain, body, ← pairOfPart domain part, held])
    goal.assign (← mkAppM ``Exists.elim #[split, introduced])
    return some split.mvarId!

/-- The conjuncts of a proposition. -/
private partial def conjunctsOf (e : Lean.Expr) : Array Lean.Expr :=
  if e.isAppOfArity ``And 2 then conjunctsOf (e.getArg! 0) ++ conjunctsOf (e.getArg! 1)
  else #[e]

/-- Construct the certified value whose plain row the body's equations
fully determine. Its certificate remains a proof obligation for the caller;
no witness is produced when any data is still unknown. -/
def determinedCertifiedWitness? (domain body : Lean.Expr) :
    MetaM (Option (Lean.Expr × MVarId)) := do
  let nominal ← whnfR domain
  unless nominal.isAppOfArity ``Subtype 2 do return none
  observing? do
    let row ← assembleWitness (nominal.getArg! 0)
    let certificate ← mkFreshExprMVar ((nominal.getArg! 1).beta #[row])
    let witness ← mkAppOptM ``Subtype.mk
      #[nominal.getArg! 0, nominal.getArg! 1, row, certificate]
    for conjunct in conjunctsOf (← whnfR (body.beta #[witness])) do
      if let some (_, left, right) := conjunct.eq? then
        discard <| isDefEq left right
    if (← instantiateMVars row).hasExprMVar then
      throwError "the equations do not determine the witness data"
    return (witness, certificate.mvarId!)

/-- A goal stating, perhaps among other conjuncts, an existential over a
value of pairs whose body equates the parts of the value to terms, as a
value's typing states it field by field: each such witness assembled from
those terms by unifying each equation. What the bodies and the other
conjuncts state is left to prove. -/
partial def assembleWitnesses (goal : MVarId) : MetaM (Array MVarId × Bool) := goal.withContext do
  let target ← whnfR (← instantiateMVars (← goal.getType))
  if target.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
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

elab "verex_assembled_witness" : tactic => focusGoal do
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
  unless flow.isAppOfArity ``Flow 4 do return none
  let carriers := flow.getArg! 0
  let value ← whnfR value
  unless value.isAppOf ``VerexIR.Proofs.Denote.Flow.value do return none
  let rec walk (row env : Lean.Expr) (found : Array (Lean.Expr × Lean.Expr)) :
      TacticM (Array (Lean.Expr × Lean.Expr)) := do
    let row ← whnfR row
    unless row.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 do return found
    let env ← whnfR env
    unless env.isAppOfArity ``Prod.mk 4 do return found
    let entry ← whnfR (env.getArg! 2)
    let found := if entry.isAppOfArity ``Option.some 2 then
        found.push (row.getArg! 0, entry.getArg! 1) else found
    walk (row.getArg! 1) (env.getArg! 3) found
  return some (carriers, ← walk (flow.getArg! 2) value.appArg! #[])

/-- Whether a binder's domain is one a state label binds: a memory, or a
copy of a mutable parameter at its clause domain (an integer's `Int`, a
Boolean's `Bool`, a textual scalar's `String`, any other value's carrier). -/
private def labelDomain (domain : Lean.Expr) : Bool :=
  domain.isAppOf ``VerexIR.Proofs.Denote.Memory || domain.isConstOf ``Int ||
    domain.isConstOf ``Bool || domain.isConstOf ``String ||
    domain.isAppOf ``VerexIR.Proofs.Denote.NTy.carrier || domain.isAppOf ``VerexIR.Proofs.Denote.HList

/-- The value of a local as a contract reads it in a binder's domain: an
integer's value, any other value itself, a reference's by its referent's
current value. `none` for a domain the value is not read in. -/
private def localInDomain (τ value domain : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let τ ← whnfR τ
  let (τ, value) ← if τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.ref 1 then
      let value ← whnfR value
      let current ← if value.isAppOfArity ``Prod.mk 4 then pure (value.getArg! 2)
        else mkAppM ``Prod.fst #[value]
      pure (τ.getArg! 0, current)
    else pure (τ, value)
  if domain.isConstOf ``Int then
    unless τ.isAppOfArity ``VerexIR.Proofs.Denote.NTy.int 2 ||
        τ.isConstOf ``VerexIR.Proofs.Denote.NTy.num do return none
    return some (← mkAppM ``VerexIR.SpecInt.val #[value])
  -- A row's carrier is its pairs only once the row unfolds.
  if ← withDefault (isDefEq domain (← inferType value)) then return some value
  -- A reference's referent in a specification is its lent current, which is
  -- what the body sees and writes (`doc/designs/data-invariants.md`);
  -- a certified value crosses to it by forgetting its certificates.
  let spelled ← whnfR domain
  let some spelling := plainSpelling? spelled | return none
  let .ty _ := spelling.payload | return none
  let some family := spelling.carriers? | return none
  let forgotten ← mkAppOptM ``VerexIR.Proofs.Denote.NTy.forget #[family, τ, value]
  if ← withDefault (isDefEq domain (← inferType forgotten)) then return some forgotten
  return none

/-- The program points of the context, each a value and a state. -/
def programPoints : MetaM (Array (Lean.Expr × Lean.Expr)) := do
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
syntax "verex_witness" : tactic

/-- The heartbeats the cheap decider spends searching an existential's
witness. A candidate the search rejects pays every premise decider on its
conjuncts, and a nested existential's whole search; a leaf whose witness needs
the context rewritten is decided by the later steps, which rewrite it once. -/
def cheapWitnessHeartbeats : Nat := 4000000

/-- Run the witness search within `cheapWitnessHeartbeats`; exhausting it is
an ordinary failure. -/
elab "verex_cheap_witness" : tactic => focusGoal do
  evalBudgeted cheapWitnessHeartbeats (← `(tactic| verex_witness))

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
      evalTactic (← `(tactic| try simp only [vir_denote, vir_denote_norm, vir_denote_eval,
        Prod.fst, Prod.snd]))
      -- A witness state's reads the context fixes, as their values.
      evalTactic (← `(tactic| all_goals try verex_memory_reads))
    evalTactic (← `(tactic| all_goals repeat' apply And.intro))
    let split := (← getGoals).toArray
    let ordered := if split.size == first.size then
        ((split.zip first).qsort fun (_, a) (_, b) => a && !b).map (·.1)
      else split
    let decide ← `(tactic| first
      | verex_witness
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
def assignWitnessed (goal : MVarId)
    (chosen : Array (Lean.Expr × Lean.Expr × Lean.Expr)) (proof : Lean.Expr) : TacticM Unit := do
  let mut proof := proof
  for (domain, predicate, witness) in chosen.reverse do
    proof ← mkAppOptM ``Exists.intro #[domain, predicate, witness, proof]
  goal.assign proof
  setGoals []

/-- A state label's binders witnessed at a program point: the point's state
for a `Memory` binder, the values of its locals in the binder's domain for the
others, each binder in turn; the witnesses and the body they instantiate. -/
partial def pointWitness? (value state target : Lean.Expr) :
    TacticM (Option (Array (Lean.Expr × Lean.Expr × Lean.Expr) × Lean.Expr)) := do
  let stateType ← inferType state
  let locals ← pointLocals value
  let rec choose (body : Lean.Expr) (used : Array Lean.Expr)
      (chosen : Array (Lean.Expr × Lean.Expr × Lean.Expr)) :
      TacticM (Option (Array (Lean.Expr × Lean.Expr × Lean.Expr) × Lean.Expr)) := do
    let body ← whnfR body
    let some (domain, predicate) := body.app2? ``Exists | return some (chosen, body)
    let memory ← try isDefEq stateType domain catch _ => pure false
    let candidates ← if memory then pure #[state] else
      match locals with
      | some (_, locals) =>
          locals.filterMapM fun (τ, entry) => do
            let some candidate ← localInDomain τ entry domain | return none
            return if used.contains candidate then none else some candidate
      | none => pure #[]
    for candidate in candidates do
      if let some found ← choose (predicate.beta #[candidate]) (used.push candidate)
          (chosen.push (domain, predicate, candidate)) then
        return some found
    return none
  choose target #[] #[]

/-- An existential goal over a state label — a memory and copies of the
mutable parameters, as nested existentials — witnessed at one program point:
the point's state for a `Memory` binder, the values of its locals in the
binder's domain for the others; the instantiated body normalized and decided
conjunct by conjunct. -/
private partial def ledgerWitness (goal : MVarId) (domain target : Lean.Expr) : TacticM Bool := do
  -- Only a label's binders: a memory, or a copy of a mutable parameter in
  -- the domain its contract reads it in.
  unless labelDomain domain do return false
  for (value, state) in ← programPoints do
    let some (chosen, body) ← pointWitness? value state target | continue
    if chosen.isEmpty then continue
    let some proof ← decideInstance body (← `(tactic| verex_instance_premise))
      (normalize := true) | continue
    assignWitnessed goal chosen proof
    return true
  return false

/-- Values already read from storage or a typed map can witness a clause
without first receiving a local name. Keep the existing read operations whole;
only these syntactic sites are checked against the binder's type. -/
private def observedWitnessValues (goal : MVarId) : MetaM (Array Lean.Expr) := goal.withContext do
  let mut terms := #[← instantiateMVars (← goal.getType)]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    terms := terms.push (← instantiateMVars decl.type)
  return sitesWhere (fun term => !term.hasLooseBVars &&
    (term.isAppOfArity ``Option.getD 3 || term.isAppOfArity ``Skolems.ofRuntime 5 ||
      term.isAppOfArity ``VerexIR.SpecMap.valueAt 6 ||
      term.isAppOfArity ``VerexIR.SpecMap.keyAt 5)) terms

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

elab_rules : tactic
  | `(tactic| verex_witness) => focusGoal do
    let goal ← getMainGoal
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
        setGoals []
        return
      restoreState saved
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
          evalTactic (← `(tactic| verex_witness_premise))
          pure (← getGoals).isEmpty
        catch _ => pure false
      restoreState saved
      unless decided do
        throwError "a conjunct of the existential not reading its binder is not decided"
    -- A candidate witness.
    let attempt (witness : Lean.Expr) (premise : Lean.TSyntax `tactic) : TacticM Bool := do
      unless ← isDefEq (← inferType witness) domain do return false
      let some proof ← decideInstance (body.beta #[witness]) premise reads | return false
      assignWitnessed goal #[(domain, body, witness)] proof
      return true
    -- An integer the body bounds, such as a mutable parameter's value between
    -- two increments. A literal bound, such as a limit of the binder's type
    -- every bounded binder states, is tried after the context's values: a
    -- candidate that fails searches every nested binder in vain.
    let (literalBounds, bounds) ← if domain.isConstOf ``Int then
        pure ((← boundWitnesses body).partition fun bound => bound.int?.isSome)
      else pure (#[], #[])
    for value in bounds do
      if ← attempt value (← `(tactic| verex_witness_premise)) then return
    -- A variable of the binder's type in the context, or the value of a
    -- certified integer for an integer binder.
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      if ← attempt decl.toExpr (← `(tactic| verex_instance_premise)) then return
    if domain.isConstOf ``Int then
      for decl in ← getLCtx do
        if decl.isImplementationDetail || decl.isLet then continue
        let some value ← (try some <$> mkAppM ``VerexIR.SpecInt.val #[decl.toExpr]
            catch _ => pure none) | continue
        if ← attempt value (← `(tactic| verex_witness_premise)) then return
    -- A stored value remains a compound read when the context names only
    -- its fields. Its existing certificate witnesses a declared value; no
    -- row or certificate is reconstructed from the scalar equations.
    for value in ← observedWitnessValues goal do
      if ← attempt value (← `(tactic| verex_witness_premise)) then return
    -- An equation can determine a nominal value's whole plain row even
    -- when no existing certified read names it. Construct the value only
    -- after proving its certificate, then decide the complete instance.
    -- A failed construction leaves no partial witness or extra goals.
    let construct : TacticM Bool := do
      let saved ← saveState
      try
        let some (witness, certificate) ← determinedCertifiedWitness? domain body
          | throwError "no determined certified witness"
        unless ← certificate.isAssigned do
          let statement ← instantiateMVars (← certificate.getType)
          let some certified ← decideInstance statement
              (← `(tactic| verex_instance_premise)) (normalize := true)
            | throwError "the witness certificate is not established"
          certificate.assign certified
        let witness ← instantiateMVars witness
        if witness.hasExprMVar then throwError "undetermined witness"
        unless ← attempt witness (← `(tactic| verex_witness_premise)) do
          throwError "the constructed witness does not establish the instance"
        return true
      catch _ =>
        saved.restore
        return false
    if ← construct then return
    -- A position the body reads an array written at, or one a lookup
    -- fact of the context reads.
    for position in ← writtenPositions body do
      if ← attempt position (← `(tactic| verex_witness_premise)) then return
    let mut read : Array Lean.Expr := #[]
    for fvarId in ← goal.getNondepPropHyps do
      let fact ← instantiateMVars (← fvarId.getType)
      unless fact.isAppOfArity ``Eq 3 && (fact.getArg! 2).isAppOfArity ``Option.some 2 do continue
      let lookup := fact.getArg! 1
      unless lookup.isAppOfArity ``GetElem?.getElem? 7 do continue
      let position ← intPosition (lookup.getArg! 6)
      unless read.contains position do read := read.push position
    for position in read do
      if ← attempt position (← `(tactic| verex_witness_premise)) then return
    for value in literalBounds do
      if ← attempt value (← `(tactic| verex_witness_premise)) then return
    -- Prefer witnesses from the predicate and context before enumerating
    -- execution points. A label is an existential, including in caller-side
    -- contracts with no callee points; arithmetic can construct its value
    -- directly without repeatedly rejecting entire point environments.
    if ← ledgerWitness goal domain target then return
    if let some split ← pairPartExistential? goal domain body then
      replaceMainGoal [split]
      evalTactic (← `(tactic| (try dsimp only); verex_witness))
      return
    throwError "no hypothesis states the existential at a witness"

/-- Discharge a frame directly from the writes in its target. Only memory
update lemmas and stated inequalities participate; no context normalization
or entry relation is needed once the relevant memory has been linked. -/
def closeFrameWrites (goal : MVarId) : MetaM Bool := do
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  unless (target.find? (·.isAppOf ``Memory.set)).isSome do return false
  let saved ← saveState
  try
    let (goal, _) ← introduceThroughMarkers goal
    let mut theorems : SimpTheorems := {}
    for lemma in #[``VerexIR.Proofs.Obligation_eq, ``Memory.set_same,
        ``Memory.set_other, ``Memory.set_other_resource, ``if_neg, ``ite_false,
        ``eq_self, ``implies_true, ``and_true, ``true_and] do
      theorems ← theorems.addConst lemma
    let ctx ← Simp.mkContext { contextual := true } #[theorems] {}
    let discharge : Simp.Discharge := fun condition => do
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let type ← instantiateMVars decl.type
        unless type.isAppOf ``Not || type.isAppOf ``Ne do continue
        if ← withReducible (isDefEq type condition) then return some decl.toExpr
      return none
    let (result, _) ← goal.withContext <|
      simpGoal goal ctx #[] (some discharge) (fvarIdsToSimp := #[])
    if let some (_, next) := result then withReducible next.refl
    return true
  catch error =>
    saved.restore
    if error.isMaxHeartbeat then throw error
    return false

elab "verex_frame_writes" : tactic => focusGoal do
  unless ← closeFrameWrites (← getMainGoal) do
    throwError "the target is not a frame established by its memory writes"
  setGoals []

/-- A disjunction as the implication from the negation of its left side,
the negation in the normal form the clause translation states one in
(`¬∃ i, … ∧ …` as `∀ i, … → ¬…`). -/
elab "verex_goal_or" : tactic => focusGoal do
  let_expr Or _ _ := (← instantiateMVars (← (← getMainGoal).getType))
    | throwError "the goal is no disjunction"
  evalTactic (← `(tactic| (
    refine Classical.or_iff_not_imp_left.mpr ?_
    intro refuted
    try simp only [not_exists, not_and, not_or, Classical.not_forall, Classical.not_not,
      Classical.not_imp] at refuted)))

/-- A nominal value certified natively has the same constructor as its
subtype representation. Check reflexivity at default transparency only
when the equation names that certification explicitly. -/
elab "verex_certified_refl" : tactic => focusGoal do
  let goal ← getMainGoal
  goal.withContext do
    let target := StatedFacts.unmarked (← instantiateMVars (← goal.getType))
    let some (_, left, right) := target.eq? | throwError "the goal is no equation"
    let env ← getEnv
    let nominalCertification (e : Lean.Expr) : Bool :=
      e.isAppOfArity ``NTy.ofDeepHolds 4 &&
        ((e.getArg! 1).isAppOfArity ``NTy.struct 3 ||
          (e.getArg! 1).isAppOfArity ``NTy.enum 5 || isNominalType env (e.getArg! 1))
    unless (target.find? nominalCertification).isSome do
      throwError "the equation names no nominal certification"
    unless ← withDefault (isDefEq left right) do
      throwError "the certified equation is not reflexive"
    goal.assign (← mkEqRefl left)

/-- The cheap deciders of a leaf: those that never rewrite the context. -/
syntax "verex_decide_cheap" : tactic

macro_rules
  | `(tactic| verex_decide_cheap) => `(tactic| verex_focus (
  first
  | verex_timed "c-frame writes" verex_frame_writes
  | ((try simp only [VerexIR.Proofs.Obligation_eq, Nat.reduceAdd, Int.reducePow,
      Int.reduceSub, Nat.reducePow, Nat.reduceSub])
     first
     | done
     | verex_timed "c-assumption" verex_assumption
     | verex_timed "c-certified reflexivity" verex_certified_refl
     | verex_timed "c-ground" verex_ground_arithmetic
     | verex_timed "c-omega" (verex_bounds; verex_linear)
     | verex_timed "c-decide" verex_decide
     | verex_timed "c-witness" verex_cheap_witness
     -- A reading of a constructor the goal itself holds, which no split
     -- reaches (`verex_variant_redex`): computed where it stands, before a
     -- disjunction is taken from the negation of its left side, which a
     -- computed name would decide outright.
     | (verex_timed "c-redex" verex_variant_redex
        all_goals verex_decide_cheap)
     -- A conjunction, such as a clause with an abort code, conjunct by
     -- conjunct.
     | (apply And.intro <;> verex_decide_cheap)
     -- A disjunction, such as a clause and its negation, from the negation
     -- of its left side.
     | (verex_timed "c-or" verex_goal_or
        verex_decide_cheap))))

/-- `trivial` at reducible transparency and without `decide` and
`contradiction`: an open goal's decision and a search for contradicting
hypotheses belong to the deciders, which bound them. -/
syntax "verex_trivial" : tactic

macro_rules
  | `(tactic| verex_trivial) => `(tactic| verex_focus (
      first
      | verex_assumption
      | with_reducible rfl
      | exact True.intro
      | (apply And.intro <;> verex_trivial)))

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
def storageAddresses (e : Lean.Expr) : Array Lean.Expr :=
  let rec walk (e : Lean.Expr) : StateM (Std.HashSet Lean.Expr × Array Lean.Expr) Unit := do
    if (← get).1.contains e then return
    modify fun (seen, found) => (seen.insert e, found)
    if e.isAppOfArity ``VerexIR.StorageKey.address 1 then
      let argument := e.appArg!
      unless (← get).2.contains argument do
        modify fun (seen, found) => (seen, found.push argument)
    else match e with
      | .app function argument => walk function; walk argument
      | .lam _ domain body _ | .forallE _ domain body _ => walk domain; walk body
      | .letE _ type value body _ => walk type; walk value; walk body
      | .mdata _ inner | .proj _ _ inner => walk inner
      | _ => pure ()
  ((walk e).run ({}, #[])).2.2

/-- A proof of a premise by a hypothesis stating it, either way round, or by
reflexivity. Both are normal, so they are compared as terms, and, where the
terms agree in their heads, up to reducible unfolding: typed statements
spell their instances and annotations, which the normal form does not fix.
A comparison up to unfolding at every hypothesis of every leaf would cost
more than the deciders that follow. -/
private def premiseByContext? (premise : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let premise ← instantiateMVars premise
  let flipped? ← match premise.eq? with
    | some (_, left, right) =>
        if left == right then return some (← mkEqRefl left)
        pure (some (← mkEq right left))
    | none => pure none
  let states (type statement : Lean.Expr) : MetaM Bool := do
    if type == statement then return true
    let type := type.consumeMData
    unless type.getAppFn == statement.getAppFn && type.getAppNumArgs == statement.getAppNumArgs do
      return false
    withReducible (isDefEq type statement)
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    if ← states type premise then return some decl.toExpr
    if let some flipped := flipped? then
      if ← states type flipped then return some (← mkEqSymm decl.toExpr)
  return none

/-- The goal over the facts omega reads without splitting cases
(`splitFree`) related to its target: such a fact is kept when it shares a
variable with the target or with a kept fact, up to a fixpoint. -/
private def clearUnrelated (goal : MVarId) : MetaM MVarId := goal.withContext do
  let mut related := (collectFVars {} (← instantiateMVars (← goal.getType))).fvarSet
  let mut props : Array (FVarId × FVarIdSet) := #[]
  let mut compound : Array FVarId := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let ty ← instantiateMVars decl.type
    if ← isProp ty then
      -- A fact omega would case-split on (`splitFree`).
      if !splitFree ty then
        compound := compound.push decl.fvarId
      else props := props.push (decl.fvarId, (collectFVars {} ty).fvarSet)
  let mut kept : FVarIdSet := {}
  let mut grown := true
  while grown do
    grown := false
    for (fvarId, vars) in props do
      if kept.contains fvarId then continue
      if vars.toList.any related.contains then
        kept := kept.insert fvarId
        related := vars.toList.foldl (init := related) fun acc v => acc.insert v
        grown := true
  goal.tryClearMany (compound ++ props.filterMap fun (fvarId, _) => if kept.contains fvarId then none else some fvarId)

/-- Each hypothesis `p₁ → … → pₙ → q` whose premises omega or a hypothesis
proves is replaced by its conclusion `q`: the conclusion of a lemma under the
conditions its statement or a proof's path puts on it. Fails when none is. -/
elab "verex_modus_ponens" : tactic => focusGoal do
  let initial ← saveState
  -- The goal's own premises are what may establish an implication's.
  let (goal, _) ← introduceThroughMarkers (← getMainGoal)
  let mut used : Array FVarId := #[]
  let mut facts : Array Lean.Expr := #[]
  let candidates ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let type ← instantiateMVars decl.type
      unless ← (pure type.isArrow <&&> isProp type) do return found
      return found.push decl
  -- Omega's decision of each premise, once: the candidates share premises,
  -- such as a position's bounds in the facts about every other position.
  let decided ← IO.mkRef (∅ : Std.HashMap Lean.Expr (Option Lean.Expr))
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
        let premise ← instantiateMVars premise
        if let some known := (← decided.get)[premise]? then
          let some proof := known | throwError "a premise is not proved"
          proofs := proofs.push proof
          continue
        let proof ← goal.withContext (mkFreshExprSyntheticOpaqueMVar premise)
        -- Over the hypotheses the premise relates to: each check would
        -- otherwise read, and case-split on, the whole context.
        setGoals [← clearUnrelated proof.mvarId!]
        let closed ← try
            evalTactic (← `(tactic| omega))
            pure (← getGoals).isEmpty
          catch _ => pure false
        unless closed do
          decided.modify (·.insert premise none)
          throwError "a premise is not proved"
        let proof ← instantiateMVars proof
        decided.modify (·.insert premise (some proof))
        proofs := proofs.push proof
      used := used.push decl.fvarId
      facts := facts.push (← goal.withContext (mkExpectedTypeHint (mkAppN decl.toExpr proofs)
        conclusion))
    catch _ => saved.restore
  if facts.isEmpty then
    initial.restore
    throwError "no implication's premises hold"
  let goal ← assertFacts goal `implied facts
  setGoals [← goal.tryClearMany used]
  -- The bounds of what the implications brought in.
  discard assertBounds

/-- Whether an application of a constant may reduce at reducible
transparency: the constant is reducible, or `whnfCore` reduces it at any
transparency (a projection of a structure, a recursor, a matcher). -/
private def headMayReduce (head : Name) : MetaM Bool := do
  if ← isReducible head then return true
  let env ← getEnv
  if (env.getProjectionFnInfo? head).isSome || isAuxRecursor env head then return true
  if (env.find? head) matches some (.recInfo _) then return true
  isMatcher head

/-- Whether two terms cannot unify at reducible transparency by their heads:
applications of different constants, neither of which may reduce. -/
private def headsDiffer (left right : Lean.Expr) : MetaM Bool := do
  let .const leftHead _ := left.getAppFn | return false
  let .const rightHead _ := right.getAppFn | return false
  if leftHead == rightHead then return false
  return !(← headMayReduce leftHead) && !(← headMayReduce rightHead)

/-- Whether a quantified hypothesis's conclusion cannot have the goal as an
instance: its relation, or a side of an equation, has another head. -/
private def conclusionDiffers (conclusion target : Lean.Expr) : MetaM Bool := do
  if ← headsDiffer conclusion target then return true
  match conclusion.eq?, target.eq? with
  | some (_, left, right), some (_, goalLeft, goalRight) =>
      return ← (headsDiffer left goalLeft <||> headsDiffer right goalRight)
  | _, _ => return false

/-- A hypothesis read up to negation (`doc/designs/proof-language.md`, "Closers"):
`¬∃ x, φ` as `∀ x, ¬φ`, `¬(A ∨ B)` as `¬A` and `¬B`, `¬(A ∧ B)` as `A → ¬B`,
`¬¬A` as `A`, and an integer comparison's negation as the comparison, through
premises. Each reading's proof and statement; none where the reading changes
nothing. No normal form holds these: a search consults them. -/
partial def negationReadings (proof type : Lean.Expr) (changed : Bool := false) :
    MetaM (Array (Lean.Expr × Lean.Expr)) := do
  let type := (← instantiateMVars type).consumeMData
  let under (name : Name) (domain : Lean.Expr)
      (read : Lean.Expr → MetaM (Array (Lean.Expr × Lean.Expr))) :
      MetaM (Array (Lean.Expr × Lean.Expr)) :=
    withLocalDeclD name domain fun x => do
      (← read x).mapM fun (inner, statement) =>
        return (← mkLambdaFVars #[x] inner, ← mkForallFVars #[x] statement)
  -- `proof` applied to what `build` makes of an assumed `held`: a proof of `¬held`.
  let negated (held : Lean.Expr) (build : Lean.Expr → MetaM Lean.Expr) : MetaM Lean.Expr :=
    withLocalDeclD `held held fun x => do
      mkLambdaFVars #[x] (mkApp proof (← build x))
  if let .forallE name domain body _ := type then
    return ← under name domain fun x =>
      negationReadings (mkApp proof x) (body.instantiate1 x) changed
  let finish := if changed then #[(proof, type)] else #[]
  let_expr Not inner := type | return finish
  match_expr inner.consumeMData with
  | Exists α body =>
      under `x α fun x => do
        let read := (mkApp body x).headBeta
        let reading ← negated read fun held =>
          mkAppOptM ``Exists.intro #[α, body, x, held]
        negationReadings reading (mkNot read) true
  | Or left right =>
      let first ← negated left fun held => return mkApp3 (mkConst ``Or.inl) left right held
      let second ← negated right fun held => return mkApp3 (mkConst ``Or.inr) left right held
      return (← negationReadings first (mkNot left) true) ++
        (← negationReadings second (mkNot right) true)
  | And left right =>
      under `left left fun held => do
        let reading ← negated right fun other =>
          return mkApp4 (mkConst ``And.intro) left right held other
        negationReadings reading (mkNot right) true
  | Not positive =>
      let reading := mkApp2 (mkConst ``Classical.byContradiction) positive proof
      negationReadings reading positive true
  | LE.le α _ a b =>
      unless ← isDefEq α (mkConst ``Int) do return finish
      let reading ← mkAppM ``Iff.mp #[← mkAppOptM ``Int.not_le #[a, b], proof]
      return #[(reading, ← inferType reading)]
  | LT.lt α _ a b =>
      unless ← isDefEq α (mkConst ``Int) do return finish
      let reading ← mkAppM ``Iff.mp #[← mkAppOptM ``Int.not_lt #[a, b], proof]
      return #[(reading, ← inferType reading)]
  | _ => return finish

/-- The readings of a goal's hypotheses up to negation (`negationReadings`),
each proof typed by its reading. -/
def contextNegationReadings : MetaM (Array Lean.Expr) := do
  (← getLCtx).foldlM (init := #[]) fun found decl => do
    if decl.isImplementationDetail then return found
    let type ← instantiateMVars decl.type
    unless (type.getForallBody.consumeMData.isAppOfArity ``Not 1) do return found
    let readings ← negationReadings decl.toExpr type
    return found ++ (← readings.mapM fun (proof, statement) => mkExpectedTypeHint proof statement)

/-- Expose a quantified pair only while selecting an instance. This lets a
membership premise determine both components even when the conclusion reads
only one projection, without expanding unrelated quantified environments. -/
private def productInstance (proof : Lean.Expr) : MetaM Lean.Expr := do
  let type ← inferType proof
  let .forallE name domain body info := type | return proof
  let_expr Prod left right := ← whnfR domain | return proof
  let predicate := Lean.Expr.lam name domain body info
  let equivalence ← mkAppOptM ``Prod.forall #[left, right, predicate]
  mkAppM ``Iff.mp #[equivalence, proof]

/-- A goal that is an instance of a quantified hypothesis: the hypothesis
applied at reducible transparency, its premises closed by omega or by a
hypothesis. A hypothesis whose conclusion's head differs is not tried. -/
elab "verex_instance" : tactic => focusGoal do
  let initial ← saveState
  let (goal, _) ← introduceThroughMarkers (← getMainGoal)
  let refuting ← goal.withContext do return (← instantiateMVars (← goal.getType)).isConstOf ``False
  let candidates ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let ty ← instantiateMVars decl.type
      -- A quantified fact, not a function the context binds, such as a
      -- shared continuation or a recursive meaning.
      unless ty.isForall && (← isProp ty) do return found
      if refuting then
        if let some unfolded := unfoldNegatedConclusion ty then
          return found.push (← mkExpectedTypeHint decl.toExpr unfolded)
      return found.push decl.toExpr
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let attempt (candidate : Lean.Expr) : TacticM Bool := do
    let candidate ← goal.withContext (productInstance candidate)
    let conclusion := (← goal.withContext do instantiateMVars (← inferType candidate)).getForallBody
    if !conclusion.hasLooseBVars || !(conclusion.getAppFn.isBVar) then
      if ← goal.withContext (conclusionDiffers conclusion target) then return false
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
      evalTactic (← `(tactic| all_goals verex_instance_premise))
      if (← getGoals).isEmpty then return true
    catch _ => pure ()
    saved.restore
    return false
  for candidate in candidates.reverse do
    if ← attempt candidate then return
  -- The hypotheses read up to negation, once no hypothesis as it stands is one.
  for candidate in ← goal.withContext contextNegationReadings do
    if ← attempt candidate then return
  initial.restore
  throwError "no quantified hypothesis has the goal as an instance"

/-- A failed call can be contradicted by its ordering guard alone. Try
the non-quantified facts before preparing every guarded postcondition of
the hypothetical successful call. Failure retains the full context. -/
elab "verex_map_contradiction" : tactic => focusGoal do
  if (← (← targetState).preparationOwners.get).isEmpty then throwError "no shared automatic owner"
  let goal ← getMainGoal
  unless (← goal.withContext (instantiateMVars (← goal.getType))).isConstOf ``False do
    throwError "the goal is not a contradiction"
  unless ← mentionsAnywhere goal (fun e => e.isConstOf ``VerexIR.Proofs.Denote.NTy.order ||
      e.isConstOf ``VerexIR.Proofs.Denote.NTy.keyOrder ||
      e.isConstOf ``VerexIR.Proofs.Denote.NTy.readOrder || e.isConstOf ``VerexIR.SpecMap.intOrder) do
    throwError "no map ordering guard"
  let compound ← goal.withContext do
    (← goal.getNondepPropHyps).filterM fun hyp => do
      return (← instantiateMVars (← hyp.getType)).isForall
  if compound.isEmpty then throwError "no guarded facts to omit"
  let saved ← saveState
  let restore ← saveWalkCaches
  try
    withBudget 4000000 do
      setGoals [← goal.tryClearMany compound]
      Lean.Elab.Tactic.withoutRecover <| evalTactic (← `(tactic| (
        simp_all only
          [not_or, not_and, Classical.not_not, Int.lt_irrefl, true_implies, false_implies]
        all_goals verex_map_positions
        all_goals omega
        done)))
  catch failure =>
    saved.restore
    restore
    throw failure

/-- Map observations whose conditionals need resolving before position
preparation, including source iterator tags and payloads. -/
def readsConditionalMap (goal : MVarId) : MetaM Bool := goal.withContext do
  let reads (expression : Lean.Expr) := (expression.find? fun term =>
    term.isAppOf ``VerexIR.SpecMap.rank || term.isAppOf ``variantName ||
    term.isAppOf ``Choices.select?).isSome
  unless reads (← instantiateMVars (← goal.getType)) do return false
  for declaration in ← getLCtx do
    if declaration.isImplementationDetail then continue
    let type ← instantiateMVars declaration.type
    if (type.find? (·.isAppOf ``VerexIR.SpecMap.Valid)).isSome then return true
  return false

/-- Instances of the hypotheses quantified over an address at the addresses
the leaf reads memory at: a global invariant assumed at entry speaks about
every address, a proof about the ones it reads, as the Move Prover's
triggers select them. -/
elab "verex_key_instances" : tactic => focusGoal do
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
  -- A new instance contains concrete reads absent from its quantified
  -- source. Commit it first, then link and normalize those reads. Optional
  -- normalization rolls back independently without losing the instance.
  -- Every leaf's deciders read the linked reads, the tree's own included.
  let saved ← saveState
  let restoreCaches ← saveWalkCaches
  try
    evalTactic (← `(tactic| all_goals try verex_context_memory_reads))
    evalTactic (← `(tactic| all_goals verex_normalize_context))
  catch _ =>
    saved.restore
    restoreCaches

/-- The local at the root of a field projection, keeping the certificate
projection as part of the path rather than unfolding its predicate. -/
private partial def projectedLocal? (e : Lean.Expr) (projected : Bool := false) : Option FVarId :=
  match e.consumeMData with
  | .fvar id => if projected then some id else none
  | .proj _ _ value => projectedLocal? value true
  | value =>
      if value.isAppOfArity ``Subtype.val 3 || value.isAppOfArity ``Prod.fst 3 ||
          value.isAppOfArity ``Prod.snd 3 then projectedLocal? value.appArg! true
      else none

/-- A vector field defined by a write reads as that write. A certified
owner stays whole, so its field cannot be substituted as a local vector
can. Only independent definitions are oriented toward their computation. -/
private def vectorFieldDefinition (proof : Lean.Expr)
    (equation : Option (Lean.Expr × Lean.Expr)) :
    MetaM (Lean.Expr × Option (Lean.Expr × Lean.Expr)) := do
  let some (left, right) := equation | return (proof, equation)
  let some owner := projectedLocal? right | return (proof, equation)
  if (projectedLocal? left).isSome || left.containsFVar owner then return (proof, equation)
  unless (← whnfR (← inferType (.fvar owner))).isAppOfArity ``Subtype 2 do
    return (proof, equation)
  unless (← whnfD (← inferType right)).isAppOfArity ``VerexIR.SpecVector 1 do
    return (proof, equation)
  return (← mkEqSymm proof, some (right, left))

/-- The rewrite rules of a hypothesis for `simpByContext`, oriented as
`orientedHypothesis` does. -/
private def contextRulesOf (decl : Lean.LocalDecl) (type : Lean.Expr) : MetaM (Array SimpTheorem) := do
  if let some rules := (← (← targetState).contextRules.get)[(decl.fvarId, type)]? then return rules
  let rules ← do
    if (← selfReferential type) || !(← rewritesByPattern type) then pure #[] else
    let (proof, equation) ← orientedHypothesis decl.fvarId
    let (proof, equation) ← vectorFieldDefinition proof equation
    let mut rules ← mkSimpTheoremFromExpr (.fvar decl.fvarId) #[] proof
    if let some (lhs, _) := equation then
      if lhs.isAppOfArity ``GetElem.getElem 8 && (lhs.getArg! 0).isAppOfArity ``Array 1 then
        let lookup ← mkAppM ``Array.getElem?_eq_getElem #[lhs.getArg! 7]
        let read ← mkEqTrans lookup
          (← mkCongrArg (← mkAppOptM ``Option.some #[lhs.getArg! 2]) proof)
        rules := rules ++ (← mkSimpTheoremFromExpr (.fvar decl.fvarId) #[] read)
    pure rules
  (← targetState).contextRules.modify (·.insert (decl.fvarId, type) rules)
  return rules

/-- The goal, or the given hypotheses, in normal form rewritten by the
other hypotheses, but for the equations that would rewrite a term into one
containing it, and an equation whose reverse is already a rule. An element
a hypothesis fixes (`a[i] = v`) is read through the lookup the leaves state
(`a[i]? = some v`). -/
private def simpByContext (goal : MVarId) (targets : Array FVarId) (simplifyTarget : Bool)
    (failIfUnchanged : Bool) : TacticM (Option MVarId) := goal.withContext do
  let stx ← `(tactic| simp only [VerexIR.Proofs.Obligation_eq, Nat.reduceAdd, Int.reducePow,
    Int.reduceSub, Nat.reducePow, Nat.reduceSub, vir_denote_norm])
  let { ctx, simprocs, .. } ← mkSimpContext stx (eraseLocal := false)
  let mut hypotheses : SimpTheorems := {}
  let mut equations : Array (Lean.Expr × Lean.Expr) := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail || targets.contains decl.fvarId then continue
    let type ← instantiateMVars decl.type
    unless ← isProp type do continue
    let rules ← contextRulesOf decl type
    if rules.isEmpty then continue
    if let some (_, lhs, rhs) := type.eq? then
      if equations.contains (rhs, lhs) then continue
      equations := equations.push (lhs, rhs)
    hypotheses := rules.foldl SimpTheorems.addSimpTheorem hypotheses
  let ctx ← Simp.mkContext { ctx.config with failIfUnchanged } (ctx.simpTheorems.push hypotheses)
    (withoutBranchConditions ctx.congrTheorems)
  match ← simpGoal goal ctx simprocs (simplifyTarget := simplifyTarget) (fvarIdsToSimp := targets) with
  | (none, _) => return none
  | (some (_, goal), _) => return some goal

elab "verex_simp_by_context" : tactic => focusGoal do
  match ← simpByContext (← getMainGoal) #[] true true with
  | none => replaceMainGoal []
  | some goal => replaceMainGoal [goal]

/-- Read a certified vector field from the equation defining its current
value before preparing the owner as a whole. Other goals do not take a
context-rewriting pass here. -/
elab "verex_vector_field_reads" : tactic => focusGoal do
  let goal ← getMainGoal
  let previous ← goal.withContext do instantiateMVars (← goal.getType)
  let readsField ← goal.withContext do
    let target := previous
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some (_, left, right) := (← instantiateMVars decl.type).eq? | continue
      let some owner := projectedLocal? right | continue
      unless target.containsFVar owner do continue
      let (oriented, _) ← vectorFieldDefinition decl.toExpr (some (left, right))
      if oriented != decl.toExpr then return true
    return false
  unless readsField do throwError "the goal reads no defined certified vector field"
  match ← simpByContext goal #[] true true with
  | none => replaceMainGoal []
  | some next =>
      let changed ← next.withContext do
        return (← instantiateMVars (← next.getType)) != previous
      unless changed do throwError "the vector field read did not change the goal"
      replaceMainGoal [next]

/-- The hypotheses stating an invocation's run or result rewritten by the
other hypotheses into the terms the goal reads. Fails when none changes. -/
elab "verex_simp_behaviors_by_context" : tactic => focusGoal do
  let goal ← getMainGoal
  let targets ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let type ← instantiateMVars decl.type
      if type.isForall then return found
      let statement := match type.eq? with
        | some (_, lhs, _) => lhs
        | none => type
      return if statement.isAppOf ``VerexIR.Proofs.EnsuresOf ||
          statement.isAppOf ``VerexIR.Proofs.ResultOf then found.push decl.fvarId
        else found
  if targets.isEmpty then throwError "no hypothesis states an invocation's run or result"
  match ← simpByContext goal targets false true with
  | none => replaceMainGoal []
  | some goal => replaceMainGoal [goal]

/-- Rewrite the goal once by an instance of a quantified equation (or
equivalence) among the hypotheses, in either direction, at a subterm the
equation's side matches, its premises proved by a hypothesis or omega; the
rewritten goal must then be decided by the context. Each hypothesis and
direction is tried once, so the search is bounded, unlike rewriting with
the equations as rules, which may cycle. -/
elab "verex_rewrite_instance" : tactic => focusGoal do
  let initial ← saveState
  let goal ← getMainGoal
  let candidates ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let ty ← instantiateMVars decl.type
      return if ty.isForall && (← isProp ty) then found.push decl.toExpr else found
  let closeByContext ← `(tactic| first
    | done
    | verex_assumption
    | omega
    | (verex_simp_by_context
       first | done | verex_assumption | omega))
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
            evalTactic (← `(tactic| first | verex_assumption | omega))
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

/-- Name the element at each lookup in the goal and the hypotheses whose
position the context proves in bounds and whose result it states nothing
about: the lookup reads `some` of the element, and a fact about the lookup
becomes one about the element. Fails when no lookup is named. -/
syntax "verex_name_lookups" : tactic

/-- Instantiate each hypothesis quantified over integer positions that reads
an array at every combination of the positions the context reads arrays
at, its other premises decided by a hypothesis or by omega, and the
instances rewritten by the context. Fails when nothing is instantiated. -/
syntax "verex_instantiate_positions" : tactic

/-- Rewrite the goal's reads of `result_of` by what its hypotheses state the
invocation returned (`rewriteResultOf`). -/
syntax "verex_rewrite_results" : tactic

/-- A goal at a position one past the range of a quantified hypothesis,
the step that extends a range invariant by one iteration: the hypothesis
states the goal for the positions its range premises admit, and one
premise, a comparison, is not proved at the goal's position. The goal
splits on that premise: where it holds, the goal is the hypothesis's
instance; where it fails, omega pins the position to the range's bound,
and the goal there is decided by the context. -/
private def rangeInstanceCore (normalize : Bool) (splitConditions := false) : TacticM Unit := do
  let initial ← saveState
  let (goal, _) ← introduceThroughMarkers (← getMainGoal)
  let candidates ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let ty ← instantiateMVars decl.type
      return if ty.isForall && (← isProp ty) then found.push decl.toExpr else found
  -- The context at the bound is normalized first, where asked: the facts
  -- there come from the iteration, such as a tested condition, as they
  -- were stated. It is the prepared leaf's normalization, which skips the
  -- hypotheses known normal.
  -- Linear goals already have omega. Congruence is useful when a symbolic
  -- product hides the accumulator equality, but general search at unrelated
  -- endpoints can consume the budget needed by an authored proof.
  let congruence ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let nonlinear := (target.find? fun e => e.isAppOfArity ``HMul.hMul 6 &&
      (e.getArg! 4).int?.isNone && (e.getArg! 5).int?.isNone).isSome
    if nonlinear then `(tactic| grind only)
    else `(tactic| fail "no symbolic product")
  -- At the bound, the iteration's element is read through the instances
  -- there of the context's position invariants, such as a frame invariant
  -- stating it unchanged, with its lookups named.
  let closeByContext ← if normalize then `(tactic| first
    | verex_assumption
    | omega
    | $congruence:tactic
    | ((try verex_normalize_context)
       verex_simp_by_context
       first | done | omega | verex_assumption)
    | ((try verex_lookups_after_writes)
       verex_instantiate_positions
       verex_name_lookups
       (try verex_native_decompose)
       (try verex_simp_behaviors_by_context)
       (try verex_rewrite_results)
       first | done | omega | verex_assumption))
    else `(tactic| first | verex_assumption | omega)
  let closeByContext ← if splitConditions then `(tactic| (
      try simp (disch := verex_assumption) only [if_pos, if_neg, dif_pos, dif_neg] at *
      all_goals $closeByContext:tactic))
    else pure closeByContext
  for candidate in candidates.reverse do
    let saved ← saveState
    try
      let premises ← withReducible (goal.apply candidate)
      let mut failing : Array MVarId := #[]
      for premise in premises do
        if ← premise.isAssigned then continue
        setGoals [premise]
        try evalTactic (← `(tactic| first | verex_assumption | omega))
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
      evalTactic (← `(tactic| all_goals first | verex_assumption | omega))
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
            if verex.solverDebug.get (← getOptions) then
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

/-- A split conditional can hide the syntactic match between a range invariant
and its instance. Specialize at integer positions in the goal, then simplify
only those new facts using guards already known in the context. The range
premises remain: the endpoint solver must still prove them or justify the new
endpoint from the current iteration. -/
private def conditionalRangeInstances : TacticM Unit := do
  let mut goal ← getMainGoal
  let facts ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let positions ← (collectFVars {} target).fvarIds.filterM fun id => do
      return (← whnfR (← inferType (.fvar id))).isConstOf ``Int
    let mut facts : Array Lean.Expr := #[]
    unless positions.isEmpty do
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let type ← instantiateMVars decl.type
        unless type.isForall && !type.isArrow && type.bindingDomain!.isConstOf ``Int do continue
        unless (type.find? fun e => e.isAppOfArity ``ite 5 || e.isAppOfArity ``dite 5).isSome do continue
        for position in positions do
          facts := facts.push (mkApp decl.toExpr (.fvar position))
    return facts
  if facts.isEmpty then throwError "no conditional range instance"
  let mut added := #[]
  for proof in facts do
    let (fact, next) ← goal.withContext do
      (← goal.assert `atPosition (← inferType proof) proof).intro1P
    goal := next
    added := added.push fact
  setGoals [goal]
  let next ← simpAt goal (← `(tactic| simp (disch := verex_assumption) only
    [if_pos, if_neg, dif_pos, dif_neg])) #[] added false
  setGoals next.toList

private def rangeInstance (normalize : Bool) : TacticM Unit := do
  let initial ← saveState
  try
    rangeInstanceCore normalize
  catch _ =>
    initial.restore
    try
      conditionalRangeInstances
      unless (← getGoals).isEmpty do rangeInstanceCore normalize (splitConditions := true)
    catch failure =>
      initial.restore
      throw failure

elab "verex_range_instance" : tactic => focusGoal <| rangeInstance true

/-- `verex_range_instance` deciding the bound by the context as
stated: a leaf the prepared deciders normalize first pays the
normalization once, there. -/
elab "verex_range_instance_stated" : tactic => focusGoal <| rangeInstance false

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
elab "verex_split_write" : tactic => focusGoal do
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
        [VerexIR.Proofs.Denote.getElem?_eraseIdx_toNat_of_lt]))
      let firstGoals ← getGoals
      setGoals [second.mvarId]
      readers (← `(tactic| simp (disch := omega) only
        [VerexIR.Proofs.Denote.getElem?_eraseIdx_toNat_of_ge]))
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
    VerexIR.Proofs.Denote.getElem?_push_of_lt, Array.getElem?_insertIdx_self, vir_denote_norm])
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
        [VerexIR.Proofs.Denote.getElem?_insertIdx_toNat_of_lt]))
      let firstGoals ← getGoals
      setGoals [second.mvarId]
      readers (← `(tactic| simp (disch := omega) only
        [VerexIR.Proofs.Denote.getElem?_insertIdx_toNat_of_gt]))
      setGoals (sameGoals ++ firstGoals ++ (← getGoals))
  | .set | .push | .erase =>
      setGoals [apart.mvarId]
      readers (← `(tactic| simp (disch := omega) only [Array.getElem?_setIfInBounds_ne,
        Array.getElem_setIfInBounds_ne, VerexIR.Proofs.Denote.getElem?_push_of_lt]))
      setGoals (sameGoals ++ (← getGoals))

/-- The closed array lookups in terms. -/
private def arrayLookups (terms : Array Lean.Expr) : Array Lean.Expr :=
  sitesWhere (fun e => e.isAppOfArity ``GetElem?.getElem? 7 && !e.hasLooseBVars &&
    (e.getArg! 0).isAppOfArity ``Array 1) terms

elab_rules : tactic
  | `(tactic| verex_name_lookups) => focusGoal do
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
        simpMainAt (← `(tactic| simp only [Option.map_some, Option.getD_some, vir_denote_norm]))
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
        simpMainAt (← `(tactic| simp only [Option.map_some, Option.getD_some, vir_denote_norm]))
          #[factVar.fvarId!] mentioning true
        goal ← getMainGoal
        named := true
      catch ex =>
        if verex.solverDebug.get (← getOptions) then
          IO.println s!"naming failed: {← ex.toMessageData.toString}"
        attempt.restore
    unless named do throwError "no lookup at a position in bounds to name"
    replaceMainGoal [goal]

elab_rules : tactic
  | `(tactic| verex_instantiate_positions) => focusGoal do
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
    let debug := verex.solverDebug.get (← getOptions)
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
          -- Element-wise properties also quantify over the value read at the
          -- position. Keep that value universally bound in the instance;
          -- bindOpenPremises retains it and its dependent read equation.
          for m in mvars do
            if ← m.mvarId!.isAssigned then continue
            let cost ← goal.withContext do
              let premise ← instantiateMVars (← m.mvarId!.getType)
              unless ← isProp premise do return 0
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
          if verex.solverDebug.get (← getOptions) then
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
elab "verex_split_disjunction" : tactic => focusGoal do
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
syntax "verex_cheap_cases " num : tactic

macro_rules
  | `(tactic| verex_cheap_cases $depth) => do
      let depth := depth.getNat
      if depth == 0 then `(tactic| fail "no cases left")
      else
        let rest := Lean.Syntax.mkNumLit (toString (depth - 1))
        `(tactic| verex_focus (
          verex_split_disjunction
          all_goals (first
            | verex_assumption
            | verex_decide_cheap
            | verex_instance
            | verex_cheap_cases $rest)
          done))

/-- Decide a branch of a written position: as an instance of the context,
a refuted hypothesis, in each case of another written position, with the
elements at positions in bounds named, or with the context's position
invariants instantiated there and the goal rewritten by the context. -/
syntax "verex_decide_split" : tactic

macro_rules
  | `(tactic| verex_decide_split) => `(tactic| verex_focus (
  first
  | done
  | verex_timed "s-omega" verex_linear
  | verex_timed "s-instance" verex_instance
  -- An existential over the written element, witnessed where its lookup
  -- reads the written position; the clause's marker is its statement.
  | verex_timed "s-witness" ((try simp only [VerexIR.Proofs.Obligation_eq]); verex_cheap_witness)
  | verex_timed "s-range" verex_range_instance
  | verex_timed "s-refute" (exfalso; verex_instance)
  | (verex_timed "s-split" verex_split_write
     all_goals verex_decide_split)
  | (verex_timed "s-name" verex_name_lookups
     verex_intro_negation
     verex_subst_vars
     first
     | done
     | omega
     | verex_instance
     | (exfalso; verex_instance))
  | (verex_timed "s-positions" (verex_intro_negation; verex_instantiate_positions)
     first
     | done
     | verex_timed "s-positions-omega" verex_linear
     | verex_instance
     | (verex_timed "s-positions-context"
          ((try verex_name_lookups); verex_simp_by_context)
        first | done | omega | verex_assumption))
  | verex_timed "s-rewrite" verex_rewrite_instance
  | verex_timed "s-grind" verex_grind))

/-- Decide a normalized leaf case by case over the values of an integer that
ranges over a few literals (`verex_split_range`), each case settled
and decided as the prepared leaf is. -/
syntax "verex_decide_ranges" : tactic

/-- Run a step, then state the hypotheses it added in the normal form of
its rewriting (`verex_simp_all [vir_denote_norm]`), by the whole context.
The rest of the context and the goal, normal already, are not rewritten
again. -/
elab "verex_normalizing_stated " label:str step:tactic : tactic => do
  let mut before : FVarIdSet := {}
  for goal in ← getGoals do
    before ← goal.withContext do
      return (← getLCtx).foldl (init := before) fun known decl => known.insert decl.fvarId
  evalTactic step
  let call ← `(tactic| verex_simp_all [vir_denote_norm])
  let mut remaining := #[]
  for goal in ← getGoals do
    let stated ← goal.withContext do
      return (← getLCtx).foldl (init := ({} : FVarIdSet)) fun stated decl =>
        if decl.isImplementationDetail || before.contains decl.fvarId then stated
        else stated.insert decl.fvarId
    if stated.isEmpty then
      remaining := remaining.push goal
      continue
    try
      timedStep label.getString do setGoals ((← simpAllAt goal call stated).toList)
      remaining := remaining ++ (← getGoals)
    catch _ => remaining := remaining.push goal
  setGoals remaining.toList

/-- The general leaf: one pipeline, each stage over the previous stage's
goals, so that a rewriting pass is never repeated for a later alternative.
Its stages report under `pipe-` labels (`verex_timed`), so a leaf's
cost is attributed past the pipeline as a whole. -/
macro "verex_pipeline" : tactic => `(tactic| verex_focus ( (
  verex_subst_vars
  try verex_split_search
  all_goals verex_reduce_projections
  all_goals verex_timed "pipe-simp" (verex_bounded
    (try simp (disch := omega) only [VerexIR.Proofs.Obligation_eq,
      Nat.reduceAdd, Int.reducePow, Int.reduceSub, Nat.reducePow, Nat.reduceSub,
      vir_denote_norm] at *))
  all_goals first
  | done
  | verex_timed "pipe-assumption" verex_assumption
  | verex_timed "pipe-omega" verex_linear
  -- Positions that range over a few literal values, case by case.
  | verex_timed "pipe-ranges" verex_decide_ranges
  | (verex_timed "pipe-round" verex_saturate_round
     all_goals (first
       | done
       | verex_timed "pipe-bounds-omega" (verex_bounds; verex_linear)
       -- An instance of a quantified hypothesis once the lookups after
       -- writes are read, before the context is saturated.
       | verex_timed "pipe-instance" verex_instance
       | verex_timed "pipe-range-instance" verex_range_instance
       | verex_timed "pipe-witness" verex_witness
       -- A lookup at a position the context does not tell apart from a
       -- written one, in each case.
       | (verex_timed "pipe-split-write" verex_split_write
          all_goals verex_decide_split)
       | (verex_timed "pipe-saturate" verex_saturate
          verex_timed "pipe-split" (verex_inheriting
            (all_goals (try (verex_split_conditional <;> (try verex_simp_all [vir_denote_norm])))))
          verex_timed "pipe-split" (verex_inheriting
            (all_goals (try (verex_split_conditional <;> (try verex_simp_all [vir_denote_norm])))))
          verex_timed "pipe-split-goal"
            (verex_inheriting (all_goals verex_split_goal))
          -- The facts the round stated, a position's under its bound, in the
          -- round's normal form, then what an implication among them
          -- concludes where its premises hold.
          verex_normalizing_stated "pipe-normalize" (verex_timed "pipe-round" verex_saturate_round)
          all_goals try verex_timed "pipe-modus-ponens"
            (verex_residual_budgeted verex_modus_ponens)
          -- Last, products and congruence, which `omega` reads as atoms, and
          -- operations whose inputs saturation fixed to literals; the cheap
          -- deciders include bounded arithmetic and ground evaluation.
          all_goals (first
            | verex_timed "pipe-cheap" verex_decide_cheap
            | verex_timed "pipe-bv" verex_bv
            | verex_timed "pipe-quotient" verex_quotient
            | verex_timed "pipe-grind" verex_grind)))))))

/-- A value the arithmetic rules named, with its definition, if a hypothesis
defines one: `named.val = e` for a local of type `SpecInt` itself. -/
private def namedDefinition? (decl : Lean.LocalDecl) : MetaM (Option (FVarId × Lean.Expr)) := do
  if decl.isImplementationDetail then return none
  let type ← instantiateMVars decl.type
  let some (_, lhs, rhs) := type.eq? | return none
  unless lhs.isAppOfArity ``VerexIR.SpecInt.val 3 do return none
  let .fvar result := lhs.appArg! | return none
  unless (← instantiateMVars (← result.getType)).isAppOfArity ``VerexIR.SpecInt 2 do
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
    if sub.isAppOfArity ``VerexIR.SpecInt.val 3 then
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
elab "verex_unname_positions" : tactic => focusGoal do
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
elab "verex_unname" : tactic => focusGoal do
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
        let uses (hidden : Bool) : MetaM Bool := next.withContext do
          if !hidden && (← next.getType).containsFVar result then return true
          return (← getLCtx).any fun decl =>
            decl.isImplementationDetail == hidden &&
              decl.fvarId != equation && decl.fvarId != result &&
              (decl.type.containsFVar result || decl.value?.any (·.containsFVar result))
        -- A step proof's scope entry (hidden) reading the name keeps it, and
        -- its definition hidden: the leaf reads the expression, so no rule
        -- folds the expression back into the name.
        goal ← if ← uses false then pure next
          else if ← uses true then do
            next.withContext do next.setFVarKind equation .implDetail
            pure next
          else (← next.clear equation).tryClear result
  replaceMainGoal [goal]

/-- Maps and address vectors connected by a quantified coverage hypothesis.
Only inspect hypotheses that visibly mention membership: callee contracts
and saved continuations are not candidates for this finite-map argument. -/
private def mapCoverageCandidates (goal : MVarId) : MetaM (Array (Lean.Expr × Lean.Expr)) :=
  goal.withContext do
    let mut candidates := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let type ← instantiateMVars decl.type
      unless type.isForall && (type.find? (·.isConstOf ``VerexIR.SpecMap.hasKey)).isSome do continue
      let originalContext ← getLCtx
      let candidate ← forallTelescopeReducing type fun _ body => do
        let some (_, lhs, rhs) := body.eq? | return none
        unless lhs.isAppOfArity ``VerexIR.SpecMap.hasKey 5 && rhs.isConstOf ``Bool.true do
          return none
        let some lookup := lhs.appArg!.find? (·.isAppOfArity ``GetElem?.getElem? 7) | return none
        let array := lookup.getArg! 5
        let map := lhs.getArg! 3
        -- Binder-dependent maps/vectors do not enumerate one fixed map.
        if array.hasLooseBVars || map.hasLooseBVars then return none
        unless ((collectFVars {} array).fvarIds ++ (collectFVars {} map).fvarIds).all
            (fun id => originalContext.contains id) do return none
        return some (map, array)
      if let some candidate := candidate then
        unless candidates.contains candidate do candidates := candidates.push candidate
    return candidates

/-- Derive a contradiction from a failed search in a complete map-key
vector, or prove coverage after removing the matching element and key.
Every premise is discharged from the current leaf; incomplete or duplicate
vectors do not justify either argument. -/
syntax "verex_map_coverage_prepared" : tactic

elab "verex_map_coverage" : tactic => focusGoal do
  let goal ← getMainGoal
  let mentionsMap ← goal.withContext do
    pure ((← instantiateMVars (← goal.getType)).find? (·.isConstOf ``VerexIR.SpecMap.hasKey)).isSome
  unless mentionsMap do throwError "the goal states no map membership"
  evalTactic (← `(tactic| verex_budgeted (
    verex_prepare_leaf
    all_goals verex_map_coverage_prepared
    done)))

elab_rules : tactic
  | `(tactic| verex_map_coverage_prepared) => focusGoal do
    let goal ← getMainGoal
    for (map, array) in ← mapCoverageCandidates goal do
      let attempt ← saveState
      try
        let proof ← goal.withContext do
          let rule ← mkConstWithFreshMVarLevels ``map_key_search_absurd
          let (args, _, _) ← forallMetaTelescope (← inferType rule)
          -- After the key and value types and the keys' equality: the map
          -- and the vector.
          unless ← isDefEq args[3]! map do throwError "map"
          unless ← isDefEq args[4]! array do throwError "vector"
          -- Membership determines the missing key, the coverage the
          -- fallback; the cardinality, distinctness and failed-search facts
          -- then have concrete types.
          for i in #[10, 6, 7, 8, 11] do
            setGoals [args[i]!.mvarId!]
            evalTactic (← `(tactic| verex_assumption))
          instantiateMVars (mkAppN rule args)
        goal.withContext do
          goal.assign (mkApp2 (mkConst ``False.elim [Lean.Level.zero]) (← goal.getType) proof)
        setGoals []
        return
      catch _ => attempt.restore
      try
        let rule ← goal.withContext do
          let rule ← mkConstWithFreshMVarLevels ``map_keys_after_remove
          let (args, _, _) ← forallMetaBoundedTelescope (← inferType rule) 5
          unless ← isDefEq args[3]! map do throwError "map"
          unless ← isDefEq args[4]! array do throwError "vector"
          instantiateMVars (mkAppN rule args)
        setGoals (← goal.apply rule)
        evalTactic (← `(tactic| all_goals verex_assumption))
        evalTactic (← `(tactic| done))
        return
      catch _ => attempt.restore
    throwError "no complete map-key enumeration or matching removal"

/-- Cheap deciders, also available explicitly to an authored proof. Bound
the attempt: even omega can branch heavily on a prepared map's conditional
facts. An authored script should not spend its budget on a failed decider. -/
macro "verex_decide_residual" : tactic => `(tactic| verex_focus ( verex_residual_budgeted (
  first
  | done
  | verex_timed "residual omega" omega
  | verex_timed "residual cheap" verex_decide_cheap
  | verex_timed "residual instance" verex_instance
  | verex_timed "residual range" verex_range_instance
  | verex_timed "residual context" (
      verex_simp_by_context
      first | done | (verex_bounds; omega))
  | verex_trivial)))

macro_rules
  | `(tactic| verex_decide_by_context) => `(tactic| verex_focus (
  (verex_timed "ctx-simp" verex_simp_by_context
   first
   | done
   | (verex_timed "ctx-bounds" verex_bounds
      verex_timed "ctx-omega" omega))))

/-- The deciders over a prepared leaf: the goal as a decided instance of the
context, with the lookups after writes read, in each case of a position not
told apart from a written one. -/
syntax "verex_decide_written" : tactic

macro_rules
  | `(tactic| verex_decide_written) => `(tactic| verex_focus (
  first
  | done
  | verex_timed "w-omega" verex_omega_unsplit
  -- A lookup at a position the context does not tell apart from a written
  -- one, in each case, before the deciders that would read it undecided.
  | (verex_timed "w-split" verex_split_write
     all_goals verex_timed "ws" verex_decide_split)
  | verex_timed "w-cheap" verex_decide_cheap
  | verex_timed "w-instance" verex_instance
  | verex_timed "w-refute" (exfalso; verex_instance)
  | verex_timed "w-range" verex_range_instance
  | verex_timed "w-context" verex_decide_by_context
  | verex_timed "w-trivial" verex_trivial
  -- A conjunction, such as a clause with an abort code, conjunct by
  -- conjunct.
  | verex_timed "w-and" (apply And.intro <;> verex_decide_written)
  | (verex_timed "w-lookups" verex_lookups_after_writes
     first
     | done
     | verex_timed "wl-omega" verex_omega_unsplit
     | verex_timed "wl-instance" verex_instance
     | verex_timed "wl-range" verex_range_instance)
  | (verex_timed "w-positions" (verex_intro_negation; verex_instantiate_positions)
     first
     | done
     | omega
     | verex_instance)))

/-- Whether a quantified hypothesis ranges over the values of an integer
type, such as a map's keys: its integer binder bounded by the type's range
alone, the normal form of the membership a clause quantifies with
(`IntegerValueFits_unsigned_succ`, `IntegerValueFits_signed_succ`). -/
private def rangesOverIntegerType (type : Lean.Expr) : MetaM Bool := do
  let .forallE _ _ (.forallE _ low (.forallE _ high _ _) _) _ := type | return false
  let_expr LE.le _ _ lower bound := low | return false
  let_expr LE.le _ _ bound' upper := high | return false
  unless bound == .bvar 0 && bound' == .bvar 1 do return false
  let some lower ← getIntValue? lower | return false
  let some upper ← getIntValue? upper | return false
  let size := upper + 1
  let powerOfTwo := 0 < size && (size.toNat &&& (size.toNat - 1)) == 0
  return powerOfTwo && (lower == 0 || (lower == -size && 1 < size))

/-- Whether the written deciders can decide the leaf at all: they decide
instances of hypotheses quantified over integer positions, lookups after
writes, and the position facts produced by searches. A leaf with none of
these (a storage leaf, or one whose quantifiers range over the values of a
type, such as keys) is prepared only when it holds atoms `grind` relates
(`verex_has_atoms`), and is otherwise left to the pipeline, whose normalization would only be paid
twice. -/
elab "verex_prepared_applies" : tactic => focusGoal do
  let goal ← getMainGoal
  let applies ← goal.withContext do
    let needsPreparation (e : Lean.Expr) : Bool :=
      e.isConstOf ``Array.setIfInBounds || e.isConstOf ``VerexIR.SpecVector.set ||
      e.isConstOf ``Array.eraseIdxIfInBounds || e.isConstOf ``Array.insertIdxIfInBounds ||
      e.isConstOf ``findIndex?
    if ((← instantiateMVars (← goal.getType)).find? needsPreparation).isSome then return true
    (← getLCtx).anyM fun decl => do
      if decl.isImplementationDetail then return false
      let ty ← instantiateMVars decl.type
      if ← (pure ty.isForall <&&> isProp ty) then
        if (← whnfR ty.bindingDomain!).isConstOf ``Int && !(← rangesOverIntegerType ty) then
          return true
      return (ty.find? needsPreparation).isSome
  unless applies do
    throwError "the leaf has no position hypothesis, vector write, or search"

/-- Whether a leaf holds terms omega reads as atoms that `grind` relates:
observations of a map value (`hasKey`, `valueAt`, `keyAt`, `size`, `rank`),
which the map lemmas relate, behavioral predicates (`ensures_of`, `aborts_of`,
`result_of`, `state_of`), whose lemmas the contracts and invariants state, or
applications of a recursive specification function (`f.spec`), equal at
equal arguments. -/
elab "verex_has_atoms" : tactic => focusGoal do
  let goal ← getMainGoal
  let env ← getEnv
  let found ← goal.withContext do
    let isAtom (e : Lean.Expr) : Bool :=
      e.isConstOf ``VerexIR.SpecMap.hasKey || e.isConstOf ``VerexIR.SpecMap.valueAt ||
      e.isConstOf ``VerexIR.SpecMap.keyAt || e.isConstOf ``VerexIR.SpecMap.size ||
      e.isConstOf ``VerexIR.SpecMap.rank ||
      e.isConstOf ``VerexIR.Proofs.EnsuresOf || e.isConstOf ``VerexIR.Proofs.AbortsOf ||
      e.isConstOf ``VerexIR.Proofs.ResultOf || e.isConstOf ``VerexIR.Proofs.StateOf ||
      (match e with
        | .const name _ => (specDefinition? env name).isSome
        | _ => false)
    if ((← instantiateMVars (← goal.getType)).find? isAtom).isSome then return true
    (← getLCtx).anyM fun decl => do
      if decl.isImplementationDetail then return false
      return ((← instantiateMVars decl.type).find? isAtom).isSome
  unless found do throwError "the leaf observes no map and applies no recursive specification function"

/-- The memory after a call, where a hypothesis is a frame of the call: global
memory reads the same after it as before, at every key or resource type it
ranges over, or everywhere. -/
private def frameAfter? (type : Lean.Expr) : MetaM (Option Lean.Expr) :=
  forallTelescope type fun _ body => do
    let_expr Eq _ after before := body | return none
    let afterHead := after.getAppFn
    let beforeHead := before.getAppFn
    unless afterHead.isFVar && beforeHead.isFVar && afterHead != beforeHead &&
        after.getAppArgs == before.getAppArgs do
      return none
    unless (← whnfR (← inferType afterHead)).isAppOfArity ``VerexIR.Proofs.Denote.Memory 2 do
      return none
    return some afterHead

/-- Decide that two memory slots differ, from the context. -/
macro "verex_keys_differ" : tactic => `(tactic| verex_focus ( (
  -- A disequality of variables has no constructors to take apart: the
  -- context states it, as `symmetricDisequalities` does either way round.
  try simp only [ne_eq, VerexIR.Proofs.Denote.TableMemory.value_ne_allocationResource,
    VerexIR.Proofs.Denote.TableMemory.allocationResource_ne_value,
    VerexIR.Proofs.Denote.ResourceType.mk.injEq, VerexIR.Proofs.Denote.NTy.struct.injEq,
    VerexIR.Proofs.Denote.NTy.enum.injEq,
    VerexIR.Proofs.Denote.NRow.cons.injEq, VerexIR.StructHandle.mk.injEq,
    VerexIR.NamespaceId.mk.injEq, VerexIR.StorageKey.bool.injEq,
    VerexIR.StorageKey.character.injEq, VerexIR.StorageKey.integer.injEq,
    VerexIR.StorageKey.address.injEq, VerexIR.StorageKey.signer.injEq,
    VerexIR.StorageKey.string.injEq, VerexIR.StorageKey.bytes.injEq, reduceCtorEq, true_and,
    and_true, false_and, and_false, not_false_eq_true]
  first
  | done
  | assumption
  -- A discharger runs at reducible transparency, which does not unfold `¬`.
  | with_unfolding_all (intro equal; exact absurd equal.symm ‹_›)
  | omega)))

/-- A discharger answering a condition it was asked before, as each hypothesis
simp rewrites asks the same conditions again; a proof found under a binder simp
entered may read its local and is not kept. -/
private def rememberingDischarge (discharge : Simp.Discharge) : MetaM Simp.Discharge := do
  let context ← getLCtx
  let decided ← IO.mkRef ({} : Std.HashMap Lean.Expr (Option Lean.Expr))
  return fun condition => do
    let condition ← instantiateMVars condition
    if let some known := (← decided.get)[condition]? then return known
    let result ← discharge condition
    if result.isNone || (← getLCtx).decls.size == context.decls.size then
      decided.modify (·.insert condition result)
    return result

/-- Read each global a call leaves alone at the state before the call: the
frames of the calls rewrite the goal, and what the leaf states of the memory
after a call, where the context decides that a key is not one the callee
modifies. -/
elab "verex_frames" : tactic => focusGoal do
  let goal ← getMainGoal
  goal.withContext do
    let mut lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) := #[]
    let mut later : Array Lean.Expr := #[]
    let mut facts : Array Lean.LocalDecl := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      let type ← instantiateMVars declaration.type
      if let some after ← frameAfter? type then
        lemmas := lemmas.push
          (← `(Lean.Parser.Tactic.simpLemma| $(← Lean.Elab.Term.exprToSyntax declaration.toExpr):term))
        later := later.push after
      else if ← isProp type then
        facts := facts.push declaration
    if lemmas.isEmpty then throwError "the leaf has no frame of a call"
    let reading := facts.filter fun declaration =>
      (declaration.type.find? fun e => e.isApp && later.contains e.getAppFn).isSome
    let { ctx, simprocs, dischargeWrapper, .. } ← mkSimpContext
      (← `(tactic| simp (disch := verex_keys_differ) only [$lemmas,*])) (eraseLocal := false)
    let (result, _) ← dischargeWrapper.with fun discharge? => do
      let remembering ← discharge?.mapM (fun discharge => (rememberingDischarge discharge : MetaM _))
      simpGoal goal ctx simprocs remembering (fvarIdsToSimp := reading.map (·.fvarId))
    replaceMainGoal (result.toList.map (·.2))

/-- The facts a proof states: its conjuncts, recursively, each with its
type. -/
partial def typedConjuncts (proof : Lean.Expr) : MetaM (Array (Lean.Expr × Lean.Expr)) := do
  let type ← whnfR (← instantiateMVars (← inferType proof))
  if let some (left, right) := type.and? then
    return (← typedConjuncts (mkApp3 (mkConst ``And.left) left right proof)) ++
      (← typedConjuncts (mkApp3 (mkConst ``And.right) left right proof))
  return #[(proof, type)]

/-- The facts a proof states: its conjuncts, recursively. -/
def conjuncts (proof : Lean.Expr) : MetaM (Array Lean.Expr) :=
  return (← typedConjuncts proof).map (·.1)

/-- A proof of the stored invariants of a memory from the context: a
hypothesis or a conjunct of one stating them, or an implication concluding
them whose premise has such a proof, as a function preserving them states. -/
-- Retired with doc/plan/data-and-global-invariants.md, "The stored-invariant layer
-- still exists for the declarations the carriers do not state": the carrier replaces it.
private partial def storedInvariantsProof? (memory : Lean.Expr) : MetaM (Option Lean.Expr) := do
  -- Index the finite context once. A fixed recursion depth loses preservation
  -- chains after enough calls; revisiting the context recursively also repeats
  -- work on shared predecessors. Key by the whole proposition, so different
  -- invariant predicates at the same memory cannot be mixed.
  let known ← IO.mkRef ({} : Std.HashMap Lean.Expr Lean.Expr)
  let edges ← IO.mkRef ({} : Std.HashMap Lean.Expr (Array (Lean.Expr × Lean.Expr)))
  let roots ← IO.mkRef (#[] : Array Lean.Expr)
  let isInvariant (type : Lean.Expr) :=
    type.isAppOfArity ``VerexIR.Proofs.Denote.DataInvariants 4
  let rec collect (proof type : Lean.Expr) : MetaM Unit := do
    checkSystem "stored invariants"
    if isInvariant type then
      known.modify (·.insert type proof)
      if type.getArg! 3 == memory then roots.modify (·.push type)
    else if let some (left, right) := type.and? then
      collect (mkApp3 (mkConst ``And.left) left right proof) left
      collect (mkApp3 (mkConst ``And.right) left right proof) right
    else if type.isArrow && isInvariant type.bindingDomain! && isInvariant type.bindingBody! then
      let premise := type.bindingDomain!
      let conclusion := type.bindingBody!
      edges.modify fun index => index.insert conclusion
        ((index[conclusion]?.getD #[]).push (proof, premise))
      if conclusion.getArg! 3 == memory then roots.modify (·.push conclusion)
  for declaration in ← getLCtx do
    unless declaration.isImplementationDetail do
      collect declaration.toExpr (← instantiateMVars declaration.type)
  let rec prove (statement : Lean.Expr) (visiting : Std.HashSet Lean.Expr) :
      MetaM (Option Lean.Expr) := do
    checkSystem "stored invariants"
    if let some proof := (← known.get)[statement]? then return some proof
    if visiting.contains statement then return none
    let visiting := visiting.insert statement
    for (implication, premise) in (← edges.get)[statement]?.getD #[] do
      if let some held ← prove premise visiting then
        let proof := mkApp implication held
        known.modify (·.insert statement proof)
        return some proof
    return none
  for statement in ← roots.get do
    if let some proof ← prove statement {} then return some proof
  return none

/-- That a stored value's resource arguments are inhabitable, as the case of
its invariant reads them (`StoredCase`): evaluated, or through the type
arguments they are read from. -/
private def argumentsInhabitable? (resource : Lean.Expr) : TacticM (Option Lean.Expr) := do
  let statement ← mkEq (mkApp (mkConst ``NRow.inhabitable)
    (mkApp (mkConst ``ResourceType.arguments) resource)) (mkConst ``Bool.true)
  let proof ← mkFreshExprMVar statement
  let saved ← getGoals
  try
    setGoals [proof.mvarId!]
    evalTactic (← `(tactic| first
      | rfl
      | (simp (config := { decide := true }) only [NRow.inhabitable, NTy.inhabitable,
          NRows.firstInhabitable, TypeArgs.getD_inhabitable, TypeArgs.subst_inhabitable, NRow.map,
          Skolems.resource, Skolems.resolve_runtime, Skolems.resolve_instantiate, Bool.and_true,
          Bool.true_and, Bool.and_self]
         done)))
    if (← getGoals).isEmpty then return some (← instantiateMVars proof) else return none
  catch _ => return none
  finally setGoals saved

/-- A unit's runtime frame: `Skolems.runtime`, which takes the runtime
family's inhabitants as well, synthesized here — in a leaf they come from
the frame in the context (`Skolems.runtimeCarriersNonempty`). -/
private def runtimeFrame (unit functions : Lean.Expr) : MetaM Lean.Expr := do
  let family := mkApp2 (mkConst ``VerexIR.Proofs.Denote.Carriers.runtime) unit functions
  let nonempty ← synthInstance (mkApp (mkConst ``VerexIR.Proofs.Denote.CarriersNonempty) family)
  return mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.runtime) unit functions nonempty

/-- The frame a resource is read at: the instance's own type arguments over
the runtime frame where they are decided inhabitable, otherwise the frame
whose resolution spells the resource (a reader at a generic frame). -/
private def resourceFrame? (unit functions σ arguments resource : Lean.Expr) :
    TacticM (Option (Lean.Expr × Option Lean.Expr)) := do
  let atFrame (frame : Lean.Expr) : TacticM Bool := do
    let atInstance := mkApp5 (mkConst ``Skolems.resource) unit functions frame σ arguments
    withDefault (isDefEq resource atInstance)
  if let some inhabitable ← argumentsInhabitable? resource then
    let predicate ← withLocalDeclD `row (mkConst ``NRow) fun row => do
      mkLambdaFVars #[row] (← mkEq (mkApp (mkConst ``NRow.inhabitable) row) (mkConst ``Bool.true))
    -- The instance at the resource's arguments as a row, which the
    -- transports to its frame evaluate (`ofRuntimeOuterFrame`).
    let resourceArguments ← whnfR (mkApp (mkConst ``ResourceType.arguments) resource)
    let θ := mkApp4 (mkConst ``Subtype.mk [Level.one]) (mkConst ``NRow) predicate
      resourceArguments inhabitable
    let frame := mkApp4 (mkConst ``Skolems.instantiate) unit functions θ
      (← runtimeFrame unit functions)
    if ← atFrame frame then return some (frame, some θ)
  let type ← whnfR (mkApp (mkConst ``ResourceType.type) resource)
  let_expr Skolems.resolve _ _ frame _ := type | return none
  if ← atFrame frame then return some (frame, none) else return none

/-- A stored value's invariant (`StoredCase`) as the case of the value at its
resource's type arguments, from the frame it is read or written at
(`resourceFrame?`): the frame they induce, where the resource spells it, or
an abstract frame's view of the value moved there (`StoredCase.atFrame`).
The case's statement, the stored invariant from a proof of it, and it from
the stored invariant; none where the resource is not one of the
declaration's. -/
-- Retired with doc/plan/data-and-global-invariants.md, "The stored-invariant layer
-- still exists for the declarations the carriers do not state": the carrier replaces it.
private def storedCase? (type : Lean.Expr) (direct : Bool := false) :
    TacticM (Option (Lean.Expr × (Lean.Expr → MetaM Lean.Expr) × (Lean.Expr → MetaM Lean.Expr))) := do
  -- A stored case takes the runtime family's inhabitants as well
  -- (`StoredCase`), so it is spelled with eight arguments.
  unless type.isAppOfArity ``StoredCase 8 do return none
  let #[unit, functions, nonempty, σ, arguments, holds, resource, value] := type.getAppArgs
    | return none
  let some (frame, θ?) ← resourceFrame? unit functions σ arguments resource | return none
  match θ? with
  | some θ =>
      let atInstance := mkApp5 (mkConst ``Skolems.resource) unit functions frame σ arguments
      let same ← mkExpectedTypeHint (← mkEqRefl resource) (← mkEq resource atInstance)
      -- A read of a declaration without type parameters (`direct`) is of the
      -- stored value itself, as its invariant is stated at the runtime frame.
      let read := mkApp5 (mkConst ``Skolems.ofRuntime) unit functions frame σ value
      let read ← if direct && arguments.isConstOf ``NRow.nil then do
          if ← withDefault (isDefEq read value) then pure value else pure read
        else pure read
      let statement := mkApp2 holds θ read
      let inhabitable := θ.appArg!
      return some (statement,
        fun held => mkAppOptM ``StoredCase.intro
          #[some unit, some functions, some nonempty, none, none, none, none, none,
            some inhabitable, some same, some held],
        fun fact => mkExpectedTypeHint (mkApp2 fact inhabitable same) statement)
  | none =>
      -- The declaration's parameters and the bound of its type, decided.
      let mut atFrame := mkAppN (mkConst ``StoredCase.atFrame)
        #[unit, functions, nonempty, frame, σ, arguments, holds]
      for _ in [0:2] do
        let .forallE _ condition _ _ ← whnfR (← inferType atFrame) | return none
        let some decided ← decidedFrameFree? condition | return none
        atFrame := mkApp atFrame decided
      let iff := mkApp atFrame value
      let stated ← whnfR (← inferType iff)
      let_expr Iff _ statement := stated | return none
      return some (statement, fun held => mkAppM ``Iff.mpr #[iff, held],
        fun fact => mkAppM ``Iff.mp #[iff, fact])

section
variable {unit : Validation.ValidatedUnit} [functions : FunctionCarriers unit]
  [CarriersNonempty (Carriers.runtime unit)] (Θ : Skolems unit)
  {params : NRow} (own : params.isParamsFrom 0 = true)

theorem paramsAll_table_key {allowed : Nat → Bool} {key value : NTy}
    (bounded : (NTy.table key value).paramsAll allowed = true) : key.paramsAll allowed = true := by
  simp only [NTy.paramsAll, Bool.and_eq_true] at bounded; exact bounded.1

theorem paramsAll_table_value {allowed : Nat → Bool} {key value : NTy}
    (bounded : (NTy.table key value).paramsAll allowed = true) : value.paramsAll allowed = true := by
  simp only [NTy.paramsAll, Bool.and_eq_true] at bounded; exact bounded.2

/-- A table's entries move with it. -/
theorem Skolems.reframe_table_entries {key value : NTy} (bounded)
    (table : @NTy.carrier Θ.toCarriers (.table key value)) :
    (Θ.reframe own (τ := .table key value) bounded table).entries =
      table.entries.map fun entry =>
        (Θ.reframe own (paramsAll_table_key bounded) entry.1,
         Θ.reframe own (paramsAll_table_value bounded) entry.2) := by
  let Φ := (Θ.instance params own).toCarriers
  have encoded := Θ.encode_reframe own bounded table
  rw [@NTy.encode_table Φ, @NTy.encode_table Θ.toCarriers] at encoded
  simp only [RuntimeValue.vector.injEq] at encoded
  have encoded := List.toArray_inj encoded
  rw [show (table.entries.map fun entry =>
        RuntimeValue.tuple #[@NTy.encode Θ.toCarriers key entry.1, @NTy.encode Θ.toCarriers value entry.2]) =
      (table.entries.map fun entry =>
        (Θ.reframe own (paramsAll_table_key bounded) entry.1,
         Θ.reframe own (paramsAll_table_value bounded) entry.2)).map
        (fun entry => RuntimeValue.tuple #[@NTy.encode Φ key entry.1, @NTy.encode Φ value entry.2]) by
      simp only [List.map_map, Function.comp_def, Φ, Θ.encode_reframe]] at encoded
  have injective : ∀ a b : @NTy.carrier Φ key × @NTy.carrier Φ value,
      RuntimeValue.tuple #[@NTy.encode Φ key a.1, @NTy.encode Φ value a.2] =
      RuntimeValue.tuple #[@NTy.encode Φ key b.1, @NTy.encode Φ value b.2] → a = b := by
    intro a b same
    simp only [RuntimeValue.tuple.injEq, Array.mk.injEq, List.cons.injEq, and_true] at same
    exact Prod.ext (@NTy.encode_injective Φ key _ _ same.1) (@NTy.encode_injective Φ value _ _ same.2)
  exact List.map_inj_right (fun a b same => injective a b same) |>.mp encoded

/-- The validity of a sequence map, under injective reads of its keys, is that
of the keys: it does not see their move to the frame of the type arguments. -/
theorem Skolems.valid_sequence_reframe {key value : NTy} (bounded)
    (table : @NTy.carrier Θ.toCarriers (.table key value))
    {K V K' V' : Type} (readKey : @NTy.carrier (Θ.instance params own).toCarriers key → K)
    (readValue : @NTy.carrier (Θ.instance params own).toCarriers value → V)
    (readKey' : @NTy.carrier Θ.toCarriers key → K') (readValue' : @NTy.carrier Θ.toCarriers value → V')
    (injective : Function.Injective readKey) (injective' : Function.Injective readKey') :
    SpecMap.Valid .sequence
        (SpecMap.ofEntries readKey readValue (Θ.reframe own (τ := .table key value) bounded table)) ↔
      SpecMap.Valid .sequence (SpecMap.ofEntries readKey' readValue' table) := by
  simp only [SpecMap.Valid, SpecMap.Distinct, SpecMap.ofEntries, Skolems.reframe_table_entries,
    List.map_map, List.Nodup, List.pairwise_map, Function.comp_def]
  have reads : ∀ a b, readKey (Θ.reframe own (paramsAll_table_key bounded) a) ≠
      readKey (Θ.reframe own (paramsAll_table_key bounded) b) ↔ readKey' a ≠ readKey' b := by
    intro a b
    refine not_congr ⟨fun same => ?_, fun same => by rw [injective' same]⟩
    rw [(Θ.reframe_inj own _ a b).mp (injective same)]
  simp only [reads]

/-- A lent table moves each key and value to the declaration's frame. -/
theorem Skolems.plainReframe_table_entries {key value : NTy} (bounded)
    (entries : @NTy.plain Θ.toCarriers (.table key value)) :
    (Θ.plainReframe own (τ := .table key value) bounded entries).entries =
      entries.entries.map (fun entry =>
        (Θ.plainReframe own (τ := key)
          (by simp only [NTy.paramsAll, Bool.and_eq_true] at bounded; exact bounded.1) entry.1,
         Θ.plainReframe own (τ := value)
          (by simp only [NTy.paramsAll, Bool.and_eq_true] at bounded; exact bounded.2) entry.2)) := by
  have encoded := Θ.plainEncode_plainReframe own bounded entries
  rw [@NTy.plainEncode_table (Θ.instance params own).toCarriers,
    @NTy.plainEncode_table Θ.toCarriers] at encoded
  simp only [RuntimeValue.vector.injEq] at encoded
  have encoded := List.toArray_inj encoded
  let encode := fun entry : @NTy.plain (Θ.instance params own).toCarriers key ×
      @NTy.plain (Θ.instance params own).toCarriers value =>
    RuntimeValue.tuple #[@NTy.plainEncode (Θ.instance params own).toCarriers key entry.1,
      @NTy.plainEncode (Θ.instance params own).toCarriers value entry.2]
  apply List.map_inj_right (f := encode) (fun left right same => ?_) |>.mp
  · rw [List.map_map]
    refine encoded.trans ?_
    apply List.map_congr_left
    intro entry _
    simp only [Function.comp_apply, encode, Θ.plainEncode_plainReframe]
  · simp only [encode, RuntimeValue.tuple.injEq, Array.mk.injEq, List.cons.injEq,
      and_true] at same
    exact Prod.ext
      (@NTy.plainEncode_injective (Θ.instance params own).toCarriers key _ _ same.1)
      (@NTy.plainEncode_injective (Θ.instance params own).toCarriers value _ _ same.2)



theorem plainReframe_injective {τ : NTy} (bounded) :
    Function.Injective (Θ.plainReframe own (τ := τ) bounded) := by
  intro left right same
  apply @NTy.plainEncode_injective Θ.toCarriers τ
  rw [← Θ.plainEncode_plainReframe own bounded left,
    ← Θ.plainEncode_plainReframe own bounded right, same]


/-- Sequence validity is independent of the frame of the plain table. -/
theorem valid_sequence_plainReframe {key value : NTy} (bounded)
    (table : @NTy.plain Θ.toCarriers (.table key value))
    {K V K' V' : Type} (readKey : @NTy.plain (Θ.instance params own).toCarriers key → K)
    (readValue : @NTy.plain (Θ.instance params own).toCarriers value → V)
    (readKey' : @NTy.plain Θ.toCarriers key → K') (readValue' : @NTy.plain Θ.toCarriers value → V')
    (injective : Function.Injective readKey) (injective' : Function.Injective readKey') :
    SpecMap.Valid .sequence
        (SpecMap.ofEntries readKey readValue (Θ.plainReframe own (τ := .table key value) bounded table)) ↔
      SpecMap.Valid .sequence (SpecMap.ofEntries readKey' readValue' table) := by
  simp only [SpecMap.Valid, SpecMap.Distinct, SpecMap.ofEntries,
    Skolems.plainReframe_table_entries, List.map_map, List.Nodup, List.pairwise_map,
    Function.comp_def]
  have reads : ∀ a b,
      readKey (Θ.plainReframe own (τ := key)
        (by simp only [NTy.paramsAll, Bool.and_eq_true] at bounded; exact bounded.1) a) ≠
      readKey (Θ.plainReframe own (τ := key)
        (by simp only [NTy.paramsAll, Bool.and_eq_true] at bounded; exact bounded.1) b) ↔
      readKey' a ≠ readKey' b := by
    intro a b
    refine not_congr ⟨fun same => ?_, fun same => by rw [injective' same]⟩
    rw [plainReframe_injective Θ own _ (injective same)]
  simp only [reads]



theorem read_ofDeepHolds [Φ : Carriers] [CarriersNonempty Φ]
    (τ : NTy) (inhabitable : τ.inhabitable = true) (value : τ.plain)
    (held : NTy.deepHolds τ value) :
    NTy.read τ (NTy.ofDeepHolds τ value held) = NTy.readPlain τ inhabitable value := by
  rw [← NTy.readPlain_forget τ inhabitable, NTy.forget_ofDeepHolds]

theorem ofEntries_read_ofDeepHolds [Φ : Carriers] [CarriersNonempty Φ]
    (key value : NTy) (keyHeld : key.inhabitable = true) (valueHeld : value.inhabitable = true)
    (table : (NTy.table key value).plain) (held : NTy.deepHolds (.table key value) table) :
    SpecMap.ofEntries (NTy.read key) (NTy.read value)
        (NTy.ofDeepHolds (.table key value) table held) =
      SpecMap.ofEntries (NTy.readPlain key keyHeld) (NTy.readPlain value valueHeld) table :=
  SpecMap.ofEntries_ofDeepHolds_table key value
    (NTy.readPlain key keyHeld) (NTy.readPlain value valueHeld)
    (NTy.read key) (NTy.read value)
    (read_ofDeepHolds key keyHeld) (read_ofDeepHolds value valueHeld) table held

/-- A sequence map's distinctness does not depend on the certificates of its keys. -/
theorem valid_sequence_ofDeepHolds [Φ : Carriers] {key value : NTy}
    (table : (NTy.table key value).plain) (held : NTy.deepHolds (.table key value) table)
    {K V : Type} (readKey : key.carrier → K) (readValue : value.carrier → V)
    (injective : Function.Injective readKey) :
    SpecMap.Valid .sequence (SpecMap.ofEntries readKey readValue
        (NTy.ofDeepHolds (.table key value) table held)) ↔
      SpecMap.Valid .sequence (SpecMap.ofEntries (fun x => x) (fun x => x) table) := by
  simp only [SpecMap.Valid, SpecMap.Distinct, SpecMap.ofEntries, NTy.ofDeepHolds,
    List.map_map, Function.comp_def]
  conv => rhs; rw [← List.attach_map_val (l := table.entries) (f := Prod.fst)]
  simp only [List.Nodup, List.pairwise_map]
  constructor
  · intro pairwise
    apply pairwise.imp
    intro a b different same
    apply different
    apply congrArg readKey
    apply NTy.forget_injective key
    simpa only [NTy.forget_ofDeepHolds] using same
  · intro pairwise
    apply pairwise.imp
    intro a b different same
    apply different
    have certified := injective same
    have forgotten := congrArg (NTy.forget key) certified
    simpa only [NTy.forget_ofDeepHolds] using forgotten

/-- Reading well-formed plain keys preserves and reflects their distinctness. -/
theorem valid_sequence_readPlain [Φ : Carriers] [CarriersNonempty Φ]
    (key value : NTy) (keyHeld : key.inhabitable = true) (valueHeld : value.inhabitable = true)
    (table : (NTy.table key value).plain) (held : NTy.deepHolds (.table key value) table) :
    SpecMap.Valid .sequence (SpecMap.ofEntries (NTy.readPlain key keyHeld)
        (NTy.readPlain value valueHeld) table) ↔
      SpecMap.Valid .sequence (SpecMap.ofEntries (fun x => x) (fun x => x) table) := by
  rw [← ofEntries_read_ofDeepHolds key value keyHeld valueHeld table held]
  exact valid_sequence_ofDeepHolds table held (NTy.read key) (NTy.read value)
    (NativeMaps.NTy.read_injective key)

end

open Lean Meta in
/-- The validity of a sequence map read at the frame of its resource's type
arguments (`Skolems.reframe`), at the leaf's frame with the views of its types
there. -/
simproc ↓ [vir_denote, vir_denote_norm] validReframe (SpecMap.Valid _ _) := fun e => do
  let_expr SpecMap.Valid _ _ discipline map := e | return .continue
  let_expr SpecMap.ofEntries _ _ _ _ readKey readValue table := map | return .continue
  let_expr Skolems.reframe unit functions nonempty frame params own τ bounded value := table
    | return .continue
  let_expr NTy.table key element := ← withDefault (whnf τ) | return .continue
  let carriers := mkApp3 (mkConst ``Skolems.toCarriers) unit functions frame
  let some keys ← NativeMaps.typedView? carriers key | return .continue
  let some values ← NativeMaps.typedView? carriers element | return .continue
  let instanceCarriers := mkApp3 (mkConst ``Skolems.toCarriers) unit functions
    (mkAppN (mkConst ``Skolems.instance) #[unit, functions, nonempty, frame, params, own])
  let injective (carriers τ read : Lean.Expr) : MetaM (Option Lean.Expr) := do
    let proof := mkApp2 (mkConst ``NativeMaps.NTy.read_injective) carriers τ
    let statement ← mkAppM ``Function.Injective #[read]
    unless ← withDefault (isDefEq (← inferType proof) statement) do return none
    return some (← mkExpectedTypeHint proof statement)
  unless discipline.isAppOf ``SpecMap.Discipline.sequence do return .continue
  let some onInstance ← injective instanceCarriers key readKey | return .continue
  let some onFrame ← injective carriers key keys.read | return .continue
  let statement ← mkAppOptM ``SpecMap.Valid #[keys.carrier, values.carrier,
    ← mkAppOptM ``SpecMap.Discipline.sequence #[keys.carrier],
    ← mkAppM ``SpecMap.ofEntries #[keys.read, values.read, value]]
  let iff ← mkAppOptM ``Skolems.valid_sequence_reframe
    #[unit, functions, nonempty, frame, params, own, key, element, bounded, value, none, none, none, none,
      readKey, readValue, keys.read, values.read, onInstance, onFrame]
  return .visit { expr := statement, proof? := some (← mkPropExt iff) }

/-- A proof assembled only from context facts, their applications, and the
introductions of universals and conjunctions. No arithmetic or search stage
runs while reading an entry's certificate. -/
private partial def entryHoldsProof? (type : Lean.Expr) (fuel : Nat := 8) : MetaM (Option Lean.Expr) := do
  if fuel == 0 then return none
  withTransparency .default do
  let type ← instantiateMVars type
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    if ← withNewMCtxDepth (isDefEq decl.type type) then return some decl.toExpr
  let reduced ← whnf type
  if reduced.isConstOf ``True then return some (mkConst ``True.intro)
  if let some (left, right) := reduced.app2? ``And then
    let some leftProof ← entryHoldsProof? left (fuel - 1) | return none
    let some rightProof ← entryHoldsProof? right (fuel - 1) | return none
    return some (← mkAppM ``And.intro #[leftProof, rightProof])
  if let .forallE name domain body binderInfo := reduced then
    return ← withLocalDecl name binderInfo domain fun arg => do
      let some proof ← entryHoldsProof? (body.instantiate1 arg) (fuel - 1) | return none
      return some (← mkLambdaFVars #[arg] proof)
  for decl in ← getLCtx do
    if decl.isImplementationDetail || !decl.type.isForall then continue
    let proof? ← observing? <| withNewMCtxDepth do
      let (arguments, _, conclusion) ← forallMetaTelescope decl.type
      unless ← isDefEq conclusion type do failure
      for argument in arguments do
        let argument ← instantiateMVars argument
        if argument.hasExprMVar then
          let premise ← inferType argument
          unless ← isProp premise do failure
          let some proof ← entryHoldsProof? premise (fuel - 1) | failure
          unless ← isDefEq argument proof do failure
      let proof ← instantiateMVars (mkAppN decl.toExpr arguments)
      if proof.hasExprMVar then failure
      pure proof
    if let some proof := proof? then return some proof
  return none

private def sequenceReadInjective? (carriers type reader : Lean.Expr) : MetaM (Option Lean.Expr) :=
    observing? <| withTransparency .default do
  let statement ← mkAppM ``Function.Injective #[reader]
  if let some proof ← observing? do
      forallTelescopeReducing statement fun arguments conclusion => do
        unless arguments.size == 3 do failure
        let proof := arguments[2]!
        unless ← isDefEq (← inferType proof) conclusion do failure
        mkLambdaFVars arguments proof then return proof
  let proof := mkApp2 (mkConst ``NativeMaps.NTy.read_injective) carriers type
  unless ← isDefEq (← inferType proof) statement do failure
  instantiateMVars proof

/-- Sequence validity has one spelling: the distinct plain keys of the
entries. Certification and reframing are injective; reading a plain key
is injective where its existing entry certificate holds. -/
simproc ↓ [vir_denote, vir_denote_norm] validPlainSequence (SpecMap.Valid _ _) := fun e => do
  let_expr SpecMap.Valid _ _ discipline map := e | return .continue
  unless discipline.isAppOf ``SpecMap.Discipline.sequence do return .continue
  let_expr SpecMap.ofEntries _ _ _ _ readKey readValue table := map | return .continue
  let table ← NativeMaps.certificationView table
  let application : MetaM (Option (Lean.Expr × Lean.Expr)) := observing? <| withTransparency .default do
    let proof ← if table.isAppOfArity ``NTy.ofDeepHolds 4 then do
        let_expr NTy.ofDeepHolds carriers τ entries held := table | failure
        let_expr NTy.table key value := ← whnfR τ | failure
        let some injective ← sequenceReadInjective? carriers key readKey | failure
        mkAppOptM ``valid_sequence_ofDeepHolds
          #[carriers, key, value, entries, held, none, none, readKey, readValue, injective]
      else if table.isAppOfArity ``Skolems.plainReframe 9 then do
        let_expr Skolems.plainReframe unit functions nonempty frame params own τ bounded entries := table
          | failure
        let_expr NTy.table key value := ← whnfR τ | failure
        let carriers := mkApp3 (mkConst ``Skolems.toCarriers) unit functions frame
        let frame' := mkApp6 (mkConst ``Skolems.instance) unit functions nonempty frame params own
        let carriers' := mkApp3 (mkConst ``Skolems.toCarriers) unit functions frame'
        let some injective ← sequenceReadInjective? carriers' key readKey | failure
        let identity (τ : Lean.Expr) : MetaM Lean.Expr :=
          withLocalDeclD `value (mkApp2 (mkConst ``NTy.plain) carriers τ) fun value =>
            mkLambdaFVars #[value] value
        let keyRead ← identity key
        let valueRead ← identity value
        let some injective' ← sequenceReadInjective? carriers key keyRead | failure
        mkAppOptM ``valid_sequence_plainReframe
          #[unit, functions, nonempty, frame, params, own, key, value, bounded, entries,
            none, none, none, none, readKey, readValue, keyRead, valueRead, injective, injective']
      else do
        let_expr NTy.readPlain carriers nonempty key keyHeld := readKey | failure
        let_expr NTy.readPlain carriers' _ value valueHeld := readValue | failure
        unless ← isDefEq carriers carriers' do failure
        let τ := mkApp2 (mkConst ``NTy.table) key value
        let some held ← entryHoldsProof? (mkApp3 (mkConst ``NTy.deepHolds) carriers τ table)
          | failure
        mkAppOptM ``valid_sequence_readPlain
          #[carriers, nonempty, key, value, keyHeld, valueHeld, table, held]
    let some (_, rhs) := (← whnfR (← inferType proof)).app2? ``Iff | failure
    pure (rhs, ← mkPropExt proof)
  let some (rhs, proof) ← application | return .continue
  return .visit { expr := rhs, proof? := some proof }

/-- The frame a leaf states its values at: its type parameters' frame, or the
runtime frame where it has none. -/
private def leafFrame? (unit functions : Lean.Expr) : MetaM Lean.Expr := do
  for declaration in ← getLCtx do
    if declaration.isImplementationDetail then continue
    if (← instantiateMVars declaration.type).isAppOf ``VerexIR.Proofs.Denote.Skolems then
      return declaration.toExpr
  runtimeFrame unit functions

/-- A term with each native operation at closed types spelled at the frame
`reader`: carriers, instances and reads at a closed type do not look at their
frame, and convert where the operation reduces on its types. -/
private def respellClosedAt (reader e : Lean.Expr) : MetaM Lean.Expr :=
  Meta.transform e (post := fun e => do
    let some head := e.getAppFn.constName? | return .continue
    unless (`VerexIR.Proofs.Denote).isPrefixOf head do return .continue
    let args := e.getAppArgs
    -- The frame: of the carriers it takes first, or the one after its unit and
    -- function values.
    let family := (args[0]?.getD e).consumeMData
    let frameArgument := (args[2]?.getD e).consumeMData
    let some (position, frame, atReader) :=
        if family.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.toCarriers 3 then
          some (0, family.getArg! 2, mkApp3 family.appFn!.appFn!.appFn! (family.getArg! 0)
            (family.getArg! 1) reader)
        else if frameArgument.isAppOf ``VerexIR.Proofs.Denote.Skolems.runtime ||
            frameArgument.isAppOf ``VerexIR.Proofs.Denote.Skolems.instantiate ||
            frameArgument.isAppOf ``VerexIR.Proofs.Denote.Skolems.instance then
          some (2, frameArgument, reader)
        else none
      | return .continue
    if frame == reader then return .continue
    let env ← getEnv
    let spelledType (arg : Lean.Expr) : Bool :=
      isNominalType env arg || match arg.consumeMData.getAppFn.constName? with
      | some name => env.isConstructor name &&
          [``VerexIR.Proofs.Denote.NTy, ``VerexIR.Proofs.Denote.NRow,
            ``VerexIR.Proofs.Denote.NRows].contains name.getPrefix
      | none => false
    let mut typed := 0
    for index in [position + 1:args.size] do
      if spelledType args[index]! then
        if (args[index]!.find? (·.isAppOfArity ``VerexIR.Proofs.Denote.NTy.param 1)).isSome then
          return .continue
        typed := index + 1
    if typed == 0 then return .continue
    let operation := mkAppN e.getAppFn (args.extract 0 typed)
    let atFrame := mkAppN e.getAppFn ((args.extract 0 typed).set! position atReader)
    unless ← isDefEq operation atFrame do return .continue
    return .done (mkAppN e.getAppFn (args.set! position atReader)))

/-- A statement of a stored value at another frame in the leaf's spelling: the
frame its type arguments induce at the outer frame with the types substituted
(`FamilyTransport.respellInducedFamily`), and an operation at closed types,
which reads no frame, at the leaf's frame (`respellClosedAt`). The spellings
convert. -/
private def inLeafSpelling (unit functions statement : Lean.Expr) : MetaM Lean.Expr :=
    withTransparency .default do
  let statement ← FamilyTransport.respellInducedFamily statement
  respellClosedAt (← leafFrame? unit functions) statement

/-- Each goal stating a stored value's invariant (`StoredCase`), as the case
of the value at its resource's type arguments (`storedCase?`). -/
-- Retired with doc/plan/data-and-global-invariants.md, "The stored-invariant layer
-- still exists for the declarations the carriers do not state": the carrier replaces it.
elab "verex_stored_cases" : tactic => do
  let mut goals := #[]
  for goal in ← getGoals do
    if ← goal.isAssigned then continue
    let rewritten ← goal.withContext do
      let some (statement, intro, _) ← storedCase? (← instantiateMVars (← goal.getType))
        | return goal
      let held ← mkFreshExprSyntheticOpaqueMVar statement
      goal.assign (← intro held)
      return held.mvarId!
    goals := goals.push rewritten
  setGoals goals.toList

/-- A goal stating a declaration's data invariant of a row encoded at the
leaf's frame, restated with the row at the frame the declaration's type
arguments induce, where the clauses are stated: the two frames give a row
the same encoding (`Skolems.plainEncode_plainReframe`), and the invariant is
a predicate of the encoding.

Taking the two frames for one instead rests on reducing the leaf frame's
`Skolems.resolve`, a projection of a local frame, which the elaborator takes
and the kernel refuses, so the step's proof is accepted and then rejected
(`rowInhabitable?`).  The declaration's parameter row and its type carry no
frame, so the two premises of the reframing are decided
(`decidedFrameFree?`). -/
private def reframeOwnRow (goal : MVarId) (unit functions frame σ params row : Lean.Expr) :
    MetaM MVarId := do
  let some own ← decidedFrameFree? (← mkEq
      (mkApp2 (mkConst ``VerexIR.Proofs.Denote.NRow.isParamsFrom) (mkNatLit 0) params)
      (mkConst ``Bool.true))
    | throwError "the type arguments {params} of the declaration are not its parameters"
  let some nonempty ← synthInstance? (mkApp (mkConst ``VerexIR.Proofs.Denote.CarriersNonempty)
      (mkApp2 (mkConst ``VerexIR.Proofs.Denote.Carriers.runtime) unit functions))
    | throwError "the leaf's family has no inhabitants of its certified carriers"
  let reframing := mkAppN
    (mkConst ``VerexIR.Proofs.Denote.Skolems.plainEncode_plainReframe)
    #[unit, functions, nonempty, frame, params, own, σ]
  let some bounded ← decidedFrameFree? (← whnf (← inferType reframing)).bindingDomain!
    | throwError "the declaration's type {σ} is not over its own parameters"
  let equation ← nominalPlainReframeEquation (mkApp2 reframing bounded row) σ
  let rewritten ← goal.rewrite (← goal.getType) equation (symm := true)
  goal.replaceTargetEq rewritten.eNew rewritten.eqProof

/-- A row of type arguments with each frame's resolution of a type without
parameters replaced by that type (`Skolems.resolve_paramFree`), and the
equation; none where no entry is such a resolution. -/
private partial def closedResolutions? (row : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let row := row.consumeMData
  unless row.isAppOfArity ``VerexIR.Proofs.Denote.NRow.cons 2 do return none
  let (τ, rest) := (row.getArg! 0, row.getArg! 1)
  let tail? ← closedResolutions? rest
  let resolved := τ.consumeMData
  let head? ← if resolved.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.resolve 4 then
      let type := resolved.appArg!.consumeMData
      match ← decidedFrameFree? (← mkEq (mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.paramFree) type)
          (mkConst ``Bool.true)) with
      | some free =>
          let arguments := resolved.getAppArgs
          let nonempty := mkApp3 (mkConst ``VerexIR.Proofs.Denote.Skolems.runtimeNonempty)
            arguments[0]! arguments[1]! arguments[2]!
          pure (some (type, mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.resolve_paramFree)
            #[arguments[0]!, arguments[1]!, nonempty, arguments[2]!, type, free]))
      | none => pure none
    else pure none
  if head?.isNone && tail?.isNone then return none
  let (head, headEq) ← match head? with
    | some found => pure found
    | none => pure (τ, ← mkEqRefl τ)
  let (tail, tailEq) ← match tail? with
    | some found => pure found
    | none => pure (rest, ← mkEqRefl rest)
  let cons := mkConst ``VerexIR.Proofs.Denote.NRow.cons
  return some (mkApp2 cons head tail, ← mkCongr (← mkCongrArg cons headEq) tailEq)

/-- A goal stating a declaration's data invariant of an encoding, with the
row at the frame the type arguments induce: unchanged where it is already
there, moved where the row is stated at the leaf's frame over the
declaration's own parameters (`reframeOwnRow`), and spelled without the
frame's resolutions of closed types where the instance is closed: a closed
instance's row reads no parameter, so it is the same term at both frames.
An instance over other parameters is refused by `reframeOwnRow`. -/
private def ownRowAtArguments (goal : MVarId) : MetaM MVarId := do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOf ``VerexIR.Proofs.Denote.OwnAtArguments do
    throwError "the goal does not state a data invariant of an encoding"
  let arguments := target.getAppArgs
  unless arguments.size == 7 do
    throwError "a data invariant of an encoding takes seven arguments, not {arguments.size}"
  let value := arguments[6]!.consumeMData
  unless value.isAppOfArity ``VerexIR.Proofs.Denote.NTy.plainEncode 3 do
    throwError "the value a data invariant is stated of is not an encoding: {value}"
  let carriers := (value.getArg! 0).consumeMData
  unless carriers.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.toCarriers 3 do return goal
  -- The leaf's own frame is a binder of its theorem; a frame the type
  -- arguments induce is a term built over the runtime frame, and a row
  -- already stated at one needs no move.
  let frame := (carriers.getArg! 2).consumeMData
  unless frame.isFVar do return goal
  -- A declaration of either shape states its clauses at the frame of its own
  -- type arguments, so a row stated at the leaf's frame moves there.  A
  -- struct's plain row is a product recursion (`HListPlainOf`) that the kernel
  -- reduces at a constructor row, which makes the two frames' rows the same
  -- term where every field is a scalar; where a field's plain representation
  -- reads the family, as a table's or a vector's does, they are not, and the
  -- predicate of the encoding does not unify with the clauses.
  let σ := (value.getArg! 1).consumeMData
  let view ← nativeTypeView σ
  unless view.isAppOfArity ``VerexIR.Proofs.Denote.NTy.enum 5 ||
      view.isAppOfArity ``VerexIR.Proofs.Denote.NTy.struct 3 do return goal
  let params := (view.getArg! 1).consumeMData
  let own ← decidedFrameFree? (← mkEq
      (mkApp2 (mkConst ``VerexIR.Proofs.Denote.NRow.isParamsFrom) (mkNatLit 0) params)
      (mkConst ``Bool.true))
  if own.isSome then
    reframeOwnRow goal (carriers.getArg! 0) (carriers.getArg! 1) frame σ params (value.getArg! 2)
  else if (← decidedFrameFree? (← mkEq (mkApp (mkConst ``VerexIR.Proofs.Denote.NTy.paramFree) σ)
      (mkConst ``Bool.true))).isSome then
    -- A closed instance's row reads no parameter, so it is the same term at
    -- both frames once the invariant's arguments are spelled without the
    -- leaf frame's resolutions of closed types.
    let some (_, same) ← closedResolutions? arguments[5]! | return goal
    let rewritten ← goal.rewrite target same
    goal.replaceTargetEq rewritten.eNew rewritten.eqProof
  else
    -- An instance over other parameters has no move yet: the reframing
    -- refuses it with its reason.
    reframeOwnRow goal (carriers.getArg! 0) (carriers.getArg! 1) frame σ params (value.getArg! 2)

/-- Open a nominal obligation already stated at its own interpretation.
The raw predicate quantifies decoded rows. Its encoding determines that row
by the codec's inverse law. Transporting the declared clause along that law
preserves its dependent arguments without assuming parameter invariants. -/
def decodedOwnGoal? (goal : MVarId) : MetaM (Option MVarId) := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``Carriers.own 4 do return none
  let encoded := target.appArg!.consumeMData
  unless encoded.isAppOfArity ``NTy.plainEncode 3 do return none
  let codec := mkApp2 (mkConst ``NTy.plainCodec) (encoded.getArg! 0) (encoded.getArg! 1)
  let row := encoded.getArg! 2
  let context ← getLCtx
  let instances ← getLocalInstances
  let tag ← goal.getTag
  observing? do
    let exposed ← whnfD target
    unless exposed.isForall do throwError "the nominal predicate has no decoded row"
    withLocalDecl exposed.bindingName! exposed.binderInfo exposed.bindingDomain! fun decodedRow => do
      let decoding ← whnfD (exposed.bindingBody!.instantiate1 decodedRow)
      unless decoding.isForall do throwError "the nominal predicate has no decoding premise"
      withLocalDecl decoding.bindingName! decoding.binderInfo decoding.bindingDomain! fun decoded => do
        let clause := decoding.bindingBody!.instantiate1 decoded
        if clause.containsFVar decoded.fvarId! then
          throwError "the nominal clause depends on the decoding proof"
        let predicate ← mkLambdaFVars #[decodedRow] clause
        let statement ← deltaExpand (predicate.beta #[row])
          (fun name => (name.toString.splitOn "dataOwn.case").length != 1)
        let next ← mkFreshExprMVarAt context instances statement .syntheticOpaque tag
        let inverse ← mkAppM ``Codec.decode_encode #[codec, row]
        let sameOption ← withTransparency .default <|
          mkEqTrans (← mkEqSymm inverse) decoded
        let sameRow ← mkAppM ``Option.some.inj #[sameOption]
        let transported ← mkEqMP (← mkCongrArg predicate sameRow) next
        let proof ← mkLambdaFVars #[decodedRow, decoded] transported
        goal.assign proof
        return next.mvarId!

/-- A declaration's data invariant of an encoding, as the clauses of the
value encoded (`OwnAtArguments`): the row moved to the frame the type
arguments induce where it is stated at the leaf's (`ownRowAtArguments`),
then `OwnAtArguments.plainEncode`, whose premises the encoding determines
(`Codec.decode_encode`).  What is left is the declaration's clauses of the
row. Open that selected predicate for this obligation; ordinary normalization
keeps predicates in retained certificates nominal
(`doc/designs/data-invariants.md`). -/
elab "verex_own_encoded" : tactic => focusGoal do
  let goal ← getMainGoal
  let next ← goal.withContext do
    let goal ← ownRowAtArguments goal
    let target ← instantiateMVars (← goal.getType)
    let arguments := target.getAppArgs
    let value := arguments[6]!.consumeMData
    let frame := (value.getArg! 0).consumeMData
    let row := value.getArg! 2
    -- The frame the value is encoded at carries the inhabitability of the
    -- arguments (`Skolems.atArguments`).  A frame spelled otherwise has it
    -- proved: a ground row is decided, and a row of a frame's resolutions,
    -- which a generic declaration's arguments are, from the frame's law
    -- that it resolves every type parameter to an inhabitable type
    -- (`Skolems.type_inhabitable`, `NRow.inhabitable_map_resolve`).
    let frame ← if frame.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.toCarriers 3 then
        pure (frame.getArg! 2).consumeMData
      else pure frame
    let inhabitable ←
      if frame.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.atArguments 5 then
        pure (frame.getArg! 4)
      else
        match ← rowInhabitable? arguments[5]! with
        | some proof => pure proof
        | none => throwError
            "the type arguments {arguments[6]!} are not known to be inhabitable"
    let rule ← mkAppM ``Iff.mpr
      #[mkAppN (mkConst ``VerexIR.Proofs.Denote.OwnAtArguments.plainEncode)
        (arguments.extract 0 6 ++ #[inhabitable, row])]
    match ← goal.apply rule with
    | [clauses] =>
        let statement ← clauses.withContext do
          deltaExpand (← instantiateMVars (← clauses.getType))
            (fun name => (name.toString.splitOn "dataOwn.case").length != 1)
        clauses.withContext <| clauses.change statement
    | goals => throwError
        "the data invariant of an encoding left {goals.length} obligations"
  replaceMainGoal [next]

/-- Establish the data invariants of stored resources (`DataInvariants`)
of a memory the leaf's writes made from one the context has them of:
through each write, which owes the value written its own invariant. A goal
preserving them takes them of the earlier memory first. -/
-- Retired with doc/plan/data-and-global-invariants.md, "The stored-invariant layer
-- still exists for the declarations the carriers do not state": the carrier replaces it.
elab "verex_data_invariants" : tactic => focusGoal do
  let rec establish (goal : MVarId) : Nat → TacticM (List MVarId)
    | 0 => throwError "too many writes to establish the stored invariants through"
    | fuel + 1 => goal.withContext do
      let target ← instantiateMVars (← goal.getType)
      if target.isArrow && target.bindingBody!.isAppOfArity
          ``VerexIR.Proofs.Denote.DataInvariants 4 then
        let (_, goal) ← goal.intro1P
        return ← establish goal fuel
      unless target.isAppOfArity ``VerexIR.Proofs.Denote.DataInvariants 4 do
        throwError "the leaf owes no stored invariants"
      if let some proof ← storedInvariantsProof? (target.getArg! 3) then
        goal.assign proof
        return []
      let written := target.getArg! 3
      unless written.isAppOfArity ``VerexIR.Proofs.Denote.Memory.set 6 do
        throwError "no hypothesis states the stored invariants of the memory written"
      let value := written.getArg! 5
      if value.isAppOfArity ``Option.none 1 then
        let [holds] ← goal.apply (mkConst ``VerexIR.Proofs.Denote.DataInvariants.set_none)
          | throwError "the removal lemma did not leave its obligation"
        return ← establish holds fuel
      let lemma := if value.isAppOfArity ``Option.some 2 then
          ``VerexIR.Proofs.Denote.DataInvariants.set_some
        else ``VerexIR.Proofs.Denote.DataInvariants.set
      let [holds, stored] ← goal.apply (mkConst lemma)
        | throwError "the write lemma did not leave its two obligations"
      return (← establish holds fuel) ++ [stored]
  replaceMainGoal (← establish (← getMainGoal) 256)
  -- Select the invariant of each written resource before running the leaf
  -- pipeline over the surrounding context. Constructor comparisons in the
  -- unit's dispatch table are computations, not new proof obligations; the
  -- selected case is stated at the resource's own type arguments.
  evalTactic (← `(tactic| all_goals (try simp only
    [vir_denote, vir_denote_norm, vir_denote_eval, Prod.fst, Prod.snd])))
  evalTactic (← `(tactic| verex_stored_cases))
  evalTactic (← `(tactic| all_goals (try simp only
    [vir_denote, vir_denote_norm, vir_denote_eval, Prod.fst, Prod.snd])))
  -- A clause in the leaf's spelling, as the context states its values.
  setGoals (← (← getGoals).mapM fun goal => goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let some unit := target.find? (·.isAppOfArity ``Skolems.toCarriers 3) | return goal
    let respelled ← inLeafSpelling (unit.getArg! 0) (unit.getArg! 1) target
    if respelled == target then return goal else goal.replaceTargetDefEq respelled)
  -- What a write owes of the value it stores is the invariant the
  -- construction's or the mutation's cut already established of that value,
  -- in the same spelling: close it from the context here instead of leaving
  -- a leaf to prove it again
  -- (`doc/plan/data-and-global-invariants.md`).
  setGoals (← (← getGoals).filterM fun goal =>
    return !(← closesBy goal (← `(tactic| verex_assumption))))

/-- Whether a goal states the stored invariants of a memory a write made. -/
-- Retired with doc/plan/data-and-global-invariants.md, "The stored-invariant layer
-- still exists for the declarations the carriers do not state": the carrier replaces it.
def writesStoredInvariants (goal : MVarId) : MetaM Bool := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  return target.isAppOfArity ``VerexIR.Proofs.Denote.DataInvariants 4 &&
    (target.getArg! 3).isAppOfArity ``VerexIR.Proofs.Denote.Memory.set 6

/-- Whether a leaf states the stored invariants of a memory it reads. -/
-- Retired with doc/plan/data-and-global-invariants.md, "The stored-invariant layer
-- still exists for the declarations the carriers do not state": the carrier replaces it.
def readsStoredInvariants (goal : MVarId) : MetaM Bool := goal.withContext do
  (← getLCtx).foldlM (init := false) fun invariants declaration => do
    if declaration.isImplementationDetail then return invariants
    let type ← instantiateMVars declaration.type
    return invariants || (type.find? (·.isConstOf ``VerexIR.Proofs.Denote.DataInvariants)).isSome

/-- The data invariant of each stored value the leaf reads, from the stored
invariants of the memory it reads (`DataInvariants.read`). -/
-- Retired with doc/plan/data-and-global-invariants.md, "The stored-invariant layer
-- still exists for the declarations the carriers do not state": the carrier replaces it.
elab "verex_stored_reads" : tactic => focusGoal do
  let goal ← getMainGoal
  unless ← readsStoredInvariants goal do
    throwError "the leaf reads no stored value with stored invariants"
  -- The invariant of a typed map's entry is stated under the entry's presence
  -- (`NativeMaps.presentKey`), which decides its conditionals. A fact states
  -- its invariant's clauses without their markers, which only goals carry.
  let (ctx, simprocs) ← do
    let stx ← `(tactic| simp only [vir_denote, vir_denote_norm, vir_denote_eval, Prod.fst, Prod.snd,
      eq_self_iff_true, ite_true, ite_false, ↓reduceIte, VerexIR.Proofs.Obligation_eq])
    let { ctx, simprocs, .. } ← mkSimpContext stx (eraseLocal := false)
    pure (ctx, simprocs)
  let facts ← goal.withContext do
    let mut facts : Array Lean.Expr := #[]
    let normalized (fact : Lean.Expr) : MetaM Lean.Expr := do
      let (result, _) ← Simp.main (← inferType fact) ctx
        (methods := Simp.mkDefaultMethodsCore simprocs)
      result.mkEqMP fact
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      let type ← instantiateMVars declaration.type
      let_expr Eq _ read held := type | continue
      unless held.isAppOfArity ``Option.some 2 && read.getAppNumArgs == 2 do continue
      let memory := read.appFn!.appFn!
      unless (← whnfR (← inferType memory)).isAppOfArity ``VerexIR.Proofs.Denote.Memory 2 do continue
      let some holds ← storedInvariantsProof? memory | continue
      let fact ← normalized
        (← mkAppM ``VerexIR.Proofs.Denote.DataInvariants.read #[holds, declaration.toExpr])
      -- The selected case, at the resource's own type arguments.
      let case ← whnfR (← inferType fact)
      match ← storedCase? case (direct := true) with
      | some (_, _, elim) =>
          let selected ← normalized (← elim fact)
          let stated ← inferType selected
          let respelled ← inLeafSpelling (case.getArg! 0) (case.getArg! 1) stated
          facts := facts.push <| ← if respelled == stated then pure selected
            else mkExpectedTypeHint selected respelled
      | none => facts := facts.push fact
    pure facts
  if facts.isEmpty then throwError "the leaf reads no stored value with stored invariants"
  replaceMainGoal [← assertFacts goal `stored facts]

/-- Under `verex.solverDebug`, print the goal and the propositional
hypotheses of every open goal, each on one truncated line. -/
elab "verex_trace_state " label:str : tactic => do
  unless verex.solverDebug.get (← getOptions) do return
  for goal in ← getGoals do
    goal.withContext do
      let line (e : Lean.Expr) (limit : Nat := 220) : MetaM String := do
        return ((toString (← ppExpr e)).replace "\n" " " |>.take limit).toString
      IO.println s!"    {label.getString} ⊢ {← line (← instantiateMVars (← goal.getType)) 3000}"
      for fvarId in ← goal.getNondepPropHyps do
        let decl ← fvarId.getDecl
        IO.println s!"      {decl.userName} : {← line (← instantiateMVars decl.type)}"

-- A leaf holding values in a generic callee's view reads its ranges once the
-- substitutions settled; each case is decided by the pipeline, which splits
-- on a further position.
macro_rules
  | `(tactic| verex_decide_ranges) => `(tactic| verex_focus ( (
      first
      | verex_timed "range split" verex_split_range
      | (verex_holds_transport
         verex_settle verex_normalize_context
         verex_timed "range split" verex_split_range)
      all_goals try rfl
      all_goals verex_reduce_projections
      all_goals verex_normalize_context
      -- The quantifiers the case's value bounds by literals, instance by
      -- instance.
      all_goals (try (verex_expand_ranges; verex_normalize_context))
      -- The specification functions the case's values fix, unfolded to
      -- their base cases.
      all_goals (try (verex_timed "range unfold" verex_specs_to_base
                      verex_normalize_context))
      all_goals verex_settle verex_normalize_context
      all_goals (first
        | done
        | verex_timed "range omega" verex_omega_unsplit
        -- A range invariant extended by the case's iteration.
        | verex_timed "range instance" verex_range_instance
        | (verex_prepared_applies
           verex_timed "written" verex_decide_written)
        | verex_timed "range pipeline" verex_pipeline))))

/-- Decide one leaf. -/
syntax "verex_leaf_instance" : tactic
syntax "verex_leaf" : tactic
/-- One attempt at a leaf: decided, or the cases of its prepared form's
variant split left open (`verex_split_prepared`). -/
syntax "verex_leaf_attempt" : tactic

/-- Whether the leaf tests a local's variant (`rankedTestedLocals`), which
its prepared form may split on. -/
elab "verex_tests_variants" : tactic => focusGoal do
  if (← rankedTestedLocals (← getMainGoal)).isEmpty then
    throwError "the leaf tests no local's variant"

/-- The tag of a case of a prepared leaf's variant split left open as an
attempt of its own (`verex_split_prepared`). -/
def preparedCaseTag : Name := `verex_prepared_case

/-- The tag of a leaf left open for its prepared form, an attempt of its own
(`verex_open_prepared`). -/
def preparationTag : Name := `verex_preparation

/-- A leaf whose deciders need the normal form an authored proof receives
(the normalization is one pass over the context, where the saturation of
the pipeline rewrites the hypotheses with each other for rounds), left open
for its preparation, an attempt of its own (`verex_prepared_leaf`). -/
syntax "verex_open_prepared" : tactic
/-- A prepared leaf, decided (`verex_prepared_case`). -/
syntax "verex_prepared_leaf" : tactic
/-- Decide a prepared goal. -/
syntax "verex_decide_prepared_goal" : tactic
/-- A case of a prepared leaf's variant split, decided as the prepared leaf
(`verex_prepared_decide`). -/
syntax "verex_prepared_case" : tactic

/-- Tag the goal as left open for its preparation (`preparationTag`). -/
elab "verex_tag_preparation" : tactic => focusGoal do
  (← getMainGoal).setTag preparationTag

macro_rules
  | `(tactic| verex_open_prepared) => `(tactic| verex_focus ( (
      (first | verex_prepared_applies | verex_has_atoms | verex_tests_variants)
      verex_timed "intro" verex_intro_any
      verex_tag_preparation)))

/-- Expose the fields of a certified record when field equations constrain
it and an opaque observation reads the whole record. Substituting the fields
then lets ordinary congruence compare that observation. -/
elab "verex_observed_row_fields" : tactic => focusGoal do
  let goal ← getMainGoal
  let candidates ← goal.withContext do
    let env ← getEnv
    let mut terms := #[← goal.getType]
    let mut candidates : Array FVarId := #[]
    let rec owner? (term : Lean.Expr) : Nat → Option FVarId
      | 0 => none
      | fuel + 1 => do
        if term.isAppOfArity ``Prod.fst 3 || term.isAppOfArity ``Prod.snd 3 then
          owner? term.appArg! fuel
        else
          guard <| term.isAppOfArity ``Subtype.val 3
          let .fvar id := term.appArg! | none
          some id
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let type ← instantiateMVars decl.type
      terms := terms.push type
      let some (_, lhs, rhs) := type.eq? | continue
      for side in #[lhs, rhs] do
        let some owner := owner? side 64 | continue
        unless candidates.contains owner do candidates := candidates.push owner
    candidates.filterM fun value => do
      let type ← instantiateMVars (← value.getType)
      unless type.isAppOfArity ``NTy.carrier 2 do return false
      unless (← nativeTypeView (type.getArg! 1)).isAppOfArity ``NTy.struct 3 do return false
      return terms.any fun term => (term.find? fun sub =>
        if !sub.isApp || !sub.containsFVar value then false
        else if sub.isAppOf ``VerexIR.Proofs.serializedBytes then true
        else match sub.getAppFn with
          | .const name _ => match env.find? name with
            | some (.opaqueInfo _) | some (.axiomInfo _) => true
            | _ => false
          | _ => false).isSome
  if candidates.isEmpty then throwError "no constrained record read by an opaque observation"
  let mut goals := #[goal]
  for candidate in candidates do
    goals ← goals.flatMapM fun goal => goal.withContext do
      unless (← getLCtx).contains candidate do return #[goal]
      destructureOuterVariant goal candidate
  setGoals goals.toList

/-- The type of a certified value, recognized without unfolding a named
declaration or inspecting unrelated implicit arguments of an observation. -/
private def certifiedCongruenceType (type : Lean.Expr) : Bool :=
  type.isAppOfArity ``NTy.carrier 2 || type.isAppOfArity ``Carriers.carrier 2 ||
    (type.isAppOfArity ``Subtype 2 &&
      ((type.getArg! 1).find? fun part => part.isConstOf ``NTy.deepHolds ||
        part.isConstOf ``HListPlain.deepHolds || part.isConstOf ``variantPlain.deepHolds).isSome)

/-- Certification, a named plain default constructor, or a direct observation
of either. An ordinary computation
does not become a certification equation merely because its operands' types
mention a carrier. Opaque observations retain the certified-record case
handled by `verex_observed_row_fields`. -/
private partial def certifiedCongruenceObservation (term : Lean.Expr)
    (payload : Bool := false) : MetaM Bool := do
  let term := term.consumeMData
  if term.isAppOf ``NTy.ofDeepHolds || term.isAppOf ``Var.selectCertified ||
      term.isAppOf ``ParamPlain.certify || term.isAppOf ``ParamPlain.of ||
      term.isAppOf ``NTy.plainInhabitantOf || term.isAppOf ``NTy.plainInhabitant ||
      term.isAppOf ``HListPlain.inhabitant || term.isAppOf ``variantPlain.inhabitant then return true
  if let .fvar id := term then return certifiedCongruenceType (← id.getType)
  if let .proj _ _ owner := term then return ← certifiedCongruenceObservation owner
  if term.isAppOf ``Subtype.val || term.isAppOf ``Prod.fst || term.isAppOf ``Prod.snd ||
      term.isAppOf ``VerexIR.SpecInt.val || term.isAppOf ``VerexIR.SpecVector.values ||
      term.isAppOf ``NTy.forget then
    return ← certifiedCongruenceObservation term.appArg!
  if term.isAppOfArity ``Subtype.mk 4 then
    return certifiedCongruenceType
      (mkApp2 (mkConst ``Subtype term.getAppFn.constLevels!) (term.getArg! 0) (term.getArg! 1))
  let some name := term.getAppFn.constName? | return false
  let env ← getEnv
  let opaqueObservation := name == ``VerexIR.Proofs.serializedBytes ||
    match env.find? name with
    | some (.opaqueInfo _) | some (.axiomInfo _) => true
    | _ => false
  unless opaqueObservation || (payload && env.isConstructor name) do return false
  for argument in term.getAppArgs do
    unless argument.hasLooseBVars do
      if (← isType argument) || (← isProof argument) then continue
    if ← certifiedCongruenceObservation argument true then return true
  return false

private partial def certifiedCongruenceGoal (target : Lean.Expr) : MetaM Bool := do
  let target := StatedFacts.unmarked target
  if let some (type, left, right) := target.eq? then
    return certifiedCongruenceType type || (← certifiedCongruenceObservation left) ||
      (← certifiedCongruenceObservation right)
  if target.isAppOfArity ``Iff 2 || target.isAppOfArity ``And 2 || target.isAppOfArity ``Or 2 then
    return (← certifiedCongruenceGoal (target.getArg! 0)) ||
      (← certifiedCongruenceGoal (target.getArg! 1))
  if let .forallE _ domain body _ := target then
    return (← certifiedCongruenceGoal domain) || (← certifiedCongruenceGoal body)
  return false

/-- Reserve default-transparency structural congruence for native value
construction equations and their observations. -/
elab "verex_certified_congr_applies" : tactic => focusGoal do
  let goal ← getMainGoal
  unless ← goal.withContext (certifiedCongruenceGoal (← instantiateMVars (← goal.getType))) do
    throwError "the goal states no native-value construction equation or observation"

/-- An obligation whose two sides differ only by the equations the other
side states: the equivalence is split, and each direction substitutes the
equations it is given and closes by reflexivity, the context's facts
discharging what is left.

The only closer that proves a goal stating two certified values.  A
certificate is irrelevant, so once the values a certified row is built from
are substituted the two sides are the same term; no rewriting reaches that,
because a certificate's type mentions the value it certifies and rewriting
under it is not type-correct.  This is the closer's half of
`certifiedByConstruction`, which states the equation of two built certified
values but cannot carry the equations of their parts into the certificates
(`doc/designs/data-invariants.md`). -/
syntax "verex_certified_congr" : tactic

-- All or nothing, like every closer: the rewriting a direction ends with
-- can succeed without closing, and an alternative that leaves a goal open
-- would preempt the ones after it and turn the leaf into a report.
macro_rules
  | `(tactic| verex_certified_congr) => `(tactic| verex_focus (
      verex_certified_congr_applies
      try simp only [VerexIR.Proofs.Obligation_eq] at *
      (first
        | done
        | (verex_observed_row_fields; all_goals
            (subst_vars; first | assumption | rfl | simp_all only [vir_denote_norm]))
        | (constructor <;>
            (intro verexGiven
             try obtain ⟨_, _⟩ := verexGiven
             subst_vars
             first | rfl | simp_all))
        | (intro verexGiven
           try obtain ⟨_, _⟩ := verexGiven
           subst_vars
           first | rfl | simp_all)
        | (subst_vars; rfl)); done))

/-- Fail unless the leaf's goal is `False`: the path is refuted, and what
closes it is a contradiction among the context's facts rather than a proof
of the obligation. -/
elab "verex_goal_refuted" : tactic => focusGoal do
  let goal ← getMainGoal
  unless (← goal.withContext do whnfR (← instantiateMVars (← goal.getType))).isConstOf ``False do
    throwError "the leaf's goal is no contradiction"

/-- A refuted path whose contradiction is that it leaves a variant value no
variant: a leaf that reads the name of such a value and refutes every name
of its declaration is refuted by the value's own type, which admits only
those names (`variantPlainName_mem`, `variantName_mem`).  The membership is
a fact of the type, as the bounds of a certified integer are
(`verex_bounds`), and the path's refutations decide it; gated on the goal
and all or nothing, as the other refutations are.

A clause that concludes the contradiction under premises states facts of
the path as much as the context does — a name the clause's own implication
gives is a reading like any other — so the premises are introduced first
and the refutation then reads `False` as it does on a refuted path's own
goal. -/
elab "verex_refute_variant" : tactic => focusGoal do
  let mut goal ← getMainGoal
  for _ in [0:8] do
    let reduced ← goal.withContext do
      whnfR (StatedFacts.unmarked (← instantiateMVars (← goal.getType)))
    if reduced.isConstOf ``False then break
    unless reduced.isArrow do break
    goal ← goal.withContext do
      pure (← (← goal.replaceTargetDefEq reduced).intro1P).2
  goal.withContext do
  let target := StatedFacts.unmarked (← instantiateMVars (← goal.getType))
  unless (← whnfR target).isConstOf ``False do
    throwError "the leaf's goal is no contradiction"
  -- Every name read of a variant value the leaf states, each once.
  let readings : Array (Name × Name) :=
    #[(``VerexIR.Proofs.Denote.variantPlainName, ``VerexIR.Proofs.Denote.variantPlainName_mem),
      (``VerexIR.Proofs.Denote.variantName, ``VerexIR.Proofs.Denote.variantName_mem)]
  let mut facts : Array Hypothesis := #[]
  let mut read : Array Lean.Expr := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let stated := StatedFacts.unmarked (← instantiateMVars decl.type)
    for name in sitesWhere (fun e => readings.any (fun (reading, _) => e.isAppOfArity reading 4) &&
        !e.hasLooseBVars) #[stated] do
      if read.contains name then continue
      read := read.push name
      let some (_, rule) := readings.find? fun (reading, _) => name.isAppOf reading | continue
      let proof := mkAppN (mkConst rule name.getAppFn.constLevels!) name.getAppArgs
      facts := facts.push { userName := `verexVariants, type := ← inferType proof, value := proof }
  if facts.isEmpty then throwError "the leaf reads no variant's name"
  -- The contradiction is the path's, not the clause's: the marker the
  -- obligation carries is dropped so that the closer reads `False`.
  let goal ← goal.replaceTargetDefEq target
  let (_, next) ← goal.assertHypotheses facts
  setGoals [next]
  evalTactic (← `(tactic| (simp_all only [List.mem_cons, List.not_mem_nil, or_false]; done)))

/-- A refuted path closed by linear arithmetic over the normal form an
authored proof receives (`verex_prepare_leaf`).

The facts a contradiction is between are stated in the spellings the walk
left — a projection of a tuple the walk assembled, an obligation under its
source marker, a certified value's part through an intermediate the
preparation substitutes — and arithmetic reads each of those as an atom of
its own, so a refutation the prepared context decides by two facts about
one term is invisible to the leaf's own `omega`.  Gated on the goal, and a
refutation either closes the leaf or declines, so the preparation is paid
only where nothing cheaper applies and never leaves a goal open. -/
syntax "verex_refute_prepared" : tactic

macro_rules
  | `(tactic| verex_refute_prepared) => `(tactic| verex_focus (
      verex_goal_refuted
      verex_prepare_leaf
      all_goals verex_omega_unsplit
      done))

/-- The deciders of a leaf after the prepared ones: general rewriting, a
disjunctive hypothesis case by case, then the certified-value congruence. -/
syntax "verex_leaf_fallbacks" : tactic

macro_rules
  | `(tactic| verex_leaf_fallbacks) => `(tactic| verex_focus (
      first
      -- A path that leaves a variant value no variant, by the membership
      -- its type states: gated on the goal, all or nothing, and cheaper
      -- than the preparation the arithmetic refutation pays.
      | verex_timed "refuted variant" verex_refute_variant
      -- A refuted path, by arithmetic over its prepared context: gated on
      -- the goal and all or nothing, so it precedes the speculative
      -- rewriting that is otherwise the only step reaching the same facts.
      | verex_timed "refuted" (verex_function_budgeted verex_refute_prepared)
      -- Substitution and reflexivity before general rewriting: this is a
      -- cheap structural closer, and an obligation over certified values
      -- has no other, while a failing `verex_pipeline` can spend the whole
      -- leaf budget and leave no alternative a turn.
      | verex_timed "certified congruence"
         (verex_function_budgeted verex_certified_congr)
      -- General rewriting is speculative after the cheaper deciders
      -- failed. An invalid clause must leave a diagnostic rather than
      -- spend the entire target budget in this fallback.
      | verex_timed "pipeline" (verex_function_budgeted verex_pipeline)
      -- A disjunctive hypothesis, case by case.
      | verex_function_budgeted
         (verex_timed "cases" verex_split_disjunction
          all_goals verex_leaf_attempt)
      | verex_trivial))

/-- A prepared goal's unfoldings, positions and determined variants, then
its prepared deciders, else the fallbacks of a leaf. -/
syntax "verex_prepared_decide" : tactic

macro_rules
  | `(tactic| verex_prepared_decide) => `(tactic| verex_focus ( (
      -- The facts a solver's map axioms give hold at the positions the
      -- unfolded definitions read as well.
      (try (verex_timed "p-unfold specs" verex_unfold_specs
            verex_timed "p-map positions" verex_map_positions))
      all_goals (try verex_timed "p-determined" verex_determined_variants)
      all_goals (first | verex_assumption | verex_decide_prepared_goal | verex_leaf_fallbacks))))

macro_rules
  | `(tactic| verex_prepared_case) => `(tactic| verex_focus verex_prepared_decide)

macro_rules
  | `(tactic| verex_prepared_leaf) => `(tactic| verex_focus ( (
      (try verex_timed "positions" verex_unname_positions)
      all_goals verex_prepared_decide)))

/-- The hypotheses a prepared split rewrote normalized again, the others
cached normal. -/
macro "verex_prepare_case" : tactic => `(tactic| verex_focus ( (
  -- A split exposes constructor equations and native certificate facts.
  -- Propagate them before normalizing the case that now knows its variant.
  verex_timed "c-split" verex_split_hypotheses
  all_goals verex_timed "c-subst" verex_subst_vars
  all_goals verex_timed "c-normalize" verex_normalize_context
  all_goals verex_check_native "a case of a prepared leaf")))

/-- Split a prepared goal on the variant of the first local it tests
(`rankedTestedLocals`) that has cases, outer and computed
(`splitVariantCases`), its cases left open (`preparedCaseTag`): a case is a
restart from the prepared leaf with one more assumption, an attempt of its
own (`verex_prepared_case`). -/
elab "verex_split_prepared" : tactic => focusGoal do
  let goal ← getMainGoal
  for local_ in ← rankedTestedLocals goal do
    let cases ← splitVariantCases goal #[local_] true (computation := true)
    if cases == #[goal] then continue
    for case in cases do case.setTag preparedCaseTag
    replaceMainGoal cases.toList
    return
  throwError "the prepared leaf tests no local with variants"

/-- The open prepared goals among `goals`: leaves left for their preparation
(`preparationTag`, `verex_prepared_leaf`) and cases of prepared splits
(`preparedCaseTag`, `verex_prepared_case`), each prepared, then decided
within `attempt`, the cases it leaves in turn; the other goals. The target's
budget alone bounds a preparation, which is proportional to the context and
decides nothing. -/
def decidePreparedCases (goals : List MVarId) (attempt : TacticM Unit → TacticM Unit := id) :
    TacticM (List MVarId) := do
  let mut pending := goals
  let mut others := #[]
  while let goal :: rest := pending do
    pending := rest
    let tag ← goal.getTag
    unless tag == preparationTag || tag == preparedCaseTag do
      others := others.push goal
      continue
    let (preparation, step) ← if tag == preparationTag then
        pure (← `(tactic| verex_timed "prepare" verex_prepare_leaf),
          ← `(tactic| all_goals verex_prepared_leaf))
      else
        pure (← `(tactic| verex_prepare_case), ← `(tactic| all_goals verex_prepared_case))
    let saved ← saveState
    try
      setGoals [goal]
      Lean.Elab.Tactic.withoutRecover (evalTactic preparation)
      let prepared ← getUnsolvedGoals
      let decided ← saveState
      let failed ← tryCatch
        (do
          attempt do
            setGoals prepared
            Lean.Elab.Tactic.withoutRecover (evalTactic step)
          pure none)
        (fun failure => pure (some failure))
      if let some failure := failed then
        -- The refuted alternatives of an abort condition left undecided:
        -- each one source clause, a case of its own over the context
        -- prepared once.
        decided.restore
        let cases ← if tag == preparationTag then
            prepared.foldlM (fun cases goal => return cases ++ (← negatedAlternatives goal)) []
          else pure prepared
        if cases.length == prepared.length then throw failure
        let mut left := []
        for case in cases do
          attempt do
            setGoals [case]
            Lean.Elab.Tactic.withoutRecover (evalTactic step)
          left := left ++ (← getUnsolvedGoals)
        setGoals left
    catch failure =>
      -- Under `verex.solverDebug`, the prepared goal the attempt failed on.
      if verex.solverDebug.get (← getOptions) then
        saved.restore
        IO.println (← (m!"prepared attempt not decided: {failure.toMessageData}").toString)
        setGoals [goal]
        evalTactic (← `(tactic| verex_trace_state "undecided attempt"))
      throw failure
    pending := (← getUnsolvedGoals) ++ pending
  return others.toList

/-- Decide the open cases of prepared splits among the goals, within the
enclosing budget. -/
elab "verex_prepared_cases" : tactic => do
  setGoals (← decidePreparedCases (← getUnsolvedGoals))

/-- Decide a leaf in the normal form an authored proof receives: its
prepared form and the cases of its variant splits, within the enclosing
budget. -/
macro "verex_decide_prepared" : tactic =>
  `(tactic| verex_focus (verex_open_prepared; verex_prepared_cases))

macro_rules
  | `(tactic| verex_decide_prepared_goal) => `(tactic| verex_focus (
      first
      | (verex_prepared_applies
         verex_timed "written" verex_decide_written)
      -- A function value's declared precondition, the table's entry for
      -- its target, decided in its place.
      | (verex_timed "p-behavior" verex_behavior
         all_goals verex_decide_prepared_goal)
      -- Linear arithmetic over the terms the normalization left.
      | verex_timed "p-omega" verex_omega_unsplit
      -- An instance of a quantified hypothesis, as the normal form states both.
      | verex_timed "p-instance" verex_instance
      -- An integer that ranges over a few literal values, case by case.
      | verex_decide_ranges
      -- An existential over a few literal values, at each of them.
      | verex_timed "p-witnesses" verex_goal_witnesses
      -- The variants of the locals the leaf tests, each case an attempt of
      -- its own: after the deciders that need no split, as the cases
      -- multiply.
      | verex_timed "p-variants" verex_split_prepared
      -- Congruence over the terms omega reads as atoms, such as products.
      | verex_timed "p-grind" verex_grind
      | (verex_trace_state "undecided prepared"; fail "the prepared leaf is undecided")))

/-- Decide one leaf cheaply, without rewriting its context. -/
macro "verex_leaf_cheap" : tactic => `(tactic| verex_focus (
  (verex_canonical_families
   verex_clear_computations
   first
   | verex_assumption
   | (verex_split_hypotheses
      verex_decide_residual)
   | verex_budgeted
      (verex_intro
       verex_split_hypotheses
       verex_decide_residual)
   -- The deciders failed on the goal as stated and, where it states a
   -- binder or a premise, under it.  Without this arm the reported failure
   -- is the strict `verex_intro`'s of the arm above, which says the goal
   -- states no binder where the deciders are what left it open.
   | fail "the leaf's deciders left the goal undecided")))

/-- The bounded deciders of the `decide` closer, of the automatic leaf's cascade,
within a speculative attempt's allowance: never the residual attempt's `omega`
over the whole context, which is search the proof did not ask for. -/
macro "verex_decide_bounded" : tactic => `(tactic| verex_focus ( verex_residual_budgeted (
  first
  | done
  | verex_timed "decide cheap" verex_decide_cheap
  | verex_timed "decide instance" verex_instance
  | verex_timed "decide range" verex_range_instance
  | verex_timed "decide context" verex_decide_by_context
  | verex_trivial)))

/-- `verex_leaf_cheap` by the `decide` closer's deciders. -/
macro "verex_leaf_decide" : tactic => `(tactic| verex_focus (
  (verex_canonical_families
   verex_clear_computations
   first
   | verex_assumption
   | (verex_split_hypotheses
      verex_decide_bounded)
   | verex_budgeted
      (verex_intro
       verex_split_hypotheses
       verex_decide_bounded)
   -- The deciders failed on the goal as stated and, where it states a
   -- binder or a premise, under it.  Without this arm the reported failure
   -- is the strict `verex_intro`'s of the arm above, which says the goal
   -- states no binder where the deciders are what left it open.
   | fail "the leaf's deciders left the goal undecided")))

/-- Transport raw-entry memberships through their write equations, then
classify an updated entry by the existing map-membership theorem. The old
entry's nominal invariant is read only at the surviving membership. -/
private def rawEntryMemberships (goal : MVarId) : TacticM (List MVarId) := do
  let equations ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | return found
      unless lhs.isAppOfArity ``VerexIR.SpecEntries.entries 2 &&
          (rhs.isAppOfArity ``VerexIR.SpecMap.update 7 ||
           rhs.isAppOfArity ``VerexIR.SpecMap.remove 6) do return found
      return found.push decl.fvarId
  if equations.isEmpty then return [goal]
  let targets ← goal.getNondepPropHyps
  let some goal ← simpAt goal (← `(tactic| simp only [])) equations targets true
    | return []
  let mut pending : List (MVarId × Array FVarId) := [(goal, #[])]
  let mut finished := []
  while let (goal, classified) :: rest := pending do
    pending := rest
    let member? ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail || classified.contains decl.fvarId then return none
        let type ← instantiateMVars decl.type
        unless type.isAppOfArity ``Membership.mem 5 do return none
        let map := type.getArg! 3
        unless map.isAppOfArity ``VerexIR.SpecMap.update 7 ||
            map.isAppOfArity ``VerexIR.SpecMap.remove 6 do return none
        return some (decl.fvarId, map.isAppOfArity ``VerexIR.SpecMap.update 7)
    if let some (member, update) := member? then
      let proof ← goal.withContext do
        mkAppM (if update then ``VerexIR.SpecMap.mem_update else ``VerexIR.SpecMap.mem_remove)
          #[mkFVar member]
      let (fact, next) ← (← goal.assert `entryOrigin
        (← goal.withContext (inferType proof)) proof).intro1P
      let next ← next.tryClear member
      -- A dependent hypothesis can keep the original membership alive.
      -- Classify each proof once even when it cannot be cleared.
      let classified := classified.push member
      if update then
        pending := ((← next.cases fact).map fun branch => (branch.mvarId, classified)).toList ++ pending
      else pending := (next, classified) :: pending
      continue
    -- A surviving old entry has exactly the invariant quantified over the
    -- old entries. Observe the selected nominal clause at this producer:
    -- certificate extraction has already run before the entry is classified.
    let facts ← goal.withContext do
      let context ← getLCtx
      let mut facts := #[]
      for member in context do
        if member.isImplementationDetail then continue
        let type ← instantiateMVars member.type
        unless type.isAppOfArity ``Membership.mem 5 do continue
        let entry := type.getArg! 4
        for invariant in context do
          if invariant.isImplementationDetail then continue
          let .forallE _ _ (.forallE _ _ body _) _ := invariant.type | continue
          unless body.isAppOfArity ``VerexIR.Proofs.Denote.OwnAtArguments 7 do continue
          let proof? ← (try
            let proof ← mkAppM' invariant.toExpr #[entry, member.toExpr]
            unless ← isTypeCorrect proof do return none
            pure (some proof)
            catch _ => pure none)
          if let some proof := proof? then
            let statement ← inferType proof
            unless (← context.anyM fun decl => withNewMCtxDepth (withReducible (isDefEq decl.type statement))) ||
                (← facts.anyM fun other => do withReducible (isDefEq (← inferType other) statement)) do
              facts := facts.push proof
      return facts
    let mut next := goal
    for proof in facts do
      next ← next.withContext do
        let type ← inferType proof
        let (statement, proof) ← match ← selectedOwnClauses?
            (fun statement => return (← canonicalFamilyResult statement).expr) type proof with
          | some observed => pure observed
          | none => pure (type, proof)
        return (← (← next.assert `entryInvariant statement proof).intro1P).2
    finished := finished ++ [next]
  return finished

/-- The reads of maps written back into tables, `SpecMap.ofEntries key value
(SpecMap.writeBack unread map)`, each once. -/
private def writtenBackReads (e : Lean.Expr) : Array Lean.Expr :=
  let rec walk (e : Lean.Expr) : StateM (Std.HashSet Lean.Expr × Array Lean.Expr) Unit := do
    if (← get).1.contains e then return
    modify fun (seen, found) => (seen.insert e, found)
    if e.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 && !e.hasLooseBVars &&
        e.appArg!.isAppOfArity ``VerexIR.SpecMap.writeBack 6 then
      modify fun (seen, found) => (seen, found.push e)
    match e with
    | .app function argument => walk function; walk argument
    | .lam _ type body _ | .forallE _ type body _ => walk type; walk body
    | .letE _ type value body _ => walk type; walk value; walk body
    | .mdata _ body | .proj _ _ body => walk body
    | _ => pure ()
  ((walk e).run ({}, #[])).2.2

/-- Use a named map's defining read only for the entry roundtrip obligation.
The rest of the leaf keeps the named map and its stated observations. -/
private def entriesSatisfyRead (goal : MVarId) : MetaM (List MVarId) := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``VerexIR.SpecMap.EntriesSatisfy 4 do
    throwError "not an entry roundtrip obligation"
  let map := target.appArg!
  unless map.isFVar do throwError "the map is not named"
  for decl in ← getLCtx do
    let some (_, source, named) := (← instantiateMVars decl.type).eq? | continue
    unless named == map && source.isAppOfArity ``VerexIR.SpecMap.ofEntries 7 do continue
    let same ← mkCongrArg target.appFn! (← mkEqSymm decl.toExpr)
    return [← goal.replaceTargetEq (mkApp target.appFn! source) same]
  throwError "the named map has no defining read"

elab "verex_typed_written_back" : tactic => focusGoal do
    let goal ← getMainGoal
    let reads ← goal.withContext do
      let mut found := writtenBackReads (← instantiateMVars (← goal.getType))
      for decl in ← getLCtx do
        unless decl.isImplementationDetail do
          for read in writtenBackReads (← instantiateMVars decl.type) do
            unless found.contains read do found := found.push read
      pure found
    if reads.isEmpty then return
    -- Each entry reads back: the entries of the maps it is built from, and
    -- each entry an update writes.
    let roundtrip ← `(tactic| (
      repeat (first
        | with_reducible apply VerexIR.SpecMap.entriesSatisfy_update
        | with_reducible apply VerexIR.SpecMap.entriesSatisfy_remove
        | with_reducible apply VerexIR.SpecMap.entriesSatisfy_nil
        | with_reducible apply VerexIR.SpecMap.entriesSatisfy_ofEntries
        | run_tac liftMetaTactic entriesSatisfyRead)
      all_goals (intros; verex_leaf_cheap)))
    -- The length: through the maps it is built from, an update at a key the
    -- context has, an update or a removal within the length.
    let bounded ← `(tactic| (
      repeat (first
        | with_reducible apply VerexIR.SpecMap.size_ofEntries_lt
        | (rw [VerexIR.SpecMap.size_update_sequence, if_pos (by assumption)])
        | with_reducible refine VerexIR.SpecMap.size_update_present_lt (by assumption) (by assumption) ?_
        | with_reducible refine VerexIR.SpecMap.size_update_lt (by assumption) ?_
        | with_reducible apply VerexIR.SpecMap.size_remove_lt)
      all_goals verex_leaf_cheap))
    let mut goal := goal
    let mut equations := #[]
    for read in reads do
      -- A read the context states, such as a definition of the map, or the
      -- bridge of a carrier written back (`writeBackCarrier`), stated where
      -- the deciders see it.
      let stated ← goal.withContext do (← getLCtx).findDeclM? fun decl => do
          if decl.isAuxDecl then return none
          let some (_, lhs, _) := (← instantiateMVars decl.type).eq? | return none
          return if lhs == read then some decl else none
      if let some decl := stated then
        if decl.isImplementationDetail then
          let (equation, next) ← (← goal.withContext do
            goal.assert `writtenBack decl.type decl.toExpr).intro1P
          goal := next
          equations := equations.push equation
        continue
      let proof? ← goal.withContext do
        let rule ← mkConstWithFreshMVarLevels ``VerexIR.SpecMap.ofEntries_writeBack
        let (args, _, statement) ← forallMetaTelescope (← inferType rule)
        let some (_, lhs, _) := statement.eq? | return none
        unless ← isDefEq lhs read do return none
        unless ← closesBy args[8]!.mvarId! bounded do return none
        unless ← closesBy args[9]!.mvarId! roundtrip do return none
        return some (← instantiateMVars (mkAppN rule args))
      let some proof := proof? | continue
      let (equation, next) ← (← goal.withContext do
        goal.assert `writtenBack (← inferType proof) proof).intro1P
      goal := next
      equations := equations.push equation
    replaceMainGoal [goal]
    if equations.isEmpty then return
    let targets ← goal.getNondepPropHyps
    simpMainAt (← `(tactic| simp only [])) equations targets true

elab_rules : tactic
  | `(tactic| verex_written_back) => focusGoal do
    replaceMainGoal (← rawEntryMemberships (← getMainGoal))
    evalTactic (← `(tactic| all_goals verex_typed_written_back))

/-- Available clauses, through conjunctions and diagnostic markers only.
Do not read hypothetical observations inside quantified invariants. -/
private partial def booleanObservationClauses (type : Lean.Expr) : Array Lean.Expr :=
  if type.isArrow then #[type]
  else if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then booleanObservationClauses (type.getArg! 3)
  else if type.isAppOfArity ``And 2 then booleanObservationClauses (type.getArg! 0) ++ booleanObservationClauses (type.getArg! 1)
  else #[]
/-- Conditional equalities grouped by their Boolean guard and polarity.
Duplicate contract facts do not count as independent observations. -/
private def booleanObservations : MetaM (Array (Lean.Expr × Array Lean.Expr × Array Lean.Expr)) := do
  let mut guards : Array (Lean.Expr × Array Lean.Expr × Array Lean.Expr) := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    let arrows := booleanObservationClauses type
    for arrow in arrows do
      let premise := arrow.bindingDomain!
      unless premise.isAppOfArity ``Eq 3 && (premise.getArg! 0).isConstOf ``Bool do continue
      let expr := premise.getArg! 1
      let value := premise.getArg! 2
      if expr.hasLooseBVars || expr.isConstOf ``Bool.true || expr.isConstOf ``Bool.false then continue
      unless value.isConstOf ``Bool.true || value.isConstOf ``Bool.false do continue
      let mut consequence := arrow.bindingBody!
      if consequence.isAppOfArity ``VerexIR.Proofs.Obligation 4 then consequence := consequence.getArg! 3
      unless consequence.isAppOfArity ``Eq 3 do continue
      let previous := guards.findIdx? (fun entry => entry.1 == expr)
      let (_, yes, no) := previous.map (guards[·]!) |>.getD (expr, #[], #[])
      let yes := if value.isConstOf ``Bool.true && !yes.contains consequence then yes.push consequence else yes
      let no := if value.isConstOf ``Bool.false && !no.contains consequence then no.push consequence else no
      guards := match previous with
        | some i => guards.set! i (expr, yes, no)
        | none => guards.push (expr, yes, no)
  return guards
elab "verex_boolean_cases" : tactic => focusGoal do
  let goal ← getMainGoal
  let guards ← goal.withContext booleanObservations
  let some (choice, _, _) := guards.find? (fun (_, yes, no) => yes.size > 1 && no.size > 1)
    | throwError "no shared Boolean observation"
  let expr ← goal.withContext (Lean.Elab.Term.exprToSyntax choice)
  evalTactic (← `(tactic| cases h : $expr:term <;> simp_all))

/-- Compare values whose observations cover both outcomes of a Boolean.
Preparing the leaf identifies unchanged memories, so observations made at
successive opaque calls share a guard. Failed attempts restore the leaf. -/
elab "verex_boolean_observations" : tactic => focusGoal do
  let goal ← getMainGoal
  let guards ← goal.withContext booleanObservations
  let yesCount := guards.foldl (fun count (_, yes, _) => count + yes.size) 0
  let noCount := guards.foldl (fun count (_, _, no) => count + no.size) 0
  unless yesCount > 1 && noCount > 1 do throwError "no repeated Boolean observation"
  evalBudgeted 3000000 (← `(tactic| (
    verex_prepare_leaf
    all_goals verex_boolean_cases
    done)))

/-- Close a case of call observations by arithmetic over its facts, or by
rewriting with them where packed results and memories must agree first. -/
macro "verex_observations_close" : tactic => `(tactic| first
  | verex_arithmetic_only
  | (simp (disch := verex_arithmetic_only)
        [*, VerexIR.Proofs.Obligation_eq, VerexIR.packResults_single]
     all_goals verex_arithmetic_only
     done))

/-- Reuse observations before expanding behavioral alternatives. Arithmetic
names and equal memories must agree with the actual invocation's arguments
before rewriting its packed result. This is the shared walk's bounded early
attempt; a failure restores the leaf and all context-dependent caches. -/
elab "verex_shared_call_observations" : tactic => focusGoal do
  evalBudgeted 3000000 (← `(tactic| (
    verex_canonical_families
    verex_clear_computations
    verex_split_hypotheses
    all_goals verex_subst_vars
    all_goals verex_unname
    all_goals verex_rewrite_results
    all_goals verex_reduce_projections
    all_goals verex_observation_bounds
    all_goals verex_observations_close
    done)))

/-- Solve a call's arithmetic observations using the facts already supplied
by its call rule, before reconstructing runs and deriving their contracts.
All transformations are speculative; callers restore the original leaf on
failure. Each case first keeps singleton results packed until their
invocation is known; where the packed facts do not decide it, the arithmetic
the program named is read as the clauses spell it (`verex_unname`), as a
clause over a call's arguments states their ranges over the specification's
expressions; normalizing the whole case comes last. -/
elab "verex_call_observations" : tactic => focusGoal do
  evalBudgeted 6000000 (← `(tactic| (
    verex_canonical_families
    verex_clear_computations
    verex_split_hypotheses
    all_goals verex_subst_vars
    all_goals verex_reduce_projections
    all_goals verex_observation_bounds
    all_goals first
    | verex_timed "o-packed" verex_observations_close
    | verex_timed "o-unnamed"
        (verex_unname
         verex_rewrite_results
         verex_reduce_projections
         verex_observation_bounds
         verex_observations_close)
    | verex_timed "o-normalized"
        (simp_all only [VerexIR.Proofs.Obligation_eq, VerexIR.packResults_single, vir_denote_norm]
         all_goals verex_omega)
    done)))

/-- An opened entry retains a callee's marked constructor equation, of a
local's encoding or of its native value. Expose just these leading
annotations so substitution can consume the native value before family
conversion and general leaf search. Target markers stay intact. -/
private def entryEncodings (goal : MVarId) : MetaM (Option MVarId) := do
  let equations ← goal.withContext do
    (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let mut type ← instantiateMVars decl.type
      while type.isAppOfArity ``VerexIR.Proofs.Obligation 4 do
        type := type.getArg! 3
      return if fixesLocal type then found.push (decl.fvarId, type)
        else found
  if equations.isEmpty then return none
  let mut goal := goal
  for (id, type) in equations do
    goal ← goal.withContext (goal.replaceLocalDeclDefEq id type)
  return some goal

/-- Entry relations connect rows through tuple and constructor equalities.
Link native constructor facts before general search; normalize their newly
split cases immediately so impossible variants never enter that search.
This is leaf preparation under the same owner, not common preparation with a
renewed slice. General contextual rewriting stays with the leaf pipeline. -/
elab "verex_shared_entry" : tactic => focusGoal do
  withBudget 1000000 <| evalTactic (← `(tactic| (
    verex_clear_computations
    verex_intro_any
    verex_reduce_projections
    first | verex_assumption | skip)))
  if (← getUnsolvedGoals).isEmpty then return
  let saved ← saveState
  let restoreCaches ← saveWalkCaches
  tryCatchRuntimeEx
    (withBudget 3000000 do
      let some goal ← entryEncodings (← getMainGoal) | return
      setGoals [goal]
      Lean.Elab.Tactic.withoutRecover <| evalTactic (← `(tactic| (
        verex_split_hypotheses
        all_goals verex_subst_vars
        all_goals verex_reduce_projections
        all_goals verex_normalize_context_stage))))
    fun _ => do
      saved.restore
      restoreCaches

/-- What an obligation located at an authored clause says: a loop's
invariant is owed at entry and after each iteration, any other clause by the
function. -/
private def clauseFailure (origin : Option Provenance) (snippet : String) : MessageData :=
  match origin with
  | some (.loopEntry _) => m!"the loop invariant `{snippet}` is not established at entry"
  | some (.loopIteration _) => m!"the loop invariant `{snippet}` is not preserved by an iteration"
  | some .assertion => m!"the assertion `{snippet}` does not hold"
  | some .mutationEnd => m!"the data invariant `{snippet}` does not hold where a mutation ends"
  | some .construction =>
      m!"the data invariant `{snippet}` does not hold where a value is constructed"
  | some .handOver =>
      m!"the data invariant `{snippet}` does not hold where a mutable reference is passed to a call"
  | some .memoryWritten =>
      m!"the module invariant `{snippet}` does not hold after a write of memory it reads"
  | some .callReturned =>
      m!"the module invariant `{snippet}` does not hold after a call that writes memory it reads"
  | some .lemmaRequirement => m!"the requirement of the lemma applied at `{snippet}` does not hold"
  | some (.precondition callee none) =>
      m!"the precondition `{snippet}` of `{callee}` does not hold at this call"
  | some (.precondition callee (some call)) =>
      m!"the precondition `{snippet}` of `{callee}` does not hold at call {call}"
  | some .storedWrite =>
      m!"the data invariant `{snippet}` does not hold for a value written to memory"
  | some .ghostWrite =>
      m!"the update `{snippet}` writes a value out of the ghost field type's domain"
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
    if verex.certifyDebug.get (← getOptions) && !(ranges.isEmpty && origin.isNone) then
      logError m!"residual obligation:\n{(← Lean.Meta.ppGoal goal)}"
    return reported

/-- Decide one leaf whose recursive specification functions are unfolded. -/
syntax "verex_leaf_unfolded" : tactic
/-- The decision a leaf's context reads lead to: the repeated Boolean
observation, else the recursive specifications unfolded once and the leaf
decided over them. Named so that `verex_pin_terms` can be tried in front of it
and fully undone when it does not pay for itself: a decision stage must never
make `decide` weaker than it was without the stage. -/
syntax "verex_leaf_decided" : tactic

-- A leaf decided with the cases of its prepared split, within the enclosing
-- budget.
macro_rules
  | `(tactic| verex_leaf) => `(tactic| verex_focus (verex_leaf_attempt; verex_prepared_cases))

macro_rules
  | `(tactic| verex_leaf_attempt) => `(tactic| verex_focus (
      (verex_timed "clear" verex_clear_computations
       first
       -- A stated goal needs no family conversion or context normalization.
       | verex_timed "assumption" verex_assumption
       | verex_timed "stated facts" verex_stated
       -- An impossible path closes before preparing its context. Arithmetic
       -- conclusions wait for normalization, which first computes their guards.
       | verex_timed "early contradiction" (exfalso; verex_omega_unsplit)
       | (verex_timed "vector field reads" verex_vector_field_reads
          all_goals first
          | (intros
             simp (disch := omega) only [Array.getElem?_setIfInBounds_ne,
               Array.getElem?_setIfInBounds_self_of_lt, Option.getD_some]
             done)
          | verex_leaf_attempt)
       | (verex_timed "memory aliases" verex_memory_aliases
          all_goals verex_leaf_attempt)
       | (verex_canonical_families
          verex_timed "early subst" verex_subst_vars
          first | verex_timed "stated after subst" verex_stated | skip
          try verex_timed "memory reads" verex_memory_reads
          try verex_timed "context reads" verex_context_memory_reads
          -- A local this leaf's bounds pin to a term is read as that term by
          -- the decision below, which decides over the facts and the goal
          -- rather than over the local. The pin is an alternative, not a
          -- prefix: where the decision does not close the leaf with it, the
          -- rewriting is undone and the same decision runs on the leaf as it
          -- was, so the stage can only add.
          first
          | (verex_timed "pin terms" verex_pin_terms
             all_goals verex_leaf_decided)
          | verex_leaf_decided))))

macro_rules
  | `(tactic| verex_leaf_decided) => `(tactic|
      first
      | verex_timed "Boolean observations" verex_boolean_observations
      | (try verex_timed "unfold specs" verex_unfold_specs
         all_goals verex_leaf_unfolded))

macro_rules
  | `(tactic| verex_leaf_unfolded) => `(tactic| verex_focus (
      (first
       -- Stored invariants, through the writes to their memory.
       | (verex_timed "stored invariants" verex_data_invariants
          all_goals verex_leaf_attempt)
       -- A goal a hypothesis states, as the unfolding gives it.
       | verex_timed "assumption" verex_assumption
       -- Atomic hypotheses first: each is rewritten, used, and dropped on
       -- its own. A goal whose binders a loop step introduced is an
       -- instance of a quantified hypothesis as much as a quantified one.
       | (try verex_timed "stored reads" verex_stored_reads
          verex_timed "split" verex_split_hypotheses
          -- The variables the destructuring equates, such as a resolved
          -- prophecy and the value written, are one to the deciders, and the
          -- components it projects off literal pairs are the components.
          all_goals verex_timed "subst" verex_subst_vars
          all_goals verex_timed "projections" verex_reduce_projections
          -- The path's Boolean decisions as the propositions they decide,
          -- before the steps that read propositions.
          all_goals try verex_timed "booleans" verex_boolean_decisions
          -- The variants the observations determine, in place. This step,
          -- the positions and modus ponens prepare the cheap deciders'
          -- facts; the prepared form repeats them, so each is a speculative
          -- attempt here (`verex_residual_budgeted`).
          all_goals try verex_timed "determined" (verex_residual_budgeted verex_determined_variants)
          try verex_timed "map contradiction" verex_map_contradiction
          try verex_timed "key instances" verex_key_instances
          try verex_timed "frames" verex_frames
          try verex_timed "map positions" (verex_residual_budgeted verex_map_positions)
          -- A goal its literal inputs decide, before the implications'
          -- premises are decided one by one.
          try verex_timed "ground" (verex_ground_inputs; ((try simp only [VerexIR.Proofs.Obligation_eq,
            Nat.reduceAdd, Int.reducePow, Int.reduceSub, Nat.reducePow, Nat.reduceSub]); verex_ground_arithmetic))
          -- The conditionals of a clause restating the program's, read as
          -- the branch the path's decisions take.
          all_goals try verex_timed "conditionals"
            (verex_residual_budgeted verex_decided_conditionals)
          -- What an implication concludes where its premises hold.
          all_goals try verex_timed "modus ponens" (verex_residual_budgeted verex_modus_ponens)
          all_goals try verex_timed "determined2" (verex_residual_budgeted verex_determined_variants)
          all_goals first
          | verex_timed "witnesses" verex_goal_witnesses
          | verex_timed "cheap" (verex_function_budgeted verex_decide_cheap)
          | verex_timed "instance" verex_instance
          | (verex_timed "assembled witness" verex_assembled_witness
             all_goals verex_leaf_attempt)
          -- A range invariant extended by the iteration: its range split
          -- by the hypothesis, the bound decided as the prepared leaf would.
          | verex_timed "range" verex_range_instance
          | verex_timed "cheap cases" (verex_function_budgeted (verex_cheap_cases 3))
          -- The refuted alternatives of an abort condition the deciders
          -- leave, each by the cheap deciders.
          | (verex_timed "negated alternatives" verex_split_negated_goal
             all_goals (first
               | verex_timed "witnesses" verex_goal_witnesses
               | verex_timed "cheap" (verex_function_budgeted verex_decide_cheap)
               | verex_timed "instance" verex_instance
               | verex_timed "range" verex_range_instance)
             done)
          -- The prepared form, an attempt of its own.
          | verex_timed "prepared" (verex_function_budgeted verex_open_prepared)
          | verex_leaf_fallbacks)
       -- Conjoined clauses, each at an arbitrary instance of its own.
       | verex_budgeted
          (verex_timed "conjuncts" verex_split_conjunction_goal
           all_goals verex_leaf_instance)
       -- A quantified or conditional goal, at an arbitrary instance.
       | verex_budgeted
          verex_leaf_instance)))

/-- A quantified or conditional clause, decided at an arbitrary instance:
its binders and premises introduced, the positions it reads stated, then
the deciders over the instance. -/
macro_rules
  | `(tactic| verex_leaf_instance) => `(tactic|
          (verex_intro
           verex_split_hypotheses
           try verex_timed "frames3" verex_frames
           verex_timed "map positions3" verex_map_positions
           first
           | verex_timed "cheap3" verex_decide_cheap
           -- An instance of a quantified hypothesis, such as a loop
           -- invariant, at the position or one past its range.
           | verex_timed "instance3" verex_instance
           | verex_timed "range3" verex_range_instance_stated
           | verex_timed "prepared3" verex_open_prepared
           | verex_timed "pipeline3" verex_pipeline
           | (verex_timed "cases3" verex_split_disjunction
              all_goals verex_leaf_attempt)
           | verex_trivial))

end VerexIR.Proofs.Denote
