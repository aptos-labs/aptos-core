-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
open Lean Meta Elab Command VerexSL VerexSL.Contract VerexIR.Proofs.Denote

-- Scalar comparisons must not enter the default-transparency certified
-- closer, even when their arguments explicitly observe a certified value.
example (order : Ordering) : order = order := by
  fail_if_success verex_certified_congr
  rfl

-- A non-equational clause is also outside that closer's scope.
example (p : Prop) (held : p) : p := by
  fail_if_success verex_certified_congr_applies
  exact held

verex module 0x42::field_projection_view where
  pragma verify = false
  struct Leaf {T} has Copy, Drop where
    value : T

run_cmd do
  let some unit := registeredUnit? (← getEnv) `«0x42».field_projection_view
    | throwError "missing unit"
  let name ← ensureUnitDefinition #["0x42", "field_projection_view"] unit
  liftTermElabM do
    withNominalUnit name do
      let some leaf := structNTy unit ⟨⟨0⟩, 0⟩ | throwError "missing leaf"
      let concrete := leaf.subst (.cons .bool .nil)
      let quoted ← quoteNTy concrete
      withLocalDeclD `family (mkConst ``Carriers) fun family => do
        let checkView (carriers type : Lean.Expr) : MetaM Unit := do
          let some view ← NativeMaps.typedView? carriers type | throwError "missing view"
          unless view.type == quoted && view.carrier == carrierType family quoted do
            throwError "view expanded descriptor: {view.type}"
          unless view.read.isLambda && view.read.bindingDomain! == carrierType family quoted do
            throwError "view identity binder expanded descriptor"
        checkView family quoted
        let row ← quoteRow (.cons concrete .nil)
        let predicate ← withLocalDeclD `row (mkConst ``NRow) fun row => do
          mkLambdaFVars #[row] (← mkEq (mkApp (mkConst ``NRow.inhabitable) row) (toExpr true))
        let θ ← mkAppOptM ``Subtype.mk #[mkConst ``NRow, predicate, row,
          ← inhabitableProof (← quoteNTy (.tuple (.cons concrete .nil)))]
        let induced := mkApp2 (mkConst ``Carriers.instantiate) θ family
        checkView induced (mkApp (mkConst ``NTy.param) (toExpr (0 : Nat)))
        let goal ← mkFreshExprMVar (mkConst ``True)
        discard <| Tactic.run goal.mvarId! do
          let (ctx, simprocs) ← normalization
          for (carriers, type) in #[(family, quoted),
              (induced, mkApp (mkConst ``NTy.param) (toExpr (0 : Nat)))] do
            let read := mkApp2 (mkConst ``NTy.read) carriers type
            let (result, _) ← Simp.main read ctx (methods := Simp.mkDefaultMethodsCore simprocs)
            unless result.expr.isLambda && result.expr.bindingDomain! == carrierType family quoted do
              throwError "read evaluator expanded descriptor: {result.expr}"
            if (result.expr.find? (·.isConstOf ``Eq.ndrec)).isSome then
              throwError "read contains transport"
          (← Tactic.getMainGoal).assign (mkConst ``True.intro)

        let parameter := mkApp (mkConst ``NTy.param) (toExpr (0 : Nat))
        let readDefault ← mkAppOptM ``NTy.readInhabited #[induced, parameter,
          ← inhabitableProof parameter]
        let quotedProof ← inhabitableProof quoted
        let expectedStructuralDefault :=
          mkApp3 (mkConst ``nativeInhabited) family quoted
            (mkApp3 (mkConst ``NTy.inhabitantOf) family quoted quotedProof)
        let expectedFallbackDefault :=
          mkApp3 (mkConst ``nativeInhabited) family quoted
            (mkApp3 (mkConst ``NTy.plainFallback) family quoted quotedProof)
        let some expectedView ← NativeMaps.typedView? family quoted | throwError "missing expected view"
        unless expectedView.inhabited == expectedFallbackDefault do
          throwError "view default did not use plainFallback: {expectedView.inhabited}"
        let goal ← mkFreshExprMVar (← mkEq readDefault readDefault)
        discard <| Tactic.run goal.mvarId! do
          canonicalFamiliesOf true
          let goal ← Tactic.getMainGoal
          let some (_, actual, _) := (← goal.getType).eq? | throwError "missing default equation"
          unless actual == expectedStructuralDefault do
            throwError "default did not preserve the canonical resolved witness: {actual}"
          goal.assign (← mkEqRefl actual)

run_meta withSyntheticNativeTypes do
  withLocalDeclD `family (mkConst ``Carriers) fun family => do
    let source : VerexIR.StructHandle := { namespaceId := ⟨0⟩, structId := 0 }
    let nativeType := NTy.struct source .nil
      (.cons .address (.cons (.table (.int 64 false) (.param 0)) (.cons (.param 0) .nil)))
    let type ← quoteNTy nativeType
    withLocalDeclD `value (carrierType family type) fun value => do
      let fields ← nativeComponents family type value
      unless fields.size == 3 do throwError "wrong field count"
      for (_, field) in fields do
        unless field.isAppOf ``Prod.fst do
          throwError "not a named row projection: {field}"
        discard <| inferType field

-- Field selection must specialize by conversion before any free-field simplification.
example [outer : Carriers] :
    @Var.select
      (Carriers.instantiate ⟨.cons (.int 8 false) (.cons (.int 64 false) .nil), by decide⟩ outer)
      (.cons (.table (.param 0) (.param 1)) .nil)
      (.table (.param 0) (.param 1)) .here =
    @Var.select outer (.cons (.table (.int 8 false) (.int 64 false)) .nil)
      (.table (.int 8 false) (.int 64 false)) .here := rfl

set_option maxHeartbeats 25000
set_option verex.verifyHeartbeats 25000
verex module 0x42::certified_nested_fields where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  struct Payload has Copy, Drop, Store where
    value : u64
  spec Payload where
    invariant value <= 10
  struct Cell {T} has Copy, Drop, Store where
    entries : Vector<T>
  spec Cell where
    invariant entries.length <= 1
  public fun count {T has Copy, Drop, Store}(t : &Table<u64, Cell<T> >, key : u64) -> u64 :=
    borrow(t, key).entries.length
  spec count where
    requires has(t, key)
    aborts_if false
    ensures result <= 1
  verify count

  -- A field read through two nested intrinsic-map borrows: the value's own
  -- invariant is observed at the concrete family of each borrow, so the bound
  -- on the payload's field follows within the module's budget.
  public fun nested(t : &Table<u64, Table<u64, Payload> >, a : u64, b : u64) -> u64 :=
    borrow(borrow(t, a), b).value
  spec nested where
    requires has(t, a)
    requires has(get(t, a), b)
    aborts_if false
    ensures result <= 10
  verify nested

  -- Returning a mutable table unchanged owes its intrinsic invariant. The
  -- invariant's generic field selector must be observed at the field's
  -- concrete family, even though its owner's certificate remains generic.
  public fun preserve_scalar(t : &mut Table<u64, u64>) -> Unit := ()
  verify preserve_scalar

  public fun preserve_nominal(t : &mut Table<u64, Payload>) -> Unit := ()
  verify preserve_nominal
