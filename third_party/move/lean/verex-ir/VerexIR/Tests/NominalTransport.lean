-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.FamilyTransport
import VerexIR.Validation.Check
open Lean Meta Elab Command
open VerexIR VerexIR.Import VerexIR.Validation VerexIR.Proofs.Denote

-- Ordinary, phantom, and nested generic declarations share the same substitution
-- contract while retaining their declaration and unit identities.
private def fixture : RawUnit where
  tables := {
    files := #[{ name := "nominal.rs" }]
    locations := #[{ primary := some { file := ⟨0⟩, startByte := 0, endByte := 1 } }]
    types := #[.unit, .bool, .typeParameter 0,
      .nominal ⟨0⟩ #[.typeArg { typeId := ⟨2⟩, loc := ⟨0⟩ }]]
    namespaces := #[{ segments := #["test", "Nominal"] }]
    names := #[{ namespaceId := ⟨0⟩, name := "Box" },
      { namespaceId := ⟨0⟩, name := "Phantom" },
      { namespaceId := ⟨0⟩, name := "Wrapper" },
      { namespaceId := ⟨0⟩, name := "value" }] }
  profiles := #[{ profile := .rust, name := "rust-test" }]
  namespaces := #[{
    loc := ⟨0⟩, identity := ⟨0⟩, profile := some .rust,
    structs := #[
      { loc := ⟨0⟩, name := ⟨0⟩,
        generics := #[{ name := "T", kind := .typeArg, loc := ⟨0⟩ }],
        fields := #[{ loc := ⟨0⟩, name := ⟨3⟩, type := { typeId := ⟨2⟩, loc := ⟨0⟩ } }] },
      { loc := ⟨0⟩, name := ⟨1⟩,
        generics := #[{ name := "T", kind := .typeArg, loc := ⟨0⟩ }] },
      { loc := ⟨0⟩, name := ⟨2⟩,
        generics := #[{ name := "T", kind := .typeArg, loc := ⟨0⟩ }],
        fields := #[{ loc := ⟨0⟩, name := ⟨3⟩, type := { typeId := ⟨3⟩, loc := ⟨0⟩ } }] }] }]
run_meta do
  let .ok unit := validate #[{ profile := .rust, name := "rust-test" }] fixture
    | throwError "fixture validation failed: {repr (validate #[{ profile := .rust, name := "rust-test" }] fixture)}"
  registerNominalUnit `VerexIR.Tests.NominalTransport.unit unit
  registerNominalUnit `VerexIR.Tests.NominalTransport.other unit
  let parameter := mkApp (mkConst ``NTy.param) (mkNatLit 0)
  let boolean := mkConst ``NTy.bool
  let row := mkApp2 (mkConst ``NRow.cons) boolean (mkConst ``NRow.nil)
  for declarationIndex in [0, 1, 2] do
    let some schema := structNTy unit ⟨⟨0⟩, declarationIndex⟩ | throwError "missing schema"
    let generic ← withNominalUnit `VerexIR.Tests.NominalTransport.unit (quoteNativeType schema)
    let instantiated ← withNominalUnit `VerexIR.Tests.NominalTransport.unit
      (quoteNativeType (schema.subst (.cons .bool .nil)))
    let other ← withNominalUnit `VerexIR.Tests.NominalTransport.other (quoteNativeType schema)
    let env ← getEnv
    unless FamilyTransport.spelledNTy generic (some env) do throwError "descriptor not spelled"
    unless FamilyTransport.substituteNTy row generic (some env) == some instantiated do
      throwError "nominal substitution did not preserve descriptor"
    unless generic.getAppFn != other.getAppFn do throwError "different units share descriptor"
    unless instantiated.getArg! 0 == boolean && generic.getArg! 0 == parameter do
      throwError "explicit arguments lost, including phantom"
    unless ← isDefEq (mkApp2 (mkConst ``NTy.subst) row generic) instantiated do
      throwError "substitution is not kernel definitionally equal"
    unless (FamilyTransport.substituteNTy row generic.getAppFn (some env)).isNone do
      throwError "partial application accepted"
    unless (FamilyTransport.substituteNTy row (mkApp generic boolean) (some env)).isNone do
      throwError "overapplication accepted"
    let expression := mkApp2 (mkConst ``NTy.subst) row generic
    unless (← FamilyTransport.respellInducedFamily expression) == instantiated do
      throwError "induced substitution did not preserve nominal descriptor"
    let nil := mkConst ``NRow.nil
    let genericRow := mkApp2 (mkConst ``NRow.cons) generic nil
    let instantiatedRow := mkApp2 (mkConst ``NRow.cons) instantiated nil
    unless FamilyTransport.substituteNRow row genericRow (some env) == some instantiatedRow do
      throwError "row lost its descriptor"
    let index := mkApp2 (mkConst ``Var.here) nil generic
    let instantiatedIndex := mkApp2 (mkConst ``Var.here) nil instantiated
    unless FamilyTransport.respellIndex? env row index == some instantiatedIndex do
      throwError "index lost its descriptor"
    let typeArgs ← whnf (mkConst ``TypeArgs)
    let args := mkApp4 (mkConst ``Subtype.mk [Level.one]) (mkConst ``NRow)
      (typeArgs.getArg! 1) row (← mkEqRefl (mkConst ``Bool.true))
    withLocalDeclD `outer (mkConst ``Carriers) fun outer => do
      withLocalDeclD `nonempty (mkApp (mkConst ``CarriersNonempty) outer) fun nonempty => do
        let family := mkApp3 (mkConst ``Carriers.instantiate) args outer nonempty
        let carried := mkApp2 (mkConst ``NTy.carrier) family generic
        let expected := mkApp2 (mkConst ``NTy.carrier) outer instantiated
        unless (← FamilyTransport.respellInducedFamily carried) == expected do
          throwError "carrier lost its descriptor"
        unless ← isDefEq carried expected do throwError "carriers differ"
        let encoding := mkApp2 (mkConst ``NTy.encode) family generic
        let expectedEncoding := mkApp2 (mkConst ``NTy.encode) outer instantiated
        unless (← FamilyTransport.respellOperation? encoding) == some expectedEncoding do
          throwError "operation lost its descriptor"

        -- HEnv is a product of optional carriers. Its canonical spelling
        -- retains registered nominal descriptors, including phantom and
        -- nested arguments, while substituting every heterogeneous slot.
        let cons := mkApp2 (mkConst ``NRow.cons)
        let tuple := mkApp (mkConst ``NTy.tuple) (cons parameter (cons boolean nil))
        let fields := cons parameter (cons boolean (cons generic
          (cons (mkApp (mkConst ``NTy.ghost) generic) (cons tuple nil))))
        let concreteTuple := mkApp (mkConst ``NTy.tuple) (cons boolean (cons boolean nil))
        let concreteFields := cons boolean (cons boolean (cons instantiated
          (cons (mkApp (mkConst ``NTy.ghost) instantiated) (cons concreteTuple nil))))
        for (fields, concreteFields) in [(nil, nil), (cons boolean nil, cons boolean nil),
            (fields, concreteFields)] do
          let source := mkApp2 (mkConst ``HEnv) family fields
          let expected := mkApp2 (mkConst ``HEnv) outer concreteFields
          let .done actual ← FamilyTransport.respellInducedNode source
              (fun _ _ => throwError "structural HEnv spelling requested conversion")
            | throwError "environment was not structurally respelled"
          unless actual == expected do throwError "environment lost its canonical row spelling"
          unless (← FamilyTransport.respellInducedFamily source) == expected do
            throwError "environment traversal changed its canonical spelling"
          unless (← FamilyTransport.respellInducedFamily expected) == expected do
            throwError "environment canonicalization is not idempotent"
          unless ← isDefEq source expected do throwError "environment carriers do not convert"
          -- A reflexivity proof at the canonical type must also certify the
          -- original type equality in the kernel, under an arbitrary family.
          let theoremType ← mkForallFVars #[outer, nonempty] (← mkEq source expected)
          let proof ← mkLambdaFVars #[outer, nonempty] (← mkEqRefl expected)
          let suffix := if fields == nil then "nil" else if fields == cons boolean nil
            then "bool" else "heterogeneous"
          addDecl (.thmDecl {
            name := Name.str (Name.num `VerexIR.Tests.NominalTransport.henv declarationIndex) suffix,
            levelParams := [], type := theoremType, value := proof })
        withLocalDeclD `opaqueRow (mkConst ``NRow) fun opaqueRow => do
          for fields in [opaqueRow, cons generic opaqueRow] do
            let source := mkApp2 (mkConst ``HEnv) family fields
            unless (← FamilyTransport.respellInducedFamily source) == source do
              throwError "opaque environment row was partially substituted"
    let unknown := mkConst `UnregisteredDescriptor
    unless (FamilyTransport.substituteNTy row unknown (some env)).isNone do
      throwError "arbitrary constant accepted"
