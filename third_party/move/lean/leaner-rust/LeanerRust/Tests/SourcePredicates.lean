-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerRust.Source

namespace LeanerIR.Rust.Tests

open LeanerIR Import

private def predicateFixture (unsupported : Bool) : IO RawUnit := do
  let path : System.FilePath := "rust-exporter/tests/raw-unit/basic.exp.json"
  let raw ← match Import.decodeJson (← IO.FS.readFile path) with
    | .ok unit => pure unit
    | .error message => throw <| IO.userError s!"predicate fixture does not decode: {message}"
  let function := raw.namespaces[0]!.functions[0]!
  let generics : Array GenericBinder := #[
    { name := "a", kind := .lifetime, loc := function.loc,
      predicates := if unsupported then #[.constEq .unit .unit]
        else #[.lifetimeOutlives ⟨0⟩ ⟨1⟩] },
    { name := "b", kind := .lifetime, loc := function.loc },
    { name := "T", kind := .typeArg, abilities := #[.copy], loc := function.loc }]
  let function := { function with
    signature := { function.signature with generics } }
  pure {
    raw with
    tables := { raw.tables with lifetimes := #[
      { kind := .parameter 0, loc := function.loc },
      { kind := .parameter 1, loc := function.loc }] }
    namespaces := raw.namespaces.set! 0 {
      raw.namespaces[0]! with
      functions := raw.namespaces[0]!.functions.set! 0 function } }

private def checkFunctionPredicateRendering : IO Unit := do
  let unit ← match Rust.validate (← predicateFixture false) with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"lifetime predicate fixture does not validate: {repr diagnostics}"
  let rendered ← match Rust.Source.render unit with
    | .ok rendered => pure rendered
    | .error message => throw <| IO.userError s!"lifetime predicate does not render: {message}"
  unless rendered.contains "pub fn answer<'a, 'b, T>()" &&
      rendered.contains "where 'a: 'b, T: Copy" do
    throw <| IO.userError s!"lifetime predicate was not reconstructed:\n{rendered}"
  let unsupported ← match Rust.validate (← predicateFixture true) with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"unsupported predicate fixture does not validate: {repr diagnostics}"
  match Rust.Source.render unsupported with
  | .error _ => pure ()
  | .ok _ => throw <| IO.userError "unsupported function predicate was silently omitted"

private def nominalPredicateFixture (unsupported : Bool) : IO RawUnit := do
  let path : System.FilePath := "rust-exporter/tests/raw-unit/generic_adt.exp.json"
  let raw ← match Import.decodeJson (← IO.FS.readFile path) with
    | .ok unit => pure unit
    | .error message => throw <| IO.userError s!"nominal predicate fixture does not decode: {message}"
  let declaration := raw.namespaces[0]!.structs[0]!
  let binder := declaration.generics[0]!
  let binder := { binder with abilities := #[if unsupported then .store else .copy] }
  pure { raw with namespaces := raw.namespaces.set! 0 {
    raw.namespaces[0]! with structs := raw.namespaces[0]!.structs.set! 0 {
      declaration with generics := declaration.generics.set! 0 binder } } }

private def checkNominalPredicateRendering : IO Unit := do
  let unit ← match Rust.validate (← nominalPredicateFixture false) with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"nominal predicate fixture does not validate: {repr diagnostics}"
  let rendered ← match Rust.Source.render unit with
    | .ok rendered => pure rendered
    | .error message => throw <| IO.userError s!"nominal predicate does not render: {message}"
  unless rendered.contains "pub struct Wrapper<T> where T: Copy {" do
    throw <| IO.userError s!"nominal Copy predicate was not reconstructed:\n{rendered}"
  -- A `store` binder bound is unsatisfiable at its Rust-profile use sites, so
  -- authoritative typing rejects the unit before any backend runs.
  match Rust.validate (← nominalPredicateFixture true) with
  | .ok _ => throw <| IO.userError "unsupported nominal ability was silently accepted"
  | .error diagnostics =>
      unless diagnostics.any (·.code == "LIR-SEMANTIC-ABILITY") do
        throw <| IO.userError s!"unsupported nominal ability rejection has the wrong code: {repr diagnostics}"

private def checkNominalLifetimePredicateRendering : IO Unit := do
  let path : System.FilePath := "rust-exporter/tests/raw-unit/generic_lifetime_adt.exp.json"
  let raw ← match Import.decodeJson (← IO.FS.readFile path) with
    | .ok unit => pure unit
    | .error message => throw <| IO.userError s!"nominal lifetime fixture does not decode: {message}"
  let declaration := raw.namespaces[0]!.structs[0]!
  let binder := declaration.generics[0]!
  let binder := { binder with predicates := #[.lifetimeOutlives ⟨1⟩ ⟨1⟩] }
  let raw := { raw with namespaces := raw.namespaces.set! 0 {
    raw.namespaces[0]! with structs := raw.namespaces[0]!.structs.set! 0 {
      declaration with generics := declaration.generics.set! 0 binder } } }
  let unit ← match Rust.validate raw with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"nominal lifetime fixture does not validate: {repr diagnostics}"
  let rendered ← match Rust.Source.render unit with
    | .ok rendered => pure rendered
    | .error message => throw <| IO.userError s!"nominal lifetime predicate does not render: {message}"
  unless rendered.contains "pub struct Borrowed<'a, T> where 'a: 'a {" do
    throw <| IO.userError s!"nominal lifetime predicate was not reconstructed:\n{rendered}"

private def checkCoreCopyPredicateRendering : IO Unit := do
  let path : System.FilePath := "rust-exporter/tests/raw-unit/generic_adt.exp.json"
  let raw ← match Import.decodeJson (← IO.FS.readFile path) with
    | .ok unit => pure unit
    | .error message => throw <| IO.userError s!"core ability fixture does not decode: {message}"
  let declaration := raw.namespaces[0]!.structs[0]!
  let binder := declaration.generics[0]!
  let withAbility (ability : Ability) :=
    let abilities := if ability == .copy then #[.copy] else #[]
    { raw with namespaces := raw.namespaces.set! 0 {
      raw.namespaces[0]! with structs := raw.namespaces[0]!.structs.set! 0 {
        declaration with generics := declaration.generics.set! 0 {
          binder with abilities, predicates := #[.ability ⟨2⟩ ability] } } } }
  let unit ← match Rust.validate (withAbility .copy) with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"core Copy predicate fixture does not validate: {repr diagnostics}"
  let rendered ← match Rust.Source.render unit with
    | .ok rendered => pure rendered
    | .error message => throw <| IO.userError s!"core Copy predicate does not render: {message}"
  unless rendered.contains "pub struct Wrapper<T> where T: Copy {" &&
      (rendered.splitOn "T: Copy").length == 2 do
    throw <| IO.userError s!"core Copy predicate was not reconstructed:\n{rendered}"
  let unsupported ← match Rust.validate (withAbility .store) with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"core Store predicate fixture does not validate: {repr diagnostics}"
  match Rust.Source.render unsupported with
  | .error _ => pure ()
  | .ok _ => throw <| IO.userError "unsupported core Store predicate was silently omitted"

private def checkSemanticMetadataRejection : IO Unit := do
  let raw ← predicateFixture false
  let function := raw.namespaces[0]!.functions[0]!
  let raw := { raw with namespaces := raw.namespaces.set! 0 {
    raw.namespaces[0]! with functions := raw.namespaces[0]!.functions.set! 0 {
      function with contract := { function.contract with hasFrame := true } } } }
  let unit ← match Rust.validate raw with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"function metadata fixture does not validate: {repr diagnostics}"
  match Rust.Source.render unit with
  | .error _ => pure ()
  | .ok _ => throw <| IO.userError "function contract metadata was silently omitted"

  let raw ← nominalPredicateFixture false
  let declaration := raw.namespaces[0]!.structs[0]!
  -- Keep the mutated declaration well typed: declared abilities must be
  -- satisfied by every field under authoritative typing, so the generic field
  -- type receives the matching binder ability.
  let binder := declaration.generics[0]!
  let raw := { raw with namespaces := raw.namespaces.set! 0 {
    raw.namespaces[0]! with structs := raw.namespaces[0]!.structs.set! 0 {
      declaration with
        abilities := #[.copy]
        generics := declaration.generics.set! 0 {
          binder with abilities := #[.copy] } } } }
  let unit ← match Rust.validate raw with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"nominal metadata fixture does not validate: {repr diagnostics}"
  match Rust.Source.render unit with
  | .error _ => pure ()
  | .ok _ => throw <| IO.userError "nominal ability metadata was silently omitted"

#guard_msgs in
#eval checkFunctionPredicateRendering

#guard_msgs in
#eval checkNominalPredicateRendering

#guard_msgs in
#eval checkNominalLifetimePredicateRendering

#guard_msgs in
#eval checkCoreCopyPredicateRendering

#guard_msgs in
#eval checkSemanticMetadataRejection

end LeanerIR.Rust.Tests
