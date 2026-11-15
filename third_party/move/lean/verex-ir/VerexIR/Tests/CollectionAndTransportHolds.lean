-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

namespace VerexIR.Tests.CollectionAndTransportHolds

open VerexIR VerexIR.Proofs.Denote

/-! # Unit tests for frame/skolem transport `Holds` laws and collection/`SpecMap` `Holds` lemmas -/

section TransportTests
variable [Θ : Carriers] [@CarriersNonempty Θ]

example (θ : TypeArgs) (τ : NTy) (v : (τ.subst θ.1).plain) :
    (@NTy.Holds (Carriers.instantiate θ Θ) τ (NTy.plainToSkolem θ τ v)) ↔
      @NTy.Holds Θ (τ.subst θ.1) v := by
  simp only [vir_transport]

example (θ : TypeArgs) (τ : NTy) (v : @NTy.plain (Carriers.instantiate θ Θ) τ) :
    (@NTy.Holds Θ (τ.subst θ.1) (NTy.plainOfSkolem θ τ v)) ↔
      @NTy.Holds (Carriers.instantiate θ Θ) τ v := by
  simp only [vir_transport]

example (θ : TypeArgs) (row : NRow) (vs : HListPlain (NRow.subst θ.1 row)) :
    (@HListPlain.Holds (Carriers.instantiate θ Θ) row (HListPlain.toSkolem θ row vs)) ↔
      @HListPlain.Holds Θ (NRow.subst θ.1 row) vs := by
  simp only [vir_transport]

example (θ : TypeArgs) (row : NRow) (vs : @HListPlain (Carriers.instantiate θ Θ) row) :
    (@HListPlain.Holds Θ (NRow.subst θ.1 row) (HListPlain.ofSkolem θ row vs)) ↔
      @HListPlain.Holds (Carriers.instantiate θ Θ) row vs := by
  simp only [vir_transport]

example (θ : TypeArgs) (names : List String) (rows : NRows)
    (v : variantPlain names (NRows.subst θ.1 rows)) :
    (@variantPlain.Holds (Carriers.instantiate θ Θ) names rows
        (variantPlain.toSkolem θ names rows v)) ↔
      @variantPlain.Holds Θ names (NRows.subst θ.1 rows) v := by
  simp only [vir_transport]

example (θ : TypeArgs) (names : List String) (rows : NRows)
    (v : @variantPlain (Carriers.instantiate θ Θ) names rows) :
    (@variantPlain.Holds Θ names (NRows.subst θ.1 rows)
        (variantPlain.ofSkolem θ names rows v)) ↔
      @variantPlain.Holds (Carriers.instantiate θ Θ) names rows v := by
  simp only [vir_transport]

end TransportTests

section FrameTests
variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]

example [Θ : Skolems unit] (τ : NTy) (v : @NTy.plain Θ.toCarriers τ) :
    @NTy.Holds (Carriers.runtime unit) (Θ.resolve τ) (Θ.plainToRuntime τ v) ↔
      @NTy.Holds Θ.toCarriers τ v := by
  simp only [vir_transport]

example [Θ : Skolems unit] (τ : NTy) (v : @NTy.plain (Carriers.runtime unit) (Θ.resolve τ)) :
    @NTy.Holds Θ.toCarriers τ (Θ.plainOfRuntime τ v) ↔
      @NTy.Holds (Carriers.runtime unit) (Θ.resolve τ) v := by
  simp only [vir_transport]

example [@CarriersNonempty (Carriers.runtime unit)] (Θ : Skolems unit) {params : NRow}
    (own : params.isParamsFrom 0 = true) {τ : NTy}
    (bounded : τ.paramsAll (fun index => decide (index < params.length)) = true)
    (v : @NTy.plain Θ.toCarriers τ) :
    @NTy.Holds (Θ.instance params own).toCarriers τ (Θ.plainReframe own bounded v) ↔
      @NTy.Holds Θ.toCarriers τ v := by
  simp only [vir_transport]

example [Θ : Skolems unit] (θ : TypeArgs) (τ : NTy)
    (v : @NTy.plain Θ.toCarriers (τ.subst θ.1)) :
    @NTy.Holds (Skolems.instantiate θ Θ).toCarriers τ (NTy.plainToSkolem θ τ v) ↔
      @NTy.Holds Θ.toCarriers (τ.subst θ.1) v := by
  simp only [vir_transport]

end FrameTests

section CollectionTests
variable [Carriers]

example {τ : NTy} {vec : (NTy.vector τ).plain}
    (held : NTy.Holds (.vector τ) vec) (i : Nat) (h : i < vec.values.size) :
    τ.Holds (vec.values[i]'h) :=
  held.of_getElem i h

example {τ : NTy} {len : Nat} {arr : (NTy.array τ len).plain}
    (held : NTy.Holds (.array τ len) arr) :
    NTy.Holds (.vector τ) arr.vector :=
  held.arrayVector

example {τ : NTy} {vec : SpecVector τ.plain}
    (held : NTy.Holds (.vector τ) vec) (len : Nat) (sized : vec.values.size = len) :
    NTy.Holds (.array τ len) ⟨vec, sized⟩ :=
  held.of_arrayOf sized

example {τ : NTy} {ref : (NTy.sliceRef τ).plain}
    (held : NTy.Holds (.sliceRef τ) ref) :
    NTy.Holds (.vector τ) ref.currentVector :=
  NTy.Holds.slice_iff_vector.mp held.sliceCurrent

example {τ : NTy} (x y : τ.plain) (hx : τ.Holds x) (hy : τ.Holds y)
    {v : (NTy.vector τ).plain}
    (built : SpecVector.ofArray? (HListPlain.toListOf 2 (x, (y, ()))).toArray = some v) :
    NTy.Holds (.vector τ) v :=
  have held : HListPlain.Holds (NRow.replicate 2 τ) (x, (y, ())) := ⟨hx, hy, trivial⟩
  NTy.Holds.of_vectorLit held built

example {τ : NTy} {vec res : (NTy.vector τ).plain} {x : τ.plain}
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

example {τ : NTy} {vec rPush rIns rRem rSwap rCat rSl rRev : (NTy.vector τ).plain}
    {x y : τ.plain} {i j : Nat}
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

end CollectionTests

section MapAndEnvTests
variable [Θ : Carriers] [@CarriersNonempty Θ]

-- Identity plain view (concrete non-integer types)
example {κ ν : NTy} [DecidableEq κ.plain] [Inhabited κ.plain] [Inhabited ν.plain]
    {table : (NTy.table κ ν).plain} (held : NTy.Holds (.table κ ν) table)
    (k k2 : κ.plain) (v2 : ν.plain) (hk2 : κ.Holds k2) (hv2 : ν.Holds v2)
    (valFound : ν.plain)
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

-- Integer key + `readPlain` value view
example {ν : NTy} (hν : ν.inhabitable = true)
    {table : (NTy.table (.int 64 false) ν).plain}
    (held : NTy.Holds (.table (.int 64 false) ν) table)
    (fallback : SpecInt (.bits 64) false) (k : Int)
    (present : SpecMap.hasKey (SpecMap.ofEntries SpecInt.val (NTy.readPlain ν hν) table) k = true) :
    let unreadVal := fun x => NTy.forget ν (NTy.unread ν hν x)
    let _ : Inhabited ν.readCarrier := NTy.readInhabited ν hν
    let _ : DecidableEq ν.readCarrier := NTy.readDecEq ν
    ν.Holds (unreadVal
      (@SpecMap.valueAt Int ν.readCarrier _ (NTy.readInhabited ν hν)
        (SpecMap.ofEntries SpecInt.val (NTy.readPlain ν hν) table) k)) ∧
      NTy.Holds (.table (.int 64 false) ν)
        (SpecMap.writeBack (fun entry => (SpecInt.ofInt fallback entry.1, unreadVal entry.2))
          (SpecMap.ofEntries SpecInt.val (NTy.readPlain ν hν) table)) := by
  intro unreadVal _ _
  have hMap : SpecMap.EntriesHold (.int 64 false) ν (SpecInt.ofInt fallback) unreadVal
      (SpecMap.ofEntries SpecInt.val (NTy.readPlain ν hν) table) :=
    SpecMap.entries_hold_ofEntries held (valueHolds := fun _ h => h.unread_readPlain hν)
  exact ⟨@NTy.Holds.of_specMap_valueAt _ Int ν.readCarrier _ (NTy.readInhabited ν hν)
      (.int 64 false) ν _ _ _ hMap k present,
    NTy.Holds.of_writeBack hMap⟩

-- Plain environment `HEnvPlain.Holds`
example {τ σ : NTy} (v : τ.plain) (hv : τ.Holds v) (env : HEnv (.cons τ (.cons σ .nil))) :
    HEnvPlain.Holds (.cons τ (.cons σ .nil)) (some v, (none, ())) ∧
      HEnvPlain.Holds (.cons τ (.cons σ .nil)) (HEnv.forget (.cons τ (.cons σ .nil)) env) :=
  ⟨⟨hv, trivial, trivial⟩, HEnv.holds_forget (.cons τ (.cons σ .nil)) env⟩

end MapAndEnvTests

end VerexIR.Tests.CollectionAndTransportHolds
