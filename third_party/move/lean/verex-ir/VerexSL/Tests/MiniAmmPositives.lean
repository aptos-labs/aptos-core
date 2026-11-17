-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Tests.Rendered.mini_amm

/-! # The theorems the mini-AMM rendering advertises

`#guard_rendering` compares the rendering's diagnostics, and the cost pin of
`quote` says that one target ran. Neither says that the other positive cells
still prove anything, and the module deliberately holds a caught elaboration
error, so a cell that silently stopped being verified would leave both of them
green. This names the artifacts instead: every positive cell has its
`typedVerified` theorem, and every negative and open cell has none.

The imported snapshot is elaborated by prover-smoke on its own
(`rendered mini_amm`); here it is only imported, so this module costs a read
of its `.olean`.
-/

namespace VerexSL.Tests.MiniAmmPositives

open Lean Elab Command

set_option Elab.async false

/-- The cells that must keep a verified theorem: the compliant pricing
function and its pack, the quote that uses both rules, the publish, the two
reference-parameter quotes, the pricing functions of the negatives (whose own
contracts hold; it is their packs that fail), the `Fee` pair's two functions
with the guarded pack, and the `result_of` wrapper's function and pack. -/
private def positives : List Name :=
  [`capped, `make_capped, `quote, `create, `quote_pool, `quote_pool_mut,
    `aborting, `whole, `fee_guarded, `make_fee_guarded, `fee_unguarded,
    `identity, `make_bounded]

/-- The cells that must keep none: the three negatives, the `Fee` pair's
negative, and the three open ones (`quote_stored` is never scheduled, and
`quote_stored_unbounded` and `quote_bounded` leave an obligation). -/
private def withoutTheorem : List Name :=
  [`make_aborting, `make_whole, `quote_strict, `make_fee_unguarded,
    `quote_stored, `quote_stored_unbounded, `quote_bounded]

run_cmd do
  let env ← getEnv
  let unit := `«0x42».mini_amm
  for cell in positives do
    unless env.contains (unit ++ cell ++ `typedVerified) do
      throwError "the mini-AMM's positive cell `{cell}` has no verified theorem"
  for cell in withoutTheorem do
    if env.contains (unit ++ cell ++ `typedVerified) then
      throwError "the mini-AMM's cell `{cell}` is verified, which its \
        expectation says it is not: the fixture's baseline and this list \
        disagree, and one of them is now wrong"

end VerexSL.Tests.MiniAmmPositives
