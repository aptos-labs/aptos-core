-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Tests.Rendered.map_size_update

/-! # The theorems the map-size rendering advertises

The snapshot's `#guard_rendering` compares the diagnostics of a module
whose negative fails; a positive cell that stopped being scheduled, or
stopped proving anything, would leave those diagnostics as they are. This
names the artifacts instead: each cell whose authored proof applies a size
law of the library has its `typedVerified` theorem, and the negative has
none.

The two laws the proofs apply, `SpecMap.size_update_sequence_present` and
`SpecMap.size_update_sequence_absent`, are stated beside their parent in
`VerexIR/Proofs/Denote/NativeMaps.lean`; this is the path by which a proof
file reaches them through the owner's own `add`.

The snapshot is elaborated by prover-smoke on its own
(`rendered map_size_update`); here it is only imported, so this module
costs a read of its `.olean`.
-/

namespace VerexSL.Tests.MapSizeUpdatePositives

open Lean Elab Command

set_option Elab.async false

/-- The cells that must keep a verified theorem: the size after an entry of
a key the map does not hold, and after one of a key it holds. -/
private def positives : List Name := [`size_after_insert, `size_after_override]

/-- The cell that must keep none: of a key that may be held, neither size
is stated, and the automatic route states neither. -/
private def withoutTheorem : List Name := [`size_after_unknown]

run_cmd do
  let env ← getEnv
  let theorem_ (cell : Name) : Name := `«0x1» ++ `map_size_update ++ cell ++ `typedVerified
  for cell in positives do
    unless env.contains (theorem_ cell) do
      throwError "the positive cell `map_size_update.{cell}` has no verified theorem: the \
        size law the proof applies does not establish what its assertion owes"
  for cell in withoutTheorem do
    if env.contains (theorem_ cell) then
      throwError "the cell `map_size_update.{cell}` is verified, which its expectation says \
        it is not: the fixture's baseline and this list disagree, and one of them is now wrong"

end VerexSL.Tests.MapSizeUpdatePositives
