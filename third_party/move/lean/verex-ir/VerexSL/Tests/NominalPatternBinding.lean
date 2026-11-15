-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Tests.NominalQuotation

open Lean Meta Elab Command VerexSL VerexSL.Contract VerexIR.Proofs.Denote

private def wildcardPatterns (context : NRow) : (row : NRow) → Pats context row
  | .nil => .nil
  | .cons _ rest => .cons .wildcard (wildcardPatterns context rest)

-- Pattern matching observes one nominal layer, including at symbolic
-- families and nested generic fields, while leaving unknown types unchanged.
run_cmd do
  let some unit := registeredUnit? (← getEnv) `«0x42».nominal_quotation
    | throwError "missing nominal pattern fixture"
  let name ← ensureUnitDefinition #["0x42", "nominal_quotation"] unit
  liftTermElabM <| withNominalUnit name do
    let procs ← Simp.SimprocsArray.add #[] ``nominalPatternBinding false
    let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
    withLocalDeclD `family (mkConst ``Carriers) fun family =>
      withLocalDeclD `unknown (mkConst ``NTy) fun unknown =>
        withLocalDeclD `pattern (mkApp2 (mkConst ``Pat) (mkConst ``NRow.nil) unknown) fun pattern =>
          withLocalDeclD `value (mkApp2 (mkConst ``NTy.carrier) family unknown) fun value =>
            withLocalDeclD `locals (mkApp2 (mkConst ``HEnv) family (mkConst ``NRow.nil)) fun locals => do
              let call := mkAppN (mkConst ``Pat.bindValue)
                #[family, mkConst ``NRow.nil, unknown, pattern, value, locals]
              let (result, _) ← simp call context procs
              unless result.expr == call do
                throwError "nominal pattern binding unfolded an unknown type: {result.expr}"
    for index in [0, 5] do
      let some schema := structNTy unit ⟨⟨0⟩, index⟩ | throwError "missing pattern schema"
      let owner ← quoteNTy schema
      let patterns : Array Lean.Expr ← match schema with
        | .struct source arguments fields =>
            pure #[← VerexSL.Verify.quotePat (τ := .struct source arguments fields)
              (.struct (wildcardPatterns .nil fields))]
        | .enum source arguments (firstName :: secondName :: restNames) (.cons first (.cons second rest)) distinct =>
            let owner := NTy.enum source arguments (firstName :: secondName :: restNames)
              (.cons first (.cons second rest)) distinct
            pure #[← VerexSL.Verify.quotePat (τ := owner)
              (.variant .here (wildcardPatterns .nil first)),
              ← VerexSL.Verify.quotePat (τ := owner)
                (.variant (.there .here) (wildcardPatterns .nil second))]
        | _ => throwError "unexpected pattern schema"
      for pattern in patterns do
        withLocalDeclD `family (mkConst ``Carriers) fun family =>
          withLocalDeclD `locals (mkApp2 (mkConst ``HEnv) family (mkConst ``NRow.nil)) fun locals => do
            let carrier := mkApp2 (mkConst ``NTy.carrier) family owner
            withLocalDeclD `value carrier fun value => do
              let call := mkAppN (mkConst ``Pat.bindValue)
                #[family, mkConst ``NRow.nil, owner, pattern, value, locals]
              let (result, _) ← simp call context procs
              if result.expr == call then throwError "named pattern did not select its binding equation"
              let some proof := result.proof? | throwError "pattern binding has no equation"
              unless ← isTypeCorrect proof do throwError "pattern binding equation is ill typed"
