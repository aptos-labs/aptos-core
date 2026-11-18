-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `requires_of_disjunct` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/requires_of_disjunct.move` that the
source-verification driver writes to `.verex/requires_of_disjunct.move.lean`,
its `verex sources at` command dropped; the diagnostics are those of
`requires_of_disjunct.exp`. Neither is edited here:
`scripts/prover-smoke-renderings.sh --write` writes this file again, and
without `--write` tells whether it is what the frontend renders.
-/

/--
error: the precondition `requires skip || requires_of<f>(x)` of `modify` does not hold at call 1
---
error: verex verification failed: the automatic verification of `call_disjunct` failed; provide a proof in `VerexE2ETests/SourceVerify/requires_of_disjunct.proof.lean`:
  verify call_disjunct by lean =>
    verex_obligations
    verex_on
    | requires_1_of_modify_at_call_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_rendering in
verex module 0x42::requires_of_disjunct where
  pragma proof_file = "VerexE2ETests/SourceVerify/requires_of_disjunct.proof.lean"

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  -- A precondition `skip || requires_of<f>(x)` of a closure taking `&mut` is not
  -- established at the call, and `call_disjunct` is a parked gap. `call_plain` is
  -- the control: the precondition `requires_of<f>(x)` alone holds at its call.
  fun modify(skip : Bool, x : &mut u64, f : Fn(&mut u64) -> u64) -> u64 := do
    if skip then return 0;
    invoke(f, x)

  spec modify where
    pragma opaque
    pragma verify = false
    requires skip || requires_of<f>(x)
    aborts_if !skip && aborts_of<f>(x)

  fun modify_plain(x : &mut u64, f : Fn(&mut u64) -> u64) -> u64 := invoke(f, x)

  spec modify_plain where
    pragma opaque
    pragma verify = false
    requires requires_of<f>(x)
    aborts_if aborts_of<f>(x)

  fun call_disjunct(skip : Bool) -> u64 := do
    let mut x := 1
    modify(
      skip, &mut x, function[Fn(&mut u64) -> u64](__lambda__1__call_disjunct)
    )

  fun __lambda__1__call_disjunct(v : &mut u64) -> u64 := do
    let o := *v
    *v := 50
    o

  fun call_plain() -> u64 := do
    let mut x := 1
    modify_plain(&mut x, function[Fn(&mut u64) -> u64](__lambda__1__call_plain))

  fun __lambda__1__call_plain(v : &mut u64) -> u64 := do
    let o := *v
    *v := 50
    o
