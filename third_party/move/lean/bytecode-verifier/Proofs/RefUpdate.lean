-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefRefs

/-!
# Writing in place, wherever the location is

UNTRUSTED (checked by the kernel). A write through a reference changes the
value at the location's root: a local of the current frame, a local of a
suspended frame (through a reference parameter), a value the transaction
holds, or a global resource. Everything not strictly inside the location still resolves, every
root keeps its type, and every frame's locals still conform; a suspended
frame keeps its references and resumes as before.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

variable {P : Program}

/-- A frame after a write at `l`, whose root now holds `w'`. -/
def updFrame (l : Location) (w' : Value) (g : Frame) : Frame :=
  match l.root with
  | .local_ f i => if g.id == f then { g with locals := g.locals.setIfInBounds i (some w') } else g
  | _ => g

/-- The held values after a write at `l`, whose root now holds `w'`. -/
def updHeld (l : Location) (w' : Value) (held : List Value) : List Value :=
  match l.root with
  | .held k => held.set k w'
  | _ => held

/-- Global storage after a write at `l`, whose root now holds `w'`. -/
def updStorage (l : Location) (w' : Value) (st : GlobalStorage) : GlobalStorage :=
  match l.root with
  | .global a t => st.update a t w'
  | _ => st

theorem updFrame_eq (l : Location) (w' : Value) (g : Frame) :
    (updFrame l w' g).id = g.id ∧ (updFrame l w' g).function = g.function ∧
      (updFrame l w' g).pc = g.pc ∧ (updFrame l w' g).stack = g.stack := by
  unfold updFrame
  split
  · split <;> exact ⟨rfl, rfl, rfl, rfl⟩
  · exact ⟨rfl, rfl, rfl, rfl⟩

theorem updFrame_skel (l : Location) (w' : Value) (g : Frame) :
    (updFrame l w' g).skel = g.skel := by
  unfold updFrame
  split
  · split <;> rfl
  · rfl

theorem map_updFrame_skel (l : Location) (w' : Value) (frs : List Frame) :
    (frs.map (updFrame l w')).map Frame.skel = frs.map Frame.skel := by
  rw [List.map_map]
  exact List.map_congr_left fun g _ => updFrame_skel l w' g

/-- A frame other than the one written is the same. -/
theorem updFrame_ne {l : Location} {w' : Value} {g : Frame}
    (h : ∀ i, l.root ≠ .local_ g.id i) : updFrame l w' g = g := by
  unfold updFrame
  split
  · next f i hρ =>
    have : (g.id == f) = false := by
      simp only [beq_eq_false_iff_ne]; intro e; exact h i (by rw [hρ, e])
    simp [this]
  · rfl

/-- The locals of the frame written. -/
theorem updFrame_locals {l : Location} {w' : Value} {g : Frame} {i : Nat}
    (h : l.root = .local_ g.id i) :
    (updFrame l w' g).locals = g.locals.setIfInBounds i (some w') := by
  unfold updFrame
  split
  · next f i' hρ =>
    rw [h, Root.local_.injEq] at hρ
    obtain ⟨rfl, rfl⟩ := hρ
    simp
  · next hne => exact absurd h (hne _ _)

/-- The state after a write at `l` through the current frame `fr₁`, which
then pushes `vals` and advances. -/
def writeState (s : State) (fr₁ : Frame) (rest : List Frame) (l : Location) (w' : Value)
    (vals : List Value) : State :=
  { s with
    frames := Step.advance (Step.pushAll (updFrame l w' fr₁) vals) :: rest.map (updFrame l w')
    held := updHeld l w' s.held
    storage := updStorage l w' s.storage
    invalidations := .mutated l :: s.invalidations }

/-! ## Global storage after a write -/

theorem GlobalStorage.get?_update_ne {st : GlobalStorage} {a a' : Address} {t t' : StructId}
    {v : Value} (h : ¬ (a = a' ∧ t = t')) : (st.update a t v).get? a' t' = st.get? a' t' := by
  unfold GlobalStorage.get? GlobalStorage.update
  rw [List.find?_map]
  have hp : ((fun r : Resource => r.address == a' && r.type == t') ∘
      fun r : Resource => if r.address == a && r.type == t then { r with value := v } else r) =
      fun r : Resource => r.address == a' && r.type == t' := by
    funext r; simp only [Function.comp]; split <;> rfl
  rw [hp]
  cases hf : st.find? (fun r : Resource => r.address == a' && r.type == t') with
  | none => rfl
  | some r =>
    have hr := List.find?_some hf
    simp only [Bool.and_eq_true, beq_iff_eq] at hr
    have hne : ¬ (r.address = a ∧ r.type = t) := fun ⟨ha, ht⟩ =>
      h ⟨by rw [← ha, hr.1], by rw [← ht, hr.2]⟩
    simp [hne]

theorem GlobalStorage.get?_update_self {st : GlobalStorage} {a : Address} {t : StructId}
    {v w : Value} (h : st.get? a t = some w) : (st.update a t v).get? a t = some v := by
  unfold GlobalStorage.get? GlobalStorage.update at *
  rw [List.find?_map]
  have hp : ((fun r : Resource => r.address == a && r.type == t) ∘
      fun r : Resource => if r.address == a && r.type == t then { r with value := v } else r) =
      fun r : Resource => r.address == a && r.type == t := by
    funext r; simp only [Function.comp]; split <;> rfl
  rw [hp]
  cases hf : st.find? (fun r : Resource => r.address == a && r.type == t) with
  | none => rw [hf] at h; cases h
  | some r =>
    have hr := List.find?_some hf
    simp only [Bool.and_eq_true, beq_iff_eq] at hr
    simp [hr]

/-- **A write** replaces the value at the root of `l`. -/
theorem update_eq {s : State} {l : Location} {v w w' : Value} (hroot : s.root? l.root = some w)
    (hset : w.set? l.path v = some w') :
    Step.update s l v = .ok { s with
      frames := s.frames.map (updFrame l w')
      held := updHeld l w' s.held
      storage := updStorage l w' s.storage
      invalidations := .mutated l :: s.invalidations } := by
  have hwr : s.write? l v = some (s.setRoot l.root w') := by
    simp [State.write?, hroot, hset]
  simp only [Step.update, hwr, Step.need, bind, Except.bind, pure, Except.pure, State.invalidate]
  have hid : ∀ (h : ∀ f i, l.root ≠ .local_ f i), s.frames.map (updFrame l w') = s.frames := by
    intro h
    conv => rhs; rw [← List.map_id s.frames]
    exact List.map_congr_left fun g _ => updFrame_ne fun i => h g.id i
  cases hρ : l.root with
  | local_ f i => simp [State.setRoot, updFrame, updHeld, updStorage, hρ]
  | held k =>
    rw [hid (fun f i e => by rw [hρ] at e; cases e)]
    simp [State.setRoot, updHeld, updStorage, hρ]
  | global a t =>
    rw [hid (fun f i e => by rw [hρ] at e; cases e)]
    simp [State.setRoot, updHeld, updStorage, hρ]

/-- The frames after a write, found by identity. -/
theorem frame?_upd {frs : List Frame} {l : Location} {w' : Value} (f : FrameId) :
    (frs.map (updFrame l w')).find? (·.id == f) = (frs.find? (·.id == f)).map (updFrame l w') := by
  have : ((fun x : Frame => x.id == f) ∘ updFrame l w') = (fun x : Frame => x.id == f) :=
    funext fun g => by simp [Function.comp, (updFrame_eq l w' g).1]
  rw [List.find?_map, this]

/-- Two states whose frames, found by identity, have the same locals and the
same function as the frames after a write. -/
structure AfterWrite (P : Program) (s s₂ : State) (l : Location) (w' : Value) : Prop where
  locals : ∀ f, (s₂.frame? f).map Frame.locals = ((s.frame? f).map (updFrame l w')).map Frame.locals
  functions : ∀ f, (s₂.frame? f).map Frame.function = (s.frame? f).map Frame.function
  held : s₂.held = updHeld l w' s.held
  storage : s₂.storage = updStorage l w' s.storage
  invalidations : s₂.invalidations = .mutated l :: s.invalidations

/-- **The roots after a write**: only the root written changes. -/
theorem root?_update {s s₂ : State} {l : Location} {w w' : Value} (hw : AfterWrite P s s₂ l w')
    (hroot : s.root? l.root = some w) (ρ : Root) :
    s₂.root? ρ = if ρ = l.root then some w' else s.root? ρ := by
  cases ρ with
  | local_ f j =>
    have hloc := hw.locals f
    simp only [State.root?]
    cases hf : s.frame? f with
    | none =>
      cases hf₂ : s₂.frame? f with
      | none =>
        split
        · next he => rw [← he] at hroot; simp [State.root?, hf] at hroot
        · rfl
      | some g₂ => rw [hf, hf₂] at hloc; simp at hloc
    | some g =>
      have hgid := (frame?_mem hf).2
      cases hf₂ : s₂.frame? f with
      | none => rw [hf, hf₂] at hloc; simp at hloc
      | some g₂ =>
        rw [hf, hf₂] at hloc
        simp only [Option.map_some, Option.some.injEq] at hloc
        simp only [Option.bind_some, bind]
        rw [hloc]
        by_cases he : Root.local_ f j = l.root
        · rw [if_pos he]
          rw [← he] at hroot
          simp only [State.root?, hf, Option.bind_some, bind] at hroot
          have hlj : j < g.locals.size := by
            cases hgl : g.locals[j]? with
            | none => rw [hgl] at hroot; simp at hroot
            | some o => exact (Array.getElem?_eq_some_iff.1 hgl).1
          rw [updFrame_locals (by rw [← he, hgid])]
          simp [hlj]
        · rw [if_neg he]
          by_cases hl : ∃ i, l.root = .local_ g.id i
          · obtain ⟨i, hi⟩ := hl
            rw [updFrame_locals hi]
            have hji : j ≠ i := fun e => he (by rw [hi, hgid, e])
            rw [Array.getElem?_setIfInBounds_ne hji]
          · rw [updFrame_ne (fun i hi => hl ⟨i, hi⟩)]
  | held k =>
    simp only [State.root?, hw.held]
    by_cases he : Root.held k = l.root
    · rw [if_pos he]
      rw [← he] at hroot
      simp only [State.root?] at hroot
      have hk : k < s.held.length := (List.getElem?_eq_some_iff.1 hroot).1
      simp [updHeld, ← he, hk]
    · rw [if_neg he]
      cases hρ : l.root with
      | held k' =>
        have : k ≠ k' := fun e => he (by rw [hρ, e])
        simp [updHeld, hρ, List.getElem?_set_ne (Ne.symm this)]
      | local_ f i => simp [updHeld, hρ]
      | global a t => simp [updHeld, hρ]
  | global a t =>
    simp only [State.root?, hw.storage]
    by_cases he : Root.global a t = l.root
    · rw [if_pos he]
      rw [← he] at hroot
      simp only [State.root?] at hroot
      simp only [updStorage, ← he]
      exact GlobalStorage.get?_update_self hroot
    · rw [if_neg he]
      cases hρ : l.root with
      | global a' t' =>
        simp only [updStorage, hρ]
        exact GlobalStorage.get?_update_ne fun ⟨h1, h2⟩ => he (by rw [hρ, h1, h2])
      | local_ f i => simp [updStorage, hρ]
      | held k => simp [updStorage, hρ]

/-- **Reads after a write**: every location not strictly inside the location
written still resolves. -/
theorem read_update {s s₂ : State} {l : Location} {v w w' : Value} (hw : AfterWrite P s s₂ l w')
    (hroot : s.root? l.root = some w) (hset : w.set? l.path v = some w')
    {l' : Location} (hnot : ¬ l'.StrictlyInside l)
    (hr : (s.read? l').isSome = true) : (s₂.read? l').isSome = true := by
  simp only [State.read?] at hr ⊢
  rw [root?_update hw hroot]
  split
  · next he =>
    rw [he, hroot] at hr
    simp only [Option.bind_some, bind] at hr ⊢
    obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1 hr
    have hnot' : ¬ (l.path <+: l'.path ∧ l.path ≠ l'.path) := fun ⟨h1, h2⟩ =>
      hnot ⟨⟨he, h1⟩, fun e => h2 e.symm⟩
    obtain ⟨u', hu'⟩ := get_after_set _ _ hset hnot' hu
    simp [hu']
  · exact hr

/-- **Root types after a write** of a value of the location's type: every root
keeps its type, except the root written, whose type is refined (it changes
only when the whole of a held closure is replaced). -/
theorem rootType_update {s s₂ : State} {l : Location} {v w w' : Value} {T t : RuntimeType}
    (hw : AfterWrite P s s₂ l w') (hroot : s.root? l.root = some w)
    (hset : w.set? l.path v = some w') (hT : RootType P s l.root = some T)
    (hpath : pathType P T l.path = some t) (hv : v.conforms P t = true) (ρ : Root) :
    ∀ T₀, RootType P s ρ = some T₀ → ∃ T₂, RootType P s₂ ρ = some T₂ ∧ RootRefines ρ T₂ T₀ := by
  have key : RootType P s₂ ρ = RootType P s ρ ∨
      (ρ = l.root ∧ ∃ T₂, RootType P s₂ ρ = some T₂ ∧ RootRefines ρ T₂ T) := by
    cases ρ with
    | local_ f j =>
      left
      have hfun := hw.functions f
      simp only [RootType]
      cases hf : s.frame? f with
      | none =>
        cases hf₂ : s₂.frame? f with
        | none => rfl
        | some g => rw [hf, hf₂] at hfun; simp at hfun
      | some g =>
        cases hf₂ : s₂.frame? f with
        | none => rw [hf, hf₂] at hfun; simp at hfun
        | some g₂ =>
          rw [hf, hf₂] at hfun
          simp only [Option.map_some, Option.some.injEq] at hfun
          simp [hfun]
    | held k =>
      by_cases he : Root.held k = l.root
      · right
        refine ⟨he, ?_⟩
        rw [← he] at hroot hT
        simp only [State.root?] at hroot
        simp only [RootType, hroot, Option.bind_some] at hT
        split at hT
        · next hwf =>
          simp only [Option.some.injEq] at hT
          have hwT : w.conforms P T = true := conforms_of (by rw [hT]; exact assignable_refl _) hwf
          have hw'T := set_conforms hwT hpath hv hset
          have hk : k < s.held.length := (List.getElem?_eq_some_iff.1 hroot).1
          refine ⟨w'.type, ?_, Or.inr ⟨fun _ _ h => Root.noConfusion h, conforms_assignable hw'T⟩⟩
          simp only [RootType, hw.held, updHeld, ← he, List.getElem?_set_self hk, Option.bind_some,
            (conforms_iff.1 hw'T).2, ↓reduceIte]
        · cases hT
      · left
        simp only [RootType, hw.held]
        cases hρ : l.root with
        | held k' =>
          have : k ≠ k' := fun e => he (by rw [hρ, e])
          simp [updHeld, hρ, List.getElem?_set_ne (Ne.symm this)]
        | local_ f i => simp [updHeld, hρ]
        | global a t => simp [updHeld, hρ]
    | global a t =>
      by_cases he : Root.global a t = l.root
      · right
        refine ⟨he, ?_⟩
        rw [← he] at hroot hT
        simp only [State.root?] at hroot
        simp only [RootType, hroot, Option.bind_some] at hT
        split at hT
        · next hwf =>
          simp only [Option.some.injEq] at hT
          have hwT : w.conforms P T = true := conforms_of (by rw [hT]; exact assignable_refl _) hwf
          have hw'T := set_conforms hwT hpath hv hset
          refine ⟨w'.type, ?_, Or.inr ⟨fun _ _ h => Root.noConfusion h, conforms_assignable hw'T⟩⟩
          simp only [RootType, hw.storage, updStorage, ← he, GlobalStorage.get?_update_self hroot,
            Option.bind_some, (conforms_iff.1 hw'T).2, ↓reduceIte]
        · cases hT
      · left
        simp only [RootType, hw.storage]
        cases hρ : l.root with
        | global a' t' =>
          simp only [updStorage, hρ]
          rw [GlobalStorage.get?_update_ne fun ⟨h1, h2⟩ => he (by rw [hρ, h1, h2])]
        | local_ f i => simp [updStorage, hρ]
        | held k => simp [updStorage, hρ]
  intro T₀ hT₀
  rcases key with h | ⟨hρ, T₂, h2, hr⟩
  · exact ⟨T₀, by rw [h, hT₀], Or.inl rfl⟩
  · subst hρ
    rw [hT, Option.some.injEq] at hT₀
    subst hT₀
    exact ⟨T₂, h2, hr⟩

/-- **Root types after a write**, but the root written's. -/
theorem rootType_update_other {s s₂ : State} {l : Location} {w' : Value}
    (hw : AfterWrite P s s₂ l w') {ρ : Root} (hρ : ρ ≠ l.root) :
    RootType P s₂ ρ = RootType P s ρ := by
  cases ρ with
  | local_ f j =>
    have hfun := hw.functions f
    simp only [RootType]
    cases hf : s.frame? f with
    | none =>
      cases hf₂ : s₂.frame? f with
      | none => rfl
      | some g => rw [hf, hf₂] at hfun; simp at hfun
    | some g =>
      cases hf₂ : s₂.frame? f with
      | none => rw [hf, hf₂] at hfun; simp at hfun
      | some g₂ =>
        rw [hf, hf₂] at hfun
        simp only [Option.map_some, Option.some.injEq] at hfun
        simp [hfun]
  | held k =>
    simp only [RootType, hw.held]
    cases hρ' : l.root with
    | held k' =>
      have : k ≠ k' := fun e => hρ (by rw [hρ', e])
      simp [updHeld, hρ', List.getElem?_set_ne (Ne.symm this)]
    | local_ f i => simp [updHeld, hρ']
    | global a t => simp [updHeld, hρ']
  | global a t =>
    simp only [RootType, hw.storage]
    cases hρ' : l.root with
    | global a' t' =>
      simp only [updStorage, hρ']
      rw [GlobalStorage.get?_update_ne fun ⟨h1, h2⟩ => hρ (by rw [hρ', h1, h2])]
    | local_ f i => simp [updStorage, hρ']
    | held k => simp [updStorage, hρ']

/-- **The frames' locals still conform** after a write of a value of the
location's type. -/
theorem rootsOk_update {s : State} {l : Location} {v w w' : Value} {T t : RuntimeType}
    (hnd : (s.frames.map (·.id)).Nodup) (hroots : RootsOk P s) (hroot : s.root? l.root = some w)
    (hset : w.set? l.path v = some w') (hT : RootType P s l.root = some T)
    (hpath : pathType P T l.path = some t) (hv : v.conforms P t = true) (g : Frame)
    (hg : g ∈ s.frames) :
    ∃ tys, P.localTypes (updFrame l w' g).function = some tys ∧
      (updFrame l w' g).locals.size = tys.length ∧
      ∀ (i : Nat) (u : Value), (updFrame l w' g).locals[i]? = some (some u) →
        ∃ t : RuntimeType, tys[i]? = some t ∧ u.conforms P t = true := by
  obtain ⟨tys, htys, hsize, hconf⟩ := hroots g hg
  rw [(updFrame_eq l w' g).2.1]
  by_cases hl : ∃ i, l.root = .local_ g.id i
  · obtain ⟨i, hi⟩ := hl
    rw [updFrame_locals hi]
    have hgf : s.frame? g.id = some g := frame?_of_mem hnd hg
    rw [hi] at hroot hT
    simp only [State.root?, hgf, Option.bind_some, bind] at hroot
    simp only [RootType, hgf, Option.bind_some, htys] at hT
    have hroot' : g.locals[i]? = some (some w) := by
      cases hgl : g.locals[i]? with
      | none => rw [hgl] at hroot; simp at hroot
      | some o =>
        rw [hgl] at hroot
        cases o with
        | none => simp at hroot
        | some w₀ => simp at hroot; rw [hroot]
    obtain ⟨T', hT', hwT⟩ := hconf i w hroot'
    rw [hT, Option.some.injEq] at hT'
    subst hT'
    have hw'T := set_conforms hwT hpath hv hset
    refine ⟨tys, htys, by rw [Array.size_setIfInBounds, hsize], fun j u hu => ?_⟩
    rw [Array.getElem?_setIfInBounds] at hu
    split at hu
    · next hij =>
      subst hij
      split at hu
      · simp only [Option.some.injEq] at hu
        subst hu
        exact ⟨T, hT, hw'T⟩
      · cases hu
    · exact hconf j u hu
  · rw [updFrame_ne (fun i hi => hl ⟨i, hi⟩)]
    exact ⟨tys, htys, hsize, hconf⟩

/-- **Global storage stays well typed** after a write of a value of the
location's type. -/
theorem storage_update {s : State} {l : Location} {v w w' : Value} {T t : RuntimeType}
    (hstor : ∀ r ∈ s.storage, r.value.conforms P (.struct r.type) = true)
    (hroot : s.root? l.root = some w) (hset : w.set? l.path v = some w')
    (hT : RootType P s l.root = some T) (hpath : pathType P T l.path = some t)
    (hv : v.conforms P t = true) :
    ∀ r ∈ updStorage l w' s.storage, r.value.conforms P (.struct r.type) = true := by
  intro r hr
  cases hρ : l.root with
  | global a t' =>
    simp only [updStorage, hρ, GlobalStorage.update, List.mem_map] at hr
    obtain ⟨r0, hr0, rfl⟩ := hr
    split
    · next hm =>
      simp only [Bool.and_eq_true, beq_iff_eq] at hm
      rw [hρ] at hroot hT
      simp only [State.root?] at hroot
      simp only [RootType, hroot, Option.bind_some] at hT
      split at hT
      · next hwf =>
        simp only [Option.some.injEq] at hT
        have hw'T := set_conforms (conforms_of (by rw [hT]; exact assignable_refl _) hwf) hpath hv hset
        -- The stored value conforms to its declared type, so `T` is that type.
        unfold GlobalStorage.get? at hroot
        rw [Option.map_eq_some_iff] at hroot
        obtain ⟨r1, hr1, hw1⟩ := hroot
        have hr1m := List.find?_some hr1
        simp only [Bool.and_eq_true, beq_iff_eq] at hr1m
        have hc1 := hstor r1 (List.mem_of_find?_eq_some hr1)
        rw [hw1, hr1m.2] at hc1
        have hTe : T = .struct t' := hT ▸ conforms_type hc1 rfl
        rw [hTe] at hw'T
        show w'.conforms P (.struct r0.type) = true
        rw [hm.2]
        exact hw'T
      · cases hT
    · exact hstor r0 hr0
  | local_ f i => simp only [updStorage, hρ] at hr; exact hstor r hr
  | held k => simp only [updStorage, hρ] at hr; exact hstor r hr

/-- **A tracked reference not strictly inside the location written stays
tracked.** -/
theorem RefOk.afterUpdate {s s₂ : State} {l : Location} {v w w' : Value} {T t : RuntimeType}
    {frx frx' : Frame} {rx : Reference} (h : RefOk P s frx rx) (hidx : frx'.id = frx.id)
    (hw : AfterWrite P s s₂ l w') (hroot : s.root? l.root = some w)
    (hset : w.set? l.path v = some w')
    (hT : RootType P s l.root = some T) (hpath : pathType P T l.path = some t)
    (hv : v.conforms P t = true) (hin : ¬ rx.location.StrictlyInside l) :
    RefOk P s₂ frx' rx := by
  refine h.transferR hidx (rootType_update hw hroot hset hT hpath hv _)
    (es := [.mutated l]) hw.invalidations (fun e he => ?_)
    (read_update hw hroot hset hin)

  simp only [List.mem_singleton] at he
  subst he
  cases hc : Invalidation.covers (.mutated l) rx.location with
  | false => rfl
  | true =>
    exfalso
    simp only [Invalidation.covers, Bool.and_eq_true, beq_iff_eq, bne_iff_ne, ne_eq,
      List.isPrefixOf_iff_prefix] at hc
    obtain ⟨⟨h1, h2⟩, h3⟩ := hc
    exact hin ⟨⟨h1, h2⟩, fun e => h3 e.symm⟩

/-! ## The state after a write -/

theorem updFrame_locals_eq {l : Location} {w' : Value} {g1 g2 : Frame} (h1 : g1.id = g2.id)
    (h2 : g1.locals = g2.locals) : (updFrame l w' g1).locals = (updFrame l w' g2).locals := by
  unfold updFrame
  split
  · simp only [h1]
    split
    · simp [h2]
    · exact h2
  · exact h2

theorem afterWrite_writeState {s : State} {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) (st : List Value) (l : Location) (w' : Value)
    (vals : List Value) :
    AfterWrite P s (writeState s { fr with stack := st } rest l w' vals) l w' := by
  have hid1 := (updFrame_eq l w' { fr with stack := st }).1
  have hfun1 := (updFrame_eq l w' { fr with stack := st }).2.1
  refine ⟨fun f => ?_, fun f => ?_, rfl, rfl, rfl⟩
  · by_cases hf : fr.id = f
    · have h1 : (fr.id == f) = true := by simpa using hf
      have h2 : ((Step.advance (Step.pushAll (updFrame l w' { fr with stack := st }) vals)).id == f)
          = true := by simpa [Step.advance, Step.pushAll, hid1] using hf
      simp only [writeState, State.frame?, hsf, List.find?_cons, h1, h2, Option.map_some]
      simp only [Step.advance, Step.pushAll]
      exact congrArg some (updFrame_locals_eq rfl rfl)
    · have h1 : (fr.id == f) = false := by simpa using hf
      have h2 : ((Step.advance (Step.pushAll (updFrame l w' { fr with stack := st }) vals)).id == f)
          = false := by simpa [Step.advance, Step.pushAll, hid1] using hf
      simp only [writeState, State.frame?, hsf, List.find?_cons, h1, h2]
      rw [frame?_upd]
  · by_cases hf : fr.id = f
    · have h1 : (fr.id == f) = true := by simpa using hf
      have h2 : ((Step.advance (Step.pushAll (updFrame l w' { fr with stack := st }) vals)).id == f)
          = true := by simpa [Step.advance, Step.pushAll, hid1] using hf
      simp only [writeState, State.frame?, hsf, List.find?_cons, h1, h2, Option.map_some]
      simp only [Step.advance, Step.pushAll, hfun1]
    · have h1 : (fr.id == f) = false := by simpa using hf
      have h2 : ((Step.advance (Step.pushAll (updFrame l w' { fr with stack := st }) vals)).id == f)
          = false := by simpa [Step.advance, Step.pushAll, hid1] using hf
      simp only [writeState, State.frame?, hsf, List.find?_cons, h1, h2]
      rw [frame?_upd, Option.map_map]
      congr 1
      funext g
      simp [Function.comp, (updFrame_eq l w' g).2.1]

/-- A mutation at `l` covers what is strictly inside `l`, only. -/
theorem covers_mutated_false {l x : Location} (h : ¬ x.StrictlyInside l) :
    Invalidation.covers (.mutated l) x = false := by
  cases hc : Invalidation.covers (.mutated l) x with
  | false => rfl
  | true =>
    exfalso
    simp only [Invalidation.covers, Bool.and_eq_true, beq_iff_eq, bne_iff_ne, ne_eq,
      List.isPrefixOf_iff_prefix] at hc
    obtain ⟨⟨h1, h2⟩, h3⟩ := hc
    exact h ⟨⟨h1, h2⟩, fun e => h3 e.symm⟩

theorem not_strictlyInside_of_covers {l x : Location}
    (h : Invalidation.covers (.mutated l) x = false) : ¬ x.StrictlyInside l := by
  intro ⟨⟨h1, h2⟩, h3⟩
  have : Invalidation.covers (.mutated l) x = true := by
    simp [Invalidation.covers, h1, List.isPrefixOf_iff_prefix.2 h2, Ne.symm h3]
  rw [this] at h
  cases h

/-- **The roots the analysis's graph reaches from the frame's root**: a local
of the frame, or global storage of a type the frame borrows. -/
theorem RootChain.root_of {g : Graph} {L : Nat → Option Reference} {root : Nat}
    {fid : FrameId} {gsid : Nat → Option StructId} {label? : Option Label} {x : Nat}
    {rx : Reference} (h : RootChain g L root fid gsid label? x) (hx : L x = some rx) :
    (∃ i, rx.location.root = .local_ fid i) ∨
      ∃ a d t, gsid d = some t.erase ∧ rx.location.root = .global a t := by
  obtain ⟨e, n, rn, -, -, hn, ht, hrest⟩ := h
  have hroot : rx.location.root = rn.location.root := by
    rcases hrest with rfl | hch
    · rw [hn, Option.some.injEq] at hx; subst hx; rfl
    · exact (hch.facts hn hx).1
  rcases ht.root with ⟨i, hi⟩ | ⟨a, d, sid, hs, hg⟩
  · exact Or.inl ⟨i, by rw [hroot, hi]⟩
  · exact Or.inr ⟨a, d, sid, hs, by rw [hroot, hg]⟩

/-- **Where a tracked reference is**: in a local of the frame, in global
storage of a type the frame borrows, or inside a reference parameter. -/
theorem Complete.prov3 {g : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
    {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}
    (hc : Complete g L H root fid gsid ps) {x : Nat} {rx : Reference} (hx : H x)
    (hxL : L x = some rx) :
    (∃ i, rx.location.root = .local_ fid i) ∨
      (∃ a d t, gsid d = some t.erase ∧ rx.location.root = .global a t) ∨
      ∃ π ∈ ps, rx.location.Inside π.location ∧ (rx.mutable = true → π.mutable = true) := by
  rcases hc.prov x rx hx hxL with h | ⟨d, h⟩ | h
  · exact Or.inl h
  · exact (h.root_of hxL).elim (fun h => Or.inl h) (fun h => Or.inr (Or.inl h))
  · exact Or.inr (Or.inr h)

/-- **What a write through a tracked mutable reference of frame `fid` may
invalidate**: inside the frame's locals, inside a mutable parameter, or
global storage of a type the frame acquires, which nothing below protects. -/
theorem allowed_write {g : Graph} {L : Nat → Option Reference} {H : Nat → Prop} {root : Nat}
    {fid : FrameId} {ps : List Reference} {f : FunctionRef} {b : Bool}
    {sk : List (FunctionRef × Bool)} (hc : Complete g L H root fid (gsidOf P ((f, b) :: sk)) ps)
    {S : StructId → Prop} (hglob : GlobOk P f ((f, b) :: sk) ps S) {x : Nat} {rx : Reference}
    (hx : H x) (hxL : L x = some rx) (hm : rx.mutable = true) :
    AllowedBy fid ps S (.mutated rx.location) := by
  intro l hl
  simp only [Invalidation.covers, Bool.and_eq_true, beq_iff_eq, bne_iff_ne, ne_eq,
    List.isPrefixOf_iff_prefix] at hl
  obtain ⟨⟨h1, h2⟩, h3⟩ := hl
  have hs : l.StrictlyInside rx.location := ⟨⟨h1, h2⟩, fun e => h3 e.symm⟩
  rcases hc.prov3 hx hxL with ⟨i, hi⟩ | ⟨a, d, t, hd, hr⟩ | ⟨π, hπ, hin, hmm⟩
  · exact Or.inl ⟨fid, i, by rw [h1, hi], Nat.le_refl _⟩
  · refine Or.inr (Or.inr ⟨a, t, by rw [h1, hr], fun hS => ?_⟩)
    obtain ⟨hacq, hcnt⟩ := gsidOf_cons hd
    have := hglob.acq t hS hacq
    omega
  · exact Or.inr (Or.inl ⟨π, hπ, hmm hm, StrictlyInside.trans_inside hs hin⟩)

/-- A value of a type a path of value type runs through is no reference. -/
theorem notRef_of_path {v : Value} {T t : RuntimeType} {path : List PathStep}
    (hv : v.conforms P T = true) (hp : pathType P T path = some t) (ht : t.isValue = true) :
    ∀ r, v ≠ .ref r := by
  rintro r rfl
  cases path with
  | nil =>
    simp only [pathType, Option.some.injEq] at hp
    subst hp
    have := assignable_isValue (conforms_assignable hv)
    simp only [Value.type, RuntimeType.isValue] at this
    rw [← this] at ht
    cases ht
  | cons st path =>
    have hr : rigid T = true := by
      simp only [pathType, bind, Option.bind_eq_some_iff] at hp
      obtain ⟨_, hs, -⟩ := hp
      exact stepType_rigid hs
    have hT := conforms_type hv hr
    simp only [Value.type] at hT
    subst hT
    simp [pathType, stepType] at hp

/-- The local a write goes to holds a value, no reference. -/
theorem written_slot {s : State} {l : Location} {w : Value} {T t : RuntimeType}
    (hnd : (s.frames.map (·.id)).Nodup) (hroots : RootsOk P s) (hroot : s.root? l.root = some w)
    (hT : RootType P s l.root = some T) (hpath : pathType P T l.path = some t)
    (ht : t.isValue = true) {g : Frame} (hg : g ∈ s.frames) :
    ∀ i, l.root = .local_ g.id i → g.locals[i]? = some (some w) ∧ ∀ r, w ≠ .ref r := by
  intro i hi
  have hgf : s.frame? g.id = some g := frame?_of_mem hnd hg
  rw [hi] at hroot hT
  simp only [State.root?, hgf, Option.bind_some, bind] at hroot
  have hslot : g.locals[i]? = some (some w) := by
    cases hgl : g.locals[i]? with
    | none => rw [hgl] at hroot; simp at hroot
    | some o =>
      rw [hgl] at hroot
      cases o with
      | none => simp at hroot
      | some w₀ => simp at hroot; rw [hroot]
  refine ⟨hslot, ?_⟩
  obtain ⟨tys, htys, -, hconf⟩ := hroots g hg
  simp only [RootType, hgf, Option.bind_some, htys] at hT
  obtain ⟨T', hT', hwT⟩ := hconf i w hslot
  rw [hT, Option.some.injEq] at hT'
  subst hT'
  exact notRef_of_path hwT hpath ht

/-- The value a write leaves in a local is no reference. -/
theorem written_value {s : State} {l : Location} {v w w' : Value} {T t : RuntimeType}
    (hnd : (s.frames.map (·.id)).Nodup) (hroots : RootsOk P s) (hroot : s.root? l.root = some w)
    (hset : w.set? l.path v = some w') (hT : RootType P s l.root = some T)
    (hpath : pathType P T l.path = some t) (hv : v.conforms P t = true) (ht : t.isValue = true)
    {g : Frame} (hg : g ∈ s.frames) : ∀ i, l.root = .local_ g.id i → ∀ r, w' ≠ .ref r := by
  intro i hi
  have hgf : s.frame? g.id = some g := frame?_of_mem hnd hg
  have hslot := (written_slot hnd hroots hroot hT hpath ht hg i hi).1
  obtain ⟨tys, htys, -, hconf⟩ := hroots g hg
  rw [hi] at hT
  simp only [RootType, hgf, Option.bind_some, htys] at hT
  obtain ⟨T', hT', hwT⟩ := hconf i w hslot
  rw [hT, Option.some.injEq] at hT'
  subst hT'
  exact notRef_of_path (set_conforms hwT hpath hv hset) hpath ht

/-- A reference in a local after a write was there before, when the write
leaves no reference there. -/
theorem updFrame_ref {l : Location} {w' : Value} {g : Frame}
    (hw' : ∀ i, l.root = .local_ g.id i → ∀ r, w' ≠ .ref r) {j : Nat} {r : Reference}
    (h : (updFrame l w' g).locals[j]? = some (some (.ref r))) :
    g.locals[j]? = some (some (.ref r)) := by
  by_cases hl : ∃ i, l.root = .local_ g.id i
  · obtain ⟨i, hi⟩ := hl
    rw [updFrame_locals hi] at h
    by_cases hji : j = i
    · subst hji
      by_cases hlt : j < g.locals.size
      · rw [Array.getElem?_setIfInBounds_self_of_lt hlt, Option.some.injEq,
          Option.some.injEq] at h
        exact absurd h (hw' j hi r)
      · rw [Array.setIfInBounds_eq_of_size_le (Nat.le_of_not_lt hlt),
          Array.getElem?_eq_none (Nat.le_of_not_lt hlt)] at h
        cases h
    · rw [Array.getElem?_setIfInBounds_ne hji] at h
      exact h
  · rw [updFrame_ne (fun i hi => hl ⟨i, hi⟩)] at h
    exact h

/-- The abstract state still describes the frame after a write into a local
holding a value. -/
theorem Desc.setLocal {A : RefState} {fr : Frame} {gsid : Nat → Option StructId}
    {ps : List Reference} {L : Nat → Option Reference} {fr' : Frame} (hd : Desc A fr gsid ps L)
    {i : Nat} {w w' : Value} (hw : fr.locals[i]? = some (some w)) (hwr : ∀ r, w ≠ .ref r)
    (hid : fr'.id = fr.id) (hstk : fr'.stack = fr.stack)
    (hloc : fr'.locals = fr.locals.setIfInBounds i (some w')) : Desc A fr' gsid ps L := by
  refine ⟨fun j x hj => ?_, fun k x hk => by rw [hstk]; exact hd.stack k x hk,
    by rw [hid]; exact hd.complete⟩
  obtain ⟨r, hr, hL⟩ := hd.locals j x hj
  by_cases hji : j = i
  · subst hji
    rw [hw, Option.some.injEq, Option.some.injEq] at hr
    exact absurd hr (hwr r)
  · exact ⟨r, by rw [hloc, Array.getElem?_setIfInBounds_ne hji]; exact hr, hL⟩

/-- **The description after a write** into a local holding a value. -/
theorem Desc.upd {A : RefState} {g : Frame} {gsid : Nat → Option StructId} {ps : List Reference}
    {L : Nat → Option Reference} (hd : Desc A g gsid ps L) {l : Location} {w' : Value}
    (hval : ∀ i, l.root = .local_ g.id i → ∃ w, g.locals[i]? = some (some w) ∧ ∀ r, w ≠ .ref r)
    {fr' : Frame} (hid : fr'.id = g.id) (hstk : fr'.stack = g.stack)
    (hloc : fr'.locals = (updFrame l w' g).locals) : Desc A fr' gsid ps L := by
  by_cases hl : ∃ i, l.root = .local_ g.id i
  · obtain ⟨i, hi⟩ := hl
    obtain ⟨w, hw, hwr⟩ := hval i hi
    exact hd.setLocal hw hwr hid hstk (by rw [hloc, updFrame_locals hi])
  · rw [updFrame_ne (fun i hi => hl ⟨i, hi⟩)] at hloc
    exact ⟨fun j x hj => by rw [hloc]; exact hd.locals j x hj,
      fun k x hk => by rw [hstk]; exact hd.stack k x hk, by rw [hid]; exact hd.complete⟩

/-- The locals after a write: a slot holding a value still does, an unavailable
one still is. -/
theorem locals_upd {l : Location} {w' : Value} {g : Frame}
    (hslot : ∀ i, l.root = .local_ g.id i → ∃ w, g.locals[i]? = some (some w)) :
    (∀ i : Nat, (∃ v, g.locals[i]? = some (some v)) →
      ∃ v, (updFrame l w' g).locals[i]? = some (some v)) ∧
    (∀ i : Nat, g.locals[i]? = some none → (updFrame l w' g).locals[i]? = some none) := by
  by_cases hl : ∃ i, l.root = .local_ g.id i
  · obtain ⟨i, hi⟩ := hl
    obtain ⟨w, hw⟩ := hslot i hi
    have hlt : i < g.locals.size := (Array.getElem?_eq_some_iff.1 hw).1
    rw [updFrame_locals hi]
    refine ⟨fun j hj => ?_, fun j hj => ?_⟩
    · by_cases hji : j = i
      · subst hji; exact ⟨w', by simp [hlt]⟩
      · rw [Array.getElem?_setIfInBounds_ne hji]; exact hj
    · have hji : j ≠ i := by rintro rfl; rw [hw] at hj; cases hj
      rw [Array.getElem?_setIfInBounds_ne hji]; exact hj
  · rw [updFrame_ne (fun i hi => hl ⟨i, hi⟩)]
    exact ⟨fun j hj => hj, fun j hj => hj⟩

/-- **A frame agreeing with the certificates still does** after a write into
one of its locals of a value of the local's type. -/
theorem ActiveAt.upd {tc : TypeCertificate} {ac : AvailabilityCertificate} {g : Frame}
    {info : FunctionInfo} {code : CodeUnit} (h : ActiveAt P tc ac g info code) {l : Location}
    {w' : Value} (hslot : ∀ i, l.root = .local_ g.id i → ∃ w, g.locals[i]? = some (some w))
    (hlocals : LocalsOk P (updFrame l w' g)) : ActiveAt P tc ac (updFrame l w' g) info code := by
  obtain ⟨-, hfun, hpc, hstk⟩ := updFrame_eq l w' g
  obtain ⟨hav, hun⟩ := locals_upd (w' := w') hslot
  refine ⟨by rw [hfun]; exact h.fn, h.body, by rw [hpc]; exact h.pcLt,
    by rw [hpc]; exact h.reachable, by rw [hfun, hpc, hstk]; exact h.types, hlocals,
    fun i hi => ?_, fun i hi => ?_, by rw [hfun]; exact h.typeArgs⟩
  · rw [hfun, hpc] at hi; exact hav i (h.available i hi)
  · rw [hfun, hpc] at hi; exact hun i (h.unavailable i hi)

theorem updFrame_with (l : Location) (w' : Value) (g : Frame) (st : List Value) (pc : Nat) :
    updFrame l w' { g with stack := st, pc := pc } =
      { updFrame l w' g with stack := st, pc := pc } := by
  unfold updFrame
  split
  · split <;> rfl
  · rfl

/-- **The suspended frames still resume correctly** after a write into one of
them of a value of the local's type. -/
theorem ChainOk.upd {tc : TypeCertificate} {ac : AvailabilityCertificate} {l : Location}
    {w' : Value} : ∀ {f : FunctionRef} {frs : List Frame}, ChainOk P tc ac f frs →
      (∀ g ∈ frs, (∀ i, l.root = .local_ g.id i → ∃ w, g.locals[i]? = some (some w)) ∧
        LocalsOk P (updFrame l w' g)) →
      ChainOk P tc ac f (frs.map (updFrame l w'))
  | _, [], _, _ => trivial
  | f, caller :: frs, ⟨⟨rets, hrets, hres⟩, hrest⟩, hfr => by
    obtain ⟨hslot, hlocals⟩ := hfr caller List.mem_cons_self
    refine ⟨⟨rets, hrets, fun vals hvals => ?_⟩, ?_⟩
    · obtain ⟨info, code, ha⟩ := hres vals hvals
      have ha' := ha.upd (l := l) (w' := w') (by simpa using hslot) (by
        rw [updFrame_with]
        obtain ⟨tys, h1, h2, h3⟩ := hlocals
        exact ⟨tys, h1, h2, h3⟩)
      rw [updFrame_with] at ha'
      obtain ⟨-, -, hpc, hstk⟩ := updFrame_eq l w' caller
      exact ⟨info, code, by rw [hpc, hstk]; exact ha'⟩
    · have := ChainOk.upd hrest (fun g hg => hfr g (List.mem_cons_of_mem _ hg))
      rw [(updFrame_eq l w' caller).2.1]
      exact this

/-- **A tracked reference of a frame an abstract state describes** is one of
the frame's values: in a local, or in the operand stack below its top `pre`
when the reference's identity is below the abstract stack's top. -/
theorem Desc.mem_vals_drop {A : RefState} {fr : Frame} {gsid : Nat → Option StructId}
    {ps : List Reference} {L : Nat → Option Reference} (hd : Desc A fr gsid ps L)
    {pre st : List Value} (hstk : fr.stack = pre ++ st) (hh : A.stack.length = fr.stack.length)
    {x : Nat} {r : Reference} (hx : x ∈ heldIds { A with stack := A.stack.drop pre.length })
    (hL : L x = some r) : Value.ref r ∈ st ∨ ∃ i : Nat, fr.locals[i]? = some (some (.ref r)) := by
  rcases mem_heldIds.1 hx with hx | hx
  · obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 hx
    obtain ⟨r', hr', hL'⟩ := hd.locals i x hi
    rw [hL, Option.some.injEq] at hL'
    subst hL'
    exact Or.inr ⟨i, hr'⟩
  · obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hx
    simp only [List.getElem?_drop] at hj
    have hlt : pre.length + j < A.stack.length := (List.getElem?_eq_some_iff.1 hj).1
    have hk : A.stack.reverse[A.stack.length - 1 - (pre.length + j)]? =
        some (AbsValue.reference x) := by
      rw [List.getElem?_reverse (by omega)]
      have e : A.stack.length - 1 - (A.stack.length - 1 - (pre.length + j)) = pre.length + j := by
        omega
      rw [e]; exact hj
    obtain ⟨r', hr', hL'⟩ := hd.stack _ x hk
    rw [hL, Option.some.injEq] at hL'
    subst hL'
    rw [List.getElem?_reverse (by omega)] at hr'
    have e : fr.stack.length - 1 - (A.stack.length - 1 - (pre.length + j)) = pre.length + j := by
      omega
    rw [e, hstk, List.getElem?_append_right (by omega), Nat.add_sub_cancel_left] at hr'
    exact Or.inl (List.mem_of_getElem? hr')

theorem Desc.mem_vals {A : RefState} {fr : Frame} {gsid : Nat → Option StructId}
    {ps : List Reference} {L : Nat → Option Reference} (hd : Desc A fr gsid ps L)
    {x : Nat} {r : Reference} (hx : x ∈ heldIds A) (hL : L x = some r) :
    Value.ref r ∈ frameVals fr := by
  rcases mem_heldIds.1 hx with hx | hx
  · obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 hx
    obtain ⟨r', hr', hL'⟩ := hd.locals i x hi
    rw [hL, Option.some.injEq] at hL'
    subst hL'
    exact mem_frameVals_locals hr'
  · obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 (List.mem_reverse.2 hx)
    obtain ⟨r', hr', hL'⟩ := hd.stack k x hk
    rw [hL, Option.some.injEq] at hL'
    subst hL'
    exact mem_frameVals_stack (List.mem_reverse.1 (List.mem_of_getElem? hr'))

/-- **What a write of `w'` in place of `w` at the root of `l` keeps**, however
it changes the root's type: the local it writes, if any, holds a value before
and after; the frames' locals conform; global storage is well typed; and every
reference of the frames not strictly inside `l` stays tracked. -/
structure WriteFacts (P : Program) (s s₂ : State) (l : Location) (w w' : Value) : Prop where
  slot : ∀ g ∈ s.frames, ∀ i, l.root = .local_ g.id i →
    g.locals[i]? = some (some w) ∧ ∀ r, w ≠ .ref r
  value : ∀ g ∈ s.frames, ∀ i, l.root = .local_ g.id i → ∀ r, w' ≠ .ref r
  locals : ∀ g ∈ s.frames, LocalsOk P (updFrame l w' g)
  storage : ∀ r ∈ updStorage l w' s.storage, r.value.conforms P (.struct r.type) = true
  refOk : ∀ g ∈ s.frames, ∀ rx, Value.ref rx ∈ frameVals g → ∀ frx frx', RefOk P s frx rx →
    frx'.id = frx.id → ¬ rx.location.StrictlyInside l → RefOk P s₂ frx' rx

/-- **A write of a value of the location's type.** -/
theorem WriteFacts.exact {s s₂ : State} {l : Location} {v w w' : Value} {T t : RuntimeType}
    (hw : AfterWrite P s s₂ l w') (hnd : (s.frames.map (·.id)).Nodup) (hroots : RootsOk P s)
    (hstor : ∀ r ∈ s.storage, r.value.conforms P (.struct r.type) = true)
    (hroot : s.root? l.root = some w) (hset : w.set? l.path v = some w')
    (hT : RootType P s l.root = some T) (hpath : pathType P T l.path = some t)
    (hv : v.conforms P t = true) (ht : t.isValue = true) : WriteFacts P s s₂ l w w' :=
  ⟨fun _ hg => written_slot hnd hroots hroot hT hpath ht hg,
    fun _ hg => written_value hnd hroots hroot hset hT hpath hv ht hg,
    fun g hg => rootsOk_update hnd hroots hroot hset hT hpath hv g hg,
    storage_update hstor hroot hset hT hpath hv,
    fun _ _ _ _ _ _ h hid hin => h.afterUpdate hid hw hroot hset hT hpath hv hin⟩

/-- **A tracked reference after a write of the whole of held value `k`**, a
closure, of a value of a type assignable to `t`, the target of every
reference to the whole of it. -/
theorem RefOk.afterHeldWrite {s s₂ : State} {k : Nat} {v w : Value} {T t : RuntimeType}
    {frx frx' : Frame} {rx : Reference} (h : RefOk P s frx rx) (hidx : frx'.id = frx.id)
    (hw : AfterWrite P s s₂ ⟨.held k, []⟩ v) (hroot : s.root? (.held k) = some w)
    (hT : RootType P s (.held k) = some T) (hrig : rigid T = false) (hv : v.conforms P t = true)
    (htgt : rx.location = ⟨.held k, []⟩ → rx.target = t)
    (hin : ¬ rx.location.StrictlyInside ⟨.held k, []⟩) : RefOk P s₂ frx' rx := by
  have hset : w.set? [] v = some v := rfl
  refine h.transferRoot hidx ?_ (es := [.mutated ⟨.held k, []⟩]) hw.invalidations
    (fun e he => ?_) (read_update hw hroot hset hin)
  · obtain ⟨T₀, T₀', hT₀, hp₀, hf₀⟩ := h.root
    by_cases hρ : rx.location.root = .held k
    · have hpe : rx.location.path = [] := by
        cases hp : rx.location.path with
        | nil => rfl
        | cons st p =>
          rw [hp] at hp₀
          rw [hρ, hT, Option.some.injEq] at hT₀
          subst hT₀
          simp only [pathType, bind, Option.bind_eq_some_iff] at hp₀
          obtain ⟨_, hs, -⟩ := hp₀
          rw [stepType_rigid hs] at hrig
          cases hrig
      have hloc : rx.location = ⟨.held k, []⟩ := by
        rcases hl : rx.location with ⟨ρ, p⟩
        rw [hl] at hρ hpe
        simp only at hρ hpe
        rw [hρ, hpe]
      have hk : k < s.held.length := by
        simp only [State.root?] at hroot
        exact (List.getElem?_eq_some_iff.1 hroot).1
      refine ⟨v.type, v.type, ?_, by rw [hpe]; rfl,
        Or.inr ⟨⟨hpe, by rw [hρ]; exact fun _ _ e => Root.noConfusion e⟩,
          by rw [htgt hloc]; exact conforms_assignable hv⟩⟩
      rw [hρ]
      simp only [RootType, hw.held, updHeld, List.getElem?_set_self hk, Option.bind_some,
        (conforms_iff.1 hv).2, ↓reduceIte]
    · exact ⟨T₀, T₀', by rw [rootType_update_other hw (fun e => hρ e)]; exact hT₀, hp₀, hf₀⟩
  · simp only [List.mem_singleton] at he
    subst he
    exact covers_mutated_false hin

/-- **A write of the whole of held value `k`**, a closure, of a value of a type
assignable to `t`, the target of every reference to the whole of it. -/
theorem WriteFacts.held {s s₂ : State} {k : Nat} {v w : Value} {T t : RuntimeType}
    (hw : AfterWrite P s s₂ ⟨.held k, []⟩ v) (hroots : RootsOk P s)
    (hstor : ∀ r ∈ s.storage, r.value.conforms P (.struct r.type) = true)
    (hroot : s.root? (.held k) = some w)
    (hT : RootType P s (.held k) = some T) (hrig : rigid T = false) (hv : v.conforms P t = true)
    (htgt : ∀ g ∈ s.frames, ∀ rx, Value.ref rx ∈ frameVals g → rx.location = ⟨.held k, []⟩ →
      rx.target = t) : WriteFacts P s s₂ ⟨.held k, []⟩ w v :=
  ⟨fun _ _ _ e => Root.noConfusion e, fun _ _ _ e => Root.noConfusion e,
    fun g hg => hroots g hg, hstor,
    fun g hg rx hm _ _ h hid hin => h.afterHeldWrite hid hw hroot hT hrig hv (htgt g hg rx hm) hin⟩

/-- **The suspended frames keep their references** across a write through the
current frame that frame `cid` up may make. -/
theorem ChainRef.afterWrite {s s₂ : State} {l : Location} {w w' : Value}
    (hwf : WriteFacts P s s₂ l w w') :
    ∀ (frs : List Frame) (cid : FrameId) (cfn : FunctionRef) (cps : List Reference)
      (Sc : StructId → Prop),
      (∀ g ∈ frs, g ∈ s.frames) → AllowedBy cid cps Sc (.mutated l) →
      ChainRef P s cid cfn cps Sc frs → ChainRef P s₂ cid cfn cps Sc (frs.map (updFrame l w'))
  | [], _, _, _, _, _, _, _ => trivial
  | caller :: frs, cid, cfn, cps, Sc, hin, hal, ⟨ps, S, hsusp, hrest⟩ => by
    have hcm := hin caller List.mem_cons_self
    obtain ⟨hid, hfun, hpc, hstk⟩ := updFrame_eq l w' caller
    have hsk : (updFrame l w' caller :: frs.map (updFrame l w')).map Frame.skel =
        (caller :: frs).map Frame.skel := by
      rw [← List.map_cons, map_updFrame_skel]
    refine ⟨ps, S, ?_, ?_⟩
    · obtain ⟨m, rs, rets, args, A, A', A'', L, hf⟩ := hsusp.call
      have hd := hf.desc
      have hrefs := hf.refs
      have hexp := hf.exposure
      refine ⟨by rw [hid]; exact hsusp.order, hwf.locals _ hcm,
        hsusp.glob.skel hfun hsk, hsusp.sub,
        m, rs, rets, args, A, A', A'', L,
        { hf with
          after := by rw [hfun, hpc]; exact hf.after
          cpsBelow := by rw [hid]; exact hf.cpsBelow
          height := by rw [hstk]; exact hf.height
          desc := ?_
          refs := fun x r hx hxr => ?_
          exposure := ?_
          stackConf := fun v hv => hf.stackConf v (by rw [← hstk]; exact hv)
          tracked := fun r hr => hf.tracked r (by
            rcases hr with hr | ⟨j, hj, hav⟩
            · exact Or.inl (by rw [← hstk]; exact hr)
            · refine Or.inr ⟨j, updFrame_ref (hwf.value _ hcm) hj, by rw [← hfun, ← hpc]; exact hav⟩) }⟩
      · rw [hsk]
        refine hd.upd (l := l) (w' := w') (fun i hi => ?_) (by simp only; rw [hid])
          (by simp only; rw [hstk]) (updFrame_locals_eq rfl rfl)
        obtain ⟨hs, hwr⟩ := hwf.slot _ hcm i hi
        exact ⟨w, hs, hwr⟩
      · obtain ⟨hok, hsh⟩ := hrefs x r hx hxr
        have hmem : Value.ref r ∈ frameVals caller := by
          rcases hd.mem_vals_drop (pre := args.reverse) rfl (by simp [hf.height])
            (by simpa using hx) hxr with h | ⟨i, h⟩
          · exact mem_frameVals_stack h
          · exact mem_frameVals_locals h
        refine ⟨hwf.refOk caller hcm r hmem _ _ hok hid ?_, hsh⟩
        exact not_strictlyInside_of_covers
          (hsusp.not_covers hal hok hsh (hf.globals x r (heldIds_drop hx) hxr))
      · intro π hπ hm
        rcases hexp π hπ hm with ⟨i, hi⟩ | h | h
        · exact Or.inl ⟨i, by rw [hid]; exact hi⟩
        · exact Or.inr (Or.inl (by rw [hsk]; exact h))
        · exact Or.inr (Or.inr h)
    · rw [hid, hfun]
      exact ChainRef.afterWrite hwf frs caller.id
        caller.function ps S (fun g hg => hin g (List.mem_cons_of_mem _ hg)) (hsusp.allowed hal)
        hrest

/-! ## A write in place -/

/-- The state after a write at `l`, whose root now holds `w'`, with every
frame's operand stack and position unchanged. -/
def writeAt (s : State) (l : Location) (w' : Value) : State :=
  { s with
    frames := s.frames.map (updFrame l w')
    held := updHeld l w' s.held
    storage := updStorage l w' s.storage
    invalidations := .mutated l :: s.invalidations }

theorem afterWrite_writeAt (s : State) (l : Location) (w' : Value) :
    AfterWrite P s (writeAt s l w') l w' :=
  ⟨fun f => by simp only [writeAt, State.frame?]; rw [frame?_upd, Option.map_map],
    fun f => by
      simp only [writeAt, State.frame?]
      rw [frame?_upd, Option.map_map]
      congr 1
      funext g
      exact (updFrame_eq l w' g).2.1, rfl, rfl, rfl⟩

theorem writeAt_frames {s : State} {fr : Frame} {rest : List Frame} (hsf : s.frames = fr :: rest)
    (l : Location) (w' : Value) :
    (writeAt s l w').frames = updFrame l w' fr :: rest.map (updFrame l w') := by
  simp [writeAt, hsf]

/-- A write, with its invalidation, is a write in place. -/
theorem setRoot_invalidate {s : State} {l : Location} {w' : Value} :
    (s.setRoot l.root w').invalidate (.mutated l) = writeAt s l w' := by
  have hid : ∀ (h : ∀ f i, l.root ≠ .local_ f i), s.frames.map (updFrame l w') = s.frames := by
    intro h
    conv => rhs; rw [← List.map_id s.frames]
    exact List.map_congr_left fun g _ => updFrame_ne fun i => h g.id i
  simp only [writeAt, State.invalidate]
  cases hρ : l.root with
  | local_ f i => simp [State.setRoot, updFrame, updHeld, updStorage, hρ]
  | held k =>
    rw [hid (fun f i e => by rw [hρ] at e; cases e)]
    simp [State.setRoot, updHeld, updStorage, hρ]
  | global a t =>
    rw [hid (fun f i e => by rw [hρ] at e; cases e)]
    simp [State.setRoot, updHeld, updStorage, hρ]

end MoveBytecode.Proofs
