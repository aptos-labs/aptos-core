-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# In-body assumptions

A function's theorem assumes that its in-body assumptions hold where its
runs pass them (`AssumptionsHold`), and its proof uses them there. A caller
using its theorem assumes the same; one inlining it uses none of them.
-/

namespace LeanerLang.Tests.Check.Specifications.Assumptions

leaner module 0x42::in_body_assumptions where
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
example {registry : LeanerIR.Validation.SemanticsRegistry}
    {executable : LeanerIR.Validation.ExecutableUnit «0x42».in_body_assumptions.unit}
    (prepared : LeanerIR.Validation.prepareExecution registry
      «0x42».in_body_assumptions.unit = .ok executable)
    (preserved : LeanerIR.Proofs.Denote.GlobalsPreserved executable)
    (holds : ∀ {Θ : LeanerIR.Proofs.Denote.Skolems «0x42».in_body_assumptions.unit}
      {typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId)},
      LeanerIR.Proofs.AssumptionsHold
        (@«0x42».in_body_assumptions.difference.plainMeaning executable Θ typeInstantiation)
        (@«0x42».in_body_assumptions.difference.assumedMeaning executable Θ typeInstantiation)) :=
  «0x42».in_body_assumptions.calls_difference.verified prepared preserved holds

end LeanerLang.Tests.Check.Specifications.Assumptions
