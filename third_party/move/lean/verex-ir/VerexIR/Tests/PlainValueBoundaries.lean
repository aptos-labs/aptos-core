-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Term
import VerexIR.Proofs.RelationalTransfer

/-!
# Kernel Controls for Plain-Value Boundaries

Kernel-checked negative and positive controls for the six plain-value
boundary mechanisms:
1. `ghostAt` producer boundary (`checkingAt site .construction` and step-free `u64`),
2. `arbitraryIn` domain-restricted havoc (`Spec.assume (NTy.Holds τ value)`),
3. `SliceLender.resolve` `.vector` settlement (certifying check before prophecy equation),
4. Call-result `validResult` (`ResultShape.Holds`) on bodiless native and opaque callees,
5. Global storage write boundaries (`Term.globalPublish` and `Term.ghostStore` with
   `checkingAt site .publish`),
6. Slice chooser kernel decompositions (`wp_borrowArraySliceAt`, `wp_borrowVectorSliceAt`,
   and `wp_reborrowSliceAt`).

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

private theorem holds_percent_100 : NTy.Holds percentTy (mkPercent 100) :=
  (holds_percent_iff 100).mpr (by omega)

private theorem not_holds_percent_101 : ¬NTy.Holds percentTy (mkPercent 101) := by
  rw [holds_percent_iff]
  omega

-- ============================================================================
-- Section 1: `ghostAt` Producer Boundary
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

-- Positive 2: at invariant-free `u64` (`.int 64 false`), `checkingAt` simplifies
-- directly to `Spec.pure ()`.
theorem checkingAt_u64_eq_pure (site : Nat) (v : (NTy.int 64 false).carrier) :
    checkingAt (State := Unit) site .construction (.int 64 false) v = Spec.pure () := by
  simp

-- Positive 3: after `wp_ghostAt` at `τ := .int 64 false`, `simp` with
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
-- Section 2: `arbitraryIn` Domain-Restricted Havoc
-- ============================================================================

section ArbitraryInBoundary

-- Negative 1 (first): unrestricted `Spec.choose` over `percentTy.carrier` fails
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

-- Negative 2: `arbitraryIn` yields nothing stronger than `NTy.Holds` (refuted by
-- valid witness `mkPercent 100`, which satisfies `NTy.Holds percentTy` while `100 > 50`).
theorem arbitraryIn_percent_not_le_50 (entry : HEnv .nil) :
    ¬wp (arbitraryIn (State := Unit) (ρ := .none) (Γ := .nil) (τ := percentTy) entry)
      (fun flow _ => match flow with
        | .value v _ => v.1.val ≤ 50
        | _ => True)
      (fun _ => False) () := by
  intro hwp
  unfold arbitraryIn at hwp
  rw [wp_bind, wp_choose] at hwp
  have h100 := hwp (mkPercent 100)
  rw [wp_bind, wp_assume] at h100
  have hVal := h100 holds_percent_100
  rw [wp_pure] at hVal
  have hNot : ¬((100 : Nat) : Int) ≤ 50 := by omega
  exact hNot hVal

-- Negative 3 (non-vacuity): `arbitraryIn` is not vacuous at a satisfiable type
-- (refuted by valid witness `mkPercent 42`, which satisfies `NTy.Holds percentTy`).
theorem arbitraryIn_percent_nonvacuous (entry : HEnv .nil) :
    ¬wp (arbitraryIn (State := Unit) (ρ := .none) (Γ := .nil) (τ := percentTy) entry)
      (fun _ _ => False) (fun _ => False) () := by
  intro hwp
  unfold arbitraryIn at hwp
  rw [wp_bind, wp_choose] at hwp
  have h42 := hwp (mkPercent 42)
  rw [wp_bind, wp_assume] at h42
  have hFalse := h42 holds_percent_42
  rw [wp_pure] at hFalse
  exact hFalse

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
-- Section 3: `SliceLender.resolve` `.vector` Settlement
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
-- Section 4: Call Result `validResult` (`ResultShape.Holds`)
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
    (fun _r _ validRes ensured _ _ => ⟨ensured not_false, val_le_100_of_holds validRes⟩)
    (fun _ impossible => impossible)

end CallResultValidBoundary

-- ============================================================================
-- Section 5: Global Storage Write Boundaries (`Term.globalPublish`, `Term.ghostStore`)
-- ============================================================================

section GlobalPublishBoundary

attribute [-instance] testCarriers

@[reducible] private def percentFunctions (unit : Validation.ValidatedUnit) :
    FunctionCarriers unit :=
  { FunctionCarriers.runtime unit with own := testCarriers.own }

@[reducible] private noncomputable def percentSkolems (unit : Validation.ValidatedUnit) :
    @Skolems unit (percentFunctions unit) :=
  @Skolems.runtime unit (percentFunctions unit)

attribute [local instance] percentFunctions

@[reducible] private def percentVal (unit : Validation.ValidatedUnit) (n : Nat) :
    @NTy.carrier (percentSkolems unit).toCarriers percentTy :=
  (snum (n : Int), ())

private theorem runtime_holds_percent_iff (unit : Validation.ValidatedUnit) (n : Nat) :
    @NTy.Holds (percentSkolems unit).toCarriers percentTy (percentVal unit n) ↔ n ≤ 100 := by
  rw [@NTy.holds_iff (percentSkolems unit).toCarriers]
  change @NTy.deepHolds testCarriers percentTy (mkPercent n) ↔ n ≤ 100
  rw [← @NTy.holds_iff testCarriers]
  exact holds_percent_iff n

private theorem runtime_holds_percent_50 (unit : Validation.ValidatedUnit) :
    @NTy.Holds (percentSkolems unit).toCarriers percentTy (percentVal unit 50) :=
  (runtime_holds_percent_iff unit 50).mpr (by omega)

private theorem runtime_not_holds_percent_101 (unit : Validation.ValidatedUnit) :
    ¬@NTy.Holds (percentSkolems unit).toCarriers percentTy (percentVal unit 101) := by
  rw [runtime_holds_percent_iff]
  omega

private abbrev publishAddr : String := "0x1"
private abbrev percentLocalRow : NRow := .cons percentTy .nil

@[reducible] private def percentEnv (unit : Validation.ValidatedUnit) (n : Nat) :
    @HEnv (percentSkolems unit).toCarriers percentLocalRow :=
  (some (percentVal unit n), ())

@[reducible] private def percentDone (unit : Validation.ValidatedUnit)
    (v : @NTy.carrier (percentSkolems unit).toCarriers percentTy) :
    @Flow (percentSkolems unit).toCarriers .none percentLocalRow Unit :=
  @Flow.value (percentSkolems unit).toCarriers .none percentLocalRow Unit () (some v, ())

/-- `Term.globalPublish` reading its resource directly from a local variable
(`.var .here`), without passing through `Term.pack`. -/
private def publishPercentTerm (unit : Validation.ValidatedUnit) :
    Term unit .none percentLocalRow .unit :=
  Term.globalPublish 1 .nil (.lit (τ := .address) publishAddr rfl)
    (.var (τ := percentTy) .here)

/-- `Term.ghostStore` reading its ghost resource directly from a local variable
(`.var .here`), without passing through `Term.pack`. -/
private def ghostStorePercentTerm (unit : Validation.ValidatedUnit) :
    Term unit .none percentLocalRow .unit :=
  Term.ghostStore 1 .nil (.lit (τ := .address) publishAddr rfl)
    (.var (τ := percentTy) .here)

private def emptyMem (unit : Validation.ValidatedUnit) : Memory unit :=
  fun _ _ => none

private noncomputable def occupiedMem (unit : Validation.ValidatedUnit) : Memory unit :=
  Skolems.writePlain (Θ := percentSkolems unit) percentTy .nil (.address publishAddr)
    (some (percentVal unit 50)) (emptyMem unit)

private theorem publishPercentTerm_denote {unit : Validation.ValidatedUnit}
    {executable : Validation.ExecutableUnit unit}
    (meanings : @Meanings unit (percentFunctions unit) executable (percentSkolems unit))
    (v : @NTy.carrier (percentSkolems unit).toCarriers percentTy) :
    Term.denote (Θ := percentSkolems unit) meanings (publishPercentTerm unit) (some v, ()) =
      (Spec.bind Spec.get fun memory =>
        match Skolems.plainAt (Θ := percentSkolems unit) percentTy .nil
            (.address publishAddr) memory with
        | some _ => abortAt 1 (Failure.ofInts .abort #[])
        | none =>
            Spec.bind (@checkingAt (percentSkolems unit).toCarriers (Memory unit)
                1 .publish percentTy v) fun _ =>
              Spec.bind (Spec.set (Skolems.writePlain (Θ := percentSkolems unit) percentTy .nil
                  (.address publishAddr) (some v) memory))
                fun _ => Spec.pure (percentDone unit v)) := by
  letI : Skolems unit := percentSkolems unit
  dsimp only [publishPercentTerm, Term.denote, Term.denoteIn, Flow.bind, anchorAt,
    Meanings.interpreterOps, Var.get, NTy.ofGround, NTy.storageKey, percentDone]
  simp only [Spec.pure_bind]
  congr 1
  funext memory
  cases Skolems.plainAt (Θ := percentSkolems unit) percentTy .nil (.address publishAddr) memory <;>
    rfl

private theorem ghostStorePercentTerm_denote {unit : Validation.ValidatedUnit}
    {executable : Validation.ExecutableUnit unit}
    (meanings : @Meanings unit (percentFunctions unit) executable (percentSkolems unit))
    (v : @NTy.carrier (percentSkolems unit).toCarriers percentTy) :
    Term.denote (Θ := percentSkolems unit) meanings (ghostStorePercentTerm unit) (some v, ()) =
      (Spec.bind Spec.get fun memory =>
        Spec.bind (@checkingAt (percentSkolems unit).toCarriers (Memory unit)
            1 .publish percentTy v) fun _ =>
          Spec.bind (Spec.set (Skolems.writePlain (Θ := percentSkolems unit) percentTy .nil
              (.address publishAddr) (some v) memory))
            fun _ => Spec.pure (percentDone unit v)) := by
  letI : Skolems unit := percentSkolems unit
  dsimp only [ghostStorePercentTerm, Term.denote, Term.denoteIn, Flow.bind, anchorAt,
    Meanings.interpreterOps, Var.get, NTy.ofGround, NTy.storageKey, percentDone]
  simp only [Spec.pure_bind]

-- Negative 1 (first): on an unoccupied cell, `Term.globalPublish` of an invalid
-- plain `Percent` (`percentVal unit 101`) from `.var .here` fails
-- `checkingAt site .publish` and refutes `wp` even for a `True` postcondition.
theorem globalPublish_invalid_not_wp_true {unit : Validation.ValidatedUnit}
    {executable : Validation.ExecutableUnit unit}
    (meanings : @Meanings unit (percentFunctions unit) executable (percentSkolems unit))
    (aborts : Failure → Prop) :
    ¬wp (Term.denote (Θ := percentSkolems unit) meanings
          (publishPercentTerm unit) (percentEnv unit 101))
      (fun _ _ => True) aborts (emptyMem unit) := by
  letI : Skolems unit := percentSkolems unit
  intro hwp
  rw [publishPercentTerm_denote meanings (percentVal unit 101), wp_bind_get] at hwp
  have hEmpty : Skolems.plainAt (Θ := percentSkolems unit) percentTy .nil
      (.address publishAddr) (emptyMem unit) = none := by
    simp [emptyMem, Skolems.plainAt_def]
  rw [hEmpty] at hwp
  dsimp only at hwp
  rw [wp_bind, wp_checkingAt] at hwp
  exact runtime_not_holds_percent_101 unit hwp.1

-- Negative 2: `Term.ghostStore` of an invalid plain `Percent` (`percentVal unit 101`)
-- from `.var .here` fails `checkingAt site .publish` and refutes `wp` even for a
-- `True` postcondition.
theorem ghostStore_invalid_not_wp_true {unit : Validation.ValidatedUnit}
    {executable : Validation.ExecutableUnit unit}
    (meanings : @Meanings unit (percentFunctions unit) executable (percentSkolems unit))
    (aborts : Failure → Prop) :
    ¬wp (Term.denote (Θ := percentSkolems unit) meanings
          (ghostStorePercentTerm unit) (percentEnv unit 101))
      (fun _ _ => True) aborts (emptyMem unit) := by
  letI : Skolems unit := percentSkolems unit
  intro hwp
  rw [ghostStorePercentTerm_denote meanings (percentVal unit 101), wp_bind_get,
    wp_bind, wp_checkingAt] at hwp
  exact runtime_not_holds_percent_101 unit hwp.1

-- Control 3: on an already-occupied cell, `Term.globalPublish` of an invalid plain
-- `Percent` (`percentVal unit 101`) from `.var .here` aborts without
-- reaching `checkingAt`, proving an already-aborting publish owes no check.
theorem globalPublish_occupied_invalid_aborts {unit : Validation.ValidatedUnit}
    {executable : Validation.ExecutableUnit unit}
    (meanings : @Meanings unit (percentFunctions unit) executable (percentSkolems unit)) :
    wp (Term.denote (Θ := percentSkolems unit) meanings
          (publishPercentTerm unit) (percentEnv unit 101))
      (fun _ _ => False) (fun err => err = Failure.ofInts .abort #[]) (occupiedMem unit) ∧
      (Term.denote (Θ := percentSkolems unit) meanings
          (publishPercentTerm unit) (percentEnv unit 101)).aborts
        (occupiedMem unit) (Failure.ofInts .abort #[]) := by
  letI : Skolems unit := percentSkolems unit
  have hOcc : Skolems.plainAt (Θ := percentSkolems unit) percentTy .nil
      (.address publishAddr) (occupiedMem unit) = some (percentVal unit 50) := by
    unfold occupiedMem
    rw [Skolems.plainAt_writePlain_some (Θ := percentSkolems unit) percentTy .nil
      (.address publishAddr) (.address publishAddr) (emptyMem unit) (percentVal unit 50)
      (runtime_holds_percent_50 unit), if_pos rfl]
  constructor
  · rw [publishPercentTerm_denote meanings (percentVal unit 101), wp_bind_get, hOcc]
    dsimp only [abortAt]
    rw [wp_abort]
  · rw [publishPercentTerm_denote meanings (percentVal unit 101)]
    exact .inr ⟨occupiedMem unit, occupiedMem unit, ⟨rfl, rfl⟩, by dsimp only; rw [hOcc]; rfl⟩

-- Positive 1: on an unoccupied cell, `Term.globalPublish` of a valid `Percent`
-- (`percentVal unit 50`) from `.var .here` satisfies `wp` and reads back
-- `some (percentVal unit 50)` via `Skolems.plainAt`.
theorem globalPublish_valid_reads_back {unit : Validation.ValidatedUnit}
    {executable : Validation.ExecutableUnit unit}
    (meanings : @Meanings unit (percentFunctions unit) executable (percentSkolems unit)) :
    wp (Term.denote (Θ := percentSkolems unit) meanings
          (publishPercentTerm unit) (percentEnv unit 50))
      (fun flow mem =>
        flow = percentDone unit (percentVal unit 50) ∧
          Skolems.plainAt (Θ := percentSkolems unit) percentTy .nil
            (.address publishAddr) mem = some (percentVal unit 50))
      (fun _ => False)
      (emptyMem unit) := by
  letI : Skolems unit := percentSkolems unit
  rw [publishPercentTerm_denote meanings (percentVal unit 50), wp_bind_get]
  have hEmpty : Skolems.plainAt (Θ := percentSkolems unit) percentTy .nil
      (.address publishAddr) (emptyMem unit) = none := by
    simp [emptyMem, Skolems.plainAt_def]
  rw [hEmpty]
  dsimp only
  rw [wp_bind, wp_checkingAt]
  refine ⟨runtime_holds_percent_50 unit, fun held => ?_⟩
  rw [wp_bind, Denote.wp_set, wp_pure]
  refine ⟨rfl, ?_⟩
  rw [Skolems.plainAt_writePlain_some (Θ := percentSkolems unit) percentTy .nil
    (.address publishAddr) (.address publishAddr) (emptyMem unit) (percentVal unit 50) held,
    if_pos rfl]

-- Positive 2: on an unoccupied cell, `Term.ghostStore` of a valid `Percent`
-- (`percentVal unit 50`) from `.var .here` satisfies `wp` and reads back
-- `some (percentVal unit 50)` via `Skolems.plainAt`.
theorem ghostStore_valid_reads_back {unit : Validation.ValidatedUnit}
    {executable : Validation.ExecutableUnit unit}
    (meanings : @Meanings unit (percentFunctions unit) executable (percentSkolems unit)) :
    wp (Term.denote (Θ := percentSkolems unit) meanings
          (ghostStorePercentTerm unit) (percentEnv unit 50))
      (fun flow mem =>
        flow = percentDone unit (percentVal unit 50) ∧
          Skolems.plainAt (Θ := percentSkolems unit) percentTy .nil
            (.address publishAddr) mem = some (percentVal unit 50))
      (fun _ => False)
      (emptyMem unit) := by
  letI : Skolems unit := percentSkolems unit
  rw [ghostStorePercentTerm_denote meanings (percentVal unit 50), wp_bind_get,
    wp_bind, wp_checkingAt]
  refine ⟨runtime_holds_percent_50 unit, fun held => ?_⟩
  rw [wp_bind, Denote.wp_set, wp_pure]
  refine ⟨rfl, ?_⟩
  rw [Skolems.plainAt_writePlain_some (Θ := percentSkolems unit) percentTy .nil
    (.address publishAddr) (.address publishAddr) (emptyMem unit) (percentVal unit 50) held,
    if_pos rfl]

end GlobalPublishBoundary

-- ============================================================================
-- Section 6: Slice Chooser Decomposition Boundaries
-- ============================================================================

section SliceChooserBoundary

private abbrev eitherSliceHandle : StructHandle := ⟨⟨0⟩, 1⟩
private abbrev sliceSlotHandle : StructHandle := ⟨⟨0⟩, 2⟩
private abbrev eitherSliceTy : NTy :=
  .enum eitherSliceHandle .nil ["Slice", "Empty"]
    (.cons (.cons (.sliceRef percentTy) .nil) (.cons (.cons .unit .nil) .nil)) (by decide)
private abbrev sliceSlotTy : NTy := .struct sliceSlotHandle .nil (.cons (.sliceRef percentTy) .nil)

-- Negative 1 (first): `reborrowSliceAt` through an open `&mut` reference (`hasDeref = true`)
-- with an invalid current element (`mkPercent 101`) fails `checkingAt site .handOver`
-- and refutes `wp` even for a `True` postcondition.
theorem reborrowSliceAt_open_invalid_refused :
    let arrBad : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 101], by simp⟩, rfl⟩
    let arrOk : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 42], by simp⟩, rfl⟩
    let sliceBad : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arrBad arrOk
    let sliceOk : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arrOk arrOk
    let env : HEnv (.cons (.ref (.sliceRef percentTy)) .nil) := (some (sliceBad, sliceOk), ())
    ¬wp (reborrowSliceAt (State := Unit) (ρ := .none) 1 none .here (.deref .nil) env)
      (fun _ _ => True) (fun _ => False) () := by
  intro arrBad arrOk sliceBad sliceOk env hwp
  rw [wp_reborrowSliceAt] at hwp
  dsimp only [env, Var.get, Proj.get?_deref, Proj.get?_nil, Proj.hasDeref_deref] at hwp
  rw [if_pos rfl, wp_checkingAt] at hwp
  have hmem : mkPercent 101 ∈ sliceBad.currentVector.values := by
    show mkPercent 101 ∈ #[mkPercent 101]
    simp
  exact not_holds_percent_101 ((NTy.holds_slice percentTy _).mp hwp.1 _ hmem)

-- Negative 2: `reborrowSliceAt` when `path.get?` resolves to `none` (wrong enum variant)
-- and `mismatch = some err` aborts with `err` and fails `wp` under `aborts := False`.
theorem reborrowSliceAt_mismatch_some_refused (err : Failure) :
    let rightVal : eitherSliceTy.carrier := Which.inject (.there .here) ((), ())
    let env : HEnv (.cons eitherSliceTy .nil) := (some rightVal, ())
    let path : Proj eitherSliceTy (.sliceRef percentTy) := .variant (.cons .here .here .nil) .nil
    ¬wp (reborrowSliceAt (State := Unit) (ρ := .none) 1 (some err) .here path env)
      (fun _ _ => True) (fun _ => False) () := by
  intro rightVal env path hwp
  rw [wp_reborrowSliceAt] at hwp
  simp [env, rightVal, path, Var.get, Proj.get?_variant, Choices.select?, Which.inject,
    mismatchOutcome_some, wp_abort] at hwp

-- Negative 3: `reborrowSliceAt` when `path.get?` resolves to `none` and `mismatch = none`
-- is `Spec.stuck`, failing `wp` even when both `ensures` and `aborts` are `True`.
theorem reborrowSliceAt_mismatch_none_refused :
    let rightVal : eitherSliceTy.carrier := Which.inject (.there .here) ((), ())
    let env : HEnv (.cons eitherSliceTy .nil) := (some rightVal, ())
    let path : Proj eitherSliceTy (.sliceRef percentTy) := .variant (.cons .here .here .nil) .nil
    ¬wp (reborrowSliceAt (State := Unit) (ρ := .none) 1 none .here path env)
      (fun _ _ => True) (fun _ => True) () := by
  intro rightVal env path hwp
  rw [wp_reborrowSliceAt] at hwp
  simp [env, rightVal, path, Var.get, Proj.get?_variant, Choices.select?, Which.inject,
    mismatchOutcome_none, wp_stuck] at hwp

-- Negative 4: `reborrowSliceAt` constrains `final` by `NTy.Holds` (`≤ 100`) and nothing
-- stronger: claiming every chosen `final` element satisfies `≤ 50` fails `¬wp`
-- (refuted by valid witness `mkPercent 100`).
theorem reborrowSliceAt_valid_not_le_50 :
    let arr42 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 42], by simp⟩, rfl⟩
    let sliceOk : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arr42 arr42
    let env : HEnv (.cons (.sliceRef percentTy) .nil) := (some sliceOk, ())
    ¬wp (reborrowSliceAt (State := Unit) (ρ := .none) 1 none .here .nil env)
      (fun flow _ => match flow with
        | .value s _ => ∀ q ∈ s.finalVector.values, q.1.val ≤ 50
        | _ => True)
      (fun _ => False) () := by
  intro arr42 sliceOk env hwp
  rw [wp_reborrowSliceAt] at hwp
  dsimp only [env, Var.get, Proj.get?_nil, Proj.hasDeref_nil] at hwp
  rw [if_neg Bool.false_ne_true] at hwp
  let arr100 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 100], by simp⟩, rfl⟩
  have hProph : Prophecy (Var.here : Var (.cons (.sliceRef percentTy) .nil) (.sliceRef percentTy))
      .nil (SpecSliceRef.ofArrays sliceOk.current arr100, sliceOk.withCurrent arr100) := by
    unfold Prophecy
    trivial
  have hHold42 : ∀ q ∈ arr42.vector.values, NTy.Holds percentTy q := by
    intro q hq
    have heq : q = mkPercent 42 := by
      change q ∈ #[mkPercent 42] at hq
      simpa using hq
    exact heq ▸ holds_percent_42
  have hHold100 : ∀ q ∈ arr100.vector.values, NTy.Holds percentTy q := by
    intro q hq
    have heq : q = mkPercent 100 := by
      change q ∈ #[mkPercent 100] at hq
      simpa using hq
    exact heq ▸ holds_percent_100
  have hSlice : NTy.Holds (.sliceRef percentTy) (SpecSliceRef.ofArrays sliceOk.current arr100) :=
    (NTy.holds_sliceRef percentTy _).mpr ⟨hHold42, hHold100⟩
  have hOwner : NTy.Holds (.sliceRef percentTy) (sliceOk.withCurrent arr100) :=
    (NTy.holds_sliceRef percentTy _).mpr ⟨hHold100, hHold42⟩
  have hPost := hwp arr100 (sliceOk.withCurrent arr100) hProph hSlice hOwner rfl
  have h100le50 : (mkPercent 100).1.val ≤ 50 :=
    hPost (mkPercent 100) (by show mkPercent 100 ∈ #[mkPercent 100]; simp)
  have : ¬((100 : Nat) : Int) ≤ 50 := by omega
  exact this h100le50

-- Negative 5 (non-vacuity): `reborrowSliceAt` on a valid slice is not vacuous.
theorem reborrowSliceAt_valid_nonvacuous :
    let arr42 : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 42], by simp⟩, rfl⟩
    let sliceOk : (NTy.sliceRef percentTy).carrier := SpecSliceRef.ofArrays arr42 arr42
    let env : HEnv (.cons (.sliceRef percentTy) .nil) := (some sliceOk, ())
    ¬wp (reborrowSliceAt (State := Unit) (ρ := .none) 1 none .here .nil env)
      (fun _ _ => False) (fun _ => False) () := by
  intro arr42 sliceOk env hwp
  rw [wp_reborrowSliceAt] at hwp
  dsimp only [env, Var.get, Proj.get?_nil, Proj.hasDeref_nil] at hwp
  rw [if_neg Bool.false_ne_true] at hwp
  have hProph : Prophecy (Var.here : Var (.cons (.sliceRef percentTy) .nil) (.sliceRef percentTy))
      .nil (SpecSliceRef.ofArrays sliceOk.current arr42, sliceOk.withCurrent arr42) := by
    unfold Prophecy
    trivial
  have hHold42 : ∀ q ∈ arr42.vector.values, NTy.Holds percentTy q := by
    intro q hq
    have heq : q = mkPercent 42 := by
      change q ∈ #[mkPercent 42] at hq
      simpa using hq
    exact heq ▸ holds_percent_42
  have hSlice : NTy.Holds (.sliceRef percentTy) (SpecSliceRef.ofArrays sliceOk.current arr42) :=
    (NTy.holds_sliceRef percentTy _).mpr ⟨hHold42, hHold42⟩
  exact hwp arr42 (sliceOk.withCurrent arr42) hProph hSlice hSlice rfl

-- Negative 6: `borrowArraySliceAt` through an open `&mut` reference (`hasDeref = true`)
-- with an invalid element (`mkPercent 101`) fails `checkingAt site .handOver`.
theorem borrowArraySliceAt_open_invalid_refused :
    let arrBad : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 101], by simp⟩, rfl⟩
    let arrOk : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 42], by simp⟩, rfl⟩
    let env : HEnv (.cons (.ref (.array percentTy 1)) .nil) := (some (arrBad, arrOk), ())
    ¬wp (borrowArraySliceAt (State := Unit) (ρ := .none) 1 none .here (.deref .nil) env)
      (fun _ _ => True) (fun _ => False) () := by
  intro arrBad arrOk env hwp
  rw [wp_borrowArraySliceAt] at hwp
  dsimp only [env, Var.get, Proj.get?_deref, Proj.get?_nil, Proj.hasDeref_deref] at hwp
  rw [if_pos rfl, wp_checkingAt] at hwp
  have hmem : mkPercent 101 ∈ arrBad.vector.values := by
    show mkPercent 101 ∈ #[mkPercent 101]
    simp
  exact not_holds_percent_101 ((NTy.holds_array percentTy 1 _).mp hwp.1 _ hmem)

-- Negative 7: `borrowVectorSliceAt` through an open `&mut` reference (`hasDeref = true`)
-- with an invalid element (`mkPercent 101`) fails `checkingAt site .handOver`.
theorem borrowVectorSliceAt_open_invalid_refused :
    let vecBad : (NTy.vector percentTy).carrier := ⟨#[mkPercent 101], by simp⟩
    let vecOk : (NTy.vector percentTy).carrier := ⟨#[mkPercent 42], by simp⟩
    let env : HEnv (.cons (.ref (.vector percentTy)) .nil) := (some (vecBad, vecOk), ())
    ¬wp (borrowVectorSliceAt (State := Unit) (ρ := .none) 1 none .here (.deref .nil) env)
      (fun _ _ => True) (fun _ => False) () := by
  intro vecBad vecOk env hwp
  rw [wp_borrowVectorSliceAt] at hwp
  dsimp only [env, Var.get, Proj.get?_deref, Proj.get?_nil, Proj.hasDeref_deref] at hwp
  rw [if_pos rfl, wp_checkingAt] at hwp
  have hmem : mkPercent 101 ∈ vecBad.values := by
    show mkPercent 101 ∈ #[mkPercent 101]
    simp
  exact not_holds_percent_101 ((NTy.holds_vector percentTy _).mp hwp.1 _ hmem)

-- Positive 1: `wp_reborrowSliceAt` decomposes `reborrowSliceAt` directly to
-- `SpecSliceRef.ofArrays p.current final`, immediately establishing `0 < s.extent`
-- from `0 < p.extent` and preserving `NTy.Holds (.sliceRef percentTy) s`.
theorem reborrowSliceAt_valid_extent_and_holds
    (p : (NTy.sliceRef percentTy).carrier) (hPos : 0 < p.extent) :
    let env : HEnv (.cons (.sliceRef percentTy) .nil) := (some p, ())
    wp (reborrowSliceAt (State := Unit) (ρ := .none) 1 none .here .nil env)
      (fun flow _ => match flow with
        | .value s outEnv =>
            0 < s.extent ∧ s.extent = p.extent ∧
              NTy.Holds (.sliceRef percentTy) s ∧
              ∃ final : SpecArray percentTy.carrier p.extent,
                s = SpecSliceRef.ofArrays p.current final ∧
                  outEnv = (some (p.withCurrent final), ())
        | _ => False)
      (fun _ => False) () := by
  intro env
  rw [wp_reborrowSliceAt]
  dsimp only [env, Var.get, Proj.get?_nil, Proj.hasDeref_nil, Proj.set?_nil, Var.set]
  rw [if_neg Bool.false_ne_true]
  intro final rebuilt _hProph hSlice _hOwner hSet
  cases Option.some.inj hSet
  exact ⟨hPos, rfl, hSlice, final, rfl, rfl⟩

-- Positive 2: `wp_borrowArraySliceAt` on a valid open `&mut [Percent; 1]` (`hasDeref = true`)
-- discharges `.handOver` and directly exposes `s.extent = 1` and `NTy.Holds`.
theorem borrowArraySliceAt_open_valid_extent_and_holds
    (prophArr : SpecArray percentTy.carrier 1) :
    let curArr : SpecArray percentTy.carrier 1 := ⟨⟨#[mkPercent 42], by simp⟩, rfl⟩
    let env : HEnv (.cons (.ref (.array percentTy 1)) .nil) := (some (curArr, prophArr), ())
    wp (borrowArraySliceAt (State := Unit) (ρ := .none) 1 none .here (.deref .nil) env)
      (fun flow _ => match flow with
        | .value s outEnv =>
            s.extent = 1 ∧ 0 < s.extent ∧
              NTy.Holds (.sliceRef percentTy) s ∧
              ∃ final : SpecArray percentTy.carrier 1,
                s = SpecSliceRef.ofArrays curArr final ∧
                  outEnv = (some (final, prophArr), ())
        | _ => False)
      (fun _ => False) () := by
  intro curArr env
  rw [wp_borrowArraySliceAt]
  dsimp only [env, Var.get, Proj.get?_deref, Proj.get?_nil, Proj.hasDeref_deref,
    Proj.set?_deref, Proj.set?_nil, Option.map_some, Var.set]
  rw [if_pos rfl]
  apply wp_checkingAt_intro_holds
  · rw [NTy.holds_array]
    intro elem hmem
    have heq : elem = mkPercent 42 := by
      change elem ∈ #[mkPercent 42] at hmem
      simpa using hmem
    exact heq ▸ holds_percent_42
  · intro _held final rebuilt _hProph hSlice _hOwner hSet
    cases Option.some.inj hSet
    exact ⟨rfl, Nat.zero_lt_one, hSlice, final, rfl, rfl⟩

-- Positive 3: `wp_borrowVectorSliceAt` on a valid open `&mut Vec<Percent>` (`hasDeref = true`)
-- discharges `.handOver` and directly exposes `s.extent = curVec.values.size` and `NTy.Holds`.
theorem borrowVectorSliceAt_open_valid_extent_and_holds
    (prophVec : (NTy.vector percentTy).carrier) :
    let curVec : (NTy.vector percentTy).carrier := ⟨#[mkPercent 42], by simp⟩
    let env : HEnv (.cons (.ref (.vector percentTy)) .nil) := (some (curVec, prophVec), ())
    wp (borrowVectorSliceAt (State := Unit) (ρ := .none) 1 none .here (.deref .nil) env)
      (fun flow _ => match flow with
        | .value s outEnv =>
            s.extent = curVec.values.size ∧ 0 < s.extent ∧
              NTy.Holds (.sliceRef percentTy) s ∧
              ∃ final : SpecArray percentTy.carrier curVec.values.size,
                s = SpecSliceRef.ofArrays ⟨curVec, rfl⟩ final ∧
                  outEnv = (some (final.vector, prophVec), ())
        | _ => False)
      (fun _ => False) () := by
  intro curVec env
  rw [wp_borrowVectorSliceAt]
  dsimp only [env, Var.get, Proj.get?_deref, Proj.get?_nil, Proj.hasDeref_deref,
    Proj.set?_deref, Proj.set?_nil, Option.map_some, Var.set]
  rw [if_pos rfl]
  apply wp_checkingAt_intro_holds
  · rw [NTy.holds_vector]
    intro elem hmem
    have heq : elem = mkPercent 42 := by
      change elem ∈ #[mkPercent 42] at hmem
      simpa using hmem
    exact heq ▸ holds_percent_42
  · intro _held final rebuilt _hProph hSlice _hOwner hSet
    cases Option.some.inj hSet
    exact ⟨rfl, by simp [SpecSliceRef.ofArrays, curVec], hSlice, final, rfl, rfl⟩

-- Positive 4: equivalence control on `SliceSlot`: `wp (reborrowSliceAt ... (.field .here .nil))`
-- holds iff the `final`/`rebuilt` continuation holds, with `s = SpecSliceRef.ofArrays p.current final`
-- directly exposing `s.extent = p.extent` without any existential hypothesis.
theorem reborrowSliceAt_slot_wp_iff
    (p : (NTy.sliceRef percentTy).carrier)
    (ensures : Flow .none (.cons sliceSlotTy .nil) (NTy.sliceRef percentTy).carrier → Unit → Prop)
    (aborts : Failure → Prop) :
    let env : HEnv (.cons sliceSlotTy .nil) := (some (p, ()), ())
    let path : Proj sliceSlotTy (.sliceRef percentTy) := .field .here .nil
    wp (reborrowSliceAt 1 none .here path env) ensures aborts () ↔
      ∀ (final : SpecArray percentTy.carrier p.extent) (rebuilt : sliceSlotTy.carrier),
        Prophecy (Var.here : Var (.cons sliceSlotTy .nil) sliceSlotTy) path
            (SpecSliceRef.ofArrays p.current final, rebuilt) →
          NTy.Holds (.sliceRef percentTy) (SpecSliceRef.ofArrays p.current final) →
            NTy.Holds sliceSlotTy rebuilt →
              some (p.withCurrent final, ()) = some rebuilt →
                ensures (.value (SpecSliceRef.ofArrays p.current final) (some rebuilt, ())) () := by
  intro env path
  rw [wp_reborrowSliceAt]
  dsimp only [env, path, Var.get, Proj.get?_field, Proj.get?_nil, Var.select_here,
    Proj.hasDeref, Proj.hasDeref_nil, Proj.set?_field, Proj.set?_nil, Var.update_here,
    Option.map_some, Var.set]
  rw [if_neg Bool.false_ne_true]
  rfl

-- Positive 5: `x.get env = none` (`Spec.bottom` branch) reduces `wp` to `True`.
theorem reborrowSliceAt_uninitialized_wp_iff
    (ensures : Flow .none (.cons (.sliceRef percentTy) .nil) (NTy.sliceRef percentTy).carrier → Unit → Prop)
    (aborts : Failure → Prop) :
    let env : HEnv (.cons (.sliceRef percentTy) .nil) := (none, ())
    wp (reborrowSliceAt 1 none .here .nil env) ensures aborts () ↔ True := by
  intro env
  rw [wp_reborrowSliceAt]
  rfl

end SliceChooserBoundary

-- Reject any unexpected axiom in any boundary control theorem.
open Lean Elab Command in
run_cmd do
  for name in [
      ``ghostAt_invalid_some_refused, ``ghostAt_none_refused,
      ``ghostAt_assume_would_vacuously_prove_false, ``ghostAt_valid_some_holds,
      ``checkingAt_u64_eq_pure, ``checkingAt_u64_wp_eq_ensures, ``ghostAt_u64_step_free,
      ``unrestricted_choose_percent_refused, ``arbitraryIn_percent_not_le_50,
      ``arbitraryIn_percent_nonvacuous, ``arbitraryIn_percent_holds,
      ``sliceLender_resolve_vector_bad, ``sliceLender_resolve_vector_ok,
      ``badCallee_satisfies_without_validResult_but_fails_holds,
      ``badCallee_refused_by_calleeContract, ``result_holds_unavailable_without_premise,
      ``native_caller_uses_validResult_premise, ``opaque_caller_combines_validResult_and_ensures,
      ``globalPublish_invalid_not_wp_true, ``ghostStore_invalid_not_wp_true,
      ``globalPublish_occupied_invalid_aborts, ``globalPublish_valid_reads_back,
      ``ghostStore_valid_reads_back,
      ``reborrowSliceAt_open_invalid_refused, ``reborrowSliceAt_mismatch_some_refused,
      ``reborrowSliceAt_mismatch_none_refused, ``reborrowSliceAt_valid_not_le_50,
      ``reborrowSliceAt_valid_nonvacuous, ``borrowArraySliceAt_open_invalid_refused,
      ``borrowVectorSliceAt_open_invalid_refused, ``reborrowSliceAt_valid_extent_and_holds,
      ``borrowArraySliceAt_open_valid_extent_and_holds,
      ``borrowVectorSliceAt_open_valid_extent_and_holds,
      ``reborrowSliceAt_slot_wp_iff, ``reborrowSliceAt_uninitialized_wp_iff] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end VerexIR.Tests.PlainValueBoundaries
