-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefValues
import Proofs.RefCert

/-!
# The references a frame holds, at run time

UNTRUSTED (checked by the kernel). A frame of a running state agrees with an
abstract state of the reference-safety analysis (`FrameOk`) when the abstract
state describes it (`Desc`) and every reference it tracks is a valid
reference whose target is the type of the values at its location (`RefOk`):
into a local of its own frame or of an older one (through a reference
parameter), a value the transaction holds, or a global resource. This file
has the facts about validity that do not depend on the instruction.

A frame's global borrows are of the types its function acquires, while its
module is active only once (`gsidOf`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

/-! ## Activations and global borrows -/

/-- What a frame contributes to the active counts: its function and whether
it counts as an activation of its module. -/
def _root_.MoveBytecode.Frame.skel (fr : Frame) : FunctionRef × Bool :=
  (fr.function, fr.countedActive)

/-- The module a function belongs to. -/
def funModule (P : Program) (f : FunctionRef) : Option ModuleId :=
  (P.function? f).map (·.moduleId)

/-- The frames counted as activations of module `M`. -/
def countActive (P : Program) (sk : List (FunctionRef × Bool)) (M : ModuleId) : Nat :=
  (sk.filter fun p => p.2 && funModule P p.1 == some M).length

/-- The struct definitions a function's acquires annotations name. -/
def acqOf (P : Program) (f : FunctionRef) : List Nat :=
  ((P.function? f).map (·.definition.acquiresGlobalResources)).getD []

/-- The types a function acquires: the instances of the structs it names. -/
def acqTypes (P : Program) (f : FunctionRef) (t : StructId) : Prop :=
  ∃ d ∈ acqOf P f, P.definedStruct? f.module d = some t.erase

/-- **The types of global storage a frame may borrow**: those its function
acquires, while its module is active only once in it and the frames below
(`sk`, the frame's first). Global borrows of the analysis name struct
definitions of the frame's module. -/
def gsidOf (P : Program) : List (FunctionRef × Bool) → Nat → Option StructId
  | [], _ => none
  | p :: rest, d =>
    match funModule P p.1 with
    | some M =>
      if d ∈ acqOf P p.1 ∧ countActive P (p :: rest) M ≤ 1 then P.definedStruct? p.1.module d
      else none
    | none => none

/-- **The type of the values at a root**: a local's declared type in a frame
that has not returned, the type of a well-formed held value or global
resource. -/
def RootType (P : Program) (s : State) : Root → Option RuntimeType
  | .local_ f i => (s.frame? f).bind fun g => (P.localTypes g.function).bind (·[i]?)
  | .held k => (s.held[k]?).bind fun v => if v.wellFormed P then some v.type else none
  | .global a t => (s.storage.get? a t).bind fun v => if v.wellFormed P then some v.type else none

/-- The root is a local of frame `fid` or of an older frame, a held value, or
global storage. -/
def RootBelow (fid : FrameId) : Root → Prop
  | .local_ f _ => f ≤ fid
  | .held _ => True
  | .global _ _ => True

/-- **How the type along a location gives a reference's target**: exactly,
or, for the whole of a held value or a resource, as a type assignable to it. -/
def TargetFits (l : Location) (T' t : RuntimeType) : Prop :=
  T' = t ∨ (l.path = [] ∧ ∀ f i, l.root ≠ .local_ f i) ∧ T'.assignable t = true

theorem TargetFits.assignable {l : Location} {T' t : RuntimeType} (h : TargetFits l T' t) :
    T'.assignable t = true := by
  rcases h with rfl | ⟨-, h⟩
  · exact assignable_refl _
  · exact h

/-- Exactly, through a nonempty path or into a local. -/
theorem TargetFits.eq {l : Location} {T' t : RuntimeType} (h : TargetFits l T' t)
    (hl : l.path ≠ [] ∨ ∃ f i, l.root = .local_ f i) : T' = t := by
  rcases h with h | ⟨⟨hp, hr⟩, -⟩
  · exact h
  · rcases hl with hl | ⟨f, i, hfi⟩
    · exact absurd hp hl
    · exact absurd hfi (hr f i)

/-- Exactly, when the type is rigid. -/
theorem TargetFits.eq_rigid {l : Location} {T' t : RuntimeType} (h : TargetFits l T' t)
    (hr : rigid T' = true) : T' = t :=
  assignable_rigid_left h.assignable hr

/-- **A tracked reference** of frame `fr` in state `s`. -/
structure RefOk (P : Program) (s : State) (fr : Frame) (r : Reference) : Prop where
  /-- The root's type, along the path, gives the reference's target
  (`TargetFits`): equal to it, or, for a whole held value or resource, a type
  assignable to it, since a write through another reference can replace a
  whole held closure by one of any type assignable to that reference's. -/
  root : ∃ T T', RootType P s r.location.root = some T ∧ pathType P T r.location.path = some T' ∧
    TargetFits r.location T' r.target
  below : RootBelow fr.id r.location.root
  value : r.target.isValue = true
  since : r.since ≤ s.invalidations.length
  valid : (s.deref? r.location r.since).isSome = true
  /-- A mutable reference can be written through: its path goes through no
  signer's address (`PathStep.writable`), which only `signer::borrow_address`
  borrows, immutably. -/
  writable : r.mutable = true → r.location.path.all PathStep.writable = true

/-- **The current frame agrees with an abstract state** of the
reference-safety analysis, given its reference parameters `ps`. -/
def FrameOk (P : Program) (s : State) (fr : Frame) (ps : List Reference) (A : RefState) : Prop :=
  ∃ L, Desc A fr (gsidOf P (s.frames.map Frame.skel)) ps L ∧
    ∀ x r, x ∈ heldIds A → L x = some r → RefOk P s fr r

theorem FrameOk.le {P : Program} {s : State} {fr : Frame} {ps : List Reference} {A B : RefState}
    (h : FrameOk P s fr ps A) (hle : RefLe A B) : FrameOk P s fr ps B := by
  obtain ⟨L, hd, hok⟩ := h
  obtain ⟨L', hd', hsub⟩ := hle fr _ ps L hd
  refine ⟨L', hd', fun x r hx hr => ?_⟩
  obtain ⟨y, hy, hyr⟩ := hsub x r hx hr
  exact hok y r hy hyr

/-- Every frame's locals conform to their declared types. -/
def RootsOk (P : Program) (s : State) : Prop :=
  ∀ g ∈ s.frames, ∃ tys, P.localTypes g.function = some tys ∧ g.locals.size = tys.length ∧
    ∀ (i : Nat) (v : Value), g.locals[i]? = some (some v) →
      ∃ t : RuntimeType, tys[i]? = some t ∧ v.conforms P t = true

/-! ## Validity -/

theorem deref?_isSome {s : State} {l : Location} {since : Nat} :
    (s.deref? l since).isSome = true ↔
      (∀ e ∈ s.invalidatedSince since, e.covers l = false) ∧ (s.read? l).isSome = true := by
  unfold State.deref?
  split
  · next h =>
    simp only [List.any_eq_true] at h
    obtain ⟨e, he, hc⟩ := h
    simp only [Option.isSome_none, Bool.false_eq_true, false_iff, not_and]
    intro hall
    rw [hall e he] at hc
    cases hc
  · next h =>
    simp only [List.any_eq_true, not_exists, not_and, Bool.not_eq_true] at h
    exact ⟨fun hr => ⟨h, hr⟩, fun ⟨_, hr⟩ => hr⟩

theorem invalidatedSince_append {s s' : State} {es : List Invalidation} {since : Nat}
    (hsince : since ≤ s.invalidations.length) (hes : s'.invalidations = es ++ s.invalidations) :
    s'.invalidatedSince since = es ++ s.invalidatedSince since := by
  unfold State.invalidatedSince
  rw [hes, List.take_append, List.length_append]
  rw [List.take_of_length_le (by omega)]
  congr 2
  omega

/-- **Validity persists** across later invalidations that do not cover the
location, while the location still resolves. -/
theorem valid_transfer {s s' : State} {l : Location} {since : Nat} {es : List Invalidation}
    (hsince : since ≤ s.invalidations.length) (hes : s'.invalidations = es ++ s.invalidations)
    (hcov : ∀ e ∈ es, e.covers l = false)
    (hread : (s.read? l).isSome = true → (s'.read? l).isSome = true)
    (h : (s.deref? l since).isSome = true) : (s'.deref? l since).isSome = true := by
  rw [deref?_isSome] at h ⊢
  refine ⟨fun e he => ?_, hread h.2⟩
  rw [invalidatedSince_append hsince hes, List.mem_append] at he
  rcases he with he | he
  · exact hcov e he
  · exact h.1 e he

/-- The frames of a state, found by identity. -/
theorem frame?_of_mem {s : State} (hnd : (s.frames.map (·.id)).Nodup) {fr : Frame}
    (h : fr ∈ s.frames) : s.frame? fr.id = some fr := by
  unfold State.frame?
  generalize s.frames = frs at hnd h
  induction frs with
  | nil => simp at h
  | cons g frs ih =>
    simp only [List.map_cons, List.nodup_cons, List.mem_map, not_exists, not_and] at hnd
    rcases List.mem_cons.1 h with rfl | h
    · simp
    · have hne : g.id ≠ fr.id := fun e => hnd.1 fr h e.symm
      have hb : (g.id == fr.id) = false := by simpa using hne
      simp only [List.find?_cons, hb]
      exact ih hnd.2 h

theorem frame?_mem {s : State} {f : FrameId} {g : Frame} (h : s.frame? f = some g) :
    g ∈ s.frames ∧ g.id = f := by
  unfold State.frame? at h
  exact ⟨List.mem_of_find?_eq_some h, by simpa using List.find?_some h⟩

theorem read?_local {s : State} {f : FrameId} {fr : Frame} (h : s.frame? f = some fr) (i : Nat)
    (path : List PathStep) :
    s.read? ⟨.local_ f i, path⟩ = (fr.locals[i]?.bind id).bind (·.get? path) := by
  simp only [State.read?, State.root?, h, bind, Option.bind]
  cases fr.locals[i]? with
  | none => rfl
  | some o => cases o <;> rfl

theorem read?_held {s : State} (k : Nat) (path : List PathStep) :
    s.read? ⟨.held k, path⟩ = (s.held[k]?).bind (·.get? path) := by
  simp only [State.read?, State.root?, bind]

theorem Location.eta (l : Location) : l = ⟨l.root, l.path⟩ := rfl

/-- **A tracked reference stays tracked** across a step that keeps the type of
its root, still resolves its location, and records no invalidation that
covers it. -/
theorem RefOk.transfer' {P : Program} {s s' : State} {fr fr' : Frame} {r : Reference}
    (h : RefOk P s fr r) (hid : fr'.id = fr.id)
    (hroot : RootType P s' r.location.root = RootType P s r.location.root)
    {es : List Invalidation} (hes : s'.invalidations = es ++ s.invalidations)
    (hcov : ∀ e ∈ es, e.covers r.location = false)
    (hread : (s.read? r.location).isSome = true → (s'.read? r.location).isSome = true) :
    RefOk P s' fr' r := by
  obtain ⟨⟨T, T', hT, hpath, hasg⟩, hbelow, hval, hsince, hvalid, hwr⟩ := h
  exact ⟨⟨T, T', by rw [hroot]; exact hT, hpath, hasg⟩, by rw [hid]; exact hbelow, hval,
    by rw [hes, List.length_append]; omega, valid_transfer hsince hes hcov hread hvalid, hwr⟩

/-- A root type `T₂` refines `T` at root `ρ`: it is `T`, or, for a held value
or a resource, a type assignable to `T`, which differs from `T` only for a
whole closure, of a type with more abilities. -/
def RootRefines (ρ : Root) (T₂ T : RuntimeType) : Prop :=
  T₂ = T ∨ (∀ f i, ρ ≠ .local_ f i) ∧ T₂.assignable T = true

theorem pathType_refines {P : Program} {ρ : Root} {T₂ T T' : RuntimeType}
    (h : RootRefines ρ T₂ T) {path : List PathStep} (hp : pathType P T path = some T') :
    ∃ T₂', pathType P T₂ path = some T₂' ∧ TargetFits ⟨ρ, path⟩ T₂' T' := by
  rcases h with rfl | ⟨hρ, hasg⟩
  · exact ⟨T', hp, Or.inl rfl⟩
  cases path with
  | nil =>
    simp only [pathType, Option.some.injEq] at hp
    subst hp
    exact ⟨T₂, rfl, Or.inr ⟨⟨rfl, hρ⟩, hasg⟩⟩
  | cons step path =>
    have hr : rigid T = true := by
      simp only [pathType, bind, Option.bind_eq_some_iff] at hp
      obtain ⟨_, hs, -⟩ := hp
      exact stepType_rigid hs
    rw [assignable_rigid hasg hr]
    exact ⟨T', hp, Or.inl rfl⟩

theorem TargetFits.trans {l : Location} {T₂ T' t : RuntimeType} (h₂ : TargetFits l T₂ T')
    (h : TargetFits l T' t) : TargetFits l T₂ t := by
  rcases h₂ with rfl | ⟨hl, h₂⟩
  · exact h
  · exact Or.inr ⟨hl, assignable_trans h₂ h.assignable⟩

/-- **A tracked reference stays tracked** across a step that refines the type of
its root, still resolves its location, and records no invalidation that
covers it. -/
theorem RefOk.transferRoot {P : Program} {s s' : State} {fr fr' : Frame} {r : Reference}
    (h : RefOk P s fr r) (hid : fr'.id = fr.id)
    (hroot : ∃ T T', RootType P s' r.location.root = some T ∧ pathType P T r.location.path = some T' ∧
      TargetFits r.location T' r.target)
    {es : List Invalidation} (hes : s'.invalidations = es ++ s.invalidations)
    (hcov : ∀ e ∈ es, e.covers r.location = false)
    (hread : (s.read? r.location).isSome = true → (s'.read? r.location).isSome = true) :
    RefOk P s' fr' r := by
  obtain ⟨-, hbelow, hval, hsince, hvalid, hwr⟩ := h
  exact ⟨hroot, by rw [hid]; exact hbelow, hval,
    by rw [hes, List.length_append]; omega, valid_transfer hsince hes hcov hread hvalid, hwr⟩

theorem RefOk.transferR {P : Program} {s s' : State} {fr fr' : Frame} {r : Reference}
    (h : RefOk P s fr r) (hid : fr'.id = fr.id)
    (hroot : ∀ T, RootType P s r.location.root = some T →
      ∃ T₂, RootType P s' r.location.root = some T₂ ∧ RootRefines r.location.root T₂ T)
    {es : List Invalidation} (hes : s'.invalidations = es ++ s.invalidations)
    (hcov : ∀ e ∈ es, e.covers r.location = false)
    (hread : (s.read? r.location).isSome = true → (s'.read? r.location).isSome = true) :
    RefOk P s' fr' r := by
  obtain ⟨⟨T, T', hT, hpath, hasg⟩, hbelow, hval, hsince, hvalid, hwr⟩ := h
  obtain ⟨T₂, hT₂, href⟩ := hroot T hT
  obtain ⟨T₂', hp₂, hasg₂⟩ := pathType_refines href hpath
  exact ⟨⟨T₂, T₂', hT₂, hp₂, hasg₂.trans hasg⟩, by rw [hid]; exact hbelow, hval,
    by rw [hes, List.length_append]; omega, valid_transfer hsince hes hcov hread hvalid, hwr⟩

/-- Roots keep their types when every frame of `s` is in `s'` with the same
function and the held values are the same. -/
theorem rootType_eq {P : Program} {s s' : State}
    (hfr : ∀ f g, s.frame? f = some g → ∃ g', s'.frame? f = some g' ∧ g'.function = g.function)
    (hheld : s'.held = s.held) (hstor : s'.storage = s.storage) {ρ : Root} {T : RuntimeType}
    (h : RootType P s ρ = some T) : RootType P s' ρ = RootType P s ρ := by
  cases ρ with
  | local_ f i =>
    simp only [RootType, Option.bind_eq_some_iff] at h
    obtain ⟨g, hg, -⟩ := h
    obtain ⟨g', hg', hfun⟩ := hfr f g hg
    simp only [RootType, hg, hg', Option.bind_some, hfun]
  | held k => simp only [RootType, hheld]
  | global a t => simp only [RootType, hstor]

/-- Reads into a root keep resolving when the root's value is the same. -/
theorem read?_eq_of_root {s s' : State} {l : Location} (h : s'.root? l.root = s.root? l.root) :
    s'.read? l = s.read? l := by
  simp only [State.read?, h]

/-- The roots of a frame that keeps its locals, and the held values. -/
theorem root?_eq {s s' : State}
    (hfr : ∀ f g, s.frame? f = some g → ∃ g', s'.frame? f = some g' ∧ g'.locals = g.locals)
    (hheld : s'.held = s.held) (hstor : s'.storage = s.storage) {ρ : Root}
    (hs : (s.root? ρ).isSome = true) : s'.root? ρ = s.root? ρ := by
  cases ρ with
  | local_ f i =>
    simp only [State.root?, bind, Option.bind] at hs ⊢
    cases hf : s.frame? f with
    | none => simp [hf] at hs
    | some g =>
      obtain ⟨g', hg', hloc⟩ := hfr f g hf
      simp [hg', hloc]
  | held k => simp only [State.root?, hheld]
  | global a t => simp only [State.root?, hstor]

/-! ## Steps on values -/

theorem reverse_drop_getElem? {α : Type} (l : List α) (j q : Nat) (hq : q < (l.drop j).length) :
    (l.drop j).reverse[q]? = l.reverse[q]? := by
  conv => rhs; rw [← List.take_append_drop j l]
  rw [List.reverse_append, List.getElem?_append_left (by simpa using hq)]

/-- **A step on values**: the analysis pops `j` values and pushes `k`, and the
frame pops values (none of them references) and pushes some, leaving its
locals alone. Operand slots are counted from the bottom, so the counts need not
match. -/
theorem Desc.valueStep {A A' : RefState} {fr fr' : Frame} {gsid : Nat → Option StructId}
    {ps : List Reference} {L : Nat → Option Reference} {j k : Nat} (hd : Desc A fr gsid ps L)
    (hvs : ValueStep j k A A') (hid : fr'.id = fr.id) (hloc : fr'.locals = fr.locals)
    {new : List Value} {j' : Nat} (hstk : fr'.stack = new ++ fr.stack.drop j')
    (hpop : ∀ v ∈ fr.stack.take j', ∀ r, v ≠ .ref r) :
    Desc A' fr' gsid ps L := by
  obtain ⟨hj, hv, rfl⟩ := hvs
  have hheld : heldIds { A with stack := List.replicate k .nonReference ++ A.stack.drop j } =
      heldIds A := by
    simp only [heldIds, List.filterMap_append, filterMap_refId_replicate, List.nil_append,
      filterMap_refId_drop hv]
  refine ⟨fun i x hi => by rw [hloc]; exact hd.locals i x hi, fun q x hq => ?_, ?_⟩
  · simp only [List.reverse_append, List.reverse_replicate] at hq
    by_cases hlt : q < (A.stack.drop j).length
    · rw [List.getElem?_append_left (by simpa using hlt), reverse_drop_getElem? _ _ _ hlt] at hq
      obtain ⟨r, hr, hL⟩ := hd.stack q x hq
      refine ⟨r, ?_, hL⟩
      rw [hstk, List.reverse_append]
      by_cases hlt' : q < (fr.stack.drop j').length
      · rw [List.getElem?_append_left (by simpa using hlt'), reverse_drop_getElem? _ _ _ hlt']
        exact hr
      · -- The frame popped it: but it popped no reference.
        exfalso
        rw [← List.take_append_drop j' fr.stack, List.reverse_append,
          List.getElem?_append_right (by simpa using Nat.le_of_not_lt hlt')] at hr
        exact hpop _ (List.mem_reverse.1 (List.mem_of_getElem? hr)) r rfl
    · rw [List.getElem?_append_right (by simpa using Nat.le_of_not_lt hlt)] at hq
      simp [List.getElem?_replicate] at hq
  · rw [hid]
    exact hd.complete.congrH fun x => by rw [hheld]

end MoveBytecode.Proofs
