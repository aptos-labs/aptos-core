-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefGlobals
import Proofs.RefCallComplete
import Proofs.RefAcquires

/-!
# Global borrows across calls

UNTRUSTED (checked by the kernel). What protects a suspended frame's global
references while the frames above it run: a frame's module is active at or
below it; a function of the same module that it calls acquires only types it
acquires; and the analysis's call checks that the caller has borrowed none of
the types the callee acquires.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph

/-! ## Activations -/

theorem CountedOk.tail {P : Program} {p : FunctionRef × Bool} {sk : List (FunctionRef × Bool)}
    (h : CountedOk P (p :: sk)) : CountedOk P sk := by
  cases sk with
  | nil => trivial
  | cons q rest => exact h.2

/-- **A frame's module is active** at or below it. -/
theorem countActive_self {P : Program} :
    ∀ {sk : List (FunctionRef × Bool)} {p : FunctionRef × Bool} {M : ModuleId},
      CountedOk P (p :: sk) → funModule P p.1 = some M → 1 ≤ countActive P (p :: sk) M
  | [], p, M, h, hm => by
    rw [countActive_cons]
    simp only [CountedOk] at h
    simp [h, hm]
  | q :: rest, p, M, h, hm => by
    rw [countActive_cons]
    by_cases hp : p.2 = true
    · simp [hp, hm]
    · have hpq : funModule P p.1 = funModule P q.1 :=
        Decidable.of_not_not fun hne => hp (h.1 hne)
      have := countActive_self (sk := rest) (p := q) h.2 (by rw [← hpq]; exact hm)
      exact Nat.le_trans this (Nat.le_add_left _ _)

theorem countActive_le_cons (P : Program) (p : FunctionRef × Bool) (sk : List (FunctionRef × Bool))
    (M : ModuleId) : countActive P sk M ≤ countActive P (p :: sk) M := by
  rw [countActive_cons]; exact Nat.le_add_left _ _

/-! ## Global types -/

/-- Distinct struct definitions of a module name distinct types. -/
theorem definedStruct_inj {P : Program} {mi d d' : Nat} {t : StructId}
    (h : P.definedStruct? mi d = some t) (h' : P.definedStruct? mi d' = some t) : d = d' := by
  have key : ∀ d, P.definedStruct? mi d = some t → P.resolveStruct t = some ⟨mi, d⟩ := by
    intro d h
    simp only [Program.definedStruct?, bind, Option.bind_eq_some_iff] at h
    obtain ⟨info, -, name, -, h⟩ := h
    split at h
    · next hr => simp only [Option.some.injEq] at h; subst h; exact hr
    · cases h
  have := (key d h).symm.trans (key d' h')
  simp only [Option.some.injEq, StructRef.mk.injEq] at this
  exact this.2

/-- What the analysis names for a frame, by struct definition. -/
theorem gsidOf_def {P : Program} {f : FunctionRef} {b : Bool} {sk : List (FunctionRef × Bool)}
    {d : Nat} {t : StructId} (h : gsidOf P ((f, b) :: sk) d = some t) :
    d ∈ acqOf P f ∧ P.definedStruct? f.module d = some t := by
  simp only [gsidOf] at h
  split at h
  · split at h
    · next hc => exact ⟨hc.1, h⟩
    · cases h
  · cases h

/-- The types a function acquires are of its module. -/
theorem acqTypes_funModule {P : Program} {f : FunctionRef} {t : StructId} (h : acqTypes P f t) :
    funModule P f = some t.module := by
  obtain ⟨d, hd, hdef⟩ := h
  cases hf : P.function? f with
  | none => simp [acqOf, hf] at hd
  | some info =>
    have hm : t.module = info.moduleId := (definedStruct_module hdef hf :)
    simp [funModule, hf, hm]

/-- Instruction `instr` calls function handle `h`: `Call(h)`, or
`CallGeneric` of an instantiation of `h`. -/
def CallsHandle (m : CompiledModule) (instr : Bytecode) (h : Nat) : Prop :=
  instr = .call h ∨
    ∃ i fi, instr = .callGeneric i ∧ m.functionInstantiations[i]? = some fi ∧ fi.handle = h

/-- **What the acquires verifier checks of a call**: every type a same-module
callee annotates is annotated. -/
theorem acquires_call {m : CompiledModule} {fdef : FunctionDefinition} {code : CodeUnit}
    (h : verifyAcquires m fdef code = .ok ()) {i : Bytecode} (hi : i ∈ code.code.toList)
    {hh : Nat} (hcall : CallsHandle m i hh) :
    ∀ acq, Acquires.functionAcquires m hh = .ok acq →
      ∀ d ∈ acq, d ∈ fdef.acquiresGlobalResources := by
  rcases hcall with rfl | ⟨k, fi, rfl, hfi, rfl⟩
  · exact (acquires_instr h hi).2 _ rfl
  · intro acq hacq d hd
    unfold verifyAcquires at h
    simp only [bind_ok] at h
    obtain ⟨actual, hfold, -⟩ := h
    obtain ⟨b, r', hstep⟩ := foldlM_ok_mem hfold _ hi
    simp only [Acquires.instr, Acquires.callAcquire, bind_ok, lookup_ok, hfi,
      Option.some.injEq, exists_eq_left'] at hstep
    obtain ⟨a, ha, hstep⟩ := hstep
    rw [hacq, Except.ok.injEq] at ha
    subst ha
    split at hstep
    · simp [reject, Functor.map, Except.map] at hstep
    · next hc =>
      simp only [List.any_eq_true, Bool.not_eq_eq_eq_not, Bool.not_true, not_exists,
        not_and] at hc
      have := hc d hd
      simpa using this

/-- **A same-module callee acquires only what its caller does** (the acquires
verifier), struct definition by struct definition. -/
theorem acqOf_call {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit} (hf : P.function? f = some info)
    (hcode : info.definition.code = some code) {pc h : Nat} {instr : Bytecode}
    (hi : code.code[pc]? = some instr) (hcall : CallsHandle info.module instr h)
    {fh : FunctionHandle}
    (hfh : info.module.functionHandles[h]? = some fh)
    (hself : fh.module = info.module.selfModuleHandleIdx) {c : FunctionRef}
    (hres : P.resolve f.module h = some c) :
    c.module = f.module ∧ (∀ x ∈ acqOf P c, x ∈ acqOf P f) ∧
      ∀ acq, ReferenceSafety.acquiredBy info.module fh = .ok acq → ∀ x ∈ acqOf P c, x ∈ acq := by
  obtain ⟨hm, hid, hd, -⟩ := function?_parts hf
  have hv := verifyModule_ok (wellFormed_module hwf hm).1
  have hu := verifyDuplication_ok hv.2.1
  have hb := verifyBounds_handles hv.1
  obtain ⟨di, d, hres', hd', hdh, hcf⟩ := resolve_self hwf.1 hm hid hu hb hfh hself
  rw [hres, Option.some.injEq] at hres'
  subst hres'
  have hacqc : acqOf P ⟨f.module, di, []⟩ = d.acquiresGlobalResources := by simp [acqOf, hcf]
  refine ⟨rfl, fun x hx => ?_, fun acq hacq x hx => ?_⟩
  · rw [hacqc] at hx
    have hacq := function_acquires hwf hf hcode
    have hfa := functionAcquires_self hu hfh hself hd' hdh
    have := acquires_call hacq (getElem?_mem_toList hi) hcall _ hfa x hx
    simp only [acqOf, hf, Option.map_some, Option.getD_some]
    exact this
  · rw [hacqc] at hx
    exact acquiredBy_self hu hfh hself hd' hdh hacq x hx

/-- **A global type the caller borrowed is not acquired by a callee of its
module**: the analysis's call found no borrow of what the callee acquires. -/
theorem rootChain_not_acq {P : Program} {g : Graph} {L : Nat → Option Reference} {root : Nat}
    {fid : FrameId} {f : FunctionRef} {b : Bool} {sk : List (FunctionRef × Bool)}
    {lbl : Option Label} {x : Nat} {r : Reference} {a : Address} {t : StructId}
    {c : FunctionRef} {acq : List Nat}
    (h : RootChain g L root fid (gsidOf P ((f, b) :: sk)) lbl x) (hx : L x = some r)
    (hr : r.location.root = .global a t)
    (hchk : ∀ d ∈ acq, g.hasConsistentBorrows root (some (.global d)) = .ok false)
    (hsub : ∀ d ∈ acqOf P c, d ∈ acq) (hmod : c.module = f.module) : ¬ acqTypes P c t := by
  rintro ⟨d, hd, hdef⟩
  obtain ⟨e, n, rn, he, -, hn, ht, hrest⟩ := h
  have hroot : r.location.root = rn.location.root := by
    rcases hrest with rfl | hch
    · rw [hn, Option.some.injEq] at hx; subst hx; rfl
    · exact (hch.facts hn hx).1
  have hat := Graph.hasConsistentBorrows_false (hchk d (hsub d hd)) he
  unfold TrueRootEdge at ht
  split at ht
  · next i rest hp =>
    obtain ⟨h1, -⟩ := ht
    rw [← hroot, hr] at h1
    cases h1
  · next d' rest hp =>
    obtain ⟨⟨a', sid, hs, h1⟩, -⟩ := ht
    rw [← hroot, hr] at h1
    simp only [Root.global.injEq] at h1
    obtain ⟨-, rfl⟩ := h1
    obtain ⟨-, hdef'⟩ := gsidOf_def hs
    rw [hmod] at hdef
    have hdd := definedStruct_inj hdef hdef'
    subst hdd
    simp [Graph.atLabel, hp] at hat
  · next hp =>
    simp [Graph.atLabel, hp] at hat
  · exact ht.elim

/-- **A global type the analysis finds unborrowed is borrowed by no reference
the frame tracks through its root.** -/
theorem rootChain_global_borrowed {P : Program} {g : Graph} {L : Nat → Option Reference}
    {root : Nat} {fid : FrameId} {f : FunctionRef} {b : Bool} {sk : List (FunctionRef × Bool)}
    {lbl : Option Label} {x : Nat} {r : Reference} {a : Address} {t : StructId} {d : Nat}
    (h : RootChain g L root fid (gsidOf P ((f, b) :: sk)) lbl x) (hx : L x = some r)
    (hr : r.location.root = .global a t) (hdef : P.definedStruct? f.module d = some t.erase)
    (hchk : g.hasConsistentBorrows root (some (.global d)) = .ok false) : False := by
  obtain ⟨e, n, rn, he, -, hn, ht, hrest⟩ := h
  have hroot : r.location.root = rn.location.root := by
    rcases hrest with rfl | hch
    · rw [hn, Option.some.injEq] at hx; subst hx; rfl
    · exact (hch.facts hn hx).1
  have hat := Graph.hasConsistentBorrows_false hchk he
  unfold TrueRootEdge at ht
  split at ht
  · next i rest hp =>
    obtain ⟨h1, -⟩ := ht
    rw [← hroot, hr] at h1
    cases h1
  · next d' rest hp =>
    obtain ⟨⟨a', sid, hs, h1⟩, -⟩ := ht
    rw [← hroot, hr] at h1
    simp only [Root.global.injEq] at h1
    obtain ⟨-, rfl⟩ := h1
    obtain ⟨-, hdef'⟩ := gsidOf_def hs
    have hdd := definedStruct_inj hdef hdef'
    subst hdd
    simp [Graph.atLabel, hp] at hat
  · next hp =>
    simp [Graph.atLabel, hp] at hat
  · exact ht.elim

/-- Removing a resource keeps every other resource. -/
theorem GlobalStorage.get?_remove_ne {st : GlobalStorage} {a a' : Address} {t t' : StructId}
    (h : ¬ (a = a' ∧ t = t')) : (st.remove a t).get? a' t' = st.get? a' t' := by
  unfold GlobalStorage.get? GlobalStorage.remove
  rw [find?_filter_of (fun r hr => by
    simp only [Bool.and_eq_true, beq_iff_eq] at hr
    simp only [Bool.not_eq_true', Bool.and_eq_false_iff, beq_eq_false_iff_ne]
    by_cases ha : r.address = a
    · right; intro ht; exact h ⟨by rw [← ha, hr.1], by rw [← ht, hr.2]⟩
    · left; exact ha)]

/-- Removing a resource changes only what the removal invalidates. -/
theorem storageKept_remove {s s' : State} {a : Address} {t : StructId}
    (hs' : s'.storage = s.storage.remove a t) : StorageKept s s' [.removed (.global a t)] :=
  fun a' t' _ hcov => by
    have hne : ¬ (a = a' ∧ t = t') := fun ⟨h1, h2⟩ => by
      have := hcov _ List.mem_cons_self
      simp [Invalidation.covers, h1, h2] at this
    rw [hs']; exact GlobalStorage.get?_remove_ne hne

variable {P : Program} {s : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit} {view : FunctionView} {A : RefState} {stys : List SignatureToken}
  {tys : List RuntimeType} {ps : List Reference}

/-- **`MoveFrom`**: the resource it removes is of a type the function
acquires, in a module active once, so no suspended frame and no reference
parameter is into it; the analysis found the current frame's own borrows of
the type released. -/
theorem refSafe_moveFromAt (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {instr : Bytecode}
    {d : StructDefinitionIndex} {targs : List RuntimeType}
    (hctx : TopCtx P s fr info code instr view A stys tys ps)
    (hi : code.code[fr.pc]? = some instr) (hret : instr ≠ .ret)
    (hctl : instr.control = .fallThrough) (hav : ∀ a, availabilityAfter instr a = a)
    (hacqd : d ∈ acqOf P fr.function)
    (hrun : ∀ A', (ReferenceSafety.executeInner info.module view instr).run A = .ok ((), A') →
      (ReferenceSafety.executeInner info.module view (.moveFrom d)).run A = .ok ((), A'))
    (htyp : ∀ before after, typeAfter P fr.function info.module instr before = some after →
      ∃ sid brest, Step.globalType P fr.function.module d targs = .ok sid ∧
        P.definedStruct? fr.function.module d = some sid.erase ∧
        before = .address :: brest ∧ after = .struct sid :: brest)
    (hexec : Step.exec P s fr rest info code instr = Step.moveResourceFrom P s fr rest d targs) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code instr) := by
  refine ExecSafe.and (moveFrom_safeAt hc hr.storage hact hchain hi hctl hav (fun b a h => by
    obtain ⟨sid, brest, hg, -, h1, h2⟩ := htyp b a h
    exact ⟨sid, brest, hg, h1, h2⟩) hexec) ?_
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  obtain ⟨sid, brest, hglob, hds, rfl, rfl⟩ := htyp _ _ hafter
  obtain ⟨va, vs, hst, hva, hvs⟩ := conformsAll_cons_right hctx.conf
  obtain ⟨a, rfl⟩ := conforms_address hva
  rw [hexec]
  by_cases hl : s.activeCount sid.module > 1
  · simp [Step.moveResourceFrom, hglob, popAddress_of hst, Step.unlocked, hl, bind,
      Except.bind, Step.fail]
    trivial
  cases hg : s.storage.get? a sid with
  | none =>
    simp [Step.moveResourceFrom, hglob, popAddress_of hst, Step.unlocked, hl, hg, bind,
      Except.bind, pure, Except.pure, Step.fail]
    trivial
  | some v =>
    simp only [Step.moveResourceFrom, hglob, popAddress_of hst, Step.unlocked, hl,
      if_false, bind, Except.bind, pure, Except.pure, hg, Step.finish, State.invalidate]
    show RefInvariant P _
    obtain ⟨A', hx, hsucc⟩ := hctx.next hret
    obtain ⟨hcb, hvs'⟩ := exec_moveFrom (hrun _ hx)
    obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, hctl])
    obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
    obtain ⟨S0, hglob0, -⟩ := hctx.chain _ hsf
    -- The type is one the function acquires, in a module active once.
    have hacqT : acqTypes P fr.function sid := ⟨d, hacqd, hds⟩
    have hcount : countActive P ((fr :: rest).map Frame.skel) sid.module ≤ 1 := by
      rw [← hsf, ← hr.active.count]; omega
    have hnotS : ∀ S, GlobOk P fr.function ((fr :: rest).map Frame.skel) ps S → ¬ S sid :=
      fun S hS hSs => by have := hS.acq sid hSs hacqT; omega
    have hsk : s.frames.map Frame.skel = (fr.function, fr.countedActive) :: rest.map Frame.skel := by
      rw [hsf]; rfl
    -- No reference of the frame is into the resource.
    have hcov : ∀ x r, x ∈ heldIds A → L x = some r →
        Invalidation.covers (.removed (.global a sid)) r.location = false := by
      intro x r hx hxr
      cases hcv : Invalidation.covers (.removed (.global a sid)) r.location with
      | false => rfl
      | true =>
        exfalso
        have hr' : r.location.root = .global a sid := by
          simpa [Invalidation.covers] using hcv
        have hc0 := hd.complete
        rw [hsk] at hc0
        rcases hc0.prov x r hx hxr with ⟨i, hi'⟩ | ⟨d', hrc⟩ | ⟨π, hπ, hin, -⟩
        · rw [hr'] at hi'; cases hi'
        · exact rootChain_global_borrowed hrc hxr hr' hds hcb
        · exact hnotS S0 hglob0 (hglob0.params π hπ a sid (by rw [← hin.1, hr']))
    have hheldIds := hvs'.heldIds
    refine refInv_finish hr hsf (hctx.chain _ hsf) rfl rfl rfl rfl rfl rfl (Nat.le_refl _)
      (fun r' hr' => hr.storage r' (List.mem_filter.1 hr').1) (es := [.removed (.global a sid)])
      rfl (storageKept_remove rfl) (fun S hS e he l hl => ?_)
      (hd.valueStep hvs' rfl rfl (new := [v]) (j' := 1)
        (by simp [Step.advance, Step.pushAll, hst]) (by rw [hst]; exact take_one_notRef (by simp)))
      (fun x r hx hxr => ?_) hA'' hle
      (RefsFrom.next (new := [v]) (n := 1) (by simp [Step.advance, Step.pushAll, hst]) rfl
        (fun w hw => by
          simp only [List.mem_singleton] at hw
          subst hw
          obtain ⟨r0, hr0, hr0t, hr0v⟩ := get?_mem hg
          have hvc : w.conforms P (.struct sid) = true := by
            rw [← hr0v, ← hr0t]; exact hr.storage r0 hr0
          exact .of_not_ref fun q hq => by
            subst hq; have := conforms_type hvc rfl; simp [Value.type] at this))
    · simp only [List.mem_singleton] at he
      subst he
      simp only [Invalidation.covers, beq_iff_eq] at hl
      exact Or.inr (Or.inr ⟨a, sid, hl, hnotS S hS⟩)
    · rw [hheldIds] at hx
      exact (hrefs x r hx hxr).sameFrames (s' := { s with
          frames := Step.advance (Step.pushAll { fr with stack := vs } [v]) :: rest
          storage := s.storage.remove a sid
          invalidations := .removed (.global a sid) :: s.invalidations })
        (fr' := Step.advance (Step.pushAll { fr with stack := vs } [v])) hsf rfl rfl rfl rfl rfl
        (storageKept_remove rfl) rfl
        (fun e he => by simp only [List.mem_singleton] at he; subst he; exact hcov x r hx hxr) rfl

/-- **`MoveFrom`**: the resource it removes is of a type the function
acquires, in a module active once, so no suspended frame and no reference
parameter is into it; the analysis found the current frame's own borrows of
the type released. -/
theorem refSafe_moveFrom (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {d : StructDefinitionIndex}
    (hctx : TopCtx P s fr info code (.moveFrom d) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.moveFrom d)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (.moveFrom d)) := by
  have h1 := (acquires_instr (function_acquires hwf hact.fn hact.body)
    (getElem?_mem_toList hi)).1 d (Or.inl rfl)
  refine refSafe_moveFromAt (d := d) (targs := []) hc hr hsf hact hchain hctx hi
    (by intro h; cases h) rfl (fun _ => rfl)
    (by simp only [acqOf, hact.fn, Option.map_some, Option.getD_some]; exact h1) (fun _ hx => hx)
    (fun _ _ hafter => ?_) rfl
  obtain ⟨sid, brest, hds, hkey, h1, h2⟩ := typeAfter_moveFrom hafter
  exact ⟨sid, brest, globalType_of_key hds hkey,
    by rw [StructId.erase_of_nil (definedStruct?_typeArgs hds)]; exact hds, h1, h2⟩

/-- The first edge of a root chain to a reference into global storage of type
`t` is at the label of the definition naming `t`. -/
theorem RootChain.global_first {g : Graph} {L : Nat → Option Reference} {root : Nat}
    {fid : FrameId} {gsid : Nat → Option StructId} {lbl : Option Label} {x : Nat}
    {rx : Reference} {a : Address} {t : StructId} {d : Nat}
    (h : RootChain g L root fid gsid lbl x) (hx : L x = some rx)
    (hr : rx.location.root = .global a t) (hinj : ∀ d', gsid d' = some t.erase → d' = d) :
    ∃ e n rn, g.Edge root e n ∧ Graph.atLabel (some (.global d)) e = true ∧ L n = some rn ∧
      (n = x ∨ Chain g L n x) := by
  obtain ⟨e, n, rn, he, -, hn, ht, hrest⟩ := h
  refine ⟨e, n, rn, he, ?_, hn, hrest⟩
  have hroot : rx.location.root = rn.location.root := by
    rcases hrest with rfl | hch
    · rw [hn, Option.some.injEq] at hx; subst hx; rfl
    · exact (hch.facts hn hx).1
  unfold TrueRootEdge at ht
  split at ht
  · next i rest hp =>
    obtain ⟨h1, -⟩ := ht
    rw [← hroot, hr] at h1
    cases h1
  · next d' rest hp =>
    obtain ⟨⟨a', sid, hs, h1⟩, -⟩ := ht
    rw [← hroot, hr] at h1
    simp only [Root.global.injEq] at h1
    obtain ⟨-, rfl⟩ := h1
    have hdd := hinj d' hs
    subst hdd
    simp [Graph.atLabel, hp]
  · next hp => simp [Graph.atLabel, hp]
  · exact ht.elim

namespace Complete

variable {g g1 g2 : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
  {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}

/-- **Borrowing a global resource**: a new reference `c` to the resource of
type `t` under `a`, borrowed weakly from the root through the definition `d`
naming `t`, which the frame has not borrowed (mutably, for an immutable
borrow), and which no reference parameter is into. -/
theorem borrowGlobal {d c : Nat} {r : Reference} {a : Address} {t : StructId}
    (hc : Complete g L H root fid gsid ps)
    (hloc : r.location = ⟨.global a t, []⟩) (hgs : gsid d = some t.erase)
    (hinj : ∀ d', gsid d' = some t.erase → d' = d)
    (hps : ∀ π ∈ ps, ∀ a', π.location.root ≠ .global a' t)
    (hmut : r.mutable = true → g.hasConsistentBorrows root (some (.global d)) = .ok false)
    (himm : g.hasConsistentMutableBorrows root (some (.global d)) = .ok false)
    (hnew : g.newRef c r.mutable = .ok g1)
    (hadd : g1.addEdge root ⟨false, [.global d]⟩ c = .ok g2) :
    Complete g2 (updL L c r) (fun x => H x ∨ x = c) root fid gsid ps := by
  have hcfresh := (Graph.newRef_ok hnew).1
  have hcH : ¬ H c := hc.not_held_of_fresh hcfresh
  have hcr : c ≠ root := fun e => by
    have := hc.root_key; rw [← e, hcfresh] at this; cases this
  have hw1 := Graph.newRef_wf hnew hc.wf
  have hb1 := Graph.newRef_back hnew hc.back
  obtain ⟨hw2, hcov2, hmut2⟩ := Graph.addEdge_step hadd hw1
  obtain ⟨-, e₁, he₁, hle₁⟩ := Graph.addEdge_covers hadd hw1.sets
  have hLc : updL L c r c = some r := updL_self _ _ _
  have hLx : ∀ x, x ≠ c → updL L c r x = L x := fun x hx => updL_ne r hx
  generalize updL L c r = L' at hLc hLx ⊢
  have hext : ∀ x r, (g.find? x).isSome → L x = some r → L' x = some r := by
    intro x r hx h
    have : x ≠ c := fun e => by subst e; rw [hcfresh] at hx; cases hx
    rw [hLx x this]; exact h
  have hHL : ∀ x r, H x → L' x = some r → L x = some r := fun x r hx h => by
    rwa [hLx x (fun e => hcH (e ▸ hx))] at h
  have hrootL : L' root = none := by rw [hLx root (Ne.symm hcr)]; exact hc.rootNone
  have hcov : g.Covers g2 := (Graph.newRef_covers hnew).trans hcov2
  have hmapC : ∀ a b, Chain g L a b → Chain g2 L' a b := fun a b h =>
    Chain.mono hcov (h.extend' hext)
  have hmapR : ∀ l b, RootChain g L root fid gsid l b → RootChain g2 L' root fid gsid l b :=
    fun l b h => RootChain.mono hcov (h.extend' hext)
  -- No reference of the frame is into the type, mutably for an immutable borrow.
  have hglobal : ∀ x rx, H x → L x = some rx → ∀ a', rx.location.root = .global a' t →
      (r.mutable = true → False) ∧ (r.mutable = false → rx.mutable = false) := by
    intro x rx hx hxL a' hr
    rcases hc.prov x rx hx hxL with ⟨i, hi⟩ | ⟨d0, hrc⟩ | ⟨π, hπ, hin, -⟩
    · rw [hr] at hi; cases hi
    · obtain ⟨e, n, rn, he, hl, hn, hrest⟩ := hrc.global_first hxL hr hinj
      refine ⟨fun hm => ?_, fun hm => ?_⟩
      · have := Graph.hasConsistentBorrows_false (hmut hm) he
        rw [hl] at this; cases this
      · have hnH : H n := hc.rootChain_first_held he hn
        have hm' := Graph.hasConsistentMutableBorrows_false himm he hl
        rw [hc.held_mut hnH hn, Option.some.injEq] at hm'
        rcases hrest with rfl | hrest
        · rw [hn, Option.some.injEq] at hxL; subst hxL; exact hm'
        · exact (hrest.facts hn hxL).2.2 hm'
    · exact absurd (by rw [← hin.1, hr]) (hps π hπ a')
  have hnotin : ∀ rq : Reference, ¬ r.location.StrictlyInside rq.location := by
    rintro rq ⟨⟨-, hpre⟩, hne⟩
    rw [hloc] at hpre hne
    exact hne (List.prefix_nil.1 hpre).symm
  have hpi : TrueRootEdge fid gsid ⟨false, [.global d]⟩ r.location := by
    refine ⟨⟨a, t, hgs, by rw [hloc]⟩, ?_⟩
    simp [hloc, absPath]
  have hrootC : RootChain g2 L' root fid gsid (some (.global d)) c :=
    ⟨e₁, c, r, he₁, RootChain.atLabel_cover (by simp [Graph.atLabel]) hle₁, hLc, hpi.cover hle₁,
      Or.inl rfl⟩
  have hmutx : ∀ x, x ≠ c → g2.mutability x = g.mutability x := fun x hx => by
    rw [hmut2, Graph.newRef_mutability hnew x hx]
  refine {
    wf := hw2
    back := Graph.addEdge_back hadd hb1
    rootNone := hrootL
    rootMut := by rw [hmutx root (Ne.symm hcr)]; exact hc.rootMut
    rootNotHeld := fun h => h.elim hc.rootNotHeld (fun e => hcr e.symm)
    keys := fun x hx => ?_
    held := fun x hx => ?_
    strict := fun q rq x rx hq hx hqL hxL hm hin => ?_
    same := fun q rq x rx hq hx hne hqL hxL hm heq => ?_
    immInside := fun q rq x rx hq hx hqL hxL hm hin => ?_
    rootLocal := fun x rx i' hx hxL hr => ?_
    prov := fun x rx hx hxL => ?_ }
  · by_cases hxc : x = c
    · exact Or.inr (Or.inr hxc)
    · rw [Graph.isSome_of_mutability (hmutx x hxc)] at hx
      rcases hc.keys x hx with h | h
      · exact Or.inl h
      · exact Or.inr (Or.inl h)
  · rcases hx with hx | rfl
    · obtain ⟨r', hr, hm⟩ := hc.held x hx
      have hxc : x ≠ c := fun e => hcH (e ▸ hx)
      exact ⟨r', by rw [hLx x hxc]; exact hr, by rw [hmutx x hxc]; exact hm⟩
    · exact ⟨r, hLc, by rw [hmut2, Graph.newRef_mutability_self hnew]⟩
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hmapC q x (hc.strict q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL; exact absurd hin (hnotin rq)
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      exact ((hglobal x rx hx (hHL x rx hx hxL) a (by rw [hin.1.1, hloc])).1 hm).elim
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin (hnotin _)
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · rcases hc.same q rq x rx hq hx hne (hHL q rq hq hqL) (hHL x rx hx hxL) hm heq with h | h
      · exact Or.inl (hmapC q x h)
      · exact Or.inr (hmapC x q h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      have hqg := hglobal q rq hq (hHL q rq hq hqL) a (by rw [← heq, hloc])
      cases hrm : r.mutable
      · exact absurd (hqg.2 hrm) (by rw [hm]; simp)
      · exact (hqg.1 hrm).elim
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      exact ((hglobal x rx hx (hHL x rx hx hxL) a (by rw [heq, hloc])).1 hm).elim
    · exact absurd rfl hne
  · rcases hq with hq | rfl <;> rcases hx with hx | rfl
    · exact hc.immInside q rq x rx hq hx (hHL q rq hq hqL) (hHL x rx hx hxL) hm hin
    · rw [hLc, Option.some.injEq] at hxL; subst hxL; exact absurd hin (hnotin rq)
    · rw [hLc, Option.some.injEq] at hqL; subst hqL
      exact (hglobal x rx hx (hHL x rx hx hxL) a (by rw [hin.1.1, hloc])).2 hm
    · rw [hLc, Option.some.injEq] at hqL hxL; subst hqL hxL; exact absurd hin (hnotin _)
  · rcases hx with hx | rfl
    · exact hmapR _ x (hc.rootLocal x rx i' hx (hHL x rx hx hxL) hr)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      rw [hloc] at hr
      cases hr
  · rcases hx with hx | rfl
    · rcases hc.prov x rx hx (hHL x rx hx hxL) with h | ⟨d', h⟩ | h
      · exact Or.inl h
      · exact Or.inr (Or.inl ⟨d', hmapR _ x h⟩)
      · exact Or.inr (Or.inr h)
    · rw [hLc, Option.some.injEq] at hxL; subst hxL
      exact Or.inr (Or.inl ⟨d, hrootC⟩)

end Complete

theorem typeAfter_borrowGlobal {P : Program} {f : FunctionRef} {m : CompiledModule}
    {d : StructDefinitionIndex} {mut_ : Bool} {before after : List RuntimeType}
    (h : typeAfter P f m (if mut_ then .mutBorrowGlobal d else .immBorrowGlobal d) before =
      some after) :
    ∃ sid brest, P.definedStruct? f.module d = some sid ∧
      ((RuntimeType.struct sid).abilities P).key = true ∧
      before = .address :: brest ∧ after = .ref mut_ (.struct sid) :: brest := by
  have h' : (do
      let s ← (do
        let s ← P.definedStruct? f.module d
        if ((RuntimeType.struct s).abilities P).key then some s else none)
      match before with
      | .address :: rest => some (.ref mut_ (.struct s) :: rest)
      | _ => none) = some after := by cases mut_ <;> exact h
  clear h
  cases hds : P.definedStruct? f.module d with
  | none => simp [hds] at h'
  | some sid =>
    simp only [hds, bind, Option.bind_some] at h'
    split at h'
    · next hkey =>
      simp only [Option.bind_some] at h'
      split at h'
      · next brest => exact ⟨sid, brest, rfl, hkey, rfl, (Option.some.inj h').symm⟩
      · simp at h'
    · simp at h'

/-- **A global borrow** of definition `d` at type arguments `targs`:
`MutBorrowGlobal`, `ImmBorrowGlobal`, or a generic form. The new reference is
to a resource of a type the function acquires, in a module active once, which
the frame has not borrowed (mutably, for an immutable borrow) and which no
reference parameter is into. -/
theorem refSafe_borrowGlobalAt (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {instr : Bytecode}
    {d : StructDefinitionIndex} {mut_ : Bool} {targs : List RuntimeType}
    (hctx : TopCtx P s fr info code instr view A stys tys ps)
    (hi : code.code[fr.pc]? = some instr) (hret : instr ≠ .ret)
    (hctl : instr.control = .fallThrough) (hav : ∀ a, availabilityAfter instr a = a)
    (hacqd : d ∈ acqOf P fr.function)
    (hrun : ∀ A', (ReferenceSafety.executeInner info.module view instr).run A = .ok ((), A') →
      (ReferenceSafety.executeInner info.module view
        (if mut_ then .mutBorrowGlobal d else .immBorrowGlobal d)).run A = .ok ((), A'))
    (htyp : ∀ before after, typeAfter P fr.function info.module instr before = some after →
      ∃ sid brest, Step.globalType P fr.function.module d targs = .ok sid ∧
        P.definedStruct? fr.function.module d = some sid.erase ∧
        before = .address :: brest ∧ after = .ref mut_ (.struct sid) :: brest)
    (hexec : Step.exec P s fr rest info code instr = Step.borrowGlobal P s fr rest mut_ d targs) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code instr) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  obtain ⟨sid, brest, hglob, hds, rfl, rfl⟩ := htyp _ _ hafter
  obtain ⟨va, vs, hst, hva, hvs⟩ := conformsAll_cons_right hctx.conf
  obtain ⟨a, rfl⟩ := conforms_address hva
  rw [hexec]
  by_cases hl : s.activeCount sid.module > 1
  · simp [Step.borrowGlobal, hglob, popAddress_of hst, Step.unlocked, hl,
      bind, Except.bind, Step.fail]
    trivial
  cases hg : s.storage.get? a sid with
  | none =>
    simp [Step.borrowGlobal, hglob, popAddress_of hst, Step.unlocked, hl, hg,
      bind, Except.bind, pure, Except.pure, Step.fail]
    trivial
  | some v =>
    obtain ⟨r0, hr0, hr0t, hr0v⟩ := get?_mem hg
    have hvc : v.conforms P (.struct sid) = true := by rw [← hr0v, ← hr0t]; exact hr.storage r0 hr0
    obtain ⟨rn, hrn⟩ : ∃ rn : Reference,
        rn = ⟨mut_, .struct sid, ⟨.global a sid, []⟩, s.invalidations.length⟩ := ⟨_, rfl⟩
    have hstep : Step.borrowGlobal P s fr rest mut_ d targs = .ok (.done (.running { s with
        frames := Step.advance (Step.push { fr with stack := vs } (.ref rn)) :: rest })) := by
      subst hrn
      simp [Step.borrowGlobal, hglob, popAddress_of hst, Step.unlocked, hl,
        hg, bind, Except.bind, pure, Except.pure, Step.next, Step.borrow]
    rw [hstep]
    refine ⟨?_, ?_⟩
    · exact fallThrough_safe (N := N) (after := .ref mut_ (.struct sid) :: brest) hc hact hchain hi
        hctl hav hbefore hafter rfl rfl rfl
        (by
          simp only [Step.advance, Step.push, conformsAll_cons]
          refine ⟨conforms_iff.2 ⟨by rw [hrn]; exact assignable_refl _, by rw [hrn]; rfl⟩, hvs⟩)
    · obtain ⟨A', hx, hsucc⟩ := hctx.next hret
      obtain ⟨restA, g1, g2, hsa, hmutChk, hmutB, hnew, hadd, rfl⟩ := exec_borrowGlobal (hrun _ hx)
      obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, hctl])
      obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
      obtain ⟨S0, hglob0, -⟩ := hctx.chain _ hsf
      -- The type is one the function acquires, in a module active once.
      have hacqT : acqTypes P fr.function sid := ⟨d, hacqd, hds⟩
      have hsidm : sid.module = info.moduleId := (definedStruct_module hds hact.fn :)
      have hsk : s.frames.map Frame.skel = (fr.function, fr.countedActive) :: rest.map Frame.skel := by
        rw [hsf]; rfl
      have hcount : countActive P ((fr :: rest).map Frame.skel) sid.module ≤ 1 := by
        rw [← hsf, ← hr.active.count]; omega
      have hgsid : gsidOf P (s.frames.map Frame.skel) d = some sid.erase := by
        rw [hsk]
        have hfm : funModule P fr.function = some info.moduleId := by simp [funModule, hact.fn]
        have hcount' : countActive P ((fr.function, fr.countedActive) :: rest.map Frame.skel)
            info.moduleId ≤ 1 := by rw [← hsidm]; exact hcount
        simp only [gsidOf, hfm]
        rw [if_pos ⟨hacqd, hcount'⟩]
        exact hds
      have hinj : ∀ d', gsidOf P (s.frames.map Frame.skel) d' = some sid.erase → d' = d := by
        intro d' hd'
        rw [hsk] at hd'
        exact definedStruct_inj (gsidOf_def hd').2 hds
      have hps : ∀ π ∈ ps, ∀ a', π.location.root ≠ .global a' sid := fun π hπ a' hr' => by
        have := hglob0.acq sid (hglob0.params π hπ a' sid hr') hacqT
        omega
      obtain ⟨t0, stys0, hstys⟩ : ∃ t0 stys0, stys = t0 :: stys0 := by
        have := hctx.typed.nstack
        rw [hsa] at this
        cases stys with
        | nil => simp at this
        | cons t0 stys0 => exact ⟨t0, stys0, rfl⟩
      have ht1 := hctx.typed.popValue hsa hstys
      have hlen : restA.length = vs.length := by
        have := hctx.height; rw [hsa, hst] at this; simpa using this
      have hd1 := hd.pop (A' := { A with stack := restA }) (fr' := { fr with stack := vs }) hsa hst
        hlen rfl rfl rfl rfl rfl
      have hheld1 : ∀ x, x ∈ heldIds ({ A with stack := restA } : RefState) ↔ x ∈ heldIds A := by
        intro x; simp [heldIds, hsa, AbsValue.refId?]
      have hc2 := hd1.complete.borrowGlobal (r := rn) (by rw [hrn]) hgsid hinj hps
        (fun hm => by rw [hrn] at hm; subst hm; exact hmutChk rfl) hmutB
        (by rw [hrn]; exact hnew) hadd
      have hd' := hd1.pushNew (A' := { A with
          stack := .reference A.nextId :: restA, graph := g2, nextId := A.nextId + 1 })
        (fr' := Step.advance (Step.push { fr with stack := vs } (.ref rn))) ht1
        (Graph.newRef_ok hnew).1 (by simpa using hlen) hc2 ⟨rfl, rfl, rfl⟩ rfl rfl rfl
      refine refInv_finish hr hsf (hctx.chain _ hsf) rfl rfl rfl rfl rfl rfl (Nat.le_refl _)
        hr.storage (es := []) rfl (storageKept_of_eq rfl) allowedBy_nil hd'
        (fun x r₂ hx hxr => ?_) hA'' hle
        (RefsFrom.next (new := [.ref rn]) (n := 1) (by simp [Step.advance, Step.push, hst]) rfl
          (fun w hw => by
            simp only [List.mem_singleton] at hw
            subst hw
            exact .of_loc fun k h => by rw [hrn] at h; simp at h))
      by_cases hxc : x = A.nextId
      · subst hxc
        rw [updL_self, Option.some.injEq] at hxr
        subst hxr
        have hvt : v.type = .struct sid := conforms_type hvc rfl
        refine ⟨⟨.struct sid, .struct sid, ?_, by rw [hrn]; rfl, by rw [hrn]; exact Or.inl rfl⟩, by rw [hrn]; trivial, by rw [hrn]; rfl,
          by rw [hrn]; exact Nat.le_refl _, ?_, fun _ => by rw [hrn]; rfl⟩
        · rw [hrn]
          simp [RootType, hg, conforms_wellFormed hvc, hvt]
        · rw [deref?_isSome]
          refine ⟨fun e he => ?_, ?_⟩
          · rw [hrn] at he
            simp [State.invalidatedSince] at he
          · rw [show rn.location = ⟨.global a sid, []⟩ by rw [hrn]]
            simp [State.read?, State.root?, hg, Value.get?]
      · rw [updL_ne _ hxc] at hxr
        refine (hrefs x r₂ ?_ hxr).same hsf rfl rfl rfl rfl rfl rfl rfl
        simp only [heldIds, List.filterMap_cons, AbsValue.refId?, List.mem_append, List.mem_cons]
          at hx ⊢
        rw [hsa]
        simp only [List.filterMap_cons, AbsValue.refId?]
        rcases hx with hx | hx | hx
        · exact Or.inl hx
        · exact absurd hx hxc
        · exact Or.inr hx

/-- **`MutBorrowGlobal`, `ImmBorrowGlobal`**: the new reference is to a
resource of a type the function acquires, in a module active once, which the
frame has not borrowed (mutably, for an immutable borrow) and which no
reference parameter is into. -/
theorem refSafe_borrowGlobal (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest)
    {d : StructDefinitionIndex} {mut_ : Bool}
    (hctx : TopCtx P s fr info code (if mut_ then .mutBorrowGlobal d else .immBorrowGlobal d) view A
      stys tys ps)
    (hi : code.code[fr.pc]? = some (if mut_ then .mutBorrowGlobal d else .immBorrowGlobal d)) :
    ExecSafe P N (InvRefs P)
      (Step.exec P s fr rest info code (if mut_ then .mutBorrowGlobal d else .immBorrowGlobal d)) := by
  have h1 := (acquires_instr (function_acquires hwf hact.fn hact.body)
    (getElem?_mem_toList hi)).1 d (by cases mut_ <;> simp)
  refine refSafe_borrowGlobalAt (d := d) (mut_ := mut_) (targs := []) hc hr hsf hact hchain hctx hi
    (by cases mut_ <;> intro h <;> cases h) (by cases mut_ <;> rfl) (fun _ => by cases mut_ <;> rfl)
    (by simp only [acqOf, hact.fn, Option.map_some, Option.getD_some]; exact h1) (fun _ hx => hx)
    (fun _ _ hafter => ?_) (by cases mut_ <;> rfl)
  obtain ⟨sid, brest, hds, hkey, h1, h2⟩ := typeAfter_borrowGlobal hafter
  exact ⟨sid, brest, globalType_of_key hds hkey,
    by rw [StructId.erase_of_nil (definedStruct?_typeArgs hds)]; exact hds, h1, h2⟩

end MoveBytecode.Proofs
