-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Availability

/-!
# The abstract interpreter's scheduler

UNTRUSTED (checked by the kernel). `analyzeFunction` sweeps the blocks in
traversal order (reverse post-order), re-running a loop head when a back edge
changes its pre-state. When it succeeds, its invariant map is a
post-fixpoint (`PostFixpoint`), provided the traversal has the properties of
a depth-first search (`DfsFacts`): no duplicates; closed under successors; an
edge that is not a recorded back edge goes forward in the order; and every
block but the entry is reached by such an edge from an earlier block.

The sweep invariant (`LoopInv`): about to analyze block `b`, every analyzed
block before `b` in traversal order is stable, i.e. it executes from its
recorded pre-state to a post-state that is at most the pre-state of each
successor. Analyzing `b` can change only the pre-states of its successors; a
successor before `b` is reached by a back edge, so if its pre-state changed
it becomes a candidate, and the sweep resumes no later than it. A successor
seen for the first time comes after `b`: every block before `b` is already
analyzed, because its forward predecessor is stable.

Reducibility of the graph is not needed.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## The invariant map -/

theorem lookup_cons_eq {S : Type} (k : Nat) (v : S) (rest : List (Nat × S)) :
    ((k, v) :: rest).lookup k = some v := by
  simp [List.lookup]

theorem lookup_cons_ne {S : Type} {k x : Nat} (v : S) (rest : List (Nat × S)) (h : x ≠ k) :
    ((k, v) :: rest).lookup x = rest.lookup x := by
  have : (x == k) = false := by simpa using h
  simp [List.lookup, this]

theorem lookup_setPre {S : Type} (inv : List (Nat × S)) (k : Nat) (s : S) (x : Nat) :
    (setPre inv k s).lookup x = if x = k then (inv.lookup k).map (fun _ => s) else inv.lookup x := by
  induction inv with
  | nil => simp [setPre]
  | cons e rest ih =>
    obtain ⟨key, v⟩ := e
    have hmap : setPre ((key, v) :: rest) k s =
        (if key = k then (k, s) else (key, v)) :: setPre rest k s := by
      simp [setPre]
    rw [hmap]
    by_cases hk : key = k
    · subst hk
      simp only [↓reduceIte]
      by_cases hx : x = key
      · subst hx; simp
      · rw [lookup_cons_ne _ _ hx, ih, lookup_cons_ne _ _ hx]
        simp [hx]
    · simp only [hk, ↓reduceIte]
      by_cases hx : x = key
      · subst hx
        rw [lookup_cons_eq]
        simp [hk]
      · rw [lookup_cons_ne _ _ hx, ih]
        by_cases hxk : x = k
        · subst hxk
          simp [lookup_cons_ne _ _ (Ne.symm hk)]
        · simp [hxk, lookup_cons_ne _ _ hx]

theorem lookup_append_single {S : Type} (inv : List (Nat × S)) (k : Nat) (v : S) (x : Nat) :
    (inv ++ [(k, v)]).lookup x = match inv.lookup x with
      | some w => some w
      | none => if x = k then some v else none := by
  induction inv with
  | nil =>
    by_cases hx : x = k
    · subst hx; simp
    · simp [lookup_cons_ne _ _ hx, hx]
  | cons e rest ih =>
    obtain ⟨key, w⟩ := e
    by_cases hx : x = key
    · subst hx; simp
    · rw [List.cons_append, lookup_cons_ne _ _ hx, lookup_cons_ne _ _ hx, ih]

/-! ## Analyses the scheduler is sound for -/

/-- What the scheduler needs of an analysis: a preorder on its states, and a
join that is an upper bound and returns the old state when nothing changed,
for the pre-states of a block `s` that satisfy `J s`. -/
structure JoinSound {S : Type} (an : Analysis S) (Le : S → S → Prop) (J : Nat → S → Prop) :
    Prop where
  refl : ∀ a, Le a a
  trans : ∀ {a b c}, Le a b → Le b c → Le a c
  join : ∀ {s : Nat} {a b j : S} {changed : Bool}, J s a → J s b → an.join a b = .ok (j, changed) →
    Le a j ∧ Le b j ∧ (changed = false → j = a)

/-- `J` holds of the pre-states the analysis can record: a block's post-state
satisfies it at every successor, and joins keep it. -/
structure JClosed {S : Type} (an : Analysis S) (cfg : Cfg) (code : Array Bytecode)
    (J : Nat → S → Prop) : Prop where
  exec : ∀ b pre post, J b pre → executeBlock an cfg code b pre = .ok post →
    ∀ s ∈ cfg.successors b, J s post
  join : ∀ s a b j changed, J s a → J s b → an.join a b = .ok (j, changed) → J s j

/-- A post-fixpoint of analysis `an` for the order `Le`: the entry's
pre-state covers `initial`, and every analyzed block executes from its
pre-state to a post-state covered by the pre-state of every successor. -/
structure PostFixpointOf {S : Type} (an : Analysis S) (Le : S → S → Prop) (cfg : Cfg)
    (code : Array Bytecode) (initial : S) (inv : List (Nat × S)) : Prop where
  entry : ∃ pre, inv.lookup 0 = some pre ∧ Le initial pre
  closed : ∀ b pre, inv.lookup b = some pre → ∃ post,
    executeBlock an cfg code b pre = .ok post ∧
    ∀ s ∈ cfg.successors b, ∃ pre', inv.lookup s = some pre' ∧ Le post pre'

theorem joinSound_locals : JoinSound localsAnalysis StateLe (fun _ _ => True) where
  refl := stateLe_refl
  trans := stateLe_trans
  join := fun _ _ h => join_ok h

theorem jClosed_true {S : Type} (an : Analysis S) (cfg : Cfg) (code : Array Bytecode) :
    JClosed an cfg code (fun _ _ => True) := ⟨fun _ _ _ _ _ _ _ => trivial, fun _ _ _ _ _ _ _ _ => trivial⟩

/-! ## Propagation -/

section Propagation

variable {S : Type} (Le : S → S → Prop) (cfg : Cfg) (b : Nat) (post : S)
  (inv₀ : List (Nat × S)) (c0 : List Nat)

/-- What propagating a block's post-state into successors `done` did. -/
structure PropInv (done : List Nat) (inv : List (Nat × S)) (cands : List Nat) :
    Prop where
  grow : ∀ x p, inv₀.lookup x = some p → ∃ p', inv.lookup x = some p' ∧ Le p p' ∧
    (p' ≠ p → x ∈ done ∧ (cfg.isBackEdge b x = true → x ∈ cands))
  covered : ∀ s ∈ done, ∃ p', inv.lookup s = some p' ∧ Le post p'
  fresh : ∀ x p', inv.lookup x = some p' → inv₀.lookup x = none → x ∈ done
  keep : ∀ c ∈ c0, c ∈ cands
  cands : ∀ c ∈ cands, c ∈ c0 ∨ c ∈ done

theorem propInv_start (hrefl : ∀ a : S, Le a a) : PropInv Le cfg b post inv₀ c0 [] inv₀ c0 where
  grow := fun x p h => ⟨p, h, hrefl _, fun hne => absurd rfl hne⟩
  covered := by simp
  fresh := fun x p' h h' => by rw [h] at h'; cases h'
  keep := fun c hc => hc
  cands := fun c hc => Or.inl hc

theorem propInv_step {an : Analysis S} {J : Nat → S → Prop} (hJ : JoinSound an Le J)
    {done : List Nat} {inv inv' : List (Nat × S)}
    {cands cands' : List Nat} {succ : Nat} (hi : PropInv Le cfg b post inv₀ c0 done inv cands)
    (hjinv : ∀ x p, inv.lookup x = some p → J x p) (hjpost : J succ post)
    (h : propagate an cfg b post (inv, cands) succ = .ok (inv', cands')) :
    PropInv Le cfg b post inv₀ c0 (done ++ [succ]) inv' cands' := by
  unfold propagate at h
  simp only at h
  split at h
  · -- First visit: record the post-state.
    next hnone =>
    simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    refine ⟨fun x p hx => ?_, fun s hs => ?_, fun x p' hx hx0 => ?_, hi.keep, fun c hc => ?_⟩
    · obtain ⟨p', hp', hle, hch⟩ := hi.grow x p hx
      refine ⟨p', by rw [lookup_append_single, hp'], hle, fun hne => ?_⟩
      obtain ⟨hd, hc⟩ := hch hne
      exact ⟨List.mem_append_left _ hd, hc⟩
    · rcases List.mem_append.1 hs with hs | hs
      · obtain ⟨p', hp', hle⟩ := hi.covered s hs
        exact ⟨p', by rw [lookup_append_single, hp'], hle⟩
      · simp only [List.mem_singleton] at hs
        subst hs
        exact ⟨post, by rw [lookup_append_single, hnone]; simp, hJ.refl _⟩
    · rw [lookup_append_single] at hx
      cases hl : inv.lookup x with
      | some w => exact List.mem_append_left _ (hi.fresh x w hl hx0)
      | none =>
        rw [hl] at hx
        simp only at hx
        split at hx
        · next hxs => subst hxs; simp
        · simp at hx
    · rcases hi.cands c hc with h | h
      · exact Or.inl h
      · exact Or.inr (List.mem_append_left _ h)
  · -- A later visit: join into the recorded pre-state.
    next old hold =>
    simp only [bind_ok] at h
    obtain ⟨⟨joined, changed⟩, hj, h⟩ := h
    obtain ⟨hold_le, hpost_le, hunch⟩ := hJ.join (hjinv succ old hold) hjpost hj
    simp only at h
    split at h
    · next hch =>
      simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      have hcand : ∀ c ∈ cands, c ∈ (if cfg.isBackEdge b succ = true then cands ++ [succ] else cands) := by
        intro c hc; split <;> simp [hc]
      refine ⟨fun x p hx => ?_, fun s hs => ?_, fun x p' hx hx0 => ?_,
        fun c hc => hcand c (hi.keep c hc), fun c hc => ?_⟩
      · obtain ⟨p', hp', hle, hchg⟩ := hi.grow x p hx
        by_cases hxs : x = succ
        · subst hxs
          rw [hold, Option.some.injEq] at hp'
          subst hp'
          refine ⟨joined, by rw [lookup_setPre]; simp [hold], hJ.trans hle hold_le,
            fun _ => ⟨by simp, fun hback => by simp [hback]⟩⟩
        · refine ⟨p', by rw [lookup_setPre]; simp [hxs, hp'], hle, fun hne => ?_⟩
          obtain ⟨hd, hc⟩ := hchg hne
          exact ⟨List.mem_append_left _ hd, fun hback => hcand x (hc hback)⟩
      · by_cases hss : s = succ
        · subst hss
          exact ⟨joined, by rw [lookup_setPre]; simp [hold], hpost_le⟩
        · rcases List.mem_append.1 hs with hs | hs
          · obtain ⟨p', hp', hle⟩ := hi.covered s hs
            exact ⟨p', by rw [lookup_setPre]; simp [hss, hp'], hle⟩
          · simp at hs; exact absurd hs hss
      · rw [lookup_setPre] at hx
        by_cases hxs : x = succ
        · subst hxs; simp
        · simp only [hxs, ↓reduceIte] at hx
          exact List.mem_append_left _ (hi.fresh x p' hx hx0)
      · split at hc
        · rcases List.mem_append.1 hc with hc | hc
          · rcases hi.cands c hc with h | h
            · exact Or.inl h
            · exact Or.inr (List.mem_append_left _ h)
          · simp at hc; subst hc; simp
        · rcases hi.cands c hc with h | h
          · exact Or.inl h
          · exact Or.inr (List.mem_append_left _ h)
    · next hch =>
      simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      have hjo := hunch (by simpa using hch)
      subst hjo
      refine ⟨fun x p hx => ?_, fun s hs => ?_, fun x p' hx hx0 => ?_, hi.keep, fun c hc => ?_⟩
      · obtain ⟨p', hp', hle, hchg⟩ := hi.grow x p hx
        refine ⟨p', hp', hle, fun hne => ?_⟩
        obtain ⟨hd, hc⟩ := hchg hne
        exact ⟨List.mem_append_left _ hd, hc⟩
      · rcases List.mem_append.1 hs with hs | hs
        · exact hi.covered s hs
        · simp only [List.mem_singleton] at hs
          subst hs
          exact ⟨_, hold, hpost_le⟩
      · exact List.mem_append_left _ (hi.fresh x p' hx hx0)
      · rcases hi.cands c hc with h | h
        · exact Or.inl h
        · exact Or.inr (List.mem_append_left _ h)

theorem propagate_jinv {an : Analysis S} {J : Nat → S → Prop}
    (hjoinJ : ∀ s a b j changed, J s a → J s b → an.join a b = .ok (j, changed) → J s j)
    {inv inv' : List (Nat × S)} {cands cands' : List Nat} {succ : Nat}
    (hjinv : ∀ x p, inv.lookup x = some p → J x p) (hjpost : J succ post)
    (h : propagate an cfg b post (inv, cands) succ = .ok (inv', cands')) :
    ∀ x p, inv'.lookup x = some p → J x p := by
  unfold propagate at h
  simp only at h
  split at h
  · next hnone =>
    simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    intro x p hx
    rw [lookup_append_single] at hx
    cases hl : inv.lookup x with
    | some w => rw [hl] at hx; simp only [Option.some.injEq] at hx; subst hx; exact hjinv x w hl
    | none =>
      rw [hl] at hx
      simp only at hx
      split at hx
      · next hxs => subst hxs; simp only [Option.some.injEq] at hx; subst hx; exact hjpost
      · simp at hx
  · next old hold =>
    simp only [bind_ok] at h
    obtain ⟨⟨joined, changed⟩, hj, h⟩ := h
    have hjj := hjoinJ succ old post joined changed (hjinv succ old hold) hjpost hj
    simp only at h
    split at h
    · simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      intro x p hx
      rw [lookup_setPre] at hx
      by_cases hxs : x = succ
      · subst hxs; simp only [↓reduceIte, hold, Option.map_some, Option.some.injEq] at hx
        subst hx; exact hjj
      · simp only [hxs, ↓reduceIte] at hx; exact hjinv x p hx
    · simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      exact hjinv

theorem propInv_fold {an : Analysis S} {J : Nat → S → Prop} (hJ : JoinSound an Le J)
    (hjoinJ : ∀ s a b j changed, J s a → J s b → an.join a b = .ok (j, changed) → J s j) :
    ∀ (l done : List Nat) (inv inv' : List (Nat × S))
    (cands cands' : List Nat), PropInv Le cfg b post inv₀ c0 done inv cands →
    (∀ x p, inv.lookup x = some p → J x p) → (∀ s ∈ l, J s post) →
    l.foldlM (propagate an cfg b post) (inv, cands) = .ok (inv', cands') →
    PropInv Le cfg b post inv₀ c0 (done ++ l) inv' cands' ∧
      ∀ x p, inv'.lookup x = some p → J x p
  | [], done, inv, inv', cands, cands', hi, hjinv, _, h => by
    simp [pure, Except.pure] at h
    obtain ⟨rfl, rfl⟩ := h
    simpa using And.intro hi hjinv
  | succ :: l, done, inv, inv', cands, cands', hi, hjinv, hjpost, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨⟨inv1, cands1⟩, h1, h⟩ := h
    have hj1 := propagate_jinv cfg b post hjoinJ hjinv (hjpost succ (by simp)) h1
    have := propInv_fold hJ hjoinJ l (done ++ [succ]) inv1 inv' cands1 cands'
      (propInv_step Le cfg b post inv₀ c0 hJ hi hjinv (hjpost succ (by simp)) h1) hj1
      (fun s hs => hjpost s (by simp [hs])) h
    simpa using this

end Propagation



/-! ## Positions in traversal order -/

section Positions

variable {cfg : Cfg}

theorem idxOf?_of_nodup {l : List Nat} (hl : l.Nodup) {k x : Nat} (h : l[k]? = some x) :
    l.idxOf? x = some k := by
  induction l generalizing k with
  | nil => simp at h
  | cons a t ih =>
    obtain ⟨hna, hnt⟩ := List.nodup_cons.1 hl
    cases k with
    | zero =>
      simp at h
      subst h
      simp [List.idxOf?, List.findIdx?_cons]
    | succ k =>
      simp at h
      have hne : x ≠ a := fun e => hna (e ▸ List.mem_of_getElem? h)
      have hax : ¬ a = x := fun e => hne e.symm
      have := ih hnt h
      simp only [List.idxOf?, List.findIdx?_cons, beq_iff_eq] at this ⊢
      simp [hax, this]

theorem idxOf?_none {l : List Nat} {x : Nat} (h : x ∉ l) : l.idxOf? x = none := by
  simpa [List.idxOf?_eq_none_iff] using h

/-- On a duplicate-free traversal order, a block's traversal index is its
position. -/
theorem traversalIndex_eq (hnd : cfg.traversalOrder.Nodup) {k x : Nat}
    (h : cfg.traversalOrder[k]? = some x) : cfg.traversalIndex x = k := by
  have hk := (List.getElem?_eq_some_iff.1 h).1
  unfold Cfg.traversalIndex
  by_cases hlast : k + 1 = cfg.traversalOrder.length
  · have hnot : x ∉ cfg.traversalOrder.dropLast := by
      intro hm
      obtain ⟨j, hj, hjx⟩ := List.getElem_of_mem hm
      simp only [List.length_dropLast] at hj
      rw [List.getElem_dropLast] at hjx
      have := nodup_index hnd (List.getElem?_eq_some_iff.2 ⟨by omega, hjx⟩) h
      omega
    rw [idxOf?_none hnot]
    show cfg.traversalOrder.length - 1 = k
    omega
  · have hd : cfg.traversalOrder.dropLast[k]? = some x := by
      rw [List.getElem?_dropLast]
      simp [show k < cfg.traversalOrder.length - 1 by omega, h]
    rw [idxOf?_of_nodup (hnd.sublist (List.dropLast_sublist _)) hd]

theorem traversalIndex_absent {x : Nat} (h : x ∉ cfg.traversalOrder) :
    cfg.traversalIndex x = cfg.traversalOrder.length - 1 := by
  unfold Cfg.traversalIndex
  rw [idxOf?_none (fun hm => h ((List.dropLast_sublist _).subset hm))]

theorem traversalIndex_le (hnd : cfg.traversalOrder.Nodup) (x : Nat) :
    cfg.traversalIndex x ≤ cfg.traversalOrder.length - 1 := by
  by_cases hx : x ∈ cfg.traversalOrder
  · obtain ⟨k, hk, hkx⟩ := List.getElem_of_mem hx
    rw [traversalIndex_eq hnd (List.getElem?_eq_some_iff.2 ⟨hk, hkx⟩)]
    omega
  · rw [traversalIndex_absent hx]
    exact Nat.le_refl _

theorem traversalIndex_inj (hnd : cfg.traversalOrder.Nodup) {x y : Nat}
    (hx : x ∈ cfg.traversalOrder) (hy : y ∈ cfg.traversalOrder)
    (h : cfg.traversalIndex x = cfg.traversalIndex y) : x = y := by
  obtain ⟨i, hi, hix⟩ := List.getElem_of_mem hx
  obtain ⟨j, hj, hjy⟩ := List.getElem_of_mem hy
  rw [traversalIndex_eq hnd (List.getElem?_eq_some_iff.2 ⟨hi, hix⟩),
    traversalIndex_eq hnd (List.getElem?_eq_some_iff.2 ⟨hj, hjy⟩)] at h
  subst h
  rw [← hix, ← hjy]

theorem nextBlock_go {b : Nat} : ∀ (l : List Nat) {c : Nat}, Cfg.nextBlock.go b l = some c →
    ∃ k, l[k]? = some b ∧ l[k + 1]? = some c
  | [], _, h => by simp [Cfg.nextBlock.go] at h
  | [_], _, h => by simp [Cfg.nextBlock.go] at h
  | a :: d :: rest, c, h => by
    simp only [Cfg.nextBlock.go] at h
    split at h
    · next hab =>
      simp only [beq_iff_eq] at hab
      simp only [Option.some.injEq] at h
      subst hab h
      exact ⟨0, rfl, rfl⟩
    · obtain ⟨k, hk, hk1⟩ := nextBlock_go (d :: rest) h
      exact ⟨k + 1, by simpa using hk, by simpa using hk1⟩

theorem nextBlock_go_none {b : Nat} : ∀ (l : List Nat), Cfg.nextBlock.go b l = none →
    ∀ k, l[k]? = some b → l[k + 1]? = none
  | [], _, k, hk => by simp at hk
  | [_], _, k, hk => by
    cases k with
    | zero => simp
    | succ k => simp at hk
  | a :: d :: rest, h, k, hk => by
    simp only [Cfg.nextBlock.go] at h
    split at h
    · simp at h
    · next hab =>
      cases k with
      | zero =>
        simp at hk; subst hk; simp at hab
      | succ k =>
        have := nextBlock_go_none (d :: rest) h k (by simpa using hk)
        simpa using this

/-- The next block in traversal order is one position later. -/
theorem traversalIndex_next (hnd : cfg.traversalOrder.Nodup) {b c : Nat}
    (h : cfg.nextBlock b = some c) :
    c ∈ cfg.traversalOrder ∧ cfg.traversalIndex c = cfg.traversalIndex b + 1 := by
  obtain ⟨k, hk, hk1⟩ := nextBlock_go _ h
  exact ⟨List.mem_of_getElem? hk1, by
    rw [traversalIndex_eq hnd hk1, traversalIndex_eq hnd hk]⟩

/-- Without a next block, a block's traversal index is the largest. -/
theorem traversalIndex_last (hnd : cfg.traversalOrder.Nodup) {b : Nat}
    (h : cfg.nextBlock b = none) (x : Nat) : cfg.traversalIndex x ≤ cfg.traversalIndex b := by
  have hle := traversalIndex_le hnd x
  by_cases hb : b ∈ cfg.traversalOrder
  · obtain ⟨k, hk, hkb⟩ := List.getElem_of_mem hb
    have hk' : cfg.traversalOrder[k]? = some b := List.getElem?_eq_some_iff.2 ⟨hk, hkb⟩
    have hnone := nextBlock_go_none _ h k hk'
    rw [List.getElem?_eq_none_iff] at hnone
    rw [traversalIndex_eq hnd hk']
    omega
  · rw [traversalIndex_absent hb]; exact hle

theorem firstInTraversal_none : ∀ {cands : List Nat}, firstInTraversal cfg cands = none →
    cands = []
  | [], _ => rfl
  | c :: cs, h => by
    simp only [firstInTraversal] at h
    split at h <;> (try split at h) <;> simp at h

theorem firstInTraversal_some : ∀ {cands : List Nat} {n : Nat},
    firstInTraversal cfg cands = some n →
      n ∈ cands ∧ ∀ c ∈ cands, cfg.traversalIndex n ≤ cfg.traversalIndex c
  | [], n, h => by simp [firstInTraversal] at h
  | c :: cs, n, h => by
    simp only [firstInTraversal] at h
    split at h
    · simp only [Option.some.injEq] at h
      subst h
      next hnone =>
      refine ⟨List.mem_cons_self .., fun d hd => ?_⟩
      rcases List.mem_cons.1 hd with rfl | hd
      · exact Nat.le_refl _
      · have := firstInTraversal_none hnone
        subst this
        simp at hd
    · next d hd =>
      obtain ⟨hdm, hdle⟩ := firstInTraversal_some hd
      split at h
      · simp only [Option.some.injEq] at h
        subst h
        refine ⟨List.mem_cons_of_mem _ hdm, fun e he => ?_⟩
        rcases List.mem_cons.1 he with rfl | he
        · omega
        · exact hdle e he
      · simp only [Option.some.injEq] at h
        subst h
        refine ⟨List.mem_cons_self .., fun e he => ?_⟩
        rcases List.mem_cons.1 he with rfl | he
        · exact Nat.le_refl _
        · have := hdle e he; omega

end Positions



/-! ## The sweep -/

/-- What the scheduler needs of the depth-first traversal: the order has no
duplicates and contains the entry; it is closed under successors; an edge
that is not a recorded back edge goes forward in the order; and every block
but the entry is reached by such an edge from an earlier block. -/
structure DfsFacts (cfg : Cfg) : Prop where
  nodup : cfg.traversalOrder.Nodup
  entry : 0 ∈ cfg.traversalOrder
  closed : ∀ b ∈ cfg.traversalOrder, ∀ s ∈ cfg.successors b, s ∈ cfg.traversalOrder
  forward : ∀ b ∈ cfg.traversalOrder, ∀ s ∈ cfg.successors b, cfg.isBackEdge b s = false →
    cfg.traversalIndex b < cfg.traversalIndex s
  parent : ∀ x ∈ cfg.traversalOrder, x ≠ 0 →
    ∃ y ∈ cfg.traversalOrder, x ∈ cfg.successors y ∧ cfg.isBackEdge y x = false

section Sweep

/-- A block whose recorded pre-state executes, with the post-state at most
the recorded pre-state of every successor. -/
def Stable {S : Type} (an : Analysis S) (Le : S → S → Prop) (cfg : Cfg) (code : Array Bytecode)
    (inv : List (Nat × S)) (x : Nat) : Prop :=
  ∃ pre, inv.lookup x = some pre ∧ ∃ post,
    executeBlock an cfg code x pre = .ok post ∧
    ∀ s ∈ cfg.successors x, ∃ pre', inv.lookup s = some pre' ∧ Le post pre'

/-- The invariant of the main loop, about to analyze block `b`: every
analyzed block before `b` in traversal order is stable. -/
structure LoopInv {S : Type} (an : Analysis S) (Le : S → S → Prop) (J : Nat → S → Prop)
    (cfg : Cfg) (code : Array Bytecode) (initial : S) (inv : List (Nat × S)) (b : Nat) : Prop where
  jinv : ∀ x pre, inv.lookup x = some pre → J x pre
  entry : ∃ pre, inv.lookup 0 = some pre ∧ Le initial pre
  dom : ∀ x pre, inv.lookup x = some pre → x ∈ cfg.traversalOrder
  cur : b ∈ cfg.traversalOrder
  stable : ∀ x pre, inv.lookup x = some pre → cfg.traversalIndex x < cfg.traversalIndex b →
    Stable an Le cfg code inv x

variable {S : Type} {an : Analysis S} {Le : S → S → Prop} {J : Nat → S → Prop} {cfg : Cfg}
  {code : Array Bytecode} {initial : S}

theorem stable_mono (hJ : JoinSound an Le J) {inv inv' : List (Nat × S)} {x : Nat}
    (hs : Stable an Le cfg code inv x) (hx : inv'.lookup x = inv.lookup x)
    (hgrow : ∀ y p, inv.lookup y = some p → ∃ p', inv'.lookup y = some p' ∧ Le p p') :
    Stable an Le cfg code inv' x := by
  obtain ⟨pre, hpre, post, hex, hsucc⟩ := hs
  refine ⟨pre, hx.trans hpre, post, hex, fun s hs => ?_⟩
  obtain ⟨p, hp, hle⟩ := hsucc s hs
  obtain ⟨p', hp', hle'⟩ := hgrow s p hp
  exact ⟨p', hp', hJ.trans hle hle'⟩

/-- Every block before `b` in traversal order is analyzed. -/
theorem analyzed_before (hd : DfsFacts cfg) {inv : List (Nat × S)} {b : Nat}
    (hi : LoopInv an Le J cfg code initial inv b) :
    ∀ N x, cfg.traversalIndex x < N → x ∈ cfg.traversalOrder →
      cfg.traversalIndex x < cfg.traversalIndex b → ∃ pre, inv.lookup x = some pre := by
  intro N
  induction N with
  | zero => intro x hn; omega
  | succ N ih =>
    intro x hn hx hlt
    by_cases h0 : x = 0
    · subst h0
      obtain ⟨pre, hpre, -⟩ := hi.entry
      exact ⟨pre, hpre⟩
    · obtain ⟨y, hy, hxy, hback⟩ := hd.parent x hx h0
      have hfwd := hd.forward y hy x hxy hback
      obtain ⟨py, hpy⟩ := ih y (by omega) hy (by omega)
      obtain ⟨pre, hpre, post, -, hsucc⟩ := hi.stable y py hpy (by omega)
      obtain ⟨p', hp', -⟩ := hsucc x hxy
      exact ⟨p', hp'⟩

theorem stable_of_loopInv (hd : DfsFacts cfg) {inv : List (Nat × S)} {b : Nat}
    (hi : LoopInv an Le J cfg code initial inv b) (hlast : cfg.nextBlock b = none)
    (hb : inv.lookup b = none) : PostFixpointOf an Le cfg code initial inv := by
  refine ⟨hi.entry, fun x pre hx => ?_⟩
  have hxo := hi.dom x pre hx
  have hne : x ≠ b := fun e => by subst e; rw [hb] at hx; cases hx
  have hle := traversalIndex_last hd.nodup hlast x
  have hlt : cfg.traversalIndex x < cfg.traversalIndex b := by
    rcases Nat.lt_or_ge (cfg.traversalIndex x) (cfg.traversalIndex b) with h | h
    · exact h
    · exact absurd (traversalIndex_inj hd.nodup hxo hi.cur (by omega)) hne
  obtain ⟨pre', hpre', post, hex, hsucc⟩ := hi.stable x pre hx hlt
  rw [hx, Option.some.injEq] at hpre'
  subst hpre'
  exact ⟨post, hex, hsucc⟩

/-- One analyzed block: after propagating its post-state, every analyzed
block before the next one in traversal order is stable, or, when there is no
next one, every analyzed block is. -/
theorem step_inv (hJ : JoinSound an Le J) (hd : DfsFacts cfg) {inv inv' : List (Nat × S)}
    {b : Nat} {pre post : S} {cands : List Nat} (hi : LoopInv an Le J cfg code initial inv b)
    (hpre : inv.lookup b = some pre) (hex : executeBlock an cfg code b pre = .ok post)
    (hprop : PropInv Le cfg b post inv (match cfg.nextBlock b with
        | some n => [n]
        | none => []) (cfg.successors b) inv' cands) :
    (∃ pre, inv'.lookup 0 = some pre ∧ Le initial pre) ∧
    (∀ x pre, inv'.lookup x = some pre → x ∈ cfg.traversalOrder) ∧
    (∀ c ∈ cands, c ∈ cfg.traversalOrder) ∧
    ∀ q, (cfg.nextBlock b = none ∨ q ≤ cfg.traversalIndex b + 1) →
      (∀ c ∈ cands, q ≤ cfg.traversalIndex c) →
      ∀ x pre, inv'.lookup x = some pre → cfg.traversalIndex x < q →
        Stable an Le cfg code inv' x := by
  have hbo := hi.dom b pre hpre
  have hgrow : ∀ y p, inv.lookup y = some p → ∃ p', inv'.lookup y = some p' ∧ Le p p' :=
    fun y p hy => let ⟨p', h1, h2, _⟩ := hprop.grow y p hy; ⟨p', h1, h2⟩
  refine ⟨?_, fun x p hx => ?_, fun c hc => ?_, fun q hq hqc x p' hx hxq => ?_⟩
  · obtain ⟨p0, hp0, hle⟩ := hi.entry
    obtain ⟨p0', hp0', hle'⟩ := hgrow 0 p0 hp0
    exact ⟨p0', hp0', hJ.trans hle hle'⟩
  · cases hxi : inv.lookup x with
    | some w => exact hi.dom x w hxi
    | none => exact hd.closed b hbo x (hprop.fresh x p hx hxi)
  · rcases hprop.cands c hc with h | h
    · split at h
      · next n hn =>
        simp only [List.mem_singleton] at h
        subst h
        exact (traversalIndex_next hd.nodup hn).1
      · simp at h
    · exact hd.closed b hbo c h
  · -- Every analyzed block before `q` is at most `b` in traversal order.
    have hxo : x ∈ cfg.traversalOrder := by
      cases hxi : inv.lookup x with
      | some w => exact hi.dom x w hxi
      | none => exact hd.closed b hbo x (hprop.fresh x p' hx hxi)
    have hxb : cfg.traversalIndex x ≤ cfg.traversalIndex b := by
      rcases hq with hq | hq
      · exact traversalIndex_last hd.nodup hq x
      · omega
    cases hxi : inv.lookup x with
    | none =>
      -- A block first reached now is after `b`.
      exfalso
      have hxs := hprop.fresh x p' hx hxi
      rcases Nat.lt_or_ge (cfg.traversalIndex x) (cfg.traversalIndex b) with hlt | hge
      · obtain ⟨w, hw⟩ := analyzed_before hd hi _ x (Nat.lt_succ_self _) hxo hlt
        rw [hw] at hxi; cases hxi
      · have := traversalIndex_inj hd.nodup hxo hbo (by omega)
        subst this
        rw [hpre] at hxi; cases hxi
    | some w =>
      obtain ⟨w', hw', hle, hchg⟩ := hprop.grow x w hxi
      have hpw : w' = p' := Option.some.inj (hw'.symm.trans hx)
      rw [hpw] at hle hchg
      by_cases hsame : p' = w
      · subst hsame
        by_cases hxbeq : x = b
        · subst hxbeq
          rw [hpre, Option.some.injEq] at hxi
          subst hxi
          exact ⟨_, hx, post, hex, fun s hs => hprop.covered s hs⟩
        · have hlt : cfg.traversalIndex x < cfg.traversalIndex b := by
            rcases Nat.lt_or_ge (cfg.traversalIndex x) (cfg.traversalIndex b) with h | h
            · exact h
            · exact absurd (traversalIndex_inj hd.nodup hxo hbo (by omega)) hxbeq
          exact stable_mono hJ (hi.stable x p' hxi hlt) (by rw [hx, hxi]) hgrow
      · -- A pre-state that changed is a successor of `b`; it is not before
        -- `q`, being either after `b` or a back-edge target.
        exfalso
        obtain ⟨hxs, hcand⟩ := hchg hsame
        cases hback : cfg.isBackEdge b x with
        | true => have := hqc x (hcand hback); omega
        | false => have := hd.forward b hbo x hxs hback; omega

/-- **The scheduler computes a post-fixpoint**, given the traversal facts. -/
theorem loop_postFixpoint (hJ : JoinSound an Le J) (hJc : JClosed an cfg code J)
    (hd : DfsFacts cfg) :
    ∀ (fuel : Nat) (inv : List (Nat × S)) (b : Nat) (r : List (Nat × S)),
      LoopInv an Le J cfg code initial inv b →
      analyzeLoop an cfg code fuel inv (some b) = .ok r →
      PostFixpointOf an Le cfg code initial r := by
  intro fuel
  induction fuel with
  | zero => intro inv b r _ h; simp [analyzeLoop, outOfFuel] at h
  | succ fuel ih =>
    intro inv b r hi h
    simp only [analyzeLoop] at h
    split at h
    · -- Not analyzed yet: move on.
      next hb =>
      cases hn : cfg.nextBlock b with
      | none =>
        rw [hn] at h
        cases fuel <;> simp [analyzeLoop, pure, Except.pure] at h <;>
          (subst h; exact stable_of_loopInv hd hi hn hb)
      | some c =>
        rw [hn] at h
        obtain ⟨hco, hpos⟩ := traversalIndex_next hd.nodup hn
        refine ih inv c r ⟨hi.jinv, hi.entry, hi.dom, hco, fun x p hx hlt => ?_⟩ h
        have hxo := hi.dom x p hx
        have hne : x ≠ b := fun e => by subst e; rw [hb] at hx; cases hx
        have : cfg.traversalIndex x < cfg.traversalIndex b := by
          rcases Nat.lt_or_ge (cfg.traversalIndex x) (cfg.traversalIndex b) with h' | h'
          · exact h'
          · exact absurd (traversalIndex_inj hd.nodup hxo hi.cur (by omega)) hne
        exact hi.stable x p hx this
    · -- Analyze `b`.
      next pre hpre =>
      simp only [bind_ok] at h
      obtain ⟨post, hex, ⟨inv', cands⟩, hfold, h⟩ := h
      obtain ⟨hprop, hjinv'⟩ := propInv_fold Le cfg b post inv _ hJ hJc.join (cfg.successors b) []
        inv inv' _ cands (propInv_start Le cfg b post inv _ hJ.refl) hi.jinv
        (hJc.exec b pre post (hi.jinv b pre hpre) hex) hfold
      simp only [List.nil_append] at hprop
      obtain ⟨hentry, hdom, hcands, hstab⟩ := step_inv hJ hd hi hpre hex hprop
      cases hfirst : firstInTraversal cfg cands with
      | none =>
        rw [hfirst] at h
        have hnil := firstInTraversal_none hfirst
        subst hnil
        have hlast : cfg.nextBlock b = none := by
          cases hn : cfg.nextBlock b with
          | none => rfl
          | some c => have := hprop.keep c (by simp [hn]); simp at this
        have hfin : PostFixpointOf an Le cfg code initial inv' := by
          refine ⟨hentry, fun x p hx => ?_⟩
          have hxo := hdom x p hx
          have hle := traversalIndex_le hd.nodup x
          have hpos := List.length_pos_of_mem hxo
          obtain ⟨p', hp', post', hex', hs'⟩ :=
            hstab cfg.traversalOrder.length (Or.inl hlast) (by simp) x p hx (by omega)
          rw [hx, Option.some.injEq] at hp'
          subst hp'
          exact ⟨post', hex', hs'⟩
        cases fuel <;> simp [analyzeLoop, pure, Except.pure] at h <;> subst h <;> exact hfin
      | some n =>
        rw [hfirst] at h
        obtain ⟨hnm, hnmin⟩ := firstInTraversal_some hfirst
        refine ih inv' n r ⟨hjinv', hentry, hdom, hcands n hnm, fun x p hx hlt => ?_⟩ h
        refine hstab (cfg.traversalIndex n) ?_ hnmin x p hx hlt
        cases hn : cfg.nextBlock b with
        | none => exact Or.inl rfl
        | some c =>
          right
          have := hnmin c (hprop.keep c (by simp [hn]))
          have := (traversalIndex_next hd.nodup hn).2
          omega

end Sweep


/-- **The scheduler computes a post-fixpoint** of any analysis whose join is
sound for its order. -/
theorem analyzeFunctionOf_postFixpoint {S : Type} {an : Analysis S} {Le : S → S → Prop}
    {J : Nat → S → Prop} (hJ : JoinSound an Le J) {cfg : Cfg} {code : Array Bytecode}
    (hJc : JClosed an cfg code J) {initial : S} (hinit : J 0 initial)
    {height : Nat} {inv : List (Nat × S)} (hd : DfsFacts cfg)
    (h : analyzeFunction an cfg code initial height = .ok inv) :
    PostFixpointOf an Le cfg code initial inv := by
  unfold analyzeFunction at h
  refine loop_postFixpoint hJ hJc hd _ [(0, initial)] 0 inv ⟨fun x p hx => ?_,
    ⟨initial, by simp, hJ.refl _⟩, fun x p hx => ?_, hd.entry, fun x p hx hlt => ?_⟩ h
  · by_cases hx0 : x = 0
    · subst hx0; simp at hx; subst hx; exact hinit
    · rw [lookup_cons_ne _ _ hx0] at hx; simp at hx
  · by_cases hx0 : x = 0
    · subst hx0; exact hd.entry
    · rw [lookup_cons_ne _ _ hx0] at hx; simp at hx
  · by_cases hx0 : x = 0
    · subst hx0; omega
    · rw [lookup_cons_ne _ _ hx0] at hx; simp at hx

theorem analyzeFunction_postFixpoint {cfg : Cfg} {code : Array Bytecode} {initial : LocalsState}
    {height : Nat} {inv : List (Nat × LocalsState)} (hd : DfsFacts cfg)
    (h : analyzeFunction localsAnalysis cfg code initial height = .ok inv) :
    PostFixpoint cfg code initial inv :=
  let h := analyzeFunctionOf_postFixpoint joinSound_locals (jClosed_true _ _ _) trivial hd h
  ⟨h.entry, h.closed⟩

/-- The abstract interpreter is sound on every control-flow graph whose
traversal has the depth-first properties. -/
theorem schedulerSound_of_dfs (hdfs : ∀ code cfg, Cfg.build code = .ok cfg → DfsFacts cfg) :
    SchedulerSound :=
  fun code cfg _ _ _ hb ha => analyzeFunction_postFixpoint (hdfs code cfg hb) ha

end MoveBytecode.Proofs
