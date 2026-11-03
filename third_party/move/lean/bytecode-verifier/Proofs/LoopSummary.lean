-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.LoopNesting
import Proofs.Traversal

/-!
# The loop summary's depth-first search

UNTRUSTED (checked by the kernel). `LoopSummary.build` numbers the blocks
reachable from the entry in pre-order by an iterative depth-first search,
and records each edge at its target: as a back edge when the target is in
progress, otherwise as another incoming edge.

The invariant (`SInv`) describes the stack as segments: each block in
progress, latest first, sits below the frame that finishes it, with the
successors it has still to visit on top; the entry's successors lie at the
bottom with no finishing frame. The nodes in progress therefore form a
chain, latest (and largest) on top, and a visit frame on top of the stack
comes from the top of that chain. A ghost list records the nodes in the
order they finish.

At the end (`summary_facts`): the explored blocks are closed under
successors and numbered injectively from the entry's 0; every edge between
them is recorded; the recorded edges join nodes of the summary; a back edge
never leads to a later node; and every other recorded edge leads to a node
that finished earlier, so the finishing order decreases along it.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

namespace Summary

/-! ## Lookups and updates -/

/-- The node of an explored block. -/
def nodeOf (E : List (Nat × SummaryExploration)) (b : Nat) : Option Nat :=
  match E.lookup b with
  | some (.inProgress k) | some (.done k) => some k
  | none => none

theorem lookup_cons_self {α : Type} (b : Nat) (x : α) (E : List (Nat × α)) :
    ((b, x) :: E).lookup b = some x := by
  simp [List.lookup]

theorem lookup_cons_ne {α : Type} {b c : Nat} (x : α) (E : List (Nat × α)) (h : c ≠ b) :
    ((b, x) :: E).lookup c = E.lookup c := by
  have hcb : (c == b) = false := by simp [h]
  simp [List.lookup, hcb]

/-- Marking a block finished. -/
theorem lookup_finish (E : List (Nat × SummaryExploration)) (block node c : Nat) :
    (E.map fun e => if e.1 == block then (block, SummaryExploration.done node) else e).lookup c =
      if c = block then (if (E.lookup c).isSome then some (.done node) else none)
      else E.lookup c := by
  induction E with
  | nil => simp [List.lookup]
  | cons e E ih =>
    obtain ⟨b, x⟩ := e
    by_cases hb : b = block
    · subst hb
      by_cases hc : c = b
      · subst hc; simp [List.lookup]
      · simp only [List.map_cons, beq_self_eq_true, if_true]
        rw [lookup_cons_ne _ _ hc, ih, lookup_cons_ne _ _ hc]
    · have hbb : (b == block) = false := by simp [hb]
      simp only [List.map_cons, hbb, Bool.false_eq_true, if_false]
      by_cases hc : c = b
      · subst hc
        rw [lookup_cons_self, lookup_cons_self]
        simp [hb]
      · rw [lookup_cons_ne _ _ hc, ih, lookup_cons_ne _ _ hc]

theorem modifyAt_getElem! {α : Type} [Inhabited α] (a : Array α) (i : Nat) (f : α → α) (j : Nat) :
    (modifyAt a i f)[j]! = if j = i ∧ i < a.size then f a[i]! else a[j]! := by
  unfold modifyAt
  by_cases hi : i < a.size
  · have : a[i]? = some a[i] := Array.getElem?_eq_getElem hi
    rw [this]
    simp only [Array.set!, getElem!_def, Array.getElem?_setIfInBounds]
    by_cases hj : j = i
    · subst hj; simp [hi]
    · simp [Ne.symm hj, hj]
  · have : a[i]? = none := Array.getElem?_eq_none (by omega)
    rw [this]
    simp [hi]

theorem modifyAt_size {α : Type} (a : Array α) (i : Nat) (f : α → α) :
    (modifyAt a i f).size = a.size := by
  unfold modifyAt
  split <;> simp [Array.set!]

/-- The successors a newly explored block pushes, last one on top. -/
theorem push_visits (node : Nat) (succs : List Nat) (stack : List Frontier) :
    succs.foldl (fun stk succ => Frontier.visit node succ :: stk) stack =
      succs.reverse.map (Frontier.visit node) ++ stack := by
  induction succs generalizing stack with
  | nil => rfl
  | cons s succs ih => simp [ih]

/-- A list without duplicates inside another is no longer. -/
theorem length_le_of_subset : ∀ {L M : List Nat}, L.Nodup → (∀ x ∈ L, x ∈ M) →
    L.length ≤ M.length
  | [], _, _, _ => Nat.zero_le _
  | a :: L, M, hnd, hsub => by
    have ha := hsub a (List.mem_cons_self ..)
    have hnd' := List.nodup_cons.1 hnd
    have hL : ∀ x ∈ L, x ∈ M.erase a := fun x hx => by
      have hxa : x ≠ a := fun h => hnd'.1 (h ▸ hx)
      exact (List.mem_erase_of_ne hxa).2 (hsub x (List.mem_cons_of_mem _ hx))
    have := length_le_of_subset hnd'.2 hL
    rw [List.length_erase_of_mem ha] at this
    simp only [List.length_cons]
    have : 0 < M.length := List.length_pos_of_mem ha
    omega

/-! ## The invariant -/

/-- A block in progress other than the entry: its node and block, the node
it was reached from, and the successors still to visit. -/
structure Seg where
  node : Nat
  block : Nat
  parent : Nat
  pending : List Nat

/-- The stack the segments make, latest first. -/
def flatSegs : List Seg → List Frontier
  | [] => []
  | g :: rest =>
    g.pending.map (Frontier.visit g.node) ++ Frontier.finish g.block g.node g.parent :: flatSegs rest

/-- The nodes in progress, latest first; the entry's node 0 is last. -/
def chain (segs : List Seg) : List Nat := segs.map (·.node) ++ [0]

/-- The edge from node `k` to block `c` is recorded at `c`'s node. -/
def Processed (st : SummaryState) (k c : Nat) : Prop :=
  ∃ kc, nodeOf st.exploration c = some kc ∧
    (k ∈ st.summary.preds[kc]! ∨ k ∈ st.summary.backs[kc]!)

/-- The successors of a block in progress: some still to visit, the others
recorded. -/
def SuccsOk (cfg : Cfg) (st : SummaryState) (node block : Nat) (pending : List Nat) : Prop :=
  (∀ c ∈ pending, c ∈ cfg.successors block) ∧
    ∀ c ∈ cfg.successors block, c ∈ pending ∨ Processed st node c

structure SInv (cfg : Cfg) (S : List Nat) (n : Nat) (st : SummaryState) (segs : List Seg)
    (R fin : List Nat) : Prop where
  stackEq : st.stack = flatSegs segs ++ R.map (Frontier.visit 0)
  chainDec : (chain segs).Pairwise (· > ·)
  sizeBlocks : st.summary.blocks.size = n
  sizePreds : st.summary.preds.size = n
  sizeBacks : st.summary.backs.size = n
  nextLe : st.nextNode ≤ n
  nodeLt : ∀ b k : Nat, nodeOf st.exploration b = some k → k < st.nextNode
  nodeBlock : ∀ b k : Nat, nodeOf st.exploration b = some k → st.summary.blocks[k]! = b
  nodeS : ∀ b k : Nat, nodeOf st.exploration b = some k → b ∈ S
  onto : ∀ k < st.nextNode, ∃ b, nodeOf st.exploration b = some k
  gray : ∀ b k : Nat, st.exploration.lookup b = some (.inProgress k) ↔
    (b = 0 ∧ k = 0) ∨ ∃ g ∈ segs, g.block = b ∧ g.node = k
  doneFin : ∀ b k : Nat, st.exploration.lookup b = some (.done k) → k ∈ fin
  finDone : ∀ k ∈ fin, ∃ b, st.exploration.lookup b = some (.done k)
  finNodup : fin.Nodup
  segsOk : ∀ g ∈ segs, SuccsOk cfg st g.node g.block g.pending
  rootOk : SuccsOk cfg st 0 0 R
  doneSucc : ∀ b k : Nat, st.exploration.lookup b = some (.done k) →
    ∀ c ∈ cfg.successors b, Processed st k c
  rangeOk : ∀ v x : Nat, (x ∈ st.summary.preds[v]! ∨ x ∈ st.summary.backs[v]!) →
    x < st.nextNode ∧ v < st.nextNode
  backLe : ∀ h x : Nat, x ∈ st.summary.backs[h]! → h ≤ x
  predOrder : ∀ v x : Nat, x ∈ st.summary.preds[v]! →
    (v ∈ fin ∧ (x ∉ fin ∨ fin.idxOf v < fin.idxOf x)) ∨
      (v ∉ fin ∧ x ∉ fin ∧ (chain segs).idxOf v < (chain segs).idxOf x)

section Facts

variable {cfg : Cfg} {S : List Nat} {n : Nat} {st : SummaryState} {segs : List Seg}
  {R fin : List Nat}

theorem nodeOf_inProgress {E : List (Nat × SummaryExploration)} {b k : Nat}
    (h : E.lookup b = some (.inProgress k)) : nodeOf E b = some k := by
  unfold nodeOf; rw [h]

theorem nodeOf_done {E : List (Nat × SummaryExploration)} {b k : Nat}
    (h : E.lookup b = some (.done k)) : nodeOf E b = some k := by
  unfold nodeOf; rw [h]

theorem nodeOf_some {E : List (Nat × SummaryExploration)} {b k : Nat}
    (h : nodeOf E b = some k) :
    E.lookup b = some (.inProgress k) ∨ E.lookup b = some (.done k) := by
  unfold nodeOf at h
  split at h <;> simp_all

theorem nodeOf_none {E : List (Nat × SummaryExploration)} {b : Nat}
    (h : E.lookup b = none) : nodeOf E b = none := by
  unfold nodeOf; rw [h]

/-- The same node belongs to one block. -/
theorem SInv.block_eq (hi : SInv cfg S n st segs R fin) {b b' k : Nat}
    (h : nodeOf st.exploration b = some k) (h' : nodeOf st.exploration b' = some k) : b = b' := by
  rw [← hi.nodeBlock b k h, hi.nodeBlock b' k h']

/-- A node in progress has not finished. -/
theorem SInv.gray_not_fin (hi : SInv cfg S n st segs R fin) {b k : Nat}
    (h : st.exploration.lookup b = some (.inProgress k)) : k ∉ fin := by
  intro hk
  obtain ⟨b', hb'⟩ := hi.finDone k hk
  have := hi.block_eq (nodeOf_inProgress h) (nodeOf_done hb')
  subst this
  rw [h] at hb'
  cases hb'

theorem SInv.chain_gray (hi : SInv cfg S n st segs R fin) {k : Nat} (hk : k ∈ chain segs) :
    ∃ b, st.exploration.lookup b = some (.inProgress k) := by
  simp only [chain, List.mem_append, List.mem_map, List.mem_singleton] at hk
  rcases hk with ⟨g, hg, rfl⟩ | rfl
  · exact ⟨g.block, (hi.gray _ _).2 (Or.inr ⟨g, hg, rfl, rfl⟩)⟩
  · exact ⟨0, (hi.gray _ _).2 (Or.inl ⟨rfl, rfl⟩)⟩

theorem SInv.chain_lt (hi : SInv cfg S n st segs R fin) {k : Nat} (hk : k ∈ chain segs) :
    k < st.nextNode := by
  obtain ⟨b, hb⟩ := hi.chain_gray hk
  exact hi.nodeLt b k (nodeOf_inProgress hb)

theorem SInv.chain_not_fin (hi : SInv cfg S n st segs R fin) {k : Nat} (hk : k ∈ chain segs) :
    k ∉ fin := by
  obtain ⟨b, hb⟩ := hi.chain_gray hk
  exact hi.gray_not_fin hb

/-- A node not finished is in progress. -/
theorem SInv.mem_chain (hi : SInv cfg S n st segs R fin) {k : Nat} (hk : k < st.nextNode)
    (hf : k ∉ fin) : k ∈ chain segs := by
  obtain ⟨b, hb⟩ := hi.onto k hk
  rcases nodeOf_some hb with h | h
  · rcases (hi.gray b k).1 h with ⟨-, rfl⟩ | ⟨g, hg, -, rfl⟩
    · simp [chain]
    · simp only [chain, List.mem_append, List.mem_map]
      exact Or.inl ⟨g, hg, rfl⟩
  · exact absurd (hi.doneFin b k h) hf

theorem SInv.fin_lt (hi : SInv cfg S n st segs R fin) {k : Nat} (hk : k ∈ fin) :
    k < st.nextNode := by
  obtain ⟨b, hb⟩ := hi.finDone k hk
  exact hi.nodeLt b k (nodeOf_done hb)

end Facts

/-! ## Growth -/

/-- The later state keeps every node and every recorded edge. -/
structure Grows (st st' : SummaryState) : Prop where
  node : ∀ b k : Nat, nodeOf st.exploration b = some k → nodeOf st'.exploration b = some k
  preds : ∀ k x : Nat, x ∈ st.summary.preds[k]! → x ∈ st'.summary.preds[k]!
  backs : ∀ k x : Nat, x ∈ st.summary.backs[k]! → x ∈ st'.summary.backs[k]!

theorem Grows.processed {st st' : SummaryState} (h : Grows st st') {k c : Nat}
    (hp : Processed st k c) : Processed st' k c := by
  obtain ⟨kc, hn, hr⟩ := hp
  exact ⟨kc, h.node c kc hn, hr.imp (h.preds kc k) (h.backs kc k)⟩

theorem Grows.succsOk {cfg : Cfg} {st st' : SummaryState} (h : Grows st st')
    {node block : Nat} {pending : List Nat} (hs : SuccsOk cfg st node block pending) :
    SuccsOk cfg st' node block pending :=
  ⟨hs.1, fun c hc => (hs.2 c hc).imp id h.processed⟩

theorem mem_modifyAt_append {a : Array (List Nat)} {i k x t : Nat} :
    x ∈ (modifyAt a i (· ++ [t]))[k]! ↔ x ∈ a[k]! ∨ (k = i ∧ i < a.size ∧ x = t) := by
  rw [modifyAt_getElem!]
  split
  · next h => simp [h.1, h.2, List.mem_append]
  · next h =>
    constructor
    · exact Or.inl
    · rintro (h' | ⟨h1, h2, -⟩)
      · exact h'
      · exact absurd ⟨h1, h2⟩ h

theorem mem_of_idxOf_lt {l : List Nat} {a : Nat} (h : l.idxOf a < l.length) : a ∈ l :=
  List.idxOf_lt_length_iff.1 h

theorem idxOf_cons_ne {l : List Nat} {a b : Nat} (h : b ≠ a) :
    (a :: l).idxOf b = l.idxOf b + 1 := by
  have : (a == b) = false := by simp [Ne.symm h]
  simp [List.idxOf_cons, this]

/-! ## Finishing a block -/

theorem nodeOf_finish {E : List (Nat × SummaryExploration)} {block node : Nat}
    (hb : E.lookup block = some (.inProgress node)) (c : Nat) :
    nodeOf (E.map fun e => if e.1 == block then (block, .done node) else e) c = nodeOf E c := by
  unfold nodeOf
  rw [lookup_finish]
  by_cases hc : c = block
  · subst hc; simp [hb]
  · simp [hc]

/-- Popping the frame that finishes the latest block in progress. -/
theorem finish_step {cfg : Cfg} {S : List Nat} {n : Nat} {st : SummaryState} {g : Seg}
    {segs : List Seg} {R fin : List Nat} {rest : List Frontier}
    (hi : SInv cfg S n st (g :: segs) R fin) (hg : g.pending = [])
    (hst : st.stack = Frontier.finish g.block g.node g.parent :: rest) :
    SInv cfg S n (SummaryState.step cfg { st with stack := rest }
      (.finish g.block g.node g.parent)) segs R (fin ++ [g.node]) := by
  have hgray : st.exploration.lookup g.block = some (.inProgress g.node) :=
    (hi.gray _ _).2 (Or.inr ⟨g, List.mem_cons_self .., rfl, rfl⟩)
  have hgfin : g.node ∉ fin := hi.gray_not_fin hgray
  have hdec := hi.chainDec
  simp only [chain, List.map_cons, List.cons_append, List.pairwise_cons] at hdec
  have hbelow : ∀ x ∈ chain segs, x < g.node := fun x hx => hdec.1 x hx
  have hg0 : g.node ≠ 0 := by
    have := hbelow 0 (by simp [chain]); omega
  have hrest : rest = flatSegs segs ++ R.map (Frontier.visit 0) := by
    have := hi.stackEq
    rw [hst] at this
    simp [flatSegs, hg] at this
    exact this
  -- The new state.
  let E' := st.exploration.map fun e =>
    if e.1 == g.block then (g.block, SummaryExploration.done g.node) else e
  have hlook : ∀ c, E'.lookup c =
      if c = g.block then some (.done g.node) else st.exploration.lookup c := by
    intro c
    show (st.exploration.map _).lookup c = _
    rw [lookup_finish]
    by_cases hc : c = g.block
    · subst hc; simp [hgray]
    · simp [hc]
  have hnode : ∀ c, nodeOf E' c = nodeOf st.exploration c := nodeOf_finish hgray
  have hgrows : Grows st (SummaryState.step cfg { st with stack := rest }
      (.finish g.block g.node g.parent)) :=
    ⟨fun b k h => by simp only [SummaryState.step]; rw [hnode]; exact h,
      fun _ _ h => h, fun _ _ h => h⟩
  have hroot : st.exploration.lookup 0 = some (.inProgress 0) := (hi.gray 0 0).2 (Or.inl ⟨rfl, rfl⟩)
  have hgb0 : g.block ≠ 0 := by
    intro h
    rw [h, hroot] at hgray
    simp only [Option.some.injEq, SummaryExploration.inProgress.injEq] at hgray
    exact hg0 hgray.symm
  simp only [SummaryState.step]
  refine ⟨hrest, hdec.2, hi.sizeBlocks, hi.sizePreds, hi.sizeBacks, hi.nextLe,
    fun b k h => hi.nodeLt b k (by rw [← hnode]; exact h),
    fun b k h => hi.nodeBlock b k (by rw [← hnode]; exact h),
    fun b k h => hi.nodeS b k (by rw [← hnode]; exact h),
    fun k hk => by obtain ⟨b, hb⟩ := hi.onto k hk; exact ⟨b, by rw [hnode]; exact hb⟩,
    ?_, ?_, ?_, ?_, fun g' hg' => hgrows.succsOk (hi.segsOk g' (List.mem_cons_of_mem _ hg')),
    hgrows.succsOk hi.rootOk, ?_, hi.rangeOk, hi.backLe, ?_⟩
  · -- The blocks in progress lose the finished one.
    intro b k
    rw [hlook]
    by_cases hb : b = g.block
    · subst hb
      simp only [if_true, Option.some.injEq, reduceCtorEq, false_iff, not_or, not_and,
        not_exists]
      refine ⟨fun h => absurd h hgb0, fun g' hg' hgb hgk => ?_⟩
      have := (hi.gray _ _).2 (Or.inr ⟨g', List.mem_cons_of_mem _ hg', hgb, rfl⟩)
      rw [hgray] at this
      simp only [Option.some.injEq, SummaryExploration.inProgress.injEq] at this
      have := hbelow g'.node (by simp [chain]; exact Or.inl ⟨g', hg', rfl⟩)
      omega
    · simp only [hb, if_false]
      rw [hi.gray]
      constructor
      · rintro (h | ⟨g', hg', h1, h2⟩)
        · exact Or.inl h
        · rcases List.mem_cons.1 hg' with rfl | hg'
          · exact absurd h1.symm hb
          · exact Or.inr ⟨g', hg', h1, h2⟩
      · rintro (h | ⟨g', hg', h1, h2⟩)
        · exact Or.inl h
        · exact Or.inr ⟨g', List.mem_cons_of_mem _ hg', h1, h2⟩
  · intro b k h
    rw [hlook] at h
    by_cases hb : b = g.block
    · simp only [hb, if_true, Option.some.injEq, SummaryExploration.done.injEq] at h
      subst h; simp
    · simp only [hb, if_false] at h
      exact List.mem_append_left _ (hi.doneFin b k h)
  · intro k hk
    rcases List.mem_append.1 hk with hk | hk
    · obtain ⟨b, hb⟩ := hi.finDone k hk
      refine ⟨b, ?_⟩
      rw [hlook]
      have : b ≠ g.block := by intro h; rw [h, hgray] at hb; cases hb
      simp [this, hb]
    · simp only [List.mem_singleton] at hk
      subst hk
      exact ⟨g.block, by rw [hlook]; simp⟩
  · rw [List.nodup_append]
    refine ⟨hi.finNodup, by simp, ?_⟩
    intro a ha b hb hab
    simp only [List.mem_singleton] at hb
    subst hb; subst hab
    exact hgfin ha
  · intro b k h c hc
    rw [hlook] at h
    by_cases hb : b = g.block
    · simp only [hb, if_true, Option.some.injEq, SummaryExploration.done.injEq] at h
      subst h; subst hb
      rcases (hi.segsOk g (List.mem_cons_self ..)).2 c hc with h | h
      · simp [hg] at h
      · exact hgrows.processed h
    · simp only [hb, if_false] at h
      exact hgrows.processed (hi.doneSucc b k h c hc)
  · -- The finishing order extends; the chain loses its top.
    intro v x hx
    rcases hi.predOrder v x hx with ⟨hv, hx'⟩ | ⟨hv, hxf, hidx⟩
    · left
      refine ⟨List.mem_append_left _ hv, ?_⟩
      by_cases hxfin : x ∈ fin
      · right
        rcases hx' with hx' | hx'
        · exact absurd hxfin hx'
        · rw [idxOf_append_mem hv, idxOf_append_mem hxfin]; exact hx'
      · by_cases hxg : x = g.node
        · subst hxg
          right
          rw [idxOf_append_mem hv, idxOf_append_last hgfin]
          exact idxOf_lt hv
        · left
          simp [hxfin, hxg]
    · have hxlt := (hi.rangeOk v x (Or.inl hx)).1
      have hxc := hi.mem_chain hxlt hxf
      have hxg : x ≠ g.node := by
        intro h; subst h
        simp [chain] at hidx
      by_cases hvg : v = g.node
      · subst hvg
        left
        exact ⟨by simp, by simp [hxf, hxg]⟩
      · right
        refine ⟨by simp [hv, hvg], by simp [hxf, hxg], ?_⟩
        simp only [chain, List.map_cons, List.cons_append] at hidx
        rw [idxOf_cons_ne hvg, idxOf_cons_ne hxg] at hidx
        simp only [chain]
        omega

/-! ## Visiting a successor -/

/-- Popping a visit frame: from the latest block in progress, or from the
entry. -/
inductive PopShape : List Seg → List Nat → Nat → Nat → List Seg → List Nat → Prop where
  | seg (g : Seg) (segs : List Seg) (R : List Nat) (c : Nat) (E : List Nat) :
      g.pending = c :: E → PopShape (g :: segs) R g.node c ({ g with pending := E } :: segs) R
  | root (R : List Nat) (c : Nat) : PopShape [] (c :: R) 0 c [] R

section Pop

variable {cfg : Cfg} {S : List Nat} {n : Nat} {st : SummaryState} {segs segs1 : List Seg}
  {R R1 fin : List Nat} {t c : Nat} {rest : List Frontier}

theorem PopShape.rest_eq (hp : PopShape segs R t c segs1 R1)
    (hstack : st.stack = flatSegs segs ++ R.map (Frontier.visit 0))
    (hst : st.stack = Frontier.visit t c :: rest) :
    rest = flatSegs segs1 ++ R1.map (Frontier.visit 0) := by
  rw [hst] at hstack
  cases hp with
  | seg g segs R c E hg => simp [flatSegs, hg] at hstack; simpa [flatSegs] using hstack
  | root R c => simp [flatSegs] at hstack; exact hstack

theorem PopShape.chain_eq (hp : PopShape segs R t c segs1 R1) : chain segs1 = chain segs := by
  cases hp <;> rfl

theorem PopShape.blocks (hp : PopShape segs R t c segs1 R1) (b k : Nat) :
    (∃ g ∈ segs1, g.block = b ∧ g.node = k) ↔ ∃ g ∈ segs, g.block = b ∧ g.node = k := by
  cases hp with
  | seg g segs R c E hg =>
    simp only [List.mem_cons, exists_eq_or_imp]
  | root R c => rfl

/-- The popped frame comes from the top of the chain. -/
theorem PopShape.top (hp : PopShape segs R t c segs1 R1) (hdec : (chain segs).Pairwise (· > ·)) :
    t ∈ chain segs ∧ ∀ x ∈ chain segs, x ≤ t := by
  cases hp with
  | seg g segs R c E hg =>
    simp only [chain, List.map_cons, List.cons_append, List.pairwise_cons] at hdec
    refine ⟨by simp [chain], fun x hx => ?_⟩
    simp only [chain, List.map_cons, List.cons_append, List.mem_cons] at hx
    rcases hx with rfl | hx
    · exact Nat.le_refl _
    · exact Nat.le_of_lt (hdec.1 x hx)
  | root R c => exact ⟨by simp [chain], fun x hx => by simp [chain] at hx; omega⟩

/-- The block of the top node, whose successor is visited. -/
theorem PopShape.source (hp : PopShape segs R t c segs1 R1) (hi : SInv cfg S n st segs R fin) :
    ∃ tb, st.exploration.lookup tb = some (.inProgress t) ∧ c ∈ cfg.successors tb := by
  cases hp with
  | seg g segs R c E hg =>
    refine ⟨g.block, (hi.gray _ _).2 (Or.inr ⟨g, List.mem_cons_self .., rfl, rfl⟩), ?_⟩
    exact (hi.segsOk g (List.mem_cons_self ..)).1 c (by rw [hg]; exact List.mem_cons_self ..)
  | root R c =>
    exact ⟨0, (hi.gray 0 0).2 (Or.inl ⟨rfl, rfl⟩), hi.rootOk.1 c (List.mem_cons_self ..)⟩

/-- The successor lists after the pop, once the visited edge is recorded. -/
theorem PopShape.succsOk (hp : PopShape segs R t c segs1 R1) (hi : SInv cfg S n st segs R fin)
    {st' : SummaryState} (hg : Grows st st') (hpr : Processed st' t c) :
    (∀ g ∈ segs1, SuccsOk cfg st' g.node g.block g.pending) ∧ SuccsOk cfg st' 0 0 R1 := by
  cases hp with
  | seg g segs R c E hge =>
    refine ⟨fun g' hg' => ?_, hg.succsOk hi.rootOk⟩
    rcases List.mem_cons.1 hg' with rfl | hg'
    · have hs := hi.segsOk g (List.mem_cons_self ..)
      refine ⟨fun c' hc' => hs.1 c' (by rw [hge]; exact List.mem_cons_of_mem _ hc'),
        fun c' hc' => ?_⟩
      rcases hs.2 c' hc' with h | h
      · rw [hge] at h
        rcases List.mem_cons.1 h with rfl | h
        · exact Or.inr hpr
        · exact Or.inl h
      · exact Or.inr (hg.processed h)
    · exact hg.succsOk (hi.segsOk g' (List.mem_cons_of_mem _ hg'))
  | root R c =>
    refine ⟨fun g' hg' => by simp at hg', fun c' hc' => hi.rootOk.1 c' (List.mem_cons_of_mem _ hc'),
      fun c' hc' => ?_⟩
    rcases hi.rootOk.2 c' hc' with h | h
    · rcases List.mem_cons.1 h with rfl | h
      · exact Or.inr hpr
      · exact Or.inl h
    · exact Or.inr (hg.processed h)

end Pop

/-- Fewer explored blocks than blocks while one block of `S` is unexplored. -/
theorem SInv.next_lt {cfg : Cfg} {S : List Nat} {n : Nat} {st : SummaryState} {segs : List Seg}
    {R fin : List Nat} (hi : SInv cfg S n st segs R fin) (hSlen : S.length ≤ n) {c : Nat}
    (hc : c ∈ S) (hnew : nodeOf st.exploration c = none) : st.nextNode < n := by
  let f (k : Nat) : Nat := st.summary.blocks[k]!
  have hf : ∀ k < st.nextNode, nodeOf st.exploration (f k) = some k := by
    intro k hk
    obtain ⟨b, hb⟩ := hi.onto k hk
    simp only [f]; rw [hi.nodeBlock b k hb]; exact hb
  have hnd : ((List.range st.nextNode).map f).Nodup := by
    refine List.pairwise_map.2 (List.nodup_range.imp_of_mem fun ha hb hab hfab => hab ?_)
    have h1 := hf _ (List.mem_range.1 ha)
    have h2 := hf _ (List.mem_range.1 hb)
    rw [hfab, h2] at h1
    exact (Option.some.inj h1).symm
  have hsub : ∀ x ∈ (List.range st.nextNode).map f, x ∈ S.erase c := by
    intro x hx
    obtain ⟨k, hk, rfl⟩ := List.mem_map.1 hx
    have hk' := hf k (List.mem_range.1 hk)
    have hne : f k ≠ c := by intro h; rw [h, hnew] at hk'; cases hk'
    exact (List.mem_erase_of_ne hne).2 (hi.nodeS _ _ hk')
  have := length_le_of_subset hnd hsub
  rw [List.length_map, List.length_range, List.length_erase_of_mem hc] at this
  have := List.length_pos_of_mem hc
  omega

section Visit

variable {cfg : Cfg} {S : List Nat} {n : Nat} {st : SummaryState} {segs segs1 : List Seg}
  {R R1 fin : List Nat} {t c : Nat} {rest : List Frontier}

theorem SInv.gray_chain (hi : SInv cfg S n st segs R fin) {b k : Nat}
    (h : st.exploration.lookup b = some (.inProgress k)) : k ∈ chain segs := by
  rcases (hi.gray b k).1 h with ⟨-, rfl⟩ | ⟨g, hg, -, rfl⟩
  · simp [chain]
  · simp only [chain, List.mem_append, List.mem_map]
    exact Or.inl ⟨g, hg, rfl⟩

/-- Visiting a block in progress records a back edge. -/
theorem visit_gray (hi : SInv cfg S n st segs R fin)
    (hst : st.stack = Frontier.visit t c :: rest) (hp : PopShape segs R t c segs1 R1) {k : Nat}
    (hk : st.exploration.lookup c = some (.inProgress k)) :
    SInv cfg S n (SummaryState.step cfg { st with stack := rest } (.visit t c)) segs1 R1 fin := by
  have hrest := hp.rest_eq hi.stackEq hst
  have hchain := hp.chain_eq
  obtain ⟨htc, htop⟩ := hp.top hi.chainDec
  have hkn : k < st.nextNode := hi.nodeLt c k (nodeOf_inProgress hk)
  have hkc : k ∈ chain segs := hi.gray_chain hk
  have hkb : k < st.summary.backs.size := by rw [hi.sizeBacks]; have := hi.nextLe; omega
  simp only [SummaryState.step, hk]
  have hgrows : Grows st { st with stack := rest, summary := { st.summary with
      backs := modifyAt st.summary.backs k (· ++ [t]) } } :=
    ⟨fun _ _ h => h, fun _ _ h => h, fun _ _ h => mem_modifyAt_append.2 (Or.inl h)⟩
  have hpr : Processed { st with stack := rest, summary := { st.summary with
      backs := modifyAt st.summary.backs k (· ++ [t]) } } t c :=
    ⟨k, nodeOf_inProgress hk, Or.inr (mem_modifyAt_append.2 (Or.inr ⟨rfl, hkb, rfl⟩))⟩
  obtain ⟨hsegs1, hroot1⟩ := hp.succsOk hi hgrows hpr
  exact {
    stackEq := hrest
    chainDec := by rw [hchain]; exact hi.chainDec
    sizeBlocks := hi.sizeBlocks
    sizePreds := hi.sizePreds
    sizeBacks := by simp only; rw [modifyAt_size, hi.sizeBacks]
    nextLe := hi.nextLe
    nodeLt := hi.nodeLt
    nodeBlock := hi.nodeBlock
    nodeS := hi.nodeS
    onto := hi.onto
    gray := fun b k => by simp only; rw [hi.gray, hp.blocks]
    doneFin := hi.doneFin
    finDone := hi.finDone
    finNodup := hi.finNodup
    segsOk := hsegs1
    rootOk := hroot1
    doneSucc := fun b k h c hc => hgrows.processed (hi.doneSucc b k h c hc)
    rangeOk := fun v x h => by
      simp only at h
      rcases h with h | h
      · exact hi.rangeOk v x (Or.inl h)
      · rcases mem_modifyAt_append.1 h with h | ⟨rfl, -, rfl⟩
        · exact hi.rangeOk v x (Or.inr h)
        · exact ⟨hi.chain_lt htc, hkn⟩
    backLe := fun h x hx => by
      simp only at hx
      rcases mem_modifyAt_append.1 hx with hx | ⟨rfl, -, rfl⟩
      · exact hi.backLe h x hx
      · exact htop _ hkc
    predOrder := by rw [hchain]; exact hi.predOrder }

/-- Visiting a finished block records another edge. -/
theorem visit_done (hi : SInv cfg S n st segs R fin)
    (hst : st.stack = Frontier.visit t c :: rest) (hp : PopShape segs R t c segs1 R1) {k : Nat}
    (hk : st.exploration.lookup c = some (.done k)) :
    SInv cfg S n (SummaryState.step cfg { st with stack := rest } (.visit t c)) segs1 R1 fin := by
  have hrest := hp.rest_eq hi.stackEq hst
  have hchain := hp.chain_eq
  obtain ⟨htc, -⟩ := hp.top hi.chainDec
  have hkn : k < st.nextNode := hi.nodeLt c k (nodeOf_done hk)
  have hkp : k < st.summary.preds.size := by rw [hi.sizePreds]; have := hi.nextLe; omega
  simp only [SummaryState.step, hk]
  have hgrows : Grows st { st with stack := rest, summary := { st.summary with
      preds := modifyAt st.summary.preds k (· ++ [t]) } } :=
    ⟨fun _ _ h => h, fun _ _ h => mem_modifyAt_append.2 (Or.inl h), fun _ _ h => h⟩
  have hpr : Processed { st with stack := rest, summary := { st.summary with
      preds := modifyAt st.summary.preds k (· ++ [t]) } } t c :=
    ⟨k, nodeOf_done hk, Or.inl (mem_modifyAt_append.2 (Or.inr ⟨rfl, hkp, rfl⟩))⟩
  obtain ⟨hsegs1, hroot1⟩ := hp.succsOk hi hgrows hpr
  exact {
    stackEq := hrest
    chainDec := by rw [hchain]; exact hi.chainDec
    sizeBlocks := hi.sizeBlocks
    sizePreds := by simp only; rw [modifyAt_size, hi.sizePreds]
    sizeBacks := hi.sizeBacks
    nextLe := hi.nextLe
    nodeLt := hi.nodeLt
    nodeBlock := hi.nodeBlock
    nodeS := hi.nodeS
    onto := hi.onto
    gray := fun b k => by simp only; rw [hi.gray, hp.blocks]
    doneFin := hi.doneFin
    finDone := hi.finDone
    finNodup := hi.finNodup
    segsOk := hsegs1
    rootOk := hroot1
    doneSucc := fun b k h c hc => hgrows.processed (hi.doneSucc b k h c hc)
    rangeOk := fun v x h => by
      simp only at h
      rcases h with h | h
      · rcases mem_modifyAt_append.1 h with h | ⟨rfl, -, rfl⟩
        · exact hi.rangeOk v x (Or.inl h)
        · exact ⟨hi.chain_lt htc, hkn⟩
      · exact hi.rangeOk v x (Or.inr h)
    backLe := hi.backLe
    predOrder := fun v x hx => by
      simp only at hx
      rw [hchain]
      rcases mem_modifyAt_append.1 hx with hx | ⟨rfl, -, rfl⟩
      · exact hi.predOrder v x hx
      · exact Or.inl ⟨hi.doneFin c v hk, Or.inl (hi.chain_not_fin htc)⟩ }

theorem nodeOf_cons (E : List (Nat × SummaryExploration)) (c v b : Nat) :
    nodeOf ((c, .inProgress v) :: E) b = if b = c then some v else nodeOf E b := by
  by_cases hb : b = c
  · subst hb; simp [nodeOf]
  · simp only [hb, if_false]; unfold nodeOf; rw [lookup_cons_ne _ _ hb]

/-- Visiting an unexplored block numbers it and puts it in progress. -/
theorem visit_new (hS : ∀ b ∈ S, ∀ c ∈ cfg.successors b, c ∈ S) (hSlen : S.length ≤ n)
    (hi : SInv cfg S n st segs R fin)
    (hst : st.stack = Frontier.visit t c :: rest) (hp : PopShape segs R t c segs1 R1)
    (hk : st.exploration.lookup c = none) :
    SInv cfg S n (SummaryState.step cfg { st with stack := rest } (.visit t c))
      (⟨st.nextNode, c, t, (cfg.successors c).reverse⟩ :: segs1) R1 fin := by
  have hrest := hp.rest_eq hi.stackEq hst
  have hchain := hp.chain_eq
  obtain ⟨htc, -⟩ := hp.top hi.chainDec
  obtain ⟨tb, htb, hctb⟩ := hp.source hi
  have hnone : nodeOf st.exploration c = none := nodeOf_none hk
  have hcS : c ∈ S := hS tb (hi.nodeS tb t (nodeOf_inProgress htb)) c hctb
  have hNn : st.nextNode < n := hi.next_lt hSlen hcS hnone
  have hNp : st.nextNode < st.summary.preds.size := by rw [hi.sizePreds]; exact hNn
  have hNb : st.nextNode < st.summary.blocks.size := by rw [hi.sizeBlocks]; exact hNn
  have hold : ∀ b k, nodeOf st.exploration b = some k → b ≠ c := by
    intro b k h hb; subst hb; rw [hnone] at h; cases h
  have hc0 : c ≠ 0 := by
    intro h; subst h; rw [(hi.gray 0 0).2 (Or.inl ⟨rfl, rfl⟩)] at hk; cases hk
  simp only [SummaryState.step, hk]
  let N := st.nextNode
  let st' : SummaryState :=
    { summary := { st.summary with blocks := st.summary.blocks.set! N c,
                                   preds := modifyAt st.summary.preds N (· ++ [t]) }
      exploration := (c, .inProgress N) :: st.exploration
      nextNode := N + 1
      stack := (cfg.successors c).foldl (fun stk succ => Frontier.visit N succ :: stk)
        (Frontier.finish c N t :: rest) }
  show SInv cfg S n st' _ R1 fin
  have hnode : ∀ b, nodeOf st'.exploration b = if b = c then some N else nodeOf st.exploration b :=
    nodeOf_cons _ _ _
  have hlook : ∀ b, st'.exploration.lookup b =
      if b = c then some (.inProgress N) else st.exploration.lookup b := by
    intro b
    by_cases hb : b = c
    · subst hb; simp [st']
    · simp only [hb, if_false]; exact lookup_cons_ne _ _ hb
  have hgrows : Grows st st' := ⟨fun b k h => by rw [hnode, if_neg (hold b k h)]; exact h,
    fun _ _ h => mem_modifyAt_append.2 (Or.inl h), fun _ _ h => h⟩
  have hpr : Processed st' t c :=
    ⟨N, by rw [hnode]; simp, Or.inl (mem_modifyAt_append.2 (Or.inr ⟨rfl, hNp, rfl⟩))⟩
  obtain ⟨hsegs1, hroot1⟩ := hp.succsOk hi hgrows hpr
  have hchainN : chain (⟨N, c, t, (cfg.successors c).reverse⟩ :: segs1) = N :: chain segs := by
    rw [← hchain]; rfl
  have hNchain : N ∉ chain segs := fun h => by have := hi.chain_lt h; omega
  have hNfin : N ∉ fin := fun h => by have := hi.fin_lt h; omega
  exact {
    stackEq := by
      simp only [st', push_visits, hrest, flatSegs, List.append_assoc, List.cons_append]
      rfl
    chainDec := by
      rw [hchainN, List.pairwise_cons]
      exact ⟨fun x hx => hi.chain_lt hx, hi.chainDec⟩
    sizeBlocks := by simp only [st']; rw [size_set!, hi.sizeBlocks]
    sizePreds := by simp only [st']; rw [modifyAt_size, hi.sizePreds]
    sizeBacks := hi.sizeBacks
    nextLe := hNn
    nodeLt := fun b k h => by
      rw [hnode] at h
      split at h
      · cases h; show N < N + 1; omega
      · have := hi.nodeLt b k h; show k < N + 1; omega
    nodeBlock := fun b k h => by
      rw [hnode] at h
      show (st.summary.blocks.set! N c)[k]! = b
      rw [getElem!_set!]
      split at h
      · next hb =>
        cases h; subst hb
        have : N < st.summary.blocks.size := hNb
        simp [this]
      · have := hi.nodeLt b k h
        have hkN : k ≠ N := by omega
        simp only [hkN, false_and, if_false]
        exact hi.nodeBlock b k h
    nodeS := fun b k h => by
      rw [hnode] at h
      split at h
      · next hb => subst hb; exact hcS
      · exact hi.nodeS b k h
    onto := fun k hk' => by
      by_cases hkN : k = N
      · exact ⟨c, by rw [hnode, hkN]; simp⟩
      · obtain ⟨b, hb⟩ := hi.onto k (by show k < N; have : k < N + 1 := hk'; omega)
        exact ⟨b, by rw [hnode, if_neg (hold b k hb)]; exact hb⟩
    gray := fun b k => by
      rw [hlook]
      by_cases hb : b = c
      · subst hb
        simp only [if_true, Option.some.injEq, SummaryExploration.inProgress.injEq]
        constructor
        · rintro rfl; exact Or.inr ⟨_, List.mem_cons_self .., rfl, rfl⟩
        · rintro (⟨h, -⟩ | ⟨g, hg, hgb, hgk⟩)
          · exact absurd h hc0
          · rcases List.mem_cons.1 hg with rfl | hg
            · exact hgk
            · have := (hi.gray _ _).2 (Or.inr ((hp.blocks _ _).1 ⟨g, hg, hgb, rfl⟩))
              rw [hk] at this; cases this
      · simp only [hb, if_false, List.mem_cons, exists_eq_or_imp]
        rw [hi.gray, ← hp.blocks]
        constructor
        · rintro (h | h)
          · exact Or.inl h
          · exact Or.inr (Or.inr h)
        · rintro (h | ⟨h, -⟩ | h)
          · exact Or.inl h
          · exact absurd h.symm hb
          · exact Or.inr h
    doneFin := fun b k h => by
      rw [hlook] at h
      split at h
      · cases h
      · exact hi.doneFin b k h
    finDone := fun k hk' => by
      obtain ⟨b, hb⟩ := hi.finDone k hk'
      refine ⟨b, ?_⟩
      rw [hlook, if_neg (hold b k (nodeOf_done hb))]
      exact hb
    finNodup := hi.finNodup
    segsOk := fun g hg => by
      rcases List.mem_cons.1 hg with rfl | hg
      · exact ⟨fun x hx => List.mem_reverse.1 hx, fun x hx => Or.inl (List.mem_reverse.2 hx)⟩
      · exact hsegs1 g hg
    rootOk := hroot1
    doneSucc := fun b k h c' hc' => by
      rw [hlook] at h
      split at h
      · cases h
      · exact hgrows.processed (hi.doneSucc b k h c' hc')
    rangeOk := fun v x h => by
      show x < N + 1 ∧ v < N + 1
      rcases h with h | h
      · rcases mem_modifyAt_append.1 h with h | ⟨rfl, -, rfl⟩
        · have := hi.rangeOk v x (Or.inl h); omega
        · have := hi.chain_lt htc; omega
      · have := hi.rangeOk v x (Or.inr h); omega
    backLe := hi.backLe
    predOrder := fun v x hx => by
      rw [hchainN]
      rcases mem_modifyAt_append.1 hx with hx | ⟨rfl, -, rfl⟩
      · rcases hi.predOrder v x hx with h | ⟨hv, hxf, hidx⟩
        · exact Or.inl h
        · right
          have hvc := mem_of_idxOf_lt (Nat.lt_of_lt_of_le hidx (List.idxOf_le_length))
          have hxc := hi.mem_chain (hi.rangeOk v x (Or.inl hx)).1 hxf
          have hvN : v ≠ N := fun h => hNchain (h ▸ hvc)
          have hxN : x ≠ N := fun h => hNchain (h ▸ hxc)
          refine ⟨hv, hxf, ?_⟩
          rw [idxOf_cons_ne hvN, idxOf_cons_ne hxN]
          omega
      · right
        have htN : x ≠ N := fun h => hNchain (h ▸ htc)
        refine ⟨hNfin, hi.chain_not_fin htc, ?_⟩
        rw [idxOf_cons_ne htN]
        simp }

end Visit

/-! ## The search -/

theorem flatSegs_eq_nil {segs : List Seg} {R : List Nat}
    (h : flatSegs segs ++ R.map (Frontier.visit 0) = []) : segs = [] ∧ R = [] := by
  cases segs with
  | nil => simpa [flatSegs] using h
  | cons g segs => simp [flatSegs] at h

/-- Each step of the search keeps the invariant; when the stack empties,
only the entry is still in progress. -/
theorem run_inv {cfg : Cfg} {S : List Nat} {n : Nat}
    (hS : ∀ b ∈ S, ∀ c ∈ cfg.successors b, c ∈ S) (hSlen : S.length ≤ n) :
    ∀ (fuel : Nat) (st : SummaryState) (segs : List Seg) (R fin : List Nat) (st' : SummaryState),
      SInv cfg S n st segs R fin → SummaryState.run cfg fuel st = .ok st' →
      ∃ fin', SInv cfg S n st' [] [] fin'
  | 0, _, _, _, _, _, _, h => by simp [SummaryState.run, outOfFuel] at h
  | fuel + 1, st, segs, R, fin, st', hi, h => by
    unfold SummaryState.run at h
    split at h
    · next hst =>
      simp only [pure, Except.pure, Except.ok.injEq] at h
      subst h
      have := hi.stackEq
      rw [hst] at this
      obtain ⟨rfl, rfl⟩ := flatSegs_eq_nil this.symm
      exact ⟨fin, hi⟩
    · next action rest hst =>
      have hstack := hi.stackEq
      rw [hst] at hstack
      cases segs with
      | cons g segs =>
        cases hg : g.pending with
        | nil =>
          simp only [flatSegs, hg, List.map_nil, List.nil_append, List.cons_append,
            List.cons.injEq] at hstack
          obtain ⟨rfl, -⟩ := hstack
          exact run_inv hS hSlen fuel _ segs R _ st' (finish_step hi hg hst) h
        | cons c E =>
          simp only [flatSegs, hg, List.map_cons, List.cons_append, List.cons.injEq] at hstack
          obtain ⟨rfl, -⟩ := hstack
          have hp := PopShape.seg g segs R c E hg
          cases hl : st.exploration.lookup c with
          | none => exact run_inv hS hSlen fuel _ _ R fin st' (visit_new hS hSlen hi hst hp hl) h
          | some e =>
            cases e with
            | inProgress k =>
              exact run_inv hS hSlen fuel _ _ R fin st' (visit_gray hi hst hp hl) h
            | done k => exact run_inv hS hSlen fuel _ _ R fin st' (visit_done hi hst hp hl) h
      | nil =>
        cases R with
        | nil => simp [flatSegs] at hstack
        | cons c R =>
          simp only [flatSegs, List.map_cons, List.nil_append, List.cons.injEq] at hstack
          obtain ⟨rfl, -⟩ := hstack
          have hp := PopShape.root R c
          cases hl : st.exploration.lookup c with
          | none => exact run_inv hS hSlen fuel _ _ R fin st' (visit_new hS hSlen hi hst hp hl) h
          | some e =>
            cases e with
            | inProgress k =>
              exact run_inv hS hSlen fuel _ _ R fin st' (visit_gray hi hst hp hl) h
            | done k => exact run_inv hS hSlen fuel _ _ R fin st' (visit_done hi hst hp hl) h

theorem replicate_getElem! (n v : Nat) : (Array.replicate n ([] : List Nat))[v]! = [] := by
  by_cases h : v < n
  · simp [h]
  · simp [h]; rfl

theorem replicate_zero_getElem! (n v : Nat) : (Array.replicate n (0 : Nat))[v]! = 0 := by
  by_cases h : v < n
  · simp [h]
  · simp [h]

/-- The state the search starts from. -/
theorem init_inv (cfg : Cfg) {S : List Nat} (hSlen : S.length ≤ cfg.blocks.length)
    (h0 : 0 ∈ S) :
    SInv cfg S cfg.blocks.length
      { summary := { blocks := (Array.replicate cfg.blocks.length 0).set! 0 0,
                     descs := Array.replicate cfg.blocks.length 0,
                     backs := Array.replicate cfg.blocks.length [],
                     preds := Array.replicate cfg.blocks.length [] },
        exploration := [(0, .inProgress 0)], nextNode := 1,
        stack := ((cfg.successors 0).map (fun succ => Frontier.visit 0 succ)).reverse }
      [] (cfg.successors 0).reverse [] := by
  have hn : 0 < cfg.blocks.length := Nat.lt_of_lt_of_le (List.length_pos_of_mem h0) hSlen
  have hnode : ∀ b, nodeOf [(0, SummaryExploration.inProgress 0)] b =
      if b = 0 then some 0 else none := by
    intro b
    by_cases hb : b = 0
    · subst hb; simp [nodeOf]
    · simp only [hb, if_false]; unfold nodeOf; rw [lookup_cons_ne _ _ hb]; rfl
  have hlook : ∀ b, [(0, SummaryExploration.inProgress 0)].lookup b =
      if b = 0 then some (.inProgress 0) else none := by
    intro b
    by_cases hb : b = 0
    · subst hb; simp
    · simp only [hb, if_false]; rw [lookup_cons_ne _ _ hb]; rfl
  exact {
    stackEq := by simp [flatSegs, List.map_reverse]
    chainDec := by simp [chain]
    sizeBlocks := by simp [Array.set!]
    sizePreds := by simp
    sizeBacks := by simp
    nextLe := hn
    nodeLt := fun b k h => by
      rw [hnode] at h; split at h
      · cases h; exact Nat.zero_lt_one
      · cases h
    nodeBlock := fun b k h => by
      rw [hnode] at h; split at h
      · next hb =>
        cases h; subst hb; rw [getElem!_set!]
        split
        · rfl
        · exact replicate_zero_getElem! _ _
      · cases h
    nodeS := fun b k h => by
      rw [hnode] at h; split at h
      · next hb => subst hb; exact h0
      · cases h
    onto := fun k hk => by
      have hk' : k < 1 := hk
      have : k = 0 := by omega
      subst this
      exact ⟨0, by rw [hnode]; simp⟩
    gray := fun b k => by
      rw [hlook]
      by_cases hb : b = 0
      · subst hb; simp [eq_comm]
      · simp [hb]
    doneFin := fun b k h => by rw [hlook] at h; split at h <;> cases h
    finDone := fun k hk => by simp at hk
    finNodup := List.nodup_nil
    segsOk := fun g hg => by simp at hg
    rootOk := ⟨fun c hc => List.mem_reverse.1 hc, fun c hc => Or.inl (List.mem_reverse.2 hc)⟩
    doneSucc := fun b k h => by rw [hlook] at h; split at h <;> cases h
    rangeOk := fun v x h => by simp [replicate_getElem!] at h
    backLe := fun h x hx => by simp [replicate_getElem!] at hx
    predOrder := fun v x hx => by simp [replicate_getElem!] at hx }

end Summary

open Summary in
/-- **The loop summary is a depth-first search**: the explored blocks are
numbered injectively from the entry's 0 and closed under successors; every
edge between them is recorded at its target; the summary is what the loop
nesting needs; and a measure, the finishing order, decreases along every
recorded edge other than a back edge. -/
theorem summary_facts {cfg : Cfg} {s : LoopSummary} (hb : LoopSummary.build cfg = .ok s)
    {S : List Nat} (hSlen : S.length ≤ cfg.blocks.length) (h0 : 0 ∈ S)
    (hS : ∀ b ∈ S, ∀ c ∈ cfg.successors b, c ∈ S) :
    ∃ (node : Nat → Option Nat) (μ : Nat → Nat),
      node 0 = some 0 ∧
      (∀ b b' k, node b = some k → node b' = some k → b = b') ∧
      (∀ b k, node b = some k → ∀ c ∈ cfg.successors b,
        ∃ kc, node c = some kc ∧ Recorded s k kc) ∧
      SummaryOk s ∧
      (∀ v x, x ∈ s.predEdges v → μ v < μ x) := by
  unfold LoopSummary.build at hb
  simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at hb
  obtain ⟨st, hrun, rfl⟩ := hb
  obtain ⟨fin, hi⟩ := run_inv hS hSlen _ _ _ _ _ st (Summary.init_inv cfg hSlen h0) hrun
  have hroot : st.exploration.lookup 0 = some (.inProgress 0) := (hi.gray 0 0).2 (Or.inl ⟨rfl, rfl⟩)
  refine ⟨nodeOf st.exploration, fun v => if v ∈ fin then fin.idxOf v else fin.length,
    nodeOf_inProgress hroot, fun b b' k h h' => hi.block_eq h h', ?_, ⟨fun w w' h => ?_,
    fun h x hx => hi.backLe h x hx⟩, fun v x hx => ?_⟩
  · intro b k h c hc
    have hp : Processed st k c := by
      rcases nodeOf_some h with h | h
      · rcases (hi.gray b k).1 h with ⟨rfl, rfl⟩ | ⟨g, hg, -⟩
        · rcases hi.rootOk.2 c hc with h | h
          · simp at h
          · exact h
        · simp at hg
      · exact hi.doneSucc b k h c hc
    obtain ⟨kc, hkc, hr⟩ := hp
    exact ⟨kc, hkc, hr⟩
  · have := hi.rangeOk w' w h
    have := hi.nextLe
    rw [hi.sizeBlocks]
    omega
  · rcases hi.predOrder v x hx with ⟨hv, hx'⟩ | ⟨hv, hxf, hidx⟩
    · simp only [hv, if_true]
      rcases hx' with hx' | hx'
      · simp only [hx', if_false]; exact idxOf_lt hv
      · by_cases hxfin : x ∈ fin
        · simp only [hxfin, if_true]; exact hx'
        · simp only [hxfin, if_false]; exact idxOf_lt hv
    · exfalso
      have hxc := hi.mem_chain (hi.rangeOk v x (Or.inl hx)).1 hxf
      simp only [chain, List.map_nil, List.nil_append, List.mem_singleton] at hxc
      subst hxc
      simp [chain] at hidx

end MoveBytecode.Proofs
