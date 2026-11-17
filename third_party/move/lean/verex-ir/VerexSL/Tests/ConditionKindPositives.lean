-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Tests.Rendered.condition_kinds

/-! # The theorems the condition-kinds rendering advertises

`#guard_rendering` compares the rendering's diagnostics, and the fixture
deliberately holds four of them. A cell that silently stopped being verified
would leave that comparison green, and a marker that read another kind's
condition would make a positive fail without changing the diagnostics of the
negatives. This names the artifacts instead: both positive cells have their
`typedVerified` theorem, and every negative cell has none.

The imported snapshot is elaborated by prover-smoke on its own
(`rendered condition_kinds`); here it is only imported, so this module costs
a read of its `.olean`.
-/

namespace VerexSL.Tests.ConditionKindPositives

open Lean Elab Command

set_option Elab.async false

/-- The cells that must keep a verified theorem: the mutation of a resource
whose data invariant is carried, where the write of memory owes the module's
invariants, and the mutation of one whose invariant is not carried, where the
site owes conditions of both kinds. -/
private def positives : List Name := [`set_interval, `set_rate]

/-- The cells that must keep none: the module invariant broken after the
write of a mutable borrow and after a publication, and the data invariant
broken where a carried and an uncarried declaration's mutation ends. -/
private def withoutTheorem : List Name :=
  [`drop_interval, `clear_interval, `clear_rate, `publish_flag]

run_cmd do
  let env ← getEnv
  let unit := `«0x42».condition_kinds
  for cell in positives do
    unless env.contains (unit ++ cell ++ `typedVerified) do
      throwError "the condition-kinds positive cell `{cell}` has no verified theorem"
  for cell in withoutTheorem do
    if env.contains (unit ++ cell ++ `typedVerified) then
      throwError "the condition-kinds cell `{cell}` is verified, which its \
        expectation says it is not: the fixture's baseline and this list \
        disagree, and one of them is now wrong"

end VerexSL.Tests.ConditionKindPositives
