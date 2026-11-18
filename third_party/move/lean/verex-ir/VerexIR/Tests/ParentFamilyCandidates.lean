-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base

namespace VerexIR.Tests.ParentFamilyCandidates

open Lean Meta Elab Tactic VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

private abbrev mapStage (θ : TypeArgs) (outer : Carriers) (valid : Prop) : Carriers :=
  { Carriers.instantiate θ outer with holds := fun _ _ => valid }

/-- Require the observer's typed lookup theorem and the exact semantic family
it attaches to the emitted fact. -/
elab "exact_stage_element " container:ident : tactic => withMainContext do
  let id ← getFVarId container
  let type := (← instantiateMVars (← id.getType)).consumeMData
  unless type.isAppOfArity ``NTy.Holds 3 do throwError "expected a container Holds fact"
  let (family, descriptor, value) := (type.getArg! 0, type.getArg! 1, type.getArg! 2)
  let parent := mkApp (mkConst ``Carriers.toSorts) family
  let elementType := descriptor.getArg! 1
  let some view ← NativeMaps.typedView? parent elementType
    | throwError "the stage does not admit a typed table view"
  unless view.carriers == parent && view.type == elementType do
    throwError "the table view transported away from the supplied stage"
  let goal ← getMainGoal
  let wanted ← goal.getType
  for proof in ← observedElementFacts family descriptor value (.fvar id) (some wanted) do
    let stated := (← instantiateMVars (← inferType proof)).consumeMData
    unless stated.isAppOfArity ``NTy.Holds 3 && stated.getArg! 0 == family do continue
    unless (proof.find? (·.isConstOf ``NTy.Holds.of_specMap_get?)).isSome do continue
    if ← withDefault (isDefEq stated wanted) then
      check proof
      goal.assign proof
      replaceMainGoal []
      return
  throwError "the table observer returned no lookup fact at the supplied stage"

-- The lookup reads the operational parent without selecting a new predicate.
-- The observer makes its own explicit projection for the internal typed views.
example (θ : TypeArgs) (outer : Carriers) (valid : Prop)
    (entries : @NTy.carrier (Sorts.instantiate θ outer.toSorts) (.table .bool (.param 0)))
    (key : Bool) (value : @NTy.readCarrier (Sorts.instantiate θ outer.toSorts) (.param 0))
    (held : @NTy.Holds (mapStage θ outer valid) (.table .bool (.param 0)) entries)
    (found : SpecMap.get? (SpecMap.ofEntries (fun x : Bool => x)
      (@NTy.read (Sorts.instantiate θ outer.toSorts) (.param 0)) entries) key = some value) :
    @NTy.Holds (mapStage θ outer valid) (.param 0)
      (@NTy.unread (Sorts.instantiate θ outer.toSorts) (.param 0) rfl value) := by
  exact_stage_element held

private abbrev ownFrame (predicate : Bool → Prop) : GivenFrame :=
  ⟨{ Carriers.ground with own := fun _ _ raw =>
      ∀ value : Bool, Codec.bool.decode? raw = some value → predicate (Prod.fst (value, ())) }⟩

/-- Expose the encoded value's operational parent without reducing the
semantic dictionary on the ownership fact. Check the actual replacement
hypothesis for the projection that Given-clause normalization removes. -/
elab "check_own_candidate " fact:ident " normalized " expected:num : tactic => withMainContext do
  let factId ← getFVarId fact
  let type := (← instantiateMVars (← factId.getType)).consumeMData
  unless type.isAppOfArity ``Carriers.own 4 do throwError "expected a Carriers.own fact"
  let family := type.getArg! 0
  unless family.isAppOfArity ``GivenFrame.toCarriers 1 do throwError "expected a Given stage"
  let encoded := type.getArg! 3
  unless encoded.isAppOfArity ``NTy.encode 3 do throwError "expected an encoding"
  let parent ← withDefault (whnf (encoded.getArg! 0))
  if parent.isAppOfArity ``Carriers.toSorts 1 then
    throwError "the ownership control did not expose a reduced operational parent"
  let parentMatches ← withDefault (withNewMCtxDepth
    (isDefEq parent (mkApp (mkConst ``Carriers.toSorts) family)))
  unless parentMatches == (expected.getNat == 1) do
    throwError "unexpected equality between the reduced parent and the candidate's parent"
  let encoded := mkAppN encoded.getAppFn (encoded.getAppArgs.set! 0 parent)
  let type := mkAppN type.getAppFn (type.getAppArgs.set! 3 encoded)
  let (reduced, goal) ← (← (← getMainGoal).assert `reducedOwn type (.fvar factId)).intro1P
  let some next ← ownAtArgumentsFact? goal reduced
    | throwError "the ownership observer did not replace the encoded fact"
  next.withContext do
    let decl ← getLocalDeclFromUserName `reducedOwn
    let stated := (← instantiateMVars decl.type).consumeMData
    let projected := (stated.find? (fun e => e.isAppOf ``Prod.fst || e.isProj)).isSome
    unless projected == (expected.getNat == 0) do
      throwError "unexpected stage normalization in the emitted ownership clause: {repr stated}"
    check decl.toExpr
    unless ← withDefault (isDefEq stated (← next.getType)) do
      throwError "the ownership observer did not emit the selected clause"
    next.assign decl.toExpr
  replaceMainGoal []

example (predicate : Bool → Prop) (value : Bool)
    (own : @Carriers.own (ownFrame predicate).toCarriers ⟨⟨0⟩, 0⟩ .nil
      (@NTy.encode (ownFrame predicate).toCarriers.toSorts .bool value)) : predicate value := by
  check_own_candidate own normalized 1

-- Even an unrelated dictionary encodes Bool the same way. Decoding the
-- explicit own predicate remains sound, but the Given normalizer must not
-- run on the authority of this mismatched candidate.
example (predicate : Bool → Prop) (operations : Sorts) (value : Bool)
    (own : @Carriers.own (ownFrame predicate).toCarriers ⟨⟨0⟩, 0⟩ .nil
      (@NTy.encode operations .bool value)) : predicate value := by
  check_own_candidate own normalized 0

end VerexIR.Tests.ParentFamilyCandidates
