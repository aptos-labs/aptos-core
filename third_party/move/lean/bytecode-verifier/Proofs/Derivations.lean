-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Stability
import Proofs.Containment
import Proofs.BorrowInside

/-!
# Derivations and frame entries along annotated executions

UNTRUSTED (checked by the kernel). Facts about the events of annotated
executions that D4d's returns (`RefSafety.returnsDerived`) needs:

- what `Derives`, `RefSafety.derivesFrom`, `RefSafety.params?` and
  `RefSafety.returnsDerived` say of more events (they are monotone);
- the executable derivation from a `Derives` chain (`derivesFrom_of_derives`):
  each link goes to a strictly smaller borrowed identity, so the fuel of
  `RefSafety.derivesFrom` suffices;
- along a trace, a reference is borrowed after the one it derives from
  (`trace_parent_lt`), every frame entered has an identity below the next one
  (`trace_entered_lt`), and the initial events enter the entry frame with its
  reference parameters (`initial_entry`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Step

/-! ## More events -/

theorem Derives.append {E F : List Event} {r d : Nat} (h : Derives E r d) :
    Derives (E ++ F) r d := by
  induction h with
  | refl => exact .refl
  | step hb _ ih => exact .step (List.mem_append_left _ hb) ih

theorem Derives.trans {E : List Event} {a b c : Nat} (hab : Derives E a b)
    (hbc : Derives E b c) : Derives E a c := by
  induction hbc with
  | refl => exact hab
  | step hb _ ih => exact .step hb ih

theorem creation?_append {E F : List Event} {d : Nat} {x : Reference × Option Nat}
    (h : RefSafety.creation? E d = some x) : RefSafety.creation? (E ++ F) d = some x := by
  unfold RefSafety.creation? at h ⊢
  rw [List.findSome?_append, h]
  rfl

theorem derivesFrom_go_append {E F : List Event} {r : Nat} :
    ∀ {n x : Nat}, RefSafety.derivesFrom.go E r n x = true →
      RefSafety.derivesFrom.go (E ++ F) r n x = true
  | 0, _, h => by simp [RefSafety.derivesFrom.go] at h
  | n + 1, x, h => by
    simp only [RefSafety.derivesFrom.go, Bool.or_eq_true, beq_iff_eq] at h ⊢
    rcases h with h | h
    · exact Or.inl h
    · right
      split at h
      · next rd t hc =>
        rw [creation?_append hc]
        exact derivesFrom_go_append h
      · cases h
      · cases h

theorem derivesFrom_go_mono {E : List Event} {r : Nat} :
    ∀ {n m x : Nat}, n ≤ m → RefSafety.derivesFrom.go E r n x = true →
      RefSafety.derivesFrom.go E r m x = true
  | 0, _, _, _, h => by simp [RefSafety.derivesFrom.go] at h
  | n + 1, m, x, hnm, h => by
    obtain ⟨m, rfl⟩ : ∃ m', m = m' + 1 := ⟨m - 1, by omega⟩
    simp only [RefSafety.derivesFrom.go, Bool.or_eq_true, beq_iff_eq] at h ⊢
    rcases h with h | h
    · exact Or.inl h
    · right
      split at h
      · exact derivesFrom_go_mono (by omega) h
      · cases h
      · cases h

theorem derivesFrom_append {E F : List Event} {d r : Nat}
    (h : RefSafety.derivesFrom E d r = true) : RefSafety.derivesFrom (E ++ F) d r = true := by
  unfold RefSafety.derivesFrom at h ⊢
  exact derivesFrom_go_mono (by simp) (derivesFrom_go_append h)

theorem params?_append {E F : List Event} {f : FrameId} {ps : List Nat}
    (h : RefSafety.params? E f = some ps) : RefSafety.params? (E ++ F) f = some ps := by
  unfold RefSafety.params? at h ⊢
  rw [List.findSome?_append, h]
  rfl

theorem params?_append_none {E F : List Event} {f : FrameId}
    (h : RefSafety.params? E f = none) :
    RefSafety.params? (E ++ F) f = RefSafety.params? F f := by
  unfold RefSafety.params? at h ⊢
  rw [List.findSome?_append, h]
  rfl

theorem params?_of_mem {F : List Event} {f : FrameId} {ps : List Nat}
    (hu : ∀ e ∈ F, ∀ f' ps', e = .entered f' ps' → f' = f → ps' = ps)
    (hm : Event.entered f ps ∈ F) : RefSafety.params? F f = some ps := by
  induction F with
  | nil => simp at hm
  | cons e F ih =>
    unfold RefSafety.params?
    rw [List.findSome?_cons]
    have hu' : ∀ e ∈ F, ∀ f' ps', e = .entered f' ps' → f' = f → ps' = ps :=
      fun e he => hu e (List.mem_cons_of_mem _ he)
    cases e
    case entered g qs =>
      by_cases hg : g = f
      · subst hg
        have := hu _ (List.mem_cons_self ..) g qs rfl rfl
        subst this
        simp
      · have hgf : (g == f) = false := by simpa using hg
        simp only [hgf, Bool.false_eq_true, ↓reduceIte]
        rcases List.mem_cons.1 hm with he | he
        · simp only [Event.entered.injEq] at he
          exact absurd he.1.symm hg
        · exact ih hu' he
    all_goals
      simp only
      rcases List.mem_cons.1 hm with he | he
      · cases he
      · exact ih hu' he

theorem params?_none_of {E : List Event} {n : FrameId}
    (h : ∀ f ps, Event.entered f ps ∈ E → f < n) : RefSafety.params? E n = none := by
  unfold RefSafety.params?
  rw [List.findSome?_eq_none_iff]
  intro e he
  cases e with
  | entered g qs =>
    have := h g qs he
    have hgn : (g == n) = false := by simpa using Nat.ne_of_lt this
    simp [hgn]
  | _ => rfl

/-- **D4d's returns, on more events**: the returns checked before stay
checked, and the new ones are checked on all the events. -/
theorem returnsDerived_append {E F : List Event} (hE : RefSafety.returnsDerived E = true)
    (hF : ∀ e ∈ F, ∀ f rets, e = .frameEnded f rets → ∃ ps,
      RefSafety.params? (E ++ F) f = some ps ∧
        ∀ d ∈ rets, ∃ p ∈ ps, RefSafety.derivesFrom (E ++ F) d p = true) :
    RefSafety.returnsDerived (E ++ F) = true := by
  unfold RefSafety.returnsDerived at hE ⊢
  rw [List.all_eq_true] at hE ⊢
  intro e he
  rcases List.mem_append.1 he with he | he
  · have h1 := hE e he
    cases e with
    | frameEnded f rets =>
      simp only at h1 ⊢
      split at h1
      · next ps hps =>
        rw [params?_append hps]
        simp only [List.all_eq_true, List.any_eq_true] at h1 ⊢
        intro d hd
        obtain ⟨p, hp, hdp⟩ := h1 d hd
        exact ⟨p, hp, derivesFrom_append hdp⟩
      · next hnone =>
        simp only [List.isEmpty_iff] at h1
        subst h1
        split <;> simp
    | _ => rfl
  · cases e with
    | frameEnded f rets =>
      obtain ⟨ps, hps, hall⟩ := hF _ he f rets rfl
      simp only [hps, List.all_eq_true, List.any_eq_true]
      exact hall
    | _ => rfl

/-! ## The executable derivation -/

/-- A reference borrowed at most once has its `borrowed` event as its
creation. -/
theorem creation?_of_mem {E : List Event} (hnd : (borrowedIds E).Nodup) {d : Nat}
    {rd : Reference} {par : Option Nat} (hb : Event.borrowed d rd par ∈ E) :
    RefSafety.creation? E d = some (rd, par) := by
  cases hc : RefSafety.creation? E d with
  | some x =>
    obtain ⟨r', p'⟩ := x
    obtain ⟨rfl, rfl⟩ := borrowed_unique hnd hb (creation?_mem hc)
    rfl
  | none =>
    unfold RefSafety.creation? at hc
    rw [List.findSome?_eq_none_iff] at hc
    have := hc _ hb
    simp [Event.borrowed?] at this

/-- In a sublist, an element of the list missing from the sublist makes the
sublist shorter. -/
theorem Sublist.length_lt_of_mem {α : Type} {l₁ l₂ : List α} (hs : l₁.Sublist l₂) {a : α}
    (ha : a ∈ l₂) (hna : a ∉ l₁) : l₁.length < l₂.length := by
  rcases Nat.lt_or_ge l₁.length l₂.length with h | h
  · exact h
  · have := hs.eq_of_length (Nat.le_antisymm hs.length_le h)
    subst this
    exact absurd ha hna

/-- **The executable derivation of a derivation**, when each reference is
borrowed once and after the one it derives from. -/
theorem derivesFrom_of_derives {E : List Event} (hnd : (borrowedIds E).Nodup)
    (hlt : ∀ d rd t, Event.borrowed d rd (some t) ∈ E → t < d) {r d : Nat}
    (hr : r ∈ borrowedIds E) (h : Derives E r d) : RefSafety.derivesFrom E d r = true := by
  let cnt : Nat → Nat := fun x => ((borrowedIds E).filter fun y => decide (y ≤ x)).length
  have key : ∀ x, Derives E r x → RefSafety.derivesFrom.go E r (cnt x) x = true := by
    intro x hx
    induction hx with
    | refl =>
      have hpos : 0 < cnt r := List.length_pos_of_mem (List.mem_filter.2 ⟨hr, by simp⟩)
      obtain ⟨m, hm⟩ : ∃ m, cnt r = m + 1 := ⟨cnt r - 1, by omega⟩
      rw [hm]
      simp [RefSafety.derivesFrom.go]
    | @step x t rx hb _ ih =>
      have htx := hlt x rx t hb
      have hxB : x ∈ borrowedIds E := mem_borrowedIds.2 ⟨rx, some t, hb⟩
      have hsub : ((borrowedIds E).filter fun y => decide (y ≤ t)).Sublist
          ((borrowedIds E).filter fun y => decide (y ≤ x)) := by
        have h1 := (List.filter_sublist (p := fun y => decide (y ≤ t)) (l := borrowedIds E)).filter
          (fun y => decide (y ≤ x))
        rwa [List.filter_eq_self.2 (fun y hy => by
          have := (List.mem_filter.1 hy).2
          simp only [decide_eq_true_eq] at this ⊢
          omega)] at h1
      have hlen : cnt t < cnt x :=
        Sublist.length_lt_of_mem hsub (List.mem_filter.2 ⟨hxB, by simp⟩)
          (fun hm => by
            have := (List.mem_filter.1 hm).2
            simp only [decide_eq_true_eq] at this
            omega)
      obtain ⟨m, hm⟩ : ∃ m, cnt x = m + 1 := ⟨cnt x - 1, by omega⟩
      rw [hm]
      simp only [RefSafety.derivesFrom.go, Bool.or_eq_true, beq_iff_eq]
      right
      rw [creation?_of_mem hnd hb]
      exact derivesFrom_go_mono (by omega) ih
  unfold RefSafety.derivesFrom
  refine derivesFrom_go_mono ?_ (key d h)
  calc cnt d ≤ (borrowedIds E).length := List.length_filter_le _ _
    _ ≤ E.length := List.length_filterMap_le _ _

/-! ## The next frame identity never decreases -/

section NextFrame

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

theorem nf_next {s₁ : State} {rest₁ : List Frame} {fr₁ : Frame}
    (h : stepOf (next s₁ rest₁ fr₁) = .done (.running s')) : s'.nextFrame = s₁.nextFrame := by
  obtain rfl := Outcome.running.inj (done_next h)
  rfl

theorem nf_finish {s₁ : State} {vals : List Value}
    (h : stepOf (finish s₁ vals) = .done (.running s')) : s'.nextFrame = s₁.nextFrame := by
  unfold finish at h
  split at h
  · simp only [stepOf, pure, Except.pure, StepResult.done.injEq, Outcome.running.injEq] at h
    subst h
    rfl
  · exact (done_wrong h rfl).elim

theorem nf_update {s₁ s₃ : State} {l : Location} {v : Value} (hu : update s₁ l v = .ok s₃) :
    s₃.nextFrame = s₁.nextFrame := by
  obtain ⟨s₂, hw, rfl⟩ := update_ok hu
  simp only [State.write?, Option.bind_eq_bind, Option.bind_eq_some_iff] at hw
  obtain ⟨w, -, w', -, hw⟩ := hw
  simp only [Option.pure_def, Option.some.injEq] at hw
  subst hw
  show (s₁.setRoot l.root w').nextFrame = s₁.nextFrame
  cases l.root <;> rfl

private theorem nf_borrowLoc {mutable : Bool} {i : LocalIndex}
    (h : stepOf (borrowLoc P s fr rest mutable i) = .done (.running s')) :
    s.nextFrame ≤ s'.nextFrame := by
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
    · rw [nf_next h]; exact Nat.le_refl _

private theorem nf_borrowField {mutable : Bool} {hh : FieldHandleIndex} {tys : List RuntimeType}
    (h : stepOf (borrowField P s fr rest info mutable hh tys) = .done (.running s')) :
    s.nextFrame ≤ s'.nextFrame := by
  unfold borrowField at h
  obtain ⟨fh, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨t, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
    dsimp only at h
    obtain ⟨_, -, h⟩ := done_bind h rfl deref_ends
    rw [nf_finish h]; exact Nat.le_refl _
  · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

private theorem nf_borrowVariantField {mutable : Bool} {hh : VariantFieldHandleIndex}
    {tys : List RuntimeType}
    (h : stepOf (borrowVariantField P s fr rest info mutable hh tys) = .done (.running s')) :
    s.nextFrame ≤ s'.nextFrame := by
  unfold borrowVariantField at h
  obtain ⟨vfh, -, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, -, h⟩ := done_bind h rfl (mapM_ends (fun _ =>
      ends_bind (ends_need _ _) fun _ _ => ends_bind (ends_need _ _) fun _ _ => ends_need _ _) _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
    dsimp only at h
    obtain ⟨v, -, h⟩ := done_bind h rfl deref_ends
    split at h
    · split at h
      · rw [nf_finish h]; exact Nat.le_refl _
      · exact (done_fail h rfl).elim
    · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

private theorem nf_borrowGlobal {mutable : Bool} {d : StructDefinitionIndex}
    {tys : List RuntimeType}
    (h : stepOf (borrowGlobal P s fr rest mutable d tys) = .done (.running s')) :
    s.nextFrame ≤ s'.nextFrame := by
  unfold borrowGlobal at h
  obtain ⟨sid, -, h⟩ := done_bind h rfl globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h rfl popAddress_ends
  obtain ⟨_, -, h⟩ := done_bind h rfl unlocked_ends
  split at h
  · rw [nf_next h]; exact Nat.le_refl _
  · exact (done_fail h rfl).elim

private theorem nf_borrowElement {mutable : Bool} {sig : SignatureIndex}
    (h : stepOf (borrowElement s fr rest info mutable sig) = .done (.running s')) :
    s.nextFrame ≤ s'.nextFrame := by
  unfold borrowElement at h
  obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
  obtain ⟨⟨i, fr₁⟩, hpi, h⟩ := done_bind h rfl popIndex_ends
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h rfl popRefTo_ends
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
  split at h
  · rw [nf_finish h]; exact Nat.le_refl _
  · exact (done_fail h rfl).elim

private theorem nf_call {hd : FunctionHandleIndex} {tys : List RuntimeType}
    (h : stepOf (Step.call P s fr rest info hd tys) = .done (.running s')) :
    s.nextFrame ≤ s'.nextFrame := by
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
    split <;> exact Nat.le_succ _

private theorem nf_packClosure {hd : FunctionHandleIndex} {mask : BitVec 64}
    {tys : List RuntimeType}
    (h : stepOf (Step.packClosure P s fr rest info hd mask tys) = .done (.running s')) :
    s.nextFrame ≤ s'.nextFrame := by
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
  obtain ⟨⟨vals, fr₁⟩, -, h⟩ := done_bind h rfl popN_ends
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  · rw [nf_next h]; exact Nat.le_refl _

private theorem nf_callClosure {sig : SignatureIndex}
    (h : stepOf (Step.callClosure P s fr rest info sig) = .done (.running s')) :
    s.nextFrame ≤ s'.nextFrame := by
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
    obtain ⟨⟨c, fr₁⟩, -, h⟩ := done_bind h rfl pop_ends
    dsimp only at h
    split at h
    · split at h
      · exact (done_wrong h rfl).elim
      obtain ⟨⟨provided, fr₂⟩, -, h⟩ := done_bind h rfl popN_ends
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
        exact Nat.le_succ _
    · exact (done_wrong h rfl).elim
  · exact (done_wrong h rfl).elim

private theorem nf_testEnum {hh : StructVariantHandleIndex} {tys : List RuntimeType}
    (h : stepOf (Step.testEnum P s fr rest info hh tys) = .done (.running s')) :
    s.nextFrame ≤ s'.nextFrame := by
    obtain ⟨vh, -, h⟩ := done_bind h rfl (ends_need _ _)
    obtain ⟨⟨sid, sinfo⟩, -, h⟩ := done_bind h rfl localStruct_ends
    dsimp only at h
    split at h
    · obtain ⟨_, -, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨⟨r, fr₁⟩, -, h⟩ := done_bind h rfl popRefTo_ends
      dsimp only at h
      obtain ⟨v, -, h⟩ := done_bind h rfl deref_ends
      split at h
      · rw [nf_finish h]; exact Nat.le_refl _
      · exact (done_wrong h rfl).elim
    · exact (done_wrong h rfl).elim
    · exact (done_wrong h rfl).elim

private theorem nf_moveResourceFrom {d : StructDefinitionIndex} {tys : List RuntimeType}
    (h : stepOf (Step.moveResourceFrom P s fr rest d tys) = .done (.running s')) :
    s.nextFrame ≤ s'.nextFrame := by
    obtain ⟨sid, -, h⟩ := done_bind h rfl globalType_ends
    obtain ⟨⟨a, fr₁⟩, -, h⟩ := done_bind h rfl popAddress_ends
    obtain ⟨_, -, h⟩ := done_bind h rfl unlocked_ends
    split at h
    · rw [nf_finish h]; exact Nat.le_refl _
    · exact (done_fail h rfl).elim

private theorem nf_moveResourceTo {d : StructDefinitionIndex} {tys : List RuntimeType}
    (h : stepOf (Step.moveResourceTo P s fr rest d tys) = .done (.running s')) :
    s.nextFrame ≤ s'.nextFrame := by
    obtain ⟨sid, -, h⟩ := done_bind h rfl globalType_ends
    obtain ⟨⟨v, fr₁⟩, -, h⟩ := done_bind h rfl pop_ends
    obtain ⟨⟨r, fr₂⟩, -, h⟩ := done_bind h rfl popRefTo_ends
    dsimp only at h
    split at h
    · exact (done_wrong h rfl).elim
    split at h
    · exact (done_wrong h rfl).elim
    split at h
    · exact (done_wrong h rfl).elim
    obtain ⟨sv, -, h⟩ := done_bind h rfl deref_ends
    split at h
    · obtain ⟨_, -, h⟩ := done_bind h rfl unlocked_ends
      split at h
      · exact (done_fail h rfl).elim
      · rw [nf_finish h]; exact Nat.le_refl _
    · exact (done_wrong h rfl).elim

private theorem nf_ret
    (h : stepOf (Step.exec P s fr rest info code .ret) = .done (.running s')) :
    s.nextFrame ≤ s'.nextFrame := by
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
    cases hca : fr.countedActive <;> cases hle : fr.lockEntered <;>
      simp only [hca, hle, Bool.false_eq_true, ↓reduceIte] at h <;> subst h <;>
      exact Nat.le_refl _

/-- **The next frame identity never decreases** across a step of the current
frame. -/
theorem exec_nextFrame_le {instr : Bytecode} (hi : code.code[fr.pc]? = some instr)
    (h : stepOf (Step.exec P s fr rest info code instr) = .done (.running s')) :
    s.nextFrame ≤ s'.nextFrame := by
  cases hval : instr.ValueOnly
  · cases instr
    all_goals first
      | (simp [Bytecode.ValueOnly] at hval; done)
      | skip
    case pop =>
      obtain ⟨⟨v, fr₁⟩, -, h⟩ := done_bind h rfl pop_ends
      dsimp only at h
      split at h
      · exact (done_wrong h rfl).elim
      · rw [nf_next h]; exact Nat.le_refl _
    case ret => exact nf_ret h
    case copyLoc =>
      obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
      cases slot with
      | none => exact (done_wrong h rfl).elim
      | some v =>
        dsimp only at h
        split at h
        · rw [nf_next h]; exact Nat.le_refl _
        · exact (done_wrong h rfl).elim
    case moveLoc =>
      obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
      cases slot with
      | none => exact (done_wrong h rfl).elim
      | some v => rw [nf_next h]; exact Nat.le_refl _
    case stLoc =>
      obtain ⟨⟨v, fr₁⟩, -, h⟩ := done_bind h rfl pop_ends
      dsimp only at h
      obtain ⟨tys, -, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨t, -, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨slot, -, h⟩ := done_bind h rfl (ends_need _ _)
      split at h
      · exact (done_wrong h rfl).elim
      · cases slot with
        | none => rw [nf_next h]; exact Nat.le_refl _
        | some old =>
          dsimp only at h
          split at h
          · exact (done_wrong h rfl).elim
          · rw [nf_next h]; exact Nat.le_refl _
    case call => exact nf_call h
    case eq | neq =>
      obtain ⟨fr', -, h'⟩ := done_bind h rfl equality_ends
      rw [nf_next h']; exact Nat.le_refl _
    case testVariant => exact nf_testEnum h
    case readRef =>
      obtain ⟨⟨r, fr₁⟩, -, h⟩ := done_bind h rfl popRef_ends
      dsimp only at h
      obtain ⟨v, -, h⟩ := done_bind h rfl deref_ends
      split at h
      · exact (done_wrong h rfl).elim
      · rw [nf_finish h]; exact Nat.le_refl _
    case writeRef =>
      obtain ⟨⟨r, fr₁⟩, -, h⟩ := done_bind h rfl popRef_ends
      dsimp only at h
      split at h
      · exact (done_wrong h rfl).elim
      obtain ⟨⟨v, fr₂⟩, -, h⟩ := done_bind h rfl pop_ends
      dsimp only at h
      split at h
      · exact (done_wrong h rfl).elim
      obtain ⟨old, -, h⟩ := done_bind h rfl deref_ends
      split at h
      · exact (done_wrong h rfl).elim
      obtain ⟨s₃, hu, h⟩ := done_bind h rfl update_ends
      rw [nf_finish h, nf_update hu]; exact Nat.le_refl _
    case freezeRef =>
      obtain ⟨⟨r, fr₁⟩, -, h⟩ := done_bind h rfl popRef_ends
      dsimp only at h
      split at h
      · exact (done_wrong h rfl).elim
      · rw [nf_next h]; exact Nat.le_refl _
    case mutBorrowLoc | immBorrowLoc => exact nf_borrowLoc h
    case mutBorrowField | immBorrowField => exact nf_borrowField h
    case mutBorrowVariantField | immBorrowVariantField => exact nf_borrowVariantField h
    case vecLen =>
      obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
      obtain ⟨⟨r, fr₁⟩, -, h⟩ := done_bind h rfl popRefTo_ends
      dsimp only at h
      obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
      split at h
      · rw [nf_finish h]; exact Nat.le_refl _
      · exact (done_fail h rfl).elim
    case vecImmBorrow | vecMutBorrow => exact nf_borrowElement h
    case vecPushBack =>
      obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
      obtain ⟨⟨v, fr₁⟩, -, h⟩ := done_bind h rfl pop_ends
      obtain ⟨⟨r, fr₂⟩, -, h⟩ := done_bind h rfl popRefTo_ends
      dsimp only at h
      split at h
      · exact (done_wrong h rfl).elim
      obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
      obtain ⟨s₃, hu, h⟩ := done_bind h rfl update_ends
      rw [nf_finish h, nf_update hu]; exact Nat.le_refl _
    case vecPopBack =>
      obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
      obtain ⟨⟨r, fr₁⟩, -, h⟩ := done_bind h rfl popRefTo_ends
      dsimp only at h
      obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
      split at h
      · obtain ⟨s₃, hu, h⟩ := done_bind h rfl update_ends
        rw [nf_finish h, nf_update hu]; exact Nat.le_refl _
      · exact (done_fail h rfl).elim
    case vecSwap =>
      obtain ⟨t, -, h⟩ := done_bind h rfl elemType_ends
      obtain ⟨⟨j, fr₁⟩, -, h⟩ := done_bind h rfl popIndex_ends
      obtain ⟨⟨i, fr₂⟩, -, h⟩ := done_bind h rfl popIndex_ends
      obtain ⟨⟨r, fr₃⟩, -, h⟩ := done_bind h rfl popRefTo_ends
      dsimp only at h
      obtain ⟨elems, -, h⟩ := done_bind h rfl derefVector_ends
      split at h
      · obtain ⟨s₃, hu, h⟩ := done_bind h rfl update_ends
        rw [nf_finish h, nf_update hu]; exact Nat.le_refl _
      · exact (done_fail h rfl).elim
      · exact (done_fail h rfl).elim
    case moveFrom => exact nf_moveResourceFrom h
    case moveTo => exact nf_moveResourceTo h
    case mutBorrowGlobal | immBorrowGlobal => exact nf_borrowGlobal h
    case callGeneric | testVariantGeneric | moveFromGeneric | moveToGeneric
      | mutBorrowFieldGeneric | immBorrowFieldGeneric | mutBorrowVariantFieldGeneric
      | immBorrowVariantFieldGeneric | mutBorrowGlobalGeneric | immBorrowGlobalGeneric =>
      obtain ⟨x, -, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨tys, -, h⟩ := done_bind h rfl genericArgs_ends
      first
        | exact nf_call h
        | exact nf_testEnum h
        | exact nf_moveResourceFrom h
        | exact nf_moveResourceTo h
        | exact nf_borrowField h
        | exact nf_borrowVariantField h
        | exact nf_borrowGlobal h
    case packGeneric | unpackGeneric | packVariantGeneric | unpackVariantGeneric
      | existsGeneric =>
      exact Nat.le_of_eq (exec_valueGeneric hi rfl h).nextFrame.symm
    case packClosure => exact nf_packClosure h
    case packClosureGeneric =>
      obtain ⟨x, -, h⟩ := done_bind h rfl (ends_need _ _)
      obtain ⟨tys, -, h⟩ := done_bind h rfl genericArgs_ends
      exact nf_packClosure h
    case callClosure => exact nf_callClosure h
    all_goals exact (done_wrong h rfl).elim
  · exact Nat.le_of_eq (exec_value hi hval h).nextFrame.symm

end NextFrame

/-! ## The frames a step enters -/

/-- Events none of which enters a frame. -/
theorem noEntered {evs : List Event} (h : evs.all (fun e => e.entered?.isNone) = true)
    {f : FrameId} {ps : List Nat} (he : Event.entered f ps ∈ evs) : False := by
  have := List.all_eq_true.1 h _ he
  simp [Event.entered?] at this

@[simp] theorem fresh_all_entered (c : Creator) (v : Value) (n : Nat) :
    (IdTree.fresh c v n).2.2.all (fun e => e.entered?.isNone) = true := by
  rw [(fresh_spec c v n).2.2.2]; simp [Event.entered?]

/-- Proves that no event of an explicit list of a step's events enters a
frame. -/
macro "entered_quiet" : tactic =>
  `(tactic| (simp only [List.all_append, List.all_cons, List.all_nil, fresh_all_entered,
      List.all_map, Function.comp_def, Bool.and_true, Bool.true_and]
             first | done | simp [Event.entered?]))

/-- An event of a list none of which enters a frame enters none. -/
theorem noEnteredAt {evs : List Event} (h : evs.all (fun e => e.entered?.isNone) = true)
    {e : Event} (he : e ∈ evs) {p : FrameId × List Nat} (hend : e.entered? = some p) : False := by
  have := List.all_eq_true.1 h e he
  rw [hend] at this
  cases this

/-- **A step enters only a frame of the running state after it.** -/
theorem exec_enteredFrame {P : Program} {m : ModuleId} {instr : Bytecode} {s : State} {fr : Frame}
    {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event} {g : ShadowFrame}
    {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) {e : Event} (he : e ∈ evs)
    {p : FrameId × List Nat} (hend : e.entered? = some p) :
    ∃ s', o = .running s' ∧ ∃ c ∈ s'.frames, c.id = p.1 := by
  cases instr <;>
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq, Prod.mk.injEq, guard, reduceCtorEq] at hex
  all_goals (try simp_all [Event.entered?])
  case ret =>
    split at hex <;>
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_append, List.mem_map, List.mem_singleton] at he
      rcases he with ⟨_, -, rfl⟩ | rfl <;> cases hend
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldConst | ldTrue | ldFalse =>
    obtain ⟨a, -, -, rfl⟩ := hex
    exact (noEnteredAt (by simp) he hend).elim
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32
    | castI64 | castI128 | castI256 | not | negate | add | sub | mul | div | mod | bitOr | bitAnd
    | xor | or | and | lt | gt | le | ge | shl | shr =>
    obtain ⟨-, a, -, -, rfl⟩ := hex
    exact (noEnteredAt (by simp) he hend).elim
  case pack | packVariant | vecPack | packGeneric | packVariantGeneric | packClosure
    | packClosureGeneric =>
    obtain ⟨a, -, -, -, rfl⟩ := hex
    exact (noEnteredAt (by entered_quiet) he hend).elim
  case callClosure =>
    split at hex
    · next s₁ =>
      split at hex
      · next callee caller _ hfr =>
        split at hex
        · split at hex
          · obtain ⟨_, -, hex⟩ := Option.bind_eq_some_iff.1 hex
            simp only [Option.some.injEq, Prod.mk.injEq] at hex
            obtain ⟨-, rfl⟩ := hex
            simp only [List.mem_append, List.mem_map, List.mem_cons,
              List.not_mem_nil, or_false] at he
            rcases he with ⟨_, -, rfl⟩ | rfl | rfl
            · cases hend
            · simp only [Option.some.injEq] at hend
              subst hend
              exact ⟨_, rfl, callee, by rw [hfr]; exact List.mem_cons_self .., rfl⟩
            · cases hend
          · cases hex
        · cases hex
      · cases hex
    · cases hex
  case unpack | unpackVariant | vecUnpack | unpackGeneric | unpackVariantGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨v, -, -, rfl⟩ := hex
      exact (noEnteredAt (by entered_quiet) he hend).elim
    · cases hex
  case eq | neq =>
    obtain ⟨a, b, ⟨-, a₁, -, -, rfl⟩, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact (noEnteredAt (by entered_quiet) he hend).elim
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact (noEnteredAt (by simp) he hend).elim
  case copyLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact (noEnteredAt (by simp only [List.all_cons]; split <;> entered_quiet) he hend).elim
    · cases hex
  case moveLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact (noEnteredAt (by simp only [List.all_cons, List.all_append]; split <;> entered_quiet)
        he hend).elim
    · cases hex
  case stLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact (noEnteredAt (by simp only [List.all_cons]; split <;> entered_quiet) he hend).elim
    · cases hex
  case call | callGeneric =>
    split at hex
    · next s₁ =>
      split at hex
      · next callee caller _ hfr =>
        obtain ⟨_, -, hex⟩ := Option.bind_eq_some_iff.1 hex
        simp only [Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, rfl⟩ := hex
        simp only [List.mem_append, List.mem_map, List.mem_singleton] at he
        rcases he with ⟨_, -, rfl⟩ | rfl
        · cases hend
        · simp only [Option.some.injEq] at hend
          subst hend
          exact ⟨_, rfl, callee, by rw [hfr]; exact List.mem_cons_self .., rfl⟩
      · cases hex
    · cases hex
  case readRef =>
    obtain ⟨a, b, -, a₁, -, -, rfl⟩ := hex
    exact (noEnteredAt (by entered_quiet) he hend).elim
  case writeRef =>
    obtain ⟨a, b, -, a₁, -, a₂, -, -, rfl⟩ := hex
    exact (noEnteredAt (by entered_quiet) he hend).elim
  case freezeRef =>
    obtain ⟨a, b, -, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact (noEnteredAt (by entered_quiet) he hend).elim
    · cases hm
  case testVariant | testVariantGeneric =>
    obtain ⟨a, b, -, hm⟩ := hex
    split at hm
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨x, hx, -, rfl⟩ := hm
      obtain ⟨-, -, v, -, rfl⟩ := hx
      exact (noEnteredAt (by entered_quiet) he hend).elim
    · cases hm
  case mutBorrowField | immBorrowField | mutBorrowVariantField | immBorrowVariantField
    | mutBorrowFieldGeneric | immBorrowFieldGeneric | mutBorrowVariantFieldGeneric
    | immBorrowVariantFieldGeneric =>
    obtain ⟨a, b, -, hm⟩ := hex
    split at hm
    · obtain ⟨_, -, hm⟩ := Option.bind_eq_some_iff.1 hm
      split at hm
      · simp only [Option.some.injEq, Prod.mk.injEq] at hm
        obtain ⟨-, rfl⟩ := hm
        exact (noEnteredAt (by entered_quiet) he hend).elim
      · cases hm
    · cases hm
  case vecLen =>
    obtain ⟨a, b, -, a₁, b₁, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    exact (noEnteredAt (by entered_quiet) he hend).elim
  case vecImmBorrow | vecMutBorrow =>
    obtain ⟨a, b, -, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact (noEnteredAt (by entered_quiet) he hend).elim
    · cases hm
  case vecPushBack =>
    obtain ⟨a, -, a₁, b, -, a₂, -, -, rfl⟩ := hex
    exact (noEnteredAt (by entered_quiet) he hend).elim
  case vecPopBack =>
    obtain ⟨a, b, -, a₁, -, a₂, -, a₃, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact (noEnteredAt (by entered_quiet) he hend).elim
    · cases hm
  case vecSwap =>
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨x, -, y, -, -, rfl⟩ := hex
        exact (noEnteredAt (by entered_quiet) he hend).elim
      · cases hex
    · cases hex
  case exists_ =>
    obtain ⟨a, -, a₁, b, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    exact (noEnteredAt (by entered_quiet) he hend).elim
  case existsGeneric =>
    obtain ⟨_, _, _, _, _, _, _, _, a, b, ⟨_, a', _, _, rfl⟩, _, rfl⟩ := hex
    exact (noEnteredAt (by entered_quiet) he hend).elim
  case moveFrom | moveFromGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff] at hex
      obtain ⟨v, -, hex⟩ := hex
      split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨k, -, t, -, -, rfl⟩ := hex
        exact (noEnteredAt (by entered_quiet) he hend).elim
      · cases hex
    · cases hex
  case moveTo | moveToGeneric =>
    obtain ⟨a, -, a₁, -, hm⟩ := hex
    split at hm
    · simp only [Option.bind_eq_some_iff] at hm
      obtain ⟨x, -, hm⟩ := hm
      split at hm
      · simp only [Option.some.injEq, Prod.mk.injEq] at hm
        obtain ⟨-, rfl⟩ := hm
        exact (noEnteredAt (by entered_quiet) he hend).elim
      · cases hm
    · cases hm
  case mutBorrowLoc | immBorrowLoc =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact (noEnteredAt (by entered_quiet) he hend).elim
    · cases hex
  case mutBorrowGlobal | immBorrowGlobal | mutBorrowGlobalGeneric | immBorrowGlobalGeneric =>
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, -, -, rfl⟩ := hex
        exact (noEnteredAt (by entered_quiet) he hend).elim
      · cases hex
    · cases hex

/-- A native call enters no frame. -/
theorem nativeEvents_entered {ν : NativeFlows} {f : FunctionRef} {args vals : List Value}
    {g : ShadowFrame} {m : ModuleId} {fresh : Nat} {f' : FrameId} {ps : List Nat} :
    Event.entered f' ps ∉ nativeEvents ν f args vals g m fresh := by
  intro hb
  simp only [nativeEvents, List.mem_append] at hb
  rcases hb with hb | hb
  · obtain ⟨_, _, _, _, he, -⟩ := consumedFrom_mem _ hb
    cases he
  · obtain ⟨_, _, _, he⟩ := (nativeResults_spec ν f args vals _ m fresh).2.2 _ hb
    cases he

/-! ## Along a trace -/

section Trace

variable {P : Program} {N : NativeRel} {ν : NativeFlows}

/-- **The next frame identity never decreases** across a traced step. -/
theorem traced_nextFrame_le {s s' : State} {h : Shadow} {o : Outcome} {post : ShadowPost}
    {evs : List Event} (hS : TracedStep P N ν s h o post evs) (ho : o = .running s') :
    s.nextFrame ≤ s'.nextFrame := by
  cases hS with
  | core hstep hc =>
    subst ho
    obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, -⟩ := coreStep_exec hc
    have hst' : stepOf (Step.exec P s fr rest info code instr) = .done (.running s') := by
      unfold step at hstep
      rw [hsf] at hstep
      simp only [hinfo, hcode, hinstr] at hstep
      exact hstep
    exact exec_nextFrame_le hinstr hst'
  | native hstep _ hn =>
    obtain ⟨fr, rest, n, hsf, -, -, rfl, -⟩ := step_callNative hstep
    rw [resume_nextFrame ho]
    exact Nat.le_refl _

/-- **A traced step enters only a frame of the state after it.** -/
theorem traced_entered {s s' : State} {h : Shadow} {o : Outcome} {post : ShadowPost}
    {evs : List Event} (hS : TracedStep P N ν s h o post evs) (ho : o = .running s')
    {f : FrameId} {ps : List Nat} (he : Event.entered f ps ∈ evs) : ∃ c ∈ s'.frames, c.id = f := by
  cases hS with
  | core hstep hc =>
    subst ho
    obtain ⟨fr, rest, info, code, instr, -, -, -, -, hex⟩ := coreStep_exec hc
    obtain ⟨hframes, hst, hhe, fresh⟩ := h
    cases hframes with
    | nil => simp [Shadow.exec] at hex
    | cons g hs =>
      obtain ⟨s'', hs'', c, hc, hcf⟩ := exec_enteredFrame rfl hex he (p := (f, ps)) rfl
      rw [Outcome.running.injEq] at hs''
      subst hs''
      exact ⟨c, hc, hcf⟩
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      have := (nativeStep_quiet hn he).1
      simp [Event.entered?] at this
    obtain ⟨rfl, rfl, rfl, g, hs, info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at he
    exact absurd he nativeEvents_entered

/-- **A traced step borrows identities from the fresh counter on.** -/
theorem traced_borrowed_fresh {s : State} {h : Shadow} {E : List Event} {o : Outcome}
    {post : ShadowPost} {evs : List Event} (hI : RefTraceInv P s h E)
    (hS : TracedStep P N ν s h o post evs) {s' : State} (ho : o = .running s') {d : Nat}
    {r : Reference} {par : Option Nat} (hb : Event.borrowed d r par ∈ evs) : h.fresh ≤ d := by
  cases hS with
  | core hstep hc =>
    subst ho
    obtain ⟨post', evs'', hc', hG⟩ := coreStep_annot (σ := false) (π := {}) (A := [])
      (auth := []) hI.pre hstep (by simp [Outcome.continues]) (fun vs st ho => by cases ho)
    rw [hc, Option.some.injEq, Prod.mk.injEq] at hc'
    obtain ⟨rfl, rfl⟩ := hc'
    obtain ⟨keep, dropped, copies, -, -, -, -, hiss, -, -⟩ := hG.effect
    exact (hiss (d, (Value.ref r).key) (List.mem_filterMap.2 ⟨_, hb, rfl⟩)).1
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      obtain ⟨-, -, -, hd, -⟩ := nativeStep_borrowed hn hb
      exact hd
    obtain ⟨rfl, rfl, rfl, g, hs, info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    have hm : d ∈ borrowedIds evs := mem_borrowedIds.2 ⟨r, par, hb⟩
    rw [hevs, nativeEvents_borrowedIds] at hm
    cases hm

/-- **A reference is borrowed after the one it derives from**: its identity
is the larger. -/
theorem trace_parent_lt {s₀ s : State} {h : Shadow} {E : List Event}
    (h0 : AdmissibleInitial P s₀) (hT : Trace P N ν s₀ s h E) :
    ∀ d rd t, Event.borrowed d rd (some t) ∈ E → t < d := by
  induction hT with
  | start => exact fun d rd t hb => absurd hb (derived_not_initial h0)
  | step hT hS ih =>
    intro d rd t hb
    rcases List.mem_append.1 hb with hb | hb
    · exact ih d rd t hb
    · have hI := refTraceInv_trace h0 hT
      obtain ⟨rt, hu, -, -⟩ := traced_borrowsInside hS d rd t hb
      obtain ⟨par, hbt⟩ := hI.refs t rt (traced_used hI.pre hS hu)
      exact Nat.lt_of_lt_of_le (hI.below t (mem_borrowedIds.2 ⟨rt, par, hbt⟩))
        (traced_borrowed_fresh hI hS rfl hb)

end Trace

/-! ## The initial events -/

/-- The initial events: the creations of the initial values, then the entry
frame's reference parameters and its entry (`initRefs`). -/
theorem initial_events_entry {P : Program} {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    ∃ fr g, s₀.frames = [fr] ∧ (Shadow.initial s₀).1.frames = [g] ∧
      (∀ e ∈ (Shadow.initial s₀).2, (∃ i w c, e = .created i w c) ∨ e ∈ initRefs g fr) ∧
      ∀ e ∈ initRefs g fr, e ∈ (Shadow.initial s₀).2 := by
  obtain ⟨f, args, tys, storage, held, -, -, -, hinit⟩ := h0
  obtain ⟨fr, localTys, hfr, hst, -⟩ := initialState_shape hinit
  refine ⟨fr, ⟨(initL fr).1.toArray, (initK fr).1⟩, hfr, by rw [initial_eq hfr], ?_, ?_⟩
  · intro e he
    rw [initial_eq hfr] at he
    simp only [List.append_nil, List.mem_append] at he
    rcases he with (((he | he) | he) | he) | he
    · obtain ⟨ls, evs₁, n₁, hf, -, -, -, -, -, -, hev⟩ := locals_fold fr.locals.toList [] 0 []
      have h1 : (initL fr).2.2 = evs₁ := by simp [initL, hf]
      rw [h1] at he
      obtain ⟨i, w, rfl, -⟩ := hev _ he
      exact Or.inl ⟨i, w, _, rfl⟩
    · simp [initK, hst] at he
    · obtain ⟨sts, e₂, n₂, hfS, -, -, -, -, -, hevS⟩ :=
        fresh_fold .storage (s₀.storage.map (·.value)) [] (initK fr).2.1 []
      have h1 : (initS s₀ fr).2.2 = e₂ := by simp [initS, hfS]
      rw [h1] at he
      obtain ⟨i, w, rfl, -⟩ := hevS _ he
      exact Or.inl ⟨i, w, _, rfl⟩
    · obtain ⟨hts, e₃, n₃, hfH, -, -, -, -, -, hevH⟩ :=
        fresh_fold .transaction s₀.held [] (initS s₀ fr).2.1 []
      have h1 : (initH s₀ fr).2.2 = e₃ := by simp [initH, hfH]
      rw [h1] at he
      obtain ⟨i, w, rfl, -⟩ := hevH _ he
      exact Or.inl ⟨i, w, _, rfl⟩
    · exact Or.inr he
  · intro e he
    rw [initial_eq hfr]
    simp [he]

/-- **The entry of the initial events**: they enter the entry frame once,
with the identities of the references in it as its reference parameters,
which they borrow, and end no frame. -/
theorem initial_entry {P : Program} {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    ∃ fr₀ g₀ pids₀, s₀.frames = [fr₀] ∧ (Shadow.initial s₀).1.frames = [g₀] ∧
      RefSafety.params? (Shadow.initial s₀).2 fr₀.id = some pids₀ ∧
      (∀ d r, (d, Value.ref r) ∈ g₀.occurrences fr₀ → d ∈ pids₀) ∧
      (∀ p ∈ pids₀, p ∈ borrowedIds (Shadow.initial s₀).2) ∧
      (∀ f ps, Event.entered f ps ∈ (Shadow.initial s₀).2 → f = fr₀.id ∧ ps = pids₀) ∧
      ∀ f rets, Event.frameEnded f rets ∉ (Shadow.initial s₀).2 := by
  have hI := refTraceInv_initial h0
  obtain ⟨fr, g, hfr, hg, hcls, hrefs⟩ := initial_events_entry h0
  have hent : ∀ f ps, Event.entered f ps ∈ (Shadow.initial s₀).2 →
      f = fr.id ∧ ps = (initParams g fr).map (·.1) := by
    intro f ps he
    rcases hcls _ he with ⟨_, _, _, h⟩ | he
    · cases h
    · simp only [initRefs, List.mem_append, List.mem_map, List.mem_singleton] at he
      rcases he with ⟨_, -, h⟩ | h
      · cases h
      · simp only [Event.entered.injEq] at h
        exact h
  refine ⟨fr, g, (initParams g fr).map (·.1), hfr, hg, ?_, ?_, ?_, hent, ?_⟩
  · refine params?_of_mem (fun e he f' ps' hee hf' => ?_) (hrefs _ (by simp [initRefs]))
    subst hee
    exact (hent f' ps' he).2
  · intro d r hm
    have hm' : (d, Value.ref r) ∈ framesOcc (Shadow.initial s₀).1.frames s₀.frames := by
      rw [hg, hfr]
      simp [framesOcc, hm]
    obtain ⟨par, hb⟩ := hI.refs d r hm'
    rcases hcls _ hb with ⟨_, _, _, h⟩ | hb
    · cases h
    · simp only [initRefs, List.mem_append, List.mem_map, List.mem_singleton] at hb
      rcases hb with ⟨⟨i, r'⟩, hir, h⟩ | h
      · simp only [Event.borrowed.injEq] at h
        obtain ⟨rfl, -, -⟩ := h
        exact List.mem_map.2 ⟨(i, r'), hir, rfl⟩
      · cases h
  · intro p hp
    obtain ⟨⟨i, r⟩, hir, rfl⟩ := List.mem_map.1 hp
    refine mem_borrowedIds.2 ⟨r, none, hrefs _ ?_⟩
    simp only [initRefs, List.mem_append, List.mem_map]
    exact Or.inl ⟨(i, r), hir, rfl⟩
  · intro f rets he
    rcases hcls _ he with ⟨_, _, _, h⟩ | he
    · cases h
    · simp only [initRefs, List.mem_append, List.mem_map, List.mem_singleton] at he
      rcases he with ⟨_, -, h⟩ | h <;> cases h

/-- **Every frame entered along a trace has an identity below the next
one.** -/
theorem trace_entered_lt {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s : State}
    {h : Shadow} {E : List Event} (h0 : AdmissibleInitial P s₀) (hT : Trace P N ν s₀ s h E) :
    ∀ f ps, Event.entered f ps ∈ E → f < s.nextFrame := by
  induction hT with
  | start =>
    obtain ⟨fr, g, pids, hfr, -, -, -, -, hent, -⟩ := initial_entry h0
    intro f ps he
    obtain ⟨rfl, -⟩ := hent f ps he
    exact (refTraceInv_initial h0).pre.inv.framesBelow fr (by rw [hfr]; exact List.mem_singleton_self _)
  | step hT hS ih =>
    intro f ps he
    rcases List.mem_append.1 he with he | he
    · exact Nat.lt_of_lt_of_le (ih f ps he) (traced_nextFrame_le hS rfl)
    · obtain ⟨c, hc, rfl⟩ := traced_entered hS rfl he
      exact (refTraceInv_trace h0 (.step hT hS)).pre.inv.framesBelow c hc

end MoveBytecode.Proofs
