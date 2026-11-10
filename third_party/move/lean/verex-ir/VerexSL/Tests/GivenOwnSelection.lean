-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Proofs

open Lean Meta Elab Command VerexIR VerexIR.Proofs.Denote

private def declarations (given : GivenFrame) (source : StructHandle) (_ : NRow) :
    Option (GivenOwnClause given) :=
  if source.structId == 0 then none else some ⟨.param 0, fun _ => False⟩

run_cmd liftTermElabM do
  let procs ← Simp.SimprocsArray.add #[] ``givenOwnWithoutInvariant false
  let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
  withLocalDeclD `given (mkConst ``GivenFrame) fun given =>
  withLocalDeclD `raw (mkConst ``RuntimeValue) fun raw => do
    let table := mkApp (mkConst ``declarations) given
    let installed := mkApp2 (mkConst ``GivenFrame.withDeclarations) given table
    let family := mkApp (mkConst ``GivenFrame.toCarriers) installed
    for index in #[0, 1] do
      let own := mkApp4 (mkConst ``Carriers.own) family
        (toExpr (⟨⟨0⟩, index⟩ : StructHandle)) (mkConst ``NRow.nil) raw
      let (result, _) ← simp own context procs
      if index == 0 then
        unless result.expr.isConstOf ``True do throwError "absent declaration was not selected"
        unless ← isTypeCorrect (← result.getProof) do throwError "invalid empty selection proof"
      else
        unless result.expr == own do throwError "present declaration was unfolded"
    let unknownType := mkApp (mkConst ``GivenOwnDeclarations) given
    withLocalDeclD `unknown unknownType fun unknown => do
      let installed := mkApp2 (mkConst ``GivenFrame.withDeclarations) given unknown
      let own := mkApp4 (mkConst ``Carriers.own)
        (mkApp (mkConst ``GivenFrame.toCarriers) installed)
        (toExpr (⟨⟨0⟩, 0⟩ : StructHandle)) (mkConst ``NRow.nil) raw
      let (result, _) ← simp own context procs
      unless result.expr == own do throwError "unknown declaration was changed"
