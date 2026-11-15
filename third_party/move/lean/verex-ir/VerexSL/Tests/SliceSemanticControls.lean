-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

set_option Elab.async false

open Lean Meta Elab Tactic VerexIR VerexSL

/-! ## Semantic controls for `borrowVectorSliceAt`, `takeSliceAtValue`, and `Mutables.resolve_sliceAt`

Static borrow-view validation forbids borrowing a growable `Vec` directly as a
mutable slice, and Rust declarations carrying mutable-slice fields state no own
clauses at the surface today. The theorems below exercise `borrowVectorSliceAt`
under `hasDeref = true`, `takeSliceAtValue` clearing an aggregate with an
own-clause over its slice field's current reading, and `Mutables.resolve` at
`.sliceAt` through an open reference, proving both positive `wp` acceptance and
negative `¬ wp` rejection (no pruning). -/

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
        ∀ row, (@NTy.plainCodec Carriers.ground percentTy).decode? raw = some row →
          row.1.val ≤ 100
      else if src == slotHandle then
        ∀ row, (@NTy.plainCodec Carriers.ground slotTy).decode? raw = some row →
          ∀ p ∈ row.1.currentVector.values, p.1.val = 0
      else True }

attribute [local instance] testCarriers

private def mkPercentPlain (n : Int) : percentTy.plain := (snum n, ())

private theorem deepHolds_percentPlain (n : Int) :
    NTy.deepHolds percentTy (mkPercentPlain n) ↔ n ≤ 100 := by
  have hdec : (@NTy.plainCodec Carriers.ground percentTy).decode?
      (NTy.plainEncode percentTy (mkPercentPlain n)) = some (mkPercentPlain n) :=
    Codec.decode_encode _ _
  constructor
  · intro ⟨hown, _⟩
    exact hown (mkPercentPlain n) hdec
  · intro hn
    refine ⟨?_, trivial, trivial⟩
    intro row hrow
    have : some (mkPercentPlain n) = some row := hdec.symm.trans hrow
    cases this
    exact hn

private def mkPercentCarrier (n : Int) (h : n ≤ 100) : percentTy.carrier :=
  ⟨mkPercentPlain n, (deepHolds_percentPlain n).mpr h⟩

private theorem deepHolds_slotPlain (slice : (NTy.sliceRef percentTy).plain) :
    NTy.deepHolds slotTy (slice, ()) ↔
      (∀ p ∈ slice.currentVector.values, p.1.val = 0) ∧
        NTy.deepHolds (.sliceRef percentTy) slice := by
  have hdec : (@NTy.plainCodec Carriers.ground slotTy).decode?
      (NTy.plainEncode slotTy (slice, ())) = some (slice, ()) :=
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

private theorem mem_forget_ofArrays_singleton (p : percentTy.plain)
    (cur fin : percentTy.carrier) :
    p ∈ (NTy.forget (.sliceRef percentTy)
      (SpecSliceRef.ofArrays
        (⟨⟨#[cur], by simp⟩, rfl⟩ : SpecArray percentTy.carrier 1)
        (⟨⟨#[fin], by simp⟩, rfl⟩ : SpecArray percentTy.carrier 1))).currentVector.values ↔
      p = NTy.forget percentTy cur := by
  have hcur : (NTy.forget (.sliceRef percentTy)
      (SpecSliceRef.ofArrays
        (⟨⟨#[cur], by simp⟩, rfl⟩ : SpecArray percentTy.carrier 1)
        (⟨⟨#[fin], by simp⟩, rfl⟩ : SpecArray percentTy.carrier 1))).currentVector =
      (⟨#[cur], by simp⟩ : SpecVector percentTy.carrier).map (NTy.forget percentTy) := rfl
  rw [hcur]
  simp [SpecVector.map_values]

private theorem selectCertified_slotVal (ref : (NTy.sliceRef percentTy).carrier)
    (holds : NTy.deepHolds slotTy (NTy.forget (.sliceRef percentTy) ref, ())) :
    (Var.here : Var (.cons (.sliceRef percentTy) .nil) (.sliceRef percentTy)).selectCertified
      (⟨(NTy.forget (.sliceRef percentTy) ref, ()), holds⟩ : slotTy.carrier) = ref := by
  apply NTy.forget_injective (.sliceRef percentTy)
  simp [Var.forget_selectCertified]

theorem borrowVectorSliceAt_open_ok :
    let vPlain : (NTy.vector percentTy).plain := ⟨#[mkPercentPlain 50], by decide⟩
    let vCert : (NTy.vector percentTy).carrier := ⟨#[mkPercentCarrier 0 (by omega)], by decide⟩
    let env : HEnv (.cons (.ref (.vector percentTy)) .nil) := (some (vPlain, vCert), ())
    wp (borrowVectorSliceAt (State := Unit) (ρ := .none) 1 none .here (.deref .nil) env)
      (fun _ _ => True) (fun _ => False) () := by
  intro vPlain vCert env
  dsimp only [env, vPlain]
  rw [borrowVectorSliceAt]
  simp only [Var.get, Proj.get?_deref, Proj.get?_nil, Proj.hasDeref_deref, ↓reduceIte,
    wp_bind, wp_prophecyOf, wp_assume, wp_pure]
  apply wp_certifyingAt_intro _ _ (.vector percentTy)
  · intro elem hmem
    have heq : elem = mkPercentPlain 50 := by
      simpa [NTy.forget] using hmem
    subst heq
    exact (deepHolds_percentPlain 50).mpr (by omega)
  · intro _ _ _ _
    trivial

theorem borrowVectorSliceAt_open_bad :
    let vBad : (NTy.vector percentTy).plain := ⟨#[mkPercentPlain 101], by decide⟩
    let vCert : (NTy.vector percentTy).carrier := ⟨#[mkPercentCarrier 0 (by omega)], by decide⟩
    let env : HEnv (.cons (.ref (.vector percentTy)) .nil) := (some (vBad, vCert), ())
    ¬ wp (borrowVectorSliceAt (State := Unit) (ρ := .none) 1 none .here (.deref .nil) env)
      (fun _ _ => True) (fun _ => False) () := by
  intro vBad vCert env hwp
  dsimp only [env, vBad] at hwp
  rw [borrowVectorSliceAt] at hwp
  simp only [Var.get, Proj.get?_deref, Proj.get?_nil, Proj.hasDeref_deref, ↓reduceIte,
    wp_bind] at hwp
  have hcert := (wp_certifyingAt (τ := .vector percentTy) _ _ _ _ _ _).mp hwp
  have h101 : NTy.deepHolds percentTy (mkPercentPlain 101) :=
    hcert.1 (mkPercentPlain 101) (by simp [NTy.forget])
  have := (deepHolds_percentPlain 101).mp h101
  omega

theorem takeSliceAtValue_cleared_ok :
    let p0 := mkPercentCarrier 0 (by omega)
    let arr0 : SpecArray percentTy.carrier 1 := ⟨⟨#[p0], by simp⟩, rfl⟩
    let ref0 : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arr0 arr0
    let slotVal : slotTy.carrier := ⟨(NTy.forget (.sliceRef percentTy) ref0, ()),
      (deepHolds_slotPlain _).mpr ⟨fun p hp => by
        have := (mem_forget_ofArrays_singleton p p0 p0).mp hp
        subst this; rfl,
        NTy.deepHolds_forget (.sliceRef percentTy) ref0⟩⟩
    let env : HEnv (.cons slotTy .nil) := (some slotVal, ())
    wp (takeSliceAtValue (State := Unit) (ρ := .none) 2 none .here (.field .here .nil) env)
      (fun _ _ => True) (fun _ => False) () := by
  intro p0 arr0 ref0 slotVal env
  have hsel : (Var.here : Var (.cons (.sliceRef percentTy) .nil) (.sliceRef percentTy)).selectCertified
      slotVal = ref0 := selectCertified_slotVal ref0 slotVal.2
  dsimp only [env]
  rw [takeSliceAtValue]
  simp only [Var.get, Proj.getCertified?_field, Proj.getCertified?_nil, hsel,
    Proj.setOwner?_field, Proj.set?_field, Proj.set?_nil, Option.map_some, wp_bind, wp_pure]
  apply wp_certifyingAt_intro _ _ slotTy
  · dsimp only [slotVal, NTy.forget_struct, Var.updatePlain_here]
    exact (deepHolds_slotPlain _).mpr ⟨fun p hp => by
      dsimp only [ref0, arr0, SpecSliceRef.ofArrays] at hp
      have := (mem_forget_ofArrays_singleton p p0 p0).mp hp
      subst this; rfl,
      NTy.deepHolds_forget (.sliceRef percentTy) (SpecSliceRef.ofArrays ref0.final ref0.final)⟩
  · intro _
    trivial

theorem takeSliceAtValue_cleared_bad :
    let p0 := mkPercentCarrier 0 (by omega)
    let p50 := mkPercentCarrier 50 (by omega)
    let arr0 : SpecArray percentTy.carrier 1 := ⟨⟨#[p0], by simp⟩, rfl⟩
    let arr50 : SpecArray percentTy.carrier 1 := ⟨⟨#[p50], by simp⟩, rfl⟩
    let refOpen : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arr0 arr50
    let slotVal : slotTy.carrier := ⟨(NTy.forget (.sliceRef percentTy) refOpen, ()),
      (deepHolds_slotPlain _).mpr ⟨fun p hp => by
        have := (mem_forget_ofArrays_singleton p p0 p50).mp hp
        subst this; rfl,
        NTy.deepHolds_forget (.sliceRef percentTy) refOpen⟩⟩
    let env : HEnv (.cons slotTy .nil) := (some slotVal, ())
    ¬ wp (takeSliceAtValue (State := Unit) (ρ := .none) 2 none .here (.field .here .nil) env)
      (fun _ _ => True) (fun _ => False) () := by
  intro p0 p50 arr0 arr50 refOpen slotVal env hwp
  have hsel : (Var.here : Var (.cons (.sliceRef percentTy) .nil) (.sliceRef percentTy)).selectCertified
      slotVal = refOpen := selectCertified_slotVal refOpen slotVal.2
  dsimp only [env] at hwp
  rw [takeSliceAtValue] at hwp
  simp only [Var.get, Proj.getCertified?_field, Proj.getCertified?_nil, hsel,
    Proj.setOwner?_field, Proj.set?_field, Proj.set?_nil, Option.map_some, wp_bind] at hwp
  have holds := ((wp_certifyingAt (τ := slotTy) _ _ _ _ _ _).mp hwp).1
  dsimp only [slotVal, NTy.forget_struct, Var.updatePlain_here, refOpen, arr50,
    SpecSliceRef.ofArrays] at holds
  have hzero := ((deepHolds_slotPlain _).mp holds).1 (NTy.forget percentTy p50)
    ((mem_forget_ofArrays_singleton _ p50 p50).mpr rfl)
  simp [p50, mkPercentCarrier, mkPercentPlain, NTy.forget, snum] at hzero

theorem mutables_resolve_sliceAt_open_ok :
    let arr50 : SpecArray percentTy.plain 1 := ⟨⟨#[mkPercentPlain 50], by decide⟩, rfl⟩
    let slicePlain : (NTy.sliceRef percentTy).plain := SpecSliceRef.ofArrays arr50 arr50
    let p0 := mkPercentCarrier 0 (by omega)
    let arrCert : SpecArray percentTy.carrier 1 := ⟨⟨#[p0], by simp⟩, rfl⟩
    let sliceCert : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arrCert arrCert
    let env : HEnv (.cons (.ref (.sliceRef percentTy)) .nil) := (some (slicePlain, sliceCert), ())
    wp ((Mutables.sliceAt .here (.deref .nil) .nil).resolve (State := Unit) 3 env (Spec.pure ()))
      (fun _ _ => True) (fun _ => False) () := by
  intro arr50 slicePlain p0 arrCert sliceCert env
  dsimp only [env, slicePlain, arr50]
  rw [Mutables.resolve_sliceAt]
  simp only [Var.get, Option.bind_some, Proj.get?_deref, Proj.get?_nil,
    wp_bind, wp_assume, Mutables.resolve_nil, wp_pure]
  apply wp_certifyingAt_intro _ _ (.slice percentTy)
  · intro elem hmem
    have heq : elem = mkPercentPlain 50 := by
      simpa [NTy.forget, SpecSliceRef.ofArrays, SpecSliceRef.currentVector] using hmem
    subst heq
    exact (deepHolds_percentPlain 50).mpr (by omega)
  · intro _ _
    trivial

theorem mutables_resolve_sliceAt_open_bad :
    let arr101 : SpecArray percentTy.plain 1 := ⟨⟨#[mkPercentPlain 101], by decide⟩, rfl⟩
    let arr50 : SpecArray percentTy.plain 1 := ⟨⟨#[mkPercentPlain 50], by decide⟩, rfl⟩
    let slicePlain : (NTy.sliceRef percentTy).plain := SpecSliceRef.ofArrays arr101 arr50
    let p0 := mkPercentCarrier 0 (by omega)
    let arrCert : SpecArray percentTy.carrier 1 := ⟨⟨#[p0], by simp⟩, rfl⟩
    let sliceCert : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arrCert arrCert
    let env : HEnv (.cons (.ref (.sliceRef percentTy)) .nil) := (some (slicePlain, sliceCert), ())
    ¬ wp ((Mutables.sliceAt .here (.deref .nil) .nil).resolve (State := Unit) 3 env (Spec.pure ()))
      (fun _ _ => True) (fun _ => False) () := by
  intro arr101 arr50 slicePlain p0 arrCert sliceCert env hwp
  dsimp only [env, slicePlain, arr101] at hwp
  rw [Mutables.resolve_sliceAt] at hwp
  simp only [Var.get, Option.bind_some, Proj.get?_deref, Proj.get?_nil, wp_bind] at hwp
  have hcert := (wp_certifyingAt (τ := .slice percentTy) _ _ _ _ _ _).mp hwp
  have h101 : NTy.deepHolds percentTy (mkPercentPlain 101) :=
    hcert.1 (mkPercentPlain 101)
      (by simp [NTy.forget, SpecSliceRef.ofArrays, SpecSliceRef.currentVector])
  have := (deepHolds_percentPlain 101).mp h101
  omega

run_cmd do
  for name in [
      ``borrowVectorSliceAt_open_ok, ``borrowVectorSliceAt_open_bad,
      ``takeSliceAtValue_cleared_ok, ``takeSliceAtValue_cleared_bad,
      ``mutables_resolve_sliceAt_open_ok, ``mutables_resolve_sliceAt_open_bad] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end SliceSemanticControls
