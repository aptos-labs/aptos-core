-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Local

/-!
# Declarations: L5, L6, L10 from the model

UNTRUSTED (checked by the kernel). For a module the model accepts:

- L10 from the friends pass. A module handle names a module by indices into
  the identifier and address tables, which have no duplicates, so a friend
  whose module id is the module's own is the module's own handle, which the
  pass rejects.
- The abilities part of L6 from the signature checker: what it requires of
  a type, in a context of type-parameter abilities, the type has
  (`checkTy_abilitiesIn`), and it requires of each field what the struct's
  declared abilities require (`requires`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Passes of an accepted module -/

theorem verifyModule_friends {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) : verifyFriends m = .ok () := by
  unfold verifyModule at h
  simp only [show (VerifierConfig.production.scope == .nothing) = false from rfl,
    Bool.false_eq_true, ↓reduceIte, bind_ok] at h
  obtain ⟨-, -, -, -, -, -, -, -, -, -, -, -, -, -, u8, h8, -⟩ := h
  cases u8
  exact h8

theorem verifyModule_structDefs {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) : verifyStructDefs m = .ok () := by
  unfold verifyModule at h
  simp only [show (VerifierConfig.production.scope == .nothing) = false from rfl,
    Bool.false_eq_true, ↓reduceIte, bind_ok] at h
  obtain ⟨-, -, -, -, -, -, -, -, -, -, -, -, -, -, -, -, u9, h9, -⟩ := h
  cases u9
  exact h9

/-! ## L10 -/

theorem mapM_ok_of_mem {α β ε : Type} {f : α → Except ε β} :
    ∀ {l : List α} {r : List β}, l.mapM f = .ok r → ∀ x ∈ l, ∃ y ∈ r, f x = .ok y
  | [], _, _, x, hx => by simp at hx
  | a :: as, r, h, x, hx => by
    simp only [List.mapM_cons, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨b, hb, bs, hbs, rfl⟩ := h
    rcases List.mem_cons.1 hx with rfl | hx
    · exact ⟨b, List.mem_cons_self .., hb⟩
    · obtain ⟨y, hy, hfy⟩ := mapM_ok_of_mem hbs x hx
      exact ⟨y, List.mem_cons_of_mem _ hy, hfy⟩

/-- **L10** for a module the model accepts. -/
theorem friendsWellFormed_of_accepted {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) : m.FriendsWellFormed := by
  intro mh hmh
  have hb := verifyBounds_handles (verifyModule_ok h).1
  have hu := verifyDuplication_ok (verifyModule_ok h).2.1
  have hf := verifyModule_friends h
  unfold verifyFriends at hf
  simp only [bind_ok, lookup_ok] at hf
  obtain ⟨selfHandle, hself, hf⟩ := hf
  split at hf
  · simp [reject] at hf
  · next hnot =>
    simp only [bind_ok, lookup_ok] at hf
    obtain ⟨selfAddress, hsa, addresses, haddrs, hf⟩ := hf
    split at hf
    · simp [reject] at hf
    · next hall =>
      have hsm := getElem?_mem_toList hself
      obtain ⟨selfName, hsn⟩ := getElem?_of_lt (hb.moduleHandles selfHandle hsm).2
      obtain ⟨fa, hfa⟩ := getElem?_of_lt (hb.friends mh hmh).1
      obtain ⟨fn, hfn⟩ := getElem?_of_lt (hb.friends mh hmh).2
      have hfaself : fa = selfAddress := by
        obtain ⟨x, hx, hfx⟩ := mapM_ok_of_mem haddrs mh hmh
        rw [lookup_ok, hfa, Option.some.injEq] at hfx
        subst hfx
        simp only [List.any_eq_true, bne_iff_ne, ne_eq, not_exists, not_and,
          Decidable.not_not] at hall
        exact hall _ hx
      refine ⟨⟨selfAddress, selfName⟩, ⟨fa, fn⟩, ?_, ?_, hfaself, fun heq => ?_⟩
      · simp [CompiledModule.selfId, CompiledModule.moduleIdAt, CompiledModule.moduleIdOf, hself,
          hsa, hsn]
      · simp [CompiledModule.moduleIdOf, hfa, hfn]
      · simp only [ModuleId.mk.injEq] at heq
        obtain ⟨-, rfl⟩ := heq
        subst hfaself
        have ha : mh.address = selfHandle.address :=
          nodup_index hu.addresses (by rw [toList_getElem?]; exact hfa)
            (by rw [toList_getElem?]; exact hsa)
        have hn : mh.name = selfHandle.name :=
          nodup_index hu.identifiers (by rw [toList_getElem?]; exact hfn)
            (by rw [toList_getElem?]; exact hsn)
        have : mh = selfHandle := by cases mh; cases selfHandle; simp_all
        subst this
        exact hnot (by simpa using hmh)

/-! ## What the signature checker requires of a type -/

/-- The type-parameter constraints the signature checker collects hold in the
context `ctx`. -/
def ConstraintsHold (ctx : Nat → AbilitySet) (cs : SignatureV2.Constraints) : Prop :=
  ∀ p ∈ cs, p.2.subset (ctx p.1) = true

theorem ConstraintsHold.append {ctx : Nat → AbilitySet} {a b : SignatureV2.Constraints}
    (h : ConstraintsHold ctx (a ++ b)) : ConstraintsHold ctx a ∧ ConstraintsHold ctx b :=
  ⟨fun p hp => h p (List.mem_append_left _ hp), fun p hp => h p (List.mem_append_right _ hp)⟩

theorem subset_of_union_right {a b c : AbilitySet} (h : (a.union b).subset c = true) :
    b.subset c = true := by
  cases a; cases b; cases c
  simp_all [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.union]
  grind

theorem subset_of_union_left {a b c : AbilitySet} (h : (a.union b).subset c = true) :
    a.subset c = true := by
  cases a; cases b; cases c
  simp_all [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.union]
  grind

section CheckTy

variable {config : VerifierConfig} {m : CompiledModule} {tparams : List AbilitySet}
  {ctx : Nat → AbilitySet}

mutual
/-- What the signature checker requires of a type, the type has, computed with
the module's own handles, when the collected constraints hold in a context
that gives each type parameter its abilities. -/
theorem checkTy_abilitiesIn (hctx : ∀ i, ctx i = tparams.getD i AbilitySet.empty) :
    ∀ (t : SignatureToken) {allowRef : Bool} {req : AbilitySet} {cs : SignatureV2.Constraints},
      SignatureV2.checkTy config m t allowRef req = .ok cs → ConstraintsHold ctx cs →
      req.subset (t.abilitiesIn m tparams) = true
  | .typeParameter i, _, req, cs, h, hc => by
    simp only [SignatureV2.checkTy, pure, Except.pure, Except.ok.injEq] at h
    subst h
    have := hc (i, req) (List.mem_singleton_self _)
    simp only [SignatureToken.abilitiesIn]
    rw [← hctx]
    exact this
  | .vector t, _, req, cs, h, hc => by
    simp only [SignatureV2.checkTy, bind_ok] at h
    obtain ⟨_, ha, h⟩ := h
    simp only [SignatureToken.abilitiesIn]
    exact requires_vector (assertAbilities_ok ha) (checkTy_abilitiesIn hctx t h hc)
  | .function ps rs ab, _, req, cs, h, hc => by
    simp only [SignatureV2.checkTy, bind_ok] at h
    obtain ⟨_, ha, -⟩ := h
    simp only [SignatureToken.abilitiesIn]
    exact assertAbilities_ok ha
  | .struct h, _, req, cs, hc', hc => by
    simp only [SignatureV2.checkTy, bind_ok, lookup_ok] at hc'
    obtain ⟨sh, hsh, _, ha, -⟩ := hc'
    simp only [SignatureToken.abilitiesIn, hsh]
    exact assertAbilities_ok ha
  | .structInstantiation h args, _, req, cs, hc', hc => by
    simp only [SignatureV2.checkTy, bind_ok, lookup_ok] at hc'
    obtain ⟨sh, hsh, _, ha, hargs⟩ := hc'
    split at hargs
    · simp [reject, bind, Except.bind] at hargs
    · simp only [SignatureToken.abilitiesIn, hsh]
      exact AbilitySet.subset_inter.2 ⟨assertAbilities_ok ha,
        checkTyArgs_abilitiesIn hctx sh.typeParameters args hargs hc⟩
  | .reference t, _, req, cs, h, hc | .mutableReference t, _, req, cs, h, hc => by
    simp only [SignatureV2.checkTy] at h
    split at h
    · simp only [bind_ok] at h
      obtain ⟨_, ha, -⟩ := h
      simp only [SignatureToken.abilitiesIn]
      exact assertAbilities_ok ha
    · simp [reject] at h
  | .bool, _, _, _, h, _ | .u8, _, _, _, h, _ | .u16, _, _, _, h, _ | .u32, _, _, _, h, _
  | .u64, _, _, _, h, _ | .u128, _, _, _, h, _ | .u256, _, _, _, h, _ | .i8, _, _, _, h, _
  | .i16, _, _, _, h, _ | .i32, _, _, _, h, _ | .i64, _, _, _, h, _ | .i128, _, _, _, h, _
  | .i256, _, _, _, h, _ | .address, _, _, _, h, _ | .signer, _, _, _, h, _ => by
    simp only [SignatureV2.checkTy, bind_ok] at h
    obtain ⟨_, ha, -⟩ := h
    simp only [SignatureToken.abilitiesIn]
    exact assertAbilities_ok ha

/-- The type arguments of a struct instantiation have what the struct's
abilities require of them. -/
theorem checkTyArgs_abilitiesIn (hctx : ∀ i, ctx i = tparams.getD i AbilitySet.empty) :
    ∀ (ps : List StructTypeParameter) (args : List SignatureToken) {req : AbilitySet}
      {cs : SignatureV2.Constraints},
      SignatureV2.checkTyArgs config m ps args req = .ok cs → ConstraintsHold ctx cs →
      req.subset (SignatureToken.argumentAbilities m tparams args ps) = true
  | p :: ps, a :: as, req, cs, h, hc => by
    simp only [SignatureV2.checkTyArgs, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨cs1, h1, cs2, h2, rfl⟩ := h
    obtain ⟨hc1, hc2⟩ := hc.append
    simp only [SignatureToken.argumentAbilities]
    refine AbilitySet.subset_inter.2 ⟨?_, checkTyArgs_abilitiesIn hctx ps as h2 hc2⟩
    have ih := checkTy_abilitiesIn hctx a h1 hc1
    split
    · exact AbilitySet.subset_all _
    · next hp =>
      simp only [hp, Bool.false_eq_true, ↓reduceIte] at ih
      exact requires_forContainer (subset_of_union_right ih)
  | [], args, _, _, _, _ => by
    cases args <;> simp [SignatureToken.argumentAbilities, AbilitySet.subset_all]
  | _ :: _, [], _, _, _, _ => by
    simp [SignatureToken.argumentAbilities, AbilitySet.subset_all]
end

end CheckTy

/-! ## L6 -/

theorem checkInContext_ok {cs : SignatureV2.Constraints} {ctx : Nat → AbilitySet}
    (h : SignatureV2.checkInContext cs ctx = .ok ()) : ConstraintsHold ctx cs := by
  unfold SignatureV2.checkInContext at h
  split at h
  · next hall =>
    intro p hp
    simp only [List.all_eq_true] at hall
    exact hall p hp
  · simp [reject] at h

theorem checkTyInContext_ok {config : VerifierConfig} {m : CompiledModule}
    {ctx : Nat → AbilitySet} {t : SignatureToken} {allowRef : Bool} {req : AbilitySet}
    (h : SignatureV2.checkTyInContext config m ctx t allowRef req = .ok ()) :
    ∃ cs, SignatureV2.checkTy config m t allowRef req = .ok cs ∧ ConstraintsHold ctx cs := by
  unfold SignatureV2.checkTyInContext at h
  simp only [bind_ok] at h
  obtain ⟨cs, hcs, h⟩ := h
  exact ⟨cs, hcs, checkInContext_ok h⟩

/-- What the struct-definition check establishes about each field: it is a
well-formed type in the context of the struct's type-parameter constraints,
it has what the struct's abilities require when those parameters have every
ability, and its phantom parameters are in phantom positions. -/
theorem verifyStructDef_ok {config : VerifierConfig} {m : CompiledModule}
    {sd : StructDefinition} (h : SignatureV2.verifyStructDef config m sd = .ok ()) :
    ∃ sh, m.structHandles[sd.structHandle]? = some sh ∧
      ∀ f ∈ CompiledModule.StructFieldInformation.allFields sd.fieldInformation,
        SignatureV2.checkTyInContext config m
          (SignatureV2.listContext (sh.typeParameters.map (·.constraints))) f.signature false
          SignatureV2.emptySet = .ok () ∧
        SignatureV2.checkTyInContext config m
          (fun i => if i < sh.typeParameters.length then SignatureV2.allSet
            else SignatureV2.emptySet) f.signature false (requires sh.abilities) = .ok () ∧
        SignatureV2.checkPhantom m sh.typeParameters
          (SignatureV2.phantomThroughReferences config) f.signature false = .ok () := by
  unfold SignatureV2.verifyStructDef at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨sh, hsh, h⟩ := h
  refine ⟨sh, hsh, fun f hf => ?_⟩
  have hmem : f ∈ (match sd.fieldInformation with
      | .native => []
      | .declared fs => fs
      | .declaredVariants vs => vs.flatMap (fun (v : VariantDefinition) => v.fields)) := by
    cases hfi : sd.fieldInformation <;>
      simp_all [CompiledModule.StructFieldInformation.allFields]
  have := forM_ok h f hmem
  simp only [bind_ok] at this
  obtain ⟨u1, h1, h2⟩ := this
  cases u1
  split at h2
  · next hc => exact ⟨h1, hc, h2⟩
  · simp [reject, bind, Except.bind] at h2

/-- The abilities part of **L6**: every declared ability is justified by every
field, the struct's own type parameters counting as having every ability. -/
theorem structAbilities_of_accepted {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) :
    ∀ d ∈ m.structDefs.toList, ∃ sh, m.structDefHandle d = some sh ∧
      ∀ f ∈ CompiledModule.StructFieldInformation.allFields d.fieldInformation,
        sh.abilities.subset (f.signature.abilitiesIn m
          (sh.typeParameters.map fun _ => AbilitySet.all)).forContainer = true := by
  intro d hd
  obtain ⟨sh, hsh, hfields⟩ :=
    verifyStructDef_ok (verifySignatures_structDefs (verifyModule_signatures h) d hd)
  refine ⟨sh, hsh, fun f hf => ?_⟩
  obtain ⟨-, hall, -⟩ := hfields f hf
  obtain ⟨cs, hcs, hc⟩ := checkTyInContext_ok hall
  refine requires_forContainer (checkTy_abilitiesIn (fun i => ?_) f.signature hcs hc)
  simp only [List.getD_eq_getElem?_getD, List.getElem?_map]
  by_cases hi : i < sh.typeParameters.length
  · simp [hi, SignatureV2.allSet, AbilitySet.all]
  · simp [hi, SignatureV2.emptySet, AbilitySet.empty]


/-! ## L6: acyclic containment -/

theorem Plus.head {r : Nat → Nat → Prop} {a b c : Nat} (hab : r a b) (h : Plus r b c) :
    Plus r a c := by
  induction h with
  | single hbc => exact Plus.tail (Plus.single hab) hbc
  | tail _ hcd ih => exact Plus.tail ih hcd

theorem Plus.first {r : Nat → Nat → Prop} {a c : Nat} (h : Plus r a c) : ∃ z, r a z := by
  induction h with
  | single h => exact ⟨_, h⟩
  | tail _ _ ih => exact ih

theorem Plus.last {r : Nat → Nat → Prop} {a c : Nat} (h : Plus r a c) :
    ∃ y, r y c ∧ (a = y ∨ Plus r a y) := by
  cases h with
  | single h => exact ⟨a, h, Or.inl rfl⟩
  | tail h1 h2 => exact ⟨_, h2, Or.inr h1⟩

theorem Plus.mono {r s : Nat → Nat → Prop} (hrs : ∀ a b, r a b → s a b) {a c : Nat}
    (h : Plus r a c) : Plus s a c := by
  induction h with
  | single h => exact Plus.single (hrs _ _ h)
  | tail _ h2 ih => exact Plus.tail ih (hrs _ _ h2)

/-- An edge of the model's containment graph. -/
def GraphEdge (g : List (Nat × List Nat)) (a b : Nat) : Prop :=
  ∃ succs, (a, succs) ∈ g ∧ b ∈ succs

/-- Removing the nodes no remaining node points to never removes a node of a
cycle: its predecessor on the cycle remains and points to it. -/
theorem hasCycle_go_true {g0 : List (Nat × List Nat)} (S : Nat → Prop)
    (hpred : ∀ x, S x → ∃ y succs, S y ∧ (y, succs) ∈ g0 ∧ x ∈ succs)
    (hkey : ∀ x, S x → ∃ succs, (x, succs) ∈ g0) {x0 : Nat} (hx0 : S x0) :
    ∀ fuel g, (∀ x succs, S x → (x, succs) ∈ g0 → (x, succs) ∈ g) →
      StructDefs.hasCycle.go fuel g = true
  | 0, g, hinv => by
    obtain ⟨succs, hs⟩ := hkey x0 hx0
    have hmem := hinv x0 succs hx0 hs
    simp only [StructDefs.hasCycle.go, Bool.not_eq_eq_eq_not, Bool.not_true,
      List.isEmpty_eq_false_iff]
    exact List.ne_nil_of_mem hmem
  | fuel + 1, g, hinv => by
    obtain ⟨succs, hs⟩ := hkey x0 hx0
    have hmem := hinv x0 succs hx0 hs
    simp only [StructDefs.hasCycle.go]
    split
    · simp only [Bool.not_eq_eq_eq_not, Bool.not_true, List.isEmpty_eq_false_iff]
      exact List.ne_nil_of_mem hmem
    · refine hasCycle_go_true S hpred hkey hx0 fuel _ fun x sx hx hxs => ?_
      simp only [List.mem_filter, List.any_eq_true]
      refine ⟨hinv x sx hx hxs, ?_⟩
      obtain ⟨y, sy, hy, hysy, hxsy⟩ := hpred x hx
      exact ⟨(y, sy), hinv y sy hy hysy, by simpa using hxsy⟩

theorem acyclic_of_hasCycle {g : List (Nat × List Nat)} (h : StructDefs.hasCycle g = false) :
    ∀ x, ¬ Plus (GraphEdge g) x x := by
  intro x hx
  have : StructDefs.hasCycle g = true := by
    unfold StructDefs.hasCycle
    refine hasCycle_go_true (g0 := g) (fun z => Plus (GraphEdge g) z z) (fun z hz => ?_)
      (fun z hz => ?_) hx _ g (fun _ _ _ h => h)
    · obtain ⟨y, ⟨succs, hys, hzs⟩, hor⟩ := Plus.last hz
      refine ⟨y, succs, ?_, hys, hzs⟩
      rcases hor with rfl | hzy
      · exact Plus.single ⟨succs, hys, hzs⟩
      · exact Plus.head ⟨succs, hys, hzs⟩ hzy
    · obtain ⟨w, succs, hs, -⟩ := Plus.first hz
      exact ⟨succs, hs⟩
  rw [h] at this
  cases this

mutual
/-- The model's edges from a type include the definition of every handle the
type may contain. -/
theorem tokenEdges_contained {m : CompiledModule} :
    ∀ (t : SignatureToken) {ra : Bool} {es : List Nat}, StructDefs.tokenEdges m t ra = .ok es →
      ∀ h ∈ t.containedHandles, ∀ d, StructDefs.handleToDef m h = some d → d ∈ es
  | .vector t, _, es, he, h, hh, d, hd => by
    simp only [StructDefs.tokenEdges] at he
    simp only [SignatureToken.containedHandles] at hh
    exact tokenEdges_contained t he h hh d hd
  | .struct h', _, es, he, h, hh, d, hd => by
    simp only [StructDefs.tokenEdges, pure, Except.pure, Except.ok.injEq] at he
    simp only [SignatureToken.containedHandles, List.mem_singleton] at hh
    subst hh he
    simp [hd]
  | .structInstantiation h' args, _, es, he, h, hh, d, hd => by
    simp only [StructDefs.tokenEdges, bind_ok, pure, Except.pure, Except.ok.injEq] at he
    obtain ⟨es', he', rfl⟩ := he
    simp only [SignatureToken.containedHandles, List.mem_cons] at hh
    rcases hh with rfl | hh
    · simp [hd]
    · exact List.mem_append_right _ (tokenEdgesList_contained args he' h hh d hd)
  | .reference t, ra, es, he, h, hh, d, hd | .mutableReference t, ra, es, he, h, hh, d, hd => by
    simp only [StructDefs.tokenEdges] at he
    simp only [SignatureToken.containedHandles] at hh
    split at he
    · exact tokenEdges_contained t he h hh d hd
    · simp [reject] at he
  | .function .., _, _, _, _, hh, _, _ | .bool, _, _, _, _, hh, _, _ | .u8, _, _, _, _, hh, _, _
  | .u16, _, _, _, _, hh, _, _ | .u32, _, _, _, _, hh, _, _ | .u64, _, _, _, _, hh, _, _
  | .u128, _, _, _, _, hh, _, _ | .u256, _, _, _, _, hh, _, _ | .i8, _, _, _, _, hh, _, _
  | .i16, _, _, _, _, hh, _, _ | .i32, _, _, _, _, hh, _, _ | .i64, _, _, _, _, hh, _, _
  | .i128, _, _, _, _, hh, _, _ | .i256, _, _, _, _, hh, _, _ | .address, _, _, _, _, hh, _, _
  | .signer, _, _, _, _, hh, _, _ | .typeParameter _, _, _, _, _, hh, _, _ => by
    simp [SignatureToken.containedHandles] at hh

theorem tokenEdgesList_contained {m : CompiledModule} :
    ∀ (ts : List SignatureToken) {ra : Bool} {es : List Nat},
      StructDefs.tokenEdgesList m ts ra = .ok es →
      ∀ h ∈ SignatureToken.containedHandlesList ts, ∀ d, StructDefs.handleToDef m h = some d →
        d ∈ es
  | [], _, _, _, h, hh, _, _ => by simp [SignatureToken.containedHandlesList] at hh
  | t :: ts, _, es, he, h, hh, d, hd => by
    simp only [StructDefs.tokenEdgesList, bind_ok, pure, Except.pure, Except.ok.injEq] at he
    obtain ⟨e1, h1, e2, h2, rfl⟩ := he
    simp only [SignatureToken.containedHandlesList, List.mem_append] at hh
    rcases hh with hh | hh
    · exact List.mem_append_left _ (tokenEdges_contained t h1 h hh d hd)
    · exact List.mem_append_right _ (tokenEdgesList_contained ts h2 h hh d hd)
end

theorem tokenEdgesList_mem {m : CompiledModule} :
    ∀ {ts : List SignatureToken} {ra : Bool} {es : List Nat},
      StructDefs.tokenEdgesList m ts ra = .ok es → ∀ t ∈ ts,
        ∃ es', StructDefs.tokenEdges m t ra = .ok es' ∧ ∀ d ∈ es', d ∈ es
  | [], _, _, _, t, ht => by simp at ht
  | x :: xs, _, es, he, t, ht => by
    simp only [StructDefs.tokenEdgesList, bind_ok, pure, Except.pure, Except.ok.injEq] at he
    obtain ⟨e1, h1, e2, h2, rfl⟩ := he
    rcases List.mem_cons.1 ht with rfl | ht
    · exact ⟨e1, h1, fun d hd => List.mem_append_left _ hd⟩
    · obtain ⟨es', h', hsub⟩ := tokenEdgesList_mem h2 t ht
      exact ⟨es', h', fun d hd => List.mem_append_right _ (hsub d hd)⟩

/-- With no two definitions of one handle, `handleToDef` finds a definition's
own index. -/
theorem handleToDef_eq {m : CompiledModule} (hnd : (m.structDefs.toList.map (·.structHandle)).Nodup)
    {d : Nat} {sd : StructDefinition} (hd : m.structDefs[d]? = some sd) :
    StructDefs.handleToDef m sd.structHandle = some d := by
  unfold StructDefs.handleToDef
  generalize hl : m.structDefs.toList.zipIdx.filter (·.1.structHandle == sd.structHandle) = l
  have hall : ∀ e ∈ l, e.2 = d := by
    intro e he
    rw [← hl, List.mem_filter, List.mem_zipIdx_iff_getElem?] at he
    obtain ⟨he, hh⟩ := he
    simp only [beq_iff_eq] at hh
    exact nodup_map_index hnd he (by rw [toList_getElem?]; exact hd) hh
  have hmem : (sd, d) ∈ l := by
    rw [← hl, List.mem_filter, List.mem_zipIdx_iff_getElem?]
    exact ⟨by rw [toList_getElem?]; exact hd, by simp⟩
  cases hlast : l.getLast? with
  | none => simp at hlast; subst hlast; simp at hmem
  | some e => simp [hall e (List.mem_of_getLast? hlast)]

/-- The containment part of **L6**. -/
theorem localStructsAcyclic_of_accepted {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) : m.LocalStructsAcyclic := by
  have hsu := verifyDuplication_structs (verifyModule_ok h).2.1
  have hsd := verifyModule_structDefs h
  unfold verifyStructDefs at hsd
  simp only [bind_ok] at hsd
  obtain ⟨g, hg, hcyc⟩ := hsd
  have hno : StructDefs.hasCycle g = false := by
    cases hc : StructDefs.hasCycle g
    · rfl
    · simp [hc, reject] at hcyc
  have hedge : ∀ a b, m.FieldContains a b → GraphEdge g a b := by
    rintro a b ⟨sd₁, sd₂, h₁, h₂, f, hf, hc⟩
    unfold StructDefs.edges at hg
    obtain ⟨y, hy, hfy⟩ := mapM_ok_of_mem hg (sd₁, a) (by
      rw [List.mem_zipIdx_iff_getElem?, toList_getElem?]; exact h₁)
    simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at hfy
    obtain ⟨succs, hsuccs, rfl⟩ := hfy
    refine ⟨succs, hy, ?_⟩
    have hf' : f ∈ StructDefs.fields sd₁ := by
      unfold StructDefs.fields
      cases hfi : sd₁.fieldInformation <;>
        simp_all [CompiledModule.StructFieldInformation.allFields]
    obtain ⟨es, hes, hsub⟩ := tokenEdgesList_mem hsuccs f.signature (List.mem_map_of_mem hf')
    exact hsub b (tokenEdges_contained f.signature hes _ hc b (handleToDef_eq hsu.defs h₂))
  intro d hd
  exact acyclic_of_hasCycle hno d (Plus.mono hedge hd)


/-! ## L6: variant field borrows -/

/-! ## L6: phantom positions -/

mutual
/-- Whether a type mentions a reference, at any depth. -/
def hasRef : SignatureToken → Bool
  | .reference _ | .mutableReference _ => true
  | .vector e => hasRef e
  | .function ps rs _ => hasRefList ps || hasRefList rs
  | .structInstantiation _ args => hasRefList args
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 | .i8 | .i16 | .i32 | .i64 | .i128
  | .i256 | .address | .signer | .struct _ | .typeParameter _ => false

def hasRefList : List SignatureToken → Bool
  | [] => false
  | t :: ts => hasRef t || hasRefList ts
end

mutual
/-- The signature checker's phantom check gives the phantom-position rule for
a type, when it looks through references (`refs`) or the type mentions no
reference. -/
theorem checkPhantom_respected {m : CompiledModule} {ctx : List StructTypeParameter}
    {refs : Bool} :
    ∀ (t : SignatureToken), SignatureV2.checkPhantom m ctx refs t false = .ok () →
      refs = true ∨ hasRef t = false → t.PhantomRespected m ctx
  | .vector e, h, hr => by
    simp only [SignatureV2.checkPhantom] at h
    simp only [SignatureToken.PhantomRespected]
    exact checkPhantom_respected e h (hr.imp_right fun h => by simpa [hasRef] using h)
  | .function ps rs _, h, hr => by
    simp only [SignatureV2.checkPhantom] at h
    obtain ⟨h1, h2⟩ := seq_ok2 h
    simp only [SignatureToken.PhantomRespected]
    exact ⟨checkPhantomList_respected ps h1 (hr.imp_right fun h => by
        simp only [hasRef, Bool.or_eq_false_iff] at h; exact h.1),
      checkPhantomList_respected rs h2 (hr.imp_right fun h => by
        simp only [hasRef, Bool.or_eq_false_iff] at h; exact h.2)⟩
  | .structInstantiation hh args, h, hr => by
    simp only [SignatureV2.checkPhantom, bind_ok, lookup_ok] at h
    obtain ⟨sh, hsh, h⟩ := h
    have hargs := checkPhantomArgs_respected sh.typeParameters args 0 h
      (hr.imp_right fun h => by simpa [hasRef] using h)
    simp only [SignatureToken.PhantomRespected]
    exact ⟨sh, hsh, by simpa using hargs⟩
  | .typeParameter i, h, _ => by
    simp only [SignatureV2.checkPhantom, bind_ok, lookup_ok] at h
    obtain ⟨p, hp, h⟩ := h
    simp only [SignatureToken.PhantomRespected]
    refine ⟨p, by simpa using hp, ?_⟩
    split at h
    · simp [reject] at h
    · next hc => simpa using hc
  | .reference t, h, hr | .mutableReference t, h, hr => by
    rcases hr with hr | hr
    · simp only [SignatureV2.checkPhantom] at h
      rw [if_pos hr] at h
      simp only [SignatureToken.PhantomRespected]
      exact checkPhantom_respected t h (Or.inl hr)
    · simp [hasRef] at hr
  | .bool, _, _ | .u8, _, _ | .u16, _, _ | .u32, _, _ | .u64, _, _ | .u128, _, _
  | .u256, _, _ | .i8, _, _ | .i16, _, _ | .i32, _, _ | .i64, _, _ | .i128, _, _
  | .i256, _, _ | .address, _, _ | .signer, _, _ | .struct _, _, _ => by
    simp [SignatureToken.PhantomRespected]

theorem checkPhantomList_respected {m : CompiledModule} {ctx : List StructTypeParameter}
    {refs : Bool} :
    ∀ (ts : List SignatureToken), SignatureV2.checkPhantomList m ctx refs ts = .ok () →
      refs = true ∨ hasRefList ts = false → SignatureToken.PhantomRespectedAll m ctx ts
  | [], _, _ => by simp [SignatureToken.PhantomRespectedAll]
  | t :: ts, h, hr => by
    simp only [SignatureV2.checkPhantomList] at h
    obtain ⟨h1, h2⟩ := seq_ok2 h
    simp only [SignatureToken.PhantomRespectedAll]
    exact ⟨checkPhantom_respected t h1 (hr.imp_right fun h => by
        simp only [hasRefList, Bool.or_eq_false_iff] at h; exact h.1),
      checkPhantomList_respected ts h2 (hr.imp_right fun h => by
        simp only [hasRefList, Bool.or_eq_false_iff] at h; exact h.2)⟩

theorem checkPhantomArgs_respected {m : CompiledModule} {ctx : List StructTypeParameter}
    {refs : Bool} :
    ∀ (params : List StructTypeParameter) (args : List SignatureToken) (i : Nat),
      SignatureV2.checkPhantomArgs m ctx refs params args i = .ok () →
      refs = true ∨ hasRefList args = false →
      SignatureToken.PhantomRespectedArgs m ctx args (params.drop i)
  | _, [], _, _, _ => by simp [SignatureToken.PhantomRespectedArgs]
  | params, a :: as, i, h, hr => by
    simp only [SignatureV2.checkPhantomArgs, bind_ok, lookup_ok] at h
    obtain ⟨p, hp, u, h1, h2⟩ := h
    cases u
    have hp' : params[i]? = some p := by simpa using hp
    have hdrop : params.drop i = p :: params.drop (i + 1) := by
      obtain ⟨hlt, heq⟩ := List.getElem?_eq_some_iff.1 hp'
      rw [List.drop_eq_getElem_cons hlt, heq]
    rw [hdrop]
    simp only [SignatureToken.PhantomRespectedArgs]
    refine ⟨?_, checkPhantomArgs_respected params as (i + 1) h2 (hr.imp_right fun h => by
      simp only [hasRefList, Bool.or_eq_false_iff] at h; exact h.2)⟩
    cases hph : p.isPhantom
    · exact Or.inr (checkPhantom_respected a (by simpa [hph] using h1) (hr.imp_right fun h => by
        simp only [hasRefList, Bool.or_eq_false_iff] at h; exact h.1))
    · exact Or.inl rfl
end

/-- The phantom-position rule of L6 for the field types that mention a
reference, which the model's acceptance does not establish (triage
2026-10-02). **L6** is proved from it as a hypothesis. -/
def PhantomUnderReferences : Prop :=
  ∀ m, verifyModule VerifierConfig.production m = .ok () →
    ∀ d ∈ m.structDefs.toList, ∀ sh, m.structDefHandle d = some sh →
      ∀ f ∈ CompiledModule.StructFieldInformation.allFields d.fieldInformation,
        hasRef f.signature = true → f.signature.PhantomRespected m sh.typeParameters

/-- When the model's phantom check looks through references, the phantom
rule holds for every field type, references included. This is the model of
the expected fix of `check_phantom_params`; enabling it in the production
configuration discharges `PhantomUnderReferences`. -/
theorem phantomUnderReferences_of_switch
    (hsw : SignatureV2.phantomThroughReferences VerifierConfig.production = true) :
    PhantomUnderReferences := by
  intro m h d hd sh hsh f hf _
  obtain ⟨sh', hsh', hfields⟩ :=
    verifyStructDef_ok (verifySignatures_structDefs (verifyModule_signatures h) d hd)
  have hsh'' : m.structHandles[d.structHandle]? = some sh := hsh
  rw [hsh'', Option.some.injEq] at hsh'
  subst hsh'
  exact checkPhantom_respected f.signature (hfields f hf).2.2 (Or.inl hsw)

end MoveBytecode.Proofs
