-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.TokenSubst

/-!
# Well-formed stack tokens (L5 for `CallClosure`)

UNTRUSTED (checked by the kernel). The signature checker checks only that
`CallClosure`'s signature is one function type; type safety makes its
parameter and result types those of the closure on the stack, and its
abilities at most the closure's. So L5 for it follows from an invariant of
type safety: every token of the abstract stack is well formed where the
function's type parameters are in scope (`typeInstr_tokens`). Each token an
instruction pushes is one it popped, part of one, or a local's, a constant's,
a function handle's or a field's type, instantiated by type arguments L5
already establishes well formed.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## The model's generic forms -/

/-- The model's substitution, by a non-empty instantiation. -/
theorem instantiate_spec {tys : List SignatureToken} (hne : tys ≠ []) {t q : SignatureToken}
    (h : instantiate tys t = .ok q) : t.instantiate tys = some q := by
  have hne' : tys.isEmpty = false := by cases tys <;> simp_all
  simp only [instantiate, hne', Bool.false_eq_true, ↓reduceIte] at h
  exact instantiateIn_spec _ h

theorem mapM_instantiate_spec {tys : List SignatureToken} (hne : tys ≠ []) :
    ∀ {l l' : List SignatureToken}, l.mapM (instantiate tys) = .ok l' →
      SignatureToken.instantiateList tys l = some l'
  | [], _, h => by
    simp only [List.mapM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h
    rfl
  | t :: ts, _, h => by
    simp only [List.mapM_cons, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨q, hq, qs, hqs, rfl⟩ := h
    simp [SignatureToken.instantiateList, instantiate_spec hne hq, mapM_instantiate_spec hne hqs]

/-- One step of the model's variant field type under type arguments. -/
def vftgStep (sd : StructDefinition) (field : Nat) (tys : List SignatureToken)
    (acc : Option SignatureToken) (v : Nat) : VResult (Option SignatureToken) :=
  match (structFields sd.fieldInformation (some v))[field]? with
  | some fd => do
    let ty ← instantiate tys fd.signature
    match acc with
    | some t => if ty != t then reject "BORROWFIELD_TYPE_MISMATCH_ERROR" else pure acc
    | none => pure (some ty)
  | none => reject "BORROWFIELD_BAD_FIELD_ERROR"

theorem variantFieldTypeGeneric_eq (sd : StructDefinition) (field : Nat) (vs : List Nat)
    (tys : List SignatureToken) :
    variantFieldTypeGeneric sd field vs tys = vs.foldlM (vftgStep sd field tys) none := rfl

theorem vftg_some {sd : StructDefinition} {field : Nat} {tys : List SignatureToken} :
    ∀ {vs : List Nat} {t : SignatureToken} {r : Option SignatureToken},
      vs.foldlM (vftgStep sd field tys) (some t) = .ok r → r = some t ∧
        ∀ v ∈ vs, ∃ fd, (structFields sd.fieldInformation (some v))[field]? = some fd ∧
          instantiate tys fd.signature = .ok t
  | [], t, r, h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    exact ⟨h.symm, by simp⟩
  | v :: vs, t, r, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨acc', hstep, h⟩ := h
    unfold vftgStep at hstep
    cases hfd : (structFields sd.fieldInformation (some v))[field]? with
    | none => simp [hfd, reject] at hstep
    | some fd =>
      simp only [hfd, bind_ok] at hstep
      obtain ⟨ty, hty, hstep⟩ := hstep
      by_cases heq : ty = t
      · subst heq
        simp only [bne_self_eq_false, Bool.false_eq_true, ↓reduceIte, pure, Except.pure,
          Except.ok.injEq] at hstep
        subst hstep
        obtain ⟨hr, hall⟩ := vftg_some h
        refine ⟨hr, fun w hw => ?_⟩
        rcases List.mem_cons.1 hw with rfl | hw
        · exact ⟨fd, hfd, hty⟩
        · exact hall w hw
      · simp [heq, reject] at hstep

theorem vftg_none {sd : StructDefinition} {field : Nat} {tys : List SignatureToken}
    {vs : List Nat} {r : Option SignatureToken}
    (h : vs.foldlM (vftgStep sd field tys) none = .ok r) (hne : vs ≠ []) :
    ∃ t, r = some t ∧ ∀ v ∈ vs, ∃ fd, (structFields sd.fieldInformation (some v))[field]? = some fd ∧
      instantiate tys fd.signature = .ok t := by
  cases vs with
  | nil => exact absurd rfl hne
  | cons v vs =>
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨acc', hstep, h⟩ := h
    unfold vftgStep at hstep
    cases hfd : (structFields sd.fieldInformation (some v))[field]? with
    | none => simp [hfd, reject] at hstep
    | some fd =>
      simp only [hfd, bind_ok, pure, Except.pure, Except.ok.injEq] at hstep
      obtain ⟨ty, hty, rfl⟩ := hstep
      obtain ⟨hr, hall⟩ := vftg_some h
      refine ⟨ty, hr, fun w hw => ?_⟩
      rcases List.mem_cons.1 hw with rfl | hw
      · exact ⟨fd, hfd, hty⟩
      · exact hall w hw

section ModelGeneric

theorem fieldBorrowGen_ok {m : CompiledModule} {st st' : List SignatureToken} {mut_ : Bool}
    {h : Nat} {tys : List SignatureToken}
    (hok : borrowFieldGeneric m st mut_ false h tys = .ok st') :
    ∃ op rest fh sd, st = op :: rest ∧ (mut_ && !isMutableReference op) = false ∧
      m.fieldHandles[h]? = some fh ∧ m.structDefs[fh.owner]? = some sd ∧
      refTarget op = some (materializeType sd.structHandle tys) ∧
      ((∃ fd q, (structFields sd.fieldInformation none)[fh.field]? = some fd ∧
          instantiate tys fd.signature = .ok q ∧ st' = refTo mut_ q :: rest) ∨
        ((structFields sd.fieldInformation none)[fh.field]? = none ∧ st' = .bool :: rest)) := by
  simp only [borrowFieldGeneric, bind_ok, fieldHandleParts, Bool.false_eq_true, ↓reduceIte,
    lookup_ok, pure, Except.pure] at hok
  obtain ⟨⟨op, rest⟩, hpop, u, hmut, p, ⟨fh, hfh, hp⟩, sd, hsd, u', htar, h⟩ := hok
  rw [popType_ok] at hpop
  subst hpop
  simp only [Except.ok.injEq] at hp
  subst hp
  rw [rejectIf_ok] at hmut htar
  refine ⟨op, rest, fh, sd, rfl, hmut, hfh, hsd, by simpa using htar, ?_⟩
  revert h
  dsimp only
  cases hfd : (structFields sd.fieldInformation none)[fh.field]? with
  | none =>
    intro h
    right
    simp only [bind_ok] at h
    obtain ⟨r, hr, h⟩ := h
    simp only [Except.ok.injEq] at hr
    subst hr
    simp only [Except.ok.injEq] at h
    exact ⟨rfl, h.symm⟩
  | some fd =>
    intro h
    left
    simp only [bind_ok] at h
    obtain ⟨r, hr, h⟩ := h
    simp only [Functor.map, Except.map] at hr
    split at hr
    · simp at hr
    · next q hq =>
      simp only [Except.ok.injEq] at hr
      subst hr
      simp only [Except.ok.injEq] at h
      exact ⟨fd, q, rfl, hq, h.symm⟩

/-- A generic variant field borrow the model accepts, with its operand and its
result. -/
theorem variantBorrowGen_full {m : CompiledModule} {st st' : List SignatureToken} {mut_ : Bool}
    {h : Nat} {tys : List SignatureToken}
    (hok : borrowFieldGeneric m st mut_ true h tys = .ok st') :
    ∃ op rest fh sd, st = op :: rest ∧ (mut_ && !isMutableReference op) = false ∧
      m.variantFieldHandles[h]? = some fh ∧ m.structDefs[fh.structIndex]? = some sd ∧
      refTarget op = some (materializeType sd.structHandle tys) ∧ fh.variants ≠ [] ∧
      ∃ ft, st' = refTo mut_ ft :: rest ∧ ∀ v ∈ fh.variants, ∃ fd,
        (structFields sd.fieldInformation (some v))[fh.field]? = some fd ∧
        instantiate tys fd.signature = .ok ft := by
  simp only [borrowFieldGeneric, bind_ok, fieldHandleParts, ↓reduceIte, lookup_ok, pure,
    Except.pure] at hok
  obtain ⟨⟨op, rest⟩, hpop, u, hmut, p, ⟨fh, hfh, hp⟩, sd, hsd, u', htar, h⟩ := hok
  rw [popType_ok] at hpop
  subst hpop
  simp only [Except.ok.injEq] at hp
  subst hp
  rw [rejectIf_ok] at hmut htar
  dsimp only at h
  split at h
  · simp [reject, bind, Except.bind] at h
  · next hne =>
    have hne' : fh.variants ≠ [] := by simpa using hne
    simp only [bind_ok] at h
    obtain ⟨r, hr, h⟩ := h
    rw [variantFieldTypeGeneric_eq] at hr
    obtain ⟨ft, rfl, hall⟩ := vftg_none hr hne'
    simp only [Except.ok.injEq] at h
    exact ⟨op, rest, fh, sd, rfl, hmut, hfh, hsd, by simpa using htar, hne', ft, h.symm, hall⟩

theorem borrowGlobalGen_ok {m : CompiledModule} {cs : List AbilitySet}
    {st st' : List SignatureToken} {mut_ : Bool} {d : Nat} {tys : List SignatureToken}
    (h : borrowGlobalGeneric m cs st mut_ d tys = .ok st') :
    ∃ rest sd a, st = .address :: rest ∧ m.structDefs[d]? = some sd ∧
      abilities m cs (materializeType sd.structHandle tys) = .ok a ∧ a.key = true ∧
      st' = refTo mut_ (materializeType sd.structHandle tys) :: rest := by
  simp only [borrowGlobalGeneric, bind_ok, lookup_ok, pure, Except.pure] at h
  obtain ⟨⟨op, rest⟩, hpop, u, hop, sd, hsd, a, ha, u', hk, h⟩ := h
  rw [popType_ok] at hpop
  subst hpop
  rw [rejectIf_ok] at hop hk
  simp only [bne_eq_false_iff_eq, Bool.not_eq_false'] at hop hk
  simp only [Except.ok.injEq] at h
  exact ⟨rest, sd, a, by rw [hop], hsd, ha, hk, h.symm⟩

end ModelGeneric

/-! ## Slots -/

section
variable {m : CompiledModule} {tparams : List AbilitySet}

theorem slotsWF_reverse {ts : List SignatureToken}
    (h : SignatureToken.SlotsWellFormed m tparams ts) :
    SignatureToken.SlotsWellFormed m tparams ts.reverse :=
  slotsWF_iff.2 fun t ht => slotsWF_iff.1 h t (List.mem_reverse.1 ht)

theorem slotsWF_replicate {t : SignatureToken} (h : t.ValueTypeWellFormed m tparams) (n : Nat) :
    SignatureToken.SlotsWellFormed m tparams (List.replicate n t) :=
  slotsWF_iff.2 fun u hu => by rw [List.eq_of_mem_replicate hu]; exact slotsWF_of_vtwf h

theorem slotsWF_of_vtwfs {ts : List SignatureToken}
    (h : ∀ t ∈ ts, t.ValueTypeWellFormed m tparams) : SignatureToken.SlotsWellFormed m tparams ts :=
  slotsWF_iff.2 fun t ht => slotsWF_of_vtwf (h t ht)

theorem slotsWF_refTo {mut_ : Bool} {t : SignatureToken} (h : t.ValueTypeWellFormed m tparams) :
    SignatureToken.SlotsWellFormed m tparams [refTo mut_ t] := by
  cases mut_ <;> simp [refTo, SignatureToken.SlotsWellFormed, h]

theorem vtwf_of_refTarget {op inner : SignatureToken} (h : refTarget op = some inner)
    (hs : SignatureToken.SlotsWellFormed m tparams [op]) : inner.ValueTypeWellFormed m tparams := by
  cases op <;> simp [refTarget] at h <;> subst h <;> exact hs.1

theorem vtwf_of_nonref {t : SignatureToken} (h : SignatureToken.SlotsWellFormed m tparams [t])
    (hr : SignatureV2.isReference t = false) : t.ValueTypeWellFormed m tparams := by
  cases t <;> simp_all [SignatureToken.SlotsWellFormed, SignatureV2.isReference]

theorem slotsWF_prim {t : SignatureToken} (h : t.ValueTypeWellFormed m tparams)
    {st : List SignatureToken} (hst : SignatureToken.SlotsWellFormed m tparams st) :
    SignatureToken.SlotsWellFormed m tparams (t :: st) :=
  slotsWF_cons (slotsWF_of_vtwf h) hst

end

/-! ## Instantiation by the model -/

section
variable {m : CompiledModule} {tparams cs : List AbilitySet} {tys : List SignatureToken}

/-- The model's substitution by well-formed type arguments keeps a value
type well formed. -/
theorem vtwf_modelInst (hσ : SignatureToken.ArgumentsWellFormed m tparams tys cs)
    {t q : SignatureToken} (ht : t.ValueTypeWellFormed m cs) (h : instantiate tys t = .ok q) :
    q.ValueTypeWellFormed m tparams := by
  by_cases hne : tys = []
  · subst hne
    cases cs with
    | nil =>
      simp only [instantiate, List.isEmpty_nil, ↓reduceIte, pure, Except.pure,
        Except.ok.injEq] at h
      subst h
      exact vtwf_closed ht
    | cons _ _ => exact hσ.elim
  · exact vtwf_instantiate hσ t ht (instantiate_spec hne h)

theorem slotWF_modelInst (hσ : SignatureToken.ArgumentsWellFormed m tparams tys cs)
    {t q : SignatureToken} (ht : SignatureToken.SlotsWellFormed m cs [t])
    (h : instantiate tys t = .ok q) : SignatureToken.SlotsWellFormed m tparams [q] := by
  by_cases hne : tys = []
  · subst hne
    cases cs with
    | nil =>
      simp only [instantiate, List.isEmpty_nil, ↓reduceIte, pure, Except.pure,
        Except.ok.injEq] at h
      subst h
      exact slotsWF_closed ht
    | cons _ _ => exact hσ.elim
  · exact slotsWF_instantiate hσ [t] ht
      (by simp [SignatureToken.instantiateList, instantiate_spec hne h])

theorem slotsWF_mapM_modelInst (hσ : SignatureToken.ArgumentsWellFormed m tparams tys cs) :
    ∀ {l l' : List SignatureToken}, SignatureToken.SlotsWellFormed m cs l →
      l.mapM (instantiate tys) = .ok l' → SignatureToken.SlotsWellFormed m tparams l'
  | [], _, _, h => by
    simp only [List.mapM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h
    trivial
  | _ :: _, _, hw, h => by
    simp only [List.mapM_cons, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨q, hq, qs, hqs, rfl⟩ := h
    exact slotsWF_cons (slotWF_modelInst hσ (slotsWF_head hw) hq)
      (slotsWF_mapM_modelInst hσ (slotsWF_tail hw) hqs)

end

/-! ## Struct types and fields -/

section
variable {m : CompiledModule} {tparams : List AbilitySet}

theorem mem_allFields_of_structFields {fi : StructFieldInformation} {variant : Option Nat}
    {k : Nat} {fd : FieldDefinition} (h : (structFields fi variant)[k]? = some fd) :
    fd ∈ CompiledModule.StructFieldInformation.allFields fi := by
  have hm := List.mem_of_getElem? h
  cases fi with
  | native => cases variant <;> simp [structFields] at hm
  | declared fields =>
    cases variant <;> simpa [structFields, CompiledModule.StructFieldInformation.allFields] using hm
  | declaredVariants vs =>
    cases variant with
    | none => simp [structFields] at hm
    | some v =>
      simp only [structFields] at hm
      split at hm
      · next vd hvd =>
        simp only [CompiledModule.StructFieldInformation.allFields]
        exact List.mem_flatMap.2 ⟨vd, List.mem_of_getElem? hvd, hm⟩
      · simp at hm

theorem fieldsSignature_mem {sd : StructDefinition} {variant : Option Nat}
    {fs : List SignatureToken} (h : fieldsSignature sd variant = .ok fs) :
    ∀ t ∈ fs, ∃ fd ∈ CompiledModule.StructFieldInformation.allFields sd.fieldInformation,
      fd.signature = t := by
  intro t ht
  rcases fieldsSignature_ok h with ⟨-, fields, hfi, rfl⟩ | ⟨v, variants, vd, -, hfi, hvd, rfl⟩
  · obtain ⟨fd, hfd, rfl⟩ := List.mem_map.1 ht
    refine ⟨fd, ?_, rfl⟩
    rw [hfi]
    exact hfd
  · obtain ⟨fd, hfd, rfl⟩ := List.mem_map.1 ht
    refine ⟨fd, ?_, rfl⟩
    rw [hfi]
    simp only [CompiledModule.StructFieldInformation.allFields]
    exact List.mem_flatMap.2 ⟨vd, List.mem_of_getElem? hvd, hfd⟩

theorem structHandle_of_count {d : Nat} {sd : StructDefinition}
    (hsd : m.structDefs[d]? = some sd) {n : Nat} (hcount : m.structTypeParamCount d = some n) :
    ∃ sh, m.structHandles[sd.structHandle]? = some sh ∧ sh.typeParameters.length = n := by
  simp only [CompiledModule.structTypeParamCount, hsd, bind, Option.bind_some, pure] at hcount
  cases hsh : m.structHandles[sd.structHandle]? with
  | none => simp [hsh] at hcount
  | some sh =>
    simp only [hsh, Option.bind_some, Option.some.injEq] at hcount
    exact ⟨sh, rfl, hcount⟩

theorem structConstraints_eq {d : Nat} {sd : StructDefinition} (hsd : m.structDefs[d]? = some sd)
    {cs : List AbilitySet} (hcs : m.structConstraints d = some cs) :
    ∃ sh, m.structHandles[sd.structHandle]? = some sh ∧
      cs = sh.typeParameters.map (·.constraints) := by
  simp only [CompiledModule.structConstraints, hsd, CompiledModule.structDefHandle, bind,
    Option.bind_some, pure] at hcs
  cases hsh : m.structHandles[sd.structHandle]? with
  | none => simp [hsh] at hcs
  | some sh =>
    simp only [hsh, Option.bind_some, Option.some.injEq] at hcs
    exact ⟨sh, rfl, hcs.symm⟩

/-- A non-generic struct's type. -/
theorem vtwf_struct_of_count {d : Nat} {sd : StructDefinition} (hsd : m.structDefs[d]? = some sd)
    (hcount : m.structTypeParamCount d = some 0) :
    (SignatureToken.struct sd.structHandle).ValueTypeWellFormed m tparams := by
  obtain ⟨sh, hsh, hlen⟩ := structHandle_of_count hsd hcount
  exact ⟨sh, hsh, List.eq_nil_of_length_eq_zero hlen⟩

/-- A struct instance's type, at well-formed type arguments. -/
theorem vtwf_materialize {d : Nat} {sd : StructDefinition} (hsd : m.structDefs[d]? = some sd)
    {cs : List AbilitySet} (hcs : m.structConstraints d = some cs) {tys : List SignatureToken}
    (hargs : SignatureToken.ArgumentsWellFormed m tparams tys cs) :
    (materializeType sd.structHandle tys).ValueTypeWellFormed m tparams := by
  obtain ⟨sh, hsh, rfl⟩ := structConstraints_eq hsd hcs
  unfold materializeType
  split
  · next he =>
    rw [List.isEmpty_iff] at he
    subst he
    have hlen := argsWF_length hargs
    simp only [List.length_nil, List.length_map] at hlen
    exact ⟨sh, hsh, List.eq_nil_of_length_eq_zero hlen.symm⟩
  · exact ⟨sh, hsh, hargs⟩

end

/-! ## Closure types -/

theorem key_of_subset {a b : AbilitySet} (h : a.subset b = true) (hb : b.key = false) :
    a.key = false := by
  cases a; cases b
  simp_all [MoveBytecode.AbilitySet.subset]

theorem closureInit_key (fh : FunctionHandle) : (closureInitAbilities fh).key = false := by
  unfold closureInitAbilities
  split <;> rfl

theorem maskExtract_mem {α : Type} {mask : BitVec 64} {l : List α} {c : Bool} {x : α}
    (h : x ∈ maskExtract mask l c) : x ∈ l := by
  unfold maskExtract at h
  obtain ⟨p, hp, rfl⟩ := List.mem_map.1 h
  have hp' := (List.mem_filter.1 hp).1
  obtain ⟨x, i⟩ := p
  exact List.mem_of_getElem? (List.mem_zipIdx_iff_getElem?.1 hp')

/-- The type of a closure `PackClosure` creates, before instantiation. -/
theorem vtwf_closureType {m : CompiledModule} {cs : List AbilitySet} {fh : FunctionHandle}
    {params returns : List SignatureToken} {mask : BitVec 64} {ab : AbilitySet}
    (hps : SignatureToken.SlotsWellFormed m cs params)
    (hrs : SignatureToken.SlotsWellFormed m cs returns)
    (hab : ab.subset (closureInitAbilities fh) = true) :
    (SignatureToken.function (maskExtract mask params false) returns ab).ValueTypeWellFormed m cs :=
  ⟨slotsWF_iff.2 fun t ht => slotsWF_iff.1 hps t (maskExtract_mem ht), hrs,
    key_of_subset hab (closureInit_key fh)⟩

theorem assignable_function {ps rs : List SignatureToken} {a : AbilitySet} {clo : SignatureToken}
    (h : isAssignableFrom (.function ps rs a) clo = true) :
    ∃ ab, clo = .function ps rs ab ∧ a.subset ab = true := by
  cases clo <;> simp [isAssignableFrom] at h
  case function ps' rs' ab =>
    obtain ⟨⟨rfl, rfl⟩, h⟩ := h
    exact ⟨ab, rfl, h⟩

/-! ## The invariant -/

/-- What the stack-token invariant needs of the module and the function: L5
for the locals, the constants, the function handles and the fields. -/
structure TokCtx (m : CompiledModule) (view : FunctionView) : Prop where
  locals : SignatureToken.SlotsWellFormed m view.handle.typeParameters
    (view.parameters ++ view.locals)
  constants : ∀ c ∈ m.constantPool.toList, c.type.ValueTypeWellFormed m []
  handles : ∀ fh ∈ m.functionHandles.toList,
    m.SlotSignatureWellFormed fh.typeParameters fh.parameters ∧
    m.SlotSignatureWellFormed fh.typeParameters fh.returns
  fields : ∀ d ∈ m.structDefs.toList, ∃ sh, m.structDefHandle d = some sh ∧
    ∀ f ∈ CompiledModule.StructFieldInformation.allFields d.fieldInformation,
      f.signature.ValueTypeWellFormed m (sh.typeParameters.map (·.constraints))

section
variable {m : CompiledModule} {view : FunctionView}

theorem TokCtx.local (hc : TokCtx m view) {i : Nat} {t : SignatureToken}
    (h : localAt view i = .ok t) :
    SignatureToken.SlotsWellFormed m view.handle.typeParameters [t] :=
  slotsWF_iff.1 hc.locals t (List.mem_of_getElem? (localAt_ok h))

theorem TokCtx.field (hc : TokCtx m view) {d : Nat} {sd : StructDefinition}
    (hsd : m.structDefs[d]? = some sd) {cs : List AbilitySet} (hcs : m.structConstraints d = some cs)
    {fd : FieldDefinition}
    (hfd : fd ∈ CompiledModule.StructFieldInformation.allFields sd.fieldInformation) :
    fd.signature.ValueTypeWellFormed m cs := by
  obtain ⟨sh, hsh, hall⟩ := hc.fields sd (getElem?_mem_toList hsd)
  obtain ⟨sh', hsh', rfl⟩ := structConstraints_eq hsd hcs
  simp only [CompiledModule.structDefHandle] at hsh
  rw [hsh, Option.some.injEq] at hsh'
  subst hsh'
  exact hall fd hfd

theorem TokCtx.field0 (hc : TokCtx m view) {d : Nat} {sd : StructDefinition}
    (hsd : m.structDefs[d]? = some sd) (hcount : m.structTypeParamCount d = some 0)
    {fd : FieldDefinition}
    (hfd : fd ∈ CompiledModule.StructFieldInformation.allFields sd.fieldInformation) :
    fd.signature.ValueTypeWellFormed m view.handle.typeParameters := by
  obtain ⟨sh, hsh, hlen⟩ := structHandle_of_count hsd hcount
  have hcs : m.structConstraints d = some [] := by
    simp [CompiledModule.structConstraints, hsd, CompiledModule.structDefHandle, hsh,
      List.eq_nil_of_length_eq_zero hlen]
  exact vtwf_closed (hc.field hsd hcs hfd)

theorem TokCtx.fieldsSig (hc : TokCtx m view) {d : Nat} {sd : StructDefinition}
    (hsd : m.structDefs[d]? = some sd) {cs : List AbilitySet} (hcs : m.structConstraints d = some cs)
    {variant : Option Nat} {fs : List SignatureToken} (hfs : fieldsSignature sd variant = .ok fs) :
    SignatureToken.SlotsWellFormed m cs fs :=
  slotsWF_of_vtwfs fun t ht => by
    obtain ⟨fd, hfd, rfl⟩ := fieldsSignature_mem hfs t ht
    exact hc.field hsd hcs hfd

theorem TokCtx.fieldsSig0 (hc : TokCtx m view) {d : Nat} {sd : StructDefinition}
    (hsd : m.structDefs[d]? = some sd) (hcount : m.structTypeParamCount d = some 0)
    {variant : Option Nat} {fs : List SignatureToken} (hfs : fieldsSignature sd variant = .ok fs) :
    SignatureToken.SlotsWellFormed m view.handle.typeParameters fs :=
  slotsWF_of_vtwfs fun t ht => by
    obtain ⟨fd, hfd, rfl⟩ := fieldsSignature_mem hfs t ht
    exact hc.field0 hsd hcount hfd

theorem TokCtx.handle (hc : TokCtx m view) {h : Nat} {fh : FunctionHandle}
    (hfh : m.functionHandles[h]? = some fh) :
    m.SlotSignatureWellFormed fh.typeParameters fh.parameters ∧
      m.SlotSignatureWellFormed fh.typeParameters fh.returns :=
  hc.handles fh (getElem?_mem_toList hfh)

end

theorem elementType_mem {m : CompiledModule} {s : Nat} {e : SignatureToken}
    (h : elementType m s = .ok e) : ∃ sig, m.signatures[s]? = some sig ∧ e ∈ sig := by
  unfold elementType at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨sig, hsig, h⟩ := h
  split at h
  · next t ht =>
    simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact ⟨sig, hsig, List.mem_of_mem_head? ht⟩
  · simp [panicAt] at h

theorem elementType_wf {m : CompiledModule} {tparams : List AbilitySet} {s : Nat}
    {e : SignatureToken} (hl5 : m.ValueSignatureWellFormed tparams s)
    (h : elementType m s = .ok e) : e.ValueTypeWellFormed m tparams := by
  obtain ⟨sig, hsig, he⟩ := elementType_mem h
  obtain ⟨sig', hsig', hall⟩ := hl5
  rw [hsig, Option.some.injEq] at hsig'
  subst hsig'
  exact hall e he

/-- **Type safety keeps every stack token well formed**, where the function's
type parameters are in scope. -/
theorem typeInstr_tokens {m : CompiledModule} {view : FunctionView} (hc : TokCtx m view)
    {i : Bytecode} (hl9 : i.OperandsConsistent m)
    (hl5 : (∃ s, i = .callClosure s) ∨ i.TypeOperandsWellFormed m view.handle.typeParameters)
    {st st' : List SignatureToken}
    (hst : SignatureToken.SlotsWellFormed m view.handle.typeParameters st)
    (h : typeInstr m view st i = .ok st') :
    SignatureToken.SlotsWellFormed m view.handle.typeParameters st' := by
  have hl5' : ∀ (hne : ∀ s, i ≠ .callClosure s),
      i.TypeOperandsWellFormed m view.handle.typeParameters := fun hne =>
    hl5.resolve_left fun ⟨s, hs⟩ => hne s hs
  cases i <;> simp only [typeInstr, Model.castTo, typeCall, bind_ok, pure, Except.pure] at h
  case pop =>
    obtain ⟨⟨op, rest⟩, hpop, ab, -, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    split at h
    · simp [reject] at h
    · simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_tail hst
  case brTrue | brFalse | abort =>
    obtain ⟨⟨op, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    split at h
    · simp [reject] at h
    · simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_tail hst
  case stLoc li =>
    obtain ⟨⟨op, rest⟩, hpop, t, -, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    split at h
    · simp [reject] at h
    · simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_tail hst
  case ret =>
    obtain ⟨args, rfl, -, -⟩ := popExpected_ok h
    exact (slotsWF_append.1 hst).2
  case nop | branch =>
    simp only [Except.ok.injEq] at h
    subst h
    exact hst
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldTrue | ldFalse =>
    simp only [Except.ok.injEq] at h
    subst h
    exact slotsWF_prim (by trivial) hst
  case ldConst ci =>
    obtain ⟨c, hcl, h⟩ := h
    rw [lookup_ok] at hcl
    simp only [Except.ok.injEq] at h
    subst h
    exact slotsWF_prim (vtwf_closed (hc.constants c (getElem?_mem_toList hcl))) hst
  case copyLoc li =>
    obtain ⟨t, ht, ab, -, h⟩ := h
    split at h
    · simp [reject] at h
    · simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_cons (hc.local ht) hst
  case moveLoc li =>
    obtain ⟨t, ht, h⟩ := h
    simp only [Except.ok.injEq] at h
    subst h
    exact slotsWF_cons (hc.local ht) hst
  case call hi =>
    obtain ⟨fh, hfh, params, -, st1, hpop, rets, hrets, h⟩ := h
    rw [lookup_ok] at hfh hrets
    simp only [Except.ok.injEq] at h
    subst h
    have hnp : fh.typeParameters = [] := by
      simp only [Bytecode.OperandsConsistent, CompiledModule.functionTypeParamCount, hfh, bind,
        Option.bind_some, pure, Option.some.injEq] at hl9
      exact List.eq_nil_of_length_eq_zero hl9
    obtain ⟨args, rfl, -, -⟩ := popExpected_ok hpop
    obtain ⟨rs, hrs, hwf⟩ := (hc.handle hfh).2
    rw [hrets, Option.some.injEq] at hrs
    subst hrs
    rw [hnp] at hwf
    exact slotsWF_append.2 ⟨slotsWF_reverse (slotsWF_closed hwf), (slotsWF_append.1 hst).2⟩
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256
    | castI8 | castI16 | castI32 | castI64 | castI128 | castI256 =>
    obtain ⟨⟨op, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    split at h
    · simp [reject] at h
    · simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_prim (by trivial) (slotsWF_tail hst)
  case add | sub | mul | div | mod | lt | gt | le | ge | bitOr | bitAnd | xor | or | and =>
    obtain ⟨⟨op1, op2, rest⟩, hpop, h⟩ := h
    rw [popTwo_ok] at hpop
    subst hpop
    split at h
    · simp only [Except.ok.injEq] at h
      subst h
      first
        | exact slotsWF_cons (slotsWF_head hst) (slotsWF_tail (slotsWF_tail hst))
        | exact slotsWF_prim (by trivial) (slotsWF_tail (slotsWF_tail hst))
    · simp [reject] at h
  case shl | shr =>
    obtain ⟨⟨op1, op2, rest⟩, hpop, h⟩ := h
    rw [popTwo_ok] at hpop
    subst hpop
    split at h
    · simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_cons (slotsWF_head (slotsWF_tail hst)) (slotsWF_tail (slotsWF_tail hst))
    · simp [reject] at h
  case eq | neq =>
    obtain ⟨⟨op1, op2, rest⟩, hpop, ab, -, h⟩ := h
    rw [popTwo_ok] at hpop
    subst hpop
    split at h
    · simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_prim (by trivial) (slotsWF_tail (slotsWF_tail hst))
    · simp [reject] at h
  case negate =>
    obtain ⟨⟨op, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    split at h
    · simp only [Except.ok.injEq] at h
      subst h
      exact hst
    · simp [reject] at h
  case not =>
    obtain ⟨⟨op, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    split at h
    · simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_prim (by trivial) (slotsWF_tail hst)
    · simp [reject] at h
  case pack d =>
    obtain ⟨sd, hsd, h⟩ := h
    rw [lookup_ok] at hsd
    obtain ⟨fs, args, rest, -, rfl, -, -, rfl⟩ := typePack_ok h
    exact slotsWF_prim (vtwf_struct_of_count hsd hl9.1) (slotsWF_append.1 hst).2
  case unpack d =>
    obtain ⟨sd, hsd, h⟩ := h
    rw [lookup_ok] at hsd
    obtain ⟨fs, rest, rfl, hfs, rfl⟩ := typeUnpack_ok h
    exact slotsWF_append.2 ⟨slotsWF_reverse (hc.fieldsSig0 hsd hl9.1 hfs), slotsWF_tail hst⟩
  case packVariant vi =>
    obtain ⟨vh, hvh, sd, hsd, h⟩ := h
    rw [lookup_ok] at hvh hsd
    have hcount : m.structTypeParamCount vh.structIndex = some 0 := by
      simpa [Bytecode.OperandsConsistent, CompiledModule.variantTypeParamCount, hvh] using hl9
    obtain ⟨fs, args, rest, -, rfl, -, -, rfl⟩ := typePack_ok h
    exact slotsWF_prim (vtwf_struct_of_count hsd hcount) (slotsWF_append.1 hst).2
  case unpackVariant vi =>
    obtain ⟨vh, hvh, sd, hsd, h⟩ := h
    rw [lookup_ok] at hvh hsd
    have hcount : m.structTypeParamCount vh.structIndex = some 0 := by
      simpa [Bytecode.OperandsConsistent, CompiledModule.variantTypeParamCount, hvh] using hl9
    obtain ⟨fs, rest, rfl, hfs, rfl⟩ := typeUnpack_ok h
    exact slotsWF_append.2 ⟨slotsWF_reverse (hc.fieldsSig0 hsd hcount hfs), slotsWF_tail hst⟩
  case vecPack sig n =>
    obtain ⟨elem, helem, st1, hpop, h⟩ := h
    simp only [Except.ok.injEq] at h
    subst h
    obtain ⟨args, rfl, -, -⟩ := popElements_ok _ hpop
    have he := elementType_wf (hl5' (by simp)) helem
    exact slotsWF_prim (t := .vector elem) he (slotsWF_append.1 hst).2
  case vecUnpack sig n =>
    obtain ⟨⟨op, rest⟩, hp, elem, helem, h⟩ := h
    rw [popType_ok] at hp
    subst hp
    split at h
    · simp [reject] at h
    · simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_append.2 ⟨slotsWF_replicate (elementType_wf (hl5' (by simp)) helem) n,
        slotsWF_tail hst⟩
  case abortMsg =>
    obtain ⟨⟨msg, rest⟩, hp, ⟨code, rest'⟩, hp', h⟩ := h
    rw [popType_ok] at hp hp'
    subst hp hp'
    split at h
    · simp [reject] at h
    · simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_tail (slotsWF_tail hst)
  case freezeRef =>
    obtain ⟨⟨op, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    dsimp only at h
    split at h
    · next inner =>
      simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_cons ⟨(slotsWF_head hst).1, trivial⟩ (slotsWF_tail hst)
    · simp [reject] at h
  case mutBorrowField hd | immBorrowField hd =>
    obtain ⟨op, rest, fh, sd, rfl, -, hfh, hsd, -, rfl⟩ := fieldBorrow_ok h
    have hcount : m.structTypeParamCount fh.owner = some 0 := by
      simpa [Bytecode.OperandsConsistent, CompiledModule.fieldTypeParamCount, hfh] using hl9
    refine slotsWF_cons ?_ (slotsWF_tail hst)
    split
    · next fd hfd => exact slotsWF_refTo (hc.field0 hsd hcount (mem_allFields_of_structFields hfd))
    · exact slotsWF_of_vtwf (by trivial)
  case mutBorrowVariantField hd | immBorrowVariantField hd =>
    obtain ⟨op, rest, fh, sd, rfl, -, hfh, hsd, -, hne, ft, rfl, hall⟩ := variantBorrow_ok h
    have hcount : m.structTypeParamCount fh.structIndex = some 0 := by
      simpa [Bytecode.OperandsConsistent, CompiledModule.variantFieldTypeParamCount, hfh] using hl9
    obtain ⟨v, hv⟩ := List.exists_mem_of_ne_nil _ hne
    obtain ⟨fd, hfd, rfl⟩ := hall v hv
    exact slotsWF_cons (slotsWF_refTo (hc.field0 hsd hcount (mem_allFields_of_structFields hfd)))
      (slotsWF_tail hst)
  case mutBorrowLoc li | immBorrowLoc li =>
    obtain ⟨t, ht, h⟩ := h
    split at h
    · simp [reject] at h
    · next hnr =>
      simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_cons ⟨vtwf_of_nonref (hc.local ht) (by simpa using hnr), trivial⟩ hst
  case testVariant hv =>
    obtain ⟨vh, -, sd, -, ⟨arg, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    repeat' split at h
    all_goals first
      | (simp only [Except.ok.injEq] at h; subst h; exact slotsWF_prim (by trivial) (slotsWF_tail hst))
      | simp [reject] at h
  case readRef =>
    obtain ⟨⟨op, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    dsimp only at h
    split at h
    · simp only [bind_ok] at h
      obtain ⟨ab, -, h⟩ := h
      split at h
      · simp [reject] at h
      · simp only [Except.ok.injEq] at h
        subst h
        exact slotsWF_prim (slotsWF_head hst).1 (slotsWF_tail hst)
    · simp only [bind_ok] at h
      obtain ⟨ab, -, h⟩ := h
      split at h
      · simp [reject] at h
      · simp only [Except.ok.injEq] at h
        subst h
        exact slotsWF_prim (slotsWF_head hst).1 (slotsWF_tail hst)
    · simp [reject] at h
  case writeRef =>
    obtain ⟨⟨refOp, rest⟩, hpop, ⟨val, rest'⟩, hpop', h⟩ := h
    rw [popType_ok] at hpop hpop'
    subst hpop
    simp only at hpop'
    subst hpop'
    dsimp only at h
    split at h
    · simp only [bind_ok] at h
      obtain ⟨ab, -, h⟩ := h
      split at h
      · simp [reject] at h
      · split at h
        · simp [reject] at h
        · simp only [Except.ok.injEq] at h
          subst h
          exact slotsWF_tail (slotsWF_tail hst)
    · simp [reject] at h
  case mutBorrowGlobal d | immBorrowGlobal d =>
    obtain ⟨rest, sd, a, rfl, hsd, -, -, rfl⟩ := borrowGlobal_ok h
    exact slotsWF_cons (slotsWF_refTo (vtwf_struct_of_count hsd hl9)) (slotsWF_tail hst)
  case exists_ d | moveFrom d =>
    obtain ⟨sd, hsd, a, -, u, -, ⟨op, rest⟩, hpop, u', -, h⟩ := h
    rw [lookup_ok] at hsd
    rw [popType_ok] at hpop
    subst hpop
    simp only [Except.ok.injEq] at h
    subst h
    first
      | exact slotsWF_prim (by trivial) (slotsWF_tail hst)
      | exact slotsWF_prim (vtwf_struct_of_count hsd hl9) (slotsWF_tail hst)
  case moveTo d =>
    obtain ⟨sd, -, a, -, u, -, ⟨k, rest⟩, hpop, ⟨s, rest'⟩, hpop', u', -, u'', -, h⟩ := h
    rw [popType_ok] at hpop hpop'
    subst hpop hpop'
    simp only [Except.ok.injEq] at h
    subst h
    exact slotsWF_tail (slotsWF_tail hst)
  case vecLen idx =>
    obtain ⟨⟨op, rest⟩, hpop, elem, -, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    split at h
    · simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_prim (by trivial) (slotsWF_tail hst)
    · simp [reject] at h
  case vecImmBorrow idx | vecMutBorrow idx =>
    obtain ⟨elem, helem, h⟩ := h
    simp only [borrowVectorElement, bind_ok] at h
    obtain ⟨⟨i, rest⟩, hpop, ⟨vec, rest'⟩, hpop', u, -, h⟩ := h
    rw [popType_ok] at hpop hpop'
    subst hpop hpop'
    split at h
    · next e he =>
      split at h
      · next heq =>
        simp only [beq_iff_eq] at heq
        subst heq
        simp only [pure, Except.pure, Except.ok.injEq] at h
        subst h
        exact slotsWF_cons (slotsWF_refTo (elementType_wf (hl5' (by simp)) helem))
          (slotsWF_tail (slotsWF_tail hst))
      · simp [reject] at h
    · simp [reject] at h
  case vecPushBack idx =>
    obtain ⟨⟨elemOp, rest⟩, hpop, ⟨vecOp, rest'⟩, hpop', elem, -, u, -, u', -, h⟩ := h
    rw [popType_ok] at hpop hpop'
    subst hpop hpop'
    simp only [Except.ok.injEq] at h
    subst h
    exact slotsWF_tail (slotsWF_tail hst)
  case vecPopBack idx =>
    obtain ⟨⟨vecOp, rest⟩, hpop, elem, helem, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    split at h
    · simp only [Except.ok.injEq] at h
      subst h
      exact slotsWF_prim (elementType_wf (hl5' (by simp)) helem) (slotsWF_tail hst)
    · simp [reject] at h
  case vecSwap idx =>
    obtain ⟨⟨i2, rest⟩, hpop, ⟨i1, rest'⟩, hpop', ⟨v, rest''⟩, hpop'', u, -, elem, -, u', -, h⟩ :=
      h
    rw [popType_ok] at hpop hpop' hpop''
    subst hpop hpop' hpop''
    simp only [Except.ok.injEq] at h
    subst h
    exact slotsWF_tail (slotsWF_tail (slotsWF_tail hst))
  case callGeneric ii =>
    obtain ⟨fi, hfi, fh, hfh, tys, htys, h⟩ := h
    rw [lookup_ok] at hfi hfh htys
    obtain ⟨params, rets, rets', args, rest, -, hrets, hrets', rfl, -, -, rfl⟩ :=
      typeCallGeneric_ok h
    obtain ⟨x, fh', hx, hfh', args', hargs', hwf⟩ := hl5' (by simp)
    rw [hfi, Option.some.injEq] at hx
    subst hx
    rw [hfh, Option.some.injEq] at hfh'
    subst hfh'
    rw [htys, Option.some.injEq] at hargs'
    subst hargs'
    obtain ⟨rs, hrs, hrwf⟩ := (hc.handle hfh).2
    rw [hrets, Option.some.injEq] at hrs
    subst hrs
    exact slotsWF_append.2 ⟨slotsWF_reverse (slotsWF_mapM_modelInst hwf hrwf hrets'),
      (slotsWF_append.1 hst).2⟩
  case packGeneric ii | unpackGeneric ii =>
    obtain ⟨⟨d, sd, tys⟩, hparts, h⟩ := h
    rw [structInstParts_ok] at hparts
    obtain ⟨si, hsi, rfl, hsd, htys⟩ := hparts
    obtain ⟨x, cs, hx, hcs, args, hargs, hwf⟩ := hl5' (by simp)
    rw [hsi, Option.some.injEq] at hx
    subst hx
    rw [htys, Option.some.injEq] at hargs
    subst hargs
    first
      | obtain ⟨fs0, fs, args, rest, -, -, rfl, -, -, rfl⟩ := typePackGeneric_ok h
        exact slotsWF_prim (vtwf_materialize hsd hcs hwf) (slotsWF_append.1 hst).2
      | obtain ⟨fs0, fs, rest, rfl, h0, h1, rfl⟩ := typeUnpackGeneric_ok h
        exact slotsWF_append.2 ⟨slotsWF_reverse (slotsWF_mapM_modelInst hwf
          (hc.fieldsSig hsd hcs h0) h1), slotsWF_tail hst⟩
  case packVariantGeneric ii | unpackVariantGeneric ii =>
    obtain ⟨⟨vh, sd, tys⟩, hparts, h⟩ := h
    rw [variantInstParts_ok] at hparts
    obtain ⟨vi, hvi, hvh, hsd, htys⟩ := hparts
    obtain ⟨x, vh', cs, hx, hvh', hcs, args, hargs, hwf⟩ := hl5' (by simp)
    rw [hvi, Option.some.injEq] at hx
    subst hx
    rw [hvh, Option.some.injEq] at hvh'
    subst hvh'
    rw [htys, Option.some.injEq] at hargs
    subst hargs
    first
      | obtain ⟨fs0, fs, args, rest, -, -, rfl, -, -, rfl⟩ := typePackGeneric_ok h
        exact slotsWF_prim (vtwf_materialize hsd hcs hwf) (slotsWF_append.1 hst).2
      | obtain ⟨fs0, fs, rest, rfl, h0, h1, rfl⟩ := typeUnpackGeneric_ok h
        exact slotsWF_append.2 ⟨slotsWF_reverse (slotsWF_mapM_modelInst hwf
          (hc.fieldsSig hsd hcs h0) h1), slotsWF_tail hst⟩
  case testVariantGeneric ii =>
    obtain ⟨⟨vh, sd, tys⟩, -, ⟨arg, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    repeat' split at h
    all_goals first
      | (simp only [Except.ok.injEq] at h; subst h; exact slotsWF_prim (by trivial) (slotsWF_tail hst))
      | simp [reject] at h
  case mutBorrowFieldGeneric ii | immBorrowFieldGeneric ii =>
    obtain ⟨fi, hfi, tys, htys, h⟩ := h
    rw [lookup_ok] at hfi htys
    obtain ⟨op, rest, fh, sd, rfl, -, hfh, hsd, -, hcase⟩ := fieldBorrowGen_ok h
    obtain ⟨x, fdh, cs, hx, hfdh, hcs, args, hargs, hwf⟩ := hl5' (by simp)
    rw [hfi, Option.some.injEq] at hx
    subst hx
    rw [hfh, Option.some.injEq] at hfdh
    subst hfdh
    rw [htys, Option.some.injEq] at hargs
    subst hargs
    rcases hcase with ⟨fd, q, hfd, hq, rfl⟩ | ⟨-, rfl⟩
    · exact slotsWF_cons (slotsWF_refTo (vtwf_modelInst hwf
        (hc.field hsd hcs (mem_allFields_of_structFields hfd)) hq)) (slotsWF_tail hst)
    · exact slotsWF_prim (by trivial) (slotsWF_tail hst)
  case mutBorrowVariantFieldGeneric ii | immBorrowVariantFieldGeneric ii =>
    obtain ⟨fi, hfi, tys, htys, h⟩ := h
    rw [lookup_ok] at hfi htys
    obtain ⟨op, rest, fh, sd, rfl, -, hfh, hsd, -, hne, ft, rfl, hall⟩ := variantBorrowGen_full h
    obtain ⟨x, vfh, cs, hx, hvfh, hcs, args, hargs, hwf⟩ := hl5' (by simp)
    rw [hfi, Option.some.injEq] at hx
    subst hx
    rw [hfh, Option.some.injEq] at hvfh
    subst hvfh
    rw [htys, Option.some.injEq] at hargs
    subst hargs
    obtain ⟨v, hv⟩ := List.exists_mem_of_ne_nil _ hne
    obtain ⟨fd, hfd, hq⟩ := hall v hv
    exact slotsWF_cons (slotsWF_refTo (vtwf_modelInst hwf
      (hc.field hsd hcs (mem_allFields_of_structFields hfd)) hq)) (slotsWF_tail hst)
  case mutBorrowGlobalGeneric ii | immBorrowGlobalGeneric ii =>
    obtain ⟨⟨d, sd, tys⟩, hparts, h⟩ := h
    rw [structInstParts_ok] at hparts
    obtain ⟨si, hsi, rfl, hsd, htys⟩ := hparts
    obtain ⟨x, cs, hx, hcs, args, hargs, hwf⟩ := hl5' (by simp)
    rw [hsi, Option.some.injEq] at hx
    subst hx
    rw [htys, Option.some.injEq] at hargs
    subst hargs
    obtain ⟨rest, sd', a, rfl, hsd', -, -, rfl⟩ := borrowGlobalGen_ok h
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    exact slotsWF_cons (slotsWF_refTo (vtwf_materialize hsd hcs hwf)) (slotsWF_tail hst)
  case existsGeneric ii | moveFromGeneric ii =>
    obtain ⟨⟨d, sd, tys⟩, hparts, a, -, u, -, ⟨op, rest⟩, hpop, u', -, h⟩ := h
    rw [structInstParts_ok] at hparts
    obtain ⟨si, hsi, rfl, hsd, htys⟩ := hparts
    rw [popType_ok] at hpop
    subst hpop
    simp only [Except.ok.injEq] at h
    subst h
    first
      | exact slotsWF_prim (by trivial) (slotsWF_tail hst)
      | obtain ⟨x, cs, hx, hcs, args, hargs, hwf⟩ := hl5' (by simp)
        rw [hsi, Option.some.injEq] at hx
        subst hx
        rw [htys, Option.some.injEq] at hargs
        subst hargs
        exact slotsWF_prim (vtwf_materialize hsd hcs hwf) (slotsWF_tail hst)
  case moveToGeneric ii =>
    obtain ⟨⟨d, sd, tys⟩, -, a, -, u, -, ⟨k, rest⟩, hpop, ⟨s, rest'⟩, hpop', u', -, u'', -, h⟩ :=
      h
    rw [popType_ok] at hpop hpop'
    subst hpop hpop'
    simp only [Except.ok.injEq] at h
    subst h
    exact slotsWF_tail (slotsWF_tail hst)
  case packClosure hh mask =>
    obtain ⟨fh, hfh, h⟩ := h
    rw [lookup_ok] at hfh
    obtain ⟨params, caps, rest, ab, returns, t, hps, rfl, -, hab, -, hrs, ht, rfl⟩ :=
      typeClosPack_ok h
    have hnp : fh.typeParameters = [] := by
      simp only [Bytecode.OperandsConsistent, CompiledModule.functionTypeParamCount, hfh, bind,
        Option.bind_some, pure, Option.some.injEq] at hl9
      exact List.eq_nil_of_length_eq_zero hl9
    obtain ⟨⟨ps, hps', hpwf⟩, ⟨rs, hrs', hrwf⟩⟩ := hc.handle hfh
    rw [hps, Option.some.injEq] at hps'
    subst hps'
    rw [hrs, Option.some.injEq] at hrs'
    subst hrs'
    have hσ : SignatureToken.ArgumentsWellFormed m view.handle.typeParameters [] fh.typeParameters :=
      by rw [hnp]; trivial
    exact slotsWF_prim (vtwf_modelInst hσ (vtwf_closureType hpwf hrwf hab) ht)
      (slotsWF_append.1 hst).2
  case packClosureGeneric ii mask =>
    obtain ⟨fi, hfi, tys, htys, fh, hfh, h⟩ := h
    rw [lookup_ok] at hfi htys hfh
    obtain ⟨params, caps, rest, ab, returns, t, hps, rfl, -, hab, -, hrs, ht, rfl⟩ :=
      typeClosPack_ok h
    obtain ⟨x, fh', hx, hfh', args, hargs, hwf⟩ := hl5' (by simp)
    rw [hfi, Option.some.injEq] at hx
    subst hx
    rw [hfh, Option.some.injEq] at hfh'
    subst hfh'
    rw [htys, Option.some.injEq] at hargs
    subst hargs
    obtain ⟨⟨ps, hps', hpwf⟩, ⟨rs, hrs', hrwf⟩⟩ := hc.handle hfh
    rw [hps, Option.some.injEq] at hps'
    subst hps'
    rw [hrs, Option.some.injEq] at hrs'
    subst hrs'
    exact slotsWF_prim (vtwf_modelInst hwf (vtwf_closureType hpwf hrwf hab) ht)
      (slotsWF_append.1 hst).2
  case callClosure idx =>
    obtain ⟨sig, -, h⟩ := h
    split at h
    · next t ht =>
      obtain ⟨params, results, a, clo, args, rest, rfl, rfl, hass, -, -, rfl⟩ :=
        typeCallClosure_ok h
      obtain ⟨ab, rfl, -⟩ := assignable_function hass
      have hclo : (SignatureToken.function params results ab).ValueTypeWellFormed m
          view.handle.typeParameters := (slotsWF_head hst).1
      exact slotsWF_append.2 ⟨slotsWF_reverse hclo.2.1,
        (slotsWF_append.1 (slotsWF_tail hst)).2⟩
    · simp [panicAt] at h

/-- **L5 for `CallClosure`** at a well-formed stack: the closure's type is
the signature's, up to abilities the closure has, so the signature is a
well-formed function type. -/
theorem callClosure_wf {m : CompiledModule} {view : FunctionView} {s : Nat}
    {st st' : List SignatureToken}
    (hst : SignatureToken.SlotsWellFormed m view.handle.typeParameters st)
    (h : typeInstr m view st (.callClosure s) = .ok st') (hcl : ClosureOk m (.callClosure s)) :
    m.ValueSignatureWellFormed view.handle.typeParameters s := by
  obtain ⟨ps, rs, a, hs⟩ := hcl
  simp only [typeInstr, bind_ok, lookup_ok] at h
  obtain ⟨sig, hsig, h⟩ := h
  rw [hs, Option.some.injEq] at hsig
  subst hsig
  simp only [List.head?] at h
  obtain ⟨params, results, a', clo, args, rest, he, rfl, hass, -, -, -⟩ := typeCallClosure_ok h
  cases he
  obtain ⟨ab, rfl, hsub⟩ := assignable_function hass
  have hclo : (SignatureToken.function ps rs ab).ValueTypeWellFormed m
      view.handle.typeParameters := (slotsWF_head hst).1
  refine ⟨_, hs, fun t ht => ?_⟩
  simp only [List.mem_singleton] at ht
  subst ht
  exact ⟨hclo.1, hclo.2.1, key_of_subset hsub hclo.2.2⟩

/-! ## Over type safety -/

/-- An invariant of a fold's accumulator holds at every element. -/
theorem foldlM_inv_mem {α β ε : Type} {f : β → α → Except ε β} {P : β → Prop} :
    ∀ {l : List α} {b r : β}, P b → (∀ x ∈ l, ∀ c c', P c → f c x = .ok c' → P c') →
      l.foldlM f b = .ok r → P r ∧ ∀ x ∈ l, ∃ c c', P c ∧ f c x = .ok c'
  | [], b, r, hb, _, h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact ⟨hb, by simp⟩
  | y :: l, b, r, hb, hf, h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨b', hy, hl⟩ := h
    have hb' := hf y (List.mem_cons_self ..) b b' hb hy
    obtain ⟨hr, hall⟩ := foldlM_inv_mem hb' (fun x hx => hf x (List.mem_cons_of_mem _ hx)) hl
    refine ⟨hr, fun x hx => ?_⟩
    rcases List.mem_cons.1 hx with rfl | hx
    · exact ⟨b, b', hb, hy⟩
    · exact hall x hx

/-- Type safety type-checks every instruction at an abstract stack an
invariant of the instructions holds of. -/
theorem typed_inv_of_typeSafety {m : CompiledModule} {view : FunctionView}
    {P : List SignatureToken → Prop} (hts : verifyTypeSafety m view = .ok ())
    (hblocks : view.cfg.blocks = buildBlocks view.code.code) (h0 : P [])
    (hpres : ∀ i ∈ view.code.code.toList, ∀ st st', P st → typeInstr m view st i = .ok st' →
      P st') :
    ∀ i ∈ view.code.code.toList, ∃ st st', P st ∧ typeInstr m view st i = .ok st' := by
  have hstep : ∀ (pc : Nat) (st st' : List SignatureToken), P st →
      typeStep m view st pc = .ok st' → P st' := by
    intro pc st st' hp hs
    obtain ⟨i, hi, hty⟩ := typeStep_ok hs
    exact hpres i (getElem?_mem_toList hi) st st' hp hty
  have hblock : ∀ (b : BasicBlock) (st st' : List SignatureToken), P st →
      typeBlock m view st b = .ok st' → P st' := by
    intro b st st' hp hb
    unfold typeBlock at hb
    exact (foldlM_inv_mem hp (fun pc _ => hstep pc) hb).1
  intro i hi
  obtain ⟨pc, hpc⟩ := List.mem_iff_getElem?.1 hi
  rw [Array.getElem?_toList] at hpc
  obtain ⟨b, hb, h1, h2⟩ := (buildBlocks_spec view.code.code).2 pc (lt_of_getElem? hpc)
  unfold verifyTypeSafety at hts
  simp only [bind_ok] at hts
  obtain ⟨_, hfold, -⟩ := hts
  rw [hblocks] at hfold
  obtain ⟨-, hall⟩ := foldlM_inv_mem h0 (fun b _ => hblock b) hfold
  obtain ⟨st0, r, hp0, hblk⟩ := hall b hb
  unfold typeBlock at hblk
  have hmem : pc ∈ List.range' b.start (b.exit + 1 - b.start) := by
    rw [List.mem_range'_1]; omega
  obtain ⟨-, hall'⟩ := foldlM_inv_mem hp0 (fun pc _ => hstep pc) hblk
  obtain ⟨st, st', hp, hs⟩ := hall' pc hmem
  obtain ⟨i', hi', hty⟩ := typeStep_ok hs
  rw [hpc, Option.some.injEq] at hi'
  subst hi'
  exact ⟨st, st', hp, hty⟩

end MoveBytecode.Proofs
