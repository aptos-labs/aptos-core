-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.StackUsage

/-!
# The borrow graph, semantically

UNTRUSTED (checked by the kernel). What the operations of the borrow graph
(`Model/BorrowGraph.lean`) do to its edges, read through lookups
(`Graph.Edge`). The reference-safety proofs use two kinds of facts:

- **Covering**: an operation keeps, for every edge it does not mean to
  remove, an edge between the same references that covers it (`Edge.leq`: the
  same edge, or a weak edge with a prefix of its path). A covering edge is
  weaker, so whatever an edge says about concrete locations, a covering edge
  says too. Collapsing a full edge set to one weak edge with the empty path is
  covering.
- **New edges**: the edges an operation adds, up to covering.

Neither kind says that an edge is absent. Absence is what the verifier's
queries (`hasFullBorrows`, `hasConsistentBorrows`, ...) report, and the
proofs read it from their results.

Maps are association lists read with `find?`; the operations keep their keys
strictly increasing (`Graph.WF`), which `remap` needs.
-/

namespace MoveBytecode.Model

open MoveBytecode.Proofs

namespace BorrowGraph

/-! ## Maps -/

namespace NatMap

theorem find?_insert {α : Type} : ∀ (m : NatMap α) (k : Nat) (v : α) (x : Nat),
    (m.insert k v).find? x = if x = k then some v else m.find? x
  | [], k, v, x => by
    by_cases h : x = k
    · subst h; simp [NatMap.insert, NatMap.find?]
    · have hb : (x == k) = false := by simpa using h
      simp [NatMap.insert, NatMap.find?, List.lookup, hb, h]
  | (k', v') :: rest, k, v, x => by
    unfold NatMap.insert
    by_cases hlt : k < k'
    · by_cases h : x = k
      · subst h; simp [hlt, NatMap.find?]
      · simp only [hlt, ↓reduceIte, NatMap.find?, List.lookup, h]
        have : (x == k) = false := by simpa using h
        simp [this]
    · by_cases heq : k = k'
      · subst heq
        by_cases h : x = k
        · subst h; simp [NatMap.find?]
        · have : (x == k) = false := by simpa using h
          simp [NatMap.find?, List.lookup, this, h]
      · have hne : (k == k') = false := by simpa using heq
        simp only [hlt, ↓reduceIte, hne, Bool.false_eq_true]
        have ih := find?_insert rest k v x
        simp only [NatMap.find?] at ih ⊢
        by_cases h : x = k'
        · subst h
          have : x ≠ k := fun e => heq e.symm
          simp [List.lookup, this]
        · have : (x == k') = false := by simpa using h
          simp [List.lookup, this, ih]

theorem find?_erase {α : Type} (m : NatMap α) (k x : Nat) :
    (m.erase k).find? x = if x = k then none else m.find? x := by
  induction m with
  | nil => simp [NatMap.erase, NatMap.find?, List.lookup]
  | cons e rest ih =>
    obtain ⟨k', v'⟩ := e
    simp only [NatMap.erase, NatMap.find?] at ih ⊢
    rw [List.filter_cons]
    by_cases hk : k' = k
    · subst hk
      simp only [bne_self_eq_false, Bool.false_eq_true, ↓reduceIte]
      rw [ih]
      by_cases h : x = k'
      · simp [h]
      · have hb : (x == k') = false := by simpa using h
        simp [h, List.lookup, hb]
    · have : (k' != k) = true := by simpa using hk
      simp only [this, ↓reduceIte]
      by_cases h : x = k'
      · subst h; simp [List.lookup, hk]
      · have hb : (x == k') = false := by simpa using h
        simp only [List.lookup, hb]
        exact ih

end NatMap

/-! ## Edges and edge sets -/

theorem isPrefix_refl : ∀ p : List Label, isPrefix p p = true
  | [] => rfl
  | a :: as => by simp [isPrefix, isPrefix_refl as]

theorem isPrefix_trans : ∀ {p q r : List Label}, isPrefix p q = true → isPrefix q r = true →
    isPrefix p r = true
  | [], _, _, _, _ => rfl
  | _ :: _, [], _, h, _ => by simp [isPrefix] at h
  | _ :: _, _ :: _, [], _, h => by simp [isPrefix] at h
  | a :: as, b :: bs, c :: cs, h1, h2 => by
    simp only [isPrefix, Bool.and_eq_true, beq_iff_eq] at h1 h2 ⊢
    exact ⟨h1.1.trans h2.1, isPrefix_trans h1.2 h2.2⟩

theorem isPrefix_iff : ∀ {p q : List Label}, isPrefix p q = true ↔ p <+: q
  | [], q => by simp [isPrefix]
  | a :: as, [] => by simp [isPrefix]
  | a :: as, b :: bs => by
    simp only [isPrefix, Bool.and_eq_true, beq_iff_eq, List.cons_prefix_cons, isPrefix_iff]

theorem isPrefix_append (p q : List Label) : isPrefix p (p ++ q) = true :=
  isPrefix_iff.2 (List.prefix_append p q)

theorem Edge.leq_refl (e : Edge) : e.leq e = true := by simp [Edge.leq]

theorem Edge.leq_trans {a b c : Edge} (h1 : a.leq b = true) (h2 : b.leq c = true) :
    a.leq c = true := by
  simp only [Edge.leq, Bool.or_eq_true, beq_iff_eq, Bool.and_eq_true, Bool.not_eq_true'] at h1 h2 ⊢
  rcases h1 with rfl | ⟨ha, hab⟩
  · exact h2
  · rcases h2 with rfl | ⟨_, hbc⟩
    · exact Or.inr ⟨ha, hab⟩
    · exact Or.inr ⟨ha, isPrefix_trans hab hbc⟩

/-- The weak edge with the empty path covers every edge. -/
theorem Edge.leq_full (e : Edge) : (⟨false, []⟩ : Edge).leq e = true := by
  simp [Edge.leq, isPrefix]

theorem insertEdge_mem (e : Edge) : ∀ (es : List Edge), ∃ e' ∈ insertEdge e es, e'.leq e = true
  | [] => ⟨e, by simp [insertEdge], Edge.leq_refl e⟩
  | x :: xs => by
    unfold insertEdge
    split
    · exact ⟨e, by simp, Edge.leq_refl e⟩
    · next heq =>
      -- Equal in the derived order means the same edge.
      refine ⟨x, by simp, ?_⟩
      have : x = e := by
        obtain ⟨s1, p1⟩ := e
        obtain ⟨s2, p2⟩ := x
        simp only [Edge.cmp] at heq
        have hp : ∀ (p q : List Label), pathCmp p q = .eq → p = q := by
          intro p
          induction p with
          | nil => intro q h; cases q <;> simp_all [pathCmp]
          | cons a as ih =>
            intro q h
            cases q with
            | nil => simp [pathCmp] at h
            | cons b bs =>
              simp only [pathCmp, Ordering.then_eq_eq] at h
              obtain ⟨hab, hrest⟩ := h
              have : a = b := by
                cases a <;> cases b <;> simp_all [Label.cmp]
              rw [this, ih bs hrest]
        simp only [Ordering.then_eq_eq] at heq
        obtain ⟨hs, hpth⟩ := heq
        have : s1 = s2 := by cases s1 <;> cases s2 <;> simp_all [compare]
        rw [this, hp _ _ hpth]
      rw [this]; exact Edge.leq_refl e
    · obtain ⟨e', he', hle⟩ := insertEdge_mem e xs
      exact ⟨e', List.mem_cons_of_mem _ he', hle⟩

theorem insertEdge_keep (e : Edge) : ∀ (es : List Edge), ∀ x ∈ es, x ∈ insertEdge e es
  | [], x, hx => by simp at hx
  | y :: ys, x, hx => by
    unfold insertEdge
    split
    · exact List.mem_cons_of_mem _ hx
    · exact hx
    · rcases List.mem_cons.1 hx with rfl | hx
      · simp
      · exact List.mem_cons_of_mem _ (insertEdge_keep e ys x hx)


theorem mem_insertNat (x p : Nat) : ∀ l : List Nat, x ∈ insertNat p l ↔ x = p ∨ x ∈ l
  | [] => by simp [insertNat]
  | y :: ys => by
    unfold insertNat
    by_cases h1 : p < y
    · simp [h1]
    · by_cases h2 : p = y
      · subst h2; simp
      · have : (p == y) = false := by simpa using h2
        simp only [h1, ↓reduceIte, this, Bool.false_eq_true, List.mem_cons, mem_insertNat x p ys]
        constructor
        · rintro (h | h | h)
          · exact Or.inr (Or.inl h)
          · exact Or.inl h
          · exact Or.inr (Or.inr h)
        · rintro (h | h | h)
          · exact Or.inr (Or.inl h)
          · exact Or.inl h
          · exact Or.inr (Or.inr h)

/-- An edge set that has collapsed is the weak edge with the empty path. -/
def EdgeSet.WF (s : EdgeSet) : Prop := s.overflown = true → s.edges = [⟨false, []⟩]

theorem EdgeSet.wf_empty : EdgeSet.WF {} := by simp [EdgeSet.WF]

theorem EdgeSet.insert_wf {s : EdgeSet} (h : s.WF) (e : Edge) : (s.insert e).WF := by
  unfold EdgeSet.insert EdgeSet.WF
  split
  · exact h
  · split
    · simp
    · intro ho; exact absurd ho (by simp_all)

theorem EdgeSet.remove_wf {s : EdgeSet} (h : s.WF) (e : Edge) (hne : (s.remove e).edges ≠ []) :
    (s.remove e).WF := by
  intro ho
  have := h ho
  simp only [EdgeSet.remove] at hne ⊢
  rw [this] at hne ⊢
  by_cases he : e = ⟨false, []⟩
  · subst he; simp at hne
  · rw [List.erase_of_not_mem (by simpa [eq_comm] using he)]

/-- Inserting into an edge set adds an edge covering the new one, and keeps a
covering edge for each old one. -/
theorem EdgeSet.insert_covers {s : EdgeSet} (h : s.WF) (e : Edge) :
    (∃ e' ∈ (s.insert e).edges, e'.leq e = true) ∧
      ∀ x ∈ s.edges, ∃ e' ∈ (s.insert e).edges, e'.leq x = true := by
  unfold EdgeSet.insert
  split
  · next ho =>
    exact ⟨⟨_, by rw [h ho]; simp, Edge.leq_full e⟩, fun x hx => ⟨x, hx, Edge.leq_refl x⟩⟩
  · split
    · exact ⟨⟨_, by simp, Edge.leq_full e⟩, fun x _ => ⟨_, by simp, Edge.leq_full x⟩⟩
    · exact ⟨insertEdge_mem e s.edges, fun x hx => ⟨x, insertEdge_keep e _ x hx, Edge.leq_refl x⟩⟩

end BorrowGraph

open BorrowGraph

/-! ## The edges of a graph -/

namespace Graph

/-- `g` has edge `e` from reference `p` to reference `c`: `c` borrows from
`p` along `e`, and records that it does. An edge `c` does not record is not
one: releasing `c` would not see it. -/
def Edge (g : Graph) (p : Nat) (e : BorrowGraph.Edge) (c : Nat) : Prop :=
  ∃ pi set ci, g.find? p = some pi ∧ pi.borrowedBy.find? c = some set ∧ e ∈ set.edges ∧
    g.find? c = some ci ∧ p ∈ ci.borrowsFrom

/-- Every edge of `g` between references satisfying `P` has a covering edge
in `g'`. -/
def CoversOn (P : Nat → Nat → Prop) (g g' : Graph) : Prop :=
  ∀ p e c, P p c → g.Edge p e c → ∃ e', g'.Edge p e' c ∧ e'.leq e = true

/-- Every edge of `g` has a covering edge in `g'`. -/
def Covers (g g' : Graph) : Prop := CoversOn (fun _ _ => True) g g'

theorem Covers.refl (g : Graph) : g.Covers g :=
  fun _ e _ _ h => ⟨e, h, Edge.leq_refl e⟩

theorem CoversOn.trans {P : Nat → Nat → Prop} {g₁ g₂ g₃ : Graph} (h1 : CoversOn P g₁ g₂)
    (h2 : CoversOn P g₂ g₃) : CoversOn P g₁ g₃ := by
  intro p e c hp he
  obtain ⟨e', he', hle⟩ := h1 p e c hp he
  obtain ⟨e'', he'', hle'⟩ := h2 p e' c hp he'
  exact ⟨e'', he'', Edge.leq_trans hle' hle⟩

theorem Covers.trans {g₁ g₂ g₃ : Graph} (h1 : g₁.Covers g₂) (h2 : g₂.Covers g₃) : g₁.Covers g₃ :=
  CoversOn.trans h1 h2

theorem Covers.on {P : Nat → Nat → Prop} {g g' : Graph} (h : g.Covers g') : CoversOn P g g' :=
  fun p e c _ he => h p e c trivial he

/-- The references of `g` with their mutability. -/
def mutability (g : Graph) (x : Nat) : Option Bool := (g.find? x).map (·.mutable)

/-- Every edge set of `g` is well formed. -/
def SetsWF (g : Graph) : Prop :=
  ∀ p pi c s, g.find? p = some pi → pi.borrowedBy.find? c = some s → s.WF

/-- Every reference's borrowers are keyed in strictly increasing order. -/
def InnerSorted (g : Graph) : Prop :=
  ∀ p pi, g.find? p = some pi → (pi.borrowedBy.map (·.1)).Pairwise (· < ·)

/-- **Well-formed graphs**: keys strictly increasing, each reference's
borrowers too, edge sets well formed. -/
structure WF (g : Graph) : Prop where
  sorted : (g.map (·.1)).Pairwise (· < ·)
  sets : g.SetsWF
  inner : g.InnerSorted

end Graph

/-! ## Sorted maps -/

theorem insert_keys_pairwise {α : Type} : ∀ (m : NatMap α) (k : Nat) (v : α),
    (m.map (·.1)).Pairwise (· < ·) → ((m.insert k v).map (·.1)).Pairwise (· < ·) ∧
      ∀ x ∈ (m.insert k v).map (·.1), x = k ∨ x ∈ m.map (·.1)
  | [], k, v, _ => by simp [NatMap.insert]
  | (k', v') :: rest, k, v, h => by
    simp only [List.map_cons, List.pairwise_cons] at h
    obtain ⟨hlt, hrest⟩ := h
    unfold NatMap.insert
    by_cases h1 : k < k'
    · simp only [h1, ↓reduceIte, List.map_cons, List.pairwise_cons, List.mem_cons]
      refine ⟨⟨fun x hx => ?_, hlt, hrest⟩, fun x hx => ?_⟩
      · rcases hx with rfl | hx
        · exact h1
        · exact Nat.lt_trans h1 (hlt x hx)
      · rcases hx with rfl | hx
        · exact Or.inl rfl
        · exact Or.inr hx
    · by_cases h2 : k = k'
      · subst h2
        simp only [beq_self_eq_true, ↓reduceIte, h1, List.map_cons, List.pairwise_cons,
          List.mem_cons]
        exact ⟨⟨hlt, hrest⟩, fun x hx => by rcases hx with rfl | hx <;> simp_all⟩
      · have hne : (k == k') = false := by simpa using h2
        simp only [h1, ↓reduceIte, hne, Bool.false_eq_true, List.map_cons, List.pairwise_cons,
          List.mem_cons]
        obtain ⟨ih1, ih2⟩ := insert_keys_pairwise rest k v hrest
        refine ⟨⟨fun x hx => ?_, ih1⟩, fun x hx => ?_⟩
        · rcases ih2 x hx with rfl | hx
          · omega
          · exact hlt x hx
        · rcases hx with rfl | hx
          · exact Or.inr (Or.inl rfl)
          · rcases ih2 x hx with rfl | hx
            · exact Or.inl rfl
            · exact Or.inr (Or.inr hx)

theorem erase_keys_pairwise {α : Type} (m : NatMap α) (k : Nat)
    (h : (m.map (·.1)).Pairwise (· < ·)) : ((m.erase k).map (·.1)).Pairwise (· < ·) := by
  unfold NatMap.erase
  rw [← List.filterMap_eq_map] at h ⊢
  rw [List.filterMap_eq_map] at h ⊢
  exact List.Pairwise.sublist (List.Sublist.map _ (List.filter_sublist)) h


/-! ## New references -/

namespace Graph

theorem newRef_ok {g g' : Graph} {id : Nat} {m : Bool} (h : g.newRef id m = .ok g') :
    g.find? id = none ∧ g' = g.insert id { mutable := m } := by
  unfold newRef at h
  split at h
  · simp [panicAt] at h
  · next hn =>
    simp only [pure, Except.pure, Except.ok.injEq] at h
    exact ⟨by simpa using hn, h.symm⟩

theorem newRef_find {g g' : Graph} {id : Nat} {m : Bool} (h : g.newRef id m = .ok g') (x : Nat) :
    g'.find? x = if x = id then some { mutable := m } else g.find? x := by
  obtain ⟨-, rfl⟩ := newRef_ok h
  exact NatMap.find?_insert _ _ _ _

theorem newRef_edge {g g' : Graph} {id : Nat} {m : Bool} (h : g.newRef id m = .ok g')
    {p : Nat} {e : BorrowGraph.Edge} {c : Nat} : g'.Edge p e c ↔ g.Edge p e c := by
  have hf := newRef_find h
  have hn := (newRef_ok h).1
  unfold Edge
  simp only [hf]
  by_cases hp : p = id
  · subst hp
    constructor
    · rintro ⟨pi, s, ci, hpi, hs, -⟩
      simp only [↓reduceIte, Option.some.injEq] at hpi
      subst hpi
      simp [NatMap.find?, List.lookup] at hs
    · rintro ⟨pi, s, ci, hpi, -⟩
      rw [hn] at hpi; cases hpi
  · by_cases hc : c = id
    · subst hc
      simp only [hp, ↓reduceIte, Option.some.injEq]
      constructor
      · rintro ⟨pi, s, ci, -, -, -, rfl, hm⟩; simp at hm
      · rintro ⟨pi, s, ci, -, -, -, hci, -⟩; rw [hn] at hci; cases hci
    · simp [hp, hc]

theorem newRef_wf {g g' : Graph} {id : Nat} {m : Bool} (h : g.newRef id m = .ok g') (hw : g.WF) :
    g'.WF := by
  have hf := newRef_find h
  obtain ⟨hn, rfl⟩ := newRef_ok h
  refine ⟨(insert_keys_pairwise _ _ _ hw.sorted).1, fun p pi c s hp hs => ?_, fun p pi hp => ?_⟩
  · rw [hf] at hp
    split at hp
    · simp only [Option.some.injEq] at hp; subst hp; simp [NatMap.find?, List.lookup] at hs
    · exact hw.sets p pi c s hp hs
  · rw [hf] at hp
    split at hp
    · simp only [Option.some.injEq] at hp; subst hp; simp
    · exact hw.inner p pi hp

theorem newRef_covers {g g' : Graph} {id : Nat} {m : Bool} (h : g.newRef id m = .ok g') :
    g.Covers g' :=
  fun _ e _ _ he => ⟨e, (newRef_edge h).2 he, Edge.leq_refl e⟩

/-! ## Adding an edge -/

/-- The edge set from `p` to `c` in `g`, empty when there is none. -/
def edgeSet (g : Graph) (p c : Nat) : EdgeSet :=
  match g.find? p with
  | some pi => (pi.borrowedBy.find? c).getD {}
  | none => {}

theorem addEdge_ok {g g' : Graph} {p c : Nat} {e : BorrowGraph.Edge}
    (h : g.addEdge p e c = .ok g') :
    p ≠ c ∧ ∃ pi ci, g.find? p = some pi ∧ g.find? c = some ci ∧
      g' = (g.insert p { pi with
          borrowedBy := pi.borrowedBy.insert c (((pi.borrowedBy.find? c).getD {}).insert e) }).insert
        c { ci with borrowsFrom := insertNat p ci.borrowsFrom } := by
  unfold addEdge at h
  split at h
  · simp [panicAt, bind, Except.bind] at h
  · next hpc =>
    simp only [bind_ok, Graph.get] at h
    obtain ⟨pi, hpi, h⟩ := h
    split at hpi
    · next pi' hp =>
      simp only [pure, Except.pure, Except.ok.injEq] at hpi
      subst hpi
      obtain ⟨ci, hci, h⟩ := h
      split at hci
      · next ci' hc =>
        simp only [pure, Except.pure, Except.ok.injEq] at hci h
        subst hci
        have hne : p ≠ c := by simpa using hpc
        rw [NatMap.find?_insert] at hc
        simp only [Ne.symm hne, ↓reduceIte] at hc
        exact ⟨hne, pi', ci', hp, hc, h.symm⟩
      · simp [panicAt] at hci
    · simp [panicAt] at hpi

theorem addEdge_find {g g' : Graph} {p c : Nat} {e : BorrowGraph.Edge}
    (h : g.addEdge p e c = .ok g') :
    ∃ pi ci, g.find? p = some pi ∧ g.find? c = some ci ∧ ∀ x,
      g'.find? x = if x = c then some { ci with borrowsFrom := insertNat p ci.borrowsFrom }
        else if x = p then some { pi with
          borrowedBy := pi.borrowedBy.insert c ((g.edgeSet p c).insert e) }
        else g.find? x := by
  obtain ⟨hne, pi, ci, hp, hc, rfl⟩ := addEdge_ok h
  refine ⟨pi, ci, hp, hc, fun x => ?_⟩
  rw [NatMap.find?_insert, NatMap.find?_insert]
  simp [edgeSet, hp]

theorem addEdge_mutability {g g' : Graph} {p c : Nat} {e : BorrowGraph.Edge}
    (h : g.addEdge p e c = .ok g') (x : Nat) : g'.mutability x = g.mutability x := by
  obtain ⟨pi, ci, hp, hc, hf⟩ := addEdge_find h
  simp only [mutability, hf]
  by_cases hxc : x = c
  · subst hxc; simp [hc]
  · by_cases hxp : x = p
    · subst hxp; simp [hxc, hp]
    · simp [hxc, hxp]

theorem addEdge_edge {g g' : Graph} {p c : Nat} {e : BorrowGraph.Edge}
    (h : g.addEdge p e c = .ok g') {x : Nat} {e' : BorrowGraph.Edge} {y : Nat} :
    g'.Edge x e' y ↔ (x = p ∧ y = c ∧ e' ∈ ((g.edgeSet p c).insert e).edges) ∨
      (¬(x = p ∧ y = c) ∧ g.Edge x e' y) := by
  have hne := (addEdge_ok h).1
  obtain ⟨pi, ci, hp, hc, hf⟩ := addEdge_find h
  unfold Edge
  simp only [hf]
  by_cases hxp : x = p
  · subst hxp
    have hxc : x ≠ c := hne
    by_cases hyc : y = c
    · subst hyc
      simp [hxc, mem_insertNat, NatMap.find?_insert]
    · by_cases hyx : y = x
      · subst hyx; simp [hyc, hp, NatMap.find?_insert]
      · simp [hxc, hyc, hyx, hp, NatMap.find?_insert]
  · by_cases hxc : x = c
    · subst hxc
      by_cases hyc : y = x
      · subst hyc; simp [hxp, hc, mem_insertNat]
      · by_cases hyp : y = p
        · subst hyp; simp [hxp, hyc, hc, hp]
        · simp [hxp, hyc, hyp, hc]
    · by_cases hyc : y = c
      · subst hyc
        simp [hxp, hxc, mem_insertNat, hc]
      · by_cases hyp : y = p
        · subst hyp; simp [hxp, hxc, hyc, hp]
        · simp [hxp, hxc, hyc, hyp]

theorem edgeSet_wf {g : Graph} (hw : g.SetsWF) (p c : Nat) : (g.edgeSet p c).WF := by
  unfold edgeSet
  split
  · next pi hp =>
    cases hs : pi.borrowedBy.find? c with
    | some s => exact hw p pi c s hp hs
    | none => exact EdgeSet.wf_empty
  · exact EdgeSet.wf_empty

theorem edge_edgeSet {g : Graph} {p c : Nat} {e : BorrowGraph.Edge} (h : g.Edge p e c) :
    e ∈ (g.edgeSet p c).edges := by
  obtain ⟨pi, s, -, hp, hs, he, -⟩ := h
  simp [edgeSet, hp, hs, he]

/-- Adding an edge keeps a covering edge for every edge, and adds one
covering the new edge. -/
theorem addEdge_covers {g g' : Graph} {p c : Nat} {e : BorrowGraph.Edge}
    (h : g.addEdge p e c = .ok g') (hw : g.SetsWF) :
    g.Covers g' ∧ ∃ e', g'.Edge p e' c ∧ e'.leq e = true := by
  have hins := EdgeSet.insert_covers (edgeSet_wf hw p c) e
  refine ⟨fun x e'' y _ hxy => ?_, ?_⟩
  · by_cases hpc : x = p ∧ y = c
    · obtain ⟨rfl, rfl⟩ := hpc
      obtain ⟨e', he', hle⟩ := hins.2 e'' (edge_edgeSet hxy)
      exact ⟨e', (addEdge_edge h).2 (Or.inl ⟨rfl, rfl, he'⟩), hle⟩
    · exact ⟨e'', (addEdge_edge h).2 (Or.inr ⟨hpc, hxy⟩), Edge.leq_refl _⟩
  · obtain ⟨e', he', hle⟩ := hins.1
    exact ⟨e', (addEdge_edge h).2 (Or.inl ⟨rfl, rfl, he'⟩), hle⟩

theorem addEdge_setsWF {g g' : Graph} {p c : Nat} {e : BorrowGraph.Edge}
    (h : g.addEdge p e c = .ok g') (hw : g.SetsWF) : g'.SetsWF := by
  intro x xi y s hx hs
  obtain ⟨pi, ci, hp, hc, hf⟩ := addEdge_find h
  rw [hf] at hx
  by_cases hxc : x = c
  · subst hxc
    simp only [↓reduceIte, Option.some.injEq] at hx
    subst hx
    exact hw x ci y s hc hs
  · by_cases hxp : x = p
    · subst hxp
      simp only [hxc, ↓reduceIte, Option.some.injEq] at hx
      subst hx
      simp only [NatMap.find?_insert] at hs
      split at hs
      · simp only [Option.some.injEq] at hs
        subst hs
        exact EdgeSet.insert_wf (edgeSet_wf hw _ _) e
      · exact hw x pi y s hp hs
    · simp only [hxc, hxp, ↓reduceIte] at hx
      exact hw x xi y s hx hs

theorem addEdge_wf {g g' : Graph} {p c : Nat} {e : BorrowGraph.Edge}
    (h : g.addEdge p e c = .ok g') (hw : g.WF) : g'.WF := by
  refine ⟨?_, addEdge_setsWF h hw.sets, fun x xi hx => ?_⟩
  · obtain ⟨-, pi, ci, -, -, rfl⟩ := addEdge_ok h
    exact (insert_keys_pairwise _ _ _ (insert_keys_pairwise _ _ _ hw.sorted).1).1
  · obtain ⟨pi, ci, hp, hc, hf⟩ := addEdge_find h
    rw [hf] at hx
    split at hx
    · simp only [Option.some.injEq] at hx; subst hx; exact hw.inner _ ci hc
    · split at hx
      · next hxp =>
        simp only [Option.some.injEq] at hx; subst hx hxp
        exact (insert_keys_pairwise _ _ _ (hw.inner _ pi hp)).1
      · exact hw.inner x xi hx

/-! ## Folds of edge additions -/

/-- A fold of steps that each keep the graph well formed, cover its edges,
and keep its references' mutability does too. -/
theorem fold_covers {α : Type} {step : Graph → α → VResult Graph}
    (hstep : ∀ g a g', g.WF → step g a = .ok g' →
      g'.WF ∧ g.Covers g' ∧ ∀ x, g'.mutability x = g.mutability x) :
    ∀ (l : List α) (g g' : Graph), g.WF → l.foldlM step g = .ok g' →
      g'.WF ∧ g.Covers g' ∧ ∀ x, g'.mutability x = g.mutability x
  | [], g, g', hw, h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact ⟨hw, Covers.refl g, fun _ => rfl⟩
  | a :: l, g, g', hw, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨g1, h1, h⟩ := h
    obtain ⟨hw1, hc1, hm1⟩ := hstep g a g1 hw h1
    obtain ⟨hw2, hc2, hm2⟩ := fold_covers hstep l g1 g' hw1 h
    exact ⟨hw2, hc1.trans hc2, fun x => (hm2 x).trans (hm1 x)⟩

/-- In such a fold, a fact each step establishes for its element, and that
covering preserves, holds at the end for every element. -/
theorem fold_adds {α : Type} {step : Graph → α → VResult Graph}
    (hstep : ∀ g a g', g.WF → step g a = .ok g' →
      g'.WF ∧ g.Covers g' ∧ ∀ x, g'.mutability x = g.mutability x)
    (Q : Graph → α → Prop) (hnew : ∀ g a g', g.WF → step g a = .ok g' → Q g' a)
    (hmono : ∀ g g' a, g.Covers g' → Q g a → Q g' a) :
    ∀ (l : List α) (g g' : Graph), g.WF → l.foldlM step g = .ok g' → ∀ a ∈ l, Q g' a
  | [], _, _, _, _ => by simp
  | a :: l, g, g', hw, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨g1, h1, h⟩ := h
    obtain ⟨hw1, -, -⟩ := hstep g a g1 hw h1
    intro b hb
    rcases List.mem_cons.1 hb with rfl | hb
    · exact hmono _ _ _ (fold_covers hstep l g1 g' hw1 h).2.1 (hnew g b g1 hw h1)
    · exact fold_adds hstep Q hnew hmono l g1 g' hw1 h b hb

/-- The edges `addEdge` adds: up to covering. -/
def HasEdge (g : Graph) (p : Nat) (e : BorrowGraph.Edge) (c : Nat) : Prop :=
  ∃ e', g.Edge p e' c ∧ e'.leq e = true

theorem HasEdge.mono {g g' : Graph} {p c : Nat} {e : BorrowGraph.Edge} (hc : g.Covers g')
    (h : g.HasEdge p e c) : g'.HasEdge p e c := by
  obtain ⟨e', he', hle⟩ := h
  obtain ⟨e'', he'', hle'⟩ := hc p e' c trivial he'
  exact ⟨e'', he'', Edge.leq_trans hle' hle⟩

theorem addEdge_step {g g' : Graph} {p c : Nat} {e : BorrowGraph.Edge}
    (h : g.addEdge p e c = .ok g') (hw : g.WF) :
    g'.WF ∧ g.Covers g' ∧ ∀ x, g'.mutability x = g.mutability x :=
  ⟨addEdge_wf h hw, (addEdge_covers h hw.sets).1, addEdge_mutability h⟩

end Graph

/-! ## Factoring -/

namespace Graph

/-- The edges `factor` moves from `parent` to the intermediate reference:
those whose path starts with `path`, with each child. -/
def factorNeeds (pinfo : RefInfo) (path : List Label) : List (Nat × BorrowGraph.Edge) :=
  pinfo.borrowedBy.flatMap fun (child, set) =>
    (set.edges.filter fun e => isPrefix path e.path).map fun e => (child, e)

theorem lookup_mem {α : Type} : ∀ {m : NatMap α} {k : Nat} {v : α}, m.find? k = some v → (k, v) ∈ m
  | [], _, _, h => by simp [NatMap.find?, List.lookup] at h
  | (k', v') :: rest, k, v, h => by
    simp only [NatMap.find?, List.lookup] at h
    split at h
    · next heq =>
      simp only [Option.some.injEq] at h
      subst h
      have : k = k' := by simpa using heq
      subst this
      simp
    · exact List.mem_cons_of_mem _ (lookup_mem h)

theorem mem_factorNeeds {pinfo : RefInfo} {path : List Label} {y : Nat} {s : EdgeSet}
    {e : BorrowGraph.Edge} (hs : pinfo.borrowedBy.find? y = some s) (he : e ∈ s.edges)
    (hp : isPrefix path e.path = true) : (y, e) ∈ factorNeeds pinfo path := by
  simp only [factorNeeds, List.mem_flatMap, List.mem_map, List.mem_filter, Prod.mk.injEq]
  exact ⟨(y, s), lookup_mem hs, e, ⟨he, hp⟩, rfl, rfl⟩

/-- The removal stage of `factor` keeps the borrowers sorted. -/
theorem factor_remove_sorted :
    ∀ (needs : List (Nat × BorrowGraph.Edge)) (B B' : NatMap EdgeSet) (C C' : List Nat),
      (B.map (·.1)).Pairwise (· < ·) →
      needs.foldlM (fun (acc : NatMap EdgeSet × List Nat) (n : Nat × BorrowGraph.Edge) =>
        match acc.1.find? n.1 with
        | none => panicAt "factor: missing edge set"
        | some set =>
          if !set.edges.contains n.2 then panicAt "factor: missing edge"
          else
            let set := set.remove n.2
            if set.edges.isEmpty then pure (acc.1.erase n.1, insertNat n.1 acc.2)
            else pure (acc.1.insert n.1 set, acc.2)) (B, C) = .ok (B', C') →
      (B'.map (·.1)).Pairwise (· < ·)
  | [], B, B', C, C', hs, h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    exact hs
  | n :: needs, B, B', C, C', hs, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨⟨B1, C1⟩, h1, h⟩ := h
    refine factor_remove_sorted needs B1 B' C1 C' ?_ h
    split at h1
    · simp [panicAt] at h1
    · split at h1
      · simp [panicAt] at h1
      · split at h1
        · simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h1
          obtain ⟨rfl, rfl⟩ := h1
          exact erase_keys_pairwise _ _ hs
        · simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h1
          obtain ⟨rfl, rfl⟩ := h1
          exact (insert_keys_pairwise _ _ _ hs).1

/-- The removal stage of `factor`: removing edges with a prefix keeps the
others, keeps edge sets well formed, and erases exactly the children whose
set it empties, which it lists. -/
theorem factor_remove {path : List Label} :
    ∀ (needs : List (Nat × BorrowGraph.Edge)) (B B' : NatMap EdgeSet) (C C' : List Nat),
      (∀ n ∈ needs, isPrefix path n.2.path = true) →
      (∀ y s, B.find? y = some s → s.WF) →
      needs.foldlM (fun (acc : NatMap EdgeSet × List Nat) (n : Nat × BorrowGraph.Edge) =>
        match acc.1.find? n.1 with
        | none => panicAt "factor: missing edge set"
        | some set =>
          if !set.edges.contains n.2 then panicAt "factor: missing edge"
          else
            let set := set.remove n.2
            if set.edges.isEmpty then pure (acc.1.erase n.1, insertNat n.1 acc.2)
            else pure (acc.1.insert n.1 set, acc.2)) (B, C) = .ok (B', C') →
      (∀ y e s, B.find? y = some s → e ∈ s.edges → isPrefix path e.path = false →
        ∃ s', B'.find? y = some s' ∧ e ∈ s'.edges) ∧
      (∀ y s, B'.find? y = some s → s.WF) ∧
      (∀ y s', B'.find? y = some s' → ∃ s, B.find? y = some s) ∧
      (∀ y ∈ C', y ∈ C ∨ B'.find? y = none) ∧
      (∀ y ∈ C, y ∈ C')
  | [], B, B', C, C', _, hwf, h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    exact ⟨fun y e s hs he _ => ⟨s, hs, he⟩, hwf, fun y s hs => ⟨s, hs⟩,
      fun y hy => Or.inl hy, fun y hy => hy⟩
  | (y0, e0) :: needs, B, B', C, C', hpre, hwf, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨⟨B1, C1⟩, h1, h⟩ := h
    have hp0 : isPrefix path e0.path = true := hpre (y0, e0) (by simp)
    split at h1
    · simp [panicAt] at h1
    · next s0 hs0 =>
      split at h1
      · simp [panicAt] at h1
      · next hcont =>
        have hrm : ∀ e ∈ s0.edges, e ≠ e0 → e ∈ (s0.remove e0).edges := fun e he hne =>
          (List.mem_erase_of_ne hne).2 he
        -- One removal step.
        have step : (∀ y e s, B.find? y = some s → e ∈ s.edges → isPrefix path e.path = false →
              ∃ s', B1.find? y = some s' ∧ e ∈ s'.edges) ∧
            (∀ y s, B1.find? y = some s → s.WF) ∧
            (∀ y s', B1.find? y = some s' → ∃ s, B.find? y = some s) ∧
            (∀ y ∈ C1, y ∈ C ∨ B1.find? y = none) ∧ (∀ y ∈ C, y ∈ C1) := by
          split at h1
          · next hemp =>
            simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h1
            obtain ⟨rfl, rfl⟩ := h1
            refine ⟨fun y e s hs he hnp => ?_, fun y s hs => ?_, fun y s hs => ?_, fun y hy => ?_,
              fun y hy => (mem_insertNat _ _ _).2 (Or.inr hy)⟩
            · rw [NatMap.find?_erase]
              by_cases hy : y = y0
              · subst hy
                rw [hs0, Option.some.injEq] at hs
                subst hs
                have hne : e ≠ e0 := fun heq => by rw [heq, hp0] at hnp; cases hnp
                have := hrm e he hne
                simp only [List.isEmpty_iff] at hemp
                rw [hemp] at this; cases this
              · simp only [hy, ↓reduceIte]; exact ⟨s, hs, he⟩
            · rw [NatMap.find?_erase] at hs
              split at hs
              · cases hs
              · exact hwf y s hs
            · rw [NatMap.find?_erase] at hs
              split at hs
              · cases hs
              · exact ⟨s, hs⟩
            · rcases (mem_insertNat _ _ _).1 hy with rfl | hy
              · right; simp [NatMap.find?_erase]
              · exact Or.inl hy
          · next hemp =>
            simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h1
            obtain ⟨rfl, rfl⟩ := h1
            refine ⟨fun y e s hs he hnp => ?_, fun y s hs => ?_, fun y s hs => ?_,
              fun y hy => Or.inl hy, fun y hy => hy⟩
            · rw [NatMap.find?_insert]
              by_cases hy : y = y0
              · subst hy
                rw [hs0, Option.some.injEq] at hs
                subst hs
                have hne : e ≠ e0 := fun heq => by rw [heq, hp0] at hnp; cases hnp
                exact ⟨_, by simp, hrm e he hne⟩
              · simp only [hy, ↓reduceIte]; exact ⟨s, hs, he⟩
            · rw [NatMap.find?_insert] at hs
              split at hs
              · simp only [Option.some.injEq] at hs
                subst hs
                exact EdgeSet.remove_wf (hwf _ _ hs0) e0 (by simpa [List.isEmpty_iff] using hemp)
              · exact hwf y s hs
            · rw [NatMap.find?_insert] at hs
              split at hs
              · next hy => subst hy; exact ⟨s0, hs0⟩
              · exact ⟨s, hs⟩
        obtain ⟨k1, w1, d1, c1, m1⟩ := step
        obtain ⟨k2, w2, d2, c2, m2⟩ :=
          factor_remove needs B1 B' C1 C' (fun n hn => hpre n (by simp [hn])) w1 h
        refine ⟨fun y e s hs he hnp => ?_, w2, fun y s' hs' => ?_, fun y hy => ?_, fun y hy => m2 y (m1 y hy)⟩
        · obtain ⟨s1, hs1, he1⟩ := k1 y e s hs he hnp
          exact k2 y e s1 hs1 he1 hnp
        · obtain ⟨s1, hs1⟩ := d2 y s' hs'
          exact d1 y s1 hs1
        · rcases c2 y hy with hy | hy
          · rcases c1 y hy with hy | hy
            · exact Or.inl hy
            · -- Erased now; nothing re-inserts it, since a missing set panics.
              right
              cases hB : B'.find? y with
              | none => rfl
              | some s' =>
                obtain ⟨s1, hs1⟩ := d2 y s' hB
                rw [hy] at hs1; cases hs1
          · exact Or.inr hy

/-- `g'` is `g` with `parent` dropped from the back edges of the children
`C`: same references, forward edges and mutability; every other back edge
kept. -/
def BackDrop (parent : Nat) (C : List Nat) (g g' : Graph) : Prop :=
  ∀ x, (g'.find? x = none ↔ g.find? x = none) ∧ ∀ xi, g.find? x = some xi →
    ∃ xi', g'.find? x = some xi' ∧ xi'.borrowedBy = xi.borrowedBy ∧ xi'.mutable = xi.mutable ∧
      (∀ z ∈ xi.borrowsFrom, z ≠ parent → z ∈ xi'.borrowsFrom) ∧ (x ∉ C → xi' = xi)

/-- The cleanup stage of `factor`. -/
theorem factor_cleanup (parent : Nat) :
    ∀ (C : List Nat) (g g' : Graph),
      C.foldlM (fun (g : Graph) child => do
        let c ← g.get child
        if !c.borrowsFrom.contains parent then panicAt "factor: missing back edge"
        pure (g.insert child { c with borrowsFrom := c.borrowsFrom.erase parent })) g = .ok g' →
      BackDrop parent C g g' ∧ ((g.map (·.1)).Pairwise (· < ·) → (g'.map (·.1)).Pairwise (· < ·))
  | [], g, g', h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact ⟨fun x => ⟨Iff.rfl, fun xi hx => ⟨xi, hx, rfl, rfl, fun z hz _ => hz, fun _ => rfl⟩⟩,
      id⟩
  | a :: C, g, g', h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨g1, h1, h⟩ := h
    simp only [Graph.get] at h1
    obtain ⟨c, hc, h1⟩ := h1
    split at hc
    · next c' hca =>
      simp only [pure, Except.pure, Except.ok.injEq] at hc
      subst hc
      split at h1
      · simp [panicAt, bind, Except.bind] at h1
      · simp only [pure, Except.pure, Except.ok.injEq] at h1
        subst h1
        obtain ⟨hd, hs⟩ := factor_cleanup parent C _ g' h
        refine ⟨fun x => ?_, fun hg => hs (insert_keys_pairwise _ _ _ hg).1⟩
        obtain ⟨hn, hsm⟩ := hd x
        rw [NatMap.find?_insert] at hn hsm
        refine ⟨?_, fun xi hx => ?_⟩
        · rw [hn]
          by_cases hxa : x = a
          · subst hxa; simp [hca]
          · simp [hxa]
        · by_cases hxa : x = a
          · subst hxa
            rw [hca, Option.some.injEq] at hx
            subst hx
            obtain ⟨xi', h1, h2, h3, h4, -⟩ :=
              hsm { c' with borrowsFrom := c'.borrowsFrom.erase parent } (by simp)
            exact ⟨xi', h1, h2, h3, fun z hz hzp => h4 z ((List.mem_erase_of_ne hzp).2 hz) hzp,
              fun hn => absurd (List.mem_cons_self) hn⟩
          · obtain ⟨xi', h1, h2, h3, h4, h5⟩ := hsm xi (by simp [hxa, hx])
            exact ⟨xi', h1, h2, h3, h4, fun hn => h5 (fun hm => hn (List.mem_cons_of_mem _ hm))⟩
    · simp [panicAt] at hc

end Graph

namespace Graph

/-- **Factoring**: `factor parent path inter` keeps (up to covering) every
edge except those of `parent` whose path starts with `path`; it moves those
to `inter`, without the prefix; and it makes `inter` borrow `path` from
`parent` strongly. -/
theorem factor_ok {g g' : Graph} {parent : Nat} {path : List Label} {inter : Nat}
    (h : g.factor parent path inter = .ok g') (hw : g.WF) :
    g'.WF ∧ (∀ x, g'.mutability x = g.mutability x) ∧
      (∀ x e y, g.Edge x e y → (x ≠ parent ∨ isPrefix path e.path = false) →
        g'.HasEdge x e y) ∧
      (∀ e y, g.Edge parent e y → isPrefix path e.path = true →
        g'.HasEdge inter ⟨e.strong, e.path.drop path.length⟩ y) ∧
      g'.HasEdge parent ⟨true, path⟩ inter := by
  unfold factor at h
  simp only [bind_ok, Graph.get] at h
  obtain ⟨pinfo, hp, h⟩ := h
  split at hp
  · next pinfo' hpar =>
    simp only [pure, Except.pure, Except.ok.injEq] at hp
    subst hp
    obtain ⟨⟨B, C⟩, hrem, h⟩ := h
    obtain ⟨g2, hclean, h⟩ := h
    obtain ⟨g3, hadd, hlast⟩ := h
    -- Removal.
    obtain ⟨rkeep, rwf, rdom, rclean, -⟩ :=
      factor_remove (path := path) (factorNeeds pinfo' path) pinfo'.borrowedBy B [] C
        (fun n hn => by
          simp only [factorNeeds, List.mem_flatMap, List.mem_map, List.mem_filter] at hn
          obtain ⟨_, _, e, ⟨_, hpre⟩, rfl⟩ := hn
          exact hpre)
        (fun y s hs => hw.sets parent pinfo' y s hpar hs) hrem
    -- The parent's new edge sets.
    have hf1 : ∀ x, NatMap.find? (g.insert parent { pinfo' with borrowedBy := B }) x =
        if x = parent then some { pinfo' with borrowedBy := B } else g.find? x :=
      fun x => NatMap.find?_insert _ _ _ _
    -- Cleanup.
    obtain ⟨hdrop, hsort2⟩ := factor_cleanup parent C _ g2 hclean
    have hBs : (B.map (·.1)).Pairwise (· < ·) :=
      factor_remove_sorted _ _ _ _ _ (hw.inner parent pinfo' hpar) hrem
    have hw2 : g2.WF := by
      refine ⟨hsort2 (insert_keys_pairwise _ _ _ hw.sorted).1, fun x xi y s hx hs => ?_,
        fun x xi hx => ?_⟩
      · obtain ⟨hn, hsm⟩ := hdrop x
        cases hx1 : NatMap.find? (g.insert parent { pinfo' with borrowedBy := B }) x with
        | none => rw [hn.2 hx1] at hx; cases hx
        | some xi1 =>
          obtain ⟨xi', hx', hbb, -, -, -⟩ := hsm xi1 hx1
          rw [hx, Option.some.injEq] at hx'
          subst hx'
          rw [hbb] at hs
          rw [hf1] at hx1
          split at hx1
          · simp only [Option.some.injEq] at hx1; subst hx1; exact rwf y s hs
          · exact hw.sets x xi1 y s hx1 hs
      · obtain ⟨hn, hsm⟩ := hdrop x
        cases hx1 : NatMap.find? (g.insert parent { pinfo' with borrowedBy := B }) x with
        | none => rw [hn.2 hx1] at hx; cases hx
        | some xi1 =>
          obtain ⟨xi', hx', hbb, -, -, -⟩ := hsm xi1 hx1
          rw [hx, Option.some.injEq] at hx'
          subst hx'
          rw [hbb]
          rw [hf1] at hx1
          split at hx1
          · simp only [Option.some.injEq] at hx1; subst hx1; exact hBs
          · exact hw.inner x xi1 hx1
    have hmut2 : ∀ x, g2.mutability x = g.mutability x := by
      intro x
      obtain ⟨hn, hsm⟩ := hdrop x
      simp only [mutability]
      cases hx1 : NatMap.find? (g.insert parent { pinfo' with borrowedBy := B }) x with
      | none =>
        rw [hn.2 hx1]
        rw [hf1] at hx1
        split at hx1
        · cases hx1
        · rw [hx1]
      | some xi1 =>
        obtain ⟨xi', hx', -, hm, -, -⟩ := hsm xi1 hx1
        rw [hx', Option.map_some, hm]
        rw [hf1] at hx1
        split at hx1
        · next hxp => simp only [Option.some.injEq] at hx1; subst hx1 hxp; simp [hpar]
        · rw [hx1]; rfl
    -- Additions.
    have hstep : ∀ (g : Graph) (n : Nat × BorrowGraph.Edge) (g' : Graph), g.WF →
        g.addEdge inter ⟨n.2.strong, n.2.path.drop path.length⟩ n.1 = .ok g' →
        g'.WF ∧ g.Covers g' ∧ ∀ x, g'.mutability x = g.mutability x :=
      fun _ _ _ hw h => addEdge_step h hw
    obtain ⟨hw3, hcov3, hmut3⟩ := fold_covers hstep _ g2 g3 hw2 hadd
    have hadds := fold_adds hstep (fun g n => g.HasEdge inter ⟨n.2.strong, n.2.path.drop path.length⟩ n.1)
      (fun _ _ _ hw h => (addEdge_covers h hw.sets).2)
      (fun _ _ _ hc hq => hq.mono hc) _ g2 g3 hw2 hadd
    obtain ⟨hw4, hcov4, hmut4⟩ := addEdge_step hlast hw3
    refine ⟨hw4, fun x => (hmut4 x).trans ((hmut3 x).trans (hmut2 x)),
      fun x e y hxy hcase => ?_, fun e y hxy hpre => ?_,
      (addEdge_covers hlast hw3.sets).2⟩
    · -- Kept edges reach `g2`, then are covered.
      suffices g2.Edge x e y from HasEdge.mono hcov4 (HasEdge.mono hcov3 ⟨e, this, Edge.leq_refl e⟩)
      obtain ⟨xi, s, yi, hx, hs, he, hy, hb⟩ := hxy
      -- `g1`
      have hx1 : NatMap.find? (g.insert parent { pinfo' with borrowedBy := B }) x =
          some (if x = parent then { pinfo' with borrowedBy := B } else xi) := by
        rw [hf1]; split <;> simp_all
      have hy1 : NatMap.find? (g.insert parent { pinfo' with borrowedBy := B }) y =
          some (if y = parent then { pinfo' with borrowedBy := B } else yi) := by
        rw [hf1]; split <;> simp_all
      obtain ⟨xi', hx2, hbbx, -, -, -⟩ := (hdrop x).2 _ hx1
      obtain ⟨yi', hy2, -, -, hback, hkeep⟩ := (hdrop y).2 _ hy1
      have hbyi : (if y = parent then ({ pinfo' with borrowedBy := B } : RefInfo) else yi).borrowsFrom =
          yi.borrowsFrom := by
        split
        · next hyp => subst hyp; rw [hpar, Option.some.injEq] at hy; subst hy; rfl
        · rfl
      rcases hcase with hxp | hnp
      · refine ⟨xi', s, yi', hx2, ?_, he, hy2, hback x (by rw [hbyi]; exact hb) hxp⟩
        rw [hbbx]; simp [hxp, hs]
      · by_cases hxp : x = parent
        · subst hxp
          rw [hpar, Option.some.injEq] at hx
          subst hx
          obtain ⟨s', hs', he'⟩ := rkeep y e s hs he hnp
          have hyC : y ∉ C := fun hyC => by
            rcases rclean y hyC with hyC | hyC
            · simp at hyC
            · rw [hyC] at hs'; cases hs'
          refine ⟨xi', s', yi', hx2, ?_, he', hy2, ?_⟩
          · rw [hbbx]; simp [hs']
          · rw [hkeep hyC, hbyi]; exact hb
        · refine ⟨xi', s, yi', hx2, ?_, he, hy2, hback x (by rw [hbyi]; exact hb) hxp⟩
          rw [hbbx]; simp [hxp, hs]
    · obtain ⟨pi, s, -, hx, hs, he, -⟩ := hxy
      rw [hpar, Option.some.injEq] at hx
      subst hx
      exact HasEdge.mono hcov4 (hadds (y, e) (mem_factorNeeds hs he hpre))
  · simp [panicAt] at hp

end Graph

/-! ## Releasing a reference -/

namespace Graph

/-- The edge `splice` adds from a parent of a released reference to one of
its children. -/
def spliceEdge (pe ce : BorrowGraph.Edge) : BorrowGraph.Edge :=
  ⟨pe.strong && ce.strong, if pe.strong then pe.path ++ ce.path else pe.path⟩

theorem splice_step {g g' : Graph} {p c : Nat} {pe ce : BorrowGraph.Edge}
    (h : g.splice p pe c ce = .ok g') (hw : g.WF) :
    g'.WF ∧ g.Covers g' ∧ ∀ x, g'.mutability x = g.mutability x := by
  unfold splice at h
  split at h
  · simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact ⟨hw, Covers.refl _, fun _ => rfl⟩
  · exact addEdge_step h hw

theorem splice_adds {g g' : Graph} {p c : Nat} {pe ce : BorrowGraph.Edge}
    (h : g.splice p pe c ce = .ok g') (hw : g.WF) (hne : p ≠ c) :
    g'.HasEdge p (spliceEdge pe ce) c := by
  unfold splice at h
  split at h
  · next heq => exact absurd (by simpa using heq) hne
  · exact (addEdge_covers h hw.sets).2

/-- What changes the references' forward edges, other than at `q`. -/
def SameOut (q : Nat) (g g' : Graph) : Prop :=
  ∀ p, p ≠ q → (g'.find? p).map (·.borrowedBy) = (g.find? p).map (·.borrowedBy)

theorem SameOut.trans {q : Nat} {g₁ g₂ g₃ : Graph} (h1 : SameOut q g₁ g₂) (h2 : SameOut q g₂ g₃) :
    SameOut q g₁ g₃ := fun p hp => (h2 p hp).trans (h1 p hp)

theorem addEdge_sameOut {g g' : Graph} {p c : Nat} {e : BorrowGraph.Edge}
    (h : g.addEdge p e c = .ok g') : SameOut p g g' := by
  intro x hx
  obtain ⟨pi, ci, hp, hc, hf⟩ := addEdge_find h
  rw [hf]
  by_cases hxc : x = c
  · subst hxc; simp [hc]
  · simp [hxc, hx]

theorem splice_sameOut {g g' : Graph} {p c : Nat} {pe ce : BorrowGraph.Edge}
    (h : g.splice p pe c ce = .ok g') : SameOut p g g' := by
  unfold splice at h
  split at h
  · simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact fun _ _ => rfl
  · exact addEdge_sameOut h

theorem fold_sameOut {α : Type} {q : Nat} {step : Graph → α → VResult Graph}
    (hstep : ∀ g a g', step g a = .ok g' → SameOut q g g') :
    ∀ (l : List α) (g g' : Graph), l.foldlM step g = .ok g' → SameOut q g g'
  | [], g, g', h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h; exact fun _ _ => rfl
  | a :: l, g, g', h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨g1, h1, h⟩ := h
    exact (hstep g a g1 h1).trans (fold_sameOut hstep l g1 g' h)

/-- All the splices of one parent edge `pe` of the released reference, to the
children `kids` of the released reference. -/
theorem splices_ok {g g' : Graph} {q : Nat} {pe : BorrowGraph.Edge}
    {kids : NatMap EdgeSet}
    (h : kids.foldlM (fun (g : Graph) (child, childEdges) =>
      childEdges.edges.foldlM (fun (g : Graph) ce => g.splice q pe child ce) g) g = .ok g')
    (hw : g.WF) :
    (g'.WF ∧ g.Covers g' ∧ ∀ x, g'.mutability x = g.mutability x) ∧ SameOut q g g' ∧
      ∀ c cs, (c, cs) ∈ kids → ∀ ce ∈ cs.edges, q ≠ c → g'.HasEdge q (spliceEdge pe ce) c := by
  have hinner : ∀ (c : Nat) (cs : EdgeSet) (g g' : Graph), g.WF →
      cs.edges.foldlM (fun (g : Graph) ce => g.splice q pe c ce) g = .ok g' →
      (g'.WF ∧ g.Covers g' ∧ ∀ x, g'.mutability x = g.mutability x) ∧ SameOut q g g' ∧
        ∀ ce ∈ cs.edges, q ≠ c → g'.HasEdge q (spliceEdge pe ce) c := by
    intro c cs g g' hw h
    refine ⟨fold_covers (fun _ _ _ hw h => splice_step h hw) _ _ _ hw h,
      fold_sameOut (fun _ _ _ h => splice_sameOut h) _ _ _ h, fun ce hce hne => ?_⟩
    exact fold_adds (fun _ _ _ hw h => splice_step h hw)
      (fun g ce => g.HasEdge q (spliceEdge pe ce) c)
      (fun _ _ _ hw h => splice_adds h hw hne) (fun _ _ _ hc hq => hq.mono hc) _ _ _ hw h ce hce
  refine ⟨fold_covers (fun _ a _ hw h => (hinner a.1 a.2 _ _ hw h).1) _ _ _ hw h,
    fold_sameOut (fun g a g' h => ?_) _ _ _ h, fun c cs hmem ce hce hne => ?_⟩
  · -- `SameOut` does not need well-formedness.
    exact fold_sameOut (fun _ _ _ h => splice_sameOut h) _ _ _ h
  · exact fold_adds (fun _ a _ hw h => (hinner a.1 a.2 _ _ hw h).1)
      (fun g a => ∀ ce ∈ a.2.edges, q ≠ a.1 → g.HasEdge q (spliceEdge pe ce) a.1)
      (fun _ a _ hw h => (hinner a.1 a.2 _ _ hw h).2.2)
      (fun (g g' : Graph) (a : Nat × EdgeSet) (hc : g.Covers g')
        (hq : ∀ ce ∈ a.2.edges, q ≠ a.1 → g.HasEdge q (spliceEdge pe ce) a.1) ce hce hne =>
          HasEdge.mono hc (hq ce hce hne))
      _ _ _ hw h (c, cs) hmem ce hce hne

end Graph

namespace Graph

theorem erase_wf {g : Graph} (hw : g.WF) (x : Nat) : Graph.WF (g.erase x) :=
  ⟨erase_keys_pairwise _ _ hw.sorted, fun p pi c s hp hs => by
    rw [NatMap.find?_erase] at hp
    split at hp
    · cases hp
    · exact hw.sets p pi c s hp hs, fun p pi hp => by
    rw [NatMap.find?_erase] at hp
    split at hp
    · cases hp
    · exact hw.inner p pi hp⟩

theorem insert_borrowedBy_erase_wf {g : Graph} (hw : g.WF) {q id : Nat} {pi : RefInfo}
    (hq : g.find? q = some pi) :
    Graph.WF (g.insert q { pi with borrowedBy := pi.borrowedBy.erase id }) :=
  ⟨(insert_keys_pairwise _ _ _ hw.sorted).1, fun p pi' c s hp hs => by
    rw [NatMap.find?_insert] at hp
    split at hp
    · next hpq =>
      subst hpq
      simp only [Option.some.injEq] at hp
      subst hp
      rw [NatMap.find?_erase] at hs
      split at hs
      · cases hs
      · exact hw.sets _ pi c s hq hs
    · exact hw.sets p pi' c s hp hs, fun p pi' hp => by
    rw [NatMap.find?_insert] at hp
    split at hp
    · next hpq =>
      subst hpq
      simp only [Option.some.injEq] at hp
      subst hp
      exact erase_keys_pairwise _ _ (hw.inner _ pi hq)
    · exact hw.inner p pi' hp⟩

/-- The last stage of `release`: the children drop the released reference
from their back edges. -/
theorem release_kids (id : Nat) :
    ∀ (kids : NatMap EdgeSet) (g g' : Graph),
      kids.foldlM (fun (g : Graph) (child, _) => do
        let c ← g.get child
        pure (g.insert child { c with borrowsFrom := c.borrowsFrom.erase id })) g = .ok g' →
      BackDrop id (kids.map (·.1)) g g' ∧
        ((g.map (·.1)).Pairwise (· < ·) → (g'.map (·.1)).Pairwise (· < ·))
  | [], g, g', h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact ⟨fun x => ⟨Iff.rfl, fun xi hx => ⟨xi, hx, rfl, rfl, fun z hz _ => hz, fun _ => rfl⟩⟩,
      fun h => h⟩
  | (a, s) :: kids, g, g', h => by
    simp only [List.foldlM_cons, bind_ok, Graph.get] at h
    obtain ⟨g1, ⟨c, hc, h1⟩, h⟩ := h
    split at hc
    · next c' hca =>
      simp only [pure, Except.pure, Except.ok.injEq] at hc h1
      subst hc h1
      obtain ⟨hd, hs⟩ := release_kids id kids _ g' h
      refine ⟨fun x => ?_, fun hg => hs (insert_keys_pairwise _ _ _ hg).1⟩
      obtain ⟨hn, hsm⟩ := hd x
      rw [NatMap.find?_insert] at hn hsm
      refine ⟨?_, fun xi hx => ?_⟩
      · rw [hn]
        by_cases hxa : x = a
        · subst hxa; simp [hca]
        · simp [hxa]
      · by_cases hxa : x = a
        · subst hxa
          rw [hca, Option.some.injEq] at hx
          subst hx
          obtain ⟨xi', h1, h2, h3, h4, -⟩ :=
            hsm { c' with borrowsFrom := c'.borrowsFrom.erase id } (by simp)
          exact ⟨xi', h1, h2, h3, fun z hz hzp => h4 z ((List.mem_erase_of_ne hzp).2 hz) hzp,
            fun hn => absurd (by simp) hn⟩
        · obtain ⟨xi', h1, h2, h3, h4, h5⟩ := hsm xi (by simp [hxa, hx])
          exact ⟨xi', h1, h2, h3, h4, fun hn => h5 (fun hm => hn (by simp [hm]))⟩
    · simp [panicAt] at hc

/-- The invariant of `release`'s walk over the parents of the released
reference `id`, the parents `done` processed. -/
structure ReleaseInv (g : Graph) (id : Nat) (done : List Nat) (gc : Graph) : Prop where
  wf : gc.WF
  gone : gc.find? id = none
  mutab : ∀ x, x ≠ id → gc.mutability x = g.mutability x
  keep : ∀ x e y, g.Edge x e y → x ≠ id → y ≠ id → gc.HasEdge x e y
  spliced : ∀ p ∈ done, ∀ pe c ce, g.Edge p pe id → g.Edge id ce c → p ≠ c →
    gc.HasEdge p (spliceEdge pe ce) c
  same : ∀ p, p ∉ done → p ≠ id → (gc.find? p).map (·.borrowedBy) = (g.find? p).map (·.borrowedBy)

theorem release_parents {g : Graph} {id : Nat} {r : RefInfo} (hr : g.find? id = some r) :
    ∀ (L done : List Nat) (gc gc' : Graph), ReleaseInv g id done gc →
      L.foldlM (fun (g : Graph) parent => do
        let p ← g.get parent
        let parentEdges ← match p.borrowedBy.find? id with
          | some s => pure s
          | none => panicAt "release: missing parent edge"
        let g : Graph := g.insert parent { p with borrowedBy := p.borrowedBy.erase id }
        parentEdges.edges.foldlM (fun (g : Graph) pe =>
          r.borrowedBy.foldlM (fun (g : Graph) ((child, childEdges) : Nat × EdgeSet) =>
            childEdges.edges.foldlM (fun (g : Graph) ce => g.splice parent pe child ce) g) g) g)
        gc = .ok gc' →
      ReleaseInv g id (done ++ L) gc'
  | [], done, gc, gc', hi, h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h; simpa using hi
  | q :: L, done, gc, gc', hi, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨g1, h1, h⟩ := h
    suffices ReleaseInv g id (done ++ [q]) g1 by
      simpa using release_parents hr L (done ++ [q]) g1 gc' this h
    simp only [Graph.get] at h1
    obtain ⟨pi, hpi, h1⟩ := h1
    split at hpi
    · next pi' hq =>
      simp only [pure, Except.pure, Except.ok.injEq] at hpi
      subst hpi
      split at h1
      · next s' hs' =>
        simp only [pure, Except.pure, bind, Except.bind] at h1
        have hqid : q ≠ id := fun e => by subst e; rw [hi.gone] at hq; cases hq
        have hw0 := insert_borrowedBy_erase_wf hi.wf (id := id) hq
        -- The splices of every parent edge.
        have hstep : ∀ (g : Graph) (pe : BorrowGraph.Edge) (g' : Graph), g.WF →
            r.borrowedBy.foldlM (fun (g : Graph) ((child, childEdges) : Nat × EdgeSet) =>
              childEdges.edges.foldlM (fun (g : Graph) ce => g.splice q pe child ce) g) g = .ok g' →
            g'.WF ∧ g.Covers g' ∧ ∀ x, g'.mutability x = g.mutability x :=
          fun _ _ _ hw h => (splices_ok h hw).1
        obtain ⟨hw1, hcov1, hmut1⟩ := fold_covers hstep _ _ _ hw0 h1
        have hsame1 := fold_sameOut (fun _ _ _ h => (fold_sameOut (fun _ a _ h =>
          fold_sameOut (fun _ _ _ h => splice_sameOut h) _ _ _ h) _ _ _ h)) _ _ _ h1
        have hadds := fold_adds hstep
          (fun g pe => ∀ c cs, (c, cs) ∈ r.borrowedBy → ∀ ce ∈ cs.edges, q ≠ c →
            g.HasEdge q (spliceEdge pe ce) c)
          (fun _ _ _ hw h => (splices_ok h hw).2.2)
          (fun (g g' : Graph) (pe : BorrowGraph.Edge) (hc : g.Covers g')
            (hq : ∀ c cs, (c, cs) ∈ r.borrowedBy → ∀ ce ∈ cs.edges, q ≠ c →
              g.HasEdge q (spliceEdge pe ce) c) c cs hm ce hce hne =>
            HasEdge.mono hc (hq c cs hm ce hce hne)) _ _ _ hw0 h1
        have hf0 : ∀ x, NatMap.find? (gc.insert q { pi' with borrowedBy := pi'.borrowedBy.erase id }) x =
            if x = q then some { pi' with borrowedBy := pi'.borrowedBy.erase id } else gc.find? x :=
          fun x => NatMap.find?_insert _ _ _ _
        -- Dropping the parent's edges to `id` keeps every other edge.
        have hins : ∀ x e y, gc.Edge x e y →
            Graph.Edge (gc.insert q { pi' with borrowedBy := pi'.borrowedBy.erase id }) x e y := by
          rintro x e y ⟨xi, s, yi, hxi, hs, he, hyi, hb⟩
          have hy : y ≠ id := fun e => by subst e; rw [hi.gone] at hyi; cases hyi
          refine ⟨if x = q then { pi' with borrowedBy := pi'.borrowedBy.erase id } else xi, s,
            if y = q then { pi' with borrowedBy := pi'.borrowedBy.erase id } else yi,
            ?_, ?_, he, ?_, ?_⟩
          · rw [hf0]; split
            · rfl
            · exact hxi
          · split
            · next hxq =>
              subst hxq; rw [hq, Option.some.injEq] at hxi; subst hxi
              simp [NatMap.find?_erase, hy, hs]
            · exact hs
          · rw [hf0]; split
            · rfl
            · exact hyi
          · split
            · next hyq => subst hyq; rw [hq, Option.some.injEq] at hyi; subst hyi; exact hb
            · exact hb
        have hcov : ∀ x e y, gc.HasEdge x e y → g1.HasEdge x e y := by
          rintro x e y ⟨e', he', hle⟩
          exact HasEdge.mono hcov1 ⟨e', hins x e' y he', hle⟩
        refine ⟨hw1, ?_, fun x hx => ?_, fun x e y hxy hx hy => hcov x e y (hi.keep x e y hxy hx hy),
          fun p hp pe c ce hpe hce hne => ?_, fun p hp hpid => ?_⟩
        · -- `id` is still gone: nothing adds it.
          cases h' : g1.find? id with
          | none => rfl
          | some _ =>
            exfalso
            have := hmut1 id
            simp only [mutability, h', hf0, Ne.symm hqid, ↓reduceIte, hi.gone] at this
            cases this
        · rw [hmut1 x, ← hi.mutab x hx]
          simp only [mutability, hf0]
          split
          · next hxq => subst hxq; simp [hq]
          · rfl
        · by_cases hpd : p ∈ done
          · exact hcov _ _ _ (hi.spliced p hpd pe c ce hpe hce hne)
          · have hpq : p = q := by
              rcases List.mem_append.1 hp with hp | hp
              · exact absurd hp hpd
              · simpa using hp
            subst hpq
            -- The parent's edge set to `id` is the original one.
            obtain ⟨pinfo, ps, -, hpg, hps, hpe', -, -⟩ := hpe
            have hsm := hi.same p hpd hqid
            rw [hq, hpg] at hsm
            simp only [Option.map_some, Option.some.injEq] at hsm
            rw [hsm, hps, Option.some.injEq] at hs'
            subst hs'
            obtain ⟨rinfo, cs, -, hrg, hcs, hce', -, -⟩ := hce
            rw [hr, Option.some.injEq] at hrg
            subst hrg
            exact hadds pe hpe' c cs (lookup_mem hcs) ce hce' hne
        · have hpq : p ≠ q := fun e => hp (by simp [e])
          have hpd : p ∉ done := fun e => hp (by simp [e])
          rw [hsame1 p hpq, hf0]
          simp only [hpq, ↓reduceIte]
          exact hi.same p hpd hpid
      · simp [panicAt, bind, Except.bind] at h1
    · simp [panicAt] at hpi

end Graph

namespace Graph

theorem BackDrop.edge {parent : Nat} {C : List Nat} {g g' : Graph} (hd : BackDrop parent C g g')
    {x : Nat} {e : BorrowGraph.Edge} {y : Nat} (h : g.Edge x e y) (hx : x ≠ parent) :
    g'.Edge x e y := by
  obtain ⟨xi, s, yi, hxi, hs, he, hyi, hb⟩ := h
  obtain ⟨xi', hx', hbb, -, -, -⟩ := (hd x).2 xi hxi
  obtain ⟨yi', hy', -, -, hback, -⟩ := (hd y).2 yi hyi
  exact ⟨xi', s, yi', hx', by rw [hbb]; exact hs, he, hy', hback x hb hx⟩

theorem BackDrop.hasEdge {parent : Nat} {C : List Nat} {g g' : Graph}
    (hd : BackDrop parent C g g') {x : Nat} {e : BorrowGraph.Edge} {y : Nat} (h : g.HasEdge x e y)
    (hx : x ≠ parent) : g'.HasEdge x e y := by
  obtain ⟨e', he', hle⟩ := h
  exact ⟨e', hd.edge he' hx, hle⟩

theorem BackDrop.setsWF {parent : Nat} {C : List Nat} {g g' : Graph}
    (hd : BackDrop parent C g g') (hw : g.SetsWF) : g'.SetsWF := by
  intro x xi' y s hx hs
  cases hxi : g.find? x with
  | none => rw [(hd x).1.2 hxi] at hx; cases hx
  | some xi =>
    obtain ⟨xi'', hx'', hbb, -, -, -⟩ := (hd x).2 xi hxi
    rw [hx, Option.some.injEq] at hx''
    subst hx''
    rw [hbb] at hs
    exact hw x xi y s hxi hs

theorem BackDrop.inner {parent : Nat} {C : List Nat} {g g' : Graph}
    (hd : BackDrop parent C g g') (hw : g.InnerSorted) : g'.InnerSorted := by
  intro x xi' hx
  cases hxi : g.find? x with
  | none => rw [(hd x).1.2 hxi] at hx; cases hx
  | some xi =>
    obtain ⟨xi'', hx'', hbb, -, -, -⟩ := (hd x).2 xi hxi
    rw [hx, Option.some.injEq] at hx''
    subst hx''
    rw [hbb]
    exact hw x xi hxi

theorem BackDrop.mutability {parent : Nat} {C : List Nat} {g g' : Graph}
    (hd : BackDrop parent C g g') (x : Nat) : g'.mutability x = g.mutability x := by
  simp only [Graph.mutability]
  cases hxi : g.find? x with
  | none => rw [(hd x).1.2 hxi]
  | some xi =>
    obtain ⟨xi', hx', -, hm, -, -⟩ := (hd x).2 xi hxi
    rw [hx']; simp [hm]

/-- **Releasing** reference `id` removes it, keeps (up to covering) every
edge between other references, and connects each parent of `id` to each of
its children with the spliced edge. -/
theorem release_ok {g g' : Graph} {id : Nat} (h : g.release id = .ok g') (hw : g.WF) :
    g'.WF ∧ g'.find? id = none ∧ (∀ x, x ≠ id → g'.mutability x = g.mutability x) ∧
      (∀ x e y, g.Edge x e y → x ≠ id → y ≠ id → g'.HasEdge x e y) ∧
      (∀ p pe c ce, g.Edge p pe id → g.Edge id ce c → p ≠ c →
        g'.HasEdge p (spliceEdge pe ce) c) := by
  unfold release at h
  simp only [bind_ok, Graph.get] at h
  obtain ⟨r, hr, g2, hpar, hkids⟩ := h
  split at hr
  · next r' hid =>
    simp only [pure, Except.pure, Except.ok.injEq] at hr
    subst hr
    have hi0 : ReleaseInv g id [] (g.erase id) := {
      wf := erase_wf hw id
      gone := by simp [NatMap.find?_erase]
      mutab := fun x hx => by simp [mutability, NatMap.find?_erase, hx]
      keep := fun x e y hxy hx hy => by
        obtain ⟨xi, s, yi, hxi, hs, he, hyi, hb⟩ := hxy
        exact ⟨e, ⟨xi, s, yi, by simp [NatMap.find?_erase, hx, hxi], hs, he,
          by simp [NatMap.find?_erase, hy, hyi], hb⟩, Edge.leq_refl e⟩
      spliced := by simp
      same := fun p _ hp => by simp [NatMap.find?_erase, hp] }
    have hi := release_parents hid r'.borrowsFrom [] _ g2 hi0 hpar
    obtain ⟨hdrop, hsort⟩ := release_kids id r'.borrowedBy g2 g' hkids
    refine ⟨⟨hsort hi.wf.sorted, hdrop.setsWF hi.wf.sets, hdrop.inner hi.wf.inner⟩, (hdrop id).1.2 hi.gone,
      fun x hx => (hdrop.mutability x).trans (hi.mutab x hx),
      fun x e y hxy hx hy => hdrop.hasEdge (hi.keep x e y hxy hx hy) hx,
      fun p pe c ce hpe hce hne => ?_⟩
    have hpr : p ∈ r'.borrowsFrom := by
      obtain ⟨_, _, ci, -, -, -, hci, hb⟩ := hpe
      rw [hid, Option.some.injEq] at hci
      subst hci
      exact hb
    have hspl := hi.spliced p (by simpa using hpr) pe c ce hpe hce hne
    have hpid : p ≠ id := by
      rintro rfl
      obtain ⟨_, ⟨_, _, _, hxi, -⟩, -⟩ := hspl
      rw [hi.gone] at hxi; cases hxi
    exact hdrop.hasEdge hspl hpid
  · simp [panicAt] at hr

end Graph

/-! ## Back edges -/

namespace Graph

/-- Every edge set a reference keeps is recorded by its child: the child is
a reference of the graph that lists the parent among those it borrows from.
`release` finds the parents of a reference this way. -/
def Back (g : Graph) : Prop :=
  ∀ p pi c s, g.find? p = some pi → pi.borrowedBy.find? c = some s →
    ∃ ci, g.find? c = some ci ∧ p ∈ ci.borrowsFrom

theorem Back.edge {g : Graph} (hb : g.Back) {p c : Nat} {pi : RefInfo} {s : EdgeSet}
    {e : BorrowGraph.Edge} (hp : g.find? p = some pi) (hs : pi.borrowedBy.find? c = some s)
    (he : e ∈ s.edges) : g.Edge p e c := by
  obtain ⟨ci, hc, hpc⟩ := hb p pi c s hp hs
  exact ⟨pi, s, ci, hp, hs, he, hc, hpc⟩

theorem newRef_back {g g' : Graph} {id : Nat} {m : Bool} (h : g.newRef id m = .ok g')
    (hb : g.Back) : g'.Back := by
  have hf := newRef_find h
  have hn := (newRef_ok h).1
  intro p pi c s hp hs
  rw [hf] at hp
  split at hp
  · simp only [Option.some.injEq] at hp; subst hp; simp [NatMap.find?, List.lookup] at hs
  · obtain ⟨ci, hc, hpc⟩ := hb p pi c s hp hs
    have hcid : c ≠ id := fun e => by subst e; rw [hn] at hc; cases hc
    exact ⟨ci, by rw [hf]; simp [hcid, hc], hpc⟩

theorem addEdge_back {g g' : Graph} {p c : Nat} {e : BorrowGraph.Edge}
    (h : g.addEdge p e c = .ok g') (hb : g.Back) : g'.Back := by
  have hne := (addEdge_ok h).1
  obtain ⟨pi, ci, hp, hc, hf⟩ := addEdge_find h
  -- The back edges of `y` in `g'` include those in `g`.
  have hy : ∀ y yi x, g.find? y = some yi → x ∈ yi.borrowsFrom →
      ∃ yi', g'.find? y = some yi' ∧ x ∈ yi'.borrowsFrom := by
    intro y yi x hyi hx
    rw [hf]
    by_cases hyc : y = c
    · rw [hyc, hc, Option.some.injEq] at hyi; subst hyi
      exact ⟨{ ci with borrowsFrom := insertNat p ci.borrowsFrom }, by simp [hyc],
        (mem_insertNat _ _ _).2 (Or.inr hx)⟩
    · by_cases hyp : y = p
      · rw [hyp, hp, Option.some.injEq] at hyi; subst hyi
        exact ⟨{ pi with borrowedBy := pi.borrowedBy.insert c ((g.edgeSet p c).insert e) },
          by simp [hyp, hne], hx⟩
      · exact ⟨yi, by simp [hyc, hyp, hyi], hx⟩
  intro x xi y s hx hs
  rw [hf] at hx
  by_cases hxc : x = c
  · subst hxc
    simp only [↓reduceIte, Option.some.injEq] at hx
    subst hx
    obtain ⟨yi, hyi, hxy⟩ := hb x ci y s hc hs
    exact hy y yi x hyi hxy
  · by_cases hxp : x = p
    · subst hxp
      simp only [hxc, ↓reduceIte, Option.some.injEq] at hx
      subst hx
      simp only [NatMap.find?_insert] at hs
      split at hs
      · next hyc =>
        subst hyc
        refine ⟨{ ci with borrowsFrom := insertNat x ci.borrowsFrom }, ?_,
          (mem_insertNat _ _ _).2 (Or.inl rfl)⟩
        rw [hf]; simp
      · obtain ⟨yi, hyi, hxy⟩ := hb x pi y s hp hs
        exact hy y yi x hyi hxy
    · simp only [hxc, hxp, ↓reduceIte] at hx
      obtain ⟨yi, hyi, hxy⟩ := hb x xi y s hx hs
      exact hy y yi x hyi hxy

theorem fold_back {α : Type} {step : Graph → α → VResult Graph}
    (hstep : ∀ g a g', g.Back → step g a = .ok g' → g'.Back) :
    ∀ (l : List α) (g g' : Graph), g.Back → l.foldlM step g = .ok g' → g'.Back
  | [], g, g', hb, h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h; exact hb
  | a :: l, g, g', hb, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨g1, h1, h⟩ := h
    exact fold_back hstep l g1 g' (hstep g a g1 hb h1) h

theorem BackDrop.back {parent : Nat} {C : List Nat} {g g' : Graph}
    (hd : BackDrop parent C g g') (hb : g.Back)
    (hC : ∀ y ∈ C, ∀ pi, g.find? parent = some pi → pi.borrowedBy.find? y = none) : g'.Back := by
  intro x xi' y s hx hs
  cases hxi : g.find? x with
  | none => rw [(hd x).1.2 hxi] at hx; cases hx
  | some xi =>
    obtain ⟨xi'', hx'', hbb, -, -, -⟩ := (hd x).2 xi hxi
    rw [hx, Option.some.injEq] at hx''
    subst hx''
    rw [hbb] at hs
    obtain ⟨yi, hyi, hxy⟩ := hb x xi y s hxi hs
    obtain ⟨yi', hy', -, -, hback, hkeep⟩ := (hd y).2 yi hyi
    by_cases hxp : x = parent
    · subst hxp
      have hyC : y ∉ C := fun hyC => by rw [hC y hyC xi hxi] at hs; cases hs
      exact ⟨yi', hy', by rw [hkeep hyC]; exact hxy⟩
    · exact ⟨yi', hy', hback x hxy hxp⟩

theorem factor_back {g g' : Graph} {parent : Nat} {path : List Label} {inter : Nat}
    (h : g.factor parent path inter = .ok g') (hw : g.WF) (hb : g.Back) : g'.Back := by
  unfold factor at h
  simp only [bind_ok, Graph.get] at h
  obtain ⟨pinfo, hp, h⟩ := h
  split at hp
  · next pinfo' hpar =>
    simp only [pure, Except.pure, Except.ok.injEq] at hp
    subst hp
    obtain ⟨⟨B, C⟩, hrem, h⟩ := h
    obtain ⟨g2, hclean, h⟩ := h
    obtain ⟨g3, hadd, hlast⟩ := h
    obtain ⟨-, -, rdom, rclean, -⟩ :=
      factor_remove (path := path) (factorNeeds pinfo' path) pinfo'.borrowedBy B [] C
        (fun n hn => by
          simp only [factorNeeds, List.mem_flatMap, List.mem_map, List.mem_filter] at hn
          obtain ⟨_, _, e, ⟨_, hpre⟩, rfl⟩ := hn
          exact hpre)
        (fun y s hs => hw.sets parent pinfo' y s hpar hs) hrem
    have hf1 : ∀ x, NatMap.find? (g.insert parent { pinfo' with borrowedBy := B }) x =
        if x = parent then some { pinfo' with borrowedBy := B } else g.find? x :=
      fun x => NatMap.find?_insert _ _ _ _
    -- `g1`: the parent keeps a subset of its edge sets.
    have hb1 : Graph.Back (g.insert parent { pinfo' with borrowedBy := B }) := by
      intro x xi y s hx hs
      have hy : ∀ yi, g.find? y = some yi → x ∈ yi.borrowsFrom →
          ∃ yi', NatMap.find? (g.insert parent { pinfo' with borrowedBy := B }) y = some yi' ∧
            x ∈ yi'.borrowsFrom := by
        intro yi hyi hxy
        rw [hf1]
        split
        · next hyp =>
          subst hyp; rw [hpar, Option.some.injEq] at hyi; subst hyi
          exact ⟨_, rfl, hxy⟩
        · exact ⟨yi, hyi, hxy⟩
      rw [hf1] at hx
      split at hx
      · next hxp =>
        subst hxp
        simp only [Option.some.injEq] at hx
        subst hx
        obtain ⟨s0, hs0⟩ := rdom y s hs
        obtain ⟨yi, hyi, hxy⟩ := hb x pinfo' y s0 hpar hs0
        exact hy yi hyi hxy
      · obtain ⟨yi, hyi, hxy⟩ := hb x xi y s hx hs
        exact hy yi hyi hxy
    obtain ⟨hdrop, -⟩ := factor_cleanup parent C _ g2 hclean
    have hb2 : g2.Back := hdrop.back hb1 (fun y hy pi hpi => by
      rw [hf1] at hpi
      simp only [↓reduceIte, Option.some.injEq] at hpi
      subst hpi
      rcases rclean y hy with hy | hy
      · simp at hy
      · exact hy)
    exact addEdge_back hlast (fold_back (fun _ _ _ hb h => addEdge_back h hb) _ _ _ hb2 hadd)
  · simp [panicAt] at hp

end Graph

namespace Graph

/-- Every edge set of `g` is recorded by its child, except those `P` allows. -/
def BackUnless (P : Nat → Nat → Prop) (g : Graph) : Prop :=
  ∀ p pi c s, g.find? p = some pi → pi.borrowedBy.find? c = some s →
    P p c ∨ ∃ ci, g.find? c = some ci ∧ p ∈ ci.borrowsFrom

theorem addEdge_backUnless {P : Nat → Nat → Prop} {g g' : Graph} {p c : Nat}
    {e : BorrowGraph.Edge} (h : g.addEdge p e c = .ok g') (hb : g.BackUnless P) :
    g'.BackUnless P := by
  have hne := (addEdge_ok h).1
  obtain ⟨pi, ci, hp, hc, hf⟩ := addEdge_find h
  have hy : ∀ y yi x, g.find? y = some yi → x ∈ yi.borrowsFrom →
      ∃ yi', g'.find? y = some yi' ∧ x ∈ yi'.borrowsFrom := by
    intro y yi x hyi hx
    rw [hf]
    by_cases hyc : y = c
    · rw [hyc, hc, Option.some.injEq] at hyi; subst hyi
      exact ⟨{ ci with borrowsFrom := insertNat p ci.borrowsFrom }, by simp [hyc],
        (mem_insertNat _ _ _).2 (Or.inr hx)⟩
    · by_cases hyp : y = p
      · rw [hyp, hp, Option.some.injEq] at hyi; subst hyi
        exact ⟨{ pi with borrowedBy := pi.borrowedBy.insert c ((g.edgeSet p c).insert e) },
          by simp [hyp, hne], hx⟩
      · exact ⟨yi, by simp [hyc, hyp, hyi], hx⟩
  have lift : ∀ x xi y s, g.find? x = some xi → xi.borrowedBy.find? y = some s →
      P x y ∨ ∃ yi', g'.find? y = some yi' ∧ x ∈ yi'.borrowsFrom := by
    intro x xi y s hx hs
    rcases hb x xi y s hx hs with hP | ⟨yi, hyi, hxy⟩
    · exact Or.inl hP
    · exact Or.inr (hy y yi x hyi hxy)
  intro x xi y s hx hs
  rw [hf] at hx
  by_cases hxc : x = c
  · subst hxc
    simp only [↓reduceIte, Option.some.injEq] at hx
    subst hx
    exact lift x ci y s hc hs
  · by_cases hxp : x = p
    · subst hxp
      simp only [hxc, ↓reduceIte, Option.some.injEq] at hx
      subst hx
      simp only [NatMap.find?_insert] at hs
      split at hs
      · next hyc =>
        subst hyc
        refine Or.inr ⟨{ ci with borrowsFrom := insertNat x ci.borrowsFrom }, ?_,
          (mem_insertNat _ _ _).2 (Or.inl rfl)⟩
        rw [hf]; simp
      · exact lift x pi y s hp hs
    · simp only [hxc, hxp, ↓reduceIte] at hx
      exact lift x xi y s hx hs

theorem fold_backUnless {P : Nat → Nat → Prop} {α : Type} {step : Graph → α → VResult Graph}
    (hstep : ∀ g a g', g.BackUnless P → step g a = .ok g' → g'.BackUnless P) :
    ∀ (l : List α) (g g' : Graph), g.BackUnless P → l.foldlM step g = .ok g' → g'.BackUnless P
  | [], g, g', hb, h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h; exact hb
  | a :: l, g, g', hb, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨g1, h1, h⟩ := h
    exact fold_backUnless hstep l g1 g' (hstep g a g1 hb h1) h

theorem splice_backUnless {P : Nat → Nat → Prop} {g g' : Graph} {p c : Nat}
    {pe ce : BorrowGraph.Edge} (h : g.splice p pe c ce = .ok g') (hb : g.BackUnless P) :
    g'.BackUnless P := by
  unfold splice at h
  split at h
  · simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h; exact hb
  · exact addEdge_backUnless h hb

theorem addEdge_none {g g' : Graph} {p c : Nat} {e : BorrowGraph.Edge}
    (h : g.addEdge p e c = .ok g') {x : Nat} (hx : g.find? x = none) : g'.find? x = none := by
  obtain ⟨pi, ci, hp, hc, hf⟩ := addEdge_find h
  have hxc : x ≠ c := fun e => by subst e; rw [hx] at hc; cases hc
  have hxp : x ≠ p := fun e => by subst e; rw [hx] at hp; cases hp
  rw [hf]; simp [hxc, hxp, hx]

theorem splice_none {g g' : Graph} {p c : Nat} {pe ce : BorrowGraph.Edge}
    (h : g.splice p pe c ce = .ok g') {x : Nat} (hx : g.find? x = none) : g'.find? x = none := by
  unfold splice at h
  split at h
  · simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h; exact hx
  · exact addEdge_none h hx

theorem fold_none {α : Type} {x : Nat} {step : Graph → α → VResult Graph}
    (hstep : ∀ g a g', step g a = .ok g' → g.find? x = none → g'.find? x = none) :
    ∀ (l : List α) (g g' : Graph), l.foldlM step g = .ok g' → g.find? x = none →
      g'.find? x = none
  | [], g, g', h, hn => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h; exact hn
  | a :: l, g, g', h, hn => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨g1, h1, h⟩ := h
    exact fold_none hstep l g1 g' h (hstep g a g1 h1 hn)

theorem release_parents_back {id : Nat} {r : RefInfo} :
    ∀ (L done : List Nat) (gc gc' : Graph), gc.find? id = none →
      gc.BackUnless (fun p c => c = id ∧ p ∉ done ∧ p ∈ r.borrowsFrom) →
      L.foldlM (fun (g : Graph) parent => do
        let p ← g.get parent
        let parentEdges ← match p.borrowedBy.find? id with
          | some s => pure s
          | none => panicAt "release: missing parent edge"
        let g : Graph := g.insert parent { p with borrowedBy := p.borrowedBy.erase id }
        parentEdges.edges.foldlM (fun (g : Graph) pe =>
          r.borrowedBy.foldlM (fun (g : Graph) ((child, childEdges) : Nat × EdgeSet) =>
            childEdges.edges.foldlM (fun (g : Graph) ce => g.splice parent pe child ce) g) g) g)
        gc = .ok gc' →
      gc'.find? id = none ∧
        gc'.BackUnless (fun p c => c = id ∧ p ∉ done ++ L ∧ p ∈ r.borrowsFrom)
  | [], done, gc, gc', hgone, hb, h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h; simpa using And.intro hgone hb
  | q :: L, done, gc, gc', hgone, hb, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨g1, h1, h⟩ := h
    suffices g1.find? id = none ∧
        g1.BackUnless (fun p c => c = id ∧ p ∉ done ++ [q] ∧ p ∈ r.borrowsFrom) by
      simpa using release_parents_back L (done ++ [q]) g1 gc' this.1 this.2 h
    simp only [Graph.get] at h1
    obtain ⟨pi, hpi, h1⟩ := h1
    split at hpi
    · next pi' hq =>
      simp only [pure, Except.pure, Except.ok.injEq] at hpi
      subst hpi
      split at h1
      · next s' hs' =>
        simp only [pure, Except.pure, bind, Except.bind] at h1
        have hqid : q ≠ id := fun e => by subst e; rw [hgone] at hq; cases hq
        have hf0 : ∀ x, NatMap.find? (gc.insert q { pi' with borrowedBy := pi'.borrowedBy.erase id }) x =
            if x = q then some { pi' with borrowedBy := pi'.borrowedBy.erase id } else gc.find? x :=
          fun x => NatMap.find?_insert _ _ _ _
        -- After dropping the parent's edges to `id`.
        have hb0 : Graph.BackUnless (fun p c => c = id ∧ p ∉ done ++ [q] ∧ p ∈ r.borrowsFrom)
            (gc.insert q { pi' with borrowedBy := pi'.borrowedBy.erase id }) := by
          intro x xi y s hx hs
          have hy : ∀ yi, gc.find? y = some yi → x ∈ yi.borrowsFrom →
              ∃ yi', NatMap.find? (gc.insert q { pi' with borrowedBy := pi'.borrowedBy.erase id }) y =
                some yi' ∧ x ∈ yi'.borrowsFrom := by
            intro yi hyi hxy
            rw [hf0]
            split
            · next hyq =>
              subst hyq; rw [hq, Option.some.injEq] at hyi; subst hyi
              exact ⟨_, rfl, hxy⟩
            · exact ⟨yi, hyi, hxy⟩
          rw [hf0] at hx
          split at hx
          · next hxq =>
            subst hxq
            simp only [Option.some.injEq] at hx
            subst hx
            rw [NatMap.find?_erase] at hs
            split at hs
            · cases hs
            · next hyid =>
              rcases hb x pi' y s hq hs with ⟨hyid', -⟩ | ⟨yi, hyi, hxy⟩
              · exact absurd hyid' hyid
              · exact Or.inr (hy yi hyi hxy)
          · next hxq =>
            rcases hb x xi y s hx hs with ⟨hyid, hxd, hxr⟩ | ⟨yi, hyi, hxy⟩
            · exact Or.inl ⟨hyid, by simp [hxd, hxq], hxr⟩
            · exact Or.inr (hy yi hyi hxy)
        refine ⟨?_, fold_backUnless (fun g pe g' hb h =>
          fold_backUnless (fun g a g' hb h =>
            fold_backUnless (fun _ _ _ hb h => splice_backUnless h hb) _ _ _ hb h) _ _ _ hb h)
          _ _ _ hb0 h1⟩
        -- `id` is still gone: no step adds a reference.
        refine fold_none (fun g pe g' h hn => fold_none (fun g a g' h hn =>
          fold_none (fun _ _ _ h hn => splice_none h hn) _ _ _ h hn) _ _ _ h hn) _ _ _ h1 ?_
        rw [hf0]; simp [Ne.symm hqid, hgone]
      · simp [panicAt, bind, Except.bind] at h1
    · simp [panicAt] at hpi

/-- Releasing keeps every edge set recorded by its child. -/
theorem release_back {g g' : Graph} {id : Nat} (h : g.release id = .ok g') (hb : g.Back) :
    g'.Back := by
  unfold release at h
  simp only [bind_ok, Graph.get] at h
  obtain ⟨r, hr, g2, hpar, hkids⟩ := h
  split at hr
  · next r' hid =>
    simp only [pure, Except.pure, Except.ok.injEq] at hr
    subst hr
    have hb0 : Graph.BackUnless (fun p c => c = id ∧ p ∉ ([] : List Nat) ∧ p ∈ r'.borrowsFrom)
        (g.erase id) := by
      intro x xi y s hx hs
      rw [NatMap.find?_erase] at hx
      split at hx
      · cases hx
      · obtain ⟨yi, hyi, hxy⟩ := hb x xi y s hx hs
        by_cases hyid : y = id
        · subst hyid
          rw [hid, Option.some.injEq] at hyi
          subst hyi
          exact Or.inl ⟨rfl, by simp, hxy⟩
        · exact Or.inr ⟨yi, by simp [NatMap.find?_erase, hyid, hyi], hxy⟩
    obtain ⟨hgone, hb2⟩ := release_parents_back r'.borrowsFrom [] (g.erase id) g2
      (by simp [NatMap.find?_erase]) hb0 hpar
    have hb2' : g2.Back := by
      intro x xi y s hx hs
      rcases hb2 x xi y s hx hs with ⟨-, hxd, hxr⟩ | hb
      · exact absurd (by simpa using hxr) hxd
      · exact hb
    obtain ⟨hdrop, -⟩ := release_kids id r'.borrowedBy g2 g' hkids
    exact hdrop.back hb2' (fun y _ pi hpi => by rw [hgone] at hpi; cases hpi)
  · simp [panicAt] at hr

end Graph

/-! ## Joining graphs -/

namespace Graph

/-- An edge set of `g` covers edge `e` from `p` to `c`: `g` keeps an edge set
from `p` to `c` with an edge covering `e`. -/
def RawCovers (g : Graph) (p : Nat) (e : BorrowGraph.Edge) (c : Nat) : Prop :=
  ∃ pi s e', g.find? p = some pi ∧ pi.borrowedBy.find? c = some s ∧ e' ∈ s.edges ∧
    e'.leq e = true

theorem RawCovers.hasEdge {g : Graph} (hb : g.Back) {p c : Nat} {e : BorrowGraph.Edge}
    (h : g.RawCovers p e c) : g.HasEdge p e c := by
  obtain ⟨pi, s, e', hp, hs, he', hle⟩ := h
  exact ⟨e', hb.edge hp hs he', hle⟩

theorem foldlM_append_mono {α β : Type} {f : List β → α → VResult (List β)}
    (hf : ∀ acc a acc', f acc a = .ok acc' → ∃ ext, acc' = acc ++ ext) :
    ∀ (l : List α) (acc acc' : List β), l.foldlM f acc = .ok acc' → ∃ ext, acc' = acc ++ ext
  | [], acc, acc', h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    exact ⟨[], by simp [h]⟩
  | a :: l, acc, acc', h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨acc1, h1, h⟩ := h
    obtain ⟨e1, rfl⟩ := hf acc a acc1 h1
    obtain ⟨e2, rfl⟩ := foldlM_append_mono hf l _ acc' h
    exact ⟨e1 ++ e2, by simp⟩

/-- A fold that only appends, and whose steps each establish a fact about
their element that appending keeps, establishes it for every element. -/
theorem foldlM_collect {α β : Type} {F : List β → α → VResult (List β)} (Q : α → List β → Prop)
    (hstep : ∀ acc a acc', F acc a = .ok acc' → (∃ ext, acc' = acc ++ ext) ∧ Q a acc')
    (hmono : ∀ a acc ext, Q a acc → Q a (acc ++ ext)) :
    ∀ (l : List α) (acc acc' : List β), l.foldlM F acc = .ok acc' →
      (∃ ext, acc' = acc ++ ext) ∧ ∀ a ∈ l, Q a acc'
  | [], acc, acc', h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    exact ⟨⟨[], by simp [h]⟩, by simp⟩
  | a :: l, acc, acc', h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨acc1, h1, h⟩ := h
    obtain ⟨⟨e1, rfl⟩, hq1⟩ := hstep acc a acc1 h1
    obtain ⟨⟨e2, rfl⟩, hrest⟩ := foldlM_collect Q hstep hmono l _ acc' h
    refine ⟨⟨e1 ++ e2, by simp⟩, fun b hb => ?_⟩
    rcases List.mem_cons.1 hb with rfl | hb
    · simpa using hmono b _ e2 hq1
    · exact hrest b hb

/-- **Unmatched edges**: every edge of `other` is covered by an edge set of
`self`, or listed. -/
theorem unmatched_ok {self other : Graph} {U : List (Nat × Nat × BorrowGraph.Edge)}
    (h : self.unmatched other = .ok U) :
    ∀ p pi c s e, other.find? p = some pi → pi.borrowedBy.find? c = some s → e ∈ s.edges →
      self.RawCovers p e c ∨ (p, c, e) ∈ U := by
  unfold unmatched at h
  have := foldlM_collect (fun (r : Nat × RefInfo) acc => ∀ c s e, r.2.borrowedBy.find? c = some s →
      e ∈ s.edges → self.RawCovers r.1 e c ∨ (r.1, c, e) ∈ acc) ?_ ?_ _ _ _ h
  · intro p pi c s e hp hs he
    exact this.2 (p, pi) (lookup_mem hp) c s e hs he
  · -- One reference of `other`.
    rintro acc ⟨p, pi⟩ acc' hstep
    simp only at hstep
    split at hstep
    · next spi hsp =>
      simp only [pure, Except.pure, bind, Except.bind] at hstep
      have hmid := foldlM_collect (fun (k : Nat × EdgeSet) acc => ∀ e ∈ k.2.edges,
          self.RawCovers p e k.1 ∨ (p, k.1, e) ∈ acc) ?_ ?_ _ _ _ hstep
      · refine ⟨hmid.1, fun c s e hs he => ?_⟩
        exact hmid.2 (c, s) (lookup_mem hs) e he
      · -- One child.
        rintro acc ⟨c, os⟩ acc' hk
        simp only at hk
        have hin := foldlM_collect (fun (oe : BorrowGraph.Edge) acc =>
            self.RawCovers p oe c ∨ (p, c, oe) ∈ acc) ?_ ?_ _ _ _ hk
        · exact ⟨hin.1, fun e he => hin.2 e he⟩
        · -- One edge.
          rintro acc oe acc' he
          cases hsc : spi.borrowedBy.find? c with
          | some s =>
            simp only [hsc] at he
            by_cases hany : (s.edges.any fun x => x.leq oe) = true
            · simp only [hany, ↓reduceIte, Except.ok.injEq] at he
              subst he
              obtain ⟨e', he', hle⟩ := List.any_eq_true.1 hany
              exact ⟨⟨[], by simp⟩, Or.inl ⟨spi, s, e', hsp, hsc, he', hle⟩⟩
            · simp only [hany, Bool.false_eq_true, ↓reduceIte] at he
              split at he
              · simp [panicAt] at he
              · simp only [Except.ok.injEq] at he
                subst he
                exact ⟨⟨_, rfl⟩, Or.inr (by simp)⟩
          | none =>
            simp only [hsc, Bool.false_eq_true, ↓reduceIte] at he
            split at he
            · simp [panicAt] at he
            · simp only [Except.ok.injEq] at he
              subst he
              exact ⟨⟨_, rfl⟩, Or.inr (by simp)⟩
        · intro oe acc ext hq
          rcases hq with hq | hq
          · exact Or.inl hq
          · exact Or.inr (List.mem_append_left _ hq)
      · intro k acc ext hq e he
        rcases hq e he with hq | hq
        · exact Or.inl hq
        · exact Or.inr (List.mem_append_left _ hq)
    · simp [panicAt, bind, Except.bind] at hstep
  · intro r acc ext hq c s e hs he
    rcases hq c s e hs he with hq | hq
    · exact Or.inl hq
    · exact Or.inr (List.mem_append_left _ hq)

end Graph

namespace Graph

/-- **Joining** adds, up to covering, every edge of `other` to `self`. -/
theorem join_ok {self other j : Graph} (h : self.join other = .ok j) (hw : self.WF)
    (hb : self.Back) :
    j.WF ∧ j.Back ∧ self.Covers j ∧ (∀ x, j.mutability x = self.mutability x) ∧
      ∀ p pi c s e, other.find? p = some pi → pi.borrowedBy.find? c = some s → e ∈ s.edges →
        j.HasEdge p e c := by
  unfold join at h
  simp only [bind_ok] at h
  obtain ⟨U, hU, h⟩ := h
  have hstep : ∀ (g : Graph) (a : Nat × Nat × BorrowGraph.Edge) (g' : Graph), g.WF →
      g.addEdge a.1 a.2.2 a.2.1 = .ok g' →
      g'.WF ∧ g.Covers g' ∧ ∀ x, g'.mutability x = g.mutability x :=
    fun _ _ _ hw h => addEdge_step h hw
  obtain ⟨hwj, hcov, hmut⟩ := fold_covers hstep _ _ _ hw h
  have hadds := fold_adds hstep (fun g a => g.HasEdge a.1 a.2.2 a.2.1)
    (fun _ _ _ hw h => (addEdge_covers h hw.sets).2) (fun _ _ _ hc hq => HasEdge.mono hc hq)
    _ _ _ hw h
  refine ⟨hwj, fold_back (fun _ _ _ hb h => addEdge_back h hb) _ _ _ hb h, hcov, hmut,
    fun p pi c s e hp hs he => ?_⟩
  rcases unmatched_ok hU p pi c s e hp hs he with hc | hc
  · exact HasEdge.mono hcov (hc.hasEdge hb)
  · exact hadds (p, c, e) hc

/-- `leq` reports that every edge of `other` is covered by an edge set of
`self`. -/
theorem leq_ok {self other : Graph} (h : self.leq other = .ok true) :
    ∀ p pi c s e, other.find? p = some pi → pi.borrowedBy.find? c = some s → e ∈ s.edges →
      self.RawCovers p e c := by
  unfold leq at h
  simp only [bind_ok] at h
  obtain ⟨U, hU, h⟩ := h
  simp only [pure, Except.pure, Except.ok.injEq, List.isEmpty_iff] at h
  subst h
  intro p pi c s e hp hs he
  rcases unmatched_ok hU p pi c s e hp hs he with hc | hc
  · exact hc
  · simp at hc

/-! ## Queries -/

theorem get_ok {g : Graph} {id : Nat} {r : RefInfo} : g.get id = .ok r ↔ g.find? id = some r := by
  unfold get
  split <;> simp_all [panicAt, pure, Except.pure]

theorem isMutable_ok {g : Graph} {id : Nat} {b : Bool} (h : g.isMutable id = .ok b) :
    g.mutability id = some b := by
  unfold isMutable at h
  simp only [bind_ok, get_ok] at h
  obtain ⟨r, hr, h⟩ := h
  simp only [pure, Except.pure, Except.ok.injEq] at h
  simp [mutability, hr, h]

/-- No full borrows: every edge from `id` has a non-empty path. -/
theorem hasFullBorrows_false {g : Graph} {id : Nat} (h : g.hasFullBorrows id = .ok false)
    {e : BorrowGraph.Edge} {c : Nat} (he : g.Edge id e c) : e.path ≠ [] := by
  unfold hasFullBorrows at h
  simp only [bind_ok, get_ok] at h
  obtain ⟨r, hr, h⟩ := h
  simp only [pure, Except.pure, Except.ok.injEq, List.any_eq_false] at h
  obtain ⟨pi, s, -, hp, hs, he', -⟩ := he
  rw [hr, Option.some.injEq] at hp
  subst hp
  have := h (c, s) (lookup_mem hs)
  simp only [List.any_eq_true, not_exists, not_and, List.isEmpty_iff] at this
  exact this e he'

/-- No consistent borrows at `label?`: no edge from `id` counts at it. -/
theorem hasConsistentBorrows_false {g : Graph} {id : Nat} {label? : Option Label}
    (h : g.hasConsistentBorrows id label? = .ok false) {e : BorrowGraph.Edge} {c : Nat}
    (he : g.Edge id e c) : atLabel label? e = false := by
  unfold hasConsistentBorrows at h
  simp only [bind_ok, get_ok] at h
  obtain ⟨r, hr, h⟩ := h
  simp only [pure, Except.pure, Except.ok.injEq, List.any_eq_false] at h
  obtain ⟨pi, s, -, hp, hs, he', -⟩ := he
  rw [hr, Option.some.injEq] at hp
  subst hp
  have := h (c, s) (lookup_mem hs)
  simp only [List.any_eq_true, not_exists, not_and] at this
  simpa using this e he'

theorem anyM_false {α : Type} {f : α → VResult Bool} :
    ∀ {l : List α}, l.anyM f = .ok false → ∀ a ∈ l, f a = .ok false
  | [], _, a, ha => by simp at ha
  | b :: l, h, a, ha => by
    simp only [List.anyM, bind_ok] at h
    obtain ⟨x, hx, h⟩ := h
    cases x with
    | true => simp [pure, Except.pure] at h
    | false =>
      simp only at h
      rcases List.mem_cons.1 ha with rfl | ha
      · exact hx
      · exact anyM_false h a ha

/-- No consistent mutable borrows at `label?`: every edge from `id` that
counts at it goes to an immutable reference. -/
theorem hasConsistentMutableBorrows_false {g : Graph} {id : Nat} {label? : Option Label}
    (h : g.hasConsistentMutableBorrows id label? = .ok false) {e : BorrowGraph.Edge} {c : Nat}
    (he : g.Edge id e c) (hl : atLabel label? e = true) : g.mutability c = some false := by
  unfold hasConsistentMutableBorrows at h
  simp only [bind_ok, get_ok] at h
  obtain ⟨r, hr, h⟩ := h
  obtain ⟨pi, s, -, hp, hs, he', -⟩ := he
  rw [hr, Option.some.injEq] at hp
  subst hp
  have := anyM_false h (c, s) (lookup_mem hs)
  simp only at this
  have hany : (s.edges.any (atLabel label?)) = true := List.any_eq_true.2 ⟨e, he', hl⟩
  simp only [hany, ↓reduceIte] at this
  exact isMutable_ok this

/-- A writable reference is mutable and borrowed by nothing. -/
theorem isWritable_true {g : Graph} {id : Nat} (h : g.isWritable id = .ok true) :
    g.mutability id = some true ∧ ∀ e c, ¬ g.Edge id e c := by
  unfold isWritable at h
  simp only [bind_ok] at h
  obtain ⟨m, hm, h⟩ := h
  split at h
  · simp [panicAt, bind, Except.bind] at h
  · next hmut =>
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hmut
    subst hmut
    simp only [pure, Except.pure, bind, Except.bind] at h
    obtain ⟨b, hb, h⟩ := bind_ok.1 h
    simp only [Except.ok.injEq, Bool.not_eq_true'] at h
    subst h
    refine ⟨isMutable_ok hm, fun e c he => ?_⟩
    have := hasConsistentBorrows_false hb he
    simp [atLabel] at this
    split at this <;> simp_all

/-- A freezable reference is mutable, and every edge from it counting at
`label?` goes to an immutable reference. -/
theorem isFreezable_true {g : Graph} {id : Nat} {label? : Option Label}
    (h : g.isFreezable id label? = .ok true) :
    g.mutability id = some true ∧ ∀ e c, g.Edge id e c → atLabel label? e = true →
      g.mutability c = some false := by
  unfold isFreezable at h
  simp only [bind_ok] at h
  obtain ⟨m, hm, h⟩ := h
  split at h
  · simp [panicAt, bind, Except.bind] at h
  · next hmut =>
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hmut
    subst hmut
    simp only [pure, Except.pure, bind, Except.bind] at h
    obtain ⟨b, hb, h⟩ := bind_ok.1 h
    simp only [Except.ok.injEq, Bool.not_eq_true'] at h
    subst h
    exact ⟨isMutable_ok hm, fun e c he hl => hasConsistentMutableBorrows_false hb he hl⟩

/-- A readable reference is immutable, or freezable. -/
theorem isReadable_true {g : Graph} {id : Nat} {label? : Option Label}
    (h : g.isReadable id label? = .ok true) :
    g.mutability id = some false ∨ (∀ e c, g.Edge id e c → atLabel label? e = true →
      g.mutability c = some false) := by
  unfold isReadable at h
  simp only [bind_ok] at h
  obtain ⟨m, hm, h⟩ := h
  cases m with
  | false => exact Or.inl (isMutable_ok hm)
  | true =>
    simp only [Bool.not_true, Bool.false_eq_true, ↓reduceIte] at h
    exact Or.inr (isFreezable_true h).2

end Graph

/-! ## Renaming references -/

namespace BorrowGraph.NatMap

theorem lookup_append_one {α : Type} : ∀ (l : List (Nat × α)) (k x : Nat) (v : α),
    (l ++ [(k, v)]).lookup x = match l.lookup x with
      | some w => some w
      | none => if x = k then some v else none
  | [], k, x, v => by
    by_cases h : x = k
    · subst h; simp [List.lookup]
    · have : (x == k) = false := by simpa using h
      simp [List.lookup, this, h]
  | (k', w) :: rest, k, x, v => by
    by_cases h : x = k'
    · subst h; simp [List.lookup]
    · have : (x == k') = false := by simpa using h
      simp only [List.cons_append, List.lookup, this]
      exact lookup_append_one rest k x v

theorem foldl_insert_find? {α : Type} : ∀ (l : List (Nat × α)) (m : NatMap α) (x : Nat),
    (l.foldl (fun (m : NatMap α) (kv : Nat × α) => m.insert kv.1 kv.2) m).find? x =
      match l.reverse.lookup x with
      | some v => some v
      | none => m.find? x
  | [], m, x => by simp
  | (k, v) :: rest, m, x => by
    simp only [List.foldl_cons, List.reverse_cons]
    rw [foldl_insert_find? rest (m.insert k v) x, lookup_append_one, find?_insert]
    cases rest.reverse.lookup x with
    | some w => simp
    | none => by_cases hx : x = k <;> simp [hx]

theorem ofList_eq {α : Type} (l : List (Nat × α)) :
    NatMap.ofList l = l.foldl (fun (m : NatMap α) (kv : Nat × α) => m.insert kv.1 kv.2) [] := rfl

theorem lookup_of_mem_nodup {α : Type} : ∀ {l : List (Nat × α)} {k : Nat} {v : α},
    (l.map (·.1)).Nodup → (k, v) ∈ l → l.lookup k = some v
  | [], _, _, _, h => by simp at h
  | (k', w) :: rest, k, v, hnd, h => by
    simp only [List.map_cons, List.nodup_cons, List.mem_map] at hnd
    rcases List.mem_cons.1 h with heq | h
    · simp only [Prod.mk.injEq] at heq
      obtain ⟨rfl, rfl⟩ := heq
      simp [List.lookup]
    · have hne : k ≠ k' := fun e => hnd.1 ⟨(k, v), h, e⟩
      have : (k == k') = false := by simpa using hne
      simp only [List.lookup, this]
      exact lookup_of_mem_nodup hnd.2 h

/-- An association list with distinct keys, collected into a map, is read
back as itself. -/
theorem find?_ofList {α : Type} {l : List (Nat × α)} (hnd : (l.map (·.1)).Nodup) (x : Nat)
    (v : α) : (NatMap.ofList l).find? x = some v ↔ (x, v) ∈ l := by
  rw [ofList_eq, foldl_insert_find?]
  constructor
  · intro h
    split at h
    · next w hw =>
      simp only [Option.some.injEq] at h
      subst h
      exact List.mem_reverse.1 (Graph.lookup_mem (m := l.reverse) hw)
    · simp [NatMap.find?, List.lookup] at h
  · intro h
    have hnd' : (l.reverse.map (·.1)).Nodup := by
      rw [List.map_reverse]
      unfold List.Nodup at hnd ⊢
      rw [List.pairwise_reverse]
      exact hnd.imp fun h e => h e.symm
    rw [lookup_of_mem_nodup hnd' (List.mem_reverse.2 h)]

theorem ofList_sorted {α : Type} (l : List (Nat × α)) :
    ((NatMap.ofList l).map (·.1)).Pairwise (· < ·) := by
  rw [ofList_eq]
  suffices ∀ (m : NatMap α), (m.map (·.1)).Pairwise (· < ·) →
      ((l.foldl (fun (m : NatMap α) (kv : Nat × α) => m.insert kv.1 kv.2) m).map (·.1)).Pairwise (· < ·) from
    this [] (by simp)
  induction l with
  | nil => intro m hm; exact hm
  | cons kv rest ih => intro m hm; exact ih _ (insert_keys_pairwise _ _ _ hm).1

end BorrowGraph.NatMap

namespace Graph

/-- A reference's information, with references renamed by `F`. -/
def renameInfo (F : Nat → Nat) (r : RefInfo) : RefInfo :=
  { r with
    borrowedBy := NatMap.ofList (r.borrowedBy.map fun cs => (F cs.1, cs.2))
    borrowsFrom := r.borrowsFrom.foldl (fun s p => insertNat (F p) s) [] }

theorem remap_eq (g : Graph) (idMap : NatMap Nat) :
    g.remap idMap = NatMap.ofList (g.map fun x =>
      ((fun id => (idMap.find? id).getD id) x.1,
        renameInfo (fun id => (idMap.find? id).getD id) x.2)) := rfl

theorem mem_foldl_insertNat (F : Nat → Nat) : ∀ (l acc : List Nat) (x : Nat),
    (x ∈ acc ∨ ∃ y ∈ l, x = F y) → x ∈ l.foldl (fun s p => insertNat (F p) s) acc
  | [], acc, x, h => by
    rcases h with h | ⟨y, hy, -⟩
    · exact h
    · simp at hy
  | y :: l, acc, x, h => by
    simp only [List.foldl_cons]
    refine mem_foldl_insertNat F l _ x ?_
    rcases h with h | ⟨z, hz, rfl⟩
    · exact Or.inl ((mem_insertNat _ _ _).2 (Or.inr h))
    · rcases List.mem_cons.1 hz with rfl | hz
      · exact Or.inl ((mem_insertNat _ _ _).2 (Or.inl rfl))
      · exact Or.inr ⟨z, hz, rfl⟩

theorem sorted_nodup {l : List Nat} (h : l.Pairwise (· < ·)) : l.Nodup :=
  h.imp fun hlt heq => by subst heq; exact Nat.lt_irrefl _ hlt

theorem nodup_map_injOn {α β : Type} {f : α → β} {l : List α} (hl : l.Nodup)
    (hinj : ∀ x ∈ l, ∀ y ∈ l, f x = f y → x = y) : (l.map f).Nodup := by
  unfold List.Nodup at *
  rw [List.pairwise_map]
  exact hl.imp_of_mem fun ha hb hne heq => hne (hinj _ ha _ hb heq)

/-- **Renaming** with `F`, injective on the references of `g`, keeps every
reference, its mutability and its edges, renamed. -/
theorem remap_ok {g : Graph} {idMap : NatMap Nat} (hw : g.WF) (hb : g.Back)
    (hinj : ∀ x y xi yi, g.find? x = some xi → g.find? y = some yi →
      (idMap.find? x).getD x = (idMap.find? y).getD y → x = y) :
    (∀ x xi, g.find? x = some xi →
        (g.remap idMap).find? ((idMap.find? x).getD x) =
          some (renameInfo (fun id => (idMap.find? id).getD id) xi)) ∧
      (∀ y v, (g.remap idMap).find? y = some v →
        ∃ x xi, g.find? x = some xi ∧ y = (idMap.find? x).getD x ∧
          v = renameInfo (fun id => (idMap.find? id).getD id) xi) ∧
      (∀ p e c, g.Edge p e c →
        (g.remap idMap).Edge ((idMap.find? p).getD p) e ((idMap.find? c).getD c)) ∧
      (g.remap idMap).WF ∧ (g.remap idMap).Back := by
  generalize hF : (fun id => (idMap.find? id).getD id) = F
  have hFx : ∀ x, (idMap.find? x).getD x = F x := fun x => by rw [← hF]
  simp only [hFx]
  have hinj' : ∀ x y xi yi, g.find? x = some xi → g.find? y = some yi → F x = F y → x = y :=
    fun x y xi yi hx hy h => hinj x y xi yi hx hy (by rw [hFx, hFx]; exact h)
  have hgnd : (g.map (·.1)).Nodup := sorted_nodup hw.sorted
  have hmemg : ∀ x xi, (x, xi) ∈ g → g.find? x = some xi := fun x xi h =>
    NatMap.lookup_of_mem_nodup hgnd h
  -- The renamed list has distinct keys.
  have hLnd : ((g.map fun x => (F x.1, renameInfo F x.2)).map (·.1)).Nodup := by
    rw [List.map_map]
    have : (g.map ((·.1) ∘ fun x : Nat × RefInfo => (F x.1, renameInfo F x.2))) =
        (g.map (·.1)).map F := by simp [List.map_map]
    rw [this]
    refine nodup_map_injOn hgnd fun x hx y hy hxy => ?_
    obtain ⟨⟨x', xi⟩, hx', rfl⟩ := List.mem_map.1 hx
    obtain ⟨⟨y', yi⟩, hy', rfl⟩ := List.mem_map.1 hy
    exact hinj' _ _ _ _ (hmemg _ _ hx') (hmemg _ _ hy') hxy
  have hfind : ∀ y v, (g.remap idMap).find? y = some v ↔
      (y, v) ∈ g.map fun x => (F x.1, renameInfo F x.2) := by
    intro y v; rw [remap_eq, hF]; exact NatMap.find?_ofList hLnd y v
  -- A reference's renamed borrowers have distinct keys.
  have hinnd : ∀ x xi, g.find? x = some xi →
      ((xi.borrowedBy.map fun cs => (F cs.1, cs.2)).map (·.1)).Nodup := by
    intro x xi hx
    have hknd := sorted_nodup (hw.inner x xi hx)
    rw [List.map_map]
    have : (xi.borrowedBy.map ((·.1) ∘ fun cs : Nat × EdgeSet => (F cs.1, cs.2))) =
        (xi.borrowedBy.map (·.1)).map F := by simp [List.map_map]
    rw [this]
    refine nodup_map_injOn hknd fun c hc d hd hcd => ?_
    obtain ⟨⟨c', cs⟩, hc', rfl⟩ := List.mem_map.1 hc
    obtain ⟨⟨d', ds⟩, hd', rfl⟩ := List.mem_map.1 hd
    obtain ⟨ci, hci, -⟩ := hb x xi c' cs hx (NatMap.lookup_of_mem_nodup hknd hc')
    obtain ⟨di, hdi, -⟩ := hb x xi d' ds hx (NatMap.lookup_of_mem_nodup hknd hd')
    exact hinj' _ _ _ _ hci hdi hcd
  have hinfind : ∀ x xi, g.find? x = some xi → ∀ c' s,
      (renameInfo F xi).borrowedBy.find? c' = some s ↔
        (c', s) ∈ xi.borrowedBy.map fun cs => (F cs.1, cs.2) :=
    fun x xi hx c' s => NatMap.find?_ofList (hinnd x xi hx) c' s
  have hone : ∀ x xi, g.find? x = some xi → (g.remap idMap).find? (F x) = some (renameInfo F xi) :=
    fun x xi hx => (hfind _ _).2 (List.mem_map.2 ⟨(x, xi), lookup_mem hx, rfl⟩)
  have hback : ∀ y v, (g.remap idMap).find? y = some v →
      ∃ x xi, g.find? x = some xi ∧ y = F x ∧ v = renameInfo F xi := by
    intro y v hy
    obtain ⟨⟨x, xi⟩, hmem, heq⟩ := List.mem_map.1 ((hfind y v).1 hy)
    simp only [Prod.mk.injEq] at heq
    exact ⟨x, xi, hmemg _ _ hmem, heq.1.symm, heq.2.symm⟩
  -- Borrowers of a renamed reference come from borrowers of the original.
  have hkid : ∀ x xi, g.find? x = some xi → ∀ c' s,
      (renameInfo F xi).borrowedBy.find? c' = some s →
        ∃ c, xi.borrowedBy.find? c = some s ∧ c' = F c := by
    intro x xi hx c' s hs
    obtain ⟨⟨c, s'⟩, hmem, heq⟩ := List.mem_map.1 ((hinfind x xi hx c' s).1 hs)
    simp only [Prod.mk.injEq] at heq
    obtain ⟨rfl, rfl⟩ := heq
    exact ⟨c, NatMap.lookup_of_mem_nodup (sorted_nodup (hw.inner x xi hx)) hmem, rfl⟩
  refine ⟨hone, hback, fun p e c hpc => ?_, ⟨?_, fun y v c' s hy hs => ?_,
    fun y v hy => ?_⟩, fun y v c' s hy hs => ?_⟩
  · obtain ⟨pi, s, ci, hp, hs, he, hc, hb'⟩ := hpc
    refine ⟨renameInfo F pi, s, renameInfo F ci, hone p pi hp, ?_, he, hone c ci hc, ?_⟩
    · exact (hinfind p pi hp (F c) s).2 (List.mem_map.2 ⟨(c, s), lookup_mem hs, rfl⟩)
    · exact mem_foldl_insertNat F _ [] (F p) (Or.inr ⟨p, hb', rfl⟩)
  · rw [remap_eq]; exact NatMap.ofList_sorted _
  · obtain ⟨x, xi, hx, rfl, rfl⟩ := hback y v hy
    obtain ⟨c, hc, rfl⟩ := hkid x xi hx c' s hs
    exact hw.sets x xi c s hx hc
  · obtain ⟨x, xi, hx, rfl, rfl⟩ := hback y v hy
    exact NatMap.ofList_sorted _
  · obtain ⟨x, xi, hx, rfl, rfl⟩ := hback y v hy
    obtain ⟨c, hc, rfl⟩ := hkid x xi hx c' s hs
    obtain ⟨ci, hci, hxc⟩ := hb x xi c s hx hc
    exact ⟨renameInfo F ci, hone c ci hci, mem_foldl_insertNat F _ [] (F x) (Or.inr ⟨x, hxc, rfl⟩)⟩

end Graph

namespace Graph

theorem newRef_mutability {g g' : Graph} {id : Nat} {m : Bool} (h : g.newRef id m = .ok g')
    (x : Nat) (hx : x ≠ id) : g'.mutability x = g.mutability x := by
  simp [mutability, newRef_find h, hx]

theorem newRef_mutability_self {g g' : Graph} {id : Nat} {m : Bool}
    (h : g.newRef id m = .ok g') : g'.mutability id = some m := by
  simp [mutability, newRef_find h]

end Graph

end MoveBytecode.Model
