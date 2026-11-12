-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
open Lean Meta Elab Command VerexSL VerexSL.Contract VerexIR.Proofs.Denote

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
        withLocalDecl `nonempty .instImplicit (mkApp (mkConst ``CarriersNonempty) family) fun nonempty => do
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
            ← inhabitableProof (← quoteNTy (.tuple (.cons concrete .nil))) ]
          let induced := mkApp3 (mkConst ``Carriers.instantiate) θ family nonempty
          checkView induced (mkApp (mkConst ``NTy.param) (toExpr (0 : Nat)))
          -- The constructor's explicit witness is sufficient even when no
          -- ambient instance can be synthesized for the outer family.
          withLocalDeclD `explicitFamily (mkConst ``Carriers) fun explicitFamily => do
            withLocalDeclD `explicitNonempty (mkApp (mkConst ``CarriersNonempty) explicitFamily)
                fun explicitNonempty => do
              let induced := mkApp3 (mkConst ``Carriers.instantiate) θ explicitFamily explicitNonempty
              let some view ← NativeMaps.typedView? induced
                  (mkApp (mkConst ``NTy.param) (toExpr (0 : Nat)))
                | throwError "missing explicitly inhabited view"
              unless view.type == quoted && view.nonempty == explicitNonempty &&
                  view.carrier == carrierType explicitFamily quoted do
                throwError "view lost its constructor-supplied inhabitants"
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
          let readDefault ← mkAppOptM ``NTy.readInhabited #[induced, none, parameter,
            ← inhabitableProof parameter]
          let some expectedView ← NativeMaps.typedView? family quoted | throwError "missing expected view"
          let goal ← mkFreshExprMVar (← mkEq readDefault readDefault)
          discard <| Tactic.run goal.mvarId! do
            canonicalFamiliesOf true
            let goal ← Tactic.getMainGoal
            let some (_, actual, _) := (← goal.getType).eq? | throwError "missing default equation"
            unless actual == expectedView.inhabited do
              throwError "default did not preserve the canonical resolved witness: {actual}"
            goal.assign (← mkEqRefl actual)

-- A generic plain map view receives both certified role arguments and
-- plain values read through mutable references. Both use the same total
-- reader. The certified reading is preserved by readPlain_forget; arbitrary
-- plain values retain the fallback for a violated nominal invariant.
run_meta withSyntheticNativeTypes do
  withLocalDeclD `family (mkConst ``Carriers) fun family => do
    withLocalDecl `nonempty .instImplicit (mkApp (mkConst ``CarriersNonempty) family) fun nonempty => do
      let parameter ← quoteNTy (.param 0)
      let view ← typedViewOf family parameter (plain := true)
      let check (value expected : Lean.Expr) : MetaM Unit := do
        for actual in #[← TypedTable.typedOfNative view value,
            ← TypedTable.typedOf view (.param 0) value] do
          unless actual == expected do
            throwError "map argument lost its canonical reader: {actual}"
      withLocalDeclD `certified (carrierType family parameter) fun certified => do
        let forgotten := mkApp3 (mkConst ``NTy.forget) family parameter certified
        let expected := (mkApp view.read forgotten).headBeta
        check certified expected
        let agreement := mkAppN (mkConst ``NTy.readPlain_forget)
          #[family, nonempty, parameter, view.read.getArg! 3, certified]
        let certifiedRead := mkApp3 (mkConst ``NTy.read) family parameter certified
        unless ← withDefault (isDefEq (← inferType agreement) (← mkEq expected certifiedRead)) do
          throwError "the uniform plain reader changed a certified argument's reading"
      withLocalDeclD `plain (plainType family parameter) fun plain => do
        check plain ((mkApp view.read plain).headBeta)
      -- A resolved certificate-free argument keeps its identity view:
      -- its certified and plain carriers coincide.
      let arguments ← quoteTypeArgs ⟨.cons (.vector (.int 8 false)) .nil, rfl⟩
      let induced := mkApp3 (mkConst ``Carriers.instantiate) arguments family nonempty
      let view ← typedViewOf induced parameter (plain := true)
      withLocalDeclD `bytes (carrierType induced parameter) fun bytes => do
        let actual ← TypedTable.typedOfNative view bytes
        unless actual == bytes do
          throwError "certificate-free map argument lost its identity view"

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
        unless field.isAppOfArity ``Var.selectCertified 7 do
          throwError "not the shared certified field operation"
        discard <| inferType field
    withLocalDeclD `value (mkApp2 (mkConst ``NTy.plain) family type) fun value => do
      let fields ← nativeComponents family type value
      unless fields.size == 3 do throwError "wrong plain field count"
      for (_, field) in fields do
        if field.isAppOf ``Var.selectCertified then throwError "plain field was certified"

-- Field selection must specialize by conversion before any free-field simplification.
example [outer : Carriers] [CarriersNonempty outer] (source : VerexIR.StructHandle) :
    @Var.selectCertified
      (Carriers.instantiate ⟨.cons (.int 8 false) (.cons (.int 64 false) .nil), by decide⟩ outer)
      source .nil (.cons (.table (.param 0) (.param 1)) .nil)
      (.table (.param 0) (.param 1)) .here =
    @Var.selectCertified outer source .nil (.cons (.table (.int 8 false) (.int 64 false)) .nil)
      (.table (.int 8 false) (.int 64 false)) .here := rfl

-- Reading a constructed vector field recovers the original certified vector,
-- even when forgetting has exposed the elementwise map.
example [Carriers] (source : VerexIR.StructHandle) (τ : NTy)
    (value : (NTy.vector τ).carrier)
    (held : NTy.deepHolds (.struct source .nil (.cons (.vector τ) .nil))
      (NTy.forget (.vector τ) value, ())) :
    Var.selectCertified .here
      (NTy.ofDeepHolds (.struct source .nil (.cons (.vector τ) .nil))
        (NTy.forget (.vector τ) value, ()) held) = value := by
  verex_normalize

-- A constructed field recovers its certified input at every type, including
-- when a parameter's forgetting has already become its family's projection.
example [Carriers] (source : VerexIR.StructHandle) (τ : NTy)
    (value : τ.carrier)
    (held : NTy.deepHolds (.struct source .nil (.cons τ .nil))
      (NTy.forget τ value, ())) :
    Var.selectCertified .here
      (NTy.ofDeepHolds (.struct source .nil (.cons τ .nil))
        (NTy.forget τ value, ()) held) = value := by
  verex_normalize

example [Carriers] (source : VerexIR.StructHandle) (value : (NTy.param 0).carrier)
    (held : NTy.deepHolds (.struct source .nil (.cons (.param 0) .nil))
      ((Carriers.lent 0).of value, ())) :
    Var.selectCertified .here
      (NTy.ofDeepHolds (.struct source .nil (.cons (.param 0) .nil))
        ((Carriers.lent 0).of value, ()) held) = value := by
  verex_normalize

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

-- Scalar comparisons must not enter the default-transparency certified
-- closer, even when their arguments explicitly observe a certified value.
example (order : Ordering) : order = order := by
  fail_if_success verex_certified_congr
  rfl

example [Carriers] (value : Carriers.carrier 0)
    (compare : (Carriers.lent 0).carrier → Ordering) :
    compare ((Carriers.lent 0).of value) = compare ((Carriers.lent 0).of value) := by
  fail_if_success verex_certified_congr
  rfl

-- A non-equational clause is also outside that closer's scope.
example (p : Prop) (held : p) : p := by
  fail_if_success verex_certified_congr_applies
  exact held
