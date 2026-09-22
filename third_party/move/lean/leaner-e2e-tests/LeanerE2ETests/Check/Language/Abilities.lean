-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

-- Port of v0 Language/Abilities. The Leaner surface spells out the generic
-- constraints that v0 inferred from fields and declared abilities.
leaner module 0x42::abilities where
  struct Plain where
    value : u64
  struct CopyDrop has Copy, Drop where
    value : u64
  struct Stored has Store where
    value : u64
  struct Resource has Key where
    value : u64
  struct GenericValue {T : type has Copy, Drop, Store} has Copy, Drop, Store where
    value : T
  struct GenericResource {T : type has Drop, Store} has Key, Drop where
    value : T
  struct Phantom {T : phantom type} has Copy, Drop, Store where
  enum Droppable has Drop where
    | First
    | Second (value : u64)

open Lean Elab Command LeanerIR in
run_cmd do
  let some unit := LeanerLang.registeredUnit? (← getEnv) `«0x42».abilities
    | throwError "missing abilities module"
  let some namespace_ := unit.namespaces[0]? | throwError "missing abilities namespace"
  unless namespace_.structs.size == 8 do throwError "lost an ability declaration"
  let declaration (name : String) : CommandElabM StructDecl := do
    let some declaration := namespace_.structs.find? fun declaration =>
        (unit.tables.names[declaration.name.index]?.map (·.name)) == some name
      | throwError "missing ability declaration {name}"
    return declaration
  for (name, expected) in [
      ("Plain", #[]), ("CopyDrop", #[Ability.copy, .drop]), ("Stored", #[.store]),
      ("Resource", #[.key]), ("Droppable", #[.drop])] do
    unless (← declaration name).abilities == expected do
      throwError "incorrect declared abilities for {name}"
  for (name, expected) in [
      ("GenericValue", #[Ability.copy, .drop, .store]),
      ("GenericResource", #[.drop, .store]), ("Phantom", #[])] do
    let value ← declaration name
    unless value.generics.size == 1 && value.generics[0]!.name == "T" &&
        value.generics[0]!.abilities == expected do
      throwError "incorrect generic abilities for {name}"
  let phantom ← declaration "Phantom"
  unless phantom.generics[0]!.predicates ==
      #[.profile { profile := .move, tag := "typeParameter.phantom" }] do
    throwError "phantom parameter lost its profile marker"
  let droppable ← declaration "Droppable"
  unless droppable.variants.size == 2 && droppable.variants[1]!.fields.size == 1 do
    throwError "droppable enum lost its payload"
