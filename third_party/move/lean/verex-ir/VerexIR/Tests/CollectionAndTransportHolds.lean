-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Resources
import VerexIR.Proofs.Denote.VectorOperations
import VerexIR.Proofs.Denote.NativeMaps

namespace VerexIR.Tests.CollectionAndTransportHolds

open VerexIR VerexIR.Proofs.Denote

/-! # Unit tests for frame/skolem transport `Holds` laws and collection/`SpecMap` `Holds` lemmas -/

-- ============================================================================
-- Section 1: Skolem Carrier Transport `Holds` Laws (`vir_transport`)
-- ============================================================================

section TransportTests
variable [Θ : Carriers]

-- Negative 1 (first): transporting an invalid value (`¬Holds`) into a skolem
-- frame via `NTy.toSkolem` does not manufacture `NTy.Holds`.
example (θ : TypeArgs) (τ : NTy) (v : (τ.subst θ.1).carrier)
    (bad : ¬@NTy.Holds Θ (τ.subst θ.1) v) :
    ¬@NTy.Holds (Carriers.instantiate θ Θ) τ (NTy.toSkolem θ τ v) := by
  simp only [vir_transport]
  exact bad

-- Negative 2: transporting an invalid skolem value back via `NTy.ofSkolem`
-- does not manufacture `NTy.Holds`.
example (θ : TypeArgs) (τ : NTy) (v : @NTy.carrier ((Carriers.instantiate θ Θ)).toSorts τ)
    (bad : ¬@NTy.Holds (Carriers.instantiate θ Θ) τ v) :
    ¬@NTy.Holds Θ (τ.subst θ.1) (NTy.ofSkolem θ τ v) := by
  simp only [vir_transport]
  exact bad

-- Positive 1–6: `toSkolem` and `ofSkolem` preserve `Holds` bijectively across
-- `NTy`, `HList`, and `variantCarrier`.
example (θ : TypeArgs) (τ : NTy) (v : (τ.subst θ.1).carrier) :
    (@NTy.Holds (Carriers.instantiate θ Θ) τ (NTy.toSkolem θ τ v)) ↔
      @NTy.Holds Θ (τ.subst θ.1) v := by
  simp only [vir_transport]

example (θ : TypeArgs) (τ : NTy) (v : @NTy.carrier ((Carriers.instantiate θ Θ)).toSorts τ) :
    (@NTy.Holds Θ (τ.subst θ.1) (NTy.ofSkolem θ τ v)) ↔
      @NTy.Holds (Carriers.instantiate θ Θ) τ v := by
  simp only [vir_transport]

example (θ : TypeArgs) (row : NRow) (vs : HList (NRow.subst θ.1 row)) :
    (@HList.Holds (Carriers.instantiate θ Θ) row (HList.toSkolem θ row vs)) ↔
      @HList.Holds Θ (NRow.subst θ.1 row) vs := by
  simp only [vir_transport]

example (θ : TypeArgs) (row : NRow) (vs : @HList ((Carriers.instantiate θ Θ)).toSorts row) :
    (@HList.Holds Θ (NRow.subst θ.1 row) (HList.ofSkolem θ row vs)) ↔
      @HList.Holds (Carriers.instantiate θ Θ) row vs := by
  simp only [vir_transport]

example (θ : TypeArgs) (names : List String) (rows : NRows)
    (v : variantCarrier names (NRows.subst θ.1 rows)) :
    (@variantCarrier.Holds (Carriers.instantiate θ Θ) names rows
        (variantCarrier.toSkolem θ names rows v)) ↔
      @variantCarrier.Holds Θ names (NRows.subst θ.1 rows) v := by
  simp only [vir_transport]

example (θ : TypeArgs) (names : List String) (rows : NRows)
    (v : @variantCarrier ((Carriers.instantiate θ Θ)).toSorts names rows) :
    (@variantCarrier.Holds Θ names (NRows.subst θ.1 rows)
        (variantCarrier.ofSkolem θ names rows v)) ↔
      @variantCarrier.Holds (Carriers.instantiate θ Θ) names rows v := by
  simp only [vir_transport]

end TransportTests

-- ============================================================================
-- Section 2: `Skolems` Frame Transport `Holds` Laws (`vir_transport`)
-- ============================================================================

section FrameTests
variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]

-- Negative 1 (first): `Θ.toRuntime` preserves invalidity (`¬NTy.Holds`).
example [Θ : Skolems unit] (τ : NTy) (v : @NTy.carrier (Θ.toCarriers).toSorts τ)
    (bad : ¬@NTy.Holds Θ.toCarriers τ v) :
    ¬@NTy.Holds (Carriers.runtime unit) (Θ.resolve τ) (Θ.toRuntime τ v) := by
  simp only [vir_transport]
  exact bad

-- Negative 2: `Θ.reframe` preserves invalidity (`¬NTy.Holds`).
example (Θ : Skolems unit) {params : NRow}
    (own : params.isParamsFrom 0 = true) {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (v : @NTy.carrier (Θ.toCarriers).toSorts τ) (bad : ¬@NTy.Holds Θ.toCarriers τ v) :
    ¬@NTy.Holds (Θ.instance params own).toCarriers τ (Θ.reframe own bounded v) := by
  simp only [vir_transport]
  exact bad

-- Positive 1–4: `toRuntime`, `ofRuntime`, `reframe`, and `Skolems.instantiate`
-- preserve `NTy.Holds` bijectively via `vir_transport`.
example [Θ : Skolems unit] (τ : NTy) (v : @NTy.carrier (Θ.toCarriers).toSorts τ) :
    @NTy.Holds (Carriers.runtime unit) (Θ.resolve τ) (Θ.toRuntime τ v) ↔
      @NTy.Holds Θ.toCarriers τ v := by
  simp only [vir_transport]

example [Θ : Skolems unit] (τ : NTy) (v : @NTy.carrier ((Carriers.runtime unit)).toSorts (Θ.resolve τ)) :
    @NTy.Holds Θ.toCarriers τ (Θ.ofRuntime τ v) ↔
      @NTy.Holds (Carriers.runtime unit) (Θ.resolve τ) v := by
  simp only [vir_transport]

example (Θ : Skolems unit) {params : NRow}
    (own : params.isParamsFrom 0 = true) {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (v : @NTy.carrier (Θ.toCarriers).toSorts τ) :
    @NTy.Holds (Θ.instance params own).toCarriers τ (Θ.reframe own bounded v) ↔
      @NTy.Holds Θ.toCarriers τ v := by
  simp only [vir_transport]

example [Θ : Skolems unit] (θ : TypeArgs) (τ : NTy)
    (v : @NTy.carrier (Θ.toCarriers).toSorts (τ.subst θ.1)) :
    @NTy.Holds (Skolems.instantiate θ Θ).toCarriers τ (NTy.toSkolem θ τ v) ↔
      @NTy.Holds Θ.toCarriers (τ.subst θ.1) v := by
  simp only [vir_transport]

end FrameTests

-- ============================================================================
-- Section 3: Collection (`SpecVector`, `SpecArray`, `SpecSliceRef`) `Holds`
-- ============================================================================

section CollectionTests
variable [Carriers]

-- Negative 1 (first): a singleton vector holding an invalid element fails
-- `NTy.Holds (.vector τ)`.
example {τ : NTy} (x : τ.carrier) (bad : ¬τ.Holds x) :
    ¬NTy.Holds (.vector τ) (⟨#[x], by simp⟩ : (NTy.vector τ).carrier) := by
  intro held
  exact bad (held.of_getElem 0 (by simp))

-- Negative 2: pushing an invalid element onto any vector refutes
-- `NTy.Holds (.vector τ)` on the resulting vector.
example {τ : NTy} {vec rPush : (NTy.vector τ).carrier} {x : τ.carrier}
    (bad : ¬τ.Holds x)
    (bPush : SpecVector.ofArray? (vec.values.push x) = some rPush) :
    ¬NTy.Holds (.vector τ) rPush := by
  intro held
  unfold SpecVector.ofArray? at bPush
  split at bPush
  · cases Option.some.inj bPush
    exact bad (held.of_mem (Array.mem_push.mpr (.inr rfl)))
  · cases bPush

-- Negative 3 (non-equivalence boundary): out-of-bounds `SpecVector.set` on an
-- empty vector or slice preserves `NTy.Holds` even when `¬τ.Holds badVal`,
-- witnessing that `vector_set` and `slice_set` are one-way implications rather
-- than equivalences.
example {τ : NTy} (badVal : τ.carrier) (_bad : ¬τ.Holds badVal) :
    let emptyVec : SpecVector τ.carrier := ⟨#[], by simp⟩
    NTy.Holds (.vector τ) (emptyVec.set 0 badVal) ∧
      NTy.Holds (.slice τ) (emptyVec.set 0 badVal) := by
  refine ⟨(NTy.holds_vector τ _).mpr ?_, (NTy.holds_slice τ _).mpr ?_⟩
  · intro x hx
    simp [SpecVector.set, Array.set!_eq_setIfInBounds] at hx
  · intro x hx
    simp [SpecVector.set, Array.set!_eq_setIfInBounds] at hx

-- Negative 4 (non-equivalence boundary): overwriting the sole invalid slot of a
-- singleton vector with a valid element satisfies `NTy.Holds` on the result
-- even though `¬NTy.Holds (.vector τ)` on the input vector.
example {τ : NTy} (badVal goodVal : τ.carrier)
    (bad : ¬τ.Holds badVal) (good : τ.Holds goodVal) :
    let badVec : SpecVector τ.carrier := ⟨#[badVal], by simp⟩
    ¬NTy.Holds (.vector τ) badVec ∧
      NTy.Holds (.vector τ) (badVec.set 0 goodVal) ∧
      NTy.Holds (.slice τ) (badVec.set 0 goodVal) ∧
      NTy.Holds (.array τ 1) ⟨badVec.set 0 goodVal, by simp [badVec, SpecVector.set]⟩ := by
  refine ⟨?_, ?_, ?_, ?_⟩
  · intro held
    exact bad (held.of_getElem 0 (by simp))
  · refine (NTy.holds_vector τ _).mpr ?_
    intro x hx
    simp [SpecVector.set, Array.set!_eq_setIfInBounds] at hx
    exact hx ▸ good
  · refine (NTy.holds_slice τ _).mpr ?_
    intro x hx
    simp [SpecVector.set, Array.set!_eq_setIfInBounds] at hx
    exact hx ▸ good
  · refine (NTy.holds_array τ 1 _).mpr ?_
    intro x hx
    simp [SpecVector.set, Array.set!_eq_setIfInBounds] at hx
    exact hx ▸ good

-- Negative 5: writing an invalid element at an in-bounds index refutes
-- `NTy.Holds` on the resulting slice, array, and updated `SpecSliceRef`.
example {τ : NTy} {len : Nat} {arr : (NTy.array τ len).carrier}
    {ref : (NTy.sliceRef τ).carrier} {i : Nat} {badVal : τ.carrier}
    (bad : ¬τ.Holds badVal)
    (inArr : i < len)
    (inRef : i < ref.extent) :
    ¬NTy.Holds (.slice τ) (arr.vector.set i badVal) ∧
      ¬NTy.Holds (.array τ len) ⟨arr.vector.set i badVal, SpecArray.size_set arr i badVal⟩ ∧
      ¬NTy.Holds (.sliceRef τ)
        (ref.withCurrent ⟨ref.currentVector.set i badVal, by
          rw [SpecVector.values_set, Array.size_set!, SpecSliceRef.currentVector_size]⟩) := by
  have hiArr : i < arr.vector.values.size := by simpa [arr.size_eq] using inArr
  have memArr : badVal ∈ (arr.vector.set i badVal).values := by
    rw [SpecVector.values_set, Array.set!_eq_setIfInBounds]
    exact Array.mem_of_getElem? (i := i) (by simp [hiArr])
  have memRef : badVal ∈ (ref.currentVector.set i badVal).values := by
    rw [SpecVector.values_set, Array.set!_eq_setIfInBounds]
    exact Array.mem_of_getElem? (i := i) (by simp [inRef])
  refine ⟨fun h => bad (h.of_mem_slice memArr), fun h => bad (h.of_mem_array memArr), ?_⟩
  intro hRef
  exact bad (hRef.of_current memRef)

-- Positive 1–10: single-carrier collection read, write & construction `Holds` preservation.
example {τ : NTy} {vec : (NTy.vector τ).carrier}
    (held : NTy.Holds (.vector τ) vec) (i : Nat) (h : i < vec.values.size) :
    τ.Holds (vec.values[i]'h) :=
  held.of_getElem i h

example {τ : NTy} {len : Nat} {arr : (NTy.array τ len).carrier}
    (held : NTy.Holds (.array τ len) arr) :
    NTy.Holds (.vector τ) arr.vector :=
  held.arrayVector

example {τ : NTy} {vec : SpecVector τ.carrier}
    (held : NTy.Holds (.vector τ) vec) (len : Nat) (sized : vec.values.size = len) :
    NTy.Holds (.array τ len) ⟨vec, sized⟩ :=
  held.of_arrayOf sized

example {τ : NTy} {ref : (NTy.sliceRef τ).carrier}
    (held : NTy.Holds (.sliceRef τ) ref) :
    NTy.Holds (.vector τ) ref.currentVector :=
  NTy.Holds.slice_iff_vector.mp held.sliceCurrent

example {τ : NTy} (x y : τ.carrier) (hx : τ.Holds x) (hy : τ.Holds y)
    {v : (NTy.vector τ).carrier}
    (built : SpecVector.ofArray? (HList.toListOf 2 (x, (y, ()))).toArray = some v) :
    NTy.Holds (.vector τ) v :=
  have held : HList.Holds (NRow.replicate 2 τ) (x, (y, ())) := ⟨hx, hy, trivial⟩
  NTy.Holds.of_vectorLit held built

example {τ : NTy} {vec res : (NTy.vector τ).carrier} {x : τ.carrier}
    (held : NTy.Holds (.vector τ) vec) (hx : τ.Holds x)
    (i j : Nat)
    (rep : SpecVector.ofArray? (Array.replicate 3 x) = some res) :
    NTy.Holds (.vector τ) res ∧
      NTy.Holds (.vector τ) (vec.set i x) ∧
      NTy.Holds (.vector τ) (vec.extract i j) ∧
      NTy.Holds (.vector τ) (swapBounded vec i j) ∧
      NTy.Holds (.vector τ) (eraseBounded vec i) :=
  ⟨NTy.Holds.of_repeatVector hx rep,
   held.vector_set i hx,
   held.extract i j,
   held.swapBounded i j,
   held.eraseBounded i⟩

example {τ : NTy} {vec rPush rIns rRem rSwap rCat rSl rRev : (NTy.vector τ).carrier}
    {x y : τ.carrier} {i j : Nat}
    (held : NTy.Holds (.vector τ) vec) (hx : τ.Holds x) (hy : τ.Holds y)
    (bPush : SpecVector.ofArray? (vec.values.push x) = some rPush)
    (bIns : SpecVector.ofArray? (vec.values.insertIdxIfInBounds i x) = some rIns)
    (bRem : SpecVector.ofArray? (vec.values.eraseIdxIfInBounds i) = some rRem)
    (bSwap : SpecVector.ofArray? ((vec.values.set! i y).set! j x) = some rSwap)
    (bCat : SpecVector.ofArray? (vec.values ++ rPush.values) = some rCat)
    (bSl : SpecVector.ofArray? (vec.values.extract i j) = some rSl)
    (bRev : SpecVector.ofArray? (reverseSlice vec.values i j) = some rRev) :
    NTy.Holds (.vector τ) rPush ∧
      NTy.Holds (.vector τ) rIns ∧
      NTy.Holds (.vector τ) rRem ∧
      NTy.Holds (.vector τ) rSwap ∧
      NTy.Holds (.vector τ) rCat ∧
      NTy.Holds (.vector τ) rSl ∧
      NTy.Holds (.vector τ) rRev := by
  have hPush := held.of_push hx bPush
  exact ⟨hPush, held.of_insert hx bIns, held.of_remove bRem,
    held.of_swap hx hy bSwap, held.of_concat hPush bCat,
    held.slice bSl, held.reverseSlice bRev⟩

example {τ : NTy} {len : Nat} {arr : (NTy.array τ len).carrier}
    {slice : (NTy.slice τ).carrier} {ref : (NTy.sliceRef τ).carrier}
    (arrHeld : NTy.Holds (.array τ len) arr)
    (sliceHeld : NTy.Holds (.slice τ) slice)
    (sized : slice.values.size = len)
    (refHeld : NTy.Holds (.sliceRef τ) ref) :
    NTy.Holds (.slice τ) arr.vector ∧
      NTy.Holds (.array τ len) ⟨slice, sized⟩ ∧
      NTy.Holds (.slice τ) ref.finalVector ∧
      NTy.Holds (.array τ ref.extent) ref.current ∧
      NTy.Holds (.array τ ref.extent) ref.final :=
  ⟨arrHeld.arraySlice,
   sliceHeld.of_arrayOf_slice sized,
   refHeld.sliceFinal,
   refHeld.sliceCurrentArray,
   refHeld.sliceFinalArray⟩

example {τ : NTy} {len : Nat} {arr : (NTy.array τ len).carrier}
    {slice : (NTy.slice τ).carrier} {ref : (NTy.sliceRef τ).carrier}
    {x : τ.carrier} (i : Nat)
    (arrHeld : NTy.Holds (.array τ len) arr)
    (sliceHeld : NTy.Holds (.slice τ) slice)
    (refHeld : NTy.Holds (.sliceRef τ) ref)
    (hx : τ.Holds x) :
    NTy.Holds (.array τ len) ⟨arr.vector.set i x, SpecArray.size_set arr i x⟩ ∧
      NTy.Holds (.slice τ) (slice.set i x) ∧
      NTy.Holds (.sliceRef τ)
        (ref.withCurrent ⟨ref.currentVector.set i x, by
          rw [SpecVector.values_set, Array.size_set!, SpecSliceRef.currentVector_size]⟩) ∧
      NTy.Holds (.sliceRef τ)
        (ref.withCurrent ⟨ref.current.vector.set i x, SpecArray.size_set ref.current i x⟩) :=
  ⟨arrHeld.array_set i hx (SpecArray.size_set arr i x),
   sliceHeld.slice_set i hx,
   refHeld.sliceRef_withCurrent (refHeld.sliceCurrent.slice_set i hx),
   refHeld.sliceRef_withCurrent_of_array
     (refHeld.sliceCurrentArray.array_set i hx (SpecArray.size_set ref.current i x))⟩

example {τ : NTy} {extent : Nat}
    {cur fin : (NTy.array τ extent).carrier}
    (curHeld : NTy.Holds (.array τ extent) cur)
    (finHeld : NTy.Holds (.array τ extent) fin) :
    NTy.Holds (.sliceRef τ) (SpecSliceRef.ofArrays cur fin) ∧
      NTy.Holds (.sliceRef τ) (SpecSliceRef.ofArrays cur fin) :=
  ⟨NTy.Holds.sliceRef_ofArrays curHeld finHeld,
   NTy.Holds.sliceRef_ofArrays_of_slice curHeld.arraySlice finHeld.arraySlice⟩

end CollectionTests

-- ============================================================================
-- Section 4: `SpecMap` & Single-Carrier `HEnv.Holds`
-- ============================================================================

section MapAndEnvTests
variable [Θ : Carriers]

-- Negative 1 (first): an initialized slot holding an invalid value fails `HEnv.Holds`.
example {τ σ : NTy} (badVal : τ.carrier) (bad : ¬τ.Holds badVal) :
    ¬HEnv.Holds (.cons τ (.cons σ .nil)) (some badVal, (none, ())) :=
  fun held => bad held.1

-- Negative 2: a `SpecMap` containing an entry with an invalid value fails `SpecMap.EntriesHold`.
example {κ ν : NTy} [DecidableEq κ.carrier] (k : κ.carrier) (badVal : ν.carrier)
    (bad : ¬ν.Holds badVal) :
    ¬SpecMap.EntriesHold κ ν id id [(k, badVal)] := by
  intro hMap
  exact bad (NTy.Holds.of_specMap_get? (key := k) hMap (by simp [SpecMap.get?]))

-- Positive 1: identity carrier view (concrete non-integer types).
example {κ ν : NTy} [DecidableEq κ.carrier] [Inhabited κ.carrier] [Inhabited ν.carrier]
    {table : (NTy.table κ ν).carrier} (held : NTy.Holds (.table κ ν) table)
    (k k2 : κ.carrier) (v2 : ν.carrier) (hk2 : κ.Holds k2) (hv2 : ν.Holds v2)
    (valFound : ν.carrier)
    (found : SpecMap.get? (SpecMap.ofEntries id id table) k = some valFound)
    (present : SpecMap.hasKey (SpecMap.ofEntries id id table) k = true)
    (idx : Int) (low : 0 ≤ idx) (high : idx < SpecMap.size (SpecMap.ofEntries id id table)) :
    ν.Holds valFound ∧
      ν.Holds (SpecMap.valueAt (SpecMap.ofEntries id id table) k) ∧
      κ.Holds (SpecMap.keyAt (SpecMap.ofEntries id id table) idx) ∧
      NTy.Holds (.table κ ν)
        (SpecMap.writeBack id
          (SpecMap.remove .sequence
            (SpecMap.update .sequence (SpecMap.ofEntries id id table) k2 v2) k)) := by
  have hMap : SpecMap.EntriesHold κ ν id id (SpecMap.ofEntries id id table) :=
    SpecMap.entries_hold_ofEntries held
  have hUpd := SpecMap.entries_hold_update hMap hk2 hv2 (discipline := .sequence)
  have hRem := SpecMap.entries_hold_remove hUpd (discipline := .sequence) (key := k)
  exact ⟨NTy.Holds.of_specMap_get? hMap found,
    NTy.Holds.of_specMap_valueAt hMap present,
    NTy.Holds.of_specMap_keyAt hMap low high,
    NTy.Holds.of_writeBack hRem⟩

-- Positive 2: integer key + `NTy.read` / `NTy.unread` value view.
example {ν : NTy} (hν : ν.inhabitable = true)
    {table : (NTy.table (.int 64 false) ν).carrier}
    (held : NTy.Holds (.table (.int 64 false) ν) table)
    (fallback : SpecInt (.bits 64) false) (k : Int)
    (present : SpecMap.hasKey (SpecMap.ofEntries SpecInt.val (NTy.read ν) table) k = true) :
    let unreadVal := NTy.unread ν hν
    let _ : Inhabited ν.readCarrier := NTy.readInhabited ν hν
    let _ : DecidableEq ν.readCarrier := NTy.readDecEq ν
    ν.Holds (unreadVal
      (@SpecMap.valueAt Int ν.readCarrier _ (NTy.readInhabited ν hν)
        (SpecMap.ofEntries SpecInt.val (NTy.read ν) table) k)) ∧
      NTy.Holds (.table (.int 64 false) ν)
        (SpecMap.writeBack (fun entry => (SpecInt.ofInt fallback entry.1, unreadVal entry.2))
          (SpecMap.ofEntries SpecInt.val (NTy.read ν) table)) := by
  intro unreadVal _ _
  have hMap : SpecMap.EntriesHold (.int 64 false) ν (SpecInt.ofInt fallback) unreadVal
      (SpecMap.ofEntries SpecInt.val (NTy.read ν) table) :=
    SpecMap.entries_hold_ofEntries held
      (valueHolds := fun v h => by dsimp only [unreadVal]; rwa [NTy.unread_read ν hν v])
  exact ⟨@NTy.Holds.of_specMap_valueAt _ Int ν.readCarrier _ (NTy.readInhabited ν hν)
      (.int 64 false) ν _ _ _ hMap k present,
    NTy.Holds.of_writeBack hMap⟩

-- Positive 3: single-carrier environment `HEnv.Holds`.
example {τ σ : NTy} (v : τ.carrier) (hv : τ.Holds v) :
    HEnv.Holds (.cons τ (.cons σ .nil)) (some v, (none, ())) :=
  ⟨hv, trivial, trivial⟩

end MapAndEnvTests

end VerexIR.Tests.CollectionAndTransportHolds
