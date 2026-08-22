-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Tests.MultipleModules
import MoveModel.Tests.Common

/-! Explicit low-level module selection for the multiple-module example. -/

namespace Tests.MovePrograms.Modules.Client

open scoped Move.Compiler

def compiledForTest : MoveModel.IR.Module :=
  module% "ClientTest" structs [] functions [imported_identity]

#test compiledForTest.externalFuns.map (fun reference =>
    (reference.moduleName, reference.functionName)) = [("Math", "identity")]

end Tests.MovePrograms.Modules.Client
