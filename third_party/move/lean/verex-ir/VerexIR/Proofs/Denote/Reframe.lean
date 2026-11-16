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

private theorem congrArg2 {α β γ : Sort _} (f : α → β → γ) {a₁ a₂ : α} {b₁ b₂ : β}
    (ha : a₁ = a₂) (hb : b₁ = b₂) : f a₁ b₁ = f a₂ b₂ := by
  subst ha; subst hb; rfl

private theorem heq_unary {α₁ α₂ : Type} {F : Type → Type} {G : Type → Type}
    (f : ∀ {α : Type}, F α → G α) {c₁ : F α₁} {c₂ : F α₂}
    (hα : α₁ = α₂) (hc : HEq c₁ c₂) : HEq (f c₁) (f c₂) := by
  subst hα; cases hc; exact HEq.rfl

private theorem heq_binary {α₁ α₂ β₁ β₂ : Type}
    {F₁ : Type → Type} {F₂ : Type → Type} {G : Type → Type → Type}
    (f : ∀ {α β : Type}, F₁ α → F₂ β → G α β)
    {c₁ : F₁ α₁} {c₂ : F₁ α₂} {d₁ : F₂ β₁} {d₂ : F₂ β₂}
    (hα : α₁ = α₂) (hβ : β₁ = β₂) (hc : HEq c₁ c₂) (hd : HEq d₁ d₂) :
    HEq (f c₁ d₁) (f c₂ d₂) := by
  subst hα; subst hβ; cases hc; cases hd; exact HEq.rfl

private theorem heq_dep_unary {S : Type _} {s₁ s₂ : S}
    {A : S → Type _} {C : S → Type _}
    (f : ∀ (s : S), A s → C s)
    {a₁ : A s₁} {a₂ : A s₂}
    (hs : s₁ = s₂) (ha : HEq a₁ a₂) :
    HEq (f s₁ a₁) (f s₂ a₂) := by
  subst hs; cases ha; exact HEq.rfl

private theorem codec_heq {V₁ V₂ : Type} {c₁ : Codec V₁ RuntimeValue} {c₂ : Codec V₂ RuntimeValue}
    (hV : V₁ = V₂) (henc : HEq c₁.encode c₂.encode) (hdec : HEq c₁.decode? c₂.decode?) :
    HEq c₁ c₂ := by
  cases c₁; cases c₂; dsimp at henc hdec
  subst hV; cases henc; cases hdec; exact HEq.rfl

theorem FunctionCarriers.eq_with_own {unit : Validation.ValidatedUnit}
    (functions base : FunctionCarriers unit)
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals) :
    functions = { base with own := functions.own } := by
  cases functions; cases base
  dsimp at sameCarrier sameFunctions sameNominals
  subst sameCarrier; cases sameFunctions; subst sameNominals
  congr <;> exact Subsingleton.elim _ _

mutual
theorem NTy.carrier_with_own (Φ : Carriers) (newOwn : StructHandle → NRow → RuntimeValue → Prop) :
    (τ : NTy) → @NTy.carrier { Φ with own := newOwn } τ = @NTy.carrier Φ τ
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char
  | .param _ | .function _ _ _ => rfl
  | .tuple row | .struct _ _ row => HList.carrier_with_own Φ newOwn row
  | .enum _ _ names rows _ => variantCarrier.carrier_with_own Φ newOwn names rows
  | .slice e | .vector e => congrArg SpecVector (NTy.carrier_with_own Φ newOwn e)
  | .array e n => congrArg (fun α => SpecArray α n) (NTy.carrier_with_own Φ newOwn e)
  | .table k v => congrArg SpecEntries (congrArg2 Prod (NTy.carrier_with_own Φ newOwn k) (NTy.carrier_with_own Φ newOwn v))
  | .ghost s | .observed s => NTy.carrier_with_own Φ newOwn s
  | .sliceRef e => congrArg SpecSliceRef (NTy.carrier_with_own Φ newOwn e)
  | .ref r => congrArg2 Prod (NTy.carrier_with_own Φ newOwn r) (NTy.carrier_with_own Φ newOwn r)
  | .nominal src args prod => congrArg (Φ.nominals.nominal src prod) (NRow.sorts_with_own Φ newOwn args)

theorem HList.carrier_with_own (Φ : Carriers) (newOwn : StructHandle → NRow → RuntimeValue → Prop) :
    (row : NRow) → @HList { Φ with own := newOwn } row = @HList Φ row
  | .nil => rfl
  | .cons τ rest => congrArg2 Prod (NTy.carrier_with_own Φ newOwn τ) (HList.carrier_with_own Φ newOwn rest)

theorem variantCarrier.carrier_with_own (Φ : Carriers) (newOwn : StructHandle → NRow → RuntimeValue → Prop) :
    (names : List String) → (rows : NRows) →
    @variantCarrier { Φ with own := newOwn } names rows = @variantCarrier Φ names rows
  | _, .nil => rfl
  | [], .cons _ _ => rfl
  | _ :: ns, .cons r rest => congrArg2 Sum (HList.carrier_with_own Φ newOwn r) (variantCarrier.carrier_with_own Φ newOwn ns rest)

theorem NRow.sorts_with_own (Φ : Carriers) (newOwn : StructHandle → NRow → RuntimeValue → Prop) :
    (row : NRow) → @NRow.sorts { Φ with own := newOwn } row = @NRow.sorts Φ row
  | .nil => rfl
  | .cons τ rest => congrArg2 List.cons (NTy.carrier_with_own Φ newOwn τ) (NRow.sorts_with_own Φ newOwn rest)
end

mutual
theorem NTy.codec_heq_with_own (Φ : Carriers) (newOwn : StructHandle → NRow → RuntimeValue → Prop) :
    (τ : NTy) → HEq (@NTy.codec { Φ with own := newOwn } τ) (@NTy.codec Φ τ)
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char
  | .param _ | .function _ _ _ => HEq.rfl
  | .tuple elements =>
      heq_unary (F := (Codec · (List RuntimeValue))) (G := (Codec · RuntimeValue))
        (fun c => Codec.tuple c)
        (HList.carrier_with_own Φ newOwn elements) (rowCodec_heq_with_own Φ newOwn elements)
  | .struct src _ fields =>
      heq_unary (F := (Codec · (List RuntimeValue))) (G := (Codec · RuntimeValue))
        (fun c => Codec.nominalRow src none c)
        (HList.carrier_with_own Φ newOwn fields) (rowCodec_heq_with_own Φ newOwn fields)
  | .enum src _ names rows dist =>
      variantCodec_heq_with_own Φ newOwn src names rows dist
  | .slice e | .vector e =>
      heq_unary (F := (Codec · RuntimeValue)) (G := fun α => Codec (SpecVector α) RuntimeValue)
        (fun c => Codec.boundedVector c)
        (NTy.carrier_with_own Φ newOwn e) (NTy.codec_heq_with_own Φ newOwn e)
  | .array e n =>
      heq_unary (F := (Codec · RuntimeValue)) (G := fun α => Codec (SpecArray α n) RuntimeValue)
        (fun c => Codec.array c n)
        (NTy.carrier_with_own Φ newOwn e) (NTy.codec_heq_with_own Φ newOwn e)
  | .table k v =>
      heq_binary (F₁ := (Codec · RuntimeValue)) (F₂ := (Codec · RuntimeValue))
        (G := fun α β => Codec (SpecEntries (α × β)) RuntimeValue)
        (fun ck cv => Codec.entries ck cv)
        (NTy.carrier_with_own Φ newOwn k) (NTy.carrier_with_own Φ newOwn v)
        (NTy.codec_heq_with_own Φ newOwn k) (NTy.codec_heq_with_own Φ newOwn v)
  | .ghost s =>
      heq_unary (F := (Codec · RuntimeValue)) (G := (Codec · RuntimeValue))
        (fun c => Codec.ghost c)
        (NTy.carrier_with_own Φ newOwn s) (NTy.codec_heq_with_own Φ newOwn s)
  | .observed s => NTy.codec_heq_with_own Φ newOwn s
  | .sliceRef e =>
      heq_unary (F := (Codec · RuntimeValue)) (G := fun α => Codec (SpecSliceRef α) RuntimeValue)
        (fun c => Codec.sliceRef c)
        (NTy.carrier_with_own Φ newOwn e) (NTy.codec_heq_with_own Φ newOwn e)
  | .ref r =>
      heq_unary (F := (Codec · RuntimeValue)) (G := fun α => Codec (α × α) RuntimeValue)
        (fun c => Codec.prophecyPair c)
        (NTy.carrier_with_own Φ newOwn r) (NTy.codec_heq_with_own Φ newOwn r)
  | .nominal src args prod => by
      dsimp only [NTy.codec, Carriers.nominalCodec]
      exact heq_dep_unary (S := List Type) (A := SortCodecs)
        (C := fun s => Codec (Φ.nominals.nominal src prod s) RuntimeValue)
        (fun s sc => @Carriers.nominalCodec Φ src prod s sc)
        (NRow.sorts_with_own Φ newOwn args) (sortCodecs_heq_with_own Φ newOwn args)

theorem rowCodec_heq_with_own (Φ : Carriers) (newOwn : StructHandle → NRow → RuntimeValue → Prop) :
    (row : NRow) → HEq (@rowCodec { Φ with own := newOwn } row) (@rowCodec Φ row)
  | .nil => HEq.rfl
  | .cons τ rest =>
      heq_binary (F₁ := (Codec · RuntimeValue)) (F₂ := (Codec · (List RuntimeValue)))
        (G := fun α β => Codec (α × β) (List RuntimeValue))
        (fun ch ct => Codec.tupleCons ch ct)
        (NTy.carrier_with_own Φ newOwn τ) (HList.carrier_with_own Φ newOwn rest)
        (NTy.codec_heq_with_own Φ newOwn τ) (rowCodec_heq_with_own Φ newOwn rest)

theorem sortCodecs_heq_with_own (Φ : Carriers) (newOwn : StructHandle → NRow → RuntimeValue → Prop) :
    (row : NRow) → HEq (@NRow.sortCodecs { Φ with own := newOwn } row) (@NRow.sortCodecs Φ row)
  | .nil => HEq.rfl
  | .cons τ rest => by
      dsimp only [NRow.sortCodecs]
      have hc := NTy.carrier_with_own Φ newOwn τ
      have hs := NRow.sorts_with_own Φ newOwn rest
      have hd := NTy.codec_heq_with_own Φ newOwn τ
      have hsd := sortCodecs_heq_with_own Φ newOwn rest
      exact (show ∀ {A₁ A₂ : Type} {S₁ S₂ : List Type} (c₁ : Codec A₁ RuntimeValue) (c₂ : Codec A₂ RuntimeValue)
          (sc₁ : SortCodecs S₁) (sc₂ : SortCodecs S₂),
          A₁ = A₂ → S₁ = S₂ → HEq c₁ c₂ → HEq sc₁ sc₂ →
          HEq ((c₁, sc₁) : Codec A₁ RuntimeValue × SortCodecs S₁) ((c₂, sc₂) : Codec A₂ RuntimeValue × SortCodecs S₂) from
        fun _ _ _ _ hA hS hc hsc => by subst hA; subst hS; cases hc; cases hsc; exact HEq.rfl)
        _ _ _ _ hc hs hd hsd

theorem variantCodec_heq_with_own (Φ : Carriers) (newOwn : StructHandle → NRow → RuntimeValue → Prop)
    (src : StructHandle) : (names : List String) → (rows : NRows) → (dist : names.Nodup) →
    HEq (@variantCodec { Φ with own := newOwn } src names rows (@rowCodecs { Φ with own := newOwn } rows) dist)
      (@variantCodec Φ src names rows (@rowCodecs Φ rows) dist)
  | [], .nil, _ => by dsimp only [variantCodec]; exact HEq.rfl
  | _ :: _, .nil, _ => by dsimp only [variantCodec]; exact HEq.rfl
  | [], .cons _ _, _ =>
      codec_heq rfl
        (by dsimp only [variantCodec, variantEncode]; exact HEq.rfl)
        (by dsimp only [variantCodec, variantDecode?]; exact HEq.rfl)
  | name :: ns, .cons fields rest, dist => by
      have hc := HList.carrier_with_own Φ newOwn fields
      have hv := variantCarrier.carrier_with_own Φ newOwn ns rest
      have hd := rowCodec_heq_with_own Φ newOwn fields
      have hih := variantCodec_heq_with_own Φ newOwn src ns rest (List.nodup_cons.mp dist).2
      have hencRec : HEq (@variantEncode { Φ with own := newOwn } src ns rest (@rowCodecs { Φ with own := newOwn } rest))
          (@variantEncode Φ src ns rest (@rowCodecs Φ rest)) :=
        (show ∀ {V₁ V₂ : Type} (c₁ : Codec V₁ RuntimeValue) (c₂ : Codec V₂ RuntimeValue),
            V₁ = V₂ → HEq c₁ c₂ → HEq c₁.encode c₂.encode from
          fun _ _ hV hc => by subst hV; cases hc; exact HEq.rfl) _ _ hv hih
      have hdecRec : HEq (@variantDecode? { Φ with own := newOwn } src ns rest (@rowCodecs { Φ with own := newOwn } rest))
          (@variantDecode? Φ src ns rest (@rowCodecs Φ rest)) :=
        (show ∀ {V₁ V₂ : Type} (c₁ : Codec V₁ RuntimeValue) (c₂ : Codec V₂ RuntimeValue),
            V₁ = V₂ → HEq c₁ c₂ → HEq c₁.decode? c₂.decode? from
          fun _ _ hV hc => by subst hV; cases hc; exact HEq.rfl) _ _ hv hih
      refine codec_heq (congrArg2 Sum hc hv) ?_ ?_
      · dsimp only [variantCodec, variantEncode, rowCodecs]
        exact (show ∀ {A₁ A₂ V₁ V₂ : Type}
            (cf₁ : Codec A₁ (List RuntimeValue)) (cf₂ : Codec A₂ (List RuntimeValue))
            (e₁ : V₁ → RuntimeValue) (e₂ : V₂ → RuntimeValue),
            A₁ = A₂ → V₁ = V₂ → HEq cf₁ cf₂ → HEq e₁ e₂ →
            HEq (fun value : A₁ ⊕ V₁ => Sum.elim (fun f => (Codec.nominalRow src (some name) cf₁).encode f) (fun l => e₁ l) value)
                (fun value : A₂ ⊕ V₂ => Sum.elim (fun f => (Codec.nominalRow src (some name) cf₂).encode f) (fun l => e₂ l) value) from
          fun _ _ _ _ hA hV hcf he => by subst hA; subst hV; cases hcf; cases he; exact HEq.rfl)
          _ _ _ _ hc hv hd hencRec
      · dsimp only [variantCodec, variantDecode?, rowCodecs]
        exact (show ∀ {A₁ A₂ V₁ V₂ : Type}
            (cf₁ : Codec A₁ (List RuntimeValue)) (cf₂ : Codec A₂ (List RuntimeValue))
            (d₁ : RuntimeValue → Option V₁) (d₂ : RuntimeValue → Option V₂),
            A₁ = A₂ → V₁ = V₂ → HEq cf₁ cf₂ → HEq d₁ d₂ →
            HEq (fun runtime : RuntimeValue => match runtime with
                  | .nominal actualSource (some actualVariant) _ =>
                      if actualSource = src ∧ actualVariant = name then
                        Option.map (@Sum.inl A₁ V₁) ((Codec.nominalRow src (some name) cf₁).decode? runtime)
                      else Option.map (@Sum.inr A₁ V₁) (d₁ runtime)
                  | _ => (none : Option (A₁ ⊕ V₁)))
                (fun runtime : RuntimeValue => match runtime with
                  | .nominal actualSource (some actualVariant) _ =>
                      if actualSource = src ∧ actualVariant = name then
                        Option.map (@Sum.inl A₂ V₂) ((Codec.nominalRow src (some name) cf₂).decode? runtime)
                      else Option.map (@Sum.inr A₂ V₂) (d₂ runtime)
                  | _ => (none : Option (A₂ ⊕ V₂))) from
          fun _ _ _ _ hA hV hcf hd => by subst hA; subst hV; cases hcf; cases hd; exact HEq.rfl)
          _ _ _ _ hc hv hd hdecRec
end

theorem NTy.encode_with_own (Φ : Carriers) (newOwn : StructHandle → NRow → RuntimeValue → Prop)
    (τ : NTy) (v : @NTy.carrier { Φ with own := newOwn } τ) :
    @NTy.encode Φ τ (cast (NTy.carrier_with_own Φ newOwn τ) v) =
      @NTy.encode { Φ with own := newOwn } τ v := by
  dsimp only [NTy.encode]
  exact (show ∀ {A₁ A₂ : Type} (c₁ : Codec A₁ RuntimeValue) (c₂ : Codec A₂ RuntimeValue)
      (hA : A₁ = A₂) (hc : HEq c₁ c₂) (v : A₁), c₂.encode (cast hA v) = c₁.encode v from
    fun _ _ hA hc v => by subst hA; cases hc; rfl)
    _ _ (NTy.carrier_with_own Φ newOwn τ) (NTy.codec_heq_with_own Φ newOwn τ) v

/-- A type over a declaration's parameters at an instance row resolves alike
at a frame and at the frame the row's resolutions induce. Stated of any
instance row: the frame's resolution of each entry substitutes what the row
itself substitutes. -/
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

/-- The instance row of a declaration's own parameters substitutes a type over
them to itself, so the general resolution reads as it did. -/
theorem Skolems.resolve_atArguments_params {unit : Validation.ValidatedUnit}
    {functions : FunctionCarriers unit}
    (Θ : @Skolems unit functions) {params : NRow} (own : params.isParamsFrom 0 = true) {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true) :
    τ.subst (params.map Θ.resolve) = Θ.resolve τ := by
  rw [Θ.resolve_atArguments params bounded, NTy.subst_params own bounded]

private theorem runtimeCarrier_with_own {unit : Validation.ValidatedUnit}
    {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals) (ρ : NTy) :
    @NTy.carrier (@Carriers.runtime unit functions) ρ =
      @NTy.carrier (@Carriers.runtime unit base) ρ := by
  rw [FunctionCarriers.eq_with_own functions base sameCarrier sameFunctions sameNominals]
  exact NTy.carrier_with_own (@Carriers.runtime unit base) functions.own ρ

private theorem runtimeEncode_eq_of_eq {unit : Validation.ValidatedUnit}
    {functions base : FunctionCarriers unit}
    {ownFn : StructHandle → NRow → RuntimeValue → Prop}
    (hf : functions = { base with own := ownFn }) (ρ : NTy)
    (h : @NTy.carrier (@Carriers.runtime unit functions) ρ =
      @NTy.carrier (@Carriers.runtime unit base) ρ)
    (v : @NTy.carrier (@Carriers.runtime unit functions) ρ) :
    @NTy.encode (@Carriers.runtime unit base) ρ (cast h v) =
      @NTy.encode (@Carriers.runtime unit functions) ρ v := by
  subst hf
  exact NTy.encode_with_own (@Carriers.runtime unit base) ownFn ρ v

private theorem runtimeEncode_with_own {unit : Validation.ValidatedUnit}
    {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals) (ρ : NTy)
    (v : @NTy.carrier (@Carriers.runtime unit functions) ρ) :
    @NTy.encode (@Carriers.runtime unit base) ρ
        (cast (runtimeCarrier_with_own sameCarrier sameFunctions sameNominals ρ) v) =
      @NTy.encode (@Carriers.runtime unit functions) ρ v :=
  runtimeEncode_eq_of_eq
    (FunctionCarriers.eq_with_own functions base sameCarrier sameFunctions sameNominals)
    ρ _ v

theorem Skolems.runtimeCarrierEq {unit : Validation.ValidatedUnit}
    {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals)
    (Θ : @Skolems unit functions) {params : NRow} (own : params.isParamsFrom 0 = true)
    {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true) :
    @NTy.carrier (@Carriers.runtime unit functions) (Θ.resolve τ) =
      @NTy.carrier (@Carriers.runtime unit base) (τ.subst (params.map Θ.resolve)) :=
  (runtimeCarrier_with_own sameCarrier sameFunctions sameNominals (Θ.resolve τ)).trans
    (congrArg (@NTy.carrier (@Carriers.runtime unit base))
      (Skolems.resolve_atArguments_params Θ own bounded).symm)

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
    @NTy.carrier (@Carriers.runtime unit functions) (Θ.resolve (τ.subst params)) =
      @NTy.carrier (@Carriers.runtime unit base) (τ.subst (params.map Θ.resolve)) :=
  (runtimeCarrier_with_own sameCarrier sameFunctions sameNominals
      (Θ.resolve (τ.subst params))).trans
    (congrArg (@NTy.carrier (@Carriers.runtime unit base))
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
    (value : @NTy.carrier Θ.toCarriers (τ.subst params)) :
    @NTy.carrier (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ :=
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
    (value : @NTy.carrier Θ.toCarriers (τ.subst params)) :
    @NTy.encode (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable
          bounded value) =
      @NTy.encode Θ.toCarriers (τ.subst params) value := by
  unfold Skolems.reframeAtInstance
  let target := Skolems.atArguments unit base (params.map Θ.resolve) inhabitable
  rw [← @Skolems.encode_toRuntime unit base target τ,
    @Skolems.toRuntime_ofRuntime unit base target τ]
  change @NTy.encode (@Carriers.runtime unit base) (τ.subst (params.map Θ.resolve))
      (cast (Skolems.runtimeCarrierEqAtInstance sameCarrier sameFunctions sameNominals Θ params
          bounded)
        (@Skolems.toRuntime unit functions Θ (τ.subst params) value)) =
    @NTy.encode Θ.toCarriers (τ.subst params) value
  have hres := (Θ.resolve_atArguments params bounded).symm
  have hsplit : cast (Skolems.runtimeCarrierEqAtInstance sameCarrier sameFunctions sameNominals
          Θ params bounded)
        (@Skolems.toRuntime unit functions Θ (τ.subst params) value) =
      cast (congrArg (@NTy.carrier (@Carriers.runtime unit base)) hres)
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
    (value : @NTy.carrier Θ.toCarriers τ) :
    @NTy.carrier (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ :=
  Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ params inhabitable bounded
    (cast (congrArg (@NTy.carrier Θ.toCarriers) (NTy.subst_params own bounded).symm) value)

@[vir_denote_norm] theorem Skolems.encode_reframeAtBase {unit : Validation.ValidatedUnit}
    {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals)
    (Θ : @Skolems unit functions) {params : NRow} (own : params.isParamsFrom 0 = true)
    (inhabitable : (params.map Θ.resolve).inhabitable = true)
    {τ : NTy} (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (value : @NTy.carrier Θ.toCarriers τ) :
    @NTy.encode (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers τ
        (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value) =
      @NTy.encode Θ.toCarriers τ value := by
  unfold Skolems.reframeAtBase
  rw [Skolems.encode_reframeAtInstance]
  exact NTy.encode_castCarrier (NTy.subst_params own bounded).symm _ value

end VerexIR.Proofs.Denote
