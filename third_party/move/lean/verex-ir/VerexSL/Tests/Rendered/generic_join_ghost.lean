-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `generic_join_ghost` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/generic_join_ghost.move` that the
source-verification driver writes to `.verex/generic_join_ghost.move.lean`,
its `verex sources at` command dropped; the diagnostics are those of
`generic_join_ghost.exp`. Neither is edited here:
`scripts/prover-smoke-renderings.sh --write` writes this file again, and
without `--write` tells whether it is what the frontend renders.
-/

/--
error: the data invariant of `((named, ()), value)` at join 1 is not established
---
error: the data invariant of `value.snd` at join 1 is not established
---
error: verex verification failed: the automatic verification of `settle_token` failed; provide a proof in `VerexE2ETests/SourceVerify/generic_join_ghost.proof.lean`:
  verify settle_token by lean =>
    verex_obligations
    verex_on
    | data_invariant => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_rendering in
verex module 0x42::generic_join_ghost where
  pragma proof_file = "VerexE2ETests/SourceVerify/generic_join_ghost.proof.lean"

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  -- The unit declares a spec ghost, so a generic callee runs in its caller's
  -- walk: automatic verification of `settle_token` fails at the join's
  -- induced-family premises, while the inspected authored `verex_decide` route
  -- verifies without forming that join. `pick_amount` is the control.
  struct Handle {T : phantom type} has Copy, Drop, Store where
    inner : Address

  struct Token has Key where
    dummy_field : Bool

  struct Fast has Key where
    current : u64

  struct Slow has Key where
    current : u64
    log : Vector<u64>

  struct «Ghost$settled» has Copy, Drop, Store, Key where
    v : Int

  spec «Ghost$settled» where
    pragma ghost_memory

  fun settle {T has Key}(handle : &Handle<T>, amount : u64) -> Unit := do
    if amount == 0 then return ();
    let owner := handle.inner
    if exists<Fast>(owner) then
      let fast := &mut Fast[owner]
      assert!(fast.current >= amount, 1)
      fast.current := fast.current - amount
    else
      if exists<Slow>(owner) then
        let slow := &mut Slow[owner]
        assert!(slow.current >= amount, 2)
        let current := &mut slow.current
        *current := *current - amount
      else abort(3)

  spec settle where
    modifies *

  fun settle_token(handle : Handle<Token>, amount : u64) -> Unit := do
    settle(&handle, amount)

  spec settle_token where
    aborts_if amount != 0 && !exists<Fast>(handle.inner)
        && !exists<Slow>(handle.inner)
    aborts_if amount != 0 && exists<Fast>(handle.inner)
        && global<Fast>(handle.inner).current < amount
    aborts_if amount != 0 && !exists<Fast>(handle.inner)
        && exists<Slow>(handle.inner)
        && global<Slow>(handle.inner).current < amount
    modifies *

  fun pick {T has Copy, Drop}(a : T, b : T, first : Bool) -> T :=
    if first then a else b

  fun pick_amount(a : u64, b : u64, first : Bool) -> u64 := pick(a, b, first)

  spec pick_amount where
    aborts_if false
