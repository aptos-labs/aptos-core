-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.NativeMaps
import VerexIR.Proofs.Denote.GivenOwn
import VerexIR.Proofs.Denote.Solve.Normal
import VerexSL.Contract.Values

open VerexIR VerexIR.Proofs.Denote

private def byteVector : NTy := .vector (.int 8 false)
private def byteVectorArgs : TypeArgs := ⟨.cons byteVector .nil, rfl⟩

-- A concrete certificate-free reader is the identity by construction.
example [Θ : Carriers] [CarriersNonempty Θ] :
    NTy.readPlain byteVector rfl = (fun value : @NTy.plain Θ byteVector => value) := rfl

-- Instantiating a plain parameter reaches that same reader.
example [Θ : Carriers] [CarriersNonempty Θ] :
    @NTy.readPlain (Carriers.instantiate byteVectorArgs Θ) inferInstance (.param 0) rfl =
      (fun value : @NTy.plain Θ byteVector => value) := rfl

-- The map receives exactly the concrete reader, even while it is unapplied.
example [Θ : Carriers] [CarriersNonempty Θ]
    (entries : SpecEntries (SpecInt (.bits 8) false × @NTy.plain Θ byteVector)) :
    SpecMap.ofEntries SpecInt.val (NTy.readPlain byteVector rfl) entries =
      SpecMap.ofEntries SpecInt.val (fun value => value) entries := rfl

-- A declaration can enforce an invariant even when all fields are scalar.
-- Its plain read retains the total decoder and its invalid-value fallback.
example [Θ : Carriers] [CarriersNonempty Θ] (source : StructHandle)
    (value : @NTy.plain Θ (.struct source .nil (.cons (.int 8 false) .nil))) :
    NTy.readPlain (.struct source .nil (.cons (.int 8 false) .nil)) rfl value =
      NTy.readBack (.struct source .nil (.cons (.int 8 false) .nil)) rfl
        (NTy.plainEncode (.struct source .nil (.cons (.int 8 false) .nil)) value) := rfl

-- Production call contracts use the projected carriers and inhabitants of an
-- instantiated Skolems frame. This path must retain the same reader directly.
example {unit : Validation.ValidatedUnit} [FunctionCarriers unit] (frame : Skolems unit) :
    @NTy.readPlain (Skolems.instantiate byteVectorArgs frame).toCarriers inferInstance
      (.param 0) rfl =
      (fun value : @NTy.plain frame.toCarriers byteVector => value) := rfl

example (frame : GivenFrame) [CarriersNonempty frame.toCarriers] (index : Nat) :
    @NTy.readPlain frame.toCarriers inferInstance (.param index) rfl =
      (fun value : frame.operations.carrier index => value) := rfl

private def nestedCertificate : NTy :=
  .vector (.struct ⟨⟨0⟩, 0⟩ .nil (.cons (.int 8 false) .nil))

-- The constructor preserves the descriptor it was given, and declines
-- identity reads whenever a nested value can carry a certificate.
open Lean Meta Elab Command in
run_cmd liftTermElabM do
  let family := mkConst ``Carriers.ground
  let type := mkConst ``byteVector
  let some (.lam _ domain body _) ← NativeMaps.canonicalPlainReader? family type
    | throwError "a free vector must have an identity reader"
  unless domain == mkApp2 (mkConst ``NTy.carrier) family type && body == .bvar 0 do
    throwError "the canonical reader must preserve its original type descriptor"
  if (← NativeMaps.canonicalPlainReader? family (mkConst ``nestedCertificate)).isSome then
    throwError "a nested nominal certificate must retain its total reader"
  let parameter := mkApp (mkConst ``NTy.param) (mkNatLit 0)
  let checkIdentity (family : Lean.Expr) : MetaM Unit := do
    let some (.lam _ domain body _) ← NativeMaps.canonicalPlainReader? family parameter
      | throwError "a known identity parameter reader was not retained"
    unless domain == mkApp2 (mkConst ``NTy.plain) family parameter && body == .bvar 0 do
      throwError "a parameter reader changed its plain domain"
  checkIdentity family
  withLocalDeclD `unknown (mkConst ``Carriers) fun unknown => do
    if (← NativeMaps.canonicalPlainReader? unknown parameter).isSome then
      throwError "an unresolved parameter must retain its family reader"
  withLocalDeclD `given (mkConst ``GivenFrame) fun given => do
    checkIdentity (mkApp (mkConst ``GivenFrame.toCarriers) given)
    checkIdentity (mkApp (mkConst ``GivenFrame.clauseScaffold) given)

private def scalarCertificate : NTy :=
  .struct ⟨⟨0⟩, 0⟩ .nil (.cons (.int 8 false) .nil)
private def nestedCertificateArgs : TypeArgs := ⟨.cons nestedCertificate .nil, rfl⟩
private def scalarCertificateArgs : TypeArgs := ⟨.cons scalarCertificate .nil, rfl⟩

-- Concrete certificate-bearing views must use the same typed sort and total
-- reader as a generic parameter instantiated at that type, including when
-- the certificate occurs below a vector constructor.
open Lean Meta Elab Command in
run_cmd liftTermElabM do
  let family := mkConst ``Carriers.ground
  withLocalDeclD `nonempty (mkApp (mkConst ``CarriersNonempty) family) fun nonempty => do
    withLocalInstances [← nonempty.fvarId!.getDecl] do
      for (type, args) in #[(mkConst ``nestedCertificate, mkConst ``nestedCertificateArgs),
          (mkConst ``scalarCertificate, mkConst ``scalarCertificateArgs)] do
        let some concrete ← NativeMaps.typedView? family type true
          | throwError "missing concrete certificate-bearing view"
        if concrete.identity || !concrete.plain then
          throwError "a certificate-bearing plain view must retain its total reader"
        let induced := mkApp3 (mkConst ``Carriers.instantiate) args family nonempty
        let some generic ← NativeMaps.typedView? induced (mkApp (mkConst ``NTy.param) (mkNatLit 0)) true
          | throwError "missing instantiated certificate-bearing view"
        for (name, left, right) in #[("carrier", concrete.carrier, generic.carrier),
            ("reader", concrete.read, generic.read), ("unread", concrete.unread, generic.unread)] do
          unless ← withNewMCtxDepth (withTransparency .default (isDefEq left right)) do
            throwError "concrete and generic {name} disagree"

-- The full emitted map uses canonical raw and typed types, not merely
-- definitionally equal typed-view fields. Existing plain identity lambdas
-- and generic readers specializing to a free vector take the same boundary.
open Lean Meta Elab Command in
run_cmd liftTermElabM do
  let family := mkConst ``Carriers.ground
  withLocalDeclD `nonempty (mkApp (mkConst ``CarriersNonempty) family) fun nonempty => do
    withLocalInstances [← nonempty.fvarId!.getDecl] do
      let type := mkApp (mkConst ``NTy.vector)
        (mkApp2 (mkConst ``NTy.int) (mkNatLit 8) (mkConst ``Bool.false))
      let raw := mkApp2 (mkConst ``NTy.plainOf) (mkApp (mkConst ``Carriers.toPlain) family) type
      let canonicalType := mkApp2 (mkConst ``NTy.carrier) family type
      let keys := Lean.mkLambda `key .default (mkConst ``Int) (.bvar 0)
      let oldIdentity := Lean.mkLambda `value .default raw (.bvar 0)
      let identity := Lean.mkLambda `value .default canonicalType (.bvar 0)
      let pair ← mkAppM ``Prod #[mkConst ``Int, raw]
      withLocalDeclD `entries (← mkAppM ``SpecEntries #[pair]) fun entries => do
        let expected := mkAppN (mkConst ``SpecMap.ofEntries)
          #[mkConst ``Int, canonicalType, mkConst ``Int, canonicalType, keys, identity, entries]
        let built ← NativeMaps.typedMapOfEntries keys oldIdentity entries
        unless built == expected do
          throwError "the map constructor retained a plain raw type or identity domain"
        let oldMap := mkAppN (mkConst ``SpecMap.ofEntries)
          #[mkConst ``Int, raw, mkConst ``Int, raw, keys, oldIdentity, entries]
        unless (← canonicalFamilyResult oldMap).expr == expected do
          throwError "an already-materialized identity map was not canonicalized"
        let row := mkApp2 (mkConst ``NRow.cons) type (mkConst ``NRow.nil)
        let typeArgs ← whnf (mkConst ``TypeArgs)
        let arguments := mkApp4 (mkConst ``Subtype.mk [Level.zero]) (mkConst ``NRow)
          (typeArgs.getArg! 1) row (← mkEqRefl (mkConst ``Bool.true))
        let induced := mkApp3 (mkConst ``Carriers.instantiate) arguments family nonempty
        let parameter := mkApp (mkConst ``NTy.param) (mkNatLit 0)
        let plainParameter := mkApp2 (mkConst ``NTy.plainOf)
          (mkApp (mkConst ``Carriers.toPlain) induced) parameter
        let typedParameter := mkApp2 (mkConst ``NTy.readCarrier) induced parameter
        let inhabited := mkApp3 (mkConst ``CarriersNonempty.instantiate) arguments family nonempty
        let held ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
          (← mkEq (mkApp (mkConst ``NTy.inhabitable) parameter) (mkConst ``Bool.true))
        let genericReader := mkAppN (mkConst ``NTy.readPlain) #[induced, inhabited, parameter, held]
        let genericMap := mkAppN (mkConst ``SpecMap.ofEntries)
          #[mkConst ``Int, plainParameter, mkConst ``Int, typedParameter, keys, genericReader, entries]
        unless (← canonicalFamilyResult genericMap).expr == expected do
          throwError "a generic reader's full map AST differs from the concrete map"

-- A certificate-bearing specialization also rebuilds the inhabitability
-- proof at the concrete descriptor, retaining the total reader itself.
open Lean Meta Elab Command in
run_cmd liftTermElabM do
  let family := mkConst ``Carriers.ground
  withLocalDeclD `nonempty (mkApp (mkConst ``CarriersNonempty) family) fun nonempty => do
    withLocalInstances [← nonempty.fvarId!.getDecl] do
      for constant in #[``scalarCertificate, ``nestedCertificate] do
        let type ← withDefault (whnf (mkConst constant))
        let raw := mkApp2 (mkConst ``NTy.plain) family type
        let held ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
          (← mkEq (mkApp (mkConst ``NTy.inhabitable) type) (mkConst ``Bool.true))
        let reader := mkAppN (mkConst ``NTy.readPlain) #[family, nonempty, type, held]
        let keys := Lean.mkLambda `key .default (mkConst ``Int) (.bvar 0)
        let pair ← mkAppM ``Prod #[mkConst ``Int, raw]
        withLocalDeclD `entries (← mkAppM ``SpecEntries #[pair]) fun entries => do
          let concrete ← NativeMaps.typedMapOfEntries keys reader entries
          let expected := (← canonicalFamilyResult concrete).expr
          let row := mkApp2 (mkConst ``NRow.cons) type (mkConst ``NRow.nil)
          let typeArgs ← whnf (mkConst ``TypeArgs)
          let arguments := mkApp4 (mkConst ``Subtype.mk [Level.zero]) (mkConst ``NRow)
            (typeArgs.getArg! 1) row (← mkEqRefl (mkConst ``Bool.true))
          let induced := mkApp3 (mkConst ``Carriers.instantiate) arguments family nonempty
          let parameter := mkApp (mkConst ``NTy.param) (mkNatLit 0)
          let genericHeld ← mkExpectedTypeHint (← mkEqRefl (mkConst ``Bool.true))
            (← mkEq (mkApp (mkConst ``NTy.inhabitable) parameter) (mkConst ``Bool.true))
          let inhabited := mkApp3 (mkConst ``CarriersNonempty.instantiate) arguments family nonempty
          let genericReader := mkAppN (mkConst ``NTy.readPlain)
            #[induced, inhabited, parameter, genericHeld]
          let generic ← NativeMaps.typedMapOfEntries keys genericReader entries
          let specialized := (← canonicalFamilyResult generic).expr
          unless specialized == expected do
            throwError "a certificate-bearing generic map retains a parameter's representation"
          unless specialized.getArg! 5 == reader do
            throwError "the total reader lost its canonical concrete proof annotation"

private def IntegerAlias : Type := Int
private def EntriesAlias : Type := SpecEntries (Int × Int)

-- Projected table fields are reconstructed from their actual pair, so an
-- instantiated owner's old implicit getter types cannot create another map.
open Lean Meta Elab Command in
run_cmd liftTermElabM do
  let int := mkConst ``Int
  let entries ← mkAppM ``SpecEntries #[← mkAppM ``Prod #[int, int]]
  let inner ← mkAppM ``Prod #[int, entries]
  let root ← mkAppM ``Prod #[inner, mkConst ``Bool]
  withLocalDeclD `owner root fun owner => do
    let oldInner ← mkAppM ``Prod #[mkConst ``IntegerAlias, mkConst ``EntriesAlias]
    let projected := mkApp3 (mkConst ``Prod.fst [0, 0]) oldInner (mkConst ``Bool) owner
    let rawProjection := mkApp3 (mkConst ``Prod.snd [0, 0]) (mkConst ``IntegerAlias)
      (mkConst ``EntriesAlias) projected
    let canonicalInner ← mkAppM ``Prod.fst #[owner]
    let canonical ← mkAppM ``Prod.snd #[canonicalInner]
    let identity := Lean.mkLambda `value .default int (.bvar 0)
    let expected ← NativeMaps.typedMapOfEntries identity identity canonical
    let actual ← NativeMaps.typedMapOfEntries identity identity rawProjection
    unless actual == expected do
      throwError "a table getter retained an instantiated owner's old implicit field types"
    let oldMap := mkAppN (mkConst ``SpecMap.ofEntries)
      #[int, int, int, int, identity, identity, rawProjection]
    unless (← canonicalFamilyResult oldMap).expr == expected do
      throwError "a materialized map's table field was not reconstructed"

-- Nonidentity plain views keep one producer spelling for certified and
-- already-plain arguments. An invalid plain value keeps the total decoder;
-- this boundary never supplies a data-invariant certificate of its own.
open Lean Meta Elab Command in
run_cmd liftTermElabM do
  let family := mkConst ``Carriers.ground
  withLocalDeclD `nonempty (mkApp (mkConst ``CarriersNonempty) family) fun nonempty => do
    withLocalInstances [← nonempty.fvarId!.getDecl] do
      for constant in #[``scalarCertificate, ``nestedCertificate] do
        let type ← withDefault (whnf (mkConst constant))
        let some view ← NativeMaps.typedView? family type true
          | throwError "missing certificate-bearing plain view"
        withLocalDeclD `native (mkApp2 (mkConst ``NTy.carrier) family view.type) fun certified => do
          let plain := mkApp3 (mkConst ``NTy.forget) family view.type certified
          let expected := (mkApp view.read plain).headBeta
          let actual ← VerexSL.Contract.TypedTable.typedOfNative view certified
          unless actual == expected do
            throwError "a certified argument bypassed its plain view's canonical reader"
        withLocalDeclD `plain (mkApp2 (mkConst ``NTy.plain) family view.type) fun plain => do
          let actual ← VerexSL.Contract.TypedTable.typedOfNative view plain
          unless actual == (mkApp view.read plain).headBeta do
            throwError "an uncertified plain argument lost its total reader"

-- A table's actual certificate relates its certified key reading to the
-- total plain reader. No inverse or validity claim is made for an arbitrary
-- uncertified table.
example [Θ : Carriers] [CarriersNonempty Θ] (source : StructHandle)
    (entries : @NTy.plain Θ (.table (.struct source .nil (.cons (.int 8 false) .nil))
      (.int 64 false)))
    (holds : NTy.deepHolds (.table (.struct source .nil (.cons (.int 8 false) .nil))
      (.int 64 false)) entries) :
    SpecMap.ofEntries (NTy.read (.struct source .nil (.cons (.int 8 false) .nil))) SpecInt.val
        (NTy.ofDeepHolds (.table (.struct source .nil (.cons (.int 8 false) .nil))
          (.int 64 false)) entries holds) =
      SpecMap.ofEntries (NTy.readPlain (.struct source .nil (.cons (.int 8 false) .nil)) rfl)
        SpecInt.val entries := by
  simp only [vir_denote_norm]

-- Runtime-family inhabitants keep the same projection spelling a typed
-- view emits. An unrelated inhabitant choice is never replaced merely
-- because it inhabits the same family.
open Lean Meta Elab Command in
run_cmd liftTermElabM do
  withLocalDeclD `unit (mkConst ``Validation.ValidatedUnit) fun unit => do
    withLocalDeclD `functions (mkApp (mkConst ``FunctionCarriers) unit) fun functions => do
      let runtime := mkApp2 (mkConst ``Carriers.runtime) unit functions
      withLocalDeclD `supplied (mkApp (mkConst ``CarriersNonempty) runtime) fun supplied => do
        let frame := mkApp3 (mkConst ``Skolems.runtime) unit functions supplied
        let family := mkApp3 (mkConst ``Skolems.toCarriers) unit functions frame
        let expected := mkApp3 (mkConst ``Skolems.carriersNonempty) unit functions frame
        unless (← NativeMaps.canonicalInhabitants family supplied) == expected do
          throwError "the runtime family's reader lost its canonical inhabitants projection"
        withLocalDeclD `other (mkApp (mkConst ``CarriersNonempty) family) fun other => do
          unless (← NativeMaps.canonicalInhabitants family other) == other do
            throwError "canonicalization replaced an independent inhabitants choice"
