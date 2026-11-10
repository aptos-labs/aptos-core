-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.FunctionValuesSteps
import Proofs.GhostInitial
import Proofs.Typing
import Proofs.AbilitiesGen
import Proofs.GhostModeled

/-!
# D5: function values

UNTRUSTED (checked by the kernel). A program the model accepts keeps every
closure of every reachable state well formed, capturing no reference, with
abilities justified by its target and its captured values (D5a, D5b); every
closure in global storage targets a storable function (D5d); and every
`CallClosure` that continues enters its target with exactly its parameters
(D5c, `callClosure_cont`), so every execution satisfies D5
(`functionValueSafety`).

The closures of an execution are those of its initial state, those
`PackClosure` creates, and those natives return, which the semantics checks
against their declared types. Every step keeps them:

- the ghost layer follows each closure node across a step of the
  deterministic core (`StepGhost.closuresKept`): it is a closure node before
  the step, or one the step's annotation creates; a step of an instruction
  other than `PackClosure` creates none (`Proofs.FunctionValuesSteps`), and
  `PackClosure` creates a well-formed closure, which passes D5a and D5b;
- a native call replaces its arguments by its results, which conform to its
  return types.

The invariant (`FVInv`) carries a shadow of the state, so that the ghost
layer applies, the typing invariant, every resource in global storage of a
type with `key`, and `closureOk` at every occurrence. A closure in global storage
sits below a resource of a type with `key`, whose fields have `store` all
the way down, so its abilities include `store`, which only a closure over a
storable function has.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Step

section Values

variable {P : Program}

/-! ## Closure nodes -/

/-- `closureOk` holds at every value that is no closure. -/
theorem closureOk_of_not {w : Value} (h : IsClosure w = false) :
    FunctionValues.closureOk P w = true := by
  cases w <;> simp_all [IsClosure, FunctionValues.closureOk]

/-- A property closed under taking children holds at every node. -/
theorem subvalues_closed (p : Value → Prop) (hp : ∀ v c, p v → c ∈ v.children → p c) :
    ∀ v, p v → ∀ w ∈ subvalues v, p w := by
  intro v
  induction v using Value.rec
    (motive_2 := fun vs => ∀ c ∈ vs, p c → ∀ w ∈ subvalues c, p w) with
  | vector e vs ih =>
    intro hv w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hv
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      exact ih c hc (hp _ c hv hc) w hwc
  | struct s fs ih =>
    intro hv w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hv
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      exact ih c hc (hp _ c hv hc) w hwc
  | variant s tag fs ih =>
    intro hv w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hv
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      exact ih c hc (hp _ c hv hc) w hwc
  | closure f mask cs ps rs a ih =>
    intro hv w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hv
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      exact ih c hc (hp _ c hv hc) w hwc
  | table s fs ks es ihf _ ihe =>
    intro hv w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hv
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hpc := hp _ c hv hc
      simp only [Value.children, List.mem_append] at hc
      rcases hc with hc | hc
      · exact ihf c hc hpc w hwc
      · exact ihe c hc hpc w hwc
  | nil => rename_i hc _ _ _; simp at hc
  | cons x xs ihx ihxs =>
    rename_i c hc hpc w hw
    rcases List.mem_cons.1 hc with rfl | hc
    · exact ihx hpc w hw
    · exact ihxs c hc hpc w hw
  | signer a =>
    intro hv w hw
    simp only [subvalues, Value.nodes, List.map_cons, List.map_nil, List.mem_cons,
      List.not_mem_nil, or_false] at hw
    rcases hw with rfl | rfl
    · exact hv
    · exact hp _ _ hv (by simp [Value.children])
  | _ =>
    intro hv w hw
    rw [subvalues_eq] at hw
    simp only [Value.children, List.flatMap_nil, List.mem_singleton] at hw
    subst hw
    exact hv

/-- The children of a well-formed value are well formed. -/
theorem wellFormed_child {v c : Value} (hv : v.wellFormed P = true) (hc : c ∈ v.children) :
    c.wellFormed P = true := by
  cases v with
  | vector e vs =>
    rw [wellFormed_vector] at hv
    exact conforms_wellFormed (hv c hc)
  | struct s fs =>
    obtain ⟨-, tys, -, h⟩ := wellFormed_struct.1 hv
    obtain ⟨t, -, ht⟩ := conformsAll_mem h c hc
    exact conforms_wellFormed ht
  | table s fs ks es =>
    obtain ⟨-, -, ⟨tys, -, h⟩, hes⟩ := wellFormed_table.1 hv
    simp only [Value.children, List.mem_append] at hc
    rcases hc with hc | hc
    · obtain ⟨t, -, ht⟩ := conformsAll_mem h c hc
      exact conforms_wellFormed ht
    · rcases hes with rfl | ⟨-, k, v, -, -, hce, -⟩
      · simp at hc
      · exact conforms_wellFormed (allConform_iff.1 hce c hc)
  | variant s tag fs =>
    obtain ⟨tys, -, h⟩ := wellFormed_variant.1 hv
    obtain ⟨t, -, ht⟩ := conformsAll_mem h c hc
    exact conforms_wellFormed ht
  | closure f mask cs ps rs a =>
    simp only [Value.wellFormed] at hv
    split at hv
    · simp only [Bool.and_eq_true] at hv
      obtain ⟨t, -, ht⟩ := conformsAll_mem hv.2 c hc
      exact conforms_wellFormed ht
    · simp at hv
  | signer a =>
    simp only [Value.children, List.mem_singleton] at hc
    subst hc; rfl
  | _ => simp [Value.children] at hc

/-- **A well-formed closure passes D5a and D5b**, in a program whose
declared abilities are justified by their fields. -/
theorem closureOk_of_wellFormed (hP : AbilitiesSound P) {w : Value}
    (hw : w.wellFormed P = true) : FunctionValues.closureOk P w = true := by
  cases w with
  | closure f mask cs ps rs a =>
    have hw' := hw
    simp only [Value.wellFormed] at hw'
    split at hw'
    · next ctys rem res hct =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hw'
      obtain ⟨⟨⟨rfl, rfl⟩, rfl⟩, hc⟩ := hw'
      have hval := closureType?_captured_value hct
      simp only [FunctionValues.closureOk, Bool.and_eq_true, List.all_eq_true]
      refine ⟨⟨⟨fun c hcm => ?_, hw⟩, ?_⟩, fun c hcm => ?_⟩
      · obtain ⟨t, ht, hct'⟩ := conformsAll_mem hc c hcm
        obtain ⟨hs, hty⟩ := conforms_shaped hct'
        have hno := shaped_noRef c hs (by rw [assignable_isValue hty]; exact hval t ht)
        simp only [Value.referenceFree, List.all_eq_true]
        intro x hx
        have := hno x.2 (List.mem_map_of_mem hx)
        simp [this]
      · exact AbilitySet.inter_subset_left' _ _
      · obtain ⟨t, -, hct'⟩ := conformsAll_mem hc c hcm
        exact ability_subset_trans (ability_subset_trans (AbilitySet.inter_subset_right _ _)
          (capturedAbilities_sub (List.mem_map_of_mem hcm)))
          (abilities_of_conforms hP (conforms_of (assignable_refl _) (conforms_wellFormed hct')))
    · simp at hw'
  | _ => rfl

/-- Every node of a well-formed value passes D5a and D5b. -/
theorem closureOk_subvalues (hP : AbilitiesSound P) {v : Value} (hv : v.wellFormed P = true) :
    ∀ w ∈ subvalues v, FunctionValues.closureOk P w = true := fun w hw =>
  closureOk_of_wellFormed hP
    (subvalues_closed (fun v => v.wellFormed P = true) (fun _ _ h hc => wellFormed_child h hc) v
      hv w hw)

/-! ## Closures in global storage -/

theorem store_forContainer {a : AbilitySet} (h : a.forContainer.store = true) : a.store = true := h

theorem store_of_key_forContainer {a : AbilitySet} (h : a.forContainer.key = true) :
    a.store = true := h

/-- **A shaped value of a type with `store` has `store` all the way down**,
in a program whose declared abilities are justified by their fields. -/
theorem store_subvalues (hP : AbilitiesSound P) :
    ∀ v : Value, shaped P v = true → (v.type.abilities P).store = true →
      ∀ w ∈ subvalues v, (w.type.abilities P).store = true := by
  intro v
  induction v using Value.rec
    (motive_2 := fun vs => ∀ c ∈ vs, shaped P c = true → (c.type.abilities P).store = true →
      ∀ w ∈ subvalues c, (w.type.abilities P).store = true) with
  | vector e vs ih =>
    intro hs hst w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hst
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [shaped, shapedAll_iff] at hs
      obtain ⟨hct, hcs⟩ := hs c hc
      have he : (e.abilities P).store = true := by
        simp only [Value.type, RuntimeType.abilities, MoveBytecode.AbilitySet.inter,
          Bool.and_eq_true] at hst
        exact hst.2
      exact ih c hc hcs (store_of_subset' (assignable_abilities hct) he) w hwc
  | struct s fs ih =>
    intro hs hst w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hst
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [shaped] at hs
      split at hs
      · next tys htys =>
        obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hc
        obtain ⟨t, htj, hct, hcs⟩ := shapedFields_get hs hj
        have ht := hP s tys (Or.inl htys) t (List.mem_of_getElem? htj)
        have hts := store_forContainer (store_of_subset' ht hst)
        exact ih c hc hcs (store_of_subset' (assignable_abilities hct) hts) w hwc
      · simp at hs
  | variant s tag fs ih =>
    intro hs hst w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hst
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [shaped] at hs
      split at hs
      · next tys htys =>
        obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hc
        obtain ⟨t, htj, hct, hcs⟩ := shapedFields_get hs hj
        have ht := hP s tys (Or.inr ⟨tag, htys⟩) t (List.mem_of_getElem? htj)
        have hts := store_forContainer (store_of_subset' ht hst)
        exact ih c hc hcs (store_of_subset' (assignable_abilities hct) hts) w hwc
      · simp at hs
  | table s fs ks es ihf _ ihe =>
    intro hs hst w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hst
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [shaped, Bool.and_eq_true, Bool.or_eq_true, List.isEmpty_iff] at hs
      obtain ⟨⟨-, hf⟩, hes⟩ := hs
      simp only [Value.children, List.mem_append] at hc
      rcases hc with hc | hc
      · split at hf
        · next tys htys =>
          obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hc
          obtain ⟨t, htj, hct, hcs⟩ := shapedFields_get hf hj
          have ht := hP s tys (Or.inl htys) t (List.mem_of_getElem? htj)
          have hts := store_forContainer (store_of_subset' ht hst)
          exact ihf c hc hcs (store_of_subset' (assignable_abilities hct) hts) w hwc
        · simp at hf
      · rcases hes with rfl | ⟨-, he⟩
        · simp at hc
        · split at he
          · next k v hargs =>
            simp only [Bool.and_eq_true] at he
            obtain ⟨hct, hcs⟩ := (shapedAll_iff.1 he.1) c hc
            have hts := store_forContainer (store_of_subset' he.2 hst)
            exact ihe c hc hcs (store_of_subset' (assignable_abilities hct) hts) w hwc
          · simp at he
  | closure f mask cs ps rs a ih =>
    intro hs hst w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact hst
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      simp only [shaped] at hs
      split at hs
      · next ctys rem res hct =>
        simp only [Bool.and_eq_true, beq_iff_eq] at hs
        obtain ⟨⟨⟨-, -⟩, rfl⟩, hcs⟩ := hs
        simp only [Value.type, RuntimeType.abilities] at hst
        obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hc
        obtain ⟨t, -, -, hcs'⟩ := shapedFields_get hcs hj
        exact ih c hc hcs' (store_of_subset' (ability_subset_trans
          (AbilitySet.inter_subset_right _ _) (capturedAbilities_sub (List.mem_map_of_mem hc))) hst)
          w hwc
      · simp at hs
  | nil => rename_i hc _ _ _ _; simp at hc
  | cons x xs ihx ihxs =>
    rename_i c hc hcs hst w hw
    rcases List.mem_cons.1 hc with rfl | hc
    · exact ihx hcs hst w hw
    · exact ihxs c hc hcs hst w hw
  | signer a =>
    intro _ hst w hw
    simp [Value.type, RuntimeType.abilities] at hst
  | _ =>
    intro _ hst w hw
    rw [subvalues_eq] at hw
    simp only [Value.children, List.flatMap_nil, List.mem_singleton] at hw
    subst hw
    exact hst

/-- **A closure below a resource of a type with `key` has `store`.** -/
theorem closure_store_of_key (hP : AbilitiesSound P) {v : Value} {t : StructId}
    (hs : shaped P v = true) (hv : v.type = .struct t)
    (hk : ((RuntimeType.struct t).abilities P).key = true) {f : FunctionRef} {mask : BitVec 64}
    {cs : List Value} {ps rs : List RuntimeType} {a : AbilitySet}
    (hw : Value.closure f mask cs ps rs a ∈ subvalues v) : a.store = true := by
  rw [subvalues_eq] at hw
  rcases List.mem_cons.1 hw with h | hw
  · rw [← h] at hv; simp [Value.type] at hv
  · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
    have hcst : (c.type.abilities P).store = true ∧ shaped P c = true := by
      cases v with
      | struct s fs =>
        simp only [Value.type, RuntimeType.struct.injEq] at hv
        subst hv
        simp only [shaped] at hs
        split at hs
        · next tys htys =>
          obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hc
          obtain ⟨u, huj, hcu, hcs⟩ := shapedFields_get hs hj
          have hu := hP s tys (Or.inl htys) u (List.mem_of_getElem? huj)
          exact ⟨store_of_subset' (assignable_abilities hcu)
            (store_of_key_forContainer (key_of_subset' hu hk)), hcs⟩
        · simp at hs
      | variant s tag fs =>
        simp only [Value.type, RuntimeType.struct.injEq] at hv
        subst hv
        simp only [shaped] at hs
        split at hs
        · next tys htys =>
          obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hc
          obtain ⟨u, huj, hcu, hcs⟩ := shapedFields_get hs hj
          have hu := hP s tys (Or.inr ⟨tag, htys⟩) u (List.mem_of_getElem? huj)
          exact ⟨store_of_subset' (assignable_abilities hcu)
            (store_of_key_forContainer (key_of_subset' hu hk)), hcs⟩
        · simp at hs
      | table s fs ks es =>
        simp only [Value.type, RuntimeType.struct.injEq] at hv
        subst hv
        simp only [shaped, Bool.and_eq_true, Bool.or_eq_true, List.isEmpty_iff] at hs
        obtain ⟨⟨-, hf⟩, hes⟩ := hs
        simp only [Value.children, List.mem_append] at hc
        rcases hc with hc | hc
        · split at hf
          · next tys htys =>
            obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hc
            obtain ⟨u, huj, hcu, hcs⟩ := shapedFields_get hf hj
            have hu := hP s tys (Or.inl htys) u (List.mem_of_getElem? huj)
            exact ⟨store_of_subset' (assignable_abilities hcu)
              (store_of_key_forContainer (key_of_subset' hu hk)), hcs⟩
          · simp at hf
        · rcases hes with rfl | ⟨-, he⟩
          · simp at hc
          · split at he
            · next k v hargs =>
              simp only [Bool.and_eq_true] at he
              obtain ⟨hcu, hcs⟩ := (shapedAll_iff.1 he.1) c hc
              exact ⟨store_of_subset' (assignable_abilities hcu)
                (store_of_key_forContainer (key_of_subset' he.2 hk)), hcs⟩
            · simp at he
      | _ => simp [Value.type] at hv
    have := store_subvalues hP c hcst.2 hcst.1 _ hwc
    simpa [Value.type, RuntimeType.abilities] using this

/-- A closure that passes D5b and has `store` targets a storable function. -/
theorem storable_of_store {f : FunctionRef} {mask : BitVec 64} {cs : List Value}
    {ps rs : List RuntimeType} {a : AbilitySet}
    (hok : FunctionValues.closureOk P (.closure f mask cs ps rs a) = true) (hst : a.store = true) :
    P.storable f = true := by
  simp only [FunctionValues.closureOk, Bool.and_eq_true] at hok
  have := store_of_subset' hok.1.2 hst
  simpa [Program.closureAbilities] using this

end Values

/-! ## Occurrences and the values of a state -/

/-- A value of state `s`: in a frame's slots, a resource, or a held value. -/
def StateVal (s : State) (v : Value) : Prop :=
  (∃ fr ∈ s.frames, v ∈ frameVals fr) ∨ (∃ r ∈ s.storage, v = r.value) ∨ v ∈ s.held

theorem exists_zip_left {α β : Type} :
    ∀ {l₁ : List α} {l₂ : List β}, l₁.length = l₂.length → ∀ {b : β}, b ∈ l₂ →
      ∃ a, (a, b) ∈ l₁.zip l₂
  | [], [], _, _, hb => by simp at hb
  | a :: l₁, b' :: l₂, h, b, hb => by
    rcases List.mem_cons.1 hb with rfl | hb
    · exact ⟨a, by simp⟩
    · obtain ⟨a', ha'⟩ := exists_zip_left (l₁ := l₁) (by simpa using h) hb
      exact ⟨a', by simp [ha']⟩
  | [], _ :: _, h, _, _ => by simp at h
  | _ :: _, [], h, _, _ => by simp at h

/-- The occurrences in the frames are nodes of the frames' values. -/
theorem framesOcc_value {gs : List ShadowFrame} {frs : List Frame}
    (hf : framesFit gs frs = true) {x : Nat × Value} (hx : x ∈ framesOcc gs frs) :
    ∃ fr ∈ frs, ∃ v ∈ frameVals fr, x.2 ∈ subvalues v := by
  simp only [framesFit, Bool.and_eq_true, List.all_eq_true] at hf
  obtain ⟨⟨g, fr⟩, hp, hx⟩ := List.mem_flatMap.1 hx
  have hg := hf.2 _ hp
  refine ⟨fr, (List.of_mem_zip hp).2, ?_⟩
  dsimp only at hx hg
  rw [frame_occurrences] at hx
  rcases List.mem_append.1 hx with hx | hx
  · obtain ⟨⟨t, o⟩, hq, hx⟩ := List.mem_flatMap.1 hx
    rw [frame_fits] at hg
    simp only [Bool.and_eq_true] at hg
    have hs : ShadowFrame.slotFits t o = true := by
      have := hg.1.2
      simp only [slotsFit, List.all_eq_true] at this
      exact this _ hq
    cases t with
    | none => simp [ShadowFrame.slotOccurrences] at hx
    | some t =>
      cases o with
      | none => simp [ShadowFrame.slotOccurrences] at hx
      | some v =>
        simp only [ShadowFrame.slotFits] at hs
        simp only [ShadowFrame.slotOccurrences] at hx
        refine ⟨v, ?_, occurrences_mem hs hx⟩
        simp only [frameVals, List.mem_append, List.mem_filterMap, id]
        exact Or.inr ⟨some v, (List.of_mem_zip hq).2, rfl⟩
  · obtain ⟨v, hv, hw⟩ := occurrencesAll_mem (frame_stack_fits hg) hx
    exact ⟨v, mem_frameVals_stack hv, hw⟩

/-- Every occurrence of a fitting shadow is a node of a value of the state. -/
theorem occ_value {h : Shadow} {s : State} (hf : h.fits s = true) {x : Nat × Value}
    (hx : x ∈ h.occurrences s) : ∃ v, StateVal s v ∧ x.2 ∈ subvalues v := by
  rw [shadow_fits, Bool.and_eq_true] at hf
  rw [shadow_occurrences] at hx
  rcases List.mem_append.1 hx with hx | hx
  · obtain ⟨fr, hfr, v, hv, hw⟩ := framesOcc_value hf.1 hx
    exact ⟨v, Or.inl ⟨fr, hfr, hv⟩, hw⟩
  · have he := hf.2
    simp only [extFit, Bool.and_eq_true] at he
    simp only [extOcc, List.mem_append] at hx
    rcases hx with hx | hx
    · obtain ⟨v, hv, hw⟩ := occurrencesAll_mem he.1 hx
      obtain ⟨r, hr, rfl⟩ := List.mem_map.1 hv
      exact ⟨r.value, Or.inr (Or.inl ⟨r, hr, rfl⟩), hw⟩
    · obtain ⟨v, hv, hw⟩ := occurrencesAll_mem he.2 hx
      exact ⟨v, Or.inr (Or.inr hv), hw⟩

theorem mem_occurrencesAll {ts : List IdTree} {vs : List Value} (h : IdTree.fitsAll ts vs = true)
    {v w : Value} (hv : v ∈ vs) (hw : w ∈ subvalues v) :
    ∃ i, (i, w) ∈ IdTree.occurrencesAll ts vs := by
  have : w ∈ (IdTree.occurrencesAll ts vs).map (·.2) := by
    rw [occurrencesAll_snd ts vs h]
    exact List.mem_flatMap.2 ⟨v, hv, hw⟩
  obtain ⟨x, hx, rfl⟩ := List.mem_map.1 this
  exact ⟨x.1, hx⟩

/-- Every node of a value of the state is an occurrence of a fitting
shadow. -/
theorem value_occ {h : Shadow} {s : State} (hf : h.fits s = true) {v : Value} (hv : StateVal s v)
    {w : Value} (hw : w ∈ subvalues v) : ∃ i, (i, w) ∈ h.occurrences s := by
  rw [shadow_fits, Bool.and_eq_true] at hf
  have he := hf.2
  simp only [extFit, Bool.and_eq_true] at he
  rw [shadow_occurrences]
  rcases hv with ⟨fr, hfr, hv⟩ | ⟨r, hr, rfl⟩ | hv
  · have hff := hf.1
    simp only [framesFit, Bool.and_eq_true, List.all_eq_true, beq_iff_eq] at hff
    obtain ⟨g, hp⟩ := exists_zip_left hff.1 hfr
    have hg := hff.2 _ hp
    dsimp only at hg
    have hsub : ∀ y ∈ g.occurrences fr, y ∈ framesOcc h.frames s.frames := fun y hy =>
      List.mem_flatMap.2 ⟨(g, fr), hp, hy⟩
    rw [frame_fits] at hg
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    simp only [frameVals, List.mem_append, List.mem_filterMap, id] at hv
    rcases hv with hv | ⟨o, ho, rfl⟩
    · obtain ⟨i, hi⟩ := mem_occurrencesAll hg.2 hv hw
      exact ⟨i, List.mem_append_left _ (hsub _ (by
        rw [frame_occurrences]; exact List.mem_append_right _ hi))⟩
    · have hlen : g.locals.toList.length = fr.locals.toList.length := by simp [hg.1.1]
      obtain ⟨t, ht⟩ := exists_zip_left hlen ho
      have hs : ShadowFrame.slotFits t (some v) = true := by
        have := hg.1.2
        simp only [slotsFit, List.all_eq_true] at this
        exact this _ ht
      cases t with
      | none => simp [ShadowFrame.slotFits] at hs
      | some t =>
        simp only [ShadowFrame.slotFits] at hs
        have : w ∈ (t.occurrences v).map (·.2) := by rw [occurrences_snd t v hs]; exact hw
        obtain ⟨x, hx, rfl⟩ := List.mem_map.1 this
        refine ⟨x.1, List.mem_append_left _ (hsub _ ?_)⟩
        rw [frame_occurrences]
        exact List.mem_append_left _ (List.mem_flatMap.2 ⟨(some t, some v), ht, hx⟩)
  · obtain ⟨i, hi⟩ := mem_occurrencesAll he.1 (List.mem_map.2 ⟨r, hr, rfl⟩) hw
    exact ⟨i, List.mem_append_right _ (List.mem_append_left _ hi)⟩
  · obtain ⟨i, hi⟩ := mem_occurrencesAll he.2 hv hw
    exact ⟨i, List.mem_append_right _ (List.mem_append_right _ hi)⟩

/-! ## A shadow of the initial state -/

/-- A tree that fits value `v`. -/
def treeOf (v : Value) : IdTree := (IdTree.fresh .storage v 0).1

theorem treeOf_fits (v : Value) : (treeOf v).fits v = true := (fresh_spec .storage v 0).1

theorem fitsAll_map_treeOf : ∀ vs : List Value, IdTree.fitsAll (vs.map treeOf) vs = true
  | [] => rfl
  | v :: vs => by simp [IdTree.fitsAll, treeOf_fits, fitsAll_map_treeOf vs]

/-- A shadow frame that fits frame `fr`. -/
def frameShadow (fr : Frame) : ShadowFrame :=
  { locals := fr.locals.map (Option.map treeOf), stack := fr.stack.map treeOf }

/-- A shadow that fits state `s`. -/
def shadowOf (s : State) : Shadow :=
  ⟨s.frames.map frameShadow, (s.storage.map (·.value)).map treeOf, s.held.map treeOf, 0⟩

theorem frameShadow_fits (fr : Frame) : (frameShadow fr).fits fr = true := by
  rw [frame_fits]
  simp only [frameShadow, Array.size_map, beq_self_eq_true, Bool.true_and, Bool.and_eq_true]
  refine ⟨?_, fitsAll_map_treeOf _⟩
  unfold slotsFit
  simp only [Array.toList_map]
  generalize fr.locals.toList = l
  induction l with
  | nil => rfl
  | cons o l ih =>
    cases o <;> simp_all [ShadowFrame.slotFits, treeOf_fits]

theorem shadowOf_fits (s : State) : (shadowOf s).fits s = true := by
  rw [shadow_fits, Bool.and_eq_true]
  refine ⟨?_, ?_⟩
  · simp only [framesFit, shadowOf, List.length_map, beq_self_eq_true, Bool.true_and]
    generalize s.frames = frs
    induction frs with
    | nil => rfl
    | cons fr frs ih => simp_all [frameShadow_fits]
  · simp only [extFit, shadowOf, Bool.and_eq_true]
    exact ⟨fitsAll_map_treeOf _, fitsAll_map_treeOf _⟩

/-! ## Steps of accepted code -/

/-- The parts of a step of the deterministic core that continues. -/
theorem step_parts {P : Program} {s : State} {o : Outcome} (hstep : step P s = .done o)
    (hcont : o.continues = true) :
    ∃ fr rest info code instr, s.frames = fr :: rest ∧ P.function? fr.function = some info ∧
      info.definition.code = some code ∧ code.code[fr.pc]? = some instr ∧
      stepOf (Step.exec P s fr rest info code instr) = .done o := by
  unfold step at hstep
  split at hstep
  · simp only [StepResult.done.injEq] at hstep; subst hstep; simp [Outcome.continues] at hcont
  · next fr rest hsf =>
    split at hstep
    · simp only [StepResult.done.injEq] at hstep; subst hstep; simp [Outcome.continues] at hcont
    · next info hinfo =>
      split at hstep
      · simp only [StepResult.done.injEq] at hstep; subst hstep; simp [Outcome.continues] at hcont
      · next code hcode =>
        split at hstep
        · simp only [StepResult.done.injEq] at hstep; subst hstep
          simp [Outcome.continues] at hcont
        · next instr hinstr =>
          exact ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hstep⟩

/-! ## The invariant -/

/-- What every reachable state of an accepted program satisfies, with a
shadow `h`: the ghost layer's assumptions, the typing invariant, resources
of types with `key`, and D5a and D5b at every occurrence. -/
structure FVInv (P : Program) (s : State) (h : Shadow) : Prop where
  pre : Pre false P s h
  typed : StateTyped P s
  keys : TypeKeys P s
  good : ∀ x ∈ h.occurrences s, FunctionValues.closureOk P x.2 = true

section Invariant

variable {P : Program}

theorem FVInv.closureOk {s : State} {h : Shadow} (hI : FVInv P s h) {v : Value}
    (hv : StateVal s v) : ∀ w ∈ subvalues v, FunctionValues.closureOk P w = true := by
  intro w hw
  obtain ⟨i, hi⟩ := value_occ hI.pre.fits hv hw
  exact hI.good _ hi

theorem valuesOk_of {vs : List Value}
    (h : ∀ v ∈ vs, ∀ w ∈ subvalues v, FunctionValues.closureOk P w = true) :
    FunctionValues.valuesOk P vs = true := by
  simp only [FunctionValues.valuesOk, List.all_eq_true]
  intro v hv x hx
  exact h v hv x.2 (List.mem_map_of_mem hx)

/-- D5d for global storage of a state of the invariant. -/
theorem storedOk_of (hP : AbilitiesSound P) {s : State} {h : Shadow} (hI : FVInv P s h) :
    FunctionValues.storedOk P s.storage = true := by
  simp only [FunctionValues.storedOk, List.all_eq_true]
  intro r hr x hx
  have hxs : x.2 ∈ subvalues r.value := List.mem_map_of_mem hx
  obtain ⟨i, w⟩ := x
  cases w with
  | closure f mask cs ps rs a =>
    have hok := hI.closureOk (Or.inr (Or.inl ⟨r, hr, rfl⟩)) _ hxs
    exact storable_of_store hok (closure_store_of_key hP (hI.typed.storageShaped r hr)
      (hI.pre.inv.typed r hr) ((typeKeys_iff.1 hI.keys) r hr) hxs)
  | _ => rfl

/-- **D5a, D5b and D5d in a state of the invariant.** -/
theorem stateOk_of (hP : AbilitiesSound P) {s : State} {h : Shadow} (hI : FVInv P s h) :
    FunctionValues.stateOk P s = true := by
  simp only [FunctionValues.stateOk, Bool.and_eq_true]
  refine ⟨⟨⟨valuesOk_of fun v hv => hI.closureOk ?_, valuesOk_of fun v hv =>
    hI.closureOk (Or.inr (Or.inr hv))⟩, valuesOk_of fun v hv => hI.closureOk ?_⟩,
    storedOk_of hP hI⟩
  · obtain ⟨fr, hfr, hv⟩ := List.mem_flatMap.1 hv
    refine Or.inl ⟨fr, hfr, ?_⟩
    simp only [frameVals, List.mem_append] at hv ⊢
    exact hv.symm
  · obtain ⟨r, hr, rfl⟩ := List.mem_map.1 hv
    exact Or.inr (Or.inl ⟨r, hr, rfl⟩)

/-! ## The initial state -/

theorem fvInv_initial (hP : AbilitiesSound P) {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    FVInv P s₀ (shadowOf s₀) := by
  obtain ⟨f, args, tys, storage, held, hps, hconf, hst, hinit⟩ := h0
  obtain ⟨fr, localTys, hfr, hstk, -, -, -, hloc, hsto, hhe, -, hha⟩ := initialState_shape hinit
  have hT := typed_initial hinit hps hconf hst
  have hinv : StateInv false P s₀ := inv_initial hinit hst (fun h => absurd h (by decide))
  have hwt := hst
  simp only [GlobalStorage.wellTyped, Bool.and_eq_true, decide_eq_true_eq, List.all_eq_true] at hwt
  simp only [heldArguments, Bool.and_eq_true, List.all_eq_true] at hha
  have hargs : ∀ v ∈ frameVals fr, v ∈ args := by
    intro v hv
    simp only [frameVals, hstk, hloc, List.nil_append, List.toList_toArray, List.mem_filterMap, id,
      List.mem_append, List.mem_map, List.mem_replicate] at hv
    obtain ⟨o, ho, hov⟩ := hv
    rcases ho with ⟨w, hw, rfl⟩ | ⟨-, rfl⟩
    · cases hov; exact hw
    · cases hov
  have hfv : ∀ fr' ∈ s₀.frames, ∀ v ∈ frameVals fr', v ∈ args := by
    intro fr' hfr' v hv
    rw [hfr, List.mem_singleton] at hfr'
    subst hfr'
    exact hargs v hv
  have hwfv : ∀ v, StateVal s₀ v → v.wellFormed P = true := by
    rintro v (⟨fr', hfr', hv⟩ | ⟨r, hr, rfl⟩ | hv)
    · obtain ⟨t, -, hc⟩ := conformsAll_mem hconf v (hfv fr' hfr' v hv)
      exact conforms_wellFormed hc
    · rw [hsto] at hr
      exact conforms_wellFormed (hwt.2 r hr).1
    · rw [hhe] at hv
      exact (hha.1.1.1 v hv).2
  refine ⟨⟨shadowOf_fits s₀, hinv, ?_, typed_refs hT hinv, refsTop_of_typed hT hinv,
    composes_of_typed hT⟩, hT, ?_, ?_⟩
  · intro x hx
    obtain ⟨o, ho, -, h2⟩ := mem_keys_iff.1 hx
    rw [← h2]
    have hff : framesFit (shadowOf s₀).frames s₀.frames = true := by
      have := shadowOf_fits s₀
      rw [shadow_fits, Bool.and_eq_true] at this
      exact this.1
    obtain ⟨fr', hfr', v, hv, hw⟩ := framesOcc_value hff ho
    have hva := hfv fr' hfr' v hv
    cases hov : o.2 with
    | ref r =>
      rw [hov] at hw
      cases v with
      | ref r' =>
        rw [subvalues_eq] at hw
        simp only [Value.children, List.flatMap_nil, List.mem_singleton, Value.ref.injEq] at hw
        subst hw
        obtain ⟨-, h3⟩ := hha.1.2 r (List.mem_filterMap.2 ⟨_, hva, rfl⟩)
        refine ⟨fun h => absurd h (by decide), fun a t hl => ?_⟩
        rw [hl] at h3
        simp at h3
      | _ =>
        have hval := hha.1.1.2 _ hva
        obtain ⟨t, -, hc⟩ := conformsAll_mem hconf _ hva
        have := shaped_noRef _ (conforms_shaped hc).1 hval _ hw
        simp [Value.isRef] at this
    | _ => exact container_ok (by intro r h; cases h)
  · rw [typeKeys_iff]
    intro r hr
    rw [hsto] at hr
    have hk := key_of_subset' (abilities_type r.value) (hwt.2 r hr).2
    rwa [conforms_type (hwt.2 r hr).1 rfl] at hk
  · intro x hx
    obtain ⟨v, hv, hw⟩ := occ_value (shadowOf_fits s₀) hx
    exact closureOk_subvalues hP (hwfv v hv) _ hw

end Invariant

/-! ## Steps keep the invariant -/

section Steps

variable {P : Program}

/-- A shadow fitting a state whose frames are not empty has a current
frame. -/
theorem shadow_head_fits {h : Shadow} {s : State} {fr : Frame} {rest : List Frame}
    (hf : h.fits s = true) (hsf : s.frames = fr :: rest) :
    ∃ g hs, h.frames = g :: hs ∧ g.fits fr = true ∧ framesFit hs rest = true := by
  obtain ⟨frames, hst, hhe, fresh⟩ := h
  rw [shadow_fits, hsf, Bool.and_eq_true] at hf
  cases frames with
  | nil => simp [framesFit] at hf
  | cons g hs =>
    rw [framesFit_cons, Bool.and_eq_true] at hf
    exact ⟨g, hs, rfl, hf.1.1, hf.1.2⟩

/-- What a step that keeps running pushes: a replacing instruction a value
holding no closure, a packing one a container that is no closure or, for
`PackClosure`, a closure passing D5a and D5b. -/
theorem top_ok (hP : AbilitiesSound P) {s s' : State} {h : Shadow} (hI : FVInv P s h)
    {fr : Frame} {rest : List Frame} {info : FunctionInfo} {code : CodeUnit} {instr : Bytecode}
    (hexec : stepOf (Step.exec P s fr rest info code instr) = .done (.running s')) :
    TypeKeys P s' ∧ ∀ v, Shadow.topAfter (.running s') = some v →
      (Replacing instr = true → ∀ w ∈ subvalues v, IsClosure w = false) ∧
      (Packing instr = true → IsClosure v = false ∨ FunctionValues.closureOk P v = true) := by
  by_cases hpk : (∃ h m, instr = .packClosure h m) ∨ ∃ i m, instr = .packClosureGeneric i m
  · obtain ⟨hrep, s₁, he, hstor, v, hv, hwfv⟩ := packClosure_exec hpk rfl hexec
    rw [Outcome.running.injEq] at he
    subst he
    refine ⟨keys_congr hI.keys hstor, fun v' hv' => ⟨fun hr => absurd hr (by rw [hrep]; decide),
      fun _ => Or.inr ?_⟩⟩
    rw [hv, Option.some.injEq] at hv'
    subst hv'
    exact closureOk_of_wellFormed hP hwfv
  · have hnc : ∀ h m, instr ≠ .packClosure h m := fun h m e => hpk (Or.inl ⟨h, m, e⟩)
    have hng : ∀ i m, instr ≠ .packClosureGeneric i m := fun i m e => hpk (Or.inr ⟨i, m, e⟩)
    obtain ⟨hkeys, htop⟩ := execOk_elim (exec_ok (rest := rest) (info := info) (code := code)
      hI.keys hnc hng) hexec rfl
    exact ⟨hkeys, fun v hv => ⟨(topFresh_elim htop hv).1,
      fun hp => Or.inl ((topFresh_elim htop hv).2 hp)⟩⟩

/-- **A step of the deterministic core that keeps running keeps the
invariant**, for a program the model accepts. -/
theorem fvInv_core (hP : AbilitiesSound P) {s s' : State}
    {h : Shadow} (hI : FVInv P s h) (hstep : step P s = .done (.running s')) :
    ∃ h', FVInv P s' h' := by
  obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hexec⟩ := step_parts hstep rfl
  obtain ⟨hkeys, htop⟩ := top_ok hP hI hexec
  obtain ⟨post, evs, hcs, hG⟩ := coreStep_annot (σ := false) (π := {}) (A := []) (auth := [])
    hI.pre hstep rfl (fun _ _ ho => by cases ho)
  cases post with
  | returned ts sts => exact absurd hG.fits (by simp [ShadowPost.fits])
  | running h' =>
  have hex : Shadow.exec P info.moduleId instr s fr (.running s') h = some (.running h', evs) := by
    simp only [Shadow.coreStep, hsf, hinfo, hcode, hinstr, Option.bind_eq_bind,
      Option.bind_some] at hcs
    cases hx : Shadow.exec P info.moduleId instr s fr (.running s') h with
    | none => rw [hx] at hcs; simp at hcs
    | some p =>
      rw [hx] at hcs
      obtain ⟨p1, p2⟩ := p
      simp only [Option.bind_some] at hcs
      split at hcs
      · simp only [Option.some.injEq] at hcs
        rw [hcs]
      · simp at hcs
  have hT := typed_core hI.typed hI.pre.inv hstep rfl
  have hinv : StateInv false P s' := hG.inv
  refine ⟨h', ⟨⟨hG.fits, hinv, ?_, typed_refs hT hinv, refsTop_of_typed hT hinv,
    composes_of_typed hT⟩, hT, hkeys, ?_⟩⟩
  · intro x hx
    obtain ⟨o, ho, -, h2⟩ := mem_keys_iff.1 hx
    rw [← h2]
    cases hw : o.2.isRef with
    | false => exact container_ok (not_ref_of_isRef hw)
    | true =>
      obtain ⟨d, w⟩ := o
      cases w with
      | ref r =>
        rcases hG.frameRefs d r (by simpa [postFrames] using ho) with hm | ⟨par, hb⟩
        · exact hI.pre.keysOk _ (mem_keys hm)
        · exact hG.issuedOk _ (List.mem_filterMap.2 ⟨_, hb, rfl⟩)
      | _ => simp [Value.isRef] at hw
  · intro x hx
    cases hc : IsClosure x.2 with
    | false => exact closureOk_of_not hc
    | true =>
      rcases hG.closuresKept x hx hc with hpre | ⟨c, he⟩
      · exact hI.good x hpre
      · obtain ⟨v, hv, hcr⟩ := exec_created _ hex x.1 x.2 c he
        have ht := htop v hv
        rcases hcr with ⟨hr, hw⟩ | ⟨hp, hw⟩
        · rw [ht.1 hr x.2 hw] at hc; cases hc
        · rcases ht.2 hp with ht2 | ht2
          · rw [hw, ht2] at hc; cases hc
          · rw [hw]; exact ht2

/-- The frames' occurrences after a native call: those before it, and those
of its results. -/
theorem native_framesOcc {g : ShadowFrame} {hs : List ShadowFrame} {fr : Frame}
    {rest : List Frame} {n : Nat} {vals : List Value} (hg : g.fits fr = true)
    {x : Nat × Value}
    (hx : x ∈ framesOcc ({ g with stack := vals.reverse.map treeOf ++ g.stack.drop n } :: hs)
      ({ fr with stack := vals.reverse ++ fr.stack.drop n, pc := fr.pc + 1 } :: rest)) :
    x ∈ framesOcc (g :: hs) (fr :: rest) ∨ ∃ v ∈ vals, x.2 ∈ subvalues v := by
  have hfa := frame_stack_fits hg
  have hlen : (vals.reverse.map treeOf).length = vals.reverse.length := by simp
  rw [framesOcc_cons, frame_occurrences, localsOcc_stack, occurrencesAll_append hlen] at hx
  rw [framesOcc_cons, frame_occurrences, stack_split hfa n]
  simp only [List.mem_append] at hx ⊢
  rcases hx with (hx | hx | hx) | hx
  · exact Or.inl (Or.inl (Or.inl hx))
  · obtain ⟨v, hv, hw⟩ := occurrencesAll_mem (fitsAll_map_treeOf _) hx
    exact Or.inr ⟨v, List.mem_reverse.1 hv, hw⟩
  · exact Or.inl (Or.inl (Or.inr (Or.inr hx)))
  · exact Or.inl (Or.inr hx)

/-! ## Values written back -/

theorem child_mem_children {v c : Value} {st : PathStep} (h : v.child? st = some c) :
    c ∈ v.children := by
  cases v <;> cases st <;> simp only [Value.child?] at h <;> try (simp at h; done)
  all_goals first
    | exact List.mem_of_getElem? h
    | exact List.mem_append_left _ (List.mem_of_getElem? h)
    | exact List.mem_append_right _ (List.mem_of_getElem? h)
    | (split at h
       · exact List.mem_of_getElem? h
       · cases h)
    | (simp only [Option.some.injEq] at h; subst h; simp [Value.children])

theorem withChild_children {P : Program} {v c v' : Value} {st : PathStep} (h : v.withChild? st c = some v') :
    (∀ c'' ∈ v'.children, c'' = c ∨ c'' ∈ v.children) ∧ v'.isRef = false ∧
      FunctionValues.closureOk P v' = true := by
  cases v <;> cases st <;> simp only [Value.withChild?] at h <;> try (simp at h; done)
  all_goals
    split at h
    · simp only [Option.some.injEq] at h
      subst h
      refine ⟨fun c'' hc'' => ?_, rfl, rfl⟩
      simp only [Value.children] at hc'' ⊢
      first
      | exact (List.mem_or_eq_of_mem_set hc'').symm
      | (rcases List.mem_append.1 hc'' with hc'' | hc''
         · rcases List.mem_or_eq_of_mem_set hc'' with h | h
           · exact Or.inr (List.mem_append_left _ h)
           · exact Or.inl h
         · exact Or.inr (List.mem_append_right _ hc''))
      | (rcases List.mem_append.1 hc'' with hc'' | hc''
         · exact Or.inr (List.mem_append_left _ hc'')
         · rcases List.mem_or_eq_of_mem_set hc'' with h | h
           · exact Or.inr (List.mem_append_right _ h)
           · exact Or.inl h)
    · cases h

/-- **The nodes of a value with a part replaced** are nodes of the value, of
the part, or containers along the path, which are no references and no
closures. -/
theorem subvalues_set {P : Program} :
    ∀ (p : List PathStep) {v w v' : Value}, v.set? p w = some v' →
    ∀ u ∈ subvalues v', u ∈ subvalues v ∨ u ∈ subvalues w ∨
      (u.isRef = false ∧ FunctionValues.closureOk P u = true)
  | [], v, w, v', h, u, hu => by
    simp only [Value.set?, Option.some.injEq] at h
    subst h
    exact Or.inr (Or.inl hu)
  | st :: p, v, w, v', h, u, hu => by
    simp only [Value.set?, bind, Option.bind_eq_some_iff] at h
    obtain ⟨c, hc, c', hc', hv'⟩ := h
    obtain ⟨hch, hnr, hok⟩ := withChild_children (P := P) hv'
    rw [subvalues_eq] at hu
    rcases List.mem_cons.1 hu with rfl | hu
    · exact Or.inr (Or.inr ⟨hnr, hok⟩)
    obtain ⟨c'', hc'', hu⟩ := List.mem_flatMap.1 hu
    rcases hch c'' hc'' with rfl | hc''
    · rcases subvalues_set p hc' u hu with hu | hu | hu
      · left
        rw [subvalues_eq]
        exact List.mem_cons_of_mem _ (List.mem_flatMap.2 ⟨c, child_mem_children hc, hu⟩)
      · exact Or.inr (Or.inl hu)
      · exact Or.inr (Or.inr hu)
    · left
      rw [subvalues_eq]
      exact List.mem_cons_of_mem _ (List.mem_flatMap.2 ⟨c'', hc'', hu⟩)

theorem frameVals_locals_set {fr : Frame} {i : Nat} {v w : Value}
    (hv : v ∈ frameVals { fr with locals := fr.locals.setIfInBounds i (some w) }) :
    v ∈ frameVals fr ∨ v = w := by
  simp only [frameVals, List.mem_append, List.mem_filterMap, id] at hv ⊢
  rcases hv with hv | ⟨o, ho, hoe⟩
  · exact Or.inl (Or.inl hv)
  · subst hoe
    obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 ho
    rw [Array.getElem?_toList] at hj
    by_cases hij : i = j
    · subst hij
      by_cases hl : i < fr.locals.size
      · simp [hl] at hj
        exact Or.inr hj.symm
      · simp [hl] at hj
    · rw [Array.getElem?_setIfInBounds_ne hij, ← Array.getElem?_toList] at hj
      exact Or.inl (Or.inr ⟨some v, List.mem_of_getElem? hj, rfl⟩)

/-- **The values after replacing a root's value** are values before, or the new
one; in the frames, values of the frames before, or the new one, at a local. -/
theorem stateVal_setRoot {s : State} {ρ : Root} {w v : Value}
    (hv : StateVal (s.setRoot ρ w) v) : StateVal s v ∨ v = w := by
  cases ρ with
  | local_ f i =>
    rcases hv with ⟨fr', hfr', hv⟩ | hv | hv
    · simp only [State.setRoot, List.mem_map] at hfr'
      obtain ⟨fr, hfr, rfl⟩ := hfr'
      split at hv
      · rcases frameVals_locals_set hv with hv | rfl
        · exact Or.inl (Or.inl ⟨fr, hfr, hv⟩)
        · exact Or.inr rfl
      · exact Or.inl (Or.inl ⟨fr, hfr, hv⟩)
    · exact Or.inl (Or.inr (Or.inl hv))
    · exact Or.inl (Or.inr (Or.inr hv))
  | global a t =>
    rcases hv with hv | ⟨r, hr, rfl⟩ | hv
    · exact Or.inl (Or.inl hv)
    · simp only [State.setRoot, GlobalStorage.update, List.mem_map] at hr
      obtain ⟨r0, hr0, rfl⟩ := hr
      split
      · exact Or.inr rfl
      · exact Or.inl (Or.inr (Or.inl ⟨r0, hr0, rfl⟩))
    · exact Or.inl (Or.inr (Or.inr hv))
  | held k =>
    rcases hv with hv | hv | hv
    · exact Or.inl (Or.inl hv)
    · exact Or.inl (Or.inr (Or.inl hv))
    · simp only [State.setRoot] at hv
      rcases List.mem_or_eq_of_mem_set hv with hv | rfl
      · exact Or.inl (Or.inr (Or.inr hv))
      · exact Or.inr rfl

theorem frameVal_setRoot {s : State} {ρ : Root} {w v : Value} {fr' : Frame}
    (hfr' : fr' ∈ (s.setRoot ρ w).frames) (hv : v ∈ frameVals fr') :
    (∃ fr ∈ s.frames, v ∈ frameVals fr) ∨ (v = w ∧ ∃ f i, ρ = .local_ f i) := by
  cases ρ with
  | local_ f i =>
    simp only [State.setRoot, List.mem_map] at hfr'
    obtain ⟨fr, hfr, rfl⟩ := hfr'
    split at hv
    · rcases frameVals_locals_set hv with hv | rfl
      · exact Or.inl ⟨fr, hfr, hv⟩
      · exact Or.inr ⟨rfl, f, i, rfl⟩
    · exact Or.inl ⟨fr, hfr, hv⟩
  | global a t => exact Or.inl ⟨fr', hfr', hv⟩
  | held k => exact Or.inl ⟨fr', hfr', hv⟩

theorem root_stateVal {s : State} {ρ : Root} {v : Value} (h : s.root? ρ = some v) :
    StateVal s v := by
  cases ρ with
  | local_ f i =>
    simp only [State.root?, bind, Option.bind_eq_some_iff] at h
    obtain ⟨fr, hfr, o, ho, rfl⟩ := h
    refine Or.inl ⟨fr, frame_mem hfr, ?_⟩
    simp only [frameVals, List.mem_append, List.mem_filterMap, id]
    exact Or.inr ⟨some v, by rw [← Array.getElem?_toList] at ho; exact List.mem_of_getElem? ho, rfl⟩
  | global a t =>
    simp only [State.root?] at h
    unfold GlobalStorage.get? at h
    rw [Option.map_eq_some_iff] at h
    obtain ⟨r, hr, rfl⟩ := h
    exact Or.inr (Or.inl ⟨r, List.mem_of_find?_eq_some hr, rfl⟩)
  | held k =>
    simp only [State.root?] at h
    exact Or.inr (Or.inr (List.mem_of_getElem? h))

theorem root_frameVal {s : State} {f : FrameId} {i : Nat} {v : Value}
    (h : s.root? (.local_ f i) = some v) : ∃ fr ∈ s.frames, v ∈ frameVals fr := by
  simp only [State.root?, bind, Option.bind_eq_some_iff] at h
  obtain ⟨fr, hfr, o, ho, rfl⟩ := h
  refine ⟨fr, frame_mem hfr, ?_⟩
  simp only [frameVals, List.mem_append, List.mem_filterMap, id]
  exact Or.inr ⟨some v, by rw [← Array.getElem?_toList] at ho; exact List.mem_of_getElem? ho, rfl⟩

/-- **The nodes of the values after writing back** are nodes of values before,
of a value written back, or containers along a written path. -/
theorem writeBack_stateVal {P : Program} :
    ∀ (refs : List Reference) (ws : List Value) {s s' : State},
    writeBack s refs ws = some s' → ∀ v, StateVal s' v → ∀ u ∈ subvalues v,
      (∃ v0, StateVal s v0 ∧ u ∈ subvalues v0) ∨ (∃ w ∈ ws, u ∈ subvalues w) ∨
        (u.isRef = false ∧ FunctionValues.closureOk P u = true)
  | [], [], s, s', h, v, hv, u, hu => by
    simp only [writeBack, Option.some.injEq] at h
    subst h
    exact Or.inl ⟨v, hv, hu⟩
  | r :: refs, w :: ws, s, s', h, v, hv, u, hu => by
    simp only [writeBack, bind, Option.bind_eq_some_iff] at h
    obtain ⟨s₂, hw, h⟩ := h
    simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hw
    obtain ⟨v0, hv0, v'', hset, rfl⟩ := hw
    rcases writeBack_stateVal refs ws h v hv u hu with ⟨v1, hv1, hu1⟩ | ⟨w', hw', hu'⟩ | hu'
    · rcases stateVal_setRoot (s := s) (ρ := r.location.root) hv1 with hv1 | rfl
      · exact Or.inl ⟨v1, hv1, hu1⟩
      · rcases subvalues_set (P := P) r.location.path hset u hu1 with hu1 | hu1 | hu1
        · exact Or.inl ⟨v0, root_stateVal hv0, hu1⟩
        · exact Or.inr (Or.inl ⟨w, List.mem_cons_self, hu1⟩)
        · exact Or.inr (Or.inr hu1)
    · exact Or.inr (Or.inl ⟨w', List.mem_cons_of_mem _ hw', hu'⟩)
    · exact Or.inr (Or.inr hu')
  | [], _ :: _, _, _, h, _, _, _, _ => by simp [writeBack] at h
  | _ :: _, [], _, _, h, _, _, _, _ => by simp [writeBack] at h

/-- **The references in the frames after writing back** were in them before,
when the values written back hold none. -/
theorem writeBack_frameRefs {P : Program} :
    ∀ (refs : List Reference) (ws : List Value) {s s' : State},
    writeBack s refs ws = some s' → (∀ w ∈ ws, ∀ u ∈ subvalues w, u.isRef = false) →
    ∀ fr' ∈ s'.frames, ∀ v ∈ frameVals fr', ∀ r, Value.ref r ∈ subvalues v →
      ∃ fr ∈ s.frames, ∃ v0 ∈ frameVals fr, Value.ref r ∈ subvalues v0
  | [], [], s, s', h, _, fr', hfr', v, hv, r, hr => by
    simp only [writeBack, Option.some.injEq] at h
    subst h
    exact ⟨fr', hfr', v, hv, hr⟩
  | x :: refs, w :: ws, s, s', h, hws, fr', hfr', v, hv, r, hr => by
    simp only [writeBack, bind, Option.bind_eq_some_iff] at h
    obtain ⟨s₂, hw, h⟩ := h
    simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hw
    obtain ⟨v0, hv0, v'', hset, rfl⟩ := hw
    obtain ⟨fr₁, hfr₁, v₁, hv₁, hr₁⟩ := writeBack_frameRefs (P := P) refs ws h
      (fun w' hw' => hws w' (List.mem_cons_of_mem _ hw')) fr' hfr' v hv r hr
    rcases frameVal_setRoot (s := s) hfr₁ hv₁ with ⟨fr, hfr, hv⟩ | ⟨rfl, f, i, hρ⟩
    · exact ⟨fr, hfr, v₁, hv, hr₁⟩
    · rcases subvalues_set (P := P) x.location.path hset _ hr₁ with hu | hu | ⟨hu, -⟩
      · rw [hρ] at hv0
        obtain ⟨fr, hfr, hv⟩ := root_frameVal hv0
        exact ⟨fr, hfr, v0, hv, hu⟩
      · exact absurd (hws w List.mem_cons_self _ hu) (by simp [Value.isRef])
      · simp [Value.isRef] at hu
  | [], _ :: _, _, _, h, _, _, _, _, _, _, _ => by simp [writeBack] at h
  | _ :: _, [], _, _, h, _, _, _, _, _, _, _ => by simp [writeBack] at h

theorem writeBack_storageKeys : ∀ (refs : List Reference) (ws : List Value) {s s' : State},
    writeBack s refs ws = some s' →
      s'.storage.map (fun r => (r.address, r.type)) = s.storage.map (fun r => (r.address, r.type))
  | [], [], s, s', h => by simp only [writeBack, Option.some.injEq] at h; rw [h]
  | r :: refs, w :: ws, s, s', h => by
    simp only [writeBack, bind, Option.bind_eq_some_iff] at h
    obtain ⟨s₂, hw, h⟩ := h
    simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hw
    obtain ⟨_, -, _, -, rfl⟩ := hw
    rw [writeBack_storageKeys refs ws h]
    exact setRoot_storageKeys _ _ _
  | [], _ :: _, _, _, h => by simp [writeBack] at h
  | _ :: _, [], _, _, h => by simp [writeBack] at h

theorem frameVal_occ {h : Shadow} {s : State} (hf : h.fits s = true) {fr : Frame}
    (hfr : fr ∈ s.frames) {v : Value} (hv : v ∈ frameVals fr) {w : Value} (hw : w ∈ subvalues v) :
    ∃ i, (i, w) ∈ framesOcc h.frames s.frames := by
  rw [shadow_fits, Bool.and_eq_true] at hf
  have hff := hf.1
  simp only [framesFit, Bool.and_eq_true, List.all_eq_true, beq_iff_eq] at hff
  obtain ⟨g, hp⟩ := exists_zip_left hff.1 hfr
  have hg := hff.2 _ hp
  dsimp only at hg
  have hsub : ∀ y ∈ g.occurrences fr, y ∈ framesOcc h.frames s.frames := fun y hy =>
    List.mem_flatMap.2 ⟨(g, fr), hp, hy⟩
  rw [frame_fits] at hg
  simp only [Bool.and_eq_true, beq_iff_eq] at hg
  simp only [frameVals, List.mem_append, List.mem_filterMap, id] at hv
  rcases hv with hv | ⟨o, ho, rfl⟩
  · obtain ⟨i, hi⟩ := mem_occurrencesAll hg.2 hv hw
    exact ⟨i, hsub _ (by rw [frame_occurrences]; exact List.mem_append_right _ hi)⟩
  · have hlen : g.locals.toList.length = fr.locals.toList.length := by simp [hg.1.1]
    obtain ⟨t, ht⟩ := exists_zip_left hlen ho
    have hs : ShadowFrame.slotFits t (some v) = true := by
      have := hg.1.2
      simp only [slotsFit, List.all_eq_true] at this
      exact this _ ht
    cases t with
    | none => simp [ShadowFrame.slotFits] at hs
    | some t =>
      simp only [ShadowFrame.slotFits] at hs
      have : w ∈ (t.occurrences v).map (·.2) := by rw [occurrences_snd t v hs]; exact hw
      obtain ⟨x, hx, rfl⟩ := List.mem_map.1 this
      refine ⟨x.1, hsub _ ?_⟩
      rw [frame_occurrences]
      exact List.mem_append_left _ (List.mem_flatMap.2 ⟨(some t, some v), ht, hx⟩)

/-- **A native call that returns and keeps running keeps the invariant**:
its results conform to its return types. -/
theorem fvInv_native (hP : AbilitiesSound P) {s s₁ s' : State} {h : Shadow} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} (hI : FVInv P s h)
    (hstep : step P s = .callNative s₁ f args views)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s') :
    ∃ h', FVInv P s' h' := by
  have hcont : (resumeNative P s₁ f args (.returned vals writes places)).continues = true := by
    rw [ho]; rfl
  rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
  case inr =>
    obtain ⟨fr, rest, n, results, hsf, hwb, hwr, hres⟩ := resume_decomp hshape hstep ho
    have hT := typed_native hI.typed hI.pre.inv hstep hcont
    rw [ho] at hT
    have hinv := inv_native hI.typed hI.pre.inv hshape hstep ho
    have hfr : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self ..
    -- The values before the writes: the results, or values of `s`.
    have hval₁ : ∀ v, StateVal { s with
        frames := { fr with
          stack := results.reverse ++ fr.stack.drop n
          pc := fr.pc + 1 } :: rest } v → StateVal s v ∨ v ∈ results := by
      rintro v (⟨fr', hfr', hv⟩ | hv | hv)
      · rcases List.mem_cons.1 hfr' with rfl | hfr'
        · simp only [frameVals, List.mem_append, List.mem_reverse] at hv
          rcases hv with (hv | hv) | hv
          · exact Or.inr hv
          · exact Or.inl (Or.inl ⟨fr, hfr, mem_frameVals_stack (List.mem_of_mem_drop hv)⟩)
          · exact Or.inl (Or.inl ⟨fr, hfr, by simp [frameVals, hv]⟩)
        · exact Or.inl (Or.inl ⟨fr', by rw [hsf]; exact List.mem_cons_of_mem _ hfr', hv⟩)
      · exact Or.inl (Or.inr (Or.inl hv))
      · exact Or.inl (Or.inr (Or.inr hv))
    have hgood₀ : ∀ v, StateVal s v → ∀ u ∈ subvalues v, FunctionValues.closureOk P u = true :=
      fun v hv u hu => by
        obtain ⟨i, hi⟩ := value_occ hI.pre.fits hv hu
        exact hI.good _ hi
    have hresOk : ∀ v ∈ results, ∀ u ∈ subvalues v, FunctionValues.closureOk P u = true := by
      rcases hres with hres | ⟨b, rfl, -, -⟩
      · intro v hv; exact closureOk_subvalues hP (hres v hv).1
      · intro v hv u hu
        simp only [List.mem_singleton] at hv
        subst hv
        rw [subvalues_eq] at hu
        simp [Value.children] at hu
        subst hu
        rfl
    refine ⟨shadowOf s', ⟨⟨shadowOf_fits s', hinv, ?_, typed_refs hT hinv, refsTop_of_typed hT hinv,
      composes_of_typed hT⟩, hT, ?_, ?_⟩⟩
    · -- The keys of the references in the frames.
      intro x hx
      obtain ⟨o, ho', -, h2⟩ := mem_keys_iff.1 hx
      rw [← h2]
      cases hw : o.2.isRef with
      | false => exact container_ok (not_ref_of_isRef hw)
      | true =>
        obtain ⟨d, w⟩ := o
        cases w with
        | ref r =>
          have hff := shadowOf_fits s'
          rw [shadow_fits, Bool.and_eq_true] at hff
          obtain ⟨fr', hfr', v, hv, hrv⟩ := framesOcc_value hff.1 ho'
          obtain ⟨fr₁, hfr₁, v₁, hv₁, hr₁⟩ := writeBack_frameRefs (P := P) _ writes hwb
            (fun w hw => (hwr w hw).2) fr' hfr' v hv r hrv
          have hold : ∀ fr₀ ∈ s.frames, ∀ v₀ ∈ frameVals fr₀, Value.ref r ∈ subvalues v₀ →
              Key.Ok false (Value.ref r).key := by
            intro fr₀ hfr₀ v₀ hv₀ hr₀
            obtain ⟨i, hi⟩ := frameVal_occ hI.pre.fits hfr₀ hv₀ hr₀
            exact hI.pre.keysOk _ (mem_keys hi)
          rcases List.mem_cons.1 hfr₁ with rfl | hfr₁
          · simp only [frameVals, List.mem_append, List.mem_reverse] at hv₁
            rcases hv₁ with (hv₁ | hv₁) | hv₁
            · rcases hres with hres | ⟨b, rfl, hp0, -⟩
              · have := (hres v₁ hv₁).2 _ hr₁
                simp [Value.isRef] at this
              · simp only [List.mem_singleton] at hv₁
                subst hv₁
                rw [subvalues_eq] at hr₁
                simp [Value.children] at hr₁
                subst hr₁
                show Key.Ok false (.leaf (.ref _))
                refine ⟨fun h => (by cases h), fun a t hl => ?_⟩
                rw [hl] at hp0
                exact absurd rfl hp0
            · exact hold fr hfr v₁ (mem_frameVals_stack (List.mem_of_mem_drop hv₁)) hr₁
            · exact hold fr hfr v₁ (by simp [frameVals, hv₁]) hr₁
          · exact hold fr₁ (by rw [hsf]; exact List.mem_cons_of_mem _ hfr₁) v₁ hv₁ hr₁
        | _ => simp [Value.isRef] at hw
    · -- Resources keep their types.
      have hk := hI.keys
      rw [typeKeys_iff] at hk ⊢
      intro r hr
      have hm : (r.address, r.type) ∈ s'.storage.map (fun r => (r.address, r.type)) :=
        List.mem_map_of_mem hr
      rw [writeBack_storageKeys _ writes hwb] at hm
      obtain ⟨r0, hr0, he⟩ := List.mem_map.1 hm
      simp only [Prod.mk.injEq] at he
      rw [← he.2]
      exact hk r0 hr0
    · -- D5a and D5b at every occurrence.
      intro x hx
      obtain ⟨v, hv, hxv⟩ := occ_value (shadowOf_fits s') hx
      rcases writeBack_stateVal (P := P) _ writes hwb v hv x.2 hxv with ⟨v0, hv0, hu⟩ |
        ⟨w, hw, hu⟩ | ⟨-, hok⟩
      · rcases hval₁ v0 hv0 with hv0 | hv0
        · exact hgood₀ v0 hv0 _ hu
        · exact hresOk v0 hv0 _ hu
      · exact closureOk_subvalues hP (hwr w hw).1 _ hu
      · exact hok
  have hvals := native_values hstep hshape hcont
  have hT := typed_native hI.typed hI.pre.inv hstep hcont
  rw [ho] at hT
  obtain ⟨fr, rest, n, hsf, -, -, rfl, hfacts⟩ := step_callNative hstep
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  have hv := hfacts.values hshape
  obtain ⟨hs', ⟨tys, -, hconf⟩, -, -⟩ := resume_running (fr := { fr with stack := fr.stack.drop n })
    (rest := rest) rfl hshape (noRef_of_conformsAll hconfa (hv.1 _ hpt)) hv.2 hcont
  rw [ho, Outcome.running.injEq] at hs'
  subst hs'
  obtain ⟨g, hs, hh, hg, hrest⟩ := shadow_head_fits hI.pre.fits hsf
  obtain ⟨frames, hst, hhe, fresh⟩ := h
  simp only at hh
  subst hh
  have hfa := frame_stack_fits hg
  have hfit : Shadow.fits
      ⟨{ g with stack := vals.reverse.map treeOf ++ g.stack.drop n } :: hs, hst, hhe, fresh⟩
      { s with frames := { fr with stack := vals.reverse ++ fr.stack.drop n, pc := fr.pc + 1 } ::
        rest } = true := by
    have hf := hI.pre.fits
    rw [shadow_fits, hsf, Bool.and_eq_true] at hf
    rw [shadow_fits, Bool.and_eq_true, framesFit_cons, Bool.and_eq_true]
    refine ⟨⟨frame_fits_stack hg ?_ _, hrest⟩, by simpa [extFit] using hf.2⟩
    rw [fitsAll_append (by simp), fitsAll_map_treeOf, (fitsAll_take_drop n hfa).2]
    rfl
  have hinv : StateInv false P { s with frames :=
      { fr with stack := vals.reverse ++ fr.stack.drop n, pc := fr.pc + 1 } :: rest } :=
    frames_inv hI.pre.inv hsf rfl rfl rfl rfl
  have hpre_occ : ∀ x ∈ framesOcc (g :: hs) (fr :: rest), x ∈ framesOcc (g :: hs) s.frames := by
    intro x hx; rw [hsf]; exact hx
  refine ⟨_, ⟨⟨hfit, hinv, ?_, typed_refs hT hinv, refsTop_of_typed hT hinv, composes_of_typed hT⟩,
    hT, keys_congr hI.keys rfl, ?_⟩⟩
  · intro x hx
    obtain ⟨o, ho, -, h2⟩ := mem_keys_iff.1 hx
    rw [← h2]
    rcases native_framesOcc hg ho with hm | ⟨v, hv, hw⟩
    · exact hI.pre.keysOk _ (mem_keys (hpre_occ _ hm))
    · have := shaped_noRef v (hvals v hv).1 (hvals v hv).2 _ hw
      exact container_ok (not_ref_of_isRef this)
  · intro x hx
    rw [shadow_occurrences] at hx
    rcases List.mem_append.1 hx with hx | hx
    · rcases native_framesOcc hg hx with hm | ⟨v, hv, hw⟩
      · exact hI.good x (by rw [shadow_occurrences]; exact List.mem_append_left _ (hpre_occ _ hm))
      · obtain ⟨t, -, hc⟩ := conformsAll_mem hconf v hv
        exact closureOk_subvalues hP (conforms_wellFormed hc) _ hw
    · exact hI.good x (by
        rw [shadow_occurrences]
        exact List.mem_append_right _ (by simpa [extOcc] using hx))

/-- **D5a, D5b and D5d after a step that returns**: it returns values of the
current segment and commits global storage unchanged. -/
theorem returned_ok (hP : AbilitiesSound P) {s : State} {h : Shadow} (hI : FVInv P s h) {vs : List Value}
    {st : GlobalStorage} (hstep : step P s = .done (.returned vs st)) :
    FunctionValues.outcomeOk P (.returned vs st) = true := by
  obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hexec⟩ := step_parts hstep rfl
  have hnc : ∀ h m, instr ≠ .packClosure h m := fun h m e =>
    absurd (packClosure_exec (Or.inl ⟨h, m, e⟩) rfl hexec).2 (by simp)
  have hng : ∀ i m, instr ≠ .packClosureGeneric i m := fun i m e =>
    absurd (packClosure_exec (Or.inr ⟨i, m, e⟩) rfl hexec).2 (by simp)
  obtain ⟨hvs, rfl⟩ := execOk_elim (exec_ok (rest := rest) (info := info) (code := code)
    hI.keys hnc hng) hexec rfl
  have hok := stateOk_of hP hI
  simp only [FunctionValues.stateOk, Bool.and_eq_true] at hok
  simp only [FunctionValues.outcomeOk, Bool.and_eq_true]
  refine ⟨⟨valuesOk_of fun v hv => hI.closureOk (Or.inl ⟨fr, ?_, ?_⟩), hok.1.2⟩, hok.2⟩
  · rw [hsf]; exact List.mem_cons_self ..
  · exact mem_frameVals_stack (hvs v hv)

/-- A native call never returns from the transaction. -/
theorem native_ne_returned {s₁ : State} {f : FunctionRef} {args : List Value} {r : NativeResult}
    {vs : List Value} {st : GlobalStorage} : resumeNative P s₁ f args r ≠ .returned vs st := by
  intro h
  cases r with
  | failed e => simp [resumeNative] at h
  | returned vals writes places =>
    simp only [resumeNative] at h
    repeat' split at h
    all_goals simp at h

/-- **D5c**: a `CallClosure` that continues enters the closure's target, at
its first instruction, with the captured and the provided values as its
first locals, conforming to its parameter types (`callClosure_cont`); it
never calls a native. -/
theorem callEntersOk_of {N : NativeRel} {s : State} {o : Outcome} (ho : StepRel P N s o) :
    FunctionValues.callEntersOk P s o = true := by
  unfold FunctionValues.callEntersOk
  split
  · rfl
  · next fr frs hsf =>
    split
    · next sig hci =>
      unfold currentInstr at hci
      simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at hci
      obtain ⟨info, hinfo, code, hcode, hinstr⟩ := hci
      have hst : step P s = stepOf (Step.callClosure P s fr frs info sig) := by
        simp only [step, hsf, hinfo, hcode, hinstr]
        rfl
      cases ho with
      | native hstep _ =>
        rw [hst] at hstep
        exfalso
        cases hx : Step.callClosure P s fr frs info sig with
        | ok r =>
          rw [hx] at hstep
          simp only [stepOf] at hstep
          subst hstep
          exact notNative_absurd (by unfold Step.callClosure; not_native) hx
        | error o => rw [hx] at hstep; simp [stepOf] at hstep
      | core hstep =>
        rw [hst] at hstep
        cases hc : o.continues with
        | false =>
          cases o <;> simp only [Outcome.continues, Bool.true_eq_false] at hc <;>
            split <;> simp_all
        | true =>
          obtain ⟨f, mask, captured, cps, crs, ca, st, s', callee, frs', tys, hstk, rfl, -, hcs',
            hpt, hfn, hpc, hconf, hloc⟩ := callClosure_cont hc hstep
          simp only [hstk, hcs', hpt, hfn, hpc, hconf, hloc, beq_self_eq_true, Bool.and_self]
    · rfl

/-- **Every reachable state satisfies the invariant**, for a program the
model accepts. -/
theorem fvInv_reachable (hP : AbilitiesSound P) {N : NativeRel} {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    ∀ s, Reachable P N s₀ s → ∃ h, FVInv P s h := by
  intro s hr
  induction hr with
  | refl => exact ⟨_, fvInv_initial hP h0⟩
  | @step s s' _ hst ih =>
    obtain ⟨h, hI⟩ := ih
    generalize ho : Outcome.running s' = o at hst
    cases hst with
    | core hstep =>
      subst ho
      exact fvInv_core hP hI hstep
    | @native s₁ f args views r hstep _ =>
      cases r with
      | failed e => simp [resumeNative] at ho
      | returned vals writes places => exact fvInv_native hP hI hstep ho.symm

end Steps

/-! ## D5 -/

/-- **D5**: in every execution of a program the model accepts, from an
admissible initial state, every closure captures no reference and is well
formed, with abilities justified by its target and its captured values,
every closure in global storage targets a storable function, and every
`CallClosure` that continues enters its target with exactly its parameters. -/
theorem functionValueSafety : FunctionValueSafety Model.verifier := by
  intro P N s₀ hwf _ _ _ h0 s hr
  have hP := abilitiesSound_gen hwf
  obtain ⟨h, hI⟩ := fvInv_reachable hP h0 s hr
  refine ⟨stateOk_of hP hI, fun o ho => ⟨?_, callEntersOk_of ho⟩⟩
  cases ho with
  | @core o hstep =>
    cases o with
    | running s' =>
      obtain ⟨h', hI'⟩ := fvInv_core hP hI hstep
      exact stateOk_of hP hI'
    | returned vs st => exact returned_ok hP hI hstep
    | failed _ => rfl
    | wrong _ => rfl
  | @native s₁ f args views r hstep _ =>
    cases hr' : resumeNative P s₁ f args r with
    | running s' =>
      cases r with
      | failed e => simp [resumeNative] at hr'
      | returned vals writes places =>
        obtain ⟨h', hI'⟩ := fvInv_native hP hI hstep hr'
        exact stateOk_of hP hI'
    | returned vs st => exact absurd hr' native_ne_returned
    | failed _ => rfl
    | wrong _ => rfl

end MoveBytecode.Proofs
