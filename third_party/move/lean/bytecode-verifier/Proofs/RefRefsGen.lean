-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefRefs
import Proofs.RefGeneric

/-!
# The generic instructions on references, safe

UNTRUSTED (checked by the kernel). `TestVariantGeneric` and the generic field
and variant field borrows run as their non-generic forms at the instance their
instantiation names (`refSafe_testVariantAt`, `refSafe_borrowFieldAt`,
`refSafe_borrowVariantFieldAt`): the analysis runs them as the non-generic
forms at the instantiation's handle (`Proofs.RefGeneric`), and the type
certificate types them at the instance.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

variable {P : Program} {s : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit} {view : FunctionView} {A : RefState} {stys : List SignatureToken}
  {tys : List RuntimeType} {ps : List Reference}

/-- The instance a generic form names: definition `d` of the function's
module at the type arguments of signature `sig`, a type of the program. -/
def InstanceAt (P : Program) (f : FunctionRef) (m : CompiledModule) (d sig : Nat)
    (targs : List RuntimeType) (sid : StructId) : Prop :=
  ∃ s0, P.definedStruct? f.module d = some s0 ∧ m.typeArgsWith f.typeArgs sig = some targs ∧
    targs ≠ [] ∧ sid = { s0 with typeArgs := targs } ∧ (RuntimeType.struct sid).wellFormed P = true

theorem instanceAt_of {f : FunctionRef} {m : CompiledModule} {d sig : Nat} {sid s0 : StructId}
    (hdef : P.definedStruct? f.module d = some s0) {a1 a : List RuntimeType}
    (hmatch : (match a with
      | [] => none
      | t :: ts => some (t :: ts) : Option (List RuntimeType)) = some a1)
    (htargs : m.typeArgsWith f.typeArgs sig = some a)
    (hif : (if (RuntimeType.struct { s0 with typeArgs := a1 }).wellFormed P = true then
      some { s0 with typeArgs := a1 } else none) = some sid) :
    InstanceAt P f m d sig a1 sid := by
  obtain ⟨rfl, hne⟩ := typeArgsMatch_some hmatch
  split at hif
  · next hwf =>
    simp only [Option.some.injEq] at hif
    subst hif
    exact ⟨s0, hdef, htargs, hne, rfl, hwf⟩
  · simp at hif

/-- The instance determines its type arguments and struct. -/
theorem InstanceAt.unique {f : FunctionRef} {m : CompiledModule} {d sig : Nat}
    {targs targs' : List RuntimeType} {sid sid' : StructId} (h : InstanceAt P f m d sig targs sid)
    (h' : InstanceAt P f m d sig targs' sid') : targs' = targs ∧ sid' = sid := by
  obtain ⟨s0, hdef, ht, -, rfl, -⟩ := h
  obtain ⟨s0', hdef', ht', -, rfl, -⟩ := h'
  rw [hdef, Option.some.injEq] at hdef'
  rw [ht, Option.some.injEq] at ht'
  subst hdef' ht'
  exact ⟨rfl, rfl⟩

/-- The struct of an instance, with its declaration, as `Step.localStruct`
finds it. -/
theorem InstanceAt.localStruct {f : FunctionRef} {m : CompiledModule} {d sig : Nat}
    {targs : List RuntimeType} {sid : StructId} (h : InstanceAt P f m d sig targs sid)
    {sinfo : StructInfo} (hst : P.structType? sid = some sinfo) :
    Step.localStruct P f.module d targs = .ok (sid, sinfo) := by
  obtain ⟨s0, hdef, -, -, rfl, hwf⟩ := h
  exact localStruct_ok_gen hdef hst hwf

/-- The generic forms' type arguments, as `Step.genericArgs` finds them. -/
theorem InstanceAt.genericArgs {f : FunctionRef} {m : CompiledModule} {d sig : Nat}
    {targs : List RuntimeType} {sid : StructId} (h : InstanceAt P f m d sig targs sid) :
    Step.genericArgs m f.typeArgs sig = .ok targs := by
  obtain ⟨s0, -, ht, hne, -⟩ := h
  exact genericArgs_of_typeArgsWith ht hne

/-- The declaration of an instance of a definition of the function's module. -/
theorem InstanceAt.structType {f : FunctionRef} {m : CompiledModule} {view : FunctionView}
    (hg : GenCtx P f m view) {d sig : Nat} {targs : List RuntimeType} {sid : StructId}
    (h : InstanceAt P f m d sig targs sid) {sd : StructDefinition}
    (hsd : m.structDefs[d]? = some sd) :
    ∃ id sh, P.structType? sid = some ⟨m, id, sd, sh⟩ := by
  obtain ⟨s0, hdef, -, -, rfl, hwf⟩ := h
  obtain ⟨id, name, sh, -, -, hdef', hres, hst⟩ := hg.own d sd hsd
  rw [hdef, Option.some.injEq] at hdef'
  subst hdef'
  refine ⟨id, sh, ?_⟩
  simp only [RuntimeType.wellFormed] at hwf
  split at hwf
  · next sinfo hs =>
    rw [hs]
    have hres' : P.resolveStruct { module := id, name := name, typeArgs := targs } =
        some ⟨f.module, d⟩ := hres
    simp only [Program.structType?, hres', hst, bind, Option.bind_some] at hs
    split at hs
    · exact hs.symm
    · simp at hs
  · simp at hwf

/-- The type certificate's typing of a generic field borrow. -/
theorem typeAfter_fieldBorrowGeneric {f : FunctionRef} {m : CompiledModule} {i : Nat}
    {mut_ : Bool} {before after : List RuntimeType}
    (h : typeAfter P f m (if mut_ then .mutBorrowFieldGeneric i else .immBorrowFieldGeneric i)
      before = some after) :
    ∃ x fh targs sid ftys t m' brest, m.fieldInstantiations[i]? = some x ∧
      m.fieldHandles[x.handle]? = some fh ∧ InstanceAt P f m fh.owner x.typeParameters targs sid ∧
      P.fieldTypes sid = some ftys ∧ ftys[fh.field]? = some t ∧
      before = .ref m' (.struct sid) :: brest ∧ (mut_ = true → m' = true) ∧
      after = .ref mut_ t :: brest := by
  have h' : (do
      let x ← m.fieldInstantiations[i]?
      let fh ← m.fieldHandles[x.handle]?
      let s ← (do
        let s ← P.definedStruct? f.module fh.owner
        let s := { s with typeArgs := ← (do
          match ← m.typeArgsWith f.typeArgs x.typeParameters with
          | [] => none
          | t :: ts => some (t :: ts)) }
        if (RuntimeType.struct s).wellFormed P then some s else none)
      let t ← (← P.fieldTypes s)[fh.field]?
      match before with
      | .ref mut' (.struct s') :: rest =>
        if s' = s && (mut' || !mut_) then some (.ref mut_ t :: rest) else none
      | _ => none) = some after := by
    cases mut_ <;> exact h
  clear h
  simp only [bind, Option.bind_eq_some_iff] at h'
  obtain ⟨x, hx, fh, hfh, sid, ⟨s0, hdef, targs, ⟨a, htargs, hmatch⟩, hif⟩, ftys, hftys, t, ht,
    h'⟩ := h'
  have hinst := instanceAt_of hdef hmatch htargs hif
  split at h'
  · next m' s' brest =>
    split at h'
    · next hc' =>
      simp only [Bool.and_eq_true, decide_eq_true_eq, Bool.or_eq_true,
        Bool.not_eq_true'] at hc'
      obtain ⟨rfl, hm⟩ := hc'
      refine ⟨x, fh, targs, s', ftys, t, m', brest, hx, hfh, hinst, hftys, ht, rfl,
        fun hb => ?_, (Option.some.inj h').symm⟩
      subst hb
      rcases hm with hm | hm
      · exact hm
      · cases hm
    · simp at h'
  · simp at h'

/-- The type certificate's typing of a generic variant field borrow. -/
theorem typeAfter_variantBorrowGeneric {f : FunctionRef} {m : CompiledModule} {i : Nat}
    {mut_ : Bool} {before after : List RuntimeType}
    (h : typeAfter P f m
      (if mut_ then .mutBorrowVariantFieldGeneric i else .immBorrowVariantFieldGeneric i)
      before = some after) :
    ∃ x vfh targs sid t m' brest, m.variantFieldInstantiations[i]? = some x ∧
      m.variantFieldHandles[x.handle]? = some vfh ∧
      InstanceAt P f m vfh.structIndex x.typeParameters targs sid ∧
      P.variantFieldType? sid vfh = some t ∧
      before = .ref m' (.struct sid) :: brest ∧ (mut_ = true → m' = true) ∧
      after = .ref mut_ t :: brest := by
  have h' : (do
      let x ← m.variantFieldInstantiations[i]?
      let vfh ← m.variantFieldHandles[x.handle]?
      let s ← (do
        let s ← P.definedStruct? f.module vfh.structIndex
        let s := { s with typeArgs := ← (do
          match ← m.typeArgsWith f.typeArgs x.typeParameters with
          | [] => none
          | t :: ts => some (t :: ts)) }
        if (RuntimeType.struct s).wellFormed P then some s else none)
      let t ← P.variantFieldType? s vfh
      match before with
      | .ref mut' (.struct s') :: rest =>
        if s' = s && (mut' || !mut_) then some (.ref mut_ t :: rest) else none
      | _ => none) = some after := by
    cases mut_ <;> exact h
  clear h
  simp only [bind, Option.bind_eq_some_iff] at h'
  obtain ⟨x, hx, vfh, hvfh, sid, ⟨s0, hdef, targs, ⟨a, htargs, hmatch⟩, hif⟩, t, ht, h'⟩ := h'
  have hinst := instanceAt_of hdef hmatch htargs hif
  split at h'
  · next m' s' brest =>
    split at h'
    · next hc' =>
      simp only [Bool.and_eq_true, decide_eq_true_eq, Bool.or_eq_true,
        Bool.not_eq_true'] at hc'
      obtain ⟨rfl, hm⟩ := hc'
      refine ⟨x, vfh, targs, s', t, m', brest, hx, hvfh, hinst, ht, rfl,
        fun hb => ?_, (Option.some.inj h').symm⟩
      subst hb
      rcases hm with hm | hm
      · exact hm
      · cases hm
    · simp at h'
  · simp at h'

/-- The type certificate's typing of `TestVariantGeneric`. -/
theorem typeAfter_testVariantGeneric {f : FunctionRef} {m : CompiledModule} {i : Nat}
    {before after : List RuntimeType}
    (h : typeAfter P f m (.testVariantGeneric i) before = some after) :
    ∃ x vh targs sid m0 brest, m.structVariantInstantiations[i]? = some x ∧
      m.structVariantHandles[x.handle]? = some vh ∧
      InstanceAt P f m vh.structIndex x.typeParameters targs sid ∧
      before = .ref m0 (.struct sid) :: brest ∧ after = .bool :: brest := by
  have h' := h
  dsimp only [typeAfter] at h'
  simp only [bind, Option.bind_eq_some_iff] at h'
  obtain ⟨x, hx, vh, hvh, sid, ⟨s0, hdef, targs, ⟨a, htargs, hmatch⟩, hif⟩, h'⟩ := h'
  have hinst := instanceAt_of hdef hmatch htargs hif
  split at h'
  · next m0 s' brest =>
    split at h'
    · next hss =>
      subst hss
      exact ⟨x, vh, targs, s', m0, brest, hx, hvh, hinst, rfl, (Option.some.inj h').symm⟩
    · simp at h'
  · simp at h'

/-- **`MutBorrowFieldGeneric`, `ImmBorrowFieldGeneric`.** -/
theorem refSafe_borrowFieldGeneric (hc : Certified P (modelTypes P) (modelAvail P))
    {N : NativeRel} (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {i : Nat} {mut_ : Bool}
    (hctx : TopCtx P s fr info code
      (if mut_ then .mutBorrowFieldGeneric i else .immBorrowFieldGeneric i) view A stys tys ps)
    (hi : code.code[fr.pc]? =
      some (if mut_ then .mutBorrowFieldGeneric i else .immBorrowFieldGeneric i)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code
      (if mut_ then .mutBorrowFieldGeneric i else .immBorrowFieldGeneric i)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  obtain ⟨x, fh, targs, sid, -, -, -, -, hx, hfh, hinst, -⟩ := typeAfter_fieldBorrowGeneric hafter
  refine refSafe_borrowFieldAt (h := x.handle) (mut_ := mut_) (targs := targs) hc hr hsf hact hchain hctx hi
    (by cases mut_ <;> intro h <;> cases h) (by cases mut_ <;> rfl)
    (fun _ => by cases mut_ <;> rfl) (fun A' hrun => ?_) (fun b a hafter' => ?_) ?_
  · cases mut_
    · obtain ⟨x', hx', hrun'⟩ := exec_immBorrowFieldGeneric' hrun
      rw [hx, Option.some.injEq] at hx'
      subst hx'
      exact hrun'
    · obtain ⟨x', hx', hrun'⟩ := exec_mutBorrowFieldGeneric' hrun
      rw [hx, Option.some.injEq] at hx'
      subst hx'
      exact hrun'
  · obtain ⟨x', fh', targs', sid', ftys', t', m'', brest', hx', hfh', hinst', hftys', ht', h1, hm',
      h2⟩ := typeAfter_fieldBorrowGeneric hafter'
    rw [hx, Option.some.injEq] at hx'
    subst hx'
    rw [hfh, Option.some.injEq] at hfh'
    subst hfh'
    obtain ⟨rfl, rfl⟩ := hinst.unique hinst'
    exact ⟨fh, _, ftys', t', m'', brest', hfh, fun _ hst => hinst.localStruct hst, hftys', ht',
      h1, hm', h2⟩
  · cases mut_ <;>
      simp only [Bool.false_eq_true, if_false, if_true, Step.exec, Step.need, hx, bind,
        Except.bind, pure, Except.pure, hinst.genericArgs] <;> rfl

/-- **`MutBorrowVariantFieldGeneric`, `ImmBorrowVariantFieldGeneric`.** -/
theorem refSafe_borrowVariantFieldGeneric (hc : Certified P (modelTypes P) (modelAvail P))
    {N : NativeRel} (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {i : Nat} {mut_ : Bool}
    (hctx : TopCtx P s fr info code
      (if mut_ then .mutBorrowVariantFieldGeneric i else .immBorrowVariantFieldGeneric i) view A
      stys tys ps)
    (hi : code.code[fr.pc]? =
      some (if mut_ then .mutBorrowVariantFieldGeneric i else .immBorrowVariantFieldGeneric i)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code
      (if mut_ then .mutBorrowVariantFieldGeneric i else .immBorrowVariantFieldGeneric i)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  obtain ⟨x, vfh, targs, sid, -, -, -, hx, hvfh, hinst, -⟩ :=
    typeAfter_variantBorrowGeneric hafter
  refine refSafe_borrowVariantFieldAt (h := x.handle) (mut_ := mut_) (targs := targs) hc hr hsf hact hchain hctx
    hi (by cases mut_ <;> intro h <;> cases h) (by cases mut_ <;> rfl)
    (fun _ => by cases mut_ <;> rfl) (fun A' hrun => ?_) (fun b a hafter' => ?_) ?_
  · cases mut_
    · obtain ⟨x', hx', hrun'⟩ := exec_immBorrowVariantFieldGeneric' hrun
      rw [hx, Option.some.injEq] at hx'
      subst hx'
      exact hrun'
    · obtain ⟨x', hx', hrun'⟩ := exec_mutBorrowVariantFieldGeneric' hrun
      rw [hx, Option.some.injEq] at hx'
      subst hx'
      exact hrun'
  · obtain ⟨x', vfh', targs', sid', t', m'', brest', hx', hvfh', hinst', ht', h1, hm', h2⟩ :=
      typeAfter_variantBorrowGeneric hafter'
    rw [hx, Option.some.injEq] at hx'
    subst hx'
    rw [hvfh, Option.some.injEq] at hvfh'
    subst hvfh'
    obtain ⟨rfl, rfl⟩ := hinst.unique hinst'
    exact ⟨vfh, _, t', m'', brest', hvfh, fun _ hst => hinst.localStruct hst, ht', h1, hm', h2⟩
  · cases mut_ <;>
      simp only [Bool.false_eq_true, if_false, if_true, Step.exec, Step.need, hx, bind,
        Except.bind, pure, Except.pure, hinst.genericArgs] <;> rfl

/-- **`TestVariantGeneric`.** -/
theorem refSafe_testVariantGeneric (hc : Certified P (modelTypes P) (modelAvail P))
    {N : NativeRel} (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {i : Nat}
    (hctx : TopCtx P s fr info code (.testVariantGeneric i) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.testVariantGeneric i)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (.testVariantGeneric i)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  obtain ⟨x, vh, targs, sid, -, -, hx, hvh, hinst, -⟩ := typeAfter_testVariantGeneric hafter
  have hg := hctx.cert.setting.agrees
  have hb := hctx.cert.setting.bounds
  refine refSafe_testVariantAt (h := x.handle) (targs := targs) hc hr hsf hact hchain hctx hi
    (by intro h; cases h) rfl (fun _ => rfl) (fun A' hrun => ?_) (fun b a hafter' => ?_) ?_
  · obtain ⟨x', hx', hrun'⟩ := exec_testVariantGeneric' hrun
    rw [hx, Option.some.injEq] at hx'
    subst hx'
    exact hrun'
  · obtain ⟨x', vh', targs', sid', m0, brest, hx', hvh', hinst', h1, h2⟩ :=
      typeAfter_testVariantGeneric hafter'
    rw [hx, Option.some.injEq] at hx'
    subst hx'
    rw [hvh, Option.some.injEq] at hvh'
    subst hvh'
    obtain ⟨rfl, rfl⟩ := hinst.unique hinst'
    obtain ⟨sd, hsd, hvar⟩ := verifyBounds_structVariantHandle hb hvh
    obtain ⟨id, sh, hst⟩ := hinst.structType hg hsd
    obtain ⟨variants, hfi, hv⟩ : ∃ variants, sd.fieldInformation = .declaredVariants variants ∧
        vh.variant < variants.length := by
      cases hfi : sd.fieldInformation with
      | declaredVariants vs => rw [hfi] at hvar; exact ⟨vs, rfl, hvar⟩
      | declared _ => rw [hfi] at hvar; simp [variantCount] at hvar
      | native => rw [hfi] at hvar; simp [variantCount] at hvar
    exact ⟨vh, _, _, variants, m0, brest, hvh, hst, hinst.localStruct hst, hfi, hv, h1, h2⟩
  · simp only [Step.exec, Step.need, hx, bind, Except.bind, pure, Except.pure, hinst.genericArgs]

end MoveBytecode.Proofs
