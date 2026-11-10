-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefModel

/-!
# The reference invariant of executions with references

UNTRUSTED (checked by the kernel). What D1 needs beyond the type and
availability invariant `Inv` once references exist (`RefInvariant`): frame
identities are distinct and fresh, global storage is well typed, the active
counts are the frames counted as activations (R3), the current frame agrees
with the reference certificate before its instruction given its reference
parameters, and each suspended frame agrees with the certificate's state at
its call.

A suspended frame's references stay valid while the frames above it run:
none of them is strictly inside a mutable reference it passed down
(`CallFacts.refs`), and those frames change only their own locals, locations
inside the mutable references they received (`AllowedBy`), which are inside
the suspended frame's locals, its own global borrows or mutable references it
received in turn (`CallFacts.exposure`), and global storage of a type that
no frame below protects. A frame protects the global types it and the frames
below borrowed (`GlobOk`): the frames above acquire them only in a module
active again, where R3 makes global operations fail.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

/-! ## Activations -/

/-- A frame counts as an activation of its module if it is the first frame
or its caller runs in another module. A closure's frame counts even when its
caller runs in the same module (R3). -/
def CountedOk (P : Program) : List (FunctionRef × Bool) → Prop
  | [] => True
  | [p] => p.2 = true
  | p :: q :: rest => (funModule P p.1 ≠ funModule P q.1 → p.2 = true) ∧ CountedOk P (q :: rest)

/-- **The active counts are the counted frames** (R3). -/
structure ActiveCounts (P : Program) (s : State) : Prop where
  count : ∀ M, s.activeCount M = countActive P (s.frames.map Frame.skel) M
  counted : CountedOk P (s.frames.map Frame.skel)

theorem ActiveCounts.skel {P : Program} {s s' : State} (h : ActiveCounts P s)
    (hsk : s'.frames.map Frame.skel = s.frames.map Frame.skel) (hact : s'.active = s.active) :
    ActiveCounts P s' :=
  ⟨fun M => by
    rw [hsk, ← h.count M]
    simp only [State.activeCount, hact], by rw [hsk]; exact h.counted⟩

/-! ## Global storage -/

/-- **The global types a frame protects**, running `f` over the frames `sk`
(it, then those below): `S` holds the types that frames below borrowed
globally before their calls, which this frame acquires only in a module
active again, and the type of each of its reference parameters into global
storage. -/
structure GlobOk (P : Program) (f : FunctionRef) (sk : List (FunctionRef × Bool))
    (ps : List Reference) (S : StructId → Prop) : Prop where
  active : ∀ t, S t → 1 ≤ countActive P sk t.module
  acq : ∀ t, S t → acqTypes P f t → 2 ≤ countActive P sk t.module
  params : ∀ π ∈ ps, ∀ a t, π.location.root = .global a t → S t

/-- **The locations the frames from `cid` up may invalidate**: their own
locals, locations strictly inside the mutable references `ps` the frame `cid`
received, and global storage of a type that the frames below do not protect
(`S`). -/
def AllowedBy (cid : FrameId) (ps : List Reference) (S : StructId → Prop) (e : Invalidation) :
    Prop :=
  ∀ l : Location, e.covers l = true →
    (∃ f i, l.root = .local_ f i ∧ cid ≤ f) ∨
    (∃ π ∈ ps, π.mutable = true ∧ l.StrictlyInside π.location) ∨
    ∃ a t, l.root = .global a t ∧ ¬ S t

/-- **What a step of frame `fr` may invalidate**, whatever global types the
frames below protect. -/
def AllowedStep (P : Program) (fr : Frame) (rest : List Frame) (ps : List Reference)
    (es : List Invalidation) : Prop :=
  ∀ S, GlobOk P fr.function ((fr :: rest).map Frame.skel) ps S → ∀ e ∈ es, AllowedBy fr.id ps S e

/-- The references among a call's arguments, in order. -/
def argRefs (args : List Value) : List Reference :=
  args.filterMap fun v => match v with
    | .ref r => some r
    | _ => none

theorem allowedBy_nil {P : Program} {fr : Frame} {rest : List Frame} {ps : List Reference} :
    AllowedStep P fr rest ps [] := fun _ _ _ h => by simp at h

/-- What a suspended caller's call is, as the analysis and the certificates
see it: the analysis's core of the call (`CoreRun`, on the state at the call
for `Call`, once the closure is popped for `CallClosure`) from `A` to `A'`,
with result tokens `rs` of module `m`, its state `A''` after the call, the
arguments `args` the caller popped, and the caller's frame before it popped
them; the callee's reference parameters `cps` are among the arguments' (a
closure's captured arguments are values); `gsid` names the caller's global
borrows, and `Sc` is what the callee protects. -/
structure CallFacts (P : Program) (s : State) (caller : Frame) (ps : List Reference)
    (gsid : Nat → Option StructId) (cfn : FunctionRef) (cps : List Reference)
    (Sc : StructId → Prop) (m : CompiledModule) (rs : List SignatureToken)
    (rets : List RuntimeType) (args : List Value) (A A' A'' : RefState)
    (L : Nat → Option Reference) : Prop where
  retTypes : ∃ τ : List RuntimeType, (∀ x ∈ τ, x.isValue = true) ∧
    RuntimeType.ofSignatureWith m τ rs = some rets
  calleeRets : P.returnTypes cfn = some rets
  core : CoreRun A A' args.length rs
  after : modelRef P caller.function (caller.pc + 1) = some A''
  le : RefLe A' A''
  cpsSub : cps.Sublist (argRefs args)
  cpsBelow : ∀ π ∈ cps, RootBelow caller.id π.location.root
  cpsArgs : ∀ π ∈ argRefs args, ∃ x, AbsValue.reference x ∈ A.stack.take args.length ∧
    L x = some π
  distinct : (heldIds A).Nodup
  desc : Desc A { caller with stack := args.reverse ++ caller.stack } gsid ps L
  height : A.stack.length = args.length + caller.stack.length
  refs : ∀ x r, x ∈ heldIds { A with stack := A.stack.drop args.length } → L x = some r →
    RefOk P s caller r ∧ ∀ π ∈ cps, π.mutable = true → ¬ r.location.StrictlyInside π.location
  exposure : ∀ π ∈ cps, π.mutable = true →
    (∃ i, π.location.root = .local_ caller.id i) ∨
    (∃ a d t, gsid d = some t.erase ∧ π.location.root = .global a t) ∨
    ∃ π' ∈ ps, π'.mutable = true ∧ π.location.Inside π'.location
  globals : ∀ x r, x ∈ heldIds A → L x = some r → ∀ a t, r.location.root = .global a t → Sc t
  /-- Each value the caller keeps on its operand stack conforms to a slot
  type: a value type, or a reference type. -/
  stackConf : ∀ v ∈ caller.stack, ∃ t, v.conforms P t = true ∧
    (t.isValue = true ∨ ∃ b u, t = .ref b u)
  /-- An immutable reference the caller keeps is not at the location of a
  mutable reference it passed: the analysis checked that one writable. -/
  immSame : ∀ x r, x ∈ heldIds { A with stack := A.stack.drop args.length } → L x = some r →
    r.mutable = false → ∀ π ∈ cps, π.mutable = true → r.location ≠ π.location
  /-- The references the caller keeps on its operand stack, and in the locals
  the availability analysis has available, are tracked. -/
  tracked : ∀ r, (Value.ref r ∈ caller.stack ∨ ∃ j, caller.locals[j]? = some (some (.ref r)) ∧
      modelAvail P caller.function caller.pc j = .available) →
    ∃ x ∈ heldIds { A with stack := A.stack.drop args.length }, L x = some r

/-- A **suspended caller**, at a call, whose callee has identity `cid`,
function `cfn`, reference parameters `cps` and protects `Sc`; the caller,
over the frames `rest`, has reference parameters `ps` and protects `S`. -/
structure SuspOk (P : Program) (s : State) (caller : Frame) (rest : List Frame)
    (ps : List Reference) (S : StructId → Prop) (cid : FrameId) (cfn : FunctionRef)
    (cps : List Reference) (Sc : StructId → Prop) : Prop where
  order : caller.id < cid
  locals : LocalsOk P caller
  glob : GlobOk P caller.function ((caller :: rest).map Frame.skel) ps S
  sub : ∀ t, S t → Sc t
  call : ∃ (m : CompiledModule) (rs : List SignatureToken) (rets : List RuntimeType)
      (args : List Value) (A A' A'' : RefState) (L : Nat → Option Reference),
    CallFacts P s caller ps (gsidOf P ((caller :: rest).map Frame.skel)) cfn cps Sc m rs rets
      args A A' A'' L

/-- The suspended frames resume in states the reference certificate covers. -/
def ChainRef (P : Program) (s : State) :
    FrameId → FunctionRef → List Reference → (StructId → Prop) → List Frame → Prop
  | _, _, _, _, [] => True
  | cid, cfn, cps, Sc, caller :: rest =>
    ∃ ps S, SuspOk P s caller rest ps S cid cfn cps Sc ∧ ChainRef P s caller.id caller.function ps S rest

/-- **The frames below the current one**: what the current frame protects,
and the suspended frames. -/
def Below (P : Program) (s : State) (fr : Frame) (ps : List Reference) (rest : List Frame) :
    Prop :=
  ∃ S, GlobOk P fr.function ((fr :: rest).map Frame.skel) ps S ∧
    ChainRef P s fr.id fr.function ps S rest

/-! ## Active counts across calls and returns -/

theorem countActive_cons (P : Program) (p : FunctionRef × Bool) (sk : List (FunctionRef × Bool))
    (M : ModuleId) :
    countActive P (p :: sk) M =
      (if p.2 = true ∧ funModule P p.1 = some M then 1 else 0) + countActive P sk M := by
  unfold countActive
  rw [List.filter_cons]
  by_cases h : p.2 = true ∧ funModule P p.1 = some M
  · rw [if_pos (by simp [h.1, h.2]), if_pos h, List.length_cons]; omega
  · rw [if_neg (by simp only [Bool.and_eq_true, beq_iff_eq]; exact h), if_neg h]; omega

theorem find?_filter_of {α : Type} {p q : α → Bool} (hpq : ∀ a, p a = true → q a = true) :
    ∀ l : List α, (l.filter q).find? p = l.find? p
  | [] => rfl
  | a :: l => by
    rw [List.filter_cons]
    by_cases hp : p a = true
    · rw [if_pos (hpq a hp), List.find?_cons_of_pos hp, List.find?_cons_of_pos hp]
    · rw [List.find?_cons_of_neg hp]
      split
      · rw [List.find?_cons_of_neg hp]; exact find?_filter_of hpq l
      · exact find?_filter_of hpq l

theorem activeCount_filter (s : State) {id M : ModuleId} (h : M ≠ id) :
    ((s.active.filter fun e => e.1 != id).find? fun e => e.1 == M) =
      s.active.find? fun e => e.1 == M :=
  find?_filter_of (fun a ha => by simp only [beq_iff_eq] at ha; simp [ha, h]) s.active

theorem activeCount_incActive (s : State) (id M : ModuleId) :
    (s.incActive id).activeCount M = if M = id then s.activeCount id + 1 else s.activeCount M := by
  by_cases h : M = id
  · subst h
    simp [State.incActive, State.activeCount]
  · rw [if_neg h]
    have hne : ((id, s.activeCount id + 1).1 == M) = false := by simpa using Ne.symm h
    simp only [State.incActive, State.activeCount, List.find?_cons, hne]
    rw [activeCount_filter s h]

theorem activeCount_decActive (s : State) (id M : ModuleId) :
    (s.decActive id).activeCount M = if M = id then s.activeCount id - 1 else s.activeCount M := by
  have hnone : ((s.active.filter fun e => e.1 != id).find? fun e => e.1 == id) = none := by
    rw [List.find?_eq_none]
    intro e he
    simp only [List.mem_filter, bne_iff_ne, ne_eq] at he
    simpa using he.2
  by_cases hle : s.activeCount id ≤ 1
  · have e : (s.decActive id).active = s.active.filter fun e => e.1 != id := by
      simp [State.decActive, hle]
    by_cases h : M = id
    · subst h
      rw [if_pos rfl]
      simp only [State.activeCount] at hle ⊢
      rw [e, hnone]
      simp only
      omega
    · rw [if_neg h]
      simp only [State.activeCount]
      rw [e, activeCount_filter s h]
  · have e : (s.decActive id).active =
        (id, s.activeCount id - 1) :: s.active.filter fun e => e.1 != id := by
      simp [State.decActive, hle]
    by_cases h : M = id
    · subst h
      rw [if_pos rfl]
      simp only [State.activeCount]
      rw [e]
      simp only [List.find?_cons, beq_self_eq_true]
      rfl
    · rw [if_neg h]
      have hne : ((id, s.activeCount id - 1).1 == M) = false := by simpa using Ne.symm h
      simp only [State.activeCount]
      rw [e, List.find?_cons, hne]
      simp only
      rw [activeCount_filter s h]

/-- **Entering a callee** keeps the active counts: a callee of another module,
or a closure's, counts as an activation. -/
theorem ActiveCounts.push {P : Program} {s s' : State} (h : ActiveCounts P s) {fr fr1 frame : Frame}
    {rest : List Frame} (hsf : s.frames = fr :: rest) (hs'f : s'.frames = frame :: fr1 :: rest)
    (hsk : fr1.skel = fr.skel) {id cid : ModuleId} (hfm : funModule P fr.function = some id)
    (hcm : funModule P frame.function = some cid) (hcross : cid ≠ id → frame.countedActive = true)
    (hact : s'.active = (if frame.countedActive then s.incActive cid else s).active) :
    ActiveCounts P s' := by
  have hcnt := h.count
  have hco := h.counted
  rw [hsf] at hcnt hco
  refine ⟨fun M => ?_, ?_⟩
  · rw [hs'f, List.map_cons, List.map_cons, hsk, countActive_cons, ← List.map_cons, ← hcnt M]
    simp only [Frame.skel, hcm]
    cases hc : frame.countedActive
    · simp only [State.activeCount, hact, hc, Bool.false_eq_true, if_false, false_and]; omega
    · have e : State.activeCount s' M = (s.incActive cid).activeCount M := by
        simp only [State.activeCount, hact, hc, if_true]
      rw [e, activeCount_incActive]
      by_cases hM : M = cid
      · subst hM; simp <;> omega
      · rw [if_neg hM, if_neg (fun h' => hM (Option.some.inj h'.2).symm)]; omega
  · rw [hs'f, List.map_cons, List.map_cons, hsk]
    refine ⟨fun hne => hcross fun e => hne ?_, hco⟩
    simp only [Frame.skel, hcm, hfm, e]

/-- **Returning** keeps the active counts: a callee of another module stops
counting as an activation. -/
theorem ActiveCounts.pop {P : Program} {s s' : State} (h : ActiveCounts P s) {fr caller caller' : Frame}
    {rest : List Frame} (hsf : s.frames = fr :: caller :: rest) (hs'f : s'.frames = caller' :: rest)
    (hsk : caller'.skel = caller.skel) {id : ModuleId} (hfm : funModule P fr.function = some id)
    (hact : s'.active = (if fr.countedActive then s.decActive id else s).active) :
    ActiveCounts P s' := by
  have hcnt := h.count
  have hco := h.counted
  rw [hsf] at hcnt hco
  refine ⟨fun M => ?_, ?_⟩
  · have hM := hcnt M
    rw [List.map_cons, countActive_cons] at hM
    rw [hs'f, List.map_cons, hsk]
    simp only [Frame.skel, hfm] at hM
    cases hc : fr.countedActive
    · simp only [hc, Bool.false_eq_true, false_and, if_false] at hM
      have e : State.activeCount s' M = s.activeCount M := by
        simp only [State.activeCount, hact, hc, Bool.false_eq_true, if_false]
      rw [e]
      simpa [List.map_cons] using hM
    · have e : State.activeCount s' M = (s.decActive id).activeCount M := by
        simp only [State.activeCount, hact, hc, if_true]
      rw [e, activeCount_decActive]
      simp only [hc, true_and] at hM
      by_cases hMid : M = id
      · subst hMid
        simp only [if_true] at hM ⊢
        rw [List.map_cons] at hM
        omega
      · rw [if_neg hMid]
        rw [if_neg (fun h' => hMid (Option.some.inj h').symm)] at hM
        rw [List.map_cons] at hM
        omega
  · rw [hs'f, List.map_cons, hsk]
    simp only [List.map_cons] at hco
    exact hco.2

/-! ## The targets of the references to whole held values -/

/-- **One target per whole held value**: every reference of the frames to the
whole of held value `k` has target `τ k`. The transaction lends a held value
by one reference at most, which execution copies, moves and freezes but never
borrows anew; a write through any of them stores a value of a type assignable
to `τ k`, which the others then designate. -/
def HeldTargets (s : State) : Prop :=
  ∃ τ : Nat → RuntimeType, ∀ g ∈ s.frames, ∀ r, Value.ref r ∈ frameVals g →
    ∀ k, r.location = ⟨.held k, []⟩ → r.target = τ k

/-- Every reference of frame `g'` to the whole of a held value has the
location and target of one of frame `g`'s. -/
def RefsFrom (g g' : Frame) : Prop :=
  ∀ r, Value.ref r ∈ frameVals g' → ∀ k, r.location = ⟨.held k, []⟩ →
    ∃ r₀, Value.ref r₀ ∈ frameVals g ∧ r₀.location = r.location ∧ r₀.target = r.target

/-- Every reference of the frames of `s'` to the whole of a held value has the
location and target of one of the frames of `s`. -/
theorem HeldTargets.of {s s' : State} (h : HeldTargets s)
    (hf : ∀ g' ∈ s'.frames, ∀ r, Value.ref r ∈ frameVals g' → ∀ k, r.location = ⟨.held k, []⟩ →
      ∃ g ∈ s.frames, ∃ r₀, Value.ref r₀ ∈ frameVals g ∧ r₀.location = r.location ∧
        r₀.target = r.target) :
    HeldTargets s' := by
  obtain ⟨τ, hτ⟩ := h
  refine ⟨τ, fun g' hg' r hr k hk => ?_⟩
  obtain ⟨g, hg, r₀, hr₀, hloc, htgt⟩ := hf g' hg' r hr k hk
  rw [← htgt]
  exact hτ g hg r₀ hr₀ k (by rw [hloc, hk])

/-- A step of the current frame alone. -/
theorem HeldTargets.top {s s' : State} (h : HeldTargets s) {fr fr' : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) (hs'f : s'.frames = fr' :: rest) (hf : RefsFrom fr fr') :
    HeldTargets s' := by
  refine h.of fun g' hg' r hr k hk => ?_
  rw [hs'f] at hg'
  rcases List.mem_cons.1 hg' with rfl | hg'
  · obtain ⟨r₀, h₀, h₁, h₂⟩ := hf r hr k hk
    exact ⟨fr, by rw [hsf]; exact List.mem_cons_self, r₀, h₀, h₁, h₂⟩
  · exact ⟨g', by rw [hsf]; exact List.mem_cons_of_mem _ hg', r, hr, rfl, rfl⟩

theorem RefsFrom.refl (g : Frame) : RefsFrom g g := fun r hr _ _ => ⟨r, hr, rfl, rfl⟩

theorem RefsFrom.trans {g₁ g₂ g₃ : Frame} (h₁ : RefsFrom g₁ g₂) (h₂ : RefsFrom g₂ g₃) :
    RefsFrom g₁ g₃ := by
  intro r hr k hk
  obtain ⟨r₂, hr₂, hl₂, ht₂⟩ := h₂ r hr k hk
  obtain ⟨r₁, hr₁, hl₁, ht₁⟩ := h₁ r₂ hr₂ k (by rw [hl₂, hk])
  exact ⟨r₁, hr₁, hl₁.trans hl₂, ht₁.trans ht₂⟩

/-- Value `v`, if a reference to the whole of a held value, has the location
and target of a reference of frame `g`. -/
def ValFrom (g : Frame) (v : Value) : Prop :=
  ∀ r, v = .ref r → ∀ k, r.location = ⟨.held k, []⟩ →
    ∃ r₀, Value.ref r₀ ∈ frameVals g ∧ r₀.location = r.location ∧ r₀.target = r.target

theorem ValFrom.of_mem {g : Frame} {v : Value} (h : v ∈ frameVals g) : ValFrom g v :=
  fun r hr _ _ => ⟨r, hr ▸ h, rfl, rfl⟩

/-- A value that is no reference. -/
theorem ValFrom.of_not_ref {g : Frame} {v : Value} (h : ∀ r, v ≠ .ref r) : ValFrom g v :=
  fun r hr => absurd hr (h r)

/-- A reference into a local, a resource, or strictly inside a value. -/
theorem ValFrom.of_loc {g : Frame} {r : Reference} (h : ∀ k, r.location ≠ ⟨.held k, []⟩) :
    ValFrom g (.ref r) := by
  intro r' hr' k hk
  cases hr'
  exact absurd hk (h k)

/-- A reference with the location and target of one of `g`'s. -/
theorem ValFrom.of_same {g : Frame} {r r₀ : Reference} (h : Value.ref r₀ ∈ frameVals g)
    (hl : r.location = r₀.location) (ht : r.target = r₀.target) : ValFrom g (.ref r) := by
  intro r' hr' _ _
  cases hr'
  exact ⟨r₀, h, hl.symm, ht.symm⟩

/-- **Every value of `g'` has its references to whole held values from `g`.** -/
theorem RefsFrom.of_vals {g g' : Frame} (h : ∀ v ∈ frameVals g', ValFrom g v) :
    RefsFrom g g' :=
  fun r hr k hk => h _ hr r rfl k hk

theorem frameVals_advance (g : Frame) : frameVals (Step.advance g) = frameVals g := rfl

theorem mem_frameVals {g : Frame} {v : Value} :
    v ∈ frameVals g ↔ v ∈ g.stack ∨ ∃ i : Nat, g.locals[i]? = some (some v) := by
  simp only [frameVals, List.mem_append, List.mem_filterMap, id]
  refine or_congr Iff.rfl ⟨fun ⟨o, ho, he⟩ => ?_, fun ⟨i, hi⟩ => ?_⟩
  · subst he
    obtain ⟨i, hi, he⟩ := List.getElem_of_mem ho
    exact ⟨i, by rw [← Array.getElem?_toList, List.getElem?_eq_some_iff]; exact ⟨hi, he⟩⟩
  · exact ⟨some v, by rw [← Array.getElem?_toList] at hi; exact List.mem_of_getElem? hi, rfl⟩

theorem mem_setLocals {a : Array (Option Value)} {i : Nat} {o : Option Value} {v : Value}
    (h : v ∈ (a.setIfInBounds i o).toList.filterMap id) :
    o = some v ∨ v ∈ a.toList.filterMap id := by
  simp only [List.mem_filterMap, id] at h ⊢
  obtain ⟨x, hx, rfl⟩ := h
  rw [Array.toList_setIfInBounds] at hx
  rcases List.mem_or_eq_of_mem_set hx with hx | hx
  · exact Or.inr ⟨_, hx, rfl⟩
  · exact Or.inl hx.symm

/-- **The frame after popping the top `n`, pushing `new` and setting local
`i` to `o`.** -/
theorem RefsFrom.frame {g g' : Frame} {new : List Value} {n i : Nat} {o : Option Value}
    (hstk : g'.stack = new ++ g.stack.drop n) (hloc : g'.locals = g.locals.setIfInBounds i o)
    (hnew : ∀ v ∈ new, ValFrom g v) (ho : ∀ v, o = some v → ValFrom g v) :
    RefsFrom g g' := by
  refine RefsFrom.of_vals fun v hv => ?_
  simp only [frameVals, hstk, hloc, List.mem_append] at hv
  rcases hv with (hv | hv) | hv
  · exact hnew v hv
  · exact .of_mem (mem_frameVals_stack (List.mem_of_mem_drop hv))
  · rcases mem_setLocals hv with hv | hv
    · exact ho v hv
    · exact .of_mem (by simp only [frameVals, List.mem_append]; exact Or.inr hv)

/-- **The frame after popping the top `n` and pushing `new`**, the locals
kept. -/
theorem RefsFrom.next {g g' : Frame} {new : List Value} {n : Nat}
    (hstk : g'.stack = new ++ g.stack.drop n) (hloc : g'.locals = g.locals)
    (hnew : ∀ v ∈ new, ValFrom g v) :
    RefsFrom g g' := by
  refine RefsFrom.of_vals fun v hv => ?_
  simp only [frameVals, hstk, hloc, List.mem_append] at hv
  rcases hv with (hv | hv) | hv
  · exact hnew v hv
  · exact .of_mem (mem_frameVals_stack (List.mem_of_mem_drop hv))
  · exact .of_mem (by simp only [frameVals, List.mem_append]; exact Or.inr hv)

/-- **A frame entered** with arguments from the operand stack of `g`. -/
theorem RefsFrom.enter {g g' : Frame} {args : List Value} {k : Nat}
    (hstk : g'.stack = []) (hloc : g'.locals = (args.map some ++ List.replicate k none).toArray)
    (hargs : ∀ v ∈ args, v ∈ g.stack) : RefsFrom g g' := by
  refine RefsFrom.of_vals fun v hv => ?_
  simp only [frameVals, hstk, hloc, List.nil_append, List.mem_filterMap, id,
    List.toList_toArray, List.mem_append, List.mem_map, List.mem_replicate] at hv
  obtain ⟨o, ho, ho'⟩ := hv
  rcases ho with ⟨w, hw, rfl⟩ | ⟨-, ho⟩
  · simp only [Option.some.injEq] at ho'
    rw [← ho']
    exact .of_mem (mem_frameVals_stack (hargs w hw))
  · subst ho; cases ho'

/-- **The reference invariant.** -/
structure RefInvariant (P : Program) (s : State) : Prop where
  ids : (s.frames.map (·.id)).Nodup
  below : ∀ fr ∈ s.frames, fr.id < s.nextFrame
  storage : ∀ r ∈ s.storage, r.value.conforms P (.struct r.type) = true
  refs : ∃ fr rest ps, s.frames = fr :: rest ∧
    (∃ A, modelRef P fr.function fr.pc = some A ∧ FrameOk P s fr ps A) ∧ Below P s fr ps rest
  active : ActiveCounts P s
  heldTargets : HeldTargets s

/-- **The invariant of executions with references**: types, availability and
references. -/
def InvRefs (P : Program) (s : State) : Prop :=
  Inv P (modelTypes P) (modelAvail P) s ∧ RefInvariant P s

/-! ## Results that keep a predicate -/

/-- A running outcome satisfies `R`. -/
def OutRuns (R : State → Prop) : Outcome → Prop
  | .running s => R s
  | _ => True

/-- Every running state a step result leads to satisfies `R`. -/
def ExecRuns (P : Program) (N : NativeRel) (R : State → Prop) : Step.Exec StepResult → Prop
  | .ok (.done o) => OutRuns R o
  | .ok (.callNative s f args views) => ∀ r, N f views r → OutRuns R (resumeNative P s f args r)
  | .error o => OutRuns R o

theorem Safe.and {Q R : State → Prop} {o : Outcome} (h1 : Safe Q o) (h2 : OutRuns R o) :
    Safe (fun s => Q s ∧ R s) o := by
  cases o with
  | running s => exact ⟨h1, h2⟩
  | returned _ _ => exact h1
  | failed _ => exact h1
  | wrong _ => exact h1

theorem ExecSafe.and {P : Program} {N : NativeRel} {Q R : State → Prop}
    {x : Step.Exec StepResult} (h1 : ExecSafe P N Q x) (h2 : ExecRuns P N R x) :
    ExecSafe P N (fun s => Q s ∧ R s) x := by
  match x, h1, h2 with
  | .ok (.done o), h1, h2 => exact Safe.and h1 h2
  | .ok (.callNative s f args views), h1, h2 => exact fun r hr => Safe.and (h1 r hr) (h2 r hr)
  | .error o, h1, h2 => exact Safe.and h1 h2

/-! ## The suspended frames persist -/

theorem StrictlyInside.trans_inside {a b c : Location} (h1 : a.StrictlyInside b)
    (h2 : b.Inside c) : a.StrictlyInside c := by
  obtain ⟨⟨hr1, hp1⟩, hne⟩ := h1
  obtain ⟨hr2, hp2⟩ := h2
  refine ⟨⟨hr1.trans hr2, hp2.trans hp1⟩, fun e => hne ?_⟩
  have hl1 := hp1.length_le
  have hl2 := hp2.length_le
  rw [e] at hl1
  exact (hp1.eq_of_length (by rw [e] at hp1 ⊢; omega)).symm

theorem heldIds_drop {A : RefState} {n x : Nat}
    (h : x ∈ heldIds { A with stack := A.stack.drop n }) : x ∈ heldIds A := by
  rcases mem_heldIds.1 h with h | h
  · exact mem_heldIds.2 (Or.inl h)
  · exact mem_heldIds.2 (Or.inr (List.mem_of_mem_drop h))

/-- **What the analysis names for a frame**: types its function acquires,
while its module is active once. -/
theorem gsidOf_cons {P : Program} {f : FunctionRef} {b : Bool} {sk : List (FunctionRef × Bool)}
    {d : Nat} {t : StructId} (h : gsidOf P ((f, b) :: sk) d = some t.erase) :
    acqTypes P f t ∧ countActive P ((f, b) :: sk) t.module ≤ 1 := by
  simp only [gsidOf] at h
  split at h
  · next M hM =>
    split at h
    · next hc =>
      obtain ⟨hacq, hcnt⟩ := hc
      refine ⟨⟨d, hacq, h⟩, ?_⟩
      simp only [funModule, Option.map_eq_some_iff] at hM
      obtain ⟨info, hinfo, rfl⟩ := hM
      rw [show t.module = t.erase.module from rfl, definedStruct_module h hinfo]
      exact hcnt
    · cases h
  · cases h

/-- **One level down**: what the frames from `cid` up may invalidate covers
none of a suspended caller's references, and the frames from the caller up
may invalidate it too. -/
theorem SuspOk.allowed {P : Program} {s : State} {caller : Frame} {rest : List Frame}
    {ps : List Reference} {S : StructId → Prop} {cid : FrameId} {cfn : FunctionRef}
    {cps : List Reference} {Sc : StructId → Prop}
    (h : SuspOk P s caller rest ps S cid cfn cps Sc) {e : Invalidation}
    (he : AllowedBy cid cps Sc e) : AllowedBy caller.id ps S e := by
  obtain ⟨m, rs, rets, args, A, A', A'', L, hf⟩ := h.call
  have hexp := hf.exposure
  intro l hl
  rcases he l hl with ⟨f, i, hr, hf⟩ | ⟨π, hπ, hm, hin⟩ | ⟨a, t, hr, hn⟩
  · exact Or.inl ⟨f, i, hr, Nat.le_of_lt (Nat.lt_of_lt_of_le h.order hf)⟩
  · rcases hexp π hπ hm with ⟨i, hr⟩ | ⟨a, d, t, hd, hr⟩ | ⟨π', hπ', hm', hin'⟩
    · exact Or.inl ⟨caller.id, i, by rw [hin.1.1, hr], Nat.le_refl _⟩
    · refine Or.inr (Or.inr ⟨a, t, by rw [hin.1.1, hr], fun hS => ?_⟩)
      obtain ⟨hacq, hcnt⟩ := gsidOf_cons (b := caller.countedActive) (sk := rest.map Frame.skel) hd
      have := h.glob.acq t hS hacq
      have e : (caller :: rest).map Frame.skel =
          (caller.function, caller.countedActive) :: rest.map Frame.skel := rfl
      rw [e] at this
      omega
    · exact Or.inr (Or.inl ⟨π', hπ', hm', StrictlyInside.trans_inside hin hin'⟩)
  · exact Or.inr (Or.inr ⟨a, t, hr, fun hS => hn (h.sub t hS)⟩)

/-- What the frames above may invalidate covers none of a suspended caller's
references. -/
theorem SuspOk.not_covers {P : Program} {s : State} {caller : Frame} {rest : List Frame}
    {ps : List Reference} {S : StructId → Prop} {cid : FrameId} {cfn : FunctionRef}
    {cps : List Reference} {Sc : StructId → Prop}
    (h : SuspOk P s caller rest ps S cid cfn cps Sc) {e : Invalidation}
    (he : AllowedBy cid cps Sc e) {r : Reference} (hok : RefOk P s caller r)
    (hsh : ∀ π ∈ cps, π.mutable = true → ¬ r.location.StrictlyInside π.location)
    (hglob : ∀ a t, r.location.root = .global a t → Sc t) :
    e.covers r.location = false := by
  cases hc : e.covers r.location with
  | false => rfl
  | true =>
    exfalso
    have hbelow := hok.below
    rcases he _ hc with ⟨f, i, hr, hf⟩ | ⟨π, hπ, hm, hin⟩ | ⟨a, t, hr, hn⟩
    · rw [hr] at hbelow
      exact Nat.lt_irrefl _ (Nat.lt_of_lt_of_le h.order (Nat.le_trans hf hbelow))
    · exact hsh π hπ hm hin
    · exact hn (hglob a t hr)

/-- The frames below the current one, found by identity, are untouched when
the current frame is replaced by one of the same identity. -/
theorem frame?_cons_ne {s s' : State} {fr fr' : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) (hs'f : s'.frames = fr' :: rest) (hid : fr'.id = fr.id)
    {f : FrameId} (hf : f ≠ fr.id) : s'.frame? f = s.frame? f := by
  have h1 : (fr.id == f) = false := by simpa using Ne.symm hf
  have h2 : (fr'.id == f) = false := by rw [hid]; exact h1
  simp only [State.frame?, hsf, hs'f, List.find?_cons, h1, h2]

/-- An invalidation that leaves a location valid leaves its root's value. -/
theorem Invalidation.covers_root {e : Invalidation} {l : Location} (h : e.covers l = false) :
    e.covers ⟨l.root, []⟩ = false := by
  cases e with
  | removed ρ => simpa [Invalidation.covers] using h
  | mutated m =>
    simp only [Invalidation.covers, Bool.and_eq_false_iff]
    by_cases hp : m.path = []
    · right; simp [hp]
    · left; right
      cases hm : m.path with
      | nil => exact absurd hm hp
      | cons a b => simp [List.isPrefixOf]

/-- **What a step does to global storage**: a resource that a step changes is
one whose locations it invalidates. -/
def StorageKept (s s' : State) (es : List Invalidation) : Prop :=
  ∀ a t, (s.storage.get? a t).isSome = true → (∀ e ∈ es, e.covers ⟨.global a t, []⟩ = false) →
    s'.storage.get? a t = s.storage.get? a t

theorem storageKept_of_eq {s s' : State} {es : List Invalidation} (h : s'.storage = s.storage) :
    StorageKept s s' es := fun _ _ _ _ => by rw [h]

theorem GlobalStorage.get?_insert_of_some {st : GlobalStorage} {a a' : Address} {t t' : StructId}
    {v : Value} (h : (st.get? a' t').isSome = true) : (st.insert a t v).get? a' t' = st.get? a' t' := by
  unfold GlobalStorage.get? GlobalStorage.insert at *
  rw [List.find?_append]
  cases hf : st.find? (fun r => r.address == a' && r.type == t') with
  | none => rw [hf] at h; simp at h
  | some r => simp

/-- Publishing a resource keeps every resource there was. -/
theorem storageKept_insert {s s' : State} {es : List Invalidation} {a : Address} {t : StructId}
    {v : Value} (hs' : s'.storage = s.storage.insert a t v) : StorageKept s s' es :=
  fun _ _ hsome _ => by rw [hs']; exact GlobalStorage.get?_insert_of_some hsome

/-- The type and the value at a root that a step keeps. -/
theorem root_kept {P : Program} {s s' : State} {es : List Invalidation}
    (hst : StorageKept s s' es) {r : Reference} {frx : Frame}
    (hok : RefOk P s frx r) (hcov : ∀ e ∈ es, e.covers r.location = false) {a : Address}
    {t : StructId} (hρ : r.location.root = .global a t) :
    RootType P s' r.location.root = RootType P s r.location.root ∧
      s'.root? r.location.root = s.root? r.location.root := by
  have hsome : (s.storage.get? a t).isSome = true := by
    obtain ⟨T, _, hT, -⟩ := hok.root
    rw [hρ] at hT
    simp only [RootType, Option.bind_eq_some_iff] at hT
    obtain ⟨v, hv, -⟩ := hT
    simp [hv]
  have he := hst a t hsome fun e he => by
    have := Invalidation.covers_root (hcov e he); rw [hρ] at this; exact this
  rw [hρ]
  exact ⟨by simp only [RootType, he], by simp only [State.root?, he]⟩

/-- **The suspended frames keep their references** across a step of the
current frame `fr` that keeps the frames below and the held values, changes
global storage only where it invalidates, and invalidates only what the
frames from `cid` up may. -/
theorem ChainRef.transfer {P : Program} {s s' : State} {fr fr' : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) (hs'f : s'.frames = fr' :: rest) (hid : fr'.id = fr.id)
    (hheld : s'.held = s.held) {es : List Invalidation}
    (hes : s'.invalidations = es ++ s.invalidations) (hst : StorageKept s s' es) :
    ∀ (frs : List Frame) (cid : FrameId) (cfn : FunctionRef) (cps : List Reference)
      (Sc : StructId → Prop),
      cid ≤ fr.id → (∀ e ∈ es, AllowedBy cid cps Sc e) →
      ChainRef P s cid cfn cps Sc frs → ChainRef P s' cid cfn cps Sc frs
  | [], _, _, _, _, _, _, _ => trivial
  | caller :: frs, cid, cfn, cps, Sc, hcid, hal, ⟨ps, S, hsusp, hrest⟩ => by
    refine ⟨ps, S, ?_, ChainRef.transfer hsf hs'f hid hheld hes hst frs caller.id caller.function
      ps S (Nat.le_of_lt (Nat.lt_of_lt_of_le hsusp.order hcid))
      (fun e he => hsusp.allowed (hal e he)) hrest⟩
    obtain ⟨m, rs, rets, args, A, A', A'', L, hf⟩ := hsusp.call
    refine ⟨hsusp.order, hsusp.locals, hsusp.glob, hsusp.sub, m, rs, rets,
      args, A, A', A'', L, { hf with refs := fun x r hx hxr => ?_ }⟩
    obtain ⟨hok, hsh⟩ := hf.refs x r hx hxr
    refine ⟨?_, hsh⟩
    have hcov : ∀ e ∈ es, e.covers r.location = false := fun e he =>
      hsusp.not_covers (hal e he) hok hsh (hf.globals x r (heldIds_drop hx) hxr)
    -- The root is in a frame no newer than the caller, so not the current one.
    have hroot_ne : ∀ f i, r.location.root = .local_ f i → f ≠ fr.id := by
      intro f i hr e
      have hb := hok.below
      rw [hr] at hb
      exact Nat.lt_irrefl _ (Nat.lt_of_le_of_lt (e ▸ hb) (Nat.lt_of_lt_of_le hsusp.order hcid))
    have hframes : ∀ f g, s.frame? f = some g → f ≠ fr.id → s'.frame? f = some g :=
      fun f g hg hf => by rw [frame?_cons_ne hsf hs'f hid hf]; exact hg
    refine hok.transfer' rfl ?_ hes hcov ?_
    · obtain ⟨T, _, hT, -⟩ := hok.root
      cases hρ : r.location.root with
      | local_ f i =>
        rw [hρ] at hT
        simp only [RootType, Option.bind_eq_some_iff] at hT
        obtain ⟨g, hg, -⟩ := hT
        simp only [RootType, hframes f g hg (hroot_ne f i hρ), hg]
      | held k => simp only [RootType, hheld]
      | global a t => rw [← hρ]; exact (root_kept hst hok hcov hρ).1
    · intro hr
      rw [read?_eq_of_root]
      · exact hr
      cases hρ : r.location.root with
      | local_ f i =>
        simp only [State.root?]
        rw [frame?_cons_ne hsf hs'f hid (hroot_ne f i hρ)]
      | held k => simp only [State.root?, hheld]
      | global a t => rw [← hρ]; exact (root_kept hst hok hcov hρ).2

theorem GlobOk.skel {P : Program} {f f' : FunctionRef} {sk sk' : List (FunctionRef × Bool)}
    {ps : List Reference} {S : StructId → Prop} (h : GlobOk P f sk ps S) (hf : f' = f)
    (hsk : sk' = sk) : GlobOk P f' sk' ps S := by
  subst hf hsk; exact h

/-- **A step of the current frame that keeps the frames below** keeps the
reference invariant, given what it does to the current frame. -/
theorem RefInvariant.top {P : Program} {s s' : State} (h : RefInvariant P s) {fr fr' : Frame}
    {rest : List Frame} {ps : List Reference} (hsf : s.frames = fr :: rest)
    (hs'f : s'.frames = fr' :: rest) (hid : fr'.id = fr.id) (hfun : fr'.function = fr.function)
    (hca : fr'.countedActive = fr.countedActive) (hact : s'.active = s.active)
    (hnext : s.nextFrame ≤ s'.nextFrame)
    (hstor : ∀ r ∈ s'.storage, r.value.conforms P (.struct r.type) = true)
    (hheld : s'.held = s.held) {es : List Invalidation}
    (hes : s'.invalidations = es ++ s.invalidations) (hst : StorageKept s s' es)
    (hal : AllowedStep P fr rest ps es) (hchain : Below P s fr ps rest)
    (htop : ∃ A, modelRef P fr'.function fr'.pc = some A ∧ FrameOk P s' fr' ps A)
    (hfrom : RefsFrom fr fr') :
    RefInvariant P s' := by
  obtain ⟨hids, hbelow, -, -, hactive, hheldT⟩ := h
  obtain ⟨S, hglob, hch⟩ := hchain
  have hsk : (fr' :: rest).map Frame.skel = (fr :: rest).map Frame.skel := by
    rw [List.map_cons, List.map_cons, Frame.skel, Frame.skel, hfun, hca]
  have hids' : (s'.frames.map (·.id)).Nodup := by
    rw [hs'f, List.map_cons, hid]
    rw [hsf, List.map_cons] at hids
    exact hids
  refine ⟨hids', fun g hg => ?_, hstor, ⟨fr', rest, ps, hs'f, htop, S, hglob.skel hfun hsk, ?_⟩,
    hactive.skel (by rw [hs'f, hsf]; exact hsk) hact, hheldT.top hsf hs'f hfrom⟩
  · rw [hs'f] at hg
    rcases List.mem_cons.1 hg with rfl | hg
    · rw [hid]; exact Nat.lt_of_lt_of_le (hbelow fr (by rw [hsf]; exact List.mem_cons_self)) hnext
    · exact Nat.lt_of_lt_of_le (hbelow g (by rw [hsf]; exact List.mem_cons_of_mem _ hg)) hnext
  · rw [hfun, hid]
    exact ChainRef.transfer hsf hs'f hid hheld hes hst rest fr.id fr.function ps S (Nat.le_refl _)
      (hal S hglob) hch

/-- The current frame's reference parameters and the frames below, from the
invariant. -/
theorem RefInvariant.topOk {P : Program} {s : State} (h : RefInvariant P s) {fr : Frame}
    {rest : List Frame} (hsf : s.frames = fr :: rest) {A : RefState}
    (hA : modelRef P fr.function fr.pc = some A) :
    ∃ ps, FrameOk P s fr ps A ∧ Below P s fr ps rest := by
  obtain ⟨-, -, -, ⟨fr0, rest0, ps, hsf0, ⟨A0, hA0, hok⟩, hchain⟩, -⟩ := h
  rw [hsf, List.cons.injEq] at hsf0
  obtain ⟨rfl, rfl⟩ := hsf0
  rw [hA, Option.some.injEq] at hA0
  subst hA0
  exact ⟨ps, hok, hchain⟩

/-! ## Frames that keep their functions and locals -/

/-- **A tracked reference stays tracked** in a state whose frames no newer
than its root keep their functions and locals, with the same held values and
global storage and no new invalidation. -/
theorem RefOk.agree {P : Program} {s s' : State} {fr fr' : Frame} {r : Reference}
    (h : RefOk P s fr r) {b : FrameId} (hb : RootBelow b r.location.root)
    (hfr : ∀ f, f ≤ b → ∀ g, s.frame? f = some g →
      ∃ g', s'.frame? f = some g' ∧ g'.function = g.function ∧ g'.locals = g.locals)
    (hheld : s'.held = s.held) (hstor : s'.storage = s.storage)
    (hinv : s'.invalidations = s.invalidations) (hid : b ≤ fr'.id) :
    RefOk P s' fr' r := by
  obtain ⟨⟨T, T', hT, hpath, hasg⟩, -, hval, hsince, hvalid, hwr⟩ := h
  have hsame : RootType P s' r.location.root = RootType P s r.location.root ∧
      s'.root? r.location.root = s.root? r.location.root := by
    cases hρ : r.location.root with
    | local_ f i =>
      rw [hρ] at hT hb
      simp only [RootBelow] at hb
      simp only [RootType, Option.bind_eq_some_iff] at hT
      obtain ⟨g, hg, -⟩ := hT
      obtain ⟨g', hg', hfun', hloc'⟩ := hfr f hb g hg
      constructor
      · simp only [RootType, hg, hg', Option.bind_some, hfun']
      · simp only [State.root?, hg, hg', hloc', bind, Option.bind_some]
    | held k => exact ⟨by simp only [RootType, hheld], by simp only [State.root?, hheld]⟩
    | global a t => exact ⟨by simp only [RootType, hstor], by simp only [State.root?, hstor]⟩
  refine ⟨⟨T, T', by rw [hsame.1]; exact hT, hpath, hasg⟩, ?_, hval, by rw [hinv]; exact hsince, ?_,
    hwr⟩
  · cases hρ : r.location.root with
    | local_ f i =>
      rw [hρ] at hb
      exact Nat.le_trans hb hid
    | held k => trivial
    | global a t => trivial
  · exact valid_transfer hsince (es := []) (by rw [hinv]; rfl) (fun e he => by simp at he)
      (fun hr => by rw [read?_eq_of_root hsame.2]; exact hr) hvalid

/-- **The suspended frames keep their references** across a step that keeps
the functions and locals of every frame older than `bound`, the held values
and global storage, and invalidates nothing. -/
theorem ChainRef.transferBelow {P : Program} {s s' : State} {bound : FrameId}
    (hfr : ∀ f, f < bound → ∀ g, s.frame? f = some g →
      ∃ g', s'.frame? f = some g' ∧ g'.function = g.function ∧ g'.locals = g.locals)
    (hheld : s'.held = s.held) (hstor : s'.storage = s.storage)
    (hinv : s'.invalidations = s.invalidations) :
    ∀ (frs : List Frame) (cid : FrameId) (cfn : FunctionRef) (cps : List Reference)
      (Sc : StructId → Prop),
      cid ≤ bound → ChainRef P s cid cfn cps Sc frs → ChainRef P s' cid cfn cps Sc frs
  | [], _, _, _, _, _, _ => trivial
  | caller :: frs, cid, cfn, cps, Sc, hcid, ⟨ps, S, hsusp, hrest⟩ => by
    have hlt : caller.id < bound := Nat.lt_of_lt_of_le hsusp.order hcid
    refine ⟨ps, S, ?_, ChainRef.transferBelow hfr hheld hstor hinv frs caller.id caller.function ps
      S (Nat.le_of_lt hlt) hrest⟩
    obtain ⟨m, rs, rets, args, A, A', A'', L, hf⟩ := hsusp.call
    refine ⟨hsusp.order, hsusp.locals, hsusp.glob, hsusp.sub, m, rs, rets,
      args, A, A', A'', L, { hf with refs := fun x r hx hxr => ?_ }⟩
    obtain ⟨hok, hsh⟩ := hf.refs x r hx hxr
    exact ⟨hok.agree hok.below (fun f hf g hg => hfr f (Nat.lt_of_le_of_lt hf hlt) g hg) hheld
      hstor hinv (Nat.le_refl _), hsh⟩

end MoveBytecode.Proofs
