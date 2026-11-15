-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- This test asserts what the Given route does; it selects the route.
set_option verex.givenRoute true


/-! Source-level Given verification keeps declared ADT invariants at packing
and loan boundaries while treating parameter payloads as opaque values. The
negative commands restore their state and require that no proof or transfer
artifact was created; unrelated compilation or unsupported-path errors do not
count as successful rejection. -/
open Lean Meta Elab Command VerexSL VerexSL.Contract VerexSL.Verify
open VerexIR.Proofs.Denote
verex module 0x42::given_adt_source where
  pragma verify = false
  struct Bounded {T} has Copy, Drop where
    payload : T
    count : u64
  spec Bounded where
    invariant count < 10
  struct Envelope {T} has Copy, Drop where
    inner : Bounded<T>
  fun pack_good {T}(payload : T) -> Bounded<T> :=
    new Bounded<T> { payload, count := 7 }
  spec pack_good where
    aborts_if false
    ensures result.payload == payload && result.count == 7
  fun pack_bad {T}(payload : T) -> Bounded<T> :=
    new Bounded<T> { payload, count := 11 }
  spec pack_bad where
    aborts_if false
    ensures result.payload == payload && result.count == 11
  fun repair {T}(value : &mut Bounded<T>) -> Unit := do
    let counter := &mut value.count
    *counter := 11
    *counter := 7
  spec repair where
    aborts_if false
    ensures value.count == 7 && value.payload == old(value).payload
  fun break_at_return {T}(value : &mut Bounded<T>) -> Unit := do
    let counter := &mut value.count
    *counter := 11
  spec break_at_return where
    aborts_if false
    ensures value.count == 11 && value.payload == old(value).payload
  fun touch {T}(value : &mut Bounded<T>) -> Unit := ()
  spec touch where
    aborts_if false
    ensures value == old(value)
  fun handover_bad {T}(value : &mut Bounded<T>) -> Unit := do
    value.count := 11
    touch(value)
    value.count := 7
  spec handover_bad where
    aborts_if false
    ensures value.count == 7
  fun nested {T}(payload : T) -> Envelope<T> :=
    new Envelope<T> { inner := pack_good(payload) }
  spec nested where
    aborts_if false
    ensures result.inner.count == 7 && result.inner.payload == payload
  fun nested_bad {T}(payload : T) -> Envelope<T> :=
    new Envelope<T> { inner := new Bounded<T> { payload, count := 11 } }
  spec nested_bad where
    aborts_if false
    ensures result.inner.count == 11
  fun swap_values {T}(left : &mut Bounded<T>, right : &mut Bounded<T>) -> Unit := do
    let saved := *left
    *left := *right
    *right := saved
  spec swap_values where
    aborts_if false
    ensures left == old(right) && right == old(left)
verify 0x42::given_adt_source::pack_good by
  on _ { compute }
verify 0x42::given_adt_source::repair by
  on _ { compute }
verify 0x42::given_adt_source::touch by
  on _ { compute }
verify 0x42::given_adt_source::nested by
  on _ { compute }

verify 0x42::given_adt_source::swap_values by
  on _ { compute }

private def forbiddenStepSubstrings : List String := [
  "exceeded its budget", "exceeded its heartbeat budget", "maxHeartbeats",
  "maximum number of heartbeats", "heartbeats", "stopped by the budget",
  "deterministic timeout", "(deterministic) timeout",
  "maximum recursion depth", "deep recursion",
  "unknown identifier", "unknown constant", "unknown function",
  "no Verex module declares", "no denotation for",
  "given verification is unsupported", "given verification concrete transfer failed",
  "pragma verify = manual",
  "type mismatch", "application type mismatch", "failed to synthesize",
  "no goals to be solved", "is not in scope",
  "kernel", "PANIC at", "internal exception"
]

private def expectGivenRejection (base : Name) (command : TSyntax `command) : CommandElabM Unit := do
  let saved ← get
  let priorCount := saved.messages.toList.length
  let some unit := registeredUnit? saved.env base.getPrefix
    | throwError "missing rejection test module"
  let some (namespaceIndex, _, functionIndex, declaration) := findFunction? unit base.getString!
    | throwError "missing rejection test function"
  let handle : VerexIR.FunctionHandle := ⟨⟨namespaceIndex⟩, ⟨functionIndex⟩⟩
  let .ok compiled := compileFunction unit handle
    | throwError "rejection fixture failed source compilation"
  unless isGeneric declaration && (givenUnsupported? unit handle compiled).isNone do
    throwError "rejection fixture is outside the Given path"
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
    throwError "expected no proof artifact for rejected Given body: {base}"
  if thrown.any (·.1) || hasHeartbeatTag then
    throwError "unexpected heartbeat/recursion limit in `{base}`: {errorTexts}"
  if let some fallback := freshTexts.find? (·.contains "uses concrete verification") then
    throwError "unexpected concrete fallback in `{base}`: {fallback}"
  if let some bad := errorTexts.find? (fun text => forbiddenStepSubstrings.any (text.contains ·)) then
    throwError "unexpected budget or infrastructure failure in `{base}`: {bad}"
  let summary := s!"verex verification failed: the proof of `{base.getString!}` does not establish its specification"
  let hasDecideRefusal := errorTexts.any (·.startsWith "`decide` does not close the obligation from the block's facts:")
  let hasSummary := errorTexts.contains summary
  let allExpected := errorTexts.all fun text =>
    text == summary ||
    text.startsWith "`decide` does not close the obligation from the block's facts:" ||
    text.startsWith "unsolved goals"
  unless !errorTexts.isEmpty && hasDecideRefusal && hasSummary && allExpected do
    throwError "expected an invariant rejection in the Given body ({base}), got: {errorTexts}"

run_cmd do
  expectGivenRejection `«0x42».given_adt_source.pack_bad
    (← `(command| verify 0x42::given_adt_source::pack_bad by
      on _ { compute }))
  expectGivenRejection `«0x42».given_adt_source.break_at_return
    (← `(command| verify 0x42::given_adt_source::break_at_return by
      on _ { compute }))
  expectGivenRejection `«0x42».given_adt_source.handover_bad
    (← `(command| verify 0x42::given_adt_source::handover_bad by
      on _ { compute }))

  expectGivenRejection `«0x42».given_adt_source.nested_bad
    (← `(command| verify 0x42::given_adt_source::nested_bad by
      on _ { compute }))

run_cmd do
  let env ← getEnv
  for name in ["pack_good", "repair", "touch", "nested", "swap_values"] do
    let base := Name.str `«0x42».given_adt_source name
    unless (givenProofArtifacts.getState env).find? base == some (base ++ `givenTypedVerified) do
      throwError "source ADT test did not use the Given proof path: {name}"
    let some transfer := env.find? (base ++ `givenTransfer)
      | throwError "source ADT test lost explicit transfer evidence: {name}"
    unless transfer.type == mkSort .zero do
      throwError "source ADT transfer evidence is not a proposition: {name}"

run_cmd do
  let env ← getEnv
  let table := `«0x42».given_adt_source.givenOwnDeclarations
  unless env.contains table do
    throwError "missing shared Given declaration table"
  for name in ["pack_good", "repair", "touch", "nested", "swap_values"] do
    let theoremName := (Name.str `«0x42».given_adt_source name) ++ `givenTypedVerified
    let some theoremInfo := env.find? theoremName | throwError "missing theorem"
    unless (theoremInfo.type.find? (·.isConstOf table)).isSome do
      throwError "the Given theorem telescope lost its named declaration table: {name}"
    if (theoremInfo.type.find? fun e =>
        e.isAppOfArity ``VerexIR.Proofs.Denote.GivenFrame.withDeclarations 2 &&
        !(e.getArg! 1).getAppFn.isConstOf table).isSome then
      throwError "the Given theorem telescope contains an inline declaration table: {name}"

-- A read-only mutable loan must close under its entry invariant. The own
-- predicate is decoded after the surrounding context was normalized, so
-- its newly exposed plain-family transport must reach the same normal form.
namespace VerexSL.Tests.ReadOnlyInvariantLoan

verex module 0x42::read_only_invariant_loan where
  -- ## Representation

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

  public fun copy_entries {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &mut Map<K, V>) -> Vector<Entry<K, V> > :=
    map.entries
  spec copy_entries where
    aborts_if false
    ensures result.length == old(map).entries.length

end VerexSL.Tests.ReadOnlyInvariantLoan
