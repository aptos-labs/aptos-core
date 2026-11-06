-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefTypedStep

/-!
# Canonical states and joins of the reference-safety analysis

UNTRUSTED (checked by the kernel). At the end of each block the analysis puts
its state in canonical form (`canonicalize`): the reference in local `i` is
renamed `i`, the frame root keeps its number, and the graph is renamed along.
Then it joins the state into each successor's. Both keep the abstract
invariant `Typed` of an empty operand stack.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

/-- The renaming `canonicalize` applies. -/
def canonMap (locals : List AbsValue) : NatMap Nat :=
  locals.zipIdx.foldl (fun m (v, i) => match v with
    | .reference old => NatMap.insert m old i
    | .nonReference => m) ([(locals.length, locals.length)] : NatMap Nat)

/-- The locals of the canonical state. -/
def canonLocals (locals : List AbsValue) : List AbsValue :=
  locals.zipIdx.map fun (v, i) => match v with
    | .reference _ => AbsValue.reference i
    | .nonReference => .nonReference

/-- The canonical form of a state. -/
def canonState (st : RefState) : RefState :=
  { st with
    locals := canonLocals st.locals
    graph := st.graph.remap (canonMap st.locals)
    nextId := st.locals.length + 1 }

theorem canonicalize_run {st st' : RefState} (h : canonicalize.run st = .ok ((), st')) :
    st.stack = [] ∧ st' = canonState st := by
  unfold canonicalize at h
  unfold_ref
  cases hs : st.stack with
  | cons v vs => simp [hs, StateT.lift, StateT.bind, bind, Except.bind] at h
  | nil =>
    simp only [hs, List.isEmpty_nil, Bool.not_true, Bool.false_eq_true, if_false,
      StateT.set, pure, Except.pure,
      Except.ok.injEq, Prod.mk.injEq] at h
    refine ⟨rfl, ?_⟩
    rw [← h.2]
    unfold canonState canonLocals canonMap
    rw [hs]
    rfl

/-! ## The renaming -/

section Fold

/-- One step of the fold that builds `canonMap`. -/
private theorem foldCanon_other : ∀ (l : List (AbsValue × Nat)) (m : NatMap Nat) (x : Nat),
    (∀ v i, (v, i) ∈ l → v ≠ .reference x) →
    (l.foldl (fun m (vi : AbsValue × Nat) => match vi with
      | (v, i) => match v with
        | .reference old => NatMap.insert m old i
        | .nonReference => m) m).find? x = m.find? x
  | [], m, x, _ => rfl
  | (v, j) :: rest, m, x, h => by
    simp only [List.foldl_cons]
    rw [foldCanon_other rest _ x (fun w i hw => h w i (List.mem_cons_of_mem _ hw))]
    cases v with
    | nonReference => rfl
    | reference y =>
      have hy : y ≠ x := fun e => h (.reference y) j List.mem_cons_self (by rw [e])
      simp only
      rw [NatMap.find?_insert]
      simp [Ne.symm hy]

private theorem foldCanon_ref : ∀ (l : List (AbsValue × Nat)) (m : NatMap Nat) (x i : Nat),
    (.reference x, i) ∈ l → (∀ j, (.reference x, j) ∈ l → j = i) →
    (l.foldl (fun m (vi : AbsValue × Nat) => match vi with
      | (v, i) => match v with
        | .reference old => NatMap.insert m old i
        | .nonReference => m) m).find? x = some i
  | [], m, x, i, h, _ => by simp at h
  | (v, j) :: rest, m, x, i, hmem, huniq => by
    simp only [List.foldl_cons]
    by_cases hin : (.reference x, i) ∈ rest
    · exact foldCanon_ref rest _ x i hin (fun k hk => huniq k (List.mem_cons_of_mem _ hk))
    · have hhead : (v, j) = (.reference x, i) := by
        rcases List.mem_cons.1 hmem with h | h
        · exact h.symm
        · exact absurd h hin
      simp only [Prod.mk.injEq] at hhead
      obtain ⟨rfl, rfl⟩ := hhead
      rw [foldCanon_other rest _ x (fun w k hw e => by
        subst e
        have := huniq k (List.mem_cons_of_mem _ hw)
        subst this
        exact hin hw)]
      try simp [NatMap.find?_insert]

end Fold

/-- With no reference held twice, two locals holding one reference are one
local. -/
theorem filterMap_inj {α β : Type} {f : α → Option β} :
    ∀ {l : List α}, (l.filterMap f).Nodup → ∀ {i j : Nat} {a : α} {b : β},
      l[i]? = some a → l[j]? = some a → f a = some b → i = j
  | [], _, i, j, a, b, hi, _, _ => by simp at hi
  | v :: rest, hnd, i, j, a, b, hi, hj, hf => by
    have hnd' : (rest.filterMap f).Nodup := by
      rw [List.filterMap_cons] at hnd
      split at hnd
      · exact hnd
      · exact (List.nodup_cons.1 hnd).2
    have hnot : ∀ k : Nat, i = 0 → rest[k]? = some a → False := by
      intro k hi0 hk
      subst hi0
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hi
      subst hi
      rw [List.filterMap_cons, hf] at hnd
      exact (List.nodup_cons.1 hnd).1 (List.mem_filterMap.2 ⟨v, List.mem_of_getElem? hk, hf⟩)
    cases i with
    | zero =>
      cases j with
      | zero => rfl
      | succ j => exact (hnot j rfl (by simpa using hj)).elim
    | succ i =>
      cases j with
      | zero =>
        have hnot' : ∀ k : Nat, rest[k]? = some a → False := by
          intro k hk
          simp only [List.getElem?_cons_zero, Option.some.injEq] at hj
          subst hj
          rw [List.filterMap_cons, hf] at hnd
          exact (List.nodup_cons.1 hnd).1 (List.mem_filterMap.2 ⟨v, List.mem_of_getElem? hk, hf⟩)
        exact (hnot' i (by simpa using hi)).elim
      | succ j =>
        have := filterMap_inj hnd' (by simpa using hi) (by simpa using hj) hf
        omega

theorem mem_zipIdx_iff {l : List AbsValue} {v : AbsValue} {i : Nat} :
    (v, i) ∈ l.zipIdx ↔ l[i]? = some v := by
  rw [List.mem_zipIdx_iff_getElem?]

/-- The renaming sends the reference of local `i` to `i`. -/
theorem canonMap_ref {locals : List AbsValue} (hnd : (locals.filterMap AbsValue.refId?).Nodup)
    {i x : Nat} (hi : locals[i]? = some (AbsValue.reference x)) :
    (canonMap locals).find? x = some i := by
  unfold canonMap
  refine foldCanon_ref _ _ x i (mem_zipIdx_iff.2 hi) fun j hj => ?_
  exact filterMap_inj hnd (mem_zipIdx_iff.1 hj) hi rfl

theorem canonMap_root {locals : List AbsValue}
    (hroot : ∀ i : Nat, locals[i]? ≠ some (AbsValue.reference locals.length)) :
    (canonMap locals).find? locals.length = some locals.length := by
  unfold canonMap
  rw [foldCanon_other _ _ _ (fun v i hv e => hroot i (by rw [← e]; exact mem_zipIdx_iff.1 hv))]
  simp [NatMap.find?, List.lookup]

theorem canonLocals_getElem? (l : List AbsValue) (i : Nat) :
    (canonLocals l)[i]? = l[i]?.map (fun v => match v with
      | .reference _ => AbsValue.reference i
      | .nonReference => .nonReference) := by
  simp only [canonLocals, List.getElem?_map, List.getElem?_zipIdx, Nat.zero_add, Option.map_map]
  rfl

theorem canonLocals_ref {l : List AbsValue} {i x : Nat} :
    (canonLocals l)[i]? = some (AbsValue.reference x) ↔ x = i ∧ ∃ old, l[i]? = some (AbsValue.reference old) := by
  rw [canonLocals_getElem?]
  cases h : l[i]? with
  | none => simp
  | some v =>
    cases v with
    | nonReference => simp
    | reference old => simp [eq_comm]

theorem canonLocals_length (l : List AbsValue) : (canonLocals l).length = l.length := by
  simp [canonLocals]

theorem filterMap_sublist_map {α β : Type} {f : α → Option β} {h : α → β}
    (hf : ∀ a b, f a = some b → b = h a) : ∀ l : List α, (l.filterMap f).Sublist (l.map h)
  | [] => by simp
  | a :: l => by
    rw [List.filterMap_cons, List.map_cons]
    split
    · exact (filterMap_sublist_map hf l).cons _
    · next b hb =>
      rw [hf a b hb]
      exact (filterMap_sublist_map hf l).cons_cons _

theorem canonLocals_nodup (l : List AbsValue) :
    ((canonLocals l).filterMap AbsValue.refId?).Nodup := by
  unfold canonLocals
  rw [List.filterMap_map]
  refine (filterMap_sublist_map (h := Prod.snd) (fun a b hab => ?_) _).nodup ?_
  · obtain ⟨v, i⟩ := a
    cases v <;> simp_all [AbsValue.refId?]
  · rw [List.zipIdx_map_snd]
    exact List.nodup_range' _

namespace Typed

variable {ltys : List SignatureToken} {avail : LocalIndex → Availability} {st : RefState}

theorem held_local (ht : Typed ltys [] avail st) {x : Nat} :
    x ∈ heldIds st ↔ ∃ i : Nat, st.locals[i]? = some (AbsValue.reference x) := by
  have hs : st.stack = [] := List.eq_nil_of_length_eq_zero (by simpa using ht.nstack)
  rw [mem_heldIds, hs]
  simp [List.mem_iff_getElem?]

/-- **Canonical form** keeps an empty-stack state well typed. -/
theorem canonical (ht : Typed ltys [] avail st) : Typed ltys [] avail (canonState st) := by
  have hs : st.stack = [] := List.eq_nil_of_length_eq_zero (by simpa using ht.nstack)
  have hnd : (st.locals.filterMap AbsValue.refId?).Nodup := by
    have := ht.distinct
    simpa [heldIds, hs] using this
  generalize hF : (fun id => ((canonMap st.locals).find? id).getD id) = F
  have hFx : ∀ x, ((canonMap st.locals).find? x).getD x = F x := fun x => by rw [← hF]
  have hFref : ∀ (i x : Nat), st.locals[i]? = some (AbsValue.reference x) → F x = i := fun i x hi => by
    rw [← hFx, canonMap_ref hnd hi]; rfl
  have hroot : ∀ i : Nat, st.locals[i]? ≠ some (AbsValue.reference st.locals.length) := fun i hi =>
    ht.rootFree ((ht.held_local).2 ⟨i, hi⟩)
  have hFroot : F st.locals.length = st.locals.length := by
    rw [← hFx, canonMap_root hroot]; rfl
  have hkey : ∀ x, (st.graph.find? x).isSome →
      x = st.locals.length ∨ ∃ i : Nat, st.locals[i]? = some (AbsValue.reference x) := fun x hx => by
    rcases ht.keys x hx with h | h
    · exact Or.inl h
    · exact Or.inr ((ht.held_local).1 h)
  have hinj : ∀ x y xi yi, st.graph.find? x = some xi → st.graph.find? y = some yi →
      ((canonMap st.locals).find? x).getD x = ((canonMap st.locals).find? y).getD y → x = y := by
    intro x y xi yi hx hy hxy
    rw [hFx, hFx] at hxy
    have hlt : ∀ (i x : Nat), st.locals[i]? = some (AbsValue.reference x) → i < st.locals.length :=
      fun i x hi => (List.getElem?_eq_some_iff.1 hi).1
    rcases hkey x (by simp [hx]) with rfl | ⟨i, hi⟩ <;>
      rcases hkey y (by simp [hy]) with rfl | ⟨j, hj⟩
    · rfl
    · rw [hFroot, hFref j y hj] at hxy; have := hlt j y hj; omega
    · rw [hFroot, hFref i x hi] at hxy; have := hlt i x hi; omega
    · rw [hFref i x hi, hFref j y hj] at hxy
      subst hxy
      rw [hi, Option.some.injEq, AbsValue.reference.injEq] at hj
      exact hj
  obtain ⟨h1, h2, -, hw, hb⟩ := Graph.remap_ok ht.wf ht.back hinj
  simp only [hFx] at h1 h2
  have hmutF : ∀ x, (st.graph.find? x).isSome →
      (st.graph.remap (canonMap st.locals)).mutability (F x) = st.graph.mutability x := by
    intro x hx
    obtain ⟨xi, hxi⟩ := Option.isSome_iff_exists.1 hx
    simp [Graph.mutability, h1 x xi hxi, hxi, Graph.renameInfo]
  have hkeyOf : ∀ x b, st.graph.mutability x = some b → (st.graph.find? x).isSome := by
    intro x b hm
    simp only [Graph.mutability] at hm
    cases h : st.graph.find? x <;> simp_all
  have hheld' : ∀ x, x ∈ heldIds (canonState st) ↔
      ∃ old, st.locals[x]? = some (AbsValue.reference old) := by
    intro x
    rw [mem_heldIds]
    simp only [canonState, hs, List.not_mem_nil, or_false]
    rw [List.mem_iff_getElem?]
    constructor
    · rintro ⟨i, hi⟩
      obtain ⟨rfl, h⟩ := canonLocals_ref.1 hi
      exact h
    · intro h
      exact ⟨x, canonLocals_ref.2 ⟨rfl, h⟩⟩
  refine {
    wf := hw
    back := hb
    rootMut := ?_
    nlocals := by simp [canonState, canonLocals_length, ht.nlocals]
    nstack := by simp [canonState, hs]
    keys := fun y hy => ?_
    distinct := ?_
    rootFree := ?_
    locMut := fun i x hi => ?_
    stkMut := fun k x hk => by simp [canonState, hs] at hk
    stkKind := fun k t b hk _ => by simp at hk
    locKind := fun i t b hi hb' hav => ?_ }
  · simp only [canonState, canonLocals_length]
    rw [← hFroot, hmutF _ (hkeyOf _ _ ht.rootMut)]
    exact ht.rootMut
  · obtain ⟨v, hv⟩ := Option.isSome_iff_exists.1 hy
    obtain ⟨x, xi, hxi, rfl, -⟩ := h2 y v hv
    simp only [canonState, canonLocals_length]
    rcases hkey x (by simp [hxi]) with rfl | ⟨i, hi⟩
    · exact Or.inl hFroot
    · right
      rw [hFref i x hi]
      exact (hheld' i).2 ⟨x, hi⟩
  · simpa [heldIds, canonState, hs] using canonLocals_nodup st.locals
  · intro h
    obtain ⟨old, hold⟩ := (hheld' _).1 h
    have := (List.getElem?_eq_some_iff.1 hold).1
    simp [canonState, canonLocals_length] at this
  · obtain ⟨rfl, old, hold⟩ := canonLocals_ref.1 hi
    obtain ⟨t, b, h1', h2', h3'⟩ := ht.locMut x old hold
    refine ⟨t, b, h1', h2', ?_⟩
    simp only [canonState]
    rw [← hFref x old hold, hmutF _ (hkeyOf _ _ h3')]
    exact h3'
  · obtain ⟨old, hold⟩ := ht.locKind i t b hi hb' hav
    exact ⟨i, canonLocals_ref.2 ⟨rfl, old, hold⟩⟩

end Typed

namespace Typed

variable {ltys : List SignatureToken} {avail : LocalIndex → Availability} {st : RefState}

/-- **Dropping a local's reference**: the join turns a local holding a
reference on one side only into a value, and releases the reference. -/
theorem dropLocal (ht : Typed ltys [] avail st) {i x : Nat} {g : Graph}
    (hi : st.locals[i]? = some (AbsValue.reference x)) (hrel : st.graph.release x = .ok g)
    (hav : ∀ t b, ltys[i]? = some t → refKind t = some b → avail i ≠ .available) :
    Typed ltys [] avail { st with locals := st.locals.set i .nonReference, graph := g } := by
  obtain ⟨hwg, hgone, hmut, -, -⟩ := Graph.release_ok hrel ht.wf
  have hperm : List.Perm
      (heldIds { st with locals := st.locals.set i .nonReference, graph := g } ++ [x])
      (heldIds st) := by
    refine perm_of_count fun a => ?_
    have := count_filterMap_set hi .nonReference a
    simp only [AbsValue.refId?, Option.toList_some, Option.toList_none, List.count_nil] at this
    simp only [heldIds, List.count_append]
    omega
  obtain ⟨hroot, hkeys, hnd, hfree, hheld⟩ := held_transfer
    (st' := { st with locals := st.locals.set i .nonReference, graph := g })
    ht hperm (by simp) (fun y hy => by simp at hy; subst hy; exact hgone)
    (fun y hy => hmut y (by simpa using hy))
  refine {
    wf := hwg
    back := Graph.release_back hrel ht.back
    rootMut := hroot
    nlocals := by simp [ht.nlocals]
    nstack := ht.nstack
    keys := hkeys
    distinct := hnd
    rootFree := hfree
    locMut := fun j y hj => ?_
    stkMut := fun k y hk => ?_
    stkKind := fun k t b hk _ => by simp at hk
    locKind := fun j t b hj hb hav' => ?_ }
  · rw [hheld y (mem_heldIds.2 (Or.inl (List.mem_of_getElem? hj)))]
    simp only [List.getElem?_set] at hj
    split at hj
    · split at hj <;> cases hj
    · exact ht.locMut j y hj
  · have hs : st.stack = [] := List.eq_nil_of_length_eq_zero (by simpa using ht.nstack)
    simp [hs] at hk
  · by_cases hji : j = i
    · subst hji
      exact absurd hav' (hav t b hj hb)
    · obtain ⟨y, hy⟩ := ht.locKind j t b hj hb hav'
      exact ⟨y, by simp only [List.getElem?_set, Ne.symm hji, if_false]; exact hy⟩

end Typed

theorem foldlM_invariant {α β : Type} {f : β → α → VResult β} (I : β → List α → Prop)
    (hstep : ∀ acc x rest acc', I acc (x :: rest) → f acc x = .ok acc' → I acc' rest) :
    ∀ (l : List α) (acc r : β), I acc l → l.foldlM f acc = .ok r → I r []
  | [], acc, r, h, hf => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at hf
    subst hf; exact h
  | x :: l, acc, r, h, hf => by
    simp only [List.foldlM_cons, bind_ok] at hf
    obtain ⟨acc', h1, h2⟩ := hf
    exact foldlM_invariant I hstep l acc' r (hstep acc x l acc' h h1) h2

theorem zip_cons_eq {α β : Type} {l₁ : List α} {l₂ : List β} {x : α × β} {rest : List (α × β)}
    (h : l₁.zip l₂ = x :: rest) :
    ∃ a b l₁' l₂', l₁ = a :: l₁' ∧ l₂ = b :: l₂' ∧ x = (a, b) ∧ rest = l₁'.zip l₂' := by
  cases l₁ with
  | nil => simp at h
  | cons a l₁' =>
    cases l₂ with
    | nil => simp at h
    | cons b l₂' =>
      simp only [List.zip_cons_cons, List.cons.injEq] at h
      exact ⟨a, b, l₁', l₂', rfl, rfl, h.1.symm, h.2.symm⟩

/-- The join's pass over the locals: both sides stay well typed, with the
locals it builds. -/
theorem joinLocals_typed {ltys : List SignatureToken} {avail : LocalIndex → Availability}
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
    Typed ltys [] avail { a with locals := ls, graph := sg } ∧
      Typed ltys [] avail { b with locals := ls, graph := og } := by
  let I : Graph × Graph × List AbsValue → List (AbsValue × AbsValue) → Prop :=
    fun acc rest => ∃ ra rb, rest = ra.zip rb ∧ ra.length = rb.length ∧
      Typed ltys [] avail { a with locals := acc.2.2 ++ ra, graph := acc.1 } ∧
      Typed ltys [] avail { b with locals := acc.2.2 ++ rb, graph := acc.2.1 }
  have hI : I (a.graph, b.graph, []) (a.locals.zip b.locals) :=
    ⟨a.locals, b.locals, rfl, hlen, by simpa using ha.ext rfl rfl rfl,
      by simpa using hb.ext rfl rfl rfl⟩
  have hfin := foldlM_invariant I (fun acc vv rest acc' hI hf => ?_) _ _ _ hI h
  · obtain ⟨ra, rb, hz, hl, h1, h2⟩ := hfin
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
    simp only [List.append_nil] at h1 h2
    exact ⟨h1, h2⟩
  · obtain ⟨sg, og, ls⟩ := acc
    obtain ⟨v1, v2⟩ := vv
    obtain ⟨ra, rb, hz, hl, h1, h2⟩ := hI
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
        refine ⟨ra', rb', rfl, hl, ?_, ?_⟩
        · have hav : ∀ t bb, ltys[ls.length]? = some t → refKind t = some bb →
              avail ls.length ≠ .available := fun t bb ht hb' hav' => by
            obtain ⟨y, hy⟩ := h2.locKind _ t bb ht hb' hav'
            rw [show ({ b with locals := ls ++ AbsValue.nonReference :: rb', graph := og } :
              RefState).locals = ls ++ AbsValue.nonReference :: rb' from rfl] at hy
            simp at hy
          have := h1.dropLocal (i := ls.length) (x := x) (g := g) hi1 hrel hav
          exact this.ext (by simp) rfl rfl
        · exact h2.ext (by simp) rfl rfl
      | reference y =>
        simp only at hf
        split at hf
        · next heq =>
          simp only [pure, Except.pure, Except.ok.injEq] at hf
          subst hf
          simp only [beq_iff_eq, AbsValue.reference.injEq] at heq
          subst heq
          exact ⟨ra', rb', rfl, hl, h1.ext (by simp) rfl rfl, h2.ext (by simp) rfl rfl⟩
        · simp [panicAt] at hf
    | nonReference =>
      cases v2 with
      | reference y =>
        simp only [bind_ok] at hf
        obtain ⟨g, hrel, hf⟩ := hf
        simp only [pure, Except.pure, Except.ok.injEq] at hf
        subst hf
        refine ⟨ra', rb', rfl, hl, ?_, ?_⟩
        · exact h1.ext (by simp) rfl rfl
        · have hav : ∀ t bb, ltys[ls.length]? = some t → refKind t = some bb →
              avail ls.length ≠ .available := fun t bb ht hb' hav' => by
            obtain ⟨z, hz'⟩ := h1.locKind _ t bb ht hb' hav'
            rw [show ({ a with locals := ls ++ AbsValue.nonReference :: ra', graph := sg } :
              RefState).locals = ls ++ AbsValue.nonReference :: ra' from rfl] at hz'
            simp at hz'
          have := h2.dropLocal (i := ls.length) (x := y) (g := g) hi2 hrel hav
          exact this.ext (by simp) rfl rfl
      | nonReference =>
        simp only at hf
        split at hf
        · simp only [pure, Except.pure, Except.ok.injEq] at hf
          subst hf
          exact ⟨ra', rb', rfl, hl, h1.ext (by simp) rfl rfl, h2.ext (by simp) rfl rfl⟩
        · simp [panicAt] at hf


/-- What a successful join computes. -/
theorem join_run {a b j : RefState} {changed : Bool} (h : a.join b = .ok (j, changed)) :
    a.locals.length = b.locals.length ∧ a.nextId = b.nextId ∧
      ∃ sg og ls jg, (a.locals.zip b.locals).foldlM
        (fun (sg, og, ls) (v1, v2) => match v1, v2 with
          | .reference id, .nonReference => do pure (← sg.release id, og, ls ++ [AbsValue.nonReference])
          | .nonReference, .reference id => do pure (sg, ← og.release id, ls ++ [AbsValue.nonReference])
          | v1, v2 =>
            if v1 == v2 then pure (sg, og, ls ++ [v1]) else panicAt "join: different references")
        (a.graph, b.graph, []) = .ok (sg, og, ls) ∧ sg.join og = .ok jg ∧
      ((j = a ∧ changed = false ∧ a.locals = ls ∧ a.graph.leq jg = .ok true) ∨
        (j = { a with locals := ls, graph := jg } ∧ changed = true)) := by
  unfold RefState.join at h
  split at h
  · simp [panicAt, bind, Except.bind] at h
  · next hc =>
    simp only [Bool.or_eq_true, Bool.not_eq_true', Bool.and_eq_false_iff, bne_iff_ne, ne_eq,
      not_or, Decidable.not_not] at hc
    obtain ⟨⟨-, hnext⟩, hlen⟩ := hc
    refine ⟨hlen, hnext, ?_⟩
    dsimp only at h
    simp only [bind_ok] at h
    obtain ⟨⟨sg, og, ls⟩, hfold, jg, hjg, h⟩ := h
    refine ⟨sg, og, ls, jg, hfold, hjg, ?_⟩
    split at h
    · next heq =>
      simp only [beq_iff_eq] at heq
      simp only [bind_ok] at h
      obtain ⟨u, hu, h⟩ := h
      cases u
      · simp only [Bool.false_eq_true, if_false, pure, Except.pure, Except.ok.injEq,
          Prod.mk.injEq] at h
        exact Or.inr ⟨h.1.symm, h.2.symm⟩
      · simp only [if_true, pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
        exact Or.inl ⟨h.1.symm, h.2.symm, heq, hu⟩
    · simp only [pure_bind] at h
      simp only [Bool.false_eq_true, if_false, pure, Except.pure, Except.ok.injEq,
        Prod.mk.injEq] at h
      exact Or.inr ⟨h.1.symm, h.2.symm⟩

/-- **The join keeps states well typed** (`JClosed.join`). -/
theorem typed_join {ltys : List SignatureToken} {avail : LocalIndex → Availability}
    {a b j : RefState} {changed : Bool} (ha : Typed ltys [] avail a) (hb : Typed ltys [] avail b)
    (h : a.join b = .ok (j, changed)) : Typed ltys [] avail j := by
  obtain ⟨hlen, -, sg, og, ls, jg, hfold, hjg, hres⟩ := join_run h
  obtain ⟨h1, -⟩ := joinLocals_typed ha hb hlen hfold
  rcases hres with ⟨rfl, -⟩ | ⟨rfl, -⟩
  · exact ha
  · obtain ⟨hw, hbk, -, hmut, -⟩ := Graph.join_ok hjg h1.wf h1.back
    have hkey : ∀ x, (jg.find? x).isSome = (sg.find? x).isSome := fun x =>
      Graph.isSome_of_mutability (hmut x)
    exact {
      wf := hw
      back := hbk
      rootMut := by rw [hmut]; exact h1.rootMut
      nlocals := h1.nlocals
      nstack := h1.nstack
      keys := fun x hx => h1.keys x (by rw [← hkey]; exact hx)
      distinct := h1.distinct
      rootFree := h1.rootFree
      locMut := fun i x hi => by
        obtain ⟨t, bb, e1, e2, e3⟩ := h1.locMut i x hi
        exact ⟨t, bb, e1, e2, by rw [hmut]; exact e3⟩
      stkMut := fun k x hk => by
        obtain ⟨t, bb, e1, e2, e3⟩ := h1.stkMut k x hk
        exact ⟨t, bb, e1, e2, by rw [hmut]; exact e3⟩
      stkKind := h1.stkKind
      locKind := h1.locKind }

end MoveBytecode.Proofs
