-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Tests.Rendered.map_key_default
import VerexSL.Tests.Rendered.map_value_default

/-! # The theorems the map-default renderings advertise

Each snapshot's `#guard_rendering` compares the diagnostics of a module
whose negatives fail; a positive cell that stopped being scheduled, or
stopped proving anything, would leave those diagnostics as they are. This
names the artifacts instead: each cell whose proof applies a map lemma has
its `typedVerified` theorem, and each negative has none.

A positive is a fact a built-in map lemma states against an assertion that
reads the map outside it, both by the owner's typed view: the keys' default
for a key of its map (`map_key_default`) and the values' for a value of it
(`map_value_default`). The narrow and generic cells hold owners whose two
views differ, so the one default is not the other.

The snapshots are elaborated by prover-smoke on their own
(`rendered map_key_default`, `rendered map_value_default`); here they are
only imported, so this module costs a read of their `.olean`s.
-/

namespace VerexSL.Tests.MapDefaultPositives

open Lean Elab Command

set_option Elab.async false

/-- The cells that must keep a verified theorem: the key lemma at a
symbolic position, at a ground one, at a map whose values are of another
width than its keys and at one whose keys are a type parameter; the value
lemmas of an update and of a removal at a map of byte keys and word
values. -/
private def positives : List (Name × Name) :=
  [(`map_key_default, `key_at_rank), (`map_key_default, `key_at_ground),
    (`map_key_default, `key_at_rank_narrow), (`map_key_default, `key_at_rank_generic),
    (`map_value_default, `value_at_set), (`map_value_default, `value_at_erase)]

/-- The cells that must keep none: the lemma's fact is of another key, a
read outside the map, an integer-keyed law at keys that are no integers,
and the value of a key without an entry. The reads are unspecified, not
zero; the law is refused, not synthesized. -/
private def withoutTheorem : List (Name × Name) :=
  [(`map_key_default, `key_at_other), (`map_key_default, `key_at_outside),
    (`map_key_default, `key_at_lt_generic), (`map_value_default, `value_at_missing)]

run_cmd do
  let env ← getEnv
  let theorem_ (unit cell : Name) : Name := `«0x1» ++ unit ++ cell ++ `typedVerified
  for (unit, cell) in positives do
    unless env.contains (theorem_ unit cell) do
      throwError "the positive cell `{unit}.{cell}` has no verified theorem: the map \
        lemma's fact is not the fact its assertion owes"
  for (unit, cell) in withoutTheorem do
    if env.contains (theorem_ unit cell) then
      throwError "the cell `{unit}.{cell}` is verified, which its expectation says it is \
        not: the fixture's baseline and this list disagree, and one of them is now wrong"

end VerexSL.Tests.MapDefaultPositives
