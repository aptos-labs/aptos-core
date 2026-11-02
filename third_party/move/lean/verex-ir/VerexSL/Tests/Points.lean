-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Hooks at program points (`verex_obligations at …`, `after …`;
`doc/plan/proof-language.md`, "Points"): what a hook establishes there holds
on every path through the point, and one run of the hook proves it for all
of them, however the normal form distributes the paths after it. -/

namespace VerexSL.Tests.Points

-- Each hook runs once: after the operation before the branch on `flag`, at
-- the entry, after the call, and at the start of the loop's iteration.
/--
trace: after op 1
---
trace: entry
---
trace: after call
---
trace: loop step
-/
#guard_msgs in
verex module 0x42::points where
  fun pick(x : u64, flag : Bool) -> u64 := do
    let y := x + 1
    if flag then y else x
  spec pick where
    requires x < 100
    aborts_if false
    ensures result >= x

  verify pick by lean =>
    verex_obligations
      after op 1 =>
        have shared : x.val + 1 ≤ 100 := by omega
        trace "after op 1"
    verex_on
    | ensures_1 => verex_prepare values; all_goals (have _ := shared; omega)
    | _ => verex_solve

  fun inc(x : u64) -> u64 := x + 1
  spec inc where
    pragma opaque
    aborts_if x + 1 > 18446744073709551615
    ensures result == x + 1

  fun calls(x : u64, flag : Bool) -> u64 := do
    let y := inc(x)
    if flag then y else x
  spec calls where
    requires x < 100
    aborts_if false
    ensures result >= x

  verify calls by lean =>
    verex_obligations
      at entry =>
        have early : x.val ≤ 99 := by omega
        trace "entry"
      after call 1 of inc =>
        have called : inc_result.val = x.val + 1 := by
          simp only [VerexIR.Proofs.Obligation_eq] at *; omega
        trace "after call"
    verex_on
    | ensures_1 => verex_prepare values; all_goals (have _ := early; have _ := called; omega)
    | _ => verex_solve

  fun count(limit : u64) -> u64 := do
    let mut i := 0
    while i < limit do
      i := i + 1
    spec do
      invariant i <= limit
    i
  spec count where
    requires limit < 100
    ensures result == limit
    aborts_if false

  verify count by lean =>
    verex_obligations
      at loop 1 step =>
        have stepping : i.val ≤ limit.val := by assumption
        trace "loop step"
    verex_on
    | invariant_1_step => verex_prepare values; all_goals (have _ := stepping; omega)
    | _ => verex_solve

-- A hook at a point no path reaches is an error of the proof.
/--
error: no path reaches the point `after op 1`
---
error: verex verification failed: the proof of `unreached` does not establish its specification
-/
#guard_msgs in
verex module 0x42::points_unreached where
  fun unreached(x : u64) -> u64 := x
  spec unreached where
    ensures result == x

  verify unreached by lean =>
    verex_obligations
      after op 1 => skip
    all_goals verex_decide

-- An abort has no continuation to place facts after.
/-- error: an abort has no continuation: `after abort` names no point -/
#guard_msgs in
example : True := by
  verex_obligations
    after abort 1 => skip

end VerexSL.Tests.Points
