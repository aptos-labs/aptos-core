-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.SoundnessAll
import Proofs.MutableContained
import Proofs.RefStableSteps
import Proofs.RefDeadSteps
import Proofs.ApiCore

/-!
# D4b′, D4c and D4e at each step, for all accepted programs

UNTRUSTED (checked by the kernel). The one-step facts of D4b′
(`stepsContained_tracked'`), D4c (`stepsStable_tracked'`) and D4e
(`stepsDead_tracked'`) for every accepted program on the supported subset,
generic code and struct-API mutable borrows included. From a state with the
reference invariant, the M3 proofs apply (`contained_top`, `contained_below'`,
`stable_top`, `stable_below`, through `topCtxGen`); a call of a struct-API
borrow accesses nothing. While a wrapper runs (`ApiCore`, generic or not),
every tracked reference is valid and rooted below the wrapper
(`apiCore_tracked`), and the wrapper only writes its own local 0 and inspects
(`apiCore_access`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

/-- The suspended frames are older than the frame above them. -/
theorem chainRef_lt {P : Program} {s : State} :
    ∀ {cid : FrameId} {cfn : FunctionRef} {cps : List Reference} {Sc : StructId → Prop}
      {frs : List Frame}, ChainRef P s cid cfn cps Sc frs → ∀ fr' ∈ frs, fr'.id < cid
  | _, _, _, _, [], _, _, h => by simp at h
  | _, _, _, _, _ :: _, ⟨_, _, hsusp, hrest⟩, fr', hfr' => by
    rcases List.mem_cons.1 hfr' with rfl | h
    · exact hsusp.order
    · exact Nat.lt_trans (chainRef_lt hrest fr' h) hsusp.order

/-- A tracked reference is one its frame keeps where the analysis tracks it. -/
theorem trackedIn_kept {P : Program} {g : ShadowFrame} {fr : Frame} {i : Nat} {r : Reference}
    (ht : TrackedIn P g fr i r) :
    Value.ref r ∈ fr.stack ∨ ∃ j, fr.locals[j]? = some (some (.ref r)) ∧
      modelAvail P fr.function fr.pc j = .available := by
  rcases ht with ⟨t, -, hz⟩ | ⟨j, ⟨t, -, -, hl⟩, hav⟩
  · exact Or.inl (List.of_mem_zip hz).2
  · exact Or.inr ⟨j, hl, hav⟩

/-- A reference valid for an older frame is in no local of a newer one. -/
theorem refOk_root_ne {P : Program} {s : State} {frx : Frame} {r : Reference}
    (hok : RefOk P s frx r) {w : FrameId} (hlt : frx.id < w) (j : Nat) :
    r.location.root ≠ .local_ w j := by
  intro e
  have hb := hok.below
  rw [e] at hb
  exact Nat.lt_irrefl _ (Nat.lt_of_le_of_lt (show w ≤ frx.id from hb) hlt)

section Tracked

variable {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
  (hs : P.Supported) (hsigs : P.ApiSigs)

include hwf hs hsigs in
/-- **A tracked reference of a struct-API borrow in progress** is valid for a
frame older than the wrapper. -/
theorem apiCore_tracked {s sc : State} (hrun : ApiCore P s sc) {h : Shadow} {i : Nat}
    {r : Reference} (ht : Tracked P s h i r) :
    ∃ w rest' frx, s.frames = w :: rest' ∧ frx.id < w.id ∧ RefOk P s frx r := by
  obtain ⟨fr, rest, info, code, off, x, st, w, cinfo, hsc, hact, hst, -, hsf, -, hwid, hstor,
    hheld, ⟨es, hes, hesw⟩, hloc0, hpcs⟩ := hrun.call
  have hinv := hrun.inv
  have hlt : fr.id < w.id := by
    rw [hwid]; exact hinv.2.below fr (by rw [hsc]; exact List.mem_cons_self)
  have hfrsc : fr ∈ sc.frames := by rw [hsc]; exact List.mem_cons_self
  have lift := fun {frx : Frame} {r : Reference} (hok : RefOk P sc frx r)
      (hfrx : frx.id = fr.id ∨ frx.id < fr.id) =>
    api_refOk (P := P) (fr' := { fr with stack := st }) hsc hsf rfl rfl rfl hstor hheld hes hesw
      hlt hok hfrx
  -- The caller's top reference `x`.
  have hx : RefOk P s fr x :=
    lift (kept_refOk_api hwf hs hsigs hinv hfrsc (Or.inl (by rw [hst]; exact List.mem_cons_self)))
      (Or.inl rfl)
  obtain ⟨instr, hi'⟩ : ∃ instr, code.code[fr.pc]? = some instr :=
    ⟨_, Array.getElem?_eq_getElem hact.pcLt⟩
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtxGen hwf hs hsigs hinv hsc hact hi'
  obtain ⟨S, -, hchain⟩ := hctx.chain rest hsc
  obtain ⟨g', fr', hz, ht⟩ := ht
  have hfr' : fr' ∈ s.frames := (List.of_mem_zip hz).2
  have hk := trackedIn_kept ht
  rw [hsf, List.mem_cons, List.mem_cons] at hfr'
  refine ⟨w, { fr with stack := st } :: rest, ?_⟩
  rcases hfr' with rfl | rfl | hfr'
  · -- The wrapper's references: `x`, or the field borrow of it.
    rcases hk with hk | ⟨j, hj, -⟩
    · rcases hpcs with ⟨-, -, hstk⟩ | ⟨-, -, hstk⟩ | ⟨-, -, y, step, hstk, -, -, -, hy⟩
      · rw [hstk] at hk; cases hk
      · rw [hstk, List.mem_singleton, Value.ref.injEq] at hk
        subst hk
        exact ⟨fr, hsf, hlt, hx⟩
      · rw [hstk, List.mem_singleton, Value.ref.injEq] at hk
        subst hk
        exact ⟨{ fr with stack := st }, hsf, hlt, hy⟩
    · obtain rfl := hloc0 j _ hj
      rcases hpcs with ⟨-, hl, -⟩ | ⟨-, hl, -⟩ | ⟨-, hl, -⟩
      · rw [hl, Option.some.injEq, Option.some.injEq, Value.ref.injEq] at hj
        subst hj
        exact ⟨fr, hsf, hlt, hx⟩
      · rw [hl] at hj; cases hj
      · rw [hl] at hj; cases hj
  · -- The caller's other references.
    have hk' : Value.ref r ∈ fr.stack ∨ ∃ j, fr.locals[j]? = some (some (.ref r)) ∧
        modelAvail P fr.function fr.pc j = .available := by
      rcases hk with hk | hk
      · exact Or.inl (by rw [hst]; exact List.mem_cons_of_mem _ hk)
      · exact Or.inr hk
    exact ⟨fr, hsf, hlt, lift (kept_refOk_api hwf hs hsigs hinv hfrsc hk') (Or.inl rfl)⟩
  · -- The frames below the caller.
    have hlt' := chainRef_lt hchain fr' hfr'
    exact ⟨fr', hsf, Nat.lt_trans hlt' hlt,
      lift (kept_refOk_api hwf hs hsigs hinv (by rw [hsc]; exact List.mem_cons_of_mem _ hfr') hk)
        (Or.inr hlt')⟩

include hwf hs hsigs in
/-- **Every tracked reference is valid**, with the reference invariant or a
struct-API borrow in progress. -/
theorem invCore_tracked_refOk {s : State} (hinv : InvCore P s) {h : Shadow} {i : Nat}
    {r : Reference} (ht : Tracked P s h i r) : ∃ frx, RefOk P s frx r := by
  rcases hinv.2 with hr | ⟨sc, hrun⟩
  · obtain ⟨g', fr', hz, ht'⟩ := ht
    exact ⟨fr', kept_refOk_api hwf hs hsigs ⟨hinv.1, hr⟩ (List.of_mem_zip hz).2 (trackedIn_kept ht')⟩
  · obtain ⟨-, -, frx, -, -, hok⟩ := apiCore_tracked hwf hs hsigs hrun ht
    exact ⟨frx, hok⟩

end Tracked

/-- **A step of a struct-API borrow in progress** only inspects, or accesses a
local of the wrapper directly. -/
theorem apiCore_access {P : Program} {s sc : State} (hrun : ApiCore P s sc) {fr : Frame}
    {rest : List Frame} (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    {instr : Bytecode} (hinfo : P.function? fr.function = some info)
    (hcode : info.definition.code = some code) (hinstr : code.code[fr.pc]? = some instr)
    {h : Shadow} {g : ShadowFrame} {hs' : List ShadowFrame} (hh : h.frames = g :: hs')
    {o : Outcome} {post : ShadowPost} {evs : List Event}
    (hex : Shadow.exec P info.moduleId instr s fr o h = some (post, evs)) {a : Access}
    {l : Location} {via : Option Nat} (ha : Event.accessed a l via ∈ evs) :
    a = .inspect ∨ ∃ j, l = ⟨.local_ fr.id j, []⟩ ∧ via = none := by
  obtain ⟨fr₀, rest₀, info₀, code₀, off₀, x₀, st₀, w, cinfo, -, -, -, hwc, hsf', hwfun, -⟩ :=
    hrun.call
  rw [hsf, List.cons.injEq] at hsf'
  obtain ⟨rfl, -⟩ := hsf'
  rw [hwfun, Option.some.injEq] at hinfo
  subst hinfo
  obtain ⟨ccode, b, hcc, hcl, hb⟩ := hwc.code
  rw [hcc, Option.some.injEq] at hcode
  subst hcode
  rcases wrapper_at hcl hinstr with ⟨-, rfl⟩ | ⟨-, rfl⟩ | ⟨-, rfl⟩
  · cases exec_accessed hh hex ha with
    | inspect => exact Or.inl rfl
    | viaRef _ _ _ hop => simp [RefOperand] at hop
    | direct _ => exact Or.inr ⟨_, rfl, rfl⟩
    | global hop => simp [GlobalOperand] at hop
  · rcases hb with ⟨fhi, rfl⟩ | ⟨vh, rfl⟩ | ⟨i, rfl⟩ | ⟨i, rfl⟩
    all_goals
      cases exec_accessed hh hex ha with
      | inspect => exact Or.inl rfl
      | viaRef _ _ _ hop => simp [RefOperand] at hop
      | direct hop => simp [LocalOperand] at hop
      | global hop => simp [GlobalOperand] at hop
  · cases exec_accessed hh hex ha with
    | inspect => exact Or.inl rfl
    | viaRef _ _ _ hop => simp [RefOperand] at hop
    | direct hop => simp [LocalOperand] at hop
    | global hop => simp [GlobalOperand] at hop

section Steps

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
  (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
  (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀)
include hwf hs hapi hN h0

/-- **No access of a verified step reaches over a tracked `&mut`**, nor over
a reference the step borrows (D4b′ at each step). -/
theorem stepsContained_tracked' : StepsContained P N ν s₀ (Tracked P) := by
  intro s h E hT o post evs hS a l via ha i r hheld hm
  have hI := refTraceInv_trace h0 hT
  have hsigs := apiSigs_of hwf hapi
  have hinvA := invCore_reachable hwf hs hapi hN h0 s (trace_reachable hT)
  cases hS with
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      rcases hheld with ⟨g', fr', hz, ht⟩ | ⟨par, j, k', hbj, hk', hjk⟩
      · rcases hinvA.2 with hr | ⟨sc, hrun⟩
        · exact contained_native hwf hs hsigs hapi ⟨hinvA.1, hr⟩ hI.pre hshape hstep hn ha hz ht hm
        · -- A wrapper in progress calls no native.
          exfalso
          obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hi, hcall⟩ := tf_native_instr hstep
          rcases apiCore_instr hrun hsf hinfo hcode hi with ⟨-, h⟩ |
            ⟨-, ⟨f, h⟩ | ⟨v, h⟩ | ⟨f, h⟩ | ⟨v, h⟩⟩ | ⟨-, h⟩ <;>
            rcases hcall with ⟨hd, rfl⟩ | ⟨i, rfl⟩ <;> cases h
      · have := nativeStep_order hn hbj hk'
        omega
    obtain ⟨rfl, rfl, rfl, g, hs', info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at ha
    exact absurd ha nativeEvents_not_accessed
  | core hstep hc =>
    obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hex⟩ := coreStep_exec hc
    rcases hheld with ⟨g', fr', hz, ht⟩ | ⟨par, j, k', hbj, -, -⟩
    · cases hfr : h.frames with
      | nil => rw [hfr] at hz; cases hz
      | cons g hs' =>
        rcases hinvA.2 with hr | ⟨sc, hrun⟩
        · have hinv : InvRefs P s := ⟨hinvA.1, hr⟩
          rw [hfr, hsf, List.zip_cons_cons, List.mem_cons] at hz
          rcases hz with he | hz
          · simp only [Prod.mk.injEq] at he
            obtain ⟨rfl, rfl⟩ := he
            exact contained_top hwf hs hsigs hinv hI.pre hI.typed hstep hc hfr hsf ha ht hm
          · exact contained_below' hwf hs hsigs hinv hI.pre hI.typed hstep hc hfr hsf ha hz ht hm
        · obtain ⟨w, rest', frx, hsw, hlt, hok⟩ :=
            apiCore_tracked hwf hs hsigs hrun (h := h) (i := i) ⟨g', fr', hz, ht⟩
          rw [hsf, List.cons.injEq] at hsw
          obtain ⟨rfl, -⟩ := hsw
          rcases apiCore_access hrun hsf hinfo hcode hinstr hfr hex ha with rfl | ⟨j, rfl, rfl⟩
          · cases via <;> rfl
          · exact ct_reach_direct (refOk_root_ne hok hlt j)
    · rw [exec_borrow_inspect hex (List.mem_of_getElem? hbj) ha]
      cases via <;> rfl

/-- **No write of a verified step overlaps a tracked immutable reference**, nor
a reference the step borrows (D4c at each step). -/
theorem stepsStable_tracked' : StepsStable P N ν s₀ (Tracked P) := by
  intro s h E hT o post evs hS l via ha i r hheld hm
  have hI := refTraceInv_trace h0 hT
  have hsigs := apiSigs_of hwf hapi
  have hinvA := invCore_reachable hwf hs hapi hN h0 s (trace_reachable hT)
  cases hS with
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      rcases hheld with ⟨g', fr', hz, ht⟩ | ⟨par, hb⟩
      · rcases hinvA.2 with hr | ⟨sc, hrun⟩
        · exact stable_native hwf hs hsigs hapi ⟨hinvA.1, hr⟩ hI.pre hshape hstep hn ha hz ht hm
        · -- A wrapper in progress calls no native.
          exfalso
          obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hi, hcall⟩ := tf_native_instr hstep
          rcases apiCore_instr hrun hsf hinfo hcode hi with ⟨-, h⟩ |
            ⟨-, ⟨f, h⟩ | ⟨v, h⟩ | ⟨f, h⟩ | ⟨v, h⟩⟩ | ⟨-, h⟩ <;>
            rcases hcall with ⟨hd, rfl⟩ | ⟨i, rfl⟩ <;> cases h
      · -- A native that borrows writes nothing back.
        exfalso
        obtain ⟨-, -, -, -, p, hp, -⟩ := nativeStep_borrowed hn hb
        obtain ⟨tys, -, hfits⟩ := nativeStep_fits hstep hn
        have hkb := places_borrow hfits (List.ne_nil_of_mem hp)
        obtain ⟨g, hs', -, -, h1⟩ := nativeStep_events hn ha
        rcases h1 with ⟨x, -, he⟩ | ⟨x, -, -, hku, he⟩ | ⟨x, -, he⟩ | ⟨_, _, _, he, -⟩ |
          ⟨_, _, _, he⟩ | ⟨_, _, _, he⟩
        · cases he
        · rw [hkb] at hku; cases hku
        all_goals cases he
    obtain ⟨rfl, rfl, rfl, g, hs', info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at ha
    exact absurd ha nativeEvents_not_accessed
  | core hstep hc =>
    obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hex⟩ := coreStep_exec hc
    rcases hheld with ⟨g', fr', hz, ht⟩ | ⟨par, hb⟩
    · cases hfr : h.frames with
      | nil => rw [hfr] at hz; cases hz
      | cons g hs' =>
        rcases hinvA.2 with hr | ⟨sc, hrun⟩
        · have hinv : InvRefs P s := ⟨hinvA.1, hr⟩
          rw [hfr, hsf, List.zip_cons_cons, List.mem_cons] at hz
          rcases hz with he | hz
          · simp only [Prod.mk.injEq] at he
            obtain ⟨rfl, rfl⟩ := he
            exact stable_top hwf hs hsigs hinv hI.pre hstep hc hfr hsf ha ht hm
          · exact stable_below hwf hs hsigs hinv hI.pre hstep hc hfr hsf ha hz ht hm
        · obtain ⟨w, rest', frx, hsw, hlt, hok⟩ :=
            apiCore_tracked hwf hs hsigs hrun (h := h) (i := i) ⟨g', fr', hz, ht⟩
          rw [hsf, List.cons.injEq] at hsw
          obtain ⟨rfl, -⟩ := hsw
          rcases apiCore_access hrun hsf hinfo hcode hinstr hfr hex ha with hai | ⟨j, rfl, rfl⟩
          · cases hai
          · exact st_overlaps_direct (refOk_root_ne hok hlt j)
    · cases exec_borrow_inspect hex hb ha

/-- **After a step that makes a local unavailable or ends a frame, no
tracked reference points into it** (D4e at each step). -/
theorem stepsDead_tracked' : StepsDead P N ν s₀ (Tracked P) := by
  intro s h E hT s' h' evs hS e he ended hend i r ht
  have hinv' := invCore_reachable hwf hs hapi hN h0 s' (trace_reachable (Trace.step hT hS))
  obtain ⟨frx, hok⟩ := invCore_tracked_refOk hwf hs (apiSigs_of hwf hapi) hinv' ht
  exact sd_ends (refTraceInv_trace h0 hT) hS rfl he hend (deref?_isSome.1 hok.valid).2

end Steps

end MoveBytecode.Proofs
