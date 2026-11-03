-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostTyping
import Proofs.Certified

/-!
# Values along paths

UNTRUSTED (checked by the kernel). The value at a path inside a well-formed
value of type `T` is well formed and of the path's type (`pathType`); writing
a well-formed value of that type there keeps the whole well formed and of type
`T`; and every path that is not strictly inside the written one still
resolves after the write.
-/

namespace MoveBytecode.Proofs

variable {P : Program}

/-- A value conforming to a value type is not a reference. -/
theorem not_ref_of_conforms {v : Value} {t : RuntimeType} (hv : v.conforms P t = true)
    (ht : t.isValue = true) : ∀ r, v ≠ .ref r :=
  not_ref_of_type rfl (by rw [assignable_isValue (conforms_assignable hv)]; exact ht)

/-! ## Rigid containers

A path goes into a struct, an enum or a vector, whose types are rigid: a value
assignable to such a type has that type. -/

theorem assignable_rigid_left {t u : RuntimeType} (h : t.assignable u = true)
    (ht : rigid t = true) : t = u := by
  rcases assignable_cases h with ⟨_, _, _, _, rfl, -, -⟩ | ⟨_, _, rfl, -, -⟩ | rfl
  · simp [rigid] at ht
  · simp [rigid] at ht
  · rfl

theorem stepType_rigid {t u : RuntimeType} {step : PathStep} (h : stepType P t step = some u) :
    rigid t = true := by
  cases t <;> cases step <;> simp_all [stepType, rigid]

/-! ## Lists of conforming values -/

theorem conformsAll_set : ∀ {vs : List Value} {ts : List RuntimeType} {i : Nat} {v w : Value},
    conformsAll P vs ts = true → vs[i]? = some v →
      (∀ t, ts[i]? = some t → w.conforms P t = true) →
      conformsAll P (vs.set i w) ts = true
  | [], _, _, _, _, _, hv, _ => by simp at hv
  | u :: us, [], _, _, _, h, _, _ => by simp [conformsAll] at h
  | u :: us, t :: ts, i, v, w, h, hv, hw => by
    rw [conformsAll_cons] at h
    cases i with
    | zero =>
      simp only [List.set_cons_zero, conformsAll_cons]
      exact ⟨hw t rfl, h.2⟩
    | succ i =>
      simp only [List.set_cons_succ, conformsAll_cons]
      exact ⟨h.1, conformsAll_set h.2 (by simpa using hv) (fun t' ht' => hw t' (by simpa using ht'))⟩

theorem allConform_set {vs : List Value} {t : RuntimeType} {i : Nat} {v w : Value}
    (h : Value.allConform P vs t = true) (_hv : vs[i]? = some v) (hw : w.conforms P t = true) :
    Value.allConform P (vs.set i w) t = true := by
  rw [allConform_iff] at h ⊢
  intro u hu
  rcases List.mem_or_eq_of_mem_set hu with hu | rfl
  · exact h u hu
  · exact hw

/-! ## One step -/

theorem child_wf {v c : Value} {step : PathStep} (hv : v.wellFormed P = true)
    (hc : v.child? step = some c) : ∃ t, stepType P v.type step = some t ∧ c.conforms P t = true := by
  cases v <;> cases step <;> simp only [Value.child?] at hc <;> try (simp at hc; done)
  case signer.signerAddress a =>
    simp only [Option.some.injEq] at hc
    subst hc
    exact ⟨.address, rfl, by simp [Value.conforms, Value.wellFormed, Value.type]⟩
  case vector.element e es i =>
    simp only [Value.wellFormed, allConform_iff] at hv
    exact ⟨e, rfl, hv c (List.mem_of_getElem? hc)⟩
  case struct.field s fs i =>
    obtain ⟨-, tys, htys, hv⟩ := wellFormed_struct.1 hv
    obtain ⟨t, ht, hct⟩ := conformsAll_getElem hv hc
    exact ⟨t, by show stepType P (.struct s) (.field i) = some t; simp [stepType, htys, ht], hct⟩
  case variant.variantField s tag fs t i =>
    split at hc
    · next ht =>
      subst ht
      simp only [Value.wellFormed] at hv
      split at hv
      · next tys htys =>
        obtain ⟨t', ht', hct⟩ := conformsAll_getElem hv hc
        exact ⟨t', by
          show stepType P (.struct s) (.variantField tag i) = some t'
          simp [stepType, htys, ht'], hct⟩
      · simp at hv
    · simp at hc
  case table.field s fs ks es i =>
    obtain ⟨-, -, ⟨tys, htys, hv⟩, -⟩ := wellFormed_table.1 hv
    obtain ⟨t, ht, hct⟩ := conformsAll_getElem hv hc
    exact ⟨t, by show stepType P (.struct s) (.field i) = some t; simp [stepType, htys, ht], hct⟩
  case table.tableEntry s fs ks es i =>
    obtain ⟨hst, -, -, hes⟩ := wellFormed_table.1 hv
    rcases hes with rfl | ⟨-, k, v, hargs, -, hce, -⟩
    · simp at hc
    · exact ⟨tableEntryType v, by simp [Value.type, stepType, hst, hargs],
        allConform_iff.1 hce c (List.mem_of_getElem? hc)⟩

theorem withChild_wf {v c w v' : Value} {step : PathStep} (hv : v.wellFormed P = true)
    (hc : v.child? step = some c) (hw : ∀ t, stepType P v.type step = some t → w.conforms P t = true)
    (hv' : v.withChild? step w = some v') : v'.wellFormed P = true ∧ v'.type = v.type := by
  cases v <;> cases step <;> simp only [Value.child?] at hc <;> try (simp at hc; done)
  case signer.signerAddress => simp [Value.withChild?] at hv'
  case vector.element e es i =>
    simp only [Value.withChild?] at hv'
    split at hv'
    · simp only [Option.some.injEq] at hv'
      subst hv'
      simp only [Value.wellFormed] at hv ⊢
      exact ⟨allConform_set hv hc (hw e rfl), rfl⟩
    · simp at hv'
  case struct.field s fs i =>
    simp only [Value.withChild?] at hv'
    split at hv'
    · simp only [Option.some.injEq] at hv'
      subst hv'
      obtain ⟨hst, tys, htys, hv⟩ := wellFormed_struct.1 hv
      refine ⟨wellFormed_struct.2 ⟨hst, tys, htys, ?_⟩, rfl⟩
      exact conformsAll_set hv hc (fun t ht => hw t (by simp [Value.type, stepType, htys, ht]))
    · simp at hv'
  case table.field s fs ks es i =>
    simp only [Value.withChild?] at hv'
    split at hv'
    · simp only [Option.some.injEq] at hv'
      subst hv'
      obtain ⟨hst, hlen, ⟨tys, htys, hv⟩, hes⟩ := wellFormed_table.1 hv
      refine ⟨wellFormed_table.2 ⟨hst, hlen, ⟨tys, htys, ?_⟩, hes⟩, rfl⟩
      exact conformsAll_set hv hc (fun t ht => hw t (by simp [Value.type, stepType, htys, ht]))
    · simp at hv'
  case table.tableEntry s fs ks es i =>
    simp only [Value.withChild?] at hv'
    split at hv'
    · simp only [Option.some.injEq] at hv'
      subst hv'
      obtain ⟨hst, hlen, hf, hes⟩ := wellFormed_table.1 hv
      rcases hes with rfl | ⟨htab, k, v, hargs, hck, hce, hnd, hcopy, hsub⟩
      · simp at hc
      · refine ⟨wellFormed_table.2 ⟨hst, by simpa using hlen, hf, Or.inr ⟨htab, k, v, hargs, hck,
          allConform_set hce hc (hw _ (by simp [Value.type, stepType, hst, hargs])), hnd, hcopy,
          hsub⟩⟩, rfl⟩
    · simp at hv'
  case variant.variantField s tag fs t i =>
    split at hc
    · next ht =>
      subst ht
      simp only [Value.withChild?, true_and] at hv'
      split at hv'
      · simp only [Option.some.injEq] at hv'
        subst hv'
        simp only [Value.wellFormed] at hv ⊢
        split at hv
        · next tys htys =>
          refine ⟨?_, rfl⟩
          have := conformsAll_set hv hc (fun t' ht' => hw t' (by simp [Value.type, stepType, htys, ht']))
          simpa [htys] using this
        · simp at hv
      · simp at hv'
    · simp at hc

/-! ## Paths -/

theorem get_wf : ∀ (path : List PathStep) {v w : Value}, v.wellFormed P = true →
    v.get? path = some w → ∃ t, pathType P v.type path = some t ∧ w.conforms P t = true
  | [], v, w, hv, hw => by
    simp only [Value.get?, Option.some.injEq] at hw
    subst hw
    exact ⟨v.type, rfl, conforms_of (assignable_refl _) hv⟩
  | step :: path, v, w, hv, hw => by
    obtain ⟨c, hc, hw⟩ := get_cons hw
    obtain ⟨t1, h1, hct⟩ := child_wf hv hc
    cases path with
    | nil =>
      simp only [Value.get?, Option.some.injEq] at hw
      subst hw
      exact ⟨t1, by simp [pathType, h1], hct⟩
    | cons s' path =>
      obtain ⟨t2, h2, hw2⟩ := get_wf (s' :: path) (conforms_wellFormed hct) hw
      have hcr : rigid c.type = true := by
        simp only [pathType, bind, Option.bind_eq_some_iff] at h2
        obtain ⟨_, hs, -⟩ := h2
        exact stepType_rigid hs
      have hce := assignable_rigid_left (conforms_assignable hct) hcr
      rw [hce] at h2
      exact ⟨t2, by simp [pathType, h1]; simpa [pathType] using h2, hw2⟩

/-- **Reading at a path** of a value conforming to `T` gives a value
conforming to the path's type. -/
theorem get_conforms {v w : Value} {T T' : RuntimeType} {path : List PathStep}
    (hv : v.conforms P T = true) (hw : v.get? path = some w) (hT : pathType P T path = some T') :
    w.conforms P T' = true := by
  cases path with
  | nil =>
    simp only [Value.get?, Option.some.injEq] at hw
    simp only [pathType, Option.some.injEq] at hT
    subst hw hT
    exact hv
  | cons s path =>
    have hTr : rigid T = true := by
      simp only [pathType, bind, Option.bind_eq_some_iff] at hT
      obtain ⟨_, hs, -⟩ := hT
      exact stepType_rigid hs
    obtain ⟨t, ht, hwt⟩ := get_wf (s :: path) (conforms_wellFormed hv) hw
    rw [conforms_type hv hTr, hT, Option.some.injEq] at ht
    subst ht
    exact hwt

theorem set_wf : ∀ (path : List PathStep) {v w w' v' : Value}, v.wellFormed P = true →
    v.get? path = some w → (∀ t, pathType P v.type path = some t → w'.conforms P t = true) →
      v.set? path w' = some v' → v'.wellFormed P = true ∧ v'.type.assignable v.type = true
  | [], v, w, w', v', hv, hw, hw', hv' => by
    simp only [Value.set?, Option.some.injEq] at hv'
    subst hv'
    have := hw' v.type rfl
    exact ⟨conforms_wellFormed this, conforms_assignable this⟩
  | step :: path, v, w, w', v', hv, hw, hw', hv' => by
    obtain ⟨c, hc, hw⟩ := get_cons hw
    obtain ⟨t1, h1, hct⟩ := child_wf hv hc
    simp only [Value.set?, bind, Option.bind_eq_some_iff, hc, Option.bind_some] at hv'
    obtain ⟨c', hc', hv'⟩ := hv'
    have hc'c : c'.conforms P t1 = true := by
      cases path with
      | nil =>
        simp only [Value.set?, Option.some.injEq] at hc'
        subst hc'
        exact hw' t1 (by simp [pathType, h1])
      | cons s' path =>
        obtain ⟨t2, h2, -⟩ := get_wf (s' :: path) (conforms_wellFormed hct) hw
        have hcr : rigid c.type = true := by
          simp only [pathType, bind, Option.bind_eq_some_iff] at h2
          obtain ⟨_, hs, -⟩ := h2
          exact stepType_rigid hs
        have hce := assignable_rigid_left (conforms_assignable hct) hcr
        obtain ⟨hcs', hcty⟩ := set_wf (s' :: path) (conforms_wellFormed hct) hw
          (fun t ht => hw' t (by rw [← hce] at h1; simp [pathType, h1]; simpa [pathType] using ht))
          hc'
        exact conforms_of (by rw [← hce]; exact hcty) hcs'
    obtain ⟨hv1, hv2⟩ := withChild_wf hv hc
      (fun t ht => by rw [h1, Option.some.injEq] at ht; subst ht; exact hc'c) hv'
    exact ⟨hv1, by rw [hv2]; exact assignable_refl _⟩

/-- **Writing at a path** a value conforming to the path's type keeps a value
conforming to `T`. -/
theorem set_conforms {v w' v' : Value} {T T' : RuntimeType} {path : List PathStep}
    (hv : v.conforms P T = true) (hT : pathType P T path = some T')
    (hw' : w'.conforms P T' = true) (hv' : v.set? path w' = some v') :
    v'.conforms P T = true := by
  -- The path resolves: `set?` succeeded.
  have hres : ∀ (path : List PathStep) (v w' v' : Value), v.set? path w' = some v' →
      ∃ w, v.get? path = some w := by
    intro path
    induction path with
    | nil => intro v w' v' _; exact ⟨v, rfl⟩
    | cons step path ih =>
      intro v w' v' h
      simp only [Value.set?, bind, Option.bind_eq_some_iff] at h
      obtain ⟨c, hc, c', hc', -⟩ := h
      obtain ⟨w, hw⟩ := ih c w' c' hc'
      exact ⟨w, by simp [Value.get?, hc, hw]⟩
  cases path with
  | nil =>
    simp only [Value.set?, Option.some.injEq] at hv'
    simp only [pathType, Option.some.injEq] at hT
    subst hv' hT
    exact hw'
  | cons s path =>
    have hTr : rigid T = true := by
      simp only [pathType, bind, Option.bind_eq_some_iff] at hT
      obtain ⟨_, hs, -⟩ := hT
      exact stepType_rigid hs
    have hvT := conforms_type hv hTr
    obtain ⟨w, hw⟩ := hres (s :: path) v w' v' hv'
    obtain ⟨h1, h2⟩ := set_wf (s :: path) (conforms_wellFormed hv) hw
      (fun t ht => by rw [hvT, hT, Option.some.injEq] at ht; subst ht; exact hw') hv'
    exact conforms_of (by rw [← hvT]; exact h2) h1

/-! ## What a write leaves resolvable -/

theorem child_withChild_ne {v c v' : Value} {s s' : PathStep} (h : v.withChild? s c = some v')
    (hne : s' ≠ s) : v'.child? s' = v.child? s' :=
  child_withChild_other h hne

theorem child_withChild_self {v c v' : Value} {s : PathStep} (h : v.withChild? s c = some v') :
    v'.child? s = some c :=
  child_withChild_same h

/-- **A write at path `p` keeps every path `q` not strictly inside `p`
resolvable.** -/
theorem get_after_set : ∀ (p q : List PathStep) {v w v' u : Value},
    v.set? p w = some v' → ¬ (p <+: q ∧ p ≠ q) → v.get? q = some u →
      ∃ u', v'.get? q = some u'
  | [], q, v, w, v', u, hset, hnot, hq => by
    cases q with
    | nil =>
      simp only [Value.set?, Option.some.injEq] at hset
      exact ⟨v', rfl⟩
    | cons a q => exact absurd ⟨List.nil_prefix, by simp⟩ hnot
  | s :: p, [], v, w, v', u, hset, hnot, hq => ⟨v', rfl⟩
  | s :: p, s' :: q, v, w, v', u, hset, hnot, hq => by
    simp only [Value.set?, bind, Option.bind_eq_some_iff] at hset
    obtain ⟨c, hc, c', hc', hv'⟩ := hset
    obtain ⟨d, hd, hq⟩ := get_cons hq
    by_cases hss : s' = s
    · subst hss
      rw [hc, Option.some.injEq] at hd
      subst hd
      have hnot' : ¬ (p <+: q ∧ p ≠ q) := fun ⟨h1, h2⟩ =>
        hnot ⟨(List.prefix_cons_inj _).2 h1, fun e => h2 (List.cons.inj e).2⟩
      obtain ⟨u', hu'⟩ := get_after_set p q hc' hnot' hq
      exact ⟨u', by simp [Value.get?, child_withChild_self hv', hu']⟩
    · exact ⟨u, by simp [Value.get?, child_withChild_ne hv' hss, hd, hq]⟩

theorem get_append_step {v : Value} {path : List PathStep} {step : PathStep} :
    v.get? (path ++ [step]) = (v.get? path).bind (·.child? step) := by
  induction path generalizing v with
  | nil => simp [Value.get?]
  | cons s path ih =>
    simp only [List.cons_append, Value.get?, bind, Option.bind]
    cases v.child? s with
    | none => rfl
    | some c => exact ih

end MoveBytecode.Proofs
