-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefTrace

/-!
# D2 and D3 for calls of the modeled natives: the parts

UNTRUSTED (checked by the kernel). The facts `Proofs.ModeledAnnotated`
assembles into the annotation of a call of a modeled native
(`Program.nativeShape?`) and its `StepGhost`. The nodes the native sees of
each argument are of three kinds (`ownedPos`, `refLeaves`):

- a value argument's, and what a mutable reference argument it writes back
  designates, are the native's: each is passed on, or consumed;
- what any other reference argument designates stays where it is
  (`sharedAt`): a node of it passed on is a copy;
- a reference argument itself is dropped.

The effect of the call follows from these positions (`native_effectK`). The
locations written back overlap no other argument's (`exclusive_mem`), so the
occurrences of the state split into what is written back and a rest that the
writes keep, and the nodes of what the other reference arguments designate
are in that rest (`writes_perm`). The arguments' occurrences and what is
written back are the reference arguments' own and the native's nodes
(`pre_split`); its consumptions are its nodes it does not pass on
(`consumedOfNative_keys`).
-/

namespace MoveBytecode.Proofs

/-! ## Trees at locations that do not overlap -/

theorem index_inj {v c₁ c₂ : Value} {st₁ st₂ : PathStep} (h₁ : v.child? st₁ = some c₁)
    (h₂ : v.child? st₂ = some c₂) (hi : v.childIndex st₁ = v.childIndex st₂) : st₁ = st₂ := by
  cases v <;> cases st₁ <;> cases st₂ <;>
    simp_all [Value.child?, PathStep.index, Value.childIndex]
  all_goals first
    | (split at h₁ <;> split at h₂ <;> simp_all)
    | (have := (List.getElem?_eq_some_iff.1 h₁).1; omega)
    | (have := (List.getElem?_eq_some_iff.1 h₂).1; omega)

/-- Paths readable in one value, neither a prefix of the other, have node
indices neither a prefix of the other. -/
theorem indices_incomparable : ∀ (p₁ p₂ : List PathStep) {v w₁ w₂ : Value},
    v.get? p₁ = some w₁ → v.get? p₂ = some w₂ → ¬ p₁ <+: p₂ → ¬ p₂ <+: p₁ →
    ¬ (v.indicesOf p₁) <+: (v.indicesOf p₂) ∧ ¬ (v.indicesOf p₂) <+: (v.indicesOf p₁)
  | [], _, _, _, _, _, _, h₁, _ => absurd List.nil_prefix h₁
  | _ :: _, [], _, _, _, _, _, _, h₂ => absurd List.nil_prefix h₂
  | s₁ :: p₁, s₂ :: p₂, v, w₁, w₂, hg₁, hg₂, hn₁, hn₂ => by
    obtain ⟨c₁, hc₁, hg₁⟩ := get_cons hg₁
    obtain ⟨c₂, hc₂, hg₂⟩ := get_cons hg₂
    by_cases hs : s₁ = s₂
    · subst hs
      rw [hc₁, Option.some.injEq] at hc₂
      subst hc₂
      have := indices_incomparable p₁ p₂ hg₁ hg₂ (fun h => hn₁ ((List.prefix_cons_inj _).2 h))
        (fun h => hn₂ ((List.prefix_cons_inj _).2 h))
      simp only [indicesOf_child hc₁, List.prefix_cons_inj]
      exact this
    · have hi : v.childIndex s₁ ≠ v.childIndex s₂ := fun e => hs (index_inj hc₁ hc₂ e)
      simp only [indicesOf_child hc₁, indicesOf_child hc₂, List.cons_prefix_cons, hi, false_and,
        not_false_eq_true, and_self, Ne.symm hi]

/-- Replacing a tree's node at a path leaves the nodes at paths neither a
prefix of it nor extending it. -/
theorem at_set_disjoint : ∀ (q₁ q₂ : List Nat) {t x t' : IdTree}, t.set? q₁ x = some t' →
    ¬ q₁ <+: q₂ → ¬ q₂ <+: q₁ → t'.at? q₂ = t.at? q₂
  | [], _, _, _, _, _, h₁, _ => absurd List.nil_prefix h₁
  | _ :: _, [], _, _, _, _, _, h₂ => absurd List.nil_prefix h₂
  | i :: q₁, j :: q₂, t, x, t', hs, h₁, h₂ => by
    obtain ⟨id, cs⟩ := t
    simp only [IdTree.set?, IdTree.children, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at hs
    obtain ⟨c, hc, c', hc', rfl⟩ := hs
    simp only [IdTree.at?, IdTree.children]
    by_cases hij : i = j
    · subst hij
      have hlt : i < cs.length := (List.getElem?_eq_some_iff.1 hc).1
      rw [List.getElem?_set_self hlt, hc]
      exact at_set_disjoint q₁ q₂ hc' (fun h => h₁ ((List.prefix_cons_inj _).2 h))
        (fun h => h₂ ((List.prefix_cons_inj _).2 h))
    · rw [List.getElem?_set_ne hij]

theorem framePos_inj {s : State} {f₁ f₂ : FrameId} {k : Nat} (h₁ : Shadow.framePos s f₁ = some k)
    (h₂ : Shadow.framePos s f₂ = some k) : f₁ = f₂ := by
  simp only [Shadow.framePos] at h₁ h₂
  have e₁ := List.findIdx?_eq_some_iff_getElem.1 h₁
  have e₂ := List.findIdx?_eq_some_iff_getElem.1 h₂
  obtain ⟨hk, he₁, -⟩ := e₁
  obtain ⟨hk', he₂, -⟩ := e₂
  simp only [beq_iff_eq] at he₁ he₂
  rw [← he₁, ← he₂]

theorem resourcePos_inj {s : State} {a₁ a₂ : Address} {t₁ t₂ : StructId} {k : Nat}
    (h₁ : Shadow.resourcePos s a₁ t₁ = some k) (h₂ : Shadow.resourcePos s a₂ t₂ = some k) :
    a₁ = a₂ ∧ t₁ = t₂ := by
  simp only [Shadow.resourcePos] at h₁ h₂
  obtain ⟨hk, he₁, -⟩ := List.findIdx?_eq_some_iff_getElem.1 h₁
  obtain ⟨hk', he₂, -⟩ := List.findIdx?_eq_some_iff_getElem.1 h₂
  simp only [Bool.and_eq_true, beq_iff_eq] at he₁ he₂
  exact ⟨he₁.1.symm.trans he₂.1, he₁.2.symm.trans he₂.2⟩

/-- Replacing the tree at one root leaves the trees at the others. -/
theorem rootTree_withRootTree_other {h h' : Shadow} {s : State} {ρ₁ ρ₂ : Root} {t : IdTree}
    (hw : h.withRootTree? s ρ₁ t = some h') (hne : ρ₁ ≠ ρ₂) :
    h'.rootTree? s ρ₂ = h.rootTree? s ρ₂ := by
  cases ρ₁ with
  | local_ f₁ i₁ =>
    simp only [Shadow.withRootTree?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hw
    obtain ⟨k₁, hk₁, g, hg, _, -, rfl⟩ := hw
    cases ρ₂ with
    | local_ f₂ i₂ =>
      simp only [Shadow.rootTree?]
      cases hk₂ : Shadow.framePos s f₂ with
      | none => rfl
      | some k₂ =>
        simp only [bind, Option.bind_some]
        rw [List.getElem?_set]
        by_cases hkk : k₁ = k₂
        · subst hkk
          have hff := framePos_inj hk₁ hk₂
          subst hff
          have hii : i₁ ≠ i₂ := fun e => hne (by rw [e])
          rw [if_pos rfl, if_pos (List.getElem?_eq_some_iff.1 hg).1, hg]
          simp only [Option.bind_some]
          rw [Array.getElem?_setIfInBounds_ne hii]
        · rw [if_neg hkk]
    | global a t => rfl
    | held k => rfl
  | global a₁ t₁ =>
    simp only [Shadow.withRootTree?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hw
    obtain ⟨k₁, hk₁, _, -, rfl⟩ := hw
    cases ρ₂ with
    | global a₂ t₂ =>
      simp only [Shadow.rootTree?]
      cases hk₂ : Shadow.resourcePos s a₂ t₂ with
      | none => rfl
      | some k₂ =>
        simp only [bind, Option.bind_some]
        rw [List.getElem?_set]
        by_cases hkk : k₁ = k₂
        · subst hkk
          obtain ⟨rfl, rfl⟩ := resourcePos_inj hk₁ hk₂
          exact absurd rfl hne
        · rw [if_neg hkk]
    | local_ f i => rfl
    | held k => rfl
  | held k₁ =>
    simp only [Shadow.withRootTree?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hw
    obtain ⟨_, -, rfl⟩ := hw
    cases ρ₂ with
    | held k₂ =>
      have hkk : k₁ ≠ k₂ := fun e => hne (by rw [e])
      simp only [Shadow.rootTree?]
      rw [List.getElem?_set_ne hkk]
    | local_ f i => rfl
    | global a t => rfl

theorem rootTree_withRootTree_self {h h' : Shadow} {s : State} {ρ : Root} {t : IdTree}
    (hw : h.withRootTree? s ρ t = some h') : h'.rootTree? s ρ = some t := by
  cases ρ with
  | local_ f i =>
    simp only [Shadow.withRootTree?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hw
    obtain ⟨k, hk, g, hg, o, ho, rfl⟩ := hw
    have hkl : k < h.frames.length := (List.getElem?_eq_some_iff.1 hg).1
    have hil : i < g.locals.size := (Array.getElem?_eq_some_iff.1 ho).1
    simp [Shadow.rootTree?, hk, hkl, hil]
  | global a ty =>
    simp only [Shadow.withRootTree?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hw
    obtain ⟨k, hk, o, ho, rfl⟩ := hw
    have hkl : k < h.storage.length := (List.getElem?_eq_some_iff.1 ho).1
    simp [Shadow.rootTree?, hk, hkl]
  | held k =>
    simp only [Shadow.withRootTree?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hw
    obtain ⟨o, ho, rfl⟩ := hw
    have hkl : k < h.held.length := (List.getElem?_eq_some_iff.1 ho).1
    simp [Shadow.rootTree?, hkl]

/-- **Writing a tree at a location leaves the trees at the locations it does
not overlap.** -/
theorem tree_withTree_disjoint {h h' : Shadow} {s : State} {l₁ l₂ : Location} {t : IdTree}
    (hw : h.withTree? s l₁ t = some h') (hno : l₁.overlaps l₂ = false)
    {w₁ w₂ : Value} (hr₁ : s.read? l₁ = some w₁) (hr₂ : s.read? l₂ = some w₂) :
    h'.tree? s l₂ = h.tree? s l₂ := by
  simp only [Shadow.withTree?, bind, Option.bind_eq_some_iff] at hw
  obtain ⟨rt, hrt, rv, hrv, rt', hrt', hwr⟩ := hw
  by_cases hroot : l₁.root = l₂.root
  · simp only [Location.overlaps, hroot, beq_self_eq_true, Bool.true_and, Bool.or_eq_false_iff]
      at hno
    have hn1 : ¬ l₁.path <+: l₂.path := fun hp => by
      rw [List.isPrefixOf_iff_prefix.2 hp] at hno; cases hno.1
    have hn2 : ¬ l₂.path <+: l₁.path := fun hp => by
      rw [List.isPrefixOf_iff_prefix.2 hp] at hno; cases hno.2
    simp only [State.read?, bind, Option.bind_eq_some_iff] at hr₁ hr₂
    obtain ⟨v₁, hv₁, hg₁⟩ := hr₁
    obtain ⟨v₂, hv₂, hg₂⟩ := hr₂
    rw [hroot, hv₂, Option.some.injEq] at hv₁
    subst hv₁
    rw [hroot, hv₂, Option.some.injEq] at hrv
    subst hrv
    obtain ⟨hi1, hi2⟩ := indices_incomparable l₁.path l₂.path hg₁ hg₂ hn1 hn2
    simp only [Shadow.tree?, bind]
    rw [← hroot, rootTree_withRootTree_self hwr, hrt]
    simp only [Option.bind_some, hroot, hv₂]
    exact at_set_disjoint _ _ hrt' hi1 hi2
  · simp only [Shadow.tree?, bind]
    rw [rootTree_withRootTree_other hwr hroot]

/-! ## The occurrences written back -/

/-- A list has a sublist's elements, counted, when it has at least as many of
each. -/
theorem perm_of_count_le {α : Type} [BEq α] [LawfulBEq α] :
    ∀ (B R : List α), (∀ y, B.count y ≤ R.count y) → ∃ R₂, R.Perm (B ++ R₂)
  | [], R, _ => ⟨R, List.Perm.refl _⟩
  | b :: B, R, h => by
    have hb : b ∈ R := List.count_pos_iff.1 (by
      have := h b; rw [List.count_cons] at this; simp at this; omega)
    obtain ⟨R₂, hR₂⟩ := perm_of_count_le B (R.erase b) fun y => by
      have := h y
      rw [List.count_cons] at this
      by_cases hy : y = b
      · subst hy; rw [List.count_erase_self]; simp at this; omega
      · rw [List.count_erase_of_ne hy]
        have hne : (b == y) = false := by simpa using Ne.symm hy
        simp [hne] at this; omega
    exact ⟨R₂, (List.perm_cons_erase hb).trans (List.Perm.cons _ hR₂)⟩

/-- The occurrences, by key, of the value at a location, with its tree. -/
def locOcc (h : Shadow) (s : State) (l : Location) : KOcc :=
  match h.tree? s l, s.read? l with
  | some t, some w => keys (t.occurrences w)
  | _, _ => []

/-- An identity above every one in a list. -/
def idBound (L : KOcc) : Nat := (L.map (·.1)).foldr max 0 + 1

theorem lt_idBound {L : KOcc} {x : Nat × Key} (hx : x ∈ L) : x.1 < idBound L := by
  unfold idBound
  have : ∀ (l : List Nat), x.1 ∈ l → x.1 ≤ l.foldr max 0 := by
    intro l
    induction l with
    | nil => simp
    | cons a l ih =>
      intro hm
      simp only [List.foldr_cons]
      rcases List.mem_cons.1 hm with rfl | hm
      · exact Nat.le_max_left _ _
      · exact Nat.le_trans (ih hm) (Nat.le_max_right _ _)
  exact Nat.lt_succ_of_le (this _ (List.mem_map.2 ⟨x, hx, rfl⟩))

/-- A tree of a value with identities from `n` on. -/
theorem fresh_ids_ge (c : Creator) (v : Value) (n : Nat) :
    ∀ x ∈ keys ((IdTree.fresh c v n).1.occurrences v), n ≤ x.1 := by
  intro x hx
  rw [(fresh_spec c v n).2.1] at hx
  obtain ⟨o, ho, h1, -⟩ := mem_keys_iff.1 hx
  have := List.of_mem_zip ho
  have := (List.mem_range'.1 this.1)
  omega

theorem locOcc_sub {h : Shadow} {s : State} (hfit : h.fits s = true) (l : Location) :
    ∀ x ∈ locOcc h s l, x ∈ keys (h.occurrences s) := by
  intro x hx
  unfold locOcc at hx
  cases hr : s.read? l with
  | none => rw [hr] at hx; split at hx <;> simp_all
  | some w =>
    obtain ⟨tw, htw, -, R, hR⟩ := tree_read hfit hr
    rw [htw, hr] at hx
    exact keys_perm hR.symm |>.subset (by rw [keys_append]; exact List.mem_append_left _ hx)

/-- Writing back reads each location written first. -/
theorem writeBack_reads : ∀ (refs : List Reference) (ws : List Value) {s s' : State},
    writeBack s refs ws = some s' →
    refs.Pairwise (fun a b => a.location.overlaps b.location = false) →
    ∀ r ∈ refs, ∃ u, s.read? r.location = some u
  | [], [], _, _, _, _, r, hr => by simp at hr
  | x :: refs, w :: ws, s, s', h, hd, r, hr => by
    simp only [writeBack, bind, Option.bind_eq_some_iff] at h
    obtain ⟨s₂, hw, h⟩ := h
    rw [List.pairwise_cons] at hd
    rcases List.mem_cons.1 hr with rfl | hr
    · exact read_of_write hw
    · obtain ⟨u, hu⟩ := writeBack_reads refs ws h hd.2 r hr
      refine ⟨u, ?_⟩
      have : (s₂.invalidate (.mutated x.location)).read? r.location = s₂.read? r.location := rfl
      rw [this, read_write_disjoint hw (hd.1 r hr)] at hu
      exact hu
  | [], _ :: _, _, _, h, _, _, _ => by simp [writeBack] at h
  | _ :: _, [], _, _, h, _, _, _ => by simp [writeBack] at h

/-- **The occurrences written back**: writing back through locations that
overlap nowhere, the state's occurrences split into those at the locations and
a rest that the writes keep, beside the trees written. -/
theorem writes_perm {s₀ : State} :
    ∀ (W : List (Nat × Reference)) (ts : List IdTree) (ws : List Value) {h : Shadow}
      {s s' : State},
    h.fits s = true → (s.frames.map (·.id)).Nodup →
    (s.storage.map fun r => (r.address, r.type)).Nodup →
    s.frames.map (·.id) = s₀.frames.map (·.id) →
    s.storage.map (fun r => (r.address, r.type)) = s₀.storage.map (fun r => (r.address, r.type)) →
    writeBack s (W.map (·.2)) ws = some s' → IdTree.fitsAll ts ws = true →
    WritesAgree s s₀ W →
    ∃ h', (W.zip ts).foldlM (fun h x => h.withTree? s₀ x.1.2.location x.2) h = some h' ∧
      ∃ R, (keys (h.occurrences s)).Perm (W.flatMap (fun x => locOcc h s x.2.location) ++ R) ∧
        (keys (h'.occurrences s')).Perm
          ((ts.zip ws).flatMap (fun y => keys (y.1.occurrences y.2)) ++ R) ∧
        ∀ l, (∀ y ∈ W, y.2.location.overlaps l = false) → (∃ u, s.read? l = some u) →
          ∀ x ∈ locOcc h s l, x ∈ R
  | [], [], [], h, s, s', hfit, _, _, _, _, hwb, _, _ => by
    simp only [List.map_nil, writeBack, Option.some.injEq] at hwb
    subst hwb
    exact ⟨h, rfl, keys (h.occurrences s), by simp, by simp,
      fun l _ _ x hx => locOcc_sub hfit l x hx⟩
  | x :: W, t :: ts, w :: ws, h, s, s', hfit, hids, hres, hids₀, hst₀, hwb, hfa, hag => by
    simp only [List.map_cons, writeBack, bind, Option.bind_eq_some_iff] at hwb
    obtain ⟨s₂, hw, hwb⟩ := hwb
    simp only [IdTree.fitsAll, Bool.and_eq_true] at hfa
    have hd := hag.1
    rw [List.map_cons, List.pairwise_cons] at hd
    have hag₂ := hag.tail hw (.mutated x.2.location)
    obtain ⟨u, hu⟩ := read_of_write hw
    have hwrt := write?_writable hw
    obtain ⟨tw, v, v'', h₃, hv, htw, -, hset, hwr, hwith₃, hfit₃, -, -, -, R₁, hR₁, hR₁'⟩ :=
      tree_write hfit hids hres hu hfa.1 hwrt
    rw [hwr, Option.some.injEq] at hw
    subst hw
    have hids₂ := setRoot_frameIds s x.2.location.root v''
    have hst₂ := setRoot_storageKeys s x.2.location.root v''
    have hreads := writeBack_reads _ ws hwb hd.2
    -- The other locations' occurrences, after a write at this one.
    have hlocEq : ∀ (h₄ : Shadow) (t₄ : IdTree), h.withTree? s x.2.location t₄ = some h₄ →
        W.flatMap (fun y => locOcc h₄ ((s.setRoot x.2.location.root v'').invalidate
          (.mutated x.2.location)) y.2.location) = W.flatMap (fun y => locOcc h s y.2.location) := by
      intro h₄ t₄ hw₄
      refine flatMap_congr_mem fun y hy => ?_
      have hyW : y.2 ∈ W.map (·.2) := List.mem_map_of_mem hy
      have hno := hd.1 y.2 hyW
      obtain ⟨u', hu'⟩ := hreads y.2 hyW
      have hr₃ : ((s.setRoot x.2.location.root v'').invalidate (.mutated x.2.location)).read?
          y.2.location = s.read? y.2.location :=
        read_write_disjoint (s₂ := s.setRoot x.2.location.root v'') hwr hno
      have hus : s.read? y.2.location = some u' := by rw [← hr₃]; exact hu'
      unfold locOcc
      rw [(tree_state' (h := h₄) (s := s) (s' := (s.setRoot x.2.location.root v'').invalidate
          (.mutated x.2.location)) hids₂ hst₂ y.2.location t
          (by simpa using indicesIn_write_disjoint hwr hno)).1,
        tree_withTree_disjoint hw₄ hno hu hus, hr₃]
    have hwith₀ : h.withTree? s₀ x.2.location t = some h₃ := by
      rw [← (tree_state' (h := h) hids₀ hst₀ x.2.location t hag.head).2]; exact hwith₃
    have hids₃ : (((s.setRoot x.2.location.root v'').invalidate
        (.mutated x.2.location)).frames.map (·.id)).Nodup := by
      rw [show ((s.setRoot x.2.location.root v'').invalidate _).frames.map (·.id) =
        s.frames.map (·.id) from hids₂]; exact hids
    have hres₃ : (((s.setRoot x.2.location.root v'').invalidate
        (.mutated x.2.location)).storage.map fun r => (r.address, r.type)).Nodup := by
      rw [show ((s.setRoot x.2.location.root v'').invalidate _).storage.map _ =
        s.storage.map _ from hst₂]; exact hres
    obtain ⟨h', hfold, R', hR', hR'', -⟩ := writes_perm W ts ws
      (s := (s.setRoot x.2.location.root v'').invalidate (.mutated x.2.location)) hfit₃ hids₃ hres₃
      (hids₂.trans hids₀) (hst₂.trans hst₀) hwb hfa.2 hag₂
    -- A probe: the same write with a tree of identities above every one.
    have hfM := (fresh_spec .storage w (idBound (keys (h.occurrences s)))).1
    obtain ⟨twM, vM, vM'', hM, hvM, htwM, hfwM, hsetM, hwrM, hwithM, hfitM, -, -, -, RM, hRM,
      hRM'⟩ := tree_write hfit hids hres hu hfM hwrt
    rw [htw, Option.some.injEq] at htwM
    subst htwM
    rw [hv, Option.some.injEq] at hvM
    subst hvM
    rw [hset, Option.some.injEq] at hsetM
    subst hsetM
    obtain ⟨hM', -, RM', hRMp, -, hmemM⟩ := writes_perm W ts ws
      (s := (s.setRoot x.2.location.root v'').invalidate (.mutated x.2.location)) hfitM hids₃ hres₃
      (hids₂.trans hids₀) (hst₂.trans hst₀) hwb hfa.2 hag₂
    rw [hlocEq h₃ t hwith₃] at hR'
    rw [hlocEq hM _ hwithM] at hRMp
    have hRR : RM.Perm R₁ := (List.perm_append_left_iff _).1 (hRM.symm.trans hR₁)
    have hcount : ∀ y, (W.flatMap (fun z => locOcc h s z.2.location)).count y ≤ R₁.count y := by
      intro y
      by_cases hy : y ∈ W.flatMap (fun z => locOcc h s z.2.location)
      · have hy' : y ∈ keys (h.occurrences s) := by
          obtain ⟨z, -, hz⟩ := List.mem_flatMap.1 hy
          exact locOcc_sub hfit _ y hz
        have h0 : (keys ((IdTree.fresh .storage w (idBound (keys (h.occurrences s)))).1.occurrences
            w)).count y = 0 := List.count_eq_zero.2 fun hm => by
          have := fresh_ids_ge .storage w _ y hm
          have := lt_idBound hy'
          omega
        have e1 := (hRM'.symm.trans hRMp).count_eq y
        have e2 := hRR.count_eq y
        rw [List.count_append, List.count_append, h0] at e1
        omega
      · rw [List.count_eq_zero.2 hy]; exact Nat.zero_le _
    obtain ⟨R₂, hR₂⟩ := perm_of_count_le _ R₁ hcount
    have hfold' : ((x :: W).zip (t :: ts)).foldlM
        (fun h x => h.withTree? s₀ x.1.2.location x.2) h = some h' := by
      simp only [List.zip_cons_cons, List.foldlM_cons, hwith₀, bind, Option.bind_some]
      exact hfold
    refine ⟨h', hfold', R₂, ?_, ?_, ?_⟩
    · have hloc : locOcc h s x.2.location = keys (tw.occurrences u) := by
        unfold locOcc; rw [htw, hu]
      simp only [List.flatMap_cons, hloc, List.append_assoc]
      exact hR₁.trans (List.Perm.append_left _ hR₂)
    · have e : (W.flatMap (fun z => locOcc h s z.2.location) ++ R').Perm
          (W.flatMap (fun z => locOcc h s z.2.location) ++ (keys (t.occurrences w) ++ R₂)) := by
        refine hR'.symm.trans (hR₁'.trans ?_)
        refine (List.Perm.append_left _ hR₂).trans ?_
        exact List.perm_iff_count.2 fun a => by simp only [List.count_append]; omega
      have hR3 := (List.perm_append_left_iff _).1 e
      simp only [List.zip_cons_cons, List.flatMap_cons]
      refine hR''.trans ?_
      refine (List.Perm.append_left _ hR3).trans ?_
      exact List.perm_iff_count.2 fun a => by simp only [List.count_append]; omega
    · -- A location the writes do not overlap: its occurrences are in the rest.
      intro l hl ⟨u', hu'⟩ y hy
      have hxl := hl x List.mem_cons_self
      have hr₃ : ((s.setRoot x.2.location.root v'').invalidate (.mutated x.2.location)).read? l =
          s.read? l := read_write_disjoint (s₂ := s.setRoot x.2.location.root v'') hwr hxl
      have hloc : locOcc hM ((s.setRoot x.2.location.root v'').invalidate
          (.mutated x.2.location)) l = locOcc h s l := by
        unfold locOcc
        rw [(tree_state' (h := hM) (s := s) (s' := (s.setRoot x.2.location.root v'').invalidate
            (.mutated x.2.location)) hids₂ hst₂ l t
            (by simpa using indicesIn_write_disjoint hwr hxl)).1,
          tree_withTree_disjoint hwithM hxl hu hu', hr₃]
      have hyM := hmemM l (fun z hz => hl z (List.mem_cons_of_mem _ hz)) ⟨u', by rw [hr₃]; exact hu'⟩
        y (by rw [hloc]; exact hy)
      have hy' : y ∈ keys (h.occurrences s) := locOcc_sub hfit _ y hy
      have h0 : (keys ((IdTree.fresh .storage w (idBound (keys (h.occurrences s)))).1.occurrences
          w)).count y = 0 := List.count_eq_zero.2 fun hm => by
        have := fresh_ids_ge .storage w _ y hm
        have := lt_idBound hy'
        omega
      have e1 := (hRM'.symm.trans hRMp).count_eq y
      have e2 := hRR.count_eq y
      have e3 := hR₂.count_eq y
      have e4 := List.count_pos_iff.2 hyM
      rw [List.count_append, List.count_append, h0] at e1
      rw [List.count_append] at e3
      exact List.count_pos_iff.1 (by omega)
  | [], _, _ :: _, _, _, _, _, _, _, _, _, hwb, _, _ => by simp [writeBack] at hwb
  | [], _ :: _, [], _, _, _, _, _, _, _, _, _, hfa, _ => by simp [IdTree.fitsAll] at hfa
  | _ :: _, _, [], _, _, _, _, _, _, _, _, hwb, _, _ => by simp [writeBack] at hwb
  | _ :: _, [], _ :: _, _, _, _, _, _, _, _, _, _, hfa, _ => by simp [IdTree.fitsAll] at hfa

/-! ## The effect of a call of a modeled native -/

/-- **The effect of a native call**, on positions: the native's nodes at
positions `AZ` (distinct), the positions passed on `O` (a native's, or of what
stays where it is, in the rest `Rk`, of a type with `copy`), the references
dropped `Dz`, the consumptions of the native's nodes not passed on, and the
outputs `RO`: the nodes passed on and the identities issued. -/
theorem native_effectK {α : Type} [BEq α] [LawfulBEq α] {P : Program}
    (AZ : List (α × (Nat × Key))) (O : List α) (occAt : α → Nat × Key)
    (hocc : ∀ x ∈ AZ, x.2 = occAt x.1) (hnd : (AZ.map (·.1)).Nodup) {Rk : KOcc}
    (hO : ∀ a ∈ O, a ∈ AZ.map (·.1) ∨
      (occAt a ∈ Rk ∧ ((occAt a).2.type.abilities P).copy = true))
    (hcopy : ∀ x ∈ AZ, (x.2.2.type.abilities P).copy = true ∨ O.count x.1 ≤ 1)
    {Dz : KOcc} (hDz : ∀ x ∈ Dz, (x.2.type.abilities P).drop = true) {evs : List Event}
    (hCE : keys (consumedOf evs) = (AZ.filter fun x => !O.contains x.1).map (·.2))
    {RO : KOcc} (hRO : RO.Perm (O.map occAt ++ issuedOf evs)) {f f' : Nat}
    (hb : ∀ x ∈ issuedOf evs, f ≤ x.1 ∧ x.1 < f') (hnd' : ((issuedOf evs).map (·.1)).Nodup)
    (hff : f ≤ f') :
    EffectK P (Dz ++ AZ.map (·.2) ++ Rk) (RO ++ Rk) evs f f' := by
  let owned : α → Bool := fun a => (AZ.map (·.1)).contains a
  have hkeep : (AZ.filter fun x => O.contains x.1).map (·.2) =
      ((AZ.map (·.1)).filter fun a => O.contains a).map occAt := by
    rw [List.filter_map, List.map_map]
    exact List.map_congr_left fun x hx => hocc x (List.mem_filter.1 hx).1
  have hOown : ∀ a ∈ O.filter owned, a ∈ AZ.map (·.1) := fun a ha => by
    simpa [owned] using (List.mem_filter.1 ha).2
  have hperm := perm_positions hnd hOown
  have hcont : ∀ a, (O.filter owned).contains a = (owned a && O.contains a) := by
    intro a
    rw [Bool.eq_iff_iff]
    simp only [List.contains_iff_mem, List.mem_filter, Bool.and_eq_true]
    exact ⟨fun h => ⟨h.2, h.1⟩, fun h => ⟨h.2, h.1⟩⟩
  have hfilt : ((AZ.map (·.1)).filter fun a => (O.filter owned).contains a) =
      (AZ.map (·.1)).filter fun a => O.contains a := by
    refine List.filter_congr fun a ha => ?_
    rw [hcont]
    have : owned a = true := by simpa [owned] using ha
    simp [this]
  rw [hfilt] at hperm
  obtain ⟨extras, hextras⟩ : ∃ e, e = ((AZ.map (·.1)).filter fun a => O.contains a).flatMap
      fun a => List.replicate ((O.filter owned).count a - 1) a := ⟨_, rfl⟩
  rw [← hextras] at hperm
  refine ⟨(AZ.filter fun x => O.contains x.1).map (·.2) ++ Rk, Dz,
    extras.map occAt ++ (O.filter fun a => !owned a).map occAt, ?_, ?_, ?_, ?_, hb, hnd', hff⟩
  · rw [hCE]
    refine List.perm_iff_count.2 fun y => ?_
    have := ((List.filter_append_perm (fun x => O.contains x.1) AZ).map (·.2)).count_eq y
    simp only [List.map_append, List.count_append] at this ⊢
    omega
  · refine (List.Perm.append_right _ hRO).trans ?_
    have hOs := (List.filter_append_perm owned O).map occAt
    have hOs' := hperm.map occAt
    rw [List.map_append] at hOs hOs'
    refine List.perm_iff_count.2 fun y => ?_
    have e1 := hOs.count_eq y
    have e2 := hOs'.count_eq y
    rw [← hkeep] at e2
    simp only [List.count_append] at e1 e2 ⊢
    omega
  · exact hDz
  · intro x hx
    rcases List.mem_append.1 hx with hx | hx
    · obtain ⟨a, ha, rfl⟩ := List.mem_map.1 hx
      rw [hextras] at ha
      obtain ⟨b, hb', hab⟩ := List.mem_flatMap.1 ha
      rw [List.mem_replicate] at hab
      obtain ⟨hcnt, rfl⟩ := hab
      refine ⟨List.mem_append_left _ (by rw [hkeep]; exact List.mem_map_of_mem hb'), ?_⟩
      obtain ⟨hbL, -⟩ := List.mem_filter.1 hb'
      obtain ⟨z, hz, rfl⟩ := List.mem_map.1 hbL
      rcases hcopy z hz with hc | hc
      · rw [← hocc z hz]; exact hc
      · have : (O.filter owned).count z.1 ≤ O.count z.1 := List.filter_sublist.count_le _
        omega
    · obtain ⟨a, ha, rfl⟩ := List.mem_map.1 hx
      obtain ⟨haO, hna⟩ := List.mem_filter.1 ha
      rcases hO a haO with ho | ⟨hR, hc⟩
      · have : owned a = true := by simp [owned, ho]
        rw [this] at hna; cases hna
      · exact ⟨List.mem_append_right _ hR, hc⟩

/-! ## The annotation exists -/

/-! ## Helpers -/

theorem mapM_some' {α β : Type} {f : α → Option β} :
    ∀ {l : List α}, (∀ x ∈ l, ∃ b, f x = some b) → ∃ l', l.mapM f = some l'
  | [], _ => ⟨[], rfl⟩
  | x :: l, h => by
    obtain ⟨b, hb⟩ := h x List.mem_cons_self
    obtain ⟨l', hl'⟩ := mapM_some' (l := l) fun y hy => h y (List.mem_cons_of_mem _ hy)
    exact ⟨b :: l', by simp [List.mapM_cons, hb, hl']⟩

theorem mapM_length' {α β : Type} {f : α → Option β} :
    ∀ {l : List α} {l' : List β}, l.mapM f = some l' → l'.length = l.length
  | [], l', h => by simp at h; subst h; rfl
  | x :: l, l', h => by
    simp only [List.mapM_cons, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨b, -, l'', hl'', rfl⟩ := h
    simp [mapM_length' hl'']

theorem mapM_getElem_fwd {α β : Type} {f : α → Option β} :
    ∀ {l : List α} {l' : List β}, l.mapM f = some l' → ∀ {i : Nat} {a : α}, l[i]? = some a →
      ∃ b, l'[i]? = some b ∧ f a = some b
  | [], _, _, _, _, ha => by simp at ha
  | x :: l, l', h, i, a, ha => by
    simp only [List.mapM_cons, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨b, hb, l'', hl'', rfl⟩ := h
    cases i with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at ha
      subst ha
      exact ⟨b, rfl, hb⟩
    | succ i =>
      simp only [List.getElem?_cons_succ] at ha ⊢
      exact mapM_getElem_fwd hl'' ha

/-- The node of a tree at a path where the value it fits has a node is an
occurrence, with that node. -/
theorem at_occ {t u : IdTree} {v w : Value} (hf : t.fits v = true) {q : Path}
    (hw : v.at? q = some w) (hu : t.at? q = some u) : (u.id, w) ∈ t.occurrences v := by
  have hn := at_nodes q hw
  obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 hn
  have hi' : i < (t.occurrences v).length := by
    rw [occurrences_length hf]; exact (List.getElem?_eq_some_iff.1 hi).1
  obtain ⟨o, ho⟩ : ∃ o, (t.occurrences v)[i]? = some o := ⟨_, List.getElem?_eq_getElem hi'⟩
  obtain ⟨-, hid, hv⟩ := nodes_zip t v hf _ (zip_of_getElem? hi ho)
  simp only [hu, Option.map_some, Option.some.injEq] at hid hv
  have : o = (u.id, w) := by
    obtain ⟨o1, o2⟩ := o
    simp only at hid hv
    rw [hid, hv]
  rw [← this]
  exact List.mem_of_getElem? ho

/-- **A mutable reference argument overlaps no other one.** -/
theorem exclusive_mem {args : List Value} (h : Step.exclusiveArguments args = true)
    {a b : Reference} (ha : Value.ref a ∈ args) (hb : Value.ref b ∈ args) (hne : a ≠ b)
    (hm : a.mutable = true) : a.location.overlaps b.location = false := by
  unfold Step.exclusiveArguments at h
  rw [List.all_eq_true] at h
  have hmem : ∀ c, Value.ref c ∈ args → c ∈ args.filterMap fun v => match v with
      | .ref r => some r
      | _ => none := fun c hc => List.mem_filterMap.2 ⟨_, hc, rfl⟩
  obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 (hmem a ha)
  obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 (hmem b hb)
  have hai : (a, i) ∈ (args.filterMap fun v => match v with
      | .ref r => some r
      | _ => none).zipIdx := List.mem_zipIdx_iff_getElem?.2 hi
  have hbj : (b, j) ∈ (args.filterMap fun v => match v with
      | .ref r => some r
      | _ => none).zipIdx := List.mem_zipIdx_iff_getElem?.2 hj
  have hij : i ≠ j := fun e => by
    subst e
    rw [hi, Option.some.injEq] at hj
    exact hne hj
  have := h _ hai
  simp only [hm, Bool.not_true, Bool.false_or, List.all_eq_true] at this
  have := this _ hbj
  simp only [Bool.or_eq_true, beq_iff_eq, hij, false_or, Bool.not_eq_eq_eq_not, Bool.not_true]
    at this
  exact this

/-- Writing back through references to no held value keeps the held values. -/
theorem writeBack_held : ∀ (refs : List Reference) (ws : List Value) {s s' : State},
    writeBack s refs ws = some s' → (∀ r ∈ refs, r.location.root.isHeld = false) →
    s'.held = s.held
  | [], [], s, s', h, _ => by simp only [writeBack, Option.some.injEq] at h; rw [h]
  | x :: refs, w :: ws, s, s', h, hr => by
    simp only [writeBack, bind, Option.bind_eq_some_iff] at h
    obtain ⟨s₂, hw, h⟩ := h
    rw [writeBack_held refs ws h fun r hr' => hr r (List.mem_cons_of_mem _ hr')]
    simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hw
    obtain ⟨_, -, _, -, rfl⟩ := hw
    have hx := hr x List.mem_cons_self
    show (State.setRoot s x.location.root _).held = s.held
    cases hroot : x.location.root with
    | local_ _ _ => rfl
    | global _ _ => rfl
    | held _ => rw [hroot] at hx; cases hx
  | [], _ :: _, _, _, h, _ => by simp [writeBack] at h
  | _ :: _, [], _, _, h, _ => by simp [writeBack] at h

/-- **Writing back keeps every closure**: a closure of the state after was in
it before, or in a value written. -/
theorem writes_closures {s₀ : State} :
    ∀ (W : List (Nat × Reference)) (ts : List IdTree) (ws : List Value) {h h' : Shadow}
      {s s' : State},
    h.fits s = true → (s.frames.map (·.id)).Nodup →
    (s.storage.map fun r => (r.address, r.type)).Nodup →
    s.frames.map (·.id) = s₀.frames.map (·.id) →
    s.storage.map (fun r => (r.address, r.type)) = s₀.storage.map (fun r => (r.address, r.type)) →
    WritesAgree s s₀ W →
    writeBack s (W.map (·.2)) ws = some s' →
    (W.zip ts).foldlM (fun h x => h.withTree? s₀ x.1.2.location x.2) h = some h' →
    IdTree.fitsAll ts ws = true →
    ∀ x ∈ h'.occurrences s', IsClosure x.2 = true →
      x ∈ h.occurrences s ∨ ∃ y ∈ ts.zip ws, x ∈ y.1.occurrences y.2
  | [], [], [], h, h', s, s', _, _, _, _, _, _, hwb, hfold, _ => by
    simp only [List.map_nil, writeBack, Option.some.injEq] at hwb
    simp only [List.zip_nil_left, List.foldlM_nil, pure, Option.some.injEq] at hfold
    subst hwb hfold
    exact fun x hx _ => Or.inl hx
  | x :: W, t :: ts, w :: ws, h, h', s, s', hfit, hids, hres, hids₀, hst₀, hag, hwb, hfold,
      hfa => by
    simp only [List.map_cons, writeBack, bind, Option.bind_eq_some_iff] at hwb
    obtain ⟨s₂, hw, hwb⟩ := hwb
    simp only [List.zip_cons_cons, List.foldlM_cons, bind, Option.bind_eq_some_iff] at hfold
    obtain ⟨h₂, hwith, hfold⟩ := hfold
    simp only [IdTree.fitsAll, Bool.and_eq_true] at hfa
    obtain ⟨u, hu⟩ := read_of_write hw
    have hag₂ := hag.tail hw (.mutated x.2.location)
    have hwith' : h.withTree? s x.2.location t = some h₂ := by
      rw [(tree_state' (h := h) hids₀ hst₀ x.2.location t hag.head).2]; exact hwith
    obtain ⟨tw, v, v'', h₃, hv, -, -, hset, hwr, hwith₃, hfit₃, -⟩ :=
      tree_write hfit hids hres hu hfa.1 (write?_writable hw)
    rw [hwith', Option.some.injEq] at hwith₃
    subst hwith₃
    have hcl := tree_write_closures hfit hids hres hu hfa.1 hwith' hwr
    rw [hwr, Option.some.injEq] at hw
    subst hw
    have hids₂ := setRoot_frameIds s x.2.location.root v''
    have hst₂ := setRoot_storageKeys s x.2.location.root v''
    have IH := writes_closures W ts ws
      (s := (s.setRoot x.2.location.root v'').invalidate (.mutated x.2.location))
      hfit₃ (by rw [show ((s.setRoot x.2.location.root v'').invalidate _).frames.map (·.id) =
          s.frames.map (·.id) from hids₂]; exact hids)
      (by rw [show ((s.setRoot x.2.location.root v'').invalidate _).storage.map _ =
          s.storage.map _ from hst₂]; exact hres)
      (hids₂.trans hids₀) (hst₂.trans hst₀) hag₂ hwb hfold hfa.2
    intro y hy hc
    rcases IH y hy hc with hy | ⟨z, hz, hy⟩
    · rcases hcl y hy hc with hy | hy
      · exact Or.inl hy
      · exact Or.inr ⟨(t, w), List.mem_cons_self, hy⟩
    · exact Or.inr ⟨z, List.mem_cons_of_mem _ hz, hy⟩
  | [], _, _ :: _, _, _, _, _, _, _, _, _, _, _, hwb, _, _ => by simp [writeBack] at hwb
  | [], _ :: _, [], _, _, _, _, _, _, _, _, _, _, _, _, hfa => by simp [IdTree.fitsAll] at hfa
  | _ :: _, _, [], _, _, _, _, _, _, _, _, _, _, hwb, _, _ => by simp [writeBack] at hwb
  | _ :: _, [], _ :: _, _, _, _, _, _, _, _, _, _, _, _, _, hfa => by simp [IdTree.fitsAll] at hfa

/-! ## The native's nodes, by position -/

/-- Whether a native keeps what its argument `k` designates where it is
(`Program.nativeKeeps`). -/
def sharedAt (P : Program) (f : FunctionRef) (ptys : List RuntimeType) (k : Nat) : Bool :=
  (ptys[k]?).any (P.nativeKeeps f)

/-- The nodes a native sees of its arguments from `k` on that it does not
share, at their positions, with their keyed occurrences. -/
def ownedPos (sh : Nat → Bool) (k : Nat) (seen : List IdTree) (views : List Value) :
    List ((Nat × Path) × (Nat × Key)) :=
  ((seen.zip views).zipIdx k).flatMap fun x =>
    if sh x.2 then [] else
      (x.1.2.nodes.zip (x.1.1.occurrences x.1.2)).map fun y => ((x.2, y.1.1), (y.2.1, y.2.2.key))

/-- The reference arguments' own keyed occurrences. -/
def refLeaves (argIds : List IdTree) (args : List Value) : KOcc :=
  (argIds.zip args).flatMap fun x => match x.2 with
    | .ref r => [(x.1.id, (Value.ref r).key)]
    | _ => []

theorem refArgsOf_cons_ref (t : IdTree) (ts : List IdTree) (r : Reference) (as : List Value) :
    refArgsOf (t :: ts) (.ref r :: as) = (t.id, r) :: refArgsOf ts as := rfl

theorem refArgsOf_cons_val (t : IdTree) (ts : List IdTree) {a : Value} (as : List Value)
    (ha : ∀ r, a ≠ .ref r) : refArgsOf (t :: ts) (a :: as) = refArgsOf ts as := by
  cases a <;> simp_all [refArgsOf]

theorem refLeaves_cons_ref (t : IdTree) (ts : List IdTree) (r : Reference) (as : List Value) :
    refLeaves (t :: ts) (.ref r :: as) = (t.id, (Value.ref r).key) :: refLeaves ts as := rfl

theorem refLeaves_cons_val (t : IdTree) (ts : List IdTree) {a : Value} (as : List Value)
    (ha : ∀ r, a ≠ .ref r) : refLeaves (t :: ts) (a :: as) = refLeaves ts as := by
  cases a <;> simp_all [refLeaves]

theorem ownedPos_cons (sh : Nat → Bool) (k : Nat) (st : IdTree) (sts : List IdTree) (w : Value)
    (ws : List Value) :
    ownedPos sh k (st :: sts) (w :: ws) =
      (if sh k then [] else
        (w.nodes.zip (st.occurrences w)).map fun y => ((k, y.1.1), (y.2.1, y.2.2.key))) ++
        ownedPos sh (k + 1) sts ws := rfl

theorem zip_keys {t : IdTree} {v : Value} (hf : t.fits v = true) (k : Nat) :
    ((v.nodes.zip (t.occurrences v)).map fun y => ((k, y.1.1), (y.2.1, y.2.2.key))).map (·.2) =
      keys (t.occurrences v) := by
  rw [List.map_map]
  have hz := List.map_snd_zip (l₁ := v.nodes) (l₂ := t.occurrences v)
    (by simp [occurrences_length hf])
  conv => rhs; rw [← hz]
  simp [keys, Function.comp_def]

/-- **The occurrences of a native's arguments and of what it writes back**:
the reference arguments' own, and the nodes it sees of the arguments it does not
share. -/
theorem pre_split {P : Program} {f : FunctionRef} {h : Shadow} {s : State} (sh : Nat → Bool) :
    ∀ (k : Nat) (argIds : List IdTree) (args : List Value) (seen : List IdTree)
      (views : List Value),
    argIds.length = args.length → seen.length = args.length → views.length = args.length →
    (∀ (i : Nat) (st : IdTree) (w : Value), seen[i]? = some st → views[i]? = some w →
      st.fits w = true) →
    (∀ (i : Nat) (t : IdTree) (a : Value) (st : IdTree) (w : Value), argIds[i]? = some t →
      args[i]? = some a → seen[i]? = some st →
      views[i]? = some w → (∀ r, a ≠ .ref r) → st = t ∧ w = a ∧ sh (k + i) = false) →
    (∀ (i : Nat) (t : IdTree) (r : Reference) (st : IdTree) (w : Value), argIds[i]? = some t → args[i]? = some (.ref r) → seen[i]? = some st →
      views[i]? = some w → t.occurrences (.ref r) = [(t.id, .ref r)] ∧
        locOcc h s r.location = keys (st.occurrences w) ∧
        sh (k + i) = !(r.mutable && P.nativeShape? f == some .update)) →
    (keys (IdTree.occurrencesAll argIds args) ++
        (writtenOf P f (refArgsOf argIds args)).flatMap (fun x => locOcc h s x.2.location)).Perm
      (refLeaves argIds args ++ (ownedPos sh k seen views).map (·.2))
  | k, [], [], [], [], _, _, _, _, _, _ => by
    simp [IdTree.occurrencesAll, writtenOf, refArgsOf, refLeaves, ownedPos]
  | k, t :: ts, a :: as, st :: sts, w :: ws, hl1, hl2, hl3, hfits, hval, href => by
    simp only [List.length_cons, Nat.add_right_cancel_iff] at hl1 hl2 hl3
    have IH := pre_split sh (k + 1) ts as sts ws hl1 hl2 hl3
      (fun i st w h1 h2 => hfits (i + 1) st w h1 h2)
      (fun i t a st w h1 h2 h3 h4 h5 => by
        have := hval (i + 1) t a st w h1 h2 h3 h4 h5
        rwa [show k + (i + 1) = k + 1 + i by omega] at this)
      (fun i t r st w h1 h2 h3 h4 => by
        have := href (i + 1) t r st w h1 h2 h3 h4
        rwa [show k + (i + 1) = k + 1 + i by omega] at this)
    have hf0 := hfits 0 st w rfl rfl
    rw [ownedPos_cons, List.map_append]
    by_cases ha : ∃ r, a = .ref r
    · obtain ⟨r, rfl⟩ := ha
      obtain ⟨hocc, hloc, hsh⟩ := href 0 t r st w rfl rfl rfl rfl
      rw [Nat.add_zero] at hsh
      rw [refArgsOf_cons_ref, refLeaves_cons_ref]
      simp only [IdTree.occurrencesAll, hocc, keys_append, writtenOf, List.filter_cons]
      refine List.perm_iff_count.2 fun y => ?_
      have e := IH.count_eq y
      simp only [writtenOf] at e
      cases hw : (r.mutable && P.nativeShape? f == some .update)
      · simp only [hw, Bool.not_false] at hsh
        simp only [Bool.false_eq_true, ↓reduceIte, hsh, List.map_nil, List.nil_append] at e ⊢
        simp only [keys_cons, keys_nil, List.count_append, List.count_cons, List.cons_append,
          List.nil_append] at e ⊢
        omega
      · simp only [hw, Bool.not_true] at hsh
        simp only [↓reduceIte, hsh, Bool.false_eq_true, List.flatMap_cons, zip_keys hf0, ← hloc]
          at e ⊢
        simp only [keys_cons, keys_nil, List.count_append, List.count_cons, List.cons_append,
          List.nil_append] at e ⊢
        omega
    · have ha' : ∀ r, a ≠ .ref r := fun r e => ha ⟨r, e⟩
      obtain ⟨rfl, rfl, hsh⟩ := hval 0 t a st w rfl rfl rfl rfl ha'
      rw [Nat.add_zero] at hsh
      rw [refArgsOf_cons_val _ _ _ ha', refLeaves_cons_val _ _ _ ha']
      simp only [IdTree.occurrencesAll, keys_append, hsh, Bool.false_eq_true, ↓reduceIte,
        zip_keys hf0]
      refine List.perm_iff_count.2 fun y => ?_
      have e := IH.count_eq y
      simp only [List.count_append] at e ⊢
      omega

theorem mem_ownedPos {sh : Nat → Bool} {seen : List IdTree} {views : List Value}
    (hfits : ∀ (j : Nat) (st : IdTree) (w : Value), seen[j]? = some st → views[j]? = some w →
      st.fits w = true)
    {x : (Nat × Path) × (Nat × Key)} (hx : x ∈ ownedPos sh 0 seen views) :
    ∃ j q st w v i, x = ((j, q), (i, v.key)) ∧ seen[j]? = some st ∧ views[j]? = some w ∧
      sh j = false ∧ w.at? q = some v ∧ (st.at? q).map (·.id) = some i := by
  unfold ownedPos at hx
  obtain ⟨⟨⟨st, w⟩, j⟩, hz, hx⟩ := List.mem_flatMap.1 hx
  rw [List.mem_zipIdx_iff_getElem?, List.getElem?_zip_eq_some] at hz
  obtain ⟨hst, hw⟩ := hz
  simp only at hx
  cases hsh : sh j with
  | true => rw [hsh] at hx; simp at hx
  | false =>
    rw [hsh] at hx
    simp only [Bool.false_eq_true, ↓reduceIte, List.mem_map] at hx
    obtain ⟨y, hy, rfl⟩ := hx
    obtain ⟨h1, h2, h3⟩ := nodes_zip st w (hfits j st w hst hw) y hy
    exact ⟨j, y.1.1, st, w, y.1.2, y.2.1, by rw [h3], hst, hw, hsh, h1, h2⟩

theorem ownedPos_ge {sh : Nat → Bool} :
    ∀ {k : Nat} {seen : List IdTree} {views : List Value}, ∀ x ∈ ownedPos sh k seen views, k ≤ x.1.1
  | _, [], _, x, hx => by simp [ownedPos] at hx
  | _, _ :: _, [], x, hx => by simp [ownedPos] at hx
  | k, st :: sts, w :: ws, x, hx => by
    rw [ownedPos_cons, List.mem_append] at hx
    rcases hx with hx | hx
    · split at hx
      · simp at hx
      · obtain ⟨y, -, rfl⟩ := List.mem_map.1 hx
        exact Nat.le_refl _
    · have := ownedPos_ge x hx
      omega

theorem ownedPos_nodup {sh : Nat → Bool} :
    ∀ {k : Nat} {seen : List IdTree} {views : List Value},
      (∀ (j : Nat) (st : IdTree) (w : Value), seen[j]? = some st → views[j]? = some w →
        st.fits w = true) →
      ((ownedPos sh k seen views).map (·.1)).Nodup
  | _, [], _, _ => by simp [ownedPos]
  | _, _ :: _, [], _ => by simp [ownedPos]
  | k, st :: sts, w :: ws, hfits => by
    rw [ownedPos_cons, List.map_append]
    have ih := ownedPos_nodup (sh := sh) (k := k + 1) (seen := sts) (views := ws)
      fun j st w h1 h2 => hfits (j + 1) st w h1 h2
    refine List.nodup_append.2 ⟨?_, ih, ?_⟩
    · split
      · simp
      · rw [List.map_map]
        have hz : (w.nodes.zip (st.occurrences w)).map (fun y : (Path × Value) × (Nat × Value) =>
            y.1.1) = w.nodes.map (·.1) := by
          have := congrArg (List.map (·.1)) (List.map_fst_zip (l₁ := w.nodes)
            (l₂ := st.occurrences w) (by simp [occurrences_length (hfits 0 st w rfl rfl)]))
          rw [List.map_map] at this
          exact this
        have : ((fun x : (Nat × Path) × (Nat × Key) => x.1) ∘
            fun y : (Path × Value) × (Nat × Value) => ((k, y.1.1), (y.2.1, y.2.2.key))) =
            (fun q => (k, q)) ∘ fun y => y.1.1 := rfl
        rw [this, ← List.map_map, hz]
        exact List.Pairwise.map _ (fun a b hab h => hab (Prod.mk.inj h).2) (nodes_nodup w)
    · intro a ha b hb hab
      split at ha
      · simp at ha
      · obtain ⟨y, hy, rfl⟩ := List.mem_map.1 ha
        obtain ⟨y', -, rfl⟩ := List.mem_map.1 hy
        obtain ⟨z, hz, rfl⟩ := List.mem_map.1 hb
        have h1 := ownedPos_ge z hz
        have h2 : z.1.1 = k := by rw [← hab]
        omega

theorem ownedPos_mem {sh : Nat → Bool} {seen : List IdTree} {views : List Value} {j : Nat}
    {st : IdTree} {w v : Value} {q : Path} (hst : seen[j]? = some st) (hw : views[j]? = some w)
    (hf : st.fits w = true) (hsh : sh j = false) (hv : w.at? q = some v) :
    (j, q) ∈ (ownedPos sh 0 seen views).map (·.1) := by
  have hn := at_nodes q hv
  obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 hn
  have hi' : i < (st.occurrences w).length := by
    rw [occurrences_length hf]; exact (List.getElem?_eq_some_iff.1 hi).1
  obtain ⟨o, ho⟩ : ∃ o, (st.occurrences w)[i]? = some o := ⟨_, List.getElem?_eq_getElem hi'⟩
  have hz : (((st, w), j) : (IdTree × Value) × Nat) ∈ (seen.zip views).zipIdx := by
    rw [List.mem_zipIdx_iff_getElem?, List.getElem?_zip_eq_some]
    exact ⟨hst, hw⟩
  unfold ownedPos
  rw [List.mem_map]
  refine ⟨((j, q), (o.1, o.2.key)), List.mem_flatMap.2 ⟨_, hz, ?_⟩, rfl⟩
  simp only [hsh, Bool.false_eq_true, ↓reduceIte, List.mem_map]
  exact ⟨((q, v), o), zip_of_getElem? hi ho, rfl⟩

theorem consumedOf_flatMap {β : Type} (F : β → List Event) :
    ∀ (L : List β), consumedOf (L.flatMap F) = L.flatMap fun x => consumedOf (F x)
  | [] => rfl
  | x :: L => by rw [List.flatMap_cons, consumedOf_append, consumedOf_flatMap F L]; rfl

theorem consumed_keys_aux (origins : List (Nat × Path)) (c : Path → Consumer) (k : Nat) :
    ∀ (l : List ((Path × Value) × (Nat × Value))), (∀ y ∈ l, y.2.2 = y.1.2) →
    keys (consumedOf (l.filterMap fun y => if origins.contains (k, y.1.1) then none
        else some (Event.consumed y.2.1 y.1.2 (c y.1.1)))) =
      ((l.map fun y => ((k, y.1.1), (y.2.1, y.2.2.key))).filter
        fun x => !origins.contains x.1).map (·.2)
  | [], _ => rfl
  | y :: l, hl => by
    have ih := consumed_keys_aux origins c k l fun z hz => hl z (List.mem_cons_of_mem _ hz)
    have hy := hl y List.mem_cons_self
    by_cases hc : origins.contains (k, y.1.1) = true
    · simp only [List.filterMap_cons, hc, ↓reduceIte, List.map_cons, List.filter_cons,
        Bool.not_true, Bool.false_eq_true]
      exact ih
    · simp only [List.filterMap_cons, hc, ↓reduceIte, Bool.false_eq_true, List.map_cons,
        List.filter_cons, Bool.not_false]
      simp only [consumedOf, List.filterMap_cons, keys_cons] at ih ⊢
      rw [ih, hy]

/-- **What a native consumes**: the nodes it does not share and does not pass
on. -/
theorem consumedOfNative_keys {P : Program} {f : FunctionRef} {m : ModuleId}
    {ptys : List RuntimeType} {seen : List IdTree} {views : List Value}
    {origins : List (Nat × Path)}
    (hfits : ∀ (j : Nat) (st : IdTree) (w : Value), seen[j]? = some st → views[j]? = some w →
      st.fits w = true) :
    keys (consumedOf (consumedOfNative P f m ptys seen views origins)) =
      ((ownedPos (sharedAt P f ptys) 0 seen views).filter
        fun x => !origins.contains x.1).map (·.2) := by
  unfold consumedOfNative ownedPos
  rw [consumedOf_flatMap, keys, List.map_flatMap, List.filter_flatMap, List.map_flatMap]
  refine flatMap_congr_mem fun x hx => ?_
  obtain ⟨⟨st, w⟩, j⟩ := x
  rw [List.mem_zipIdx_iff_getElem?, List.getElem?_zip_eq_some] at hx
  have hf := hfits j st w hx.1 hx.2
  simp only [sharedAt]
  split
  · rfl
  · exact consumed_keys_aux origins
      (fun q => .native f m (keepsNothingInside origins j q)) j _
      fun y hy => (nodes_zip st w hf y hy).2.2

/-- **What a native sees**, argument by argument: a value argument with its
own tree, or the value at the location a reference argument designates, with
the tree there. -/
theorem seen_spec {σ : Bool} {P : Program} {s s₁ : State} {h : Shadow} {f : FunctionRef}
    {args views : List Value} (hpre : Pre σ P s h)
    (hstep : step P s = .callNative s₁ f args views) {g : ShadowFrame} {hs : List ShadowFrame}
    (hh : h.frames = g :: hs) :
    ∃ seen, seenTrees h s args (g.stack.take args.length).reverse = some seen ∧
      seen.length = args.length ∧ views.length = args.length ∧
      ((g.stack.take args.length).reverse).length = args.length ∧
      IdTree.fitsAll (g.stack.take args.length).reverse args = true ∧
      (∀ (j : Nat) (st : IdTree) (w : Value), seen[j]? = some st → views[j]? = some w →
        st.fits w = true) ∧
      (∀ (j : Nat) (t : IdTree) (a : Value) (st : IdTree) (w : Value),
        (g.stack.take args.length).reverse[j]? = some t → args[j]? = some a →
        seen[j]? = some st → views[j]? = some w → (∀ r, a ≠ .ref r) → st = t ∧ w = a) ∧
      (∀ (j : Nat) (t : IdTree) (r : Reference) (st : IdTree) (w : Value),
        (g.stack.take args.length).reverse[j]? = some t → args[j]? = some (.ref r) →
        seen[j]? = some st → views[j]? = some w →
        t.occurrences (.ref r) = [(t.id, .ref r)] ∧ h.tree? s r.location = some st ∧
          s.read? r.location = some w) := by
  obtain ⟨fr, rest, n, hsf, hnl, hargs, hs₁, hfacts⟩ := step_callNative hstep
  have hg : g.fits fr = true := by
    have := hpre.fits
    rw [shadow_fits, hsf, hh, framesFit_cons] at this
    simp only [Bool.and_eq_true] at this
    exact this.1.1
  have hfa := frame_stack_fits hg
  have hlen := fitsAll_length hfa
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  have hargfit : IdTree.fitsAll (g.stack.take args.length).reverse args = true := by
    rw [hargl, hargs]; exact fitsAll_reverse (fitsAll_take_drop n hfa).1
  have hidl : ((g.stack.take args.length).reverse).length = args.length := by
    have := fitsAll_length hargfit; omega
  -- What a reference argument designates, in the state before the call.
  have hderef : ∀ r, Value.ref r ∈ args → ∀ v, s₁.deref? r.location r.since = some v →
      s.read? r.location = some v := by
    intro r _ v hv
    rw [hs₁] at hv
    have hv' : s.deref? r.location r.since = some v := by
      rw [← hv]
      exact (deref_eq (s := s) (s' := { s with frames := { fr with stack := fr.stack.drop n } :: rest })
        (by simp [hsf]) rfl rfl rfl _ _).symm
    exact deref?_read hv'
  obtain ⟨seen, hseen⟩ : ∃ seen, seenTrees h s args (g.stack.take args.length).reverse = some seen := by
    unfold seenTrees
    refine mapM_some' fun x hx => ?_
    obtain ⟨a, t⟩ := x
    cases a with
    | ref r =>
      have hra : Value.ref r ∈ args := (List.of_mem_zip hx).1
      obtain ⟨v, hv⟩ := nativeView_deref hfacts.view r hra
      obtain ⟨tw, htw, -⟩ := tree_read hpre.fits (hderef r hra v hv)
      exact ⟨tw, htw⟩
    | _ => exact ⟨_, rfl⟩
  have hsl : seen.length = args.length := by
    rw [mapM_length' hseen, List.length_zip, hidl, Nat.min_self]
  have hvl : views.length = args.length := mapM_length' hfacts.view
  refine ⟨seen, hseen, hsl, hvl, hidl, hargfit, ?_, ?_, ?_⟩
  rotate_left
  · intro j t a st w ht ha hst hw hna
    obtain ⟨b, hb, hF⟩ := mapM_getElem_fwd hseen (i := j) (a := (a, t))
      (by simp [List.getElem?_zip_eq_some, ha, ht])
    rw [hst, Option.some.injEq] at hb
    subst hb
    obtain ⟨b', hb', hG⟩ := mapM_getElem_fwd hfacts.view ha
    rw [hw, Option.some.injEq] at hb'
    subst hb'
    cases a with
    | ref r => exact absurd rfl (hna r)
    | _ => simp_all
  · intro j t r st w ht ha hst hw
    obtain ⟨b, hb, hF⟩ := mapM_getElem_fwd hseen (i := j) (a := (.ref r, t))
      (by simp [List.getElem?_zip_eq_some, ha, ht])
    rw [hst, Option.some.injEq] at hb
    subst hb
    obtain ⟨b', hb', hG⟩ := mapM_getElem_fwd hfacts.view ha
    rw [hw, Option.some.injEq] at hb'
    subst hb'
    obtain ⟨t', ht', hft⟩ := fitsAll_getElem?_right hargfit ha
    rw [ht, Option.some.injEq] at ht'
    subst ht'
    exact ⟨ref_occ hft, hF, hderef r (List.mem_of_getElem? ha) _ hG⟩
  · intro j st w hst hw
    have hj : j < args.length := by
      have := (List.getElem?_eq_some_iff.1 hst).1; omega
    obtain ⟨a, ha⟩ : ∃ a, args[j]? = some a := ⟨_, List.getElem?_eq_getElem hj⟩
    obtain ⟨t, ht, hft⟩ := fitsAll_getElem?_right hargfit ha
    obtain ⟨b, hb, hF⟩ := mapM_getElem_fwd hseen (i := j) (a := (a, t))
      (by simp [List.getElem?_zip_eq_some, ha, ht])
    rw [hst, Option.some.injEq] at hb
    subst hb
    obtain ⟨b', hb', hG⟩ := mapM_getElem_fwd hfacts.view ha
    rw [hw, Option.some.injEq] at hb'
    subst hb'
    cases a with
    | ref r =>
      obtain ⟨tw, htw, hfw, -⟩ := tree_read hpre.fits (hderef r (List.mem_of_getElem? ha) _ hG)
      simp only at hF
      rw [htw, Option.some.injEq] at hF
      rw [← hF]
      exact hfw
    | _ => simp_all

theorem guard_some {p : Prop} [Decidable p] (h : p) : (guard p : Option Unit) = some () := by
  simp [guard, h]

/-- **A native call is annotated** when each part of its annotation exists
(the converse of `nativeStep_spec`). -/
theorem nativeStep_of {P : Program} {ν : NativeFlows} {s : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {h : Shadow}
    {g : ShadowFrame} {hs : List ShadowFrame} {info : FunctionInfo} {tys ptys : List RuntimeType}
    {seen : List IdTree} {s' : State} {fr' : Frame} {tl : List Frame}
    {newRefs : List (Nat × Reference × Nat)} {resultTrees : List IdTree} {h₂ : Shadow}
    (hh : h.frames = g :: hs) (hinfo : P.function? f = some info) (htys : P.returnTypes f = some tys)
    (hpt : P.paramTypes f = some ptys) (hlen : args.length ≤ g.stack.length)
    (hseen : seenTrees h s args (g.stack.take args.length).reverse = some seen)
    (hsl : seen.length = views.length)
    (hguard : ((nodesOf (vals ++ writes)).all fun x =>
        match ν f views (vals ++ writes) x.1 x.2.1 with
        | .arg k q => (seenIdAt seen k q).isSome
        | .fresh => true) = true)
    (hwl : (writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)).length =
      writes.length)
    (hf' : s'.frames = fr' :: tl)
    (hrr : (refResultsOf tys (fr'.stack.take tys.length).reverse).length = places.length)
    (hnew : newRefsOf (refResultsOf tys (fr'.stack.take tys.length).reverse) places
        (g.stack.take args.length).reverse
        (outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
          (vals ++ writes) h.fresh).2.1 = some newRefs)
    (hrt : nativeResultsIn tys
        ((outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
          (vals ++ writes) h.fresh).1.take vals.length)
        (newRefs.map fun (i, _, _) => IdTree.node i []) = some resultTrees)
    (hfold : ((writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)).zip
        ((outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
          (vals ++ writes) h.fresh).1.drop vals.length)).foldlM
        (fun h x => h.withTree? s x.1.2.location x.2)
        { h with
          frames := { g with stack := resultTrees.reverse ++ g.stack.drop args.length } :: hs
          fresh := (outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
            (vals ++ writes) h.fresh).2.1 + newRefs.length } = some h₂)
    (hfits : ShadowPost.fits (.running h₂) (.running s') = true) :
    Shadow.nativeStep P ν s f args views vals writes places (.running s') h =
      some (.running h₂, nativeEventsOf (refArgsOf (g.stack.take args.length).reverse args)
        (writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)) newRefs
        (consumedOfNative P f info.moduleId ptys seen views
          (passedOn (ν f views (vals ++ writes)) (vals ++ writes)))
        (outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
          (vals ++ writes) h.fresh).2.2) := by
  unfold Shadow.nativeStep
  simp only [hh, Option.bind_eq_bind]
  rw [hinfo, Option.bind_some, htys, Option.bind_some]
  refine Option.bind_eq_some_iff.2 ⟨(), guard_some hlen, ?_⟩
  refine Option.bind_eq_some_iff.2 ⟨seen, hseen, ?_⟩
  refine Option.bind_eq_some_iff.2 ⟨(), guard_some (beq_iff_eq.2 hsl), ?_⟩
  refine Option.bind_eq_some_iff.2 ⟨(), guard_some hguard, ?_⟩
  refine Option.bind_eq_some_iff.2 ⟨ptys, hpt, ?_⟩
  refine Option.bind_eq_some_iff.2 ⟨(), guard_some (beq_iff_eq.2 hwl), ?_⟩
  simp only [hf']
  refine Option.bind_eq_some_iff.2 ⟨(), guard_some (beq_iff_eq.2 hrr), ?_⟩
  refine Option.bind_eq_some_iff.2 ⟨newRefs, hnew, ?_⟩
  refine Option.bind_eq_some_iff.2 ⟨resultTrees, hrt, ?_⟩
  refine Option.bind_eq_some_iff.2 ⟨h₂, hfold, ?_⟩
  refine Option.bind_eq_some_iff.2 ⟨(), guard_some hfits, ?_⟩
  rfl

end MoveBytecode.Proofs
