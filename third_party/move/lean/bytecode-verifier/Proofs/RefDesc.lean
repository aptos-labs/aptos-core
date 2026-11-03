-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefCanon

/-!
# What the reference-safety analysis's states describe

UNTRUSTED (checked by the kernel). An abstract state of the reference-safety
analysis describes a frame (`Desc`) through a map `L` from its references to
the frame's concrete references: every reference a local or an operand slot
holds abstractly is a concrete reference there (operand slots counted from the
bottom), and the borrow graph is
complete for them (`Complete`). The analysis's order (`RefLe`) is inclusion
of what states describe; the join and the canonical form respect it.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

namespace Complete

variable {g g' : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
  {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}

/-- **A covering graph** with the same references and mutabilities is
complete for what the covered one is. -/
theorem transfer (hc : Complete g L H root fid gsid ps) (hw : g'.WF) (hb : g'.Back)
    (hkeys : ∀ x, (g'.find? x).isSome → x = root ∨ H x)
    (hmut : ∀ x, (x = root ∨ H x) → g'.mutability x = g.mutability x) (hcov : g.Covers g') :
    Complete g' L H root fid gsid ps where
  wf := hw
  back := hb
  rootNone := hc.rootNone
  rootMut := by rw [hmut _ (Or.inl rfl)]; exact hc.rootMut
  rootNotHeld := hc.rootNotHeld
  keys := hkeys
  held := fun x hx => by
    obtain ⟨r, hr, hm⟩ := hc.held x hx
    exact ⟨r, hr, by rw [hmut _ (Or.inr hx)]; exact hm⟩
  strict := fun p rp x rx hp hx hpL hxL hm hin =>
    Chain.mono hcov (hc.strict p rp x rx hp hx hpL hxL hm hin)
  same := fun p rp x rx hp hx hne hpL hxL hm heq =>
    (hc.same p rp x rx hp hx hne hpL hxL hm heq).imp (Chain.mono hcov) (Chain.mono hcov)
  immInside := hc.immInside
  rootLocal := fun x rx i hx hxL hroot =>
    RootChain.mono hcov (hc.rootLocal x rx i hx hxL hroot)
  prov := fun x rx hx hxL =>
    (hc.prov x rx hx hxL).imp id (Or.imp (fun ⟨d, h⟩ => ⟨d, RootChain.mono hcov h⟩) id)

end Complete

/-! ## Renaming -/

section Rename

variable {g g' : Graph} {L L' : Nat → Option Reference} {F : Nat → Nat}

theorem Chain.rename (hedge : ∀ p e c, g.Edge p e c → g'.Edge (F p) e (F c))
    (hL : ∀ x, (g.find? x).isSome → L' (F x) = L x) {a b : Nat} (h : Chain g L a b) :
    Chain g' L' (F a) (F b) := by
  induction h with
  | edge ha hb he ht =>
    exact .edge (by rw [hL _ (Graph.Edge.keys he).1]; exact ha)
      (by rw [hL _ (Graph.Edge.keys he).2]; exact hb) (hedge _ _ _ he) ht
  | snoc _ hb hc he ht ih =>
    exact .snoc ih (by rw [hL _ (Graph.Edge.keys he).1]; exact hb)
      (by rw [hL _ (Graph.Edge.keys he).2]; exact hc) (hedge _ _ _ he) ht

theorem RootChain.rename {root : Nat} {fid : FrameId} {gsid : Nat → Option StructId}
    {label? : Option Label} (hedge : ∀ p e c, g.Edge p e c → g'.Edge (F p) e (F c))
    (hL : ∀ x, (g.find? x).isSome → L' (F x) = L x) (hroot : F root = root) {b : Nat}
    (h : RootChain g L root fid gsid label? b) : RootChain g' L' root fid gsid label? (F b) := by
  obtain ⟨e, n, rn, he, hl, hn, ht, hrest⟩ := h
  refine ⟨e, F n, rn, hroot ▸ hedge _ _ _ he, hl,
    by rw [hL _ (Graph.Edge.keys he).2]; exact hn, ht, ?_⟩
  rcases hrest with rfl | hrest
  · exact Or.inl rfl
  · exact Or.inr (Chain.rename hedge hL hrest)

/-- **Renaming the references** keeps completeness. -/
theorem Complete.rename {H : Nat → Prop} {root : Nat} {fid : FrameId}
    {gsid : Nat → Option StructId} {ps : List Reference}
    (hc : Complete g L H root fid gsid ps) (hw : g'.WF) (hb : g'.Back)
    (hinj : ∀ x y, (g.find? x).isSome → (g.find? y).isSome → F x = F y → x = y)
    (hroot : F root = root)
    (hfind : ∀ y, (g'.find? y).isSome → ∃ x, (g.find? x).isSome ∧ y = F x)
    (hmut : ∀ x, (g.find? x).isSome → g'.mutability (F x) = g.mutability x)
    (hedge : ∀ p e c, g.Edge p e c → g'.Edge (F p) e (F c))
    (hL : ∀ x, (g.find? x).isSome → L' (F x) = L x) :
    Complete g' L' (fun y => ∃ x, H x ∧ F x = y) root fid gsid ps := by
  have hkeyH : ∀ x, H x → (g.find? x).isSome := fun x hx => by
    obtain ⟨r, -, hm⟩ := hc.held x hx
    simp only [Graph.mutability] at hm
    cases h : g.find? x <;> simp_all
  have hkeyR := hc.root_key
  have hgen : ∀ y, (∃ x, H x ∧ F x = y) → ∃ x, H x ∧ F x = y ∧ (g.find? x).isSome :=
    fun y ⟨x, hx, hxy⟩ => ⟨x, hx, hxy, hkeyH x hx⟩
  refine {
    wf := hw
    back := hb
    rootNone := by rw [← hroot, hL _ hkeyR]; exact hc.rootNone
    rootMut := by rw [← hroot, hmut _ hkeyR]; exact hc.rootMut
    rootNotHeld := fun ⟨x, hx, hxr⟩ => by
      have : x = root := hinj x root (hkeyH x hx) hkeyR (hxr.trans hroot.symm)
      subst this
      exact hc.rootNotHeld hx
    keys := fun y hy => by
      obtain ⟨x, hx, rfl⟩ := hfind y hy
      rcases hc.keys x hx with rfl | h
      · exact Or.inl hroot
      · exact Or.inr ⟨x, h, rfl⟩
    held := fun y hy => by
      obtain ⟨x, hx, rfl, hk⟩ := hgen y hy
      obtain ⟨r, hr, hm⟩ := hc.held x hx
      exact ⟨r, by rw [hL _ hk]; exact hr, by rw [hmut _ hk]; exact hm⟩
    strict := fun p rp x rx hp hx hpL hxL hm hin => by
      obtain ⟨p0, hp0, rfl, hkp⟩ := hgen p hp
      obtain ⟨x0, hx0, rfl, hkx⟩ := hgen x hx
      rw [hL _ hkp] at hpL
      rw [hL _ hkx] at hxL
      exact Chain.rename hedge hL (hc.strict p0 rp x0 rx hp0 hx0 hpL hxL hm hin)
    same := fun p rp x rx hp hx hne hpL hxL hm heq => by
      obtain ⟨p0, hp0, rfl, hkp⟩ := hgen p hp
      obtain ⟨x0, hx0, rfl, hkx⟩ := hgen x hx
      rw [hL _ hkp] at hpL
      rw [hL _ hkx] at hxL
      have hne0 : p0 ≠ x0 := fun e => hne (by rw [e])
      exact (hc.same p0 rp x0 rx hp0 hx0 hne0 hpL hxL hm heq).imp (Chain.rename hedge hL)
        (Chain.rename hedge hL)
    immInside := fun p rp x rx hp hx hpL hxL hm hin => by
      obtain ⟨p0, hp0, rfl, hkp⟩ := hgen p hp
      obtain ⟨x0, hx0, rfl, hkx⟩ := hgen x hx
      rw [hL _ hkp] at hpL
      rw [hL _ hkx] at hxL
      exact hc.immInside p0 rp x0 rx hp0 hx0 hpL hxL hm hin
    rootLocal := fun x rx i hx hxL hr => by
      obtain ⟨x0, hx0, rfl, hkx⟩ := hgen x hx
      rw [hL _ hkx] at hxL
      exact RootChain.rename hedge hL hroot (hc.rootLocal x0 rx i hx0 hxL hr)
    prov := fun x rx hx hxL => by
      obtain ⟨x0, hx0, rfl, hkx⟩ := hgen x hx
      rw [hL _ hkx] at hxL
      exact (hc.prov x0 rx hx0 hxL).imp id
        (Or.imp (fun ⟨d, h⟩ => ⟨d, RootChain.rename hedge hL hroot h⟩) id) }

end Rename

/-! ## Descriptions -/

/-- **An abstract state describes a frame**: every reference a local or an
operand slot holds abstractly is a concrete reference there, which `L` gives,
and the borrow graph is complete for them. Operand slots are counted from the
bottom of the stack, which steps on values leave alone. -/
structure Desc (A : RefState) (fr : Frame) (gsid : Nat → Option StructId) (ps : List Reference)
    (L : Nat → Option Reference) : Prop where
  locals : ∀ (i x : Nat), A.locals[i]? = some (AbsValue.reference x) →
    ∃ r, fr.locals[i]? = some (some (.ref r)) ∧ L x = some r
  stack : ∀ (k x : Nat), A.stack.reverse[k]? = some (AbsValue.reference x) →
    ∃ r, fr.stack.reverse[k]? = some (.ref r) ∧ L x = some r
  complete : Complete A.graph L (· ∈ heldIds A) A.locals.length fr.id gsid ps

/-- **The analysis's order**: what `a` describes, `b` describes, through
references `a` holds. -/
def RefLe (a b : RefState) : Prop :=
  ∀ fr gsid ps L, Desc a fr gsid ps L → ∃ L', Desc b fr gsid ps L' ∧
    ∀ x r, x ∈ heldIds b → L' x = some r → ∃ y, y ∈ heldIds a ∧ L y = some r

theorem RefLe.refl (a : RefState) : RefLe a a :=
  fun _ _ _ L hd => ⟨L, hd, fun x _ hx hr => ⟨x, hx, hr⟩⟩

theorem RefLe.trans {a b c : RefState} (h1 : RefLe a b) (h2 : RefLe b c) : RefLe a c := by
  intro fr gsid ps L hd
  obtain ⟨L1, hd1, hs1⟩ := h1 fr gsid ps L hd
  obtain ⟨L2, hd2, hs2⟩ := h2 fr gsid ps L1 hd1
  refine ⟨L2, hd2, fun x r hx hr => ?_⟩
  obtain ⟨y, hy, hyr⟩ := hs2 x r hx hr
  exact hs1 y r hy hyr

theorem Desc.ext {A A' : RefState} {fr : Frame} {gsid : Nat → Option StructId}
    {ps : List Reference} {L : Nat → Option Reference} (hd : Desc A fr gsid ps L)
    (hl : A'.locals = A.locals) (hg : A'.graph = A.graph) (hs : A'.stack = A.stack) :
    Desc A' fr gsid ps L := by
  obtain ⟨l, g, n, s⟩ := A
  obtain ⟨l', g', n', s'⟩ := A'
  simp only at hl hg hs
  subst hl hg hs
  exact ⟨hd.locals, hd.stack, hd.complete⟩

/-- **Dropping a local's reference**, as the join does. -/
theorem Desc.dropLocal {ltys : List SignatureToken} {avail : LocalIndex → Availability}
    {A : RefState} {fr : Frame} {gsid : Nat → Option StructId} {ps : List Reference}
    {L : Nat → Option Reference} (hd : Desc A fr gsid ps L) (ht : Typed ltys [] avail A)
    {i x : Nat} {g : Graph} (hi : A.locals[i]? = some (AbsValue.reference x))
    (hrel : A.graph.release x = .ok g) :
    Desc { A with locals := A.locals.set i .nonReference, graph := g } fr gsid ps L := by
  have hx : x ∈ heldIds A := mem_heldIds.2 (Or.inl (List.mem_of_getElem? hi))
  have hc := hd.complete.release hx hrel
  have hperm : List.Perm
      (heldIds { A with locals := A.locals.set i .nonReference, graph := g } ++ [x])
      (heldIds A) := by
    refine perm_of_count fun a => ?_
    have := count_filterMap_set hi .nonReference a
    simp only [AbsValue.refId?, Option.toList_some, Option.toList_none, List.count_nil] at this
    simp only [heldIds, List.count_append]
    omega
  have hnd : (heldIds { A with locals := A.locals.set i .nonReference, graph := g } ++ [x]).Nodup :=
    hperm.nodup_iff.2 ht.distinct
  refine ⟨fun j y hj => ?_, hd.stack, ?_⟩
  · simp only [List.getElem?_set] at hj
    split at hj
    · split at hj <;> cases hj
    · exact hd.locals j y hj
  · simp only [List.length_set]
    refine hc.congrH fun y => ⟨fun ⟨hy, hyx⟩ => ?_, fun hy => ⟨?_, ?_⟩⟩
    · rcases List.mem_append.1 (hperm.mem_iff.2 hy) with h | h
      · exact h
      · simp at h; exact absurd h hyx
    · exact hperm.mem_iff.1 (List.mem_append_left _ hy)
    · intro e
      subst e
      exact (List.nodup_append.1 hnd).2.2 y hy y (by simp) rfl

/-- The join's pass over the locals keeps what each side describes, with the
locals it builds, whose references each side holds. -/
theorem joinLocals_desc {ltys : List SignatureToken} {avail : LocalIndex → Availability}
    {a b : RefState} {sg og : Graph} {ls : List AbsValue}
    (ha : Typed ltys [] avail a) (hb : Typed ltys [] avail b)
    (hlen : a.locals.length = b.locals.length)
    (h : (a.locals.zip b.locals).foldlM
      (fun (sg, og, ls) (v1, v2) => match v1, v2 with
        | .reference id, .nonReference => do pure (← sg.release id, og, ls ++ [AbsValue.nonReference])
        | .nonReference, .reference id => do pure (sg, ← og.release id, ls ++ [AbsValue.nonReference])
        | v1, v2 =>
          if v1 == v2 then pure (sg, og, ls ++ [v1]) else panicAt "join: different references")
      (a.graph, b.graph, []) = .ok (sg, og, ls)) :
    (∀ fr gsid ps L, Desc a fr gsid ps L →
      Desc { a with locals := ls, graph := sg } fr gsid ps L) ∧
    (∀ fr gsid ps L, Desc b fr gsid ps L →
      Desc { b with locals := ls, graph := og } fr gsid ps L) ∧
    (∀ x, x ∈ ls.filterMap AbsValue.refId? → x ∈ a.locals.filterMap AbsValue.refId?) ∧
    (∀ x, x ∈ ls.filterMap AbsValue.refId? → x ∈ b.locals.filterMap AbsValue.refId?) := by
  let I : Graph × Graph × List AbsValue → List (AbsValue × AbsValue) → Prop :=
    fun acc rest => ∃ ra rb, rest = ra.zip rb ∧ ra.length = rb.length ∧
      Typed ltys [] avail { a with locals := acc.2.2 ++ ra, graph := acc.1 } ∧
      Typed ltys [] avail { b with locals := acc.2.2 ++ rb, graph := acc.2.1 } ∧
      (∀ fr gsid ps L, Desc a fr gsid ps L →
        Desc { a with locals := acc.2.2 ++ ra, graph := acc.1 } fr gsid ps L) ∧
      (∀ fr gsid ps L, Desc b fr gsid ps L →
        Desc { b with locals := acc.2.2 ++ rb, graph := acc.2.1 } fr gsid ps L) ∧
      (∀ x, x ∈ (acc.2.2 ++ ra).filterMap AbsValue.refId? →
        x ∈ a.locals.filterMap AbsValue.refId?) ∧
      (∀ x, x ∈ (acc.2.2 ++ rb).filterMap AbsValue.refId? →
        x ∈ b.locals.filterMap AbsValue.refId?)
  have hI : I (a.graph, b.graph, []) (a.locals.zip b.locals) :=
    ⟨a.locals, b.locals, rfl, hlen, by simpa using ha.ext rfl rfl rfl,
      by simpa using hb.ext rfl rfl rfl,
      fun _ _ _ _ hd => hd.ext (by simp) rfl rfl, fun _ _ _ _ hd => hd.ext (by simp) rfl rfl,
      fun x hx => by simpa using hx, fun x hx => by simpa using hx⟩
  have hfin := foldlM_invariant I (fun acc vv rest acc' hI hf => ?_) _ _ _ hI h
  · obtain ⟨ra, rb, hz, hl, -, -, hd1, hd2, hs1, hs2⟩ := hfin
    have hra : ra = [] := by
      cases ra with
      | nil => rfl
      | cons x xs =>
        cases rb with
        | nil => simp at hl
        | cons y ys => simp at hz
    subst hra
    have hrb : rb = [] := List.eq_nil_of_length_eq_zero (by simpa using hl.symm)
    subst hrb
    simp only [List.append_nil] at hd1 hd2 hs1 hs2
    exact ⟨hd1, hd2, hs1, hs2⟩
  · obtain ⟨sg, og, ls⟩ := acc
    obtain ⟨v1, v2⟩ := vv
    obtain ⟨ra, rb, hz, hl, h1, h2, hd1, hd2, hs1, hs2⟩ := hI
    obtain ⟨a', b', ra', rb', rfl, rfl, hx, rfl⟩ := zip_cons_eq hz.symm
    simp only [Prod.mk.injEq] at hx
    obtain ⟨rfl, rfl⟩ := hx
    simp only [List.length_cons, Nat.add_right_cancel_iff] at hl
    have hi1 : (ls ++ v1 :: ra')[ls.length]? = some v1 := by simp
    have hi2 : (ls ++ v2 :: rb')[ls.length]? = some v2 := by simp
    cases v1 with
    | reference x =>
      cases v2 with
      | nonReference =>
        simp only [bind_ok] at hf
        obtain ⟨g, hrel, hf⟩ := hf
        simp only [pure, Except.pure, Except.ok.injEq] at hf
        subst hf
        have hav : ∀ t bb, ltys[ls.length]? = some t → refKind t = some bb →
            avail ls.length ≠ .available := fun t bb ht hb' hav' => by
          obtain ⟨y, hy⟩ := h2.locKind _ t bb ht hb' hav'
          rw [show ({ b with locals := ls ++ AbsValue.nonReference :: rb', graph := og } :
            RefState).locals = ls ++ AbsValue.nonReference :: rb' from rfl] at hy
          simp at hy
        refine ⟨ra', rb', rfl, hl, (h1.dropLocal (i := ls.length) (x := x) (g := g) hi1 hrel
          hav).ext (by simp) rfl rfl, h2.ext (by simp) rfl rfl,
          fun fr gsid ps L hd => ((hd1 fr gsid ps L hd).dropLocal h1 (i := ls.length) hi1
            hrel).ext (by simp) rfl rfl,
          fun fr gsid ps L hd => (hd2 fr gsid ps L hd).ext (by simp) rfl rfl,
          fun y hy => hs1 y ?_, fun y hy => hs2 y (by simpa using hy)⟩
        simp only [List.append_assoc, List.singleton_append, List.filterMap_append,
          List.filterMap_cons, AbsValue.refId?, List.mem_append, List.mem_cons] at hy ⊢
        rcases hy with hy | hy
        · exact Or.inl hy
        · exact Or.inr (Or.inr hy)
      | reference y =>
        simp only at hf
        split at hf
        · next heq =>
          simp only [pure, Except.pure, Except.ok.injEq] at hf
          subst hf
          simp only [beq_iff_eq, AbsValue.reference.injEq] at heq
          subst heq
          exact ⟨ra', rb', rfl, hl, h1.ext (by simp) rfl rfl, h2.ext (by simp) rfl rfl,
            fun fr gsid ps L hd => (hd1 fr gsid ps L hd).ext (by simp) rfl rfl,
            fun fr gsid ps L hd => (hd2 fr gsid ps L hd).ext (by simp) rfl rfl,
            fun z hz => hs1 z (by simpa using hz), fun z hz => hs2 z (by simpa using hz)⟩
        · simp [panicAt] at hf
    | nonReference =>
      cases v2 with
      | reference y =>
        simp only [bind_ok] at hf
        obtain ⟨g, hrel, hf⟩ := hf
        simp only [pure, Except.pure, Except.ok.injEq] at hf
        subst hf
        have hav : ∀ t bb, ltys[ls.length]? = some t → refKind t = some bb →
            avail ls.length ≠ .available := fun t bb ht hb' hav' => by
          obtain ⟨z, hz'⟩ := h1.locKind _ t bb ht hb' hav'
          rw [show ({ a with locals := ls ++ AbsValue.nonReference :: ra', graph := sg } :
            RefState).locals = ls ++ AbsValue.nonReference :: ra' from rfl] at hz'
          simp at hz'
        refine ⟨ra', rb', rfl, hl, h1.ext (by simp) rfl rfl,
          (h2.dropLocal (i := ls.length) (x := y) (g := g) hi2 hrel hav).ext (by simp) rfl rfl,
          fun fr gsid ps L hd => (hd1 fr gsid ps L hd).ext (by simp) rfl rfl,
          fun fr gsid ps L hd => ((hd2 fr gsid ps L hd).dropLocal h2 (i := ls.length) hi2
            hrel).ext (by simp) rfl rfl,
          fun z hz => hs1 z (by simpa using hz), fun z hz => hs2 z ?_⟩
        simp only [List.append_assoc, List.singleton_append, List.filterMap_append,
          List.filterMap_cons, AbsValue.refId?, List.mem_append, List.mem_cons] at hz ⊢
        rcases hz with hz | hz
        · exact Or.inl hz
        · exact Or.inr (Or.inr hz)
      | nonReference =>
        simp only at hf
        split at hf
        · simp only [pure, Except.pure, Except.ok.injEq] at hf
          subst hf
          exact ⟨ra', rb', rfl, hl, h1.ext (by simp) rfl rfl, h2.ext (by simp) rfl rfl,
            fun fr gsid ps L hd => (hd1 fr gsid ps L hd).ext (by simp) rfl rfl,
            fun fr gsid ps L hd => (hd2 fr gsid ps L hd).ext (by simp) rfl rfl,
            fun z hz => hs1 z (by simpa using hz), fun z hz => hs2 z (by simpa using hz)⟩
        · simp [panicAt] at hf

/-- The edges a graph's join takes from the other side cover it. -/
theorem join_covers_other {self other j : Graph} (h : self.join other = .ok j) (hw : self.WF)
    (hb : self.Back) : other.Covers j := by
  obtain ⟨-, -, -, -, hother⟩ := Graph.join_ok h hw hb
  intro p e c _ he
  obtain ⟨pi, s, ci, hp, hs, he', -, -⟩ := he
  exact hother p pi c s e hp hs he'

/-- **The join is sound**: what either side describes, the join describes;
an unchanged join is the old state. -/
theorem join_sound {ltys : List SignatureToken} {avail : LocalIndex → Availability}
    {a b j : RefState} {changed : Bool} (ha : Typed ltys [] avail a) (hb : Typed ltys [] avail b)
    (h : a.join b = .ok (j, changed)) : RefLe a j ∧ RefLe b j ∧ (changed = false → j = a) := by
  obtain ⟨hlen, -, sg, og, ls, jg, hfold, hjg, hres⟩ := join_run h
  obtain ⟨t1, t2⟩ := joinLocals_typed ha hb hlen hfold
  obtain ⟨d1, d2, s1, s2⟩ := joinLocals_desc ha hb hlen hfold
  obtain ⟨hw, hbk, hcov, hmut, -⟩ := Graph.join_ok hjg t1.wf t1.back
  have hcovO := join_covers_other hjg t1.wf t1.back
  have has : a.stack = [] := List.eq_nil_of_length_eq_zero (by simpa using ha.nstack)
  have hbs : b.stack = [] := List.eq_nil_of_length_eq_zero (by simpa using hb.nstack)
  -- The joined state.
  let J : RefState := { a with locals := ls, graph := jg }
  have hheldJ : ∀ x, x ∈ heldIds J ↔ x ∈ ls.filterMap AbsValue.refId? := by
    intro x; simp [J, heldIds, has]
  have hheldB : ∀ x, x ∈ heldIds ({ b with locals := ls, graph := og } : RefState) ↔
      x ∈ heldIds J := by
    intro x; simp [J, heldIds, has, hbs]
  have hkeysJ : ∀ x, (jg.find? x).isSome → x = ls.length ∨ x ∈ heldIds J := by
    intro x hx
    rw [Graph.isSome_of_mutability (hmut x)] at hx
    exact t1.keys x hx
  -- Both sides agree on the mutability of the joined state's references.
  have hmutSO : ∀ x, (x = ls.length ∨ x ∈ heldIds J) → sg.mutability x = og.mutability x := by
    intro x hx
    rcases hx with rfl | hx
    · rw [show sg.mutability ls.length = some true from t1.rootMut,
        show og.mutability ls.length = some true from t2.rootMut]
    · rw [hheldJ] at hx
      obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 (Typed.mem_filterMap_refId.1 hx)
      obtain ⟨t, b1, e1, e2, e3⟩ := t1.locMut i x hi
      obtain ⟨t', b2, e1', e2', e3'⟩ := t2.locMut i x hi
      rw [e1, Option.some.injEq] at e1'
      subst e1'
      rw [e2, Option.some.injEq] at e2'
      subst e2'
      exact e3.trans e3'.symm
  have hselfJ : RefLe a J := by
    intro fr gsid ps L hd
    have hd1 := d1 fr gsid ps L hd
    refine ⟨L, ⟨hd1.locals, hd1.stack, hd1.complete.transfer hw hbk hkeysJ
      (fun x _ => hmut x) hcov⟩, fun x r hx hr => ⟨x, ?_, hr⟩⟩
    have := s1 x ((hheldJ x).1 hx)
    exact mem_heldIds.2 (Or.inl (Typed.mem_filterMap_refId.1 this))
  have hotherJ : RefLe b J := by
    intro fr gsid ps L hd
    have hd2 := d2 fr gsid ps L hd
    have hc2 := (hd2.complete.congrH hheldB)
    refine ⟨L, ⟨hd2.locals, fun k x hk => by simp [J, has] at hk, ?_⟩,
      fun x r hx hr => ⟨x, ?_, hr⟩⟩
    · exact hc2.transfer hw hbk hkeysJ (fun x hx => (hmut x).trans (hmutSO x hx)) hcovO
    · have := s2 x ((hheldJ x).1 hx)
      exact mem_heldIds.2 (Or.inl (Typed.mem_filterMap_refId.1 this))
  rcases hres with ⟨rfl, rfl, hloc, hleq⟩ | ⟨rfl, rfl⟩
  · -- Unchanged: the old state covers the join.
    have hcovA : jg.Covers j.graph := by
      intro p e c _ he
      obtain ⟨pi, s, ci, hp, hs, he', -, -⟩ := he
      exact (Graph.leq_ok hleq p pi c s e hp hs he').hasEdge ha.back
    have hJa : RefLe J j := by
      intro fr gsid ps L hd
      refine ⟨L, ⟨fun i x hi => hd.locals i x (by simpa [J, hloc] using hi), hd.stack, ?_⟩,
        fun x r hx hr => ⟨x, by simpa [J, heldIds, hloc] using hx, hr⟩⟩
      have hc := hd.complete
      have hJH : ∀ x, x ∈ heldIds J ↔ x ∈ heldIds j := by
        intro x; simp [J, heldIds, hloc]
      have hlenJ : J.locals.length = j.locals.length := by simp [J, hloc]
      rw [hlenJ] at hc
      refine (hc.congrH hJH).transfer ha.wf ha.back (fun x hx => ha.keys x hx) ?_ hcovA
      intro x hx
      rw [show J.graph = jg from rfl, hmut]
      rcases hx with hx | hx
      · rw [hx]
        rw [show j.graph.mutability j.locals.length = some true from ha.rootMut]
        have := t1.rootMut
        simp only [hloc] at this ⊢
        exact this.symm
      · obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 (Typed.mem_filterMap_refId.1
          (by simpa [heldIds, has] using hx))
        obtain ⟨t, b1, e1, e2, e3⟩ := ha.locMut i x hi
        rw [hloc] at hi
        obtain ⟨t', b2, e1', e2', e3'⟩ := t1.locMut i x hi
        rw [e1, Option.some.injEq] at e1'
        subst e1'
        rw [e2, Option.some.injEq] at e2'
        subst e2'
        exact e3.trans e3'.symm
    exact ⟨RefLe.refl _, hotherJ.trans hJa, fun _ => rfl⟩
  · exact ⟨hselfJ, hotherJ, fun h => absurd h (by simp)⟩

/-- **Canonical form is sound**: a state and its canonical form describe the
same frames. -/
theorem canon_le {ltys : List SignatureToken} {avail : LocalIndex → Availability} {st : RefState}
    (ht : Typed ltys [] avail st) : RefLe st (canonState st) := by
  have hs : st.stack = [] := List.eq_nil_of_length_eq_zero (by simpa using ht.nstack)
  have hnd : (st.locals.filterMap AbsValue.refId?).Nodup := by
    have := ht.distinct
    simpa [heldIds, hs] using this
  generalize hF : (fun id => ((canonMap st.locals).find? id).getD id) = F
  have hFx : ∀ x, ((canonMap st.locals).find? x).getD x = F x := fun x => by rw [← hF]
  have hFref : ∀ (i x : Nat), st.locals[i]? = some (AbsValue.reference x) → F x = i :=
    fun i x hi => by rw [← hFx, canonMap_ref hnd hi]; rfl
  have hroot : ∀ i : Nat, st.locals[i]? ≠ some (AbsValue.reference st.locals.length) :=
    fun i hi => ht.rootFree ((ht.held_local).2 ⟨i, hi⟩)
  have hFroot : F st.locals.length = st.locals.length := by
    rw [← hFx, canonMap_root hroot]; rfl
  have hkey : ∀ x, (st.graph.find? x).isSome →
      x = st.locals.length ∨ ∃ i : Nat, st.locals[i]? = some (AbsValue.reference x) :=
    fun x hx => (ht.keys x hx).imp id (ht.held_local).1
  have hlt : ∀ (i x : Nat), st.locals[i]? = some (AbsValue.reference x) → i < st.locals.length :=
    fun i x hi => (List.getElem?_eq_some_iff.1 hi).1
  have hinj : ∀ x y, (st.graph.find? x).isSome → (st.graph.find? y).isSome → F x = F y →
      x = y := by
    intro x y hx hy hxy
    rcases hkey x hx with rfl | ⟨i, hi⟩ <;> rcases hkey y hy with rfl | ⟨j, hj⟩
    · rfl
    · rw [hFroot, hFref j y hj] at hxy; have := hlt j y hj; omega
    · rw [hFroot, hFref i x hi] at hxy; have := hlt i x hi; omega
    · rw [hFref i x hi, hFref j y hj] at hxy
      subst hxy
      rw [hi, Option.some.injEq, AbsValue.reference.injEq] at hj
      exact hj
  obtain ⟨h1, h2, h3, hw, hb⟩ := Graph.remap_ok ht.wf ht.back (fun x y xi yi hx hy hxy => by
    rw [hFx, hFx] at hxy
    exact hinj x y (by simp [hx]) (by simp [hy]) hxy)
  simp only [hFx] at h1 h2 h3
  have hheld' : ∀ y, y ∈ heldIds (canonState st) ↔
      ∃ old, st.locals[y]? = some (AbsValue.reference old) := by
    intro y
    rw [mem_heldIds]
    simp only [canonState, hs, List.not_mem_nil, or_false]
    rw [List.mem_iff_getElem?]
    constructor
    · rintro ⟨i, hi⟩
      obtain ⟨rfl, h⟩ := canonLocals_ref.1 hi
      exact h
    · intro h
      exact ⟨y, canonLocals_ref.2 ⟨rfl, h⟩⟩
  intro fr gsid ps L hd
  let L' : Nat → Option Reference := fun y => match st.locals[y]? with
    | some (AbsValue.reference x) => L x
    | _ => none
  have hL'i : ∀ (i x : Nat), st.locals[i]? = some (AbsValue.reference x) → L' i = L x :=
    fun i x hi => by simp only [L', hi]
  have hL : ∀ x, (st.graph.find? x).isSome → L' (F x) = L x := by
    intro x hx
    rcases hkey x hx with rfl | ⟨i, hi⟩
    · rw [hFroot, hd.complete.rootNone]
      simp only [L']
      rw [List.getElem?_eq_none (Nat.le_refl _)]
    · rw [hFref i x hi, hL'i i x hi]
  have hc := hd.complete.rename (F := F) (L' := L') hw hb hinj hFroot
    (fun y hy => by
      obtain ⟨v, hv⟩ := Option.isSome_iff_exists.1 hy
      obtain ⟨x, xi, hxi, rfl, -⟩ := h2 y v hv
      exact ⟨x, by simp [hxi], rfl⟩)
    (fun x hx => by
      obtain ⟨xi, hxi⟩ := Option.isSome_iff_exists.1 hx
      simp [Graph.mutability, h1 x xi hxi, hxi, Graph.renameInfo])
    h3 hL
  refine ⟨L', ⟨fun i y hi => ?_, fun k y hk => by simp [canonState, hs] at hk, ?_⟩,
    fun y r hy hr => ?_⟩
  · obtain ⟨rfl, old, hold⟩ := canonLocals_ref.1 hi
    obtain ⟨r, hr, hLr⟩ := hd.locals _ old hold
    exact ⟨r, hr, by rw [hL'i _ old hold]; exact hLr⟩
  · simp only [canonState, canonLocals_length]
    refine hc.congrH fun y => ⟨fun ⟨x, hx, hxy⟩ => ?_, fun hy => ?_⟩
    · obtain ⟨i, hi⟩ := (ht.held_local).1 hx
      rw [hFref i x hi] at hxy
      subst hxy
      exact (hheld' i).2 ⟨x, hi⟩
    · obtain ⟨old, hold⟩ := (hheld' y).1 hy
      exact ⟨old, (ht.held_local).2 ⟨y, hold⟩, hFref y old hold⟩
  · obtain ⟨old, hold⟩ := (hheld' y).1 hy
    rw [hL'i y old hold] at hr
    exact ⟨old, (ht.held_local).2 ⟨y, hold⟩, hr⟩

end MoveBytecode.Proofs
