-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.NativeAnalysis

/-!
# The tracked references are held

UNTRUSTED (checked by the kernel). The references the verifier's analysis
tracks (`Tracked`) satisfy `HeldFacts` along the executions of an M3 program
from an admissible initial state (`heldFacts_tracked`):

- a step uses only tracked references: those on the operand stack, and those
  it copies or moves out of a local, which the availability analysis has
  available;
- a reference tracked after a step was tracked before it, or the step
  borrowed it.

The second follows the identity trees. A tree in a tracked slot after a step
was in a tracked slot before it, or is new, or was the tree of a value that is
no reference (`Origin`). An identity a `borrowed` event created labels that
reference only (`IdsKind`), so the slot the tree came from held the same
reference, and a new identity or one of a value is not a reference's.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph

/-! ## Identities of references -/

/-- The identities the events `E` borrowed label only those references, and
every identity is below the fresh counter. -/
private def IdsKind (s : State) (h : Shadow) (E : List Event) : Prop :=
  ∀ x ∈ keys (h.occurrences s), x.1 < h.fresh ∧
    (x.1 ∈ borrowedIds E → ∃ r par, x.2 = (Value.ref r).key ∧ Event.borrowed x.1 r par ∈ E)

/-- Two members of a list with distinct first components are equal when their
first components are. -/
private theorem tf_fst_unique {α β : Type} :
    ∀ {l : List (α × β)}, (l.map (·.1)).Nodup → ∀ {x y : α × β}, x ∈ l → y ∈ l → x.1 = y.1 →
      x = y
  | [], _, _, _, hx, _, _ => by simp at hx
  | a :: l, hnd, x, y, hx, hy, he => by
    simp only [List.map_cons, List.nodup_cons, List.mem_map, not_exists, not_and] at hnd
    rcases List.mem_cons.1 hx with hxa | hxl
    · rcases List.mem_cons.1 hy with hya | hyl
      · rw [hxa, hya]
      · subst hxa
        exact absurd he.symm (hnd.1 y hyl)
    · rcases List.mem_cons.1 hy with hya | hyl
      · subst hya
        exact absurd he (hnd.1 x hxl)
      · exact tf_fst_unique hnd.2 hxl hyl he

/-- **A core step keeps the identities of references apart**: every identity is
issued, and a borrowed one is a reference's. -/
private theorem idsKind_core {P : Program} {s s' : State} {h h' : Shadow} {E evs : List Event}
    (hI : RefTraceInv P s h E) (hK : IdsKind s h E) (hstep : step P s = .done (.running s'))
    (hc : Shadow.coreStep P s (.running s') h = some (.running h', evs)) :
    IdsKind s' h' (E ++ evs) := by
  obtain ⟨post, evs'', hc', hG⟩ := coreStep_annot (σ := false) (π := {}) (A := [])
    (auth := []) hI.pre hstep (by simp [Outcome.continues]) (fun vs st ho => by cases ho)
  rw [hc, Option.some.injEq, Prod.mk.injEq] at hc'
  obtain ⟨rfl, rfl⟩ := hc'
  have heff := hG.effect
  obtain ⟨-, -, -, -, -, -, -, hiss, hnd, hff⟩ := heff
  simp only [postFresh] at hiss hff
  intro x hx
  rcases hG.effect.sub hx with hx | hx
  · obtain ⟨hlt, hk⟩ := hK x hx
    refine ⟨by omega, fun hb => ?_⟩
    rw [borrowedIds_append, List.mem_append] at hb
    rcases hb with hb | hb
    · obtain ⟨r, par, h1, h2⟩ := hk hb
      exact ⟨r, par, h1, List.mem_append_left _ h2⟩
    · obtain ⟨r, par, hb'⟩ := mem_borrowedIds.1 hb
      have := (hiss (x.1, (Value.ref r).key) (List.mem_filterMap.2 ⟨_, hb', rfl⟩)).1
      omega
  · have hb' := hiss x hx
    refine ⟨hb'.2, fun hb => ?_⟩
    rw [borrowedIds_append, List.mem_append] at hb
    rcases hb with hb | hb
    · have := hI.below x.1 hb
      omega
    · obtain ⟨r, par, hbr⟩ := mem_borrowedIds.1 hb
      have hx2 : (x.1, (Value.ref r).key) ∈ issuedOf evs := List.mem_filterMap.2 ⟨_, hbr, rfl⟩
      have he := tf_fst_unique hnd hx hx2 rfl
      exact ⟨r, par, by rw [he], List.mem_append_right _ hbr⟩


/-- A tree's own occurrence. -/
theorem tf_root_occ (t : IdTree) (v : Value) : (t.id, v) ∈ t.occurrences v := by
  obtain ⟨i, cs⟩ := t
  simp [IdTree.occurrences, IdTree.id]

/-- The reference parameters of an initial frame are occurrences of its locals. -/
private theorem tf_initParams_occ {g : ShadowFrame} {fr : Frame} {d : Nat} {r : Reference}
    (h : (d, r) ∈ initParams g fr) : (d, Value.ref r) ∈ localsOcc g fr := by
  simp only [initParams, List.mem_filterMap] at h
  obtain ⟨⟨ot, ov⟩, hz, hm⟩ := h
  have hov : ∃ t, ot = some t ∧ ov = some (.ref r) ∧ d = t.id := by
    rcases ot with _ | t <;> rcases ov with _ | v <;> try (simp at hm)
    cases v <;> simp_all
  obtain ⟨t, rfl, rfl, rfl⟩ := hov
  simp only [localsOcc, List.mem_flatMap]
  exact ⟨_, hz, by simpa [ShadowFrame.slotOccurrences] using tf_root_occ t (.ref r)⟩

/-- **The identities at an admissible initial state.** -/
private theorem idsKind_initial {P : Program} {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    IdsKind s₀ (Shadow.initial s₀).1 (Shadow.initial s₀).2 := by
  obtain ⟨f, args, tys, storage, held, -, -, -, hinit⟩ := h0
  obtain ⟨fr, localTys, hfr, hst, -⟩ := initialState_shape hinit
  obtain ⟨ls, evs₁, n₁, hf, -, -, hocc, hb, hnd, -, hev⟩ := locals_fold fr.locals.toList [] 0 []
  obtain ⟨sts, e₂, n₂, hfS, -, hoccS, hbS, -, hleS, hevS⟩ :=
    fresh_fold .storage (s₀.storage.map (·.value)) [] n₁ []
  obtain ⟨hts, e₃, n₃, hfH, -, hoccH, hbH, -, hleH, hevH⟩ :=
    fresh_fold .transaction s₀.held [] n₂ []
  rw [initial_eq hfr]
  simp only [initL, initK, initS, initH, hst, freshAll_nil, hf, hfS, hfH, List.nil_append,
    List.append_nil]
  have hlo : localsOcc ⟨ls.toArray, []⟩ fr = createdOf evs₁ := by simpa [localsOcc] using hocc
  have hframes : framesOcc [⟨ls.toArray, []⟩] [fr] = createdOf evs₁ := by
    rw [← hlo]; simp [framesOcc, frame_occurrences, IdTree.occurrencesAll]
  have hocc' : Shadow.occurrences ⟨[⟨ls.toArray, []⟩], sts, hts, n₃⟩ s₀ =
      createdOf evs₁ ++ createdOf e₂ ++ createdOf e₃ := by
    rw [shadow_occurrences, extOcc, hfr]
    simp only [hframes, hoccS, hoccH, List.append_assoc]
  have hids : borrowedIds (evs₁ ++ e₂ ++ e₃ ++ initRefs ⟨ls.toArray, []⟩ fr) =
      (initParams ⟨ls.toArray, []⟩ fr).map (·.1) := by
    have h1 := borrowedIds_of_created (evs := evs₁) fun e he => by
      obtain ⟨i, w, h, -⟩ := hev e he; exact ⟨i, w, _, h⟩
    have h2 := borrowedIds_of_created (evs := e₂) fun e he => by
      obtain ⟨i, w, h, -⟩ := hevS e he; exact ⟨i, w, _, h⟩
    have h3 := borrowedIds_of_created (evs := e₃) fun e he => by
      obtain ⟨i, w, h, -⟩ := hevH e he; exact ⟨i, w, _, h⟩
    simp only [borrowedIds_append, h1, h2, h3, borrowedIds_initRefs, List.nil_append]
  rintro ⟨xi, xk⟩ hx
  rw [hocc'] at hx
  simp only [keys_append, List.mem_append] at hx
  -- Every identity is below the counter; the locals' are below `n₁`.
  have hlt : xi < n₃ := by
    rcases hx with (hx | hx) | hx
    · obtain ⟨o, ho, h1, -⟩ := mem_keys_iff.1 hx
      have := hb o ho
      simp only at h1
      omega
    · obtain ⟨o, ho, h1, -⟩ := mem_keys_iff.1 hx
      have := hbS o ho
      simp only at h1
      omega
    · obtain ⟨o, ho, h1, -⟩ := mem_keys_iff.1 hx
      have := hbH o ho
      simp only at h1
      omega
  refine ⟨hlt, fun hxb => ?_⟩
  rw [hids] at hxb
  obtain ⟨⟨d, r⟩, hp, hd⟩ := List.mem_map.1 hxb
  simp only at hd
  subst hd
  have hpo := tf_initParams_occ hp
  rw [hlo] at hpo
  have hd1 := (hb _ hpo).2
  simp only at hd1
  rcases hx with (hx | hx) | hx
  · obtain ⟨o, ho, h1, h2⟩ := mem_keys_iff.1 hx
    have he := tf_fst_unique hnd ho hpo h1
    subst he
    refine ⟨r, none, h2.symm, ?_⟩
    simp only [List.mem_append, initRefs, List.mem_map]
    exact Or.inr (Or.inl ⟨_, hp, rfl⟩)
  · obtain ⟨o, ho, h1, -⟩ := mem_keys_iff.1 hx
    have := hbS o ho
    simp only at h1
    omega
  · obtain ⟨o, ho, h1, -⟩ := mem_keys_iff.1 hx
    have := hbH o ho
    simp only at h1
    omega


/-! ## Natives -/

/-- The arguments of a call of a native that is not modeled hold no
reference. -/
private theorem tf_native_args {P : Program} {s s' : State} {f : FunctionRef}
    {args views : List Value} (h : step P s = .callNative s' f args views)
    (hshape : P.nativeShape? f = none) :
    ∀ a ∈ args, ∀ w ∈ subvalues a, w.isRef = false := by
  obtain ⟨fr, rest, n, -, -, -, -, hfacts⟩ := step_callNative h
  obtain ⟨ptys, hpt, hconf⟩ := hfacts.params
  intro a ha w hw
  obtain ⟨t, ht, hc⟩ := conformsAll_mem hconf a ha
  exact conforms_plain hc ((hfacts.values hshape).1 _ hpt t ht) w hw



/-- A node of a tree that fits a value is an occurrence. -/
private theorem tf_at_occ : ∀ (q : Path) {t u : IdTree} {v : Value}, t.fits v = true →
    t.at? q = some u → ∃ w, (u.id, w) ∈ t.occurrences v
  | [], t, u, v, _, h => by
    simp only [IdTree.at?, Option.some.injEq] at h
    subst h
    exact ⟨v, tf_root_occ _ v⟩
  | i :: p, t, u, v, hf, h => by
    obtain ⟨id, cs⟩ := t
    simp only [IdTree.at?, IdTree.children] at h
    split at h
    · next c hc =>
      simp only [IdTree.fits] at hf
      have hlen := fitsAll_length hf
      have hi : i < v.children.length := by
        have := (List.getElem?_eq_some_iff.1 hc).1
        omega
      obtain ⟨cv, hcv⟩ : ∃ cv, v.children[i]? = some cv := ⟨_, List.getElem?_eq_getElem hi⟩
      obtain ⟨c', hc', hfc⟩ := fitsAll_get hf hcv
      rw [hc, Option.some.injEq] at hc'
      subst hc'
      obtain ⟨w, hw⟩ := tf_at_occ p hfc h
      exact ⟨w, List.mem_cons_of_mem _ (occ_of_zip (zip_of_getElem? hc hcv) _ hw)⟩
    · cases h

/-- **The shadow after a native call**: it replaces the arguments by the
results, whose nodes have new identities or the identities of the arguments'
nodes, which are values. -/
private theorem native_results {P : Program} {ν : NativeFlows} {s s₁ s' : State} {h h' : Shadow}
    {E evs : List Event} {f : FunctionRef} {args views vals writes : List Value}
    {places : List NativePlace}
    (hI : RefTraceInv P s h E) (hstep : step P s = .callNative s₁ f args views)
    (hshape : P.nativeShape? f = none)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s')
    (hn : Shadow.nativeStep P ν s f args views vals writes places (.running s') h =
      some (.running h', evs)) :
    ∃ fr rest g hs ts n, s.frames = fr :: rest ∧ h.frames = g :: hs ∧
      s'.frames = { fr with stack := vals.reverse ++ fr.stack.drop n, pc := fr.pc + 1 } :: rest ∧
      h'.frames = { g with stack := ts.reverse ++ g.stack.drop n } :: hs ∧
      extOcc h' s' = extOcc h s ∧ h.fresh ≤ h'.fresh ∧ IdTree.fitsAll ts vals = true ∧
      borrowedIds evs = [] ∧
      ∀ o ∈ IdTree.occurrencesAll ts vals, (h.fresh ≤ o.1 ∧ o.1 < h'.fresh) ∨
        ∃ w, (o.1, w) ∈ h.occurrences s ∧ w.isRef = false := by
  have hcont : (resumeNative P s₁ f args (.returned vals writes places)).continues = true := by
    rw [ho]; rfl
  have hargsR := tf_native_args hstep hshape
  rw [← ho] at hn
  obtain ⟨rfl, rfl, rfl, g, hs, info, hh, -, -, hpost, hevs, -⟩ := nativeStep_some hstep hshape hn
  obtain ⟨fr, rest, n, hsf, hnl, hargs, rfl, hfacts⟩ := step_callNative hstep
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  have hv := hfacts.values hshape
  have hs' := (resume_running (fr := { fr with stack := fr.stack.drop n }) (rest := rest) rfl hshape
    (noRef_of_conformsAll hconfa (hv.1 _ hpt)) hv.2 hcont).1
  rw [ho, Outcome.running.injEq] at hs'
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  have hg : g.fits fr = true := by
    have := hI.pre.fits
    rw [shadow_fits, hsf, hh, framesFit_cons] at this
    simp only [Bool.and_eq_true] at this
    exact this.1.1
  have hfa := frame_stack_fits hg
  have hglen := fitsAll_length hfa
  obtain ⟨ts, nR, evsR, hfold, hfitR, hoccR, -, hbR, -, hle, -⟩ :=
    results_fold (resultIdOf (ν f args vals) (g.stack.take args.length).reverse)
      (.native f info.moduleId args) vals.zipIdx [] h.fresh []
  have heq : nativeResults ν f args vals (g.stack.take args.length).reverse info.moduleId
      h.fresh = (ts, nR, evsR) := by
    simp [nativeResults, hfold]
  have hzv : vals.zipIdx.map (·.1) = vals := List.zipIdx_map_fst ..
  rw [hzv] at hfitR hoccR
  simp only [nativePost, ShadowPost.running.injEq, heq] at hpost
  have hnb : borrowedIds evs = [] := by rw [hevs]; exact nativeEvents_borrowedIds
  subst hpost hs'
  refine ⟨fr, rest, g, hs, ts, n, hsf, hh, rfl, by rw [hargl], rfl, hle, hfitR, hnb, ?_⟩
  intro o ho
  rcases List.mem_append.1 (hoccR.subset ho) with ho' | ho'
  · right
    obtain ⟨⟨vr, j⟩, hvr, ho'⟩ := List.mem_flatMap.1 ho'
    obtain ⟨y, hy, hyo⟩ := List.mem_filterMap.1 ho'
    cases hi : resultIdOf (ν f args vals) (g.stack.take args.length).reverse j y.1 with
    | none => rw [hi] at hyo; cases hyo
    | some i =>
      rw [hi, Option.map_some, Option.some.injEq] at hyo
      subst hyo
      simp only [resultIdOf] at hi
      split at hi
      · next k q _ =>
        simp only [argIdAt, Option.map_eq_some_iff, Option.bind_eq_some_iff] at hi
        obtain ⟨u, ⟨t, ht, hu⟩, rfl⟩ := hi
        -- The argument node's occurrence before the step.
        have hk : k < args.length := by
          have := (List.getElem?_eq_some_iff.1 ht).1
          simp only [List.length_reverse, List.length_take] at this
          omega
        obtain ⟨a, ha⟩ : ∃ a, args[k]? = some a := ⟨_, List.getElem?_eq_getElem hk⟩
        have hz : (t, a) ∈ (g.stack.take args.length).reverse.zip args :=
          zip_of_getElem? ht ha
        rw [hargl] at hz
        rw [hargs] at hz
        have hz' := zip_rev_take hglen hz
        have hft := fitsAll_of_zip hfa hz'
        obtain ⟨w, hw⟩ := tf_at_occ q hft hu
        refine ⟨w, ?_, hargsR a (List.mem_of_getElem? ha) w (occurrences_mem hft hw)⟩
        rw [shadow_occurrences, hh, hsf, framesOcc_cons, frame_occurrences]
        exact List.mem_append_left _ (List.mem_append_left _
          (List.mem_append_right _ (occ_of_zip hz' _ hw)))
      · cases hi
  · exact Or.inl (hbR o ho')

/-- **A native call keeps the identities of references apart**: its results
are values, of new identities or of the identities of its arguments' nodes,
which are values too. -/
private theorem idsKind_native {P : Program} {ν : NativeFlows} {s s₁ s' : State} {h h' : Shadow}
    {E evs : List Event} {f : FunctionRef} {args views vals writes : List Value}
    {places : List NativePlace}
    (hI : RefTraceInv P s h E) (hK : IdsKind s h E) (hstep : step P s = .callNative s₁ f args views)
    (hshape : P.nativeShape? f = none)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s')
    (hn : Shadow.nativeStep P ν s f args views vals writes places (.running s') h =
      some (.running h', evs)) :
    IdsKind s' h' (E ++ evs) := by
  obtain ⟨fr, rest, g, hs, ts, n, hsf, hh, hs'f, hh'f, hext, hle, hfitR, hnb, hres⟩ :=
    native_results hI hstep hshape ho hn
  have hg : g.fits fr = true := by
    have := hI.pre.fits
    rw [shadow_fits, hsf, hh, framesFit_cons] at this
    simp only [Bool.and_eq_true] at this
    exact this.1.1
  have hfa := frame_stack_fits hg
  have hpre : ∀ o : Nat × Value, o ∈ framesOcc h.frames s.frames →
      (o.1, o.2.key) ∈ keys (h.occurrences s) :=
    fun o ho => by rw [shadow_occurrences]; exact mem_keys (List.mem_append_left _ ho)
  have hkept : ∀ o : Nat × Value, (o.1, o.2.key) ∈ keys (h.occurrences s) →
      o.1 < h'.fresh ∧ (o.1 ∈ borrowedIds (E ++ evs) → ∃ r par, o.2.key = (Value.ref r).key ∧
        Event.borrowed o.1 r par ∈ E ++ evs) := by
    intro o ho
    obtain ⟨hlt, hk⟩ := hK _ ho
    refine ⟨Nat.lt_of_lt_of_le hlt hle, fun hb => ?_⟩
    rw [borrowedIds_append, hnb, List.append_nil] at hb
    obtain ⟨r, par, h1, h2⟩ := hk hb
    exact ⟨r, par, h1, List.mem_append_left _ h2⟩
  rintro ⟨xi, xk⟩ hx
  obtain ⟨o, ho, rfl, rfl⟩ := mem_keys_iff.1 hx
  rw [shadow_occurrences, List.mem_append, hext] at ho
  rcases ho with ho | ho
  · -- In the frames.
    have hlenR : ts.reverse.length = vals.reverse.length := by simp [fitsAll_length hfitR]
    rw [hh'f, hs'f, framesOcc_cons, frame_occurrences, localsOcc_stack,
      occurrencesAll_append hlenR] at ho
    rw [hh, hsf] at hpre
    simp only [framesOcc_cons, frame_occurrences, List.mem_append] at ho hpre
    rcases ho with (ho | ho | ho) | ho
    · exact hkept o (hpre o (Or.inl (Or.inl ho)))
    · -- A result's node.
      rcases hres o ((occurrencesAll_reverse hfitR).subset ho) with ⟨hlo, hhi⟩ | ⟨w, hw, hwr⟩
      · refine ⟨hhi, fun hb => ?_⟩
        rw [borrowedIds_append, hnb, List.append_nil] at hb
        have := hI.below o.1 hb
        omega
      · obtain ⟨hlt, hk⟩ := hK _ (mem_keys hw)
        refine ⟨Nat.lt_of_lt_of_le hlt hle, fun hb => ?_⟩
        rw [borrowedIds_append, hnb, List.append_nil] at hb
        obtain ⟨r, -, h1, -⟩ := hk hb
        simp only at h1
        rw [key_ref h1] at hwr
        cases hwr
    · exact hkept o (hpre o (Or.inl (Or.inr (by
        rw [stack_split hfa n]; exact List.mem_append_right _ ho))))
    · exact hkept o (hpre o (Or.inr ho))
  · -- Outside the frames.
    refine hkept o ?_
    rw [shadow_occurrences]
    exact mem_keys (List.mem_append_right _ ho)

/-- **A call of a modeled native keeps the identities of references apart**:
its outputs' nodes have new identities, or those of nodes it saw, which are
no references; its new references have new identities. -/
private theorem idsKind_modeled {P : Program} {ν : NativeFlows} {s s₁ s' : State}
    {h h' : Shadow} {E evs : List Event} {f : FunctionRef} {args views vals writes : List Value}
    {places : List NativePlace} {k : NativeShape}
    (hI : RefTraceInv P s h E) (hK : IdsKind s h E) (hstep : step P s = .callNative s₁ f args views)
    (hshape : P.nativeShape? f = some k)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s')
    (hn : Shadow.nativeStep P ν s f args views vals writes places (.running s') h =
      some (.running h', evs)) :
    IdsKind s' h' (E ++ evs) := by
  obtain ⟨fr, rest, n, g, hs, F, results, resultTrees, ts, hsf, hh, hnl, hng, hargs, hwb, hfold,
    hfitR, hfitT, -, -, hF, hlt, hF', -, F₀, hF₀, hF₀F, hge, horigT, horigR⟩ :=
    nativeStep_decomp hshape hstep ho hn
  have hfit := hI.pre.fits
  rw [shadow_fits, hh, hsf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
  obtain ⟨⟨hg, hrest⟩, hext⟩ := hfit
  have hfa := frame_stack_fits hg
  obtain ⟨-, hdropf⟩ := fitsAll_take_drop n hfa
  have hlR := fitsAll_length hfitR
  have hfit₁ : Shadow.fits
      { h with
        frames := { g with stack := resultTrees.reverse ++ g.stack.drop n } :: hs
        fresh := F }
      { s with
        frames := { fr with
          stack := results.reverse ++ fr.stack.drop n
          pc := fr.pc + 1 } :: rest } = true := by
    rw [fits_head (h' := { h with
        frames := { g with stack := resultTrees.reverse ++ g.stack.drop n } :: hs
        fresh := F })
      (s' := { s with
        frames := { fr with
          stack := results.reverse ++ fr.stack.drop n
          pc := fr.pc + 1 } :: rest }) rfl rfl h s rfl rfl rfl rfl]
    simp only [Bool.and_eq_true]
    refine ⟨⟨frame_fits_stack hg ?_ _, hrest⟩, hext⟩
    rw [fitsAll_append (by simp [hlR]), fitsAll_reverse hfitR, hdropf]
    rfl
  have hids : ((fr :: rest).map (·.id)).Nodup := by rw [← hsf]; exact hI.pre.inv.frameIds
  obtain ⟨-, -, -, -, -, -, hall⟩ := writes_ghost (s₀ := s) _ ts writes hfit₁
    (by simpa using hids) hI.pre.inv.resources (by simp [hsf]) rfl
    (writesAgree_native hstep (by rw [hargs]; simp; omega)
      (root_eq (by simp [hsf]) rfl rfl)) hwb hfold hfitT
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  have hseenOk : ∀ (seen : List IdTree) (kk : Nat) (q : Path) (i : Nat),
      seenTrees h s args (g.stack.take n).reverse = some seen → seenIdAt seen kk q = some i →
      i < h.fresh ∧ i ∉ borrowedIds E := by
    intro seen kk q i hseen hi
    rw [← hargl] at hseen
    obtain ⟨w, hw, hwr⟩ := seen_nodes hI.pre hI.typed hshape hstep hh hseen kk q i hi
    obtain ⟨hlt', hb⟩ := hK _ (mem_keys hw)
    refine ⟨hlt', fun hbE => ?_⟩
    obtain ⟨r, par, h1, -⟩ := hb hbE
    simp only at h1
    rw [key_ref h1] at hwr
    cases hwr
  have hnb := (native_borrowedIds hn).2
  have hold : ∀ x ∈ keys (h.occurrences s), x.1 < h'.fresh ∧
      (x.1 ∈ borrowedIds (E ++ evs) → ∃ r par, x.2 = (Value.ref r).key ∧
        Event.borrowed x.1 r par ∈ E ++ evs) := by
    intro x hx
    obtain ⟨hlt', hb⟩ := hK x hx
    refine ⟨by rw [hF']; omega, fun hbE => ?_⟩
    rw [borrowedIds_append, List.mem_append] at hbE
    rcases hbE with hbE | hbE
    · obtain ⟨r, par, h1, h2⟩ := hb hbE
      exact ⟨r, par, h1, List.mem_append_left _ h2⟩
    · have := hnb _ hbE; omega
  have hout : ∀ o : Nat × Value, ((h.fresh ≤ o.1 ∧ o.1 < F₀) ∨
      ∃ seen kk q, seenTrees h s args (g.stack.take n).reverse = some seen ∧
        seenIdAt seen kk q = some o.1) → o.1 < h'.fresh ∧ o.1 ∉ borrowedIds (E ++ evs) := by
    intro o ho
    rcases ho with ⟨h1, h2⟩ | ⟨seen, kk, q, hseen, hq⟩
    · refine ⟨by rw [hF']; omega, fun hb => ?_⟩
      rw [borrowedIds_append, List.mem_append] at hb
      rcases hb with hb | hb
      · have := hI.below _ hb; omega
      · obtain ⟨r, par, hb'⟩ := mem_borrowedIds.1 hb
        have := hge _ _ _ hb'
        omega
    · obtain ⟨h1, h2⟩ := hseenOk seen kk q o.1 hseen hq
      refine ⟨by rw [hF']; omega, fun hb => ?_⟩
      rw [borrowedIds_append, List.mem_append] at hb
      rcases hb with hb | hb
      · exact h2 hb
      · have := hnb _ hb; omega
  have hpre_occ : h.occurrences s = g.occurrences fr ++ framesOcc hs rest ++ extOcc h s :=
    occ_head hh hsf h s rfl rfl rfl rfl
  have hold' : ∀ o ∈ h.occurrences s, o.1 < h'.fresh ∧
      (o.1 ∈ borrowedIds (E ++ evs) → ∃ r par, o.2.key = (Value.ref r).key ∧
        Event.borrowed o.1 r par ∈ E ++ evs) := fun o ho => hold _ (mem_keys ho)
  intro x hx
  rcases hall x hx with hx | ⟨y, hy, hx⟩
  · obtain ⟨o, ho', h1, h2⟩ := mem_keys_iff.1 hx
    obtain ⟨x1, x2⟩ := x
    simp only at h1 h2
    subst h1 h2
    rw [occ_head (h' := { h with
        frames := { g with stack := resultTrees.reverse ++ g.stack.drop n } :: hs
        fresh := F })
      (s' := { s with
        frames := { fr with
          stack := results.reverse ++ fr.stack.drop n
          pc := fr.pc + 1 } :: rest }) rfl rfl h s rfl rfl rfl rfl, frame_occurrences,
      localsOcc_stack, occurrencesAll_append (by simp [hlR])] at ho'
    simp only [List.mem_append] at ho'
    rcases ho' with ((hl | hR | hD) | hfs) | hx'
    · exact hold' o (by rw [hpre_occ, frame_occurrences]; simp [hl])
    · have hR' := (occurrencesAll_reverse hfitR).subset hR
      rcases horigR with horigR | ⟨d, b, t, rfl, rfl, hb⟩
      · obtain ⟨h1, h2⟩ := hout o (horigR o hR')
        exact ⟨h1, fun hb => absurd hb h2⟩
      · simp [IdTree.occurrencesAll, IdTree.occurrences] at hR'
        obtain ⟨rfl, rfl⟩ := hR'
        exact ⟨by rw [hF']; exact hlt _ _ _ hb, fun _ => ⟨b, some t, rfl,
          List.mem_append_right _ hb⟩⟩
    · exact hold' o (by
        rw [hpre_occ, frame_occurrences, stack_split hfa n]
        simp [hD])
    · exact hold' o (by rw [hpre_occ]; simp [hfs])
    · exact hold' o (by rw [hpre_occ]; simp [hx'])
  · obtain ⟨o, ho', h1, h2⟩ := mem_keys_iff.1 hx
    obtain ⟨x1, x2⟩ := x
    simp only at h1 h2
    subst h1 h2
    have hoT := occ_of_zip hy o ho'
    obtain ⟨h1, h2⟩ := hout o (horigT o hoT)
    exact ⟨h1, fun hb => absurd hb h2⟩

/-- One traced step keeps the identities of references apart. -/
private theorem tf_idsKind_step {P : Program} {N : NativeRel} {ν : NativeFlows} {s s' : State}
    {h h' : Shadow} {o : Outcome} {post : ShadowPost} {E evs : List Event}
    (hI : RefTraceInv P s h E) (hK : IdsKind s h E) (hT : TracedStep P N ν s h o post evs)
    (ho : o = .running s') (hp : post = .running h') : IdsKind s' h' (E ++ evs) := by
  subst hp
  cases hT with
  | core hstep hc =>
    subst ho
    exact idsKind_core hI hK hstep hc
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      rw [ho] at hn
      exact idsKind_modeled hI hK hstep hshape ho hn
    rw [ho] at hn
    exact idsKind_native hI hK hstep hshape ho hn

/-- **The identities of references are kept apart along every trace** from an
admissible initial state. -/
private theorem idsKind_trace {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s : State}
    {h : Shadow} {E : List Event} (h0 : AdmissibleInitial P s₀)
    (hT : Trace P N ν s₀ s h E) : IdsKind s h E := by
  induction hT with
  | start => exact idsKind_initial h0
  | step hT₀ hS ih => exact tf_idsKind_step (refTraceInv_trace h0 hT₀) ih hS rfl rfl

/-! ## Where the trees come from -/

/-- Where a tree in a tracked slot after a step comes from: a slot that was
tracked before the step, a new identity, or the identity of a value that is
no reference. -/
private def Origin (P : Program) (s : State) (h : Shadow) (t : IdTree) : Prop :=
  (∃ g fr, (g, fr) ∈ h.frames.zip s.frames ∧ t ∈ g.stack) ∨
  (∃ g fr j, (g, fr) ∈ h.frames.zip s.frames ∧ g.locals[j]? = some (some t) ∧
    modelAvail P fr.function fr.pc j = .available) ∨
  h.fresh ≤ t.id ∨
  ∃ w, (t.id, w) ∈ h.occurrences s ∧ w.isRef = false

/-- The trees in the tracked slots after a step, from `s` to `s'`, come from
before it. -/
private def Origins (P : Program) (s s' : State) (h h' : Shadow) : Prop :=
  ∀ g' fr', (g', fr') ∈ h'.frames.zip s'.frames →
    (∀ t ∈ g'.stack, Origin P s h t) ∧
    ∀ j t, g'.locals[j]? = some (some t) → modelAvail P fr'.function fr'.pc j = .available →
      Origin P s h t

/-- A frame of a fitting shadow fits its frame. -/
theorem tf_pair_fits {h : Shadow} {s : State} (hfit : h.fits s = true) {g : ShadowFrame}
    {fr : Frame} (hz : (g, fr) ∈ h.frames.zip s.frames) : g.fits fr = true := by
  rw [shadow_fits] at hfit
  simp only [framesFit, Bool.and_eq_true, List.all_eq_true] at hfit
  exact hfit.1.2 _ hz

/-- The occurrences of a frame are the shadow's. -/
theorem tf_pair_occ {h : Shadow} {s : State} {g : ShadowFrame} {fr : Frame}
    (hz : (g, fr) ∈ h.frames.zip s.frames) {o : Nat × Value} (ho : o ∈ g.occurrences fr) :
    o ∈ h.occurrences s := by
  rw [shadow_occurrences]
  exact List.mem_append_left _ (List.mem_flatMap.2 ⟨(g, fr), hz, ho⟩)

/-- A tree on a fitting operand stack is paired with a value. -/
theorem tf_stack_pair {g : ShadowFrame} {fr : Frame} (hg : g.fits fr = true) {t : IdTree}
    (ht : t ∈ g.stack) : ∃ v, (t, v) ∈ g.stack.zip fr.stack := by
  obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 ht
  have hlen := fitsAll_length (frame_stack_fits hg)
  have hkl : k < fr.stack.length := by
    have := (List.getElem?_eq_some_iff.1 hk).1
    omega
  exact ⟨_, zip_of_getElem? hk (List.getElem?_eq_getElem hkl)⟩

/-- A tree in a fitting local is paired with the local's value. -/
theorem tf_local_pair {g : ShadowFrame} {fr : Frame} (hg : g.fits fr = true) {j : Nat}
    {t : IdTree} (ht : g.locals[j]? = some (some t)) :
    ∃ v, fr.locals[j]? = some (some v) ∧ (t.id, v) ∈ g.occurrences fr := by
  rw [frame_fits] at hg
  simp only [Bool.and_eq_true, beq_iff_eq] at hg
  obtain ⟨⟨hsize, hslots⟩, -⟩ := hg
  have hjl : j < fr.locals.size := by
    rw [← hsize]; exact (Array.getElem?_eq_some_iff.1 ht).1
  obtain ⟨ov, hov⟩ : ∃ ov, fr.locals[j]? = some ov := ⟨_, Array.getElem?_eq_getElem hjl⟩
  have hz : (some t, ov) ∈ g.locals.toList.zip fr.locals.toList := by
    refine zip_of_getElem? (k := j) ?_ ?_
    · rw [Array.getElem?_toList]; exact ht
    · rw [Array.getElem?_toList]; exact hov
  have hsl : ShadowFrame.slotFits (some t) ov = true := by
    simp only [slotsFit, List.all_eq_true] at hslots
    exact hslots _ hz
  cases ov with
  | none => simp [ShadowFrame.slotFits] at hsl
  | some v =>
    refine ⟨v, hov, ?_⟩
    rw [frame_occurrences]
    refine List.mem_append_left _ (List.mem_flatMap.2 ⟨_, hz, ?_⟩)
    simpa [ShadowFrame.slotOccurrences] using tf_root_occ t v

/-- **The tracked references after a step whose trees come from before it**
were tracked before it, or the step borrowed them. -/
private theorem back_of_origins {P : Program} {s s' : State} {h h' : Shadow} {E evs : List Event}
    (hI : RefTraceInv P s h E) (hI' : RefTraceInv P s' h' (E ++ evs)) (hK : IdsKind s h E)
    (hO : Origins P s s' h h') {i : Nat} {r : Reference} (ht : Tracked P s' h' i r) :
    Tracked P s h i r ∨ ∃ par, Event.borrowed i r par ∈ evs := by
  obtain ⟨par, hb⟩ := hI'.refs i r (tracked_occ hI'.pre.fits ht)
  rcases List.mem_append.1 hb with hb | hb
  · left
    have hiE : i ∈ borrowedIds E := mem_borrowedIds.2 ⟨r, par, hb⟩
    obtain ⟨g', fr', hz', htr⟩ := ht
    obtain ⟨t, rfl, horig⟩ : ∃ t : IdTree, t.id = i ∧ Origin P s h t := by
      rcases htr with ⟨t, rfl, hm⟩ | ⟨j, ⟨t, rfl, hgl, -⟩, hav⟩
      · exact ⟨t, rfl, (hO g' fr' hz').1 t (List.of_mem_zip hm).1⟩
      · exact ⟨t, rfl, (hO g' fr' hz').2 j t hgl hav⟩
    -- What a slot holding the tree before the step held is the reference.
    have hval : ∀ v, (t.id, v) ∈ h.occurrences s → v = .ref r := by
      intro v hv
      obtain ⟨-, hk⟩ := hK (t.id, v.key) (mem_keys hv)
      obtain ⟨r0, par0, h1, h2⟩ := hk hiE
      rw [key_ref h1, (borrowed_unique hI.unique hb h2).1]
    have hfit := hI.pre.fits
    rcases horig with ⟨g, fr, hz, hmem⟩ | ⟨g, fr, j, hz, hgl, hav⟩ | hfr | ⟨w, hw, hwr⟩
    · have hg := tf_pair_fits hfit hz
      obtain ⟨v, hv⟩ := tf_stack_pair hg hmem
      have hocc : (t.id, v) ∈ h.occurrences s := by
        refine tf_pair_occ hz ?_
        rw [frame_occurrences]
        exact List.mem_append_right _ (occ_of_zip hv _ (tf_root_occ t v))
      rw [hval v hocc] at hv
      exact ⟨g, fr, hz, Or.inl ⟨t, rfl, hv⟩⟩
    · have hg := tf_pair_fits hfit hz
      obtain ⟨v, hv, hocc⟩ := tf_local_pair hg hgl
      rw [hval v (tf_pair_occ hz hocc)] at hv
      exact ⟨g, fr, hz, Or.inr ⟨j, ⟨t, rfl, hgl, hv⟩, hav⟩⟩
    · exact absurd (hI.below _ hiE) (by omega)
    · rw [hval w hw] at hwr
      cases hwr
  · exact Or.inr ⟨par, hb⟩


/-! ## Suspended frames are at calls -/

/-- The suspended frames are at reachable calls or closure calls. -/
private def AtCalls (P : Program) (s : State) : Prop :=
  ∀ fr ∈ s.frames.tail, ∃ info code instr, P.function? fr.function = some info ∧
    info.definition.code = some code ∧ CodeReachable code.code fr.pc ∧
    code.code[fr.pc]? = some instr ∧
    (((∃ hd, instr = .call hd) ∨ ∃ i, instr = .callGeneric i) ∨ ∃ sig, instr = .callClosure sig)

/-- What the step from `s`, whose shadow is `h` and whose events so far are
`E`, of instruction `instr` of the current frame `fr`, has to go on. -/
private structure StepCtx (P : Program) (s : State) (h : Shadow) (E : List Event) (fr : Frame)
    (rest : List Frame) (g : ShadowFrame) (hs : List ShadowFrame) (info : FunctionInfo)
    (code : CodeUnit) (instr : Bytecode) : Prop where
  sf : s.frames = fr :: rest
  hh : h.frames = g :: hs
  trace : RefTraceInv P s h E
  act : ActiveAt P (modelTypes P) (modelAvail P) fr info code
  hi : code.code[fr.pc]? = some instr
  cert : Certified P (modelTypes P) (modelAvail P)
  calls : AtCalls P s

namespace StepCtx

variable {P : Program} {s : State} {h : Shadow} {E : List Event} {fr : Frame} {rest : List Frame}
  {g : ShadowFrame} {hs : List ShadowFrame} {info : FunctionInfo} {code : CodeUnit}
  {instr : Bytecode}

private theorem head (C : StepCtx P s h E fr rest g hs info code instr) :
    (g, fr) ∈ h.frames.zip s.frames := by
  rw [C.hh, C.sf]; exact List.mem_cons_self ..

private theorem below (C : StepCtx P s h E fr rest g hs info code instr) {g₂ : ShadowFrame}
    {fr₂ : Frame} (hz : (g₂, fr₂) ∈ hs.zip rest) : (g₂, fr₂) ∈ h.frames.zip s.frames := by
  rw [C.hh, C.sf]; exact List.mem_cons_of_mem _ hz

private theorem fits (C : StepCtx P s h E fr rest g hs info code instr) : g.fits fr = true :=
  tf_pair_fits C.trace.pre.fits C.head

private theorem stack_origin (C : StepCtx P s h E fr rest g hs info code instr) {t : IdTree}
    (ht : t ∈ g.stack) : Origin P s h t :=
  Or.inl ⟨g, fr, C.head, ht⟩

private theorem local_origin (C : StepCtx P s h E fr rest g hs info code instr) {j : Nat} {t : IdTree}
    (ht : g.locals[j]? = some (some t)) (hav : modelAvail P fr.function fr.pc j = .available) :
    Origin P s h t :=
  Or.inr (Or.inl ⟨g, fr, j, C.head, ht, hav⟩)

/-- The availability analysis, at the step's instruction. -/
private theorem avail (C : StepCtx P s h E fr rest g hs info code instr) :
    (∀ j, instr.readsLocal = some j → modelAvail P fr.function fr.pc j = .available) ∧
      ∀ pc' ∈ successors code.code fr.pc, ∀ j,
        Availability.Refines (availabilityAfter instr (modelAvail P fr.function fr.pc) j)
          (modelAvail P fr.function pc' j) := by
  obtain ⟨-, -, -, hcode⟩ := C.cert.availability fr.function info code C.act.fn C.act.body
  exact hcode fr.pc instr C.act.reachable C.hi

/-- A local available after a step to a successor was available after the
instruction. -/
private theorem succ_avail (C : StepCtx P s h E fr rest g hs info code instr) {pc' j : Nat}
    (hpc : pc' ∈ successors code.code fr.pc)
    (hj : modelAvail P fr.function pc' j = .available) :
    availabilityAfter instr (modelAvail P fr.function fr.pc) j = .available :=
  refines_available (C.avail.2 pc' hpc j) hj

/-- **A step that changes only the current frame**, keeping its function. -/
private theorem topOnly (C : StepCtx P s h E fr rest g hs info code instr) {s' : State} {h' : Shadow}
    {fr' : Frame} {g' : ShadowFrame} (hs' : s'.frames = fr' :: rest)
    (hh' : h'.frames = g' :: hs) (hfun : fr'.function = fr.function)
    (hstk : ∀ t ∈ g'.stack, Origin P s h t)
    (hloc : ∀ j t, g'.locals[j]? = some (some t) →
      modelAvail P fr.function fr'.pc j = .available → Origin P s h t) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  refine ⟨fun g₂ fr₂ hz => ?_, fun f hf => C.calls f (by rw [C.sf]; rw [hs'] at hf; exact hf)⟩
  rw [hh', hs', List.zip_cons_cons, List.mem_cons] at hz
  rcases hz with he | hz
  · simp only [Prod.mk.injEq] at he
    obtain ⟨rfl, rfl⟩ := he
    exact ⟨hstk, fun j t ht hav => hloc j t ht (by rw [← hfun]; exact hav)⟩
  · exact ⟨fun t ht => Or.inl ⟨g₂, fr₂, C.below hz, ht⟩,
      fun j t ht hav => Or.inr (Or.inl ⟨g₂, fr₂, j, C.below hz, ht, hav⟩)⟩

/-- The locals of a step that keeps them, to a successor, for an instruction
that keeps their availability. -/
private theorem keptLocals (C : StepCtx P s h E fr rest g hs info code instr) {g' : ShadowFrame}
    {pc' : Nat} (hloc : g'.locals = g.locals) (hpc : pc' ∈ successors code.code fr.pc)
    (hid : availabilityAfter instr (modelAvail P fr.function fr.pc) =
      modelAvail P fr.function fr.pc) :
    ∀ j t, g'.locals[j]? = some (some t) →
      modelAvail P fr.function pc' j = .available → Origin P s h t := by
  intro j t ht hav
  rw [hloc] at ht
  have := C.succ_avail hpc hav
  rw [hid] at this
  exact C.local_origin ht this

end StepCtx

/-- The root of a new tree has the first new identity. -/
theorem tf_fresh_id (c : Creator) (v : Value) (n : Nat) : (IdTree.fresh c v n).1.id = n := by
  have h := (fresh_spec c v n).2.1
  generalize (IdTree.fresh c v n).1 = t at h ⊢
  obtain ⟨i, cs⟩ := t
  rw [subvalues_eq] at h
  simp only [IdTree.occurrences, List.length_cons, List.range'_succ, List.zip_cons_cons,
    List.cons.injEq, Prod.mk.injEq] at h
  exact h.1.1


/-! ## Instructions that push new trees -/

/-- The instructions whose annotation pops slots off the current frame's
operand stack and pushes new trees, keeping its locals. -/
private def _root_.MoveBytecode.Bytecode.PushesNew : Bytecode → Bool
  | .pop | .brTrue _ | .brFalse _ | .branch _ | .nop => true
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => true
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => true
  | .ldTrue | .ldFalse | .ldConst _ => true
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => true
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => true
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => true
  | .lt | .gt | .le | .ge | .shl | .shr | .negate | .eq | .neq => true
  | .pack _ | .packVariant _ | .vecPack _ _ | .exists_ _ => true
  | .testVariant _ | .vecLen _ | .freezeRef => true
  | .mutBorrowLoc _ | .immBorrowLoc _ | .mutBorrowField _ | .immBorrowField _ => true
  | .mutBorrowVariantField _ | .immBorrowVariantField _ | .vecImmBorrow _ | .vecMutBorrow _ => true
  | .mutBorrowGlobal _ | .immBorrowGlobal _ => true
  | .abort | .abortMsg | .ret | .copyLoc _ | .moveLoc _ | .stLoc _ | .call _ => false
  | .unpack _ | .unpackVariant _ | .vecUnpack _ _ | .readRef | .writeRef => false
  | .vecPushBack _ | .vecPopBack _ | .vecSwap _ | .moveFrom _ | .moveTo _ => false
  | .packGeneric _ | .packVariantGeneric _ | .existsGeneric _ => true
  | .testVariantGeneric _ | .mutBorrowFieldGeneric _ | .mutBorrowVariantFieldGeneric _ => true
  | .immBorrowFieldGeneric _ | .immBorrowVariantFieldGeneric _ => true
  | .mutBorrowGlobalGeneric _ | .immBorrowGlobalGeneric _ => true
  | .packClosure _ _ | .packClosureGeneric _ _ => true
  | .callGeneric _ | .callClosure _ => false
  | .unpackGeneric _ | .unpackVariantGeneric _ => false
  | .moveFromGeneric _ | .moveToGeneric _ => false

/-- The shadow after an instruction that pops slots and pushes new trees: the
current frame keeps its locals, and the trees on its operand stack are old
ones or new. -/
private def TopNew (h h' : Shadow) (g : ShadowFrame) (hs : List ShadowFrame) : Prop :=
  ∃ g', h'.frames = g' :: hs ∧ g'.locals = g.locals ∧ ∀ t ∈ g'.stack, t ∈ g.stack ∨ h.fresh ≤ t.id

private theorem topNew_mk {h X : Shadow} {g : ShadowFrame} {hs : List ShadowFrame} {ts : List IdTree}
    (hX : X.frames = { locals := g.locals, stack := ts } :: hs)
    (hts : ∀ u ∈ ts, u ∈ g.stack ∨ h.fresh ≤ u.id) : TopNew h X g hs :=
  ⟨_, hX, rfl, hts⟩

private theorem topNew_borrow {h h' X : Shadow} {g : ShadowFrame} {hs : List ShadowFrame}
    {ts : List IdTree} {e evs : List Event}
    (hm : some (ShadowPost.running X, e) = some (ShadowPost.running h', evs))
    (hX : X.frames = { locals := g.locals, stack := IdTree.node h.fresh [] :: ts } :: hs)
    (hts : ∀ u ∈ ts, u ∈ g.stack) : TopNew h h' g hs := by
  simp only [Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hm
  obtain ⟨rfl, -⟩ := hm
  refine topNew_mk hX fun u hu => ?_
  rcases List.mem_cons.1 hu with rfl | hu
  · exact Or.inr (by simp [IdTree.id])
  · exact Or.inl (hts u hu)

/-- **The annotation of an instruction that pushes new trees.** -/
private theorem exec_topNew {P : Program} {m : ModuleId} {instr : Bytecode} {s : State} {fr : Frame}
    {o : Outcome} {h h' : Shadow} {evs : List Event} {g : ShadowFrame}
    {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (.running h', evs))
    (hk : instr.PushesNew = true) : TopNew h h' g hs := by
  cases instr <;> simp only [Bytecode.PushesNew] at hk <;> (try exact absurd hk (by decide))
  all_goals simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff,
    Option.pure_def, Option.some.injEq, Prod.mk.injEq, guard] at hex
  all_goals (try simp_all [ShadowPost.running.injEq])
  case pop | brTrue | brFalse =>
    obtain ⟨-, rfl, -⟩ := hex
    exact topNew_mk rfl fun u hu => Or.inl (List.mem_of_mem_tail hu)
  case branch | nop =>
    obtain ⟨rfl, -⟩ := hex
    exact topNew_mk rfl fun u hu => Or.inl hu
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldConst | ldTrue | ldFalse =>
    obtain ⟨a, -, rfl, -⟩ := hex
    exact topNew_mk rfl fun u hu => by
      rcases List.mem_cons.1 hu with rfl | hu
      · exact Or.inr (by rw [tf_fresh_id]; exact Nat.le_refl _)
      · exact Or.inl hu
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32
    | castI64 | castI128 | castI256 | not | negate | add | sub | mul | div | mod | bitOr | bitAnd
    | xor | or | and | lt | gt | le | ge | shl | shr =>
    obtain ⟨-, a, -, rfl, -⟩ := hex
    exact topNew_mk rfl fun u hu => by
      rcases List.mem_cons.1 hu with rfl | hu
      · exact Or.inr (by rw [tf_fresh_id]; exact Nat.le_refl _)
      · first
        | exact Or.inl (List.mem_of_mem_tail hu)
        | exact Or.inl (List.mem_of_mem_drop hu)
  case pack | packVariant | vecPack | packGeneric | packVariantGeneric | packClosure
    | packClosureGeneric =>
    obtain ⟨a, -, -, rfl, -⟩ := hex
    exact topNew_mk rfl fun u hu => by
      rcases List.mem_cons.1 hu with rfl | hu
      · exact Or.inr (by simp [IdTree.id])
      · exact Or.inl (List.mem_of_mem_drop hu)
  case eq | neq =>
    obtain ⟨a, b, ⟨-, a₁, -, rfl, -⟩, hm⟩ := hex
    split at hm <;>
    · simp only [Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hm
      obtain ⟨rfl, -⟩ := hm
      exact topNew_mk rfl fun u hu => by
        rcases List.mem_cons.1 hu with rfl | hu
        · exact Or.inr (by rw [tf_fresh_id]; exact Nat.le_refl _)
        · exact Or.inl (List.mem_of_mem_drop hu)
  case existsGeneric =>
    obtain ⟨_, -, _, -, _, -, _, -, a₁, b, ⟨-, a, -, rfl, -⟩, hp, -⟩ := hex
    rw [ShadowPost.running.injEq] at hp
    subst hp
    exact topNew_mk rfl fun u hu => by
      rcases List.mem_cons.1 hu with rfl | hu
      · exact Or.inr (by rw [tf_fresh_id]; exact Nat.le_refl _)
      · exact Or.inl (List.mem_of_mem_tail hu)
  case exists_ =>
    obtain ⟨_, -, a₁, b, ⟨-, a, -, rfl, -⟩, hp, -⟩ := hex
    rw [ShadowPost.running.injEq] at hp
    subst hp
    exact topNew_mk rfl fun u hu => by
      rcases List.mem_cons.1 hu with rfl | hu
      · exact Or.inr (by rw [tf_fresh_id]; exact Nat.le_refl _)
      · exact Or.inl (List.mem_of_mem_tail hu)
  case vecLen =>
    obtain ⟨a, b, -, a₁, b₁, ⟨-, a', -, rfl, -⟩, hp, -⟩ := hex
    rw [ShadowPost.running.injEq] at hp
    subst hp
    exact topNew_mk rfl fun u hu => by
      rcases List.mem_cons.1 hu with rfl | hu
      · exact Or.inr (by rw [tf_fresh_id]; exact Nat.le_refl _)
      · exact Or.inl (List.mem_of_mem_tail hu)
  case testVariant | testVariantGeneric =>
    obtain ⟨a, b, -, hm⟩ := hex
    split at hm
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨x, ⟨-, -, v, -, rfl⟩, hp, -⟩ := hm
      rw [ShadowPost.running.injEq] at hp
      subst hp
      exact topNew_mk rfl fun u hu => by
        rcases List.mem_cons.1 hu with rfl | hu
        · exact Or.inr (by rw [tf_fresh_id]; exact Nat.le_refl _)
        · exact Or.inl (List.mem_of_mem_tail hu)
    · cases hm
  -- The borrows push the new reference's tree.
  case freezeRef =>
    obtain ⟨a, b, -, -, hm⟩ := hex
    split at hm
    · exact topNew_borrow hm rfl fun u hu => List.mem_of_mem_tail hu
    · cases hm
  case mutBorrowLoc | immBorrowLoc =>
    split at hex
    · exact topNew_borrow hex rfl fun u hu => hu
    · cases hex
  case mutBorrowField | immBorrowField | mutBorrowVariantField | immBorrowVariantField
    | mutBorrowFieldGeneric | immBorrowFieldGeneric | mutBorrowVariantFieldGeneric
    | immBorrowVariantFieldGeneric =>
    obtain ⟨a, b, -, hm⟩ := hex
    split at hm
    · obtain ⟨_, -, hm⟩ := Option.bind_eq_some_iff.1 hm
      split at hm
      · exact topNew_borrow hm rfl fun u hu => List.mem_of_mem_tail hu
      · cases hm
    · cases hm
  case mutBorrowGlobal | immBorrowGlobal | mutBorrowGlobalGeneric | immBorrowGlobalGeneric =>
    split at hex
    · split at hex
      · obtain ⟨_, -, hm⟩ := Option.bind_eq_some_iff.1 hex
        exact topNew_borrow hm rfl fun u hu => List.mem_of_mem_tail hu
      · cases hex
    · cases hex
  case vecImmBorrow | vecMutBorrow =>
    obtain ⟨a, b, -, -, hm⟩ := hex
    split at hm
    · exact topNew_borrow hm rfl fun u hu => List.mem_of_mem_drop hu
    · cases hm


/-! ## Steps of the current frame, in the semantics -/

/-- The step replaced only the current frame, by one of its function at its
next instruction. -/
private def NextPc (s' : State) (fr : Frame) (rest : List Frame) : Prop :=
  ∃ fr', s'.frames = fr' :: rest ∧ fr'.function = fr.function ∧ fr'.pc = fr.pc + 1

section Sem

open MoveBytecode.Step

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

private theorem nextPc_next {fr' : Frame} (h : stepOf (next s rest fr') = .done (.running s'))
    (hfun : fr'.function = fr.function) (hpc : fr'.pc = fr.pc + 1) : NextPc s' fr rest := by
  obtain rfl := Outcome.running.inj (done_next h)
  exact ⟨fr', rfl, hfun, hpc⟩

private theorem nextPc_finish {st : List Value} {vals : List Value}
    (h : stepOf (finish { s with frames := { fr with stack := st } :: rest } vals) =
      .done (.running s')) : NextPc s' fr rest := by
  obtain rfl := Outcome.running.inj (done_finish rfl h)
  exact ⟨_, rfl, rfl, rfl⟩

private theorem nextPc_pop (h : stepOf (Step.exec P s fr rest info code .pop) = .done (.running s')) :
    NextPc s' fr rest := by
  obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st, -, rfl⟩ := pop_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  · exact nextPc_next h rfl rfl

private theorem nextPc_copyLoc {i : LocalIndex}
    (h : stepOf (Step.exec P s fr rest info code (.copyLoc i)) = .done (.running s')) :
    NextPc s' fr rest := by
  obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
  cases slot with
  | none => exact (done_wrong h rfl).elim
  | some v =>
    dsimp only at h
    split at h
    · exact nextPc_next h rfl rfl
    · exact (done_wrong h rfl).elim

private theorem nextPc_equality {positive : Bool}
    (h : stepOf (equality P s fr positive >>= fun fr' => next s rest fr') = .done (.running s')) :
    NextPc s' fr rest := by
  obtain ⟨fr', hx, h'⟩ := done_bind h rfl equality_ends
  obtain ⟨-, -, -, -, -, -, c, hR⟩ := equality_ok hx
  rw [hR.2] at h'
  exact nextPc_next h' rfl rfl

private theorem nextPc_testVariant {hh : StructVariantHandleIndex} {tys : List RuntimeType}
    (h : stepOf (Step.testEnum P s fr rest info hh tys) = .done (.running s')) :
    NextPc s' fr rest := by
  obtain ⟨vh, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
    obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨v, -, h⟩ := done_bind h rfl deref_ends
    split at h
    · exact nextPc_finish h
    · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

private theorem nextPc_readRef (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code .readRef) = .done (.running s')) :
    NextPc s' fr rest ∧ ∃ r st v, fr.stack = .ref r :: st ∧ s.read? r.location = some v := by
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRef_ends
  obtain ⟨st, hst, rfl⟩ := popRef_ok hpr
  dsimp only at h
  obtain ⟨v, hv, h⟩ := done_bind h rfl deref_ends
  have hd := deref_ok hv
  split at h
  · exact (done_wrong h rfl).elim
  · refine ⟨nextPc_finish h, r, st, v, hst, ?_⟩
    rw [deref_eq (s' := { s with frames := { fr with stack := st } :: rest }) (s := s)
      (by simp [hsf]) rfl rfl rfl] at hd
    exact deref?_read hd

private theorem nextPc_freezeRef
    (h : stepOf (Step.exec P s fr rest info code .freezeRef) = .done (.running s')) :
    NextPc s' fr rest := by
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRef_ends
  obtain ⟨st, -, rfl⟩ := popRef_ok hpr
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  · exact nextPc_next h rfl rfl

private theorem nextPc_borrowLoc {mutable : Bool} {i : LocalIndex}
    (h : stepOf (Step.borrowLoc P s fr rest mutable i) = .done (.running s')) :
    NextPc s' fr rest := by
  unfold Step.borrowLoc at h
  obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
  cases slot with
  | none => exact (done_wrong h rfl).elim
  | some v =>
    dsimp only at h
    obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
    split at h
    · exact (done_wrong h rfl).elim
    · exact nextPc_next h rfl rfl

private theorem nextPc_borrowField {mutable : Bool} {hh : FieldHandleIndex} {tys : List RuntimeType}
    (h : stepOf (Step.borrowField P s fr rest info mutable hh tys) = .done (.running s')) :
    NextPc s' fr rest := by
  unfold Step.borrowField at h
  obtain ⟨fh, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨t, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
    obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨_, -, h⟩ := done_bind h rfl deref_ends
    exact nextPc_finish h
  · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

private theorem nextPc_borrowVariantField {mutable : Bool} {hh : VariantFieldHandleIndex}
    {tys : List RuntimeType}
    (h : stepOf (Step.borrowVariantField P s fr rest info mutable hh tys) = .done (.running s')) :
    NextPc s' fr rest := by
  unfold Step.borrowVariantField at h
  obtain ⟨vfh, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, -, h⟩ := done_bind h rfl (mapM_ends (fun _ =>
      ends_bind (ends_need _ _) fun _ _ => ends_bind (ends_need _ _) fun _ _ => ends_need _ _) _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
    obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨v, -, h⟩ := done_bind h rfl deref_ends
    split at h
    · split at h
      · exact nextPc_finish h
      · exact (done_fail h rfl).elim
    · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

private theorem nextPc_borrowGlobal {mutable : Bool} {d : StructDefinitionIndex}
    {tys : List RuntimeType}
    (h : stepOf (Step.borrowGlobal P s fr rest mutable d tys) = .done (.running s')) :
    NextPc s' fr rest := by
  unfold Step.borrowGlobal at h
  obtain ⟨sid, -, h⟩ := done_bind h rfl globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h rfl popAddress_ends
  obtain ⟨st, -, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h rfl unlocked_ends
  split at h
  · exact nextPc_next h rfl rfl
  · exact (done_fail h rfl).elim

private theorem nextPc_vecLen {sig : SignatureIndex}
    (h : stepOf (Step.exec P s fr rest info code (.vecLen sig)) = .done (.running s')) :
    NextPc s' fr rest := by
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
  split at h
  · exact nextPc_finish h
  · exact (done_fail h rfl).elim

private theorem nextPc_borrowElement {mutable : Bool} {sig : SignatureIndex}
    (h : stepOf (Step.borrowElement s fr rest info mutable sig) = .done (.running s')) :
    NextPc s' fr rest := by
  unfold Step.borrowElement at h
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨i, fr₁⟩, hpi, h⟩ := done_bind h rfl popIndex_ends
  obtain ⟨x, st₁, -, rfl, rfl⟩ := popIndex_ok hpi
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
  split at h
  · exact nextPc_finish h
  · exact (done_fail h rfl).elim

private theorem nextPc_moveLoc {i : LocalIndex}
    (h : stepOf (Step.exec P s fr rest info code (.moveLoc i)) = .done (.running s')) :
    ∃ fr', s'.frames = fr' :: rest ∧ fr'.function = fr.function ∧ fr'.pc = fr.pc + 1 ∧
      fr'.locals = fr.locals.setIfInBounds i none := by
  obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
  cases slot with
  | none => exact (done_wrong h rfl).elim
  | some v =>
    obtain rfl := Outcome.running.inj (done_next h)
    exact ⟨_, rfl, rfl, rfl, rfl⟩

private theorem nextPc_stLoc {i : LocalIndex}
    (h : stepOf (Step.exec P s fr rest info code (.stLoc i)) = .done (.running s')) :
    NextPc s' fr rest := by
  obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st, -, rfl⟩ := pop_ok hpop
  dsimp only at h
  obtain ⟨tys, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨t, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  · cases slot with
    | none => exact nextPc_next h rfl rfl
    | some old =>
      dsimp only at h
      split at h
      · exact (done_wrong h rfl).elim
      · exact nextPc_next h rfl rfl

private theorem nextPc_moveFrom {d : StructDefinitionIndex} {tys : List RuntimeType}
    (h : stepOf (Step.moveResourceFrom P s fr rest d tys) = .done (.running s')) :
    NextPc s' fr rest := by
  obtain ⟨sid, -, h⟩ := done_bind h rfl globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h rfl popAddress_ends
  obtain ⟨st, -, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h rfl unlocked_ends
  split at h
  · obtain rfl := Outcome.running.inj (done_finish rfl h)
    exact ⟨_, rfl, rfl, rfl⟩
  · exact (done_fail h rfl).elim

private theorem nextPc_moveTo {d : StructDefinitionIndex} {tys : List RuntimeType}
    (h : stepOf (Step.moveResourceTo P s fr rest d tys) = .done (.running s')) :
    NextPc s' fr rest := by
  obtain ⟨sid, -, h⟩ := done_bind h rfl globalType_ends
  obtain ⟨⟨v, fr₁⟩, hpv, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st₁, -, rfl⟩ := pop_ok hpv
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨sv, -, h⟩ := done_bind h rfl deref_ends
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h rfl unlocked_ends
    split at h
    · exact (done_fail h rfl).elim
    · obtain rfl := Outcome.running.inj (done_finish (s₁ := { ({ s with frames :=
        { fr with stack := st } :: rest } : State) with storage := s.storage.insert _ sid v })
        rfl h)
      exact ⟨_, rfl, rfl, rfl⟩
  · exact (done_wrong h rfl).elim

end Sem


/-! ## Closure calls -/

/-- The references among values, with the identities of their trees. -/
def slotRefs (ts : List IdTree) (vs : List Value) : List (Nat × Reference) :=
  (ts.zip vs).filterMap fun (t, v) =>
    match v with
    | .ref r => some (t.id, r)
    | _ => none

theorem mem_slotRefs {ts : List IdTree} {vs : List Value} {d : Nat} {r : Reference} :
    (d, r) ∈ slotRefs ts vs ↔ ∃ t, (t, Value.ref r) ∈ ts.zip vs ∧ d = t.id := by
  simp only [slotRefs, List.mem_filterMap]
  constructor
  · rintro ⟨⟨t, v⟩, hz, hv⟩
    cases v <;> simp at hv
    obtain ⟨rfl, rfl⟩ := hv
    exact ⟨t, hz, rfl⟩
  · rintro ⟨t, hz, rfl⟩
    exact ⟨(t, .ref r), hz, rfl⟩

section ClosureCalls

variable {P : Program} {m : ModuleId} {s s' : State} {fr : Frame} {rest : List Frame}
  {info : FunctionInfo} {g : ShadowFrame} {hs : List ShadowFrame} {hst hhe : List IdTree}
  {f : Nat} {post : ShadowPost} {evs : List Event}

open MoveBytecode.Step in
/-- **What a `CallClosure` that continues does** to the frames: the closure on
top and the provided arguments below it are popped, and the closure's
target's first frame holds the captured and the provided values. -/
theorem callClosure_concrete {sig : SignatureIndex}
    (h : stepOf (Step.callClosure P s fr rest info sig) = .done (.running s')) :
    ∃ tf mask cs cps crs ca st callee m paramTys ps0 rs0 a,
      info.module.signatures[sig]? = some [.function ps0 rs0 a] ∧
      fr.stack = .closure tf mask cs cps crs ca :: st ∧ cps.length ≤ st.length ∧
      RuntimeType.ofSlotTokensWith info.module fr.function.typeArgs ps0 = some cps ∧
      RuntimeType.ofSlotTokensWith info.module fr.function.typeArgs rs0 = some crs ∧
      conformsAll P (st.take cps.length).reverse cps = true ∧
      callee.function = tf ∧ callee.countedActive = true ∧ P.paramTypes tf = some paramTys ∧
      P.returnTypes tf = some crs ∧
      conformsAll P (maskCompose mask cs (st.take cps.length).reverse) paramTys = true ∧
      callee.locals = ((maskCompose mask cs (st.take cps.length).reverse).map some ++
        List.replicate m none).toArray ∧
      callee.stack = [] ∧ callee.id = s.nextFrame ∧
      s'.frames = callee :: { fr with stack := st.drop cps.length } :: rest := by
  unfold Step.callClosure at h
  obtain ⟨sigTys, hsigTys, h⟩ := done_bind h rfl (ends_need _ _)
  dsimp only at h
  split at h
  · obtain ⟨ps, hps, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨rs, hrs, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨_, hx, h⟩ := done_bind h rfl (ends_pure _)
    simp only [pure, Except.pure, Except.ok.injEq] at hx
    subst hx
    dsimp only at h
    obtain ⟨⟨c, fr₁⟩, hpop, h⟩ := done_bind h rfl pop_ends
    obtain ⟨st, hst, rfl⟩ := pop_ok hpop
    dsimp only at h
    split at h
    · next target mask captured cps crs ca =>
      split at h
      · exact (done_wrong h rfl).elim
      next hchk =>
      have hcps : cps = ps := by
        by_cases e : cps = ps
        · exact e
        · exfalso; apply hchk; simp [e]
      have hcrs : crs = rs := by
        by_cases e : crs = rs
        · exact e
        · exfalso; apply hchk; simp [e]
      subst hcps hcrs
      obtain ⟨⟨provided, fr₂⟩, hpop2, h⟩ := done_bind h rfl popN_ends
      obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop2
      dsimp only at h
      split at h
      · exact (done_wrong h rfl).elim
      next hprov =>
      obtain ⟨targetInfo, -, h⟩ := done_bind h rfl (ends_need _ _)
      split at h
      · exact (done_wrong h rfl).elim
      obtain ⟨paramTys, hpt, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨retTys, hrt, h⟩ := done_bind h rfl (ends_need _ _)
      split at h
      · exact (done_wrong h rfl).elim
      next hconf =>
      simp only [Bool.or_eq_true, Bool.not_eq_true', not_or, Bool.not_eq_false] at hconf
      obtain ⟨localTys, -, h⟩ := done_bind h rfl (ends_need _ _)
      have hrr : retTys = crs := by
        by_cases e : retTys = crs
        · exact e
        · exact absurd (by simp [e]) hconf.2
      have hrt' : P.returnTypes target = some crs := by rw [← hrr]; exact need_ok' hrt
      split at h <;> split at h
      all_goals first
        | exact (done_fail h rfl).elim
        | skip
      all_goals
        simp only [stepOf, pure, Except.pure, StepResult.done.injEq, Outcome.running.injEq] at h
        subst h
        exact ⟨target, mask, captured, cps, crs, ca, st, _, _, paramTys, _, _, _,
          need_ok' hsigTys, hst, hn, need_ok' hps, need_ok' hrs, by simpa using hprov, rfl, rfl,
          need_ok' hpt, hrt', hconf.1, rfl, rfl, by first | rfl | (split <;> rfl), rfl⟩
    · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

/-- **The annotation of `CallClosure`**: the callee's locals take the
captured values' and the provided arguments' identities, the step enters the
callee with the identities of the reference arguments, and consumes the
closure. -/
theorem callClosure_shadow {sig : SignatureIndex} {callee : Frame} {st : List Value}
    {tf : FunctionRef} {mask : BitVec 64} {cs : List Value} {cps crs : List RuntimeType}
    {ca : AbilitySet} {i : Nat} {cts gst : List IdTree} {n : Nat}
    (hstk : fr.stack = .closure tf mask cs cps crs ca :: st) (hgst : g.stack = .node i cts :: gst)
    (hn : n ≤ gst.length) (hn' : n ≤ st.length)
    (h2f : s'.frames = callee :: { fr with stack := st.drop n } :: rest)
    (hex : Shadow.exec P m (.callClosure sig) s fr (.running s') ⟨g :: hs, hst, hhe, f⟩ =
      some (post, evs)) :
    post = .running ⟨{ locals := ((maskCompose mask cts (gst.take n).reverse).map some ++
          List.replicate (callee.locals.size - (maskCompose mask cts (gst.take n).reverse).length)
            none).toArray, stack := [] } ::
        { g with stack := gst.drop n } :: hs, hst, hhe, f⟩ ∧
    evs = (slotRefs (gst.take n).reverse (st.take n).reverse).map
        (fun (i, r) => Event.used i r) ++
      [.entered callee.id ((slotRefs (gst.take n).reverse (st.take n).reverse).map (·.1)),
        .consumed i (.closure tf mask cs cps crs ca) (.callClosure m)] := by
  have hk : fr.stack.length - 1 - (st.drop n).length = n := by rw [hstk]; simp; omega
  have e : Shadow.exec P m (.callClosure sig) s fr (.running s') ⟨g :: hs, hst, hhe, f⟩ =
      some (.running ⟨{ locals := ((maskCompose mask cts (gst.take n).reverse).map some ++
          List.replicate (callee.locals.size - (maskCompose mask cts (gst.take n).reverse).length)
            none).toArray, stack := [] } ::
        { g with stack := gst.drop n } :: hs, hst, hhe, f⟩,
      (slotRefs (gst.take n).reverse (st.take n).reverse).map (fun (i, r) => Event.used i r) ++
        [.entered callee.id ((slotRefs (gst.take n).reverse (st.take n).reverse).map (·.1)),
          .consumed i (.closure tf mask cs cps crs ca) (.callClosure m)]) := by
    simp only [Shadow.exec, h2f, hk, guard, hgst, if_pos hn]
    rw [hstk]
    rfl
  rw [e, Option.some.injEq, Prod.mk.injEq] at hex
  exact ⟨hex.1.symm, hex.2.symm⟩

end ClosureCalls

/-! ## Where the trees of a core step come from -/

private theorem availabilityAfter_pushesNew {instr : Bytecode} (hk : instr.PushesNew = true)
    (a : LocalIndex → Availability) : availabilityAfter instr a = a := by
  cases instr <;> first | rfl | (simp [Bytecode.PushesNew] at hk)

/-- The step replaced only the current frame, by one of its function at a
successor of its instruction. -/
private def NextSucc (s' : State) (fr : Frame) (rest : List Frame) (code : CodeUnit) : Prop :=
  ∃ fr', s'.frames = fr' :: rest ∧ fr'.function = fr.function ∧
    fr'.pc ∈ successors code.code fr.pc

namespace StepCtx

variable {P : Program} {s s' : State} {h h' : Shadow} {E evs : List Event} {fr : Frame}
  {rest : List Frame} {g : ShadowFrame} {hs : List ShadowFrame} {info : FunctionInfo}
  {code : CodeUnit} {instr : Bytecode}

private theorem nextSucc (C : StepCtx P s h E fr rest g hs info code instr) (hn : NextPc s' fr rest)
    (hc : instr.control = .fallThrough) : NextSucc s' fr rest code := by
  obtain ⟨fr', h1, h2, h3⟩ := hn
  refine ⟨fr', h1, h2, ?_⟩
  simp [successors, C.hi, hc, h3]

/-- **An instruction that pushes new trees.** -/
private theorem pushNew (C : StepCtx P s h E fr rest g hs info code instr)
    (hk : instr.PushesNew = true) (hns : NextSucc s' fr rest code)
    (hex : Shadow.exec P info.moduleId instr s fr (.running s') h = some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨g', hh', hloc, hstk⟩ := exec_topNew C.hh hex hk
  obtain ⟨fr', hs', hfun, hpc⟩ := hns
  exact C.topOnly hs' hh' hfun
    (fun t ht => (hstk t ht).elim C.stack_origin fun hf => Or.inr (Or.inr (Or.inl hf)))
    (C.keptLocals hloc hpc (availabilityAfter_pushesNew hk _))

end StepCtx

namespace StepCtx

variable {P : Program} {s s' : State} {h h' : Shadow} {E evs : List Event} {fr : Frame}
  {rest : List Frame} {g : ShadowFrame} {hs : List ShadowFrame} {info : FunctionInfo}
  {code : CodeUnit}

/-- **`CopyLoc`**: the copy's tree is the local's, which is available. -/
private theorem copyLoc {j : LocalIndex} (C : StepCtx P s h E fr rest g hs info code (.copyLoc j))
    (hst : stepOf (Step.exec P s fr rest info code (.copyLoc j)) = .done (.running s'))
    (hex : Shadow.exec P info.moduleId (.copyLoc j) s fr (.running s') h =
      some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  simp only [Shadow.exec, C.hh, Option.pure_def] at hex
  split at hex
  · next t ht =>
    simp only [Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hex
    obtain ⟨rfl, -⟩ := hex
    obtain ⟨fr', hs', hfun, hpc⟩ := C.nextSucc (nextPc_copyLoc hst) rfl
    refine C.topOnly hs' rfl hfun (fun u hu => ?_) (C.keptLocals rfl hpc rfl)
    rcases List.mem_cons.1 hu with rfl | hu
    · exact C.local_origin ht (C.avail.1 j rfl)
    · exact C.stack_origin hu
  · cases hex

end StepCtx

/-- A child of a tree on a fitting stack is an occurrence of a subvalue. -/
theorem tf_child_occ {t u : IdTree} {v : Value} (hf : t.fits v = true) (hu : u ∈ t.children) :
    ∃ w, (u.id, w) ∈ t.occurrences v ∧ w ∈ subvalues v := by
  obtain ⟨i, cs⟩ := t
  simp only [IdTree.children] at hu
  simp only [IdTree.fits] at hf
  obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hu
  have hlen := fitsAll_length hf
  have hkl : k < v.children.length := by
    have := (List.getElem?_eq_some_iff.1 hk).1
    omega
  obtain ⟨w, hw⟩ : ∃ w, v.children[k]? = some w := ⟨_, List.getElem?_eq_getElem hkl⟩
  have hocc : (u.id, w) ∈ IdTree.occurrences (.node i cs) v :=
    List.mem_cons_of_mem _ (occ_of_zip (zip_of_getElem? hk hw) _ (tf_root_occ u w))
  exact ⟨w, hocc, occurrences_mem (t := .node i cs) hf hocc⟩

/-- A resource's tree is the tree of a value that is no reference. -/
theorem tf_storage_origin {P : Program} {s : State} {h : Shadow} {E : List Event}
    (hI : RefTraceInv P s h E) {k : Nat} {t : IdTree} (ht : h.storage[k]? = some t) :
    ∃ w, (t.id, w) ∈ h.occurrences s ∧ w.isRef = false := by
  have hfit := hI.pre.fits
  rw [shadow_fits] at hfit
  simp only [extFit, Bool.and_eq_true] at hfit
  have hlen := fitsAll_length hfit.2.1
  have hkl : k < s.storage.length := by
    have := (List.getElem?_eq_some_iff.1 ht).1
    simp only [List.length_map] at hlen
    omega
  have hv : (s.storage.map (·.value))[k]? = some s.storage[k].value := by simp [hkl]
  have hz := zip_of_getElem? ht hv
  refine ⟨s.storage[k].value, ?_, hI.pre.refsTop.storage _ (List.getElem_mem hkl) _
    (self_mem_subvalues _)⟩
  rw [shadow_occurrences, extOcc]
  exact List.mem_append_right _ (List.mem_append_left _ (occ_of_zip hz _ (tf_root_occ t _)))

section Sem

open MoveBytecode.Step

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

private theorem nextPc_unpack {d : StructDefinitionIndex} {tys : List RuntimeType}
    (h : stepOf (Step.unpackStruct P s fr rest d tys) = .done (.running s')) :
    NextPc s' fr rest ∧ ∃ v st, fr.stack = v :: st ∧ v.type.isValue = true := by
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h rfl pop_ends
    obtain ⟨st, hst, rfl⟩ := pop_ok hpop
    dsimp only at h
    split at h
    · split at h
      · exact (done_wrong h rfl).elim
      · exact ⟨nextPc_next h rfl rfl, _, st, hst, rfl⟩
    · split at h
      · exact (done_wrong h rfl).elim
      · exact ⟨nextPc_next h rfl rfl, _, st, hst, rfl⟩
    · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

private theorem nextPc_unpackVariant {hh : StructVariantHandleIndex} {tys : List RuntimeType}
    (h : stepOf (Step.unpackEnum P s fr rest info hh tys) = .done (.running s')) :
    NextPc s' fr rest ∧ ∃ v st, fr.stack = v :: st ∧ v.type.isValue = true := by
  obtain ⟨vh, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h rfl pop_ends
    obtain ⟨st, hst, rfl⟩ := pop_ok hpop
    dsimp only at h
    split at h
    · split at h
      · exact (done_wrong h rfl).elim
      · split at h
        · exact (done_fail h rfl).elim
        · exact ⟨nextPc_next h rfl rfl, _, st, hst, rfl⟩
    · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

private theorem nextPc_vecUnpack {sig : SignatureIndex} {n : Nat}
    (hargs : ∀ x ∈ fr.function.typeArgs, x.isValue = true)
    (h : stepOf (Step.exec P s fr rest info code (.vecUnpack sig n)) = .done (.running s')) :
    NextPc s' fr rest ∧ ∃ v st, fr.stack = v :: st ∧ v.type.isValue = true := by
  obtain ⟨t, ht, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st, hst, rfl⟩ := pop_ok hpop
  dsimp only at h
  split at h
  · next t' elems =>
    split at h
    · exact (done_wrong h rfl).elim
    · next hne =>
      split at h
      · exact (done_fail h rfl).elim
      · refine ⟨nextPc_next h rfl rfl, _, st, hst, ?_⟩
        simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hne
        simp only [Value.type, RuntimeType.isValue, hne]
        exact elemTypeWith_value hargs ht
  · exact (done_wrong h rfl).elim

end Sem

namespace StepCtx

variable {P : Program} {s s' : State} {h h' : Shadow} {E evs : List Event} {fr : Frame}
  {rest : List Frame} {g : ShadowFrame} {hs : List ShadowFrame} {info : FunctionInfo}
  {code : CodeUnit}

/-- **`MoveLoc`**: the moved tree is the local's, which is available; the
local becomes unavailable. -/
private theorem moveLoc {j : LocalIndex} (C : StepCtx P s h E fr rest g hs info code (.moveLoc j))
    (hst : stepOf (Step.exec P s fr rest info code (.moveLoc j)) = .done (.running s'))
    (hex : Shadow.exec P info.moduleId (.moveLoc j) s fr (.running s') h =
      some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  simp only [Shadow.exec, C.hh, Option.pure_def] at hex
  split at hex
  · next t ht =>
    simp only [Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hex
    obtain ⟨rfl, -⟩ := hex
    obtain ⟨fr', hs', hfun, hpc0, -⟩ := nextPc_moveLoc hst
    have hpc : fr'.pc ∈ successors code.code fr.pc := by
      simp [successors, C.hi, Bytecode.control, hpc0]
    refine C.topOnly hs' rfl hfun (fun u hu => ?_) (fun k u hu hav => ?_)
    · rcases List.mem_cons.1 hu with rfl | hu
      · exact C.local_origin ht (C.avail.1 j rfl)
      · exact C.stack_origin hu
    · have hkj : k ≠ j := by
        rintro rfl
        simp [Array.getElem?_setIfInBounds] at hu
      simp only at hu
      rw [Array.getElem?_setIfInBounds_ne hkj] at hu
      have := C.succ_avail hpc hav
      simp only [availabilityAfter, if_neg hkj] at this
      exact C.local_origin hu this
  · cases hex

/-- **`StLoc`**: the stored tree comes off the operand stack. -/
private theorem stLoc {j : LocalIndex} (C : StepCtx P s h E fr rest g hs info code (.stLoc j))
    (hst : stepOf (Step.exec P s fr rest info code (.stLoc j)) = .done (.running s'))
    (hex : Shadow.exec P info.moduleId (.stLoc j) s fr (.running s') h =
      some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  simp only [Shadow.exec, C.hh, Option.pure_def] at hex
  split at hex
  · next t st hgs =>
    simp only [Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hex
    obtain ⟨rfl, -⟩ := hex
    obtain ⟨fr', hs', hfun, hpc⟩ := C.nextSucc (nextPc_stLoc hst) rfl
    refine C.topOnly hs' rfl hfun
      (fun u hu => C.stack_origin (by rw [hgs]; exact List.mem_cons_of_mem _ hu))
      (fun k u hu hav => ?_)
    simp only at hu
    by_cases hkj : k = j
    · subst hkj
      have hut : u = t := by
        simp only [Array.getElem?_setIfInBounds, if_true] at hu
        split at hu
        · simpa using hu.symm
        · cases hu
      subst hut
      exact C.stack_origin (by rw [hgs]; exact List.mem_cons_self ..)
    · rw [Array.getElem?_setIfInBounds_ne hkj] at hu
      have := C.succ_avail hpc hav
      simp only [availabilityAfter, if_neg hkj] at this
      exact C.local_origin hu this
  · cases hex

/-- **`ReadRef`**: the pushed tree is the tree of the value at the location,
which is no reference. -/
private theorem readRef (C : StepCtx P s h E fr rest g hs info code .readRef)
    (hst : stepOf (Step.exec P s fr rest info code .readRef) = .done (.running s'))
    (hex : Shadow.exec P info.moduleId .readRef s fr (.running s') h = some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨hnp, r, st, v, hfs, hread⟩ := nextPc_readRef C.sf hst
  simp only [Shadow.exec, C.hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
    Option.some.injEq, Prod.mk.injEq] at hex
  obtain ⟨⟨a, b⟩, hra, tv, htv, hpost, -⟩ := hex
  obtain ⟨h1, -, -⟩ := refAt_spec hra
  rw [hfs] at h1
  simp only [List.getElem?_cons_zero, Option.some.injEq, Value.ref.injEq] at h1
  subst h1
  rw [ShadowPost.running.injEq] at hpost
  subst hpost
  obtain ⟨fr', hs', hfun, hpc⟩ := C.nextSucc hnp rfl
  refine C.topOnly hs' rfl hfun (fun u hu => ?_) (C.keptLocals rfl hpc rfl)
  rcases List.mem_cons.1 hu with rfl | hu
  · obtain ⟨tw, htw, -, R, hR⟩ := tree_read C.trace.pre.fits hread
    rw [htw, Option.some.injEq] at htv
    subst htv
    refine Or.inr (Or.inr (Or.inr ⟨v, hR.symm.subset (List.mem_append_left _ (tf_root_occ _ v)),
      ?_⟩))
    exact C.trace.pre.refsTop.deref fr (by rw [C.sf]; exact List.mem_cons_self ..) r
      (Or.inl (by rw [hfs]; exact List.mem_cons_self ..)) v hread v (self_mem_subvalues v)
  · exact C.stack_origin (List.mem_of_mem_drop hu)

/-- **`MoveFrom`**: the pushed tree is the resource's. -/
private theorem moveFrom {instr : Bytecode}
    (hi : (∃ d, instr = .moveFrom d) ∨ ∃ i, instr = .moveFromGeneric i)
    (C : StepCtx P s h E fr rest g hs info code instr) (hnp : NextPc s' fr rest)
    (hex : Shadow.exec P info.moduleId instr s fr (.running s') h =
      some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  rcases hi with ⟨d, rfl⟩ | ⟨i, rfl⟩
  all_goals
    simp only [Shadow.exec, C.hh, Option.bind_eq_bind, Option.pure_def] at hex
    split at hex
    · obtain ⟨v, -, hex⟩ := Option.bind_eq_some_iff.1 hex
      split at hex
      · obtain ⟨k, -, hex⟩ := Option.bind_eq_some_iff.1 hex
        obtain ⟨t, ht, hex⟩ := Option.bind_eq_some_iff.1 hex
        simp only [Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hex
        obtain ⟨rfl, -⟩ := hex
        obtain ⟨fr', hs', hfun, hpc⟩ := C.nextSucc hnp rfl
        refine C.topOnly hs' rfl hfun (fun u hu => ?_) (C.keptLocals rfl hpc rfl)
        rcases List.mem_cons.1 hu with rfl | hu
        · exact Or.inr (Or.inr (Or.inr (tf_storage_origin C.trace ht)))
        · exact C.stack_origin (List.mem_of_mem_drop hu)
      · cases hex
    · cases hex

/-- **`MoveTo`**: the operand stack only shrinks. -/
private theorem moveTo {instr : Bytecode}
    (hi : (∃ d, instr = .moveTo d) ∨ ∃ i, instr = .moveToGeneric i)
    (C : StepCtx P s h E fr rest g hs info code instr) (hnp : NextPc s' fr rest)
    (hex : Shadow.exec P info.moduleId instr s fr (.running s') h =
      some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  rcases hi with ⟨d, rfl⟩ | ⟨i, rfl⟩
  all_goals
    simp only [Shadow.exec, C.hh, Option.bind_eq_bind, Option.pure_def] at hex
    obtain ⟨tv, -, hex⟩ := Option.bind_eq_some_iff.1 hex
    obtain ⟨v, -, hex⟩ := Option.bind_eq_some_iff.1 hex
    split at hex
    · obtain ⟨x, -, hex⟩ := Option.bind_eq_some_iff.1 hex
      split at hex
      · simp only [Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hex
        obtain ⟨rfl, -⟩ := hex
        obtain ⟨fr', hs', hfun, hpc⟩ := C.nextSucc hnp rfl
        exact C.topOnly hs' rfl hfun (fun u hu => C.stack_origin (List.mem_of_mem_drop hu))
          (C.keptLocals rfl hpc rfl)
      · cases hex
    · cases hex

/-- **The unpacking instructions**: the pushed trees are the children of the
container's, whose value has a value type. -/
private theorem unpackLike {instr : Bytecode} (C : StepCtx P s h E fr rest g hs info code instr)
    (hnp : NextPc s' fr rest) (hctl : instr.control = .fallThrough)
    (hid : availabilityAfter instr (modelAvail P fr.function fr.pc) =
      modelAvail P fr.function fr.pc)
    (hv : ∃ v st, fr.stack = v :: st ∧ v.type.isValue = true)
    (hsh : ∃ t st, g.stack = t :: st ∧ h'.frames = { g with stack := t.children.reverse ++ st } :: hs) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨v, vst, hfs, hval⟩ := hv
  obtain ⟨t, st, hgs, hh'⟩ := hsh
  obtain ⟨fr', hs', hfun, hpc⟩ := C.nextSucc hnp hctl
  refine C.topOnly hs' hh' hfun (fun u hu => ?_) (C.keptLocals rfl hpc hid)
  rcases List.mem_append.1 hu with hu | hu
  · have hz : (t, v) ∈ g.stack.zip fr.stack := by
      rw [hgs, hfs]; exact List.mem_cons_self ..
    have hft := fitsAll_of_zip (frame_stack_fits C.fits) hz
    obtain ⟨w, hw, hws⟩ := tf_child_occ hft (List.mem_reverse.1 hu)
    have hshaped := C.trace.typed.valsShaped fr (by rw [C.sf]; exact List.mem_cons_self ..) v
      (mem_frameVals_stack (by rw [hfs]; exact List.mem_cons_self ..))
    refine Or.inr (Or.inr (Or.inr ⟨w, ?_, shaped_noRef v hshaped hval w hws⟩))
    refine tf_pair_occ C.head ?_
    rw [frame_occurrences]
    exact List.mem_append_right _ (occ_of_zip hz _ hw)
  · exact C.stack_origin (by rw [hgs]; exact List.mem_cons_of_mem _ hu)

end StepCtx

/-! ## Calls and returns -/

section Sem

open MoveBytecode.Step

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

/-- A call that continues pushes the callee's frame over the caller, which
keeps its position. -/
theorem tf_call_frames {hd : FunctionHandleIndex} {tys : List RuntimeType}
    (h : stepOf (Step.call P s fr rest info hd tys) = .done (.running s')) :
    ∃ cf n, s'.frames = cf :: { fr with stack := fr.stack.drop n } :: rest := by
  unfold Step.call at h
  obtain ⟨callee, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨calleeInfo, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨fh, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  dsimp only at h
  obtain ⟨paramTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨retTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨hps, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨hrs, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨⟨args, fr₁⟩, hpop, h⟩ := done_bind h rfl popN_ends
  obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_native h rfl).elim
  obtain ⟨localTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h <;> split at h
  all_goals first
    | exact (done_fail h rfl).elim
    | skip
  all_goals
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq, Outcome.running.injEq] at h
    subst h
    exact ⟨_, _, rfl⟩

/-- A return that continues resumes the caller at its next instruction. -/
theorem tf_ret_frames
    (h : stepOf (Step.exec P s fr rest info code .ret) = .done (.running s')) :
    ∃ caller rest', rest = caller :: rest' ∧ ∃ caller', s'.frames = caller' :: rest' ∧
      caller'.function = caller.function ∧ caller'.pc = caller.pc + 1 ∧
      caller'.locals = caller.locals := by
  unfold Step.exec Step.ret at h
  obtain ⟨retTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  cases rest with
  | nil => simp [stepOf, pure, Except.pure] at h
  | cons caller rest' =>
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq, Outcome.running.injEq] at h
    subst h
    exact ⟨caller, rest', rfl, _, rfl, rfl, rfl, rfl⟩

end Sem

/-- A local available after a call returns was available at the call. -/
theorem tf_avail_call {P : Program} (hc : Certified P (modelTypes P) (modelAvail P))
    {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit} {pc : Nat} {instr : Bytecode}
    (hf : P.function? f = some info) (hcode : info.definition.code = some code)
    (hreach : CodeReachable code.code pc) (hi : code.code[pc]? = some instr)
    (hcall : ((∃ hd, instr = .call hd) ∨ ∃ i, instr = .callGeneric i) ∨
      ∃ sig, instr = .callClosure sig) {j : Nat}
    (hj : modelAvail P f (pc + 1) j = .available) : modelAvail P f pc j = .available := by
  obtain ⟨-, -, -, hcert⟩ := hc.availability f info code hf hcode
  rcases hcall with (⟨hd, rfl⟩ | ⟨i, rfl⟩) | ⟨sig, rfl⟩ <;>
  exact refines_available ((hcert pc _ hreach hi).2 (pc + 1)
    (by simp [successors, hi, Bytecode.control]) j) hj

namespace StepCtx

variable {P : Program} {s s' : State} {h h' : Shadow} {E evs : List Event} {fr : Frame}
  {rest : List Frame} {g : ShadowFrame} {hs : List ShadowFrame} {info : FunctionInfo}
  {code : CodeUnit}

/-- **`Call`**: the callee's locals hold the arguments' trees; the caller
keeps its trees and its position, at a call. -/
private theorem call {instr : Bytecode}
    (hi : (∃ hd, instr = .call hd) ∨ ∃ i, instr = .callGeneric i)
    (C : StepCtx P s h E fr rest g hs info code instr) {hd : FunctionHandleIndex}
    {tys : List RuntimeType}
    (hst : stepOf (Step.call P s fr rest info hd tys) = .done (.running s'))
    (hex : Shadow.exec P info.moduleId instr s fr (.running s') h =
      some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨cf, n, hs'f⟩ := tf_call_frames hst
  have hcall := hi
  rcases hi with ⟨hd', rfl⟩ | ⟨i, rfl⟩
  all_goals
    simp only [Shadow.exec, C.hh, hs'f, Option.pure_def] at hex
    obtain ⟨_, -, hex⟩ := Option.bind_eq_some_iff.1 hex
    simp only [Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hex
    obtain ⟨rfl, -⟩ := hex
    refine ⟨fun g₂ fr₂ hz => ?_, fun f hf => ?_⟩
    · rw [hs'f] at hz
      simp only [List.zip_cons_cons, List.mem_cons, Prod.mk.injEq] at hz
      rcases hz with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩ | hz
      · refine ⟨fun t ht => by simp at ht, fun j t ht _ => ?_⟩
        simp only [List.getElem?_toArray] at ht
        have hm := List.mem_of_getElem? ht
        simp only [List.mem_append, List.mem_map, List.mem_replicate] at hm
        rcases hm with ⟨u, hu, hut⟩ | ⟨-, hne⟩
        · simp only [Option.some.injEq] at hut
          subst hut
          exact C.stack_origin (List.mem_of_mem_take (List.mem_reverse.1 hu))
        · cases hne
      · exact ⟨fun t ht => C.stack_origin (List.mem_of_mem_drop ht),
          fun j t ht hav => C.local_origin ht hav⟩
      · exact ⟨fun t ht => Or.inl ⟨_, _, C.below hz, ht⟩,
          fun j t ht hav => Or.inr (Or.inl ⟨_, _, j, C.below hz, ht, hav⟩)⟩
    · rw [hs'f] at hf
      simp only [List.tail_cons, List.mem_cons] at hf
      rcases hf with rfl | hf
      · exact ⟨info, code, _, C.act.fn, C.act.body, C.act.reachable, C.hi, Or.inl hcall⟩
      · exact C.calls f (by rw [C.sf]; exact hf)

/-- **`CallClosure`**: the callee's locals hold the closure's captured trees,
of values that are no references, and the provided arguments' trees; the
caller keeps its other trees and its position, at a closure call. -/
private theorem callClosure {sig : SignatureIndex}
    (C : StepCtx P s h E fr rest g hs info code (.callClosure sig))
    (hst : stepOf (Step.callClosure P s fr rest info sig) = .done (.running s'))
    (hex : Shadow.exec P info.moduleId (.callClosure sig) s fr (.running s') h =
      some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨tf, mask, cs, cps, crs, ca, vst, callee, m, paramTys, ps0, rs0, a, -, hstk, hn, -, -, -,
    -, -, -, -, -, hloc, hcst, hid, h2f⟩ := callClosure_concrete hst
  -- The captured values are no references.
  have hcNR : ∀ w ∈ cs, w.isRef = false := by
    obtain ⟨tys, -, hconf⟩ := C.act.types
    obtain ⟨t, -, hct⟩ := conformsAll_mem hconf _ (by rw [hstk]; exact List.mem_cons_self ..)
    have hw := conforms_wellFormed hct
    simp only [Value.wellFormed] at hw
    split at hw
    · next ctys rem rts hcty =>
      simp only [Bool.and_eq_true] at hw
      intro w hwm
      obtain ⟨t', ht', hwc⟩ := conformsAll_mem hw.2 w hwm
      have hnr := not_ref_of_conforms hwc (closureType?_captured_value hcty t' ht')
      cases w <;> first | rfl | exact absurd rfl (hnr _)
    · simp at hw
  -- The closure's tree on top of the shadow stack.
  have hg := C.fits
  have hfs := frame_stack_fits hg
  obtain ⟨ct, gst, hgst⟩ : ∃ ct gst, g.stack = ct :: gst := by
    cases hgs : g.stack with
    | nil => rw [hgs, hstk] at hfs; simp [IdTree.fitsAll] at hfs
    | cons ct gst => exact ⟨ct, gst, rfl⟩
  rw [hstk, hgst] at hfs
  simp only [IdTree.fitsAll, Bool.and_eq_true] at hfs
  obtain ⟨hctf, hgsf⟩ := hfs
  obtain ⟨i, cts⟩ := ct
  have hcts : IdTree.fitsAll cts cs = true := by
    simpa [IdTree.fits, Value.children] using hctf
  have hgl' := fitsAll_length hgsf
  have hctOcc : ∀ o ∈ IdTree.occurrencesAll cts cs, o ∈ h.occurrences s := by
    intro o ho
    refine tf_pair_occ C.head ?_
    rw [frame_occurrences, hgst, hstk]
    refine List.mem_append_right _ ?_
    simp only [IdTree.occurrencesAll, IdTree.occurrences, Value.children, List.mem_append,
      List.mem_cons]
    exact Or.inl (Or.inr ho)
  obtain ⟨frames, hsto, hhe, fresh⟩ := h
  have hh := C.hh
  simp only at hh
  subst hh
  obtain ⟨hpost, -⟩ := callClosure_shadow (sig := sig) (callee := callee) hstk hgst
    (by rw [hgl']; exact hn) hn h2f hex
  rw [ShadowPost.running.injEq] at hpost
  subst hpost
  refine ⟨fun g₂ fr₂ hz => ?_, fun f hf => ?_⟩
  · rw [h2f] at hz
    simp only [List.zip_cons_cons, List.mem_cons, Prod.mk.injEq] at hz
    rcases hz with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩ | hz
    · refine ⟨fun t ht => by simp at ht, fun j t ht _ => ?_⟩
      simp only [List.getElem?_toArray] at ht
      have hm := List.mem_of_getElem? ht
      simp only [List.mem_append, List.mem_map, List.mem_replicate] at hm
      rcases hm with ⟨u, hu, hut⟩ | ⟨-, hne⟩
      · simp only [Option.some.injEq] at hut
        subst hut
        rcases maskCompose_mem hu with hu | hu
        · -- A captured tree: a child of the closure's, of a value that is no reference.
          obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hu
          have hkl : k < cs.length := by
            have := (List.getElem?_eq_some_iff.1 hk).1
            rw [← fitsAll_length hcts]; exact this
          have hw : (u, cs[k]) ∈ cts.zip cs := zip_of_getElem? hk (List.getElem?_eq_getElem hkl)
          exact Or.inr (Or.inr (Or.inr ⟨cs[k], hctOcc _ (occ_of_zip hw _ (tf_root_occ u _)),
            hcNR _ (List.getElem_mem hkl)⟩))
        · exact C.stack_origin (by
            rw [hgst]; exact List.mem_cons_of_mem _ (List.mem_of_mem_take (List.mem_reverse.1 hu)))
      · cases hne
    · refine ⟨fun t ht => C.stack_origin ?_, fun j t ht hav => C.local_origin ht hav⟩
      rw [hgst]
      exact List.mem_cons_of_mem _ (List.mem_of_mem_drop ht)
    · exact ⟨fun t ht => Or.inl ⟨_, _, C.below hz, ht⟩,
        fun j t ht hav => Or.inr (Or.inl ⟨_, _, j, C.below hz, ht, hav⟩)⟩
  · rw [h2f] at hf
    simp only [List.tail_cons, List.mem_cons] at hf
    rcases hf with rfl | hf
    · exact ⟨info, code, _, C.act.fn, C.act.body, C.act.reachable, C.hi, Or.inr ⟨sig, rfl⟩⟩
    · exact C.calls f (by rw [C.sf]; exact hf)

/-- **`Ret`**: the caller resumes with its trees and the callee's results;
its available locals were available at the call it is at. -/
private theorem ret (C : StepCtx P s h E fr rest g hs info code .ret)
    (hst : stepOf (Step.exec P s fr rest info code .ret) = .done (.running s'))
    (hex : Shadow.exec P info.moduleId .ret s fr (.running s') h = some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨caller, rest', rfl, caller', hs'f, hfun, hpc, hloc⟩ := tf_ret_frames hst
  cases hs with
  | nil =>
    have hfit := C.trace.pre.fits
    rw [shadow_fits, C.sf, C.hh] at hfit
    simp [framesFit] at hfit
  | cons cg hs' =>
  simp only [Shadow.exec, C.hh, Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hex
  obtain ⟨rfl, -⟩ := hex
  have hcz : (cg, caller) ∈ h.frames.zip s.frames := C.below (List.mem_cons_self ..)
  obtain ⟨cinfo, ccode, chd, hcf, hcc, hcr, hci⟩ :=
    C.calls caller (by rw [C.sf]; exact List.mem_cons_self ..)
  refine ⟨fun g₂ fr₂ hz => ?_, fun f hf => ?_⟩
  · rw [hs'f] at hz
    simp only [List.zip_cons_cons, List.mem_cons, Prod.mk.injEq] at hz
    rcases hz with ⟨rfl, rfl⟩ | hz
    · refine ⟨fun t ht => ?_, fun j t ht hav => ?_⟩
      · rcases List.mem_append.1 ht with ht | ht
        · exact C.stack_origin ht
        · exact Or.inl ⟨_, _, hcz, ht⟩
      · rw [hfun, hpc] at hav
        exact Or.inr (Or.inl ⟨_, _, j, hcz, ht, tf_avail_call C.cert hcf hcc hcr hci.1 hci.2 hav⟩)
    · have hz' : (g₂, fr₂) ∈ h.frames.zip s.frames :=
        C.below (List.mem_cons_of_mem _ hz)
      exact ⟨fun t ht => Or.inl ⟨_, _, hz', ht⟩,
        fun j t ht hav => Or.inr (Or.inl ⟨_, _, j, hz', ht, hav⟩)⟩
  · rw [hs'f] at hf
    exact C.calls f (by rw [C.sf]; exact List.mem_cons_of_mem _ hf)

end StepCtx

/-! ## Writes in place -/

/-- **The frames after a root's tree is replaced**: the same trees, but for
the root's slot, which holds the new tree. -/
theorem tf_withRootTree {h₁ h₂ : Shadow} {s : State} {root : Root} {T : IdTree}
    (hw : h₁.withRootTree? s root T = some h₂) :
    h₂.frames.length = h₁.frames.length ∧
      ∀ (p : Nat) (g₂ : ShadowFrame), h₂.frames[p]? = some g₂ → ∃ g₁ : ShadowFrame,
        h₁.frames[p]? = some g₁ ∧ g₂.stack = g₁.stack ∧
        ∀ (j : Nat) (t : IdTree), g₂.locals[j]? = some (some t) →
          g₁.locals[j]? = some (some t) ∨ t = T := by
  cases root with
  | local_ f i =>
    simp only [Shadow.withRootTree?, Option.bind_eq_bind, Option.bind_eq_some_iff,
      Option.pure_def, Option.some.injEq] at hw
    obtain ⟨k, -, gk, hgk, -, -, rfl⟩ := hw
    refine ⟨by simp, fun p g₂ hp => ?_⟩
    simp only [List.getElem?_set] at hp
    split at hp
    · next hpk =>
      subst hpk
      split at hp
      · simp only [Option.some.injEq] at hp
        subst hp
        refine ⟨gk, hgk, rfl, fun j t ht => ?_⟩
        by_cases hji : j = i
        · subst hji
          right
          simp only [Array.getElem?_setIfInBounds, if_true] at ht
          split at ht
          · simpa using ht.symm
          · cases ht
        · left
          rwa [Array.getElem?_setIfInBounds_ne hji] at ht
      · cases hp
    · exact ⟨g₂, hp, rfl, fun j t ht => Or.inl ht⟩
  | global a ty =>
    simp only [Shadow.withRootTree?, Option.bind_eq_bind, Option.bind_eq_some_iff,
      Option.pure_def, Option.some.injEq] at hw
    obtain ⟨k, -, -, -, rfl⟩ := hw
    exact ⟨rfl, fun p g₂ hp => ⟨g₂, hp, rfl, fun j t ht => Or.inl ht⟩⟩
  | held k =>
    simp only [Shadow.withRootTree?, Option.bind_eq_bind, Option.bind_eq_some_iff,
      Option.pure_def, Option.some.injEq] at hw
    obtain ⟨-, -, rfl⟩ := hw
    exact ⟨rfl, fun p g₂ hp => ⟨g₂, hp, rfl, fun j t ht => Or.inl ht⟩⟩

/-- The root of a location a reference on the operand stack designates holds
a value that is no reference, whose tree's identity is the root tree's. -/
theorem tf_root_value {P : Program} {s : State} {h : Shadow} {E : List Event} {fr : Frame}
    {rest : List Frame} (hI : RefTraceInv P s h E) (hsf : s.frames = fr :: rest) {r : Reference}
    (hr : Value.ref r ∈ fr.stack) {w : Value} (hread : s.read? r.location = some w) {t₀ : IdTree}
    (ht₀ : h.rootTree? s r.location.root = some t₀) :
    ∃ v, (t₀.id, v) ∈ h.occurrences s ∧ v.isRef = false := by
  simp only [State.read?, bind, Option.bind_eq_some_iff] at hread
  obtain ⟨rv, hrv, hget⟩ := hread
  obtain ⟨t, ht, -, R, hR⟩ := root_read hI.pre.fits hrv
  rw [ht₀, Option.some.injEq] at ht
  subst ht
  refine ⟨rv, hR.symm.subset (List.mem_append_left _ (tf_root_occ _ rv)), ?_⟩
  cases hp : r.location.path with
  | nil =>
    rw [hp] at hget
    simp only [Value.get?, Option.some.injEq] at hget
    subst hget
    exact hI.pre.refsTop.deref fr (by rw [hsf]; exact List.mem_cons_self ..) r (Or.inl hr) _
      (by simp only [State.read?, bind, Option.bind_eq_some_iff]; exact ⟨_, hrv, by rw [hp]; rfl⟩)
      _ (self_mem_subvalues _)
  | cons st p =>
    rw [hp] at hget
    cases rv with
    | ref r' => simp [Value.get?, Value.child?] at hget
    | _ => rfl

/-- A tree set below a root's tree keeps the root's identity. -/
theorem tf_set_id {u t u' : IdTree} {i : Nat} {p : Path} (h : u.set? (i :: p) t = some u') :
    u'.id = u.id := by
  obtain ⟨id, cs⟩ := u
  simp only [IdTree.set?, IdTree.children, Option.bind_eq_bind, Option.bind_eq_some_iff,
    Option.pure_def, Option.some.injEq] at h
  obtain ⟨c, -, c', -, rfl⟩ := h
  rfl

/-- Setting a root's value updates its frame's local, if it is one. -/
theorem tf_setRoot_frames (s : State) (l : Location) (w' : Value) :
    (s.setRoot l.root w').frames = s.frames.map (updFrame l w') := by
  cases hρ : l.root with
  | local_ f i =>
    simp only [State.setRoot]
    refine List.map_congr_left fun fr _ => ?_
    simp only [updFrame, hρ]
  | global a t =>
    simp only [State.setRoot]
    conv => lhs; rw [← List.map_id s.frames]
    refine List.map_congr_left fun fr _ => ?_
    simp only [updFrame, hρ, id]
  | held k =>
    simp only [State.setRoot]
    conv => lhs; rw [← List.map_id s.frames]
    refine List.map_congr_left fun fr _ => ?_
    simp only [updFrame, hρ, id]

/-- **The frames after a write in place** and the end of the instruction. -/
theorem tf_update_frames {s₁ s₂ s' : State} {fr₁ : Frame} {rest : List Frame} {l : Location}
    {v : Value} {vals : List Value} (hs₁ : s₁.frames = fr₁ :: rest)
    (hu : Step.update s₁ l v = .ok s₂) (h : stepOf (Step.finish s₂ vals) = .done (.running s')) :
    ∃ w', s'.frames =
      Step.advance (Step.pushAll (updFrame l w' fr₁) vals) :: rest.map (updFrame l w') := by
  obtain ⟨s₃, hw, rfl⟩ := update_ok hu
  simp only [State.write?, bind, Option.bind_eq_some_iff, pure] at hw
  obtain ⟨w, -, w', -, hw⟩ := hw
  simp only [Option.some.injEq] at hw
  subst hw
  have hfr : ((s₁.setRoot l.root w').invalidate (.mutated l)).frames =
      updFrame l w' fr₁ :: rest.map (updFrame l w') := by
    simp only [State.invalidate, tf_setRoot_frames, hs₁, List.map_cons]
  obtain rfl := Outcome.running.inj (done_finish hfr h)
  exact ⟨w', rfl⟩

/-- A shadow whose current frame's operand stack changed has the same root
trees. -/
theorem tf_rootTree_stack {h : Shadow} {g : ShadowFrame} {hs : List ShadowFrame}
    (hh : h.frames = g :: hs) (st : List IdTree) (s : State) (root : Root) :
    ({ h with frames := { g with stack := st } :: hs } : Shadow).rootTree? s root =
      h.rootTree? s root := by
  cases root with
  | local_ f i =>
    simp only [Shadow.rootTree?, hh]
    cases Shadow.framePos s f with
    | none => rfl
    | some k =>
      cases k <;> rfl
  | global a t => rfl
  | held k => rfl

namespace StepCtx

variable {P : Program} {s s' : State} {h h' : Shadow} {E evs : List Event} {fr : Frame}
  {rest : List Frame} {g : ShadowFrame} {hs : List ShadowFrame} {info : FunctionInfo}
  {code : CodeUnit}

/-- **A write in place**: the frames keep their trees, but the written root's
slot, whose new tree comes from before; the current frame's operand stack
shrinks, or receives a tree that comes from before. -/
private theorem write {instr : Bytecode} (C : StepCtx P s h E fr rest g hs info code instr)
    (hctl : instr.control = .fallThrough)
    (hid : availabilityAfter instr (modelAvail P fr.function fr.pc) =
      modelAvail P fr.function fr.pc)
    {n : Nat} {l : Location} {w' : Value} {fr' : Frame} {T' : IdTree} {h₂ : Shadow}
    (hs' : s'.frames = fr' :: rest.map (updFrame l w'))
    (hfun : fr'.function = fr.function) (hpc : fr'.pc = fr.pc + 1)
    (hw : ({ h with frames := { g with stack := g.stack.drop n } :: hs } : Shadow).withRootTree?
      s l.root T' = some h₂)
    (hT' : Origin P s h T')
    (hh' : h'.frames = h₂.frames ∨ ∃ last g₂ hs₂, h₂.frames = g₂ :: hs₂ ∧
      h'.frames = { g₂ with stack := last :: g₂.stack } :: hs₂ ∧ Origin P s h last) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨-, hpos⟩ := tf_withRootTree hw
  have hsucc : fr'.pc ∈ successors code.code fr.pc := by
    simp [successors, C.hi, hctl, hpc]
  refine ⟨fun g₂' fr₂' hz => ?_, fun f hf => ?_⟩
  · obtain ⟨p, hp⟩ := List.mem_iff_getElem?.1 hz
    rw [List.getElem?_zip_eq_some] at hp
    obtain ⟨hgp, hfp⟩ := hp
    -- The frame of the shadow before the operand stack received a tree.
    have hshadow : ∃ g₂ : ShadowFrame, h₂.frames[p]? = some g₂ ∧ g₂'.locals = g₂.locals ∧
        ∀ t ∈ g₂'.stack, t ∈ g₂.stack ∨ Origin P s h t := by
      rcases hh' with hh' | ⟨last, g₂, hs₂, hh₂, hh', hlast⟩
      · rw [hh'] at hgp
        exact ⟨g₂', hgp, rfl, fun t ht => Or.inl ht⟩
      · rw [hh'] at hgp
        rw [hh₂]
        cases p with
        | zero =>
          simp only [List.getElem?_cons_zero, Option.some.injEq] at hgp
          subst hgp
          refine ⟨g₂, rfl, rfl, fun t ht => ?_⟩
          rcases List.mem_cons.1 ht with rfl | ht
          · exact Or.inr hlast
          · exact Or.inl ht
        | succ q => exact ⟨g₂', hgp, rfl, fun t ht => Or.inl ht⟩
    obtain ⟨g₂, hg₂, hloc₂, hstk₂⟩ := hshadow
    obtain ⟨g₁, hg₁, hstk₁, hloc₁⟩ := hpos p g₂ hg₂
    cases p with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hg₁
      rw [hs', List.getElem?_cons_zero, Option.some.injEq] at hfp
      subst hg₁ hfp
      refine ⟨fun t ht => ?_, fun j t ht hav => ?_⟩
      · rcases hstk₂ t ht with ht | ho
        · rw [hstk₁] at ht
          exact C.stack_origin (List.mem_of_mem_drop ht)
        · exact ho
      · rw [hloc₂] at ht
        rcases hloc₁ j t ht with ht | rfl
        · rw [hfun] at hav
          have := C.succ_avail hsucc hav
          rw [hid] at this
          exact C.local_origin ht this
        · exact hT'
    | succ q =>
      simp only [List.getElem?_cons_succ] at hg₁
      rw [hs', List.getElem?_cons_succ, List.getElem?_map] at hfp
      obtain ⟨fr₂, hfr₂, rfl⟩ := Option.map_eq_some_iff.1 hfp
      have hz₀ : (g₁, fr₂) ∈ h.frames.zip s.frames := C.below (zip_of_getElem? hg₁ hfr₂)
      obtain ⟨-, hfun₂, hpc₂, -⟩ := updFrame_eq l w' fr₂
      refine ⟨fun t ht => ?_, fun j t ht hav => ?_⟩
      · rcases hstk₂ t ht with ht | ho
        · rw [hstk₁] at ht
          exact Or.inl ⟨_, _, hz₀, ht⟩
        · exact ho
      · rw [hloc₂] at ht
        rcases hloc₁ j t ht with ht | rfl
        · rw [hfun₂, hpc₂] at hav
          exact Or.inr (Or.inl ⟨_, _, j, hz₀, ht, hav⟩)
        · exact hT'
  · rw [hs'] at hf
    simp only [List.tail_cons, List.mem_map] at hf
    obtain ⟨f₀, hf₀, rfl⟩ := hf
    obtain ⟨-, hfun₀, hpc₀, -⟩ := updFrame_eq l w' f₀
    rw [hfun₀, hpc₀]
    exact C.calls f₀ (by rw [C.sf]; exact hf₀)

end StepCtx

section Sem

open MoveBytecode.Step

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

/-- The function and position after a write in place. -/
theorem tf_advance_upd (l : Location) (w' : Value) (fr₁ : Frame) (vals : List Value) :
    (advance (pushAll (updFrame l w' fr₁) vals)).function = fr₁.function ∧
      (advance (pushAll (updFrame l w' fr₁) vals)).pc = fr₁.pc + 1 := by
  obtain ⟨-, hfun, hpc, -⟩ := updFrame_eq l w' fr₁
  exact ⟨hfun, congrArg (· + 1) hpc⟩

theorem tf_sem_writeRef (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code .writeRef) = .done (.running s')) :
    ∃ r st, fr.stack = .ref r :: st ∧ (∃ w, s.read? r.location = some w) ∧
      ∃ w' fr', s'.frames = fr' :: rest.map (updFrame r.location w') ∧
        fr'.function = fr.function ∧ fr'.pc = fr.pc + 1 := by
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRef_ends
  obtain ⟨st₁, hst₁, rfl⟩ := popRef_ok hpr
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨⟨v, fr₂⟩, hpv, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st, -, rfl⟩ := pop_ok hpv
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨old, hold, h⟩ := done_bind h rfl deref_ends
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨s₂', hu, h⟩ := done_bind h rfl update_ends
  obtain ⟨w', hs'⟩ := tf_update_frames rfl hu h
  have hd := deref_ok hold
  rw [deref_eq (s' := { s with frames := { fr with stack := st } :: rest }) (s := s)
    (by simp [hsf]) rfl rfl rfl] at hd
  obtain ⟨hf, hp⟩ := tf_advance_upd r.location w' { { fr with stack := st₁ } with stack := st } []
  exact ⟨r, st₁, hst₁, ⟨old, deref?_read hd⟩, w', _, hs', hf, hp⟩

end Sem

namespace StepCtx

variable {P : Program} {s s' : State} {h h' : Shadow} {E evs : List Event} {fr : Frame}
  {rest : List Frame} {g : ShadowFrame} {hs : List ShadowFrame} {info : FunctionInfo}
  {code : CodeUnit}

/-- The new tree of a written root comes from before: the root's identity, or
the written value's tree. -/
private theorem written_origin {instr : Bytecode} (C : StepCtx P s h E fr rest g hs info code instr)
    {r : Reference} (hr : Value.ref r ∈ fr.stack) {w : Value}
    (hread : s.read? r.location = some w) {st : List IdTree} {t₀ T T' : IdTree}
    (ht₀ : ({ h with frames := { g with stack := st } :: hs } : Shadow).rootTree? s r.location.root =
      some t₀) {rv : Value}
    (hT' : t₀.set? (r.location.indicesIn rv) T = some T')
    (hT : r.location.path = [] → Origin P s h T) : Origin P s h T' := by
  rw [tf_rootTree_stack C.hh] at ht₀
  cases hp : r.location.path with
  | nil =>
    simp only [Location.indicesIn, hp, Value.indicesOf, IdTree.set?, Option.some.injEq] at hT'
    subst hT'
    exact hT hp
  | cons st' p =>
    simp only [Location.indicesIn, hp, Value.indicesOf] at hT'
    obtain ⟨v, hv, hvr⟩ := tf_root_value C.trace C.sf hr hread ht₀
    exact Or.inr (Or.inr (Or.inr ⟨v, by rw [tf_set_id hT']; exact hv, hvr⟩))

/-- **`WriteRef`.** -/
private theorem writeRef (C : StepCtx P s h E fr rest g hs info code .writeRef)
    (hst : stepOf (Step.exec P s fr rest info code .writeRef) = .done (.running s'))
    (hex : Shadow.exec P info.moduleId .writeRef s fr (.running s') h = some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨r, st, hfs, ⟨w, hread⟩, w', fr', hs', hfun, hpc⟩ := tf_sem_writeRef C.sf hst
  simp only [Shadow.exec, C.hh, Option.bind_eq_bind, Option.pure_def] at hex
  obtain ⟨x, hra, hex⟩ := Option.bind_eq_some_iff.1 hex
  obtain ⟨tv, htv, hex⟩ := Option.bind_eq_some_iff.1 hex
  obtain ⟨h₂, hch, hex⟩ := Option.bind_eq_some_iff.1 hex
  simp only [Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hex
  obtain ⟨rfl, -⟩ := hex
  obtain ⟨_, -, hch⟩ := Option.bind_eq_some_iff.1 hch
  obtain ⟨vt, -, hch⟩ := Option.bind_eq_some_iff.1 hch
  change Shadow.withTree? _ s _ tv = some h₂ at hch
  simp only [Shadow.withTree?, Option.bind_eq_bind] at hch
  obtain ⟨t₀, ht₀, hch⟩ := Option.bind_eq_some_iff.1 hch
  obtain ⟨_, -, hch⟩ := Option.bind_eq_some_iff.1 hch
  obtain ⟨T', hT', hw⟩ := Option.bind_eq_some_iff.1 hch
  obtain ⟨h1, -, -⟩ := refAt_spec hra
  rw [hfs] at h1
  simp only [List.getElem?_cons_zero, Option.some.injEq, Value.ref.injEq] at h1
  rw [← h1] at hw ht₀ hT'
  exact C.write rfl rfl hs' hfun hpc hw
    (C.written_origin (by rw [hfs]; exact List.mem_cons_self ..) hread ht₀ hT'
      fun _ => C.stack_origin (List.mem_of_getElem? htv)) (Or.inl rfl)

end StepCtx

section Sem

open MoveBytecode.Step

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

theorem tf_sem_vecPushBack {sig : SignatureIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.vecPushBack sig)) = .done (.running s')) :
    ∃ r, fr.stack[1]? = some (.ref r) ∧ (∃ w, s.read? r.location = some w) ∧
      ∃ w' fr', s'.frames = fr' :: rest.map (updFrame r.location w') ∧
        fr'.function = fr.function ∧ fr'.pc = fr.pc + 1 := by
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨v, fr₁⟩, hpv, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st₁, hst₁, rfl⟩ := pop_ok hpv
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, hst₂, rfl, -, -⟩ := popRefTo_ok hpr
  simp only at hst₂
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨elems, helems, h⟩ := done_bind h rfl derefVector_ends
  obtain ⟨s₂', hu, h⟩ := done_bind h rfl update_ends
  obtain ⟨w', hs'⟩ := tf_update_frames rfl hu h
  obtain ⟨t', hd⟩ := derefVector_ok helems
  rw [deref_eq (s' := { s with frames := { fr with stack := st } :: rest }) (s := s)
    (by simp [hsf]) rfl rfl rfl] at hd
  obtain ⟨hf, hp⟩ := tf_advance_upd r.location w' { { fr with stack := st₁ } with stack := st } []
  exact ⟨r, by rw [hst₁, hst₂]; rfl, ⟨_, deref?_read hd⟩, w', _, hs', hf, hp⟩

theorem tf_sem_vecPopBack {sig : SignatureIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.vecPopBack sig)) = .done (.running s')) :
    ∃ r, fr.stack[0]? = some (.ref r) ∧ (∃ w, s.read? r.location = some w) ∧
      ∃ w' fr', s'.frames = fr' :: rest.map (updFrame r.location w') ∧
        fr'.function = fr.function ∧ fr'.pc = fr.pc + 1 := by
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, hstk, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  obtain ⟨elems, helems, h⟩ := done_bind h rfl derefVector_ends
  split at h
  · next last init _ =>
    obtain ⟨s₂', hu, h⟩ := done_bind h rfl update_ends
    obtain ⟨w', hs'⟩ := tf_update_frames rfl hu h
    obtain ⟨t', hd⟩ := derefVector_ok helems
    rw [deref_eq (s' := { s with frames := { fr with stack := st } :: rest }) (s := s)
      (by simp [hsf]) rfl rfl rfl] at hd
    obtain ⟨hf, hp⟩ := tf_advance_upd r.location w' { fr with stack := st } [last]
    exact ⟨r, by rw [hstk]; rfl, ⟨_, deref?_read hd⟩, w', _, hs', hf, hp⟩
  · exact (done_fail h rfl).elim

theorem tf_sem_vecSwap {sig : SignatureIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.vecSwap sig)) = .done (.running s')) :
    ∃ r, fr.stack[2]? = some (.ref r) ∧ (∃ w, s.read? r.location = some w) ∧
      ∃ w' fr', s'.frames = fr' :: rest.map (updFrame r.location w') ∧
        fr'.function = fr.function ∧ fr'.pc = fr.pc + 1 := by
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨j, fr₁⟩, hpj, h⟩ := done_bind h rfl popIndex_ends
  obtain ⟨jx, st₁, hst₁, rfl, rfl⟩ := popIndex_ok hpj
  obtain ⟨⟨i, fr₂⟩, hpi, h⟩ := done_bind h rfl popIndex_ends
  obtain ⟨ix, st₂, hst₂, rfl, rfl⟩ := popIndex_ok hpi
  obtain ⟨⟨r, fr₃⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, hst₃, rfl, -, -⟩ := popRefTo_ok hpr
  simp only at hst₂ hst₃
  dsimp only at h
  obtain ⟨elems, helems, h⟩ := done_bind h rfl derefVector_ends
  split at h
  · obtain ⟨s₂', hu, h⟩ := done_bind h rfl update_ends
    obtain ⟨w', hs'⟩ := tf_update_frames rfl hu h
    obtain ⟨t', hd⟩ := derefVector_ok helems
    rw [deref_eq (s' := { s with frames := { fr with stack := st } :: rest }) (s := s)
      (by simp [hsf]) rfl rfl rfl] at hd
    obtain ⟨hf, hp⟩ := tf_advance_upd r.location w'
      { { { fr with stack := st₁ } with stack := st₂ } with stack := st } []
    exact ⟨r, by rw [hst₁, hst₂, hst₃]; rfl, ⟨_, deref?_read hd⟩, w', _, hs', hf, hp⟩
  · exact (done_fail h rfl).elim
  · exact (done_fail h rfl).elim

end Sem

/-- The tree at a location of a root's tree, when the location is the root. -/
theorem tf_tree_root {h : Shadow} {s : State} {l : Location} {vt t₀ : IdTree}
    (htree : h.tree? s l = some vt) (ht₀ : h.rootTree? s l.root = some t₀)
    (hidx : l.path = []) : vt = t₀ := by
  simp only [Shadow.tree?, ht₀, bind, Option.bind_some, Option.bind_eq_some_iff] at htree
  obtain ⟨rv, -, htree⟩ := htree
  simp only [Location.indicesIn, hidx, Value.indicesOf, IdTree.at?, Option.some.injEq] at htree
  exact htree.symm

namespace StepCtx

variable {P : Program} {s s' : State} {h h' : Shadow} {E evs : List Event} {fr : Frame}
  {rest : List Frame} {g : ShadowFrame} {hs : List ShadowFrame} {info : FunctionInfo}
  {code : CodeUnit}

/-- The new tree of a vector written in place keeps the vector's identity,
the root's when the vector is the root. -/
private theorem vector_origin {instr : Bytecode} (C : StepCtx P s h E fr rest g hs info code instr)
    {r : Reference} (hr : Value.ref r ∈ fr.stack) {w : Value}
    (hread : s.read? r.location = some w) {st : List IdTree} {t₀ vt T' : IdTree}
    {cs : List IdTree}
    (htree : ({ h with frames := { g with stack := st } :: hs } : Shadow).tree? s r.location =
      some vt)
    (ht₀ : ({ h with frames := { g with stack := st } :: hs } : Shadow).rootTree? s r.location.root =
      some t₀)
    {rv : Value} (hT' : t₀.set? (r.location.indicesIn rv) (.node vt.id cs) = some T') :
    Origin P s h T' := by
  refine C.written_origin hr hread ht₀ hT' fun hidx => ?_
  have := tf_tree_root htree ht₀ hidx
  subst this
  rw [tf_rootTree_stack C.hh] at ht₀
  obtain ⟨v, hv, hvr⟩ := tf_root_value C.trace C.sf hr hread ht₀
  exact Or.inr (Or.inr (Or.inr ⟨v, hv, hvr⟩))

/-- **`VecPushBack`.** -/
private theorem vecPushBack {sig : SignatureIndex}
    (C : StepCtx P s h E fr rest g hs info code (.vecPushBack sig))
    (hst : stepOf (Step.exec P s fr rest info code (.vecPushBack sig)) = .done (.running s'))
    (hex : Shadow.exec P info.moduleId (.vecPushBack sig) s fr (.running s') h =
      some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨r, hfs, ⟨w, hread⟩, w', fr', hs', hfun, hpc⟩ := tf_sem_vecPushBack C.sf hst
  simp only [Shadow.exec, C.hh, Option.bind_eq_bind, Option.pure_def] at hex
  obtain ⟨tv, -, hex⟩ := Option.bind_eq_some_iff.1 hex
  obtain ⟨x, hra, hex⟩ := Option.bind_eq_some_iff.1 hex
  obtain ⟨h₂, hch, hex⟩ := Option.bind_eq_some_iff.1 hex
  simp only [Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hex
  obtain ⟨rfl, -⟩ := hex
  obtain ⟨_, -, hch⟩ := Option.bind_eq_some_iff.1 hch
  obtain ⟨vt, htree, hch⟩ := Option.bind_eq_some_iff.1 hch
  change Shadow.withTree? _ s _ _ = some h₂ at hch
  simp only [Shadow.withTree?, Option.bind_eq_bind] at hch
  obtain ⟨t₀, ht₀, hch⟩ := Option.bind_eq_some_iff.1 hch
  obtain ⟨_, -, hch⟩ := Option.bind_eq_some_iff.1 hch
  obtain ⟨T', hT', hw⟩ := Option.bind_eq_some_iff.1 hch
  obtain ⟨h1, -, -⟩ := refAt_spec hra
  rw [hfs, Option.some.injEq, Value.ref.injEq] at h1
  rw [← h1] at hw ht₀ hT' htree
  exact C.write rfl rfl hs' hfun hpc hw
    (C.vector_origin (List.mem_of_getElem? hfs) hread htree ht₀ hT') (Or.inl rfl)

/-- **`VecPopBack`**: the popped element's tree is a child of the vector's,
of a value that is no reference. -/
private theorem vecPopBack {sig : SignatureIndex}
    (C : StepCtx P s h E fr rest g hs info code (.vecPopBack sig))
    (hst : stepOf (Step.exec P s fr rest info code (.vecPopBack sig)) = .done (.running s'))
    (hex : Shadow.exec P info.moduleId (.vecPopBack sig) s fr (.running s') h =
      some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨r, hfs, ⟨w, hread⟩, w', fr', hs', hfun, hpc⟩ := tf_sem_vecPopBack C.sf hst
  simp only [Shadow.exec, C.hh, Option.bind_eq_bind, Option.pure_def] at hex
  obtain ⟨x, hra, hex⟩ := Option.bind_eq_some_iff.1 hex
  obtain ⟨tw, htw, hex⟩ := Option.bind_eq_some_iff.1 hex
  obtain ⟨last, hlast, hex⟩ := Option.bind_eq_some_iff.1 hex
  obtain ⟨h₂, hch, hex⟩ := Option.bind_eq_some_iff.1 hex
  obtain ⟨h1, -, -⟩ := refAt_spec hra
  rw [hfs, Option.some.injEq, Value.ref.injEq] at h1
  subst h1
  split at hex
  · next g₂ hs₂ hh₂ =>
    simp only [Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hex
    obtain ⟨rfl, -⟩ := hex
    obtain ⟨_, -, hch⟩ := Option.bind_eq_some_iff.1 hch
    obtain ⟨vt, htree, hch⟩ := Option.bind_eq_some_iff.1 hch
    change Shadow.withTree? _ s _ _ = some h₂ at hch
    simp only [Shadow.withTree?, Option.bind_eq_bind] at hch
    obtain ⟨t₀, ht₀, hch⟩ := Option.bind_eq_some_iff.1 hch
    obtain ⟨_, -, hch⟩ := Option.bind_eq_some_iff.1 hch
    obtain ⟨T', hT', hw⟩ := Option.bind_eq_some_iff.1 hch
    have hr : Value.ref x.2 ∈ fr.stack := List.mem_of_getElem? hfs
    -- The popped element.
    have hlo : Origin P s h last := by
      obtain ⟨tw', htw', hfw, R, hR⟩ := tree_read C.trace.pre.fits hread
      rw [htw', Option.some.injEq] at htw
      subst htw
      obtain ⟨cv, hcv, hcvs⟩ := tf_child_occ hfw (List.mem_of_getLast? hlast)
      refine Or.inr (Or.inr (Or.inr ⟨cv, hR.symm.subset (List.mem_append_left _ hcv), ?_⟩))
      exact C.trace.pre.refsTop.deref fr (by rw [C.sf]; exact List.mem_cons_self ..) x.2
        (Or.inl hr) w hread cv hcvs
    exact C.write rfl rfl hs' hfun hpc hw (C.vector_origin hr hread htree ht₀ hT')
      (Or.inr ⟨last, g₂, hs₂, hh₂, rfl, hlo⟩)
  · cases hex

/-- **`VecSwap`.** -/
private theorem vecSwap {sig : SignatureIndex}
    (C : StepCtx P s h E fr rest g hs info code (.vecSwap sig))
    (hst : stepOf (Step.exec P s fr rest info code (.vecSwap sig)) = .done (.running s'))
    (hex : Shadow.exec P info.moduleId (.vecSwap sig) s fr (.running s') h =
      some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨r, hfs, ⟨w, hread⟩, w', fr', hs', hfun, hpc⟩ := tf_sem_vecSwap C.sf hst
  simp only [Shadow.exec, C.hh, Option.bind_eq_bind, Option.pure_def] at hex
  split at hex
  · split at hex
    · obtain ⟨x, hra, hex⟩ := Option.bind_eq_some_iff.1 hex
      obtain ⟨h₂, hch, hex⟩ := Option.bind_eq_some_iff.1 hex
      simp only [Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hex
      obtain ⟨rfl, -⟩ := hex
      obtain ⟨_, -, hch⟩ := Option.bind_eq_some_iff.1 hch
      obtain ⟨vt, htree, hch⟩ := Option.bind_eq_some_iff.1 hch
      obtain ⟨T, hT, hch⟩ := Option.bind_eq_some_iff.1 hch
      obtain ⟨_, -, hT⟩ := Option.bind_eq_some_iff.1 hT
      obtain ⟨_, -, hT⟩ := Option.bind_eq_some_iff.1 hT
      simp only [Option.some.injEq] at hT
      subst hT
      simp only [Shadow.withTree?, Option.bind_eq_bind] at hch
      obtain ⟨t₀, ht₀, hch⟩ := Option.bind_eq_some_iff.1 hch
      obtain ⟨_, -, hch⟩ := Option.bind_eq_some_iff.1 hch
      obtain ⟨T', hT', hw⟩ := Option.bind_eq_some_iff.1 hch
      obtain ⟨h1, -, -⟩ := refAt_spec hra
      rw [hfs, Option.some.injEq, Value.ref.injEq] at h1
      rw [← h1] at hw ht₀ hT' htree
      exact C.write rfl rfl hs' hfun hpc hw
        (C.vector_origin (List.mem_of_getElem? hfs) hread htree ht₀ hT') (Or.inl rfl)
    · cases hex
  · cases hex

end StepCtx

/-- An annotated core step that continues runs an instruction of the current
frame, and the ghost layer annotates that instruction. -/
theorem tf_unfold {P : Program} {s s' : State} {h h' : Shadow} {evs : List Event}
    (hstep : step P s = .done (.running s'))
    (hc : Shadow.coreStep P s (.running s') h = some (.running h', evs)) :
    ∃ fr rest info code instr, s.frames = fr :: rest ∧ P.function? fr.function = some info ∧
      info.definition.code = some code ∧ code.code[fr.pc]? = some instr ∧
      stepOf (Step.exec P s fr rest info code instr) = .done (.running s') ∧
      Shadow.exec P info.moduleId instr s fr (.running s') h = some (.running h', evs) := by
  unfold step at hstep
  split at hstep
  · simp at hstep
  · next fr rest hsf =>
    split at hstep
    · simp at hstep
    · next info hinfo =>
      split at hstep
      · simp at hstep
      · next code hcode =>
        split at hstep
        · simp at hstep
        · next instr hinstr =>
          refine ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hstep, ?_⟩
          unfold Shadow.coreStep at hc
          rw [hsf] at hc
          simp only [Option.bind_eq_bind, Option.bind_eq_some_iff, hinfo, hcode, hinstr,
            Option.some.injEq, exists_eq_left'] at hc
          obtain ⟨⟨post', evs'⟩, hex, hc⟩ := hc
          split at hc
          · simp only [Option.some.injEq, Prod.mk.injEq] at hc
            obtain ⟨rfl, rfl⟩ := hc
            exact hex
          · cases hc

/-- **The trees of a core step that continues come from before it**, and the
suspended frames stay at calls. -/
private theorem core_origins {P : Program} {s s' : State} {h h' : Shadow} {E evs : List Event}
    (hc : Certified P (modelTypes P) (modelAvail P))
    (hI : RefTraceInv P s h E)
    (hinv : Inv P (modelTypes P) (modelAvail P) s) (hcalls : AtCalls P s)
    (hstep : step P s = .done (.running s'))
    (hcs : Shadow.coreStep P s (.running s') h = some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hi, hst, hex⟩ := tf_unfold hstep hcs
  obtain ⟨fr₀, rest₀, hsf₀, ⟨info₀, code₀, hact⟩, -⟩ := hinv
  rw [hsf, List.cons.injEq] at hsf₀
  obtain ⟨rfl, rfl⟩ := hsf₀
  have hi₀ : info = info₀ := by rw [hact.fn, Option.some.injEq] at hinfo; exact hinfo.symm
  subst hi₀
  have hc₀ : code = code₀ := by rw [hact.body, Option.some.injEq] at hcode; exact hcode.symm
  subst hc₀
  obtain ⟨hframes, hst', hhe, fresh⟩ := h
  cases hframes with
  | nil =>
    have hfit := hI.pre.fits
    rw [shadow_fits, hsf] at hfit
    simp [framesFit] at hfit
  | cons g hs' =>
  have C : StepCtx P s ⟨g :: hs', hst', hhe, fresh⟩ E fr rest g hs' info code instr :=
    ⟨hsf, rfl, hI, hact, hi, hc, hcalls⟩
  -- A generic form's step: its instantiation, its type arguments, then its helper.
  have hgen : ∀ {α : Type} {t : Option α} {sig : α → Nat}
      {k : α → List RuntimeType → Step.Exec StepResult},
      stepOf (Step.need t .badIndex >>= fun x =>
        Step.genericArgs info.module fr.function.typeArgs (sig x) >>= fun tys => k x tys) =
        .done (.running s') → ∃ x tys, stepOf (k x tys) = .done (.running s') := by
    intro α t sig k h
    obtain ⟨x, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h rfl genericArgs_ends
    exact ⟨x, tys, h⟩
  by_cases hk : instr.PushesNew = true
  · by_cases hv : instr.ValueOnly = true
    · obtain ⟨⟨fr', -, -, hs', -, hfun, -, -, -, hpc, -⟩, -⟩ := exec_value hi hv hst
      exact C.pushNew hk ⟨fr', hs', hfun, hpc⟩ hex
    · by_cases hvg : instr.ValueOnlyGeneric = true
      · obtain ⟨⟨fr', -, -, hs', -, hfun, -, -, -, hpc, -⟩, -⟩ := exec_valueGeneric hi hvg hst
        exact C.pushNew hk ⟨fr', hs', hfun, hpc⟩ hex
      cases instr
      case pop => exact C.pushNew rfl (C.nextSucc (nextPc_pop hst) rfl) hex
      case eq => exact C.pushNew rfl (C.nextSucc (nextPc_equality hst) rfl) hex
      case neq => exact C.pushNew rfl (C.nextSucc (nextPc_equality hst) rfl) hex
      case testVariant hh =>
        have hst' : stepOf (Step.testEnum P s fr rest info hh) = .done (.running s') := hst
        exact C.pushNew rfl (C.nextSucc (nextPc_testVariant hst') rfl) hex
      case vecLen => exact C.pushNew rfl (C.nextSucc (nextPc_vecLen hst) rfl) hex
      case freezeRef => exact C.pushNew rfl (C.nextSucc (nextPc_freezeRef hst) rfl) hex
      case mutBorrowLoc => exact C.pushNew rfl (C.nextSucc (nextPc_borrowLoc hst) rfl) hex
      case immBorrowLoc => exact C.pushNew rfl (C.nextSucc (nextPc_borrowLoc hst) rfl) hex
      case mutBorrowField => exact C.pushNew rfl (C.nextSucc (nextPc_borrowField hst) rfl) hex
      case immBorrowField => exact C.pushNew rfl (C.nextSucc (nextPc_borrowField hst) rfl) hex
      case mutBorrowVariantField =>
        exact C.pushNew rfl (C.nextSucc (nextPc_borrowVariantField hst) rfl) hex
      case immBorrowVariantField =>
        exact C.pushNew rfl (C.nextSucc (nextPc_borrowVariantField hst) rfl) hex
      case vecImmBorrow => exact C.pushNew rfl (C.nextSucc (nextPc_borrowElement hst) rfl) hex
      case vecMutBorrow => exact C.pushNew rfl (C.nextSucc (nextPc_borrowElement hst) rfl) hex
      case mutBorrowGlobal => exact C.pushNew rfl (C.nextSucc (nextPc_borrowGlobal hst) rfl) hex
      case immBorrowGlobal => exact C.pushNew rfl (C.nextSucc (nextPc_borrowGlobal hst) rfl) hex
      case testVariantGeneric =>
        obtain ⟨x, tys, hst'⟩ := hgen hst
        exact C.pushNew rfl (C.nextSucc (nextPc_testVariant hst') rfl) hex
      case mutBorrowFieldGeneric | immBorrowFieldGeneric =>
        obtain ⟨x, tys, hst'⟩ := hgen hst
        exact C.pushNew rfl (C.nextSucc (nextPc_borrowField hst') rfl) hex
      case mutBorrowVariantFieldGeneric | immBorrowVariantFieldGeneric =>
        obtain ⟨x, tys, hst'⟩ := hgen hst
        exact C.pushNew rfl (C.nextSucc (nextPc_borrowVariantField hst') rfl) hex
      case mutBorrowGlobalGeneric | immBorrowGlobalGeneric =>
        obtain ⟨x, tys, hst'⟩ := hgen hst
        exact C.pushNew rfl (C.nextSucc (nextPc_borrowGlobal hst') rfl) hex
      all_goals simp [Bytecode.PushesNew, Bytecode.ValueOnly, Bytecode.ValueOnlyGeneric] at hk hv hvg
  · cases instr
    case copyLoc j => exact C.copyLoc hst hex
    case moveLoc j => exact C.moveLoc hst hex
    case stLoc j => exact C.stLoc hst hex
    case readRef => exact C.readRef hst hex
    case moveFrom d =>
      have hst' : stepOf (Step.moveResourceFrom P s fr rest d) = .done (.running s') := hst
      exact C.moveFrom (Or.inl ⟨d, rfl⟩) (nextPc_moveFrom hst') hex
    case moveTo d =>
      have hst' : stepOf (Step.moveResourceTo P s fr rest d) = .done (.running s') := hst
      exact C.moveTo (Or.inl ⟨d, rfl⟩) (nextPc_moveTo hst') hex
    case moveFromGeneric i =>
      obtain ⟨x, tys, hst'⟩ := hgen hst
      exact C.moveFrom (Or.inr ⟨i, rfl⟩) (nextPc_moveFrom hst') hex
    case moveToGeneric i =>
      obtain ⟨x, tys, hst'⟩ := hgen hst
      exact C.moveTo (Or.inr ⟨i, rfl⟩) (nextPc_moveTo hst') hex
    case unpack d | unpackVariant d | vecUnpack d n | unpackGeneric d | unpackVariantGeneric d =>
      obtain ⟨hnp, hv⟩ : NextPc s' fr rest ∧ ∃ v st, fr.stack = v :: st ∧ v.type.isValue = true := by
        first
        | exact nextPc_unpack (show stepOf (Step.unpackStruct P s fr rest d) = _ from hst)
        | exact nextPc_unpackVariant (show stepOf (Step.unpackEnum P s fr rest info d) = _ from hst)
        | exact nextPc_vecUnpack (typeArgsOk_values hact.typeArgs) hst
        | (obtain ⟨x, tys, hst'⟩ := hgen hst; exact nextPc_unpack hst')
        | (obtain ⟨x, tys, hst'⟩ := hgen hst; exact nextPc_unpackVariant hst')
      refine C.unpackLike hnp rfl rfl hv ?_
      simp only [Shadow.exec, Option.pure_def] at hex
      split at hex
      · next t st hgs =>
        obtain ⟨v, -, hex⟩ := Option.bind_eq_some_iff.1 hex
        simp only [Option.some.injEq, Prod.mk.injEq, ShadowPost.running.injEq] at hex
        obtain ⟨rfl, -⟩ := hex
        exact ⟨t, st, hgs, rfl⟩
      · cases hex
    case writeRef => exact C.writeRef hst hex
    case vecPushBack sig => exact C.vecPushBack hst hex
    case vecPopBack sig => exact C.vecPopBack hst hex
    case vecSwap sig => exact C.vecSwap hst hex
    case call hd => exact C.call (Or.inl ⟨hd, rfl⟩) hst hex
    case callGeneric i =>
      obtain ⟨x, tys, hst'⟩ := hgen hst
      exact C.call (Or.inr ⟨i, rfl⟩) hst' hex
    case ret => exact C.ret hst hex
    case callClosure sig => exact C.callClosure hst hex
    all_goals first
      | (simp [Shadow.exec] at hex; done)
      | (simp [Bytecode.PushesNew] at hk; done)


/-! ## Native calls -/

/-- A tree of a list of trees as long as a list of values is paired with a
value. -/
theorem tf_list_pair {ts : List IdTree} {vs : List Value} (hlen : ts.length = vs.length)
    {t : IdTree} (ht : t ∈ ts) : ∃ v, (t, v) ∈ ts.zip vs := by
  obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 ht
  have hkl : k < vs.length := by
    have := (List.getElem?_eq_some_iff.1 hk).1
    omega
  exact ⟨_, zip_of_getElem? hk (List.getElem?_eq_getElem hkl)⟩

/-- **The trees after a native call come from before it**: the results' are
new or of values, and the caller's locals were available at the call. -/
private theorem native_origins {P : Program} {ν : NativeFlows} {s s₁ s' : State} {h h' : Shadow}
    {E evs : List Event} {f : FunctionRef} {args views vals writes : List Value}
    {places : List NativePlace}
    (hc : Certified P (modelTypes P) (modelAvail P)) (hI : RefTraceInv P s h E)
    (hinv : Inv P (modelTypes P) (modelAvail P) s) (hcalls : AtCalls P s)
    (hstep : step P s = .callNative s₁ f args views) (hshape : P.nativeShape? f = none)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s')
    (hn : Shadow.nativeStep P ν s f args views vals writes places (.running s') h =
      some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨fr, rest, g, hs, ts, n, hsf, hh, hs'f, hh'f, -, -, hfitR, -, hres⟩ :=
    native_results hI hstep hshape ho hn
  obtain ⟨fr₁, rest₁, info, code, instr, hsf₁, hinfo, hcode, hi, hcall⟩ := tf_native_instr hstep
  rw [hsf, List.cons.injEq] at hsf₁
  obtain ⟨rfl, rfl⟩ := hsf₁
  obtain ⟨fr₀, rest₀, hsf₀, ⟨info₀, code₀, hact⟩, -⟩ := hinv
  rw [hsf, List.cons.injEq] at hsf₀
  obtain ⟨rfl, rfl⟩ := hsf₀
  have hi₀ : info = info₀ := by rw [hact.fn, Option.some.injEq] at hinfo; exact hinfo.symm
  subst hi₀
  have hc₀ : code = code₀ := by rw [hact.body, Option.some.injEq] at hcode; exact hcode.symm
  subst hc₀
  have hz0 : (g, fr) ∈ h.frames.zip s.frames := by rw [hh, hsf]; exact List.mem_cons_self ..
  refine ⟨fun g₂ fr₂ hz => ?_, fun f' hf => ?_⟩
  · rw [hh'f, hs'f] at hz
    simp only [List.zip_cons_cons, List.mem_cons, Prod.mk.injEq] at hz
    rcases hz with ⟨rfl, rfl⟩ | hz
    · refine ⟨fun t ht => ?_, fun j t ht hav => ?_⟩
      · rcases List.mem_append.1 ht with ht | ht
        · obtain ⟨v, hv⟩ := tf_list_pair (fitsAll_length hfitR) (List.mem_reverse.1 ht)
          rcases hres _ (occ_of_zip hv _ (tf_root_occ t v)) with ⟨hlo, -⟩ | ⟨w, hw, hwr⟩
          · exact Or.inr (Or.inr (Or.inl hlo))
          · exact Or.inr (Or.inr (Or.inr ⟨w, hw, hwr⟩))
        · exact Or.inl ⟨g, fr, hz0, List.mem_of_mem_drop ht⟩
      · exact Or.inr (Or.inl ⟨g, fr, j, hz0, ht,
          tf_avail_call hc hact.fn hact.body hact.reachable hi (Or.inl hcall) hav⟩)
    · have hz' : (g₂, fr₂) ∈ h.frames.zip s.frames := by
        rw [hh, hsf]; exact List.mem_cons_of_mem _ hz
      exact ⟨fun t ht => Or.inl ⟨_, _, hz', ht⟩,
        fun j t ht hav => Or.inr (Or.inl ⟨_, _, j, hz', ht, hav⟩)⟩
  · rw [hs'f] at hf
    exact hcalls f' (by rw [hsf]; exact hf)


/-- **The trees after a call of a modeled native come from before it**: the
results' and the values written back have new identities or those of nodes it
saw, which are no references; a local it writes keeps its identity, of a value
that is no reference; the caller's other locals were available at the call. -/
private theorem origins_modeled {P : Program} {ν : NativeFlows} {s s₁ s' : State}
    {h h' : Shadow} {E evs : List Event} {f : FunctionRef} {args views vals writes : List Value}
    {places : List NativePlace} {k : NativeShape}
    (hc : Certified P (modelTypes P) (modelAvail P)) (hI : RefTraceInv P s h E)
    (hinv : Inv P (modelTypes P) (modelAvail P) s) (hcalls : AtCalls P s)
    (hshape : P.nativeShape? f = some k)
    (hstep : step P s = .callNative s₁ f args views)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s')
    (hn : Shadow.nativeStep P ν s f args views vals writes places (.running s') h =
      some (.running h', evs)) :
    Origins P s s' h h' ∧ AtCalls P s' := by
  obtain ⟨fr, rest, n, g, hs, F, results, resultTrees, ts, hsf, hh, hnl, hng, hargs, hwb, hfold,
    hfitR, hfitT, hwr, -, -, -, -, -, F₀, hF₀, -, hge, horigT, horigR⟩ :=
    nativeStep_decomp hshape hstep ho hn
  obtain ⟨fr₁, rest₁, info, code, instr, hsf₁, hinfo, hcode, hi, hcall⟩ := tf_native_instr hstep
  rw [hsf, List.cons.injEq] at hsf₁
  obtain ⟨rfl, rfl⟩ := hsf₁
  obtain ⟨fr₀, rest₀, hsf₀, ⟨info₀, code₀, hact⟩, -⟩ := hinv
  rw [hsf, List.cons.injEq] at hsf₀
  obtain ⟨rfl, rfl⟩ := hsf₀
  have hi₀ : info = info₀ := by rw [hact.fn, Option.some.injEq] at hinfo; exact hinfo.symm
  subst hi₀
  have hc₀ : code = code₀ := by rw [hact.body, Option.some.injEq] at hcode; exact hcode.symm
  subst hc₀
  have hfit := hI.pre.fits
  rw [shadow_fits, hh, hsf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
  obtain ⟨⟨hg, hrest⟩, hext⟩ := hfit
  have hfa := frame_stack_fits hg
  obtain ⟨-, hdropf⟩ := fitsAll_take_drop n hfa
  have hlR := fitsAll_length hfitR
  have hfit₁ : Shadow.fits
      { h with
        frames := { g with stack := resultTrees.reverse ++ g.stack.drop n } :: hs
        fresh := F }
      { s with
        frames := { fr with
          stack := results.reverse ++ fr.stack.drop n
          pc := fr.pc + 1 } :: rest } = true := by
    rw [fits_head (h' := { h with
        frames := { g with stack := resultTrees.reverse ++ g.stack.drop n } :: hs
        fresh := F })
      (s' := { s with
        frames := { fr with
          stack := results.reverse ++ fr.stack.drop n
          pc := fr.pc + 1 } :: rest }) rfl rfl h s rfl rfl rfl rfl]
    simp only [Bool.and_eq_true]
    refine ⟨⟨frame_fits_stack hg ?_ _, hrest⟩, hext⟩
    rw [fitsAll_append (by simp [hlR]), fitsAll_reverse hfitR, hdropf]
    rfl
  have hids : ((fr :: rest).map (·.id)).Nodup := by rw [← hsf]; exact hI.pre.inv.frameIds
  have hslots := writes_slots (s₀ := s) _ ts writes hfit₁ (by simpa using hids)
    hI.pre.inv.resources (by simp [hsf]) rfl
    (writesAgree_native hstep (by rw [hargs]; simp; omega)
      (root_eq (by simp [hsf]) rfl rfl)) hwb hfold hfitT
    (fun w hw => hwr w hw w (self_mem_subvalues w))
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  -- Identities of the outputs' nodes.
  have hO : ∀ i : Nat, ((h.fresh ≤ i ∧ i < F₀) ∨ ∃ seen kk q,
      seenTrees h s args (g.stack.take n).reverse = some seen ∧ seenIdAt seen kk q = some i) →
      h.fresh ≤ i ∨ ∃ w, (i, w) ∈ h.occurrences s ∧ w.isRef = false := by
    intro i hi
    rcases hi with ⟨h1, -⟩ | ⟨seen, kk, q, hseen, hq⟩
    · exact Or.inl h1
    · rw [← hargl] at hseen
      exact Or.inr (seen_nodes hI.pre hI.typed hshape hstep hh hseen kk q i hq)
  have hTs : ∀ y ∈ ts, h.fresh ≤ y.id ∨ ∃ w, (y.id, w) ∈ h.occurrences s ∧ w.isRef = false := by
    intro y hy
    obtain ⟨w, hyw⟩ := tf_list_pair (fitsAll_length hfitT) hy
    exact hO y.id (horigT _ (occ_of_zip hyw _ (tf_root_occ y w)))
  have hz0 : (g, fr) ∈ h.frames.zip s.frames := by rw [hh, hsf]; exact List.mem_cons_self ..
  -- A local after the call, as `SlotAfter` describes it.
  have hslotO : ∀ (gA : ShadowFrame) (frA : Frame) (gB : ShadowFrame) (frB : Frame) (j : Nat)
      (t : IdTree), (gA, frA) ∈ h.frames.zip s.frames → gA.fits frA = true →
      SlotAfter ts gA gB frA frB j → gB.locals[j]? = some (some t) →
      (gA.locals[j]? = some (some t) → Origin P s h t) → Origin P s h t := by
    intro gA frA gB frB j t hzA hgA hsl ht hun
    rcases hsl with ⟨a1, -⟩ | ⟨tB, vB, hgB, -, -, hor⟩
    · exact hun (by rw [← a1]; exact ht)
    · rw [hgB, Option.some.injEq, Option.some.injEq] at ht
      subst ht
      rcases hor with ⟨tA, vA, hgA', hfA, hvA, hid⟩ | ⟨y, hy, hid⟩
      · obtain ⟨v, hv, hocc⟩ := tf_local_pair hgA hgA'
        rw [hfA, Option.some.injEq, Option.some.injEq] at hv
        subst hv
        exact Or.inr (Or.inr (Or.inr ⟨vA, by rw [hid]; exact tf_pair_occ hzA hocc, hvA⟩))
      · rcases hTs y hy with h1 | ⟨w, hw, hwr'⟩
        · exact Or.inr (Or.inr (Or.inl (by rw [hid]; exact h1)))
        · exact Or.inr (Or.inr (Or.inr ⟨w, by rw [hid]; exact hw, hwr'⟩))
  obtain ⟨hfit', -, hids', -, -, -, -⟩ := writes_ghost (s₀ := s) _ ts writes hfit₁
    (by simpa using hids) hI.pre.inv.resources (by simp [hsf]) rfl
    (writesAgree_native hstep (by rw [hargs]; simp; omega)
      (root_eq (by simp [hsf]) rfl rfl)) hwb hfold hfitT
  have hl1 : s'.frames.length = rest.length + 1 := by
    rw [← List.length_map (f := (·.id)), hids']; simp
  have hpre : ∀ kk, kk + 1 < s'.frames.length → ∃ (frK : Frame) (gK : ShadowFrame),
      rest[kk]? = some frK ∧ hs[kk]? = some gK ∧ (gK, frK) ∈ h.frames.zip s.frames ∧
      gK.fits frK = true := by
    intro kk hk
    have hkr : kk < rest.length := by omega
    have hlh : hs.length = rest.length := by
      have := shadow_frames_length hfit₁
      simp at this
      exact this
    have hfrK := List.getElem?_eq_getElem hkr
    have hgK := List.getElem?_eq_getElem (show kk < hs.length by omega)
    have hzK : (hs[kk], rest[kk]) ∈ h.frames.zip s.frames := by
      rw [hh, hsf]
      exact zip_of_getElem? (k := kk + 1) (by simp) (by simp)
    exact ⟨_, _, hfrK, hgK, hzK, tf_pair_fits hI.pre.fits hzK⟩
  refine ⟨fun gB frB hz => ?_, fun f' hf => ?_⟩
  · obtain ⟨kk, hkk⟩ := List.mem_iff_getElem?.1 hz
    rw [List.getElem?_zip_eq_some] at hkk
    obtain ⟨h1, h2⟩ := hkk
    cases kk with
    | zero =>
      obtain ⟨e1, -, e3, e4, hsl⟩ := hslots 0 gB frB _ _ h1 h2 rfl rfl
      refine ⟨fun t ht => ?_, fun j t ht hav => ?_⟩
      · rw [e1] at ht
        rcases List.mem_append.1 ht with ht | ht
        · have ht' := List.mem_reverse.1 ht
          obtain ⟨v, htv⟩ := tf_list_pair hlR ht'
          rcases horigR with horigR | ⟨d, b, t', rfl, rfl, hb⟩
          · rcases hO t.id (horigR _ (occ_of_zip htv _ (tf_root_occ t v))) with h1 | h1
            · exact Or.inr (Or.inr (Or.inl h1))
            · exact Or.inr (Or.inr (Or.inr h1))
          · simp only [List.mem_singleton] at ht'
            subst ht'
            have := hge _ _ _ hb
            exact Or.inr (Or.inr (Or.inl (by simp only [IdTree.id]; omega)))
        · exact Or.inl ⟨g, fr, hz0, List.mem_of_mem_drop ht⟩
      · refine hslotO g fr gB frB j t hz0 hg (hsl j) ht fun hgt => ?_
        rw [e3, e4] at hav
        exact Or.inr (Or.inl ⟨g, fr, j, hz0, hgt,
          tf_avail_call hc hact.fn hact.body hact.reachable hi (Or.inl hcall) hav⟩)
    | succ kk =>
      obtain ⟨frK, gK, hfrK, hgK, hzK, hgK'⟩ := hpre kk (List.getElem?_eq_some_iff.1 h2).1
      obtain ⟨e1, -, e3, e4, hsl⟩ := hslots (kk + 1) gB frB gK frK h1 h2 (by simpa using hgK)
        (by simpa using hfrK)
      refine ⟨fun t ht => Or.inl ⟨gK, frK, hzK, by rw [← e1]; exact ht⟩, fun j t ht hav => ?_⟩
      refine hslotO gK frK gB frB j t hzK hgK' (hsl j) ht fun hgt => ?_
      rw [e3, e4] at hav
      exact Or.inr (Or.inl ⟨gK, frK, j, hzK, hgt, hav⟩)
  · obtain ⟨kk, hkk⟩ := List.mem_iff_getElem?.1 hf
    rw [List.getElem?_tail] at hkk
    obtain ⟨frK, gK, hfrK, hgK, -, -⟩ := hpre kk (List.getElem?_eq_some_iff.1 hkk).1
    obtain ⟨gB, hgB⟩ : ∃ gB, h'.frames[kk + 1]? = some gB :=
      ⟨_, List.getElem?_eq_getElem (by
        rw [shadow_frames_length hfit']; exact (List.getElem?_eq_some_iff.1 hkk).1)⟩
    obtain ⟨-, -, e3, e4, -⟩ := hslots (kk + 1) gB f' gK frK hgB hkk (by simpa using hgK)
      (by simpa using hfrK)
    obtain ⟨info', code', instr', hf', hcode', hreach', hi', hk'⟩ :=
      hcalls frK (by rw [hsf]; exact List.mem_of_getElem? hfrK)
    exact ⟨info', code', instr', by rw [e3]; exact hf', hcode', by rw [e4]; exact hreach',
      by rw [e4]; exact hi', hk'⟩

/-! ## Along the traces of a program with the type invariant -/

section Typed

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
  (hc : Certified P (modelTypes P) (modelAvail P))
  (hinvR : ∀ s, Reachable P N s₀ s → Inv P (modelTypes P) (modelAvail P) s)
  (h0 : AdmissibleInitial P s₀)

include hc hinvR h0 in
/-- One traced step: the trees after it come from before it, and the
suspended frames stay at calls. -/
private theorem tf_origins_step {s s' : State} {h h' : Shadow} {E evs : List Event}
    {o : Outcome} {post : ShadowPost} (hT : Trace P N ν s₀ s h E) (hcalls : AtCalls P s)
    (hS : TracedStep P N ν s h o post evs) (ho : o = .running s') (hp : post = .running h') :
    Origins P s s' h h' ∧ AtCalls P s' := by
  have hI := refTraceInv_trace h0 hT
  have hinv := hinvR s (trace_reachable hT)
  subst hp
  cases hS with
  | core hstep hcs =>
    subst ho
    exact core_origins hc hI hinv hcalls hstep hcs
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      rw [ho] at hn
      exact origins_modeled hc hI hinv hcalls hshape hstep ho hn
    rw [ho] at hn
    exact native_origins hc hI hinv hcalls hstep hshape ho hn

include hc hinvR h0 in
/-- **The suspended frames are at calls** along every trace. -/
private theorem atCalls_trace {s : State} {h : Shadow} {E : List Event}
    (hT : Trace P N ν s₀ s h E) : AtCalls P s := by
  induction hT with
  | start =>
    obtain ⟨f, args, tys, storage, held, -, -, -, hinit⟩ := h0
    obtain ⟨fr, -, hfr, -⟩ := initialState_shape hinit
    intro g hg
    rw [hfr] at hg
    cases hg
  | step hT₀ hS ih => exact (tf_origins_step hc hinvR h0 hT₀ ih hS rfl rfl).2

include hc hinvR h0 in
/-- **The verifier's tracked references are held** in the sense of
`HeldFacts`, along the executions from an admissible initial state of a
certified program whose reachable states keep the type and availability
invariant. -/
theorem heldFacts_tracked_of : HeldFacts P N ν s₀ (Tracked P) where
  occ _ _ _ hT _ _ ht := tracked_occ (refTraceInv_trace h0 hT).pre.fits ht
  used s h E hT o post evs hS i r hu := by
    have hI := refTraceInv_trace h0 hT
    cases hS with
    | core hstep hcs =>
      obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hi, hex⟩ := coreStep_exec hcs
      obtain ⟨hframes, hst, hhe, fresh⟩ := h
      cases hframes with
      | nil =>
        have hfit := hI.pre.fits
        rw [shadow_fits, hsf] at hfit
        simp [framesFit] at hfit
      | cons g hs' =>
        have hz : (g, fr) ∈ (g :: hs').zip s.frames := by rw [hsf]; exact List.mem_cons_self ..
        have hg := tf_pair_fits hI.pre.fits hz
        rcases exec_used rfl hg hex i r hu with hsl | ⟨j, hj, hl⟩
        · exact ⟨g, fr, hz, Or.inl hsl⟩
        · refine ⟨g, fr, hz, Or.inr ⟨j, hl, ?_⟩⟩
          have hinv := hinvR s (trace_reachable hT)
          obtain ⟨fr₀, rest₀, hsf₀, ⟨info₀, code₀, hact⟩, -⟩ := hinv
          rw [hsf, List.cons.injEq] at hsf₀
          obtain ⟨rfl, rfl⟩ := hsf₀
          have hi₀ : info = info₀ := by
            rw [hact.fn, Option.some.injEq] at hinfo; exact hinfo.symm
          subst hi₀
          have hc₀ : code = code₀ := by
            rw [hact.body, Option.some.injEq] at hcode; exact hcode.symm
          subst hc₀
          obtain ⟨-, -, -, hcert⟩ := hc.availability fr.function info code hinfo hcode
          exact (hcert fr.pc instr hact.reachable hi).1 j (by rcases hj with rfl | rfl <;> rfl)
    | native hstep _ hn =>
      rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
      case inr =>
        obtain ⟨g, hs', hh, -, hx⟩ := nativeStep_used hn hu
        obtain ⟨t, rfl, hz⟩ := mem_refArgsOf hx
        obtain ⟨fr, rest, n, hsf, hnl, hargs, -, -⟩ := step_callNative hstep
        have hz0 : (g, fr) ∈ h.frames.zip s.frames := by
          rw [hh, hsf]; exact List.mem_cons_self ..
        have hg := tf_pair_fits hI.pre.fits hz0
        subst hargs
        have hargl : (fr.stack.take n).reverse.length = n := by simp [Nat.min_eq_left hnl]
        rw [hargl] at hz
        exact ⟨g, fr, hz0, Or.inl ⟨t, rfl, zip_rev_take (fitsAll_length (frame_stack_fits hg)) hz⟩⟩
      obtain ⟨rfl, rfl, rfl, g, hs', info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
      rw [hevs] at hu
      exact absurd hu nativeEvents_used
  back s h E hT s' h' evs hS i r ht := by
    have hcalls := atCalls_trace hc hinvR h0 hT
    have hO := (tf_origins_step hc hinvR h0 hT hcalls hS rfl rfl).1
    exact back_of_origins (refTraceInv_trace h0 hT) (refTraceInv_trace h0 (.step hT hS))
      (idsKind_trace h0 hT) hO ht

end Typed

/-! ## Along the traces of a verified program -/

section Verified

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
  (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.SupportedNonGeneric) (hm3 : P.M3)
  (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀)

include hwf hs hm3 hN h0 in
/-- **The verifier's tracked references are held** in the sense of
`HeldFacts`, along the executions of an M3 program from an admissible
initial state. -/
theorem heldFacts_tracked : HeldFacts P N ν s₀ (Tracked P) :=
  heldFacts_tracked_of (certified_of_wellFormed schedulerSound hwf hs)
    (fun s hr => (invM3_reachable hwf hs hm3 (certified_of_wellFormed schedulerSound hwf hs) hN h0
      s hr).1) h0

end Verified

end MoveBytecode.Proofs
