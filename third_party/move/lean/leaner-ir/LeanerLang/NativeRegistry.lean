-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Typed

namespace LeanerLang.NativeRegistry

open Lean

/-- Published only after native generation has completed without errors.
These certificates support calls whose native computations may abort. -/
structure Entry where
  relation : Name
  base : Name
  artifacts : Typed.Artifacts
  rawContract : Name
  typedContract : Name

initialize entries : SimplePersistentEnvExtension Entry (NameMap Entry) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun state entry => state.insert entry.relation entry
    addImportedFn := fun imported => mkStateFromImportedEntries
      (fun state entry => state.insert entry.relation entry) {} imported }

end LeanerLang.NativeRegistry
