-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.DanglingDefs

/-!
# A hit before a later use is excluded

UNTRUSTED (checked by the kernel). On events satisfying D4b′ (`contained`),
D4c (`stable`) and D4e (`deadSlotsUnborrowed`), no event between the creation
of a reference and a later use of it hits the reference's location
(`hits_absurd`): a direct write over a `&mut` reaches over it, a write through
a reference strictly enclosing it reaches over it, a write over an `&`
overlaps a location it is live on, and the end of its frame ends a root it
points into while it is live.
-/

namespace MoveBytecode.Proofs

/-- An event at a position is in the indexed events. -/
private theorem ev_mem_zipIdx {E : List Event} {p : Nat} {e : Event} (hp : E[p]? = some e) :
    (e, p) ∈ E.zipIdx :=
  List.mem_zipIdx_iff_getElem?.2 hp

/-- An event at a later position is among the indexed events after `p`. -/
private theorem ev_mem_zipIdx_drop {E : List Event} {p q : Nat} {e : Event}
    (hq : E[q]? = some e) (hpq : p < q) : (e, q) ∈ E.zipIdx.drop (p + 1) := by
  have : (E.zipIdx.drop (p + 1))[q - (p + 1)]? = some (e, q) := by
    rw [List.getElem?_drop, show p + 1 + (q - (p + 1)) = q by omega, List.getElem?_zipIdx, hq]
    simp
  exact List.mem_of_getElem? this

/-- An event at a later position is among the events after `q`. -/
private theorem ev_mem_drop {E : List Event} {q u : Nat} {e : Event} (hu : E[u]? = some e)
    (hqu : q < u) : e ∈ E.drop (q + 1) := by
  have : (E.drop (q + 1))[u - (q + 1)]? = some e := by
    rw [List.getElem?_drop, show q + 1 + (u - (q + 1)) = u by omega, hu]
  exact List.mem_of_getElem? this

/-- An event at an earlier position is among the events before `q`. -/
private theorem ev_mem_take {E : List Event} {p q : Nat} {e : Event} (hp : E[p]? = some e)
    (hpq : p < q) : e ∈ E.take q := by
  have : (E.take q)[p]? = some e := by rw [List.getElem?_take, if_pos hpq, hp]
  exact List.mem_of_getElem? this

/-- A reference derives from itself. -/
private theorem ev_derivesFrom_self {E : List Event} {i : Nat} (h : 0 < E.length) :
    RefSafety.derivesFrom E i i = true := by
  unfold RefSafety.derivesFrom
  obtain ⟨n, hn⟩ : ∃ n, E.length = n + 1 := ⟨E.length - 1, by omega⟩
  rw [hn]
  simp [RefSafety.derivesFrom.go]

/-- A use at a later position: the reference itself is used after `q`. -/
private theorem ev_usedItselfAfter {E : List Event} {q u i : Nat} {r : Reference}
    (hu : E[u]? = some (.used i r)) (hqu : q < u) : RefSafety.usedItselfAfter E i q = true := by
  unfold RefSafety.usedItselfAfter
  rw [List.any_eq_true]
  exact ⟨_, ev_mem_drop hu hqu, by simp [Event.used?]⟩

/-- A use at a later position: the reference is live after `q`. -/
private theorem ev_usedAfter {E : List Event} {q u i : Nat} {r : Reference} (hlen : 0 < E.length)
    (hu : E[u]? = some (.used i r)) (hqu : q < u) : RefSafety.usedAfter E i q = true := by
  unfold RefSafety.usedAfter
  rw [List.any_eq_true]
  exact ⟨_, ev_mem_drop hu hqu, by simp [Event.used?, ev_derivesFrom_self hlen]⟩

/-- A use at a later position: the reference is live on its location after
`q`. -/
private theorem ev_live {E : List Event} {q u i : Nat} {r : Reference} (hlen : 0 < E.length)
    (hu : E[u]? = some (.used i r)) (hqu : q < u) :
    r.location ∈ RefSafety.liveLocations E i q := by
  unfold RefSafety.liveLocations
  rw [List.mem_filterMap]
  exact ⟨_, ev_mem_drop hu hqu, by simp [Event.used?, ev_derivesFrom_self hlen]⟩

/-- **A hit before a later use is excluded** by D4b′, D4c and D4e: between the
creation of reference `i` at position `p` and its use at `u`, no event at `q`
hits its location. -/
theorem hits_absurd {E : List Event} (hc : RefSafety.contained E = true)
    (hs : RefSafety.stable E = true) (hd : RefSafety.deadSlotsUnborrowed E = true)
    {p q u i : Nat} {r : Reference} {par : Option Nat} {e : Event}
    (hp : E[p]? = some (.borrowed i r par)) (hq : E[q]? = some e)
    (hu : E[u]? = some (.used i r)) (hpq : p < q) (hqu : q < u)
    (hh : Hits r.location e) : False := by
  have hlen : 0 < E.length := by
    have := (List.getElem?_eq_some_iff.1 hp).1
    omega
  rcases hh with ⟨m, rfl, hov⟩ | ⟨m, t, rfl, hin⟩ | ⟨f, rs, j, rfl, hroot⟩
  · cases hmut : r.mutable
    · unfold RefSafety.stable at hs
      rw [List.all_eq_true] at hs
      have h1 := hs _ (ev_mem_zipIdx hp)
      simp only [Event.borrowed?, hmut, Bool.false_or, List.all_eq_true] at h1
      have h2 := h1 _ (ev_mem_zipIdx_drop hq hpq)
      simp only [Event.accessed?, List.all_eq_true] at h2
      have h3 := h2 _ (ev_live hlen hu hqu)
      rw [hov] at h3
      cases h3
    · unfold RefSafety.contained at hc
      rw [List.all_eq_true] at hc
      have h1 := hc _ (ev_mem_zipIdx hp)
      simp only [Event.borrowed?, hmut, Bool.not_true, Bool.false_or, List.all_eq_true] at h1
      have h2 := h1 _ (ev_mem_zipIdx_drop hq hpq)
      simp only [Event.accessed?, Access.reachesOver, hov, ev_usedItselfAfter hu hqu] at h2
      cases h2
  · cases hmut : r.mutable
    · unfold RefSafety.stable at hs
      rw [List.all_eq_true] at hs
      have h1 := hs _ (ev_mem_zipIdx hp)
      simp only [Event.borrowed?, hmut, Bool.false_or, List.all_eq_true] at h1
      have h2 := h1 _ (ev_mem_zipIdx_drop hq hpq)
      simp only [Event.accessed?, List.all_eq_true] at h2
      have h3 := h2 _ (ev_live hlen hu hqu)
      have hov : Access.write.overlaps m r.location = true := by
        simp only [Location.strictlyInside, Bool.and_eq_true, beq_iff_eq] at hin
        simp [Access.overlaps, hin.1.1, hin.1.2]
      rw [hov] at h3
      cases h3
    · unfold RefSafety.contained at hc
      rw [List.all_eq_true] at hc
      have h1 := hc _ (ev_mem_zipIdx hp)
      simp only [Event.borrowed?, hmut, Bool.not_true, Bool.false_or, List.all_eq_true] at h1
      have h2 := h1 _ (ev_mem_zipIdx_drop hq hpq)
      simp only [Event.accessed?, Access.reachesOver, hin, ev_usedItselfAfter hu hqu] at h2
      cases h2
  · unfold RefSafety.deadSlotsUnborrowed at hd
    rw [List.all_eq_true] at hd
    have h1 := hd _ (ev_mem_zipIdx hq)
    simp only [Event.ends?, List.all_eq_true] at h1
    have h2 := h1 _ (ev_mem_take hp hpq)
    simp only [Event.borrowed?, hroot, ev_usedAfter hlen hu hqu] at h2
    simp at h2

end MoveBytecode.Proofs
