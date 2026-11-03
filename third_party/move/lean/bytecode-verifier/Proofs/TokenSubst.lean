-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Declarations

/-!
# Substitution preserves well-formed types (L5)

UNTRUSTED (checked by the kernel). A token well formed where type parameters
have constraints `cs`, instantiated by type arguments well formed where type
parameters have constraints `tparams` and satisfying `cs`, is well formed
where type parameters have constraints `tparams`, with at least the abilities
the original token's handles declare (`vtwf_instantiate`,
`abilitiesIn_instantiate`). A closed token instantiates to itself
(`instantiate_closed`).
-/

namespace MoveBytecode.Proofs

mutual
/-- A closed value type (no type parameters in scope) instantiates to
itself. -/
theorem instantiate_closed {m : CompiledModule} :
    ∀ (t : SignatureToken), t.ValueTypeWellFormed m [] → t.instantiate [] = some t
  | .bool, _ | .u8, _ | .u16, _ | .u32, _ | .u64, _ | .u128, _ | .u256, _ => rfl
  | .i8, _ | .i16, _ | .i32, _ | .i64, _ | .i128, _ | .i256, _ => rfl
  | .address, _ | .signer, _ | .struct _, _ => rfl
  | .vector e, h => by
    simp only [SignatureToken.ValueTypeWellFormed] at h
    simp [SignatureToken.instantiate, instantiate_closed e h]
  | .function ps rs ab, h => by
    obtain ⟨hp, hr, -⟩ := h
    simp [SignatureToken.instantiate, instantiateSlots_closed ps hp,
      instantiateSlots_closed rs hr]
  | .structInstantiation hh args, h => by
    obtain ⟨sh, -, ha⟩ := h
    simp [SignatureToken.instantiate, instantiateArgs_closed args _ ha]
  | .reference _, h | .mutableReference _, h => h.elim
  | .typeParameter i, h => absurd h (Nat.not_lt_zero i)

theorem instantiateSlots_closed {m : CompiledModule} :
    ∀ (ts : List SignatureToken), SignatureToken.SlotsWellFormed m [] ts →
      SignatureToken.instantiateList [] ts = some ts
  | [], _ => rfl
  | .reference t :: ts, h => by
    obtain ⟨h1, h2⟩ := h
    simp [SignatureToken.instantiateList, SignatureToken.instantiate, instantiate_closed t h1,
      instantiateSlots_closed ts h2]
  | .mutableReference t :: ts, h => by
    obtain ⟨h1, h2⟩ := h
    simp [SignatureToken.instantiateList, SignatureToken.instantiate, instantiate_closed t h1,
      instantiateSlots_closed ts h2]
  | .bool :: ts, h | .u8 :: ts, h | .u16 :: ts, h | .u32 :: ts, h | .u64 :: ts, h
  | .u128 :: ts, h | .u256 :: ts, h | .i8 :: ts, h | .i16 :: ts, h | .i32 :: ts, h
  | .i64 :: ts, h | .i128 :: ts, h | .i256 :: ts, h | .address :: ts, h | .signer :: ts, h
  | .struct _ :: ts, h | .vector _ :: ts, h | .function _ _ _ :: ts, h
  | .structInstantiation _ _ :: ts, h | .typeParameter _ :: ts, h => by
    obtain ⟨h1, h2⟩ := h
    simp [SignatureToken.instantiateList, instantiate_closed _ h1, instantiateSlots_closed ts h2]

theorem instantiateArgs_closed {m : CompiledModule} :
    ∀ (args : List SignatureToken) (cs : List AbilitySet),
      SignatureToken.ArgumentsWellFormed m [] args cs →
      SignatureToken.instantiateList [] args = some args
  | [], [], _ => rfl
  | a :: args, _ :: cs, h => by
    obtain ⟨h1, -, h3⟩ := h
    simp [SignatureToken.instantiateList, instantiate_closed a h1, instantiateArgs_closed args cs h3]
  | [], _ :: _, h => h.elim
  | _ :: _, [], h => h.elim
end

/-! ## Well-formed type arguments -/

section
variable {m : CompiledModule} {tparams : List AbilitySet}

theorem argsWF_length :
    ∀ {σ : List SignatureToken} {cs : List AbilitySet},
      SignatureToken.ArgumentsWellFormed m tparams σ cs → σ.length = cs.length
  | [], [], _ => rfl
  | _ :: _, _ :: _, h => by
    obtain ⟨-, -, h3⟩ := h
    simp [argsWF_length h3]
  | [], _ :: _, h | _ :: _, [], h => h.elim

theorem argsWF_getElem :
    ∀ {σ : List SignatureToken} {cs : List AbilitySet},
      SignatureToken.ArgumentsWellFormed m tparams σ cs → ∀ {i : Nat} {q : SignatureToken}
        {c : AbilitySet}, σ[i]? = some q → cs[i]? = some c →
        q.ValueTypeWellFormed m tparams ∧ c.subset (q.abilitiesIn m tparams) = true
  | [], _, _, _, _, _, hq, _ => by simp at hq
  | _ :: _, [], h, _, _, _, _, _ => h.elim
  | a :: σ, c :: cs, h, i, q, c', hq, hc => by
    obtain ⟨h1, h2, h3⟩ := h
    cases i with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hq hc
      subst hq hc
      exact ⟨h1, h2⟩
    | succ i =>
      simp only [List.getElem?_cons_succ] at hq hc
      exact argsWF_getElem h3 hq hc

theorem slotsWF_cons {t : SignatureToken} {ts : List SignatureToken}
    (h1 : SignatureToken.SlotsWellFormed m tparams [t])
    (h2 : SignatureToken.SlotsWellFormed m tparams ts) :
    SignatureToken.SlotsWellFormed m tparams (t :: ts) := by
  cases t <;> simp_all [SignatureToken.SlotsWellFormed]

theorem slotsWF_of_vtwf {t : SignatureToken} (h : t.ValueTypeWellFormed m tparams) :
    SignatureToken.SlotsWellFormed m tparams [t] := by
  cases t <;> simp_all [SignatureToken.SlotsWellFormed, SignatureToken.ValueTypeWellFormed]

theorem slotsWF_head {t : SignatureToken} {ts : List SignatureToken}
    (h : SignatureToken.SlotsWellFormed m tparams (t :: ts)) :
    SignatureToken.SlotsWellFormed m tparams [t] := by
  cases t <;> simp_all [SignatureToken.SlotsWellFormed]

theorem slotsWF_tail {t : SignatureToken} {ts : List SignatureToken}
    (h : SignatureToken.SlotsWellFormed m tparams (t :: ts)) :
    SignatureToken.SlotsWellFormed m tparams ts := by
  cases t <;> simp_all [SignatureToken.SlotsWellFormed]

theorem slotsWF_iff : ∀ {ts : List SignatureToken},
    SignatureToken.SlotsWellFormed m tparams ts ↔
      ∀ t ∈ ts, SignatureToken.SlotsWellFormed m tparams [t]
  | [] => by simp [SignatureToken.SlotsWellFormed]
  | t :: ts => by
    constructor
    · intro h u hu
      rcases List.mem_cons.1 hu with rfl | hu
      · exact slotsWF_head h
      · exact slotsWF_iff.1 (slotsWF_tail h) u hu
    · intro h
      exact slotsWF_cons (h t (List.mem_cons_self ..))
        (slotsWF_iff.2 fun u hu => h u (List.mem_cons_of_mem _ hu))

theorem slotsWF_append : ∀ {a b : List SignatureToken},
    SignatureToken.SlotsWellFormed m tparams (a ++ b) ↔
      SignatureToken.SlotsWellFormed m tparams a ∧ SignatureToken.SlotsWellFormed m tparams b
  | [], _ => by simp [SignatureToken.SlotsWellFormed]
  | t :: a, b => by
    rw [List.cons_append]
    constructor
    · intro h
      obtain ⟨h1, h2⟩ := slotsWF_append.1 (slotsWF_tail h)
      exact ⟨slotsWF_cons (slotsWF_head h) h1, h2⟩
    · rintro ⟨h1, h2⟩
      exact slotsWF_cons (slotsWF_head h1) (slotsWF_append.2 ⟨slotsWF_tail h1, h2⟩)

end

/-! ## Substitution -/

theorem AbilitySet.subset_refl'' (a : AbilitySet) : a.subset a = true := by
  cases a; simp [MoveBytecode.AbilitySet.subset]

theorem AbilitySet.inter_mono {a b c d : AbilitySet} (h1 : a.subset c = true)
    (h2 : b.subset d = true) : (a.inter b).subset (c.inter d) = true := by
  cases a; cases b; cases c; cases d
  simp_all [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter]
  grind

theorem AbilitySet.forContainer_mono'' {a b : AbilitySet} (h : a.subset b = true) :
    a.forContainer.subset b.forContainer = true := by
  cases a; cases b
  simp_all [MoveBytecode.AbilitySet.subset, AbilitySet.forContainer]

theorem AbilitySet.subset_trans'' {a b c : AbilitySet} (h1 : a.subset b = true)
    (h2 : b.subset c = true) : a.subset c = true := by
  cases a; cases b; cases c
  simp_all [MoveBytecode.AbilitySet.subset]
  grind

section
variable {m : CompiledModule} {cs tparams : List AbilitySet} {σ : List SignatureToken}
  (hσ : SignatureToken.ArgumentsWellFormed m tparams σ cs)
include hσ

set_option linter.unusedSectionVars false in
mutual
/-- Instantiation keeps at least the abilities the handles declare. -/
theorem abilitiesIn_instantiate :
    ∀ (t : SignatureToken) {q : SignatureToken}, t.instantiate σ = some q →
      (t.abilitiesIn m cs).subset (q.abilitiesIn m tparams) = true
  | .bool, _, h | .u8, _, h | .u16, _, h | .u32, _, h | .u64, _, h | .u128, _, h
  | .u256, _, h | .i8, _, h | .i16, _, h | .i32, _, h | .i64, _, h | .i128, _, h
  | .i256, _, h | .address, _, h | .signer, _, h | .struct _, _, h => by
    simp only [SignatureToken.instantiate, Option.some.injEq] at h
    subst h
    exact AbilitySet.subset_refl'' _
  | .vector e, _, h => by
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨e', he', rfl⟩ := h
    simp only [SignatureToken.abilitiesIn]
    exact AbilitySet.inter_mono (AbilitySet.subset_refl'' _)
      (AbilitySet.forContainer_mono'' (abilitiesIn_instantiate e he'))
  | .function .., _, h => by
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨_, -, _, -, rfl⟩ := h
    exact AbilitySet.subset_refl'' _
  | .structInstantiation hh args, _, h => by
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨args', ha', rfl⟩ := h
    simp only [SignatureToken.abilitiesIn]
    split
    · exact AbilitySet.inter_mono (AbilitySet.subset_refl'' _)
        (argumentAbilities_instantiate args _ ha')
    · exact AbilitySet.subset_refl'' _
  | .reference _, _, h | .mutableReference _, _, h => by
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨_, -, rfl⟩ := h
    exact AbilitySet.subset_refl'' _
  | .typeParameter i, q, h => by
    simp only [SignatureToken.instantiate] at h
    simp only [SignatureToken.abilitiesIn, List.getD_eq_getElem?_getD]
    have hi : i < cs.length := by
      rw [← argsWF_length hσ]; exact (List.getElem?_eq_some_iff.1 h).1
    obtain ⟨c, hc⟩ : ∃ c, cs[i]? = some c := ⟨_, List.getElem?_eq_getElem hi⟩
    rw [hc, Option.getD_some]
    exact (argsWF_getElem hσ h hc).2

theorem argumentAbilities_instantiate :
    ∀ (args : List SignatureToken) (ps : List StructTypeParameter) {qs : List SignatureToken},
      SignatureToken.instantiateList σ args = some qs →
      (SignatureToken.argumentAbilities m cs args ps).subset
        (SignatureToken.argumentAbilities m tparams qs ps) = true
  | [], _, _, h => by
    simp only [SignatureToken.instantiateList, Option.some.injEq] at h
    subst h
    exact AbilitySet.subset_refl'' _
  | a :: args, [], _, h => by
    simp only [SignatureToken.instantiateList, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨_, -, _, -, rfl⟩ := h
    exact AbilitySet.subset_refl'' _
  | a :: args, p :: ps, _, h => by
    simp only [SignatureToken.instantiateList, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨a', ha', args', hargs', rfl⟩ := h
    simp only [SignatureToken.argumentAbilities]
    refine AbilitySet.inter_mono ?_ (argumentAbilities_instantiate args ps hargs')
    split
    · exact AbilitySet.subset_refl'' _
    · exact AbilitySet.forContainer_mono'' (abilitiesIn_instantiate a ha')
end

set_option linter.unusedSectionVars false in
mutual
/-- **Substitution preserves well-formed value types.** -/
theorem vtwf_instantiate :
    ∀ (t : SignatureToken) {q : SignatureToken}, t.ValueTypeWellFormed m cs →
      t.instantiate σ = some q → q.ValueTypeWellFormed m tparams
  | .bool, _, _, h | .u8, _, _, h | .u16, _, _, h | .u32, _, _, h | .u64, _, _, h
  | .u128, _, _, h | .u256, _, _, h | .i8, _, _, h | .i16, _, _, h | .i32, _, _, h
  | .i64, _, _, h | .i128, _, _, h | .i256, _, _, h | .address, _, _, h | .signer, _, _, h => by
    simp only [SignatureToken.instantiate, Option.some.injEq] at h
    subst h
    trivial
  | .struct _, _, hw, h => by
    simp only [SignatureToken.instantiate, Option.some.injEq] at h
    subst h
    exact hw
  | .vector e, _, hw, h => by
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨e', he', rfl⟩ := h
    have := vtwf_instantiate e hw he'
    exact this
  | .function ps rs a, _, hw, h => by
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨ps', hps, rs', hrs, rfl⟩ := h
    obtain ⟨h1, h2, h3⟩ := hw
    exact ⟨slotsWF_instantiate ps h1 hps, slotsWF_instantiate rs h2 hrs, h3⟩
  | .structInstantiation hh args, _, hw, h => by
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨args', ha', rfl⟩ := h
    obtain ⟨sh, hsh, hargs⟩ := hw
    exact ⟨sh, hsh, argsWF_instantiate args _ hargs ha'⟩
  | .reference _, _, hw, _ | .mutableReference _, _, hw, _ => hw.elim
  | .typeParameter i, q, hw, h => by
    simp only [SignatureToken.instantiate] at h
    simp only [SignatureToken.ValueTypeWellFormed] at hw
    obtain ⟨c, hc⟩ : ∃ c, cs[i]? = some c := ⟨_, List.getElem?_eq_getElem hw⟩
    exact (argsWF_getElem hσ h hc).1

theorem slotsWF_instantiate :
    ∀ (ts : List SignatureToken) {qs : List SignatureToken},
      SignatureToken.SlotsWellFormed m cs ts → SignatureToken.instantiateList σ ts = some qs →
      SignatureToken.SlotsWellFormed m tparams qs
  | [], _, _, h => by
    simp only [SignatureToken.instantiateList, Option.some.injEq] at h
    subst h
    trivial
  | t :: ts, _, hw, h => by
    simp only [SignatureToken.instantiateList, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨q, hq, qs, hqs, rfl⟩ := h
    have hrest := slotsWF_instantiate ts (slotsWF_tail hw) hqs
    refine slotsWF_cons ?_ hrest
    have hhead := slotsWF_head hw
    cases t with
    | reference t0 =>
      simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
        Option.some.injEq] at hq
      obtain ⟨q0, hq0, rfl⟩ := hq
      simp only [SignatureToken.SlotsWellFormed] at hhead ⊢
      exact ⟨vtwf_instantiate t0 hhead.1 hq0, trivial⟩
    | mutableReference t0 =>
      simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
        Option.some.injEq] at hq
      obtain ⟨q0, hq0, rfl⟩ := hq
      simp only [SignatureToken.SlotsWellFormed] at hhead ⊢
      exact ⟨vtwf_instantiate t0 hhead.1 hq0, trivial⟩
    | _ =>
      exact slotsWF_of_vtwf (vtwf_instantiate _
        (by simpa [SignatureToken.SlotsWellFormed] using hhead) hq)

theorem argsWF_instantiate :
    ∀ (args : List SignatureToken) (ds : List AbilitySet) {qs : List SignatureToken},
      SignatureToken.ArgumentsWellFormed m cs args ds →
      SignatureToken.instantiateList σ args = some qs →
      SignatureToken.ArgumentsWellFormed m tparams qs ds
  | [], [], _, _, h => by
    simp only [SignatureToken.instantiateList, Option.some.injEq] at h
    subst h
    trivial
  | a :: args, d :: ds, _, hw, h => by
    simp only [SignatureToken.instantiateList, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨q, hq, qs, hqs, rfl⟩ := h
    obtain ⟨h1, h2, h3⟩ := hw
    exact ⟨vtwf_instantiate a h1 hq,
      AbilitySet.subset_trans'' h2 (abilitiesIn_instantiate hσ a hq),
      argsWF_instantiate args ds h3 hqs⟩
  | [], _ :: _, _, hw, _ | _ :: _, [], _, hw, _ => hw.elim
end

end

/-- A closed value type is well formed in any context. -/
theorem vtwf_closed {m : CompiledModule} {tparams : List AbilitySet} {t : SignatureToken}
    (h : t.ValueTypeWellFormed m []) : t.ValueTypeWellFormed m tparams :=
  vtwf_instantiate (σ := []) (cs := []) (by trivial) t h (instantiate_closed t h)

/-- Closed slots are well formed in any context. -/
theorem slotsWF_closed {m : CompiledModule} {tparams : List AbilitySet} {ts : List SignatureToken}
    (h : SignatureToken.SlotsWellFormed m [] ts) : SignatureToken.SlotsWellFormed m tparams ts :=
  slotsWF_instantiate (σ := []) (cs := []) (by trivial) ts h (instantiateSlots_closed ts h)

end MoveBytecode.Proofs
