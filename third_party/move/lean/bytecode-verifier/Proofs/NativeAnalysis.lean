-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefTracked

/-!
# The analysis at a call of a modeled native

UNTRUSTED (checked by the kernel). A native is called by a `Call` or
`CallGeneric` of the current frame (`tf_native_instr`), of the handle the
analysis runs as `Call` (`exec_callNative_link`), whose parameters are the
native's arguments. In a state of the reference invariant, the analysis's
state at the call describes the current frame, and checked each reference
argument: readable where immutable, writable where mutable
(`nativeCall_analysis`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety MoveBytecode.Step

set_option maxHeartbeats 1000000 in
theorem tf_exec_call {P : Program} {s s' : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {instr : Bytecode} {f : FunctionRef}
    {args views : List Value}
    (h : Step.exec P s fr rest info code instr = .ok (.callNative s' f args views)) :
    (∃ hd, instr = .call hd) ∨ ∃ i, instr = .callGeneric i := by
  cases instr
  case call hd => exact Or.inl ⟨hd, rfl⟩
  case callGeneric i => exact Or.inr ⟨i, rfl⟩
  case ret =>
    refine notNative_absurd ?_ h
    unfold Step.exec Step.ret
    not_native
  case packClosure hd mask =>
    refine notNative_absurd ?_ h
    show NotNative (Step.packClosure P s fr rest info hd mask)
    unfold Step.packClosure
    not_native
  case packClosureGeneric i mask =>
    refine notNative_absurd ?_ h
    show NotNative (do
      let x ← Step.need info.module.functionInstantiations[i]? .badIndex
      Step.packClosure P s fr rest info x.handle mask
        (← Step.genericArgs info.module fr.function.typeArgs x.typeParameters))
    refine notNative_bind fun _ _ => ?_
    refine notNative_bind fun _ _ => ?_
    unfold Step.packClosure
    not_native
  case callClosure sig =>
    refine notNative_absurd ?_ h
    show NotNative (Step.callClosure P s fr rest info sig)
    unfold Step.callClosure
    not_native
  all_goals
    refine notNative_absurd ?_ h
    unfold Step.exec
    not_native

/-- A native is called by a call instruction of the current frame. -/
theorem tf_native_instr {P : Program} {s s' : State} {f : FunctionRef} {args views : List Value}
    (h : step P s = .callNative s' f args views) :
    ∃ fr rest info code instr, s.frames = fr :: rest ∧ P.function? fr.function = some info ∧
      info.definition.code = some code ∧ code.code[fr.pc]? = some instr ∧
      ((∃ hd, instr = .call hd) ∨ ∃ i, instr = .callGeneric i) := by
  unfold step at h
  split at h
  · simp at h
  · next fr rest hsf =>
    split at h
    · simp at h
    · next info hinfo =>
      split at h
      · simp at h
      · next code hcode =>
        split at h
        · simp at h
        · next instr hinstr =>
          split at h
          · next r hr =>
            subst h
            exact ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, tf_exec_call hr⟩
          · simp at h


/-- A step that calls a native runs a call instruction of the current frame. -/
theorem native_exec {P : Program} {s s' : State} {f : FunctionRef} {args views : List Value}
    (h : step P s = .callNative s' f args views) :
    ∃ fr rest info code instr, s.frames = fr :: rest ∧ P.function? fr.function = some info ∧
      info.definition.code = some code ∧ code.code[fr.pc]? = some instr ∧
      Step.exec P s fr rest info code instr = .ok (.callNative s' f args views) := by
  unfold step at h
  split at h
  · simp at h
  · next fr rest hsf =>
    split at h
    · simp at h
    · next info hinfo =>
      split at h
      · simp at h
      · next code hcode =>
        split at h
        · simp at h
        · next instr hinstr =>
          split at h
          · next r hr =>
            subst h
            exact ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hr⟩
          · simp at h

/-- **A call of a native names its handle**: the native is the instance of
the function the handle resolves to, and its arguments are the handle's
parameters. -/
theorem call_callNative_link {P : Program} {s s' : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {hd : FunctionHandleIndex} {tys : List RuntimeType} {f : FunctionRef}
    {args views : List Value}
    (h : Step.call P s fr rest info hd tys = .ok (.callNative s' f args views)) :
    ∃ c0 params, P.resolve fr.function.module hd = some c0 ∧ f = { c0 with typeArgs := tys } ∧
      P.handleParamTypesAt fr.function.module hd tys = some params ∧
      args.length = params.length := by
  unfold Step.call at h
  rw [bind_ok'] at h
  obtain ⟨callee, hcallee, h⟩ := h
  split at h
  · simp [Step.wrong] at h
  rw [bind_ok'] at h
  obtain ⟨calleeInfo, -, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨fh, -, h⟩ := h
  split at h
  · simp [Step.wrong] at h
  split at h
  · simp [Step.wrong] at h
  split at h
  · simp [Step.wrong] at h
  rw [bind_ok'] at h
  obtain ⟨paramTys, -, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨retTys, -, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨hps, hhps, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨hrs, -, h⟩ := h
  split at h
  · simp [Step.wrong] at h
  next hlink =>
  split at h
  · simp [Step.wrong] at h
  rw [bind_ok'] at h
  obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := h
  obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · simp [Step.wrong] at h
  split at h
  · rw [bind_ok'] at h
    obtain ⟨views', -, h⟩ := h
    split at h
    · simp [Step.wrong] at h
    simp only [pure, Except.pure, Except.ok.injEq, StepResult.callNative.injEq] at h
    obtain ⟨rfl, rfl, rfl, rfl⟩ := h
    have hl : hps = paramTys := by
      simp only [Bool.or_eq_true, bne_iff_ne, ne_eq, not_or, Decidable.not_not] at hlink
      exact hlink.1
    refine ⟨callee, hps, need_ok' hcallee, rfl, need_ok' hhps, ?_⟩
    rw [hl]
    simp [Nat.min_eq_left hn]
  · rw [bind_ok'] at h
    obtain ⟨_, -, h⟩ := h
    split at h <;> split at h <;> simp [Step.fail, pure, Except.pure] at h

/-- **A call of a native, `Call` or `CallGeneric`, names its handle**, which
the analysis runs as `Call`. -/
theorem exec_callNative_link {P : Program} {s s' : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {instr : Bytecode} {f : FunctionRef}
    {args views : List Value}
    (hex : Step.exec P s fr rest info code instr = .ok (.callNative s' f args views)) :
    ∃ hd τ c0 params, CallsHandle info.module instr hd ∧ instr ≠ .ret ∧
      instr.control = .fallThrough ∧ (∀ a, availabilityAfter instr a = a) ∧
      (∀ (view : FunctionView) (A A' : RefState),
        (ReferenceSafety.executeInner info.module view instr).run A = .ok ((), A') →
          (ReferenceSafety.executeInner info.module view (.call hd)).run A = .ok ((), A')) ∧
      P.resolve fr.function.module hd = some c0 ∧ f = { c0 with typeArgs := τ } ∧
      P.handleParamTypesAt fr.function.module hd τ = some params ∧
      args.length = params.length := by
  rcases tf_exec_call hex with ⟨hd, rfl⟩ | ⟨i, rfl⟩
  · obtain ⟨c0, params, hres, rfl, hparams, hlen⟩ := call_callNative_link hex
    exact ⟨hd, [], c0, params, Or.inl rfl, (by intro h; cases h), rfl, fun _ => rfl,
      fun _ _ _ hx => hx, hres, rfl, hparams, hlen⟩
  · simp only [Step.exec] at hex
    rw [bind_ok'] at hex
    obtain ⟨x, hx, hex⟩ := hex
    rw [bind_ok'] at hex
    obtain ⟨τ, -, hex⟩ := hex
    obtain ⟨c0, params, hres, rfl, hparams, hlen⟩ := call_callNative_link hex
    refine ⟨x.handle, τ, c0, params, Or.inr ⟨i, x, rfl, need_ok' hx, rfl⟩, (by intro h; cases h),
      rfl, fun _ => rfl, fun view A A' hrun => ?_, hres, rfl, hparams, hlen⟩
    obtain ⟨x', hx', h'⟩ := exec_callGeneric' hrun
    rw [need_ok' hx, Option.some.injEq] at hx'
    subst hx'
    exact h'

/-- **The analysis at a call of a modeled native**, in a state of the
reference invariant: its state at the call describes the current frame, and
it checked each reference argument readable where immutable and writable where
mutable. -/
theorem nativeCall_analysis {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) (hsigs : P.ApiSigs)
    (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
      ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
    {s s₁ : State} (hinv : InvRefs P s) {f : FunctionRef} {args views : List Value}
    {k : NativeShape} (hshape : P.nativeShape? f = some k)
    (hstep : step P s = .callNative s₁ f args views) :
    ∃ (fr : Frame) (rest : List Frame) (info : FunctionInfo) (code : CodeUnit) (instr : Bytecode)
      (view : FunctionView) (A : RefState) (stys : List SignatureToken) (tys : List RuntimeType)
      (ps : List Reference) (L : Nat → Option Reference),
      s.frames = fr :: rest ∧ ActiveAt P (modelTypes P) (modelAvail P) fr info code ∧
      code.code[fr.pc]? = some instr ∧ TopCtx P s fr info code instr view A stys tys ps ∧
      Desc A fr (gsidOf P (s.frames.map Frame.skel)) ps L ∧
      (∀ x r, x ∈ heldIds A → L x = some r → RefOk P s fr r) ∧
      ∀ r, Value.ref r ∈ fr.stack.take args.length → ∃ (kk x : Nat), fr.stack[kk]? = some (.ref r) ∧
        A.stack[kk]? = some (.reference x) ∧ x ∈ heldIds A ∧ L x = some r ∧
        ((r.mutable = false ∧ A.graph.isReadable x none = .ok true) ∨
          (r.mutable = true ∧ A.graph.isWritable x = .ok true)) := by
  obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hi, hex⟩ := native_exec hstep
  obtain ⟨fr₁, rest₁, hsf₁, ⟨info₁, code₁, hact⟩, -⟩ := hinv.1
  rw [hsf, List.cons.injEq] at hsf₁
  obtain ⟨rfl, rfl⟩ := hsf₁
  have e1 : info₁ = info := Option.some.inj (hact.fn.symm.trans hinfo)
  subst info₁
  have e2 : code₁ = code := Option.some.inj (hact.body.symm.trans hcode)
  subst code₁
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtxGen hwf hs hsigs hinv hsf hact hi
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  obtain ⟨hdl, τ, c0, params, -, hret, -, -, hconv, hres, rfl, hparams, hlen⟩ :=
    exec_callNative_link hex
  obtain ⟨A', hx, -⟩ := hctx.next hret
  have hx' := hconv view A A' hx
  obtain ⟨fh, hfh, -⟩ := exec_call hx'
  have hattr : ReferenceSafety.borrowFieldMutable fh.attributes = none := by
    cases hb : ReferenceSafety.borrowFieldMutable fh.attributes with
    | none => rfl
    | some off =>
      exfalso
      obtain ⟨cinfo, -, hcf, hcode', -⟩ := nativeShape_entry hshape
      obtain ⟨⟨code', -, hc', -⟩, -⟩ := apiCall_wrapperG hwf hapi hact.fn hfh hb hres hcf
      rw [hcode'] at hc'
      cases hc'
  obtain ⟨psig, rs, hps, -, hcore⟩ := exec_call_core hx' hfh hattr
  have hmod := (function?_parts hact.fn).1
  have hpsig : RuntimeType.ofSignatureWith info.module τ psig = some params := by
    have h1 : P.handleParamTypesAt fr.function.module hdl τ = some params := hparams
    simp only [Program.handleParamTypesAt, hmod, hfh, hps, bind, Option.bind_some] at h1
    exact h1
  have hnp : params.length = psig.length := ofSignature_length hpsig
  obtain ⟨_, _, _, _, _, -, -, -, -, hwr, -⟩ := hcore
  refine ⟨fr, rest, info, code, instr, view, A, stys, tys, ps, L, hsf, hact, hi, hctx, hd, hrefs,
    fun r hr => ?_⟩
  obtain ⟨kk, hkk⟩ := List.mem_iff_getElem?.1 hr
  have hkn : kk < args.length := by
    have := (List.getElem?_eq_some_iff.1 hkk).1; simp at this; omega
  rw [List.getElem?_take_of_lt hkn] at hkk
  obtain ⟨x, hx, hxL⟩ := abs_of_ref hd hctx.typed hctx.targs hctx.sig hctx.conf hctx.height hkk
  have hH : x ∈ heldIds A := mem_heldIds.2 (Or.inr (List.mem_of_getElem? hx))
  have hxs : AbsValue.reference x ∈ A.stack.take psig.length :=
    List.mem_iff_getElem?.2 ⟨kk, by rw [List.getElem?_take_of_lt (by omega)]; exact hx⟩
  have hism := isMutable_of (hd.complete.held_mut hH hxL)
  refine ⟨kk, x, hkk, hx, hH, hxL, ?_⟩
  cases hm : r.mutable
  · left
    refine ⟨rfl, ?_⟩
    rw [hm] at hism
    simp [Graph.isReadable, hism, bind, Except.bind, pure, Except.pure]
  · right
    exact ⟨rfl, hwr x hxs (by rw [hism, hm])⟩

end MoveBytecode.Proofs
