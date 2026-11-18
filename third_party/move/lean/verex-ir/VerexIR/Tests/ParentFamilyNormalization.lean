-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.GivenOwn
import VerexIR.Proofs.Denote.Solve.Normal

namespace VerexIR.Tests.ParentFamilyNormalization

open Lean Meta Elab Tactic VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-- Normalize the argument to an opaque consumer and inspect its resulting
syntax. Its proof remains a hypothesis even if a reducible family changes. -/
private def argumentOf (hyp : TSyntax `ident) : TacticM Lean.Expr := do
  let id ← getFVarId hyp
  return (← instantiateMVars (← id.getType)).consumeMData.appArg!.consumeMData

elab "check_parent_head " h:ident : tactic => withMainContext do
  let term ← argumentOf h
  unless term.isAppOfArity ``NTy.carrier 2 &&
      (term.getArg! 0).consumeMData.isAppOfArity ``Carriers.toSorts 1 do
    throwError "normalized carrier lost its semantic parent: {repr term}"

macro "normalize_parent_at " h:ident : tactic =>
  `(tactic| (simp (config := { failIfUnchanged := false }) only [vir_denote_norm, vir_denote_eval] at $h:ident
             dsimp (config := { failIfUnchanged := false }) only at $h:ident))

macro "probe_parent_at " h:ident : tactic =>
  `(tactic| (simp (config := { failIfUnchanged := false }) only [vir_denote_norm, vir_denote_eval] at $h:ident
             check_parent_head $h
             dsimp (config := { failIfUnchanged := false }) only at $h:ident
             check_parent_head $h))

example (θ : TypeArgs) (outer : Carriers) (unit : Validation.ValidatedUnit)
    [FunctionCarriers unit] (frame : Skolems unit) (τ : NTy) (P : Type → Prop)
    (_induced : P (@NTy.carrier (Carriers.instantiate θ outer).toSorts τ))
    (_runtime : P (@NTy.carrier (Carriers.runtime unit).toSorts τ))
    (_skolem : P (@NTy.carrier (Skolems.instantiate θ frame).toCarriers.toSorts τ)) : True := by
  probe_parent_at _induced
  probe_parent_at _runtime
  probe_parent_at _skolem
  trivial

/-- Both Given recognizers must actually produce their canonical output.
Definitional equality alone would also accept a recognizer returning none. -/
elab "check_given_projection " actual:ident " against " expected:ident : tactic => withMainContext do
  let term ← argumentOf actual
  let expected ← argumentOf expected
  let canonical ← canonicalEmission {} none
  let output ← canonical term
  check output
  unless output == expected && (← canonical output) == output do
    throwError "canonical emission did not stabilize at the expected Given projection"
  if term == expected then
    check term
    return
  let result ← if term.isAppOfArity ``Sorts.carrier 2 then
      pure (givenOperationProjection? term)
    else givenCarrierViewProjection? term (fun a b => withDefault (isDefEq a b))
  let some result := result | throwError "no Given projection for {repr term}"
  unless result == expected do
    throwError "Given projection produced {repr result}; expected {repr expected}"
  check result

example (frame : GivenFrame) (i : Nat) (τ : NTy) (P : Type → Prop)
    (_operation : P (@Sorts.carrier frame.clauseScaffold.toSorts i))
    (_expectedOp : P (@Sorts.carrier frame.operations.toSorts i))
    (_carrier : P (@NTy.carrier frame.clauseScaffold.toSorts τ))
    (_expectedCarrier : P (@NTy.carrier frame.toCarriers.toSorts τ)) : True := by
  check_given_projection _operation against _expectedOp
  normalize_parent_at _operation
  check_given_projection _operation against _expectedOp
  check_given_projection _carrier against _expectedCarrier
  normalize_parent_at _carrier
  check_given_projection _carrier against _expectedCarrier
  trivial

/-- The non-ground declaration and free Skolem frame select unsealedOwn?
through selectedOwnClauses?. Check the returned proof as the goal. -/
local notation "boxSchema" => NTy.struct ⟨⟨0⟩, 0⟩ (NRow.cons (NTy.param 0) NRow.nil) (NRow.cons (NTy.param 0) NRow.nil)

elab "exact_normalized_own " h:ident : tactic => withMainContext do
  let id ← getFVarId h
  let ty := (← instantiateMVars (← id.getType)).consumeMData
  let some (_, proof) ← selectedOwnClauses? pure ty (.fvar id)
    | do
      unless ty.isAppOfArity ``OwnAtArguments 6 do
        throwError "the normalized hypothesis is not OwnAtArguments: {repr ty}"
      let encoded := (ty.getArg! 5).consumeMData
      unless encoded.isAppOfArity ``NTy.encode 3 do
        throwError "the normalized value is not NTy.encode: {repr encoded}"
      let parent := (encoded.getArg! 0).consumeMData
      unless parent.isAppOfArity ``Carriers.toSorts 1 do
        throwError "normalized ownership encoding has bare parent: {repr parent}"
      let family := parent.appArg!.consumeMData
      unless family.isAppOfArity ``Skolems.toCarriers 3 do
        throwError "normalized ownership family is not Skolems.toCarriers: {repr family}"
      let _ ← reframedOwnAtInstance (family.getArg! 0) (family.getArg! 1) (ty.getArg! 1)
        (family.getArg! 2) (ty.getArg! 2) (encoded.getArg! 1) (ty.getArg! 4) (encoded.getArg! 2)
      throwError "reframing succeeded but the selected ownership fact did not unseal: {repr ty}"
  unless (proof.find? (·.isConstOf ``OwnAtArguments.encode)).isSome do
    throwError "the ownership proof did not use the unsealing equivalence"
  check proof
  let some goal ← ownAtArgumentsFact? (← getMainGoal) id
    | throwError "the decoded fact was not installed in the goal"
  replaceMainGoal [goal]
  evalTactic (← `(tactic| assumption))

example (unit : Validation.ValidatedUnit) [f : FunctionCarriers unit] (frame : Skolems unit)
    (value : @NTy.carrier frame.toCarriers.toSorts boxSchema) (P : Prop)
    (h : OwnAtArguments unit f boxSchema (fun _ _ => P)
        ((NRow.cons (.param 0) .nil).map frame.resolve)
        (@NTy.encode frame.toCarriers.toSorts boxSchema value)) : P := by
  normalize_parent_at h
  exact_normalized_own h

/-- Without a semantic frame the operational spelling does not authorize
unsealing or replacing the source ownership hypothesis. -/
elab "check_bare_own " h:ident : tactic => withMainContext do
  let id ← getFVarId h
  let ty := (← id.getType).consumeMData
  let parent := (ty.getArg! 5).getArg! 0
  unless [``Sorts.instantiate, ``Sorts.runtime, ``Sorts.ground].any parent.isAppOf do
    throwError "expected a bare operational parent: {parent}"
  unless (← selectedOwnClauses? pure ty (.fvar id)).isNone &&
      (← ownAtArgumentsFact? (← getMainGoal) id).isNone do
    throwError "decoded ownership without a semantic frame: {ty}"

elab "check_bare_given " h:ident : tactic => withMainContext do
  let term ← argumentOf h
  unless (givenOperationProjection? term).isNone &&
      (← givenCarrierViewProjection? term (fun a b => withDefault (isDefEq a b))).isNone do
    throwError "recovered a Given projection without a semantic frame"
  let canonical ← canonicalEmission {} none
  let output ← canonical term
  check output
  unless output == term do
    throwError "canonical emission changed bare projection {term} to {output}"

example (unit : Validation.ValidatedUnit) [f : FunctionCarriers unit] (frame : Skolems unit)
    (θ : TypeArgs) (outer : Carriers)
    (value : @NTy.carrier (Sorts.instantiate θ outer.toSorts) boxSchema) (P : Prop)
    (_held : OwnAtArguments unit f boxSchema (fun _ _ => P)
        ((NRow.cons (.param 0) .nil).map frame.resolve)
        (@NTy.encode (Sorts.instantiate θ outer.toSorts) boxSchema value))
    (Q : Type → Prop) (i : Nat) (τ : NTy)
    (_op : Q (@Sorts.carrier (Sorts.instantiate θ outer.toSorts) i))
    (_view : Q (@NTy.carrier (Sorts.instantiate θ outer.toSorts) τ)) : True := by
  check_bare_own _held
  check_bare_given _op
  check_bare_given _view
  trivial

example (unit : Validation.ValidatedUnit) [f : FunctionCarriers unit] (frame : Skolems unit)
    (value : @NTy.carrier (Sorts.runtime unit) boxSchema) (P : Prop)
    (_held : OwnAtArguments unit f boxSchema (fun _ _ => P)
        ((NRow.cons (.param 0) .nil).map frame.resolve)
        (@NTy.encode (Sorts.runtime unit) boxSchema value))
    (Q : Type → Prop) (i : Nat) (τ : NTy)
    (_op : Q (@Sorts.carrier (Sorts.runtime unit) i))
    (_view : Q (@NTy.carrier (Sorts.runtime unit) τ)) : True := by
  check_bare_own _held
  check_bare_given _op
  check_bare_given _view
  trivial

example (unit : Validation.ValidatedUnit) [f : FunctionCarriers unit] (frame : Skolems unit)
    (value : @NTy.carrier Sorts.ground boxSchema) (P : Prop)
    (_held : OwnAtArguments unit f boxSchema (fun _ _ => P)
        ((NRow.cons (.param 0) .nil).map frame.resolve)
        (@NTy.encode Sorts.ground boxSchema value))
    (Q : Type → Prop) (i : Nat) (τ : NTy)
    (_op : Q (@Sorts.carrier Sorts.ground i))
    (_view : Q (@NTy.carrier Sorts.ground τ)) : True := by
  check_bare_own _held
  check_bare_given _op
  check_bare_given _view
  trivial

/-- For a known table read the observer must construct the actual typed
fact. Its two typed views and emitted result pass the parent recognizer. -/
elab "exact_observed_table " held:ident : tactic => withMainContext do
  let id ← getFVarId held
  let ty := (← id.getType).consumeMData
  let wanted ← (← getMainGoal).getType
  let built ← observedElementFacts (ty.getArg! 0) (ty.getArg! 1) (ty.getArg! 2) (.fvar id) (some wanted)
  for fact in built do
    if ← withDefault (isDefEq (← inferType fact) wanted) then
      check fact
      (← getMainGoal).assign fact
      replaceMainGoal []
      return
  throwError "no matching typed fact for the normalized table lookup"

example (unit : Validation.ValidatedUnit) [FunctionCarriers unit] (entries : SpecEntries (Bool × Bool))
    (key value : Bool) (held : @NTy.Holds (Carriers.runtime unit) (.table .bool .bool) entries)
    (found : SpecMap.get? (SpecMap.ofEntries (fun x : Bool => x) (fun x : Bool => x) entries) key =
      some value) : @NTy.Holds (Carriers.runtime unit) .bool value := by
  normalize_parent_at found
  exact_observed_table held

end VerexIR.Tests.ParentFamilyNormalization
