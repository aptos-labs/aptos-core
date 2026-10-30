-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# In-body assumptions

A function's theorem assumes that its in-body assumptions hold where its
runs pass them (`AssumptionsHold`), and its proof uses them there. A caller
using its theorem assumes the same; one inlining it uses none of them.
-/

namespace VerexSL.Tests.Check.Specifications.Assumptions

verex module 0x42::in_body_assumptions where
  fun ordered(x : u64, y : u64) -> Unit := do
    spec assume x > y
    spec assert x >= y

  -- The assumption does not give the assertion.
  fun ordered_incorrect(x : u64, y : u64) -> Unit := do
    spec assume x >= y
    spec assert x > y

  fun increasing(a : u64, b : u64, c : u64) -> u64 := do
    spec assume a < b
    spec assume b < c
    a + b + c
  spec increasing where
    ensures result != 2

  fun inlines_increasing() -> u64 := increasing(1, 2, 3)
  spec inlines_increasing where
    ensures result == 6

  fun difference(a : u64, b : u64) -> u64 := do
    spec assume a < b
    b - a
  spec difference where
    pragma opaque
    ensures result > 0

  fun calls_difference() -> u64 := difference(1, 3)
  spec calls_difference where
    ensures result > 0

/-- The theorem of a caller through the contract takes the callee's
hypothesis. -/
example {registry : VerexIR.Validation.SemanticsRegistry}
    {executable : VerexIR.Validation.ExecutableUnit «0x42».in_body_assumptions.unit}
    (prepared : VerexIR.Validation.prepareExecution registry
      «0x42».in_body_assumptions.unit = .ok executable)
    (preserved : VerexIR.Proofs.Denote.GlobalsPreserved executable)
    (integral : VerexIR.Proofs.Denote.FailuresIntegral executable)
    (holds : ∀ {Θ : VerexIR.Proofs.Denote.Skolems «0x42».in_body_assumptions.unit}
      {typeInstantiation : Array (VerexIR.TypeId × VerexIR.TypeId)},
      VerexIR.Proofs.AssumptionsHold
        (@«0x42».in_body_assumptions.difference.plainMeaning executable Θ
          (VerexIR.Proofs.Denote.closedMeanings executable typeInstantiation))
        (@«0x42».in_body_assumptions.difference.assumedMeaning executable Θ
          (VerexIR.Proofs.Denote.closedMeanings executable typeInstantiation) executable)) :=
  «0x42».in_body_assumptions.calls_difference.verified prepared preserved integral holds

end VerexSL.Tests.Check.Specifications.Assumptions
