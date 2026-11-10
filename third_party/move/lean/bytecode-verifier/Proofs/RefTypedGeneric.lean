-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefTypedStep
import Proofs.RefGeneric

/-!
# The analysis's states stay well typed on generic forms

UNTRUSTED (checked by the kernel). `Typed` relates the reference analysis's
abstract state to the type checker's stack only through the tokens' lengths
and reference kinds (`refKind`). The analysis runs each generic form as its
non-generic counterpart (`Proofs/RefGeneric.lean`); the type checker pushes
the counterpart's tokens instantiated by the form's type arguments, which are
never references, so each pushed token has the reference kind the
counterpart's would. Calls and global borrows are `typed_exec_M2`'s.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

/-- The signature of type arguments a generic form names, if any. -/
def instSig? (m : CompiledModule) : Bytecode → Option SignatureIndex
  | .callGeneric i | .packClosureGeneric i _ => (m.functionInstantiations[i]?).map (·.typeParameters)
  | .packGeneric i | .unpackGeneric i | .mutBorrowGlobalGeneric i | .immBorrowGlobalGeneric i
  | .existsGeneric i | .moveFromGeneric i | .moveToGeneric i =>
    (m.structDefInstantiations[i]?).map (·.typeParameters)
  | .packVariantGeneric i | .unpackVariantGeneric i | .testVariantGeneric i =>
    (m.structVariantInstantiations[i]?).map (·.typeParameters)
  | .mutBorrowFieldGeneric i | .immBorrowFieldGeneric i =>
    (m.fieldInstantiations[i]?).map (·.typeParameters)
  | .mutBorrowVariantFieldGeneric i | .immBorrowVariantFieldGeneric i =>
    (m.variantFieldInstantiations[i]?).map (·.typeParameters)
  | _ => none

/-- The type arguments an instruction names are no references. -/
def InstNonRef (m : CompiledModule) (instr : Bytecode) : Prop :=
  ∀ s, instSig? m instr = some s → ∀ sig, m.signatures[s]? = some sig → ∀ t ∈ sig, refKind t = none

theorem refKind_materialize (h : StructHandleIndex) (tys : List SignatureToken) :
    refKind (materializeType h tys) = none := by
  unfold materializeType; split <;> rfl

/-- Instantiating by type arguments that are no references keeps a token's
reference kind. -/
theorem refKind_instantiate {tys : List SignatureToken} (htys : ∀ x ∈ tys, refKind x = none)
    {t q : SignatureToken} (h : instantiate tys t = .ok q) : refKind q = refKind t := by
  unfold instantiate at h
  split at h
  · simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    rfl
  · cases t with
    | reference t' =>
      simp only [instantiateIn, bind_ok, pure, Except.pure, Except.ok.injEq] at h
      obtain ⟨_, -, rfl⟩ := h
      rfl
    | mutableReference t' =>
      simp only [instantiateIn, bind_ok, pure, Except.pure, Except.ok.injEq] at h
      obtain ⟨_, -, rfl⟩ := h
      rfl
    | typeParameter i =>
      simp only [instantiateIn, lookup_ok, List.getElem?_toArray] at h
      rw [htys q (List.mem_of_getElem? h)]
      rfl
    | vector e =>
      simp only [instantiateIn, bind_ok, pure, Except.pure, Except.ok.injEq] at h
      obtain ⟨_, -, rfl⟩ := h
      rfl
    | function ps rs a =>
      simp only [instantiateIn, bind_ok, pure, Except.pure, Except.ok.injEq] at h
      obtain ⟨_, -, _, -, rfl⟩ := h
      rfl
    | structInstantiation hh args =>
      simp only [instantiateIn, bind_ok, pure, Except.pure, Except.ok.injEq] at h
      obtain ⟨_, -, rfl⟩ := h
      rfl
    | _ =>
      simp only [instantiateIn, pure, Except.pure, Except.ok.injEq] at h
      subst h
      rfl

/-- Instantiated field tokens are no references. -/
theorem fieldsGeneric_value {P : Program} {f : FunctionRef} {m : CompiledModule}
    {view : FunctionView} (ha : GenCtx P f m view) {d : Nat} {sd : StructDefinition}
    (hsd : m.structDefs[d]? = some sd) {variant : Option Nat} {tys fs0 fs : List SignatureToken}
    (htys : ∀ x ∈ tys, refKind x = none) (h0 : fieldsSignature sd variant = .ok fs0)
    (h1 : fs0.mapM (instantiate tys) = .ok fs) : ∀ t ∈ fs, refKind t = none := by
  intro t ht
  obtain ⟨-, hget⟩ := mapM_ok_getElem' h1
  obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 ht
  obtain ⟨t0, ht0, hinst⟩ := hget k t hk
  rw [refKind_instantiate htys hinst]
  exact fieldsSignature_value ha hsd h0 t0 (List.mem_of_getElem? ht0)

set_option maxHeartbeats 4000000 in
/-- **Each generic form but a call or a global borrow keeps the analysis's
states well typed.** -/
theorem typed_exec_generic {P : Program} {f : FunctionRef} {m : CompiledModule}
    {view : FunctionView} {ltys stys stys' : List SignatureToken}
    {avail : LocalIndex → Availability} {st st' : RefState} {instr : Bytecode}
    (ha : GenCtx P f m view) (hb : verifyBounds m = .ok ()) (hgen : genericForm instr = true)
    (hcall : ∀ i, instr ≠ .callGeneric i) (hg1 : ∀ i, instr ≠ .mutBorrowGlobalGeneric i)
    (hg2 : ∀ i, instr ≠ .immBorrowGlobalGeneric i) (hops : InstNonRef m instr)
    (ht : Typed ltys stys avail st) (hty : typeInstr m view stys instr = .ok stys')
    (hx : (executeInner m view instr).run st = .ok ((), st')) :
    Typed ltys stys' (availabilityAfter instr avail) st' := by
  cases instr
  all_goals try (simp [genericForm] at hgen; done)
  case callGeneric i => exact absurd rfl (hcall i)
  case mutBorrowGlobalGeneric i => exact absurd rfl (hg1 i)
  case immBorrowGlobalGeneric i => exact absurd rfl (hg2 i)
  case packGeneric i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨d, sd, tys⟩, hparts, hty⟩ := hty
    obtain ⟨si, hsi, rfl, hsd, -⟩ := structInstParts_ok.1 hparts
    dsimp only at hty
    obtain ⟨fs0, fs, args, rest, h0, h1, rfl, hlen, -, rfl⟩ := typePackGeneric_ok hty
    obtain ⟨si', hsi', hx'⟩ := exec_packGeneric' hx
    rw [hsi, Option.some.injEq] at hsi'
    subst hsi'
    obtain ⟨sd', hsd', hvs⟩ := exec_pack hx'
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    rw [fieldCount_of_generic h0 h1, ← hlen] at hvs
    exact ht.ofValueStep (P := [materializeType sd.structHandle tys]) hvs rfl
      (by simp [refKind_materialize]) (by simp)
  case unpackGeneric i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨d, sd, tys⟩, hparts, hty⟩ := hty
    obtain ⟨si, hsi, rfl, hsd, htys⟩ := structInstParts_ok.1 hparts
    dsimp only at hty
    obtain ⟨fs0, fs, rest, rfl, h0, h1, rfl⟩ := typeUnpackGeneric_ok hty
    obtain ⟨si', hsi', hx'⟩ := exec_unpackGeneric' hx
    rw [hsi, Option.some.injEq] at hsi'
    subst hsi'
    obtain ⟨sd', hsd', hvs⟩ := exec_unpack hx'
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    rw [fieldCount_of_generic h0 h1] at hvs
    have hnr := hops si.typeParameters (by simp [instSig?, hsi]) tys htys
    exact ht.ofValueStep (P := fs.reverse) hvs (by simp)
      (fun t htm => fieldsGeneric_value ha hsd hnr h0 h1 t (List.mem_reverse.1 htm)) (by simp)
  case packVariantGeneric i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨vh, sd, tys⟩, hparts, hty⟩ := hty
    obtain ⟨vi, hvi, hvh, hsd, -⟩ := variantInstParts_ok.1 hparts
    dsimp only at hty
    obtain ⟨fs0, fs, args, rest, h0, h1, rfl, hlen, -, rfl⟩ := typePackGeneric_ok hty
    obtain ⟨vi', hvi', hx'⟩ := exec_packVariantGeneric' hx
    rw [hvi, Option.some.injEq] at hvi'
    subst hvi'
    obtain ⟨vh', sd', hvh', hsd', hvs⟩ := exec_packVariant hx'
    rw [hvh, Option.some.injEq] at hvh'
    subst hvh'
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    rw [fieldCount_of_generic h0 h1, ← hlen] at hvs
    exact ht.ofValueStep (P := [materializeType sd.structHandle tys]) hvs rfl
      (by simp [refKind_materialize]) (by simp)
  case unpackVariantGeneric i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨vh, sd, tys⟩, hparts, hty⟩ := hty
    obtain ⟨vi, hvi, hvh, hsd, htys⟩ := variantInstParts_ok.1 hparts
    dsimp only at hty
    obtain ⟨fs0, fs, rest, rfl, h0, h1, rfl⟩ := typeUnpackGeneric_ok hty
    obtain ⟨vi', hvi', hx'⟩ := exec_unpackVariantGeneric' hx
    rw [hvi, Option.some.injEq] at hvi'
    subst hvi'
    obtain ⟨vh', sd', hvh', hsd', hvs⟩ := exec_unpackVariant hx'
    rw [hvh, Option.some.injEq] at hvh'
    subst hvh'
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    rw [fieldCount_of_generic h0 h1] at hvs
    have hnr := hops vi.typeParameters (by simp [instSig?, hvi]) tys htys
    exact ht.ofValueStep (P := fs.reverse) hvs (by simp)
      (fun t htm => fieldsGeneric_value ha hsd hnr h0 h1 t (List.mem_reverse.1 htm)) (by simp)
  case testVariantGeneric i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨vh, sd, tys⟩, -, ⟨arg, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    have hst : stys' = .bool :: rest := by
      split at hty
      · split at hty
        · simp only [pure, Except.pure, Except.ok.injEq] at hty; exact hty.symm
        · simp [reject] at hty
      · split at hty
        · simp only [pure, Except.pure, Except.ok.injEq] at hty; exact hty.symm
        · simp [reject] at hty
      · simp [reject] at hty
    subst hst
    obtain ⟨vi, -, hx'⟩ := exec_testVariantGeneric' hx
    obtain ⟨id, r, g, hs, hrel, rfl⟩ := exec_testVariant hx'
    exact ht.releasePush hs hrel hp rfl rfl rfl rfl
  case mutBorrowFieldGeneric i | immBorrowFieldGeneric i =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨fi, hfi, tys, -, hty⟩ := hty
    obtain ⟨op, rest, fh, sd, rfl, -, hfh, hsd, -, hcase⟩ := fieldBorrowGen_ok hty
    obtain ⟨sd', hsd', hlt⟩ := verifyBounds_fieldHandle hb hfh
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    rcases hcase with ⟨fd, q, -, -, rfl⟩ | ⟨hnone, -⟩
    · first
        | obtain ⟨fi', hfi', hx'⟩ := exec_mutBorrowFieldGeneric' hx
          rw [hfi, Option.some.injEq] at hfi'
          subst hfi'
          obtain ⟨fh', hfh', id, r, g1, g2, g3, hs, -, hnew, hfac, hrel, rfl⟩ :=
            exec_mutBorrowField hx'
          exact ht.borrowFrom hs rfl hnew (newFactor hnew hfac ht.wf ht.back) hrel
            (refKind_refTo _ _) rfl rfl rfl
        | obtain ⟨fi', hfi', hx'⟩ := exec_immBorrowFieldGeneric' hx
          rw [hfi, Option.some.injEq] at hfi'
          subst hfi'
          obtain ⟨fh', hfh', id, r, g1, g2, g3, hs, -, hnew, hfac, hrel, rfl⟩ :=
            exec_immBorrowField hx'
          exact ht.borrowFrom hs rfl hnew (newFactor hnew hfac ht.wf ht.back) hrel
            (refKind_refTo _ _) rfl rfl rfl
    · exfalso
      cases hfi' : sd.fieldInformation with
      | declared fields =>
        rw [hfi'] at hlt hnone
        simp only [fieldCount] at hlt
        simp [Model.structFields, List.getElem?_eq_getElem hlt] at hnone
      | declaredVariants _ => rw [hfi'] at hlt; simp [fieldCount] at hlt
      | native => rw [hfi'] at hlt; simp [fieldCount] at hlt
  case mutBorrowVariantFieldGeneric i | immBorrowVariantFieldGeneric i =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨fi, hfi, tys, -, hty⟩ := hty
    obtain ⟨op, rest, fh, sd, rfl, -, hfh, hsd, -, -, ft, rfl, -⟩ := variantBorrowGen_full hty
    first
      | obtain ⟨fi', hfi', hx'⟩ := exec_mutBorrowVariantFieldGeneric' hx
        obtain ⟨fh', hfh', id, r, g1, g2, g3, hs, -, hnew, hfac, hrel, rfl⟩ :=
          exec_mutBorrowVariantField hx'
        exact ht.borrowFrom hs rfl hnew (newFactor hnew hfac ht.wf ht.back) hrel
          (refKind_refTo _ _) rfl rfl rfl
      | obtain ⟨fi', hfi', hx'⟩ := exec_immBorrowVariantFieldGeneric' hx
        obtain ⟨fh', hfh', id, r, g1, g2, g3, hs, -, hnew, hfac, hrel, rfl⟩ :=
          exec_immBorrowVariantField hx'
        exact ht.borrowFrom hs rfl hnew (newFactor hnew hfac ht.wf ht.back) hrel
          (refKind_refTo _ _) rfl rfl rfl
  case existsGeneric i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨d, sd, tys⟩, -, a, -, u, -, ⟨op, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    obtain ⟨u', hop, hty⟩ := hty
    rw [rejectIf_ok] at hop
    simp only [bne_eq_false_iff_eq] at hop
    subst hop
    simp only [pure, Except.pure, Except.ok.injEq] at hty
    subst hty
    rw [exec_existsGeneric' 0] at hx
    exact ht.retype' (exec_nothing (by simp) hx) hp rfl rfl
  case moveFromGeneric i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨d, sd, tys⟩, -, a, -, u, -, ⟨op, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    obtain ⟨u', -, hty⟩ := hty
    simp only [pure, Except.pure, Except.ok.injEq] at hty
    subst hty
    obtain ⟨si, -, hx'⟩ := exec_moveFromGeneric' hx
    obtain ⟨-, hvs⟩ := exec_moveFrom hx'
    exact ht.ofValueStep (P := [materializeType sd.structHandle tys]) hvs rfl
      (by simp [refKind_materialize]) (by simp [hp])
  case moveToGeneric i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨d, sd, tys⟩, -, a, -, u, -, ⟨keyOp, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    obtain ⟨⟨signerOp, rest'⟩, hp', hty⟩ := hty
    rw [popType_ok] at hp'
    try dsimp only at hty
    obtain ⟨u', -, u'', -, hty⟩ := hty
    simp only [pure, Except.pure, Except.ok.injEq] at hty
    subst hty hp'
    rw [exec_moveToGeneric' 0] at hx
    obtain ⟨v, r, hs, hcase⟩ := exec_moveTo hx
    have h1 := ht.popValue hs hp
    rcases hcase with ⟨rfl, rfl⟩ | ⟨id, g, rfl, hrel, rfl⟩
    · exact (h1.popValue (rest := r) rfl rfl).ext rfl rfl rfl
    · exact (h1.popRelease (id := id) (rest := r) (g := g) rfl hrel rfl).ext rfl rfl rfl

end MoveBytecode.Proofs
