-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Mono.Transform

/-!
# Type helper lemmas for monomorphization correctness

The executable pass uses generated `BEq` instances because its worklists are
computable. These lemmas expose only the equality facts required by the proof;
they do not add classical equality to the transformation.
-/

namespace MoveModel.IR

/-- Pointwise equality of the runtime encodings of two type-argument lists. -/
def TypeArgsTagEq (lhs rhs : List Ty) : Prop :=
  lhs.map Ty.toTag = rhs.map Ty.toTag

theorem TypeArgsTagEq.refl (args : List Ty) : TypeArgsTagEq args args := rfl

theorem TypeArgsTagEq.symm {lhs rhs : List Ty}
    (h : TypeArgsTagEq lhs rhs) : TypeArgsTagEq rhs lhs := Eq.symm h

theorem TypeArgsTagEq.trans {first second third : List Ty}
    (h₁ : TypeArgsTagEq first second) (h₂ : TypeArgsTagEq second third) :
    TypeArgsTagEq first third := Eq.trans h₁ h₂

/-- Tag-equivalent substitutions have the same arity. -/
theorem TypeArgsTagEq.length_eq {lhs rhs : List Ty}
    (h : TypeArgsTagEq lhs rhs) : lhs.length = rhs.length := by
  have := congrArg List.length h
  simpa [TypeArgsTagEq] using this

/-- Corresponding optional arguments have the same runtime tag. -/
theorem TypeArgsTagEq.getElem?_map_toTag {lhs rhs : List Ty}
    (h : TypeArgsTagEq lhs rhs) (index : Nat) :
    lhs[index]?.map Ty.toTag = rhs[index]?.map Ty.toTag := by
  have := congrArg (fun tags => tags[index]?) h
  simpa [TypeArgsTagEq, List.getElem?_map] using this

mutual

/-- Substituting tag-equivalent arguments into a type produces the same
runtime type tag. -/
theorem Ty.toTag_instantiate_eq {lhs rhs : List Ty}
    (hargs : TypeArgsTagEq lhs rhs) :
    ∀ ty : Ty, (ty.instantiate lhs).toTag = (ty.instantiate rhs).toTag
  | .bool => by simp [Ty.instantiate, Ty.toTag]
  | .uint _ => by simp [Ty.instantiate, Ty.toTag]
  | .sint _ => by simp [Ty.instantiate, Ty.toTag]
  | .address => by simp [Ty.instantiate, Ty.toTag]
  | .signer => by simp [Ty.instantiate, Ty.toTag]
  | .struct resource => by simp [Ty.instantiate, Ty.toTag]
  | .enum resource => by simp [Ty.instantiate, Ty.toTag]
  | .typeParam index => by
      have hget := hargs.getElem?_map_toTag index
      cases hlhs : lhs[index]? <;> cases hrhs : rhs[index]? <;>
        simp_all [Ty.instantiate, Ty.toTag]
  | .structInst resource types => by
      have htypes := TyList.toTag_instantiate_eq hargs types
      simpa [Ty.instantiate, Ty.toTag, List.length_map, List.flatMap_def,
        List.map_map, Function.comp_def] using
        congrArg (fun tags =>
          TypeTagToken.struct resource types.length :: tags.flatten) htypes
  | .enumInst resource types => by
      have htypes := TyList.toTag_instantiate_eq hargs types
      simpa [Ty.instantiate, Ty.toTag, List.length_map, List.flatMap_def,
        List.map_map, Function.comp_def] using
        congrArg (fun tags =>
          TypeTagToken.enum resource types.length :: tags.flatten) htypes
  | .vector elem | .ref elem | .mutRef elem => by
      simp only [Ty.instantiate, Ty.toTag]
      rw [Ty.toTag_instantiate_eq hargs elem]

/-- Pointwise form of `Ty.toTag_instantiate_eq`. -/
theorem TyList.toTag_instantiate_eq {lhs rhs : List Ty}
    (hargs : TypeArgsTagEq lhs rhs) : ∀ types : List Ty,
    (types.map fun ty => (ty.instantiate lhs).toTag) =
      (types.map fun ty => (ty.instantiate rhs).toTag)
  | [] => rfl
  | ty :: types => by
      simp only [List.map_cons, List.cons.injEq]
      exact ⟨Ty.toTag_instantiate_eq hargs ty,
        TyList.toTag_instantiate_eq hargs types⟩

end

/-- Instantiating a type list preserves tag-equivalence of substitutions. -/
theorem TypeArgsTagEq.instantiateTypes {lhs rhs : List Ty}
    (hargs : TypeArgsTagEq lhs rhs) (types : List Ty) :
    TypeArgsTagEq (instantiateTypes lhs types) (instantiateTypes rhs types) := by
  unfold TypeArgsTagEq MoveModel.IR.instantiateTypes
  simp only [List.map_map, Function.comp_def]
  exact TyList.toTag_instantiate_eq hargs types

/-- A generic resource expression denotes the same key under tag-equivalent
substitutions. -/
theorem resourceKey_instantiateTypes_eq {lhs rhs : List Ty}
    (hargs : TypeArgsTagEq lhs rhs) (resource : ResourceId) (types : List Ty) :
    resourceKey resource (instantiateTypes lhs types) =
      resourceKey resource (instantiateTypes rhs types) := by
  unfold resourceKey MoveModel.IR.instantiateTypes
  simp only [List.map_map, Function.comp_def]
  exact congrArg (fun tags => (⟨resource, tags⟩ : ResourceKey))
    (TyList.toTag_instantiate_eq hargs types)

/-- Every observed effect denotes the same runtime key under tag-equivalent
substitutions. -/
theorem TagEffect.resourceKeyAt_eq {lhs rhs : List Ty}
    (hargs : TypeArgsTagEq lhs rhs) (effect : TagEffect) :
    effect.resourceKeyAt lhs = effect.resourceKeyAt rhs := by
  exact resourceKey_instantiateTypes_eq hargs effect.resource effect.typeArgs

/-- The executable key comparison decides runtime-key equality. -/
theorem MonoKey.beq_eq_true_iff_runtimeEq (lhs rhs : MonoKey) :
    (lhs == rhs) = true ↔ lhs.RuntimeEq rhs := by
  change MonoKey.beq lhs rhs = true ↔ lhs.RuntimeEq rhs
  constructor
  · intro h
    have parts := Bool.and_eq_true_iff.mp h
    exact ⟨eq_of_beq parts.1, eq_of_beq parts.2⟩
  · rintro ⟨hfun, htypes⟩
    exact Bool.and_eq_true_iff.mpr ⟨beq_of_eq hfun, beq_of_eq htypes⟩

/-- Equality of monomorphization keys is reflexive. -/
theorem MonoKey.beq_self (key : MonoKey) : (key == key) = true := by
  exact _root_.beq_self_eq_true key

theorem MonoKey.RuntimeEq.refl (key : MonoKey) : key.RuntimeEq key :=
  ⟨rfl, rfl⟩

theorem MonoKey.RuntimeEq.symm {lhs rhs : MonoKey}
    (h : lhs.RuntimeEq rhs) : rhs.RuntimeEq lhs :=
  ⟨h.1.symm, h.2.symm⟩

theorem MonoKey.RuntimeEq.trans {first second third : MonoKey}
    (h₁ : first.RuntimeEq second) (h₂ : second.RuntimeEq third) :
    first.RuntimeEq third :=
  ⟨h₁.1.trans h₂.1, h₁.2.trans h₂.2⟩

/-- Substituting runtime-equivalent caller arguments produces
runtime-equivalent callee keys. -/
theorem MonoKey.RuntimeEq.instantiatedCall {caller representative call : MonoKey}
    (h : caller.RuntimeEq representative) :
    (⟨call.funId, instantiateTypes caller.typeArgs call.typeArgs⟩ : MonoKey).RuntimeEq
      ⟨call.funId, instantiateTypes representative.typeArgs call.typeArgs⟩ := by
  exact ⟨rfl, TypeArgsTagEq.instantiateTypes h.2 call.typeArgs⟩

/-- Every explicit member of a plan has a generated function id. -/
theorem MonoPlan.generatedFunId?_isSome_of_mem {plan : MonoPlan} {key : MonoKey}
    (hmem : key ∈ plan.entries) :
    ∃ generated, plan.generatedFunId? key = some generated := by
  unfold MonoPlan.generatedFunId?
  exact ⟨_, List.findIdx?_eq_some_of_exists
    ⟨key, hmem, MonoKey.beq_self key⟩⟩

/-- A plan member runtime-equivalent to a key makes that key resolvable. -/
theorem MonoPlan.generatedFunId?_isSome_of_runtimeEq_mem {plan : MonoPlan}
    {entry key : MonoKey} (hmem : entry ∈ plan.entries)
    (heq : entry.RuntimeEq key) :
    ∃ generated, plan.generatedFunId? key = some generated := by
  unfold MonoPlan.generatedFunId?
  exact ⟨_, List.findIdx?_eq_some_of_exists
    ⟨entry, hmem, (MonoKey.beq_eq_true_iff_runtimeEq entry key).mpr heq⟩⟩

/-- A generated id points to an entry runtime-equivalent to its lookup key. -/
theorem MonoPlan.entry_of_generatedFunId?_eq_some {plan : MonoPlan}
    {key : MonoKey} {generated : FunId}
    (h : plan.generatedFunId? key = some generated) :
    ∃ entry, plan.entries[generated]? = some entry ∧ entry.RuntimeEq key := by
  unfold MonoPlan.generatedFunId? at h
  obtain ⟨hbound, hmatch, _⟩ :=
    List.findIdx?_eq_some_iff_getElem.mp h
  refine ⟨plan.entries[generated], List.getElem?_eq_getElem hbound, ?_⟩
  exact (MonoKey.beq_eq_true_iff_runtimeEq _ _).mp hmatch

end MoveModel.IR
