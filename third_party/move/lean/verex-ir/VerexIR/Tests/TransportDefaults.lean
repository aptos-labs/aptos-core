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
  withLocalDecl `family .instImplicit (mkConst ``Sorts) fun family => do
    let θ ← Term.elabTerm (← `((⟨NRow.nil, rfl⟩ : TypeArgs))) none
    let τ := mkConst ``NTy.bool
    let carrier := mkApp2 (mkConst ``NTy.carrier) family τ
    let induced := mkApp2 (mkConst ``Sorts.instantiate) θ family
    let nested := mkApp2 (mkConst ``Sorts.instantiate) θ induced
    let proof ← mkEqRefl (mkConst ``Bool.true)
    -- Structural `NTy.inhabitantOf` normalization across `Carriers.instantiate`:
    -- negative check first (outer family stays untouched), then induced and nested frames.
    let ownInhabitant := mkApp3 (mkConst ``NTy.inhabitantOf) family τ proof
    checkTransportHandler ``inhabitantOuterFamily true ownInhabitant ownInhabitant
    checkTransportHandler ``inhabitantOuterFamily true
      (mkApp3 (mkConst ``NTy.inhabitantOf) induced τ proof) ownInhabitant
    checkTransportHandler ``inhabitantOuterFamily true
      (mkApp3 (mkConst ``NTy.inhabitantOf) nested τ proof) ownInhabitant
    -- Observable `SpecMap.valueAt` default normalization (`NTy.plainFallback`):
    let ownDefault := mkApp3 (mkConst ``NTy.plainFallback) family τ proof
    let ownInstance := mkApp3 (mkConst ``nativeInhabited) family τ ownDefault
    let key := mkConst ``Int
    let decEq ← synthInstance (← mkAppM ``DecidableEq #[key])
    let mapType := mkApp2 (mkConst ``VerexIR.SpecMap) key carrier
    withLocalDeclD `map mapType fun map => do
      let index ← Term.elabTerm (← `((0 : Int))) none
      let expected := mkAppN (mkConst ``VerexIR.SpecMap.valueAt)
        #[key, carrier, decEq, ownInstance, map, index]
      -- Negative check first: an independent callee default is not the transported caller default.
      withLocalDeclD `independent carrier fun independent => do
        let independentInstance := mkApp3 (mkConst ``nativeInhabited) induced τ independent
        let original := mkAppN (mkConst ``VerexIR.SpecMap.valueAt)
          #[key, carrier, decEq, independentInstance, map, index]
        checkTransportHandler ``valueAtOwnInstance false original original
      -- Positive checks: `NTy.plainFallback` at induced and nested `Carriers.instantiate` frames.
      let read (frame : Expr) : Expr :=
        let fallback := mkApp3 (mkConst ``NTy.plainFallback) frame τ proof
        let instanceExpr := mkApp3 (mkConst ``nativeInhabited) frame τ fallback
        mkAppN (mkConst ``VerexIR.SpecMap.valueAt) #[key, carrier, decEq, instanceExpr, map, index]
      checkTransportHandler ``valueAtOwnInstance false (read induced) expected
      checkTransportHandler ``valueAtOwnInstance false (read nested) expected
    withLocalDeclD `value carrier fun value => do
      let transported ← mkAppOptM ``NTy.ofSkolem
        #[some family, some θ, some τ, some value]
      checkTransportHandler ``paramFreeValueTransport false transported value
    let row := mkApp2 (mkConst ``NRow.cons) τ (mkConst ``NRow.nil)
    withLocalDeclD `values (mkApp2 (mkConst ``HList) family row) fun values => do
      let transported ← mkAppOptM ``HList.ofSkolem
        #[some family, some θ, some row, some values]
      checkTransportHandler ``paramFreeRowTransport false transported values
    withLocalDeclD `values (mkApp (mkConst ``Array [Level.zero]) carrier) fun values => do
      let transport := mkAppN (mkConst ``NTy.toSkolem) #[family, θ, τ]
      let mapped ← mkAppM ``Array.map #[transport, values]
      checkTransportHandler ``mapTransportIdentity false mapped values
    withLocalDeclD `value (mkApp (mkConst ``Option [Level.zero]) carrier) fun value => do
      let transport := mkAppN (mkConst ``NTy.toSkolem) #[family, θ, τ]
      let mapped ← mkAppM ``Option.map #[transport, value]
      checkTransportHandler ``optionTransportIdentity false mapped value
    let encoded ← FamilyTransport.encodingEq induced family θ τ τ
    unless ← isTypeCorrect encoded do throwError "transported encoding proof is ill-typed"
    withLocalDeclD `left carrier fun left => do
      withLocalDeclD `right carrier fun right => do
        let input := mkAppN (mkConst ``NTy.equal)
          #[induced, mkConst ``Bool.false, τ, left, right]
        let transport := mkAppN (mkConst ``NTy.ofSkolem) #[family, θ, τ]
        let expected := mkAppN (mkConst ``NTy.equal)
          #[family, mkConst ``Bool.false, τ, mkApp transport left, mkApp transport right]
        checkTransportHandler ``equalOuterFamily true input expected

run_cmd liftTermElabM do
  withLocalDeclD `unit (mkConst ``VerexIR.Validation.ValidatedUnit) fun unit => do
    withLocalDecl `functions .instImplicit (mkApp (mkConst ``FunctionCarriers) unit) fun functions => do
      withLocalDecl `skolems .instImplicit (mkApp2 (mkConst ``Skolems) unit functions) fun skolems => do
        let family := mkApp (mkConst ``Carriers.toSorts) (mkApp3 (mkConst ``Skolems.toCarriers) unit functions skolems)
        let θ ← Term.elabTerm (← `((⟨NRow.nil, rfl⟩ : TypeArgs))) none
        let names ← Term.elabTerm (← `((["End", "Present"] : List String))) none
        let rows ← Term.elabTerm
          (← `((NRows.cons NRow.nil (NRows.cons (NRow.cons NTy.bool NRow.nil) NRows.nil)))) none
        let inducedFrame := mkApp4 (mkConst ``Skolems.instantiate) unit functions θ skolems
        let induced := mkApp (mkConst ``Carriers.toSorts) (mkApp3 (mkConst ``Skolems.toCarriers) unit functions inducedFrame)
        withLocalDeclD `value (mkApp3 (mkConst ``variantCarrier) induced names rows) fun value => do
          let transported ← mkAppOptM ``variantCarrier.ofSkolem
            #[some family, some θ, some names, some rows, some value]
          let input ← mkAppOptM ``variantName
            #[some family, some names, none, some transported]
          let atInner := mkApp4 (mkConst ``variantName) induced names rows value
          checkTransportHandler ``variantNameTransport true input atInner
          let atOuter := mkApp4 (mkConst ``variantName) family names rows value
          checkTransportHandler ``variantNameOuterFamily true atInner atOuter

-- Kernel controls for structural `NTy.inhabitantOf` and `NTy.readBack` across `Carriers.instantiate`.
namespace VerexIR.Tests.TransportDefaults

@[reducible] private def boolParamCarriers (b : Bool) : Carriers :=
  let base := Carriers.instantiate ⟨.cons .bool .nil, rfl⟩ Carriers.ground
  { base with
    inhabited := fun index =>
      match index with
      | 0 => ⟨b⟩
      | n + 1 => base.inhabited (n + 1) }

-- Negative 1 (first): independent `Carriers` families with distinct parameter inhabitants
-- yield distinct `NTy.inhabitantOf` values at `.param 0`.
theorem inhabitantOf_independent_families_ne :
    @NTy.inhabitantOf ((boolParamCarriers true)).toSorts (.param 0) rfl ≠
      @NTy.inhabitantOf ((boolParamCarriers false)).toSorts (.param 0) rfl := by
  intro h
  unfold NTy.inhabitantOf at h
  change true = false at h
  cases h

-- Negative 2: independent `Carriers` families with distinct parameter inhabitants
-- yield distinct `NTy.readBack` fallbacks on an unencoded runtime value (`.unit`).
theorem readBack_independent_families_ne :
    @NTy.readBack ((boolParamCarriers true)).toSorts (.tuple (.cons (.param 0) .nil)) rfl .unit ≠
      @NTy.readBack ((boolParamCarriers false)).toSorts (.tuple (.cons (.param 0) .nil)) rfl .unit := by
  intro h
  rw [@NTy.readBack_unit ((boolParamCarriers true)).toSorts (.tuple (.cons (.param 0) .nil)) rfl,
    @NTy.readBack_unit ((boolParamCarriers false)).toSorts (.tuple (.cons (.param 0) .nil)) rfl] at h
  change @NTy.inhabitantOf ((boolParamCarriers true)).toSorts (.tuple (.cons (.param 0) .nil)) rfl =
    @NTy.inhabitantOf ((boolParamCarriers false)).toSorts (.tuple (.cons (.param 0) .nil)) rfl at h
  unfold NTy.inhabitantOf at h
  simp only [NTy.inhabitant, HList.inhabitant] at h
  exact Bool.noConfusion (Prod.mk.inj h).1

-- Positive 1: `NTy.inhabitantOf` at an induced `Carriers.instantiate` family agrees
-- with the outer family's `NTy.inhabitantOf` at the substituted type (`NTy.inhabitantOf_instantiate`).
theorem inhabitantOf_instantiate_param [family : Carriers] :
    @NTy.inhabitantOf ((Carriers.instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) family)).toSorts
        (.param 0) rfl =
      @NTy.inhabitantOf (family).toSorts .bool rfl :=
  NTy.inhabitantOf_instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) (.param 0) rfl

-- Positive 2: `NTy.inhabitantOf` on a compound parameterized tuple agrees across
-- single and nested `Carriers.instantiate` frames.
theorem inhabitantOf_instantiate_tuple [family : Carriers] :
    @NTy.inhabitantOf ((Carriers.instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) family)).toSorts
        (.tuple (.cons (.param 0) .nil)) rfl =
      @NTy.inhabitantOf (family).toSorts (.tuple (.cons .bool .nil)) rfl :=
  eq_of_heq (FamilyTransport.inhabitantOf_instantiate_heq
    (⟨.cons .bool .nil, rfl⟩ : TypeArgs) (.tuple (.cons (.param 0) .nil)) rfl rfl)

theorem inhabitantOf_instantiate_nested_tuple [family : Carriers] :
    let outer := Carriers.instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) family
    let nested := Carriers.instantiate (⟨.cons (.param 0) .nil, rfl⟩ : TypeArgs) outer
    @NTy.inhabitantOf (nested).toSorts (.tuple (.cons (.param 0) .nil)) rfl =
      @NTy.inhabitantOf (family).toSorts (.tuple (.cons .bool .nil)) rfl := by
  intro outer nested
  have step1 :
      @NTy.inhabitantOf (nested).toSorts (.tuple (.cons (.param 0) .nil)) rfl =
        @NTy.inhabitantOf (outer).toSorts (.tuple (.cons (.param 0) .nil)) rfl :=
    eq_of_heq (@FamilyTransport.inhabitantOf_instantiate_heq outer.toSorts
      (⟨.cons (.param 0) .nil, rfl⟩ : TypeArgs) (.tuple (.cons (.param 0) .nil)) rfl rfl)
  exact step1.trans (@inhabitantOf_instantiate_tuple family)

-- Positive 3: `NTy.readBack` at `.param 0` under `Carriers.instantiate` is definitionally
-- the outer family's `NTy.readBack` at the substituted type, and on `.unit` agrees directly
-- with the outer family's structural `NTy.inhabitantOf`.
theorem readBack_instantiate_param [family : Carriers] :
    @NTy.readBack ((Carriers.instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) family)).toSorts
        (.param 0) rfl =
      @NTy.readBack (family).toSorts .bool rfl := rfl

theorem readBack_unit_instantiate_eq_inhabitantOf [family : Carriers] (source : VerexIR.StructHandle) :
    let τ : NTy := .struct source .nil (.cons .bool .nil)
    let θ : TypeArgs := ⟨.cons τ .nil, rfl⟩
    @NTy.readBack ((Carriers.instantiate θ family)).toSorts (.param 0) rfl .unit =
      @NTy.inhabitantOf (family).toSorts τ rfl := by
  intro τ θ
  exact @NTy.readBack_unit (family).toSorts τ rfl

-- Positive 4: `NTy.readBack` on a compound parameterized tuple agrees across
-- `Carriers.instantiate` for every runtime value `raw`, and on `.unit` equals the
-- outer family's structural `NTy.inhabitantOf`.
theorem readBack_instantiate_tuple [family : Carriers] (raw : VerexIR.RuntimeValue) :
    @NTy.readBack ((Carriers.instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) family)).toSorts
        (.tuple (.cons (.param 0) .nil)) rfl raw =
      @NTy.readBack (family).toSorts (.tuple (.cons .bool .nil)) rfl raw :=
  congrArg (((@NTy.codec (family).toSorts (.tuple (.cons .bool .nil))).decode? raw).getD ·)
    inhabitantOf_instantiate_tuple

theorem readBack_unit_instantiate_tuple_eq_inhabitantOf [family : Carriers] :
    @NTy.readBack ((Carriers.instantiate (⟨.cons .bool .nil, rfl⟩ : TypeArgs) family)).toSorts
        (.tuple (.cons (.param 0) .nil)) rfl .unit =
      @NTy.inhabitantOf (family).toSorts (.tuple (.cons .bool .nil)) rfl :=
  (readBack_instantiate_tuple .unit).trans (@NTy.readBack_unit (family).toSorts (.tuple (.cons .bool .nil)) rfl)

open Lean Elab Command in
run_cmd do
  for name in [``inhabitantOf_independent_families_ne, ``readBack_independent_families_ne,
      ``inhabitantOf_instantiate_param, ``inhabitantOf_instantiate_tuple,
      ``inhabitantOf_instantiate_nested_tuple, ``readBack_instantiate_param,
      ``readBack_unit_instantiate_eq_inhabitantOf, ``readBack_instantiate_tuple,
      ``readBack_unit_instantiate_tuple_eq_inhabitantOf] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end VerexIR.Tests.TransportDefaults
