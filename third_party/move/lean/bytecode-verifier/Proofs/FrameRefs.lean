-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefTrace

/-!
# The references of each frame across a step

UNTRUSTED (checked by the kernel). Across a traced step that keeps the number
of frames, every reference in a frame was in that frame before, or, in the
current frame, the step borrowed it (`traced_frameRefsAt`). The references of
the frames as a whole (`traced_frameRefs`) do not say which frame a reference
came from, and D4 follows references frame by frame: a frame returns only
references derived from its own parameters.
-/

namespace MoveBytecode.Proofs

/-- A state and a shadow that fits it have as many frames. -/
theorem frames_length {h : Shadow} {s : State} (hfit : h.fits s = true) :
    h.frames.length = s.frames.length := by
  rw [shadow_fits, Bool.and_eq_true] at hfit
  simp only [framesFit, Bool.and_eq_true, beq_iff_eq] at hfit
  exact hfit.1.1

/-- Per frame, a native call: it changes only the current frame's operand
stack, to values without references. -/
theorem native_frameRefsAt {P : Program} {ν : NativeFlows} {s s₁ s' : State} {h h' : Shadow}
    {evs evs' : List Event} {f : FunctionRef} {args views vals writes : List Value}
    {places : List NativePlace}
    (hI : RefTraceInv P s h evs) (hstep : step P s = .callNative s₁ f args views)
    (hshape : P.nativeShape? f = none)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s')
    (hn : Shadow.nativeStep P ν s f args views vals writes places (.running s') h =
      some (.running h', evs'))
    {k : Nat} {g g' : ShadowFrame} {fr fr' : Frame}
    (hg : h.frames[k]? = some g) (hfr : s.frames[k]? = some fr)
    (hg' : h'.frames[k]? = some g') (hfr' : s'.frames[k]? = some fr')
    {d : Nat} {r : Reference} (hm : (d, Value.ref r) ∈ g'.occurrences fr') :
    (d, Value.ref r) ∈ g.occurrences fr := by
  have hcont : (resumeNative P s₁ f args (.returned vals writes places)).continues = true := by
    rw [ho]; rfl
  have hvals := native_values hstep hshape hcont
  rw [← ho] at hn
  obtain ⟨rfl, rfl, rfl, g₀, hs, info, hh, -, -, hpost, -, -⟩ := nativeStep_some hstep hshape hn
  obtain ⟨fr₀, rest, n, hsf, hnl, hargs, rfl, hfacts⟩ := step_callNative hstep
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  have hv := hfacts.values hshape
  have hs' := (resume_running (fr := { fr₀ with stack := fr₀.stack.drop n }) (rest := rest) rfl
    hshape (noRef_of_conformsAll hconfa (hv.1 _ hpt)) hv.2 hcont).1
  rw [ho, Outcome.running.injEq] at hs'
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  have hg₀ : g₀.fits fr₀ = true := by
    have := hI.pre.fits
    rw [shadow_fits, hsf, hh, framesFit_cons] at this
    simp only [Bool.and_eq_true] at this
    exact this.1.1
  have hfa := frame_stack_fits hg₀
  obtain ⟨hfR, -, -⟩ := nativeResults_spec ν f args vals (g₀.stack.take args.length).reverse
    info.moduleId h.fresh
  simp only [nativePost, ShadowPost.running.injEq] at hpost
  generalize nativeResults ν f args vals (g₀.stack.take args.length).reverse info.moduleId h.fresh = R
    at hfR hpost
  have hh' : h' = { h with
      frames := ({ g₀ with stack := R.1.reverse ++ g₀.stack.drop args.length } :: hs)
      fresh := R.2.1 } := hpost
  rw [hh'] at hg'
  rw [hs'] at hfr'
  rw [hh] at hg
  rw [hsf] at hfr
  cases k with
  | zero =>
    simp only [List.getElem?_cons_zero, Option.some.injEq] at hg hfr hg' hfr'
    subst hg hfr hg' hfr'
    have hlenR : R.1.reverse.length = vals.reverse.length := by
      simp [fitsAll_length hfR]
    rw [hargl, frame_occurrences, localsOcc_stack, occurrencesAll_append hlenR] at hm
    rw [frame_occurrences, stack_split hfa n]
    simp only [List.mem_append] at hm ⊢
    rcases hm with hm | hm | hm
    · exact Or.inl hm
    · obtain ⟨v, hv, hsub⟩ := occurrencesAll_mem (fitsAll_reverse hfR) hm
      have := shaped_noRef v (hvals v (List.mem_reverse.1 hv)).1
        (hvals v (List.mem_reverse.1 hv)).2 _ hsub
      simp [Value.isRef] at this
    · exact Or.inr (Or.inr hm)
  | succ k =>
    simp only [List.getElem?_cons_succ] at hg hfr hg' hfr'
    rw [hg, Option.some.injEq] at hg'
    rw [hfr, Option.some.injEq] at hfr'
    subst hg' hfr'
    exact hm

/-- **Per frame, the references after a traced step that keeps the frames**
were in that frame before, or, in the current frame, the step borrowed them. -/
theorem traced_frameRefsAt {P : Program} {N : NativeRel} {ν : NativeFlows} {s s' : State}
    {h h' : Shadow} {o : Outcome} {post : ShadowPost} {evs evs' : List Event}
    (hI : RefTraceInv P s h evs) (hT : TracedStep P N ν s h o post evs') (ho : o = .running s')
    (hp : post = .running h') (hlen : s'.frames.length = s.frames.length)
    {k : Nat} {g g' : ShadowFrame} {fr fr' : Frame}
    (hg : h.frames[k]? = some g) (hfr : s.frames[k]? = some fr)
    (hg' : h'.frames[k]? = some g') (hfr' : s'.frames[k]? = some fr')
    {d : Nat} {r : Reference} (hm : (d, Value.ref r) ∈ g'.occurrences fr') :
    (d, Value.ref r) ∈ g.occurrences fr ∨ (k = 0 ∧ ∃ par, Event.borrowed d r par ∈ evs') := by
  subst hp
  cases hT with
  | core hstep hc =>
    subst ho
    obtain ⟨post, evs'', hc', hG⟩ := coreStep_annot (σ := false) (π := {}) (A := [])
      (auth := []) hI.pre hstep (by simp [Outcome.continues]) (fun vs st ho => by cases ho)
    rw [hc, Option.some.injEq, Prod.mk.injEq] at hc'
    obtain ⟨rfl, rfl⟩ := hc'
    have hlen' : (postPairs (.running h') (.running s')).length =
        (h.frames.zip s.frames).length := by
      have h1 := frames_length (h := h') (s := s') (by simpa [ShadowPost.fits] using hG.fits)
      have h2 := frames_length hI.pre.fits
      simp only [postPairs, List.length_zip]
      omega
    exact hG.refsAt hlen' k g' fr' g fr
      (by simp only [postPairs]; exact List.getElem?_zip_eq_some.2 ⟨hg', hfr'⟩)
      (List.getElem?_zip_eq_some.2 ⟨hg, hfr⟩) d r hm
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k', hshape⟩
    case inr =>
      rw [ho] at hn
      exact (nativeStep_frameRefsAt hI.pre hshape hstep ho hn).2.2 k g' fr' g fr hg' hfr' hg hfr d
        r hm
    rw [ho] at hn
    exact Or.inl (native_frameRefsAt hI hstep hshape ho hn hg hfr hg' hfr' hm)

end MoveBytecode.Proofs
