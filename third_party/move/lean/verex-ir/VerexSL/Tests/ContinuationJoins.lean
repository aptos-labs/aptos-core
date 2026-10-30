-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Shared continuations at imperative joins

The source fixtures expose a sequence of diamonds with one common suffix.
The rule examples keep correlated incoming states and abrupt destinations.
-/

set_option Elab.async false

-- Capture only profiling output. Verification diagnostics and the artifact
-- audit still run normally, so a failed verification cannot pass this guard.
open Lean Elab Command in
elab "#guard_continuations " expected:str " in " command:command : command => do
  let environment ← getEnv
  let (output, ()) ← IO.FS.withIsolatedStreams (isolateStderr := false) <|
    elabCommand (← `(command| set_option verex.solverProfile true in $command))
  setEnv environment
  let lines := (output.splitOn "\n").filter (·.startsWith "  continuations:")
  unless lines == ["  continuations: " ++ expected.getString] do
    throwError "unexpected continuation profile: {lines}"

open Lean Elab Command in
elab "#guard_joins " expected:str " in " command:command : command => do
  let environment ← getEnv
  let (output, ()) ← IO.FS.withIsolatedStreams (isolateStderr := false) <|
    elabCommand (← `(command|
      set_option verex.sharedContinuations true in
      set_option verex.solverProfile true in $command))
  setEnv environment
  let lines := (output.splitOn "\n").filter (·.startsWith "  joins:")
  unless lines == ["  joins: " ++ expected.getString] do
    throwError "unexpected shared join profile: {lines}"

open Lean Elab Command in
elab "#guard_preparation " owners:num " in " command:command : command => do
  let environment ← getEnv
  let (output, ()) ← IO.FS.withIsolatedStreams (isolateStderr := false) <|
    elabCommand (← `(command|
      set_option verex.sharedContinuations true in
      set_option verex.solverProfile true in $command))
  setEnv environment
  let lines := output.splitOn "\n"
  let count := owners.getNat
  unless lines.contains s!"  automatic owners: {count}, prepared {count}, exhausted 0" &&
      lines.any (·.startsWith "  stage automatic preparation ×1 ") do
    throwError "common preparation did not serve exactly {count} owners once"

open Lean Elab Command in
elab "#guard_owners " expected:str " in " command:command : command => do
  let environment ← getEnv
  let (output, ()) ← IO.FS.withIsolatedStreams (isolateStderr := false) <|
    elabCommand (← `(command|
      set_option verex.sharedContinuations true in
      set_option verex.solverProfile true in $command))
  setEnv environment
  let lines := (output.splitOn "\n").filter (·.startsWith "  automatic owners:")
  unless lines == ["  automatic owners: " ++ expected.getString] do
    throwError "unexpected automatic owners: {lines}"

open Lean Elab Command in
elab "#fresh_verification " "in " command:command : command => do
  let environment ← getEnv
  try elabCommand command finally setEnv environment

verex module 0x42::continuation_joins where
  -- The guards below must measure fresh proofs, not cached automatic ones.
  pragma verify = false

  fun one(b : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    x + 10
  spec one where
    aborts_if false
    ensures result == if b then 11 else 12

  fun two(b : Bool, c : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    if c then x := x + 3 else x := x + 4
    x + 10
  spec two where
    aborts_if false
    ensures result == (if b then 1 else 2) + (if c then 3 else 4) + 10

  fun three(b : Bool, c : Bool, d : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    if c then x := x + 3 else x := x + 4
    if d then x := x + 5 else x := x + 6
    x + 10
  spec three where
    aborts_if false
    ensures result == (if b then 1 else 2) + (if c then 3 else 4) +
      (if d then 5 else 6) + 10

  fun nested(b : Bool, c : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then
      if c then x := 1 else x := 2
    else
      x := 3
    x + 10
  spec nested where
    aborts_if false
    ensures result == if b then (if c then 11 else 12) else 13

  opaque fun increment(x : u64) -> u64
  spec increment where
    requires x < 10
    aborts_if false
    ensures result == x + 1

  fun call_after(b : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    increment(x)
  spec call_after where
    aborts_if false
    ensures result == if b then 2 else 3

  fun limits(x : u64) -> u64 := x
  spec limits where
    aborts_if false
    ensures result >= 0
    ensures result <= 18446744073709551615

  fun wrong(b : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    x + 10
  spec wrong where
    aborts_if false
    ensures result == 11

  fun early(b : Bool) -> u64 := do
    if b then return 7
    let mut x : u64 := 2
    x := x + 10
    x
  spec early where
    aborts_if false
    ensures result == if b then 7 else 12

  fun count(limit : u64) -> u64 := do
    let mut i : u64 := 0
    while i < limit do
      i := i + 1
    spec do
      invariant i <= limit
    i
  spec count where
    aborts_if false
    ensures result == limit

  fun diamonds(limit : u64, b : Bool) -> u64 := do
    let mut i : u64 := 0
    let mut x : u64 := 0
    while i < limit do
      if b then x := 1 else x := 2
      i := i + 1
    spec do
      invariant i <= limit
      invariant x <= 2
    i
  spec diamonds where
    aborts_if false
    ensures result == limit

  fun transfers(limit : u64, skip : Bool, quit : Bool) -> u64 := do
    let mut i : u64 := 0
    while i < limit do
      if skip then
        i := i + 1
        continue
      if quit then break
      i := i + 1
    spec do
      invariant i <= limit
    i
  spec transfers where
    aborts_if false
    ensures result <= limit

  fun checked(b : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    spec do
      assert x <= 2
    x + 10
  spec checked where
    aborts_if false
    ensures result == if b then 11 else 12

  fun wrong_checked(b : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    spec do
      assert x == 1
    x
  spec wrong_checked where
    aborts_if false
    ensures result == 1

  fun wrong_invariant(limit : u64, b : Bool) -> u64 := do
    let mut i : u64 := 0
    let mut x : u64 := 0
    while i < limit do
      if b then x := 1 else x := 2
      i := i + 1
    spec do
      invariant i <= limit
      invariant x <= 1
    i
  spec wrong_invariant where
    aborts_if false
    ensures result == limit

  fun unrolled_diamonds(n : u8, b : Bool) -> u8 := do
    let mut i : u8 := 0
    let mut x : u8 := 0
    while i < n do
      if b then x := 1 else x := 2
      i := i + 1
    x
  spec unrolled_diamonds where
    pragma unroll = 2
    requires n <= 2
    aborts_if false
    ensures result == if n == 0 then 0 else if b then 1 else 2

  fun unrolled_outrun(n : u8, b : Bool) -> u8 := do
    let mut i : u8 := 0
    let mut x : u8 := 0
    while i < n do
      if b then x := 1 else x := 2
      i := i + 1
    x
  spec unrolled_outrun where
    pragma unroll = 1
    requires n <= 3
    aborts_if false
    ensures result == if n == 0 then 0 else if b then 1 else 2

-- These are measurements of the current path traversal, not a target for
-- the join scheduler. Its implementation should reduce the dynamic folds
-- and arrivals while preserving the checked specifications.
#guard_continuations "branch folds 2, loop folds 0, arrivals 4, revisits 2, untracked 0" in
  verify 0x42::continuation_joins::one
#guard_continuations "branch folds 4, loop folds 0, arrivals 10, revisits 6, untracked 0" in
  verify 0x42::continuation_joins::two
#guard_continuations "branch folds 8, loop folds 0, arrivals 22, revisits 14, untracked 0" in
  verify 0x42::continuation_joins::three
#guard_continuations "branch folds 3, loop folds 1, arrivals 5, revisits 1, untracked 3" in
  verify 0x42::continuation_joins::count

-- One checked summary per source diamond, including consecutive joins.
-- Candidate folds with only one predecessor resume their concrete states.
-- Retaining present slots avoids extra candidate folds at subsequent reads.
#guard_joins "nodes 2, edges 3, summaries 1, proofs 1" in
  verify 0x42::continuation_joins::one
#guard_joins "nodes 3, edges 5, summaries 2, proofs 2" in
  verify 0x42::continuation_joins::two
#guard_joins "nodes 4, edges 7, summaries 3, proofs 3" in
  verify 0x42::continuation_joins::three
#guard_joins "nodes 0, edges 0, summaries 0, proofs 0" in
  verify 0x42::continuation_joins::early
set_option verex.sharedContinuations true in
verify 0x42::continuation_joins::nested
set_option verex.sharedContinuations true in
verify 0x42::continuation_joins::call_after

-- Both clauses use the same integer certificate, prepared once before
-- their proof destinations split. Each receives the shared debit.
#guard_preparation 2 in
  verify 0x42::continuation_joins::limits

-- An exhausted shared slice must still report both independent clauses.
/--
error: the specification clause `ensures result >= 0` is not established
---
error: the specification clause `ensures result <= 18446744073709551615` is not established
---
error: verex verification failed: the automatic verification of `limits` failed; provide a proof in the module:
  verify limits by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | ensures_2 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
set_option verex.sharedContinuations true in
set_option verex.leafHeartbeats 1 in
verify 0x42::continuation_joins::limits

-- Authored routing retains its existing paths; invariant loops use the checked loop rule.
#fresh_verification in
set_option verex.sharedContinuations true in
verify 0x42::continuation_joins::one by lean =>
  verex_obligations
  all_goals verex_solve
set_option verex.sharedContinuations true in
verify 0x42::continuation_joins::count

set_option verex.sharedContinuations true in
verify 0x42::continuation_joins::diamonds
set_option verex.sharedContinuations true in
verify 0x42::continuation_joins::transfers
set_option verex.sharedContinuations true in
verify 0x42::continuation_joins::checked

-- Explicit unrolling keeps the joins of each finite iteration. A bound
-- failure must drain both the ordinary queue and the retained join holes.
#guard_joins "nodes 8, edges 10, summaries 3, proofs 3" in
verify 0x42::continuation_joins::unrolled_diamonds

/--
error: the bound `pragma unroll = 1` of a loop is not established
---
error: verex verification failed: the automatic verification of `unrolled_outrun` failed; provide a proof in the module:
  verify unrolled_outrun by lean =>
    verex_obligations
    verex_on
    | unroll => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
#guard_joins "nodes 6, edges 8, summaries 2, proofs 0" in
verify 0x42::continuation_joins::unrolled_outrun

-- Zero keeps the existing unlimited-leaf meaning.
set_option verex.sharedContinuations true in
set_option verex.leafHeartbeats 0 in
verify 0x42::continuation_joins::one

/--
error: the specification clause `ensures result == 11` is not established
---
error: verex verification failed: the automatic verification of `wrong` failed; provide a proof in the module:
  verify wrong by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
#guard_joins "nodes 2, edges 3, summaries 1, proofs 0" in
verify 0x42::continuation_joins::wrong

-- The suffix may assume a cut only while its proof remains owed. Failed
-- assertions and invariant clauses must reach the verification artifact audit.
/--
error: the assertion `x == 1` does not hold
---
error: verex verification failed: the automatic verification of `wrong_checked` failed; provide a proof in the module:
  verify wrong_checked by lean =>
    verex_obligations
    verex_on
    | assert_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
set_option verex.sharedContinuations true in
verify 0x42::continuation_joins::wrong_checked

-- The first invariant is valid; only the second may be reported after the
-- joined row's decoding wrappers have been routed to their source clauses.
/--
error: the loop invariant `x <= 1` is not preserved by an iteration
---
error: verex verification failed: the automatic verification of `wrong_invariant` failed; provide a proof in the module:
  verify wrong_invariant by lean =>
    verex_obligations
    verex_on
    | invariant_2_step => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
set_option verex.sharedContinuations true in
verify 0x42::continuation_joins::wrong_invariant

-- Requested map observations are derived for the clause group, then reused
-- by both kernel-checked leaves without discarding their source markers.
open VerexIR VerexIR.Proofs in
#guard_preparation 2 in
example (m : SpecMap Int Int) (present : SpecMap.hasKey m 0 = true) :
    Obligation "shared-map" 0 1 (SpecMap.rank m 0 < SpecMap.size m) ∧
    Obligation "shared-map" 2 3 (SpecMap.keyAt m (SpecMap.rank m 0) = 0) := by
  verex_solve_goals [] naming
    [("rank", "shared-map", 0, 1), ("key", "shared-map", 2, 3)]

-- Refuting several abort alternatives is still one logical occurrence.
#guard_owners "1, prepared 0, exhausted 0" in
example (n : Int) (above : 10 ≤ n) : ¬(n ≤ 1 ∨ n ≤ 2 ∨ n ≤ 3) := by
  verex_solve_goals []

-- A quantified clause keeps one owner through its binder and premise.
open VerexIR.Proofs in
#guard_preparation 2 in
example (n : Int) :
    Obligation "shared-quantified" 0 1 (∀ m : Int, n < m → n < m + 1) ∧
    Obligation "shared-quantified" 2 3 (n < n + 1) := by
  verex_solve_goals [] naming
    [("quantified", "shared-quantified", 0, 1), ("next", "shared-quantified", 2, 3)]

-- A structural result binder precedes two independent source clauses.
open VerexIR.Proofs in
#guard_preparation 2 in
example : ∀ n : Int,
    Obligation "shared-result" 0 1 (n < n + 1) ∧
    Obligation "shared-result" 2 3 (n < n + 2) := by
  verex_solve_goals [] naming
    [("next", "shared-result", 0, 1), ("later", "shared-result", 2, 3)]

-- Decoding an abstract slot must not merge the source clauses it wraps.
open VerexIR.Proofs in
#guard_preparation 2 in
example (slot : Option Int) (n : Int) (present : slot = some n) :
    slot.elim False (fun value =>
      Obligation "shared-slot" 0 1 (value < value + 1) ∧
      Obligation "shared-slot" 2 3 (value < value + 2)) := by
  verex_solve_goals [] naming
    [("next", "shared-slot", 0, 1), ("later", "shared-slot", 2, 3)]

-- Logical conjunctions and quantifiers inside one source occurrence keep
-- one allowance. An unmarked quantified goal also has one structural owner.
open VerexIR.Proofs in
#guard_owners "1, prepared 0, exhausted 0" in
example : Obligation "shared-atomic" 0 1
    (∀ n : Int, (n < n + 1 ∧ n < n + 2) ∧ n < n + 3) := by
  verex_solve_goals [] naming [("atomic", "shared-atomic", 0, 1)]

#guard_owners "1, prepared 0, exhausted 0" in
example : ∀ n : Int, n < n + 1 ∧ n < n + 2 := by
  verex_solve_goals []

section JoinRule

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

variable {unit : Validation.ValidatedUnit} [Skolems unit]
variable {ρ : ResultShape} {Γ : NRow}

-- The continuation is checked under a disjunction that preserves each
-- branch's value and memory, with no assumption that both branches hold.
example (b : Bool) (env : HEnv Γ) (initial : Memory unit) :
    wp (Flow.bind
      (if b then Spec.pure (.value (1 : Nat) env) else Spec.pure (.value 2 env))
      (fun n locals => Spec.pure (.value (n + 10) locals)))
      (fun (flow : Flow ρ Γ Nat) final => match flow with
        | .value n locals => locals = env ∧ final = initial ∧
            ((b = true ∧ n = 11) ∨ (b = false ∧ n = 12))
        | .return_ _ _ | .break_ _ _ | .continue_ _ _ => False)
      (fun _ => False) initial := by
  apply wp_flowBind_join (entry := fun n locals final =>
    locals = env ∧ final = initial ∧ ((b = true ∧ n = 1) ∨ (b = false ∧ n = 2)))
  · cases b <;> simp [Flow.joinPost, wp_pure]
  · intro n locals final reached
    rcases reached with ⟨rfl, rfl, ⟨branch, rfl⟩ | ⟨branch, rfl⟩⟩ <;>
      simp [wp_pure, branch]

-- Memory at the join is the selected branch's outgoing memory. It cannot
-- be replaced by the initial memory or taken from the other branch.
example (b : Bool) (env : HEnv Γ) (initial left right : Memory unit) :
    wp (Flow.bind
      (if b then Spec.bind (Spec.set left) (fun _ => Spec.pure (.value (1 : Nat) env))
       else Spec.bind (Spec.set right) (fun _ => Spec.pure (.value 2 env)))
      (fun n locals => Spec.pure (.value (n + 10) locals)))
      (fun (flow : Flow ρ Γ Nat) final => match flow with
        | .value n _ => (b = true ∧ n = 11 ∧ final = left) ∨
            (b = false ∧ n = 12 ∧ final = right)
        | .return_ _ _ | .break_ _ _ | .continue_ _ _ => False)
      (fun _ => False) initial := by
  apply wp_flowBind_join (entry := fun n _ final =>
    (b = true ∧ n = 1 ∧ final = left) ∨ (b = false ∧ n = 2 ∧ final = right))
  · cases b <;> simp [wp, Spec.bind, Spec.set, Spec.pure, Flow.joinPost]
  · intro n locals final reached
    rcases reached with ⟨branch, rfl, rfl⟩ | ⟨branch, rfl, rfl⟩ <;>
      simp [wp_pure, branch]

-- A return never has to establish the normal join predicate, even when
-- that predicate is false and the continuation would not verify.
example (result : ρ.carrier) (env : HEnv Γ) (state : Memory unit)
    (next : Nat → HEnv Γ → Comp unit (Flow ρ Γ Nat)) :
    wp (Flow.bind (Spec.pure (.return_ result env)) next)
      (fun flow final => flow = .return_ result env ∧ final = state)
      (fun _ => False) state := by
  apply wp_flowBind_join (entry := fun _ _ _ => False)
  · simp [Flow.joinPost, wp_pure]
  · intro _ _ _ impossible
    exact False.elim impossible

-- A break and a continue also bypass the normal continuation, retaining
-- the nesting depth that identifies their destination.
example (nest : Nat) (env : HEnv Γ) (state : Memory unit)
    (next : Nat → HEnv Γ → Comp unit (Flow ρ Γ Nat)) :
    wp (Flow.bind (Spec.pure (.break_ nest env)) next)
      (fun flow final => flow = .break_ nest env ∧ final = state)
      (fun _ => False) state := by
  apply wp_flowBind_join (entry := fun _ _ _ => False)
  · simp [Flow.joinPost, wp_pure]
  · intro _ _ _ impossible
    exact False.elim impossible

example (nest : Nat) (env : HEnv Γ) (state : Memory unit)
    (next : Nat → HEnv Γ → Comp unit (Flow ρ Γ Nat)) :
    wp (Flow.bind (Spec.pure (.continue_ nest env)) next)
      (fun flow final => flow = .continue_ nest env ∧ final = state)
      (fun _ => False) state := by
  apply wp_flowBind_join (entry := fun _ _ _ => False)
  · simp [Flow.joinPost, wp_pure]
  · intro _ _ _ impossible
    exact False.elim impossible

example (error : Failure) (state : Memory unit)
    (next : Nat → HEnv Γ → Comp unit (Flow ρ Γ Nat)) :
    wp (Flow.bind (Spec.abort error) next)
      (fun _ _ => False) (fun failed => failed = error) state := by
  apply wp_flowBind_join (entry := fun _ _ _ => False)
  · simp [wp_abort]
  · intro _ _ _ impossible
    exact False.elim impossible

-- An incorrect incoming-state predicate leaves an actual proof obligation.
example (env : HEnv Γ) (state : Memory unit) :
    ¬ wp (Spec.pure (.value (1 : Nat) env))
      (Flow.joinPost (fun n _ _ => n = 2)
        (fun (_ : Flow ρ Γ Nat) _ => True)) (fun (_ : Failure) => False) state := by
  simp [Flow.joinPost, wp_pure]

end JoinRule
