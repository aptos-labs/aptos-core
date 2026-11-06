-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.DanglingDefs
import Proofs.RefTrace
import Proofs.RefUpdate
import Proofs.RefExecValue

/-!
# What a step does to valid locations

UNTRUSTED (checked by the kernel). A step keeps every valid location valid
unless one of its events hits it (`coreStep_keeps`): moving a local out or
overwriting it writes the local directly, a write through a reference
changes only what is strictly inside its location, a resource moved out of
global storage is written directly, and a frame that returns ends. Every
other step, and every native call (`native_keeps`), changes no root a valid
location is in and records no invalidation.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Step

/-! ## Validity across a change of state -/

/-- Validity persists across invalidations that do not cover the location,
while the location still resolves. -/
private theorem keeps_transfer {s s' : State} {l : Location} {n : Nat} {es : List Invalidation}
    (hes : s'.invalidations = es ++ s.invalidations) (hcov : ∀ e ∈ es, e.covers l = false)
    (hread : (s.read? l).isSome = true → (s'.read? l).isSome = true) (hv : Valid s l n) :
    Valid s' l n :=
  ⟨valid_transfer hv.2 hes hcov hread hv.1, by have := hv.2; rw [hes, List.length_append]; omega⟩

/-- A step that keeps the root of a valid location and records no
invalidation keeps the location valid. -/
private theorem keeps_root {s s' : State} {l : Location} {n : Nat}
    (hroot : s'.root? l.root = s.root? l.root) (hinv : s'.invalidations = s.invalidations)
    (hv : Valid s l n) : Valid s' l n := by
  unfold Valid
  rw [deref?_congr hroot hinv, hinv]
  exact hv

/-- A direct write of a whole root hits every location in it. -/
private theorem keeps_hit_root {l : Location} {ρ : Root} (h : l.root = ρ) :
    Hits l (.accessed .write ⟨ρ, []⟩ none) :=
  Or.inl ⟨_, rfl, by simp [Access.overlaps, h]⟩

/-- A root keeps its value when the current frame is replaced by one with its
identity, which keeps the root if it is one of its locals, and global storage
keeps the root if it is a resource. -/
private theorem keeps_root_top {s s' : State} {fr fr' : Frame} {rest : List Frame} {ρ : Root}
    (hsf : s.frames = fr :: rest) (h' : s'.frames = fr' :: rest) (hid : fr'.id = fr.id)
    (hhe : s'.held = s.held)
    (hloc : ∀ j, ρ = .local_ fr.id j → fr'.locals[j]? = fr.locals[j]?)
    (hst : ∀ a t, ρ = .global a t → s'.storage.get? a t = s.storage.get? a t) :
    s'.root? ρ = s.root? ρ := by
  cases ρ with
  | local_ f j =>
    simp only [State.root?]
    by_cases hf : f = fr.id
    · subst hf
      rw [frame?_head hsf, show s'.frame? fr.id = some fr' by rw [← hid]; exact frame?_head h']
      simp [hloc j rfl]
    · rw [frame?_cons_ne hsf h' hid hf]
  | global a t => simp only [State.root?, hst a t rfl]
  | held k => simp only [State.root?, hhe]

/-- A location keeps resolving when its root keeps its value. -/
private theorem keeps_read {s s' : State} {fr fr' : Frame} {rest : List Frame} {l : Location}
    (hsf : s.frames = fr :: rest) (h' : s'.frames = fr' :: rest) (hid : fr'.id = fr.id)
    (hhe : s'.held = s.held)
    (hloc : ∀ j, l.root = .local_ fr.id j → fr'.locals[j]? = fr.locals[j]?)
    (hst : ∀ a t, l.root = .global a t → s'.storage.get? a t = s.storage.get? a t)
    (hread : (s.read? l).isSome = true) : (s'.read? l).isSome = true := by
  rw [read?_eq_of_root (keeps_root_top hsf h' hid hhe hloc hst)]
  exact hread

/-- The step replaced only the current frame, by one with its identity and
locals. -/
private def TopOnly (s s' : State) (fr : Frame) (rest : List Frame) : Prop :=
  ∃ fr', s' = { s with frames := fr' :: rest } ∧ fr'.id = fr.id ∧ fr'.locals = fr.locals

private theorem keeps_topOnly {s s' : State} {fr : Frame} {rest : List Frame} {l : Location}
    {n : Nat} (hsf : s.frames = fr :: rest) (h : TopOnly s s' fr rest) (hv : Valid s l n) :
    Valid s' l n := by
  obtain ⟨fr', rfl, hid, hloc⟩ := h
  exact keeps_root (keeps_root_top (s' := { s with frames := fr' :: rest }) hsf rfl hid rfl
    (fun j _ => by rw [hloc]) fun _ _ _ => rfl) rfl hv

/-! ## The step of an annotated state -/

/-- An annotated core step that continues runs an instruction of the current
frame, and the ghost layer annotates that instruction. -/
private theorem keeps_unfold {P : Program} {s s' : State} {h : Shadow} {post : ShadowPost}
    {evs : List Event} (hstep : step P s = .done (.running s'))
    (hc : Shadow.coreStep P s (.running s') h = some (post, evs)) :
    ∃ fr rest info code instr, s.frames = fr :: rest ∧ code.code[fr.pc]? = some instr ∧
      stepOf (Step.exec P s fr rest info code instr) = .done (.running s') ∧
      ∃ post', Shadow.exec P info.moduleId instr s fr (.running s') h = some (post', evs) := by
  unfold step at hstep
  split at hstep
  · simp at hstep
  · next fr rest hsf =>
    split at hstep
    · simp at hstep
    · next info hinfo =>
      split at hstep
      · simp at hstep
      · next code hcode =>
        split at hstep
        · simp at hstep
        · next instr hinstr =>
          refine ⟨fr, rest, info, code, instr, hsf, hinstr, hstep, ?_⟩
          unfold Shadow.coreStep at hc
          rw [hsf] at hc
          simp only [Option.bind_eq_bind, Option.bind_eq_some_iff, hinfo, hcode, hinstr,
            Option.some.injEq, exists_eq_left'] at hc
          obtain ⟨⟨post', evs'⟩, hex, hc⟩ := hc
          split at hc
          · simp only [Option.some.injEq, Prod.mk.injEq] at hc
            obtain ⟨-, rfl⟩ := hc
            exact ⟨post', hex⟩
          · cases hc


/-! ## Steps on the current frame only -/

section TopOnly

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

private theorem keeps_next {fr' : Frame} (h : stepOf (next s rest fr') = .done (.running s'))
    (hid : fr'.id = fr.id) (hloc : fr'.locals = fr.locals) : TopOnly s s' fr rest := by
  obtain rfl := Outcome.running.inj (done_next h)
  exact ⟨fr', rfl, hid, hloc⟩

private theorem keeps_finish {st : List Value} {vals : List Value}
    (h : stepOf (finish { s with frames := { fr with stack := st } :: rest } vals) =
      .done (.running s')) : TopOnly s s' fr rest := by
  obtain rfl := Outcome.running.inj (done_finish rfl h)
  exact ⟨_, rfl, rfl, rfl⟩

private theorem keeps_pop
    (h : stepOf (Step.exec P s fr rest info code .pop) = .done (.running s')) :
    TopOnly s s' fr rest := by
  obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st, -, rfl⟩ := pop_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  · exact keeps_next h rfl rfl

private theorem keeps_copyLoc {i : LocalIndex}
    (h : stepOf (Step.exec P s fr rest info code (.copyLoc i)) = .done (.running s')) :
    TopOnly s s' fr rest := by
  obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
  cases slot with
  | none => exact (done_wrong h rfl).elim
  | some v =>
    dsimp only at h
    split at h
    · exact keeps_next h rfl rfl
    · exact (done_wrong h rfl).elim

private theorem keeps_equality {positive : Bool}
    (h : stepOf (equality P s fr positive >>= fun fr' => next s rest fr') = .done (.running s')) :
    TopOnly s s' fr rest := by
  obtain ⟨fr', hx, h'⟩ := done_bind h rfl equality_ends
  obtain ⟨-, -, -, -, -, -, c, hR⟩ := equality_ok hx
  rw [hR.2] at h'
  exact keeps_next h' rfl rfl

private theorem keeps_testVariant {hh : StructVariantHandleIndex} {tys : List RuntimeType}
    (h : stepOf (Step.testEnum P s fr rest info hh tys) = .done (.running s')) :
    TopOnly s s' fr rest := by
  obtain ⟨vh, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
    obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨v, -, h⟩ := done_bind h rfl deref_ends
    split at h
    · exact keeps_finish h
    · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

private theorem keeps_readRef
    (h : stepOf (Step.exec P s fr rest info code .readRef) = .done (.running s')) :
    TopOnly s s' fr rest := by
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRef_ends
  obtain ⟨st, -, rfl⟩ := popRef_ok hpr
  dsimp only at h
  obtain ⟨v, -, h⟩ := done_bind h rfl deref_ends
  split at h
  · exact (done_wrong h rfl).elim
  · exact keeps_finish h

private theorem keeps_freezeRef
    (h : stepOf (Step.exec P s fr rest info code .freezeRef) = .done (.running s')) :
    TopOnly s s' fr rest := by
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRef_ends
  obtain ⟨st, -, rfl⟩ := popRef_ok hpr
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  · exact keeps_next h rfl rfl

/-- `PackClosure`: it pops the captured values and pushes the closure. -/
private theorem keeps_packClosure {hd : FunctionHandleIndex} {mask : BitVec 64}
    {tys : List RuntimeType}
    (h : stepOf (Step.packClosure P s fr rest info hd mask tys) = .done (.running s')) :
    TopOnly s s' fr rest := by
  unfold Step.packClosure at h
  obtain ⟨target, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨targetInfo, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨fh, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨paramTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨retTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  rcases hsplit : maskSplit mask paramTys with ⟨capturedTys, remaining⟩
  rw [hsplit] at h
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h rfl popN_ends
  obtain ⟨-, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  · exact keeps_next h rfl rfl

private theorem keeps_borrowLoc {mutable : Bool} {i : LocalIndex}
    (h : stepOf (borrowLoc P s fr rest mutable i) = .done (.running s')) : TopOnly s s' fr rest := by
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
    · exact keeps_next h rfl rfl

private theorem keeps_borrowField {mutable : Bool} {hh : FieldHandleIndex} {tys : List RuntimeType}
    (h : stepOf (borrowField P s fr rest info mutable hh tys) = .done (.running s')) :
    TopOnly s s' fr rest := by
  unfold borrowField at h
  obtain ⟨fh, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨t, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
    obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨_, -, h⟩ := done_bind h rfl deref_ends
    exact keeps_finish h
  · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

private theorem keeps_borrowVariantField {mutable : Bool} {hh : VariantFieldHandleIndex}
    {tys : List RuntimeType}
    (h : stepOf (borrowVariantField P s fr rest info mutable hh tys) = .done (.running s')) :
    TopOnly s s' fr rest := by
  unfold borrowVariantField at h
  obtain ⟨vfh, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, -, h⟩ := done_bind h rfl (mapM_ends (fun _ =>
      ends_bind (ends_need _ _) fun _ _ => ends_bind (ends_need _ _) fun _ _ => ends_need _ _) _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
    obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨v, -, h⟩ := done_bind h rfl deref_ends
    split at h
    · split at h
      · exact keeps_finish h
      · exact (done_fail h rfl).elim
    · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

private theorem keeps_borrowGlobal {mutable : Bool} {d : StructDefinitionIndex}
    {tys : List RuntimeType}
    (h : stepOf (borrowGlobal P s fr rest mutable d tys) = .done (.running s')) :
    TopOnly s s' fr rest := by
  unfold borrowGlobal at h
  obtain ⟨sid, -, h⟩ := done_bind h rfl globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h rfl popAddress_ends
  obtain ⟨st, -, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h rfl unlocked_ends
  split at h
  · exact keeps_next h rfl rfl
  · exact (done_fail h rfl).elim

private theorem keeps_vecLen {sig : SignatureIndex}
    (h : stepOf (Step.exec P s fr rest info code (.vecLen sig)) = .done (.running s')) :
    TopOnly s s' fr rest := by
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
  split at h
  · exact keeps_finish h
  · exact (done_fail h rfl).elim

private theorem keeps_borrowElement {mutable : Bool} {sig : SignatureIndex}
    (h : stepOf (borrowElement s fr rest info mutable sig) = .done (.running s')) :
    TopOnly s s' fr rest := by
  unfold borrowElement at h
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨i, fr₁⟩, hpi, h⟩ := done_bind h rfl popIndex_ends
  obtain ⟨x, st₁, -, rfl, rfl⟩ := popIndex_ok hpi
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
  split at h
  · exact keeps_finish h
  · exact (done_fail h rfl).elim

end TopOnly


/-! ## Steps that write a root directly -/

/-- Removing a resource keeps every other resource. -/
private theorem keeps_get?_remove {st : GlobalStorage} {a a' : Address} {t t' : StructId}
    (h : ¬ (a = a' ∧ t = t')) : (st.remove a t).get? a' t' = st.get? a' t' := by
  unfold GlobalStorage.get? GlobalStorage.remove
  rw [find?_filter_of (fun r hr => by
    simp only [Bool.and_eq_true, beq_iff_eq] at hr
    simp only [Bool.not_eq_true', Bool.and_eq_false_iff, beq_eq_false_iff_ne]
    by_cases ha : r.address = a
    · right; intro ht; exact h ⟨by rw [← ha, hr.1], by rw [← ht, hr.2]⟩
    · left; exact ha)]

/-- A step that changes local `i` of the current frame, and whose
invalidations cover only locations in it, keeps every valid location
elsewhere. -/
private theorem keeps_local {s s' : State} {fr fr' : Frame} {rest : List Frame} {i : Nat}
    {es : List Invalidation} {l : Location} {n : Nat} (hsf : s.frames = fr :: rest)
    (h' : s'.frames = fr' :: rest)
    (hid : fr'.id = fr.id) (hloc : ∀ j, j ≠ i → fr'.locals[j]? = fr.locals[j]?)
    (hst : s'.storage = s.storage) (hhe : s'.held = s.held)
    (hes : s'.invalidations = es ++ s.invalidations)
    (hcov : ∀ e ∈ es, ∀ l : Location, l.root ≠ .local_ fr.id i → e.covers l = false)
    (hv : Valid s l n) : Valid s' l n ∨ l.root = .local_ fr.id i := by
  by_cases hr : l.root = .local_ fr.id i
  · exact Or.inr hr
  · exact Or.inl (keeps_transfer hes (fun e he => hcov e he l hr)
      (keeps_read hsf h' hid hhe (fun j hj => hloc j fun e => hr (by rw [hj, e]))
        fun a t _ => by rw [hst]) hv)

section Direct

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

private theorem keeps_moveLoc {i : LocalIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.moveLoc i)) = .done (.running s'))
    {l : Location} {n : Nat} (hv : Valid s l n) : Valid s' l n ∨ l.root = .local_ fr.id i := by
  obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
  cases slot with
  | none => exact (done_wrong h rfl).elim
  | some v =>
    obtain rfl := Outcome.running.inj (done_next h)
    refine keeps_local (es := [.removed (.local_ fr.id i)]) hsf rfl rfl
      (fun j hj => Array.getElem?_setIfInBounds_ne hj) rfl rfl rfl ?_ hv
    intro e he l' hl'
    simp only [List.mem_singleton] at he
    subst he
    simpa [Invalidation.covers] using hl'

private theorem keeps_stLoc {i : LocalIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.stLoc i)) = .done (.running s'))
    {l : Location} {n : Nat} (hv : Valid s l n) : Valid s' l n ∨ l.root = .local_ fr.id i := by
  obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st, -, rfl⟩ := pop_ok hpop
  dsimp only at h
  obtain ⟨tys, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨t, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
  have hloc : ∀ j, j ≠ i → (fr.locals.setIfInBounds i (some v))[j]? = fr.locals[j]? :=
    fun j hj => Array.getElem?_setIfInBounds_ne hj
  split at h
  · exact (done_wrong h rfl).elim
  · cases slot with
    | none =>
      obtain rfl := Outcome.running.inj (done_next h)
      exact keeps_local (es := []) hsf rfl rfl hloc rfl rfl rfl (by simp) hv
    | some old =>
      dsimp only at h
      split at h
      · exact (done_wrong h rfl).elim
      · obtain rfl := Outcome.running.inj (done_next h)
        refine keeps_local (es := [.mutated ⟨.local_ fr.id i, []⟩]) hsf rfl rfl hloc rfl rfl rfl
          ?_ hv
        intro e he l' hl'
        simp only [List.mem_singleton] at he
        subst he
        simp [Invalidation.covers, hl']

/-- `MoveFrom` removes a resource; every other root keeps its value. -/
private theorem keeps_moveFrom {d : StructDefinitionIndex} {tys : List RuntimeType}
    (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.moveResourceFrom P s fr rest d tys) = .done (.running s'))
    {l : Location} {n : Nat} (hv : Valid s l n) :
    ∃ a sid v, fr.stack.head? = some (.address a) ∧ s.storage.get? a sid = some v ∧
      Shadow.topAfter (.running s') = some v ∧ (Valid s' l n ∨ l.root = .global a sid) := by
  obtain ⟨sid, -, h⟩ := done_bind h rfl globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h rfl popAddress_ends
  obtain ⟨st, hstk, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h rfl unlocked_ends
  split at h
  · next v hget =>
    obtain rfl := Outcome.running.inj (done_finish rfl h)
    refine ⟨a, sid, v, by simp [hstk], hget, by simp [Shadow.topAfter, advance, pushAll], ?_⟩
    by_cases hr : l.root = .global a sid
    · exact Or.inr hr
    · refine Or.inl (keeps_transfer (s := s) (es := [.removed (.global a sid)]) rfl
        (fun e he => by
          simp only [List.mem_singleton] at he
          subst he
          simpa [Invalidation.covers] using hr) (keeps_read hsf rfl rfl rfl (fun _ _ => rfl)
            fun a' t' hρ => keeps_get?_remove fun ⟨h1, h2⟩ => hr (by rw [hρ, h1, h2])) hv)
  · exact (done_fail h rfl).elim

/-- `MoveTo` publishes a resource where there was none; every root keeps its
value. -/
private theorem keeps_moveTo {d : StructDefinitionIndex} {tys : List RuntimeType}
    (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.moveResourceTo P s fr rest d tys) = .done (.running s'))
    {l : Location} {n : Nat} (hv : Valid s l n) : Valid s' l n := by
  obtain ⟨sid, -, h⟩ := done_bind h rfl globalType_ends
  obtain ⟨⟨v, fr₁⟩, hpv, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st₁, -, rfl⟩ := pop_ok hpv
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, -, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨sv, -, h⟩ := done_bind h rfl deref_ends
  split at h
  · next a =>
    obtain ⟨_, -, h⟩ := done_bind h rfl unlocked_ends
    split at h
    · exact (done_fail h rfl).elim
    · obtain rfl := Outcome.running.inj (done_finish (s₁ := { ({ s with frames :=
        { fr with stack := st } :: rest } : State) with storage := s.storage.insert a sid v })
        rfl h)
      refine keeps_root (s := s) ?_ rfl hv
      have hr := (deref?_isSome.1 hv.1).2
      refine keeps_root_top hsf rfl rfl rfl (fun _ _ => rfl) fun a' t' hρ => ?_
      simp only [State.read?, hρ, State.root?] at hr
      exact GlobalStorage.get?_insert_of_some (Option.isSome_of_isSome_bind hr)
  · exact (done_wrong h rfl).elim


end Direct

/-! ## Writes in place -/

/-- **A write in place** at `m` keeps every valid location not strictly
inside `m`. -/
private theorem keeps_write {s s' s₃ : State} {fr : Frame} {rest : List Frame}
    {st : List Value} {m : Location} {v : Value} {vals : List Value} {l : Location} {n : Nat}
    (P : Program) (hsf : s.frames = fr :: rest)
    (hu : update { s with frames := { fr with stack := st } :: rest } m v = .ok s₃)
    (hfin : stepOf (finish s₃ vals) = .done (.running s')) (hv : Valid s l n) :
    Valid s' l n ∨ l.strictlyInside m = true := by
  obtain ⟨s₂, hw, -⟩ := update_ok hu
  simp only [State.write?, Option.bind_eq_bind, Option.bind_eq_some_iff] at hw
  obtain ⟨w, hroot, w', hset, -⟩ := hw
  rw [update_eq hroot hset] at hu
  obtain rfl := Except.ok.inj hu
  obtain rfl := Outcome.running.inj (done_finish rfl hfin)
  rw [root?_replaceTop (s₁ := s) (s₂ := { s with frames := { fr with stack := st } :: rest })
    hsf rfl rfl rfl rfl rfl m.root] at hroot
  cases hin : l.strictlyInside m
  · have hcov : Invalidation.covers (.mutated m) l = false := hin
    exact Or.inl (keeps_transfer (s := s) (es := [.mutated m]) rfl (by simpa using hcov)
      (read_update (afterWrite_writeState (P := P) hsf st m w' vals) hroot hset
        (not_strictlyInside_of_covers hcov)) hv)
  · exact Or.inr rfl

/-- **Writing back** keeps every valid location not strictly inside a
location it writes. -/
theorem writeBack_keeps (P : Program) : ∀ (refs : List Reference) (ws : List Value) {s s' : State}
    {l : Location} {n : Nat}, writeBack s refs ws = some s' → Valid s l n →
    Valid s' l n ∨ ∃ r ∈ refs, l.strictlyInside r.location = true
  | [], [], s, s', l, n, h, hv => by
    simp only [writeBack, Option.some.injEq] at h
    subst h
    exact Or.inl hv
  | r :: refs, w :: ws, s, s', l, n, h, hv => by
    simp only [writeBack, bind, Option.bind_eq_some_iff] at h
    obtain ⟨s₂, hw, h⟩ := h
    simp only [State.write?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hw
    obtain ⟨v, hroot, w', hset, rfl⟩ := hw
    rw [setRoot_invalidate] at h
    cases hin : l.strictlyInside r.location
    · have hcov : Invalidation.covers (.mutated r.location) l = false := hin
      have hv₂ : Valid (writeAt s r.location w') l n :=
        keeps_transfer (s := s) (es := [.mutated r.location]) rfl (by simpa using hcov)
          (read_update (afterWrite_writeAt (P := P) s r.location w') hroot hset
            (not_strictlyInside_of_covers hcov)) hv
      rcases writeBack_keeps P refs ws h hv₂ with h1 | ⟨r', hr', h1⟩
      · exact Or.inl h1
      · exact Or.inr ⟨r', List.mem_cons_of_mem _ hr', h1⟩
    · exact Or.inr ⟨r, List.mem_cons_self, hin⟩
  | [], _ :: _, _, _, _, _, h, _ => by simp [writeBack] at h
  | _ :: _, [], _, _, _, _, h, _ => by simp [writeBack] at h

section Writes

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit} {l : Location} {n : Nat}

private theorem keeps_writeRef (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code .writeRef) = .done (.running s'))
    (hv : Valid s l n) :
    ∃ r st, fr.stack = .ref r :: st ∧ (Valid s' l n ∨ l.strictlyInside r.location = true) := by
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRef_ends
  obtain ⟨st₁, hst₁, rfl⟩ := popRef_ok hpr
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨⟨v, fr₂⟩, hpv, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st, -, rfl⟩ := pop_ok hpv
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨old, -, h⟩ := done_bind h rfl deref_ends
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨s₃, hu, h⟩ := done_bind h rfl update_ends
  exact ⟨r, st₁, hst₁, keeps_write P hsf hu h hv⟩

private theorem keeps_vecPushBack {sig : SignatureIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.vecPushBack sig)) = .done (.running s'))
    (hv : Valid s l n) :
    ∃ r v st, fr.stack = v :: .ref r :: st ∧
      (Valid s' l n ∨ l.strictlyInside r.location = true) := by
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨v, fr₁⟩, hpv, h⟩ := done_bind h rfl pop_ends
  obtain ⟨st₁, hst₁, rfl⟩ := pop_ok hpv
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, hst₂, rfl, -, -⟩ := popRefTo_ok hpr
  simp only at hst₂
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
  obtain ⟨s₃, hu, h⟩ := done_bind h rfl update_ends
  exact ⟨r, v, st, by rw [hst₁, hst₂], keeps_write P hsf hu h hv⟩

private theorem keeps_vecPopBack {sig : SignatureIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.vecPopBack sig)) = .done (.running s'))
    (hv : Valid s l n) :
    ∃ r st, fr.stack = .ref r :: st ∧ (Valid s' l n ∨ l.strictlyInside r.location = true) := by
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, hst, rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
  split at h
  · obtain ⟨s₃, hu, h⟩ := done_bind h rfl update_ends
    exact ⟨r, st, hst, keeps_write P hsf hu h hv⟩
  · exact (done_fail h rfl).elim

private theorem keeps_vecSwap {sig : SignatureIndex} (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.vecSwap sig)) = .done (.running s'))
    (hv : Valid s l n) :
    ∃ r x y st, fr.stack = .int .u64 y :: .int .u64 x :: .ref r :: st ∧
      (Valid s' l n ∨ l.strictlyInside r.location = true) := by
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨j, fr₁⟩, hpj, h⟩ := done_bind h rfl popIndex_ends
  obtain ⟨y, st₁, hst₁, rfl, rfl⟩ := popIndex_ok hpj
  obtain ⟨⟨i, fr₂⟩, hpi, h⟩ := done_bind h rfl popIndex_ends
  obtain ⟨x, st₂, hst₂, rfl, rfl⟩ := popIndex_ok hpi
  obtain ⟨⟨r, fr₃⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  obtain ⟨st, hst₃, rfl, -, -⟩ := popRefTo_ok hpr
  simp only at hst₂ hst₃
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
  split at h
  · obtain ⟨s₃, hu, h⟩ := done_bind h rfl update_ends
    exact ⟨r, x, y, st, by rw [hst₁, hst₂, hst₃], keeps_write P hsf hu h hv⟩
  · exact (done_fail h rfl).elim
  · exact (done_fail h rfl).elim

end Writes

/-! ## Calls and returns -/

/-- Entering a frame, whose identity no frame has, keeps every root. -/
private theorem keeps_enter {s s' : State} {fr callee : Frame} {rest : List Frame}
    {st : List Value} {l : Location} {n : Nat} (hsf : s.frames = fr :: rest)
    (hbelow : ∀ g ∈ s.frames, g.id < s.nextFrame)
    (h' : s'.frames = callee :: { fr with stack := st } :: rest) (hid : callee.id = s.nextFrame)
    (hst : s'.storage = s.storage) (hhe : s'.held = s.held)
    (hinv : s'.invalidations = s.invalidations) (hv : Valid s l n) : Valid s' l n := by
  refine keeps_root (s := s) ?_ hinv hv
  have hread := (deref?_isSome.1 hv.1).2
  rw [← root?_replaceTop (s₁ := s) (s₂ := { s with frames := { fr with stack := st } :: rest })
    hsf rfl rfl rfl rfl rfl l.root]
  cases hρ : l.root with
  | local_ f j =>
    cases hfr : s.frame? f with
    | none => simp [State.read?, State.root?, hρ, hfr] at hread
    | some g =>
      obtain ⟨hg, rfl⟩ := frame?_mem hfr
      have hne : (callee.id == g.id) = false := by
        rw [hid, beq_eq_false_iff_ne]
        exact Nat.ne_of_gt (hbelow g hg)
      simp only [State.root?, State.frame?, h', List.find?_cons, hne]
  | global a t => simp only [State.root?, hst]
  | held k => simp only [State.root?, hhe]

section Calls

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit} {l : Location} {n : Nat}

private theorem keeps_call {hd : FunctionHandleIndex} {tys : List RuntimeType}
    (hsf : s.frames = fr :: rest) (hbelow : ∀ g ∈ s.frames, g.id < s.nextFrame)
    (h : stepOf (Step.call P s fr rest info hd tys) = .done (.running s'))
    (hv : Valid s l n) : Valid s' l n := by
  unfold Step.call at h
  obtain ⟨callee, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨calleeInfo, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨fh, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  dsimp only at h
  obtain ⟨paramTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨retTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨hps, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨hrs, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨⟨args, fr₁⟩, hpop, h⟩ := done_bind h rfl popN_ends
  obtain ⟨-, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_native h rfl).elim
  obtain ⟨localTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h <;> split at h
  all_goals first
    | exact (done_fail h rfl).elim
    | skip
  all_goals
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq, Outcome.running.injEq] at h
    subst h
    exact keeps_enter hsf hbelow rfl (by split <;> rfl) (by split <;> rfl) (by split <;> rfl)
      (by split <;> rfl) hv

/-- `CallClosure`: it enters the closure's function, like a call. -/
private theorem keeps_callClosure {sig : SignatureIndex}
    (hsf : s.frames = fr :: rest) (hbelow : ∀ g ∈ s.frames, g.id < s.nextFrame)
    (h : stepOf (Step.callClosure P s fr rest info sig) = .done (.running s'))
    (hv : Valid s l n) : Valid s' l n := by
  unfold Step.callClosure at h
  obtain ⟨sigTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  dsimp only at h
  split at h
  · obtain ⟨ps, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨rs, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨_, hx, h⟩ := done_bind h rfl (ends_pure _)
    simp only [pure, Except.pure, Except.ok.injEq] at hx
    subst hx
    dsimp only at h
    obtain ⟨⟨c, fr₁⟩, hpop, h⟩ := done_bind h rfl pop_ends
    obtain ⟨st, -, rfl⟩ := pop_ok hpop
    dsimp only at h
    split at h
    · split at h
      · exact (done_wrong h rfl).elim
      obtain ⟨⟨provided, fr₂⟩, hpop2, h⟩ := done_bind h rfl popN_ends
      obtain ⟨-, rfl, rfl⟩ := popN_ok hpop2
      dsimp only at h
      split at h
      · exact (done_wrong h rfl).elim
      obtain ⟨targetInfo, -, h⟩ := done_bind h rfl (ends_need _ _)
      split at h
      · exact (done_wrong h rfl).elim
      obtain ⟨paramTys, -, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨retTys, -, h⟩ := done_bind h rfl (ends_need _ _)
      split at h
      · exact (done_wrong h rfl).elim
      obtain ⟨localTys, -, h⟩ := done_bind h rfl (ends_need _ _)
      split at h <;> split at h
      all_goals first
        | exact (done_fail h rfl).elim
        | skip
      all_goals
        simp only [stepOf, pure, Except.pure, StepResult.done.injEq, Outcome.running.injEq] at h
        subst h
        exact keeps_enter hsf hbelow rfl rfl rfl rfl rfl hv
    · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

private theorem keeps_ret (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code .ret) = .done (.running s'))
    (hv : Valid s l n) : Valid s' l n ∨ ∃ j, l.root = .local_ fr.id j := by
  unfold Step.exec Step.ret at h
  obtain ⟨retTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  cases rest with
  | nil => simp [stepOf, pure, Except.pure] at h
  | cons caller rest' =>
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq, Outcome.running.injEq] at h
    by_cases hr : ∃ j, l.root = .local_ fr.id j
    · exact Or.inr hr
    · left
      cases hca : fr.countedActive <;> cases hle : fr.lockEntered <;>
        simp only [hca, hle, Bool.false_eq_true, ↓reduceIte] at h <;> subst h <;>
        refine keeps_root (s := s) ?_ rfl hv <;>
        cases hρ : l.root with
        | local_ f j =>
          have hf : (fr.id == f) = false := by
            simp only [beq_eq_false_iff_ne, ne_eq]
            exact fun e => hr ⟨j, by rw [hρ, e]⟩
          simp only [State.root?, State.frame?, hsf, List.find?_cons, hf]
          cases (caller.id == f) <;> rfl
        | global a t => rfl
        | held k => rfl

end Calls

/-! ## The events of the steps that hit -/

section Events

variable {P : Program} {m : ModuleId} {s : State} {fr : Frame} {o : Outcome} {h : Shadow}
  {post : ShadowPost} {evs : List Event}

private theorem keeps_ev_moveLoc {i : Nat}
    (hex : Shadow.exec P m (.moveLoc i) s fr o h = some (post, evs)) :
    Event.accessed .write ⟨.local_ fr.id i, []⟩ none ∈ evs := by
  unfold Shadow.exec at hex
  cases hh : h.frames with
  | nil => simp [hh] at hex
  | cons g hs =>
    cases hl : g.locals[i]? with
    | none => simp [hh, hl] at hex
    | some ot =>
      cases ot with
      | none => simp [hh, hl] at hex
      | some t =>
        simp only [hh, hl, pure, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, rfl⟩ := hex
        simp

private theorem keeps_ev_stLoc {i : Nat}
    (hex : Shadow.exec P m (.stLoc i) s fr o h = some (post, evs)) :
    Event.accessed .write ⟨.local_ fr.id i, []⟩ none ∈ evs := by
  unfold Shadow.exec at hex
  cases hh : h.frames with
  | nil => simp [hh] at hex
  | cons g hs =>
    cases hgs : g.stack with
    | nil => simp [hh, hgs] at hex
    | cons t st =>
      simp only [hh, hgs, pure, Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp

private theorem keeps_ev_ret (hex : Shadow.exec P m .ret s fr o h = some (post, evs)) :
    ∃ rs, Event.frameEnded fr.id rs ∈ evs := by
  unfold Shadow.exec at hex
  cases hh : h.frames with
  | nil => simp [hh] at hex
  | cons g hs =>
    cases hs with
    | nil =>
      simp only [hh, Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp
    | cons c hs' =>
      simp only [hh, Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp

private theorem keeps_ev_writeRef {r : Reference} {st : List Value}
    (hst : fr.stack = .ref r :: st) (hex : Shadow.exec P m .writeRef s fr o h = some (post, evs)) :
    ∃ t, Event.accessed .write r.location (some t) ∈ evs := by
  unfold Shadow.exec at hex
  cases hh : h.frames with
  | nil => simp [hh] at hex
  | cons g hs =>
    simp only [hh, hst, List.getElem?_cons_zero, Option.bind_eq_bind,
      Option.bind_eq_some_iff] at hex
    obtain ⟨a, ⟨t, -, ha⟩, _tv, -, _h₂, -, hfin⟩ := hex
    simp only [pure, Option.some.injEq, Prod.mk.injEq] at ha hfin
    obtain ⟨-, rfl⟩ := hfin
    subst ha
    exact ⟨t.id, by simp⟩

private theorem keeps_ev_vecPushBack {r : Reference} {v : Value} {st : List Value}
    {sig : SignatureIndex} (hst : fr.stack = v :: .ref r :: st)
    (hex : Shadow.exec P m (.vecPushBack sig) s fr o h = some (post, evs)) :
    ∃ t, Event.accessed .write r.location (some t) ∈ evs := by
  unfold Shadow.exec at hex
  cases hh : h.frames with
  | nil => simp [hh] at hex
  | cons g hs =>
    simp only [hh, hst, List.getElem?_cons_succ, List.getElem?_cons_zero, Option.bind_eq_bind,
      Option.bind_eq_some_iff] at hex
    obtain ⟨_tv, -, a, ⟨t, -, ha⟩, _h₂, -, hfin⟩ := hex
    simp only [pure, Option.some.injEq, Prod.mk.injEq] at ha hfin
    obtain ⟨-, rfl⟩ := hfin
    subst ha
    exact ⟨t.id, by simp⟩

private theorem keeps_ev_vecPopBack {r : Reference} {st : List Value} {sig : SignatureIndex}
    (hst : fr.stack = .ref r :: st)
    (hex : Shadow.exec P m (.vecPopBack sig) s fr o h = some (post, evs)) :
    ∃ t, Event.accessed .write r.location (some t) ∈ evs := by
  unfold Shadow.exec at hex
  cases hh : h.frames with
  | nil => simp [hh] at hex
  | cons g hs =>
    simp only [hh, hst, List.getElem?_cons_zero, Option.bind_eq_bind,
      Option.bind_eq_some_iff] at hex
    obtain ⟨a, ⟨t, -, ha⟩, _tl, -, _last, -, _h₃, -, hfin⟩ := hex
    simp only [pure, Option.some.injEq] at ha
    subst ha
    split at hfin
    · simp only [pure, Option.some.injEq, Prod.mk.injEq] at hfin
      obtain ⟨-, rfl⟩ := hfin
      exact ⟨t.id, by simp⟩
    · cases hfin

private theorem keeps_ev_vecSwap {r : Reference} {x y : Int} {st : List Value}
    {sig : SignatureIndex} (hst : fr.stack = .int .u64 y :: .int .u64 x :: .ref r :: st)
    (hex : Shadow.exec P m (.vecSwap sig) s fr o h = some (post, evs)) :
    ∃ t, Event.accessed .write r.location (some t) ∈ evs := by
  unfold Shadow.exec at hex
  cases hh : h.frames with
  | nil => simp [hh] at hex
  | cons g hs =>
    simp only [hh, hst, List.getElem?_cons_succ, List.getElem?_cons_zero, Option.bind_eq_bind,
      Option.bind_eq_some_iff] at hex
    obtain ⟨a, ⟨t, -, ha⟩, _h₁, -, hfin⟩ := hex
    simp only [pure, Option.some.injEq, Prod.mk.injEq] at ha hfin
    obtain ⟨-, rfl⟩ := hfin
    subst ha
    exact ⟨t.id, by simp⟩

private theorem keeps_ev_moveFrom {a : Address} {instr : Bytecode}
    (hi : (∃ d, instr = .moveFrom d) ∨ ∃ i, instr = .moveFromGeneric i)
    (hhead : fr.stack.head? = some (.address a))
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) :
    ∃ v sid, Shadow.topAfter o = some v ∧ v.type = .struct sid ∧
      Event.accessed .write ⟨.global a sid, []⟩ none ∈ evs := by
  rcases hi with ⟨d, rfl⟩ | ⟨i, rfl⟩
  all_goals
    unfold Shadow.exec at hex
    cases hh : h.frames with
    | nil => simp [hh] at hex
    | cons g hs =>
      simp only [hh, hhead, Option.bind_eq_bind, Option.bind_eq_some_iff] at hex
      obtain ⟨v, hv, hm⟩ := hex
      split at hm
      · next sid htype =>
        simp only [Option.bind_eq_some_iff] at hm
        obtain ⟨k, -, t, -, hfin⟩ := hm
        simp only [pure, Option.some.injEq, Prod.mk.injEq] at hfin
        obtain ⟨-, rfl⟩ := hfin
        exact ⟨v, sid, hv, htype, by simp⟩
      · cases hm

end Events


/-- A resource holds a value of its type. -/
private theorem keeps_get?_type {st : GlobalStorage}
    (ht : ∀ r ∈ st, r.value.type = .struct r.type) {a : Address} {sid : StructId} {v : Value}
    (h : st.get? a sid = some v) : v.type = .struct sid := by
  unfold GlobalStorage.get? at h
  obtain ⟨r, hf, rfl⟩ := Option.map_eq_some_iff.1 h
  have hp := List.find?_some hf
  simp only [Bool.and_eq_true, beq_iff_eq] at hp
  rw [ht r (List.mem_of_find?_eq_some hf), hp.2]

/-! ## Steps -/

/-- **A core step keeps every valid location valid**, unless one of its
events hits it: a direct write overlapping it, a write through a reference
strictly enclosing it, or the end of its frame. -/
theorem coreStep_keeps {P : Program} {s s' : State} {h : Shadow} {post : ShadowPost}
    {evs : List Event} (hpre : Pre false P s h) (_htyped : StateTyped P s)
    (hstep : step P s = .done (.running s'))
    (hc : Shadow.coreStep P s (.running s') h = some (post, evs))
    {l : Location} {n : Nat} (hv : Valid s l n) :
    Valid s' l n ∨ ∃ e ∈ evs, Hits l e := by
  obtain ⟨fr, rest, info, code, instr, hsf, hi, h, post', hex⟩ := keeps_unfold hstep hc
  cases hval : instr.ValueOnly
  · cases instr
    all_goals first
      | (simp [Bytecode.ValueOnly] at hval; done)
      | skip
    case pop => exact Or.inl (keeps_topOnly hsf (keeps_pop h) hv)
    case ret =>
      rcases keeps_ret hsf h hv with hv' | ⟨j, hj⟩
      · exact Or.inl hv'
      · obtain ⟨rs, hm⟩ := keeps_ev_ret hex
        exact Or.inr ⟨_, hm, Or.inr (Or.inr ⟨fr.id, rs, j, rfl, hj⟩)⟩
    case copyLoc => exact Or.inl (keeps_topOnly hsf (keeps_copyLoc h) hv)
    case moveLoc =>
      rcases keeps_moveLoc hsf h hv with hv' | hr
      · exact Or.inl hv'
      · exact Or.inr ⟨_, keeps_ev_moveLoc hex, keeps_hit_root hr⟩
    case stLoc =>
      rcases keeps_stLoc hsf h hv with hv' | hr
      · exact Or.inl hv'
      · exact Or.inr ⟨_, keeps_ev_stLoc hex, keeps_hit_root hr⟩
    case call => exact Or.inl (keeps_call hsf hpre.inv.framesBelow h hv)
    case eq => exact Or.inl (keeps_topOnly hsf (keeps_equality (positive := true) h) hv)
    case neq => exact Or.inl (keeps_topOnly hsf (keeps_equality (positive := false) h) hv)
    case testVariant => exact Or.inl (keeps_topOnly hsf (keeps_testVariant h) hv)
    case readRef => exact Or.inl (keeps_topOnly hsf (keeps_readRef h) hv)
    case writeRef =>
      obtain ⟨r, st, hst, hv' | hin⟩ := keeps_writeRef hsf h hv
      · exact Or.inl hv'
      · obtain ⟨t, hm⟩ := keeps_ev_writeRef hst hex
        exact Or.inr ⟨_, hm, Or.inr (Or.inl ⟨_, t, rfl, hin⟩)⟩
    case freezeRef => exact Or.inl (keeps_topOnly hsf (keeps_freezeRef h) hv)
    case mutBorrowLoc => exact Or.inl (keeps_topOnly hsf (keeps_borrowLoc h) hv)
    case immBorrowLoc => exact Or.inl (keeps_topOnly hsf (keeps_borrowLoc h) hv)
    case mutBorrowField => exact Or.inl (keeps_topOnly hsf (keeps_borrowField h) hv)
    case immBorrowField => exact Or.inl (keeps_topOnly hsf (keeps_borrowField h) hv)
    case mutBorrowVariantField =>
      exact Or.inl (keeps_topOnly hsf (keeps_borrowVariantField h) hv)
    case immBorrowVariantField =>
      exact Or.inl (keeps_topOnly hsf (keeps_borrowVariantField h) hv)
    case vecLen => exact Or.inl (keeps_topOnly hsf (keeps_vecLen h) hv)
    case vecImmBorrow => exact Or.inl (keeps_topOnly hsf (keeps_borrowElement h) hv)
    case vecMutBorrow => exact Or.inl (keeps_topOnly hsf (keeps_borrowElement h) hv)
    case vecPushBack =>
      obtain ⟨r, v, st, hst, hv' | hin⟩ := keeps_vecPushBack hsf h hv
      · exact Or.inl hv'
      · obtain ⟨t, hm⟩ := keeps_ev_vecPushBack hst hex
        exact Or.inr ⟨_, hm, Or.inr (Or.inl ⟨_, t, rfl, hin⟩)⟩
    case vecPopBack =>
      obtain ⟨r, st, hst, hv' | hin⟩ := keeps_vecPopBack hsf h hv
      · exact Or.inl hv'
      · obtain ⟨t, hm⟩ := keeps_ev_vecPopBack hst hex
        exact Or.inr ⟨_, hm, Or.inr (Or.inl ⟨_, t, rfl, hin⟩)⟩
    case vecSwap =>
      obtain ⟨r, x, y, st, hst, hv' | hin⟩ := keeps_vecSwap hsf h hv
      · exact Or.inl hv'
      · obtain ⟨t, hm⟩ := keeps_ev_vecSwap hst hex
        exact Or.inr ⟨_, hm, Or.inr (Or.inl ⟨_, t, rfl, hin⟩)⟩
    case moveFrom d =>
      obtain ⟨a, sid, v, hhead, hget, htop, hv' | hr⟩ := keeps_moveFrom hsf h hv
      · exact Or.inl hv'
      · obtain ⟨v', sid', htop', htype, hm⟩ := keeps_ev_moveFrom (Or.inl ⟨d, rfl⟩) hhead hex
        rw [htop, Option.some.injEq] at htop'
        subst htop'
        rw [keeps_get?_type hpre.inv.typed hget, RuntimeType.struct.injEq] at htype
        subst htype
        exact Or.inr ⟨_, hm, keeps_hit_root hr⟩
    case moveFromGeneric i =>
      obtain ⟨x, -, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨tys, -, h⟩ := done_bind h rfl genericArgs_ends
      obtain ⟨a, sid, v, hhead, hget, htop, hv' | hr⟩ := keeps_moveFrom hsf h hv
      · exact Or.inl hv'
      · obtain ⟨v', sid', htop', htype, hm⟩ := keeps_ev_moveFrom (Or.inr ⟨i, rfl⟩) hhead hex
        rw [htop, Option.some.injEq] at htop'
        subst htop'
        rw [keeps_get?_type hpre.inv.typed hget, RuntimeType.struct.injEq] at htype
        subst htype
        exact Or.inr ⟨_, hm, keeps_hit_root hr⟩
    case moveTo => exact Or.inl (keeps_moveTo hsf h hv)
    case moveToGeneric =>
      obtain ⟨x, -, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨tys, -, h⟩ := done_bind h rfl genericArgs_ends
      exact Or.inl (keeps_moveTo hsf h hv)
    case mutBorrowGlobal => exact Or.inl (keeps_topOnly hsf (keeps_borrowGlobal h) hv)
    case immBorrowGlobal => exact Or.inl (keeps_topOnly hsf (keeps_borrowGlobal h) hv)
    case callGeneric =>
      obtain ⟨x, -, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨tys, -, h⟩ := done_bind h rfl genericArgs_ends
      exact Or.inl (keeps_call hsf hpre.inv.framesBelow h hv)
    case testVariantGeneric | mutBorrowFieldGeneric | immBorrowFieldGeneric
      | mutBorrowVariantFieldGeneric | immBorrowVariantFieldGeneric | mutBorrowGlobalGeneric
      | immBorrowGlobalGeneric =>
      obtain ⟨x, -, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨tys, -, h⟩ := done_bind h rfl genericArgs_ends
      first
        | exact Or.inl (keeps_topOnly hsf (keeps_testVariant h) hv)
        | exact Or.inl (keeps_topOnly hsf (keeps_borrowField h) hv)
        | exact Or.inl (keeps_topOnly hsf (keeps_borrowVariantField h) hv)
        | exact Or.inl (keeps_topOnly hsf (keeps_borrowGlobal h) hv)
    case packGeneric | unpackGeneric | packVariantGeneric | unpackVariantGeneric
      | existsGeneric =>
      obtain ⟨⟨fr', new, j, hfr, hid, -, hloc, -, -, -, -⟩, hinv, -, hst, hhe, -⟩ :=
        exec_valueGeneric hi rfl h
      exact Or.inl (keeps_root (keeps_root_top hsf hfr hid hhe (fun _ _ => by rw [hloc])
        fun _ _ _ => by rw [hst]) hinv hv)
    case packClosure => exact Or.inl (keeps_topOnly hsf (keeps_packClosure h) hv)
    case packClosureGeneric =>
      obtain ⟨x, -, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨tys, -, h⟩ := done_bind h rfl genericArgs_ends
      exact Or.inl (keeps_topOnly hsf (keeps_packClosure h) hv)
    case callClosure => exact Or.inl (keeps_callClosure hsf hpre.inv.framesBelow h hv)
    all_goals exact (done_wrong h rfl).elim
  · obtain ⟨⟨fr', new, j, hfr, hid, -, hloc, -, -, -, -⟩, hinv, -, hst, hhe, -⟩ :=
      exec_value hi hval h
    exact Or.inl (keeps_root (keeps_root_top hsf hfr hid hhe (fun _ _ => by rw [hloc])
      fun _ _ _ => by rw [hst]) hinv hv)

/-- **A native call keeps every valid location valid**: it changes only the
current frame's operand stack. -/
theorem native_keeps {P : Program} {s s₁ s' : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace}
    (hstep : step P s = .callNative s₁ f args views) (hshape : P.nativeShape? f = none)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s')
    {l : Location} {n : Nat} (hv : Valid s l n) : Valid s' l n := by
  have hcont : (resumeNative P s₁ f args (.returned vals writes places)).continues = true := by
    rw [ho]; rfl
  obtain ⟨fr, rest, k, hsf, -, -, rfl, hfacts⟩ := step_callNative hstep
  obtain ⟨ptys, hpt, hconfa⟩ := hfacts.params
  have hvv := hfacts.values hshape
  have hs' := (resume_running (fr := { fr with stack := fr.stack.drop k }) (rest := rest) rfl hshape
    (noRef_of_conformsAll hconfa (hvv.1 _ hpt)) hvv.2 hcont).1
  rw [ho, Outcome.running.injEq] at hs'
  subst hs'
  exact keeps_topOnly hsf ⟨_, rfl, rfl, rfl⟩ hv

/-- **A call of a modeled native keeps every valid location valid**, but the
locations strictly inside one it writes back through. -/
theorem native_keeps_modeled {P : Program} {ν : NativeFlows} {s s₁ s' : State} {h h' : Shadow}
    {f : FunctionRef} {args views vals writes : List Value} {places : List NativePlace}
    {evs : List Event} {k : NativeShape} (hshape : P.nativeShape? f = some k)
    (hstep : step P s = .callNative s₁ f args views)
    (ho : resumeNative P s₁ f args (.returned vals writes places) = .running s')
    (hn : Shadow.nativeStep P ν s f args views vals writes places (.running s') h =
      some (.running h', evs)) {l : Location} {n : Nat} (hv : Valid s l n) :
    Valid s' l n ∨ ∃ e ∈ evs, Hits l e := by
  obtain ⟨fr, rest, m, g, hs, F, results, resultTrees, ts, hsf, -, -, -, -, hwb, -, -, -, -, -, -,
    -, -, hwev, -⟩ := nativeStep_decomp hshape hstep ho hn
  have hv₁ := keeps_topOnly (s' := { s with
      frames := { fr with
        stack := results.reverse ++ fr.stack.drop m
        pc := fr.pc + 1 } :: rest }) hsf ⟨_, rfl, rfl, rfl⟩ hv
  rcases writeBack_keeps P _ writes hwb hv₁ with h1 | ⟨r, hr, h1⟩
  · exact Or.inl h1
  · obtain ⟨x, hx, rfl⟩ := List.mem_map.1 hr
    exact Or.inr ⟨_, hwev x hx, Or.inr (Or.inl ⟨_, _, rfl, h1⟩)⟩

end MoveBytecode.Proofs
