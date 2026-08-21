-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Tests.Move.Modules.Math

/-!
# Multiple Lean-authored Move modules

Importing another Move module is an ordinary Lean import.  Its functions are
available under the module namespace for both source-level proofs and Move
bytecode translation.
-/

namespace Tests.MovePrograms.Modules

open Move
open scoped Move Move.Compiler Move.Spec

move_module Client where

  /-! ## Functions -/

  fun importedIdentity (value : U64) : U64 :=
    Math.identity (Math.identity value)

  spec importedIdentity (value : U64) where
    ensures result = value

/-! ## Proofs -/

/-- Imported specifications and kernel-checked proofs are ordinary Lean
declarations available to downstream proofs. -/
theorem importedMathContract : Math.identity.contract := Math.identity.verified

namespace Client

  verify importedIdentity

end Client

end Tests.MovePrograms.Modules
