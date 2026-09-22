-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/- A failed later certificate may leave the completed marking definition in
the environment. Preparation must reuse it, not report a duplicate declaration.
Seed that partial state directly so the regression needs no deliberate timeout. -/

leaner module 0x42::preparation_retry where
  fun constant() -> u64 := 7
  spec constant where
    ensures result == 7
    aborts_if false

open Lean LeanerIR.Validation in
run_cmd do
  let segments := #["0x42", "preparation_retry"]
  let some unit := LeanerLang.registeredUnit? (← getEnv) `«0x42».preparation_retry
    | throwError "missing preparation-retry unit"
  discard <| LeanerLang.Contract.ensureUnitDefinition segments unit
  let name := Name.str (LeanerLang.Contract.semanticsName segments) "marked"
  Lean.Elab.Command.liftTermElabM do
    addDecl (.defnDecl {
      name, levelParams := []
      type := mkConst ``ValidatedUnit
      value := toExpr (markLoanDeaths unit).1
      hints := .abbrev, safety := .safe })
    enableRealizationsForConst name

#leaner_unit 0x42::preparation_retry
#leaner_unit 0x42::preparation_retry
#leaner_verify 0x42::preparation_retry::constant

open Lean in
run_cmd do
  for name in #[
      `«0x42».preparation_retry.semantics.marked_eq,
      `«0x42».preparation_retry.semantics.erasureIndexes_eq,
      `«0x42».preparation_retry.semantics.erasureIndexedChunks_eq,
      `«0x42».preparation_retry.semantics.erasureChunks_eq,
      `«0x42».preparation_retry.semantics.erasureSorted_eq,
      `«0x42».preparation_retry.semantics.erasurePlan_eq,
      `«0x42».preparation_retry.semantics.expressionArenasIndexed_eq,
      `«0x42».preparation_retry.semantics.expressionArenas_eq,
      `«0x42».preparation_retry.semantics.erasureArenasApplied_eq,
      `«0x42».preparation_retry.semantics.erasureApplied_eq,
      `«0x42».preparation_retry.semantics_eq,
      `«0x42».preparation_retry.constant.verified] do
    unless (← getEnv).contains name do throwError "missing preparation artifact: {name}"
    if (← collectAxioms name).contains ``sorryAx then
      throwError "preparation artifact contains an admission: {name}"
