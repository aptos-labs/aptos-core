-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Mono.Correctness.Types

/-!
# Runtime-tag coverage relations

The finite pass represents infinitely many concrete type substitutions by
their equality pattern on observed global resource keys. This file develops
that quotient independently of generated-function lookup and execution.
-/

namespace MoveModel.IR

theorem SameTagInteractions.refl (effects : List TagEffect)
    (args : List Ty) : SameTagInteractions effects args args := by
  intro lhs _ rhs _
  exact Iff.rfl

theorem SameTagInteractions.symm {effects : List TagEffect} {lhs rhs : List Ty}
    (h : SameTagInteractions effects lhs rhs) :
    SameTagInteractions effects rhs lhs := by
  intro first hfirst second hsecond
  exact (h first hfirst second hsecond).symm

theorem SameTagInteractions.trans {effects : List TagEffect}
    {first second third : List Ty}
    (h₁ : SameTagInteractions effects first second)
    (h₂ : SameTagInteractions effects second third) :
    SameTagInteractions effects first third := by
  intro lhs hlhs rhs hrhs
  exact (h₁ lhs hlhs rhs hrhs).trans (h₂ lhs hlhs rhs hrhs)

/-- Equal runtime tags are a stronger relation than equality-pattern
coverage: every observed resource key itself is equal. -/
theorem TypeArgsTagEq.sameTagInteractions {lhs rhs : List Ty}
    (h : TypeArgsTagEq lhs rhs) (effects : List TagEffect) :
    SameTagInteractions effects lhs rhs := by
  intro first _ second _
  rw [first.resourceKeyAt_eq h, second.resourceKeyAt_eq h]

/-- One observed source key corresponds to the same observation evaluated at
the representative substitution. -/
def ObservedKeyRel (effects : List TagEffect) (lhsArgs rhsArgs : List Ty)
    (lhsKey rhsKey : ResourceKey) : Prop :=
  ∃ effect ∈ effects,
    lhsKey = effect.resourceKeyAt lhsArgs ∧
    rhsKey = effect.resourceKeyAt rhsArgs

/-- Equality patterns make observed-key correspondence functional from the
concrete instance to its representative. -/
theorem SameTagInteractions.observedKeyRel_right_unique
    {effects : List TagEffect} {lhsArgs rhsArgs : List Ty}
    (h : SameTagInteractions effects lhsArgs rhsArgs)
    {lhsKey rhsKey₁ rhsKey₂ : ResourceKey}
    (h₁ : ObservedKeyRel effects lhsArgs rhsArgs lhsKey rhsKey₁)
    (h₂ : ObservedKeyRel effects lhsArgs rhsArgs lhsKey rhsKey₂) :
    rhsKey₁ = rhsKey₂ := by
  obtain ⟨first, hfirst, rfl, rfl⟩ := h₁
  obtain ⟨second, hsecond, hleft, rfl⟩ := h₂
  apply (h first hfirst second hsecond).mp
  exact hleft

/-- Equality patterns also make observed-key correspondence injective. -/
theorem SameTagInteractions.observedKeyRel_left_unique
    {effects : List TagEffect} {lhsArgs rhsArgs : List Ty}
    (h : SameTagInteractions effects lhsArgs rhsArgs)
    {lhsKey₁ lhsKey₂ rhsKey : ResourceKey}
    (h₁ : ObservedKeyRel effects lhsArgs rhsArgs lhsKey₁ rhsKey)
    (h₂ : ObservedKeyRel effects lhsArgs rhsArgs lhsKey₂ rhsKey) :
    lhsKey₁ = lhsKey₂ := by
  obtain ⟨first, hfirst, rfl, rfl⟩ := h₁
  obtain ⟨second, hsecond, rfl, hright⟩ := h₂
  apply (h first hfirst second hsecond).mpr
  exact hright

/-- Memories agree on all resource observations paired by two
substitutions. This is the state relation used by the future representative
execution simulation; unobserved global keys intentionally remain free. -/
def ObservedMemoryEq (effects : List TagEffect) (lhsArgs rhsArgs : List Ty)
    (lhs rhs : Memory) : Prop :=
  ∀ effect ∈ effects, ∀ address,
    lhs (effect.resourceKeyAt lhsArgs) address =
      rhs (effect.resourceKeyAt rhsArgs) address

theorem ObservedMemoryEq.refl (effects : List TagEffect) (args : List Ty)
    (memory : Memory) : ObservedMemoryEq effects args args memory memory := by
  intro _ _ _
  rfl

theorem ObservedMemoryEq.symm {effects : List TagEffect} {lhsArgs rhsArgs : List Ty}
    {lhs rhs : Memory} (h : ObservedMemoryEq effects lhsArgs rhsArgs lhs rhs) :
    ObservedMemoryEq effects rhsArgs lhsArgs rhs lhs := by
  intro effect heffect address
  exact (h effect heffect address).symm

end MoveModel.IR
