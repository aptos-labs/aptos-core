-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `condition_kinds` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/condition_kinds.move` that the source-verification
driver writes to `.verex/condition_kinds.move.lean`, its `verex sources at`
command dropped; the diagnostics are those of `condition_kinds.exp`. Neither
is edited here: `scripts/prover-smoke-renderings.sh --write` writes this file
again, and without `--write` tells whether it is what the frontend renders.
-/

/--
error: the module invariant `invariant exists<Config>(@0x42) ==> global<Config>(@0x42).interval >= 2` does not hold after a write of memory it reads
---
error: verex verification failed: the automatic verification of `drop_interval` failed; provide a proof in `VerexE2ETests/SourceVerify/condition_kinds.proof.lean`:
  verify drop_interval by lean =>
    verex_obligations
    verex_on
    | module_invariant => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
---
error: the data invariant `invariant interval > 0` does not hold where a mutation ends
---
error: verex verification failed: the automatic verification of `clear_interval` failed; provide a proof in `VerexE2ETests/SourceVerify/condition_kinds.proof.lean`:
  verify clear_interval by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
---
error: the data invariant `invariant rate > 0` does not hold where a mutation ends
---
error: verex verification failed: the automatic verification of `clear_rate` failed; provide a proof in `VerexE2ETests/SourceVerify/condition_kinds.proof.lean`:
  verify clear_rate by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
---
error: the module invariant `invariant exists<Flag>(@0x42) ==> exists<Config>(@0x42)` does not hold after a write of memory it reads
---
error: verex verification failed: the automatic verification of `publish_flag` failed; provide a proof in `VerexE2ETests/SourceVerify/condition_kinds.proof.lean`:
  verify publish_flag by lean =>
    verex_obligations
    verex_on
    | module_invariant => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_rendering in
verex module 0x42::condition_kinds where
  pragma proof_file = "VerexE2ETests/SourceVerify/condition_kinds.proof.lean"

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  -- One expression owes conditions of more than one kind: a mutable borrow of a
  -- resource owes the module's invariants where its write of memory ends, and
  -- the resource's data invariant where its mutation ends, both at its site.
  -- Each marker reads the condition of its own kind there, so a violation of
  -- either is still rejected and neither predicate stands for the other.
  --
  -- `Config`'s invariant is part of its carrier, so the certifying cut owns it
  -- and its mutation end owes nothing; `Pricing` holds a function value, so its
  -- invariant is not carried and the mutation end owes it. The module's last
  -- invariant reads `Pricing`'s memory, so the write of that borrow owes a
  -- condition of its own at the same site.
  struct Flag has Key where
    dummy_field : Bool

  struct Config has Key where
    interval : u64

  spec Config where
    invariant interval > 0

  struct Pricing has Key where
    rate : u64
    price : Fn(u64) -> u64 has Copy, Drop, Store

  spec Pricing where
    invariant rate > 0

  spec module where
    invariant exists<Flag>(@0x42) ==> exists<Config>(@0x42)

  spec module where
    invariant exists<Config>(@0x42) ==> global<Config>(@0x42).interval >= 2

  spec module where
    invariant exists<Pricing>(@0x42) ==> exists<Config>(@0x42)

  -- Both the module's invariants and the data invariant hold where the loan
  -- of `Config` ends.
  fun set_interval(interval : u64) -> Unit := do
    let config := &mut Config[@0x42]
    config.interval := interval

  spec set_interval where
    requires interval >= 2
    aborts_if !exists<Config>(@0x42)
    ensures global<Config>(@0x42).interval == interval
    modifies *

  -- The module's invariant is still checked where the write of a mutable
  -- borrow ends, although the data invariant holds.
  fun drop_interval() -> Unit := do
    let config := &mut Config[@0x42]
    config.interval := 1

  spec drop_interval where
    pragma aborts_if_is_partial
    modifies *

  -- error: the module invariant fails after the write
  -- The data invariant of a carried declaration is owed at the certifying
  -- cut, where its mutation ends owes nothing.
  fun clear_interval() -> Unit := do
    let config := &mut Config[@0x42]
    config.interval := 0

  spec clear_interval where
    pragma aborts_if_is_partial
    modifies *

  -- error: the data invariant fails
  -- The borrow of `Pricing` owes conditions of both kinds; both hold.
  fun set_rate(rate : u64) -> Unit := do
    let pricing := &mut Pricing[@0x42]
    pricing.rate := rate

  spec set_rate where
    requires rate > 0
    aborts_if !exists<Pricing>(@0x42)
    modifies *

  -- The data invariant owed where the mutation of `Pricing` ends, at the
  -- site that owes the module's invariant too.
  fun clear_rate() -> Unit := do
    let pricing := &mut Pricing[@0x42]
    pricing.rate := 0

  spec clear_rate where
    pragma aborts_if_is_partial
    modifies *

  -- error: the data invariant fails where the mutation ends
  -- A publication owes the module's invariants where its write ends, and no
  -- data invariant.
  fun publish_flag(owner : &Signer) -> Unit := do
    move_to<Flag>(owner, new Flag { dummy_field := false })

  spec publish_flag where
    pragma aborts_if_is_partial
    modifies *

  -- error: the module invariant fails after the write
