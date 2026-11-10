-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.StackUsage

/-!
# Syntactic bounds (L11) from the limits pass

UNTRUSTED (checked by the kernel). The limits pass (`Model.verifyLimits`)
bounds the numbers of definitions, fields and variants, the parameters and
return values of every function handle, and the weighted size and depth of
every token of the signature pool and every field type. Each node of a token
weighs at least one, so the weighted size bounds the token's number of nodes
(`nodeCount_le_weight`), and the depth the pass checks node by node, counting
the root as depth one, reaches the token's depth at its deepest node
(`exists_deepest`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Tokens -/

theorem one_le_typeNodeWeight (t : SignatureToken) : 1 ≤ typeNodeWeight t := by
  cases t <;> simp [typeNodeWeight]

/-- The weighted size of a list of preorder entries. -/
def weightOf (l : List (SignatureToken × Nat)) : Nat :=
  (l.map fun p => typeNodeWeight p.1).sum

theorem weightOf_append (a b : List (SignatureToken × Nat)) :
    weightOf (a ++ b) = weightOf a + weightOf b := by
  simp [weightOf]

theorem weightOf_cons (p : SignatureToken × Nat) (l : List (SignatureToken × Nat)) :
    weightOf (p :: l) = typeNodeWeight p.1 + weightOf l := by
  simp [weightOf]

mutual
/-- A token has at most its weighted size in nodes. -/
theorem nodeCount_le_weight : ∀ (t : SignatureToken) (d : Nat),
    t.nodeCount ≤ weightOf (preorderDepth t d)
  | .vector e, d => by
    have := nodeCount_le_weight e (d + 1)
    have := one_le_typeNodeWeight (.vector e)
    simp only [preorderDepth, SignatureToken.nodeCount, weightOf_cons]
    omega
  | .reference e, d => by
    have := nodeCount_le_weight e (d + 1)
    have := one_le_typeNodeWeight (.reference e)
    simp only [preorderDepth, SignatureToken.nodeCount, weightOf_cons]
    omega
  | .mutableReference e, d => by
    have := nodeCount_le_weight e (d + 1)
    have := one_le_typeNodeWeight (.mutableReference e)
    simp only [preorderDepth, SignatureToken.nodeCount, weightOf_cons]
    omega
  | .structInstantiation h args, d => by
    have := nodeCountList_le_weight args (d + 1)
    have := one_le_typeNodeWeight (.structInstantiation h args)
    simp only [preorderDepth, SignatureToken.nodeCount, weightOf_cons]
    omega
  | .function ps rs a, d => by
    have := nodeCountList_le_weight ps (d + 1)
    have := nodeCountList_le_weight rs (d + 1)
    have := one_le_typeNodeWeight (.function ps rs a)
    simp only [preorderDepth, SignatureToken.nodeCount, weightOf_cons, weightOf_append]
    omega
  | .bool, d | .u8, d | .u16, d | .u32, d | .u64, d | .u128, d | .u256, d | .i8, d | .i16, d
  | .i32, d | .i64, d | .i128, d | .i256, d | .address, d | .signer, d | .struct _, d
  | .typeParameter _, d => by
    simp only [preorderDepth, SignatureToken.nodeCount, weightOf]
    have := one_le_typeNodeWeight (.bool)
    simp [typeNodeWeight]

theorem nodeCountList_le_weight : ∀ (ts : List SignatureToken) (d : Nat),
    SignatureToken.nodeCountList ts ≤ weightOf (preorderDepthList ts d)
  | [], _ => by simp [SignatureToken.nodeCountList, preorderDepthList, weightOf]
  | t :: ts, d => by
    have := nodeCount_le_weight t d
    have := nodeCountList_le_weight ts d
    simp only [SignatureToken.nodeCountList, preorderDepthList, weightOf_append]
    omega
end

theorem leaf_deepest {t : SignatureToken} {d : Nat} (h : preorderDepth t d = [(t, d)])
    (hd : t.depth = 1) : ∃ p ∈ preorderDepth t d, p.2 + 1 = d + t.depth :=
  ⟨(t, d), by rw [h]; simp, by simp [hd]⟩

mutual
/-- The deepest node of a token, from depth `d`, is at depth
`d + depth - 1`. -/
theorem exists_deepest : ∀ (t : SignatureToken) (d : Nat),
    ∃ p ∈ preorderDepth t d, p.2 + 1 = d + t.depth
  | .vector e, d => by
    obtain ⟨p, hp, hd⟩ := exists_deepest e (d + 1)
    exact ⟨p, by simp [preorderDepth, hp], by simp [SignatureToken.depth]; omega⟩
  | .reference e, d => by
    obtain ⟨p, hp, hd⟩ := exists_deepest e (d + 1)
    exact ⟨p, by simp [preorderDepth, hp], by simp [SignatureToken.depth]; omega⟩
  | .mutableReference e, d => by
    obtain ⟨p, hp, hd⟩ := exists_deepest e (d + 1)
    exact ⟨p, by simp [preorderDepth, hp], by simp [SignatureToken.depth]; omega⟩
  | .structInstantiation h args, d => by
    cases args with
    | nil => exact ⟨(.structInstantiation h [], d), by simp [preorderDepth], by
        simp [SignatureToken.depth, SignatureToken.depthList]⟩
    | cons a as =>
      obtain ⟨p, hp, hd⟩ := exists_deepestList (a :: as) (d + 1) (by simp)
      exact ⟨p, by simp only [preorderDepth, List.mem_cons]; exact Or.inr hp,
        by simp only [SignatureToken.depth]; omega⟩
  | .function ps rs ab, d => by
    by_cases hps : ps = []
    · by_cases hrs : rs = []
      · subst hps hrs
        exact ⟨(.function [] [] ab, d), by simp [preorderDepth], by
          simp [SignatureToken.depth, SignatureToken.depthList]⟩
      · obtain ⟨p, hp, hd⟩ := exists_deepestList rs (d + 1) hrs
        subst hps
        exact ⟨p, by simp [preorderDepth, hp], by
          simp only [SignatureToken.depth, SignatureToken.depthList]; omega⟩
    · by_cases hrs : rs = []
      · obtain ⟨p, hp, hd⟩ := exists_deepestList ps (d + 1) hps
        subst hrs
        exact ⟨p, by simp [preorderDepth, hp], by
          simp only [SignatureToken.depth, SignatureToken.depthList]; omega⟩
      · obtain ⟨p, hp, hd⟩ := exists_deepestList ps (d + 1) hps
        obtain ⟨q, hq, hd'⟩ := exists_deepestList rs (d + 1) hrs
        by_cases hle : SignatureToken.depthList rs ≤ SignatureToken.depthList ps
        · exact ⟨p, by simp [preorderDepth, hp], by
            simp only [SignatureToken.depth]; omega⟩
        · exact ⟨q, by simp [preorderDepth, hq], by
            simp only [SignatureToken.depth]; omega⟩
  | .bool, d | .u8, d | .u16, d | .u32, d | .u64, d | .u128, d | .u256, d | .i8, d | .i16, d
  | .i32, d | .i64, d | .i128, d | .i256, d | .address, d | .signer, d | .struct _, d
  | .typeParameter _, d => leaf_deepest rfl rfl

theorem exists_deepestList : ∀ (ts : List SignatureToken) (d : Nat), ts ≠ [] →
    ∃ p ∈ preorderDepthList ts d, p.2 + 1 = d + SignatureToken.depthList ts
  | [], _, h => absurd rfl h
  | t :: ts, d, _ => by
    obtain ⟨p, hp, hd⟩ := exists_deepest t d
    by_cases hts : ts = []
    · subst hts
      exact ⟨p, by simp [preorderDepthList, hp], by
        simp only [SignatureToken.depthList]; omega⟩
    · obtain ⟨q, hq, hd'⟩ := exists_deepestList ts d hts
      by_cases hle : SignatureToken.depthList ts ≤ t.depth
      · exact ⟨p, by simp [preorderDepthList, hp], by
          simp only [SignatureToken.depthList]; omega⟩
      · exact ⟨q, by simp [preorderDepthList, hq], by
          simp only [SignatureToken.depthList]; omega⟩
end

/-- The size fold of `verify_type_node` returns the weighted size, having
checked every node's depth. -/
theorem typeNode_fold {config : VerifierConfig} :
    ∀ (l : List (SignatureToken × Nat)) (size r : Nat),
      l.foldlM (fun size (p : SignatureToken × Nat) => do
        if Limits.over config.maxTypeDepth p.2 then reject "TOO_MANY_TYPE_NODES"
        match p.1 with
        | .function ps rs _ =>
          if Limits.over config.maxFunctionParameters ps.length then reject "TOO_MANY_PARAMETERS"
          if Limits.over config.maxFunctionReturnValues rs.length then reject "TOO_MANY_PARAMETERS"
        | _ => pure ()
        pure (size + typeNodeWeight p.1)) size = .ok r →
      r = size + weightOf l ∧ ∀ p ∈ l, Limits.over config.maxTypeDepth p.2 = false
  | [], size, r, h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    simp [h, weightOf]
  | ⟨tok, dd⟩ :: l, size, r, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨s1, h1, h2⟩ := h
    have hdepth : Limits.over config.maxTypeDepth dd = false := by
      cases hov : Limits.over config.maxTypeDepth dd
      · rfl
      · simp [hov, reject, bind, Except.bind] at h1
    have hs1 : s1 = size + typeNodeWeight tok := by
      simp only [hdepth, Bool.false_eq_true, ↓reduceIte] at h1
      cases tok <;> simp only at h1 <;> (try (repeat' split at h1)) <;>
        simp_all [reject, bind, Except.bind, pure, Except.pure]
    subst hs1
    obtain ⟨hr, hall⟩ := typeNode_fold l _ r h2
    refine ⟨by rw [hr, weightOf_cons]; simp only; omega, fun q hq => ?_⟩
    rcases List.mem_cons.1 hq with rfl | hq
    · exact hdepth
    · exact hall q hq

theorem over_false {limit : Option Nat} {n : Nat} (h : Limits.over limit n = false) :
    WithinLimit limit n := by
  intro k hk
  subst hk
  simpa [Limits.over] using h

/-- A token the limits pass accepts is within the size and depth bounds. -/
theorem typeNode_within {config : VerifierConfig} {t : SignatureToken}
    (hset : config.maxTypeNodes.isSome ∨ config.maxTypeDepth.isSome)
    (h : Limits.typeNode config t = .ok ()) : CompiledModule.TokenWithinLimits config t := by
  unfold Limits.typeNode at h
  have hne : (config.maxTypeNodes.isNone && config.maxFunctionParameters.isNone &&
      config.maxFunctionReturnValues.isNone && config.maxTypeDepth.isNone) = false := by
    rcases hset with h' | h' <;> simp_all [Option.isNone_iff_eq_none, Option.isSome_iff_ne_none]
  simp only [hne, Bool.false_eq_true, ↓reduceIte, bind_ok] at h
  obtain ⟨size, hfold, h⟩ := h
  obtain ⟨hsize, hdepths⟩ := typeNode_fold _ 0 size hfold
  have hsz : Limits.over config.maxTypeNodes size = false := by
    cases hov : Limits.over config.maxTypeNodes size
    · rfl
    · simp [hov, reject] at h
  refine ⟨fun k hk => ?_, fun k hk => ?_⟩
  · have := over_false hsz k hk
    have := nodeCount_le_weight t 1
    omega
  · obtain ⟨p, hp, hd⟩ := exists_deepest t 1
    have := over_false (hdepths p hp) k hk
    omega

/-! ## The module -/

theorem rejectIf_unit {c : Bool} {code : String} {x : VResult Unit}
    (h : ((if c then reject code else pure ()) >>= fun _ => x) = .ok ()) : c = false ∧ x = .ok () := by
  cases c <;> simp_all [reject, bind, Except.bind, pure, Except.pure]

/-- What the limits pass establishes about a module. -/
theorem verifyLimits_parts {config : VerifierConfig} {m : CompiledModule}
    (h : verifyLimits config m = .ok ()) :
    Limits.functionHandles config m = .ok () ∧ Limits.typeNodes config m = .ok () ∧
      Limits.definitions config m = .ok () := by
  unfold verifyLimits at h
  simp only [bind_ok] at h
  obtain ⟨u1, h1, u2, -, u3, h3, h4⟩ := h
  cases u1; cases u3
  exact ⟨h1, h3, h4⟩

theorem functionHandles_within {config : VerifierConfig} {m : CompiledModule}
    (h : Limits.functionHandles config m = .ok ()) {fh : FunctionHandle}
    (hfh : fh ∈ m.functionHandles.toList) {params returns : Signature}
    (hp : m.signatures[fh.parameters]? = some params)
    (hr : m.signatures[fh.returns]? = some returns) :
    WithinLimit config.maxFunctionParameters params.length ∧
      WithinLimit config.maxFunctionReturnValues returns.length := by
  have hc := forM_ok h fh hfh
  have hpl : lookup m.signatures fh.parameters "signature_at" = .ok params := lookup_ok.2 hp
  have hrl : lookup m.signatures fh.returns "signature_at" = .ok returns := lookup_ok.2 hr
  simp only [hpl, hrl, bind, Except.bind, pure, Except.pure, reject] at hc
  refine ⟨over_false ?_, over_false ?_⟩ <;>
  · revert hc
    cases Limits.over config.maxGenericInstantiationLength fh.typeParameters.length <;>
    cases config.maxFunctionParameters <;> cases config.maxFunctionReturnValues <;>
    simp [Limits.over] <;> (try split) <;> (try split) <;> simp_all <;> omega

theorem typeNodes_within {config : VerifierConfig} {m : CompiledModule}
    (hset : config.maxTypeNodes.isSome ∨ config.maxTypeDepth.isSome)
    (h : Limits.typeNodes config m = .ok ()) :
    (∀ sig ∈ m.signatures.toList, ∀ t ∈ sig, CompiledModule.TokenWithinLimits config t) ∧
    (∀ d ∈ m.structDefs.toList,
      ∀ f ∈ CompiledModule.StructFieldInformation.allFields d.fieldInformation,
        CompiledModule.TokenWithinLimits config f.signature) := by
  unfold Limits.typeNodes at h
  simp only [bind_ok] at h
  obtain ⟨u1, h1, u2, -, h3⟩ := h
  cases u1
  refine ⟨fun sig hsig t ht => typeNode_within hset (forM_ok (forM_ok h1 sig hsig) t ht),
    fun d hd f hf => ?_⟩
  have hc := forM_ok h3 d hd
  revert hf hc
  cases d.fieldInformation with
  | native => simp [CompiledModule.StructFieldInformation.allFields]
  | declared fields =>
    intro hf hc
    simp only [CompiledModule.StructFieldInformation.allFields] at hf
    exact typeNode_within hset (forM_ok hc f hf)
  | declaredVariants variants =>
    intro hf hc
    simp only [CompiledModule.StructFieldInformation.allFields, List.mem_flatMap] at hf
    obtain ⟨v, hv, hf⟩ := hf
    exact typeNode_within hset (forM_ok (forM_ok hc v hv) f hf)

theorem foldl_max_mono (l : List VariantDefinition) {a b : Nat} (h : a ≤ b) :
    a ≤ l.foldl (fun mx w => Nat.max mx w.fields.length) b := by
  induction l generalizing b with
  | nil => simpa using h
  | cons w ws ih => exact ih (Nat.le_trans h (Nat.le_max_left _ _))

theorem foldl_max_ge {l : List VariantDefinition} {v : VariantDefinition} (hv : v ∈ l) (a : Nat) :
    v.fields.length ≤ l.foldl (fun mx w => Nat.max mx w.fields.length) a := by
  induction l generalizing a with
  | nil => cases hv
  | cons w ws ih =>
    simp only [List.foldl_cons]
    rcases List.mem_cons.1 hv with rfl | hv
    · exact foldl_max_mono ws (Nat.le_max_right _ _)
    · exact ih hv _

theorem definitions_within {config : VerifierConfig} {m : CompiledModule}
    (h : Limits.definitions config m = .ok ()) :
    WithinLimit config.maxStructDefinitions m.structDefs.size ∧
    WithinLimit config.maxFunctionDefinitions m.functionDefs.size ∧
    (∀ d ∈ m.structDefs.toList, match d.fieldInformation with
      | .native => True
      | .declared fields => WithinLimit config.maxFieldsInStruct fields.length
      | .declaredVariants variants =>
        WithinLimit config.maxStructVariants variants.length ∧
        ∀ v ∈ variants, WithinLimit config.maxFieldsInStruct v.fields.length) := by
  unfold Limits.definitions at h
  have hfd : Limits.over config.maxFunctionDefinitions m.functionDefs.size = false := by
    revert h; cases Limits.over config.maxFunctionDefinitions m.functionDefs.size <;>
      simp [reject, bind, Except.bind]
  have hsd : Limits.over config.maxStructDefinitions m.structDefs.size = false := by
    revert h; cases Limits.over config.maxStructDefinitions m.structDefs.size <;>
      simp [hfd, reject, bind, Except.bind]
  simp only [hfd, hsd, Bool.false_eq_true, ↓reduceIte, pure, bind, Except.bind] at h
  refine ⟨over_false hsd, over_false hfd, fun d hd => ?_⟩
  -- The variants and the fields of the definition.
  have hfields : ∀ k, config.maxFieldsInStruct = some k →
      Limits.over (some k) (match d.fieldInformation with
        | .native => 0
        | .declared fields => fields.length
        | .declaredVariants variants =>
          variants.foldl (fun mx v => Nat.max mx v.fields.length) 0) = false := by
    intro k hk
    simp only [hk, Option.isSome_some, ↓reduceIte] at h
    split at h
    · simp at h
    · next u hfm =>
      have hc := forM_ok hfm d hd
      revert hc
      cases Limits.over (some k) (match d.fieldInformation with
        | .native => 0
        | .declared fields => fields.length
        | .declaredVariants variants =>
          variants.foldl (fun mx v => Nat.max mx v.fields.length) 0) <;>
        simp [reject, Except.pure]
  have hvars : ∀ variants, d.fieldInformation = .declaredVariants variants →
      Limits.over config.maxStructVariants variants.length = false := by
    intro variants hfi
    have hlast : (m.structDefs.toList.forM fun sd =>
        match sd.fieldInformation with
        | .declaredVariants variants =>
          if Limits.over config.maxStructVariants variants.length = true then
            reject "MAX_STRUCT_VARIANTS_REACHED" else Except.pure ()
        | .native | .declared _ => Except.pure ()) = .ok () := by
      cases hm : config.maxFieldsInStruct with
      | none =>
        simp only [hm, Option.isSome_none, Bool.false_eq_true, ↓reduceIte] at h
        exact h
      | some k =>
        simp only [hm, Option.isSome_some, ↓reduceIte] at h
        split at h
        · simp at h
        · exact h
    have hc := forM_ok hlast d hd
    simp only [hfi] at hc
    cases hov : Limits.over config.maxStructVariants variants.length
    · rfl
    · simp [hov, reject] at hc
  cases hfi : d.fieldInformation with
  | native => trivial
  | declared fields =>
    intro k hk
    have := hfields k hk
    rw [hfi] at this
    exact over_false this k rfl
  | declaredVariants variants =>
    refine ⟨over_false (hvars variants hfi), fun v hv k hk => ?_⟩
    have := hfields k hk
    rw [hfi] at this
    exact Nat.le_trans (foldl_max_ge hv 0) (over_false this k rfl)

/-! ## Basic blocks -/

theorem nodup_length_le : ∀ {L M : List Nat}, L.Nodup → (∀ x ∈ L, x ∈ M) →
    L.length ≤ M.length
  | [], _, _, _ => Nat.zero_le _
  | a :: L, M, hnd, hsub => by
    have ha : a ∈ M := hsub a (List.mem_cons_self ..)
    have hnd' := List.nodup_cons.1 hnd
    have hL : ∀ x ∈ L, x ∈ M.erase a := fun x hx => by
      have hxa : x ≠ a := fun h => hnd'.1 (h ▸ hx)
      exact (List.mem_erase_of_ne hxa).2 (hsub x (List.mem_cons_of_mem _ hx))
    have := nodup_length_le hnd'.2 hL
    rw [List.length_erase_of_mem ha] at this
    have : 0 < M.length := List.length_pos_of_mem ha
    simp only [List.length_cons]
    omega

/-- Every block leader starts one of the model's basic blocks, so there are no
more leaders than blocks. -/
theorem basicBlocksAtMost_of {code : Array Bytecode} {cfg : Cfg} {k : Nat}
    (hcfg : cfg.blocks = buildBlocks code) (hk : cfg.blocks.length ≤ k) :
    BasicBlocksAtMost code k := by
  intro leaders hnd hl
  have hsub : ∀ l ∈ leaders, l ∈ (buildBlocks code).map (·.start) := by
    intro l hlm
    obtain ⟨hlt, hlead⟩ := hl l hlm
    have hmem := isLeader_mem hlt hlead
    obtain ⟨b, hb, h1, h2⟩ := (buildBlocks_spec code).2 l hlt
    have hok := (buildBlocks_spec code).1 b hb
    have hstart : b.start = l := by
      rcases Nat.lt_or_eq_of_le h1 with hlt' | heq
      · exact absurd hmem (hok.inner l hlt' h2)
      · exact heq
    exact List.mem_map.2 ⟨b, hb, hstart⟩
  have := nodup_length_le hnd hsub
  rw [List.length_map, ← hcfg] at this
  omega

/-- **L11** from the limits pass, given the lookups of the function
definitions' handles (L1) and the bounds on basic blocks and loop depth
(checked by the control-flow pass). -/
theorem withinLimits_of {config : VerifierConfig} {m : CompiledModule}
    (hset : config.maxTypeNodes.isSome ∨ config.maxTypeDepth.isSome)
    (h : verifyLimits config m = .ok ())
    (hlook : ∀ d ∈ m.functionDefs.toList, ∃ fh params returns,
      m.functionHandles[d.function]? = some fh ∧ m.signatures[fh.parameters]? = some params ∧
      m.signatures[fh.returns]? = some returns)
    (hcode : ∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code →
      (∀ k, config.maxBasicBlocks = some k → BasicBlocksAtMost code.code k) ∧
      (∀ k, config.maxLoopDepth = some k → LoopDepthAtMost code.code k)) :
    m.WithinLimits config := by
  obtain ⟨hfh, hty, hdef⟩ := verifyLimits_parts h
  obtain ⟨hsd, hfd, hfields⟩ := definitions_within hdef
  obtain ⟨hsigs, hflds⟩ := typeNodes_within hset hty
  refine ⟨hsd, hfd, hfields, fun d hd => ?_, hcode, hsigs, hflds⟩
  obtain ⟨fh, params, returns, hf, hp, hr⟩ := hlook d hd
  exact ⟨fh, params, returns, hf, hp, hr,
    functionHandles_within hfh (Array.mem_toList_iff.2 (Array.mem_of_getElem? hf)) hp hr⟩

end MoveBytecode.Proofs
