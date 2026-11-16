-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Tests.NominalQuotation

/-! # A named declaration observed under a frame's operations

A generic reader's frame spells a resource's type as its resolution of the
declaration (`Skolems.resolve`), and holds a stored value moved to the frame
of the resource's type arguments (`Skolems.reframe`).  Two observations of
the declaration cross those operations without opening it: its identity
(`nominalHandle`), which dispatches a stored value to its declaration's
invariant, and the projection of a field of the moved value
(`nominalReframeHead`, `nominalReframeRest`), which is the field moved.  The
field's own type keeps its name.

Negatives first: the resolution of a type parameter has no identity to
observe, and a moved value that is not projected stays whole.
-/

open Lean Meta Elab Command VerexSL VerexSL.Contract VerexIR.Proofs.Denote

namespace VerexSL.Tests.NominalFrameObservations

/-- The observations under test and the structural laws they hand over to. -/
private def observe (term : Lean.Expr) : MetaM Simp.Result := do
  let procs ← Simp.SimprocsArray.add #[] ``nominalHandle true
  let procs ← procs.add ``nominalReframeHead true
  let procs ← procs.add ``nominalReframeRest true
  let theorems ← ({} : SimpTheorems).addConst ``Skolems.reframe_struct_fst
  let theorems ← theorems.addConst ``Skolems.reframe_struct_snd
  let context ← Simp.mkContext
    { zeta := false, beta := false, iota := false, proj := false } #[theorems]
  let (result, _) ← simp term context procs
  -- What an observation states it proves.
  if let some proof := result.proof? then
    Meta.check proof
    unless ← withDefault (isDefEq (← inferType proof) (← mkEq term result.expr)) do
      throwError "an observation's proof does not state its rewrite: {term}"
  return result

run_cmd do
  let some unit := registeredUnit? (← getEnv) `«0x42».nominal_quotation
    | throwError "missing nominal quotation fixture"
  let name ← ensureUnitDefinition #["0x42", "nominal_quotation"] unit
  liftTermElabM <| withNominalUnit name do
    let schema (index : Nat) : MetaM Lean.Expr := do
      let some schema := structNTy unit ⟨⟨0⟩, index⟩ | throwError "missing fixture schema {index}"
      quoteNTy schema
    let (leaf, left, right, diamond, choice) :=
      (← schema 0, ← schema 2, ← schema 3, ← schema 4, ← schema 5)
    let handleOf (index : Nat) : MetaM Lean.Expr :=
      mkAppM ``Option.some #[toExpr (⟨⟨0⟩, index⟩ : VerexIR.StructHandle)]
    withLocalDeclD `functions (mkApp (mkConst ``FunctionCarriers) (mkConst name)) fun functions =>
    withLocalDeclD `frame (mkApp2 (mkConst ``Skolems) (mkConst name) functions) fun frame => do
      let resolve (τ : Lean.Expr) : Lean.Expr :=
        mkApp4 (mkConst ``Skolems.resolve) (mkConst name) functions frame τ
      -- A type parameter's resolution is whatever the frame reads: no handle.
      let parameter := mkApp (mkConst ``NTy.handle?)
        (resolve (mkApp (mkConst ``NTy.param) (toExpr (0 : Nat))))
      unless (← observe parameter).expr == parameter do
        throwError "the resolution of a type parameter was given an identity"
      -- A struct and an enum keep their identity where a frame resolves them,
      -- and the declaration stays named in the proof.
      for (owner, index) in [(leaf, 0), (diamond, 4), (choice, 5)] do
        let observed ← observe (mkApp (mkConst ``NTy.handle?) (resolve owner))
        unless observed.expr == (← handleOf index) do
          throwError "the declaration {index} was not observed under its resolution: {observed.expr}"
        let some proof := observed.proof? | throwError "an observation without a proof"
        unless (proof.find? (· == owner)).isSome do
          throwError "the observation's proof erased the descriptor"
      -- At its own frame the declaration is observed as before.
      unless (← observe (mkApp (mkConst ``NTy.handle?) leaf)).expr == (← handleOf 0) do
        throwError "the declaration was not observed at its own frame"
      -- A value of `Diamond<T>` moved to the frame of its type arguments.
      let params := mkApp2 (mkConst ``NRow.cons) (mkApp (mkConst ``NTy.param) (toExpr (0 : Nat)))
        (mkConst ``NRow.nil)
      let own ← mkDecideProof (← mkEq (mkApp2 (mkConst ``NRow.isParamsFrom) (toExpr (0 : Nat)) params)
        (mkConst ``Bool.true))
      let allowed ← withLocalDeclD `index (mkConst ``Nat) fun index => do
        mkLambdaFVars #[index] (← mkDecide (← mkAppM ``LT.lt
          #[index, mkApp (mkConst ``NRow.length) params]))
      let boundedType ← mkEq (mkApp2 (mkConst ``NTy.paramsAll) allowed diamond) (mkConst ``Bool.true)
      let carriers := mkApp3 (mkConst ``Skolems.toCarriers) (mkConst name) functions frame
      withLocalDeclD `bounded boundedType fun bounded =>
      withLocalDeclD `value (mkApp2 (mkConst ``NTy.carrier) carriers diamond) fun value => do
        let moved := mkAppN (mkConst ``Skolems.reframe)
          #[mkConst name, functions, frame, params, own, diamond, bounded, value]
        Meta.check moved
        -- Not projected, the moved value stays whole.
        unless (← observe moved).expr == moved do
          throwError "a moved value was opened without a projection"
        let first (e : Lean.Expr) : MetaM Lean.Expr := withDefault (mkAppM ``Prod.fst #[e])
        let second (e : Lean.Expr) : MetaM Lean.Expr := withDefault (mkAppM ``Prod.snd #[e])
        -- A projection of the moved value is the field moved, at the field's
        -- own named type.
        let fieldMoved (projected : Lean.Expr) (field : Lean.Expr) (owner : Lean.Expr)
            (label : String) : MetaM Unit := do
          let observed ← observe projected
          let result := observed.expr
          unless result.isAppOfArity ``Skolems.reframe 8 do
            throwError "the {label} field of a moved value is not the field moved: {result}"
          unless result.getArg! 5 == owner do
            throwError "the {label} field moved lost its type's name: {result.getArg! 5}"
          unless ← withDefault (isDefEq (result.getArg! 7) field) do
            throwError "the {label} field moved is another value: {result.getArg! 7}"
          if (result.getArg! 5).find? (·.isAppOf ``NTy.struct) |>.isSome then
            throwError "the {label} field's layout leaked"
        fieldMoved (← first moved) (← first value) left "head"
        fieldMoved (← first (← second moved)) (← first (← second value)) right "later"
        fieldMoved (← first (← first moved)) (← first (← first value)) leaf "nested"
        -- The same field spelled as the primitive projection of the moved
        -- value carries no argument a projection rule could read: the rules
        -- decline it and leave the term as it is.
        for (label, index) in [("head", 0), ("rest", 1)] do
          let projection := mkProj ``Prod index moved
          let observed ← observe projection
          unless observed.expr == projection do
            throwError "the primitive {label} projection of a moved value was rewritten: \
              {observed.expr}"
          unless observed.proof? matches none do
            throwError "the primitive {label} projection of a moved value was given a proof"
        -- Under metadata the application is still an application: the head
        -- rule reads through the annotation and gives the field moved.
        let annotated := mkMData (KVMap.empty.insert `control (DataValue.ofBool true))
          (← first moved)
        let observed := (← observe annotated).expr.consumeMData
        unless observed.isAppOfArity ``Skolems.reframe 8 do
          throwError "an annotated projection of a moved value was declined: {observed}"
        unless observed.getArg! 5 == left do
          throwError "an annotated projection lost its type's name: {observed.getArg! 5}"

/-! A pair's field is also spelled as the primitive projection of the pair,
which is no application of `Prod.fst` or `Prod.snd` and so carries no
argument a projection rule could read. The two rules decline it and leave the
term as it is: at the head, at the rest, and where the projected structure is
itself a projection. -/

open Lean Meta Elab Command VerexIR.Proofs.Denote in
run_cmd liftTermElabM do
  let natType := mkConst ``Nat
  let inner ← mkAppM ``Prod #[natType, natType]
  withLocalDeclD `pair (← mkAppM ``Prod #[natType, inner]) fun pair => do
    let projections := #[("head", mkProj ``Prod 0 pair), ("rest", mkProj ``Prod 1 pair),
      ("nested head", mkProj ``Prod 0 (mkProj ``Prod 1 pair)),
      ("nested rest", mkProj ``Prod 1 (mkProj ``Prod 1 pair))]
    for (label, projection) in projections do
      Meta.check projection
      let observed ← observe projection
      unless observed.expr == projection do
        throwError "the primitive {label} projection was rewritten: {observed.expr}"
      unless observed.proof? matches none do
        throwError "the primitive {label} projection was given a proof"

end VerexSL.Tests.NominalFrameObservations
