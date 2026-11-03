-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Scheduler

/-!
# The depth-first traversal

UNTRUSTED (checked by the kernel). `Cfg.build` orders the blocks by an
iterative depth-first search that keeps each block on the stack, below the
successors it pushes, until it finishes. Its result has the properties the
scheduler needs (`DfsFacts`), hence `schedulerSound`.

The invariant (`DfsInv`) describes the stack as segments: each block in
progress, latest first, sits below the successors it pushed that are still
waiting. A waiting entry was unseen when pushed, so the edge to it is not a
back edge; and no block in progress at or below its segment is waiting in
it, so popping a block in progress always finishes the latest one. When a
block finishes, each successor is finished already or a recorded back edge,
so in the finishing order (post-order) every other successor comes earlier.
Every explored block but the entry was pushed by an explored block along an
edge that is not a back edge.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Back edges -/

/-- Whether `heads` records `cur → next` as a back edge, as `Cfg.isBackEdge`
reads it. -/
def backEdge (heads : List (Nat × List Nat)) (cur next : Nat) : Bool :=
  match heads.lookup next with
  | some srcs => srcs.contains cur
  | none => false

theorem isBackEdge_eq (cfg : Cfg) (cur next : Nat) :
    cfg.isBackEdge cur next = backEdge cfg.loopHeads cur next := rfl

theorem find?_key_lookup {S : Type} : ∀ {l : List (Nat × S)} {h : Nat} {e : Nat × S},
    l.find? (·.1 == h) = some e → l.lookup h = some e.2
  | [], _, _, hf => by simp at hf
  | (k, v) :: rest, h, e, hf => by
    by_cases hk : k = h
    · subst hk
      simp only [List.find?_cons, beq_self_eq_true, Option.some.injEq] at hf
      subst hf
      simp
    · have hf' : rest.find? (·.1 == h) = some e := by
        simpa [List.find?_cons, hk] using hf
      rw [lookup_cons_ne _ _ (Ne.symm hk)]
      exact find?_key_lookup hf'

theorem find?_key_none {S : Type} : ∀ {l : List (Nat × S)} {h : Nat},
    l.find? (·.1 == h) = none → l.lookup h = none
  | [], _, _ => rfl
  | (k, v) :: rest, h, hf => by
    by_cases hk : k = h
    · subst hk; simp at hf
    · have hf' : rest.find? (·.1 == h) = none := by
        simpa [List.find?_cons, hk] using hf
      rw [lookup_cons_ne _ _ (Ne.symm hk)]
      exact find?_key_none hf'

theorem backEdge_add (heads : List (Nat × List Nat)) (h s cur next : Nat) :
    backEdge (addBackEdge heads h s) cur next = true ↔
      backEdge heads cur next = true ∨ (cur = s ∧ next = h) := by
  unfold addBackEdge
  split
  · next k srcs hf =>
    have hl := find?_key_lookup hf
    simp only at hl
    have hset : heads.map (fun e => if e.1 == h then (h, if srcs.contains s then srcs else
        srcs ++ [s]) else e) = setPre heads h (if srcs.contains s then srcs else srcs ++ [s]) :=
      rfl
    rw [hset]
    unfold backEdge
    rw [lookup_setPre]
    by_cases hn : next = h
    · subst hn
      simp only [↓reduceIte, hl, Option.map_some]
      by_cases hc : srcs.contains s = true
      · have hmem : s ∈ srcs := by simpa using hc
        simp only [hc, ↓reduceIte, List.contains_iff_mem]
        constructor
        · intro h1; exact Or.inl h1
        · rintro (h1 | ⟨rfl, -⟩)
          · exact h1
          · exact hmem
      · simp only [hc, Bool.false_eq_true, ↓reduceIte, List.contains_iff_mem,
          List.mem_append, List.mem_singleton, and_true] at hc ⊢
    · simp [hn]
  · next hf =>
    have hl := find?_key_none hf
    unfold backEdge
    rw [lookup_append_single]
    by_cases hn : next = h
    · subst hn
      simp [hl, eq_comm]
    · cases hx : heads.lookup next <;> simp [hn]


/-! ## Exploring a block -/

/-- The successor loop of one exploration, given its effect on each successor:
push an unseen one, record a back edge to one in progress, skip a finished
one. -/
theorem foldl_successors (block : Nat) (E : List (Nat × Exploration))
    (f : Traversal → Nat → Traversal)
    (hnone : ∀ t s, t.exploration.lookup s = none → f t s = { t with stack := s :: t.stack })
    (hgray : ∀ t s, t.exploration.lookup s = some .inProgress →
      f t s = { t with loopHeads := addBackEdge t.loopHeads s block })
    (hdone : ∀ t s, t.exploration.lookup s = some .done → f t s = t) :
    ∀ (L : List Nat) (t : Traversal), t.exploration = E →
      (L.foldl f t).exploration = E ∧ (L.foldl f t).postOrder = t.postOrder ∧
      (L.foldl f t).stack = (L.filter (fun s => E.lookup s == none)).reverse ++ t.stack ∧
      ∀ cur next, backEdge (L.foldl f t).loopHeads cur next = true ↔
        backEdge t.loopHeads cur next = true ∨
          (cur = block ∧ next ∈ L ∧ E.lookup next = some .inProgress)
  | [], t, hE => by simp [hE]
  | s :: L, t, hE => by
    simp only [List.foldl_cons]
    cases hs : E.lookup s with
    | none =>
      have := foldl_successors block E f hnone hgray hdone L (f t s) (by rw [hnone t s (hE ▸ hs)]; exact hE)
      rw [hnone t s (hE ▸ hs)] at this ⊢
      obtain ⟨h1, h2, h3, h4⟩ := this
      refine ⟨h1, h2, by rw [h3]; simp [hs], fun cur next => ?_⟩
      rw [h4]
      constructor
      · rintro (h | ⟨rfl, hn, hg⟩)
        · exact Or.inl h
        · exact Or.inr ⟨rfl, List.mem_cons_of_mem _ hn, hg⟩
      · rintro (h | ⟨rfl, hn, hg⟩)
        · exact Or.inl h
        · rcases List.mem_cons.1 hn with rfl | hn
          · rw [hs] at hg; cases hg
          · exact Or.inr ⟨rfl, hn, hg⟩
    | some ex =>
      cases ex with
      | inProgress =>
        have := foldl_successors block E f hnone hgray hdone L (f t s)
          (by rw [hgray t s (hE ▸ hs)]; exact hE)
        rw [hgray t s (hE ▸ hs)] at this ⊢
        obtain ⟨h1, h2, h3, h4⟩ := this
        refine ⟨h1, h2, by rw [h3]; simp [hs], fun cur next => ?_⟩
        rw [h4, backEdge_add]
        constructor
        · rintro ((h | ⟨rfl, rfl⟩) | ⟨rfl, hn, hg⟩)
          · exact Or.inl h
          · exact Or.inr ⟨rfl, List.mem_cons_self .., hs⟩
          · exact Or.inr ⟨rfl, List.mem_cons_of_mem _ hn, hg⟩
        · rintro (h | ⟨rfl, hn, hg⟩)
          · exact Or.inl (Or.inl h)
          · rcases List.mem_cons.1 hn with rfl | hn
            · exact Or.inl (Or.inr ⟨rfl, rfl⟩)
            · exact Or.inr ⟨rfl, hn, hg⟩
      | done =>
        have := foldl_successors block E f hnone hgray hdone L (f t s)
          (by rw [hdone t s (hE ▸ hs)]; exact hE)
        rw [hdone t s (hE ▸ hs)] at this ⊢
        obtain ⟨h1, h2, h3, h4⟩ := this
        refine ⟨h1, h2, by rw [h3]; simp [hs], fun cur next => ?_⟩
        rw [h4]
        constructor
        · rintro (h | ⟨rfl, hn, hg⟩)
          · exact Or.inl h
          · exact Or.inr ⟨rfl, List.mem_cons_of_mem _ hn, hg⟩
        · rintro (h | ⟨rfl, hn, hg⟩)
          · exact Or.inl h
          · rcases List.mem_cons.1 hn with rfl | hn
            · rw [hs] at hg; cases hg
            · exact Or.inr ⟨rfl, hn, hg⟩

/-- Exploring an unseen block: mark it in progress, keep it on the stack
below its unseen successors, and record back edges to the successors in
progress. -/
theorem traversalStep_white {blocks : List BasicBlock} {t : Traversal} {block : Nat}
    {rest : List Nat} (h : t.exploration.lookup block = none) :
    let E := (block, Exploration.inProgress) :: t.exploration
    let t' := traversalStep blocks t block rest
    t'.exploration = E ∧ t'.postOrder = t.postOrder ∧
      t'.stack = ((blockSuccessors blocks block).filter (fun s => E.lookup s == none)).reverse ++
        block :: rest ∧
      ∀ cur next, backEdge t'.loopHeads cur next = true ↔
        backEdge t.loopHeads cur next = true ∨
          (cur = block ∧ next ∈ blockSuccessors blocks block ∧ E.lookup next = some .inProgress) := by
  intro E t'
  simp only [t', traversalStep, h]
  exact foldl_successors block E _ (fun t s hs => by simp [hs]) (fun t s hs => by simp [hs])
    (fun t s hs => by simp [hs]) _ _ rfl


theorem traversalStep_done {blocks : List BasicBlock} {t : Traversal} {block : Nat}
    {rest : List Nat} (h : t.exploration.lookup block = some .done) :
    traversalStep blocks t block rest = { t with stack := rest } := by
  simp [traversalStep, h]

theorem traversalStep_gray {blocks : List BasicBlock} {t : Traversal} {block : Nat}
    {rest : List Nat} (h : t.exploration.lookup block = some .inProgress) :
    traversalStep blocks t block rest =
      { t with stack := rest, postOrder := t.postOrder ++ [block],
               exploration := setPre t.exploration block .done } := by
  simp [traversalStep, h, setPre]

/-! ## The traversal invariant -/

/-- The stack, from the blocks in progress (latest first), each above the
successors it pushed that are still waiting. -/
def flatSegs : List (Nat × List Nat) → List Nat
  | [] => []
  | (g, E) :: rest => E ++ g :: flatSegs rest

/-- Each waiting entry is a successor of the block that pushed it, not a
recorded back edge, and not in progress at or below that block. -/
def Pending (blocks : List BasicBlock) (heads : List (Nat × List Nat)) :
    List (Nat × List Nat) → Prop
  | [] => True
  | (g, E) :: rest =>
    (∀ e ∈ E, e ∈ blockSuccessors blocks g ∧ backEdge heads g e = false ∧
      e ∉ g :: rest.map (·.1)) ∧ Pending blocks heads rest

/-- Each successor of a block in progress is a recorded back edge, finished,
waiting above it, or in progress above it (`above`). -/
def Succs (blocks : List BasicBlock) (heads : List (Nat × List Nat))
    (expl : List (Nat × Exploration)) (above : List Nat) : List (Nat × List Nat) → Prop
  | [] => True
  | (g, E) :: rest =>
    (∀ s ∈ blockSuccessors blocks g, backEdge heads g s = true ∨
      expl.lookup s = some .done ∨ s ∈ E ∨ s ∈ above) ∧
    Succs blocks heads expl (g :: above) rest

structure DfsInv (blocks : List BasicBlock) (t : Traversal) (segs : List (Nat × List Nat))
    (root : List Nat) : Prop where
  stackEq : t.stack = flatSegs segs ++ root
  rootOk : root = [] ∨ (root = [0] ∧ segs = [] ∧ ∀ x, t.exploration.lookup x = none)
  entryOk : root = [] → t.exploration.lookup 0 ≠ none
  gray : ∀ x, t.exploration.lookup x = some .inProgress ↔ x ∈ segs.map (·.1)
  grayNodup : (segs.map (·.1)).Nodup
  pending : Pending blocks t.loopHeads segs
  succs : Succs blocks t.loopHeads t.exploration [] segs
  postNodup : t.postOrder.Nodup
  post : ∀ x, x ∈ t.postOrder ↔ t.exploration.lookup x = some .done
  finished : ∀ b ∈ t.postOrder, ∀ s ∈ blockSuccessors blocks b,
    backEdge t.loopHeads b s = true ∨ (s ∈ t.postOrder ∧ t.postOrder.idxOf s < t.postOrder.idxOf b)
  parent : ∀ x, t.exploration.lookup x ≠ none → x ≠ 0 → ∃ y, t.exploration.lookup y ≠ none ∧
    x ∈ blockSuccessors blocks y ∧ backEdge t.loopHeads y x = false
  edges : ∀ cur next, backEdge t.loopHeads cur next = true →
    t.exploration.lookup cur ≠ none ∧ t.exploration.lookup next ≠ none

theorem pending_mono {blocks : List BasicBlock} {heads heads' : List (Nat × List Nat)}
    {block : Nat} (hh : ∀ cur next, backEdge heads' cur next = true →
      backEdge heads cur next = true ∨ cur = block) :
    ∀ segs, block ∉ segs.map (·.1) → Pending blocks heads segs → Pending blocks heads' segs
  | [], _, _ => trivial
  | (g, E) :: rest, hb, ⟨h1, h2⟩ => by
    simp only [List.map_cons, List.mem_cons, not_or] at hb
    refine ⟨fun e he => ?_, pending_mono hh rest hb.2 h2⟩
    obtain ⟨hs, hback, hng⟩ := h1 e he
    refine ⟨hs, ?_, hng⟩
    cases hb' : backEdge heads' g e with
    | false => rfl
    | true =>
      rcases hh g e hb' with h | h
      · rw [hback] at h; cases h
      · exact absurd h.symm hb.1

theorem succs_mono {blocks : List BasicBlock} {heads heads' : List (Nat × List Nat)}
    {expl expl' : List (Nat × Exploration)}
    (hh : ∀ cur next, backEdge heads cur next = true → backEdge heads' cur next = true)
    (hd : ∀ x, expl.lookup x = some .done → expl'.lookup x = some .done) :
    ∀ segs above above', (∀ x ∈ above, x ∈ above') → Succs blocks heads expl above segs →
      Succs blocks heads' expl' above' segs
  | [], _, _, _, _ => trivial
  | (g, E) :: rest, above, above', ha, ⟨h1, h2⟩ => by
    refine ⟨fun s hs => ?_, succs_mono hh hd rest (g :: above) (g :: above') ?_ h2⟩
    · rcases h1 s hs with h | h | h | h
      · exact Or.inl (hh _ _ h)
      · exact Or.inr (Or.inl (hd _ h))
      · exact Or.inr (Or.inr (Or.inl h))
      · exact Or.inr (Or.inr (Or.inr (ha _ h)))
    · intro x hx
      rcases List.mem_cons.1 hx with rfl | hx
      · exact List.mem_cons_self ..
      · exact List.mem_cons_of_mem _ (ha x hx)


theorem succs_drop {blocks : List BasicBlock} {heads : List (Nat × List Nat)}
    {expl expl' : List (Nat × Exploration)}
    (hd : ∀ x, expl.lookup x = some .done → expl'.lookup x = some .done) :
    ∀ segs above above', (∀ x ∈ above, x ∈ above' ∨ expl'.lookup x = some .done) →
      Succs blocks heads expl above segs → Succs blocks heads expl' above' segs
  | [], _, _, _, _ => trivial
  | (g, E) :: rest, above, above', ha, ⟨h1, h2⟩ => by
    refine ⟨fun s hs => ?_, succs_drop hd rest (g :: above) (g :: above') ?_ h2⟩
    · rcases h1 s hs with h | h | h | h
      · exact Or.inl h
      · exact Or.inr (Or.inl (hd _ h))
      · exact Or.inr (Or.inr (Or.inl h))
      · rcases ha s h with h' | h'
        · exact Or.inr (Or.inr (Or.inr h'))
        · exact Or.inr (Or.inl h')
    · intro x hx
      rcases List.mem_cons.1 hx with rfl | hx
      · exact Or.inl (List.mem_cons_self ..)
      · rcases ha x hx with h | h
        · exact Or.inl (List.mem_cons_of_mem _ h)
        · exact Or.inr h

theorem idxOf_append_mem {l : List Nat} {b : Nat} (hb : b ∈ l) (l' : List Nat) :
    (l ++ l').idxOf b = l.idxOf b := by
  induction l with
  | nil => simp at hb
  | cons a t ih =>
    by_cases hab : a = b
    · subst hab; simp [List.idxOf_cons_self]
    · have hbt : b ∈ t := by simpa [Ne.symm hab] using hb
      have hab' : (a == b) = false := by simp [hab]
      simp [List.idxOf_cons, hab', ih hbt]

theorem idxOf_append_last {l : List Nat} {b : Nat} (hb : b ∉ l) : (l ++ [b]).idxOf b = l.length := by
  induction l with
  | nil => simp
  | cons a t ih =>
    have hab : a ≠ b := fun e => hb (e ▸ List.mem_cons_self ..)
    have hbt : b ∉ t := fun h => hb (List.mem_cons_of_mem _ h)
    have hab' : (a == b) = false := by simp [hab]
    simp [List.idxOf_cons, hab', ih hbt]

theorem idxOf_lt {l : List Nat} {b : Nat} (hb : b ∈ l) : l.idxOf b < l.length :=
  List.idxOf_lt_length_iff.2 hb

/-- Finishing the block in progress on top. -/
theorem gray_step {blocks : List BasicBlock} {t : Traversal} {block : Nat} {rest : List Nat}
    {rs : List (Nat × List Nat)} {root : List Nat} (hi : DfsInv blocks t ((block, []) :: rs) root)
    (hstk : t.stack = block :: rest) (hg : t.exploration.lookup block = some .inProgress) :
    DfsInv blocks (traversalStep blocks t block rest) rs root := by
  rw [traversalStep_gray hg]
  have hrest : rest = flatSegs rs ++ root := by
    have := hi.stackEq; rw [hstk] at this; simpa [flatSegs] using this
  have hroot : root = [] := by
    rcases hi.rootOk with h | ⟨_, h, _⟩
    · exact h
    · cases h
  have hnd := hi.grayNodup
  simp only [List.map_cons, List.nodup_cons] at hnd
  have hlk : ∀ x, (setPre t.exploration block Exploration.done).lookup x =
      if x = block then some .done else t.exploration.lookup x := by
    intro x; rw [lookup_setPre]; by_cases hx : x = block <;> simp [hx, hg]
  have hexpl : ∀ x, t.exploration.lookup x ≠ none →
      (setPre t.exploration block Exploration.done).lookup x ≠ none := by
    intro x hx; rw [hlk]; split <;> simp_all
  have hdone : ∀ x, t.exploration.lookup x = some .done →
      (setPre t.exploration block Exploration.done).lookup x = some .done := by
    intro x hx; rw [hlk]; split <;> simp_all
  have hnotpost : block ∉ t.postOrder := by
    intro h; rw [hi.post, hg] at h; cases h
  have hexpl' : ∀ x, (setPre t.exploration block Exploration.done).lookup x ≠ none →
      t.exploration.lookup x ≠ none := by
    intro x hx; rw [hlk] at hx; split at hx
    · next h => subst h; rw [hg]; simp
    · exact hx
  refine ⟨hrest, Or.inl hroot, fun _ => hexpl 0 (hi.entryOk hroot), fun x => ?_, hnd.2,
    hi.pending.2, ?_, ?_, fun x => ?_, fun b hb s hs => ?_, fun x hx hx0 => ?_,
    fun cur next h => ?_⟩
  · rw [hlk]
    by_cases hx : x = block
    · subst hx; simp [hnd.1]
    · simp only [hx, ↓reduceIte, hi.gray, List.map_cons, List.mem_cons, false_or]
  · exact succs_drop hdone rs [block] [] (fun x hx => by
      simp only [List.mem_singleton] at hx; subst hx; right; simp [hlk]) hi.succs.2
  · exact List.nodup_append.2 ⟨hi.postNodup, by simp, by
      intro a ha b hb; simp only [List.mem_singleton] at hb; subst hb
      exact fun e => hnotpost (e ▸ ha)⟩
  · rw [List.mem_append, hi.post, List.mem_singleton, hlk]
    by_cases hx : x = block <;> simp [hx]
  · rcases List.mem_append.1 hb with hb | hb
    · rcases hi.finished b hb s hs with h | ⟨hsp, hlt⟩
      · exact Or.inl h
      · right
        refine ⟨List.mem_append_left _ hsp, ?_⟩
        rw [idxOf_append_mem hsp, idxOf_append_mem hb]
        exact hlt
    · simp only [List.mem_singleton] at hb
      subst hb
      rcases hi.succs.1 s hs with h | h | h | h
      · exact Or.inl h
      · right
        have hsp := (hi.post s).2 h
        refine ⟨List.mem_append_left _ hsp, ?_⟩
        rw [idxOf_append_mem hsp, idxOf_append_last hnotpost]
        exact idxOf_lt hsp
      · simp at h
      · simp at h
  · obtain ⟨y, hy, hxy, hback⟩ := hi.parent x (hexpl' x hx) hx0
    exact ⟨y, hexpl y hy, hxy, hback⟩
  · obtain ⟨h1, h2⟩ := hi.edges cur next h
    exact ⟨hexpl cur h1, hexpl next h2⟩

/-- Dropping a finished block waiting on top. -/
theorem done_step {blocks : List BasicBlock} {t : Traversal} {block : Nat} {rest : List Nat}
    {g : Nat} {E' : List Nat} {rs : List (Nat × List Nat)} {root : List Nat}
    (hi : DfsInv blocks t ((g, block :: E') :: rs) root) (hstk : t.stack = block :: rest)
    (hd : t.exploration.lookup block = some .done) :
    DfsInv blocks (traversalStep blocks t block rest) ((g, E') :: rs) root := by
  rw [traversalStep_done hd]
  have hrest : rest = flatSegs ((g, E') :: rs) ++ root := by
    have := hi.stackEq; rw [hstk] at this; simpa [flatSegs] using this
  obtain ⟨⟨hp1, hp2⟩, hs⟩ := And.intro hi.pending hi.succs
  have hroot : root = [] := by
    rcases hi.rootOk with h | ⟨_, h, _⟩
    · exact h
    · cases h
  refine ⟨hrest, Or.inl hroot, hi.entryOk, hi.gray, hi.grayNodup,
    ⟨fun e he => hp1 e (List.mem_cons_of_mem _ he), hp2⟩, ⟨fun s hs' => ?_, hs.2⟩,
    hi.postNodup, hi.post, hi.finished, hi.parent, hi.edges⟩
  rcases hs.1 s hs' with h | h | h | h
  · exact Or.inl h
  · exact Or.inr (Or.inl h)
  · rcases List.mem_cons.1 h with rfl | h
    · exact Or.inr (Or.inl hd)
    · exact Or.inr (Or.inr (Or.inl h))
  · exact Or.inr (Or.inr (Or.inr h))


section White

variable {blocks : List BasicBlock} {t : Traversal} {block : Nat} {rest : List Nat}

/-- Facts about exploring an unseen block, in the invariant's terms. -/
theorem white_facts (hw : t.exploration.lookup block = none)
    (hedges : ∀ cur next, backEdge t.loopHeads cur next = true →
      t.exploration.lookup cur ≠ none ∧ t.exploration.lookup next ≠ none) :
    let E1 := (block, Exploration.inProgress) :: t.exploration
    let t' := traversalStep blocks t block rest
    let W := ((blockSuccessors blocks block).filter (fun s => E1.lookup s == none)).reverse
    t'.exploration = E1 ∧ t'.postOrder = t.postOrder ∧ t'.stack = W ++ block :: rest ∧
    (∀ x, E1.lookup x = if x = block then some .inProgress else t.exploration.lookup x) ∧
    (∀ cur next, backEdge t'.loopHeads cur next = true →
      backEdge t.loopHeads cur next = true ∨ (cur = block ∧ E1.lookup next = some .inProgress)) ∧
    (∀ cur next, backEdge t.loopHeads cur next = true → backEdge t'.loopHeads cur next = true) ∧
    (∀ s ∈ blockSuccessors blocks block, E1.lookup s = some .inProgress →
      backEdge t'.loopHeads block s = true) ∧
    (∀ e ∈ W, e ∈ blockSuccessors blocks block ∧ backEdge t'.loopHeads block e = false ∧
      E1.lookup e = none) ∧
    (∀ s ∈ blockSuccessors blocks block, E1.lookup s = none → s ∈ W) := by
  intro E1 t' W
  obtain ⟨h1, h2, h3, h4⟩ := traversalStep_white (blocks := blocks) (rest := rest) hw
  have hE1 : ∀ x, E1.lookup x = if x = block then some .inProgress else t.exploration.lookup x := by
    intro x
    by_cases hx : x = block
    · subst hx; simp [E1]
    · simp [E1, lookup_cons_ne _ _ hx, hx]
  refine ⟨h1, h2, h3, hE1, fun cur next h => ?_, fun cur next h => (h4 cur next).2 (Or.inl h),
    fun s hs hg => (h4 block s).2 (Or.inr ⟨rfl, hs, hg⟩), fun e he => ?_, fun s hs hn => ?_⟩
  · rcases (h4 cur next).1 h with h | ⟨rfl, -, hg⟩
    · exact Or.inl h
    · exact Or.inr ⟨rfl, hg⟩
  · simp only [W, List.mem_reverse, List.mem_filter, beq_iff_eq] at he
    obtain ⟨hs, hn⟩ := he
    refine ⟨hs, ?_, hn⟩
    cases hb : backEdge t'.loopHeads block e with
    | false => rfl
    | true =>
      rcases (h4 block e).1 hb with h | ⟨-, -, hg⟩
      · exact absurd hw (hedges block e h).1
      · rw [hn] at hg; cases hg
  · simp [W, List.mem_filter, hs, hn]

end White



/-- Exploring an unseen block waiting on top of a block in progress. -/
theorem white_seg {blocks : List BasicBlock} {t : Traversal} {block : Nat} {rest : List Nat}
    {g : Nat} {E' : List Nat} {rs : List (Nat × List Nat)} {root : List Nat}
    (hi : DfsInv blocks t ((g, block :: E') :: rs) root) (hstk : t.stack = block :: rest)
    (hw : t.exploration.lookup block = none) :
    ∃ W, DfsInv blocks (traversalStep blocks t block rest) ((block, W) :: (g, E') :: rs) root := by
  obtain ⟨hx1, hx2, hx3, hE1, hbnew', hbold, hbgray, hW, hWin⟩ :=
    white_facts (blocks := blocks) (rest := rest) hw hi.edges
  have hbnew : ∀ cur next, backEdge (traversalStep blocks t block rest).loopHeads cur next = true →
      backEdge t.loopHeads cur next = true ∨ cur = block :=
    fun c n h => (hbnew' c n h).imp id (·.1)
  refine ⟨((blockSuccessors blocks block).filter
    (fun s => ((block, Exploration.inProgress) :: t.exploration).lookup s == none)).reverse, ?_⟩
  have hrest : rest = flatSegs ((g, E') :: rs) ++ root := by
    have := hi.stackEq; rw [hstk] at this; simpa [flatSegs] using this
  have hroot : root = [] := by
    rcases hi.rootOk with h | ⟨_, h, _⟩
    · exact h
    · cases h
  have hbg : block ∉ (g :: rs.map (·.1)) := by
    intro h
    have := (hi.gray block).2 (by simpa using h)
    rw [hw] at this; cases this
  have hgx : t.exploration.lookup g = some .inProgress := (hi.gray g).2 (by simp)
  have hgb : g ≠ block := fun e => by subst e; rw [hw] at hgx; cases hgx
  have hexpl : ∀ x, t.exploration.lookup x ≠ none →
      ((block, Exploration.inProgress) :: t.exploration).lookup x ≠ none := by
    intro x hx; rw [hE1]; split <;> simp_all
  have hdone : ∀ x, ((block, Exploration.inProgress) :: t.exploration).lookup x = some .done ↔
      t.exploration.lookup x = some .done := by
    intro x; rw [hE1]; split
    · next h => subst h; simp [hw]
    · exact Iff.rfl
  obtain ⟨⟨hp1, hp2⟩, ⟨hs1, hs2⟩⟩ := And.intro hi.pending hi.succs
  refine ⟨by rw [hx3, hrest]; simp [flatSegs], Or.inl hroot,
    fun _ => by rw [hx1]; exact hexpl 0 (hi.entryOk hroot), fun x => ?_, ?_, ?_, ?_,
    by rw [hx2]; exact hi.postNodup, fun x => by rw [hx2, hx1, hdone]; exact hi.post x,
    fun b hb s hs => ?_, fun x hx hx0 => ?_, fun cur next h => ?_⟩
  · -- in progress
    rw [hx1, hE1]
    by_cases hx : x = block
    · subst hx; simp
    · simp only [hx, ↓reduceIte, hi.gray, List.map_cons, List.mem_cons, false_or]
  · -- no duplicates
    simp only [List.map_cons, List.nodup_cons] at hbg ⊢
    exact ⟨hbg, by simpa using hi.grayNodup⟩
  · -- pending
    refine ⟨fun e he => ?_, pending_mono hbnew ((g, E') :: rs) hbg
      ⟨fun e he => hp1 e (List.mem_cons_of_mem _ he), hp2⟩⟩
    obtain ⟨hs, hb, hn⟩ := hW e he
    refine ⟨hs, hb, ?_⟩
    rw [hE1] at hn
    intro hm
    rcases List.mem_cons.1 hm with rfl | hm
    · simp at hn
    · split at hn
      · simp at hn
      · have := (hi.gray e).2 (by simpa using hm)
        rw [hn] at this; cases this
  · -- successors
    rw [hx1]
    refine ⟨fun s hs => ?_, fun s hs => ?_, ?_⟩
    · cases hs' : ((block, Exploration.inProgress) :: t.exploration).lookup s with
      | none => exact Or.inr (Or.inr (Or.inl (hWin s hs hs')))
      | some ex =>
        cases ex with
        | inProgress => exact Or.inl (hbgray s hs hs')
        | done => exact Or.inr (Or.inl rfl)
    · rcases hs1 s hs with h | h | h | h
      · exact Or.inl (hbold _ _ h)
      · exact Or.inr (Or.inl ((hdone s).2 h))
      · rcases List.mem_cons.1 h with rfl | h
        · exact Or.inr (Or.inr (Or.inr (List.mem_singleton_self _)))
        · exact Or.inr (Or.inr (Or.inl h))
      · simp at h
    · exact succs_mono hbold (fun x h => (hdone x).2 h) rs [g] [g, block]
        (fun x hx => by simp at hx; simp [hx]) hs2
  · -- finished
    rw [hx2] at hb ⊢
    rcases hi.finished b hb s hs with h | h
    · exact Or.inl (hbold _ _ h)
    · exact Or.inr h
  · -- parent
    rw [hx1] at hx ⊢
    by_cases hxb : x = block
    · subst hxb
      refine ⟨g, hexpl g (by rw [hgx]; simp), (hp1 x (List.mem_cons_self ..)).1, ?_⟩
      cases hb : backEdge (traversalStep blocks t x rest).loopHeads g x with
      | false => rfl
      | true =>
        rcases hbnew g x hb with h | h
        · rw [(hp1 x (List.mem_cons_self ..)).2.1] at h; cases h
        · exact absurd h hgb
    · have hx' : t.exploration.lookup x ≠ none := by rw [hE1] at hx; simpa [hxb] using hx
      obtain ⟨y, hy, hxy, hback⟩ := hi.parent x hx' hx0
      refine ⟨y, hexpl y hy, hxy, ?_⟩
      cases hb : backEdge (traversalStep blocks t block rest).loopHeads y x with
      | false => rfl
      | true =>
        rcases hbnew y x hb with h | h
        · rw [hback] at h; cases h
        · subst h; exact absurd hw hy
  · -- recorded back edges
    rw [hx1]
    rcases hbnew' cur next h with h' | ⟨rfl, hg⟩
    · obtain ⟨h1, h2⟩ := hi.edges cur next h'
      exact ⟨hexpl cur h1, hexpl next h2⟩
    · exact ⟨by rw [hE1]; simp, by rw [hg]; simp⟩


/-- Exploring the entry. -/
theorem white_root {blocks : List BasicBlock} {t : Traversal} {rest : List Nat}
    (hi : DfsInv blocks t [] [0]) (hnone : ∀ x, t.exploration.lookup x = none)
    (hstk : t.stack = 0 :: rest) :
    ∃ W, DfsInv blocks (traversalStep blocks t 0 rest) [(0, W)] [] := by
  have hrest : rest = [] := by
    have := hi.stackEq; rw [hstk] at this; simpa [flatSegs] using this
  subst hrest
  have hw := hnone 0
  obtain ⟨hx1, hx2, hx3, hE1, hbnew, hbold, hbgray, hW, hWin⟩ :=
    white_facts (blocks := blocks) (rest := []) hw hi.edges
  refine ⟨((blockSuccessors blocks 0).filter
    (fun s => ((0, Exploration.inProgress) :: t.exploration).lookup s == none)).reverse, ?_⟩
  have hE : ∀ x, ((0, Exploration.inProgress) :: t.exploration).lookup x =
      if x = 0 then some .inProgress else none := by
    intro x; rw [hE1, hnone]
  have hpost : ∀ x, x ∉ t.postOrder := fun x hx => by
    rw [hi.post, hnone] at hx; cases hx
  refine ⟨by rw [hx3]; simp [flatSegs], Or.inl rfl, fun _ => by rw [hx1, hE]; simp,
    fun x => by rw [hx1, hE]; by_cases hx : x = 0 <;> simp [hx], by simp,
    ⟨fun e he => ?_, trivial⟩, ⟨fun s hs => ?_, trivial⟩, by rw [hx2]; exact hi.postNodup,
    fun x => ?_, fun b hb => by rw [hx2] at hb; exact absurd hb (hpost b),
    fun x hx hx0 => by rw [hx1, hE] at hx; simp [hx0] at hx, fun cur next h => ?_⟩
  · obtain ⟨hs, hb, hn⟩ := hW e he
    refine ⟨hs, hb, ?_⟩
    rw [hE] at hn
    intro hm
    simp only [List.map_nil, List.mem_singleton] at hm
    subst hm
    simp at hn
  · cases hs' : ((0, Exploration.inProgress) :: t.exploration).lookup s with
    | none => exact Or.inr (Or.inr (Or.inl (hWin s hs hs')))
    | some ex =>
      cases ex with
      | inProgress => exact Or.inl (hbgray s hs hs')
      | done => exact Or.inr (Or.inl (by rw [hx1]; exact hs'))
  · rw [hx2, hx1, hE]
    constructor
    · intro hx; exact absurd hx (hpost x)
    · intro hx; split at hx <;> simp at hx
  · rw [hx1]
    rcases hbnew cur next h with h' | ⟨rfl, hg⟩
    · obtain ⟨h1, -⟩ := hi.edges cur next h'
      exact absurd (hnone cur) h1
    · exact ⟨by rw [hE]; simp, by rw [hg]; simp⟩

/-- One step of the traversal preserves the invariant. -/
theorem traversalStep_inv {blocks : List BasicBlock} {t : Traversal} {segs : List (Nat × List Nat)}
    {root : List Nat} (hi : DfsInv blocks t segs root) {block : Nat} {rest : List Nat}
    (hstk : t.stack = block :: rest) :
    ∃ segs' root', DfsInv blocks (traversalStep blocks t block rest) segs' root' := by
  cases segs with
  | nil =>
    have hroot : root = block :: rest := by
      have := hi.stackEq; rw [hstk] at this; simpa [flatSegs] using this.symm
    rcases hi.rootOk with h | ⟨h, -, hnone⟩
    · rw [h] at hroot; cases hroot
    · rw [h] at hroot hi
      simp only [List.cons.injEq] at hroot
      obtain ⟨rfl, -⟩ := hroot
      obtain ⟨W, hW⟩ := white_root hi hnone hstk
      exact ⟨_, _, hW⟩
  | cons seg rs =>
    obtain ⟨g, E⟩ := seg
    cases E with
    | nil =>
      have hb : block = g := by
        have := hi.stackEq; rw [hstk] at this; simp [flatSegs] at this; exact this.1
      subst hb
      exact ⟨_, _, gray_step hi hstk ((hi.gray block).2 (by simp))⟩
    | cons e E' =>
      have hb : block = e := by
        have := hi.stackEq; rw [hstk] at this; simp [flatSegs] at this; exact this.1
      subst hb
      cases hl : t.exploration.lookup block with
      | none =>
        obtain ⟨W, hW⟩ := white_seg hi hstk hl
        exact ⟨_, _, hW⟩
      | some ex =>
        cases ex with
        | done => exact ⟨_, _, done_step hi hstk hl⟩
        | inProgress =>
          exfalso
          have hng := (hi.pending.1 block (List.mem_cons_self ..)).2.2
          exact hng (by have := (hi.gray block).1 hl; simpa using this)

/-- The traversal ends with an empty stack, every explored block finished. -/
theorem traverse_inv {blocks : List BasicBlock} :
    ∀ (fuel : Nat) (t t' : Traversal) (segs : List (Nat × List Nat)) (root : List Nat),
      DfsInv blocks t segs root → traverse blocks fuel t = .ok t' →
      ∃ segs root, DfsInv blocks t' segs root ∧ t'.stack = []
  | 0, _, _, _, _, _, h => by simp [traverse, outOfFuel] at h
  | fuel + 1, t, t', segs, root, hi, h => by
    unfold traverse at h
    split at h
    · next hs =>
      simp only [pure, Except.pure, Except.ok.injEq] at h
      subst h
      exact ⟨segs, root, hi, hs⟩
    · next block rest hs =>
      obtain ⟨segs', root', hi'⟩ := traversalStep_inv hi hs
      exact traverse_inv fuel _ t' segs' root' hi' h


/-! ## The traversal's properties -/

theorem flatSegs_nil {segs : List (Nat × List Nat)} {root : List Nat}
    (h : flatSegs segs ++ root = []) : segs = [] ∧ root = [] := by
  cases segs with
  | nil => simpa [flatSegs] using h
  | cons s rest => obtain ⟨g, E⟩ := s; simp [flatSegs] at h

theorem init_inv (blocks : List BasicBlock) :
    DfsInv blocks { stack := [0], exploration := [], loopHeads := [], postOrder := [] } [] [0] where
  stackEq := rfl
  rootOk := Or.inr ⟨rfl, rfl, fun _ => rfl⟩
  entryOk := fun h => by cases h
  gray := fun x => by simp
  grayNodup := by simp
  pending := trivial
  succs := trivial
  postNodup := by simp
  post := fun x => by simp
  finished := fun b hb => by simp at hb
  parent := fun x hx => by simp at hx
  edges := fun cur next h => by simp [backEdge] at h

theorem getElem?_idxOf' : ∀ {l : List Nat} {x : Nat}, x ∈ l → l[l.idxOf x]? = some x
  | [], _, hx => by simp at hx
  | a :: t, x, hx => by
    by_cases hax : a = x
    · subst hax; simp [List.idxOf_cons_self]
    · have hxt : x ∈ t := by simpa [Ne.symm hax] using hx
      have hab : (a == x) = false := by simp [hax]
      simp [List.idxOf_cons, hab, getElem?_idxOf' hxt]

theorem nodup_reverse' {l : List Nat} (h : l.Nodup) : l.reverse.Nodup :=
  List.pairwise_reverse.2 (h.imp fun hne => Ne.symm hne)

theorem reverse_index {l : List Nat} {x : Nat} (hx : x ∈ l) :
    (l.reverse)[l.length - 1 - l.idxOf x]? = some x := by
  have hlt := idxOf_lt hx
  rw [List.getElem?_reverse (by omega)]
  rw [show l.length - 1 - (l.length - 1 - l.idxOf x) = l.idxOf x by omega]
  exact getElem?_idxOf' hx

/-- **The traversal is a depth-first search.** -/
theorem build_dfs {code : Array Bytecode} {cfg : Cfg} (h : Cfg.build code = .ok cfg) :
    DfsFacts cfg := by
  unfold Cfg.build at h
  simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at h
  obtain ⟨t, ht, rfl⟩ := h
  obtain ⟨segs, root, hi, hs⟩ := traverse_inv _ _ t [] [0] (init_inv _) ht
  obtain ⟨rfl, rfl⟩ := flatSegs_nil (hi.stackEq ▸ hs)
  have hfin : ∀ x, t.exploration.lookup x ≠ none → x ∈ t.postOrder := by
    intro x hx
    rw [hi.post]
    cases hl : t.exploration.lookup x with
    | none => exact absurd hl hx
    | some ex =>
      cases ex with
      | done => rfl
      | inProgress => have := (hi.gray x).1 hl; simp at this
  have hnd : t.postOrder.reverse.Nodup := nodup_reverse' hi.postNodup
  refine ⟨hnd, ?_, fun b hb s hs => ?_, fun b hb s hs hback => ?_, fun x hx hx0 => ?_⟩
  · exact List.mem_reverse.2 (hfin 0 (hi.entryOk rfl))
  · simp only [List.mem_reverse] at hb ⊢
    rcases hi.finished b hb s hs with h | ⟨h, -⟩
    · exact hfin s (hi.edges b s h).2
    · exact h
  · simp only [List.mem_reverse] at hb
    rcases hi.finished b hb s hs with h | ⟨hsp, hlt⟩
    · rw [isBackEdge_eq] at hback
      simp only at hback
      rw [hback] at h; cases h
    · rw [traversalIndex_eq (cfg := ⟨buildBlocks code, t.postOrder.reverse, t.loopHeads⟩) hnd
        (reverse_index hb), traversalIndex_eq (cfg := ⟨buildBlocks code, t.postOrder.reverse,
        t.loopHeads⟩) hnd (reverse_index hsp)]
      have := idxOf_lt hb
      omega
  · simp only [List.mem_reverse] at hx
    have hx' : t.exploration.lookup x ≠ none := by
      intro h; rw [hi.post, h] at hx; cases hx
    obtain ⟨y, hy, hxy, hback⟩ := hi.parent x hx' hx0
    exact ⟨y, List.mem_reverse.2 (hfin y hy), hxy, hback⟩

/-- **The abstract interpreter's scheduler is sound.** -/
theorem schedulerSound : SchedulerSound :=
  schedulerSound_of_dfs fun _ _ h => build_dfs h

/-- **The abstract interpreter computes a post-fixpoint** of every analysis
whose join is sound for its order. -/
theorem analyze_postFixpoint {S : Type} {an : Analysis S} {Le : S → S → Prop}
    {J : Nat → S → Prop} (hJ : JoinSound an Le J) {code : Array Bytecode} {cfg : Cfg}
    (hJc : JClosed an cfg code J) {initial : S} (hinit : J 0 initial) {height : Nat}
    {inv : List (Nat × S)} (hb : Cfg.build code = .ok cfg)
    (h : analyzeFunction an cfg code initial height = .ok inv) :
    PostFixpointOf an Le cfg code initial inv :=
  analyzeFunctionOf_postFixpoint hJ hJc hinit (build_dfs hb) h

section Justified

variable {S : Type} {an : Analysis S} (J : Nat → S → Prop) {cfg : Cfg} {code : Array Bytecode}

/-- Propagating a post-state of block `b` that is justified at each successor
keeps every recorded pre-state justified. -/
theorem propagate_justified
    (hjoin : ∀ s old new j changed, J s old → J s new → an.join old new = .ok (j, changed) →
      J s j) {b : Nat} {post : S} :
    ∀ (l : List Nat) (inv inv' : List (Nat × S)) (cands cands' : List Nat),
      (∀ s ∈ l, J s post) → (∀ x p, inv.lookup x = some p → J x p) →
      l.foldlM (propagate an cfg b post) (inv, cands) = .ok (inv', cands') →
      ∀ x p, inv'.lookup x = some p → J x p
  | [], inv, inv', cands, cands', _, hinv, h => by
    simp [pure, Except.pure] at h
    obtain ⟨rfl, rfl⟩ := h
    exact hinv
  | succ :: l, inv, inv', cands, cands', hpost, hinv, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨⟨inv1, cands1⟩, h1, h⟩ := h
    refine propagate_justified hjoin l inv1 inv' cands1 cands' (fun s hs => hpost s (by simp [hs]))
      ?_ h
    unfold propagate at h1
    simp only at h1
    split at h1
    · next hnone =>
      simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h1
      obtain ⟨rfl, rfl⟩ := h1
      intro x p hx
      rw [lookup_append_single] at hx
      cases hl : inv.lookup x with
      | some w => rw [hl] at hx; simp only [Option.some.injEq] at hx; subst hx; exact hinv x w hl
      | none =>
        rw [hl] at hx
        simp only at hx
        split at hx
        · next hxs => subst hxs; simp only [Option.some.injEq] at hx; subst hx; exact hpost _ (by simp)
        · simp at hx
    · next old hold =>
      simp only [bind_ok] at h1
      obtain ⟨⟨joined, changed⟩, hj, h1⟩ := h1
      have hjj := hjoin succ old post joined changed (hinv succ old hold) (hpost succ (by simp)) hj
      simp only at h1
      split at h1
      · simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h1
        obtain ⟨rfl, rfl⟩ := h1
        intro x p hx
        rw [lookup_setPre] at hx
        by_cases hxs : x = succ
        · subst hxs; simp only [↓reduceIte, hold, Option.map_some, Option.some.injEq] at hx
          subst hx; exact hjj
        · simp only [hxs, ↓reduceIte] at hx; exact hinv x p hx
      · simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h1
        obtain ⟨rfl, rfl⟩ := h1
        exact hinv

/-- **Every recorded pre-state is justified**: a property of pre-states that
holds of the initial state at the entry, carries a block's pre-state over to
its post-state at every successor, and survives joins, holds of every
pre-state the analysis records. -/
theorem analyze_justified {initial : S} {height : Nat} {inv : List (Nat × S)}
    (hinit : J 0 initial)
    (hexec : ∀ b pre post, J b pre → executeBlock an cfg code b pre = .ok post →
      ∀ s ∈ cfg.successors b, J s post)
    (hjoin : ∀ s old new j changed, J s old → J s new → an.join old new = .ok (j, changed) →
      J s j)
    (h : analyzeFunction an cfg code initial height = .ok inv) :
    ∀ b pre, inv.lookup b = some pre → J b pre := by
  unfold analyzeFunction at h
  suffices ∀ (fuel : Nat) (inv₀ : List (Nat × S)) (nb : Option Nat),
      (∀ x p, inv₀.lookup x = some p → J x p) → analyzeLoop an cfg code fuel inv₀ nb = .ok inv →
      ∀ b pre, inv.lookup b = some pre → J b pre from
    this _ _ _ (fun x p hx => by
      by_cases hx0 : x = 0
      · subst hx0; simp at hx; subst hx; exact hinit
      · rw [lookup_cons_ne _ _ hx0] at hx; simp at hx) h
  intro fuel
  induction fuel with
  | zero =>
    intro inv₀ nb hinv h
    cases nb with
    | none => simp [analyzeLoop, pure, Except.pure] at h; subst h; exact hinv
    | some _ => simp [analyzeLoop, outOfFuel] at h
  | succ fuel ih =>
    intro inv₀ nb hinv h
    cases nb with
    | none => simp [analyzeLoop, pure, Except.pure] at h; subst h; exact hinv
    | some b =>
      simp only [analyzeLoop] at h
      split at h
      · exact ih _ _ hinv h
      · next pre hpre =>
        simp only [bind_ok] at h
        obtain ⟨post, hex, ⟨inv', cands⟩, hfold, h⟩ := h
        exact ih _ _ (propagate_justified J hjoin _ _ _ _ _
          (fun s hs => hexec b pre post (hinv b pre hpre) hex s hs) hinv hfold) h

end Justified

end MoveBytecode.Proofs
