-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open Lean Meta Elab Tactic VerexIR VerexSL

namespace LoanDeathControls

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
  "no goals to be solved",
  "kernel", "PANIC at", "internal exception"
]

/-- The error diagnostics of a `verify` command that must be rejected, with the
state before it restored. No proof artifact may have been published, and
budget/timeout/infrastructure errors are refused. -/
def rejectedDiagnostics (base : Name) (command : TSyntax `command) :
    Command.CommandElabM (List String) := do
  let saved ← get
  let priorCount := saved.messages.toList.length
  let command := Id.run <| command.raw.replaceM fun stx =>
    pure <| if stx.isIdent then some (mkIdentFrom stx stx.getId.eraseMacroScopes) else none
  let thrown ← try
    Command.elabCommand command
    pure none
  catch error =>
    pure (some (error.isMaxHeartbeat || error.isMaxRecDepth, ← error.toMessageData.toString))
  let attempted ← get
  let curNs ← getCurrNamespace
  let fresh := attempted.messages.toList.drop priorCount
  let freshErrors := fresh.filter (·.severity == .error)
  let hasHeartbeatTag := fresh.any (·.data.hasTag (· == `runtime.maxHeartbeats))
  let diagnostics := (← freshErrors.mapM fun message => message.data.toString) ++
    (thrown.map (·.2)).toList
  let published := attempted.env.contains (base ++ `typedVerified) ||
    attempted.env.contains (curNs ++ base ++ `typedVerified) ||
    attempted.env.contains (base ++ `verified) ||
    attempted.env.contains (curNs ++ base ++ `verified) ||
    attempted.env.contains (base ++ `givenTypedVerified) ||
    attempted.env.contains (curNs ++ base ++ `givenTypedVerified) ||
    attempted.env.contains (base ++ `givenTransfer) ||
    attempted.env.contains (curNs ++ base ++ `givenTransfer) ||
    ((VerexSL.Verify.givenProofArtifacts.getState attempted.env).find? base).isSome ||
    ((VerexSL.Verify.givenProofArtifacts.getState attempted.env).find? (curNs ++ base)).isSome
  modify fun _ => saved
  if published then throwError "rejected proof published {base}"
  if thrown.any (·.1) || hasHeartbeatTag then
    throwError "{base}: verification hit heartbeat/recursion budget: {diagnostics}"
  if let some bad := diagnostics.find? (fun text => forbiddenSubstrings.any (text.contains ·)) then
    throwError "{base}: unexpected budget or infrastructure diagnostic: {bad}"
  let fn := base.getString!
  let isSummary (text : String) : Bool :=
    (text.startsWith "verex verification failed: the automatic verification of `" &&
      text.contains s!"{fn}` failed;") ||
    (text.startsWith "verex verification failed: the proof of `" &&
      text.endsWith s!"{fn}` does not establish its specification")
  unless !diagnostics.isEmpty && diagnostics.any isSummary &&
      diagnostics.any (fun text => !(text.startsWith "verex verification failed: ")) do
    throwError "expected a non-budget semantic rejection of {base}; {diagnostics}"
  return diagnostics

/-- The automatic verification is rejected, and the cuts it names are exactly
the expected ones: a clause and where it is owed. `clauses` are the source
clauses the report may also name as unestablished; `none` allows any, as a
mixed report has them. -/
def expectCuts (base : Name) (expected : List (String × String))
    (command : TSyntax `command) (clauses : Option (List String) := none) :
    Command.CommandElabM Unit := do
  unless !expected.isEmpty do
    throwError "{base}: expected cuts list must be non-empty"
  let diagnostics ← rejectedDiagnostics base command
  let fn := base.getString!
  unless diagnostics.any fun text =>
      text.startsWith "verex verification failed: the automatic verification of `" &&
      text.contains s!"{fn}` failed;" do
    throwError "{base}: expected automatic verification failure, found {diagnostics}"
  if let some other := diagnostics.find? fun text =>
      !(text.startsWith "the data invariant " ||
        text.startsWith "the specification clause " ||
        text.startsWith "`no panic" ||
        text.startsWith "`no abort" ||
        text.startsWith "verex verification failed: the automatic verification of `") then
    throwError "{base}: unexpected diagnostic during cut check: {other}"
  let cuts := diagnostics.filter (·.startsWith "the data invariant ")
  let wanted := expected.map fun (clause, boundary) =>
    s!"the data invariant `{clause}` does not hold {boundary}"
  unless cuts == wanted do
    throwError "{base}: expected the cuts {wanted}, found {diagnostics}"
  if let some clauses := clauses then
    let named := diagnostics.filter (·.startsWith "the specification clause ")
    let wantedClauses := clauses.map fun clause =>
      s!"the specification clause `{clause}` is not established"
    unless named == wantedClauses do
      throwError "{base}: expected the clauses {wantedClauses}, found {diagnostics}"

/-- The automatic verification is rejected while the report names the clause
as one the proof does not establish, not as a cut of its own: the obligation
is reported at the function's exit or at a leaf below one, where the
reporting is by source clause. -/
def expectUnestablished (base : Name) (expected : List String)
    (command : TSyntax `command) : Command.CommandElabM Unit := do
  unless !expected.isEmpty do
    throwError "{base}: expected unestablished clauses list must be non-empty"
  let diagnostics ← rejectedDiagnostics base command
  let fn := base.getString!
  unless diagnostics.any fun text =>
      text.startsWith "verex verification failed: the automatic verification of `" &&
      text.contains s!"{fn}` failed;" do
    throwError "{base}: expected automatic verification failure, found {diagnostics}"
  if let some other := diagnostics.find? fun text =>
      !(text.startsWith "the specification clause " ||
        text.startsWith "`no panic" ||
        text.startsWith "`no abort" ||
        text.startsWith "verex verification failed: the automatic verification of `") then
    throwError "{base}: unexpected diagnostic during clause check: {other}"
  let named := diagnostics.filter (·.startsWith "the specification clause ")
  let wanted := expected.map fun clause =>
    s!"the specification clause `{clause}` is not established"
  unless named == wanted do
    throwError "{base}: expected the clauses {wanted}, found {diagnostics}"

/-- The authored contradiction script is rejected, and every goal it fails on
is one its arithmetic could not close: no other tactic, name or compilation
failure stands in for the cut. -/
def expectScriptFails (base : Name) (command : TSyntax `command) :
    Command.CommandElabM Unit := do
  let diagnostics ← rejectedDiagnostics base command
  let fn := base.getString!
  let isProofSummary (text : String) : Bool :=
    text.startsWith "verex verification failed: the proof of `" &&
    text.endsWith s!"{fn}` does not establish its specification"
  unless diagnostics.any (·.startsWith "omega could not prove the goal") &&
      diagnostics.any isProofSummary do
    throwError "{base}: the script did not fail on its arithmetic; {diagnostics}"
  if let some other := diagnostics.find? fun message =>
      !(message.startsWith "omega could not prove the goal" || isProofSummary message) then
    throwError "{base}: the script failed elsewhere: {other}"

def mutationEnd := "where a mutation ends"
def exit := "where a mutable parameter's loan ends, at the function's exit"
def handOver := "where a mutable reference is passed to a call"
def construction := "where a value is constructed"

end LoanDeathControls
