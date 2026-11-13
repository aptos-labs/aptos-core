-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A common proof case is scheduled once before its source clauses
split. The same owners cover both cases; clause-local binders stay local. -/

set_option Elab.async false

open Lean Elab Command in
elab "#guard_shared_cases " count:num " owners " owners:num " in " command:command : command => do
  let environment ← getEnv
  let (output, ()) ← IO.FS.withIsolatedStreams (isolateStderr := false) <|
    elabCommand (← `(command|
      set_option verex.sharedContinuations true in
      set_option verex.solverProfile true in $command))
  setEnv environment
  unless (output.splitOn "\n").any (·.startsWith s!"  automatic owners: {owners.getNat},") do
    throwError "shared cases changed the number of automatic owners"
  let lines := (output.splitOn "\n").filter fun line =>
    line.startsWith "  stage common conditional " || line.startsWith "  stage common variant " ||
      line.startsWith "  stage common entry "
  if count.getNat == 0 then
    unless lines.isEmpty do throwError "an unshared conditional was lifted: {lines}"
  else
    unless lines.length == 1 && lines.all (fun line =>
        line.startsWith s!"  stage common conditional ×{count.getNat} " ||
        line.startsWith s!"  stage common variant ×{count.getNat} " ||
        line.startsWith s!"  stage common entry ×{count.getNat} ") do
      throwError "unexpected shared conditional count: {lines}"

open VerexIR.Proofs

/-! Sharing exists only where a sealed class of several arrivals is walked
once: each program below joins a diamond whose arrivals agree in shape, and
its clauses follow the join. A goal under no summary is the tree's own path,
with neither owners nor common cases. -/

verex module 0x42::shared_cases where
  pragma verify = false

  -- Both clauses of the completed suffix open their incoming relation once,
  -- with two owners covering both predecessor alternatives; the spec's own
  -- conditional stays in each leaf.
  fun shared(b : Bool, c : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    x
  spec shared where
    aborts_if false
    ensures 1 <= (if c then result else result + 1)
    ensures (if c then result else result + 1) <= 3

  -- Only one common opening precedes clause decisions. The remaining
  -- Booleans stay in each leaf, rather than forming all combinations up front.
  fun two_cases(b : Bool, c : Bool, d : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    x
  spec two_cases where
    aborts_if false
    ensures 4 <= result + (if c then 3 else 4) + (if d then 0 else 1)
    ensures result + (if c then 3 else 4) + (if d then 0 else 1) <= 7

  -- An existing assumption decides the spec's conditional without a split.
  fun known_case(b : Bool, c : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    x
  spec known_case where
    requires c == true
    aborts_if false
    ensures 1 <= (if c then result else result + 1)
    ensures (if c then result else result + 1) <= 2

  -- Logical subformulas inside one source occurrence retain one leaf owner.
  fun one_case(b : Bool, c : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    x
  spec one_case where
    aborts_if false
    ensures 1 <= (if c then result else result + 1) && (if c then result else result + 1) <= 3

#guard_shared_cases 1 owners 2 in
verify 0x42::shared_cases::shared
#guard_shared_cases 1 owners 2 in
verify 0x42::shared_cases::two_cases
#guard_shared_cases 1 owners 2 in
verify 0x42::shared_cases::known_case
#guard_shared_cases 0 owners 1 in
verify 0x42::shared_cases::one_case

-- Without a join there is no sharing: the group is the tree's own path.
#guard_shared_cases 0 owners 0 in
example (b : Bool) :
    Obligation "shared-cases" 0 1 (1 ≤ (if b then (1 : Int) else 2)) ∧
    Obligation "shared-cases" 2 3 ((if b then (1 : Int) else 2) ≤ 2) := by
  verex_solve_goals [] naming [("lower", "shared-cases", 0, 1), ("upper", "shared-cases", 2, 3)]

-- The completed suffix's clauses share one opening of their incoming
-- relation, with two owners covering both predecessor alternatives.
verex module 0x42::shared_entry_cases where
  pragma verify = false
  fun bounded(b : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    x
  spec bounded where
    aborts_if false
    ensures 1 <= result
    ensures result <= 2

  fun wrong(b : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    x
  spec wrong where
    aborts_if false
    ensures 1 <= result
    ensures result == 1

#guard_shared_cases 1 owners 2 in
verify 0x42::shared_entry_cases::bounded

/--
error: the specification clause `ensures result == 1` is not established
---
error: verex verification failed: the automatic verification of `wrong` failed; provide a proof in the module:
  verify wrong by lean =>
    verex_obligations
    verex_on
    | ensures_2 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
#guard_shared_cases 1 owners 2 in
verify 0x42::shared_entry_cases::wrong

-- An impossible ordering guard does not need the successful call's
-- quantified postconditions. They remain available if that attempt fails.
#guard_shared_cases 0 owners 0 in
example (map : VerexIR.SpecMap Int Int) (k : Int)
    (nonempty : 0 < VerexIR.SpecMap.size map)
    (outOfOrder : k ≤ VerexIR.SpecMap.keyAt map (VerexIR.SpecMap.size map - 1))
    (successful : ¬(0 < VerexIR.SpecMap.size map ∧
      VerexIR.SpecMap.intOrder.compare (VerexIR.SpecMap.keyAt map (VerexIR.SpecMap.size map - 1)) k ≠ .lt ∨
      VerexIR.SpecMap.size map < VerexIR.SpecMap.size map ∧
        VerexIR.SpecMap.intOrder.compare k (VerexIR.SpecMap.keyAt map (VerexIR.SpecMap.size map)) ≠ .lt))
    (_postcondition : ∀ i : Int, 0 ≤ i → i < VerexIR.SpecMap.size map →
      VerexIR.SpecMap.keyAt map i = VerexIR.SpecMap.keyAt map i) : False := by
  verex_solve_goals []

verex module 0x42::shared_case_errors where
  pragma verify = false
  fun wrong(b : Bool, c : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    x
  spec wrong where
    aborts_if false
    ensures (if c then result + 1 else result + 2) >= 2
    ensures (if c then result + 1 else result + 2) == 2

-- Both predecessors are required. The second clause fails on one of them,
-- while its valid sibling retains its own verdict and source identity.
/--
error: the specification clause `ensures (if c then result + 1 else result + 2) == 2` is not established
---
error: verex verification failed: the automatic verification of `wrong` failed; provide a proof in the module:
  verify wrong by lean =>
    verex_obligations
    verex_on
    | ensures_2 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
#guard_shared_cases 1 owners 2 in
verify 0x42::shared_case_errors::wrong
