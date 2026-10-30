-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Decide

/-!
# The solver: normalization, splits, behaviors, and invocations

Normalization around a goal's clauses, the structural split of a goal,
the behaviors of function values, and invocations over a model's meaning.
-/

namespace VerexIR.Proofs.Denote

open Lean Elab Tactic Meta

macro "verex_normalize" location:(Lean.Parser.Tactic.location)? : tactic =>
  `(tactic| simp only [vir_denote, vir_denote_norm, vir_denote_eval, Prod.fst, Prod.snd]
    $[$location]?)

/-- The default simp methods, leaving the subterms `stop` holds as they are;
a subterm `rewrite?` steps is stepped so, before. -/
private def stoppingMethods (simprocs : Simp.SimprocsArray) (stop : Lean.Expr → SimpM Bool)
    (rewrite? : Lean.Expr → SimpM (Option Simp.Step) := fun _ => pure none) : Simp.Methods :=
  let base := Simp.mkDefaultMethodsCore simprocs
  { base with
    pre := fun e => do
      if let some step ← rewrite? e then return step
      if ← stop e then return .done { expr := e }
      base.pre e
    dpre := fun e => do
      if ← stop e then return .done e
      base.dpre e }

/-- Normalize the main goal's target as `verex_normalize` does, taking
the subterms in `normal` as they are. The solver keeps every goal it holds
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
      let target ← instantiateMVars (← goal.getType)
      let (result, _) ← Simp.main target ctx
        (methods := stoppingMethods simprocs fun e => pure (known.contains e))
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

/-- The newest hypothesis of the main goal named `name` up to macro scopes,
as a cut introduced it: its name and type. -/
def cutHypothesis? (name : Name) : TacticM (Option (Name × Lean.Expr)) := do
  let some goal := (← getGoals).head? | return none
  goal.withContext do
    let some decl := (← getLCtx).findDeclRev? fun decl =>
        if decl.userName.eraseMacroScopes == name then some decl else none
      | return none
    return some (decl.userName, ← instantiateMVars decl.type)

/-- Normalize a cut's hypothesis again in each goal where the substitutions
after the cut rewrote it, so that it is normal over the values they fix, as
the goal is normalized over them. -/
def renormalizeCut (normalization : Simp.Context × Simp.SimprocsArray)
    (cut : Option (Name × Lean.Expr)) : TacticM Unit := do
  let some (name, normal) := cut | return
  let (ctx, simprocs) := normalization
  let mut goals := #[]
  for goal in ← getGoals do
    let rewritten? ← goal.withContext do
      let some decl := (← getLCtx).findFromUserName? name | return none
      if (← instantiateMVars decl.type) == normal then return none
      return some decl.fvarId
    let some hypothesis := rewritten? | goals := goals.push goal; continue
    let saved ← saveState
    try
      match ← simpGoal goal ctx simprocs (simplifyTarget := false)
          (fvarIdsToSimp := #[hypothesis]) with
      | (none, _) => pure ()
      | (some (_, next), _) => goals := goals.push next
    catch _ =>
      saved.restore
      goals := goals.push goal
  setGoals goals.toList

/-- Normalize a hypothesis that instantiates a normalized predicate at
fresh variables, as `verex_normalize` would. The predicate's
clauses, the contents of its `Obligation` markers, are normal there: only
the structure around them is traversed, where a clause's conjunction
reassociates or a premise curries. The markers stay: an assumed clause
becomes a fact under the clause's name when the hypotheses are split. -/
def normalizeAroundClauses (normalization : Simp.Context × Simp.SimprocsArray)
    (goal : MVarId) (hypothesis : FVarId) (unwrap : Bool) : MetaM (Option MVarId) :=
  goal.withContext do
    let (ctx, simprocs) := normalization
    let clauses ← IO.mkRef ({} : Std.HashSet Lean.Expr)
    -- A clause and its conjuncts, as reassociation regroups them.
    let conjuncts (clause : Lean.Expr) (found : Std.HashSet Lean.Expr) := Id.run do
      let mut found := found
      let mut pending := [clause]
      while !pending.isEmpty do
        let e := pending.head!
        pending := pending.tail
        found := found.insert e
        if e.isAppOfArity ``And 2 then pending := e.appFn!.appArg! :: e.appArg! :: pending
      return found
    let unwrapped (e : Lean.Expr) : SimpM (Option Simp.Step) := do
      unless unwrap && e.isAppOfArity ``VerexIR.Proofs.Obligation 4 do return none
      let clause := e.appArg!
      clauses.modify (conjuncts clause)
      let proof ← mkPropExt (← mkAppOptM ``VerexIR.Proofs.Obligation_iff (e.getAppArgs.map some))
      return some (.done { expr := clause, proof? := some proof })
    let stop (e : Lean.Expr) : SimpM Bool := do
      return e.isAppOfArity ``VerexIR.Proofs.Obligation 4 || (← clauses.get).contains e
    let type ← instantiateMVars (← hypothesis.getType)
    let (result, _) ← Simp.main type ctx (methods := stoppingMethods simprocs stop unwrapped)
    if result.expr == type then return some goal
    return (← applySimpResultToLocalDecl goal hypothesis result false).map (·.2)

/-- The loop a goal's weakest precondition is over, if any: its site, and
the arguments of `wp_loopAt` besides the invariant. -/
def loopGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.loopAt 8 do return none
    let some site ← (evalNat (action.getArg! 5)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The in-body assertion a goal's weakest precondition is over, if any: its
site, and the arguments of `wp_assertAt` besides the condition. -/
def assertionGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.assertAt 7 do return none
    let some site ← (evalNat (action.getArg! 5)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The end of a mutation a goal's weakest precondition is over, if any: its
site, and the arguments of `wp_mutationEndAt` besides the invariant. -/
def mutationEndGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.mutationEndAt 7 do return none
    let some site ← (evalNat (action.getArg! 5)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The end of a write of global memory a goal's weakest precondition is
over, if any: its site, and the arguments of `wp_memoryWrittenAt` besides the
invariant. -/
def memoryWrittenGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.memoryWrittenAt 7 do return none
    let some site ← (evalNat (action.getArg! 5)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The site of the abort a goal's weakest precondition is over, if any
(`wp_abortAt`). -/
def abortAtGoal? (goal : MVarId) : MetaM (Option Nat) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOf ``VerexIR.Proofs.Denote.abortAt && action.getAppNumArgs ≥ 2 do
      return none
    (evalNat (action.getArg! (action.getAppNumArgs - 2))).run

/-- The call a goal's weakest precondition is over, if any: its site, and
the arguments of `wp_callAt`. -/
def callAtGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.callAt 6 do return none
    let some site ← (evalNat (action.getArg! 4)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The in-body assumption a goal's weakest precondition is over, if any:
the arguments of `wp_assumeAt` before its continuation. -/
def assumeGoal? (goal : MVarId) : MetaM (Option (Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.assumeAt 8 do return none
    return some (action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The invariants owed where a call returns and what follows, if a goal
is over them (`CallChecked`). -/
def callCheckedGoal? (goal : MVarId) : MetaM (Option (Lean.Expr × Lean.Expr)) :=
  goal.withContext do
    let target := (← instantiateMVars (← goal.getType)).headBeta
    unless target.isAppOfArity ``VerexIR.Proofs.Denote.CallChecked 2 do return none
    return some (target.getArg! 0, target.getArg! 1)

/-- The construction a goal's weakest precondition is over, if any: its
site, and the arguments of `wp_constructedAt` besides the invariant. -/
def constructionGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.constructedAt 9 do return none
    let some site ← (evalNat (action.getArg! 6)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The ghost write a goal's weakest precondition is over, if any: its site,
and the arguments of `wp_ghostAt`. -/
def ghostWriteGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.ghostAt 9 do return none
    let some site ← (evalNat (action.getArg! 6)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The state anchor a goal's weakest precondition is over, if any: its
site, and the arguments of `wp_anchorAt`. -/
def anchorGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.anchorAt 7 do return none
    let some site ← (evalNat (action.getArg! 5)).run | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- Whether the path records the anchor of a site: the solver records one
where it steps the site's `anchorAt`. -/
def anchorRecorded (goal : MVarId) (site : Nat) : MetaM Bool := goal.withContext do
  for declaration in ← getLCtx do
    let recorded ← instantiateMVars declaration.type
    if recorded.isAppOfArity ``AnchorSaved 5 then
      if (← (evalNat (recorded.getArg! 2)).run) == some site then return true
  return false

/-- A predicate over the locals and state of state anchors, applied to those
the anchors recorded last on the path. -/
partial def resolveAnchors (goal : MVarId) (predicate : Lean.Expr) :
    MetaM Lean.Expr := goal.withContext do
  let .lam _ type _ _ := predicate | return predicate
  unless type.isAppOfArity ``VerexIR.Proofs.Denote.AnchorOf 5 do return predicate
  let some site ← (evalNat (type.getArg! 3)).run
    | throwError m!"an anchor site must be a numeral, not {type.getArg! 3}"
  let mut saved := none
  for declaration in ← getLCtx do
    let recorded ← instantiateMVars declaration.type
    if recorded.isAppOfArity ``AnchorSaved 5 then
      if (← (evalNat (recorded.getArg! 2)).run) == some site then
        saved := some (recorded.getArg! 3, recorded.getArg! 4)
  let some (env, state) := saved
    | throwError m!"the state anchor at site {site} is read where it is not recorded"
  resolveAnchors goal (← whnfR (mkApp predicate (← mkAppM ``Prod.mk #[env, state])))

/-- Split a binder, conjunction or conditional. Dependent actions introduce
  their branch assumptions before normalization. If a computation cannot be
  split, preparation may expose more program steps for the structural walker.
  Without `obligations`, a conditional is split only where the goal still
  runs program: an obligation's own conditionals stay in its leaf. -/
def splitOnce (goal : MVarId) (obligations : Bool := true) (prepare : Bool := true) :
    TacticM (Option (List MVarId × Bool)) := do
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  if target.isForall then
    -- A quantified variable keeps its name, for an authored proof; a
    -- borrow's prophecy mark is consumed.
    let (introduced, next) ← goal.intro1P
    if (prophecyMark? target.bindingDomain!).isSome then
      return some ([← consumeProphecy next introduced], false)
    return some ([next], false)
  if target.isAppOfArity ``And 2 then
    let subgoals ← goal.apply (mkConst ``And.intro)
    return some (subgoals, false)
  -- A dependent action carries its bounds proof into the result. Introduce
  -- the branch assumption before normalization, so the continuation is
  -- processed as computation rather than left beneath a proof binder.
  if target.isAppOfArity ``VerexIR.Proofs.wp 7 then
    let action := target.getArg! 3
    if action.isAppOfArity ``dite 5 then
      let rule := mkAppN (mkConst ``VerexIR.Proofs.wp_dite_rule)
        #[target.getArg! 0, target.getArg! 1, target.getArg! 2,
          action.getArg! 1, action.getArg! 2, action.getArg! 3, action.getArg! 4,
          target.getArg! 4, target.getArg! 5, target.getArg! 6]
      let_expr Iff lhs rhs := ← goal.withContext (do whnfR (← inferType rule))
        | throwError "a dependent conditional rule is not an equivalence"
      setGoals (← goal.apply (mkApp3 (mkConst ``Iff.mpr) lhs rhs rule))
      evalTactic (← `(tactic| constructor <;> intro h))
      return some (← getGoals, true)
  let saved ← saveState
  let running := (target.find? (·.isAppOf ``VerexIR.Proofs.wp)).isSome
  -- An obligation a hypothesis states closes before its cases multiply.
  if obligations && !running then
    try
      setGoals [goal]
      evalTactic (← `(tactic| verex_assumption))
      return some ([], true)
    catch _ =>
      saved.restore
  if obligations || running then
    try
      setGoals [goal]
      evalTactic (← `(tactic| verex_split))
      return some (← getGoals, true)
    catch _ =>
      saved.restore
  -- Preparation may substitute a computed index and expose more program
  -- steps. Resume the structural walker only when the computation changes;
  -- an unchanged wp falls through to the ordinary leaf solver.
  if prepare && target.isAppOfArity ``VerexIR.Proofs.wp 7 then
    try
      setGoals [goal]
      evalTactic (← `(tactic| verex_prepare_leaf))
      let goals ← getGoals
      if goals.isEmpty then return some ([], true)
      if let [next] := goals then
        let nextTarget ← next.withContext (instantiateMVars (← next.getType))
        if nextTarget != target then return some (goals, true)
    catch _ => pure ()
    saved.restore
  return none

/-- Solve a goal `∃ a, slot = some a ∧ P a` (or `∃ a, slot = some a`) whose
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
def recursiveHypothesis? (goal : MVarId) : TacticM (Option (List MVarId)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let recursive := (target.getArg! 3).getAppFn
    unless recursive.isFVar do return none
    -- A call in the loop can leave a source marker on the abort continuation,
    -- while normalization has removed it from the induction hypothesis.
    -- `apply` need not unfold that semireducible marker during unification.
    -- The marker is definitionally its proposition; keep the back edge in
    -- the same form as the hypothesis before matching it.
    let stripAbortMarkers (type : Lean.Expr) := type.replace fun e =>
      if e.isAppOfArity ``VerexIR.Proofs.wp 7 then
        let args := e.getAppArgs
        let abort := args[5]!.replace fun marker =>
          if marker.isAppOfArity ``VerexIR.Proofs.Obligation 4 then some (marker.getArg! 3) else none
        some (mkAppN e.getAppFn (args.set! 5 abort))
      else none
    let unmarked := stripAbortMarkers target
    (← getLCtx).findDeclM? fun decl => do
      if decl.isImplementationDetail then return none
      let ty ← instantiateMVars decl.type
      unless ty.isForall do return none
      let conclusion := ty.getForallBody
      unless conclusion.isAppOfArity ``VerexIR.Proofs.wp 7 &&
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
        if verex.solverDebug.get (← getOptions) then
          IO.println s!"recursive hypothesis mismatch: {← failure.toMessageData.toString}"
        return none

/-- The callee a goal's weakest precondition is over, if any, with the
action: the handle of a `propheticMeaning` application, or a callee given
as a local function, as a function calling itself is. -/
def callGoal? (goal : MVarId) (callees : Array (Lean.Expr × String × Lean.Expr)) :
    MetaM (Option (Lean.Expr × Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    if action.isAppOfArity ``VerexIR.Proofs.Denote.propheticMeaning 9 then
      return some (action.getArg! 5, action)
    let head := action.getAppFn
    if head.isFVar && callees.any (·.1 == head) then return some (head, action)
    return none

/-- The contract constants a callee theorem speaks about, to unfold at a
call site: the typed contract and the raw contract it wraps. -/
def contractConstants (theoremType : Lean.Expr) : MetaM (Array Name) := do
  let type ← instantiateMVars theoremType
  let type := type.getForallBody
  unless type.isAppOfArity ``VerexIR.Proofs.Satisfies 6 do return #[]
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
    (arguments : Array Lean.Expr) : MetaM Lean.Expr := do
  -- The unit, its function values, its executable and the frame lead the
  -- rule's binders, in either order of the frame and the executable.
  let functions ← functionCarriersOfFrame frame
  let roles ← forallBoundedTelescope (← getConstInfo name).type (some 4) fun binders _ =>
    binders.mapM fun binder => do
      let decl ← binder.fvarId!.getDecl
      if decl.type.isAppOf ``FunctionCarriers then return functions
      if decl.binderInfo.isInstImplicit then return frame
      if decl.type.isAppOf ``VerexIR.Validation.ExecutableUnit then return executable
      return unit
  mkAppM' (mkAppN (mkConst name) roles) arguments

/-- A statement without its obligation marker. -/
def unmarked (statement : Lean.Expr) : Lean.Expr :=
  if statement.isAppOfArity ``VerexIR.Proofs.Obligation 4 then statement.appArg! else statement

/-- The results and final memory a run states. -/
private structure BehaviorRun where
  result : Lean.Expr
  post : Lean.Expr

/-- An `ensures_of` or `aborts_of` statement's parts: the run it states, if
one, the invocation meaning, the supplied row, its shared flags, the result
shape, the function value, the native arguments, and the state it starts
from. -/
private structure BehaviorView where
  run? : Option BehaviorRun
  invoke : Lean.Expr
  supplied : Lean.Expr
  shared : Lean.Expr
  shape : Lean.Expr
  function : Lean.Expr
  arguments : Lean.Expr
  state : Lean.Expr

/-- The parts of an `ensures_of` or `aborts_of` statement. -/
private def behaviorView? (statement : Lean.Expr) : Option BehaviorView :=
  if statement.isAppOfArity ``VerexIR.Proofs.EnsuresOf 12 then
    some {
      run? := some { result := statement.getArg! 9, post := statement.getArg! 11 }
      invoke := statement.getArg! 3, supplied := statement.getArg! 4
      shared := statement.getArg! 5, shape := statement.getArg! 6
      function := statement.getArg! 7, arguments := statement.getArg! 8
      state := statement.getArg! 10 }
  else if statement.isAppOfArity ``VerexIR.Proofs.AbortsOf 10 then
    some {
      run? := none, invoke := statement.getArg! 3, supplied := statement.getArg! 4
      shared := statement.getArg! 5, shape := statement.getArg! 6
      function := statement.getArg! 7, arguments := statement.getArg! 8
      state := statement.getArg! 9 }
  else none

/-- A known function value (`LiteralValue`: a construction or a closure
site's constructor) inside an expression. -/
private def literalFunction? (environment : Environment) (e : Lean.Expr) : Option Lean.Expr :=
  e.find? (isLiteralValue environment)

/-- The executable unit an invocation meaning runs: the closures' meaning
over it (`closureMeaning`); none for a model's meaning. -/
private def executableOf? (invoke : Lean.Expr) : Option Lean.Expr :=
  let invoke := invoke.consumeMData
  if invoke.isAppOfArity ``closureMeaning 4 then some (invoke.getArg! 2) else none

/-- Whether a statement is `ensures_of` or `aborts_of` under premises. -/
private partial def states (statement : Lean.Expr) : Bool :=
  if statement.isArrow then states statement.bindingBody!
  else
    let statement := unmarked statement
    statement.isAppOf ``VerexIR.Proofs.EnsuresOf || statement.isAppOf ``VerexIR.Proofs.AbortsOf

/-- Behavioral contract derivation only handles literal closures. An abstract
function value has no target theorem to re-derive, so trying to avoid that
work by preparing the whole context would add only speculative cost. -/
private partial def statesLiteralBehavior (environment : Environment) (statement : Lean.Expr) :
    Bool :=
  if statement.isArrow then statesLiteralBehavior environment statement.bindingBody!
  else
    match behaviorView? (unmarked statement) with
    | some view => (literalFunction? environment view.function).isSome
    | none => false

/-- Whether a premise of a goal's binders states `ensures_of` or `aborts_of`. -/
private partial def spineStates : Lean.Expr → Bool
  | .forallE _ domain body _ => states domain || spineStates body
  | _ => false

/-- A goal's `ensures_of` and `aborts_of` as hypotheses: the premises of its
spine that state them, and a negated one, whose clause keeps its marker on
`False`. -/
def introduceBehaviors (goal : MVarId) : TacticM MVarId := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let goal ← if spineStates target then pure (← introduceThroughMarkers goal).1 else pure goal
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let negated := unmarked target
  unless negated.isAppOfArity ``Not 1 && states negated.appArg! do return goal
  let goal ← if target.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
      let marked := mkAppN target.getAppFn (target.getAppArgs.pop.push (mkConst ``False))
      goal.replaceTargetDefEq (.forallE `verexAborts negated.appArg! marked .default)
    else pure goal
  return (← goal.intro1P).2

/-- Whether an alternative is an abort of a literal closure. -/
private partial def literalAbortAlternative (environment : Environment) (statement : Lean.Expr) :
    Bool :=
  let statement := unmarked statement
  if statement.isAppOfArity ``Or 2 then
    literalAbortAlternative environment (statement.getArg! 0) ||
      literalAbortAlternative environment (statement.getArg! 1)
  else
    statement.isAppOfArity ``VerexIR.Proofs.AbortsOf 10 && statesLiteralBehavior environment statement

/-- Expose the abort alternatives of a literal closure's contract before
reading another invocation's result. A disjunction only states that one run
aborted, or that a call's other abort condition holds; it does not decide
the individual runs. Split only a disjunction an alternative of which is a
literal closure's abort, including the source clause's marker: that case is
then refuted by the closure's target. -/
def behaviorAlternatives? (goal : MVarId) : TacticM (Option (List MVarId)) := do
  let some (hypothesis, statement) ← goal.withContext do
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let statement := unmarked (← instantiateMVars decl.type)
        if statement.isAppOfArity ``Or 2 && literalAbortAlternative (← getEnv) statement then
          return some (decl.fvarId, statement)
      return none
    | return none
  -- The marker is definitionally its proposition, but the solver keeps it
  -- opaque to ordinary unification so diagnostics retain the source clause.
  let goal ← goal.replaceLocalDeclDefEq hypothesis statement
  return some ((← goal.cases hypothesis).map (·.mvarId)).toList

/-- Reference-freedom of a row or type, decided by the kernel. -/
private def decidedTrue (statement : Lean.Expr) : MetaM Lean.Expr := do
  mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true)) (← mkEq statement (mkConst ``Bool.true))

/-- The weave and the captured and supplied rows of a known function
value. -/
private def closureWeave? (literal : LiteralValue) :
    MetaM (Option (Lean.Expr × Lean.Expr × Lean.Expr)) := do
  let mask := literal.mask
  unless mask.isAppOfArity ``Weave.mask 4 do return none
  let weave := mask.appArg!
  let rows := (← whnfR (← inferType weave)).getAppArgs
  let (some captured, some supplied) := (rows[1]?, rows[2]?) | return none
  return some (weave, captured, supplied)

/-- What a leaf learns from `ensures_of` and `aborts_of` of a closure whose
target, weave, and captures it sees, and whose target is a verified
callee: where the leaf establishes the target's precondition at the woven
arguments, what the target's contract ensures of the run, or the failures it
permits (`ensuresOf_closureOf_verified`, `abortsOf_closureOf_verified`).
`none` where the requires meaning closes the goal. -/
def dispatchBehavior (goal : MVarId) (callees : Array (Lean.Expr × String × Lean.Expr)) :
    TacticM (Option MVarId) := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  -- `requires_of`: the value's requires meaning.
  if (unmarked target).isAppOf ``VerexIR.Proofs.RequiresOf then
    let state ← saveState
    try
      setGoals [goal]
      evalTactic (← `(tactic| verex_behavior))
      return (← getGoals).head?
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
    if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
      stated := mkApp4 (mkConst ``Iff.mp) type type.appArg!
        (mkAppN (mkConst ``VerexIR.Proofs.Obligation_iff) type.getAppArgs) stated
      type := type.appArg!
    let some view := behaviorView? type | continue
    -- A predicate over a model's invocation meaning is read by its run
    -- (`modelDenotedRun`).
    let some executable := executableOf? view.invoke | continue
    let some found := literalFunction? (← getEnv) view.function | continue
    let some closure ← literalValue? found | continue
    let some (weave, captured, _) ← closureWeave? closure | continue
    let handle := closure.handle
    -- A target the call rule inlines is listed by its theorem as well.
    let some (_, _, theoremProof) ← callees.findM? fun (candidate, _, proof) => do
        unless ← isDefEq candidate handle do return false
        forallTelescope (← inferType proof) fun _ statement =>
          pure (statement.isAppOfArity ``VerexIR.Proofs.Satisfies 6)
      | continue
    let (theoremArguments, _, statement) ← forallMetaTelescope (← inferType theoremProof)
    unless statement.isAppOfArity ``VerexIR.Proofs.Satisfies 6 do continue
    let meaning := statement.getArg! 4
    unless meaning.isAppOf ``propheticMeaning do continue
    -- Fix the target theorem's family before unifying its dependent contract.
    -- Scalar argument carriers alone do not determine this implicit family.
    unless ← isDefEq (meaning.getArg! 3) closure.frame do continue
    let verified ← instantiateMVars (mkAppN theoremProof theoremArguments)
    -- A closure of a target without type arguments runs at the empty
    -- instantiation, which the runtime frame is coherent with.
    let coherent ← mkAppM ``coherent_runtime #[type.getArg! 0, handle]
    let capturedFree ← decidedTrue (mkApp (mkConst ``NRow.refFree) captured)
    let rule := if view.run?.isSome then ``VerexIR.Proofs.ensuresOf_closureOf_verified
      else ``VerexIR.Proofs.abortsOf_closureOf_verified
    let implication? ← try
        some <$> mkAppAtFrame rule (type.getArg! 0) executable closure.frame
          #[weave, capturedFree, coherent, closure.captures, view.arguments, verified, stated,
            closure.closed]
      catch ex =>
        if verex.solverDebug.get (← getOptions) then
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
      if verex.solverDebug.get (← getOptions) then IO.println "behavior dispatch: theorem assumptions missing"
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
          ((try simp only [$lemmas,*, Weave.compose, vir_denote_norm]) <;> verex_leaf))) do
        established := false
        break
      fact := mkApp fact (← instantiateMVars premise)
    unless established do
      if verex.solverDebug.get (← getOptions) then IO.println "behavior dispatch: contract premises unproved"
      continue
    let (_, next) ← (← current.assert `verexBehavior (← inferType fact) fact).intro1P
    current := next
    let state ← saveState
    try
      setGoals [current]
      -- The failures a contract permits, each absent: the negated
      -- disjunction as the conjunction of the negations.
      evalTactic (← `(tactic| simp only [$lemmas,*, Weave.compose, vir_denote_norm,
        not_false_eq_true, true_implies, not_or] at $(mkIdent `verexBehavior):ident))
      current ← getMainGoal
    catch _ => state.restore
  return some current

/-- Whether a statement reads a behavioral predicate. -/
private def readsBehavior (statement : Lean.Expr) : Bool :=
  (statement.find? fun e => e.isAppOf ``VerexIR.Proofs.RequiresOf ||
    e.isAppOf ``VerexIR.Proofs.AbortsOf || e.isAppOf ``VerexIR.Proofs.EnsuresOf).isSome

/-- `next` with the facts `before` does not state out of their clause
markers: the deciders read a derived fact's statement. -/
private def unmarkIntroduced (before next : MVarId) : MetaM MVarId := do
  let known ← before.withContext do return (← getLCtx).getFVarIds
  next.withContext do
    let mut goal := next
    for decl in ← getLCtx do
      if decl.isImplementationDetail || known.contains decl.fvarId then continue
      let type ← instantiateMVars decl.type
      if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
        goal ← goal.replaceLocalDeclDefEq decl.fvarId type.appArg!
    return goal

/-- An existential over a state label whose body states behavioral
predicates, witnessed at the first program point that decides its body: each
conjunct, instantiated there, derived through the literal closures' targets
(`dispatchBehavior`: the table of declared preconditions, verified
contracts) and decided by the cheap deciders. -/
def behaviorWitness (goal : MVarId) (callees : Array (Lean.Expr × String × Lean.Expr)) :
    TacticM Bool := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let statement := unmarked target
  let some (domain, _) := statement.app2? ``Exists | return false
  unless domain.isAppOf ``VerexIR.Proofs.Denote.Memory && readsBehavior statement do return false
  let original ← saveState
  let saved ← getGoals
  let goal ← if target == statement then pure goal else goal.replaceTargetDefEq statement
  goal.withContext do
  let debug := verex.solverDebug.get (← getOptions)
  let points ← programPoints
  if debug then IO.println s!"behavior witness: {points.size} program points"
  for (value, state) in points do
    let some (chosen, body) ← pointWitness? value state statement
      | if debug then IO.println s!"behavior witness: no witness at {← ppExpr value}"
        continue
    let attempt ← saveState
    let instantiated ← mkFreshExprSyntheticOpaqueMVar body
    setGoals [instantiated.mvarId!]
    evalTactic (← `(tactic| repeat' apply And.intro))
    let mut decided := true
    for conjunct in ← getGoals do
      let some dispatched ← dispatchBehavior conjunct callees | continue
      let next ← unmarkIntroduced conjunct dispatched
      unless ← closesBy next (← `(tactic| verex_leaf_cheap)) do
        if debug then
          IO.println s!"behavior witness: undecided at a point\n{← ppGoal next}"
        decided := false
        break
    if decided then
      assignWitnessed goal chosen (← instantiateMVars instantiated)
      setGoals saved
      return true
    attempt.restore
  original.restore
  return false

/-- A runtime value built of encodings typed at a closed semantic type:
scalars, integers by their certified bounds or a literal's, and structures
whose declaration the unit gives, by evaluation over it. -/
syntax "verex_has_type" : tactic
/-- A row of such values typed at a row of types. -/
syntax "verex_has_types" : tactic
macro_rules
  | `(tactic| verex_has_type) => `(tactic| verex_focus ( first
      | exact VerexIR.HasType.bool _
      | exact VerexIR.HasType.address _
      | exact VerexIR.HasType.signer _
      | exact VerexIR.HasType.string _
      | exact VerexIR.HasType.bytes _
      | exact VerexIR.HasType.unit
      | (refine VerexIR.HasType.integer _ _ _ ?_
         first | exact VerexIR.Proofs.holdsAt_val _ _ | decide)
      | (refine VerexIR.HasType.nominal _ _ _ _ [] _ _ _ rfl rfl
           (VerexIR.Proofs.ResolvesAll.ofResolveIn rfl) ?_
         verex_has_types)))
macro_rules
  | `(tactic| verex_has_types) => `(tactic| verex_focus ( first
      | exact VerexIR.HasTypes.nil
      | (refine VerexIR.HasTypes.cons ?_ ?_
         · verex_has_type
         · verex_has_types)))

/-- A memory's runtime encodings have typed representations: every
memory's do, for a unit whose resource types correspond to their keys' types
and whose typed values are representable, which hypotheses state
(`memoryTyped`). -/
private def proveMemoryTyped (facts : Array Lean.Expr) (goal : MVarId) : TacticM Bool := do
  let type ← goal.withContext do instantiateMVars (← goal.getType)
  let_expr VerexIR.Proofs.MemoryTyped unit functions memory := type | return false
  let find (constant : Name) : TacticM (Option Lean.Expr) := facts.findM? fun fact => do
    let factType ← goal.withContext do instantiateMVars (← inferType fact)
    unless factType.isAppOfArity constant 1 do return false
    goal.withContext (isDefEq (factType.getArg! 0) unit)
  let some resources ← find ``VerexIR.Proofs.ResourcesTyped | return false
  let some representable ← find ``VerexIR.Proofs.OwnersRepresentable | return false
  goal.assign
    (mkApp5 (mkConst ``VerexIR.Proofs.memoryTyped) unit functions resources representable memory)
  return true

/-- Inspect a frame under a source clause's diagnostic marker. -/
private partial def frameFactType (type : Lean.Expr) : MetaM Lean.Expr := do
  let type ← instantiateMVars type
  if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
    frameFactType type.appArg!
  else pure type

/-- Remove diagnostic markers from a frame proof before applying it. -/
private partial def frameFactProof (fact : Lean.Expr) : MetaM Lean.Expr := do
  let type ← instantiateMVars (← inferType fact)
  if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
    frameFactProof (mkApp4 (mkConst ``Iff.mp) type type.appArg!
      (mkAppN (mkConst ``VerexIR.Proofs.Obligation_iff) type.getAppArgs) fact)
  else pure fact

/-- The parameter row and function value of a stored frame hypothesis. -/
private def framedFact? (type : Lean.Expr) : MetaM (Option (Lean.Expr × Lean.Expr)) := do
  let type ← frameFactType type
  if type.isAppOfArity ``VerexIR.Proofs.KeepsMemoryAt 8 then
    return some (type.getArg! 4, type.getArg! 7)
  if type.isAppOfArity ``VerexIR.Proofs.FramedAt 9 then
    return some (type.getArg! 4, type.getArg! 8)
  if type.isAppOfArity ``VerexIR.Proofs.KeepsMemoryAtOf 8 then
    return some (type.getArg! 4, type.getArg! 7)
  if type.isAppOfArity ``VerexIR.Proofs.FramedAtOf 9 then
    return some (type.getArg! 4, type.getArg! 8)
  return none

/-- A run of a function value a hypothesis frames (`KeepsMemoryAt`,
`FramedAt`) keeps the frame: of a value that keeps memory, the run's final
memory, `verexFinal`, is substituted by its initial one; of another, its
frame at the run's arguments is a hypothesis. -/
private def keptMemory (goal : MVarId) (action : Lean.Expr) (facts : Array Lean.Expr) :
    TacticM MVarId := do
  let kept? ← goal.withContext do
    let some ensures := (← getLCtx).findFromUserName? `verexEnsures | return none
    let ensuresType ← instantiateMVars ensures.type
    unless ensuresType.isAppOfArity ``VerexIR.Proofs.EnsuresOf 12 do return none
    let some executable := executableOf? (ensuresType.getArg! 3) | return none
    for fact in facts do
      let factType ← frameFactType (← inferType fact)
      let some (row, closure) ← framedFact? factType | continue
      unless factType.isAppOf ``VerexIR.Proofs.FramedAt ||
          factType.isAppOf ``VerexIR.Proofs.KeepsMemoryAt do continue
      unless ← isDefEq closure (action.getArg! 7) do
        if verex.solverDebug.get (← getOptions) then logInfo m!"frame closure mismatch: {closure} vs {action.getArg! 7}"
        continue
      unless ← isDefEq (factType.getArg! 2) executable do continue
      let typed ← mkFreshExprMVar (← mkAppM ``VerexIR.Proofs.MemoryTyped
        #[ensuresType.getArg! 0, ensuresType.getArg! 10])
      unless ← proveMemoryTyped facts typed.mvarId! do continue
      let plain := mkApp (mkConst ``NRow.refFree) row
      unless ← isDefEq plain (mkConst ``Bool.true) do continue
      let shape := ensuresType.getArg! 6
      let shapePlain := mkApp (mkConst ``NRow.refFree) (← mkAppM ``ResultShape.row #[shape])
      unless ← isDefEq shapePlain (mkConst ``Bool.true) do continue
      let kept := mkAppN (← frameFactProof fact) #[← decidedTrue plain,
        ← decidedTrue shapePlain, ensuresType.getArg! 8, ensuresType.getArg! 9,
        ensuresType.getArg! 10, ensuresType.getArg! 11, ← instantiateMVars typed, ensures.toExpr]
      try check kept catch error =>
        if verex.solverDebug.get (← getOptions) then logInfo m!"frame application: {error.toMessageData}"
        continue
      return some (kept, factType.isAppOfArity ``VerexIR.Proofs.KeepsMemoryAt 8)
    return none
  let some (kept, keeps) := kept? | return goal
  let (equation, next) ← goal.withContext do
    (← goal.assert `verexKept (← whnfR (← inferType kept)).headBeta kept).intro1P
  -- Substitute using the frame equation itself. Substituting `verexFinal`
  -- by name can select the earlier StateOf equation instead, leaving the
  -- memory hidden behind the invocation's state-label expression.
  if keeps then next.withContext (Lean.Meta.subst next equation) else pure next

/-- A row the frame resolves to scalars (`ScalarAt`): a hypothesis of the
theorem, or by evaluation where the frame is the runtime frame or one a
call's type arguments induce. -/
def scalarAt? (facts : Array Lean.Expr) (goal : MVarId) : TacticM Bool := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``VerexIR.Proofs.ScalarAt 4 do return false
  for fact in facts do
    let factType ← goal.withContext do instantiateMVars (← inferType fact)
    unless factType.isAppOfArity ``VerexIR.Proofs.ScalarAt 4 do continue
    if ← goal.withContext do isDefEq factType target then
      goal.assign fact
      return true
  let state ← saveState
  try
    setGoals [goal]
    evalTactic (← `(tactic|
      (simp only [VerexIR.Proofs.ScalarAt, VerexIR.Proofs.Denote.NRow.resolved,
          VerexIR.Proofs.Denote.Skolems.resolve_instantiate,
          VerexIR.Proofs.Denote.Skolems.resolve_runtime,
          VerexIR.Proofs.Denote.Skolems.resolve_paramFree, VerexIR.Proofs.Denote.NTy.paramFree,
          VerexIR.Proofs.Denote.NRow.paramFree, VerexIR.Proofs.Denote.NRows.paramFree,
          VerexIR.Proofs.Denote.NTy.subst, VerexIR.Proofs.Denote.NRow.subst,
          VerexIR.Proofs.Denote.NRows.subst, VerexIR.Proofs.Denote.NRow.getD,
          Bool.and_true, Bool.true_and]
       first | rfl | decide)))
    unless (← getGoals).isEmpty do throwError "a row is not scalar"
    return true
  catch _ =>
    state.restore
    return false

/-- Whether a function value is a known one (`LiteralValue`). -/
private def isKnownFunction (e : Lean.Expr) : MetaM Bool :=
  return isLiteralValue (← getEnv) e.consumeMData

/-- An invocation of a function value the proof does not see: the rule with
the theorem's typing assumptions (`wp_closureMeaning_unseen_at`). Its
runs and aborts remain, stated by the behavioral predicates. -/
def unseenInvocation? (goal : MVarId) : TacticM (Option (List MVarId)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := (target.getArg! 3).consumeMData
    unless action.isAppOfArity ``closureMeaning 9 do return none
    if ← isKnownFunction (action.getArg! 7) then return none
    let original ← saveState
    let result ← do
      -- A closure read from a resource gets its field frame from that value's
      -- stored invariant. Recover it before recording the invocation's final
      -- memory, so the call can use the frame just as a parameter call does.
      let goal ← if !(← readsStoredInvariants goal) then pure goal else do
        let saved ← saveState
        try
          setGoals [goal]
          evalTactic (← `(tactic| verex_split_hypotheses))
          evalTactic (← `(tactic| verex_stored_reads))
          evalTactic (← `(tactic| simp_all only [Which.project?_eq_some_iff,
            Which.inject_here, Which.inject_there, vir_denote_norm]))
          -- Tuple/enum matches expose another payload equation after the
          -- first normalization. Substitute it before matching the frame to
          -- the closure being invoked.
          for _ in [0:2] do
            evalTactic (← `(tactic| verex_split_hypotheses))
            evalTactic (← `(tactic| verex_subst_vars))
            evalTactic (← `(tactic| verex_reduce_projections))
            evalTactic (← `(tactic| verex_normalize_hypotheses))
          getMainGoal
        catch error =>
          if verex.solverDebug.get (← getOptions) then
            IO.println s!"stored closure frame: {← error.toMessageData.toString}"
          saved.restore
          pure goal
      goal.withContext do
        let target ← instantiateMVars (← goal.getType)
        unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
        let action := (target.getArg! 3).consumeMData
        unless action.isAppOfArity ``closureMeaning 9 do return none
        let mut facts := #[]
        for declaration in ← getLCtx do
          if declaration.isImplementationDetail then continue
          if ← isProp declaration.type then
            facts := facts ++ (← conjuncts declaration.toExpr)
        let state ← saveState
        let rule ← mkConstWithFreshMVarLevels ``VerexIR.Proofs.wp_closureMeaning_unseen_at
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
          else if type.isAppOfArity ``VerexIR.Proofs.MemoryTyped 3 then
            found ← proveMemoryTyped facts id
          else if type.isAppOfArity ``VerexIR.Proofs.ScalarAt 4 then
            found ← scalarAt? facts id
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
          [`verexResult, `verexFinal, `verexEnsures, `verexUnaborted, `verexResultOf,
            `verexStateOf]
        let returns ← keptMemory returns action facts
        return some [returns, fails]
    -- Preparation can assign the original goal. A failed match must undo
    -- that preparation as well as the attempted invocation rule.
    if result.isNone then original.restore
    return result

/-- A literal closure keeps a frame (`FramedAt`, `KeepsMemoryAt`) where its
target's theorem applies at every typed memory and its frame lies within it
(`FramedAt.ofVerified`), or else where its target's body keeps it, an
obligation over every argument and typed memory (`FramedAt.ofBody`); any
function value does at a row with references, where the frame is vacuous.
The goals left, if the closure's frame is decided. -/
partial def literalClosureKeepsMemory? (goal : MVarId)
    (callees : Array (Lean.Expr × String × Lean.Expr)) : TacticM (Option (List MVarId)) := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let target := if target.isAppOfArity ``VerexIR.Proofs.Obligation 4 then target.appArg! else target
  let target ← if target.isAppOfArity ``VerexIR.Proofs.KeepsMemoryAt 8 then
      goal.withContext (whnfR target) else pure target
  unless target.isAppOfArity ``VerexIR.Proofs.FramedAt 9 do return none
  if verex.solverDebug.get (← getOptions) then logInfo "frame: inspecting closure"
  let held ← goal.withContext do
    let plain := mkApp (mkConst ``NRow.refFree) (target.getArg! 4)
    if ← isDefEq plain (mkConst ``Bool.false) then
      some <$> mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.false))
        (← mkEq plain (mkConst ``Bool.false))
    else pure none
  if let some held := held then
    goal.assign (← goal.withContext do
      mkAppOptM ``VerexIR.Proofs.FramedAt.of_refFree_false
        #[target.getArg! 0, target.getArg! 1, target.getArg! 2, target.getArg! 3,
          target.getArg! 4, target.getArg! 5, target.getArg! 6, target.getArg! 7, target.getArg! 8,
          held])
    return some []
  -- A frame at the frame a call's type arguments induce is the frame read in
  -- the caller's view, at the caller's (`FramedAt.of_instantiate`): a stored
  -- closure's frame a generic callee requires is the one its caller holds.
  if let some (θ, outer) ← goal.withContext do
      let_expr VerexIR.Proofs.Denote.Skolems.instantiate _ _ θ outer :=
          (target.getArg! 3).consumeMData
        | return none
      return some (θ, outer) then
    let next? ← try goal.withContext do
      let free ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
        (← mkEq (mkApp (mkConst ``NRow.refFree) (← mkAppM ``Subtype.val #[θ])) (mkConst ``Bool.true))
      let rule ← mkAppOptM ``VerexIR.Proofs.FramedAt.of_instantiate
        #[target.getArg! 0, target.getArg! 1, target.getArg! 2, outer, θ, free, target.getArg! 4,
          target.getArg! 5, target.getArg! 6, target.getArg! 7, target.getArg! 8]
      let [next] ← goal.apply rule | return none
      pure (some next)
      catch _ => pure none
    if let some next := next? then
      return ← literalClosureKeepsMemory? next callees
  -- A frame a hypothesis states of the same function value, within it.
  if verex.solverDebug.get (← getOptions) then logInfo "frame: checking stated frames"
  let stated ← try goal.withContext do
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some (_, closure) ← framedFact? decl.type | continue
      unless ← isDefEq closure (target.getArg! 8) do continue
      let narrow ← whnfR (← frameFactType decl.type)
      unless narrow.isAppOfArity ``VerexIR.Proofs.FramedAt 9 do continue
      let .forallE _ argumentsType _ _ ← whnfR (← inferType (target.getArg! 7)) | continue
      let memoryType :=
        mkApp2 (mkConst ``VerexIR.Proofs.Denote.Memory) (target.getArg! 0) (target.getArg! 1)
      let withinType ← withLocalDeclD `verexArguments argumentsType fun args =>
        withLocalDeclD `pre memoryType fun pre =>
          withLocalDeclD `post memoryType fun post => do
            mkForallFVars #[args, pre, post] (← mkArrow
              (mkApp3 (narrow.getArg! 7) args pre post).headBeta
              (mkApp3 (target.getArg! 7) args pre post).headBeta)
      let within ← mkFreshExprSyntheticOpaqueMVar withinType
      unless ← closesBy within.mvarId! (← `(tactic| (
          intro _ _ _ $(mkIdent `verexFrame):ident
          first
          | exact $(mkIdent `verexFrame)
          | verex_bounded
              (simp only [vir_denote_norm, vir_denote_eval] at $(mkIdent `verexFrame):ident ⊢
               grind)))) do continue
      return some (← mkAppAtFrame ``VerexIR.Proofs.FramedAt.mono
          (target.getArg! 0) (target.getArg! 2) (target.getArg! 3)
        #[← mkExpectedTypeHint (← frameFactProof decl.toExpr) narrow, ← instantiateMVars within])
    return none
    catch _ => pure none
  if let some proof := stated then
    if verex.solverDebug.get (← getOptions) then logInfo "frame: applying stated frame"
    goal.assign proof
    return some []
  if verex.solverDebug.get (← getOptions) then logInfo "frame: checking literal target"
  let some closure ← goal.withContext (literalValue? (target.getArg! 8)) | return none
  let mask := closure.mask
  unless mask.isAppOfArity ``Weave.mask 4 do return none
  goal.withContext do
  let weave := mask.appArg!
  let rows := (← whnfR (← inferType weave)).getAppArgs
  let some captured := rows[1]? | return none
  let handle := closure.handle
  let free := fun (statement : Lean.Expr) => do
    mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true)) (← mkEq statement (mkConst ``Bool.true))
  let coherent ← mkAppM ``coherent_runtime
    #[target.getArg! 0, handle]
  let state ← saveState
  try
    let some (_, _, theoremProof) ← callees.findM? fun (candidate, _, proof) => do
        unless ← isDefEq candidate handle do return false
        forallTelescope (← inferType proof) fun _ statement =>
          pure (statement.isAppOfArity ``VerexIR.Proofs.Satisfies 6)
      | throwError "the target has no theorem"
    let (theoremArguments, _, statement) ← forallMetaTelescope (← inferType theoremProof)
    let meaning := statement.getArg! 4
    unless meaning.isAppOf ``propheticMeaning do throwError "not a function's meaning"
    unless ← isDefEq (meaning.getArg! 3) (target.getArg! 3) do
      throwError "the closure and its target theorem use different type families"
    let rule ← mkAppAtFrame ``VerexIR.Proofs.FramedAt.ofVerified
        (target.getArg! 0) (target.getArg! 2) (target.getArg! 3)
      #[weave, ← free (mkApp (mkConst ``NRow.refFree) captured), target.getArg! 6,
        closure.instantiation, coherent, closure.captures, closure.value, closure.closed,
        ← instantiateMVars (mkAppN theoremProof theoremArguments)]
    let rule := mkApp rule (target.getArg! 7)
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
        intro _ _ $(mkIdent `verexTyped):ident
        (try simp only [$lemmas,*, Weave.compose, vir_denote_norm])
        all_goals (try constructor)
        all_goals verex_leaf))) do
      throwError "the target's theorem does not apply everywhere"
    -- The target's frame lies within the closure's.
    let framed ← mkFreshExprSyntheticOpaqueMVar framedType
    unless ← closesBy framed.mvarId! (← `(tactic| (
        intro _ _ _ _ $(mkIdent `verexFrame):ident
        (try simp only [$lemmas,*, Weave.compose, vir_denote_norm]
          at $(mkIdent `verexFrame):ident ⊢)
        first
        | exact $(mkIdent `verexFrame)
        | verex_leaf))) do
      throwError "the target's frame exceeds the closure's"
    let proof ← instantiateMVars (mkApp2 rule held framed)
    unless ← isDefEq (← inferType proof) target do throwError "not the closure"
    goal.assign proof
    return some []
  catch error =>
    if verex.solverDebug.get (← getOptions) then
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
    let rule ← mkAppAtFrame ``VerexIR.Proofs.FramedAt.ofBody (target.getArg! 0) (target.getArg! 2) (target.getArg! 3)
      #[weave, ← free (mkApp (mkConst ``NRow.refFree) captured), target.getArg! 6,
        closure.instantiation, coherent, closure.captures, closure.value, closure.closed]
    let rule := mkApp rule (target.getArg! 7)
    let .forallE _ keptType _ _ ← whnfR (← inferType rule) | throwError "no body premise"
    let kept ← mkFreshExprSyntheticOpaqueMVar keptType
    let proof ← instantiateMVars (mkApp rule kept)
    unless ← isDefEq (← inferType proof) target do throwError "not the closure"
    goal.assign proof
    let (_, kept) ← kept.mvarId!.introN 3 [`verexArguments, `verexStart, `verexTyped]
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
def contextFacts (goal : MVarId) : MetaM (Array Lean.Expr) := goal.withContext do
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
    -- The invocation's row, shape, and arguments, as the predicate states them.
    let rule ← mkConstWithFreshMVarLevels ``VerexIR.Proofs.returns_of_terminating
    let ruleType ← inferType rule
    let (arguments, _, _) ← forallMetaTelescope ruleType
    let names := ruleType.getForallBinderNames
    -- The unit, its function values and the frame, as the predicate states them.
    for argument in arguments do
      let type ← whnfR (← inferType argument)
      if type.isAppOf ``Skolems then
        unless ← isDefEq argument (aborts.getArg! 2) do throwError "frame does not fit"
      else if type.isAppOf ``FunctionCarriers then
        unless ← isDefEq argument (aborts.getArg! 1) do throwError "function values do not fit"
    for (name, value) in [(`σs, aborts.getArg! 4), (`shared, aborts.getArg! 5),
        (`shape, aborts.getArg! 6), (`function, aborts.getArg! 7), (`args, aborts.getArg! 8)] do
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
      else if type.isAppOfArity ``VerexIR.Proofs.MemoryTyped 3 then
        unless ← proveMemoryTyped facts id do throwError "memory is not typed"
      else if type.isAppOfArity ``VerexIR.Proofs.ScalarAt 4 then
        unless ← scalarAt? facts id do throwError "a row is not scalar at the frame"
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
      if e == stated.getArg! 7 then some (aborts.getArg! 7)
      else if e == stated.getArg! 8 then some (aborts.getArg! 8) else none
    mkExpectedTypeHint proof ((← instantiateMVars (← inferType proof)).replace spelled)
  let (_, next) ← goal.withContext do
    (← goal.assert `verexReturns (← inferType proof) proof).intro1P
  setGoals [next]
  evalTactic (← `(tactic| obtain ⟨_, _, $(mkIdent `verexEnsures), $(mkIdent `verexResult),
    $(mkIdent `verexStateOf)⟩ :=
    $(mkIdent `verexReturns)))
  evalTactic (← `(tactic| try verex_normalize at $(mkIdent `verexEnsures):ident))
  evalTactic (← `(tactic| try verex_normalize at $(mkIdent `verexResult):ident))
  let [obtained] ← getGoals | throwError "the run leaves one goal"
  obtained.withContext do
    let some result := (← getLCtx).findFromUserName? `verexResult
      | throwError "no result equation"
    let mut current := obtained
    for decl in ← getLCtx do
      if decl.isImplementationDetail || decl.fvarId == result.fvarId then continue
      let type ← instantiateMVars decl.type
      unless (type.find? (·.isAppOf ``VerexIR.Proofs.ResultOf)).isSome do continue
      try
        current ← current.withContext do
          let rewritten ← current.rewrite type result.toExpr
          pure (← current.replaceLocalDecl decl.fvarId rewritten.eNew rewritten.eqProof).mvarId
      catch _ => pure ()
    return current

/-- The `result_of` reads of a term: those of function values a closure
literal builds, outside binders. -/
private def resultOfReads (environment : Environment) (terms : Array Lean.Expr) :
    Array Lean.Expr :=
  sitesWhere (fun e =>
    (e.isAppOfArity ``VerexIR.Proofs.ResultOf 11 ||
      e.isAppOfArity ``VerexIR.Proofs.StateOf 10) && !e.hasLooseBVars &&
      (literalFunction? environment (e.getArg! 7)).isSome) terms

/-- The `aborts_of` of the invocation a `result_of` or `state_of` reads. -/
private def readAborts (read : Lean.Expr) : Lean.Expr :=
  mkAppN (mkConst ``VerexIR.Proofs.AbortsOf) (read.getAppArgs.extract 0 10)

/-- A leaf that reads `result_of` of a literal closure where it knows
neither that the invocation aborts nor that it does not, case by case on
the abort: where it aborts, the target's body or theorem decides the leaf;
where it does not, the run `result_of` reads (`terminatingRuns`), as the
Move Prover reads `result_of` under `!aborts_of`. -/
def abortCases? (goal : MVarId) : TacticM (Option (List MVarId)) := do
  let some aborts ← goal.withContext do
      let facts ← contextFacts goal
      unless ← facts.anyM (fun fact => do
          return (← instantiateMVars (← inferType fact)).isAppOfArity
            ``VerexIR.Proofs.Terminating 3) do
        return none
      let target ← instantiateMVars (← goal.getType)
      let mut statements := #[target]
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let type ← instantiateMVars decl.type
        statements := statements.push type
      for read in resultOfReads (← getEnv) statements do
        let aborts := readAborts read
        if statements.any (fun statement => (statement.find? (· == aborts)).isSome) then continue
        -- A known abort decision needs no fresh case split; definitional
        -- equality supplies the match.
        let known ← facts.anyM fun fact => do
          let type := unmarked (← instantiateMVars (← inferType fact))
          let predicate := type.not?.getD type
          unless predicate.isAppOfArity ``VerexIR.Proofs.AbortsOf 10 do return false
          withReducible (isDefEq predicate aborts)
        if known then continue
        return some aborts
      return none
    | return none
  let (aborted, unaborted) ← goal.byCases aborts `verexAborted
  return some [aborted.mvarId, unaborted.mvarId]

/-- A named call already supplies the chosen result and final state, together
with its postcondition. Reconstructing that same invocation introduces fresh
copies only to equate them to these values again. Match all three facts before
skipping that reconstruction; partial information still takes the ordinary path. -/
private def hasKnownInvocation (aborts : Lean.Expr) : MetaM Bool := do
  let state := mkAppN (mkConst ``VerexIR.Proofs.StateOf) aborts.getAppArgs
  let mut results := #[]
  let mut states := #[]
  let mut runs := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type ← instantiateMVars decl.type
    -- A named call's run, marked read where the call took its contract.
    let type := if type.isAppOfArity ``VerexIR.Proofs.Denoted 1 then type.appArg! else type
    if type.isAppOfArity ``VerexIR.Proofs.EnsuresOf 12 then
      runs := runs.push type
    -- A result is stated at every default it is read at.
    let mut equation := type
    while equation.isForall do equation := equation.bindingBody!
    if let some (_, lhs, rhs) := equation.eq? then
      if lhs.isAppOfArity ``VerexIR.Proofs.ResultOf 11 && !(readAborts lhs).hasLooseBVars &&
          !rhs.hasLooseBVars then
        if ← withReducible (isDefEq (readAborts lhs) aborts) then results := results.push rhs
      if lhs.isAppOfArity ``VerexIR.Proofs.StateOf 10 then
        if ← withReducible (isDefEq lhs state) then states := states.push rhs
  if results.isEmpty || states.isEmpty then return false
  for run in runs do
    -- The run's invocation from its pre-state, as `aborts_of` states it.
    let actual := mkAppN (mkConst ``VerexIR.Proofs.AbortsOf)
      (run.getAppArgs.extract 0 9 |>.push (run.getArg! 10))
    unless ← withReducible (isDefEq actual aborts) do continue
    unless ← results.anyM (fun value => withReducible (isDefEq value (run.getArg! 9))) do continue
    if ← states.anyM (fun value => withReducible (isDefEq value (run.getArg! 11))) then
      return true
  return false

/-- Where the theorem holds that the unit's runs end (`Terminating`), each
literal closure's invocation the leaf knows not to abort returns: a run
`ensures_of` names, and the result `result_of` reads, which replaces it in
the hypotheses (`returns_of_terminating`). One invocation a round, so that
an invocation at another's result is met once that result is read. -/
def terminatingRuns (goal : MVarId) : TacticM MVarId := do
  let saved ← getGoals
  let terminating ← goal.withContext do (← contextFacts goal).anyM fun fact => do
    return (← instantiateMVars (← inferType fact)).isAppOfArity ``VerexIR.Proofs.Terminating 3
  unless terminating do return goal
  let mut current := goal
  let mut handled : Array Lean.Expr := #[]
  let debug := verex.solverDebug.get (← getOptions)
  for _ in [0:16] do
    let facts ← contextFacts current
    let candidates ← current.withContext do
      -- The results the runs found so far, read where an invocation's
      -- arguments name them.
      let mut results := #[]
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let type ← instantiateMVars decl.type
        if type.isAppOfArity ``Eq 3 && (type.getArg! 1).isAppOf ``VerexIR.Proofs.ResultOf then
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
          unless aborts.isAppOfArity ``VerexIR.Proofs.AbortsOf 10 &&
              (literalFunction? (← getEnv) (aborts.getArg! 7)).isSome do continue
          unless handled.contains aborts || found.any (·.2 == aborts) do
            unless ← hasKnownInvocation aborts do
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

/-! ## Invocations over a model's meaning

In a unit verified through its model (`doc/designs/ghost-fields.md`) an
invocation denotes the theorem's invocation meaning at the frame. At the
base frame, a closure site's constructor of a routed member is rewritten to
the member's meaning on the woven arguments by the theorem's routing
hypothesis of the site (`invokeRoutes_…`); any other invocation is unseen,
defined where the
target's declared precondition holds (`verexInvokeDefined`), its runs and
aborts stated by the behavioral predicates and the frames stated of the
value (`wp_invoke_unseen`). -/

/-- The failure condition permitting every failure. -/
private def trueFailures (ε : Lean.Expr) : MetaM Lean.Expr :=
  withLocalDeclD `error ε fun error => mkLambdaFVars #[error] (mkConst ``True)

/-- The theorem's routing hypotheses, as the lemmas a rewrite uses. -/
private def routingLemmas : MetaM (Array (TSyntax ``Lean.Parser.Tactic.simpLemma)) := do
  (← getLCtx).foldlM (init := #[]) fun found decl => do
    unless decl.userName.toString.startsWith "invokeRoutes_" do return found
    return found.push (← `(Lean.Parser.Tactic.simpLemma| $(mkIdent decl.userName):ident))

/-- An invocation over a model's meaning (`invoke Θ σs shared shape
function args`). -/
private structure ModelInvocation where
  invoke : Lean.Expr
  supplied : Lean.Expr
  shared : Lean.Expr
  shape : Lean.Expr
  function : Lean.Expr
  arguments : Lean.Expr

/-- An invocation over a model's meaning: the meaning at the frame, the
supplied row, its shared flags, the result shape, the function value, and
the arguments. -/
private def modelInvocation? (action : Lean.Expr) : MetaM (Option ModelInvocation) := do
  let action := action.consumeMData
  unless action.getAppFn.isFVar && action.getAppNumArgs == 6 do return none
  let invoke := mkApp action.getAppFn (action.getArg! 0)
  unless (← instantiateMVars (← inferType invoke)).isAppOf ``InvokeMeaning do return none
  return some {
    invoke := invoke
    supplied := action.getArg! 1
    shared := action.getArg! 2
    shape := action.getArg! 3
    function := action.getArg! 4
    arguments := action.getArg! 5 }

/-- An invocation of a literal closure over a model's meaning, rewritten to
its routed target's meaning. -/
def routedInvocation? (goal : MVarId) : TacticM (Option (List MVarId)) := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
  let some invocation ← goal.withContext (modelInvocation? (target.getArg! 3))
    | return none
  unless ← isKnownFunction invocation.function do return none
  let lemmas ← goal.withContext routingLemmas
  if lemmas.isEmpty then return none
  let saved ← saveState
  try
    setGoals [goal]
    evalTactic (← `(tactic| simp only [$lemmas,*, VerexIR.Proofs.Denote.Weave.compose]))
    let [next] ← getGoals | throwError "the routing left several goals"
    let rewritten ← next.withContext do instantiateMVars (← next.getType)
    if rewritten == target then throwError "no routing hypothesis matches"
    return some [next]
  catch error =>
    if verex.solverDebug.get (← getOptions) then
      IO.println s!"routed invocation skipped: {← error.toMessageData.toString}"
    saved.restore
    return none

/-- A run of an unseen invocation over a model's meaning keeps the frames
the hypotheses state of the value (`FramedAtOf`, `KeepsMemoryAtOf`): of a
value keeping memory, the run's final memory is substituted by its initial
one; of another, its frame at the run's arguments is a hypothesis. -/
private def keptMemoryOf (goal : MVarId) (invocation : ModelInvocation) (initial : Lean.Expr)
    (result final run : Lean.Expr) (facts : Array Lean.Expr) : TacticM MVarId := do
  let mut goal := goal
  for fact in facts do
    let factType ← goal.withContext do frameFactType (← inferType fact)
    let keeps := factType.isAppOfArity ``VerexIR.Proofs.KeepsMemoryAtOf 8
    unless keeps || factType.isAppOfArity ``VerexIR.Proofs.FramedAtOf 9 do continue
    let (row, framed) := if keeps then (factType.getArg! 4, factType.getArg! 7)
      else (factType.getArg! 4, factType.getArg! 8)
    let kept? ← goal.withContext do
      unless ← isDefEq framed invocation.function do return none
      unless ← isDefEq row invocation.supplied do return none
      unless ← isDefEq (factType.getArg! 3) invocation.invoke do return none
      let typed ← mkFreshExprMVar (← mkAppM ``VerexIR.Proofs.MemoryTyped
        #[factType.getArg! 0, initial])
      unless ← proveMemoryTyped facts typed.mvarId! do return none
      let kept := mkAppN (← frameFactProof fact)
        #[invocation.arguments, result, initial, final, ← instantiateMVars typed, run]
      try check kept catch error =>
        if verex.solverDebug.get (← getOptions) then
          logInfo m!"model frame application: {error.toMessageData}"
        return none
      return some kept
    let some kept := kept? | continue
    let (equation, next) ← goal.withContext do
      (← goal.assert `verexKept (← whnfR (← inferType kept)).headBeta kept).intro1P
    -- Substituting the final memory ends the run's facts: the other frames
    -- would speak of a memory no longer named.
    if keeps then return ← next.withContext (Lean.Meta.subst next equation)
    goal := next
  return goal

/-- An invocation of a function value the proof does not see, over a model's
meaning (`wp_invoke_unseen`). The target's declared precondition, which
defines the invocation, is decided where every target of the unit's
closures declares none; otherwise it stays an obligation. -/
def unseenModelInvocation? (goal : MVarId) : TacticM (Option (List MVarId)) := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
  let some invocation ← goal.withContext (modelInvocation? (target.getArg! 3))
    | return none
  if ← isKnownFunction invocation.function then return none
  let original ← saveState
  -- A closure read from a resource gets its frame from that value's stored
  -- invariant (`unseenInvocation?`).
  let goal ← if !(← readsStoredInvariants goal) then pure goal else do
    let saved ← saveState
    try
      setGoals [goal]
      evalTactic (← `(tactic| verex_split_hypotheses))
      evalTactic (← `(tactic| verex_stored_reads))
      evalTactic (← `(tactic| simp_all only [Which.project?_eq_some_iff,
        Which.inject_here, Which.inject_there, vir_denote_norm]))
      for _ in [0:2] do
        evalTactic (← `(tactic| verex_split_hypotheses))
        evalTactic (← `(tactic| verex_subst_vars))
        evalTactic (← `(tactic| verex_reduce_projections))
        evalTactic (← `(tactic| verex_normalize_hypotheses))
      getMainGoal
    catch error =>
      if verex.solverDebug.get (← getOptions) then
        IO.println s!"model stored closure frame: {← error.toMessageData.toString}"
      saved.restore
      pure goal
  let stage ← IO.mkRef "start"
  let debug := verex.solverDebug.get (← getOptions)
  let result ← tryCatchRuntimeEx (goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let some invocation ← modelInvocation? (target.getArg! 3) | return none
    let initial := target.getArg! 6
    if debug then IO.println s!"model unseen invocation of {← ppExpr invocation.function}"
    let some definedHypothesis := (← getLCtx).findFromUserName? `verexInvokeDefined | return none
    let definedAt := mkAppN definedHypothesis.toExpr
      #[invocation.invoke.appArg!, invocation.supplied, invocation.shared, invocation.shape,
        invocation.function, invocation.arguments, initial]
    let .forallE _ requiresType _ _ ← whnfR (← inferType definedAt) | return none
    let requires ← mkFreshExprSyntheticOpaqueMVar requiresType
    stage.set "rule"
    -- The meaning's type as spelled: `InvokeMeaning` is an abbreviation.
    let meaningType ← instantiateMVars (← inferType invocation.invoke)
    unless meaningType.isAppOfArity ``InvokeMeaning 3 do return none
    let rule ← mkAppOptM' (mkApp3 (mkConst ``VerexIR.Proofs.wp_invoke_unseen)
        (meaningType.getArg! 0) (meaningType.getArg! 1) (meaningType.getArg! 2))
      #[some invocation.invoke, some invocation.supplied, some invocation.shared,
        some invocation.shape, some invocation.function, some invocation.arguments, some initial,
        some (target.getArg! 4), some (target.getArg! 5), some (mkApp definedAt requires)]
    let .forallE _ returnsType rest _ ← whnfR (← inferType rule) | return none
    let returns ← mkFreshExprSyntheticOpaqueMVar returnsType (← goal.getTag)
    let .forallE _ failsType _ _ ← whnfR (rest.instantiate1 returns) | return none
    let fails ← mkFreshExprSyntheticOpaqueMVar failsType (← goal.getTag)
    let rule := mkApp2 rule returns fails
    unless ← isDefEq (← inferType rule) target do return none
    goal.assign rule
    let (returns, fails) := (returns.mvarId!, fails.mvarId!)
    stage.set "facts"
    let mut facts := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      if ← isProp declaration.type then facts := facts ++ (← conjuncts declaration.toExpr)
    stage.set "frames"
    let (introduced, returns) ← returns.introN 3 [`verexResult, `verexFinal, `verexEnsures]
    let returns ← keptMemoryOf returns invocation initial
      (.fvar introduced[0]!) (.fvar introduced[1]!) (.fvar introduced[2]!) facts
    let (_, fails) ← fails.introN 3 [`verexError, `verexAborted, `verexAborts]
    -- The precondition declared of the value: a hypothesis the contract
    -- states, or, of a known value, its target's.
    stage.set "precondition"
    let requiresGoal := requires.mvarId!
    -- A unit none of whose closure sites' targets declares a precondition
    -- defines every invocation (the generated `functionRequires_everywhere`).
    let environment ← getEnv
    let everywhere? : Option Name := do
      guard (requiresType.isAppOfArity ``VerexIR.Proofs.RequiresOf 10)
      let name := (← (requiresType.getArg! 3).getAppFn.constName?).appendAfter "_everywhere"
      guard (environment.contains name)
      pure name
    let decided ← match everywhere? with
      | some name => closesBy requiresGoal (← `(tactic| exact $(mkCIdent name) _ _ _ _ _ _))
      | none => pure false
    let decided ← if decided then pure true else closesBy requiresGoal (← `(tactic| first
      | assumption
      | (verex_behavior; done)))
    if debug then
      IO.println s!"model unseen invocation applied; precondition decided: {decided}"
    return some ([returns, fails] ++ if decided then [] else [requiresGoal]))
    fun error => do
      if debug then
        IO.println s!"model unseen invocation failed at {← stage.get}: {← error.toMessageData.toString}"
      pure none
  if result.isNone then original.restore
  return result

/-- A routing hypothesis at a function value: its binders but the
invocation's arguments instantiated so that the value it routes is the
given one, as `∀ args, invoke … value args = self …`. -/
private def routesOf? (decl : Lean.LocalDecl) (value : Lean.Expr) : MetaM Lean.Expr := do
  let (binders, _, statement) ← forallMetaTelescope (← instantiateMVars decl.type)
  let some (_, lhs, _) := statement.eq? | throwError "a routing hypothesis is no equation"
  let some argsBinder := binders.back? | throwError "a routing hypothesis takes no arguments"
  unless lhs.getAppNumArgs ≥ 2 && (← isDefEq (lhs.getArg! (lhs.getAppNumArgs - 2)) value) do
    throwError "another function value"
  withLocalDeclD `args (← instantiateMVars (← inferType argsBinder)) fun args => do
    argsBinder.mvarId!.assign args
    let routes ← instantiateMVars (mkAppN decl.toExpr binders)
    if routes.hasExprMVar then throwError "the routing leaves binders open"
    mkLambdaFVars #[args] routes

/-- A frame of a literal closure over a model's meaning (`FramedAtOf`,
`KeepsMemoryAtOf`): by the routed target's meaning on the woven arguments,
an obligation over every argument and typed memory
(`FramedAtOf.ofRouted`); or a frame a hypothesis states of the same value,
within it (`FramedAtOf.mono`). The goals left. -/
def routedClosureFrame? (goal : MVarId) : TacticM (Option (List MVarId)) := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let target := if target.isAppOfArity ``VerexIR.Proofs.Obligation 4 then target.appArg! else target
  let target ← if target.isAppOfArity ``VerexIR.Proofs.KeepsMemoryAtOf 8 then
      goal.withContext (whnfR target) else pure target
  unless target.isAppOfArity ``VerexIR.Proofs.FramedAtOf 9 do return none
  let closureValue := target.getArg! 8
  -- A frame a hypothesis states of the same function value, within it.
  let stated ← try goal.withContext do
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some (_, closure) ← framedFact? decl.type | continue
      unless ← isDefEq closure closureValue do continue
      let narrow ← whnfR (← frameFactType decl.type)
      unless narrow.isAppOfArity ``VerexIR.Proofs.FramedAtOf 9 do continue
      let .forallE _ argumentsType _ _ ← whnfR (← inferType (target.getArg! 7)) | continue
      let memoryType :=
        mkApp2 (mkConst ``VerexIR.Proofs.Denote.Memory) (target.getArg! 0) (target.getArg! 1)
      let withinType ← withLocalDeclD `verexArguments argumentsType fun args =>
        withLocalDeclD `pre memoryType fun pre =>
          withLocalDeclD `post memoryType fun post => do
            mkForallFVars #[args, pre, post] (← mkArrow
              (mkApp3 (narrow.getArg! 7) args pre post).headBeta
              (mkApp3 (target.getArg! 7) args pre post).headBeta)
      let within ← mkFreshExprSyntheticOpaqueMVar withinType
      unless ← closesBy within.mvarId! (← `(tactic| (
          intro _ _ _ $(mkIdent `verexFrame):ident
          first
          | exact $(mkIdent `verexFrame)
          | verex_bounded
              (simp only [vir_denote_norm, vir_denote_eval] at $(mkIdent `verexFrame):ident ⊢
               grind)))) do continue
      return some (← mkAppOptM' (mkApp3 (mkConst ``VerexIR.Proofs.FramedAtOf.mono)
          (target.getArg! 0) (target.getArg! 1) (target.getArg! 2))
        #[none, none, none, none, none, none, none,
          some (← mkExpectedTypeHint (← frameFactProof decl.toExpr) narrow),
          some (← instantiateMVars within)])
    return none
    catch _ => pure none
  if let some proof := stated then
    goal.assign proof
    return some []
  let some closure ← goal.withContext (literalValue? closureValue) | return none
  let mask := closure.mask
  unless mask.isAppOfArity ``Weave.mask 4 do return none
  let weave := mask.appArg!
  let state ← saveState
  try
    goal.withContext do
    -- The routing hypothesis of the site at the value's captures, leaving
    -- the invocation's arguments quantified.
    let mut routed? := none
    for decl in ← getLCtx do
      unless decl.userName.toString.startsWith "invokeRoutes_" do continue
      let attempt ← saveState
      try
        let routes ← routesOf? decl closure.value
        let rule ← mkAppOptM' (mkApp3 (mkConst ``VerexIR.Proofs.FramedAtOf.ofRouted)
            (target.getArg! 0) (target.getArg! 1) (target.getArg! 2))
          #[some (target.getArg! 3), none, none, none, some (target.getArg! 6), none, some weave,
            some closure.captures, some (target.getArg! 5), some closure.value, some routes,
            some (target.getArg! 7)]
        let .forallE _ keptType _ _ ← whnfR (← inferType rule)
          | throwError "the frame rule leaves no obligation"
        let kept ← mkFreshExprSyntheticOpaqueMVar keptType (← goal.getTag)
        let rule := mkApp rule kept
        unless ← isDefEq (← inferType rule) target do
          throwError m!"another target: {← inferType rule} vs {target}"
        routed? := some (← instantiateMVars rule)
        break
      catch error =>
        if verex.solverDebug.get (← getOptions) then
          IO.println s!"routing attempt {decl.userName}: {← error.toMessageData.toString}"
        attempt.restore
    let some rule := routed? | throwError "no routing hypothesis for the closure's target"
    let .mvar kept := rule.appArg! | throwError "the frame's obligation is not open"
    goal.assign rule
    let (_, kept) ← kept.introN 3 [`verexArguments, `verexStart, `verexTyped]
    setGoals [kept]
    evalTactic (← `(tactic| try simp only [VerexIR.Proofs.Denote.Weave.compose]))
    return some (← getGoals)
  catch error =>
    if verex.solverDebug.get (← getOptions) then
      IO.println s!"routed closure frame: {← error.toMessageData.toString}"
    state.restore
    return none

/-- A leaf's run or abort of a literal closure over a model's meaning
(`EnsuresOfAt`, `AbortsOfAt`), as a hypothesis states it: a run of its
invocation (`ensuresOfAt_closureOf`), routed to its target's meaning. Of a
target used through its contract, what the contract states of the run
(`runOfSatisfies`) or abort (`abortOfSatisfies`) is a hypothesis; of an
inlined target the leaf becomes the weakest precondition of its body under
which it holds of each run or abort (`forall_ok_of_wp`, `of_aborts_of_wp`),
which the call rule inlines. The hypothesis stays, marked read. -/
private def modelDenotedRun (goal : MVarId) (callees : Array (Lean.Expr × String × Lean.Expr))
    (hypothesis : FVarId) (stated : Lean.Expr) (view : BehaviorView) : TacticM (Option MVarId) := do
  unless (literalFunction? (← getEnv) view.function).isSome do return none
  -- The run or abort of the closure's invocation, as the predicate defines it.
  let action := mkAppN view.invoke
    #[view.supplied, view.shared, view.shape, view.function, view.arguments]
  let fact ← match view.run? with
    | some run =>
        mkExpectedTypeHint stated (← mkAppM ``VerexIR.Proofs.Spec.ok
          #[action, view.state, run.result, run.post])
    | none =>
        let failure := mkConst ``VerexIR.Proofs.Failure
        let aborted ← withLocalDeclD `failure failure fun error => do
          mkLambdaFVars #[error] (← mkAppM ``VerexIR.Proofs.Spec.aborts #[action, view.state, error])
        mkExpectedTypeHint stated (← mkAppM ``Exists #[aborted])
  let goal ← goal.replaceLocalDeclDefEq hypothesis
    (mkApp (mkConst ``VerexIR.Proofs.Denoted) (← instantiateMVars (← hypothesis.getType)))
  let (runVar, goal) ← goal.withContext do
    (← goal.assert `verexRun (← inferType fact) fact).intro1P
  -- The invocation routed to the target's meaning on the woven arguments.
  let lemmas ← goal.withContext routingLemmas
  setGoals [goal]
  evalTactic (← `(tactic| simp only [$lemmas,*, VerexIR.Proofs.Denote.Weave.compose]
    at $(mkIdent `verexRun):ident))
  let goal ← getMainGoal
  goal.withContext do
  let runVar := (← getLCtx).findFromUserName? `verexRun |>.map (·.fvarId) |>.getD runVar
  let runType ← instantiateMVars (← runVar.getType)
  -- The run `(self args).ok pre result post`, or the abort
  -- `∃ failure, (self args).aborts pre failure`.
  let action ← match view.run? with
    | some _ =>
        unless runType.isAppOfArity ``VerexIR.Proofs.Spec.ok 7 do throwError m!"not a run: {runType}"
        pure (runType.getArg! 3)
    | none =>
        let .app _ (.lam _ _ aborts _) := runType | throwError "the abort is not an existential"
        unless aborts.isAppOfArity ``VerexIR.Proofs.Spec.aborts 6 do throwError "not an abort"
        pure (aborts.getArg! 3)
  let head := action.getAppFn
  unless head.isFVar do throwError m!"the invocation is not routed: {action}"
  let some (_, _, theoremProof) ← callees.findM? fun (candidate, _, _) => isDefEq candidate head
    | throwError "no theorem for the routed target"
  let statement ← forallTelescope (← whnfR (← zetaReduce (← inferType theoremProof)))
    fun _ statement => pure statement
  if statement.isAppOfArity ``VerexIR.Proofs.Satisfies 6 then
    -- What the target's contract states of the run or abort, where what its
    -- theorem assumes and its precondition hold.
    let (theoremArguments, _, _) ← forallMetaTelescope (← inferType theoremProof)
    let verified := mkAppN theoremProof theoremArguments
    let implication ← if view.run?.isSome then
        mkAppOptM ``VerexIR.Proofs.runOfSatisfies
          #[none, none, none, none, none, none, some verified, none, none, none, none,
            some (.fvar runVar)]
      else
        mkAppOptM ``VerexIR.Proofs.abortOfSatisfies
          #[none, none, none, none, none, none, some verified, none, none, some (.fvar runVar)]
    let implication ← instantiateMVars implication
    let constants ← contractConstants (← inferType theoremProof)
    let lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ← constants.mapM fun name =>
      `(Lean.Parser.Tactic.simpLemma| $(mkIdent (rootNamespace ++ name)):ident)
    let mut fact := implication
    for _ in [0:2] do
      let .forallE _ premiseType _ _ ← whnfR (← inferType fact)
        | throwError "the contract's premises are not open"
      let premise ← mkFreshExprSyntheticOpaqueMVar premiseType
      unless ← closesBy premise.mvarId! (← `(tactic|
          ((try simp only [$lemmas,*, VerexIR.Proofs.Denote.Weave.compose, vir_denote_norm]) <;>
            verex_leaf))) do
        throwError "the target's contract premises are not established"
      fact := mkApp fact (← instantiateMVars premise)
    for argument in theoremArguments do
      if let .mvar id ← instantiateMVars argument then
        unless ← id.isAssigned do id.assumption
    let (_, next) ← (← goal.assert `verexBehavior (← inferType fact) fact).intro1P
    let state ← saveState
    try
      setGoals [next]
      evalTactic (← `(tactic| simp only [$lemmas,*, VerexIR.Proofs.Denote.Weave.compose,
        vir_denote_norm, not_false_eq_true, true_implies, not_or] at $(mkIdent `verexBehavior):ident))
      return some (← getMainGoal)
    catch _ =>
      state.restore
      return some next
  unless statement.isAppOf ``VerexIR.Proofs.Spec.Refines do
    throwError "the routed target has neither a contract nor a body"
  -- The inlined target: its body's run or abort.
  let (binders, _, refined) ← forallMetaTelescope (← whnfR (← zetaReduce (← inferType theoremProof)))
  let .app (.app _ left) body ← whnfR refined
    | throwError "an inlined target's refinement does not name its body"
  unless ← isDefEq left action do throwError "an inlined target's refinement does not meet its run"
  let refinement ← instantiateMVars (mkAppN theoremProof binders)
  let body ← instantiateMVars body
  let unfoldNames := match body.getAppFn.constName?, body.getAppArgs[5]? with
    | some ``VerexIR.Proofs.Denote.Function.denoteWith, some (.const name _) =>
        [name, name.getPrefix ++ `body, name.getPrefix ++ `mutables, name.getPrefix ++ `params,
          name.getPrefix ++ `locals, name.getPrefix ++ `shape, name.getPrefix ++ `row]
    | _, _ => []
  let unfolds : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ← unfoldNames.toArray.mapM
    fun name => `(Lean.Parser.Tactic.simpLemma| $(mkIdent (rootNamespace ++ name)):ident)
  match view.run? with
  | some _ =>
      -- The leaf, of every run of the body returning the stated result and
      -- final memory: the weakest precondition of the body under which a
      -- run's result and memory being the stated ones gives the leaf.
      let bodyRun ← mkAppM ``VerexIR.Proofs.okOfRefines #[refinement, .fvar runVar]
      let bodyRunType ← instantiateMVars (← inferType bodyRun)
      unless bodyRunType.isAppOfArity ``VerexIR.Proofs.Spec.ok 7 do throwError "not a body run"
      let (σ, ε, α) := (bodyRunType.getArg! 0, bodyRunType.getArg! 1, bodyRunType.getArg! 2)
      let (body, initial) := (bodyRunType.getArg! 3, bodyRunType.getArg! 4)
      let (result, final) := (bodyRunType.getArg! 5, bodyRunType.getArg! 6)
      let leaf ← goal.getType
      let ensures ← withLocalDeclD `result α fun r => withLocalDeclD `final σ fun f => do
        mkLambdaFVars #[r, f] (← mkArrow (← mkEq r result) (← mkArrow (← mkEq f final) leaf))
      let established ← mkFreshExprSyntheticOpaqueMVar
        (mkAppN (mkConst ``VerexIR.Proofs.wp) #[σ, ε, α, body, ensures, ← trueFailures ε, initial])
        (← goal.getTag)
      goal.assign (mkAppN (mkConst ``VerexIR.Proofs.forall_ok_of_wp)
        #[σ, ε, α, body, ensures, initial, established, result, final, bodyRun,
          ← mkEqRefl result, ← mkEqRefl final])
      let next := established.mvarId!
      setGoals [next]
      evalTactic (← `(tactic| try simp only [$unfolds,*, VerexIR.Proofs.Denote.Function.denoteWith,
        VerexIR.Proofs.Denote.NRow.nil_append, VerexIR.Proofs.Denote.NRow.cons_append]))
      return some (← getMainGoal)
  | none =>
      let bodyAborts ← mkAppM ``VerexIR.Proofs.abortsOfRefines #[refinement, .fvar runVar]
      let abortsType ← instantiateMVars (← inferType bodyAborts)
      let .app _ (.lam _ _ aborts _) := abortsType | throwError "the abort is not an existential"
      let (σ, ε, α) := (aborts.getArg! 0, aborts.getArg! 1, aborts.getArg! 2)
      let (bodyAction, initial) := (aborts.getArg! 3, aborts.getArg! 4)
      let leaf ← goal.getType
      let returns ← withLocalDeclD `result α fun result => withLocalDeclD `final σ fun final =>
        mkLambdaFVars #[result, final] (mkConst ``True)
      let fails ← withLocalDeclD `error ε fun error => mkLambdaFVars #[error] leaf
      let established ← mkFreshExprSyntheticOpaqueMVar
        (mkAppN (mkConst ``VerexIR.Proofs.wp) #[σ, ε, α, bodyAction, returns, fails, initial])
      goal.assign (mkAppN (mkConst ``VerexIR.Proofs.of_aborts_of_wp)
        #[σ, ε, α, bodyAction, initial, leaf, established, bodyAborts])
      setGoals [established.mvarId!]
      evalTactic (← `(tactic| try simp only [$unfolds,*, VerexIR.Proofs.Denote.Function.denoteWith,
        VerexIR.Proofs.Denote.NRow.nil_append, VerexIR.Proofs.Denote.NRow.cons_append]))
      return some (← getMainGoal)

/-- The run or abort of a literal closure a hypothesis states, of a target
the proof inlines — compiled, for a run without a theorem, as a function
without a specification is: the target's prophetic meaning runs or aborts
(`ensuresOf_closureOf`, `abortsOf_closureOf`). The leaf becomes the
weakest precondition of that meaning under which it holds of each run or
abort (`forall_ok_of_wp`, `of_aborts_of_wp`), which the call rule inlines;
the hypothesis stays, marked read (`Denoted`). -/
private def denotedRun (goal : MVarId) (callees : Array (Lean.Expr × String × Lean.Expr))
    (hypothesis : FVarId) (stated type : Lean.Expr) : TacticM (Option MVarId) := do
  let some view := behaviorView? type | return none
  let some executable := executableOf? view.invoke
    | return ← modelDenotedRun goal callees hypothesis stated view
  let ensures := view.run?.isSome
  let some found := literalFunction? (← getEnv) view.function | return none
  let some closure ← literalValue? found | return none
  let some (weave, captured, _) ← closureWeave? closure | throwError "no weave"
  let handle := closure.handle
  -- The runs of a target with a theorem are read by its contract
  -- (`dispatchBehavior`); its aborts by its body, which states them where
  -- the contract need not, as the Move Prover derives them.
  let mut compiled? := none
  for (candidate, _, proof) in callees do
    unless ← isDefEq candidate handle do continue
    let statement ← forallTelescope (← inferType proof) fun _ statement => pure statement
    if ensures && statement.isAppOfArity ``VerexIR.Proofs.Satisfies 6 then
      throwError "a theorem"
    if statement.isAppOfArity ``Eq 3 && (statement.getArg! 2).isAppOfArity ``Except.ok 3 then
      compiled? := some (statement.getArg! 2).appArg!
  let some _ := compiled? | throwError "not compiled"
  let coherent ← mkAppM ``coherent_runtime #[type.getArg! 0, handle]
  let capturedFree ← decidedTrue (mkApp (mkConst ``NRow.refFree) captured)
  if let some run := view.run? then
    let unitShape := view.shape.isConstOf ``ResultShape.none
    let resultVar? ← if unitShape then pure none else
      let .fvar resultVar := run.result | throwError m!"result not a variable {run.result}"
      pure (some resultVar)
    let .fvar finalVar := run.post | throwError "final not a variable"
    let run ← mkAppAtFrame ``VerexIR.Proofs.ensuresOf_closureOf (type.getArg! 0) executable
      closure.frame #[weave, capturedFree, coherent, closure.captures, view.arguments, stated,
        closure.closed]
    let runType ← inferType run
    let goal ← goal.replaceLocalDeclDefEq hypothesis
      (mkApp (mkConst ``VerexIR.Proofs.Denoted) (← instantiateMVars (← hypothesis.getType)))
    let (runVar, goal) ← goal.withContext do (← goal.assert `verexRun runType run).intro1P
    -- What depends on the run's result and final state goes into the goal,
    -- the run first, so that the leaf is a fact about every run.
    let reverted := (resultVar?.map (#[·])).getD #[] |>.push finalVar
    let dependents ← goal.withContext do
      (← getLCtx).foldlM (init := #[]) fun found decl => do
        if decl.isImplementationDetail || decl.fvarId == runVar || reverted.contains decl.fvarId then
          return found
        let declType ← instantiateMVars decl.type
        if reverted.any declType.containsFVar then return found.push decl.fvarId
        return found
    let (_, goal) ← goal.revert dependents (preserveOrder := true)
    let (_, goal) ← goal.revert #[runVar]
    let (_, goal) ← goal.revert reverted (preserveOrder := true)
    let rule := if unitShape then ``VerexIR.Proofs.forall_ok_unit_of_wp
      else ``VerexIR.Proofs.forall_ok_of_wp
    let [next] ← goal.apply (← mkConstWithFreshMVarLevels rule) | return none
    return some next
  else
    let run ← mkAppAtFrame ``VerexIR.Proofs.abortsOf_closureOf (type.getArg! 0) executable
      closure.frame #[weave, capturedFree, coherent, closure.captures, view.arguments, stated,
        closure.closed]
    let goal ← goal.replaceLocalDeclDefEq hypothesis
      (mkApp (mkConst ``VerexIR.Proofs.Denoted) (← instantiateMVars (← hypothesis.getType)))
    goal.withContext do
      -- The abort's action and start, read off `∃ failure, action.aborts start failure`.
      let runType ← instantiateMVars (← inferType run)
      let .lam _ _ aborts _ := runType.appArg! | throwError "the abort is not an existential"
      unless aborts.isAppOfArity ``VerexIR.Proofs.Spec.aborts 6 do
        throwError "the abort is not of an action"
      let (σ, ε, α) := (aborts.getArg! 0, aborts.getArg! 1, aborts.getArg! 2)
      let (action, initial) := (aborts.getArg! 3, aborts.getArg! 4)
      let leaf ← goal.getType
      let returns ← withLocalDeclD `result α fun result => withLocalDeclD `final σ fun final =>
        mkLambdaFVars #[result, final] (mkConst ``True)
      let fails ← withLocalDeclD `error ε fun error => mkLambdaFVars #[error] leaf
      let established ← mkFreshExprSyntheticOpaqueMVar
        (mkAppN (mkConst ``VerexIR.Proofs.wp) #[σ, ε, α, action, returns, fails, initial])
      goal.assign (mkAppN (mkConst ``VerexIR.Proofs.of_aborts_of_wp)
        #[σ, ε, α, action, initial, leaf, established, run])
      return some established.mvarId!

/-- A leaf's first run or abort of a literal closure of an inlined target
(`denotedRun`), as a hypothesis states it under premises the leaf holds
and its clause's marker. -/
def denotedRun? (goal : MVarId) (callees : Array (Lean.Expr × String × Lean.Expr)) :
    TacticM (Option MVarId) := do
  let environment ← getEnv
  let candidates ← goal.withContext do
    -- The statement under its premises and marker, found without
    -- traversing the context's types: this runs at every conjunction.
    let literal (type : Lean.Expr) : Option Lean.Expr := Id.run do
      let mut type := type
      while type.isArrow do type := type.bindingBody!
      let statement := unmarked type
      let some view := behaviorView? statement | none
      if (literalFunction? environment view.function).isSome then some statement else none
    let stating ← (← getLCtx).foldlM (init := #[]) fun found decl => do
      if decl.isImplementationDetail then return found
      let type ← instantiateMVars decl.type
      if type.isAppOfArity ``VerexIR.Proofs.Denoted 1 then return found
      return if (literal type).isSome then found.push decl.fvarId else found
    if stating.isEmpty then return #[]
    let mut found := #[]
    for hypothesis in ← goal.getNondepPropHyps do
      unless stating.contains hypothesis do continue
      let mut type ← instantiateMVars (← hypothesis.getType)
      let mut stated := Lean.Expr.fvar hypothesis
      let mut held := true
      while type.isArrow do
        let some premise ← findLocalDeclWithType? type.bindingDomain! | held := false; break
        stated := mkApp stated (.fvar premise)
        type := type.bindingBody!
      unless held do continue
      if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
        stated := mkApp4 (mkConst ``Iff.mp) type type.appArg!
          (mkAppN (mkConst ``VerexIR.Proofs.Obligation_iff) type.getAppArgs) stated
        type := type.appArg!
      found := found.push (hypothesis, stated, type)
    pure found
  for (hypothesis, stated, type) in candidates do
    let state ← saveState
    try
      if let some next ← goal.withContext (denotedRun goal callees hypothesis stated type) then
        return some next
      state.restore
    catch ex =>
      if verex.solverDebug.get (← getOptions) then
        IO.println s!"denoted run skipped: {← ex.toMessageData.toString}"
      state.restore
  return none

/-- Rewrite a goal's reads of `result_of` by what its hypotheses state the
invocation returned: by `simp`, which matches the read up to the
abbreviations the typed spellings use and does not check the goal at
instances transparency, where a native row's pair is not well typed. None
where the rewriting closes the goal. -/
def rewriteResultOf (goal : MVarId) : TacticM (Option MVarId) := goal.withContext do
  let mut goal := goal
  -- The runs' results and states, at every default they are read at, all
  -- at once: a read inside another's arguments rewrites first, and the
  -- argument it leaves is the value a local names (`cutNamedValue`).
  let mut facts : Array (FVarId × Bool) := #[]
  for declaration in ← getLCtx do
    let mut type ← instantiateMVars declaration.type
    while type.isForall do type := type.bindingBody!
    unless type.isAppOfArity ``Eq 3 do continue
    if (type.getArg! 1).isAppOfArity ``VerexIR.Proofs.ResultOf 11 then
      facts := facts.push (declaration.fvarId, false)
    else if (type.getArg! 1).isAppOfArity ``VerexIR.Proofs.StateOf 10 then
      facts := facts.push (declaration.fvarId, true)
  if facts.isEmpty then return some goal
  let cut := mkIdent `VerexIR.Proofs.Denote.cutNamedValue
  try
    setGoals [goal]
    match ← simpAt goal (← `(tactic| simp only [$cut:ident])) (facts.map (·.1)) #[] true with
    | some next => goal := next
    | none => return none
  catch _ => pure ()
  -- A run's state, in the facts that read it.
  for (fact, isState) in facts do
    unless isState do continue
    let others ← goal.withContext do
      (← getLCtx).foldlM (init := #[]) fun found other => do
        if other.isImplementationDetail || other.fvarId == fact then return found
        unless (other.type.find? (·.isAppOf ``VerexIR.Proofs.StateOf)).isSome do return found
        return found.push other.fvarId
    for other in others do
      try
        goal ← goal.withContext do
          let rewritten ← goal.rewrite (← other.getType) (mkFVar fact)
          pure (← goal.replaceLocalDecl other rewritten.eNew rewritten.eqProof).mvarId
      catch _ => pure ()
  return some goal


elab_rules : tactic
  | `(tactic| verex_rewrite_results) => focusGoal do
    replaceMainGoal (← rewriteResultOf (← getMainGoal)).toList

/-- A named call's goal with the run its continuation assumes and the abort
its failure continuation assumes marked read (`Denoted`). -/
private def markNamedRead (goal : MVarId) : MetaM MVarId := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return goal
  let mark (premise : Lean.Expr) := mkApp (mkConst ``VerexIR.Proofs.Denoted) premise
  let args := target.getAppArgs
  let ensures := match args[4]! with
    | .lam r rt (.lam f ft (.forallE n run body bi) fbi) rbi =>
        Lean.Expr.lam r rt (.lam f ft (.forallE n (mark run) body bi) fbi) rbi
    | other => other
  let aborts := match args[5]! with
    | .lam e et (.forallE n abort body bi) ebi => Lean.Expr.lam e et (.forallE n (mark abort) body bi) ebi
    | other => other
  goal.replaceTargetDefEq (mkAppN target.getAppFn ((args.set! 4 ensures).set! 5 aborts))

/-- The first of what `pick` finds in expressions, also through the
continuations they reach, which the solver binds as local definitions
(`continuationName`), and the values of the local definitions they use. -/
private def findReached? {α : Type} (es : Array Lean.Expr) (continuations : Array Name)
    (pick : Lean.Expr → MetaM (Option α)) : MetaM (Option α) := do
  let mut pending := es
  let mut visited : Std.HashSet FVarId := {}
  let mut visitedContinuations : Std.HashSet Name := {}
  while let some next := pending.back? do
    pending := pending.pop
    if let some found ← pick next then return some found
    for name in next.getUsedConstants do
      if !continuations.contains name || visitedContinuations.contains name then continue
      visitedContinuations := visitedContinuations.insert name
      if let .defnInfo definition ← getConstInfo name then
        pending := pending.push definition.value
    for fvarId in (Lean.collectFVars {} next).fvarIds do
      if visited.contains fvarId then continue
      visited := visited.insert fvarId
      if let some value ← fvarId.getValue? then
        pending := pending.push (← instantiateMVars value)
  return none

/-- Whether an expression states a behavioral predicate, also through the
continuations it reaches (`findReached?`). -/
def statesBehavior (es : Array Lean.Expr) (continuations : Array Name := #[]) : MetaM Bool := do
  let behavioral (sub : Lean.Expr) : Bool :=
    sub.isConstOf ``VerexIR.Proofs.EnsuresOf || sub.isConstOf ``VerexIR.Proofs.AbortsOf ||
      sub.isConstOf ``VerexIR.Proofs.ResultOf || sub.isConstOf ``VerexIR.Proofs.StateOf
  return (← findReached? es continuations fun next =>
    pure (if (next.find? behavioral).isSome then some () else none)).isSome

/-- The statements a goal owes: its target, and the hypotheses the
traversal re-enters, those concluding in a `wp`, as a loop's back edge
(`wp_loopAt`). -/
private def owedStatements (target : Lean.Expr) : MetaM (Array Lean.Expr) := do
  (← getLCtx).foldlM (init := #[target]) fun found decl => do
    if decl.isImplementationDetail then return found
    let type ← instantiateMVars decl.type
    return if type.getForallBody.isAppOf ``VerexIR.Proofs.wp then found.push type else found

/-- Whether a goal owes a behavioral predicate (`owedStatements`). -/
private def obligesBehavior (target : Lean.Expr) (continuations : Array Name := #[]) : MetaM Bool := do
  statesBehavior (← owedStatements target) continuations

/-- That a function value the goal owes a behavioral predicate of names a
call's target: a known function value capturing nothing (`LiteralValue`) of
the target at the call's parameter row and instantiation, spelled as the
goal spells it, and whose closure is the closure naming the target. -/
private def namingLiteral? (target : Lean.Expr) (continuations : Array Name)
    (handle full instantiation : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let environment ← getEnv
  findReached? (← owedStatements target) continuations fun next => do
    for value in sitesWhere (isLiteralValue environment) #[next] do
      let literal? ← if value.hasLooseBVars then siteOfConstruct? value else literalValue? value
      let some literal := literal? | continue
      unless literal.captured.consumeMData.isConstOf ``NRow.nil do continue
      unless ← isDefEq literal.handle handle do continue
      unless ← isDefEq literal.parameters full do continue
      unless ← isDefEq literal.instantiation instantiation do continue
      return some literal.closed
    return none

/-- A call of a target taking and returning no reference, where the goal
owes a behavioral predicate (`obligesBehavior`): its continuations assume
`ensures_of` and `aborts_of` of the function value naming the target
(`wp_named`) and, where runs agree and the rows lend flatly, the absence of
its aborts and the result and state `result_of` and `state_of` read, where
its returned reference is settled (`wp_named_typed`). Other calls stay as
they are. A call through the target's contract states that
contract itself, so the run and the abort it names are marked read
(`Denoted`): deriving the contract from them again would add nothing. -/
def namedCall (goal : MVarId) (throughContract : Bool)
    (continuations : Array Name := #[]) : TacticM MVarId := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  unless ← goal.withContext (obligesBehavior target continuations) do return goal
  -- The rules at the call's executable and frame, which no instance search
  -- finds.
  unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return goal
  let meaning := target.getArg! 3
  unless meaning.isAppOfArity ``propheticMeaning 9 do return goal
  let (unit, functions, executable, frame) :=
    (meaning.getArg! 0, meaning.getArg! 1, meaning.getArg! 2, meaning.getArg! 3)
  let some naming ← goal.withContext (namingLiteral? target continuations (meaning.getArg! 5)
      (meaning.getArg! 6) (meaning.getArg! 4))
    | return goal
  let saved ← saveState
  let hypothesis? (constant : Name) : TacticM (Option Lean.Expr) := goal.withContext do
    (← getLCtx).findDeclM? fun declaration => do
      if declaration.isImplementationDetail then return none
      let type ← instantiateMVars declaration.type
      return if type.isAppOfArity constant 2 then some declaration.toExpr else none
  let shifts? ← hypothesis? ``VerexIR.Proofs.RunsAgree
  -- The syntax of each hypothesis is made within the attempt: an attempt
  -- restores the state it started from, which forgets the syntax made
  -- before it.
  if let some shifts := shifts? then
    try
        setGoals [goal]
        let shiftsSyntax ← goal.withContext (Lean.Elab.Term.exprToSyntax shifts)
        let namingSyntax ← goal.withContext (Lean.Elab.Term.exprToSyntax naming)
        let rule ← goal.withContext <| Lean.Elab.Term.exprToSyntax
          (mkApp4 (mkConst ``VerexIR.Proofs.wp_named_typed) unit functions executable frame)
        -- A rule whose row conditions fail must fail here, not elaborate
        -- them as `sorry`.
        Term.withoutErrToSorry <| withoutRecover <|
          evalTactic (← `(tactic|
            refine $rule $shiftsSyntax (by decide) (by decide) $namingSyntax ?_))
        let [named] ← getGoals
          | throwError m!"the typed named call leaves one goal, not {(← getGoals).length}: \
              {← (← getGoals).mapM fun goal => do pure (← goal.getType)}"
        return ← if throughContract then markNamedRead named else pure named
    catch failure =>
        saved.restore
        if verex.solverDebug.get (← getOptions) then
          logInfo m!"typed named call not taken: {failure.toMessageData}"
  do
    try
      setGoals [goal]
      let namingSyntax ← goal.withContext (Lean.Elab.Term.exprToSyntax naming)
      let rule ← goal.withContext <| Lean.Elab.Term.exprToSyntax
        (mkApp4 (mkConst ``VerexIR.Proofs.wp_named) unit functions frame executable)
      Term.withoutErrToSorry <| withoutRecover <|
        evalTactic (← `(tactic| refine $rule $namingSyntax ?_))
      let [named] ← getGoals
        | throwError m!"the named call leaves one goal, not {(← getGoals).length}: \
            {← (← getGoals).mapM fun goal => do pure (← goal.getType)}"
      return ← if throughContract then markNamedRead named else pure named
    catch failure =>
      saved.restore
      if verex.solverDebug.get (← getOptions) then
        logInfo m!"named call not taken: {failure.toMessageData}"
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
re-introduce locals, so the solver recognizes it by name rather than by
identity. -/
def continuationName : Name := `verexContinuation

/-- Whether an action can branch, so that its continuation would be reached
on several paths. -/
private def mayBranch (action : Lean.Expr) : MetaM Bool := do
  let env ← getEnv
  return (action.find? fun sub => match sub.getAppFn with
    | .const name _ =>
        name == ``ite || name == ``dite || name == ``VerexIR.Proofs.Denote.loopAt ||
          Lean.Meta.isMatcherCore env name
    | _ => false).isSome

/-- A bind at the head of a goal, taken by its rule: the goal with the
action at the head, whether the action can branch, and what the normalizer
takes as it is: the action, the continuation the rule builds (normalized
where it is applied), the two conditions, and the state, in this order. -/
def bindRule? (goal : MVarId) : MetaM (Option (MVarId × Bool × Array Lean.Expr)) :=
    goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
  let action := (target.getArg! 3).consumeMData.headBeta
  let conditions := #[target.getArg! 4, target.getArg! 5, target.getArg! 6]
  let rule ← if action.isAppOfArity ``VerexIR.Proofs.Spec.bind 6 then
      mkAppM ``VerexIR.Proofs.wp_bind (#[action.getArg! 4, action.getArg! 5] ++ conditions)
    else if action.isAppOfArity ``VerexIR.Proofs.Denote.Flow.bind 9 then
      let inner := (action.getArg! 7).consumeMData.headBeta
      if inner.isAppOfArity ``VerexIR.Proofs.Denote.Flow.bind 9 then
        -- Flows associate to the right, so that the goal's continuation
        -- stays out of the inner bind's arms.
        pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_flowBind_flowBind)
          (action.getAppArgs.extract 0 5 ++
            #[inner.getArg! 5, inner.getArg! 6, action.getArg! 6, inner.getArg! 7,
              inner.getArg! 8, action.getArg! 8] ++ conditions))
      else if inner.isAppOfArity ``VerexIR.Proofs.Spec.bind 6 then
        -- A flow after a bind binds inside the bind's continuation, where the
        -- goal's continuation appears once.
        pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_flowBind_specBind)
          ((action.getAppArgs.extract 0 7).push (inner.getArg! 2) ++
            #[inner.getArg! 4, inner.getArg! 5, action.getArg! 8] ++ conditions))
      else
      -- The flow's own skolem family and types instantiate the rule.
      pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_flowBind) (action.getAppArgs ++ conditions))
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
def foldContinuation (goal : MVarId) : MetaM (MVarId × Option FVarId) :=
    goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return (goal, none)
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
solver takes it apart, its quantifiers introduced by name for an authored
proof and its existentials left to a witness, so an invariant goal's
markers are set aside rather than sealing its clauses. -/
partial def stripObligations (goal : MVarId) (clauses : Array ObligationRange) :
    MetaM (MVarId × Array ObligationRange) := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``VerexIR.Proofs.Obligation 4 do return (goal, clauses)
  let clauses := match obligationRange? target with
    | some range => clauses.push range
    | none => clauses
  stripObligations (← goal.replaceTargetDefEq (target.getArg! 3)) clauses


/-- The goal with the flagged locals as implementation details. -/
def markDetails (goal : MVarId) (fvars : Array (FVarId × Bool)) : MetaM MVarId :=
  goal.withContext do
    let mut lctx ← getLCtx
    for (fvarId, detail) in fvars do
      if detail then lctx := lctx.modifyLocalDecl fvarId (·.setKind .implDetail)
    let next ← mkFreshExprMVarAt lctx (← getLocalInstances) (← goal.getType) .syntheticOpaque
      (← goal.getTag)
    goal.assign next
    return next.mvarId!

/-- The leaves of one clause as one goal, a conjunction over their paths:
each leaf's statement quantified over the locals and hypotheses its path
introduced beyond the leaves' common context, and each leaf proved by its
part of the conjunction. -/
def combineLeaves (tag : Name) (leaves : Array MVarId) : MetaM (Option MVarId) := do
  let contexts ← leaves.mapM fun leaf => do
    return ((← leaf.getDecl).lctx.decls.toList.filterMap id).toArray
  let first := contexts[0]!
  -- The common prefix of the leaves' contexts, as far as it is well scoped:
  -- a replaced hypothesis may read a local declared after it.
  let fvarsOf (decl : Lean.LocalDecl) : Array FVarId :=
    (collectFVars (decl.value?.map (collectFVars {}) |>.getD {}) decl.type).fvarIds
  let mut shared : Std.HashSet FVarId := {}
  let mut common := 0
  while common < first.size && contexts.all (fun decls =>
      common < decls.size && decls[common]!.fvarId == first[common]!.fvarId) &&
      (fvarsOf first[common]!).all shared.contains do
    shared := shared.insert first[common]!.fvarId
    common := common + 1
  let mut lctx := (← leaves[0]!.getDecl).lctx
  for decl in first.extract common first.size do lctx := lctx.erase decl.fvarId
  let instances := (← leaves[0]!.getDecl).localInstances.filter fun inst =>
    shared.contains inst.fvar.fvarId!
  let mut statements := #[]
  let mut paths := #[]
  let mut orders := #[]
  for leaf in leaves, decls in contexts do
    -- The path's locals in an order their types allow.
    let mut remaining := decls.extract common decls.size
    let mut placed := shared
    let mut extras := #[]
    while !remaining.isEmpty do
      let some index := remaining.findIdx? fun decl => (fvarsOf decl).all placed.contains
        | return none
      extras := extras.push remaining[index]!
      placed := placed.insert remaining[index]!.fvarId
      remaining := remaining.eraseIdx! index
    -- The leaf's statement under its path's locals, each a binder: a `let`
    -- for a defined local, whether or not the statement reads it.
    let statement ← leaf.withContext do
      let xs := extras.map (·.toExpr)
      let mut body := (← instantiateMVars (← leaf.getType)).abstractRange xs.size xs
      for index in [0:extras.size] do
        let i := extras.size - 1 - index
        let decl := extras[i]!
        let type := (← instantiateMVars decl.type).abstractRange i xs
        body ← match decl.value? with
          | some value => do
              pure (mkLet decl.userName type ((← instantiateMVars value).abstractRange i xs) body)
          | none => pure (Lean.mkForall decl.userName decl.binderInfo type body)
      pure body
    if statement.hasAnyFVar (fun id => !shared.contains id) then return none
    statements := statements.push statement
    orders := orders.push extras
    let (provenance, clauses) := ((← (← targetState).leafOrigins.get)[leaf]?).getD (none, #[])
    let decisions := ((← (← targetState).leafDecisions.get)[leaf]?).getD #[]
    paths := paths.push (Name.mkSimple s!"{tag}_{paths.size + 1}", provenance, clauses,
      extras.map (·.isImplementationDetail), decisions)
  let mut statement := statements.back!
  for part in statements.pop.reverse do statement := mkAnd part statement
  let combined ← mkFreshExprMVarAt lctx instances statement .syntheticOpaque tag
  for leaf in leaves, extras in orders, index in [0:leaves.size] do
    let mut part := combined
    for _ in [0:index] do part := .proj ``And 1 part
    if index + 1 < leaves.size then part := .proj ``And 0 part
    for decl in extras do
      unless decl.isLet do part := mkApp part decl.toExpr
    leaf.assign part
  (← targetState).leafPaths.modify (·.insert combined.mvarId! paths)
  return some combined.mvarId!

end VerexIR.Proofs.Denote
