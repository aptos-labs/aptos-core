-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

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

-- An unchanged vector need not contain the requested element.
run_cmd do
  let saved ← get
  let base := `«0x42».given_vector_search.unchanged
  let thrown ← try
    let command ← `(command| verify 0x42::given_vector_search::unchanged)
    let command := Id.run <| command.raw.replaceM fun stx =>
      pure <| if stx.isIdent then some (mkIdentFrom stx stx.getId.eraseMacroScopes) else none
    elabCommand command
    pure none
  catch error => pure (some (← error.toMessageData.toString))
  let state ← get
  let messages ← state.messages.toList.mapM fun message => message.data.toString
  let noProof := !state.env.contains (base ++ `givenTypedVerified) &&
    !state.env.contains (base ++ `typedVerified) &&
    !state.env.contains (base ++ `givenTransfer) &&
    ((givenProofArtifacts.getState state.env).find? base).isNone
  let expectedFailure := (messages ++ thrown.toList).any fun message =>
    message.startsWith "verex verification failed: " &&
      message.contains s!"`{base.getString!}`"
  set saved
  unless noProof && expectedFailure do
    throwError "expected unchanged-vector membership to fail without proof artifacts"
