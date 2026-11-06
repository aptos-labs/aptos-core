-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Stability

/-!
# A derived reference is inside the one it derives from

UNTRUSTED (checked by the kernel). `FreezeRef` and the borrows of a field, a
variant field or a vector element derive a reference from the one on the
operand stack, which the step uses: at its location (`FreezeRef`) or one step
inside it, and immutable when it is (`traced_borrowsInside`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Step

/-- **A reference a step derives from another**: the step's result, from the
reference at depth `k` of the operand stack, which the step uses. -/
theorem exec_derived {P : Program} {m : ModuleId} {instr : Bytecode} {s : State} {fr : Frame}
    {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event} {g : ShadowFrame}
    {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) {d t : Nat} {rd : Reference}
    (hb : Event.borrowed d rd (some t) ∈ evs) :
    Shadow.topAfter o = some (.ref rd) ∧ ∃ k rp, fr.stack[k]? = some (.ref rp) ∧
      Event.used t rp ∈ evs ∧
      (instr = .freezeRef ∧ k = 0 ∨
        (∃ j, instr = .mutBorrowField j ∨ instr = .immBorrowField j ∨
          instr = .mutBorrowVariantField j ∨ instr = .immBorrowVariantField j ∨
          instr = .mutBorrowFieldGeneric j ∨ instr = .immBorrowFieldGeneric j ∨
          instr = .mutBorrowVariantFieldGeneric j ∨ instr = .immBorrowVariantFieldGeneric j) ∧
          k = 0 ∨
        (∃ j, instr = .vecImmBorrow j ∨ instr = .vecMutBorrow j) ∧ k = 1) := by
  obtain ⟨htop, hnew⟩ := exec_borrowed hh hex hb
  refine ⟨htop, ?_⟩
  rcases hnew with ⟨rfl, -⟩ | ⟨j, rfl | rfl⟩ | ⟨j, rfl | rfl | rfl | rfl⟩ |
    ⟨j, rfl | rfl | rfl | rfl⟩ | ⟨j, rfl | rfl⟩ | ⟨j, rfl | rfl | rfl | rfl⟩ <;>
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      guard] at hex
  · -- `FreezeRef`.
    obtain ⟨a, hra, _, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp at hb
      obtain ⟨-, -, rfl⟩ := hb
      obtain ⟨h1, -, -⟩ := refAt_spec hra
      exact ⟨0, a.2, h1, by simp, by simp⟩
    · cases hm
  all_goals first
    | (obtain ⟨_, -, hm⟩ := hex
       split at hm
       · simp only [Option.some.injEq, Prod.mk.injEq] at hm
         obtain ⟨-, rfl⟩ := hm
         simp at hb
       · cases hm)
    | (obtain ⟨a, hra, hm⟩ := hex
       split at hm
       · obtain ⟨_, -, hm⟩ := Option.bind_eq_some_iff.1 hm
         split at hm
         · simp only [Option.some.injEq, Prod.mk.injEq] at hm
           obtain ⟨-, rfl⟩ := hm
           simp at hb
           obtain ⟨-, -, rfl⟩ := hb
           obtain ⟨h1, -, -⟩ := refAt_spec hra
           exact ⟨0, a.2, h1, by simp, by simp⟩
         · cases hm
       · cases hm)
    | (obtain ⟨a, hra, _, -, hm⟩ := hex
       split at hm
       · simp only [Option.some.injEq, Prod.mk.injEq] at hm
         obtain ⟨-, rfl⟩ := hm
         simp at hb
         obtain ⟨-, -, rfl⟩ := hb
         obtain ⟨h1, -, -⟩ := refAt_spec hra
         exact ⟨1, a.2, h1, by simp, by simp⟩
       · cases hm)
    | (split at hex
       · split at hex
         · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
           obtain ⟨-, -, -, rfl⟩ := hex
           simp at hb
         · cases hex
       · cases hex)

/-- A location one step inside another is inside it. -/
theorem child_inside (l : Location) (st : PathStep) : (l.child st).Inside l :=
  ⟨rfl, List.prefix_append _ _⟩

/-- What a step's derived reference is, against the reference on the operand
stack it derives from. -/
def DerivedFrom (rd rp : Reference) : Prop :=
  rd.location.Inside rp.location ∧ (rp.mutable = false → rd.mutable = false)

private theorem bi_freeze {P : Program} {s s' : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {rd rp : Reference}
    (h : stepOf (Step.exec P s fr rest info code .freezeRef) = .done (.running s'))
    (htop : Shadow.topAfter (.running s') = some (.ref rd))
    (hrp : fr.stack[0]? = some (.ref rp)) : DerivedFrom rd rp := by
  have hcont : (Outcome.running s').continues = true := rfl
  obtain ⟨⟨rp', fr₁⟩, hpr, h⟩ := done_bind h hcont popRef_ends
  obtain ⟨st, hst, rfl⟩ := popRef_ok hpr
  rw [hst] at hrp
  simp only [List.getElem?_cons_zero, Option.some.injEq, Value.ref.injEq] at hrp
  subst hrp
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  · have ho := Outcome.running.inj (done_next h)
    subst ho
    simp only [Shadow.topAfter, List.head?_cons, advance, push, Option.bind_eq_bind,
      Option.bind_some, Option.some.injEq, Value.ref.injEq] at htop
    subst htop
    exact ⟨Location.Inside.refl _, fun _ => rfl⟩

private theorem bi_field {P : Program} {s s' : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {mutable : Bool} {hh : FieldHandleIndex} {tys : List RuntimeType}
    {rd rp : Reference}
    (h : stepOf (borrowField P s fr rest info mutable hh tys) = .done (.running s'))
    (htop : Shadow.topAfter (.running s') = some (.ref rd))
    (hrp : fr.stack[0]? = some (.ref rp)) : DerivedFrom rd rp := by
  have hcont : (Outcome.running s').continues = true := rfl
  unfold borrowField at h
  obtain ⟨fh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨t, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨rp', fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hst, rfl, -, hmut⟩ := popRefTo_ok hpr
    rw [hst] at hrp
    simp only [List.getElem?_cons_zero, Option.some.injEq, Value.ref.injEq] at hrp
    subst hrp
    dsimp only at h
    obtain ⟨w, -, h⟩ := done_bind h hcont deref_ends
    have ho := Outcome.running.inj (done_finish rfl h)
    subst ho
    simp only [Shadow.topAfter, List.head?_cons, advance, pushAll, borrow, List.reverse_cons,
      List.reverse_nil, List.nil_append, List.cons_append, Option.bind_eq_bind,
      Option.bind_some, Option.some.injEq, Value.ref.injEq] at htop
    subst htop
    refine ⟨child_inside _ _, fun hm => ?_⟩
    cases hb : mutable
    · rfl
    · rw [hmut hb] at hm; cases hm
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

private theorem bi_variantField {P : Program} {s s' : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {mutable : Bool} {hh : VariantFieldHandleIndex}
    {tys : List RuntimeType} {rd rp : Reference}
    (h : stepOf (borrowVariantField P s fr rest info mutable hh tys) = .done (.running s'))
    (htop : Shadow.topAfter (.running s') = some (.ref rd))
    (hrp : fr.stack[0]? = some (.ref rp)) : DerivedFrom rd rp := by
  have hcont : (Outcome.running s').continues = true := rfl
  unfold borrowVariantField at h
  obtain ⟨vfh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, -, h⟩ := done_bind h hcont (mapM_ends (fun _ =>
      ends_bind (ends_need _ _) fun _ _ => ends_bind (ends_need _ _) fun _ _ => ends_need _ _) _)
    obtain ⟨⟨rp', fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hst, rfl, -, hmut⟩ := popRefTo_ok hpr
    rw [hst] at hrp
    simp only [List.getElem?_cons_zero, Option.some.injEq, Value.ref.injEq] at hrp
    subst hrp
    dsimp only at h
    obtain ⟨w, -, h⟩ := done_bind h hcont deref_ends
    split at h
    · split at h
      · have ho := Outcome.running.inj (done_finish rfl h)
        subst ho
        simp only [Shadow.topAfter, List.head?_cons, advance, pushAll, borrow, List.reverse_cons,
          List.reverse_nil, List.nil_append, List.cons_append, Option.bind_eq_bind,
          Option.bind_some, Option.some.injEq, Value.ref.injEq] at htop
        subst htop
        refine ⟨child_inside _ _, fun hm => ?_⟩
        cases hb : mutable
        · rfl
        · rw [hmut hb] at hm; cases hm
      · exact (done_fail h hcont).elim
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

private theorem bi_element {s s' : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {mutable : Bool} {sig : SignatureIndex} {rd rp : Reference}
    (h : stepOf (borrowElement s fr rest info mutable sig) = .done (.running s'))
    (htop : Shadow.topAfter (.running s') = some (.ref rd))
    (hrp : fr.stack[1]? = some (.ref rp)) : DerivedFrom rd rp := by
  have hcont : (Outcome.running s').continues = true := rfl
  unfold borrowElement at h
  obtain ⟨t, -, h⟩ := done_bind h hcont elemType_ends
  obtain ⟨⟨i, fr₁⟩, hpi, h⟩ := done_bind h hcont popIndex_ends
  obtain ⟨x, st₁, hst₁, rfl, rfl⟩ := popIndex_ok hpi
  obtain ⟨⟨rp', fr₂⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
  obtain ⟨st, hst, rfl, -, hmut⟩ := popRefTo_ok hpr
  simp only at hst
  rw [hst₁, hst] at hrp
  simp only [List.getElem?_cons_succ, List.getElem?_cons_zero, Option.some.injEq,
    Value.ref.injEq] at hrp
  subst hrp
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h hcont derefVector_ends
  split at h
  · have ho := Outcome.running.inj (done_finish rfl h)
    subst ho
    simp only [Shadow.topAfter, List.head?_cons, advance, pushAll, borrow, List.reverse_cons,
      List.reverse_nil, List.nil_append, List.cons_append, Option.bind_eq_bind,
      Option.bind_some, Option.some.injEq, Value.ref.injEq] at htop
    subst htop
    refine ⟨child_inside _ _, fun hm => ?_⟩
    cases hb : mutable
    · rfl
    · rw [hmut hb] at hm; cases hm
  · exact (done_fail h hcont).elim

/-- **A traced step's derived references are inside the ones they derive
from** (`BorrowsInside`). -/
theorem traced_borrowsInside {P : Program} {N : NativeRel} {ν : NativeFlows} {s : State}
    {h : Shadow} {o : Outcome} {post : ShadowPost} {evs : List Event}
    (hT : TracedStep P N ν s h o post evs) : BorrowsInside evs := by
  intro d rd t hb
  cases hT with
  | core hstep hc =>
    obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hex⟩ := coreStep_exec hc
    obtain ⟨hframes, hst, hhe, fresh⟩ := h
    cases hframes with
    | nil => simp [Shadow.exec] at hex
    | cons g hs =>
      obtain ⟨htop, k, rp, hrp, hu, hk⟩ := exec_derived rfl hex hb
      refine ⟨rp, hu, ?_⟩
      have hst' : stepOf (Step.exec P s fr rest info code instr) = .done o := by
        unfold step at hstep
        rw [hsf] at hstep
        simp only [hinfo, hcode, hinstr] at hstep
        exact hstep
      cases o with
      | running s' =>
        rcases hk with ⟨rfl, rfl⟩ |
          ⟨⟨j, rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl⟩, rfl⟩ | ⟨⟨j, rfl | rfl⟩, rfl⟩
        · exact bi_freeze hst' htop hrp
        · exact bi_field hst' htop hrp
        · exact bi_field hst' htop hrp
        · exact bi_variantField hst' htop hrp
        · exact bi_variantField hst' htop hrp
        all_goals first
          | exact bi_element hst' htop hrp
          | (obtain ⟨x, -, hst'⟩ := done_bind hst' rfl (ends_need _ _)
             obtain ⟨tys, -, hst'⟩ := done_bind hst' rfl genericArgs_ends
             first
               | exact bi_field hst' htop hrp
               | exact bi_variantField hst' htop hrp)
      | returned _ _ => simp [Shadow.topAfter] at htop
      | failed _ => simp [Shadow.topAfter] at htop
      | wrong _ => simp [Shadow.topAfter] at htop
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      obtain ⟨tys, -, hfits⟩ := nativeStep_fits hstep hn
      obtain ⟨-, -, -, -, p, hp, -⟩ := nativeStep_borrowed hn hb
      have hkb := places_borrow hfits (List.ne_nil_of_mem hp)
      obtain ⟨-, -, -, -, r0, t0, m, tgt, p, d', -, -, -, -, hmut, -, -, hu, -, hall, -⟩ :=
        nativeStep_borrowEvents hstep hn hkb
      rcases hall _ hb with he | he | he | ⟨_, _, _, he⟩ | ⟨_, _, _, he⟩ <;> cases he
      refine ⟨r0, hu, ⟨rfl, List.prefix_append _ _⟩, fun hm0 => ?_⟩
      cases hm : m
      · rfl
      · rw [hmut hm] at hm0; cases hm0
    obtain ⟨rfl, rfl, rfl, g, hs, info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    have hm : d ∈ borrowedIds evs := mem_borrowedIds.2 ⟨rd, some t, hb⟩
    rw [hevs, nativeEvents_borrowedIds] at hm
    cases hm

end MoveBytecode.Proofs
