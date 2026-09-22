-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import LeanerLang.Profile

namespace LeanerLang

open Lean
open LeanerIR.Validation

private initialize unitExtension :
    SimplePersistentEnvExtension (Name × ValidatedUnit) (NameMap ValidatedUnit) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun units (name, unit) => units.insert name unit
    addImportedFn := fun entries =>
      mkStateFromImportedEntries
        (fun units (name, unit) => units.insert name unit) {} entries }

/-- Retrieve a validated Leaner namespace registered in this module or an
imported module. -/
def registeredUnit? (environment : Environment) (name : Name) : Option ValidatedUnit :=
  (unitExtension.getState environment).find? name

/-- Persist a checked source unit. Repeating an identical declaration is
idempotent; changing the meaning of an existing name is rejected. -/
def registerUnit (environment : Environment) (name : Name) (unit : ValidatedUnit) :
    Except String Environment :=
  match registeredUnit? environment name with
  | none => .ok (unitExtension.addEntry environment (name, unit))
  | some previous =>
      if previous == unit then .ok environment
      else .error s!"Leaner namespace `{name}` is already registered with a different unit"

end LeanerLang
