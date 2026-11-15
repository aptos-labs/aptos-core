-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- This test asserts what the Given route does; it selects the route.
set_option verex.givenRoute true


open Lean Meta Elab Command VerexSL.Contract VerexIR.Proofs.Denote

-- Plain defaults are constructible without certified inhabitants and depend
-- only on the plain family, even when nominal declaration predicates differ.
run_cmd liftTermElabM <| withSyntheticNativeTypes do
  withLocalDeclD `given (mkConst ``GivenFrame) fun given => do
    let frame := mkApp (mkConst ``GivenFrame.toCarriers) given
    let scaffold := mkApp (mkConst ``GivenFrame.clauseScaffold) given
    let type : NTy := .struct ⟨⟨0⟩, 0⟩ (.cons (.param 0) .nil)
      (.cons (.param 0) (.cons (.int 64 false) .nil))
    let quoted ← quoteNTy type
    let actual ← plainInhabitant frame quoted
    let translated ← plainInhabitant scaffold quoted
    unless actual.isAppOfArity ``NTy.plainInhabitant 3 do
      throwError "plain default lost its named construction"
    unless ← isDefEq actual translated do
      throwError "plain defaults depend on declaration predicates"
    if (actual.find? (·.isConstOf ``CarriersNonempty.inhabitant)).isSome then
      throwError "plain default uses a certified inhabitant"
    let expected := mkApp3 (mkConst ``NTy.plainInhabitant) frame quoted
      (← inhabitableProof quoted)
    unless ← isDefEq actual expected do
      throwError "plain default differs from the structural plain value"

run_cmd liftTermElabM <| withSyntheticNativeTypes do
  withLocalDeclD `given (mkConst ``GivenFrame) fun given => do
    let frame := mkApp (mkConst ``GivenFrame.toCarriers) given
    let scaffold := mkApp (mkConst ``GivenFrame.clauseScaffold) given
    let inhabitants := mkApp (mkConst ``GivenFrame.clauseScaffoldNonempty) given
    let row : NRow := .cons (.param 0) (.cons (.param 1) .nil)
    let rowExpr ← quoteRow row
    let predicate ← withLocalDeclD `row (mkConst ``NRow) fun row => do
      mkLambdaFVars #[row] (← mkEq (mkApp (mkConst ``NRow.inhabitable) row) (toExpr true))
    let θ ← mkAppOptM ``Subtype.mk #[mkConst ``NRow, predicate, rowExpr,
      ← inhabitableProof (← quoteNTy (.tuple row))]
    let induced := mkApp3 (mkConst ``Carriers.instantiate) θ scaffold inhabitants
    let type : NTy := .struct ⟨⟨0⟩, 0⟩ row row
    let quoted ← quoteNTy type
    let actual ← plainInhabitant frame quoted
    let translated ← plainInhabitant induced quoted
    unless ← isDefEq actual translated do
      throwError "plain defaults differ in identity-substituted declaration scaffold"

-- `plainFallback` introduces no custom axioms (only standard `Classical.choice`),
-- and any two `Nonempty` witnesses at the same type are definitionally equal.
/-- info: 'VerexIR.Proofs.Denote.plainFallback' depends on axioms: [Classical.choice] -/
#guard_msgs in
#print axioms VerexIR.Proofs.Denote.plainFallback

example : plainFallback Int ⟨0⟩ = plainFallback Int ⟨42⟩ := rfl

-- `plainFallbackValue` builds `plainFallback` at the canonical plain worker
-- type without `CarriersNonempty`, and agrees definitionally across a Given
-- frame's carrier view, declaration scaffold, and identity `Carriers.instantiate`.
run_cmd liftTermElabM <| withSyntheticNativeTypes do
  withLocalDeclD `given (mkConst ``GivenFrame) fun given => do
    let frame := mkApp (mkConst ``GivenFrame.toCarriers) given
    let scaffold := mkApp (mkConst ``GivenFrame.clauseScaffold) given
    let inhabitants := mkApp (mkConst ``GivenFrame.clauseScaffoldNonempty) given
    let row : NRow := .cons (.param 0) (.cons (.param 1) .nil)
    let rowExpr ← quoteRow row
    let predicate ← withLocalDeclD `row (mkConst ``NRow) fun row => do
      mkLambdaFVars #[row] (← mkEq (mkApp (mkConst ``NRow.inhabitable) row) (toExpr true))
    let θ ← mkAppOptM ``Subtype.mk #[mkConst ``NRow, predicate, rowExpr,
      ← inhabitableProof (← quoteNTy (.tuple row))]
    let induced := mkApp3 (mkConst ``Carriers.instantiate) θ scaffold inhabitants
    let type : NTy := .struct ⟨⟨0⟩, 0⟩ row
      (.cons (.param 0) (.cons (.int 64 false) .nil))
    let quoted ← quoteNTy type
    let actual ← plainFallbackValue frame quoted
    let atScaffold ← plainFallbackValue scaffold quoted
    let atInduced ← plainFallbackValue induced quoted
    unless actual.isAppOfArity ``plainFallback 2 do
      throwError "plainFallbackValue lost its opaque plainFallback head: {actual}"
    unless actual.getArg! 0 == plainWorkerType frame quoted do
      throwError "plainFallbackValue did not target plainWorkerType"
    if (actual.find? fun sub =>
        sub.isConstOf ``CarriersNonempty || sub.isConstOf ``CarriersNonempty.inhabitant).isSome then
      throwError "plainFallbackValue referenced CarriersNonempty"
    let expected := mkApp3 (mkConst ``NTy.plainFallback) frame quoted (← inhabitableProof quoted)
    unless ← isDefEq actual expected do
      throwError "plainFallbackValue differs from NTy.plainFallback"
    unless ← isDefEq actual atScaffold do
      throwError "plainFallbackValue depends on declaration predicates"
    unless ← isDefEq actual atInduced do
      throwError "plainFallbackValue differs across identity Carriers.instantiate"
    -- Certificate-free `plainFallbackValue` also builds `plainFallback` without
    -- `CarriersNonempty` and preserves GivenFrame / instantiate conversion.
    for freeType in #[NTy.int 64 false, NTy.bool, .tuple row] do
      let freeQuoted ← quoteNTy freeType
      let fFrame ← plainFallbackValue frame freeQuoted
      let fScaffold ← plainFallbackValue scaffold freeQuoted
      let fInduced ← plainFallbackValue induced freeQuoted
      unless fFrame.isAppOfArity ``plainFallback 2 do
        throwError "certificate-free plainFallbackValue leaked a concrete value: {fFrame}"
      if (fFrame.find? fun sub =>
          sub.isConstOf ``CarriersNonempty || sub.isConstOf ``CarriersNonempty.inhabitant).isSome then
        throwError "certificate-free plainFallbackValue referenced CarriersNonempty"
      unless ← isDefEq fFrame fScaffold do
        throwError "certificate-free plainFallbackValue depends on declaration predicates"
      unless ← isDefEq fFrame fInduced do
        throwError "certificate-free plainFallbackValue differs across identity Carriers.instantiate"

-- Both layers of a genuinely nominal field retain their own default
-- constructor until the switch. Empty field types stay unsupported before
-- selecting a default.
run_cmd liftTermElabM <| withSyntheticNativeTypes do
  withLocalDeclD `given (mkConst ``GivenFrame) fun given => do
    let frame := mkApp (mkConst ``GivenFrame.toCarriers) given
    let args : NRow := .cons (.param 0) .nil
    let item : NTy := .struct ⟨⟨0⟩, 0⟩ args (.cons (.param 0) (.cons (.int 64 false) .nil))
    let choice : NTy := .enum ⟨⟨0⟩, 1⟩ args ["Empty", "Present"]
      (.cons .nil (.cons (.cons item .nil) .nil)) (by decide)
    let quoted ← quoteNTy choice
    withLocalDeclD `plain (plainType frame quoted) fun plain => do
      let some (_, read) ← typedVariantField? choice #[("Present", 0)] frame quoted plain
        | throwError "plain variant field became unsupported"
      let fallback := read.getArg! 2
      unless fallback.isAppOfArity ``NTy.plainInhabitant 3 do
        throwError "plain variant selected a certified default: {fallback}"
      if (fallback.find? (·.isConstOf ``CarriersNonempty.inhabitant)).isSome then
        throwError "plain variant fallback used a certified inhabitant"
    withLocalDecl `nonempty .instImplicit (mkApp (mkConst ``CarriersNonempty) frame)
        fun _ => do
      withLocalDeclD `certified (carrierType frame quoted) fun certified => do
        let some (_, read) ← typedVariantField? choice #[("Present", 0)] frame quoted certified
          | throwError "certified variant field became unsupported"
        unless (read.getArg! 2).isAppOfArity ``NTy.inhabitant 4 do
          throwError "certified variant lost its certified default"
    let empty : NTy := .enum ⟨⟨0⟩, 2⟩ .nil [] .nil (by decide)
    let emptyChoice : NTy := .enum ⟨⟨0⟩, 3⟩ .nil ["Empty", "Present"]
      (.cons .nil (.cons (.cons empty .nil) .nil)) (by decide)
    let quotedEmpty ← quoteNTy emptyChoice
    withLocalDeclD `plain (plainType frame quotedEmpty) fun plain => do
      if (← typedVariantField? emptyChoice #[("Present", 0)] frame quotedEmpty plain).isSome then
        throwError "a field of an empty type acquired a default"

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

open VerexSL.Verify in
private def assertVerifyRejected (base : Name) (command : TSyntax `command) (reason : String) :
    CommandElabM Unit := do
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
    throwError "{reason}: proof artifact published for `{base}`"
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
    throwError "{reason}: {errorTexts}"

verex module 0x42::plain_variant_invariant where
  pragma verify = false
  struct Item {T} has Copy, Drop where
    payload : T
    count : u64
  spec Item where
    invariant count > 0
  enum Choice {T} has Copy, Drop where
    | Empty
    | Present (value : Item<T>)
  struct Wrapper {T} has Copy, Drop where
    choice : Choice<T>
  spec Wrapper where
    invariant !(choice is Empty) ||
      (core.data.selectVariants[Choice<T>, Present.value](choice)).count == 0
  fun empty {T}() -> Wrapper<T> :=
    new Wrapper<T> { choice := new Choice<T>::Empty {} }
  spec empty where
    aborts_if false

verify 0x42::plain_variant_invariant::empty by
  on _ { compute }

verex module 0x42::plain_variant_default where
  pragma verify = false
  struct Item {T} has Copy, Drop where
    payload : T
    count : u64
  spec Item where
    invariant count > 0
  enum Choice {T} has Copy, Drop where
    | Empty
    | Present (value : Item<T>)
  fun observe {T}(choice : &mut Choice<T>) -> Unit := ()
  spec observe where
    requires choice is Empty
    aborts_if false
    ensures (core.data.selectVariants[Choice<T>, Present.value](choice)).count == 0

-- A postcondition reads the certified exit prophecy. Neither layer leaks a
-- structural-zero default at that boundary.
run_cmd do
  assertVerifyRejected `«0x42».plain_variant_default.observe
    (← `(command| verify 0x42::plain_variant_default::observe by
      on _ { compute }))
    "the certified exit prophecy unexpectedly used a plain structural default"

-- Certified choices must not affect either family's plain default.
run_cmd liftTermElabM <| withSyntheticNativeTypes do
  withLocalDeclD `given (mkConst ``GivenFrame) fun given => do
    let frame := mkApp (mkConst ``GivenFrame.toCarriers) given
    let scaffold := mkApp (mkConst ``GivenFrame.clauseScaffold) given
    withLocalDecl `actualDefault .instImplicit (mkApp (mkConst ``CarriersNonempty) frame)
        fun _ => do
      withLocalDecl `scaffoldDefault .instImplicit (mkApp (mkConst ``CarriersNonempty) scaffold)
          fun _ => do
        let type : NTy := .struct ⟨⟨0⟩, 0⟩ (.cons (.param 0) .nil)
          (.cons (.param 0) .nil)
        let quoted ← quoteNTy type
        let actual ← plainInhabitant frame quoted
        let translated ← plainInhabitant scaffold quoted
        unless ← isDefEq actual translated do
          throwError "plain defaults depend on independent certified inhabitants"
