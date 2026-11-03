-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefCallSteps
import Proofs.RefCanon
import Proofs.RefTypedGeneric

/-!
# The abstract invariant across calls with references

UNTRUSTED (checked by the kernel). The reference-safety analysis's state
after a `Call` with reference arguments or results is well typed (`Typed`):
the arguments' references are released, and each reference result is a new
reference of the result's mutability.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

/-! ## The graph folds -/

theorem addBorrows_props {ps : List Nat} {g g' : Graph} {c : Nat}
    (h : addBorrows g ps c = .ok g') (hw : g.WF) :
    g'.WF ∧ (g.Back → g'.Back) ∧ (∀ x, g'.mutability x = g.mutability x) ∧ g.Covers g' ∧
      (∀ p ∈ ps, g'.HasEdge p ⟨false, []⟩ c) := by
  have hstep : ∀ (g : Graph) (p : Nat) (g' : Graph), g.WF →
      g.addEdge p ⟨false, []⟩ c = .ok g' →
        g'.WF ∧ g.Covers g' ∧ ∀ x, g'.mutability x = g.mutability x :=
    fun g p g' hw h => Graph.addEdge_step h hw
  obtain ⟨hw', hc', hm'⟩ := Graph.fold_covers hstep ps g g' hw h
  refine ⟨hw', fun hb => Graph.fold_back (fun g p g' hb h => Graph.addEdge_back h hb) ps g g' hb h,
    hm', hc', Graph.fold_adds hstep (fun g p => g.HasEdge p ⟨false, []⟩ c)
      (fun g p g' hw h => ?_) (fun g g' p hc hp => hp.mono hc) ps g g' hw h⟩
  obtain ⟨-, e', he', hle⟩ := Graph.addEdge_covers h hw.sets
  exact ⟨e', he', hle⟩

theorem releaseAll_props : ∀ {ps : List Nat} {g g' : Graph}, releaseAll g ps = .ok g' → g.WF →
    g'.WF ∧ (g.Back → g'.Back) ∧ (∀ x, x ∉ ps → g'.mutability x = g.mutability x) ∧
      (∀ x ∈ ps, g'.find? x = none)
  | [], g, g', h, hw => by
    simp only [releaseAll, List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact ⟨hw, id, fun _ _ => rfl, fun x hx => by simp at hx⟩
  | p :: ps, g, g', h, hw => by
    simp only [releaseAll, List.foldlM_cons, bind, Except.bind] at h
    split at h
    · cases h
    · next g1 hg1 =>
      obtain ⟨hw1, hgone1, hm1, -, -⟩ := Graph.release_ok hg1 hw
      obtain ⟨hw', hb', hm', hgone'⟩ := releaseAll_props (ps := ps) h hw1
      refine ⟨hw', fun hb => hb' (Graph.release_back hg1 hb), fun x hx => ?_, fun x hx => ?_⟩
      · simp only [List.mem_cons, not_or] at hx
        rw [hm' x hx.2, hm1 x hx.1]
      · rcases List.mem_cons.1 hx with rfl | hx
        · by_cases hxs : x ∈ ps
          · exact hgone' x hxs
          · have := hm' x hxs
            simp only [Graph.mutability, hgone1, Option.map_none] at this
            cases h' : g'.find? x with
            | none => rfl
            | some r => rw [h'] at this; cases this
        · exact hgone' x hx

/-- **The results' references**: each reference result is a new reference of
the result type's mutability; nothing else changes but the borrows. -/
theorem callResults_run {all muts : List Nat} :
    ∀ (rs : List SignatureToken) (st st' : RefState) (vals : List AbsValue),
      (rs.mapM (callResult all muts)).run st = .ok (vals, st') → st.graph.WF →
      vals.length = rs.length ∧
      (∀ (k : Nat) (t : SignatureToken), rs[k]? = some t → refKind t = none →
        vals[k]? = some AbsValue.nonReference) ∧
      (∀ (k : Nat) (t : SignatureToken) (m : Bool), rs[k]? = some t → refKind t = some m →
        ∃ x, vals[k]? = some (AbsValue.reference x)) ∧
      (∀ (k x : Nat), vals[k]? = some (AbsValue.reference x) → ∃ t m, rs[k]? = some t ∧
        refKind t = some m ∧
        st'.graph.mutability x = some m ∧ st.graph.find? x = none) ∧
      (vals.filterMap AbsValue.refId?).Nodup ∧
      st'.locals = st.locals ∧ st'.stack = st.stack ∧
      (∀ x, (st.graph.find? x).isSome → st'.graph.mutability x = st.graph.mutability x) ∧
      (∀ x, (st'.graph.find? x).isSome → (st.graph.find? x).isSome ∨ .reference x ∈ vals) ∧
      st'.graph.WF ∧ (st.graph.Back → st'.graph.Back)
  | [], st, st', vals, h, hw => by
    simp only [List.mapM_nil] at h
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    refine ⟨rfl, fun k t h => by simp at h, fun k t m h => by simp at h, fun k x h => by simp at h,
      by simp, rfl, rfl, fun _ _ => rfl, fun x hx => Or.inl hx, hw, id⟩
  | t :: rs, st, st', vals, h, hw => by
    simp only [List.mapM_cons] at h
    rw [run_bind] at h
    obtain ⟨v, st₁, h1, h⟩ := h
    rw [run_bind] at h
    obtain ⟨vs, st₂, h2, h⟩ := h
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    -- The first result.
    have hone : (refKind t = none ∧ v = .nonReference ∧ st₁ = st) ∨
        ∃ m, refKind t = some m ∧ v = .reference st.nextId ∧ st.graph.find? st.nextId = none ∧
          st₁.graph.mutability st.nextId = some m ∧ st₁.locals = st.locals ∧
          st₁.stack = st.stack ∧
          (∀ x, (st.graph.find? x).isSome → st₁.graph.mutability x = st.graph.mutability x) ∧
          (∀ x, (st₁.graph.find? x).isSome → (st.graph.find? x).isSome ∨ x = st.nextId) ∧
          st₁.graph.WF ∧ (st.graph.Back → st₁.graph.Back) := by
      rcases callResult_run h1 with ⟨hk, rfl, rfl⟩ | ⟨m, g1, g2, hk, rfl, hnew, hadd, rfl⟩
      · exact Or.inl ⟨hk, rfl, rfl⟩
      · have hfresh := (Graph.newRef_ok hnew).1
        have hw1 := Graph.newRef_wf hnew hw
        obtain ⟨hw2, hb2, hm2, -, -⟩ := addBorrows_props hadd hw1
        refine Or.inr ⟨m, hk, rfl, hfresh, ?_, rfl, rfl, fun x hx => ?_, fun x hx => ?_, hw2,
          fun hb => hb2 (Graph.newRef_back hnew hb)⟩
        · rw [hm2]; exact Graph.newRef_mutability_self hnew
        · have hxn : x ≠ st.nextId := fun e => by subst e; rw [hfresh] at hx; cases hx
          rw [hm2, Graph.newRef_mutability hnew x hxn]
        · rw [Graph.isSome_of_mutability (hm2 x)] at hx
          by_cases hxn : x = st.nextId
          · exact Or.inr hxn
          · rw [← Graph.isSome_of_mutability (Graph.newRef_mutability hnew x hxn)]
            exact Or.inl hx
    have hw1 : st₁.graph.WF := by
      rcases hone with ⟨-, -, rfl⟩ | ⟨-, -, -, -, -, -, -, -, -, h, -⟩
      · exact hw
      · exact h
    obtain ⟨hlen, hnone, hsome, hmuts, hnd, hloc, hstk, hkeep, hkeys, hw', hb'⟩ :=
      callResults_run rs st₁ st' vs h2 hw1
    rcases hone with ⟨hk, rfl, rfl⟩ | ⟨m, hk, rfl, hfresh, hm1, hloc1, hstk1, hkeep1, hkeys1, -, hb1⟩
    · refine ⟨by simp [hlen], fun k t' ht hk' => ?_, fun k t' m ht hk' => ?_, fun k x hx => ?_,
        by simpa [AbsValue.refId?] using hnd, hloc, hstk, hkeep, fun x hx => ?_, hw', hb'⟩
      · cases k with
        | zero => rfl
        | succ k => exact hnone k t' (by simpa using ht) hk'
      · cases k with
        | zero => simp at ht; subst ht; rw [hk] at hk'; cases hk'
        | succ k => exact hsome k t' m (by simpa using ht) hk'
      · cases k with
        | zero => simp at hx
        | succ k => exact hmuts k x (by simpa using hx)
      · rcases hkeys x hx with h | h
        · exact Or.inl h
        · exact Or.inr (List.mem_cons_of_mem _ h)
    · -- `st.nextId` is a key of `st₁`, so none of the later ones.
      have hkey1 : (st₁.graph.find? st.nextId).isSome := by
        rw [← Graph.isSome_of_mutability (g' := st₁.graph) (g := st₁.graph) rfl]
        simp [Graph.mutability] at hm1
        obtain ⟨r, hr, -⟩ := hm1
        simp [hr]
      have hnotin : .reference st.nextId ∉ vs := by
        intro hin
        obtain ⟨k, hk'⟩ := List.mem_iff_getElem?.1 hin
        obtain ⟨-, -, -, -, -, hf⟩ := hmuts k st.nextId hk'
        rw [hf] at hkey1; cases hkey1
      refine ⟨by simp [hlen], fun k t' ht hk' => ?_, fun k t' m' ht hk' => ?_, fun k x hx => ?_,
        ?_, hloc.trans hloc1, hstk.trans hstk1, fun x hx => ?_, fun x hx => ?_, hw', fun hb => hb' (hb1 hb)⟩
      · cases k with
        | zero => simp at ht; subst ht; rw [hk] at hk'; cases hk'
        | succ k => exact hnone k t' (by simpa using ht) hk'
      · cases k with
        | zero => exact ⟨_, rfl⟩
        | succ k => exact hsome k t' m' (by simpa using ht) hk'
      · cases k with
        | zero =>
          simp only [List.getElem?_cons_zero, Option.some.injEq, AbsValue.reference.injEq] at hx
          subst hx
          exact ⟨t, m, rfl, hk, by rw [hkeep _ hkey1]; exact hm1, hfresh⟩
        | succ k =>
          obtain ⟨t', m', h1', h2', h3', h4'⟩ := hmuts k x (by simpa using hx)
          refine ⟨t', m', by simpa using h1', h2', h3', ?_⟩
          cases hfx : st.graph.find? x with
          | none => rfl
          | some r =>
            have hxs : (st.graph.find? x).isSome := by simp [hfx]
            have := hkeep1 x hxs
            rw [← Graph.isSome_of_mutability this] at hxs
            rw [h4'] at hxs; cases hxs
      · simp only [List.filterMap_cons, AbsValue.refId?, List.nodup_cons]
        refine ⟨fun h => hnotin ?_, hnd⟩
        rw [Typed.mem_filterMap_refId] at h
        exact h
      · have hx1 : (st₁.graph.find? x).isSome := by
          rw [Graph.isSome_of_mutability (hkeep1 x hx)]; exact hx
        rw [hkeep x hx1, hkeep1 x hx]
      · rcases hkeys x hx with h | h
        · rcases hkeys1 x h with h | rfl
          · exact Or.inl h
          · exact Or.inr List.mem_cons_self
        · exact Or.inr (List.mem_cons_of_mem _ h)

namespace Typed

variable {ltys stys : List SignatureToken} {avail : LocalIndex → Availability} {st : RefState}

/-- **Replacing the top operands**: the top `n` are dropped and their
references removed from the graph, and `post` is pushed, whose references
are new ones of their types' mutability; every other reference keeps its
mutability. -/
theorem replaceTop (ht : Typed ltys stys avail st) {st' : RefState} {n : Nat}
    {post : List AbsValue} {ptys : List SignatureToken}
    (hn : n ≤ st.stack.length) (hloc : st'.locals = st.locals)
    (hstk : st'.stack = post ++ st.stack.drop n) (hptys : post.length = ptys.length)
    (hwf : st'.graph.WF) (hback : st'.graph.Back)
    (hkind : ∀ (k : Nat) (t : SignatureToken) (b : Bool), ptys[k]? = some t →
      refKind t = some b → ∃ x, post[k]? = some (AbsValue.reference x))
    (hmutP : ∀ (k x : Nat), post[k]? = some (AbsValue.reference x) →
      ∃ t b, ptys[k]? = some t ∧ refKind t = some b ∧ st'.graph.mutability x = some b)
    (hfresh : ∀ x, AbsValue.reference x ∈ post → st.graph.find? x = none)
    (hnd' : (post.filterMap AbsValue.refId?).Nodup)
    (hgone : ∀ x, AbsValue.reference x ∈ st.stack.take n → st'.graph.find? x = none)
    (hkeep : ∀ x, (st.graph.find? x).isSome → AbsValue.reference x ∉ st.stack.take n →
      st'.graph.mutability x = st.graph.mutability x)
    (hkeys : ∀ x, (st'.graph.find? x).isSome →
      (st.graph.find? x).isSome ∨ AbsValue.reference x ∈ post) :
    Typed ltys (ptys ++ stys.drop n) avail st' := by
  -- The references held before and after.
  obtain ⟨Lr, hLr⟩ : ∃ l, l = st.locals.filterMap AbsValue.refId? := ⟨_, rfl⟩
  obtain ⟨Ar, hAr⟩ : ∃ l, l = (st.stack.take n).filterMap AbsValue.refId? := ⟨_, rfl⟩
  obtain ⟨Dr, hDr⟩ : ∃ l, l = (st.stack.drop n).filterMap AbsValue.refId? := ⟨_, rfl⟩
  obtain ⟨Pr, hPr⟩ : ∃ l, l = post.filterMap AbsValue.refId? := ⟨_, rfl⟩
  have hheld : heldIds st = Lr ++ Ar ++ Dr := by
    rw [hLr, hAr, hDr, List.append_assoc, ← List.filterMap_append, List.take_append_drop]
    rfl
  have hheld' : heldIds st' = Lr ++ (Pr ++ Dr) := by
    simp only [heldIds, hloc, hstk, List.filterMap_append, hLr, hPr, hDr]
  have hnd0 := ht.distinct
  rw [hheld] at hnd0
  -- Old references kept: keys, not arguments.
  have hkey : ∀ x, x ∈ Lr ∨ x ∈ Dr → (st.graph.find? x).isSome := fun x hx =>
    ht.held_key (by rw [hheld]; rcases hx with h | h <;> simp [h])
  have hnotA : ∀ x, x ∈ Lr ∨ x ∈ Dr → AbsValue.reference x ∉ st.stack.take n := by
    intro x hx hA
    have hxA : x ∈ Ar := hAr ▸ Typed.mem_filterMap_refId.2 hA
    rw [List.nodup_append, List.nodup_append] at hnd0
    obtain ⟨⟨-, -, hLA⟩, -, hLAD⟩ := hnd0
    rcases hx with h | h
    · exact hLA x h x hxA rfl
    · exact hLAD x (List.mem_append_right _ hxA) x h rfl
  have hmutK : ∀ x, x ∈ Lr ∨ x ∈ Dr → st'.graph.mutability x = st.graph.mutability x :=
    fun x hx => hkeep x (hkey x hx) (hnotA x hx)
  have hPfresh : ∀ x, x ∈ Pr → st.graph.find? x = none := fun x hx =>
    hfresh x (Typed.mem_filterMap_refId.1 (hPr ▸ hx))
  have hroot : (st.graph.find? st.locals.length).isSome := by
    have := ht.rootMut
    simp only [Graph.mutability, Option.map_eq_some_iff] at this
    obtain ⟨r, hr, -⟩ := this
    simp [hr]
  have hrootA : AbsValue.reference st.locals.length ∉ st.stack.take n := fun h =>
    ht.rootFree (by rw [hheld]; simp [hAr ▸ Typed.mem_filterMap_refId.2 h])
  have hlenS := ht.nstack
  refine {
    wf := hwf
    back := hback
    rootMut := by rw [hloc, hkeep _ hroot hrootA]; exact ht.rootMut
    nlocals := by rw [hloc]; exact ht.nlocals
    nstack := by
      rw [hstk]; simp only [List.length_append, List.length_drop, hptys]; omega
    keys := fun x hx => ?_
    distinct := ?_
    rootFree := ?_
    locMut := fun i x hi => ?_
    stkMut := fun k x hk => ?_
    stkKind := fun k t b hk hb => ?_
    locKind := by rw [hloc]; exact ht.locKind }
  · rw [hheld', hloc]
    rcases hkeys x hx with h | h
    · rcases ht.keys x h with h | h
      · exact Or.inl h
      · rw [hheld] at h
        simp only [List.mem_append] at h
        rcases h with (h | h) | h
        · exact Or.inr (List.mem_append_left _ h)
        · rw [hgone x (Typed.mem_filterMap_refId.1 (hAr ▸ h))] at hx; cases hx
        · exact Or.inr (List.mem_append_right _ (List.mem_append_right _ h))
    · exact Or.inr (List.mem_append_right _ (List.mem_append_left _
        (hPr ▸ Typed.mem_filterMap_refId.2 h)))
  · rw [hheld']
    rw [List.nodup_append, List.nodup_append] at hnd0
    obtain ⟨⟨hLn, hAn, hLA⟩, hDn, hLAD⟩ := hnd0
    rw [List.nodup_append, List.nodup_append]
    refine ⟨hLn, ⟨hPr ▸ hnd', hDn, fun a ha b hb e => ?_⟩, fun a ha b hb e => ?_⟩
    · subst e
      have h1 := hPfresh a ha
      have h2 := hkey a (Or.inr hb)
      rw [h1] at h2; cases h2
    · subst e
      rcases List.mem_append.1 hb with hb | hb
      · have h1 := hPfresh a hb
        have h2 := hkey a (Or.inl ha)
        rw [h1] at h2; cases h2
      · exact hLAD a (List.mem_append_left _ ha) a hb rfl
  · rw [hheld', hloc]
    intro h
    rcases List.mem_append.1 h with h | h
    · exact ht.rootFree (by rw [hheld]; simp [h])
    · rcases List.mem_append.1 h with h | h
      · rw [hPfresh _ h] at hroot; cases hroot
      · exact ht.rootFree (by rw [hheld]; simp [h])
  · rw [hloc] at hi
    obtain ⟨t, b, h1, h2, h3⟩ := ht.locMut i x hi
    refine ⟨t, b, h1, h2, ?_⟩
    rw [hmutK x (Or.inl (hLr ▸ Typed.mem_filterMap_refId.2 (List.mem_of_getElem? hi)))]
    exact h3
  · rw [hstk] at hk
    by_cases hkp : k < post.length
    · rw [List.getElem?_append_left hkp] at hk
      obtain ⟨t, b, h1, h2, h3⟩ := hmutP k x hk
      exact ⟨t, b, by rw [List.getElem?_append_left (by omega)]; exact h1, h2, h3⟩
    · rw [List.getElem?_append_right (by omega), List.getElem?_drop] at hk
      obtain ⟨t, b, h1, h2, h3⟩ := ht.stkMut _ x hk
      refine ⟨t, b, ?_, h2, ?_⟩
      · rw [List.getElem?_append_right (by omega), List.getElem?_drop, ← hptys]; exact h1
      · have hxD : x ∈ Dr := hDr ▸ Typed.mem_filterMap_refId.2 (by
          rw [List.mem_iff_getElem?]
          exact ⟨k - post.length, by rw [List.getElem?_drop]; exact hk⟩)
        rw [hmutK x (Or.inr hxD)]; exact h3
  · by_cases hkp : k < ptys.length
    · rw [List.getElem?_append_left hkp] at hk
      obtain ⟨x, hx⟩ := hkind k t b hk hb
      exact ⟨x, by rw [hstk, List.getElem?_append_left (by omega)]; exact hx⟩
    · rw [List.getElem?_append_right (by omega), List.getElem?_drop] at hk
      obtain ⟨x, hx⟩ := ht.stkKind _ t b hk hb
      exact ⟨x, by rw [hstk, List.getElem?_append_right (by omega), List.getElem?_drop, hptys]; exact hx⟩

end Typed

/-! ## The initial state with reference parameters -/

theorem Graph.nil_wf : Graph.WF ([] : Graph) :=
  ⟨by simp, fun p pi c s hp _ => by simp [NatMap.find?] at hp,
    fun p pi hp => by simp [NatMap.find?] at hp⟩

theorem Graph.nil_back : Graph.Back ([] : Graph) :=
  fun p pi c s hp _ => by simp [NatMap.find?] at hp

/-- A parameter of reference type. -/
def RefParam (ps : List SignatureToken) (i : Nat) (m : Bool) : Prop :=
  ∃ t, ps[i]? = some t ∧ refKind t = some m

/-- What the pass over the parameters of `RefState.new` has done, up to
parameter `k`. -/
structure NewInv (ps : List SignatureToken) (n k : Nat) (st : RefState) : Prop where
  len : st.locals.length = n
  stack : st.stack = []
  next : st.nextId = n + 1
  locals : ∀ (i x : Nat), st.locals[i]? = some (AbsValue.reference x) →
    x = i ∧ i < k ∧ ∃ m, RefParam ps i m
  params : ∀ i m, i < k → RefParam ps i m → st.locals[i]? = some (AbsValue.reference i)
  keys : ∀ x, (st.graph.find? x).isSome → x < k ∧ ∃ m, RefParam ps x m
  muts : ∀ x m, x < k → RefParam ps x m → st.graph.mutability x = some m
  wf : st.graph.WF
  back : st.graph.Back
  noEdge : ∀ p e c, ¬ st.graph.Edge p e c

theorem newInv_step {ps : List SignatureToken} {n k : Nat} {st st' : RefState}
    {ty : SignatureToken} (hI : NewInv ps n k st) (hk : ps[k]? = some ty) (hkn : k < n)
    (h : (match refKind ty with
      | some mut_ => do
        pure { st with graph := ← st.graph.newRef k mut_, locals := st.locals.set k (.reference k) }
      | none => pure st : VResult RefState) = .ok st') : NewInv ps n (k + 1) st' := by
  cases hkind : refKind ty with
  | none =>
    simp only [hkind, pure, Except.pure, Except.ok.injEq] at h
    subst h
    have hnot : ∀ m, ¬ RefParam ps k m := fun m ⟨t, ht, htm⟩ => by
      rw [hk, Option.some.injEq] at ht; subst ht; rw [hkind] at htm; cases htm
    refine ⟨hI.len, hI.stack, hI.next, fun i x hi => ?_, fun i m hi hr => ?_, fun x hx => ?_,
      fun x m hx hr => ?_, hI.wf, hI.back, hI.noEdge⟩
    · obtain ⟨rfl, h1, h2⟩ := hI.locals i x hi
      exact ⟨rfl, by omega, h2⟩
    · by_cases hik : i = k
      · subst hik; exact absurd hr (hnot m)
      · exact hI.params i m (by omega) hr
    · obtain ⟨h1, h2⟩ := hI.keys x hx
      exact ⟨by omega, h2⟩
    · by_cases hxk : x = k
      · subst hxk; exact absurd hr (hnot m)
      · exact hI.muts x m (by omega) hr
  | some mut_ =>
    simp only [hkind, bind, Except.bind] at h
    split at h
    · cases h
    · next g hg =>
      simp only [pure, Except.pure, Except.ok.injEq] at h
      subst h
      have hrk : RefParam ps k mut_ := ⟨ty, hk, hkind⟩
      have hfresh := (Graph.newRef_ok hg).1
      have hlt : k < st.locals.length := by rw [hI.len]; exact hkn
      refine ⟨by simp [hI.len], hI.stack, hI.next, fun i x hi => ?_, fun i m hi hr => ?_,
        fun x hx => ?_, fun x m hx hr => ?_, Graph.newRef_wf hg hI.wf,
        Graph.newRef_back hg hI.back, fun p e c he => hI.noEdge p e c ((Graph.newRef_edge hg).1 he)⟩
      · simp only at hi
        rw [List.getElem?_set] at hi
        split at hi
        · next hki =>
          subst hki
          simp only [Option.some.injEq, AbsValue.reference.injEq] at hi
          exact ⟨hi.symm, by omega, mut_, hrk⟩
        · obtain ⟨rfl, h1, h2⟩ := hI.locals i x hi
          exact ⟨rfl, by omega, h2⟩
      · simp only
        by_cases hik : i = k
        · subst hik; exact List.getElem?_set_self hlt
        · rw [List.getElem?_set_ne (Ne.symm hik)]
          exact hI.params i m (by omega) hr
      · simp only at hx
        rw [Graph.newRef_find hg] at hx
        split at hx
        · next hxk => subst hxk; exact ⟨by omega, mut_, hrk⟩
        · obtain ⟨h1, h2⟩ := hI.keys x hx
          exact ⟨by omega, h2⟩
      · simp only
        by_cases hxk : x = k
        · subst hxk
          obtain ⟨t, ht, htm⟩ := hr
          rw [hk, Option.some.injEq] at ht; subst ht
          rw [hkind, Option.some.injEq] at htm; subst htm
          exact Graph.newRef_mutability_self hg
        · rw [Graph.newRef_mutability hg x hxk]
          exact hI.muts x m (by omega) hr

/-- **The analysis's initial state**: each reference parameter `i` holds
reference `i` of its mutability, with no borrows; the frame root follows. -/
theorem new_general {view : FunctionView} {st : RefState} (h : RefState.new view = .ok st) :
    ∃ st₁, NewInv view.parameters (view.parameters.length + view.locals.length)
        view.parameters.length st₁ ∧
      ∃ g, st₁.graph.newRef (view.parameters.length + view.locals.length) true = .ok g ∧
        st = { st₁ with graph := g } := by
  unfold RefState.new at h
  simp only [bind_ok] at h
  obtain ⟨st₁, hfold, h⟩ := h
  have hI := foldlM_invariant
    (I := fun st rest => ∃ k, rest = view.parameters.zipIdx.drop k ∧ k ≤ view.parameters.length ∧
      NewInv view.parameters (view.parameters.length + view.locals.length) k st)
    (fun acc x rest acc' hI hf => ?_) _ _ _
    ⟨0, rfl, Nat.zero_le _, ⟨by simp, rfl, rfl, fun i x hi => by simp [List.getElem?_replicate] at hi,
      fun i m hi => by omega, fun x hx => by simp [NatMap.find?] at hx,
      fun x m hx => by omega, Graph.nil_wf, Graph.nil_back,
      fun p e c he => by obtain ⟨pi, -, -, hp, -⟩ := he; simp [NatMap.find?] at hp⟩⟩ hfold
  · obtain ⟨k, hk, hkl, hI⟩ := hI
    have hk' : k = view.parameters.length := by
      have := congrArg List.length hk
      simp at this; omega
    subst hk'
    obtain ⟨g, hg, h⟩ := h
    simp only [pure, Except.pure, Except.ok.injEq] at h
    exact ⟨st₁, hI, g, hg, h.symm⟩
  · obtain ⟨k, hk, hkl, hIk⟩ := hI
    obtain ⟨ty, i⟩ := x
    have hklt : k < view.parameters.length := by
      rcases Nat.lt_or_ge k view.parameters.length with hlt | hge
      · exact hlt
      · have : view.parameters.zipIdx.drop k = [] := by simp; omega
        rw [this] at hk; cases hk
    rw [List.drop_eq_getElem_cons (by simpa using hklt), List.getElem_zipIdx] at hk
    simp only [List.cons.injEq, Prod.mk.injEq, Nat.zero_add] at hk
    obtain ⟨⟨rfl, rfl⟩, rfl⟩ := hk
    refine ⟨i + 1, rfl, hklt, newInv_step hIk (List.getElem?_eq_getElem hklt) (by omega) hf⟩

theorem filterMap_refId_increasing : ∀ (l : List AbsValue) (o : Nat),
    (∀ (i x : Nat), l[i]? = some (AbsValue.reference x) → x = i + o) →
      (l.filterMap AbsValue.refId?).Pairwise (· < ·) ∧ ∀ y ∈ l.filterMap AbsValue.refId?, o ≤ y
  | [], o, _ => by simp
  | v :: l, o, h => by
    have ih := filterMap_refId_increasing l (o + 1) (fun i x hi => by
      have := h (i + 1) x (by simpa using hi); omega)
    cases v with
    | nonReference =>
      simp only [List.filterMap_cons, AbsValue.refId?]
      exact ⟨ih.1, fun y hy => by have := ih.2 y hy; omega⟩
    | reference x =>
      have hx : x = o := by have := h 0 x rfl; omega
      subst hx
      simp only [List.filterMap_cons, AbsValue.refId?, List.pairwise_cons, List.mem_cons]
      exact ⟨⟨fun y hy => by have := ih.2 y hy; omega, ih.1⟩, fun y hy => by
        rcases hy with rfl | hy
        · exact Nat.le_refl _
        · have := ih.2 y hy; omega⟩

/-- **The initial state is well typed**, reference parameters included, when
only parameters are available. -/
theorem typed_new_general {view : FunctionView} {st : RefState}
    {avail : LocalIndex → Availability} (h : RefState.new view = .ok st)
    (havail : ∀ i, avail i = .available → i < view.parameters.length) :
    Typed (view.parameters ++ view.locals) [] avail st := by
  obtain ⟨st₁, hI, g, hg, rfl⟩ := new_general h
  have hfresh := (Graph.newRef_ok hg).1
  have hroot : (view.parameters.length + view.locals.length) = st₁.locals.length := hI.len.symm
  have hheldP : ∀ x, x ∈ heldIds { st₁ with graph := g } →
      x < view.parameters.length ∧ st₁.locals[x]? = some (AbsValue.reference x) := by
    intro x hx
    simp only [heldIds, hI.stack, List.filterMap_nil, List.append_nil] at hx
    obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 (Typed.mem_filterMap_refId.1 hx)
    obtain ⟨rfl, h1, -⟩ := hI.locals i x hi
    exact ⟨h1, hi⟩
  have hmutx : ∀ x, x < view.parameters.length → g.mutability x = st₁.graph.mutability x :=
    fun x hx => Graph.newRef_mutability hg x (by omega)
  refine {
    wf := Graph.newRef_wf hg hI.wf
    back := Graph.newRef_back hg hI.back
    rootMut := by simp only; rw [hI.len]; exact Graph.newRef_mutability_self hg
    nlocals := by simp [hI.len]
    nstack := by simp [hI.stack]
    keys := fun x hx => ?_
    distinct := ?_
    rootFree := fun hr => ?_
    locMut := fun i x hi => ?_
    stkMut := fun k x hk => by simp [hI.stack] at hk
    stkKind := fun k t b hk _ => by simp at hk
    locKind := fun i t b hi hb hav => ?_ }
  · simp only at hx ⊢
    rw [Graph.newRef_find hg] at hx
    split at hx
    · next hxr => exact Or.inl (by rw [hxr, hI.len])
    · obtain ⟨hxk, m, hrp⟩ := hI.keys x hx
      right
      simp only [heldIds, hI.stack, List.filterMap_nil, List.append_nil]
      exact Typed.mem_filterMap_refId.2 (List.mem_of_getElem? (hI.params x m hxk hrp))
  · simp only [heldIds, hI.stack, List.filterMap_nil, List.append_nil]
    exact (filterMap_refId_increasing st₁.locals 0 (fun i x hi => by
      have := (hI.locals i x hi).1; omega)).1.imp Nat.ne_of_lt
  · obtain ⟨h1, -⟩ := hheldP _ hr
    simp only at h1
    rw [hI.len] at h1
    omega
  · simp only at hi
    obtain ⟨rfl, h1, m, t, ht, htm⟩ := hI.locals i x hi
    refine ⟨t, m, by rw [List.getElem?_append_left h1]; exact ht, htm, ?_⟩
    rw [hmutx x h1]
    exact hI.muts x m h1 ⟨t, ht, htm⟩
  · have hlt := havail i hav
    rw [List.getElem?_append_left hlt] at hi
    exact ⟨i, hI.params i b hlt ⟨t, hi, hb⟩⟩

/-! ## `Call` -/

/-- **The state after a call's core is well typed**: the `n` arguments'
references are released, and the results are new references of their types
(`coreCall`, shared by `Call` and `CallClosure`). -/
theorem typed_coreCall {ltys args st1 rs : List SignatureToken}
    {avail : LocalIndex → Availability} {st : RefState} {n : Nat} {all muts : List Nat}
    {vals : List AbsValue} {st₂ : RefState} {g : Graph}
    (ht : Typed ltys (args ++ st1) avail st) (hlen : args.length = n)
    (hn : n ≤ st.stack.length) (hall : ∀ x, x ∈ all ↔ .reference x ∈ st.stack.take n)
    (hmap : (rs.mapM (callResult all muts)).run { st with stack := st.stack.drop n } =
      .ok (vals, st₂))
    (hg : releaseAll st₂.graph all = .ok g) :
    Typed ltys (rs.reverse ++ st1) avail
      { st₂ with graph := g, stack := vals.reverse ++ st₂.stack } := by
  obtain ⟨hvlen, -, hsome, hmuts, hnd, hloc, hstk, hkeep, hkeys, hw2, hb2⟩ :=
    callResults_run rs _ _ _ hmap ht.wf
  obtain ⟨hw3, hb3, hm3, hgone3⟩ := releaseAll_props hg hw2
  -- The arguments are keys of the graph.
  have hallKey : ∀ x ∈ all, (st.graph.find? x).isSome := fun x hx =>
    ht.held_key (mem_heldIds.2 (Or.inr (List.mem_of_mem_take ((hall x).1 hx))))
  have hvalsFresh : ∀ x, AbsValue.reference x ∈ vals → st.graph.find? x = none := by
    intro x hx
    obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hx
    obtain ⟨-, -, -, -, -, h4⟩ := hmuts k x hk
    exact h4
  have hvalsNotAll : ∀ x, AbsValue.reference x ∈ vals → x ∉ all := fun x hx hxa => by
    have := hallKey x hxa; rw [hvalsFresh x hx] at this; cases this
  have hst1 : (args ++ st1).drop n = st1 := by
    rw [← hlen, List.drop_left]
  rw [← hst1]
  refine Typed.replaceTop ht (n := n) (post := vals.reverse) (ptys := rs.reverse) hn
    (by simp only; exact hloc) (by simp only; rw [hstk]) (by simp [hvlen]) hw3
    (hb3 (hb2 ht.back)) (fun k t b hk hb => ?_) (fun k x hk => ?_)
    (fun x hx => hvalsFresh x (List.mem_reverse.1 hx)) ?_ (fun x hx => ?_) (fun x hx hna => ?_)
    (fun x hx => ?_)
  · have hk' : k < rs.length := by
      have := (List.getElem?_eq_some_iff.1 hk).1; simpa using this
    rw [List.getElem?_reverse hk'] at hk
    obtain ⟨x, hx⟩ := hsome _ t b hk hb
    exact ⟨x, by rw [List.getElem?_reverse (by omega), hvlen]; exact hx⟩
  · have hk' : k < vals.length := by
      have := (List.getElem?_eq_some_iff.1 hk).1; simpa using this
    rw [List.getElem?_reverse hk'] at hk
    obtain ⟨t, b, h1, h2, h3, h4⟩ := hmuts _ x hk
    refine ⟨t, b, by rw [List.getElem?_reverse (by omega), ← hvlen]; exact h1, h2, ?_⟩
    rw [hm3 x (hvalsNotAll x (List.mem_of_getElem? hk))]
    exact h3
  · rw [List.filterMap_reverse]
    exact (List.reverse_perm _).nodup_iff.2 hnd
  · exact hgone3 x ((hall x).2 hx)
  · have hxa : x ∉ all := fun h' => hna ((hall x).1 h')
    rw [hm3 x hxa, hkeep x hx]
  · have hxa : x ∉ all := fun h' => by rw [hgone3 x h'] at hx; cases hx
    rw [Graph.isSome_of_mutability (hm3 x hxa)] at hx
    rcases hkeys x hx with h | h
    · exact Or.inl h
    · exact Or.inr (List.mem_reverse.2 h)

/-- **The state after a call is well typed**: the arguments' references are
released, and the results are new references of their types. -/
theorem typed_refCall {m : CompiledModule} {view : FunctionView}
    {ltys stys st1 : List SignatureToken} {avail : LocalIndex → Availability}
    {st st' : RefState} {h : Nat} {fh : FunctionHandle} {ps rs : List SignatureToken}
    (hfh : m.functionHandles[h]? = some fh) (hattr : borrowFieldMutable fh.attributes = none)
    (hps : m.signatures[fh.parameters]? = some ps) (hrs : m.signatures[fh.returns]? = some rs)
    (ht : Typed ltys stys avail st) (hpop : ∃ args, stys = args ++ st1 ∧ args.length = ps.length)
    (hx : (executeInner m view (.call h)).run st = .ok ((), st')) :
    Typed ltys (rs.reverse ++ st1) avail st' := by
  obtain ⟨args, rfl, hlen⟩ := hpop
  obtain ⟨fh', hfh', hcall⟩ := exec_call hx
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  obtain ⟨ps', rs', all, muts, vals, st₂, g, hps', hrs', hn, hall, -, -, -, -, hmap, hg, rfl⟩ :=
    hcall hattr
  rw [hps, Option.some.injEq] at hps'
  subst hps'
  rw [hrs, Option.some.injEq] at hrs'
  subst hrs'
  exact typed_coreCall ht hlen hn hall hmap hg

/-- **The state after a closure call is well typed**: the closure, a value,
is popped, and the call's core follows (`typed_coreCall`). -/
theorem typed_refCallClosure {m : CompiledModule} {view : FunctionView}
    {ltys st1 : List SignatureToken} {avail : LocalIndex → Availability}
    {st st' : RefState} {idx : Nat} {sg : List SignatureToken} {clo : SignatureToken}
    {params results args : List SignatureToken} {a : AbilitySet}
    (hsg : m.signatures[idx]? = some sg) (hhd : sg.head? = some (.function params results a))
    (hclo : refKind clo = none)
    (ht : Typed ltys (clo :: (args ++ st1)) avail st) (hlen : args.length = params.length)
    (hx : (executeInner m view (.callClosure idx)).run st = .ok ((), st')) :
    Typed ltys (results.reverse ++ st1) avail st' := by
  obtain ⟨ps, rs, ab, v, rest, hhead, hs, all, muts, vals, st₂, g, hn, hall, -, -, -, -, hmap, hg,
    rfl⟩ := exec_callClosure hx
  rw [hsg] at hhead
  simp only [Option.bind_some, hhd, Option.some.injEq, SignatureToken.function.injEq] at hhead
  obtain ⟨rfl, rfl, -⟩ := hhead
  have hv : v = .nonReference := ht.nonRef_of_value (k := 0) rfl hclo (by rw [hs]; rfl)
  subst hv
  have ht' := ht.popValue hs rfl
  exact typed_coreCall ht' hlen hn hall hmap hg

/-- **`MutBorrowGlobal`, `ImmBorrowGlobal`**: the analysis pops the address,
checks that the type is not borrowed (mutably, for an immutable borrow), and
pushes a new reference borrowed from the frame root through the type. -/
theorem exec_borrowGlobal {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {d : Nat} {mut_ : Bool}
    (h : (executeInner m view (if mut_ then .mutBorrowGlobal d else .immBorrowGlobal d)).run st =
      .ok ((), st')) :
    ∃ rest g1 g2, st.stack = .nonReference :: rest ∧
      (mut_ = true → st.graph.hasConsistentBorrows st.locals.length (some (.global d)) = .ok false) ∧
      st.graph.hasConsistentMutableBorrows st.locals.length (some (.global d)) = .ok false ∧
      st.graph.newRef st.nextId mut_ = .ok g1 ∧
      g1.addEdge st.locals.length ⟨false, [.global d]⟩ st.nextId = .ok g2 ∧
      st' = { st with stack := .reference st.nextId :: rest, graph := g2, nextId := st.nextId + 1 } := by
  have h' : (do ReferenceSafety.popValue; ReferenceSafety.push (← ReferenceSafety.borrowGlobal mut_ d) :
      RefM Unit).run st = .ok ((), st') := by
    cases mut_ <;> simpa [executeInner] using h
  rw [run_bind] at h'
  obtain ⟨u, st₁, hp, h'⟩ := h'
  obtain ⟨rest, hs, rfl⟩ := popValue_run.1 hp
  rw [run_bind] at h'
  obtain ⟨v, st₂, hb, h'⟩ := h'
  simp only [ReferenceSafety.borrowGlobal, run_bind] at hb
  obtain ⟨b1, st₃, hb1, hb⟩ := hb
  simp only [isGlobalBorrowed] at hb1
  rw [frameRoot_bind_run] at hb1
  obtain ⟨hq1, rfl⟩ := query_ok.1 hb1
  obtain ⟨b2, st₄, hb2, hb⟩ := hb
  simp only [isGlobalMutablyBorrowed] at hb2
  rw [frameRoot_bind_run] at hb2
  obtain ⟨hq2, rfl⟩ := query_ok.1 hb2
  split at hb
  · exact absurd hb reject_bind_run
  · next hc =>
    rw [run_bind] at hb
    obtain ⟨newId, st₅, hn, hb⟩ := hb
    obtain ⟨g1, hg1, rfl, rfl⟩ := newRef_run.1 hn
    rw [run_bind] at hb
    obtain ⟨u2, st₆, hadd, hb⟩ := hb
    simp only [addResourceBorrow, frameRoot_bind_run] at hadd
    obtain ⟨g2, hg2, rfl⟩ := graphOp_ok.1 hadd
    obtain ⟨rfl, rfl⟩ := pure_run.1 hb
    simp only [Bool.or_eq_true, Bool.and_eq_true, not_or, not_and] at hc
    refine ⟨rest, g1, g2, hs, fun hm => ?_, ?_, hg1, hg2, push_run.1 h'⟩
    · have : b1 = false := by simpa using hc.1 hm
      rw [this] at hq1; exact hq1
    · have : b2 = false := by simpa using hc.2
      rw [this] at hq2; exact hq2

/-- The analysis's states stay well typed across a global borrow. -/
theorem typed_refBorrowGlobal {m : CompiledModule} {view : FunctionView}
    {ltys stys stys' : List SignatureToken} {avail : LocalIndex → Availability}
    {st st' : RefState} {d : Nat} {mut_ : Bool} (ht : Typed ltys stys avail st)
    (hty : Model.borrowGlobal m view.handle.typeParameters stys mut_ d = .ok stys')
    (hx : (executeInner m view (if mut_ then .mutBorrowGlobal d else .immBorrowGlobal d)).run st =
      .ok ((), st')) :
    Typed ltys stys' avail st' := by
  unfold Model.borrowGlobal at hty
  simp only [bind_ok, lookup_ok] at hty
  obtain ⟨⟨op, rest_t⟩, hp, hty⟩ := hty
  rw [popType_ok'] at hp
  obtain ⟨u, -, sd, -, a, -, u', -, hty⟩ := hty
  simp only [pure, Except.pure, Except.ok.injEq] at hty
  subst hty
  obtain ⟨rest, g1, g2, hs, -, -, hnew, hadd, rfl⟩ := exec_borrowGlobal hx
  have ht1 := ht.popValue hs hp
  have hnew' : ({ st with stack := rest } : RefState).graph.newRef st.nextId mut_ = .ok g1 := hnew
  obtain ⟨hw2, -, hm2⟩ := Graph.addEdge_step hadd (Graph.newRef_wf hnew ht.wf)
  have hb2 := Graph.addEdge_back hadd (Graph.newRef_back hnew ht.back)
  exact (ht1.pushNew hnew' hw2 hb2 hm2 (refKind_refTo mut_ _)).ext rfl rfl rfl


/-- The signature of a struct-API mutable borrow: one parameter, and one
mutable reference result. -/
def ApiSig (m : CompiledModule) (fh : FunctionHandle) : Prop :=
  ∃ p r, m.signatures[fh.parameters]? = some [p] ∧ m.signatures[fh.returns]? = some [r] ∧
    refKind r = some true

/-- **The state after a struct-API mutable borrow call is well typed**: the
analysis borrows a field of the argument, as `MutBorrowField` does, and the
call's signature types the result as a mutable reference. -/
theorem typed_refCallApi {m : CompiledModule} {view : FunctionView}
    {ltys stys st1 : List SignatureToken} {avail : LocalIndex → Availability}
    {st st' : RefState} {h : Nat} {fh : FunctionHandle} {ps rs : List SignatureToken} {off : Nat}
    (hfh : m.functionHandles[h]? = some fh) (hoff : borrowFieldMutable fh.attributes = some off)
    (hsig : ApiSig m fh) (hps : m.signatures[fh.parameters]? = some ps)
    (hrs : m.signatures[fh.returns]? = some rs)
    (ht : Typed ltys stys avail st) (hpop : ∃ args, stys = args ++ st1 ∧ args.length = ps.length)
    (hx : (executeInner m view (.call h)).run st = .ok ((), st')) :
    Typed ltys (rs.reverse ++ st1) avail st' := by
  obtain ⟨p, r, hps', hrs', hr⟩ := hsig
  rw [hps, Option.some.injEq] at hps'
  subst hps'
  rw [hrs, Option.some.injEq] at hrs'
  subst hrs'
  obtain ⟨args, rfl, hlen⟩ := hpop
  obtain ⟨op, rfl⟩ : ∃ op, args = [op] := by
    match args, hlen with
    | [op], _ => exact ⟨op, rfl⟩
  obtain ⟨id, rest, g1, g2, g3, hs, -, hnew, hfac, hrel, rfl⟩ := exec_callApi hx hfh hoff
  exact ht.borrowFrom hs rfl hnew (newFactor hnew hfac ht.wf ht.back) hrel hr rfl rfl rfl

/-- The restriction on an instruction for the analysis's states to stay well
typed: a call's callee, when a struct-API mutable borrow, has its signature
(`Ret` ends the analysis apart). -/
def M2Instr (m : CompiledModule) : Bytecode → Prop
  | .ret => False
  | .call h => ∀ fh, m.functionHandles[h]? = some fh → ∀ off,
      borrowFieldMutable fh.attributes = some off → ApiSig m fh
  | .callGeneric i => ∀ fi fh, m.functionInstantiations[i]? = some fi →
      m.functionHandles[fi.handle]? = some fh → ∀ off,
      borrowFieldMutable fh.attributes = some off → ApiSig m fh
  | _ => True

namespace Typed

variable {ltys stys : List SignatureToken} {avail : LocalIndex → Availability} {st : RefState}

/-- `Typed` sees the stack's tokens only through their reference kinds. -/
theorem retoken (ht : Typed ltys stys avail st) {stys' : List SignatureToken}
    (h : stys.map refKind = stys'.map refKind) : Typed ltys stys' avail st := by
  have hlen : stys.length = stys'.length := by
    simpa using congrArg List.length h
  have hk : ∀ k : Nat, (stys[k]?).map refKind = (stys'[k]?).map refKind := by
    intro k
    rw [← List.getElem?_map, ← List.getElem?_map, h]
  refine ⟨ht.wf, ht.back, ht.rootMut, ht.nlocals, ht.nstack.trans hlen, ht.keys, ht.distinct,
    ht.rootFree, ht.locMut, fun k x hx => ?_, fun k t b ht' hkb => ?_, ht.locKind⟩
  · obtain ⟨t, b, hkt, hkb, hm⟩ := ht.stkMut k x hx
    have := hk k
    rw [hkt] at this
    cases ht2 : stys'[k]? with
    | none => rw [ht2] at this; simp at this
    | some u =>
      rw [ht2] at this
      simp only [Option.map_some, Option.some.injEq] at this
      exact ⟨u, b, rfl, this ▸ hkb, hm⟩
  · have := hk k
    rw [ht'] at this
    cases ht1 : stys[k]? with
    | none => rw [ht1] at this; simp at this
    | some u =>
      rw [ht1] at this
      simp only [Option.map_some, Option.some.injEq] at this
      exact ht.stkKind k u b ht1 (this.trans hkb)

end Typed

/-- Instantiating by type arguments that are no references keeps every
token's reference kind. -/
theorem mapM_instantiate_kinds {tys : List SignatureToken} (htys : ∀ x ∈ tys, refKind x = none) :
    ∀ {rs rs' : List SignatureToken}, rs.mapM (instantiate tys) = .ok rs' →
      rs'.map refKind = rs.map refKind
  | [], rs', h => by
    simp only [List.mapM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h
    rfl
  | r :: rs, rs', h => by
    simp only [List.mapM_cons, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨q, hq, qs, hqs, rfl⟩ := h
    simp [refKind_instantiate htys hq, mapM_instantiate_kinds htys hqs]

/-- A generic global borrow the model accepts keeps the analysis's state well
typed. -/
theorem typed_refBorrowGlobalGeneric {m : CompiledModule} {view : FunctionView}
    {ltys stys stys' : List SignatureToken} {avail : LocalIndex → Availability}
    {st st' : RefState} {i d : Nat} {tys : List SignatureToken} {mut_ : Bool}
    (ht : Typed ltys stys avail st)
    (hty : borrowGlobalGeneric m view.handle.typeParameters stys mut_ d tys = .ok stys')
    (hx : (executeInner m view
      (if mut_ then .mutBorrowGlobalGeneric i else .immBorrowGlobalGeneric i)).run st =
        .ok ((), st')) :
    Typed ltys stys' avail st' := by
  obtain ⟨rest_t, sd, a, rfl, -, -, -, rfl⟩ := borrowGlobalGen_ok hty
  obtain ⟨si, -, hx'⟩ := exec_borrowGlobalGeneric' hx
  obtain ⟨rest, g1, g2, hs, -, -, hnew, hadd, rfl⟩ := exec_borrowGlobal hx'
  have ht1 := ht.popValue hs rfl
  have hnew' : ({ st with stack := rest } : RefState).graph.newRef st.nextId mut_ = .ok g1 := hnew
  obtain ⟨hw2, -, hm2⟩ := Graph.addEdge_step hadd (Graph.newRef_wf hnew ht.wf)
  have hb2 := Graph.addEdge_back hadd (Graph.newRef_back hnew ht.back)
  exact (ht1.pushNew hnew' hw2 hb2 hm2 (refKind_refTo mut_ _)).ext rfl rfl rfl

/-- **Each instruction keeps the analysis's states well typed** (M3). -/
theorem typed_exec_M2 {P : Program} {f : FunctionRef} {m : CompiledModule} {view : FunctionView}
    {ltys stys stys' : List SignatureToken} {avail : LocalIndex → Availability}
    {st st' : RefState} {instr : Bytecode} {before : List RuntimeType}
    (ha : GenCtx P f m view) (hb : verifyBounds m = .ok ())
    (hltys : ltys = view.parameters ++ view.locals)
    (hsig : RuntimeType.ofSignatureWith m f.typeArgs stys = some before) (hM2 : M2Instr m instr)
    (hops : InstNonRef m instr)
    (ht : Typed ltys stys avail st) (hty : typeInstr m view stys instr = .ok stys')
    (hread : ∀ i, instr.readsLocal = some i → avail i = .available)
    (hx : (executeInner m view instr).run st = .ok ((), st')) :
    Typed ltys stys' (availabilityAfter instr avail) st' := by
  cases instr
  case call h =>
    simp only [typeInstr, typeCall, bind_ok, lookup_ok] at hty
    obtain ⟨fh, hfh, ps, hps, st1, hpop, rs, hrs, hty⟩ := hty
    simp only [pure, Except.pure, Except.ok.injEq] at hty
    subst hty
    obtain ⟨args, hst, hlen, -⟩ := popExpected_ok hpop
    cases hattr : borrowFieldMutable fh.attributes with
    | none => exact typed_refCall hfh hattr hps hrs ht ⟨args, hst, hlen⟩ hx
    | some off =>
      exact typed_refCallApi hfh hattr (hM2 fh hfh off hattr) hps hrs ht ⟨args, hst, hlen⟩ hx
  case callGeneric i =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨fi, hfi, fh, hfh, tys, htys, hty⟩ := hty
    obtain ⟨params, rets, rets', args, rest, hps, hrs, hrets', hst, hlen, -, rfl⟩ :=
      typeCallGeneric_ok hty
    obtain ⟨fi', hfi', hx'⟩ := exec_callGeneric' hx
    rw [hfi, Option.some.injEq] at hfi'
    subst hfi'
    have hnr : ∀ x ∈ tys, refKind x = none :=
      hops fi.typeParameters (by simp [instSig?, hfi]) tys htys
    have hk := mapM_instantiate_kinds hnr hrets'
    have hre : (rets.reverse ++ rest).map refKind = (rets'.reverse ++ rest).map refKind := by
      simp [List.map_append, List.map_reverse, hk]
    cases hattr : borrowFieldMutable fh.attributes with
    | none => exact (typed_refCall hfh hattr hps hrs ht ⟨args, hst, hlen⟩ hx').retoken hre
    | some off =>
      exact (typed_refCallApi hfh hattr (hM2 fi fh hfi hfh off hattr) hps hrs ht
        ⟨args, hst, hlen⟩ hx').retoken hre
  case callClosure idx =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨sg, hsg, hty⟩ := hty
    split at hty
    · next t hhd =>
      obtain ⟨params, results, a, clo, args, rest, rfl, rfl, hassign, hlen, -, rfl⟩ :=
        typeCallClosure_ok hty
      exact typed_refCallClosure hsg hhd (by rw [← refKind_of_assignable hassign]; rfl) ht hlen hx
    · simp [panicAt] at hty
  case mutBorrowGlobal d => exact typed_refBorrowGlobal (mut_ := true) ht hty hx
  case immBorrowGlobal d => exact typed_refBorrowGlobal (mut_ := false) ht hty hx
  case mutBorrowGlobalGeneric i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨d, sd, tys⟩, -, hty⟩ := hty
    exact typed_refBorrowGlobalGeneric (mut_ := true) ht hty hx
  case immBorrowGlobalGeneric i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨d, sd, tys⟩, -, hty⟩ := hty
    exact typed_refBorrowGlobalGeneric (mut_ := false) ht hty hx
  case packGeneric | unpackGeneric | packVariantGeneric | unpackVariantGeneric
    | testVariantGeneric | mutBorrowFieldGeneric | immBorrowFieldGeneric
    | mutBorrowVariantFieldGeneric | immBorrowVariantFieldGeneric | existsGeneric
    | moveFromGeneric | moveToGeneric =>
    exact typed_exec_generic ha hb rfl (fun _ h => by cases h) (fun _ h => by cases h)
      (fun _ h => by cases h) hops ht hty hx
  all_goals exact typed_exec ha hb hltys hsig hM2 ht hty hread hx

end MoveBytecode.Proofs
