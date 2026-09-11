-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# End-to-end verification over the prophetic reference model

The gate demos of [`designs/prophetic-references.md`](../../../designs/prophetic-references.md):
reference-taking functions authored in LeanerLang verify through
`leaner_wp` against contracts phrased in the prophetic vocabulary.

Under ownership passing a borrow arrives as a value carrying the loaned
content; writes update the borrow at rest; the function's dying frame
exports each unreconciled mutable loan into the state's pending set. A
contract names the loan's exported final value — the caller's view of the
prophecy — through the pending-push equation, and assumes the loan's hole
is not in a global slot (it lives in a caller frame), which keeps the
write-back search symbolic.

Two routes are exercised:

- the in-module `verify` items generate the contract from the authored
  `spec` block —
  a mutable-reference parameter binds its entry value (`old(x)`) and its
  exit value (bare `x`) — and proves it by the scripted symbolic
  execution;
- a hand-written `FunctionContract` for `replace` pins the tactic-level
  interface (`leaner_cases`-free entry, explicit facts to `leaner_wp`).
-/

open LeanerIR LeanerIR.Validation LeanerIR.Proofs

leaner module 0x99::refs where
  public fun replace (target : &mut u64, replacement : u64) -> u64 := do
    *target := replacement
    *target
  spec replace where
    ensures core.prim.equal(result, replacement);
    ensures core.prim.equal(target, replacement);
  verify replace
  public fun swap_in (cell : &mut u64, fresh : u64) -> u64 := do
    let previous := *cell
    *cell := fresh
    previous
  spec swap_in where
    ensures core.prim.equal(result, old(cell));
    ensures core.prim.equal(cell, fresh);
  verify swap_in
  public fun observe (source : &u64) -> u64 :=
    *source
  spec observe where
    ensures core.prim.equal(result, source);
  verify observe
  -- A branch in statement position: the condition runs once and the
  -- boolean it produced selects the arm, which is the shape the shallow
  -- denotation covers.
  public fun raise (slot : &mut u64, flag : Bool) -> Unit := do
    if flag then *slot := 1
  spec raise where
    ensures flag ==> slot == 1
    aborts_if false
  verify raise

