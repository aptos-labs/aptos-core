-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Typed

namespace LeanerLang.NativeLoopInfo

open Lean

/-- Source-authored invariant over a typed header product. The predicate
takes function arguments, entry/header locals, and the current store. -/
structure Loop where
  site : LeanerIR.ExprId
  slots : Array LeanerIR.LocalId
  representations : Array Typed.ValueRep
  predicate : Name

structure Entry where
  function : Name
  loops : Array Loop

initialize entries : SimplePersistentEnvExtension Entry (NameMap (Array Loop)) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun state entry => state.insert entry.function entry.loops
    addImportedFn := fun imported => mkStateFromImportedEntries
      (fun state entry => state.insert entry.function entry.loops) {} imported }

end LeanerLang.NativeLoopInfo
