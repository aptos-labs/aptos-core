-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Frontend.XIR
import MoveModel.Tests.Common

/-! Helpers for tests which intentionally begin at the XIR exchange boundary. -/

namespace Tests

open MoveModel.IR
open MoveModel.Frontend.XIR

/-- Materialize an XIR fixture as semantic IR, then run it with the suite's
standard fuel. Semantic Move-source tests use `Tests.run` on `IR.Module`
directly. -/
def runXIR (m : MProgram) (f : String) (mem : IMem)
    (args : List Value) : Outcome :=
  interpFun m.toProgram 1000 (m.funId f) mem args

end Tests
