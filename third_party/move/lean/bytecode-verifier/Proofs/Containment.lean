-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Dangling

/-!
# D4b′ from containment at each step

UNTRUSTED (checked by the kernel). D4b′ (`MutableContained`) asks that no
access reach over a `&mut` reference between its creation and a later use of
it. A reference used later is held at every step until that use, for a notion
of "held" that a step's uses imply and that a step only narrows, apart from
the references it borrows (`HeldFacts`); each reference is borrowed once. So
D4b′ follows when no access of a step reaches over a `&mut` reference held
before the step or borrowed by it (`mutableContained_of_steps`): a fact about
one step, which is what the verifier guarantees.

The references in the frames are held in this sense (`heldFacts_frames`): a
step's frames hold only references that were in them before or that the step
borrows (`traced_frameRefs`). The verifier's analysis tracks fewer: a
reference left in a local that is available on one path into a join only is
released, and the analysis lets the code write over its location. Such a
reference cannot be used again, so its own notion of held suffices.
-/

namespace MoveBytecode.Proofs

/-! ## Continuations of a trace -/

/-- Annotated steps that keep running, from `s` with shadow `h`, with their
events. -/
inductive Steps (P : Program) (N : NativeRel) (ν : NativeFlows) (s : State) (h : Shadow) :
    State → Shadow → List Event → Prop where
  | refl : Steps P N ν s h s h []
  | step {s₁ s₂ : State} {h₁ h₂ : Shadow} {G evs : List Event} :
      Steps P N ν s h s₁ h₁ G → TracedStep P N ν s₁ h₁ (.running s₂) (.running h₂) evs →
      Steps P N ν s h s₂ h₂ (G ++ evs)

/-- A trace continued by steps is a trace. -/
theorem _root_.MoveBytecode.Trace.steps {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s s' : State}
    {h h' : Shadow} {E G : List Event} (hT : Trace P N ν s₀ s h E)
    (hS : Steps P N ν s h s' h' G) : Trace P N ν s₀ s' h' (E ++ G) := by
  induction hS with
  | refl => simpa using hT
  | step _ hst ih =>
    rw [← List.append_assoc]
    exact .step ih hst

/-- The initial events access nothing. -/
theorem initial_not_accessed {P : Program} {s₀ : State} (h0 : AdmissibleInitial P s₀)
    {a : Access} {m : Location} {via : Option Nat} :
    Event.accessed a m via ∉ (Shadow.initial s₀).2 := by
  intro hb
  obtain ⟨f, args, tys, storage, held, -, -, -, hinit⟩ := h0
  obtain ⟨fr, localTys, hfr, hst, -⟩ := initialState_shape hinit
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
  · simp [initRefs] at hb

/-- **The step of an access**: an access among a trace's events is in one of
its steps. -/
theorem trace_split {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s : State} {h : Shadow}
    {E : List Event} (h0 : AdmissibleInitial P s₀) (hT : Trace P N ν s₀ s h E) {q : Nat}
    {a : Access} {m : Location} {via : Option Nat} (hq : E[q]? = some (.accessed a m via)) :
    ∃ s₁ h₁ E₁ s₂ h₂ E₂ G, Trace P N ν s₀ s₁ h₁ E₁ ∧
      TracedStep P N ν s₁ h₁ (.running s₂) (.running h₂) E₂ ∧ Steps P N ν s₂ h₂ s h G ∧
      E = E₁ ++ E₂ ++ G ∧ E₁.length ≤ q ∧ q < E₁.length + E₂.length := by
  induction hT generalizing q with
  | start => exact absurd (List.mem_of_getElem? hq) (initial_not_accessed h0)
  | @step s₁ s₂ h₁ h₂ E₁ E₂ hT₀ hS ih =>
    by_cases hlt : q < E₁.length
    · rw [List.getElem?_append_left hlt] at hq
      obtain ⟨t₁, k₁, F₁, t₂, k₂, F₂, G, hT₁, hS₁, hG, hE, hle, hlt'⟩ := ih hq
      exact ⟨t₁, k₁, F₁, t₂, k₂, F₂, G ++ E₂, hT₁, hS₁, .step hG hS, by simp [hE], hle, hlt'⟩
    · have := (List.getElem?_eq_some_iff.1 hq).1
      rw [List.length_append] at this
      exact ⟨s₁, h₁, E₁, s₂, h₂, E₂, [], hT₀, hS, .refl, by simp, by omega, by omega⟩

/-! ## What the frames hold, step by step -/

/-- What a predicate `Held s h i r`, "the frames of `s` hold reference `i`
with value `r`", must satisfy along the traces from `s₀` for D4b′ to follow
from containment at each step: it holds only of references in the frames; a
step uses only held references; and a reference held after a step was held
before it, or the step borrowed it. The references in the frames are such a
predicate (`heldFacts_frames`); the verifier's is narrower: it tracks only the
references its analysis can still use. -/
structure HeldFacts (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State)
    (Held : State → Shadow → Nat → Reference → Prop) : Prop where
  occ : ∀ s h E, Trace P N ν s₀ s h E → ∀ i r, Held s h i r →
    (i, Value.ref r) ∈ framesOcc h.frames s.frames
  used : ∀ s h E, Trace P N ν s₀ s h E → ∀ o post evs, TracedStep P N ν s h o post evs →
    ∀ i r, Event.used i r ∈ evs → Held s h i r
  back : ∀ s h E, Trace P N ν s₀ s h E → ∀ s' h' evs,
    TracedStep P N ν s h (.running s') (.running h') evs → ∀ i r, Held s' h' i r →
      Held s h i r ∨ ∃ par, Event.borrowed i r par ∈ evs

/-- The references in the frames. -/
theorem heldFacts_frames {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
    (h0 : AdmissibleInitial P s₀) :
    HeldFacts P N ν s₀ fun s h i r => (i, Value.ref r) ∈ framesOcc h.frames s.frames where
  occ _ _ _ _ _ _ hm := hm
  used _ _ _ hT _ _ _ hS _ _ hu := traced_used (refTraceInv_trace h0 hT).pre hS hu
  back _ _ _ hT _ _ _ hS _ _ hm := traced_frameRefs (refTraceInv_trace h0 hT) hS rfl rfl hm

/-- **A reference held after some steps** that none of them borrowed was held
before them. -/
theorem held_back {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s s' : State}
    {h h' : Shadow} {E G : List Event} {Held : State → Shadow → Nat → Reference → Prop}
    (hH : HeldFacts P N ν s₀ Held) (hT : Trace P N ν s₀ s h E) (hS : Steps P N ν s h s' h' G)
    {d : Nat} {r : Reference} (hm : Held s' h' d r) (hnb : d ∉ borrowedIds G) : Held s h d r := by
  induction hS with
  | refl => exact hm
  | step hS₁ hst ih =>
    rw [borrowedIds_append, List.mem_append, not_or] at hnb
    rcases hH.back _ _ _ (hT.steps hS₁) _ _ _ hst _ _ hm with hm' | ⟨par, hb⟩
    · exact ih hm' hnb.1
    · exact absurd (mem_borrowedIds.2 ⟨r, par, hb⟩) hnb.2

/-- **A reference some steps use** that none of them borrowed was held before
them. -/
theorem used_held {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s s' : State}
    {h h' : Shadow} {E G : List Event} {Held : State → Shadow → Nat → Reference → Prop}
    (hH : HeldFacts P N ν s₀ Held) (hT : Trace P N ν s₀ s h E) (hS : Steps P N ν s h s' h' G)
    {d : Nat} {r : Reference} (hu : Event.used d r ∈ G) (hnb : d ∉ borrowedIds G) :
    Held s h d r := by
  induction hS with
  | refl => cases hu
  | step hS₁ hst ih =>
    rw [borrowedIds_append, List.mem_append, not_or] at hnb
    rcases List.mem_append.1 hu with hu | hu
    · exact ih hu hnb.1
    · exact held_back hH hT hS₁ (hH.used _ _ _ (hT.steps hS₁) _ _ _ hst _ _ hu) hnb.1

/-! ## D4b′ on events -/

/-- An indexed event after position `p`. -/
theorem mem_zipIdx_drop {E : List Event} {p q : Nat} {e : Event}
    (h : (e, q) ∈ E.zipIdx.drop (p + 1)) : p < q ∧ E[q]? = some e := by
  obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 h
  rw [List.getElem?_drop, List.getElem?_zipIdx] at hk
  cases he : E[p + 1 + k]? with
  | none => rw [he] at hk; cases hk
  | some e' =>
    rw [he] at hk
    simp only [Option.map_some, Option.some.injEq, Prod.mk.injEq] at hk
    obtain ⟨rfl, hq⟩ := hk
    refine ⟨by omega, ?_⟩
    rw [← he]
    congr 1
    omega

/-- An event after position `q`. -/
theorem mem_drop_pos {E : List Event} {q : Nat} {e : Event} (h : e ∈ E.drop (q + 1)) :
    ∃ u, q < u ∧ E[u]? = some e := by
  obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 h
  rw [List.getElem?_drop] at hk
  exact ⟨q + 1 + k, by omega, hk⟩

/-- **D4b′ on events, by positions**: no access between the creation of a
`&mut` reference and a later use of it reaches over it. -/
theorem contained_of {E : List Event}
    (h : ∀ (p q u i : Nat) (r : Reference) (par : Option Nat) (a : Access) (m : Location)
      (via : Option Nat) (r' : Reference), E[p]? = some (.borrowed i r par) → r.mutable = true →
      E[q]? = some (.accessed a m via) → E[u]? = some (.used i r') → p < q → q < u →
      Access.reachesOver a m via r.location = false) : RefSafety.contained E = true := by
  unfold RefSafety.contained
  rw [List.all_eq_true]
  rintro ⟨e, p⟩ hep
  have hp : E[p]? = some e := List.mem_zipIdx_iff_getElem?.1 hep
  cases e
  case borrowed i r par =>
    simp only [Event.borrowed?]
    cases hm : r.mutable
    · rfl
    · simp only [Bool.not_true, Bool.false_or, List.all_eq_true]
      rintro ⟨e', q⟩ heq
      obtain ⟨hpq, hq⟩ := mem_zipIdx_drop heq
      cases e'
      case accessed a m via =>
        simp only [Event.accessed?]
        cases hr : Access.reachesOver a m via r.location
        · rfl
        · simp only [Bool.not_true, Bool.false_or, Bool.not_eq_true']
          unfold RefSafety.usedItselfAfter
          rw [List.any_eq_false]
          intro e'' he''
          obtain ⟨u, hqu, hu⟩ := mem_drop_pos he''
          cases e''
          case used d r' =>
            simp only [Event.used?, beq_iff_eq]
            rintro rfl
            have := h p q u d r par a m via r' hp hm hq hu hpq hqu
            rw [hr] at this
            cases this
          all_goals simp [Event.used?]
      all_goals simp [Event.accessed?]
  all_goals simp [Event.borrowed?]

/-! ## D4b′ from containment at each step -/

/-- What the verifier guarantees of one step: no access of the step reaches
over a `&mut` reference held before it, or borrowed by it before the access. -/
def StepsContained (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State)
    (Held : State → Shadow → Nat → Reference → Prop) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs → ∀ o post evs', TracedStep P N ν s h o post evs' →
    ∀ a m via, Event.accessed a m via ∈ evs' → ∀ i r,
      (Held s h i r ∨ ∃ (par : Option Nat) (j k : Nat), evs'[j]? = some (Event.borrowed i r par) ∧
        evs'[k]? = some (Event.accessed a m via) ∧ j < k) →
      r.mutable = true → Access.reachesOver a m via r.location = false

/-- The core of `mutableContained_of_steps`: an access of the trace `E`, then
a use in `E` or in later events `F` of references held after `E`. -/
theorem contained_core {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s : State}
    {h : Shadow} {E F : List Event} {Held : State → Shadow → Nat → Reference → Prop}
    (h0 : AdmissibleInitial P s₀) (hH : HeldFacts P N ν s₀ Held)
    (hVS : StepsContained P N ν s₀ Held) (hT : Trace P N ν s₀ s h E)
    (hF : ∀ d r, Event.used d r ∈ F → Held s h d r) (hnd : (borrowedIds E).Nodup)
    {p q u i : Nat} {r r' : Reference} {par : Option Nat} {a : Access} {m : Location}
    {via : Option Nat} (hp : (E ++ F)[p]? = some (.borrowed i r par)) (hm : r.mutable = true)
    (hq : (E ++ F)[q]? = some (.accessed a m via)) (hu : (E ++ F)[u]? = some (.used i r'))
    (hpq : p < q) (hqu : q < u) (hqE : q < E.length) :
    Access.reachesOver a m via r.location = false := by
  rw [List.getElem?_append_left hqE] at hq
  rw [List.getElem?_append_left (by omega)] at hp
  obtain ⟨s₁, h₁, E₁, s₂, h₂, E₂, G, hT₁, hst, hG, rfl, hle, hlt⟩ := trace_split h0 hT hq
  have hI₁ := refTraceInv_trace h0 hT₁
  have hT₂ : Trace P N ν s₀ s₂ h₂ (E₁ ++ E₂) := .step hT₁ hst
  have hb : Event.borrowed i r par ∈ E₁ ++ E₂ ++ G := List.mem_of_getElem? hp
  have hpE₁₂ : p < (E₁ ++ E₂).length := by rw [List.length_append]; omega
  have hb₁₂ : Event.borrowed i r par ∈ E₁ ++ E₂ := by
    rw [List.append_assoc, ← List.append_assoc E₁, List.getElem?_append_left hpE₁₂] at hp
    exact List.mem_of_getElem? hp
  -- `i` is not borrowed by the steps after the access's step.
  have hnG : i ∉ borrowedIds G := by
    intro hiG
    rw [borrowedIds_append, List.nodup_append] at hnd
    exact hnd.2.2 i (mem_borrowedIds.2 ⟨r, par, hb₁₂⟩) i hiG rfl
  -- The access is in the step from `s₁`.
  have hqE₂ : Event.accessed a m via ∈ E₂ := by
    rw [List.append_assoc, List.getElem?_append_right hle,
      List.getElem?_append_left (by omega)] at hq
    exact List.mem_of_getElem? hq
  -- The reference used at `u` is held at `s₁`, or the step borrows it.
  have hheld : Held s₁ h₁ i r' ∨ ∃ par', Event.borrowed i r' par' ∈ E₂ := by
    by_cases hu₂ : u < E₁.length + E₂.length
    · have hu' : Event.used i r' ∈ E₂ := by
        rw [List.append_assoc, List.append_assoc, List.getElem?_append_right (by omega),
          List.getElem?_append_left (by omega)] at hu
        exact List.mem_of_getElem? hu
      exact Or.inl (hH.used _ _ _ hT₁ _ _ _ hst _ _ hu')
    · have h₂ : Held s₂ h₂ i r' := by
        by_cases huE : u < (E₁ ++ E₂ ++ G).length
        · have hu' : Event.used i r' ∈ G := by
            rw [List.getElem?_append_left huE, List.getElem?_append_right
              (by rw [List.length_append]; omega)] at hu
            exact List.mem_of_getElem? hu
          exact used_held hH hT₂ hG hu' hnG
        · rw [List.getElem?_append_right (by omega)] at hu
          exact held_back hH hT₂ hG (hF i r' (List.mem_of_getElem? hu)) hnG
      exact hH.back _ _ _ hT₁ _ _ _ hst _ _ h₂
  -- It is `r`: each reference is borrowed once.
  have hrr : r' = r := by
    rcases hheld with hh | ⟨par', hb'⟩
    · obtain ⟨par', hb'⟩ := hI₁.refs i r' (hH.occ _ _ _ hT₁ _ _ hh)
      exact (borrowed_unique hnd (List.mem_append_left _ (List.mem_append_left _ hb')) hb).1
    · exact (borrowed_unique hnd (List.mem_append_left _ (List.mem_append_right _ hb')) hb).1
  subst hrr
  refine hVS s₁ h₁ E₁ hT₁ _ _ E₂ hst a m via hqE₂ i r' ?_ hm
  rcases hheld with hh | ⟨par', hb'⟩
  · exact Or.inl hh
  · -- The borrow is in the access's step, before the access.
    right
    have hpE₁ : E₁.length ≤ p := by
      rcases Nat.lt_or_ge p E₁.length with h1 | h1
      · exfalso
        have hp1 := hp
        rw [List.append_assoc, List.getElem?_append_left h1] at hp1
        have hb1 := List.mem_of_getElem? hp1
        rw [borrowedIds_append, borrowedIds_append, List.nodup_append, List.nodup_append] at hnd
        exact hnd.1.2.2 i (mem_borrowedIds.2 ⟨_, _, hb1⟩) i (mem_borrowedIds.2 ⟨_, _, hb'⟩) rfl
      · exact h1
    refine ⟨par, p - E₁.length, q - E₁.length, ?_, ?_, by omega⟩
    · have hp1 := hp
      rw [List.append_assoc, List.getElem?_append_right hpE₁,
        List.getElem?_append_left (by omega)] at hp1
      exact hp1
    · have hq1 := hq
      rw [List.append_assoc, List.getElem?_append_right hle,
        List.getElem?_append_left (by omega)] at hq1
      exact hq1

/-- **D4b′ from containment at each step**, from an admissible initial state:
when no access of a step reaches over a `&mut` reference held before it, for
a predicate of the held references with `HeldFacts`, no access reaches over a
`&mut` reference still to be used. -/
theorem mutableContained_of_steps {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
    {Held : State → Shadow → Nat → Reference → Prop} (h0 : AdmissibleInitial P s₀)
    (hH : HeldFacts P N ν s₀ Held) (hVS : StepsContained P N ν s₀ Held) :
    MutableContained P N ν s₀ := by
  intro s h E hT
  have hI := refTraceInv_trace h0 hT
  refine ⟨contained_of fun p q u i r par a m via r' hp hm hq hu hpq hqu => ?_,
    fun o post evs' hS => contained_of fun p q u i r par a m via r' hp hm hq hu hpq hqu => ?_⟩
  · have hqE : q < E.length := (List.getElem?_eq_some_iff.1 hq).1
    rw [← List.append_nil E] at hp hq hu
    exact contained_core h0 hH hVS hT (F := []) (fun _ _ h => by cases h) hI.unique
      hp hm hq hu hpq hqu hqE
  · by_cases hqE : q < E.length
    · exact contained_core h0 hH hVS hT (fun d r hu => hH.used _ _ _ hT _ _ _ hS d r hu)
        hI.unique hp hm hq hu hpq hqu hqE
    -- The access and the use are in the last step.
    have hq' : Event.accessed a m via ∈ evs' := by
      rw [List.getElem?_append_right (by omega)] at hq
      exact List.mem_of_getElem? hq
    have hu' : Event.used i r' ∈ evs' := by
      rw [List.getElem?_append_right (by omega)] at hu
      exact List.mem_of_getElem? hu
    have hheld := hH.used _ _ _ hT _ _ _ hS _ _ hu'
    by_cases hpE : p < E.length
    · rw [List.getElem?_append_left hpE] at hp
      obtain ⟨par', hb'⟩ := hI.refs i r' (hH.occ _ _ _ hT _ _ hheld)
      have hrr := (borrowed_unique hI.unique hb' (List.mem_of_getElem? hp)).1
      subst hrr
      exact hVS s h E hT o post evs' hS a m via hq' i r' (Or.inl hheld) hm
    · rw [List.getElem?_append_right (by omega)] at hp
      have hq'' : evs'[q - E.length]? = some (Event.accessed a m via) := by
        rw [List.getElem?_append_right (by omega)] at hq; exact hq
      exact hVS s h E hT o post evs' hS a m via hq' i r
        (Or.inr ⟨par, p - E.length, q - E.length, hp, hq'', by omega⟩) hm

end MoveBytecode.Proofs
