-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ModeledOwnership

/-!
# Every call of a modeled native is annotated

UNTRUSTED (checked by the kernel). Under the native contract
(`NativesRespectOwnership`), a call of a modeled native that returns and
continues has an annotation (`Shadow.nativeStep`), and the annotation
establishes `StepGhost` (`nativeAnnotated_modeled`): every part of the
annotation exists (`nativeStep_of`), its effect is the contract's on the
nodes the native sees (`native_effectK`), what it writes back overlaps no
other argument (`writes_perm`), and it creates as the contract lets it. With
the natives that take values only (`Proofs.GhostNative`), every native call is
annotated (`nativeAnnotated`), and the ghost layer accounts for every step
(`shadowAccounts_admissible`).
-/

namespace MoveBytecode.Proofs

theorem writeBack_length : ∀ {refs : List Reference} {ws : List Value} {s s' : State},
    writeBack s refs ws = some s' → refs.length = ws.length
  | [], [], _, _, _ => rfl
  | _ :: refs, _ :: ws, s, s', h => by
    simp only [writeBack, bind, Option.bind_eq_some_iff] at h
    obtain ⟨_, -, h⟩ := h
    simp [writeBack_length h]
  | [], _ :: _, _, _, h => by simp [writeBack] at h
  | _ :: _, [], _, _, h => by simp [writeBack] at h

/-- A consumption of a native: of a node it sees of an argument it does not
share, which it does not pass on. -/
theorem consumedOfNative_mem' {P : Program} {f : FunctionRef} {m : ModuleId}
    {ptys : List RuntimeType} {seen : List IdTree} {views : List Value}
    {origins : List (Nat × Path)} {e : Event}
    (he : e ∈ consumedOfNative P f m ptys seen views origins) :
    ∃ k q i w, e = .consumed i w (.native f m (keepsNothingInside origins k q)) ∧
      sharedAt P f ptys k = false ∧ origins.contains (k, q) = false ∧ (k, q, w) ∈ nodesOf views := by
  simp only [consumedOfNative, List.mem_flatMap] at he
  obtain ⟨⟨⟨t, a⟩, k⟩, hx, he⟩ := he
  rw [List.mem_zipIdx_iff_getElem?, List.getElem?_zip_eq_some] at hx
  simp only at he
  split at he
  · simp at he
  · next hsh =>
    obtain ⟨y, hy, hye⟩ := List.mem_filterMap.1 he
    split at hye
    · cases hye
    · next hc =>
      simp only [Option.some.injEq] at hye
      refine ⟨k, y.1.1, y.2.1, y.1.2, hye.symm, by simpa [sharedAt] using hsh,
        by simpa using hc, ?_⟩
      unfold nodesOf
      refine List.mem_flatMap.2 ⟨(a, k), List.mem_zipIdx_iff_getElem?.2 hx.2, ?_⟩
      exact List.mem_map.2 ⟨y.1, (List.of_mem_zip hy).1, rfl⟩

/-- The key of a reference on the current frame's segment satisfies the key
invariant. -/
theorem arg_key_ok {σ : Bool} {P : Program} {s : State} {h : Shadow} {fr : Frame}
    {rest : List Frame} {g : ShadowFrame} {hs : List ShadowFrame} (hp : Pre σ P s h)
    (hsf : s.frames = fr :: rest) (hh : h.frames = g :: hs) {r : Reference}
    (hr : Value.ref r ∈ fr.stack) : (Value.ref r).key.Ok σ := by
  have hg : g.fits fr = true := by
    have := hp.fits
    rw [shadow_fits, hsf, hh, framesFit_cons] at this
    simp only [Bool.and_eq_true] at this
    exact this.1.1
  obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hr
  obtain ⟨t, htk, ht⟩ := fitsAll_get (frame_stack_fits hg) hk
  have hmem : (t.id, Value.ref r) ∈ framesOcc h.frames s.frames := by
    rw [hh, hsf, framesOcc_cons, frame_occurrences]
    have h1 : (t.id, Value.ref r) ∈ t.occurrences (.ref r) := by rw [ref_occ ht]; simp
    have h2 := occurrencesAll_eq_flatMap g.stack fr.stack
    refine List.mem_append_left _ (List.mem_append_right _ ?_)
    rw [h2]
    exact List.mem_flatMap.2 ⟨(t, .ref r), List.mem_of_getElem? (zip_getElem? htk hk), h1⟩
  exact hp.keysOk _ (mem_keys hmem)

/-- What a reference argument designates is shared exactly when the native
does not write it back. -/
theorem sharedAt_ref {P : Program} {f : FunctionRef} {args : List Value}
    {ptys : List RuntimeType} (hc : conformsAll P args ptys = true) {k : Nat} {r : Reference}
    (ha : args[k]? = some (.ref r)) :
    sharedAt P f ptys k = !(r.mutable && P.nativeShape? f == some .update) := by
  obtain ⟨t, ht, hct⟩ := conformsAll_get hc ha
  unfold sharedAt
  rw [ht]
  rcases assignable_cases (conforms_assignable hct) with ⟨_, _, _, _, he, -⟩ | ⟨v, w, he, rfl, -⟩ |
    he
  · simp [Value.type] at he
  · simp only [Value.type, RuntimeType.ref.injEq] at he
    simp [Program.nativeKeeps, he.1]
  · simp only [Value.type] at he
    subst he
    rfl

/-- A value argument is the native's. -/
theorem sharedAt_val {P : Program} {f : FunctionRef} {args : List Value}
    {ptys : List RuntimeType} (hc : conformsAll P args ptys = true) {k : Nat} {a : Value}
    (ha : args[k]? = some a) (hna : ∀ r, a ≠ .ref r) : sharedAt P f ptys k = false := by
  obtain ⟨t, ht, hct⟩ := conformsAll_get hc ha
  unfold sharedAt
  rw [ht]
  rcases assignable_cases (conforms_assignable hct) with ⟨_, _, _, _, -, rfl, -⟩ |
    ⟨v, w, he, rfl, -⟩ | he
  · rfl
  · cases a <;> simp [Value.type] at he
    exact absurd rfl (hna _)
  · subst he
    cases a <;> simp [Value.type, Program.nativeKeeps]
    exact absurd rfl (hna _)

theorem filterMap_none {α β : Type} (l : List α) : l.filterMap (fun _ => (none : Option β)) = [] := by
  induction l <;> simp_all

theorem issuedOf_nativeEvents (refArgs written : List (Nat × Reference))
    (newRefs : List (Nat × Reference × Nat)) (consumed created : List Event) :
    issuedOf (nativeEventsOf refArgs written newRefs consumed created) =
      newRefs.map (fun x => (x.1, (Value.ref x.2.1).key)) ++ issuedOf consumed ++
        issuedOf created := by
  simp [nativeEventsOf, issuedOf, List.filterMap_map, Function.comp_def, filterMap_none]

theorem consumedOf_nativeEvents (refArgs written : List (Nat × Reference))
    (newRefs : List (Nat × Reference × Nat)) (consumed created : List Event) :
    consumedOf (nativeEventsOf refArgs written newRefs consumed created) =
      consumedOf consumed ++ consumedOf created := by
  simp [nativeEventsOf, consumedOf, List.filterMap_map, Function.comp_def, filterMap_none]

theorem nativeAnnotated_modeled {σ : Bool} {P : Program} {N : NativeRel} {ν : NativeFlows}
    {π : NativePolicy} {A : List StructId} {auth : List Address}
    (hown : NativesRespectOwnership P N ν π) {s s₁ : State} {h : Shadow}
    {f : FunctionRef} {args views vals writes : List Value} {places : List NativePlace}
    (hp : Pre σ P s h) (ht : StateTyped P s) (hstep : step P s = .callNative s₁ f args views)
    (hN : N f views (.returned vals writes places))
    (hcont : (resumeNative P s₁ f args (.returned vals writes places)).continues = true)
    {k : NativeShape} (hshape : P.nativeShape? f = some k) :
    ∃ post evs, Shadow.nativeStep P ν s f args views vals writes places
        (resumeNative P s₁ f args (.returned vals writes places)) h = some (post, evs) ∧
      StepGhost σ P π A auth (h.occurrences s) (framesOcc h.frames s.frames)
        (h.frames.zip s.frames) h.fresh (resumeNative P s₁ f args (.returned vals writes places))
        post evs := by
  obtain ⟨fr, rest, n, hsf, hnl, hargs, hs₁, hfacts⟩ := step_callNative hstep
  obtain ⟨tys, views', sW, results, frW, restW, htys, hviews, hfits, hwb, hres, hsW, ho⟩ :=
    resume_spec hcont
  obtain ⟨s', hs'e⟩ : ∃ s', s' = { sW with
      frames := { frW with stack := results.reverse ++ frW.stack, pc := frW.pc + 1 } :: restW } :=
    ⟨_, rfl⟩
  rw [← hs'e] at ho
  rw [ho]
  rw [hfacts.view, Option.some.injEq] at hviews
  subst hviews
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  obtain ⟨L, restW', hsWf, htop⟩ := writeBack_top _ _ (s := s₁) (by rw [hs₁]) hwb
  rw [hsW, List.cons.injEq] at hsWf
  obtain ⟨rfl, rfl⟩ := hsWf
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  -- The state after the results are pushed, before the values are written back.
  have hwb' : writeBack
      { s with
        frames := { fr with
          stack := results.reverse ++ fr.stack.drop n
          pc := fr.pc + 1 } :: rest }
      (P.writtenReferences f args) writes = some s' := by
    rw [hs'e]
    subst hs₁
    exact htop _ _
  have hfit := hp.fits
  cases hh : h.frames with
  | nil => rw [shadow_fits, hsf, hh] at hfit; simp [framesFit] at hfit
  | cons g hs =>
  have hfit' := hfit
  rw [shadow_fits, hsf, hh, framesFit_cons] at hfit'
  simp only [Bool.and_eq_true] at hfit'
  have hg : g.fits fr = true := hfit'.1.1
  have hfa := frame_stack_fits hg
  have hlenG := fitsAll_length hfa
  obtain ⟨seen, hseen, hsl, hvl, hidl, hargfit, hsfits, hsval, hsref⟩ := seen_spec hp hstep hh
  -- The native contract.
  obtain ⟨info, hinfo, hresp⟩ := hown f views vals writes places hN
  generalize hφ : ν f views (vals ++ writes) = φ at hresp
  simp only [NativeCallRespects, Bool.and_eq_true, List.all_eq_true] at hresp
  obtain ⟨H1, H2⟩ := hresp
  have hhonest : ∀ x ∈ nodesOf (vals ++ writes), ∀ k q, φ x.1 x.2.1 = .arg k q →
      views[k]?.bind (·.at? q) = some x.2.2 := by
    intro x hx k q hxq
    have := H1 x hx
    obtain ⟨j, p, v⟩ := x
    simp only at hxq this ⊢
    rw [hxq] at this
    simpa using this
  have hcreate : ∀ x ∈ nodesOf (vals ++ writes), φ x.1 x.2.1 = .fresh →
      (Creator.native f info.moduleId views).Permits π [] [] x.2.2 = true := by
    intro x hx hxq
    have := H1 x hx
    obtain ⟨j, p, v⟩ := x
    simp only at hxq this ⊢
    rw [hxq] at this
    exact this
  have hguard' : ∀ x ∈ nodesOf (vals ++ writes), ∀ k q, φ x.1 x.2.1 = .arg k q →
      (argIdAt seen k q).isSome = true := by
    intro x hx k q hxq
    have hh' := hhonest x hx k q hxq
    cases hk : views[k]? with
    | none => rw [hk] at hh'; simp at hh'
    | some w =>
      rw [hk, Option.bind_some] at hh'
      have hkl : k < seen.length := by have := (List.getElem?_eq_some_iff.1 hk).1; omega
      obtain ⟨st, hst⟩ : ∃ st, seen[k]? = some st := ⟨_, List.getElem?_eq_getElem hkl⟩
      obtain ⟨t', ht', -⟩ := at_fits q (hsfits k st w hst hk) hh'
      simp [argIdAt, hst, ht']
  have hguard : ((nodesOf (vals ++ writes)).all fun x =>
      match ν f views (vals ++ writes) x.1 x.2.1 with
      | .arg k q => (seenIdAt seen k q).isSome
      | .fresh => true) = true := by
    rw [hφ, List.all_eq_true]
    intro x hx
    cases hxq : φ x.1 x.2.1 with
    | arg k q => simpa [seenIdAt, argIdAt] using hguard' x hx k q hxq
    | fresh => rfl
  -- The trees of the outputs.
  obtain ⟨ts, F, evsO, hfoldO, hfitO, hpermO, hcO, hbO, hndO, hleO, hevO⟩ :=
    results_fold (resultIdOf φ seen) (.native f info.moduleId views) (vals ++ writes).zipIdx []
      h.fresh []
  rw [List.zipIdx_map_fst] at hfitO hpermO
  have hOut : outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
      (vals ++ writes) h.fresh = (ts, F, evsO) := by
    rw [hφ, outTreesOf_eq, hfoldO]; simp
  have hpassed : (IdTree.occurrencesAll ts (vals ++ writes)).Perm
      ((passedOn φ (vals ++ writes)).map (argOcc seen views) ++ createdOf evsO) := by
    rw [← passed_eq hguard' hhonest]; exact hpermO
  obtain ⟨hfv, hfw⟩ := fitsAll_take_drop vals.length hfitO
  rw [List.take_left] at hfv
  rw [List.drop_left] at hfw
  have hsplitO := stack_split hfitO vals.length
  rw [List.take_left, List.drop_left] at hsplitO
  have hτ := typeArgsOk_values hfacts.typeArgs
  have hwr : ∀ w ∈ writes, ∀ u ∈ subvalues w, u.isRef = false := by
    by_cases hku : k = .update
    · subst hku
      have hfit' := hfits
      simp only [Program.nativeResultFits, hshape, hpt, Bool.and_eq_true] at hfit'
      exact noRef_of_values hfit'.1.2 (mutableTargets_values hshape hpt hτ)
    · rw [fits_writes_nil hshape hku hfits]; simp
  -- The results, and the references made for the places.
  obtain ⟨resultTrees, newRefs, hrlen, hrr, hnew, hrt, hfitRes, hpostRes, hnewIds, hclRes, hokRes,
      hvr⟩ : ∃ (resultTrees : List IdTree) (newRefs : List (Nat × Reference × Nat)),
      results.length = tys.length ∧
      (refResultsOf tys results).length = places.length ∧
      newRefsOf (refResultsOf tys results) places (g.stack.take args.length).reverse F =
        some newRefs ∧
      nativeResultsIn tys (ts.take vals.length) (newRefs.map fun (i, _, _) => IdTree.node i []) =
        some resultTrees ∧
      IdTree.fitsAll resultTrees results = true ∧
      (keys (IdTree.occurrencesAll resultTrees results) ++
          keys (IdTree.occurrencesAll (ts.drop vals.length) writes)).Perm
        (keys (IdTree.occurrencesAll ts (vals ++ writes)) ++
          newRefs.map fun x => (x.1, (Value.ref x.2.1).key)) ∧
      (∀ i x, newRefs[i]? = some x → x.1 = F + i) ∧
      (∀ x ∈ IdTree.occurrencesAll resultTrees results, IsClosure x.2 = true →
        x ∈ IdTree.occurrencesAll ts (vals ++ writes)) ∧
      (∀ x ∈ newRefs, ((Value.ref x.2.1).key).Ok σ) ∧
      (∀ v ∈ vals, ∀ u ∈ subvalues v, u.isRef = false) := by
    by_cases hkb : k = .borrow
    · subst hkb
      obtain ⟨rfl, rfl, m, am, X, tgt, p, rfl, rfl, harg, hpX, hmam, -, hXt, hplace, -, -⟩ :=
        borrow_result hshape hpt htys (typeArgsOk_values hfacts.typeArgs) hfits
      obtain ⟨r0, hra, hr0m, -⟩ := conformsAll_ref_at hconfa hpX
      have hpr : (placeReferences sW [.ref m tgt] args [p]).bind (nativeResultsIn [.ref m tgt] []) =
          some [Step.borrow sW m tgt (r0.location.descend p.path)] := by
        simp [placeReferences, nativeResultsIn, harg, hra, RuntimeType.isValue]
      rw [hpr, Option.some.injEq] at hres
      subst hres
      have hts : ts = [] := by
        cases ts with
        | nil => rfl
        | cons => simp [IdTree.fitsAll] at hfitO
      subst hts
      have hn0 : 0 < args.length := (List.getElem?_eq_some_iff.1 hra).1
      obtain ⟨t0, ht0⟩ : ∃ t0, (g.stack.take args.length).reverse[0]? = some t0 :=
        ⟨_, List.getElem?_eq_getElem (by omega)⟩
      have hp0 : p.path ≠ [] := by
        intro hnil
        rw [hnil, placeType?_nil, Option.some.injEq] at hplace
        subst hplace
        exact borrow_ne hXt rfl
      -- The argument's key, from the invariant.
      have hok0 : (Value.ref r0).key.Ok σ := by
        obtain ⟨t', ht', hft'⟩ := fitsAll_getElem?_right hargfit hra
        rw [ht0, Option.some.injEq] at ht'
        subst ht'
        have hz : (t0, Value.ref r0) ∈ g.stack.zip fr.stack := by
          have := zip_of_getElem? ht0 hra
          rw [hargl] at this
          rw [hargs] at this
          exact zip_rev_take hlenG this
        have hmem : (t0.id, Value.ref r0) ∈ framesOcc h.frames s.frames := by
          rw [hh, hsf, framesOcc_cons, frame_occurrences]
          refine List.mem_append_left _ (List.mem_append_right _ ?_)
          exact occ_of_zip hz _ (by rw [ref_occ hft']; simp)
        exact hp.keysOk _ (mem_keys hmem)
      refine ⟨[.node F []], [(F, ⟨m, tgt, r0.location.descend p.path, sW.invalidations.length⟩,
        t0.id)], rfl, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, by simp⟩
      · simp [refResultsOf, Step.borrow, RuntimeType.isValue]
      · simp [refResultsOf, newRefsOf, Step.borrow, RuntimeType.isValue, harg, ht0]
      · simp [nativeResultsIn, RuntimeType.isValue]
      · simp [IdTree.fitsAll, IdTree.fits, Value.children, Step.borrow]
      · simp [IdTree.occurrencesAll, Step.borrow]
      · intro i x hx
        cases i with
        | zero => simp at hx; rw [← hx]; rfl
        | succ i => simp at hx
      · intro x hx hc
        simp [IdTree.occurrencesAll, Step.borrow] at hx
        rw [hx] at hc
        simp [IsClosure] at hc
      · intro x hx
        simp only [List.mem_singleton] at hx
        subst hx
        simp only [Value.key, Key.Ok, Location.descend] at hok0 ⊢
        refine ⟨fun hσ hheld => ?_, fun a ty hl => ?_⟩
        · cases hm : m
          · rfl
          · have := hok0.1 hσ hheld
            rw [hr0m, hmam hm] at this
            cases this
        · simp only [Location.mk.injEq] at hl
          exact absurd (List.append_eq_nil_iff.1 hl.2).2 hp0
    · obtain ⟨hcv, rfl⟩ := fits_values hshape hkb hfits
      have hvt := returns_values hshape hkb htys hτ
      simp only [placeReferences_nil, Option.bind_some,
        nativeResultsIn_values hvt (conformsAll_length hcv), Option.some.injEq] at hres
      subst hres
      have hrl : vals.length = tys.length := conformsAll_length hcv
      have hnoref : refResultsOf tys vals = [] := by
        unfold refResultsOf
        rw [List.filterMap_eq_nil_iff]
        intro x hx
        obtain ⟨t, v⟩ := x
        have ht := hvt t (List.of_mem_zip hx).1
        cases v <;> simp [ht]
      refine ⟨ts.take vals.length, [], hrl, by simp [hnoref], by simp [hnoref, newRefsOf], ?_,
        hfv, ?_, by simp, ?_, by simp, noRef_of_values hcv hvt⟩
      · rw [List.map_nil]
        exact nativeResultsIn_values hvt (by rw [List.length_take, fitsAll_length hfitO]; simp; omega)
      · rw [hsplitO, keys_append, List.map_nil, List.append_nil]
      · intro x hx _
        rw [hsplitO]; exact List.mem_append_left _ hx
  -- The references written back, which overlap nowhere.
  have hwm := writtenOf_map (P := P) (f := f) hidl
  have hdisj : (P.writtenReferences f args).Pairwise
      (fun a b => a.location.overlaps b.location = false) := by
    by_cases hku : k = .update
    · subst hku; rw [writtenReferences_update hshape]; exact mutRefs_disjoint hfacts.exclusive
    · rw [writtenReferences_other (by rw [hshape]; simpa using hku)]; exact List.Pairwise.nil
  have hidsS : (s.frames.map (·.id)).Nodup := hp.inv.frameIds
  have hresS : (s.storage.map fun r => (r.address, r.type)).Nodup := hp.inv.resources
  obtain ⟨sM, hsM⟩ : ∃ sM : State, sM = { s with
      frames := { fr with
        stack := results.reverse ++ fr.stack.drop n
        pc := fr.pc + 1 } :: rest } := ⟨_, rfl⟩
  rw [← hsM] at hwb'
  obtain ⟨h₁, hh₁⟩ : ∃ h₁ : Shadow, h₁ = { h with
      frames := { g with stack := resultTrees.reverse ++ g.stack.drop n } :: hs
      fresh := F + newRefs.length } := ⟨_, rfl⟩
  obtain ⟨hP, hhP⟩ : ∃ hP : Shadow, hP = { h with
      frames := { g with stack := g.stack.drop n } :: hs } := ⟨_, rfl⟩
  have hidsM : sM.frames.map (·.id) = s.frames.map (·.id) := by
    rw [hsM]; simp [hsf]
  have hidsP : s₁.frames.map (·.id) = s.frames.map (·.id) := by
    rw [hs₁]; simp [hsf]
  have hlenRT := fitsAll_length hfitRes
  have hrest : framesFit hs rest = true := by
    have := hfit
    rw [shadow_fits, hsf, hh, framesFit_cons] at this
    simp only [Bool.and_eq_true] at this
    exact this.1.2
  have hext : extFit h s = true := by
    have := hfit; rw [shadow_fits, Bool.and_eq_true] at this; exact this.2
  -- The shadow with the results, before the values are written back.
  have hfit₁ : h₁.fits sM = true := by
    rw [hh₁, hsM, shadow_fits, framesFit_cons]
    simp only [Bool.and_eq_true]
    refine ⟨⟨frame_fits_stack hg (by
      rw [fitsAll_append (by simp [hlenRT])]
      simp [fitsAll_reverse hfitRes, (fitsAll_take_drop n hfa).2]) _, hrest⟩,
      (extFit_congr (h := h) (s := s) rfl rfl rfl rfl).trans hext⟩
  have hfitP : hP.fits s₁ = true := by
    rw [hhP, hs₁, shadow_fits, framesFit_cons]
    simp only [Bool.and_eq_true]
    refine ⟨⟨?_, hrest⟩, (extFit_congr (h := h) (s := s) rfl rfl rfl rfl).trans hext⟩
    have := frame_fits_stack hg (fitsAll_take_drop n hfa).2 fr.pc
    simpa using this
  -- The values written back, in the state and in the shadow.
  have hwbM : writeBack sM ((writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)).map
      (·.2)) writes = some s' := by rw [hwm]; exact hwb'
  have hwbP : writeBack s₁ ((writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)).map
      (·.2)) writes = some sW := by rw [hwm]; exact hwb
  have hdisjW : ((writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)).map
      (·.2)).Pairwise (fun a b => a.location.overlaps b.location = false) := by
    rw [hwm]; exact hdisj
  have hagM : WritesAgree sM s (writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)) :=
    WritesAgree.of_roots hdisjW
      (root_eq (s := s) (s' := sM) (by rw [hsM, hsf]; rfl) (by rw [hsM]) (by rw [hsM]))
  have hagP : WritesAgree s₁ s (writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)) :=
    WritesAgree.of_roots hdisjW
      (root_eq (s := s) (s' := s₁) (by rw [hs₁, hsf]; rfl) (by rw [hs₁]) (by rw [hs₁]))
  obtain ⟨h₂, hfold₂, Rb, hRb1, hRb2, -⟩ := writes_perm (s₀ := s) _ (ts.drop vals.length) writes
    hfit₁ (by rw [hidsM]; exact hidsS) (by rw [hsM]; exact hresS) hidsM (by rw [hsM]) hwbM hfw
    hagM
  obtain ⟨-, -, Rk, hRk1, -, hRkmem⟩ := writes_perm (s₀ := s) _ (ts.drop vals.length) writes
    hfitP (by rw [hidsP]; exact hidsS) (by rw [hs₁]; exact hresS) hidsP (by rw [hs₁]) hwbP hfw
    hagP
  obtain ⟨hfit₂, hfresh₂, -, -, -, -, -⟩ := writes_ghost (s₀ := s) _ (ts.drop vals.length) writes
    hfit₁ (by rw [hidsM]; exact hidsS) (by rw [hsM]; exact hresS) hidsM (by rw [hsM]) hagM hwbM
    hfold₂ hfw
  -- The annotation.
  obtain ⟨fr', tl, hf', hfr'⟩ : ∃ (fr' : Frame) (tl : List Frame), s'.frames = fr' :: tl ∧
      fr'.stack = results.reverse ++ fr.stack.drop n := ⟨_, _, by rw [hs'e], rfl⟩
  have hstack : (fr'.stack.take tys.length).reverse = results := by
    rw [hfr', List.take_left' (by simp [hrlen]), List.reverse_reverse]
  have hwlen : (writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)).length =
      writes.length := by
    rw [← List.length_map (f := (·.2)), hwm]; exact writeBack_length hwb
  have hn := nativeStep_of (P := P) (ν := ν) (s := s) (views := views) (places := places)
    (vals := vals) (writes := writes) (h := h) (hh := hh) (hinfo := hinfo) (htys := htys)
    (hpt := hpt) (hlen := by omega) (hseen := hseen) (hsl := by omega) (hguard := hguard)
    (hwl := hwlen) (hf' := hf') (hrr := by rw [hstack]; exact hrr)
    (hnew := by rw [hstack, hOut, hargl]; rw [hargl] at hnew; exact hnew)
    (hrt := by rw [hOut]; exact hrt)
    (hfold := by rw [hOut, hargl]; rw [hargl, hh₁] at hfold₂; exact hfold₂)
    (hfits := by simpa [ShadowPost.fits] using hfit₂)
  rw [hOut, hφ] at hn
  rw [hφ] at hguard
  refine ⟨_, _, hn, ?_⟩
  have H2' : ∀ k q w, (k, q, w) ∈ nodesOf views → sharedAt P f ptys k = false →
      ((w.type.abilities P).copy || decide ((passedOn φ (vals ++ writes)).count (k, q) ≤ 1)) =
        true ∧
      ((passedOn φ (vals ++ writes)).contains (k, q) ||
        (Consumer.native f info.moduleId (keepsNothingInside (passedOn φ (vals ++ writes)) k q)).Permits
          P π w) = true := by
    intro k q w hm hsh
    have := H2 _ hm
    simp only [hpt, Option.bind_some] at this
    unfold sharedAt at hsh
    simp only [hsh, Bool.false_eq_true, ↓reduceIte, Bool.and_eq_true] at this
    exact this
  have H2s : ∀ k q w, (k, q, w) ∈ nodesOf views → sharedAt P f ptys k = true →
      ((w.type.abilities P).copy || !(passedOn φ (vals ++ writes)).contains (k, q)) = true := by
    intro k q w hm hsh
    have := H2 _ hm
    simp only [hpt, Option.bind_some] at this
    unfold sharedAt at hsh
    simp only [hsh, ↓reduceIte] at this
    exact this
  -- The creations: of output nodes the flow marks fresh, which hold no reference.
  have hcreated : ∀ e ∈ evsO, ∃ j p w i, (j, p, w) ∈ nodesOf (vals ++ writes) ∧
      φ j p = .fresh ∧ e = .created i w (.native f info.moduleId views) ∧ w.isRef = false := by
    intro e he
    obtain ⟨x, hx, y, hy, hnone, i, rfl⟩ := hevO e he
    have hmem : (x.2, y.1, y.2) ∈ nodesOf (vals ++ writes) :=
      List.mem_flatMap.2 ⟨x, hx, List.mem_map_of_mem hy⟩
    have hfresh : φ x.2 y.1 = .fresh := by
      cases hxq : φ x.2 y.1 with
      | arg k q =>
        have := hguard' _ hmem k q hxq
        simp only [resultIdOf, hxq] at hnone
        rw [hnone] at this
        simp at this
      | fresh => rfl
    have hx1 : x.1 ∈ vals ++ writes := by
      have := List.mem_map_of_mem (f := Prod.fst) hx
      rwa [List.zipIdx_map_fst] at this
    have hsub : y.2 ∈ subvalues x.1 := by
      exact List.mem_map_of_mem (f := fun p => p.2) hy
    have hnr : y.2.isRef = false := by
      rcases List.mem_append.1 hx1 with hv | hw
      · exact hvr _ hv _ hsub
      · exact hwr _ hw _ hsub
    exact ⟨x.2, y.1, y.2, i, hmem, hfresh, rfl, hnr⟩
  refine {
    fits := by simpa [ShadowPost.fits] using hfit₂
    effect := ?_
    legit := ?_
    respected := ?_
    issuedOk := ?_
    inv := ?_
    createdPlain := ?_
    frameRefs := fun d r hm => by
      have := nativeStep_frameRefs hp hshape hstep ho hn hm
      rw [hh] at this
      exact this
    refsAt := by
      have := (nativeStep_frameRefsAt hp hshape hstep ho hn).2.2
      rw [hh] at this
      exact refsPerFrame_of this
    closures := ?_
    closuresKept := ?_ }
  · -- The effect.
    unfold Effect
    simp only [ShadowPost.occurrences, postFresh, hfresh₂]
    rw [hh₁]
    dsimp only
    -- The occurrences before: the arguments, and those of the state popped.
    have hpop : (keys (h.occurrences s)).Perm
        (keys (IdTree.occurrencesAll (g.stack.take args.length).reverse args) ++
          keys (hP.occurrences s₁)) := by
      rw [hhP, hs₁]
      have := keys_perm (occ_pop hh hsf hfa n)
      rw [keys_append] at this
      refine this.trans (List.Perm.append_right _ ?_)
      rw [hargl, hargs]
      exact keys_perm (occurrencesAll_reverse (fitsAll_take_drop n hfa).1).symm
    -- The occurrences after the results are pushed.
    have hpush : (keys (h₁.occurrences sM)).Perm
        (keys (IdTree.occurrencesAll resultTrees results) ++ keys (hP.occurrences s₁)) := by
      have := occ_push (h := hP) (s := s₁) (g := { g with stack := g.stack.drop n })
        (fr := { fr with stack := fr.stack.drop n }) (hs := hs) (rest := rest) (by rw [hhP])
        (by rw [hs₁]) (ts := resultTrees.reverse) (vs := results.reverse) (by simp [hlenRT]) s₁
        rfl rfl (fr.pc + 1) (F + newRefs.length)
      have hl : h₁.occurrences sM = Shadow.occurrences
          { hP with
            frames := { { g with stack := g.stack.drop n } with
              stack := resultTrees.reverse ++ ({ g with stack := g.stack.drop n } : ShadowFrame).stack } :: hs
            fresh := F + newRefs.length }
          { s₁ with
            frames := { { fr with stack := fr.stack.drop n } with
              stack := results.reverse ++ ({ fr with stack := fr.stack.drop n } : Frame).stack
              pc := fr.pc + 1 } :: rest } := by
        rw [hh₁, hsM, hhP, hs₁]
      rw [hl]
      refine (keys_perm this).trans ?_
      rw [keys_append]
      exact List.Perm.append_right _ (keys_perm (occurrencesAll_reverse hfitRes))
    -- The locations depend on the frames' locals only.
    have hlocP : ∀ l, locOcc hP s₁ l = locOcc h s l := by
      intro l
      unfold locOcc
      rw [tree_eq (h := h) (h' := hP) (s := s) (s' := s₁) hidsP (by rw [hs₁])
          (by rw [hhP, hh]; rfl) (by rw [hhP]) (by rw [hhP]) l
          (root_eq (s := s) (s' := s₁) (by rw [hs₁, hsf]; rfl) (by rw [hs₁]) (by rw [hs₁]) _),
        read_eq (s := s) (s' := s₁) (by rw [hs₁, hsf]; rfl) (by rw [hs₁]) (by rw [hs₁]) l]
    have hlocM : ∀ l, locOcc h₁ sM l = locOcc h s l := by
      intro l
      unfold locOcc
      rw [tree_eq (h := h) (h' := h₁) (s := s) (s' := sM) hidsM (by rw [hsM])
          (by rw [hh₁, hh]; rfl) (by rw [hh₁]) (by rw [hh₁]) l
          (root_eq (s := s) (s' := sM) (by rw [hsM, hsf]; rfl) (by rw [hsM]) (by rw [hsM]) _),
        read_eq (s := s) (s' := sM) (by rw [hsM, hsf]; rfl) (by rw [hsM]) (by rw [hsM]) l]
    rw [flatMap_congr_mem (fun x _ => hlocP x.2.location)] at hRk1
    rw [flatMap_congr_mem (fun x _ => hlocM x.2.location)] at hRb1
    have hRbk : Rb.Perm (keys (IdTree.occurrencesAll resultTrees results) ++ Rk) := by
      refine (List.perm_append_left_iff
        ((writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)).flatMap
          fun x => locOcc h s x.2.location)).1 ?_
      refine hRb1.symm.trans (hpush.trans ?_)
      refine (List.Perm.append_left _ hRk1).trans ?_
      exact List.perm_iff_count.2 fun y => by simp only [List.count_append]; omega
    -- The native's nodes, by position.
    have hps := pre_split (P := P) (f := f) (h := h) (s := s) (sharedAt P f ptys) 0
      (g.stack.take args.length).reverse args seen views hidl hsl hvl hsfits
      (fun i t a st w h1 h2 h3 h4 h5 => by
        obtain ⟨e1, e2⟩ := hsval i t a st w h1 h2 h3 h4 h5
        exact ⟨e1, e2, by rw [Nat.zero_add]; exact sharedAt_val hconfa h2 h5⟩)
      (fun i t r st w h1 h2 h3 h4 => by
        obtain ⟨e1, e2, e3⟩ := hsref i t r st w h1 h2 h3 h4
        refine ⟨e1, ?_, by rw [Nat.zero_add]; exact sharedAt_ref hconfa h2⟩
        unfold locOcc; rw [e2, e3])
    have hpre : (keys (h.occurrences s)).Perm
        (refLeaves (g.stack.take args.length).reverse args ++
          (ownedPos (sharedAt P f ptys) 0 seen views).map (·.2) ++ Rk) := by
      refine hpop.trans ((List.Perm.append_left _ hRk1).trans ?_)
      rw [← List.append_assoc]
      exact List.Perm.append_right _ hps
    have hWts : ((ts.drop vals.length).zip writes).flatMap (fun y => keys (y.1.occurrences y.2)) =
        keys (IdTree.occurrencesAll (ts.drop vals.length) writes) := by
      rw [occurrencesAll_eq_flatMap, keys, List.map_flatMap]; rfl
    have hpost : (keys (h₂.occurrences s')).Perm
        ((keys (IdTree.occurrencesAll resultTrees results) ++
          keys (IdTree.occurrencesAll (ts.drop vals.length) writes)) ++ Rk) := by
      refine hRb2.trans ?_
      rw [hWts]
      refine (List.Perm.append_left _ hRbk).trans ?_
      exact List.perm_iff_count.2 fun y => by simp only [List.count_append]; omega
    -- The events: what is issued and consumed.
    have hiss0 : issuedOf (consumedOfNative P f info.moduleId ptys seen views
        (passedOn φ (vals ++ writes))) = [] := by
      unfold issuedOf
      rw [List.filterMap_eq_nil_iff]
      intro e he
      obtain ⟨_, _, _, _, rfl, -⟩ := consumedOfNative_mem' he
      rfl
    have hnbO : ∀ i r p, Event.borrowed i r p ∉ evsO := fun i r p hb => by
      obtain ⟨_, _, _, _, _, _, heq, _⟩ := hcreated _ hb; cases heq
    have hiss : issuedOf (nativeEventsOf (refArgsOf (g.stack.take args.length).reverse args)
        (writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)) newRefs
        (consumedOfNative P f info.moduleId ptys seen views (passedOn φ (vals ++ writes))) evsO) =
        newRefs.map (fun x => (x.1, (Value.ref x.2.1).key)) ++ keys (createdOf evsO) := by
      rw [issuedOf_nativeEvents, hiss0, List.append_nil, issuedOf_created hnbO]
    have hcons : keys (consumedOf (nativeEventsOf (refArgsOf (g.stack.take args.length).reverse args)
        (writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)) newRefs
        (consumedOfNative P f info.moduleId ptys seen views (passedOn φ (vals ++ writes))) evsO)) =
        ((ownedPos (sharedAt P f ptys) 0 seen views).filter
          fun x => !(passedOn φ (vals ++ writes)).contains x.1).map (·.2) := by
      rw [consumedOf_nativeEvents, hcO, List.append_nil, consumedOfNative_keys hsfits]
    obtain ⟨occAt, hoccAt⟩ : ∃ occAt : Nat × Path → Nat × Key,
        occAt = fun a => ((argOcc seen views a).1, (argOcc seen views a).2.key) := ⟨_, rfl⟩
    have hnodes : ∀ k q w v, views[k]? = some w → w.at? q = some v → (k, q, v) ∈ nodesOf views := by
      intro k q w v hk hv
      unfold nodesOf
      exact List.mem_flatMap.2 ⟨(w, k), List.mem_zipIdx_iff_getElem?.2 hk,
        List.mem_map.2 ⟨(q, v), at_nodes q hv, rfl⟩⟩
    have hocc : ∀ x ∈ ownedPos (sharedAt P f ptys) 0 seen views, x.2 = occAt x.1 := by
      intro x hx
      obtain ⟨j, q, st, w, v, i, rfl, hst, hw, -, hv, hi⟩ := mem_ownedPos hsfits hx
      rw [hoccAt]
      simp only [argOcc, hst, hw, Option.bind_some, hv, Option.getD_some, hi]
    have hO : ∀ a ∈ passedOn φ (vals ++ writes),
        a ∈ (ownedPos (sharedAt P f ptys) 0 seen views).map (·.1) ∨
          (occAt a ∈ Rk ∧ ((occAt a).2.type.abilities P).copy = true) := by
      intro a ha
      have ha0 := ha
      unfold passedOn at ha
      obtain ⟨x, hx, hxa⟩ := List.mem_filterMap.1 ha
      obtain ⟨j, p, v⟩ := x
      simp only at hxa
      split at hxa
      · next k q hxq =>
        simp only [Option.some.injEq] at hxa
        subst hxa
        have hhv := hhonest (j, p, v) hx k q hxq
        cases hk : views[k]? with
        | none => rw [hk] at hhv; simp at hhv
        | some w =>
        rw [hk, Option.bind_some] at hhv
        have hkl : k < args.length := by have := (List.getElem?_eq_some_iff.1 hk).1; omega
        obtain ⟨st, hst⟩ : ∃ st, seen[k]? = some st := ⟨_, List.getElem?_eq_getElem (by omega)⟩
        obtain ⟨a', ha'⟩ : ∃ a', args[k]? = some a' := ⟨_, List.getElem?_eq_getElem hkl⟩
        obtain ⟨t, ht⟩ : ∃ t, (g.stack.take args.length).reverse[k]? = some t :=
          ⟨_, List.getElem?_eq_getElem (by omega)⟩
        have hfsw := hsfits k st w hst hk
        cases hsh : sharedAt P f ptys k with
        | false => exact Or.inl (ownedPos_mem hst hk hfsw hsh hhv)
        | true =>
          right
          cases a' with
          | ref r =>
            obtain ⟨-, htree, hread⟩ := hsref k t r st w ht ha' hst hk
            have hnw : (r.mutable && P.nativeShape? f == some .update) = false := by
              have := sharedAt_ref (f := f) hconfa ha'
              rw [hsh] at this
              revert this
              cases (r.mutable && P.nativeShape? f == some .update) <;> simp
            obtain ⟨u, hu, -⟩ := at_fits q hfsw hhv
            have hoa : occAt (k, q) = (u.id, v.key) := by
              rw [hoccAt]; simp [argOcc, hst, hk, hu, hhv]
            rw [hoa]
            refine ⟨?_, ?_⟩
            · refine hRkmem r.location (fun y hy => ?_) ⟨w, ?_⟩ _ ?_
              · obtain ⟨yi, yr⟩ := y
                obtain ⟨hy1, hy2⟩ := List.mem_filter.1 hy
                simp only at hy2
                obtain ⟨t', -, hz⟩ := mem_refArgsOf hy1
                have hry : Value.ref yr ∈ args := (List.of_mem_zip hz).2
                have hne : yr ≠ r := fun e => by
                  rw [e, hnw] at hy2; cases hy2
                simp only [Bool.and_eq_true] at hy2
                exact exclusive_mem hfacts.exclusive hry (List.mem_of_getElem? ha') hne hy2.1
              · rw [read_eq (s := s) (s' := s₁) (by rw [hs₁, hsf]; rfl) (by rw [hs₁]) (by rw [hs₁])]
                exact hread
              · rw [hlocP]
                unfold locOcc
                rw [htree, hread]
                exact mem_keys (at_occ hfsw hhv hu)
            · have := H2s k q v (hnodes k q w v hk hhv) hsh
              have hc : (passedOn φ (vals ++ writes)).contains (k, q) = true :=
                List.contains_iff_mem.2 ha0
              rw [hc] at this
              simpa [key_type] using this
          | _ =>
            rw [sharedAt_val hconfa ha' (by simp)] at hsh
            cases hsh
      · cases hxa
    have hcopy : ∀ x ∈ ownedPos (sharedAt P f ptys) 0 seen views,
        (x.2.2.type.abilities P).copy = true ∨ (passedOn φ (vals ++ writes)).count x.1 ≤ 1 := by
      intro x hx
      obtain ⟨j, q, st, w, v, i, rfl, hst, hw, hsh, hv, -⟩ := mem_ownedPos hsfits hx
      have := (H2' j q v (hnodes j q w v hw hv) hsh).1
      simp only [Bool.or_eq_true, decide_eq_true_eq] at this
      simpa [key_type] using this
    have hDz : ∀ x ∈ refLeaves (g.stack.take args.length).reverse args,
        (x.2.type.abilities P).drop = true := by
      intro x hx
      unfold refLeaves at hx
      obtain ⟨⟨t, a⟩, -, hx⟩ := List.mem_flatMap.1 hx
      cases a with
      | ref r =>
        simp only [List.mem_singleton] at hx
        subst hx
        simp [Value.key, Key.type, Value.type, RuntimeType.abilities]
      | _ => simp at hx
    have hRO : (keys (IdTree.occurrencesAll resultTrees results) ++
        keys (IdTree.occurrencesAll (ts.drop vals.length) writes)).Perm
        ((passedOn φ (vals ++ writes)).map occAt ++
          (newRefs.map (fun x => (x.1, (Value.ref x.2.1).key)) ++ keys (createdOf evsO))) := by
      refine hpostRes.trans ?_
      have hk := keys_perm hpassed
      rw [keys_append] at hk
      have hkm : keys ((passedOn φ (vals ++ writes)).map (argOcc seen views)) =
          (passedOn φ (vals ++ writes)).map occAt := by
        rw [hoccAt]; simp [keys, List.map_map, Function.comp_def]
      rw [hkm] at hk
      refine List.perm_iff_count.2 fun y => ?_
      have := hk.count_eq y
      simp only [List.count_append] at this ⊢
      omega
    have hnewlt : ∀ x ∈ newRefs, F ≤ x.1 ∧ x.1 < F + newRefs.length := by
      intro x hx
      obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 hx
      have := hnewIds i x hi
      have hl := (List.getElem?_eq_some_iff.1 hi).1
      omega
    have hnewmap : newRefs.map (·.1) = List.range' F newRefs.length := by
      apply List.ext_getElem?
      intro i
      by_cases hl : i < newRefs.length
      · rw [List.getElem?_map, List.getElem?_range' hl]
        obtain ⟨x, hx⟩ : ∃ x, newRefs[i]? = some x := ⟨_, List.getElem?_eq_getElem hl⟩
        rw [hx]
        simp [hnewIds i x hx]
      · rw [List.getElem?_map, List.getElem?_eq_none (by omega),
          List.getElem?_eq_none (by simp; omega)]
        rfl
    have hb : ∀ x ∈ newRefs.map (fun x => (x.1, (Value.ref x.2.1).key)) ++ keys (createdOf evsO),
        h.fresh ≤ x.1 ∧ x.1 < F + newRefs.length := by
      intro x hx
      rcases List.mem_append.1 hx with hx | hx
      · obtain ⟨y, hy, rfl⟩ := List.mem_map.1 hx; have := hnewlt y hy; omega
      · obtain ⟨o, ho, h1, -⟩ := mem_keys_iff.1 hx; have := hbO o ho; omega
    have hnd' : ((newRefs.map (fun x => (x.1, (Value.ref x.2.1).key)) ++
        keys (createdOf evsO)).map (·.1)).Nodup := by
      rw [List.map_append, List.map_map]
      have e1 : ((·.1) ∘ fun x : Nat × Reference × Nat => (x.1, (Value.ref x.2.1).key)) =
          (·.1) := rfl
      have e2 : (keys (createdOf evsO)).map (·.1) = (createdOf evsO).map (·.1) := by
        simp [keys, List.map_map, Function.comp_def]
      rw [e1, e2, hnewmap]
      refine List.nodup_append.2 ⟨List.nodup_range', hndO, fun a ha b hb' hab => ?_⟩
      rw [List.mem_range'] at ha
      obtain ⟨o, ho, rfl⟩ := List.mem_map.1 hb'
      have := hbO o ho
      omega
    have hE := native_effectK (P := P) (ownedPos (sharedAt P f ptys) 0 seen views)
      (passedOn φ (vals ++ writes)) occAt hocc (ownedPos_nodup hsfits) (Rk := Rk) hO hcopy
      (Dz := refLeaves (g.stack.take args.length).reverse args) hDz hcons
      (RO := keys (IdTree.occurrencesAll resultTrees results) ++
        keys (IdTree.occurrencesAll (ts.drop vals.length) writes)) (by rw [hiss]; exact hRO) (by rw [hiss]; exact hb) (by rw [hiss]; exact hnd')
      (by omega)
    exact hE.perm hpre hpost
  · -- Consumption is legitimate.
    intro i w c he
    rcases nativeEventsOf_cases he with ⟨x, -, he'⟩ | ⟨x, -, he'⟩ | ⟨x, -, he'⟩ | ⟨x, -, he'⟩ |
      he' | he'
    · cases he'
    · cases he'
    · cases he'
    · cases he'
    · obtain ⟨k', q, i', w', heq, hsh, hc, hm⟩ := consumedOfNative_mem' he'
      simp only [Event.consumed.injEq] at heq
      obtain ⟨rfl, rfl, rfl⟩ := heq
      have := (H2' _ _ _ hm hsh).2
      simp only [Bool.or_eq_true] at this
      exact (consumer_permits (A := A) (i := i) (this.resolve_left (by simpa using hc))).1
    · obtain ⟨_, _, _, _, _, _, heq, _⟩ := hcreated _ he'; cases heq
  · -- D3 for every event.
    intro e he
    rcases nativeEventsOf_cases he with ⟨x, -, rfl⟩ | ⟨x, -, rfl⟩ | ⟨x, -, rfl⟩ | ⟨x, -, rfl⟩ |
      he' | he'
    · exact ⟨rfl, rfl⟩
    · exact ⟨rfl, rfl⟩
    · exact ⟨rfl, rfl⟩
    · exact ⟨rfl, rfl⟩
    · obtain ⟨k', q, i', w', rfl, hsh, hc, hm⟩ := consumedOfNative_mem' he'
      have := (H2' _ _ _ hm hsh).2
      simp only [Bool.or_eq_true] at this
      exact ⟨(consumer_permits (A := A) (i := i') (this.resolve_left (by simpa using hc))).2, rfl⟩
    · obtain ⟨j, p, w, i, hm, hfr, rfl, -⟩ := hcreated _ he'
      exact native_permits_respected (hcreate _ hm hfr)
  · -- The identities issued: the new values, and the new references.
    intro x hx
    rcases mem_issuedOf hx with ⟨w, c, he, hk⟩ | ⟨r, par, he, hk⟩
    · rcases nativeEventsOf_cases he with ⟨_, -, he'⟩ | ⟨_, -, he'⟩ | ⟨_, -, he'⟩ |
        ⟨_, -, he'⟩ | he' | he'
      · cases he'
      · cases he'
      · cases he'
      · cases he'
      · obtain ⟨_, _, _, _, he'', -⟩ := consumedOfNative_mem' he'; cases he''
      · obtain ⟨j, p, w', i, -, -, heq, hnr⟩ := hcreated _ he'
        simp only [Event.created.injEq] at heq
        obtain ⟨-, rfl, -⟩ := heq
        rw [← hk]
        exact container_ok (not_ref_of_isRef hnr)
    · rcases nativeEventsOf_cases he with ⟨_, -, he'⟩ | ⟨_, -, he'⟩ | ⟨_, -, he'⟩ |
        ⟨y, hy, he'⟩ | he' | he'
      · cases he'
      · cases he'
      · cases he'
      · simp only [Event.borrowed.injEq] at he'
        obtain ⟨-, rfl, -⟩ := he'
        rw [← hk]
        exact hokRes y hy
      · obtain ⟨_, _, _, _, he'', -⟩ := consumedOfNative_mem' he'; cases he''
      · obtain ⟨_, _, _, _, _, _, heq, _⟩ := hcreated _ he'; cases heq
  · -- The state invariant: a transaction writes back into no value it holds.
    have hinv := inv_native ht hp.inv hshape hstep ho
    refine { hinv with heldDrop := fun hσ v hv => ?_ }
    have hheld : s'.held = s.held := by
      have := writeBack_held _ _ hwb' (fun r hr => by
        by_cases hku : k = .update
        · subst hku
          rw [writtenReferences_update hshape] at hr
          obtain ⟨hra, hrm⟩ := mem_mutRefsOf hr
          have hrs : Value.ref r ∈ fr.stack := by
            rw [hargs] at hra; exact List.mem_of_mem_take (List.mem_reverse.1 hra)
          have hok := arg_key_ok hp hsf hh hrs
          simp only [Value.key, Key.Ok] at hok
          cases hheld : r.location.root.isHeld
          · rfl
          · rw [hok.1 hσ hheld] at hrm; cases hrm
        · rw [writtenReferences_other (by rw [hshape]; simpa using hku)] at hr
          cases hr)
      rw [this, hsM]
    rw [hheld] at hv
    exact hp.inv.heldDrop hσ v hv
  · -- No reference is created by `created`.
    intro i w c he
    rcases nativeEventsOf_cases he with ⟨_, -, he'⟩ | ⟨_, -, he'⟩ | ⟨_, -, he'⟩ | ⟨_, -, he'⟩ |
      he' | he'
    · cases he'
    · cases he'
    · cases he'
    · cases he'
    · obtain ⟨_, _, _, _, he'', -⟩ := consumedOfNative_mem' he'; cases he''
    · obtain ⟨j, p, w', i', -, -, heq, hnr⟩ := hcreated _ he'
      simp only [Event.created.injEq] at heq
      obtain ⟨-, rfl, -⟩ := heq
      exact hnr
  · -- D3c: a native creates no closure.
    intro e he
    rcases nativeEventsOf_cases he with ⟨x, -, rfl⟩ | ⟨x, -, rfl⟩ | ⟨x, -, rfl⟩ | ⟨x, -, rfl⟩ |
      he' | he'
    · rfl
    · rfl
    · rfl
    · rfl
    · obtain ⟨k', q, i', w', rfl, -⟩ := consumedOfNative_mem' he'
      rfl
    · obtain ⟨j, p, w, i, hm, hfr, rfl, -⟩ := hcreated _ he'
      exact native_closures (hcreate _ hm hfr)
  · -- A closure after the call was there before, or the native created it.
    show ∀ x ∈ h₂.occurrences s', IsClosure x.2 = true → x ∈ h.occurrences s ∨ ∃ c, _
    -- What the native sees is in the state before it.
    have hseenOcc : ∀ (k : Nat) (st : IdTree) (w : Value), seen[k]? = some st → views[k]? = some w →
        ∀ o ∈ st.occurrences w, o ∈ h.occurrences s := by
      intro k st w hst hw o ho
      have hkl : k < args.length := by have := (List.getElem?_eq_some_iff.1 hst).1; omega
      obtain ⟨a, ha⟩ : ∃ a, args[k]? = some a := ⟨_, List.getElem?_eq_getElem hkl⟩
      obtain ⟨t, ht⟩ : ∃ t, (g.stack.take args.length).reverse[k]? = some t :=
        ⟨_, List.getElem?_eq_getElem (by omega)⟩
      by_cases hra : ∃ r, a = .ref r
      · obtain ⟨r, rfl⟩ := hra
        obtain ⟨-, htree, hread⟩ := hsref k t r st w ht ha hst hw
        obtain ⟨tw, htw, -, R, hR⟩ := tree_read hp.fits hread
        rw [htree, Option.some.injEq] at htw
        subst htw
        exact hR.symm.subset (List.mem_append_left _ ho)
      · obtain ⟨rfl, rfl⟩ := hsval k t a st w ht ha hst hw (fun r e => hra ⟨r, e⟩)
        have hz : (st, w) ∈ g.stack.zip fr.stack := by
          have := zip_of_getElem? ht ha
          rw [hargl] at this
          rw [hargs] at this
          exact zip_rev_take hlenG this
        rw [shadow_occurrences, hh, hsf, framesOcc_cons, frame_occurrences]
        exact List.mem_append_left _ (List.mem_append_left _ (List.mem_append_right _
          (occ_of_zip hz _ ho)))
    -- The outputs' closures.
    have hout : ∀ x ∈ IdTree.occurrencesAll ts (vals ++ writes),
        x ∈ h.occurrences s ∨ ∃ c, Event.created x.1 x.2 c ∈ evsO := by
      intro x hx
      rcases List.mem_append.1 (hpassed.subset hx) with hx | hx
      · left
        obtain ⟨a, ha, rfl⟩ := List.mem_map.1 hx
        unfold passedOn at ha
        obtain ⟨y, hy, hya⟩ := List.mem_filterMap.1 ha
        obtain ⟨j, p, v⟩ := y
        simp only at hya
        split at hya
        · next k q hxq =>
          simp only [Option.some.injEq] at hya
          subst hya
          have hhv := hhonest (j, p, v) hy k q hxq
          cases hk : views[k]? with
          | none => rw [hk] at hhv; simp at hhv
          | some w =>
          rw [hk, Option.bind_some] at hhv
          obtain ⟨st, hst⟩ : ∃ st, seen[k]? = some st :=
            ⟨_, List.getElem?_eq_getElem (by have := (List.getElem?_eq_some_iff.1 hk).1; omega)⟩
          have hfsw := hsfits k st w hst hk
          obtain ⟨u, hu, -⟩ := at_fits q hfsw hhv
          have hao : argOcc seen views (k, q) = (u.id, v) := by simp [argOcc, hst, hk, hu, hhv]
          rw [hao]
          exact hseenOcc k st w hst hk _ (at_occ hfsw hhv hu)
        · cases hya
      · exact Or.inr (mem_createdOf.1 hx)
    have hpopO := occ_pop hh hsf hfa n
    have hpushO := occ_push (h := hP) (s := s₁) (g := { g with stack := g.stack.drop n })
      (fr := { fr with stack := fr.stack.drop n }) (hs := hs) (rest := rest) (by rw [hhP])
      (by rw [hs₁]) (ts := resultTrees.reverse) (vs := results.reverse) (by simp [hlenRT]) s₁
      rfl rfl (fr.pc + 1) (F + newRefs.length)
    have hl : h₁.occurrences sM = Shadow.occurrences
        { hP with
          frames := { { g with stack := g.stack.drop n } with
            stack := resultTrees.reverse ++ ({ g with stack := g.stack.drop n } : ShadowFrame).stack } :: hs
          fresh := F + newRefs.length }
        { s₁ with
          frames := { { fr with stack := fr.stack.drop n } with
            stack := results.reverse ++ ({ fr with stack := fr.stack.drop n } : Frame).stack
            pc := fr.pc + 1 } :: rest } := by
      rw [hh₁, hsM, hhP, hs₁]
    have hPs : ∀ x ∈ hP.occurrences s₁, x ∈ h.occurrences s := by
      intro x hx
      rw [hhP, hs₁] at hx
      exact hpopO.symm.subset (List.mem_append_right _ hx)
    have hwc := writes_closures (s₀ := s) _ (ts.drop vals.length) writes hfit₁
      (by rw [hidsM]; exact hidsS) (by rw [hsM]; exact hresS) hidsM (by rw [hsM]) hagM hwbM hfold₂
      hfw
    have hlift : ∀ x, (x ∈ h.occurrences s ∨ ∃ c, Event.created x.1 x.2 c ∈ evsO) →
        x ∈ h.occurrences s ∨ ∃ c, Event.created x.1 x.2 c ∈
          nativeEventsOf (refArgsOf (g.stack.take args.length).reverse args)
            (writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)) newRefs
            (consumedOfNative P f info.moduleId ptys seen views (passedOn φ (vals ++ writes)))
            evsO := by
      rintro x (hx | ⟨c, hc⟩)
      · exact Or.inl hx
      · exact Or.inr ⟨c, List.mem_append_right _ hc⟩
    intro x hx hcl
    refine hlift x ?_
    rcases hwc x hx hcl with hx | ⟨y, hy, hx⟩
    · rw [hl] at hx
      rcases List.mem_append.1 (hpushO.subset hx) with hx | hx
      · exact hout x (hclRes x ((occurrencesAll_reverse hfitRes).subset hx) hcl)
      · exact Or.inl (hPs x hx)
    · refine hout x ?_
      rw [hsplitO]
      exact List.mem_append_right _ (occ_of_zip hy _ hx)

/-- **Every native call that returns and continues is annotated**, under
the native contract. -/
theorem nativeAnnotated {σ : Bool} {P : Program} {N : NativeRel} {ν : NativeFlows} {π : NativePolicy}
    {A : List StructId} {auth : List Address} (hown : NativesRespectOwnership P N ν π) :
    NativeAnnotated σ P N ν π A auth := by
  intro s h s' f args views vals writes places hp ht hstep hN hcont
  obtain ⟨fr, rest, n, hsf, hn, hargs, hs', hfacts⟩ := step_callNative hstep
  cases hshape : P.nativeShape? f with
  | some k => exact nativeAnnotated_modeled hown hp ht hstep hN hcont hshape
  | none =>
  have hvals := native_values hstep hshape hcont
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  have hv := hfacts.values hshape
  have hnoref := noRef_of_conformsAll hconfa (hv.1 _ hpt)
  have hviews : views = args := by
    have := hfacts.view
    rw [nativeView_of_noRefs _ hnoref, Option.some.injEq] at this
    exact this.symm
  subst hviews
  have hs'f : s'.frames = { fr with stack := fr.stack.drop n } :: rest := by rw [hs']
  obtain ⟨ho, ⟨tys, htys, hconf⟩, rfl, rfl⟩ := resume_running hs'f hshape hnoref hv.2 hcont
  obtain ⟨info, hinfo, hresp⟩ := hown f views vals [] [] hN
  rw [List.append_nil] at hresp
  have hns : ∀ k : Nat, ((P.paramTypes f).bind (·[k]?)).any (P.nativeKeeps f) = false := by
    intro k
    rw [hpt, Option.bind_some]
    cases hk : ptys[k]? with
    | none => rfl
    | some t =>
      have := hv.1 _ hpt t (List.mem_of_getElem? hk)
      cases t <;> simp_all [Program.nativeKeeps, RuntimeType.isValue]
  have hfit := hp.fits
  cases hh : h.frames with
  | nil => rw [shadow_fits, hsf, hh] at hfit; simp [framesFit] at hfit
  | cons g hs =>
    rw [shadow_fits, hsf, hh, framesFit_cons] at hfit
    simp only [Bool.and_eq_true] at hfit
    obtain ⟨hlen, hguard, hG⟩ := native_ghost (π := π) (A := A) (auth := auth) (fresh := h.fresh)
      hfit.1.1 hn hargs hresp hns hvals
    have hSG := lift_frame (s₂ := s') hp hsf hh (by rw [hs']) (by rw [hs']) (by rw [hs']) (by rfl) hG
    rw [hh] at hSG
    rw [ho]
    refine ⟨_, _, ?_, hSG⟩
    rw [nativeStep_values hh hinfo hpt htys (hv.1 _ hpt) (hv.2 _ htys)
      (conformsAll_length hconf) hnoref, if_pos ⟨hlen, hguard, hSG.fits⟩]
    rfl

/-- **The ghost layer accounts for every step** from an admissible initial
state, when the natives' flows explain them, whatever the values the
transaction holds. -/
theorem shadowAccounts_admissible {P : Program} {N : NativeRel} {ν : NativeFlows}
    {π : NativePolicy} {s₀ : State} (hown : NativesRespectOwnership P N ν π)
    (h0 : AdmissibleInitial P s₀) : ShadowAccounts P N ν s₀ := by
  intro s h evs ht o hstep hcont
  have hI := refTraceInv_trace h0 ht
  cases hstep with
  | core hs =>
    obtain ⟨post, evs', hc⟩ := coreStep_exists hI.pre hs hcont
    exact ⟨post, evs', .core hs hc⟩
  | native hs hN =>
    rename_i s' f args views r
    cases r with
    | failed e => simp [resumeNative, Outcome.continues] at hcont
    | returned vals writes places =>
      obtain ⟨post, evs', hn, -⟩ := nativeAnnotated (σ := false) (A := []) (auth := []) hown
        _ _ _ _ _ _ _ _ _ hI.pre hI.typed hs hN hcont
      exact ⟨post, evs', .native hs hN hn⟩

end MoveBytecode.Proofs
