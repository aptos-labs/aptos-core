-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base

/-!
# Reachable hidden-theorem dependency closure across shared certificates

Checks that `declareHiddenTheorems`, `hiddenTheorem`, and
`SharedContinuations.certifySharedProof` restore registered `hiddenTheorem`
declarations in dependency order while keeping stored `state.hiddenTheorems`
and environment declarations in exact agreement whenever both are present,
leaving unrelated rolled-back hidden theorems absent, and rejecting unregistered
constants fail-closed.
-/

namespace VerexIR.Tests.HiddenTheoremClosure

open Lean Meta Elab Command Tactic
open VerexIR.Proofs.Denote

private def assertRegistryEnvAgreement (state : TargetState) : MetaM Unit := do
  let env ← getEnv
  for (name, decl) in ← state.hiddenTheorems.get do
    if let some envInfo := env.find? name then
      match decl, envInfo with
      | .thmDecl stored, .thmInfo present =>
          unless stored.type == present.type && stored.value == present.value do
            throwError m!"stored hiddenTheorem `{name}` diverged from environment declaration"
      | _, _ =>
          throwError m!"stored declaration kind for `{name}` diverged from environment"

run_cmd liftTermElabM do
  let rootName := `VerexIR.Tests.HiddenTheoremClosure.rollback_shared_control
  Term.withDeclName rootName do
    let state ← targetState
    state.proofName.set rootName
    -- 1. Direct finalizer boundary control (`declareHiddenTheorems` alone, before
    -- any `certifySharedProof` call): `outerExpr` directly mentions only `outerName`,
    -- whose checked body in `state.hiddenTheorems` references `innerName`. Both were
    -- rolled back from `env`. The old direct-constant scan fails with
    -- `(kernel) unknown constant` on `innerName` when adding `outerName`; the
    -- dependency-ordered traversal restores `innerName` first, then `outerName`,
    -- while leaving `abandonedName` absent.
    let prop1 := mkAnd (mkConst ``True) (mkConst ``True)
    let prop2 := mkAnd prop1 (mkConst ``True)
    let propAbandoned := mkOr (mkConst ``True) (mkConst ``False)
    let saved ← Meta.saveState
    let (innerName, outerExpr, outerName, abandonedName) ← do
      let innerProof := mkApp4 (mkConst ``And.intro)
        (mkConst ``True) (mkConst ``True) (mkConst ``True.intro) (mkConst ``True.intro)
      let innerExpr ← hiddenTheorem prop1 innerProof
      let innerName := innerExpr.getAppFn.constName!
      let outerProof := mkApp4 (mkConst ``And.intro)
        prop1 (mkConst ``True) innerExpr (mkConst ``True.intro)
      let outerExpr ← hiddenTheorem prop2 outerProof
      let outerName := outerExpr.getAppFn.constName!
      let abandonedProof := mkApp3 (mkConst ``Or.inl)
        (mkConst ``True) (mkConst ``False) (mkConst ``True.intro)
      let abandonedExpr ← hiddenTheorem propAbandoned abandonedProof
      let abandonedName := abandonedExpr.getAppFn.constName!
      assertRegistryEnvAgreement state
      pure (innerName, outerExpr, outerName, abandonedName)
    saved.restore
    if (← getEnv).contains innerName || (← getEnv).contains outerName || (← getEnv).contains abandonedName then
      throwError "rolled-back hidden theorems unexpectedly remained in the environment"
    if outerExpr.getUsedConstants != #[outerName] then
      throwError "outer expression had unexpected direct constants"
    declareHiddenTheorems outerExpr
    assertRegistryEnvAgreement state
    unless (← getEnv).contains innerName do
      throwError m!"direct finalizer `declareHiddenTheorems` did not restore nested dependency `{innerName}`"
    unless (← getEnv).contains outerName do
      throwError m!"direct finalizer `declareHiddenTheorems` did not restore `{outerName}`"
    if (← getEnv).contains abandonedName then
      throwError m!"unreferenced rolled-back hidden theorem `{abandonedName}` was spuriously restored"

    -- 2. Statement-deduplication boundary control (`hiddenTheorem` hitting
    -- `state.hiddenStatements` at `Solve/Base.lean:1753–1756` after rollback):
    -- Roll back again so both `innerName` and `outerName` are absent from `env`,
    -- then call `hiddenTheorem prop2 directProof2` where `directProof2` does not
    -- mention `innerName` or `outerName`. `hiddenTheorem` finds `earlier = outerName`
    -- in `state.hiddenStatements` and calls `declareHiddenTheorems (mkConst outerName)`
    -- where only `outerName` is mentioned directly and `outerName`'s checked body
    -- in `state.hiddenTheorems` retains an actual edge to the absent `innerName`.
    -- The old direct-constant finalizer fails this control in the kernel.
    saved.restore
    if (← getEnv).contains innerName || (← getEnv).contains outerName then
      throwError "rolled-back hidden theorems unexpectedly remained after second restore"
    let directProof2 := mkApp4 (mkConst ``And.intro)
      prop1 (mkConst ``True)
      (mkApp4 (mkConst ``And.intro)
        (mkConst ``True) (mkConst ``True) (mkConst ``True.intro) (mkConst ``True.intro))
      (mkConst ``True.intro)
    let dedupExpr ← withOptions (Elab.async.set · false) do
      hiddenTheorem prop2 directProof2
    if dedupExpr.getAppFn.constName! != outerName then
      throwError m!"expected statement deduplication to reuse `{outerName}`, got `{dedupExpr}`"
    assertRegistryEnvAgreement state
    unless (← getEnv).contains innerName && (← getEnv).contains outerName do
      throwError "statement deduplication did not restore `innerName` and `outerName` in dependency order"
    if (← getEnv).contains abandonedName then
      throwError m!"unreferenced rolled-back hidden theorem `{abandonedName}` was spuriously restored"

    -- 3. Shared-certificate creation control (`SharedContinuations.certifySharedProof`):
    -- Roll back again so `innerName` and `outerName` are absent from `env`, then
    -- certify `certExpr` (`certifySharedProof prop2 outerExpr`), which restores
    -- `innerName` and `outerName` via `prepareCertificate` before `mkAuxTheorem`.
    saved.restore
    if (← getEnv).contains innerName || (← getEnv).contains outerName then
      throwError "rolled-back hidden theorems unexpectedly remained after third restore"
    let certExpr ← withOptions (Elab.async.set · false) do
      SharedContinuations.certifySharedProof prop2 outerExpr declareHiddenTheorems
    let certName := certExpr.getAppFn.constName!
    if certExpr.getUsedConstants != #[certName] then
      throwError "certificate root expression had unexpected direct constants"
    assertRegistryEnvAgreement state
    unless (← getEnv).contains innerName && (← getEnv).contains outerName do
      throwError "certificate creation did not restore reachable hidden dependencies"
    if (← getEnv).contains abandonedName then
      throwError m!"unreferenced rolled-back hidden theorem `{abandonedName}` was spuriously restored"
    withOptions (Elab.async.set · false) do
      addDecl (.thmDecl {
        name := rootName
        levelParams := []
        type := prop2
        value := certExpr
      })

    -- 4. Fail-closed negative check: an unregistered constant is neither restored nor accepted.
    let unregistered := rootName ++ `_unregistered_missing
    declareHiddenTheorems (mkConst unregistered)
    if (← getEnv).contains unregistered then
      throwError m!"unregistered constant `{unregistered}` was spuriously declared"
    let rejected ← try
      withOptions (Elab.async.set · false) do
        addDecl (.thmDecl {
          name := rootName ++ `_bad
          levelParams := []
          type := mkConst ``True
          value := mkConst unregistered
        })
      pure false
    catch _ =>
      pure true
    unless rejected do
      throwError "kernel unexpectedly accepted an unregistered constant"

end VerexIR.Tests.HiddenTheoremClosure
