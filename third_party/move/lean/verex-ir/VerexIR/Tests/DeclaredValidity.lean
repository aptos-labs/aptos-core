-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.SharedContinuations

namespace VerexIR.Tests.DeclaredValidity

open Lean Meta Elab Tactic
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote
open VerexIR.Proofs.Denote.SharedContinuations

private def owner : StructHandle := ⟨⟨0⟩, 0⟩
private def kind : NominalKind := ⟨true, [], .owned, true⟩
private abbrev nominal : NTy := .nominal owner .nil kind

elab "close_declared_validity" : tactic => do
  let goal ← getMainGoal
  unless ← defaultProveHoldsGoal? goal do throwError "the declared fact was not used"
  unless ← goal.isAssigned do throwError "the declared fact did not close the goal"
  replaceMainGoal []

/-- Ask the actual closer to try a goal that would need a different certificate. -/
elab "declines_declared_validity " proposition:term : tactic => withMainContext do
  let type ← instantiateMVars (← elabTerm proposition (some (mkSort .zero)))
  let saved ← Lean.Elab.Tactic.saveState
  let goal := (← mkFreshExprSyntheticOpaqueMVar type).mvarId!
  if ← defaultProveHoldsGoal? goal then
    throwError "the closer accepted a missing or mismatched declared fact"
  if ← goal.isAssigned then throwError "the declined validity goal was assigned"
  unless (← goal.getType) == type do
    throwError "the declined validity goal was rewritten"
  saved.restore

example (family : Carriers) (value : @NTy.carrier family.toSorts nominal)
    (exactFact : family.declared owner .nil kind (@NTy.encode family.toSorts nominal value)) :
    @NTy.Holds family nominal value := by
  close_declared_validity

example (family : Carriers) (_value : @NTy.carrier family.toSorts nominal) : True := by
  declines_declared_validity (@NTy.Holds family nominal _value)
  trivial

example (family other : Carriers) (value : @NTy.carrier family.toSorts nominal)
    (_wrongFamily : other.declared owner .nil kind (@NTy.encode family.toSorts nominal value)) :
    True := by
  declines_declared_validity (@NTy.Holds family nominal value)
  trivial

example (family : Carriers) (value : @NTy.carrier family.toSorts nominal)
    (_wrongKind : family.declared owner .nil ⟨false, [], .owned, true⟩
      (@NTy.encode family.toSorts nominal value)) : True := by
  declines_declared_validity (@NTy.Holds family nominal value)
  trivial

example (family : Carriers) (_value other : @NTy.carrier family.toSorts nominal)
    (_wrongValue : family.declared owner .nil kind (@NTy.encode family.toSorts nominal other)) :
    True := by
  declines_declared_validity (@NTy.Holds family nominal _value)
  trivial

example (family : Carriers) (value : @NTy.carrier family.toSorts nominal)
    (_wrongRow : family.declared owner (.cons .bool .nil) kind
      (@NTy.encode family.toSorts nominal value)) : True := by
  declines_declared_validity (@NTy.Holds family nominal value)
  trivial

private abbrev stage (base : Carriers)
    (predicate : StructHandle → NRow → NominalKind → RuntimeValue → Prop) : Carriers :=
  { base with declared := predicate }

-- The constructor and codec convert across stages. Their predicates are
-- independent, and only a receiving-stage fact may close the obligation.
example (base : Carriers)
    (before after : StructHandle → NRow → NominalKind → RuntimeValue → Prop)
    (value : @NTy.carrier (stage base after).toSorts nominal)
    (_earlier : (stage base before).declared owner .nil kind
      (@NTy.encode (stage base before).toSorts nominal value)) : True := by
  have _sameParent : (stage base before).toSorts = (stage base after).toSorts := rfl
  declines_declared_validity (@NTy.Holds (stage base after) nominal value)
  trivial

example (base : Carriers)
    (before after : StructHandle → NRow → NominalKind → RuntimeValue → Prop)
    (value : @NTy.carrier (stage base after).toSorts nominal)
    (_earlier : (stage base before).declared owner .nil kind
      (@NTy.encode (stage base before).toSorts nominal value))
    (received : (stage base after).declared owner .nil kind
      (@NTy.encode (stage base after).toSorts nominal value)) :
    @NTy.Holds (stage base after) nominal value := by
  close_declared_validity

-- The normalizer exposes the declared proposition. An opaque predicate
-- remains as a real obligation.
example (family : Carriers) (value : @NTy.carrier family.toSorts nominal)
    (fact : family.declared owner .nil kind (@NTy.encode family.toSorts nominal value)) :
    @NTy.Holds family nominal value := by
  simp only [vir_denote_norm]
  run_tac withMainContext do
    unless (← getMainTarget).consumeMData.isAppOfArity ``Carriers.declared 5 do
      throwError "expected the declared obligation after normalization"
  exact fact

private abbrev row : NRow := .cons (.param 0) .nil
private abbrev substitutedRow : NRow := .cons .bool .nil
private abbrev substitution : TypeArgs := ⟨substitutedRow, rfl⟩
private abbrev genericNominal : NTy := .nominal owner row kind

-- Nonidentity substitution keeps the caller's predicate at the substituted
-- row and encoding. The outer predicate is arbitrary.
example (outer : Carriers)
    (value : @NTy.carrier outer.toSorts (.nominal owner substitutedRow kind))
    (fact : outer.declared owner substitutedRow kind
      (@NTy.encode outer.toSorts (.nominal owner substitutedRow kind) value)) :
    @NTy.Holds (Carriers.instantiate substitution outer) genericNominal
      (@NTy.toSkolem outer.toSorts substitution genericNominal value) := by
  apply (@NTy.holds_toSkolem_iff outer substitution genericNominal value).mpr
  exact ⟨fact⟩

/-- With no declared type anywhere in the supported type, parameter validity
and raw own suffice. The predicate `declared` is unrestricted. The row motives
are internal to this one induction, not an evaluator support API. -/
private theorem holds_of_supported_without_declarations [Θ : Carriers]
    (parameters : ∀ index (value : Θ.carrier index), Θ.holds index value)
    (own : ∀ source arguments raw, Θ.own source arguments raw)
    (τ : NTy) (supported : τ.supported (fun _ _ => false) = true)
    (value : τ.carrier) : NTy.Holds τ value := by
  apply NTy.Holds.intro
  revert value supported
  apply NTy.rec
    (motive_1 := fun τ => τ.supported (fun _ _ => false) = true →
      (value : τ.carrier) → NTy.deepHolds τ value)
    (motive_2 := fun row => row.supported (fun _ _ => false) = true →
      (values : HList row) → HList.deepHolds row values)
    (motive_3 := fun rows => rows.supported (fun _ _ => false) = true →
      (names : List String) → (value : variantCarrier names rows) →
        variantCarrier.deepHolds names rows value)
    (t := τ)
  · intro _ _; trivial
  · intro _ _; trivial
  · intro _ _ _ _; trivial
  · intro _ _; trivial
  · intro _ _; trivial
  · intro _ _; trivial
  · intro _ _; trivial
  · intro _ _; trivial
  · intro _ _; trivial
  · intro _ ih held values
    exact ih held values
  · intro source arguments fields _ ih held values
    simp only [NTy.supported, Bool.and_eq_true] at held
    exact ⟨own source arguments _, ih held.2 values⟩
  · intro source arguments names rows distinct _ ih held value
    simp only [NTy.supported, Bool.and_eq_true] at held
    exact ⟨own source arguments _, ih held.2 names value⟩
  · intro _ ih held _
    exact fun value _ => ih held value
  · intro _ ih held _
    exact fun value _ => ih held value
  · intro _ _ ih held _
    exact fun value _ => ih held value
  · intro _ _ keyIH valueIH held _
    simp only [NTy.supported, Bool.and_eq_true] at held
    exact fun entry _ => ⟨keyIH held.1 entry.1,
      valueIH held.2 entry.2⟩
  · intro _ ih held value
    exact ih held value
  · intro _ ih held value
    exact ih held value.2
  · intro _ ih held _
    exact ⟨fun value _ => ih held value, fun value _ => ih held value⟩
  · intro index _ value
    exact parameters index value
  · intro _ _ _ _ _ _ _; trivial
  · intro _ ih held value
    exact ih held value
  · intro _ _ _ _ held _
    simp [NTy.supported] at held
  · intro _ _; trivial
  · intro _ _ headIH tailIH held values
    simp only [NRow.supported, Bool.and_eq_true] at held
    exact ⟨headIH held.1 values.1,
      tailIH held.2 values.2⟩
  · intro _ _ _; trivial
  · intro _ _ fieldIH tailIH held names value
    simp only [NRows.supported, Bool.and_eq_true] at held
    cases names with
    | nil => trivial
    | cons name names =>
      cases value with
      | inl fields => exact fieldIH held.1 fields
      | inr rest => exact tailIH held.2 names rest

-- The support premise excludes declared types but still admits a structural
-- value whose field and owner need actual hypotheses.
example (family : Carriers)
    (parameters : ∀ index (value : family.carrier index), family.holds index value)
    (own : ∀ source arguments raw, family.own source arguments raw)
    (value : @NTy.carrier family.toSorts (.struct owner row row)) :
    @NTy.Holds family (.struct owner row row) value :=
  @holds_of_supported_without_declarations family parameters own
    (.struct owner row row) rfl value

example : nominal.supported (fun _ _ => false) = false := rfl

-- A productive default carrier is inhabited and its declared validity is closed.
example : ∃ value : @NTy.carrier Carriers.ground.toSorts nominal,
    ¬ @NTy.Holds Carriers.ground nominal value := by
  refine ⟨⟨rfl⟩, ?_⟩
  intro held
  exact @NTy.Holds.deepHolds Carriers.ground _ _ held

run_cmd do
  for theoremName in [``holds_of_supported_without_declarations,
      ``NTy.holds_toSkolem_iff, ``NTy.holds_ofSkolem_iff] do
    let axioms ← collectAxioms theoremName
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {theoremName}: {axioms}"

end VerexIR.Tests.DeclaredValidity
