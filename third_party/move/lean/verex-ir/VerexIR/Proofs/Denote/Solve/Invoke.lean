-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Decide
import VerexIR.Proofs.ReceiverScoped

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
    (rewrite? : Lean.Expr → SimpM (Option Simp.Step) := fun _ => pure none)
    (base? : Option Simp.Methods := none) : Simp.Methods :=
  let base := base?.getD (Simp.mkDefaultMethodsCore simprocs)
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
    (normal : Array Lean.Expr) (fixedPoints : Option Name := none) : TacticM Unit := do
  let goal ← getMainGoal
  let saved ← saveState
  try
    goal.withContext do
      let (ctx, simprocs) := normalization
      let known : Std.HashSet Lean.Expr := normal.foldl (·.insert ·) {}
      let target ← instantiateMVars (← goal.getType)
      let stop := fun e => pure (known.contains e)
      let fp ← FixedPoints.new? fixedPoints
      let base := Simp.mkDefaultMethodsCore simprocs
      let base ← canonicalEmissionMethods base known fp
      let methods := match fp with
        | some fp => fp.wrap base stop
        | none => stoppingMethods simprocs stop (base? := some base)
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
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.loopAt 7 do return none
    let some site ← siteOf? (action.getArg! 4) | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The in-body assertion a goal's weakest precondition is over, if any: its
site, and the arguments of `wp_assertAt` besides the condition. -/
def assertionGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.assertAt 6 do return none
    let some site ← siteOf? (action.getArg! 4) | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The end of a mutation a goal's weakest precondition is over, if any: its
site, and the arguments of `wp_mutationEndAt` besides the invariant. -/
def mutationEndGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.mutationEndAt 6 do return none
    let some site ← siteOf? (action.getArg! 4) | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The end of a write of global memory a goal's weakest precondition is
over, if any: its site, and the arguments of `wp_memoryWrittenAt` besides the
invariant. -/
def memoryWrittenGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.memoryWrittenAt 6 do return none
    let some site ← siteOf? (action.getArg! 4) | return none
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
    siteOf? (action.getArg! (action.getAppNumArgs - 2))

/-- The call a goal's weakest precondition is over, if any: its site, and
the arguments of `wp_callAt`. -/
def callAtGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.callAt 4 do return none
    let some site ← siteOf? (action.getArg! 2) | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The in-body assumption a goal's weakest precondition is over, if any:
the arguments of `wp_assumeAt` before its continuation. -/
def assumeGoal? (goal : MVarId) : MetaM (Option (Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.assumeAt 7 do return none
    return some (action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The invariants owed where a call returns and what follows, if a goal
is over them (`CallChecked`). -/
def callCheckedGoal? (goal : MVarId) : MetaM (Option (Lean.Expr × Lean.Expr)) :=
  goal.withContext do
    let target := (← instantiateMVars (← goal.getType)).headBeta
    unless target.isAppOfArity ``VerexIR.Proofs.Denote.CallChecked 2 do return none
    return some (target.getArg! 0, target.getArg! 1)

/-- A certifying step at the head of the goal: the site it is reported at,
what it certifies, and the arguments the cut takes in the order
`wp_certifyingAt_intro` reads them (`certifyingAt`). -/
def certifyingGoal? (goal : MVarId) :
    MetaM (Option (Nat × Lean.Expr × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.certifyingAt 6 do return none
    let some site ← siteOf? (action.getArg! 2) | return none
    return some (site, action.getArg! 3,
      action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The construction a goal's weakest precondition is over, if any: its
site, and the arguments of `wp_constructedAt` besides the invariant. -/
def constructionGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.constructedAt 8 do return none
    let some site ← siteOf? (action.getArg! 5) | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The ghost write a goal's weakest precondition is over, if any: its site,
and the arguments of `wp_ghostAt`. -/
def ghostWriteGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.ghostAt 8 do return none
    let some site ← siteOf? (action.getArg! 5) | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- The state anchor a goal's weakest precondition is over, if any: its
site, and the arguments of `wp_anchorAt`. -/
def anchorGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``VerexIR.Proofs.Denote.anchorAt 6 do return none
    let some site ← siteOf? (action.getArg! 4) | return none
    return some (site, action.getAppArgs ++ #[target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- Whether the path records the anchor of a site: the solver records one
where it steps the site's `anchorAt`. -/
def anchorRecorded (goal : MVarId) (site : Nat) : MetaM Bool := goal.withContext do
  for declaration in ← getLCtx do
    let recorded ← instantiateMVars declaration.type
    if recorded.isAppOfArity ``AnchorSaved 5 then
      if (← siteOf? (recorded.getArg! 2)) == some site then return true
  return false

/-- A predicate over the locals and state of state anchors, applied to those
the anchors recorded last on the path. -/
partial def resolveAnchors (goal : MVarId) (predicate : Lean.Expr) :
    MetaM Lean.Expr := goal.withContext do
  let .lam _ type _ _ := predicate | return predicate
  unless type.isAppOfArity ``VerexIR.Proofs.Denote.AnchorOf 4 do return predicate
  let some site ← siteOf? (type.getArg! 2)
    | throwError m!"an anchor site must be a numeral, not {type.getArg! 2}"
  let mut saved := none
  for declaration in ← getLCtx do
    let recorded ← instantiateMVars declaration.type
    if recorded.isAppOfArity ``AnchorSaved 5 then
      if (← siteOf? (recorded.getArg! 2)) == some site then
        saved := some (recorded.getArg! 3, recorded.getArg! 4)
  let some (env, state) := saved
    | throwError m!"the state anchor at site {site} is read where it is not recorded"
  resolveAnchors goal (← whnfR (mkApp predicate (← mkAppM ``Prod.mk #[env, state])))

/-- A case of a split, its match reduced at the pattern the case fixed: the
computation of the discriminant takes its branch there, so the matcher
reduces to the alternative the case runs. -/
private def reduceCaseMatcher (goal : MVarId) : MetaM MVarId := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  let action := (target.getArg! 3).consumeMData
  let .const name _ := action.getAppFn
    | throwError "a match on a constructor does not reduce"
  let some info ← getMatcherInfo? name
    | throwError "a match on a constructor does not reduce"
  let position := info.getFirstDiscrPos
  let arguments := action.getAppArgs
  let computed ← whnfCore arguments[position]!
  let .reduced alternative ←
      reduceMatcher? (mkAppN action.getAppFn (arguments.set! position computed))
    | throwError "a match on a constructor does not reduce"
  goal.replaceTargetDefEq
    (mkAppN target.getAppFn (target.getAppArgs.set! 3 alternative.headBeta))

/-- The cases of a weakest precondition over a match whose discriminant
reads an option (`withSome`), by cases on that option: the reading takes
the branch of the pattern the case fixed, and the case states the option's
own pattern (`withSome_cases`).  Taken this way and not by generalizing the
reading, which would leave each case stating the computation the split has
just consumed: a leaf cannot read that statement without taking the
computation again, while the option's pattern is what the normal forms
decide and what a context fixing the value refutes.  A chain of readings
(`Choices.selectCertified?`) is split one reading at a time: the branch past
the reading split here is the next reading, whose own match this rule takes
again, so that every case states one projection's pattern.  `none` where the
discriminant reads no option, or already reads a pattern. -/
private def splitReadingAction? (goal : MVarId) (target reading : Lean.Expr) :
    MetaM (Option (List MVarId)) := do
  unless reading.isAppOfArity ``withSome 5 do return none
  let arguments := reading.getAppArgs
  let (α, β, option, found, missing) :=
    (arguments[0]!, arguments[1]!, arguments[2]!, arguments[3]!, arguments[4]!)
  let read ← whnfR option
  if read.isAppOf ``Option.some || read.isAppOf ``Option.none then return none
  -- The goal as a property of the value read, abstracted at this reading:
  -- every point of the goal reading it reads the branch of the same case.
  let abstracted ← kabstract target reading
  unless abstracted.hasLooseBVars do return none
  let motive := Lean.mkLambda `verexRead .default β abstracted
  unless ← isTypeCorrect motive do return none
  let isSome ← withLocalDeclD `verexRead α fun value => do
    let pattern ← mkEq option (← mkAppOptM ``Option.some #[α, value])
    withLocalDeclD `verexReads pattern fun equation =>
      mkForallFVars #[value, equation]
        (motive.beta #[mkApp2 found value equation])
  let isNone ← do
    let pattern ← mkEq option (← mkAppOptM ``Option.none #[α])
    withLocalDeclD `verexReads pattern fun equation =>
      mkForallFVars #[equation] (motive.beta #[missing])
  let tag ← goal.getTag
  let isSome ← mkFreshExprSyntheticOpaqueMVar isSome tag
  let isNone ← mkFreshExprSyntheticOpaqueMVar isNone tag
  let cases ← observing? do
    goal.assign (mkAppN (mkConst ``withSome_cases)
      #[α, β, motive, option, found, missing, isSome, isNone])
    let (_, taken) ← isSome.mvarId!.introNP 2
    let (_, missed) ← isNone.mvarId!.introNP 1
    -- The branch the reading takes reads the pattern its case fixed, so its
    -- match reduces.  The branch past it reduces where it is a pattern of its
    -- own, and is the next reading of a chain where it is not: that match is
    -- kept, for this rule to take again.
    let reduced ← observing? (reduceCaseMatcher missed)
    pure [← reduceCaseMatcher taken, reduced.getD missed]
  return cases

/-- The cases of a weakest precondition over a match on one discriminant,
by cases on that discriminant at the goal's own instance: each case states
the pattern it took (`d = some x`) and runs its alternative. `none` where
the match has several discriminants or a dependent motive. -/
def splitMatchAction? (goal : MVarId) : MetaM (Option (List MVarId)) := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
  -- The action as the walk reads it, under the elaborator's annotations.
  let action := (target.getArg! 3).consumeMData
  let .const name _ := action.getAppFn | return none
  let some info ← getMatcherInfo? name | return none
  unless info.numDiscrs == 1 && action.getAppNumArgs == info.arity do return none
  let arguments := action.getAppArgs
  let .lam _ _ motive _ := arguments[info.getMotivePos]! | return none
  if motive.hasLooseBVars then return none
  let discriminant := arguments[info.getFirstDiscrPos]!
  if let some cases ←
      splitReadingAction? goal target discriminant.consumeMData then
    return some cases
  let type ← inferType discriminant
  -- The instance, generalized over its discriminant.
  let generalized ← withLocalDeclD `x type fun x => do
    let body := mkAppN target.getAppFn (target.getAppArgs.set! 3
      (mkAppN action.getAppFn (arguments.set! info.getFirstDiscrPos x)))
    withLocalDeclD `heq (← mkEq discriminant x) fun heq => mkForallFVars #[x, heq] body
  let next ← mkFreshExprSyntheticOpaqueMVar generalized (← goal.getTag)
  goal.assign (mkApp2 next discriminant (← mkEqRefl discriminant))
  let (introduced, next) ← next.mvarId!.introNP 2
  let some cases ← observing? (next.cases introduced[0]!) | return none
  cases.toList.mapM fun case => case.mvarId.withContext do
    let caseTarget ← instantiateMVars (← case.mvarId.getType)
    let .reduced alternative ← reduceMatcher? (caseTarget.getArg! 3)
      | throwError "a match on a constructor does not reduce"
    case.mvarId.replaceTargetDefEq
      (mkAppN caseTarget.getAppFn (caseTarget.getAppArgs.set! 3 alternative.headBeta))

/-- Split a binder, conjunction or conditional. Dependent actions introduce
  their branch assumptions before normalization. If a computation cannot be
  split, preparation may expose more program steps for the structural walker.
  With `obligations`, an obligation a hypothesis states closes whole, before
  its parts split. Without `cases`, a conditional is split only where the goal
  still runs program: an obligation's own conditionals stay in its leaf. -/
def splitOnce (goal : MVarId) (obligations : Bool := true) (cases : Bool := obligations)
    (prepare : Bool := true) :
    TacticM (Option (List MVarId × Bool)) := do
  -- The exit binds `result` in a step proof, before the paths split.
  let bound ← IO.mkRef none
  timedStep "split/result" do bound.set (← bindResult goal)
  if let some next ← bound.get then return some ([next], false)
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  -- A quantified or conjoined obligation a hypothesis states closes as it is:
  -- its parts, once split, are no longer the hypothesis's.
  let statement := StatedFacts.unmarked target
  if obligations && (statement.isForall || statement.isAppOfArity ``And 2) &&
      (statement.find? (·.isAppOf ``VerexIR.Proofs.wp)).isNone then
    if ← StatedFacts.close goal (exact := true) then return some ([], true)
  if target.isForall then
    -- A quantified variable keeps its name, for an authored proof; a
    -- borrow's prophecy mark is consumed.
    let (introduced, next) ← goal.intro1P
    noteKeptPremise next introduced
    if (prophecyMark? target.bindingDomain!).isSome then
      return some ([← consumeProphecy next introduced], false)
    return some ([next], false)
  if target.isAppOfArity ``And 2 then
    let subgoals ← goal.apply (mkConst ``And.intro)
    return some (subgoals, false)
  -- A dependent action carries its bounds proof into the result. Introduce
  -- the branch assumption before normalization, so the continuation is
  -- processed as computation rather than left beneath a proof binder.
  -- A source `let` binds its locals' names in a step proof, and the exit
  -- `result`.
  timedStep "split/let" do bound.set (← bindLet goal)
  if let some next ← bound.get then return some ([next], false)
  if target.isAppOfArity ``VerexIR.Proofs.wp 7 then
    let action := (target.getArg! 3).consumeMData
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
    -- A conditional splits by its rule at the goal's own instance, not by
    -- `split`, which abstracts the conditional over the whole goal.
    if action.isAppOfArity ``ite 5 then
      let rule := mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_ite)
        #[target.getArg! 0, target.getArg! 2, action.getArg! 1, action.getArg! 2,
          action.getArg! 3, action.getArg! 4, target.getArg! 4, target.getArg! 5, target.getArg! 6]
      let_expr Iff lhs rhs := ← goal.withContext (do whnfR (← inferType rule))
        | throwError "a conditional rule is not an equivalence"
      setGoals (← goal.apply (mkApp3 (mkConst ``Iff.mpr) lhs rhs rule))
      evalTactic (← `(tactic| constructor <;> intro h))
      return some (← getGoals, true)
  let saved ← saveState
  -- A match splits by cases on its discriminant at the goal's own instance,
  -- not by `split`, which abstracts it over the whole goal; a read of memory
  -- the context fixes decides it first.
  -- A match elaborated over a compound discriminant carries the
  -- elaborator's `save_info` annotation; the walk reads an action under
  -- its annotations.
  let matching ← if target.isAppOfArity ``VerexIR.Proofs.wp 7 then
      isMatcherApp (target.getArg! 3).consumeMData
    else pure false
  if matching then
    setGoals [goal]
    evalTactic (← `(tactic| try verex_memory_reads))
    if (← getGoals) != [goal] then return some (← getGoals, true)
    let known ← goal.withContext do return (← getLCtx).getFVarIds
    if let some cases ← splitMatchAction? goal then
      return some (← statePatterns known cases, true)
    saved.restore
  let running := (target.find? (·.isAppOf ``VerexIR.Proofs.wp)).isSome
  -- An obligation a hypothesis states closes before its cases multiply.
  if obligations && !running then
    try
      setGoals [goal]
      evalTactic (← `(tactic| verex_assumption))
      return some ([], true)
    catch _ =>
      saved.restore
  if cases || running then
    try
      setGoals [goal]
      timedStep "split/first" (evalTactic (← `(tactic| verex_split)))
      return some (← getGoals, true)
    catch _ =>
      saved.restore
  -- Preparation may substitute a computed index and expose more program
  -- steps. Resume the structural walker only when the computation changes;
  -- an unchanged wp falls through to the ordinary leaf solver.
  if prepare && target.isAppOfArity ``VerexIR.Proofs.wp 7 then
    try
      setGoals [goal]
      timedStep "split/prepare" (evalTactic (← `(tactic| verex_prepare_leaf)))
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
as a local function, as a function calling itself is, or the registered
function applied to the call's value arguments. The latter includes an
interpreter capability specialized to its target and signature. -/
def callGoal? (goal : MVarId) (callees : Array (Lean.Expr × String × Lean.Expr)) :
    MetaM (Option (Lean.Expr × Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    if action.isAppOfArity ``VerexIR.Proofs.Denote.propheticMeaning 9 then
      return some (action.getArg! 5, action)
    if action.isAppOfArity ``VerexIR.Proofs.Denote.propheticMeaningAt 10 then
      return some (action.getArg! 6, action)
    let head := action.getAppFn
    if head.isFVar && callees.any (·.1 == head) then return some (head, action)
    if action.isApp then
      let function := action.appFn!
      for (candidate, _, _) in callees do
        if candidate == function ||
            (← withNewMCtxDepth <| withReducible (isDefEq candidate function)) then
          return some (candidate, action)
    return none

/-- The reads of `params` in `body`: its maximal subterms that mention one
of them and no other local, closed under the body's own binders. A bare
occurrence is a read of its own. -/
private def readsOf (body : Lean.Expr) (params : Array FVarId) : Array Lean.Expr := Id.run do
  let (_, mentioned) := (localMentions body (params.contains ·)).run {}
  let mut reads : Array Lean.Expr := #[]
  let mut visited : Std.HashSet Lean.Expr := {}
  let mut pending : Array Lean.Expr := #[body]
  while let some e := pending.back? do
    pending := pending.pop
    if visited.contains e then continue
    visited := visited.insert e
    let (mentions, mentionsOther) := mentioned.getD e (false, false)
    unless mentions do continue
    if !e.hasLooseBVars && !mentionsOther then
      unless reads.contains e do reads := reads.push e
      continue
    match e with
    | .app f a => pending := pending.push a |>.push f
    | .lam _ t b _ | .forallE _ t b _ => pending := pending.push b |>.push t
    | .letE _ t v b _ => pending := pending.push b |>.push v |>.push t
    | .mdata _ b | .proj _ _ b => pending := pending.push b
    | _ => pure ()
  return reads

/-- A continuation `fun rs => B` abstracted over its reads of `rs`: the
value `fun vs => B[vs/reads]`, and the continuation again as that value
applied to the reads, `fun rs => h reads`, with `h` the placeholder for the
value's name. None where a read's type depends on the parameters. -/
private def abstractReads (continuation : Lean.Expr) :
    MetaM (Option (Lean.Expr × FVarId × Lean.Expr)) :=
  lambdaTelescope continuation fun rs body => do
    let params := rs.map (·.fvarId!)
    let reads := readsOf body params
    let types ← reads.mapM fun read => do instantiateMVars (← inferType read)
    if types.any (·.hasAnyFVar (params.contains ·)) then return none
    let declarations := types.zipIdx.map fun (type, i) =>
      (Name.mkSimple s!"read{i}", fun (_ : Array Lean.Expr) => pure type)
    withLocalDeclsD declarations fun vs => do
      let abstracted := body.replace fun e =>
        (reads.findIdx? (· == e)).map fun i => vs[i]!
      let value ← mkLambdaFVars vs abstracted
      withLocalDeclD `held (← inferType value) fun h => do
        let wrapper ← mkLambdaFVars rs (mkAppN h reads)
        return some (value, h.fvarId!, wrapper)

/-- The main goal with the continuations `parts`, lambdas of its weakest
precondition, as local definitions abstracted over their reads of their
parameters (`abstractReads`), wherever they occur: a rule applying a
continuation to fresh variables, or to a transport of the result, then
substitutes into a visible read, which a step's simp normalizes without
walking the body, normal as it was. The definitions with their values. -/
def holdContinuations (parts : Array Lean.Expr) : TacticM (Array (FVarId × Lean.Expr)) := do
  let mut goal ← getMainGoal
  let mut held := #[]
  for part in parts do
    unless part.consumeMData.isLambda do continue
    let (next, entry) ← goal.withContext do
      let target ← instantiateMVars (← goal.getType)
      unless (target.find? (· == part)).isSome do return (goal, none)
      let some (value, placeholder, wrapper) ← abstractReads part.consumeMData
        | return (goal, none)
      let defined ← goal.define `verexHeld (← inferType value) value
      let (local_, folded) ← defined.intro1P
      folded.withContext do
        let wrapper := wrapper.replaceFVarId placeholder (mkFVar local_)
        let target ← instantiateMVars (← folded.getType)
        let folded ← folded.replaceTargetDefEq
          (target.replace fun e => if e == part then some wrapper else none)
        return (folded, some (local_, value))
    goal := next
    if let some entry := entry then held := held.push entry
  replaceMainGoal [goal]
  return held

/-- The subterms of `e`, without those that hold one of `values`. -/
private def subtermsWithout (e : Lean.Expr) (values : Std.HashSet Lean.Expr) :
    Std.HashSet Lean.Expr := Id.run do
  let mut holds : Std.HashMap Lean.Expr Bool := {}
  let mut pending : Array (Lean.Expr × Bool) := #[(e, false)]
  while let some (e, visited) := pending.back? do
    pending := pending.pop
    if holds.contains e then continue
    let children := match e with
      | .app f a => #[f, a]
      | .lam _ t b _ | .forallE _ t b _ => #[t, b]
      | .letE _ t v b _ => #[t, v, b]
      | .mdata _ b | .proj _ _ b => #[b]
      | _ => #[]
    if values.contains e then
      holds := holds.insert e true
    else if visited || children.isEmpty then
      holds := holds.insert e (children.any fun c => holds.getD c false)
    else
      pending := pending.push (e, true) ++ children.map (·, false)
  return holds.fold (init := {}) fun known e held => if held then known else known.insert e

/-- Whether `e` is built by a logical connective or quantifier. -/
private def isConnective (e : Lean.Expr) : Bool :=
  e.isForall || e.isAppOfArity ``And 2 || e.isAppOfArity ``Or 2 || e.isAppOfArity ``Not 1 ||
    e.isAppOfArity ``Iff 2 || e.isAppOfArity ``Exists 2 || e.isConstOf ``True || e.isConstOf ``False

/-- Normalize the main goal's propositional structure, taking every
proposition that is no connective as it is. -/
def normalizeConnectives (normalization : Simp.Context × Simp.SimprocsArray) : TacticM Unit := do
  let goal ← getMainGoal
  let saved ← saveState
  try
    goal.withContext do
      let (ctx, simprocs) := normalization
      let target ← instantiateMVars (← goal.getType)
      let (result, _) ← Simp.main target ctx (methods := stoppingMethods simprocs fun e =>
        pure !isConnective e.consumeMData <&&> (Lean.Expr.isProp <$> inferType e))
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

/-- The goal without the held definitions `locals`: cleared at once where
nothing else reads them, else each as far as possible. -/
private def clearHeld (goal : MVarId) (locals : Array FVarId) : MetaM MVarId :=
  goal.withContext do
    if locals.isEmpty then return goal
    let held (id : FVarId) := locals.contains id
    let read ← (← getLCtx).anyM fun decl =>
      if held decl.fvarId then pure false else localDeclDependsOnPred decl held
    if read || (← dependsOnPred (← goal.getType) held) then
      return ← goal.tryClearMany locals
    let lctx := locals.foldl (·.erase ·) (← getLCtx)
    let next ← mkFreshExprMVarAt lctx (← getLocalInstances) (← goal.getType)
      .syntheticOpaque (← goal.getTag)
    goal.assign next
    return next.mvarId!

/-- Resume a released continuation by normalizing its action first. A bind
still waiting for its action keeps its postconditions intact until the solver
cuts that bind. This avoids expanding every branch of a function's finish
before the completed action has supplied its result and environment.

This scheduling belongs to continuation release: initial normalization must
still prepare the clauses of branches and loops in full. -/
private def releaseWpAction (e : Lean.Expr) : SimpM Simp.Step := do
  let args := e.getAppArgs
  let action := args[3]!
  let result ← Simp.simp action
  let complete := result.expr.isAppOf ``Spec.pure || result.expr.isAppOf ``Spec.abort
  if result.expr == action then
    if action.isAppOf ``Flow.bind || action.isAppOf ``Spec.bind then
      return .done { expr := e }
    return .continue
  let head := mkAppN e.getAppFn (args.extract 0 3)
  let rest := args.extract 4 args.size
  let expr := mkAppN (mkApp head result.expr) rest
  let proof? ← result.proof?.mapM fun proof => do
    let p ← mkCongrArg head proof
    rest.foldlM (fun proof arg => mkCongrFun proof arg) p
  if complete then return .visit { expr, proof? }
  if result.expr.isAppOf ``Flow.bind || result.expr.isAppOf ``Spec.bind then
    return .done { expr, proof? }
  return .visit { expr, proof? }

/-- The goals with the continuations `holdContinuations` set aside back in
place, reduced at their arguments. The goal's leading binders, the call's
result, final state and premises, are introduced, so that the continuation's
body is closed: a variable argument leaves it normal, so the goal is
normalized along the paths to the other arguments, everything else taken as
it is. -/
def releaseContinuations (normalization : Simp.Context × Simp.SimprocsArray)
    (held : Array (FVarId × Lean.Expr)) (fixedPoints : Option Name := none) : TacticM Unit := do
  if held.isEmpty then return
  let lookup (e : Lean.Expr) : Option Lean.Expr :=
    match e.getAppFn with
    | .fvar id => (held.find? (·.1 == id)).map (·.2)
    | _ => none
  let (ctx, simprocs) := normalization
  let locals := held.map (·.1)
  let mut released := #[]
  for goal in ← getGoals do
    let target ← goal.withContext do instantiateMVars (← goal.getType)
    unless (target.find? fun e => (lookup e).isSome).isSome do
      released := released.push (← clearHeld goal locals)
      continue
    let rec binders : Lean.Expr → Nat
      | .forallE _ _ b _ => binders b + 1
      | .mdata _ b => binders b
      | _ => 0
    let saved ← saveState
    try
      let (introduced, inner) ← goal.introNP (binders target)
      let next ← inner.withContext do
        let target ← instantiateMVars (← inner.getType)
        -- The arguments other than variables: a read a value stands in
        -- may reduce with what surrounds it.
        let mut arguments : Array Lean.Expr := #[]
        -- The applications, and whether one is under a binder.
        let (applications, bound) := Id.run do
          let mut found : Array Lean.Expr := #[]
          let mut bound := false
          let mut visited : Std.HashSet (Lean.Expr × Bool) := {}
          let mut pending := #[(target, false)]
          while let some (e, under) := pending.back? do
            pending := pending.pop
            if visited.contains (e, under) then continue
            visited := visited.insert (e, under)
            if (lookup e).isSome then
              found := found.push e
              bound := bound || under
              continue
            match e with
            | .app f a => pending := pending.push (a, under) |>.push (f, under)
            | .lam _ t b _ | .forallE _ t b _ => pending := pending.push (b, true) |>.push (t, true)
            | .letE _ t v b _ => pending := pending.push (b, true) |>.push (v, true) |>.push (t, true)
            | .mdata _ b | .proj _ _ b => pending := pending.push (b, under)
            | _ => pure ()
          return (found, bound)
        for e in applications do
          for a in e.getAppArgs do
            unless a.isFVar || arguments.contains a do arguments := arguments.push a
        let mut substituted : Std.HashSet Lean.Expr := arguments.foldl (·.insert ·) {}
        -- What the call assigned, such as a generic callee's instantiation,
        -- stands in the held values like an argument.
        for (_, value) in held do
          for mvar in (value.collectMVars {}).result do
            if ← mvar.isAssigned then
              substituted := substituted.insert (← instantiateMVars (mkMVar mvar))
        let restored ← instantiateMVars
          (target.replace fun e => (lookup e).map (·.beta e.getAppArgs))
        let next ← clearHeld (← inner.replaceTargetDefEq restored) locals
        -- Premises over the continuation's body are a rule's, as
        -- `wp_named` adds them, and normalized in full.
        let rec conclusion : Lean.Expr → Lean.Expr
          | .forallE _ _ b _ => conclusion b
          | .mdata _ b => conclusion b
          | e => e
        -- The structure outside the continuation's lambdas is a rule's, as
        -- `wp_bind` builds it, and normalized too; the binders it reaches.
        let (top, binders) := Id.run do
          let mut found : Std.HashSet Lean.Expr := {}
          let mut binders : Std.HashSet Lean.Expr := {}
          let mut pending := #[conclusion restored]
          while let some e := pending.back? do
            pending := pending.pop
            if found.contains e then continue
            found := found.insert e
            match e with
            | .app f a => pending := pending.push a |>.push f
            | .mdata _ b | .proj _ _ b => pending := pending.push b
            | .lam .. | .forallE .. | .letE .. => binders := binders.insert e
            | _ => pure ()
          return (found, binders)
        -- With nothing substituted, the binders are as the contract's
        -- normalization or the continuation left them, normal: only the
        -- structure, and what a rewrite of it yields, is traversed. The
        -- subterms taken as they are below a rewrite are found on demand.
        let direct := substituted.isEmpty && !bound
        let computed ← IO.mkRef none
        let normal : IO (Std.HashSet Lean.Expr) := do
          if let some normal ← computed.get then return normal
          let normal := (subtermsWithout (conclusion restored) substituted).filter (!top.contains ·)
          computed.set (some normal)
          return normal
        next.withContext do
          let stop := fun e => do
            if direct then
              if binders.contains e then return true
              if top.contains e then return false
            return (← normal).contains e
          let base := Simp.mkDefaultMethodsCore simprocs
          let base := { base with pre := fun e => do
            if e.isAppOfArity ``VerexIR.Proofs.wp 7 then
              match ← releaseWpAction e with
              | .continue none => base.pre e
              | step => pure step
            else base.pre e }
          let methods := match ← FixedPoints.new? fixedPoints with
            | some fp => fp.wrap base stop
            | none => { base with
                pre := fun e => do
                  if ← stop e then return .done { expr := e }
                  base.pre e
                dpre := fun e => do
                  if ← stop e then return .done e
                  base.dpre e }
          let (result, _) ← Simp.main restored ctx (methods := methods)
          if result.expr == restored then return next
          applySimpResultToTarget next restored result
      let (_, next) ← next.revert introduced (preserveOrder := true)
      -- What the body became may restructure the binders over it, as
      -- `forall_and` does; the propositions below them are normal.
      setGoals [next]
      normalizeConnectives normalization
      released := released ++ (← getGoals).toArray
    catch _ =>
      saved.restore
      -- Unreleased, the goal is put back in full and normalized again.
      let next ← goal.withContext do
        let restored := target.replace fun e => (lookup e).map (·.beta e.getAppArgs)
        clearHeld (← goal.replaceTargetDefEq restored) locals
      setGoals [next]
      normalizeAround normalization #[] fixedPoints
      released := released ++ (← getGoals).toArray
  setGoals released.toList

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
  e.find? (fun value => isLiteralValue environment value || value.isAppOf ``functionOfGeneric)

/-- The executable unit an invocation meaning runs: the closures' meaning
over it (`closureMeaning`); none for a model's meaning. -/
private def executableOf? (invoke : Lean.Expr) : Option Lean.Expr :=
  let invoke := invoke.consumeMData
  if invoke.isAppOfArity ``closureMeaning 4 then some (invoke.getArg! 2) else none

/-- The arguments row a set of entry values names: the row their settling is
of, which a clause builds them from.  `none` where the values name no row,
as the current of a mutable reference names no prophecy. -/
private def recoverRow? (carriers row entries : Lean.Expr) : MetaM (Option Lean.Expr) :=
  withNewMCtxDepth do
    let args ← mkFreshExprMVar (mkApp2 (mkConst ``HList) carriers row)
    unless ← isDefEq (mkApp3 (mkConst ``HList.settle) carriers row args) entries do
      return none
    let args ← instantiateMVars args
    if args.hasExprMVar then return none
    return some args

/-- The entry values of an arguments row at a frame (`HList.settle`): the
row the clause form of the behavioral predicates reads. -/
private def settledRow (unit functions frame row args : Lean.Expr) : Lean.Expr :=
  mkApp3 (mkConst ``HList.settle) (mkApp3 (mkConst ``Skolems.toCarriers) unit functions frame)
    row args

/-- Whether a statement is `ensures_of` or `aborts_of` of a known target read
directly (`CallEnsures`, `CallAborts`). -/
private def statesDirectCall (statement : Lean.Expr) : Bool :=
  statement.isAppOfArity ``VerexIR.Proofs.CallEnsures 7 ||
    statement.isAppOfArity ``VerexIR.Proofs.CallAborts 5

/-- Whether a statement is `ensures_of` or `aborts_of` under premises. -/
private partial def states (statement : Lean.Expr) : Bool :=
  if statement.isArrow then states statement.bindingBody!
  else
    let statement := unmarked statement
    statement.isAppOf ``VerexIR.Proofs.EnsuresOf || statement.isAppOf ``VerexIR.Proofs.AbortsOf ||
      statement.isAppOf ``VerexIR.Proofs.AbortsOfEntries || statement.isAppOf ``AtEntries ||
      statement.isAppOf ``VerexIR.Proofs.EnsuresOfEntries || statement.isAppOf ``AtEntryFinals ||
      statesDirectCall statement

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

/-- Whether a statement is `ensures_of` or `aborts_of`, or a disjunction one
of whose alternatives is: a clause's abort conditions, each a condition of
its own (`aborts_if`). -/
private partial def statesAlternative (statement : Lean.Expr) : Bool :=
  let statement := unmarked statement
  if statement.isAppOfArity ``Or 2 then
    statesAlternative (statement.getArg! 0) || statesAlternative (statement.getArg! 1)
  else states statement

/-- Whether a premise of a goal's binders states `ensures_of` or `aborts_of`. -/
private partial def spineStates : Lean.Expr → Bool
  | .forallE _ domain body _ => states domain || spineStates body
  | _ => false

/-- A goal's `ensures_of` and `aborts_of` as hypotheses: the premises of its
spine that state them, and a negated one, or a negated disjunction of abort
conditions one of which states one, whose clause keeps its marker on
`False`.  A negated statement over entry values is refuted by opening it
(`openAtEntries`), which only a hypothesis is. -/
private def introducedBehaviors (goal : MVarId) : TacticM MVarId := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let goal ← if spineStates target then pure (← introduceThroughMarkers goal).1 else pure goal
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let negated := unmarked target
  unless negated.isAppOfArity ``Not 1 && statesAlternative negated.appArg! do return goal
  let goal ← if target.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
      let marked := mkAppN target.getAppFn (target.getAppArgs.pop.push (mkConst ``False))
      goal.replaceTargetDefEq (.forallE `verexAborts negated.appArg! marked .default)
    else pure goal
  return (← goal.intro1P).2

/-- The parts of a statement over the entry values of an arguments row
(`AtEntries`), which a behavioral predicate of a clause is one of
(`AbortsOfEntries`, `EnsuresOfEntries`): the family's carriers, the row,
the property of a row, the entry values, and — where the statement names
them — the finals of the row's mutable references (`AtEntryFinals`). -/
private structure EntriesView where
  carriers : Lean.Expr
  row : Lean.Expr
  holds : Lean.Expr
  entries : Lean.Expr
  finals? : Option Lean.Expr := none

/-- A statement over the entry values of an arguments row, read through the
definition of the predicate that states it. -/
private def entriesView? (statement : Lean.Expr) : MetaM (Option EntriesView) := do
  -- A clause's statement as its marker and a read fact carry it.
  let statement := unmarked statement.consumeMData
  let statement := if statement.isAppOfArity ``VerexIR.Proofs.Denoted 1 then statement.appArg!
    else statement
  let statement := unmarked statement.consumeMData
  if statement.isAppOfArity ``AtEntries 4 then
    return some {
      carriers := statement.getArg! 0
      row := statement.getArg! 1
      holds := statement.getArg! 2
      entries := statement.getArg! 3 }
  if statement.isAppOfArity ``AtEntryFinals 5 then
    return some {
      carriers := statement.getArg! 0
      row := statement.getArg! 1
      holds := statement.getArg! 2
      entries := statement.getArg! 3
      finals? := some (statement.getArg! 4) }
  -- `ensures_of` of a clause, as its definition states it: the invocation
  -- runs at a row of the entry values whose mutable references end at the
  -- finals.
  if statement.isAppOfArity ``VerexIR.Proofs.EnsuresOfEntries 13 then
    let parts := statement.getAppArgs
    let carriers := mkApp3 (mkConst ``Skolems.toCarriers) parts[0]! parts[1]! parts[2]!
    let row := parts[4]!
    let holds ← withLocalDeclD `args (mkApp2 (mkConst ``HList) carriers row) fun args =>
      mkLambdaFVars #[args] (mkAppN (mkConst ``VerexIR.Proofs.EnsuresOf)
        ((parts.extract 0 8 |>.push args) ++ parts.extract 10 13))
    return some { carriers, row, holds, entries := parts[8]!, finals? := some parts[9]! }
  -- `aborts_of` of a clause, as its definition states it: the invocation
  -- aborts at a row of the entry values.
  unless statement.isAppOfArity ``VerexIR.Proofs.AbortsOfEntries 10 do return none
  let parts := statement.getAppArgs
  let carriers := mkApp3 (mkConst ``Skolems.toCarriers) parts[0]! parts[1]! parts[2]!
  let row := parts[4]!
  let holds ← withLocalDeclD `args (mkApp2 (mkConst ``HList) carriers row) fun args =>
    mkLambdaFVars #[args] (mkAppN (mkConst ``VerexIR.Proofs.AbortsOf)
      (parts.extract 0 8 |>.push args |>.push parts[9]!))
  return some { carriers, row, holds, entries := parts[8]! }

/-- The components of a row's equation of two rows of values: `congrArg` of
`Prod.fst` and `Prod.snd` at the type arguments the two sides themselves
spell, recursively, with a component of two equal sides dropped.

The projections are built from the sides, never from the row's type.  A
projection of a row spelled at its own type (`HList row`, `HListPlain row`)
is type-correct only where that type unfolds to a product, which needs
default transparency, and simp declines to work on a goal holding such a
term at all — that is why the splitter of the normal form
(`pairInjectionRows`) cannot open this equation, and why opening it by
rewriting is not a choice of steps but the only form available. -/
private partial def rowComponentEquations (proof : Lean.Expr) (depth : Nat := 8) :
    MetaM (Array Lean.Expr) := do
  let some (_, lhs, rhs) := (← inferType proof).eq? | return #[]
  let (lhs, rhs) := (lhs.consumeMData, rhs.consumeMData)
  if lhs == rhs then return #[]
  if depth == 0 then return #[proof]
  unless lhs.isAppOfArity ``Prod.mk 4 && rhs.isAppOfArity ``Prod.mk 4 do return #[proof]
  let levels := lhs.getAppFn.constLevels!
  let (α, β) := (lhs.getArg! 0, lhs.getArg! 1)
  let mut components := #[]
  for (projection, index) in [(``Prod.fst, 2), (``Prod.snd, 3)] do
    let read := mkApp2 (mkConst projection levels) α β
    let component ← mkExpectedTypeHint (← mkCongrArg read proof)
      (← mkEq (lhs.getArg! index) (rhs.getArg! index))
    components := components ++ (← rowComponentEquations component (depth - 1))
  return components

/-- The components of an entry-value equation a point reads, rewritten into
it.  An equation whose side is the row itself substitutes while the row is
opened (`openEntryValues`); one whose sides are rows of reads of a row does
not, and a point states what it reads only once the behavioral predicate
has been read as of the row (`dispatchBehavior`).  Its components are taken
by projection (`rowComponentEquations`) and each rewritten into the goal
and into the facts: a point that establishes a precondition reads the
component in its goal, and one refuted by a permitted failure reads it in
the fact, its goal being `False`.  A component that is read nowhere
rewrites nothing, so only what the point reads is opened and no other
point's shape changes. -/
private def readEntryValues (goal : MVarId) : TacticM MVarId := do
  let equations ← goal.withContext do
    let mut found := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      unless ["verexEntryValues", "verexEntryFinals"].any
          (decl.userName.toString.startsWith ·) do
        continue
      let some (_, lhs, _) := (← instantiateMVars decl.type).eq? | continue
      if lhs.consumeMData.isFVar then continue
      found := found.push decl.fvarId
    return found
  -- One component rewritten wherever the point reads it.  The equation
  -- itself is left as it stands: it states the row the rules read.
  let rewriteRead (goal : MVarId) (component : Lean.Expr) : TacticM MVarId := do
    let mut current ← (try
        goal.withContext do
          let result ← goal.rewrite (← instantiateMVars (← goal.getType)) component
          goal.replaceTargetEq result.eNew result.eqProof
      catch _ => pure goal)
    let facts ← current.withContext do
      let mut names := #[]
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        if ["verexEntryValues", "verexEntryFinals"].any
            (decl.userName.toString.startsWith ·) then
          continue
        unless ← isProp decl.type do continue
        names := names.push decl.userName
      return names
    for fact in facts do
      current ← (try
          current.withContext do
            let some decl := (← getLCtx).findFromUserName? fact | pure current
            let result ← current.rewrite (← instantiateMVars decl.type) component
            pure (← current.replaceLocalDecl decl.fvarId result.eNew result.eqProof).mvarId
        catch _ => pure current)
    return current
  -- Each equation and its components are read again in the goal at hand: a
  -- rewrite into a fact reintroduces the hypotheses after it, the equations
  -- among them, under new free variables, so no free variable of an earlier
  -- goal is carried into a later one.  An equation is found by its statement,
  -- which no rewrite changes (the equations are not rewritten).
  let statements ← goal.withContext do
    equations.mapM fun equation => do instantiateMVars (← equation.getType)
  let componentsOf (current : MVarId) (statement : Lean.Expr) : TacticM (Array Lean.Expr) :=
    current.withContext do
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        if (← instantiateMVars decl.type) == statement then
          return ← (try rowComponentEquations decl.toExpr catch _ => pure #[])
      return #[]
  let mut current := goal
  let mut stated := #[]
  for statement in statements do
    let count := (← componentsOf current statement).size
    for index in [0:count] do
      let some component := (← componentsOf current statement)[index]? | break
      let componentStatement ← current.withContext do instantiateMVars (← inferType component)
      if stated.contains componentStatement then continue
      stated := stated.push componentStatement
      current ← rewriteRead current component
  return current

/-- A certified value whose forgetting is a lent value is that value
certified: its certificate the one the value satisfies. -/
theorem NTy.eq_ofDeepHolds_of_forget_eq [Carriers] {τ : NTy} {certified : τ.carrier}
    {value : τ.plain} (equal : τ.forget certified = value) (holds : τ.deepHolds value) :
    certified = τ.ofDeepHolds value holds := by
  subst equal
  exact (NTy.ofDeepHolds_forget τ certified).symm

/-- The certificate a lent value satisfies where a certified value forgets
to it. -/
theorem NTy.deepHolds_of_forget_eq [Carriers] {τ : NTy} {certified : τ.carrier}
    {value : τ.plain} (equal : τ.forget certified = value) : τ.deepHolds value :=
  equal ▸ τ.deepHolds_forget certified

/-- An entry-value equation of a certified argument and the lent value a
clause names (`NTy.forget τ a = v`), as the argument certified at that value
(`NTy.eq_ofDeepHolds_of_forget_eq`): the clause names values and no
certificate, so the value is the argument's only spelling the leaf reads,
and the call that passed the argument spells it the same way
(`Term.certifying`).  The certificate is a local of its own, so the
argument substitutes away.  `none` where the equation is not of that
shape. -/
private def certifiedComponent? (goal : MVarId) (component : FVarId) (name : Name) :
    MetaM (Option (MVarId × FVarId)) := goal.withContext do
  let type ← instantiateMVars (← component.getType)
  let some (_, lhs, rhs) := type.eq? | return none
  let (lhs, rhs) := (lhs.consumeMData, rhs.consumeMData)
  let (equation, forgotten, value) ←
    if lhs.isAppOfArity ``NTy.forget 3 && !rhs.isAppOfArity ``NTy.forget 3 then
      pure (Lean.Expr.fvar component, lhs, rhs)
    else if rhs.isAppOfArity ``NTy.forget 3 && !lhs.isAppOfArity ``NTy.forget 3 then
      pure (← mkEqSymm (.fvar component), rhs, lhs)
    else return none
  let certified := forgotten.getArg! 2
  unless certified.isFVar do return none
  if value.containsFVar certified.fvarId! then return none
  -- The family's carriers as the equation spells them: a frame's are no
  -- instance the leaf could synthesize.
  let (carriers, τ) := (forgotten.getArg! 0, forgotten.getArg! 1)
  let holds := mkAppN (mkConst ``NTy.deepHolds_of_forget_eq)
    #[carriers, τ, certified, value, equation]
  let holdsType := mkApp3 (mkConst ``NTy.deepHolds) carriers τ value
  let (holdsVar, goal) ← (← goal.assert (name.appendAfter "Holds") holdsType holds).intro1P
  goal.withContext do
  let proof := mkAppN (mkConst ``NTy.eq_ofDeepHolds_of_forget_eq)
    #[carriers, τ, certified, value, equation, .fvar holdsVar]
  let equalType ← mkEq certified
    (mkApp4 (mkConst ``NTy.ofDeepHolds) carriers τ value (.fvar holdsVar))
  let (equal, goal) ← (← goal.assert name equalType proof).intro1P
  let goal ← goal.clear component
  return some (goal, equal)

/-- The constructor a type a unit declares unfolds to: a type named by a
constant, which the equations of `NTy.settle` and `NTy.forget` do not match.
`none` where the type is spelled as a constructor or is no constant. -/
private def declaredType? (τ : Lean.Expr) : MetaM (Option Name) := do
  let τ := τ.consumeMData
  unless τ.getAppFn.isConst do return none
  if (← getEnv).isConstructor τ.getAppFn.constName! then return none
  let unfolded ← whnfD τ
  let .const name _ := unfolded.getAppFn | return none
  return some name

open Lean Meta Simp in
/-- The settling of an argument of a type a unit declares (a type named by a
constant, not spelled as a constructor), as its forgetting: settling is
forgetting at every type but a mutable reference's (`NTy.settle`), and a
declared type is not one, so the equation reads the argument's certified
value at its own type and not the unfolded cases of `NTy.settle`. -/
dsimproc_decl settleDeclared (@NTy.settle ?carriers _ _) := fun e => do
  unless e.isAppOfArity ``NTy.settle 3 do return .continue
  let some name ← declaredType? (e.getArg! 1) | return .continue
  unless [``NTy.struct, ``NTy.enum, ``NTy.tuple, ``NTy.vector, ``NTy.table].contains name do
    return .continue
  return .visit (mkApp3 (mkConst ``NTy.forget) (e.getArg! 0) (e.getArg! 1) (e.getArg! 2))

/-- An equation of two spelled rows as the equations of their components,
each named `name`, by the injectivity of pairing; an equation of units is
dropped. -/
private partial def injectRowEquation (goal : MVarId) (equation : FVarId) (name : Name) :
    MetaM MVarId := goal.withContext do
  let some (_, left, right) := (← instantiateMVars (← equation.getType)).eq? | return goal
  if left.isAppOfArity ``Prod.mk 4 && right.isAppOfArity ``Prod.mk 4 then
    match ← Lean.Meta.injection goal equation with
    | .solved => return goal
    | .subgoal next components _ =>
        let mut current := next
        for component in components do
          current ← injectRowEquation current component name
        return current
  else if left == right then
    try goal.clear equation catch _ => pure goal
  else goal.rename equation name

/-- The entry-value equation of an arguments row, opened per component: the
row's settling computed (`HList.settle`), a mutable reference's currents
equated, and any other argument's value read at its own type
(`NTy.forget_eq_forget`).  Both entries forms are opened by it: a clause's
`aborts_of` as a hypothesis (`openAtEntries`) and its `requires_of` as a
goal (`dispatchBehavior`).

Its set is closed: the splitter of a row's equation
(`pairInjectionRows`) and the steps that settle its reads back into the
values they came from (`rowOfReads`, `certifiedByConstruction`) belong to
the normal form, not to a pass, and taken here on every point they leave
shapes the pass does not settle again.  Where the goal reads a component
the equation does not substitute, it is opened once the goal states the
read (`readEntryValues`). -/
private def openEntryValues (goal : MVarId) (equation : Name) : TacticM MVarId := do
  let saved ← getGoals
  let opened ← try
      setGoals [goal]
      evalTactic (← `(tactic| simp only [VerexIR.Proofs.Denote.HList.settle, ↓settleDeclared,
        VerexIR.Proofs.Denote.NTy.settle, VerexIR.Proofs.Denote.HList.finals,
        VerexIR.Proofs.Denote.NTy.forget_eq_forget,
        VerexIR.Proofs.Denote.Carriers.lent_of_eq_of,
        Prod.mk.injEq, Prod.fst, Prod.snd, and_true, true_and] at $(mkIdent equation):ident))
      let goals ← getGoals
      setGoals saved
      pure (goals.headD goal)
    catch error =>
      setGoals saved
      if verex.solverDebug.get (← getOptions) then
        IO.println s!"entry values not rewritten: {← error.toMessageData.toString}"
      -- A row whose type is spelled at the plain layer or at another family
      -- is no product to the rewriting; its pairs are constructors all the same.
      try
        goal.withContext do
          let some decl := (← getLCtx).findFromUserName? equation | pure goal
          injectRowEquation goal decl.fvarId equation
      catch _ => pure goal
  -- The components substituted, not carried: the row a clause spells
  -- replaces the row the opening named, so that every later reading — the
  -- deciders' arithmetic among them — sees one row and not two.
  let mut current := opened
  for _ in [0:12] do
    let found ← current.withContext do
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        unless decl.userName == equation do continue
        let type ← instantiateMVars decl.type
        if type.isAppOfArity ``And 2 then return some (decl.fvarId, true)
        if type.eq?.isSome then return some (decl.fvarId, false)
      return none
    let some (component, conjunction) := found | break
    if conjunction then
      let parts ← current.cases component (givenNames := #[⟨false, [equation, equation]⟩])
      let some part := parts[0]? | break
      current := part.mvarId
    else
      -- An equation of two pairs the normalization left whole — a row at a
      -- type it does not reduce under the instances it unifies at — split
      -- into its components by projection (`rowComponentEquations`).
      let split ← current.withContext do
        let decl ← component.getDecl
        let some (_, lhs, rhs) := (← instantiateMVars decl.type).eq? | return none
        unless lhs.consumeMData.isAppOfArity ``Prod.mk 4 &&
            rhs.consumeMData.isAppOfArity ``Prod.mk 4 do
          return none
        let components ← rowComponentEquations (.fvar component)
        if components.isEmpty then return some (← current.clear component)
        let mut goal := current
        let mut stated : Array Lean.Expr := #[]
        for proof in components do
          -- A component stated twice (a mutable reference's current, which
          -- settling reads twice) is one equation.
          let statement ← instantiateMVars (← inferType proof)
          if stated.contains statement then continue
          stated := stated.push statement
          let (_, next) ← (← goal.assert equation statement proof).intro1P
          goal := next
        return some (← goal.clear component)
      if let some next := split then
        current := next
        continue
      -- A certified argument equated with a lent value: the argument as
      -- that value certified (`certifiedComponent?`).
      let (current', component) ← match ← (try
            certifiedComponent? current component equation
          catch _ => pure none) with
        | some (goal, equal) => pure (goal, equal)
        | none => pure (current, component)
      current := current'
      current ← try
          Lean.Meta.substCore current component (symm := true) <&> (·.2)
        catch _ => try
          Lean.Meta.substCore current component (symm := false) <&> (·.2)
        catch _ => current.withContext do
          current.rename component (equation.appendAfter "Kept")
  return current

/-- Marks a statement over entry values that an opening has taken on this
path (`openAtEntries`).  A hypothesis rather than a ledger of one
invocation, because the walk reaches a leaf many times along one path while
the row an opening names, its equation and its fact are the path's: the
record is what tells a statement arriving again from one not yet opened,
where the equation that would say so has substituted away.  Substitution
rewrites it with the statement it names, so a statement the path respells
is recognized as the one that was opened. -/
def EntriesOpened (_statement : Prop) : Prop := True

theorem EntriesOpened.intro (statement : Prop) : EntriesOpened statement := trivial

/-- Marks a behavior of a function value that the path has read through its
verified target (`dispatchBehavior`).  A hypothesis, as an opening's record is
(`EntriesOpened`): a substitution respells the behavior and its record
together, so the behavior is not read through its target again. -/
def BehaviorDispatched (_statement : Prop) : Prop := True

theorem BehaviorDispatched.intro (statement : Prop) : BehaviorDispatched statement := trivial

/-- A hypothesis stating a property of an arguments row at their entry
values (`AtEntries`), as every rule over a run reads it: the row it names,
that the row passes those values, and the property of that row.  It is
opened by elimination, so the row is a local of the leaf and no hypothesis
carries a choice of it.  An opening is taken once on a path
(`EntriesOpened`): opening a statement again names a second row of the same
values, which respells every argument the leaf reads and leaves the walk
taking the invocation it states a second time. -/
private def openAtEntries (goal : MVarId) : TacticM MVarId := do
  -- Whether a hypothesis states the same property at a row of the leaf's
  -- own, as the rule at an invocation states it beside the clause form:
  -- that row is the one every rule reads, and naming another row of the
  -- same entry values states nothing more.
  let readsHoldsAt (view : EntriesView) : MetaM Bool := do
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let type ← instantiateMVars decl.type
      unless type.isAppOfArity ``VerexIR.Proofs.AbortsOf 10 do continue
      let args := type.getAppArgs
      let stated := mkAppN (mkConst ``VerexIR.Proofs.AbortsOfEntries)
        (args.extract 0 8
          |>.push (settledRow args[0]! args[1]! args[2]! args[4]! args[8]!)
          |>.push args[9]!)
      let some other ← entriesView? stated | continue
      if ← withNewMCtxDepth do
          pure ((← isDefEq other.holds view.holds) && (← isDefEq other.entries view.entries)) then
        return true
    return false
  -- Whether the context states that the property does not hold at these
  -- entry values: the statement then refutes the leaf as it stands, and
  -- naming a row of it says nothing the refutation needs.
  let excluded (type : Lean.Expr) : MetaM Bool := do
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some negated := (unmarked (← instantiateMVars decl.type)).not? | continue
      if ← withNewMCtxDepth (isDefEq negated type) then return true
    return false
  -- Whether a statement at these entry values has already been opened at
  -- this leaf: the equation an opening leaves states that a row of the
  -- leaf passes them.
  let openedAt (view : EntriesView) : MetaM Bool := do
    -- An opening leaves an equation of the row it names with these entry
    -- values; two statements of the same function value differ only in
    -- those values, so the equation and not the property identifies one.
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let some (_, _, rhs) := (← instantiateMVars decl.type).eq? | continue
      if ← withNewMCtxDepth (isDefEq rhs view.entries) then return true
    return false
  -- Whether an opening on this path has already taken this statement
  -- (`EntriesOpened`).  The record names the statement and not its entry
  -- values, because a leaf reads two statements of one invocation — what it
  -- ensures and what it aborts of — at the same values, and each is opened
  -- at a row of its own.
  let openedOnPath (type : Lean.Expr) : MetaM Bool := do
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let recorded ← instantiateMVars decl.type
      unless recorded.isAppOfArity ``EntriesOpened 1 do continue
      if recorded.appArg!.consumeMData == type.consumeMData then return true
    return false
  let mut current := goal
  let mut opened : Array Lean.Expr := #[]
  for round in [0:8] do
    -- The names of this opening: a leaf may read two statements over entry
    -- values, and each names its own row and its own equation.
    let rowName := if round == 0 then `verexArgs else (`verexArgs).appendIndexAfter round
    let equationName := if round == 0 then `verexEntryValues
      else (`verexEntryValues).appendIndexAfter round
    let finalsName := if round == 0 then `verexEntryFinals
      else (`verexEntryFinals).appendIndexAfter round
    let found ← current.withContext do
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let type ← instantiateMVars decl.type
        let some view ← entriesView? type | continue
        if opened.contains type then continue
        if ← openedOnPath type then continue
        if ← openedAt view then continue
        if ← readsHoldsAt view then continue
        if ← excluded type then continue
        return some (decl.fvarId, type, view)
      return none
    let some (hypothesis, type, view) := found | break
    opened := opened.push type
    let next ← current.withContext do
      try
        -- The existential the predicate is defined as, which `cases` reads.
        let marked := type.isAppOfArity ``VerexIR.Proofs.Obligation 4
        let statement := unmarked type
        let stated ← whnfD statement
        unless stated.isAppOfArity ``Exists 2 do return current
        -- The opening recorded on the path before it is taken, so that the
        -- substitutions it makes rewrite the record with the statement it
        -- names (`EntriesOpened`).
        let (_, started) ← (← current.assert `verexEntriesOpened
          (mkApp (mkConst ``EntriesOpened) type)
          (mkApp (mkConst ``EntriesOpened.intro) type)).intro1P
        -- The statement stays a fact of the leaf and its existential is
        -- opened beside it: a goal stating the same behavior at the same
        -- entry values — an `aborts_if` naming the abort a call stated — is
        -- then the fact itself, in the one spelling the clause and the rule
        -- share, rather than a statement only the opened row could rebuild.
        -- A clause's statement keeps its marker, which names the clause in
        -- a diagnostic.
        let fact := if marked then
            mkApp4 (mkConst ``Iff.mp) type statement
              (mkAppN (mkConst ``VerexIR.Proofs.Obligation_iff) type.getAppArgs) (.fvar hypothesis)
          else .fvar hypothesis
        let (opening, goal) ← (← started.assert `verexAtEntries stated fact).intro1P
        let opened ← goal.cases opening
          (givenNames := #[⟨false, [rowName, `verexAtEntries]⟩])
        let some subgoal := opened[0]? | return current
        let some conjunction := subgoal.fields[1]? | return current
        let parts ← subgoal.mvarId.cases conjunction.fvarId!
          (givenNames := #[⟨false, [equationName, `verexHolds]⟩])
        let some part := parts[0]? | return current
        let mut opened := part.mvarId
        -- A statement that names the finals of the row's mutable references
        -- leaves a second equation beside the entry values'
        -- (`AtEntryFinals`): the currents come from the settling, the
        -- prophecies from the finals.
        let mut finals? : Option Name := none
        if view.finals?.isSome then
          let split ← opened.withContext do
            let some decl := (← getLCtx).findFromUserName? `verexHolds | return none
            unless (← instantiateMVars decl.type).isAppOfArity ``And 2 do return none
            let parts ← opened.cases decl.fvarId
              (givenNames := #[⟨false, [finalsName, `verexHolds]⟩])
            return parts[0]?
          let some next := split | return current
          opened := next.mvarId
          finals? := some finalsName
        -- The row opened into its components, so that each component of the
        -- entry-value equation has a local on one side and replaces the
        -- value the clause spells (`openEntryValues`).
        -- A mutable reference's entry is opened as well, into its current and
        -- its prophecy, which the two equations determine one each.
        let mut row ← opened.withContext do
          return ((← getLCtx).findFromUserName? rowName).map (·.toExpr)
        for _ in [0:8] do
          let some current := row | break
          let some fvar := current.fvarId? | break
          let split ← opened.withContext do
            let type ← whnfD (← fvar.getType)
            unless type.isAppOfArity ``Prod 2 do return none
            let parts ← opened.cases fvar (givenNames := #[⟨false, [rowName, rowName]⟩])
            return parts[0]?
          let some next := split | break
          opened := next.mvarId
          row := next.fields[1]?
          let some entry := next.fields[0]?.bind (·.fvarId?) | continue
          let pair ← opened.withContext do
            let type ← whnfD (← entry.getType)
            unless type.isAppOfArity ``Prod 2 do return none
            let parts ← opened.cases entry (givenNames := #[⟨false, [rowName, rowName]⟩])
            return parts[0]?
          if let some pair := pair then
            opened := pair.mvarId
            row := row.map pair.subst.apply
        if verex.solverDebug.get (← getOptions) then
          logInfo m!"clause entries opened at the row {rowName} names"
        let withEntries ← openEntryValues opened equationName
        match finals? with
        | some name => openEntryValues withEntries name
        | none => pure withEntries
      catch error =>
        if verex.solverDebug.get (← getOptions) then
          IO.println s!"clause entries not opened: {← error.toMessageData.toString}"
        pure current
    current := next
  return current

/-- A row of values at a spelled row, one metavariable per entry: the row an
entry-value or a finals equation is solved for.  Spelled as a product per
entry, because a single metavariable of the row's type stands under the
projections the settling takes of it (`Prod.fst`), which the unifier does
not solve; its components do.  Each product is spelled at the entry's
carrier and the rest of the row, the spelling a row of values has
elsewhere, so that a goal read at this row and a hypothesis stating one are
the same proposition reducibly (`verex_assumption`). -/
private partial def rowMetavariables (carriers row : Lean.Expr) :
    MetaM (Option Lean.Expr) := do
  let row := (← whnfR row).consumeMData
  if row.isConstOf ``NRow.nil then return some (mkConst ``Unit.unit)
  unless row.isAppOfArity ``NRow.cons 2 do return none
  let (τ, rest) := (row.getArg! 0, row.getArg! 1)
  -- A mutable reference's carrier is its plain current and its certified
  -- prophecy, which the two equations determine one each: the entry is
  -- spelled as that pair, or the settling would stand under a projection
  -- of it (`NTy.carrier`, `doc/designs/data-invariants.md`).  The
  -- current is spelled in the canonical spelling of the plain layer
  -- (`PlainSpelling`).
  let spelled := (← whnfR τ).consumeMData
  let head ← if spelled.isAppOfArity ``NTy.ref 1 then
      let σ := spelled.appArg!
      let referent := mkApp2 (mkConst ``NTy.plainOf)
        (mkApp (mkConst ``Carriers.toPlain) carriers) σ
      let prophecy := mkApp2 (mkConst ``NTy.carrier) carriers σ
      mkAppOptM ``Prod.mk #[referent, prophecy, ← mkFreshExprMVar referent,
        ← mkFreshExprMVar prophecy]
    else mkFreshExprMVar (mkApp2 (mkConst ``NTy.carrier) carriers τ)
  let some tail ← rowMetavariables carriers rest | return none
  return some (← mkAppOptM ``Prod.mk #[mkApp2 (mkConst ``NTy.carrier) carriers τ,
    mkApp2 (mkConst ``HList) carriers rest, head, tail])

/-- The row a clause's entry values name where no row of the clause's own
forgets to them: the values themselves, certified at the hand-over to the
invocation.

A value a specification builds is plain, its data invariants owed by the cut
at the position that holds it (`nativeConstructed`).  The position that
holds the entry values of an invocation is the hand-over to it, where the
caller owes the invariants of what it hands over, as it owes them of a
`&mut` argument's referent.  So the certificate this row carries is that
hand-over's obligation, read where it is owed: the leaf proves it, and where
it cannot the row is not read and the residual reports the violation at the
hand-over.

Reference-free rows only: a reference's settling reads its current twice, so
no certified row built from settled values is the row the clause named. -/
private partial def certifiedEntries? (carriers row entries : Lean.Expr) :
    TacticM (Option Lean.Expr) := do
  let free := mkApp (mkConst ``NRow.refFree) row
  unless (← withDefault (whnf free)).isConstOf ``Bool.true do return none
  -- Spelled per entry, as the hand-over of a call spells its certified
  -- arguments (`Term.certifying`, `NTy.ofDeepHolds` of each value): a
  -- leaf's facts about the call and the statement read at this row are
  -- then the same proposition, certificates aside.
  let rec spell (row entries : Lean.Expr) : TacticM (Option Lean.Expr) := do
    let row := (← whnfR row).consumeMData
    if row.isConstOf ``NRow.nil then return some (mkConst ``Unit.unit)
    unless row.isAppOfArity ``NRow.cons 2 do return none
    let (τ, rest) := (row.getArg! 0, row.getArg! 1)
    let entries := entries.consumeMData
    unless entries.isAppOfArity ``Prod.mk 4 do return none
    let (value, others) := (entries.getArg! 2, entries.getArg! 3)
    let holds := mkApp3 (mkConst ``NTy.deepHolds) carriers τ value
    let certificate ← mkFreshExprSyntheticOpaqueMVar holds
    unless ← closesBy certificate.mvarId!
        (← `(tactic| ((try simp only [vir_denote_norm]) <;> verex_leaf))) do
      return none
    let head := mkApp4 (mkConst ``NTy.ofDeepHolds) carriers τ value
      (← instantiateMVars certificate)
    let some tail ← spell rest others | return none
    return some (← mkAppOptM ``Prod.mk #[mkApp2 (mkConst ``NTy.carrier) carriers τ,
      mkApp2 (mkConst ``HList) carriers rest, head, tail])
  spell row entries

/-- A proposition with every statement over entry values in a positive
position read at the row those values name, and the implication from it
(`atEntries_of_holds`, `atEntryFinals_of_holds`): a clause owes its
`aborts_of` of the row its own values name, and a disjunction of such
statements — the abort conditions of a clause — one row each.  A statement
that names the finals of the row's mutable references names the whole row:
settling gives the currents and the finals the prophecies, so both
equations are solved for one row (`AtEntryFinals`).  Where no row forgets to
the values — a value a specification builds is plain and names no certified
row — the row is the values certified (`certifiedEntries?`).  `none` where
it reads no statement so. -/
private partial def strengthenEntries (statement : Lean.Expr) :
    TacticM (Option (Lean.Expr × Lean.Expr)) := do
  let identity (proposition : Lean.Expr) : MetaM Lean.Expr :=
    withLocalDeclD `held proposition fun held => mkLambdaFVars #[held] held
  -- A clause's marker on an alternative, as on the whole goal: the
  -- statement under it read, and the marker kept, so the alternative is
  -- still reported at its own source.
  if statement.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
    let some (inner, implication) ← strengthenEntries statement.appArg! | return none
    let markerArgs := statement.getAppArgs.pop
    let innerMarked := mkAppN (mkConst ``VerexIR.Proofs.Obligation) (markerArgs.push inner)
    let rule ← withLocalDeclD `held innerMarked fun held => do
      let opened := mkApp4 (mkConst ``Iff.mp) innerMarked inner
        (mkAppN (mkConst ``VerexIR.Proofs.Obligation_iff) (markerArgs.push inner)) held
      let closed := mkApp4 (mkConst ``Iff.mpr) statement statement.appArg!
        (mkAppN (mkConst ``VerexIR.Proofs.Obligation_iff) statement.getAppArgs)
        (← Core.betaReduce (mkApp implication opened))
      mkLambdaFVars #[held] closed
    return some (innerMarked, rule)
  if let some view ← entriesView? statement then
    -- The row the entry values are the settling of, which the clause built
    -- them from; its own metavariable, so that the goal's are untouched.
    let read? ← withNewMCtxDepth do
      -- A statement naming finals determines the row entry by entry: the
      -- currents from the settling, the prophecies from the finals, so the
      -- row is spelled per entry (`rowMetavariables`).  A statement naming
      -- entry values alone leaves a mutable reference's prophecy free and
      -- so determines no row; it is read only where one metavariable of the
      -- row's type is solved, and the leaf keeps the statement otherwise.
      let args ← match ← (if view.finals?.isSome then rowMetavariables view.carriers view.row
          else pure none) with
        | some spelled => pure spelled
        | none => mkFreshExprMVar (mkApp2 (mkConst ``HList) view.carriers view.row)
      unless ← isDefEq (mkApp3 (mkConst ``HList.settle) view.carriers view.row args)
          view.entries do
        return none
      -- The prophecies the entry values do not name, which the finals do.
      if let some finals := view.finals? then
        unless ← isDefEq (mkApp3 (mkConst ``HList.finals) view.carriers view.row args)
            finals do
          return none
      let args ← instantiateMVars args
      if args.hasExprMVar then return none
      return some args
    -- The values a specification builds are plain and name no certified
    -- row, so unification finds none; the row is then the values certified
    -- at the hand-over (`certifiedEntries?`), whose settling is the values
    -- again, as it is of any reference-free row.
    let args? ← match read? with
      | some args => pure (some args)
      | none =>
          match ← certifiedEntries? view.carriers view.row view.entries with
          | none => pure none
          | some args => withDefault do
              let settling := mkApp3 (mkConst ``HList.settle) view.carriers view.row args
              let finals := mkApp3 (mkConst ``HList.finals) view.carriers view.row args
              let settled ← isDefEq settling view.entries
              let finalled ← match view.finals? with
                | none => pure true
                | some named => isDefEq finals named
              pure (if settled && finalled then some args else none)
    let some args := args? | return none
    let inner ← Core.betaReduce (mkApp view.holds args)
    let rule := if view.finals?.isSome then ``atEntryFinals_of_holds else ``atEntries_of_holds
    let implication ← withLocalDeclD `held inner fun held =>
      mkLambdaFVars #[held] (mkAppN (mkConst rule)
        #[view.carriers, view.row, view.holds, args, held])
    return some (inner, implication)
  for (connective, rule) in [(``Or, ``Or.imp), (``And, ``And.imp)] do
    unless statement.isAppOfArity connective 2 do continue
    let (left, right) := (statement.getArg! 0, statement.getArg! 1)
    let leftRead ← strengthenEntries left
    let rightRead ← strengthenEntries right
    if leftRead.isNone && rightRead.isNone then return none
    let (leftInner, leftImplication) ← match leftRead with
      | some read => pure read
      | none => pure (left, ← identity left)
    let (rightInner, rightImplication) ← match rightRead with
      | some read => pure read
      | none => pure (right, ← identity right)
    -- The implicit propositions of `Or.imp` and `And.imp` are bound in the
    -- order their implications read them (`a`, `c`, `b`, `d`), so the rule
    -- is applied to the two implications and its propositions inferred.
    return some (mkApp2 (mkConst connective) leftInner rightInner,
      ← mkAppM rule #[leftImplication, rightImplication])
  return none

/-- A goal stating a property of an arguments row at their entry values
(`AtEntries`), also under a disjunction of a clause's abort conditions: the
property of the row the entry values name, where they name one — a clause
builds them of the values it reads, so the row is the one their settling is
of (`strengthenEntries`).  The obligation marker is kept, so a clause that
does not hold is reported at its own source. -/
private def closeAtEntries (goal : MVarId) : TacticM MVarId := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let marked := target.isAppOfArity ``VerexIR.Proofs.Obligation 4
  let statement := unmarked target
  -- Nothing to read where the goal states no entry values: the walk of its
  -- connectives is spent only on a goal that holds one.
  unless (statement.find? fun e =>
      e.isAppOf ``AtEntries || e.isAppOf ``VerexIR.Proofs.AbortsOfEntries ||
        e.isAppOf ``AtEntryFinals || e.isAppOf ``VerexIR.Proofs.EnsuresOfEntries).isSome do
    return goal
  let result ← goal.withContext do
    let some (inner, implication) ← strengthenEntries statement | return none
    -- A statement a hypothesis already makes is left to the deciders: the
    -- rule at an invocation states the clause form of its abort beside the
    -- run's own, and reading it at another row states nothing more.
    if ← (← getLCtx).anyM fun decl => do
        if decl.isImplementationDetail then return false
        withNewMCtxDepth (isDefEq (unmarked (← instantiateMVars decl.type)) statement) then
      return none
    let innerType := if marked then
        mkAppN (mkConst ``VerexIR.Proofs.Obligation) (target.getAppArgs.pop.push inner)
      else inner
    let next ← mkFreshExprSyntheticOpaqueMVar innerType (← goal.getTag)
    let held := if marked then
        mkApp4 (mkConst ``Iff.mp) innerType inner
          (mkAppN (mkConst ``VerexIR.Proofs.Obligation_iff)
            (target.getAppArgs.pop.push inner)) next
      else next
    let proof := mkApp implication held
    let proof := if marked then
        mkApp4 (mkConst ``Iff.mpr) target statement
          (mkAppN (mkConst ``VerexIR.Proofs.Obligation_iff) target.getAppArgs) proof
      else proof
    unless ← isDefEq (← inferType proof) target do return none
    goal.assign proof
    return some next.mvarId!
  match result with
  | some next => return next
  | none => return goal

/-- A hypothesis denying a statement over entry values, also a disjunction
of a contract's abort conditions one of which is (`¬(c ∨ AtEntries …)`, as
the caller of a function whose `aborts_if` reads a known target's
`aborts_of` holds it where the call returns): the denial of the property at
the row the entry values name, by the same reading a goal stating it takes
(`strengthenEntries`), contraposed.  The hypothesis is replaced, so the leaf
reads one statement of the denial. -/
private def weakenDeniedEntries (goal : MVarId) : TacticM MVarId := do
  let mut current := goal
  for _ in [0:4] do
    let found ← current.withContext do
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let type ← instantiateMVars decl.type
        let some denied := (unmarked type).not? | continue
        unless (denied.find? fun e => e.isAppOf ``AtEntries).isSome do continue
        let some (inner, implication) ← strengthenEntries denied | continue
        let proof ← withLocalDeclD `held inner fun held => do
          let fact := if type.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
              mkApp4 (mkConst ``Iff.mp) type (unmarked type)
                (mkAppN (mkConst ``VerexIR.Proofs.Obligation_iff) type.getAppArgs) decl.toExpr
            else decl.toExpr
          mkLambdaFVars #[held] (mkApp fact (← Core.betaReduce (mkApp implication held)))
        let stated := mkNot inner
        unless ← isDefEq (← inferType proof) stated do continue
        return some (decl.fvarId, decl.userName, stated, proof)
      return none
    let some (hypothesis, name, stated, proof) := found | break
    let (_, next) ← (← current.assert name stated proof).intro1P
    current ← (try next.clear hypothesis catch _ => pure next)
  return current

/-- A hypothesis stating a run that returns a mutable reference, whose
prophecy the statement quantifies (`EnsuresOfSettled`): opened by
elimination into the prophecy, the current it forgets, and the run itself,
which the rule over entry values opens in turn (`openAtEntries`). -/
private def openSettled (goal : MVarId) : TacticM MVarId := do
  let mut current := goal
  for round in [0:4] do
    let prophecyName := if round == 0 then `verexProphecy
      else (`verexProphecy).appendIndexAfter round
    let currentName := if round == 0 then `verexSettled
      else (`verexSettled).appendIndexAfter round
    let found ← current.withContext do
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let type ← instantiateMVars decl.type
        unless (unmarked type.consumeMData).isAppOf ``VerexIR.Proofs.EnsuresOfSettled do continue
        return some (decl.fvarId, type)
      return none
    let some (hypothesis, type) := found | break
    let next ← current.withContext do
      try
        -- A clause's statement keeps its marker, which names the clause in
        -- a diagnostic; the run read out of it is opened beside it.
        let marked := type.isAppOfArity ``VerexIR.Proofs.Obligation 4
        let statement := unmarked type
        let stated ← whnfD statement
        unless stated.isAppOfArity ``Exists 2 do return current
        let (opening, goal) ← if marked then
            let fact := mkApp4 (mkConst ``Iff.mp) type statement
              (mkAppN (mkConst ``VerexIR.Proofs.Obligation_iff) type.getAppArgs) (.fvar hypothesis)
            (← current.assert `verexSettledRun stated fact).intro1P
          else
            pure (hypothesis, ← current.replaceLocalDeclDefEq hypothesis stated)
        let opened ← goal.cases opening
          (givenNames := #[⟨false, [prophecyName, `verexSettledRun]⟩])
        let some subgoal := opened[0]? | return current
        let some conjunction := subgoal.fields[1]? | return current
        let parts ← subgoal.mvarId.cases conjunction.fvarId!
          (givenNames := #[⟨false, [currentName, `verexSettledRun]⟩])
        let some part := parts[0]? | return current
        return part.mvarId
      catch error =>
        if verex.solverDebug.get (← getOptions) then
          IO.println s!"settled run not opened: {← error.toMessageData.toString}"
        pure current
    if next == current then break
    current := next
  return current

/-- The prophecy a leaf's call returned at a type: the second component of
the result of a run returning a mutable reference (`EnsuresOf`,
`CallEnsures`).  A clause names such a result by its current value alone,
so the prophecy a statement of it quantifies is the one the call
returned. -/
private def returnedProphecy? (carriers σ : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let wanted := mkApp2 (mkConst ``VerexIR.Proofs.Denote.NTy.carrier) carriers σ
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    let type := unmarked (← instantiateMVars decl.type).consumeMData
    let result? :=
      if type.isAppOfArity ``VerexIR.Proofs.EnsuresOf 12 then some (type.getArg! 9)
      else if type.isAppOfArity ``VerexIR.Proofs.CallEnsures 7 then some (type.getArg! 4)
      else none
    let some result := result? | continue
    let result := result.consumeMData
    unless result.isAppOfArity ``Prod.mk 4 do continue
    let prophecy := result.getArg! 3
    unless ← withNewMCtxDepth (isDefEq (← inferType prophecy) wanted) do continue
    return some prophecy
  return none

/-- A goal stating a run that returns a mutable reference, whose prophecy
the statement quantifies (`EnsuresOfSettled`, and a predicate of a known
target read directly, which states the quantifier itself): the witness is
the prophecy the leaf's call returned (`returnedProphecy?`), leaving the
current it forgets and the run at it as one goal, which the rule over entry
values then reads (`closeAtEntries`).  The obligation marker is kept, so a
clause that does not hold is reported at its own source. -/
private def closeSettled (goal : MVarId) : TacticM MVarId := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let marked := target.isAppOfArity ``VerexIR.Proofs.Obligation 4
  let statement := unmarked target
  unless statement.isAppOf ``VerexIR.Proofs.EnsuresOfSettled ||
      statement.isAppOfArity ``Exists 2 do
    return goal
  let result ← goal.withContext do
    let stated ← whnfD statement
    unless stated.isAppOfArity ``Exists 2 do return none
    let domain := stated.getArg! 0
    let body := stated.getArg! 1
    -- The binder's type as the statement spells it: the carrier of a struct
    -- unfolds to its subtype at reducible transparency, which names no type.
    let_expr VerexIR.Proofs.Denote.NTy.carrier carriers σ := domain.consumeMData
      | return none
    -- The statement's own shape: the current the prophecy forgets, and the
    -- run at it.  Nothing else quantified over a carrier is witnessed here.
    let shaped ← withLocalDeclD `prophecy domain fun prophecy => do
      let inner ← Core.betaReduce (mkApp body prophecy)
      unless inner.isAppOfArity ``And 2 do return false
      let some (_, lhs, _) := (inner.getArg! 0).eq? | return false
      return (lhs.consumeMData).isAppOfArity ``VerexIR.Proofs.Denote.NTy.forget 3
    unless shaped do return none
    let some prophecy ← returnedProphecy? carriers σ | return none
    let inner ← Core.betaReduce (mkApp body prophecy)
    let innerType := if marked then
        mkAppN (mkConst ``VerexIR.Proofs.Obligation) (target.getAppArgs.pop.push inner)
      else inner
    let next ← mkFreshExprSyntheticOpaqueMVar innerType (← goal.getTag)
    let held := if marked then
        mkApp4 (mkConst ``Iff.mp) innerType inner
          (mkAppN (mkConst ``VerexIR.Proofs.Obligation_iff)
            (target.getAppArgs.pop.push inner)) next
      else next
    let proof ← mkAppOptM ``Exists.intro #[some domain, some body, some prophecy, some held]
    let proof := if marked then
        mkApp4 (mkConst ``Iff.mpr) target statement
          (mkAppN (mkConst ``VerexIR.Proofs.Obligation_iff) target.getAppArgs) proof
      else proof
    unless ← isDefEq (← inferType proof) target do return none
    goal.assign proof
    return some next.mvarId!
  match result with
  | some next => return next
  | none => return goal

/-- A goal's behavioral predicates as hypotheses (`introducedBehaviors`),
the clause forms of `ensures_of` and `aborts_of` among them opened
(`openSettled`, `openAtEntries`), and a goal stating one witnessed by the
prophecy its call returned (`closeSettled`) and read at the row its entry
values name (`closeAtEntries`). -/
def introduceBehaviors (goal : MVarId) : TacticM MVarId :=
  introducedBehaviors goal >>= openSettled >>= openAtEntries >>= weakenDeniedEntries >>=
    closeSettled >>= closeAtEntries

/-- Whether an alternative is an abort of a literal closure or of a known
target read directly (`CallAborts`, and its clause form at entry values,
`AtEntries`, whose property is the call's abort or the contract's). -/
private partial def literalAbortAlternative (environment : Environment) (statement : Lean.Expr) :
    Bool :=
  let statement := unmarked statement
  if statement.isAppOfArity ``Or 2 then
    literalAbortAlternative environment (statement.getArg! 0) ||
      literalAbortAlternative environment (statement.getArg! 1)
  else
    (statement.isAppOfArity ``VerexIR.Proofs.AbortsOf 10 && statesLiteralBehavior environment statement) ||
      (statement.isAppOfArity ``VerexIR.Proofs.AbortsOfEntries 10 &&
        (literalFunction? environment (statement.getArg! 7)).isSome) ||
      statement.isAppOfArity ``AtEntries 4 ||
      statement.isAppOfArity ``VerexIR.Proofs.CallAborts 5

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

/-- Reference-freedom of a row or type, decided by the kernel: none where
the statement is not frame-free or does not evaluate (`frameFreeDecision`),
so that no rule is applied with a certificate the kernel refuses. -/
private def decidedTrue? (statement : Lean.Expr) : MetaM (Option Lean.Expr) := do
  unless frameFreeDecision statement do return none
  unless (← withDefault (whnf statement)).isConstOf ``Bool.true do return none
  return some (← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
    (← mkEq statement (mkConst ``Bool.true)))

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

/-- Instantiate a generic closure target's theorem at its actual frame,
including captured evidence, before reading its postcondition. -/
private def genericBehavior? (view : BehaviorView) (stated : Lean.Expr)
    (callees : Array (Lean.Expr × String × Lean.Expr)) :
    TacticM (Option (Lean.Expr × Array Lean.Expr × Lean.Expr)) := do
  let function := view.function.consumeMData
  unless function.isAppOfArity ``functionOfGeneric 15 && view.run?.isSome do return none
  let θ := function.getArg! 5
  let weave := function.getArg! 9
  let shape := function.getArg! 10
  let shared := function.getArg! 11
  let handle := function.getArg! 12
  let some (_, _, theoremProof) ← callees.findM? fun (candidate, _, proof) => do
      unless ← isDefEq candidate handle do return false
      forallTelescope (← inferType proof) fun _ statement =>
        pure (statement.isAppOfArity ``VerexIR.Proofs.Satisfies 6)
    | return none
  let (theoremArguments, _, statement) ← forallMetaTelescope (← inferType theoremProof)
  let meaning := statement.getArg! 4
  unless meaning.isAppOf ``propheticMeaning do return none
  let frame ← mkAppM ``Skolems.instantiate #[θ, function.getArg! 2]
  unless ← isDefEq (meaning.getArg! 3) frame do return none
  let verified ← instantiateMVars (mkAppN theoremProof theoremArguments)
  let some free ← decidedTrue? (mkApp (mkConst ``NRow.refFree) (← mkAppM ``Subtype.val #[θ]))
    | return none
  let some capturedFree ← decidedTrue? (mkApp (mkConst ``NRow.refFree) (function.getArg! 7))
    | return none
  let instantiation ← mkAppM ``frameInstantiation #[function.getArg! 0, handle,
    function.getArg! 3, function.getArg! 4]
  let coherentType ← mkAppOptM ``Coherent #[function.getArg! 0, function.getArg! 1,
    frame, handle, instantiation]
  let some coherent ← coherentProof? coherentType | return none
  let some executable := executableOf? view.invoke | return none
  for (actual, expected, label) in #[(meaning.getArg! 2, executable, "executable"),
      (meaning.getArg! 3, frame, "frame"), (meaning.getArg! 4, instantiation, "instantiation"),
      (meaning.getArg! 6, function.getArg! 6, "parameters"), (meaning.getArg! 7, shape, "result")] do
    unless ← withTransparency .all <| isDefEq actual expected do
      if verex.solverDebug.get (← getOptions) then
        logInfo m!"generic behavior {label} mismatch: {actual} versus {expected}"
      return none
  let verified ← instantiateMVars verified
  let implication ← mkAppAtFrame ``VerexIR.Proofs.ensuresOf_generic_verified
    (function.getArg! 0) executable (function.getArg! 2) #[θ, free, weave,
    capturedFree, shape, handle, function.getArg! 3, function.getArg! 4, coherent,
    function.getArg! 13, shared, function.getArg! 14, view.arguments, verified, stated]
  return some (theoremProof, theoremArguments, implication)

private def literalBehavior? (view : BehaviorView) (stated type executable : Lean.Expr)
    (callees : Array (Lean.Expr × String × Lean.Expr)) :
    TacticM (Option (Lean.Expr × Array Lean.Expr × Lean.Expr)) := do
  let some found := literalFunction? (← getEnv) view.function | return none
  let some closure ← literalValue? found | return none
  let some (weave, captured, _) ← closureWeave? closure | return none
  let handle := closure.handle
  -- A target the call rule inlines is listed by its theorem as well.
  let some (_, _, theoremProof) ← callees.findM? fun (candidate, _, proof) => do
      unless ← isDefEq candidate handle do return false
      forallTelescope (← inferType proof) fun _ statement =>
        pure (statement.isAppOfArity ``VerexIR.Proofs.Satisfies 6)
    | return none
  let (theoremArguments, _, statement) ← forallMetaTelescope (← inferType theoremProof)
  unless statement.isAppOfArity ``VerexIR.Proofs.Satisfies 6 do return none
  let meaning := statement.getArg! 4
  unless meaning.isAppOf ``propheticMeaning do return none
  -- Fix the target theorem's family before unifying its dependent contract.
  -- Scalar argument carriers alone do not determine this implicit family.
  unless ← isDefEq (meaning.getArg! 3) closure.frame do return none
  let verified ← instantiateMVars (mkAppN theoremProof theoremArguments)
  -- A closure of a target without type arguments runs at the empty
  -- instantiation, which the runtime frame is coherent with.
  let coherent ← mkAppM ``coherent_runtime #[type.getArg! 0, handle]
  let some capturedFree ← decidedTrue? (mkApp (mkConst ``NRow.refFree) captured)
    | return none
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
  let some implication := implication? | return none
  return some (theoremProof, theoremArguments, implication)

/-- Whether a statement holds `ensures_of` at entry values at the frame a
call's type arguments induce. -/
private def entriesAtInducedFrame (statement : Lean.Expr) : Bool :=
  (statement.find? fun term =>
    term.isAppOf ``VerexIR.Proofs.EnsuresOfEntries && term.getAppNumArgs > 2 &&
      (term.getArg! 2).consumeMData.isAppOf ``Skolems.instantiate).isSome

/-- Whether a statement holds `ensures_of` or `aborts_of` at the frame a
call's type arguments induce. -/
private def statedAtInducedFrame (statement : Lean.Expr) : Bool :=
  (statement.find? fun term =>
    (term.isAppOf ``VerexIR.Proofs.EnsuresOf || term.isAppOf ``VerexIR.Proofs.AbortsOf) &&
      term.getAppNumArgs > 2 &&
      (term.getArg! 2).consumeMData.isAppOf ``Skolems.instantiate).isSome

/-- What a leaf learns from `ensures_of` and `aborts_of` of a closure whose
target, weave, and captures it sees, and whose target is a verified
callee: where the leaf establishes the target's precondition at the woven
arguments, what the target's contract ensures of the run, or the failures it
permits (`ensuresOf_closureOf_verified`, `abortsOf_closureOf_verified`).
`none` where the requires meaning closes the goal. -/
partial def dispatchBehavior (goal : MVarId) (callees : Array (Lean.Expr × String × Lean.Expr))
    (handled : Array Lean.Expr := #[]) :
    TacticM (Option MVarId) := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  -- `requires_of` of a clause: the precondition at a row of the entry values
  -- the clause names.  The row and the equation that it passes them are
  -- introduced, the equation opened per component (`openEntryValues`), and
  -- the precondition then read as of any row (`verex_behavior`), the
  -- components rewritten to the values the clause reads.
  if (unmarked target).isAppOf ``VerexIR.Proofs.RequiresOfEntries then
    let state ← saveState
    try
      let goal ← if target.isAppOfArity ``VerexIR.Proofs.Obligation 4 then
          goal.replaceTargetDefEq (unmarked target)
        else pure goal
      let (_, goal) ← goal.intro `verexArgs
      let (_, goal) ← goal.intro `verexEntryValues
      let goal ← openEntryValues goal `verexEntryValues
      setGoals [goal]
      -- The components of the entry-value equation are substituted into the
      -- goal where they can be (`openEntryValues`), so the precondition is
      -- read as of any row, at the values the clause spells.
      evalTactic (← `(tactic| simp (disch := first
          | (simp only [List.length_cons, List.length_nil, NRow.length] <;> fail)
          | skip) only
        [VerexIR.Proofs.requiresOf_runtime, vir_denote_norm, Prod.fst, Prod.snd]))
      -- A site's value whose captures the normal forms collapsed to a plain
      -- row is read by the arm the requires meaning unfolds to there, which
      -- the normal forms alone do not reach (`verex_behavior`).
      evalTactic (← `(tactic| all_goals try verex_behavior))
      -- A component the goal reads only now that the precondition is read
      -- (`readEntryValues`).
      setGoals (← (← getGoals).filterM (fun goal => not <$> goal.isAssigned))
      setGoals (← (← getGoals).mapM readEntryValues)
      return (← getGoals).head?
    catch error =>
      if verex.solverDebug.get (← getOptions) then
        IO.println s!"clause precondition not opened: {← error.toMessageData.toString}"
      state.restore
  -- `requires_of`: the value's requires meaning.
  if (unmarked target).isAppOf ``VerexIR.Proofs.RequiresOf then
    let state ← saveState
    try
      setGoals [goal]
      evalTactic (← `(tactic| verex_behavior))
      -- A component of an entry-value equation the goal reads only now that
      -- the meaning is read (`readEntryValues`).
      setGoals (← (← getGoals).filterM (fun goal => not <$> goal.isAssigned))
      setGoals (← (← getGoals).mapM readEntryValues)
      return (← getGoals).head?
    catch _ => state.restore
  let goal ← introduceBehaviors goal
  goal.withContext do
  let mut current := goal
  for hypothesis in ← goal.getNondepPropHyps do
    -- A predicate under premises the leaf holds, as a caller's contract
    -- states it under the absence of the failures it declares.
    let mut type ← instantiateMVars (← hypothesis.getType)
    -- Expose a composed receiver transition before dispatching its concrete
    -- dictionaries. Do this in the existing hypothesis traversal so ordinary
    -- leaves pay no additional context scan.
    let proposition := unmarked type
    -- A composition a generic callee's contract states at the frame its type
    -- arguments induce is opened as well, whatever the function value: each
    -- behavior it holds is then read at the caller's frame.
    if (proposition.isAppOf ``Exists || proposition.isAppOf ``And) &&
        (proposition.find? (fun e => e.isAppOf ``VerexIR.Proofs.EnsuresOf ||
          e.isAppOf ``VerexIR.Proofs.EnsuresOfEntries)).isSome &&
        ((literalFunction? (← getEnv) proposition).isSome ||
          entriesAtInducedFrame proposition) then
      current ← current.replaceLocalDeclDefEq hypothesis proposition
      let cases ← current.cases hypothesis
      let #[next] := cases | throwError "a behavioral conjunction has multiple alternatives"
      return ← dispatchBehavior next.mvarId callees handled
    unless states type do continue
    -- A behavior stated at the frame a call's type arguments induce, as a
    -- generic callee's contract states one of a function value it was passed,
    -- is the behavior at the caller's frame (`vir_transport`), where the
    -- value's target states its own.
    if statedAtInducedFrame type then
      let state ← saveState
      let name ← hypothesis.getUserName
      let transported ← try
          let renamed ← current.rename hypothesis `verexInducedBehavior
          setGoals [renamed]
          evalTactic (← `(tactic| simp only [vir_transport, vir_denote_norm]
            at $(mkIdent `verexInducedBehavior):ident))
          -- The entry values the opening equated are spelled at the callee's
          -- family; canonical, they meet the caller's row componentwise.
          -- Their rows are read through `NRow.finals`, which a family's
          -- respelling does not evaluate.
          evalTactic (← `(tactic| try simp only [VerexIR.Proofs.Denote.NRow.finals] at *))
          evalTactic (← `(tactic| try verex_canonical_hypotheses))
          let [next] ← getGoals | throwError "a transported behavior closed or split the leaf"
          -- An entry-value equation the opening kept whole, because its two
          -- sides were spelled at two families, opens now (`openEntryValues`).
          let kept ← next.withContext do
            (← getLCtx).foldlM (init := #[]) fun kept decl => do
              if decl.isImplementationDetail then return kept
              let some (_, left, right) := (← instantiateMVars decl.type).eq? | return kept
              if left.isAppOfArity ``Prod.mk 4 && right.isAppOfArity ``Prod.mk 4 then
                return kept.push decl.userName
              return kept
          let mut next := next
          for equation in kept do
            next ← openEntryValues next equation
          setGoals [next]
          next.withContext do
            let some decl := (← getLCtx).findFromUserName? `verexInducedBehavior
              | throwError "a transported behavior left the context"
            if statedAtInducedFrame (← instantiateMVars decl.type) then
              throwError "a behavior at an induced frame has no transport"
            pure (some (← next.rename decl.fvarId name))
        catch error =>
          if verex.solverDebug.get (← getOptions) then
            IO.println s!"behavior not transported: {← error.toMessageData.toString}"
          state.restore
          pure none
      if let some next := transported then
        return ← dispatchBehavior next callees handled
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
    if handled.contains type then continue
    if ← (← getLCtx).anyM (fun decl => do
        let recorded ← instantiateMVars decl.type
        return recorded.isAppOfArity ``BehaviorDispatched 1 &&
          recorded.appArg!.consumeMData == type.consumeMData) then continue
    let some view := behaviorView? type | continue
    -- A predicate over a model's invocation meaning is read by its run
    -- (`modelDenotedRun`).
    let some executable := executableOf? view.invoke | continue
    let dispatched ← if view.function.consumeMData.isAppOf ``functionOfGeneric then
        genericBehavior? view stated callees
      else literalBehavior? view stated type executable callees
    let some (theoremProof, theoremArguments, implication) := dispatched | continue
    -- The natives the target's theorem assumes, the caller assumes too.
    let mut assumed := true
    for argument in theoremArguments do
      if let .mvar id ← instantiateMVars argument then
        unless ← id.isAssigned do
          try id.assumption catch _ =>
            if (← instantiateMVars (← id.getType)).isAppOf ``FrameOf then
              unless ← closesBy id (← `(tactic| exact FrameOf.frame rfl)) do assumed := false
            else assumed := false
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
          ((try simp only [$lemmas,*, Weave.subst, Weave.compose, vir_denote_norm]) <;>
            verex_split_hypotheses <;> (repeat apply And.intro) <;> first
              | assumption
              | (apply VerexIR.Proofs.MethodContract.of_instantiate _ (by decide)
                 simp only [ResultShape.functionOfSkolem, NTy.ofSkolem_toSkolem, vir_denote_norm]
                 assumption)
              | (apply VerexIR.Proofs.ReceiverScopedMethod.of_instantiate _
                 simp only [ResultShape.functionOfSkolem, NTy.ofSkolem_toSkolem, vir_denote_norm]
                 assumption)
              | verex_leaf))) do
        established := false
        break
      fact := mkApp fact (← instantiateMVars premise)
    unless established do
      if verex.solverDebug.get (← getOptions) then IO.println "behavior dispatch: contract premises unproved"
      continue
    let (_, next) ← (← current.assert `verexBehavior (← inferType fact) fact).intro1P
    let (_, next) ← (← next.assert `verexBehaviorDispatched
      (mkApp (mkConst ``BehaviorDispatched) type)
      (mkApp (mkConst ``BehaviorDispatched.intro) type)).intro1P
    current := next
    let state ← saveState
    try
      setGoals [current]
      -- The failures a contract permits, each absent: the negated
      -- disjunction as the conjunction of the negations.
      -- A captured dictionary's behavioral predicate is stated in the
      -- adapter's instantiated frame. Transport it back to the caller before
      -- the leaf compares it with an abstract callback's own behavior.
      evalTactic (← `(tactic| simp only [$lemmas,*, Weave.subst, Weave.compose, ResultShape.toSkolem,
        vir_transport, vir_denote_norm, not_false_eq_true, true_implies, not_or]
        at $(mkIdent `verexBehavior):ident))
      current ← getMainGoal
    catch _ => state.restore
    -- A derived fact states its target's clauses without their markers, which
    -- name an obligation in a diagnostic: the deciders read its statement.
    current ← current.withContext do
      let some decl := (← getLCtx).findFromUserName? `verexBehavior | pure current
      let type ← instantiateMVars decl.type
      let stated := type.replace fun term =>
        if term.isAppOfArity ``VerexIR.Proofs.Obligation 4 then some term.appArg! else none
      if stated == type then pure current
      else try current.replaceLocalDeclDefEq decl.fvarId stated catch _ => pure current
    -- A forwarding contract can itself state a concrete callback's behavior.
    -- Read that fact through its verified target too. Keep the observations
    -- already dispatched so a contract repeating its own run is not expanded
    -- again; abstract callback facts simply remain for the leaf deciders.
    return ← dispatchBehavior current callees (handled.push type)
  return some current

/-- Whether a statement reads a behavioral predicate. -/
private def readsBehavior (statement : Lean.Expr) : Bool :=
  (statement.find? fun e => e.isAppOf ``VerexIR.Proofs.RequiresOf ||
    e.isAppOf ``VerexIR.Proofs.RequiresOfEntries ||
    e.isAppOf ``VerexIR.Proofs.AbortsOf || e.isAppOf ``VerexIR.Proofs.AbortsOfEntries ||
    e.isAppOf ``AtEntries || e.isAppOf ``AtEntryFinals ||
    e.isAppOf ``VerexIR.Proofs.EnsuresOfEntries ||
    e.isAppOf ``VerexIR.Proofs.EnsuresOf || statesDirectCall e).isSome

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
      -- A point may already state this behavior, as a completed call does.
      -- Reuse it before opening the predicate and its quantified arguments.
      if ← closesBy conjunct (← `(tactic| verex_assumption)) then continue
      let some dispatched ← dispatchBehavior conjunct callees | continue
      -- A component of an entry-value equation the point reads, once the
      -- behavioral predicates are read (`readEntryValues`).
      let next ← unmarkIntroduced conjunct (← readEntryValues dispatched)
      -- The projections of the rows a clause builds reduced: a point's
      -- statement and the facts of the call that reaches it name the same
      -- value only once each is read off its row.
      unless ← closesBy next (← `(tactic|
          ((try verex_reduce_projections); verex_leaf_cheap))) do
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
           (VerexIR.Proofs.ResolvesAll.ofResolveAt rfl) ?_
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
      let some plainFree ← decidedTrue? plain | continue
      let some shapeFree ← decidedTrue? shapePlain | continue
      let kept := mkAppN (← frameFactProof fact) #[plainFree,
        shapeFree, ensuresType.getArg! 8, ensuresType.getArg! 9,
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
  -- A frame equation fixing a summary's parameter stays (`substEquation?`).
  if keeps then return (← substEquation? next equation).getD next
  -- Any other frame stays a fact, for the leaf and the behavior run.
  noteKeptFact equation
  pure next

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
          VerexIR.Proofs.Denote.NTy.unmark, Bool.and_true, Bool.true_and]
       first | rfl | decide)))
    unless (← getGoals).isEmpty do throwError "a row is not scalar"
    return true
  catch _ =>
    state.restore
    return false

/-- Whether a function value is a known one (`LiteralValue`). -/
private def isKnownFunction (e : Lean.Expr) : MetaM Bool :=
  return isLiteralValue (← getEnv) e.consumeMData

/-- Read a literal through the frame conversions introduced by generic calls.
The conversion preserves the actual closure, so no receipt is recomputed or
assumed at a different frame. -/
private def scopedClosureEquation? : Nat → Lean.Expr → MetaM (Option Lean.Expr)
  | 0, _ => pure none
  | fuel + 1, value => do
      let value := value.consumeMData
      if let some literal ← literalValue? value then return some literal.closed
      if value.isAppOf ``functionOfGeneric then
        return some (mkAppN (mkConst ``functionOfGeneric_closure) value.getAppArgs)
      unless value.isAppOfArity ``ResultShape.functionOfSkolem 8 do return none
      let some closed ← scopedClosureEquation? fuel (value.getArg! 7) | return none
      let transported := mkAppN (mkConst ``ResultShape.closure_functionOfSkolem) value.getAppArgs
      return some (← mkAppM ``Eq.trans #[transported, closed])

/-- A scoped dictionary is either a checked literal or a forwarded assumption.
The refinement follows the actual closure, including captured dictionaries. -/
def receiverScopedMethod? (goal : MVarId) : TacticM (Option (List MVarId)) := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``VerexIR.Proofs.ReceiverScopedMethod 7 do return none
  for declaration in ← getLCtx do
    if declaration.isImplementationDetail then continue
    for fact in ← conjuncts declaration.toExpr do
      if ← isDefEq (← inferType fact) target then
        goal.assign fact
        return some []
  if let some closed ← scopedClosureEquation? 16 (target.getArg! 6) then
    let rule ← mkAppOptM ``VerexIR.Proofs.ReceiverScopedMethod.of_closure_scoped
      #[target.getArg! 0, target.getArg! 1, target.getArg! 2,
        target.getArg! 3, target.getArg! 4, target.getArg! 5, target.getArg! 6, none, closed]
    return some (← goal.apply rule)
  if let some (θ, outer) ← (do
      let_expr VerexIR.Proofs.Denote.Skolems.instantiate _ _ θ outer := target.getArg! 2
        | return none
      return some (θ, outer)) then
    let rule ← mkAppOptM ``VerexIR.Proofs.ReceiverScopedMethod.of_instantiate
      #[target.getArg! 0, target.getArg! 1, outer, θ,
        target.getArg! 3, target.getArg! 4, target.getArg! 5, target.getArg! 6]
    return some (← goal.apply rule)
  return none

/-- Check a literal's receipt and recursively discharge its captured dictionary
requirements. Unknown function values need an explicit scoped assumption. -/
def receiverScopedClosure? (goal : MVarId) : TacticM (Option (List MVarId)) := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``VerexIR.Proofs.ReceiverScopedClosure 2 do return none
  for declaration in ← getLCtx do
    if declaration.isImplementationDetail then continue
    for fact in ← conjuncts declaration.toExpr do
      let type ← inferType fact
      unless type.isAppOf ``VerexIR.Proofs.ReceiverScopedMethod ||
          type.isAppOf ``VerexIR.Proofs.ReceiverScopedClosure do continue
      if ← isDefEq type target then
        goal.assign fact
        return some []
  let raw ← withTransparency .all <| whnf (target.getArg! 1)
  unless raw.isAppOfArity ``VerexIR.RuntimeValue.closure 4 do return none
  -- Project the target and mask before deciding the receipt. Generic closure
  -- records also contain open frame/capture values which these fields ignore.
  let handle ← withTransparency .all <| whnf (raw.getArg! 0)
  let mask ← withTransparency .all <| whnf (raw.getArg! 1)
  let requirements ← mkAppM ``VerexIR.Proofs.scopedDictionaryCaptures?
    #[target.getArg! 0, handle, mask, raw.getArg! 3]
  let values ← withTransparency .all <| whnf requirements
  unless values.isAppOfArity ``Option.some 2 do return none
  let rule ← mkAppOptM ``VerexIR.Proofs.ReceiverScopedClosure.checked
    #[target.getArg! 0, handle, mask, raw.getArg! 2,
      raw.getArg! 3, values.appArg!]
  let saved ← saveState
  try
    let [receipt, shape, requirements, captured] ← goal.apply rule
      | throwError "unexpected scoped receiver obligations"
    unless ← closesBy receipt (← `(tactic| decide +kernel)) do
      throwError "the callback has no checked receiver receipt"
    unless ← closesBy shape (← `(tactic| first | assumption | decide +kernel)) do
      throwError "the callback has no checked closure shape"
    requirements.refl
    setGoals [captured]
    evalTactic (← `(tactic| try simp only [List.mem_reverse, List.mem_cons, List.not_mem_nil, forall_eq_or_imp,
      forall_false, forall_const, and_true]))
    return some (← getGoals)
  catch error =>
    saved.restore
    if verex.solverDebug.get (← getOptions) then
      IO.println s!"scoped receiver receipt: {← error.toMessageData.toString}"
    return none

/-- A dictionary's contract is proved at the caller's frame, then by the
ordinary verified-target invocation rule at every argument and memory. -/
def methodContract? (goal : MVarId) : TacticM (Option (List MVarId)) := goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``VerexIR.Proofs.MethodContract 8 do return none
  for declaration in ← getLCtx do
    if declaration.isImplementationDetail then continue
    for fact in ← conjuncts declaration.toExpr do
      if ← isDefEq (← inferType fact) target then
        goal.assign fact
        return some []
  if let some (θ, outer) ← (do
      let_expr VerexIR.Proofs.Denote.Skolems.instantiate _ _ θ outer := target.getArg! 3
        | return none
      return some (θ, outer)) then
    let free ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
      (← mkEq (mkApp (mkConst ``NRow.refFree) (← mkAppM ``Subtype.val #[θ])) (mkConst ``Bool.true))
    let rule ← mkAppOptM ``VerexIR.Proofs.MethodContract.of_instantiate
      #[target.getArg! 0, target.getArg! 1, target.getArg! 2, outer, θ, free,
        target.getArg! 4, target.getArg! 5, target.getArg! 6, target.getArg! 7]
    return some (← goal.apply rule)
  let (_, next) ← goal.introN 2 [`verexMethodArguments, `verexMethodInitial]
  return some [next]

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
    let mut methodFacts := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      unless (declaration.type.find? (·.isConstOf ``VerexIR.Proofs.MethodContract)).isSome do continue
      methodFacts := methodFacts ++ (← conjuncts declaration.toExpr)
    for fact in methodFacts do
      unless (← inferType fact).isAppOf ``VerexIR.Proofs.MethodContract do continue
      let saved ← saveState
      try
        let rule ← mkAppM ``VerexIR.Proofs.MethodContract.invoke #[fact, action.getArg! 8]
        let [next] ← goal.apply rule | saved.restore; continue
        let (_, next) ← next.introN 2 [`verexResult, `verexEnsures]
        return some [next]
      catch _ => saved.restore
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
            Which.projectCertified?_eq_some_iff,
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
        (try simp only [$lemmas,*, Weave.subst, Weave.compose, vir_denote_norm])
        all_goals (try constructor)
        all_goals verex_leaf))) do
      throwError "the target's theorem does not apply everywhere"
    -- The target's frame lies within the closure's.
    let framed ← mkFreshExprSyntheticOpaqueMVar framedType
    unless ← closesBy framed.mvarId! (← `(tactic| (
        intro _ _ _ _ $(mkIdent `verexFrame):ident
        (try simp only [$lemmas,*, Weave.subst, Weave.compose, vir_denote_norm]
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
literal builds, outside binders, and those the rows of the reads found name
in turn — a read of an invocation on the result of an earlier one names that
earlier read in its row, and a leaf reads both invocations. -/
private partial def resultOfReads (environment : Environment) (terms : Array Lean.Expr) :
    Array Lean.Expr :=
  let found := sitesWhere (fun e =>
    (e.isAppOfArity ``VerexIR.Proofs.ResultOf 11 ||
      e.isAppOfArity ``VerexIR.Proofs.StateOf 10) && !e.hasLooseBVars &&
      (literalFunction? environment (e.getArg! 7)).isSome) terms
  if found.isEmpty then found
  else found ++ resultOfReads environment (found.map (·.getArg! 8))

/-- The `aborts_of` of the invocation a `result_of` or `state_of` reads, in
the clause form all three read: at the same entry values
(`AbortsOfEntries`). -/
private def readAborts (read : Lean.Expr) : Lean.Expr :=
  mkAppN (mkConst ``VerexIR.Proofs.AbortsOfEntries) (read.getAppArgs.extract 0 10)

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
      let debug := verex.solverDebug.get (← getOptions)
      for read in resultOfReads (← getEnv) statements do
        let aborts := readAborts read
        if statements.any (fun statement => (statement.find? (· == aborts)).isSome) then continue
        -- A known abort decision needs no fresh case split; definitional
        -- equality supplies the match.
        let known ← facts.anyM fun fact => do
          let type := unmarked (← instantiateMVars (← inferType fact))
          let predicate := type.not?.getD type
          unless predicate.isAppOfArity ``VerexIR.Proofs.AbortsOfEntries 10 do return false
          -- The row a clause spells and the settling of a run's arguments
          -- are the same row, which the comparison reads.
          withNewMCtxDepth (isDefEq predicate aborts)
        if known then continue
        if debug then
          logInfo m!"abort cases at {goal.name}: splitting on{indentExpr (aborts.getArg! 8)}"
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
    -- The run's invocation from its pre-state, as a clause's `aborts_of`
    -- states it: at the entry values its arguments pass.
    let args := run.getAppArgs
    let stated := mkAppN (mkConst ``VerexIR.Proofs.AbortsOfEntries)
      (args.extract 0 8
        |>.push (settledRow args[0]! args[1]! args[2]! args[4]! args[8]!)
        |>.push args[10]!)
    -- The entry values a clause reads are the settling of the run's
    -- arguments, which unfolds: the comparison reads it.
    unless ← isDefEq stated aborts do continue
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
          unless aborts.isAppOfArity ``VerexIR.Proofs.AbortsOfEntries 10 &&
              (literalFunction? (← getEnv) (aborts.getArg! 7)).isSome do continue
          unless handled.contains aborts || found.any (·.2 == aborts) do
            unless ← hasKnownInvocation aborts do
              found := found.push (negation, aborts)
      pure found
    if debug && !candidates.isEmpty then
      logInfo m!"terminating runs at {current.name}: {candidates.size} candidates"
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
    -- would speak of a memory no longer named. A substitution refused
    -- because the equation fixes a summary's parameter keeps the equation
    -- and the memory named, so the search goes on.
    if keeps then
      if let some substituted ← substEquation? next equation then return substituted
    noteKeptFact equation
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
        Which.projectCertified?_eq_some_iff,
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
    -- The abort: the failure, the meaning's abort on it, and the invocation's
    -- abort in both readings, at the arguments and at their entry values
    -- (`AbortsOf`, `AbortsOfEntries`).
    let (_, fails) ← fails.introN 4
      [`verexError, `verexAborted, `verexAborts, `verexAbortsOfEntries]
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
      | exact VerexIR.Proofs.requiresOf_of_entries (by assumption)
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
  unless ← (pure (decide (lhs.getAppNumArgs ≥ 2)) <&&> isDefEq (lhs.getArg! (lhs.getAppNumArgs - 2)) value) do
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
  let refined ← whnfR refined
  unless refined.isAppOfArity ``VerexIR.Proofs.Spec.Refines 5 do
    throwError "an inlined target's refinement does not name its body"
  unless ← isDefEq (refined.getArg! 3) action do
    throwError "an inlined target's refinement does not meet its run"
  let refinement ← instantiateMVars (mkAppN theoremProof binders)
  let refined ← instantiateMVars refined
  let (σ, ε, α) := (refined.getArg! 0, refined.getArg! 1, refined.getArg! 2)
  let (left, body) := (refined.getArg! 3, refined.getArg! 4)
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
      let runType ← instantiateMVars (← inferType (.fvar runVar))
      unless runType.isAppOfArity ``VerexIR.Proofs.Spec.ok 7 do throwError "not a run"
      let (initial, result, final) := (runType.getArg! 4, runType.getArg! 5, runType.getArg! 6)
      let leaf ← goal.getType
      let ensures ← withLocalDeclD `result α fun r => withLocalDeclD `final σ fun f => do
        mkLambdaFVars #[r, f] (← mkArrow (← mkEq r result) (← mkArrow (← mkEq f final) leaf))
      let established ← mkFreshExprSyntheticOpaqueMVar
        (mkAppN (mkConst ``VerexIR.Proofs.wp) #[σ, ε, α, body, ensures, ← trueFailures ε, initial])
        (← goal.getTag)
      goal.assign (mkAppN (mkConst ``VerexIR.Proofs.forall_ok_of_refines_of_wp)
        #[σ, ε, α, left, body, refinement, ensures, initial, established, result, final,
          .fvar runVar, ← mkEqRefl result, ← mkEqRefl final])
      let next := established.mvarId!
      setGoals [next]
      evalTactic (← `(tactic| try simp only [$unfolds,*, VerexIR.Proofs.Denote.Function.denoteWith,
        VerexIR.Proofs.Denote.NRow.nil_append, VerexIR.Proofs.Denote.NRow.cons_append]))
      return some (← getMainGoal)
  | none =>
      let abortsType ← instantiateMVars (← inferType (.fvar runVar))
      let .app _ (.lam _ _ aborts _) := abortsType | throwError "the abort is not an existential"
      unless aborts.isAppOfArity ``VerexIR.Proofs.Spec.aborts 6 do throwError "not an abort"
      let initial := aborts.getArg! 4
      let leaf ← goal.getType
      let returns ← withLocalDeclD `result α fun result => withLocalDeclD `final σ fun final =>
        mkLambdaFVars #[result, final] (mkConst ``True)
      let fails ← withLocalDeclD `error ε fun error => mkLambdaFVars #[error] leaf
      let established ← mkFreshExprSyntheticOpaqueMVar
        (mkAppN (mkConst ``VerexIR.Proofs.wp) #[σ, ε, α, body, returns, fails, initial])
      goal.assign (mkAppN (mkConst ``VerexIR.Proofs.of_aborts_of_refines_of_wp)
        #[σ, ε, α, left, body, refinement, initial, leaf, established, .fvar runVar])
      setGoals [established.mvarId!]
      evalTactic (← `(tactic| try simp only [$unfolds,*, VerexIR.Proofs.Denote.Function.denoteWith,
        VerexIR.Proofs.Denote.NRow.nil_append, VerexIR.Proofs.Denote.NRow.cons_append]))
      return some (← getMainGoal)

/-- A leaf stated as the weakest precondition of the action its run is a run
of: the rule reading every run of the action, and the precondition it leaves
as the goal.

A run whose start and end are one local is a run that ends where it started,
which the clause states where the invocation leaves the memory it names
unchanged. The precondition cannot then be read at a start the leaf names,
so the rule quantifies the start (`forall_ok_self_of_wp`) and the goal is
read at the state it names, as the goal of a run from a named start is read
at that start (`forall_ok_of_wp`). -/
private def forallOkOfWp (goal : MVarId) (unitShape self : Bool) : MetaM (Option MVarId) := do
  let rule := match unitShape, self with
    | true, false => ``VerexIR.Proofs.forall_ok_unit_of_wp
    | false, false => ``VerexIR.Proofs.forall_ok_of_wp
    | true, true => ``VerexIR.Proofs.forall_ok_unit_self_of_wp
    | false, true => ``VerexIR.Proofs.forall_ok_self_of_wp
  let [next] ← goal.apply (← mkConstWithFreshMVarLevels rule) | return none
  if self then return some (← next.intro1P).2
  return some next

/-- Whether a run's start and end are one local, which a clause states of an
invocation that leaves the memory it names unchanged (`forallOkOfWp`). -/
private def selfRun (pre post : Lean.Expr) : Bool :=
  pre.consumeMData == post.consumeMData

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
  let some capturedFree ← decidedTrue? (mkApp (mkConst ``NRow.refFree) captured)
    | throwError "a captured row a frame resolves"
  if let some run := view.run? then
    let unitShape := view.shape.isConstOf ``ResultShape.none
    let resultVar? ← if unitShape then pure none else
      let .fvar resultVar := run.result | throwError m!"result not a variable {run.result}"
      pure (some resultVar)
    let .fvar finalVar := run.post | throwError "final not a variable"
    let self := selfRun view.state run.post
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
    forallOkOfWp goal unitShape self
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

/-- A leaf's run or abort of a known target read directly (`CallEnsures`,
`CallAborts`): the leaf becomes the weakest precondition of the call under which it
holds of each run or abort (`forallOkOfWp`, `of_aborts_of_wp`), which the call
rule reads by the target's theorem or body, as a body's call. The hypothesis stays,
marked read. A run naming no local for its result stays a fact. -/
private def denotedDirectCall (goal : MVarId) (hypothesis : FVarId) (stated type : Lean.Expr) :
    MetaM (Option MVarId) := do
  let mark := mkApp (mkConst ``VerexIR.Proofs.Denoted) type
  if type.isAppOfArity ``VerexIR.Proofs.CallEnsures 7 then
    let (call, result, initial, post) :=
      (type.getArg! 3, type.getArg! 4, type.getArg! 5, type.getArg! 6)
    let some shape := if call.isAppOfArity ``propheticMeaning 9 then some (call.getArg! 7) else none
      | return none
    let unitShape := shape.consumeMData.isConstOf ``ResultShape.none
    let .fvar finalVar := post | return none
    let self := selfRun initial post
    let resultVar? ← if unitShape then pure none else
      let .fvar resultVar := result | return none
      pure (some resultVar)
    let run ← mkExpectedTypeHint stated (← mkAppM ``VerexIR.Proofs.Spec.ok #[call, initial, result, post])
    let runType ← inferType run
    let goal ← goal.replaceLocalDeclDefEq hypothesis mark
    let (runVar, goal) ← goal.withContext do (← goal.assert `verexRun runType run).intro1P
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
    return ← forallOkOfWp goal unitShape self
  let (call, initial) := (type.getArg! 3, type.getArg! 4)
  let callType ← whnfR (← inferType call)
  unless callType.isAppOfArity ``VerexIR.Proofs.Spec 3 do return none
  let (σ, ε, α) := (callType.getArg! 0, callType.getArg! 1, callType.getArg! 2)
  let run ← withLocalDeclD `error ε fun error => do
    let failed ← mkAppM ``VerexIR.Proofs.Spec.aborts #[call, initial, error]
    mkExpectedTypeHint stated (← mkAppM ``Exists #[← mkLambdaFVars #[error] failed])
  let goal ← goal.replaceLocalDeclDefEq hypothesis mark
  goal.withContext do
    let leaf ← goal.getType
    let returns ← withLocalDeclD `result α fun result => withLocalDeclD `final σ fun final =>
      mkLambdaFVars #[result, final] (mkConst ``True)
    let fails ← withLocalDeclD `error ε fun error => mkLambdaFVars #[error] leaf
    let established ← mkFreshExprSyntheticOpaqueMVar
      (mkAppN (mkConst ``VerexIR.Proofs.wp) #[σ, ε, α, call, returns, fails, initial])
    goal.assign (mkAppN (mkConst ``VerexIR.Proofs.of_aborts_of_wp)
      #[σ, ε, α, call, initial, leaf, established, run])
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
      if statesDirectCall statement then return some statement
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
      if statesDirectCall type then
        if let some next ← goal.withContext (denotedDirectCall goal hypothesis stated type) then
          return some next
      else if let some next ← goal.withContext (denotedRun goal callees hypothesis stated type) then
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

/-- The predicates stating a function value's behavior, and the direct
readings of a call's target (`CallEnsures`, `CallAborts`). -/
def behaviorPredicates : Array Name :=
  #[``VerexIR.Proofs.EnsuresOf, ``VerexIR.Proofs.AbortsOf, ``VerexIR.Proofs.AbortsOfEntries,
    ``AtEntries, ``VerexIR.Proofs.EnsuresOfEntries, ``AtEntryFinals,
    ``VerexIR.Proofs.ResultOf, ``VerexIR.Proofs.StateOf,
    ``VerexIR.Proofs.CallEnsures, ``VerexIR.Proofs.CallAborts]

/-- Whether an expression states a behavioral predicate (`behaviorPredicates`),
also through the continuations it reaches (`findReached?`). -/
def statesBehavior (es : Array Lean.Expr) (continuations : Array Name := #[]) : MetaM Bool := do
  let behavioral (sub : Lean.Expr) : Bool := behaviorPredicates.any sub.isConstOf
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

/-- The direct reading of a call's target the goal owes (`CallEnsures`, `CallAborts`):
the meaning it states, at the call's family and target, whose instantiation names
the call's type arguments, however they are located. -/
private def owedDirectCall? (target : Lean.Expr) (continuations : Array Name)
    (meaning : Lean.Expr) : MetaM (Option Lean.Expr) := do
  findReached? (← owedStatements target) continuations fun next => do
    for statement in sitesWhere statesDirectCall #[next] do
      let owed := (statement.getArg! 3).consumeMData
      unless owed.isAppOfArity ``propheticMeaning 9 do continue
      if (owed.getArg! 5).hasLooseBVars || (owed.getArg! 4).hasLooseBVars ||
          (owed.getArg! 3).hasLooseBVars then continue
      unless ← isDefEq (owed.getArg! 5) (meaning.getArg! 5) do continue
      unless ← isDefEq (owed.getArg! 3) (meaning.getArg! 3) do continue
      return some owed
    return none

/-- Whether a goal's call is of a target the goal owes the direct reading of. -/
def owesDirectCall (goal : MVarId) (continuations : Array Name := #[]) : MetaM Bool :=
    goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  unless target.isAppOfArity ``VerexIR.Proofs.wp 7 do return false
  let meaning := target.getArg! 3
  unless meaning.isAppOfArity ``propheticMeaning 9 do return false
  return (← owedDirectCall? target continuations meaning).isSome

/-- The goal's call at the instantiation a direct reading states, where the two name
the same types (`frameInstantiation_congr`). -/
private def callAtOwedInstantiation (goal : MVarId) (owed : Lean.Expr) : MetaM (Option MVarId) :=
    goal.withContext do
  let target ← instantiateMVars (← goal.getType)
  let meaning := target.getArg! 3
  let (actual, stated) := (meaning.getArg! 4, owed.getArg! 4)
  if ← withReducible (isDefEq actual stated) then return some goal
  unless actual.isAppOfArity ``frameInstantiation 4 && stated.isAppOfArity ``frameInstantiation 4 do
    return none
  unless ← isDefEq (actual.getArg! 2) (stated.getArg! 2) do return none
  unless ← isDefEq (actual.getArg! 1) (stated.getArg! 1) do return none
  let ids (instantiation : Lean.Expr) : MetaM Lean.Expr := do
    let typeUse := mkConst ``VerexIR.TypeUse
    let toTypeId := .lam `use typeUse (mkApp (mkConst ``VerexIR.TypeUse.typeId) (.bvar 0)) .default
    mkAppM ``List.map #[toTypeId, ← mkAppM ``Array.toList #[instantiation.getArg! 3]]
  let (left, right) := (← ids actual, ← ids stated)
  unless ← withDefault (isDefEq left right) do return none
  let same ← mkExpectedTypeHint (← mkEqRefl left) (← mkEq left right)
  let congr ← mkAppOptM ``frameInstantiation_congr
    #[actual.getArg! 0, actual.getArg! 1, actual.getArg! 2, actual.getArg! 3, stated.getArg! 3, same]
  let args := target.getAppArgs
  let motive ← withLocalDeclD `instantiation (← inferType actual) fun instantiation => do
    mkLambdaFVars #[instantiation]
      (mkAppN target.getAppFn (args.set! 3 (mkAppN meaning.getAppFn
        (meaning.getAppArgs.set! 4 instantiation))))
  let proof ← mkCongrArg motive congr
  let next := mkAppN target.getAppFn (args.set! 3 (mkAppN meaning.getAppFn
    (meaning.getAppArgs.set! 4 stated)))
  return some (← goal.replaceTargetEq next proof)

/-- A call where the goal owes the direct reading of its target
(`owedDirectCall?`), as a body's call of it reads: its continuations assume the
call's `ensures_of` and `aborts_of` (`wp_call_runs`) and, where runs agree and the
row lends flatly, the absence of its abort (`wp_call_runs_agree`). -/
private def directCall (goal : MVarId) (throughContract : Bool) (owed : Lean.Expr) :
    TacticM (Option MVarId) := do
  let some goal ← callAtOwedInstantiation goal owed | return none
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let meaning := target.getArg! 3
  let (unit, functions, executable, frame) :=
    (meaning.getArg! 0, meaning.getArg! 1, meaning.getArg! 2, meaning.getArg! 3)
  let agrees? ← goal.withContext do
    (← getLCtx).findDeclM? fun declaration => do
      if declaration.isImplementationDetail then return none
      let type ← instantiateMVars declaration.type
      return if type.isAppOfArity ``VerexIR.Proofs.RunsAgree 2 then some declaration.toExpr else none
  let saved ← saveState
  if let some agrees := agrees? then
    try
      setGoals [goal]
      let agreesSyntax ← goal.withContext (Lean.Elab.Term.exprToSyntax agrees)
      let rule ← goal.withContext <| Lean.Elab.Term.exprToSyntax
        (mkApp4 (mkConst ``VerexIR.Proofs.wp_call_runs_agree) unit functions executable frame)
      Term.withoutErrToSorry <| withoutRecover <|
        evalTactic (← `(tactic| refine $rule $agreesSyntax (by decide) ?_))
      let [named] ← getGoals | throwError "the direct call leaves one goal"
      return some (← if throughContract then markNamedRead named else pure named)
    catch failure =>
      saved.restore
      if verex.solverDebug.get (← getOptions) then
        logInfo m!"direct call where runs agree not taken: {failure.toMessageData}"
  let [named] ← goal.withContext do
    goal.apply (mkApp2 (mkConst ``VerexIR.Proofs.wp_call_runs) unit functions)
    | return none
  return some (← if throughContract then markNamedRead named else pure named)

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
  if let some owed ← goal.withContext (owedDirectCall? target continuations meaning) then
    if let some next ← directCall goal throughContract owed then return next
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
    else if action.isAppOfArity ``VerexIR.Proofs.Denote.Flow.bind 8 then
      let inner := (action.getArg! 6).consumeMData.headBeta
      if inner.isAppOfArity ``VerexIR.Proofs.Denote.Flow.bind 8 then
        -- Flows associate to the right, so that the goal's continuation
        -- stays out of the inner bind's arms.
        pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_flowBind_flowBind)
          (action.getAppArgs.extract 0 4 ++
            #[inner.getArg! 4, inner.getArg! 5, action.getArg! 5, inner.getArg! 6,
              inner.getArg! 7, action.getArg! 7] ++ conditions))
      else if inner.isAppOfArity ``VerexIR.Proofs.Spec.bind 6 then
        -- A flow after a bind binds inside the bind's continuation, where the
        -- goal's continuation appears once.
        pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.wp_flowBind_specBind)
          ((action.getAppArgs.extract 0 6).push (inner.getArg! 2) ++
            #[inner.getArg! 4, inner.getArg! 5, action.getArg! 7] ++ conditions))
      else
      -- The flow's carrier family, state and types instantiate the rule.
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
  -- Preserve the conjunction as one proof when later scopes close their
  -- projection consumers. Authored paths keep the fresh goal and its tag.
  let pending ← SharedContinuations.markSharedProof combined.mvarId!
  (← targetState).leafPaths.modify (·.insert pending paths)
  return some pending

end VerexIR.Proofs.Denote
