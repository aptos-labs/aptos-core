-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open Lean Meta Elab Command VerexSL VerexSL.Contract VerexIR.Proofs.Denote

verex module 0x42::nominal_quotation where
  pragma verify = false
  struct Leaf {T} has Copy, Drop where
    value : T
  struct Phantom {T} has Copy, Drop where
    marker : u64
  struct Left {T} has Copy, Drop where
    leaf : Leaf<T>
  struct Right {T} has Copy, Drop where
    leaf : Leaf<T>
  struct Diamond {T} has Copy, Drop where
    left : Left<T>
    right : Right<T>
  enum Choice {T} has Copy, Drop where
    | Empty
    | Present (value : Diamond<T>)
  spec fun choose(value : Leaf<u64>, n : Int) : Leaf<u64> decreases n :=
    if n <= 0 then value else choose(value, n - 1)
  fun unwrap {T}(value : Leaf<T>) -> T := value.value
  spec unwrap where
    ensures result == value.value
  fun concrete(value : Leaf<u64>) -> u64 := value.value
  spec concrete where
    ensures result == value.value

verex module 0x42::nominal_quotation_other where
  pragma verify = false
  struct Leaf {T} has Copy, Drop where
    other : Bool

verify 0x42::nominal_quotation::unwrap by
  on _ { compute }
verify 0x42::nominal_quotation::concrete by
  on _ { compute }

run_cmd do
  let segments := #["0x42", "nominal_quotation"]
  let otherSegments := #["0x42", "nominal_quotation_other"]
  let some unit := registeredUnit? (← getEnv) `«0x42».nominal_quotation
    | throwError "missing nominal quotation unit"
  let some other := registeredUnit? (← getEnv) `«0x42».nominal_quotation_other
    | throwError "missing other nominal quotation unit"
  let name ← ensureUnitDefinition segments unit
  let otherName ← ensureUnitDefinition otherSegments other
  -- Reusing a defined name must retain the unit that its constant denotes.
  discard <| ensureUnitDefinition segments other
  liftTermElabM do
    let source : VerexIR.StructHandle := ⟨⟨0⟩, 0⟩
    let contract ← getConstInfo `«0x42».nominal_quotation.unwrap.givenTypedContract
    let descriptor := nominalDescriptorName name source
    unless ((contract.value?.getD contract.type).find? (·.isConstOf descriptor)).isSome do
      throwError "the generated source contract does not use the nominal descriptor"
    let concreteContract ← getConstInfo `«0x42».nominal_quotation.concrete.typedContract
    unless ((concreteContract.value?.getD concreteContract.type).find?
        (·.isConstOf descriptor)).isSome do
      throwError "the concrete source contract does not use the nominal descriptor"
    let recursive ← getConstInfo `«0x42».nominal_quotation.choose.spec
    unless (recursive.type.find? (·.isConstOf descriptor)).isSome do
      throwError "the recursive specification type does not use the nominal descriptor"
    let some leaf := structNTy unit source | throwError "missing Leaf schema"
    let some phantom := structNTy unit ⟨⟨0⟩, 1⟩ | throwError "missing Phantom schema"
    let some diamond := structNTy unit ⟨⟨0⟩, 4⟩ | throwError "missing Diamond schema"
    let some choice := structNTy unit ⟨⟨0⟩, 5⟩ | throwError "missing Choice schema"
    withNominalUnit name do
      let quoted ← quoteNTy leaf
      unless quoted.getAppFn.isConstOf (nominalDescriptorName name source) &&
          quoted.getAppNumArgs == 1 do
        throwError "Leaf was not quoted as its canonical generic descriptor"
      unless isNominalType (← getEnv) quoted do throwError "descriptor is not registered"
      unless (← withReducible (whnf quoted)) == quoted do
        throwError "reducible normalization unfolded the nominal descriptor"
      let expanded ← nativeTypeView quoted
      unless expanded.isAppOfArity ``NTy.struct 3 do throwError "one-layer view lost the struct"
      unless ← isDefEq quoted expanded do throwError "descriptor changed its native type"
      let repeated ← quoteNTy leaf
      unless repeated == quoted do throwError "repeated quotation changed descriptor spelling"
      let concrete ← quoteNTy (leaf.subst (.cons (.int 64 false) .nil))
      unless concrete.getAppFn == quoted.getAppFn && concrete.getArg! 0 != quoted.getArg! 0 do
        throwError "instantiation changed descriptor identity or erased arguments"
      let phantomA ← quoteNTy (phantom.subst (.cons .bool .nil))
      let phantomB ← quoteNTy (phantom.subst (.cons .address .nil))
      unless phantomA.getAppFn == phantomB.getAppFn && phantomA != phantomB do
        throwError "phantom type arguments were erased"
      let diamondQuoted ← quoteNTy diamond
      let diamondView ← nativeTypeView diamondQuoted
      let .app (.app _ left) right := diamondView.getArg! 2
        | throwError "unexpected Diamond fields"
      unless left.getAppFn != (right.getArg! 0).getAppFn do
        throwError "equal field layouts erased declaration identity"
      let leftView ← nativeTypeView left
      let rightView ← nativeTypeView (right.getArg! 0)
      unless leftView.isAppOfArity ``NTy.struct 3 && rightView.isAppOfArity ``NTy.struct 3 do
        throwError "nested descriptors were expanded eagerly"
      let leftLeaf := (leftView.getArg! 2).getArg! 0
      let rightLeaf := (rightView.getArg! 2).getArg! 0
      unless leftLeaf == rightLeaf && leftLeaf.getAppFn == quoted.getAppFn do
        throwError "the diamond does not share one Leaf descriptor"
      let choiceQuoted ← quoteNTy choice
      unless (← nativeTypeView choiceQuoted).isAppOfArity ``NTy.enum 5 do
        throwError "enum descriptor has no one-layer view"
      withLocalDeclD `types (← mkArrow (mkConst ``Nat) (mkConst ``NTy)) fun types => do
        let resolved ← quoteResolvedNTy types diamond
        unless resolved.getArg! 0 == mkApp types (toExpr (0 : Nat)) do
          throwError "parameter was not resolved before hiding the layout"
        let view ← nativeTypeView resolved
        let left := (view.getArg! 2).getArg! 0
        unless left.getArg! 0 == resolved.getArg! 0 do
          throwError "resolved argument did not reach the nested descriptor"
      let wrong : NTy := .struct source (.cons (.param 0) .nil) .nil
      let rejected ← try
        discard <| withSyntheticNativeTypes (quoteNTy wrong)
        pure false
      catch _ => pure true
      unless rejected do throwError "quotation accepted a mismatched declaration layout"
      let otherQuoted ← withNominalUnit otherName do
        let some otherLeaf := structNTy other source | throwError "missing other Leaf schema"
        quoteNTy otherLeaf
      unless otherQuoted.getAppFn != quoted.getAppFn do
        throwError "two defined units share a descriptor identity"
      unless (← quoteNTy leaf) == quoted do
        throwError "nested unit scope was not restored"
    let rejected ← try
      discard <| quoteNTy leaf
      pure false
    catch _ => pure true
    unless rejected do throwError "nominal quotation accepted a missing unit scope"
