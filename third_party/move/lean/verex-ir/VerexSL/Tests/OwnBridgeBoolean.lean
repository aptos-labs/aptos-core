-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open Lean Elab Command VerexSL VerexSL.Contract VerexIR.Proofs.Denote

verex module 0x42::own_bridge_boolean where
  struct Enabled {T : phantom type} has Copy, Drop, Store where
    flag : Bool
  spec Enabled where
    invariant flag == true

  struct Disabled {T : phantom type} has Copy, Drop, Store where
    flag : Bool
  spec Disabled where
    invariant flag == false

  fun generic {T}(onFlag : &Enabled<T>, offFlag : &Disabled<T>) -> Bool := onFlag.flag
  spec generic where
    ensures result == onFlag.flag
    aborts_if false

  fun closed(onFlag : &Enabled<u64>, offFlag : &Disabled<u64>) -> Bool := offFlag.flag
  spec closed where
    ensures result == offFlag.flag
    aborts_if false

verify 0x42::own_bridge_boolean::generic
verify 0x42::own_bridge_boolean::closed

-- Inspect the kernel-checked theorem of each source clause at both rows.
-- A successful function verification alone could use a different own route.
run_cmd do
  let unitName := VerexSL.Contract.unitName #["0x42", "own_bridge_boolean"]
  let some unit := registeredUnit? (← getEnv) `«0x42».own_bridge_boolean
    | throwError "missing Boolean bridge unit"
  for (index, literal) in [(0, ``Bool.true), (1, ``Bool.false)] do
    let owner : VerexIR.StructHandle := ⟨⟨0⟩, index⟩
    let some declared := structNTy unit owner | throwError "missing Boolean declaration"
    for row in [NRow.cons (.param 0) .nil, NRow.cons (.int 64 false) .nil] do
      let view ← liftTermElabM <| withNominalUnit unitName do
        nativeTypeView (← quoteNTy (declared.subst row))
      let some entry := OwnBridge.bridge? (← getEnv) unitName (view.getArg! 0) (view.getArg! 1)
        | throwError "missing generated bridge for {literal} at {typeArgumentsSpelling row}"
      let .thmInfo info ← getConstInfo entry.theoremName
        | throwError "the registered bridge is not a theorem"
      -- The source language represents a Boolean proposition by equality to true.
      -- The disabled clause translates to (field = true ↔ False).
      unless (info.value.find? fun part =>
          part.isAppOfArity ``Skolems.reframeAtInstance_of_encode 14 &&
          (part.getArg! 11).isConstOf ``Bool.true).isSome do
        throwError "{entry.theoremName} did not transport the Boolean field comparison"
      let condition := if literal == ``Bool.false then ``False else ``True
      unless (info.type.find? (·.isConstOf condition)).isSome do
        throwError "{entry.theoremName} omitted its source comparison to {literal}"
      unless (← collectAxioms entry.theoremName).all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
        throwError "unexpected axioms in {entry.theoremName}"

-- Exercise the same typed-operand builder at a generated theorem's frame.
-- Source-level false becomes False before the builder runs. An address has a
-- different carrier and encoding and must use the same identity law.
run_cmd do
  let .thmInfo info ← getConstInfo ``«0x42».own_bridge_boolean.dataOwn.case1.at_T0.bridge
    | throwError "missing Disabled parameter bridge"
  liftTermElabM <| Lean.Meta.lambdaTelescope info.value fun _ body => do
    let some used := body.find? (·.isAppOfArity ``Skolems.reframeAtInstance_of_encode 14)
      | throwError "missing move in generated proof"
    let frames : VerexSL.Verify.OwnBridge.Frames := { leading := used.getAppArgs.extract 0 9 }
    for (type, literal) in [(mkConst ``NTy.bool, mkConst ``Bool.false),
        (mkConst ``NTy.bool, mkConst ``Bool.true), (mkConst ``NTy.address, mkStrLit "0x42")] do
      let moved ← (VerexSL.Verify.OwnBridge.movedOperand type literal literal).run frames
      unless (moved.proof.find? (·.isAppOfArity ``Skolems.reframeAtInstance_of_encode 14)).isSome do
        throwError "missing closed-value identity for {literal}"
      Lean.Meta.check moved.proof
      let ty ← Lean.Meta.inferType moved.proof
      unless ← Lean.Meta.isDefEq ty (← Lean.Meta.mkEq literal moved.moved) do
        throwError "wrong transport of {literal}"
