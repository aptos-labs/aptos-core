-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Support for the shape matrix

The matrix runs one small function per mechanism and type shape. A cell this
tree does not establish stays in its file, in the `Open cells` section at the
end, under `expectOpen`: the cell's `verify` must be rejected by the
verification itself, with the diagnostic the cell states first, which keeps
the suite green while the cell is open and turns it red the moment the cell
is established, where the cell moves up to the positive controls. The check
pins the first diagnostic and the categories the others may be of, not the
whole list. An open cell is a sentinel of a missing capability, not a
verified target and not coverage. Each open cell
is a command of its own, so a file reports every cell that is established,
and `README.md` beside this file lists the open cells.

A rejection counts only as what it is: every diagnostic of the attempt must
be one the verification of that target reports. A budget, a kernel or
elaboration error, an unknown name or any other message fails the check, as
does a published proof artifact.
-/

open Lean Elab Command

namespace VerexSL.Tests.Shapes

/-- What the attempt of a `verify` command left: its error diagnostics in
order, whether a proof artifact of the target was published, and whether it
ran out of a budget. The state before the command is restored. -/
private def attempt (base : Name) (command : TSyntax `command) :
    CommandElabM (List String × Bool × Bool) := do
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
  let budget := thrown.any (·.1) || fresh.any (·.data.hasTag (· == `runtime.maxHeartbeats))
  let diagnostics := (← (fresh.filter (·.severity == .error)).mapM (·.data.toString)) ++
    (thrown.map (·.2)).toList
  let published := [`verified, `typedVerified, `givenTypedVerified, `givenTransfer].any
    fun suffix => attempted.env.contains (base ++ suffix) ||
      attempted.env.contains (curNs ++ base ++ suffix)
  modify fun _ => saved
  return (diagnostics, published, budget)

/-- The report that closes a rejected automatic verification of `base`. -/
private def isSummary (base : Name) (text : String) : Bool :=
  text.startsWith "verex verification failed: the automatic verification of `" &&
    text.contains s!"{base.getString!}` failed;"

/-- Whether a diagnostic is one a rejected verification reports of its own
target: an unestablished clause, a data-invariant cut, a continuation or a
precondition of a call, a residual obligation, or the closing report. -/
private def isVerificationDiagnostic (base : Name) (text : String) : Bool :=
  isSummary base text ||
    ["the specification clause ", "the data invariant ", "the continuation after ",
      "the precondition of ", "verification failed with a residual obligation"].any
      (text.startsWith ·)

/-- The diagnostics of a `verify` command the verification rejects, all of
them the verification's own; anything else is an error of the check. -/
private def rejection (base : Name) (command : TSyntax `command) :
    CommandElabM (List String) := do
  let (diagnostics, published, budget) ← attempt base command
  if published && diagnostics.isEmpty then
    throwError "`{base}` is established now"
  if published then throwError "a proof of `{base}` was published beside {diagnostics}"
  if budget then throwError "the verification of `{base}` ran out of a budget"
  if let some other := diagnostics.find? (!isVerificationDiagnostic base ·) then
    throwError "the attempt of `{base}` reports an error that is not a verification failure of \
      it: {other}"
  unless diagnostics.any (isSummary base) do
    throwError "the verification of `{base}` was not rejected: {diagnostics}"
  return diagnostics.filter (!isSummary base ·)

/-- Elaborate the `verify` command of an open cell: the verification must
reject it, publish no proof artifact, and begin its first report with
`expected`; every diagnostic must be a verification failure of the cell. -/
def expectOpen (base : Name) (expected : String) (command : TSyntax `command) :
    CommandElabM Unit := do
  let diagnostics ← try rejection base command catch error =>
    let text ← error.toMessageData.toString
    if text == s!"`{base}` is established now" then
      throwError "the open cell `{base}` is established now: move it up to the positive controls"
    else throw error
  -- A residual obligation's report goes on with its goal, so the recorded
  -- text is where the first report starts.
  unless diagnostics.head?.any (·.startsWith expected) do
    throwError "the open cell `{base}` fails otherwise than recorded: expected first\n  \
      {expected}\nfound {diagnostics}"

/-- Elaborate the `verify` command of a negative whose failure has no source
clause to name: the verification must reject it with residual obligations
and nothing else, and publish no proof artifact. -/
def expectResidual (base : Name) (command : TSyntax `command) : CommandElabM Unit := do
  let diagnostics ← rejection base command
  unless !diagnostics.isEmpty &&
      diagnostics.all (·.startsWith "verification failed with a residual obligation") do
    throwError "expected residual obligations of `{base}` and no other report, found \
      {diagnostics}"

end VerexSL.Tests.Shapes
