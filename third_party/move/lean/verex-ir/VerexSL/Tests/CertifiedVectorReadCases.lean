-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Proofs
open Lean Meta Elab Command VerexIR VerexIR.Proofs.Denote

run_cmd liftTermElabM do
  withLocalDeclD `family (mkConst ``Carriers) fun family =>
  withLocalDeclD `element (mkConst ``NTy) fun element => do
    let vectorType := mkApp (mkConst ``NTy.vector) element
    withLocalDeclD `plain (mkApp2 (mkConst ``NTy.plain) family vectorType) fun plain =>
    withLocalDeclD `held (mkApp3 (mkConst ``NTy.deepHolds) family vectorType plain) fun held => do
      let vector := mkApp4 (mkConst ``NTy.ofDeepHolds) family vectorType plain held
      let values ← mkAppM ``VerexIR.SpecVector.values #[vector]
      let lookup ← mkAppM ``GetElem?.getElem? #[values, mkNatLit 0]
      let check (result : Lean.Expr) : MetaM Unit := do
        withLocalDeclD `originalRead (← mkEq lookup result) fun equation => do
          let goal ← mkFreshExprMVar (mkConst ``True)
          let next ← assertPlainVectorLookup goal.mvarId! equation
          unless next != goal.mvarId! do throwError "certified vector case was not observed"
          next.withContext do
            unless (← getLCtx).contains equation.fvarId! do
              throwError "original certified lookup equation was discarded"
            let some observed := (← getLCtx).lastDecl | throwError "missing plain lookup"
            let plainValues ← mkAppM ``VerexIR.SpecVector.values #[plain]
            let plainLookup ← mkAppM ``GetElem?.getElem? #[plainValues, mkNatLit 0]
            let mapped ← mkAppM ``Option.map #[mkApp2 (mkConst ``NTy.forget) family element, result]
            let expected ← mkEq plainLookup (← whnfD mapped)
            unless ← withDefault (isDefEq observed.type expected) do
              throwError "case changed its actual forgotten result"
            unless ← isTypeCorrect observed.toExpr do throwError "ill-typed observation"
      let carrier := mkApp2 (mkConst ``NTy.carrier) family element
      withLocalDeclD `value carrier fun value => do
        check (← mkAppM ``Option.some #[value])
      check (mkApp (mkConst ``Option.none [0]) carrier)
