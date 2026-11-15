-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- This test asserts what the Given route does; it selects the route.
set_option verex.givenRoute true


open Lean Elab Command VerexSL.Verify

/-! Generic vector membership uses one search witness after certification and
its full-array bound have reached the same canonical representation. -/
verex module 0x42::given_vector_search where
  pragma verify = false

  fun ensure_present {E has Drop}(items : &mut Vector<E>, value : E) -> Unit :=
    if !core.prim.containsVector(*items, *&value) then
      *items := core.prim.pushVector(*items, value)

  spec ensure_present where
    ensures ∃ (element in items), element == value

  fun unchanged {E has Drop}(items : &mut Vector<E>, value : E) -> Unit := ()

  spec unchanged where
    ensures ∃ (element in items), element == value

verify 0x42::given_vector_search::ensure_present

run_cmd do
  let env ← getEnv
  let base := `«0x42».given_vector_search.ensure_present
  unless (givenProofArtifacts.getState env).find? base == some (base ++ `givenTypedVerified) do
    throwError "generic vector membership did not use the Given proof path"

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
    text.startsWith "the specification clause " && text.endsWith " is not established"
  let hasSummary := errorTexts.any (·.startsWith summaryPrefix)
  let hasClause := errorTexts.any fun text => isSemanticClause text && text.contains expectedClause
  let allExpected := errorTexts.all fun text =>
    text.startsWith summaryPrefix || isSemanticClause text
  unless !errorTexts.isEmpty && hasSummary && hasClause && allExpected do
    throwError "expected unchanged-vector membership to fail on `{expectedClause}`, got: {errorTexts}"

-- An unchanged vector need not contain the requested element.
run_cmd do
  assertGivenRejected `«0x42».given_vector_search.unchanged
    "element == value"
    (← `(command| verify 0x42::given_vector_search::unchanged))
