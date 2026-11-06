-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefSteps
import Proofs.Typing
import Proofs.TypingGen

/-!
# The reference-safety analysis's states are well typed

UNTRUSTED (checked by the kernel). An invariant of the abstract states the
reference-safety analysis computes, relative to the type certificate and the
availability of locals (`Typed`):

- the borrow graph is well formed, the frame root is a mutable reference, and
  every other reference of the graph is held by a local or an operand slot,
  exactly once;
- a held reference's mutability is that of the reference type of the slot
  holding it;
- an operand slot of a reference type holds a reference, and so does an
  available local of a reference type.

The join needs the first two of a block's recorded pre-state, which no
concrete state constrains; the third is what makes `CopyLoc` and `MoveLoc` of
a reference push a reference the analysis tracks.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

/-- The mutability of a reference type; `none` for a value type. -/
def _root_.MoveBytecode.RuntimeType.refMut : RuntimeType → Option Bool
  | .ref m _ => some m
  | .bool | .int _ | .address | .signer | .vector _ | .struct _ | .function .. => none

/-- The references the abstract state's locals and operand slots hold. -/
def heldIds (st : RefState) : List Nat :=
  st.locals.filterMap AbsValue.refId? ++ st.stack.filterMap AbsValue.refId?

/-- **Well-typed abstract states**, for local types `ltys`, operand-stack
types `stys` (top first) and availability `avail`. -/
structure Typed (ltys stys : List SignatureToken) (avail : LocalIndex → Availability)
    (st : RefState) : Prop where
  wf : st.graph.WF
  back : st.graph.Back
  rootMut : st.graph.mutability st.locals.length = some true
  nlocals : st.locals.length = ltys.length
  nstack : st.stack.length = stys.length
  keys : ∀ x, (st.graph.find? x).isSome → x = st.locals.length ∨ x ∈ heldIds st
  distinct : (heldIds st).Nodup
  rootFree : st.locals.length ∉ heldIds st
  locMut : ∀ (i x : Nat), st.locals[i]? = some (AbsValue.reference x) →
    ∃ (t : SignatureToken) (b : Bool), ltys[i]? = some t ∧ refKind t = some b ∧ st.graph.mutability x = some b
  stkMut : ∀ (k x : Nat), st.stack[k]? = some (AbsValue.reference x) →
    ∃ (t : SignatureToken) (b : Bool), stys[k]? = some t ∧ refKind t = some b ∧ st.graph.mutability x = some b
  stkKind : ∀ (k : Nat) (t : SignatureToken) (b : Bool), stys[k]? = some t → refKind t = some b →
    ∃ x, st.stack[k]? = some (AbsValue.reference x)
  locKind : ∀ (i : Nat) (t : SignatureToken) (b : Bool), ltys[i]? = some t → refKind t = some b → avail i = .available →
    ∃ x, st.locals[i]? = some (AbsValue.reference x)

theorem mem_heldIds {st : RefState} {x : Nat} :
    x ∈ heldIds st ↔ .reference x ∈ st.locals ∨ .reference x ∈ st.stack := by
  have : ∀ (l : List AbsValue), x ∈ l.filterMap AbsValue.refId? ↔ .reference x ∈ l := by
    intro l
    simp only [List.mem_filterMap]
    constructor
    · rintro ⟨v, hv, hx⟩
      cases v with
      | reference y => simp [AbsValue.refId?] at hx; subst hx; exact hv
      | nonReference => simp [AbsValue.refId?] at hx
    · intro h; exact ⟨_, h, rfl⟩
  simp [heldIds, this]

namespace Typed

variable {ltys stys : List SignatureToken} {avail : LocalIndex → Availability} {st : RefState}

/-- A held reference is a reference of the graph. -/
theorem held_key (ht : Typed ltys stys avail st) {x : Nat} (hx : x ∈ heldIds st) :
    (st.graph.find? x).isSome := by
  have hkey : ∀ b, st.graph.mutability x = some b → (st.graph.find? x).isSome := by
    intro b hm; simp only [Graph.mutability] at hm; cases h : st.graph.find? x <;> simp_all
  rcases mem_heldIds.1 hx with h | h
  · obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 h
    obtain ⟨t, b, -, -, hm⟩ := ht.locMut i x hi
    exact hkey b hm
  · obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 h
    obtain ⟨t, b, -, -, hm⟩ := ht.stkMut k x hk
    exact hkey b hm

end Typed

theorem filterMap_refId_replicate (k : Nat) :
    (List.replicate k AbsValue.nonReference).filterMap AbsValue.refId? = [] := by
  induction k with
  | zero => rfl
  | succ k ih => simp [List.replicate_succ, AbsValue.refId?, ih]

theorem filterMap_refId_drop {l : List AbsValue} {j : Nat}
    (hv : ∀ v ∈ l.take j, v = .nonReference) :
    (l.drop j).filterMap AbsValue.refId? = l.filterMap AbsValue.refId? := by
  conv => rhs; rw [← List.take_append_drop j l]
  rw [List.filterMap_append]
  have : (l.take j).filterMap AbsValue.refId? = [] := by
    rw [List.filterMap_eq_nil_iff]
    intro v hv'
    rw [hv v hv']; rfl
  rw [this, List.nil_append]

namespace Typed

variable {ltys stys : List SignatureToken} {avail : LocalIndex → Availability} {st : RefState}

/-- **A step on values**: popping `j` non-references and pushing `k` values
of value types keeps the state well typed. -/
theorem valueStep (ht : Typed ltys stys avail st) {st' : RefState} {j k : Nat}
    {P stys' : List SignatureToken} (hv : ∀ v ∈ st.stack.take j, v = .nonReference)
    (hst : st' = { st with stack := List.replicate k .nonReference ++ st.stack.drop j })
    (hP : P.length = k) (hPv : ∀ t ∈ P, refKind t = none) (hstys : stys' = P ++ stys.drop j) :
    Typed ltys stys' avail st' := by
  subst hst hstys hP
  have hheld : heldIds { st with stack := List.replicate P.length .nonReference ++ st.stack.drop j } =
      heldIds st := by
    simp only [heldIds, List.filterMap_append, filterMap_refId_replicate, List.nil_append,
      filterMap_refId_drop hv]
  have hlen := ht.nstack
  refine {
    wf := ht.wf
    back := ht.back
    rootMut := ht.rootMut
    nlocals := ht.nlocals
    nstack := by simp [hlen]
    keys := fun x hx => by rw [hheld]; exact ht.keys x hx
    distinct := by rw [hheld]; exact ht.distinct
    rootFree := by rw [hheld]; exact ht.rootFree
    locMut := ht.locMut
    stkMut := fun k' x hk => ?_
    stkKind := fun k' t b hk hb => ?_
    locKind := ht.locKind }
  · by_cases hlt : P.length ≤ k'
    · rw [List.getElem?_append_right (by simpa using hlt), List.getElem?_drop] at hk
      obtain ⟨t, b, ht', hb, hm⟩ := ht.stkMut _ x hk
      refine ⟨t, b, ?_, hb, hm⟩
      rw [List.getElem?_append_right hlt, List.getElem?_drop]
      simpa using ht'
    · rw [List.getElem?_append_left (by simpa using Nat.lt_of_not_le hlt)] at hk
      simp [List.getElem?_replicate] at hk
  · by_cases hlt : k' < P.length
    · rw [List.getElem?_append_left hlt] at hk
      rw [hPv t (List.mem_of_getElem? hk)] at hb
      cases hb
    · rw [List.getElem?_append_right (Nat.le_of_not_lt hlt), List.getElem?_drop] at hk
      obtain ⟨x, hx⟩ := ht.stkKind _ t b hk hb
      refine ⟨x, ?_⟩
      simp only
      rw [List.getElem?_append_right (by simp; omega), List.getElem?_drop]
      simpa using hx

end Typed

namespace Typed

variable {ltys stys : List SignatureToken} {avail : LocalIndex → Availability} {st : RefState}

/-- A slot of a value type holds no reference. -/
theorem nonRef_of_value (ht : Typed ltys stys avail st) {k : Nat} {t : SignatureToken}
    {v : AbsValue} (htk : stys[k]? = some t) (hk : refKind t = none) (hv : st.stack[k]? = some v) :
    v = .nonReference := by
  cases v with
  | nonReference => rfl
  | reference x =>
    obtain ⟨t', b, ht', hb, -⟩ := ht.stkMut k x hv
    rw [htk, Option.some.injEq] at ht'
    subst ht'
    rw [hk] at hb; cases hb

theorem mem_filterMap_refId {l : List AbsValue} {x : Nat} :
    x ∈ l.filterMap AbsValue.refId? ↔ AbsValue.reference x ∈ l := by
  simp only [List.mem_filterMap]
  constructor
  · rintro ⟨v, hv, hx⟩
    cases v with
    | reference y => simp [AbsValue.refId?] at hx; subst hx; exact hv
    | nonReference => simp [AbsValue.refId?] at hx
  · intro h; exact ⟨_, h, rfl⟩

/-- **Popping a reference and releasing it.** -/
theorem popRelease (ht : Typed ltys stys avail st) {id : Nat} {rest : List AbsValue} {g : Graph}
    {t : SignatureToken} {stys' : List SignatureToken} (hs : st.stack = .reference id :: rest)
    (hrel : st.graph.release id = .ok g) (hstys : stys = t :: stys') :
    Typed ltys stys' avail { st with stack := rest, graph := g } := by
  subst hstys
  obtain ⟨hwg, hgone, hmut, -, -⟩ := Graph.release_ok hrel ht.wf
  have hheld : heldIds st = st.locals.filterMap AbsValue.refId? ++
      id :: rest.filterMap AbsValue.refId? := by
    simp [heldIds, hs, AbsValue.refId?]
  have hnd := ht.distinct
  rw [hheld, List.nodup_append, List.nodup_cons] at hnd
  obtain ⟨-, ⟨hidR, -⟩, hdisj⟩ := hnd
  have hidL : id ∉ st.locals.filterMap AbsValue.refId? := fun h =>
    hdisj id h id List.mem_cons_self rfl
  have hheld' : heldIds { st with stack := rest, graph := g } =
      st.locals.filterMap AbsValue.refId? ++ rest.filterMap AbsValue.refId? := rfl
  have hidroot : st.locals.length ≠ id := by
    intro e; apply ht.rootFree; rw [hheld, e]; simp
  have hne : ∀ x, x ∈ heldIds { st with stack := rest, graph := g } → x ≠ id := by
    intro x hx e; subst e
    rw [hheld', List.mem_append] at hx
    rcases hx with hx | hx
    · exact hidL hx
    · exact hidR hx
  have hlen := ht.nstack
  rw [hs] at hlen
  refine {
    wf := hwg
    back := Graph.release_back hrel ht.back
    rootMut := by rw [hmut _ hidroot]; exact ht.rootMut
    nlocals := ht.nlocals
    nstack := by simpa using hlen
    keys := fun x hx => ?_
    distinct := ?_
    rootFree := fun h => ht.rootFree (by
      rw [hheld]; rw [hheld', List.mem_append] at h
      rcases h with h | h
      · exact List.mem_append_left _ h
      · exact List.mem_append_right _ (List.mem_cons_of_mem _ h))
    locMut := fun i x hi => ?_
    stkMut := fun k x hk => ?_
    stkKind := fun k t' b hk hb => ?_
    locKind := ht.locKind }
  · have hxid : x ≠ id := fun e => by subst e; rw [hgone] at hx; cases hx
    rw [Graph.isSome_of_mutability (hmut x hxid)] at hx
    rcases ht.keys x hx with h | h
    · exact Or.inl h
    · right
      rw [hheld, List.mem_append, List.mem_cons] at h
      rw [hheld', List.mem_append]
      rcases h with h | h | h
      · exact Or.inl h
      · exact absurd h hxid
      · exact Or.inr h
  · rw [hheld']
    have := ht.distinct
    rw [hheld] at this
    exact List.Nodup.sublist (List.Sublist.append_left (List.sublist_cons_self _ _) _) this
  · obtain ⟨t', b, h1, h2, h3⟩ := ht.locMut i x hi
    have hx : x ≠ id := hne x (by
      rw [hheld', List.mem_append]; exact Or.inl (mem_filterMap_refId.2 (List.mem_of_getElem? hi)))
    exact ⟨t', b, h1, h2, by rw [hmut x hx]; exact h3⟩
  · have hk' : st.stack[k + 1]? = some (AbsValue.reference x) := by rw [hs]; simpa using hk
    obtain ⟨t', b, h1, h2, h3⟩ := ht.stkMut (k + 1) x hk'
    have hx : x ≠ id := hne x (by
      rw [hheld', List.mem_append]
      exact Or.inr (mem_filterMap_refId.2 (List.mem_of_getElem? (by simpa using hk))))
    exact ⟨t', b, by simpa using h1, h2, by rw [hmut x hx]; exact h3⟩
  · obtain ⟨x, hx⟩ := ht.stkKind (k + 1) t' b (by simpa using hk) hb
    rw [hs] at hx
    exact ⟨x, by simpa using hx⟩

end Typed

namespace Typed

variable {ltys stys : List SignatureToken} {avail : LocalIndex → Availability} {st : RefState}

/-- **Pushing a new reference** `c` of mutability `m`, at a reference type `t`
of that mutability, after graph operations that keep the references and
their mutability. -/
theorem pushNew (ht : Typed ltys stys avail st) {c : Nat} {m : Bool} {g1 g2 : Graph}
    {t : SignatureToken} (hnew : st.graph.newRef c m = .ok g1) (hw2 : g2.WF) (hb2 : g2.Back)
    (hmut2 : ∀ x, g2.mutability x = g1.mutability x) (hm : refKind t = some m) :
    Typed ltys (t :: stys) avail
      { st with graph := g2, stack := .reference c :: st.stack } := by
  have hcfresh := (Graph.newRef_ok hnew).1
  have hcheld : c ∉ heldIds st := fun h => by
    have := ht.held_key h; rw [hcfresh] at this; cases this
  have hroot : st.locals.length ≠ c := fun e => by
    have := ht.rootMut; rw [e] at this
    simp [Graph.mutability, hcfresh] at this
  have hheld' : heldIds { st with graph := g2, stack := .reference c :: st.stack } =
      st.locals.filterMap AbsValue.refId? ++ c :: st.stack.filterMap AbsValue.refId? := by
    simp [heldIds, AbsValue.refId?]
  have hm' : ∀ x, x ≠ c → g2.mutability x = st.graph.mutability x := fun x hx => by
    rw [hmut2, Graph.newRef_mutability hnew x hx]
  have hheldne : ∀ x, x ∈ heldIds st → x ≠ c := fun x hx e => hcheld (e ▸ hx)
  refine {
    wf := hw2
    back := hb2
    rootMut := by rw [hm' _ hroot]; exact ht.rootMut
    nlocals := ht.nlocals
    nstack := by simp [ht.nstack]
    keys := fun x hx => ?_
    distinct := ?_
    rootFree := ?_
    locMut := fun i x hi => ?_
    stkMut := fun k x hk => ?_
    stkKind := fun k t' b hk hb => ?_
    locKind := ht.locKind }
  · rw [hheld']
    by_cases hxc : x = c
    · exact Or.inr (by simp [hxc])
    · rw [Graph.isSome_of_mutability (hm' x hxc)] at hx
      rcases ht.keys x hx with h | h
      · exact Or.inl h
      · right
        simp only [heldIds, List.mem_append] at h
        simp only [List.mem_append, List.mem_cons]
        rcases h with h | h
        · exact Or.inl h
        · exact Or.inr (Or.inr h)
  · rw [hheld', List.nodup_append, List.nodup_cons]
    have hnd := ht.distinct
    simp only [heldIds, List.nodup_append] at hnd
    obtain ⟨h1, h2, h3⟩ := hnd
    refine ⟨h1, ⟨fun h => hcheld (by simp [heldIds, h]), h2⟩, fun a ha b hb => ?_⟩
    rcases List.mem_cons.1 hb with rfl | hb
    · exact fun e => hcheld (by simp [heldIds, e ▸ ha])
    · exact h3 a ha b hb
  · rw [hheld']
    intro h
    simp only [List.mem_append, List.mem_cons] at h
    rcases h with h | h | h
    · exact ht.rootFree (by simp [heldIds, h])
    · exact hroot h
    · exact ht.rootFree (by simp [heldIds, h])
  · obtain ⟨t', b, h1, h2, h3⟩ := ht.locMut i x hi
    have hx : x ≠ c := hheldne x (by
      simp only [heldIds, List.mem_append]
      exact Or.inl (mem_filterMap_refId.2 (List.mem_of_getElem? hi)))
    exact ⟨t', b, h1, h2, by rw [hm' x hx]; exact h3⟩
  · cases k with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq, AbsValue.reference.injEq] at hk
      subst hk
      exact ⟨t, m, rfl, hm, by rw [hmut2, Graph.newRef_mutability_self hnew]⟩
    | succ k =>
      simp only [List.getElem?_cons_succ] at hk
      obtain ⟨t', b, h1, h2, h3⟩ := ht.stkMut k x hk
      have hx : x ≠ c := hheldne x (by
        simp only [heldIds, List.mem_append]
        exact Or.inr (mem_filterMap_refId.2 (List.mem_of_getElem? hk)))
      exact ⟨t', b, by simpa using h1, h2, by rw [hm' x hx]; exact h3⟩
  · cases k with
    | zero => exact ⟨c, rfl⟩
    | succ k =>
      simp only [List.getElem?_cons_succ] at hk
      obtain ⟨x, hx⟩ := ht.stkKind k t' b hk hb
      exact ⟨x, by simpa using hx⟩

end Typed

theorem getElem?_append_skip {α : Type} (pre rest : List α) (a : α) (k : Nat) :
    (pre ++ rest)[k]? = (pre ++ a :: rest)[if k < pre.length then k else k + 1]? := by
  split
  · next h => rw [List.getElem?_append_left h, List.getElem?_append_left h]
  · next h =>
    rw [List.getElem?_append_right (by omega), List.getElem?_append_right (by omega)]
    have : k + 1 - pre.length = (k - pre.length) + 1 := by omega
    rw [this, List.getElem?_cons_succ]

namespace Typed

variable {ltys stys : List SignatureToken} {avail : LocalIndex → Availability} {st : RefState}

/-- Only the locals, the graph and the operand stack matter. -/
theorem ext (ht : Typed ltys stys avail st) {st' : RefState} (hl : st'.locals = st.locals)
    (hg : st'.graph = st.graph) (hs : st'.stack = st.stack) : Typed ltys stys avail st' := by
  obtain ⟨l, g, n, s⟩ := st
  obtain ⟨l', g', n', s'⟩ := st'
  simp only at hl hg hs
  subst hl hg hs
  exact ⟨ht.wf, ht.back, ht.rootMut, ht.nlocals, ht.nstack, ht.keys, ht.distinct, ht.rootFree,
    ht.locMut, ht.stkMut, ht.stkKind, ht.locKind⟩

/-- **The graph part of the invariant** after a step that drops the
references `D` from the slots and from the graph, keeping the others. -/
theorem held_transfer (ht : Typed ltys stys avail st) {st' : RefState} {D : List Nat}
    (hperm : (heldIds st' ++ D).Perm (heldIds st))
    (hlen : st'.locals.length = st.locals.length)
    (hgone : ∀ y ∈ D, st'.graph.find? y = none)
    (hmut : ∀ x, x ∉ D → st'.graph.mutability x = st.graph.mutability x) :
    st'.graph.mutability st'.locals.length = some true ∧
      (∀ x, (st'.graph.find? x).isSome → x = st'.locals.length ∨ x ∈ heldIds st') ∧
      (heldIds st').Nodup ∧ st'.locals.length ∉ heldIds st' ∧
      (∀ x ∈ heldIds st', st'.graph.mutability x = st.graph.mutability x) := by
  have hnd : (heldIds st' ++ D).Nodup := hperm.nodup_iff.2 ht.distinct
  have hsub : ∀ x, x ∈ heldIds st' ++ D → x ∈ heldIds st := fun x hx => hperm.mem_iff.1 hx
  have hdisj : ∀ x ∈ heldIds st', x ∉ D := by
    intro x hx hD
    rw [List.nodup_append] at hnd
    exact hnd.2.2 x hx x hD rfl
  have hrootD : st.locals.length ∉ D := fun h =>
    ht.rootFree (hsub _ (List.mem_append_right _ h))
  rw [hlen]
  refine ⟨by rw [hmut _ hrootD]; exact ht.rootMut, fun x hx => ?_, ?_, fun h => ?_,
    fun x hx => hmut x (hdisj x hx)⟩
  · have hxD : x ∉ D := fun h => by rw [hgone x h] at hx; cases hx
    rw [Graph.isSome_of_mutability (hmut x hxD)] at hx
    rcases ht.keys x hx with h | h
    · exact Or.inl h
    · right
      rcases List.mem_append.1 (hperm.mem_iff.2 h) with h | h
      · exact h
      · exact absurd h hxD
  · exact (List.nodup_append.1 hnd).1
  · exact ht.rootFree (hsub _ (List.mem_append_left _ h))

/-- **Removing a reference from the operand stack and releasing it**, at
any depth. -/
theorem releaseAt (ht : Typed ltys stys avail st) {pre rest : List AbsValue} {id : Nat}
    {g : Graph} {tpre stys' : List SignatureToken} {t : SignatureToken}
    (hs : st.stack = pre ++ .reference id :: rest) (hrel : st.graph.release id = .ok g)
    (hstys : stys = tpre ++ t :: stys') (hlen : pre.length = tpre.length) :
    Typed ltys (tpre ++ stys') avail { st with stack := pre ++ rest, graph := g } := by
  obtain ⟨hwg, hgone, hmut, -, -⟩ := Graph.release_ok hrel ht.wf
  have hperm : (heldIds { st with stack := pre ++ rest, graph := g } ++ [id]).Perm
      (heldIds st) := by
    simp only [heldIds, hs, List.filterMap_append, List.filterMap_cons, AbsValue.refId?,
      List.append_assoc]
    refine List.Perm.append_left _ ?_
    refine List.Perm.append_left _ ?_
    exact List.perm_middle.trans (by simp)
  obtain ⟨hroot, hkeys, hnd, hfree, hheld⟩ :=
    held_transfer (st' := { st with stack := pre ++ rest, graph := g }) ht hperm rfl
      (fun y hy => by simp at hy; subst hy; exact hgone)
      (fun x hx => hmut x (by simpa using hx))
  have hidx : ∀ k, (pre ++ rest)[k]? = st.stack[if k < pre.length then k else k + 1]? := by
    intro k; rw [hs]; exact getElem?_append_skip _ _ _ _
  have hidxT : ∀ k, (tpre ++ stys')[k]? = stys[if k < pre.length then k else k + 1]? := by
    intro k; rw [hstys, hlen]; exact getElem?_append_skip _ _ _ _
  have hlenS := ht.nstack
  rw [hs, hstys] at hlenS
  simp only [List.length_append, List.length_cons] at hlenS
  refine {
    wf := hwg
    back := Graph.release_back hrel ht.back
    rootMut := hroot
    nlocals := ht.nlocals
    nstack := by simp only [List.length_append]; omega
    keys := hkeys
    distinct := hnd
    rootFree := hfree
    locMut := fun i x hi => ?_
    stkMut := fun k x hk => ?_
    stkKind := fun k t' b hk hb => ?_
    locKind := ht.locKind }
  · obtain ⟨t', b, h1, h2, h3⟩ := ht.locMut i x hi
    refine ⟨t', b, h1, h2, ?_⟩
    rw [hheld x (mem_heldIds.2 (Or.inl (List.mem_of_getElem? hi)))]
    exact h3
  · have hk' := hk
    simp only at hk'
    rw [hidx] at hk'
    obtain ⟨t', b, h1, h2, h3⟩ := ht.stkMut _ x hk'
    refine ⟨t', b, by rw [hidxT]; exact h1, h2, ?_⟩
    rw [hheld x (mem_heldIds.2 (Or.inr (List.mem_of_getElem? hk)))]
    exact h3
  · rw [hidxT] at hk
    obtain ⟨x, hx⟩ := ht.stkKind _ t' b hk hb
    exact ⟨x, by simp only; rw [hidx]; exact hx⟩

/-- **A step on values**, with the counts of the analysis. -/
theorem ofValueStep (ht : Typed ltys stys avail st) {st' : RefState} {j k : Nat}
    {P stys' : List SignatureToken} (hvs : ValueStep j k st st') (hP : P.length = k)
    (hPv : ∀ t ∈ P, refKind t = none) (hstys : stys' = P ++ stys.drop j) :
    Typed ltys stys' avail st' :=
  ht.valueStep hvs.2.1 hvs.2.2 hP hPv hstys

/-- The operand on top, of a value type, retyped as another value type. -/
theorem retype (ht : Typed ltys stys avail st) {t t' : SignatureToken}
    {rest : List SignatureToken} (hstys : stys = t :: rest) (ht1 : refKind t = none)
    (ht2 : refKind t' = none) : Typed ltys (t' :: rest) avail st := by
  have hlen := ht.nstack
  rw [hstys] at hlen
  obtain ⟨v, vs, hv⟩ : ∃ v vs, st.stack = v :: vs := by
    cases h : st.stack with
    | nil => rw [h] at hlen; simp at hlen
    | cons v vs => exact ⟨v, vs, rfl⟩
  have hv0 : v = .nonReference := ht.nonRef_of_value (k := 0) (by rw [hstys]; rfl) ht1
    (by rw [hv]; rfl)
  subst hv0
  exact ht.valueStep (j := 1) (k := 1) (P := [t']) (by simp [hv])
    (by cases st; simp only at hv; subst hv; simp) rfl (by simpa using ht2) (by simp [hstys])

end Typed

theorem count_filterMap_set {l : List AbsValue} {i : Nat} {v : AbsValue}
    (hl : l[i]? = some v) (w : AbsValue) (a : Nat) :
    ((l.set i w).filterMap AbsValue.refId?).count a + (AbsValue.refId? v).toList.count a =
      (l.filterMap AbsValue.refId?).count a + (AbsValue.refId? w).toList.count a := by
  obtain ⟨hi, hv⟩ := List.getElem?_eq_some_iff.1 hl
  have hsplit : l = l.take i ++ v :: l.drop (i + 1) := by
    conv => lhs; rw [← List.take_append_drop i l]
    rw [← List.getElem_cons_drop hi, hv]
  have hset : l.set i w = l.take i ++ w :: l.drop (i + 1) := by
    rw [List.set_eq_take_append_cons_drop, if_pos hi]
  rw [hset]
  conv => rhs; rw [hsplit]
  cases v <;> cases w <;>
    simp [List.filterMap_append, AbsValue.refId?, List.count_append,
      List.count_cons] <;> omega

theorem perm_of_count {l₁ l₂ : List Nat} (h : ∀ a, l₁.count a = l₂.count a) : l₁.Perm l₂ :=
  List.perm_iff_count.2 h

/-- `isAssignableFrom` keeps reference kinds. -/
theorem refKind_of_assignable {t op : SignatureToken} (h : isAssignableFrom t op = true) :
    refKind t = refKind op := by
  cases t <;> cases op <;> simp_all [isAssignableFrom, refKind]

namespace Typed

variable {ltys stys : List SignatureToken} {avail : LocalIndex → Availability} {st : RefState}

/-- **`MoveLoc`**: the local's value moves onto the operand stack. -/
theorem moveLocal (ht : Typed ltys stys avail st) {i : Nat} {v : AbsValue} {t : SignatureToken}
    (hloc : st.locals[i]? = some v) (hti : ltys[i]? = some t) (hav : avail i = .available) :
    Typed ltys (t :: stys) (fun j => if j = i then .unavailable else avail j)
      { st with locals := st.locals.set i .nonReference, stack := v :: st.stack } := by
  have hperm : List.Perm
      (heldIds { st with locals := st.locals.set i .nonReference, stack := v :: st.stack } ++ [])
      (heldIds st) := by
    refine perm_of_count fun a => ?_
    have := count_filterMap_set hloc .nonReference a
    cases v <;> simp_all [heldIds, List.count_append, AbsValue.refId?,
      List.count_cons] <;> omega
  obtain ⟨hroot, hkeys, hnd, hfree, -⟩ := held_transfer
    (st' := { st with locals := st.locals.set i .nonReference, stack := v :: st.stack })
    ht hperm (by simp) (by simp) (fun _ _ => rfl)
  refine {
    wf := ht.wf
    back := ht.back
    rootMut := hroot
    nlocals := by simp [ht.nlocals]
    nstack := by simp [ht.nstack]
    keys := hkeys
    distinct := hnd
    rootFree := hfree
    locMut := fun j x hj => ?_
    stkMut := fun k x hk => ?_
    stkKind := fun k t' b hk hb => ?_
    locKind := fun j t' b hj hb hav' => ?_ }
  · simp only [List.getElem?_set] at hj
    split at hj
    · split at hj <;> cases hj
    · exact ht.locMut j x hj
  · cases k with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hk
      subst hk
      obtain ⟨t'', b, h1, h2, h3⟩ := ht.locMut i x hloc
      rw [hti, Option.some.injEq] at h1
      subst h1
      exact ⟨_, b, rfl, h2, h3⟩
    | succ k =>
      obtain ⟨t', b, h1, h2, h3⟩ := ht.stkMut k x (by simpa using hk)
      exact ⟨t', b, by simpa using h1, h2, h3⟩
  · cases k with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hk
      rw [← hk] at hb
      obtain ⟨x, hx⟩ := ht.locKind i t b hti hb hav
      rw [hloc, Option.some.injEq] at hx
      exact ⟨x, by simp [hx]⟩
    | succ k =>
      obtain ⟨x, hx⟩ := ht.stkKind k t' b (by simpa using hk) hb
      exact ⟨x, by simpa using hx⟩
  · have hji : j ≠ i := fun e => by subst e; simp at hav'
    simp only [hji, if_false] at hav'
    obtain ⟨x, hx⟩ := ht.locKind j t' b hj hb hav'
    exact ⟨x, by simp only [List.getElem?_set, Ne.symm hji, if_false]; exact hx⟩

/-- **`StLoc`**: the operand on top moves into the local, whose old value is
dropped: `g` is the graph with the old reference, if any, released. -/
theorem storeLocal (ht : Typed ltys stys avail st) {i : Nat} {v old : AbsValue}
    {rest : List AbsValue} {t op : SignatureToken} {stys' : List SignatureToken} {g : Graph}
    (hs : st.stack = v :: rest) (hloc : st.locals[i]? = some old) (hti : ltys[i]? = some t)
    (hstys : stys = op :: stys') (hassign : isAssignableFrom t op = true)
    (hwf : g.WF) (hback : g.Back)
    (hgone : ∀ y ∈ (AbsValue.refId? old).toList, g.find? y = none)
    (hmut : ∀ x, x ∉ (AbsValue.refId? old).toList → g.mutability x = st.graph.mutability x) :
    Typed ltys stys' (fun j => if j = i then .available else avail j)
      { st with locals := st.locals.set i v, stack := rest, graph := g } := by
  have hperm : (heldIds { st with locals := st.locals.set i v, stack := rest, graph := g } ++
      (AbsValue.refId? old).toList).Perm (heldIds st) := by
    refine perm_of_count fun a => ?_
    have := count_filterMap_set hloc v a
    simp only [heldIds, List.count_append, hs] at this ⊢
    cases v <;> simp_all [AbsValue.refId?, List.count_cons] <;> omega
  obtain ⟨hroot, hkeys, hnd, hfree, hheld⟩ := held_transfer
    (st' := { st with locals := st.locals.set i v, stack := rest, graph := g })
    ht hperm (by simp) hgone hmut
  have hlen := ht.nstack
  rw [hs, hstys] at hlen
  have hkind := refKind_of_assignable hassign
  have hheld' : ∀ x, AbsValue.reference x ∈ st.locals.set i v ∨ AbsValue.reference x ∈ rest →
      g.mutability x = st.graph.mutability x := fun x hx => hheld x (mem_heldIds.2 hx)
  refine {
    wf := hwf
    back := hback
    rootMut := hroot
    nlocals := by simp [ht.nlocals]
    nstack := by simpa using hlen
    keys := hkeys
    distinct := hnd
    rootFree := hfree
    locMut := fun j x hj => ?_
    stkMut := fun k x hk => ?_
    stkKind := fun k t' b hk hb => ?_
    locKind := fun j t' b hj hb hav' => ?_ }
  · rw [hheld' x (Or.inl (List.mem_of_getElem? hj))]
    simp only [List.getElem?_set] at hj
    split at hj
    · next hji =>
      subst hji
      split at hj
      · simp only [Option.some.injEq] at hj
        subst hj
        obtain ⟨t', b, h1, h2, h3⟩ := ht.stkMut 0 x (by simp [hs])
        rw [hstys] at h1
        simp only [List.getElem?_cons_zero, Option.some.injEq] at h1
        subst h1
        exact ⟨t, b, hti, by rw [hkind]; exact h2, h3⟩
      · cases hj
    · exact ht.locMut j x hj
  · obtain ⟨t', b, h1, h2, h3⟩ := ht.stkMut (k + 1) x (by simpa [hs] using hk)
    rw [hheld' x (Or.inr (List.mem_of_getElem? hk))]
    exact ⟨t', b, by simpa [hstys] using h1, h2, h3⟩
  · obtain ⟨x, hx⟩ := ht.stkKind (k + 1) t' b (by simpa [hstys] using hk) hb
    exact ⟨x, by simpa [hs] using hx⟩
  · by_cases hji : j = i
    · subst hji
      rw [hti, Option.some.injEq] at hj
      subst hj
      obtain ⟨x, hx⟩ := ht.stkKind 0 op b (by simp [hstys]) (by rw [← hkind]; exact hb)
      rw [hs] at hx
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hx
      have hjl : j < st.locals.length := (List.getElem?_eq_some_iff.1 hloc).1
      exact ⟨x, by simp [hjl, hx]⟩
    · simp only [hji, if_false] at hav'
      obtain ⟨x, hx⟩ := ht.locKind j t' b hj hb hav'
      exact ⟨x, by simp only [List.getElem?_set, Ne.symm hji, if_false]; exact hx⟩

end Typed

end MoveBytecode.Proofs
