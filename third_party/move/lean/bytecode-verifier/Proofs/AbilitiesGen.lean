-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.TypingGen
import Proofs.TypeFormation

/-!
# Declared abilities at every instance

UNTRUSTED (checked by the kernel). Generic structs included, a struct instance
has no ability its fields at the instance do not justify (`abilitiesSound_gen`):
every value of an instance then has the abilities of its type
(`abilities_of_conforms`).

The struct-definition check verifies each field with every type parameter
assumed to have every ability (L6, `structAbilities_of_accepted`). At an
instance, an ability of the instance requires the field rule of every
non-phantom type argument (`RuntimeType.argumentAbilities`), and phantom type
parameters occur in a field only in phantom positions outside function types,
whose abilities are their own (`PhantomVal`, which the model's phantom check
establishes), and the abilities ignore phantom arguments. So what the instance's abilities require
of a field transfers from the check, where parameters have every ability, to
the instance, where only non-phantom parameters are used
(`abilitiesIn_transfer`). The requirement has no `key` (`reqOf`), so it passes
through containers unchanged.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Requirements -/

/-- What abilities `a` of a container require of its contents: `forContainer`
read backwards (`AbilitySet::requires`). -/
def reqOf (a : AbilitySet) : AbilitySet := ⟨a.copy, a.drop, a.store || a.key, false⟩

theorem reqOf_key (a : AbilitySet) : (reqOf a).key = false := rfl

theorem reqOf_sub_of_forContainer {a x : AbilitySet} (h : a.subset x.forContainer = true) :
    (reqOf a).subset x = true := by
  cases a; cases x
  simp_all [reqOf, MoveBytecode.AbilitySet.subset, AbilitySet.forContainer]
  grind

theorem forContainer_of_reqOf {a x : AbilitySet} (h : (reqOf a).subset x = true) :
    a.subset x.forContainer = true := by
  cases a; cases x
  simp_all [reqOf, MoveBytecode.AbilitySet.subset, AbilitySet.forContainer]
  grind

theorem sub_of_forContainer_keyFree {r x : AbilitySet} (hr : r.key = false)
    (h : r.subset x.forContainer = true) : r.subset x = true := by
  cases r; cases x
  simp_all [MoveBytecode.AbilitySet.subset, AbilitySet.forContainer]

theorem forContainer_of_keyFree {r x : AbilitySet} (hr : r.key = false)
    (h : r.subset x = true) : r.subset x.forContainer = true := by
  cases r; cases x
  simp_all [MoveBytecode.AbilitySet.subset, AbilitySet.forContainer]

/-- What an instance's argument abilities give each non-phantom argument. -/
theorem argumentAbilities_getElem {P : Program} {a : AbilitySet} :
    ∀ {us : List RuntimeType} {qs : List StructTypeParameter},
      a.subset (RuntimeType.argumentAbilities P us qs) = true →
      ∀ {i : Nat} {x : RuntimeType} {q : StructTypeParameter}, us[i]? = some x → qs[i]? = some q →
        q.isPhantom = false → a.subset (x.abilities P).forContainer = true
  | [], _, _, _, _, _, hx, _, _ => by simp at hx
  | _ :: _, [], _, _, _, _, _, hq, _ => by simp at hq
  | u :: us, q :: qs, h, i, x, q', hx, hq, hph => by
    simp only [RuntimeType.argumentAbilities] at h
    obtain ⟨h1, h2⟩ := AbilitySet.subset_inter_iff.1 h
    cases i with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hx hq
      subst hx hq
      simpa [hph] using h1
    | succ i =>
      simp only [List.getElem?_cons_succ] at hx hq
      exact argumentAbilities_getElem h2 hx hq hph

/-! ## The phantom rule outside function types

A function type's abilities are those it carries, whatever its parameter and
result types, so the transfer of abilities needs the phantom-position rule
only outside function types. The model's phantom check establishes it there
whether or not it looks through references: references occur in a value type
only under a function type. -/

mutual
/-- The phantom-position rule of `SignatureToken.PhantomRespected`, outside
function types. -/
def PhantomVal (m : CompiledModule) (params : List StructTypeParameter) : SignatureToken → Prop
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 => True
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => True
  | .address | .signer | .struct _ => True
  | .function .. | .reference _ | .mutableReference _ => True
  | .vector elem => PhantomVal m params elem
  | .structInstantiation h args =>
    ∃ sh, m.structHandles[h]? = some sh ∧ PhantomValArgs m params args sh.typeParameters
  | .typeParameter i => ∃ p, params[i]? = some p ∧ p.isPhantom = false

def PhantomValArgs (m : CompiledModule) (params : List StructTypeParameter) :
    List SignatureToken → List StructTypeParameter → Prop
  | [], _ => True
  | arg :: args, q :: qs => (q.isPhantom = true ∨ PhantomVal m params arg) ∧
    PhantomValArgs m params args qs
  | arg :: args, [] => PhantomVal m params arg ∧ PhantomValArgs m params args []
end

mutual
theorem checkPhantom_val {m : CompiledModule} {ctx : List StructTypeParameter} {refs : Bool} :
    ∀ (t : SignatureToken), SignatureV2.checkPhantom m ctx refs t false = .ok () →
      PhantomVal m ctx t
  | .vector e, h => by
    simp only [SignatureV2.checkPhantom] at h
    simpa only [PhantomVal] using checkPhantom_val e h
  | .structInstantiation hh args, h => by
    simp only [SignatureV2.checkPhantom, bind_ok, lookup_ok] at h
    obtain ⟨sh, hsh, h⟩ := h
    simp only [PhantomVal]
    exact ⟨sh, hsh, by simpa using checkPhantomArgs_val sh.typeParameters args 0 h⟩
  | .typeParameter i, h => by
    simp only [SignatureV2.checkPhantom, bind_ok, lookup_ok] at h
    obtain ⟨p, hp, h⟩ := h
    simp only [PhantomVal]
    refine ⟨p, by simpa using hp, ?_⟩
    split at h
    · simp [reject] at h
    · next hc => simpa using hc
  | .function .., _ | .reference _, _ | .mutableReference _, _
  | .bool, _ | .u8, _ | .u16, _ | .u32, _ | .u64, _ | .u128, _ | .u256, _ | .i8, _ | .i16, _
  | .i32, _ | .i64, _ | .i128, _ | .i256, _ | .address, _ | .signer, _ | .struct _, _ => by
    simp [PhantomVal]

theorem checkPhantomArgs_val {m : CompiledModule} {ctx : List StructTypeParameter} {refs : Bool} :
    ∀ (params : List StructTypeParameter) (args : List SignatureToken) (i : Nat),
      SignatureV2.checkPhantomArgs m ctx refs params args i = .ok () →
      PhantomValArgs m ctx args (params.drop i)
  | _, [], _, _ => by simp [PhantomValArgs]
  | params, a :: as, i, h => by
    simp only [SignatureV2.checkPhantomArgs, bind_ok, lookup_ok] at h
    obtain ⟨p, hp, u, h1, h2⟩ := h
    cases u
    have hp' : params[i]? = some p := by simpa using hp
    have hdrop : params.drop i = p :: params.drop (i + 1) := by
      obtain ⟨hlt, heq⟩ := List.getElem?_eq_some_iff.1 hp'
      rw [List.drop_eq_getElem_cons hlt, heq]
    rw [hdrop]
    simp only [PhantomValArgs]
    refine ⟨?_, checkPhantomArgs_val params as (i + 1) h2⟩
    cases hph : p.isPhantom
    · exact Or.inr (checkPhantom_val a (by simpa [hph] using h1))
    · exact Or.inl rfl
end

/-! ## The transfer -/

section Transfer

variable {P : Program} {m : CompiledModule} (hk : HandlesResolve P m)
  {params : List StructTypeParameter} {targs : List RuntimeType}
  (hv : ∀ x ∈ targs, x.isValue = true) {r : AbilitySet} (hr : r.key = false)
  (hnp : ∀ (i : Nat) (p : StructTypeParameter) (x : RuntimeType), params[i]? = some p →
    p.isPhantom = false → targs[i]? = some x → r.subset (x.abilities P) = true)

include hk hv hr hnp

set_option linter.unusedSectionVars false in
mutual
/-- A requirement `r` without `key` that a token meets with every type
parameter given every ability, its translation meets at an instance whose
non-phantom type arguments meet `r`, when phantom type parameters occur only
in phantom positions and struct handles are conservative (K1). -/
theorem abilitiesIn_transfer :
    ∀ (t : SignatureToken) {u : RuntimeType}, RuntimeType.ofTokenWith m targs t = some u →
      PhantomVal m params t →
      r.subset (t.abilitiesIn m (params.map fun _ => AbilitySet.all)) = true →
      r.subset (u.abilities P) = true
  | .bool, _, h, _, ha | .u8, _, h, _, ha | .u16, _, h, _, ha | .u32, _, h, _, ha
  | .u64, _, h, _, ha | .u128, _, h, _, ha | .u256, _, h, _, ha | .i8, _, h, _, ha
  | .i16, _, h, _, ha | .i32, _, h, _, ha | .i64, _, h, _, ha | .i128, _, h, _, ha
  | .i256, _, h, _, ha | .address, _, h, _, ha | .signer, _, h, _, ha => by
    simp only [RuntimeType.ofTokenWith, Option.some.injEq] at h
    subst h
    simpa [SignatureToken.abilitiesIn, RuntimeType.abilities] using ha
  | .vector e, u, h, hp, ha => by
    simp only [RuntimeType.ofTokenWith, Option.map_eq_some_iff] at h
    obtain ⟨e', he', rfl⟩ := h
    simp only [PhantomVal] at hp
    simp only [SignatureToken.abilitiesIn] at ha
    obtain ⟨h1, h2⟩ := AbilitySet.subset_inter_iff.1 ha
    have ih := abilitiesIn_transfer e he' hp (sub_of_forContainer_keyFree hr h2)
    simp only [RuntimeType.abilities]
    exact AbilitySet.subset_inter_iff.2 ⟨h1, forContainer_of_keyFree hr ih⟩
  | .struct h, u, hu, _, ha => by
    simp only [RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff] at hu
    obtain ⟨sh, hsh, hu⟩ := hu
    split at hu
    · next hnp' =>
      simp only [pure, Option.bind_eq_some_iff, Option.some.injEq] at hu
      obtain ⟨sid, hsid, rfl⟩ := hu
      obtain ⟨s, r', info, hs, hres, hi, hcon⟩ := hk h sh hsh
      rw [hsid, Option.some.injEq] at hs
      subst hs
      have hnp'' : sh.typeParameters = [] := List.isEmpty_iff.1 hnp'
      have hargs : sid.typeArgs = [] := by rw [← structIdAt_typeArgs hsid]
      have hlen : info.handle.typeParameters = [] := by
        have := hcon.2.1
        rw [hnp''] at this
        exact List.eq_nil_of_length_eq_zero this.symm
      have hst : P.structType? sid = some info := by
        have := structType?_instance (tys := []) hres hi (by simp [hlen]) (by simp)
        rwa [show ({ sid with typeArgs := [] } : StructId) = sid by cases sid; simp_all] at this
      simp only [SignatureToken.abilitiesIn, hsh] at ha
      simp only [RuntimeType.abilities, hst, hargs, hlen, RuntimeType.argumentAbilities,
        AbilitySet.all_inter]
      exact AbilitySet.subset_trans' ha hcon.1
    · simp at hu
  | .structInstantiation h args, u, hu, hp, ha => by
    simp only [RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff] at hu
    obtain ⟨sh, hsh, sid, hsid, targs', htargs', hu⟩ := hu
    split at hu
    · next hlen =>
      simp only [pure, Option.some.injEq] at hu
      subst hu
      obtain ⟨s, r', info, hs, hres, hi, hcon⟩ := hk h sh hsh
      rw [hsid, Option.some.injEq] at hs
      subst hs
      have hv' : ∀ x ∈ targs', x.isValue = true := ofTokenListWith_values hv htargs'
      have hst := structType?_instance hres hi (hlen.trans hcon.2.1) hv'
      simp only [PhantomVal] at hp
      obtain ⟨sh', hsh', hpa⟩ := hp
      rw [hsh, Option.some.injEq] at hsh'
      subst hsh'
      simp only [SignatureToken.abilitiesIn, hsh] at ha
      obtain ⟨h1, h2⟩ := AbilitySet.subset_inter_iff.1 ha
      have hargsab := argumentsIn_transfer args sh.typeParameters info.handle.typeParameters
        htargs' ((ofTokenListWith_length htargs').symm.trans hlen) hcon.2.1 hcon.2.2 hpa h2
      simp only [RuntimeType.abilities, hst]
      exact AbilitySet.subset_inter_iff.2 ⟨hargsab, AbilitySet.subset_trans' h1 hcon.1⟩
    · simp at hu
  | .typeParameter i, u, hu, hp, _ => by
    simp only [RuntimeType.ofTokenWith] at hu
    simp only [PhantomVal] at hp
    obtain ⟨p, hpi, hph⟩ := hp
    exact hnp i p u hpi hph hu
  | .function .., u, hu, _, ha => by
    simp only [RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at hu
    obtain ⟨_, -, _, -, rfl⟩ := hu
    simpa [SignatureToken.abilitiesIn, RuntimeType.abilities] using ha
  | .reference _, _, hu, _, _ | .mutableReference _, _, hu, _, _ => by
    simp [RuntimeType.ofTokenWith] at hu

/-- The arguments of a struct instantiation, against the parameters of the
handle (`ps`) and of the definition (`qs`) it is a conservative view of. -/
theorem argumentsIn_transfer :
    ∀ (args : List SignatureToken) (ps qs : List StructTypeParameter) {us : List RuntimeType},
      RuntimeType.ofTokenListWith m targs args = some us →
      args.length = ps.length → ps.length = qs.length →
      (∀ (i : Nat) (p q : StructTypeParameter), ps[i]? = some p → qs[i]? = some q →
        p.ConservativeFor q) →
      PhantomValArgs m params args ps →
      r.subset (SignatureToken.argumentAbilities m (params.map fun _ => AbilitySet.all) args ps)
        = true →
      r.subset (RuntimeType.argumentAbilities P us qs) = true
  | [], [], [], us, h, _, _, _, _, _ => by
    simp only [RuntimeType.ofTokenListWith, Option.some.injEq] at h
    subst h
    simp only [RuntimeType.argumentAbilities]
    exact AbilitySet.subset_all' _
  | a :: as, p :: ps, q :: qs, us, h, hl1, hl2, hcon, hpa, ha => by
    simp only [RuntimeType.ofTokenListWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨u, hu, us', hus', rfl⟩ := h
    simp only [PhantomValArgs] at hpa
    obtain ⟨hpa1, hpa2⟩ := hpa
    simp only [SignatureToken.argumentAbilities] at ha
    obtain ⟨h1, h2⟩ := AbilitySet.subset_inter_iff.1 ha
    have hpq := hcon 0 p q rfl rfl
    have ih := argumentsIn_transfer as ps qs hus' (by simpa using hl1) (by simpa using hl2)
      (fun i p' q' e1 e2 => hcon (i + 1) p' q' e1 e2) hpa2 h2
    simp only [RuntimeType.argumentAbilities]
    refine AbilitySet.subset_inter_iff.2 ⟨?_, ih⟩
    cases hq : q.isPhantom
    · have hp : p.isPhantom = false := by
        cases hpp : p.isPhantom
        · rfl
        · rw [hpq.1 hpp] at hq
          cases hq
      simp only [Bool.false_eq_true, ↓reduceIte]
      simp only [hp, Bool.false_eq_true, ↓reduceIte] at h1
      have hresp : PhantomVal m params a := by
        rcases hpa1 with h' | h'
        · rw [hp] at h'
          cases h'
        · exact h'
      exact forContainer_of_keyFree hr
        (abilitiesIn_transfer a hu hresp (sub_of_forContainer_keyFree hr h1))
    · simp only [↓reduceIte]
      exact AbilitySet.subset_all' _
  | [], _ :: _, _, _, _, hl1, _, _, _, _ | _ :: _, [], _, _, _, hl1, _, _, _, _ => by
    simp at hl1
  | [], [], _ :: _, _, _, _, hl2, _, _, _ | _ :: _, _ :: _, [], _, _, _, hl2, _, _, _ => by
    simp at hl2
end

end Transfer

/-! ## Declared abilities -/

/-- **Declared abilities are justified by the fields**, at every instance,
generic structs included. The struct-definition check requires of each
field's type, with the struct's type parameters given every ability, what
the declared abilities require; an instance's abilities require the same of
its non-phantom type arguments, phantom parameters occur only in phantom
positions, and by K1 the defining module's handles declare at most the
abilities of the types they name. -/
theorem abilitiesSound_gen {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P) :
    AbilitiesSound P := by
  intro s tys htys t ht
  obtain ⟨info, hty, fields, hfields, hmap⟩ := fieldTypes_parts htys
  obtain ⟨mi, di, hm, hsd, hsh⟩ := structType_parts hty
  obtain ⟨f, hf, htf⟩ := mapM_some_mem hmap t ht
  have hacc := (wellFormed_module hwf hm).1
  have hdef := getElem?_mem_toList hsd
  obtain ⟨sh, hsh', hab⟩ := structAbilities_of_accepted hacc _ hdef
  have hsh'' : info.module.structHandles[info.definition.structHandle]? = some sh := hsh'
  rw [hsh, Option.some.injEq] at hsh''
  subst hsh''
  obtain ⟨sh', hsh2, hchecks⟩ :=
    verifyStructDef_ok (verifySignatures_structDefs (verifyModule_signatures hacc) _ hdef)
  rw [hsh, Option.some.injEq] at hsh2
  subst hsh2
  have hph : PhantomVal info.module info.handle.typeParameters f.signature :=
    checkPhantom_val f.signature (hchecks f (hfields f hf)).2.2
  have hA : (RuntimeType.struct s).abilities P =
      (RuntimeType.argumentAbilities P s.typeArgs info.handle.typeParameters).inter
        info.handle.abilities := by
    simp [RuntimeType.abilities, hty]
  rw [hA]
  apply forContainer_of_reqOf
  refine abilitiesIn_transfer (fun h sh hsh => struct_handle_resolves hwf hm hsh)
    (structType?_args_value hty) (reqOf_key _) (fun i p x hp hph hx => ?_) f.signature htf hph
    (reqOf_sub_of_forContainer (AbilitySet.subset_trans'
      (AbilitySet.inter_subset_right'' _ _) (hab f (hfields f hf))))
  exact reqOf_sub_of_forContainer (argumentAbilities_getElem
    (AbilitySet.inter_subset_left'' _ _) hx hp hph)

end MoveBytecode.Proofs
