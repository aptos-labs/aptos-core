-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefUpdate

/-!
# Completeness across a call's results

UNTRUSTED (checked by the kernel). After a call, the analysis gives each
reference result a new reference borrowed from the arguments it may come
from, then releases the arguments. The graph stays complete for the caller's
references and the results the callee returned (`Complete.addNode`), given
what the caller's graph says of the arguments and what the callee's return
says of the results.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph

namespace Complete

variable {g g1 g2 : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
  {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}

/-- **A new reference borrowed from several held ones**: the graph stays
complete when every reference it may be strictly or exactly inside has an
anchor, a parent it is inside, and the new reference relates to the held
ones as the hypotheses say. -/
theorem addNode {Q : List Nat} {c : Nat} {r : Reference}
    (hc : Complete g L H root fid gsid ps) (hQ : ∀ q ∈ Q, H q)
    (hnew : g.newRef c r.mutable = .ok g1) (hadd : addBorrows g1 Q c = .ok g2)
    (hdown : ∀ q rq, H q → L q = some rq → rq.mutable = true →
      r.location.Inside rq.location →
      ∃ p ∈ Q, ∃ rp, L p = some rp ∧ r.location.Inside rp.location ∧
        (r.mutable = true → rp.mutable = true) ∧ (q = p ∨ Chain g L q p))
    (hsame : r.mutable = true → ∀ x rx, H x → L x = some rx → rx.location = r.location →
      ∃ p ∈ Q, ∃ rp, L p = some rp ∧ r.location.Inside rp.location ∧
        (r.mutable = true → rp.mutable = true) ∧ (x = p ∨ Chain g L x p))
    (hup : r.mutable = true → ∀ x rx, H x → L x = some rx →
      ¬ rx.location.StrictlyInside r.location)
    (himm : r.mutable = false → ∀ x rx, H x → L x = some rx →
      rx.location.StrictlyInside r.location → rx.mutable = false)
    (himm2 : ∀ p rp, H p → L p = some rp → rp.mutable = false →
      r.location.StrictlyInside rp.location → r.mutable = false)
    (hrootLocal : ∀ i, r.location.root = .local_ fid i →
      ∃ p ∈ Q, ∃ rp, L p = some rp ∧ r.location.Inside rp.location ∧
        (r.mutable = true → rp.mutable = true) ∧ RootChain g L root fid gsid (some (.local_ i)) p)
    (hprov : (∃ i, r.location.root = .local_ fid i) ∨
      (∃ d, ∃ p ∈ Q, ∃ rp, L p = some rp ∧ r.location.Inside rp.location ∧
        (r.mutable = true → rp.mutable = true) ∧ RootChain g L root fid gsid (some (.global d)) p) ∨
      ∃ π ∈ ps, r.location.Inside π.location ∧ (r.mutable = true → π.mutable = true)) :
    Complete g2 (updL L c r) (fun x => H x ∨ x = c) root fid gsid ps := by
  have hcfresh := (Graph.newRef_ok hnew).1
  have hcH : ¬ H c := hc.not_held_of_fresh hcfresh
  have hcr : c ≠ root := fun e => by
    have := hc.root_key; rw [← e, hcfresh] at this; cases this
  have hw1 := Graph.newRef_wf hnew hc.wf
  have hb1 := Graph.newRef_back hnew hc.back
  obtain ⟨hw2, hb2, hmut2, hcov2, hedges⟩ := addBorrows_props hadd hw1
  have hLc : updL L c r c = some r := updL_self _ _ _
  have hLx : ∀ x, x ≠ c → updL L c r x = L x := fun x hx => updL_ne r hx
  generalize updL L c r = L' at hLc hLx ⊢
  have hext : ∀ x r, (g.find? x).isSome → L x = some r → L' x = some r := by
    intro x r hx h
    have : x ≠ c := fun e => by subst e; rw [hcfresh] at hx; cases hx
    rw [hLx x this]; exact h
  have hHL : ∀ x r, H x → L' x = some r → L x = some r := fun x r hx h => by
    rwa [hLx x (fun e => hcH (e ▸ hx))] at h
  have hLH : ∀ x r, H x → L x = some r → L' x = some r := fun x r hx h => by
    rw [hLx x (fun e => hcH (e ▸ hx))]; exact h
  have hcov : g.Covers g2 := (Graph.newRef_covers hnew).trans hcov2
  have hmapC : ∀ a b, Chain g L a b → Chain g2 L' a b := fun a b h =>
    Chain.mono hcov (h.extend' hext)
  have hmapR : ∀ l b, RootChain g L root fid gsid l b → RootChain g2 L' root fid gsid l b :=
    fun l b h => RootChain.mono hcov (h.extend' hext)
  -- An anchor's edge to `c` is true.
  have htrue : ∀ rp : Reference, r.location.Inside rp.location →
      (r.mutable = true → rp.mutable = true) →
      TrueEdge rp.location rp.mutable ⟨false, []⟩ r.location r.mutable := by
    intro rp hin hm
    refine ⟨hin.1, ?_, fun h0 => ?_⟩
    · simp only [Bool.false_eq_true, ↓reduceIte, List.append_nil]
      exact hin.abs
    · cases hrm : r.mutable
      · rfl
      · rw [hm hrm] at h0; cases h0
  have hanchor : ∀ p ∈ Q, ∀ rp, L p = some rp → r.location.Inside rp.location →
      (r.mutable = true → rp.mutable = true) → Chain g2 L' p c := by
    intro p hp rp hpL hin hm
    obtain ⟨e', he', hle⟩ := hedges p hp
    exact .edge (hLH p rp (hQ p hp) hpL) hLc he' (TrueEdge.cover (htrue rp hin hm) hle)
  have hto : ∀ q, (∃ p ∈ Q, ∃ rp, L p = some rp ∧ r.location.Inside rp.location ∧
      (r.mutable = true → rp.mutable = true) ∧ (q = p ∨ Chain g L q p)) → Chain g2 L' q c := by
    rintro q ⟨p, hp, rp, hpL, hin, hm, rfl | hch⟩
    · exact hanchor q hp rp hpL hin hm
    · exact (hmapC q p hch).trans (hanchor p hp rp hpL hin hm)
  have hmut2' : ∀ x, x ≠ c → g2.mutability x = g.mutability x := fun x hx => by
    rw [hmut2, Graph.newRef_mutability hnew x hx]
  refine {
    wf := hw2
    back := hb2 hb1
    rootNone := by rw [hLx root (Ne.symm hcr)]; exact hc.rootNone
    rootMut := by rw [hmut2' root (Ne.symm hcr)]; exact hc.rootMut
    rootNotHeld := fun h => h.elim hc.rootNotHeld (fun e => hcr e.symm)
    keys := fun x hx => ?_
    held := fun x hx => ?_
    strict := fun q rq x rx hq hx hqL hxL hm hin => ?_
    same := fun q rq x rx hq hx hne hqL hxL hm heq => ?_
    immInside := fun q rq x rx hq hx hqL hxL hm hin => ?_
    rootLocal := fun x rx i hx hxL hr => ?_
    prov := fun x rx hx hxL => ?_ }
  · by_cases hxc : x = c
    · exact Or.inr (Or.inr hxc)
    · rw [Graph.isSome_of_mutability (hmut2' x hxc)] at hx
      rcases hc.keys x hx with h | h
      · exact Or.inl h
      · exact Or.inr (Or.inl h)
  · rcases hx with hx | rfl
    · obtain ⟨r', hr, hm⟩ := hc.held x hx
      have hxc : x ≠ c := fun e => hcH (e ▸ hx)
      exact ⟨r', by rw [hLx x hxc]; exact hr, by rw [hmut2' x hxc]; exact hm⟩
    · exact ⟨r, hLc, by rw [hmut2, Graph.newRef_mutability_self hnew]⟩
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hmapC q x (hc.strict q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      exact hto q (hdown q rq hq (hHL q rq hq hqL) hm hin.1)
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      exact absurd hin (hup hm x rx hx (hHL x rx hx hxL))
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin StrictlyInside.ne
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · rcases hc.same q rq x rx hq hx hne (hHL q rq hq hqL) (hHL x rx hx hxL) hm heq with h | h
      · exact Or.inl (hmapC q x h)
      · exact Or.inr (hmapC x q h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      exact Or.inl (hto q (hdown q rq hq (hHL q rq hq hqL) hm
        ⟨by rw [heq], by rw [heq]; exact List.prefix_refl _⟩))
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      exact Or.inr (hto x (hsame hm x rx hx (hHL x rx hx hxL) heq))
    · exact absurd rfl hne
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hc.immInside q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      exact himm2 q rq hq (hHL q rq hq hqL) hm hin
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      exact himm hm x rx hx (hHL x rx hx hxL) hin
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin StrictlyInside.ne
  · rcases hx with hx | rfl
    · exact hmapR _ x (hc.rootLocal x rx i hx (hHL x rx hx hxL) hr)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      obtain ⟨p, hp, rp, hpL, hin, hm, hrc⟩ := hrootLocal i hr
      obtain ⟨e', he', hle⟩ := hedges p hp
      exact (hmapR _ p hrc).snoc (hLH p rp (hQ p hp) hpL) hLc he'
        (TrueEdge.cover (htrue rp hin hm) hle)
  · rcases hx with hx | rfl
    · rcases hc.prov x rx hx (hHL x rx hx hxL) with h | ⟨d, h⟩ | h
      · exact Or.inl h
      · exact Or.inr (Or.inl ⟨d, hmapR _ x h⟩)
      · exact Or.inr (Or.inr h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      rcases hprov with h | ⟨d, p, hp, rp, hpL, hin, hm, hrc⟩ | h
      · exact Or.inl h
      · obtain ⟨e', he', hle⟩ := hedges p hp
        exact Or.inr (Or.inl ⟨d, (hmapR _ p hrc).snoc (hLH p rp (hQ p hp) hpL) hLc he'
          (TrueEdge.cover (htrue rp hin hm) hle)⟩)
      · exact Or.inr (Or.inr h)

end Complete

/-- Two locations a third is inside are nested one in the other. -/
theorem inside_compare {r a b : Location} (ha : r.Inside a) (hb : r.Inside b) :
    a.StrictlyInside b ∨ a = b ∨ b.StrictlyInside a := by
  have hroot : a.root = b.root := ha.1.symm.trans hb.1
  rcases Nat.lt_trichotomy a.path.length b.path.length with h | h | h
  · have hp : a.path <+: b.path := List.prefix_of_prefix_length_le ha.2 hb.2 (Nat.le_of_lt h)
    exact Or.inr (Or.inr ⟨⟨hroot.symm, hp⟩, fun e => by rw [e] at h; omega⟩)
  · have hp : a.path <+: b.path := List.prefix_of_prefix_length_le ha.2 hb.2 (Nat.le_of_eq h)
    exact Or.inr (Or.inl (Location.eq_of hroot (hp.eq_of_length h)))
  · have hp : b.path <+: a.path := List.prefix_of_prefix_length_le hb.2 ha.2 (Nat.le_of_lt h)
    exact Or.inl ⟨⟨hroot, hp⟩, fun e => by rw [e] at h; omega⟩

/-- **What the caller's graph says of a call's result** inside argument `a`:
everything `Complete.addNode` asks of the references the caller holds. -/
theorem result_facts {g : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
    {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}
    (hc : Complete g L H root fid gsid ps)
    {all muts : List Nat}
    (hmuts : ∀ a ∈ all, ∀ ra, L a = some ra → (a ∈ muts ↔ ra.mutable = true))
    (hwr : ∀ a ∈ muts, g.isWritable a = .ok true)
    {r : Reference} {a : Nat} {ra : Reference} (ha : a ∈ all) (haH : H a) (haL : L a = some ra)
    (hin : r.location.Inside ra.location) (hm : r.mutable = true → ra.mutable = true) :
    (∀ q rq, H q → L q = some rq → rq.mutable = true → r.location.Inside rq.location →
      ∃ p ∈ (if r.mutable then muts else all), ∃ rp, L p = some rp ∧
        r.location.Inside rp.location ∧ (r.mutable = true → rp.mutable = true) ∧
        (q = p ∨ Chain g L q p)) ∧
    (r.mutable = true → ∀ x rx, H x → L x = some rx → rx.location = r.location →
      ∃ p ∈ (if r.mutable then muts else all), ∃ rp, L p = some rp ∧
        r.location.Inside rp.location ∧ (r.mutable = true → rp.mutable = true) ∧
        (x = p ∨ Chain g L x p)) ∧
    (r.mutable = true → ∀ x rx, H x → L x = some rx → ¬ rx.location.StrictlyInside r.location) ∧
    (r.mutable = false → ∀ x rx, H x → L x = some rx →
      rx.location.StrictlyInside r.location → rx.mutable = false) ∧
    (∀ p rp, H p → L p = some rp → rp.mutable = false →
      r.location.StrictlyInside rp.location → r.mutable = false) ∧
    (∀ i, r.location.root = .local_ fid i →
      ∃ p ∈ (if r.mutable then muts else all), ∃ rp, L p = some rp ∧
        r.location.Inside rp.location ∧ (r.mutable = true → rp.mutable = true) ∧
        RootChain g L root fid gsid (some (.local_ i)) p) ∧
    ((∃ i, r.location.root = .local_ fid i) ∨
      (∃ d, ∃ p ∈ (if r.mutable then muts else all), ∃ rp, L p = some rp ∧
        r.location.Inside rp.location ∧ (r.mutable = true → rp.mutable = true) ∧
        RootChain g L root fid gsid (some (.global d)) p) ∨
      ∃ π ∈ ps, r.location.Inside π.location ∧ (r.mutable = true → π.mutable = true)) := by
  have haQ : a ∈ (if r.mutable then muts else all) := by
    cases hrm : r.mutable
    · exact ha
    · exact (hmuts a ha ra haL).2 (hm hrm)
  -- A mutable argument borrows nothing.
  have hnoOut : ra.mutable = true → ∀ x, ¬ Chain g L a x := by
    intro hram x h
    obtain ⟨e, n, -, -, -, -, he, -⟩ := h.first
    exact (Graph.isWritable_true (hwr a ((hmuts a ha ra haL).2 hram))).2 e n he
  -- Nothing mutable is reached from an immutable reference.
  have himmChain : ∀ x rx, L x = some rx → Chain g L x a → rx.mutable = false →
      ra.mutable = false := fun x rx hxL h hrx => (h.facts hxL haL).2.2 hrx
  have hanch : ∃ p ∈ (if r.mutable then muts else all), ∃ rp, L p = some rp ∧
      r.location.Inside rp.location ∧ (r.mutable = true → rp.mutable = true) ∧ p = a :=
    ⟨a, haQ, ra, haL, hin, hm, rfl⟩
  refine ⟨fun q rq hq hqL hqm hrq => ?_, fun hrm x rx hx hxL heq => ?_, fun hrm x rx hx hxL hxin => ?_,
    fun hrm x rx hx hxL hxin => ?_, fun p rp hp hpL hpm hrin => ?_, fun i hi => ?_, ?_⟩
  · rcases inside_compare hin hrq with h | h | h
    · exact ⟨a, haQ, ra, haL, hin, hm, Or.inr (hc.strict q rq a ra hq haH hqL haL hqm h)⟩
    · by_cases hqa : q = a
      · subst hqa; exact ⟨q, haQ, ra, haL, hin, hm, Or.inl rfl⟩
      · rcases hc.same q rq a ra hq haH hqa hqL haL hqm h with h' | h'
        · exact ⟨a, haQ, ra, haL, hin, hm, Or.inr h'⟩
        · exfalso
          cases hram : ra.mutable
          · have := (h'.facts haL hqL).2.2 hram; rw [hqm] at this; cases this
          · exact hnoOut hram q h'
    · exfalso
      cases hram : ra.mutable
      · have := hc.immInside a ra q rq haH hq haL hqL hram h; rw [hqm] at this; cases this
      · exact hnoOut hram q (hc.strict a ra q rq haH hq haL hqL hram h)
  · have hram := hm hrm
    have hxin : rx.location.Inside ra.location := by rw [heq]; exact hin
    rcases inside_compare hxin (Location.Inside.refl _) with h | h | h
    · exact absurd (StrictlyInside.trans' h hxin) StrictlyInside.ne
    · by_cases hxa : x = a
      · subst hxa; exact ⟨x, haQ, ra, haL, hin, hm, Or.inl rfl⟩
      · rcases hc.same a ra x rx haH hx (Ne.symm hxa) haL hxL hram h.symm with h' | h'
        · exact absurd h' (hnoOut hram x)
        · exact ⟨a, haQ, ra, haL, hin, hm, Or.inr h'⟩
    · exact absurd (hc.strict a ra x rx haH hx haL hxL hram h) (hnoOut hram x)
  · have hram := hm hrm
    exact hnoOut hram x (hc.strict a ra x rx haH hx haL hxL hram (StrictlyInside.trans' hxin hin))
  · have hxa := StrictlyInside.trans' hxin hin
    cases hram : ra.mutable
    · exact hc.immInside a ra x rx haH hx haL hxL hram hxa
    · exact absurd (hc.strict a ra x rx haH hx haL hxL hram hxa) (hnoOut hram x)
  · cases hrm : r.mutable
    · rfl
    · exfalso
      have hram := hm hrm
      rcases inside_compare hin hrin.1 with h | h | h
      · have := hc.immInside p rp a ra hp haH hpL haL hpm h; rw [hram] at this; cases this
      · by_cases hpa : p = a
        · subst hpa; rw [haL, Option.some.injEq] at hpL; subst hpL; rw [hram] at hpm; cases hpm
        · rcases hc.same a ra p rp haH hp (Ne.symm hpa) haL hpL hram h.symm with h' | h'
          · exact hnoOut hram p h'
          · have := himmChain p rp hpL h' hpm; rw [hram] at this; cases this
      · exact hnoOut hram p (hc.strict a ra p rp haH hp haL hpL hram h)
  · have hra : ra.location.root = .local_ fid i := by rw [← hin.1, hi]
    exact ⟨a, haQ, ra, haL, hin, hm, hc.rootLocal a ra i haH haL hra⟩
  · rcases hc.prov a ra haH haL with ⟨i, hi⟩ | ⟨d, hd⟩ | ⟨π, hπ, hπin, hπm⟩
    · exact Or.inl ⟨i, by rw [hin.1, hi]⟩
    · exact Or.inr (Or.inl ⟨d, a, haQ, ra, haL, hin, hm, hd⟩)
    · exact Or.inr (Or.inr ⟨π, hπ, hin.trans hπin, fun h => hπm (hm h)⟩)

theorem Inside.strict_or_eq {l m : Location} (h : l.Inside m) : l.StrictlyInside m ∨ l = m := by
  by_cases he : l.path = m.path
  · exact Or.inr (Location.eq_of h.1 he)
  · exact Or.inl ⟨h, he⟩

/-- Two references a call returns, compatible: nothing is inside a mutable
one, and what is strictly inside an immutable one is immutable. -/
def Compat (d r : Reference) : Prop :=
  (d.mutable = true → ¬ r.location.StrictlyInside d.location ∧ r.location ≠ d.location) ∧
  (r.mutable = true → ¬ d.location.StrictlyInside r.location ∧ d.location ≠ r.location) ∧
  (r.mutable = false → d.location.StrictlyInside r.location → d.mutable = false) ∧
  (d.mutable = false → r.location.StrictlyInside d.location → r.mutable = false)

theorem argRefs_cons_ref (r : Reference) (vs : List Value) :
    argRefs (.ref r :: vs) = r :: argRefs vs := rfl

theorem argRefs_cons_val {v : Value} (hv : ∀ r, v ≠ .ref r) (vs : List Value) :
    argRefs (v :: vs) = argRefs vs := by
  cases v <;> first | rfl | exact absurd rfl (hv _)

/-- **The results' references, one at a time**: the graph stays complete for
the caller's references and the results so far. -/
theorem results_complete {g0 : Graph} {L : Nat → Option Reference} {H0 : Nat → Prop}
    {root : Nat} {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}
    {all muts : List Nat}
    (hc0 : Complete g0 L H0 root fid gsid ps) (hallH : ∀ a ∈ all, H0 a)
    (hma : ∀ a ∈ muts, a ∈ all)
    (hmuts : ∀ a ∈ all, ∀ ra, L a = some ra → (a ∈ muts ↔ ra.mutable = true))
    (hwr : ∀ a ∈ muts, g0.isWritable a = .ok true) :
    ∀ (rs : List SignatureToken) (rv : List Value) (st st' : RefState) (vals : List AbsValue)
      (Lj : Nat → Option Reference) (Hj : Nat → Prop) (done : List Reference),
      (rs.mapM (callResult all muts)).run st = .ok (vals, st') →
      rv.length = rs.length →
      (∀ (j : Nat) t m, rs[j]? = some t → refKind t = some m →
        ∃ r, rv[j]? = some (.ref r) ∧ r.mutable = m) →
      (∀ (j : Nat) t, rs[j]? = some t → refKind t = none → ∀ v, rv[j]? = some v →
        ∀ r, v ≠ .ref r) →
      (∀ r ∈ argRefs rv, ∃ a ∈ all, ∃ ra, L a = some ra ∧ r.location.Inside ra.location ∧
        (r.mutable = true → ra.mutable = true)) →
      (done ++ argRefs rv).Pairwise Compat →
      Complete st.graph Lj Hj root fid gsid ps →
      g0.Covers st.graph →
      (∀ x, (g0.find? x).isSome → (st.graph.find? x).isSome) →
      (∀ x r, (g0.find? x).isSome → L x = some r → Lj x = some r) →
      (∀ x, H0 x → Hj x) →
      (∀ x rx, Hj x → Lj x = some rx → (H0 x ∧ L x = some rx) ∨ rx ∈ done) →
      ∃ L', Complete st'.graph L' (fun x => Hj x ∨ AbsValue.reference x ∈ vals) root fid
          gsid ps ∧
        (∀ x r, Hj x → Lj x = some r → L' x = some r) ∧
        (∀ (j x : Nat) r, vals[j]? = some (.reference x) → rv[j]? = some (.ref r) → L' x = some r)
  | [], rv, st, st', vals, Lj, Hj, done, h, _, _, _, _, _, hc, _, _, _, _, _ => by
    simp only [List.mapM_nil] at h
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    exact ⟨Lj, hc.congrH fun x => by simp, fun x r _ h => h, fun j x r h => by simp at h⟩
  | t :: rs, rv, st, st', vals, Lj, Hj, done, h, hlen, hkind, hnone, hrc1, hpw, hc, hcov, hkeys,
      hLL, hH0, hHj => by
    simp only [List.mapM_cons] at h
    rw [run_bind] at h
    obtain ⟨v, st₁, h1, h⟩ := h
    rw [run_bind] at h
    obtain ⟨vs, st₂, h2, h⟩ := h
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    obtain ⟨v0, rv', rfl⟩ : ∃ v0 rv', rv = v0 :: rv' := by
      cases rv with
      | nil => simp at hlen
      | cons v0 rv' => exact ⟨v0, rv', rfl⟩
    have hlen' : rv'.length = rs.length := by simpa using hlen
    have hkind' : ∀ (j : Nat) t m, rs[j]? = some t → refKind t = some m →
        ∃ r, rv'[j]? = some (.ref r) ∧ r.mutable = m := fun j t m h1 h2 =>
      hkind (j + 1) t m (by simpa using h1) h2
    have hnone' : ∀ (j : Nat) t, rs[j]? = some t → refKind t = none → ∀ v, rv'[j]? = some v →
        ∀ r, v ≠ .ref r := fun j t h1 h2 v hv => hnone (j + 1) t (by simpa using h1) h2 v
          (by simpa using hv)
    rcases callResult_run h1 with ⟨hk, rfl, rfl⟩ | ⟨m, g1, g2, hk, rfl, hnew, hadd, rfl⟩
    · -- A value.
      have hv0 : ∀ r, v0 ≠ .ref r := hnone 0 t rfl hk v0 rfl
      rw [argRefs_cons_val hv0] at hrc1 hpw
      obtain ⟨L', hc', hext, hmap⟩ := results_complete hc0 hallH hma hmuts hwr rs rv' _ st' vs Lj
        Hj done h2 hlen' hkind' hnone' hrc1 hpw hc hcov hkeys hLL hH0 hHj
      refine ⟨L', hc'.congrH fun x => by simp, hext, fun j x r hj hr => ?_⟩
      cases j with
      | zero => simp at hj
      | succ j => exact hmap j x r (by simpa using hj) (by simpa using hr)
    · -- A reference.
      obtain ⟨r, hr0, hrm⟩ := hkind 0 t m rfl hk
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hr0
      subst hr0
      subst hrm
      rw [argRefs_cons_ref] at hrc1 hpw
      have hfresh := (Graph.newRef_ok hnew).1
      obtain ⟨a, ha, ra, haL, hin, hm⟩ := hrc1 r List.mem_cons_self
      obtain ⟨hD, hS, hU, hI, hI2, hRL, hPV⟩ :=
        result_facts hc0 hmuts hwr ha (hallH a ha) haL hin hm
      have hmapC : ∀ a b, Chain g0 L a b → Chain st.graph Lj a b := fun a b h =>
        Chain.mono hcov (h.extend' hLL)
      have hmapR : ∀ l b, RootChain g0 L root fid gsid l b →
          RootChain st.graph Lj root fid gsid l b := fun l b h =>
        RootChain.mono hcov (h.extend' hLL)
      have hkeyL : ∀ p rp, H0 p → L p = some rp → Lj p = some rp := fun p rp hp hpL => by
        have hm' := hc0.held_mut hp hpL
        have hk' : (g0.find? p).isSome := by
          simp only [Graph.mutability, Option.map_eq_some_iff] at hm'
          obtain ⟨i, hi, -⟩ := hm'
          simp [hi]
        exact hLL p rp hk' hpL
      have hQH : ∀ q ∈ (if r.mutable then muts else all), H0 q := fun q hq => by
        cases hrm : r.mutable
        · rw [hrm] at hq; exact hallH q hq
        · rw [hrm] at hq; exact hallH q (hma q hq)
      have hdoneC : ∀ d ∈ done, Compat d r := fun d hd =>
        (List.pairwise_append.1 hpw).2.2 d hd r List.mem_cons_self
      -- Translate the call-time facts to the current graph.
      have hlift : ∀ q, (∃ p ∈ (if r.mutable then muts else all), ∃ rp, L p = some rp ∧
          r.location.Inside rp.location ∧ (r.mutable = true → rp.mutable = true) ∧
          (q = p ∨ Chain g0 L q p)) →
          ∃ p ∈ (if r.mutable then muts else all), ∃ rp, Lj p = some rp ∧
            r.location.Inside rp.location ∧ (r.mutable = true → rp.mutable = true) ∧
            (q = p ∨ Chain st.graph Lj q p) := by
        rintro q ⟨p, hp, rp, hpL, hin', hm', hqp⟩
        refine ⟨p, hp, rp, hkeyL p rp (hQH p hp) hpL, hin', hm', ?_⟩
        rcases hqp with h | h
        · exact Or.inl h
        · exact Or.inr (hmapC q p h)
      have hc2 := hc.addNode (Q := if r.mutable then muts else all) (c := st.nextId) (r := r)
        (fun q hq => hH0 q (hQH q hq)) hnew hadd
        (fun q rq hq hqL hqm hqin => by
          rcases hHj q rq hq hqL with ⟨hq0, hqL0⟩ | hd
          · exact hlift q (hD q rq hq0 hqL0 hqm hqin)
          · exfalso
            obtain ⟨h1, h2⟩ := (hdoneC rq hd).1 hqm
            rcases Inside.strict_or_eq hqin with h | h
            · exact h1 h
            · exact h2 h)
        (fun hrm x rx hx hxL heq => by
          rcases hHj x rx hx hxL with ⟨hx0, hxL0⟩ | hd
          · exact hlift x (hS hrm x rx hx0 hxL0 heq)
          · exact absurd heq ((hdoneC rx hd).2.1 hrm).2)
        (fun hrm x rx hx hxL hxin => by
          rcases hHj x rx hx hxL with ⟨hx0, hxL0⟩ | hd
          · exact hU hrm x rx hx0 hxL0 hxin
          · exact ((hdoneC rx hd).2.1 hrm).1 hxin)
        (fun hrm x rx hx hxL hxin => by
          rcases hHj x rx hx hxL with ⟨hx0, hxL0⟩ | hd
          · exact hI hrm x rx hx0 hxL0 hxin
          · exact (hdoneC rx hd).2.2.1 hrm hxin)
        (fun p rp hp hpL hpm hrin => by
          rcases hHj p rp hp hpL with ⟨hp0, hpL0⟩ | hd
          · exact hI2 p rp hp0 hpL0 hpm hrin
          · exact (hdoneC rp hd).2.2.2 hpm hrin)
        (fun i hi => by
          obtain ⟨p, hp, rp, hpL, hin', hm', hrc⟩ := hRL i hi
          exact ⟨p, hp, rp, hkeyL p rp (hQH p hp) hpL, hin', hm', hmapR _ p hrc⟩)
        (by
          rcases hPV with h | ⟨d, p, hp, rp, hpL, hin', hm', hrc⟩ | h
          · exact Or.inl h
          · exact Or.inr (Or.inl ⟨d, p, hp, rp, hkeyL p rp (hQH p hp) hpL, hin', hm',
              hmapR _ p hrc⟩)
          · exact Or.inr (Or.inr h))
      -- Continue with the other results.
      have hw1 := Graph.newRef_wf hnew hc.wf
      obtain ⟨-, -, hmut2, hcov2, -⟩ := addBorrows_props hadd hw1
      have hkey2 : ∀ x, (st.graph.find? x).isSome → (g2.find? x).isSome := fun x hx => by
        have hxc : x ≠ st.nextId := fun e => by subst e; rw [hfresh] at hx; cases hx
        rw [Graph.isSome_of_mutability (hmut2 x), ← Graph.isSome_of_mutability
          (Graph.newRef_mutability hnew x hxc).symm]
        exact hx
      have hcH : ¬ Hj st.nextId := hc.not_held_of_fresh hfresh
      obtain ⟨L', hc', hext, hmap⟩ := results_complete hc0 hallH hma hmuts hwr rs rv'
        { st with graph := g2, nextId := st.nextId + 1 } st' vs (updL Lj st.nextId r)
        (fun x => Hj x ∨ x = st.nextId) (done ++ [r]) h2 hlen' hkind' hnone'
        (fun r' hr' => hrc1 r' (List.mem_cons_of_mem _ hr')) (by simpa using hpw) hc2
        (hcov.trans ((Graph.newRef_covers hnew).trans hcov2))
        (fun x hx => hkey2 x (hkeys x hx))
        (fun x r' hx hxL => by
          have hxc : x ≠ st.nextId := fun e => by
            subst e; have := hkeys _ hx; rw [hfresh] at this; cases this
          rw [updL_ne _ hxc]; exact hLL x r' hx hxL)
        (fun x hx => Or.inl (hH0 x hx))
        (fun x rx hx hxL => by
          rcases hx with hx | rfl
          · have hxc : x ≠ st.nextId := fun e => hcH (e ▸ hx)
            rw [updL_ne _ hxc] at hxL
            rcases hHj x rx hx hxL with h | h
            · exact Or.inl h
            · exact Or.inr (List.mem_append_left _ h)
          · rw [updL_self, Option.some.injEq] at hxL
            subst hxL
            exact Or.inr (List.mem_append_right _ List.mem_cons_self))
      refine ⟨L', hc'.congrH fun x => ?_, fun x r' hx hxL => ?_, fun j x r' hj hr => ?_⟩
      · simp only [List.mem_cons, AbsValue.reference.injEq]
        constructor
        · rintro ((h | h) | h)
          · exact Or.inl h
          · exact Or.inr (Or.inl h)
          · exact Or.inr (Or.inr h)
        · rintro (h | h | h)
          · exact Or.inl (Or.inl h)
          · exact Or.inl (Or.inr h)
          · exact Or.inr h
      · have hxc : x ≠ st.nextId := fun e => hcH (e ▸ hx)
        exact hext x r' (Or.inl hx) (by rw [updL_ne _ hxc]; exact hxL)
      · cases j with
        | zero =>
          simp only [List.getElem?_cons_zero, Option.some.injEq, AbsValue.reference.injEq,
            Value.ref.injEq] at hj hr
          subst hj hr
          exact hext _ _ (Or.inr rfl) (updL_self _ _ _)
        | succ j => exact hmap j x r' (by simpa using hj) (by simpa using hr)

/-! ## Releasing several references -/

theorem releaseAll_complete {g g' : Graph} {L : Nat → Option Reference} {H : Nat → Prop}
    {root : Nat} {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference} :
    ∀ (all : List Nat), all.Nodup → (∀ a ∈ all, H a) → Complete g L H root fid gsid ps →
      releaseAll g all = .ok g' → Complete g' L (fun x => H x ∧ x ∉ all) root fid gsid ps
  | [], _, _, hc, h => by
    simp only [releaseAll, List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact hc.congrH fun x => by simp
  | a :: as, hnd, hH, hc, h => by
    simp only [releaseAll, List.foldlM_cons, bind, Except.bind] at h
    split at h
    · cases h
    · next g1 hg1 =>
      have hc1 := hc.release (hH a List.mem_cons_self) hg1
      have hnd' := List.nodup_cons.1 hnd
      have := releaseAll_complete (g := g1) as hnd'.2
        (fun b hb => ⟨hH b (List.mem_cons_of_mem _ hb), fun e => hnd'.1 (e ▸ hb)⟩) hc1 h
      refine this.congrH fun x => ?_
      simp only [List.mem_cons, not_or]
      constructor
      · rintro ⟨⟨h1, h2⟩, h3⟩; exact ⟨h1, h2, h3⟩
      · rintro ⟨h1, h2, h3⟩; exact ⟨⟨h1, h2⟩, h3⟩

theorem mem_localRefs_aux : ∀ (l : List AbsValue) (acc : List Nat) (x : Nat),
    x ∈ l.foldl refStep acc ↔ x ∈ acc ∨ AbsValue.reference x ∈ l
  | [], acc, x => by simp
  | v :: l, acc, x => by
    simp only [List.foldl_cons]
    rw [mem_localRefs_aux l _ x]
    cases v with
    | nonReference => simp [refStep]
    | reference id =>
      simp only [refStep]
      rw [mem_insertNat]
      simp only [List.mem_cons, AbsValue.reference.injEq]
      constructor
      · rintro ((h | h) | h)
        · exact Or.inr (Or.inl h)
        · exact Or.inl h
        · exact Or.inr (Or.inr h)
      · rintro (h | h | h)
        · exact Or.inl (Or.inr h)
        · exact Or.inl (Or.inl h)
        · exact Or.inr h

theorem mem_localRefs {l : List AbsValue} {x : Nat} :
    x ∈ localRefs l ↔ AbsValue.reference x ∈ l := by
  unfold localRefs
  rw [mem_localRefs_aux]
  simp

theorem localRefs_sorted_aux : ∀ (l : List AbsValue) (acc : List Nat), acc.Pairwise (· < ·) →
    (l.foldl refStep acc).Pairwise (· < ·)
  | [], acc, h => h
  | v :: l, acc, h => by
    simp only [List.foldl_cons]
    refine localRefs_sorted_aux l _ ?_
    cases v with
    | nonReference => exact h
    | reference id => exact insertNat_sorted id h

theorem localRefs_nodup (l : List AbsValue) : (localRefs l).Nodup :=
  (localRefs_sorted_aux l [] List.Pairwise.nil).imp Nat.ne_of_lt

/-- **Two held references are compatible** when every mutable one held is
writable. -/
theorem compat_of_complete {g : Graph} {L : Nat → Option Reference} {H : Nat → Prop}
    {root : Nat} {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}
    (hc : Complete g L H root fid gsid ps)
    (hwr : ∀ x rx, H x → L x = some rx → rx.mutable = true → g.isWritable x = .ok true)
    {x y : Nat} {rx ry : Reference} (hx : H x) (hy : H y) (hxy : x ≠ y) (hxL : L x = some rx)
    (hyL : L y = some ry) : Compat rx ry := by
  have hnoOut : ∀ z rz, H z → L z = some rz → rz.mutable = true → ∀ w, ¬ Chain g L z w := by
    intro z rz hz hzL hzm w h
    obtain ⟨e, n, -, -, -, -, he, -⟩ := h.first
    exact (Graph.isWritable_true (hwr z rz hz hzL hzm)).2 e n he
  have hpair : ∀ a b ra rb, H a → H b → a ≠ b → L a = some ra → L b = some rb →
      ra.mutable = true → ¬ rb.location.StrictlyInside ra.location ∧ rb.location ≠ ra.location := by
    intro a b ra rb ha hb hab haL hbL ham
    refine ⟨fun hin => hnoOut a ra ha haL ham b (hc.strict a ra b rb ha hb haL hbL ham hin),
      fun heq => ?_⟩
    rcases hc.same a ra b rb ha hb hab haL hbL ham heq with h | h
    · exact hnoOut a ra ha haL ham b h
    · cases hbm : rb.mutable
      · have := (h.facts hbL haL).2.2 hbm; rw [ham] at this; cases this
      · exact hnoOut b rb hb hbL hbm a h
  exact ⟨hpair x y rx ry hx hy hxy hxL hyL, hpair y x ry rx hy hx (Ne.symm hxy) hyL hxL,
    fun hm hin => hc.immInside y ry x rx hy hx hyL hxL hm hin,
    fun hm hin => hc.immInside x rx y ry hx hy hxL hyL hm hin⟩

end MoveBytecode.Proofs
