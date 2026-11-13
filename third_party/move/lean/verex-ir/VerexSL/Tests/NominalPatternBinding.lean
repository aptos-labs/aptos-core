-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Tests.NominalQuotation

open Lean Meta Elab Command VerexSL VerexSL.Contract VerexIR.Proofs.Denote

private def wildcardPatterns (context : NRow) : (row : NRow) → Pats context row
  | .nil => .nil
  | .cons _ rest => .cons .wildcard (wildcardPatterns context rest)

-- Pattern matching observes one nominal layer, including at symbolic
-- families and nested generic fields. The resulting equation is checked.
run_cmd do
  let some unit := registeredUnit? (← getEnv) `«0x42».nominal_quotation
    | throwError "missing nominal pattern fixture"
  let name ← ensureUnitDefinition #["0x42", "nominal_quotation"] unit
  liftTermElabM <| withNominalUnit name do
    let certified ← Simp.SimprocsArray.add #[] ``nominalPatternBinding false
    let plain ← Simp.SimprocsArray.add #[] ``nominalPlainPatternBinding false
    let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
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
            for (operation, procs, certifiedValue) in
                [(``Pat.bindValue, certified, true), (``Pat.bindPlain, plain, false)] do
              let carrier := mkApp2 (mkConst (if certifiedValue then ``NTy.carrier else ``NTy.plain))
                family owner
              withLocalDeclD `value carrier fun value => do
                let initialArgs := #[family, mkConst ``NRow.nil, owner, pattern, value]
                let check (call : Lean.Expr) : MetaM Unit := do
                  let (result, _) ← simp call context procs
                  if result.expr == call then throwError "named pattern did not select its binding equation"
                  let some proof := result.proof? | throwError "pattern binding has no equation"
                  unless ← isTypeCorrect proof do throwError "pattern binding equation is ill typed"
                if certifiedValue then
                  check (mkAppN (mkConst operation) (initialArgs.push locals))
                else
                  withLocalDeclD `holds (mkApp3 (mkConst ``NTy.deepHolds) family owner value) fun holds =>
                    check (mkAppN (mkConst operation) (initialArgs ++ #[holds, locals]))
