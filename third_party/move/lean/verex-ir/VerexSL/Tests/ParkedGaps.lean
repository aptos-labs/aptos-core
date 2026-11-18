-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Tests.Rendered.split_reference_end
import VerexSL.Tests.Rendered.split_pair_frame
import VerexSL.Tests.Rendered.generic_join_ghost
import VerexSL.Tests.Rendered.pure_spec_map_fact

/-! # The gaps four renderings record, and the controls beside them

Each of these four fixtures holds a target the solver does not verify, kept
as a reproducer of a gap that is parked rather than repaired, and beside it a
control that exercises the same construct and does verify. The diagnostics of
a gap are compared by its snapshot's `#guard_rendering`; they say nothing
about the control, and a gap whose message merely changed its wording would
still read as a gap. This names the artifacts instead: every target below is
scheduled (it has a `typedContract` and an attempt), each control has its
`typedVerified` theorem, and each gap has none.

A repair that closes one of these gaps makes this test fail and names the
fixture: the gap's row then moves to the positives, its section goes from
`scripts/prover-smoke-coverage.md`'s "Parked gaps" to the regression classes
the suite guards, and the fixture's baseline is written again.

The snapshots are elaborated by prover-smoke on their own (`rendered
split_reference_end` and the other three); here they are only imported, so
this module costs a read of their `.olean`s.
-/

namespace VerexSL.Tests.ParkedGaps

open Lean Elab Command

set_option Elab.async false

/-- A fixture's unit, the targets whose verification is the parked gap, and
the controls that must keep verifying beside them. -/
private structure Fixture where
  unit : Name
  gaps : List Name
  controls : List Name

/-- The four gaps.

`split_reference_end`: the end of a `&mut` of the whole vector splits on the
loop's state, and the split leaves a pattern equation that is not in normal
form, so the authored `compute` of `drain` keeps a `data_invariant` leaf; the
same loop by the automatic route verifies.

`split_pair_frame`: the same mechanism under a match on a pair, where the
closure invoke drops the equations the split left, so `process` stops on a
residual obligation; the pair match without the invoke verifies.

`generic_join_ghost`: the unit declares a spec ghost, so the generic callee
runs in its caller's walk, and what the join owes of the values it abstracts
is stated at the carriers the type arguments induce while the arriving paths
state their facts at the caller's; a join of plain values owes nothing there
and verifies.

`pure_spec_map_fact`: at a spec-only map owner, the length-update lemma and
the precondition equating a parameter with an updated map do not close these
authored assertions; the parameter-map rank control verifies. The failing
contexts include True hypotheses, but the first lost correspondence and cause
remain unlocalized. -/
private def fixtures : List Fixture :=
  [{ unit := `«0x42».split_reference_end, gaps := [`drain], controls := [`drain_automatic] },
   { unit := `«0x43».split_pair_frame, gaps := [`process], controls := [`plain] },
   { unit := `«0x42».generic_join_ghost, gaps := [`settle_token], controls := [`pick_amount] },
   { unit := `«0x1».pure_spec_map_fact, gaps := [`v_len_set, `v_set_param],
     controls := [`v_rank_nonneg] }]

run_cmd do
  let env ← getEnv
  for fixture in fixtures do
    for cell in fixture.gaps ++ fixture.controls do
      let target := fixture.unit ++ cell
      unless env.contains (target ++ `typedContract) do
        throwError "the target `{target}` is not scheduled: the fixture no longer holds the \
          cell this test is about"
      unless env.contains (target ++ `typedVerified ++ `attempt_1) do
        throwError "the target `{target}` was not attempted: a verification this test reads \
          the outcome of did not run"
    for cell in fixture.controls do
      unless env.contains (fixture.unit ++ cell ++ `typedVerified) do
        throwError "the control `{fixture.unit ++ cell}` has no verified theorem: the \
          construct its gap is about is broken beyond that gap, so the fixture no longer \
          distinguishes the two"
    for cell in fixture.gaps do
      if env.contains (fixture.unit ++ cell ++ `typedVerified) then
        throwError "the parked gap `{fixture.unit ++ cell}` is verified. If a repair closed \
          it, promote the fixture: move its row of `scripts/prover-smoke-coverage.md` out of \
          \"Parked gaps\", name the target here as a control, and write the baseline again"

end VerexSL.Tests.ParkedGaps
