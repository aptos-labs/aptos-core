-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostLocations

/-!
# Writes at locations that do not overlap

UNTRUSTED (checked by the kernel). A native of the update shape writes back
through each of its mutable reference arguments, which overlap no other
reference argument (`Step.exclusiveArguments`). A write at a location leaves
every location it does not overlap reading as before
(`read_write_disjoint`), the type of every root value but its own
(`write_type`), and the frames, but for the locals of the frame it writes
into (`write_frames`).
-/

namespace MoveBytecode.Proofs

/-! ## One step inside a value -/

theorem withChild?_type {v w v' : Value} {st : PathStep} (h : v.withChild? st w = some v') :
    v'.type = v.type := by
  cases v <;> cases st <;> simp only [Value.withChild?] at h <;> try (simp at h; done)
  all_goals (split at h <;> simp only [Option.some.injEq, reduceCtorEq] at h <;> subst h <;> rfl)

theorem child_withChild_other {v c v' : Value} {s s' : PathStep} (h : v.withChild? s c = some v')
    (hne : s' ≠ s) : v'.child? s' = v.child? s' := by
  cases v <;> cases s <;> simp only [Value.withChild?] at h <;> try (simp at h; done)
  case struct.field sid fs i =>
    split at h
    · simp only [Option.some.injEq] at h
      subst h
      cases s' with
      | field j =>
        have hji : j ≠ i := fun e => hne (by rw [e])
        simp [Value.child?, Ne.symm hji]
      | variantField _ _ => rfl
      | element _ => rfl
      | signerAddress => rfl
      | tableEntry _ => rfl
    · simp at h
  case variant.variantField sid tag fs t i =>
    split at h
    · next hc =>
      obtain ⟨rfl, -⟩ := hc
      simp only [Option.some.injEq] at h
      subst h
      cases s' with
      | variantField t' j =>
        simp only [Value.child?]
        by_cases ht : tag = t'
        · subst ht
          have hji : j ≠ i := fun e => hne (by rw [e])
          simp [Ne.symm hji]
        · simp [ht]
      | field _ => rfl
      | element _ => rfl
      | signerAddress => rfl
      | tableEntry _ => rfl
    · simp at h
  case vector.element e es i =>
    split at h
    · simp only [Option.some.injEq] at h
      subst h
      cases s' with
      | element j =>
        have hji : j ≠ i := fun e => hne (by rw [e])
        simp [Value.child?, Ne.symm hji]
      | field _ => rfl
      | variantField _ _ => rfl
      | signerAddress => rfl
      | tableEntry _ => rfl
    · simp at h
  case table.field sid fs ks es i =>
    split at h
    · simp only [Option.some.injEq] at h
      subst h
      cases s' with
      | field j =>
        have hji : j ≠ i := fun e => hne (by rw [e])
        simp [Value.child?, Ne.symm hji]
      | _ => rfl
    · simp at h
  case table.tableEntry sid fs ks es i =>
    split at h
    · simp only [Option.some.injEq] at h
      subst h
      cases s' with
      | tableEntry j =>
        have hji : j ≠ i := fun e => hne (by rw [e])
        simp [Value.child?, Ne.symm hji]
      | _ => rfl
    · simp at h

theorem child_withChild_same {v c v' : Value} {s : PathStep} (h : v.withChild? s c = some v') :
    v'.child? s = some c := by
  cases v <;> cases s <;> simp only [Value.withChild?] at h <;> try (simp at h; done)
  case struct.field sid fs i =>
    split at h
    · next hi => simp only [Option.some.injEq] at h; subst h; simp [Value.child?, hi]
    · simp at h
  case variant.variantField sid tag fs t i =>
    split at h
    · next hc =>
      obtain ⟨rfl, hi⟩ := hc
      simp only [Option.some.injEq] at h
      subst h
      simp [Value.child?, hi]
    · simp at h
  case vector.element e es i =>
    split at h
    · next hi => simp only [Option.some.injEq] at h; subst h; simp [Value.child?, hi]
    · simp at h
  case table.field sid fs ks es i =>
    split at h
    · next hi => simp only [Option.some.injEq] at h; subst h; simp [Value.child?, hi]
    · simp at h
  case table.tableEntry sid fs ks es i =>
    split at h
    · next hi => simp only [Option.some.injEq] at h; subst h; simp [Value.child?, hi]
    · simp at h

/-- Changing one child keeps every step's child index: a table keeps its
number of fields. -/
theorem childIndex_withChild {v w v' : Value} {st : PathStep} (h : v.withChild? st w = some v')
    (st' : PathStep) : v'.childIndex st' = v.childIndex st' := by
  cases v <;> cases st <;> simp only [Value.withChild?] at h <;> try (simp at h; done)
  all_goals (split at h <;> simp only [Option.some.injEq, reduceCtorEq] at h <;> subst h <;>
    cases st' <;> simp [Value.childIndex])

/-! ## Paths -/

/-- Writing a nonempty path keeps the value's type. -/
theorem set?_type_cons {v w v' : Value} {st : PathStep} {p : List PathStep}
    (h : v.set? (st :: p) w = some v') : v'.type = v.type := by
  simp only [Value.set?, bind, Option.bind_eq_some_iff] at h
  obtain ⟨_, -, _, -, h⟩ := h
  exact withChild?_type h

/-- **Writing a path leaves every path it does not overlap as it was.** -/
theorem get_set_disjoint : ∀ (p q : List PathStep) {v w v' : Value}, v.set? p w = some v' →
    ¬ p <+: q → ¬ q <+: p → v'.get? q = v.get? q
  | [], _, _, _, _, _, h1, _ => absurd List.nil_prefix h1
  | _ :: _, [], _, _, _, _, _, h2 => absurd List.nil_prefix h2
  | s :: p, s' :: q, v, w, v', hset, h1, h2 => by
    simp only [Value.set?, bind, Option.bind_eq_some_iff] at hset
    obtain ⟨c, hc, c', hc', hv'⟩ := hset
    by_cases hss : s' = s
    · subst hss
      have h1' : ¬ p <+: q := fun h => h1 ((List.prefix_cons_inj _).2 h)
      have h2' : ¬ q <+: p := fun h => h2 ((List.prefix_cons_inj _).2 h)
      simp only [Value.get?, child_withChild_same hv', hc, bind, Option.bind_some]
      exact get_set_disjoint p q hc' h1' h2'
    · simp only [Value.get?, child_withChild_other hv' hss, bind]

/-- **Writing a path leaves the node indices of every path it does not
overlap** (`Value.indicesOf`). -/
theorem indicesOf_set_disjoint : ∀ (p q : List PathStep) {v w v' : Value}, v.set? p w = some v' →
    ¬ p <+: q → ¬ q <+: p → v'.indicesOf q = v.indicesOf q
  | [], _, _, _, _, _, h1, _ => absurd List.nil_prefix h1
  | _ :: _, [], _, _, _, _, _, h2 => absurd List.nil_prefix h2
  | s :: p, s' :: q, v, w, v', hset, h1, h2 => by
    simp only [Value.set?, bind, Option.bind_eq_some_iff] at hset
    obtain ⟨c, hc, c', hc', hv'⟩ := hset
    by_cases hss : s' = s
    · subst hss
      have h1' : ¬ p <+: q := fun h => h1 ((List.prefix_cons_inj _).2 h)
      have h2' : ¬ q <+: p := fun h => h2 ((List.prefix_cons_inj _).2 h)
      simp only [Value.indicesOf, childIndex_withChild hv', child_withChild_same hv', hc]
      rw [indicesOf_set_disjoint p q hc' h1' h2']
    · simp only [Value.indicesOf, childIndex_withChild hv', child_withChild_other hv' hss]

/-! ## States -/

theorem root?_setRoot_other {s : State} {ρ ρ' : Root} {v : Value} (hne : ρ' ≠ ρ) :
    (s.setRoot ρ v).root? ρ' = s.root? ρ' := by
  cases ρ with
  | local_ f i =>
    cases ρ' with
    | local_ f' i' =>
      simp only [State.setRoot, State.root?, State.frame?, bind]
      rw [List.find?_map]
      by_cases hf : f' = f
      · subst hf
        have hi : i' ≠ i := fun e => hne (by rw [e])
        have hcomp : ((fun fr : Frame => fr.id == f') ∘ fun fr =>
            if fr.id == f' then { fr with locals := fr.locals.setIfInBounds i (some v) } else fr) =
            fun fr => fr.id == f' := by
          funext fr; simp only [Function.comp]; split <;> rfl
        rw [hcomp]
        cases hfd : s.frames.find? (fun fr => fr.id == f') with
        | none => rfl
        | some fr =>
          have hid := List.find?_some hfd
          simp only [Option.map_some, Option.bind_some, hid, ↓reduceIte]
          rw [Array.getElem?_setIfInBounds_ne (Ne.symm hi)]
      · have hcomp : ((fun fr : Frame => fr.id == f') ∘ fun fr =>
            if fr.id == f then { fr with locals := fr.locals.setIfInBounds i (some v) } else fr) =
            fun fr => fr.id == f' := by
          funext fr; simp only [Function.comp]; split <;> rfl
        rw [hcomp]
        cases hfd : s.frames.find? (fun fr => fr.id == f') with
        | none => rfl
        | some fr =>
          have hid : fr.id = f' := by simpa using List.find?_some hfd
          have : (fr.id == f) = false := by simp [hid, hf]
          simp only [Option.map_some, Option.bind_some, this, Bool.false_eq_true, ↓reduceIte]
    | held _ => rfl
    | global _ _ => rfl
  | held k =>
    cases ρ' with
    | held k' =>
      have hk : k' ≠ k := fun e => hne (by rw [e])
      simp only [State.setRoot, State.root?]
      rw [List.getElem?_set_ne (Ne.symm hk)]
    | local_ _ _ => rfl
    | global _ _ => rfl
  | global a t =>
    cases ρ' with
    | global a' t' =>
      simp only [State.setRoot, State.root?]
      have hne' : ¬ (a = a' ∧ t = t') := fun ⟨e1, e2⟩ => hne (by rw [e1, e2])
      unfold GlobalStorage.get? GlobalStorage.update
      rw [List.find?_map]
      have hp : ((fun r : Resource => r.address == a' && r.type == t') ∘
          fun r : Resource => if r.address == a && r.type == t then { r with value := v } else r) =
          fun r : Resource => r.address == a' && r.type == t' := by
        funext r; simp only [Function.comp]; split <;> rfl
      rw [hp]
      cases hf : s.storage.find? (fun r : Resource => r.address == a' && r.type == t') with
      | none => rfl
      | some r =>
        have hr := List.find?_some hf
        simp only [Bool.and_eq_true, beq_iff_eq] at hr
        simp only [Option.map_some, Option.some.injEq]
        split
        · next hc =>
          simp only [Bool.and_eq_true, beq_iff_eq] at hc
          exact absurd ⟨hc.1.symm.trans hr.1, hc.2.symm.trans hr.2⟩ hne'
        · rfl
    | local_ _ _ => rfl
    | held _ => rfl

theorem root?_setRoot_self {s : State} {ρ : Root} {v w : Value} (hv : s.root? ρ = some w) :
    (s.setRoot ρ v).root? ρ = some v := by
  cases ρ with
  | local_ f i =>
    simp only [State.root?, State.frame?, bind, Option.bind_eq_some_iff] at hv
    obtain ⟨fr, hfr, o, ho, rfl⟩ := hv
    simp only [State.setRoot, State.root?, State.frame?, bind]
    rw [List.find?_map]
    have hcomp : ((fun fr : Frame => fr.id == f) ∘ fun fr =>
        if fr.id == f then { fr with locals := fr.locals.setIfInBounds i (some v) } else fr) =
        fun fr => fr.id == f := by
      funext fr; simp only [Function.comp]; split <;> rfl
    rw [hcomp, hfr]
    have hid := List.find?_some hfr
    simp only [Option.map_some, Option.bind_some, hid, ↓reduceIte]
    have hlt : i < fr.locals.size := (Array.getElem?_eq_some_iff.1 ho).1
    simp [hlt]
  | held k =>
    simp only [State.root?] at hv
    have hk : k < s.held.length := (List.getElem?_eq_some_iff.1 hv).1
    simp [State.setRoot, State.root?, hk]
  | global a t =>
    simp only [State.root?] at hv
    simp only [State.setRoot, State.root?]
    unfold GlobalStorage.get? at hv ⊢
    unfold GlobalStorage.update
    rw [List.find?_map]
    have hp : ((fun r : Resource => r.address == a && r.type == t) ∘
        fun r : Resource => if r.address == a && r.type == t then { r with value := v } else r) =
        fun r : Resource => r.address == a && r.type == t := by
      funext r; simp only [Function.comp]; split <;> rfl
    rw [hp]
    cases hf : s.storage.find? (fun r : Resource => r.address == a && r.type == t) with
    | none => rw [hf] at hv; cases hv
    | some r =>
      have hr := List.find?_some hf
      simp only [Bool.and_eq_true, beq_iff_eq] at hr
      simp [hr]

/-- **A write leaves every location it does not overlap reading as before.** -/
theorem read_write_disjoint {s s₂ : State} {l m : Location} {w : Value}
    (hwr : s.write? l w = some s₂) (hno : l.overlaps m = false) : s₂.read? m = s.read? m := by
  simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hwr
  obtain ⟨rv, hrv, v'', hset, rfl⟩ := hwr
  by_cases hr : m.root = l.root
  · simp only [Location.overlaps, hr, beq_self_eq_true, Bool.true_and, Bool.or_eq_false_iff] at hno
    simp only [State.read?, hr, root?_setRoot_self hrv, hrv, bind, Option.bind_some]
    exact get_set_disjoint _ _ hset
      (fun h => by rw [List.isPrefixOf_iff_prefix.2 h] at hno; cases hno.1)
      (fun h => by rw [List.isPrefixOf_iff_prefix.2 h] at hno; cases hno.2)
  · simp only [State.read?, root?_setRoot_other hr]

/-- **A write leaves the node indices of every location it does not overlap**,
in the value at its root (`Location.indicesIn`). -/
theorem indicesIn_write_disjoint {s s₂ : State} {l m : Location} {w : Value}
    (hwr : s.write? l w = some s₂) (hno : l.overlaps m = false) :
    (s₂.root? m.root).map m.indicesIn = (s.root? m.root).map m.indicesIn := by
  simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hwr
  obtain ⟨rv, hrv, v'', hset, rfl⟩ := hwr
  by_cases hr : m.root = l.root
  · simp only [Location.overlaps, hr, beq_self_eq_true, Bool.true_and, Bool.or_eq_false_iff] at hno
    simp only [hr, root?_setRoot_self hrv, hrv, Option.map_some, Option.some.injEq,
      Location.indicesIn]
    exact indicesOf_set_disjoint _ _ hset
      (fun h => by rw [List.isPrefixOf_iff_prefix.2 h] at hno; cases hno.1)
      (fun h => by rw [List.isPrefixOf_iff_prefix.2 h] at hno; cases hno.2)
  · simp only [root?_setRoot_other hr]

end MoveBytecode.Proofs
