-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.DenotationWP
import LeanerIR.Proofs.Plain

/-!
# Constructive closing

The native drive leaves one obligation per exit path, carrying the values
the body computed and the range facts its checked operations established.
After one directed unfolding of the contract surface those obligations are
built from a fixed grammar: conjunction, existentials whose witnesses the
codec equations determine, implications and negations over
argument-row equations, decoder applications stuck on a range test, and
arithmetic leaves.

This tactic closes that grammar by construction. Each shape is recognized
syntactically and its proof term is assembled directly — the only search
permitted is `omega` on an isolated arithmetic leaf, which is a decision
procedure, not lemma search. An unrecognized shape fails the whole tactic,
cleanly, so the generated script's `try (… done)` wrapper falls back to
the search closing unchanged.

The measured reason this exists: the search closing has a floor of roughly
20–50M heartbeats per function that does not vary with the function, and
five perturbations of its rule set were neutral or worse — the cost is the
obligation goal's own size, which no rule adjustment shrinks. Construction
does not walk the goal repeatedly, so it has no such floor.
-/

namespace LeanerIR.Proofs

open Lean Elab Tactic Meta

namespace Certify

/-- Close `goal` with the tactic `tac`; `false` if it fails or leaves
subgoals. The surrounding state is untouched either way. -/
private def solvedBy (goal : MVarId) (tac : TacticM Unit) : TacticM Bool := do
  let saved ← saveState
  try
    setGoals [goal]
    tac
    let remaining ← getGoals
    if remaining.isEmpty then
      pure true
    else
      saved.restore
      pure false
  catch _ =>
    saved.restore
    pure false

/-- The certified integers whose value `e` mentions: every closed
`SpecInt.val x` subterm, by its certified value `x`. -/
private partial def certifiedValues (e : Lean.Expr) (found : Array Lean.Expr := #[]) :
    Array Lean.Expr :=
  if e.isAppOfArity ``LeanerIR.SpecInt.val 3 && !e.hasLooseBVars then
    let value := e.getArg! 2
    if found.contains value then found else found.push value
  else match e with
    | .app f a => certifiedValues a (certifiedValues f found)
    | .lam _ t b _ | .forallE _ t b _ => certifiedValues b (certifiedValues t found)
    | .letE _ t v b _ => certifiedValues b (certifiedValues v (certifiedValues t found))
    | .mdata _ b => certifiedValues b found
    | .proj _ _ b => certifiedValues b found
    | _ => found

/-- Expose the bounds every unsigned certificate carries, as the two
inequalities `omega` reads: the certificates in context, and the
certificates of the certified integers the target mentions — a field of
a stored twin carries its own.  One `have` per certificate; nothing is
searched. -/
private def exposeBounds (goal : MVarId) : MetaM MVarId := do
  let certificates ← goal.withContext do
    let mut found : Array Lean.Expr := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      let ty ← instantiateMVars declaration.type
      if ty.isAppOfArity ``LeanerIR.IntegerValueFits 3 then
        let width := ty.getArg! 0
        let signed := ty.getArg! 1
        if width.isAppOfArity ``LeanerIR.IntWidth.bits 1 &&
            signed.isConstOf ``Bool.false then
          found := found.push (mkFVar declaration.fvarId)
    for value in certifiedValues (← instantiateMVars (← goal.getType)) do
      let ty ← whnf (← inferType value)
      if ty.isAppOfArity ``LeanerIR.SpecInt 2 then
        let width := ty.getArg! 0
        let signed := ty.getArg! 1
        if width.isAppOfArity ``LeanerIR.IntWidth.bits 1 &&
            signed.isConstOf ``Bool.false then
          found := found.push (← mkAppM ``LeanerIR.SpecInt.fits #[value])
    pure found
  let mut goal := goal
  for certificate in certificates do
    let bounds ← goal.withContext do
      mkAppM ``LeanerIR.IntegerValueFits.unsigned_bounds #[certificate]
    let boundsType ← goal.withContext do inferType bounds
    /- Once per certificate: a leaf visited after another already has the
    bounds in context. -/
    let present ← goal.withContext do
      (← getLCtx).anyM fun declaration => do
        pure ((← instantiateMVars declaration.type) == boundsType)
    if present then continue
    let (_, next) ← (← goal.assert `certifiedBounds boundsType bounds).intro1P
    goal := next
  pure goal

/-- Prove `statement`, whose weak-head normal form is expected to be an
arithmetic proposition, an `Option`-wrapped decidable test such as the
range condition of a certified-integer decoder, or a closed reflexivity.
Returns the proof or fails. -/
private def proveCondition (statement : Lean.Expr) : TacticM Lean.Expr := do
  /- The condition may be a certificate already in context — a decoded
  value's own range certificate, say.  Matched syntactically: unifying
  against every hypothesis would unfold execution equations. -/
  for declaration in ← getLCtx do
    if declaration.isImplementationDetail then continue
    if (← instantiateMVars declaration.type) == statement then
      return mkFVar declaration.fvarId
  /- A certified integer's value carries its own certificate. -/
  if statement.isAppOfArity ``LeanerIR.IntegerValueFits 3 then
    let value := statement.getArg! 2
    if value.isAppOfArity ``LeanerIR.SpecInt.val 3 then
      let certificate ← mkAppM ``LeanerIR.SpecInt.fits #[value.getArg! 2]
      if ← isDefEq (← inferType certificate) statement then
        return certificate
  /- A proof of the reduced statement is a proof of the original by
  definitional equality, and reducing first turns a range predicate such
  as `IntegerValueFits` into its defining equation and that equation's
  sides into their `decide` cores. -/
  let headReduced ← whnf statement
  let reduced ← match headReduced.eq? with
    | some (_, lhs, rhs) => do mkEq (← whnf lhs) (← whnf rhs)
    | none => pure headReduced
  let m ← mkFreshExprMVar reduced
  /- The arithmetic reads the bounds of the certificates in context,
  exposed on the condition's own goal: the caller's goal is untouched
  when the condition cannot be established. -/
  let bounded ← exposeBounds m.mvarId!
  let closed ← solvedBy bounded <| evalTactic <| ← `(tactic|
    first
    | rfl
    | omega
    | (simp only [Option.some.injEq, Bool.and_eq_true, decide_eq_true_eq,
        Bool.decide_and]
       omega))
  unless closed do
    throwError "certified closing: cannot establish condition {statement} reduced to {reduced}"
  instantiateMVars m

/-- Reduce every structure-projection application in `e` whose reduction
makes progress, recursively, leaving class methods and everything else at
its authored spelling.  This restores one spelling per value: a codec
equation speaks about `(encode ⟨v, h⟩)` and a range fact about `v`, and a
decision procedure needs them to be the same atom. -/
private partial def normalizeProjections (e : Lean.Expr) : MetaM Lean.Expr :=
  Meta.transform e (post := fun e => do
    /- Unification can leave a beta redex where a value belongs; it is one
    `whnfCore` from its payload. -/
    if e.getAppFn.isLambda then
      let reduced ← whnfCore e
      if reduced != e then
        return .done (← normalizeProjections reduced)
    /- A primitive projection out of a literal is that field; one out of a
    variable is respelled as the named projection the facts use. -/
    if let .proj structName index owner := e then
      if ← Meta.isConstructorApp (← whnf owner) then
        let reduced ← whnfCore e
        if reduced != e then
          return .done (← normalizeProjections reduced)
      else if let some info := getStructureInfo? (← getEnv) structName then
        if let some field := info.fieldNames[index]? then
          return .done (← Meta.mkProjection owner field)
    if let .const name _ := e.getAppFn then
      if let some info ← getProjectionFnInfo? name then
        unless info.fromClass do
          /- Only a projection out of a literal is a respelling of a
          value; one applied to a variable merely unfolds to the primitive
          projection, a different atom from the facts' authored one. -/
          if let some owner := e.getAppArgs[info.numParams]? then
            if ← Meta.isConstructorApp (← whnf owner) then
              /- Unfold the projection function to its primitive projection
              and reduce that: the field is respelled, and its own head —
              an integer sum, say — is left at its authored spelling. -/
              if let some unfolded ← Meta.unfoldDefinition? e then
                let reduced ← whnfCore unfolded
                if reduced != e then
                  return .done (← normalizeProjections reduced)
    return .done e)

/-- The hypothesis at context position `index` in `goal`.  Operations such
as `changeLocalDecl` re-introduce a hypothesis under a fresh `FVarId`, so
position is the only identity that survives them. -/
private def hypothesisAt (goal : MVarId) (index : Nat) : MetaM FVarId :=
  goal.withContext do
    for declaration in ← getLCtx do
      if declaration.index == index then
        return declaration.fvarId
    throwError "certified closing: no hypothesis at context position {index}"

/-- Substitute or decompose the equation `fvarId` until nothing more
follows from it: substitution when one side is a variable, injection when
both sides are applications of one constructor. The recursion mirrors what
the drive does to execution equations, on the small context of a refuted
leaf. -/
private partial def consumeEquation (goal : MVarId) (fvarId : FVarId)
    (fuel : Nat) : MetaM (Option MVarId) := do
  match fuel with
  | 0 => return some goal
  | fuel + 1 =>
    /- Normalize the equation's sides so literals expose the constructors
    injection needs; `replaceLocalDeclDefEq` keeps the `FVarId`, which
    `changeLocalDecl` would not. -/
    let goal ← try
      let ty ← goal.withContext do instantiateMVars (← fvarId.getType)
      match ty.eq? with
      | some (_, lhs, rhs) => goal.withContext do
          /- `whnf` respells a literal array as an `Array.mk` record; fold
          it back so one spelling reaches every later rewrite — the
          two-spellings hazard the audit records as F3. -/
          let respell (e : Lean.Expr) : MetaM Lean.Expr :=
            Meta.transform e (post := fun e => do
              if e.isAppOfArity ``Array.mk 2 then
                return .done (← mkAppM ``List.toArray #[e.appArg!])
              return .done e)
          let lhs ← respell (← normalizeProjections (← whnf lhs))
          let rhs ← respell (← normalizeProjections (← whnf rhs))
          goal.replaceLocalDeclDefEq fvarId (← mkEq lhs rhs)
      | none => pure goal
    catch _ => pure goal
    try
      let (_, substituted) ← substCore goal fvarId (symm := true)
      return some substituted
    catch _ =>
    try
      let (_, substituted) ← substCore goal fvarId
      return some substituted
    catch _ =>
    try
      match ← injection goal fvarId with
      | .solved => return none
      | .subgoal subgoal fvarIds _ =>
          let mut current := subgoal
          for fvarId in fvarIds do
            match ← consumeEquation current fvarId fuel with
            | some next => current := next
            | none => return none
          return some current
    catch _ => return some goal

/-- The family-representation facts in context: each says what the map
reads at every key of one family, at one map.  A storage leaf is
normalized through exactly these — a `simp only` with named hypotheses,
never a search — after which the read is the typed contents. -/
private def representationFacts (goal : MVarId) : MetaM (Array FVarId) :=
  goal.withContext do
    let mut facts := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      let ty ← instantiateMVars declaration.type
      let isRepresentation :=
        ty.isAppOfArity ``LeanerIR.FamilyRepresentation 6 ||
        (ty.isForall && (ty.bindingBody!.eq?.any fun (_, lhs, _) =>
          lhs.isAppOfArity ``LeanerIR.GlobalMap.lookup 2))
      /- A presence equation `contents k = some c` on a family binder is
      the same kind of fact: what the typed contents hold at one key. -/
      let isPresence ← match ty.eq? with
        | some (_, lhs, _) =>
            if lhs.isApp && lhs.appFn!.isFVar then do
              let binderType ← instantiateMVars (← inferType lhs.appFn!)
              pure (binderType.isArrow &&
                binderType.bindingDomain!.isConstOf ``LeanerIR.StorageKey)
            else pure false
        | none => pure false
      if isRepresentation || isPresence then
        facts := facts.push declaration.fvarId
    pure facts

/- A read at the written key of a map no fact represents — a lender's
export carrying a hole — resolves through the map law itself, after the
facts had their chance: the law sits below them in the inventory. -/
attribute [lir_data_norm low] GlobalMap.lookup_insert_self GlobalMap.lookup_erase_self

/-- The closed evaluations of the write-back resolution: a callee's
returned reborrows fill the lenders' holes, whole or at a focus whose
siblings are loan-free. -/
syntax "leaner_resolve_rows" (" at " "*")? : tactic

macro_rules
  | `(tactic| leaner_resolve_rows) =>
      `(tactic| try simp (disch := (first | omega | leaner_plain)) only
        [LeanerIR.SemanticOperations.resolveReturnedBorrows_empty,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_integer,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedBorrow,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedBorrow_integer,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedBorrow_focus,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedPair_left,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedPair_right])
  | `(tactic| leaner_resolve_rows at *) =>
      `(tactic| try simp (disch := (first | omega | leaner_plain)) only
        [LeanerIR.SemanticOperations.resolveReturnedBorrows_empty,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_integer,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedBorrow,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedBorrow_integer,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedBorrow_focus,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedPair_left,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedPair_right] at *)

/-- Normalize the storage reads of `goal` through the representation
facts: the runtime lookups become typed contents, pointwise updates at
the read key resolve, and the keyed map laws settle reads at other keys
whose disequality is in context. -/
private def normalizeStorage (goal : MVarId) (everywhere : Bool := false) :
    TacticM MVarId := do
  let facts ← representationFacts goal
  if facts.isEmpty then return goal
  /- A folded representation fact is an atom to the simplifier; only its
  unfolded `∀ key, lookup … = …` form rewrites.  Unfold in place, keeping
  the hypothesis's identity. -/
  let mut goal := goal
  for fact in facts do
    let ty ← goal.withContext do instantiateMVars (← fact.getType)
    if ty.isAppOfArity ``LeanerIR.FamilyRepresentation 6 then
      let unfolded ← goal.withContext do whnf ty
      goal ← goal.replaceLocalDeclDefEq fact unfolded
  let lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ←
    goal.withContext do
      facts.mapM fun fvarId => do
        let name := mkIdent (← fvarId.getDecl).userName
        `(Lean.Parser.Tactic.simpLemma| $name:ident)
  let saved ← saveState
  try
    setGoals [goal]
    /- The closed resolve rows run first: the data inventory unfolds
    `resolveReturnedBorrows` eagerly, which would bury a lender's
    resolution under the fold before the rows could read it. -/
    if everywhere then
      evalTactic <| ← `(tactic| leaner_resolve_rows at *)
    else
      evalTactic <| ← `(tactic| leaner_resolve_rows)
    /- A refutation's material is in the context, so there the reads are
    normalized in every hypothesis; a leaf target is normalized alone. -/
    if everywhere then
      evalTactic <| ← `(tactic|
        simp (disch := leaner_denotation_discharge) only
          [$lemmas,*, LeanerIR.FamilyRepresentation,
           LeanerIR.updateContents, LeanerIR.RuntimeValue.storageKey,
           LeanerIR.RuntimeValue.storageKey?, Option.getD_some,
           Option.bind_some, Option.map_none, Option.bind_none, Option.getD_none,
           Option.isSome_none, Option.isSome_some, Option.isSome_map,
           Bool.false_eq_true,
           Bool.true_eq_false, not_true_eq_false, not_false_eq_true,
           GlobalMap.lookup_insert_other, GlobalMap.lookup_erase_other, lir_data_norm,
           ite_true, ite_false, reduceIte, eq_self_iff_true] at *)
    else
      evalTactic <| ← `(tactic|
        simp (disch := leaner_denotation_discharge) only
          [$lemmas,*, LeanerIR.FamilyRepresentation,
           LeanerIR.updateContents, LeanerIR.RuntimeValue.storageKey,
           LeanerIR.RuntimeValue.storageKey?, Option.getD_some,
           Option.bind_some, Option.map_none, Option.bind_none, Option.getD_none,
           Option.isSome_none, Option.isSome_some, Option.isSome_map,
           Bool.false_eq_true,
           Bool.true_eq_false, not_true_eq_false, not_false_eq_true,
           GlobalMap.lookup_insert_other, GlobalMap.lookup_erase_other, lir_data_norm,
           ite_true, ite_false, reduceIte, eq_self_iff_true])
    match ← getGoals with
    | [normalized] => pure normalized
    | [] => pure goal
    | _ => saved.restore; pure goal
  catch _ =>
    saved.restore
    pure goal

/-- Refute the hypotheses of `goal`, whose target is `False`: destructure
every existential, conjunction, and disjunction that entered with the
refuted proposition, consume the resulting equations, and hand the
arithmetic core to `omega`.

Hypotheses are identified by context position, not identity — substitution
rebuilds the context, so a held `FVarId` goes stale the moment an equation
is consumed.  Everything at or after the refuted hypothesis's position is
this closing's material; the verification preamble sits before it and is
never walked. -/
private partial def refute (goal : MVarId) (root : FVarId) : TacticM Unit := do
  let baseline ← goal.withContext do
    return (← root.getDecl).index
  let rec work (goal : MVarId) (skip : List Nat) : TacticM Unit := do
    /- A callee's clause arrives under its `Obligation` marker, which is
    sealed; the marked proposition is the material, and the hypothesis is
    given that type before the walk reads it. -/
    let marked? ← goal.withContext do
      for declaration in ← getLCtx do
        if declaration.isImplementationDetail then continue
        if declaration.index < baseline then continue
        let ty ← instantiateMVars declaration.type
        if ty.isAppOfArity ``Obligation 3 then
          return some (declaration.fvarId, ty.getArg! 2)
      return none
    if let some (fvarId, inner) := marked? then
      return ← work (← goal.replaceLocalDeclDefEq fvarId inner) skip
    let next? ← goal.withContext do
      for declaration in ← getLCtx do
        if declaration.isImplementationDetail then continue
        if declaration.index < baseline then continue
        if skip.contains declaration.index then continue
        let ty ← whnf (← instantiateMVars declaration.type)
        if ty.isAppOf ``Exists || ty.isAppOf ``And || ty.isAppOf ``Or ||
            ty.isAppOf ``Eq || ty.isConstOf ``False then
          return some (declaration.fvarId, declaration.index, ty)
      return none
    match next? with
    | none =>
        let goal ← normalizeStorage goal (everywhere := true)
        /- A `False` hypothesis closes the goal inside the normalization. -/
        if ← goal.isAssigned then return
        let goal ← exposeBounds goal
        /- A refuted inequality arrives head-reduced — `Nat.succ (Nat.add n 0)`
        for `n + 1` — which `omega` does not read; respell it. -/
        let closed ← solvedBy goal <| evalTactic <| ← `(tactic|
          first
          | omega
          | (simp (disch := omega) only [Int.ofNat_eq_natCast,
               Int.tdiv_eq_ediv_of_nonneg, Int.tmod_eq_emod_of_nonneg,
               Nat.succ_eq_add_one, Nat.add_eq, Nat.add_zero] at *
             omega))
        unless closed do
          throwError m!"certified closing: refutation did not reach a contradiction\n{goal}"
    | some (fvarId, index, ty) =>
        /- A clause that unfolds to `False` — an absent abort condition —
        refutes itself. -/
        if ty.isConstOf ``False then
          goal.withContext do
            goal.assign (← mkAppOptM ``False.elim #[← goal.getType, mkFVar fvarId])
        else if ty.isAppOf ``Eq then
          let some after ← consumeEquation goal fvarId 8
            | return
          if after == goal then
            work goal (index :: skip)
          else
            /- Substitution renumbered the context; earlier skips keep
            their meaning because everything they named sat before the
            consumed pair. -/
            work after skip
        else
          /- A conjunction or existential opens into one goal; a
          disjunction — the abort clauses of a refuted failure — into one
          per clause, each refuted on its own. -/
          for sub in ← goal.cases fvarId do
            work sub.mvarId skip
  work goal []

/-- Reduce the head of `e` just far enough to expose a decoder's `dite`,
without reducing the `dite` itself — plain `whnf` would unfold it into
`Decidable.rec` and lose the shape the rewrite needs.  Iota and beta run
freely; delta runs one head definition at a time and stops as soon as a
`dite` appears anywhere on the spine. -/
private partial def exposeHead (e : Lean.Expr) (fuel : Nat := 32) :
    MetaM Lean.Expr := do
  if fuel == 0 then return e
  if e.isAppOf ``dite || e.isAppOf ``ite then return e
  let core ← whnfCore e
  if core.isAppOf ``dite || core.isAppOf ``ite then return core
  if core != e then
    exposeHead core (fuel - 1)
  else
    match ← unfoldDefinition? e with
    | some unfolded => exposeHead unfolded (fuel - 1)
    | none =>
      /- The head is stuck — typically a `match` or projection whose
      scrutinee is itself a folded codec application.  Expose the first
      argument that makes progress and resume at the rebuilt term, so a
      `dite` buried under an `Option.bind` match becomes visible without
      reducing anything beyond the spine that hides it. -/
      let arguments := e.getAppArgs
      for i in [0:arguments.size] do
        let exposed ← exposeHead arguments[i]! (fuel - 1)
        if exposed != arguments[i]! then
          return ← exposeHead
            (mkAppN e.getAppFn (arguments.set! i exposed)) (fuel - 1)
      return e

/-- One targeted `dite` elimination: find a branch whose condition holds by
`proveCondition`, rewrite it away with `dif_pos`, and return the new goal.
Fails when the target has no closed `dite`. -/
private def resolveDite (goal : MVarId) : TacticM MVarId := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let some dite := target.find? fun e =>
      e.isAppOf ``dite && e.getAppNumArgs == 5 && !e.hasLooseBVars
    | throwError "certified closing: no closed dite to resolve in {target}"
  goal.withContext do
    let condition := dite.getAppArgs[1]!
    let conditionProof ← proveCondition condition
    /- State the proof at the unreduced condition so the rewrite matches
    the goal syntactically; the two types are definitionally equal.  The
    `Decidable` instance is the dite's own — the range condition is a
    proposition with no synthesizable instance. -/
    let stated ← mkFreshExprMVar condition
    stated.mvarId!.assign conditionProof
    let arguments := dite.getAppArgs
    let equation ← mkAppOptM ``dif_pos
      #[some condition, some arguments[2]!, some stated,
        some arguments[0]!, some arguments[3]!, some arguments[4]!]
    let result ← goal.rewrite target equation
    let goal ← goal.replaceTargetEq result.eNew result.eqProof
    pure goal

/-- Expose the head of the equation's left side in place. -/
private def exposeEquationHead (goal : MVarId) : TacticM MVarId :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let some (_, lhs, rhs) := target.eq?
      | throwError "certified closing: not an equation"
    let exposed ← exposeHead lhs
    /- Only a `dite` is worth exposing; a head that reduces to something
    else — an integer sum into its `match` — is left at its authored
    spelling, which is what the arithmetic leaf reads. -/
    let hasDite := (exposed.find? fun e => e.isAppOf ``dite).isSome
    if exposed == lhs || !hasDite then pure goal
    else goal.change (← mkEq exposed rhs) (checkDefEq := false)

/-- A decoder equation resolves through its range `dite`.  The attempt
precedes the storage normalization: that normalization unfolds the range
condition into its bounds arithmetic, after which the `dite` is no longer
the decoder's.  An equation over a semantic operation applied to a
symbolic state resolves through the closed reconcile rows instead — and
applying them here, to one conjunct, is bounded by that conjunct's size,
unlike the search closing this replaces, which applied them to the whole
obligation at once. -/
private def resolveDite? (goal : MVarId) : TacticM (MVarId × Bool) := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let some (_, lhs, rhs) := target.eq?
    | throwError "certified closing: not an equation"
  /- The head is exposed only when that shows a `dite`: a storage read's
  authored spelling is what the representation facts rewrite, and must
  survive an attempt that finds nothing.  The goal handed back is the
  live one in either case — a goal `change` leaves behind is assigned to
  its replacement, and continuing on it would drop that replacement. -/
  /- A decoder equation is recognized by its decoder before any reduction:
  exposing the head of a storage read or an export equation would pay a
  reduction for nothing. -/
  let mentionsDecoder := (lhs.find? fun e =>
    e.isConstOf ``LeanerIR.Proofs.Codec.decode? || e.isConstOf ``LeanerIR.decodeInt? ||
    e.isAppOf ``dite).isSome
  unless mentionsDecoder do return (goal, false)
  let exposed ← goal.withContext do exposeHead lhs
  let hasDite := (exposed.find? fun e =>
    e.isAppOf ``dite && e.getAppNumArgs == 5 && !e.hasLooseBVars).isSome
  unless hasDite do return (goal, false)
  let goal ← if exposed == lhs then pure goal
    else goal.change (← mkEq exposed rhs) (checkDefEq := false)
  try pure (← resolveDite goal, true)
  catch _ => pure (goal, false)

/-- The arithmetic leaf: bounded integer reasoning over the context, with
`Bool` facts propositionalized on the leaves plain `omega` missed, and a
certified integer's bounds exposed only on the leaves those missed too. -/
def arithmeticLeaf (goal : MVarId) : TacticM Bool := do
  /- A value spelled as a structure-literal projection is the same atom
  as the projected variable the facts use only after core normalization. -/
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let normalized ← goal.withContext do normalizeProjections target
  let goal ← if normalized == target then pure goal
    else goal.change normalized (checkDefEq := false)
  /- A fact already in context is matched syntactically, never by
  unification: `assumption` would try `isDefEq` against every hypothesis,
  and unfolding one mismatched contract or execution equation costs
  seconds. -/
  let matched ← goal.withContext do
    let mut found : Option FVarId := none
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      if found.isSome then break
      if (← normalizeProjections (← instantiateMVars declaration.type)) == normalized then
        found := some declaration.fvarId
    pure found
  if let some fvarId := matched then
    goal.assign (mkFVar fvarId)
    return true
  let closed ← solvedBy goal <| evalTactic <| ← `(tactic|
    first
    | rfl
    | omega
    | (simp only [Bool.and_eq_true, decide_eq_true_eq, decide_eq_false_iff_not,
         Int.ofNat_eq_natCast] at *
       omega))
  if closed then return true
  let goal ← exposeBounds goal
  /- A clause that selects by a condition the path decided — `if value == 0
  then 1 else 2` under a guard on that decision — splits on the goal's
  conditional, each side decided against the guard. -/
  solvedBy goal <| evalTactic <| ← `(tactic|
    first
    | omega
    | (simp (disch := omega) only [Bool.and_eq_true, decide_eq_true_eq,
         decide_eq_false_iff_not, Int.ofNat_eq_natCast, Int.tdiv_eq_ediv_of_nonneg,
         Int.tmod_eq_emod_of_nonneg] at *
       omega)
    | (simp only [Bool.and_eq_true, decide_eq_true_eq, decide_eq_false_iff_not,
         Int.ofNat_eq_natCast] at *
       split <;> omega))

/-- Whether the path this goal sits on is unreachable: its branch guards
and range facts contradict under bounded arithmetic. -/
def unreachable (goal : MVarId) : TacticM Bool := do
  /- Exposing the bounds asserts into the goal; a failed attempt must
  leave the goal as it was, for the report that follows. -/
  let saved ← saveState
  /- A clause's own antecedents — an implication guarded by a `Bool`
  argument the path fixed — are part of what contradicts, so the marker
  is opened for them to be introduced. -/
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  let goal ← if target.isAppOfArity ``Obligation 3 then
      goal.change (target.getArg! 2) (checkDefEq := false)
    else pure goal
  let bounded ← exposeBounds goal
  let closed ← solvedBy bounded <| evalTactic <| ← `(tactic|
    (intros
     exfalso
     simp (disch := omega) only [Bool.and_eq_true, decide_eq_true_eq, Bool.not_eq_true,
       Bool.false_eq_true, Bool.true_eq_false,
       decide_eq_false_iff_not, not_and, Int.not_le, Int.not_lt, Nat.not_le, Nat.not_lt,
       Int.ofNat_eq_natCast, Int.tdiv_eq_ediv_of_nonneg, Int.tmod_eq_emod_of_nonneg] at *
       <;> omega))
  unless closed do saved.restore
  pure closed

/-- Byte ranges of the `Obligation` markers in an expression. -/
partial def obligationRanges (expression : Lean.Expr) : Array (Nat × Nat) :=
  let rec walk (e : Lean.Expr) (found : Array (Nat × Nat)) : Array (Nat × Nat) :=
    let found :=
      if e.isAppOfArity ``Obligation 3 then
        match ((e.getArg! 0).nat? <|> (e.getArg! 0).rawNatLit?),
            ((e.getArg! 1).nat? <|> (e.getArg! 1).rawNatLit?) with
        | some startByte, some endByte => found.push (startByte, endByte)
        | _, _ => found
      else found
    match e with
    | .app function argument => walk argument (walk function found)
    | .lam _ type body _ => walk body (walk type found)
    | .forallE _ type body _ => walk body (walk type found)
    | .letE _ type value body _ => walk body (walk value (walk type found))
    | .mdata _ body => walk body found
    | .proj _ _ body => walk body found
    | _ => found
  walk expression #[]

/-- Show the goal of every clause the certified closing cannot establish,
beside the clause report. -/
register_option leaner.certifyDebug : Bool := {
  defValue := false
  descr := "show the residual goal of a specification clause the certified closing cannot establish"
}

/-- Report a verification condition that was not established, at every
authored clause its `Obligation` markers locate — or, with no marker, as a
residual obligation with its goal. -/
def reportObligation (goal : MVarId) : TacticM Unit := do
  let fileMap ← getFileMap
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let ranges := (obligationRanges target).toList.eraseDups
    if ranges.isEmpty then
      logError m!"verification failed with a residual obligation:\n{(← Lean.Meta.ppGoal goal)}"
    else
      for (startByte, endByte) in ranges do
        let snippet := Substring.Raw.toString
          ⟨fileMap.source, ⟨startByte⟩, ⟨endByte⟩⟩
        let reference := Syntax.atom (.synthetic ⟨startByte⟩ ⟨endByte⟩) snippet
        logErrorAt reference
          m!"the specification clause `{snippet}` is not established"
      if leaner.certifyDebug.get (← getOptions) then
        logError m!"residual obligation:\n{(← Lean.Meta.ppGoal goal)}"

/-- Rewrite a hypothesis spelling a storage key through `storageKey` into
the constructor spelling the runtime's map laws use. -/
private def normalizeKeySpelling (goal : MVarId) (fvarId : FVarId) : TacticM MVarId := do
  let mentionsKey ← goal.withContext do
    let ty ← instantiateMVars (← fvarId.getType)
    pure (ty.find? (·.isConstOf ``LeanerIR.RuntimeValue.storageKey)).isSome
  unless mentionsKey do return goal
  let name ← goal.withContext do pure (← fvarId.getDecl).userName
  setGoals [goal]
  let hypothesis := mkIdent name
  evalTactic <| ← `(tactic| try simp only [LeanerIR.RuntimeValue.storageKey,
    LeanerIR.RuntimeValue.storageKey?, Option.getD_some] at $hypothesis:ident)
  match ← getGoals with
  | [goal] => pure goal
  | _ => throwError "certified closing: normalizing a key hypothesis lost the goal"

/-- The resolution rows over a goal: the goal they leave, or none when
they close it.  A goal without a resolution is left alone: every row is
keyed on the definition, and the traversal is not free. -/
private def resolveRows (goal : MVarId) : TacticM (Option MVarId) := do
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  unless (target.find?
      (·.isConstOf ``LeanerIR.SemanticOperations.resolveReturnedBorrows)).isSome do
    return some goal
  setGoals [goal]
  evalTactic <| ← `(tactic| leaner_resolve_rows)
  match ← getGoals with
  | [] => pure none
  | [goal] => pure (some goal)
  | _ => throwError "certified closing: the resolution rows split the goal"

mutual

/-- Close a target of the obligation grammar, or fail.

The target is head-normalized first, so the folded contract surface —
generated contract projections, codec applications, `Obligation`, `Not` —
exposes the connective underneath without any rewriting pass over the
goal.  Head reduction touches only the spine; sealed twin decoders stay
folded, and a shape they block falls back to the search closing. -/
private partial def close (deferred : IO.Ref (Array MVarId)) (tentative : IO.Ref Nat)
    (goal : MVarId) : TacticM Unit := do
  if ← goal.isAssigned then return
  /- Metadata a script's `simp` leaves around a target hides its head from
  the shape tests; the connective is recognized through it. -/
  let target₀ ← goal.withContext do
    pure (← instantiateMVars (← goal.getType)).consumeMData
  match ← recognize deferred tentative goal target₀ with
  | some run => run
  | none =>
    /- Storage reads normalize first, through the representation facts in
    context: a leaf that read the map becomes a typed leaf, and may even
    expose grammar (an equation, a conjunction) to re-enter with. -/
    let goal ← normalizeStorage goal
    if ← goal.isAssigned then return
    let target₀ ← goal.withContext do
      pure (← instantiateMVars (← goal.getType)).consumeMData
    match ← recognize deferred tentative goal target₀ with
    | some run => run
    | none =>
    /- A folded contract projection needs one head reduction to show its
    connective.  A shape the reduced form still does not offer is a leaf,
    and the leaf is tried at the original spelling: reduction turns an
    integer comparison into `Int.NonNeg`, which `omega` does not read. -/
    let target ← goal.withContext do whnf target₀
    let goal ← if target == target₀ then pure goal
      else goal.change target (checkDefEq := false)
    let attempt ← if target == target₀ then pure none
      else recognize deferred tentative goal target
    match attempt with
    | some run => run
    | none =>
      if target₀.hasExprMVar then
        deferred.modify (·.push goal)
        return
      /- A witness determined by a codec equation arrives spelled as a
      structure-literal projection, while the drive's range facts use the
      projected variable itself.  Reducing every subterm to its core
      normal form restores one spelling per value, so the decision
      procedure sees the same atoms the facts use. -/
      let normalized ← goal.withContext do
        normalizeProjections target₀
      /- Compared against the goal's current spelling, not `target₀`: when
      the head reduction above rewrote the goal (an integer comparison
      whose reduced form is `Int.NonNeg`) and normalization returns the
      original unchanged, the goal still must be restored to that original
      spelling for the decision procedure to read it. -/
      let goal ← if normalized == target then pure goal
        else goal.change normalized (checkDefEq := false)
      let closed ← arithmeticLeaf goal
      unless closed do
        goal.withContext do
          throwError "certified closing: unsupported obligation shape {normalized}"

/-- The handler for a recognized obligation connective, at this exact
target spelling; `none` when the shape offers no structure. -/
private partial def recognize (deferred : IO.Ref (Array MVarId)) (tentative : IO.Ref Nat)
    (goal : MVarId) (target : Lean.Expr) :
    TacticM (Option (TacticM Unit)) := do
  if target.isConstOf ``True then
    return some (goal.assign (mkConst ``True.intro))
  else if target.isAppOfArity ``Obligation 3 then
    return some do
      /- A clause the closing cannot establish is reported at its authored
      range and admitted, so the script's remaining branches run on the
      goals they expect; the elaborating command fails on the report.  The
      attempt's assignments and deferrals are rolled back first. -/
      if (← tentative.get) > 0 then
        /- Under an alternative that may still backtrack, a failure is the
        alternative's to handle. -/
        close deferred tentative (← goal.change (target.getArg! 2) (checkDefEq := false))
      else
        let saved ← saveState
        let deferredBefore ← deferred.get
        try
          close deferred tentative (← goal.change (target.getArg! 2) (checkDefEq := false))
        catch failure =>
          if leaner.certifyDebug.get (← getOptions) then
            IO.eprintln s!"clause attempt failed: {← failure.toMessageData.toString}"
          saved.restore
          deferred.set deferredBefore
          /- A clause under an unreachable branch is vacuous: the branch's
          guard and the range facts of the path contradict.  Bounded
          arithmetic over the context decides that; only then is the
          clause reported. -/
          unless ← unreachable goal do
            reportObligation goal
            goal.admit
  else if target.isAppOfArity ``And 2 then
    return some do
      let subgoals ← goal.apply (mkConst ``And.intro)
      for subgoal in subgoals do
        close deferred tentative subgoal
  else if target.isAppOfArity ``Exists 2 then
    return some (closeExists deferred tentative goal)
  else if target.isAppOfArity ``Not 1 || target.isArrow then
    return some do
      let (fvarId, opened) ← goal.intro1P
      let inner ← opened.withContext do instantiateMVars (← opened.getType)
      if inner.isConstOf ``False then
        refute opened fvarId
      else
        /- A frame's key hypothesis arrives spelled through the twin's key
        function; the map laws match it against the runtime's constructor
        spelling, so it is given that one spelling here. -/
        let opened ← normalizeKeySpelling opened fvarId
        close deferred tentative opened
  else if target.isForall then
    /- A frame clause quantifies over the keys it does not modify;
    introduce the key and continue. -/
    return some do
      let (_, opened) ← goal.intro1P
      close deferred tentative opened
  else if target.isAppOfArity ``Eq 3 then
    return some (closeEquation goal)
  else if target.isAppOfArity ``Or 2 then
    /- A contract with several `aborts_if` clauses states its abort
    condition as their disjunction; the branch the current path
    establishes is one of two, a bounded choice. -/
    return some do
      let saved ← saveState
      let attempted ← try
        tentative.modify (· + 1)
        let subgoals ← goal.apply (mkConst ``Or.inl)
        for subgoal in subgoals do
          close deferred tentative subgoal
        tentative.modify (· - 1)
        pure true
      catch _ =>
        tentative.modify (· - 1)
        pure false
      unless attempted do
        saved.restore
        let subgoals ← goal.apply (mkConst ``Or.inr)
        for subgoal in subgoals do
          close deferred tentative subgoal
  else if target.isAppOfArity ``SemanticOperations.LoanDiscipline 2 then
    /- The loan-discipline clause of a generated frame is closed through
    its two intro lemmas, never by walking its own conjunction: the
    registry either returns to its entry spelling or carries exactly the
    one returned registration, and each shape is one application with
    reflexivity-or-arithmetic side goals. -/
    return some do
      let closed ← solvedBy goal <| evalTactic <| ← `(tactic|
        first
        | assumption
        | exact LeanerIR.SemanticOperations.LoanDiscipline.of_eq rfl
            (Nat.le_refl _)
        | (apply LeanerIR.SemanticOperations.LoanDiscipline.of_eq <;>
            first | rfl | omega | (dsimp only; omega))
        | (apply LeanerIR.SemanticOperations.LoanDiscipline.of_registered <;>
            first | rfl | omega | (dsimp only; omega))
        | (apply LeanerIR.SemanticOperations.LoanDiscipline.of_transfer
           case loans_eq => rfl
           case transfer_eq =>
             simp [LeanerIR.SemanticOperations.transferredLoan?,
               LeanerIR.SemanticOperations.findFirst,
             LeanerIR.SemanticOperations.findFirstList]
             try rfl
           all_goals first | rfl | omega | (dsimp only; omega)))
      unless closed do
        throwError "certified closing: loan discipline not established"
  else
    return none

/-- Close an existential: introduce a witness metavariable and let the
first determining equation assign it. -/
private partial def closeExists (deferred : IO.Ref (Array MVarId)) (tentative : IO.Ref Nat)
    (goal : MVarId) : TacticM Unit := do
  /- The introduction is built directly rather than applied: unifying the
  goal against `Exists ?p` reaches into the body with definitional
  unfolding, and a sealed operation the rows must read — a lender's
  resolution — would arrive at its leaf unfolded.  The witness is a
  natural metavariable, for the first determining equation to assign;
  proving the body determines it. -/
  let target ← goal.withContext do
    pure (← instantiateMVars (← goal.getType)).consumeMData
  let .app (.app (.const ``Exists levels) type) predicate := target
    | throwError "certified closing: not an existential"
  let (witness, body) ← goal.withContext do
    let witness ← mkFreshExprMVar type
    let body ← mkFreshExprMVar (predicate.beta #[witness]) (kind := .syntheticOpaque)
    goal.assign (mkApp4 (mkConst ``Exists.intro levels) type predicate witness body)
    pure (witness.mvarId!, body.mvarId!)
  close deferred tentative body
  for witness in [witness] do
    unless ← witness.isAssigned do
      /- A witness no equation determines is one the clause does not read;
      any inhabitant serves.  `∃ _ : Unit, True` is the common case, from a
      contract with no results. -/
      try
        let type ← witness.getType
        witness.assign (← mkAppOptM ``Inhabited.default #[some type, none])
      catch _ =>
        throwError "certified closing: an existential witness was not determined"

/-- Close an equation: reflexivity up to reduction, after eliminating any
decoder `dite` whose range condition the context establishes.  The left
side is exposed to its weak head first, so a folded codec application
shows the `dite` this needs to find. -/
private partial def closeEquation (goal : MVarId) : TacticM Unit := do
  if ← solvedBy goal (liftMetaTactic fun g => do g.refl; pure []) then
    return
  /- A lender's resolution under a field read is evaluated first:
  exposing the equation's head unfolds the definition the read is
  applied to, and would bury the resolution under its fold. -/
  let some goal ← resolveRows goal | return
  let (goal, resolved) ← resolveDite? goal
  if resolved then return ← closeEquation goal
  let goal ← normalizeStorage goal
  if ← goal.isAssigned then return
  if ← solvedBy goal (liftMetaTactic fun g => do g.refl; pure []) then
    return
  let (goal, resolved) ← resolveDite? goal
  if resolved then return ← closeEquation goal
  let goal ← exposeEquationHead goal
  /- An equation the drive or a call's contract already established —
  the frame's globals clause after a call, say — is in the context.
  Matched syntactically after projection normalization, never by
  unification: `assumption` would try `isDefEq` against every
  hypothesis, and unfolding one mismatched execution equation costs
  seconds. -/
  let matched ← goal.withContext do
    let target ← normalizeProjections
      (← instantiateMVars (← goal.getType))
    let mut found : Option FVarId := none
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      if found.isSome then break
      let ty ← instantiateMVars declaration.type
      unless ty.isEq do continue
      if (← normalizeProjections ty) == target then
        found := some declaration.fvarId
    pure found
  /- An equation a hypothesis states for every key — a callee's frame
  clause — is instantiated at the goal's, its side condition the
  introduced key hypothesis. -/
  let quantified ← goal.withContext do
    let mut found : Array Name := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      let ty ← instantiateMVars declaration.type
      if ty.isForall && (ty.getForallBody.eq?).isSome then
        found := found.push declaration.userName
    pure found
  for name in quantified do
    let hypothesis := mkIdent name
    if ← solvedBy goal (evalTactic (← `(tactic| (apply $hypothesis:ident <;> assumption)))) then
      return
  if let some fvarId := matched then
    /- The assignment ends the leaf: a `return` inside the context block
    would only leave that block, and the attempts below would then run on
    an assigned goal. -/
    let assigned ← goal.withContext do
      let proof := mkFVar fvarId
      let expected ← goal.getType
      let actual ← inferType proof
      if ← isDefEq expected actual then
        goal.assign proof
        pure true
      else pure false
    if assigned then return
  let saved ← saveState
  let residual ← try
    /- A returned value spelled through its codec — a tuple of encoded
    reborrows — is the runtime value only after projection normalization,
    which the resolution rows below match against.  Inside the saved
    region: a failed attempt must leave the goal unassigned. -/
    let goal ← goal.withContext do
      let target ← instantiateMVars (← goal.getType)
      let normalized ← normalizeProjections target
      if normalized == target then pure goal
      else goal.change normalized (checkDefEq := false)
    setGoals [goal]
    /- Closed evaluations of the write-back resolution go first, in
    their own step: the data inventory also carries the definition as
    an eager unfold, which would otherwise preempt them. -/
    evalTactic <| ← `(tactic|
      (leaner_resolve_rows
       try rfl
       /- A resolution at a focus leaves the focused struct as a literal
       under the read; a bracket's facts are consumed by now. -/
       try (simp (disch := leaner_denotation_discharge) only
         [lir_reconcile, lir_data_norm,
          LeanerIR.SemanticOperations.focusValue_cons,
          LeanerIR.SemanticOperations.focusValue_nil,
          LeanerIR.SemanticOperations.FocusStep.fill, List.push_toArray,
          List.append_toArray, List.cons_append, List.nil_append]; try rfl)))
    getGoals
  catch failure =>
    if leaner.certifyDebug.get (← getOptions) then
      IO.eprintln s!"closeEquation attempt raised: {← failure.toMessageData.toString}"
    saved.restore
    goal.withContext do
      throwError m!"certified closing: equation outside the supported grammar: {← Lean.instantiateMVars (← goal.getType)}"
  /- An equation the rewriting leaves open may still follow from the
  facts in context — a callee's post-state names the slot's new value
  through its own clause — by bounded arithmetic. -/
  let mut open_ := false
  for g in residual do
    if leaner.certifyDebug.get (← getOptions) then
      IO.eprintln s!"closeEquation residual:\n{← g.withContext do pure (← Lean.Meta.ppGoal g)}"
    unless ← arithmeticLeaf g do open_ := true
  if !open_ then
    setGoals []
  else
    saved.restore
    goal.withContext do
      throwError m!"certified closing: equation outside the supported grammar: {← Lean.instantiateMVars (← goal.getType)}"

end

/-- Close the leaves deferred until their witnesses were assigned. -/
private def settleDeferred (deferred : IO.Ref (Array MVarId)) : TacticM Unit := do
  for goal in ← deferred.get do
    unless ← goal.isAssigned do
      let closed ← solvedBy goal <| evalTactic <| ← `(tactic|
        first
        | rfl
        | assumption
        | omega
        | (simp only [Bool.and_eq_true, decide_eq_true_eq, Int.ofNat_eq_natCast] at *
           omega))
      unless closed do
        goal.withContext do
          throwError m!"certified closing: a deferred leaf did not close: {← instantiateMVars (← goal.getType)}"

end Certify

namespace Certify

end Certify

/-- Close the obligations a generated script leaves, by construction.  A
clause that cannot be established is reported at its authored range and
admitted, and the elaborating command fails on the report; a shape outside
the obligation grammar that is not under a clause is an error, since it is
the route's, not the specification's. -/
elab "leaner_certified_close!" : tactic => do
  let deferred ← IO.mkRef #[]
  let tentative ← IO.mkRef 0
  for goal in ← getGoals do
    unless ← goal.isAssigned do
      Certify.close deferred tentative goal
  Certify.settleDeferred deferred
  setGoals []

end LeanerIR.Proofs
