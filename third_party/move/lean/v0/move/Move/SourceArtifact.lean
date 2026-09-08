-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-!
# Retained Leaner Move module source

The `module` elaborator expands source items into ordinary Lean declarations,
compiler attributes, and verification definitions.  Some of that expansion is
necessarily many-to-one: in particular, the generated contract theorem does
not retain the authored specification-clause boundary.

This persistent artifact keeps the already parsed module items at the
frontend boundary.  Consumers must translate them to their own semantic IR;
the artifact is not a backend representation and must not be consulted after
raw LIR construction.
-/

namespace Move

open Lean

structure SourceModuleArtifact where
  /-- The complete parsed items between `module ... where` and its end. -/
  items : Array Syntax
  deriving Inhabited

private initialize sourceModuleArtifactExt :
    SimplePersistentEnvExtension (Name × SourceModuleArtifact)
      (NameMap SourceModuleArtifact) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun modules (namespaceName, artifact) =>
      modules.insert namespaceName artifact
    addImportedFn := fun entries =>
      mkStateFromImportedEntries
        (fun modules (namespaceName, artifact) =>
          modules.insert namespaceName artifact) {} entries
  }

/-- Persist the parsed source owned by one Leaner Move module namespace. -/
def registerSourceModuleArtifact (env : Environment) (namespaceName : Name)
    (artifact : SourceModuleArtifact) : Environment :=
  sourceModuleArtifactExt.addEntry env (namespaceName, artifact)

/-- Retrieve the parsed source of a local or imported Leaner Move module. -/
def sourceModuleArtifact? (env : Environment) (namespaceName : Name) :
    Option SourceModuleArtifact :=
  (sourceModuleArtifactExt.getState env).find? namespaceName

end Move
