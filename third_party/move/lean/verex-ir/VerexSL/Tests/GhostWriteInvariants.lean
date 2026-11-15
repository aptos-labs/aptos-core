-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# `.ghostWrite` producer and invariant controls

Tests producer-form domain checking at `ghostAt` (`wp_ghostAt` / `ghostWriteStep` ->
`checkingAt site .construction τ value`) when `spec do update` writes to a ghost field:
1. **Negative controls (first)**:
   - `bad_open_mut`: writes `new Item { count := 0 }` to `s.tag` through `s : &mut Holder`
     (where `Proj.setOwner?_deref` skips owner checks on write) and immediately repairs
     `s.tag` to `new Item { count := 1 }` before exit. Rejected at `.construction`
     naming `` `invariant count > 0` `` (`"the data invariant \`invariant count > 0\` does not hold where a value is constructed"`).
   - `bad_ensures_false`: writes `new Item { count := 0 }` to `h.tag` with `ensures false`,
     confirming `ghostAt` *checks* `checkingAt site .construction τ value` (`undefined` on
     failure, never vacuous `Spec.assume`). Rejected at `.construction` naming `` `invariant count > 0` ``.
2. **Positive controls (second)**:
   - `ok_update`: under `requires n > 0`, `update s.tag := new Item { count := n }` verifies
     cleanly and establishes `s.tag.count == n && s.tag.count > 0` (`ghostHolds` retained).
   - `ok_u64_ghost_write`: invariant-free `ghost idx : u64` simplifies `checkingAt` to
     `Spec.pure ()` during `ghostWriteStep`, costing zero `checkingAt` / invariant steps in
     the compiled proof term.
-/

namespace VerexSL.Tests.GhostWriteInvariants

open Lean Meta Elab Command VerexSL.Verify

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

private def assertRejectedConstruction (base : Name) (command : TSyntax `command)
    (expectedInvariant : String) (label : String) : CommandElabM Unit := do
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
    throwError "expected `{label}` ({base}) not to publish any proof artifact"
  if thrown.any (·.1) || hasHeartbeatTag then
    throwError "expected semantic rejection for `{label}` ({base}), got heartbeat/recursion limit: {errorTexts}"
  if let some bad := errorTexts.find? (fun text => forbiddenSubstrings.any (text.contains ·)) then
    throwError "expected semantic rejection for `{label}` ({base}), got budget/infrastructure diagnostic: {bad}"
  let fn := base.getString!
  let summaryPrefix := s!"verex verification failed: the automatic verification of `{fn}` failed;"
  let expectedDiag :=
    s!"the data invariant `{expectedInvariant}` does not hold where a value is constructed"
  let isExpectedOther (text : String) : Bool :=
    text.startsWith summaryPrefix ||
    text == expectedDiag ||
    (text.startsWith "the specification clause " && text.endsWith " is not established")
  let hasSummary := errorTexts.any (·.startsWith summaryPrefix)
  let hasNamedConstruction := errorTexts.any (· == expectedDiag)
  let allExpected := errorTexts.all isExpectedOther
  unless !errorTexts.isEmpty && hasSummary && hasNamedConstruction && allExpected do
    throwError "expected `{label}` ({base}) to fail with `{expectedDiag}`, got: {errorTexts}"

verex module 0x42::ghost_write_invariants where
  pragma verify = false

  struct Item has Copy, Drop, Store where
    count : u64
  spec Item where
    invariant count > 0

  struct Holder has Copy, Drop where
    x : u64
  spec Holder where
    ghost tag : Item

  struct HolderU64 has Copy, Drop where
    x : u64
  spec HolderU64 where
    ghost idx : u64

  -- Negative 1 (first): write through `&mut Holder` where `Proj.setOwner?_deref` skips
  -- owner checking on write, followed by an immediate repair before exit; caught solely
  -- by `ghostAt` binding `checkingAt site .construction Item value`.
  fun bad_open_mut(s : &mut Holder) -> Unit := do
    spec do
      update s.tag := new Item { count := 0 }
    spec do
      update s.tag := new Item { count := 1 }
  spec bad_open_mut where
    aborts_if false
    ensures s.tag.count == 1

  -- Negative 2 (second): invalid ghost write followed by `ensures false`; confirms
  -- `ghostAt` checks `checkingAt site .construction τ value` (`undefined` on failure)
  -- rather than assuming `NTy.Holds τ value` (which would make `0 > 0` vacuous).
  fun bad_ensures_false(s : Holder) -> Holder := do
    let mut h := s
    spec do
      update h.tag := new Item { count := 0 }
    h
  spec bad_ensures_false where
    aborts_if false
    ensures false

  -- Positive 1 (third): valid ghost write under `requires n > 0` discharges `.construction`
  -- and retains `NTy.Holds Item` (`s.tag.count > 0`) in the continuation.
  fun ok_update(s : &mut Holder, n : u64) -> Unit := do
    spec do
      update s.tag := new Item { count := n }
  spec ok_update where
    requires n > 0
    aborts_if false
    ensures s.tag.count == n && s.tag.count > 0

  -- Positive 2 (fourth): invariant-free `u64` ghost write simplifies `checkingAt` to
  -- `Spec.pure ()`, costing zero `checkingAt` / invariant steps.
  fun ok_u64_ghost_write(s : &mut HolderU64, n : u64) -> Unit := do
    spec do
      update s.idx := n
  spec ok_u64_ghost_write where
    aborts_if false
    ensures s.idx == n

/-! ## 1. Negative Controls (First) -/

run_cmd do
  assertRejectedConstruction `«0x42».ghost_write_invariants.bad_open_mut
    (← `(command| verify 0x42::ghost_write_invariants::bad_open_mut))
    "invariant count > 0"
    "ghostWrite invalid Item through &mut Holder (repaired before exit)"
  assertRejectedConstruction `«0x42».ghost_write_invariants.bad_ensures_false
    (← `(command| verify 0x42::ghost_write_invariants::bad_ensures_false))
    "invariant count > 0"
    "ghostWrite invalid Item zero-vacuity control (ensures false)"

/-! ## 2. Positive Controls (Second) -/

verify 0x42::ghost_write_invariants::ok_update
verify 0x42::ghost_write_invariants::ok_u64_ghost_write

run_cmd liftTermElabM do
  let env ← getEnv
  let curNs ← getCurrNamespace
  for fn in [`ok_update, `ok_u64_ghost_write] do
    let thm := if env.contains (curNs ++ `«0x42».ghost_write_invariants ++ fn ++ `typedVerified) then
        curNs ++ `«0x42».ghost_write_invariants ++ fn ++ `typedVerified
      else `«0x42».ghost_write_invariants ++ fn ++ `typedVerified
    let some info := env.find? thm
      | throwError "missing model verification theorem {thm}"
    let axioms ← collectAxioms thm
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {thm}: {axioms}"
    if fn == `ok_u64_ghost_write then
      let some value := info.value? (allowOpaque := true)
        | throwError "{thm} has no proof term"
      if value.getUsedConstants.contains ``VerexIR.Proofs.Denote.checkingAt then
        throwError "expected invariant-free u64 ghost write in {thm} to erase `checkingAt`, but proof term still mentions `checkingAt`"

end VerexSL.Tests.GhostWriteInvariants
