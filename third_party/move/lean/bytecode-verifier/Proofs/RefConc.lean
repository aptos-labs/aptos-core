-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.BorrowGraph

/-!
# What a borrow graph says about references

UNTRUSTED (checked by the kernel). The concretization of the reference-safety
analysis's borrow graphs: when a concrete frame's references are described by
an abstract state.

A reference id of the abstract state is *held* when a local or an operand
slot of the abstract state holds it; `L` maps each held id to the concrete
reference it stands for. An edge of the graph *is true* of two references
when they have the same root, the edge's path describes where the child is
below the parent at the analysis's level of detail (`absPath`: field steps by
field index; vector elements, which the analysis does not distinguish,
vanish), a strong edge exactly and a weak one up to a prefix, and an
immutable parent has only immutable children. The graph may have false edges
too: the join adds the edges of every incoming path, and a call's result
borrows from every argument it could come from.

What the verifier's checks need is the converse, *completeness*: whenever a
mutable reference has another reference strictly inside it, a chain of true
edges leads from the first to the second (`Chain`), so the first has a
borrower and is not writable. Completeness is stated over the set of held ids
and `L` only, not over where they are held, so instructions that only move
values about keep it.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph

/-! ## Paths -/

/-- The labels the analysis uses for a concrete path: struct and enum fields
by field index; vector elements, a signer's address and table entries vanish. -/
def absPath : List PathStep → List Label
  | [] => []
  | .field i :: p => .field i :: absPath p
  | .variantField _ i :: p => .field i :: absPath p
  | .element _ :: p => absPath p
  | .signerAddress :: p => absPath p
  | .tableEntry _ :: p => absPath p

theorem absPath_append : ∀ (p q : List PathStep), absPath (p ++ q) = absPath p ++ absPath q
  | [], _ => rfl
  | .field _ :: p, q => by simp [absPath, absPath_append p q]
  | .variantField _ _ :: p, q => by simp [absPath, absPath_append p q]
  | .element _ :: p, q => by simp [absPath, absPath_append p q]
  | .signerAddress :: p, q => by simp [absPath, absPath_append p q]
  | .tableEntry _ :: p, q => by simp [absPath, absPath_append p q]

/-- Location `l` is inside location `m`, or is `m`. -/
def _root_.MoveBytecode.Location.Inside (l m : Location) : Prop := l.root = m.root ∧ m.path <+: l.path

/-- Location `l` is strictly inside location `m`. -/
def _root_.MoveBytecode.Location.StrictlyInside (l m : Location) : Prop := l.Inside m ∧ l.path ≠ m.path

theorem _root_.MoveBytecode.Location.Inside.refl (l : Location) : l.Inside l := ⟨rfl, List.prefix_refl _⟩

theorem _root_.MoveBytecode.Location.Inside.trans {a b c : Location} (h1 : a.Inside b) (h2 : b.Inside c) :
    a.Inside c := ⟨h1.1.trans h2.1, h2.2.trans h1.2⟩

theorem _root_.MoveBytecode.Location.Inside.abs {a b : Location} (h : a.Inside b) :
    absPath b.path <+: absPath a.path := by
  obtain ⟨t, ht⟩ := h.2
  rw [← ht, absPath_append]
  exact List.prefix_append _ _

/-! ## True edges -/

/-- Edge `e` is true of a parent reference at `lp` (mutable `mp`) and a child
at `lc` (mutable `mc`). -/
def TrueEdge (lp : Location) (mp : Bool) (e : BorrowGraph.Edge) (lc : Location) (mc : Bool) :
    Prop :=
  lc.root = lp.root ∧
    (if e.strong then absPath lc.path = absPath lp.path ++ e.path
      else absPath lp.path ++ e.path <+: absPath lc.path) ∧
    (mp = false → mc = false)

theorem TrueEdge.prefix {lp lc : Location} {mp mc : Bool} {e : BorrowGraph.Edge}
    (h : TrueEdge lp mp e lc mc) : absPath lp.path ++ e.path <+: absPath lc.path := by
  obtain ⟨-, h, -⟩ := h
  split at h
  · rw [h]; exact List.prefix_refl _
  · exact h

/-- A covering edge is true wherever the edge it covers is. -/
theorem TrueEdge.cover {lp lc : Location} {mp mc : Bool} {e e' : BorrowGraph.Edge}
    (h : TrueEdge lp mp e lc mc) (hle : e'.leq e = true) : TrueEdge lp mp e' lc mc := by
  simp only [Edge.leq, Bool.or_eq_true, beq_iff_eq, Bool.and_eq_true, Bool.not_eq_true'] at hle
  rcases hle with rfl | ⟨hw, hp⟩
  · exact h
  · refine ⟨h.1, ?_, h.2.2⟩
    rw [if_neg (by simp [hw])]
    exact (List.prefix_append_right_inj _).2 (isPrefix_iff.1 hp) |>.trans h.prefix

/-- A spliced edge is true when both edges it splices are. -/
theorem TrueEdge.splice {la lb lc : Location} {ma mb mc : Bool} {pe ce : BorrowGraph.Edge}
    (h1 : TrueEdge la ma pe lb mb) (h2 : TrueEdge lb mb ce lc mc) :
    TrueEdge la ma (Graph.spliceEdge pe ce) lc mc := by
  refine ⟨h2.1.trans h1.1, ?_, fun h => h2.2.2 (h1.2.2 h)⟩
  obtain ⟨-, hp1, -⟩ := h1
  obtain ⟨-, hp2, -⟩ := h2
  simp only [Graph.spliceEdge]
  cases hs1 : pe.strong <;> cases hs2 : ce.strong <;> simp only [hs1, hs2] at hp1 hp2 ⊢ <;>
    simp only [Bool.and_true, Bool.and_false, Bool.false_eq_true, ↓reduceIte]
  · exact hp1.trans ((List.prefix_append _ _).trans hp2)
  · rw [hp2]; exact hp1.trans (List.prefix_append _ _)
  · rw [hp1, List.append_assoc] at hp2; exact hp2
  · rw [hp2, hp1, List.append_assoc]

/-! ## The frame root -/

/-- The struct a struct type instantiates: the type without its type
arguments. The analysis names a global type by its definition, so its
borrows of global storage stand for every instance. -/
def _root_.MoveBytecode.StructIdOf.erase {α : Type} (t : StructIdOf α) : StructIdOf α :=
  { t with typeArgs := [] }

@[simp] theorem StructId.erase_typeArgs (t : StructId) : t.erase.typeArgs = [] := rfl

/-- A struct type without type arguments is its own struct. -/
theorem StructId.erase_of_nil {t : StructId} (h : t.typeArgs = []) : t.erase = t := by
  cases t; simp_all [StructIdOf.erase]

/-- Edge `e` of the frame root is true of a reference at `lc`: it borrows a
local of frame `fid` (`[local i]`), a resource of an instance of a struct of
the frame's module (`[global d]`, the definition `d` naming the struct,
`gsid`), or, weakly with the empty path, either. -/
def TrueRootEdge (fid : FrameId) (gsid : Nat → Option StructId) (e : BorrowGraph.Edge)
    (lc : Location) : Prop :=
  match e.path with
  | .local_ i :: rest => lc.root = .local_ fid i ∧
    (if e.strong then absPath lc.path = rest else rest <+: absPath lc.path)
  | .global d :: rest => (∃ a sid, gsid d = some sid.erase ∧ lc.root = .global a sid) ∧
    (if e.strong then absPath lc.path = rest else rest <+: absPath lc.path)
  | [] => e.strong = false ∧
    ((∃ i, lc.root = .local_ fid i) ∨ ∃ a d sid, gsid d = some sid.erase ∧ lc.root = .global a sid)
  | .field _ :: _ => False

theorem TrueRootEdge.root {fid : FrameId} {gsid : Nat → Option StructId} {e : BorrowGraph.Edge}
    {lc : Location} (h : TrueRootEdge fid gsid e lc) :
    (∃ i, lc.root = .local_ fid i) ∨
      ∃ a d sid, gsid d = some sid.erase ∧ lc.root = .global a sid := by
  unfold TrueRootEdge at h
  split at h
  · exact Or.inl ⟨_, h.1⟩
  · obtain ⟨⟨a, sid, hs, hr⟩, -⟩ := h
    exact Or.inr ⟨a, _, sid, hs, hr⟩
  · exact h.2
  · exact h.elim

theorem TrueRootEdge.weak_of_nil {fid : FrameId} {gsid : Nat → Option StructId}
    {e : BorrowGraph.Edge} {lc : Location} (h : TrueRootEdge fid gsid e lc) :
    e.path = [] → e.strong = false := by
  intro hp
  unfold TrueRootEdge at h
  rw [hp] at h
  exact h.1

/-- A covering edge of the frame root is true wherever the edge it covers is. -/
theorem TrueRootEdge.cover {fid : FrameId} {gsid : Nat → Option StructId}
    {e e' : BorrowGraph.Edge} {lc : Location} (h : TrueRootEdge fid gsid e lc)
    (hle : e'.leq e = true) : TrueRootEdge fid gsid e' lc := by
  simp only [Edge.leq, Bool.or_eq_true, beq_iff_eq, Bool.and_eq_true, Bool.not_eq_true'] at hle
  rcases hle with rfl | ⟨hw, hp⟩
  · exact h
  · have hroot := h.root
    obtain ⟨t, ht⟩ := isPrefix_iff.1 hp
    obtain ⟨s', p'⟩ := e'
    obtain ⟨s, p⟩ := e
    simp only at hw ht
    subst hw ht
    unfold TrueRootEdge at h ⊢
    simp only
    cases p' with
    | nil => exact ⟨by first | rfl | trivial, hroot⟩
    | cons l rest' =>
      cases l with
      | local_ i =>
        simp only [List.cons_append] at h
        refine ⟨h.1, ?_⟩
        simp only [Bool.false_eq_true, ↓reduceIte]
        split at h
        · rw [h.2]; exact List.prefix_append _ _
        · exact (List.prefix_append _ _).trans h.2
      | global d =>
        simp only [List.cons_append] at h
        refine ⟨h.1, ?_⟩
        simp only [Bool.false_eq_true, ↓reduceIte]
        split at h
        · rw [h.2]; exact List.prefix_append _ _
        · exact (List.prefix_append _ _).trans h.2
      | field _ => simp only [List.cons_append] at h

/-- An edge of the frame root spliced with a true edge below it is true. -/
theorem TrueRootEdge.splice {fid : FrameId} {gsid : Nat → Option StructId}
    {pe ce : BorrowGraph.Edge} {lb lc : Location} {mb mc : Bool}
    (h1 : TrueRootEdge fid gsid pe lb) (h2 : TrueEdge lb mb ce lc mc) :
    TrueRootEdge fid gsid (Graph.spliceEdge pe ce) lc := by
  obtain ⟨hroot, hp2, -⟩ := h2
  have hpre : absPath lb.path <+: absPath lc.path := by
    split at hp2
    · rw [hp2]; exact List.prefix_append _ _
    · exact (List.prefix_append _ _).trans hp2
  obtain ⟨ps, pp⟩ := pe
  unfold TrueRootEdge at h1 ⊢
  simp only [Graph.spliceEdge] at h1 ⊢
  cases pp with
  | nil =>
    obtain ⟨hw, hr⟩ := h1
    subst hw
    simp only [Bool.false_and, Bool.false_eq_true, ↓reduceIte]
    rw [hroot]
    exact ⟨by first | rfl | trivial, hr⟩
  | cons l rest =>
    cases l with
    | local_ i =>
      obtain ⟨hr, hp1⟩ := h1
      cases hs : ps
      · simp only [hs, Bool.false_eq_true, ↓reduceIte, Bool.false_and] at hp1 ⊢
        exact ⟨hroot.trans hr, hp1.trans hpre⟩
      · simp only [hs, ↓reduceIte, Bool.true_and, List.cons_append] at hp1 ⊢
        refine ⟨hroot.trans hr, ?_⟩
        split at hp2
        · next hcs => simp only [hcs, ↓reduceIte]; rw [hp2, hp1]
        · next hcs => simp only [hcs, Bool.false_eq_true, ↓reduceIte]; rw [← hp1]; exact hp2
    | global d =>
      obtain ⟨⟨a, sid, hs', hr⟩, hp1⟩ := h1
      cases hs : ps
      · simp only [hs, Bool.false_eq_true, ↓reduceIte, Bool.false_and] at hp1 ⊢
        exact ⟨⟨a, sid, hs', hroot.trans hr⟩, hp1.trans hpre⟩
      · simp only [hs, ↓reduceIte, Bool.true_and, List.cons_append] at hp1 ⊢
        refine ⟨⟨a, sid, hs', hroot.trans hr⟩, ?_⟩
        split at hp2
        · next hcs => simp only [hcs, ↓reduceIte]; rw [hp2, hp1]
        · next hcs => simp only [hcs, Bool.false_eq_true, ↓reduceIte]; rw [← hp1]; exact hp2
    | field _ => exact h1.elim

/-! ## Chains -/

/-- A chain of true edges, from the reference `a` stands for to the one `b`
stands for, through references `L` gives. -/
inductive Chain (g : Graph) (L : Nat → Option Reference) : Nat → Nat → Prop
  | edge {a b : Nat} {e : BorrowGraph.Edge} {ra rb : Reference} :
      L a = some ra → L b = some rb → g.Edge a e b →
      TrueEdge ra.location ra.mutable e rb.location rb.mutable → Chain g L a b
  | snoc {a b c : Nat} {e : BorrowGraph.Edge} {rb rc : Reference} :
      Chain g L a b → L b = some rb → L c = some rc → g.Edge b e c →
      TrueEdge rb.location rb.mutable e rc.location rc.mutable → Chain g L a c

namespace Chain

variable {g g' : Graph} {L : Nat → Option Reference}

theorem src {a b : Nat} (h : Chain g L a b) : ∃ ra, L a = some ra := by
  induction h with
  | edge ha _ _ _ => exact ⟨_, ha⟩
  | snoc _ _ _ _ _ ih => exact ih

theorem trans {a b c : Nat} (h1 : Chain g L a b) (h2 : Chain g L b c) : Chain g L a c := by
  induction h2 with
  | edge hb hc he ht => exact .snoc h1 hb hc he ht
  | snoc _ hb hc he ht ih => exact .snoc ih hb hc he ht

/-- What a chain says: the same root, the abstract path of the start a prefix
of the end's, and immutability passed down. -/
theorem facts {a b : Nat} {ra rb : Reference} (h : Chain g L a b) (ha : L a = some ra)
    (hb : L b = some rb) :
    rb.location.root = ra.location.root ∧ absPath ra.location.path <+: absPath rb.location.path ∧
      (ra.mutable = false → rb.mutable = false) := by
  induction h generalizing rb with
  | edge ha' hb' _ ht =>
    rw [ha, Option.some.injEq] at ha'; subst ha'
    rw [hb, Option.some.injEq] at hb'; subst hb'
    exact ⟨ht.1, (List.prefix_append _ _).trans ht.prefix, ht.2.2⟩
  | snoc _ hb' hc' _ ht ih =>
    rw [hb, Option.some.injEq] at hc'; subst hc'
    obtain ⟨h1, h2, h3⟩ := ih hb'
    exact ⟨ht.1.trans h1, h2.trans ((List.prefix_append _ _).trans ht.prefix),
      fun h => ht.2.2 (h3 h)⟩

/-- Covering keeps chains. -/
theorem mono {a b : Nat} (hc : g.Covers g') (h : Chain g L a b) : Chain g' L a b := by
  induction h with
  | edge ha hb he ht =>
    obtain ⟨e', he', hle⟩ := hc _ _ _ trivial he
    exact .edge ha hb he' (ht.cover hle)
  | snoc _ hb hc' he ht ih =>
    obtain ⟨e', he', hle⟩ := hc _ _ _ trivial he
    exact .snoc ih hb hc' he' (ht.cover hle)

/-- **Releasing** a reference keeps chains between the others, spliced
around it. -/
theorem release {id a b : Nat} (hrel : g.release id = .ok g') (hw : g.WF)
    (h : Chain g L a b) (ha : a ≠ id) :
    (b ≠ id → a = b ∨ Chain g' L a b) ∧
      (b = id → ∃ x rx rid pe, L x = some rx ∧ L id = some rid ∧ x ≠ id ∧
        (x = a ∨ Chain g' L a x) ∧ g.Edge x pe id ∧
        TrueEdge rx.location rx.mutable pe rid.location rid.mutable) := by
  obtain ⟨-, -, -, hkeep, hsplice⟩ := Graph.release_ok hrel hw
  revert ha
  induction h with
  | @edge b e ra rb ha' hb' he ht =>
    intro ha
    refine ⟨fun hb => Or.inr ?_, fun hb => ?_⟩
    · obtain ⟨e', he', hle⟩ := hkeep _ _ _ he ha hb
      exact .edge ha' hb' he' (ht.cover hle)
    · subst hb
      exact ⟨a, ra, rb, e, ha', hb', ha, Or.inl rfl, he, ht⟩
  | @snoc b c e rb rc hab hb' hc' he ht ih =>
    intro ha
    obtain ⟨ih1, ih2⟩ := ih ha
    refine ⟨fun hc => ?_, fun hc => ?_⟩
    · by_cases hbid : b = id
      · -- Through `id`: splice.
        subst hbid
        obtain ⟨x, rx, rid, pe, hx, hid, hxid, hax, hpe, htp⟩ := ih2 rfl
        rw [hb', Option.some.injEq] at hid; subst hid
        by_cases hxc : x = c
        · subst hxc
          rcases hax with rfl | hax
          · exact Or.inl rfl
          · exact Or.inr hax
        · obtain ⟨e', he', hle⟩ := hsplice x pe c e hpe he hxc
          have ht' := (htp.splice ht).cover hle
          rcases hax with rfl | hax
          · exact Or.inr (.edge hx hc' he' ht')
          · exact Or.inr (.snoc hax hx hc' he' ht')
      · obtain ⟨e', he', hle⟩ := hkeep _ _ _ he hbid hc
        rcases ih1 hbid with rfl | hab'
        · exact Or.inr (.edge hb' hc' he' (ht.cover hle))
        · exact Or.inr (.snoc hab' hb' hc' he' (ht.cover hle))
    · by_cases hbid : b = id
      · obtain ⟨x, rx, rid, pe, hx, hid, hxid, hax, hpe, htp⟩ := ih2 hbid
        exact ⟨x, rx, rid, pe, hx, hid, hxid, hax, hpe, htp⟩
      · refine ⟨b, rb, rc, e, hb', hc ▸ hc', hbid, ?_, hc ▸ he, ht⟩
        rcases ih1 hbid with h | hab'
        · exact Or.inl h.symm
        · exact Or.inr hab'

end Chain

/-- A chain from the frame root `root` to the reference `b` stands for, whose
first edge counts at `label?` (`atLabel`). -/
def RootChain (g : Graph) (L : Nat → Option Reference) (root : Nat) (fid : FrameId)
    (gsid : Nat → Option StructId) (label? : Option Label) (b : Nat) : Prop :=
  ∃ e n rn, g.Edge root e n ∧ Graph.atLabel label? e = true ∧ L n = some rn ∧
    TrueRootEdge fid gsid e rn.location ∧ (n = b ∨ Chain g L n b)

namespace Chain

variable {g g' : Graph} {L L' : Nat → Option Reference}

/-- More references described keeps chains. -/
theorem extend {a b : Nat} (hL : ∀ x r, L x = some r → L' x = some r) (h : Chain g L a b) :
    Chain g L' a b := by
  induction h with
  | edge ha hb he ht => exact .edge (hL _ _ ha) (hL _ _ hb) he ht
  | snoc _ hb hc he ht ih => exact .snoc ih (hL _ _ hb) (hL _ _ hc) he ht

/-- The first edge of a chain. -/
theorem first {a b : Nat} (h : Chain g L a b) :
    ∃ e m ra rm, L a = some ra ∧ L m = some rm ∧ g.Edge a e m ∧
      TrueEdge ra.location ra.mutable e rm.location rm.mutable ∧ (m = b ∨ Chain g L m b) := by
  induction h with
  | edge ha hb he ht => exact ⟨_, _, _, _, ha, hb, he, ht, Or.inl rfl⟩
  | snoc _ hb hc he ht ih =>
    obtain ⟨e', m, ra, rm, ha, hm, he', ht', hrest⟩ := ih
    refine ⟨e', m, ra, rm, ha, hm, he', ht', Or.inr ?_⟩
    rcases hrest with rfl | hrest
    · rw [hm, Option.some.injEq] at hb; subst hb
      exact .edge hm hc he ht
    · exact .snoc hrest hb hc he ht

/-- The first edge of a chain that leaves its start. -/
theorem firstOut {a b : Nat} (h : Chain g L a b) (hab : a ≠ b) :
    ∃ e m ra rm, L a = some ra ∧ L m = some rm ∧ g.Edge a e m ∧ m ≠ a ∧
      TrueEdge ra.location ra.mutable e rm.location rm.mutable ∧ (m = b ∨ Chain g L m b) := by
  induction h with
  | edge ha hb he ht => exact ⟨_, _, _, _, ha, hb, he, Ne.symm hab, ht, Or.inl rfl⟩
  | @snoc b c e rb rc hab' hb hc he ht ih =>
    by_cases hba : b = a
    · subst hba
      exact ⟨e, c, rb, rc, hb, hc, he, Ne.symm hab, ht, Or.inl rfl⟩
    · obtain ⟨e', m, ra, rm, ha, hm, he', hma, ht', hrest⟩ := ih (Ne.symm hba)
      refine ⟨e', m, ra, rm, ha, hm, he', hma, ht', Or.inr ?_⟩
      rcases hrest with rfl | hrest
      · rw [hm, Option.some.injEq] at hb; subst hb
        exact .edge hm hc he ht
      · exact .snoc hrest hb hc he ht

/-- Every reference on a chain is described. -/
theorem dst {a b : Nat} (h : Chain g L a b) : ∃ rb, L b = some rb := by
  cases h with
  | edge _ hb _ _ => exact ⟨_, hb⟩
  | snoc _ _ hc _ _ => exact ⟨_, hc⟩

/-- Mutability only decreases along a chain: a chain that ends at a mutable
reference starts at one. -/
theorem mutable_src {a b : Nat} (h : Chain g L a b) {ra rb : Reference} (ha : L a = some ra)
    (hb : L b = some rb) (hm : rb.mutable = true) : ra.mutable = true := by
  induction h generalizing rb with
  | edge ha' hb' _ ht =>
    rw [ha, Option.some.injEq] at ha'; subst ha'
    rw [hb, Option.some.injEq] at hb'; subst hb'
    obtain ⟨-, -, hmono⟩ := ht
    cases hra : ra.mutable
    · rw [hmono hra] at hm; cases hm
    · rfl
  | @snoc _ _ _ rb' _ _ hb' hc' _ ht ih =>
    rw [hb, Option.some.injEq] at hc'; subst hc'
    obtain ⟨-, -, hmono⟩ := ht
    refine ih hb' ?_
    cases hrb : rb'.mutable
    · rw [hmono hrb] at hm; cases hm
    · rfl

end Chain

namespace RootChain

variable {g g' : Graph} {L L' : Nat → Option Reference} {root : Nat} {fid : FrameId}
  {gsid : Nat → Option StructId} {label? : Option Label}

theorem atLabel_iff {e : BorrowGraph.Edge} :
    Graph.atLabel label? e = true ↔
      e.path = [] ∨ label? = none ∨ ∃ l rest, e.path = l :: rest ∧ label? = some l := by
  unfold Graph.atLabel
  cases e.path with
  | nil => simp
  | cons l rest =>
    cases label? with
    | none => simp
    | some l' =>
      simp only [beq_iff_eq]
      constructor
      · intro h; exact Or.inr (Or.inr ⟨l, rest, rfl, by rw [h]⟩)
      · rintro (h | h | ⟨l₁, rest₁, h1, h2⟩)
        · cases h
        · cases h
        · simp only [List.cons.injEq, Option.some.injEq] at h1 h2
          rw [h1.1, h2]

theorem atLabel_cover {e e' : BorrowGraph.Edge} (h : Graph.atLabel label? e = true)
    (hle : e'.leq e = true) : Graph.atLabel label? e' = true := by
  simp only [Edge.leq, Bool.or_eq_true, beq_iff_eq, Bool.and_eq_true, Bool.not_eq_true'] at hle
  rcases hle with rfl | ⟨-, hp⟩
  · exact h
  · obtain ⟨t, ht⟩ := isPrefix_iff.1 hp
    rw [atLabel_iff] at h ⊢
    cases hep : e'.path with
    | nil => exact Or.inl rfl
    | cons l rest =>
      rw [hep] at ht
      rcases h with h | h | ⟨l', rest', h1, h2⟩
      · rw [← ht] at h; simp at h
      · exact Or.inr (Or.inl h)
      · rw [← ht, List.cons_append, List.cons.injEq] at h1
        exact Or.inr (Or.inr ⟨l, rest, rfl, h1.1 ▸ h2⟩)

theorem atLabel_splice {pe ce : BorrowGraph.Edge} (h : Graph.atLabel label? pe = true)
    (hw : pe.path = [] → pe.strong = false) :
    Graph.atLabel label? (Graph.spliceEdge pe ce) = true := by
  rw [atLabel_iff] at h ⊢
  simp only [Graph.spliceEdge]
  split
  · next hs =>
    rcases h with h | h | ⟨l, rest, h1, h2⟩
    · rw [hw h] at hs; cases hs
    · exact Or.inr (Or.inl h)
    · exact Or.inr (Or.inr ⟨l, rest ++ ce.path, by rw [h1]; rfl, h2⟩)
  · exact h

theorem extend {b : Nat} (hL : ∀ x r, L x = some r → L' x = some r)
    (h : RootChain g L root fid gsid label? b) : RootChain g L' root fid gsid label? b := by
  obtain ⟨e, n, rn, he, hl, hn, ht, hrest⟩ := h
  exact ⟨e, n, rn, he, hl, hL _ _ hn, ht, hrest.imp id (Chain.extend hL)⟩

theorem mono {b : Nat} (hc : g.Covers g') (h : RootChain g L root fid gsid label? b) :
    RootChain g' L root fid gsid label? b := by
  obtain ⟨e, n, rn, he, hl, hn, ht, hrest⟩ := h
  obtain ⟨e', he', hle⟩ := hc _ _ _ trivial he
  exact ⟨e', n, rn, he', atLabel_cover hl hle, hn, ht.cover hle, hrest.imp id (Chain.mono hc)⟩

/-- Releasing a reference keeps root chains to the others. -/
theorem release {id b : Nat} (hrel : g.release id = .ok g') (hw : g.WF) (hroot : root ≠ id)
    (hroot' : L root = none) (h : RootChain g L root fid gsid label? b) (hb : b ≠ id) :
    RootChain g' L root fid gsid label? b := by
  obtain ⟨-, -, -, hkeep, hsplice⟩ := Graph.release_ok hrel hw
  obtain ⟨e, n, rn, he, hl, hn, ht, hrest⟩ := h
  by_cases hnid : n = id
  · -- The first reference is released: splice past it.
    subst hnid
    have hch : Chain g L n b := by
      rcases hrest with rfl | hrest
      · exact absurd rfl hb
      · exact hrest
    obtain ⟨ce, m, rn', rm, hn', hm, hce, hmid, htc, hmb⟩ := hch.firstOut (Ne.symm hb)
    rw [hn, Option.some.injEq] at hn'; subst hn'
    have hrm : root ≠ m := fun e => by subst e; rw [hroot'] at hm; cases hm
    obtain ⟨e', he', hle⟩ := hsplice root e m ce he hce hrm
    refine ⟨e', m, rm, he', atLabel_cover (atLabel_splice hl ht.weak_of_nil) hle, hm,
      (ht.splice htc).cover hle, ?_⟩
    rcases hmb with rfl | hmb
    · exact Or.inl rfl
    · exact (Chain.release hrel hw hmb hmid).1 hb
  · obtain ⟨e', he', hle⟩ := hkeep _ _ _ he hroot hnid
    refine ⟨e', n, rn, he', atLabel_cover hl hle, hn, ht.cover hle, ?_⟩
    rcases hrest with rfl | hrest
    · exact Or.inl rfl
    · exact (Chain.release hrel hw hrest hnid).1 hb

end RootChain

/-! ## Completeness -/

/-- **Completeness** of graph `g` for the references held (`H`) and what they
stand for (`L`), in frame `fid`, with frame root `root`, whose module's
definitions name the struct types `gsid`, and whose reference parameters are
`ps`:

- `strict`: a mutable reference with another strictly inside it has a chain
  to it, so it is borrowed;
- `same`: of two references to one location, one of them mutable, one has a
  chain to the other;
- `immInside`: nothing strictly inside an immutable reference is mutable;
- `rootLocal`: a reference into a local of the frame has a chain from the
  frame root through an edge at that local;
- `prov`: every reference is into a local of the frame, into a resource the
  frame borrowed (a chain from the root through an edge at its type), or
  inside a reference parameter, mutable only if the parameter is. -/
structure Complete (g : Graph) (L : Nat → Option Reference) (H : Nat → Prop) (root : Nat)
    (fid : FrameId) (gsid : Nat → Option StructId) (ps : List Reference) : Prop where
  wf : g.WF
  back : g.Back
  rootNone : L root = none
  rootMut : g.mutability root = some true
  rootNotHeld : ¬ H root
  keys : ∀ x, (g.find? x).isSome → x = root ∨ H x
  held : ∀ x, H x → ∃ r, L x = some r ∧ g.mutability x = some r.mutable
  strict : ∀ p rp x rx, H p → H x → L p = some rp → L x = some rx → rp.mutable = true →
    rx.location.StrictlyInside rp.location → Chain g L p x
  same : ∀ p rp x rx, H p → H x → p ≠ x → L p = some rp → L x = some rx → rp.mutable = true →
    rx.location = rp.location → Chain g L p x ∨ Chain g L x p
  immInside : ∀ p rp x rx, H p → H x → L p = some rp → L x = some rx → rp.mutable = false →
    rx.location.StrictlyInside rp.location → rx.mutable = false
  rootLocal : ∀ x rx i, H x → L x = some rx → rx.location.root = .local_ fid i →
    RootChain g L root fid gsid (some (.local_ i)) x
  prov : ∀ x rx, H x → L x = some rx →
    (∃ i, rx.location.root = .local_ fid i) ∨
      (∃ d, RootChain g L root fid gsid (some (.global d)) x) ∨
      ∃ π ∈ ps, rx.location.Inside π.location ∧ (rx.mutable = true → π.mutable = true)

theorem Graph.isSome_of_mutability {g g' : Graph} {x : Nat}
    (h : g'.mutability x = g.mutability x) : (g'.find? x).isSome = (g.find? x).isSome := by
  simp only [Graph.mutability] at h
  cases h1 : g'.find? x <;> cases h2 : g.find? x <;> simp_all

theorem StrictlyInside.ne {l : Location} : ¬ l.StrictlyInside l := fun h => h.2 rfl

theorem prefix_antisymm {α : Type} {a b : List α} (h1 : a <+: b) (h2 : b <+: a) : a = b :=
  h1.sublist.eq_of_length_le h2.length_le

/-- Strictly inside something inside `c` is strictly inside `c`. -/
theorem StrictlyInside.trans' {a b c : Location} (h1 : a.StrictlyInside b) (h2 : b.Inside c) :
    a.StrictlyInside c := by
  refine ⟨h1.1.trans h2, fun he => h1.2 ?_⟩
  have hab : b.path <+: a.path := h1.1.2
  have hba : a.path <+: b.path := by rw [he]; exact h2.2
  exact prefix_antisymm hba hab

namespace Complete

variable {g g' : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
  {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}

/-- **Consuming** a held reference and releasing it keeps completeness. -/
theorem release {id : Nat} (hc : Complete g L H root fid gsid ps) (hid : H id)
    (hrel : g.release id = .ok g') :
    Complete g' L (fun x => H x ∧ x ≠ id) root fid gsid ps := by
  obtain ⟨hw', hgone, hmut, -, -⟩ := Graph.release_ok hrel hc.wf
  have hrid : root ≠ id := fun e => hc.rootNotHeld (e ▸ hid)
  refine {
    wf := hw'
    back := Graph.release_back hrel hc.back
    rootNone := hc.rootNone
    rootMut := (hmut root hrid).trans hc.rootMut
    rootNotHeld := fun h => hc.rootNotHeld h.1
    keys := fun x hx => ?_
    held := fun x hx => ?_
    strict := fun p rp x rx hp hx hpL hxL hm hin => ?_
    same := fun p rp x rx hp hx hne hpL hxL hm heq => ?_
    immInside := fun p rp x rx hp hx => hc.immInside p rp x rx hp.1 hx.1
    rootLocal := fun x rx i hx hxL hr =>
      RootChain.release hrel hc.wf hrid hc.rootNone (hc.rootLocal x rx i hx.1 hxL hr) hx.2
    prov := fun x rx hx hxL => ?_ }
  · have hxid : x ≠ id := fun e => by subst e; rw [hgone] at hx; cases hx
    rw [Graph.isSome_of_mutability (hmut x hxid)] at hx
    rcases hc.keys x hx with h | h
    · exact Or.inl h
    · exact Or.inr ⟨h, hxid⟩
  · obtain ⟨r, hr, hm⟩ := hc.held x hx.1
    exact ⟨r, hr, (hmut x hx.2).trans hm⟩
  · rcases (Chain.release hrel hc.wf (hc.strict p rp x rx hp.1 hx.1 hpL hxL hm hin) hp.2).1 hx.2
      with rfl | h
    · rw [hpL, Option.some.injEq] at hxL; subst hxL; exact absurd hin StrictlyInside.ne
    · exact h
  · rcases hc.same p rp x rx hp.1 hx.1 hne hpL hxL hm heq with h | h
    · rcases (Chain.release hrel hc.wf h hp.2).1 hx.2 with h | h
      · exact absurd h hne
      · exact Or.inl h
    · rcases (Chain.release hrel hc.wf h hx.2).1 hp.2 with h | h
      · exact absurd h.symm hne
      · exact Or.inr h
  · rcases hc.prov x rx hx.1 hxL with h | ⟨d, h⟩ | h
    · exact Or.inl h
    · exact Or.inr (Or.inl ⟨d, RootChain.release hrel hc.wf hrid hc.rootNone h hx.2⟩)
    · exact Or.inr (Or.inr h)

end Complete

namespace Chain

variable {g g' : Graph} {L L' : Nat → Option Reference}

/-- Chains survive a change that turns every true edge into a chain. -/
theorem of_edges
    (hedge : ∀ x e y rx ry, g.Edge x e y → L x = some rx → L y = some ry →
      TrueEdge rx.location rx.mutable e ry.location ry.mutable → Chain g' L' x y)
    {a b : Nat} (h : Chain g L a b) : Chain g' L' a b := by
  induction h with
  | edge ha hb he ht => exact hedge _ _ _ _ _ he ha hb ht
  | snoc _ hb hc he ht ih => exact ih.trans (hedge _ _ _ _ _ he hb hc ht)

/-- Under `factor parent path inter`, with `inter` new, standing for `ri`
true below `parent` along `path`, and every moved edge true from `ri`: an
edge is kept, or it becomes a chain through `inter`. -/
theorem factor {parent inter : Nat} {path : List Label} {ri : Reference}
    (hf : g.factor parent path inter = .ok g') (hw : g.WF)
    (hL : ∀ x r, L x = some r → L' x = some r) (hi : L' inter = some ri)
    (hpi : ∀ rp, L parent = some rp → g'.HasEdge parent ⟨true, path⟩ inter →
      TrueEdge rp.location rp.mutable ⟨true, path⟩ ri.location ri.mutable)
    (hmove : ∀ e y rp ry, g.Edge parent e y → isPrefix path e.path = true → L parent = some rp →
      L y = some ry → TrueEdge rp.location rp.mutable e ry.location ry.mutable →
      TrueEdge ri.location ri.mutable ⟨e.strong, e.path.drop path.length⟩ ry.location ry.mutable)
    {a b : Nat} (h : Chain g L a b) : Chain g' L' a b := by
  obtain ⟨-, -, hkeep, hmoved, hnew⟩ := Graph.factor_ok hf hw
  refine of_edges (fun x e y rx ry he hx hy ht => ?_) h
  cases hpre : isPrefix path e.path with
  | false =>
    obtain ⟨e', he', hle⟩ := hkeep x e y he (Or.inr hpre)
    exact .edge (hL _ _ hx) (hL _ _ hy) he' (ht.cover hle)
  | true =>
    by_cases hxp : x = parent
    · subst hxp
      obtain ⟨e₁, he₁, hle₁⟩ := hnew
      obtain ⟨e₂, he₂, hle₂⟩ := hmoved e y he hpre
      have h1 := (hpi rx hx ⟨e₁, he₁, hle₁⟩).cover hle₁
      have h2 := (hmove e y rx ry he hpre hx hy ht).cover hle₂
      exact .snoc (.edge (hL _ _ hx) hi he₁ h1) hi (hL _ _ hy) he₂ h2
    · obtain ⟨e', he', hle⟩ := hkeep x e y he (Or.inl hxp)
      exact .edge (hL _ _ hx) (hL _ _ hy) he' (ht.cover hle)

end Chain

/-- `L` with reference `x` standing for `r`. -/
def updL (L : Nat → Option Reference) (x : Nat) (r : Reference) : Nat → Option Reference :=
  fun y => if y = x then some r else L y

theorem updL_self (L : Nat → Option Reference) (x : Nat) (r : Reference) : updL L x r x = some r := by
  simp [updL]

theorem updL_ne {L : Nat → Option Reference} {x y : Nat} (r : Reference) (h : y ≠ x) :
    updL L x r y = L y := by
  simp [updL, h]

theorem Graph.Edge.keys {g : Graph} {a b : Nat} {e : BorrowGraph.Edge} (h : g.Edge a e b) :
    (g.find? a).isSome ∧ (g.find? b).isSome := by
  obtain ⟨_, _, _, hp, _, _, hc, _⟩ := h
  simp [hp, hc]

namespace Chain

variable {g g' : Graph} {L L' : Nat → Option Reference}

theorem src_key {a b : Nat} (h : Chain g L a b) : (g.find? a).isSome := by
  obtain ⟨e, m, ra, rm, -, -, ⟨pi, -, -, hp, -⟩, -⟩ := h.first
  simp [hp]

theorem nodes_keys {a b : Nat} (h : Chain g L a b) : (g.find? a).isSome ∧ (g.find? b).isSome := by
  refine ⟨h.src_key, ?_⟩
  cases h with
  | edge _ _ he _ => obtain ⟨_, _, ci, _, _, _, hc, _⟩ := he; simp [hc]
  | snoc _ _ _ he _ => obtain ⟨_, _, ci, _, _, _, hc, _⟩ := he; simp [hc]

/-- Describing references outside the graph differently keeps chains. -/
theorem extend' (hL : ∀ x r, (g.find? x).isSome → L x = some r → L' x = some r) {a b : Nat}
    (h : Chain g L a b) : Chain g L' a b := by
  induction h with
  | edge ha hb he ht =>
    exact .edge (hL _ _ (Graph.Edge.keys he).1 ha) (hL _ _ (Graph.Edge.keys he).2 hb) he ht
  | snoc _ hb hc he ht ih =>
    exact .snoc ih (hL _ _ (Graph.Edge.keys he).1 hb) (hL _ _ (Graph.Edge.keys he).2 hc) he ht

/-- Under `factor parent path inter`, a chain from `parent` whose first edge
moves continues from `inter`. -/
theorem factor_from {parent inter : Nat} {path : List Label} {ri : Reference}
    (hf : g.factor parent path inter = .ok g') (hw : g.WF)
    (hL : ∀ x r, L x = some r → L' x = some r) (hi : L' inter = some ri)
    (hpi : ∀ rp, L parent = some rp → g'.HasEdge parent ⟨true, path⟩ inter →
      TrueEdge rp.location rp.mutable ⟨true, path⟩ ri.location ri.mutable)
    (hmove : ∀ e y rp ry, g.Edge parent e y → isPrefix path e.path = true → L parent = some rp →
      L y = some ry → TrueEdge rp.location rp.mutable e ry.location ry.mutable →
      TrueEdge ri.location ri.mutable ⟨e.strong, e.path.drop path.length⟩ ry.location ry.mutable)
    {b : Nat} (h : Chain g L parent b) (hpre : ∀ e y, g.Edge parent e y → isPrefix path e.path = true) :
    Chain g' L' inter b := by
  obtain ⟨-, -, -, hmoved, -⟩ := Graph.factor_ok hf hw
  obtain ⟨e, m, rp, rm, hp, hm, he, ht, hrest⟩ := h.first
  obtain ⟨e', he', hle⟩ := hmoved e m he (hpre e m he)
  have h1 := (hmove e m rp rm he (hpre e m he) hp hm ht).cover hle
  have hfirst : Chain g' L' inter m := .edge hi (hL _ _ hm) he' h1
  rcases hrest with rfl | hrest
  · exact hfirst
  · exact hfirst.trans (Chain.factor hf hw hL hi hpi hmove hrest)

end Chain

namespace RootChain

variable {g g' : Graph} {L L' : Nat → Option Reference} {root : Nat} {fid : FrameId}
  {gsid : Nat → Option StructId} {label? : Option Label}

theorem snoc {b c : Nat} {e : BorrowGraph.Edge} {rb rc : Reference}
    (h : RootChain g L root fid gsid label? b) (hb : L b = some rb) (hc : L c = some rc)
    (he : g.Edge b e c) (ht : TrueEdge rb.location rb.mutable e rc.location rc.mutable) :
    RootChain g L root fid gsid label? c := by
  obtain ⟨e₀, n, rn, he₀, hl, hn, ht₀, hrest⟩ := h
  refine ⟨e₀, n, rn, he₀, hl, hn, ht₀, Or.inr ?_⟩
  rcases hrest with rfl | hrest
  · rw [hn, Option.some.injEq] at hb; subst hb
    exact .edge hn hc he ht
  · exact .snoc hrest hb hc he ht

theorem extend' (hL : ∀ x r, (g.find? x).isSome → L x = some r → L' x = some r) {b : Nat}
    (h : RootChain g L root fid gsid label? b) : RootChain g L' root fid gsid label? b := by
  obtain ⟨e, n, rn, he, hl, hn, ht, hrest⟩ := h
  have hnk : (g.find? n).isSome := by obtain ⟨_, _, ci, _, _, _, hc, _⟩ := he; simp [hc]
  exact ⟨e, n, rn, he, hl, hL _ _ hnk hn, ht, hrest.imp id (Chain.extend' hL)⟩

/-- Under `factor parent path inter` of a reference `parent` (not the root),
root chains are kept. -/
theorem factor {parent inter : Nat} {path : List Label} {ri : Reference}
    (hf : g.factor parent path inter = .ok g') (hw : g.WF) (hpr : root ≠ parent)
    (hL : ∀ x r, L x = some r → L' x = some r) (hi : L' inter = some ri)
    (hpi : ∀ rp, L parent = some rp → g'.HasEdge parent ⟨true, path⟩ inter →
      TrueEdge rp.location rp.mutable ⟨true, path⟩ ri.location ri.mutable)
    (hmove : ∀ e y rp ry, g.Edge parent e y → isPrefix path e.path = true → L parent = some rp →
      L y = some ry → TrueEdge rp.location rp.mutable e ry.location ry.mutable →
      TrueEdge ri.location ri.mutable ⟨e.strong, e.path.drop path.length⟩ ry.location ry.mutable)
    {b : Nat} (h : RootChain g L root fid gsid label? b) : RootChain g' L' root fid gsid label? b := by
  obtain ⟨-, -, hkeep, -, -⟩ := Graph.factor_ok hf hw
  obtain ⟨e, n, rn, he, hl, hn, ht, hrest⟩ := h
  obtain ⟨e', he', hle⟩ := hkeep root e n he (Or.inl hpr)
  exact ⟨e', n, rn, he', atLabel_cover hl hle, hL _ _ hn, ht.cover hle,
    hrest.imp id (Chain.factor hf hw hL hi hpi hmove)⟩

end RootChain

namespace Complete

variable {g g1 g2 : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
  {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}

theorem not_held_of_fresh (hc : Complete g L H root fid gsid ps) {x : Nat}
    (hx : g.find? x = none) : ¬ H x := by
  intro h
  obtain ⟨r, -, hm⟩ := hc.held x h
  simp [Graph.mutability, hx] at hm

theorem root_key (hc : Complete g L H root fid gsid ps) : (g.find? root).isSome := by
  have := hc.rootMut
  simp only [Graph.mutability] at this
  cases h : g.find? root <;> simp_all

/-- **Copying** held reference `p` (`CopyLoc` of a local holding it): a new
reference `c` standing for the same reference, borrowing everything `p`
borrowed, and borrowed by `p`. -/
theorem copy {p c : Nat} {rp : Reference} (hc : Complete g L H root fid gsid ps) (hp : H p)
    (hpL : L p = some rp) (hnew : g.newRef c rp.mutable = .ok g1)
    (hfac : g1.factor p [] c = .ok g2) :
    Complete g2 (updL L c rp) (fun x => H x ∨ x = c) root fid gsid ps := by
  have hcfresh := (Graph.newRef_ok hnew).1
  have hcH : ¬ H c := hc.not_held_of_fresh hcfresh
  have hcp : c ≠ p := fun e => hcH (e ▸ hp)
  have hcr : c ≠ root := fun e => by
    have := hc.root_key; rw [← e, hcfresh] at this; cases this
  have hrp : root ≠ p := fun e => hc.rootNotHeld (e ▸ hp)
  have hw1 := Graph.newRef_wf hnew hc.wf
  have hb1 := Graph.newRef_back hnew hc.back
  obtain ⟨hw2, hmut2, -, -, hnewE⟩ := Graph.factor_ok hfac hw1
  have hLc : updL L c rp c = some rp := updL_self _ _ _
  have hLx : ∀ x, x ≠ c → updL L c rp x = L x := fun x hx => updL_ne rp hx
  generalize updL L c rp = L' at hLc hLx ⊢
  have hext : ∀ x r, (g.find? x).isSome → L x = some r → L' x = some r := by
    intro x r hx h
    have : x ≠ c := fun e => by subst e; rw [hcfresh] at hx; cases hx
    rw [hLx x this]; exact h
  have hext1 : ∀ x r, L' x = some r → L' x = some r := fun _ _ h => h
  have hpi : ∀ r, L' p = some r → g2.HasEdge p ⟨true, []⟩ c →
      TrueEdge r.location r.mutable ⟨true, []⟩ rp.location rp.mutable := by
    intro r hr _
    rw [hLx p (Ne.symm hcp), hpL, Option.some.injEq] at hr
    subst hr
    exact ⟨rfl, by simp, id⟩
  have hmove : ∀ e y r ry, g1.Edge p e y → isPrefix [] e.path = true → L' p = some r →
      L' y = some ry → TrueEdge r.location r.mutable e ry.location ry.mutable →
      TrueEdge rp.location rp.mutable ⟨e.strong, e.path.drop ([] : List Label).length⟩
        ry.location ry.mutable := by
    intro e y r ry _ _ hr _ ht
    rw [hLx p (Ne.symm hcp), hpL, Option.some.injEq] at hr
    subst hr
    simpa using ht
  -- Old chains, through the new graph.
  have hmapC : ∀ a b, Chain g L a b → Chain g2 L' a b := fun a b h =>
    Chain.factor hfac hw1 hext1 hLc hpi hmove
      (Chain.mono (Graph.newRef_covers hnew) (h.extend' hext))
  have hmapR : ∀ l b, RootChain g L root fid gsid l b → RootChain g2 L' root fid gsid l b :=
    fun l b h => RootChain.factor hfac hw1 hrp hext1 hLc hpi hmove
      (RootChain.mono (Graph.newRef_covers hnew) (h.extend' hext))
  have hpc : Chain g2 L' p c := by
    obtain ⟨e', he', hle⟩ := hnewE
    exact .edge (by rw [hLx p (Ne.symm hcp)]; exact hpL) hLc he'
      ((hpi rp (by rw [hLx p (Ne.symm hcp)]; exact hpL) ⟨e', he', hle⟩).cover hle)
  have hfrom : ∀ b, Chain g L p b → Chain g2 L' c b := fun b h =>
    Chain.factor_from hfac hw1 hext1 hLc hpi hmove
      (Chain.mono (Graph.newRef_covers hnew) (h.extend' hext)) (fun _ _ _ => rfl)
  -- The held references and what they stand for.
  have hHL : ∀ x r, H x → L' x = some r → L x = some r := fun x r hx h => by
    rwa [hLx x (fun e => hcH (e ▸ hx))] at h
  refine {
    wf := hw2
    back := Graph.factor_back hfac hw1 hb1
    rootNone := by rw [hLx root (Ne.symm hcr)]; exact hc.rootNone
    rootMut := by rw [hmut2, Graph.newRef_mutability hnew root (Ne.symm hcr)]; exact hc.rootMut
    rootNotHeld := fun h => h.elim hc.rootNotHeld (fun e => hcr e.symm)
    keys := fun x hx => ?_
    held := fun x hx => ?_
    strict := fun q rq x rx hq hx hqL hxL hm hin => ?_
    same := fun q rq x rx hq hx hne hqL hxL hm heq => ?_
    immInside := fun q rq x rx hq hx hqL hxL hm hin => ?_
    rootLocal := fun x rx i hx hxL hr => ?_
    prov := fun x rx hx hxL => ?_ }
  · rw [Graph.isSome_of_mutability (hmut2 x)] at hx
    by_cases hxc : x = c
    · exact Or.inr (Or.inr hxc)
    · rw [Graph.isSome_of_mutability (Graph.newRef_mutability hnew x hxc)] at hx
      rcases hc.keys x hx with h | h
      · exact Or.inl h
      · exact Or.inr (Or.inl h)
  · rcases hx with hx | rfl
    · obtain ⟨r, hr, hm⟩ := hc.held x hx
      have hxc : x ≠ c := fun e => hcH (e ▸ hx)
      exact ⟨r, by rw [hLx x hxc]; exact hr, by
        rw [hmut2, Graph.newRef_mutability hnew x hxc]; exact hm⟩
    · exact ⟨rp, hLc, by rw [hmut2, Graph.newRef_mutability_self hnew]⟩
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hmapC q x (hc.strict q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin)
    · -- Inside a held reference: so is `p`, which `c` copies.
      rw [hLc, Option.some.injEq] at hxL; subst hxL
      exact (hmapC q p (hc.strict q rq p rp hq hp (hHL q rq hq hqL) hpL hm hin)).trans hpc
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      exact hfrom x (hc.strict p rp x rx hp hx hpL (hHL x rx hx hxL) hm hin)
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin StrictlyInside.ne
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · rcases hc.same q rq x rx hq hx hne (hHL q rq hq hqL) (hHL x rx hx hxL) hm heq with h | h
      · exact Or.inl (hmapC q x h)
      · exact Or.inr (hmapC x q h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      by_cases hqp : q = p
      · subst hqp; exact Or.inl hpc
      · rcases hc.same q rq p rp hq hp hqp (hHL q rq hq hqL) hpL hm heq with h | h
        · exact Or.inl ((hmapC q p h).trans hpc)
        · exact Or.inr (hfrom q h)
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      by_cases hxp : x = p
      · subst hxp; exact Or.inr hpc
      · rcases hc.same p rp x rx hp hx (Ne.symm hxp) hpL (hHL x rx hx hxL) hm heq with h | h
        · exact Or.inl (hfrom x h)
        · exact Or.inr ((hmapC x p h).trans hpc)
    · exact absurd rfl hne
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hc.immInside q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      exact hc.immInside q rq p rp hq hp (hHL q rq hq hqL) hpL hm hin
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      exact hc.immInside p rp x rx hp hx hpL (hHL x rx hx hxL) hm hin
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin StrictlyInside.ne
  · rcases hx with hx | rfl
    · exact hmapR _ x (hc.rootLocal x rx i hx (hHL x rx hx hxL) hr)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      obtain ⟨e', he', hle⟩ := hnewE
      exact (hmapR _ p (hc.rootLocal p rp i hp hpL hr)).snoc
        (by rw [hLx p (Ne.symm hcp)]; exact hpL) hLc he'
        ((hpi rp (by rw [hLx p (Ne.symm hcp)]; exact hpL) ⟨e', he', hle⟩).cover hle)
  · rcases hx with hx | rfl
    · rcases hc.prov x rx hx (hHL x rx hx hxL) with h | ⟨d, h⟩ | h
      · exact Or.inl h
      · exact Or.inr (Or.inl ⟨d, hmapR _ x h⟩)
      · exact Or.inr (Or.inr h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      rcases hc.prov p rp hp hpL with h | ⟨d, h⟩ | h
      · exact Or.inl h
      · obtain ⟨e', he', hle⟩ := hnewE
        exact Or.inr (Or.inl ⟨d, (hmapR _ p h).snoc
          (by rw [hLx p (Ne.symm hcp)]; exact hpL) hLc he'
          ((hpi rp (by rw [hLx p (Ne.symm hcp)]; exact hpL) ⟨e', he', hle⟩).cover hle)⟩)
      · exact Or.inr (Or.inr h)

end Complete

namespace RootChain

variable {g g' : Graph} {L L' : Nat → Option Reference} {root : Nat} {fid : FrameId}
  {gsid : Nat → Option StructId} {label? : Option Label}

theorem atLabel_head {lab : Label} {e : BorrowGraph.Edge} (h : Graph.atLabel label? e = true)
    (hp : isPrefix [lab] e.path = true) : Graph.atLabel label? ⟨true, [lab]⟩ = true := by
  rw [atLabel_iff] at h ⊢
  obtain ⟨t, ht⟩ := isPrefix_iff.1 hp
  rcases h with h | h | ⟨l, rest, h1, h2⟩
  · rw [← ht] at h; simp at h
  · exact Or.inr (Or.inl h)
  · rw [← ht, List.singleton_append, List.cons.injEq] at h1
    exact Or.inr (Or.inr ⟨lab, [], rfl, h1.1 ▸ h2⟩)

/-- Under `factor root [lab] c`: root chains are kept, the moved ones now
through `c`; and a root chain whose first edge moved gives a chain from `c`. -/
theorem factorRoot {lab : Label} {c : Nat} {rc : Reference}
    (hf : g.factor root [lab] c = .ok g') (hw : g.WF)
    (hL : ∀ x r, L x = some r → L' x = some r) (hrootL : L' root = none) (hcL : L' c = some rc)
    (hpi : TrueRootEdge fid gsid ⟨true, [lab]⟩ rc.location)
    (hmove : ∀ e y ry, g.Edge root e y → isPrefix [lab] e.path = true → L y = some ry →
      TrueRootEdge fid gsid e ry.location →
      TrueEdge rc.location rc.mutable ⟨e.strong, e.path.drop 1⟩ ry.location ry.mutable)
    {b : Nat} (h : RootChain g L root fid gsid label? b) :
    RootChain g' L' root fid gsid label? b ∧
      ∀ e n rn, g.Edge root e n → L n = some rn → TrueRootEdge fid gsid e rn.location →
        isPrefix [lab] e.path = true → (n = b ∨ Chain g L n b) → Chain g' L' c b := by
  obtain ⟨-, -, hkeep, hmoved, hnew⟩ := Graph.factor_ok hf hw
  have hrootL0 : L root = none := by
    cases h0 : L root with
    | none => rfl
    | some r0 => have := hL _ _ h0; rw [hrootL] at this; cases this
  have hpi' : ∀ rp, L root = some rp → g'.HasEdge root ⟨true, [lab]⟩ c →
      TrueEdge rp.location rp.mutable ⟨true, [lab]⟩ rc.location rc.mutable := by
    intro rp h; rw [hrootL0] at h; cases h
  have hmove' : ∀ e y rp ry, g.Edge root e y → isPrefix [lab] e.path = true → L root = some rp →
      L y = some ry → TrueEdge rp.location rp.mutable e ry.location ry.mutable →
      TrueEdge rc.location rc.mutable ⟨e.strong, e.path.drop [lab].length⟩ ry.location ry.mutable := by
    intro e y rp _ _ _ h
    rw [hrootL0] at h; cases h
  have hmapC : ∀ a b, Chain g L a b → Chain g' L' a b := fun a b h =>
    Chain.factor hf hw hL hcL hpi' hmove' h
  have hfromRoot : ∀ e n rn, g.Edge root e n → L n = some rn → TrueRootEdge fid gsid e rn.location →
      isPrefix [lab] e.path = true → (n = b ∨ Chain g L n b) → Chain g' L' c b := by
    intro e n rn he hn ht hpre hrest
    obtain ⟨e₂, he₂, hle₂⟩ := hmoved e n he hpre
    have h1 : Chain g' L' c n := .edge hcL (hL _ _ hn) he₂ ((hmove e n rn he hpre hn ht).cover hle₂)
    rcases hrest with rfl | hrest
    · exact h1
    · exact h1.trans (hmapC n b hrest)
  refine ⟨?_, hfromRoot⟩
  obtain ⟨e, n, rn, he, hl, hn, ht, hrest⟩ := h
  cases hpre : isPrefix [lab] e.path with
  | false =>
    obtain ⟨e', he', hle⟩ := hkeep root e n he (Or.inr hpre)
    exact ⟨e', n, rn, he', atLabel_cover hl hle, hL _ _ hn, ht.cover hle,
      hrest.imp id (hmapC n b)⟩
  | true =>
    obtain ⟨e₁, he₁, hle₁⟩ := hnew
    exact ⟨e₁, c, rc, he₁, atLabel_cover (atLabel_head hl hpre) hle₁, hcL, hpi.cover hle₁,
      Or.inr (hfromRoot e n rn he hn ht hpre hrest)⟩

end RootChain

namespace Complete

variable {g g1 g2 : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
  {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}

theorem held_mut (hc : Complete g L H root fid gsid ps) {x : Nat} {r : Reference} (hx : H x)
    (hxL : L x = some r) : g.mutability x = some r.mutable := by
  obtain ⟨r', hr', hm⟩ := hc.held x hx
  rw [hxL, Option.some.injEq] at hr'; subst hr'; exact hm

theorem key_held (hc : Complete g L H root fid gsid ps) {x : Nat} (hx : (g.find? x).isSome)
    (hr : x ≠ root) : H x := by
  rcases hc.keys x hx with h | h
  · exact absurd h hr
  · exact h

/-- The first reference of a root chain is held. -/
theorem rootChain_first_held (hc : Complete g L H root fid gsid ps) {e : BorrowGraph.Edge}
    {n : Nat} {rn : Reference} (he : g.Edge root e n) (hn : L n = some rn) : H n := by
  refine hc.key_held (Graph.Edge.keys he).2 ?_
  rintro rfl; rw [hc.rootNone] at hn; cases hn

/-- **Borrowing a local**: a new reference `c` to local `i` of the frame,
taking over the borrows of the local. -/
theorem borrowLoc {i c : Nat} {r : Reference} (hc : Complete g L H root fid gsid ps)
    (hloc : r.location = ⟨.local_ fid i, []⟩)
    (himm : r.mutable = false → g.hasConsistentMutableBorrows root (some (.local_ i)) = .ok false)
    (hfull : r.mutable = true → g.hasFullBorrows root = .ok false)
    (hnew : g.newRef c r.mutable = .ok g1) (hfac : g1.factor root [.local_ i] c = .ok g2) :
    Complete g2 (updL L c r) (fun x => H x ∨ x = c) root fid gsid ps := by
  have hcfresh := (Graph.newRef_ok hnew).1
  have hcH : ¬ H c := hc.not_held_of_fresh hcfresh
  have hcr : c ≠ root := fun e => by
    have := hc.root_key; rw [← e, hcfresh] at this; cases this
  have hw1 := Graph.newRef_wf hnew hc.wf
  have hb1 := Graph.newRef_back hnew hc.back
  obtain ⟨hw2, hmut2, -, -, -⟩ := Graph.factor_ok hfac hw1
  have hLc : updL L c r c = some r := updL_self _ _ _
  have hLx : ∀ x, x ≠ c → updL L c r x = L x := fun x hx => updL_ne r hx
  generalize updL L c r = L' at hLc hLx ⊢
  have hext : ∀ x r, (g.find? x).isSome → L x = some r → L' x = some r := by
    intro x r hx h
    have : x ≠ c := fun e => by subst e; rw [hcfresh] at hx; cases hx
    rw [hLx x this]; exact h
  have hHL : ∀ x r, H x → L' x = some r → L x = some r := fun x r hx h => by
    rwa [hLx x (fun e => hcH (e ▸ hx))] at h
  have hrootL : L' root = none := by rw [hLx root (Ne.symm hcr)]; exact hc.rootNone
  -- The edges moved from the root are true from `c`.
  have hpi : TrueRootEdge fid gsid ⟨true, [.local_ i]⟩ r.location := by
    simp [TrueRootEdge, hloc, absPath]
  have hmove : ∀ e y ry, g1.Edge root e y → isPrefix [.local_ i] e.path = true →
      L' y = some ry → TrueRootEdge fid gsid e ry.location →
      TrueEdge r.location r.mutable ⟨e.strong, e.path.drop 1⟩ ry.location ry.mutable := by
    intro e y ry he hpre hy ht
    obtain ⟨t, ht'⟩ := isPrefix_iff.1 hpre
    have he0 : g.Edge root e y := (Graph.newRef_edge hnew).1 he
    unfold TrueRootEdge at ht
    rw [← ht'] at ht ⊢
    simp only [List.singleton_append, List.drop_succ_cons, List.drop_zero] at ht ⊢
    obtain ⟨hroot, hpath⟩ := ht
    refine ⟨by rw [hroot, hloc], ?_, fun hm => ?_⟩
    · rw [hloc]; simpa [absPath] using hpath
    · -- An immutable borrow: the local's borrowers are immutable.
      have hyH : H y := hc.rootChain_first_held he0 (by
        have : y ≠ c := fun e => by
          subst e; have := (Graph.Edge.keys he0).2; rw [hcfresh] at this; cases this
        rwa [hLx y this] at hy)
      have hm' := Graph.hasConsistentMutableBorrows_false (himm hm) he0
        (by rw [RootChain.atLabel_iff]; exact Or.inr (Or.inr ⟨.local_ i, t, by rw [← ht']; rfl, rfl⟩))
      rw [hc.held_mut hyH (hHL y ry hyH hy), Option.some.injEq] at hm'
      exact hm'
  have hmapC : ∀ a b, Chain g L a b → Chain g2 L' a b := fun a b h =>
    Chain.factor hfac hw1 (fun _ _ h => h) hLc (fun rp h => by rw [hrootL] at h; cases h)
      (fun e y rp ry _ _ h => by rw [hrootL] at h; cases h)
      (Chain.mono (Graph.newRef_covers hnew) (h.extend' hext))
  have hmapR := fun l b (h : RootChain g L root fid gsid l b) =>
    RootChain.factorRoot (L' := L') hfac hw1 (fun _ _ h => h) hrootL hLc hpi hmove
      (RootChain.mono (Graph.newRef_covers hnew) (h.extend' hext))
  -- A reference into the local has a root chain whose first edge moves to `c`
  -- (unless it is a full borrow), and whose first borrower is immutable when
  -- the borrow is.
  have hlocal : ∀ x rx, H x → L x = some rx → rx.location.root = .local_ fid i →
      (r.mutable = true → Chain g2 L' c x) ∧ (r.mutable = false → rx.mutable = false) := by
    intro x rx hx hxL hroot
    obtain ⟨e, n, rn, he, hl, hn, ht, hrest⟩ := hc.rootLocal x rx i hx hxL hroot
    have hnH : H n := hc.rootChain_first_held he hn
    refine ⟨fun hm => ?_, fun hm => ?_⟩
    · have hne : e.path ≠ [] := Graph.hasFullBorrows_false (hfull hm) he
      have hpre : isPrefix [.local_ i] e.path = true := by
        rw [RootChain.atLabel_iff] at hl
        rcases hl with h | h | ⟨l, rest, h1, h2⟩
        · exact absurd h hne
        · cases h
        · simp only [Option.some.injEq] at h2; subst h2; rw [h1]; simp [isPrefix]
      exact ((hmapR _ x ⟨e, n, rn, he, hl, hn, ht, hrest⟩).2 e n rn
        ((Graph.newRef_edge hnew).2 he) (hext _ _ (Graph.Edge.keys he).2 hn) ht hpre
        (hrest.imp id (fun h => Chain.mono (Graph.newRef_covers hnew) (h.extend' hext))))
    · have hm' := Graph.hasConsistentMutableBorrows_false (himm hm) he hl
      rw [hc.held_mut hnH hn, Option.some.injEq] at hm'
      rcases hrest with rfl | hrest
      · rw [hn, Option.some.injEq] at hxL; subst hxL; exact hm'
      · exact (hrest.facts hn hxL).2.2 hm'
  have hnotin : ∀ rq : Reference, ¬ r.location.StrictlyInside rq.location := by
    rintro rq ⟨⟨-, hpre⟩, hne⟩
    rw [hloc] at hpre hne
    exact hne (List.prefix_nil.1 hpre).symm
  obtain ⟨e₁, he₁, hle₁⟩ := (Graph.factor_ok hfac hw1).2.2.2.2
  have hrootC : RootChain g2 L' root fid gsid (some (.local_ i)) c :=
    ⟨e₁, c, r, he₁, RootChain.atLabel_cover (by simp [Graph.atLabel]) hle₁, hLc, hpi.cover hle₁,
      Or.inl rfl⟩
  refine {
    wf := hw2
    back := Graph.factor_back hfac hw1 hb1
    rootNone := hrootL
    rootMut := by rw [hmut2, Graph.newRef_mutability hnew root (Ne.symm hcr)]; exact hc.rootMut
    rootNotHeld := fun h => h.elim hc.rootNotHeld (fun e => hcr e.symm)
    keys := fun x hx => ?_
    held := fun x hx => ?_
    strict := fun q rq x rx hq hx hqL hxL hm hin => ?_
    same := fun q rq x rx hq hx hne hqL hxL hm heq => ?_
    immInside := fun q rq x rx hq hx hqL hxL hm hin => ?_
    rootLocal := fun x rx i' hx hxL hr => ?_
    prov := fun x rx hx hxL => ?_ }
  · rw [Graph.isSome_of_mutability (hmut2 x)] at hx
    by_cases hxc : x = c
    · exact Or.inr (Or.inr hxc)
    · rw [Graph.isSome_of_mutability (Graph.newRef_mutability hnew x hxc)] at hx
      rcases hc.keys x hx with h | h
      · exact Or.inl h
      · exact Or.inr (Or.inl h)
  · rcases hx with hx | rfl
    · obtain ⟨r', hr, hm⟩ := hc.held x hx
      have hxc : x ≠ c := fun e => hcH (e ▸ hx)
      exact ⟨r', by rw [hLx x hxc]; exact hr, by
        rw [hmut2, Graph.newRef_mutability hnew x hxc]; exact hm⟩
    · exact ⟨r, hLc, by rw [hmut2, Graph.newRef_mutability_self hnew]⟩
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hmapC q x (hc.strict q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL; exact absurd hin (hnotin rq)
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      have hxL' := hHL x rx hx hxL
      exact (hlocal x rx hx hxL' (by rw [hin.1.1, hloc])).1 hm
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin (hnotin _)
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · rcases hc.same q rq x rx hq hx hne (hHL q rq hq hqL) (hHL x rx hx hxL) hm heq with h | h
      · exact Or.inl (hmapC q x h)
      · exact Or.inr (hmapC x q h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      have hqL' := hHL q rq hq hqL
      have hqroot : rq.location.root = .local_ fid i := by rw [← heq, hloc]
      cases hrm : r.mutable
      · exact absurd ((hlocal q rq hq hqL' hqroot).2 hrm) (by rw [hm]; simp)
      · exact Or.inr ((hlocal q rq hq hqL' hqroot).1 hrm)
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      exact Or.inl ((hlocal x rx hx (hHL x rx hx hxL) (by rw [heq, hloc])).1 hm)
    · exact absurd rfl hne
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hc.immInside q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin
    · rw [hLc, Option.some.injEq] at hxL; subst hxL; exact absurd hin (hnotin rq)
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      exact (hlocal x rx hx (hHL x rx hx hxL) (by rw [hin.1.1, hloc])).2 hm
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin (hnotin _)
  · rcases hx with hx | rfl
    · exact (hmapR _ x (hc.rootLocal x rx i' hx (hHL x rx hx hxL) hr)).1
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      rw [hloc] at hr
      simp only [Root.local_.injEq, true_and] at hr
      subst hr
      exact hrootC
  · rcases hx with hx | rfl
    · rcases hc.prov x rx hx (hHL x rx hx hxL) with h | ⟨d, h⟩ | h
      · exact Or.inl h
      · exact Or.inr (Or.inl ⟨d, (hmapR _ x h).1⟩)
      · exact Or.inr (Or.inr h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      exact Or.inl ⟨i, by rw [hloc]⟩

end Complete

theorem _root_.MoveBytecode.Location.eq_of {a b : Location} (h1 : a.root = b.root)
    (h2 : a.path = b.path) : a = b := by
  cases a; cases b; simp_all

/-! ## Prefixes of abstract paths -/

theorem prefix_label {pre ep A : List Label} {lab : Label} (h1 : pre ++ ep <+: A)
    (h2 : pre ++ [lab] <+: A) : ep = [] ∨ isPrefix [lab] ep = true := by
  rcases List.prefix_or_prefix_of_prefix h1 h2 with h | h
  · rw [List.prefix_append_right_inj] at h
    obtain ⟨t, ht⟩ := h
    cases ep with
    | nil => exact Or.inl rfl
    | cons a as =>
      right
      have : as = [] ∧ a = lab := by
        cases as <;> cases t <;> simp_all
      obtain ⟨rfl, rfl⟩ := this
      simp [isPrefix]
  · rw [List.prefix_append_right_inj] at h
    exact Or.inr (isPrefix_iff.2 h)

theorem prefix_self_append {pre ep : List Label} (h : pre ++ ep <+: pre) : ep = [] := by
  have := h.length_le
  simp only [List.length_append] at this
  exact List.eq_nil_of_length_eq_zero (by omega)

theorem prefix_of_strict_child {q p : List PathStep} {s : PathStep} (h1 : q <+: p ++ [s])
    (h2 : p ++ [s] ≠ q) : q <+: p := by
  rcases List.prefix_or_prefix_of_prefix h1 (List.prefix_append p [s]) with h | h
  · exact h
  · obtain ⟨t, rfl⟩ := h
    obtain ⟨u, hu⟩ := h1
    rw [List.append_assoc] at hu
    have hu' := List.append_cancel_left hu
    cases t with
    | nil => simp
    | cons a as =>
      cases as with
      | nil =>
        cases u with
        | nil => simp at hu'; subst hu'; exact absurd rfl h2
        | cons _ _ => simp at hu'
      | cons _ _ => simp at hu'

namespace Complete

variable {g g1 g2 g3 : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
  {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}

/-- What a chain from `p` to a reference inside `p.f` begins with: an edge at
field `f`, full or with a path starting with `f`. -/
theorem chain_first_at_field {p x f : Nat} {rp rx : Reference} {s : PathStep}
    (hs : absPath [s] = [.field f]) (h : Chain g L p x) (hpL : L p = some rp)
    (hxL : L x = some rx) (hin : rx.location.Inside ⟨rp.location.root, rp.location.path ++ [s]⟩) :
    ∃ e n rn, g.Edge p e n ∧ L n = some rn ∧
      TrueEdge rp.location rp.mutable e rn.location rn.mutable ∧ (n = x ∨ Chain g L n x) ∧
      (e.path = [] ∨ isPrefix [.field f] e.path = true) := by
  obtain ⟨e, n, rp', rn, hp', hn, he, ht, hrest⟩ := h.first
  rw [hpL, Option.some.injEq] at hp'; subst hp'
  refine ⟨e, n, rn, he, hn, ht, hrest, ?_⟩
  have h1 : absPath rp.location.path ++ e.path <+: absPath rx.location.path := by
    rcases hrest with rfl | hrest
    · rw [hn, Option.some.injEq] at hxL; subst hxL; exact ht.prefix
    · exact ht.prefix.trans (hrest.facts hn hxL).2.1
  have h2 : absPath rp.location.path ++ [.field f] <+: absPath rx.location.path := by
    have := hin.abs
    simp only [absPath_append, hs] at this
    exact this
  exact prefix_label h1 h2

end Complete

namespace Complete

variable {g g1 g2 g3 : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
  {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}

/-- **Borrowing a field** of held reference `p` (`MutBorrowField`,
`ImmBorrowField` and their variant forms), before `p` is released: a new
reference `c` one step `s` (field `f`) inside `p`, taking over the borrows of
`p` at `f`. -/
theorem fieldBorrowPre {p c f : Nat} {rp r : Reference} {s : PathStep}
    (hc : Complete g L H root fid gsid ps) (hp : H p) (hpL : L p = some rp)
    (hs : absPath [s] = [.field f]) (hloc : r.location = rp.location.child s)
    (hmut : r.mutable = true → rp.mutable = true)
    (hfull : r.mutable = true → g.hasFullBorrows p = .ok false)
    (hread : r.mutable = false → g.isReadable p (some (.field f)) = .ok true)
    (hnew : g.newRef c r.mutable = .ok g1) (hfac : g1.factor p [.field f] c = .ok g2) :
    Complete g2 (updL L c r) (fun x => H x ∨ x = c) root fid gsid ps := by
  have hcfresh := (Graph.newRef_ok hnew).1
  have hcH : ¬ H c := hc.not_held_of_fresh hcfresh
  have hcp : c ≠ p := fun e => hcH (e ▸ hp)
  have hcr : c ≠ root := fun e => by
    have := hc.root_key; rw [← e, hcfresh] at this; cases this
  have hrp : root ≠ p := fun e => hc.rootNotHeld (e ▸ hp)
  have hw1 := Graph.newRef_wf hnew hc.wf
  have hb1 := Graph.newRef_back hnew hc.back
  obtain ⟨hw2, hmut2, -, hmoved, hnewE⟩ := Graph.factor_ok hfac hw1
  -- Where `c` is.
  have hrin : r.location.Inside rp.location := by
    rw [hloc]; exact ⟨rfl, List.prefix_append _ _⟩
  have hrabs : absPath r.location.path = absPath rp.location.path ++ [.field f] := by
    rw [hloc]; simp [Location.child, absPath_append, hs]
  have hrinC : ∀ l : Location, l.Inside r.location →
      l.Inside ⟨rp.location.root, rp.location.path ++ [s]⟩ := by
    intro l h; rw [hloc] at h; exact h
  -- What the check said about `p`'s borrowers at `f`.
  have hreadF : r.mutable = false → rp.mutable = false ∨
      ∀ e y, g.Edge p e y → Graph.atLabel (some (.field f)) e = true →
        g.mutability y = some false := by
    intro hm
    rcases Graph.isReadable_true (hread hm) with h | h
    · rw [hc.held_mut hp hpL, Option.some.injEq] at h; exact Or.inl h
    · exact Or.inr h
  have hLc : updL L c r c = some r := updL_self _ _ _
  have hLx : ∀ x, x ≠ c → updL L c r x = L x := fun x hx => updL_ne r hx
  generalize updL L c r = L' at hLc hLx ⊢
  have hext : ∀ x r, (g.find? x).isSome → L x = some r → L' x = some r := by
    intro x r hx h
    have : x ≠ c := fun e => by subst e; rw [hcfresh] at hx; cases hx
    rw [hLx x this]; exact h
  have hHL : ∀ x r, H x → L' x = some r → L x = some r := fun x r hx h => by
    rwa [hLx x (fun e => hcH (e ▸ hx))] at h
  have hpL' : L' p = some rp := by rw [hLx p (Ne.symm hcp)]; exact hpL
  have hpi : ∀ rp', L' p = some rp' → g2.HasEdge p ⟨true, [.field f]⟩ c →
      TrueEdge rp'.location rp'.mutable ⟨true, [.field f]⟩ r.location r.mutable := by
    intro rp' h _
    rw [hpL', Option.some.injEq] at h; subst h
    refine ⟨hrin.1, by simp [hrabs], fun hm => ?_⟩
    cases hr : r.mutable
    · rfl
    · rw [hmut hr] at hm; cases hm
  have hmove : ∀ e y rp' ry, g1.Edge p e y → isPrefix [.field f] e.path = true →
      L' p = some rp' → L' y = some ry →
      TrueEdge rp'.location rp'.mutable e ry.location ry.mutable →
      TrueEdge r.location r.mutable ⟨e.strong, e.path.drop [Label.field f].length⟩
        ry.location ry.mutable := by
    intro e y rp' ry he hpre h hy ht
    rw [hpL', Option.some.injEq] at h; subst h
    have he0 : g.Edge p e y := (Graph.newRef_edge hnew).1 he
    obtain ⟨t, ht'⟩ := isPrefix_iff.1 hpre
    obtain ⟨hroot, hpath, hmt⟩ := ht
    refine ⟨hroot.trans hrin.1.symm, ?_, fun hm => ?_⟩
    · rw [hrabs]
      rw [← ht'] at hpath ⊢
      simp only [List.singleton_append, List.length_singleton, List.drop_succ_cons,
        List.drop_zero] at hpath ⊢
      split at hpath
      · rw [if_pos (by assumption), hpath]; simp
      · rw [if_neg (by assumption)]; simpa using hpath
    · rcases hreadF hm with h | h
      · exact hmt h
      · have hyH : H y := hc.key_held (Graph.Edge.keys he0).2 (by
          intro hyr; rw [hyr, hLx root (Ne.symm hcr), hc.rootNone] at hy; cases hy)
        have := h e y he0 (by
          rw [RootChain.atLabel_iff]; exact Or.inr (Or.inr ⟨.field f, t, by rw [← ht']; rfl, rfl⟩))
        rw [hc.held_mut hyH (hHL y ry hyH hy), Option.some.injEq] at this
        exact this
  have hmapC : ∀ a b, Chain g L a b → Chain g2 L' a b := fun a b h =>
    Chain.factor hfac hw1 (fun _ _ h => h) hLc hpi hmove
      (Chain.mono (Graph.newRef_covers hnew) (h.extend' hext))
  have hmapR : ∀ l b, RootChain g L root fid gsid l b → RootChain g2 L' root fid gsid l b :=
    fun l b h => RootChain.factor hfac hw1 hrp (fun _ _ h => h) hLc hpi hmove
      (RootChain.mono (Graph.newRef_covers hnew) (h.extend' hext))
  have hpc : Chain g2 L' p c := by
    obtain ⟨e', he', hle⟩ := hnewE
    exact .edge hpL' hLc he' ((hpi rp hpL' ⟨e', he', hle⟩).cover hle)
  -- A chain from `p` whose first edge moved continues from `c`.
  have hfrom : ∀ x e n rn, g.Edge p e n → L n = some rn →
      TrueEdge rp.location rp.mutable e rn.location rn.mutable → (n = x ∨ Chain g L n x) →
      isPrefix [.field f] e.path = true → Chain g2 L' c x := by
    intro x e n rn he hn ht hrest hpre
    have he1 : g1.Edge p e n := (Graph.newRef_edge hnew).2 he
    have hn' : L' n = some rn := hext _ _ (Graph.Edge.keys he).2 hn
    obtain ⟨e₂, he₂, hle₂⟩ := hmoved e n he1 hpre
    have h1 : Chain g2 L' c n := .edge hLc hn' he₂
      ((hmove e n rp rn he1 hpre hpL' hn' ht).cover hle₂)
    rcases hrest with rfl | hrest
    · exact h1
    · exact h1.trans (hmapC n x hrest)
  -- A reference strictly inside `c` is strictly inside `p`, reached from `p`
  -- by an edge at `f`.
  have hstrictP : ∀ rx : Reference,
      rx.location.Inside ⟨rp.location.root, rp.location.path ++ [s]⟩ →
      rx.location.StrictlyInside rp.location := by
    intro rx h
    have hl : rp.location.path.length + 1 ≤ rx.location.path.length := by
      have := h.2.length_le
      simpa using this
    refine ⟨⟨h.1, (List.prefix_append _ _).trans h.2⟩, fun he => ?_⟩
    rw [he] at hl
    omega
  have hatF : ∀ x (rx : Reference), H x → L x = some rx → rp.mutable = true →
      rx.location.Inside ⟨rp.location.root, rp.location.path ++ [s]⟩ →
      ∃ e n rn, g.Edge p e n ∧ L n = some rn ∧
        TrueEdge rp.location rp.mutable e rn.location rn.mutable ∧ (n = x ∨ Chain g L n x) ∧
        (e.path = [] ∨ isPrefix [.field f] e.path = true) := by
    intro x rx hx hxL hm hin
    exact chain_first_at_field hs (hc.strict p rp x rx hp hx hpL hxL hm (hstrictP rx hin))
      hpL hxL hin
  -- Through a full edge at `f`, everything is immutable when `c` is.
  have hfullImm : ∀ x (rx : Reference) (e : BorrowGraph.Edge) n (rn : Reference),
      r.mutable = false → rp.mutable = true → g.Edge p e n →
      L n = some rn → (n = x ∨ Chain g L n x) → L x = some rx →
      (e.path = [] ∨ isPrefix [.field f] e.path = true) → rx.mutable = false := by
    intro x rx e n rn hm hpm he hn hrest hxL hat
    rcases hreadF hm with h | h
    · rw [hpm] at h; cases h
    · have hnH : H n := hc.key_held (Graph.Edge.keys he).2 (by
        intro hnr; rw [hnr, hc.rootNone] at hn; cases hn)
      have := h e n he (by
        rw [RootChain.atLabel_iff]
        rcases hat with h0 | h0
        · exact Or.inl h0
        · obtain ⟨t, ht⟩ := isPrefix_iff.1 h0
          exact Or.inr (Or.inr ⟨.field f, t, by rw [← ht]; rfl, rfl⟩))
      rw [hc.held_mut hnH hn, Option.some.injEq] at this
      rcases hrest with rfl | hrest
      · rw [hn, Option.some.injEq] at hxL; subst hxL; exact this
      · exact (hrest.facts hn hxL).2.2 this
  refine {
    wf := hw2
    back := Graph.factor_back hfac hw1 hb1
    rootNone := by rw [hLx root (Ne.symm hcr)]; exact hc.rootNone
    rootMut := by rw [hmut2, Graph.newRef_mutability hnew root (Ne.symm hcr)]; exact hc.rootMut
    rootNotHeld := fun h => h.elim hc.rootNotHeld (fun e => hcr e.symm)
    keys := fun x hx => ?_
    held := fun x hx => ?_
    strict := fun q rq x rx hq hx hqL hxL hm hin => ?_
    same := fun q rq x rx hq hx hne hqL hxL hm heq => ?_
    immInside := fun q rq x rx hq hx hqL hxL hm hin => ?_
    rootLocal := fun x rx i hx hxL hr => ?_
    prov := fun x rx hx hxL => ?_ }
  · rw [Graph.isSome_of_mutability (hmut2 x)] at hx
    by_cases hxc : x = c
    · exact Or.inr (Or.inr hxc)
    · rw [Graph.isSome_of_mutability (Graph.newRef_mutability hnew x hxc)] at hx
      rcases hc.keys x hx with h | h
      · exact Or.inl h
      · exact Or.inr (Or.inl h)
  · rcases hx with hx | rfl
    · obtain ⟨r', hr, hm⟩ := hc.held x hx
      have hxc : x ≠ c := fun e => hcH (e ▸ hx)
      exact ⟨r', by rw [hLx x hxc]; exact hr, by
        rw [hmut2, Graph.newRef_mutability hnew x hxc]; exact hm⟩
    · exact ⟨r, hLc, by rw [hmut2, Graph.newRef_mutability_self hnew]⟩
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hmapC q x (hc.strict q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin)
    · -- `c` strictly inside `q`: `p` is inside `q` too.
      rw [hLc, Option.some.injEq] at hxL; subst hxL
      have hqL' := hHL q rq hq hqL
      have hpq : rp.location.Inside rq.location := by
        rw [hloc] at hin
        exact ⟨hin.1.1, prefix_of_strict_child hin.1.2 hin.2⟩
      by_cases hsame : rp.location = rq.location
      · by_cases hqp : q = p
        · subst hqp; exact hpc
        · rcases hc.same q rq p rp hq hp hqp hqL' hpL hm hsame with h | h
          · exact (hmapC q p h).trans hpc
          · -- A chain from `p` to a reference at `p` begins with a full edge.
            exfalso
            obtain ⟨e, n, rp', rn, hp', hn, he, ht, hrest⟩ := h.first
            rw [hpL, Option.some.injEq] at hp'; subst hp'
            have h1 : absPath rp.location.path ++ e.path <+: absPath rq.location.path := by
              rcases hrest with rfl | hrest
              · rw [hn, Option.some.injEq] at hqL'; subst hqL'; exact ht.prefix
              · exact ht.prefix.trans (hrest.facts hn hqL').2.1
            rw [← hsame] at h1
            have hnil := prefix_self_append h1
            cases hrm : r.mutable
            · have := hfullImm q rq e n rn hrm (by
                cases hpm : rp.mutable
                · have := (h.facts hpL hqL').2.2 hpm; rw [hm] at this; cases this
                · rfl) he hn hrest hqL' (Or.inl hnil)
              rw [hm] at this; cases this
            · exact Graph.hasFullBorrows_false (hfull hrm) he hnil
      · have hstrict : rp.location.StrictlyInside rq.location :=
          ⟨hpq, fun he => hsame (Location.eq_of hpq.1 he)⟩
        exact (hmapC q p (hc.strict q rq p rp hq hp hqL' hpL hm hstrict)).trans hpc
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      have hxL' := hHL x rx hx hxL
      obtain ⟨e, n, rn, he, hn, ht, hrest, hat⟩ := hatF x rx hx hxL' (hmut hm) (hrinC _ hin.1)
      rcases hat with hnil | hpre
      · exact absurd hnil (Graph.hasFullBorrows_false (hfull hm) he)
      · exact hfrom x e n rn he hn ht hrest hpre
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin StrictlyInside.ne
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · rcases hc.same q rq x rx hq hx hne (hHL q rq hq hqL) (hHL x rx hx hxL) hm heq with h | h
      · exact Or.inl (hmapC q x h)
      · exact Or.inr (hmapC x q h)
    · -- A mutable `q` at `c`'s location is strictly inside `p`.
      rw [hLc, Option.some.injEq] at hxL; subst hxL
      have hqL' := hHL q rq hq hqL
      have hqin : rq.location.Inside ⟨rp.location.root, rp.location.path ++ [s]⟩ := by
        rw [← heq, hloc]; exact Location.Inside.refl _
      cases hpm : rp.mutable
      · have := hc.immInside p rp q rq hp hq hpL hqL' hpm (hstrictP rq hqin)
        rw [hm] at this; cases this
      · obtain ⟨e, n, rn, he, hn, ht, hrest, hat⟩ := hatF q rq hq hqL' hpm hqin
        rcases hat with hnil | hpre
        · cases hrm : r.mutable
          · have := hfullImm q rq e n rn hrm hpm he hn hrest hqL' (Or.inl hnil)
            rw [hm] at this; cases this
          · exact absurd hnil (Graph.hasFullBorrows_false (hfull hrm) he)
        · exact Or.inr (hfrom q e n rn he hn ht hrest hpre)
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      have hxL' := hHL x rx hx hxL
      have hxin : rx.location.Inside ⟨rp.location.root, rp.location.path ++ [s]⟩ := by
        rw [heq, hloc]; exact Location.Inside.refl _
      obtain ⟨e, n, rn, he, hn, ht, hrest, hat⟩ := hatF x rx hx hxL' (hmut hm) hxin
      rcases hat with hnil | hpre
      · exact absurd hnil (Graph.hasFullBorrows_false (hfull hm) he)
      · exact Or.inl (hfrom x e n rn he hn ht hrest hpre)
    · exact absurd rfl hne
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hc.immInside q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin
    · -- `c` strictly inside an immutable `q`: `p` is inside `q`.
      rw [hLc, Option.some.injEq] at hxL; subst hxL
      have hqL' := hHL q rq hq hqL
      cases hrm : r.mutable
      · rfl
      · exfalso
        have hpm := hmut hrm
        have hpq : rp.location.Inside rq.location := by
          rw [hloc] at hin
          exact ⟨hin.1.1, prefix_of_strict_child hin.1.2 hin.2⟩
        by_cases hsame : rp.location = rq.location
        · have hqp : q ≠ p := fun e => by
            subst e; rw [hpL, Option.some.injEq] at hqL'; subst hqL'
            rw [hpm] at hm; cases hm
          rcases hc.same p rp q rq hp hq (Ne.symm hqp) hpL hqL' hpm hsame.symm with h | h
          · obtain ⟨e, n, rp', rn, hp', hn, he, ht, hrest⟩ := h.first
            rw [hpL, Option.some.injEq] at hp'; subst hp'
            have h1 : absPath rp.location.path ++ e.path <+: absPath rq.location.path := by
              rcases hrest with rfl | hrest
              · rw [hn, Option.some.injEq] at hqL'; subst hqL'; exact ht.prefix
              · exact ht.prefix.trans (hrest.facts hn hqL').2.1
            rw [← hsame] at h1
            exact Graph.hasFullBorrows_false (hfull hrm) he (prefix_self_append h1)
          · have := (h.facts hqL' hpL).2.2 hm
            rw [hpm] at this; cases this
        · have := hc.immInside q rq p rp hq hp hqL' hpL hm
            ⟨hpq, fun he => hsame (Location.eq_of hpq.1 he)⟩
          rw [hpm] at this; cases this
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      have hxL' := hHL x rx hx hxL
      have hxin := hrinC _ hin.1
      cases hpm : rp.mutable
      · exact hc.immInside p rp x rx hp hx hpL hxL' hpm (hstrictP rx hxin)
      · obtain ⟨e, n, rn, he, hn, ht, hrest, hat⟩ := hatF x rx hx hxL' hpm hxin
        exact hfullImm x rx e n rn hm hpm he hn hrest hxL' hat
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin StrictlyInside.ne
  · rcases hx with hx | rfl
    · exact hmapR _ x (hc.rootLocal x rx i hx (hHL x rx hx hxL) hr)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      obtain ⟨e', he', hle⟩ := hnewE
      exact (hmapR _ p (hc.rootLocal p rp i hp hpL (by rw [← hr, hrin.1]))).snoc hpL' hLc he'
        ((hpi rp hpL' ⟨e', he', hle⟩).cover hle)
  · rcases hx with hx | rfl
    · rcases hc.prov x rx hx (hHL x rx hx hxL) with h | ⟨d, h⟩ | h
      · exact Or.inl h
      · exact Or.inr (Or.inl ⟨d, hmapR _ x h⟩)
      · exact Or.inr (Or.inr h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      rcases hc.prov p rp hp hpL with h | ⟨d, h⟩ | ⟨π, hπ, hin, hπm⟩
      · obtain ⟨i, hi⟩ := h; exact Or.inl ⟨i, by rw [hrin.1, hi]⟩
      · obtain ⟨e', he', hle⟩ := hnewE
        exact Or.inr (Or.inl ⟨d, (hmapR _ p h).snoc hpL' hLc he'
          ((hpi rp hpL' ⟨e', he', hle⟩).cover hle)⟩)
      · exact Or.inr (Or.inr ⟨π, hπ, hrin.trans hin, fun h => hπm (hmut h)⟩)

end Complete

namespace Complete

variable {g g1 g2 g3 : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
  {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}

theorem congrH {H' : Nat → Prop} (hc : Complete g L H root fid gsid ps) (hH : ∀ x, H x ↔ H' x) :
    Complete g L H' root fid gsid ps := by
  have : H = H' := funext fun x => propext (hH x)
  subst this; exact hc

/-- **Freezing** held mutable reference `p`, before `p` is released: a new
immutable reference `c` to the same location, taking over the borrows of `p`,
all immutable. -/
theorem freezePre {p c : Nat} {rp r : Reference} (hc : Complete g L H root fid gsid ps) (hp : H p)
    (hpL : L p = some rp) (hpm : rp.mutable = true) (hloc : r.location = rp.location)
    (hrm : r.mutable = false) (hfrz : g.isFreezable p none = .ok true)
    (hnew : g.newRef c false = .ok g1) (hfac : g1.factor p [] c = .ok g2) :
    Complete g2 (updL L c r) (fun x => H x ∨ x = c) root fid gsid ps := by
  have hcfresh := (Graph.newRef_ok hnew).1
  have hcH : ¬ H c := hc.not_held_of_fresh hcfresh
  have hcp : c ≠ p := fun e => hcH (e ▸ hp)
  have hcr : c ≠ root := fun e => by
    have := hc.root_key; rw [← e, hcfresh] at this; cases this
  have hrp : root ≠ p := fun e => hc.rootNotHeld (e ▸ hp)
  have hw1 := Graph.newRef_wf hnew hc.wf
  have hb1 := Graph.newRef_back hnew hc.back
  obtain ⟨hw2, hmut2, -, hmoved, hnewE⟩ := Graph.factor_ok hfac hw1
  -- Every borrower of `p` is immutable.
  have hkids : ∀ e y, g.Edge p e y → g.mutability y = some false := fun e y he =>
    (Graph.isFreezable_true hfrz).2 e y he (by rw [RootChain.atLabel_iff]; cases e.path <;> simp)
  have hLc : updL L c r c = some r := updL_self _ _ _
  have hLx : ∀ x, x ≠ c → updL L c r x = L x := fun x hx => updL_ne r hx
  generalize updL L c r = L' at hLc hLx ⊢
  have hext : ∀ x r, (g.find? x).isSome → L x = some r → L' x = some r := by
    intro x r hx h
    have : x ≠ c := fun e => by subst e; rw [hcfresh] at hx; cases hx
    rw [hLx x this]; exact h
  have hHL : ∀ x r, H x → L' x = some r → L x = some r := fun x r hx h => by
    rwa [hLx x (fun e => hcH (e ▸ hx))] at h
  have hpL' : L' p = some rp := by rw [hLx p (Ne.symm hcp)]; exact hpL
  -- The first borrower on a chain from `p` is immutable, and so is the end.
  have hchainImm : ∀ x rx, L x = some rx → Chain g L p x → rx.mutable = false := by
    intro x rx hxL h
    obtain ⟨e, n, rp', rn, hp', hn, he, ht, hrest⟩ := h.first
    have hnH : H n := hc.key_held (Graph.Edge.keys he).2 (by
      intro hnr; rw [hnr, hc.rootNone] at hn; cases hn)
    have hnm := hkids e n he
    rw [hc.held_mut hnH hn, Option.some.injEq] at hnm
    rcases hrest with rfl | hrest
    · rw [hn, Option.some.injEq] at hxL; subst hxL; exact hnm
    · exact (hrest.facts hn hxL).2.2 hnm
  have hpi : ∀ rp', L' p = some rp' → g2.HasEdge p ⟨true, []⟩ c →
      TrueEdge rp'.location rp'.mutable ⟨true, []⟩ r.location r.mutable := by
    intro rp' h _
    rw [hpL', Option.some.injEq] at h; subst h
    refine ⟨by rw [hloc], by simp [hloc], fun hm => ?_⟩
    rw [hpm] at hm; cases hm
  have hmove : ∀ e y rp' ry, g1.Edge p e y → isPrefix [] e.path = true → L' p = some rp' →
      L' y = some ry → TrueEdge rp'.location rp'.mutable e ry.location ry.mutable →
      TrueEdge r.location r.mutable ⟨e.strong, e.path.drop ([] : List Label).length⟩
        ry.location ry.mutable := by
    intro e y rp' ry he _ h hy ht
    rw [hpL', Option.some.injEq] at h; subst h
    have he0 : g.Edge p e y := (Graph.newRef_edge hnew).1 he
    have hyH : H y := hc.key_held (Graph.Edge.keys he0).2 (by
      intro hyr; rw [hyr, hLx root (Ne.symm hcr), hc.rootNone] at hy; cases hy)
    have hym := hkids e y he0
    rw [hc.held_mut hyH (hHL y ry hyH hy), Option.some.injEq] at hym
    obtain ⟨h1, h2, -⟩ := ht
    exact ⟨by rw [h1, hloc], by simpa [hloc] using h2, fun _ => hym⟩
  have hmapC : ∀ a b, Chain g L a b → Chain g2 L' a b := fun a b h =>
    Chain.factor hfac hw1 (fun _ _ h => h) hLc hpi hmove
      (Chain.mono (Graph.newRef_covers hnew) (h.extend' hext))
  have hmapR : ∀ l b, RootChain g L root fid gsid l b → RootChain g2 L' root fid gsid l b :=
    fun l b h => RootChain.factor hfac hw1 hrp (fun _ _ h => h) hLc hpi hmove
      (RootChain.mono (Graph.newRef_covers hnew) (h.extend' hext))
  have hpc : Chain g2 L' p c := by
    obtain ⟨e', he', hle⟩ := hnewE
    exact .edge hpL' hLc he' ((hpi rp hpL' ⟨e', he', hle⟩).cover hle)
  refine {
    wf := hw2
    back := Graph.factor_back hfac hw1 hb1
    rootNone := by rw [hLx root (Ne.symm hcr)]; exact hc.rootNone
    rootMut := by rw [hmut2, Graph.newRef_mutability hnew root (Ne.symm hcr)]; exact hc.rootMut
    rootNotHeld := fun h => h.elim hc.rootNotHeld (fun e => hcr e.symm)
    keys := fun x hx => ?_
    held := fun x hx => ?_
    strict := fun q rq x rx hq hx hqL hxL hm hin => ?_
    same := fun q rq x rx hq hx hne hqL hxL hm heq => ?_
    immInside := fun q rq x rx hq hx hqL hxL hm hin => ?_
    rootLocal := fun x rx i hx hxL hr => ?_
    prov := fun x rx hx hxL => ?_ }
  · rw [Graph.isSome_of_mutability (hmut2 x)] at hx
    by_cases hxc : x = c
    · exact Or.inr (Or.inr hxc)
    · rw [Graph.isSome_of_mutability (Graph.newRef_mutability hnew x hxc)] at hx
      rcases hc.keys x hx with h | h
      · exact Or.inl h
      · exact Or.inr (Or.inl h)
  · rcases hx with hx | rfl
    · obtain ⟨r', hr, hm⟩ := hc.held x hx
      have hxc : x ≠ c := fun e => hcH (e ▸ hx)
      exact ⟨r', by rw [hLx x hxc]; exact hr, by
        rw [hmut2, Graph.newRef_mutability hnew x hxc]; exact hm⟩
    · exact ⟨r, hLc, by rw [hmut2, Graph.newRef_mutability_self hnew, hrm]⟩
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hmapC q x (hc.strict q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      rw [hloc] at hin
      exact (hmapC q p (hc.strict q rq p rp hq hp (hHL q rq hq hqL) hpL hm hin)).trans hpc
    · rw [hLc, Option.some.injEq] at hqL; subst hqL; rw [hrm] at hm; cases hm
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin StrictlyInside.ne
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · rcases hc.same q rq x rx hq hx hne (hHL q rq hq hqL) (hHL x rx hx hxL) hm heq with h | h
      · exact Or.inl (hmapC q x h)
      · exact Or.inr (hmapC x q h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      have hqL' := hHL q rq hq hqL
      by_cases hqp : q = p
      · subst hqp; exact Or.inl hpc
      · rw [hloc] at heq
        rcases hc.same q rq p rp hq hp hqp hqL' hpL hm heq with h | h
        · exact Or.inl ((hmapC q p h).trans hpc)
        · have := hchainImm q rq hqL' h; rw [hm] at this; cases this
    · rw [hLc, Option.some.injEq] at hqL; subst hqL; rw [hrm] at hm; cases hm
    · exact absurd rfl hne
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hc.immInside q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin
    · rw [hLc, Option.some.injEq] at hxL; subst hxL; exact hrm
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      have hxL' := hHL x rx hx hxL
      rw [hloc] at hin
      exact hchainImm x rx hxL' (hc.strict p rp x rx hp hx hpL hxL' hpm hin)
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin StrictlyInside.ne
  · rcases hx with hx | rfl
    · exact hmapR _ x (hc.rootLocal x rx i hx (hHL x rx hx hxL) hr)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      obtain ⟨e', he', hle⟩ := hnewE
      exact (hmapR _ p (hc.rootLocal p rp i hp hpL (by rw [← hr, hloc]))).snoc hpL' hLc he'
        ((hpi rp hpL' ⟨e', he', hle⟩).cover hle)
  · rcases hx with hx | rfl
    · rcases hc.prov x rx hx (hHL x rx hx hxL) with h | ⟨d, h⟩ | h
      · exact Or.inl h
      · exact Or.inr (Or.inl ⟨d, hmapR _ x h⟩)
      · exact Or.inr (Or.inr h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      rcases hc.prov p rp hp hpL with ⟨i, hi⟩ | ⟨d, h⟩ | ⟨π, hπ, hin, -⟩
      · exact Or.inl ⟨i, by rw [hloc, hi]⟩
      · obtain ⟨e', he', hle⟩ := hnewE
        exact Or.inr (Or.inl ⟨d, (hmapR _ p h).snoc hpL' hLc he'
          ((hpi rp hpL' ⟨e', he', hle⟩).cover hle)⟩)
      · exact Or.inr (Or.inr ⟨π, hπ, by rw [hloc]; exact hin, fun h => by rw [hrm] at h; cases h⟩)

end Complete

namespace Complete

variable {g g1 g2 g3 : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
  {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}

/-- **Borrowing a vector element** through held reference `p`, before `p` is
released: a new reference `c` to an element, weakly borrowing all of `p`. A
mutable borrow needs `p` unborrowed; an immutable one needs `p` immutable or
borrowed only immutably. -/
theorem elemPre {p c k : Nat} {rp r : Reference} (hc : Complete g L H root fid gsid ps)
    (hp : H p) (hpL : L p = some rp) (hloc : r.location = rp.location.child (.element k))
    (hmut : r.mutable = true → rp.mutable = true)
    (hwr : r.mutable = true → g.isWritable p = .ok true)
    (hrd : r.mutable = false → g.isReadable p none = .ok true)
    (hnew : g.newRef c r.mutable = .ok g1) (hadd : g1.addEdge p ⟨false, []⟩ c = .ok g2) :
    Complete g2 (updL L c r) (fun x => H x ∨ x = c) root fid gsid ps := by
  have hcfresh := (Graph.newRef_ok hnew).1
  have hcH : ¬ H c := hc.not_held_of_fresh hcfresh
  have hcp : c ≠ p := fun e => hcH (e ▸ hp)
  have hcr : c ≠ root := fun e => by
    have := hc.root_key; rw [← e, hcfresh] at this; cases this
  have hw1 := Graph.newRef_wf hnew hc.wf
  have hb1 := Graph.newRef_back hnew hc.back
  obtain ⟨hcov2, hnewE⟩ := Graph.addEdge_covers hadd hw1.sets
  have hmut2 := Graph.addEdge_mutability hadd
  have hrin : r.location.Inside rp.location := by
    rw [hloc]; exact ⟨rfl, List.prefix_append _ _⟩
  have hrstrict : r.location.StrictlyInside rp.location := by
    refine ⟨hrin, fun he => ?_⟩
    rw [hloc] at he
    simp [Location.child] at he
  -- Through `p`, everything is immutable when `c` is.
  have hreadF : r.mutable = false → rp.mutable = false ∨
      ∀ e y, g.Edge p e y → g.mutability y = some false := by
    intro hm
    rcases Graph.isReadable_true (hrd hm) with h | h
    · rw [hc.held_mut hp hpL, Option.some.injEq] at h; exact Or.inl h
    · exact Or.inr fun e y he => h e y he (by rw [RootChain.atLabel_iff]; cases e.path <;> simp)
  have hchainImm : ∀ x rx, r.mutable = false → L x = some rx → Chain g L p x →
      rx.mutable = false := by
    intro x rx hm hxL h
    rcases hreadF hm with h0 | h0
    · exact (h.facts hpL hxL).2.2 h0
    · obtain ⟨e, n, rp', rn, hp', hn, he, ht, hrest⟩ := h.first
      have hnH : H n := hc.key_held (Graph.Edge.keys he).2 (by
        intro hnr; rw [hnr, hc.rootNone] at hn; cases hn)
      have hnm := h0 e n he
      rw [hc.held_mut hnH hn, Option.some.injEq] at hnm
      rcases hrest with rfl | hrest
      · rw [hn, Option.some.injEq] at hxL; subst hxL; exact hnm
      · exact (hrest.facts hn hxL).2.2 hnm
  -- A mutable borrow: `p` borrows nothing, so nothing is strictly inside it.
  have hnone : r.mutable = true → ∀ x rx, H x → L x = some rx →
      ¬ rx.location.StrictlyInside rp.location := by
    intro hm x rx hx hxL hin
    obtain ⟨e, n, rp', rn, -, -, he, -⟩ := (hc.strict p rp x rx hp hx hpL hxL (hmut hm) hin).first
    exact (Graph.isWritable_true (hwr hm)).2 e n he
  have hLc : updL L c r c = some r := updL_self _ _ _
  have hLx : ∀ x, x ≠ c → updL L c r x = L x := fun x hx => updL_ne r hx
  generalize updL L c r = L' at hLc hLx ⊢
  have hext : ∀ x r, (g.find? x).isSome → L x = some r → L' x = some r := by
    intro x r hx h
    have : x ≠ c := fun e => by subst e; rw [hcfresh] at hx; cases hx
    rw [hLx x this]; exact h
  have hHL : ∀ x r, H x → L' x = some r → L x = some r := fun x r hx h => by
    rwa [hLx x (fun e => hcH (e ▸ hx))] at h
  have hpL' : L' p = some rp := by rw [hLx p (Ne.symm hcp)]; exact hpL
  have hcov : g.Covers g2 := (Graph.newRef_covers hnew).trans hcov2
  have hmapC : ∀ a b, Chain g L a b → Chain g2 L' a b := fun a b h =>
    Chain.mono hcov (h.extend' hext)
  have hmapR : ∀ l b, RootChain g L root fid gsid l b → RootChain g2 L' root fid gsid l b :=
    fun l b h => RootChain.mono hcov (h.extend' hext)
  have hpc : Chain g2 L' p c := by
    obtain ⟨e', he', hle⟩ := hnewE
    refine .edge hpL' hLc he' (TrueEdge.cover ⟨hrin.1, ?_, fun h0 => ?_⟩ hle)
    · simp [hloc, Location.child, absPath_append, absPath]
    · cases hrm : r.mutable
      · rfl
      · rw [hmut hrm] at h0; cases h0
  refine {
    wf := Graph.addEdge_wf hadd hw1
    back := Graph.addEdge_back hadd hb1
    rootNone := by rw [hLx root (Ne.symm hcr)]; exact hc.rootNone
    rootMut := by rw [hmut2, Graph.newRef_mutability hnew root (Ne.symm hcr)]; exact hc.rootMut
    rootNotHeld := fun h => h.elim hc.rootNotHeld (fun e => hcr e.symm)
    keys := fun x hx => ?_
    held := fun x hx => ?_
    strict := fun q rq x rx hq hx hqL hxL hm hin => ?_
    same := fun q rq x rx hq hx hne hqL hxL hm heq => ?_
    immInside := fun q rq x rx hq hx hqL hxL hm hin => ?_
    rootLocal := fun x rx i hx hxL hr => ?_
    prov := fun x rx hx hxL => ?_ }
  · rw [Graph.isSome_of_mutability (hmut2 x)] at hx
    by_cases hxc : x = c
    · exact Or.inr (Or.inr hxc)
    · rw [Graph.isSome_of_mutability (Graph.newRef_mutability hnew x hxc)] at hx
      rcases hc.keys x hx with h | h
      · exact Or.inl h
      · exact Or.inr (Or.inl h)
  · rcases hx with hx | rfl
    · obtain ⟨r', hr, hm⟩ := hc.held x hx
      have hxc : x ≠ c := fun e => hcH (e ▸ hx)
      exact ⟨r', by rw [hLx x hxc]; exact hr, by
        rw [hmut2, Graph.newRef_mutability hnew x hxc]; exact hm⟩
    · exact ⟨r, hLc, by rw [hmut2, Graph.newRef_mutability_self hnew]⟩
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hmapC q x (hc.strict q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin)
    · -- `c` strictly inside `q`: `p` is inside `q` too.
      rw [hLc, Option.some.injEq] at hxL; subst hxL
      have hqL' := hHL q rq hq hqL
      have hpq : rp.location.Inside rq.location := by
        rw [hloc] at hin
        exact ⟨hin.1.1, prefix_of_strict_child hin.1.2 hin.2⟩
      by_cases hsame : rp.location = rq.location
      · by_cases hqp : q = p
        · subst hqp; exact hpc
        · rcases hc.same q rq p rp hq hp hqp hqL' hpL hm hsame with h | h
          · exact (hmapC q p h).trans hpc
          · exfalso
            cases hrm : r.mutable
            · have := hchainImm q rq hrm hqL' h; rw [hm] at this; cases this
            · obtain ⟨e, n, -, -, -, -, he, -⟩ := h.first
              exact (Graph.isWritable_true (hwr hrm)).2 e n he
      · exact (hmapC q p (hc.strict q rq p rp hq hp hqL' hpL hm
          ⟨hpq, fun he => hsame (Location.eq_of hpq.1 he)⟩)).trans hpc
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      exact absurd (StrictlyInside.trans' hin hrin) (hnone hm x rx hx (hHL x rx hx hxL))
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin StrictlyInside.ne
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · rcases hc.same q rq x rx hq hx hne (hHL q rq hq hqL) (hHL x rx hx hxL) hm heq with h | h
      · exact Or.inl (hmapC q x h)
      · exact Or.inr (hmapC x q h)
    · -- A mutable `q` at an element of `p`.
      exfalso
      rw [hLc, Option.some.injEq] at hxL; subst hxL
      have hqL' := hHL q rq hq hqL
      have hqin : rq.location.StrictlyInside rp.location := by rw [← heq]; exact hrstrict
      cases hpm : rp.mutable
      · have := hc.immInside p rp q rq hp hq hpL hqL' hpm hqin; rw [hm] at this; cases this
      · have h := hc.strict p rp q rq hp hq hpL hqL' hpm hqin
        cases hrm : r.mutable
        · have := hchainImm q rq hrm hqL' h; rw [hm] at this; cases this
        · exact hnone hrm q rq hq hqL' hqin
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      exact absurd (by rw [heq]; exact hrstrict) (hnone hm x rx hx (hHL x rx hx hxL))
    · exact absurd rfl hne
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hc.immInside q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      have hqL' := hHL q rq hq hqL
      cases hrm : r.mutable
      · rfl
      · exfalso
        have hpm := hmut hrm
        have hpq : rp.location.Inside rq.location := by
          rw [hloc] at hin
          exact ⟨hin.1.1, prefix_of_strict_child hin.1.2 hin.2⟩
        by_cases hsame : rp.location = rq.location
        · have hqp : q ≠ p := fun e => by
            subst e; rw [hpL, Option.some.injEq] at hqL'; subst hqL'
            rw [hpm] at hm; cases hm
          rcases hc.same p rp q rq hp hq (Ne.symm hqp) hpL hqL' hpm hsame.symm with h | h
          · obtain ⟨e, n, -, -, -, -, he, -⟩ := h.first
            exact (Graph.isWritable_true (hwr hrm)).2 e n he
          · have := (h.facts hqL' hpL).2.2 hm; rw [hpm] at this; cases this
        · have := hc.immInside q rq p rp hq hp hqL' hpL hm
            ⟨hpq, fun he => hsame (Location.eq_of hpq.1 he)⟩
          rw [hpm] at this; cases this
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      have hxL' := hHL x rx hx hxL
      have hxin := StrictlyInside.trans' hin hrin
      cases hpm : rp.mutable
      · exact hc.immInside p rp x rx hp hx hpL hxL' hpm hxin
      · exact hchainImm x rx hm hxL' (hc.strict p rp x rx hp hx hpL hxL' hpm hxin)
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin StrictlyInside.ne
  · rcases hx with hx | rfl
    · exact hmapR _ x (hc.rootLocal x rx i hx (hHL x rx hx hxL) hr)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      obtain ⟨e', he', hle⟩ := hnewE
      have hr' : rp.location.root = .local_ fid i := by rw [← hrin.1, hr]
      refine (hmapR _ p (hc.rootLocal p rp i hp hpL hr')).snoc hpL' hLc he' (TrueEdge.cover ⟨hrin.1,
        ?_, fun h0 => ?_⟩ hle)
      · simp [hloc, Location.child, absPath_append, absPath]
      · cases hrm : r.mutable
        · rfl
        · rw [hmut hrm] at h0; cases h0
  · rcases hx with hx | rfl
    · rcases hc.prov x rx hx (hHL x rx hx hxL) with h | ⟨d, h⟩ | h
      · exact Or.inl h
      · exact Or.inr (Or.inl ⟨d, hmapR _ x h⟩)
      · exact Or.inr (Or.inr h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      rcases hc.prov p rp hp hpL with ⟨i, hi⟩ | ⟨d, h⟩ | ⟨π, hπ, hin, hπm⟩
      · exact Or.inl ⟨i, by rw [hrin.1, hi]⟩
      · obtain ⟨e', he', hle⟩ := hnewE
        refine Or.inr (Or.inl ⟨d, (hmapR _ p h).snoc hpL' hLc he' (TrueEdge.cover ⟨hrin.1,
          ?_, fun h0 => ?_⟩ hle)⟩)
        · simp [hloc, Location.child, absPath_append, absPath]
        · cases hrm : r.mutable
          · rfl
          · rw [hmut hrm] at h0; cases h0
      · exact Or.inr (Or.inr ⟨π, hπ, hrin.trans hin, fun h => hπm (hmut h)⟩)

end Complete

namespace Complete

variable {g g1 g2 : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
  {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}

/-- A root chain to a reference into a resource of type `gsid d` begins with
an edge at `global d`, when the module's definitions name distinct types. -/
theorem rootChain_at_global (hginj : ∀ d₁ d₂ s, gsid d₁ = some s → gsid d₂ = some s → d₁ = d₂)
    {l : Option Label} {x d : Nat} {rx : Reference} {a : Address} {sid : StructId}
    (h : RootChain g L root fid gsid l x) (hxL : L x = some rx)
    (hroot : rx.location.root = .global a sid) (hd : gsid d = some sid.erase) :
    ∃ e n rn, g.Edge root e n ∧ Graph.atLabel (some (.global d)) e = true ∧ L n = some rn ∧
      (n = x ∨ Chain g L n x) := by
  obtain ⟨e, n, rn, he, -, hn, ht, hrest⟩ := h
  refine ⟨e, n, rn, he, ?_, hn, hrest⟩
  have hnroot : rn.location.root = .global a sid := by
    rcases hrest with rfl | hrest
    · rw [hn, Option.some.injEq] at hxL; subst hxL; exact hroot
    · rw [← (hrest.facts hn hxL).1]; exact hroot
  rw [RootChain.atLabel_iff]
  unfold TrueRootEdge at ht
  cases hp : e.path with
  | nil => exact Or.inl rfl
  | cons l rest =>
    rw [hp] at ht
    cases l with
    | local_ i => rw [hnroot] at ht; cases ht.1
    | global d' =>
      obtain ⟨⟨a', sid', hs', hr'⟩, -⟩ := ht
      rw [hnroot, Root.global.injEq] at hr'
      have hdd : d' = d := hginj d' d sid'.erase hs' (hr'.2 ▸ hd)
      exact Or.inr (Or.inr ⟨.global d', rest, rfl, by rw [hdd]⟩)
    | field _ => exact ht.elim

/-- A reference into a resource of a type of the frame's module, when no
reference parameter is into that type, was borrowed in the frame: it has a
root chain whose first edge is at the type. -/
theorem global_rootChain (hc : Complete g L H root fid gsid ps)
    (hginj : ∀ d₁ d₂ s, gsid d₁ = some s → gsid d₂ = some s → d₁ = d₂)
    {x d : Nat} {rx : Reference} {a : Address} {sid : StructId} (hx : H x) (hxL : L x = some rx)
    (hroot : rx.location.root = .global a sid) (hd : gsid d = some sid.erase)
    (hparams : ∀ π ∈ ps, ∀ a', π.location.root ≠ .global a' sid) :
    ∃ e n rn, g.Edge root e n ∧ Graph.atLabel (some (.global d)) e = true ∧ L n = some rn ∧
      (n = x ∨ Chain g L n x) := by
  rcases hc.prov x rx hx hxL with ⟨i, hi⟩ | ⟨d', h⟩ | ⟨π, hπ, hin, -⟩
  · rw [hroot] at hi; cases hi
  · exact rootChain_at_global hginj h hxL hroot hd
  · exact absurd (by rw [← hin.1, hroot]) (hparams π hπ a)

/-- **Borrowing a resource** of the frame's module, `gsid d`, under an
address: a new reference `c`, which the frame root borrows weakly at the
type. A mutable borrow needs the type unborrowed; an immutable one needs it
borrowed only immutably. -/
theorem globalBorrow {c d : Nat} {r : Reference} {a : Address} {sid : StructId}
    (hc : Complete g L H root fid gsid ps)
    (hginj : ∀ d₁ d₂ s, gsid d₁ = some s → gsid d₂ = some s → d₁ = d₂)
    (hd : gsid d = some sid.erase) (hloc : r.location = ⟨.global a sid, []⟩)
    (hparams : ∀ π ∈ ps, ∀ a', π.location.root ≠ .global a' sid)
    (hmb : r.mutable = true → g.hasConsistentBorrows root (some (.global d)) = .ok false)
    (hib : g.hasConsistentMutableBorrows root (some (.global d)) = .ok false)
    (hnew : g.newRef c r.mutable = .ok g1) (hadd : g1.addEdge root ⟨false, [.global d]⟩ c = .ok g2) :
    Complete g2 (updL L c r) (fun x => H x ∨ x = c) root fid gsid ps := by
  have hcfresh := (Graph.newRef_ok hnew).1
  have hcH : ¬ H c := hc.not_held_of_fresh hcfresh
  have hcr : c ≠ root := fun e => by
    have := hc.root_key; rw [← e, hcfresh] at this; cases this
  have hw1 := Graph.newRef_wf hnew hc.wf
  have hb1 := Graph.newRef_back hnew hc.back
  obtain ⟨hcov2, hnewE⟩ := Graph.addEdge_covers hadd hw1.sets
  have hmut2 := Graph.addEdge_mutability hadd
  -- What is in the resource was borrowed here, immutably unless the borrow is.
  have hinR : ∀ x rx, H x → L x = some rx → rx.location.root = .global a sid →
      (r.mutable = false → rx.mutable = false) ∧ r.mutable = false := by
    intro x rx hx hxL hroot
    obtain ⟨e, n, rn, he, hl, hn, hrest⟩ := hc.global_rootChain hginj hx hxL hroot hd hparams
    have hnH : H n := hc.key_held (Graph.Edge.keys he).2 (by
      intro hnr; rw [hnr, hc.rootNone] at hn; cases hn)
    have hnm := Graph.hasConsistentMutableBorrows_false hib he hl
    rw [hc.held_mut hnH hn, Option.some.injEq] at hnm
    have hxm : rx.mutable = false := by
      rcases hrest with rfl | hrest
      · rw [hn, Option.some.injEq] at hxL; subst hxL; exact hnm
      · exact (hrest.facts hn hxL).2.2 hnm
    refine ⟨fun _ => hxm, ?_⟩
    cases hrm : r.mutable
    · rfl
    · have := Graph.hasConsistentBorrows_false (hmb hrm) he; rw [hl] at this; cases this
  have hnotin : ∀ rq : Reference, ¬ r.location.StrictlyInside rq.location := by
    rintro rq ⟨⟨-, hpre⟩, hne⟩
    rw [hloc] at hpre hne
    exact hne (List.prefix_nil.1 hpre).symm
  have hLc : updL L c r c = some r := updL_self _ _ _
  have hLx : ∀ x, x ≠ c → updL L c r x = L x := fun x hx => updL_ne r hx
  generalize updL L c r = L' at hLc hLx ⊢
  have hext : ∀ x r, (g.find? x).isSome → L x = some r → L' x = some r := by
    intro x r hx h
    have : x ≠ c := fun e => by subst e; rw [hcfresh] at hx; cases hx
    rw [hLx x this]; exact h
  have hHL : ∀ x r, H x → L' x = some r → L x = some r := fun x r hx h => by
    rwa [hLx x (fun e => hcH (e ▸ hx))] at h
  have hcov : g.Covers g2 := (Graph.newRef_covers hnew).trans hcov2
  have hmapC : ∀ a b, Chain g L a b → Chain g2 L' a b := fun a b h =>
    Chain.mono hcov (h.extend' hext)
  have hmapR : ∀ l b, RootChain g L root fid gsid l b → RootChain g2 L' root fid gsid l b :=
    fun l b h => RootChain.mono hcov (h.extend' hext)
  refine {
    wf := Graph.addEdge_wf hadd hw1
    back := Graph.addEdge_back hadd hb1
    rootNone := by rw [hLx root (Ne.symm hcr)]; exact hc.rootNone
    rootMut := by rw [hmut2, Graph.newRef_mutability hnew root (Ne.symm hcr)]; exact hc.rootMut
    rootNotHeld := fun h => h.elim hc.rootNotHeld (fun e => hcr e.symm)
    keys := fun x hx => ?_
    held := fun x hx => ?_
    strict := fun q rq x rx hq hx hqL hxL hm hin => ?_
    same := fun q rq x rx hq hx hne hqL hxL hm heq => ?_
    immInside := fun q rq x rx hq hx hqL hxL hm hin => ?_
    rootLocal := fun x rx i hx hxL hr => ?_
    prov := fun x rx hx hxL => ?_ }
  · rw [Graph.isSome_of_mutability (hmut2 x)] at hx
    by_cases hxc : x = c
    · exact Or.inr (Or.inr hxc)
    · rw [Graph.isSome_of_mutability (Graph.newRef_mutability hnew x hxc)] at hx
      rcases hc.keys x hx with h | h
      · exact Or.inl h
      · exact Or.inr (Or.inl h)
  · rcases hx with hx | rfl
    · obtain ⟨r', hr, hm⟩ := hc.held x hx
      have hxc : x ≠ c := fun e => hcH (e ▸ hx)
      exact ⟨r', by rw [hLx x hxc]; exact hr, by
        rw [hmut2, Graph.newRef_mutability hnew x hxc]; exact hm⟩
    · exact ⟨r, hLc, by rw [hmut2, Graph.newRef_mutability_self hnew]⟩
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hmapC q x (hc.strict q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL; exact absurd hin (hnotin rq)
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      have := (hinR x rx hx (hHL x rx hx hxL) (by rw [hin.1.1, hloc])).2
      rw [hm] at this; cases this
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin (hnotin _)
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · rcases hc.same q rq x rx hq hx hne (hHL q rq hq hqL) (hHL x rx hx hxL) hm heq with h | h
      · exact Or.inl (hmapC q x h)
      · exact Or.inr (hmapC x q h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      have h := hinR q rq hq (hHL q rq hq hqL) (by rw [← heq, hloc])
      have := h.1 h.2; rw [hm] at this; cases this
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      have := (hinR x rx hx (hHL x rx hx hxL) (by rw [heq, hloc])).2
      rw [hm] at this; cases this
    · exact absurd rfl hne
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hc.immInside q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin
    · rw [hLc, Option.some.injEq] at hxL; subst hxL; exact absurd hin (hnotin rq)
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      exact (hinR x rx hx (hHL x rx hx hxL) (by rw [hin.1.1, hloc])).1 hm
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin (hnotin _)
  · rcases hx with hx | rfl
    · exact hmapR _ x (hc.rootLocal x rx i hx (hHL x rx hx hxL) hr)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL; rw [hloc] at hr; cases hr
  · rcases hx with hx | rfl
    · rcases hc.prov x rx hx (hHL x rx hx hxL) with h | ⟨d, h⟩ | h
      · exact Or.inl h
      · exact Or.inr (Or.inl ⟨d, hmapR _ x h⟩)
      · exact Or.inr (Or.inr h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      obtain ⟨e', he', hle⟩ := hnewE
      refine Or.inr (Or.inl ⟨d, e', x, r, he', RootChain.atLabel_cover (by simp [Graph.atLabel]) hle,
        hLc, TrueRootEdge.cover ?_ hle, Or.inl rfl⟩)
      simp [TrueRootEdge, hloc, hd, absPath]

end Complete

/-! ## What the checks guarantee -/

namespace Complete

variable {g : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
  {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}

/-- Nothing held is strictly inside a writable reference. -/
theorem writable_inside (hc : Complete g L H root fid gsid ps) {p : Nat} {rp : Reference}
    (hp : H p) (hpL : L p = some rp) (hw : g.isWritable p = .ok true) {x : Nat} {rx : Reference}
    (hx : H x) (hxL : L x = some rx) : ¬ rx.location.StrictlyInside rp.location := by
  intro hin
  obtain ⟨hpm, hnone⟩ := Graph.isWritable_true hw
  rw [hc.held_mut hp hpL, Option.some.injEq] at hpm
  obtain ⟨e, n, -, -, -, -, he, -⟩ := (hc.strict p rp x rx hp hx hpL hxL hpm hin).first
  exact hnone e n he

/-- Nothing mutable held is strictly inside a readable reference: reading
through it never observes, as part of a larger value, a location a held
`&mut` designates (D4b′). -/
theorem readable_inside (hc : Complete g L H root fid gsid ps) {p : Nat} {rp : Reference}
    (hp : H p) (hpL : L p = some rp) (hr : g.isReadable p none = .ok true) {x : Nat}
    {rx : Reference} (hx : H x) (hxL : L x = some rx) (hxm : rx.mutable = true) :
    ¬ rx.location.StrictlyInside rp.location := by
  intro hin
  cases hpm : rp.mutable with
  | false =>
    have := hc.immInside p rp x rx hp hx hpL hxL hpm hin
    rw [hxm] at this; cases this
  | true =>
    rcases Graph.isReadable_true hr with himm | hfz
    · rw [hc.held_mut hp hpL, hpm] at himm; cases himm
    · obtain ⟨e, n, ra, rn, ha, hn, he, -, hrest⟩ :=
        (hc.strict p rp x rx hp hx hpL hxL hpm hin).first
      have hnm : rn.mutable = true := by
        rcases hrest with rfl | hrest
        · rw [hn, Option.some.injEq] at hxL; subst hxL; exact hxm
        · exact hrest.mutable_src hn hxL hxm
      have hnr : n ≠ root := by rintro rfl; rw [hc.rootNone] at hn; cases hn
      have hnH : H n := hc.key_held (Graph.Edge.keys he).2 hnr
      have := hfz e n he (by cases hpe : e.path <;> simp [Graph.atLabel, hpe])
      rw [hc.held_mut hnH hn, hnm] at this; cases this

/-- Nothing held is into a local the frame root does not borrow. -/
theorem local_unborrowed (hc : Complete g L H root fid gsid ps) {i : Nat}
    (hb : g.hasConsistentBorrows root (some (.local_ i)) = .ok false) {x : Nat} {rx : Reference}
    (hx : H x) (hxL : L x = some rx) : rx.location.root ≠ .local_ fid i := by
  intro hr
  obtain ⟨e, n, rn, he, hl, -⟩ := hc.rootLocal x rx i hx hxL hr
  have := Graph.hasConsistentBorrows_false hb he
  rw [hl] at this; cases this

/-- Nothing held is into a resource of a type the frame root does not borrow,
when no reference parameter is into that type. -/
theorem global_unborrowed (hc : Complete g L H root fid gsid ps)
    (hginj : ∀ d₁ d₂ s, gsid d₁ = some s → gsid d₂ = some s → d₁ = d₂) {d : Nat}
    {sid : StructId} (hd : gsid d = some sid.erase)
    (hparams : ∀ π ∈ ps, ∀ a', π.location.root ≠ .global a' sid)
    (hb : g.hasConsistentBorrows root (some (.global d)) = .ok false) {x : Nat} {rx : Reference}
    (hx : H x) (hxL : L x = some rx) (a : Address) : rx.location.root ≠ .global a sid := by
  intro hr
  obtain ⟨e, n, rn, he, hl, -⟩ := hc.global_rootChain hginj hx hxL hr hd hparams
  have := Graph.hasConsistentBorrows_false hb he
  rw [hl] at this; cases this

end Complete

end MoveBytecode.Proofs
