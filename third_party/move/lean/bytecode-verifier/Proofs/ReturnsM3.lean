-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.NoEscapeM3
import Proofs.BorrowInside
import Proofs.RefContainTop

/-!
# What a verified frame borrows and returns

UNTRUSTED (checked by the kernel). The facts of M3 executions that D4d's
returns need: a reference a step borrows with no parent is a borrow of a
local of the current frame, or of a resource of a type the frame's function
acquires while its module is active once (R3); and a verified `Ret` returns no
reference into a local of the frame, nor into a resource of a type its
function acquires unless its module is active more than once.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

/-! ## Root borrows -/

/-- **A `borrowed` event with no parent** is recorded by a borrow of a local or
of a resource. -/
theorem exec_rootBorrow {P : Program} {m : ModuleId} {instr : Bytecode} {s : State}
    {fr : Frame} {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event}
    {g : ShadowFrame} {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) {d : Nat} {rd : Reference}
    (hb : Event.borrowed d rd none ∈ evs) :
    Shadow.topAfter o = some (.ref rd) ∧
      ((∃ j, instr = .mutBorrowLoc j ∨ instr = .immBorrowLoc j) ∨
        ∃ k, instr = .mutBorrowGlobal k ∨ instr = .immBorrowGlobal k ∨
          instr = .mutBorrowGlobalGeneric k ∨ instr = .immBorrowGlobalGeneric k) := by
  obtain ⟨htop, hnew⟩ := exec_borrowed hh hex hb
  refine ⟨htop, ?_⟩
  rcases hnew with ⟨rfl, -⟩ | ⟨j, hj⟩ | ⟨j, rfl | rfl | rfl | rfl⟩ |
    ⟨j, rfl | rfl | rfl | rfl⟩ | ⟨j, rfl | rfl⟩ | ⟨j, hj⟩
  · exfalso
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      guard] at hex
    obtain ⟨a, -, _, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp at hb
    · cases hm
  · exact Or.inl ⟨j, hj⟩
  all_goals first
    | exact Or.inr ⟨j, hj⟩
    | (exfalso
       simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
         guard] at hex
       first
         | (obtain ⟨a, -, hm⟩ := hex
            split at hm
            · obtain ⟨_, -, hm⟩ := Option.bind_eq_some_iff.1 hm
              split at hm
              · simp only [Option.some.injEq, Prod.mk.injEq] at hm
                obtain ⟨-, rfl⟩ := hm
                simp at hb
              · cases hm
            · cases hm)
         | (obtain ⟨a, -, _, -, hm⟩ := hex
            split at hm
            · simp only [Option.some.injEq, Prod.mk.injEq] at hm
              obtain ⟨-, rfl⟩ := hm
              simp at hb
            · cases hm))

/-- The reference a borrow of a local pushes designates the local. -/
theorem borrowLoc_top {P : Program} {s s' : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {instr : Bytecode} {j : LocalIndex}
    (hi : instr = .mutBorrowLoc j ∨ instr = .immBorrowLoc j)
    (h : stepOf (Step.exec P s fr rest info code instr) = .done (.running s')) {r : Reference}
    (htop : Shadow.topAfter (.running s') = some (.ref r)) :
    r.location = ⟨.local_ fr.id j, []⟩ := by
  have hb : ∃ mutable, stepOf (borrowLoc P s fr rest mutable j) = .done (.running s') := by
    rcases hi with rfl | rfl
    · exact ⟨true, h⟩
    · exact ⟨false, h⟩
  obtain ⟨mutable, h⟩ := hb
  unfold borrowLoc at h
  obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
  cases slot with
  | none => exact (done_wrong h rfl).elim
  | some v =>
    dsimp only at h
    obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
    split at h
    · exact (done_wrong h rfl).elim
    · obtain rfl := Outcome.running.inj (done_next h)
      simp only [Shadow.topAfter, List.head?_cons, advance, push, borrow, Option.bind_eq_bind,
        Option.bind_some, Option.some.injEq, Value.ref.injEq] at htop
      rw [← htop]

/-- The reference a borrow of a resource pushes designates the resource, of an
instance of the struct the operand names, whose module R3 found active at most
once. -/
theorem borrowGlobal_top {P : Program} {s s' : State} {fr : Frame} {rest : List Frame}
    {mutable : Bool} {d : StructDefinitionIndex} {tys : List RuntimeType}
    (h : stepOf (Step.borrowGlobal P s fr rest mutable d tys) = .done (.running s'))
    {r : Reference} (htop : Shadow.topAfter (.running s') = some (.ref r)) :
    ∃ a sid, r.location = ⟨.global a sid, []⟩ ∧
      P.definedStruct? fr.function.module d = some sid.erase ∧ s.activeCount sid.module ≤ 1 := by
  unfold Step.borrowGlobal at h
  obtain ⟨sid, hsid, h⟩ := done_bind h rfl globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h rfl popAddress_ends
  obtain ⟨st, -, rfl⟩ := popAddress_ok hpa
  obtain ⟨u, hu, h⟩ := done_bind h rfl unlocked_ends
  split at h
  · obtain rfl := Outcome.running.inj (done_next h)
    simp only [Shadow.topAfter, List.head?_cons, advance, push, borrow, Option.bind_eq_bind,
      Option.bind_some, Option.some.injEq, Value.ref.injEq] at htop
    refine ⟨a, sid, by rw [← htop], globalType_erase hsid, ?_⟩
    have := ct_unlocked hu
    omega
  · exact (done_fail h rfl).elim

/-! ## Returns -/

/-- A `Ret` that continues hands back values conforming to the function's
return types. -/
theorem ret_conforms {P : Program} {s : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {o : Outcome}
    (h : stepOf (Step.exec P s fr rest info code .ret) = .done o) (hcont : o.continues = true) :
    ∃ rets, P.returnTypes fr.function = some rets ∧ conformsAll P fr.stack.reverse rets = true := by
  unfold Step.exec Step.ret at h
  obtain ⟨rets, hrets, h⟩ := done_bind h hcont (ends_need _ _)
  refine ⟨rets, need_ok hrets, ?_⟩
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  · next hc => simpa using hc

/-- **A reference a verified step borrows with no parent** is into a local of
the current frame, or into a resource of a type the frame's function acquires
while its module is active at most once in the frames. -/
theorem rootBorrow_roots {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {s s' : State} (hinv : InvRefs P s) {fr : Frame} {rest : List Frame} {info : FunctionInfo}
    {code : CodeUnit} {instr : Bytecode}
    (hinfo : P.function? fr.function = some info) (hcode : info.definition.code = some code)
    (hinstr : code.code[fr.pc]? = some instr)
    (h : stepOf (Step.exec P s fr rest info code instr) = .done (.running s'))
    {hsh : Shadow} {post : ShadowPost} {evs : List Event}
    (hex : Shadow.exec P info.moduleId instr s fr (.running s') hsh = some (post, evs))
    {d : Nat} {rd : Reference} (hb : Event.borrowed d rd none ∈ evs) :
    (∃ i, rd.location.root = .local_ fr.id i) ∨
      ∃ a t, rd.location.root = .global a t ∧ acqTypes P fr.function t ∧
        countActive P (s.frames.map Frame.skel) t.module ≤ 1 := by
  obtain ⟨g, hs', hh⟩ : ∃ g hs', hsh.frames = g :: hs' := by
    cases hf : hsh.frames with
    | nil => simp [Shadow.exec, hf] at hex
    | cons g hs' => exact ⟨g, hs', rfl⟩
  obtain ⟨htop, ⟨j, hj⟩ | ⟨k, hk⟩⟩ := exec_rootBorrow hh hex hb
  · exact Or.inl ⟨j, by rw [borrowLoc_top hj h htop]⟩
  · -- A global borrow, generic or not, of a struct the function acquires.
    have hacq := function_acquires hwf hinfo hcode
    have hmem := getElem?_mem_toList hinstr
    obtain ⟨d, hd, mutable, tys, hst⟩ : ∃ d, d ∈ info.definition.acquiresGlobalResources ∧
        ∃ mutable tys, stepOf (Step.borrowGlobal P s fr rest mutable d tys) =
          .done (.running s') := by
      rcases hk with rfl | rfl | rfl | rfl
      · exact ⟨k, (acquires_instr hacq hmem).1 k (Or.inr (Or.inl rfl)), true, [], h⟩
      · exact ⟨k, (acquires_instr hacq hmem).1 k (Or.inr (Or.inr rfl)), false, [], h⟩
      · obtain ⟨x, hx, h⟩ := done_bind h rfl (ends_need _ _)
        obtain ⟨tys, -, h⟩ := done_bind h rfl genericArgs_ends
        exact ⟨x.def_, acquires_instrGeneric hacq hmem (Or.inr (Or.inl rfl)) (need_ok' hx), true,
          tys, h⟩
      · obtain ⟨x, hx, h⟩ := done_bind h rfl (ends_need _ _)
        obtain ⟨tys, -, h⟩ := done_bind h rfl genericArgs_ends
        exact ⟨x.def_, acquires_instrGeneric hacq hmem (Or.inr (Or.inr rfl)) (need_ok' hx), false,
          tys, h⟩
    obtain ⟨a, sid, hloc, hds, hcnt⟩ := borrowGlobal_top hst htop
    refine Or.inr ⟨a, sid, by rw [hloc], ⟨d, ?_, hds⟩, ?_⟩
    · simp only [acqOf, hinfo, Option.map_some, Option.getD_some]; exact hd
    · rw [← hinv.2.active.count]; exact hcnt

section M3

variable {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
  (hs : P.SupportedNonGeneric) (hm3 : P.M3)
include hwf hs hm3

/-- **What a verified `Ret` hands back** is into no local of the frame, and
into a resource of a type the frame's function acquires only while the type's
module is active more than once. -/
theorem ret_roots {s : State} (hinv : InvRefs P s) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hinfo : P.function? fr.function = some info) (hcode : info.definition.code = some code)
    (hinstr : code.code[fr.pc]? = some .ret) {rets : List RuntimeType}
    (hrets : P.returnTypes fr.function = some rets)
    (hconf : conformsAll P fr.stack.reverse rets = true) {r : Reference}
    (hr : Value.ref r ∈ fr.stack) :
    (∀ i, r.location.root ≠ .local_ fr.id i) ∧
      ∀ a t, r.location.root = .global a t → acqTypes P fr.function t →
        2 ≤ countActive P (s.frames.map Frame.skel) t.module := by
  obtain ⟨fr₀, rest₀, hsf₀, ⟨info₀, code₀, hact⟩, -⟩ := hinv.1
  rw [hsf, List.cons.injEq] at hsf₀
  obtain ⟨rfl, rfl⟩ := hsf₀
  have e1 := hact.fn
  rw [hinfo, Option.some.injEq] at e1
  subst e1
  have e2 := hact.body
  rw [hcode, Option.some.injEq] at e2
  subst e2
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtx hwf hs hm3 hinv hsf hact hinstr
  obtain ⟨rv0, hrv0, hsig0⟩ := hctx.cert.setting.agrees.returns
  rw [hrets, Option.some.injEq] at hrv0
  subst hrv0
  have hlenA : A.stack.length ≤ view.returns.length := by
    rw [hctx.height, ← List.length_reverse, conformsAll_length hconf, ofSignature_length hsig0]
    exact Nat.le_refl _
  obtain ⟨hrc1, -, -, hrc4⟩ := return_contract hctx hlenA
  have hra := mem_argRefs.2 hr
  refine ⟨hrc4 r hra, fun a t hroot hacq => ?_⟩
  obtain ⟨π, hπ, hin, -⟩ := hrc1 r hra
  obtain ⟨S, hglob, -⟩ := hctx.chain rest hsf
  have hSt := hglob.params π hπ a t (by rw [← hin.1, hroot])
  rw [hsf]
  exact hglob.acq t hSt hacq

end M3

/-! ## Uses -/

/-- **A reference a traced step uses is in the current frame** before it. -/
theorem traced_used_top {P : Program} {N : NativeRel} {ν : NativeFlows} {s : State} {h : Shadow}
    {o : Outcome} {post : ShadowPost} {evs : List Event} (hpre : Pre false P s h)
    (hT : TracedStep P N ν s h o post evs) {i : Nat} {r : Reference}
    (hu : Event.used i r ∈ evs) :
    ∃ g fr, h.frames.head? = some g ∧ s.frames.head? = some fr ∧
      (i, Value.ref r) ∈ g.occurrences fr := by
  cases hT with
  | core hstep hc =>
    obtain ⟨fr, rest, info, code, instr, hsf, -, -, -, hex⟩ := coreStep_exec hc
    obtain ⟨hframes, hst, hhe, fresh⟩ := h
    cases hframes with
    | nil =>
      have hfit := hpre.fits
      rw [shadow_fits, hsf] at hfit
      simp [framesFit] at hfit
    | cons g hs =>
      have hg : g.fits fr = true := (pre_head hpre hsf).1
      refine ⟨g, fr, rfl, by rw [hsf]; rfl, refSlot_occ hg ?_⟩
      rcases exec_used rfl hg hex i r hu with hsl | ⟨j, -, hl⟩
      · exact hsl.refSlot
      · exact hl.refSlot
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      obtain ⟨g, hs, fr, rest, hh, hsf, hm⟩ := native_used_top hpre hstep hn hu
      exact ⟨g, fr, by rw [hh]; rfl, by rw [hsf]; rfl, hm⟩
    obtain ⟨rfl, rfl, rfl, g, hs, info, hh, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at hu
    exact absurd hu nativeEvents_used

end MoveBytecode.Proofs
