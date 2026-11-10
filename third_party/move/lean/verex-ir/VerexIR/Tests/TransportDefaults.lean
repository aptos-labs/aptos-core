-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Normal

open Lean Meta Elab Command VerexIR.Proofs.Denote

private def checkTransportHandler (name : Name) (pre : Bool) (input expected : Expr) : MetaM Unit := do
  let procs ← Simp.SimprocsArray.add #[] name pre
  let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
  let (result, _) ← simp input context procs
  if input == expected then
    unless result.expr == input do throwError "{name} changed an independent default"
  else
    if result.expr == input then throwError "{name} did not run"
    unless ← withTransparency .default (isDefEq result.expr expected) do
      throwError "{name} produced {result.expr}, expected {expected}"
  if let some proof := result.proof? then
    unless ← isTypeCorrect proof do throwError "{name} produced an ill-typed proof"

-- Handler-only checks prevent unrelated reductions from hiding stale layouts.
run_cmd liftTermElabM do
  withLocalDecl `family .instImplicit (mkConst ``Carriers) fun family => do
    withLocalDecl `nonempty .instImplicit (mkApp (mkConst ``CarriersNonempty) family) fun nonempty => do
      let θ ← Term.elabTerm (← `((⟨NRow.nil, rfl⟩ : TypeArgs))) none
      let τ := mkConst ``NTy.bool
      let carrier := mkApp2 (mkConst ``NTy.carrier) family τ
      withLocalDeclD `value carrier fun value => do
        let transported ← mkAppOptM ``NTy.ofSkolem
          #[some family, some nonempty, some θ, some τ, some value]
        checkTransportHandler ``paramFreeValueTransport false transported value
      let row := mkApp2 (mkConst ``NRow.cons) τ (mkConst ``NRow.nil)
      withLocalDeclD `values (mkApp2 (mkConst ``HList) family row) fun values => do
        let transported ← mkAppOptM ``HList.ofSkolem
          #[some family, some nonempty, some θ, some row, some values]
        checkTransportHandler ``paramFreeRowTransport false transported values
      withLocalDeclD `values (mkApp (mkConst ``Array [Level.zero]) carrier) fun values => do
        let transport := mkAppN (mkConst ``NTy.toSkolem) #[family, nonempty, θ, τ]
        let mapped ← mkAppM ``Array.map #[transport, values]
        checkTransportHandler ``mapTransportIdentity false mapped values
      withLocalDeclD `value (mkApp (mkConst ``Option [Level.zero]) carrier) fun value => do
        let transport := mkAppN (mkConst ``NTy.toSkolem) #[family, nonempty, θ, τ]
        let mapped ← mkAppM ``Option.map #[transport, value]
        checkTransportHandler ``optionTransportIdentity false mapped value
      let induced := mkApp3 (mkConst ``Carriers.instantiate) θ family nonempty
      let encoded ← FamilyTransport.encodingEq induced family θ τ τ
      unless ← isTypeCorrect encoded do throwError "transported encoding proof is ill-typed"
      -- The induced family records which outer witness it transports. A
      -- second instance in scope must not replace that recorded witness.
      withLocalDecl `distractor .instImplicit (mkApp (mkConst ``CarriersNonempty) family) fun _ => do
        let encoded ← FamilyTransport.encodingEq induced family θ τ τ
        let some transport := encoded.find? (·.isAppOfArity ``NTy.encode_ofSkolem 5)
          | throwError "encoding transport lost its named law"
        unless transport.getArg! 1 == nonempty do
          throwError "encoding transport captured another outer inhabitant instance"
        unless ← isTypeCorrect encoded do throwError "transported encoding proof is ill-typed"
      withLocalDeclD `left carrier fun left => do
        withLocalDeclD `right carrier fun right => do
          let input := mkAppN (mkConst ``NTy.equal)
            #[induced, mkConst ``Bool.false, τ, left, right]
          let transport := mkAppN (mkConst ``NTy.ofSkolem) #[family, nonempty, θ, τ]
          let expected := mkAppN (mkConst ``NTy.equal)
            #[family, mkConst ``Bool.false, τ, mkApp transport left, mkApp transport right]
          checkTransportHandler ``equalOuterFamily true input expected
      let canonical := mkApp3 (mkConst ``CarriersNonempty.instantiate) θ family nonempty
      let proof ← mkEqRefl (mkConst ``Bool.true)
      let ownDefault := mkApp4 (mkConst ``NTy.inhabitantOf) family nonempty τ proof
      let ownInstance := mkApp3 (mkConst ``nativeInhabited) family τ ownDefault
      let key := mkConst ``Int
      let decEq ← synthInstance (← mkAppM ``DecidableEq #[key])
      let mapType := mkApp2 (mkConst ``VerexIR.SpecMap) key carrier
      withLocalDeclD `map mapType fun map => do
        let index ← Term.elabTerm (← `((0 : Int))) none
        let expected := mkAppN (mkConst ``VerexIR.SpecMap.valueAt)
          #[key, carrier, decEq, ownInstance, map, index]
        let read (frame defaults : Expr) : Expr :=
          let fallback := mkApp4 (mkConst ``NTy.inhabitantOf) frame defaults τ proof
          let instanceExpr := mkApp3 (mkConst ``nativeInhabited) frame τ fallback
          mkAppN (mkConst ``VerexIR.SpecMap.valueAt) #[key, carrier, decEq, instanceExpr, map, index]
        checkTransportHandler ``valueAtOwnInstance false (read induced canonical) expected
        let nested := mkApp3 (mkConst ``Carriers.instantiate) θ induced canonical
        let nestedDefaults := mkApp3 (mkConst ``CarriersNonempty.instantiate) θ induced canonical
        checkTransportHandler ``valueAtOwnInstance false (read nested nestedDefaults) expected
        -- An independent callee default is not the transported caller default.
        withLocalDeclD `independent (mkApp (mkConst ``CarriersNonempty) induced) fun independent => do
          let original := read induced independent
          checkTransportHandler ``valueAtOwnInstance false original original

run_cmd liftTermElabM do
  withLocalDeclD `unit (mkConst ``VerexIR.Validation.ValidatedUnit) fun unit => do
    withLocalDecl `functions .instImplicit (mkApp (mkConst ``FunctionCarriers) unit) fun functions => do
      withLocalDecl `skolems .instImplicit (mkApp2 (mkConst ``Skolems) unit functions) fun skolems => do
        let family := mkApp3 (mkConst ``Skolems.toCarriers) unit functions skolems
        let nonempty := mkApp3 (mkConst ``Skolems.carriersNonempty) unit functions skolems
        let θ ← Term.elabTerm (← `((⟨NRow.nil, rfl⟩ : TypeArgs))) none
        let names ← Term.elabTerm (← `((["End", "Present"] : List String))) none
        let rows ← Term.elabTerm
          (← `((NRows.cons NRow.nil (NRows.cons (NRow.cons NTy.bool NRow.nil) NRows.nil)))) none
        let inducedFrame := mkApp4 (mkConst ``Skolems.instantiate) unit functions θ skolems
        let induced := mkApp3 (mkConst ``Skolems.toCarriers) unit functions inducedFrame
        withLocalDeclD `value (mkApp3 (mkConst ``variantPlain) induced names rows) fun value => do
          let transported ← mkAppOptM ``variantPlain.ofSkolem
            #[some family, some nonempty, some θ, some names, some rows, some value]
          let input ← mkAppOptM ``variantPlainName
            #[some family, some names, none, some transported]
          let atInner := mkApp4 (mkConst ``variantPlainName) induced names rows value
          checkTransportHandler ``variantNameTransport true input atInner
          let atOuter := mkApp4 (mkConst ``variantPlainName) family names rows value
          checkTransportHandler ``variantNameOuterFamily true atInner atOuter
