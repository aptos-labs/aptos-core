-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Tests.Move.MultipleModules
import Tests.Common

/-! Exchange-format checks underlying the idiomatic multiple-module example. -/

namespace Tests.MovePrograms.Modules.Client

open MoveModel.Frontend.XIR
open scoped Move.Compiler

def compiledForXirTest : MModule :=
  module% "ClientTest" structs [] functions [imported_identity]

#test compiledForXirTest.externalFuns.map (fun reference =>
    (reference.moduleName, reference.functionName)) = [("Math", "identity")]

private def externalNamesFromJson : Except String (List (String × String)) := do
  let decoded ← decodeMModule (← compiledForXirTest.encodeJson)
  return decoded.externalFuns.map fun reference =>
    (reference.moduleName, reference.functionName)

#test (match externalNamesFromJson with
  | .ok names => names == [("Math", "identity")]
  | .error _ => false) = true

end Tests.MovePrograms.Modules.Client
