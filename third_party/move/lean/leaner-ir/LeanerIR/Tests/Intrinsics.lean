-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Import.Json
import LeanerIR.Validation.Check

namespace LeanerIR.Tests.Intrinsics

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def config : ProfileConfig := { profile := .rust, name := "rust-test" }
private def schema : ProfileSchema := { profile := .rust, name := "rust-test" }

private def ownRef (name : Nat) : QualifiedRef :=
  { namespaceId := ⟨0⟩, name := ⟨name⟩ }

private def fixture : RawUnit where
  tables := {
    files := #[{ name := "intrinsics.lir" }]
    locations := (Array.range 8).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .leanerSource, location := ⟨0⟩ }]
    alignments := #[{
      source := ⟨0⟩, trust := .checked, description := "intrinsic fixture" }]
    types := #[.unit]
    namespaces := #[
      { segments := #["test", "Intrinsics"] },
      { segments := #["test", "External"] }]
    names := #[
      { namespaceId := ⟨0⟩, name := "Carrier" },
      { namespaceId := ⟨0⟩, name := "create" },
      { namespaceId := ⟨0⟩, name := "spec_create" },
      { namespaceId := ⟨1⟩, name := "external" },
      { namespaceId := ⟨0⟩, name := "OtherCarrier" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    structs := #[
      { loc := ⟨1⟩, name := ⟨0⟩ },
      { loc := ⟨2⟩, name := ⟨4⟩ }]
    functions := #[{
      loc := ⟨1⟩
      name := ⟨1⟩
      profile := .rust
      signature := {}
      body := .absent
      origin := ⟨0⟩
      alignment := ⟨0⟩ }]
    specFunctions := #[{
      loc := ⟨2⟩
      name := ⟨2⟩
      profile := .rust
      signature := {}
      origin := ⟨0⟩ }]
    intrinsics := #[{
      loc := ⟨1⟩
      model := "map"
      owner := ⟨0⟩
      profile := .rust
      executableBindings := #[{ loc := ⟨3⟩, role := "new", target := ownRef 1 }]
      specBindings := #[{ loc := ⟨4⟩, role := "spec_new", target := ownRef 2 }] }] }]

private def diagnostics (unit : RawUnit) : Array Diagnostic :=
  match validate #[schema] unit with
  | .ok _ => #[]
  | .error diagnostics => diagnostics

private def hasCode (unit : RawUnit) (code : String) : Bool :=
  (diagnostics unit).any (fun diagnostic => diagnostic.code == code)

#guard (validate #[schema] fixture).isOk

#guard match decodeJson (encodeJson fixture) with
  | .ok decoded => decoded == fixture && (validate #[schema] decoded).isOk
  | .error _ => false

private def badOwner : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{ ns with intrinsics := #[{
      ns.intrinsics[0]! with owner := ⟨1⟩ }] }] }

#guard hasCode badOwner "LIR-INTRINSIC-OWNER"

private def badTargetKinds : RawUnit :=
  let ns := fixture.namespaces[0]!
  let intrinsic := ns.intrinsics[0]!
  { fixture with namespaces := #[{ ns with intrinsics := #[{
      intrinsic with
      executableBindings := #[{ loc := ⟨3⟩, role := "new", target := ownRef 2 }]
      specBindings := #[{ loc := ⟨4⟩, role := "spec_new", target := ownRef 1 }] }] }] }

#guard hasCode badTargetKinds "LIR-INTRINSIC-TARGET-KIND"

private def externalTarget : RawUnit :=
  let ns := fixture.namespaces[0]!
  let intrinsic := ns.intrinsics[0]!
  { fixture with namespaces := #[{ ns with intrinsics := #[{
      intrinsic with executableBindings := #[{
        loc := ⟨5⟩
        role := "new"
        target := { namespaceId := ⟨1⟩, name := ⟨3⟩ } }] }] }] }

#guard hasCode externalTarget "LIR-INTRINSIC-TARGET-NAMESPACE"

private def duplicateRole : RawUnit :=
  let ns := fixture.namespaces[0]!
  let intrinsic := ns.intrinsics[0]!
  { fixture with namespaces := #[{ ns with intrinsics := #[{
      intrinsic with executableBindings := intrinsic.executableBindings.push {
        loc := ⟨5⟩, role := "new", target := ownRef 2 } }] }] }

#guard (diagnostics duplicateRole).any fun diagnostic =>
  diagnostic.code == "LIR-INTRINSIC-ROLE-DUPLICATE" &&
    diagnostic.primary == some ⟨5⟩ &&
    diagnostic.related.any (fun related => related.loc == ⟨3⟩)

private def duplicateOwner : RawUnit :=
  let ns := fixture.namespaces[0]!
  let intrinsic := ns.intrinsics[0]!
  { fixture with namespaces := #[{ ns with intrinsics := ns.intrinsics.push {
      intrinsic with loc := ⟨6⟩, executableBindings := #[], specBindings := #[] } }] }

#guard (diagnostics duplicateOwner).any fun diagnostic =>
  diagnostic.code == "LIR-INTRINSIC-OWNER-DUPLICATE" && diagnostic.primary == some ⟨6⟩

private def sharedTarget : RawUnit :=
  let ns := fixture.namespaces[0]!
  let intrinsic := ns.intrinsics[0]!
  { fixture with namespaces := #[{ ns with intrinsics := #[{
      intrinsic with executableBindings := intrinsic.executableBindings.push {
        loc := ⟨7⟩, role := "empty", target := ownRef 1 } }] }] }

#guard (diagnostics sharedTarget).any fun diagnostic =>
  diagnostic.code == "LIR-INTRINSIC-TARGET-SHARED" &&
    diagnostic.primary == some ⟨7⟩ &&
    diagnostic.related.any (fun related => related.loc == ⟨3⟩)

end LeanerIR.Tests.Intrinsics
