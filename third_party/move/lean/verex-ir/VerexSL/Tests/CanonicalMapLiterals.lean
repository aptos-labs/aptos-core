-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Proofs

open Lean Meta Elab Command VerexIR VerexIR.Proofs.Denote

private def namedKeys : List Int := [1, 2, 3]
private def namedKey : Int := 1
private def namedEntries : List (Int × Int) := [(0, 5)]

-- Observe an explicit closed construction at the same producer boundary that
-- specializes call contracts. Named maps and values retain their definitions.
run_cmd liftTermElabM do
  let elaborate (term : TSyntax `term) : TermElabM Lean.Expr := do
    instantiateMVars (← Term.elabTerm term none)
  let construction ← elaborate (← `(SpecMap.updateAll (SpecMap.Discipline.ordered SpecMap.intOrder)
    ([] : List (Int × Int)) [1, 2, 3] [10, 20, 30]))
  let result ← canonicalFamilyResult construction
  let some (_, entries) := result.expr.listLit?
    | throwError "explicit literal updateAll was not materialized"
  unless entries.length == 3 do throwError "literal construction changed entry count"
  unless ← isDefEq construction result.expr do
    throwError "literal construction changed the map"
  for term in [
      ← `(SpecMap.updateAll (SpecMap.Discipline.ordered SpecMap.intOrder)
        ([] : List (Int × Int)) namedKeys [10, 20, 30]),
      ← `(SpecMap.updateAll (SpecMap.Discipline.ordered SpecMap.intOrder)
        ([] : List (Int × Int)) [namedKey] [10]),
      ← `(SpecMap.updateAll (SpecMap.Discipline.ordered SpecMap.intOrder)
        namedEntries [1] [10])] do
    let source ← elaborate term
    let result ← canonicalFamilyResult source
    unless result.expr == source do
      throwError "literal observation unfolded a named input: {result.expr}"
