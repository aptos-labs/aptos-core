-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex module 0x42::given_failure_diagnostics where
  pragma verify = false
  opaque spec fun permitted {T}(value : u64) : Bool
  struct Tagged {T} has Copy, Drop, Store where
    value : u64
  spec Tagged where
    invariant permitted::<T>(value)
  public fun wrong_type {T}(t : &Tagged<T>) -> u64 := t.value
  spec wrong_type where
    aborts_if false
    ensures permitted::<Bool>(result)

/--
error: the specification clause `ensures permitted::<Bool>(result)` is not established
---
error: verex verification failed: the automatic verification of `wrong_type` failed; provide a proof in the module:
  verify wrong_type by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::given_failure_diagnostics::wrong_type

-- Rejection must not publish either proof artifact.
run_cmd do
  let env ← Lean.getEnv
  for suffix in [`givenTypedVerified, `typedVerified] do
    if env.contains (`«0x42».given_failure_diagnostics.wrong_type ++ suffix) then
      throwError "a rejected given proof was published"

-- Successful backtracking must discard an authored branch's diagnostics,
-- while an error that survives a completed tactic still rejects publication.
verex module 0x42::given_recovery_diagnostics where
  pragma verify = false
  fun recovered {T}(value : T) -> T := value
  spec recovered where
    ensures result == value
  fun reported {T}(value : T) -> T := value
  spec reported where
    ensures result == value

verify 0x42::given_recovery_diagnostics::recovered by lean =>
  first
  | (run_tac Lean.logError "discarded authored diagnostic"; fail "backtrack")
  | (verex_obligations; all_goals verex_decide)

/--
error: surviving authored diagnostic
---
error: verex verification failed: the proof of `reported` does not establish its specification
-/
#guard_msgs in
verify 0x42::given_recovery_diagnostics::reported by lean =>
  verex_obligations
  all_goals verex_decide
  run_tac Lean.logError "surviving authored diagnostic"

run_cmd do
  let env ← Lean.getEnv
  unless env.contains `«0x42».given_recovery_diagnostics.recovered.verified do
    throwError "recovered authored Given proof did not publish its verified artifact"
  for suffix in [`givenTypedVerified, `typedVerified, `verified] do
    if env.contains (`«0x42».given_recovery_diagnostics.reported ++ suffix) then
      throwError "a Given proof with a surviving error was published"

-- Check structured metadata as well as text. A budget report is not an
-- ordinary undecided result, and a setup failure has no reached clause.
open Lean Elab Command in
elab "check_given_budget " amount:num stage:ident " in " command:command : command => do
  let before := (← get).messages
  try
    try elabCommand command catch failure => logException failure
    let messages := (← get).messages.toArray
    let messages := messages.extract before.toArray.size messages.size
    let errors := messages.filter (·.severity == .error)
    unless errors.any (fun message => message.data.hasTag (· == `runtime.maxHeartbeats)) do
      throwError "Given budget failure lost its structured heartbeat tag"
    let texts ← errors.mapM (fun message => message.data.toString)
    let text := "\n".intercalate texts.toList
    if stage.getId == `authored then
      unless text.contains "does not establish its specification" &&
          text.contains s!"maximum number of heartbeats ({amount.getNat})" do
        throwError "Given authored failure lost its detailed cause or summary: {text}"
      return
    unless text.contains s!"exceeded its budget of {amount.getNat} maxHeartbeats" do
      throwError "Given attempt did not report its actual budget: {text}"
    if text.contains "(deterministic) timeout" then
      throwError "Given automatic attempt leaked its raw heartbeat exception"
    if stage.getId == `before_walk then
      if text.contains "after deciding" || text.contains "specification clause" ||
          text.contains "ensures_1" then
        throwError "Given setup failure invented or inherited a reached obligation: {text}"
    else if stage.getId == `during_walk then
      unless text.contains "after deciding" && text.contains "(stage " do
        throwError "Given traversal failure lost its real position: {text}"
    else if stage.getId == `after_leaf then
      unless text.contains "specification clause" && text.contains "ensures_1" do
        throwError "Given attempt discarded an existing clause diagnostic: {text}"
    else
      throwError "unknown budget test stage"
  finally
    modify fun state => { state with messages := before }

verex module 0x42::given_setup_budget where
  pragma verify = false
  fun identity {T}(value : T) -> T := value
  spec identity where
    aborts_if false
    ensures result == value

check_given_budget 1 before_walk in
set_option verex.verifyHeartbeats 1 in
verify 0x42::given_setup_budget::identity

check_given_budget 1 authored in
set_option verex.verifyHeartbeats 1 in
verify 0x42::given_setup_budget::identity by lean =>
  verex_obligations
  all_goals verex_decide

check_given_budget 1000 after_leaf in
set_option verex.verifyHeartbeats 1000 in
verify 0x42::given_failure_diagnostics::wrong_type

run_cmd do
  let env ← Lean.getEnv
  for base in [`«0x42».given_setup_budget.identity,
      `«0x42».given_failure_diagnostics.wrong_type] do
    for suffix in [`givenTypedVerified, `typedVerified, `verified] do
      if env.contains (base ++ suffix) then
        throwError "a budget-exhausted Given proof was published"

-- A native call makes the walk itself exceed the target cap. Preserve its
-- actual stop position as well as any earlier leaf diagnostic.
verex module 0x42::given_walk_budget where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  opaque spec fun permitted {T}(value : u64) : Bool
  struct Tagged {T} has Copy, Drop, Store where
    value : u64
  spec Tagged where
    invariant permitted::<T>(value)
  public fun wrong_type {T}(t : &Table<u64, Tagged<T> >, k : u64) -> u64 := borrow(t, k).value
  spec wrong_type where
    requires has(t, k)
    aborts_if false
    ensures permitted::<Bool>(result)

check_given_budget 5000 during_walk in
set_option verex.verifyHeartbeats 5000 in
verify 0x42::given_walk_budget::wrong_type

run_cmd do
  let env ← Lean.getEnv
  for suffix in [`givenTypedVerified, `typedVerified, `verified] do
    if env.contains (`«0x42».given_walk_budget.wrong_type ++ suffix) then
      throwError "a Given proof interrupted during traversal was published"

-- Even directly after a walk timeout, a new target's setup has no inherited
-- position or failed leaf. This also covers repeated attempts in one module.
check_given_budget 1 before_walk in
set_option verex.verifyHeartbeats 1 in
verify 0x42::given_setup_budget::identity


-- Exercise the pinned MessageLog API's real flush operation before and inside
-- an attempt. The cumulative view must retain new errors after they are flushed,
-- while pending-message replacement must leave the reported prefix untouched.
open Lean Elab Command VerexSL VerexSL.Contract VerexSL.Verify VerexIR.Proofs.Denote in
elab "check_given_flush " kind:ident : command => do
  let saved ← get
  try
    logInfo "prior reported diagnostic"
    modify fun state => { state with messages := state.messages.markAllReported }
    logError "prior pending diagnostic"
    let before := (← get).messages
    let some unit := registeredUnit? (← getEnv) `«0x42».given_setup_budget
      | throwError "missing flush test unit"
    let some (namespaceIndex, _, functionIndex, _) := findFunction? unit "identity"
      | throwError "missing flush test function"
    let handle : VerexIR.FunctionHandle := ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
    let .ok compiled := compileFunction unit handle
      | throwError "flush test source compilation failed"
    let base := `given_flush ++ kind.getId
    let timedOut := kind.getId != `error
    let budget := verex.verifyHeartbeats.get (← getOptions)
    let script ← if kind.getId == `before then
        `(tactic| run_tac Lean.Core.throwMaxHeartbeat `flush_test `maxHeartbeats 25000)
      else if kind.getId == `timeout then
        `(tactic| run_tac do
          Lean.logError (.tagged `runtime.maxHeartbeats "flushed raw timeout")
          Lean.Core.setMessageLog (← Lean.Core.getMessageLog).markAllReported
          (Lean.Elab.throwAbortTerm : Lean.Elab.Tactic.TacticM Unit))
      else
        `(tactic| (run_tac do
                     Lean.logError "flushed surviving error"
                     Lean.Core.setMessageLog (← Lean.Core.getMessageLog).markAllReported
                   verex_solve_goals [] with []))
    try
      discard <| declareGivenFunction unit handle compiled base
        (some fun _ _ => do pure (← `(tactic| skip), script))
    catch failure => logException failure
    let messages := (← get).messages
    let allMessages := messages.reportedPlusUnreported.toArray
    let texts ← allMessages.mapM (fun message => message.data.toString)
    for marker in ["prior reported diagnostic", "prior pending diagnostic"] do
      unless texts.countP (· == marker) == 1 do
        throwError "flush changed or duplicated an earlier diagnostic: {marker}"
    let reportedTexts ← messages.reported.toArray.mapM (fun message => message.data.toString)
    let priorReportedTexts ← before.reported.toArray.mapM (fun message => message.data.toString)
    unless reportedTexts[:priorReportedTexts.size].toArray == priorReportedTexts do
      throwError "Given attempt changed already reported diagnostics"
    if timedOut then
      unless texts.any (·.contains s!"exceeded its budget of {budget} maxHeartbeats") do
        throwError "flush hid the actual heartbeat failure: {texts}"
      if kind.getId == `timeout then
        unless reportedTexts.contains "flushed raw timeout" do
          throwError "Given attempt retracted an already reported timeout"
    else
      unless texts.contains "flushed surviving error" &&
          texts.any (·.contains "automatic verification") do
        throwError "flush hid a surviving proof error: {texts}"
    for suffix in [`givenTypedVerified, `typedVerified, `verified] do
      if (← getEnv).contains (base ++ suffix) then
        throwError "a Given proof with a flushed failure was published"
  finally
    modify fun state => { state with messages := saved.messages }

check_given_flush before
check_given_flush timeout
check_given_flush error
