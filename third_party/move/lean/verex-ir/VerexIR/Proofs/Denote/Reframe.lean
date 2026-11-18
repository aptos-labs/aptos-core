-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Resources
import VerexIR.Proofs.Denote.Attr

/-!
# The move of a value to the frame a declaration states its clauses at

A unit's function values and its base function values differ in the `own`
predicate only, so a type has one carrier and one encoding at both. From that,
`Skolems.reframeAtInstance` carries a frame's value of a declaration's type at
an instance row to the frame the row's resolutions induce over the base
function values, where the declaration's clauses are stated, and keeps its
encoding. `Skolems.reframeAtBase` is the move at the declaration's own
parameter row.
-/

namespace VerexIR.Proofs.Denote

/-- Runtime operations agree independently of the own predicates. -/
theorem Sorts.runtime_eq {unit : Validation.ValidatedUnit}
    (functions base : FunctionCarriers unit)
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals) :
    @Sorts.runtime unit functions = @Sorts.runtime unit base := by
  cases functions; cases base
  dsimp at sameCarrier sameFunctions sameNominals
  subst sameCarrier; cases sameFunctions; subst sameNominals
  rfl

theorem Skolems.resolve_atArguments {unit : Validation.ValidatedUnit}
    {functions : FunctionCarriers unit}
    (Θ : @Skolems unit functions) (params : NRow) {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true) :
    τ.subst (params.map Θ.resolve) = Θ.resolve (τ.subst params) := by
  letI := functions
  rw [Θ.resolve_eq (τ.subst params), NTy.substWith_subst]
  refine (NTy.subst_eq_substWith (params.map Θ.resolve)
      (fun index => Θ.resolve (params.argument index)) params.length
      (fun index below => NRow.getD_map Θ.resolve params index below (.param index))
      τ bounded).trans
    (NTy.substWith_unmarked_congr (fun index => ?_) τ)
  rw [Θ.resolve_eq (params.argument index)]
  generalize params.argument index = entry
  cases entry <;> first | rfl | exact NTy.unmark_unmark _

private theorem runtimeCarrier_with_own {unit : Validation.ValidatedUnit}
    {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals) (ρ : NTy) :
    @NTy.carrier ((@Carriers.runtime unit functions)).toSorts ρ =
      @NTy.carrier ((@Carriers.runtime unit base)).toSorts ρ :=
  congrArg (fun operations => @NTy.carrier operations ρ)
    (Sorts.runtime_eq functions base sameCarrier sameFunctions sameNominals)

private theorem runtimeEncode_eq_of_eq {operations base : Sorts}
    (same : operations = base) (ρ : NTy)
    (h : @NTy.carrier operations ρ = @NTy.carrier base ρ)
    (v : @NTy.carrier operations ρ) :
    @NTy.encode base ρ (cast h v) = @NTy.encode operations ρ v := by
  cases same
  rfl

private theorem runtimeEncode_with_own {unit : Validation.ValidatedUnit}
    {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals) (ρ : NTy)
    (v : @NTy.carrier ((@Carriers.runtime unit functions)).toSorts ρ) :
    @NTy.encode ((@Carriers.runtime unit base)).toSorts ρ
        (cast (runtimeCarrier_with_own sameCarrier sameFunctions sameNominals ρ) v) =
      @NTy.encode ((@Carriers.runtime unit functions)).toSorts ρ v :=
  runtimeEncode_eq_of_eq
    (Sorts.runtime_eq functions base sameCarrier sameFunctions sameNominals) ρ _ v

/-- The carriers of a type over a declaration's parameters at an instance row:
what the frame reads the instance as, and what the frame the row's resolutions
induce reads the declaration's own type as. -/
theorem Skolems.runtimeCarrierEqAtInstance {unit : Validation.ValidatedUnit}
    {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals)
    (Θ : @Skolems unit functions) (params : NRow)
    {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true) :
    @NTy.carrier ((@Carriers.runtime unit functions)).toSorts (Θ.resolve (τ.subst params)) =
      @NTy.carrier ((@Carriers.runtime unit base)).toSorts (τ.subst (params.map Θ.resolve)) :=
  (runtimeCarrier_with_own sameCarrier sameFunctions sameNominals
      (Θ.resolve (τ.subst params))).trans
    (congrArg (@NTy.carrier ((@Carriers.runtime unit base)).toSorts)
      (Θ.resolve_atArguments params bounded).symm)

/-- A frame's value of a declaration's type at an instance row, at the frame
the row's resolutions induce, where the declaration states its clauses: the
same runtime value. The instance row is any row the declaration's arguments
are, so the value's type is what the row substitutes. -/
noncomputable def Skolems.reframeAtInstance {unit : Validation.ValidatedUnit}
    {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals)
    (Θ : @Skolems unit functions) (params : NRow)
    (inhabitable : (params.map Θ.resolve).inhabitable = true)
    {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (τ.subst params)) :
    @NTy.carrier ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts τ :=
  (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).ofRuntime τ
    (cast (Skolems.runtimeCarrierEqAtInstance sameCarrier sameFunctions sameNominals Θ params bounded)
      (@Skolems.toRuntime unit functions Θ (τ.subst params) value))

/-- Moving a declaration's value from an instance row to the frame the row
induces keeps its encoding, as the two frames read the same runtime value. -/
theorem Skolems.encode_reframeAtInstance {unit : Validation.ValidatedUnit}
    {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals)
    (Θ : @Skolems unit functions) (params : NRow)
    (inhabitable : (params.map Θ.resolve).inhabitable = true)
    {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (τ.subst params)) :
    @NTy.encode ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts τ
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
          bounded value) =
      @NTy.encode (Θ.toCarriers).toSorts (τ.subst params) value := by
  unfold Skolems.reframeAtInstance
  let target := Skolems.atArguments unit base (params.map Θ.resolve) inhabitable
  rw [← @Skolems.encode_toRuntime unit base target τ,
    @Skolems.toRuntime_ofRuntime unit base target τ]
  change @NTy.encode ((@Carriers.runtime unit base)).toSorts (τ.subst (params.map Θ.resolve))
      (cast (Skolems.runtimeCarrierEqAtInstance sameCarrier sameFunctions sameNominals Θ params
          bounded)
        (@Skolems.toRuntime unit functions Θ (τ.subst params) value)) =
    @NTy.encode (Θ.toCarriers).toSorts (τ.subst params) value
  have hres := (Θ.resolve_atArguments params bounded).symm
  have hsplit : cast (Skolems.runtimeCarrierEqAtInstance sameCarrier sameFunctions sameNominals
          Θ params bounded)
        (@Skolems.toRuntime unit functions Θ (τ.subst params) value) =
      cast (congrArg (@NTy.carrier ((@Carriers.runtime unit base)).toSorts) hres)
        (cast (runtimeCarrier_with_own sameCarrier sameFunctions sameNominals
            (Θ.resolve (τ.subst params)))
          (@Skolems.toRuntime unit functions Θ (τ.subst params) value)) := by
    simp [cast_cast]
  rw [hsplit, NTy.encode_castCarrier hres,
    runtimeEncode_with_own sameCarrier sameFunctions sameNominals (Θ.resolve (τ.subst params))]
  exact @Skolems.encode_toRuntime unit functions Θ (τ.subst params) value

/-- The move at the instance row of the declaration's own parameters, which
substitutes a type over them to itself (`NTy.subst_params`): the general move
at that row. -/
noncomputable def Skolems.reframeAtBase {unit : Validation.ValidatedUnit}
    {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals)
    (Θ : @Skolems unit functions) {params : NRow} (own : params.isParamsFrom 0 = true)
    (inhabitable : (params.map Θ.resolve).inhabitable = true)
    {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts τ) :
    @NTy.carrier ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts τ :=
  Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable bounded
    (cast (congrArg (@NTy.carrier (Θ.toCarriers).toSorts) (NTy.subst_params own bounded).symm) value)

@[vir_denote_norm] theorem Skolems.encode_reframeAtBase {unit : Validation.ValidatedUnit}
    {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals)
    (Θ : @Skolems unit functions) {params : NRow} (own : params.isParamsFrom 0 = true)
    (inhabitable : (params.map Θ.resolve).inhabitable = true)
    {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts τ) :
    @NTy.encode ((Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers).toSorts τ
        (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value) =
      @NTy.encode (Θ.toCarriers).toSorts τ value := by
  unfold Skolems.reframeAtBase
  rw [Skolems.encode_reframeAtInstance]
  exact NTy.encode_castCarrier (NTy.subst_params own bounded).symm _ value

end VerexIR.Proofs.Denote
