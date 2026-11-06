-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefTrace
import Proofs.DanglingEvents
import Proofs.DanglingCreates
import Proofs.DanglingKeeps
import Proofs.RefRuntime

/-!
# D4b′, D4c and D4e imply D4a

UNTRUSTED (checked by the kernel). `NoDanglingFromContainment`
(`Spec.Properties.Theorems`): along every annotated execution from an
admissible initial state, whatever the natives do, a reference is valid when
it is used, provided that no access reaches over a live `&mut` (D4b′), no write
overlaps a live `&` (D4c), and no local or frame a live reference points into
ends (D4e).

The argument follows a reference from its creation to a use. It is valid when
created (`coreStep_creates`, and the transaction's references at the initial
state, `valid_initial`); each later step keeps it valid unless one of the
step's events hits its location (`coreStep_keeps`, `native_keeps`); and a hit
between the creation and a later use is what D4b′, D4c and D4e exclude
(`hits_absurd`). The reference a step uses is in the frames before the step,
so it was created by a `borrowed` event of the trace (`RefTraceInv`), with
the same value (`borrowed_unique`).
-/

namespace MoveBytecode.Proofs

/-! ## The initial state -/

theorem initialState_invalidations {P : Program} {f : FunctionRef} {args : List Value}
    {storage : GlobalStorage} {held : List Value} {s₀ : State}
    (h : initialState P f args storage held = some s₀) : s₀.invalidations = [] := by
  unfold initialState at h
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def] at h
  obtain ⟨info, -, _, -, localTys, -, h⟩ := h
  split at h
  · simp at h
  · split at h
    · simp at h
    · simp only [Option.some.injEq] at h
      subst h
      rfl

/-- **The references the transaction lends are valid** at an admissible
initial state: they designate held values, and nothing is invalidated yet. -/
theorem valid_initial {P : Program} {s₀ : State} (h0 : AdmissibleInitial P s₀) {i : Nat}
    {r : Reference} {par : Option Nat} (hb : Event.borrowed i r par ∈ (Shadow.initial s₀).2) :
    Valid s₀ r.location r.since := by
  obtain ⟨f, args, tys, storage, held, -, -, -, hinit⟩ := h0
  obtain ⟨fr, localTys, hfr, hst, -, -, -, hloc, -, hhe, -, hha⟩ := initialState_shape hinit
  have hinv := initialState_invalidations hinit
  obtain ⟨ls, evs₁, n₁, hf, -, -, -, -, -, -, hev⟩ := locals_fold fr.locals.toList [] 0 []
  obtain ⟨sts, e₂, n₂, hfS, -, -, -, -, -, hevS⟩ :=
    fresh_fold .storage (s₀.storage.map (·.value)) [] n₁ []
  obtain ⟨hts, e₃, n₃, hfH, -, -, -, -, -, hevH⟩ := fresh_fold .transaction s₀.held [] n₂ []
  rw [initial_eq hfr] at hb
  simp only [initL, initK, initS, initH, hst, freshAll_nil, hf, hfS, hfH, List.nil_append,
    List.append_nil, List.mem_append] at hb
  rcases hb with ((hb | hb) | hb) | hb
  · obtain ⟨_, _, he, -⟩ := hev _ hb; cases he
  · obtain ⟨_, _, he, -⟩ := hevS _ hb; cases he
  · obtain ⟨_, _, he, -⟩ := hevH _ hb; cases he
  simp only [initRefs, List.mem_append, List.mem_map, List.mem_singleton] at hb
  rcases hb with ⟨⟨i', r'⟩, hp, he⟩ | he
  · simp only [Event.borrowed.injEq] at he
    obtain ⟨-, rfl, -⟩ := he
    simp only [initParams, List.mem_filterMap] at hp
    obtain ⟨⟨ot, ov⟩, hz, hm⟩ := hp
    have hv : ov ∈ fr.locals.toList := (List.of_mem_zip hz).2
    have hov : ov = some (.ref r') := by
      rcases ot with _ | t <;> rcases ov with _ | v <;> try (simp at hm)
      cases v <;> simp_all
    rw [hov, hloc, List.toList_toArray, List.mem_append, List.mem_map] at hv
    rcases hv with ⟨a, ha, haa⟩ | hv
    · simp only [Option.some.injEq] at haa
      subst haa
      simp only [heldArguments, Bool.and_eq_true, List.all_eq_true] at hha
      obtain ⟨hsince, h3⟩ := hha.1.2 r' (List.mem_filterMap.2 ⟨_, ha, rfl⟩)
      have hsince : r'.since = 0 := by simpa using hsince
      split at h3
      · next k hroot hpath =>
        split at h3
        · next v hk =>
          refine ⟨?_, by rw [hsince]; exact Nat.zero_le _⟩
          rw [deref?_isSome]
          refine ⟨fun e he => by simp [State.invalidatedSince, hinv] at he, ?_⟩
          rw [Location.eta r'.location, hroot, hpath, read?_held, hhe, hk]
          simp [Value.get?]
        · cases h3
      · cases h3
    · simp [List.mem_replicate] at hv
  · cases he


/-! ## Along a trace -/

/-- **A traced step keeps every valid location valid**, unless one of its
events hits it. -/
theorem traced_keeps {P : Program} {N : NativeRel} {ν : NativeFlows} {s s' : State} {h : Shadow}
    {o : Outcome} {post : ShadowPost} {evs evs' : List Event} (hI : RefTraceInv P s h evs)
    (hT : TracedStep P N ν s h o post evs') (ho : o = .running s') {l : Location} {n : Nat}
    (hv : Valid s l n) : Valid s' l n ∨ ∃ e ∈ evs', Hits l e := by
  cases hT with
  | core hstep hc =>
    subst ho
    exact coreStep_keeps hI.pre hI.typed hstep hc hv
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      obtain ⟨g₁, hs₁, info₁, tys₁, ptys₁, seen₁, s₂, fr₂, tl₂, newRefs₁, resultTrees₁, h₂, -, -, -,
        -, -, -, -, -, -, -, -, -, -, -, -, hpost, -⟩ := nativeStep_spec hn
      subst hpost
      rw [ho] at hn
      exact native_keeps_modeled hshape hstep ho hn hv
    exact Or.inl (native_keeps hstep hshape ho hv)

/-- **The references a traced step creates are valid** after it, when the
references it uses are valid before it. -/
theorem traced_creates {P : Program} {N : NativeRel} {ν : NativeFlows} {s s' : State}
    {h : Shadow} {o : Outcome} {post : ShadowPost} {evs evs' : List Event}
    (hI : RefTraceInv P s h evs) (hT : TracedStep P N ν s h o post evs') (ho : o = .running s')
    (huses : ∀ t r, Event.used t r ∈ evs' → Valid s r.location r.since) {i : Nat}
    {r : Reference} {par : Option Nat} (hb : Event.borrowed i r par ∈ evs') :
    Valid s' r.location r.since := by
  cases hT with
  | core hstep hc =>
    subst ho
    exact coreStep_creates hI.pre hI.typed hstep hc huses hb
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      obtain ⟨tys, -, hfits⟩ := nativeStep_fits hstep hn
      obtain ⟨-, -, -, -, p, hp, -⟩ := nativeStep_borrowed hn hb
      have hkb := places_borrow hfits (List.ne_nil_of_mem hp)
      obtain ⟨fr, rest, -, -, r0, t0, m, tgt, p', d', hsf, -, -, -, -, -, -, -, -, hall,
        ⟨w, hw, hget⟩, n, hres⟩ := nativeStep_borrowEvents hstep hn hkb
      rcases hall _ hb with he | he | he | ⟨_, _, _, he⟩ | ⟨_, _, _, he⟩ <;> cases he
      rw [ho, Outcome.running.injEq] at hres
      subst hres
      refine ⟨?_, Nat.le_refl _⟩
      rw [deref?_isSome]
      refine ⟨fun e he => (by
        have := invalidatedSince_self s
        simp only [State.invalidatedSince] at this he
        rw [this] at he
        cases he), ?_⟩
      simp only
      rw [read?_descend, read_eq (s := s) (s' := { s with
        frames := { fr with
          stack := Value.ref ⟨m, tgt, r0.location.descend p'.path, s.invalidations.length⟩ ::
            fr.stack.drop n
          pc := fr.pc + 1 } :: rest }) (by simp [hsf]) rfl rfl, deref?_read hw]
      exact hget
    obtain ⟨rfl, rfl, rfl, g, hs, info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    have hm : i ∈ borrowedIds evs' := mem_borrowedIds.2 ⟨r, par, hb⟩
    rw [hevs, nativeEvents_borrowedIds] at hm
    cases hm

/-- **A hit between a reference's creation and a later use is excluded** by
D4b′, D4c and D4e. -/
theorem hit_between {A B F : List Event} (hc : RefSafety.contained (A ++ B ++ F) = true)
    (hs : RefSafety.stable (A ++ B ++ F) = true)
    (hd : RefSafety.deadSlotsUnborrowed (A ++ B ++ F) = true) {i : Nat} {r : Reference}
    {par : Option Nat} {e : Event} (hb : Event.borrowed i r par ∈ A) (he : e ∈ B)
    (hu : Event.used i r ∈ F) (hh : Hits r.location e) : False := by
  obtain ⟨p, hp⟩ := List.mem_iff_getElem?.1 hb
  obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 he
  obtain ⟨m, hm⟩ := List.mem_iff_getElem?.1 hu
  have hpl : p < A.length := (List.getElem?_eq_some_iff.1 hp).1
  have hkl : k < B.length := (List.getElem?_eq_some_iff.1 hk).1
  refine hits_absurd hc hs hd (i := i) (par := par) (p := p) (q := A.length + k)
    (u := A.length + B.length + m)
    ?_ ?_ ?_ (by omega) (by omega) hh
  · rw [List.append_assoc, List.getElem?_append_left hpl, hp]
  · rw [List.append_assoc, List.getElem?_append_right (by omega),
      List.getElem?_append_left (by omega), Nat.add_sub_cancel_left, hk]
  · rw [List.getElem?_append_right (by simp), List.length_append, Nat.add_sub_cancel_left, hm]

/-- **A reference is valid until its last use**: along a trace from an
admissible initial state whose events, followed by `F`, satisfy D4b′, D4c and
D4e, every reference created so far and used in `F` is valid. -/
theorem valid_along {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
    (h0 : AdmissibleInitial P s₀) {s : State} {h : Shadow} {evs : List Event}
    (hT : Trace P N ν s₀ s h evs) (F : List Event)
    (hc : RefSafety.contained (evs ++ F) = true) (hs : RefSafety.stable (evs ++ F) = true)
    (hd : RefSafety.deadSlotsUnborrowed (evs ++ F) = true) {i : Nat} {r : Reference}
    {par : Option Nat} (hb : Event.borrowed i r par ∈ evs) (hu : Event.used i r ∈ F) :
    Valid s r.location r.since := by
  induction hT generalizing F i r par with
  | start => exact valid_initial h0 hb
  | @step s₁ s₂ h₁ h₂ evs₁ evs₂ hT₀ hS ih =>
    have hI := refTraceInv_trace h0 hT₀
    rw [List.append_assoc] at hc hs hd
    have huses : ∀ t rt, Event.used t rt ∈ evs₂ → Valid s₁ rt.location rt.since := by
      intro t rt hu'
      obtain ⟨par', hb'⟩ := hI.refs t rt (traced_used hI.pre hS hu')
      exact ih _ hc hs hd hb' (List.mem_append_left _ hu')
    rcases List.mem_append.1 hb with hb | hb
    · rcases traced_keeps hI hS rfl (ih _ hc hs hd hb (List.mem_append_right _ hu)) with
        hv | ⟨e, he, hh⟩
      · exact hv
      · rw [← List.append_assoc] at hc hs hd
        exact (hit_between hc hs hd hb he hu hh).elim
    · exact traced_creates hI hS rfl huses hb

/-- **D4b′, D4c and D4e imply D4a**, from an admissible initial state and
whatever the natives do. -/
theorem noDanglingFromContainment : NoDanglingFromContainment := by
  intro P N ν s₀ _ h0 hmc hstab hdead s h evs hT o post evs' hS
  have hI := refTraceInv_trace h0 hT
  have hc := (hmc s h evs hT).2 o post evs' hS
  have hs := (hstab s h evs hT).2 o post evs' hS
  have hd := (hdead s h evs hT).2 o post evs' hS
  unfold RefSafety.usesValid
  rw [List.all_eq_true]
  intro e he
  cases e
  case used i r =>
    obtain ⟨par, hb⟩ := hI.refs i r (traced_used hI.pre hS he)
    exact (valid_along h0 hT evs' hc hs hd hb he).1
  all_goals rfl

end MoveBytecode.Proofs
