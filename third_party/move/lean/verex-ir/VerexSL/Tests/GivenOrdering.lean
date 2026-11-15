-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- This test asserts what the Given route does; it selects the route.
set_option verex.givenRoute true


/-! A named ordering specification observes plain keys from a generic ADT's
invariant. Given clause generation must choose the plain ordering operation
without requiring a certificate for a key inside that clause. The resulting
proof keeps the shared nominal declaration table and rejects unsorted packing. -/
open Lean Meta Elab Command VerexSL VerexSL.Verify

verex module 0x42::given_ordering where
  pragma verify = false

  struct Entry {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    key : K
    value : V

  struct Map {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    entries : Vector<Entry<K, V> >

  spec Map where
    invariant sorted_keys(entries)

  -- Keys strictly increase.
  spec fun sorted_keys {K has Copy, Drop, Store} {V has Copy, Drop, Store}(entries : Vector<Entry<K, V> >) : Bool :=
    forall (i : Int), forall (j : Int), 0 <= i && i < j && j < entries.length ==>
      core.prim.compare(entries[i].key, entries[j].key) < 0

  public fun empty {K has Copy, Drop, Store} {V has Copy, Drop, Store}() -> Map<K, V> :=
    new Map<K, V> { entries := vector<Entry<K, V> >[] }
  spec empty where
    ensures result.entries.length == 0
    aborts_if false


  public fun invalid_order {V has Copy, Drop, Store}(value : V) -> Map<u64, V> :=
    new Map<u64, V> { entries := vector<Entry<u64, V> >[
      new Entry<u64, V> { key := 2, value := value },
      new Entry<u64, V> { key := 1, value := value }] }
  spec invalid_order where
    aborts_if false


verify 0x42::given_ordering::empty

run_cmd do
  let env ← getEnv
  let base := `«0x42».given_ordering.empty
  let table := `«0x42».given_ordering.givenOwnDeclarations
  unless (givenProofArtifacts.getState env).find? base == some (base ++ `givenTypedVerified) do
    throwError "empty did not use the Given proof path"
  let some (.thmInfo theoremInfo) := env.find? (base ++ `givenTypedVerified)
    | throwError "missing kernel-checked Given theorem for empty"
  unless env.contains table && (theoremInfo.type.find? (·.isConstOf table)).isSome do
    throwError "the Given theorem lost its named declaration table"
  if (theoremInfo.type.find? fun e =>
      e.isAppOfArity ``VerexIR.Proofs.Denote.GivenFrame.withDeclarations 2 &&
      !(e.getArg! 1).getAppFn.isConstOf table).isSome then
    throwError "the Given theorem contains an inline declaration table"

private def forbiddenSubstrings : List String := [
  "exceeded its budget", "exceeded its heartbeat budget", "maxHeartbeats",
  "maximum number of heartbeats", "heartbeats", "stopped by the budget",
  "deterministic timeout", "(deterministic) timeout",
  "maximum recursion depth", "deep recursion",
  "unknown identifier", "unknown constant", "unknown function",
  "no Verex module declares", "no denotation for",
  "given verification is unsupported", "given verification concrete transfer failed",
  "pragma verify = manual",
  "type mismatch", "application type mismatch", "failed to synthesize",
  "unsolved goals", "no goals to be solved",
  "kernel", "PANIC at", "internal exception"
]

private def assertGivenRejected (base : Name) (expectedClause : String)
    (command : TSyntax `command) : CommandElabM Unit := do
  let saved ← get
  let priorCount := saved.messages.toList.length
  -- The quoted commands name existing source declarations, not hygienic locals.
  let command := Id.run <| command.raw.replaceM fun stx =>
    pure <| if stx.isIdent then some (mkIdentFrom stx stx.getId.eraseMacroScopes) else none
  let thrown ← try
    elabCommand command
    pure none
  catch error =>
    pure (some (error.isMaxHeartbeat || error.isMaxRecDepth, ← error.toMessageData.toString))
  let attempted ← get
  let curNs ← getCurrNamespace
  let fresh := attempted.messages.toList.drop priorCount
  let freshErrors := fresh.filter (·.severity == .error)
  let hasHeartbeatTag := fresh.any (·.data.hasTag (· == `runtime.maxHeartbeats))
  let freshTexts ← fresh.mapM fun message => message.data.toString
  let errorTexts := (← freshErrors.mapM fun message => message.data.toString) ++
    (thrown.map (·.2)).toList
  let noProof := !attempted.env.contains (base ++ `verified) &&
    !attempted.env.contains (curNs ++ base ++ `verified) &&
    !attempted.env.contains (base ++ `givenTypedVerified) &&
    !attempted.env.contains (curNs ++ base ++ `givenTypedVerified) &&
    !attempted.env.contains (base ++ `typedVerified) &&
    !attempted.env.contains (curNs ++ base ++ `typedVerified) &&
    !attempted.env.contains (base ++ `givenTransfer) &&
    !attempted.env.contains (curNs ++ base ++ `givenTransfer) &&
    ((givenProofArtifacts.getState attempted.env).find? base).isNone &&
    ((givenProofArtifacts.getState attempted.env).find? (curNs ++ base)).isNone
  modify fun _ => saved
  unless noProof do
    throwError "expected `{base}` not to publish any proof artifact"
  if thrown.any (·.1) || hasHeartbeatTag then
    throwError "expected semantic rejection for `{base}`, got heartbeat/recursion limit: {errorTexts}"
  if let some fallback := freshTexts.find? (·.contains "uses concrete verification") then
    throwError "expected Given rejection for `{base}`, got concrete fallback: {fallback}"
  if let some bad := errorTexts.find? (fun text => forbiddenSubstrings.any (text.contains ·)) then
    throwError "expected semantic rejection for `{base}`, got budget/infrastructure diagnostic: {bad}"
  let summaryPrefix := s!"verex verification failed: the automatic verification of `{base.getString!}` failed;"
  let isSemanticClause (text : String) : Bool :=
    (text.startsWith "the data invariant " && text.contains " does not hold ") ||
    (text.startsWith "the specification clause " && text.endsWith " is not established")
  let hasSummary := errorTexts.any (·.startsWith summaryPrefix)
  let hasClause := errorTexts.any fun text => isSemanticClause text && text.contains expectedClause
  let allExpected := errorTexts.all fun text =>
    text.startsWith summaryPrefix || isSemanticClause text
  unless !errorTexts.isEmpty && hasSummary && hasClause && allExpected do
    throwError "expected the unsorted Given construction to fail on `{expectedClause}`, got: {errorTexts}"

run_cmd do
  assertGivenRejected `«0x42».given_ordering.invalid_order
    "invariant sorted_keys(entries)"
    (← `(command| verify 0x42::given_ordering::invalid_order))
