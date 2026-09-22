-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Agreement
import LeanerIR.Proofs.Certify

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

/-- Show the normalized goal before it is split and closed. -/
register_option leaner.denoteDebug : Bool := {
  defValue := false
  descr := "show the normalized verification condition of a denotation before closing"
}

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

/-- The bound of one bit operation, when its operands are certified. -/
private def operationFact? (sub : Lean.Expr) : MetaM (Option Lean.Expr) := do
  if sub.hasLooseBVars then return none
  if sub.isAppOfArity ``LeanerIR.Proofs.IntegerArithmetic.bitwiseAnd 2 then
    match unsignedValue? (sub.getArg! 0), unsignedValue? (sub.getArg! 1) with
    | some (_, left), some (_, right) =>
        return some (← mkAppM ``LeanerIR.Proofs.Denote.BitOp.eval_bounds
          #[mkConst ``LeanerIR.Proofs.Denote.BitOp.and, left, right])
    | _, _ => return none
  if sub.isAppOfArity ``Int.shiftRight 2 then
    match unsignedValue? (sub.getArg! 0) with
    | some (_, value) =>
        return some (← mkAppM ``LeanerIR.Proofs.Denote.shiftRight_bounds #[value, sub.getArg! 1])
    | none => return none
  if sub.isAppOfArity ``HMul.hMul 6 && (sub.getArg! 0).isConstOf ``Int then
    match unsignedValue? (sub.getArg! 4), unsignedValue? (sub.getArg! 5) with
    | some (_, left), some (_, right) =>
        let leftNonnegative ← mkAppM ``And.left #[← mkAppM ``LeanerIR.SpecInt.unsigned_bounds #[left]]
        let rightNonnegative ← mkAppM ``And.left #[← mkAppM ``LeanerIR.SpecInt.unsigned_bounds #[right]]
        return some (← mkAppM ``Int.mul_nonneg #[leftNonnegative, rightNonnegative])
    | _, _ => return none
  if sub.isAppOfArity ``Int.tdiv 2 || sub.isAppOfArity ``Int.tmod 2 then
    match unsignedValue? (sub.getArg! 0), unsignedValue? (sub.getArg! 1) with
    | some (_, left), some (_, right) =>
        let leftNonnegative ← mkAppM ``And.left #[← mkAppM ``LeanerIR.SpecInt.unsigned_bounds #[left]]
        let rightNonnegative ← mkAppM ``And.left #[← mkAppM ``LeanerIR.SpecInt.unsigned_bounds #[right]]
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
    match unsignedValue? (sub.getArg! 0) with
    | some (_, value) =>
        let bounds ← mkAppM ``LeanerIR.SpecInt.unsigned_bounds #[value]
        let nonnegative ← mkAppM ``And.left #[bounds]
        return some (← mkAppM ``LeanerIR.Proofs.Denote.shiftLeft_nonneg
          #[sub.getArg! 0, sub.getArg! 1, nonnegative])
    | none => return none
  return none

/-- The bit operations a proposition mentions. -/
private partial def operationSites (e : Lean.Expr) : Array Lean.Expr :=
  let here := if e.isAppOfArity ``LeanerIR.Proofs.IntegerArithmetic.bitwiseAnd 2 ||
      e.isAppOfArity ``Int.shiftRight 2 || e.isAppOfArity ``Int.shiftLeft 2 ||
      e.isAppOfArity ``Int.tdiv 2 || e.isAppOfArity ``Int.tmod 2 ||
      e.isAppOfArity ``HMul.hMul 6 then #[e] else #[]
  match e with
  | .app f a => here ++ operationSites f ++ operationSites a
  | .lam _ t b _ | .forallE _ t b _ => here ++ operationSites t ++ operationSites b
  | .letE _ t v b _ => here ++ operationSites t ++ operationSites v ++ operationSites b
  | .mdata _ b | .proj _ _ b => here ++ operationSites b
  | _ => here

/-- The closed applications of structure projections a proposition mentions. -/
private partial def projectionSites (env : Environment) (e : Lean.Expr) : Array Lean.Expr :=
  let here := match e.getAppFn with
    | .const name _ =>
        if e.isApp && !e.hasLooseBVars && (env.getProjectionFnInfo? name).isSome then #[e] else #[]
    | _ => #[]
  match e with
  | .app f a => here ++ projectionSites env f ++ projectionSites env a
  | .lam _ t b _ | .forallE _ t b _ => here ++ projectionSites env t ++ projectionSites env b
  | .letE _ t v b _ => here ++ projectionSites env t ++ projectionSites env v ++ projectionSites env b
  | .mdata _ b | .proj _ _ b => here ++ projectionSites env b
  | _ => here

/-- The facts a leaf needs beside its hypotheses: the bounds of every
certified integer in context, and the bounds of every bit operation on
them that the goal or a hypothesis mentions. -/
elab "leaner_denote_bounds" : tactic => do
  if (← getGoals).isEmpty then return
  let goal ← getMainGoal
  let facts ← goal.withContext do
    let mut facts : Array Lean.Expr := #[]
    let mut expressions := #[← instantiateMVars (← goal.getType)]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← whnfR (← instantiateMVars decl.type)
      expressions := expressions.push (← instantiateMVars decl.type)
      if ty.isAppOfArity ``LeanerIR.SpecInt 2 then
        let width := ty.getArg! 0
        if width.isAppOfArity ``LeanerIR.IntWidth.bits 1 then
          let lemma := if (ty.getArg! 1).isConstOf ``Bool.true
            then ``LeanerIR.SpecInt.signed_bounds else ``LeanerIR.SpecInt.unsigned_bounds
          facts := facts.push (mkApp2 (mkConst lemma) (width.getArg! 0) decl.toExpr)
    for expression in expressions do
      for site in operationSites expression do
        if let some fact ← operationFact? site then facts := facts.push fact
    -- Range certificates and their negations in context yield bounds as
    -- separate facts; rewriting them would leave casts in the terms that
    -- depend on them.
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
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
    for expression in expressions do
      for site in projectionSites (← getEnv) expression do
        let ty ← whnfR (← inferType site)
        if ty.isAppOfArity ``LeanerIR.SpecInt 2 then
          let width := ty.getArg! 0
          if width.isAppOfArity ``LeanerIR.IntWidth.bits 1 then
            let lemma := if (ty.getArg! 1).isConstOf ``Bool.true
              then ``LeanerIR.SpecInt.signed_bounds else ``LeanerIR.SpecInt.unsigned_bounds
            facts := facts.push (mkApp2 (mkConst lemma) (width.getArg! 0) site)
    pure facts
  let mut goal := goal
  for proof in facts do
    let type ← goal.withContext (inferType proof)
    let asserted ← goal.assert `bounds type proof
    let (_, next) ← asserted.intro1P
    goal := next
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

/-- Freshness is preserved by the loan discipline. -/
theorem freshOfDiscipline {initial final : RuntimeState}
    (fresh : LeanerIR.SemanticOperations.FreshGlobalLoanIds initial)
    (discipline : LeanerIR.SemanticOperations.LoanDiscipline initial final) :
    LeanerIR.SemanticOperations.FreshGlobalLoanIds final :=
  discipline.1 fresh

/-- The facts in context, with conjunctions flattened to their parts. -/
private partial def contextFacts : MetaM (Array (Lean.Expr × Lean.Expr)) := do
  let mut found := #[]
  for decl in ← getLCtx do
    if decl.isImplementationDetail then continue
    found := found ++ (← flatten decl.toExpr (← instantiateMVars decl.type))
  return found
where
  flatten (proof type : Lean.Expr) : MetaM (Array (Lean.Expr × Lean.Expr)) := do
    if type.isAppOfArity ``And 2 then
      let left := type.getArg! 0
      let right := type.getArg! 1
      let leftFacts ← flatten (mkAppN (mkConst ``And.left) #[left, right, proof]) left
      let rightFacts ← flatten (mkAppN (mkConst ``And.right) #[left, right, proof]) right
      return leftFacts ++ rightFacts
    else return #[(proof, type)]

/-- The state a record of state fields is built from, when its registry is
that state's registry: a minted or exported state names its base this way. -/
private def baseState? (state : Lean.Expr) : Option Lean.Expr :=
  if state.isAppOfArity ``LeanerIR.RuntimeState.mk 4 then
    let registry := state.getArg! 1
    if registry.isAppOfArity ``LeanerIR.RuntimeState.globalLoans 1 then some (registry.getArg! 0)
    else if registry.isAppOfArity ``List.cons 3 &&
        (registry.getArg! 2).isAppOfArity ``LeanerIR.RuntimeState.globalLoans 1 then
      some ((registry.getArg! 2).getArg! 0)
    else none
  else none

/-- The loan a record registers on top of its base's registry, with its key. -/
private def registeredLoan? (state : Lean.Expr) : Option (Lean.Expr × Lean.Expr) :=
  if state.isAppOfArity ``LeanerIR.RuntimeState.mk 4 then
    let registry := state.getArg! 1
    if registry.isAppOfArity ``List.cons 3 then
      let entry := registry.getArg! 1
      if entry.isAppOfArity ``Prod.mk 4 then some (entry.getArg! 2, entry.getArg! 3) else none
    else none
  else none

/-- A proof of a linear frontier fact by omega, in the goal's context. -/
private def frontierProof? (statement : Lean.Expr) : TacticM (Option Lean.Expr) := do
  let proof ← mkFreshExprMVar statement
  let saved ← saveState
  let goals ← getGoals
  try
    setGoals [proof.mvarId!]
    evalTactic (← `(tactic| (try dsimp only); first | done | omega))
    setGoals goals
    return some (← instantiateMVars proof)
  catch failure =>
    restoreState saved
    if leaner.denoteDebug.get (← getOptions) then
      logInfo m!"frontier not established: {statement}: {failure.toMessageData}"
    return none

/-- The discipline from a state to a record built on it: the registry is
shared, and the frontier is the same or advanced. -/
private def structuralDiscipline? (from_ to : Lean.Expr) : TacticM (Option Lean.Expr) := do
  let some base := baseState? to | return none
  unless ← isDefEq base from_ do return none
  if let some (loan, _) := registeredLoan? to then
    -- A registration on top of the base's registry: `of_registered`.
    let registry ← mkEqRefl (← mkAppM ``LeanerIR.RuntimeState.globalLoans #[to])
    let some minted ← frontierProof? (← mkAppM ``LE.le
        #[← mkAppM ``LeanerIR.RuntimeState.nextLoan #[from_], loan]) | return none
    let some live ← frontierProof? (← mkAppM ``LT.lt
        #[loan, ← mkAppM ``LeanerIR.RuntimeState.nextLoan #[to]]) | return none
    return some (← mkAppM ``LeanerIR.SemanticOperations.LoanDiscipline.of_registered
      #[registry, minted, live])
  let registry ← mkEqRefl (← mkAppM ``LeanerIR.RuntimeState.globalLoans #[to])
  let frontier ← mkAppM ``LE.le #[← mkAppM ``LeanerIR.RuntimeState.nextLoan #[from_],
    ← mkAppM ``LeanerIR.RuntimeState.nextLoan #[to]]
  let next ← mkAppM ``LeanerIR.RuntimeState.nextLoan #[from_]
  let candidates : List (MetaM Lean.Expr) := [
    mkAppM ``Nat.le_refl #[next], mkAppM ``Nat.le_succ #[next]]
  for candidate in candidates do
    try
      let proof ← candidate
      if ← isDefEq (← inferType proof) frontier then
        return some (mkAppN (mkConst ``LeanerIR.Proofs.Denote.LoanDiscipline_of_same_registry)
          #[from_, to, registry, proof])
    catch _ => pure ()
  let proof ← mkFreshExprMVar frontier
  let saved ← saveState
  let goals ← getGoals
  try
    setGoals [proof.mvarId!]
    evalTactic (← `(tactic| (try dsimp only); first | done | omega))
    setGoals goals
    return some (mkAppN (mkConst ``LeanerIR.Proofs.Denote.LoanDiscipline_of_same_registry)
      #[from_, to, registry, ← instantiateMVars proof])
  catch _ =>
    restoreState saved
    return none

/-- A proof of a state fact — freshness of a state, or the discipline
between two states — chained through the discipline facts in context and
the structure of minted and exported states. -/
private partial def stateFact? (facts : Array (Lean.Expr × Lean.Expr)) (target : Lean.Expr)
    (depth : Nat) : TacticM (Option Lean.Expr) := do
  if depth == 0 then return none
  let target ← instantiateMVars target
  let isStateFact (e : Lean.Expr) : Bool :=
    e.isAppOfArity ``LeanerIR.SemanticOperations.FreshGlobalLoanIds 1 ||
      e.isAppOfArity ``LeanerIR.SemanticOperations.LoanDiscipline 2
  unless isStateFact target do return none
  for (proof, type) in facts do
    if isStateFact type && type.getAppFn == target.getAppFn then
      if ← isDefEq type target then return some proof
  if target.isAppOfArity ``LeanerIR.SemanticOperations.FreshGlobalLoanIds 1 then
    let final := target.getArg! 0
    for (proof, type) in facts do
      if type.isAppOfArity ``LeanerIR.SemanticOperations.LoanDiscipline 2 then
        if ← isDefEq (type.getArg! 1) final then
          let earlier := mkApp (mkConst ``LeanerIR.SemanticOperations.FreshGlobalLoanIds)
            (type.getArg! 0)
          if let some fresh ← stateFact? facts earlier (depth - 1) then
            return some (mkAppN (mkConst ``LeanerIR.Proofs.Denote.freshOfDiscipline)
              #[type.getArg! 0, final, fresh, proof])
    if let some base := baseState? final then
      if let some discipline ← structuralDiscipline? base final then
        let earlier := mkApp (mkConst ``LeanerIR.SemanticOperations.FreshGlobalLoanIds) base
        if let some fresh ← stateFact? facts earlier (depth - 1) then
          return some (mkAppN (mkConst ``LeanerIR.Proofs.Denote.freshOfDiscipline)
            #[base, final, fresh, discipline])
    return none
  if target.isAppOfArity ``LeanerIR.SemanticOperations.LoanDiscipline 2 then
    let initial := target.getArg! 0
    let final := target.getArg! 1
    if ← isDefEq initial final then
      return some (mkAppN (mkConst ``LeanerIR.SemanticOperations.LoanDiscipline.of_eq)
        #[initial, final, ← mkEqRefl (← mkAppM ``LeanerIR.RuntimeState.globalLoans #[initial]),
          ← mkAppM ``Nat.le_refl #[← mkAppM ``LeanerIR.RuntimeState.nextLoan #[initial]]])
    if let some structural ← structuralDiscipline? initial final then return some structural
    for (proof, type) in facts do
      if type.isAppOfArity ``LeanerIR.SemanticOperations.LoanDiscipline 2 then
        let from_ := type.getArg! 0
        -- The discipline from `initial` to the fact's start: the fact's
        -- start is `initial`, a state built on it, or a state built on
        -- one the discipline already reaches.
        let toStart? ← if ← isDefEq from_ initial then pure none
          else match ← structuralDiscipline? initial from_ with
            | some structural => pure (some structural)
            | none =>
                match baseState? from_ with
                | some base =>
                    if ← isDefEq base initial then pure none
                    else
                      let reach := mkAppN (mkConst ``LeanerIR.SemanticOperations.LoanDiscipline)
                        #[initial, base]
                      match ← stateFact? facts reach (depth - 1) with
                      | some head =>
                          match ← structuralDiscipline? base from_ with
                          | some structural =>
                              pure (some (mkAppN
                                (mkConst ``LeanerIR.SemanticOperations.LoanDiscipline.trans)
                                #[initial, base, from_, head, structural]))
                          | none => pure none
                      | none => pure none
                | none => pure none
        let step? ← if ← isDefEq from_ initial then pure (some proof)
          else match toStart? with
            | some toStart =>
                pure (some (mkAppN (mkConst ``LeanerIR.SemanticOperations.LoanDiscipline.trans)
                  #[initial, from_, type.getArg! 1, toStart, proof]))
            | none => pure none
        if let some step := step? then
          let rest := mkAppN (mkConst ``LeanerIR.SemanticOperations.LoanDiscipline)
            #[type.getArg! 1, final]
          if let some tail ← stateFact? facts rest (depth - 1) then
            return some (mkAppN (mkConst ``LeanerIR.SemanticOperations.LoanDiscipline.trans)
              #[initial, type.getArg! 1, final, step, tail])
    if let some base := baseState? final then
      if let some structural ← structuralDiscipline? base final then
        let rest := mkAppN (mkConst ``LeanerIR.SemanticOperations.LoanDiscipline) #[initial, base]
        if let some head ← stateFact? facts rest (depth - 1) then
          return some (mkAppN (mkConst ``LeanerIR.SemanticOperations.LoanDiscipline.trans)
            #[initial, base, final, head, structural])
    -- A retired global loan: `final` removes a loan from a callee's final
    -- registry, and a fact carries the callee from the registering state.
    if final.isAppOfArity ``LeanerIR.RuntimeState.mk 4 then
      let registry := final.getArg! 1
      if registry.isAppOfArity ``LeanerIR.SemanticOperations.removeGlobalLoan 2 then
        let calleeFinal := (registry.getArg! 0).getArg! 0
        let loan := registry.getArg! 1
        if (registry.getArg! 0).isAppOfArity ``LeanerIR.RuntimeState.globalLoans 1 then
          for (proof, type) in facts do
            if type.isAppOfArity ``LeanerIR.SemanticOperations.LoanDiscipline 2 then
              let debug := leaner.denoteDebug.get (← getOptions)
              unless ← isDefEq (type.getArg! 1) calleeFinal do
                if debug then logInfo m!"retired: callee final differs {type.getArg! 1} vs {calleeFinal}"
                continue
              let registered := type.getArg! 0
              let some (registeredLoan, _) := registeredLoan? registered
                | if debug then logInfo m!"retired: no registration in {registered}"
                  continue
              unless ← isDefEq registeredLoan loan do
                if debug then logInfo m!"retired: loan differs {registeredLoan} vs {loan}"
                continue
              let some base := baseState? registered
                | if debug then logInfo m!"retired: no base"
                  continue
              unless ← isDefEq base initial do
                if debug then logInfo m!"retired: base differs {base} vs {initial}"
                continue
              let registryEq ← mkEqRefl (← mkAppM ``LeanerIR.RuntimeState.globalLoans #[registered])
              let some minted ← frontierProof? (← mkAppM ``LE.le
                  #[← mkAppM ``LeanerIR.RuntimeState.nextLoan #[initial], loan]) | continue
              let some live ← frontierProof? (← mkAppM ``LT.lt
                  #[loan, ← mkAppM ``LeanerIR.RuntimeState.nextLoan #[registered]]) | continue
              let retiredRegistry ← mkEqRefl (← mkAppM ``LeanerIR.RuntimeState.globalLoans #[final])
              let some retiredFrontier ← frontierProof? (← mkEq
                  (← mkAppM ``LeanerIR.RuntimeState.nextLoan #[final])
                  (← mkAppM ``LeanerIR.RuntimeState.nextLoan #[calleeFinal])) | continue
              return some (← mkAppM ``LeanerIR.Proofs.Denote.LoanDiscipline.through_global_loan
                #[registryEq, minted, live, proof, retiredRegistry, retiredFrontier])
    return none
  return none

/-- Close a state-fact goal through the discipline hypotheses: freshness,
discipline, or the absence of a fresh loan from a state's registry. -/
elab "leaner_denote_state" : tactic => do
  let goal ← getMainGoal
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  if target.isAppOfArity ``Eq 3 then
    let lookup := target.getArg! 1
    unless lookup.isAppOfArity ``LeanerIR.SemanticOperations.globalLoanKeyIn? 2 &&
        (target.getArg! 2).isAppOf ``Option.none &&
        (lookup.getArg! 0).isAppOfArity ``LeanerIR.RuntimeState.globalLoans 1 do
      throwError "not a state fact"
    let state := (lookup.getArg! 0).getArg! 0
    let loan := lookup.getArg! 1
    let some fresh ← goal.withContext do
        stateFact? (← contextFacts)
          (mkApp (mkConst ``LeanerIR.SemanticOperations.FreshGlobalLoanIds) state) 8
      | throwError "no discipline chain establishes freshness"
    let bound ← goal.withContext do
      mkFreshExprMVar (← mkAppM ``LE.le #[← mkAppM ``LeanerIR.RuntimeState.nextLoan #[state], loan])
    setGoals [bound.mvarId!]
    evalTactic (← `(tactic| omega))
    let proof ← goal.withContext do
      mkAppM ``LeanerIR.SemanticOperations.FreshGlobalLoanIds.lookup_of_le
        #[fresh, ← instantiateMVars bound]
    goal.assign proof
    setGoals []
    return
  unless target.isAppOfArity ``LeanerIR.SemanticOperations.FreshGlobalLoanIds 1 ||
      target.isAppOfArity ``LeanerIR.SemanticOperations.LoanDiscipline 2 do
    throwError "not a state fact"
  let some proof ← goal.withContext do stateFact? (← contextFacts) target 8
    | throwError "no discipline chain closes the goal"
  goal.assign proof
  replaceMainGoal []

/-- The registry lookups of a state's loans a proposition mentions. -/
private partial def registryLookups (e : Lean.Expr) (acc : Array Lean.Expr) : Array Lean.Expr :=
  let acc := if e.isAppOfArity ``LeanerIR.SemanticOperations.globalLoanKeyIn? 2 &&
      (e.getArg! 0).isAppOfArity ``LeanerIR.RuntimeState.globalLoans 1 && !e.hasLooseBVars
    then acc.push e else acc
  match e with
  | .app f a => registryLookups a (registryLookups f acc)
  | .lam _ t b _ | .forallE _ t b _ => registryLookups b (registryLookups t acc)
  | .letE _ t v b _ => registryLookups b (registryLookups v (registryLookups t acc))
  | .mdata _ b | .proj _ _ b => registryLookups b acc
  | _ => acc

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
  -- Resolve registry lookups of a callee's final state through its discipline.
  let resolutions ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let mut found : Array (Lean.Expr × Lean.Expr) := #[]
    let disciplines := (← getLCtx).foldl (init := #[]) fun acc decl =>
      if decl.isImplementationDetail then acc
      else
        let ty := decl.type
        if ty.isAppOfArity ``LeanerIR.SemanticOperations.LoanDiscipline 2 then
          acc.push (decl.toExpr, ty.getArg! 0, ty.getArg! 1)
        else acc
    for site in registryLookups target #[] do
      let state := (site.getArg! 0).getArg! 0
      let loan := site.getArg! 1
      for (proof, from_, to) in disciplines do
        unless ← isDefEq to state do continue
        let some bound ← frontierProof? (← mkAppM ``LT.lt
            #[loan, ← mkAppM ``LeanerIR.RuntimeState.nextLoan #[from_]]) | continue
        let stable ← mkAppM ``And.left #[← mkAppM ``And.right #[proof]]
        found := found.push (site, mkApp2 stable loan bound)
        break
    pure found
  for (site, proof) in resolutions do
    let type ← goal.withContext (inferType proof)
    let asserted ← goal.assert `registry type proof
    let (_, next) ← asserted.intro1P
    goal := next
    let _ := site
  -- Rewrite with state-component equations.
  let equations ← goal.withContext do
    let mut found := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      if ty.isAppOfArity ``Eq 3 then
        let lhs := ty.getArg! 1
        if lhs.isApp && lhs.appArg!.isFVar &&
            (lhs.isAppOfArity ``LeanerIR.RuntimeState.pending 1 ||
              lhs.isAppOfArity ``LeanerIR.RuntimeState.globals 1 ||
              lhs.isAppOfArity ``LeanerIR.RuntimeState.nextLoan 1 ||
              lhs.isAppOfArity ``LeanerIR.RuntimeState.globalLoans 1) then
          found := found.push decl.fvarId
        if lhs.isAppOfArity ``LeanerIR.SemanticOperations.globalLoanKeyIn? 2 then
          found := found.push decl.fvarId
    pure found
  setGoals [goal]
  if !equations.isEmpty then
    let idents ← equations.mapM fun fvarId => do
      let name := (← goal.withContext (fvarId.getDecl)).userName
      `(Lean.Parser.Tactic.simpLemma| $(mkIdent name):ident)
    evalTactic (← `(tactic| try simp only [$idents,*]))

/-- Split every conjunctive or existential hypothesis into its parts, and
every row-valued variable into its components, so that a leaf is over
scalars. -/
elab "leaner_denote_split_hypotheses" : tactic => do
  if (← getGoals).isEmpty then return
  let mut goal ← getMainGoal
  let mut progress := true
  while progress do
    progress := false
    let conjunction ← goal.withContext do
      (← getLCtx).findDeclM? fun decl => do
        if decl.isImplementationDetail then return none
        let ty ← instantiateMVars decl.type
        if ty.isAppOfArity ``And 2 || ty.isAppOfArity ``Exists 2 then return some decl.fvarId
        if (← whnfD ty).isAppOfArity ``Prod 2 then return some decl.fvarId
        return none
    if let some fvarId := conjunction then
      let subgoals ← goal.cases fvarId
      match subgoals with
      | #[subgoal] =>
          goal := subgoal.mvarId
          progress := true
      | _ => break
  replaceMainGoal [goal]

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
    let (_, next) ← goal.intro1
    return ← splitGoal next
  return [goal]

elab "leaner_denote_split_goal" : tactic => do
  if (← getGoals).isEmpty then return
  let goals ← (← getMainGoal).withContext (splitGoal (← getMainGoal))
  replaceMainGoal goals

/-- Saturate a leaf's context: rewrite with every hypothesis, destructure
what the rewriting exposes, substitute witnesses, and repeat while new
hypotheses appear. -/
macro "leaner_denote_saturate" : tactic => `(tactic| (
  try simp_all [lir_denote_norm]
  leaner_denote_split_hypotheses
  try subst_vars
  try simp_all [lir_denote_norm]
  leaner_denote_split_hypotheses
  try subst_vars
  try simp_all [lir_denote_norm]
  leaner_denote_split_hypotheses
  try subst_vars
  try simp_all [lir_denote_norm]
  leaner_denote_split_hypotheses
  try subst_vars
  try simp_all [lir_denote_norm]))

/-- Decide one leaf. -/
syntax "leaner_denote_leaf" : tactic

macro_rules
  | `(tactic| leaner_denote_leaf) => `(tactic|
      (leaner_denote_clear_computations
       first
      | (simp only [LeanerIR.Proofs.Obligation_iff, Nat.reduceAdd, Int.reducePow, Int.reduceSub,
          Nat.reducePow, Nat.reduceSub]; done)
      | leaner_denote_state
      | (simp only [LeanerIR.Proofs.Obligation_iff, Nat.reduceAdd, Int.reducePow, Int.reduceSub,
          Nat.reducePow, Nat.reduceSub]
         leaner_denote_bounds
         omega)
      | (simp only [LeanerIR.Proofs.Obligation_iff, Nat.reduceAdd, Int.reducePow, Int.reduceSub,
          Nat.reducePow, Nat.reduceSub]
         decide)
      | (try subst_vars
         leaner_denote_bounds
         try simp (disch := omega) only [LeanerIR.Proofs.Obligation_iff, Nat.reduceAdd,
           Int.reducePow, Int.reduceSub, Nat.reducePow, Nat.reduceSub, Int.tmod_eq_emod_of_nonneg,
           Int.tdiv_eq_ediv_of_nonneg, lir_denote_norm] at *
         first
         | done
         | omega
         | (simp_all [lir_denote_norm]; done)
         | (simp_all [lir_denote_norm]
            all_goals leaner_denote_split_hypotheses
            all_goals (try subst_vars)
            all_goals leaner_denote_bounds
            all_goals omega)
         | (leaner_denote_saturate
            all_goals (try (split <;> (try simp_all [lir_denote_norm])))
            all_goals (try (split <;> (try simp_all [lir_denote_norm])))
            all_goals leaner_denote_split_goal
            all_goals (try simp_all [lir_denote_norm])
            all_goals (leaner_denote_bounds; omega)))
      | trivial))

/- The normalization of a verification condition: the denotation's rules,
the weakest-precondition rules, and the propositional normal forms a leaf
is decided in. -/
attribute [lir_denote_norm] LeanerIR.Proofs.wp_bind LeanerIR.Proofs.wp_pure
  LeanerIR.Proofs.wp_abort LeanerIR.Proofs.Spec.pure_bind
  LeanerIR.Proofs.Denote.ResultShape.bodyType Bool.not_eq_true decide_eq_true_eq
  Bool.and_eq_true Bool.or_eq_true and_assoc exists_and_left
  exists_and_right exists_eq_left exists_eq_left' and_true true_and and_imp
  forall_and forall_eq forall_eq' Prod.mk.injEq exists_eq exists_eq' and_false
  false_and not_false_eq_true not_true_eq_false true_implies false_implies
  implies_true imp_self eq_self_iff_true ite_true ite_false Bool.false_eq_true
  Bool.true_eq_false Bool.not_eq_false decide_eq_false_iff_not
  Bool.and_eq_false_imp Bool.not_true Bool.not_false Bool.not_not Option.some.injEq
  Option.elim_some Option.elim_none
  LeanerIR.SemanticOperations.LoanDiscipline.self
  LeanerIR.RuntimeValue.field LeanerIR.RuntimeValue.asInt LeanerIR.RuntimeValue.asBool
  LeanerIR.RuntimeValue.asString LeanerIR.RuntimeValue.variant?
  List.getElem?_toArray List.getElem?_cons_zero List.getElem?_cons_succ Option.getD_some
  LeanerIR.RuntimeValue.nominal.injEq LeanerIR.RuntimeValue.tuple.injEq
  LeanerIR.RuntimeValue.integer.injEq LeanerIR.RuntimeValue.bool.injEq
  LeanerIR.RuntimeValue.address.injEq List.cons.injEq List.nil_eq beq_self_eq_true
  LeanerIR.RuntimeValue.borrow.injEq LeanerIR.Proofs.Denote.Array.push_eq_push_iff
  LeanerIR.Proofs.Denote.specInt_encode LeanerIR.Proofs.Denote.bool_encode
  LeanerIR.Proofs.Denote.address_encode
  LeanerIR.SemanticOperations.globalLoanKey? Nat.lt_succ_self Nat.lt_add_one Nat.le_refl
  Option.some.injEq Option.map_some Option.map_none LeanerIR.Proofs.Denote.specInt_decode_integer
  LeanerIR.Proofs.Denote.bool_decode_bool LeanerIR.Proofs.Denote.address_decode_address
  LeanerIR.Proofs.Denote.signer_decode_signer
  LeanerIR.Proofs.Denote.NTy.codec_int LeanerIR.Proofs.Denote.NTy.codec_bool
  LeanerIR.Proofs.Denote.NTy.codec_address LeanerIR.Proofs.Denote.NTy.codec_unit
  LeanerIR.Proofs.Denote.NTy.codec_ref LeanerIR.Proofs.Denote.nat_eq_add_succ_iff
  LeanerIR.Proofs.Denote.nat_add_succ_eq_iff LeanerIR.Proofs.Denote.nat_eq_succ_iff
  LeanerIR.Proofs.Denote.nat_succ_eq_iff LeanerIR.Proofs.Denote.Family.key_eq
  LeanerIR.Proofs.Denote.NTy.codec_encode Option.bind_some Option.bind_none
  LeanerIR.FamilyRepresentation Option.getD_some Option.getD_none
  LeanerIR.Proofs.Denote.HList.encode_inj LeanerIR.Proofs.Denote.NTy.decode?_struct_literal
  LeanerIR.Proofs.Denote.NTy.decode?_struct_nominal LeanerIR.Proofs.Denote.rowCodec_decode?_cons
  LeanerIR.Proofs.Denote.rowCodec_decode?_nil
  LeanerIR.Proofs.Denote.NTy.decode?_tuple_literal LeanerIR.Proofs.Denote.NTy.encode_inj
  LeanerIR.Proofs.Denote.NTy.decode?_encode
  LeanerIR.Proofs.Denote.storageKey_address LeanerIR.Proofs.Denote.storageKey_signer
  LeanerIR.SemanticOperations.globalKey LeanerIR.GlobalMap.lookup_insert_self
  LeanerIR.GlobalMap.lookup_erase_self LeanerIR.SemanticOperations.removeGlobalLoan
  LeanerIR.SemanticOperations.globalLoanKeyIn?_head LeanerIR.SemanticOperations.instantiatedTypeId_empty
  LeanerIR.Proofs.Denote.globalLoanKeyIn?_cons Nat.add_assoc LeanerIR.Proofs.Denote.nat_self_eq_add_iff
  LeanerIR.Proofs.Denote.nat_add_eq_self_iff
  Option.isSome_some Option.isSome_none Option.isSome_map LeanerIR.GlobalKey.mk.injEq
  LeanerIR.StorageKey.address.injEq Option.map_eq_some_iff Option.map_eq_none_iff
  LeanerIR.StructHandle.mk.injEq LeanerIR.NamespaceId.mk.injEq
  LeanerIR.Proofs.Contract.typed LeanerIR.Proofs.Denote.hlistCodec
  LeanerIR.Proofs.Denote.resultCodec LeanerIR.Proofs.Denote.NTy.encode_int
  LeanerIR.Proofs.Denote.NTy.encode_bool LeanerIR.Proofs.Denote.NTy.encode_unit
  LeanerIR.Proofs.Denote.NTy.encode_address LeanerIR.Proofs.Denote.NTy.encode_tuple
  LeanerIR.Proofs.Denote.NTy.encode_struct LeanerIR.Proofs.Denote.NTy.encode_enum_inl
  LeanerIR.Proofs.Denote.NTy.encode_enum_inr LeanerIR.Proofs.Denote.HList.encode_nil
  LeanerIR.Proofs.Denote.HList.encode_cons List.toArray_eq_iff List.toList_toArray
  exists_prop exists_const Int.not_lt Int.not_le ne_eq Decidable.not_not
  Bool.true_or Bool.false_or Bool.or_true Bool.or_false


macro "leaner_denote_normalize" location:(Lean.Parser.Tactic.location)? : tactic =>
  `(tactic| simp only [lir_denote, lir_denote_norm, Prod.fst, Prod.snd, reduceCtorEq,
    Nat.reduceAdd, Int.reducePow, Int.reduceSub, Nat.reduceEqDiff, String.reduceBEq,
    String.reduceEq, String.reduceBNe, String.reduceNe] $[$location]?)

/-- The loop a goal's weakest precondition is over, if any: its site,
iteration, and entry locals, with the `wp` instance's other arguments. -/
private def loopGoal? (goal : MVarId) : MetaM (Option (Nat × Array Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``LeanerIR.Proofs.Denote.loopAt 5 do return none
    let some site ← (evalNat (action.getArg! 2)).run | return none
    return some (site, #[action.getArg! 0, action.getArg! 1, action.getArg! 2, action.getArg! 3,
      action.getArg! 4, target.getArg! 4, target.getArg! 5, target.getArg! 6])

/-- One structural split of a goal, on its syntax alone: a binder, a
conjunction, or a conditional.  Nothing is unfolded to find one. -/
private def splitOnce (goal : MVarId) : TacticM (Option (List MVarId)) := do
  let target ← goal.withContext (instantiateMVars (← goal.getType))
  if target.isForall then
    let (_, next) ← goal.intro1
    return some [next]
  if target.isAppOfArity ``And 2 then
    let subgoals ← goal.apply (mkConst ``And.intro)
    return some subgoals
  let saved ← saveState
  try
    setGoals [goal]
    evalTactic (← `(tactic| split))
    return some (← getGoals)
  catch _ =>
    saved.restore
    return none

/-- The loop hypothesis a recursive-call goal is an instance of, if any. -/
private def recursiveHypothesis? (goal : MVarId) : TacticM (Option FVarId) :=
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
        saved.restore
        return if subgoals.length ≥ 1 then some decl.fvarId else none
      catch _ =>
        saved.restore
        return none

/-- The callee a goal's weakest precondition is over, if any: the handle
and the `calleeMeaning` application. -/
private def callGoal? (goal : MVarId) : MetaM (Option (Lean.Expr × Lean.Expr)) :=
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    unless target.isAppOfArity ``LeanerIR.Proofs.wp 7 do return none
    let action := target.getArg! 3
    unless action.isAppOfArity ``LeanerIR.Proofs.Denote.calleeMeaning 5 do return none
    return some (action.getArg! 1, action)

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

/-- Close every goal: a loop by its invariant, a recursive-call leaf by the
loop hypothesis, a call by the callee's theorem, a structural node by
splitting, and a leaf by decision. -/
partial def closeGoals (invariants : Array (Nat × Lean.Expr))
    (callees : Array (Lean.Expr × String × Lean.Expr)) : TacticM Unit := do
  let mut pending : Array (MVarId × Option String) := (← getGoals).toArray.map fun g => (g, none)
  while let some (goal, provenance) := pending.back? do
    pending := pending.pop
    if ← goal.isAssigned then continue
    setGoals [goal]
    if let some (site, arguments) ← loopGoal? goal then
      let some (_, invariant) := invariants.find? (·.1 == site)
        | throwError m!"no invariant for the loop at site {site}"
      let invariant ← goal.withContext (whnfR (mkApp invariant arguments[4]!))
      let rule := mkAppN (mkConst ``LeanerIR.Proofs.Denote.wp_loopAt)
        (arguments.extract 0 5 ++ #[invariant] ++ arguments.extract 5 8)
      let subgoals ← goal.apply rule
      match subgoals with
      | [entryHolds, step] =>
          setGoals [step]
          evalTactic (← `(tactic| intro recursive loopHypothesis env loopInvariant))
          evalTactic (← `(tactic| leaner_denote_normalize at loopHypothesis loopInvariant ⊢))
          pending := pending ++ (← getGoals).toArray.map fun g => (g, provenance)
          setGoals [entryHolds]
          evalTactic (← `(tactic| try leaner_denote_normalize))
          pending := pending ++ (← getGoals).toArray.map fun g => (g, some "a loop invariant at entry")
      | _ => throwError "the loop rule did not produce its two obligations"
      continue
    if let some (handle, _) ← callGoal? goal then
      let some (_, calleeName, theoremProof) ← callees.findM? fun (candidate, _, _) =>
          goal.withContext (isDefEq candidate handle)
        | throwError m!"no verified callee for {handle}"
      let proofSyntax ← goal.withContext (Lean.Elab.Term.exprToSyntax theoremProof)
      setGoals [goal]
      evalTactic (← `(tactic| refine LeanerIR.Proofs.Denote.wp_call $proofSyntax ?_ ?_ ?_))
      let subgoals ← getGoals
      let constants ← goal.withContext (contractConstants (← inferType theoremProof))
      let unfoldNames := constants
      let lemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ← unfoldNames.mapM fun name => do
        let ident := mkIdent (rootNamespace ++ name)
        `(Lean.Parser.Tactic.simpLemma| $ident:ident)
      for (subgoal, index) in subgoals.toArray.zipIdx do
        setGoals [subgoal]
        evalTactic (← `(tactic| try simp only [$lemmas,*, lir_denote_norm]))
        evalTactic (← `(tactic| try leaner_denote_normalize))
        let origin := if index == 0 then some s!"the precondition of `{calleeName}`"
          else if index == 1 then some s!"the continuation after `{calleeName}`" else none
        pending := pending ++ (← getGoals).toArray.map fun g => (g, origin)
      continue
    if let some hypothesis ← recursiveHypothesis? goal then
      let subgoals ← goal.apply (mkFVar hypothesis)
      pending := pending ++ subgoals.toArray.map fun g => (g, some "a loop invariant at an iteration")
      continue
    if let some subgoals ← splitOnce goal then
      for subgoal in subgoals do
        setGoals [subgoal]
        let target ← subgoal.withContext (instantiateMVars (← subgoal.getType))
        if target.isAppOf ``LeanerIR.Proofs.wp then
          if (provenance.map (·.startsWith "the continuation after")).getD false then
            evalTactic (← `(tactic| try leaner_denote_consume))
          evalTactic (← `(tactic| try leaner_denote_normalize))
        pending := pending ++ (← getGoals).toArray.map fun g => (g, provenance)
      continue
    setGoals [goal]
    let closed ← try
        evalTactic (← `(tactic| leaner_denote_leaf))
        pure (← getGoals).isEmpty
      catch failure =>
        if leaner.denoteDebug.get (← getOptions) then
          logInfo m!"leaf not decided: {failure.toMessageData}"
          let steps : Array (String × Syntax.Tactic) := #[
            ("clear", ← `(tactic| leaner_denote_clear_computations)),
            ("subst", ← `(tactic| try subst_vars)),
            ("bounds", ← `(tactic| leaner_denote_bounds)),
            ("simp", ← `(tactic| try simp (disch := omega) only [LeanerIR.Proofs.Obligation_iff,
              Nat.reduceAdd, Int.reducePow, Int.reduceSub, Nat.reducePow, Nat.reduceSub,
              Int.tmod_eq_emod_of_nonneg, Int.tdiv_eq_ediv_of_nonneg, lir_denote_norm] at *)),
            ("simp_all 1", ← `(tactic| try simp_all [lir_denote_norm])),
            ("split hypotheses 1", ← `(tactic| leaner_denote_split_hypotheses)),
            ("subst 1", ← `(tactic| try subst_vars)),
            ("simp_all 2", ← `(tactic| try simp_all [lir_denote_norm])),
            ("split hypotheses 2", ← `(tactic| leaner_denote_split_hypotheses)),
            ("subst 2", ← `(tactic| try subst_vars)),
            ("simp_all 3", ← `(tactic| try simp_all [lir_denote_norm])),
            ("split hypotheses 3", ← `(tactic| leaner_denote_split_hypotheses)),
            ("subst 3", ← `(tactic| try subst_vars)),
            ("simp_all 4", ← `(tactic| try simp_all [lir_denote_norm])),
            ("split conditionals", ← `(tactic| all_goals (try (split <;> (try simp_all [lir_denote_norm]))))),
            ("split goal", ← `(tactic| all_goals leaner_denote_split_goal)),
            ("simp_all 5", ← `(tactic| all_goals (try simp_all [lir_denote_norm]))),
            ("bounds 2", ← `(tactic| all_goals leaner_denote_bounds)),
            ("omega", ← `(tactic| all_goals omega))]
          let saved ← saveState
          setGoals [goal]
          let mut report : Array MessageData := #[]
          for (label, step) in steps do
            try
              evalTactic step
              let goals ← getGoals
              let shown ← goals.mapM fun g => do
                pure (MessageData.ofFormat (← Lean.Meta.ppGoal g))
              report := report.push m!"after {label}: {shown}"
              if goals.isEmpty then break
            catch stepFailure =>
              report := report.push m!"{label} failed: {stepFailure.toMessageData}"
              break
          saved.restore
          for line in report do logInfo line
        pure false
    unless closed do
      setGoals [goal]
      match provenance with
      | some origin =>
          logError m!"{origin} is not established"
          if LeanerIR.Proofs.Certify.leaner.certifyDebug.get (← getOptions) then
            logError m!"residual obligation:\n{← Lean.Meta.ppGoal goal}"
      | none => LeanerIR.Proofs.Certify.reportObligation goal
      admitGoal goal
  setGoals []

/-- Split the normalized goal into leaves and decide each; report the
clause of every leaf that is not decided.  Loops are handled by the
invariants given as `(site, invariant)` pairs. -/
syntax "leaner_denote_close" (" [" term,* "]")? (" with" " [" term,* "]")? : tactic

elab_rules : tactic
  | `(tactic| leaner_denote_close $[[$loops:term,*]]? $[with [$calls:term,*]]?) => do
      if leaner.denoteDebug.get (← getOptions) then
        logInfo m!"normalized verification condition:\n{← getMainGoal}"
      let mut invariants : Array (Nat × Lean.Expr) := #[]
      for loop in (loops.map (·.getElems)).getD #[] do
        let pair ← Lean.Elab.Tactic.elabTerm loop none
        let pair ← instantiateMVars pair
        let pair ← whnf pair
        unless pair.isAppOfArity ``Prod.mk 4 do
          throwError m!"a loop invariant must be a `(site, invariant)` pair, not {pair}"
        let some site ← (evalNat (pair.getArg! 2)).run
          | throwError m!"a loop site must be a numeral, not {pair.getArg! 2}"
        invariants := invariants.push (site, pair.getArg! 3)
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
      closeGoals invariants callees

end LeanerIR.Proofs.Denote
