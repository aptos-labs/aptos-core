-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostTyping

/-!
# Ownership accounting and encapsulation along traces

UNTRUSTED (checked by the kernel). The trace invariant (`TraceInv`): the shadow
fits the state; the state invariant (`StateInv`) and the typing invariant
(`StateTyped`) hold; every key present is allowed (`Key.Ok`); every key present was
issued, identities are issued with one key and below the fresh counter
(`PreFacts`); identities without `copy` are unique; every event so far
respects D3; and every identity created without `drop` is still present or
was consumed legitimately. It holds initially (given the facts of the
initial shadow) and after every annotated step whose annotation establishes
`StepGhost`. D2 and D3 follow.

The steps' annotations are the core's (`Proofs.GhostExec`) and the natives'
(`NativeAnnotated`, `Proofs.GhostNative`); the typing invariant is kept by
the semantics alone (`Proofs.GhostTyping`).
-/

namespace MoveBytecode.Proofs

/-- The trace invariant. -/
structure TraceInv (P : Program) (π : NativePolicy) (A : List StructId) (auth : List Address)
    (s : State) (h : Shadow) (evs : List Event) : Prop where
  fits : h.fits s = true
  inv : StateInv true P s
  keysOk : ∀ x ∈ keys (h.occurrences s), x.2.Ok true
  typed : StateTyped P s
  facts : PreFacts (h.occurrences s) evs h.fresh
  unique : Ownership.unique P (h.occurrences s) = true
  respected : ∀ e ∈ evs, Encapsulation.structsRespected π A e = true ∧
    Encapsulation.signersRespected π auth e = true
  conserve : ∀ o ∈ createdOf evs, (o.2.type.abilities P).drop = true ∨
    (∃ o' ∈ h.occurrences s, o'.1 = o.1) ∨
    ∃ w c, Event.consumed o.1 w c ∈ evs ∧ c.Legitimate π w = true
  /-- Every event so far respects D3c. -/
  closures : ∀ e ∈ evs, Encapsulation.closuresRespected P e = true
  /-- Every closure present was created as it is, with its identity (D3c). -/
  closuresMade : ∀ x ∈ h.occurrences s, IsClosure x.2 = true → ∃ c, Event.created x.1 x.2 c ∈ evs

/-- What a step's annotation assumes holds in every invariant state. -/
theorem TraceInv.pre {P : Program} {π : NativePolicy} {A : List StructId} {auth : List Address}
    {s : State} {h : Shadow} {evs : List Event} (hI : TraceInv P π A auth s h evs) : Pre true P s h :=
  ⟨hI.fits, hI.inv, fun x hx => hI.keysOk x (by
    rw [shadow_occurrences, keys_append]; exact List.mem_append_left _ hx),
    typed_refs hI.typed hI.inv, refsTop_of_typed hI.typed hI.inv, composes_of_typed hI.typed⟩

/-- Every native step that continues from a state whose shadow fits it, with
the state invariants and the typing invariant, is annotated, and its
annotation establishes `StepGhost`. -/
def NativeAnnotated (σ : Bool) (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy)
    (A : List StructId) (auth : List Address) : Prop :=
  ∀ (s : State) (h : Shadow) (s' : State) (f : FunctionRef) (args views vals writes : List Value)
    (places : List NativePlace),
    Pre σ P s h → StateTyped P s → step P s = .callNative s' f args views →
    N f views (.returned vals writes places) →
    (resumeNative P s' f args (.returned vals writes places)).continues = true →
    ∃ post evs, Shadow.nativeStep P ν s f args views vals writes places
        (resumeNative P s' f args (.returned vals writes places)) h = some (post, evs) ∧
      StepGhost σ P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames)
        h.fresh (resumeNative P s' f args (.returned vals writes places)) post evs

theorem fits_continues {post : ShadowPost} {o : Outcome} (h : post.fits o = true) :
    o.continues = true := by
  cases post <;> cases o <;> simp_all [ShadowPost.fits, Outcome.continues]

/-- An annotated native step continues: its annotation fits only `running`
outcomes. -/
theorem nativeStep_continues {P : Program} {ν : NativeFlows} {s : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {o : Outcome} {h : Shadow}
    {post : ShadowPost} {evs : List Event}
    (hn : Shadow.nativeStep P ν s f args views vals writes places o h = some (post, evs)) :
    o.continues = true := by
  cases o with
  | running _ => rfl
  | _ =>
    simp [Shadow.nativeStep] at hn
    split at hn <;> simp at hn

section Steps

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {π : NativePolicy} {A : List StructId}
  {auth : List Address}

/-- What an annotated step from an invariant state establishes, and the
typing invariant after it. -/
theorem traced_ghost (hnat : NativeAnnotated true P N ν π A auth) {s : State} {h : Shadow}
    {evs : List Event} (hI : TraceInv P π A auth s h evs) {o : Outcome} {post : ShadowPost}
    {evs' : List Event} (ht : TracedStep P N ν s h o post evs') :
    StepGhost true P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh o post evs' ∧ PostTyped P o := by
  cases ht with
  | core hstep hc =>
    obtain ⟨post', evs'', hc', hg⟩ := coreStep_annot (π := π) (A := A) (auth := auth) hI.pre
      hstep (coreStep_continues hc) (fun _ _ _ => hI.inv.heldDrop rfl)
    rw [hc] at hc'
    simp only [Option.some.injEq, Prod.mk.injEq] at hc'
    obtain ⟨rfl, rfl⟩ := hc'
    exact ⟨hg, typed_core hI.typed hI.inv hstep (coreStep_continues hc)⟩
  | native hstep hN hn =>
    obtain ⟨post', evs'', hn', hg⟩ := hnat _ _ _ _ _ _ _ _ _ hI.pre hI.typed hstep hN
      (nativeStep_continues hn)
    rw [hn] at hn'
    simp only [Option.some.injEq, Prod.mk.injEq] at hn'
    obtain ⟨rfl, rfl⟩ := hn'
    exact ⟨hg, typed_native hI.typed hI.inv hstep (nativeStep_continues hn)⟩

/-- Conservation carries over a step. -/
theorem conserve_step {s : State} {h : Shadow} {evs : List Event}
    (hI : TraceInv P π A auth s h evs)
    {o : Outcome} {post : ShadowPost} {evs' : List Event}
    (hG : StepGhost true P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh o post evs') :
    ∀ x ∈ createdOf (evs ++ evs'), (x.2.type.abilities P).drop = true ∨
      (∃ x' ∈ post.occurrences o, x'.1 = x.1) ∨
      ∃ w c, Event.consumed x.1 w c ∈ evs ++ evs' ∧ c.Legitimate π w = true := by
  have he := hG.effect
  intro x hx
  rw [createdOf_append, List.mem_append] at hx
  rcases hx with hx | hx
  · rcases hI.conserve x hx with hd | ⟨x', hx', hid⟩ | ⟨w, c, hc, hl⟩
    · exact Or.inl hd
    · by_cases hd : (x.2.type.abilities P).drop = true
      · exact Or.inl hd
      · -- `x'` has `x`'s key: both were issued with the same identity.
        have hkey : x'.2.key = x.2.key := by
          have h1 := hI.facts.provenance _ (mem_keys hx')
          have h2 := keys_created_sub _ (mem_keys hx)
          have := hI.facts.functional _ h1 _ h2 hid
          simp only [Prod.mk.injEq] at this
          exact this.2
        have htype : x'.2.type = x.2.type := by
          have := congrArg Key.type hkey
          simpa using this
        have hd' : (x'.2.type.abilities P).drop = false := by rw [htype]; simpa using hd
        have hcount := he.count hI.facts hx' hd'
        have hpos : 0 < Ownership.count (h.occurrences s) x'.1 :=
          List.countP_pos_iff.2 ⟨x', hx', by simp⟩
        by_cases hpost : 0 < Ownership.count (post.occurrences o) x'.1
        · obtain ⟨y, hy, hyi⟩ := List.countP_pos_iff.1 hpost
          refine Or.inr (Or.inl ⟨y, hy, ?_⟩)
          rw [← hid]; simpa using hyi
        · have hcons : 0 < Ownership.consumptions evs' x'.1 := by omega
          rw [consumptions_eq] at hcons
          obtain ⟨y, hy, hyi⟩ := List.countP_pos_iff.1 hcons
          simp only [beq_iff_eq] at hyi
          obtain ⟨z, hz, hz1, hz2⟩ := mem_keys_iff.1 hy
          obtain ⟨c, hc⟩ := mem_consumedOf.1 hz
          have hyk : y = (x'.1, x'.2.key) := hI.facts.key (he.mem_pre hy) (mem_keys hx') hyi
          have hzk : z.2.key = x'.2.key := by rw [hz2, hyk]
          have hzt : z.2.type = x.2.type := by
            have := congrArg Key.type hzk
            simp only [key_type] at this
            rw [this, htype]
          have hzi : z.1 = x.1 := by rw [hz1, hyi, hid]
          refine Or.inr (Or.inr ⟨z.2, c, ?_, ?_⟩)
          · rw [← hzi]; exact List.mem_append_right _ hc
          · rcases hG.legit _ _ _ hc with hdz | hlz
            · rw [hzt] at hdz; exact absurd hdz hd
            · exact hlz
    · exact Or.inr (Or.inr ⟨w, c, List.mem_append_left _ hc, hl⟩)
  · obtain ⟨keep, dropped, copies, -, hpost, -⟩ := he
    have hk := keys_created_sub _ (mem_keys hx)
    have hk' : (x.1, x.2.key) ∈ keys (post.occurrences o) :=
      hpost.symm.subset (List.mem_append_right _ hk)
    obtain ⟨y, hy, hy1, -⟩ := mem_keys_iff.1 hk'
    exact Or.inr (Or.inl ⟨y, hy, hy1⟩)

/-- The closures after a step were made as they are. -/
theorem closuresMade_step {s : State} {h : Shadow} {evs : List Event}
    (hI : TraceInv P π A auth s h evs) {o : Outcome} {post : ShadowPost} {evs' : List Event}
    (hG : StepGhost true P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh o post evs') :
    ∀ x ∈ post.occurrences o, IsClosure x.2 = true → ∃ c, Event.created x.1 x.2 c ∈ evs ++ evs' := by
  intro x hx hc
  rcases hG.closuresKept x hx hc with hx | ⟨c, hc'⟩
  · obtain ⟨c, hc'⟩ := hI.closuresMade x hx hc
    exact ⟨c, List.mem_append_left _ hc'⟩
  · exact ⟨c, List.mem_append_right _ hc'⟩

/-- The invariant carries over a running step. -/
theorem inv_step {s s' : State} {h h' : Shadow} {evs evs' : List Event}
    (hI : TraceInv P π A auth s h evs)
    (hG : StepGhost true P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh (.running s') (.running h') evs')
    (hT : PostTyped P (.running s')) :
    TraceInv P π A auth s' h' (evs ++ evs') where
  fits := hG.fits
  inv := hG.inv
  keysOk := by
    intro x hx
    rcases hG.effect.sub hx with hx | hx
    · exact hI.keysOk x hx
    · exact hG.issuedOk x hx
  typed := hT
  facts := hG.effect.preFacts hI.facts
  unique := hG.effect.unique hI.facts hI.unique
  respected := by
    intro e he
    rcases List.mem_append.1 he with he | he
    · exact hI.respected e he
    · exact hG.respected e he
  conserve := conserve_step hI hG
  closures := by
    intro e he
    rcases List.mem_append.1 he with he | he
    · exact hI.closures e he
    · exact hG.closures e he
  closuresMade := closuresMade_step hI hG

/-- The invariant holds along every trace. -/
theorem trace_inv (hnat : NativeAnnotated true P N ν π A auth) {s₀ : State}
    (hinit : TraceInv P π A auth s₀ (Shadow.initial s₀).1 (Shadow.initial s₀).2) {s : State}
    {h : Shadow} {evs : List Event} (ht : Trace P N ν s₀ s h evs) :
    TraceInv P π A auth s h evs := by
  induction ht with
  | start => exact hinit
  | step _ hs ih =>
    obtain ⟨hG, hT⟩ := traced_ghost hnat ih hs
    exact inv_step ih hG hT

end Steps

/-! ## D2 and D3 from the invariant -/

section Theorems

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {π : NativePolicy} {A : List StructId}
  {auth : List Address} {s₀ : State}
  (hnat : NativeAnnotated true P N ν π A auth)
  (hinit : TraceInv P π A auth s₀ (Shadow.initial s₀).1 (Shadow.initial s₀).2)

include hnat hinit

theorem accounts_of : ShadowAccounts P N ν s₀ := by
  intro s h evs ht o hstep hcont
  have hI := trace_inv hnat hinit ht
  cases hstep with
  | core hs =>
    obtain ⟨post, evs', hc, -⟩ :=
      coreStep_annot (π := π) (A := A) (auth := auth) hI.pre hs hcont
        (fun _ _ _ => hI.inv.heldDrop rfl)
    exact ⟨post, evs', .core hs hc⟩
  | native hs hN =>
    rename_i s' f args views r
    cases r with
    | failed e => simp [resumeNative, Outcome.continues] at hcont
    | returned vals writes places =>
      obtain ⟨post, evs', hn, -⟩ := hnat _ _ _ _ _ _ _ _ _ hI.pre hI.typed hs hN hcont
      exact ⟨post, evs', .native hs hN hn⟩

theorem identitiesUnique_of : IdentitiesUnique P N ν s₀ := by
  intro s h evs ht
  have hI := trace_inv hnat hinit ht
  exact ⟨hI.unique, fun o post evs' hs =>
    (traced_ghost hnat hI hs).1.effect.unique hI.facts hI.unique⟩

theorem noImplicitDestruction_of : NoImplicitDestruction P N ν π s₀ := by
  intro s h evs ht o post evs' hs
  have hI := trace_inv hnat hinit ht
  have hG := (traced_ghost hnat hI hs).1
  simp only [Ownership.destroysNothing, Bool.and_eq_true, List.all_eq_true]
  refine ⟨fun x hx => ?_, fun e he => ?_⟩
  · obtain ⟨i, v⟩ := x
    by_cases hd : (v.type.abilities P).drop = true
    · simp [hd]
    · have := hG.effect.count hI.facts hx (by simpa using hd)
      simp [hd, this]
  · cases e with
    | consumed i w c =>
      have := hG.legit i w c he
      simpa using this
    | created | borrowed | used | accessed | vacated | entered | frameEnded | privileged => rfl

theorem conservation_of : Conservation P N ν π s₀ := by
  intro s h evs ht vals storage post evs' hs
  have hI := trace_inv hnat hinit ht
  have hG := (traced_ghost hnat hI hs).1
  have hc := conserve_step hI hG
  simp only [Ownership.conserved, List.all_eq_true]
  intro e he
  cases e with
  | created i v c =>
    have hx : (i, v) ∈ createdOf (evs ++ evs') := mem_createdOf.2 ⟨c, he⟩
    dsimp only
    simp only [Bool.or_eq_true]
    rcases hc _ hx with hd | ⟨x', hx', hid⟩ | ⟨w, c', hc', hl⟩
    · exact Or.inl (Or.inl hd)
    · exact Or.inl (Or.inr (List.any_eq_true.2 ⟨x', hx', by simpa using hid⟩))
    · exact Or.inr (List.any_eq_true.2 ⟨_, hc', by simpa using hl⟩)
  | consumed | borrowed | used | accessed | vacated | entered | frameEnded | privileged => rfl

omit hnat hinit in
theorem structsAuthentic_of {occ : Occ} {evs : List Event}
    (hprov : ∀ x ∈ keys occ, x ∈ issuedOf evs)
    (hresp : ∀ e ∈ evs, Encapsulation.structsRespected π A e = true) :
    Encapsulation.structsAuthentic π A occ evs = true := by
  simp only [Encapsulation.structsAuthentic, List.all_eq_true]
  rintro ⟨i, v⟩ hx
  have hk : (i, v.key) ∈ issuedOf evs := hprov _ (mem_keys hx)
  split
  · next st hs =>
    rcases mem_issuedOf hk with ⟨w, c, hc, hwk⟩ | ⟨r, p, -, hrk⟩
    · have hwt : w.type = v.type := by
        have := congrArg Key.type hwk
        simpa using this
      have hr := hresp _ hc
      simp only [Encapsulation.structsRespected, hwt, hs] at hr
      exact List.any_eq_true.2 ⟨_, hc, by simp [hwt, hs, hr]⟩
    · have := congrArg Key.type hrk
      simp only [key_type] at this
      rw [hs] at this
      simp [Value.type] at this
  · rfl

omit hnat hinit in
theorem signersAuthentic_of {occ : Occ} {evs : List Event}
    (hprov : ∀ x ∈ keys occ, x ∈ issuedOf evs)
    (hresp : ∀ e ∈ evs, Encapsulation.signersRespected π auth e = true) :
    Encapsulation.signersAuthentic π auth occ evs = true := by
  simp only [Encapsulation.signersAuthentic, List.all_eq_true]
  rintro ⟨i, v⟩ hx
  have hk : (i, v.key) ∈ issuedOf evs := hprov _ (mem_keys hx)
  cases v with
  | signer a =>
    rcases mem_issuedOf hk with ⟨w, c, hc, hwk⟩ | ⟨r, p, -, hrk⟩
    · have hw : w = .signer a := by
        cases w <;> simp_all [Value.key]
      subst hw
      have hr := hresp _ hc
      simp only [Encapsulation.signersRespected] at hr
      exact List.any_eq_true.2 ⟨_, hc, by simp [hr]⟩
    · simp [Value.key] at hrk
  | _ => rfl

omit hnat hinit in
theorem closuresAuthentic_of {occ : Occ} {evs : List Event}
    (hmade : ∀ x ∈ occ, IsClosure x.2 = true → ∃ c, Event.created x.1 x.2 c ∈ evs)
    (hresp : ∀ e ∈ evs, Encapsulation.closuresRespected P e = true) :
    Encapsulation.closuresAuthentic P occ evs = true := by
  simp only [Encapsulation.closuresAuthentic, List.all_eq_true]
  rintro ⟨i, v⟩ hx
  cases v with
  | closure f mask cs ps rs a =>
    obtain ⟨c, hc⟩ := hmade _ hx rfl
    have hr := hresp _ hc
    simp only [Encapsulation.closuresRespected] at hr
    exact List.any_eq_true.2 ⟨_, hc, by simp [hr]⟩
  | _ => rfl

theorem structsEncapsulated_of : StructsEncapsulated P N ν π A s₀ := by
  intro s h evs ht
  have hI := trace_inv hnat hinit ht
  refine ⟨fun e he => (hI.respected e he).1,
    structsAuthentic_of hI.facts.provenance (fun e he => (hI.respected e he).1),
    fun o post evs' hs => ?_⟩
  have hG := (traced_ghost hnat hI hs).1
  refine ⟨fun e he => (hG.respected e he).1,
    structsAuthentic_of (hG.effect.provenance hI.facts) ?_⟩
  intro e he
  rcases List.mem_append.1 he with he | he
  · exact (hI.respected e he).1
  · exact (hG.respected e he).1

theorem signersUnforgeable_of : SignersUnforgeable P N ν π auth s₀ := by
  intro s h evs ht
  have hI := trace_inv hnat hinit ht
  refine ⟨fun e he => (hI.respected e he).2,
    signersAuthentic_of hI.facts.provenance (fun e he => (hI.respected e he).2),
    fun o post evs' hs => ?_⟩
  have hG := (traced_ghost hnat hI hs).1
  refine ⟨fun e he => (hG.respected e he).2,
    signersAuthentic_of (hG.effect.provenance hI.facts) ?_⟩
  intro e he
  rcases List.mem_append.1 he with he | he
  · exact (hI.respected e he).2
  · exact (hG.respected e he).2

theorem closuresUnforgeable_of : ClosuresUnforgeable P N ν s₀ := by
  intro s h evs ht
  have hI := trace_inv hnat hinit ht
  refine ⟨hI.closures, closuresAuthentic_of hI.closuresMade hI.closures,
    fun o post evs' hs => ?_⟩
  have hG := (traced_ghost hnat hI hs).1
  refine ⟨hG.closures, closuresAuthentic_of (closuresMade_step hI hG) ?_⟩
  intro e he
  rcases List.mem_append.1 he with he | he
  · exact hI.closures e he
  · exact hG.closures e he

theorem ownershipAccounted_of : OwnershipAccounted P N ν π s₀ :=
  ⟨accounts_of hnat hinit, identitiesUnique_of hnat hinit, noImplicitDestruction_of hnat hinit,
    conservation_of hnat hinit⟩

theorem encapsulated_of : Encapsulated P N ν π auth A s₀ :=
  ⟨accounts_of hnat hinit, structsEncapsulated_of hnat hinit, signersUnforgeable_of hnat hinit,
    closuresUnforgeable_of hnat hinit⟩

end Theorems

end MoveBytecode.Proofs
