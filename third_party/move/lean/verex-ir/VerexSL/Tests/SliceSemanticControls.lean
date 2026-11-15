-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

set_option Elab.async false

open Lean Meta Elab Tactic VerexIR VerexSL

/-! ## Semantic controls for `borrowVectorSliceAt`, `SliceLender.resolve`, `takeSliceAtValue`, and `Mutables.resolve_sliceAt`

Static borrow-view validation forbids borrowing a growable `Vec` directly as a
mutable slice, and Rust declarations carrying mutable-slice fields state no own
clauses at the surface today. The theorems below exercise `borrowVectorSliceAt`
under `hasDeref = true`, `SliceLender.resolve` at `.vector`, `takeSliceAtValue`
clearing an aggregate with an own-clause over its slice field's current reading,
and `Mutables.resolve` at `.sliceAt` through an open reference — proving in each
pair that invalid current states fail `wp` (`¬ wp`, negative first) and valid
current states satisfy `wp` (positive second). -/

namespace SliceSemanticControls
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

private abbrev percentHandle : StructHandle := ⟨⟨0⟩, 0⟩
private abbrev slotHandle : StructHandle := ⟨⟨0⟩, 1⟩

private abbrev percentTy : NTy := .struct percentHandle .nil (.cons .num .nil)
private abbrev slotTy : NTy := .struct slotHandle .nil (.cons (.sliceRef percentTy) .nil)

private abbrev snum (n : Int) : SpecInt .unbounded true := ⟨n, rfl⟩

@[reducible] private def testCarriers : Carriers :=
  { Carriers.ground with
    own := fun src _ raw =>
      if src == percentHandle then
        ∀ row, (@NTy.codec Carriers.ground percentTy).decode? raw = some row →
          row.1.val ≤ 100
      else if src == slotHandle then
        ∀ row, (@NTy.codec Carriers.ground slotTy).decode? raw = some row →
          ∀ p ∈ row.1.currentVector.values, p.1.val = 0
      else True }

attribute [local instance] testCarriers

private def mkPercent (n : Int) : percentTy.carrier := (snum n, ())

private theorem holds_percent (n : Int) :
    NTy.Holds percentTy (mkPercent n) ↔ n ≤ 100 := by
  rw [NTy.holds_iff]
  have hdec : (@NTy.codec Carriers.ground percentTy).decode?
      (NTy.encode percentTy (mkPercent n)) = some (mkPercent n) :=
    Codec.decode_encode _ _
  constructor
  · intro ⟨hown, _⟩
    exact hown (mkPercent n) hdec
  · intro hn
    refine ⟨?_, trivial, trivial⟩
    intro row hrow
    have : some (mkPercent n) = some row := hdec.symm.trans hrow
    cases this
    exact hn

private theorem holds_slot (slice : (NTy.sliceRef percentTy).carrier) :
    NTy.Holds slotTy (slice, ()) ↔
      (∀ p ∈ slice.currentVector.values, p.1.val = 0) ∧
        NTy.Holds (.sliceRef percentTy) slice := by
  rw [NTy.holds_iff, NTy.holds_iff]
  have hdec : (@NTy.codec Carriers.ground slotTy).decode?
      (NTy.encode slotTy (slice, ())) = some (slice, ()) :=
    Codec.decode_encode _ _
  constructor
  · intro ⟨hown, hslice, _⟩
    exact ⟨hown (slice, ()) hdec, hslice⟩
  · intro ⟨hzero, hslice⟩
    refine ⟨?_, hslice, trivial⟩
    intro row hrow
    have : some (slice, ()) = some row := hdec.symm.trans hrow
    cases this
    exact hzero

/-! ### 1. `borrowVectorSliceAt` (`hasDeref = true`): negative first, positive second -/

theorem borrowVectorSliceAt_open_bad :
    let vBad : (NTy.vector percentTy).carrier := ⟨#[mkPercent 101], by decide⟩
    let vProph : (NTy.vector percentTy).carrier := ⟨#[mkPercent 0], by decide⟩
    let env : HEnv (.cons (.ref (.vector percentTy)) .nil) := (some (vBad, vProph), ())
    ¬ wp (borrowVectorSliceAt (State := Unit) (ρ := .none) 1 none .here (.deref .nil) env)
      (fun _ _ => True) (fun _ => False) () := by
  intro vBad vProph env hwp
  dsimp only [env, vBad] at hwp
  rw [borrowVectorSliceAt] at hwp
  simp only [Var.get, Proj.get?_deref, Proj.get?_nil, Proj.hasDeref_deref, ↓reduceIte,
    wp_bind, wp_checkingAt] at hwp
  have h101 : NTy.Holds percentTy (mkPercent 101) :=
    (NTy.holds_vector percentTy _).mp hwp.1 (mkPercent 101) (by simp)
  have := (holds_percent 101).mp h101
  omega

theorem borrowVectorSliceAt_open_ok :
    let vOk : (NTy.vector percentTy).carrier := ⟨#[mkPercent 50], by decide⟩
    let vProph : (NTy.vector percentTy).carrier := ⟨#[mkPercent 0], by decide⟩
    let env : HEnv (.cons (.ref (.vector percentTy)) .nil) := (some (vOk, vProph), ())
    wp (borrowVectorSliceAt (State := Unit) (ρ := .none) 1 none .here (.deref .nil) env)
      (fun _ _ => True) (fun _ => False) () := by
  intro vOk vProph env
  dsimp only [env, vOk]
  rw [borrowVectorSliceAt]
  simp only [Var.get, Proj.get?_deref, Proj.get?_nil, Proj.hasDeref_deref, ↓reduceIte,
    wp_bind, wp_prophecyOf, wp_assume, wp_pure]
  apply wp_checkingAt_intro 1 .handOver (.vector percentTy)
  · rw [NTy.holds_vector]
    intro elem hmem
    have heq : elem = mkPercent 50 := by simpa using hmem
    subst heq
    exact (holds_percent 50).mpr (by omega)
  · rintro _ _ _ ⟨_, _, _, _, _⟩
    trivial

/-! ### 2. `SliceLender.resolve` at `.vector` (`hasDeref = true`): negative first, positive second -/

theorem sliceLender_resolve_vector_bad :
    let arr101 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 101], by decide⟩, rfl⟩
    let arr50 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 50], by decide⟩, rfl⟩
    let reference : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arr101 arr50
    let vCur : (NTy.vector percentTy).carrier := ⟨#[mkPercent 0], by decide⟩
    let vProph : (NTy.vector percentTy).carrier := ⟨#[mkPercent 50], by decide⟩
    let env : HEnv (.cons (.ref (.vector percentTy)) .nil) := (some (vCur, vProph), ())
    ¬ wp ((SliceLender.vector .here (.deref .nil)).resolve (State := Unit) (ρ := .none) 4 env reference)
      (fun _ _ => True) (fun _ => False) () := by
  intro arr101 arr50 reference vCur vProph env hwp
  dsimp only [env, reference, arr101] at hwp
  rw [SliceLender.resolve] at hwp
  simp only [Var.get, Proj.set?_deref, Proj.set?_nil, Option.map_some, Proj.hasDeref_deref,
    ↓reduceIte, wp_bind, wp_checkingAt] at hwp
  have h101 : NTy.Holds percentTy (mkPercent 101) :=
    (NTy.holds_slice percentTy _).mp hwp.1 (mkPercent 101)
      (by simp [SpecSliceRef.ofArrays, SpecSliceRef.currentVector])
  have := (holds_percent 101).mp h101
  omega

theorem sliceLender_resolve_vector_ok :
    let arr50 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 50], by decide⟩, rfl⟩
    let reference : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arr50 arr50
    let vCur : (NTy.vector percentTy).carrier := ⟨#[mkPercent 0], by decide⟩
    let vProph : (NTy.vector percentTy).carrier := ⟨#[mkPercent 50], by decide⟩
    let env : HEnv (.cons (.ref (.vector percentTy)) .nil) := (some (vCur, vProph), ())
    wp ((SliceLender.vector .here (.deref .nil)).resolve (State := Unit) (ρ := .none) 4 env reference)
      (fun _ _ => True) (fun _ => False) () := by
  intro arr50 reference vCur vProph env
  dsimp only [env, reference, arr50, vProph]
  rw [SliceLender.resolve]
  simp only [Var.get, Proj.set?_deref, Proj.set?_nil, Option.map_some, Proj.hasDeref_deref,
    Proj.isRootDeref_deref, ↓reduceIte, Spec.pure_bind, wp_bind, wp_assume, wp_pure]
  have h50 : ∀ elem ∈ (#[mkPercent 50] : Array percentTy.carrier), NTy.Holds percentTy elem := by
    intro elem hmem
    have heq : elem = mkPercent 50 := by simpa using hmem
    subst heq
    exact (holds_percent 50).mpr (by omega)
  apply wp_checkingAt_intro 4 .mutationEnd (.slice percentTy)
  · rw [NTy.holds_slice]
    intro elem hmem
    exact h50 elem (by simpa [SpecSliceRef.ofArrays, SpecSliceRef.currentVector] using hmem)
  · intros
    trivial

/-! ### 3. `takeSliceAtValue` (clearing aggregate slice-current invariant): negative first, positive second -/

theorem takeSliceAtValue_cleared_bad :
    let arr0 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 0], by decide⟩, rfl⟩
    let arr50 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 50], by decide⟩, rfl⟩
    let refOpen : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arr0 arr50
    let slotVal : slotTy.carrier := (refOpen, ())
    let env : HEnv (.cons slotTy .nil) := (some slotVal, ())
    ¬ wp (takeSliceAtValue (State := Unit) (ρ := .none) 2 none .here (.field .here .nil) env)
      (fun _ _ => True) (fun _ => False) () := by
  intro arr0 arr50 refOpen slotVal env hwp
  dsimp only [env, slotVal] at hwp
  rw [takeSliceAtValue] at hwp
  simp only [Var.get, Proj.get?_field, Proj.get?_nil, Var.select,
    Proj.setOwner?_field, Proj.set?_field, Proj.set?_nil, Option.map_some,
    wp_bind, wp_checkingAt] at hwp
  dsimp only [Var.update, refOpen, arr50, SpecSliceRef.ofArrays] at hwp
  have hzero := ((holds_slot _).mp hwp.1).1 (mkPercent 50)
    (by simp [SpecSliceRef.currentVector])
  simp [mkPercent, snum] at hzero

theorem takeSliceAtValue_cleared_ok :
    let arr0 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 0], by decide⟩, rfl⟩
    let ref0 : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arr0 arr0
    let slotVal : slotTy.carrier := (ref0, ())
    let env : HEnv (.cons slotTy .nil) := (some slotVal, ())
    wp (takeSliceAtValue (State := Unit) (ρ := .none) 2 none .here (.field .here .nil) env)
      (fun _ _ => True) (fun _ => False) () := by
  intro arr0 ref0 slotVal env
  dsimp only [env, slotVal]
  rw [takeSliceAtValue]
  simp only [Var.get, Proj.get?_field, Proj.get?_nil, Var.select,
    Proj.setOwner?_field, Proj.set?_field, Proj.set?_nil, Option.map_some, wp_bind, wp_pure]
  apply wp_checkingAt_intro 2 .mutationEnd slotTy
  · dsimp only [Var.update, ref0, arr0, SpecSliceRef.ofArrays]
    refine (holds_slot _).mpr ⟨?_, ?_⟩
    · intro p hp
      have heq : p = mkPercent 0 := by
        simpa [SpecSliceRef.ofArrays, SpecSliceRef.currentVector] using hp
      subst heq
      rfl
    · rw [NTy.holds_sliceRef]
      refine ⟨?_, ?_⟩ <;>
      · intro elem hmem
        have heq : elem = mkPercent 0 := by
          simpa [SpecSliceRef.ofArrays, SpecSliceRef.currentVector, SpecSliceRef.finalVector] using hmem
        subst heq
        exact (holds_percent 0).mpr (by omega)
  · intro _
    trivial

/-! ### 4. `Mutables.resolve_sliceAt`: negative first, positive second -/

theorem mutables_resolve_sliceAt_open_bad :
    let arr101 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 101], by decide⟩, rfl⟩
    let arr50 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 50], by decide⟩, rfl⟩
    let sliceCur : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arr101 arr50
    let arr0 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 0], by decide⟩, rfl⟩
    let sliceProph : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arr0 arr0
    let env : HEnv (.cons (.ref (.sliceRef percentTy)) .nil) := (some (sliceCur, sliceProph), ())
    ¬ wp ((Mutables.sliceAt .here (.deref .nil) .nil).resolve (State := Unit) 3 env (Spec.pure ()))
      (fun _ _ => True) (fun _ => False) () := by
  intro arr101 arr50 sliceCur arr0 sliceProph env hwp
  dsimp only [env, sliceCur, arr101] at hwp
  rw [Mutables.resolve_sliceAt] at hwp
  simp only [Var.get, Option.bind_some, Proj.get?_deref, Proj.get?_nil,
    wp_bind, wp_checkingAt] at hwp
  have h101 : NTy.Holds percentTy (mkPercent 101) :=
    (NTy.holds_slice percentTy _).mp hwp.1 (mkPercent 101)
      (by simp [SpecSliceRef.ofArrays, SpecSliceRef.currentVector])
  have := (holds_percent 101).mp h101
  omega

theorem mutables_resolve_sliceAt_open_ok :
    let arr50 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 50], by decide⟩, rfl⟩
    let sliceCur : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arr50 arr50
    let arr0 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 0], by decide⟩, rfl⟩
    let sliceProph : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arr0 arr0
    let env : HEnv (.cons (.ref (.sliceRef percentTy)) .nil) := (some (sliceCur, sliceProph), ())
    wp ((Mutables.sliceAt .here (.deref .nil) .nil).resolve (State := Unit) 3 env (Spec.pure ()))
      (fun _ _ => True) (fun _ => False) () := by
  intro arr50 sliceCur arr0 sliceProph env
  dsimp only [env, sliceCur, arr50]
  rw [Mutables.resolve_sliceAt]
  simp only [Var.get, Option.bind_some, Proj.get?_deref, Proj.get?_nil,
    wp_bind, wp_assume, Mutables.resolve_nil, wp_pure]
  apply wp_checkingAt_intro 3 .exit (.slice percentTy)
  · rw [NTy.holds_slice]
    intro elem hmem
    have heq : elem = mkPercent 50 := by
      simpa [SpecSliceRef.ofArrays, SpecSliceRef.currentVector] using hmem
    subst heq
    exact (holds_percent 50).mpr (by omega)
  · intro _ _
    trivial

run_cmd do
  for name in [
      ``borrowVectorSliceAt_open_bad, ``borrowVectorSliceAt_open_ok,
      ``sliceLender_resolve_vector_bad, ``sliceLender_resolve_vector_ok,
      ``takeSliceAtValue_cleared_bad, ``takeSliceAtValue_cleared_ok,
      ``mutables_resolve_sliceAt_open_bad, ``mutables_resolve_sliceAt_open_ok] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end SliceSemanticControls
