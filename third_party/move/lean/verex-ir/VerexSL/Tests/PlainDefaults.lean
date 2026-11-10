-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

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

-- Both layers of a genuinely nominal field retain their own default
-- constructor. Empty field types stay unsupported before selecting a default.
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
        throwError "plain variant selected a certified default"
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

-- A postcondition reads the certified exit prophecy. The plain default
-- constructor must not replace the certified default at that boundary.
open VerexSL.Verify in
run_cmd do
  let saved ← get
  let base := `«0x42».plain_variant_default.observe
  let command ← `(command| verify 0x42::plain_variant_default::observe by
    on _ { compute })
  let command := Id.run <| command.raw.replaceM fun stx =>
    pure <| if stx.isIdent then some (mkIdentFrom stx stx.getId.eraseMacroScopes) else none
  let thrown ← try
    elabCommand command
    pure none
  catch error => pure (some (← error.toMessageData.toString))
  let state ← get
  let messages ← state.messages.toList.mapM fun message => message.data.toString
  let noProof := !state.env.contains (base ++ `givenTypedVerified) &&
    !state.env.contains (base ++ `typedVerified) &&
    !state.env.contains (base ++ `givenTransfer) &&
    ((givenProofArtifacts.getState state.env).find? base).isNone
  let rejected := (messages ++ thrown.toList).any fun message =>
    message.startsWith "verex verification failed: " &&
      message.contains s!"`{base.getString!}`"
  set saved
  unless noProof && rejected do
    throwError "the certified exit prophecy unexpectedly used a plain structural default"

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
