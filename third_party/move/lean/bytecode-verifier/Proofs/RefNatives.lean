-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefCalls

/-!
# D1 with references: calls of the modeled natives

UNTRUSTED (checked by the kernel). A call of a modeled native
(`Program.nativeShape?`) keeps the type and availability invariant and the
reference invariant (`InvRefs`). The analysis takes it for a call
(`ReferenceSafety.coreCall`), and the semantics runs it as a call whose callee
writes back through its mutable reference arguments and returns at once:

- each write is through a tracked, writable, mutable reference argument, which
  no other tracked reference is strictly inside (`Complete.writable_inside`),
  and keeps the invariant in place, the caller still holding its arguments
  (`invRefs_writeAt`);
- the caller then resumes as from a callee that returns references inside its
  arguments (`resume_frameOk`, as `refInv_return` does), the places of a
  borrow-shape native lying inside its reference argument
  (`NativeSigs.borrow_result`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

variable {P : Program} {s : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

/-- **A write through a tracked writable reference of the current frame, in
place**, keeps the invariant: the frames keep their stacks and positions, the
current one its analysis state, and the references they hold stay tracked. -/
theorem invRefs_writeAt (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest)
    {A : RefState} (hA : modelRef P fr.function fr.pc = some A)
    {ps : List Reference} {L : Nat → Option Reference}
    (hd : Desc A fr (gsidOf P (s.frames.map Frame.skel)) ps L)
    (hrefs : ∀ x r, x ∈ heldIds A → L x = some r → RefOk P s fr r)
    (hchainR : Below P s fr ps rest)
    {p : Nat} {r : Reference} (hp : p ∈ heldIds A) (hpL : L p = some r)
    (hpw : A.graph.isWritable p = .ok true) (hpm : r.mutable = true)
    {w w' : Value} (hwf : WriteFacts P s (writeAt s r.location w') r.location w w') :
    Inv P (modelTypes P) (modelAvail P) (writeAt s r.location w') ∧
      RefInvariant P (writeAt s r.location w') ∧
      ActiveAt P (modelTypes P) (modelAvail P) (updFrame r.location w' fr) info code ∧
      ChainOk P (modelTypes P) (modelAvail P) fr.function (rest.map (updFrame r.location w')) ∧
      Desc A (updFrame r.location w' fr)
        (gsidOf P ((writeAt s r.location w').frames.map Frame.skel)) ps L ∧
      (∀ x r', x ∈ heldIds A → L x = some r' →
        RefOk P (writeAt s r.location w') (updFrame r.location w' fr) r') ∧
      Below P (writeAt s r.location w') (updFrame r.location w' fr) ps
        (rest.map (updFrame r.location w')) := by
  have hnd := hr.ids
  have hfrm : fr ∈ s.frames := by rw [hsf]; exact List.mem_cons_self
  have hval : ∀ g ∈ s.frames, ∀ i, r.location.root = .local_ g.id i →
      g.locals[i]? = some (some w) ∧ ∀ r', w ≠ .ref r' := fun g hg => hwf.slot g hg
  have hslot : ∀ g ∈ s.frames, ∀ i, r.location.root = .local_ g.id i →
      ∃ w₀, g.locals[i]? = some (some w₀) := fun g hg i hi => ⟨w, (hval g hg i hi).1⟩
  obtain ⟨hid1, hfun1, hpc1, hstk1⟩ := updFrame_eq r.location w' fr
  have hlocalsTop := hwf.locals fr hfrm
  have hupd := hact.upd (l := r.location) (w' := w') (hslot fr hfrm) hlocalsTop
  have hsk : (writeAt s r.location w').frames.map Frame.skel = s.frames.map Frame.skel := by
    rw [writeAt_frames hsf, hsf]
    show (updFrame r.location w' fr).skel :: (rest.map (updFrame r.location w')).map Frame.skel =
      fr.skel :: rest.map Frame.skel
    congr 1
    · exact updFrame_skel r.location w' fr
    · exact map_updFrame_skel r.location w' rest
  have hgs : s.frames.map Frame.skel = (fr.function, fr.countedActive) :: rest.map Frame.skel := by
    rw [hsf]; rfl
  have hchainU : ChainOk P (modelTypes P) (modelAvail P) fr.function
      (rest.map (updFrame r.location w')) :=
    ChainOk.upd (l := r.location) (w' := w') hchain (fun g hg => ⟨hslot g (by
      rw [hsf]; exact List.mem_cons_of_mem _ hg), hwf.locals g
        (by rw [hsf]; exact List.mem_cons_of_mem _ hg)⟩)
  have hdU : Desc A (updFrame r.location w' fr)
      (gsidOf P ((writeAt s r.location w').frames.map Frame.skel)) ps L := by
    rw [hsk]
    refine hd.upd (l := r.location) (w' := w') (fun i hi' => ?_) hid1 hstk1 rfl
    obtain ⟨hs, hwr⟩ := hval fr hfrm i hi'
    exact ⟨w, hs, hwr⟩
  have hokU : ∀ x r', x ∈ heldIds A → L x = some r' →
      RefOk P (writeAt s r.location w') (updFrame r.location w' fr) r' := fun x rx hx hxL =>
    hwf.refOk fr hfrm rx (hd.mem_vals hx hxL) _ _ (hrefs x rx hx hxL) hid1
      (hd.complete.writable_inside hp hpL hpw hx hxL)
  obtain ⟨S, hglob, hch⟩ := hchainR
  have hbelowU : Below P (writeAt s r.location w') (updFrame r.location w' fr) ps
      (rest.map (updFrame r.location w')) := by
    refine ⟨S, ?_, ?_⟩
    · refine hglob.skel hfun1 ?_
      have := hsk
      rw [writeAt_frames hsf, hsf] at this
      exact this
    · have hdc := hd.complete
      rw [hgs] at hdc
      have := ChainRef.afterWrite hwf rest fr.id fr.function ps S
        (fun g hg => by rw [hsf]; exact List.mem_cons_of_mem _ hg)
        (allowed_write hdc hglob hp hpL hpm) hch
      rw [hid1, hfun1]
      exact this
  have hids : ((writeAt s r.location w').frames.map (·.id)) = s.frames.map (·.id) := by
    simp only [writeAt, List.map_map]
    exact List.map_congr_left fun g _ => (updFrame_eq r.location w' g).1
  refine ⟨⟨_, _, writeAt_frames hsf _ _, ⟨info, code, hupd⟩, by rw [hfun1]; exact hchainU⟩,
    ⟨by rw [hids]; exact hnd,
    fun g hg => ?_, hwf.storage, ⟨_, _, ps, writeAt_frames hsf _ _, ⟨A, by
      rw [hfun1, hpc1]; exact hA, L, hdU, hokU⟩, hbelowU⟩, hr.active.skel hsk rfl, ?_⟩,
    hupd, hchainU, hdU, hokU, hbelowU⟩
  · have hgid : g.id ∈ s.frames.map (·.id) := by
      rw [← hids]; exact List.mem_map_of_mem hg
    obtain ⟨g0, hg0, he⟩ := List.mem_map.1 hgid
    rw [← he]
    exact hr.below g0 hg0
  · refine hr.heldTargets.of fun g' hg' rx hm k hk => ?_
    simp only [writeAt, List.mem_map] at hg'
    obtain ⟨g, hg, rfl⟩ := hg'
    exact ⟨g, hg, rx, frameVals_updFrame (hwf.value g hg) hm, rfl, rfl⟩

/-- **A write of a value of its target through a tracked reference** in a
frame of a state of the invariant has the facts a write needs. -/
theorem writeFacts_tracked (hr : RefInvariant P s) (hroots : RootsOk P s) {frx : Frame}
    (hfrx : frx ∈ s.frames) {r : Reference} (hok : RefOk P s frx r)
    (hmem : Value.ref r ∈ frameVals frx) {v w w' : Value} (hv : v.conforms P r.target = true)
    (hroot : s.root? r.location.root = some w) (hset : w.set? r.location.path v = some w') :
    WriteFacts P s (writeAt s r.location w') r.location w w' := by
  have hw := afterWrite_writeAt (P := P) s r.location w'
  obtain ⟨⟨T, T', hT, hpath, hfit⟩, -, htv, -, -⟩ := hok
  by_cases hex : T' = r.target
  · subst hex
    exact WriteFacts.exact hw hr.ids hroots hr.storage hroot hset hT hpath hv htv
  rcases hfit with hfit | ⟨⟨hpe, hnl⟩, hasg'⟩
  · exact absurd hfit hex
  have hTT : T' = T := by rw [hpe] at hpath; simp only [pathType] at hpath; exact
    (Option.some.inj hpath).symm
  subst hTT
  have hrig : rigid T' = false := by
    cases h : rigid T' with
    | false => rfl
    | true => exact absurd (assignable_rigid_left hasg' h) hex
  cases hρ : r.location.root with
  | local_ f i => exact absurd hρ (hnl f i)
  | global a t' =>
    exfalso
    rw [hρ] at hT hroot
    simp only [State.root?] at hroot
    simp only [RootType, hroot, Option.bind_some] at hT
    split at hT
    · simp only [Option.some.injEq] at hT
      unfold GlobalStorage.get? at hroot
      rw [Option.map_eq_some_iff] at hroot
      obtain ⟨r1, hr1, hw1⟩ := hroot
      have hr1m := List.find?_some hr1
      simp only [Bool.and_eq_true, beq_iff_eq] at hr1m
      have hc1 := hr.storage r1 (List.mem_of_find?_eq_some hr1)
      rw [hw1, hr1m.2] at hc1
      rw [← hT, conforms_type hc1 rfl] at hrig
      simp [rigid] at hrig
    · cases hT
  | held k =>
    have hl : r.location = ⟨.held k, []⟩ := by
      rcases hl : r.location with ⟨ρ, q⟩
      rw [hl] at hρ hpe
      simp only at hρ hpe
      rw [hρ, hpe]
    rw [hl] at hset hT hroot hw ⊢
    have hw'v : w' = v := (Option.some.inj hset).symm
    subst hw'v
    obtain ⟨τ, hτ⟩ := hr.heldTargets
    have hrτ := hτ frx hfrx r hmem k hl
    exact WriteFacts.held hw hroots hr.storage hroot hT hrig hv
      (fun g hg rx hm hx => (hτ g hg rx hm k hx).trans hrτ.symm)

/-! ## Writing back, write by write -/

/-- **The context of a write in place**: the invariant, with the current
frame at its analysis state `A`, described with `ps` and `L`. -/
structure WriteCtx (P : Program) (s : State) (fr : Frame) (rest : List Frame) (info : FunctionInfo)
    (code : CodeUnit) (A : RefState) (ps : List Reference) (L : Nat → Option Reference) : Prop where
  frames : s.frames = fr :: rest
  inv : Inv P (modelTypes P) (modelAvail P) s
  ref : RefInvariant P s
  act : ActiveAt P (modelTypes P) (modelAvail P) fr info code
  chain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest
  model : modelRef P fr.function fr.pc = some A
  desc : Desc A fr (gsidOf P (s.frames.map Frame.skel)) ps L
  refs : ∀ x r, x ∈ heldIds A → L x = some r → RefOk P s fr r
  below : Below P s fr ps rest
  roots : RootsOk P s

theorem updFrame_withStack (l : Location) (w' : Value) (g : Frame) (st : List Value) :
    updFrame l w' { g with stack := st } = { updFrame l w' g with stack := st } := by
  unfold updFrame
  split
  · split <;> rfl
  · rfl

theorem writeAt_withStack (l : Location) (w' : Value) (st : List Value) :
    writeAt { s with frames := { fr with stack := st } :: rest } l w' =
      { writeAt s l w' with
        frames := { updFrame l w' fr with stack := st } :: rest.map (updFrame l w') } := by
  simp [writeAt, updFrame_withStack]

theorem root?_withStack' (hsf : s.frames = fr :: rest) (st : List Value) (ρ : Root) :
    ({ s with frames := { fr with stack := st } :: rest } : State).root? ρ = s.root? ρ :=
  root?_replaceTop (s₁ := s) (s₂ := { s with frames := { fr with stack := st } :: rest }) hsf rfl
    rfl rfl rfl rfl ρ

/-- **Writing back through tracked writable mutable references**, pairwise
apart, of values of their targets: the writes succeed, from the caller's state
with any operand stack, and keep the context of a write, with the operand
stacks and positions unchanged. -/
theorem writeCtx_writeBack {A : RefState} {ps : List Reference} {L : Nat → Option Reference} :
    ∀ (refs : List Reference) (ws : List Value) {s : State} {fr : Frame} {rest : List Frame},
    WriteCtx P s fr rest info code A ps L →
    (∀ π ∈ refs, ∃ x ∈ heldIds A, L x = some π ∧ A.graph.isWritable x = .ok true ∧
      π.mutable = true ∧ Value.ref π ∈ fr.stack) →
    refs.Pairwise (fun a b => a.location.overlaps b.location = false) →
    conformsAll P ws (refs.map (·.target)) = true →
    ∃ V frV restV, WriteCtx P V frV restV info code A ps L ∧ frV.stack = fr.stack ∧
      frV.pc = fr.pc ∧ frV.id = fr.id ∧ frV.function = fr.function ∧
      frV.countedActive = fr.countedActive ∧ V.nextFrame = s.nextFrame ∧ V.active = s.active ∧
      V.lockDepth = s.lockDepth ∧
      ∀ st, writeBack { s with frames := { fr with stack := st } :: rest } refs ws =
        some { V with frames := { frV with stack := st } :: restV }
  | [], [], s, fr, rest, hw, _, _, _ =>
    ⟨s, fr, rest, hw, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, fun st => by simp [writeBack]⟩
  | π :: refs, v :: ws, s, fr, rest, hw, hπs, hdis, hconf => by
    simp only [List.map_cons, conformsAll_cons] at hconf
    obtain ⟨hv, hconf⟩ := hconf
    obtain ⟨x, hx, hxL, hxw, hπm, hπst⟩ := hπs π List.mem_cons_self
    have hok := hw.refs x π hx hxL
    obtain ⟨w, old, hroot, hwo, -⟩ := hok.root_value
    obtain ⟨w', hset⟩ := set_some _ v hwo (hok.writable hπm)
    have hfrm : fr ∈ s.frames := by rw [hw.frames]; exact List.mem_cons_self
    have hwf := writeFacts_tracked hw.ref hw.roots hfrm hok (mem_frameVals_stack hπst) hv hroot hset
    obtain ⟨hinv₁, hr₁, hact₁, hchain₁, hd₁, hok₁, hbelow₁⟩ :=
      invRefs_writeAt hw.ref hw.frames hw.act hw.chain hw.model hw.desc hw.refs hw.below hx hxL hxw
        hπm hwf
    obtain ⟨hid1, hfun1, hpc1, hstk1⟩ := updFrame_eq π.location w' fr
    have hw₁ : WriteCtx P (writeAt s π.location w') (updFrame π.location w' fr)
        (rest.map (updFrame π.location w')) info code A ps L :=
      ⟨writeAt_frames hw.frames _ _, hinv₁, hr₁, hact₁, by rw [hfun1]; exact hchain₁,
        by rw [hfun1, hpc1]; exact hw.model, hd₁, hok₁, hbelow₁, fun g hg => by
          rw [writeAt_frames hw.frames] at hg
          rcases List.mem_cons.1 hg with rfl | hg
          · exact hwf.locals fr hfrm
          · obtain ⟨g0, hg0, rfl⟩ := List.mem_map.1 hg
            exact hwf.locals g0 (by rw [hw.frames]; exact List.mem_cons_of_mem _ hg0)⟩
    rw [List.pairwise_cons] at hdis
    obtain ⟨V, frV, restV, hwV, hstkV, hpcV, hidV, hfunV, hcaV, hnfV, hacV, hldV, hwbV⟩ :=
      writeCtx_writeBack refs ws hw₁ (fun π' hπ' => by
        obtain ⟨x', hx', hxL', hxw', hm', hst'⟩ := hπs π' (List.mem_cons_of_mem _ hπ')
        exact ⟨x', hx', hxL', hxw', hm', by rw [hstk1]; exact hst'⟩) hdis.2 hconf
    have hca1 : (updFrame π.location w' fr).countedActive = fr.countedActive := by
      unfold updFrame; split
      · split <;> rfl
      · rfl
    refine ⟨V, frV, restV, hwV, hstkV.trans hstk1, hpcV.trans hpc1, hidV.trans hid1,
      hfunV.trans hfun1, hcaV.trans hca1, hnfV, hacV, hldV, fun st => ?_⟩
    have hroot' : ({ s with frames := { fr with stack := st } :: rest } : State).root?
        π.location.root = some w := by rw [root?_withStack' hw.frames]; exact hroot
    have hwr : ({ s with frames := { fr with stack := st } :: rest } : State).write? π.location v =
        some (({ s with frames := { fr with stack := st } :: rest } : State).setRoot
          π.location.root w') := by
      simp [State.write?, hroot', hset]
    simp only [writeBack, hwr, bind, Option.bind_some]
    rw [setRoot_invalidate, writeAt_withStack]
    exact hwbV st
  | [], _ :: _, _, _, _, _, _, _, h => by simp at h
  | _ :: _, [], _, _, _, _, _, _, h => by simp at h

/-! ## Resuming after the native -/

/-- The caller after a native returns `rv`, having popped `n` arguments. -/
abbrev resumedFrame (fr : Frame) (rv : List Value) (n : Nat) : Frame :=
  { fr with stack := rv.reverse ++ fr.stack.drop n, pc := fr.pc + 1 }

/-- The context of the current frame carries over to a state of the context
of a write with the same current frame's function, position and operand
stack. -/
theorem TopCtx.ofWriteCtx {instr : Bytecode} {view : FunctionView} {A : RefState}
    {stys : List SignatureToken} {tys : List RuntimeType} {ps : List Reference}
    (hctx : TopCtx P s fr info code instr view A stys tys ps)
    {V : State} {frV : Frame} {restV : List Frame} {L : Nat → Option Reference}
    (hW : WriteCtx P V frV restV info code A ps L) (hfun : frV.function = fr.function)
    (hpc : frV.pc = fr.pc) (hstk : frV.stack = fr.stack) :
    TopCtx P V frV info code instr view A stys tys ps where
  cert := by rw [hfun]; exact hctx.cert
  ref := by rw [hfun, hpc]; exact hctx.ref
  typesBefore := by rw [hpc]; exact hctx.typesBefore
  typed := by rw [hfun, hpc]; exact hctx.typed
  sig := by rw [hfun]; exact hctx.sig
  conf := by rw [hstk]; exact hctx.conf
  tc := by rw [hfun, hpc]; exact hctx.tc
  frameOk := ⟨L, hW.desc, hW.refs⟩
  chain := fun rest' h => by
    rw [hW.frames, List.cons.injEq] at h
    rw [← h.2]; exact hW.below
  roots := hW.roots
  exec := hctx.exec
  height := by rw [hstk]; exact hctx.height
  next := fun h => by
    obtain ⟨A', hx, hsucc⟩ := hctx.next h
    exact ⟨A', hx, fun pc' hpc' => by rw [hpc] at hpc'; rw [hfun]; exact hsucc pc' hpc'⟩
  view := by rw [hfun]; exact hctx.view
  targs := by rw [hfun]; exact hctx.targs

/-- **The caller resumes after a native**: from the state its writes left,
the caller still holding the arguments at the call, it pops them and pushes the
native's results, whose references are inside its reference arguments,
compatible with one another and tracked. -/
theorem invRefs_nativeResume (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hc : Certified P (modelTypes P) (modelAvail P))
    {V : State} {frV : Frame} {restV : List Frame} {A : RefState} {ps : List Reference}
    {L : Nat → Option Reference} (hW : WriteCtx P V frV restV info code A ps L)
    {instr : Bytecode} {view : FunctionView} {stys : List SignatureToken} {tys : List RuntimeType}
    (hctx : TopCtx P V frV info code instr view A stys tys ps)
    {h : FunctionHandleIndex} (hi : code.code[frV.pc]? = some instr)
    (hcallI : CallsHandle info.module instr h) (hret : instr ≠ .ret)
    (hctl : instr.control = .fallThrough) (hav : ∀ a, availabilityAfter instr a = a)
    (hrun : ∀ A', (ReferenceSafety.executeInner info.module view instr).run A = .ok ((), A') →
      (ReferenceSafety.executeInner info.module view (.call h)).run A = .ok ((), A'))
    {fh : FunctionHandle} (hfh : info.module.functionHandles[h]? = some fh)
    (hattr : ReferenceSafety.borrowFieldMutable fh.attributes = none)
    {c0 : FunctionRef} {cinfo : FunctionInfo}
    (hres : P.resolve frV.function.module h = some c0) (hcf : P.function? c0 = some cinfo)
    {τ : List RuntimeType} (hτ : ∀ x ∈ τ, x.isValue = true) {params returns : List RuntimeType}
    (hparams : P.handleParamTypesAt frV.function.module h τ = some params)
    (hreturns : P.handleReturnTypesAt frV.function.module h τ = some returns)
    (hcr : P.returnTypes { c0 with typeArgs := τ } = some returns)
    (hn : params.length ≤ frV.stack.length) {before after : List RuntimeType}
    (hbefore : modelTypes P frV.function frV.pc = some before)
    (hafter : typeAfter P frV.function info.module instr before = some after)
    (hta : returns.reverse ++ before.drop params.length = after)
    {rv : List Value} (hconf : conformsAll P rv returns = true)
    (hrc1 : ∀ r ∈ argRefs rv, ∃ π ∈ argRefs (frV.stack.take params.length).reverse,
      r.location.Inside π.location ∧ (r.mutable = true → π.mutable = true))
    (hrc2 : (argRefs rv).Pairwise Compat)
    (hheldNew : ∀ r ∈ argRefs rv, ∀ k, r.location ≠ ⟨.held k, []⟩)
    (hnew : ∀ r ∈ argRefs rv, RefOk P
      { V with frames := resumedFrame frV rv params.length :: restV }
      (resumedFrame frV rv params.length) r) :
    InvRefs P { V with frames := resumedFrame frV rv params.length :: restV } := by
  have hsf := hW.frames
  have hr := hW.ref
  obtain ⟨S, hglob, hch⟩ := hW.below
  have hcid : frV.id < V.nextFrame := hr.below frV (by rw [hsf]; exact List.mem_cons_self)
  obtain ⟨Sc, -, -, hsusp, -⟩ := call_suspendAt hwf hctx hW.act.locals
    (typeArgsOk_values hW.act.typeArgs) hi hcallI hret hctl hrun hW.act.fn hW.act.body hfh hattr hres
    hcf hτ hparams hreturns hcr hn hcid hsf hr.active.counted hglob
  obtain ⟨m, rs, rets', args, A0, A0', A0'', L0, hf⟩ := hsusp.call
  have e := hf.calleeRets
  rw [hcr, Option.some.injEq] at e
  subst e
  -- The type and availability invariant.
  have hdrop : conformsAll P (frV.stack.drop params.length) (before.drop params.length) = true := by
    obtain ⟨tys', htys', hconf'⟩ := hW.act.types
    rw [hbefore, Option.some.injEq] at htys'
    subst htys'
    exact conformsAll_drop params.length hconf'
  have hsucc : frV.pc + 1 ∈ successors code.code frV.pc := by simp [successors, hi, hctl]
  have hactN : ActiveAt P (modelTypes P) (modelAvail P)
      (resumedFrame frV rv params.length) info code :=
    activeAt_succ hc hW.act hi hbefore hafter rfl hsucc
      (by rw [← hta]; exact conformsAll_append (conformsAll_reverse hconf) hdrop)
      (localsOk_of hW.act.locals rfl rfl) (fun i hi' => by rw [hav] at hi'; exact hW.act.available i hi')
      (fun i hi' => by rw [hav] at hi'; exact hW.act.unavailable i hi')
  -- The frames no newer than the caller keep their functions and locals.
  have hfr : ∀ f, f ≤ frV.id → ∀ g, V.frame? f = some g →
      ∃ g', ({ V with frames := resumedFrame frV rv params.length :: restV } : State).frame? f =
        some g' ∧ g'.function = g.function ∧ g'.locals = g.locals := by
    intro f _ g hg
    simp only [State.frame?, hsf, List.find?_cons] at hg ⊢
    by_cases h2 : frV.id = f
    · have h2' : (frV.id == f) = true := by simpa using h2
      simp only [h2', Option.some.injEq] at hg ⊢
      subst hg
      exact ⟨_, rfl, rfl, rfl⟩
    · have h2' : (frV.id == f) = false := by simpa using h2
      simp only [h2'] at hg ⊢
      exact ⟨g, hg, rfl, rfl⟩
  have hok : FrameOk P { V with frames := resumedFrame frV rv params.length :: restV }
      (resumedFrame frV rv params.length) ps A0' := by
    obtain ⟨τ', hτ', hrets⟩ := hf.retTypes
    refine resume_frameOk (caller := { frV with stack := frV.stack.drop params.length })
      (s' := { V with frames := resumedFrame frV rv params.length :: restV })
      rfl hτ' hrets hf.core hf.distinct hf.desc hf.height hconf
      (fun r hr' => ?_) hf.cpsArgs hrc2 rfl rfl rfl (fun x r hx hxr => ?_) hnew
    · obtain ⟨π, hπ, hin, hm⟩ := hrc1 r hr'
      exact ⟨π, hf.cpsSub.subset hπ, hin, hm⟩
    · obtain ⟨hok, -⟩ := hf.refs x r hx hxr
      exact hok.agree hok.below hfr rfl rfl rfl (Nat.le_refl _)
  refine ⟨⟨_, restV, rfl, ⟨info, code, hactN⟩, hW.chain⟩, ⟨?_, fun g hg => ?_, hr.storage,
    ⟨_, restV, ps, rfl, ⟨A0'', hf.after, hok.le hf.le⟩, S, hsusp.glob.skel rfl (by
      simp [Frame.skel]),
    ChainRef.transferBelow (fun f hf' => hfr f (Nat.le_of_lt hf')) rfl rfl rfl restV frV.id
      frV.function ps S (Nat.le_refl _) hch⟩, hr.active.skel (by simp [hsf, Frame.skel]) rfl, ?_⟩⟩
  · have := hr.ids
    rw [hsf] at this
    simpa using this
  · rcases List.mem_cons.1 hg with rfl | hg
    · exact hr.below frV (by rw [hsf]; exact List.mem_cons_self)
    · exact hr.below g (by rw [hsf]; exact List.mem_cons_of_mem _ hg)
  · refine hr.heldTargets.of fun g' hg' rx hm k hk => ?_
    rcases List.mem_cons.1 hg' with rfl | hg'
    · simp only [frameVals, List.mem_append, List.mem_reverse] at hm
      rcases hm with (hm | hm) | hm
      · exact absurd hk (hheldNew rx (mem_argRefs.2 hm) k)
      · exact ⟨frV, by rw [hsf]; exact List.mem_cons_self, rx,
          mem_frameVals_stack (List.mem_of_mem_drop hm), rfl, rfl⟩
      · exact ⟨frV, by rw [hsf]; exact List.mem_cons_self, rx,
          by simp only [frameVals, List.mem_append]; exact Or.inr hm, rfl, rfl⟩
    · exact ⟨g', by rw [hsf]; exact List.mem_cons_of_mem _ hg', rx, hm, rfl, rfl⟩

/-! ## What the call checks -/

/-- **A native sees its arguments** when every reference argument is valid:
a value argument as it is, a reference argument as the value it designates. -/
theorem nativeView_some {s : State} :
    ∀ {args : List Value}, (∀ r, Value.ref r ∈ args → ∃ v, s.deref? r.location r.since = some v) →
      ∃ views, s.nativeView args = some views ∧ views.length = args.length ∧
        ∀ (i : Nat) (r : Reference), args[i]? = some (.ref r) →
          views[i]? = s.deref? r.location r.since
  | [], _ => ⟨[], rfl, rfl, fun i r h => by simp at h⟩
  | a :: args, h => by
    obtain ⟨views, hv, hl, hi⟩ := nativeView_some (args := args)
      fun r hr => h r (List.mem_cons_of_mem _ hr)
    unfold State.nativeView at hv ⊢
    rw [List.mapM_cons, hv]
    have hrest : ∀ (b : Value), (∀ r, a = .ref r → some b = s.deref? r.location r.since) →
        ∀ (i : Nat) (r : Reference), (a :: args)[i]? = some (.ref r) →
          (b :: views)[i]? = s.deref? r.location r.since := by
      intro b hb i r hir
      cases i with
      | zero =>
        simp only [List.getElem?_cons_zero, Option.some.injEq] at hir ⊢
        exact hb r hir
      | succ i => simpa using hi i r (by simpa using hir)
    cases a with
    | ref r =>
      obtain ⟨b, hb⟩ := h r List.mem_cons_self
      exact ⟨b :: views, by simp [hb], by simp [hl], hrest b fun r' e => by cases e; exact hb.symm⟩
    | _ => exact ⟨_ :: views, rfl, by simp [hl], hrest _ fun r' e => by cases e⟩

theorem overlaps_symm (l m : Location) : l.overlaps m = m.overlaps l := by
  simp only [Location.overlaps]
  cases hr : (l.root == m.root) <;> cases hr' : (m.root == l.root) <;> simp_all [Bool.or_comm]

theorem overlaps_cases {l m : Location} (h : l.overlaps m = true) :
    l = m ∨ l.StrictlyInside m ∨ m.StrictlyInside l := by
  simp only [Location.overlaps, Bool.and_eq_true, beq_iff_eq, Bool.or_eq_true,
    List.isPrefixOf_iff_prefix] at h
  obtain ⟨hr, hp | hp⟩ := h
  · by_cases he : l.path = m.path
    · left; cases l; cases m; simp_all
    · right; right; exact ⟨⟨hr.symm, hp⟩, Ne.symm he⟩
  · by_cases he : l.path = m.path
    · left; cases l; cases m; simp_all
    · right; left; exact ⟨⟨hr, hp⟩, he⟩

/-- Two compatible references, one of them mutable, do not overlap. -/
theorem compat_no_overlap {d r : Reference} (h : Compat d r) (hm : d.mutable = true ∨ r.mutable = true) :
    d.location.overlaps r.location = false := by
  cases ho : d.location.overlaps r.location with
  | false => rfl
  | true =>
    exfalso
    obtain ⟨h1, h2, h3, h4⟩ := h
    rcases overlaps_cases ho with he | hin | hin
    · rcases hm with hm | hm
      · exact (h1 hm).2 he.symm
      · exact (h2 hm).2 he
    · cases hr : r.mutable
      · cases hd : d.mutable
        · rw [hd, hr] at hm; simp at hm
        · rw [h3 hr hin] at hd; cases hd
      · exact (h2 hr).1 hin
    · cases hd : d.mutable
      · cases hr : r.mutable
        · rw [hd, hr] at hm; simp at hm
        · rw [h4 hd hin] at hr; cases hr
      · exact (h1 hd).1 hin

theorem exclusive_of_pairwise {R : List Reference}
    (h : ∀ (i j : Nat), i ≠ j → ∀ r r', R[i]? = some r → R[j]? = some r' → r.mutable = true →
      r.location.overlaps r'.location = false) :
    (R.zipIdx.all fun x => !x.1.mutable || R.zipIdx.all fun y =>
      x.2 == y.2 || !x.1.location.overlaps y.1.location) = true := by
  rw [List.all_eq_true]
  intro x hx
  simp only [Bool.or_eq_true, Bool.not_eq_true', List.all_eq_true]
  cases hm : x.1.mutable
  · exact Or.inl rfl
  · refine Or.inr fun y hy => ?_
    by_cases hij : x.2 = y.2
    · exact Or.inl (by simp [hij])
    · right
      obtain ⟨hxl, hxe⟩ := List.mem_zipIdx' hx
      obtain ⟨hyl, hye⟩ := List.mem_zipIdx' hy
      exact h x.2 y.2 hij x.1 y.1 (by rw [hxe]; exact List.getElem?_eq_getElem hxl)
        (by rw [hye]; exact List.getElem?_eq_getElem hyl) hm

/-- Compatible reference arguments overlap nowhere where mutable. -/
theorem exclusive_of_compat {args : List Value} (h : (argRefs args).Pairwise Compat) :
    Step.exclusiveArguments args = true := by
  unfold Step.exclusiveArguments
  refine exclusive_of_pairwise (R := argRefs args) fun i j hij r r' hi hj hm => ?_
  rw [List.pairwise_iff_getElem] at h
  have hil := (List.getElem?_eq_some_iff.1 hi).1
  have hjl := (List.getElem?_eq_some_iff.1 hj).1
  have hri : (argRefs args)[i] = r := by rw [List.getElem?_eq_getElem hil] at hi; exact Option.some.inj hi
  have hrj : (argRefs args)[j] = r' := by rw [List.getElem?_eq_getElem hjl] at hj; exact Option.some.inj hj
  rcases Nat.lt_or_gt_of_ne hij with hlt | hlt
  · have := h i j hil hjl hlt
    rw [hri, hrj] at this
    exact compat_no_overlap this (Or.inl hm)
  · have := h j i hjl hil hlt
    rw [hri, hrj] at this
    rw [overlaps_symm]
    exact compat_no_overlap this (Or.inr hm)

/-- **A mutable reference argument of a call is a tracked, writable node** of
the analysis's state at the call, which checked it. -/
theorem mutArgs_writable {instr : Bytecode} {view : FunctionView} {A : RefState}
    {stys : List SignatureToken} {tys : List RuntimeType} {ps : List Reference}
    (hctx : TopCtx P s fr info code instr view A stys tys ps) {L : Nat → Option Reference}
    {gsid : Nat → Option StructId} (hd : Desc A fr gsid ps L) (hret : instr ≠ .ret)
    {h : FunctionHandleIndex}
    (hrun : ∀ A', (ReferenceSafety.executeInner info.module view instr).run A = .ok ((), A') →
      (ReferenceSafety.executeInner info.module view (.call h)).run A = .ok ((), A'))
    (hfn : P.function? fr.function = some info)
    {fh : FunctionHandle} (hfh : info.module.functionHandles[h]? = some fh)
    (hattr : ReferenceSafety.borrowFieldMutable fh.attributes = none)
    {τ params : List RuntimeType}
    (hparams : P.handleParamTypesAt fr.function.module h τ = some params) :
    ∀ π ∈ mutRefsOf (fr.stack.take params.length).reverse, ∃ x ∈ heldIds A, L x = some π ∧
      A.graph.isWritable x = .ok true ∧ π.mutable = true ∧ Value.ref π ∈ fr.stack := by
  obtain ⟨A', hx, -⟩ := hctx.next hret
  have hx := hrun _ hx
  obtain ⟨psig, rs, hps, -, hcore⟩ := exec_call_core hx hfh hattr
  have hmod := (function?_parts hfn).1
  have hpsig : RuntimeType.ofSignatureWith info.module τ psig = some params := by
    have h1 : P.handleParamTypesAt fr.function.module h τ = some params := hparams
    simp only [Program.handleParamTypesAt, hmod, hfh, hps, bind, Option.bind_some] at h1
    exact h1
  have hnp : params.length = psig.length := ofSignature_length hpsig
  obtain ⟨_, _, _, _, _, -, -, -, -, hwr, -⟩ := hcore
  intro π hπ
  obtain ⟨hmem, hm⟩ := mem_mutRefsOf hπ
  have h1 : Value.ref π ∈ fr.stack.take params.length := List.mem_reverse.1 hmem
  obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 h1
  have hkn : k < params.length := by
    have := (List.getElem?_eq_some_iff.1 hk).1; simp at this; omega
  rw [List.getElem?_take_of_lt hkn] at hk
  obtain ⟨x, hx, hxL⟩ := abs_of_ref hd hctx.typed hctx.targs hctx.sig hctx.conf hctx.height hk
  have hH : x ∈ heldIds A := mem_heldIds.2 (Or.inr (List.mem_of_getElem? hx))
  have hxs : AbsValue.reference x ∈ A.stack.take psig.length :=
    List.mem_iff_getElem?.2 ⟨k, by rw [List.getElem?_take_of_lt (hnp ▸ hkn)]; exact hx⟩
  exact ⟨x, hH, hxL, hwr x hxs (isMutable_of (by rw [hd.complete.held_mut hH hxL, hm])), hm,
    List.mem_of_mem_take h1⟩

/-- **A new reference to a place inside a tracked one** is tracked, when the
place exists in the value there, of the declared type the reference's target
needs. -/
theorem RefOk.place {r0 : Reference} (h : RefOk P s fr r0) {X tgt : RuntimeType}
    (hX : r0.target = X) (hrig : rigid X = true) {path : List PathStep}
    (hp : RuntimeType.placeType? P X path = some tgt) (htv : tgt.isValue = true) {w : Value}
    (hw : s.deref? r0.location r0.since = some w) (hget : (w.get? path).isSome = true)
    (mutable : Bool) (hmut : mutable = true → r0.mutable = true ∧ path.all PathStep.writable = true) :
    RefOk P s fr ⟨mutable, tgt, r0.location.descend path, s.invalidations.length⟩ := by
  obtain ⟨⟨T, T0, hT, hpath, hfit⟩, hbelow, -, -, -, hwr⟩ := h
  have hT0 : T0 = r0.target := assignable_rigid hfit.assignable (by rw [hX]; exact hrig)
  refine ⟨⟨T, tgt, hT, ?_, Or.inl rfl⟩, hbelow, htv, Nat.le_refl _, ?_, fun hm => ?_⟩
  · simp only [Location.descend]
    rw [pathType_append', hpath, Option.bind_some, hT0, hX, pathType_eq_placeType]
    exact hp
  · rw [deref?_isSome]
    refine ⟨fun e he => by simp [invalidatedSince_self] at he, ?_⟩
    simp only
    rw [read?_descend, deref?_read hw]
    exact hget
  · obtain ⟨hm0, hpw⟩ := hmut hm
    simp only [Location.descend, List.all_append, hwr hm0, hpw, Bool.and_self]

/-! ## The call -/

/-- **A call of a modeled native** of handle `h` at type arguments `τ`,
`Call` or `CallGeneric`, is safe and keeps the invariant, given the analysis
runs it as `Call(h)` and the facts of the call the certificates establish. -/
theorem refSafe_nativeCallWith (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel} (hN : NativesRespect P N)
    (hinv : InvRefs P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {instr : Bytecode}
    {view : FunctionView} {A : RefState} {stys : List SignatureToken} {tys : List RuntimeType}
    {ps : List Reference} {h : FunctionHandleIndex} {τ : List RuntimeType} {c0 : FunctionRef}
    {cinfo : FunctionInfo} {fh : FunctionHandle} {params returns : List RuntimeType}
    (hctx : TopCtx P s fr info code instr view A stys tys ps)
    (hi : code.code[fr.pc]? = some instr) (hcallI : CallsHandle info.module instr h)
    (hret : instr ≠ .ret) (hctl : instr.control = .fallThrough)
    (hav : ∀ a, availabilityAfter instr a = a)
    (hrun : ∀ A', (ReferenceSafety.executeInner info.module view instr).run A = .ok ((), A') →
      (ReferenceSafety.executeInner info.module view (.call h)).run A = .ok ((), A'))
    (hattr : ReferenceSafety.borrowFieldMutable fh.attributes = none)
    (hres : P.resolve fr.function.module h = some c0)
    (hvis : P.visibleFrom info.moduleId c0 = true) (hcf : P.function? c0 = some cinfo)
    (hfh : info.module.functionHandles[h]? = some fh)
    (hlen : fh.typeParameters.length = τ.length)
    (hclen : cinfo.handle.typeParameters.length = τ.length)
    (hwfa : RuntimeType.argumentsWellFormed P τ cinfo.handle.typeParameters = true)
    (hpar' : P.paramTypes { c0 with typeArgs := τ } = some params)
    (hret' : P.returnTypes { c0 with typeArgs := τ } = some returns)
    (hparams : P.handleParamTypesAt fr.function.module h τ = some params)
    (hreturns : P.handleReturnTypesAt fr.function.module h τ = some returns)
    (hpre : RuntimeType.assignableAll (tys.take params.length) params.reverse = true)
    (hafter : typeAfter P fr.function info.module instr tys =
      some (returns.reverse ++ tys.drop params.length))
    {k : NativeShape} (hshape : P.nativeShape? c0 = some k) :
    ExecSafe P N (InvRefs P) (Step.call P s fr rest info h τ) := by
  have hr := hinv.2
  obtain ⟨info', e, hf', hcode, -⟩ := nativeShape_entry hshape
  rw [hcf, Option.some.injEq] at hf'
  subst hf'
  have hcf' : P.function? { c0 with typeArgs := τ } = some cinfo := by
    rw [function?_typeArgs]; exact hcf
  have htok : P.typeArgsOk { c0 with typeArgs := τ } = true := by
    simp only [Program.typeArgsOk, hcf']; exact hwfa
  have hτ : ∀ x ∈ τ, x.isValue = true := typeArgsOk_values htok
  have hshape' : P.nativeShape? { c0 with typeArgs := τ } = some k :=
    (nativeShape?_typeArgs c0 τ).trans hshape
  have hstack := hctx.conf
  have hlen' := conformsAll_length hstack
  have htl := assignableAll_length hpre
  have hge : params.length ≤ fr.stack.length := by
    rw [hlen']; simp only [List.length_take, List.length_reverse] at htl; omega
  have hargs : conformsAll P (fr.stack.take params.length).reverse params = true := by
    have := conformsAll_reverse
      (conformsAll_assignable (conformsAll_take params.length hstack) hpre)
    rwa [List.reverse_reverse] at this
  have hcid : fr.id < s.nextFrame := hr.below fr (by rw [hsf]; exact List.mem_cons_self)
  obtain ⟨S, hglob, -⟩ := hctx.chain _ hsf
  obtain ⟨-, hexcl, hargOk, -, -⟩ := call_suspendAt hwf hctx hact.locals
    (typeArgsOk_values hact.typeArgs) hi hcallI hret hctl hrun hact.fn
    hact.body hfh hattr hres hcf hτ hparams hreturns hret' hge hcid hsf hr.active.counted hglob
  have hderef : ∀ l since, ({ s with frames := { fr with stack := fr.stack.drop params.length } ::
      rest } : State).deref? l since = s.deref? l since :=
    fun l since => deref?_congr (root?_withStack' hsf _ l.root) rfl since
  obtain ⟨views, hviews, -, hvi⟩ := nativeView_some
    (s := { s with frames := { fr with stack := fr.stack.drop params.length } :: rest })
    (args := (fr.stack.take params.length).reverse) fun r hr' => by
      rw [hderef]; exact Option.isSome_iff_exists.1 (hargOk r (mem_argRefs.2 hr')).valid
  have hexA := exclusive_of_compat hexcl
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  have hW0 : WriteCtx P s fr rest info code A ps L :=
    ⟨hsf, hinv.1, hr, hact, hchain, hctx.ref, hd, hrefs, hctx.chain _ hsf, hctx.roots⟩
  have hmw := mutArgs_writable hctx hd hret hrun hact.fn hfh hattr hparams
  simp only [Step.call, Step.need, hres, hvis, hcf, hfh, hlen, hclen, hwfa, hpar', hret',
    hparams, hreturns, Step.popN, Nat.not_lt.2 hge, hargs, bne_self_eq_false, Bool.or_false,
    Bool.not_true, Bool.false_eq_true, ↓reduceIte, bind, Except.bind, pure, Except.pure, hcode,
    Option.isNone_none, hshape', Option.isNone_some, Bool.true_and, Bool.false_and, hviews, hexA]
  intro r hNr
  cases r with
  | failed e => trivial
  | returned vals writes places =>
  obtain ⟨tys', htys', hfits⟩ := hN _ _ _ _ _ hNr
  rw [hret', Option.some.injEq] at htys'
  subst htys'
  -- Resuming, from the state the writes left.
  have hfin : ∀ (V : State) (frV : Frame) (restV : List Frame),
      WriteCtx P V frV restV info code A ps L → frV.stack = fr.stack → frV.pc = fr.pc →
      frV.function = fr.function →
      writeBack { s with frames := { fr with stack := fr.stack.drop params.length } :: rest }
        (P.writtenReferences { c0 with typeArgs := τ } (fr.stack.take params.length).reverse)
        writes = some { V with frames := { frV with stack := fr.stack.drop params.length } :: restV } →
      ∀ rv, (placeReferences { V with frames := { frV with stack := fr.stack.drop params.length } ::
          restV } returns (fr.stack.take params.length).reverse places).bind
          (nativeResultsIn returns vals) = some rv →
      conformsAll P rv returns = true →
      (∀ r ∈ argRefs rv, ∃ π ∈ argRefs (fr.stack.take params.length).reverse,
        r.location.Inside π.location ∧ (r.mutable = true → π.mutable = true)) →
      (argRefs rv).Pairwise Compat →
      (∀ r ∈ argRefs rv, ∀ k, r.location ≠ ⟨.held k, []⟩) →
      (∀ r ∈ argRefs rv, RefOk P { V with frames := resumedFrame frV rv params.length :: restV }
        (resumedFrame frV rv params.length) r) →
      Safe (InvRefs P) (resumeNative P
        { s with frames := { fr with stack := fr.stack.drop params.length } :: rest }
        { c0 with typeArgs := τ } (fr.stack.take params.length).reverse
        (.returned vals writes places)) := by
    intro V frV restV hW hstkV hpcV hfunV hwb rv hpr hconf hrc1 hrc2 hheldNew hnew
    simp only [resumeNative, hret', hviews, hfits, Bool.not_true, Bool.false_eq_true, ↓reduceIte,
      hwb, hpr]
    rw [← hstkV]
    exact invRefs_nativeResume hwf hc hW (TopCtx.ofWriteCtx hctx hW hfunV hpcV hstkV)
      (by rw [hpcV]; exact hi) hcallI hret hctl hav hrun hfh hattr (by rw [hfunV]; exact hres) hcf
      hτ (by rw [hfunV]; exact hparams) (by rw [hfunV]; exact hreturns) hret'
      (by rw [hstkV]; exact hge) (by rw [hfunV, hpcV]; exact hctx.tc) (by rw [hfunV]; exact hafter)
      rfl hconf (by rw [hstkV]; exact hrc1) hrc2 hheldNew hnew
  by_cases hkb : k = .borrow
  · -- A borrow: no writes, and a reference to a place inside the first argument.
    subst hkb
    obtain ⟨rfl, rfl, m, am, X, tgt, p, rfl, rfl, harg, hpX, hmam, hrig, hXt, hplace, hmw', view0,
      hv0, hget⟩ := borrow_result hshape' hpar' hret' hτ hfits
    obtain ⟨r0, hra, hrm, hrX⟩ := conformsAll_ref_at hargs hpX
    have hrX' : r0.target = X := assignable_rigid hrX hrig
    have hr0 : r0 ∈ argRefs (fr.stack.take params.length).reverse :=
      mem_argRefs.2 (List.mem_of_getElem? hra)
    have h0 := hargOk r0 hr0
    have hw0 : s.deref? r0.location r0.since = some view0 := by
      rw [← hderef, ← hvi 0 r0 hra, hv0]
    have htv : tgt.isValue = true := by
      exact borrow_value hXt (hrX' ▸ h0.value)
    have hp0 : p.path ≠ [] := by
      intro hnil
      rw [hnil, placeType?_nil, Option.some.injEq] at hplace
      subst hplace
      exact borrow_ne hXt rfl
    have hok := h0.place hrX' hrig hplace htv hw0 hget m fun hm => ⟨by rw [hrm]; exact hmam hm,
      hmw' hm⟩
    have hpr : (placeReferences { s with frames := { fr with stack := fr.stack.drop params.length } ::
        rest } [.ref m tgt] (fr.stack.take params.length).reverse [p]).bind
        (nativeResultsIn [.ref m tgt] []) = some [Step.borrow
          { s with frames := { fr with stack := fr.stack.drop params.length } :: rest } m tgt
          (r0.location.descend p.path)] := by
      simp [placeReferences, nativeResultsIn, harg, hra, RuntimeType.isValue]
    refine hfin s fr rest hW0 rfl rfl rfl ?_ _ hpr ?_ ?_ ?_ ?_ ?_
    · rw [writtenReferences_other (by rw [hshape']; simp)]
      rfl
    · simp [conformsAll, Step.borrow, Value.type, Value.wellFormed, assignable_refl, htv]
    · intro r' hr'
      simp only [argRefs, Step.borrow, List.filterMap_cons, List.filterMap_nil,
        List.mem_singleton] at hr'
      subst hr'
      exact ⟨r0, hr0, ⟨rfl, List.prefix_append _ _⟩, fun hm => by rw [hrm]; exact hmam hm⟩
    · simp [argRefs, Step.borrow]
    · intro r' hr' k' he
      simp only [argRefs, Step.borrow, List.filterMap_cons, List.filterMap_nil,
        List.mem_singleton] at hr'
      subst hr'
      simp only [Location.descend, Location.mk.injEq] at he
      exact hp0 (List.append_eq_nil_iff.1 he.2).2
    · intro r' hr'
      simp only [argRefs, Step.borrow, List.filterMap_cons, List.filterMap_nil,
        List.mem_singleton] at hr'
      subst hr'
      exact hok.same hsf rfl rfl rfl rfl rfl rfl rfl
  · -- Values: written back through the mutable arguments, for an update.
    obtain ⟨hcv, rfl⟩ := fits_values hshape' hkb hfits
    have hvt := returns_values hshape' hkb hret' hτ
    have hnr : argRefs vals = [] := List.eq_nil_iff_forall_not_mem.2 fun r hr' =>
      noRef_of_conformsAll hcv hvt _ (mem_argRefs.1 hr') r rfl
    have hpr : ∀ S : State, (placeReferences S returns (fr.stack.take params.length).reverse
        []).bind (nativeResultsIn returns vals) = some vals := fun S => by
      rw [placeReferences_nil, Option.bind_some, nativeResultsIn_values hvt (conformsAll_length hcv)]
    by_cases hku : k = .update
    · subst hku
      have hfit' := hfits
      simp only [Program.nativeResultFits, hshape', hpar', Bool.and_eq_true] at hfit'
      obtain ⟨V, frV, restV, hW, h1, h2, -, h4, -, -, -, -, hwb⟩ :=
        writeCtx_writeBack (mutRefsOf _) writes hW0 hmw (mutRefs_disjoint hexA)
          (by rw [mutRefs_targets hargs]; exact hfit'.1.2)
      refine hfin V frV restV hW h1 h2 h4 ?_ vals (hpr _) hcv (by simp [hnr]) (by rw [hnr]; simp)
        (by simp [hnr]) (by simp [hnr])
      rw [writtenReferences_update hshape']
      exact hwb _
    · refine hfin s fr rest hW0 rfl rfl rfl ?_ vals (hpr _) hcv (by simp [hnr]) (by rw [hnr]; simp)
        (by simp [hnr]) (by simp [hnr])
      rw [writtenReferences_other (by rw [hshape']; simpa using hku),
        fits_writes_nil hshape' hku hfits]
      rfl

end MoveBytecode.Proofs
