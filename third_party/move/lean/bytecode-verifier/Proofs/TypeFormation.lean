-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.StackTokens

/-!
# Type formation (L5) from the model

UNTRUSTED (checked by the kernel). For a module the model accepts, every type
it writes down is well formed (`CompiledModule.TypeFormation`):

- The signature checker gives the shape: references only at the top of a
  slot (a parameter, return or local type, or a function type's parameter or
  result), type arguments one per type parameter, each with the abilities its
  parameter's constraints require (`checkTy_abilitiesIn`), function types
  without `key`. The production configuration checks function types'
  parameters, results and abilities.
- The bounds checker gives the rest: a struct named without type arguments
  has none, and type parameters are in scope (through L1).
- Constant types are primitives and vectors of them (the constants pass).
- In a module the model accepts, every instruction is in the supported
  subset, where only the vector instructions have a type operand.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Bounds facts about a token -/

/-- Every struct the token names without type arguments has no type
parameters, as the bounds checker requires. -/
def Arity (m : CompiledModule) (t : SignatureToken) : Prop :=
  ∀ i sh, SignatureToken.struct i ∈ preorder t → m.structHandles[i]? = some sh →
    sh.typeParameters = []

/-- What the bounds checker gives about a token in a scope of `n` type
parameters. -/
def Good (m : CompiledModule) (n : Nat) (t : SignatureToken) : Prop :=
  m.TokenInRange n t ∧ Arity m t

theorem arity_of_checkType {m : CompiledModule} {t : SignatureToken}
    (h : Bounds.checkType m t = .ok ()) : Arity m t := by
  intro i sh hi hsh
  have hb := forM_ok h _ hi
  obtain ⟨-, hb⟩ := seq_ok2 hb
  simp only [hsh] at hb
  split at hb
  · simp [reject] at hb
  · next hne => simpa using hne

theorem refs_sub_refsList {t : SignatureToken} :
    ∀ {ts : List SignatureToken}, t ∈ ts → ∀ r ∈ t.refs, r ∈ SignatureToken.refsList ts
  | [], ht => by simp at ht
  | x :: xs, ht => by
    intro r hr
    simp only [SignatureToken.refsList, List.mem_append]
    rcases List.mem_cons.1 ht with rfl | ht
    · exact Or.inl hr
    · exact Or.inr (refs_sub_refsList ht r hr)

section Good

variable {m : CompiledModule} {n : Nat}

theorem Good.vector {e : SignatureToken} (h : Good m n (.vector e)) : Good m n e :=
  ⟨fun r hr => h.1 r (by simpa [SignatureToken.refs] using hr),
    fun i sh hi hsh => h.2 i sh (by simp [preorder, hi]) hsh⟩

theorem Good.reference {e : SignatureToken} (h : Good m n (.reference e)) : Good m n e :=
  ⟨fun r hr => h.1 r (by simpa [SignatureToken.refs] using hr),
    fun i sh hi hsh => h.2 i sh (by simp [preorder, hi]) hsh⟩

theorem Good.mutableReference {e : SignatureToken} (h : Good m n (.mutableReference e)) :
    Good m n e :=
  ⟨fun r hr => h.1 r (by simpa [SignatureToken.refs] using hr),
    fun i sh hi hsh => h.2 i sh (by simp [preorder, hi]) hsh⟩

theorem Good.params {ps rs : List SignatureToken} {a : AbilitySet}
    (h : Good m n (.function ps rs a)) : ∀ t ∈ ps, Good m n t :=
  fun t ht => ⟨fun r hr => h.1 r (by
      simp only [SignatureToken.refs, List.mem_append]
      exact Or.inl (refs_sub_refsList ht r hr)),
    fun i sh hi hsh => h.2 i sh (by
      simp only [preorder, List.mem_cons, List.mem_append]
      exact Or.inr (Or.inl (mem_preorderList.2 ⟨t, ht, hi⟩))) hsh⟩

theorem Good.results {ps rs : List SignatureToken} {a : AbilitySet}
    (h : Good m n (.function ps rs a)) : ∀ t ∈ rs, Good m n t :=
  fun t ht => ⟨fun r hr => h.1 r (by
      simp only [SignatureToken.refs, List.mem_append]
      exact Or.inr (refs_sub_refsList ht r hr)),
    fun i sh hi hsh => h.2 i sh (by
      simp only [preorder, List.mem_cons, List.mem_append]
      exact Or.inr (Or.inr (mem_preorderList.2 ⟨t, ht, hi⟩))) hsh⟩

theorem Good.args {h : StructHandleIndex} {args : List SignatureToken}
    (hg : Good m n (.structInstantiation h args)) : ∀ t ∈ args, Good m n t :=
  fun t ht => ⟨fun r hr => hg.1 r (by
      simp only [SignatureToken.refs, List.mem_cons]
      exact Or.inr (refs_sub_refsList ht r hr)),
    fun i sh hi hsh => hg.2 i sh (by
      simp only [preorder, List.mem_cons]
      exact Or.inr (mem_preorderList.2 ⟨t, ht, hi⟩)) hsh⟩

end Good

/-! ## Slots -/

theorem slots_cons {m : CompiledModule} {tparams : List AbilitySet} {t : SignatureToken}
    {ts : List SignatureToken} (h1 : SignatureToken.SlotsWellFormed m tparams [t])
    (h2 : SignatureToken.SlotsWellFormed m tparams ts) :
    SignatureToken.SlotsWellFormed m tparams (t :: ts) := by
  cases t <;> simp_all [SignatureToken.SlotsWellFormed]

theorem slots_single {m : CompiledModule} {tparams : List AbilitySet} {t : SignatureToken}
    (hr : SignatureV2.isReference t = false) (h : t.ValueTypeWellFormed m tparams) :
    SignatureToken.SlotsWellFormed m tparams [t] := by
  cases t <;> simp_all [SignatureToken.SlotsWellFormed, SignatureV2.isReference]

theorem valueType_of_slots {m : CompiledModule} {tparams : List AbilitySet} {t : SignatureToken}
    (hr : SignatureV2.isReference t = false) (h : SignatureToken.SlotsWellFormed m tparams [t]) :
    t.ValueTypeWellFormed m tparams := by
  cases t <;> simp_all [SignatureToken.SlotsWellFormed, SignatureV2.isReference]

theorem checkTy_noRef {config : VerifierConfig} {m : CompiledModule} {t : SignatureToken}
    {req : AbilitySet} {cs : SignatureV2.Constraints}
    (h : SignatureV2.checkTy config m t false req = .ok cs) : SignatureV2.isReference t = false := by
  cases t <;> first | rfl | simp [SignatureV2.checkTy, reject] at h

/-! ## What the signature checker establishes about a type -/

section CheckTy

variable {config : VerifierConfig} {m : CompiledModule} {tparams : List AbilitySet}
  {ctx : Nat → AbilitySet}

mutual
/-- A type the signature checker accepts, whose collected constraints hold in
the context, and which the bounds checker accepted, is well formed as a slot,
and as a value type unless it is a reference. -/
theorem checkTy_wf (hfix : config.sigCheckerV2FixFunctionSignatures = true)
    (hfab : config.checkFunctionTypeAbilities = true)
    (hctx : ∀ i, ctx i = tparams.getD i AbilitySet.empty) :
    ∀ (t : SignatureToken) {allowRef : Bool} {req : AbilitySet} {cs : SignatureV2.Constraints},
      SignatureV2.checkTy config m t allowRef req = .ok cs → ConstraintsHold ctx cs →
      Good m tparams.length t →
      SignatureToken.SlotsWellFormed m tparams [t] ∧
        (SignatureV2.isReference t = false → t.ValueTypeWellFormed m tparams)
  | .typeParameter i, _, req, cs, h, hc, hg => by
    have hi := hg.1 (.typeParameter i) (by simp [SignatureToken.refs])
    have hv : (SignatureToken.typeParameter i).ValueTypeWellFormed m tparams := by
      simpa [SignatureToken.ValueTypeWellFormed] using hi
    exact ⟨slots_single rfl hv, fun _ => hv⟩
  | .vector t, _, req, cs, h, hc, hg => by
    simp only [SignatureV2.checkTy, bind_ok] at h
    obtain ⟨_, -, h⟩ := h
    have ih := (checkTy_wf hfix hfab hctx t h hc hg.vector).2 (checkTy_noRef h)
    have hv : (SignatureToken.vector t).ValueTypeWellFormed m tparams := by
      simpa [SignatureToken.ValueTypeWellFormed] using ih
    exact ⟨slots_single rfl hv, fun _ => hv⟩
  | .function ps rs ab, _, req, cs, h, hc, hg => by
    simp only [SignatureV2.checkTy, bind_ok] at h
    obtain ⟨_, -, h⟩ := h
    split at h
    · simp [reject, bind, Except.bind] at h
    · next hab =>
      simp only [hfab, Bool.true_and, Bool.not_eq_true', Bool.not_eq_false] at hab
      simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at h
      obtain ⟨c1, h1, c2, h2, rfl⟩ := h
      obtain ⟨hc1, hc2⟩ := hc.append
      have hp := checkTyList_wf hfix hfab hctx ps h1 hc1 hg.params
      have hr := checkTyList_wf hfix hfab hctx rs h2 hc2 hg.results
      have hk : ab.key = false := by
        cases ab
        simp_all [MoveBytecode.AbilitySet.subset, publicFunctionAbilities]
      have hv : (SignatureToken.function ps rs ab).ValueTypeWellFormed m tparams := by
        simp only [SignatureToken.ValueTypeWellFormed]
        exact ⟨hp, hr, hk⟩
      exact ⟨slots_single rfl hv, fun _ => hv⟩
  | .struct h, _, req, cs, hc', hc, hg => by
    simp only [SignatureV2.checkTy, bind_ok, lookup_ok] at hc'
    obtain ⟨sh, hsh, -⟩ := hc'
    have hnp := hg.2 h sh (by simp [preorder]) hsh
    have hv : (SignatureToken.struct h).ValueTypeWellFormed m tparams := by
      simp only [SignatureToken.ValueTypeWellFormed]
      exact ⟨sh, hsh, hnp⟩
    exact ⟨slots_single rfl hv, fun _ => hv⟩
  | .structInstantiation h args, _, req, cs, hc', hc, hg => by
    simp only [SignatureV2.checkTy, bind_ok, lookup_ok] at hc'
    obtain ⟨sh, hsh, _, -, hargs⟩ := hc'
    split at hargs
    · simp [reject, bind, Except.bind] at hargs
    · next hlen =>
      simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hlen
      have ha := checkTyArgs_wf hfix hfab hctx sh.typeParameters args hargs hc hg.args hlen.symm
      have hv : (SignatureToken.structInstantiation h args).ValueTypeWellFormed m tparams := by
        simp only [SignatureToken.ValueTypeWellFormed]
        exact ⟨sh, hsh, ha⟩
      exact ⟨slots_single rfl hv, fun _ => hv⟩
  | .reference t, allowRef, req, cs, h, hc, hg => by
    simp only [SignatureV2.checkTy] at h
    split at h
    · simp only [bind_ok] at h
      obtain ⟨_, -, h⟩ := h
      have ih := (checkTy_wf hfix hfab hctx t h hc hg.reference).2 (checkTy_noRef h)
      exact ⟨by simp [SignatureToken.SlotsWellFormed, ih], by simp [SignatureV2.isReference]⟩
    · simp [reject] at h
  | .mutableReference t, allowRef, req, cs, h, hc, hg => by
    simp only [SignatureV2.checkTy] at h
    split at h
    · simp only [bind_ok] at h
      obtain ⟨_, -, h⟩ := h
      have ih := (checkTy_wf hfix hfab hctx t h hc hg.mutableReference).2 (checkTy_noRef h)
      exact ⟨by simp [SignatureToken.SlotsWellFormed, ih], by simp [SignatureV2.isReference]⟩
    · simp [reject] at h
  | .bool, _, _, _, _, _, _ | .u8, _, _, _, _, _, _ | .u16, _, _, _, _, _, _
  | .u32, _, _, _, _, _, _ | .u64, _, _, _, _, _, _ | .u128, _, _, _, _, _, _
  | .u256, _, _, _, _, _, _ | .i8, _, _, _, _, _, _ | .i16, _, _, _, _, _, _
  | .i32, _, _, _, _, _, _ | .i64, _, _, _, _, _, _ | .i128, _, _, _, _, _, _
  | .i256, _, _, _, _, _, _ | .address, _, _, _, _, _, _ | .signer, _, _, _, _, _, _ => by
    exact ⟨by simp [SignatureToken.SlotsWellFormed, SignatureToken.ValueTypeWellFormed],
      fun _ => by simp [SignatureToken.ValueTypeWellFormed]⟩

/-- A list of types the signature checker accepts is well formed as slots. -/
theorem checkTyList_wf (hfix : config.sigCheckerV2FixFunctionSignatures = true)
    (hfab : config.checkFunctionTypeAbilities = true)
    (hctx : ∀ i, ctx i = tparams.getD i AbilitySet.empty) :
    ∀ (ts : List SignatureToken) {allowRef : Bool} {req : AbilitySet}
      {cs : SignatureV2.Constraints},
      SignatureV2.checkTyList config m ts allowRef req = .ok cs → ConstraintsHold ctx cs →
      (∀ t ∈ ts, Good m tparams.length t) → SignatureToken.SlotsWellFormed m tparams ts
  | [], _, _, _, _, _, _ => by simp [SignatureToken.SlotsWellFormed]
  | t :: ts, _, _, cs, h, hc, hg => by
    simp only [SignatureV2.checkTyList, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨c1, h1, c2, h2, rfl⟩ := h
    obtain ⟨hc1, hc2⟩ := hc.append
    exact slots_cons (checkTy_wf hfix hfab hctx t h1 hc1 (hg t (List.mem_cons_self ..))).1
      (checkTyList_wf hfix hfab hctx ts h2 hc2 (fun u hu => hg u (List.mem_cons_of_mem _ hu)))

/-- The type arguments of a struct instantiation the signature checker
accepts instantiate its type parameters. -/
theorem checkTyArgs_wf (hfix : config.sigCheckerV2FixFunctionSignatures = true)
    (hfab : config.checkFunctionTypeAbilities = true)
    (hctx : ∀ i, ctx i = tparams.getD i AbilitySet.empty) :
    ∀ (ps : List StructTypeParameter) (args : List SignatureToken) {req : AbilitySet}
      {cs : SignatureV2.Constraints},
      SignatureV2.checkTyArgs config m ps args req = .ok cs → ConstraintsHold ctx cs →
      (∀ t ∈ args, Good m tparams.length t) → args.length = ps.length →
      SignatureToken.ArgumentsWellFormed m tparams args (ps.map (·.constraints))
  | p :: ps, a :: as, req, cs, h, hc, hg, hlen => by
    simp only [SignatureV2.checkTyArgs, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨c1, h1, c2, h2, rfl⟩ := h
    obtain ⟨hc1, hc2⟩ := hc.append
    have hv := (checkTy_wf hfix hfab hctx a h1 hc1 (hg a (List.mem_cons_self ..))).2
      (checkTy_noRef h1)
    have hab := checkTy_abilitiesIn hctx a h1 hc1
    have hcon : p.constraints.subset (a.abilitiesIn m tparams) = true := by
      split at hab
      · exact hab
      · exact subset_of_union_left hab
    simp only [List.map_cons, SignatureToken.ArgumentsWellFormed]
    exact ⟨hv, hcon, checkTyArgs_wf hfix hfab hctx ps as h2 hc2
      (fun u hu => hg u (List.mem_cons_of_mem _ hu)) (by simpa using hlen)⟩
  | [], [], _, _, _, _, _, _ => by simp [SignatureToken.ArgumentsWellFormed]
  | [], _ :: _, _, _, _, _, _, hlen => by simp at hlen
  | _ :: _, [], _, _, _, _, _, hlen => by simp at hlen
end

/-- A type argument the signature checker accepts is not a reference, and
checks as a type. -/
theorem typeArg_ok {ty : SignatureToken} {req : AbilitySet} {c : SignatureV2.Constraints}
    (h : SignatureV2.typeArg config m ty req = .ok c) :
    SignatureV2.checkTy config m ty false req = .ok c := by
  unfold SignatureV2.typeArg at h
  split at h
  · simp [reject, bind, Except.bind] at h
  · simpa using h

/-- The type arguments of a function instantiation the signature checker
accepts, in a context where their collected constraints hold, instantiate the
callee's type parameters. -/
theorem functionArgs_wf (hfix : config.sigCheckerV2FixFunctionSignatures = true)
    (hfab : config.checkFunctionTypeAbilities = true)
    (hctx : ∀ i, ctx i = tparams.getD i AbilitySet.empty) :
    ∀ (args : List SignatureToken) (ps : List AbilitySet) {css : List SignatureV2.Constraints},
      (args.zip ps).mapM (fun (x : SignatureToken × AbilitySet) =>
        SignatureV2.typeArg config m x.1 x.2) = .ok css →
      ConstraintsHold ctx css.flatten → (∀ t ∈ args, Good m tparams.length t) →
      args.length = ps.length → SignatureToken.ArgumentsWellFormed m tparams args ps
  | a :: as, p :: ps, css, h, hc, hg, hlen => by
    simp only [List.zip_cons_cons, List.mapM_cons, bind_ok, pure, Except.pure,
      Except.ok.injEq] at h
    obtain ⟨c1, h1, c2, h2, rfl⟩ := h
    simp only [List.flatten_cons] at hc
    obtain ⟨hc1, hc2⟩ := hc.append
    have h1 := typeArg_ok h1
    simp only [SignatureToken.ArgumentsWellFormed]
    exact ⟨(checkTy_wf hfix hfab hctx a h1 hc1 (hg a (List.mem_cons_self ..))).2
        (checkTy_noRef h1), checkTy_abilitiesIn hctx a h1 hc1,
      functionArgs_wf hfix hfab hctx as ps h2 hc2
        (fun u hu => hg u (List.mem_cons_of_mem _ hu)) (by simpa using hlen)⟩
  | [], [], _, _, _, _, _ => by simp [SignatureToken.ArgumentsWellFormed]
  | [], _ :: _, _, _, _, _, hlen => by simp at hlen
  | _ :: _, [], _, _, _, _, hlen => by simp at hlen

/-- The type arguments of a struct instantiation the signature checker
accepts, in a context where their collected constraints hold, instantiate the
struct's type parameters. -/
theorem structArgs_wf (hfix : config.sigCheckerV2FixFunctionSignatures = true)
    (hfab : config.checkFunctionTypeAbilities = true)
    (hctx : ∀ i, ctx i = tparams.getD i AbilitySet.empty) {req : AbilitySet} :
    ∀ (args : List SignatureToken) (ps : List StructTypeParameter)
      {css : List SignatureV2.Constraints},
      (args.zip ps).mapM (fun (x : SignatureToken × StructTypeParameter) =>
        SignatureV2.typeArg config m x.1 (if x.2.isPhantom then x.2.constraints
          else x.2.constraints.union (Model.requires req))) = .ok css →
      ConstraintsHold ctx css.flatten → (∀ t ∈ args, Good m tparams.length t) →
      args.length = ps.length →
      SignatureToken.ArgumentsWellFormed m tparams args (ps.map (·.constraints))
  | a :: as, p :: ps, css, h, hc, hg, hlen => by
    simp only [List.zip_cons_cons, List.mapM_cons, bind_ok, pure, Except.pure,
      Except.ok.injEq] at h
    obtain ⟨c1, h1, c2, h2, rfl⟩ := h
    simp only [List.flatten_cons] at hc
    obtain ⟨hc1, hc2⟩ := hc.append
    have h1 := typeArg_ok h1
    have hab := checkTy_abilitiesIn hctx a h1 hc1
    have hcon : p.constraints.subset (a.abilitiesIn m tparams) = true := by
      split at hab
      · exact hab
      · exact subset_of_union_left hab
    simp only [List.map_cons, SignatureToken.ArgumentsWellFormed]
    exact ⟨(checkTy_wf hfix hfab hctx a h1 hc1 (hg a (List.mem_cons_self ..))).2
        (checkTy_noRef h1), hcon,
      structArgs_wf hfix hfab hctx as ps h2 hc2
        (fun u hu => hg u (List.mem_cons_of_mem _ hu)) (by simpa using hlen)⟩
  | [], [], _, _, _, _, _ => by simp [SignatureToken.ArgumentsWellFormed]
  | [], _ :: _, _, _, _, _, hlen => by simp at hlen
  | _ :: _, [], _, _, _, _, hlen => by simp at hlen

end CheckTy

/-- The tokens of a signature whose type parameters are in scope, which the
bounds checker accepted. -/
theorem good_of_inRange {m : CompiledModule} {n a : Nat} {args : List SignatureToken}
    (hr : m.SignatureInRange n a) (hargs : m.signatures[a]? = some args)
    (hsigs : ∀ sig ∈ m.signatures.toList, ∀ t ∈ sig, Bounds.checkType m t = .ok ()) :
    ∀ t ∈ args, Good m n t := by
  obtain ⟨sig, hsig, hin⟩ := hr
  rw [hargs, Option.some.injEq] at hsig
  subst hsig
  exact fun t ht => ⟨hin t ht, arity_of_checkType (hsigs _ (getElem?_mem_toList hargs) t ht)⟩

/-! ## The passes, position by position -/

theorem listContext_eq (l : List AbilitySet) (i : Nat) :
    SignatureV2.listContext l i = l.getD i AbilitySet.empty := by
  simp only [SignatureV2.listContext, List.getD_eq_getElem?_getD]
  rfl

theorem signatureConstraints_ok {config : VerifierConfig} {m : CompiledModule} {s : Nat}
    {cs : SignatureV2.Constraints} (h : SignatureV2.signatureConstraints config m s = .ok cs) :
    ∃ sig, m.signatures[s]? = some sig ∧
      SignatureV2.checkTyList config m sig true SignatureV2.emptySet = .ok cs := by
  unfold SignatureV2.signatureConstraints at h
  simp only [bind_ok, lookup_ok] at h
  exact h

/-- A signature the signature checker accepts in a context, whose tokens the
bounds checker accepted, is well formed as a list of slots. -/
theorem slotSignature_of_checked {config : VerifierConfig} {m : CompiledModule}
    (hfix : config.sigCheckerV2FixFunctionSignatures = true)
    (hfab : config.checkFunctionTypeAbilities = true) {tparams : List AbilitySet} {s : Nat}
    {cs : SignatureV2.Constraints} (hsc : SignatureV2.signatureConstraints config m s = .ok cs)
    (hctx : SignatureV2.checkInContext cs (SignatureV2.listContext tparams) = .ok ())
    (hr : m.SignatureInRange tparams.length s)
    (hsigs : ∀ sig ∈ m.signatures.toList, ∀ t ∈ sig, Bounds.checkType m t = .ok ()) :
    ∃ sig, m.signatures[s]? = some sig ∧ SignatureToken.SlotsWellFormed m tparams sig := by
  obtain ⟨sig, hsig, hl⟩ := signatureConstraints_ok hsc
  obtain ⟨sig', hsig', hin⟩ := hr
  rw [hsig, Option.some.injEq] at hsig'
  subst hsig'
  exact ⟨sig, hsig, checkTyList_wf hfix hfab (listContext_eq tparams) sig hl
    (checkInContext_ok hctx) fun t ht =>
      ⟨hin t ht, arity_of_checkType (hsigs sig (getElem?_mem_toList hsig) t ht)⟩⟩

theorem verifySignatures_handles {config : VerifierConfig} {m : CompiledModule}
    (h : verifySignatures config m = .ok ()) :
    ∀ fh ∈ m.functionHandles.toList,
      (∃ cs, SignatureV2.signatureConstraints config m fh.parameters = .ok cs ∧
        SignatureV2.checkInContext cs (SignatureV2.listContext fh.typeParameters) = .ok ()) ∧
      (∃ cs, SignatureV2.signatureConstraints config m fh.returns = .ok cs ∧
        SignatureV2.checkInContext cs (SignatureV2.listContext fh.typeParameters) = .ok ()) := by
  unfold verifySignatures SignatureV2.verify at h
  split at h
  · simp [reject, bind, Except.bind] at h
  · simp only [bind_ok] at h
    obtain ⟨-, -, -, -, -, -, -, -, -, -, -, -, u, h, -⟩ := h
    cases u
    intro fh hfh
    have := forM_ok h fh hfh
    simp only [bind_ok] at this
    obtain ⟨cs1, h1, u1, h1', cs2, h2, h2'⟩ := this
    cases u1
    exact ⟨⟨cs1, h1, h1'⟩, ⟨cs2, h2, h2'⟩⟩

/-- What the signature checker establishes about a code unit: its locals, and
the element type of each vector instruction, a single type that is not a
reference. -/
theorem verifyCode_parts {config : VerifierConfig} {m : CompiledModule}
    {abilities : List AbilitySet} {code : CodeUnit}
    (h : SignatureV2.verifyCode config m abilities code = .ok ()) :
    (∃ cs, SignatureV2.signatureConstraints config m code.locals = .ok cs ∧
      SignatureV2.checkInContext cs (SignatureV2.listContext abilities) = .ok ()) ∧
    ∀ s n, (Bytecode.vecPack s n ∈ code.code.toList ∨ Bytecode.vecUnpack s n ∈ code.code.toList ∨
      Bytecode.vecLen s ∈ code.code.toList ∨ Bytecode.vecImmBorrow s ∈ code.code.toList ∨
      Bytecode.vecMutBorrow s ∈ code.code.toList ∨ Bytecode.vecPushBack s ∈ code.code.toList ∨
      Bytecode.vecPopBack s ∈ code.code.toList ∨ Bytecode.vecSwap s ∈ code.code.toList) →
      ∃ ty cs, m.signatures[s]? = some [ty] ∧ SignatureV2.isReference ty = false ∧
        SignatureV2.signatureConstraints config m s = .ok cs ∧
        SignatureV2.checkInContext cs (SignatureV2.listContext abilities) = .ok () := by
  unfold SignatureV2.verifyCode at h
  simp only [bind_ok] at h
  obtain ⟨cs, hcs, u, hu, h⟩ := h
  cases u
  refine ⟨⟨cs, hcs, hu⟩, fun s n hi => ?_⟩
  rcases hi with hi | hi | hi | hi | hi | hi | hi | hi <;>
  · have := forM_ok h _ hi
    simp only [bind_ok, lookup_ok] at this
    obtain ⟨args, hargs, this⟩ := this
    split at this
    · next ty =>
      split at this
      · simp [reject, bind, Except.bind] at this
      · next hr =>
        simp only [bind_ok] at this
        obtain ⟨cs', hcs', this⟩ := this
        exact ⟨ty, cs', hargs, by simpa using hr, hcs', this⟩
    · simp [reject] at this

theorem fieldDef_checkType {m : CompiledModule} {tp : Nat} {f : FieldDefinition}
    (h : Bounds.checkFieldDef m tp f = .ok ()) : Bounds.checkType m f.signature = .ok () := by
  unfold Bounds.checkFieldDef at h
  obtain ⟨-, h⟩ := seq_ok2 h
  exact (seq_ok2 h).1

theorem structDef_checkType {m : CompiledModule} {sd : StructDefinition}
    (h : Bounds.checkStructDef m sd = .ok ()) :
    ∀ f ∈ CompiledModule.StructFieldInformation.allFields sd.fieldInformation,
      Bounds.checkType m f.signature = .ok () := by
  unfold Bounds.checkStructDef at h
  obtain ⟨h1, h⟩ := seq_ok2 h
  obtain ⟨sh, hsh⟩ := getElem?_of_lt (checkIdx_ok h1)
  simp only [hsh] at h
  intro f hf
  cases hfi : sd.fieldInformation with
  | native => simp [CompiledModule.StructFieldInformation.allFields, hfi] at hf
  | declared fields =>
    rw [hfi] at h
    simp only [CompiledModule.StructFieldInformation.allFields, hfi] at hf
    exact fieldDef_checkType (forM_ok h f hf)
  | declaredVariants vs =>
    rw [hfi] at h
    simp only at h
    obtain ⟨h2, -⟩ := seq_ok2 h
    simp only [CompiledModule.StructFieldInformation.allFields, hfi, List.mem_flatMap] at hf
    obtain ⟨v, hvm, hfm⟩ := hf
    exact fieldDef_checkType (forM_ok (seq_ok2 (forM_ok h2 v hvm)).2 f hfm)

/-- A type a constant may have is a closed value type. -/
theorem valueType_of_validConstant {m : CompiledModule} :
    ∀ {t : SignatureToken}, isValidForConstant t = true → t.ValueTypeWellFormed m []
  | .vector t, h => by
    simp only [isValidForConstant] at h
    simp only [SignatureToken.ValueTypeWellFormed]
    exact valueType_of_validConstant h
  | .bool, _ | .u8, _ | .u16, _ | .u32, _ | .u64, _ | .u128, _ | .u256, _ | .address, _
  | .i8, _ | .i16, _ | .i32, _ | .i64, _ | .i128, _ | .i256, _ => by
    simp [SignatureToken.ValueTypeWellFormed]
  | .signer, h | .function .., h | .struct _, h | .structInstantiation .., h | .reference _, h
  | .mutableReference _, h | .typeParameter _, h => by simp [isValidForConstant] at h

/-! ## L5 -/

/-- **L5** for a module the model accepts. -/
theorem typeFormation_of_accepted {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) : m.TypeFormation := by
  have hfix : VerifierConfig.production.sigCheckerV2FixFunctionSignatures = true := rfl
  have hfab : VerifierConfig.production.checkFunctionTypeAbilities = true := rfl
  obtain ⟨hb, -, hcon, hcu⟩ := verifyModule_ok h
  have s := verifyBounds_steps hb
  have hsigs : ∀ sig ∈ m.signatures.toList, ∀ t ∈ sig, Bounds.checkType m t = .ok () :=
    fun sig hsig t ht => forM_ok (forM_ok s.sigs sig hsig) t ht
  have hsv := verifyModule_signatures h
  obtain ⟨-, -, -, hfhr, -, -, -, -, -, -, -, -, -, hsdr, -, hfdr⟩ := indexIntegrity_of_accepted h
  -- Function handles.
  have hhandles : ∀ fh ∈ m.functionHandles.toList,
      m.SlotSignatureWellFormed fh.typeParameters fh.parameters ∧
      m.SlotSignatureWellFormed fh.typeParameters fh.returns := by
    intro fh hfh
    obtain ⟨⟨cs1, h1, h1'⟩, ⟨cs2, h2, h2'⟩⟩ := verifySignatures_handles hsv fh hfh
    obtain ⟨-, -, hr1, hr2⟩ := hfhr fh hfh
    exact ⟨slotSignature_of_checked hfix hfab h1 h1' hr1 hsigs,
      slotSignature_of_checked hfix hfab h2 h2' hr2 hsigs⟩
  -- Struct fields.
  have hfields : ∀ d ∈ m.structDefs.toList, ∃ sh, m.structDefHandle d = some sh ∧
      ∀ f ∈ CompiledModule.StructFieldInformation.allFields d.fieldInformation,
        f.signature.ValueTypeWellFormed m (sh.typeParameters.map (·.constraints)) := by
    intro d hd
    obtain ⟨sh, hsh, hfields⟩ := verifyStructDef_ok (verifySignatures_structDefs hsv d hd)
    obtain ⟨sh', hsh', hin⟩ := hsdr d hd
    rw [hsh, Option.some.injEq] at hsh'
    subst hsh'
    refine ⟨sh, hsh, fun f hf => ?_⟩
    obtain ⟨hctx, -, -⟩ := hfields f hf
    obtain ⟨cs, hcs, hc⟩ := checkTyInContext_ok hctx
    have hg : Good m (sh.typeParameters.map (·.constraints)).length f.signature :=
      ⟨by rw [List.length_map]; exact (hin f hf).2,
        arity_of_checkType (structDef_checkType (forM_ok s.structDefs d hd) f hf)⟩
    exact (checkTy_wf hfix hfab (listContext_eq _) f.signature hcs hc hg).2 (checkTy_noRef hcs)
  -- Constants.
  have hconsts : ∀ c ∈ m.constantPool.toList, c.type.ValueTypeWellFormed m [] := fun c hc =>
    valueType_of_validConstant (validConstant_of_verify (forM_ok hcon c hc))
  refine ⟨hhandles, hfields, hconsts, fun d hd code hcode => ?_⟩
  · -- Function definitions.
    obtain ⟨fh, hfh, hvc⟩ := verifySignatures_code hsv d hd code hcode
    obtain ⟨⟨cs, hcs, hcs'⟩, hvec⟩ := verifyCode_parts hvc
    obtain ⟨fh', hfh', -, hrange⟩ := hfdr d hd
    rw [hfh, Option.some.injEq] at hfh'
    subst hfh'
    obtain ⟨params, locals, -, -, hlr, hir⟩ := hrange code hcode
    have hlocals := slotSignature_of_checked hfix hfab hcs hcs' hlr hsigs
    refine ⟨fh, hfh, hlocals, ?_⟩
    obtain ⟨index, hidx⟩ := List.mem_iff_getElem?.1 hd
    rw [Array.getElem?_toList] at hidx
    obtain ⟨n, hn⟩ := verifyCodeUnits_fn hcu hidx
    obtain ⟨handle, hv⟩ := verifyFunction_parts hcode hn
    -- Every instruction but `CallClosure`, from the signature checker.
    have hops : ∀ i ∈ code.code.toList, (∃ s, i = .callClosure s) ∨
        i.TypeOperandsWellFormed m fh.typeParameters := by
      intro i hi
      by_cases hcc : ∃ s, i = .callClosure s
      · exact Or.inl hcc
      right
      -- The element type of a vector instruction is a value type.
      have hvs : ∀ sig k, (Bytecode.vecPack sig k ∈ code.code.toList ∨
        Bytecode.vecUnpack sig k ∈ code.code.toList ∨ Bytecode.vecLen sig ∈ code.code.toList ∨ Bytecode.vecImmBorrow sig ∈ code.code.toList ∨
        Bytecode.vecMutBorrow sig ∈ code.code.toList ∨ Bytecode.vecPushBack sig ∈ code.code.toList ∨
        Bytecode.vecPopBack sig ∈ code.code.toList ∨ Bytecode.vecSwap sig ∈ code.code.toList) →
          m.SignatureInRange fh.typeParameters.length sig →
          m.ValueSignatureWellFormed fh.typeParameters sig := by
        intro sig k hmem hr
        obtain ⟨ty, cs', hty1, hnr, hsc, hctx⟩ := hvec sig k hmem
        obtain ⟨sig', hsig', hslots⟩ := slotSignature_of_checked hfix hfab hsc hctx hr hsigs
        rw [hty1, Option.some.injEq] at hsig'
        subst hsig'
        refine ⟨[ty], hty1, fun t ht => ?_⟩
        simp only [List.mem_singleton] at ht
        subst ht
        exact valueType_of_slots hnr hslots
      cases i <;> simp only [Bytecode.TypeOperandsWellFormed]
      case callClosure s => exact absurd ⟨s, rfl⟩ hcc
      case vecPack sig k =>
        exact hvs sig k (Or.inl hi) (hir _ hi (.signature sig) (by simp [Bytecode.indexRefs]))
      case vecUnpack sig k =>
        exact hvs sig k (Or.inr (Or.inl hi)) (hir _ hi (.signature sig) (by simp [Bytecode.indexRefs]))
      case callGeneric x | packClosureGeneric x _ =>
        obtain ⟨cs, hcs, hctx⟩ := verifyCode_instr hvc hi
        obtain ⟨fi, cfh, args, hfi, hcfh, hargs, hlen, css, hcss, rfl⟩ := functionInstantiation_ok hcs
        obtain ⟨fi', hfi', hr⟩ := hir _ hi (.functionInstantiation x) (by simp [Bytecode.indexRefs])
        rw [hfi, Option.some.injEq] at hfi'
        subst hfi'
        exact ⟨fi, cfh, hfi, hcfh, args, hargs, functionArgs_wf hfix hfab (listContext_eq _) args
          cfh.typeParameters hcss (checkInContext_ok hctx) (good_of_inRange hr hargs hsigs) hlen⟩
      case packGeneric x | unpackGeneric x | mutBorrowGlobalGeneric x | immBorrowGlobalGeneric x
        | existsGeneric x | moveFromGeneric x | moveToGeneric x =>
        obtain ⟨cs, hcs, hctx⟩ := verifyCode_instr hvc hi
        unfold SignatureV2.structInstantiation at hcs
        simp only [bind_ok, lookup_ok] at hcs
        obtain ⟨si, hsi, hcs⟩ := hcs
        obtain ⟨sd, sh, args, hsd, hsh, hargs, hlen, -, css, hcss, rfl⟩ := structTypeParams_ok hcs
        obtain ⟨si', hsi', hr⟩ := hir _ hi (.structDefInstantiation x) (by simp [Bytecode.indexRefs])
        rw [hsi, Option.some.injEq] at hsi'
        subst hsi'
        exact ⟨si, _, hsi, by simp [CompiledModule.structConstraints, CompiledModule.structDefHandle,
          hsd, hsh], args, hargs, structArgs_wf hfix hfab (listContext_eq _) args sh.typeParameters
          hcss (checkInContext_ok hctx) (good_of_inRange hr hargs hsigs) hlen⟩
      case packVariantGeneric x | unpackVariantGeneric x | testVariantGeneric x =>
        obtain ⟨cs, hcs, hctx⟩ := verifyCode_instr hvc hi
        unfold SignatureV2.structVariantInstantiation at hcs
        simp only [bind_ok, lookup_ok] at hcs
        obtain ⟨si, hsi, vh, hvh, hcs⟩ := hcs
        obtain ⟨sd, sh, args, hsd, hsh, hargs, hlen, -, css, hcss, rfl⟩ := structTypeParams_ok hcs
        obtain ⟨si', hsi', hr⟩ := hir _ hi (.structVariantInstantiation x)
          (by simp [Bytecode.indexRefs])
        rw [hsi, Option.some.injEq] at hsi'
        subst hsi'
        exact ⟨si, vh, _, hsi, hvh, by simp [CompiledModule.structConstraints,
          CompiledModule.structDefHandle, hsd, hsh], args, hargs, structArgs_wf hfix hfab
          (listContext_eq _) args sh.typeParameters hcss (checkInContext_ok hctx)
          (good_of_inRange hr hargs hsigs) hlen⟩
      case mutBorrowFieldGeneric x | immBorrowFieldGeneric x =>
        obtain ⟨cs, hcs, hctx⟩ := verifyCode_instr hvc hi
        unfold SignatureV2.fieldInstantiation at hcs
        simp only [bind_ok, lookup_ok] at hcs
        obtain ⟨si, hsi, fdh, hfdh, hcs⟩ := hcs
        obtain ⟨sd, sh, args, hsd, hsh, hargs, hlen, -, css, hcss, rfl⟩ := structTypeParams_ok hcs
        obtain ⟨si', hsi', hr⟩ := hir _ hi (.fieldInstantiation x) (by simp [Bytecode.indexRefs])
        rw [hsi, Option.some.injEq] at hsi'
        subst hsi'
        exact ⟨si, fdh, _, hsi, hfdh, by simp [CompiledModule.structConstraints,
          CompiledModule.structDefHandle, hsd, hsh], args, hargs, structArgs_wf hfix hfab
          (listContext_eq _) args sh.typeParameters hcss (checkInContext_ok hctx)
          (good_of_inRange hr hargs hsigs) hlen⟩
      case mutBorrowVariantFieldGeneric x | immBorrowVariantFieldGeneric x =>
        obtain ⟨cs, hcs, hctx⟩ := verifyCode_instr hvc hi
        unfold SignatureV2.variantFieldInstantiation at hcs
        simp only [bind_ok, lookup_ok] at hcs
        obtain ⟨si, hsi, fdh, hfdh, hcs⟩ := hcs
        obtain ⟨sd, sh, args, hsd, hsh, hargs, hlen, -, css, hcss, rfl⟩ := structTypeParams_ok hcs
        obtain ⟨si', hsi', hr⟩ := hir _ hi (.variantFieldInstantiation x)
          (by simp [Bytecode.indexRefs])
        rw [hsi, Option.some.injEq] at hsi'
        subst hsi'
        exact ⟨si, fdh, _, hsi, hfdh, by simp [CompiledModule.structConstraints,
          CompiledModule.structDefHandle, hsd, hsh], args, hargs, structArgs_wf hfix hfab
          (listContext_eq _) args sh.typeParameters hcss (checkInContext_ok hctx)
          (good_of_inRange hr hargs hsigs) hlen⟩
      all_goals first
        | exact hvs _ 0 (by simp [hi]) (hir _ hi (.signature _) (by simp [Bytecode.indexRefs]))
    -- `CallClosure`: from type safety, at a stack of well-formed tokens.
    intro i hi
    rcases hops i hi with ⟨s, rfl⟩ | hok
    · obtain ⟨cfg, view, hcf, hview, -, hts, -⟩ := hv.views
      obtain ⟨hvc', hvh, hvcfg, hvp, -, hvl, -⟩ := functionView_fields hview
      have hblocks : view.cfg.blocks = buildBlocks view.code.code := by
        rw [hvcfg, verifyControlFlow_blocks hcf, hvc']
      have hhandle : handle = fh := by
        rw [hv.hasHandle, Option.some.injEq] at hfh
        exact hfh
      subst hhandle hvh hvc'
      have htc : TokCtx m view := by
        obtain ⟨ps, hps, hpwf⟩ := (hhandles _ (getElem?_mem_toList hv.hasHandle)).1
        rw [hvp, Option.some.injEq] at hps
        subst hps
        obtain ⟨ls, hls, hlwf⟩ := hlocals
        rw [hvl, Option.some.injEq] at hls
        subst hls
        exact ⟨slotsWF_append.2 ⟨hpwf, hlwf⟩, hconsts, hhandles, hfields⟩
      have hl9 := instructionConsistency_of_accepted h d hd view.code hcode
      obtain ⟨st, st', hst, hty⟩ := typed_inv_of_typeSafety
        (P := SignatureToken.SlotsWellFormed m view.handle.typeParameters) hts hblocks trivial
        (fun j hj st st' hp hj' => typeInstr_tokens htc (hl9 j hj) (hops j hj) hp hj') _ hi
      exact callClosure_wf hst hty (closureOk_of_accepted h hd hcode _ hi)
    · exact hok

/-! ## L6: shared variant fields -/


/-- A non-generic variant field borrow that the type checker accepts names a
field whose listed variants share one type. -/
theorem variantFieldSharedAt_of_borrow {m : CompiledModule} (hb : verifyBounds m = .ok ())
    (htf : m.TypeFormation) {h : Nat} (hcount : m.variantFieldTypeParamCount h = some 0)
    {st st' : List SignatureToken} {mut_ : Bool} (hbf : borrowField m st mut_ true h = .ok st') :
    m.VariantFieldSharedAt h [] := by
  obtain ⟨-, -, fh, sd, -, -, hfh, hsd, -, hne, ft, -, hall⟩ := variantBorrow_ok hbf
  obtain ⟨sd', hsd', -, hvb⟩ := verifyBounds_variantFieldHandle hb hfh
  rw [hsd, Option.some.injEq] at hsd'
  subst hsd'
  obtain ⟨v0, hv0⟩ : ∃ v0, v0 ∈ fh.variants := List.exists_mem_of_ne_nil _ hne
  -- The enum is non-generic, so its field types are closed.
  simp only [CompiledModule.variantFieldTypeParamCount, CompiledModule.structTypeParamCount, hfh,
    hsd, bind, Option.bind_some, Option.bind_eq_some_iff, pure, Option.some.injEq] at hcount
  obtain ⟨sh, hsh, hlen⟩ := hcount
  obtain ⟨sh', hsh', hwf⟩ := htf.2.1 sd (getElem?_mem_toList hsd)
  have hsh'' : m.structHandles[sd.structHandle]? = some sh' := hsh'
  rw [hsh, Option.some.injEq] at hsh''
  subst hsh''
  rw [List.length_eq_zero_iff] at hlen
  rw [hlen] at hwf
  cases hfi : sd.fieldInformation with
  | declaredVariants vs =>
    refine ⟨fh, vs, ft, hfh, by simp [CompiledModule.variants, hsd, hfi], fun v hv => ?_⟩
    obtain ⟨fd, hfd, hsig⟩ := hall v hv
    obtain ⟨hvlt, -⟩ := hvb v hv
    rw [hfi] at hvlt
    simp only [variantCount] at hvlt
    obtain ⟨vd, hvd⟩ : ∃ vd, vs[v]? = some vd := ⟨_, List.getElem?_eq_getElem hvlt⟩
    simp only [structFields, hfi, hvd] at hfd
    refine ⟨vd, fd, hvd, hfd, ?_⟩
    have hmem : fd ∈ CompiledModule.StructFieldInformation.allFields sd.fieldInformation := by
      simp only [CompiledModule.StructFieldInformation.allFields, hfi, List.mem_flatMap]
      exact ⟨vd, List.mem_of_getElem? hvd, List.mem_of_getElem? hfd⟩
    have := hwf fd hmem
    simp only [List.map_nil] at this
    rw [instantiate_closed _ this, hsig]
  | declared _ =>
    obtain ⟨hvlt, -⟩ := hvb v0 hv0
    rw [hfi] at hvlt
    simp [variantCount] at hvlt
  | native =>
    obtain ⟨hvlt, -⟩ := hvb v0 hv0
    rw [hfi] at hvlt
    simp [variantCount] at hvlt

/-- A generic variant field borrow the model accepts: every listed variant has
the field, of one type after substituting the type arguments. -/
theorem variantBorrowGeneric_ok {m : CompiledModule} {st st' : List SignatureToken}
    {mut_ : Bool} {h : Nat} {tys : List SignatureToken}
    (hok : borrowFieldGeneric m st mut_ true h tys = .ok st') :
    ∃ fh sd, m.variantFieldHandles[h]? = some fh ∧ m.structDefs[fh.structIndex]? = some sd ∧
      fh.variants ≠ [] ∧ ∃ ft, ∀ v ∈ fh.variants, ∃ fd,
        (structFields sd.fieldInformation (some v))[fh.field]? = some fd ∧
        instantiate tys fd.signature = .ok ft := by
  simp only [borrowFieldGeneric, bind_ok, fieldHandleParts, ↓reduceIte, lookup_ok, pure,
    Except.pure] at hok
  obtain ⟨⟨op, rest⟩, -, u, -, p, ⟨fh, hfh, hp⟩, sd, hsd, u', -, h⟩ := hok
  simp only [Except.ok.injEq] at hp
  subst hp
  dsimp only at h
  split at h
  · simp [reject, bind, Except.bind] at h
  · next hne =>
    have hne' : fh.variants ≠ [] := by simpa using hne
    simp only [bind_ok] at h
    obtain ⟨r, hr, -⟩ := h
    rw [variantFieldTypeGeneric_eq] at hr
    obtain ⟨ft, -, hall⟩ := vftg_none hr hne'
    exact ⟨fh, sd, hfh, hsd, hne', ft, hall⟩

/-- The variant field borrow of a generic form the model accepts, at type
arguments `tys` (not empty), names a field the listed variants share at that
instance. -/
theorem variantFieldSharedAt_of_generic {m : CompiledModule} (hb : verifyBounds m = .ok ())
    {h : Nat} {tys : List SignatureToken} (hne : tys ≠ []) {st st' : List SignatureToken}
    {mut_ : Bool} (hbf : borrowFieldGeneric m st mut_ true h tys = .ok st') :
    m.VariantFieldSharedAt h tys := by
  obtain ⟨fh, sd, hfh, hsd, hvne, ft, hall⟩ := variantBorrowGeneric_ok hbf
  obtain ⟨sd', hsd', -, hvb⟩ := verifyBounds_variantFieldHandle hb hfh
  rw [hsd, Option.some.injEq] at hsd'
  subst hsd'
  obtain ⟨v0, hv0⟩ : ∃ v0, v0 ∈ fh.variants := List.exists_mem_of_ne_nil _ hvne
  cases hfi : sd.fieldInformation with
  | declaredVariants vs =>
    refine ⟨fh, vs, ft, hfh, by simp [CompiledModule.variants, hsd, hfi], fun v hv => ?_⟩
    obtain ⟨fd, hfd, hsig⟩ := hall v hv
    obtain ⟨hvlt, -⟩ := hvb v hv
    rw [hfi] at hvlt
    simp only [variantCount] at hvlt
    obtain ⟨vd, hvd⟩ : ∃ vd, vs[v]? = some vd := ⟨_, List.getElem?_eq_getElem hvlt⟩
    simp only [structFields, hfi, hvd] at hfd
    refine ⟨vd, fd, hvd, hfd, ?_⟩
    have hne' : tys.isEmpty = false := by cases tys <;> simp_all
    simp only [instantiate, hne', Bool.false_eq_true, ↓reduceIte] at hsig
    exact instantiateIn_spec _ hsig
  | declared _ =>
    obtain ⟨hvlt, -⟩ := hvb v0 hv0
    rw [hfi] at hvlt
    simp [variantCount] at hvlt
  | native =>
    obtain ⟨hvlt, -⟩ := hvb v0 hv0
    rw [hfi] at hvlt
    simp [variantCount] at hvlt

/-- The variant-field part of **L6** for a module the model accepts: its type
checker accepts every variant field borrow only of a field the listed
variants share, comparing the field types after substituting the type
arguments of a generic form. -/
theorem variantFieldsShared_of_accepted {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) : m.VariantFieldsShared := by
  intro d hd code hcode
  obtain ⟨index, hidx⟩ := List.mem_iff_getElem?.1 hd
  rw [Array.getElem?_toList] at hidx
  obtain ⟨hb, -, -, hcu⟩ := verifyModule_ok h
  obtain ⟨n, hn⟩ := verifyCodeUnits_fn hcu hidx
  obtain ⟨handle, hv⟩ := verifyFunction_parts hcode hn
  have hic := instructionConsistency_of_accepted h
  have htf := typeFormation_of_accepted h
  refine ⟨fun hh hmem => ?_, fun _ hmem => ?_⟩
  · rcases hmem with hmem | hmem
    · obtain ⟨view, st, st', hty⟩ := typed_of_verified hv _ hmem
      exact variantFieldSharedAt_of_borrow hb htf (hic d hd code hcode _ hmem) hty
    · obtain ⟨view, st, st', hty⟩ := typed_of_verified hv _ hmem
      exact variantFieldSharedAt_of_borrow hb htf (hic d hd code hcode _ hmem) hty
  · rcases hmem with hmem | hmem <;>
    · obtain ⟨view, st, st', hty⟩ := typed_of_verified hv _ hmem
      have hl9 := hic d hd code hcode _ hmem
      simp only [typeInstr, bind_ok, lookup_ok] at hty
      obtain ⟨fi, hfi, tys, htys, hty⟩ := hty
      simp only [Bytecode.OperandsConsistent] at hl9
      obtain ⟨x, hx, n, args, -, hpos, hargs, hlen⟩ := hl9
      rw [hfi, Option.some.injEq] at hx
      subst hx
      rw [htys, Option.some.injEq] at hargs
      subst hargs
      have hne : tys ≠ [] := by
        intro h0
        rw [h0] at hlen
        simp at hlen
        omega
      exact ⟨fi, tys, hfi, htys, variantFieldSharedAt_of_generic hb hne hty⟩

/-- **L6** for a module the model accepts, given the phantom-position rule for
field types that mention a reference. -/
theorem structDefinitionsWellFormed_of_accepted (hph : PhantomUnderReferences)
    {m : CompiledModule} (h : verifyModule VerifierConfig.production m = .ok ()) :
    m.StructDefinitionsWellFormed := by
  refine ⟨fun d hd => ?_, localStructsAcyclic_of_accepted h, variantFieldsShared_of_accepted h⟩
  obtain ⟨sh, hsh, hab⟩ := structAbilities_of_accepted h d hd
  obtain ⟨sh', hsh', hfields⟩ :=
    verifyStructDef_ok (verifySignatures_structDefs (verifyModule_signatures h) d hd)
  have hsh'' : m.structHandles[d.structHandle]? = some sh := hsh
  rw [hsh'', Option.some.injEq] at hsh'
  subst hsh'
  refine ⟨sh, hsh, fun f hf => ⟨hab f hf, ?_⟩⟩
  cases hr : hasRef f.signature
  · exact checkPhantom_respected f.signature (hfields f hf).2.2 (Or.inr hr)
  · exact hph m h d hd sh hsh f hf hr


end MoveBytecode.Proofs
