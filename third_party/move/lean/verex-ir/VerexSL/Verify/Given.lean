-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Verify.Diagnostics
import VerexIR.Proofs.Denote.Authored

namespace VerexSL.Verify

open Lean Meta Elab Command
open VerexIR (FunctionHandle)
open VerexIR.Validation (ValidatedUnit)
open VerexIR.Proofs VerexIR.Proofs.Denote
open VerexSL.Contract

/-- Actual generated proof artifacts in the declaration's given sorts. -/
structure GivenFunctionArtifacts where
  function : Name
  contract : Name
  verified : Name
  operationCount : Nat

/-- An unsupported interpretation is reported before starting a given proof. -/
def givenUnsupported? (unit : ValidatedUnit) (handle : FunctionHandle)
    (compiled : Function unit) : Option String := do
  let ns ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← ns.functions[handle.functionId.index]?
  if declaresGhosts unit then return "ghost interpretation"
  if statesAssumptions ns declaration then return "in-body assumption interpretation"
  -- A `modifies` clause frames memory slots at resource types, which only
  -- the concrete interpretation states, whether or not the body reads memory.
  if !(Contract.frameUses unit handle.namespaceId declaration).memory.isEmpty ||
      !declaration.contract.modifies.isEmpty then
    return "storage interpretation"
  let unsupported := compiled.body.foldNodes (fun callee _ _ found =>
    found || callee == handle) (fun kind _ found => found || kind == "invoke")
    (fun _ _ _ _ _ => true) false
  if unsupported then return "recursive or function-value call interpretation"
  -- A type-reflecting native's value is computed by the translator from the
  -- concrete frame; a given family carries no frame to compute it at. The
  -- aggregator model also reads this frame for its element type and fields.
  let reflecting := compiled.body.foldCalls (fun callee _ _ found => found ||
      (do
        let calleeNs ← unit.namespaces[callee.namespaceId.index]?
        let calleeDeclaration ← calleeNs.functions[callee.functionId.index]?
        nativeModelOf? unit callee calleeNs calleeDeclaration).any fun model =>
        match model with
        | VerexSL.NativeModel.uninterpreted _ _ _ (some _)
        | VerexSL.NativeModel.aggregator _ => true
        | _ => false)
    (fun _ _ found => found) false
  if reflecting then return "type reflection interpretation"
  -- A callee the Given proof can only state as a `Satisfies` premise needs a
  -- contract to state it at: one without a declared contract is read through
  -- its body, which the given carriers do not interpret, so the premise would
  -- say nothing and the caller could not establish its own clauses. Such a
  -- target is verified concretely, where the callee is inlined. A native, an
  -- intrinsic map's role and a prelude model state their own contract, which a
  -- caller assumes as the Move Prover does.
  let contractless := compiled.body.foldCalls (fun callee _ _ found => found ||
      (do
        let calleeNs ← unit.namespaces[callee.namespaceId.index]?
        let calleeDeclaration ← calleeNs.functions[callee.functionId.index]?
        pure (calleeDeclaration.contract.loc.isNone &&
          !(calleeDeclaration.body == VerexIR.Validation.FunctionBody.absent) &&
          (Contract.mapRoleOf? unit callee.namespaceId calleeNs calleeDeclaration).isNone &&
          (nativeModelOf? unit callee calleeNs calleeDeclaration).isNone)).getD false)
    (fun _ _ found => found) false
  if contractless then return "calls to callees without a contract"
  if Contract.contractReadsUnit unit handle.namespaceId ns declaration ||
      Contract.contractReadsRequires unit handle.namespaceId declaration then
    return "behavioral contract interpretation"
  if let .structured root := declaration.body then
    if !(bodySpecifications ns root (· matches .assertion | .apply)).isEmpty then
      return "authored assertion interpretation"
  none

/-- Check the complete interpretation capability before choosing the Given
proof path. Declaration clauses belong to the entire unit, so even a body that
uses no unsupported operation can require an unsupported ADT interpretation.
Only an explicit capability result selects the concrete path; translation bugs
and failed proofs remain errors. Successful clauses are reused by the emitter. -/
def givenDataOwnFor (unit : ValidatedUnit) (handle : FunctionHandle)
    (compiled : Function unit) : MetaM (Except String Contract.DataOwn) := do
  if let some reason := givenUnsupported? unit handle compiled then
    return .error reason
  let own ← givenDataOwnTerms unit
  unless own.unsupported.isEmpty do
    return .error s!"data invariant interpretation: {repr own.unsupported}"
  return .ok own

/-- A Given attempt owns its diagnostic state, including failures during setup
before the walker starts. Translate only heartbeat exceptions into the target's
budget report; keep clause and authored diagnostics that already occurred.
The report retains Lean's heartbeat tag for structured consumers. -/
private def givenVerificationAttempt (ns : VerexIR.Validation.ValidatedNamespace) (function : String)
    (authored : Bool) (budget : Nat) (act : TermElabM α) : TermElabM α := do
  let previous := targetStateExt.getState (← getEnv)
  let state ← TargetState.new
  modifyEnv (targetStateExt.setState · (some ((← getEnv).asyncPrefix?.getD .anonymous, state)))
  let before := (← Core.getMessageLog).reportedPlusUnreported.size
  try
    tryCatchRuntimeEx act fun failure => do
      let rejected := match failure with
        | .internal id _ => id == abortTermExceptionId
        | _ => false
      unless failure.isMaxHeartbeat || rejected do throw failure
      let messages ← Core.getMessageLog
      let allMessages := messages.reportedPlusUnreported.toArray
      let newMessages := allMessages.extract before allMessages.size
      let overBudget := failure.isMaxHeartbeat ||
        newMessages.any (fun message => message.data.hasTag (· == `runtime.maxHeartbeats))
      if !overBudget then
        throwError (verificationFailureMessage ns function authored false budget
          (← failedLeafNames) (← state.budgetStop.get))
      if authored then
        -- The authored tactic's location and detailed cause remain useful.
        -- Its summary deliberately says only that the proof did not close.
        if failure.isMaxHeartbeat then logException failure
        throwError (verificationFailureMessage ns function true true budget
          (← failedLeafNames) (← state.budgetStop.get))
      -- A flush preserves insertion order but moves the unreported prefix
      -- into reported. Already reported diagnostics cannot be retracted; keep
      -- them exactly, and replace only this attempt's pending raw timeouts.
      let pending := messages.unreported.toArray
      let pendingBefore := before - messages.reported.size
      let kept := pending[:pendingBefore].toArray ++
        (pending.extract pendingBefore pending.size).filter
          (fun message => !message.data.hasTag (· == `runtime.maxHeartbeats))
      Core.setMessageLog { messages with unreported := kept.foldl (·.push ·) {} }
      throwError (.tagged `runtime.maxHeartbeats
        (verificationFailureMessage ns function authored true budget
          (← failedLeafNames) (← state.budgetStop.get)))
  finally
    modifyEnv (targetStateExt.setState · previous)

/-- Prove the actual compiled body against the shared translated contract.
Every external callable is an explicit Satisfies premise at this same carrier
family. The concrete export separately requires relational transfer evidence. -/
private def declareGivenFunctionCore (unit : ValidatedUnit) (handle : FunctionHandle)
    (compiled : Function unit) (base : Name)
    (strategy? : Option (Array Term → Array Term → TermElabM
      (TSyntax `tactic × TSyntax `tactic)))
    (own? : Option Contract.DataOwn) (isAuthored : Bool)
    (scopedArguments? : Option (TSyntax `tactic)) : CommandElabM GivenFunctionArtifacts := do
  let own ← match own? with
    | some own => pure own
    | none => match ← liftTermElabM (givenDataOwnFor unit handle compiled) with
      | .ok own => pure own
      | .error reason => throwError "given verification is unsupported here: {reason}"
  let some ns := unit.namespaces[handle.namespaceId.index]?
    | throwError "given function namespace is missing"
  let some declaration := ns.functions[handle.functionId.index]?
    | throwError "given function declaration is missing"
  let some view := compileNamespaceAt? unit handle.namespaceId
    | throwError "given function compiler view is missing"
  let functionName := base ++ `givenFunction
  let contractName := base ++ `givenTypedContract
  let theoremName := base ++ `givenTypedVerified
  let calls := compiled.body.foldCalls (fun callee uses arguments found =>
    if found.any (fun (other, otherUses, otherArgs) =>
        other == callee && otherUses == uses && otherArgs.map (·.1) == arguments.map (·.1))
      then found else found.push (callee, uses, arguments)) (fun _ _ found => found) #[]
  let budget := (heartbeatBudget? declaration).getD (verex.verifyHeartbeats.get (← getOptions))
  let leafBudget := (leafHeartbeatBudget? declaration).getD (verex.leafHeartbeats.get (← getOptions))
  let function := ((ns.nameOf declaration.name).map (·.name)).getD base.toString
  liftTermElabM <| givenVerificationAttempt ns function isAuthored budget <|
    withCurrHeartbeats <|
    withTheReader Core.Context (fun context => { context with maxHeartbeats := budget * 1000 }) <|
    withOptions (fun options =>
      let options := maxHeartbeats.set options budget
      let options := verex.leafHeartbeats.set options leafBudget
      if selectsBitVectors declaration then BitLift.verex.bitVectors.set options true else options) do
    unless (← getEnv).contains functionName do
      let function ← withUnitVariable .implicit fun unitExpr _ => do
        let row ← quoteRow compiled.frame
        let function ← quoteFunction unitExpr compiled (← quoteRow compiled.params)
          (← quoteRow compiled.locals) row (← mkEqRefl row) (← quoteTerm unitExpr compiled.body)
          (← quoteMutables compiled.mutables) (toExpr compiled.exitSite)
        mkLambdaFVars #[unitExpr] function
      addAbbrev functionName function
    let declarations ← declareGivenOwnDeclarations (base.getPrefix ++ `givenOwnDeclarations) own
    withGivenInterpretation own (declarations? := some declarations) fun binders frame => do
      let signature ← quoteNativeSignatureAt frame compiled.params compiled.result
      -- The body is proved against its implementation, as on the concrete
      -- route, whose contract the transfer relates this one to.
      let contract ← buildContract unit handle.namespaceId ns
        (ContractView.implementation.of declaration) signature
        (types := some frame.types) (own := own)
      unless (← getEnv).contains contractName do
        addAbbrev contractName (← mkLambdaFVars binders contract)
      withLocalDeclD `ops (mkApp3 (mkConst ``InterpreterOps) frame.unit frame.carriers frame.state)
          fun ops => do
        let mut meanings : Array Lean.Expr := #[]
        let mut evidence : Array (Name × Lean.Expr) := #[]
        let mut labels : Array String := #[]
        for (callee, uses, arguments) in calls, index in [:calls.size] do
          let some calleeNs := unit.namespaces[callee.namespaceId.index]?
            | throwError "given callee namespace is missing"
          let some calleeDeclaration := calleeNs.functions[callee.functionId.index]?
            | throwError "given callee declaration is missing"
          let some (params, result) := nativeSignature? unit callee.namespaceId calleeDeclaration
            | throwError "given callee has no native signature"
          let meaning ← match arguments with
            | some arguments => pure (mkAppN (mkConst ``InterpreterOps.generic)
                #[frame.unit, frame.carriers, frame.state, ops, toExpr callee, toExpr uses,
                  ← quoteTypeArgs arguments, ← quoteRow params, ← quoteShape result])
            | none => pure (mkAppN (mkConst ``InterpreterOps.call)
                #[frame.unit, frame.carriers, frame.state, ops, toExpr callee,
                  ← quoteRow params, ← quoteShape result])
          -- A caller sees a callee's interface, as on the concrete route.
          let calleeContract ← callerTypedContractOf unit callee.namespaceId.index calleeNs
            (ContractView.interface.of calleeDeclaration) params result
            (arguments.map (·.1) |>.getD .nil) frame own
            (nativeModelOf? unit callee calleeNs calleeDeclaration)
          meanings := meanings.push meaning
          evidence := evidence.push (Name.mkSimple s!"given_call_{index}",
            ← mkAppM ``Satisfies #[meaning, calleeContract])
          labels := labels.push (((calleeNs.nameOf calleeDeclaration.name).map (·.name)).getD "callee")
        withLocalDeclsDND evidence fun proofs => do
          let conditions ← loopInvariants unit frame.unit handle.namespaceId ns declaration
            compiled.params compiled.frame (some frame.types) none (markedInvariant own view)
            (some frame)
          let mut loopPairs : Array Term := #[]
          for (site, condition, _) in conditions do
            let pair ← mkAppM ``Prod.mk #[toExpr handle, condition]
            loopPairs := loopPairs.push (← Term.exprToSyntax
              (← mkAppM ``Prod.mk #[toExpr site, pair]))
          let mut calleePairs : Array Term := #[]
          for meaning in meanings, proof in proofs, label in labels do
            let pair ← mkAppM ``PProd.mk #[toExpr label, proof]
            calleePairs := calleePairs.push (← Term.exprToSyntax
              (← mkAppM ``PProd.mk #[meaning, pair]))
          let meaning := mkApp5 (mkConst ``Function.denoteIn)
            frame.unit frame.carriers frame.state ops (mkApp (mkConst functionName) frame.unit)
          let statement ← mkAppM ``Satisfies #[meaning, mkAppN (mkConst contractName) binders]
          let handleTerm ← Term.exprToSyntax (toExpr handle)
          let patternText := if compiled.params.length == 0 then "_" else
            "⟨" ++ ", ".intercalate ((List.range compiled.params.length).map (s!"argument_{·}")) ++ ", _⟩"
          let .ok pattern := Parser.runParserCategory (← getEnv) `rcasesPat patternText
            | throwError "cannot construct given argument pattern"
          let pattern : TSyntax `rcasesPat := ⟨pattern⟩
          -- A proof in steps binds the parameters' scope entries as it takes
          -- them apart, as the concrete proof's introduction does.
          let arguments := mkIdent `verexArguments
          let destructure ← match scopedArguments? with
            | some introduction => `(tactic| ($introduction:tactic; all_goals verex_scope_state initialState))
            | none => `(tactic| rcases $arguments:ident with $pattern:rcasesPat)
          let (automatic, authored) ← match strategy? with
            | some strategy => strategy loopPairs calleePairs
            | none => pure (← `(tactic| verex_solve_goals [$loopPairs,*] with [$calleePairs,*]),
                ← `(tactic| skip))
          let errorsBefore := (← Core.getMessageLog).reportedPlusUnreported.toArray.countP (·.severity == .error)
          let proof ← Term.elabTerm (← `(term| by
            apply VerexIR.Proofs.satisfies_of_wp
            intro $arguments:ident initialState assumed permitted
            have := VerexIR.Proofs.Denote.FunctionStart.intro $handleTerm $arguments initialState
            $destructure:tactic
            simp only [$(rootIdent functionName):ident, $(rootIdent contractName):ident,
              VerexIR.Proofs.Denote.Function.denoteIn,
              VerexIR.Proofs.Denote.Carriers.lent, vir_denote_norm,
              reduceCtorEq, Nat.reduceEqDiff, String.reduceBEq, String.reduceEq,
              String.reduceBNe, String.reduceNe] at assumed permitted ⊢
            all_goals try simp only [VerexIR.Proofs.Obligation_eq] at permitted
            all_goals verex_normalize at permitted ⊢
            all_goals $automatic:tactic
            all_goals $authored:tactic)) (some statement)
          Term.synthesizeSyntheticMVarsNoPostponing
          let proof ← instantiateMVars proof
          if proof.hasSorry ||
              (← Core.getMessageLog).reportedPlusUnreported.toArray.countP (·.severity == .error) > errorsBefore then
            throwAbortTerm
          let allBinders := binders ++ #[ops] ++ proofs
          let closedType ← mkForallFVars allBinders statement
          let closedProof ← mkLambdaFVars allBinders proof
          addDecl (.thmDecl { name := theoremName, levelParams := [], type := closedType, value := closedProof })
  return { function := functionName, contract := contractName, verified := theoremName, operationCount := calls.size }

/-- Prove in the selected given family. Project its parameter operations at
all normalization boundaries, including cuts introduced after initial setup.
The projection rule is local to this proof; the concrete normalization set is
unchanged. -/
def declareGivenFunction (unit : ValidatedUnit) (handle : FunctionHandle)
    (compiled : Function unit) (base : Name)
    (strategy? : Option (Array Term → Array Term → TermElabM
      (TSyntax `tactic × TSyntax `tactic)) := none)
    (own? : Option Contract.DataOwn := none) (authored : Bool := false)
    (scopedArguments? : Option (TSyntax `tactic) := none) :
    CommandElabM GivenFunctionArtifacts := do
  elabCommand (← `(command| section))
  try
    elabCommand (← `(command| attribute [local vir_denote_norm] VerexIR.Proofs.Denote.Carriers.lent))
    declareGivenFunctionCore unit handle compiled base strategy? own? authored scopedArguments?
  finally
    elabCommand (← `(command| end))

end VerexSL.Verify
