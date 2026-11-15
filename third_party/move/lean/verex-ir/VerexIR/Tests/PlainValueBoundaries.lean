-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Term
import VerexIR.Proofs.RelationalTransfer

/-!
# Phase 2b Kernel Controls for Plain-Value Boundaries

Kernel-checked negative and positive controls for the four plain-value
boundary mechanisms (`FABLE-CONSULT-REPLY-13.md` §§2–4):
1. `ghostAt` producer boundary (`checkingAt site .construction` + step-free `u64`),
2. `arbitraryIn` domain-restricted havoc (`Spec.assume (NTy.Holds τ value)`),
3. `SliceLender.resolve` `.vector` settlement (check before prophecy equation),
4. Call-result `validResult` (`ResultShape.Holds`) on bodiless native and opaque callees.

In every section, negative controls precede positive controls.
-/

namespace VerexIR.Tests.PlainValueBoundaries

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

private abbrev percentHandle : StructHandle := ⟨⟨0⟩, 0⟩
private abbrev percentTy : NTy := .struct percentHandle .nil (.cons .num .nil)
private abbrev snum (n : Int) : SpecInt .unbounded true := ⟨n, rfl⟩

@[reducible] private def testCarriers : Carriers :=
  { Carriers.ground with
    own := fun src _ raw =>
      if src == percentHandle then
        ∀ row, (@NTy.codec Carriers.ground percentTy).decode? raw = some row →
          row.1.val ≤ 100
      else True }

attribute [local instance] testCarriers

private def mkPercent (n : Nat) : percentTy.carrier := (snum (n : Int), ())

private theorem holds_percent_iff (n : Nat) :
    NTy.Holds percentTy (mkPercent n) ↔ n ≤ 100 := by
  rw [NTy.holds_iff]
  have hdec : (@NTy.codec Carriers.ground percentTy).decode?
      (NTy.encode percentTy (mkPercent n)) = some (mkPercent n) :=
    Codec.decode_encode _ _
  constructor
  · intro ⟨hown, _⟩
    have hle : (n : Int) ≤ 100 := hown (mkPercent n) hdec
    omega
  · intro hn
    refine ⟨?_, trivial, trivial⟩
    intro row hrow
    have : some (mkPercent n) = some row := hdec.symm.trans hrow
    cases this
    show (n : Int) ≤ 100
    omega

private theorem val_le_100_of_holds {r : percentTy.carrier}
    (hr : NTy.Holds percentTy r) : r.1.val ≤ 100 := by
  have hdec : (@NTy.codec Carriers.ground percentTy).decode?
      (NTy.encode percentTy r) = some r :=
    Codec.decode_encode _ _
  exact hr.deepHolds.1 r hdec

private theorem holds_percent_42 : NTy.Holds percentTy (mkPercent 42) :=
  (holds_percent_iff 42).mpr (by omega)

private theorem not_holds_percent_101 : ¬NTy.Holds percentTy (mkPercent 101) := by
  rw [holds_percent_iff]
  omega

-- ============================================================================
-- Section 1: `ghostAt` Producer Boundary (`FABLE-CONSULT-REPLY-13.md` §2)
-- ============================================================================

section GhostAtBoundary

-- Negative 1 (first): `chosen := fun _ => some (mkPercent 101)` makes `ghostAt`
-- undefined at `checkingAt site .construction` and fails even a `True` postcondition.
theorem ghostAt_invalid_some_refused :
    ¬wp (ghostAt (State := Unit) (ρ := .none) (Γ := .nil) (τ := percentTy)
        1 (fun _ => some (mkPercent 101)) ())
      (fun _ _ => True) (fun _ => False) () := by
  intro hwp
  unfold ghostAt at hwp
  rw [wp_bind, wp_certifyState] at hwp
  have h1 := hwp.2
  rw [wp_bind_get, wp_bind, wp_choose] at h1
  have h2 := h1 (mkPercent 101)
  rw [wp_bind, wp_assume] at h2
  have h3 := h2 rfl
  rw [wp_bind, wp_checkingAt] at h3
  exact not_holds_percent_101 h3.1

-- Negative 2: `chosen := fun _ => none` fails `certifyState ((chosen state).isSome)`.
theorem ghostAt_none_refused :
    ¬wp (ghostAt (State := Unit) (ρ := .none) (Γ := .nil) (τ := percentTy)
        1 (fun _ => none) ())
      (fun _ _ => True) (fun _ => False) () := by
  intro hwp
  unfold ghostAt at hwp
  rw [wp_bind, wp_certifyState] at hwp
  cases hwp.1

-- Negative 3 (soundness contrast): replacing `checkingAt` with `Spec.assume`
-- at `ghostAt` would prune invalid `some (mkPercent 101)` and vacuously prove `False`.
theorem ghostAt_assume_would_vacuously_prove_false :
    let assumeGhost : Spec Unit Failure (Flow .none .nil percentTy.carrier) :=
      Spec.bind (Spec.certifyState fun _ => (some (mkPercent 101)).isSome) fun _ =>
      Spec.bind Spec.get fun _ => Spec.bind Spec.choose fun value =>
        Spec.bind (Spec.assume (some (mkPercent 101) = some value)) fun _ =>
          Spec.bind (Spec.assume (NTy.Holds percentTy value)) fun _ =>
            Spec.pure (.value value ())
    wp assumeGhost (fun _ _ => False) (fun _ => False) () := by
  intro assumeGhost
  dsimp only [assumeGhost]
  rw [wp_bind, wp_certifyState]
  refine ⟨rfl, ?_⟩
  rw [wp_bind_get, wp_bind, wp_choose]
  intro value
  rw [wp_bind, wp_assume]
  intro eq
  cases Option.some.inj eq
  rw [wp_bind, wp_assume]
  intro held
  exact absurd held not_holds_percent_101

-- Positive 1: `chosen := fun _ => some (mkPercent 42)` decomposes through
-- `wp_ghostAt` + `wp_checkingAt_intro_holds` and delivers `NTy.Holds percentTy v`.
theorem ghostAt_valid_some_holds :
    wp (ghostAt (State := Unit) (ρ := .none) (Γ := .nil) (τ := percentTy)
        1 (fun _ => some (mkPercent 42)) ())
      (fun flow _ => match flow with
        | .value v _ => NTy.Holds percentTy v ∧ v = mkPercent 42
        | _ => False)
      (fun _ => False) () := by
  apply wp_ghostAt
  · rfl
  · intro value hChosen
    cases Option.some.inj hChosen
    apply wp_checkingAt_intro_holds
    · exact holds_percent_42
    · exact fun held => ⟨held, rfl⟩

-- Positive 2a: at invariant-free `u64` (`.int 64 false`), `checkingAt` simplifies
-- directly to `Spec.pure ()`.
theorem checkingAt_u64_eq_pure (site : Nat) (v : (NTy.int 64 false).carrier) :
    checkingAt (State := Unit) site .construction (.int 64 false) v = Spec.pure () := by
  simp

-- Positive 2b: after `wp_ghostAt` at `τ := .int 64 false`, `simp` with
-- `checkingAt_int` and `wp_pure` reduces the standing `checkingAt` continuation
-- directly to `ensures (.value value entry) initial`, costing zero invariant steps.
theorem checkingAt_u64_wp_eq_ensures {ρ : ResultShape} {Γ : NRow} (site : Nat)
    (value : (NTy.int 64 false).carrier) (entry : HEnv Γ)
    (ensures : Flow ρ Γ (NTy.int 64 false).carrier → Unit → Prop)
    (aborts : Failure → Prop) (initial : Unit) :
    wp (checkingAt (State := Unit) site .construction (.int 64 false) value)
        (fun _ after => ensures (.value value entry) after) aborts initial ↔
      ensures (.value value entry) initial := by
  simp only [checkingAt_int, wp_pure]

theorem ghostAt_u64_step_free {ρ : ResultShape} {Γ : NRow} (site : Nat)
    (v : (NTy.int 64 false).carrier) (entry : HEnv Γ)
    (ensures : Flow ρ Γ (NTy.int 64 false).carrier → Unit → Prop)
    (aborts : Failure → Prop) (initial : Unit)
    (post : ensures (.value v entry) initial) :
    wp (ghostAt (State := Unit) (ρ := ρ) (Γ := Γ) (τ := .int 64 false)
        site (fun _ => some v) entry) ensures aborts initial := by
  apply wp_ghostAt
  · rfl
  · intro value hChosen
    cases Option.some.inj hChosen
    simp only [checkingAt_int, wp_pure]
    exact post

end GhostAtBoundary

-- ============================================================================
-- Section 2: `arbitraryIn` Domain-Restricted Havoc (`FABLE-CONSULT-REPLY-13.md` §3, Row 1)
-- ============================================================================

section ArbitraryInBoundary

-- Negative (first): unrestricted `Spec.choose` over `percentTy.carrier` fails
-- to establish `NTy.Holds percentTy` (refuted by counterexample `mkPercent 101`).
theorem unrestricted_choose_percent_refused (entry : HEnv .nil) :
    ¬wp (Spec.bind (Spec.choose : Spec Unit Failure percentTy.carrier) fun value =>
        Spec.pure (Flow.value (ρ := .none) value entry))
      (fun flow _ => match flow with
        | .value v _ => NTy.Holds percentTy v
        | _ => True)
      (fun _ => False) () := by
  intro hwp
  rw [wp_bind, wp_choose] at hwp
  have h101 := hwp (mkPercent 101)
  rw [wp_pure] at h101
  exact not_holds_percent_101 h101

-- Positive: `arbitraryIn` restricts `Spec.choose` by `Spec.assume (NTy.Holds τ value)`
-- and delivers `NTy.Holds percentTy v` on every normal outcome.
theorem arbitraryIn_percent_holds (entry : HEnv .nil) :
    wp (arbitraryIn (State := Unit) (ρ := .none) (Γ := .nil) (τ := percentTy) entry)
      (fun flow _ => match flow with
        | .value v _ => NTy.Holds percentTy v
        | _ => False)
      (fun _ => False) () := by
  unfold arbitraryIn
  rw [wp_bind, wp_choose]
  intro value
  rw [wp_bind, wp_assume]
  intro held
  rw [wp_pure]
  exact held

end ArbitraryInBoundary

-- ============================================================================
-- Section 3: `SliceLender.resolve` `.vector` Settlement (`FABLE-CONSULT-REPLY-13.md` §3, Row 4)
-- ============================================================================

section SliceLenderVectorBoundary

-- Negative (first): mutating an element of a vector-backed slice to invalid
-- `mkPercent 101` makes `SliceLender.resolve` at `.vector` fail `wp` even for a
-- `True` postcondition, because `checkingAt` on the rebuilt vector precedes
-- `Spec.assume` equating it with the valid prophecy `settled`.
theorem sliceLender_resolve_vector_bad :
    let curBad : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 101], by simp⟩, rfl⟩
    let finValid : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 42], by simp⟩, rfl⟩
    let sliceBad : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays curBad finValid
    let settled : (NTy.vector percentTy).carrier := finValid.vector
    let env : HEnv (.cons (.vector percentTy) .nil) := (some settled, ())
    ¬wp ((SliceLender.vector .here .nil).resolve (State := Unit) (ρ := .none) 1 env sliceBad)
      (fun _ _ => True) (fun _ => False) () := by
  intro curBad finValid sliceBad settled env hwp
  change wp
    (Spec.bind (checkingAt 1 .mutationEnd (.vector percentTy) sliceBad.currentVector) fun _ =>
      Spec.bind (Spec.assume (sliceBad.currentVector = settled)) fun _ =>
        Spec.bind (Spec.assume (sliceBad.current = sliceBad.final)) fun _ =>
          Spec.pure (Flow.value (ρ := .none) () env))
    (fun _ _ => True) (fun _ => False) () at hwp
  rw [wp_bind, wp_checkingAt] at hwp
  have hmem : mkPercent 101 ∈ sliceBad.currentVector.values := by
    show mkPercent 101 ∈ #[mkPercent 101]
    simp
  exact not_holds_percent_101 (hwp.1.of_mem hmem)

-- Positive: mutating an element of a vector-backed slice to valid `mkPercent 42`
-- (under `NTy.Holds (.vector percentTy) settled.vector`) satisfies
-- `SliceLender.resolve` at `.vector`, equating `settled.vector` with the updated
-- vector and preserving `NTy.Holds (.vector percentTy)`.
theorem sliceLender_resolve_vector_ok (settled : SpecArray percentTy.carrier 1)
    (_hSettled : NTy.Holds (.vector percentTy) settled.vector) :
    let curOk : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 42], by simp⟩, rfl⟩
    let sliceOk : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays curOk settled
    let env : HEnv (.cons (.vector percentTy) .nil) := (some settled.vector, ())
    wp ((SliceLender.vector .here .nil).resolve (State := Unit) (ρ := .none) 1 env sliceOk)
      (fun flow _ => match flow with
        | .value () outEnv =>
            outEnv = (some curOk.vector, ()) ∧
              NTy.Holds (.vector percentTy) curOk.vector
        | _ => False)
      (fun _ => False) () := by
  intro curOk sliceOk env
  change wp
    (Spec.bind (checkingAt 1 .mutationEnd (.vector percentTy) sliceOk.currentVector) fun _ =>
      Spec.bind (Spec.assume (sliceOk.currentVector = settled.vector)) fun _ =>
        Spec.bind (Spec.assume (sliceOk.current = sliceOk.final)) fun _ =>
          Spec.pure (Flow.value (ρ := .none) () env))
    _ (fun _ => False) ()
  rw [wp_bind]
  apply wp_checkingAt_intro_holds
  · rw [NTy.holds_vector]
    intro elem hmem
    have heq : elem = mkPercent 42 := by
      change elem ∈ #[mkPercent 42] at hmem
      simpa using hmem
    subst heq
    exact holds_percent_42
  · intro held
    rw [wp_bind, wp_assume]
    intro hEqVec
    rw [wp_bind, wp_assume]
    intro _hEqArr
    rw [wp_pure]
    dsimp only [env]
    have hCurEq : curOk.vector = settled.vector := hEqVec
    rw [← hCurEq]
    exact ⟨rfl, held⟩

end SliceLenderVectorBoundary

-- ============================================================================
-- Section 4: 5th Quantifier — Call Result `validResult` (`FABLE-CONSULT-REPLY-13.md` §4)
-- ============================================================================

section CallResultValidBoundary

-- Generated contract shape for a bodiless native callee returning `percentTy`
-- with trivial behavioral clauses: `validResult` carries `ResultShape.Holds`.
private def calleeContract :
    Contract Unit Failure (HList .nil) (ResultShape.carrier (.one percentTy)) where
  requires := fun _ _ => True
  ensures := fun _ _ _ _ => True
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  validArgs := HList.Holds .nil
  validResult := ResultShape.Holds (.one percentTy)

-- Generated contract shape for an opaque callee stating a partial behavioral
-- postcondition (`10 ≤ r.1.val`) while relying on `validResult` for `percentTy`'s
-- nominal invariant (`r.1.val ≤ 100`).
private def opaqueCalleeContract :
    Contract Unit Failure (HList .nil) (ResultShape.carrier (.one percentTy)) where
  requires := fun _ _ => True
  ensures := fun _ _ r _ => 10 ≤ r.1.val
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  validArgs := HList.Holds .nil
  validResult := ResultShape.Holds (.one percentTy)

private def badCallee : HList .nil → Spec Unit Failure (ResultShape.carrier (.one percentTy)) :=
  fun _ => Spec.pure (mkPercent 101)

-- Negative 1 (first): `badCallee` returns `mkPercent 101`, which would satisfy
-- `ensures := True` if `validResult` were omitted (`fun _ => True`), yet fails
-- `wp` for `ResultShape.Holds (.one percentTy)`.
theorem badCallee_satisfies_without_validResult_but_fails_holds :
    Satisfies badCallee { calleeContract with validResult := fun _ => True } ∧
      ¬wp (badCallee ()) (fun r _ => ResultShape.Holds (.one percentTy) r) (fun _ => False) () := by
  refine ⟨?_, ?_⟩
  · intro _ _ _ _ _
    refine ⟨?_, fun _ h => h, fun h => h⟩
    intro result final execution
    rcases execution with ⟨rfl, rfl⟩
    exact ⟨trivial, fun _ => trivial, rfl, fun h => h⟩
  · intro hwp
    rw [badCallee, wp_pure] at hwp
    exact not_holds_percent_101 hwp

-- Negative 2: `calleeContract`'s `validResult` refuses `badCallee`.
theorem badCallee_refused_by_calleeContract :
    ¬Satisfies badCallee calleeContract := by
  intro verified
  have h := (verified () () ⟨⟩ trivial trivial).1 (mkPercent 101) () ⟨rfl, rfl⟩
  exact not_holds_percent_101 h.1

-- Negative 3: without an explicit `Satisfies callee calleeContract` premise,
-- `ResultShape.Holds (.one percentTy)` is not available for an arbitrary callee.
theorem result_holds_unavailable_without_premise :
    ¬(∀ callee : HList .nil → Spec Unit Failure (ResultShape.carrier (.one percentTy)),
        wp (callee ()) (fun r _ => ResultShape.Holds (.one percentTy) r) (fun _ => False) ()) := by
  intro allHold
  exact badCallee_satisfies_without_validResult_but_fails_holds.2 (allHold badCallee)

-- Positive 1 (bodiless native callee): under explicit `(premise : Satisfies callee calleeContract)`,
-- a caller running `callee ()` obtains `ResultShape.Holds (.one percentTy) r`
-- (and hence `r.1.val ≤ 100`) directly from `(premise () () ⟨⟩ trivial trivial).1`.
theorem native_caller_uses_validResult_premise
    (callee : HList .nil → Spec Unit Failure (ResultShape.carrier (.one percentTy)))
    (premise : Satisfies callee calleeContract) :
    wp (callee ())
      (fun r _ => ResultShape.Holds (.one percentTy) r ∧ r.1.val ≤ 100)
      (fun _ => False) () := by
  have established := premise () () ⟨⟩ trivial trivial
  refine ⟨?_, established.2.1, established.2.2⟩
  intro r final execution
  have hr : ResultShape.Holds (.one percentTy) r := (established.1 r final execution).1
  exact ⟨hr, val_le_100_of_holds hr⟩

-- Positive 2 (opaque callee via `wp_call_valid`): under explicit
-- `(premise : Satisfies opaqueCallee opaqueCalleeContract)`, a caller combines
-- the opaque postcondition (`10 ≤ r.1.val`) with `validResult` (`r.1.val ≤ 100`).
theorem opaque_caller_combines_validResult_and_ensures
    (opaqueCallee : HList .nil → Spec Unit Failure (ResultShape.carrier (.one percentTy)))
    (premise : Satisfies opaqueCallee opaqueCalleeContract) :
    wp (opaqueCallee ())
      (fun r _ => 10 ≤ r.1.val ∧ r.1.val ≤ 100)
      (fun _ => False) () :=
  wp_call_valid premise ⟨⟩ trivial trivial
    (fun r _ validRes ensured _ _ => ⟨ensured not_false, val_le_100_of_holds validRes⟩)
    (fun _ impossible => impossible)

end CallResultValidBoundary

end VerexIR.Tests.PlainValueBoundaries
