-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ByValue

/-!
# Instances: substitution and well-formed translation

UNTRUSTED (checked by the kernel). Two facts about translating signature
tokens under type arguments, which carry the static typing of generic code
to its instances:

- **Substitution** (`ofTokenWith_instantiate`): translating an instantiated
  token under the caller's type arguments is translating the original token
  under the translated instantiation. A generic call's or struct operation's
  types, as the analysis substitutes them, are the callee's or the struct's
  own types at the instance the semantics builds.
- **Well-formed translation** (`translate_wf`): a token that L5 calls well
  formed, translated under type arguments that satisfy the enclosing
  function's constraints, is a type of the program (`RuntimeType.wellFormed`),
  with at least the abilities the token's handles declare
  (`SignatureToken.abilitiesIn`). It needs the struct handles to be
  conservative views of their definitions (K1, `HandlesResolve`). This
  discharges the constraint checks the semantics makes at generic calls and
  struct operations.
-/

namespace MoveBytecode.Proofs

/-! ## Ability sets -/

theorem AbilitySet.subset_refl' (a : AbilitySet) : a.subset a = true := by
  cases a; simp [MoveBytecode.AbilitySet.subset]

theorem AbilitySet.inter_subset_inter {a b c d : AbilitySet} (h1 : a.subset c = true)
    (h2 : b.subset d = true) : (a.inter b).subset (c.inter d) = true := by
  cases a; cases b; cases c; cases d
  simp_all [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter]
  grind

theorem AbilitySet.inter_comm' (a b : AbilitySet) : a.inter b = b.inter a := by
  cases a; cases b; simp [MoveBytecode.AbilitySet.inter, Bool.and_comm]

theorem AbilitySet.subset_all' (a : AbilitySet) : a.subset AbilitySet.all = true := by
  cases a; simp [MoveBytecode.AbilitySet.subset, AbilitySet.all]

theorem AbilitySet.forContainer_mono' {a b : AbilitySet} (h : a.subset b = true) :
    a.forContainer.subset b.forContainer = true := by
  cases a; cases b
  simp_all [MoveBytecode.AbilitySet.subset, AbilitySet.forContainer]

theorem AbilitySet.subset_trans' {a b c : AbilitySet} (h1 : a.subset b = true)
    (h2 : b.subset c = true) : a.subset c = true := by
  cases a; cases b; cases c
  simp_all [MoveBytecode.AbilitySet.subset]
  grind

/-! ## Substitution -/

theorem ofTokenListWith_getElem? {m : CompiledModule} {τ : List RuntimeType} :
    ∀ {ts : List SignatureToken} {us : List RuntimeType},
      RuntimeType.ofTokenListWith m τ ts = some us → ∀ {i : Nat} {t : SignatureToken},
        ts[i]? = some t → us[i]? = RuntimeType.ofTokenWith m τ t
  | [], _, _, _, _, ht => by simp at ht
  | t' :: ts, us, h, i, t, ht => by
    simp only [RuntimeType.ofTokenListWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨u, hu, us', hus', rfl⟩ := h
    cases i with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at ht
      subst ht
      simp [hu]
    | succ i =>
      simp only [List.getElem?_cons_succ] at ht ⊢
      exact ofTokenListWith_getElem? hus' ht

/-- A slot list whose head is no reference translates its head as a value
type. -/
theorem ofSlotTokensWith_cons_val {m : CompiledModule} {τ : List RuntimeType}
    {q : SignatureToken} {qs : List SignatureToken}
    (h1 : ∀ x, q ≠ .reference x) (h2 : ∀ x, q ≠ .mutableReference x) :
    RuntimeType.ofSlotTokensWith m τ (q :: qs) =
      (do pure ((← RuntimeType.ofTokenWith m τ q) :: (← RuntimeType.ofSlotTokensWith m τ qs))) := by
  cases q <;> simp_all [RuntimeType.ofSlotTokensWith]

/-- A token that translates is no reference. -/
theorem ofTokenWith_some_not_ref {m : CompiledModule} {τ : List RuntimeType} {q : SignatureToken}
    {u : RuntimeType} (h : RuntimeType.ofTokenWith m τ q = some u) :
    (∀ x, q ≠ .reference x) ∧ (∀ x, q ≠ .mutableReference x) := by
  constructor <;> intro x hx <;> subst hx <;> simp [RuntimeType.ofTokenWith] at h

mutual
/-- Translating an instantiated token under `τ` is translating the token under
the instantiation translated under `τ`. -/
theorem ofTokenWith_instantiate {m : CompiledModule} {τ σ' : List RuntimeType}
    {σ : List SignatureToken} (hσ : RuntimeType.ofTokenListWith m τ σ = some σ') :
    ∀ {t q : SignatureToken}, t.instantiate σ = some q →
      RuntimeType.ofTokenWith m τ q = RuntimeType.ofTokenWith m σ' t
  | .bool, _, h | .u8, _, h | .u16, _, h | .u32, _, h | .u64, _, h | .u128, _, h
  | .u256, _, h | .i8, _, h | .i16, _, h | .i32, _, h | .i64, _, h | .i128, _, h
  | .i256, _, h | .address, _, h | .signer, _, h | .struct _, _, h => by
    simp only [SignatureToken.instantiate, Option.some.injEq] at h
    subst h
    rfl
  | .vector e, _, h => by
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨e', he', rfl⟩ := h
    simp only [RuntimeType.ofTokenWith, ofTokenWith_instantiate hσ he']
  | .structInstantiation hh args, _, h => by
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨args', ha', rfl⟩ := h
    simp only [RuntimeType.ofTokenWith, ofTokenListWith_instantiate hσ ha']
  | .typeParameter _, _, h => by
    simp only [SignatureToken.instantiate] at h
    simp only [RuntimeType.ofTokenWith]
    exact (ofTokenListWith_getElem? hσ h).symm
  | .function ps rs a, _, h => by
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨ps', hps, rs', hrs, rfl⟩ := h
    simp only [RuntimeType.ofTokenWith, ofSlotTokensWith_instantiate hσ hps,
      ofSlotTokensWith_instantiate hσ hrs]
  | .reference _, _, h | .mutableReference _, _, h => by
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨_, -, rfl⟩ := h
    rfl

/-- The same for slot lists. -/
theorem ofSlotTokensWith_instantiate {m : CompiledModule} {τ σ' : List RuntimeType}
    {σ : List SignatureToken} (hσ : RuntimeType.ofTokenListWith m τ σ = some σ') :
    ∀ {ts qs : List SignatureToken}, SignatureToken.instantiateList σ ts = some qs →
      RuntimeType.ofSlotTokensWith m τ qs = RuntimeType.ofSlotTokensWith m σ' ts
  | [], _, h => by
    simp only [SignatureToken.instantiateList, Option.some.injEq] at h
    subst h
    rfl
  | .reference t0 :: ts, _, h => by
    simp only [SignatureToken.instantiateList, SignatureToken.instantiate, bind,
      Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨q, ⟨q0, hq0, rfl⟩, qs, hqs, rfl⟩ := h
    simp only [RuntimeType.ofSlotTokensWith, ofTokenWith_instantiate hσ hq0,
      ofSlotTokensWith_instantiate hσ hqs]
  | .mutableReference t0 :: ts, _, h => by
    simp only [SignatureToken.instantiateList, SignatureToken.instantiate, bind,
      Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨q, ⟨q0, hq0, rfl⟩, qs, hqs, rfl⟩ := h
    simp only [RuntimeType.ofSlotTokensWith, ofTokenWith_instantiate hσ hq0,
      ofSlotTokensWith_instantiate hσ hqs]
  | .typeParameter i :: ts, _, h => by
    simp only [SignatureToken.instantiateList, SignatureToken.instantiate, bind,
      Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨q, hq, qs, hqs, rfl⟩ := h
    have hu := ofTokenListWith_getElem? hσ hq
    have hi : i < σ'.length := by
      rw [ofTokenListWith_length hσ]
      exact (List.getElem?_eq_some_iff.1 hq).1
    obtain ⟨u, hu'⟩ : ∃ u, σ'[i]? = some u := ⟨_, List.getElem?_eq_getElem hi⟩
    rw [hu'] at hu
    obtain ⟨hn1, hn2⟩ := ofTokenWith_some_not_ref hu.symm
    rw [ofSlotTokensWith_cons_val hn1 hn2, ← hu]
    simp only [RuntimeType.ofSlotTokensWith, RuntimeType.ofTokenWith, hu',
      ofSlotTokensWith_instantiate hσ hqs]
  | .bool :: ts, _, h | .u8 :: ts, _, h | .u16 :: ts, _, h | .u32 :: ts, _, h
  | .u64 :: ts, _, h | .u128 :: ts, _, h | .u256 :: ts, _, h | .i8 :: ts, _, h
  | .i16 :: ts, _, h | .i32 :: ts, _, h | .i64 :: ts, _, h | .i128 :: ts, _, h
  | .i256 :: ts, _, h | .address :: ts, _, h | .signer :: ts, _, h | .struct _ :: ts, _, h
  | .vector _ :: ts, _, h | .structInstantiation _ _ :: ts, _, h
  | .function _ _ _ :: ts, _, h => by
    simp only [SignatureToken.instantiateList, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨q, hq, qs, hqs, rfl⟩ := h
    have hti := hq
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at hti
    have hn : (∀ x, q ≠ .reference x) ∧ (∀ x, q ≠ .mutableReference x) := by
      constructor <;> intro x hx <;> subst hx <;> simp_all
    rw [ofSlotTokensWith_cons_val hn.1 hn.2]
    simp only [RuntimeType.ofSlotTokensWith, ofTokenWith_instantiate hσ hq,
      ofSlotTokensWith_instantiate hσ hqs]

theorem ofTokenListWith_instantiate {m : CompiledModule} {τ σ' : List RuntimeType}
    {σ : List SignatureToken} (hσ : RuntimeType.ofTokenListWith m τ σ = some σ') :
    ∀ {ts qs : List SignatureToken}, SignatureToken.instantiateList σ ts = some qs →
      RuntimeType.ofTokenListWith m τ qs = RuntimeType.ofTokenListWith m σ' ts
  | [], _, h => by
    simp only [SignatureToken.instantiateList, Option.some.injEq] at h
    subst h
    rfl
  | t :: ts, _, h => by
    simp only [SignatureToken.instantiateList, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨q, hq, qs, hqs, rfl⟩ := h
    simp only [RuntimeType.ofTokenListWith, ofTokenWith_instantiate hσ hq,
      ofTokenListWith_instantiate hσ hqs]
end

/-- A token that translates under no reference: not a reference. -/
theorem ofTokenWith_not_ref {m : CompiledModule} {τ : List RuntimeType} {t : SignatureToken}
    {u : RuntimeType} (h : RuntimeType.ofTokenWith m τ t = some u) :
    RuntimeType.ofSlotTokenWith m τ t = some u := by
  cases t <;> first
    | (simp [RuntimeType.ofTokenWith] at h; done)
    | simpa [RuntimeType.ofSlotTokenWith] using h

/-- The same for slot tokens: an instantiation's tokens translate as value
types, so a type parameter never stands for a reference. -/
theorem ofSlotTokenWith_instantiate {m : CompiledModule} {τ σ' : List RuntimeType}
    {σ : List SignatureToken} (hσ : RuntimeType.ofTokenListWith m τ σ = some σ')
    {t q : SignatureToken} (h : t.instantiate σ = some q) :
    RuntimeType.ofSlotTokenWith m τ q = RuntimeType.ofSlotTokenWith m σ' t := by
  cases t with
  | reference t0 =>
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨q0, hq0, rfl⟩ := h
    simp only [RuntimeType.ofSlotTokenWith, ofTokenWith_instantiate hσ hq0]
  | mutableReference t0 =>
    simp only [SignatureToken.instantiate, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨q0, hq0, rfl⟩ := h
    simp only [RuntimeType.ofSlotTokenWith, ofTokenWith_instantiate hσ hq0]
  | typeParameter i =>
    simp only [SignatureToken.instantiate] at h
    have hu := ofTokenListWith_getElem? hσ h
    have hi : i < σ'.length := by
      rw [ofTokenListWith_length hσ]
      exact (List.getElem?_eq_some_iff.1 h).1
    obtain ⟨u, hu'⟩ : ∃ u, σ'[i]? = some u := ⟨_, List.getElem?_eq_getElem hi⟩
    rw [hu'] at hu
    have hq : RuntimeType.ofTokenWith m τ q = some u := hu.symm
    rw [ofTokenWith_not_ref hq]
    simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, hu']
  | _ =>
    have := ofTokenWith_instantiate hσ h
    cases q <;> simp_all [SignatureToken.instantiate, RuntimeType.ofSlotTokenWith, bind,
      Option.bind_eq_some_iff]

/-- The same for signatures. -/
theorem ofSignatureWith_instantiate {m : CompiledModule} {τ σ' : List RuntimeType}
    {σ : List SignatureToken} (hσ : RuntimeType.ofTokenListWith m τ σ = some σ') :
    ∀ {ts qs : List SignatureToken}, SignatureToken.instantiateList σ ts = some qs →
      RuntimeType.ofSignatureWith m τ qs = RuntimeType.ofSignatureWith m σ' ts
  | [], _, h => by
    simp only [SignatureToken.instantiateList, Option.some.injEq] at h
    subst h
    rfl
  | t :: ts, _, h => by
    simp only [SignatureToken.instantiateList, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨q, hq, qs, hqs, rfl⟩ := h
    have ih := ofSignatureWith_instantiate hσ hqs
    simp only [RuntimeType.ofSignatureWith, List.mapM_cons] at ih ⊢
    rw [ofSlotTokenWith_instantiate hσ hq, ih]

/-! ## Well-formed translation -/

/-- The struct handles of module `m` resolve to definitions of which they are
conservative views (K1, `Program.StructHandleResolution`, for one module). -/
def HandlesResolve (P : Program) (m : CompiledModule) : Prop :=
  ∀ (h : StructHandleIndex) (sh : StructHandle), m.structHandles[h]? = some sh →
    ∃ s r info, m.structIdAt h = some s ∧ P.resolveStruct s = some r ∧ P.struct? r = some info ∧
      sh.ConservativeFor info.handle

theorem resolveStruct_typeArgs {P : Program} (s : StructId) (tys : List RuntimeType) :
    P.resolveStruct { s with typeArgs := tys } = P.resolveStruct s := rfl

/-- An instance of a resolved definition with one value type argument per
type parameter is a type of the program. -/
theorem structType?_instance {P : Program} {s : StructId} {r : StructRef} {info : StructInfo}
    (hr : P.resolveStruct s = some r) (hi : P.struct? r = some info) {tys : List RuntimeType}
    (hlen : tys.length = info.handle.typeParameters.length)
    (hv : ∀ x ∈ tys, x.isValue = true) :
    P.structType? { s with typeArgs := tys } = some info := by
  simp only [Program.structType?, resolveStruct_typeArgs, hr, hi, bind, Option.bind_some]
  rw [if_pos ⟨hlen, List.all_eq_true.2 hv⟩]

theorem argumentsWellFormed_getElem {P : Program} :
    ∀ {τ : List RuntimeType} {cs : List AbilitySet},
      RuntimeType.argumentsWellFormed P τ cs = true → ∀ {i : Nat} {u : RuntimeType} {c : AbilitySet},
        τ[i]? = some u → cs[i]? = some c →
        u.isValue = true ∧ u.wellFormed P = true ∧ c.subset (u.abilities P) = true
  | [], _, _, _, _, _, hu, _ => by simp at hu
  | _ :: _, [], h, _, _, _, _, _ => by simp [RuntimeType.argumentsWellFormed] at h
  | a :: as, c :: cs, h, i, u, c', hu, hc => by
    simp only [RuntimeType.argumentsWellFormed, Bool.and_eq_true] at h
    cases i with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hu hc
      subst hu hc
      exact ⟨h.1.1.1, h.1.1.2, h.1.2⟩
    | succ i =>
      simp only [List.getElem?_cons_succ] at hu hc
      exact argumentsWellFormed_getElem h.2 hu hc

theorem argumentsWellFormed_length {P : Program} :
    ∀ {τ : List RuntimeType} {cs : List AbilitySet},
      RuntimeType.argumentsWellFormed P τ cs = true → τ.length = cs.length
  | [], [], _ => rfl
  | [], _ :: _, h | _ :: _, [], h => by simp [RuntimeType.argumentsWellFormed] at h
  | _ :: _, _ :: _, h => by
    simp only [RuntimeType.argumentsWellFormed, Bool.and_eq_true] at h
    simp [argumentsWellFormed_length h.2]

section
variable {P : Program} {m : CompiledModule} (hk : HandlesResolve P m) {tparams : List AbilitySet}
  {τ : List RuntimeType} (hτ : RuntimeType.argumentsWellFormed P τ tparams = true)

include hk hτ

set_option linter.unusedSectionVars false in
mutual
/-- **Well-formed translation.** A token L5 calls well formed, translated under
type arguments that satisfy the constraints in scope, is a type of the program
with at least the abilities the token's handles declare. -/
theorem translate_wf :
    ∀ (t : SignatureToken) {u : RuntimeType}, t.ValueTypeWellFormed m tparams →
      RuntimeType.ofTokenWith m τ t = some u →
      u.wellFormed P = true ∧ (t.abilitiesIn m tparams).subset (u.abilities P) = true
  | .bool, _, _, h | .u8, _, _, h | .u16, _, _, h | .u32, _, _, h | .u64, _, _, h
  | .u128, _, _, h | .u256, _, _, h | .i8, _, _, h | .i16, _, _, h | .i32, _, _, h
  | .i64, _, _, h | .i128, _, _, h | .i256, _, _, h | .address, _, _, h | .signer, _, _, h => by
    simp only [RuntimeType.ofTokenWith, Option.some.injEq] at h
    subst h
    refine ⟨rfl, ?_⟩
    simp [SignatureToken.abilitiesIn, RuntimeType.abilities, AbilitySet.subset_refl']
  | .vector e, u, hw, h => by
    simp only [RuntimeType.ofTokenWith, Option.map_eq_some_iff] at h
    obtain ⟨e', he', rfl⟩ := h
    simp only [SignatureToken.ValueTypeWellFormed] at hw
    obtain ⟨ihw, iha⟩ := translate_wf e hw he'
    have hv := ofTokenWith_isValue (argumentsWellFormed_values hτ) he'
    refine ⟨by simp [RuntimeType.wellFormed, hv, ihw], ?_⟩
    simp only [SignatureToken.abilitiesIn, RuntimeType.abilities]
    exact AbilitySet.inter_subset_inter (AbilitySet.subset_refl' _) (AbilitySet.forContainer_mono' iha)
  | .struct h, u, hw, hu => by
    obtain ⟨sh, hsh, hnp⟩ := hw
    simp only [RuntimeType.ofTokenWith, hsh, bind, Option.bind_some, hnp, List.isEmpty_nil,
      ↓reduceIte, pure, Option.bind_eq_some_iff, Option.some.injEq] at hu
    obtain ⟨sid, hsid, rfl⟩ := hu
    obtain ⟨s, r, info, hs, hr, hi, hcon⟩ := hk h sh hsh
    rw [hsid, Option.some.injEq] at hs
    subst hs
    have hargs : sid.typeArgs = [] := by rw [← structIdAt_typeArgs hsid]
    have hlen : info.handle.typeParameters = [] := by
      have := hcon.2.1
      rw [hnp] at this
      exact (List.eq_nil_of_length_eq_zero this.symm)
    have hst : P.structType? sid = some info := by
      have := structType?_instance (tys := []) hr hi (by simp [hlen]) (by simp)
      rwa [show ({ sid with typeArgs := [] } : StructId) = sid by
        cases sid; simp_all] at this
    refine ⟨by simp [RuntimeType.wellFormed, hst, hargs, hlen, RuntimeType.argumentsWellFormed], ?_⟩
    simp only [SignatureToken.abilitiesIn, hsh, RuntimeType.abilities, hst, hargs, hlen,
      RuntimeType.argumentAbilities, AbilitySet.all_inter]
    exact hcon.1
  | .structInstantiation h args, u, hw, hu => by
    obtain ⟨sh, hsh, hargs⟩ := hw
    simp only [RuntimeType.ofTokenWith, hsh, bind, Option.bind_some,
      Option.bind_eq_some_iff] at hu
    obtain ⟨sid, hsid, targs, htargs, hu⟩ := hu
    split at hu
    · next hlen =>
      simp only [pure, Option.some.injEq] at hu
      subst hu
      obtain ⟨s, r, info, hs, hr, hi, hcon⟩ := hk h sh hsh
      rw [hsid, Option.some.injEq] at hs
      subst hs
      have hv : ∀ x ∈ targs, x.isValue = true :=
        ofTokenListWith_values (argumentsWellFormed_values hτ) htargs
      have hst := structType?_instance hr hi (hlen.trans hcon.2.1) hv
      obtain ⟨haw, hab⟩ := translateArgs_wf args sh.typeParameters info.handle.typeParameters
        hargs hcon.2.1 hcon.2.2 htargs
      refine ⟨by simp only [RuntimeType.wellFormed, hst]; exact haw, ?_⟩
      simp only [SignatureToken.abilitiesIn, hsh, RuntimeType.abilities, hst]
      rw [AbilitySet.inter_comm' (RuntimeType.argumentAbilities P targs _)]
      exact AbilitySet.inter_subset_inter hcon.1 hab
    · simp at hu
  | .typeParameter i, u, hw, hu => by
    simp only [RuntimeType.ofTokenWith] at hu
    simp only [SignatureToken.ValueTypeWellFormed] at hw
    obtain ⟨c, hc⟩ : ∃ c, tparams[i]? = some c := ⟨_, List.getElem?_eq_getElem hw⟩
    obtain ⟨-, hwf, hsub⟩ := argumentsWellFormed_getElem hτ hu hc
    refine ⟨hwf, ?_⟩
    simp only [SignatureToken.abilitiesIn, List.getD_eq_getElem?_getD, hc, Option.getD_some]
    exact hsub
  | .function ps rs a, u, hw, hu => by
    simp only [RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at hu
    obtain ⟨ps', hps, rs', hrs, rfl⟩ := hu
    obtain ⟨hpw, hrw, hk⟩ := hw
    refine ⟨?_, ?_⟩
    · simp [RuntimeType.wellFormed, hk, translateSlots_wf ps hpw hps,
        translateSlots_wf rs hrw hrs]
    · simp only [SignatureToken.abilitiesIn, RuntimeType.abilities]
      exact AbilitySet.subset_refl' _
  | .reference _, _, _, hu | .mutableReference _, _, _, hu => by
    simp [RuntimeType.ofTokenWith] at hu

/-- A slot list L5 calls well formed translates to slot types of the
program. -/
theorem translateSlots_wf :
    ∀ (ts : List SignatureToken) {us : List RuntimeType},
      SignatureToken.SlotsWellFormed m tparams ts →
      RuntimeType.ofSlotTokensWith m τ ts = some us → RuntimeType.slotsWellFormed P us = true
  | [], us, _, h => by
    simp only [RuntimeType.ofSlotTokensWith, Option.some.injEq] at h
    subst h
    rfl
  | .reference t :: ts, us, hw, h => by
    simp only [RuntimeType.ofSlotTokensWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨u, hu, us', hus', rfl⟩ := h
    obtain ⟨htw, hrest⟩ := hw
    have hv := ofTokenWith_isValue (argumentsWellFormed_values hτ) hu
    simp [RuntimeType.slotsWellFormed, hv, (translate_wf t htw hu).1,
      translateSlots_wf ts hrest hus']
  | .mutableReference t :: ts, us, hw, h => by
    simp only [RuntimeType.ofSlotTokensWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨u, hu, us', hus', rfl⟩ := h
    obtain ⟨htw, hrest⟩ := hw
    have hv := ofTokenWith_isValue (argumentsWellFormed_values hτ) hu
    simp [RuntimeType.slotsWellFormed, hv, (translate_wf t htw hu).1,
      translateSlots_wf ts hrest hus']
  | .bool :: ts, us, hw, h | .u8 :: ts, us, hw, h | .u16 :: ts, us, hw, h
  | .u32 :: ts, us, hw, h | .u64 :: ts, us, hw, h | .u128 :: ts, us, hw, h
  | .u256 :: ts, us, hw, h | .i8 :: ts, us, hw, h | .i16 :: ts, us, hw, h
  | .i32 :: ts, us, hw, h | .i64 :: ts, us, hw, h | .i128 :: ts, us, hw, h
  | .i256 :: ts, us, hw, h | .address :: ts, us, hw, h | .signer :: ts, us, hw, h
  | .struct _ :: ts, us, hw, h | .vector _ :: ts, us, hw, h
  | .structInstantiation _ _ :: ts, us, hw, h | .typeParameter _ :: ts, us, hw, h
  | .function _ _ _ :: ts, us, hw, h => by
    simp only [RuntimeType.ofSlotTokensWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨u, hu, us', hus', rfl⟩ := h
    obtain ⟨htw, hrest⟩ := hw
    have hv := ofTokenWith_isValue (argumentsWellFormed_values hτ) hu
    have hwf := (translate_wf _ htw hu).1
    have hrest' := translateSlots_wf ts hrest hus'
    cases u <;> simp_all [RuntimeType.slotsWellFormed, RuntimeType.isValue]

/-- The arguments of a struct instantiation: translated, they instantiate the
definition's type parameters, with at least the abilities the handle grants. -/
theorem translateArgs_wf :
    ∀ (args : List SignatureToken) (ps qs : List StructTypeParameter) {us : List RuntimeType},
      SignatureToken.ArgumentsWellFormed m tparams args (ps.map (·.constraints)) →
      ps.length = qs.length →
      (∀ (i : Nat) (p q : StructTypeParameter), ps[i]? = some p → qs[i]? = some q →
        p.ConservativeFor q) →
      RuntimeType.ofTokenListWith m τ args = some us →
      RuntimeType.argumentsWellFormed P us (qs.map (·.constraints)) = true ∧
      (SignatureToken.argumentAbilities m tparams args ps).subset
        (RuntimeType.argumentAbilities P us qs) = true
  | [], [], [], us, _, _, _, h => by
    simp only [RuntimeType.ofTokenListWith, Option.some.injEq] at h
    subst h
    simp [RuntimeType.argumentsWellFormed, SignatureToken.argumentAbilities,
      RuntimeType.argumentAbilities, AbilitySet.subset_refl']
  | a :: as, p :: ps, q :: qs, us, hw, hlen, hcon, h => by
    simp only [RuntimeType.ofTokenListWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨u, hu, us', hus', rfl⟩ := h
    simp only [List.map_cons, SignatureToken.ArgumentsWellFormed] at hw
    obtain ⟨haw, hpc, hrest⟩ := hw
    obtain ⟨ihw, iha⟩ := translate_wf a haw hu
    have hpq := hcon 0 p q rfl rfl
    obtain ⟨irw, ira⟩ := translateArgs_wf as ps qs hrest (by simpa using hlen)
      (fun i p' q' h1 h2 => hcon (i + 1) p' q' h1 h2) hus'
    have huv := ofTokenWith_isValue (argumentsWellFormed_values hτ) hu
    refine ⟨?_, ?_⟩
    · simp only [List.map_cons, RuntimeType.argumentsWellFormed, Bool.and_eq_true]
      exact ⟨⟨⟨huv, ihw⟩, AbilitySet.subset_trans' hpq.2 (AbilitySet.subset_trans' hpc iha)⟩, irw⟩
    · simp only [SignatureToken.argumentAbilities, RuntimeType.argumentAbilities]
      refine AbilitySet.inter_subset_inter ?_ ira
      by_cases hq : q.isPhantom = true
      · simp only [hq, ↓reduceIte]
        exact AbilitySet.subset_all' _
      · have hp : p.isPhantom = false := by
          cases hpp : p.isPhantom
          · rfl
          · exact absurd (hpq.1 hpp) hq
        simp only [hp, Bool.false_eq_true, ↓reduceIte, hq]
        exact AbilitySet.forContainer_mono' iha
  | [], _ :: _, _, _, hw, _, _, _ | _ :: _, [], _, _, hw, _, _, _ => by
    simp [SignatureToken.ArgumentsWellFormed] at hw
  | [], [], _ :: _, _, _, hlen, _, _ => by simp at hlen
  | _ :: _, _ :: _, [], _, _, hlen, _, _ => by simp at hlen
end

/-- The type arguments of an instantiation L5 calls well formed for
constraints `cs`, translated under constraint-satisfying type arguments,
satisfy `cs`. -/
theorem translateList_ok :
    ∀ {args : List SignatureToken} {cs : List AbilitySet} {us : List RuntimeType},
      SignatureToken.ArgumentsWellFormed m tparams args cs →
      RuntimeType.ofTokenListWith m τ args = some us →
      RuntimeType.argumentsWellFormed P us cs = true
  | [], [], _, _, h => by
    simp only [RuntimeType.ofTokenListWith, Option.some.injEq] at h
    subst h
    rfl
  | a :: as, c :: cs, us, hw, h => by
    simp only [RuntimeType.ofTokenListWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨u, hu, us', hus', rfl⟩ := h
    obtain ⟨haw, hc, hrest⟩ := hw
    obtain ⟨ihw, iha⟩ := translate_wf hk hτ a haw hu
    have huv := ofTokenWith_isValue (argumentsWellFormed_values hτ) hu
    simp only [RuntimeType.argumentsWellFormed, Bool.and_eq_true]
    exact ⟨⟨⟨huv, ihw⟩, AbilitySet.subset_trans' hc iha⟩, translateList_ok hrest hus'⟩
  | [], _ :: _, _, hw, _ | _ :: _, [], _, hw, _ => by
    simp [SignatureToken.ArgumentsWellFormed] at hw

end

/-- Type arguments that satisfy constraints `cs` satisfy constraints `ds`
that are each included in `cs`'s. -/
theorem argumentsWellFormed_weaken {P : Program} :
    ∀ {us : List RuntimeType} {cs ds : List AbilitySet},
      RuntimeType.argumentsWellFormed P us cs = true → ds.length = cs.length →
      (∀ (i : Nat) (c d : AbilitySet), cs[i]? = some c → ds[i]? = some d → d.subset c = true) →
      RuntimeType.argumentsWellFormed P us ds = true
  | [], [], [], _, _, _ => rfl
  | u :: us, c :: cs, d :: ds, h, hlen, hsub => by
    simp only [RuntimeType.argumentsWellFormed, Bool.and_eq_true] at h ⊢
    exact ⟨⟨h.1.1, AbilitySet.subset_trans' (hsub 0 c d rfl rfl) h.1.2⟩,
      argumentsWellFormed_weaken h.2 (by simpa using hlen)
        (fun i c' d' h1 h2 => hsub (i + 1) c' d' h1 h2)⟩
  | [], _ :: _, _, h, _, _ | _ :: _, [], _, h, _, _ => by
    simp [RuntimeType.argumentsWellFormed] at h
  | _, [], _ :: _, _, hlen, _ | _, _ :: _, [], _, hlen, _ => by simp at hlen

end MoveBytecode.Proofs
