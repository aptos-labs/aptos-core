-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Reconstruct the updated Counter from its bounded field using the codec
-- round trip. The state label uses that value at callers as well as callees.
theorem decoded_counter [LeanerIR.Proofs.Denote.Carriers] (source : LeanerIR.StructHandle) (n : Int)
    (bounds : 0 ≤ n ∧ n ≤ 18446744073709551615) :
    let τ := LeanerIR.Proofs.Denote.NTy.struct source .nil (.cons (.int 64 false) .nil)
    (τ.codec.decode? (.nominal source none #[.integer n])).isSome = true ∧
      ∃ stored, τ.codec.decode? (.nominal source none #[.integer n]) = some stored ∧
        LeanerIR.RuntimeValue.nominal source none #[.integer stored.fst.val] =
          .nominal source none #[.integer n] := by
  let value : LeanerIR.Proofs.Denote.HList (.cons (.int 64 false) .nil) :=
    (⟨n, by
      simpa [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?,
        LeanerIR.Ty.integerBounds?] using bounds⟩, ())
  have decoded : (LeanerIR.Proofs.Denote.NTy.struct source .nil (.cons (.int 64 false) .nil)).codec.decode?
      (.nominal source none #[.integer n]) = some value :=
    LeanerIR.Proofs.Denote.NTy.decode?_struct_literal source (.cons (.int 64 false) .nil) value
  exact ⟨by rw [decoded]; rfl, value, decoded, rfl⟩

verify test_non_old_cross_resource by
  all_goals expose_names
  all_goals cases entry : (initialState
    { type := .struct ⟨⟨0⟩, 0⟩ .nil (.cons (.int 64 false) .nil), arguments := .nil }
    (.address addr)) <;>
    simp only [entry, Option.map_none, Option.map_some, Option.getD_none,
      Option.getD_some, Option.isSome_none, Option.isSome_some] at *
  case none => exact False.elim (right (Or.inl trivial))
  case some value =>
    refine ⟨True.intro, @decoded_counter (LeanerIR.Proofs.Denote.Carriers.runtime unit)
      ⟨⟨0⟩, 0⟩ (value.fst.val + 1) ?_⟩
    simp only [LeanerIR.RuntimeValue.asInt] at permitted right
    constructor <;> omega

verify test_uses_old_cross_resource by
  all_goals expose_names
  all_goals cases entry : (initialState
    { type := .struct ⟨⟨0⟩, 0⟩ .nil (.cons (.int 64 false) .nil), arguments := .nil }
    (.address addr)) <;>
    simp only [entry, Option.map_none, Option.map_some, Option.getD_none,
      Option.getD_some, Option.isSome_none, Option.isSome_some] at *
  case leaf_1.none => exact False.elim (right (Or.inl trivial))
  case leaf_2.none => exact False.elim (right (Or.inl trivial))
  case leaf_1.some value =>
    have lower := value.fst.fits.unsigned_bounds
    simp only [LeanerIR.RuntimeValue.asInt] at right
    have bounds : 0 ≤ value.fst.val + 1 ∧ value.fst.val + 1 ≤ 18446744073709551615 := by omega
    obtain ⟨_, stored, decoded, encoded⟩ :=
      @decoded_counter (LeanerIR.Proofs.Denote.Carriers.runtime unit) ⟨⟨0⟩, 0⟩
        (value.fst.val + 1) bounds
    have stored_eq : stored.fst.val = value.fst.val + 1 := by
      have equal := congrArg (fun v : LeanerIR.RuntimeValue => (v.field 0).asInt) encoded
      simpa [LeanerIR.RuntimeValue.field, LeanerIR.RuntimeValue.asInt] using equal
    have frame := left_3
      { type := .struct ⟨⟨0⟩, 0⟩ .nil (.cons (.int 64 false) .nil), arguments := .nil }
      (by decide)
    have increment := left_4 right
    simp only [LeanerIR.RuntimeValue.asInt] at increment ⊢
    rw [decoded]
    simp only [Option.map_some, Option.getD_some, LeanerIR.RuntimeValue.asInt, stored_eq]
    rw [frame]
    exact increment.symm
  case leaf_2.some value =>
    have lower := value.fst.fits.unsigned_bounds
    simp only [LeanerIR.RuntimeValue.asInt] at right
    refine ⟨True.intro, @decoded_counter (LeanerIR.Proofs.Denote.Carriers.runtime unit)
      ⟨⟨0⟩, 0⟩ (value.fst.val + 1) ?_⟩
    constructor <;> omega

-- The direct calls already supply the facts needed at both invocation labels.
verify test_behavior_cross_resource by
  all_goals simp_all
