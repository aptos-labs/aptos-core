-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Linking

/-!
# Local properties L1–L4 from the model

UNTRUSTED (checked by the kernel). For a module the model accepts: L2 from the
duplication checker, L1 from the bounds checker (and the constants pass, which
rules out type parameters in constant types), L4 from stack usage, L8 from
the constants pass, and L9 from instruction consistency, type safety and the
signature checker. L4 and L9 cover every function because the model's type
checker rejects instructions outside the supported subset.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## L2 -/

/-- **L2** from the duplication checker. -/
theorem uniqueness_of_duplication {m : CompiledModule} (h : verifyDuplication m = .ok ()) :
    m.Uniqueness := by
  have hf := verifyDuplication_ok h
  unfold verifyDuplication at h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨hfr, h⟩ := seq_ok2 h
  obtain ⟨hsh, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨-, h⟩ := seq_ok2 h
  obtain ⟨hsd, -⟩ := seq_ok2 h
  unfold checkStructDefinitions at hsd
  obtain ⟨hsdn, hsd⟩ := seq_ok2 hsd
  obtain ⟨-, hsd⟩ := seq_ok2 hsd
  obtain ⟨hown, -⟩ := seq_ok2 hsd
  refine ⟨hf.identifiers, hf.addresses, hf.moduleHandles, noDuplicate_ok hfr, noDuplicate_ok hsh,
    hf.handles, noDuplicate_ok hsdn, hf.defs, fun d hd => ?_, hf.ownDefs⟩
  have := forM_ok hown d hd
  simp only [bind_ok, lookup_ok] at this
  obtain ⟨sh, hsh', hm⟩ := this
  refine ⟨sh, hsh', ?_⟩
  split at hm
  · simp [reject] at hm
  · next hne => simpa using hne

/-! ## L4 -/

/-! ## Type tokens -/

theorem mem_preorderList {x : SignatureToken} :
    ∀ {l : List SignatureToken}, x ∈ preorderList l ↔ ∃ t ∈ l, x ∈ preorder t
  | [] => by simp [preorderList]
  | t :: ts => by
    simp only [preorderList, List.mem_append, List.mem_cons, exists_eq_or_imp]
    rw [mem_preorderList]

/-- The token in a traversal that a reference comes from. -/
def RefTok (r : TokenRef) (l : List SignatureToken) : Prop :=
  match r with
  | .structHandle i =>
    SignatureToken.struct i ∈ l ∨ ∃ args, SignatureToken.structInstantiation i args ∈ l
  | .typeParameter i => SignatureToken.typeParameter i ∈ l

theorem RefTok.mono {r : TokenRef} {l l' : List SignatureToken} (h : ∀ x ∈ l, x ∈ l')
    (hr : RefTok r l) : RefTok r l' := by
  cases r with
  | structHandle i =>
    rcases hr with hr | ⟨args, hr⟩
    · exact Or.inl (h _ hr)
    · exact Or.inr ⟨args, h _ hr⟩
  | typeParameter i => exact h _ hr

/-- Every reference of a token comes from a token in its pre-order traversal. -/
theorem refs_preorder (t : SignatureToken) : ∀ r ∈ t.refs, RefTok r (preorder t) := by
  induction t using SignatureToken.rec
    (motive_2 := fun l => ∀ r ∈ SignatureToken.refsList l, RefTok r (preorderList l)) with
  | vector e ih =>
    intro r hr
    exact RefTok.mono (fun x hx => by simp [preorder, hx]) (ih r (by simpa [SignatureToken.refs] using hr))
  | reference e ih =>
    intro r hr
    exact RefTok.mono (fun x hx => by simp [preorder, hx]) (ih r (by simpa [SignatureToken.refs] using hr))
  | mutableReference e ih =>
    intro r hr
    exact RefTok.mono (fun x hx => by simp [preorder, hx]) (ih r (by simpa [SignatureToken.refs] using hr))
  | function ps rs ab ihp ihr =>
    intro r hr
    simp only [SignatureToken.refs, List.mem_append] at hr
    rcases hr with hr | hr
    · exact RefTok.mono (fun x hx => by simp [preorder, hx]) (ihp r hr)
    · exact RefTok.mono (fun x hx => by simp [preorder, hx]) (ihr r hr)
  | struct h =>
    intro r hr
    simp only [SignatureToken.refs, List.mem_singleton] at hr
    subst hr
    exact Or.inl (by simp [preorder])
  | structInstantiation h args ih =>
    intro r hr
    simp only [SignatureToken.refs, List.mem_cons] at hr
    rcases hr with rfl | hr
    · exact Or.inr ⟨args, by simp [preorder]⟩
    · exact RefTok.mono (fun x hx => by simp [preorder, hx]) (ih r hr)
  | typeParameter i =>
    intro r hr
    simp only [SignatureToken.refs, List.mem_singleton] at hr
    subst hr
    show SignatureToken.typeParameter i ∈ preorder (.typeParameter i)
    simp [preorder]
  | nil => rename_i r hr; simp [SignatureToken.refsList] at hr
  | cons t ts iht ihts =>
    rename_i r hr
    simp only [SignatureToken.refsList, List.mem_append] at hr
    rcases hr with hr | hr
    · exact RefTok.mono (fun x hx => by simp [preorderList, hx]) (iht r hr)
    · exact RefTok.mono (fun x hx => by simp [preorderList, hx]) (ihts r hr)
  | _ => intro r hr; simp [SignatureToken.refs] at hr


/-! ## The bounds checker's per-entry checks -/

section Checks

variable {m : CompiledModule}

theorem checkType_ok {t : SignatureToken} (h : Bounds.checkType m t = .ok ()) :
    ∀ x ∈ preorder t, (∀ i, x = .struct i → i < m.structHandles.size) ∧
      (∀ i args, x = .structInstantiation i args → i < m.structHandles.size) := by
  intro x hx
  have hb := forM_ok h x hx
  refine ⟨fun i hi => ?_, fun i args hi => ?_⟩ <;> subst hi
  · exact checkIdx_ok (seq_ok2 hb).1
  · exact checkIdx_ok (seq_ok2 hb).1

theorem checkTypeParametersInTy_ok {t : SignatureToken} {count : Nat}
    (h : Bounds.checkTypeParametersInTy t count = .ok ()) :
    ∀ i, SignatureToken.typeParameter i ∈ preorder t → i < count := by
  intro i hi
  have hb := forM_ok h _ hi
  rcases Nat.lt_or_ge i count with hlt | hge
  · exact hlt
  · simp [hge, reject] at hb

theorem le_max_left' (a b : Nat) : a ≤ Nat.max a b := by
  simp only [Nat.max_def]; split <;> omega

theorem le_max_right' (a b : Nat) : b ≤ Nat.max a b := by
  simp only [Nat.max_def]; split <;> omega

/-- The largest type-parameter index of a list, as the bounds checker folds
it. -/
theorem maxIdx_fold (f : Option Nat → SignatureToken → Option Nat)
    (htp : ∀ acc i, f acc (.typeParameter i) = some (match acc with
      | some a => Nat.max a i
      | none => i))
    (hother : ∀ acc t, (∀ i, t ≠ .typeParameter i) → f acc t = acc) :
    ∀ (l : List SignatureToken) (acc r : Option Nat), l.foldl f acc = r →
      (∀ a, acc = some a → ∃ mx, r = some mx ∧ a ≤ mx) ∧
      (∀ i, SignatureToken.typeParameter i ∈ l → ∃ mx, r = some mx ∧ i ≤ mx)
  | [], acc, r, hr => by
    simp only [List.foldl_nil] at hr
    subst hr
    exact ⟨fun a ha => ⟨a, ha, Nat.le_refl _⟩, by simp⟩
  | t :: ts, acc, r, hr => by
    simp only [List.foldl_cons] at hr
    have ih := maxIdx_fold f htp hother ts (f acc t) r hr
    by_cases ht : ∃ i, t = .typeParameter i
    · obtain ⟨j, rfl⟩ := ht
      rw [htp] at ih
      obtain ⟨mx, hmx, hle⟩ := ih.1 _ rfl
      refine ⟨fun a ha => ⟨mx, hmx, ?_⟩, fun i hi => ?_⟩
      · subst ha; exact Nat.le_trans (le_max_left' a j) hle
      · rcases List.mem_cons.1 hi with hi | hi
        · simp only [SignatureToken.typeParameter.injEq] at hi
          subst hi
          refine ⟨mx, hmx, ?_⟩
          cases acc with
          | none => exact hle
          | some a => exact Nat.le_trans (le_max_right' a i) hle
        · exact ih.2 i hi
    · have hne : ∀ i, t ≠ .typeParameter i := fun i h => ht ⟨i, h⟩
      rw [hother acc t hne] at ih
      refine ⟨ih.1, fun i hi => ?_⟩
      rcases List.mem_cons.1 hi with hi | hi
      · exact absurd hi.symm (hne i)
      · exact ih.2 i hi

theorem checkTypeParametersInSignature_ok {s count : Nat}
    (h : Bounds.checkTypeParametersInSignature m s count = .ok ()) :
    ∃ sig, m.signatures[s]? = some sig ∧
      ∀ i, SignatureToken.typeParameter i ∈ preorderList sig → i < count := by
  unfold Bounds.checkTypeParametersInSignature at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨sig, hsig, h⟩ := h
  refine ⟨sig, hsig, fun i hi => ?_⟩
  generalize hr : List.foldl _ none (preorderList sig) = r at h
  obtain ⟨mx, hmx, hle⟩ := (maxIdx_fold _ (fun acc i => by cases acc <;> rfl)
    (fun acc t ht => by cases t <;> first | rfl | exact absurd rfl (ht _)) _ _ _ hr).2 i hi
  subst hmx
  simp only at h
  split at h
  · simp [reject] at h
  · next hlt => exact Nat.lt_of_le_of_lt hle (Nat.lt_of_not_ge hlt)

/-- A signature is in range when its tokens pass `check_type` and its type
parameters are in scope. -/
theorem signatureInRange (hsigs : ∀ sig ∈ m.signatures.toList, ∀ t ∈ sig,
      Bounds.checkType m t = .ok ()) {s tp : Nat}
    (h : Bounds.checkTypeParametersInSignature m s tp = .ok ()) : m.SignatureInRange tp s := by
  obtain ⟨sig, hsig, htp⟩ := checkTypeParametersInSignature_ok h
  refine ⟨sig, hsig, fun t ht r hr => ?_⟩
  have hct := checkType_ok (hsigs sig (getElem?_mem_toList hsig) t ht)
  have href := refs_preorder t r hr
  cases r with
  | structHandle i =>
    rcases href with href | ⟨args, href⟩
    · exact (hct _ href).1 i rfl
    · exact (hct _ href).2 i args rfl
  | typeParameter i =>
    exact htp i (mem_preorderList.2 ⟨t, ht, href⟩)

/-- A token is in range when it passes `check_type` and its type parameters
are in scope. -/
theorem tokenInRange {t : SignatureToken} {tp : Nat} (hct : Bounds.checkType m t = .ok ())
    (htp : Bounds.checkTypeParametersInTy t tp = .ok ()) : m.TokenInRange tp t := by
  intro r hr
  have hc := checkType_ok hct
  have href := refs_preorder t r hr
  cases r with
  | structHandle i =>
    rcases href with href | ⟨args, href⟩
    · exact (hc _ href).1 i rfl
    · exact (hc _ href).2 i args rfl
  | typeParameter i => exact checkTypeParametersInTy_ok htp i href

end Checks


/-! ## The bounds checker, table by table -/

/-- The steps of `verify_bounds`, each having succeeded. -/
structure BoundsSteps (m : CompiledModule) : Prop where
  sigs : m.signatures.toList.forM (fun sig => sig.forM (Bounds.checkType m)) = .ok ()
  moduleHandles : m.moduleHandles.toList.forM (Bounds.checkModuleHandle m) = .ok ()
  self : checkIdx m.moduleHandles.size m.selfModuleHandleIdx = .ok ()
  structHandles : m.structHandles.toList.forM (fun h => do
    checkIdx m.moduleHandles.size h.module
    checkIdx m.identifiers.size h.name) = .ok ()
  functionHandles : m.functionHandles.toList.forM (Bounds.checkFunctionHandle m) = .ok ()
  fieldHandles : m.fieldHandles.toList.forM (Bounds.checkFieldHandle m) = .ok ()
  friends : m.friendDecls.toList.forM (Bounds.checkModuleHandle m) = .ok ()
  structDefInsts : m.structDefInstantiations.toList.forM (fun i => do
    checkIdx m.structDefs.size i.def_
    checkIdx m.signatures.size i.typeParameters) = .ok ()
  functionInsts : m.functionInstantiations.toList.forM (fun i => do
    checkIdx m.functionHandles.size i.handle
    checkIdx m.signatures.size i.typeParameters) = .ok ()
  fieldInsts : m.fieldInstantiations.toList.forM (fun i => do
    checkIdx m.fieldHandles.size i.handle
    checkIdx m.signatures.size i.typeParameters) = .ok ()
  structDefs : m.structDefs.toList.forM (Bounds.checkStructDef m) = .ok ()
  variantFieldHandles : m.variantFieldHandles.toList.forM (Bounds.checkVariantFieldHandle m) = .ok ()
  structVariantHandles : m.structVariantHandles.toList.forM (Bounds.checkStructVariantHandle m) =
    .ok ()
  variantFieldInsts : m.variantFieldInstantiations.toList.forM (fun i => do
    checkIdx m.variantFieldHandles.size i.handle
    checkIdx m.signatures.size i.typeParameters) = .ok ()
  structVariantInsts : m.structVariantInstantiations.toList.forM (fun i => do
    checkIdx m.structVariantHandles.size i.handle
    checkIdx m.signatures.size i.typeParameters) = .ok ()
  functionDefs : m.functionDefs.toList.forM (Bounds.checkFunctionDef m) = .ok ()

theorem verifyBounds_steps {m : CompiledModule} (hb : verifyBounds m = .ok ()) : BoundsSteps m := by
  unfold verifyBounds at hb
  simp only at hb
  split at hb
  · simp [reject, bind, Except.bind] at hb
  obtain ⟨h1, hb⟩ := seq_ok2 hb
  obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨h3, hb⟩ := seq_ok2 hb
  obtain ⟨h4, hb⟩ := seq_ok2 hb
  obtain ⟨h5, hb⟩ := seq_ok2 hb
  obtain ⟨h6, hb⟩ := seq_ok2 hb
  obtain ⟨h7, hb⟩ := seq_ok2 hb
  obtain ⟨h8, hb⟩ := seq_ok2 hb
  obtain ⟨h9, hb⟩ := seq_ok2 hb
  obtain ⟨h10, hb⟩ := seq_ok2 hb
  obtain ⟨h11, hb⟩ := seq_ok2 hb
  obtain ⟨h12, hb⟩ := seq_ok2 hb
  obtain ⟨h13, hb⟩ := seq_ok2 hb
  obtain ⟨h14, hb⟩ := seq_ok2 hb
  obtain ⟨h15, hb⟩ := seq_ok2 hb
  obtain ⟨h16, hb⟩ := seq_ok2 hb
  exact ⟨h1, h3, h4, h5, h6, h7, h8, h9, h10, h11, h12, h13, h14, h15, h16, hb⟩

theorem pair_ok {a b : Nat} {x y : Nat} (h : (do checkIdx a x; checkIdx b y : VResult Unit) = .ok ()) :
    x < a ∧ y < b :=
  ⟨checkIdx_ok (seq_ok2 h).1, checkIdx_ok (seq_ok2 h).2⟩


section Tables

variable {m : CompiledModule}

theorem fieldHandle_ok {h : FieldHandle} (hc : Bounds.checkFieldHandle m h = .ok ()) :
    ∃ n, m.declaredFieldCount h.owner = some n ∧ h.field < n := by
  unfold Bounds.checkFieldHandle at hc
  obtain ⟨h1, hc⟩ := seq_ok2 hc
  obtain ⟨sd, hsd⟩ := getElem?_of_lt (checkIdx_ok h1)
  rw [hsd] at hc
  simp only at hc
  split at hc
  · simp [reject] at hc
  · next hlt =>
    simp only [CompiledModule.declaredFieldCount, hsd, bind, Option.bind_some]
    cases hfi : sd.fieldInformation with
    | declared fields =>
      rw [hfi] at hlt
      exact ⟨fields.length, rfl, by simpa [fieldCount] using hlt⟩
    | native => rw [hfi] at hlt; simp [fieldCount] at hlt
    | declaredVariants vs => rw [hfi] at hlt; simp [fieldCount] at hlt

theorem variantIndex_ok {sd : StructDefinition} {v : Nat}
    (hc : Bounds.checkVariantIndex sd v = .ok ()) :
    ∃ vs, sd.fieldInformation = .declaredVariants vs ∧ v < vs.length := by
  unfold Bounds.checkVariantIndex at hc
  split at hc
  · simp [reject] at hc
  · next hlt =>
    cases hfi : sd.fieldInformation with
    | declaredVariants vs => rw [hfi] at hlt; exact ⟨vs, rfl, by simpa [variantCount] using hlt⟩
    | native => rw [hfi] at hlt; simp [variantCount] at hlt
    | declared fs => rw [hfi] at hlt; simp [variantCount] at hlt

theorem variantFieldHandle_ok {h : VariantFieldHandle}
    (hc : Bounds.checkVariantFieldHandle m h = .ok ()) :
    ∃ vs, m.variants h.structIndex = some vs ∧
      ∀ v ∈ h.variants, ∃ vd, vs[v]? = some vd ∧ h.field < vd.fields.length := by
  unfold Bounds.checkVariantFieldHandle at hc
  obtain ⟨-, hc⟩ := seq_ok2 hc
  simp only at hc
  split at hc
  · simp [reject, bind, Except.bind] at hc
  · next hne =>
    simp only [bind_ok, lookup_ok] at hc
    obtain ⟨sd, hsd, hc⟩ := hc
    have hv := forM_ok hc
    cases hvs : h.variants with
    | nil => simp [hvs] at hne
    | cons v₀ rest =>
      rw [hvs] at hv
      obtain ⟨vs, hfi, -⟩ := variantIndex_ok (seq_ok2 (hv v₀ (List.mem_cons_self ..))).1
      refine ⟨vs, by simp [CompiledModule.variants, hsd, hfi], fun v hvm => ?_⟩
      obtain ⟨h1, h2⟩ := seq_ok2 (hv v hvm)
      obtain ⟨vs', hfi', hlt⟩ := variantIndex_ok h1
      rw [hfi] at hfi'
      cases hfi'
      obtain ⟨vd, hvd⟩ : ∃ vd, vs[v]? = some vd := ⟨vs[v], List.getElem?_eq_getElem hlt⟩
      refine ⟨vd, hvd, ?_⟩
      split at h2
      · simp [reject] at h2
      · next hlt2 =>
        rw [hfi] at hlt2
        simp only [fieldCount, hvd] at hlt2
        exact Nat.lt_of_not_ge hlt2

theorem structVariantHandle_ok {h : StructVariantHandle}
    (hc : Bounds.checkStructVariantHandle m h = .ok ()) :
    ∃ vs, m.variants h.structIndex = some vs ∧ h.variant < vs.length := by
  unfold Bounds.checkStructVariantHandle at hc
  obtain ⟨-, hc⟩ := seq_ok2 hc
  simp only [bind_ok, lookup_ok] at hc
  obtain ⟨sd, hsd, hc⟩ := hc
  obtain ⟨vs, hfi, hlt⟩ := variantIndex_ok hc
  exact ⟨vs, by simp [CompiledModule.variants, hsd, hfi], hlt⟩

end Tables


section Defs

variable {m : CompiledModule}
  (hsigs : ∀ sig ∈ m.signatures.toList, ∀ t ∈ sig, Bounds.checkType m t = .ok ())

include hsigs in
/-- Each reference of an instruction that passes the bounds check is in
range. -/
theorem checkInstr_refs {codeLen lc tp : Nat} {i : Bytecode}
    (h : Bounds.checkInstr m codeLen lc tp i = .ok ()) :
    ∀ r ∈ i.indexRefs, m.RefInRange lc codeLen tp r := by
  intro r hr
  cases i <;> simp only [Bytecode.indexRefs, List.mem_cons, List.not_mem_nil, or_false]
    at hr <;> (try subst hr) <;> simp only [Bounds.checkInstr] at h
  all_goals first
    | exact checkIdx_ok h
    | (obtain ⟨h1, h2⟩ := seq_ok2 h
       obtain ⟨x, hx⟩ := getElem?_of_lt (checkIdx_ok h1)
       rw [hx] at h2
       exact ⟨x, hx, signatureInRange hsigs h2⟩)
    | (obtain ⟨h1, h2⟩ := seq_ok2 h; exact signatureInRange hsigs h2)
    | (simp at hr)

end Defs


section Defs2

variable {m : CompiledModule}

theorem fieldDef_ok {tp : Nat} {f : FieldDefinition} (h : Bounds.checkFieldDef m tp f = .ok ()) :
    f.name < m.identifiers.size ∧ m.TokenInRange tp f.signature := by
  unfold Bounds.checkFieldDef at h
  obtain ⟨h1, h⟩ := seq_ok2 h
  obtain ⟨h2, h3⟩ := seq_ok2 h
  exact ⟨checkIdx_ok h1, tokenInRange h2 h3⟩

theorem structDef_ok {sd : StructDefinition} (h : Bounds.checkStructDef m sd = .ok ()) :
    ∃ sh, m.structHandles[sd.structHandle]? = some sh ∧
      (∀ f ∈ CompiledModule.StructFieldInformation.allFields sd.fieldInformation,
        f.name < m.identifiers.size ∧ m.TokenInRange sh.typeParameters.length f.signature) ∧
      (∀ vs, sd.fieldInformation = .declaredVariants vs → ∀ v ∈ vs, v.name < m.identifiers.size) := by
  unfold Bounds.checkStructDef at h
  obtain ⟨h1, h⟩ := seq_ok2 h
  obtain ⟨sh, hsh⟩ := getElem?_of_lt (checkIdx_ok h1)
  simp only [hsh] at h
  refine ⟨sh, hsh, ?_⟩
  cases hfi : sd.fieldInformation with
  | native => exact ⟨by simp [CompiledModule.StructFieldInformation.allFields], by simp⟩
  | declared fields =>
    rw [hfi] at h
    refine ⟨fun f hf => fieldDef_ok (forM_ok h f (by simpa
      [CompiledModule.StructFieldInformation.allFields] using hf)), by simp⟩
  | declaredVariants vs =>
    rw [hfi] at h
    simp only at h
    obtain ⟨h2, -⟩ := seq_ok2 h
    have hv := forM_ok h2
    refine ⟨fun f hf => ?_, fun vs' hvs v hv' => ?_⟩
    · simp only [CompiledModule.StructFieldInformation.allFields, List.mem_flatMap] at hf
      obtain ⟨v, hvm, hfm⟩ := hf
      exact fieldDef_ok (forM_ok (seq_ok2 (hv v hvm)).2 f hfm)
    · simp only [StructFieldInformation.declaredVariants.injEq] at hvs
      subst hvs
      exact checkIdx_ok (seq_ok2 (hv v hv')).1

theorem functionDef_ok
    (hsigs : ∀ sig ∈ m.signatures.toList, ∀ t ∈ sig, Bounds.checkType m t = .ok ())
    {d : FunctionDefinition} (h : Bounds.checkFunctionDef m d = .ok ()) :
    ∃ fh, m.functionHandles[d.function]? = some fh ∧
      (∀ a ∈ d.acquiresGlobalResources, a < m.structDefs.size) ∧
      ∀ code, d.code = some code →
        ∃ params locals, m.signatures[fh.parameters]? = some params ∧
          m.signatures[code.locals]? = some locals ∧
          m.SignatureInRange fh.typeParameters.length code.locals ∧
          ∀ instr ∈ code.code.toList, ∀ r ∈ instr.indexRefs,
            m.RefInRange (params.length + locals.length) code.code.size
              fh.typeParameters.length r := by
  unfold Bounds.checkFunctionDef at h
  obtain ⟨h1, h⟩ := seq_ok2 h
  obtain ⟨h2, h⟩ := seq_ok2 h
  obtain ⟨fh, hfh⟩ := getElem?_of_lt (checkIdx_ok h1)
  refine ⟨fh, hfh, fun a ha => checkIdx_ok (forM_ok h2 a ha), fun code hcode => ?_⟩
  rw [hcode] at h
  simp only [hfh, pure, Except.pure, bind, Except.bind] at h
  cases hps : m.signatures[fh.parameters]? with
  | none => simp [hps, reject] at h
  | some params =>
    simp only [hps] at h
    unfold Bounds.checkCode at h
    obtain ⟨-, h⟩ := seq_ok2 h
    cases hls : m.signatures[code.locals]? with
    | none => simp [hls, reject, bind, Except.bind] at h
    | some locals =>
      simp only [hls, pure_bind] at h
      split at h
      · simp [reject, bind, Except.bind] at h
      · obtain ⟨h3, h4⟩ := seq_ok2 h
        refine ⟨params, locals, rfl, rfl, signatureInRange hsigs h3, fun instr hi r hr => ?_⟩
        rw [Nat.add_comm]
        exact checkInstr_refs hsigs (forM_ok h4 instr hi) r hr

end Defs2

/-- A constant type has no references: it is primitive or a vector of
those. -/
theorem refs_of_validConstant : ∀ {t : SignatureToken}, isValidForConstant t = true → t.refs = []
  | .vector e, h => by
    simp only [isValidForConstant] at h
    simpa [SignatureToken.refs] using refs_of_validConstant h
  | .bool, _ | .u8, _ | .u16, _ | .u32, _ | .u64, _ | .u128, _ | .u256, _ | .address, _
  | .i8, _ | .i16, _ | .i32, _ | .i64, _ | .i128, _ | .i256, _ => rfl
  | .signer, h | .function .., h | .struct _, h | .structInstantiation .., h | .reference _, h
  | .mutableReference _, h | .typeParameter _, h => by simp [isValidForConstant] at h


theorem moduleHandle_ok {m : CompiledModule} {mh : ModuleHandle}
    (h : Bounds.checkModuleHandle m mh = .ok ()) :
    mh.address < m.addressIdentifiers.size ∧ mh.name < m.identifiers.size := by
  unfold Bounds.checkModuleHandle at h
  exact ⟨checkIdx_ok (seq_ok2 h).1, checkIdx_ok (seq_ok2 h).2⟩

theorem validConstant_of_verify {c : Constant} (h : verifyConstant c = .ok ()) :
    isValidForConstant c.type = true := by
  unfold verifyConstant at h
  split at h
  · simp [reject] at h
  · next hv => simpa using hv

/-- **L1** for a module the model accepts. -/
theorem indexIntegrity_of_accepted {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) : m.IndexIntegrity := by
  obtain ⟨hb, -, hc, -⟩ := verifyModule_ok h
  have s := verifyBounds_steps hb
  have hsigs : ∀ sig ∈ m.signatures.toList, ∀ t ∈ sig, Bounds.checkType m t = .ok () :=
    fun sig hsig t ht => forM_ok (forM_ok s.sigs sig hsig) t ht
  refine ⟨checkIdx_ok s.self, fun mh hmh => ?_, fun sh hsh => pair_ok (forM_ok s.structHandles sh hsh),
    fun fh hfh => ?_, fun fh hfh => fieldHandle_ok (forM_ok s.fieldHandles fh hfh),
    fun vh hvh => variantFieldHandle_ok (forM_ok s.variantFieldHandles vh hvh),
    fun vh hvh => structVariantHandle_ok (forM_ok s.structVariantHandles vh hvh),
    fun x hx => (pair_ok (forM_ok s.structDefInsts x hx)).1,
    fun x hx => (pair_ok (forM_ok s.functionInsts x hx)).1,
    fun x hx => (pair_ok (forM_ok s.fieldInsts x hx)).1,
    fun x hx => (pair_ok (forM_ok s.structVariantInsts x hx)).1,
    fun x hx => (pair_ok (forM_ok s.variantFieldInsts x hx)).1,
    fun c hcm => ?_, fun d hd => ?_, fun d hd vs hvs => ?_,
    fun d hd => functionDef_ok hsigs (forM_ok s.functionDefs d hd)⟩
  · rcases List.mem_append.1 hmh with hmh | hmh
    · exact moduleHandle_ok (forM_ok s.moduleHandles mh hmh)
    · exact moduleHandle_ok (forM_ok s.friends mh hmh)
  · have hf := forM_ok s.functionHandles fh hfh
    unfold Bounds.checkFunctionHandle at hf
    obtain ⟨h1, hf⟩ := seq_ok2 hf
    obtain ⟨h2, hf⟩ := seq_ok2 hf
    obtain ⟨-, hf⟩ := seq_ok2 hf
    obtain ⟨-, hf⟩ := seq_ok2 hf
    obtain ⟨h5, h6⟩ := seq_ok2 hf
    exact ⟨checkIdx_ok h1, checkIdx_ok h2, signatureInRange hsigs h5, signatureInRange hsigs h6⟩
  · intro r hr
    rw [refs_of_validConstant (validConstant_of_verify (forM_ok hc c hcm))] at hr
    simp at hr
  · obtain ⟨sh, hsh, hf, -⟩ := structDef_ok (forM_ok s.structDefs d hd)
    exact ⟨sh, hsh, hf⟩
  · exact (structDef_ok (forM_ok s.structDefs d hd)).choose_spec.2.2 vs hvs

/-! ## L8 -/

theorem isValidForConstant_eq : ∀ t : SignatureToken,
    isValidForConstant t = t.isConstantType
  | .vector e => by simp [isValidForConstant, SignatureToken.isConstantType, isValidForConstant_eq e]
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 | .address | .i8 | .i16 | .i32 | .i64
  | .i128 | .i256 | .signer | .function .. | .struct _ | .structInstantiation .. | .reference _
  | .mutableReference _ | .typeParameter _ => rfl

/-- **L8** for a module the model accepts. -/
theorem constantsWellFormed_of_accepted {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) : m.ConstantsWellFormed := by
  obtain ⟨-, -, hc, -⟩ := verifyModule_ok h
  intro c hcm
  obtain ⟨hvalid, hdecode⟩ := constants_decode hc c hcm
  exact ⟨by rw [← isValidForConstant_eq]; exact hvalid, hdecode⟩

/-! ## L9 -/

theorem verifySignatures_code {config : VerifierConfig} {m : CompiledModule}
    (h : verifySignatures config m = .ok ()) :
    ∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code →
      ∃ fh, m.functionHandles[d.function]? = some fh ∧
        SignatureV2.verifyCode config m fh.typeParameters code = .ok () := by
  unfold verifySignatures SignatureV2.verify at h
  split at h
  · simp [reject, bind, Except.bind] at h
  · simp only [bind_ok] at h
    obtain ⟨-, -, -, -, -, -, -, -, -, -, -, -, -, -, u, h, -⟩ := h
    cases u
    intro d hd code hcode
    have := forM_ok h d hd
    simp only [hcode, bind_ok, lookup_ok] at this
    exact this

/-- The signature checker requires the signature of a vector instruction to be
a single type. -/
theorem verifyCode_vector {config : VerifierConfig} {m : CompiledModule}
    {abilities : List AbilitySet} {code : CodeUnit}
    (h : SignatureV2.verifyCode config m abilities code = .ok ()) {s n : Nat}
    (hi : Bytecode.vecPack s n ∈ code.code.toList ∨ Bytecode.vecUnpack s n ∈ code.code.toList ∨
      Bytecode.vecLen s ∈ code.code.toList ∨ Bytecode.vecImmBorrow s ∈ code.code.toList ∨
      Bytecode.vecMutBorrow s ∈ code.code.toList ∨ Bytecode.vecPushBack s ∈ code.code.toList ∨
      Bytecode.vecPopBack s ∈ code.code.toList ∨ Bytecode.vecSwap s ∈ code.code.toList) :
    m.SingleType s := by
  unfold SignatureV2.verifyCode at h
  simp only [bind_ok] at h
  obtain ⟨_, -, _, -, h⟩ := h
  rcases hi with hi | hi | hi | hi | hi | hi | hi | hi <;>
  · have := forM_ok h _ hi
    simp only [bind_ok, lookup_ok] at this
    obtain ⟨args, hargs, this⟩ := this
    split at this
    · exact ⟨_, hargs⟩
    · simp [reject] at this

theorem checkStructOp_ok {m : CompiledModule} {d : Nat}
    (h : InstructionConsistency.checkStructOp m d false = .ok ()) :
    m.structTypeParamCount d = some 0 := by
  unfold InstructionConsistency.checkStructOp at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨sd, hsd, sh, hsh, h⟩ := h
  split at h
  · simp [reject] at h
  · next hne =>
    simp only [beq_iff_eq] at hne
    cases hp : sh.typeParameters with
    | nil => simp [CompiledModule.structTypeParamCount, hsd, hsh, hp]
    | cons _ _ => simp [hp] at hne

theorem checkStructOp_generic {m : CompiledModule} {d : Nat}
    (h : InstructionConsistency.checkStructOp m d true = .ok ()) :
    ∃ n, m.structTypeParamCount d = some n ∧ 0 < n := by
  unfold InstructionConsistency.checkStructOp at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨sd, hsd, sh, hsh, h⟩ := h
  split at h
  · simp [reject] at h
  · next hne =>
    cases hp : sh.typeParameters with
    | nil => simp [hp] at hne
    | cons _ _ =>
      exact ⟨sh.typeParameters.length, by simp [CompiledModule.structTypeParamCount, hsd, hsh],
        by simp [hp]⟩

theorem checkFunctionOp_generic {m : CompiledModule} {h : Nat}
    (hc : InstructionConsistency.checkFunctionOp m h true = .ok ()) :
    ∃ n, m.functionTypeParamCount h = some n ∧ 0 < n := by
  unfold InstructionConsistency.checkFunctionOp at hc
  simp only [bind_ok, lookup_ok] at hc
  obtain ⟨fh, hfh, hc⟩ := hc
  split at hc
  · simp [reject] at hc
  · next hne =>
    cases hp : fh.typeParameters with
    | nil => simp [hp] at hne
    | cons _ _ =>
      exact ⟨fh.typeParameters.length, by simp [CompiledModule.functionTypeParamCount, hfh],
        by simp [hp]⟩

/-- `verify_function_instantiation_contextless`: one type argument per type
parameter of the callee. -/
theorem functionInstantiation_ok {config : VerifierConfig} {m : CompiledModule} {i : Nat}
    {cs : SignatureV2.Constraints} (h : SignatureV2.functionInstantiation config m i = .ok cs) :
    ∃ fi fh args, m.functionInstantiations[i]? = some fi ∧
      m.functionHandles[fi.handle]? = some fh ∧ m.signatures[fi.typeParameters]? = some args ∧
      args.length = fh.typeParameters.length ∧
      ∃ css, (args.zip fh.typeParameters).mapM
        (fun (x : SignatureToken × AbilitySet) => SignatureV2.typeArg config m x.1 x.2) = .ok css ∧
        cs = css.flatten := by
  unfold SignatureV2.functionInstantiation at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨fi, hfi, fh, hfh, args, hargs, h⟩ := h
  split at h
  · simp [reject, bind, Except.bind] at h
  · next hne =>
    simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨css, hcss, rfl⟩ := h
    exact ⟨fi, fh, args, hfi, hfh, hargs, by simp at hne; omega, css, hcss, rfl⟩

/-- `verify_struct_type_params`: one type argument per type parameter of the
struct, which has the abilities `req` asks. -/
theorem structTypeParams_ok {config : VerifierConfig} {m : CompiledModule} {req : AbilitySet}
    {d a : Nat} {cs : SignatureV2.Constraints}
    (h : SignatureV2.structTypeParams config m req d a = .ok cs) :
    ∃ sd sh args, m.structDefs[d]? = some sd ∧ m.structHandles[sd.structHandle]? = some sh ∧
      m.signatures[a]? = some args ∧ args.length = sh.typeParameters.length ∧
      req.subset sh.abilities = true ∧
      ∃ css, (args.zip sh.typeParameters).mapM
        (fun (x : SignatureToken × StructTypeParameter) => SignatureV2.typeArg config m x.1
          (if x.2.isPhantom then x.2.constraints
            else x.2.constraints.union (Model.requires req))) = .ok css ∧
        cs = css.flatten := by
  unfold SignatureV2.structTypeParams at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨sd, hsd, sh, hsh, args, hargs, h⟩ := h
  split at h
  · simp [reject, bind, Except.bind] at h
  · next hne =>
    split at h
    · simp [reject, bind, Except.bind] at h
    · next hsub =>
      simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at h
      obtain ⟨css, hcss, rfl⟩ := h
      exact ⟨sd, sh, args, hsd, hsh, hargs, by simp at hne; omega,
        by simpa using hsub, css, hcss, rfl⟩

/-- The signature checker's check of one instruction of a code unit. -/
theorem verifyCode_instr {config : VerifierConfig} {m : CompiledModule}
    {abilities : List AbilitySet} {code : CodeUnit}
    (h : SignatureV2.verifyCode config m abilities code = .ok ()) {i : Bytecode}
    (hi : i ∈ code.code.toList) :
    (match i with
      | .callGeneric x | .packClosureGeneric x _ => ∃ cs,
        SignatureV2.functionInstantiation config m x = .ok cs ∧
        SignatureV2.checkInContext cs (SignatureV2.listContext abilities) = .ok ()
      | .packGeneric x | .unpackGeneric x => ∃ cs,
        SignatureV2.structInstantiation config m x SignatureV2.emptySet = .ok cs ∧
        SignatureV2.checkInContext cs (SignatureV2.listContext abilities) = .ok ()
      | .packVariantGeneric x | .unpackVariantGeneric x | .testVariantGeneric x => ∃ cs,
        SignatureV2.structVariantInstantiation config m x SignatureV2.emptySet = .ok cs ∧
        SignatureV2.checkInContext cs (SignatureV2.listContext abilities) = .ok ()
      | .existsGeneric x | .moveFromGeneric x | .moveToGeneric x | .immBorrowGlobalGeneric x
      | .mutBorrowGlobalGeneric x => ∃ cs,
        SignatureV2.structInstantiation config m x ⟨false, false, false, true⟩ = .ok cs ∧
        SignatureV2.checkInContext cs (SignatureV2.listContext abilities) = .ok ()
      | .immBorrowFieldGeneric x | .mutBorrowFieldGeneric x => ∃ cs,
        SignatureV2.fieldInstantiation config m x = .ok cs ∧
        SignatureV2.checkInContext cs (SignatureV2.listContext abilities) = .ok ()
      | .immBorrowVariantFieldGeneric x | .mutBorrowVariantFieldGeneric x => ∃ cs,
        SignatureV2.variantFieldInstantiation config m x = .ok cs ∧
        SignatureV2.checkInContext cs (SignatureV2.listContext abilities) = .ok ()
      | _ => True) := by
  unfold SignatureV2.verifyCode at h
  simp only [bind_ok] at h
  obtain ⟨_, -, _, -, h⟩ := h
  have := forM_ok h _ hi
  cases i <;> first | trivial | (simp only [bind_ok] at this; exact this)

/-! ## Closure instructions -/

/-- The fold of `max_captured` up to `n`: the highest captured position below
`n`, if any. -/
theorem maxCaptured_fold (mask : BitVec 64) : ∀ (n i : Nat), i < n → mask.getLsbD i = true →
    ∃ mx, (List.range n).foldl (fun acc j => if mask.getLsbD j then some j else acc) none =
      some mx ∧ i ≤ mx
  | 0, _, hi, _ => absurd hi (Nat.not_lt_zero _)
  | n + 1, i, hi, hb => by
    rw [List.range_succ, List.foldl_append, List.foldl_cons, List.foldl_nil]
    by_cases hn : mask.getLsbD n = true
    · exact ⟨n, by simp [hn], by omega⟩
    · have hin : i < n := by
        rcases Nat.lt_or_eq_of_le (Nat.le_of_lt_succ hi) with h' | h'
        · exact h'
        · subst h'; exact absurd hb hn
      obtain ⟨mx, hmx, hle⟩ := maxCaptured_fold mask n i hin hb
      exact ⟨mx, by simp [hn, hmx], hle⟩

/-- A captured position is at most `max_captured`. -/
theorem maxCaptured_ge {mask : BitVec 64} {i : Nat} (hb : mask.getLsbD i = true) :
    ∃ mx, maxCaptured mask = some mx ∧ i ≤ mx := by
  have hi : i < 64 := by
    rcases Nat.lt_or_ge i 64 with h' | h'
    · exact h'
    · rw [BitVec.getLsbD_of_ge mask i h'] at hb; cases hb
  exact maxCaptured_fold mask 64 i hi hb

/-- `check_closure_mask`: a captured position is a parameter of the target. -/
theorem checkClosureMask_ok {m : CompiledModule} {h : Nat} {mask : BitVec 64}
    (hc : InstructionConsistency.checkClosureMask m h mask = .ok ()) :
    ∀ fh params, m.functionHandles[h]? = some fh → m.signatures[fh.parameters]? = some params →
      ∀ i, mask.getLsbD i = true → i < params.length := by
  intro fh params hfh hp i hb
  unfold InstructionConsistency.checkClosureMask at hc
  simp only [bind_ok, lookup_ok] at hc
  obtain ⟨fh', hfh', sig, hsig, hc⟩ := hc
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  rw [hp, Option.some.injEq] at hsig
  subst hsig
  obtain ⟨mx, hmx, hle⟩ := maxCaptured_ge hb
  rw [hmx] at hc
  simp only at hc
  split at hc
  · simp [reject] at hc
  · omega

/-- What instruction consistency and the signature checker establish about
every closure instruction of a module the model accepts. -/
theorem closureOk_of_accepted {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) {d : FunctionDefinition}
    (hd : d ∈ m.functionDefs.toList) {code : CodeUnit} (hcode : d.code = some code) :
    ∀ i ∈ code.code.toList, ClosureOk m i := by
  intro i hi
  have hic := forM_ok (verifyModule_instructions h) d hd
  simp only [hcode] at hic
  have hci := forM_ok hic i hi
  obtain ⟨fh, -, hvc⟩ := verifySignatures_code (verifyModule_signatures h) d hd code hcode
  cases i <;> simp only [ClosureOk] <;> try trivial
  case packClosure hh mask =>
    simp only [InstructionConsistency.checkInstr, bind_ok] at hci
    obtain ⟨u, -, hci⟩ := hci
    exact checkClosureMask_ok hci
  case packClosureGeneric x mask =>
    simp only [InstructionConsistency.checkInstr, bind_ok, lookup_ok] at hci
    obtain ⟨fi, hfi, u, -, hci⟩ := hci
    intro fi' fh' params hfi' hfh' hp
    rw [hfi, Option.some.injEq] at hfi'
    subst hfi'
    exact checkClosureMask_ok hci fh' params hfh' hp
  case callClosure s =>
    unfold SignatureV2.verifyCode at hvc
    simp only [bind_ok] at hvc
    obtain ⟨_, -, _, -, hvc⟩ := hvc
    have := forM_ok hvc _ hi
    simp only [bind_ok, lookup_ok] at this
    obtain ⟨sig, hsig, this⟩ := this
    split at this
    · next ps rs a => exact ⟨ps, rs, a, hsig⟩
    · simp [reject] at this

/-- **L4** for every function of a module the model accepts. -/
theorem stackDiscipline_of_accepted {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) {d : FunctionDefinition}
    (hd : d ∈ m.functionDefs.toList) {code : CodeUnit} (hcode : d.code = some code) :
    ∃ fh, m.functionHandles[d.function]? = some fh ∧
      StackDiscipline m fh code.code VerifierConfig.production.maxValueStackSize := by
  obtain ⟨index, hidx⟩ := List.mem_iff_getElem?.1 hd
  rw [Array.getElem?_toList] at hidx
  obtain ⟨-, -, -, hcu⟩ := verifyModule_ok h
  obtain ⟨n, hn⟩ := verifyCodeUnits_fn hcu hidx
  obtain ⟨handle, hv⟩ := verifyFunction_parts hcode hn
  exact stackDiscipline_of_verifyFunction hcode (closureOk_of_accepted h hd hcode) hn

/-- **L9** for a module the model accepts. Its code is in the supported subset.
Instruction consistency checks that a direct call, `Pack`, `Unpack` and the
variant forms name an entity without type parameters; type safety that
`Pack` and `Unpack` name a struct with declared fields; and the signature
checker that a vector instruction's signature is a single type. -/
theorem instructionConsistency_of_accepted {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) : m.InstructionConsistency := by
  intro d hd code hcode i hi
  obtain ⟨index, hidx⟩ := List.mem_iff_getElem?.1 hd
  rw [Array.getElem?_toList] at hidx
  obtain ⟨-, -, -, hcu⟩ := verifyModule_ok h
  obtain ⟨n, hn⟩ := verifyCodeUnits_fn hcu hidx
  obtain ⟨handle, hv⟩ := verifyFunction_parts hcode hn
  obtain ⟨view, st, st', hty⟩ := typed_of_verified hv i hi
  have hsub := typeInstr_subset hty
  have hic := forM_ok (verifyModule_instructions h) d hd
  simp only [hcode] at hic
  have hci := forM_ok hic i hi
  obtain ⟨fh, -, hvc⟩ := verifySignatures_code (verifyModule_signatures h) d hd code hcode
  cases i <;> simp [Bytecode.inSubsetNG, Bytecode.inCore, Bytecode.inDataByValue,
    Bytecode.inReferences, genericForm] at hsub <;>
    simp only [Bytecode.OperandsConsistent]
  case call hh =>
    simp only [InstructionConsistency.checkInstr, InstructionConsistency.checkFunctionOp,
      bind_ok, lookup_ok] at hci
    obtain ⟨fh, hfh, hc⟩ := hci
    split at hc
    · simp [reject] at hc
    · next hne =>
      simp only [beq_iff_eq] at hne
      simp [CompiledModule.functionTypeParamCount, hfh]
      cases hfp : fh.typeParameters with
      | nil => rfl
      | cons _ _ => simp [hfp] at hne
  case pack d' =>
    refine ⟨checkStructOp_ok hci, ?_⟩
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨sd, hsd, hty⟩ := hty
    obtain ⟨fs, -, -, hfs, -⟩ := typePack_ok hty
    rcases fieldsSignature_ok hfs with ⟨-, fields, hfi, -⟩ | ⟨_, _, _, hv, -⟩
    · simp [CompiledModule.DeclaredStruct, CompiledModule.declaredFieldCount, hsd, hfi]
    · cases hv
  case unpack d' =>
    refine ⟨checkStructOp_ok hci, ?_⟩
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨sd, hsd, hty⟩ := hty
    obtain ⟨fs, -, -, hfs, -⟩ := typeUnpack_ok hty
    rcases fieldsSignature_ok hfs with ⟨-, fields, hfi, -⟩ | ⟨_, _, _, hv, -⟩
    · simp [CompiledModule.DeclaredStruct, CompiledModule.declaredFieldCount, hsd, hfi]
    · cases hv
  case packVariant hh | unpackVariant hh =>
    simp only [InstructionConsistency.checkInstr, InstructionConsistency.checkVariantOp,
      bind_ok, lookup_ok] at hci
    obtain ⟨vh, hvh, hc⟩ := hci
    simp [CompiledModule.variantTypeParamCount, hvh, checkStructOp_ok hc]
  case vecPack s k => exact verifyCode_vector (n := k) hvc (Or.inl hi)
  case vecUnpack s k => exact verifyCode_vector (n := k) hvc (Or.inr (Or.inl hi))
  case vecLen s => exact verifyCode_vector (n := 0) hvc (by simp [hi])
  case vecImmBorrow s => exact verifyCode_vector (n := 0) hvc (by simp [hi])
  case vecMutBorrow s => exact verifyCode_vector (n := 0) hvc (by simp [hi])
  case vecPushBack s => exact verifyCode_vector (n := 0) hvc (by simp [hi])
  case vecPopBack s => exact verifyCode_vector (n := 0) hvc (by simp [hi])
  case vecSwap s => exact verifyCode_vector (n := 0) hvc (by simp [hi])
  case mutBorrowField hh | immBorrowField hh =>
    simp only [InstructionConsistency.checkInstr, bind_ok, lookup_ok] at hci
    obtain ⟨fh, hfh, hc⟩ := hci
    simp [CompiledModule.fieldTypeParamCount, hfh, checkStructOp_ok hc]
  case mutBorrowVariantField hh | immBorrowVariantField hh =>
    simp only [InstructionConsistency.checkInstr, bind_ok, lookup_ok] at hci
    obtain ⟨fh, hfh, hc⟩ := hci
    simp [CompiledModule.variantFieldTypeParamCount, hfh, checkStructOp_ok hc]
  case testVariant hh =>
    simp only [InstructionConsistency.checkInstr, InstructionConsistency.checkVariantOp,
      bind_ok, lookup_ok] at hci
    obtain ⟨vh, hvh, hc⟩ := hci
    simp [CompiledModule.variantTypeParamCount, hvh, checkStructOp_ok hc]
  case mutBorrowGlobal d' | immBorrowGlobal d' | exists_ d' | moveFrom d' | moveTo d' =>
    exact checkStructOp_ok hci
  case callGeneric x =>
    simp only [InstructionConsistency.checkInstr, bind_ok, lookup_ok] at hci
    obtain ⟨fi, hfi, hc⟩ := hci
    obtain ⟨n, hn, hpos⟩ := checkFunctionOp_generic hc
    obtain ⟨cs, hcs, -⟩ := verifyCode_instr hvc hi
    obtain ⟨fi', fh, args, hfi', hfh, hargs, hlen, -⟩ := functionInstantiation_ok hcs
    rw [hfi, Option.some.injEq] at hfi'
    subst hfi'
    refine ⟨fi, hfi, n, args, hn, hpos, hargs, ?_⟩
    simp [CompiledModule.functionTypeParamCount, hfh] at hn
    omega
  case packClosure hh mask =>
    simp only [InstructionConsistency.checkInstr, bind_ok] at hci
    obtain ⟨u, hc, -⟩ := hci
    cases u
    simp only [InstructionConsistency.checkFunctionOp, bind_ok, lookup_ok] at hc
    obtain ⟨fh, hfh, hc⟩ := hc
    split at hc
    · simp [reject] at hc
    · next hne =>
      simp only [beq_iff_eq] at hne
      simp [CompiledModule.functionTypeParamCount, hfh]
      cases hfp : fh.typeParameters with
      | nil => rfl
      | cons _ _ => simp [hfp] at hne
  case packClosureGeneric x mask =>
    simp only [InstructionConsistency.checkInstr, bind_ok, lookup_ok] at hci
    obtain ⟨fi, hfi, u, hc, -⟩ := hci
    cases u
    obtain ⟨n, hn, hpos⟩ := checkFunctionOp_generic hc
    obtain ⟨cs, hcs, -⟩ := verifyCode_instr hvc hi
    obtain ⟨fi', fh, args, hfi', hfh, hargs, hlen, -⟩ := functionInstantiation_ok hcs
    rw [hfi, Option.some.injEq] at hfi'
    subst hfi'
    refine ⟨fi, hfi, n, args, hn, hpos, hargs, ?_⟩
    simp [CompiledModule.functionTypeParamCount, hfh] at hn
    omega
  case callClosure s =>
    have hok : ClosureOk m (.callClosure s) := closureOk_of_accepted h hd hcode _ hi
    obtain ⟨ps, rs, a, hs⟩ := hok
    exact ⟨ps, rs, a, hs⟩
  case packGeneric x =>
    simp only [InstructionConsistency.checkInstr, bind_ok, lookup_ok] at hci
    obtain ⟨si, hsi, hc⟩ := hci
    obtain ⟨n, hn, hpos⟩ := checkStructOp_generic hc
    obtain ⟨cs, hcs, -⟩ := verifyCode_instr hvc hi
    unfold SignatureV2.structInstantiation at hcs
    simp only [bind_ok, lookup_ok] at hcs
    obtain ⟨si', hsi', hcs⟩ := hcs
    rw [hsi, Option.some.injEq] at hsi'
    subst hsi'
    obtain ⟨sd, sh, args, hsd, hsh, hargs, hlen, -⟩ := structTypeParams_ok hcs
    refine ⟨si, hsi, ⟨n, args, hn, hpos, hargs, ?_⟩, ?_⟩
    · simp [CompiledModule.structTypeParamCount, hsd, hsh] at hn
      omega
    · simp only [typeInstr, bind_ok] at hty
      obtain ⟨⟨d', sd', tys⟩, hparts, hty⟩ := hty
      obtain ⟨si', hsi', rfl, hsd', -⟩ := structInstParts_ok.1 hparts
      rw [hsi, Option.some.injEq] at hsi'
      subst hsi'
      dsimp only at hty
      obtain ⟨fs0, -, -, -, hfs, -⟩ := typePackGeneric_ok hty
      rcases fieldsSignature_ok hfs with ⟨-, fields, hfi, -⟩ | ⟨_, _, _, hv, -⟩
      · simp [CompiledModule.DeclaredStruct, CompiledModule.declaredFieldCount, hsd', hfi]
      · cases hv
  case unpackGeneric x =>
    simp only [InstructionConsistency.checkInstr, bind_ok, lookup_ok] at hci
    obtain ⟨si, hsi, hc⟩ := hci
    obtain ⟨n, hn, hpos⟩ := checkStructOp_generic hc
    obtain ⟨cs, hcs, -⟩ := verifyCode_instr hvc hi
    unfold SignatureV2.structInstantiation at hcs
    simp only [bind_ok, lookup_ok] at hcs
    obtain ⟨si', hsi', hcs⟩ := hcs
    rw [hsi, Option.some.injEq] at hsi'
    subst hsi'
    obtain ⟨sd, sh, args, hsd, hsh, hargs, hlen, -⟩ := structTypeParams_ok hcs
    refine ⟨si, hsi, ⟨n, args, hn, hpos, hargs, ?_⟩, ?_⟩
    · simp [CompiledModule.structTypeParamCount, hsd, hsh] at hn
      omega
    · simp only [typeInstr, bind_ok] at hty
      obtain ⟨⟨d', sd', tys⟩, hparts, hty⟩ := hty
      obtain ⟨si', hsi', rfl, hsd', -⟩ := structInstParts_ok.1 hparts
      rw [hsi, Option.some.injEq] at hsi'
      subst hsi'
      dsimp only at hty
      obtain ⟨fs0, -, -, -, hfs, -⟩ := typeUnpackGeneric_ok hty
      rcases fieldsSignature_ok hfs with ⟨-, fields, hfi, -⟩ | ⟨_, _, _, hv, -⟩
      · simp [CompiledModule.DeclaredStruct, CompiledModule.declaredFieldCount, hsd', hfi]
      · cases hv
  case mutBorrowGlobalGeneric x | immBorrowGlobalGeneric x | existsGeneric x
    | moveFromGeneric x | moveToGeneric x =>
    simp only [InstructionConsistency.checkInstr, bind_ok, lookup_ok] at hci
    obtain ⟨si, hsi, hc⟩ := hci
    obtain ⟨n, hn, hpos⟩ := checkStructOp_generic hc
    obtain ⟨cs, hcs, -⟩ := verifyCode_instr hvc hi
    unfold SignatureV2.structInstantiation at hcs
    simp only [bind_ok, lookup_ok] at hcs
    obtain ⟨si', hsi', hcs⟩ := hcs
    rw [hsi, Option.some.injEq] at hsi'
    subst hsi'
    obtain ⟨sd, sh, args, hsd, hsh, hargs, hlen, -⟩ := structTypeParams_ok hcs
    refine ⟨si, hsi, n, args, hn, hpos, hargs, ?_⟩
    simp [CompiledModule.structTypeParamCount, hsd, hsh] at hn
    omega
  case mutBorrowFieldGeneric x | immBorrowFieldGeneric x =>
    simp only [InstructionConsistency.checkInstr, bind_ok, lookup_ok] at hci
    obtain ⟨fi, hfi, fdh, hfdh, hc⟩ := hci
    obtain ⟨n, hn, hpos⟩ := checkStructOp_generic hc
    obtain ⟨cs, hcs, -⟩ := verifyCode_instr hvc hi
    unfold SignatureV2.fieldInstantiation at hcs
    simp only [bind_ok, lookup_ok] at hcs
    obtain ⟨fi', hfi', fdh', hfdh', hcs⟩ := hcs
    rw [hfi, Option.some.injEq] at hfi'
    subst hfi'
    rw [hfdh, Option.some.injEq] at hfdh'
    subst hfdh'
    obtain ⟨sd, sh, args, hsd, hsh, hargs, hlen, -⟩ := structTypeParams_ok hcs
    refine ⟨fi, hfi, n, args, by simp [CompiledModule.fieldTypeParamCount, hfdh, hn], hpos,
      hargs, ?_⟩
    simp [CompiledModule.structTypeParamCount, hsd, hsh] at hn
    omega
  case mutBorrowVariantFieldGeneric x | immBorrowVariantFieldGeneric x =>
    simp only [InstructionConsistency.checkInstr, bind_ok, lookup_ok] at hci
    obtain ⟨fi, hfi, fdh, hfdh, hc⟩ := hci
    obtain ⟨n, hn, hpos⟩ := checkStructOp_generic hc
    obtain ⟨cs, hcs, -⟩ := verifyCode_instr hvc hi
    unfold SignatureV2.variantFieldInstantiation at hcs
    simp only [bind_ok, lookup_ok] at hcs
    obtain ⟨fi', hfi', fdh', hfdh', hcs⟩ := hcs
    rw [hfi, Option.some.injEq] at hfi'
    subst hfi'
    rw [hfdh, Option.some.injEq] at hfdh'
    subst hfdh'
    obtain ⟨sd, sh, args, hsd, hsh, hargs, hlen, -⟩ := structTypeParams_ok hcs
    refine ⟨fi, hfi, n, args, by simp [CompiledModule.variantFieldTypeParamCount, hfdh, hn],
      hpos, hargs, ?_⟩
    simp [CompiledModule.structTypeParamCount, hsd, hsh] at hn
    omega
  case packVariantGeneric x | unpackVariantGeneric x | testVariantGeneric x =>
    simp only [InstructionConsistency.checkInstr, InstructionConsistency.checkVariantOp,
      bind_ok, lookup_ok] at hci
    obtain ⟨vi, hvi, vh, hvh, hc⟩ := hci
    obtain ⟨n, hn, hpos⟩ := checkStructOp_generic hc
    obtain ⟨cs, hcs, -⟩ := verifyCode_instr hvc hi
    unfold SignatureV2.structVariantInstantiation at hcs
    simp only [bind_ok, lookup_ok] at hcs
    obtain ⟨vi', hvi', vh', hvh', hcs⟩ := hcs
    rw [hvi, Option.some.injEq] at hvi'
    subst hvi'
    rw [hvh, Option.some.injEq] at hvh'
    subst hvh'
    obtain ⟨sd, sh, args, hsd, hsh, hargs, hlen, -⟩ := structTypeParams_ok hcs
    refine ⟨vi, hvi, n, args, by simp [CompiledModule.variantTypeParamCount, hvh, hn], hpos,
      hargs, ?_⟩
    simp [CompiledModule.structTypeParamCount, hsd, hsh] at hn
    omega

end MoveBytecode.Proofs
