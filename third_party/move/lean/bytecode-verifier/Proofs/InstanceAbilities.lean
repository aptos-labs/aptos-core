-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Instances
import Proofs.StackUsage

/-!
# The model's abilities at instances

UNTRUSTED (checked by the kernel). The abilities the model computes for a
token (`Model.abilities`, over the constraints of the type parameters in
scope) are at most those of the token's runtime type at any instance whose
type arguments satisfy the constraints, given conservative struct handles
(K1). Unlike `translate_wf`, this needs no well-formedness of the token: the
analysis computes abilities of tokens it has instantiated itself.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

theorem requiredBy_eq (a : AbilitySet) : requiredBy a = a.forContainer := by cases a; rfl

theorem AbilitySet.inter_subset_left'' (a b : AbilitySet) : (a.inter b).subset a = true := by
  cases a; cases b
  simp [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter]
  grind

theorem AbilitySet.inter_subset_right'' (a b : AbilitySet) : (a.inter b).subset b = true := by
  cases a; cases b
  simp [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter]
  grind

theorem AbilitySet.subset_inter_iff {a b c : AbilitySet} :
    a.subset (b.inter c) = true ↔ a.subset b = true ∧ a.subset c = true := by
  cases a; cases b; cases c
  simp [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter]
  grind

/-- The fold of `polymorphic_abilities` is within the declared abilities and
what the runtime type arguments permit, when every parameter the definition
does not treat as phantom is not phantom in the handle either, and the model's
ability of its argument is within the runtime type's. -/
theorem polyFold_sub {P : Program} :
    ∀ (aa : List AbilitySet) (ph : List Bool) (us : List RuntimeType)
      (qs : List StructTypeParameter) (acc : AbilitySet),
      aa.length = ph.length → ph.length = us.length → us.length = qs.length →
      (∀ (i : Nat) (a : AbilitySet) (p : Bool) (u : RuntimeType) (q : StructTypeParameter),
        aa[i]? = some a → ph[i]? = some p → us[i]? = some u → qs[i]? = some q →
        q.isPhantom = false → p = false ∧ a.subset (u.abilities P) = true) →
      ((aa.zip ph).foldl (fun acc (x : AbilitySet × Bool) =>
          if x.2 then acc else acc.inter (requiredBy x.1)) acc).subset
        (acc.inter (RuntimeType.argumentAbilities P us qs)) = true
  | [], [], [], [], acc, _, _, _, _ => by
    simp only [List.zip_nil_left, List.foldl_nil, RuntimeType.argumentAbilities]
    cases acc
    simp [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter, AbilitySet.all]
  | a :: aa, p :: ph, u :: us, q :: qs, acc, h1, h2, h3, hpt => by
    simp only [List.zip_cons_cons, List.foldl_cons]
    have ih := polyFold_sub aa ph us qs (if p then acc else acc.inter (requiredBy a))
      (by simpa using h1) (by simpa using h2) (by simpa using h3)
      (fun i a' p' u' q' e1 e2 e3 e4 => hpt (i + 1) a' p' u' q' e1 e2 e3 e4)
    simp only [RuntimeType.argumentAbilities]
    have hacc : (if p then acc else acc.inter (requiredBy a)).subset acc = true := by
      split
      · exact AbilitySet.subset_refl' _
      · exact AbilitySet.inter_subset_left'' _ _
    have hfac : (if p then acc else acc.inter (requiredBy a)).subset
        (if q.isPhantom then AbilitySet.all else (u.abilities P).forContainer) = true := by
      by_cases hq : q.isPhantom = true
      · simp only [hq, ↓reduceIte]
        exact AbilitySet.subset_all' _
      · have hq' : q.isPhantom = false := by simpa using hq
        obtain ⟨hp, hsub⟩ := hpt 0 a p u q rfl rfl rfl rfl hq'
        subst hp
        simp only [hq', Bool.false_eq_true, ↓reduceIte]
        rw [requiredBy_eq]
        exact AbilitySet.subset_trans' (AbilitySet.inter_subset_right'' _ _)
          (AbilitySet.forContainer_mono' hsub)
    obtain ⟨hl, hr⟩ := AbilitySet.subset_inter_iff.1 ih
    refine AbilitySet.subset_inter_iff.2 ⟨AbilitySet.subset_trans' hl hacc, ?_⟩
    exact AbilitySet.subset_inter_iff.2 ⟨AbilitySet.subset_trans' hl hfac, hr⟩
  | [], _ :: _, _, _, _, h1, _, _, _ | _ :: _, [], _, _, _, h1, _, _, _ => by simp at h1
  | [], [], _ :: _, _, _, _, h2, _, _ | _ :: _, _ :: _, [], _, _, _, h2, _, _ => by simp at h2
  | [], [], [], _ :: _, _, _, _, h3, _ | _ :: _, _ :: _, _ :: _, [], _, _, _, h3, _ => by
    simp at h3

theorem abilities_attach {m : CompiledModule} {cs : List AbilitySet} (l : List SignatureToken) :
    l.attach.mapM (fun ⟨x, _⟩ => abilities m cs x) = l.mapM (abilities m cs) := by
  rw [List.mapM_subtype (g := abilities m cs) (fun _ _ => rfl), List.unattach_attach]

theorem mapM_ok_getElem' {α β ε : Type} {f : α → Except ε β} :
    ∀ {l : List α} {r : List β}, l.mapM f = .ok r →
      r.length = l.length ∧ ∀ (i : Nat) (b : β), r[i]? = some b → ∃ a, l[i]? = some a ∧ f a = .ok b
  | [], r, h => by simp [pure, Except.pure] at h; subst h; simp
  | x :: xs, r, h => by
    simp only [List.mapM_cons, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨b, hb, bs, hbs, rfl⟩ := h
    obtain ⟨hlen, hget⟩ := mapM_ok_getElem' hbs
    refine ⟨by simp [hlen], fun i c hc => ?_⟩
    cases i with
    | zero => simp at hc; subst hc; exact ⟨x, rfl, hb⟩
    | succ i => simpa using hget i c (by simpa using hc)

section
variable {P : Program} {m : CompiledModule} (hk : HandlesResolve P m) {cs : List AbilitySet}
  {τ : List RuntimeType} (hτ : RuntimeType.argumentsWellFormed P τ cs = true)

include hk hτ

set_option linter.unusedSectionVars false in
mutual
/-- **The model's abilities are at most the runtime type's**, for a token
translated under type arguments that satisfy the constraints the model
assumes, given conservative struct handles (K1). -/
theorem abilities_sub_gen :
    ∀ (t : SignatureToken) {a : AbilitySet} {u : RuntimeType}, abilities m cs t = .ok a →
      RuntimeType.ofTokenWith m τ t = some u → a.subset (u.abilities P) = true
  | .bool, _, _, ha, hu | .u8, _, _, ha, hu | .u16, _, _, ha, hu | .u32, _, _, ha, hu
  | .u64, _, _, ha, hu | .u128, _, _, ha, hu | .u256, _, _, ha, hu | .i8, _, _, ha, hu
  | .i16, _, _, ha, hu | .i32, _, _, ha, hu | .i64, _, _, ha, hu | .i128, _, _, ha, hu
  | .i256, _, _, ha, hu | .address, _, _, ha, hu | .signer, _, _, ha, hu => by
    simp [abilities, RuntimeType.ofTokenWith, pure, Except.pure] at ha hu
    subst ha hu
    first
      | exact AbilitySet.subset_refl' _
      | decide
  | .vector t, a, u, ha, hu => by
    simp only [abilities, bind_ok] at ha
    obtain ⟨a', ha', hpoly⟩ := ha
    simp only [RuntimeType.ofTokenWith, Option.map_eq_some_iff] at hu
    obtain ⟨u', hu', rfl⟩ := hu
    have ih := abilities_sub_gen t ha' hu'
    simp [polymorphicAbilities, pure, Except.pure] at hpoly
    subst hpoly
    simp only [RuntimeType.abilities]
    rw [requiredBy_eq]
    exact AbilitySet.inter_subset_inter (AbilitySet.subset_refl' _)
      (AbilitySet.forContainer_mono' ih)
  | .struct h, a, u, ha, hu => by
    simp only [abilities, bind_ok, lookup_ok, pure, Except.pure, Except.ok.injEq] at ha
    obtain ⟨sh, hsh, rfl⟩ := ha
    simp only [RuntimeType.ofTokenWith, hsh, bind, Option.bind_some] at hu
    split at hu
    · next hnp =>
      simp only [pure, Option.bind_eq_some_iff, Option.some.injEq] at hu
      obtain ⟨sid, hsid, rfl⟩ := hu
      obtain ⟨s, r, info, hs, hr, hi, hcon⟩ := hk h sh hsh
      rw [hsid, Option.some.injEq] at hs
      subst hs
      have hnp' : sh.typeParameters = [] := List.isEmpty_iff.1 hnp
      have hargs : sid.typeArgs = [] := by rw [← structIdAt_typeArgs hsid]
      have hlen : info.handle.typeParameters = [] := by
        have := hcon.2.1
        rw [hnp'] at this
        exact List.eq_nil_of_length_eq_zero this.symm
      have hst : P.structType? sid = some info := by
        have := structType?_instance (tys := []) hr hi (by simp [hlen]) (by simp)
        rwa [show ({ sid with typeArgs := [] } : StructId) = sid by cases sid; simp_all] at this
      simp only [RuntimeType.abilities, hst, hargs, hlen, RuntimeType.argumentAbilities,
        AbilitySet.all_inter]
      exact hcon.1
    · simp at hu
  | .structInstantiation h args, a, u, ha, hu => by
    simp only [abilities, bind_ok, lookup_ok, abilities_attach] at ha
    obtain ⟨sh, hsh, aa, haa, hpoly⟩ := ha
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
      simp only [polymorphicAbilities] at hpoly
      split at hpoly
      · simp [reject] at hpoly
      · next hl =>
        simp only [pure, Except.pure, Except.ok.injEq] at hpoly
        subst hpoly
        obtain ⟨haal, haget⟩ := mapM_ok_getElem' haa
        have htl := ofTokenListWith_length htargs
        have hfold := polyFold_sub (P := P) aa (sh.typeParameters.map (·.isPhantom)) targs
          info.handle.typeParameters sh.abilities
          (by simp at hl ⊢; omega) (by simp; omega) (by rw [← hcon.2.1]; omega)
          (fun i a' p u' q e1 e2 e3 e4 hq => by
            obtain ⟨t, ht, hta⟩ := haget i a' e1
            have hut := ofTokenListWith_getElem? htargs ht
            rw [e3] at hut
            obtain ⟨p', hp', rfl⟩ : ∃ p', sh.typeParameters[i]? = some p' ∧ p = p'.isPhantom := by
              simp only [List.getElem?_map, Option.map_eq_some_iff] at e2
              obtain ⟨p', hp', rfl⟩ := e2
              exact ⟨p', hp', rfl⟩
            have hpq := hcon.2.2 i p' q hp' e4
            refine ⟨?_, abilitiesList_sub_gen args haa htargs i a' u' e1 e3⟩
            cases hpp : p'.isPhantom
            · rfl
            · rw [hpq.1 hpp] at hq
              cases hq)
        simp only [RuntimeType.abilities, hst]
        rw [AbilitySet.inter_comm' (RuntimeType.argumentAbilities P targs _)]
        obtain ⟨hl1, hl2⟩ := AbilitySet.subset_inter_iff.1 hfold
        exact AbilitySet.subset_inter_iff.2 ⟨AbilitySet.subset_trans' hl1 hcon.1, hl2⟩
    · simp at hu
  | .typeParameter i, a, u, ha, hu => by
    simp only [abilities, lookup_ok, List.getElem?_toArray] at ha
    simp only [RuntimeType.ofTokenWith] at hu
    exact (argumentsWellFormed_getElem hτ hu ha).2.2
  | .function ps rs ab, a, u, ha, hu => by
    simp only [abilities, pure, Except.pure, Except.ok.injEq] at ha
    simp only [RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at hu
    obtain ⟨_, -, _, -, rfl⟩ := hu
    subst ha
    simp only [RuntimeType.abilities]
    exact AbilitySet.subset_refl' _
  | .reference _, _, _, _, hu | .mutableReference _, _, _, _, hu => by
    simp [RuntimeType.ofTokenWith] at hu

theorem abilitiesList_sub_gen :
    ∀ (args : List SignatureToken) {aa : List AbilitySet} {us : List RuntimeType},
      args.mapM (abilities m cs) = .ok aa → RuntimeType.ofTokenListWith m τ args = some us →
      ∀ (i : Nat) (a : AbilitySet) (u : RuntimeType), aa[i]? = some a → us[i]? = some u →
        a.subset (u.abilities P) = true
  | [], aa, us, ha, _, i, a, u, hai, _ => by
    simp only [List.mapM_nil, pure, Except.pure, Except.ok.injEq] at ha
    subst ha
    simp at hai
  | t :: ts, aa, us, ha, hu, i, a, u, hai, hui => by
    simp only [List.mapM_cons, bind_ok, pure, Except.pure, Except.ok.injEq] at ha
    obtain ⟨b, hb, bs, hbs, rfl⟩ := ha
    simp only [RuntimeType.ofTokenListWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at hu
    obtain ⟨v, hv, vs, hvs, rfl⟩ := hu
    cases i with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hai hui
      subst hai hui
      exact abilities_sub_gen t hb hv
    | succ i =>
      simp only [List.getElem?_cons_succ] at hai hui
      exact abilitiesList_sub_gen ts hbs hvs i a u hai hui
end

/-- The same for slot tokens. -/
theorem abilities_sub_slot_gen (t : SignatureToken) {a : AbilitySet} {u : RuntimeType}
    (ha : abilities m cs t = .ok a) (hu : RuntimeType.ofSlotTokenWith m τ t = some u) :
    a.subset (u.abilities P) = true := by
  cases t with
  | reference t' | mutableReference t' =>
    simp only [abilities, pure, Except.pure, Except.ok.injEq] at ha
    simp only [RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at hu
    obtain ⟨_, -, rfl⟩ := hu
    subst ha
    simp [RuntimeType.abilities, referenceAbilities, MoveBytecode.AbilitySet.subset]
  | _ => exact abilities_sub_gen hk hτ _ ha (by simpa [RuntimeType.ofSlotTokenWith] using hu)

end

end MoveBytecode.Proofs
