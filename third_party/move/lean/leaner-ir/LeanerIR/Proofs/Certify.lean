-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.DenotationWP
import LeanerIR.Proofs.Plain
import LeanerIR.Proofs.Decode
import LeanerIR.Proofs.IntegerArithmetic

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

/-- Native Boolean equality and the specification's truth equivalence agree. -/
theorem boolean_beq_true (left right : Bool) :
    (left == right) = true ↔ (left = true ↔ right = true) := by
  cases left <;> cases right <;> decide

theorem boolean_bne_true (left right : Bool) :
    (left != right) = true ↔ ¬(left = true ↔ right = true) := by
  cases left <;> cases right <;> decide

/-- Peel only the finite registration prefix; the symbolic registry tail
is discharged by the caller's freshness invariant, never searched. -/
syntax "leaner_fresh_loan" : tactic

theorem loanLookupStable {initial final : RuntimeState} {loan : Nat}
    (discipline : SemanticOperations.LoanDiscipline initial final)
    (bound : loan < initial.nextLoan) :
    SemanticOperations.globalLoanKeyIn? final.globalLoans loan =
      SemanticOperations.globalLoanKeyIn? initial.globalLoans loan :=
  discipline.2.1 loan bound

macro_rules
  | `(tactic| leaner_fresh_loan) =>
      `(tactic| first
        | (apply SemanticOperations.FreshGlobalLoanIds.lookup_of_le
           · assumption
           · omega)
        | (apply SemanticOperations.globalLoanKeyIn?_cons_none
           · omega
           · leaner_fresh_loan))

/-- Show the goal of every clause the certified closing cannot establish,
beside the clause report. -/
register_option leaner.certifyDebug : Bool := {
  defValue := false
  descr := "show the residual goal of a specification clause the certified closing cannot establish"
}

initialize registerTraceClass `leaner.certifyLeaves

/-- Close `goal` with the tactic `tac`; `false` if it fails or leaves
subgoals. The surrounding state is untouched either way. -/
private def solvedBy (goal : MVarId) (tac : TacticM Unit) : TacticM Bool := do
  let saved ← saveState
  try
    setGoals [goal]
    -- Speculative tactics must throw, not log a recovered error and then
    -- let a later tactic close the goal with that diagnostic still present.
    withoutRecover tac
    let remaining ← getGoals
    if remaining.isEmpty then
      pure true
    else
      saved.restore
      pure false
  catch _ =>
    saved.restore
    pure false

/-- Certified integers and vectors whose payload `e` mentions. -/
private partial def certifiedValues (e : Lean.Expr) (found : Array Lean.Expr := #[]) :
    Array Lean.Expr :=
  if e.isAppOfArity ``LeanerIR.SpecInt.val 3 && !e.hasLooseBVars then
    let value := e.getArg! 2
    if found.contains value then found else found.push value
  else if e.isAppOfArity ``LeanerIR.SpecVector.values 2 && !e.hasLooseBVars then
    let value := e.getArg! 1
    if found.contains value then found else found.push value
  else match e with
    | .app f a => certifiedValues a (certifiedValues f found)
    | .lam _ t b _ | .forallE _ t b _ => certifiedValues b (certifiedValues t found)
    | .letE _ t v b _ => certifiedValues b (certifiedValues v (certifiedValues t found))
    | .mdata _ b => certifiedValues b found
    | .proj _ _ b => certifiedValues b found
    | _ => found

/-- Expose fixed-width bounds of unsigned quotient/remainder operations from
their operand certificates. -/
private def exposeUnsignedDivision (goal : MVarId) (certificates : Array Lean.Expr) :
    MetaM MVarId := do
  let facts ← goal.withContext do
    let mut expressions := #[← instantiateMVars (← goal.getType)]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      expressions := expressions.push (← instantiateMVars declaration.type)
    let mut operations : Array Lean.Expr := #[]
    for expression in expressions do
      while let some operation := expression.find? fun e =>
          (e.isAppOfArity ``Int.tdiv 2 || e.isAppOfArity ``Int.tmod 2) &&
          !e.hasLooseBVars && !operations.contains e do
        operations := operations.push operation
    let mut facts := #[]
    for operation in operations do
      let left := operation.getArg! 0
      let right := operation.getArg! 1
      let mut leftCertificate : Option Lean.Expr := none
      let mut rightCertificate : Option Lean.Expr := none
      for certificate in certificates do
        let type ← inferType certificate
        if type.isAppOfArity ``LeanerIR.IntegerValueFits 3 &&
            (type.getArg! 1).isConstOf ``Bool.false then
          if type.getArg! 2 == left then leftCertificate := some certificate
          if type.getArg! 2 == right then rightCertificate := some certificate
      let some leftProof := leftCertificate | continue
      let some rightProof := rightCertificate | continue
      if operation.isAppOfArity ``Int.tdiv 2 then
        facts := facts.push (← mkAppM ``IntegerArithmetic.unsigned_quotient_bounds
          #[leftProof, rightProof])
      else
        let mut nonzero : Option Lean.Expr := none
        for declaration in ← getLCtx do
          if declaration.isImplementationDetail then continue
          let type ← instantiateMVars declaration.type
          if type.consumeMData.isAppOfArity ``Not 1 then
            if let some (_, lhs, rhs) := type.consumeMData.getArg! 0 |>.eq? then
              if lhs == right && (rhs.nat? <|> rhs.rawNatLit?) == some 0 then
                nonzero := some (mkFVar declaration.fvarId)
        let some nonzeroProof := nonzero | continue
        facts := facts.push (← mkAppM ``IntegerArithmetic.unsigned_remainder_bounds
          #[leftProof, rightProof, nonzeroProof])
    pure facts
  let mut goal := goal
  for fact in facts do
    let type ← goal.withContext do inferType fact
    let present ← goal.withContext do
      (← getLCtx).anyM fun declaration => pure (declaration.type == type)
    if present then continue
    let (_, next) ← (← goal.assert `unsignedDivisionBounds type fact).intro1P
    goal := next
  return goal

/-- Instantiate signed division facts only on a leaf that actually mentions
division, using a matching operand certificate to choose the range. Keep
their arithmetic premises explicit; the leaf solver discharges them from
the path. No runtime evaluator or frame is unfolded. -/
private def exposeSignedDivision (goal : MVarId) (certificates : Array Lean.Expr) :
    MetaM MVarId := do
  let signed ← goal.withContext do
    certificates.filterM fun proof => do
      pure (((← inferType proof).getArg! 1).isConstOf ``Bool.true)
  if signed.isEmpty then return goal
  let facts ← goal.withContext do
    let mut expressions := #[← instantiateMVars (← goal.getType)]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      expressions := expressions.push (← instantiateMVars declaration.type)
    let mut divisions : Array Lean.Expr := #[]
    for expression in expressions do
      while let some operation := expression.find? fun e =>
          (e.isAppOfArity ``Int.tdiv 2 || e.isAppOfArity ``Int.tmod 2) &&
          !e.hasLooseBVars && !divisions.contains e do
        divisions := divisions.push operation
    let mut facts := #[]
    for operation in divisions do
      let isQuotient := operation.isAppOfArity ``Int.tdiv 2
      let operand := operation.getArg! (if isQuotient then 0 else 1)
      for certificate in signed do
        let type ← inferType certificate
        unless type.getArg! 2 == operand do continue
        let width := (type.getArg! 0).getArg! 0
        let some width := width.nat? <|> width.rawNatLit? | continue
        let limit := mkNatLit (2 ^ (width - 1))
        let proof ← if isQuotient then do
            pure <| mkAppN (mkConst ``IntegerArithmetic.signed_quotient_bounds)
              #[limit, operation.getArg! 0, operation.getArg! 1]
          else
            pure (mkAppN (mkConst ``IntegerArithmetic.signed_remainder_bounds)
              #[limit, operation.getArg! 0, operation.getArg! 1])
        facts := facts.push proof
        break
    pure facts
  let mut goal := goal
  for fact in facts do
    let type ← goal.withContext do inferType fact
    let present ← goal.withContext do
      (← getLCtx).anyM fun declaration => pure (declaration.type == type)
    if present then continue
    let (_, next) ← (← goal.assert `signedDivisionBounds type fact).intro1P
    goal := next
  return goal

/-- Expose the bounds every fixed-width certificate carries, as the two
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
            (signed.isConstOf ``Bool.false || signed.isConstOf ``Bool.true) then
          found := found.push (mkFVar declaration.fvarId)
    for value in certifiedValues (← instantiateMVars (← goal.getType)) do
      let ty ← whnf (← inferType value)
      if ty.isAppOfArity ``LeanerIR.SpecInt 2 then
        let width := ty.getArg! 0
        let signed := ty.getArg! 1
        if width.isAppOfArity ``LeanerIR.IntWidth.bits 1 &&
            (signed.isConstOf ``Bool.false || signed.isConstOf ``Bool.true) then
          found := found.push (← mkAppM ``LeanerIR.SpecInt.fits #[value])
    pure found
  let mut goal := goal
  for certificate in certificates do
    let bounds ← goal.withContext do
      let type ← inferType certificate
      let rule := if (type.getArg! 1).isConstOf ``Bool.true then
        ``LeanerIR.IntegerValueFits.signed_bounds else
        ``LeanerIR.IntegerValueFits.unsigned_bounds
      mkAppM rule #[certificate]
    let boundsType ← goal.withContext do inferType bounds
    /- Once per certificate: a leaf visited after another already has the
    bounds in context. -/
    let present ← goal.withContext do
      (← getLCtx).anyM fun declaration => do
        pure ((← instantiateMVars declaration.type) == boundsType)
    if present then continue
    let (_, next) ← (← goal.assert `certifiedBounds boundsType bounds).intro1P
    goal := next
  let afterUnsigned ← exposeUnsignedDivision goal certificates
  goal ← exposeSignedDivision afterUnsigned certificates
  let vectorBounds ← goal.withContext do
    let mut found := #[]
    for value in certifiedValues (← instantiateMVars (← goal.getType)) do
      if (← inferType value).isAppOfArity ``LeanerIR.SpecVector 1 then
        found := found.push (← mkAppM ``LeanerIR.SpecVector.bounded #[value])
    pure found
  for bounds in vectorBounds do
    let type ← goal.withContext do inferType bounds
    let present ← goal.withContext do
      (← getLCtx).anyM fun declaration => pure (declaration.type == type)
    unless present do
      let (_, next) ← (← goal.assert `certifiedLength type bounds).intro1P
      goal := next
  return goal

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
partial def normalizeProjections (e : Lean.Expr) : MetaM Lean.Expr :=
  Meta.transform e (post := fun e => do
    /- Native vector decoding rebuilds the list-backed array. This is
    definitional eta, but leaving the wrapper gives exact fact lookup two
    spellings for the same array. -/
    if e.isAppOfArity ``List.toArray 2 &&
        (e.getArg! 1).isAppOfArity ``Array.toList 2 then
      return .done ((e.getArg! 1).getArg! 1)
    /- Unification can leave a beta redex where a value belongs; it is one
    `whnfCore` from its payload. -/
    if e.getAppFn.isLambda then
      let reduced ← whnfCore e
      if reduced != e then
        return .done (← normalizeProjections reduced)
    /- Specification projections over the closed runtime image of a native
    value are equally just projections.  Reduce them only when the operand
    itself exposes a constructor: an unknown `RuntimeValue` stays folded,
    so this does not branch over the universal runtime representation. -/
    let runtimeProjection :=
      e.isAppOfArity ``LeanerIR.RuntimeValue.field 2 ||
      e.isAppOfArity ``LeanerIR.RuntimeValue.asInt 1 ||
      e.isAppOfArity ``LeanerIR.RuntimeValue.asBool 1 ||
      e.isAppOfArity ``LeanerIR.RuntimeValue.asString 1
    if runtimeProjection then
      let some owner := e.getAppArgs[0]? | unreachable!
      if ← Meta.isConstructorApp (← whnf owner) then
        let reduced ← whnf e
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

/- Monomorphic contracts use the same type-indexed accessors as generic
contracts.  Restore their concrete key before applying map/frame laws. -/
attribute [lir_data_norm] SemanticOperations.instantiatedTypeId_empty
attribute [lir_data_norm] IntegerArithmetic.bitwiseAnd_mod
  IntegerArithmetic.shiftLeft_mod IntegerArithmetic.shiftRight_mod
attribute [lir_data_norm] RuntimeValue.bne_integer RuntimeValue.beq_bool
  RuntimeValue.beq_address

theorem field_vector_getElem (values : Array RuntimeValue)
    (index : Nat) (bound : index < values.size) :
    (RuntimeValue.vector values).field index = values[index] := by
  simp [RuntimeValue.field, bound]

theorem field_vector_outOfBounds (values : Array RuntimeValue)
    (index : Nat) (bound : ¬ index < values.size) :
    (RuntimeValue.vector values).field index = .unit := by
  simp [RuntimeValue.field, bound]

theorem asInt_integer (value : Int) :
    (RuntimeValue.integer value).asInt = value := rfl

theorem asInt_unit : RuntimeValue.unit.asInt = 0 := rfl

/-- Index guards use the original vector's length, independent of mapping
or writing one element. Reduce only those size wrappers before arithmetic. -/
macro "leaner_spec_index_bound" : tactic =>
  `(tactic|
    (simp (config := { failIfUnchanged := false }) only
      [Array.size_map, Array.size_setIfInBounds, Array.length_toList,
      List.length_map, List.length_set, List.size_toArray, Int.toNat_zero]
     omega))

/-- Vector observations use a small, scoped pure inventory. None of these
array/update rules are added to unrelated arithmetic normalization. -/
macro "leaner_spec_vector_read" : tactic =>
  `(tactic|
    simp (disch := leaner_spec_index_bound) only
      [lir_spec_norm, field_vector_getElem, field_vector_outOfBounds,
       asInt_integer, asInt_unit, SemanticOperations.resolveReturnedBorrows_empty,
       Array.size_map, Array.size_setIfInBounds, Array.length_toList,
       List.length_map, List.length_set, List.size_toArray,
       Array.getElem_map, List.getElem_toArray, Array.getElem_toList,
       List.getElem_set_self, List.getElem_set_ne,
       Array.getElem_setIfInBounds_self, Array.getElem_setIfInBounds_ne,
       Int.toNat_zero])

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
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedPair_right,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_singleBorrow,
         LeanerIR.SemanticOperations.fillHole?_focusValue,
         LeanerIR.SemanticOperations.rewriteFirst_holeFill?_focusValue,
         LeanerIR.SemanticOperations.fillHole?, LeanerIR.SemanticOperations.holeFill?,
         LeanerIR.SemanticOperations.rewriteFirst_loanHole,
         LeanerIR.SemanticOperations.rewriteFirst_integer,
         LeanerIR.SemanticOperations.rewriteFirst_nominal,
         LeanerIR.SemanticOperations.rewriteFirstList_nil,
         LeanerIR.SemanticOperations.rewriteFirstList_cons,
         beq_self_eq_true, Option.getD_some, Option.getD_none, List.toList_toArray,
         ite_true, ite_false, Option.map_some, Option.map_none])
  | `(tactic| leaner_resolve_rows at *) =>
      `(tactic| try simp (disch := (first | omega | leaner_plain)) only
        [LeanerIR.SemanticOperations.resolveReturnedBorrows_empty,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_integer,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedBorrow,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedBorrow_integer,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedBorrow_focus,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedPair_left,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_returnedPair_right,
         LeanerIR.SemanticOperations.resolveReturnedBorrows_singleBorrow,
         LeanerIR.SemanticOperations.fillHole?_focusValue,
         LeanerIR.SemanticOperations.rewriteFirst_holeFill?_focusValue,
         LeanerIR.SemanticOperations.fillHole?, LeanerIR.SemanticOperations.holeFill?,
         LeanerIR.SemanticOperations.rewriteFirst_loanHole,
         LeanerIR.SemanticOperations.rewriteFirst_integer,
         LeanerIR.SemanticOperations.rewriteFirst_nominal,
         LeanerIR.SemanticOperations.rewriteFirstList_nil,
         LeanerIR.SemanticOperations.rewriteFirstList_cons,
         beq_self_eq_true, Option.getD_some, Option.getD_none, List.toList_toArray,
         ite_true, ite_false, Option.map_some, Option.map_none] at *)

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
  /- A written generic resource is exposed as a decoder applied to the
  nominal literal.  The twin's certified literal roundtrip is generated
  beside its erasure; include it here because this storage-normalization
  phase runs after the caller's outer simp set. -/
  let decoderLemmas : Array (TSyntax ``Lean.Parser.Tactic.simpLemma) ←
    goal.withContext do
      let env ← getEnv
      facts.filterMapM fun fvarId => do
        let type ← instantiateMVars (← fvarId.getType)
        let erase? :=
          if type.isAppOfArity ``LeanerIR.FamilyRepresentation 6 then
            some type.getAppArgs[0]!
          else
            match type with
            | .forallE _ _ body _ =>
                if body.isAppOfArity ``Eq 3 then
                  let rhs := body.getArg! 2
                  if rhs.isAppOfArity ``Option.map 4 then some (rhs.getArg! 2)
                  else none
                else none
            | _ => none
        let some erase := erase? | return none
        let some eraseName := erase.getAppFn.constName?
          | return none
        let literalRoundtrip := eraseName.getPrefix ++ `decode?_literal
        unless env.contains literalRoundtrip do return none
        return some
          (← `(Lean.Parser.Tactic.simpLemma| $(mkIdent literalRoundtrip):ident))
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
          [$lemmas,*, $decoderLemmas,*, LeanerIR.FamilyRepresentation,
           LeanerIR.updateContents, LeanerIR.RuntimeValue.storageKey,
           LeanerIR.RuntimeValue.storageKey?, Option.getD_some,
           LeanerIR.Proofs.Codec.decode_encode_apply,
           Option.bind_some, Option.map_none, Option.map_eq_none_iff,
           Option.bind_none, Option.getD_none,
           Option.isSome_none, Option.isSome_some, Option.isSome_map,
           Bool.false_eq_true,
           Bool.true_eq_false, not_true_eq_false, not_false_eq_true,
           GlobalMap.lookup_insert_other, GlobalMap.lookup_erase_other, lir_data_norm,
           ite_true, ite_false, reduceIte, eq_self_iff_true] at *)
    else
      evalTactic <| ← `(tactic|
        simp (disch := leaner_denotation_discharge) only
          [$lemmas,*, $decoderLemmas,*, LeanerIR.FamilyRepresentation,
           LeanerIR.updateContents, LeanerIR.RuntimeValue.storageKey,
           LeanerIR.RuntimeValue.storageKey?, Option.getD_some,
           LeanerIR.Proofs.Codec.decode_encode_apply,
           Option.bind_some, Option.map_none, Option.map_eq_none_iff,
           Option.bind_none, Option.getD_none,
           Option.isSome_none, Option.isSome_some, Option.isSome_map,
           Bool.false_eq_true,
           Bool.true_eq_false, not_true_eq_false, not_false_eq_true,
           GlobalMap.lookup_insert_other, GlobalMap.lookup_erase_other, lir_data_norm,
           ite_true, ite_false, reduceIte, eq_self_iff_true])
    match ← getGoals with
    | [normalized] => pure normalized
    | [] => pure goal
    | _ => saved.restore; pure goal
  catch failure =>
    if leaner.certifyDebug.get (← getOptions) then
      IO.eprintln s!"normalizeStorage raised: {← failure.toMessageData.toString}"
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
          | (simp_all only [Option.isSome_none, Option.isSome_some, Option.map_none,
               Bool.false_eq_true, Bool.true_eq_false, Bool.not_true, Bool.not_false,
               not_true_eq_false, not_false_eq_true, false_and, and_false, ite_self] <;> done)
          | omega
          | (simp (disch := (first | omega | leaner_plain))
               [lir_data_norm] at * <;> omega)
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

/-- Inspect arithmetic operands, not typeclass dictionaries or the proof
arguments hidden inside certified scalar values. A generic expression walk
here charges large reference-call certificates to every arithmetic leaf. -/
private partial def hasVectorRead (expression : Lean.Expr) : Bool :=
  let expression := expression.consumeMData
  if !expression.isApp then false else
  match expression.getAppFn.constName? with
  | some ``RuntimeValue.asInt =>
      let observed := expression.appArg!
      observed.isAppOfArity ``RuntimeValue.field 2 &&
        (observed.getArg! 0).isAppOf ``RuntimeValue.vector
  | some ``Eq | some ``LT.lt | some ``LE.le |
      some ``HAdd.hAdd | some ``HSub.hSub | some ``HMul.hMul |
      some ``HDiv.hDiv | some ``HMod.hMod |
      some ``Int.add | some ``Int.sub | some ``Int.mul |
      some ``Int.tdiv | some ``Int.tmod =>
      expression.getAppNumArgs ≥ 2 &&
        (hasVectorRead expression.appArg! || hasVectorRead expression.appFn!.appArg!)
  | some ``Not | some ``Neg.neg | some ``Int.neg =>
      hasVectorRead expression.appArg!
  | _ => false

/-- Apply the product-range certificate only to an authored multiplication.
Unifying its conclusion with an unrelated literal upper bound can unfold
integer multiplication into a linear-size natural-number recursion. -/
elab "leaner_product_upper" : tactic => do
  let target ← instantiateMVars (← (← getMainGoal).getType)
  unless target.isAppOfArity ``LT.lt 4 &&
      (target.getArg! 0).isConstOf ``Int do
    throwError "expected an integer product upper-bound obligation"
  let right := (target.getArg! 3).consumeMData
  unless right.isAppOfArity ``HMul.hMul 6 || right.isAppOfArity ``Int.mul 2 do
    throwError "expected an authored multiplication on the right"
  evalTactic (← `(tactic| exact IntegerArithmetic.product_upper_of_failed_range
    (by omega) (by omega) (by assumption)))

/-- Match a normalized fact without unfolding unrelated execution
equalities while rejecting a false scalar obligation. -/
elab "leaner_exact_fact" : tactic => withMainContext do
  let goal ← getMainGoal
  let target ← instantiateMVars (← goal.getType)
  for declaration in ← getLCtx do
    if declaration.isImplementationDetail then continue
    if (← instantiateMVars declaration.type) == target then
      goal.assign (mkFVar declaration.fvarId)
      replaceMainGoal []
      return
  throwError "no matching normalized fact"

/-- The arithmetic leaf: bounded integer reasoning over the context, with
`Bool` facts propositionalized on the leaves plain `omega` missed, and a
certified integer's bounds exposed only on the leaves those missed too. -/
private def closeBooleanConditional (goal : MVarId) : TacticM Bool := do
  let conditional ← goal.withContext do
    let some (carrier, lhs, rhs) := (← instantiateMVars (← goal.getType)).eq? | return false
    unless carrier.isConstOf ``Int || carrier.isConstOf ``Nat do return false
    return [lhs, rhs].any fun expression =>
      expression.isAppOfArity ``ite 5 &&
        let condition := expression.getArg! 1
        (condition.find? fun clause =>
          clause.isAppOfArity ``Eq 3 && (clause.getArg! 0).isConstOf ``Bool).isSome
  unless conditional do return false
  solvedBy goal (evalTactic (← `(tactic|
    split <;> simp_all only [Bool.false_eq_true, Bool.true_eq_false,
      true_implies, true_and, and_true, false_and, and_false,
      true_or, or_true, false_or, or_false, not_or, ite_true, ite_false] <;> omega)))

def arithmeticLeaf (goal : MVarId) : TacticM Bool := do
  /- A value spelled as a structure-literal projection is the same atom
  as the projected variable the facts use only after core normalization. -/
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  /- Checked vector reads need the path's bound before projection reduction.
  Keep this exception local to arithmetic: other equations still normalize
  constructor projections with the original inventory and cost. -/
  let vectorRead := hasVectorRead target
  let normalized ← if vectorRead then pure target
    else goal.withContext do normalizeProjections target
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
      let type ← instantiateMVars declaration.type
      if type == normalized then
        found := some declaration.fvarId
        continue
      -- A decoder, frame equation, or contract cannot be this arithmetic
      -- fact. Reject its head/type before traversing its (possibly large)
      -- value and proof arguments to normalize projections.
      unless type.getAppFn == normalized.getAppFn do continue
      if type.isAppOfArity ``Eq 3 && type.getArg! 0 != normalized.getArg! 0 then
        continue
      if (← normalizeProjections type) == normalized then
        found := some declaration.fvarId
    pure found
  if let some fvarId := matched then
    goal.assign (mkFVar fvarId)
    return true
  /- Typed vector lengths are known before trying general decidability or
  context-wide normalization. Their runtime length may still be spelled
  through an element map and the VM's u64 remainder. -/
  if (normalized.find? (·.isConstOf ``LeanerIR.SpecVector.values)).isSome then
    if leaner.certifyDebug.get (← getOptions) then
      IO.eprintln s!"typed vector arithmetic start: {normalized}"
    let saved ← saveState
    let bounded ← exposeBounds goal
    if leaner.certifyDebug.get (← getOptions) then
      IO.eprintln s!"typed vector bounds:\n{← bounded.withContext do ppGoal bounded}"
    if ← solvedBy bounded (evalTactic (← `(tactic|
        (simp (failIfUnchanged := false) only
          [lir_spec_norm, Array.size_map, List.size_toArray, List.length_set, Array.length_toList,
           Int.ofNat_eq_natCast, Int.reduceAbs] at * <;> omega)))) then
      if leaner.certifyDebug.get (← getOptions) then
        IO.eprintln "typed vector arithmetic done"
      return true
    if leaner.certifyDebug.get (← getOptions) then
      IO.eprintln "typed vector arithmetic fallback"
    saved.restore
  let closed ← solvedBy goal <| evalTactic <| ← `(tactic|
    first
    | rfl
    | leaner_product_upper
    | omega
    | decide
    | (simp only [Bool.and_eq_true, decide_eq_true_eq, decide_eq_false_iff_not,
         Int.ofNat_eq_natCast] at *
       omega))
  if closed then return true
  /- A finite Boolean match's fallthrough records conditional exclusions,
  e.g. `left = true → right = false` and `left = true → right = true`.
  Split just the Boolean guard at the head of a scalar result expression;
  do not enumerate Boolean parameters or branch inside execution terms. -/
  if ← closeBooleanConditional goal then return true
  /- Constructor projections can be exposed only after an Obligation has
  opened. Normalize this small pure-spec inventory on the target alone;
  the frame/state evaluator inventory is deliberately excluded. -/
  if vectorRead then
    if leaner.certifyDebug.get (← getOptions) then
      IO.eprintln "certified vector-read arithmetic"
    let closed ← solvedBy goal do
      evalTactic (← `(tactic| leaner_spec_vector_read))
      /- A checked projection can reveal a previously hidden `SpecInt`
      element. Expose its bounds only on this vector-read path. -/
      setGoals (← (← getGoals).mapM fun goal => do exposeBounds goal)
      evalTactic (← `(tactic| all_goals omega))
    if closed then return true
  let closed ← solvedBy goal <| evalTactic <| ← `(tactic|
    (simp only [lir_spec_norm] <;> omega))
  if closed then return true
  /- An absent/present storage branch can make the leaf literally `False`.
  Restrict the context-wide option simplification to that contradiction:
  on ordinary invariant arithmetic it is pure failed search over a large
  two-family context. -/
  if normalized.isConstOf ``False then
    let closed ← solvedBy goal <| evalTactic <| ← `(tactic|
      (simp_all only [Option.isSome_none, Option.isSome_some, Option.map_none,
         Bool.false_eq_true, Bool.true_eq_false, not_true_eq_false,
         not_false_eq_true] <;> done))
    if closed then return true
  /- Only local data-invariant assumptions need the contract-side erasure
  inventory below.  Module invariants contain global lookups and use the
  storage normalizer; trying the much broader data simplifier on those
  leaves duplicates its work and breaks their deliberately tight budget. -/
  let hasLocalDataInvariant ← goal.withContext do
    (← getLCtx).anyM fun declaration => do
      if declaration.isImplementationDetail then return false
      unless declaration.userName.toString.startsWith "requires_" do return false
      let ty ← instantiateMVars declaration.type
      let hasProjection := (ty.find? fun e =>
        e.isConstOf ``LeanerIR.RuntimeValue.field ||
        e.isConstOf ``LeanerIR.RuntimeValue.asInt ||
        e.isConstOf ``LeanerIR.RuntimeValue.asBool ||
        e.isConstOf ``LeanerIR.RuntimeValue.asString).isSome
      return hasProjection &&
        (ty.find? (·.isConstOf ``LeanerIR.GlobalMap.lookup)).isNone
  let goal ← exposeBounds goal
  /- A clause that selects by a condition the path decided — `if value == 0
  then 1 else 2` under a guard on that decision — splits on the goal's
  conditional, each side decided against the guard. -/
  /- The rewriting may close the goal by itself (a remainder spelled two
  ways, say); the arithmetic then has nothing left to do. -/
  if hasLocalDataInvariant then
    let closed ← solvedBy goal <| evalTactic <| ← `(tactic|
      (simp (disch := omega) only [lir_data_norm, beq_self_eq_true, beq_iff_eq,
         if_true, if_false, Bool.true_and, Bool.false_eq_true,
         true_implies, String.reduceEq, decide_true, decide_false] at * <;> omega))
    if closed then return true
  /- Aggregate guards can contradict a scalar classification clause.
  Keep this context-wide equality closure off unrelated invariant leaves. -/
  let hasVectorGuard ← goal.withContext do
    (← getLCtx).anyM fun declaration => do
      if declaration.isImplementationDetail then return false
      pure <| (declaration.type.find? fun term =>
        let equality := term.isAppOfArity ``Eq 3
        let comparison := term.isAppOfArity ``BEq.beq 4
        (equality || comparison) && (term.getArg! 0).isConstOf ``RuntimeValue &&
          ((term.getArg! (if equality then 1 else 2)).isAppOf ``RuntimeValue.vector ||
           (term.getArg! (if equality then 2 else 3)).isAppOf ``RuntimeValue.vector)).isSome
  if hasVectorGuard then
    let closed ← solvedBy goal <| evalTactic <| ← `(tactic|
      (simp_all only [beq_iff_eq, beq_eq_false_iff_ne,
         RuntimeValue.vector.injEq, Array.mk.injEq, List.cons.injEq,
         reduceCtorEq, and_false, false_and, not_true_eq_false]; done))
    if closed then return true
  let closed ← solvedBy goal <| evalTactic <| ← `(tactic|
    first
    | omega
    | (simp only [RuntimeValue.beq_address, decide_eq_true_eq] at *; leaner_exact_fact)
    | (simp (disch := omega) only [Bool.and_eq_true, decide_eq_true_eq,
         decide_eq_false_iff_not, Array.size_map, Int.ofNat_eq_natCast, Int.tdiv_eq_ediv_of_nonneg,
         Int.tmod_eq_emod_of_nonneg] at * <;> omega)
    | (simp only [Bool.and_eq_true, decide_eq_true_eq, decide_eq_false_iff_not,
         Int.ofNat_eq_natCast] at * <;> (split <;> omega)))
  if closed then return true
  if (normalized.find? (·.isConstOf ``IntegerArithmetic.bitwiseAnd)).isSome then
    return ← solvedBy goal <| evalTactic <| ← `(tactic|
      (apply IntegerArithmetic.bitwiseAnd_mod <;> omega))
  return false

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
  /- Keep the arithmetic-only path first: it is the common script-route
  case, and `simp_all` over a large borrow context is substantially more
  expensive.  Storage-aware contradictions use the second pass. -/
  let closed ← solvedBy bounded <| evalTactic <| ← `(tactic|
    (intros
     exfalso
     simp (disch := omega) only [Bool.and_eq_true, decide_eq_true_eq, Bool.not_eq_true,
       Bool.false_eq_true, Bool.true_eq_false,
       decide_eq_false_iff_not, not_and, Int.not_le, Int.not_lt, Nat.not_le, Nat.not_lt,
       Int.ofNat_eq_natCast, Int.tdiv_eq_ediv_of_nonneg, Int.tmod_eq_emod_of_nonneg] at *
       <;> omega))
  if closed then return true
  let closed ← solvedBy bounded <| evalTactic <| ← `(tactic|
    (intros
     exfalso
     simp_all (disch := omega) only [Bool.and_eq_true, decide_eq_true_eq, Bool.not_eq_true,
       Bool.false_eq_true, Bool.true_eq_false,
       Option.isSome_none, Option.isSome_some, Option.map_eq_none_iff,
       SemanticOperations.globalKey, RuntimeValue.storageKey,
       RuntimeValue.storageKey?, Option.getD_some,
       decide_eq_false_iff_not, not_and, Int.not_le, Int.not_lt, Nat.not_le, Nat.not_lt,
       Int.ofNat_eq_natCast, Int.tdiv_eq_ediv_of_nonneg, Int.tmod_eq_emod_of_nonneg]
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
    pure (ty.find? fun e => e.isConstOf ``LeanerIR.RuntimeValue.storageKey ||
      e.isConstOf ``LeanerIR.GlobalKey).isSome
  unless mentionsKey do return goal
  -- Generated binders can share an inaccessible displayed name. Address
  -- this exact local, not whichever declaration name resolution selects.
  setGoals [goal]
  let normalized ← goal.withContext do
    let invocation ← `(tactic| simp only [lir_data_norm, LeanerIR.RuntimeValue.storageKey,
      LeanerIR.RuntimeValue.storageKey?, Option.getD_some])
    let context ← mkSimpContext invocation (eraseLocal := false)
    let (some (_, normalized), _) ← simpLocalDecl goal fvarId context.ctx context.simprocs
      (mayCloseGoal := false)
      | throwError "certified closing: normalizing a key hypothesis lost the goal"
    pure normalized
  setGoals [normalized]
  pure normalized

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

/-- The decoders applied to nominal literals in `e`. -/
private partial def literalDecodings (e : Lean.Expr) (acc : Array (Lean.Name × Lean.Expr)) : Array (Lean.Name × Lean.Expr) :=
  let acc := if e.isApp && !e.hasLooseBVars then
      match e.appFn!.consumeMData.constName? with
      | some name =>
          if name.getString! == "decode?" &&
              e.appArg!.consumeMData.isAppOfArity ``LeanerIR.RuntimeValue.nominal 3 &&
              !acc.any (fun (_, e') => e' == e) then
            acc.push (name, e)
          else acc
      | none => acc
    else acc
  match e with
  | .app f a => literalDecodings a (literalDecodings f acc)
  | .lam _ t b _ | .forallE _ t b _ => literalDecodings b (literalDecodings t acc)
  | .letE _ t v b _ => literalDecodings b (literalDecodings v (literalDecodings t acc))
  | .mdata _ b => literalDecodings b acc
  | .proj _ _ b => literalDecodings b acc
  | _ => acc

/-- Decode literal twins in a goal or in one callee-summary hypothesis.
Symbolic decoders under storage lookups are deliberately left folded. -/
def decodeLiteralTwins (goal : MVarId) (hypothesis? : Option FVarId := none) :
    TacticM (Option MVarId) := do
  let decodings ← goal.withContext do
    let target ← instantiateMVars (← match hypothesis? with
      | some hypothesis => hypothesis.getType
      | none => goal.getType)
    pure (literalDecodings target #[])
  if decodings.isEmpty then return some goal
  let mut goal := goal
  let mut names : Array Lean.Name := #[]
  for (decode, application) in decodings do
    let some (_, proof) ← goal.withContext
        (Denotation.RowSpec.evaluateDecoding #[decode] application) | continue
    let name ← mkFreshUserName `decoded
    let asserted ← goal.assert name (← goal.withContext (inferType proof)) proof
    let (_, next) ← asserted.intro1P
    goal := next
    names := names.push name
  if names.isEmpty then return some goal
  setGoals [goal]
  let lemmas ← names.mapM fun name => `(Lean.Parser.Tactic.simpLemma| $(mkIdent name):ident)
  match hypothesis? with
  | none =>
      evalTactic (← `(tactic| try simp only [$lemmas,*, Option.getD_some, Option.bind_some,
        Option.map_some]))
  | some hypothesis =>
      let name ← goal.withContext do pure (mkIdent (← hypothesis.getDecl).userName)
      evalTactic (← `(tactic| simp (config := { failIfUnchanged := false }) only
        [$lemmas,*, Option.getD_some, Option.bind_some, Option.map_some] at $name:ident))
  match ← getGoals with
  | [remaining] => pure (some remaining)
  | [] => pure none
  | _ => throwError "certified closing: decoding the literals split the goal"

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
    /- Resource exposure eagerly turns entry invariants at present keys into
    typed arithmetic facts.  When target-only storage normalization has also
    made the leaf storage-free, try it before rewriting every hypothesis;
    successful invariant leaves avoid traversing the large path condition. -/
    if (target₀.find? (·.isConstOf ``LeanerIR.GlobalMap.lookup)).isNone then
      let saved ← saveState
      let closed ← try arithmeticLeaf goal catch _ => pure false
      if closed then return
      saved.restore
    /- A storage-valued arithmetic premise may be the only fact connecting
    the typed post-state leaf to its initial values. Normalize the context
    once at this leaf, after the target-only pass has exposed its shape. -/
    let goal ← normalizeStorage goal (everywhere := true)
    if ← goal.isAssigned then return
    /- A freshly published nominal literal can occur in an invariant's
    arithmetic leaf, not only in an equality.  Decode those literals once
    here as well; the decoder evaluator constructs the proof from the
    parameter certificates already in context. -/
    let some goal ← decodeLiteralTwins goal | return
    if ← goal.isAssigned then return
    let target₀ ← goal.withContext do
      pure (← instantiateMVars (← goal.getType)).consumeMData
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
  else if target.isAppOfArity ``Iff 2 then
    return some do
      let falseBoolean := [target.getArg! 0, target.getArg! 1].any fun side =>
        side.isAppOfArity ``Eq 3 && (side.getArg! 0).isConstOf ``Bool &&
          (((side.getArg! 1).isConstOf ``Bool.false && (side.getArg! 2).isConstOf ``Bool.true) ||
           ((side.getArg! 1).isConstOf ``Bool.true && (side.getArg! 2).isConstOf ``Bool.false))
      let compound := [target.getArg! 0, target.getArg! 1].any fun side =>
        side.isAppOfArity ``And 2 || side.isAppOfArity ``Or 2 || side.isAppOfArity ``Exists 2
      if falseBoolean && compound then
        -- A false Boolean result negates the other side of the contract.
        -- Expose that negation before introducing an implication whose
        -- conclusion still looks like an ordinary Boolean equation. Keep
        -- scalar equations on their existing path: negation introduction
        -- can expose Int's implementation instead of its arithmetic form.
        setGoals [goal]
        evalTactic (← `(tactic| simp only
          [Bool.false_eq_true, Bool.true_eq_false, false_iff, iff_false]))
        match ← getGoals with
        | [] => return
        | [next] => return ← close deferred tentative next
        | _ => throwError "Boolean constant normalization split an equivalence"
      -- Use a named native Boolean result before splitting its truth-value
      -- equivalence. Only variable-left equations that occur in this leaf
      -- qualify; constructor equations are never installed as rewrite rules.
      let booleanResults ← goal.withContext do
        let mut facts := #[]
        for declaration in ← getLCtx do
          if declaration.isImplementationDetail then continue
          let some (carrier, left, _) := declaration.type.eq? | continue
          if carrier.isConstOf ``Bool && left.isFVar &&
              (target.find? (· == left)).isSome then
            facts := facts.push (← Term.exprToSyntax declaration.toExpr)
        return facts
      for fact in booleanResults do
        if ← solvedBy goal (evalTactic (← `(tactic|
            simp only [$fact:term, Bool.not_eq_true', Bool.eq_false_iff,
              lir_data_norm]))) then return
      let mut booleanGoal := goal
      if (target.find? fun expression =>
          ((expression.isAppOfArity ``BEq.beq 4 || expression.isAppOfArity ``bne 4) &&
            (expression.getArg! 0).isConstOf ``Bool) ||
          expression.isAppOfArity ``Bool.and 2 || expression.isAppOfArity ``Bool.or 2 ||
          expression.isAppOfArity ``Bool.not 1).isSome then
        -- Keep partial Boolean normalization: the remaining equivalence
        -- may need the arithmetic closer, after truth tests are removed.
        -- Reduce only constructor-backed projections, not the WP context.
        let normalized ← goal.withContext do normalizeProjections target
        setGoals [← goal.change normalized (checkDefEq := false)]
        evalTactic (← `(tactic| simp (config := { failIfUnchanged := false }) only
          [boolean_beq_true, boolean_bne_true,
            Bool.and_eq_true, Bool.or_eq_true, Bool.not_eq_true', Bool.eq_false_iff,
            ne_eq, decide_eq_true_eq, Decidable.not_not,
            eq_self_iff_true, Bool.false_eq_true, true_iff, iff_true,
            false_iff, iff_false]))
        match ← getGoals with
        | [] => return
        | [next] =>
            let normalized ← next.withContext do instantiateMVars (← next.getType)
            if normalized != target then
              -- Boolean negation of arithmetic must reach omega while the
              -- integer operations still have their algebraic spelling.
              if ← solvedBy next (evalTactic (← `(tactic| omega))) then return
              return ← close deferred tentative next
            booleanGoal := next
        | _ => throwError "Boolean normalization split an equivalence"
      let goal := booleanGoal
      if (target.getArg! 0).isAppOfArity ``ite 5 ||
          (target.getArg! 1).isAppOfArity ``ite 5 then
        if ← solvedBy goal (evalTactic (← `(tactic|
            simp (disch := assumption) only [if_pos, if_neg]))) then return
      /- A typed Boolean equality is represented as a decision at the
      runtime boundary.  Close its logical equivalence before splitting it:
      after implication introduction, weak-head reduction would expose the
      concrete `Decidable` implementation (notably String's byte walk). -/
      if (target.find? (·.isConstOf ``decide)).isSome then
        if ← solvedBy goal (evalTactic (← `(tactic|
            simp only [decide_eq_true_eq])) ) then
          return
      let mut current := goal
      -- A Boolean specification match can be an equivalence with a
      -- closed variant test on one side. Normalize its literal data
      -- before introducing a direction: otherwise a false branch is
      -- hidden in the premise and cannot be recognized as vacuous.
      if (target.find? (·.isAppOfArity ``LeanerIR.RuntimeValue.nominal 3)).isSome then
        setGoals [goal]
        evalTactic (← `(tactic| simp (config := { failIfUnchanged := false }) only
          [lir_data_norm, String.reduceBEq, beq_self_eq_true,
           Bool.false_eq_true, Bool.true_eq_false, ite_true, ite_false,
           eq_self_iff_true]))
        match ← getGoals with
        | [] => return
        | [next] =>
            let normalized ← next.withContext do instantiateMVars (← next.getType)
            if normalized != target then return ← close deferred tentative next
            current := next
        | _ => throwError "literal data normalization split an equivalence"
      let goal := current
      -- A modular call often supplies this equivalence verbatim, up to
      -- constructor projections such as `RuntimeValue.storageKey`. Use
      -- that fact before splitting away the connective. Only explicit
      -- equivalences are candidates: never unfold execution or contract
      -- hypotheses in a general `assumption` search.
      let candidates ← goal.withContext do
        let mut candidates := #[]
        for declaration in ← getLCtx do
          if declaration.isImplementationDetail then continue
          let type ← instantiateMVars declaration.type
          if type.consumeMData.isAppOfArity ``Iff 2 then
            candidates := candidates.push declaration.fvarId
        pure candidates
      for candidate in candidates do
        if ← solvedBy goal (liftMetaTactic fun g => g.withContext do
            unless ← isDefEq (← g.getType) (← candidate.getType) do
              throwError "callee equivalence does not match"
            g.assign (mkFVar candidate)
            pure []) then return
        -- Generic callees characterize equality of encoded data, while
        -- their callers can compare the native fields. Normalize only
        -- this summary and the target, never the execution context.
        let fact ← goal.withContext do Term.exprToSyntax (mkFVar candidate)
        if ← solvedBy goal (evalTactic (← `(tactic|
            simpa only [RuntimeValue.nominal.injEq, RuntimeValue.vector.injEq,
              RuntimeValue.integer.injEq, RuntimeValue.bool.injEq,
              RuntimeValue.address.injEq, Array.mk.injEq, List.cons.injEq,
              eq_self_iff_true, true_and, and_true, Codec.encode_eq_encode]
              using $fact))) then return
      -- Keep both sides visible while normalizing resource accessors. If
      -- an accessor moves into an introduced hypothesis first, a Boolean
      -- conclusion no longer names the definitions its reduction needs.
      let goal ← normalizeStorage goal
      if ← goal.isAssigned then return
      let normalized ← goal.withContext do instantiateMVars (← goal.getType)
      unless normalized.consumeMData.isAppOfArity ``Iff 2 do
        return ← close deferred tentative goal
      let subgoals ← goal.apply (mkConst ``Iff.intro)
      for subgoal in subgoals do
        close deferred tentative subgoal
  else if target.isAppOfArity ``Exists 2 then
    return some (closeExists deferred tentative goal)
  else if target.isAppOfArity ``Not 1 || target.isArrow then
    return some do
      let (fvarId, opened) ← goal.intro1P
      -- An equivalence may introduce a vacuous direction, such as
      -- `false = true → P`. Inspect only the newly introduced hypothesis;
      -- do not scan or unfold the surrounding execution context.
      let contradictory ← opened.withContext do
        let type ← instantiateMVars (← fvarId.getType)
        if type.isConstOf ``False then
          opened.assign (← mkFalseElim (← opened.getType) (mkFVar fvarId))
          return true
        let some (_, lhs, rhs) := type.eq? | return false
        let some left := lhs.getAppFn.constName? | return false
        let some right := rhs.getAppFn.constName? | return false
        let env ← getEnv
        let some (.ctorInfo leftCtor) := env.find? left | return false
        let some (.ctorInfo rightCtor) := env.find? right | return false
        unless leftCtor.induct == rightCtor.induct && left != right do return false
        opened.assign (← mkNoConfusion (← opened.getType) (mkFVar fvarId))
        return true
      if contradictory then return
      let inner ← opened.withContext do instantiateMVars (← opened.getType)
      if inner.isConstOf ``False then
        refute opened fvarId
      else
        /- An invariant implication may be vacuous because its existence
        guard contradicts the storage branch selected by the operation.
        Refute a structured premise once before proving the conclusion;
        failed refutation restores the untouched goal. -/
        let premise ← opened.withContext do whnf (← fvarId.getType)
        if premise.isAppOf ``And || premise.isAppOf ``Or ||
            premise.isAppOf ``Exists then
          /- Existence guards are vacuous on an absent storage branch (a
          publish is the common case).  Do not launch the context-wide
          refuter on present branches: those guards are satisfiable and the
          failed speculative pass dominates multi-resource invariants. -/
          let hasStorageAbsence ← opened.withContext do
            (← getLCtx).anyM fun declaration => do
              if declaration.isImplementationDetail then return false
              let type ← instantiateMVars declaration.type
              let some (_, lhs, rhs) := type.eq? | return false
              let absent := rhs.consumeMData.isAppOfArity ``Option.none 1
              unless absent do return false
              if lhs.isAppOfArity ``LeanerIR.GlobalMap.lookup 2 then return true
              unless lhs.isApp && lhs.appFn!.isFVar do return false
              let functionType ← whnf (← inferType lhs.appFn!)
              return functionType.isArrow &&
                functionType.bindingDomain!.isConstOf ``LeanerIR.StorageKey
          let erasesResource :=
            (premise.find? (·.isConstOf ``LeanerIR.GlobalMap.erase)).isSome
          if hasStorageAbsence || erasesResource then
            let saved ← saveState
            try
              refute opened fvarId
              return
            catch failure =>
              if leaner.certifyDebug.get (← getOptions) then
                IO.eprintln s!"premise refutation raised: {← failure.toMessageData.toString}"
              saved.restore
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
    return some (closeEquation deferred tentative goal)
  else if target.isAppOfArity ``Or 2 then
    /- Normalize a storage-valued proposition before choosing its branch.
    Otherwise the failed-range fact is exposed only after the choice and the
    two possible endpoints can no longer be proved as one disjunction. -/
    if (target.find? (·.isConstOf ``LeanerIR.GlobalMap.lookup)).isSome then
      return none
    /- A contract with several `aborts_if` clauses states its abort
    condition as their disjunction; the branch the current path
    establishes is one of two, a bounded choice. -/
    return some do
      /- A failed range check establishes "below OR above", without fixing
      which endpoint was crossed. Keep the whole arithmetic proposition for
      the decision procedure before choosing a branch independently. -/
      if ← solvedBy goal (evalTactic (← `(tactic|
          first
          | exact IntegerArithmetic.outside_i64_of_failed_range (by assumption)
          | exact IntegerArithmetic.outside_of_failed_range (by assumption)
          | omega
          | (simp only [Nat.reducePow, Nat.reduceSub, Int.reduceNeg] at * <;>
              omega)))) then return
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
  else if target.isAppOfArity ``SemanticOperations.Plain 1 then
    return some do
      unless ← solvedBy goal (evalTactic (← `(tactic|
          first
          | leaner_exact_fact
          | (simp_all only [Codec.encode_eq_encode]; done)
          | leaner_plain))) do
        throwError "certified closing: local slot is not proved loan-free"
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
        | (simp (disch := leaner_plain) only
            [lir_reconcile, LeanerIR.SemanticOperations.LoanDiscipline.self])
        | exact LeanerIR.SemanticOperations.LoanDiscipline.of_eq rfl
            (Nat.le_refl _)
        | (apply LeanerIR.SemanticOperations.LoanDiscipline.of_eq <;>
            first | rfl | omega | (dsimp only; omega))
        | (apply LeanerIR.SemanticOperations.LoanDiscipline.trans (by assumption)
           apply LeanerIR.SemanticOperations.LoanDiscipline.of_eq <;>
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
private partial def closeEquation (deferred : IO.Ref (Array MVarId)) (tentative : IO.Ref Nat)
    (goal : MVarId) : TacticM Unit := do
  if ← goal.isAssigned then return
  -- A pure Boolean-selected scalar is not a runtime reconciliation problem.
  -- Close its head guard before loading codec and loan normalization rules.
  if ← closeBooleanConditional goal then return
  trace[leaner.certifyLeaves] "equation start ({← IO.getNumHeartbeats}): {← goal.withContext do
    ppExpr (← instantiateMVars (← goal.getType))}"
  /- Generic equality needs only its one path fact. Simplifying the entire
  context also visits the executable preparation and agreement equations,
  which is unnecessary work for an injective codec. -/
  let equalityFacts ← goal.withContext do
    let mut facts := #[]
    for declaration in ← getLCtx do
      if declaration.isImplementationDetail then continue
      let type ← instantiateMVars declaration.type
      let some (carrier, left, _) := type.eq? | continue
      let runtimeComparison := left.isAppOfArity ``RuntimeValue.beq 2 ||
        (left.isAppOfArity ``BEq.beq 4 && (left.getArg! 0).isConstOf ``RuntimeValue)
      if runtimeComparison || carrier.getAppFn.isFVar then
        facts := facts.push (← Term.exprToSyntax declaration.toExpr)
    pure facts
  for fact in equalityFacts do
    if ← solvedBy goal (evalTactic (← `(tactic|
        simpa only [beq_iff_eq, RuntimeValue.beq_eq_true, Codec.encode_eq_encode]
          using $fact:term))) then return
  let mut current := goal
  -- An exposed enum decoder can retain closed variant-name tests ahead
  -- of its range check. Reduce only literal string comparisons here;
  -- symbolic guards and arithmetic are left to their existing rules.
  let closedTag ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let some (type, lhs, _) := target.eq? | return false
    unless type.isAppOfArity ``Option 1 do return false
    pure <| (lhs.find? fun expression =>
      expression.isAppOfArity ``BEq.beq 4 &&
        (expression.getArg! 0).isConstOf ``String &&
        (expression.getArg! 2).isLit && (expression.getArg! 3).isLit).isSome
  if closedTag then
    let before ← goal.withContext do instantiateMVars (← goal.getType)
    setGoals [goal]
    evalTactic (← `(tactic| simp only [String.reduceBEq, ite_true, ite_false,
      Bool.false_eq_true, eq_self_iff_true]))
    match ← getGoals with
    | [] => return
    | [next] =>
        let after ← next.withContext do instantiateMVars (← next.getType)
        if before != after then return ← closeEquation deferred tentative next
        current := next
    | _ => throwError "literal enum tag reduction split an equation"
  let goal := current
  /- Native vector lengths are arithmetic, even after an element update.
  Close them before trying storage, reconciliation, or context-wide equation
  search; those rules can otherwise interpret a length as a loan id. -/
  let vectorLength ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let some (type, _, _) := target.eq? | return false
    return (type.isConstOf ``Nat || type.isConstOf ``Int) &&
      (target.find? (·.isConstOf ``LeanerIR.SpecVector.values)).isSome
  if vectorLength then
    let saved ← saveState
    let closed ← try arithmeticLeaf goal catch _ => pure false
    if closed then return
    saved.restore
  if ← solvedBy goal (liftMetaTactic fun g => do g.refl; pure []) then
    return
  -- Once a call summary exposes a concrete integer constructor, its
  -- decoder only needs the existing range certificate. Do this before
  -- context-wide equation normalization visits the callee's semantics.
  let integerDecoder ← goal.withContext do
    let some (_, lhs, _) := (← instantiateMVars (← goal.getType)).eq? | return false
    return lhs.isAppOfArity ``LeanerIR.decodeInt? 3 &&
      (lhs.getArg! 2).isAppOfArity ``RuntimeValue.integer 1
  if integerDecoder then
    let impossible ← goal.withContext do
      let some (_, lhs, rhs) := (← instantiateMVars (← goal.getType)).eq? | return false
      return rhs.isAppOfArity ``Option.some 2 &&
        (← whnf lhs).isAppOfArity ``Option.none 1
    if impossible then throwError "certified closing: integer result is outside its declared range"
  let (goal, resolved) ← if integerDecoder then resolveDite? goal else pure (goal, false)
  if resolved then return ← closeEquation deferred tentative goal
  let updatedVector ← goal.withContext do
    pure ((← instantiateMVars (← goal.getType)).find?
      (·.isConstOf ``List.set)).isSome
  if updatedVector then
    if ← solvedBy goal (evalTactic (← `(tactic|
        simp only [List.map_toArray, List.map_set, Array.toArray_toList] <;> rfl))) then
      return
  /- The context records immutable aggregate contents, not their runtime
  constructor wrapper. Preserve that one equation through congruence before
  looking for storage or loan rewrites. -/
  let vectorEquality ← goal.withContext do
    let some (_, lhs, rhs) := (← instantiateMVars (← goal.getType)).eq?
      | return none
    unless lhs.isAppOfArity ``RuntimeValue.vector 1 &&
        rhs.isAppOfArity ``RuntimeValue.vector 1 do return none
    return some (lhs.getArg! 0, rhs.getArg! 0)
  if let some (lhs, rhs) := vectorEquality then
    let congruence ← goal.withContext do
      mkAppOptM ``congrArg #[none, none, some lhs, some rhs,
        some (mkConst ``RuntimeValue.vector)]
    for next in ← goal.apply congruence do
      closeEquation deferred tentative next
    return
  /- Local postcondition equations commonly form a short chain from the
  callee's typed value to the caller's value.  `simp_all only` uses just
  those hypotheses (and definitional proof irrelevance), without opening
  the global simp inventory or unfolding execution terms. -/
  if ← solvedBy goal (evalTactic (← `(tactic| simp_all only <;> done))) then
    return
  /- Runtime codec equations are often equalities between short array
  literals.  Expose their element equations before the generic row
  normalizer; unification can then determine the enclosing existential
  record without searching. -/
  if ← solvedBy goal (evalTactic (← `(tactic|
      simp only [Array.mk.injEq, List.cons.injEq, and_true,
        RuntimeValue.address.injEq, RuntimeValue.integer.injEq,
        RuntimeValue.bool.injEq]))) then
    return
  /- A generated storage representation may choose a fresh loan witness.
  Its remaining leaf is precisely the reachable-state freshness invariant
  already present at function entry. -/
  let loanLookup ← goal.withContext do
    let some (_, lhs, _) := (← goal.getType).eq? | return false
    return lhs.isAppOf ``SemanticOperations.globalLoanKeyIn? ||
      lhs.isAppOf ``SemanticOperations.globalLoanKey?
  if loanLookup then
    if ← solvedBy goal (evalTactic (← `(tactic| leaner_fresh_loan))) then return
    let retiredLookup ← goal.withContext do
      let some (_, lhs, _) := (← goal.getType).eq? | return false
      return lhs.isAppOfArity ``SemanticOperations.globalLoanKeyIn? 2 &&
        (lhs.getArg! 0).isAppOfArity ``SemanticOperations.removeGlobalLoan 2
    -- Do not retry freshness/arithmetic for ordinary parameter lookups.
    if retiredLookup && (← solvedBy goal (evalTactic (← `(tactic|
        (simp (config := { failIfUnchanged := false }) (disch := omega) only
          [SemanticOperations.globalLoanKeyIn?_remove_other]
         first
         | leaner_fresh_loan
         | (apply loanLookupStable
            · assumption
            · omega)))))) then return
  /- A pointwise storage frame is map algebra, independent of the family's
  payload codec. Close it before representation rewriting can obscure the
  same symbolic query on the two sides. -/
  let mapFrame ← goal.withContext do
    -- This is a head-shape filter, not a request to instantiate the large
    -- pending-write payload or unresolved result witnesses underneath it.
    let some (_, lhs, rhs) := (← goal.getType).eq? | return false
    return lhs.isAppOfArity ``GlobalMap.lookup 2 && rhs.isAppOfArity ``GlobalMap.lookup 2
  let goal ← if mapFrame then do
      let keyFacts ← goal.withContext do
        return (← getLCtx).foldl (init := #[]) fun found declaration =>
          if (declaration.type.isAppOfArity ``Ne 3 || declaration.type.isAppOfArity ``Not 1) &&
              (declaration.type.find? (·.isConstOf ``RuntimeValue.storageKey)).isSome then
            found.push declaration.fvarId else found
      let mut keyGoal := goal
      for fact in keyFacts do keyGoal ← normalizeKeySpelling keyGoal fact
      pure keyGoal
    else pure goal
  if mapFrame then
    if ← solvedBy goal (evalTactic (← `(tactic|
        (simp (disch := assumption) only
          [GlobalMap.lookup_insert_other, GlobalMap.lookup_erase_other]; done)))) then return
  /- A lender's resolution under a field read is evaluated first:
  exposing the equation's head unfolds the definition the read is
  applied to, and would bury the resolution under its fold. -/
  let some goal ← resolveRows goal | return
  /- Pure vector observations should use their checked-index rules before
  equation-head/storage unfolding turns the projection into an Option
  matcher. Keep this bounded attempt off storage and codec equations. -/
  let vectorObservation ← goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let some (type, _, _) := target.eq? | return false
    unless type.isConstOf ``Int do return false
    unless hasVectorRead target do return false
    return (target.find? (·.isConstOf ``GlobalMap.lookup)).isNone
  if vectorObservation then
    let saved ← saveState
    let closed ← try arithmeticLeaf goal catch _ => pure false
    if closed then return
    saved.restore
  let (goal, resolved) ← resolveDite? goal
  if resolved then return ← closeEquation deferred tentative goal
  let goal ← normalizeStorage goal
  if ← goal.isAssigned then return
  -- Decoding a Boolean equation can expose an implication or equivalence.
  -- Re-enter the connective dispatcher rather than requiring it to remain
  -- an equation after representation normalization.
  let target ← goal.withContext do instantiateMVars (← goal.getType)
  unless target.consumeMData.isEq do return ← close deferred tentative goal
  if ← solvedBy goal (liftMetaTactic fun g => do g.refl; pure []) then
    return
  let (goal, resolved) ← resolveDite? goal
  if resolved then return ← closeEquation deferred tentative goal
  /- A composed resource-reading callee receives the caller's runtime
  presence fact through the typed family representation.  Apply the one
  exact bridge rather than unfolding the global map or searching through
  execution hypotheses. -/
  if ← solvedBy goal (evalTactic (← `(tactic|
      (apply LeanerIR.FamilyRepresentation.isSome_of_lookup <;> assumption)))) then
    return
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
      if ty.getArg! 0 != target.getArg! 0 then continue
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
    /- A twin decoded from a literal the normal form wrote — a nominal
    literal under a twin's `decode?` — decodes to the twin whose
    certificates the context proves; the decoding equation joins the
    context before the rows run. -/
    let some goal ← decodeLiteralTwins goal | pure []
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
         [lir_reconcile, lir_data_norm, lir_eval,
          Option.bind_some, Option.bind_none, if_pos, if_neg,
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
