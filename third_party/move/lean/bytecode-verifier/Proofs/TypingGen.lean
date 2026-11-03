-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.TypeFormation
import Proofs.InstanceAbilities

/-!
# The type certificate C1 at every instance

UNTRUSTED (checked by the kernel). The model's type-safety pass computes an
abstract stack of signature tokens over a function's type parameters. At an
instance of the function, whose type arguments satisfy its constraints,
translating the stacks under the type arguments gives a certificate that is
consistent in the specification's sense (`TypeCertificate.Consistent`): the
model's acceptance of an instruction implies the specification's typing rule
`typeAfter` at the instance.

This generalizes `Proofs/Typing.lean`, which translates closed tokens, to
generic code:

- a generic form's types, as the model substitutes them, are the named
  entity's own types at the instance the specification builds
  (`ofTokenWith_instantiate`, `Proofs/Instances.lean`);
- the abilities the model computes are at most the instance's
  (`abilities_sub_gen`, `Proofs/InstanceAbilities.lean`), given the
  constraint-satisfying type arguments of the instance and K1;
- the type arguments a generic form supplies are types of the program
  satisfying the constraints they must (`translateList_ok`), from L5;
- an entity named by a non-generic form has closed types (L9 and the
  supported subset), which translate the same under any type arguments.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Signature tokens under type arguments -/

section TokensW

variable {m : CompiledModule} {τ : List RuntimeType}

theorem ofSigW_nil : RuntimeType.ofSignatureWith m τ [] = some [] := rfl

theorem ofSlotW_of_ofTokenW {t : SignatureToken} {u : RuntimeType}
    (h : RuntimeType.ofTokenWith m τ t = some u) : RuntimeType.ofSlotTokenWith m τ t = some u :=
  ofTokenWith_not_ref h

theorem ofTokenW_of_ofSlotW {t : SignatureToken} {u : RuntimeType}
    (h : RuntimeType.ofSlotTokenWith m τ t = some u) (hr : t.isRefToken = false) :
    RuntimeType.ofTokenWith m τ t = some u := by
  cases t <;> first | exact h | (simp [SignatureToken.isRefToken] at hr)

theorem ofSigW_cons {t : SignatureToken} {ts : List SignatureToken} {r : List RuntimeType} :
    RuntimeType.ofSignatureWith m τ (t :: ts) = some r ↔
      ∃ u us, RuntimeType.ofSlotTokenWith m τ t = some u ∧
        RuntimeType.ofSignatureWith m τ ts = some us ∧ r = u :: us := by
  simp only [RuntimeType.ofSignatureWith, List.mapM_cons, bind, Option.bind_eq_some_iff, pure,
    Option.some.injEq]
  constructor
  · rintro ⟨u, hu, us, hus, rfl⟩; exact ⟨u, us, hu, hus, rfl⟩
  · rintro ⟨u, us, hu, hus, rfl⟩; exact ⟨u, hu, us, hus, rfl⟩

theorem ofSigW_cons_slot {t : SignatureToken} {ts : List SignatureToken} {u : RuntimeType}
    {us : List RuntimeType} (hu : RuntimeType.ofSlotTokenWith m τ t = some u)
    (hus : RuntimeType.ofSignatureWith m τ ts = some us) :
    RuntimeType.ofSignatureWith m τ (t :: ts) = some (u :: us) :=
  ofSigW_cons.2 ⟨u, us, hu, hus, rfl⟩

theorem ofSigW_cons_eq {t : SignatureToken} {ts : List SignatureToken} {u : RuntimeType}
    {us : List RuntimeType} (hu : RuntimeType.ofTokenWith m τ t = some u)
    (hus : RuntimeType.ofSignatureWith m τ ts = some us) :
    RuntimeType.ofSignatureWith m τ (t :: ts) = some (u :: us) :=
  ofSigW_cons_slot (ofSlotW_of_ofTokenW hu) hus

theorem ofSlotTokensW_eq :
    ∀ ts : List SignatureToken,
      RuntimeType.ofSlotTokensWith m τ ts = RuntimeType.ofSignatureWith m τ ts
  | [] => rfl
  | t :: ts => by
    have ih := ofSlotTokensW_eq ts
    simp only [RuntimeType.ofSignatureWith, List.mapM_cons] at ih ⊢
    cases t <;> simp [RuntimeType.ofSlotTokensWith, RuntimeType.ofSlotTokenWith, ih] <;>
      (rename_i inner; cases RuntimeType.ofTokenWith m τ inner <;> rfl)

theorem ofSigW_append {a b : List SignatureToken} {r : List RuntimeType} :
    RuntimeType.ofSignatureWith m τ (a ++ b) = some r ↔
      ∃ ra rb, RuntimeType.ofSignatureWith m τ a = some ra ∧
        RuntimeType.ofSignatureWith m τ b = some rb ∧ r = ra ++ rb := by
  induction a generalizing r with
  | nil => simp [ofSigW_nil]
  | cons t ts ih =>
    rw [List.cons_append, ofSigW_cons]
    constructor
    · rintro ⟨u, us, hu, hus, rfl⟩
      obtain ⟨ra, rb, ha, hb, rfl⟩ := ih.1 hus
      exact ⟨u :: ra, rb, ofSigW_cons_slot hu ha, hb, rfl⟩
    · rintro ⟨ra, rb, ha, hb, rfl⟩
      obtain ⟨u, us, hu, hus, rfl⟩ := ofSigW_cons.1 ha
      exact ⟨u, us ++ rb, hu, ih.2 ⟨us, rb, hus, hb, rfl⟩, rfl⟩

theorem ofSigW_append_eq {a b : List SignatureToken} {ra rb : List RuntimeType}
    (ha : RuntimeType.ofSignatureWith m τ a = some ra)
    (hb : RuntimeType.ofSignatureWith m τ b = some rb) :
    RuntimeType.ofSignatureWith m τ (a ++ b) = some (ra ++ rb) :=
  ofSigW_append.2 ⟨ra, rb, ha, hb, rfl⟩

theorem ofSigW_reverse {a : List SignatureToken} {r : List RuntimeType}
    (h : RuntimeType.ofSignatureWith m τ a = some r) :
    RuntimeType.ofSignatureWith m τ a.reverse = some r.reverse := by
  induction a generalizing r with
  | nil => simp at h ⊢; simpa [ofSigW_nil] using h
  | cons t ts ih =>
    obtain ⟨u, us, hu, hus, rfl⟩ := ofSigW_cons.1 h
    rw [List.reverse_cons, List.reverse_cons]
    exact ofSigW_append_eq (ih hus) (ofSigW_cons_slot hu ofSigW_nil)

theorem ofSigW_getElem {a : List SignatureToken} {r : List RuntimeType} {i : Nat}
    {t : SignatureToken} (h : RuntimeType.ofSignatureWith m τ a = some r) (ht : a[i]? = some t) :
    ∃ u, r[i]? = some u ∧ RuntimeType.ofSlotTokenWith m τ t = some u := by
  induction a generalizing r i with
  | nil => simp at ht
  | cons x xs ih =>
    obtain ⟨u, us, hu, hus, rfl⟩ := ofSigW_cons.1 h
    cases i with
    | zero => simp at ht; subst ht; exact ⟨u, rfl, hu⟩
    | succ i => simpa using ih hus (by simpa using ht)

theorem ofSigW_replicate {t : SignatureToken} {u : RuntimeType}
    (h : RuntimeType.ofTokenWith m τ t = some u) :
    ∀ n, RuntimeType.ofSignatureWith m τ (List.replicate n t) = some (List.replicate n u)
  | 0 => rfl
  | n + 1 => ofSigW_cons_eq h (ofSigW_replicate h n)

theorem ofSigW_tokens {st : List SignatureToken} {r : List RuntimeType}
    (h : RuntimeType.ofSignatureWith m τ st = some r) :
    ∀ a ∈ st, (RuntimeType.ofSlotTokenWith m τ a).isSome := by
  induction st generalizing r with
  | nil => simp
  | cons x xs ih =>
    obtain ⟨u, us, hu, hus, rfl⟩ := ofSigW_cons.1 h
    intro a ha
    rcases List.mem_cons.1 ha with rfl | ha
    · simp [hu]
    · exact ih hus a ha

/-- The translation of value tokens as a signature is their translation as
value types. -/
theorem ofSigW_of_tokenList {ts : List SignatureToken} {us : List RuntimeType}
    (h : RuntimeType.ofTokenListWith m τ ts = some us) :
    RuntimeType.ofSignatureWith m τ ts = some us := by
  induction ts generalizing us with
  | nil => simp [RuntimeType.ofTokenListWith] at h; subst h; rfl
  | cons t ts ih =>
    simp only [RuntimeType.ofTokenListWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨u, hu, us', hus', rfl⟩ := h
    exact ofSigW_cons_eq hu (ih hus')

/-- The field types of a struct, as the specification translates them. -/
theorem ofSigW_fields {fields : List FieldDefinition} {us : List RuntimeType}
    (h : fields.mapM (fun f => RuntimeType.ofTokenWith m τ f.signature) = some us) :
    RuntimeType.ofSignatureWith m τ (fields.map (·.signature)) = some us := by
  induction fields generalizing us with
  | nil => simp at h; subst h; rfl
  | cons f fs ih =>
    simp only [List.mapM_cons, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨u, hu, us', hus', rfl⟩ := h
    exact ofSigW_cons_eq hu (ih hus')

theorem isInteger_ofTokenW {t : SignatureToken} (h : isInteger t = true) :
    ∃ it, RuntimeType.ofTokenWith m τ t = some (.int it) := by
  cases t <;> simp_all [isInteger, RuntimeType.ofTokenWith]

theorem isUnsigned_ofTokenW {t : SignatureToken} (h : isUnsignedInteger t = true) :
    ∃ it, RuntimeType.ofTokenWith m τ t = some (.int it) ∧ it.isSigned = false := by
  cases t <;> simp_all [isUnsignedInteger, RuntimeType.ofTokenWith, IntType.isSigned]

theorem isSigned_ofTokenW {t : SignatureToken} (h : isSignedInteger t = true) :
    ∃ it, RuntimeType.ofTokenWith m τ t = some (.int it) ∧ it.isSigned = true := by
  cases t <;> simp_all [isSignedInteger, RuntimeType.ofTokenWith, IntType.isSigned]

/-- A token the model finds assignable from a value token that translates
translates too, to a type the other's translation is assignable to: function
types may lose abilities, and otherwise the tokens are equal. -/
theorem assignable_coreW {p a : SignatureToken} {ua : RuntimeType}
    (ha : RuntimeType.ofTokenWith m τ a = some ua) (h : isAssignableFrom p a = true) :
    ∃ up, RuntimeType.ofTokenWith m τ p = some up ∧ ua.assignable up = true := by
  unfold isAssignableFrom at h
  split at h
  · simp only [Bool.and_eq_true, beq_iff_eq] at h
    obtain ⟨⟨rfl, rfl⟩, hab⟩ := h
    simp only [RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at ha ⊢
    obtain ⟨xs, hxs, ys, hys, rfl⟩ := ha
    exact ⟨_, ⟨xs, hxs, ys, hys, rfl⟩, by simp [RuntimeType.assignable, hab]⟩
  · simp [RuntimeType.ofTokenWith] at ha
  · simp only [beq_iff_eq] at h
    subst h
    exact ⟨ua, ha, assignable_refl _⟩

/-- The same for slot tokens: immutable references are covariant. -/
theorem assignable_slotW {p a : SignatureToken} {ua : RuntimeType}
    (ha : RuntimeType.ofSlotTokenWith m τ a = some ua) (h : isAssignableFrom p a = true) :
    ∃ up, RuntimeType.ofSlotTokenWith m τ p = some up ∧ ua.assignable up = true := by
  cases a with
  | reference t2 =>
    cases p with
    | reference t1 =>
      simp only [isAssignableFrom] at h
      simp only [RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at ha ⊢
      obtain ⟨v, hv, rfl⟩ := ha
      obtain ⟨w, hw, hvw⟩ := assignable_coreW hv h
      exact ⟨_, ⟨w, hw, rfl⟩, by simpa [RuntimeType.assignable] using hvw⟩
    | _ => simp [isAssignableFrom] at h
  | mutableReference t2 =>
    have : p = .mutableReference t2 := by cases p <;> simp_all [isAssignableFrom]
    subst this
    exact ⟨ua, ha, assignable_refl _⟩
  | _ =>
    obtain ⟨up, hup, hassign⟩ := assignable_coreW (ofTokenW_of_ofSlotW ha rfl) h
    exact ⟨up, ofSlotW_of_ofTokenW hup, hassign⟩

/-- Arguments the model finds assignable, one for one, to tokens translate to
types assignable to the tokens' translations. -/
theorem allPairs_coreW {args ps : List SignatureToken} {ra : List RuntimeType}
    (h : AllPairs (fun a p => isAssignableFrom p a = true) args ps)
    (hra : RuntimeType.ofSignatureWith m τ args = some ra) :
    ∃ rp, RuntimeType.ofSignatureWith m τ ps = some rp ∧ RuntimeType.assignableAll ra rp = true := by
  induction h generalizing ra with
  | nil =>
    simp only [RuntimeType.ofSignatureWith, List.mapM_nil, pure, Option.some.injEq] at hra
    subst hra
    exact ⟨[], rfl, rfl⟩
  | cons hap _ ih =>
    obtain ⟨u, us, hu, hus, rfl⟩ := ofSigW_cons.1 hra
    obtain ⟨up, hup, h1⟩ := assignable_slotW hu hap
    obtain ⟨rp, hrp, h2⟩ := ih hus
    exact ⟨up :: rp, ofSigW_cons_slot hup hrp, by simp [RuntimeType.assignableAll, h1, h2]⟩

theorem elementType_supportedW {s : SignatureIndex} {elem : SignatureToken}
    (hops : (m.vectorElemTypeWith τ s).isSome) (h : elementType m s = .ok elem) :
    ∃ t, m.vectorElemTypeWith τ s = some t ∧ RuntimeType.ofTokenWith m τ elem = some t := by
  unfold elementType at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨sig, hsig, h⟩ := h
  unfold CompiledModule.vectorElemTypeWith at hops ⊢
  rw [hsig] at hops ⊢
  rcases sig with _ | ⟨t, _ | ⟨t2, rest⟩⟩
  · simp at hops
  · simp only [List.head?_cons, pure, Except.pure, Except.ok.injEq] at h
    subst h
    obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1 hops
    exact ⟨u, hu, hu⟩
  · simp at hops

theorem ofSlotW_refTo {mut_ : Bool} {t : SignatureToken} {u : RuntimeType}
    (h : RuntimeType.ofTokenWith m τ t = some u) :
    RuntimeType.ofSlotTokenWith m τ (refTo mut_ t) = some (.ref mut_ u) := by
  cases mut_ <;> simp [refTo, RuntimeType.ofSlotTokenWith, h]

theorem ofSlotW_refTarget {op inner : SignatureToken} {u : RuntimeType}
    (h : refTarget op = some inner) (hu : RuntimeType.ofSlotTokenWith m τ op = some u) :
    ∃ mut_ v, op = refTo mut_ inner ∧ RuntimeType.ofTokenWith m τ inner = some v ∧
      u = .ref mut_ v := by
  cases op <;> simp [refTarget] at h <;> subst h
  · simp only [RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at hu
    obtain ⟨v, hv, rfl⟩ := hu
    exact ⟨false, v, rfl, hv, rfl⟩
  · simp only [RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at hu
    obtain ⟨v, hv, rfl⟩ := hu
    exact ⟨true, v, rfl, hv, rfl⟩

theorem ofSlotW_vectorRef {mut' : Bool} {e : SignatureToken} {t : RuntimeType}
    (het : RuntimeType.ofTokenWith m τ e = some t) {u : RuntimeType}
    (hu : RuntimeType.ofSlotTokenWith m τ (refTo mut' (.vector e)) = some u) :
    u = .ref mut' (.vector t) := by
  cases mut' <;> simp_all [refTo, RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith]

/-- A closed token, one that translates under no type arguments, translates
the same under any. -/
theorem ofSigW_closed {sig : Signature} {r : List RuntimeType}
    (h : RuntimeType.ofSignature m sig = some r) : RuntimeType.ofSignatureWith m τ sig = some r :=
  ofSignatureWith_closed h

end TokensW


/-! ## The context of one function instance -/

/-- What the typing of an instance `f` of a function of module `m` (at index
`f.module`) needs: the instance's type arguments satisfy the function's
constraints, the module's struct handles are conservative views (K1), and
the facts of the supported subset about the module's signatures, constants
and struct definitions, and the module's own struct definitions resolving to
themselves (L2). -/
structure GenCtx (P : Program) (f : FunctionRef) (m : CompiledModule) (view : FunctionView) :
    Prop where
  module : P.modules[f.module]? = some m
  resolves : HandlesResolve P m
  targs : RuntimeType.argumentsWellFormed P f.typeArgs view.handle.typeParameters = true
  locals : ∃ r, P.localTypes f = some r ∧
    RuntimeType.ofSignatureWith m f.typeArgs (view.parameters ++ view.locals) = some r
  returns : ∃ r, P.returnTypes f = some r ∧
    RuntimeType.ofSignatureWith m f.typeArgs view.returns = some r
  constants : ∀ c ∈ m.constantPool.toList, (RuntimeType.ofToken m c.type).isSome
  handles : ∀ fh ∈ m.functionHandles.toList,
    m.SupportedSignature fh.parameters fh.typeParameters.length ∧
    m.SupportedSignature fh.returns fh.typeParameters.length
  fields : ∀ d ∈ m.structDefs.toList, ∃ sh, m.structDefHandle d = some sh ∧
    ∀ fd ∈ CompiledModule.StructFieldInformation.allFields d.fieldInformation,
      ∀ tys : List RuntimeType, tys.length = sh.typeParameters.length →
        (RuntimeType.ofTokenWith m tys fd.signature).isSome
  own : ∀ d sd, m.structDefs[d]? = some sd → ∃ id name sh,
    m.structHandles[sd.structHandle]? = some sh ∧
    m.structIdAt sd.structHandle = some ⟨id, name, []⟩ ∧
    P.definedStruct? f.module d = some ⟨id, name, []⟩ ∧
    P.resolveStruct ⟨id, name, []⟩ = some ⟨f.module, d⟩ ∧
    P.struct? ⟨f.module, d⟩ = some ⟨m, id, sd, sh⟩
  /-- Every function handle resolves, to a storable function when the handle
  says `persistent` (K1, `PersistenceResolution`). -/
  targets : ∀ h fh, m.functionHandles[h]? = some fh → ∃ target, P.resolve f.module h = some target ∧
    (FunctionAttribute.persistent ∈ fh.attributes → P.storable target = true)

section Ctx

variable {P : Program} {f : FunctionRef} {m : CompiledModule} {view : FunctionView}

theorem GenCtx.targsLen (hc : GenCtx P f m view) :
    f.typeArgs.length = view.handle.typeParameters.length :=
  argumentsWellFormed_length hc.targs

theorem GenCtx.targsValues (hc : GenCtx P f m view) : ∀ x ∈ f.typeArgs, x.isValue = true :=
  argumentsWellFormed_values hc.targs

/-- A local's type at the instance. -/
theorem GenCtx.localType (hc : GenCtx P f m view) {i : Nat} {t : SignatureToken}
    (h : localAt view i = .ok t) :
    ∃ u, RuntimeType.ofSlotTokenWith m f.typeArgs t = some u ∧
      (do (← P.localTypes f)[i]?) = some u := by
  obtain ⟨r, hr, hsig⟩ := hc.locals
  obtain ⟨u, hu, ht⟩ := ofSigW_getElem hsig (localAt_ok h)
  exact ⟨u, ht, by simp [hr, hu]⟩

/-- The field signatures of a struct definition of the module, at any type
arguments of its arity. -/
theorem GenCtx.fieldsAt (hc : GenCtx P f m view) {d : Nat} {sd : StructDefinition}
    (hsd : m.structDefs[d]? = some sd) {sh : StructHandle}
    (hsh : m.structHandles[sd.structHandle]? = some sh) {fields : List FieldDefinition}
    (hf : ∀ fd ∈ fields, fd ∈ CompiledModule.StructFieldInformation.allFields sd.fieldInformation)
    {tys : List RuntimeType} (hlen : tys.length = sh.typeParameters.length) :
    ∃ us, fields.mapM (fun fd => RuntimeType.ofTokenWith m tys fd.signature) = some us := by
  obtain ⟨sh', hsh', hall⟩ := hc.fields sd (getElem?_mem_toList hsd)
  have hsh'' : m.structHandles[sd.structHandle]? = some sh' := hsh'
  rw [hsh, Option.some.injEq] at hsh''
  subst hsh''
  clear hsh'
  induction fields with
  | nil => exact ⟨[], rfl⟩
  | cons fd fs ih =>
    obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1 (hall fd (hf fd (List.mem_cons_self ..)) tys hlen)
    obtain ⟨us, hus⟩ := ih (fun g hg => hf g (List.mem_cons_of_mem _ hg))
    exact ⟨u :: us, by simp [hu, hus]⟩

/-- The struct type a definition of the module defines, at type arguments of
its arity that are value types: its own declaration. -/
theorem GenCtx.instance (hc : GenCtx P f m view) {d : Nat} {sd : StructDefinition}
    (hsd : m.structDefs[d]? = some sd) :
    ∃ id name sh, m.structHandles[sd.structHandle]? = some sh ∧
      m.structIdAt sd.structHandle = some ⟨id, name, []⟩ ∧
      P.definedStruct? f.module d = some ⟨id, name, []⟩ ∧
      ∀ tys : List RuntimeType, tys.length = sh.typeParameters.length →
        (∀ x ∈ tys, x.isValue = true) →
        P.structType? ⟨id, name, tys⟩ = some ⟨m, id, sd, sh⟩ := by
  obtain ⟨id, name, sh, hsh, hsid, hdef, hres, hst⟩ := hc.own d sd hsd
  refine ⟨id, name, sh, hsh, hsid, hdef, fun tys hlen hv => ?_⟩
  exact structType?_instance (s := ⟨id, name, []⟩) hres hst hlen hv

theorem fieldTypes_of_inst {P : Program} {s : StructId} {info : StructInfo}
    (hst : P.structType? s = some info) {fields : List FieldDefinition}
    (hfi : info.definition.fieldInformation = .declared fields) :
    P.fieldTypes s =
      fields.mapM (fun fd => RuntimeType.ofTokenWith info.module s.typeArgs fd.signature) := by
  simp [Program.fieldTypes, hst, hfi]

theorem variantFieldTypes_of_inst {P : Program} {s : StructId} {info : StructInfo}
    (hst : P.structType? s = some info) {variants : List VariantDefinition}
    (hfi : info.definition.fieldInformation = .declaredVariants variants) {v : Nat}
    {vd : VariantDefinition} (hvd : variants[v]? = some vd) :
    P.variantFieldTypes s v =
      vd.fields.mapM (fun fd => RuntimeType.ofTokenWith info.module s.typeArgs fd.signature) := by
  simp [Program.variantFieldTypes, hst, hfi, hvd]

/-- A struct definition a non-generic form names (L9): no type parameters,
closed field types. -/
theorem GenCtx.closedStruct (hc : GenCtx P f m view) {d : Nat} {sd : StructDefinition}
    (hsd : m.structDefs[d]? = some sd) (hcount : m.structTypeParamCount d = some 0) :
    ∃ s sh id, P.definedStruct? f.module d = some s ∧
      m.structHandles[sd.structHandle]? = some sh ∧ sh.typeParameters = [] ∧
      RuntimeType.ofTokenWith m f.typeArgs (.struct sd.structHandle) = some (.struct s) ∧
      P.structType? s = some ⟨m, id, sd, sh⟩ ∧ s.typeArgs = [] := by
  obtain ⟨id, name, sh, hsh, hsid, hdef, hinst⟩ := hc.instance hsd
  have hnp : sh.typeParameters = [] := by
    simp only [CompiledModule.structTypeParamCount, hsd, hsh, bind, Option.bind_some, pure,
      Option.some.injEq] at hcount
    exact List.eq_nil_of_length_eq_zero hcount
  refine ⟨⟨id, name, []⟩, sh, id, hdef, hsh, hnp, ?_, hinst [] (by simp [hnp]) (by simp), rfl⟩
  simp [RuntimeType.ofTokenWith, hsh, hnp, hsid]

end Ctx


/-! ## Substituted tokens -/

section Subst

variable {m : CompiledModule} {τ : List RuntimeType}

/-- Arguments assignable to instantiated parameter types translate to types
assignable to the parameter types at the instance. -/
theorem allPairs_instW {tys : List SignatureToken} (hne : tys ≠ []) {σ' : List RuntimeType}
    (hσ : RuntimeType.ofTokenListWith m τ tys = some σ') :
    ∀ {args ps : List SignatureToken} {ra : List RuntimeType},
      AllPairs (fun a p => ∃ q, instantiate tys p = .ok q ∧ isAssignableFrom q a = true) args ps →
      RuntimeType.ofSignatureWith m τ args = some ra →
      ∃ rp, RuntimeType.ofSignatureWith m σ' ps = some rp ∧ RuntimeType.assignableAll ra rp = true
  | [], [], _, _, hra => by
    simp only [RuntimeType.ofSignatureWith, List.mapM_nil, pure, Option.some.injEq] at hra
    subst hra
    exact ⟨[], rfl, rfl⟩
  | a :: args, p :: ps, _, .cons ⟨q, hq, haq⟩ hrest, hra => by
    obtain ⟨u, us, hu, hus, rfl⟩ := ofSigW_cons.1 hra
    obtain ⟨up, hup, h1⟩ := assignable_slotW hu haq
    rw [ofSlotTokenWith_instantiate hσ (instantiate_spec hne hq)] at hup
    obtain ⟨rp, hrp, h2⟩ := allPairs_instW hne hσ hrest hus
    exact ⟨up :: rp, ofSigW_cons_slot hup hrp, by simp [RuntimeType.assignableAll, h1, h2]⟩

/-- Instantiated return types translate as the return types at the instance. -/
theorem mapM_instW {tys : List SignatureToken} (hne : tys ≠ []) {σ' : List RuntimeType}
    (hσ : RuntimeType.ofTokenListWith m τ tys = some σ') {l l' : List SignatureToken}
    (h : l.mapM (instantiate tys) = .ok l') :
    RuntimeType.ofSignatureWith m τ l' = RuntimeType.ofSignatureWith m σ' l :=
  ofSignatureWith_instantiate hσ (mapM_instantiate_spec hne h)

theorem typeArgsWith_eq {sig : SignatureIndex} {toks : List SignatureToken}
    (hsig : m.signatures[sig]? = some toks) :
    m.typeArgsWith τ sig = RuntimeType.ofTokenListWith m τ toks := by
  simp [CompiledModule.typeArgsWith, hsig]

/-- The struct type a generic form names, at the instance. -/
theorem ofTokenWith_materialize {h : StructHandleIndex} {sh : StructHandle} {id : ModuleId}
    {name : String} (hsh : m.structHandles[h]? = some sh)
    (hsid : m.structIdAt h = some ⟨id, name, []⟩) {toks : List SignatureToken} (hne : toks ≠ [])
    {σ' : List RuntimeType} (hσ : RuntimeType.ofTokenListWith m τ toks = some σ')
    (hlen : σ'.length = sh.typeParameters.length) :
    RuntimeType.ofTokenWith m τ (materializeType h toks) = some (.struct ⟨id, name, σ'⟩) := by
  have hne' : toks.isEmpty = false := by cases toks <;> simp_all
  simp only [materializeType, hne', Bool.false_eq_true, ↓reduceIte, RuntimeType.ofTokenWith, hsh,
    hsid, hσ, bind, Option.bind_some, hlen, ↓reduceIte, pure]

end Subst

section Ctx2

variable {P : Program} {f : FunctionRef} {m : CompiledModule} {view : FunctionView}

/-- A non-generic handle's signatures are closed; at the instance they are the
types the handle declares. -/
theorem GenCtx.handleTypes0 (hc : GenCtx P f m view) {h : FunctionHandleIndex}
    {fh : FunctionHandle} (hfh : m.functionHandles[h]? = some fh) (hnp : fh.typeParameters = [])
    {params returns : Signature} (hp : m.signatures[fh.parameters]? = some params)
    (hr : m.signatures[fh.returns]? = some returns) :
    ∃ ps rs, RuntimeType.ofSignatureWith m f.typeArgs params = some ps ∧
      RuntimeType.ofSignatureWith m f.typeArgs returns = some rs ∧
      P.handleParamTypesAt f.module h [] = some ps ∧
      P.handleReturnTypesAt f.module h [] = some rs := by
  obtain ⟨⟨psig, hpsig, hps⟩, ⟨rsig, hrsig, hrs⟩⟩ := hc.handles fh (getElem?_mem_toList hfh)
  rw [hp, Option.some.injEq] at hpsig
  rw [hr, Option.some.injEq] at hrsig
  subst hpsig hrsig
  obtain ⟨ps, hps⟩ := Option.isSome_iff_exists.1 (hps [] (by simp [hnp]))
  obtain ⟨rs, hrs⟩ := Option.isSome_iff_exists.1 (hrs [] (by simp [hnp]))
  exact ⟨ps, rs, ofSignatureWith_closed hps, ofSignatureWith_closed hrs,
    by simp [Program.handleParamTypesAt, hc.module, hfh, hp, hps],
    by simp [Program.handleReturnTypesAt, hc.module, hfh, hr, hrs]⟩

/-- A generic handle's signatures at the type arguments `σ'` of its arity. -/
theorem GenCtx.handleTypesAt (hc : GenCtx P f m view) {h : FunctionHandleIndex}
    {fh : FunctionHandle} (hfh : m.functionHandles[h]? = some fh)
    {params returns : Signature} (hp : m.signatures[fh.parameters]? = some params)
    (hr : m.signatures[fh.returns]? = some returns) {σ' : List RuntimeType}
    (hlen : σ'.length = fh.typeParameters.length) :
    ∃ ps rs, RuntimeType.ofSignatureWith m σ' params = some ps ∧
      RuntimeType.ofSignatureWith m σ' returns = some rs ∧
      P.handleParamTypesAt f.module h σ' = some ps ∧
      P.handleReturnTypesAt f.module h σ' = some rs := by
  obtain ⟨⟨psig, hpsig, hps⟩, ⟨rsig, hrsig, hrs⟩⟩ := hc.handles fh (getElem?_mem_toList hfh)
  rw [hp, Option.some.injEq] at hpsig
  rw [hr, Option.some.injEq] at hrsig
  subst hpsig hrsig
  obtain ⟨ps, hps⟩ := Option.isSome_iff_exists.1 (hps σ' hlen)
  obtain ⟨rs, hrs⟩ := Option.isSome_iff_exists.1 (hrs σ' hlen)
  exact ⟨ps, rs, hps, hrs, by simp [Program.handleParamTypesAt, hc.module, hfh, hp, hps],
    by simp [Program.handleReturnTypesAt, hc.module, hfh, hr, hrs]⟩

/-- The instance a generic struct form names: the definition `d` of the module
at the type arguments its instantiation signature supplies, translated under
the instance's, is a type of the program. -/
theorem GenCtx.instAt (hc : GenCtx P f m view) {d : Nat} {sd : StructDefinition}
    (hsd : m.structDefs[d]? = some sd) {sig : SignatureIndex} {toks : List SignatureToken}
    (hsig : m.signatures[sig]? = some toks) (hne : toks ≠ [])
    (hcount : m.structTypeParamCount d = some toks.length)
    (hops : m.SupportedValueSignature sig view.handle.typeParameters.length)
    (hwf : ∃ cs, m.structConstraints d = some cs ∧
      m.InstantiationWellFormed view.handle.typeParameters sig cs) :
    ∃ id name sh σ', m.structHandles[sd.structHandle]? = some sh ∧
      m.structIdAt sd.structHandle = some ⟨id, name, []⟩ ∧
      RuntimeType.ofTokenListWith m f.typeArgs toks = some σ' ∧
      σ'.length = sh.typeParameters.length ∧
      P.definedStruct? f.module d = some ⟨id, name, []⟩ ∧
      P.structType? ⟨id, name, σ'⟩ = some ⟨m, id, sd, sh⟩ ∧
      (RuntimeType.struct ⟨id, name, σ'⟩).wellFormed P = true ∧
      m.typeArgsWith f.typeArgs sig = some σ' ∧
      RuntimeType.ofTokenWith m f.typeArgs (materializeType sd.structHandle toks) =
        some (.struct ⟨id, name, σ'⟩) := by
  obtain ⟨id, name, sh, hsh, hsid, hdef, hinst⟩ := hc.instance hsd
  have hcount' : toks.length = sh.typeParameters.length := by
    simp only [CompiledModule.structTypeParamCount, hsd, hsh, bind, Option.bind_some, pure,
      Option.some.injEq] at hcount
    exact hcount.symm
  obtain ⟨sig', hsig', hall⟩ := hops
  rw [hsig, Option.some.injEq] at hsig'
  subst hsig'
  obtain ⟨σ', hσ⟩ := Option.isSome_iff_exists.1 (hall f.typeArgs hc.targsLen)
  have hlen : σ'.length = sh.typeParameters.length := by
    rw [ofTokenListWith_length hσ, hcount']
  have hv := ofTokenListWith_values hc.targsValues hσ
  have hst := hinst σ' hlen hv
  obtain ⟨cs, hcs, args, hargs, hwfa⟩ := hwf
  rw [hsig, Option.some.injEq] at hargs
  subst hargs
  have hok := translateList_ok hc.resolves hc.targs hwfa hσ
  have hcs' : cs = sh.typeParameters.map (·.constraints) := by
    simp only [CompiledModule.structConstraints, hsd, CompiledModule.structDefHandle, hsh, bind,
      Option.bind_some, pure, Option.some.injEq] at hcs
    exact hcs.symm
  subst hcs'
  refine ⟨id, name, sh, σ', hsh, hsid, hσ, hlen, hdef, hst, ?_, by rw [typeArgsWith_eq hsig, hσ],
    ofTokenWith_materialize hsh hsid hne hσ hlen⟩
  simp only [RuntimeType.wellFormed, hst]
  exact hok

end Ctx2


section Ctx3

variable {P : Program} {f : FunctionRef} {m : CompiledModule} {view : FunctionView}

/-- The instance a generic struct form names, from its L9, L5 and subset
facts. -/
theorem GenCtx.instOf (hc : GenCtx P f m view) {d : Nat} {sd : StructDefinition}
    (hsd : m.structDefs[d]? = some sd) {sig : SignatureIndex} {toks : List SignatureToken}
    (hsig : m.signatures[sig]? = some toks)
    (hgen : m.GenericInstance (m.structTypeParamCount d) sig)
    (hops : m.SupportedValueSignature sig view.handle.typeParameters.length)
    (hwf : ∃ cs, m.structConstraints d = some cs ∧
      m.InstantiationWellFormed view.handle.typeParameters sig cs) :
    toks ≠ [] ∧ ∃ id name sh σ', m.structHandles[sd.structHandle]? = some sh ∧
      m.structIdAt sd.structHandle = some ⟨id, name, []⟩ ∧
      RuntimeType.ofTokenListWith m f.typeArgs toks = some σ' ∧
      σ'.length = sh.typeParameters.length ∧
      P.definedStruct? f.module d = some ⟨id, name, []⟩ ∧
      P.structType? ⟨id, name, σ'⟩ = some ⟨m, id, sd, sh⟩ ∧
      (RuntimeType.struct ⟨id, name, σ'⟩).wellFormed P = true ∧
      m.typeArgsWith f.typeArgs sig = some σ' ∧
      RuntimeType.ofTokenWith m f.typeArgs (materializeType sd.structHandle toks) =
        some (.struct ⟨id, name, σ'⟩) := by
  obtain ⟨n, args, hcount, hpos, hargs, hlen⟩ := hgen
  rw [hsig, Option.some.injEq] at hargs
  subst hargs
  have hne : toks ≠ [] := by intro h0; subst h0; simp at hlen; omega
  exact ⟨hne, hc.instAt hsd hsig hne (by rw [hcount, hlen]) hops hwf⟩

/-- The instance is a nonempty list of type arguments. -/
theorem cons_of_length_pos {α : Type} {l : List α} (h : l ≠ []) : ∃ x xs, l = x :: xs := by
  cases l with
  | nil => exact absurd rfl h
  | cons x xs => exact ⟨x, xs, rfl⟩

end Ctx3


/-! ## Each instruction at an instance -/

/-- `PackClosure` and `PackClosureGeneric`. The specification gives the
closure the abilities of the captured arguments' types at the instance, which
may be more than the model's (Rust's) abilities of their tokens: the model's
type after is only assignable from the specification's, so `typeAfter_gen`
excludes them, and `typeAfter_closure` takes them. -/
def packForm : Bytecode → Bool
  | .packClosure _ _ | .packClosureGeneric _ _ => true
  | _ => false

set_option maxHeartbeats 4000000 in
/-- **C1 at an instance.** An instruction the model's type checker accepts
has the specification's typing at every instance of its function whose type
arguments satisfy the function's constraints: `typeAfter` applies to the
certified types, translated under the instance's type arguments, and gives
the certified types after it. -/
theorem typeAfter_gen {P : Program} {f : FunctionRef} {m : CompiledModule}
    {view : FunctionView} (hc : GenCtx P f m view) {i : Bytecode} (hsub : i.inSubset = true)
    (hnpk : packForm i = false)
    (hops : m.SupportedOperands i view.handle.typeParameters.length)
    (hl9 : i.OperandsConsistent m) (hl5 : i.TypeOperandsWellFormed m view.handle.typeParameters)
    {st st' : List SignatureToken} {before : List RuntimeType}
    (hb : verifyBounds m = .ok ())
    (hst : RuntimeType.ofSignatureWith m f.typeArgs st = some before)
    (h : typeInstr m view st i = .ok st')
    (hret : i = .ret → st' = []) :
    ∃ after, RuntimeType.ofSignatureWith m f.typeArgs st' = some after ∧
      typeAfter P f m i before = some after := by
  have hab := abilities_sub_slot_gen (P := P) (cs := view.handle.typeParameters) hc.resolves
    hc.targs
  have habv := abilities_sub_gen (P := P) (cs := view.handle.typeParameters) hc.resolves hc.targs
  cases i <;> simp [Bytecode.inSubset, Bytecode.inCore, Bytecode.inDataByValue,
    Bytecode.inReferences, Bytecode.inGenerics] at hsub <;>
    simp only [typeInstr, Model.castTo, typeCall, bind_ok, pure, Except.pure] at h
  case packClosure | packClosureGeneric => simp [packForm] at hnpk
  case callClosure s =>
    obtain ⟨sig, hsig, h⟩ := h
    rw [lookup_ok] at hsig
    obtain ⟨ps, rs, a, hs⟩ := hl9
    rw [hs, Option.some.injEq] at hsig
    subst hsig
    simp only [List.head?] at h
    obtain ⟨params, results, a', clo, args, rest, he, rfl, hass, -, hall, rfl⟩ :=
      typeCallClosure_ok h
    cases he
    obtain ⟨ab, rfl, hsub'⟩ := assignable_function hass
    obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
    simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff,
      pure, Option.some.injEq] at hu1
    obtain ⟨ps', hps', rs', hrs', rfl⟩ := hu1
    rw [ofSlotTokensW_eq] at hps' hrs'
    obtain ⟨ra, rb, hra, hrb, rfl⟩ := ofSigW_append.1 hus1
    obtain ⟨rp, hrp, hassign⟩ := allPairs_coreW hall hra
    rw [ofSigW_reverse hps', Option.some.injEq] at hrp
    subst hrp
    obtain ⟨htake, hdrop⟩ := assignable_take (rb := rb) hassign
    refine ⟨_, ofSigW_append_eq (ofSigW_reverse hrs') hrb, ?_⟩
    dsimp only [typeAfter]
    simp [hs, ofSlotTokensW_eq, hps', hrs', hsub', htake, hdrop, hassign]
  case mutBorrowField hd | immBorrowField hd =>
    obtain ⟨op, rest, fh, sd, rfl, hmut, hfh, hsd, htar, rfl⟩ := fieldBorrow_ok h
    have hcount : m.structTypeParamCount fh.owner = some 0 := by
      simpa [Bytecode.OperandsConsistent, CompiledModule.fieldTypeParamCount, hfh] using hl9
    obtain ⟨s, sh, id, hdef, hsh, hnp, hu, hty, hsta⟩ := hc.closedStruct hsd hcount
    obtain ⟨sd', hsd', hlt⟩ := verifyBounds_fieldHandle hb hfh
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    cases hfi : sd.fieldInformation with
    | declared fields =>
      rw [hfi] at hlt
      simp only [fieldCount] at hlt
      obtain ⟨fd, hfd⟩ : ∃ fd, fields[fh.field]? = some fd := ⟨_, List.getElem?_eq_getElem hlt⟩
      obtain ⟨tys, htys⟩ := hc.fieldsAt hsd hsh (fields := fields)
        (by simp [CompiledModule.StructFieldInformation.allFields, hfi]) (tys := []) (by simp [hnp])
      obtain ⟨ft, hft, hfdt⟩ := mapM_getElem? htys hfd
      have hfts : P.fieldTypes s = some tys := by
        rw [fieldTypes_of_inst hty hfi, hsta]; exact htys
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      obtain ⟨mut', v, rfl, hv, rfl⟩ := ofSlotW_refTarget htar hu1
      rw [hu, Option.some.injEq] at hv
      subst hv
      simp only [isMutableReference_refTo, Bool.and_eq_false_iff, Bool.not_eq_false'] at hmut
      simp only [structFields, hfd]
      refine ⟨_, ofSigW_cons_slot (ofSlotW_refTo (ofTokenWith_closed hfdt)) hus1, ?_⟩
      dsimp only [typeAfter]
      simp only [hfh, hdef, hfts, hft, bind, Option.bind_some]
      cases mut' <;> simp_all
    | declaredVariants _ => rw [hfi] at hlt; simp [fieldCount] at hlt
    | native => rw [hfi] at hlt; simp [fieldCount] at hlt
  case add | sub | mul | div | mod | lt | gt | le | ge =>
    obtain ⟨⟨op1, op2, rest⟩, hpop, h⟩ := h
    rw [popTwo_ok] at hpop
    subst hpop
    simp only at h
    split at h
    · next hcd =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hcd
      obtain ⟨hi, rfl⟩ := hcd
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨it, hit⟩ := isInteger_ofTokenW (m := m) (τ := f.typeArgs) hi
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      obtain ⟨u2, us2, hu2, hus2, rfl⟩ := ofSigW_cons.1 hus1
      rw [ofSlotW_of_ofTokenW hit, Option.some.injEq] at hu1 hu2
      subst hu1 hu2
      first
        | exact ⟨_, ofSigW_cons_eq hit hus2, by dsimp only [typeAfter]; simp [intBinary]⟩
        | exact ⟨_, ofSigW_cons_eq rfl hus2, by dsimp only [typeAfter]; simp [intBinary]⟩
    · simp [reject] at h
  case bitOr | bitAnd | xor =>
    obtain ⟨⟨op1, op2, rest⟩, hpop, h⟩ := h
    rw [popTwo_ok] at hpop
    subst hpop
    simp only at h
    split at h
    · next hcd =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hcd
      obtain ⟨hi, rfl⟩ := hcd
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨it, hit, hs⟩ := isUnsigned_ofTokenW (m := m) (τ := f.typeArgs) hi
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      obtain ⟨u2, us2, hu2, hus2, rfl⟩ := ofSigW_cons.1 hus1
      rw [ofSlotW_of_ofTokenW hit, Option.some.injEq] at hu1 hu2
      subst hu1 hu2
      exact ⟨_, ofSigW_cons_eq hit hus2, by dsimp only [typeAfter]; simp [intBinary, hs]⟩
    · simp [reject] at h
  case shl | shr =>
    obtain ⟨⟨op1, op2, rest⟩, hpop, h⟩ := h
    rw [popTwo_ok] at hpop
    subst hpop
    simp only at h
    split at h
    · next hcd =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hcd
      obtain ⟨hi, rfl⟩ := hcd
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨it, hit, hs⟩ := isUnsigned_ofTokenW (m := m) (τ := f.typeArgs) hi
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      obtain ⟨u2, us2, hu2, hus2, rfl⟩ := ofSigW_cons.1 hus1
      rw [ofSlotW_of_ofTokenW hit, Option.some.injEq] at hu2
      simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, Option.some.injEq] at hu1
      subst hu1 hu2
      exact ⟨_, ofSigW_cons_eq hit hus2, by dsimp only [typeAfter]; simp [hs]⟩
    · simp [reject] at h
  case or | and =>
    obtain ⟨⟨op1, op2, rest⟩, hpop, h⟩ := h
    rw [popTwo_ok] at hpop
    subst hpop
    simp only at h
    split at h
    · next hcd =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hcd
      obtain ⟨rfl, rfl⟩ := hcd
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      obtain ⟨u2, us2, hu2, hus2, rfl⟩ := ofSigW_cons.1 hus1
      simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, Option.some.injEq] at hu1 hu2
      subst hu1 hu2
      exact ⟨_, ofSigW_cons_eq rfl hus2, by dsimp only [typeAfter]⟩
    · simp [reject] at h
  case eq | neq =>
    obtain ⟨⟨op1, op2, rest⟩, hpop, ab, hab', h⟩ := h
    rw [popTwo_ok] at hpop
    subst hpop
    simp only at h
    split at h
    · next hcd =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hcd
      obtain ⟨hd, rfl⟩ := hcd
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      obtain ⟨u2, us2, hu2, hus2, rfl⟩ := ofSigW_cons.1 hus1
      rw [hu1, Option.some.injEq] at hu2
      subst hu2
      have hdrop := AbilitySet.drop_of_subset (hab op1 hab' hu1) hd
      exact ⟨_, ofSigW_cons_eq rfl hus2, by dsimp only [typeAfter]; simp [hdrop, comparable_refl]⟩
    · simp [reject] at h
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldTrue | ldFalse =>
    simp only [Except.ok.injEq] at h
    subst h
    exact ⟨_, ofSigW_cons_eq rfl hst, by dsimp only [typeAfter]⟩
  case nop | branch =>
    simp only [Except.ok.injEq] at h
    subst h
    exact ⟨_, hst, by dsimp only [typeAfter]⟩
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256
    | castI8 | castI16 | castI32 | castI64 | castI128 | castI256 =>
    obtain ⟨⟨op, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    simp only at h
    split at h
    · simp [reject] at h
    · next hi =>
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hi
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨it, hit⟩ := isInteger_ofTokenW (m := m) (τ := f.typeArgs) hi
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      rw [ofSlotW_of_ofTokenW hit, Option.some.injEq] at hu1
      subst hu1
      exact ⟨_, ofSigW_cons_eq rfl hus1, by dsimp only [typeAfter]; simp [castTo]⟩
  case negate =>
    obtain ⟨⟨op, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    simp only at h
    split at h
    · next hi =>
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨it, hit, hs⟩ := isSigned_ofTokenW (m := m) (τ := f.typeArgs) hi
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      rw [ofSlotW_of_ofTokenW hit, Option.some.injEq] at hu1
      subst hu1
      exact ⟨_, ofSigW_cons_eq hit hus1, by dsimp only [typeAfter]; simp [hs]⟩
    · simp [reject] at h
  case not =>
    obtain ⟨⟨op, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    simp only at h
    split at h
    · next hi =>
      simp only [beq_iff_eq] at hi
      subst hi
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, Option.some.injEq] at hu1
      subst hu1
      exact ⟨_, ofSigW_cons_eq rfl hus1, by dsimp only [typeAfter]⟩
    · simp [reject] at h
  case brTrue | brFalse | abort =>
    obtain ⟨⟨op, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    simp only at h
    split at h
    · simp [reject] at h
    · next hi =>
      simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hi
      subst hi
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, Option.some.injEq] at hu1
      subst hu1
      exact ⟨_, hus1, by dsimp only [typeAfter]⟩
  case abortMsg =>
    obtain ⟨⟨msg, rest⟩, hp, ⟨code, rest'⟩, hp', h⟩ := h
    rw [popType_ok] at hp hp'
    subst hp hp'
    split at h
    · simp [reject] at h
    · next hcd =>
      simp only [Bool.or_eq_true, bne_iff_ne, ne_eq, not_or, Decidable.not_not] at hcd
      obtain ⟨rfl, hmsg⟩ := hcd
      have : msg = .vector .u8 := by cases msg <;> simp_all [vectorElement]
      subst this
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      obtain ⟨u2, us2, hu2, hus2, rfl⟩ := ofSigW_cons.1 hus1
      simp [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith] at hu1 hu2
      subst hu1 hu2
      exact ⟨_, hus2, by dsimp only [typeAfter]⟩
  case pop =>
    obtain ⟨⟨op, rest⟩, hpop, ab, hab', h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    split at h
    · simp [reject] at h
    · next hd =>
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hd
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      have hdrop := AbilitySet.drop_of_subset (hab op hab' hu1) hd
      exact ⟨_, hus1, by dsimp only [typeAfter]; simp [hdrop]⟩
  case ldConst ci =>
    obtain ⟨c, hcl, h⟩ := h
    rw [lookup_ok] at hcl
    simp only [Except.ok.injEq] at h
    subst h
    obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1 (hc.constants c (getElem?_mem_toList hcl))
    exact ⟨_, ofSigW_cons_eq (ofTokenWith_closed hu) hst, by
      dsimp only [typeAfter]; simp [hcl, hu]⟩
  case copyLoc li =>
    obtain ⟨t, ht, ab, hab', h⟩ := h
    split at h
    · simp [reject] at h
    · next hcp =>
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hcp
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨u, hu, hl⟩ := hc.localType ht
      have hcopy := AbilitySet.copy_of_subset (hab t hab' hu) hcp
      refine ⟨_, ofSigW_cons_slot hu hst, ?_⟩
      dsimp only [typeAfter]
      simp only [bind] at hl ⊢
      rw [hl]
      simp [hcopy]
  case moveLoc li =>
    obtain ⟨t, ht, h⟩ := h
    simp only [Except.ok.injEq] at h
    subst h
    obtain ⟨u, hu, hl⟩ := hc.localType ht
    refine ⟨_, ofSigW_cons_slot hu hst, ?_⟩
    dsimp only [typeAfter]
    simp only [bind] at hl ⊢
    rw [hl]
    rfl
  case stLoc li =>
    obtain ⟨⟨op, rest⟩, hpop, t, ht, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    split at h
    · simp [reject] at h
    · next hi =>
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hi
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      obtain ⟨up, hup, hassign⟩ := assignable_slotW hu1 hi
      obtain ⟨u, hu, hl⟩ := hc.localType ht
      rw [hu, Option.some.injEq] at hup
      subst hup
      refine ⟨_, hus1, ?_⟩
      dsimp only [typeAfter]
      simp only [bind] at hl ⊢
      rw [hl]
      simp [hassign]
  case ret =>
    obtain ⟨args, hargs, -, hall⟩ := popExpected_ok h
    have hnil := hret rfl
    subst hnil
    rw [List.append_nil] at hargs
    subst hargs
    obtain ⟨rets, hrets, hrsig⟩ := hc.returns
    obtain ⟨rp, hrp, hassign⟩ := allPairs_coreW hall hst
    rw [ofSigW_reverse hrsig, Option.some.injEq] at hrp
    subst hrp
    exact ⟨[], rfl, by dsimp only [typeAfter]; simp [hrets, hassign]⟩
  case call hi =>
    obtain ⟨fh, hfh, params, hparams, st1, hpop, rets, hrets, h⟩ := h
    rw [lookup_ok] at hfh hparams hrets
    simp only [Except.ok.injEq] at h
    subst h
    have hnp : fh.typeParameters = [] := by
      simp only [Bytecode.OperandsConsistent, CompiledModule.functionTypeParamCount, hfh, bind,
        Option.bind_some, pure, Option.some.injEq] at hl9
      exact List.eq_nil_of_length_eq_zero hl9
    obtain ⟨args, hargs, -, hall⟩ := popExpected_ok hpop
    subst hargs
    obtain ⟨ps, rs, hps, hrs, hhp, hhr⟩ := hc.handleTypes0 hfh hnp hparams hrets
    obtain ⟨ra, r1, hra, hr1, rfl⟩ := ofSigW_append.1 hst
    obtain ⟨rp, hrp, hassign⟩ := allPairs_coreW hall hra
    rw [ofSigW_reverse hps, Option.some.injEq] at hrp
    subst hrp
    obtain ⟨htake, hdrop⟩ := assignable_take (rb := r1) hassign
    refine ⟨_, ofSigW_append_eq (ofSigW_reverse hrs) hr1, ?_⟩
    dsimp only [typeAfter]
    simp [hfh, hnp, hhp, hhr, htake, hdrop, hassign]
  case pack d =>
    obtain ⟨sd, hsd, h⟩ := h
    rw [lookup_ok] at hsd
    have hcount : m.structTypeParamCount d = some 0 := by
      simp only [Bytecode.OperandsConsistent] at hl9; exact hl9.1
    obtain ⟨fs, args, rest, hfs, rfl, -, hall, rfl⟩ := typePack_ok h
    rcases fieldsSignature_ok hfs with ⟨-, fields, hfi, rfl⟩ | ⟨_, _, _, hv, -⟩
    · obtain ⟨ra, rb, hra, hrb, rfl⟩ := ofSigW_append.1 hst
      obtain ⟨rp, hrp, hassign⟩ := allPairs_coreW hall hra
      obtain ⟨s, sh, id, hdef, hsh, hnp, hu, hty, hsta⟩ := hc.closedStruct hsd hcount
      obtain ⟨us, hus⟩ := hc.fieldsAt hsd hsh (fields := fields)
        (by simp [CompiledModule.StructFieldInformation.allFields, hfi]) (tys := []) (by simp [hnp])
      have hsig := ofSignatureWith_closed (tys := f.typeArgs) (ofSigW_fields (τ := []) hus)
      rw [ofSigW_reverse hsig, Option.some.injEq] at hrp
      subst hrp
      have hft : P.fieldTypes s = some us := by
        rw [fieldTypes_of_inst hty hfi, hsta]; exact hus
      refine ⟨_, ofSigW_cons_eq hu hrb, ?_⟩
      dsimp only [typeAfter]
      simp [hdef, hft, popPush_assignable hassign]
    · cases hv
  case unpack d =>
    obtain ⟨sd, hsd, h⟩ := h
    rw [lookup_ok] at hsd
    have hcount : m.structTypeParamCount d = some 0 := by
      simp only [Bytecode.OperandsConsistent] at hl9; exact hl9.1
    obtain ⟨fs, rest, rfl, hfs, rfl⟩ := typeUnpack_ok h
    rcases fieldsSignature_ok hfs with ⟨-, fields, hfi, rfl⟩ | ⟨_, _, _, hv, -⟩
    · obtain ⟨s, sh, id, hdef, hsh, hnp, hu, hty, hsta⟩ := hc.closedStruct hsd hcount
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      rw [ofSlotW_of_ofTokenW hu, Option.some.injEq] at hu1
      subst hu1
      obtain ⟨us, hus⟩ := hc.fieldsAt hsd hsh (fields := fields)
        (by simp [CompiledModule.StructFieldInformation.allFields, hfi]) (tys := []) (by simp [hnp])
      have hsig := ofSignatureWith_closed (tys := f.typeArgs) (ofSigW_fields (τ := []) hus)
      have hft : P.fieldTypes s = some us := by
        rw [fieldTypes_of_inst hty hfi, hsta]; exact hus
      refine ⟨_, ofSigW_append_eq (ofSigW_reverse hsig) hus1, ?_⟩
      dsimp only [typeAfter]
      simp [hdef, hft, popPushAll]
    · cases hv
  case packVariant vi =>
    obtain ⟨vh, hvh, sd, hsd, h⟩ := h
    rw [lookup_ok] at hvh hsd
    have hcount : m.structTypeParamCount vh.structIndex = some 0 := by
      simpa [Bytecode.OperandsConsistent, CompiledModule.variantTypeParamCount, hvh] using hl9
    obtain ⟨fs, args, rest, hfs, rfl, -, hall, rfl⟩ := typePack_ok h
    rcases fieldsSignature_ok hfs with ⟨hn, -⟩ | ⟨v, variants, vd, hv, hfi, hvd, rfl⟩
    · cases hn
    · simp only [Option.some.injEq] at hv
      subst hv
      obtain ⟨ra, rb, hra, hrb, rfl⟩ := ofSigW_append.1 hst
      obtain ⟨rp, hrp, hassign⟩ := allPairs_coreW hall hra
      obtain ⟨s, sh, id, hdef, hsh, hnp, hu, hty, hsta⟩ := hc.closedStruct hsd hcount
      obtain ⟨us, hus⟩ := hc.fieldsAt hsd hsh (fields := vd.fields) (by
          intro fd hfd
          simp only [CompiledModule.StructFieldInformation.allFields, hfi, List.mem_flatMap]
          exact ⟨vd, List.mem_of_getElem? hvd, hfd⟩) (tys := []) (by simp [hnp])
      have hsig := ofSignatureWith_closed (tys := f.typeArgs) (ofSigW_fields (τ := []) hus)
      rw [ofSigW_reverse hsig, Option.some.injEq] at hrp
      subst hrp
      have hft : P.variantFieldTypes s vh.variant = some us := by
        rw [variantFieldTypes_of_inst hty hfi hvd, hsta]; exact hus
      refine ⟨_, ofSigW_cons_eq hu hrb, ?_⟩
      dsimp only [typeAfter]
      simp [hvh, hdef, hft, popPush_assignable hassign]
  case unpackVariant vi =>
    obtain ⟨vh, hvh, sd, hsd, h⟩ := h
    rw [lookup_ok] at hvh hsd
    have hcount : m.structTypeParamCount vh.structIndex = some 0 := by
      simpa [Bytecode.OperandsConsistent, CompiledModule.variantTypeParamCount, hvh] using hl9
    obtain ⟨fs, rest, rfl, hfs, rfl⟩ := typeUnpack_ok h
    rcases fieldsSignature_ok hfs with ⟨hn, -⟩ | ⟨v, variants, vd, hv, hfi, hvd, rfl⟩
    · cases hn
    · simp only [Option.some.injEq] at hv
      subst hv
      obtain ⟨s, sh, id, hdef, hsh, hnp, hu, hty, hsta⟩ := hc.closedStruct hsd hcount
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      rw [ofSlotW_of_ofTokenW hu, Option.some.injEq] at hu1
      subst hu1
      obtain ⟨us, hus⟩ := hc.fieldsAt hsd hsh (fields := vd.fields) (by
          intro fd hfd
          simp only [CompiledModule.StructFieldInformation.allFields, hfi, List.mem_flatMap]
          exact ⟨vd, List.mem_of_getElem? hvd, hfd⟩) (tys := []) (by simp [hnp])
      have hsig := ofSignatureWith_closed (tys := f.typeArgs) (ofSigW_fields (τ := []) hus)
      have hft : P.variantFieldTypes s vh.variant = some us := by
        rw [variantFieldTypes_of_inst hty hfi hvd, hsta]; exact hus
      refine ⟨_, ofSigW_append_eq (ofSigW_reverse hsig) hus1, ?_⟩
      dsimp only [typeAfter]
      simp [hvh, hdef, hft, popPushAll]
  case vecPack sig n =>
    obtain ⟨elem, helem, st1, hpop, h⟩ := h
    simp only [Except.ok.injEq] at h
    subst h
    obtain ⟨t, hvt, het⟩ := elementType_supportedW (hops f.typeArgs hc.targsLen) helem
    obtain ⟨args, rfl, hlen, hall⟩ := popElements_ok _ hpop
    obtain ⟨ra, rb, hra, hrb, rfl⟩ := ofSigW_append.1 hst
    obtain ⟨rp, hrp, hassign⟩ := allPairs_coreW (allPairs_replicate hall) hra
    rw [ofSigW_replicate het, Option.some.injEq] at hrp
    subst hrp
    have hn : args.length = n := by simpa using hlen
    rw [hn, ← List.reverse_replicate] at hassign
    refine ⟨_, ofSigW_cons_eq (u := .vector t) (by simp [RuntimeType.ofTokenWith, het]) hrb, ?_⟩
    dsimp only [typeAfter]
    simp [hvt, popPush_assignable hassign]
  case vecUnpack sig n =>
    obtain ⟨⟨op, rest⟩, hp, elem, helem, h⟩ := h
    rw [popType_ok] at hp
    subst hp
    split at h
    · simp [reject] at h
    · next hcd =>
      simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hcd
      subst hcd
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨t, hvt, het⟩ := elementType_supportedW (hops f.typeArgs hc.targsLen) helem
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      simp [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, het] at hu1
      subst hu1
      refine ⟨_, ofSigW_append_eq (ofSigW_replicate het n) hus1, ?_⟩
      dsimp only [typeAfter]
      simp [hvt, popPushAll]
  case freezeRef =>
    obtain ⟨⟨op, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    dsimp only at h
    split at h
    · next inner =>
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      simp only [RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at hu1
      obtain ⟨v, hv, rfl⟩ := hu1
      exact ⟨_, ofSigW_cons_slot (ofSlotW_refTo (mut_ := false) hv) hus1,
        by dsimp only [typeAfter]⟩
    · simp [reject] at h
  case mutBorrowLoc li | immBorrowLoc li =>
    obtain ⟨t, ht, h⟩ := h
    split at h
    · simp [reject] at h
    · next hr =>
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨u, hu, hl⟩ := hc.localType ht
      have hnr : t.isRefToken = false := by
        cases t <;> simp_all [SignatureV2.isReference, SignatureToken.isRefToken]
      have hu' := ofTokenW_of_ofSlotW hu hnr
      first
        | refine ⟨_, ofSigW_cons_slot (ofSlotW_refTo (mut_ := true) hu') hst, ?_⟩
        | refine ⟨_, ofSigW_cons_slot (ofSlotW_refTo (mut_ := false) hu') hst, ?_⟩
      all_goals
        dsimp only [typeAfter]
        simp only [bind] at hl ⊢
        rw [hl]
        simp [ofTokenWith_isValue hc.targsValues hu']
  case mutBorrowVariantField hd | immBorrowVariantField hd =>
    obtain ⟨op, rest, fh, sd, rfl, hmut, hfh, hsd, htar, hne, ft, rfl, hall⟩ := variantBorrow_ok h
    have hcount : m.structTypeParamCount fh.structIndex = some 0 := by
      simpa [Bytecode.OperandsConsistent, CompiledModule.variantFieldTypeParamCount, hfh] using hl9
    obtain ⟨s, sh, id, hdef, hsh, hnp, hu, hty, hsta⟩ := hc.closedStruct hsd hcount
    obtain ⟨sd', hsd', -, hvb⟩ := verifyBounds_variantFieldHandle hb hfh
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    obtain ⟨v0, hv0⟩ : ∃ v0, v0 ∈ fh.variants := List.exists_mem_of_ne_nil _ hne
    cases hfi : sd.fieldInformation with
    | declaredVariants variants =>
      have hvar : ∀ v ∈ fh.variants, ∃ vd fd tys ft', variants[v]? = some vd ∧
          vd.fields[fh.field]? = some fd ∧ fd.signature = ft ∧
          vd.fields.mapM (fun fd => RuntimeType.ofTokenWith m [] fd.signature) = some tys ∧
          tys[fh.field]? = some ft' ∧ RuntimeType.ofTokenWith m [] ft = some ft' := by
        intro v hv
        obtain ⟨fd, hfd, hsig⟩ := hall v hv
        obtain ⟨hvlt, -⟩ := hvb v hv
        rw [hfi] at hvlt
        simp only [variantCount] at hvlt
        obtain ⟨vd, hvd⟩ : ∃ vd, variants[v]? = some vd := ⟨_, List.getElem?_eq_getElem hvlt⟩
        simp only [structFields, hfi, hvd] at hfd
        obtain ⟨tys, htys⟩ := hc.fieldsAt hsd hsh (fields := vd.fields) (by
          intro fd' hfd'
          simp only [CompiledModule.StructFieldInformation.allFields, hfi, List.mem_flatMap]
          exact ⟨vd, List.mem_of_getElem? hvd, hfd'⟩) (tys := []) (by simp [hnp])
        obtain ⟨ft', hft', hfdt⟩ := mapM_getElem? htys hfd
        exact ⟨vd, fd, tys, ft', hvd, hfd, hsig, htys, hft', hsig ▸ hfdt⟩
      obtain ⟨-, -, -, ft', -, -, -, -, -, hft⟩ := hvar v0 hv0
      have hvft : P.variantFieldType? s fh = some ft' := by
        have hmap : fh.variants.mapM (fun v => do (← P.variantFieldTypes s v)[fh.field]?) =
            some (List.replicate fh.variants.length ft') := by
          refine mapM_const fun v hv => ?_
          obtain ⟨vd, -, tys, ft'', hvd, -, -, htys, hft'', hft''t⟩ := hvar v hv
          rw [hft, Option.some.injEq] at hft''t
          subst hft''t
          rw [variantFieldTypes_of_inst hty hfi hvd, hsta]
          simp [htys, hft'']
        unfold Program.variantFieldType?
        simp only [bind, Option.bind_eq_some_iff]
        refine ⟨_, hmap, ?_⟩
        obtain ⟨n, hn⟩ : ∃ n, fh.variants.length = n + 1 :=
          ⟨fh.variants.length - 1, by have := List.length_pos_iff.2 hne; omega⟩
        rw [hn, List.replicate_succ]
        simp
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      obtain ⟨mut', v, rfl, hv, rfl⟩ := ofSlotW_refTarget htar hu1
      rw [hu, Option.some.injEq] at hv
      subst hv
      simp only [isMutableReference_refTo, Bool.and_eq_false_iff, Bool.not_eq_false'] at hmut
      refine ⟨_, ofSigW_cons_slot (ofSlotW_refTo (ofTokenWith_closed hft)) hus1, ?_⟩
      dsimp only [typeAfter]
      simp only [hfh, hdef, hvft, bind, Option.bind_some]
      cases mut' <;> simp_all
    | declared _ =>
      obtain ⟨hvlt, -⟩ := hvb v0 hv0
      rw [hfi] at hvlt
      simp [variantCount] at hvlt
    | native =>
      obtain ⟨hvlt, -⟩ := hvb v0 hv0
      rw [hfi] at hvlt
      simp [variantCount] at hvlt
  case testVariant hv =>
    obtain ⟨vh, hvh, sd, hsd, ⟨arg, rest⟩, hpop, h⟩ := h
    rw [lookup_ok] at hvh hsd
    rw [popType_ok] at hpop
    subst hpop
    have hcount : m.structTypeParamCount vh.structIndex = some 0 := by
      simpa [Bytecode.OperandsConsistent, CompiledModule.variantTypeParamCount, hvh] using hl9
    obtain ⟨s, sh, id, hdef, hsh, hnp, hu, -⟩ := hc.closedStruct hsd hcount
    obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
    dsimp only at h
    have key : ∃ mut', arg = refTo mut' (.struct sd.structHandle) ∧ st' = .bool :: rest := by
      split at h
      · next inner =>
        split at h
        · next heq =>
          simp only [beq_iff_eq] at heq
          subst heq
          simp only [Except.ok.injEq] at h
          exact ⟨false, rfl, h.symm⟩
        · simp [reject] at h
      · next inner =>
        split at h
        · next heq =>
          simp only [beq_iff_eq] at heq
          subst heq
          simp only [Except.ok.injEq] at h
          exact ⟨true, rfl, h.symm⟩
        · simp [reject] at h
      · simp [reject] at h
    obtain ⟨mut', rfl, rfl⟩ := key
    obtain ⟨mm, v, -, hv, rfl⟩ := ofSlotW_refTarget refTarget_refTo hu1
    rw [hu, Option.some.injEq] at hv
    subst hv
    refine ⟨_, ofSigW_cons_eq rfl hus1, ?_⟩
    dsimp only [typeAfter]
    simp [hvh, hdef]
  case readRef =>
    obtain ⟨⟨op, rest⟩, hpop, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    dsimp only at h
    obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
    have key : ∃ inner ab, refTarget op = some inner ∧
        abilities m view.handle.typeParameters inner = .ok ab ∧ ab.copy = true ∧
        st' = inner :: rest := by
      split at h
      · next inner =>
        simp only [bind_ok] at h
        obtain ⟨ab, hab', h⟩ := h
        split at h
        · simp [reject] at h
        · next hcp =>
          simp only [Bool.not_eq_true', Bool.not_eq_false] at hcp
          simp only [Except.ok.injEq] at h
          exact ⟨inner, ab, rfl, hab', hcp, h.symm⟩
      · next inner =>
        simp only [bind_ok] at h
        obtain ⟨ab, hab', h⟩ := h
        split at h
        · simp [reject] at h
        · next hcp =>
          simp only [Bool.not_eq_true', Bool.not_eq_false] at hcp
          simp only [Except.ok.injEq] at h
          exact ⟨inner, ab, rfl, hab', hcp, h.symm⟩
      · simp [reject] at h
    obtain ⟨inner, ab, htar, hab', hcp, rfl⟩ := key
    obtain ⟨mut', v, rfl, hv, rfl⟩ := ofSlotW_refTarget htar hu1
    have hcopy := AbilitySet.copy_of_subset (habv inner hab' hv) hcp
    refine ⟨_, ofSigW_cons_eq hv hus1, ?_⟩
    dsimp only [typeAfter]
    simp [hcopy]
  case writeRef =>
    obtain ⟨⟨refOp, rest⟩, hpop, ⟨val, rest'⟩, hpop', h⟩ := h
    rw [popType_ok] at hpop hpop'
    subst hpop
    simp only at hpop'
    subst hpop'
    dsimp only at h
    obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
    obtain ⟨u2, us2, hu2, hus2, rfl⟩ := ofSigW_cons.1 hus1
    have key : ∃ inner ab, refOp = .mutableReference inner ∧
        abilities m view.handle.typeParameters inner = .ok ab ∧ ab.drop = true ∧
        isAssignableFrom inner val = true ∧ st' = rest' := by
      split at h
      · next inner =>
        simp only [bind_ok] at h
        obtain ⟨ab, hab', h⟩ := h
        split at h
        · simp [reject] at h
        · next hd =>
          split at h
          · simp [reject] at h
          · next has =>
            simp only [Bool.not_eq_true', Bool.not_eq_false] at hd has
            simp only [Except.ok.injEq] at h
            exact ⟨inner, ab, rfl, hab', hd, has, h.symm⟩
      · simp [reject] at h
    obtain ⟨inner, ab, rfl, hab', hd, has, rfl⟩ := key
    simp only [RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at hu1
    obtain ⟨v, hv, rfl⟩ := hu1
    obtain ⟨up, hup, hassign⟩ := assignable_slotW hu2 has
    rw [ofSlotW_of_ofTokenW hv, Option.some.injEq] at hup
    subst hup
    have hdrop := AbilitySet.drop_of_subset (habv inner hab' hv) hd
    refine ⟨_, hus2, ?_⟩
    dsimp only [typeAfter]
    simp [hdrop, hassign]
  case mutBorrowGlobal d | immBorrowGlobal d =>
    obtain ⟨rest, sd, a, rfl, hsd, ha', hk, rfl⟩ := borrowGlobal_ok h
    have hcount : m.structTypeParamCount d = some 0 := by
      simpa [Bytecode.OperandsConsistent] using hl9
    obtain ⟨s, sh, id, hdef, hsh, hnp, hu, -⟩ := hc.closedStruct hsd hcount
    have hkey := AbilitySet.key_of_subset (habv _ ha' hu) hk
    obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
    simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, Option.some.injEq] at hu1
    subst hu1
    refine ⟨_, ofSigW_cons_slot (ofSlotW_refTo hu) hus1, ?_⟩
    dsimp only [typeAfter]
    simp [hdef, hkey]
  case exists_ d | moveFrom d =>
    obtain ⟨sd, hsd, a, ha', u, hk, ⟨op, rest⟩, hpop, u', hop, h⟩ := h
    rw [lookup_ok] at hsd
    rw [popType_ok] at hpop
    subst hpop
    rw [rejectIf_ok] at hk hop
    simp only [bne_eq_false_iff_eq, Bool.not_eq_false'] at hk hop
    subst hop
    simp only [Except.ok.injEq] at h
    subst h
    have hcount : m.structTypeParamCount d = some 0 := by
      simpa [Bytecode.OperandsConsistent] using hl9
    obtain ⟨s, sh, id, hdef, hsh, hnp, hu, -⟩ := hc.closedStruct hsd hcount
    have hkey := AbilitySet.key_of_subset (habv _ ha' hu) hk
    obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
    simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, Option.some.injEq] at hu1
    subst hu1
    first
      | exact ⟨_, ofSigW_cons_eq rfl hus1, by dsimp only [typeAfter]; simp [hdef, hkey]⟩
      | exact ⟨_, ofSigW_cons_eq hu hus1, by dsimp only [typeAfter]; simp [hdef, hkey]⟩
  case moveTo d =>
    obtain ⟨sd, hsd, a, ha', u, hk, ⟨keyOp, rest⟩, hpop, ⟨signerOp, rest'⟩, hpop', u', hko, u'',
      hso, h⟩ := h
    rw [lookup_ok] at hsd
    rw [popType_ok] at hpop hpop'
    subst hpop
    simp only at hpop'
    subst hpop'
    rw [rejectIf_ok] at hk hko hso
    simp only [bne_eq_false_iff_eq, Bool.not_eq_false'] at hk hko hso
    subst hko hso
    simp only [Except.ok.injEq] at h
    subst h
    have hcount : m.structTypeParamCount d = some 0 := by
      simpa [Bytecode.OperandsConsistent] using hl9
    obtain ⟨s, sh, id, hdef, hsh, hnp, hu, -⟩ := hc.closedStruct hsd hcount
    have hkey := AbilitySet.key_of_subset (habv _ ha' hu) hk
    obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
    obtain ⟨u2, us2, hu2, hus2, rfl⟩ := ofSigW_cons.1 hus1
    rw [ofSlotW_of_ofTokenW hu, Option.some.injEq] at hu1
    subst hu1
    simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, Option.map_some,
      Option.some.injEq] at hu2
    subst hu2
    refine ⟨_, hus2, ?_⟩
    dsimp only [typeAfter]
    simp [hdef, hkey]
  case vecLen idx =>
    obtain ⟨⟨op, rest⟩, hpop, elem, helem, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    obtain ⟨t, hvt, het⟩ := elementType_supportedW (hops f.typeArgs hc.targsLen) helem
    split at h
    · next hv =>
      simp only [beq_iff_eq] at hv
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨mut', rfl, -⟩ := vectorRefElement_some hv
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      rw [ofSlotW_vectorRef het hu1]
      refine ⟨_, ofSigW_cons_eq rfl hus1, ?_⟩
      dsimp only [typeAfter]
      simp [hvt]
    · simp [reject] at h
  case vecImmBorrow idx | vecMutBorrow idx =>
    obtain ⟨elem, helem, h⟩ := h
    obtain ⟨t, hvt, het⟩ := elementType_supportedW (hops f.typeArgs hc.targsLen) helem
    simp only [borrowVectorElement, bind_ok] at h
    obtain ⟨⟨i, rest⟩, hpop, ⟨vec, rest'⟩, hpop', u, hi, h⟩ := h
    rw [popType_ok] at hpop hpop'
    subst hpop
    simp only at hpop'
    subst hpop'
    rw [rejectIf_ok] at hi
    simp only [bne_eq_false_iff_eq] at hi
    subst hi
    dsimp only at h
    split at h
    · next e he =>
      split at h
      · next heq =>
        simp only [beq_iff_eq] at heq
        subst heq
        simp only [pure, Except.pure, Except.ok.injEq] at h
        subst h
        obtain ⟨mut', rfl, hmut⟩ := vectorRefElement_some he
        obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
        simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, Option.some.injEq] at hu1
        subst hu1
        obtain ⟨u2, us2, hu2, hus2, rfl⟩ := ofSigW_cons.1 hus1
        rw [ofSlotW_vectorRef het hu2]
        refine ⟨_, ofSigW_cons_slot (ofSlotW_refTo het) hus2, ?_⟩
        dsimp only [typeAfter]
        simp only [hvt, bind, Option.bind_some]
        cases mut' <;> simp_all
      · simp [reject] at h
    · simp [reject] at h
  case vecPushBack idx =>
    obtain ⟨⟨elemOp, rest⟩, hpop, ⟨vecOp, rest'⟩, hpop', elem, helem, u, has, u', hv, h⟩ := h
    rw [popType_ok] at hpop hpop'
    subst hpop
    simp only at hpop'
    subst hpop'
    obtain ⟨t, hvt, het⟩ := elementType_supportedW (hops f.typeArgs hc.targsLen) helem
    rw [rejectIf_ok] at has hv
    simp only [Bool.not_eq_false', bne_eq_false_iff_eq] at has hv
    simp only [Except.ok.injEq] at h
    subst h
    obtain ⟨mut', rfl, hmut⟩ := vectorRefElement_some hv
    have hm := hmut rfl
    subst hm
    obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
    obtain ⟨up, hup, hassign⟩ := assignable_slotW hu1 has
    rw [ofSlotW_of_ofTokenW het, Option.some.injEq] at hup
    subst hup
    obtain ⟨u2, us2, hu2, hus2, rfl⟩ := ofSigW_cons.1 hus1
    rw [ofSlotW_vectorRef het hu2]
    refine ⟨_, hus2, ?_⟩
    dsimp only [typeAfter]
    simp [hvt, hassign]
  case vecPopBack idx =>
    obtain ⟨⟨vecOp, rest⟩, hpop, elem, helem, h⟩ := h
    rw [popType_ok] at hpop
    subst hpop
    obtain ⟨t, hvt, het⟩ := elementType_supportedW (hops f.typeArgs hc.targsLen) helem
    split at h
    · next hv =>
      simp only [beq_iff_eq] at hv
      simp only [Except.ok.injEq] at h
      subst h
      obtain ⟨mut', rfl, hmut⟩ := vectorRefElement_some hv
      have hm := hmut rfl
      subst hm
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      rw [ofSlotW_vectorRef het hu1]
      refine ⟨_, ofSigW_cons_eq het hus1, ?_⟩
      dsimp only [typeAfter]
      simp [hvt]
    · simp [reject] at h
  case vecSwap idx =>
    obtain ⟨⟨i2, rest⟩, hpop, ⟨i1, rest'⟩, hpop', ⟨vecOp, rest''⟩, hpop'', u, hi, elem, helem,
      u', hv, h⟩ := h
    rw [popType_ok] at hpop hpop' hpop''
    subst hpop
    simp only at hpop' hpop''
    subst hpop' hpop''
    obtain ⟨t, hvt, het⟩ := elementType_supportedW (hops f.typeArgs hc.targsLen) helem
    rw [rejectIf_ok] at hi hv
    simp only [Bool.or_eq_false_iff, bne_eq_false_iff_eq] at hi hv
    obtain ⟨rfl, rfl⟩ := hi
    simp only [Except.ok.injEq] at h
    subst h
    obtain ⟨mut', rfl, hmut⟩ := vectorRefElement_some hv
    have hm := hmut rfl
    subst hm
    obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
    obtain ⟨u2, us2, hu2, hus2, rfl⟩ := ofSigW_cons.1 hus1
    obtain ⟨u3, us3, hu3, hus3, rfl⟩ := ofSigW_cons.1 hus2
    simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, Option.some.injEq] at hu1 hu2
    subst hu1 hu2
    rw [ofSlotW_vectorRef het hu3]
    refine ⟨_, hus3, ?_⟩
    dsimp only [typeAfter]
    simp [hvt]
  case callGeneric ii =>
    obtain ⟨fi, hfi, fh, hfh, tys, htys, h⟩ := h
    rw [lookup_ok] at hfi hfh htys
    obtain ⟨params, rets, rets', args, rest, hparams, hrets, hrets', rfl, hlen, hall, rfl⟩ :=
      typeCallGeneric_ok h
    simp only [Bytecode.OperandsConsistent] at hl9
    obtain ⟨x, hx, n, args0, hcount, hpos, hargs0, hlen0⟩ := hl9
    rw [hfi, Option.some.injEq] at hx
    subst hx
    rw [htys, Option.some.injEq] at hargs0
    subst hargs0
    have hcount' : tys.length = fh.typeParameters.length := by
      simp only [CompiledModule.functionTypeParamCount, hfh, bind, Option.bind_some, pure,
        Option.some.injEq] at hcount
      omega
    have hne : tys ≠ [] := by intro h0; subst h0; simp at hlen0; omega
    simp only [CompiledModule.SupportedOperands] at hops
    obtain ⟨x, hx, sig', hsig', hallsig⟩ := hops
    rw [hfi, Option.some.injEq] at hx
    subst hx
    rw [htys, Option.some.injEq] at hsig'
    subst hsig'
    obtain ⟨σ', hσ⟩ := Option.isSome_iff_exists.1 (hallsig f.typeArgs hc.targsLen)
    have hσlen : σ'.length = fh.typeParameters.length := by
      rw [ofTokenListWith_length hσ, hcount']
    simp only [Bytecode.TypeOperandsWellFormed] at hl5
    obtain ⟨x, fh', hx, hfh', targs0, htargs0, hwf⟩ := hl5
    rw [hfi, Option.some.injEq] at hx
    subst hx
    rw [hfh, Option.some.injEq] at hfh'
    subst hfh'
    rw [htys, Option.some.injEq] at htargs0
    subst htargs0
    have hok := translateList_ok hc.resolves hc.targs hwf hσ
    obtain ⟨ra, r1, hra, hr1, rfl⟩ := ofSigW_append.1 hst
    obtain ⟨rp, hrp, hassign⟩ := allPairs_instW hne hσ hall hra
    obtain ⟨ps, rs, hps, hrs, hhp, hhr⟩ := hc.handleTypesAt hfh hparams hrets hσlen
    rw [ofSigW_reverse hps, Option.some.injEq] at hrp
    subst hrp
    obtain ⟨htake, hdrop⟩ := assignable_take (rb := r1) hassign
    have hrets'' := mapM_instW hne hσ hrets'
    rw [hrs] at hrets''
    refine ⟨_, ofSigW_append_eq (ofSigW_reverse hrets'') hr1, ?_⟩
    obtain ⟨t0, ts0, hcons⟩ := cons_of_length_pos (l := σ') (by
      intro h0; subst h0; simp at hσlen; omega)
    subst hcons
    dsimp only [typeAfter]
    simp [hfi, hfh, typeArgsWith_eq htys, hσ, hok, hhp, hhr, htake, hdrop, hassign]
  case packGeneric ii | unpackGeneric ii =>
    obtain ⟨⟨d, sd, tys⟩, hparts, h⟩ := h
    obtain ⟨si, hsi, rfl, hsd, htys⟩ := structInstParts_ok.1 hparts
    dsimp only at h
    simp only [Bytecode.OperandsConsistent] at hl9
    obtain ⟨x, hx, hgen, hdecl⟩ := hl9
    rw [hsi, Option.some.injEq] at hx
    subst hx
    simp only [CompiledModule.SupportedOperands] at hops
    obtain ⟨x, hx, hops'⟩ := hops
    rw [hsi, Option.some.injEq] at hx
    subst hx
    simp only [Bytecode.TypeOperandsWellFormed] at hl5
    obtain ⟨x, cs, hx, hcs, hwf⟩ := hl5
    rw [hsi, Option.some.injEq] at hx
    subst hx
    obtain ⟨hne, id, name, sh, σ', hsh, hsid, hσ, hσlen, hdef, hst', hwfst, htargs, hmat⟩ :=
      hc.instOf hsd htys hgen hops' ⟨cs, hcs, hwf⟩
    obtain ⟨t0, ts0, hcons⟩ := cons_of_length_pos (l := σ') (by
      intro h0; subst h0
      simp only [List.length_nil] at hσlen
      have := ofTokenListWith_length hσ
      simp only [List.length_nil] at this
      exact hne (List.eq_nil_of_length_eq_zero this.symm))
    first
    | obtain ⟨fs0, fs, args, rest, h0, h1, rfl, -, hall, rfl⟩ := typePackGeneric_ok h
      rcases fieldsSignature_ok h0 with ⟨-, fields, hfi, rfl⟩ | ⟨_, _, _, hv, -⟩
      · obtain ⟨ra, rb, hra, hrb, rfl⟩ := ofSigW_append.1 hst
        obtain ⟨rp, hrp, hassign⟩ := allPairs_coreW hall hra
        obtain ⟨us, hus⟩ := hc.fieldsAt hsd hsh (fields := fields)
          (by simp [CompiledModule.StructFieldInformation.allFields, hfi]) (tys := σ') hσlen
        have hsig : RuntimeType.ofSignatureWith m f.typeArgs fs = some us := by
          rw [mapM_instW hne hσ h1]; exact ofSigW_fields hus
        rw [ofSigW_reverse hsig, Option.some.injEq] at hrp
        subst hrp
        have hft : P.fieldTypes ⟨id, name, σ'⟩ = some us := by
          rw [fieldTypes_of_inst hst' hfi]; exact hus
        refine ⟨_, ofSigW_cons_eq hmat hrb, ?_⟩
        subst hcons
        dsimp only [typeAfter]
        simp [hsi, hdef, htargs, hwfst, hft, popPush_assignable hassign]
      · cases hv
    | obtain ⟨fs0, fs, rest, rfl, h0, h1, rfl⟩ := typeUnpackGeneric_ok h
      rcases fieldsSignature_ok h0 with ⟨-, fields, hfi, rfl⟩ | ⟨_, _, _, hv, -⟩
      · obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
        rw [ofSlotW_of_ofTokenW hmat, Option.some.injEq] at hu1
        subst hu1
        obtain ⟨us, hus⟩ := hc.fieldsAt hsd hsh (fields := fields)
          (by simp [CompiledModule.StructFieldInformation.allFields, hfi]) (tys := σ') hσlen
        have hsig : RuntimeType.ofSignatureWith m f.typeArgs fs = some us := by
          rw [mapM_instW hne hσ h1]; exact ofSigW_fields hus
        have hft : P.fieldTypes ⟨id, name, σ'⟩ = some us := by
          rw [fieldTypes_of_inst hst' hfi]; exact hus
        refine ⟨_, ofSigW_append_eq (ofSigW_reverse hsig) hus1, ?_⟩
        subst hcons
        dsimp only [typeAfter]
        simp [hsi, hdef, htargs, hwfst, hft, popPushAll]
      · cases hv
  case packVariantGeneric ii | unpackVariantGeneric ii =>
    obtain ⟨⟨vh, sd, tys⟩, hparts, h⟩ := h
    obtain ⟨vi, hvi, hvh, hsd, htys⟩ := variantInstParts_ok.1 hparts
    dsimp only at h
    simp only [Bytecode.OperandsConsistent] at hl9
    obtain ⟨x, hx, hgen⟩ := hl9
    rw [hvi, Option.some.injEq] at hx
    subst hx
    simp only [CompiledModule.variantTypeParamCount, hvh, bind, Option.bind_some] at hgen
    simp only [CompiledModule.SupportedOperands] at hops
    obtain ⟨x, hx, hops'⟩ := hops
    rw [hvi, Option.some.injEq] at hx
    subst hx
    simp only [Bytecode.TypeOperandsWellFormed] at hl5
    obtain ⟨x, vh', cs, hx, hvh', hcs, hwf⟩ := hl5
    rw [hvi, Option.some.injEq] at hx
    subst hx
    rw [hvh, Option.some.injEq] at hvh'
    subst hvh'
    obtain ⟨hne, id, name, sh, σ', hsh, hsid, hσ, hσlen, hdef, hst', hwfst, htargs, hmat⟩ :=
      hc.instOf hsd htys hgen hops' ⟨cs, hcs, hwf⟩
    obtain ⟨t0, ts0, hcons⟩ := cons_of_length_pos (l := σ') (by
      intro h0; subst h0
      have := ofTokenListWith_length hσ
      simp only [List.length_nil] at this
      exact hne (List.eq_nil_of_length_eq_zero this.symm))
    first
    | obtain ⟨fs0, fs, args, rest, h0, h1, rfl, -, hall, rfl⟩ := typePackGeneric_ok h
      rcases fieldsSignature_ok h0 with ⟨hn, -⟩ | ⟨v, variants, vd, hv, hfi, hvd, rfl⟩
      · cases hn
      · simp only [Option.some.injEq] at hv
        subst hv
        obtain ⟨ra, rb, hra, hrb, rfl⟩ := ofSigW_append.1 hst
        obtain ⟨rp, hrp, hassign⟩ := allPairs_coreW hall hra
        obtain ⟨us, hus⟩ := hc.fieldsAt hsd hsh (fields := vd.fields) (by
            intro fd hfd
            simp only [CompiledModule.StructFieldInformation.allFields, hfi, List.mem_flatMap]
            exact ⟨vd, List.mem_of_getElem? hvd, hfd⟩) (tys := σ') hσlen
        have hsig : RuntimeType.ofSignatureWith m f.typeArgs fs = some us := by
          rw [mapM_instW hne hσ h1]; exact ofSigW_fields hus
        rw [ofSigW_reverse hsig, Option.some.injEq] at hrp
        subst hrp
        have hft : P.variantFieldTypes ⟨id, name, σ'⟩ vh.variant = some us := by
          rw [variantFieldTypes_of_inst hst' hfi hvd]; exact hus
        refine ⟨_, ofSigW_cons_eq hmat hrb, ?_⟩
        subst hcons
        dsimp only [typeAfter]
        simp [hvi, hvh, hdef, htargs, hwfst, hft, popPush_assignable hassign]
    | obtain ⟨fs0, fs, rest, rfl, h0, h1, rfl⟩ := typeUnpackGeneric_ok h
      rcases fieldsSignature_ok h0 with ⟨hn, -⟩ | ⟨v, variants, vd, hv, hfi, hvd, rfl⟩
      · cases hn
      · simp only [Option.some.injEq] at hv
        subst hv
        obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
        rw [ofSlotW_of_ofTokenW hmat, Option.some.injEq] at hu1
        subst hu1
        obtain ⟨us, hus⟩ := hc.fieldsAt hsd hsh (fields := vd.fields) (by
            intro fd hfd
            simp only [CompiledModule.StructFieldInformation.allFields, hfi, List.mem_flatMap]
            exact ⟨vd, List.mem_of_getElem? hvd, hfd⟩) (tys := σ') hσlen
        have hsig : RuntimeType.ofSignatureWith m f.typeArgs fs = some us := by
          rw [mapM_instW hne hσ h1]; exact ofSigW_fields hus
        have hft : P.variantFieldTypes ⟨id, name, σ'⟩ vh.variant = some us := by
          rw [variantFieldTypes_of_inst hst' hfi hvd]; exact hus
        refine ⟨_, ofSigW_append_eq (ofSigW_reverse hsig) hus1, ?_⟩
        subst hcons
        dsimp only [typeAfter]
        simp [hvi, hvh, hdef, htargs, hwfst, hft, popPushAll]
  case testVariantGeneric ii =>
    obtain ⟨⟨vh, sd, tys⟩, hparts, ⟨arg, rest⟩, hpop, h⟩ := h
    obtain ⟨vi, hvi, hvh, hsd, htys⟩ := variantInstParts_ok.1 hparts
    rw [popType_ok] at hpop
    subst hpop
    dsimp only at h
    simp only [Bytecode.OperandsConsistent] at hl9
    obtain ⟨x, hx, hgen⟩ := hl9
    rw [hvi, Option.some.injEq] at hx
    subst hx
    simp only [CompiledModule.variantTypeParamCount, hvh, bind, Option.bind_some] at hgen
    simp only [CompiledModule.SupportedOperands] at hops
    obtain ⟨x, hx, hops'⟩ := hops
    rw [hvi, Option.some.injEq] at hx
    subst hx
    simp only [Bytecode.TypeOperandsWellFormed] at hl5
    obtain ⟨x, vh', cs, hx, hvh', hcs, hwf⟩ := hl5
    rw [hvi, Option.some.injEq] at hx
    subst hx
    rw [hvh, Option.some.injEq] at hvh'
    subst hvh'
    obtain ⟨hne, id, name, sh, σ', hsh, hsid, hσ, hσlen, hdef, hst', hwfst, htargs, hmat⟩ :=
      hc.instOf hsd htys hgen hops' ⟨cs, hcs, hwf⟩
    obtain ⟨t0, ts0, hcons⟩ := cons_of_length_pos (l := σ') (by
      intro h0; subst h0
      have := ofTokenListWith_length hσ
      simp only [List.length_nil] at this
      exact hne (List.eq_nil_of_length_eq_zero this.symm))
    obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
    have key : ∃ mut', arg = refTo mut' (materializeType sd.structHandle tys) ∧
        st' = .bool :: rest := by
      split at h
      · next inner =>
        split at h
        · next heq =>
          simp only [beq_iff_eq] at heq
          subst heq
          simp only [Except.ok.injEq] at h
          exact ⟨false, rfl, h.symm⟩
        · simp [reject] at h
      · next inner =>
        split at h
        · next heq =>
          simp only [beq_iff_eq] at heq
          subst heq
          simp only [Except.ok.injEq] at h
          exact ⟨true, rfl, h.symm⟩
        · simp [reject] at h
      · simp [reject] at h
    obtain ⟨mut', rfl, rfl⟩ := key
    obtain ⟨mm, v, -, hv, rfl⟩ := ofSlotW_refTarget refTarget_refTo hu1
    rw [hmat, Option.some.injEq] at hv
    subst hv
    refine ⟨_, ofSigW_cons_eq rfl hus1, ?_⟩
    subst hcons
    dsimp only [typeAfter]
    simp [hvi, hvh, hdef, htargs, hwfst]
  case mutBorrowFieldGeneric ii | immBorrowFieldGeneric ii =>
    obtain ⟨fi, hfi, tys, htys, h⟩ := h
    rw [lookup_ok] at hfi htys
    obtain ⟨op, rest, fh, sd, rfl, hmut, hfh, hsd, htar, hres⟩ := fieldBorrowGen_ok h
    simp only [Bytecode.OperandsConsistent] at hl9
    obtain ⟨x, hx, hgen⟩ := hl9
    rw [hfi, Option.some.injEq] at hx
    subst hx
    simp only [CompiledModule.fieldTypeParamCount, hfh, bind, Option.bind_some] at hgen
    simp only [CompiledModule.SupportedOperands] at hops
    obtain ⟨x, hx, hops'⟩ := hops
    rw [hfi, Option.some.injEq] at hx
    subst hx
    simp only [Bytecode.TypeOperandsWellFormed] at hl5
    obtain ⟨x, fh', cs, hx, hfh', hcs, hwf⟩ := hl5
    rw [hfi, Option.some.injEq] at hx
    subst hx
    rw [hfh, Option.some.injEq] at hfh'
    subst hfh'
    obtain ⟨hne, id, name, sh, σ', hsh, hsid, hσ, hσlen, hdef, hst', hwfst, htargs, hmat⟩ :=
      hc.instOf hsd htys hgen hops' ⟨cs, hcs, hwf⟩
    obtain ⟨t0, ts0, hcons⟩ := cons_of_length_pos (l := σ') (by
      intro h0; subst h0
      have := ofTokenListWith_length hσ
      simp only [List.length_nil] at this
      exact hne (List.eq_nil_of_length_eq_zero this.symm))
    obtain ⟨sd', hsd', hlt⟩ := verifyBounds_fieldHandle hb hfh
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    cases hfi' : sd.fieldInformation with
    | declared fields =>
      rw [hfi'] at hlt
      simp only [fieldCount] at hlt
      obtain ⟨fd, hfd⟩ : ∃ fd, fields[fh.field]? = some fd := ⟨_, List.getElem?_eq_getElem hlt⟩
      rcases hres with ⟨fd', q, hfd', hq, rfl⟩ | ⟨hnone, -⟩
      · simp only [structFields, hfi', hfd, Option.some.injEq] at hfd'
        subst hfd'
        obtain ⟨us, hus⟩ := hc.fieldsAt hsd hsh (fields := fields)
          (by simp [CompiledModule.StructFieldInformation.allFields, hfi']) (tys := σ') hσlen
        obtain ⟨ft, hft, hfdt⟩ := mapM_getElem? hus hfd
        have hqt : RuntimeType.ofTokenWith m f.typeArgs q = some ft := by
          rw [ofTokenWith_instantiate hσ (instantiate_spec hne hq)]; exact hfdt
        have hfts : P.fieldTypes ⟨id, name, σ'⟩ = some us := by
          rw [fieldTypes_of_inst hst' hfi']; exact hus
        obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
        obtain ⟨mut', v, rfl, hv, rfl⟩ := ofSlotW_refTarget htar hu1
        rw [hmat, Option.some.injEq] at hv
        subst hv
        simp only [isMutableReference_refTo, Bool.and_eq_false_iff, Bool.not_eq_false'] at hmut
        refine ⟨_, ofSigW_cons_slot (ofSlotW_refTo hqt) hus1, ?_⟩
        subst hcons
        dsimp only [typeAfter]
        simp only [hfi, hfh, hdef, htargs, hwfst, bind, Option.bind_some]
        cases mut' <;> simp_all
      · simp [structFields, hfi', hfd] at hnone
    | declaredVariants _ => rw [hfi'] at hlt; simp [fieldCount] at hlt
    | native => rw [hfi'] at hlt; simp [fieldCount] at hlt
  case mutBorrowVariantFieldGeneric ii | immBorrowVariantFieldGeneric ii =>
    obtain ⟨fi, hfi, tys, htys, h⟩ := h
    rw [lookup_ok] at hfi htys
    obtain ⟨op, rest, fh, sd, rfl, hmut, hfh, hsd, htar, hne', ft, rfl, hall⟩ :=
      variantBorrowGen_full h
    simp only [Bytecode.OperandsConsistent] at hl9
    obtain ⟨x, hx, hgen⟩ := hl9
    rw [hfi, Option.some.injEq] at hx
    subst hx
    simp only [CompiledModule.variantFieldTypeParamCount, hfh, bind, Option.bind_some] at hgen
    simp only [CompiledModule.SupportedOperands] at hops
    obtain ⟨x, hx, hops'⟩ := hops
    rw [hfi, Option.some.injEq] at hx
    subst hx
    simp only [Bytecode.TypeOperandsWellFormed] at hl5
    obtain ⟨x, fh', cs, hx, hfh', hcs, hwf⟩ := hl5
    rw [hfi, Option.some.injEq] at hx
    subst hx
    rw [hfh, Option.some.injEq] at hfh'
    subst hfh'
    obtain ⟨hne, id, name, sh, σ', hsh, hsid, hσ, hσlen, hdef, hst', hwfst, htargs, hmat⟩ :=
      hc.instOf hsd htys hgen hops' ⟨cs, hcs, hwf⟩
    obtain ⟨t0, ts0, hcons⟩ := cons_of_length_pos (l := σ') (by
      intro h0; subst h0
      have := ofTokenListWith_length hσ
      simp only [List.length_nil] at this
      exact hne (List.eq_nil_of_length_eq_zero this.symm))
    obtain ⟨sd', hsd', -, hvb⟩ := verifyBounds_variantFieldHandle hb hfh
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    obtain ⟨v0, hv0⟩ : ∃ v0, v0 ∈ fh.variants := List.exists_mem_of_ne_nil _ hne'
    cases hfi' : sd.fieldInformation with
    | declaredVariants variants =>
      have hvar : ∀ v ∈ fh.variants, ∃ vd fd tys' ft', variants[v]? = some vd ∧
          vd.fields[fh.field]? = some fd ∧ instantiate tys fd.signature = .ok ft ∧
          vd.fields.mapM (fun fd => RuntimeType.ofTokenWith m σ' fd.signature) = some tys' ∧
          tys'[fh.field]? = some ft' ∧ RuntimeType.ofTokenWith m f.typeArgs ft = some ft' := by
        intro v hv
        obtain ⟨fd, hfd, hsig⟩ := hall v hv
        obtain ⟨hvlt, -⟩ := hvb v hv
        rw [hfi'] at hvlt
        simp only [variantCount] at hvlt
        obtain ⟨vd, hvd⟩ : ∃ vd, variants[v]? = some vd := ⟨_, List.getElem?_eq_getElem hvlt⟩
        simp only [structFields, hfi', hvd] at hfd
        obtain ⟨tys', htys'⟩ := hc.fieldsAt hsd hsh (fields := vd.fields) (by
          intro fd' hfd'
          simp only [CompiledModule.StructFieldInformation.allFields, hfi', List.mem_flatMap]
          exact ⟨vd, List.mem_of_getElem? hvd, hfd'⟩) (tys := σ') hσlen
        obtain ⟨ft', hft', hfdt⟩ := mapM_getElem? htys' hfd
        refine ⟨vd, fd, tys', ft', hvd, hfd, hsig, htys', hft', ?_⟩
        rw [ofTokenWith_instantiate hσ (instantiate_spec hne hsig)]
        exact hfdt
      obtain ⟨-, -, -, ft', -, -, -, -, -, hft⟩ := hvar v0 hv0
      have hvft : P.variantFieldType? ⟨id, name, σ'⟩ fh = some ft' := by
        have hmap : fh.variants.mapM
            (fun v => do (← P.variantFieldTypes ⟨id, name, σ'⟩ v)[fh.field]?) =
            some (List.replicate fh.variants.length ft') := by
          refine mapM_const fun v hv => ?_
          obtain ⟨vd, -, tys', ft'', hvd, -, -, htys', hft'', hft''t⟩ := hvar v hv
          rw [hft, Option.some.injEq] at hft''t
          subst hft''t
          rw [variantFieldTypes_of_inst hst' hfi' hvd]
          simp [htys', hft'']
        unfold Program.variantFieldType?
        simp only [bind, Option.bind_eq_some_iff]
        refine ⟨_, hmap, ?_⟩
        obtain ⟨n, hn⟩ : ∃ n, fh.variants.length = n + 1 :=
          ⟨fh.variants.length - 1, by have := List.length_pos_iff.2 hne'; omega⟩
        rw [hn, List.replicate_succ]
        simp
      obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
      obtain ⟨mut', v, rfl, hv, rfl⟩ := ofSlotW_refTarget htar hu1
      rw [hmat, Option.some.injEq] at hv
      subst hv
      simp only [isMutableReference_refTo, Bool.and_eq_false_iff, Bool.not_eq_false'] at hmut
      refine ⟨_, ofSigW_cons_slot (ofSlotW_refTo hft) hus1, ?_⟩
      subst hcons
      dsimp only [typeAfter]
      simp only [hfi, hfh, hdef, htargs, hwfst, bind, Option.bind_some]
      cases mut' <;> simp_all
    | declared _ =>
      obtain ⟨hvlt, -⟩ := hvb v0 hv0
      rw [hfi'] at hvlt
      simp [variantCount] at hvlt
    | native =>
      obtain ⟨hvlt, -⟩ := hvb v0 hv0
      rw [hfi'] at hvlt
      simp [variantCount] at hvlt
  case mutBorrowGlobalGeneric ii | immBorrowGlobalGeneric ii =>
    obtain ⟨⟨d, sd, tys⟩, hparts, h⟩ := h
    obtain ⟨si, hsi, rfl, hsd, htys⟩ := structInstParts_ok.1 hparts
    dsimp only at h
    obtain ⟨rest, sd', a, rfl, hsd', ha', hk, rfl⟩ := borrowGlobalGen_ok h
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    simp only [Bytecode.OperandsConsistent] at hl9
    obtain ⟨x, hx, hgen⟩ := hl9
    rw [hsi, Option.some.injEq] at hx
    subst hx
    simp only [CompiledModule.SupportedOperands] at hops
    obtain ⟨x, hx, hops'⟩ := hops
    rw [hsi, Option.some.injEq] at hx
    subst hx
    simp only [Bytecode.TypeOperandsWellFormed] at hl5
    obtain ⟨x, cs, hx, hcs, hwf⟩ := hl5
    rw [hsi, Option.some.injEq] at hx
    subst hx
    obtain ⟨hne, id, name, sh, σ', hsh, hsid, hσ, hσlen, hdef, hst', hwfst, htargs, hmat⟩ :=
      hc.instOf hsd htys hgen hops' ⟨cs, hcs, hwf⟩
    obtain ⟨t0, ts0, hcons⟩ := cons_of_length_pos (l := σ') (by
      intro h0; subst h0
      have := ofTokenListWith_length hσ
      simp only [List.length_nil] at this
      exact hne (List.eq_nil_of_length_eq_zero this.symm))
    have hkey := AbilitySet.key_of_subset (habv _ ha' hmat) hk
    obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
    simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, Option.some.injEq] at hu1
    subst hu1
    refine ⟨_, ofSigW_cons_slot (ofSlotW_refTo hmat) hus1, ?_⟩
    subst hcons
    dsimp only [typeAfter]
    simp [hsi, hdef, htargs, hwfst, hkey]
  case existsGeneric ii | moveFromGeneric ii =>
    obtain ⟨⟨d, sd, tys⟩, hparts, a, ha', u, hk, ⟨op, rest⟩, hpop, u', hop, h⟩ := h
    obtain ⟨si, hsi, rfl, hsd, htys⟩ := structInstParts_ok.1 hparts
    rw [popType_ok] at hpop
    subst hpop
    rw [rejectIf_ok] at hk hop
    simp only [bne_eq_false_iff_eq, Bool.not_eq_false'] at hk hop
    subst hop
    simp only [Except.ok.injEq] at h
    subst h
    simp only [Bytecode.OperandsConsistent] at hl9
    obtain ⟨x, hx, hgen⟩ := hl9
    rw [hsi, Option.some.injEq] at hx
    subst hx
    simp only [CompiledModule.SupportedOperands] at hops
    obtain ⟨x, hx, hops'⟩ := hops
    rw [hsi, Option.some.injEq] at hx
    subst hx
    simp only [Bytecode.TypeOperandsWellFormed] at hl5
    obtain ⟨x, cs, hx, hcs, hwf⟩ := hl5
    rw [hsi, Option.some.injEq] at hx
    subst hx
    obtain ⟨hne, id, name, sh, σ', hsh, hsid, hσ, hσlen, hdef, hst', hwfst, htargs, hmat⟩ :=
      hc.instOf hsd htys hgen hops' ⟨cs, hcs, hwf⟩
    obtain ⟨t0, ts0, hcons⟩ := cons_of_length_pos (l := σ') (by
      intro h0; subst h0
      have := ofTokenListWith_length hσ
      simp only [List.length_nil] at this
      exact hne (List.eq_nil_of_length_eq_zero this.symm))
    have hkey := AbilitySet.key_of_subset (habv _ ha' hmat) hk
    obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
    simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, Option.some.injEq] at hu1
    subst hu1
    subst hcons
    first
      | exact ⟨_, ofSigW_cons_eq rfl hus1, by
          dsimp only [typeAfter]; simp [hsi, hdef, htargs, hwfst, hkey]⟩
      | exact ⟨_, ofSigW_cons_eq hmat hus1, by
          dsimp only [typeAfter]; simp [hsi, hdef, htargs, hwfst, hkey]⟩
  case moveToGeneric ii =>
    obtain ⟨⟨d, sd, tys⟩, hparts, a, ha', u, hk, ⟨keyOp, rest⟩, hpop, ⟨signerOp, rest'⟩, hpop',
      u', hko, u'', hso, h⟩ := h
    obtain ⟨si, hsi, rfl, hsd, htys⟩ := structInstParts_ok.1 hparts
    rw [popType_ok] at hpop hpop'
    subst hpop
    simp only at hpop'
    subst hpop'
    rw [rejectIf_ok] at hk hko hso
    simp only [bne_eq_false_iff_eq, Bool.not_eq_false'] at hk hko hso
    subst hko hso
    simp only [Except.ok.injEq] at h
    subst h
    simp only [Bytecode.OperandsConsistent] at hl9
    obtain ⟨x, hx, hgen⟩ := hl9
    rw [hsi, Option.some.injEq] at hx
    subst hx
    simp only [CompiledModule.SupportedOperands] at hops
    obtain ⟨x, hx, hops'⟩ := hops
    rw [hsi, Option.some.injEq] at hx
    subst hx
    simp only [Bytecode.TypeOperandsWellFormed] at hl5
    obtain ⟨x, cs, hx, hcs, hwf⟩ := hl5
    rw [hsi, Option.some.injEq] at hx
    subst hx
    obtain ⟨hne, id, name, sh, σ', hsh, hsid, hσ, hσlen, hdef, hst', hwfst, htargs, hmat⟩ :=
      hc.instOf hsd htys hgen hops' ⟨cs, hcs, hwf⟩
    obtain ⟨t0, ts0, hcons⟩ := cons_of_length_pos (l := σ') (by
      intro h0; subst h0
      have := ofTokenListWith_length hσ
      simp only [List.length_nil] at this
      exact hne (List.eq_nil_of_length_eq_zero this.symm))
    have hkey := AbilitySet.key_of_subset (habv _ ha' hmat) hk
    obtain ⟨u1, us1, hu1, hus1, rfl⟩ := ofSigW_cons.1 hst
    obtain ⟨u2, us2, hu2, hus2, rfl⟩ := ofSigW_cons.1 hus1
    rw [ofSlotW_of_ofTokenW hmat, Option.some.injEq] at hu1
    subst hu1
    simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, Option.map_some,
      Option.some.injEq] at hu2
    subst hu2
    refine ⟨_, hus2, ?_⟩
    subst hcons
    dsimp only [typeAfter]
    simp [hsi, hdef, htargs, hwfst, hkey]



/-! ## Closures -/

section ClosuresW

variable {m : CompiledModule} {τ : List RuntimeType}

theorem ofSigW_filterZip (p : Nat → Bool) :
    ∀ {l : List SignatureToken} {r : List RuntimeType} (k : Nat),
      RuntimeType.ofSignatureWith m τ l = some r →
      RuntimeType.ofSignatureWith m τ (((l.zipIdx k).filter (fun x => p x.2)).map (·.1)) =
        some (((r.zipIdx k).filter (fun x => p x.2)).map (·.1))
  | [], r, k, h => by
    simp only [ofSigW_nil, Option.some.injEq] at h
    subst h
    simp [ofSigW_nil]
  | t :: ts, r, k, h => by
    obtain ⟨u, us, hu, hus, rfl⟩ := ofSigW_cons.1 h
    have ih := ofSigW_filterZip p (k + 1) hus
    simp only [List.zipIdx_cons, List.filter_cons]
    split
    · simp only [List.map_cons]
      exact ofSigW_cons.2 ⟨u, _, hu, ih, rfl⟩
    · exact ih

/-- Translation commutes with a mask's split of the parameters. -/
theorem ofSigW_maskSplit {mask : BitVec 64} {l : List SignatureToken} {r : List RuntimeType}
    (h : RuntimeType.ofSignatureWith m τ l = some r) :
    RuntimeType.ofSignatureWith m τ (maskExtract mask l true) = some (maskSplit mask r).1 ∧
      RuntimeType.ofSignatureWith m τ (maskExtract mask l false) = some (maskSplit mask r).2 := by
  constructor
  · have := ofSigW_filterZip (m := m) (τ := τ) (fun i => mask.getLsbD i == true) 0 h
    simpa [maskExtract, maskSplit] using this
  · have := ofSigW_filterZip (m := m) (τ := τ) (fun i => mask.getLsbD i == false) 0 h
    simpa [maskExtract, maskSplit] using this

theorem ofSigW_length : ∀ {l : List SignatureToken} {r : List RuntimeType},
    RuntimeType.ofSignatureWith m τ l = some r → r.length = l.length
  | [], r, h => by
    simp only [ofSigW_nil, Option.some.injEq] at h
    subst h
    rfl
  | _ :: _, r, h => by
    obtain ⟨_, _, -, hvs, rfl⟩ := ofSigW_cons.1 h
    simp [ofSigW_length hvs]

/-- Tokens that are no references translate to value types, under type
arguments that are value types. -/
theorem ofSigW_values (hτ : ∀ x ∈ τ, x.isValue = true) :
    ∀ {l : List SignatureToken} {r : List RuntimeType}, RuntimeType.ofSignatureWith m τ l = some r →
      (∀ t ∈ l, SignatureV2.isReference t = false) → ∀ u ∈ r, u.isValue = true
  | [], r, h, _, u, hu => by
    simp only [ofSigW_nil, Option.some.injEq] at h
    subst h
    simp at hu
  | t :: _, r, h, hl, u, hu => by
    obtain ⟨v, vs, hv, hvs, rfl⟩ := ofSigW_cons.1 h
    rcases List.mem_cons.1 hu with rfl | hu
    · have ht := hl t (List.mem_cons_self ..)
      exact ofSlotTokenWith_value hτ
        (by cases t <;> simp_all [SignatureV2.isReference, SignatureToken.isRefToken]) hv
    · exact ofSigW_values hτ hvs (fun x hx => hl x (List.mem_cons_of_mem _ hx)) u hu

theorem ofSlotW_function {ps rs : List SignatureToken} {ab : AbilitySet}
    {ps' rs' : List RuntimeType} (hps : RuntimeType.ofSignatureWith m τ ps = some ps')
    (hrs : RuntimeType.ofSignatureWith m τ rs = some rs') :
    RuntimeType.ofSlotTokenWith m τ (.function ps rs ab) = some (.function ps' rs' ab) := by
  simp [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, ofSlotTokensW_eq, hps, hrs]

end ClosuresW

theorem AllPairs.imp' {α β : Type} {R S : α → β → Prop} (h : ∀ a b, R a b → S a b) :
    ∀ {as : List α} {bs : List β}, AllPairs R as bs → AllPairs S as bs
  | _, _, .nil => .nil
  | _, _, .cons hr hrest => .cons (h _ _ hr) (AllPairs.imp' h hrest)

theorem AllPairs.right_forall {α β : Type} {R : α → β → Prop} {p : β → Prop}
    (h : ∀ a b, R a b → p b) : ∀ {as : List α} {bs : List β}, AllPairs R as bs → ∀ b ∈ bs, p b
  | _, _, .nil, _, hb => by simp at hb
  | _, _, .cons hr hrest, b, hb => by
    rcases List.mem_cons.1 hb with rfl | hb
    · exact h _ _ hr
    · exact AllPairs.right_forall h hrest b hb

/-- The captured arguments' abilities: a lower bound of the model's on every
captured token is a lower bound of the runtime types'. -/
theorem caps_abilities {P : Program} {m : CompiledModule} {cs : List AbilitySet}
    {τ : List RuntimeType} (hk : HandlesResolve P m)
    (hτ : RuntimeType.argumentsWellFormed P τ cs = true) {ab : AbilitySet} :
    ∀ {caps : List SignatureToken} {ps : List SignatureToken} {rc : List RuntimeType},
      AllPairs (fun a (_ : SignatureToken) => ∃ aa, abilities m cs a = .ok aa ∧ ab.subset aa = true)
        caps ps →
      RuntimeType.ofSignatureWith m τ caps = some rc → ∀ u ∈ rc, ab.subset (u.abilities P) = true
  | _, _, rc, .nil, hrc, u, hu => by
    simp only [ofSigW_nil, Option.some.injEq] at hrc
    subst hrc
    simp at hu
  | _ :: _, _, rc, .cons ⟨aa, haa, hsub⟩ hrest, hrc, u, hu => by
    obtain ⟨v, vs, hv, hvs, rfl⟩ := ofSigW_cons.1 hrc
    rcases List.mem_cons.1 hu with rfl | hu
    · exact AbilitySet.subset_trans hsub (abilities_sub_slot_gen hk hτ _ haa hv)
    · exact caps_abilities hk hτ hrest hvs u hu

/-- The model's initial closure abilities are within the specification's for
the target: a `persistent` handle resolves to a storable function (K1). -/
theorem closureInit_sub {P : Program} {fh : FunctionHandle} {g : FunctionRef}
    (h : FunctionAttribute.persistent ∈ fh.attributes → P.storable g = true) :
    (closureInitAbilities fh).subset (P.closureAbilities g) = true := by
  unfold closureInitAbilities
  split
  · next hp =>
    simp only [List.contains_iff_mem] at hp
    simp [publicFunctionAbilities, Program.closureAbilities, AbilitySet.subset, h hp]
  · simp [privateFunctionAbilities, Program.closureAbilities, AbilitySet.subset]

section ClosCore

variable {P : Program} {f : FunctionRef} {m : CompiledModule} {view : FunctionView}

/-- The part of `PackClosure` and `PackClosureGeneric` common to both: the
parameters at type arguments `τ'` split by the mask, the captured arguments
on the stack, the closure's type. The closure the model types has the
abilities it starts from and those of the captured arguments' tokens, within
the specification's: the target's and the captured arguments' types'. -/
theorem closure_core (hc : GenCtx P f m view) {τ' : List RuntimeType}
    (hτ' : ∀ x ∈ τ', x.isValue = true) {fh : FunctionHandle} {mask : BitVec 64}
    {params : List SignatureToken} {ps captured remaining rs : List RuntimeType}
    (hps : RuntimeType.ofSignatureWith m τ' params = some ps)
    (hsplit : maskSplit mask ps = (captured, remaining))
    (hbits : ∀ i, mask.getLsbD i = true → i < params.length)
    {caps rest : List SignatureToken} {before : List RuntimeType}
    (hst : RuntimeType.ofSignatureWith m f.typeArgs (caps ++ rest) = some before)
    {ab : AbilitySet}
    (hcaps : AllPairs (fun a t => SignatureV2.isReference t = false ∧
      ∃ aa, abilities m view.handle.typeParameters a = .ok aa ∧ ab.subset aa = true) caps
      (maskExtract mask params true).reverse)
    (hassign : ∀ rc, RuntimeType.ofSignatureWith m f.typeArgs caps = some rc →
      ∃ rp, RuntimeType.ofSignatureWith m τ' (maskExtract mask params true).reverse = some rp ∧
        RuntimeType.assignableAll rc rp = true)
    (hab : ab.subset (closureInitAbilities fh) = true) {target : FunctionRef}
    (hstor : FunctionAttribute.persistent ∈ fh.attributes → P.storable target = true)
    {t : SignatureToken}
    (ht : RuntimeType.ofSlotTokenWith m f.typeArgs t = some (.function remaining rs ab))
    (tys : List RuntimeType) :
    ∃ rb, RuntimeType.ofSignatureWith m f.typeArgs (t :: rest) =
        some (.function remaining rs ab :: rb) ∧
      maskWithin mask ps.length = true ∧ captured.all RuntimeType.isValue = true ∧
      popPush captured (.function remaining rs
        ((P.closureAbilities { target with typeArgs := tys }).inter
          (RuntimeType.capturedAbilities P (before.take captured.length)))) before =
        some (.function remaining rs ((P.closureAbilities { target with typeArgs := tys }).inter
          (RuntimeType.capturedAbilities P (before.take captured.length))) :: rb) ∧
      ab.subset ((P.closureAbilities { target with typeArgs := tys }).inter
        (RuntimeType.capturedAbilities P (before.take captured.length))) = true := by
  obtain ⟨rc, rb, hrc, hrb, rfl⟩ := ofSigW_append.1 hst
  obtain ⟨rp, hrp, hac⟩ := hassign rc hrc
  have hm := ofSigW_maskSplit (mask := mask) hps
  simp only [hsplit] at hm
  rw [ofSigW_reverse hm.1, Option.some.injEq] at hrp
  subst hrp
  obtain ⟨htake, -⟩ := assignable_take (rb := rb) hac
  refine ⟨rb, ofSigW_cons_slot ht hrb, ?_, ?_, ?_, ?_⟩
  · have hlen := ofSigW_length hps
    simp only [maskWithin, List.all_eq_true, List.mem_range]
    intro i _
    cases hb : mask.getLsbD i
    · rfl
    · simp [hlen, hbits i hb]
  · rw [List.all_eq_true]
    exact ofSigW_values hτ' hm.1 (fun t ht =>
      AllPairs.right_forall (fun _ _ h => h.1) hcaps t (List.mem_reverse.2 ht))
  · rw [htake]
    exact popPush_assignable hac
  · rw [htake]
    exact AbilitySet.subset_inter.2 ⟨AbilitySet.subset_trans hab (closureInit_sub hstor),
      capturedAbilities_ge (caps_abilities hc.resolves hc.targs
        (AllPairs.imp' (fun _ _ h => h.2) hcaps) hrc)⟩

theorem instantiate_nil {t q : SignatureToken} (h : instantiate [] t = .ok q) : q = t := by
  simp only [instantiate, List.isEmpty_nil, ↓reduceIte, pure, Except.pure, Except.ok.injEq] at h
  exact h.symm

set_option maxHeartbeats 1000000 in
/-- **C1 at an instance**, `PackClosure` and `PackClosureGeneric`: the
specification's type of the closure has the abilities of the captured
arguments' types at the instance, which may be more than the model's, from
the captured arguments' tokens, so the model's type after is assignable from
the specification's. -/
theorem typeAfter_closure (hc : GenCtx P f m view) {i : Bytecode} (hpk : packForm i = true)
    (hops : m.SupportedOperands i view.handle.typeParameters.length)
    (hl9 : i.OperandsConsistent m) (hl5 : i.TypeOperandsWellFormed m view.handle.typeParameters)
    (hcl : ClosureOk m i) {st st' : List SignatureToken} {before : List RuntimeType}
    (hst : RuntimeType.ofSignatureWith m f.typeArgs st = some before)
    (h : typeInstr m view st i = .ok st') :
    ∃ after a', RuntimeType.ofSignatureWith m f.typeArgs st' = some a' ∧
      typeAfter P f m i before = some after ∧ RuntimeType.assignableAll after a' = true := by
  cases i <;> simp [packForm] at hpk
  case packClosure hh mask =>
    simp only [typeInstr, bind_ok] at h
    obtain ⟨fh, hfh, h⟩ := h
    rw [lookup_ok] at hfh
    obtain ⟨params, caps, rest, ab, returns, t, hparams, rfl, -, hab, hall, hrets, ht, rfl⟩ :=
      typeClosPack_ok h
    have hnp : fh.typeParameters = [] := by
      simp only [Bytecode.OperandsConsistent, CompiledModule.functionTypeParamCount, hfh, bind,
        Option.bind_some, pure, Option.some.injEq] at hl9
      exact List.eq_nil_of_length_eq_zero hl9
    have ht := instantiate_nil ht
    subst ht
    obtain ⟨ps, rs, hps, hrs, hhp, hhr⟩ := hc.handleTypes0 hfh hnp hparams hrets
    obtain ⟨target, hres, hstor⟩ := hc.targets hh fh hfh
    rcases hsplit : maskSplit mask ps with ⟨captured, remaining⟩
    have hm := ofSigW_maskSplit (mask := mask) hps
    simp only [hsplit] at hm
    obtain ⟨rb, hst', hwithin, hval, hpp, hsubA⟩ := closure_core hc
      (argumentsWellFormed_values hc.targs) hps hsplit (hcl fh params hfh hparams) hst
      (AllPairs.imp' (fun _ _ h => h.2) hall)
      (fun rc hrc => allPairs_coreW (AllPairs.imp' (fun _ _ h => by
        obtain ⟨⟨q, hq, hqa⟩, -⟩ := h
        rw [← instantiate_nil hq]
        exact hqa) hall) hrc)
      hab hstor (ofSlotW_function hm.2 hrs) []
    refine ⟨.function remaining rs ((P.closureAbilities { target with typeArgs := [] }).inter
      (RuntimeType.capturedAbilities P (before.take captured.length))) :: rb, _, hst', ?_, ?_⟩
    · dsimp only [typeAfter]
      simp [hfh, hnp, hhp, hhr, hres, hwithin, hsplit, hpp]
      exact List.all_eq_true.1 hval
    · simp [RuntimeType.assignableAll, RuntimeType.assignable, hsubA]
  case packClosureGeneric ii mask =>
    simp only [typeInstr, bind_ok] at h
    obtain ⟨fi, hfi, tys, htys, fh, hfh, h⟩ := h
    rw [lookup_ok] at hfi hfh htys
    obtain ⟨params, caps, rest, ab, returns, t, hparams, rfl, -, hab, hall, hrets, ht, rfl⟩ :=
      typeClosPack_ok h
    simp only [Bytecode.OperandsConsistent] at hl9
    obtain ⟨x, hx, n, args0, hcount, hpos, hargs0, hlen0⟩ := hl9
    rw [hfi, Option.some.injEq] at hx
    subst hx
    rw [htys, Option.some.injEq] at hargs0
    subst hargs0
    have hcount' : tys.length = fh.typeParameters.length := by
      simp only [CompiledModule.functionTypeParamCount, hfh, bind, Option.bind_some, pure,
        Option.some.injEq] at hcount
      omega
    have hne : tys ≠ [] := by intro h0; subst h0; simp at hlen0; omega
    simp only [CompiledModule.SupportedOperands] at hops
    obtain ⟨x, hx, sig', hsig', hallsig⟩ := hops
    rw [hfi, Option.some.injEq] at hx
    subst hx
    rw [htys, Option.some.injEq] at hsig'
    subst hsig'
    obtain ⟨σ', hσ⟩ := Option.isSome_iff_exists.1 (hallsig f.typeArgs hc.targsLen)
    have hσlen : σ'.length = fh.typeParameters.length := by
      rw [ofTokenListWith_length hσ, hcount']
    simp only [Bytecode.TypeOperandsWellFormed] at hl5
    obtain ⟨x, fh', hx, hfh', targs0, htargs0, hwf⟩ := hl5
    rw [hfi, Option.some.injEq] at hx
    subst hx
    rw [hfh, Option.some.injEq] at hfh'
    subst hfh'
    rw [htys, Option.some.injEq] at htargs0
    subst htargs0
    have hok := translateList_ok hc.resolves hc.targs hwf hσ
    obtain ⟨ps, rs, hps, hrs, hhp, hhr⟩ := hc.handleTypesAt hfh hparams hrets hσlen
    obtain ⟨target, hres, hstor⟩ := hc.targets fi.handle fh hfh
    rcases hsplit : maskSplit mask ps with ⟨captured, remaining⟩
    have hm := ofSigW_maskSplit (mask := mask) hps
    simp only [hsplit] at hm
    have ht' : RuntimeType.ofSlotTokenWith m f.typeArgs t = some (.function remaining rs ab) := by
      rw [ofSlotTokenWith_instantiate hσ (instantiate_spec hne ht)]
      exact ofSlotW_function hm.2 hrs
    obtain ⟨rb, hst', hwithin, hval, hpp, hsubA⟩ := closure_core hc
      (argumentsWellFormed_values hok) hps hsplit (hcl fi fh params hfi hfh hparams) hst
      (AllPairs.imp' (fun _ _ h => h.2) hall)
      (fun rc hrc => allPairs_instW hne hσ (AllPairs.imp' (fun _ _ h => h.1) hall) hrc)
      hab hstor ht' σ'
    refine ⟨.function remaining rs ((P.closureAbilities { target with typeArgs := σ' }).inter
      (RuntimeType.capturedAbilities P (before.take captured.length))) :: rb, _, hst', ?_, ?_⟩
    · obtain ⟨t0, ts0, hcons⟩ := cons_of_length_pos (l := σ') (by
        intro h0; subst h0; simp at hσlen; omega)
      subst hcons
      dsimp only [typeAfter]
      simp [hfi, hfh, typeArgsWith_eq htys, hσ, hok, hhp, hhr, hres, hwithin, hsplit, hpp]
      exact List.all_eq_true.1 hval
    · simp [RuntimeType.assignableAll, RuntimeType.assignable, hsubA]

/-- **C1 at an instance**, every instruction: the model's type after is
assignable from the specification's. -/
theorem typeAfter_gen' (hc : GenCtx P f m view) {i : Bytecode} (hsub : i.inSubset = true)
    (hops : m.SupportedOperands i view.handle.typeParameters.length)
    (hl9 : i.OperandsConsistent m) (hl5 : i.TypeOperandsWellFormed m view.handle.typeParameters)
    (hcl : ClosureOk m i) {st st' : List SignatureToken} {before : List RuntimeType}
    (hb : verifyBounds m = .ok ())
    (hst : RuntimeType.ofSignatureWith m f.typeArgs st = some before)
    (h : typeInstr m view st i = .ok st')
    (hret : i = .ret → st' = []) :
    ∃ after a', RuntimeType.ofSignatureWith m f.typeArgs st' = some a' ∧
      typeAfter P f m i before = some after ∧ RuntimeType.assignableAll after a' = true := by
  cases hpk : packForm i
  · obtain ⟨after, h1, h2⟩ := typeAfter_gen hc hsub hpk hops hl9 hl5 hb hst h hret
    exact ⟨after, after, h1, h2, assignableAll_refl after⟩
  · exact typeAfter_closure hc hpk hops hl9 hl5 hcl hst h

end ClosCore

/-! ## The certificate at every instance -/

section Agreement

variable {P : Program} {f : FunctionRef} {m : CompiledModule} {view : FunctionView}

/-- At every instruction, the model's stack before it translates under the
instance's type arguments, and the instruction has the specification's
typing. -/
theorem types_at_gen (hc : GenCtx P f m view) (hb : verifyBounds m = .ok ())
    (hsub : ∀ i ∈ view.code.code.toList,
      i.inSubset = true ∧ m.SupportedOperands i view.handle.typeParameters.length)
    (hcl : ∀ i ∈ view.code.code.toList, ClosureOk m i)
    (hl9 : ∀ i ∈ view.code.code.toList, i.OperandsConsistent m)
    (hl5 : ∀ i ∈ view.code.code.toList, i.TypeOperandsWellFormed m view.handle.typeParameters)
    (hcert : ∀ pc, pc < view.code.code.size → ∃ st st',
      typesBefore m view pc = some st ∧ typeStep m view st pc = .ok st' ∧
      (pc + 1 < view.code.code.size → pc + 1 ∉ blockIds view.code.code →
        typesBefore m view (pc + 1) = some st') ∧
      (pc + 1 = view.code.code.size ∨ pc + 1 ∈ blockIds view.code.code → st' = [])) :
    ∀ pc, pc < view.code.code.size → ∃ st st' before after a' instr,
      view.code.code[pc]? = some instr ∧ typesBefore m view pc = some st ∧
      RuntimeType.ofSignatureWith m f.typeArgs st = some before ∧ typeStep m view st pc = .ok st' ∧
      RuntimeType.ofSignatureWith m f.typeArgs st' = some a' ∧
      typeAfter P f m instr before = some after ∧ RuntimeType.assignableAll after a' = true ∧
      (pc + 1 < view.code.code.size → pc + 1 ∉ blockIds view.code.code →
        typesBefore m view (pc + 1) = some st') ∧
      (pc + 1 = view.code.code.size ∨ pc + 1 ∈ blockIds view.code.code → st' = []) := by
  intro pc
  induction pc with
  | zero =>
    intro hpc
    obtain ⟨st, st', h1, h2, h3, h4⟩ := hcert 0 hpc
    simp only [typesBefore, Option.some.injEq] at h1
    subst h1
    obtain ⟨instr, hi⟩ : ∃ instr, view.code.code[0]? = some instr :=
      ⟨_, Array.getElem?_eq_getElem hpc⟩
    rw [typeStep_eq hi] at h2
    have hmem := getElem?_mem_toList hi
    obtain ⟨after, a', ha1, ha2, ha3⟩ := typeAfter_gen' hc (hsub _ hmem).1 (hsub _ hmem).2
      (hl9 _ hmem) (hl5 _ hmem) (hcl _ hmem) hb ofSigW_nil h2
      (fun hr => h4 (branch_ends hi (by subst hr; rfl)))
    exact ⟨[], st', [], after, a', instr, hi, rfl, ofSigW_nil, (typeStep_eq hi).trans h2, ha1, ha2,
      ha3, h3, h4⟩
  | succ p ih =>
    intro hpc
    obtain ⟨st, st', h1, h2, h3, h4⟩ := hcert (p + 1) hpc
    obtain ⟨before, hbefore⟩ : ∃ before, RuntimeType.ofSignatureWith m f.typeArgs st = some before := by
      by_cases hbid : p + 1 ∈ blockIds view.code.code
      · rw [typesBefore_of_mem m view hbid, Option.some.injEq] at h1
        subst h1
        exact ⟨[], ofSigW_nil⟩
      · obtain ⟨_, stp', _, _, afterp, _, _, _, _, _, hafp, _, _, hfall, _⟩ := ih (by omega)
        rw [hfall hpc hbid, Option.some.injEq] at h1
        subst h1
        exact ⟨afterp, hafp⟩
    obtain ⟨instr, hi⟩ : ∃ instr, view.code.code[p + 1]? = some instr :=
      ⟨_, Array.getElem?_eq_getElem hpc⟩
    rw [typeStep_eq hi] at h2
    have hmem := getElem?_mem_toList hi
    obtain ⟨after, a', ha1, ha2, ha3⟩ := typeAfter_gen' hc (hsub _ hmem).1 (hsub _ hmem).2
      (hl9 _ hmem) (hl5 _ hmem) (hcl _ hmem) hb hbefore h2
      (fun hr => h4 (branch_ends hi (by subst hr; rfl)))
    exact ⟨st, st', before, after, a', instr, hi, h1, hbefore, (typeStep_eq hi).trans h2, ha1, ha2,
      ha3, h3, h4⟩

end Agreement

/-- The struct definitions of a module of an accepted program resolve to
themselves. -/
theorem own_structs {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {mi : Nat} {m : CompiledModule} (hm : P.modules[mi]? = some m) :
    ∀ d sd, m.structDefs[d]? = some sd → ∃ id name sh,
      m.structHandles[sd.structHandle]? = some sh ∧
      m.structIdAt sd.structHandle = some ⟨id, name, []⟩ ∧
      P.definedStruct? mi d = some ⟨id, name, []⟩ ∧
      P.resolveStruct ⟨id, name, []⟩ = some ⟨mi, d⟩ ∧
      P.struct? ⟨mi, d⟩ = some ⟨m, id, sd, sh⟩ := by
  intro d sd hsd
  have hv := verifyModule_ok (wellFormed_module hwf hm).1
  obtain ⟨id, hid⟩ := module_selfId hwf hm
  obtain ⟨sh, name, hsh, hname, hsid, hres, hst⟩ := resolve_own_struct hwf.1 hm hid
    (verifyDuplication_ok hv.2.1) (verifyDuplication_structs hv.2.1) (verifyBounds_handles hv.1) hsd
  refine ⟨id, name, sh, hsh, hsid, ?_, hres, hst⟩
  simp [Program.definedStruct?, hst, CompiledModule.structDefName,
    CompiledModule.structDefHandle, hsh, hname, hres]

/-- **The context of an instance** of a function of an accepted program of
the supported subset, whose type arguments satisfy its constraints. -/
theorem genCtx_of {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit}
    (hf : P.function? f = some info) (hcode : info.definition.code = some code)
    (hok : P.typeArgsOk f = true) {cfg : Cfg} {view : FunctionView}
    (hview : FunctionView.function info.module f.index code info.handle cfg = .ok view) :
    GenCtx P f info.module view := by
  obtain ⟨hm, -, hd, hfh⟩ := function?_parts hf
  obtain ⟨-, hvh, -, hps, hrs, hls, -⟩ := functionView_fields hview
  have hsup := hs.1 _ (module_mem hm)
  obtain ⟨hhandles, -, hsdefs, hconsts, hdefs⟩ := hsup
  have htargs : RuntimeType.argumentsWellFormed P f.typeArgs view.handle.typeParameters = true := by
    simp only [Program.typeArgsOk, hf] at hok
    rw [hvh]; exact hok
  have hlen : f.typeArgs.length = info.handle.typeParameters.length := by
    rw [← hvh]; exact argumentsWellFormed_length htargs
  obtain ⟨⟨psig, hpsig, hpsc⟩, ⟨rsig, hrsig, hrsc⟩⟩ := hhandles _ (getElem?_mem_toList hfh)
  obtain ⟨fh', hfh', ⟨lsig, hlsig, hlsc⟩, -⟩ := hdefs _ (getElem?_mem_toList hd) code hcode
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  rw [hps, Option.some.injEq] at hpsig
  rw [hrs, Option.some.injEq] at hrsig
  rw [hls, Option.some.injEq] at hlsig
  subst hpsig hrsig hlsig
  obtain ⟨ps, hps'⟩ := Option.isSome_iff_exists.1 (hpsc f.typeArgs hlen)
  obtain ⟨rs, hrs'⟩ := Option.isSome_iff_exists.1 (hrsc f.typeArgs hlen)
  obtain ⟨ls, hls'⟩ := Option.isSome_iff_exists.1 (hlsc f.typeArgs hlen)
  refine ⟨hm, fun h sh hsh => struct_handle_resolves hwf hm hsh, htargs,
    ⟨ps ++ ls, ?_, ofSigW_append_eq hps' hls'⟩, ⟨rs, ?_, hrs'⟩, hconsts, hhandles,
    fun d hd' => (hsdefs d hd').2, own_structs hwf hm, fun h fh hfh => ?_⟩
  · simp [Program.localTypes, hf, hcode, hps, hps', hls, hls']
  · simp [Program.returnTypes, hf, hrs, hrs']
  · obtain ⟨id, hid⟩ := module_selfId hwf hm
    obtain ⟨target, -, hres, -, -, -, -, -, hst⟩ := handle_resolves_full hwf hs hm hid hfh
    exact ⟨target, hres, hst⟩

/-- **C1** (static) on the supported subset, generic code included: the
model's type certificate is consistent at every instance whose type arguments
satisfy its function's constraints. -/
theorem modelTypes_consistent_gen {P : Program}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported) :
    (modelTypes P).Consistent P := by
  intro f info code hf hcode hok
  obtain ⟨-, ⟨cfg, view, hcf, hview, hsu, hts, -⟩⟩ := function_verified hwf hf hcode
  obtain ⟨hvc, hvh, hvcfg, -, hrs, -, -⟩ := functionView_fields hview
  have hc := genCtx_of hwf hs hf hcode hok hview
  have hacc := (wellFormed_module hwf (function?_parts hf).1).1
  have hsub : ∀ i ∈ view.code.code.toList,
      i.inSubset = true ∧ info.module.SupportedOperands i view.handle.typeParameters.length := by
    obtain ⟨-, -, -, -, hdefs⟩ := hs.1 _ (module_mem (function?_parts hf).1)
    obtain ⟨fh, hfh, -, hcodes⟩ :=
      hdefs _ (getElem?_mem_toList (function?_parts hf).2.2.1) code hcode
    rw [(function?_parts hf).2.2.2, Option.some.injEq] at hfh
    subst hfh
    rw [hvc, hvh]
    exact hcodes
  have hl9 : ∀ i ∈ view.code.code.toList, i.OperandsConsistent info.module := by
    rw [hvc]
    exact instructionConsistency_of_accepted hacc _ (getElem?_mem_toList (function?_parts hf).2.2.1)
      code hcode
  have hl5 : ∀ i ∈ view.code.code.toList,
      i.TypeOperandsWellFormed info.module view.handle.typeParameters := by
    obtain ⟨-, -, -, htf⟩ := typeFormation_of_accepted hacc
    obtain ⟨fh, hfh, -, hops⟩ := htf _ (getElem?_mem_toList (function?_parts hf).2.2.1) code hcode
    rw [(function?_parts hf).2.2.2, Option.some.injEq] at hfh
    subst hfh
    rw [hvc, hvh]
    exact hops
  have hblocks : view.cfg.blocks = buildBlocks view.code.code := by
    rw [hvcfg, verifyControlFlow_blocks hcf, hvc]
  have hcl : ∀ i ∈ view.code.code.toList, ClosureOk info.module i := by
    rw [hvc]
    exact closureOk_of_accepted hacc (getElem?_mem_toList (function?_parts hf).2.2.1) hcode
  have hcert := typeCertificate (effectsAgree_of_typeSafety hrs hcl hts hblocks) hcl hblocks hsu hts
  have hbounds := (verifyModule_ok hacc).1
  have hat := types_at_gen hc hbounds hsub hcl hl9 hl5 hcert
  rw [hvc] at hat
  have hfall := verifyControlFlow_fallthrough hcf
  have hlt : ∀ pc, ∀ s ∈ successors code.code pc, s < code.code.size :=
    successors_lt hfall (fun pc o i hi ho => branch_targets hbounds
      (getElem?_mem_toList (function?_parts hf).2.2.1) hcode hi ho)
  have htypes := modelTypes_eq hf hcode hview
  refine ⟨by rw [htypes]; rfl, fun pc instr hreach hinstr => ?_⟩
  have hpc := lt_of_getElem? hinstr
  obtain ⟨st, st', before, after, a', instr', hi, h1, hb, -, ha1, ha2, ha3, hfl, hend⟩ :=
    hat pc hpc
  rw [hinstr, Option.some.injEq] at hi
  subst hi
  refine ⟨before, after, by rw [htypes, h1]; exact hb, ha2, fun s hs' => ⟨a', ?_, ha3⟩⟩
  have hslt := hlt pc s hs'
  rw [htypes]
  have hzero : pc + 1 = code.code.size ∨ pc + 1 ∈ blockIds code.code → a' = [] := by
    intro hend'
    rw [hend hend'] at ha1
    simpa [ofSigW_nil] using ha1.symm
  have hmem : ∀ x ∈ blockIds code.code,
      (typesBefore info.module view x).bind
        (RuntimeType.ofSignatureWith info.module f.typeArgs) = some [] := by
    intro x hx
    rw [← hvc] at hx
    rw [typesBefore_of_mem info.module view hx]
    rfl
  simp only [successors, hinstr] at hs'
  cases hctl : instr.control with
  | fallThrough =>
    rw [hctl] at hs'
    simp only [List.mem_singleton] at hs'
    subst hs'
    by_cases hbid : pc + 1 ∈ blockIds code.code
    · rw [hzero (Or.inr hbid)]; exact hmem _ hbid
    · rw [hfl hslt hbid]; exact ha1
  | jump o =>
    rw [hctl] at hs'
    simp only [List.mem_singleton] at hs'
    subst hs'
    obtain ⟨ho, hbr⟩ := control_jump hctl
    rw [hzero (branch_ends hinstr hbr)]
    exact hmem _ (offset_mem hinstr ho)
  | branch o =>
    rw [hctl] at hs'
    obtain ⟨ho, hbr⟩ := control_branch hctl
    rw [hzero (branch_ends hinstr hbr)]
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hs'
    rcases hs' with rfl | rfl
    · exact hmem _ (offset_mem hinstr ho)
    · rcases branch_ends hinstr hbr with h' | h'
      · omega
      · exact hmem _ h'
  | exit => rw [hctl] at hs'; simp at hs'

/-- **C1** is expressed over each function's type parameters: the model's
certificate at an instance is its stacks, the same for every instance,
translated under the instance's type arguments. -/
theorem modelTypes_parametric (P : Program) : (modelTypes P).Parametric P := by
  refine ⟨fun mi idx pc =>
    match P.function? ⟨mi, idx, []⟩ with
    | none => none
    | some info =>
      match info.definition.code with
      | none => none
      | some code =>
        match FunctionView.function info.module idx code info.handle default with
        | .error _ => none
        | .ok view => typesBefore info.module view pc, fun f info hf pc => ?_⟩
  have hf' : P.function? ⟨f.module, f.index, []⟩ = some info := by
    rw [← hf]; rfl
  simp only [modelTypes, hf, hf']
  cases info.definition.code with
  | none => rfl
  | some code =>
    simp only
    cases FunctionView.function info.module f.index code info.handle default <;> rfl

end MoveBytecode.Proofs
