-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
import Lean

namespace VerexIR.Tests.CanonicalEmissionState
structure Observation where
  count : Nat := 0
  badCount : Nat := 0
  firstBad : String := ""
  deriving Inhabited

initialize observation : IO.Ref Observation ← IO.mkRef {}
end VerexIR.Tests.CanonicalEmissionState
