-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Accounting

/-!
# The invariant at a transaction's initial state

UNTRUSTED (checked by the kernel). The shadow of a transaction's initial
state (`Shadow.initial`) gives every node of every argument and of every held
value a fresh identity created by the transaction, and every node of global
storage one created by `Creator.storage`; the reference arguments are borrowed
from the held values. Its occurrences are exactly the creations, so the trace
invariant (`TraceInv`) holds. D3 holds of the creations by E1 (every node of an
accepted argument has an accepted type, all the way down), E2 (every signer is
for an authenticated address), and well-typed storage (a resource has `key`,
so every node below it has `store`, which no signer has).
-/

namespace MoveBytecode.Proofs

/-! ## E1 all the way down -/

theorem allConform_mem {P : Program} {t : RuntimeType} :
    ∀ {vs : List Value}, Value.allConform P vs t = true → ∀ c ∈ vs,
      c.type.assignable t = true ∧ c.wellFormed P = true
  | [], _, _, hc => by simp at hc
  | v :: vs, h, c, hc => by
    simp only [Value.allConform, Bool.and_eq_true] at h
    rcases List.mem_cons.1 hc with rfl | hc
    · exact h.1
    · exact allConform_mem h.2 c hc

theorem conformsAll_type {P : Program} :
    ∀ {vs : List Value} {ts : List RuntimeType}, conformsAll P vs ts = true → ∀ c ∈ vs,
      ∃ t ∈ ts, c.type.assignable t = true ∧ c.wellFormed P = true
  | [], _, _, _, hc => by simp at hc
  | _ :: _, [], h, _, _ => by simp [conformsAll] at h
  | v :: vs, t :: ts, h, c, hc => by
    simp only [conformsAll, Bool.and_eq_true] at h
    rcases List.mem_cons.1 hc with rfl | hc
    · exact ⟨t, List.mem_cons_self .., h.1⟩
    · obtain ⟨t', ht', h'⟩ := conformsAll_type h.2 c hc
      exact ⟨t', List.mem_cons_of_mem _ ht', h'⟩

section Arguments

variable {P : Program} {A : List StructId}

theorem fieldTypes_argument (hA : P.TransactionArgumentStructs A) {s : StructId}
    (hs : A.contains s = true) {tys : List RuntimeType} (hf : P.fieldTypes s = some tys) :
    ∀ t ∈ tys, t.IsTransactionArgument A = true := by
  obtain ⟨-, -, info, hinfo, hmatch⟩ := hA s (by simpa using hs)
  have hf' := hf
  simp only [Program.fieldTypes, hinfo, Option.bind_eq_bind, Option.bind_some] at hf'
  revert hmatch hf'
  cases info.definition.fieldInformation with
  | declared fields =>
    rintro ⟨tys', hf'', hall⟩ -
    rw [hf] at hf''
    simp only [Option.some.injEq] at hf''
    rw [hf'']
    exact hall
  | declaredVariants _ => intro _ h; simp at h
  | native => intro h; exact h.elim

theorem variantFieldTypes_argument (hA : P.TransactionArgumentStructs A) {s : StructId}
    (hs : A.contains s = true) {tag : VariantIndex} {tys : List RuntimeType}
    (hf : P.variantFieldTypes s tag = some tys) :
    ∀ t ∈ tys, t.IsTransactionArgument A = true := by
  obtain ⟨-, -, info, hinfo, hmatch⟩ := hA s (by simpa using hs)
  have hf' := hf
  simp only [Program.variantFieldTypes, hinfo, Option.bind_eq_bind, Option.bind_some] at hf'
  revert hmatch hf'
  cases info.definition.fieldInformation with
  | declaredVariants variants =>
    intro hall hf'
    have htag : tag < variants.length := by
      cases hv : variants[tag]? with
      | none => simp [hv] at hf'
      | some _ => exact (List.getElem?_eq_some_iff.1 hv).1
    obtain ⟨tys', hf'', hall'⟩ := hall tag htag
    rw [hf] at hf''
    simp only [Option.some.injEq] at hf''
    rw [hf'']
    exact hall'
  | declared _ => intro _ h; simp at h
  | native => intro h; exact h.elim

/-- A transaction argument type has no subtypes. -/
theorem argument_rigid {t : RuntimeType} (h : t.IsTransactionArgument A = true) : rigid t = true := by
  cases t with
  | function => simp [RuntimeType.IsTransactionArgument] at h
  | ref => simp [RuntimeType.IsTransactionArgument] at h
  | _ => rfl

/-- **E1 all the way down**: every node of a well-formed value of an accepted
argument type has an accepted argument type. -/
theorem deep_argument (hA : P.TransactionArgumentStructs A) (v : Value)
    (hwf : v.wellFormed P = true) (hv : v.type.IsTransactionArgument A = true) :
    ∀ w ∈ subvalues v, w.type.IsTransactionArgument A = true := by
  induction v using Value.rec
    (motive_2 := fun vs => ∀ c ∈ vs, c.wellFormed P = true →
      c.type.IsTransactionArgument A = true →
      ∀ w ∈ subvalues c, w.type.IsTransactionArgument A = true) with
  | vector e vs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hv
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [Value.wellFormed] at hwf
      obtain ⟨hct, hcw⟩ := allConform_mem hwf c hc
      refine ih c hc hcw ?_ w hwc
      have he : e.IsTransactionArgument A = true := by
        simpa [Value.type, RuntimeType.IsTransactionArgument] using hv
      rw [assignable_rigid hct (argument_rigid he)]
      exact he
  | struct s fs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hv
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [Value.type, RuntimeType.IsTransactionArgument] at hv
      obtain ⟨-, tys, htys, hwf⟩ := wellFormed_struct.1 hwf
      obtain ⟨t, ht, hct, hcw⟩ := conformsAll_type hwf c hc
      have hta := fieldTypes_argument hA hv htys t ht
      exact ih c hc hcw (assignable_rigid hct (argument_rigid hta) ▸ hta) w hwc
  | table s fs ks es ihf _ ihe =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hv
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [Value.type, RuntimeType.IsTransactionArgument] at hv
      obtain ⟨-, -, ⟨tys, htys, hwf'⟩, hes⟩ := wellFormed_table.1 hwf
      simp only [Value.children, List.mem_append] at hc
      rcases hc with hc | hc
      · obtain ⟨t, ht, hct, hcw⟩ := conformsAll_type hwf' c hc
        have hta := fieldTypes_argument hA hv htys t ht
        exact ihf c hc hcw (assignable_rigid hct (argument_rigid hta) ▸ hta) w hwc
      · -- An accepted type has `copy`, and a table whose type has `copy` has no entries.
        rcases hes with rfl | ⟨-, -, -, -, -, -, -, hcopy, -⟩
        · simp at hc
        · obtain ⟨hc', -⟩ := hA s (by simpa using hv)
          rw [hc'] at hcopy
          cases hcopy
  | variant s tag fs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hv
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [Value.type, RuntimeType.IsTransactionArgument] at hv
      simp only [Value.wellFormed] at hwf
      split at hwf
      · next tys htys =>
        obtain ⟨t, ht, hct, hcw⟩ := conformsAll_type hwf c hc
        have hta := variantFieldTypes_argument hA hv htys t ht
        exact ih c hc hcw (assignable_rigid hct (argument_rigid hta) ▸ hta) w hwc
      · simp at hwf
  | nil => rename_i hc _ _ _ _; simp at hc
  | cons x xs ihx ihxs =>
    rename_i c hc hcw hct w hw
    rcases List.mem_cons.1 hc with rfl | hc
    · exact ihx hcw hct w hw
    · exact ihxs c hc hcw hct w hw
  | closure =>
    simp [Value.type, RuntimeType.IsTransactionArgument] at hv
  | signer a =>
    intro w hw
    simp only [subvalues, Value.nodes, List.map_cons, List.map_nil, List.mem_cons,
      List.not_mem_nil, or_false] at hw
    rcases hw with rfl | rfl
    · exact hv
    · rfl
  | _ =>
    intro w hw
    rw [subvalues_eq] at hw
    simp only [Value.children, List.flatMap_nil, List.mem_singleton] at hw
    subst hw
    exact hv

end Arguments

/-! ## Global storage holds no signer -/

theorem store_of_subset' {a b : AbilitySet} (h : a.subset b = true) (ha : a.store = true) :
    b.store = true := by
  cases a; cases b; simp_all [MoveBytecode.AbilitySet.subset]

theorem key_of_subset' {a b : AbilitySet} (h : a.subset b = true) (ha : a.key = true) :
    b.key = true := by
  cases a; cases b; simp_all [MoveBytecode.AbilitySet.subset]

/-- A value with `store` has `store` all the way down. -/
theorem deep_store {P : Program} (v : Value) (h : (v.abilities P).store = true) :
    ∀ w ∈ subvalues v, (w.type.abilities P).store = true := by
  induction v using Value.rec
    (motive_2 := fun vs => ∀ c ∈ vs, (c.abilities P).store = true →
      ∀ w ∈ subvalues c, (w.type.abilities P).store = true) with
  | vector e vs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact store_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := store_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      exact ih c hc hc' w hwc
  | struct s fs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact store_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := store_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      exact ih c hc hc' w hwc
  | variant s tag fs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact store_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := store_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      exact ih c hc hc' w hwc
  | nil => rename_i hc _ _ _; simp at hc
  | cons x xs ihx ihxs =>
    rename_i c hc hab w hw
    rcases List.mem_cons.1 hc with rfl | hc
    · exact ihx hab w hw
    · exact ihxs c hc hab w hw
  | closure f mask cs ps rs a ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact store_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := store_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      exact ih c hc hc' w hwc
  | table s fs ks es ihf _ ihe =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact store_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := store_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      simp only [Value.children, List.mem_append] at hc
      rcases hc with hc | hc
      · exact ihf c hc hc' w hwc
      · exact ihe c hc hc' w hwc
  | signer a =>
    intro w hw
    simp [Value.abilities, RuntimeType.abilities] at h
  | _ =>
    intro w hw
    rw [subvalues_eq] at hw
    simp only [Value.children, List.flatMap_nil, List.mem_singleton] at hw
    subst hw
    exact store_of_subset' (abilities_type _) h

/-- A resource holds no signer: every node below its root has `store`, which a
signer lacks, and its root is a struct. -/
theorem resource_no_signer {P : Program} {v : Value} {t : StructId} (hv : v.type = .struct t)
    (hk : (v.abilities P).key = true) : ∀ w ∈ subvalues v, ∀ a, w ≠ .signer a := by
  intro w hw a hwa
  subst hwa
  rw [subvalues_eq] at hw
  rcases List.mem_cons.1 hw with h | hw
  · rw [← h] at hv; simp [Value.type] at hv
  · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
    have hc' := key_of_subset' (abilities_child v c hc) hk
    simp only [MoveBytecode.AbilitySet.forContainer] at hc'
    have := deep_store c hc' _ hwc
    simp [Value.type, RuntimeType.abilities] at this

/-! ## The shadow of the initial state -/

/-- One step of annotating an initial frame's locals, as `Shadow.initial`
does. -/
def initLocalsStep : List (Option IdTree) × Nat × List Event → Option Value →
    List (Option IdTree) × Nat × List Event :=
  fun (ls, n, evs) slot =>
    match slot with
    | some v => let (t, n, e) := IdTree.fresh .transaction v n; (ls ++ [some t], n, evs ++ e)
    | none => (ls ++ [none], n, evs)

/-- One step of `IdTree.freshAll`. -/
def freshStep (c : Creator) : List IdTree × Nat × List Event → Value →
    List IdTree × Nat × List Event :=
  fun (ts, n, evs) v => let (t, n, e) := IdTree.fresh c v n; (ts ++ [t], n, evs ++ e)

/-- The reference parameters of an initial frame, as `Shadow.initial` finds
them. -/
def initParams (g : ShadowFrame) (fr : Frame) : List (Nat × Reference) :=
  (g.locals.toList.zip fr.locals.toList).filterMap fun (t, v) =>
    match t, v with
    | some t, some (.ref r) => some (t.id, r)
    | _, _ => none

/-- The reference events of an initial frame. -/
def initRefs (g : ShadowFrame) (fr : Frame) : List Event :=
  (initParams g fr).map (fun (i, r) => Event.borrowed i r none) ++
    [.entered fr.id ((initParams g fr).map (·.1))]

/-- The parts of the initial shadow: the entry frame's locals, its stack, global
storage and the held values, each annotated after the one before. -/
def initL (fr : Frame) : List (Option IdTree) × Nat × List Event :=
  fr.locals.toList.foldl initLocalsStep ([], 0, [])

def initK (fr : Frame) : List IdTree × Nat × List Event :=
  IdTree.freshAll .transaction fr.stack (initL fr).2.1

def initS (s : State) (fr : Frame) : List IdTree × Nat × List Event :=
  (s.storage.map (·.value)).foldl (freshStep .storage) ([], (initK fr).2.1, [])

def initH (s : State) (fr : Frame) : List IdTree × Nat × List Event :=
  s.held.foldl (freshStep .transaction) ([], (initS s fr).2.1, [])

/-- The shadow of a state with one frame. -/
theorem initial_eq {s : State} {fr : Frame} (hs : s.frames = [fr]) :
    Shadow.initial s =
      (⟨[⟨(initL fr).1.toArray, (initK fr).1⟩], (initS s fr).1, (initH s fr).1, (initH s fr).2.1⟩,
        (initL fr).2.2 ++ (initK fr).2.2 ++ (initS s fr).2.2 ++ (initH s fr).2.2 ++
          (initRefs ⟨(initL fr).1.toArray, (initK fr).1⟩ fr ++ [])) := by
  unfold Shadow.initial
  rw [hs]
  rfl

@[simp] theorem freshAll_nil (c : Creator) (n : Nat) : IdTree.freshAll c [] n = ([], n, []) := rfl

/-- Annotating locals: each value gets a tree of fresh identities, created by
the transaction; the occurrences are exactly the creations. -/
theorem locals_fold : ∀ (L : List (Option Value)) (ls0 : List (Option IdTree)) (n0 : Nat)
    (evs0 : List Event),
    ∃ ls evs n, L.foldl initLocalsStep (ls0, n0, evs0) = (ls0 ++ ls, n, evs0 ++ evs) ∧
      ls.length = L.length ∧
      (ls.zip L).all (fun (p : Option IdTree × Option Value) =>
        ShadowFrame.slotFits p.1 p.2) = true ∧
      (ls.zip L).flatMap (fun (p : Option IdTree × Option Value) =>
        ShadowFrame.slotOccurrences p.1 p.2) = createdOf evs ∧
      (∀ o ∈ createdOf evs, n0 ≤ o.1 ∧ o.1 < n) ∧ ((createdOf evs).map (·.1)).Nodup ∧
      n0 ≤ n ∧
      (∀ e ∈ evs, ∃ i w, e = .created i w .transaction ∧
        ∃ v, some v ∈ L ∧ w ∈ subvalues v)
  | [], ls0, n0, evs0 =>
    ⟨[], [], n0, by simp, rfl, rfl, rfl, by simp, by simp, Nat.le_refl _, by simp⟩
  | none :: L, ls0, n0, evs0 => by
    obtain ⟨ls, evs, n, hf, hlen, hfit, hocc, hb, hnd, hle, hev⟩ :=
      locals_fold L (ls0 ++ [none]) n0 evs0
    refine ⟨none :: ls, evs, n, ?_, by simp [hlen], ?_, ?_, hb, hnd, hle, ?_⟩
    · show List.foldl initLocalsStep (ls0 ++ [none], n0, evs0) L = _
      rw [hf]
      simp
    · simpa [ShadowFrame.slotFits] using hfit
    · simpa [ShadowFrame.slotOccurrences] using hocc
    · intro e he
      obtain ⟨i, w, rfl, v, hv, hw⟩ := hev e he
      exact ⟨i, w, rfl, v, List.mem_cons_of_mem _ hv, hw⟩
  | some v :: L, ls0, n0, evs0 => by
    obtain ⟨hft, hocc0, hfresh, hevs0⟩ := fresh_spec .transaction v n0
    obtain ⟨ls, evs, n, hf, hlen, hfit, hocc, hb, hnd, hle, hev⟩ :=
      locals_fold L (ls0 ++ [some (IdTree.fresh .transaction v n0).1])
        (IdTree.fresh .transaction v n0).2.1 (evs0 ++ (IdTree.fresh .transaction v n0).2.2)
    have hz : ((List.range' n0 (subvalues v).length).zip (subvalues v)).map (·.1) =
        List.range' n0 (subvalues v).length := List.map_fst_zip (by simp)
    have hc0 : createdOf (IdTree.fresh .transaction v n0).2.2 =
        (List.range' n0 (subvalues v).length).zip (subvalues v) := by
      rw [hevs0, createdOf_map_created]
    have hb0 : ∀ o ∈ createdOf (IdTree.fresh .transaction v n0).2.2,
        n0 ≤ o.1 ∧ o.1 < n0 + (subvalues v).length := by
      intro o ho
      rw [hc0] at ho
      have := (List.of_mem_zip ho).1
      rw [List.mem_range'] at this
      obtain ⟨k, hk, hko⟩ := this
      omega
    refine ⟨some (IdTree.fresh .transaction v n0).1 :: ls,
      (IdTree.fresh .transaction v n0).2.2 ++ evs,
      n, ?_, by simp [hlen], ?_, ?_, ?_, ?_, ?_, ?_⟩
    · show List.foldl initLocalsStep (ls0 ++ [some (IdTree.fresh .transaction v n0).1],
        (IdTree.fresh .transaction v n0).2.1, evs0 ++ (IdTree.fresh .transaction v n0).2.2) L = _
      rw [hf]
      simp
    · simpa [ShadowFrame.slotFits, hft] using hfit
    · rw [List.zip_cons_cons, List.flatMap_cons, hocc, createdOf_append, hc0]
      simp only [ShadowFrame.slotOccurrences, hocc0]
    · intro o ho
      rw [createdOf_append, List.mem_append] at ho
      rcases ho with ho | ho
      · have := hb0 o ho
        rw [hfresh] at hle
        omega
      · have := hb o ho
        rw [hfresh] at this
        omega
    · rw [createdOf_append, List.map_append, List.nodup_append, hc0, hz]
      refine ⟨List.nodup_range' .., hnd, fun a ha b hb' hab => ?_⟩
      rw [List.mem_range'] at ha
      obtain ⟨x, hx, hxb⟩ := List.mem_map.1 hb'
      have := (hb x hx).1
      rw [hfresh] at this
      omega
    · rw [hfresh] at hle
      omega
    · intro e he
      rcases List.mem_append.1 he with he | he
      · rw [hevs0] at he
        obtain ⟨⟨i, w⟩, hiw, rfl⟩ := List.mem_map.1 he
        exact ⟨i, w, rfl, v, List.mem_cons_self .., (List.of_mem_zip hiw).2⟩
      · obtain ⟨i, w, rfl, v', hv', hw⟩ := hev e he
        exact ⟨i, w, rfl, v', List.mem_cons_of_mem _ hv', hw⟩


/-- Annotating values created whole by `c`: the occurrences are exactly the
creations. -/
theorem fresh_fold (c : Creator) : ∀ (vs : List Value) (ts0 : List IdTree) (n0 : Nat)
    (evs0 : List Event),
    ∃ ts evs n, vs.foldl (freshStep c) (ts0, n0, evs0) = (ts0 ++ ts, n, evs0 ++ evs) ∧
      IdTree.fitsAll ts vs = true ∧ IdTree.occurrencesAll ts vs = createdOf evs ∧
      (∀ o ∈ createdOf evs, n0 ≤ o.1 ∧ o.1 < n) ∧ ((createdOf evs).map (·.1)).Nodup ∧
      n0 ≤ n ∧ (∀ e ∈ evs, ∃ i w, e = .created i w c ∧ ∃ v ∈ vs, w ∈ subvalues v)
  | [], ts0, n0, evs0 =>
    ⟨[], [], n0, by simp, by simp [IdTree.fitsAll], by simp [IdTree.occurrencesAll], by simp,
      by simp, Nat.le_refl _, by simp⟩
  | v :: vs, ts0, n0, evs0 => by
    obtain ⟨hft, hocc0, hfresh, hevs0⟩ := fresh_spec c v n0
    obtain ⟨ts, evs, n, hf, hfit, hocc, hb, hnd, hle, hev⟩ :=
      fresh_fold c vs (ts0 ++ [(IdTree.fresh c v n0).1]) (IdTree.fresh c v n0).2.1
        (evs0 ++ (IdTree.fresh c v n0).2.2)
    have hz : ((List.range' n0 (subvalues v).length).zip (subvalues v)).map (·.1) =
        List.range' n0 (subvalues v).length := List.map_fst_zip (by simp)
    have hc0 : createdOf (IdTree.fresh c v n0).2.2 =
        (List.range' n0 (subvalues v).length).zip (subvalues v) := by
      rw [hevs0, createdOf_map_created]
    have hb0 : ∀ o ∈ createdOf (IdTree.fresh c v n0).2.2,
        n0 ≤ o.1 ∧ o.1 < n0 + (subvalues v).length := by
      intro o ho
      rw [hc0] at ho
      have := (List.of_mem_zip ho).1
      rw [List.mem_range'] at this
      obtain ⟨k, hk, hko⟩ := this
      omega
    refine ⟨(IdTree.fresh c v n0).1 :: ts, (IdTree.fresh c v n0).2.2 ++ evs, n, ?_, ?_, ?_, ?_, ?_,
      ?_, ?_⟩
    · show List.foldl (freshStep c) (ts0 ++ [(IdTree.fresh c v n0).1],
        (IdTree.fresh c v n0).2.1, evs0 ++ (IdTree.fresh c v n0).2.2) vs = _
      rw [hf]
      simp
    · simp [IdTree.fitsAll, hft, hfit]
    · simp only [IdTree.occurrencesAll]
      rw [hocc, createdOf_append, hc0, hocc0]
    · intro o ho
      rw [createdOf_append, List.mem_append] at ho
      rcases ho with ho | ho
      · have := hb0 o ho
        rw [hfresh] at hle
        omega
      · have := hb o ho
        rw [hfresh] at this
        omega
    · rw [createdOf_append, List.map_append, List.nodup_append, hc0, hz]
      refine ⟨List.nodup_range' .., hnd, fun a ha b hb' hab => ?_⟩
      rw [List.mem_range'] at ha
      obtain ⟨x, hx, hxb⟩ := List.mem_map.1 hb'
      have := (hb x hx).1
      rw [hfresh] at this
      omega
    · rw [hfresh] at hle
      omega
    · intro e he
      rcases List.mem_append.1 he with he | he
      · rw [hevs0] at he
        obtain ⟨⟨i, w⟩, hiw, rfl⟩ := List.mem_map.1 he
        exact ⟨i, w, rfl, v, List.mem_cons_self .., (List.of_mem_zip hiw).2⟩
      · obtain ⟨i, w, rfl, v', hv', hw⟩ := hev e he
        exact ⟨i, w, rfl, v', List.mem_cons_of_mem _ hv', hw⟩

theorem createdOf_borrowedAll : ∀ (ps : List (Nat × Reference)),
    createdOf (ps.map fun (i, r) => Event.borrowed i r none) = []
  | [] => rfl
  | (i, r) :: ps => by simp [createdOf_borrowedAll ps]

theorem createdOf_initRefs (g : ShadowFrame) (fr : Frame) : createdOf (initRefs g fr) = [] := by
  simp only [initRefs, createdOf_append, createdOf_borrowedAll]
  rfl

/-- The references an initial frame borrows are the reference values in its
locals, with their identities. -/
theorem initRefs_issued {g : ShadowFrame} {fr : Frame} (hfit : slotsFit g fr = true) :
    ∀ x ∈ issuedOf (initRefs g fr), x ∈ keys (localsOcc g fr) := by
  intro x hx
  rcases mem_issuedOf hx with ⟨w, c, hc, -⟩ | ⟨r, p, hb, hrk⟩
  · simp [initRefs] at hc
  · simp only [initRefs, List.mem_append, List.mem_map, List.mem_singleton] at hb
    rcases hb with ⟨⟨i, r'⟩, hq, he⟩ | he
    · simp only [Event.borrowed.injEq] at he
      obtain ⟨rfl, rfl, -⟩ := he
      simp only [initParams, List.mem_filterMap] at hq
      obtain ⟨⟨ot, ov⟩, hz, hm⟩ := hq
      cases ot with
      | none => simp at hm
      | some t =>
        cases ov with
        | none => simp at hm
        | some v =>
          cases v with
          | ref r'' =>
            simp only [Option.some.injEq, Prod.mk.injEq] at hm
            obtain ⟨hti, rfl⟩ := hm
            have hsl : ShadowFrame.slotFits (some t) (some (.ref r'')) = true := by
              simp only [slotsFit, List.all_eq_true] at hfit
              exact hfit _ hz
            obtain ⟨i0, cs⟩ := t
            have hcs : cs = [] := by
              cases cs with
              | nil => rfl
              | cons _ _ =>
                simp [ShadowFrame.slotFits, IdTree.fits, Value.children, IdTree.fitsAll] at hsl
            subst hcs
            refine mem_keys_iff.2 ⟨(i0, .ref r''), ?_, by simpa [IdTree.id] using hti, by rw [← hrk]⟩
            simp only [localsOcc, List.mem_flatMap]
            exact ⟨_, hz, by simp [ShadowFrame.slotOccurrences, IdTree.occurrences]⟩
          | _ => simp at hm
    · cases he

/-! ## The initial state -/

theorem initialState_shape {P : Program} {f : FunctionRef} {args : List Value}
    {storage : GlobalStorage} {held : List Value} {s₀ : State}
    (h : initialState P f args storage held = some s₀) :
    ∃ fr localTys, s₀.frames = [fr] ∧ fr.stack = [] ∧ fr.function = f ∧ fr.id = 0 ∧
      P.localTypes f = some localTys ∧
      fr.locals = (args.map some ++ List.replicate (localTys.length - args.length) none).toArray ∧
      s₀.storage = storage ∧ s₀.held = held ∧ s₀.nextFrame = 1 ∧
      heldArguments P held args = true := by
  unfold initialState at h
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def] at h
  obtain ⟨info, -, _, -, localTys, hlt, h⟩ := h
  split at h
  · simp at h
  split at h
  · simp at h
  · next hha =>
    simp only [Option.some.injEq] at h
    subst h
    exact ⟨_, localTys, rfl, rfl, rfl, rfl, hlt, rfl, rfl, rfl, rfl, by simpa using hha⟩

/-- An initial state runs an instance whose type arguments satisfy its
function's constraints. -/
theorem initialState_typeArgsOk {P : Program} {f : FunctionRef} {args : List Value}
    {storage : GlobalStorage} {held : List Value} {s₀ : State}
    (h : initialState P f args storage held = some s₀) : P.typeArgsOk f = true := by
  unfold initialState at h
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def] at h
  obtain ⟨info, -, _, -, localTys, -, h⟩ := h
  split at h
  · simp at h
  · next hok => simpa using hok

/-- The state invariant at an initial state. -/
theorem inv_initial {P : Program} {f : FunctionRef} {args : List Value} {storage : GlobalStorage}
    {held : List Value} {s₀ : State} (h : initialState P f args storage held = some s₀)
    (hst : storage.wellTyped P = true)
    (hheld : σ = true → ∀ v ∈ held, (v.abilities P).drop = true) :
    StateInv σ P s₀ := by
  obtain ⟨fr, localTys, hfr, -, -, hid, -, -, hsto, hhe, hnf, -⟩ := initialState_shape h
  have hwt := hst
  simp only [GlobalStorage.wellTyped, Bool.and_eq_true, decide_eq_true_eq, List.all_eq_true] at hwt
  refine ⟨by simp [hfr], fun x hx => ?_, by rw [hsto]; exact hwt.1, fun r hr => ?_,
    by rw [hhe]; exact hheld⟩
  · rw [hfr, List.mem_singleton] at hx
    subst hx
    rw [hid, hnf]
    exact Nat.zero_lt_one
  · rw [hsto] at hr
    exact conforms_type (hwt.2 r hr).1 rfl

/-- The typing invariant at an initial state. -/
theorem typed_initial {P : Program} {f : FunctionRef} {args : List Value}
    {storage : GlobalStorage} {held : List Value} {s₀ : State} {ps : List RuntimeType}
    (h : initialState P f args storage held = some s₀) (hps : P.paramTypes f = some ps)
    (hconf : conformsAll P args ps = true) (hst : storage.wellTyped P = true) :
    StateTyped P s₀ := by
  obtain ⟨fr, localTys, hfr, hstk, hfn, -, hlt, hloc, hsto, hhe, -, hha⟩ := initialState_shape h
  have hwt := hst
  simp only [GlobalStorage.wellTyped, Bool.and_eq_true, decide_eq_true_eq, List.all_eq_true] at hwt
  have hvals : ∀ v ∈ frameVals fr, v ∈ args := by
    intro v hv
    simp only [frameVals, hstk, hloc, List.nil_append, List.toList_toArray, List.mem_filterMap, id,
      List.mem_append, List.mem_map, List.mem_replicate] at hv
    obtain ⟨o, ho, hov⟩ := hv
    rcases ho with ⟨w, hw, rfl⟩ | ⟨-, rfl⟩
    · cases hov; exact hw
    · cases hov
  have hargs : ∀ x ∈ s₀.frames, ∀ y ∈ x.function.typeArgs, y.isValue = true := by
    intro x hx
    rw [hfr, List.mem_singleton] at hx
    subst hx
    rw [hfn]
    exact typeArgsOk_values (initialState_typeArgsOk h)
  refine ⟨?_, ?_, ?_, ?_, ?_, hargs⟩
  rotate_left 4
  · intro v hv
    simp only [heldArguments, Bool.and_eq_true, List.all_eq_true] at hha
    rw [hhe] at hv
    obtain ⟨hval, hwf⟩ := hha.1.1.1 v hv
    have hc : v.conforms P v.type = true := by simp [Value.conforms, hwf]
    exact ⟨(conforms_shaped hc).1, hval⟩
  · intro x hx v hv
    rw [hfr, List.mem_singleton] at hx
    subst hx
    obtain ⟨t, -, hc⟩ := conformsAll_mem hconf v (hvals v hv)
    exact (conforms_shaped hc).1
  · intro r hr
    rw [hsto] at hr
    exact (conforms_shaped (hwt.2 r hr).1).1
  · intro x hx
    rw [hfr, List.mem_singleton] at hx
    subst hx
    rw [hfn]
    obtain ⟨ls, rfl⟩ := localTypes_params hlt hps
    refine ⟨_, hlt, ?_⟩
    rw [hloc]
    exact args_locals_typed hconf
  · intro x hx r hr
    rw [hfr, List.mem_singleton] at hx
    subst hx
    have hra := hvals _ hr
    simp only [heldArguments, Bool.and_eq_true, List.all_eq_true] at hha
    have h3 := hha.1.2 r (List.mem_filterMap.2 ⟨_, hra, rfl⟩)
    obtain ⟨-, h3⟩ := h3
    split at h3
    · next k hroot hpath =>
      split at h3
      · next v hv =>
        refine ⟨fun T hT => ?_, fun f i hf => ?_, fun k' w hk' hw => ?_⟩
        · rw [hroot] at hT
          obtain rfl : v.type = T := by simpa [rootType, hhe, hv] using hT
          rw [hpath]
          rcases eq_or_nonrigid (conforms_shaped h3).2 with he | he
          · exact Or.inl (by simp [pathType, he])
          · exact Or.inr ⟨⟨k, hroot⟩, rfl, he⟩
        · rw [hroot] at hf; cases hf
        · rw [hroot, Root.held.injEq] at hk'
          subst hk'
          rw [hhe, hv, Option.some.injEq] at hw
          subst hw
          exact (conforms_shaped h3).1
      · simp at h3
    · simp at h3

/-- D3 for a value the transaction creates of an accepted argument type. -/
theorem transaction_respected {π : NativePolicy} {A : List StructId} {auth : List Address}
    {i : Nat} {w : Value} (hw : w.type.IsTransactionArgument A = true) :
    Encapsulation.structsRespected π A (.created i w .transaction) = true ∧
      Encapsulation.signersRespected π auth (.created i w .transaction) = true := by
  cases w <;> simp_all [Encapsulation.structsRespected, Encapsulation.signersRespected, Value.type,
    RuntimeType.IsTransactionArgument, Creator.MayCreateStruct]

/-! ## What the transaction creates -/

section Creations

variable {P : Program} {π : NativePolicy} {A : List StructId} {auth : List Address}

/-- What the invariant needs of a value the initial shadow creates by `c`. -/
def Fine (π : NativePolicy) (A : List StructId) (auth : List Address) (c : Creator) (w : Value) :
    Prop :=
  w.key.Ok true ∧ ∀ i, Encapsulation.structsRespected π A (.created i w c) = true ∧
    Encapsulation.signersRespected π auth (.created i w c) = true

theorem signer_fine {a : Address} (ha : a ∈ auth) : Fine π A auth .transaction (.signer a) :=
  ⟨by simp [Value.key, Key.Ok], fun i => by
    simp [Encapsulation.structsRespected, Encapsulation.signersRespected, Value.type,
      Creator.MayCreateSigner, ha]⟩

/-- A signer's address, created with it by the transaction. -/
theorem address_fine {a : Address} : Fine π A auth .transaction (.address a) :=
  ⟨by simp [Value.key, Key.Ok], fun i => by
    simp [Encapsulation.structsRespected, Encapsulation.signersRespected, Value.type]⟩

/-- A signer's nodes: itself and its address. -/
theorem mem_subvalues_signer {a : Address} {w : Value} (hw : w ∈ subvalues (.signer a)) :
    w = .signer a ∨ w = .address a := by
  simpa [subvalues, Value.nodes] using hw

theorem signerArgs_mem : ∀ {signers : List (Address × Bool)} {k : Nat} {v : Value},
    v ∈ signerArgs signers k → (∃ a, (a, false) ∈ signers ∧ v = .signer a) ∨
      ∃ k', v = .ref ⟨false, .signer, ⟨.held k', []⟩, 0⟩
  | [], _, _, h => by simp [signerArgs] at h
  | (a, false) :: rest, k, v, h => by
    simp only [signerArgs, List.mem_cons] at h
    rcases h with rfl | h
    · exact Or.inl ⟨a, List.mem_cons_self .., rfl⟩
    · rcases signerArgs_mem h with ⟨a', ha', rfl⟩ | h'
      · exact Or.inl ⟨a', List.mem_cons_of_mem _ ha', rfl⟩
      · exact Or.inr h'
  | (a, true) :: rest, k, v, h => by
    simp only [signerArgs, List.mem_cons] at h
    rcases h with rfl | h
    · exact Or.inr ⟨k, rfl⟩
    · rcases signerArgs_mem h with ⟨a', ha', rfl⟩ | h'
      · exact Or.inl ⟨a', List.mem_cons_of_mem _ ha', rfl⟩
      · exact Or.inr h'

/-- E1 and E2: the arguments' nodes. -/
theorem args_fine (hA : P.TransactionArgumentStructs A) {signers : List (Address × Bool)}
    (hauth : ∀ x ∈ signers, x.1 ∈ auth) {tys : List RuntimeType}
    (htys : ∀ t ∈ tys, t.IsTransactionArgument A = true) {args : List Value}
    (hconf : conformsAll P args tys = true) :
    ∀ v ∈ signerArgs signers 0 ++ args, ∀ w ∈ subvalues v, Fine π A auth .transaction w := by
  intro v hv w hw
  rcases List.mem_append.1 hv with hv | hv
  · rcases signerArgs_mem hv with ⟨a, ha, rfl⟩ | ⟨k, rfl⟩
    · rcases mem_subvalues_signer hw with rfl | rfl
      · exact signer_fine (hauth _ ha)
      · exact address_fine
    · simp only [subvalues_eq, Value.children, List.flatMap_nil, List.mem_singleton] at hw
      subst hw
      refine ⟨by simp [Value.key, Key.Ok, Root.isHeld], fun i => ?_⟩
      simp [Encapsulation.structsRespected, Encapsulation.signersRespected, Value.type]
  · obtain ⟨t, ht, hvt, hwf⟩ := conformsAll_type hconf v hv
    have hwT := deep_argument hA v hwf
      (assignable_rigid hvt (argument_rigid (htys t ht)) ▸ htys t ht) w hw
    refine ⟨container_ok fun r hr => ?_, fun i => transaction_respected hwT⟩
    subst hr
    simp [Value.type, RuntimeType.IsTransactionArgument] at hwT

/-- E1: the transaction supplies no closure (D3c). -/
theorem args_closures (hA : P.TransactionArgumentStructs A) {signers : List (Address × Bool)}
    {tys : List RuntimeType} (htys : ∀ t ∈ tys, t.IsTransactionArgument A = true)
    {args : List Value} (hconf : conformsAll P args tys = true) :
    ∀ v ∈ signerArgs signers 0 ++ args, ∀ w ∈ subvalues v, ∀ i,
      Encapsulation.closuresRespected P (.created i w .transaction) = true := by
  intro v hv w hw i
  rcases List.mem_append.1 hv with hv | hv
  · rcases signerArgs_mem hv with ⟨a, ha, rfl⟩ | ⟨k, rfl⟩
    · rcases mem_subvalues_signer hw with rfl | rfl <;> rfl
    · simp only [subvalues_eq, Value.children, List.flatMap_nil, List.mem_singleton] at hw
      subst hw; rfl
  · obtain ⟨t, ht, hvt, hwf⟩ := conformsAll_type hconf v hv
    have hwT := deep_argument hA v hwf
      (assignable_rigid hvt (argument_rigid (htys t ht)) ▸ htys t ht) w hw
    cases w with
    | closure => simp [Value.type, RuntimeType.IsTransactionArgument] at hwT
    | _ => rfl

/-- Well-typed storage: its nodes are neither references nor signers. -/
theorem storage_fine {storage : GlobalStorage} (hst : storage.wellTyped P = true) :
    ∀ v ∈ storage.map (·.value), ∀ w ∈ subvalues v, Fine π A auth .storage w := by
  intro v hv w hw
  obtain ⟨r, hr, rfl⟩ := List.mem_map.1 hv
  simp only [GlobalStorage.wellTyped, Bool.and_eq_true, decide_eq_true_eq, List.all_eq_true] at hst
  obtain ⟨hc, hk⟩ := hst.2 r hr
  obtain ⟨hs, -⟩ := conforms_shaped hc
  have hty := conforms_type hc rfl
  have hnr := not_ref_of_isRef (shaped_noRef r.value hs (by rw [hty]; rfl) w hw)
  have hns := resource_no_signer hty hk w hw
  refine ⟨container_ok hnr, fun i => ?_⟩
  cases w with
  | signer a => exact absurd rfl (hns a)
  | _ => simp [Encapsulation.structsRespected, Encapsulation.signersRespected, Value.type,
      Creator.MayCreateStruct]

/-- E2: the held values are authenticated signers. -/
theorem held_fine {signers : List (Address × Bool)} (hauth : ∀ x ∈ signers, x.1 ∈ auth) :
    ∀ v ∈ heldSigners signers, ∀ w ∈ subvalues v, Fine π A auth .transaction w := by
  intro v hv w hw
  simp only [heldSigners, List.mem_map, List.mem_filter] at hv
  obtain ⟨⟨a, b⟩, ⟨hab, -⟩, rfl⟩ := hv
  rcases mem_subvalues_signer hw with rfl | rfl
  · exact signer_fine (hauth _ hab)
  · exact address_fine

theorem heldSigners_signer {signers : List (Address × Bool)} :
    ∀ v ∈ heldSigners signers, ∃ a, v = .signer a := by
  intro v hv
  simp only [heldSigners, List.mem_map, List.mem_filter] at hv
  obtain ⟨⟨a, b⟩, -, rfl⟩ := hv
  exact ⟨a, rfl⟩

end Creations

/-- **The invariant at a transaction's initial state** (E1, E2). -/
theorem entry_inv {P : Program} {π : NativePolicy} {A : List StructId} {auth : List Address}
    {s₀ : State} (hE : EntryInitial P auth A s₀) :
    TraceInv P π A auth s₀ (Shadow.initial s₀).1 (Shadow.initial s₀).2 := by
  obtain ⟨hA, f, signers, tys, args, storage, -, -, hparams, hauth, htys, hconf, hwt, hinit⟩ := hE
  obtain ⟨fr, localTys, hfr, hst, -, -, -, hloc, hsto, hhe, -, -⟩ := initialState_shape hinit
  have hconf' := conformsAll_signerArgs P signers 0 args tys hconf
  -- Every node is created, by the transaction or by global storage.
  obtain ⟨ls, evs₁, n₁, hf, hlen, hfit, hocc, hb, hnd, -, hev⟩ :=
    locals_fold fr.locals.toList [] 0 []
  obtain ⟨sts, e₂, n₂, hfS, hfitS, hoccS, hbS, hndS, hleS, hevS⟩ :=
    fresh_fold .storage (s₀.storage.map (·.value)) [] n₁ []
  obtain ⟨hts, e₃, n₃, hfH, hfitH, hoccH, hbH, hndH, hleH, hevH⟩ :=
    fresh_fold .transaction s₀.held [] n₂ []
  rw [initial_eq hfr]
  simp only [initL, initK, initS, initH, hst, freshAll_nil, hf, hfS, hfH, List.nil_append,
    List.append_nil]
  have hcr : ∀ e ∈ evs₁ ++ e₂ ++ e₃, ∃ i w c, e = .created i w c ∧ Fine π A auth c w := by
    intro e he
    simp only [List.mem_append] at he
    rcases he with (he | he) | he
    · obtain ⟨i, w, rfl, v, hv, hw⟩ := hev e he
      refine ⟨i, w, _, rfl, args_fine hA hauth htys hconf v ?_ w hw⟩
      rw [hloc, List.toList_toArray, List.mem_append, List.mem_map] at hv
      rcases hv with ⟨v', hv', hvv⟩ | hv
      · simp only [Option.some.injEq] at hvv
        subst hvv
        exact hv'
      · simp [List.mem_replicate] at hv
    · obtain ⟨i, w, rfl, v, hv, hw⟩ := hevS e he
      rw [hsto] at hv
      exact ⟨i, w, _, rfl, storage_fine hwt v hv w hw⟩
    · obtain ⟨i, w, rfl, v, hv, hw⟩ := hevH e he
      rw [hhe] at hv
      exact ⟨i, w, _, rfl, held_fine hauth v hv w hw⟩
  have hclo : ∀ e ∈ evs₁ ++ e₂ ++ e₃, Encapsulation.closuresRespected P e = true := by
    intro e he
    simp only [List.mem_append] at he
    rcases he with (he | he) | he
    · obtain ⟨i, w, rfl, v, hv, hw⟩ := hev e he
      refine args_closures (signers := signers) hA htys hconf v ?_ w hw i
      rw [hloc, List.toList_toArray, List.mem_append, List.mem_map] at hv
      rcases hv with ⟨v', hv', hvv⟩ | hv
      · simp only [Option.some.injEq] at hvv
        subst hvv
        exact hv'
      · simp [List.mem_replicate] at hv
    · obtain ⟨i, w, rfl, v, hv, hw⟩ := hevS e he
      cases w <;> rfl
    · obtain ⟨i, w, rfl, v, hv, hw⟩ := hevH e he
      rw [hhe] at hv
      obtain ⟨a, rfl⟩ := heldSigners_signer v hv
      rcases mem_subvalues_signer hw with rfl | rfl <;> rfl
  have hfit' : slotsFit ⟨ls.toArray, []⟩ fr = true := by simpa [slotsFit] using hfit
  have hlo : localsOcc ⟨ls.toArray, []⟩ fr = createdOf evs₁ := by simpa [localsOcc] using hocc
  have hoccs : (Shadow.mk [⟨ls.toArray, []⟩] sts hts n₃).occurrences s₀ =
      createdOf evs₁ ++ createdOf e₂ ++ createdOf e₃ := by
    rw [shadow_occurrences, hfr]
    simp only [framesOcc, List.zip_nil_left, List.flatMap_nil, List.append_nil,
      frame_occurrences, hlo, extOcc, hoccS, hoccH, IdTree.occurrencesAll, List.append_assoc,
      List.zip_cons_cons, List.flatMap_cons]
  have hcall : createdOf (evs₁ ++ e₂ ++ e₃ ++ initRefs ⟨ls.toArray, []⟩ fr) =
      createdOf evs₁ ++ createdOf e₂ ++ createdOf e₃ := by
    simp only [createdOf_append, createdOf_initRefs, List.append_nil]
  have hK : ((createdOf evs₁ ++ createdOf e₂ ++ createdOf e₃).map (·.1)).Nodup := by
    rw [List.map_append, List.map_append, List.nodup_append, List.nodup_append]
    refine ⟨⟨hnd, hndS, fun a ha b hb' hab => ?_⟩, hndH, fun a ha b hb' hab => ?_⟩
    · obtain ⟨x, hx, rfl⟩ := List.mem_map.1 ha
      obtain ⟨y, hy, rfl⟩ := List.mem_map.1 hb'
      have h1 := (hb x hx).2
      have h2 := (hbS y hy).1
      omega
    · obtain ⟨y, hy, rfl⟩ := List.mem_map.1 hb'
      have h2 := (hbH y hy).1
      rcases List.mem_append.1 ha with ha | ha
      · obtain ⟨x, hx, rfl⟩ := List.mem_map.1 ha
        have h1 := (hb x hx).2
        omega
      · obtain ⟨x, hx, rfl⟩ := List.mem_map.1 ha
        have h1 := (hbS x hx).2
        omega
  have hiss : ∀ x ∈ issuedOf (evs₁ ++ e₂ ++ e₃ ++ initRefs ⟨ls.toArray, []⟩ fr),
      x ∈ keys (createdOf evs₁ ++ createdOf e₂ ++ createdOf e₃) := by
    intro x hx
    rw [issuedOf_append, List.mem_append] at hx
    rcases hx with hx | hx
    · have hnb : ∀ i r p, Event.borrowed i r p ∉ evs₁ ++ e₂ ++ e₃ := fun i r p hp => by
        obtain ⟨_, _, _, he, -⟩ := hcr _ hp
        cases he
      rw [issuedOf_created hnb, createdOf_append, createdOf_append] at hx
      exact hx
    · have := initRefs_issued hfit' x hx
      rw [hlo] at this
      simp only [keys_append, List.mem_append]
      exact Or.inl (Or.inl this)
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · have hg : (⟨ls.toArray, []⟩ : ShadowFrame).fits fr = true := by
      rw [frame_fits]
      simp only [Bool.and_eq_true, beq_iff_eq]
      refine ⟨⟨by simp [hlen], hfit'⟩, by rw [hst]; rfl⟩
    rw [shadow_fits, hfr, framesFit_cons, hg]
    simp [framesFit, extFit, hfitS, hfitH]
  · refine inv_initial hinit hwt fun _ v hv => ?_
    obtain ⟨a, rfl⟩ := heldSigners_signer v hv
    simp [Value.abilities, RuntimeType.abilities]
  · intro x hx
    rw [hoccs] at hx
    obtain ⟨o, ho, -, hox⟩ := mem_keys_iff.1 hx
    have ho' : o ∈ createdOf (evs₁ ++ e₂ ++ e₃) := by
      rw [createdOf_append, createdOf_append]; exact ho
    obtain ⟨c, hc⟩ := mem_createdOf.1 ho'
    obtain ⟨i, w, c', he, hfine⟩ := hcr _ hc
    simp only [Event.created.injEq] at he
    obtain ⟨-, hw, -⟩ := he
    rw [← hox, hw]
    exact hfine.1
  · exact typed_initial hinit hparams hconf' hwt
  · refine ⟨fun x hx => ?_, fun x hx y hy hxy => ?_, fun x hx => ?_⟩
    · rw [hoccs, ← hcall] at hx
      exact keys_created_sub x hx
    · have hK' : ((keys (createdOf evs₁ ++ createdOf e₂ ++ createdOf e₃)).map (·.1)).Nodup := by
        simpa [keys, Function.comp_def] using hK
      exact eq_of_fst_nodup hK' (hiss x hx) (hiss y hy) hxy
    · obtain ⟨o, ho, ho1, -⟩ := mem_keys_iff.1 (hiss x hx)
      rw [← ho1]
      show o.1 < n₃
      simp only [List.mem_append] at ho
      rcases ho with (ho | ho) | ho
      · have := (hb o ho).2
        omega
      · have := (hbS o ho).2
        omega
      · exact (hbH o ho).2
  · rw [hoccs]
    simp only [Ownership.unique, List.all_eq_true]
    intro x _
    simp only [Bool.or_eq_true, decide_eq_true_eq]
    exact Or.inr (count_le_one_of_nodup (by simpa only [List.append_assoc] using hK) _)
  · intro e he
    rcases List.mem_append.1 he with he | he
    · obtain ⟨i, w, c, rfl, hfine⟩ := hcr e he
      exact hfine.2 i
    · simp only [initRefs, List.mem_append, List.mem_map, List.mem_singleton] at he
      rcases he with ⟨⟨i, r⟩, -, rfl⟩ | rfl <;>
        simp [Encapsulation.structsRespected, Encapsulation.signersRespected]
  · intro o ho
    rw [hcall] at ho
    exact Or.inr (Or.inl ⟨o, by rw [hoccs]; exact ho, rfl⟩)
  · intro e he
    rcases List.mem_append.1 he with he | he
    · exact hclo e he
    · simp only [initRefs, List.mem_append, List.mem_map, List.mem_singleton] at he
      rcases he with ⟨⟨i, r⟩, -, rfl⟩ | rfl <;> rfl
  · intro x hx _
    rw [hoccs] at hx
    have ho' : x ∈ createdOf (evs₁ ++ e₂ ++ e₃) := by
      rw [createdOf_append, createdOf_append]; exact hx
    obtain ⟨c, hc⟩ := mem_createdOf.1 ho'
    exact ⟨c, List.mem_append_left _ hc⟩

end MoveBytecode.Proofs
