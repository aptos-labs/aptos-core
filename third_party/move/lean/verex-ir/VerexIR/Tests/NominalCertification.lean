-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.GivenOwn
import VerexIR.Proofs.Denote.Compile

/-!
# Carrier independence and invariant sensitivity of nominals

Under the plain carrier representation:
- `NTy.Holds` depends semantically on `Carriers.own` and `Carriers.holds` (negative controls
  first), while `NTy.carrier` is independent of both `Carriers.own` and `Carriers.holds`
  (positive controls, definitionally on concrete argument rows and propositionally across all
  `arguments : NRow`);
- `NTy.subst` distinguishes distinct nominal kinds (negative control) and reaches
  a nominal through its type arguments alone (positive control);
- a declared type's inhabitability reads the arguments its kind demands, and the
  kind's four components are each read into a declared type's token stream.
-/

namespace VerexIR.Proofs.Denote
namespace NominalCertification

private abbrev testHandle : StructHandle := ⟨⟨0⟩, 0⟩
private abbrev structTy : NTy := .struct testHandle .nil (.cons (.int 64 false) .nil)

@[reducible] private def carriersOwnFalse : Carriers :=
  { Carriers.ground with
    own := fun src _ _ => if src == testHandle then False else True }

@[reducible] private def carriersOwnTrue : Carriers :=
  { Carriers.ground with
    own := fun src _ _ => if src == testHandle then True else True }

private def zeroStruct : @NTy.carrier Carriers.ground structTy := (⟨0, rfl⟩, ())

variable {source : StructHandle} {arguments : NRow} {kind : NominalKind}

/-- Negative and positive semantic kernel control (first): the concrete value `zeroStruct`
satisfies `¬ NTy.Holds` under `carriersOwnFalse` and `NTy.Holds` under `carriersOwnTrue`,
proving that `NTy.Holds` depends on `Carriers.own`. -/
theorem holds_depends_on_own_zero :
    ¬ @NTy.Holds carriersOwnFalse structTy zeroStruct ∧
      @NTy.Holds carriersOwnTrue structTy zeroStruct :=
  ⟨fun h => (@NTy.Holds.deepHolds carriersOwnFalse structTy zeroStruct h).1,
    @NTy.Holds.intro carriersOwnTrue structTy zeroStruct ⟨trivial, trivial, trivial⟩⟩

/-- Over every plain carrier value `v` of `structTy`, `NTy.Holds` is refuted under
`carriersOwnFalse` and holds under `carriersOwnTrue`. -/
theorem holds_depends_on_own (v : @NTy.carrier Carriers.ground structTy) :
    ¬ @NTy.Holds carriersOwnFalse structTy v ∧
      @NTy.Holds carriersOwnTrue structTy v :=
  ⟨fun h => (@NTy.Holds.deepHolds carriersOwnFalse structTy v h).1,
    @NTy.Holds.intro carriersOwnTrue structTy v ⟨trivial, trivial, trivial⟩⟩

/-- Negative semantic kernel control: `NTy.Holds` on a type parameter depends on `Carriers.holds`. -/
theorem holds_depends_on_param_holds (base : Carriers) (v : base.carrier 0) :
    ¬ @NTy.Holds { base with holds := fun _ _ => False } (.param 0) v ∧
      @NTy.Holds { base with holds := fun _ _ => True } (.param 0) v :=
  ⟨fun h => @NTy.Holds.deepHolds { base with holds := fun _ _ => False } (.param 0) v h,
    @NTy.Holds.intro { base with holds := fun _ _ => True } (.param 0) v trivial⟩

set_option linter.unusedVariables false in
/-- Non-reduction check: `NTy.Holds` under distinct `Carriers.own` predicates does not
reduce to definitionally equal propositions. -/
example (base : Carriers) (v : @NTy.carrier base (.struct source .nil .nil)) : True := by
  fail_if_success
    have : @NTy.Holds { base with own := fun _ _ _ => True } (.struct source .nil .nil) v =
        @NTy.Holds { base with own := fun _ _ _ => False } (.struct source .nil .nil) v := rfl
  trivial

/-- Positive: On `structTy`, `carriersOwnFalse` and `carriersOwnTrue` share the identical
plain carrier type definitionally. -/
theorem carrier_structTy_indep_own :
    @NTy.carrier carriersOwnFalse structTy = @NTy.carrier carriersOwnTrue structTy := rfl

/-- Positive: On concrete argument rows, the plain carrier type of `NTy.nominal` is
definitionally independent of `Carriers.own`. -/
example (base : Carriers) :
    @NTy.carrier { base with own := fun _ _ _ => True }
        (NTy.nominal source (.cons (.param 0) .nil) kind) =
      @NTy.carrier { base with own := fun _ _ _ => False }
        (NTy.nominal source (.cons (.param 0) .nil) kind) := rfl

/-- Positive: Across every argument row, `NTy.carrier` of `NTy.nominal` is independent
of `Carriers.own`. -/
example (base : Carriers) :
    @NTy.carrier { base with own := fun _ _ _ => True } (NTy.nominal source arguments kind) =
      @NTy.carrier { base with own := fun _ _ _ => False }
        (NTy.nominal source arguments kind) :=
  (NTy.carrier_withOwn base _ (NTy.nominal source arguments kind)).trans
    (NTy.carrier_withOwn base _ (NTy.nominal source arguments kind)).symm

/-- Positive: Nor does the plain carrier type of `NTy.nominal` depend on `Carriers.holds`. -/
example (base : Carriers) :
    @NTy.carrier { base with holds := fun _ _ => True }
        (NTy.nominal source (.cons (.param 0) .nil) kind) =
      @NTy.carrier { base with holds := fun _ _ => False }
        (NTy.nominal source (.cons (.param 0) .nil) kind) := rfl

/-- Negative semantic kernel control (first): substitution does not conflate distinct
kinds on `NTy.nominal`. -/
theorem subst_nominal_productive_ne (θ : NRow) :
    NTy.subst θ (NTy.nominal source arguments ⟨true, [], .owned, true⟩) ≠
      NTy.subst θ (NTy.nominal source arguments ⟨false, [], .owned, true⟩) := by
  intro h
  change NTy.nominal source (NRow.subst θ arguments) ⟨true, [], .owned, true⟩ =
    NTy.nominal source (NRow.subst θ arguments) ⟨false, [], .owned, true⟩ at h
  injection h with _ _ hkind
  exact absurd hkind (by decide)

set_option linter.unusedVariables false in
/-- Non-reduction check: substitution at distinct kinds does not reduce to
definitionally equal types. -/
example (θ : NRow) : True := by
  fail_if_success
    have : NTy.subst θ (NTy.nominal source arguments ⟨true, [], .owned, true⟩) =
        NTy.subst θ (NTy.nominal source arguments ⟨false, [], .owned, true⟩) := rfl
  trivial

/-- Positive: Substitution reaches a nominal through its type arguments alone. -/
example (θ : NRow) :
    NTy.subst θ (NTy.nominal source arguments kind) =
      NTy.nominal source (NRow.subst θ arguments) kind := rfl

set_option linter.unusedVariables false in
/-- Negative non-reduction check (first): `NTy.plainFallback` is opaque and does not
definitionally reduce to a concrete value. -/
example [outer : Carriers] : True := by
  fail_if_success
    have : (@NTy.plainFallback outer .bool rfl) = false := rfl
  trivial

/-- Positive: `NTy.plainFallback` converts definitionally across `Carriers.instantiate`
once the plain carriers coincide. -/
example [outer : Carriers] :
    let θ : TypeArgs := ⟨.cons (.int 64 false) .nil, rfl⟩
    let τ : NTy := .struct source (.cons (.param 0) .nil) (.cons (.param 0) .nil)
    @NTy.plainFallback (Carriers.instantiate θ outer) τ rfl =
      @NTy.plainFallback outer (τ.subst θ.1) rfl := rfl

private abbrev functionRow : NRow := .cons (.function .nil ⟨[], []⟩ .nil) .nil
private abbrev kindTokens (kind : NominalKind) : Nat :=
  NTy.siteTokens (.nominal testHandle .nil kind) 0

/-- Positive and negative: a declared type at a function-sort argument has values
where its kind does not demand that argument, and none where it does, a function
type alone having no value of its own. -/
theorem inhabitable_demanded_function :
    (NTy.nominal testHandle functionRow ⟨true, [], .owned, false⟩).inhabitable = true ∧
      (NTy.nominal testHandle functionRow ⟨true, [0], .owned, false⟩).inhabitable = false :=
  ⟨rfl, rfl⟩

/-- Positive: each of the kind's four components is read into a declared type's
token stream, so the stream differs where any one of them differs. -/
theorem siteTokens_kind_components :
    kindTokens ⟨false, [0], .owned, true⟩ ≠ kindTokens ⟨true, [0], .owned, true⟩ ∧
      kindTokens ⟨true, [], .owned, true⟩ ≠ kindTokens ⟨true, [0], .owned, true⟩ ∧
        kindTokens ⟨true, [0], .shared, true⟩ ≠ kindTokens ⟨true, [0], .owned, true⟩ ∧
          kindTokens ⟨true, [0], .owned, false⟩ ≠ kindTokens ⟨true, [0], .owned, true⟩ := by
  decide

open Lean Elab Command in
run_cmd do
  for name in [``holds_depends_on_own_zero, ``holds_depends_on_own,
      ``holds_depends_on_param_holds, ``carrier_structTy_indep_own,
      ``subst_nominal_productive_ne, ``inhabitable_demanded_function,
      ``siteTokens_kind_components] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end NominalCertification
end VerexIR.Proofs.Denote
