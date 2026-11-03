-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.LoopNesting
import Spec.Properties.Limits

/-!
# Loop depth (L11) from the reducibility check

UNTRUSTED (checked by the kernel). `verifyReducibility` collapses the loop of
each head of the loop summary into the head, in reverse pre-order, and
records the head's depth: one more than the greatest depth of the head and
of the loops it absorbs. It rejects a depth above the configured bound.

**On the loop summary** (`loopDepth_summary`). The natural loop of a head `h`
(`SLoop`) holds `h` and every node that reaches a back-edge source of `h`
without passing through `h`. When `h` is processed, every node of its
natural loop other than `h` is in a class the body absorbs (`walk_body`), so
the class of a node gains depth when the node is in the loop
(`processHead_depth`). Hence the number of heads processed so far whose loops
contain a node is at most the depth of the node's class, which the check
bounds.

**On instructions** (`loopDepthAtMost_of`). A back edge of the
specification (an edge to a dominator of a reachable instruction) joins the
last instruction of a block to the first of a block, and the loop summary
records it as a back edge: on a walk from the entry without repeated
instructions, every edge between blocks is recorded as another incoming edge
(a back edge's target dominates its source, so it would repeat the target),
along which the search's finishing order decreases; an edge recorded only as
another incoming edge to a dominator would close a cycle on which it
decreases strictly. A walk to the back edge's source that avoids its target
stays outside the target's block, so it is a walk of the summary that avoids
the head. Every head of an instruction's natural loops is thus a head of the
summary whose loop holds the instruction's block, and distinct heads start
distinct blocks.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## On the loop summary -/

/-- Node `x` is in the natural loop of head `h` of the loop summary: `h` has a
back edge, and `x` is `h` or reaches the back edge's source without passing
through `h`. -/
def SLoop (s : LoopSummary) (h x : Nat) : Prop :=
  ∃ u ∈ s.backEdges h, x = h ∨ ∃ path, RWalk s x u path ∧ h ∉ path

theorem SLoop.head_lt {s : LoopSummary} (hs : SummaryOk s) {h x : Nat} (hl : SLoop s h x) :
    h < s.blocks.size := by
  obtain ⟨u, hu, -⟩ := hl
  exact (hs.range u h (Or.inr hu)).2

/-! ### The partition's depths -/

theorem containingLoop_depths {p : LoopPartition} {id fuel r : Nat} {p' : LoopPartition}
    (h : p.containingLoop id fuel = .ok (r, p')) : p'.depths = p.depths := by
  unfold LoopPartition.containingLoop at h
  simp only [LoopPartition.parent] at h
  split at h
  · simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    rw [← h.2]
  · simp only [bind_ok] at h
    obtain ⟨⟨root, ds⟩, -, h⟩ := h
    simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    rw [← h.2]

theorem backBody_depths {nodeFuel head : Nat} :
    ∀ (back body : List Nat) (p : LoopPartition) (body' : List Nat) (p' : LoopPartition),
      back.foldlM (backStep nodeFuel head) (body, p) = .ok (body', p') → p'.depths = p.depths
  | [], body, p, body', p', h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    rw [← h.2]
  | u :: back, body, p, body', p', h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨⟨body1, p1⟩, h1, h⟩ := h
    rw [backBody_depths back body1 p1 body' p' h]
    unfold backStep at h1
    simp only [bind_ok] at h1
    obtain ⟨⟨node', p0⟩, hc, h1⟩ := h1
    simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h1
    rw [← h1.2]
    exact containingLoop_depths hc

theorem visitPred_depths {s : LoopSummary} {head nodeFuel : Nat} {body frontier : List Nat}
    {p : LoopPartition} {q : Nat} {body' frontier' : List Nat} {p' : LoopPartition}
    (h : visitPred s head nodeFuel (body, frontier, p) q = .ok (body', frontier', p')) :
    p'.depths = p.depths := by
  unfold visitPred at h
  simp only [bind_ok] at h
  obtain ⟨⟨q', p1⟩, hc, h⟩ := h
  have hd := containingLoop_depths hc
  split at h
  · simp [reject, bind, Except.bind] at h
  · simp only [pure, Except.pure] at h
    split at h
    · generalize insertSorted body q' = ins at h
      rcases ins with ⟨body1, inserted⟩
      simp only [Except.ok.injEq, Prod.mk.injEq] at h
      rw [← h.2.2, hd]
    · simp only [Except.ok.injEq, Prod.mk.injEq] at h
      rw [← h.2.2, hd]

theorem visitPreds_depths {s : LoopSummary} {head nodeFuel : Nat} :
    ∀ (qs : List Nat) (body frontier : List Nat) (p : LoopPartition)
      (body' frontier' : List Nat) (p' : LoopPartition),
      qs.foldlM (visitPred s head nodeFuel) (body, frontier, p) = .ok (body', frontier', p') →
      p'.depths = p.depths
  | [], body, frontier, p, body', frontier', p', h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    rw [← h.2.2]
  | q :: qs, body, frontier, p, body', frontier', p', h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨⟨body1, frontier1, p1⟩, h1, h⟩ := h
    rw [visitPreds_depths qs body1 frontier1 p1 body' frontier' p' h, visitPred_depths h1]

theorem growBody_depths {s : LoopSummary} {head nodeFuel : Nat} :
    ∀ (fuel : Nat) (body frontier : List Nat) (p : LoopPartition) (B : List Nat)
      (p' : LoopPartition), growBody s head nodeFuel fuel body frontier p = .ok (B, p') →
      p'.depths = p.depths
  | 0, _, _, _, _, _, h => by simp [growBody, outOfFuel] at h
  | fuel + 1, body, frontier, p, B, p', h => by
    unfold growBody at h
    split at h
    · simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      rw [← h.2]
    · simp only [bind_ok] at h
      obtain ⟨⟨body1, frontier1, p1⟩, h1, h⟩ := h
      rw [growBody_depths fuel body1 frontier1 p1 B p' h, visitPreds_depths _ _ _ _ _ _ _ h1]

theorem collapse_depth_aux (p : LoopPartition) (head : Nat) :
    ∀ (l : List Nat) (d : Nat) (ps : Array Nat),
      d ≤ (l.foldl (fun (acc : Nat × Array Nat) c =>
        match acc with
        | (depth, parents) => (Nat.max (p.depths[c]!) depth, parents.set! c head)) (d, ps)).1 ∧
      ∀ c ∈ l, p.depths[c]! ≤ (l.foldl (fun (acc : Nat × Array Nat) c =>
        match acc with
        | (depth, parents) => (Nat.max (p.depths[c]!) depth, parents.set! c head)) (d, ps)).1
  | [], _, _ => ⟨Nat.le_refl _, fun c hc => by simp at hc⟩
  | c :: l, d, ps => by
    obtain ⟨h1, h2⟩ := collapse_depth_aux p head l (Nat.max (p.depths[c]!) d) (ps.set! c head)
    simp only [List.foldl_cons]
    refine ⟨Nat.le_trans (Nat.le_max_right _ _) h1, fun c' hc' => ?_⟩
    rcases List.mem_cons.1 hc' with rfl | hc'
    · exact Nat.le_trans (Nat.le_max_left _ _) h1
    · exact h2 c' hc'

/-- Collapsing a loop: the head's new depth exceeds its own and that of every
class it absorbs, and is the only depth that changes. -/
theorem collapse_depth (p : LoopPartition) (head : Nat) (body : List Nat) :
    p.depths[head]! + 1 ≤ (p.collapseLoop head body).1 ∧
    (∀ c ∈ body, p.depths[c]! + 1 ≤ (p.collapseLoop head body).1) ∧
    (p.collapseLoop head body).2.depths = p.depths.set! head (p.collapseLoop head body).1 := by
  obtain ⟨h1, h2⟩ := collapse_depth_aux p head body (p.depths[head]!) p.parents
  unfold LoopPartition.collapseLoop
  generalize hf : List.foldl _ _ body = r at h1 h2
  rcases r with ⟨d, ps⟩
  exact ⟨Nat.succ_le_succ h1, fun c hc => Nat.succ_le_succ (h2 c hc), rfl⟩

/-! ### Processing one head -/

/-- Processing head `k`: the depth array keeps its size and takes only
accepted depths, the depth of a node's class never decreases, and it grows
for every node of the natural loop of `k`. -/
theorem processHead_depth {config : VerifierConfig} {s : LoopSummary} (hs : SummaryOk s)
    {kmax : Nat} (hkm : config.maxLoopDepth = some kmax)
    {nodeFuel growFuel k : Nat} {p p' : LoopPartition} (hinv : HeadInv s p.parents (k + 1))
    (hsize : p.depths.size = s.blocks.size)
    (h : processHead config s nodeFuel growFuel p k = .ok p') :
    p'.depths.size = s.blocks.size ∧
    (∀ r : Nat, p'.depths[r]! = p.depths[r]! ∨ p'.depths[r]! ≤ kmax) ∧
    (∀ x, p.depths[rep p.parents x]! ≤ p'.depths[rep p'.parents x]!) ∧
    (∀ x, SLoop s k x → p.depths[rep p.parents x]! + 1 ≤ p'.depths[rep p'.parents x]!) := by
  unfold processHead at h
  by_cases hempty : (s.backEdges k).isEmpty
  · have hnil : s.backEdges k = [] := List.isEmpty_iff.1 hempty
    simp only [hempty, if_true, pure, Except.pure, Except.ok.injEq] at h
    subst h
    refine ⟨hsize, fun r => Or.inl rfl, fun x => Nat.le_refl _, fun x hx => ?_⟩
    obtain ⟨u, hu, -⟩ := hx
    simp [hnil] at hu
  · simp only [hempty, Bool.false_eq_true, if_false, bind_ok] at h
    obtain ⟨⟨body0, p1⟩, h1, h⟩ := h
    obtain ⟨⟨B, p2⟩, h2, h⟩ := h
    have hrep0 : SameReps p.parents p := ⟨hinv.down, fun _ => rfl, rfl⟩
    obtain ⟨hp1, hnd0, hnew0, hall0⟩ := backBody_fold _ [] p body0 p1 hrep0 List.nodup_nil h1
    have hd1 := backBody_depths _ [] p body0 p1 h1
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
    have hd2 := growBody_depths _ _ _ _ _ _ h2
    have hroots : ∀ c ∈ B, p2.parents[c]! = c ∧ k < c := fun c hc =>
      ⟨root_of_rep hp2.down (by rw [hp2.same]; exact (hgdB c hc).1), (hgdB c hc).2.1⟩
    have hkn : k < n := by
      obtain ⟨u, hu⟩ : ∃ u, u ∈ s.backEdges k := by
        cases hb : s.backEdges k with
        | nil => simp [hb] at hempty
        | cons u _ => exact ⟨u, by simp⟩
      exact (hs.range u k (Or.inr hu)).2
    have hkroot : rep p.parents k = k := (hinv.unproc k (by omega) k hkn).2 rfl
    obtain ⟨-, -, hrep3⟩ := collapse_fold B hp2.down
      (root_of_rep hp2.down (by rw [hp2.same]; exact hkroot)) hndB hroots
    have hkB : k ∉ B := fun hk => by have := (hgdB k hk).2.1; omega
    have hback : ∀ u ∈ s.backEdges k, rep p.parents u = k ∨ rep p.parents u ∈ B :=
      fun u hu => (hall0 u hu).imp id (hsubB _)
    obtain ⟨hdk, hdB, hdset⟩ := collapse_depth p2 k B
    have hpar := collapse_parents p2 k B
    generalize hcl : p2.collapseLoop k B = col at h hdk hdB hdset hpar
    rcases col with ⟨depth, p3⟩
    simp only [hkm] at h
    split at h
    · simp [reject] at h
    · next hle =>
      simp only [pure, Except.pure, Except.ok.injEq] at h
      subst h
      simp only at hdk hdB hdset hpar
      rw [hd2, hd1] at hdk hdB hdset
      have hreps : ∀ w, rep p3.parents w =
          if rep p.parents w ∈ B then k else rep p.parents w := fun w => by
        rw [hpar, hrep3, hp2.same]
      have hdepths : ∀ r : Nat, p3.depths[r]! = if r = k then depth else p.depths[r]! :=
          fun r => by
        rw [hdset, getElem!_set!]
        by_cases hr : r = k
        · subst hr
          have hk' : r < s.blocks.size := hkn
          simp [hsize, hk']
        · simp [hr]
      have hle' : depth ≤ kmax := by omega
      refine ⟨by rw [hdset, Array.size_set!, hsize], fun r => ?_, fun x => ?_, fun x hx => ?_⟩
      · rw [hdepths]
        split
        · exact Or.inr hle'
        · exact Or.inl rfl
      · rw [hreps, hdepths]
        split
        · next hB => simp only [if_true]; exact Nat.le_of_succ_le (hdB _ hB)
        · split
          · next _ hk => rw [hk]; exact Nat.le_of_succ_le hdk
          · exact Nat.le_refl _
      · -- A node of the loop of `k` joins the head's class.
        have hin : rep p.parents x = k ∨ rep p.parents x ∈ B := by
          obtain ⟨u, hu, hx⟩ := hx
          rcases hx with rfl | ⟨path, hw, hkp⟩
          · exact Or.inl hkroot
          · exact Or.inr (walk_body hs hinv (fun r hr => (hgdB r hr).2.1) hclB hback hu hw rfl hkp)
        rw [hreps, hdepths]
        rcases hin with hin | hin
        · simp only [hin, hkB, if_false, if_true]; exact hdk
        · simp only [hin, if_true]; exact hdB _ hin

/-- Every node is in the natural loops of at most as many processed heads as
the depth of its class, which the check bounds. -/
theorem heads_depth {config : VerifierConfig} {s : LoopSummary} (hs : SummaryOk s)
    {kmax : Nat} (hkm : config.maxLoopDepth = some kmax) {nodeFuel growFuel : Nat} :
    ∀ (t : Nat) (p final : LoopPartition), HeadInv s p.parents t →
      p.depths.size = s.blocks.size → (∀ r : Nat, p.depths[r]! ≤ kmax) →
      (∀ x (heads : List Nat), heads.Nodup → (∀ h ∈ heads, t ≤ h ∧ SLoop s h x) →
        heads.length ≤ p.depths[rep p.parents x]!) →
      (List.range t).reverse.foldlM (processHead config s nodeFuel growFuel) p = .ok final →
      ∀ x (heads : List Nat), heads.Nodup → (∀ h ∈ heads, SLoop s h x) → heads.length ≤ kmax
  | 0, p, final, _, _, hbound, hcount, _ => fun x heads hnd hall =>
    Nat.le_trans (hcount x heads hnd fun h hh => ⟨Nat.zero_le _, hall h hh⟩) (hbound _)
  | t + 1, p, final, hinv, hsize, hbound, hcount, hfold => by
    rw [List.range_succ, List.reverse_append, List.reverse_singleton, List.singleton_append,
      List.foldlM_cons, bind_ok] at hfold
    obtain ⟨p1, h1, hfold⟩ := hfold
    obtain ⟨hinv1, -⟩ := processHead_ok hs hinv h1
    obtain ⟨hsize1, hb1, hmono, hgrow⟩ := processHead_depth hs hkm hinv hsize h1
    refine heads_depth hs hkm t p1 final hinv1 hsize1 (fun r => ?_) (fun x heads hnd hall => ?_)
      hfold
    · rcases hb1 r with h | h
      · rw [h]; exact hbound r
      · exact h
    · have hold := hcount x (heads.erase t) (hnd.erase t) fun h hh => by
        obtain ⟨hne, hmem⟩ := (List.Nodup.mem_erase_iff hnd).1 hh
        obtain ⟨hle, hl⟩ := hall h hmem
        exact ⟨by omega, hl⟩
      by_cases ht : t ∈ heads
      · rw [List.length_erase_of_mem ht] at hold
        have := hgrow x (hall t ht).2
        have := List.length_pos_of_mem ht
        omega
      · rw [List.erase_of_not_mem ht] at hold
        exact Nat.le_trans hold (hmono x)

theorem replicate_zero_getElem!' (n r : Nat) : (Array.replicate n (0 : Nat))[r]! = 0 := by
  by_cases h : r < n <;> simp [h]

/-- **Loop depth on the loop summary**: if the model accepts the control-flow
graph, no node is in the natural loops of more distinct heads than the
configured bound. -/
theorem loopDepth_summary {config : VerifierConfig} {cfg : Cfg}
    (h : verifyReducibility config cfg = .ok ()) {s : LoopSummary}
    (hb : LoopSummary.build cfg = .ok s) (hs : SummaryOk s) {kmax : Nat}
    (hkm : config.maxLoopDepth = some kmax) :
    ∀ x (heads : List Nat), heads.Nodup → (∀ h ∈ heads, SLoop s h x) → heads.length ≤ kmax := by
  unfold verifyReducibility at h
  rw [hb] at h
  simp only [bind_ok] at h
  obtain ⟨s', hs', h⟩ := h
  cases hs'
  obtain ⟨final, hfold, -⟩ := h
  refine heads_depth hs hkm _ _ final (headInv_new hs) (by simp [LoopPartition.new])
    (fun r => by simp [LoopPartition.new, replicate_zero_getElem!']) (fun x heads hnd hall => ?_)
    hfold
  cases heads with
  | nil => exact Nat.zero_le _
  | cons h _ =>
    obtain ⟨hle, hl⟩ := hall h (List.mem_cons_self ..)
    have := SLoop.head_lt hs hl
    omega

/-! ## On instructions

The control-flow facts below repeat lemmas of `Proofs.Acceptance` and
`Proofs.Reducible` (under `ld_` names, private to this file), so that this
file depends only on the block structure and the loop nesting. -/

section Copies

private theorem ld_not_unconditional {i : Bytecode}
    (h : i.control = .fallThrough ∨ ∃ o, i.control = .branch o) :
    isUnconditionalBranch i = false := by
  cases i <;> simp_all [Bytecode.control, isUnconditionalBranch]

private theorem ld_next_lt {code : Array Bytecode} {pc : Nat} {i : Bytecode}
    (hi : code[pc]? = some i)
    (hfall : ∃ last, code.back? = some last ∧ isUnconditionalBranch last = true)
    (hc : i.control = .fallThrough ∨ ∃ o, i.control = .branch o) : pc + 1 < code.size := by
  have hpc := lt_of_getElem? hi
  obtain ⟨last, hlast, hu⟩ := hfall
  by_cases h : pc + 1 = code.size
  · exfalso
    simp only [Array.back?] at hlast
    rw [show code.size - 1 = pc by omega, hi, Option.some.injEq] at hlast
    subst hlast
    rw [ld_not_unconditional hc] at hu
    exact Bool.false_ne_true hu
  · omega

private theorem ld_successors_lt {code : Array Bytecode}
    (hfall : ∃ last, code.back? = some last ∧ isUnconditionalBranch last = true)
    (hbr : ∀ (pc o : Nat) (i : Bytecode), code[pc]? = some i → offset i = some o → o < code.size) :
    ∀ pc, ∀ s ∈ successors code pc, s < code.size := by
  intro pc s hs
  unfold successors at hs
  split at hs
  · simp at hs
  · next i hi =>
    cases hc : i.control with
    | fallThrough =>
      rw [hc] at hs
      simp only [List.mem_singleton] at hs
      subst hs
      exact ld_next_lt hi hfall (Or.inl hc)
    | jump o =>
      rw [hc] at hs
      simp only [List.mem_singleton] at hs
      subst hs
      exact hbr pc _ i hi (control_jump hc).1
    | branch o =>
      rw [hc] at hs
      simp only [List.mem_cons, List.not_mem_nil, or_false] at hs
      rcases hs with rfl | rfl
      · exact hbr pc _ i hi (control_branch hc).1
      · exact ld_next_lt hi hfall (Or.inr ⟨o, hc⟩)
    | exit => rw [hc] at hs; simp at hs

private theorem ld_verifyControlFlow_fallthrough {config : VerifierConfig} {version : Nat}
    {code : CodeUnit} {cfg : Cfg} (h : verifyControlFlow config version code = .ok cfg) :
    ∃ last, code.code.back? = some last ∧ isUnconditionalBranch last = true := by
  unfold verifyControlFlow at h
  by_cases hv : version ≤ 5
  · simp [hv, panicAt, bind, Except.bind] at h
  · simp only [hv, if_false, bind_ok] at h
    obtain ⟨u, hf, -⟩ := h
    unfold verifyFallthrough at hf
    split at hf
    · simp [reject] at hf
    · next last hlast =>
      refine ⟨last, hlast, ?_⟩
      cases hu : isUnconditionalBranch last <;> simp_all [reject]

private theorem ld_verifyControlFlow_parts {config : VerifierConfig} {version : Nat}
    {c : CodeUnit} {cfg : Cfg} (h : verifyControlFlow config version c = .ok cfg) :
    cfg.blocks = buildBlocks c.code ∧ verifyReducibility config cfg = .ok () := by
  refine ⟨verifyControlFlow_blocks h, ?_⟩
  unfold verifyControlFlow at h
  by_cases hv : version ≤ 5
  · simp [hv, panicAt, bind, Except.bind] at h
  · simp only [hv, if_false, bind_ok] at h
    obtain ⟨_, _, cfg', _, u, hr, h⟩ := h
    simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    cases u
    exact hr

private theorem ld_verifyReducibility_build {config : VerifierConfig} {cfg : Cfg}
    (h : verifyReducibility config cfg = .ok ()) : ∃ s, LoopSummary.build cfg = .ok s := by
  unfold verifyReducibility at h
  simp only [bind_ok] at h
  obtain ⟨s, hs, -⟩ := h
  exact ⟨s, hs⟩

private theorem ld_walk_head {succ : Nat → List Nat} {a c : Nat} {p : List Nat}
    (h : Walk succ a c p) : a ∈ p := by
  cases h <;> simp

private theorem ld_walk_preserve {succ : Nat → List Nat} {P : Nat → Prop}
    (step : ∀ a b, P a → b ∈ succ a → P b) :
    ∀ {a c : Nat} {p : List Nat}, Walk succ a c p → P a → P c
  | _, _, _, .nil _, h => h
  | _, _, _, .cons hab w, h => ld_walk_preserve step w (step _ _ h hab)

private theorem ld_walk_avoid {succ : Nat → List Nat} {P : Nat → Prop} {x : Nat}
    (step : ∀ a b, P a → b ∈ succ a → b ≠ x → P b) :
    ∀ {a c : Nat} {p : List Nat}, Walk succ a c p → x ∉ p → P a → P c
  | _, _, _, .nil _, _, h => h
  | _, _, _, .cons hab w, hx, h =>
    ld_walk_avoid step w (fun h' => hx (List.mem_cons_of_mem _ h'))
      (step _ _ h hab fun hb => hx (List.mem_cons_of_mem _ (hb ▸ ld_walk_head w)))

private theorem ld_rwalk_snoc {s : LoopSummary} :
    ∀ {a b : Nat} {p : List Nat}, RWalk s a b p → ∀ {c : Nat}, Recorded s b c →
      RWalk s a c (p ++ [c])
  | _, _, _, .nil _, _, hc => RWalk.cons hc (RWalk.nil _)
  | _, _, _, .cons hab w, _, hc => RWalk.cons hab (ld_rwalk_snoc w hc)

variable {code : Array Bytecode}

private theorem ld_block_eq {b b' : BasicBlock} (hb : BlockOk code b) (hb' : BlockOk code b')
    {pc : Nat} (h1 : b.start ≤ pc) (h2 : pc ≤ b.exit) (h1' : b'.start ≤ pc) (h2' : pc ≤ b'.exit) :
    b = b' := by
  have hs : b.start = b'.start := by
    rcases Nat.lt_trichotomy b.start b'.start with h | h | h
    · exact absurd hb'.start_mem (hb.inner _ h (by omega))
    · exact h
    · exact absurd hb.start_mem (hb'.inner _ h (by omega))
  have he : b.exit = b'.exit := by
    rcases Nat.lt_trichotomy b.exit b'.exit with h | h | h
    · rcases (isEndOfBlock_iff code b.exit).1 hb.exit_end with h' | h'
      · have := hb'.exit_lt; omega
      · exact absurd h' (hb'.inner _ (by omega) (by omega))
    · exact h
    · rcases (isEndOfBlock_iff code b'.exit).1 hb'.exit_end with h' | h'
      · have := hb.exit_lt; omega
      · exact absurd h' (hb.inner _ (by omega) (by omega))
  have hsucc := hb.successors
  have hsucc' := hb'.successors
  cases b
  cases b'
  simp only at hs he hsucc hsucc'
  subst hs he
  rw [hsucc, hsucc']

private theorem ld_blockId_start {c : Nat} (hc : c < code.size) (hid : c ∈ blockIds code) :
    ∃ b ∈ buildBlocks code, b.start = c := by
  obtain ⟨b, hb, h1, h2⟩ := (buildBlocks_spec code).2 c hc
  refine ⟨b, hb, ?_⟩
  rcases Nat.lt_or_ge b.start c with h | h
  · exact absurd hid (((buildBlocks_spec code).1 b hb).inner c h h2)
  · omega

private theorem ld_blockSuccessors_eq {b : BasicBlock} (hb : b ∈ buildBlocks code) :
    blockSuccessors (buildBlocks code) b.start = b.successors := by
  unfold blockSuccessors
  cases hf : (buildBlocks code).find? (·.start == b.start) with
  | none =>
    rw [List.find?_eq_none] at hf
    exact absurd (by simp) (hf b hb)
  | some bb =>
    have hbb := List.mem_of_find?_eq_some hf
    have hst := List.find?_some hf
    simp only [beq_iff_eq] at hst
    have hok := (buildBlocks_spec code).1
    have := ld_block_eq (hok bb hbb) (hok b hb) (pc := b.start) (by omega)
      (by have := (hok bb hbb).start_le_exit; omega) (Nat.le_refl _) (hok b hb).start_le_exit
    rw [this]

private theorem ld_unconditional_of_control {i : Bytecode}
    (h : i.control = .exit ∨ ∃ o, i.control = .jump o) : isUnconditionalBranch i = true := by
  rcases h with h | ⟨o, h⟩ <;> cases i <;> simp_all [Bytecode.control, isUnconditionalBranch]

variable (hfall : ∃ last, code.back? = some last ∧ isUnconditionalBranch last = true)
  (hbr : ∀ (pc o : Nat) (i : Bytecode), code[pc]? = some i → offset i = some o → o < code.size)

include hfall in
private theorem ld_getSuccessors_mem {pc c : Nat} :
    c ∈ getSuccessors pc code ↔ c ∈ successors code pc := by
  cases hi : code[pc]? with
  | none => simp [getSuccessors, successors, hi]
  | some i =>
    cases hc : i.control with
    | fallThrough =>
      obtain ⟨ho, -⟩ := control_fallThrough hc
      have hlt := ld_next_lt hi hfall (Or.inl hc)
      have hu := ld_not_unconditional (Or.inl hc)
      simp [getSuccessors, successors, hi, hc, ho, hu, Nat.not_le.2 hlt]
    | jump o =>
      obtain ⟨ho, -⟩ := control_jump hc
      have hu := ld_unconditional_of_control (Or.inr ⟨o, hc⟩)
      simp [getSuccessors, successors, hi, hc, ho, hu]
    | branch o =>
      obtain ⟨ho, -⟩ := control_branch hc
      have hlt := ld_next_lt hi hfall (Or.inr ⟨o, hc⟩)
      have hu := ld_not_unconditional (Or.inr ⟨o, hc⟩)
      by_cases hop : o = pc + 1
      · subst hop
        simp [getSuccessors, successors, hi, hc, ho, hu, Nat.not_le.2 hlt]
      · have hop' : (pc + 1 == o) = false := by simp [Ne.symm hop]
        simp only [getSuccessors, successors, hi, hc, ho, hu, Nat.not_le.2 hlt, if_false,
          Bool.not_false, Bool.true_and, List.contains_cons, List.contains_nil, Bool.or_false,
          hop', if_true, List.singleton_append]
        split <;> simp [or_comm]
    | exit =>
      obtain ⟨ho, -⟩ := control_exit hc
      have hu := ld_unconditional_of_control (Or.inl hc)
      simp [getSuccessors, successors, hi, hc, ho, hu]

private theorem ld_inner_successors_eq {b : BasicBlock} (hb : BlockOk code b) {pc : Nat}
    (h1 : b.start ≤ pc) (h2 : pc < b.exit) : successors code pc = [pc + 1] := by
  have hpc : pc < code.size := Nat.lt_trans h2 hb.exit_lt
  obtain ⟨i, hi⟩ : ∃ i, code[pc]? = some i := ⟨code[pc], Array.getElem?_eq_getElem hpc⟩
  have hend := fun hbr => ends_at_exit hb h1 (Nat.le_of_lt h2) (branch_ends hi hbr)
  unfold successors
  rw [hi]
  cases hc : i.control with
  | fallThrough => simp [hc]
  | jump o => exact absurd (hend (control_jump hc).2) (Nat.ne_of_lt h2)
  | branch o => exact absurd (hend (control_branch hc).2) (Nat.ne_of_lt h2)
  | exit => exact absurd (hend (control_exit hc).2) (Nat.ne_of_lt h2)

include hfall hbr in
private theorem ld_exit_successor {b : BasicBlock} (hb : BlockOk code b) {s : Nat}
    (hs : s ∈ successors code b.exit) : ∃ c ∈ buildBlocks code, c.start = s := by
  have hslt := ld_successors_lt hfall hbr b.exit s hs
  refine ld_blockId_start hslt ?_
  obtain ⟨i, hi⟩ : ∃ i, code[b.exit]? = some i :=
    ⟨code[b.exit]'hb.exit_lt, Array.getElem?_eq_getElem hb.exit_lt⟩
  have hnext : b.exit + 1 < code.size → b.exit + 1 ∈ blockIds code := fun hlt => by
    rcases (isEndOfBlock_iff code b.exit).1 hb.exit_end with h | h
    · omega
    · exact h
  cases hc : i.control with
  | fallThrough =>
    simp only [successors, hi, hc, List.mem_singleton] at hs
    subst hs; exact hnext hslt
  | jump o =>
    simp only [successors, hi, hc, List.mem_singleton] at hs
    subst hs; exact offset_mem hi (control_jump hc).1
  | branch o =>
    simp only [successors, hi, hc, List.mem_cons, List.not_mem_nil, or_false] at hs
    rcases hs with rfl | rfl
    · exact offset_mem hi (control_branch hc).1
    · exact hnext hslt
  | exit => simp [successors, hi, hc] at hs

/-- The only edge into an instruction strictly inside a block comes from the
instruction before it. -/
private theorem ld_pred_inner {b : BasicBlock} (hb : BlockOk code b) {q pc : Nat}
    (h1 : b.start < pc) (h2 : pc ≤ b.exit) (hq : pc ∈ successors code q) : q + 1 = pc := by
  have hnot : pc ∉ blockIds code := hb.inner pc h1 h2
  unfold successors at hq
  split at hq
  · simp at hq
  · next i hi =>
    cases hc : i.control with
    | fallThrough =>
      rw [hc] at hq; simp only [List.mem_singleton] at hq; exact hq.symm
    | jump o =>
      rw [hc] at hq; simp only [List.mem_singleton] at hq; subst hq
      exact absurd (offset_mem hi (control_jump hc).1) hnot
    | branch o =>
      rw [hc] at hq
      simp only [List.mem_cons, List.not_mem_nil, or_false] at hq
      rcases hq with rfl | rfl
      · exact absurd (offset_mem hi (control_branch hc).1) hnot
      · rfl
    | exit => rw [hc] at hq; simp at hq

end Copies

/-! ### Walks -/

section Walks

variable {succ : Nat → List Nat}

theorem walk_head_eq : ∀ {x y z : Nat} {W : List Nat}, Walk succ x y (z :: W) → x = z
  | _, _, _, _, .nil _ => rfl
  | _, _, _, _, .cons _ _ => rfl

theorem walk_mem_last : ∀ {x y : Nat} {W : List Nat}, Walk succ x y W → y ∈ W
  | _, _, _, .nil _ => by simp
  | _, _, _, .cons _ w => List.mem_cons_of_mem _ (walk_mem_last w)

/-- The suffix of a walk from any instruction on it. -/
theorem walk_from : ∀ {x y : Nat} {W : List Nat}, Walk succ x y W → ∀ {z : Nat}, z ∈ W →
    ∃ W', Walk succ z y W' ∧ W' <:+ W
  | _, _, _, .nil n, z, hz => by
    simp only [List.mem_singleton] at hz
    subst hz
    exact ⟨[z], .nil z, List.suffix_refl _⟩
  | _, _, _, @Walk.cons _ a b c p hab w, z, hz => by
    rcases List.mem_cons.1 hz with rfl | hz
    · exact ⟨_, .cons hab w, List.suffix_refl _⟩
    · obtain ⟨W', hw, hs⟩ := walk_from w hz
      exact ⟨W', hw, hs.trans (List.suffix_cons a p)⟩

/-- Every walk can be shortened to one without repeated instructions. -/
theorem walk_nodup : ∀ {x y : Nat} {W : List Nat}, Walk succ x y W →
    ∃ W', Walk succ x y W' ∧ W'.Nodup
  | _, _, _, .nil n => ⟨[n], .nil n, by simp⟩
  | _, _, _, @Walk.cons _ a b c p hab w => by
    obtain ⟨W', hw, hnd⟩ := walk_nodup w
    by_cases ha : a ∈ W'
    · obtain ⟨W'', hw'', hs⟩ := walk_from hw ha
      exact ⟨W'', hw'', List.Nodup.sublist hs.sublist hnd⟩
    · exact ⟨a :: W', .cons hab hw, List.nodup_cons.2 ⟨ha, hnd⟩⟩

/-- The prefix of a walk up to an instruction on it. -/
theorem walk_prefix {z : Nat} : ∀ {x y : Nat} {W : List Nat}, Walk succ x y W →
    ∀ (W1 W2 : List Nat), W = W1 ++ z :: W2 → Walk succ x z (W1 ++ [z])
  | _, _, _, .nil n, W1, W2, hW => by
    cases W1 with
    | nil =>
      simp only [List.nil_append, List.cons.injEq] at hW
      obtain ⟨rfl, -⟩ := hW
      exact .nil _
    | cons a W1 => simp at hW
  | _, _, _, @Walk.cons _ a b c p hab w, W1, W2, hW => by
    cases W1 with
    | nil =>
      simp only [List.nil_append, List.cons.injEq] at hW
      obtain ⟨rfl, -⟩ := hW
      exact .nil _
    | cons a' W1 =>
      simp only [List.cons_append, List.cons.injEq] at hW
      obtain ⟨rfl, hp⟩ := hW
      exact .cons hab (walk_prefix w W1 W2 hp)

/-- Consecutive instructions of a walk are joined by an edge. -/
theorem walk_pair {q q' : Nat} : ∀ {x y : Nat} {W : List Nat}, Walk succ x y W →
    ∀ (W1 W2 : List Nat), W = W1 ++ q :: q' :: W2 → q' ∈ succ q
  | _, _, _, .nil n, W1, W2, hW => by
    cases W1 with
    | nil => simp at hW
    | cons a W1 => simp at hW
  | _, _, _, @Walk.cons _ a b c p hab w, W1, W2, hW => by
    cases W1 with
    | nil =>
      simp only [List.nil_append, List.cons.injEq] at hW
      obtain ⟨rfl, rfl⟩ := hW
      rw [← walk_head_eq w]
      exact hab
    | cons a' W1 =>
      simp only [List.cons_append, List.cons.injEq] at hW
      exact walk_pair w W1 W2 hW.2

end Walks

/-! ### The instructions of a code unit and the loop summary -/

/-- Instruction `pc` lies in block `B` of the code, which the loop summary
numbers `kb`. -/
def InBlockAt (code : Array Bytecode) (node : Nat → Option Nat) (pc : Nat) (B : BasicBlock)
    (kb : Nat) : Prop :=
  B ∈ buildBlocks code ∧ B.start ≤ pc ∧ pc ≤ B.exit ∧ node B.start = some kb

/-- What the lifting needs of a code unit whose control flow the model
accepts, with the loop summary's numbering `node` of the blocks and its
finishing order `μ`. -/
structure InstrCtx (code : Array Bytecode) (s : LoopSummary) (node : Nat → Option Nat)
    (μ : Nat → Nat) : Prop where
  size : 0 < code.size
  fall : ∃ last, code.back? = some last ∧ isUnconditionalBranch last = true
  br : ∀ (pc o : Nat) (i : Bytecode), code[pc]? = some i → offset i = some o → o < code.size
  node0 : node 0 = some 0
  inj : ∀ b b' k, node b = some k → node b' = some k → b = b'
  edge : ∀ B ∈ buildBlocks code, ∀ kB, node B.start = some kB → ∀ c ∈ successors code B.exit,
    ∃ kc, node c = some kc ∧ Recorded s kB kc
  sok : SummaryOk s
  mu : ∀ v x, x ∈ s.predEdges v → μ v < μ x
  dom : ∀ h u, u ∈ s.backEdges h → ∀ path, RWalk s 0 u path → h ∈ path

namespace InstrCtx

variable {code : Array Bytecode} {s : LoopSummary} {node : Nat → Option Nat} {μ : Nat → Nat}
  (hc : InstrCtx code s node μ)

include hc

omit hc in
/-- An instruction lies in one block. -/
theorem unique {pc : Nat} {A B : BasicBlock} {ka kb : Nat} (hA : InBlockAt code node pc A ka)
    (hB : InBlockAt code node pc B kb) : A = B ∧ ka = kb := by
  have hok := (buildBlocks_spec code).1
  have := ld_block_eq (hok A hA.1) (hok B hB.1) hA.2.1 hA.2.2.1 hB.2.1 hB.2.2.1
  subst this
  have h1 := hA.2.2.2
  have h2 := hB.2.2.2
  rw [h1, Option.some.injEq] at h2
  exact ⟨rfl, h2⟩

/-- Blocks the summary numbers alike are the same block. -/
theorem block_of_node {A B : BasicBlock} {k : Nat} (hA : A ∈ buildBlocks code)
    (hB : B ∈ buildBlocks code) (hkA : node A.start = some k) (hkB : node B.start = some k) :
    A = B := by
  have hs := hc.inj _ _ _ hkA hkB
  have hok := (buildBlocks_spec code).1
  exact ld_block_eq (hok A hA) (hok B hB) (pc := A.start) (Nat.le_refl _)
    (hok A hA).start_le_exit (by omega) (by have := (hok B hB).start_le_exit; omega)

/-- One edge: inside a block it falls through; from a block's last
instruction it enters the first of a block, along a recorded edge. -/
theorem step {a b : Nat} {A : BasicBlock} {ka : Nat} (hA : InBlockAt code node a A ka)
    (hb : b ∈ successors code a) :
    ∃ B kb, InBlockAt code node b B kb ∧
      ((B = A ∧ kb = ka ∧ a < A.exit ∧ b = a + 1) ∨
        (a = A.exit ∧ b = B.start ∧ Recorded s ka kb)) := by
  have hok := (buildBlocks_spec code).1
  obtain ⟨hAm, h1, h2, hka⟩ := hA
  rcases Nat.lt_or_ge a A.exit with hlt | hge
  · rw [ld_inner_successors_eq (hok A hAm) h1 hlt, List.mem_singleton] at hb
    subst hb
    exact ⟨A, ka, ⟨hAm, by omega, hlt, hka⟩, Or.inl ⟨rfl, rfl, hlt, rfl⟩⟩
  · have hae : a = A.exit := by omega
    subst hae
    obtain ⟨C, hC, rfl⟩ := ld_exit_successor hc.fall hc.br (hok A hAm) hb
    obtain ⟨kc, hkc, hrec⟩ := hc.edge A hAm ka hka C.start hb
    exact ⟨C, kc, ⟨hC, Nat.le_refl _, (hok C hC).start_le_exit, hkc⟩,
      Or.inr ⟨rfl, rfl, hrec⟩⟩

/-- The entry lies in the block the summary numbers 0. -/
theorem entry : ∃ B, InBlockAt code node 0 B 0 := by
  obtain ⟨B0, hB0, h1, h2⟩ := (buildBlocks_spec code).2 0 hc.size
  have h0 : B0.start = 0 := by omega
  exact ⟨B0, hB0, by omega, h2, by rw [h0]; exact hc.node0⟩

/-- A reachable instruction lies in a block the summary numbers. -/
theorem reach {pc : Nat} (h : EntryReaches code pc) : ∃ B kb, InBlockAt code node pc B kb := by
  obtain ⟨p, hw⟩ := h
  obtain ⟨B0, hB0⟩ := hc.entry
  refine ld_walk_preserve (P := fun pc => ∃ B kb, InBlockAt code node pc B kb)
    (fun a b ⟨A, ka, hA⟩ hb => ?_) hw ⟨B0, 0, hB0⟩
  obtain ⟨B, kb, hB, -⟩ := hc.step hA hb
  exact ⟨B, kb, hB⟩

/-- A back edge of the summary enters a dominator of its source. -/
theorem back_dom {B C : BasicBlock} {kB kC : Nat} (hB : B ∈ buildBlocks code)
    (hkB : node B.start = some kB) (hC : C ∈ buildBlocks code) (hkC : node C.start = some kC)
    (hbk : kB ∈ s.backEdges kC) : Dominates (successors code) C.start B.exit := by
  have hok := (buildBlocks_spec code).1
  intro p hw
  refine Classical.byContradiction fun hp => ?_
  let P (pc : Nat) : Prop := ∃ D kD, InBlockAt code node pc D kD ∧
    ∃ path, RWalk s 0 kD path ∧ kC ∉ path
  have hP : ∀ a b, P a → b ∈ successors code a → b ≠ C.start → P b := by
    rintro a b ⟨D, kD, hD, path, hpath, hkp⟩ hb hbC
    obtain ⟨E, kE, hE, hcase⟩ := hc.step hD hb
    rcases hcase with ⟨rfl, rfl, -, -⟩ | ⟨-, hbE, hrec⟩
    · exact ⟨E, kE, hE, path, hpath, hkp⟩
    · refine ⟨E, kE, hE, _, ld_rwalk_snoc hpath hrec, fun hm => ?_⟩
      rcases List.mem_append.1 hm with hm | hm
      · exact hkp hm
      · simp only [List.mem_singleton] at hm
        subst hm
        exact hbC (by rw [hbE, hc.block_of_node hE.1 hC hE.2.2.2 hkC])
  obtain ⟨B0, hB0⟩ := hc.entry
  have hC0 : kC ≠ 0 := by
    intro h0'
    subst h0'
    have := hc.inj _ _ _ hc.node0 hkC
    rw [← this] at hp
    exact hp (ld_walk_head hw)
  obtain ⟨D, kD, hD, path, hpath, hkp⟩ := ld_walk_avoid hP hw hp
    ⟨B0, 0, hB0, [0], RWalk.nil 0, by simpa using hC0⟩
  obtain ⟨rfl, rfl⟩ := InstrCtx.unique hD ⟨hB, (hok B hB).start_le_exit, Nat.le_refl _, hkB⟩
  exact hkp (hc.dom kC _ hbk path hpath)

/-- Along a walk without repeated instructions from the entry, the finishing
order of the instructions' blocks never increases: every edge between blocks
on it is recorded as another incoming edge. -/
theorem ld_mono : ∀ {z a : Nat} {W2 : List Nat}, Walk (successors code) z a W2 →
    ∀ (W1 : List Nat), Walk (successors code) 0 a (W1 ++ W2) → (W1 ++ W2).Nodup →
    ∀ {Z A : BasicBlock} {kz ka : Nat}, InBlockAt code node z Z kz →
      InBlockAt code node a A ka → μ ka ≤ μ kz
  | _, _, _, .nil n, W1, _, _, Z, A, kz, ka, hZ, hA => by
    obtain ⟨-, rfl⟩ := InstrCtx.unique hZ hA
    exact Nat.le_refl _
  | _, _, _, @Walk.cons _ z z' a p hzz w, W1, hW, hnd, Z, A, kz, ka, hZ, hA => by
    obtain ⟨Z', kz', hZ', hcase⟩ := hc.step hZ hzz
    have hW' : Walk (successors code) 0 a ((W1 ++ [z]) ++ p) := by simpa using hW
    have hnd' : ((W1 ++ [z]) ++ p).Nodup := by simpa using hnd
    have ih := ld_mono w (W1 ++ [z]) hW' hnd' hZ' hA
    rcases hcase with ⟨-, rfl, -, -⟩ | ⟨hze, hz'e, hrec⟩
    · exact ih
    · rcases hrec with hpred | hbk
      · exact Nat.le_trans ih (Nat.le_of_lt (hc.mu _ _ hpred))
      · exfalso
        have hdom := hc.back_dom hZ.1 hZ.2.2.2 hZ'.1 hZ'.2.2.2 hbk
        rw [← hz'e, ← hze] at hdom
        have hpre := walk_prefix hW W1 p rfl
        have hz'pre := hdom _ hpre
        have hp : z' ∈ p := ld_walk_head w
        exact (List.nodup_append.1 hnd').2.2 z' hz'pre z' hp rfl

/-- **A back edge of the specification is a back edge of the summary**: it
joins the last instruction of a block to the first of a block, which the
summary records as a back edge. -/
theorem classify {a h : Nat} (hb : BackEdge code a h) :
    ∃ A H ka kh, InBlockAt code node a A ka ∧ InBlockAt code node h H kh ∧ a = A.exit ∧
      h = H.start ∧ ka ∈ s.backEdges kh := by
  obtain ⟨hha, ⟨W, hW⟩, hdom⟩ := hb
  obtain ⟨W0, hW0, hnd⟩ := walk_nodup hW
  have hhW : h ∈ W0 := hdom W0 hW0
  obtain ⟨A, ka, hA⟩ := hc.reach ⟨W, hW⟩
  obtain ⟨H, kh, hH, hcase⟩ := hc.step hA hha
  -- The suffix of the walk from `h` to `a`.
  obtain ⟨W', hW', hsuf⟩ := walk_from hW0 hhW
  obtain ⟨U, rfl⟩ := hsuf
  rcases hcase with ⟨rfl, rfl, hlt, rfl⟩ | ⟨hae, hhs, hrec⟩
  · -- Inside a block: the only edge into `a + 1` is from `a`, which then
    -- occurs both before `a + 1` and at the end of the walk.
    exfalso
    have hok := (buildBlocks_spec code).1
    have hin1 : H.start < a + 1 := by have := hA.2.1; omega
    have hin2 : a + 1 ≤ H.exit := hlt
    obtain ⟨W'', rfl⟩ : ∃ W'', W' = (a + 1) :: W'' := by
      cases hW' with
      | cons _ _ => exact ⟨_, rfl⟩
    have hU : U ≠ [] := by
      rintro rfl
      have := walk_head_eq hW0
      omega
    obtain ⟨U', q, rfl⟩ : ∃ U' q, U = U' ++ [q] := by
      rcases List.eq_nil_or_concat U with h | ⟨U', q, h⟩
      · exact absurd h hU
      · exact ⟨U', q, by simpa using h⟩
    have hq : a + 1 ∈ successors code q :=
      walk_pair hW0 U' W'' (by simp)
    have hqa : q = a := by have := ld_pred_inner (hok H hA.1) hin1 hin2 hq; omega
    subst hqa
    have hlast : q ∈ (q + 1) :: W'' := walk_mem_last hW'
    have hne : q ∈ W'' := by
      rcases List.mem_cons.1 hlast with h | h
      · omega
      · exact h
    exact (List.nodup_append.1 hnd).2.2 q (by simp) q (List.mem_cons_of_mem _ hne) rfl
  · refine ⟨A, H, ka, kh, hA, hH, hae, hhs, ?_⟩
    rcases hrec with hpred | hbk
    · exfalso
      have h1 := hc.mu _ _ hpred
      have h2 := hc.ld_mono hW' U hW0 hnd hH hA
      omega
    · exact hbk

/-- A walk that avoids a block's first instruction, from outside the block,
stays outside it, and is a walk of the summary that avoids the block. -/
theorem lift_walk {H : BasicBlock} {kh : Nat} (hH : H ∈ buildBlocks code)
    (hkh : node H.start = some kh) :
    ∀ {y a : Nat} {P : List Nat}, Walk (successors code) y a P → H.start ∉ P →
      ∀ {Y A : BasicBlock} {ky ka : Nat}, InBlockAt code node y Y ky → Y ≠ H →
        InBlockAt code node a A ka → ∃ path, RWalk s ky ka path ∧ kh ∉ path
  | _, _, _, .nil n, _, Y, A, ky, ka, hY, hYH, hA => by
    obtain ⟨rfl, rfl⟩ := InstrCtx.unique hY hA
    refine ⟨[ky], RWalk.nil ky, fun hm => ?_⟩
    simp only [List.mem_singleton] at hm
    subst hm
    exact hYH (hc.block_of_node hY.1 hH hY.2.2.2 hkh)
  | _, _, _, @Walk.cons _ y y' a p hyy w, hP, Y, A, ky, ka, hY, hYH, hA => by
    have hP' : H.start ∉ p := fun h => hP (List.mem_cons_of_mem _ h)
    obtain ⟨Y', ky', hY', hcase⟩ := hc.step hY hyy
    rcases hcase with ⟨rfl, rfl, -, -⟩ | ⟨-, hy', hrec⟩
    · exact lift_walk hH hkh w hP' hY' hYH hA
    · have hY'H : Y' ≠ H := by
        rintro rfl
        exact hP' (hy' ▸ ld_walk_head w)
      obtain ⟨path, hpath, hkp⟩ := lift_walk hH hkh w hP' hY' hY'H hA
      refine ⟨ky :: path, RWalk.cons hrec hpath, fun hm => ?_⟩
      rcases List.mem_cons.1 hm with hm | hm
      · subst hm
        exact hYH (hc.block_of_node hY.1 hH hY.2.2.2 hkh)
      · exact hkp hm

/-- **A natural loop of the specification is a natural loop of the summary**:
its head starts a block whose node is a head of the summary, and whose loop
holds the node of the block of every instruction of the loop. -/
theorem lift {h pc : Nat} (hl : InLoop code h pc) {X : BasicBlock} {kx : Nat}
    (hx : InBlockAt code node pc X kx) :
    ∃ H kh, InBlockAt code node h H kh ∧ h = H.start ∧ SLoop s kh kx := by
  obtain ⟨a, hbe, hpc⟩ := hl
  obtain ⟨A, H, ka, kh, hA, hH, rfl, rfl, hback⟩ := hc.classify hbe
  refine ⟨H, kh, hH, rfl, ka, hback, ?_⟩
  rcases hpc with heq | ⟨P, hP, hhP⟩
  · subst heq
    exact Or.inl (InstrCtx.unique hx hH).2
  · by_cases hXH : X = H
    · subst hXH
      exact Or.inl (InstrCtx.unique hx ⟨hH.1, hx.2.1, hx.2.2.1, hH.2.2.2⟩).2
    · exact Or.inr (hc.lift_walk hH.1 hH.2.2.2 hP hhP hx hXH hA)

end InstrCtx

theorem nodup_map_of_inj {f : Nat → Nat} :
    ∀ {l : List Nat}, l.Nodup → (∀ x ∈ l, ∀ y ∈ l, f x = f y → x = y) → (l.map f).Nodup
  | [], _, _ => List.nodup_nil
  | a :: l, hnd, hinj => by
    have hnd' := List.nodup_cons.1 hnd
    refine List.nodup_cons.2 ⟨fun hm => ?_, nodup_map_of_inj hnd'.2
      fun x hx y hy => hinj x (List.mem_cons_of_mem _ hx) y (List.mem_cons_of_mem _ hy)⟩
    obtain ⟨b, hb, hfb⟩ := List.mem_map.1 hm
    have := hinj b (List.mem_cons_of_mem _ hb) a (List.mem_cons_self ..) hfb
    subst this
    exact hnd'.1 hb

/-- **L11, loop depth**: if the model accepts a code unit's control flow,
whose branch targets are in the code, no reachable instruction lies in the
natural loops of more distinct heads than the configured loop depth. The
loop summary's facts are those of `summary_facts` (`Proofs/LoopSummary.lean`),
taken as a hypothesis. -/
theorem loopDepthAtMost_of {config : VerifierConfig} {version : Nat} {code : CodeUnit}
    {cfg : Cfg} {k : Nat} (hcf : verifyControlFlow config version code = .ok cfg)
    (hk : config.maxLoopDepth = some k)
    (hbr : ∀ (pc o : Nat) (i : Bytecode), code.code[pc]? = some i → offset i = some o →
      o < code.code.size)
    (hsum : ∀ {s : LoopSummary}, LoopSummary.build cfg = .ok s → ∀ {S : List Nat},
      S.length ≤ cfg.blocks.length → 0 ∈ S → (∀ b ∈ S, ∀ c ∈ cfg.successors b, c ∈ S) →
      ∃ (node : Nat → Option Nat) (μ : Nat → Nat),
        node 0 = some 0 ∧
        (∀ b b' k, node b = some k → node b' = some k → b = b') ∧
        (∀ b k, node b = some k → ∀ c ∈ cfg.successors b,
          ∃ kc, node c = some kc ∧ Recorded s k kc) ∧
        SummaryOk s ∧
        (∀ v x, x ∈ s.predEdges v → μ v < μ x)) :
    LoopDepthAtMost code.code k := by
  have hfall := ld_verifyControlFlow_fallthrough hcf
  obtain ⟨hblocks, hred⟩ := ld_verifyControlFlow_parts hcf
  obtain ⟨s, hbuild⟩ := ld_verifyReducibility_build hred
  have hok := (buildBlocks_spec code.code).1
  have hsize : 0 < code.code.size := by
    obtain ⟨last, hlast, -⟩ := hfall
    rcases Nat.eq_zero_or_pos code.code.size with h0 | h0
    · simp [Array.back?, h0] at hlast
    · exact h0
  have hsucc : ∀ b ∈ buildBlocks code.code, cfg.successors b.start = b.successors := by
    intro b hb; simp only [Cfg.successors, hblocks]; exact ld_blockSuccessors_eq hb
  have hS : ∀ b ∈ cfg.blocks.map (·.start), ∀ x ∈ cfg.successors b,
      x ∈ cfg.blocks.map (·.start) := by
    intro b hb x hx
    rw [hblocks] at hb ⊢
    obtain ⟨B, hB, rfl⟩ := List.mem_map.1 hb
    rw [hsucc B hB, (hok B hB).successors, ld_getSuccessors_mem hfall] at hx
    obtain ⟨C, hC, hCs⟩ := ld_exit_successor hfall hbr (hok B hB) hx
    exact List.mem_map.2 ⟨C, hC, hCs⟩
  obtain ⟨B0, hB0, hB0s, -⟩ := (buildBlocks_spec code.code).2 0 hsize
  have h0 : 0 ∈ cfg.blocks.map (·.start) := by
    rw [hblocks]; exact List.mem_map.2 ⟨B0, hB0, by omega⟩
  obtain ⟨node, μ, hnode0, hinj, hedge, hsok, hμ⟩ := hsum hbuild (by simp) h0 hS
  have hc : InstrCtx code.code s node μ :=
    { size := hsize
      fall := hfall
      br := hbr
      node0 := hnode0
      inj := hinj
      edge := fun B hB kB hkB c hcs => hedge B.start kB hkB c (by
        rw [hsucc B hB, (hok B hB).successors, ld_getSuccessors_mem hfall]; exact hcs)
      sok := hsok
      mu := hμ
      dom := back_dominated hred hbuild hsok }
  intro pc hpc heads hnd hall
  obtain ⟨X, kx, hx⟩ := hc.reach hpc
  let f : Nat → Nat := fun h => (node h).getD 0
  have hf : ∀ h ∈ heads, node h = some (f h) ∧ SLoop s (f h) kx := by
    intro h hh
    obtain ⟨H, kh, hH, hs, hl⟩ := hc.lift (hall h hh) hx
    have hnh : node h = some kh := by rw [hs]; exact hH.2.2.2
    refine ⟨by simp [f, hnh], ?_⟩
    simp only [f, hnh, Option.getD_some]
    exact hl
  have hnd' : (heads.map f).Nodup := nodup_map_of_inj hnd fun x hx y hy hxy =>
    hc.inj _ _ _ (hf x hx).1 (by rw [hxy]; exact (hf y hy).1)
  have := loopDepth_summary hred hbuild hsok hk kx (heads.map f) hnd' fun h hh => by
    obtain ⟨h', hh', rfl⟩ := List.mem_map.1 hh
    exact (hf h' hh').2
  simpa using this

end MoveBytecode.Proofs
