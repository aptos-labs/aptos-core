-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

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

run_cmd do
  let saved ← get
  let base := `«0x42».given_ordering.invalid_order
  let thrown ← try
    let command ← `(command| verify 0x42::given_ordering::invalid_order)
    -- The quoted commands name existing source declarations, not hygienic locals.
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
    throwError "expected the unsorted Given construction to fail with no proof artifacts"
