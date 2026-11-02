-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import VerexSL.Profile

namespace VerexSL

open Lean
open VerexIR.Validation

private initialize unitExtension :
    SimplePersistentEnvExtension (Name × ValidatedUnit) (NameMap ValidatedUnit) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun units (name, unit) => units.insert name unit
    addImportedFn := fun entries =>
      mkStateFromImportedEntries
        (fun units (name, unit) => units.insert name unit) {} entries }

/-- Retrieve a validated Verex namespace registered in this module or an
imported module. -/
def registeredUnit? (environment : Environment) (name : Name) : Option ValidatedUnit :=
  (unitExtension.getState environment).find? name

/-- Every validated Verex namespace registered in this module or an imported
module. -/
def registeredUnits (environment : Environment) : List (Name × ValidatedUnit) :=
  (unitExtension.getState environment).toList

/-- The validated Verex namespaces this module registers, in order. -/
def moduleUnits (environment : Environment) : List (Name × ValidatedUnit) :=
  (unitExtension.getEntries environment).reverse

/-- Persist a checked source unit. Repeating an identical declaration is
idempotent; changing the meaning of an existing name is rejected. -/
def registerUnit (environment : Environment) (name : Name) (unit : ValidatedUnit) :
    Except String Environment :=
  match registeredUnit? environment name with
  | none => .ok (unitExtension.addEntry environment (name, unit))
  | some previous =>
      if previous == unit then .ok environment
      else .error s!"Verex namespace `{name}` is already registered with a different unit"

end VerexSL
