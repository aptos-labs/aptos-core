-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Linking
import Proofs.RefModel
import Model.StructApi
import Model.Acquires

/-!
# The callee of a struct-API mutable borrow

UNTRUSTED (checked by the kernel). The reference-safety analysis treats a
call through a handle with a `BorrowFieldMutable off` attribute as a mutable
borrow of field `off` of its reference argument
(`ReferenceSafety.callOrSpecialize`). In an accepted, supported program
whose modules carry struct-API attributes only from bytecode version 10, the
callee is exactly such a borrow (`apiCall_wrapper`): the attribute is on the
definition's own handle (the same handle, or an imported one the dependency
check found compatible: the attribute cannot be removed), its module is of
version 10 or later, so the struct-API checker validated it: a single
parameter `&mut S`, a single result `&mut T` with `T` the type of field
`off`, and the code `MoveLoc(0); MutBorrowField | MutBorrowVariantField; Ret`
borrowing field `off` of `S`. With no global operation and no call, it
acquires nothing.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Attributes -/

theorem borrowFieldMutable_mem {attrs : List FunctionAttribute} {off : Nat}
    (h : ReferenceSafety.borrowFieldMutable attrs = some off) :
    FunctionAttribute.borrowFieldMutable off ∈ attrs := by
  unfold ReferenceSafety.borrowFieldMutable at h
  obtain ⟨a, ha, hf⟩ := List.exists_of_findSome?_eq_some h
  cases a <;> simp at hf
  subst hf
  exact ha

theorem borrowFieldMutable_of_mem {attrs : List FunctionAttribute} {off : Nat}
    (h : FunctionAttribute.borrowFieldMutable off ∈ attrs) :
    ∃ off', ReferenceSafety.borrowFieldMutable attrs = some off' := by
  unfold ReferenceSafety.borrowFieldMutable
  cases hf : attrs.findSome? (fun a => match a with
    | .borrowFieldMutable off => some off
    | .persistent | .moduleLock | .pack | .packVariant _ | .unpack | .unpackVariant _
    | .testVariant _ | .borrowFieldImmutable _ => none) with
  | some off' => exact ⟨off', rfl⟩
  | none =>
    rw [List.findSome?_eq_none_iff] at hf
    simp at hf
    exact absurd (hf _ h) (by simp)

/-- The struct-API attribute of a handle that carries `BorrowFieldMutable off`
is that one. -/
theorem structApiAttr_bfm {attrs : List FunctionAttribute} {attr? : Option FunctionAttribute}
    (h : StructApi.structApiAttr attrs = .ok attr?) {off : Nat}
    (hm : FunctionAttribute.borrowFieldMutable off ∈ attrs) :
    attr? = some (.borrowFieldMutable off) := by
  unfold StructApi.structApiAttr at h
  have hf : FunctionAttribute.borrowFieldMutable off ∈ attrs.filter StructApi.isStructApi :=
    List.mem_filter.2 ⟨hm, rfl⟩
  split at h
  · next he => rw [he] at hf; cases hf
  · next a he =>
    rw [he, List.mem_singleton] at hf
    subst hf
    simp only [pure, Except.pure, Except.ok.injEq] at h
    exact h.symm
  · simp [StructApi.apiError, reject] at h

/-! ## The definition's handle -/

/-- An attribute that cannot be removed, on an imported handle, is on the
definition's handle the dependency check found. -/
theorem verifyDependencies_attrs {m : CompiledModule} {dependencies : List CompiledModule}
    (h : verifyDependencies m dependencies = .ok ()) :
    ∃ self deps, selfModuleId m = .ok self ∧
      (∀ e ∈ deps, e.2 ∈ dependencies ∧ e.2.selfId = some e.1 ∧ e.1 ≠ self) ∧
      ∀ fh ∈ m.functionHandles.toList, fh.module ≠ m.selfModuleHandleIdx →
        ∃ ownerId name owner oh od dfh, m.moduleIdAt fh.module = some ownerId ∧
          m.identifiers[fh.name]? = some name ∧ dependencyLookup deps ownerId = some owner ∧
          callableFunction self owner name = .ok (some (oh, od)) ∧
          owner.functionHandles[oh]? = some dfh ∧
          (∃ params dparams, m.signatures[fh.parameters]? = some params ∧
            owner.signatures[dfh.parameters]? = some dparams ∧
            compareSignatures m owner params dparams = .ok ()) ∧
          (∃ returns dreturns, m.signatures[fh.returns]? = some returns ∧
            owner.signatures[dfh.returns]? = some dreturns ∧
            compareSignatures m owner returns dreturns = .ok ()) ∧
          ∀ a ∈ fh.attributes, cannotBeRemoved a = true → a ≠ .persistent →
            a ∈ dfh.attributes := by
  unfold verifyDependencies at h
  simp only [bind_ok] at h
  obtain ⟨self, hself, deps, hdeps, h⟩ := h
  refine ⟨self, deps, hself, deps_fold hdeps, fun fh hfh hne => ?_⟩
  obtain ⟨_, -, _, -, _, -, h⟩ := h
  have hb := forM_ok h fh hfh
  simp only [bne_iff_ne, ne_eq, hne, not_false_eq_true, ↓reduceIte, bind_ok,
    lookup_ok] at hb
  obtain ⟨mh, hmh, ownerId, hid, name, hname, hb⟩ := hb
  split at hb
  · next owner howner =>
    simp only [bind_ok] at hb
    obtain ⟨r, hr, hb⟩ := hb
    split at hb
    · next oh od =>
      simp only [bind_ok, lookup_ok] at hb
      obtain ⟨dfh, hdfh, hb⟩ := hb
      split at hb
      · simp [reject, bind, Except.bind] at hb
      · simp only [bind_ok, lookup_ok] at hb
        obtain ⟨params, hps, dparams, hdps, u1, hc1, returns, hrs, dreturns, hdrs, u2, hc2,
          hb⟩ := hb
        cases u1
        cases u2
        refine ⟨ownerId, name, owner, oh, od, dfh,
          by simp [CompiledModule.moduleIdAt, hmh, moduleIdForHandle_ok hid], hname, howner, hr,
          hdfh, ⟨params, dparams, hps, hdps, hc1⟩, ⟨returns, dreturns, hrs, hdrs, hc2⟩,
          fun a ha hc hp => ?_⟩
        have hcomp : ∀ D : List FunctionAttribute,
            (if (!attributesCompatible fh.attributes D) = true then reject "LINKER_ERROR"
              else (Except.ok () : VResult Unit)) = .ok () → a ∈ D := by
          intro D hD
          split at hD
          · simp [reject] at hD
          · next hcomp =>
            simp only [Bool.not_eq_true', Bool.not_eq_false] at hcomp
            unfold attributesCompatible at hcomp
            have := List.all_eq_true.1 hcomp a ha
            simpa [hc] using this
        by_cases hcond : (!fh.attributes.isEmpty && dfh.attributes.isEmpty) = true
        · simp only [hcond, ↓reduceIte, bind_ok, lookup_ok] at hb
          obtain ⟨fd, -, hb⟩ := hb
          split at hb
          · simp only [pure, Except.pure, bind, Except.bind] at hb
            have := hcomp _ hb
            simp only [List.mem_singleton] at this
            exact absurd this hp
          · simp only [pure, Except.pure, bind, Except.bind] at hb
            exact hcomp _ hb
        · simp only [hcond, Bool.false_eq_true, ↓reduceIte] at hb
          simp only [pure, Except.pure, bind, Except.bind] at hb
          exact hcomp _ hb
    · simp [reject] at hb
  · simp [reject] at hb

/-- **An attribute that cannot be removed** from the handle a call names is on
the called definition's own handle. -/
theorem resolve_attr {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {mi : Nat} {m : CompiledModule} (hm : P.modules[mi]? = some m) {h : FunctionHandleIndex}
    {fh : FunctionHandle} (hfh : m.functionHandles[h]? = some fh) {callee : FunctionRef}
    (hres : P.resolve mi h = some callee) {cinfo : FunctionInfo}
    (hcf : P.function? callee = some cinfo) {a : FunctionAttribute} (ha : a ∈ fh.attributes)
    (hc : cannotBeRemoved a = true) (hp : a ≠ .persistent) : a ∈ cinfo.handle.attributes := by
  have hdist := hwf.1
  have hv := verifyModule_ok (wellFormed_module hwf hm).1
  have hu := verifyDuplication_ok hv.2.1
  have hb := verifyBounds_handles hv.1
  obtain ⟨id, hid⟩ := module_selfId hwf hm
  by_cases hself : fh.module = m.selfModuleHandleIdx
  · obtain ⟨di, d, hres', -, -, hf⟩ := resolve_self hdist hm hid hu hb hfh hself
    rw [hres, Option.some.injEq] at hres'
    subst hres'
    rw [hcf, Option.some.injEq] at hf
    subst hf
    exact ha
  · obtain ⟨self, deps, hselfId, hdeps, himp⟩ :=
      verifyDependencies_attrs (by
        have := (wellFormed_module hwf hm).2
        rw [verifyLinking, show VerifierConfig.production.scope = .everything from rfl] at this
        simpa using this)
    rw [selfModuleId_ok hselfId, Option.some.injEq] at hid
    subst hid
    obtain ⟨ownerId, name, owner, oh, od, dfh, hmid, hname, howner, hcall, hdfh, -, -, hattr⟩ :=
      himp fh (getElem?_mem_toList hfh) hself
    obtain ⟨hin, hoid, -⟩ := hdeps _ (dependencyLookup_some howner)
    obtain ⟨mi', -, hmi'⟩ := mem_others hin
    have hu' := verifyDuplication_ok (verifyModule_ok (wellFormed_module hwf hmi').1).2.1
    obtain ⟨fdef, fh', hfd, hfdf, hfh', hfname, -⟩ := callableFunction_some hcall
    rw [hdfh, Option.some.injEq] at hfh'
    subst hfh' hfdf
    have hdn : owner.functionDefName fdef = some name := by
      simp [CompiledModule.functionDefName, hdfh, hfname]
    have huniq : Program.uniqueIdx owner.functionDefs
        (fun d => owner.functionDefName d == some name) = some od :=
      uniqueIdx_eq hfd (by simp [hdn]) fun j e hj he => by
        simp only [beq_iff_eq] at he
        exact defName_unique hu' hj hfd he hdn
    have hres' : P.resolve mi h = some ⟨mi', od, []⟩ := by
      simp only [Program.resolve, hm, hfh, hmid, hname, findModule_eq hdist hmi' hoid,
        Option.bind_eq_bind, Option.bind_some]
      rw [hmi']
      simp [huniq]
    have hf : P.function? ⟨mi', od, []⟩ = some ⟨owner, ownerId, fdef, dfh⟩ := by
      simp [Program.function?, hmi', hoid, hfd, CompiledModule.functionDefHandle, hdfh]
    rw [hres, Option.some.injEq] at hres'
    subst hres'
    rw [hcf, Option.some.injEq] at hf
    subst hf
    exact hattr a ha hc hp

/-! ## The struct-API checker -/

/-- Every function definition of a module passed the struct-API check. -/
theorem structApiCheck_of {config : VerifierConfig} {m : CompiledModule}
    (h : verifyCodeUnits config m = .ok ()) {index : Nat} {fdef : FunctionDefinition}
    (hd : m.functionDefs[index]? = some fdef) :
    ∃ ctx?, StructApi.contextFor m = .ok ctx? ∧ StructApi.check m ctx? fdef = .ok () := by
  unfold verifyCodeUnits at h
  simp only [bind_ok] at h
  obtain ⟨ctx?, hctx, total, hfold, -⟩ := h
  have hmem : (fdef, index) ∈ m.functionDefs.toList.zipIdx := by
    rw [List.mem_zipIdx_iff_getElem?]
    simpa using hd
  obtain ⟨acc, r, hr⟩ := foldlM_ok_mem hfold _ hmem
  simp only [bind_ok] at hr
  obtain ⟨u, hu, -⟩ := hr
  cases u
  exact ⟨ctx?, hctx, hu⟩

theorem contextFor_ten {m : CompiledModule} {ctx? : Option StructApi.Context}
    (h : StructApi.contextFor m = .ok ctx?) (hv : 10 ≤ m.version) :
    ∃ ctx, ctx? = some ctx ∧ StructApi.context m = .ok ctx := by
  unfold StructApi.contextFor at h
  rw [if_pos hv] at h
  cases hc : StructApi.context m with
  | error e => rw [hc] at h; simp [Functor.map, Except.map] at h
  | ok ctx =>
    rw [hc] at h
    simp only [Functor.map, Except.map, Except.ok.injEq] at h
    exact ⟨ctx, h.symm, rfl⟩

/-- The context records each struct definition with its handle. -/
theorem context_structs {m : CompiledModule} {ctx : StructApi.Context}
    (h : StructApi.context m = .ok ctx) :
    ∀ e ∈ ctx.structs, ∃ sd, m.structDefs[e.2.2]? = some sd ∧ sd.structHandle = e.2.1 := by
  unfold StructApi.context at h
  simp only [bind_ok] at h
  obtain ⟨⟨structs, enums⟩, hfold, u, -, h⟩ := h
  simp only [pure, Except.pure, Except.ok.injEq] at h
  subst h
  refine foldlM_inv (fun (acc : List (String × Nat × Nat) × List (String × StructApi.EnumMaps)) =>
    ∀ e ∈ acc.1, ∃ sd, m.structDefs[e.2.2]? = some sd ∧ sd.structHandle = e.2.1) (by simp) ?_
    hfold
  intro acc ⟨sd, idx⟩ r hmem hacc hr
  rw [List.mem_zipIdx_iff_getElem?] at hmem
  simp only [Array.getElem?_toList] at hmem
  simp only [bind_ok, lookup_ok] at hr
  obtain ⟨sh, -, name, -, hr⟩ := hr
  have hnew : ∀ e ∈ acc.1 ++ [(name, sd.structHandle, idx)],
      ∃ sd', m.structDefs[e.2.2]? = some sd' ∧ sd'.structHandle = e.2.1 := by
    intro e he
    rcases List.mem_append.1 he with he | he
    · exact hacc e he
    · simp only [List.mem_singleton] at he
      subst he
      exact ⟨sd, by simpa using hmem, rfl⟩
  split at hr
  · simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at hr
    obtain ⟨_, -, rfl⟩ := hr
    exact hnew
  · simp only [pure, Except.pure, Except.ok.injEq] at hr
    subst hr
    exact hnew
  · simp only [pure, Except.pure, Except.ok.injEq] at hr
    subst hr
    exact hnew

theorem context_struct? {ctx : StructApi.Context} {name : String} {h d : Nat}
    (hs : ctx.struct? name = some (h, d)) : ∃ n, (n, h, d) ∈ ctx.structs := by
  unfold StructApi.Context.struct? at hs
  simp only [Option.map_eq_some_iff] at hs
  obtain ⟨e, he, hed⟩ := hs
  have hmem := List.mem_of_getLast? he
  obtain ⟨n, h', d'⟩ := e
  simp only at hed
  rw [hed] at hmem
  exact ⟨n, (List.mem_filter.1 hmem).1⟩

/-- The code `checkPattern` accepts with one `MoveLoc`. -/
theorem checkPattern_one {code : CodeUnit} {p : Bytecode → Bool}
    (h : StructApi.checkPattern code 1 p = .ok ()) :
    ∃ b, code.code.toList = [.moveLoc 0, b, .ret] ∧ p b = true := by
  unfold StructApi.checkPattern at h
  generalize code.code.toList = l at h ⊢
  rcases l with _ | ⟨x, _ | ⟨y, _ | ⟨z, _ | ⟨w, l⟩⟩⟩⟩
  all_goals try (simp [StructApi.apiError, reject, bind, Except.bind] at h; done)
  cases hz : z
  all_goals try (rw [hz] at h; simp [StructApi.apiError, reject, bind, Except.bind] at h; done)
  rw [hz] at h
  cases hp : p y
  · simp [hp, StructApi.apiError, reject, bind, Except.bind] at h
  · cases hx : x
    all_goals try (rw [hx] at h; simp [hp, StructApi.apiError, reject, bind, Except.bind] at h)
    rename_i j
    by_cases hj : j = 0
    · subst hj
      exact ⟨y, rfl, hp⟩
    · simp [hj] at h

/-! ## A wrapper the struct-API checker accepted -/

/-- The borrow a struct-API mutable borrow wrapper performs: a field borrow of
field `off` of struct definition `sd`, or a variant field borrow at offset
`off` of the variants of enum `sd` listed in its handle; a generic form at
the canonical instance, the wrapper's `n` type parameters in order. -/
def BorrowOp (m : CompiledModule) (b : Bytecode) (sd off n : Nat) : Prop :=
  (∃ fhi fhd, b = .mutBorrowField fhi ∧ m.fieldHandles[fhi]? = some fhd ∧ fhd.owner = sd ∧
    fhd.field = off) ∨
  (∃ vh vfhd, b = .mutBorrowVariantField vh ∧ m.variantFieldHandles[vh]? = some vfhd ∧
    vfhd.structIndex = sd ∧ vfhd.field = off) ∨
  (∃ i fi fhd args, b = .mutBorrowFieldGeneric i ∧ m.fieldInstantiations[i]? = some fi ∧
    m.fieldHandles[fi.handle]? = some fhd ∧ fhd.owner = sd ∧ fhd.field = off ∧
    m.signatures[fi.typeParameters]? = some args ∧ StructApi.ensureCanonical args n = .ok ()) ∨
  (∃ i fi vfhd args, b = .mutBorrowVariantFieldGeneric i ∧
    m.variantFieldInstantiations[i]? = some fi ∧ m.variantFieldHandles[fi.handle]? = some vfhd ∧
    vfhd.structIndex = sd ∧ vfhd.field = off ∧
    m.signatures[fi.typeParameters]? = some args ∧ StructApi.ensureCanonical args n = .ok ())

/-- Canonical type arguments are as many as the type parameters. -/
theorem ensureCanonical_length {args : List SignatureToken} {n : Nat}
    (h : StructApi.ensureCanonical args n = .ok ()) : args.length = n := by
  unfold StructApi.ensureCanonical at h
  split at h
  · next hc => simp only [Bool.and_eq_true, beq_iff_eq] at hc; exact hc.1
  · simp [StructApi.apiError, reject] at h

/-- The parameter of a canonical signature `[&mut S<ts>]` names `S` at its
canonical instance. -/
theorem checkTokensCanonical_mutRef {sh n : Nat} {inner : SignatureToken}
    (h : StructApi.checkTokensCanonical sh n [.mutableReference inner] = .ok ()) :
    ∀ ts, inner = .structInstantiation sh ts → StructApi.ensureCanonical ts n = .ok () := by
  intro ts hts
  subst hts
  simp only [StructApi.checkTokensCanonical, StructApi.checkTokenCanonical, bind_ok] at h
  obtain ⟨u, hu, -⟩ := h
  simp only [beq_self_eq_true, ↓reduceIte, bind_ok] at hu
  obtain ⟨u', hu', -⟩ := hu
  cases u'
  exact hu'

theorem checkBorrowParam_mut {w : StructApi.Wrapper} (h : w.checkBorrowParam true = .ok ()) :
    ∃ inner, w.m.signatures[w.handle.parameters]? = some [.mutableReference inner] ∧
      (inner = .struct w.structHandle ∨ ∃ ts, inner = .structInstantiation w.structHandle ts) := by
  unfold StructApi.Wrapper.checkBorrowParam StructApi.Wrapper.signatureAt at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨sig, hsig, h⟩ := h
  match sig, h with
  | [t], h =>
    cases t <;> simp [StructApi.apiError, reject, bind, Except.bind, pure, Except.pure] at h
    · rename_i inner
      cases inner <;> simp at h
    · rename_i inner
      refine ⟨inner, hsig, ?_⟩
      cases inner <;> simp at h
      · exact Or.inl (by rw [h])
      · exact Or.inr ⟨_, by rw [h]⟩
  | [], h => simp [StructApi.apiError, reject] at h
  | _ :: _ :: _, h => simp [StructApi.apiError, reject] at h

theorem validateBorrowReturnType_mut {w : StructApi.Wrapper} {sn : String} {off : Nat}
    {to : Option Nat} {ty : SignatureToken}
    (h : w.validateBorrowReturnType sn off to true = .ok ty) :
    w.m.signatures[w.handle.returns]? = some [.mutableReference ty] := by
  unfold StructApi.Wrapper.validateBorrowReturnType StructApi.Wrapper.signatureAt at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨sig, hsig, h⟩ := h
  match sig, h with
  | [t], h =>
    cases t
    case mutableReference inner =>
      suffices hty : ty = inner by rw [hty]; exact hsig
      simp only [↓reduceIte, pure, Except.pure, bind_ok,
        Except.ok.injEq, exists_eq_left'] at h
      obtain ⟨sdef, -, h⟩ := h
      repeat' split at h
      all_goals simp only [bind, Except.bind, StructApi.apiError, reject] at h
      all_goals (try split at h)
      all_goals simp_all
    all_goals simp [StructApi.apiError, reject, bind, Except.bind] at h
  | [], h => simp [StructApi.apiError, reject, bind, Except.bind] at h
  | _ :: _ :: _, h => simp [StructApi.apiError, reject, bind, Except.bind] at h

theorem resolveField_ok_gen {w : StructApi.Wrapper} {h o sig : Nat}
    (hr : w.resolveField h (some sig) = .ok o) :
    ∃ fhd, w.m.fieldHandles[h]? = some fhd ∧ fhd.owner = w.structDef ∧ fhd.field = o ∧
      ∃ args, w.m.signatures[sig]? = some args ∧
        StructApi.ensureCanonical args w.numTypeParams = .ok () := by
  unfold StructApi.Wrapper.resolveField StructApi.Wrapper.signatureAt at hr
  simp only [bind_ok, lookup_ok] at hr
  obtain ⟨fhd, hfhd, hr⟩ := hr
  split at hr
  · simp [StructApi.apiError, reject, bind, Except.bind] at hr
  · next hne =>
    simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hne
    simp only [bind_ok, lookup_ok, pure, Except.pure, Except.ok.injEq] at hr
    obtain ⟨args, hargs, u, hc, hr⟩ := hr
    exact ⟨fhd, hfhd, hne, hr, args, hargs, hc⟩

theorem resolveVariantField_ok_gen {w : StructApi.Wrapper} {h o sig : Nat}
    (hr : w.resolveVariantField h (some sig) = .ok o) :
    ∃ vfhd, w.m.variantFieldHandles[h]? = some vfhd ∧ vfhd.structIndex = w.structDef ∧
      vfhd.field = o ∧ ∃ args, w.m.signatures[sig]? = some args ∧
        StructApi.ensureCanonical args w.numTypeParams = .ok () := by
  unfold StructApi.Wrapper.resolveVariantField StructApi.Wrapper.signatureAt at hr
  simp only [bind_ok, lookup_ok] at hr
  obtain ⟨vfhd, hvfhd, hr⟩ := hr
  split at hr
  · simp [StructApi.apiError, reject, bind, Except.bind] at hr
  · next hne =>
    simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hne
    simp only [bind_ok, lookup_ok, pure, Except.pure, Except.ok.injEq] at hr
    obtain ⟨args, hargs, u, hc, hr⟩ := hr
    exact ⟨vfhd, hvfhd, hne, hr, args, hargs, hc⟩

theorem resolveField_ok {w : StructApi.Wrapper} {h o : Nat}
    (hr : w.resolveField h none = .ok o) :
    ∃ fhd, w.m.fieldHandles[h]? = some fhd ∧ fhd.owner = w.structDef ∧ fhd.field = o := by
  unfold StructApi.Wrapper.resolveField at hr
  simp only [bind_ok, lookup_ok] at hr
  obtain ⟨fhd, hfhd, hr⟩ := hr
  split at hr
  · simp [StructApi.apiError, reject, bind, Except.bind] at hr
  · next hne =>
    simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hne
    simp only [pure, Except.pure, Except.ok.injEq] at hr
    exact ⟨fhd, hfhd, hne, hr⟩

theorem resolveVariantField_ok {w : StructApi.Wrapper} {h o : Nat}
    (hr : w.resolveVariantField h none = .ok o) :
    ∃ vfhd, w.m.variantFieldHandles[h]? = some vfhd ∧ vfhd.structIndex = w.structDef ∧
      vfhd.field = o := by
  unfold StructApi.Wrapper.resolveVariantField at hr
  simp only [bind_ok, lookup_ok] at hr
  obtain ⟨vfhd, hvfhd, hr⟩ := hr
  split at hr
  · simp [StructApi.apiError, reject, bind, Except.bind] at hr
  · next hne =>
    simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hne
    simp only [pure, Except.pure, Except.ok.injEq] at hr
    exact ⟨vfhd, hvfhd, hne, hr⟩

theorem getBorrowFieldOffset_mut {w : StructApi.Wrapper} {b : Bytecode} {o : Nat}
    (hp : (match b with
      | .immBorrowField _ | .immBorrowFieldGeneric _ | .immBorrowVariantField _
      | .immBorrowVariantFieldGeneric _ => !true
      | .mutBorrowField _ | .mutBorrowFieldGeneric _ | .mutBorrowVariantField _
      | .mutBorrowVariantFieldGeneric _ => true
      | _ => false) = true)
    (h : w.getBorrowFieldOffset b true = .ok o) :
    BorrowOp w.m b w.structDef o w.numTypeParams := by
  cases b <;> simp at hp
  case mutBorrowField fhi =>
    unfold StructApi.Wrapper.getBorrowFieldOffset at h
    cases hr : w.resolveField fhi none with
    | error e => simp [hr, bind, Except.bind] at h
    | ok f =>
      simp [hr, bind, Except.bind, pure, Except.pure] at h
      obtain ⟨fhd, h1, h2, h3⟩ := resolveField_ok hr
      exact Or.inl ⟨fhi, fhd, rfl, h1, h2, by rw [h3]; simpa using h⟩
  case mutBorrowVariantField vh =>
    unfold StructApi.Wrapper.getBorrowFieldOffset at h
    cases hr : w.resolveVariantField vh none with
    | error e => simp [hr, bind, Except.bind] at h
    | ok f =>
      simp [hr, bind, Except.bind, pure, Except.pure] at h
      obtain ⟨vfhd, h1, h2, h3⟩ := resolveVariantField_ok hr
      exact Or.inr (Or.inl ⟨vh, vfhd, rfl, h1, h2, by rw [h3]; simpa using h⟩)
  case mutBorrowFieldGeneric i =>
    unfold StructApi.Wrapper.getBorrowFieldOffset at h
    simp only [bind_ok, lookup_ok] at h
    obtain ⟨fi, hfi, f', hr, p, hp, h⟩ := h
    simp only [pure, Except.pure, Except.ok.injEq] at hp
    subst hp
    obtain ⟨fhd, h1, h2, h3, args, ha, hc⟩ := resolveField_ok_gen hr
    exact Or.inr (Or.inr (Or.inl ⟨i, fi, fhd, args, rfl, hfi, h1, h2,
      by rw [h3]; simpa [pure, Except.pure, StructApi.apiError, reject] using h, ha, hc⟩))
  case mutBorrowVariantFieldGeneric i =>
    unfold StructApi.Wrapper.getBorrowFieldOffset at h
    simp only [bind_ok, lookup_ok] at h
    obtain ⟨fi, hfi, f', hr, p, hp, h⟩ := h
    simp only [pure, Except.pure, Except.ok.injEq] at hp
    subst hp
    obtain ⟨vfhd, h1, h2, h3, args, ha, hc⟩ := resolveVariantField_ok_gen hr
    exact Or.inr (Or.inr (Or.inr ⟨i, fi, vfhd, args, rfl, hfi, h1, h2,
      by rw [h3]; simpa [pure, Except.pure, StructApi.apiError, reject] using h, ha, hc⟩))

theorem checkBorrowPattern_mut {w : StructApi.Wrapper} {sn : String} {off : Nat}
    {ty : SignatureToken} (h : w.checkBorrowPattern true sn off ty = .ok ()) :
    ∃ b, w.code.code.toList = [.moveLoc 0, b, .ret] ∧
      BorrowOp w.m b w.structDef off w.numTypeParams := by
  unfold StructApi.Wrapper.checkBorrowPattern at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨u, hpat, op, hop, o, ho, h⟩ := h
  obtain ⟨b, hb, hp⟩ := checkPattern_one hpat
  have hob : op = b := by
    have h1 : w.code.code.toList[1]? = some b := by rw [hb]; rfl
    rw [Array.getElem?_toList, hop, Option.some.injEq] at h1
    exact h1
  subst hob
  refine ⟨op, hb, ?_⟩
  have hoo : o = off := by
    split at h
    · simp [StructApi.apiError, reject, bind, Except.bind] at h
    · next hne => simpa using hne
  subst hoo
  exact getBorrowFieldOffset_mut hp ho

/-- **What the struct-API checker establishes** of a definition whose handle
carries `BorrowFieldMutable off`: one parameter `&mut S`, one result `&mut T`,
and the code `MoveLoc(0); <mutable borrow of field off of S>; Ret`. -/
structure ApiDef (m : CompiledModule) (handle : FunctionHandle) (fdef : FunctionDefinition)
    (off : Nat) : Prop where
  code : ∃ code b, fdef.code = some code ∧ code.code.toList = [.moveLoc 0, b, .ret] ∧
    ∃ sd sh, BorrowOp m b sd off handle.typeParameters.length ∧
      ∃ inner, m.signatures[handle.parameters]? = some [.mutableReference inner] ∧
        (inner = .struct sh ∨ ∃ ts, inner = .structInstantiation sh ts ∧
          StructApi.ensureCanonical ts handle.typeParameters.length = .ok ())
  returns : ∃ ty, m.signatures[handle.returns]? = some [.mutableReference ty]

theorem checkFunction_bfm {m : CompiledModule} {ctx : StructApi.Context}
    {fdef : FunctionDefinition} (h : StructApi.checkFunction m ctx fdef = .ok ())
    {handle : FunctionHandle} (hh : m.functionHandles[fdef.function]? = some handle) {off : Nat}
    (hm : FunctionAttribute.borrowFieldMutable off ∈ handle.attributes) :
    ApiDef m handle fdef off := by
  unfold StructApi.checkFunction at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨handle', hh', name, -, attr?, hattr, h⟩ := h
  rw [hh, Option.some.injEq] at hh'
  subst hh'
  obtain rfl := structApiAttr_bfm hattr hm
  simp only at h
  split at h
  all_goals first
    | (simp [StructApi.apiError, reject] at h; done)
    | (simp_all; done)
    | skip
  rename_i _ _ data attr hpn hat
  simp only [Option.some.injEq] at hat
  subst hat
  split at h
  · simp [StructApi.apiError, reject, bind, Except.bind] at h
  next hma =>
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨sh, -, h⟩ := h
  split at h
  · simp [StructApi.apiError, reject, bind, Except.bind] at h
  split at h
  · simp [StructApi.apiError, reject, bind, Except.bind] at h
  next hlenTP =>
  have hlen : handle.typeParameters.length = sh.typeParameters.length := by simpa using hlenTP
  split at h
  · next code hcode =>
    simp only [pure, Except.pure, bind_ok, Except.ok.injEq, exists_eq_left'] at h
    obtain ⟨u, hcan, h⟩ := h
    simp only [StructApi.Wrapper.validate] at h
    have hmut : data.isMutable = true := by
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hma
      simp only [StructApi.Parsed.matchesAttr, Bool.and_eq_true, beq_iff_eq] at hma
      exact hma.2
    rw [hmut] at h
    split at h
    · simp only [bind_ok, pure, Except.pure, Except.ok.injEq, exists_eq_left'] at h
      split at h
      · simp [StructApi.apiError, reject, bind, Except.bind] at h
      · simp only [bind_ok] at h
        obtain ⟨u1, hparam, ty, hret, hpat⟩ := h
        obtain ⟨inner, hps, hinner⟩ := checkBorrowParam_mut hparam
        obtain ⟨b, hb, hop⟩ := checkBorrowPattern_mut hpat
        simp only [StructApi.Wrapper.checkSignatureCanonical, List.forM, bind_ok,
          StructApi.Wrapper.signatureAt, lookup_ok] at hcan
        obtain ⟨u2, ⟨sig, hsig, hcs⟩, -⟩ := hcan
        simp only at hps
        rw [hps, Option.some.injEq] at hsig
        subst hsig
        have hcm := checkTokensCanonical_mutRef hcs
        refine ⟨⟨code, b, hcode, hb, data.structDef, data.structHandle,
          by rw [hlen]; exact hop, inner, hps, ?_⟩,
          ⟨ty, validateBorrowReturnType_mut hret⟩⟩
        rcases hinner with h1 | ⟨ts, h1⟩
        · exact Or.inl h1
        · exact Or.inr ⟨ts, h1, by rw [hlen]; exact hcm ts h1⟩
    · simp [StructApi.apiError, reject, bind, Except.bind] at h
  · simp [StructApi.apiError, reject, bind, Except.bind] at h

/-! ## Every struct-API mutable borrow handle -/

theorem compareSignatures_single {m d : CompiledModule} {ps : List SignatureToken}
    {x : SignatureToken} (h : compareSignatures m d ps [x] = .ok ()) :
    ∃ p, ps = [p] ∧ compareTypes m d p x = .ok () := by
  match ps, h with
  | [], h => simp [compareSignatures, reject] at h
  | [p], h =>
    simp only [compareSignatures, bind_ok] at h
    obtain ⟨u, hu, -⟩ := h
    cases u
    exact ⟨p, rfl, hu⟩
  | p :: q :: rest, h =>
    simp only [compareSignatures, bind_ok] at h
    obtain ⟨u, -, h⟩ := h
    simp [reject] at h

theorem compareTypes_mutRef {m d : CompiledModule} {p x : SignatureToken}
    (h : compareTypes m d p (.mutableReference x) = .ok ()) :
    ∃ y, p = .mutableReference y := by
  cases p <;> simp [compareTypes, reject] at h
  exact ⟨_, rfl⟩

/-- **A definition whose handle carries `BorrowFieldMutable off`**, in an
accepted program whose modules carry struct-API attributes only from version
10, is a wrapper the struct-API checker accepted. -/
theorem defApi {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
      ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
    {mi : Nat} {m : CompiledModule} (hm : P.modules[mi]? = some m) {i : Nat}
    {fdef : FunctionDefinition} (hd : m.functionDefs[i]? = some fdef) {handle : FunctionHandle}
    (hh : m.functionHandles[fdef.function]? = some handle) {off : Nat}
    (hmem : FunctionAttribute.borrowFieldMutable off ∈ handle.attributes) :
    ApiDef m handle fdef off ∧ 10 ≤ m.version := by
  obtain ⟨-, -, -, hcu⟩ := verifyModule_ok (wellFormed_module hwf hm).1
  obtain ⟨ctx?, hctx, hchk⟩ := structApiCheck_of hcu hd
  obtain ⟨off', hoff'⟩ := borrowFieldMutable_of_mem hmem
  have hv := hapi m (module_mem hm) handle (getElem?_mem_toList hh) off' hoff'
  obtain ⟨ctx, rfl, -⟩ := contextFor_ten hctx hv
  exact ⟨checkFunction_bfm hchk hh hmem, hv⟩

/-- **Every struct-API mutable borrow handle** of an accepted program whose
modules carry struct-API attributes only from version 10 has one parameter
and one mutable reference result (`ApiSig`): a handle of the module's own
function is its definition's, which the struct-API checker shaped; an
imported one carries the attribute to the definition, whose signature the
dependency check compared with the handle's. -/
theorem apiSigs_of {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
      ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version) :
    P.ApiSigs := by
  intro m hmm fh hfhm off hoff
  have hmem := borrowFieldMutable_mem hoff
  obtain ⟨mi, hm⟩ := List.mem_iff_getElem?.1 hmm
  rw [Array.getElem?_toList] at hm
  obtain ⟨h, hfh⟩ := List.mem_iff_getElem?.1 hfhm
  rw [Array.getElem?_toList] at hfh
  have hv := verifyModule_ok (wellFormed_module hwf hm).1
  have hu := verifyDuplication_ok hv.2.1
  by_cases hself : fh.module = m.selfModuleHandleIdx
  · obtain ⟨fd, hfd, hfdh⟩ := hu.implemented h fh hfh hself
    obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 hfd
    rw [Array.getElem?_toList] at hi
    obtain ⟨⟨-, -, -, -, -, -, -, inner, hps, -⟩, ty, hrs⟩ :=
      (defApi hwf hapi hm hi (by rw [hfdh]; exact hfh) hmem).1
    exact ⟨_, _, hps, hrs, rfl⟩
  · obtain ⟨self, deps, hselfId, hdeps, himp⟩ :=
      verifyDependencies_attrs (by
        have := (wellFormed_module hwf hm).2
        rw [verifyLinking, show VerifierConfig.production.scope = .everything from rfl] at this
        simpa using this)
    obtain ⟨ownerId, name, owner, oh, od, dfh, -, -, howner, hcall, hdfh,
      ⟨params, dparams, hps, hdps, hc1⟩, ⟨returns, dreturns, hrs, hdrs, hc2⟩, hattr⟩ :=
      himp fh hfhm hself
    obtain ⟨hin, -, -⟩ := hdeps _ (dependencyLookup_some howner)
    obtain ⟨mi', -, hmi'⟩ := mem_others hin
    obtain ⟨fdef, fh', hfd, hfdf, hfh', -, -⟩ := callableFunction_some hcall
    rw [hdfh, Option.some.injEq] at hfh'
    subst hfh' hfdf
    have hdm := hattr _ hmem rfl (by simp)
    obtain ⟨⟨-, -, -, -, -, -, -, inner, hdps', -⟩, ty, hdrs'⟩ :=
      (defApi hwf hapi hmi' hfd hdfh hdm).1
    rw [hdps, Option.some.injEq] at hdps'
    rw [hdrs, Option.some.injEq] at hdrs'
    subst hdps' hdrs'
    obtain ⟨p, rfl, hp⟩ := compareSignatures_single hc1
    obtain ⟨r, rfl, hr⟩ := compareSignatures_single hc2
    obtain ⟨y, rfl⟩ := compareTypes_mutRef hr
    exact ⟨p, _, hps, hrs, rfl⟩

/-! ## The callee of a struct-API mutable borrow call -/

/-- A wrapper with no global operation and no call acquires nothing: the
acquires verifier rejects an annotation the code does not use. -/
theorem acquires_wrapper {m : CompiledModule} {fdef : FunctionDefinition} {code : CodeUnit}
    (h : verifyAcquires m fdef code = .ok ()) {b : Bytecode}
    (hb : code.code.toList = [.moveLoc 0, b, .ret])
    (hbk : (∃ i, b = .mutBorrowField i) ∨ ∃ i, b = .mutBorrowVariantField i) :
    fdef.acquiresGlobalResources = [] := by
  unfold verifyAcquires at h
  rw [hb] at h
  simp only [bind_ok] at h
  obtain ⟨actual, hfold, hann⟩ := h
  have hact : actual = [] := by
    rcases hbk with ⟨i, rfl⟩ | ⟨i, rfl⟩ <;>
      simp [List.foldlM, Acquires.instr, pure, Except.pure, bind, Except.bind] at hfold <;>
      exact hfold
  subst hact
  cases hann' : fdef.acquiresGlobalResources with
  | nil => rfl
  | cons s rest =>
    rw [hann'] at hann
    simp [List.forM, Acquires.annotation, reject, bind, Except.bind] at hann

/-- **The callee of a struct-API mutable borrow call**: everything the
semantics needs to run it, and what the caller's analysis assumed. The
definition's own handle carries the attribute; its module is of version 10
or later; it acquires nothing; it takes one `&mut` to a struct `sh` (struct
definition `sd`, a struct or an enum, named as a struct or as its instance
with no type arguments) and returns one `&mut`; its code is
`MoveLoc(0)`, a mutable borrow of field `off` of `sd` (a field handle with
owner `sd`, or a variant field handle of `sd` listing at least one variant),
then `Ret`. -/
structure WrapperCode (P : Program) (callee : FunctionRef) (cinfo : FunctionInfo) (off : Nat) :
    Prop where
  attr : FunctionAttribute.borrowFieldMutable off ∈ cinfo.handle.attributes
  version : 10 ≤ cinfo.module.version
  acquires : cinfo.definition.acquiresGlobalResources = []
  code : ∃ code b sd sdef sh inner, cinfo.definition.code = some code ∧
    code.code.toList = [.moveLoc 0, b, .ret] ∧
    cinfo.module.signatures[cinfo.handle.parameters]? = some [.mutableReference inner] ∧
    (inner = .struct sh ∨ inner = .structInstantiation sh []) ∧
    cinfo.module.structDefs[sd]? = some sdef ∧
    ((∃ fhi fhd, b = .mutBorrowField fhi ∧ cinfo.module.fieldHandles[fhi]? = some fhd ∧
        fhd.owner = sd ∧ fhd.field = off) ∨
      (∃ vh vfhd, b = .mutBorrowVariantField vh ∧
        cinfo.module.variantFieldHandles[vh]? = some vfhd ∧ vfhd.structIndex = sd ∧
        vfhd.field = off ∧ vfhd.variants ≠ []))
  returns : ∃ ty, cinfo.module.signatures[cinfo.handle.returns]? = some [.mutableReference ty]

/-- **A struct-API mutable borrow call calls a wrapper**, in an accepted,
supported program whose modules carry struct-API attributes only from
version 10. -/
theorem apiCall_wrapper {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.SupportedNonGeneric)
    (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
      ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
    {f : FunctionRef} {info : FunctionInfo} (hf : P.function? f = some info) {h : Nat}
    {fh : FunctionHandle} (hfh : info.module.functionHandles[h]? = some fh) {off : Nat}
    (hoff : ReferenceSafety.borrowFieldMutable fh.attributes = some off) {callee : FunctionRef}
    (hres : P.resolve f.module h = some callee) {cinfo : FunctionInfo}
    (hcf : P.function? callee = some cinfo) : WrapperCode P callee cinfo off := by
  obtain ⟨hm, -, -, -⟩ := function?_parts hf
  have hattr := resolve_attr hwf hm hfh hres hcf (borrowFieldMutable_mem hoff) rfl (by simp)
  obtain ⟨hcm, -, hcd, hch⟩ := function?_parts hcf
  obtain ⟨⟨⟨code, b, hcode, hb, sd, sh, hop, inner, hps, hinner⟩, hret⟩, hv⟩ :=
    defApi hwf hapi hcm hcd hch hattr
  have hsup := hs _ (module_mem hcm)
  have hbsub : b.inSubsetNG = true :=
    ((hsup.2.2.2.2.2 _ (getElem?_mem_toList hcd) code hcode).2 b (by rw [hb]; simp)).1
  obtain ⟨-, hpc, -⟩ := hsup.1 cinfo.handle (getElem?_mem_toList hch)
  -- A non-generic struct named as an instance has no type arguments.
  have hinner' : inner = .struct sh ∨ inner = .structInstantiation sh [] := by
    rcases hinner with h1 | ⟨ts, rfl, -⟩
    · exact Or.inl h1
    · right
      obtain ⟨sig, hsig, hsome⟩ := hpc
      rw [hps, Option.some.injEq] at hsig
      subst hsig
      simp only [RuntimeType.ofSignature, RuntimeType.ofSignatureWith, List.mapM_cons,
        List.mapM_nil, RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, bind,
        Option.isSome_bind] at hsome
      cases hshi : cinfo.module.structHandles[sh]? with
      | none => simp [hshi] at hsome
      | some shd =>
        have hnp := hsup.2.2.1 shd (getElem?_mem_toList hshi)
        cases ts with
        | nil => rfl
        | cons t ts =>
          exfalso
          revert hsome
          simp only [hshi]
          cases cinfo.module.structIdAt sh <;> simp
          cases hl : RuntimeType.ofTokenListWith cinfo.module [] (t :: ts) <;> simp
          intro hc
          have := ofTokenListWith_length hl
          simp only [List.length_cons] at this
          simp only [hnp, List.length_nil, this] at hc
          omega
  have hb0 := (verifyModule_ok (wellFormed_module hwf hcm).1).1
  have hbk : (∃ i, b = .mutBorrowField i) ∨ ∃ i, b = .mutBorrowVariantField i := by
    rcases hop with ⟨fhi, -, rfl, -⟩ | ⟨vh, -, rfl, -⟩ | ⟨i, -, -, -, rfl, -⟩ |
      ⟨i, -, -, -, rfl, -⟩
    · exact Or.inl ⟨fhi, rfl⟩
    · exact Or.inr ⟨vh, rfl⟩
    · simp [Bytecode.inSubsetNG, Bytecode.inCore, Bytecode.inDataByValue,
        Bytecode.inReferences] at hbsub
    · simp [Bytecode.inSubsetNG, Bytecode.inCore, Bytecode.inDataByValue,
        Bytecode.inReferences] at hbsub
  refine ⟨hattr, hv, acquires_wrapper (function_acquires hwf hcf hcode) hb hbk, ?_, hret⟩
  rcases hop with ⟨fhi, fhd, rfl, hfhd, hown, hfield⟩ | ⟨vh, vfhd, rfl, hvfhd, hsi, hfield⟩ |
    ⟨i, -, -, -, rfl, -⟩ | ⟨i, -, -, -, rfl, -⟩
  · obtain ⟨sdef, hsdef, -⟩ := verifyBounds_fieldHandle hb0 hfhd
    rw [hown] at hsdef
    exact ⟨code, _, sd, sdef, sh, inner, hcode, hb, hps, hinner', hsdef,
      Or.inl ⟨fhi, fhd, rfl, hfhd, hown, hfield⟩⟩
  · obtain ⟨sdef, hsdef, hne, -⟩ := verifyBounds_variantFieldHandle hb0 hvfhd
    rw [hsi] at hsdef
    exact ⟨code, _, sd, sdef, sh, inner, hcode, hb, hps, hinner', hsdef,
      Or.inr ⟨vh, vfhd, rfl, hvfhd, hsi, hfield, hne⟩⟩
  · simp at hbk
  · simp at hbk

/-- **A struct-API mutable borrow call of a non-generic wrapper calls a
wrapper**, in an accepted program of the whole subset: the wrapper's
canonical instance has no type arguments, and L9 makes its borrow the
non-generic form. -/
theorem apiCall_wrapper' {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
      ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
    {f : FunctionRef} {info : FunctionInfo} (hf : P.function? f = some info) {h : Nat}
    {fh : FunctionHandle} (hfh : info.module.functionHandles[h]? = some fh) {off : Nat}
    (hoff : ReferenceSafety.borrowFieldMutable fh.attributes = some off) {callee : FunctionRef}
    (hres : P.resolve f.module h = some callee) {cinfo : FunctionInfo}
    (hcf : P.function? callee = some cinfo) (hnp : cinfo.handle.typeParameters = []) :
    WrapperCode P callee cinfo off := by
  obtain ⟨hm, -, -, -⟩ := function?_parts hf
  have hattr := resolve_attr hwf hm hfh hres hcf (borrowFieldMutable_mem hoff) rfl (by simp)
  obtain ⟨hcm, -, hcd, hch⟩ := function?_parts hcf
  obtain ⟨⟨⟨code, b, hcode, hb, sd, sh, hop, inner, hps, hinner⟩, hret⟩, hv⟩ :=
    defApi hwf hapi hcm hcd hch hattr
  rw [hnp] at hop hinner
  have hinner' : inner = .struct sh ∨ inner = .structInstantiation sh [] := by
    rcases hinner with h1 | ⟨ts, rfl, hcan⟩
    · exact Or.inl h1
    · right
      have hl := ensureCanonical_length hcan
      simp only [List.length_nil] at hl
      rw [List.eq_nil_of_length_eq_zero hl]
  have hl9 := instructionConsistency_of_accepted (wellFormed_module hwf hcm).1 _
    (getElem?_mem_toList hcd) code hcode b (by rw [hb]; simp)
  have hb0 := (verifyModule_ok (wellFormed_module hwf hcm).1).1
  have hbk : (∃ i, b = .mutBorrowField i) ∨ ∃ i, b = .mutBorrowVariantField i := by
    rcases hop with ⟨fhi, -, rfl, -⟩ | ⟨vh, -, rfl, -⟩ |
      ⟨i, fi, -, args, rfl, hfi, -, -, -, hargs, hcan⟩ |
      ⟨i, fi, -, args, rfl, hfi, -, -, -, hargs, hcan⟩
    · exact Or.inl ⟨fhi, rfl⟩
    · exact Or.inr ⟨vh, rfl⟩
    all_goals
      exfalso
      have hl := ensureCanonical_length hcan
      obtain ⟨x, hx, n, args', -, hpos, hargs', hlen⟩ := hl9
      rw [hfi, Option.some.injEq] at hx
      subst hx
      rw [hargs, Option.some.injEq] at hargs'
      subst hargs'
      simp only [List.length_nil] at hl
      omega
  refine ⟨hattr, hv, acquires_wrapper (function_acquires hwf hcf hcode) hb hbk, ?_, hret⟩
  rcases hop with ⟨fhi, fhd, rfl, hfhd, hown, hfield⟩ | ⟨vh, vfhd, rfl, hvfhd, hsi, hfield⟩ |
    ⟨i, -, -, -, rfl, -⟩ | ⟨i, -, -, -, rfl, -⟩
  · obtain ⟨sdef, hsdef, -⟩ := verifyBounds_fieldHandle hb0 hfhd
    rw [hown] at hsdef
    exact ⟨code, _, sd, sdef, sh, inner, hcode, hb, hps, hinner', hsdef,
      Or.inl ⟨fhi, fhd, rfl, hfhd, hown, hfield⟩⟩
  · obtain ⟨sdef, hsdef, hne, -⟩ := verifyBounds_variantFieldHandle hb0 hvfhd
    rw [hsi] at hsdef
    exact ⟨code, _, sd, sdef, sh, inner, hcode, hb, hps, hinner', hsdef,
      Or.inr ⟨vh, vfhd, rfl, hvfhd, hsi, hfield, hne⟩⟩
  · simp at hbk
  · simp at hbk

end MoveBytecode.Proofs
