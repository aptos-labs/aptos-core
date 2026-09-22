-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Port of v0 Language/Attributes. Nested calls and assigned literals are
structured declaration metadata; intrinsic bindings have a separate meaning.
LeanerLang spells v0's positional `randomness 7` as `randomness = 7`. -/

leaner module 0x42::attributes where
  @[resource_group (scope («global»))]
  struct Registry has Key where
    value : u64

  /-- Documentation and declaration attributes are retained together. -/
  @[resource_group_member (Registry), custom_marker]
  enum Mode has Drop where
    | Idle
    | Busy (level : u64)

  @[«view»]
  fun peek(x : u64) -> u64 := x + 0

  @[semantic_marker (Registry)]
  opaque spec fun modeled_value(value : Registry) : Int

  @[defined_spec_marker]
  spec fun modeled_flag(value : Registry) : Bool := value.value == 0

  @[randomness = 7, «lint.skip»]
  entry fun act(addr : Address) -> Unit := do
    let value := &mut Registry[addr].value
    *value := *value + 1

  @[move_public]
  fun compatPublic(x : u64) -> u64 := x

/- Ordinary metadata may accompany intrinsic declarations and roles without
being added to the role graph. -/
leaner module 0x42::attribute_intrinsics where
  @[intrinsic_map, owner_note]
  struct Table {K} {V} where
    marker : Bool

  @[map_new (Table), allocation_note]
  native fun empty {K} {V} () -> Table<K, V>

  @[map_spec_len (Table), specification_note]
  opaque spec fun size {K} {V} (table : Table<K, V>) : Int

namespace LeanerLang.Tests.Attributes

open Lean Elab Command LeanerIR

private partial def withoutLocations : LeanerIR.Attribute → LeanerIR.Attribute
  | .call name arguments _ => .call name (arguments.map withoutLocations)
  | .assign name value _ => .assign name value

private def metadata (unit : Validation.ValidatedUnit) : Array (String × Array LeanerIR.Attribute) :=
  let ns := unit.namespaces[0]!
  let nameOf (name : NameId) := ns.tables.names[name.index]!.name
  (ns.structs.map fun declaration =>
    (nameOf declaration.name, declaration.attributes.map withoutLocations)) ++
  (ns.functions.map fun declaration =>
    (nameOf declaration.name, declaration.attributes.map withoutLocations)) ++
  (ns.specFunctions.filterMap fun declaration =>
    if declaration.contract.pragmas.isEmpty then none else
      some (nameOf declaration.name, declaration.contract.pragmas.map withoutLocations))

run_cmd do
  let env ← getEnv
  let some unit := registeredUnit? env `«0x42».attributes
    | throwError "missing attribute module"
  let entries := metadata unit
  let expect (name : String) (attributes : Array LeanerIR.Attribute) : CommandElabM Unit := do
    unless entries.find? (·.1 == name) == some (name, attributes) do
      throwError "metadata mismatch for {name}: {repr entries}"
  expect "Registry" #[.call "resource_group" #[.call "scope" #[.call "global" #[]]]]
  expect "Mode" #[.call "resource_group_member" #[.call "Registry" #[]], .call "custom_marker" #[]]
  expect "peek" #[.assign "visibility" (.qualifiedName "private"), .call "view" #[]]
  expect "act" #[.assign "visibility" (.qualifiedName "private"), .call "entry" #[],
    .assign "randomness" (.constant (.integer 7)), .call "lint.skip" #[]]
  expect "compatPublic" #[.assign "visibility" (.qualifiedName "public")]
  expect "modeled_value" #[.call "semantic_marker" #[.call "Registry" #[]]]
  expect "modeled_flag" #[.call "defined_spec_marker" #[]]
  unless unit.namespaces[0]!.intrinsics.isEmpty do
    throwError "user metadata was interpreted as intrinsic bindings"
  let roundtrip ← match Print.reimportUnit env unit with
    | .ok result => pure result
    | .error failure => throwError "attribute reimport failed: {failure.message}"
  unless metadata roundtrip == entries do
    throwError "structured declaration metadata changed during reimport"
  let source ← match Print.render env unit with
    | .ok result => pure result
    | .error failure => throwError "attribute rendering failed: {failure.message}"
  let canonical ← match Print.render env roundtrip with
    | .ok result => pure result
    | .error failure => throwError "reimported attribute rendering failed: {failure.message}"
  unless source == canonical do
    throwError "attribute source is not a canonical print/reparse fixed point"

run_cmd do
  let env ← getEnv
  let some unit := registeredUnit? env `«0x42».attribute_intrinsics
    | throwError "missing mixed intrinsic-attribute module"
  let ns := unit.namespaces[0]!
  unless ns.intrinsics.size == 1 do throwError "lost intrinsic owner"
  let intrinsic := ns.intrinsics[0]!
  unless intrinsic.model == "map" &&
      intrinsic.executableBindings.map (·.role) == #["map_new"] &&
      intrinsic.specBindings.map (·.role) == #["map_spec_len"] do
    throwError "user metadata contaminated the intrinsic role graph"
  let entries := metadata unit
  unless entries.find? (·.1 == "Table") == some ("Table", #[.call "owner_note" #[]]) &&
      entries.find? (·.1 == "size") == some ("size", #[.call "specification_note" #[]]) do
    throwError "ordinary intrinsic-owner/role metadata was lost"
  let roundtrip ← match Print.reimportUnit env unit with
    | .ok result => pure result
    | .error failure => throwError "mixed intrinsic reimport failed: {failure.message}"
  unless metadata roundtrip == entries &&
      roundtrip.namespaces[0]!.intrinsics.size == 1 &&
      roundtrip.namespaces[0]!.intrinsics[0]!.executableBindings.map (·.role) == #["map_new"] &&
      roundtrip.namespaces[0]!.intrinsics[0]!.specBindings.map (·.role) == #["map_spec_len"] do
    throwError "intrinsic/metadata separation changed during reimport"

end LeanerLang.Tests.Attributes

open Lean Elab Command LeanerLang in
run_cmd do
  let env ← getEnv
  for (source, expected) in #[
      ("leaner module 0x42::bad_owner where\n" ++
        "  @[intrinsic_map (Table)]\n  struct Table where\n    marker : Bool\n",
        "an intrinsic owner attribute takes no arguments"),
      ("leaner module 0x42::missing_owner where\n" ++
        "  struct Table where\n    marker : Bool\n" ++
        "  @[map_new (Table)]\n  native fun empty() -> Table\n",
        "attribute `map_new` names `Table`, which has no intrinsic marker"),
      ("leaner module 0x42::wrong_target where\n" ++
        "  @[map_new (Table)]\n  struct Table where\n    marker : Bool\n",
        "an intrinsic role attribute must annotate a function or specification function"),
      ("leaner module 0x42::bad_role where\n" ++
        "  @[intrinsic_map]\n  struct Table where\n    marker : Bool\n" ++
        "  @[map_new = 7]\n  native fun empty() -> Table\n",
        "attribute `map_new` must name exactly one intrinsic owner")] do
    let parsedSyntax ← match Parser.runParserCategory env `command source with
      | .ok result => pure result
      | .error failure => throwError "attribute rejection fixture did not parse: {failure}"
    let parsed ← match compilationUnitOfSyntax parsedSyntax "<attribute rejection>" with
      | .ok result => pure result
      | .error (_, failure) => throwError "attribute rejection fixture did not elaborate: {failure}"
    match compile parsed with
    | .error (.frontend diagnostics) =>
      unless diagnostics.size == 1 && diagnostics[0]!.code == "LEANER-ATTRIBUTE" &&
          diagnostics[0]!.message == expected do
        throwError "wrong intrinsic-attribute rejection: {repr diagnostics}"
    | .error (.lir diagnostics) => throwError "wrong LIR rejection: {repr diagnostics}"
    | .ok _ => throwError "malformed intrinsic attribute was accepted"
