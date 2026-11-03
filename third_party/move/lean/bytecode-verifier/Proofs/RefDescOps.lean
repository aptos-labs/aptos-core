-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefTop

/-!
# Descriptions across the analysis's operations on references

UNTRUSTED (checked by the kernel). How a description (`Desc`) follows the
operand stack when the analysis and the frame pop and push together: the top
slots correspond when the two stacks have the same height.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph

variable {A : RefState} {fr : Frame} {gsid : Nat → Option StructId} {ps : List Reference}
  {L : Nat → Option Reference}

theorem reverse_cons_getElem?_lt {α : Type} {a : α} {l : List α} {q : Nat} (hq : q < l.length) :
    (a :: l).reverse[q]? = l.reverse[q]? := by
  rw [List.reverse_cons, List.getElem?_append_left (by simpa using hq)]

theorem reverse_cons_getElem?_top {α : Type} {a : α} {l : List α} :
    (a :: l).reverse[l.length]? = some a := by
  rw [List.reverse_cons, List.getElem?_append_right (by simp)]
  simp

/-- **The top slots correspond** when the stacks have the same height. -/
theorem Desc.top (hd : Desc A fr gsid ps L) {x : Nat} {ra : List AbsValue} {v : Value}
    {rc : List Value} (ha : A.stack = .reference x :: ra) (hc : fr.stack = v :: rc)
    (hlen : ra.length = rc.length) : ∃ r, v = .ref r ∧ L x = some r := by
  obtain ⟨r, hr, hL⟩ := hd.stack ra.length x (by rw [ha]; exact reverse_cons_getElem?_top)
  rw [hc, hlen, reverse_cons_getElem?_top, Option.some.injEq] at hr
  exact ⟨r, hr, hL⟩

/-- **Popping together**, the abstract slots popped holding no reference. -/
theorem Desc.pop {A' : RefState} {fr' : Frame} (hd : Desc A fr gsid ps L) {a : AbsValue}
    {ra : List AbsValue} {v : Value} {rc : List Value} (ha : A.stack = a :: ra)
    (hc : fr.stack = v :: rc) (hlen : ra.length = rc.length) (hnr : a = .nonReference)
    (hA' : A' = { A with stack := ra }) (hid : fr'.id = fr.id) (hloc : fr'.locals = fr.locals)
    (hs' : fr'.stack = rc) : Desc A' fr' gsid ps L := by
  subst hA' hnr
  have hheld : heldIds { A with stack := ra } = heldIds A := by
    simp [heldIds, ha, AbsValue.refId?]
  refine ⟨fun i x hi => by rw [hloc]; exact hd.locals i x hi, fun q x hq => ?_, ?_⟩
  · have hq' : q < ra.length := by
      have := (List.getElem?_eq_some_iff.1 hq).1; simpa using this
    obtain ⟨r, hr, hL⟩ := hd.stack q x (by rw [ha, reverse_cons_getElem?_lt hq']; exact hq)
    rw [hc, reverse_cons_getElem?_lt (by omega)] at hr
    exact ⟨r, by rw [hs']; exact hr, hL⟩
  · rw [hid]; exact hd.complete.congrH fun x => by rw [hheld]

/-- **Pushing a value together.** -/
theorem Desc.pushVal {A' : RefState} {fr' : Frame} (hd : Desc A fr gsid ps L) {v : Value}
    (hA' : A' = { A with stack := .nonReference :: A.stack }) (hid : fr'.id = fr.id)
    (hloc : fr'.locals = fr.locals) (hs' : fr'.stack = v :: fr.stack) :
    Desc A' fr' gsid ps L := by
  refine hd.valueStep (j := 0) (k := 1) (new := [v]) (j' := 0) ?_ hid hloc (by simp [hs'])
    (by simp)
  subst hA'
  exact ValueStep.pushOne

/-- **Popping a reference and releasing it.** -/
theorem Desc.popRelease {ltys stys : List SignatureToken} {avail : LocalIndex → Availability}
    {A' : RefState} {fr' : Frame} (hd : Desc A fr gsid ps L) (ht : Typed ltys stys avail A)
    {x : Nat} {ra : List AbsValue} {v : Value} {rc : List Value} {g : Graph}
    (ha : A.stack = .reference x :: ra) (hc : fr.stack = v :: rc) (hlen : ra.length = rc.length)
    (hrel : A.graph.release x = .ok g) (hA' : A' = { A with stack := ra, graph := g })
    (hid : fr'.id = fr.id) (hloc : fr'.locals = fr.locals) (hs' : fr'.stack = rc) :
    Desc A' fr' gsid ps L := by
  subst hA'
  have hx : x ∈ heldIds A := mem_heldIds.2 (Or.inr (by rw [ha]; exact List.mem_cons_self))
  have hc' := hd.complete.release hx hrel
  have hheld : heldIds A = A.locals.filterMap AbsValue.refId? ++
      x :: ra.filterMap AbsValue.refId? := by simp [heldIds, ha, AbsValue.refId?]
  have hnd := ht.distinct
  rw [hheld] at hnd
  have hiff : ∀ y, (y ∈ heldIds A ∧ y ≠ x) ↔
      y ∈ heldIds { A with stack := ra, graph := g } := by
    intro y
    simp only [heldIds] at hheld ⊢
    rw [hheld]
    simp only [List.mem_append, List.mem_cons]
    constructor
    · rintro ⟨h | h | h, hne⟩
      · exact Or.inl h
      · exact absurd h hne
      · exact Or.inr h
    · intro h
      rcases h with h | h
      · refine ⟨Or.inl h, fun e => ?_⟩
        subst e
        rw [List.nodup_append] at hnd
        exact hnd.2.2 _ h _ List.mem_cons_self rfl
      · refine ⟨Or.inr (Or.inr h), fun e => ?_⟩
        subst e
        rw [List.nodup_append, List.nodup_cons] at hnd
        exact hnd.2.1.1 h
  refine ⟨fun i y hi => by rw [hloc]; exact hd.locals i y hi, fun q y hq => ?_, ?_⟩
  · have hq' : q < ra.length := by
      have := (List.getElem?_eq_some_iff.1 hq).1; simpa using this
    obtain ⟨r, hr, hL⟩ := hd.stack q y (by rw [ha, reverse_cons_getElem?_lt hq']; exact hq)
    rw [hc, reverse_cons_getElem?_lt (by omega)] at hr
    exact ⟨r, by rw [hs']; exact hr, hL⟩
  · rw [hid]; exact hc'.congrH hiff

/-- **Pushing a new reference** `c` standing for `r`, given the graph's
completeness with it. -/
theorem Desc.pushNew {ltys stys : List SignatureToken} {avail : LocalIndex → Availability}
    {A' : RefState} {fr' : Frame} (hd : Desc A fr gsid ps L) (ht : Typed ltys stys avail A)
    {c : Nat} {r : Reference} {g : Graph} (hfresh : A.graph.find? c = none)
    (hlen : A.stack.length = fr.stack.length)
    (hc : Complete g (updL L c r) (fun x => x ∈ heldIds A ∨ x = c) A.locals.length fr.id gsid ps)
    (hA' : A'.locals = A.locals ∧ A'.graph = g ∧ A'.stack = .reference c :: A.stack)
    (hid : fr'.id = fr.id) (hloc : fr'.locals = fr.locals) (hs' : fr'.stack = .ref r :: fr.stack) :
    Desc A' fr' gsid ps (updL L c r) := by
  obtain ⟨hl, hg, hs⟩ := hA'
  have hcH : c ∉ heldIds A := fun h => by
    have := ht.held_key h; rw [hfresh] at this; cases this
  have hne : ∀ y, y ∈ heldIds A → y ≠ c := fun y hy e => hcH (e ▸ hy)
  refine ⟨fun i y hi => ?_, fun q y hq => ?_, ?_⟩
  · rw [hl] at hi
    obtain ⟨r', hr', hL⟩ := hd.locals i y hi
    have hy : y ≠ c := hne y (mem_heldIds.2 (Or.inl (List.mem_of_getElem? hi)))
    exact ⟨r', by rw [hloc]; exact hr', by rw [updL_ne _ hy]; exact hL⟩
  · rw [hs] at hq
    by_cases hq' : q < A.stack.length
    · rw [reverse_cons_getElem?_lt hq'] at hq
      obtain ⟨r', hr', hL⟩ := hd.stack q y hq
      have hy : y ≠ c := hne y (mem_heldIds.2 (Or.inr
        (List.mem_reverse.1 (List.mem_of_getElem? hq))))
      refine ⟨r', ?_, by rw [updL_ne _ hy]; exact hL⟩
      rw [hs', reverse_cons_getElem?_lt (by omega)]
      exact hr'
    · have hqe : q = A.stack.length := by
        have := (List.getElem?_eq_some_iff.1 hq).1; simp at this; omega
      subst hqe
      rw [reverse_cons_getElem?_top, Option.some.injEq, AbsValue.reference.injEq] at hq
      subst hq
      refine ⟨r, ?_, updL_self _ _ _⟩
      rw [hs', hlen, reverse_cons_getElem?_top]
  · rw [hl, hg, hid]
    refine hc.congrH fun y => ?_
    simp only [heldIds, hl, hs, List.filterMap_cons, AbsValue.refId?, List.mem_append,
      List.mem_cons]
    constructor
    · rintro ((h | h) | h)
      · exact Or.inl h
      · exact Or.inr (Or.inr h)
      · exact Or.inr (Or.inl h)
    · rintro (h | h | h)
      · exact Or.inl (Or.inl h)
      · exact Or.inr h
      · exact Or.inl (Or.inr h)

end MoveBytecode.Proofs
