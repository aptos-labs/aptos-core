-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefTracked
import Proofs.NativeAnalysis

/-!
# Containment in the current frame

UNTRUSTED (checked by the kernel). No access of a step of a verified program
reaches over a `&mut` reference the current frame tracks (`contained_top`,
D4b′ within the frame). An access is an inspection, which reaches over
nothing; a read or write through a reference, which the analysis checked
readable or writable, so nothing mutable it tracks is strictly inside
(`Complete.readable_inside`, `Complete.writable_inside`; `MoveTo` reads an
immutable `&signer`, inside which nothing tracked is mutable); a direct access
to a local, which the analysis checked unborrowed (mutably, for `CopyLoc`), or
which holds a reference, so nothing tracked is into it; or a direct write to
a resource, which `MoveFrom` makes only when the analysis found its type
unborrowed, and `MoveTo` only when no resource is there, so no valid
reference is into it.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety MoveBytecode.Step

/-- A core step, unfolded: the instruction, its execution and its annotation. -/
private theorem ct_parts {P : Program} {s : State} {o : Outcome} {h : Shadow}
    {post : ShadowPost} {evs : List Event} (hstep : step P s = .done o)
    (hc : Shadow.coreStep P s o h = some (post, evs)) :
    ∃ fr rest info code instr, s.frames = fr :: rest ∧ P.function? fr.function = some info ∧
      info.definition.code = some code ∧ code.code[fr.pc]? = some instr ∧
      stepOf (Step.exec P s fr rest info code instr) = .done o ∧
      Shadow.exec P info.moduleId instr s fr o h = some (post, evs) := by
  obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hex⟩ := coreStep_exec hc
  refine ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, ?_, hex⟩
  simp only [step, hsf, hinfo, hcode, hinstr] at hstep
  exact hstep


/-! ## Reaching over -/

theorem ct_strictlyInside {l m : Location} (h : l.strictlyInside m = true) :
    l.StrictlyInside m := by
  simp only [Location.strictlyInside, Bool.and_eq_true, beq_iff_eq, bne_iff_ne, ne_eq,
    List.isPrefixOf_iff_prefix] at h
  exact ⟨⟨h.1.1, h.1.2⟩, fun e => h.2 e.symm⟩

/-- An access through a reference reaches over only what is strictly inside
its location. -/
theorem ct_reach_via {a : Access} {l m : Location} {t : Nat} (h : ¬ m.StrictlyInside l) :
    Access.reachesOver a l (some t) m = false := by
  have key : m.strictlyInside l = false := by
    cases hb : m.strictlyInside l
    · rfl
    · exact absurd (ct_strictlyInside hb) h
  cases a <;> simp [Access.reachesOver, key]

/-- A direct access to a root reaches over only what is rooted there. -/
theorem ct_reach_direct {a : Access} {ρ : Root} {m : Location} (h : m.root ≠ ρ) :
    Access.reachesOver a ⟨ρ, []⟩ none m = false := by
  have key : (ρ == m.root) = false := by simpa using fun e => h e.symm
  cases a <;> simp [Access.reachesOver, Access.overlaps, key]

/-! ## The analysis's checks -/

/-- Nothing held and mutable is into a local the frame root does not borrow
mutably. -/
theorem ct_local_mut_unborrowed {g : Graph} {L : Nat → Option Reference} {H : Nat → Prop}
    {root : Nat} {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}
    (hc : Complete g L H root fid gsid ps) {i : Nat}
    (hb : g.hasConsistentMutableBorrows root (some (.local_ i)) = .ok false) {x : Nat}
    {rx : Reference} (hx : H x) (hxL : L x = some rx) (hxm : rx.mutable = true) :
    rx.location.root ≠ .local_ fid i := by
  intro hr
  obtain ⟨e, n, rn, he, hl, hn, -, hrest⟩ := hc.rootLocal x rx i hx hxL hr
  have hnm := Graph.hasConsistentMutableBorrows_false hb he hl
  rw [hc.held_mut (hc.rootChain_first_held he hn) hn] at hnm
  have : rn.mutable = true := by
    rcases hrest with rfl | hrest
    · rw [hn, Option.some.injEq] at hxL; subst hxL; exact hxm
    · exact hrest.mutable_src hn hxL hxm
  simp [this] at hnm

/-- The analysis compares references only when both are readable. -/
theorem ct_comparison {v1 v2 r : AbsValue} {st st' : RefState}
    (h : (comparison v1 v2).run st = .ok (r, st')) :
    (v1 = .nonReference ∧ v2 = .nonReference) ∨
      ∃ id1 id2, v1 = .reference id1 ∧ v2 = .reference id2 ∧
        st.graph.isReadable id1 none = .ok true ∧ st.graph.isReadable id2 none = .ok true := by
  unfold comparison at h
  split at h
  · next id1 id2 =>
    simp only [run_bind] at h
    obtain ⟨b1, st₁, hb1, h⟩ := h
    obtain ⟨hr1, hst₁⟩ := isReadable_run.1 hb1
    rw [hst₁] at h
    obtain ⟨b2, st₂, hb2, h⟩ := h
    obtain ⟨hr2, hst₂⟩ := isReadable_run.1 hb2
    rw [hst₂] at h
    split at h
    · exact absurd h reject_bind_run
    · next hc =>
      simp only [Bool.or_eq_true, Bool.not_eq_true', not_or, Bool.not_eq_false] at hc
      rw [hc.1] at hr1
      rw [hc.2] at hr2
      exact Or.inr ⟨id1, id2, rfl, rfl, hr1, hr2⟩
  · exact Or.inl ⟨rfl, rfl⟩
  · simp [panicAt, StateT.run, liftM, monadLift, MonadLift.monadLift, StateT.lift, bind,
      StateT.bind, Except.bind] at h
  · simp [panicAt, StateT.run, liftM, monadLift, MonadLift.monadLift, StateT.lift, bind,
      StateT.bind, Except.bind] at h

/-- `Eq` and `Neq` on references check both readable. -/
theorem ct_exec_eq {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {instr : Bytecode} (hi : instr = .eq ∨ instr = .neq)
    (hx : (executeInner m view instr).run st = .ok ((), st')) :
    ∃ v1 v2 rest, st.stack = v1 :: v2 :: rest ∧
      ((v1 = .nonReference ∧ v2 = .nonReference) ∨
        ∃ id1 id2, v1 = .reference id1 ∧ v2 = .reference id2 ∧
          st.graph.isReadable id1 none = .ok true ∧ st.graph.isReadable id2 none = .ok true) := by
  have hx' : (do
      let v1 ← ReferenceSafety.pop
      let v2 ← ReferenceSafety.pop
      ReferenceSafety.push (← comparison v1 v2) : RefM Unit).run st = .ok ((), st') := by
    rcases hi with rfl | rfl <;> simpa [executeInner] using hx
  rw [run_bind] at hx'
  obtain ⟨v1, st₁, hp1, hx'⟩ := hx'
  obtain ⟨rest₁, hs₁, rfl⟩ := pop_run.1 hp1
  rw [run_bind] at hx'
  obtain ⟨v2, st₂, hp2, hx'⟩ := hx'
  obtain ⟨rest, hs₂, rfl⟩ := pop_run.1 hp2
  rw [run_bind] at hx'
  obtain ⟨r, st₃, hc, -⟩ := hx'
  simp only at hs₂
  subst hs₂
  have hcmp := ct_comparison hc
  exact ⟨v1, v2, rest, hs₁, hcmp⟩

/-- `VecPopBack` checks its reference writable. -/
theorem ct_exec_vecPopBack {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {sig : Nat} (hx : (executeInner m view (.vecPopBack sig)).run st = .ok ((), st')) :
    ∃ id rest, st.stack = .reference id :: rest ∧ st.graph.isWritable id = .ok true := by
  simp only [executeInner, run_bind] at hx
  obtain ⟨v, st₁, hp, hx⟩ := hx
  obtain ⟨rest, hs, rfl⟩ := pop_run.1 hp
  obtain ⟨u, st₂, hop, -⟩ := hx
  obtain ⟨id, g, rfl, hw, -, -⟩ := vectorOp_run hop
  exact ⟨id, rest, hs, hw rfl⟩


/-! ## The resource a global storage operation writes -/

/-- `MoveFrom` writes the resource of the type of the value it moves out, at
the address on top of the stack. -/
theorem ct_moveFrom_loc {P : Program} {m : ModuleId} {instr : Bytecode} {s : State}
    (hi : (∃ d, instr = .moveFrom d) ∨ ∃ i, instr = .moveFromGeneric i)
    {fr : Frame} {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event}
    {g : ShadowFrame} {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) {l : Location}
    (ha : Event.accessed .write l none ∈ evs) :
    ∃ a v sid, fr.stack.head? = some (.address a) ∧ Shadow.topAfter o = some v ∧
      v.type = .struct sid ∧ l = ⟨.global a sid, []⟩ := by
  rcases hi with ⟨d, rfl⟩ | ⟨i, rfl⟩
  all_goals
    simp only [Shadow.exec, hh, Option.bind_eq_bind] at hex
    split at hex
    · next a ha' =>
      simp only [Option.bind_eq_some_iff] at hex
      obtain ⟨v, hv, hex⟩ := hex
      split at hex
      · next sid hsid =>
        simp only [Option.bind_eq_some_iff] at hex
        obtain ⟨k, -, t, -, -, rfl⟩ := hex
        simp only [List.mem_cons, Event.accessed.injEq, reduceCtorEq, List.not_mem_nil,
          or_false] at ha
        exact ⟨a, v, sid, ha', hv, hsid, ha.2.1⟩
      · cases hex
    · cases hex

/-- `MoveTo` writes the resource of the type of the value it moves in, at the
address of the signer below it. -/
theorem ct_moveTo_loc {P : Program} {m : ModuleId} {instr : Bytecode} {s : State}
    (hi : (∃ d, instr = .moveTo d) ∨ ∃ i, instr = .moveToGeneric i)
    {fr : Frame} {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event}
    {g : ShadowFrame} {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) {l : Location}
    (ha : Event.accessed .write l none ∈ evs) :
    ∃ v sid rs a, fr.stack.head? = some v ∧ v.type = .struct sid ∧
      fr.stack[1]? = some (.ref rs) ∧ s.deref? rs.location rs.since = some (.signer a) ∧
      l = ⟨.global a sid, []⟩ := by
  rcases hi with ⟨d, rfl⟩ | ⟨i, rfl⟩
  all_goals
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      ] at hex
    obtain ⟨tv, -, v, hv, hm⟩ := hex
    split at hm
    · next sid hsid =>
      simp only [Option.bind_eq_some_iff] at hm
      obtain ⟨x, hra, hm⟩ := hm
      split at hm
      · next a hd =>
        simp only [Option.some.injEq, Prod.mk.injEq] at hm
        obtain ⟨-, rfl⟩ := hm
        simp only [List.mem_cons, Event.accessed.injEq, reduceCtorEq, List.not_mem_nil, or_false,
          false_or, false_and, true_and, and_true] at ha
        obtain ⟨h1, -, -⟩ := refAt_spec hra
        exact ⟨v, sid, x.2, a, hv, hsid, h1, hd, ha⟩
      · cases hm
    · cases hm


/-! ## The global storage operations, as they run -/

theorem ct_unlocked {s : State} {sid : StructId} {u : Unit} (h : Step.unlocked s sid = .ok u) :
    ¬ s.activeCount sid.module > 1 := by
  unfold Step.unlocked at h
  split at h
  · simp [Step.fail] at h
  · assumption

/-- A `MoveFrom` that continues: the type, the address, the resource moved
out, in a module active at most once. -/
theorem ct_moveFrom_stepAt {P : Program} {s : State} {fr : Frame} {rest : List Frame}
    {d : StructDefinitionIndex} {tys : List RuntimeType} {o : Outcome}
    (h : stepOf (Step.moveResourceFrom P s fr rest d tys) = .done o)
    (hcont : o.continues = true) :
    ∃ sid a v st, globalType P fr.function.module d tys = .ok sid ∧ fr.stack = .address a :: st ∧
      ¬ s.activeCount sid.module > 1 ∧ s.storage.get? a sid = some v ∧
      Shadow.topAfter o = some v := by
  obtain ⟨sid, hsid, h⟩ := done_bind h hcont globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h hcont popAddress_ends
  obtain ⟨st, hstk, rfl⟩ := popAddress_ok hpa
  obtain ⟨u, hu, h⟩ := done_bind h hcont unlocked_ends
  split at h
  · next v hget =>
    have ho := done_finish rfl h
    subst ho
    exact ⟨sid, a, v, st, hsid, hstk, ct_unlocked hu, hget, by
      simp [Shadow.topAfter, advance, pushAll]⟩
  · exact (done_fail h hcont).elim

/-- A `MoveTo` that continues: the value moved in, of the type, and the
immutable `&signer` below it, whose address holds no resource of the type. -/
theorem ct_moveTo_stepAt {P : Program} {s : State} {fr : Frame} {rest : List Frame}
    {d : StructDefinitionIndex} {tys : List RuntimeType} {o : Outcome}
    (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.moveResourceTo P s fr rest d tys) = .done o)
    (hcont : o.continues = true) :
    ∃ sid v rs st a, fr.stack = v :: .ref rs :: st ∧ rs.mutable = false ∧
      v.type = .struct sid ∧ s.deref? rs.location rs.since = some (.signer a) ∧
      s.storage.get? a sid = none := by
  obtain ⟨sid, -, h⟩ := done_bind h hcont globalType_ends
  obtain ⟨⟨v, fr₁⟩, hpv, h⟩ := done_bind h hcont pop_ends
  obtain ⟨st₁, hst₁, rfl⟩ := pop_ok hpv
  obtain ⟨⟨rs, fr₂⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
  obtain ⟨st, hst₂, rfl, -, -⟩ := popRefTo_ok hpr
  simp only at hst₂
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  next hmut =>
  split at h
  · exact (done_wrong h hcont).elim
  next hconf =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hconf
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨sv, hsv, h⟩ := done_bind h hcont deref_ends
  split at h
  · next a =>
    obtain ⟨_, -, h⟩ := done_bind h hcont unlocked_ends
    split at h
    · exact (done_fail h hcont).elim
    · next hnone =>
      have hd := deref_ok hsv
      rw [deref_eq (s' := { s with frames := { fr with stack := st } :: rest }) (s := s)
        (by simp [hsf]) rfl rfl rfl] at hd
      refine ⟨sid, v, rs, st, a, by rw [hst₁, hst₂], by simpa using hmut,
        conforms_type' hconf rfl, hd, by simpa using hnone⟩
  · exact (done_wrong h hcont).elim

/-- A continuing `MoveFrom`, generic or not: its definition and type arguments,
and the resource moved out of the instance. -/
theorem ct_moveFrom_step {P : Program} {s : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {instr : Bytecode}
    (hi : (∃ d, instr = .moveFrom d) ∨ ∃ i, instr = .moveFromGeneric i) {o : Outcome}
    (h : stepOf (Step.exec P s fr rest info code instr) = .done o)
    (hcont : o.continues = true) :
    ∃ d tys, ((instr = .moveFrom d ∧ tys = []) ∨ ∃ i x, instr = .moveFromGeneric i ∧
        info.module.structDefInstantiations[i]? = some x ∧ x.def_ = d ∧
        Step.genericArgs info.module fr.function.typeArgs x.typeParameters = .ok tys) ∧
      ∃ sid a v st, globalType P fr.function.module d tys = .ok sid ∧
        fr.stack = .address a :: st ∧ ¬ s.activeCount sid.module > 1 ∧
        s.storage.get? a sid = some v ∧ Shadow.topAfter o = some v := by
  rcases hi with ⟨d, rfl⟩ | ⟨i, rfl⟩
  · exact ⟨d, [], Or.inl ⟨rfl, rfl⟩, ct_moveFrom_stepAt h hcont⟩
  · simp only [Step.exec] at h
    obtain ⟨x, hx, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, htys, h⟩ := done_bind h hcont genericArgs_ends
    exact ⟨x.def_, tys, Or.inr ⟨i, x, rfl, need_ok hx, rfl, htys⟩, ct_moveFrom_stepAt h hcont⟩

/-- A continuing `MoveTo`, generic or not. -/
theorem ct_moveTo_step {P : Program} {s : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {instr : Bytecode}
    (hi : (∃ d, instr = .moveTo d) ∨ ∃ i, instr = .moveToGeneric i) {o : Outcome}
    (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code instr) = .done o)
    (hcont : o.continues = true) :
    ∃ sid v rs st a, fr.stack = v :: .ref rs :: st ∧ rs.mutable = false ∧
      v.type = .struct sid ∧ s.deref? rs.location rs.since = some (.signer a) ∧
      s.storage.get? a sid = none := by
  rcases hi with ⟨d, rfl⟩ | ⟨i, rfl⟩
  · exact ct_moveTo_stepAt hsf h hcont
  · simp only [Step.exec] at h
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact ct_moveTo_stepAt hsf h hcont

/-- The definition of a global type names the type without its arguments. -/
theorem globalType_erase {P : Program} {mi d : Nat} {tys : List RuntimeType} {sid : StructId}
    (h : Step.globalType P mi d tys = .ok sid) : P.definedStruct? mi d = some sid.erase := by
  obtain ⟨sid₀, h1, rfl⟩ := globalType_ok h
  have h0 := definedStruct?_typeArgs h1
  rw [h1]
  cases sid₀
  simp_all [StructIdOf.erase]

/-- A `MoveFrom`, generic or not, is the analysis's `MoveFrom` of its
definition, which the function acquires. -/
theorem moveFrom_analysis {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit} (hf : P.function? f = some info)
    (hcode : info.definition.code = some code) {pc : Nat} {instr : Bytecode}
    (hinstr : code.code[pc]? = some instr) {d : Nat} {tys : List RuntimeType}
    {τ : List RuntimeType}
    (hdcase : (instr = .moveFrom d ∧ tys = []) ∨ ∃ i x, instr = .moveFromGeneric i ∧
        info.module.structDefInstantiations[i]? = some x ∧ x.def_ = d ∧
        Step.genericArgs info.module τ x.typeParameters = .ok tys)
    {view : FunctionView} {A A' : RefState}
    (hx : (executeInner info.module view instr).run A = .ok ((), A')) :
    (executeInner info.module view (.moveFrom d)).run A = .ok ((), A') ∧
      d ∈ info.definition.acquiresGlobalResources := by
  have hacq := function_acquires hwf hf hcode
  rcases hdcase with ⟨rfl, -⟩ | ⟨i, x, rfl, hx0, rfl, -⟩
  · exact ⟨hx, (acquires_instr hacq (getElem?_mem_toList hinstr)).1 _ (Or.inl rfl)⟩
  · obtain ⟨si, hsi, hx'⟩ := exec_moveFromGeneric' hx
    rw [hx0, Option.some.injEq] at hsi
    subst hsi
    exact ⟨hx', acquires_instrGeneric hacq (getElem?_mem_toList hinstr) (Or.inl rfl) hx0⟩

/-! ## Containment in the current frame -/

/-- **A tracked reference of the current frame is held by the analysis**: on
the operand stack, or in a local the availability analysis has available. -/
theorem tracked_top_held {P : Program} {s : State} {fr : Frame} {info : FunctionInfo}
    {code : CodeUnit} {instr : Bytecode} {view : FunctionView} {A : RefState}
    {stys : List SignatureToken} {tys : List RuntimeType} {ps : List Reference}
    (hctx : TopCtx P s fr info code instr view A stys tys ps)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    {L : Nat → Option Reference} (hd : Desc A fr (gsidOf P (s.frames.map Frame.skel)) ps L)
    {g : ShadowFrame} {i : Nat} {r : Reference} (ht : TrackedIn P g fr i r) :
    ∃ y, y ∈ heldIds A ∧ L y = some r := by
  rcases ht with ⟨t, -, hz⟩ | ⟨j, ⟨t, -, -, hj⟩, hav⟩
  · obtain ⟨k, -, hk⟩ := mem_zip_getElem? hz
    obtain ⟨x, hx', hxL⟩ := abs_of_ref hd hctx.typed hctx.targs hctx.sig hctx.conf hctx.height hk
    exact ⟨x, mem_heldIds.2 (Or.inr (List.mem_of_getElem? hx')), hxL⟩
  · obtain ⟨t', ht', hkind⟩ := localToken_ref hact.fn hact.body hctx.view hact.locals (typeArgsOk_values hact.typeArgs) hj
    obtain ⟨x, hx'⟩ := hctx.typed.locKind j t' r.mutable ht' hkind hav
    obtain ⟨r', hr', hxL⟩ := hd.locals j x hx'
    rw [hj, Option.some.injEq, Option.some.injEq, Value.ref.injEq] at hr'
    subst hr'
    exact ⟨x, mem_heldIds.2 (Or.inl (List.mem_of_getElem? hx')), hxL⟩

/-- **No access of a verified step reaches over a tracked `&mut` reference of
the current frame** (D4b′ within the frame). -/
theorem contained_top {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) (hsigs : P.ApiSigs) {s : State} (hinv : InvRefs P s) {h : Shadow}
    (hpre : Pre false P s h) (_htyped : StateTyped P s) {o : Outcome} {post : ShadowPost}
    {evs : List Event} (hstep : step P s = .done o) (hc : Shadow.coreStep P s o h = some (post, evs))
    {g : ShadowFrame} {hs' : List ShadowFrame} {fr : Frame} {rest : List Frame}
    (hh : h.frames = g :: hs') (hsf : s.frames = fr :: rest)
    {a : Access} {l : Location} {via : Option Nat} (ha : Event.accessed a l via ∈ evs)
    {i : Nat} {r : Reference} (ht : TrackedIn P g fr i r) (hm : r.mutable = true) :
    Access.reachesOver a l via r.location = false := by
  obtain ⟨fr₀, rest₀, info, code, instr, hsf₀, hinfo, hcode, hinstr, hstep', hex⟩ :=
    ct_parts hstep hc
  rw [hsf, List.cons.injEq] at hsf₀
  obtain ⟨rfl, rfl⟩ := hsf₀
  have hcont : o.continues = true := coreStep_continues hc
  -- The analysis at the current instruction.
  obtain ⟨fr₁, rest₁, hsf₁, ⟨info₁, code₁, hact⟩, -⟩ := hinv.1
  rw [hsf, List.cons.injEq] at hsf₁
  obtain ⟨rfl, -⟩ := hsf₁
  have hi₁ : info₁ = info := by rw [hact.fn] at hinfo; exact Option.some.inj hinfo
  subst hi₁
  have hc₁ : code₁ = code := by rw [hact.body] at hcode; exact Option.some.inj hcode
  subst hc₁
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtxGen hwf hs hsigs hinv hsf hact hinstr
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  obtain ⟨A', hx⟩ := hctx.exec
  -- The tracked reference is held by the analysis.
  obtain ⟨y, hy, hyL⟩ := tracked_top_held hctx hact hd ht
  have hok := hrefs y r hy hyL
  -- The analysis's identity of a reference on the operand stack.
  have habs : ∀ {k : Nat} {rt : Reference}, fr.stack[k]? = some (.ref rt) →
      ∃ x, A.stack[k]? = some (.reference x) ∧ x ∈ heldIds A ∧ L x = some rt := by
    intro k rt hk
    obtain ⟨x, hx', hxL⟩ := abs_of_ref hd hctx.typed hctx.targs hctx.sig hctx.conf hctx.height hk
    exact ⟨x, hx', mem_heldIds.2 (Or.inr (List.mem_of_getElem? hx')), hxL⟩
  -- Nothing tracked is into a local holding a reference.
  have hnotref : ∀ {j y' : Nat}, A.locals[j]? = some (.reference y') →
      r.location.root ≠ .local_ fr.id j := by
    intro j y' hj' e
    obtain ⟨ry, hry, -⟩ := hd.locals j y' hj'
    obtain ⟨T, hT, hTr⟩ := localType_ref hact.locals hry
    exact hok.not_into_ref (frame?_head hsf) hT hTr j e rfl
  cases exec_accessed hh hex ha with
  | inspect => rfl
  | @viaRef a' k t rt _ hk _ hop =>
    apply ct_reach_via
    obtain ⟨x, hxk, hxH, hxL⟩ := habs hk
    have hread : A.graph.isReadable x none = .ok true → ¬ r.location.StrictlyInside rt.location :=
      fun hrd => hd.complete.readable_inside hxH hxL hrd hy hyL hm
    have hwrite : A.graph.isWritable x = .ok true → ¬ r.location.StrictlyInside rt.location :=
      fun hwr => hd.complete.writable_inside hxH hxL hwr hy hyL
    unfold RefOperand at hop
    rcases hop with ⟨hi, hk1, rfl⟩ | ⟨rfl, rfl, rfl⟩ | ⟨rfl, rfl, rfl⟩ | ⟨⟨sig, rfl⟩, rfl, rfl⟩ |
      ⟨⟨sig, rfl⟩, rfl, rfl⟩ | ⟨⟨sig, rfl⟩, rfl, rfl⟩ | ⟨hmt, rfl, rfl⟩
    · -- `Eq`, `Neq`: both operands checked readable.
      obtain ⟨v1, v2, rest', hst, hcase⟩ := ct_exec_eq hi hx
      rw [hst] at hxk
      rcases hcase with ⟨rfl, rfl⟩ | ⟨id1, id2, rfl, rfl, hr1, hr2⟩
      · cases k with
        | zero => simp at hxk
        | succ k =>
          cases k with
          | zero => simp at hxk
          | succ k => omega
      · cases k with
        | zero =>
          simp only [List.getElem?_cons_zero, Option.some.injEq, AbsValue.reference.injEq] at hxk
          subst hxk
          exact hread hr1
        | succ k =>
          cases k with
          | zero =>
            simp only [List.getElem?_cons_succ, List.getElem?_cons_zero, Option.some.injEq,
              AbsValue.reference.injEq] at hxk
            subst hxk
            exact hread hr2
          | succ k => omega
    · obtain ⟨id, rest', g', hst, hrd, -, -⟩ := exec_readRef hx
      rw [hst] at hxk
      simp only [List.getElem?_cons_zero, Option.some.injEq, AbsValue.reference.injEq] at hxk
      subst hxk
      exact hread hrd
    · obtain ⟨id, rest', g', hst, hwr, -, -⟩ := exec_writeRef hx
      rw [hst] at hxk
      simp only [List.getElem?_cons_zero, Option.some.injEq, AbsValue.reference.injEq] at hxk
      subst hxk
      exact hwrite hwr
    · obtain ⟨id, rest', g', hst, hwr, -, -⟩ := exec_vecPushBack hx
      rw [hst] at hxk
      simp only [List.getElem?_cons_succ, List.getElem?_cons_zero, Option.some.injEq,
        AbsValue.reference.injEq] at hxk
      subst hxk
      exact hwrite hwr
    · obtain ⟨id, rest', hst, hwr⟩ := ct_exec_vecPopBack hx
      rw [hst] at hxk
      simp only [List.getElem?_cons_zero, Option.some.injEq, AbsValue.reference.injEq] at hxk
      subst hxk
      exact hwrite hwr
    · obtain ⟨id, rest', g', hst, hwr, -, -⟩ := exec_vecSwap hx
      rw [hst] at hxk
      simp only [List.getElem?_cons_succ, List.getElem?_cons_zero, Option.some.injEq,
        AbsValue.reference.injEq] at hxk
      subst hxk
      exact hwrite hwr
    · -- `MoveTo`, generic or not, reads through an immutable `&signer`.
      obtain ⟨-, v, rs, st, -, hstk, himm, -, -, -⟩ := ct_moveTo_step hmt hsf hstep' hcont
      rw [hstk] at hk
      simp only [List.getElem?_cons_succ, List.getElem?_cons_zero, Option.some.injEq,
        Value.ref.injEq] at hk
      subst hk
      intro hin
      have := hd.complete.immInside x _ y r hxH hy hxL hyL himm hin
      rw [hm] at this
      cases this
  | @direct a' j hop =>
    apply ct_reach_direct
    unfold LocalOperand at hop
    rcases hop with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
    · obtain ⟨va, hva⟩ := copyLoc_local hx
      cases va with
      | nonReference =>
        exact ct_local_mut_unborrowed hd.complete (exec_copyLoc_val hva hx).1 hy hyL hm
      | reference y' => exact hnotref hva
    · obtain ⟨va, hva, -, hunb⟩ := exec_moveLoc hx
      cases va with
      | nonReference => exact hd.complete.local_unborrowed (hunb rfl) hy hyL
      | reference y' => exact hnotref hva
    · obtain ⟨v, rest', old, -, hold, hcase⟩ := exec_stLoc hx
      rcases hcase with ⟨rfl, hunb, -⟩ | ⟨id, g', rfl, -, -⟩
      · exact hd.complete.local_unborrowed hunb hy hyL
      · exact hnotref hold
  | @global addr sid hop =>
    apply ct_reach_direct
    unfold GlobalOperand at hop
    have hcase : ((∃ d, instr = .moveFrom d) ∨ ∃ i, instr = .moveFromGeneric i) ∨
        ((∃ d, instr = .moveTo d) ∨ ∃ i, instr = .moveToGeneric i) := by
      rcases hop with h | h | h | h
      · exact Or.inl (Or.inl h)
      · exact Or.inr (Or.inl h)
      · exact Or.inl (Or.inr h)
      · exact Or.inr (Or.inr h)
    rcases hcase with hmf | hmt
    · -- `MoveFrom`, generic or not: the analysis found the type unborrowed.
      obtain ⟨addr', v, sid', hhead, htop, hvt, hl⟩ := ct_moveFrom_loc hmf hh hex ha
      simp only [Location.mk.injEq, Root.global.injEq, and_true] at hl
      obtain ⟨rfl, rfl⟩ := hl
      obtain ⟨d, tys, hdcase, sid0, a0, v0, st, hsid, hstk, hl1, hget, htop0⟩ :=
        ct_moveFrom_step hmf hstep' hcont
      rw [hstk] at hhead
      simp only [List.head?_cons, Option.some.injEq, Value.address.injEq] at hhead
      subst hhead
      rw [htop0, Option.some.injEq] at htop
      subst htop
      obtain ⟨res, hres, hrt, hrv⟩ := get?_mem hget
      have hvt0 := hpre.inv.typed res hres
      rw [hrt, hrv, hvt, RuntimeType.struct.injEq] at hvt0
      subst hvt0
      have hds : P.definedStruct? fr.function.module d = some sid.erase := globalType_erase hsid
      obtain ⟨hx', hacq⟩ := moveFrom_analysis hwf hact.fn hact.body hinstr hdcase hx
      obtain ⟨hcb, -⟩ := exec_moveFrom hx'
      obtain ⟨S0, hglob0, -⟩ := hctx.chain _ hsf
      have hacqT : acqTypes P fr.function sid :=
        ⟨d, by simp only [acqOf, hact.fn, Option.map_some, Option.getD_some]; exact hacq, hds⟩
      have hcount : countActive P ((fr :: rest).map Frame.skel) sid.module ≤ 1 := by
        rw [← hsf, ← hinv.2.active.count]; omega
      have hsk : s.frames.map Frame.skel =
          (fr.function, fr.countedActive) :: rest.map Frame.skel := by
        rw [hsf]; rfl
      intro hr'
      have hc0 := hd.complete
      rw [hsk] at hc0
      rcases hc0.prov y r hy hyL with ⟨i', hi'⟩ | ⟨d', hrc⟩ | ⟨π, hπ, hin, -⟩
      · rw [hr'] at hi'; cases hi'
      · exact rootChain_global_borrowed hrc hyL hr' hds hcb
      · have := hglob0.acq sid (hglob0.params π hπ _ sid (by rw [← hin.1, hr'])) hacqT
        omega
    · -- `MoveTo`, generic or not: no resource is there, so no valid reference is into it.
      obtain ⟨v, sid', rs, addr', hhead, hvt, hrs, hderef, hl⟩ := ct_moveTo_loc hmt hh hex ha
      simp only [Location.mk.injEq, Root.global.injEq, and_true] at hl
      obtain ⟨rfl, rfl⟩ := hl
      obtain ⟨sid0, v0, rs0, st, a0, hstk, -, hvt0, hderef0, hget⟩ :=
        ct_moveTo_step hmt hsf hstep' hcont
      rw [hstk] at hhead hrs
      simp only [List.head?_cons, Option.some.injEq] at hhead
      subst hhead
      simp only [List.getElem?_cons_succ, List.getElem?_cons_zero, Option.some.injEq,
        Value.ref.injEq] at hrs
      subst hrs
      rw [hvt0, RuntimeType.struct.injEq] at hvt
      subst hvt
      rw [hderef0, Option.some.injEq, Value.signer.injEq] at hderef
      subst hderef
      intro hr'
      have hread := (deref?_isSome.1 hok.valid).2
      rw [Location.eta r.location, hr'] at hread
      simp [State.read?, State.root?, hget] at hread

end MoveBytecode.Proofs
