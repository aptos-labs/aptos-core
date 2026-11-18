-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.NativeMaps
import VerexIR.Proofs.Denote.GivenOwn
import VerexIR.Proofs.Denote.Solve.Normal
import VerexSL.Contract.Values

open VerexIR VerexIR.Proofs.Denote

private def byteVector : NTy := .vector (.int 8 false)
private def byteVectorArgs : TypeArgs := ⟨.cons byteVector .nil, rfl⟩
private def nestedCertificate : NTy :=
  .vector (.struct ⟨⟨0⟩, 0⟩ .nil (.cons (.int 8 false) .nil))
private def scalarCertificate : NTy :=
  .struct ⟨⟨0⟩, 0⟩ .nil (.cons (.int 8 false) .nil)
private def nestedCertificateArgs : TypeArgs := ⟨.cons nestedCertificate .nil, rfl⟩
private def scalarCertificateArgs : TypeArgs := ⟨.cons scalarCertificate .nil, rfl⟩

-- Negative control first: an unresolved parameter at an unknown `Carriers` family
-- retains its family `NTy.read` view (`identity = false`) rather than collapsing to identity,
-- and an uninhabited enum declines `typedView?` (`none`).
open Lean Meta Elab Command in
run_cmd liftTermElabM do
  let parameter := mkApp (mkConst ``NTy.param) (mkNatLit 0)
  withLocalDeclD `unknown (mkConst ``Sorts) fun unknown => do
    let some unresolved ← NativeMaps.typedView? unknown parameter
      | throwError "unresolved parameter lost its typed view"
    if unresolved.identity then
      throwError "an unresolved parameter at an unknown family must retain NTy.read"
    unless unresolved.read == mkApp2 (mkConst ``NTy.read) unknown parameter do
      throwError "unresolved parameter reader changed: {unresolved.read}"
  let emptyEnum : NTy := .enum ⟨⟨0⟩, 2⟩ .nil [] .nil (by decide)
  let quotedEmpty ← withSyntheticNativeTypes (VerexSL.Contract.quoteNTy emptyEnum)
  if (← NativeMaps.typedView? (mkConst ``Sorts.ground) quotedEmpty).isSome then
    throwError "an uninhabited enum acquired a typed map view"

-- A frame's type arguments are read by the named field of their subtype, the
-- form clause and write-back producers emit: the view of a parameter at a
-- symbolic induced frame holds no primitive projection, and its terms, the
-- resolution's inhabitability proof among them, type-check as they stand
-- (`resolvedType`).
open Lean Meta Elab Command in
run_cmd liftTermElabM do
  let parameter := mkApp (mkConst ``NTy.param) (mkNatLit 0)
  withLocalDeclD `arguments (mkConst ``TypeArgs) fun arguments =>
    withLocalDeclD `outer (mkConst ``Sorts) fun outer => do
      let induced := mkApp2 (mkConst ``Sorts.instantiate) arguments outer
      let some view ← NativeMaps.typedView? induced parameter
        | throwError "a symbolic frame's parameter lost its typed view"
      for (name, term) in #[("type", view.type), ("carrier", view.carrier),
          ("reader", view.read), ("inhabited", view.inhabited), ("decEq", view.decEq),
          ("unread", view.unread)] do
        if (term.find? fun e => match e with
            | .proj owner _ _ => owner == ``Subtype
            | _ => false).isSome then
          throwError "the {name} of a resolved parameter holds a primitive projection \
            of the frame's type arguments: {term}"
        check term

-- A concrete non-integer reader is the identity by construction over the single carrier.
example [Θ : Carriers] :
    NTy.read byteVector = (fun value : @NTy.carrier (Θ).toSorts byteVector => value) := rfl

-- Instantiating a parameter reaches that same reader.
example [Θ : Carriers] :
    @NTy.read ((Carriers.instantiate byteVectorArgs Θ)).toSorts (.param 0) =
      (fun value : @NTy.carrier (Θ).toSorts byteVector => value) := rfl

-- The map receives the concrete reader directly, even while unapplied.
example [Θ : Carriers]
    (entries : SpecEntries (SpecInt (.bits 8) false × @NTy.carrier (Θ).toSorts byteVector)) :
    SpecMap.ofEntries SpecInt.val (NTy.read byteVector) entries =
      SpecMap.ofEntries SpecInt.val (fun value => value) entries := rfl

-- Production call contracts use the projected carriers of an instantiated
-- `Skolems` frame. This path retains the same reader directly.
example {unit : Validation.ValidatedUnit} [FunctionCarriers unit] (frame : Skolems unit) :
    @NTy.read ((Skolems.instantiate byteVectorArgs frame).toCarriers).toSorts (.param 0) =
      (fun value : @NTy.carrier (frame.toCarriers).toSorts byteVector => value) := rfl

example (frame : GivenFrame) (index : Nat) :
    @NTy.read (frame.toCarriers).toSorts (.param index) =
      (fun value : frame.operations.carrier index => value) := rfl

-- Concrete views use the same typed sort, reader, inverse, and `plainFallback`
-- `Inhabited` instance as a generic parameter instantiated at that type,
-- including nominal structs and vectors of nominal structs.
open Lean Meta Elab Command in
run_cmd liftTermElabM do
  let family := mkConst ``Sorts.ground
  for (type, args) in #[(mkConst ``byteVector, mkConst ``byteVectorArgs),
      (mkConst ``nestedCertificate, mkConst ``nestedCertificateArgs),
      (mkConst ``scalarCertificate, mkConst ``scalarCertificateArgs)] do
    let some concrete ← NativeMaps.typedView? family type
      | throwError "missing concrete typed view"
    let induced := mkApp2 (mkConst ``Sorts.instantiate) args family
    let some generic ← NativeMaps.typedView? induced (mkApp (mkConst ``NTy.param) (mkNatLit 0))
      | throwError "missing instantiated typed view"
    for (name, left, right) in #[("carrier", concrete.carrier, generic.carrier),
        ("reader", concrete.read, generic.read),
        ("unread", concrete.unread, generic.unread),
        ("inhabited", concrete.inhabited, generic.inhabited)] do
      check left
      check right
      unless ← withNewMCtxDepth (withTransparency .default (isDefEq left right)) do
        throwError "concrete and generic {name} disagree"
  withLocalDeclD `given (mkConst ``GivenFrame) fun given => do
    let givenCarriers := mkApp (mkConst ``Carriers.toSorts) (mkApp (mkConst ``GivenFrame.toCarriers) given)
    let some view ← NativeMaps.typedView? givenCarriers (mkApp (mkConst ``NTy.param) (mkNatLit 0))
      | throwError "missing Given parameter typed view"
    unless view.identity do
      throwError "Given parameter typed view lost its identity representation"

-- Emitted generic maps at an instantiated family canonicalize (`canonicalFamilyResult`)
-- to the exact concrete map AST for both free vectors and nominal-bearing types.
open Lean Meta Elab Command in
run_cmd liftTermElabM do
  let family := mkConst ``Sorts.ground
  for constant in #[``byteVector, ``scalarCertificate, ``nestedCertificate] do
    let type ← withDefault (whnf (mkConst constant))
    let canonicalType := mkApp2 (mkConst ``NTy.carrier) family type
    let keys := Lean.mkLambda `key .default (mkConst ``Int) (.bvar 0)
    let identity := Lean.mkLambda `value .default canonicalType (.bvar 0)
    let pair ← mkAppM ``Prod #[mkConst ``Int, canonicalType]
    withLocalDeclD `entries (← mkAppM ``SpecEntries #[pair]) fun entries => do
      let expected := mkAppN (mkConst ``SpecMap.ofEntries)
        #[mkConst ``Int, canonicalType, mkConst ``Int, canonicalType, keys, identity, entries]
      let row := mkApp2 (mkConst ``NRow.cons) type (mkConst ``NRow.nil)
      let typeArgs ← whnf (mkConst ``TypeArgs)
      let arguments := mkApp4 (mkConst ``Subtype.mk [Level.succ Level.zero]) (mkConst ``NRow)
        (typeArgs.getArg! 1) row (← mkEqRefl (mkConst ``Bool.true))
      let induced := mkApp2 (mkConst ``Sorts.instantiate) arguments family
      let parameter := mkApp (mkConst ``NTy.param) (mkNatLit 0)
      let paramCarrier := mkApp2 (mkConst ``NTy.carrier) induced parameter
      let typedParameter := mkApp2 (mkConst ``NTy.readCarrier) induced parameter
      let genericReader := mkApp2 (mkConst ``NTy.read) induced parameter
      let genericMap := mkAppN (mkConst ``SpecMap.ofEntries)
        #[mkConst ``Int, paramCarrier, mkConst ``Int, typedParameter, keys, genericReader, entries]
      check expected
      check genericMap
      unless (← canonicalFamilyResult genericMap).expr == expected do
        throwError "a generic reader's full map AST differs from the concrete map for {constant}"

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

-- A variant's field selection (`Choices.select?`) at an induced family is
-- the outer family's at the substituted rows, apart from the value it is
-- applied to: the selection of another field does not convert, the
-- respelling of the operation alone does, and a clause applying it to a
-- value that is no constructor is stated at the outer family.
open Lean Meta Elab Command in
run_cmd liftTermElabM do
  let closed (term : TSyntax `term) : TermElabM Lean.Expr := do
    let value ← Term.elabTerm term none
    Term.synthesizeSyntheticMVarsNoPostponing
    instantiateMVars value
  let genericFirst ← closed (← `((Choices.cons (Which.there (Which.there Which.here)) Var.here .nil :
    Choices ["None", "Direct", "Split"]
      (.cons .nil (.cons (.cons (.param 0) .nil)
        (.cons (.cons (.int 64 false) (.cons (.param 0) .nil)) .nil))) (.int 64 false))))
  let booleanFirst ← closed (← `((Choices.cons (Which.there (Which.there Which.here)) Var.here .nil :
    Choices ["None", "Direct", "Split"]
      (.cons .nil (.cons (.cons .bool .nil)
        (.cons (.cons (.int 64 false) (.cons .bool .nil)) .nil))) (.int 64 false))))
  let booleanSecond ← closed (← `((Choices.cons (Which.there (Which.there Which.here))
      (Var.there Var.here) .nil :
    Choices ["None", "Direct", "Split"]
      (.cons .nil (.cons (.cons .bool .nil)
        (.cons (.cons (.int 64 false) (.cons .bool .nil)) .nil))) .bool)))
  let arguments ← closed (← `((⟨.cons .bool .nil, rfl⟩ : TypeArgs)))
  let select (family chosen : Lean.Expr) : MetaM Lean.Expr := do
    let type ← whnfR (← inferType chosen)
    return mkAppN (mkConst ``Choices.select?) (#[family] ++ type.getAppArgs ++ #[chosen])
  withLocalDeclD `outer (mkConst ``Carriers) fun outer => do
    let induced := mkApp2 (mkConst ``Carriers.instantiate) arguments outer
    let first ← select induced genericFirst
    let firstOuter ← select outer booleanFirst
    let secondOuter ← select outer booleanSecond
    if ← withNewMCtxDepth (withDefault (isDefEq first secondOuter)) then
      throwError "the selections of two fields convert"
    unless (← FamilyTransport.respellOperation? first) == some firstOuter do
      throwError "a variant's field selection kept its induced family"
    let carrier ← withDefault do
      let .forallE _ carrier _ _ ← whnf (← inferType first)
        | throwError "a selection is not a function of its value"
      pure carrier
    withLocalDeclD `value carrier fun value => do
      let clause ← mkEq (mkApp first value) (mkApp first value)
      let canonical := (← canonicalFamilyResult clause).expr
      unless (canonical.find? fun term => term.isAppOf ``Choices.select? &&
          term.getAppArgs[0]? == some induced).isNone do
        throwError "a clause selecting a field of an unsplit value kept the induced family"
      unless (canonical.find? fun term => term == mkApp firstOuter value).isSome do
        throwError "the clause's selection is not the outer family's at the substituted rows"
