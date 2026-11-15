-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove

namespace VerexIR.Move.Tests.Intrinsics

open VerexIR
open VerexIR.Import
open VerexIR.Validation
open VerexIR.Move.Intrinsics

private def ownRef (name : Nat) : QualifiedRef :=
  { namespaceId := ⟨0⟩, name := ⟨name⟩ }

private def typeUse (type : Nat) : TypeUse :=
  { typeId := ⟨type⟩, loc := ⟨1⟩ }

private def mapSignature (parameters : Array Nat) (result : Nat) : Signature :=
  {
    generics := #[
      { name := "K", kind := .typeArg, loc := ⟨1⟩ },
      { name := "V", kind := .typeArg, loc := ⟨1⟩ }]
    parameters := parameters.mapIdx fun index type =>
      { name := s!"p{index}", typeUse := typeUse type }
    results := #[typeUse result] }

private def specFunction (name : Nat) (signature : Signature) : SpecFunctionDecl :=
  let locals := signature.parameters.mapIdx fun index parameter => {
    id := ⟨index⟩
    name := parameter.name
    type := parameter.typeUse
    loc := parameter.typeUse.loc }
  {
    loc := ⟨1⟩
    name := ⟨name⟩
    profile := .move
    signature
    origin := ⟨0⟩
    locals }

private def fixture : RawUnit where
  tables := {
    files := #[{ name := "move-intrinsics.vir" }]
    locations := (Array.range 8).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .moveSource, location := ⟨0⟩ }]
    alignments := #[{
      source := ⟨0⟩, trust := .checked, description := "Move intrinsic fixture" }]
    types := #[
      .unit,
      .typeParameter 0,
      .typeParameter 1,
      .nominal ⟨0⟩ #[
        .typeArg (typeUse 1),
        .typeArg (typeUse 2)],
      .bool]
    namespaces := #[{ segments := #["0x1", "Intrinsics"] }]
    names := #[
      { namespaceId := ⟨0⟩, name := "Carrier" },
      { namespaceId := ⟨0⟩, name := "spec_get" },
      { namespaceId := ⟨0⟩, name := "spec_set" },
      { namespaceId := ⟨0⟩, name := "spec_del" },
      { namespaceId := ⟨0⟩, name := "spec_has_key" },
      { namespaceId := ⟨0⟩, name := "length" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .move
    structs := #[{
      loc := ⟨1⟩
      name := ⟨0⟩
      generics := #[
        { name := "K", kind := .typeArg, loc := ⟨1⟩ },
        { name := "V", kind := .typeArg, loc := ⟨1⟩ }] }]
    functions := #[{
      loc := ⟨6⟩
      name := ⟨5⟩
      profile := .move
      signature := {}
      body := .absent
      origin := ⟨0⟩
      alignment := ⟨0⟩ }]
    specFunctions := #[
      specFunction 1 (mapSignature #[3, 1] 2),
      specFunction 2 (mapSignature #[3, 1, 2] 3),
      specFunction 3 (mapSignature #[3, 1] 3),
      specFunction 4 (mapSignature #[3, 1] 4)]
    intrinsics := #[{
      loc := ⟨1⟩
      model := "map"
      owner := ⟨0⟩
      profile := .move
      specBindings := #[
        { loc := ⟨2⟩, role := "map_spec_get", target := ownRef 1 },
        { loc := ⟨3⟩, role := "map_spec_set", target := ownRef 2 },
        { loc := ⟨4⟩, role := "map_spec_del", target := ownRef 3 },
        { loc := ⟨5⟩, role := "map_spec_has_key", target := ownRef 4 }] }] }]

private def diagnostics (unit : RawUnit) : Array Diagnostic :=
  match validate unit with
  | .ok _ => #[]
  | .error diagnostics => diagnostics

private def hasCode (unit : RawUnit) (code : String) : Bool :=
  (diagnostics unit).any (fun diagnostic => diagnostic.code == code)

#guard registryComplete
#guard roleSchemas.size == 66
#guard roleSchemas.countP (fun role => role.kind == .executable) == 38
#guard roleSchemas.countP (fun role => role.kind == .specification) == 28
#guard signaturePatterns (.specification .abortsNewWithConfig) == #[{
  parameters := #[.num, .num, .bool]
  result := .bool }]
#guard signaturePatterns (.specification .abortsTrim) == #[{
  parameters := #[.owner, .num]
  result := .bool }]
#guard (roleSchema? "map_spec_insertion_key_at").map (·.role) == some (.specification .insertionKeyAt)
#guard (roleSchema? "map_spec_insertion_rank").map (·.role) == some (.specification .insertionRank)
#guard signaturePatterns (.specification .insertionKeyAt) == #[{
  parameters := #[.owner, .num]
  result := .key }]
#guard signaturePatterns (.specification .insertionRank) == #[{
  parameters := #[.owner, .key]
  result := .num }]
#guard (validate fixture).isOk

private def unknownModel : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{ ns with intrinsics := #[{
      ns.intrinsics[0]! with model := "future-model" }] }] }

#guard hasCode unknownModel "VIR-MOVE-INTRINSIC-MODEL"

private def unknownRole : RawUnit :=
  let ns := fixture.namespaces[0]!
  let intrinsic := ns.intrinsics[0]!
  { fixture with namespaces := #[{ ns with intrinsics := #[{
      intrinsic with specBindings := intrinsic.specBindings.set! 1 {
        intrinsic.specBindings[1]! with role := "map_spec_future" } }] }] }

#guard hasCode unknownRole "VIR-MOVE-INTRINSIC-ROLE"

private def wrongRoleKind : RawUnit :=
  let ns := fixture.namespaces[0]!
  let intrinsic := ns.intrinsics[0]!
  { fixture with namespaces := #[{ ns with intrinsics := #[{
      intrinsic with
      executableBindings := #[{
        loc := ⟨6⟩, role := "map_spec_get", target := ownRef 5 }]
      specBindings := intrinsic.specBindings.eraseIdx 0 }] }] }

#guard hasCode wrongRoleKind "VIR-MOVE-INTRINSIC-ROLE-KIND"

private def missingRequiredRole : RawUnit :=
  let ns := fixture.namespaces[0]!
  let intrinsic := ns.intrinsics[0]!
  { fixture with namespaces := #[{ ns with intrinsics := #[{
      intrinsic with specBindings := intrinsic.specBindings.eraseIdx 1 }] }] }

#guard hasCode missingRequiredRole "VIR-MOVE-INTRINSIC-ROLE-REQUIRED"

private def missingDependency : RawUnit :=
  let ns := fixture.namespaces[0]!
  let intrinsic := ns.intrinsics[0]!
  { fixture with namespaces := #[{ ns with intrinsics := #[{
      intrinsic with executableBindings := #[{
        loc := ⟨6⟩, role := "map_len", target := ownRef 5 }] }] }] }

#guard hasCode missingDependency "VIR-MOVE-INTRINSIC-ROLE-DEPENDENCY"

private def badOwnerShape : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{ ns with structs := #[{
      ns.structs[0]! with generics := ns.structs[0]!.generics.eraseIdx 1 }] }] }

#guard hasCode badOwnerShape "VIR-MOVE-INTRINSIC-OWNER-SHAPE"

private def badSignature : RawUnit :=
  let ns := fixture.namespaces[0]!
  let specFunctions := ns.specFunctions.set! 0 (specFunction 1 (mapSignature #[3, 1] 4))
  { fixture with namespaces := #[{ ns with specFunctions }] }

#guard hasCode badSignature "VIR-MOVE-INTRINSIC-SIGNATURE"

-- A configured table is a table the runtime builds.
private def configuredTable : RawUnit :=
  let ns := fixture.namespaces[0]!
  let signature := mapSignature #[5, 5, 4] 3
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types.push (.integer (.bits 16) false) }
    namespaces := #[{ ns with
      functions := #[{ ns.functions[0]! with signature, locals := (specFunction 5 signature).locals }]
      intrinsics := #[{ ns.intrinsics[0]! with executableBindings := #[{
        loc := ⟨6⟩, role := "map_new_with_config", target := ownRef 5 }] }] }] }

#guard match validate configuredTable with
  | .error _ => false
  | .ok unit => (prepareExecution #[semantics] unit) matches .ok _

-- A frontend may recognize a role the shared vocabulary does not name. Its
-- schema accepts the same signature as new_with_config; preparation must
-- still reject it, as the runtime has no step for it.
private def futureTable : RawUnit :=
  let ns := configuredTable.namespaces[0]!
  let intrinsic := ns.intrinsics[0]!
  { configuredTable with namespaces := #[{ ns with intrinsics := #[{
      intrinsic with executableBindings := #[{
        intrinsic.executableBindings[0]! with role := "map_future_role" }] }] }] }

private def futureFrontendSchema : ProfileSchema := {
  schema with checkIntrinsic := fun unit ns intrinsic =>
    schema.checkIntrinsic unit ns { intrinsic with
      executableBindings := intrinsic.executableBindings.map fun binding =>
        if binding.role == "map_future_role" then { binding with role := "map_new_with_config" }
        else binding } }

#guard match Validation.validate #[futureFrontendSchema] futureTable with
  | .error _ => false
  | .ok unit => match prepareExecution #[semantics] unit with
      | .ok _ => false
      | .error diagnostics => diagnostics.any fun diagnostic =>
          diagnostic.code == "VIR-EXEC-UNSUPPORTED" &&
            diagnostic.message.endsWith "`map_future_role` is not an executable map role"

end VerexIR.Move.Tests.Intrinsics
