-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostSemantics

/-!
# The ghost layer follows every step of the deterministic core

UNTRUSTED (checked by the kernel). For a step of the deterministic core that
continues, from a state whose shadow fits it, the ghost layer's annotation
exists and fits the step's outcome (`ShadowAccounts`), and the step has an
`Effect` on the occurrences, consumes only legitimately, creates and takes
apart only as D3 allows, and keeps the state invariant (`StepGhost`).

**The state invariant** (`StateInv`, `Key.Ok`). The ghost layer finds a
frame, a resource and a held value as the state does only while frames and
resources are unique; global storage holds each resource at its type; and
the values the transaction holds are dropped when the transaction returns, so
they must have `drop`. They do: they are signers, and nothing writes them,
because the transaction lends them by immutable reference and no mutable
reference into them is ever made (`Key.Ok`). A reference to a resource as a
whole has the resource's type, so writing through it keeps the storage
typed.

The patterns here cover steps that change only the current frame
(`FrameGhost`, lifted by `lift_frame`), calls and returns; writes through
references and global storage are in `Proofs.GhostWrites`.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Step

/-! ## Reading a step's result -/

/-- The step result of a partial step, as `step` reads it. -/
def stepOf : Exec StepResult → StepResult
  | .ok r => r
  | .error o => .done o

theorem done_bind {α : Type} {x : Exec α} {g : α → Exec StepResult} {o : Outcome}
    (h : stepOf (x >>= g) = .done o) (hcont : o.continues = true) (hx : Ends x) :
    ∃ a, x = .ok a ∧ stepOf (g a) = .done o := by
  cases hx' : x with
  | error e =>
    rw [hx'] at h hx
    simp only [bind, Except.bind, stepOf, StepResult.done.injEq] at h
    subst h
    simp [Ends] at hx
    rw [hx] at hcont
    exact absurd hcont (by simp)
  | ok a =>
    rw [hx'] at h
    exact ⟨a, rfl, h⟩

theorem done_wrong {o : Outcome} {r : WrongReason} (h : stepOf (wrong r) = .done o)
    (hcont : o.continues = true) : False := by
  simp only [stepOf, wrong, StepResult.done.injEq] at h
  subst h
  simp [Outcome.continues] at hcont

theorem done_fail {o : Outcome} {f : Failure} (h : stepOf (fail f) = .done o)
    (hcont : o.continues = true) : False := by
  simp only [stepOf, fail, StepResult.done.injEq] at h
  subst h
  simp [Outcome.continues] at hcont

theorem done_next {o : Outcome} {s : State} {rest : List Frame} {fr : Frame}
    (h : stepOf (next s rest fr) = .done o) : o = .running { s with frames := fr :: rest } := by
  simp only [stepOf, next, pure, Except.pure, StepResult.done.injEq] at h
  exact h.symm

/-! ## The state invariant -/

/-- Whether a root is a value the transaction holds. -/
def _root_.MoveBytecode.Root.isHeld : Root → Bool
  | .held _ => true
  | .local_ .. => false
  | .global .. => false

/-- What the trace invariant requires of a key: a reference to a resource as a
whole has the resource's type, and, for transactions (`σ`), a reference into a
value the transaction holds is immutable. -/
def Key.Ok (σ : Bool) : Key → Prop
  | .leaf (.ref r) => (σ = true → r.location.root.isHeld = true → r.mutable = false) ∧
      ∀ a t, r.location = ⟨.global a t, []⟩ → r.target = .struct t
  | .leaf (.bool _) => True
  | .leaf (.int _ _) => True
  | .leaf (.address _) => True
  | .leaf (.signer _) => True
  | .leaf (.vector _ _) => True
  | .leaf (.struct _ _) => True
  | .leaf (.variant _ _ _) => True
  | .leaf (.closure ..) => True
  | .leaf (.table ..) => True
  | .node _ => True

/-- The facts about a state the ghost layer needs (module docstring). The
values a transaction holds have `drop` (`σ`, for transactions; any admissible
initial state may hold others). -/
structure StateInv (σ : Bool) (P : Program) (s : State) : Prop where
  frameIds : (s.frames.map (·.id)).Nodup
  framesBelow : ∀ fr ∈ s.frames, fr.id < s.nextFrame
  resources : (s.storage.map fun r => (r.address, r.type)).Nodup
  typed : ∀ r ∈ s.storage, r.value.type = .struct r.type
  heldDrop : σ = true → ∀ v ∈ s.held, (v.abilities P).drop = true

/-- The state invariant after a step that continues. -/
def PostInv (σ : Bool) (P : Program) : Outcome → Prop
  | .running s => StateInv σ P s
  | .returned _ _ => True
  | .failed _ => True
  | .wrong _ => True

/-- A reference in a frame whose location is valid, to a target type without
subtypes (`rigid`), designates a value of that type (`Proofs.GhostTyping`). -/
def RefsTyped (s : State) : Prop :=
  ∀ fr ∈ s.frames, ∀ r : Reference,
    (Value.ref r ∈ fr.stack ∨ some (Value.ref r) ∈ fr.locals.toList) →
    ∀ v, s.deref? r.location r.since = some v → rigid r.target = true → v.type = r.target

/-- A step from `s` with outcome `o` that ends the transaction discards the
values it holds, so they must have `drop` (D2): for transactions they do
(`StateInv.heldDrop`); an admissible initial state may hold others. -/
def RetOk (P : Program) (s : State) (o : Outcome) : Prop :=
  ∀ vs st, o = .returned vs st → ∀ v ∈ s.held, (v.abilities P).drop = true

/-- References occur only at the top of the frames' slots: the frames'
references designate values of value types, which hold none, and global
storage holds none (`Proofs.GhostTyping` derives this from the typing
invariant). -/
structure RefsTop (s : State) : Prop where
  targets : ∀ fr ∈ s.frames, ∀ r : Reference,
    (Value.ref r ∈ fr.stack ∨ some (Value.ref r) ∈ fr.locals.toList) → r.target.isValue = true
  storage : ∀ r ∈ s.storage, ∀ w ∈ subvalues r.value, w.isRef = false
  deref : ∀ fr ∈ s.frames, ∀ r : Reference,
    (Value.ref r ∈ fr.stack ∨ some (Value.ref r) ∈ fr.locals.toList) →
    ∀ v, s.read? r.location = some v → ∀ w ∈ subvalues v, w.isRef = false

/-- A closure whose mask captures, among the parameters it has captured and
remaining values for, as many as it holds captured values: calling it passes
each captured and each provided value (`maskCompose`). A table holds entries
only where the program's tables may hold them (`Program.tablesHoldEntries`), whose
module unpacks no table (`unpack_noEntries`). -/
def Composes (P : Program) : Value → Prop
  | .closure _ mask cs ps _ _ =>
    ((List.range (cs.length + ps.length)).map mask.getLsbD).count true = cs.length
  | .table s _ _ es => es ≠ [] → P.tablesHoldEntries = true ∧ s.isTable = true
  | _ => True

/-- What the annotation of a step from `s`, whose shadow is `h`, assumes. -/
structure Pre (σ : Bool) (P : Program) (s : State) (h : Shadow) : Prop where
  fits : h.fits s = true
  inv : StateInv σ P s
  keysOk : ∀ x ∈ keys (framesOcc h.frames s.frames), x.2.Ok σ
  typed : RefsTyped s
  refsTop : RefsTop s
  composes : ∀ fr ∈ s.frames, ∀ v ∈ fr.stack, Composes P v

/-! ## What a step's annotation establishes -/

/-- The fresh counter after a step's shadow. -/
def postFresh (f : Nat) : ShadowPost → Nat
  | .running h => h.fresh
  | .returned _ _ => f

/-- The occurrences in the frames after a step that keeps running. -/
def postFrames : ShadowPost → Outcome → Occ
  | .running h, .running s => framesOcc h.frames s.frames
  | .running _, .returned _ _ | .running _, .failed _ | .running _, .wrong _ => []
  | .returned _ _, .running _ | .returned _ _, .returned _ _ | .returned _ _, .failed _
  | .returned _ _, .wrong _ => []

/-- The frames after a step that keeps running, each with its shadow. -/
def postPairs : ShadowPost → Outcome → List (ShadowFrame × Frame)
  | .running h, .running s => h.frames.zip s.frames
  | .running _, .returned _ _ | .running _, .failed _ | .running _, .wrong _ => []
  | .returned _ _, .running _ | .returned _ _, .returned _ _ | .returned _ _, .failed _
  | .returned _ _, .wrong _ => []

/-- **Per frame** (D4), for a step that keeps the number of frames `preP`,
each with its shadow: every reference in a frame after the step was in that
frame before, or, in the current frame, the step borrowed it. -/
def RefsPerFrame (preP : List (ShadowFrame × Frame)) (post : ShadowPost) (o : Outcome)
    (evs : List Event) : Prop :=
  (postPairs post o).length = preP.length → ∀ k g' fr' g fr,
    (postPairs post o)[k]? = some (g', fr') → preP[k]? = some (g, fr) →
    ∀ d r, (d, Value.ref r) ∈ g'.occurrences fr' →
      (d, Value.ref r) ∈ g.occurrences fr ∨ (k = 0 ∧ ∃ par, Event.borrowed d r par ∈ evs)

/-- Whether a value is a closure. -/
def IsClosure : Value → Bool
  | .closure .. => true
  | _ => false

/-- Every closure occurrence of `post` is one of `pre`, or created by `evs`:
a closure node keeps its value (D3c). -/
def ClosuresKept (pre post : Occ) (evs : List Event) : Prop :=
  ∀ x ∈ post, IsClosure x.2 = true → x ∈ pre ∨ ∃ c, Event.created x.1 x.2 c ∈ evs

theorem ClosuresKept.of_sub {pre post : Occ} {evs : List Event}
    (h : ∀ x ∈ post, IsClosure x.2 = true → x ∈ pre) : ClosuresKept pre post evs :=
  fun x hx hc => Or.inl (h x hx hc)

theorem ClosuresKept.mono {pre pre' post : Occ} {evs evs' : List Event}
    (h : ClosuresKept pre post evs) (hp : ∀ x ∈ pre, x ∈ pre')
    (he : ∀ e ∈ evs, e ∈ evs') : ClosuresKept pre' post evs' := by
  intro x hx hc
  rcases h x hx hc with h | ⟨c, hc'⟩
  · exact Or.inl (hp x h)
  · exact Or.inr ⟨c, he _ hc'⟩

theorem ClosuresKept.append {pre post : Occ} {evs : List Event} (h : ClosuresKept pre post evs)
    (A B : Occ) : ClosuresKept (A ++ pre ++ B) (A ++ post ++ B) evs := by
  intro x hx hc
  simp only [List.mem_append] at hx
  rcases hx with (hx | hx) | hx
  · exact Or.inl (by simp [hx])
  · rcases h x hx hc with h | h
    · exact Or.inl (by simp [h])
    · exact Or.inr h
  · exact Or.inl (by simp [hx])

theorem ClosuresKept.perm {pre pre' post post' : Occ} {evs : List Event}
    (h : ClosuresKept pre post evs) (hp : pre.Perm pre') (hq : post'.Perm post) :
    ClosuresKept pre' post' evs := by
  intro x hx hc
  rcases h x (hq.subset hx) hc with h | h
  · exact Or.inl (hp.subset h)
  · exact Or.inr h

/-- What the annotation of a step from occurrences `pre`, of which `preF` are
in the frames `preP`, fresh counter `f`, with outcome `o`, establishes. -/
structure StepGhost (σ : Bool) (P : Program) (π : NativePolicy) (A : List StructId) (auth : List Address)
    (pre preF : Occ) (preP : List (ShadowFrame × Frame)) (f : Nat) (o : Outcome) (post : ShadowPost)
    (evs : List Event) : Prop where
  fits : post.fits o = true
  effect : Effect P pre (post.occurrences o) evs f (postFresh f post)
  legit : ∀ i w c, Event.consumed i w c ∈ evs →
    (w.type.abilities P).drop = true ∨ c.Legitimate π w = true
  respected : ∀ e ∈ evs, Encapsulation.structsRespected π A e = true ∧
    Encapsulation.signersRespected π auth e = true
  issuedOk : ∀ x ∈ issuedOf evs, x.2.Ok σ
  inv : PostInv σ P o
  /-- A step creates no reference by `created`: every reference is created by
  a `borrowed` event, which D4 follows. -/
  createdPlain : ∀ i w c, Event.created i w c ∈ evs → w.isRef = false
  /-- Every reference in the frames after the step was in them before, or the
  step borrowed it (D4). -/
  frameRefs : ∀ d r, (d, Value.ref r) ∈ postFrames post o →
    (d, Value.ref r) ∈ preF ∨ ∃ par, Event.borrowed d r par ∈ evs
  /-- The same, per frame, for a step that keeps the number of frames (D4). -/
  refsAt : RefsPerFrame preP post o evs
  /-- A closure is created only by a creator that may create it (D3c). -/
  closures : ∀ e ∈ evs, Encapsulation.closuresRespected P e = true
  /-- A closure node keeps its value (D3c). -/
  closuresKept : ClosuresKept pre (post.occurrences o) evs

/-- A step that changes the number of frames says nothing per frame. -/
theorem refsPerFrame_of_ne {preP : List (ShadowFrame × Frame)} {post : ShadowPost} {o : Outcome}
    {evs : List Event} (h : (postPairs post o).length ≠ preP.length) :
    RefsPerFrame preP post o evs :=
  fun e => absurd e h

/-- Per frame, from the facts at each position. -/
theorem refsPerFrame_of {h' : Shadow} {s' : State} {hs₀ : List ShadowFrame} {frs₀ : List Frame}
    {evs : List Event}
    (hk : ∀ k g' fr' g fr, h'.frames[k]? = some g' → s'.frames[k]? = some fr' →
      hs₀[k]? = some g → frs₀[k]? = some fr → ∀ d r, (d, Value.ref r) ∈ g'.occurrences fr' →
        (d, Value.ref r) ∈ g.occurrences fr ∨ (k = 0 ∧ ∃ par, Event.borrowed d r par ∈ evs)) :
    RefsPerFrame (hs₀.zip frs₀) (.running h') (.running s') evs := by
  intro _ k g' fr' g fr h1 h0 d r hm
  simp only [postPairs, List.getElem?_zip_eq_some] at h1 h0
  exact hk k g' fr' g fr h1.1 h1.2 h0.1 h0.2 d r hm

/-- Per frame, for a step that changes the current frame only. -/
theorem refsPerFrame_top {g g' : ShadowFrame} {fr fr' : Frame} {hs : List ShadowFrame}
    {rest : List Frame} {h' : Shadow} {s' : State} {evs : List Event}
    (hh' : h'.frames = g' :: hs) (hs' : s'.frames = fr' :: rest)
    (htop : ∀ d r, (d, Value.ref r) ∈ g'.occurrences fr' →
      (d, Value.ref r) ∈ g.occurrences fr ∨ ∃ par, Event.borrowed d r par ∈ evs) :
    RefsPerFrame ((g :: hs).zip (fr :: rest)) (.running h') (.running s') evs := by
  refine refsPerFrame_of fun k g₁ fr₁ g₀ fr₀ h1 h2 h3 h4 d r hm => ?_
  rw [hh'] at h1
  rw [hs'] at h2
  cases k with
  | zero =>
    simp only [List.getElem?_cons_zero, Option.some.injEq] at h1 h2 h3 h4
    subst h1 h2 h3 h4
    rcases htop d r hm with h | h
    · exact Or.inl h
    · exact Or.inr ⟨rfl, h⟩
  | succ k =>
    simp only [List.getElem?_cons_succ] at h1 h2 h3 h4
    rw [h1, Option.some.injEq] at h3
    rw [h2, Option.some.injEq] at h4
    subst h3 h4
    exact Or.inl hm

/-- What the annotation of a step that changes only the current frame, from
`g` fitting `fr` to `g'` fitting `fr'`, establishes of that frame. -/
structure FrameGhost (σ : Bool) (P : Program) (π : NativePolicy) (A : List StructId) (auth : List Address)
    (g : ShadowFrame) (fr : Frame) (f : Nat) (g' : ShadowFrame) (fr' : Frame) (f' : Nat)
    (evs : List Event) : Prop where
  fits : g'.fits fr' = true
  effect : Effect P (g.occurrences fr) (g'.occurrences fr') evs f f'
  legit : ∀ i w c, Event.consumed i w c ∈ evs →
    (w.type.abilities P).drop = true ∨ c.Legitimate π w = true
  respected : ∀ e ∈ evs, Encapsulation.structsRespected π A e = true ∧
    Encapsulation.signersRespected π auth e = true
  issuedOk : ∀ x ∈ issuedOf evs, x.2.Ok σ
  createdPlain : ∀ i w c, Event.created i w c ∈ evs → w.isRef = false
  closures : ∀ e ∈ evs, Encapsulation.closuresRespected P e = true
  closuresKept : ClosuresKept (g.occurrences fr) (g'.occurrences fr') evs

/-- Silent events create no closure. -/
theorem closures_silent {P : Program} {evs : List Event} (h : ∀ e ∈ evs, e.Silent = true) :
    ∀ e ∈ evs, Encapsulation.closuresRespected P e = true := by
  intro e he
  have := h e he
  cases e <;> simp_all [Event.Silent, Encapsulation.closuresRespected]

/-- Creating a value that is no closure respects D3c. -/
theorem closures_created {P : Program} {i : Nat} {w : Value} {c : Creator}
    (hw : ∀ f mask cs ps rs a, w ≠ .closure f mask cs ps rs a) :
    Encapsulation.closuresRespected P (.created i w c) = true := by
  cases w with
  | closure f mask cs ps rs a => exact absurd rfl (hw f mask cs ps rs a)
  | _ => rfl

theorem consumed_mem {evs : List Event} {i : Nat} {w : Value} {c : Consumer}
    (h : Event.consumed i w c ∈ evs) : (i, w) ∈ consumedOf evs :=
  (mem_consumedOf (o := (i, w))).2 ⟨c, h⟩

/-- An effect, given the step's issued and consumed keys. -/
theorem EffectK.mk' {P : Program} {pre post : KOcc} {evs : List Event} {f f' : Nat}
    (keep dropped copies I C : KOcc) (hI : issuedOf evs = I) (hC : keys (consumedOf evs) = C)
    (hpre : pre.Perm (keep ++ dropped ++ C)) (hpost : post.Perm (keep ++ copies ++ I))
    (hd : ∀ x ∈ dropped, (x.2.type.abilities P).drop = true)
    (hc : ∀ x ∈ copies, x ∈ keep ∧ (x.2.type.abilities P).copy = true)
    (hb : ∀ x ∈ I, f ≤ x.1 ∧ x.1 < f') (hnd : (I.map (·.1)).Nodup) (hff : f ≤ f') :
    EffectK P pre post evs f f' := by
  subst hI hC
  exact ⟨keep, dropped, copies, hpre, hpost, hd, hc, hb, hnd, hff⟩

/-- Silent events added to an effect of none. -/
theorem EffectK.of_silent {P : Program} {pre post : KOcc} {evs : List Event} {f f' : Nat}
    (he : EffectK P pre post [] f f') (h : ∀ e ∈ evs, e.Silent = true) :
    EffectK P pre post evs f f' := by
  simpa using he.silent h (es₂ := []) (fun _ h => by simp at h)

/-- A value whose key is a reference's is that reference. -/
theorem key_ref {w : Value} {r : Reference} (h : w.key = (Value.ref r).key) : w = .ref r := by
  cases w <;> simp_all [Value.key]

/-- **The references after an effect** were there before, or are borrowed by
its events, when those create no reference. -/
theorem effect_refs {P : Program} {pre post : Occ} {evs : List Event} {f f' : Nat}
    (he : Effect P pre post evs f f') (hcp : ∀ i w c, Event.created i w c ∈ evs → w.isRef = false) :
    ∀ d r, (d, Value.ref r) ∈ post → (d, Value.ref r) ∈ pre ∨ ∃ par, Event.borrowed d r par ∈ evs := by
  intro d r hm
  rcases he.sub (mem_keys hm) with hk | hk
  · obtain ⟨o, ho, h1, h2⟩ := mem_keys_iff.1 hk
    obtain ⟨o1, o2⟩ := o
    simp only at h1 h2
    subst h1
    rw [key_ref h2] at ho
    exact Or.inl ho
  · rcases mem_issuedOf hk with ⟨w, c, hc, hwk⟩ | ⟨r', p, hb, hrk⟩
    · have := hcp _ _ _ hc
      rw [key_ref hwk] at this
      simp [Value.isRef] at this
    · simp only at hrk hb
      have h' := key_ref hrk
      simp only [Value.ref.injEq] at h'
      subst h'
      exact Or.inr ⟨p, hb⟩

/-- Events that create nothing create no reference. -/
theorem createdPlain_of {evs : List Event} (h : createdOf evs = []) :
    ∀ i w c, Event.created i w c ∈ evs → w.isRef = false :=
  fun i w _ he => absurd ((mem_createdOf (o := (i, w))).2 ⟨_, he⟩) (by simp [h])

section Lift

variable {P : Program} {π : NativePolicy} {A : List StructId} {auth : List Address}

/-- Silent events that respect D3 change nothing a step establishes. -/
theorem FrameGhost.silent {g g' : ShadowFrame} {fr fr' : Frame} {f f' : Nat}
    {evs es₁ es₂ : List Event} (hG : FrameGhost σ P π A auth g fr f g' fr' f' evs)
    (h₁ : ∀ e ∈ es₁, e.Silent = true ∧ Encapsulation.structsRespected π A e = true)
    (h₂ : ∀ e ∈ es₂, e.Silent = true ∧ Encapsulation.structsRespected π A e = true) :
    FrameGhost σ P π A auth g fr f g' fr' f' (es₁ ++ evs ++ es₂) := by
  have hs₁ := silent_of fun e he => (h₁ e he).1
  have hs₂ := silent_of fun e he => (h₂ e he).1
  have hsig : ∀ e, e.Silent = true → Encapsulation.signersRespected π auth e = true := by
    intro e he; cases e <;> simp_all [Event.Silent, Encapsulation.signersRespected]
  refine ⟨hG.fits, EffectK.silent hG.effect (fun e he => (h₁ e he).1) (fun e he => (h₂ e he).1),
    fun i w c he => ?_, fun e he => ?_, fun x hx => ?_, fun i w c he => ?_, fun e he => ?_,
    hG.closuresKept.mono (fun _ h => h) (fun e he => by simp [he])⟩
  · rcases List.mem_append.1 he with he | he
    · rcases List.mem_append.1 he with he | he
      · exact absurd (consumed_mem he) (by simp [hs₁.2.1])
      · exact hG.legit i w c he
    · exact absurd (consumed_mem he) (by simp [hs₂.2.1])
  · rcases List.mem_append.1 he with he | he
    · rcases List.mem_append.1 he with he | he
      · exact ⟨(h₁ e he).2, hsig e (h₁ e he).1⟩
      · exact hG.respected e he
    · exact ⟨(h₂ e he).2, hsig e (h₂ e he).1⟩
  · simp only [issuedOf_append, hs₁.2.2, hs₂.2.2, List.nil_append, List.append_nil] at hx
    exact hG.issuedOk x hx
  · rcases List.mem_append.1 he with he | he
    · rcases List.mem_append.1 he with he | he
      · exact absurd ((mem_createdOf (o := (i, w))).2 ⟨c, he⟩) (by simp [hs₁.1])
      · exact hG.createdPlain i w c he
    · exact absurd ((mem_createdOf (o := (i, w))).2 ⟨c, he⟩) (by simp [hs₂.1])
  · rcases List.mem_append.1 he with he | he
    · rcases List.mem_append.1 he with he | he
      · exact closures_silent (fun e he => (h₁ e he).1) e he
      · exact hG.closures e he
    · exact closures_silent (fun e he => (h₂ e he).1) e he

theorem frames_inv {s s₂ : State} {fr fr' : Frame} {rest : List Frame} (hinv : StateInv σ P s)
    (hsf : s.frames = fr :: rest) (hst : s₂.storage = s.storage) (hhe : s₂.held = s.held)
    (hnf : s₂.nextFrame = s.nextFrame) (hid : fr'.id = fr.id) :
    StateInv σ P { s₂ with frames := fr' :: rest } where
  frameIds := by
    have := hinv.frameIds
    rw [hsf] at this
    simpa [hid] using this
  framesBelow := by
    intro x hx
    simp only [List.mem_cons] at hx
    rw [hnf]
    rcases hx with rfl | hx
    · rw [hid]; exact hinv.framesBelow fr (by rw [hsf]; exact List.mem_cons_self ..)
    · exact hinv.framesBelow x (by rw [hsf]; exact List.mem_cons_of_mem _ hx)
  resources := by rw [hst]; exact hinv.resources
  typed := by rw [hst]; exact hinv.typed
  heldDrop := by rw [hhe]; exact hinv.heldDrop

/-- A step that changes only the current frame: its frame's ghost facts give
the step's. -/
theorem lift_frame {s s₂ : State} {h : Shadow} {fr fr' : Frame} {rest : List Frame}
    {g g' : ShadowFrame} {hs : List ShadowFrame} {f' : Nat} {evs : List Event}
    (hp : Pre σ P s h) (hsf : s.frames = fr :: rest) (hhf : h.frames = g :: hs)
    (hst : s₂.storage = s.storage) (hhe : s₂.held = s.held) (hnf : s₂.nextFrame = s.nextFrame)
    (hid : fr'.id = fr.id) (hG : FrameGhost σ P π A auth g fr h.fresh g' fr' f' evs) :
    StepGhost σ P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh (.running { s₂ with frames := fr' :: rest })
      (.running { h with frames := g' :: hs, fresh := f' }) evs := by
  have hfit := hp.fits
  rw [shadow_fits, hsf, hhf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
  refine ⟨?_, ?_, hG.legit, hG.respected, hG.issuedOk, frames_inv hp.inv hsf hst hhe hnf hid,
    hG.createdPlain, ?_, ?_, hG.closures, by
      show ClosuresKept _ (Shadow.occurrences _ _) _
      have hx : extOcc { h with frames := g' :: hs, fresh := f' } { s₂ with frames := fr' :: rest } =
          extOcc h s := extOcc_congr rfl rfl hst hhe
      rw [shadow_occurrences, shadow_occurrences, hsf, hhf, framesOcc_cons, framesOcc_cons, hx]
      simpa [List.append_assoc] using hG.closuresKept.append [] (framesOcc hs rest ++ extOcc h s)⟩
  rotate_right
  · rw [hhf, hsf]
    exact refsPerFrame_top rfl rfl (effect_refs hG.effect hG.createdPlain)
  · simp only [ShadowPost.fits, shadow_fits, framesFit_cons, hG.fits, hfit.1.2, Bool.and_self,
      Bool.true_and]
    exact (extFit_congr (h := h) (s := s) rfl rfl hst hhe).trans hfit.2
  · show Effect P _ (Shadow.occurrences _ _) _ _ f'
    have hx : extOcc { h with frames := g' :: hs, fresh := f' } { s₂ with frames := fr' :: rest } =
        extOcc h s := extOcc_congr rfl rfl hst hhe
    rw [shadow_occurrences, shadow_occurrences, hsf, hhf, framesOcc_cons, framesOcc_cons, hx]
    simpa [List.append_assoc] using hG.effect.append (framesOcc hs rest ++ extOcc h s)
  · intro d r hm
    simp only [postFrames, framesOcc_cons, List.mem_append] at hm
    rw [hhf, hsf, framesOcc_cons, List.mem_append]
    rcases hm with hm | hm
    · rcases effect_refs hG.effect hG.createdPlain d r hm with hm | hm
      · exact Or.inl (Or.inl hm)
      · exact Or.inr hm
    · exact Or.inl (Or.inr hm)

end Lift

/-! ## Effects on a frame -/

section Frames

variable {P : Program} {f f' : Nat} {evs : List Event}

/-- A change of the operand stack alone. -/
theorem effect_stack {g : ShadowFrame} {fr : Frame} {ts : List IdTree} {vs : List Value}
    {pc : Nat} (he : Effect P (IdTree.occurrencesAll g.stack fr.stack) (IdTree.occurrencesAll ts vs)
      evs f f') :
    Effect P (g.occurrences fr)
      ({ g with stack := ts }.occurrences { fr with stack := vs, pc := pc })
      evs f f' := by
  rw [frame_occurrences, frame_occurrences, localsOcc_stack]
  exact he.prepend _

/-- Every node of a value with `drop` has a type with `drop`. -/
theorem occ_drop {t : IdTree} {v : Value} (ht : t.fits v = true)
    (hd : (v.abilities P).drop = true) :
    ∀ o ∈ t.occurrences v, (o.2.type.abilities P).drop = true :=
  fun o ho => deep_drop v hd o.2 (occurrences_mem ht ho)

/-- Discarding the top of the stack. -/
theorem effect_discard {t : IdTree} {ts : List IdTree} {v : Value} {vs : List Value}
    (ht : t.fits v = true) (hd : (v.abilities P).drop = true) :
    Effect P (IdTree.occurrencesAll (t :: ts) (v :: vs)) (IdTree.occurrencesAll ts vs) [] f f :=
  Effect.drop (occ_drop ht hd) _ (Nat.le_refl f)

end Frames

/-! ## Values D3 does not restrict -/

/-- Booleans, integers, addresses and vectors: values any code may create,
and that are neither signers nor references. -/
def IsPlain : Value → Bool
  | .bool _ | .int _ _ | .address _ | .vector _ _ => true
  | .signer _ | .struct _ _ | .variant _ _ _ | .ref _ | .closure .. | .table .. => false

theorem plain_respected {π : NativePolicy} {A : List StructId} {auth : List Address} {i : Nat}
    {w : Value} {c : Creator} (hw : IsPlain w = true) :
    Encapsulation.structsRespected π A (.created i w c) = true ∧
      Encapsulation.signersRespected π auth (.created i w c) = true := by
  cases w <;> simp_all [IsPlain, Encapsulation.structsRespected, Encapsulation.signersRespected,
    Value.type]

theorem plain_ok {w : Value} (hw : IsPlain w = true) : w.key.Ok σ := by
  cases w <;> simp_all [IsPlain, Value.key, Key.Ok]

/-- A container's key is a type, which every key invariant allows. -/
theorem container_ok {w : Value} (hw : ∀ r, w ≠ .ref r) : w.key.Ok σ := by
  cases w <;> simp_all [Value.key, Key.Ok]

theorem decodeSeq_mem {α : Type} {dec : List UInt8 → Option (α × List UInt8)} :
    ∀ {n : Nat} {d r : List UInt8} {as : List α}, decodeSeq dec n d = some (as, r) →
      ∀ a ∈ as, ∃ d' r', dec d' = some (a, r')
  | 0, d, r, as, h, a, ha => by
    simp only [decodeSeq, Option.some.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, -⟩ := h
    simp at ha
  | n + 1, d, r, as, h, a, ha => by
    simp only [decodeSeq, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨⟨a', d₁⟩, h1, ⟨as', d₂⟩, h2, h⟩ := h
    simp only [Prod.mk.injEq] at h
    obtain ⟨rfl, -⟩ := h
    rcases List.mem_cons.1 ha with rfl | ha
    · exact ⟨d, d₁, h1⟩
    · exact decodeSeq_mem h2 a ha

/-- A decoded value is plain all the way down: constants are booleans,
integers, addresses and vectors of them. -/
theorem bcsDecode_plain : ∀ (t : RuntimeType) {d r : List UInt8} {v : Value},
    bcsDecode t d = some (v, r) → ∀ w ∈ subvalues v, IsPlain w = true
  | .vector elem, d, r, v, h, w, hw => by
    simp only [bcsDecode, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨⟨n, d₁⟩, -, ⟨elems, d₂⟩, h2, h⟩ := h
    simp only [Prod.mk.injEq] at h
    obtain ⟨rfl, -⟩ := h
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · rfl
    · obtain ⟨e, he, hwe⟩ := List.mem_flatMap.1 hw
      obtain ⟨d', r', hd'⟩ := decodeSeq_mem h2 e he
      exact bcsDecode_plain elem hd' w hwe
  | .bool, d, r, v, h, w, hw => by
    unfold bcsDecode at h
    split at h
    · split at h
      · simp only [Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, -⟩ := h
        simp [subvalues_eq, Value.children] at hw; subst hw; rfl
      · split at h
        · simp only [Option.some.injEq, Prod.mk.injEq] at h
          obtain ⟨rfl, -⟩ := h
          simp [subvalues_eq, Value.children] at hw; subst hw; rfl
        · simp at h
    · simp at h
  | .int it, d, r, v, h, w, hw => by
    unfold bcsDecode at h
    simp only [] at h
    split at h
    · simp at h
    · simp only [Option.some.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, -⟩ := h
      simp [subvalues_eq, Value.children] at hw; subst hw; rfl
  | .address, d, r, v, h, w, hw => by
    unfold bcsDecode at h
    split at h
    · simp at h
    · simp only [Option.some.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, -⟩ := h
      simp [subvalues_eq, Value.children] at hw; subst hw; rfl
  | .signer, _, _, _, h, _, _ => by simp [bcsDecode] at h
  | .struct _, _, _, _, h, _, _ => by simp [bcsDecode] at h
  | .ref _ _, _, _, _, h, _, _ => by simp [bcsDecode] at h

theorem decodeConstant_plain {t : RuntimeType} {d : List UInt8} {v : Value}
    (h : decodeConstant t d = some v) : ∀ w ∈ subvalues v, IsPlain w = true := by
  unfold decodeConstant at h
  split at h
  · next v' heq => simp only [Option.some.injEq] at h; subst h; exact bcsDecode_plain t heq
  · simp at h
  · simp at h

/-! ## Frame shapes -/

section Patterns

variable {P : Program} {π : NativePolicy} {A : List StructId} {auth : List Address}
  {fr : Frame} {g : ShadowFrame} {f : Nat}

theorem fitsAll_cons_right {ts : List IdTree} {v : Value} {vs : List Value}
    (h : IdTree.fitsAll ts (v :: vs) = true) :
    ∃ t ts', ts = t :: ts' ∧ t.fits v = true ∧ IdTree.fitsAll ts' vs = true := by
  cases ts with
  | nil => simp [IdTree.fitsAll] at h
  | cons t ts' => simp only [IdTree.fitsAll, Bool.and_eq_true] at h; exact ⟨t, ts', rfl, h⟩

theorem fitsAll_take_drop : ∀ {ts : List IdTree} {vs : List Value} (n : Nat),
    IdTree.fitsAll ts vs = true →
      IdTree.fitsAll (ts.take n) (vs.take n) = true ∧
        IdTree.fitsAll (ts.drop n) (vs.drop n) = true
  | [], [], n, _ => by simp [IdTree.fitsAll]
  | _ :: _, _ :: _, 0, h => by simpa [IdTree.fitsAll] using h
  | t :: ts, v :: vs, n + 1, h => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp only [List.take_succ_cons, List.drop_succ_cons, IdTree.fitsAll, Bool.and_eq_true]
    exact ⟨⟨h.1, (fitsAll_take_drop n h.2).1⟩, (fitsAll_take_drop n h.2).2⟩
  | [], _ :: _, _, h => by simp [IdTree.fitsAll] at h
  | _ :: _, [], _, h => by simp [IdTree.fitsAll] at h

theorem frame_stack_fits (hg : g.fits fr = true) : IdTree.fitsAll g.stack fr.stack = true := by
  rw [frame_fits] at hg; simp only [Bool.and_eq_true] at hg; exact hg.2

theorem frame_fits_stack (hg : g.fits fr = true) {ts : List IdTree} {vs : List Value}
    (h : IdTree.fitsAll ts vs = true) (pc : Nat) :
    ({ g with stack := ts } : ShadowFrame).fits { fr with stack := vs, pc := pc } = true := by
  rw [frame_fits] at hg ⊢
  simp only [Bool.and_eq_true] at hg ⊢
  exact ⟨hg.1, h⟩

theorem stack_split {ts : List IdTree} {vs : List Value} (h : IdTree.fitsAll ts vs = true)
    (n : Nat) :
    IdTree.occurrencesAll ts vs =
      IdTree.occurrencesAll (ts.take n) (vs.take n) ++
        IdTree.occurrencesAll (ts.drop n) (vs.drop n) := by
  conv => lhs; rw [← List.take_append_drop n ts, ← List.take_append_drop n vs]
  exact occurrencesAll_append (by simp [fitsAll_length h])

/-- `Pop`, `BrTrue`, `BrFalse`: the top value, which has `drop`, is
discarded. -/
theorem kept_stack {ts : List IdTree} {vs : List Value} {pc : Nat} {evs : List Event}
    (h : ClosuresKept (IdTree.occurrencesAll g.stack fr.stack) (IdTree.occurrencesAll ts vs) evs) :
    ClosuresKept (g.occurrences fr) ({ g with stack := ts }.occurrences { fr with stack := vs, pc := pc })
      evs := by
  rw [frame_occurrences, frame_occurrences, localsOcc_stack]
  have h' := h.append (localsOcc g fr) []
  simp only [List.append_nil] at h'
  exact h'

/-- A step whose frames' occurrences are among those before, and which keeps
global storage and the held values, keeps every closure. -/
theorem kept_frames {h h' : Shadow} {s s' : State} {evs : List Event}
    (hst : h'.storage = h.storage) (hhe : h'.held = h.held) (sst : s'.storage = s.storage)
    (she : s'.held = s.held)
    (hsub : ∀ x ∈ framesOcc h'.frames s'.frames, x ∈ framesOcc h.frames s.frames) :
    ClosuresKept (h.occurrences s) (ShadowPost.occurrences (.running h') (.running s')) evs := by
  refine ClosuresKept.of_sub fun x hx _ => ?_
  simp only [ShadowPost.occurrences, shadow_occurrences, List.mem_append,
    extOcc_congr hst hhe sst she] at hx ⊢
  rcases hx with hx | hx
  · exact Or.inl (hsub x hx)
  · exact Or.inr hx

theorem plain_not_closure {w : Value} (h : IsPlain w = true) : IsClosure w = false := by
  cases w <;> simp_all [IsPlain, IsClosure]

theorem discard_ghost (hg : g.fits fr = true) {v : Value} {st : List Value}
    (hst : fr.stack = v :: st) (hd : (v.abilities P).drop = true) (pc : Nat) :
    ∃ t ts, g.stack = t :: ts ∧
      FrameGhost σ P π A auth g fr f { g with stack := ts } { fr with stack := st, pc := pc } f [] := by
  have hfa := frame_stack_fits hg
  rw [hst] at hfa
  obtain ⟨t, ts, hgst, ht, hts⟩ := fitsAll_cons_right hfa
  refine ⟨t, ts, hgst, frame_fits_stack hg hts pc, ?_, by simp, by simp, by simp, by simp, by simp,
    kept_stack (ClosuresKept.of_sub fun x hx _ => by
      rw [hgst, hst]; simp only [IdTree.occurrencesAll, List.mem_append]; exact Or.inr hx)⟩
  exact effect_stack (by rw [hgst, hst]; exact effect_discard ht hd)

/-- `Branch`, `Nop`: only the program counter moves. -/
theorem keep_ghost (hg : g.fits fr = true) (pc : Nat) :
    FrameGhost σ P π A auth g fr f g { fr with pc := pc } f [] :=
  ⟨hg, Effect.keep (List.Perm.refl _) (Nat.le_refl f), by simp, by simp, by simp, by simp, by simp,
    ClosuresKept.of_sub fun _ hx _ => hx⟩

theorem createdOf_map_created (c : Creator) :
    ∀ (L : Occ), createdOf (L.map fun (i, w) => Event.created i w c) = L
  | [] => rfl
  | (i, w) :: L => by simp [createdOf_map_created c L]

theorem consumedOf_map_created (c : Creator) :
    ∀ (L : Occ), consumedOf (L.map fun (i, w) => Event.created i w c) = []
  | [] => rfl
  | (i, w) :: L => by simp [consumedOf_map_created c L]

theorem issuedOf_map_created (c : Creator) :
    ∀ (L : Occ), issuedOf (L.map fun (i, w) => Event.created i w c) = keys L
  | [] => rfl
  | (i, w) :: L => by simp [issuedOf_map_created c L]

/-- Literals, constants, casts, arithmetic, comparisons, equality and the
boolean operators: the top `n` values, which have `drop`, are discarded and
a plain value is created in their place. -/
theorem replace_ghost (hg : g.fits fr = true) {n : Nat} {v : Value}
    (hdrop : ∀ w ∈ fr.stack.take n, (w.abilities P).drop = true)
    (hplain : ∀ w ∈ subvalues v, IsPlain w = true) (c : Creator) :
    FrameGhost σ P π A auth g fr f
      { g with stack := (IdTree.fresh c v f).1 :: g.stack.drop n }
      { fr with stack := v :: fr.stack.drop n, pc := fr.pc + 1 }
      (IdTree.fresh c v f).2.1 (IdTree.fresh c v f).2.2 := by
  obtain ⟨hft, hocc, hfresh, hevs⟩ := fresh_spec c v f
  have hfa := frame_stack_fits hg
  have hlen := fitsAll_length hfa
  obtain ⟨htake, hdropf⟩ := fitsAll_take_drop n hfa
  refine ⟨frame_fits_stack hg (by simp [IdTree.fitsAll, hft, hdropf]) _, ?_, ?_, ?_, ?_, ?_, ?_,
    kept_stack (ClosuresKept.of_sub fun x hx hc => ?_)⟩
  rotate_right
  · simp only [IdTree.occurrencesAll, hocc, List.mem_append] at hx
    rw [stack_split hfa n, List.mem_append]
    rcases hx with hx | hx
    · rw [plain_not_closure (hplain x.2 (List.of_mem_zip hx).2)] at hc
      cases hc
    · exact Or.inr hx
  · refine effect_stack ?_
    rw [stack_split hfa n]
    simp only [IdTree.occurrencesAll, hocc]
    refine EffectK.mk' (keys (IdTree.occurrencesAll (g.stack.drop n) (fr.stack.drop n)))
      (keys (IdTree.occurrencesAll (g.stack.take n) (fr.stack.take n))) []
      (keys ((List.range' f (subvalues v).length).zip (subvalues v))) []
      (by rw [hevs]; exact issuedOf_map_created c _)
      (by rw [hevs, consumedOf_map_created]; rfl) ?_ ?_ ?_ (by simp) ?_ ?_ ?_
    · simpa using List.perm_append_comm
    · simpa using List.perm_append_comm
    · intro x hx
      obtain ⟨o, ho, -, h2⟩ := mem_keys_iff.1 hx
      obtain ⟨w, hw, hwo⟩ := occurrencesAll_mem htake ho
      rw [← h2, key_type]
      exact deep_drop w (hdrop w hw) o.2 hwo
    · intro x hx
      obtain ⟨o, ho, h1, -⟩ := mem_keys_iff.1 hx
      have := (List.of_mem_zip ho).1
      rw [List.mem_range'] at this
      obtain ⟨k, hk, hko⟩ := this
      rw [← h1, hfresh]
      omega
    · simp only [keys, List.map_map]
      have : ((fun (o : Nat × Key) => o.1) ∘ fun (o : Nat × Value) => (o.1, o.2.key)) =
          (·.1) := rfl
      rw [this, List.map_fst_zip (by simp)]
      exact List.nodup_range' _
    · rw [hfresh]; omega
  · intro i w c' he
    rw [hevs] at he
    simp at he
  · intro e he
    rw [hevs] at he
    obtain ⟨⟨i, w⟩, hiw, rfl⟩ := List.mem_map.1 he
    exact plain_respected (hplain w (List.of_mem_zip hiw).2)
  · intro x hx
    rw [hevs, issuedOf_map_created] at hx
    obtain ⟨o, ho, -, h2⟩ := mem_keys_iff.1 hx
    rw [← h2]
    exact plain_ok (hplain o.2 (List.of_mem_zip ho).2)
  · intro i w c' he
    rw [hevs] at he
    obtain ⟨⟨i', w'⟩, hiw, he'⟩ := List.mem_map.1 he
    simp only [Event.created.injEq] at he'
    obtain ⟨-, rfl, -⟩ := he'
    have := hplain w' (List.of_mem_zip hiw).2
    cases w' <;> simp_all [IsPlain, Value.isRef]
  · intro e he
    rw [hevs] at he
    obtain ⟨⟨i, w⟩, hiw, rfl⟩ := List.mem_map.1 he
    have := hplain w (List.of_mem_zip hiw).2
    exact closures_created fun f mask cs ps rs a h => by subst h; simp [IsPlain] at this

theorem frame_sizes (hg : g.fits fr = true) : g.locals.size = fr.locals.size := by
  rw [frame_fits] at hg
  simp only [Bool.and_eq_true, beq_iff_eq] at hg
  exact hg.1.1

theorem frame_slots (hg : g.fits fr = true) : slotsFit g fr = true := by
  rw [frame_fits] at hg
  simp only [Bool.and_eq_true] at hg
  exact hg.1.2

/-- The shadow of a local that holds a value. -/
theorem slot_some (hg : g.fits fr = true) {i : Nat} (hi : i < fr.locals.size) {v : Value}
    (hv : fr.locals[i] = some v) :
    ∃ t, g.locals[i]'(by rw [frame_sizes hg]; exact hi) = some t ∧ t.fits v = true := by
  have hs := slot_fits (frame_sizes hg) (frame_slots hg) hi
  rw [hv] at hs
  revert hs
  cases g.locals[i]'(by rw [frame_sizes hg]; exact hi) with
  | none => simp [ShadowFrame.slotFits]
  | some t => intro hs; exact ⟨t, rfl, hs⟩

theorem slot_none (hg : g.fits fr = true) {i : Nat} (hi : i < fr.locals.size)
    (hv : fr.locals[i] = none) : g.locals[i]'(by rw [frame_sizes hg]; exact hi) = none := by
  have hs := slot_fits (frame_sizes hg) (frame_slots hg) hi
  rw [hv] at hs
  revert hs
  cases g.locals[i]'(by rw [frame_sizes hg]; exact hi) with
  | none => simp
  | some t => simp [ShadowFrame.slotFits]

theorem perm_swap3 {α : Type} (L C S : List α) : (L ++ (C ++ S)).Perm (C ++ (L ++ S)) := by
  rw [← List.append_assoc, ← List.append_assoc]
  exact List.perm_append_comm.append_right S

/-- `CopyLoc`: the local's identities occur again on the stack. -/
theorem copy_ghost (hg : g.fits fr = true) {i : Nat}
    (hi : i < fr.locals.size) {v : Value} (hv : fr.locals[i] = some v)
    (hc : (v.abilities P).copy = true) :
    ∃ t, g.locals[i]? = some (some t) ∧
      FrameGhost σ P π A auth g fr f { g with stack := t :: g.stack }
        { fr with stack := v :: fr.stack, pc := fr.pc + 1 } f [] := by
  obtain ⟨t, ht, htv⟩ := slot_some hg hi hv
  have hgi : i < g.locals.size := by rw [frame_sizes hg]; exact hi
  refine ⟨t, by rw [Array.getElem?_eq_getElem hgi, ht], ?_⟩
  have hfa := frame_stack_fits hg
  obtain ⟨R, hL, -⟩ := localsOcc_set (frame_sizes hg) hi (g.locals[i]) (fr.locals[i])
  rw [ht, hv] at hL
  simp only [ShadowFrame.slotOccurrences] at hL
  have hsub : ∀ o ∈ t.occurrences v, o ∈ localsOcc g fr := fun o ho =>
    hL.symm.subset (List.mem_append_left _ ho)
  refine ⟨frame_fits_stack hg (by simp [IdTree.fitsAll, htv, hfa]) _, ?_, by simp, by simp,
    by simp, by simp, by simp, ?_⟩
  have he := Effect.copy (P := P) (f := f) (f' := f)
    (fun o ho => deep_copy v hc o.2 (occurrences_mem htv ho))
    (localsOcc g fr ++ IdTree.occurrencesAll g.stack fr.stack)
    (fun o ho => List.mem_append_left _ (hsub o ho)) (Nat.le_refl f)
  refine he.perm (List.Perm.refl _) ?_
  simp only [frame_occurrences, IdTree.occurrencesAll]
  exact perm_swap3 _ _ _
  refine ClosuresKept.of_sub fun x hx _ => ?_
  simp only [frame_occurrences, localsOcc_stack, IdTree.occurrencesAll, List.mem_append] at hx ⊢
  rcases hx with hx | hx | hx
  · exact Or.inl hx
  · exact Or.inl (hsub x hx)
  · exact Or.inr hx

/-- `MoveLoc`: the local's identities move to the stack. -/
theorem move_ghost (hg : g.fits fr = true) {i : Nat}
    (hi : i < fr.locals.size) {v : Value} (hv : fr.locals[i] = some v) :
    ∃ t, g.locals[i]? = some (some t) ∧
      FrameGhost σ P π A auth g fr f
        { g with locals := g.locals.setIfInBounds i none, stack := t :: g.stack }
        { fr with locals := fr.locals.setIfInBounds i none, stack := v :: fr.stack, pc := fr.pc + 1 }
        f [] := by
  obtain ⟨t, ht, htv⟩ := slot_some hg hi hv
  have hgi : i < g.locals.size := by rw [frame_sizes hg]; exact hi
  refine ⟨t, by rw [Array.getElem?_eq_getElem hgi, ht], ?_⟩
  have hfa := frame_stack_fits hg
  have hperm : ({ g with locals := g.locals.setIfInBounds i none, stack := t :: g.stack } :
      ShadowFrame).occurrences
        { fr with locals := fr.locals.setIfInBounds i none, stack := v :: fr.stack, pc := fr.pc + 1 }
        |>.Perm (g.occurrences fr) := by
    obtain ⟨R, hL, hL'⟩ := localsOcc_set (frame_sizes hg) hi (none : Option IdTree)
      (none : Option Value)
    rw [ht, hv] at hL
    simp only [ShadowFrame.slotOccurrences, List.nil_append] at hL hL'
    simp only [frame_occurrences, IdTree.occurrencesAll]
    exact ((hL'.append_right _).trans (perm_swap3 _ _ _ |>.trans
      (by rw [← List.append_assoc]; exact hL.symm.append_right _)))
  refine ⟨?_, Effect.keep hperm (Nat.le_refl f), by simp, by simp, by simp, by simp, by simp,
    ClosuresKept.of_sub fun x hx _ => hperm.subset hx⟩
  · rw [frame_fits]
    simp only [Bool.and_eq_true, beq_iff_eq, Array.size_setIfInBounds]
    exact ⟨⟨frame_sizes hg, slotsFit_set (frame_slots hg) i (by rfl)⟩,
      by simp [IdTree.fitsAll, htv, hfa]⟩

/-- `StLoc`: the top value moves into the local, discarding what it held,
which has `drop`. -/
theorem stLoc_ghost (hg : g.fits fr = true) {i : Nat}
    (hi : i < fr.locals.size) {v : Value} {st : List Value} (hst : fr.stack = v :: st)
    (hold : ∀ old, fr.locals[i] = some old → (old.abilities P).drop = true) :
    ∃ t ts, g.stack = t :: ts ∧
      FrameGhost σ P π A auth g fr f
        { g with locals := g.locals.setIfInBounds i (some t), stack := ts }
        { fr with stack := st, locals := fr.locals.setIfInBounds i (some v), pc := fr.pc + 1 }
        f [] := by
  have hfa := frame_stack_fits hg
  rw [hst] at hfa
  obtain ⟨t, ts, hgst, ht, hts⟩ := fitsAll_cons_right hfa
  refine ⟨t, ts, hgst, ?_, ?_, by simp, by simp, by simp, by simp, by simp, ?_⟩
  · rw [frame_fits]
    simp only [Bool.and_eq_true, beq_iff_eq, Array.size_setIfInBounds]
    exact ⟨⟨frame_sizes hg,
        slotsFit_set (frame_slots hg) i (by simpa [ShadowFrame.slotFits])⟩,
      hts⟩
  · obtain ⟨R, hL, hL'⟩ := localsOcc_set (frame_sizes hg) hi (some t) (some v)
    simp only [ShadowFrame.slotOccurrences] at hL'
    have hgi : i < g.locals.size := by rw [frame_sizes hg]; exact hi
    have hdrop : ∀ o ∈ ShadowFrame.slotOccurrences g.locals[i] fr.locals[i],
        (o.2.type.abilities P).drop = true := by
      cases hfi : fr.locals[i] with
      | none =>
        rw [slot_none hg hi hfi]
        simp [ShadowFrame.slotOccurrences]
      | some old =>
        obtain ⟨told, htold, hfit⟩ := slot_some hg hi hfi
        rw [htold]
        intro o ho
        exact deep_drop old (hold old hfi) o.2 (occurrences_mem hfit ho)
    have he := Effect.drop (P := P) (f := f) (f' := f) hdrop
      (R ++ (t.occurrences v ++ IdTree.occurrencesAll ts st)) (Nat.le_refl f)
    refine he.perm ?_ ?_
    · simp only [frame_occurrences, hgst, hst, IdTree.occurrencesAll]
      refine (hL.append_right _).trans ?_
      simp only [List.append_assoc]; exact List.Perm.refl _
    · simp only [frame_occurrences]
      refine (hL'.append_right _).trans ?_
      simp only [List.append_assoc]
      exact perm_swap3 _ _ _ |>.trans (perm_swap3 _ _ _).symm |>.trans (by
        rw [← List.append_assoc, ← List.append_assoc]
        exact (List.perm_append_comm.append_right _))
  · refine ClosuresKept.of_sub fun x hx _ => ?_
    obtain ⟨R, hL, hL'⟩ := localsOcc_set (frame_sizes hg) hi (some t) (some v)
    simp only [ShadowFrame.slotOccurrences] at hL'
    simp only [frame_occurrences] at hx
    rw [frame_occurrences, hgst, hst]
    have hx' := (hL'.append_right _).subset hx
    simp only [IdTree.occurrencesAll, List.mem_append] at hx' ⊢
    rcases hx' with (hx' | hx') | hx' <;>
      first
        | exact Or.inl (hL.symm.subset (List.mem_append_right _ hx'))
        | exact Or.inr (Or.inl hx')
        | exact Or.inr (Or.inr hx')

/-- `Pack`, `PackVariant`, `VecPack`: the top `n` values move into a
container the instruction creates. -/
theorem pack_ghost (hg : g.fits fr = true) {n : Nat}
    {c : Value} (hch : c.children = (fr.stack.take n).reverse) (hcr : ∀ r, c ≠ .ref r)
    (cr : Creator)
    (hresp : Encapsulation.structsRespected π A (.created f c cr) = true ∧
      Encapsulation.signersRespected π auth (.created f c cr) = true)
    (hclo : Encapsulation.closuresRespected P (.created f c cr) = true) :
    FrameGhost σ P π A auth g fr f
      { g with stack := .node f (g.stack.take n).reverse :: g.stack.drop n }
      { fr with stack := c :: fr.stack.drop n, pc := fr.pc + 1 } (f + 1)
      [.created f c cr] := by
  have hfa := frame_stack_fits hg
  obtain ⟨htake, hdropf⟩ := fitsAll_take_drop n hfa
  have hnode : (IdTree.node f (g.stack.take n).reverse).fits c = true := by
    simp only [IdTree.fits, hch]
    exact fitsAll_reverse htake
  refine ⟨frame_fits_stack hg (by simp [IdTree.fitsAll, hnode, hdropf]) _, ?_, by simp,
    by simpa using hresp, ?_, ?_, by simpa using hclo, kept_stack fun x hx hc => ?_⟩
  rotate_right
  · simp only [IdTree.occurrencesAll, IdTree.occurrences, hch, List.mem_append,
      List.mem_cons] at hx
    rcases hx with (rfl | hx) | hx
    · exact Or.inr ⟨cr, List.mem_singleton_self _⟩
    · refine Or.inl ?_
      rw [stack_split hfa n]
      exact List.mem_append_left _ ((occurrencesAll_reverse htake).subset hx)
    · refine Or.inl ?_
      rw [stack_split hfa n]
      exact List.mem_append_right _ hx
  · refine effect_stack ?_
    rw [stack_split hfa n]
    simp only [IdTree.occurrencesAll, IdTree.occurrences, hch]
    refine EffectK.mk' (keys (IdTree.occurrencesAll (g.stack.take n) (fr.stack.take n) ++
        IdTree.occurrencesAll (g.stack.drop n) (fr.stack.drop n))) [] [] [(f, c.key)] []
      (by simp) (by simp) (by simp) ?_ (by simp) (by simp) (by simp) (by simp) (by omega)
    simp only [keys_cons, keys_append, List.append_nil]
    exact (((keys_perm (occurrencesAll_reverse htake)).append_right _).cons _).trans
      (by perm_parts)
  · intro x hx
    simp only [issuedOf_cons_created, issuedOf_nil, List.mem_singleton] at hx
    subst hx
    exact container_ok hcr
  · intro i w c' he
    simp only [List.mem_singleton, Event.created.injEq] at he
    rw [he.2.1]
    cases c <;> simp_all [Value.isRef]

/-- `Unpack`, `UnpackVariant`, `VecUnpack`: the instruction consumes the
container on top and its contents move onto the stack. -/
theorem unpack_ghost (hg : g.fits fr = true) {c : Value}
    {st : List Value} (hst : fr.stack = c :: st) (cons : Consumer)
    (hlegit : (c.type.abilities P).drop = true ∨ cons.Legitimate π c = true)
    (hresp : ∀ i, Encapsulation.structsRespected π A (.consumed i c cons) = true) :
    ∃ t ts, g.stack = t :: ts ∧
      FrameGhost σ P π A auth g fr f { g with stack := t.children.reverse ++ ts }
        { fr with stack := c.children.reverse ++ st, pc := fr.pc + 1 } f
        [.consumed t.id c cons] := by
  have hfa := frame_stack_fits hg
  rw [hst] at hfa
  obtain ⟨t, ts, hgst, ht, hts⟩ := fitsAll_cons_right hfa
  obtain ⟨i, cs⟩ := t
  simp only [IdTree.fits] at ht
  have hlen := fitsAll_length ht
  refine ⟨.node i cs, ts, hgst, frame_fits_stack hg ?_ _, ?_, ?_, ?_, by simp, by simp,
    by simp [Encapsulation.closuresRespected], kept_stack (ClosuresKept.of_sub fun x hx _ => ?_)⟩
  rotate_right
  · rw [hgst, hst]
    simp only [IdTree.children] at hx
    rw [occurrencesAll_append (by simp [hlen])] at hx
    simp only [IdTree.occurrencesAll, IdTree.occurrences, List.mem_append, List.mem_cons] at hx ⊢
    rcases hx with hx | hx
    · exact Or.inl (Or.inr ((occurrencesAll_reverse ht).subset hx))
    · exact Or.inr hx
  · simp only [IdTree.children]
    rw [fitsAll_append (by simp [hlen])]
    simp [fitsAll_reverse ht, hts]
  · refine effect_stack ?_
    rw [hgst, hst]
    simp only [IdTree.occurrencesAll, IdTree.occurrences, IdTree.children, IdTree.id]
    rw [occurrencesAll_append (by simp [hlen])]
    refine EffectK.mk' (keys (IdTree.occurrencesAll cs c.children ++ IdTree.occurrencesAll ts st))
      [] [] [] [(i, c.key)] (by simp) (by simp) ?_ ?_ (by simp) (by simp) (by simp) (by simp)
      (Nat.le_refl f)
    · simp only [keys_cons, keys_append, List.append_nil]
      perm_parts
    · simp only [keys_append, List.append_nil]
      exact (keys_perm (occurrencesAll_reverse ht)).append_right _
  · intro i' w c' he
    simp only [List.mem_singleton, Event.consumed.injEq] at he
    obtain ⟨-, rfl, rfl⟩ := he
    exact hlegit
  · intro e he
    simp only [List.mem_singleton] at he
    subst he
    exact ⟨hresp _, rfl⟩

/-- The borrows and `FreezeRef`: the top `n` values, which have `drop`, are
discarded, and a new reference, with the next fresh identity, is pushed. -/
theorem borrow_ghost (hg : g.fits fr = true) {n : Nat}
    (hdrop : ∀ w ∈ fr.stack.take n, (w.abilities P).drop = true) {r : Reference}
    (hok : (Value.ref r).key.Ok σ) (parent : Option Nat) :
    FrameGhost σ P π A auth g fr f { g with stack := .node f [] :: g.stack.drop n }
      { fr with stack := .ref r :: fr.stack.drop n, pc := fr.pc + 1 } (f + 1)
      [.borrowed f r parent] := by
  have hfa := frame_stack_fits hg
  obtain ⟨htake, hdropf⟩ := fitsAll_take_drop n hfa
  refine ⟨frame_fits_stack hg (by simp [IdTree.fitsAll, IdTree.fits, Value.children, hdropf]) _,
    ?_, by simp, ?_, ?_, by simp, by simp [Encapsulation.closuresRespected],
    kept_stack (ClosuresKept.of_sub fun x hx hc => ?_)⟩
  rotate_right
  · simp only [IdTree.occurrencesAll, occurrences_node, List.mem_cons, List.mem_append] at hx
    rcases hx with (rfl | hx) | hx
    · simp [IsClosure] at hc
    · simp at hx
    · rw [stack_split hfa n]; exact List.mem_append_right _ hx
  · refine effect_stack ?_
    rw [stack_split hfa n]
    refine EffectK.mk' (keys (IdTree.occurrencesAll (g.stack.drop n) (fr.stack.drop n)))
      (keys (IdTree.occurrencesAll (g.stack.take n) (fr.stack.take n))) [] [(f, (Value.ref r).key)] []
      (by simp) (by simp) (by simpa using List.perm_append_comm) ?_ ?_ (by simp) (by simp) (by simp)
      (by omega)
    · simp only [IdTree.occurrencesAll, occurrences_node, keys_cons, keys_append, List.append_nil]
      simp only [keys_nil]
      exact List.perm_append_comm
    · intro x hx
      obtain ⟨o, ho, -, h2⟩ := mem_keys_iff.1 hx
      obtain ⟨w, hw, hwo⟩ := occurrencesAll_mem htake ho
      rw [← h2, key_type]
      exact deep_drop w (hdrop w hw) o.2 hwo
  · intro e he
    simp only [List.mem_singleton] at he
    subst he
    exact ⟨rfl, rfl⟩
  · intro x hx
    simp only [issuedOf_cons_borrowed, issuedOf_nil, List.mem_singleton] at hx
    subst hx
    exact hok

end Patterns

/-! ## Calls and returns -/

section Calls

variable {P : Program} {π : NativePolicy} {A : List StructId} {auth : List Address}

theorem args_occurrences {ts : List IdTree} {vs : List Value} (h : ts.length = vs.length)
    (m : Nat) :
    ((ts.map some ++ List.replicate m none).zip (vs.map some ++ List.replicate m none)).flatMap
      (fun (p : Option IdTree × Option Value) => ShadowFrame.slotOccurrences p.1 p.2) =
      IdTree.occurrencesAll ts vs := by
  rw [List.zip_append (by simp [h]), List.flatMap_append, occurrencesAll_eq_flatMap]
  have h1 : ((ts.map some).zip (vs.map some)) = (ts.zip vs).map fun (t, v) => (some t, some v) := by
    rw [List.zip_map]; rfl
  have h2 : ((List.replicate m (none : Option IdTree)).zip
      (List.replicate m (none : Option Value))).flatMap
      (fun (p : Option IdTree × Option Value) => ShadowFrame.slotOccurrences p.1 p.2) = [] := by
    induction m with
    | zero => rfl
    | succ m => simp [List.replicate_succ, ShadowFrame.slotOccurrences]
  rw [h1, h2, List.flatMap_map]
  simp [ShadowFrame.slotOccurrences]

theorem fitsAll_mem : ∀ {ts : List IdTree} {vs : List Value}, IdTree.fitsAll ts vs = true →
    ∀ p ∈ ts.zip vs, p.1.fits p.2 = true
  | [], _, _, _, hp => by simp at hp
  | _ :: _, [], _, _, hp => by simp at hp
  | t :: ts, v :: vs, h, p, hp => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    rcases List.mem_cons.1 hp with rfl | hp
    · exact h.1
    · exact fitsAll_mem h.2 p hp

theorem args_fit {ts : List IdTree} {vs : List Value} (h : IdTree.fitsAll ts vs = true) (m : Nat) :
    ((ts.map some ++ List.replicate m none).zip (vs.map some ++ List.replicate m none)).all
      (fun (p : Option IdTree × Option Value) => ShadowFrame.slotFits p.1 p.2) = true := by
  rw [List.zip_append (by simp [fitsAll_length h]), List.all_append, Bool.and_eq_true]
  constructor
  · rw [List.zip_map, List.all_map, List.all_eq_true]
    intro p hp
    exact fitsAll_mem h p hp
  · rw [List.all_eq_true]
    intro p hp
    have := List.of_mem_zip hp
    rw [List.mem_replicate, List.mem_replicate] at this
    rw [this.1.2, this.2.2]
    rfl

theorem signers_respected {e : Event} (he : e.Silent = true) (auth : List Address) :
    Encapsulation.signersRespected π auth e = true := by
  cases e <;> simp_all [Event.Silent, Encapsulation.signersRespected]

/-- `Call` of a function with code: the arguments move from the caller's
stack into the callee's first locals, in a frame with a new identity. -/
theorem call_ghost {s s₂ : State} {h : Shadow} {fr : Frame} {rest : List Frame} {g : ShadowFrame}
    {hs : List ShadowFrame} (hp : Pre σ P s h) (hsf : s.frames = fr :: rest)
    (hhf : h.frames = g :: hs) {n : Nat} (hn : n ≤ fr.stack.length) (callee : Frame) (m : Nat)
    (hloc : callee.locals = ((fr.stack.take n).reverse.map some ++ List.replicate m none).toArray)
    (hstk : callee.stack = []) (hid : callee.id = s.nextFrame)
    (h2f : s₂.frames = callee :: { fr with stack := fr.stack.drop n } :: rest)
    (hst : s₂.storage = s.storage) (hhe : s₂.held = s.held) (hnf : s₂.nextFrame = s.nextFrame + 1)
    {evs : List Event} (hevs : ∀ e ∈ evs, e.Silent = true ∧ Encapsulation.structsRespected π A e = true) :
    StepGhost σ P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh (.running s₂)
      (.running { h with frames := { locals := ((g.stack.take n).reverse.map some ++
          List.replicate (callee.locals.size - n) none).toArray, stack := [] } ::
        { g with stack := g.stack.drop n } :: hs }) evs := by
  have hfit := hp.fits
  rw [shadow_fits, hsf, hhf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
  obtain ⟨⟨hg, hrest⟩, hext⟩ := hfit
  have hfa := frame_stack_fits hg
  have hlen := fitsAll_length hfa
  obtain ⟨htake, hdropf⟩ := fitsAll_take_drop n hfa
  have hsize : callee.locals.size - n = m := by
    rw [hloc]; simp [List.length_take, Nat.min_eq_left hn]
  have hrl : ((g.stack.take n).reverse).length = ((fr.stack.take n).reverse).length := by
    simp [hlen]
  have hcallee : ({ locals := ((g.stack.take n).reverse.map some ++
      List.replicate (callee.locals.size - n) none).toArray, stack := [] } : ShadowFrame).fits
      callee = true := by
    rw [frame_fits, hsize]
    simp only [slotsFit, hloc, hstk, Bool.and_eq_true, beq_iff_eq, List.toList_toArray]
    refine ⟨⟨by simp [hlen], ?_⟩, by simp [IdTree.fitsAll]⟩
    exact args_fit (fitsAll_reverse htake) m
  have hcaller : ({ g with stack := g.stack.drop n } : ShadowFrame).fits
      { fr with stack := fr.stack.drop n } = true := frame_fits_stack hg hdropf fr.pc
  have hsil := silent_of fun e he => (hevs e he).1
  have hrefs : ∀ x, x ∈ postFrames (.running { h with frames :=
      { locals := ((g.stack.take n).reverse.map some ++
        List.replicate (callee.locals.size - n) none).toArray, stack := [] } ::
      { g with stack := g.stack.drop n } :: hs }) (.running s₂) →
      x ∈ framesOcc h.frames s.frames := by
    intro x hm
    simp only [postFrames, h2f, framesOcc_cons, List.mem_append] at hm
    rw [hhf, hsf, framesOcc_cons, List.mem_append, frame_occurrences, List.mem_append,
      stack_split hfa n, List.mem_append]
    rcases hm with hm | hm | hm
    · rw [frame_occurrences, List.mem_append] at hm
      rcases hm with hm | hm
      · have hlo : localsOcc { locals := ((g.stack.take n).reverse.map some ++
            List.replicate (callee.locals.size - n) none).toArray, stack := [] } callee =
            IdTree.occurrencesAll (g.stack.take n).reverse (fr.stack.take n).reverse := by
          rw [hsize]
          simp only [localsOcc, hloc, List.toList_toArray]
          exact args_occurrences hrl m
        rw [hlo] at hm
        exact Or.inl (Or.inr (Or.inl ((occurrencesAll_reverse htake).subset hm)))
      · rw [hstk] at hm
        simp [IdTree.occurrencesAll] at hm
    · rw [frame_occurrences, List.mem_append] at hm
      rcases hm with hm | hm
      · exact Or.inl (Or.inl hm)
      · exact Or.inl (Or.inr (Or.inr hm))
    · exact Or.inr hm
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, createdPlain_of hsil.1, fun d r hm => Or.inl (hrefs _ hm),
    refsPerFrame_of_ne ?_, closures_silent fun e he => (hevs e he).1,
    kept_frames rfl rfl hst hhe hrefs⟩
  rotate_right
  · simp only [postPairs, h2f, hhf, hsf, List.zip_cons_cons, List.length_cons]
    omega
  · simp only [ShadowPost.fits, shadow_fits, framesFit_cons, h2f, hcallee, hcaller, hrest,
      Bool.and_self, Bool.true_and]
    exact (extFit_congr (h := h) (s := s) rfl rfl hst hhe).trans hext
  · show Effect P _ (Shadow.occurrences _ _) _ _ _
    have hkeep : Effect P (h.occurrences s)
        (Shadow.occurrences { h with frames := { locals := ((g.stack.take n).reverse.map some ++
          List.replicate (callee.locals.size - n) none).toArray, stack := [] } ::
          { g with stack := g.stack.drop n } :: hs } s₂) [] h.fresh h.fresh := by
      refine Effect.keep ?_ (Nat.le_refl _)
      have hx : extOcc { h with frames := { locals := ((g.stack.take n).reverse.map some ++
          List.replicate (callee.locals.size - n) none).toArray, stack := [] } ::
          { g with stack := g.stack.drop n } :: hs } s₂ = extOcc h s := extOcc_congr rfl rfl hst hhe
      rw [shadow_occurrences, shadow_occurrences, hsf, hhf, hx, h2f]
      simp only [framesOcc_cons, frame_occurrences, localsOcc_stack]
      rw [hsize]
      simp only [localsOcc, hloc, List.toList_toArray, IdTree.occurrencesAll, List.append_nil]
      rw [args_occurrences hrl m, stack_split hfa n]
      have hr := occurrencesAll_reverse htake
      rw [List.perm_iff_count]; intro a
      have := List.perm_iff_count.1 hr a
      simp only [List.count_append] at this ⊢
      omega
    exact EffectK.of_silent hkeep fun e he => (hevs e he).1
  · intro i w c he
    exact absurd (consumed_mem he) (by simp [hsil.2.1])
  · intro e he
    exact ⟨(hevs e he).2, signers_respected (hevs e he).1 auth⟩
  · intro x hx
    simp [hsil.2.2] at hx
  · have hinv := hp.inv
    have hbelow : ∀ x ∈ fr :: rest, x.id < s.nextFrame := fun x hx =>
      hinv.framesBelow x (by rw [hsf]; exact hx)
    refine ⟨?_, ?_, ?_, ?_, ?_⟩
    · have := hinv.frameIds
      rw [hsf] at this
      rw [h2f]
      simp only [List.map_cons] at this ⊢
      refine List.nodup_cons.2 ⟨fun hm => ?_, this⟩
      have hm' : callee.id ∈ (fr :: rest).map (·.id) := hm
      obtain ⟨x, hx, hxe⟩ := List.mem_map.1 hm'
      have hlt := hbelow x hx
      rw [hxe, hid] at hlt
      exact Nat.lt_irrefl _ hlt
    · intro x hx
      rw [hnf]
      rw [h2f] at hx
      simp only [List.mem_cons] at hx
      rcases hx with rfl | rfl | hx
      · rw [hid]; exact Nat.lt_succ_self _
      · exact Nat.lt_succ_of_lt (hbelow fr (List.mem_cons_self ..))
      · exact Nat.lt_succ_of_lt (hbelow x (List.mem_cons_of_mem _ hx))
    · rw [hst]; exact hinv.resources
    · rw [hst]; exact hinv.typed
    · rw [hhe]; exact hinv.heldDrop

/-! ### Closure calls -/

theorem maskCompose_go_fits : ∀ (bits : List Bool) {ts₁ ts₂ : List IdTree} {vs₁ vs₂ : List Value},
    IdTree.fitsAll ts₁ vs₁ = true → IdTree.fitsAll ts₂ vs₂ = true →
    IdTree.fitsAll (maskCompose.go bits ts₁ ts₂) (maskCompose.go bits vs₁ vs₂) = true
  | [], _, _, _, _, _, _ => by simp [maskCompose.go, IdTree.fitsAll]
  | true :: _, [], _, [], _, _, _ => by simp [maskCompose.go, IdTree.fitsAll]
  | true :: bits, _ :: _, _, _ :: _, _, h₁, h₂ => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h₁
    simp only [maskCompose.go, IdTree.fitsAll, Bool.and_eq_true]
    exact ⟨h₁.1, maskCompose_go_fits bits h₁.2 h₂⟩
  | false :: _, _, [], _, [], _, _ => by simp [maskCompose.go, IdTree.fitsAll]
  | false :: bits, _, _ :: _, _, _ :: _, h₁, h₂ => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h₂
    simp only [maskCompose.go, IdTree.fitsAll, Bool.and_eq_true]
    exact ⟨h₂.1, maskCompose_go_fits bits h₁ h₂.2⟩
  | true :: _, [], _, _ :: _, _, h₁, _ => by simp [IdTree.fitsAll] at h₁
  | true :: _, _ :: _, _, [], _, h₁, _ => by simp [IdTree.fitsAll] at h₁
  | false :: _, _, [], _, _ :: _, _, h₂ => by simp [IdTree.fitsAll] at h₂
  | false :: _, _, _ :: _, _, [], _, h₂ => by simp [IdTree.fitsAll] at h₂

theorem maskCompose_go_occ : ∀ (bits : List Bool) {ts₁ ts₂ : List IdTree} {vs₁ vs₂ : List Value},
    IdTree.fitsAll ts₁ vs₁ = true → IdTree.fitsAll ts₂ vs₂ = true →
    bits.count true = ts₁.length → bits.count false = ts₂.length →
    (IdTree.occurrencesAll (maskCompose.go bits ts₁ ts₂) (maskCompose.go bits vs₁ vs₂)).Perm
      (IdTree.occurrencesAll ts₁ vs₁ ++ IdTree.occurrencesAll ts₂ vs₂)
  | [], ts₁, ts₂, vs₁, vs₂, h₁, h₂, c₁, c₂ => by
    simp only [List.count_nil] at c₁ c₂
    cases ts₁ with
    | cons => simp at c₁
    | nil =>
      cases ts₂ with
      | cons => simp at c₂
      | nil =>
        cases vs₁ with
        | cons => simp [IdTree.fitsAll] at h₁
        | nil =>
          cases vs₂ with
          | cons => simp [IdTree.fitsAll] at h₂
          | nil => simp [maskCompose.go, IdTree.occurrencesAll]
  | true :: bits, ts₁, ts₂, vs₁, vs₂, h₁, h₂, c₁, c₂ => by
    simp only [List.count_cons, beq_self_eq_true, if_true] at c₁
    simp only [List.count_cons] at c₂
    cases ts₁ with
    | nil => simp at c₁
    | cons t ts₁ =>
      cases vs₁ with
      | nil => simp [IdTree.fitsAll] at h₁
      | cons v vs₁ =>
        simp only [IdTree.fitsAll, Bool.and_eq_true] at h₁
        simp only [maskCompose.go, IdTree.occurrencesAll, List.append_assoc]
        refine List.Perm.append_left _ (maskCompose_go_occ bits h₁.2 h₂ ?_ ?_)
        · simpa using c₁
        · simpa using c₂
  | false :: bits, ts₁, ts₂, vs₁, vs₂, h₁, h₂, c₁, c₂ => by
    simp only [List.count_cons] at c₁
    simp only [List.count_cons, beq_self_eq_true, if_true] at c₂
    cases ts₂ with
    | nil => simp at c₂
    | cons t ts₂ =>
      cases vs₂ with
      | nil => simp [IdTree.fitsAll] at h₂
      | cons v vs₂ =>
        simp only [IdTree.fitsAll, Bool.and_eq_true] at h₂
        simp only [maskCompose.go, IdTree.occurrencesAll]
        refine (List.Perm.append_left _ (maskCompose_go_occ bits h₁ h₂.2 ?_ ?_)).trans ?_
        · simpa using c₁
        · simpa using c₂
        · rw [← List.append_assoc, ← List.append_assoc]
          exact List.Perm.append_right _ List.perm_append_comm

theorem count_true_add_false : ∀ (bs : List Bool), bs.count true + bs.count false = bs.length
  | [] => rfl
  | b :: bs => by
    have := count_true_add_false bs
    cases b <;> simp <;> omega

theorem maskCompose_fits {mask : BitVec 64} {ts₁ ts₂ : List IdTree} {vs₁ vs₂ : List Value}
    (h₁ : IdTree.fitsAll ts₁ vs₁ = true) (h₂ : IdTree.fitsAll ts₂ vs₂ = true) :
    IdTree.fitsAll (maskCompose mask ts₁ ts₂) (maskCompose mask vs₁ vs₂) = true := by
  unfold maskCompose
  rw [fitsAll_length h₁, fitsAll_length h₂]
  exact maskCompose_go_fits _ h₁ h₂

/-- The occurrences in the arguments a closure call composes are those in
its captured and provided values, when the mask composes them completely
(`Composes`). -/
theorem maskCompose_occ {mask : BitVec 64} {ts₁ ts₂ : List IdTree} {vs₁ vs₂ : List Value}
    (h₁ : IdTree.fitsAll ts₁ vs₁ = true) (h₂ : IdTree.fitsAll ts₂ vs₂ = true)
    (hc : ((List.range (vs₁.length + vs₂.length)).map mask.getLsbD).count true = vs₁.length) :
    (IdTree.occurrencesAll (maskCompose mask ts₁ ts₂) (maskCompose mask vs₁ vs₂)).Perm
      (IdTree.occurrencesAll ts₁ vs₁ ++ IdTree.occurrencesAll ts₂ vs₂) := by
  unfold maskCompose
  rw [fitsAll_length h₁, fitsAll_length h₂]
  have := count_true_add_false ((List.range (vs₁.length + vs₂.length)).map mask.getLsbD)
  simp only [List.length_map, List.length_range] at this
  exact maskCompose_go_occ _ h₁ h₂ (by rw [hc, fitsAll_length h₁])
    (by rw [fitsAll_length h₂]; omega)

/-- `CallClosure` of a closure whose function has code: the closure on top is
consumed, and its captured values and the provided arguments move into the
callee's first locals, in parameter order, in a frame with a new identity. -/
theorem callClosure_ghost {s s₂ : State} {h : Shadow} {fr : Frame} {rest : List Frame}
    {g : ShadowFrame} {hs : List ShadowFrame} (hp : Pre σ P s h) (hsf : s.frames = fr :: rest)
    (hhf : h.frames = g :: hs) {c : Value} {st : List Value} (hst : fr.stack = c :: st)
    {target : FunctionRef} {mask : BitVec 64} {cs : List Value} {cps crs : List RuntimeType}
    {ca : AbilitySet} (hc : c = .closure target mask cs cps crs ca) {i : Nat} {cts gst : List IdTree}
    (hgst : g.stack = .node i cts :: gst) {k : Nat}
    (hk : k = cps.length) (hn : k ≤ st.length) (callee : Frame) (m : Nat)
    (hloc : callee.locals = ((maskCompose mask cs (st.take k).reverse).map some ++
      List.replicate m none).toArray)
    (hstk : callee.stack = []) (hid : callee.id = s.nextFrame)
    (h2f : s₂.frames = callee :: { fr with stack := st.drop k } :: rest)
    (h2st : s₂.storage = s.storage) (h2he : s₂.held = s.held)
    (h2nf : s₂.nextFrame = s.nextFrame + 1)
    {evs : List Event} (hevs : ∀ e ∈ evs, e.Silent = true ∧ Encapsulation.structsRespected π A e = true)
    (mod : ModuleId) :
      StepGhost σ P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames)
        h.fresh (.running s₂)
        (.running { h with frames :=
          { locals := ((maskCompose mask cts (gst.take k).reverse).map some ++
              List.replicate (callee.locals.size - (maskCompose mask cts (gst.take k).reverse).length)
                none).toArray, stack := [] } ::
          { g with stack := gst.drop k } :: hs })
        (evs ++ [.consumed i c (.callClosure mod)]) := by
  have hfit := hp.fits
  rw [shadow_fits, hsf, hhf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
  obtain ⟨⟨hg, hrest⟩, hext⟩ := hfit
  have hfa := frame_stack_fits hg
  rw [hst, hgst] at hfa
  simp only [IdTree.fitsAll, Bool.and_eq_true] at hfa
  obtain ⟨hct, hgsf⟩ := hfa
  have hcts : IdTree.fitsAll cts cs = true := by
    simpa [IdTree.fits, hc, Value.children] using hct
  have hcomp : Composes P c :=
    hp.composes fr (by rw [hsf]; exact List.mem_cons_self ..) c (by rw [hst]; exact List.mem_cons_self ..)
  rw [hc] at hcomp
  simp only [Composes] at hcomp
  obtain ⟨htake, hdropf⟩ := fitsAll_take_drop k hgsf
  have hpl : ((st.take k).reverse).length = cps.length := by
    rw [List.length_reverse, List.length_take, Nat.min_eq_left hn, hk]
  have hfargs := maskCompose_fits (mask := mask) hcts (fitsAll_reverse htake)
  have hocc : (IdTree.occurrencesAll (maskCompose mask cts (gst.take k).reverse)
      (maskCompose mask cs (st.take k).reverse)).Perm
      (IdTree.occurrencesAll cts cs ++ IdTree.occurrencesAll (gst.take k) (st.take k)) :=
    (maskCompose_occ hcts (fitsAll_reverse htake) (by rw [hpl]; exact hcomp)).trans
      (List.Perm.append_left _ (occurrencesAll_reverse htake))
  generalize maskCompose mask cts (gst.take k).reverse = argsT at hfargs hocc ⊢
  generalize maskCompose mask cs (st.take k).reverse = argsV at hfargs hocc hloc
  have hrl := fitsAll_length hfargs
  have hsize : callee.locals.size - argsT.length = m := by
    rw [hloc, hrl]; simp
  have hcallee : ({ locals := (argsT.map some ++ List.replicate (callee.locals.size - argsT.length)
      none).toArray, stack := [] } : ShadowFrame).fits callee = true := by
    rw [frame_fits, hsize]
    simp only [slotsFit, hloc, hstk, Bool.and_eq_true, beq_iff_eq, List.toList_toArray]
    refine ⟨⟨by simp [hrl], ?_⟩, by simp [IdTree.fitsAll]⟩
    exact args_fit hfargs m
  have hcaller : ({ g with stack := gst.drop k } : ShadowFrame).fits
      { fr with stack := st.drop k } = true := frame_fits_stack hg hdropf fr.pc
  have hsil := silent_of fun e he => (hevs e he).1
  have hlo : localsOcc ({ locals := (argsT.map some ++ List.replicate (callee.locals.size -
      argsT.length) none).toArray, stack := [] } : ShadowFrame) callee =
      IdTree.occurrencesAll argsT argsV := by
    rw [hsize]
    simp only [localsOcc, hloc, List.toList_toArray]
    exact args_occurrences hrl m
  have hpreF : framesOcc h.frames s.frames = localsOcc g fr ++
      ((i, c) :: IdTree.occurrencesAll cts cs ++ (IdTree.occurrencesAll (gst.take k) (st.take k) ++
        IdTree.occurrencesAll (gst.drop k) (st.drop k))) ++ framesOcc hs rest := by
    rw [hhf, hsf, framesOcc_cons, frame_occurrences, hgst, hst]
    have hcc : IdTree.occurrences (.node i cts) c = (i, c) :: IdTree.occurrencesAll cts cs := by
      rw [hc]; rfl
    simp only [IdTree.occurrencesAll, hcc]
    rw [stack_split hgsf k]
    try simp [List.append_assoc]
  have hpostF : framesOcc (({ locals := (argsT.map some ++ List.replicate (callee.locals.size -
      argsT.length) none).toArray, stack := [] } : ShadowFrame) ::
      { g with stack := gst.drop k } :: hs) s₂.frames =
      IdTree.occurrencesAll argsT argsV ++
      (localsOcc g fr ++ IdTree.occurrencesAll (gst.drop k) (st.drop k)) ++ framesOcc hs rest := by
    simp only [h2f, framesOcc_cons, frame_occurrences, hlo,
      IdTree.occurrencesAll, List.append_nil]
    rw [localsOcc_stack]
    try simp [List.append_assoc]
  have hsubF : ∀ x ∈ framesOcc (({ locals := (argsT.map some ++ List.replicate
      (callee.locals.size - argsT.length) none).toArray, stack := [] } : ShadowFrame) ::
      { g with stack := gst.drop k } :: hs) s₂.frames, x ∈ framesOcc h.frames s.frames := by
    intro x hm
    rw [hpostF] at hm
    rw [hpreF]
    simp only [List.mem_append, List.mem_cons] at hm ⊢
    rcases hm with (hm | hm | hm) | hm
    · rcases List.mem_append.1 (hocc.subset hm) with hm | hm
      · exact Or.inl (Or.inr (Or.inl (Or.inr hm)))
      · exact Or.inl (Or.inr (Or.inr (Or.inl hm)))
    · exact Or.inl (Or.inl hm)
    · exact Or.inl (Or.inr (Or.inr (Or.inr hm)))
    · exact Or.inr hm
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, createdPlain_of (by simp [hsil.1]), ?_, refsPerFrame_of_ne ?_,
    fun e he => ?_, kept_frames rfl rfl h2st h2he hsubF⟩
  rotate_right
  · rcases List.mem_append.1 he with he | he
    · exact closures_silent (fun e he => (hevs e he).1) e he
    · simp only [List.mem_singleton] at he
      subst he
      rfl
  rotate_right
  · simp only [postPairs, h2f, hhf, hsf, List.zip_cons_cons, List.length_cons]
    omega
  · simp only [ShadowPost.fits, shadow_fits, framesFit_cons, h2f, hcallee, hcaller, hrest,
      Bool.and_self, Bool.true_and]
    exact (extFit_congr (h := h) (s := s) rfl rfl h2st h2he).trans hext
  · show Effect P _ (Shadow.occurrences _ _) _ _ _
    have hx : extOcc ({ h with frames := ({ locals := (argsT.map some ++
        List.replicate (callee.locals.size - argsT.length) none).toArray, stack := [] } :
          ShadowFrame) :: { g with stack := gst.drop k } :: hs } : Shadow) s₂ = extOcc h s :=
      extOcc_congr rfl rfl h2st h2he
    unfold Effect
    rw [shadow_occurrences, shadow_occurrences, hpreF, hx]
    dsimp only
    rw [hpostF]
    refine EffectK.mk' (keys (localsOcc g fr ++ IdTree.occurrencesAll cts cs ++
        IdTree.occurrencesAll (gst.take k) (st.take k) ++
        IdTree.occurrencesAll (gst.drop k) (st.drop k) ++ framesOcc hs rest ++ extOcc h s))
      [] [] [] [(i, c.key)] (by simp [hsil.2.2]) (by simp [hsil.2.1]) ?_ ?_ (by simp) (by simp)
      (by simp) (by simp) (Nat.le_refl _)
    · simp only [keys_append, keys_cons, List.append_nil]
      perm_parts
    · have hk' := keys_perm hocc
      rw [List.perm_iff_count]; intro a
      have := List.perm_iff_count.1 hk' a
      simp only [keys_append, List.count_append, List.append_nil] at this ⊢
      omega
  · intro i' w c' he
    rcases List.mem_append.1 he with he | he
    · exact absurd (consumed_mem he) (by simp [hsil.2.1])
    · simp only [List.mem_singleton, Event.consumed.injEq] at he
      obtain ⟨-, rfl, rfl⟩ := he
      right; rw [hc]; rfl
  · intro e he
    rcases List.mem_append.1 he with he | he
    · exact ⟨(hevs e he).2, signers_respected (hevs e he).1 auth⟩
    · simp only [List.mem_singleton] at he
      subst he
      rw [hc]
      exact ⟨rfl, rfl⟩
  · intro x hx
    simp [hsil.2.2] at hx
  · have hinv := hp.inv
    have hbelow : ∀ x ∈ fr :: rest, x.id < s.nextFrame := fun x hx =>
      hinv.framesBelow x (by rw [hsf]; exact hx)
    refine ⟨?_, ?_, ?_, ?_, ?_⟩
    · have := hinv.frameIds
      rw [hsf] at this
      rw [h2f]
      simp only [List.map_cons] at this ⊢
      refine List.nodup_cons.2 ⟨fun hm => ?_, this⟩
      have hm' : callee.id ∈ (fr :: rest).map (·.id) := hm
      obtain ⟨x, hx, hxe⟩ := List.mem_map.1 hm'
      have hlt := hbelow x hx
      rw [hxe, hid] at hlt
      exact Nat.lt_irrefl _ hlt
    · intro x hx
      rw [h2nf]
      rw [h2f] at hx
      simp only [List.mem_cons] at hx
      rcases hx with rfl | rfl | hx
      · rw [hid]; exact Nat.lt_succ_self _
      · exact Nat.lt_succ_of_lt (hbelow fr (List.mem_cons_self ..))
      · exact Nat.lt_succ_of_lt (hbelow x (List.mem_cons_of_mem _ hx))
    · rw [h2st]; exact hinv.resources
    · rw [h2st]; exact hinv.typed
    · rw [h2he]; exact hinv.heldDrop
  · intro d r hm
    exact Or.inl (hsubF _ hm)

/-- The identities in the locals of a frame whose values all have `drop`
have types with `drop`. -/
theorem localsOcc_drop {g : ShadowFrame} {fr : Frame} (hg : g.fits fr = true)
    (hl : ∀ v, some v ∈ fr.locals.toList → (v.abilities P).drop = true) :
    ∀ o ∈ localsOcc g fr, (o.2.type.abilities P).drop = true := by
  intro o ho
  simp only [localsOcc, List.mem_flatMap] at ho
  obtain ⟨⟨t, v⟩, hp, ho⟩ := ho
  have hfit : ShadowFrame.slotFits t v = true := by
    have := frame_slots hg
    simp only [slotsFit, List.all_eq_true] at this
    exact this _ hp
  rcases t with _ | t <;> rcases v with _ | v
  · simp [ShadowFrame.slotOccurrences] at ho
  · simp [ShadowFrame.slotOccurrences] at ho
  · simp [ShadowFrame.slotOccurrences] at ho
  · exact deep_drop v (hl v (List.of_mem_zip hp).2) o.2 (occurrences_mem hfit ho)

/-- The identities of the held values, which have `drop`. -/
theorem heldOcc_drop {h : Shadow} {s : State}
    (hd : ∀ v ∈ s.held, (v.abilities P).drop = true)
    (hfa : IdTree.fitsAll h.held s.held = true) :
    ∀ o ∈ IdTree.occurrencesAll h.held s.held, (o.2.type.abilities P).drop = true := by
  intro o ho
  obtain ⟨v, hv, hov⟩ := occurrencesAll_mem hfa ho
  exact deep_drop v (hd v hv) o.2 hov

/-- `Ret` from the outermost frame: the segment's values become the returned
values and global storage is committed; the locals' values and the held
values, which have `drop`, are discarded. -/
theorem ret_last_ghost {s : State} {h : Shadow} {fr : Frame} {g : ShadowFrame}
    (hp : Pre σ P s h) (hsf : s.frames = [fr]) (hhf : h.frames = [g])
    (hd : ∀ v ∈ s.held, (v.abilities P).drop = true)
    (hl : ∀ v, some v ∈ fr.locals.toList → (v.abilities P).drop = true) {st : GlobalStorage}
    (hst : st = s.storage) {evs : List Event}
    (hevs : ∀ e ∈ evs, e.Silent = true ∧ Encapsulation.structsRespected π A e = true) :
    StepGhost σ P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh (.returned fr.stack.reverse st)
      (.returned g.stack.reverse h.storage) evs := by
  have hfit := hp.fits
  rw [shadow_fits, hsf, hhf, Bool.and_eq_true] at hfit
  obtain ⟨hfr, hext⟩ := hfit
  rw [framesFit_cons, Bool.and_eq_true] at hfr
  have hg := hfr.1
  have hfa := frame_stack_fits hg
  simp only [extFit, Bool.and_eq_true] at hext
  have hsil := silent_of fun e he => (hevs e he).1
  subst hst
  refine ⟨?_, ?_, ?_, ?_, ?_, trivial, createdPlain_of hsil.1, fun d r hm => by
    simp [postFrames] at hm, refsPerFrame_of_ne (by simp [postPairs, hhf, hsf]),
    closures_silent fun e he => (hevs e he).1, ClosuresKept.of_sub fun x hx _ => ?_⟩
  rotate_right
  · simp only [ShadowPost.occurrences, List.mem_append] at hx
    rw [shadow_occurrences, hsf, hhf, framesOcc_cons, frame_occurrences, extOcc]
    simp only [List.mem_append]
    rcases hx with hx | hx
    · exact Or.inl (Or.inl (Or.inr ((occurrencesAll_reverse hfa).subset hx)))
    · exact Or.inr (Or.inl hx)
  · simp [ShadowPost.fits, fitsAll_reverse hfa, hext.1]
  · show Effect P _ (ShadowPost.occurrences _ _) _ _ _
    have he := Effect.drop (P := P) (f := h.fresh) (f' := h.fresh)
      (d := localsOcc g fr ++ IdTree.occurrencesAll h.held s.held)
      (fun o ho => by
        rcases List.mem_append.1 ho with ho | ho
        · exact localsOcc_drop hg hl o ho
        · exact heldOcc_drop hd hext.2 o ho)
      (IdTree.occurrencesAll g.stack fr.stack ++
        IdTree.occurrencesAll h.storage (s.storage.map (·.value))) (Nat.le_refl _)
    have he' := he.perm (pre₂ := h.occurrences s)
      (post₂ := ShadowPost.occurrences (.returned g.stack.reverse h.storage)
        (.returned fr.stack.reverse s.storage)) ?_ ?_
    · exact EffectK.of_silent he' fun e he => (hevs e he).1
    · rw [shadow_occurrences, hsf, hhf, framesOcc_cons, frame_occurrences, extOcc]
      simp only [framesOcc, List.zip_nil_left, List.flatMap_nil, List.append_nil]
      perm_parts
    · simp only [ShadowPost.occurrences]
      exact (occurrencesAll_reverse hfa).append_right _
  · intro i w c he
    exact absurd (consumed_mem he) (by simp [hsil.2.1])
  · intro e he
    exact ⟨(hevs e he).2, signers_respected (hevs e he).1 auth⟩
  · intro x hx
    simp [hsil.2.2] at hx

/-- `Ret` to a caller: the segment's values move onto the caller's segment;
the locals' values, which have `drop`, are discarded. -/
theorem ret_caller_ghost {s s₂ : State} {h : Shadow} {fr caller : Frame} {rest' : List Frame}
    {g cg : ShadowFrame} {hs' : List ShadowFrame} (hp : Pre σ P s h)
    (hsf : s.frames = fr :: caller :: rest') (hhf : h.frames = g :: cg :: hs')
    (hl : ∀ v, some v ∈ fr.locals.toList → (v.abilities P).drop = true)
    (hst : s₂.storage = s.storage) (hhe : s₂.held = s.held) (hnf : s₂.nextFrame = s.nextFrame)
    {evs : List Event}
    (hevs : ∀ e ∈ evs, e.Silent = true ∧ Encapsulation.structsRespected π A e = true) :
    StepGhost σ P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh
      (.running { s₂ with frames := { caller with stack := fr.stack ++ caller.stack, pc := caller.pc + 1 } :: rest' })
      (.running { h with frames := { cg with stack := g.stack ++ cg.stack } :: hs' }) evs := by
  have hfit := hp.fits
  rw [shadow_fits, hsf, hhf, Bool.and_eq_true] at hfit
  obtain ⟨hfr, hext⟩ := hfit
  rw [framesFit_cons, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfr
  obtain ⟨hg, hcg, hrest⟩ := hfr
  have hfa := frame_stack_fits hg
  have hca := frame_stack_fits hcg
  have happ : IdTree.fitsAll (g.stack ++ cg.stack) (fr.stack ++ caller.stack) = true := by
    rw [fitsAll_append (fitsAll_length hfa)]; simp [hfa, hca]
  have hsil := silent_of fun e he => (hevs e he).1
  have hrefs : ∀ x, x ∈ postFrames
      (.running { h with frames := { cg with stack := g.stack ++ cg.stack } :: hs' })
      (.running { s₂ with frames := { caller with stack := fr.stack ++ caller.stack, pc := caller.pc + 1 } :: rest' }) →
      x ∈ framesOcc h.frames s.frames := by
    intro x hm
    simp only [postFrames, framesOcc_cons, frame_occurrences, localsOcc_stack,
      occurrencesAll_append (fitsAll_length hfa), List.mem_append] at hm
    rw [hhf, hsf, framesOcc_cons, framesOcc_cons, frame_occurrences, frame_occurrences]
    simp only [List.mem_append]
    rcases hm with (hm | hm | hm) | hm
    · exact Or.inr (Or.inl (Or.inl hm))
    · exact Or.inl (Or.inr hm)
    · exact Or.inr (Or.inl (Or.inr hm))
    · exact Or.inr (Or.inr hm)
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, createdPlain_of hsil.1, fun d r hm => Or.inl (hrefs _ hm),
    refsPerFrame_of_ne ?_, closures_silent fun e he => (hevs e he).1,
    kept_frames rfl rfl hst hhe hrefs⟩
  rotate_right
  · simp only [postPairs, hhf, hsf, List.zip_cons_cons, List.length_cons]
    omega
  · simp only [ShadowPost.fits, shadow_fits, framesFit_cons, frame_fits_stack hcg happ,
      hrest, Bool.and_self, Bool.true_and]
    exact (extFit_congr (h := h) (s := s) rfl rfl hst hhe).trans hext
  · show Effect P _ (Shadow.occurrences _ _) _ _ _
    have he := (Effect.drop (P := P) (f := h.fresh) (f' := h.fresh) (localsOcc_drop hg hl)
      (IdTree.occurrencesAll g.stack fr.stack ++ framesOcc (cg :: hs') (caller :: rest') ++
        extOcc h s) (Nat.le_refl _))
    have hx : extOcc { h with frames := { cg with stack := g.stack ++ cg.stack } :: hs' }
        { s₂ with frames := { caller with stack := fr.stack ++ caller.stack, pc := caller.pc + 1 } ::
          rest' } = extOcc h s := extOcc_congr rfl rfl hst hhe
    have he' := he.perm (pre₂ := h.occurrences s)
      (post₂ := Shadow.occurrences { h with frames := { cg with stack := g.stack ++ cg.stack } :: hs' }
        { s₂ with frames := { caller with stack := fr.stack ++ caller.stack, pc := caller.pc + 1 } ::
          rest' }) ?_ ?_
    · exact EffectK.of_silent he' fun e he => (hevs e he).1
    · rw [shadow_occurrences, hsf, hhf, framesOcc_cons, frame_occurrences]
      simp [List.append_assoc]
    · rw [shadow_occurrences, framesOcc_cons, framesOcc_cons, frame_occurrences, frame_occurrences,
        localsOcc_stack, hx, occurrencesAll_append (fitsAll_length hfa)]
      perm_parts
  · intro i w c he
    exact absurd (consumed_mem he) (by simp [hsil.2.1])
  · intro e he
    exact ⟨(hevs e he).2, signers_respected (hevs e he).1 auth⟩
  · intro x hx
    simp [hsil.2.2] at hx
  · have hinv := hp.inv
    refine ⟨?_, ?_, ?_, ?_, ?_⟩
    · have := hinv.frameIds
      rw [hsf] at this
      simp only [List.map_cons, List.nodup_cons] at this ⊢
      exact this.2
    · intro x hx
      rw [hnf]
      simp only [List.mem_cons] at hx
      rcases hx with rfl | hx
      · exact hinv.framesBelow caller (by rw [hsf]; simp)
      · exact hinv.framesBelow x (by rw [hsf]; simp [hx])
    · rw [hst]; exact hinv.resources
    · rw [hst]; exact hinv.typed
    · rw [hhe]; exact hinv.heldDrop

end Calls

end MoveBytecode.Proofs
