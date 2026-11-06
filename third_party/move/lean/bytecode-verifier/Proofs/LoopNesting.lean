-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.StackUsage

/-!
# Loop nesting: every back edge enters through a dominator

UNTRUSTED (checked by the kernel). `verifyReducibility` processes the nodes
of the loop summary's depth-first spanning tree in reverse pre-order. For a
head with back edges, it collects the loops of the back-edge sources, grows
that body backwards along the other incoming edges, requires every loop
found to start at or after the head, and collapses the body into the head.

If it accepts, every walk from the entry to a back-edge source passes
through the back edge's target (`back_dominated`). The invariant (`HeadInv`)
keeps the loop partition as a union-find forest whose classes are the
collapsed loops: a node not yet processed is alone in its class, and an
edge between two classes enters a class at its representative, along a back
edge only into a node not yet processed. A walk from the entry that reached
a back-edge source of the head without the head would, read backwards, stay
inside the body, which holds only nodes after the head; but the entry comes
first.

Only two facts about the loop summary are used (`SummaryOk`): the recorded
edges join nodes of the summary, and a back edge never leads to a later
node.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Arrays -/

theorem getElem!_set! (ps : Array Nat) (d v w : Nat) :
    (ps.set! d v)[w]! = if w = d ∧ d < ps.size then v else ps[w]! := by
  simp only [Array.set!, getElem!_def, Array.getElem?_setIfInBounds]
  by_cases h : w = d
  · subst h
    by_cases hd : w < ps.size <;> simp [hd]
  · simp [Ne.symm h, h]

theorem size_set! (ps : Array Nat) (d v : Nat) : (ps.set! d v).size = ps.size := by
  simp [Array.set!]

/-! ## Union-find -/

/-- The representative of a node in the loop partition: follow the parents
while they decrease. -/
def rep (ps : Array Nat) (w : Nat) : Nat :=
  if _ : ps[w]! < w then rep ps ps[w]! else w
termination_by w

/-- No parent lies above its child. -/
abbrev Down (ps : Array Nat) : Prop := ∀ w : Nat, ps[w]! ≤ w

theorem rep_of_lt {ps : Array Nat} {w : Nat} (h : ps[w]! < w) : rep ps w = rep ps ps[w]! := by
  rw [rep]; simp [h]

theorem rep_of_not_lt {ps : Array Nat} {w : Nat} (h : ¬ ps[w]! < w) : rep ps w = w := by
  rw [rep]; simp [h]

theorem rep_le (ps : Array Nat) (w : Nat) : rep ps w ≤ w := by
  induction w using Nat.strongRecOn with
  | _ w ih =>
    by_cases h : ps[w]! < w
    · rw [rep_of_lt h]; exact Nat.le_trans (ih _ h) (Nat.le_of_lt h)
    · rw [rep_of_not_lt h]; exact Nat.le_refl w

theorem rep_root {ps : Array Nat} (hd : Down ps) (w : Nat) : ps[rep ps w]! = rep ps w := by
  induction w using Nat.strongRecOn with
  | _ w ih =>
    by_cases h : ps[w]! < w
    · rw [rep_of_lt h]; exact ih _ h
    · rw [rep_of_not_lt h]; exact Nat.le_antisymm (hd w) (Nat.le_of_not_lt h)

theorem rep_of_root {ps : Array Nat} {w : Nat} (h : ps[w]! = w) : rep ps w = w :=
  rep_of_not_lt (by omega)

theorem rep_rep {ps : Array Nat} (hd : Down ps) (w : Nat) : rep ps (rep ps w) = rep ps w :=
  rep_of_root (rep_root hd w)

/-- Pointing a node at its representative changes no representative. -/
theorem set_compress {ps : Array Nat} (hd : Down ps) {d r : Nat} (hr : rep ps d = r) :
    Down (ps.set! d r) ∧ ∀ w, rep (ps.set! d r) w = rep ps w := by
  have hrd : r ≤ d := hr ▸ rep_le ps d
  have hroot : ps[r]! = r := hr ▸ rep_root hd d
  refine ⟨fun w => ?_, fun w => ?_⟩
  · rw [getElem!_set!]
    split
    · next h => omega
    · exact hd w
  · induction w using Nat.strongRecOn with
    | ind w ih =>
      by_cases hw : w = d ∧ d < ps.size
      · obtain ⟨rfl, hws⟩ := hw
        have hv : (ps.set! w r)[w]! = r := by rw [getElem!_set!]; simp [hws]
        by_cases hlt : r < w
        · have hr' : (ps.set! w r)[r]! = r := by
            rw [getElem!_set!]; simp [Nat.ne_of_lt hlt, hroot]
          rw [rep_of_lt (by rw [hv]; exact hlt), hv, rep_of_root hr', hr]
        · rw [rep_of_not_lt (by rw [hv]; exact hlt)]
          omega
      · have hv : (ps.set! d r)[w]! = ps[w]! := by rw [getElem!_set!]; simp [hw]
        by_cases hlt : ps[w]! < w
        · rw [rep_of_lt (by rw [hv]; exact hlt), hv, ih _ hlt, ← rep_of_lt hlt]
        · rw [rep_of_not_lt (by rw [hv]; exact hlt), rep_of_not_lt hlt]

theorem compress_fold {r : Nat} : ∀ (ds : List Nat) {ps : Array Nat}, Down ps →
    (∀ d ∈ ds, rep ps d = r) →
    Down (ds.foldl (fun ps d => ps.set! d r) ps) ∧
      (∀ w, rep (ds.foldl (fun ps d => ps.set! d r) ps) w = rep ps w) ∧
      (ds.foldl (fun ps d => ps.set! d r) ps).size = ps.size
  | [], _, hd, _ => ⟨hd, fun _ => rfl, rfl⟩
  | d :: ds, ps, hd, hds => by
    obtain ⟨hd1, hrep1⟩ := set_compress hd (hds d (List.mem_cons_self ..))
    obtain ⟨hd2, hrep2, hsize2⟩ := compress_fold ds hd1 fun d' hd' => by
      rw [hrep1]; exact hds d' (List.mem_cons_of_mem _ hd')
    refine ⟨hd2, fun w => by simp only [List.foldl_cons]; rw [hrep2, hrep1], ?_⟩
    simp only [List.foldl_cons]; rw [hsize2, size_set!]

/-- The walk of `containing_loop` up a parent chain. -/
theorem walk_ok (p : LoopPartition) (hd : Down p.parents) :
    ∀ fuel c pa g ds r ds', p.parents[c]! = pa → p.parents[pa]! = g → pa ≠ c → pa ≠ g →
      LoopPartition.containingLoop.walk p fuel c pa g ds = .ok (r, ds') →
      r = rep p.parents c ∧ ∃ extra, ds' = ds ++ extra ∧ ∀ d ∈ extra, rep p.parents d = r
  | 0, _, _, _, _, _, _, _, _, _, _, h => by
    simp [LoopPartition.containingLoop.walk, outOfFuel] at h
  | fuel + 1, c, pa, g, ds, r, ds', hpa, hg, hc, hpg, h => by
    have hlt1 : pa < c := by have := hd c; omega
    have hlt2 : g < pa := by have := hd pa; omega
    have hrc : rep p.parents c = rep p.parents g := by
      rw [rep_of_lt (by omega), hpa, rep_of_lt (by omega), hg]
    simp only [LoopPartition.containingLoop.walk, LoopPartition.parent] at h
    split at h
    · next hgg =>
      simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      have hroot : rep p.parents g = g := rep_of_root (by simp at hgg; omega)
      refine ⟨by rw [hrc, hroot], [c], rfl, fun d hd' => ?_⟩
      simp only [List.mem_singleton] at hd'
      subst hd'; rw [hrc, hroot]
    · next hgg =>
      obtain ⟨hr, extra, hds, hextra⟩ :=
        walk_ok p hd fuel pa g _ _ r ds' hg rfl (Nat.ne_of_lt hlt2) (by simpa using hgg) h
      subst hds
      have hrpa : rep p.parents pa = rep p.parents g := by rw [rep_of_lt (by omega), hg]
      refine ⟨by rw [hr, hrc, hrpa], [c] ++ extra, by simp, fun d hd' => ?_⟩
      rcases List.mem_append.1 hd' with hd' | hd'
      · simp only [List.mem_singleton] at hd'; subst hd'; rw [hr, hrc, hrpa]
      · exact hextra d hd'

/-- `containing_loop` returns the representative and changes none. -/
theorem containingLoop_ok {p : LoopPartition} (hd : Down p.parents) {id fuel r : Nat}
    {p' : LoopPartition} (h : p.containingLoop id fuel = .ok (r, p')) :
    r = rep p.parents id ∧ Down p'.parents ∧ (∀ w, rep p'.parents w = rep p.parents w) ∧
      p'.parents.size = p.parents.size := by
  unfold LoopPartition.containingLoop at h
  simp only [LoopPartition.parent] at h
  split at h
  · next hc =>
    simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    refine ⟨?_, hd, fun _ => rfl, rfl⟩
    simp only [Bool.or_eq_true, beq_iff_eq] at hc
    rcases hc with hc | hc
    · rw [← hc, rep_of_root hc.symm]
    · by_cases hid : p.parents[id]! < id
      · rw [rep_of_lt hid]
        exact (rep_of_root hc.symm).symm
      · have := hd id
        rw [rep_of_not_lt hid]
        omega
  · next hc =>
    simp only [Bool.or_eq_true, beq_iff_eq, not_or] at hc
    simp only [bind_ok] at h
    obtain ⟨⟨root, ds⟩, hw, h⟩ := h
    simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    obtain ⟨hr, extra, hds, hextra⟩ :=
      walk_ok p hd _ _ _ _ _ _ _ rfl rfl (Ne.symm hc.1) hc.2 hw
    subst hds
    obtain ⟨hd2, hrep2, hsize2⟩ := compress_fold _ hd hextra
    exact ⟨hr, by simpa using hd2, by simpa using hrep2, by simpa using hsize2⟩

/-! ## Collapsing a loop -/

theorem getElem!_ge {ps : Array Nat} {w : Nat} (h : ps.size ≤ w) : ps[w]! = 0 := by
  simp [h]

/-- Pointing a root at an earlier root merges its class into the latter's. -/
theorem set_collapse {ps : Array Nat} (hd : Down ps) {c head : Nat} (hc : ps[c]! = c)
    (hhead : ps[head]! = head) (hlt : head < c) :
    Down (ps.set! c head) ∧ (ps.set! c head)[head]! = head ∧
      ∀ w, rep (ps.set! c head) w = if rep ps w = c then head else rep ps w := by
  have hhead' : (ps.set! c head)[head]! = head := by
    rw [getElem!_set!]; simp [Nat.ne_of_lt hlt, hhead]
  refine ⟨fun w => ?_, hhead', fun w => ?_⟩
  · rw [getElem!_set!]
    split
    · next h => omega
    · exact hd w
  · induction w using Nat.strongRecOn with
    | ind w ih =>
      by_cases hw : w = c ∧ c < ps.size
      · obtain ⟨rfl, hws⟩ := hw
        have hv : (ps.set! w head)[w]! = head := by rw [getElem!_set!]; simp [hws]
        rw [rep_of_lt (by rw [hv]; exact hlt), hv, rep_of_root hhead', rep_of_root hc]
        simp
      · have hv : (ps.set! c head)[w]! = ps[w]! := by rw [getElem!_set!]; simp [hw]
        by_cases hl : ps[w]! < w
        · rw [rep_of_lt (by rw [hv]; exact hl), hv, ih _ hl, ← rep_of_lt hl]
        · rw [rep_of_not_lt (by rw [hv]; exact hl), rep_of_not_lt hl]
          have hwc : w ≠ c := by
            intro hwc
            subst hwc
            have : ps.size ≤ w := by omega
            rw [getElem!_ge this] at hc
            omega
          simp [hwc]

theorem collapse_fold {head : Nat} : ∀ (body : List Nat) {ps : Array Nat}, Down ps →
    ps[head]! = head → body.Nodup → (∀ c ∈ body, ps[c]! = c ∧ head < c) →
    Down (body.foldl (fun ps c => ps.set! c head) ps) ∧
      (body.foldl (fun ps c => ps.set! c head) ps).size = ps.size ∧
      ∀ w, rep (body.foldl (fun ps c => ps.set! c head) ps) w =
        if rep ps w ∈ body then head else rep ps w
  | [], _, hd, _, _, _ => ⟨hd, rfl, fun w => by simp⟩
  | c :: body, ps, hd, hh, hnd, hbody => by
    obtain ⟨hc, hlt⟩ := hbody c (List.mem_cons_self ..)
    obtain ⟨hd1, hh1, hrep1⟩ := set_collapse hd hc hh hlt
    have hnd' := List.nodup_cons.1 hnd
    obtain ⟨hd2, hsize2, hrep2⟩ := collapse_fold body hd1 hh1 hnd'.2 fun c' hc' => by
      obtain ⟨h1, h2⟩ := hbody c' (List.mem_cons_of_mem _ hc')
      refine ⟨?_, h2⟩
      have : c' ≠ c := fun h => hnd'.1 (h ▸ hc')
      rw [getElem!_set!]
      simp [this, h1]
    refine ⟨hd2, by simp only [List.foldl_cons]; rw [hsize2, size_set!], fun w => ?_⟩
    simp only [List.foldl_cons]
    rw [hrep2, hrep1]
    by_cases hw : rep ps w = c
    · have : head ∉ body := fun hm => by
        have := (hbody head (List.mem_cons_of_mem _ hm)).2
        omega
      simp [hw, this]
    · simp [hw]

theorem collapse_parents_aux (p : LoopPartition) (head : Nat) :
    ∀ (l : List Nat) (d : Nat) (ps : Array Nat),
      (l.foldl (fun (acc : Nat × Array Nat) c =>
        match acc with
        | (depth, parents) => (Nat.max (p.depths[c]!) depth, parents.set! c head)) (d, ps)).2 =
        l.foldl (fun ps c => ps.set! c head) ps
  | [], _, _ => rfl
  | _ :: l, _, _ => collapse_parents_aux p head l _ _

theorem collapse_parents (p : LoopPartition) (head : Nat) (body : List Nat) :
    (p.collapseLoop head body).2.parents = body.foldl (fun ps c => ps.set! c head) p.parents := by
  rw [← collapse_parents_aux p head body (p.depths[head]!) p.parents]
  unfold LoopPartition.collapseLoop
  generalize List.foldl _ _ body = r
  rcases r with ⟨d, ps⟩
  rfl

/-! ## Sorted insertion -/

theorem insertSorted_mem {xs : List Nat} {x y : Nat} :
    y ∈ (insertSorted xs x).1 ↔ y ∈ xs ∨ y = x := by
  unfold insertSorted
  split
  · next h =>
    simp only [List.contains_iff_mem] at h
    constructor
    · exact Or.inl
    · rintro (h' | rfl)
      · exact h'
      · exact h
  · simp [(List.mergeSort_perm _ _).mem_iff]

theorem insertSorted_nodup {xs : List Nat} {x : Nat} (h : xs.Nodup) :
    (insertSorted xs x).1.Nodup := by
  unfold insertSorted
  split
  · exact h
  · next hx =>
    simp only [List.contains_iff_mem] at hx
    refine (List.mergeSort_perm _ _).symm.nodup_iff.1 ?_
    rw [List.nodup_append]
    simp [h]
    intro a ha hax
    exact hx (hax ▸ ha)

theorem insertSorted_flag {xs : List Nat} {x : Nat} (h : (insertSorted xs x).2 = false) :
    x ∈ xs := by
  unfold insertSorted at h
  split at h
  · next hx => simpa using hx
  · simp at h

/-! ## The body of a loop -/

/-- The partition's steps change no representative. -/
structure SameReps (ps0 : Array Nat) (p : LoopPartition) : Prop where
  down : Down p.parents
  same : ∀ w, rep p.parents w = rep ps0 w
  size : p.parents.size = ps0.size

theorem SameReps.containingLoop {ps0 : Array Nat} {p : LoopPartition} (hp : SameReps ps0 p)
    {id fuel r : Nat} {p' : LoopPartition} (h : p.containingLoop id fuel = .ok (r, p')) :
    r = rep ps0 id ∧ SameReps ps0 p' := by
  obtain ⟨rfl, hd, hrep, hsize⟩ := containingLoop_ok hp.down h
  exact ⟨hp.same id, hd, fun w => by rw [hrep, hp.same], by rw [hsize, hp.size]⟩

theorem backStep_ok {ps0 : Array Nat} {nodeFuel head : Nat} {body : List Nat}
    {p : LoopPartition} {node : Nat} {body' : List Nat} {p' : LoopPartition}
    (hp : SameReps ps0 p) (h : backStep nodeFuel head (body, p) node = .ok (body', p')) :
    SameReps ps0 p' ∧
      body' = if rep ps0 node ≠ head then (insertSorted body (rep ps0 node)).1 else body := by
  unfold backStep at h
  simp only [bind_ok] at h
  obtain ⟨⟨node', p1⟩, hc, h⟩ := h
  simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
  obtain ⟨rfl, rfl⟩ := h
  obtain ⟨rfl, hp1⟩ := hp.containingLoop hc
  refine ⟨hp1, ?_⟩
  by_cases hh : rep ps0 node = head <;> simp [hh]

/-- The loops of the back-edge sources: each joins the body unless it is the
head. -/
theorem backBody_fold {ps0 : Array Nat} {nodeFuel head : Nat} :
    ∀ (back body : List Nat) (p : LoopPartition) (body' : List Nat) (p' : LoopPartition),
      SameReps ps0 p → body.Nodup →
      back.foldlM (backStep nodeFuel head) (body, p) = .ok (body', p') →
      SameReps ps0 p' ∧ body'.Nodup ∧
        (∀ r ∈ body', r ∈ body ∨ ∃ u ∈ back, rep ps0 u = r ∧ r ≠ head) ∧
        (∀ u ∈ back, rep ps0 u = head ∨ rep ps0 u ∈ body')
  | [], body, p, body', p', hp, hnd, h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    exact ⟨hp, hnd, fun r hr => Or.inl hr, fun u hu => by simp at hu⟩
  | u :: back, body, p, body', p', hp, hnd, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨⟨body1, p1⟩, h1, h⟩ := h
    obtain ⟨hp1, rfl⟩ := backStep_ok hp h1
    have hnd1 : (if rep ps0 u ≠ head then (insertSorted body (rep ps0 u)).1 else body).Nodup := by
      split
      · exact insertSorted_nodup hnd
      · exact hnd
    have hsub1 : ∀ r ∈ (if rep ps0 u ≠ head then (insertSorted body (rep ps0 u)).1 else body),
        r ∈ body ∨ (r = rep ps0 u ∧ r ≠ head) := by
      intro r hr
      split at hr
      · next hh =>
        rcases insertSorted_mem.1 hr with hr | rfl
        · exact Or.inl hr
        · exact Or.inr ⟨rfl, hh⟩
      · exact Or.inl hr
    obtain ⟨hp2, hnd2, hnew2, hall2⟩ := backBody_fold back _ p1 body' p' hp1 hnd1 h
    refine ⟨hp2, hnd2, fun r hr => ?_, fun u' hu' => ?_⟩
    · rcases hnew2 r hr with hr | ⟨u', hu', h1, h2⟩
      · rcases hsub1 r hr with hr | ⟨rfl, hne⟩
        · exact Or.inl hr
        · exact Or.inr ⟨u, List.mem_cons_self .., rfl, hne⟩
      · exact Or.inr ⟨u', List.mem_cons_of_mem _ hu', h1, h2⟩
    · rcases List.mem_cons.1 hu' with rfl | hu'
      · by_cases hh : rep ps0 u' = head
        · exact Or.inl hh
        · -- The loop joined the body, which only grows afterwards.
          have hin : rep ps0 u' ∈ (if rep ps0 u' ≠ head then
              (insertSorted body (rep ps0 u')).1 else body) := by
            simp only [ne_eq, hh, not_false_eq_true, if_true]
            exact insertSorted_mem.2 (Or.inr rfl)
          exact Or.inr (body_mono back hp1 hnd1 h _ hin)
      · exact hall2 u' hu'
where
  /-- The body only grows. -/
  body_mono : ∀ (back : List Nat) {body : List Nat} {p : LoopPartition} {body' : List Nat}
      {p' : LoopPartition}, SameReps ps0 p → body.Nodup →
      back.foldlM (backStep nodeFuel head) (body, p) = .ok (body', p') → ∀ r ∈ body, r ∈ body'
    | [], _, _, _, _, _, _, h => by
      simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      exact fun r hr => hr
    | u :: back, body, p, body', p', hp, hnd, h => by
      simp only [List.foldlM_cons, bind_ok] at h
      obtain ⟨⟨body1, p1⟩, h1, h⟩ := h
      obtain ⟨hp1, rfl⟩ := backStep_ok hp h1
      have hnd1 : (if rep ps0 u ≠ head then (insertSorted body (rep ps0 u)).1 else body).Nodup := by
        split
        · exact insertSorted_nodup hnd
        · exact hnd
      intro r hr
      refine body_mono back hp1 hnd1 h r ?_
      split
      · exact insertSorted_mem.2 (Or.inl hr)
      · exact hr

theorem isDescendant_le {s : LoopSummary} {a d : Nat} (h : s.isDescendant a d = true) : a ≤ d := by
  simp only [LoopSummary.isDescendant, Bool.and_eq_true, decide_eq_true_eq] at h
  exact h.1

/-- One predecessor of a frontier node. -/
theorem visitPred_ok {ps0 : Array Nat} {s : LoopSummary} {head nodeFuel : Nat}
    {body frontier : List Nat} {p : LoopPartition} {q : Nat}
    {body' frontier' : List Nat} {p' : LoopPartition} (hp : SameReps ps0 p) (hnd : body.Nodup)
    (h : visitPred s head nodeFuel (body, frontier, p) q = .ok (body', frontier', p')) :
    SameReps ps0 p' ∧ body'.Nodup ∧ head ≤ rep ps0 q ∧
      (rep ps0 q = head ∨ rep ps0 q ∈ body') ∧ (∀ r ∈ body, r ∈ body') ∧
      (∀ r ∈ frontier, r ∈ frontier') ∧
      (∀ r ∈ body', r ∈ body ∨ (r = rep ps0 q ∧ r ≠ head ∧ r ∈ frontier')) := by
  unfold visitPred at h
  simp only [bind_ok] at h
  obtain ⟨⟨q', p1⟩, hc, h⟩ := h
  obtain ⟨rfl, hp1⟩ := hp.containingLoop hc
  split at h
  · simp [reject, bind, Except.bind] at h
  · next hdesc =>
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hdesc
    have hle := isDescendant_le (by simpa using hdesc)
    simp only [pure, Except.pure] at h
    split at h
    · next hne =>
      simp only [bne_iff_ne, ne_eq] at hne
      generalize hins : insertSorted body (rep ps0 q) = ins at h
      rcases ins with ⟨body1, inserted⟩
      simp only [Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl, rfl⟩ := h
      have hmem : ∀ r, r ∈ body1 ↔ r ∈ body ∨ r = rep ps0 q := by
        intro r; rw [← insertSorted_mem, hins]
      refine ⟨hp1, by have := insertSorted_nodup (x := rep ps0 q) hnd; rwa [hins] at this, hle,
        Or.inr ((hmem _).2 (Or.inr rfl)), fun r hr => (hmem r).2 (Or.inl hr),
        fun r hr => by split <;> simp [hr], fun r hr => ?_⟩
      rcases (hmem r).1 hr with hr | rfl
      · exact Or.inl hr
      · cases hflag : inserted
        · have := insertSorted_flag (xs := body) (x := rep ps0 q) (by rw [hins]; exact hflag)
          exact Or.inl this
        · exact Or.inr ⟨rfl, hne, by simp⟩
    · next hne =>
      simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hne
      simp only [Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl, rfl⟩ := h
      exact ⟨hp1, hnd, hle, Or.inl hne, fun r hr => hr, fun r hr => hr, fun r hr => Or.inl hr⟩

/-- The predecessors of one frontier node. -/
theorem visitPreds_ok {ps0 : Array Nat} {s : LoopSummary} {head nodeFuel : Nat} :
    ∀ (qs : List Nat) (body frontier : List Nat) (p : LoopPartition)
      (body' frontier' : List Nat) (p' : LoopPartition), SameReps ps0 p → body.Nodup →
      qs.foldlM (visitPred s head nodeFuel) (body, frontier, p) = .ok (body', frontier', p') →
      SameReps ps0 p' ∧ body'.Nodup ∧ (∀ r ∈ body, r ∈ body') ∧
        (∀ r ∈ frontier, r ∈ frontier') ∧
        (∀ q ∈ qs, rep ps0 q = head ∨ rep ps0 q ∈ body') ∧
        (∀ r ∈ body', r ∈ body ∨ (r ∈ frontier' ∧ head < r ∧ ∃ q ∈ qs, rep ps0 q = r))
  | [], body, frontier, p, body', frontier', p', hp, hnd, h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl, rfl⟩ := h
    exact ⟨hp, hnd, fun r hr => hr, fun r hr => hr, fun q hq => by simp at hq,
      fun r hr => Or.inl hr⟩
  | q :: qs, body, frontier, p, body', frontier', p', hp, hnd, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨⟨body1, frontier1, p1⟩, h1, h⟩ := h
    obtain ⟨hp1, hnd1, hle1, hq1, hsub1, hfr1, hnew1⟩ := visitPred_ok hp hnd h1
    obtain ⟨hp2, hnd2, hsub2, hfr2, hqs2, hnew2⟩ :=
      visitPreds_ok qs body1 frontier1 p1 body' frontier' p' hp1 hnd1 h
    refine ⟨hp2, hnd2, fun r hr => hsub2 r (hsub1 r hr), fun r hr => hfr2 r (hfr1 r hr),
      fun q' hq' => ?_, fun r hr => ?_⟩
    · rcases List.mem_cons.1 hq' with rfl | hq'
      · rcases hq1 with h | h
        · exact Or.inl h
        · exact Or.inr (hsub2 _ h)
      · exact hqs2 q' hq'
    · rcases hnew2 r hr with hr | ⟨hfr, hlt, q', hq', hrq⟩
      · rcases hnew1 r hr with hr | ⟨rfl, hne, hfr⟩
        · exact Or.inl hr
        · exact Or.inr ⟨hfr2 _ hfr, by omega, q, List.mem_cons_self .., rfl⟩
      · exact Or.inr ⟨hfr, hlt, q', List.mem_cons_of_mem _ hq', hrq⟩

/-- Every predecessor of a body node lies in the body or in the head's class. -/
def Closed (ps0 : Array Nat) (s : LoopSummary) (head : Nat) (B : List Nat) (r : Nat) : Prop :=
  ∀ q ∈ s.predEdges r, rep ps0 q = head ∨ rep ps0 q ∈ B

/-- Growing the body: when the frontier empties, every body node is closed. -/
theorem growBody_ok {ps0 : Array Nat} {s : LoopSummary} {head nodeFuel : Nat}
    (good : Nat → Prop)
    (hnew : ∀ r q, head < r → rep ps0 q = r → (∃ node, q ∈ s.predEdges node) → good r) :
    ∀ (fuel : Nat) (body frontier : List Nat) (p : LoopPartition) (B : List Nat)
      (p' : LoopPartition), SameReps ps0 p → body.Nodup → (∀ r ∈ body, good r) →
      (∀ r ∈ body, r ∈ frontier ∨ Closed ps0 s head body r) →
      growBody s head nodeFuel fuel body frontier p = .ok (B, p') →
      SameReps ps0 p' ∧ B.Nodup ∧ (∀ r ∈ body, r ∈ B) ∧ (∀ r ∈ B, good r) ∧
        ∀ r ∈ B, Closed ps0 s head B r
  | 0, _, _, _, _, _, _, _, _, _, h => by simp [growBody, outOfFuel] at h
  | fuel + 1, body, frontier, p, B, p', hp, hnd, hgd, hst, h => by
    unfold growBody at h
    split at h
    · simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      refine ⟨hp, hnd, fun r hr => hr, hgd, fun r hr => ?_⟩
      rcases hst r hr with h | h
      · simp at h
      · exact h
    · next node rest =>
      simp only [bind_ok] at h
      obtain ⟨⟨body1, frontier1, p1⟩, h1, h⟩ := h
      obtain ⟨hp1, hnd1, hsub1, hfr1, hqs1, hnew1⟩ := visitPreds_ok _ _ _ _ _ _ _ hp hnd h1
      have hgd1 : ∀ r ∈ body1, good r := by
        intro r hr
        rcases hnew1 r hr with hr | ⟨-, hlt, q, hq, hrq⟩
        · exact hgd r hr
        · exact hnew r q hlt hrq ⟨node, hq⟩
      have hst1 : ∀ r ∈ body1, r ∈ frontier1 ∨ Closed ps0 s head body1 r := by
        intro r hr
        rcases hnew1 r hr with hr | ⟨hfr, -, -⟩
        · rcases hst r hr with hfr | hcl
          · rcases List.mem_cons.1 hfr with rfl | hfr
            · exact Or.inr hqs1
            · exact Or.inl (hfr1 r hfr)
          · exact Or.inr fun q hq => (hcl q hq).imp id (hsub1 _)
        · exact Or.inl hfr
      obtain ⟨hp2, hnd2, hsub2, hgd2, hcl2⟩ :=
        growBody_ok good hnew fuel body1 frontier1 p1 B p' hp1 hnd1 hgd1 hst1 h
      exact ⟨hp2, hnd2, fun r hr => hsub2 r (hsub1 r hr), hgd2, hcl2⟩

/-! ## The heads -/

/-- An edge the loop summary records: `w` is a predecessor of `w'`. -/
def Recorded (s : LoopSummary) (w w' : Nat) : Prop :=
  w ∈ s.predEdges w' ∨ w ∈ s.backEdges w'

/-- A walk along recorded edges, listing its nodes. -/
inductive RWalk (s : LoopSummary) : Nat → Nat → List Nat → Prop where
  | nil (a : Nat) : RWalk s a a [a]
  | cons {a b c : Nat} {p : List Nat} : Recorded s a b → RWalk s b c p → RWalk s a c (a :: p)

/-- What the loop nesting needs of the loop summary: the recorded edges join
nodes of the summary, and a back edge never leads to a later node. -/
structure SummaryOk (s : LoopSummary) : Prop where
  range : ∀ w w', Recorded s w w' → w < s.blocks.size ∧ w' < s.blocks.size
  backLe : ∀ h x, x ∈ s.backEdges h → h ≤ x

/-- The partition before the heads below `k` are processed: a node below `k`
is alone in its class, and an edge between classes enters its target's
representative, along a back edge only below `k`. -/
structure HeadInv (s : LoopSummary) (ps : Array Nat) (k : Nat) : Prop where
  down : Down ps
  size : ps.size = s.blocks.size
  unproc : ∀ y < k, ∀ w < s.blocks.size, rep ps w = y ↔ w = y
  pred : ∀ w' w, w ∈ s.predEdges w' → rep ps w ≠ rep ps w' → rep ps w' = w'
  back : ∀ w' w, w ∈ s.backEdges w' → rep ps w ≠ rep ps w' → w' < k ∧ rep ps w' = w'

/-- Read backwards, a walk to a back-edge source of head `k` that avoids
`k` stays in the loop body. -/
theorem walk_body {s : LoopSummary} (hs : SummaryOk s) {ps : Array Nat} {k : Nat}
    (hinv : HeadInv s ps (k + 1)) {B : List Nat} (hgt : ∀ r ∈ B, k < r)
    (hcl : ∀ r ∈ B, Closed ps s k B r)
    (hback : ∀ u ∈ s.backEdges k, rep ps u = k ∨ rep ps u ∈ B) {u : Nat}
    (hu : u ∈ s.backEdges k) :
    ∀ {a c : Nat} {path : List Nat}, RWalk s a c path → c = u → k ∉ path → rep ps a ∈ B := by
  intro a c path hw
  induction hw with
  | nil a =>
    intro hc hk
    subst hc
    have hne : a ≠ k := fun h => hk (by simp [h])
    rcases hback a hu with h | h
    · exact absurd ((hinv.unproc k (by omega) a (hs.range a k (Or.inr hu)).1).1 h) hne
    · exact h
  | @cons a b c p hrec _ ih =>
    intro hc hk
    have hb := ih hc fun h => hk (List.mem_cons_of_mem _ h)
    have hne : a ≠ k := fun h => hk (by simp [h])
    by_cases heq : rep ps a = rep ps b
    · rw [heq]; exact hb
    · have ha := (hs.range a b hrec).1
      rcases hrec with hp | hbk
      · rw [hinv.pred b a hp heq] at hb
        rcases hcl b hb a hp with h | h
        · exact absurd ((hinv.unproc k (by omega) a ha).1 h) hne
        · exact h
      · obtain ⟨hlt, hroot⟩ := hinv.back b a hbk heq
        rw [hroot] at hb
        have := hgt b hb
        omega

theorem root_of_rep {ps : Array Nat} (hd : Down ps) {r : Nat} (h : rep ps r = r) : ps[r]! = r := by
  have := rep_root hd r
  rwa [h] at this

/-- Processing head `k`: the invariant moves to `k`, and every walk from the
entry to a back-edge source of `k` passes through `k`. -/
theorem processHead_ok {config : VerifierConfig} {s : LoopSummary} (hs : SummaryOk s)
    {nodeFuel growFuel k : Nat} {p p' : LoopPartition} (hinv : HeadInv s p.parents (k + 1))
    (h : processHead config s nodeFuel growFuel p k = .ok p') :
    HeadInv s p'.parents k ∧ ∀ u ∈ s.backEdges k, ∀ path, RWalk s 0 u path → k ∈ path := by
  unfold processHead at h
  by_cases hempty : (s.backEdges k).isEmpty
  · have hnil : s.backEdges k = [] := List.isEmpty_iff.1 hempty
    simp only [hempty, if_true, pure, Except.pure, Except.ok.injEq] at h
    subst h
    refine ⟨⟨hinv.down, hinv.size, fun y hy => hinv.unproc y (by omega), hinv.pred,
      fun w' w hw hne => ?_⟩, fun u hu => by simp [hnil] at hu⟩
    obtain ⟨hlt, hroot⟩ := hinv.back w' w hw hne
    refine ⟨?_, hroot⟩
    rcases Nat.lt_or_ge w' k with h | h
    · exact h
    · have : w' = k := by omega
      subst this; simp [hnil] at hw
  · simp only [hempty, Bool.false_eq_true, if_false, bind_ok] at h
    obtain ⟨⟨body0, p1⟩, h1, h⟩ := h
    obtain ⟨⟨B, p2⟩, h2, h⟩ := h
    have hrep0 : SameReps p.parents p := ⟨hinv.down, fun _ => rfl, rfl⟩
    obtain ⟨hp1, hnd0, hnew0, hall0⟩ := backBody_fold _ [] p body0 p1 hrep0 List.nodup_nil h1
    let n := s.blocks.size
    let good (r : Nat) : Prop := rep p.parents r = r ∧ k < r ∧ r < n
    have hgd0 : ∀ r ∈ body0, good r := by
      intro r hr
      rcases hnew0 r hr with hr | ⟨u, hu, rfl, hne⟩
      · simp at hr
      · have hun := (hs.range u k (Or.inr hu)).1
        have hku := hs.backLe k u hu
        refine ⟨rep_rep hinv.down u, ?_, Nat.lt_of_le_of_lt (rep_le _ u) hun⟩
        rcases Nat.lt_or_ge k (rep p.parents u) with h | h
        · exact h
        · exfalso
          have := (hinv.unproc _ (by omega) u hun).1 rfl
          have hk : u = k := by have := rep_le p.parents u; omega
          subst hk
          exact hne ((hinv.unproc u (by omega) u hun).2 rfl)
    have hnew : ∀ r q, k < r → rep p.parents q = r → (∃ node, q ∈ s.predEdges node) →
        good r := by
      rintro r q hlt rfl ⟨node, hq⟩
      exact ⟨rep_rep hinv.down q, hlt,
        Nat.lt_of_le_of_lt (rep_le _ q) (hs.range q node (Or.inl hq)).1⟩
    obtain ⟨hp2, hndB, hsubB, hgdB, hclB⟩ := growBody_ok good hnew growFuel body0 body0.reverse
      p1 B p2 hp1 hnd0 hgd0 (fun r hr => Or.inl (List.mem_reverse.2 hr)) h2
    -- Collapse the body into the head.
    have hroots : ∀ c ∈ B, p2.parents[c]! = c ∧ k < c := fun c hc =>
      ⟨root_of_rep hp2.down (by rw [hp2.same]; exact (hgdB c hc).1), (hgdB c hc).2.1⟩
    have hkn : k < n := by
      obtain ⟨u, hu⟩ : ∃ u, u ∈ s.backEdges k := by
        cases hb : s.backEdges k with
        | nil => simp [hb] at hempty
        | cons u _ => exact ⟨u, by simp⟩
      exact (hs.range u k (Or.inr hu)).2
    have hkroot : rep p.parents k = k := (hinv.unproc k (by omega) k hkn).2 rfl
    obtain ⟨hd3, hsize3, hrep3⟩ := collapse_fold B hp2.down
      (root_of_rep hp2.down (by rw [hp2.same]; exact hkroot)) hndB hroots
    have hrep3' : ∀ w, rep (B.foldl (fun ps c => ps.set! c k) p2.parents) w =
        if rep p.parents w ∈ B then k else rep p.parents w := fun w => by
      rw [hrep3, hp2.same]
    have hp' : p'.parents = B.foldl (fun ps c => ps.set! c k) p2.parents := by
      rw [← collapse_parents]
      generalize hcl : p2.collapseLoop k B = col at h
      rcases col with ⟨depth, p3⟩
      simp only at h ⊢
      split at h
      · split at h
        · simp [reject] at h
        · simp only [pure, Except.pure, Except.ok.injEq] at h
          subst h; rfl
      · simp only [pure, Except.pure, Except.ok.injEq] at h
        subst h; rfl
    have hkB : k ∉ B := fun hk => by have := (hgdB k hk).2.1; omega
    have hback : ∀ u ∈ s.backEdges k, rep p.parents u = k ∨ rep p.parents u ∈ B :=
      fun u hu => (hall0 u hu).imp id (hsubB _)
    refine ⟨⟨by rw [hp']; exact hd3, by rw [hp', hsize3, hp2.size, hinv.size], ?_, ?_, ?_⟩, ?_⟩
    · -- A node below `k` is still alone in its class.
      intro y hy w hw
      rw [hp', hrep3']
      split
      · next hB =>
        have := (hgdB _ hB).2.1
        constructor
        · intro h; omega
        · intro hwy
          subst hwy
          rw [(hinv.unproc w (by omega) w hw).2 rfl] at hB
          have := (hgdB _ hB).2.1
          omega
      · exact hinv.unproc y (by omega) w hw
    · -- An edge between classes enters its target's representative.
      intro w' w hw hne
      simp only [hp', hrep3'] at hne ⊢
      have hne0 : rep p.parents w ≠ rep p.parents w' := fun h => hne (by rw [h])
      have hroot := hinv.pred w' w hw hne0
      by_cases hB : w' ∈ B
      · exfalso
        rw [hroot] at hne
        rcases hclB w' hB w hw with h | h
        · simp only [h, hkB, if_false, hB, if_true] at hne; exact hne rfl
        · simp only [h, if_true, hB] at hne; exact hne rfl
      · rw [hroot]; simp [hB]
    · -- A back edge between classes enters below `k`.
      intro w' w hw hne
      simp only [hp', hrep3'] at hne ⊢
      have hne0 : rep p.parents w ≠ rep p.parents w' := fun h => hne (by rw [h])
      obtain ⟨hlt, hroot⟩ := hinv.back w' w hw hne0
      have hw'k : w' ≠ k := by
        rintro rfl
        rw [hkroot] at hne
        rcases hback w hw with h | h
        · simp only [h, hkB, if_false] at hne; exact hne rfl
        · simp only [h, if_true, hkB, if_false] at hne; exact hne rfl
      have hw'B : w' ∉ B := fun h => by have := (hgdB w' h).2.1; omega
      refine ⟨by omega, ?_⟩
      rw [hroot]; simp [hw'B]
    · -- Every walk from the entry to a back-edge source passes through `k`.
      intro u hu path hw
      refine Classical.byContradiction fun hk => ?_
      have h0 := walk_body hs hinv (fun r hr => (hgdB r hr).2.1) hclB hback hu hw rfl hk
      have : rep p.parents 0 = 0 := Nat.le_antisymm (rep_le _ 0) (Nat.zero_le _)
      rw [this] at h0
      have := (hgdB 0 h0).2.1
      omega

/-- The heads below `k`, in reverse pre-order. -/
theorem heads_ok {config : VerifierConfig} {s : LoopSummary} (hs : SummaryOk s)
    {nodeFuel growFuel : Nat} :
    ∀ (k : Nat) (p final : LoopPartition), HeadInv s p.parents k →
      (List.range k).reverse.foldlM (processHead config s nodeFuel growFuel) p = .ok final →
      ∀ h < k, ∀ u ∈ s.backEdges h, ∀ path, RWalk s 0 u path → h ∈ path
  | 0, _, _, _, _ => fun _ hh => absurd hh (Nat.not_lt_zero _)
  | k + 1, p, final, hinv, hfold => by
    rw [List.range_succ, List.reverse_append, List.reverse_singleton, List.singleton_append,
      List.foldlM_cons, bind_ok] at hfold
    obtain ⟨p1, h1, hfold⟩ := hfold
    obtain ⟨hinv1, hk⟩ := processHead_ok hs hinv h1
    intro h hh
    rcases Nat.lt_or_ge h k with hlt | hge
    · exact heads_ok hs k p1 final hinv1 hfold h hlt
    · have : h = k := by omega
      subst this
      exact hk

theorem range_getElem! (n w : Nat) : (List.range n).toArray[w]! = if w < n then w else 0 := by
  by_cases h : w < n <;> simp [h]

/-- The partition before any head is processed. -/
theorem headInv_new {s : LoopSummary} (hs : SummaryOk s) :
    HeadInv s (LoopPartition.new s).parents s.blocks.size := by
  have hroot : ∀ w < s.blocks.size, rep (LoopPartition.new s).parents w = w := fun w hw =>
    rep_of_root (by simp only [LoopPartition.new]; rw [range_getElem!]; simp [hw])
  refine ⟨fun w => ?_, by simp [LoopPartition.new], fun y hy w hw => by rw [hroot w hw],
    fun w' w hw _ => hroot w' (hs.range w w' (Or.inl hw)).2,
    fun w' w hw _ => ⟨(hs.range w w' (Or.inr hw)).2, hroot w' (hs.range w w' (Or.inr hw)).2⟩⟩
  simp only [LoopPartition.new]
  rw [range_getElem!]
  split <;> omega

/-- **Every back edge enters through a dominator**: if the model accepts
the control-flow graph, every walk from the entry to the source of a back
edge passes through its target. -/
theorem back_dominated {config : VerifierConfig} {cfg : Cfg}
    (h : verifyReducibility config cfg = .ok ()) {s : LoopSummary}
    (hb : LoopSummary.build cfg = .ok s) (hs : SummaryOk s) :
    ∀ h u, u ∈ s.backEdges h → ∀ path, RWalk s 0 u path → h ∈ path := by
  unfold verifyReducibility at h
  rw [hb] at h
  simp only [bind_ok] at h
  obtain ⟨s', hs', h⟩ := h
  cases hs'
  obtain ⟨final, hfold, -⟩ := h
  intro hd u hu
  exact heads_ok hs _ _ final (headInv_new hs) hfold hd (hs.range u hd (Or.inr hu)).2 u hu

end MoveBytecode.Proofs
