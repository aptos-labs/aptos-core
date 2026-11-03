-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.SoundnessAll
import Proofs.RefGlobalsGen
import Proofs.SoundnessGen
import Proofs.RefNatives

/-!
# D1 on generic code: the instructions

UNTRUSTED (checked by the kernel). On the whole supported subset, generic
functions, structs and enums included, every instruction but a call of a
struct-API mutable borrow is safe from the reference invariant and keeps it
(`exec_safe_refGen`). The generic forms run as their non-generic forms at the
instance their instantiation names: `CallGeneric` as a call at its type
arguments (`refRuns_callWith`), the generic struct and enum forms by value as
steps on values, and the generic forms on references and global storage as
the non-generic ones at the instance (`Proofs.RefRefsGen`,
`Proofs.RefGlobalsGen`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit} {view : FunctionView} {A : RefState} {stys : List SignatureToken}
  {tys : List RuntimeType} {ps : List Reference}

/-! ## Steps on values -/

/-- **The analysis's step on generic values**: it pops and pushes values. -/
theorem exec_valueStepGeneric {m : CompiledModule} {view : FunctionView} {A A' : RefState}
    {instr : Bytecode} (hv : instr.ValueOnlyGeneric = true)
    (hx : (ReferenceSafety.executeInner m view instr).run A = .ok ((), A')) :
    ∃ j k, ValueStep j k A A' := by
  cases instr <;> simp only [Bytecode.ValueOnlyGeneric] at hv <;> try (exact absurd hv (by decide))
  case packGeneric i =>
    obtain ⟨si, -, hx⟩ := exec_packGeneric' hx
    obtain ⟨sd, -, h⟩ := exec_pack hx
    exact ⟨_, _, h⟩
  case unpackGeneric i =>
    obtain ⟨si, -, hx⟩ := exec_unpackGeneric' hx
    obtain ⟨sd, -, h⟩ := exec_unpack hx
    exact ⟨_, _, h⟩
  case packVariantGeneric i =>
    obtain ⟨vi, -, hx⟩ := exec_packVariantGeneric' hx
    obtain ⟨vh, sd, -, -, h⟩ := exec_packVariant hx
    exact ⟨_, _, h⟩
  case unpackVariantGeneric i =>
    obtain ⟨vi, -, hx⟩ := exec_unpackVariantGeneric' hx
    obtain ⟨vh, sd, -, -, h⟩ := exec_unpackVariant hx
    exact ⟨_, _, h⟩
  case existsGeneric i =>
    rw [exec_existsGeneric' 0] at hx
    rw [exec_nothing (by simp) hx]
    exact ⟨0, 0, ValueStep.refl _⟩
  case packClosure hd mask =>
    obtain ⟨_, _, -, -, hvs⟩ := exec_packClosure hx
    exact ⟨_, _, hvs⟩
  case packClosureGeneric i mask =>
    obtain ⟨_, _, _, -, -, -, hvs⟩ := exec_packClosureGeneric hx
    exact ⟨_, _, hvs⟩

/-- **A step of the shape of a step on values keeps the reference
invariant.** -/
theorem refInv_valueShape (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hargs : ∀ x ∈ fr.function.typeArgs, x.isValue = true)
    (hshape : ValueShape P s s' fr rest code)
    {A A' : RefState} (hA : modelRef P fr.function fr.pc = some A) {j k : Nat}
    (hvs : ValueStep j k A A')
    (hsucc : ∀ pc' ∈ successors code.code fr.pc, ∃ A'', modelRef P fr.function pc' = some A'' ∧
      RefLe A' A'') (hwf : ∀ v ∈ fr.stack, v.wellFormed P = true) : RefInvariant P s' := by
  obtain ⟨⟨fr', new, j', hs'f, hid, hfun, hloc, hstk, hpop, hpc, hca, hnew⟩, hinv, hnext,
    hstor, hheld, hact⟩ := hshape
  obtain ⟨ps, ⟨L, hd, hrefs⟩, hchain⟩ := hr.topOk hsf hA
  have hd' := hd.valueStep hvs hid hloc hstk (hpop hargs)
  have hheldIds := hvs.heldIds
  have hok' : FrameOk P s' fr' ps A' := ⟨L, by rw [gsidOf_frames hsf hs'f hfun hca]; exact hd',
    fun x r hx hxr => by
      rw [hheldIds] at hx
      exact (hrefs x r hx hxr).same hsf hs'f hid hfun hloc hinv hheld hstor⟩
  obtain ⟨A'', hA'', hle⟩ := hsucc fr'.pc hpc
  exact hr.top hsf hs'f hid hfun hca hact (Nat.le_of_eq hnext.symm)
    (fun r hr' => hr.storage r (by rw [← hstor]; exact hr'))
    hheld (es := []) (by rw [hinv]; rfl) (storageKept_of_eq hstor) allowedBy_nil hchain
    ⟨A'', by rw [hfun]; exact hA'', hok'.le hle⟩
    (RefsFrom.next hstk hloc fun v hv' => .of_not_ref
      (hnew hargs (fun w hw => hwf w (List.mem_of_mem_take hw)) v hv'))

/-! ## `TestVariant` on the whole subset -/

/-- **`TestVariant`**, of a non-generic enum (L9), in code of the whole
subset. -/
theorem refSafe_testVariantG (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {h : Nat}
    (hl9 : info.module.variantTypeParamCount h = some 0)
    (hctx : TopCtx P s fr info code (.testVariant h) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.testVariant h)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (.testVariant h)) := by
  have hg := hctx.cert.setting.agrees
  have hb := hctx.cert.setting.bounds
  refine refSafe_testVariantAt (targs := []) hc hr hsf hact hchain hctx hi (by intro h; cases h)
    rfl (fun _ => rfl) (fun _ hx => hx) (fun before after hafter => ?_) rfl
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨vh, hvh, sid, hdef, hta⟩ := hta
  split at hta
  · next m0 s' brest =>
    split at hta
    · next hss =>
      subst hss
      simp only [Option.some.injEq] at hta
      subst hta
      obtain ⟨sd, hsd, hvar⟩ := verifyBounds_structVariantHandle hb hvh
      have hcount : info.module.structTypeParamCount vh.structIndex = some 0 := by
        simpa [CompiledModule.variantTypeParamCount, hvh] using hl9
      obtain ⟨s₂, sh, id, hdef₂, -, -, -, hst₂, -⟩ := hg.closedStruct hsd hcount
      rw [hdef, Option.some.injEq] at hdef₂
      subst hdef₂
      obtain ⟨variants, hfi, hv⟩ : ∃ variants, sd.fieldInformation = .declaredVariants variants ∧
          vh.variant < variants.length := by
        cases hfi : sd.fieldInformation with
        | declaredVariants vs => rw [hfi] at hvar; exact ⟨vs, rfl, hvar⟩
        | declared _ => rw [hfi] at hvar; simp [variantCount] at hvar
        | native => rw [hfi] at hvar; simp [variantCount] at hvar
      exact ⟨vh, _, _, variants, m0, brest, hvh, hst₂, localStruct_ok hdef hst₂, hfi, hv, rfl,
        rfl⟩
    · simp at hta
  · simp at hta

section Main

variable (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)

/-! ## `CallGeneric` -/

include hwf hs in
/-- **`CallGeneric`** keeps the reference invariant, into the callee instance
or, for a native, back in the caller. -/
theorem refRuns_callGeneric (hapi : P.ApiSigs) (hc : Certified P (modelTypes P) (modelAvail P))
    {N : NativeRel} (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code) {i : Nat}
    (hctx : TopCtx P s fr info code (.callGeneric i) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.callGeneric i))
    (hattrH : ∀ fi fh, info.module.functionInstantiations[i]? = some fi →
      info.module.functionHandles[fi.handle]? = some fh →
      ReferenceSafety.borrowFieldMutable fh.attributes = none)
    (hnm : NoModeledCall P fr.function.module info.module (.callGeneric i)) :
    ExecRuns P N (RefInvariant P) (Step.exec P s fr rest info code (.callGeneric i)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨x, hx, fh, hfh, tys', ⟨targs, htargs, hmatch⟩, hta⟩ := hta
  obtain ⟨rfl, hne⟩ := typeArgsMatch_some hmatch
  obtain ⟨callee, cinfo, fh', hres, hvis, hcf, hfh', hclen, hcons, hsig⟩ :=
    hc.callsGeneric fr.function info code hact.fn hact.body fr.pc i x hact.reachable hi hx
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  split at hta
  · simp at hta
  next hwfT =>
  have hwfT' : RuntimeType.argumentsWellFormed P targs fh.typeParameters = true := by
    simpa using hwfT
  have hlen := argumentsWellFormed_length hwfT'
  simp only [Option.bind_eq_some_iff] at hta
  obtain ⟨params, hparams, returns, hreturns, hta⟩ := hta
  split at hta
  · next hpre =>
    obtain ⟨hpar, hret⟩ := hsig targs hlen
    have hexec : Step.exec P s fr rest info code (.callGeneric i) =
        Step.call P s fr rest info x.handle targs := by
      simp only [Step.exec, Step.need, hx, pure, Except.pure,
        genericArgs_of_typeArgsWith htargs hne, bind, Except.bind]
    rw [hexec]
    exact refRuns_callWith hwf hs hapi hc hr hsf hact hctx hi (Or.inr ⟨i, x, rfl, hx, rfl⟩)
      (by intro h; cases h) rfl
      (fun A' hrun => by
        obtain ⟨x', hx', h'⟩ := exec_callGeneric' hrun
        rw [hx, Option.some.injEq] at hx'
        subst hx'
        exact h')
      (hattrH x fh hx hfh) hres hvis hcf hfh hlen.symm (hclen.trans hlen.symm)
      (argumentsWellFormed_weaken hwfT' hclen hcons) (hpar.trans hparams) (hret.trans hreturns)
      hparams hreturns hpre (hnm _ _ hx hres)
  · simp at hta

/-! ## Calls of the modeled natives -/

include hwf in
/-- **`Call` of a modeled native** is safe and keeps the invariant. -/
theorem refSafe_callModeled (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) (hinv : InvRefs P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {h : FunctionHandleIndex}
    (hctx : TopCtx P s fr info code (.call h) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.call h))
    (hattrH : ∀ fh, info.module.functionHandles[h]? = some fh →
      ReferenceSafety.borrowFieldMutable fh.attributes = none)
    (hnm : ¬ NoModeledCall P fr.function.module info.module (.call h)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (.call h)) := by
  obtain ⟨callee, cinfo, hres, hvis, hcf, htp, hpar, hret⟩ :=
    hc.calls fr.function info code hact.fn hact.body fr.pc h hact.reachable hi
  obtain ⟨k, hk⟩ : ∃ k, P.nativeShape? callee = some k := by
    cases hk : P.nativeShape? callee with
    | some k => exact ⟨k, rfl⟩
    | none =>
      exact absurd (fun c hc' => by rw [hres, Option.some.injEq] at hc'; subst hc'; exact hk) hnm
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨cfh, hcfh, hta⟩ := hta
  split at hta
  · simp at hta
  rename_i hcne
  have hcfh0 : cfh.typeParameters = [] := by simpa using hcne
  simp only [Option.bind_eq_some_iff] at hta
  obtain ⟨params, hparams, returns, hreturns, hta⟩ := hta
  split at hta
  case isFalse => simp at hta
  rename_i hpre
  have hae : returns.reverse ++ before.drop params.length = after := by simpa using hta
  subst hae
  have hceq := withTypeArgs_self (resolve_typeArgs hres)
  exact refSafe_nativeCallWith (τ := []) hwf hc hN hinv hsf hact hchain hctx hi (Or.inl rfl)
    (by intro h; cases h) rfl (fun _ => rfl) (fun _ hx => hx) (hattrH cfh hcfh) hres hvis hcf hcfh
    (by rw [hcfh0]; rfl) (by rw [htp]; rfl) (by rw [htp]; rfl) (by rw [hceq]; exact hpar.trans hparams)
    (by rw [hceq]; exact hret.trans hreturns) hparams hreturns hpre hafter hk

include hwf in
/-- **`CallGeneric` of a modeled native** is safe and keeps the invariant. -/
theorem refSafe_callGenericModeled (hc : Certified P (modelTypes P) (modelAvail P))
    {N : NativeRel} (hN : NativesRespect P N) (hinv : InvRefs P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {i : Nat}
    (hctx : TopCtx P s fr info code (.callGeneric i) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.callGeneric i))
    (hattrH : ∀ fi fh, info.module.functionInstantiations[i]? = some fi →
      info.module.functionHandles[fi.handle]? = some fh →
      ReferenceSafety.borrowFieldMutable fh.attributes = none)
    (hnm : ¬ NoModeledCall P fr.function.module info.module (.callGeneric i)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (.callGeneric i)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨x, hx, fh, hfh, tys', ⟨targs, htargs, hmatch⟩, hta⟩ := hta
  obtain ⟨rfl, hne⟩ := typeArgsMatch_some hmatch
  obtain ⟨callee, cinfo, fh', hres, hvis, hcf, hfh', hclen, hcons, hsig⟩ :=
    hc.callsGeneric fr.function info code hact.fn hact.body fr.pc i x hact.reachable hi hx
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  obtain ⟨k, hk⟩ : ∃ k, P.nativeShape? callee = some k := by
    cases hk : P.nativeShape? callee with
    | some k => exact ⟨k, rfl⟩
    | none =>
      exact absurd (fun x' c hx' hc' => by
        rw [hx, Option.some.injEq] at hx'
        subst hx'
        rw [hres, Option.some.injEq] at hc'
        subst hc'
        exact hk) hnm
  split at hta
  · simp at hta
  next hwfT =>
  have hwfT' : RuntimeType.argumentsWellFormed P targs fh.typeParameters = true := by
    simpa using hwfT
  have hlen := argumentsWellFormed_length hwfT'
  simp only [Option.bind_eq_some_iff] at hta
  obtain ⟨params, hparams, returns, hreturns, hta⟩ := hta
  split at hta
  · next hpre =>
    obtain ⟨hpar, hret⟩ := hsig targs hlen
    have hexec : Step.exec P s fr rest info code (.callGeneric i) =
        Step.call P s fr rest info x.handle targs := by
      simp only [Step.exec, Step.need, hx, pure, Except.pure,
        genericArgs_of_typeArgsWith htargs hne, bind, Except.bind]
    rw [hexec]
    have hae : returns.reverse ++ before.drop params.length = after := by simpa using hta
    subst hae
    exact refSafe_nativeCallWith hwf hc hN hinv hsf hact hchain hctx hi (Or.inr ⟨i, x, rfl, hx, rfl⟩)
      (by intro h; cases h) rfl (fun _ => rfl)
      (fun A' hrun => by
        obtain ⟨x', hx', h'⟩ := exec_callGeneric' hrun
        rw [hx, Option.some.injEq] at hx'
        subst hx'
        exact h')
      (hattrH x fh hx hfh) hres hvis hcf hfh hlen.symm (hclen.trans hlen.symm)
      (argumentsWellFormed_weaken hwfT' hclen hcons) (hpar.trans hparams) (hret.trans hreturns)
      hparams hreturns hpre hafter hk
  · simp at hta

/-! ## Every instruction but a struct-API call -/

variable (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)

include hwf hs hapi in
/-- **Every instruction but a call of a struct-API mutable borrow**, generic
forms included, is safe from the reference invariant and keeps it. -/
theorem exec_safe_refGen (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) (hinv : InvRefs P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {instr : Bytecode}
    (hi : code.code[fr.pc]? = some instr)
    (hattr : ∀ h, instr = .call h → ∀ fh, info.module.functionHandles[h]? = some fh →
      ReferenceSafety.borrowFieldMutable fh.attributes = none)
    (hattrG : ∀ i, instr = .callGeneric i → ∀ fi fh,
      info.module.functionInstantiations[i]? = some fi →
      info.module.functionHandles[fi.handle]? = some fh →
      ReferenceSafety.borrowFieldMutable fh.attributes = none) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code instr) := by
  have hr := hinv.2
  have hsigs := apiSigs_of hwf hapi
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtxGen hwf hs hsigs hinv hsf hact hi
  have hparts := function?_parts hact.fn
  have hmod : info.module.Supported := hs.1 _ (module_mem hparts.1)
  obtain ⟨fh0, -, -, hcodes⟩ := hmod.2.2.2.2 _ (getElem?_mem_toList hparts.2.2.1) code hact.body
  have hsub := hcodes instr (getElem?_mem_toList hi)
  have hsub1 := hsub.1
  have hl9 : instr.OperandsConsistent info.module :=
    instructionConsistency_of_accepted (wellFormed_module hwf hparts.1).1 _
      (getElem?_mem_toList hparts.2.2.1) code hact.body instr (getElem?_mem_toList hi)
  have hvecHead : ∀ sig k, instr = .vecUnpack sig k →
      ∀ ty, (info.module.signatures[sig]?.bind (·.head?)) = some ty → refKind ty = none := by
    intro sig k e
    subst e
    obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1
      (hsub.2 (List.replicate fh0.typeParameters.length .bool) (by simp))
    exact vectorElemTypeWith_head (by rw [hu]; rfl)
  have hcore : (instr.inCore || instr.inDataByValue) = true → instr ≠ .eq → instr ≠ .neq →
      NoModeledCall P fr.function.module info.module instr →
      ExecSafe P N (Inv P (modelTypes P) (modelAvail P)) (Step.exec P s fr rest info code instr) :=
    fun hb h1 h2 hnm => exec_safe hc hN hact hchain hi hb (fun h => by
      rcases h with h | h
      · exact absurd h h1
      · exact absurd h h2) hnm
  have hvalue : instr.ValueOnly = true →
      ExecRuns P N (RefInvariant P) (Step.exec P s fr rest info code instr) := fun hv =>
    execRuns_of (exec_notNative (fun h e => by subst e; simp [Bytecode.ValueOnly] at hv)
      (fun i e => by subst e; simp [Bytecode.ValueOnly] at hv))
      (fun s' h => by
        obtain ⟨A', hx, hsucc⟩ := hctx.next (by intro e; subst e; simp [Bytecode.ValueOnly] at hv)
        obtain ⟨j, k, hvs⟩ := exec_valueStep hv hvecHead hmod.2.2.2.1 hx
        exact refInv_valueOnly hr hsf (typeArgsOk_values hact.typeArgs) hi hv h hctx.ref hvs hsucc
          (conformsAll_wellFormed hctx.conf))
  have hvalueG : instr.ValueOnlyGeneric = true →
      ExecRuns P N (RefInvariant P) (Step.exec P s fr rest info code instr) := fun hv =>
    execRuns_of (exec_notNative (fun h e => by subst e; simp [Bytecode.ValueOnlyGeneric] at hv)
      (fun i e => by subst e; simp [Bytecode.ValueOnlyGeneric] at hv))
      (fun s' h => by
        obtain ⟨A', hx, hsucc⟩ :=
          hctx.next (by intro e; subst e; simp [Bytecode.ValueOnlyGeneric] at hv)
        obtain ⟨j, k, hvs⟩ := exec_valueStepGeneric hv hx
        exact refInv_valueShape hr hsf (typeArgsOk_values hact.typeArgs)
          (exec_valueGeneric hi hv h) hctx.ref hvs hsucc (conformsAll_wellFormed hctx.conf))
  have hruns : (∀ h, instr ≠ .call h) → (∀ i, instr ≠ .callGeneric i) →
      (∀ s', stepOf (Step.exec P s fr rest info code instr) = .done (.running s') →
        RefInvariant P s') →
      ExecRuns P N (RefInvariant P) (Step.exec P s fr rest info code instr) := fun hne hng h =>
    execRuns_of (exec_notNative hne hng) h
  cases instr
  case brTrue | brFalse | branch | nop | ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8
    | ldI16 | ldI32 | ldI64 | ldI128 | ldI256 | ldTrue | ldFalse | ldConst | castU8 | castU16
    | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32 | castI64 | castI128
    | castI256 | add | sub | mul | mod | div | bitOr | bitAnd | xor | or | and | not | lt | gt | le
    | ge | shl | shr | negate | abort | abortMsg | pack | unpack | packVariant | unpackVariant
    | vecPack | vecUnpack =>
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h) trivial) (hvalue rfl)
  case exists_ d => exact ExecSafe.and (exists_safe hc hact hchain hi) (hvalue rfl)
  case moveFrom d => exact refSafe_moveFrom hwf hc hr hsf hact hchain hctx hi
  case pop =>
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h) trivial)
      (hruns (fun _ h => by cases h) (fun _ h => by cases h) fun s' h =>
        refInv_pop hr hsf hctx hi h)
  case copyLoc i =>
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h) trivial)
      (hruns (fun _ h => by cases h) (fun _ h => by cases h) fun s' h =>
        refInv_copyLoc hr hsf hctx hi h)
  case moveLoc i =>
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h) trivial)
      (hruns (fun _ h => by cases h) (fun _ h => by cases h) fun s' h =>
        refInv_moveLoc hr hsf hctx hact.locals hi h)
  case stLoc i =>
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h) trivial)
      (hruns (fun _ h => by cases h) (fun _ h => by cases h) fun s' h =>
        refInv_stLoc hr hsf hctx hact.locals hi h)
  case call h =>
    by_cases hnm : NoModeledCall P fr.function.module info.module (.call h)
    case neg => exact refSafe_callModeled hwf hc hN hinv hsf hact hchain hctx hi (hattr h rfl) hnm
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h) hnm)
      (refRuns_call' hwf hs hsigs hc hr hsf hact hctx hi (hattr h rfl) hnm)
  case ret =>
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h) trivial)
      (refRuns_ret hr hsf hctx hact.fn)
  case eq => exact refSafe_eq (positive := true) hc hr hsf hact hchain hctx hi
  case neq => exact refSafe_eq (positive := false) hc hr hsf hact hchain hctx hi
  case readRef => exact refSafe_readRef hc hr hsf hact hchain hctx hi
  case writeRef => exact refSafe_writeRef hc hr hsf hact hchain hctx hi
  case freezeRef => exact refSafe_freezeRef hc hr hsf hact hchain hctx hi
  case mutBorrowLoc i => exact refSafe_borrowLoc (mut_ := true) hc hr hsf hact hchain hctx hi
  case immBorrowLoc i => exact refSafe_borrowLoc (mut_ := false) hc hr hsf hact hchain hctx hi
  case mutBorrowField h => exact refSafe_borrowField (mut_ := true) hc hr hsf hact hchain hctx hi
  case immBorrowField h => exact refSafe_borrowField (mut_ := false) hc hr hsf hact hchain hctx hi
  case mutBorrowVariantField h =>
    exact refSafe_borrowVariantField (mut_ := true) hc hr hsf hact hchain hctx hi
  case immBorrowVariantField h =>
    exact refSafe_borrowVariantField (mut_ := false) hc hr hsf hact hchain hctx hi
  case testVariant h => exact refSafe_testVariantG hc hr hsf hact hchain hl9 hctx hi
  case vecLen sig => exact refSafe_vecLen hc hr hsf hact hchain hctx hi
  case vecImmBorrow sig => exact refSafe_vecBorrow (mut_ := false) hc hr hsf hact hchain hctx hi
  case vecMutBorrow sig => exact refSafe_vecBorrow (mut_ := true) hc hr hsf hact hchain hctx hi
  case vecPushBack sig => exact refSafe_vecPushBack hc hr hsf hact hchain hctx hi
  case vecPopBack sig => exact refSafe_vecPopBack hc hr hsf hact hchain hctx hi
  case vecSwap sig => exact refSafe_vecSwap hc hr hsf hact hchain hctx hi
  case moveTo d => exact refSafe_moveTo hc hr hsf hact hchain hctx hi
  case mutBorrowGlobal d =>
    exact refSafe_borrowGlobal (mut_ := true) hwf hc hr hsf hact hchain hctx hi
  case immBorrowGlobal d =>
    exact refSafe_borrowGlobal (mut_ := false) hwf hc hr hsf hact hchain hctx hi
  case packGeneric | unpackGeneric | packVariantGeneric | unpackVariantGeneric =>
    exact ExecSafe.and (execGeneric_safe hc hN hact hchain hi rfl trivial) (hvalueG rfl)
  case existsGeneric i => exact ExecSafe.and (exists_safeGeneric hc hact hchain hi) (hvalueG rfl)
  case callGeneric i =>
    by_cases hnm : NoModeledCall P fr.function.module info.module (.callGeneric i)
    case neg =>
      exact refSafe_callGenericModeled hwf hc hN hinv hsf hact hchain hctx hi (hattrG i rfl) hnm
    exact ExecSafe.and (execGeneric_safe hc hN hact hchain hi rfl hnm)
      (refRuns_callGeneric hwf hs hsigs hc hr hsf hact hctx hi (hattrG i rfl) hnm)
  case testVariantGeneric i => exact refSafe_testVariantGeneric hc hr hsf hact hchain hctx hi
  case mutBorrowFieldGeneric i =>
    exact refSafe_borrowFieldGeneric (mut_ := true) hc hr hsf hact hchain hctx hi
  case immBorrowFieldGeneric i =>
    exact refSafe_borrowFieldGeneric (mut_ := false) hc hr hsf hact hchain hctx hi
  case mutBorrowVariantFieldGeneric i =>
    exact refSafe_borrowVariantFieldGeneric (mut_ := true) hc hr hsf hact hchain hctx hi
  case immBorrowVariantFieldGeneric i =>
    exact refSafe_borrowVariantFieldGeneric (mut_ := false) hc hr hsf hact hchain hctx hi
  case moveFromGeneric i => exact refSafe_moveFromGeneric hwf hc hr hsf hact hchain hctx hi
  case moveToGeneric i => exact refSafe_moveToGeneric hc hr hsf hact hchain hctx hi
  case mutBorrowGlobalGeneric i =>
    exact refSafe_borrowGlobalGeneric (mut_ := true) hwf hc hr hsf hact hchain hctx hi
  case immBorrowGlobalGeneric i =>
    exact refSafe_borrowGlobalGeneric (mut_ := false) hwf hc hr hsf hact hchain hctx hi
  case packClosure | packClosureGeneric =>
    exact ExecSafe.and (execClosure_safe hc hact hchain hi rfl) (hvalueG rfl)
  case callClosure sig =>
    exact ExecSafe.and (execClosure_safe hc hact hchain hi rfl)
      (refRuns_callClosure hwf hs hsigs hc hr hsf hact hctx hi)
  all_goals
    simp [Bytecode.inSubset, Bytecode.inCore, Bytecode.inDataByValue, Bytecode.inReferences,
      Bytecode.inGenerics] at hsub1

end Main

end MoveBytecode.Proofs
