-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.StepsDefs
import Proofs.RefAccess

/-!
# D4c and D4e from what each step guarantees

UNTRUSTED (checked by the kernel). D4c (`ImmutableStable`) and D4e
(`DeadSlotsUnborrowed`) constrain the steps while a reference is live: until
the last use of the reference or of one derived from it. A derived reference
is inside the one it is derived from, and is created by a step that uses
that one (`BorrowsInside`); so a reference derived after a step from one
created before it makes that one used after the step. Along a derivation,
some reference created before the step is used after it (`live_chain`), and
it is held before the step (D4c) or after it (D4e), by `HeldFacts`. What each
step guarantees of the held references (`StepsStable`, `StepsDead`) then
gives D4c and D4e (`immutableStable_of_steps`, `deadSlots_of_steps`).
-/

namespace MoveBytecode.Proofs

/-! ## The steps of a trace -/

/-- What the initial events are: creations, the transaction's references,
and the entry. -/
theorem initial_events {P : Program} {s₀ : State} (h0 : AdmissibleInitial P s₀) {e : Event}
    (he : e ∈ (Shadow.initial s₀).2) :
    (∃ i w c, e = .created i w c) ∨ (∃ i r, e = .borrowed i r none) ∨
      ∃ f ps, e = .entered f ps := by
  obtain ⟨f, args, tys, storage, held, -, -, -, hinit⟩ := h0
  obtain ⟨fr, localTys, hfr, hst, -⟩ := initialState_shape hinit
  obtain ⟨ls, evs₁, n₁, hf, -, -, -, -, -, -, hev⟩ := locals_fold fr.locals.toList [] 0 []
  obtain ⟨sts, e₂, n₂, hfS, -, -, -, -, -, hevS⟩ :=
    fresh_fold .storage (s₀.storage.map (·.value)) [] n₁ []
  obtain ⟨hts, e₃, n₃, hfH, -, -, -, -, -, hevH⟩ := fresh_fold .transaction s₀.held [] n₂ []
  rw [initial_eq hfr] at he
  simp only [initL, initK, initS, initH, hst, freshAll_nil, hf, hfS, hfH, List.nil_append,
    List.append_nil, List.mem_append] at he
  rcases he with ((he | he) | he) | he
  · obtain ⟨i, w, rfl, -⟩ := hev _ he; exact Or.inl ⟨i, w, _, rfl⟩
  · obtain ⟨i, w, rfl, -⟩ := hevS _ he; exact Or.inl ⟨i, w, _, rfl⟩
  · obtain ⟨i, w, rfl, -⟩ := hevH _ he; exact Or.inl ⟨i, w, _, rfl⟩
  · simp only [initRefs, List.mem_append, List.mem_map, List.mem_singleton] at he
    rcases he with ⟨⟨i, r⟩, -, rfl⟩ | rfl
    · exact Or.inr (Or.inl ⟨i, r, rfl⟩)
    · exact Or.inr (Or.inr ⟨_, _, rfl⟩)

/-- **The step of an event** of a trace that is not an initial event. -/
theorem trace_split' {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s : State}
    {h : Shadow} {E : List Event} (hT : Trace P N ν s₀ s h E) {q : Nat} {e : Event}
    (hq : E[q]? = some e) (he : e ∉ (Shadow.initial s₀).2) :
    ∃ s₁ h₁ E₁ s₂ h₂ E₂ G, Trace P N ν s₀ s₁ h₁ E₁ ∧
      TracedStep P N ν s₁ h₁ (.running s₂) (.running h₂) E₂ ∧ Steps P N ν s₂ h₂ s h G ∧
      E = E₁ ++ E₂ ++ G ∧ E₁.length ≤ q ∧ q < E₁.length + E₂.length := by
  induction hT generalizing q with
  | start => exact absurd (List.mem_of_getElem? hq) he
  | @step s₁ s₂ h₁ h₂ E₁ E₂ hT₀ hS ih =>
    by_cases hlt : q < E₁.length
    · rw [List.getElem?_append_left hlt] at hq
      obtain ⟨t₁, k₁, F₁, t₂, k₂, F₂, G, hT₁, hS₁, hG, hE, hle, hlt'⟩ := ih hq
      exact ⟨t₁, k₁, F₁, t₂, k₂, F₂, G ++ E₂, hT₁, hS₁, .step hG hS, by simp [hE], hle, hlt'⟩
    · have := (List.getElem?_eq_some_iff.1 hq).1
      rw [List.length_append] at this
      exact ⟨s₁, h₁, E₁, s₂, h₂, E₂, [], hT₀, hS, .refl, by simp, by omega, by omega⟩

/-- A use is no initial event. -/
theorem used_not_initial {P : Program} {s₀ : State} (h0 : AdmissibleInitial P s₀) {d : Nat}
    {r : Reference} : Event.used d r ∉ (Shadow.initial s₀).2 := fun he => by
  rcases initial_events h0 he with ⟨_, _, _, h⟩ | ⟨_, _, h⟩ | ⟨_, _, h⟩ <;> cases h

/-- A reference borrowed from another is no initial event. -/
theorem derived_not_initial {P : Program} {s₀ : State} (h0 : AdmissibleInitial P s₀) {d t : Nat}
    {r : Reference} : Event.borrowed d r (some t) ∉ (Shadow.initial s₀).2 := fun he => by
  rcases initial_events h0 he with ⟨_, _, _, h⟩ | ⟨_, _, h⟩ | ⟨_, _, h⟩ <;> cases h

/-- **The value a reference is used with** is the value it was borrowed with. -/
theorem used_value {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s : State} {h : Shadow}
    {E F : List Event} {Held : State → Shadow → Nat → Reference → Prop}
    (h0 : AdmissibleInitial P s₀) (hH : HeldFacts P N ν s₀ Held) (hT : Trace P N ν s₀ s h E)
    (hF : ∀ d r, Event.used d r ∈ F → Held s h d r) {u d : Nat} {r : Reference}
    (hu : (E ++ F)[u]? = some (.used d r)) : ∃ par, Event.borrowed d r par ∈ E := by
  by_cases huE : u < E.length
  · rw [List.getElem?_append_left huE] at hu
    obtain ⟨s₁, h₁, E₁, s₂, h₂, E₂, G, hT₁, hst, -, rfl, hle, hlt⟩ :=
      trace_split' hT hu (used_not_initial h0)
    have hu' : Event.used d r ∈ E₂ := by
      rw [List.append_assoc, List.getElem?_append_right hle,
        List.getElem?_append_left (by omega)] at hu
      exact List.mem_of_getElem? hu
    have hI₁ := refTraceInv_trace h0 hT₁
    obtain ⟨par, hb⟩ := hI₁.refs d r (traced_used hI₁.pre hst hu')
    exact ⟨par, List.mem_append_left _ (List.mem_append_left _ hb)⟩
  · rw [List.getElem?_append_right (by omega)] at hu
    have hI := refTraceInv_trace h0 hT
    exact hI.refs d r (hH.occ _ _ _ hT _ _ (hF d r (List.mem_of_getElem? hu)))

/-- **A reference borrowed before an event and used after it is held** before
the event's step, or borrowed by it; and, when the use comes after the step,
after it. -/
theorem held_across {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s : State}
    {h : Shadow} {E F : List Event} {Held : State → Shadow → Nat → Reference → Prop}
    (hH : HeldFacts P N ν s₀ Held) (hT : Trace P N ν s₀ s h E)
    (hF : ∀ d r, Event.used d r ∈ F → Held s h d r) {q : Nat} {e : Event}
    (hq : E[q]? = some e) (he : e ∉ (Shadow.initial s₀).2) {c u : Nat} {rc : Reference}
    (hcq : ∀ p r par, E[p]? = some (.borrowed c r par) → p < q)
    (hu : (E ++ F)[u]? = some (.used c rc)) (hqu : q < u) :
    ∃ s₁ h₁ E₁ s₂ h₂ E₂ G, Trace P N ν s₀ s₁ h₁ E₁ ∧
      TracedStep P N ν s₁ h₁ (.running s₂) (.running h₂) E₂ ∧ Steps P N ν s₂ h₂ s h G ∧
      E = E₁ ++ E₂ ++ G ∧ E₁.length ≤ q ∧ q < E₁.length + E₂.length ∧
      (Held s₁ h₁ c rc ∨ ∃ par, Event.borrowed c rc par ∈ E₂) ∧
      (E₁.length + E₂.length ≤ u → Held s₂ h₂ c rc) := by
  obtain ⟨s₁, h₁, E₁, s₂, h₂, E₂, G, hT₁, hst, hG, rfl, hle, hlt⟩ := trace_split' hT hq he
  have hT₂ : Trace P N ν s₀ s₂ h₂ (E₁ ++ E₂) := .step hT₁ hst
  -- `c` is not borrowed by the steps after the event's step.
  have hnG : c ∉ borrowedIds G := by
    intro hcG
    obtain ⟨r, par, hb⟩ := mem_borrowedIds.1 hcG
    obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hb
    have hkl := (List.getElem?_eq_some_iff.1 hk).1
    have := hcq ((E₁ ++ E₂).length + k) r par (by
      rw [List.getElem?_append_right (by simp), Nat.add_sub_cancel_left]; exact hk)
    rw [List.length_append] at this
    omega
  have hpost : E₁.length + E₂.length ≤ u → Held s₂ h₂ c rc := by
    intro hu₂
    by_cases huE : u < (E₁ ++ E₂ ++ G).length
    · have hu' : Event.used c rc ∈ G := by
        rw [List.getElem?_append_left huE, List.getElem?_append_right
          (by rw [List.length_append]; omega)] at hu
        exact List.mem_of_getElem? hu
      exact used_held hH hT₂ hG hu' hnG
    · rw [List.getElem?_append_right (by omega)] at hu
      exact held_back hH hT₂ hG (hF c rc (List.mem_of_getElem? hu)) hnG
  refine ⟨s₁, h₁, E₁, s₂, h₂, E₂, G, hT₁, hst, hG, rfl, hle, hlt, ?_, hpost⟩
  by_cases hu₂ : u < E₁.length + E₂.length
  · have hu' : Event.used c rc ∈ E₂ := by
      rw [List.append_assoc, List.append_assoc, List.getElem?_append_right (by omega),
        List.getElem?_append_left (by omega)] at hu
      exact List.mem_of_getElem? hu
    exact Or.inl (hH.used _ _ _ hT₁ _ _ _ hst _ _ hu')
  · exact hH.back _ _ _ hT₁ _ _ _ hst _ _ (hpost (by omega))

/-! ## Derivations -/

/-- Reference `d` derives from reference `r` along the `borrowed` events of
`E`. -/
inductive Derives (E : List Event) (r : Nat) : Nat → Prop
  | refl : Derives E r r
  | step {d t : Nat} {rd : Reference} : Event.borrowed d rd (some t) ∈ E → Derives E r t →
      Derives E r d

theorem creation?_mem {E : List Event} {d : Nat} {rd : Reference} {par : Option Nat}
    (h : RefSafety.creation? E d = some (rd, par)) : Event.borrowed d rd par ∈ E := by
  unfold RefSafety.creation? at h
  obtain ⟨e, he, hf⟩ := List.exists_of_findSome?_eq_some h
  cases e
  case borrowed i r p =>
    simp only [Event.borrowed?, Option.bind_some] at hf
    split at hf
    · next hi =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hf
      obtain ⟨rfl, rfl⟩ := hf
      simp only [beq_iff_eq] at hi
      subst hi
      exact he
    · cases hf
  all_goals simp [Event.borrowed?] at hf

/-- The executable derivation is a derivation. -/
theorem derives_of_derivesFrom {E : List Event} {d r : Nat}
    (h : RefSafety.derivesFrom E d r = true) : Derives E r d := by
  unfold RefSafety.derivesFrom at h
  generalize E.length = n at h
  induction n generalizing d with
  | zero => simp [RefSafety.derivesFrom.go] at h
  | succ n ih =>
    simp only [RefSafety.derivesFrom.go, Bool.or_eq_true, beq_iff_eq] at h
    rcases h with rfl | h
    · exact .refl
    · split at h
      · next rd t hc => exact .step (creation?_mem hc) (ih h)
      · cases h
      · cases h

/-! ## What the steps that end a slot record -/

/-- How an instruction ends a slot: `MoveLoc` makes its local unavailable and
`Ret` ends its frame, each as its last event. -/
inductive EndsKind (fr : Frame) (instr : Bytecode) (evs : List Event) : Event → Prop where
  | vacated {j : Nat} : instr = .moveLoc j →
      (∃ A, evs = A ++ [.vacated (.local_ fr.id j)] ∧ ∀ e ∈ A, e.ends? = none) →
      EndsKind fr instr evs (.vacated (.local_ fr.id j))
  | frameEnded {rs : List Nat} : instr = .ret →
      (∃ A, evs = A ++ [.frameEnded fr.id rs] ∧ ∀ e ∈ A, e.ends? = none) →
      EndsKind fr instr evs (.frameEnded fr.id rs)

/-- Events none of which ends a slot. -/
theorem no_ends {evs : List Event} (h : evs.all (fun e => e.ends?.isNone) = true) {e : Event}
    (he : e ∈ evs) {ended : Root → Bool} (hend : e.ends? = some ended) : False := by
  have := List.all_eq_true.1 h e he
  rw [hend] at this
  cases this

@[simp] theorem fresh_all_ends (c : Creator) (v : Value) (n : Nat) :
    (IdTree.fresh c v n).2.2.all (fun e => e.ends?.isNone) = true := by
  rw [(fresh_spec c v n).2.2.2]; simp [Event.ends?]

/-- Proves that no event of an explicit list of a step's events ends a slot. -/
macro "ends_none" : tactic =>
  `(tactic| (simp only [List.all_append, List.all_cons, List.all_nil, fresh_all_ends, List.all_map,
      Function.comp_def, Bool.and_true, Bool.true_and]
             first | done | simp [Event.ends?]))

/-- **The events that end a slot.** -/
theorem exec_ends {P : Program} {m : ModuleId} {instr : Bytecode} {s : State} {fr : Frame}
    {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event} {g : ShadowFrame}
    {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) {e : Event} (he : e ∈ evs)
    {ended : Root → Bool} (hend : e.ends? = some ended) : EndsKind fr instr evs e := by
  cases instr <;>
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq, Prod.mk.injEq, guard, reduceCtorEq] at hex
  all_goals (try simp_all [Event.ends?])
  case ret =>
    split at hex <;>
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_append, List.mem_map, List.mem_singleton] at he
      rcases he with ⟨_, -, rfl⟩ | rfl
      · cases hend
      · refine .frameEnded rfl ⟨_, rfl, fun e he => ?_⟩
        simp only [List.mem_map] at he
        obtain ⟨_, -, rfl⟩ := he
        rfl
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldConst | ldTrue | ldFalse =>
    obtain ⟨a, -, -, rfl⟩ := hex
    exact (no_ends (by simp) he hend).elim
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32
    | castI64 | castI128 | castI256 | not | negate | add | sub | mul | div | mod | bitOr | bitAnd
    | xor | or | and | lt | gt | le | ge | shl | shr =>
    obtain ⟨-, a, -, -, rfl⟩ := hex
    exact (no_ends (by simp) he hend).elim
  case pack | packVariant | vecPack | packGeneric | packVariantGeneric | packClosure
    | packClosureGeneric =>
    obtain ⟨a, -, -, -, rfl⟩ := hex
    exact (no_ends (by ends_none) he hend).elim
  case callClosure =>
    split at hex
    all_goals (try split at hex)
    all_goals (try split at hex)
    all_goals (try split at hex)
    all_goals first
      | cases hex
      | (obtain ⟨_, -, hex⟩ := Option.bind_eq_some_iff.1 hex
         simp only [Option.some.injEq, Prod.mk.injEq] at hex
         obtain ⟨-, rfl⟩ := hex
         exact (no_ends (by ends_none) he hend).elim)
      | simp at hex
  case unpack | unpackVariant | vecUnpack | unpackGeneric | unpackVariantGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨v, -, -, rfl⟩ := hex
      exact (no_ends (by ends_none) he hend).elim
    · cases hex
  case eq | neq =>
    obtain ⟨a, b, ⟨-, a₁, -, -, rfl⟩, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact (no_ends (by ends_none) he hend).elim
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact (no_ends (by simp) he hend).elim
  case copyLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact (no_ends (by simp only [List.all_cons]; split <;> ends_none) he hend).elim
    · cases hex
  case moveLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_cons, List.mem_append, List.not_mem_nil, or_false] at he
      rcases he with rfl | he | rfl
      · cases hend
      · split at he
        · rw [List.mem_singleton] at he
          subst he
          simp at hend
        · simp at he
      · refine .vacated rfl ⟨_, List.cons_append.symm, fun e he => ?_⟩
        rcases List.mem_cons.1 he with rfl | he
        · rfl
        · split at he
          · rw [List.mem_singleton] at he
            subst he
            rfl
          · simp at he
    · cases hex
  case stLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact (no_ends (by simp only [List.all_cons]; split <;> ends_none) he hend).elim
    · cases hex
  case call | callGeneric =>
    split at hex
    · split at hex
      · obtain ⟨_, -, hex⟩ := Option.bind_eq_some_iff.1 hex
        simp only [Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, rfl⟩ := hex
        exact (no_ends (by ends_none) he hend).elim
      · cases hex
    · cases hex
  case readRef =>
    obtain ⟨a, b, -, a₁, -, -, rfl⟩ := hex
    exact (no_ends (by ends_none) he hend).elim
  case writeRef =>
    obtain ⟨a, b, -, a₁, -, a₂, -, -, rfl⟩ := hex
    exact (no_ends (by ends_none) he hend).elim
  case freezeRef =>
    obtain ⟨a, b, -, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact (no_ends (by ends_none) he hend).elim
    · cases hm
  case testVariant | testVariantGeneric =>
    obtain ⟨a, b, -, hm⟩ := hex
    split at hm
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨x, hx, -, rfl⟩ := hm
      obtain ⟨-, -, v, -, rfl⟩ := hx
      exact (no_ends (by ends_none) he hend).elim
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
        exact (no_ends (by ends_none) he hend).elim
      · cases hm
    · cases hm
  case vecLen =>
    obtain ⟨a, b, -, a₁, b₁, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    exact (no_ends (by ends_none) he hend).elim
  case vecImmBorrow | vecMutBorrow =>
    obtain ⟨a, b, -, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact (no_ends (by ends_none) he hend).elim
    · cases hm
  case vecPushBack =>
    obtain ⟨a, -, a₁, b, -, a₂, -, -, rfl⟩ := hex
    exact (no_ends (by ends_none) he hend).elim
  case vecPopBack =>
    obtain ⟨a, b, -, a₁, -, a₂, -, a₃, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact (no_ends (by ends_none) he hend).elim
    · cases hm
  case vecSwap =>
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨x, -, y, -, -, rfl⟩ := hex
        exact (no_ends (by ends_none) he hend).elim
      · cases hex
    · cases hex
  case exists_ =>
    obtain ⟨a, -, a₁, b, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    exact (no_ends (by ends_none) he hend).elim
  case existsGeneric =>
    obtain ⟨_, _, _, _, _, _, _, _, a, b, ⟨_, a', _, _, rfl⟩, _, rfl⟩ := hex
    exact (no_ends (by ends_none) he hend).elim
  case moveFrom | moveFromGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff] at hex
      obtain ⟨v, -, hex⟩ := hex
      split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨k, -, t, -, -, rfl⟩ := hex
        exact (no_ends (by ends_none) he hend).elim
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
        exact (no_ends (by ends_none) he hend).elim
      · cases hm
    · cases hm
  case mutBorrowLoc | immBorrowLoc =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      exact (no_ends (by ends_none) he hend).elim
    · cases hex
  case mutBorrowGlobal | immBorrowGlobal | mutBorrowGlobalGeneric | immBorrowGlobalGeneric =>
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, -, -, rfl⟩ := hex
        exact (no_ends (by ends_none) he hend).elim
      · cases hex
    · cases hex

/-! ## What a traced step records -/

/-- A step that borrows writes nothing. -/
theorem traced_borrow_write {P : Program} {N : NativeRel} {ν : NativeFlows} {s : State}
    {h : Shadow} {o : Outcome} {post : ShadowPost} {evs : List Event}
    (hT : TracedStep P N ν s h o post evs) {d : Nat} {r : Reference} {par : Option Nat}
    (hb : Event.borrowed d r par ∈ evs) {l : Location} {via : Option Nat}
    (hw : Event.accessed .write l via ∈ evs) : False := by
  cases hT with
  | core _ hc =>
    obtain ⟨fr, rest, info, code, instr, -, -, -, -, hex⟩ := coreStep_exec hc
    have := exec_borrow_inspect hex hb hw
    cases this
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      obtain ⟨-, -, -, -, p, hp, -⟩ := nativeStep_borrowed hn hb
      obtain ⟨tys, -, hfits⟩ := nativeStep_fits hstep hn
      have hkb := places_borrow hfits (List.ne_nil_of_mem hp)
      obtain ⟨g, hs, -, -, h1⟩ := nativeStep_events hn hw
      rcases h1 with ⟨x, -, he⟩ | ⟨x, -, -, hku, he⟩ | ⟨x, -, he⟩ | ⟨_, _, _, he, -⟩ |
        ⟨_, _, _, he⟩ | ⟨_, _, _, he⟩
      · cases he
      · rw [hkb] at hku; cases hku
      all_goals cases he
    obtain ⟨rfl, rfl, rfl, g, hs, info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    have hm : d ∈ borrowedIds evs := mem_borrowedIds.2 ⟨r, par, hb⟩
    rw [hevs, nativeEvents_borrowedIds] at hm
    cases hm

/-- **A step that ends a slot ends it last, and borrows nothing.** -/
theorem traced_ends {P : Program} {N : NativeRel} {ν : NativeFlows} {s : State} {h : Shadow}
    {o : Outcome} {post : ShadowPost} {evs : List Event} (hpre : Pre false P s h)
    (hT : TracedStep P N ν s h o post evs) {e : Event} (he : e ∈ evs) {ended : Root → Bool}
    (hend : e.ends? = some ended) :
    (∃ A, evs = A ++ [e] ∧ ∀ e' ∈ A, e'.ends? = none) ∧
      ∀ d r par, Event.borrowed d r par ∉ evs := by
  cases hT with
  | core _ hc =>
    obtain ⟨fr, rest, info, code, instr, hsf, -, -, -, hex⟩ := coreStep_exec hc
    obtain ⟨hframes, hst, hhe, fresh⟩ := h
    cases hframes with
    | nil =>
      have hfit := hpre.fits
      rw [shadow_fits, hsf] at hfit
      simp [framesFit] at hfit
    | cons g hs =>
      have hnb : ∀ j, (instr = .moveLoc j ∨ instr = .ret) → ∀ d r par, Event.borrowed d r par ∉ evs := by
        intro j hi d r par hb
        obtain ⟨-, hnew⟩ := exec_borrowed rfl hex hb
        rcases hi with rfl | rfl <;>
        rcases hnew with ⟨h1, -⟩ | ⟨_, h1 | h1⟩ | ⟨_, h1 | h1 | h1 | h1⟩ |
          ⟨_, h1 | h1 | h1 | h1⟩ | ⟨_, h1 | h1⟩ | ⟨_, h1 | h1 | h1 | h1⟩ <;> cases h1
      cases exec_ends rfl hex he hend with
      | vacated hi hA => exact ⟨hA, hnb _ (Or.inl hi)⟩
      | frameEnded hi hA => exact ⟨hA, hnb 0 (Or.inr hi)⟩
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      rw [(nativeStep_quiet hn he).2.2.1] at hend
      cases hend
    obtain ⟨rfl, rfl, rfl, g, hs, info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at he
    simp only [nativeEvents, List.mem_append] at he
    rcases he with he | he
    · obtain ⟨_, _, _, _, rfl, -⟩ := consumedFrom_mem _ he
      cases hend
    · obtain ⟨_, _, _, rfl⟩ := (nativeResults_spec _ _ _ _ _ info.moduleId _).2.2 _ he
      cases hend

/-- The position of an event that ends a slot is the last of its step. -/
theorem ends_last {A : List Event} {e e' : Event} (hA : ∀ e' ∈ A, e'.ends? = none) {k : Nat}
    (hk : (A ++ [e])[k]? = some e') {ended : Root → Bool} (hend : e'.ends? = some ended) :
    k = A.length := by
  by_cases hlt : k < A.length
  · rw [List.getElem?_append_left hlt] at hk
    rw [hA e' (List.mem_of_getElem? hk)] at hend
    cases hend
  · have := (List.getElem?_eq_some_iff.1 hk).1
    simp at this
    omega

theorem coreStep_fits {P : Program} {s : State} {o : Outcome} {h : Shadow} {post : ShadowPost}
    {evs : List Event} (hc : Shadow.coreStep P s o h = some (post, evs)) : post.fits o = true := by
  unfold Shadow.coreStep at hc
  split at hc
  · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at hc
    obtain ⟨_, -, _, -, _, -, ⟨post', evs'⟩, -, hc⟩ := hc
    split at hc
    · next hf =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hc
      obtain ⟨rfl, -⟩ := hc
      exact hf
    · cases hc
  · cases hc

/-- Each reference is borrowed at one position. -/
theorem borrowed_pos_unique : ∀ {E : List Event}, (borrowedIds E).Nodup →
    ∀ {p₁ p₂ d : Nat} {r₁ r₂ : Reference} {par₁ par₂ : Option Nat},
    E[p₁]? = some (.borrowed d r₁ par₁) → E[p₂]? = some (.borrowed d r₂ par₂) → p₁ = p₂
  | [], _, _, _, _, _, _, _, _, h₁, _ => by simp at h₁
  | e :: E, hnd, p₁, p₂, d, r₁, r₂, par₁, par₂, h₁, h₂ => by
    have hnd' : (borrowedIds E).Nodup := by
      cases e <;> simp_all [borrowedIds, List.nodup_cons]
    cases p₁ with
    | zero =>
      cases p₂ with
      | zero => rfl
      | succ k =>
        simp only [List.getElem?_cons_zero, Option.some.injEq] at h₁
        subst h₁
        simp only [List.getElem?_cons_succ] at h₂
        simp only [borrowedIds, List.filterMap_cons, List.nodup_cons] at hnd
        exact absurd (mem_borrowedIds.2 ⟨_, _, List.mem_of_getElem? h₂⟩) hnd.1
    | succ k =>
      cases p₂ with
      | zero =>
        simp only [List.getElem?_cons_zero, Option.some.injEq] at h₂
        subst h₂
        simp only [List.getElem?_cons_succ] at h₁
        simp only [borrowedIds, List.filterMap_cons, List.nodup_cons] at hnd
        exact absurd (mem_borrowedIds.2 ⟨_, _, List.mem_of_getElem? h₁⟩) hnd.1
      | succ k' =>
        simp only [List.getElem?_cons_succ] at h₁ h₂
        rw [borrowed_pos_unique hnd' h₁ h₂]

/-- **The references of a trace and its last step are borrowed once.** -/
theorem traced_nodup {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s : State}
    {h : Shadow} {E : List Event} (h0 : AdmissibleInitial P s₀) (hT : Trace P N ν s₀ s h E)
    {o : Outcome} {post : ShadowPost} {evs : List Event}
    (hS : TracedStep P N ν s h o post evs) : (borrowedIds (E ++ evs)).Nodup := by
  have hI := refTraceInv_trace h0 hT
  by_cases hb : ∃ d r par, Event.borrowed d r par ∈ evs
  · obtain ⟨d, r, par, hb⟩ := hb
    cases hS with
    | core hstep hc =>
      obtain ⟨fr, rest, info, code, instr, -, -, -, -, hex⟩ := coreStep_exec hc
      obtain ⟨hframes, hst, hhe, fresh⟩ := h
      cases hframes with
      | nil => simp [Shadow.exec] at hex
      | cons g hs =>
        obtain ⟨htop, -⟩ := exec_borrowed rfl hex hb
        cases o with
        | running s' =>
          cases post with
          | running h' => exact (refTraceInv_core hI hstep hc).unique
          | returned _ _ =>
            have := coreStep_fits hc
            simp [ShadowPost.fits] at this
        | returned _ _ => simp [Shadow.topAfter] at htop
        | failed _ => simp [Shadow.topAfter] at htop
        | wrong _ => simp [Shadow.topAfter] at htop
    | native hstep _ hn =>
      rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
      case inr =>
        obtain ⟨hnd, hge⟩ := native_borrowedIds hn
        rw [borrowedIds_append]
        refine List.nodup_append.2 ⟨hI.unique, hnd, fun a ha b hb hab => ?_⟩
        have h1 := hI.below a ha
        have h2 := hge b hb
        omega
      obtain ⟨rfl, rfl, rfl, g, hs, info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
      have hm : d ∈ borrowedIds evs := mem_borrowedIds.2 ⟨r, par, hb⟩
      rw [hevs, nativeEvents_borrowedIds] at hm
      cases hm
  · have hnil : borrowedIds evs = [] := by
      rw [List.eq_nil_iff_forall_not_mem]
      intro d hd
      obtain ⟨r, par, hb'⟩ := mem_borrowedIds.1 hd
      exact hb ⟨d, r, par, hb'⟩
    rw [borrowedIds_append, hnil, List.append_nil]
    exact hI.unique

/-- What a derivation needs of the steps of a trace `E` and its last step's
events `F`. -/
theorem borrows_inside_all {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s : State}
    {h : Shadow} {E F : List Event} (h0 : AdmissibleInitial P s₀) (hT : Trace P N ν s₀ s h E)
    (hB : ∀ s h E, Trace P N ν s₀ s h E → ∀ o post evs, TracedStep P N ν s h o post evs →
      BorrowsInside evs)
    (hBF : BorrowsInside F) : BorrowsInside (E ++ F) := by
  intro d rd t hb
  rcases List.mem_append.1 hb with hb | hb
  · obtain ⟨p, hp⟩ := List.mem_iff_getElem?.1 hb
    obtain ⟨s₁, h₁, E₁, s₂, h₂, E₂, G, hT₁, hst, -, rfl, hle, hlt⟩ :=
      trace_split' hT hp (derived_not_initial h0)
    have hb' : Event.borrowed d rd (some t) ∈ E₂ := by
      rw [List.append_assoc, List.getElem?_append_right hle,
        List.getElem?_append_left (by omega)] at hp
      exact List.mem_of_getElem? hp
    obtain ⟨rt, hu, hin, him⟩ := hB _ _ _ hT₁ _ _ _ hst d rd t hb'
    exact ⟨rt, List.mem_append_left _ (List.mem_append_left _ (List.mem_append_right _ hu)),
      hin, him⟩
  · obtain ⟨rt, hu, hin, him⟩ := hBF d rd t hb
    exact ⟨rt, List.mem_append_right _ hu, hin, him⟩

/-- **Along a derivation, references are inside the first**, and immutable
when it is. -/
theorem derives_inside {E : List Event} (hnd : (borrowedIds E).Nodup) (hBI : BorrowsInside E)
    (hval : ∀ d x, Event.used d x ∈ E → ∃ par, Event.borrowed d x par ∈ E) {i : Nat}
    {ri : Reference} {pi : Option Nat} (hi : Event.borrowed i ri pi ∈ E) {c : Nat}
    (hc : Derives E i c) :
    ∀ {rc : Reference} {pc : Option Nat}, Event.borrowed c rc pc ∈ E →
      rc.location.Inside ri.location ∧ (ri.mutable = false → rc.mutable = false) := by
  induction hc with
  | refl =>
    intro rc pc hbc
    obtain ⟨rfl, -⟩ := borrowed_unique hnd hi hbc
    exact ⟨Location.Inside.refl _, id⟩
  | step hb _ ih =>
    intro rc pc hbc
    obtain ⟨rfl, -⟩ := borrowed_unique hnd hb hbc
    obtain ⟨rt, hu, hin, him⟩ := hBI _ _ _ hb
    obtain ⟨pt, hbt⟩ := hval _ _ hu
    obtain ⟨h1, h2⟩ := ih hbt
    exact ⟨hin.trans h1, fun hm => him (h2 hm)⟩

/-- **Along a derivation, some reference borrowed before a point is used after
it**, when each reference borrowed after the point has the one it is derived
from used after it; the used reference's location is inside it. -/
theorem live_chain {E : List Event} (hnd : (borrowedIds E).Nodup) {q : Nat}
    (hpar : ∀ p d rd t, E[p]? = some (.borrowed d rd (some t)) → q < p →
      ∃ u rt, E[u]? = some (.used t rt) ∧ q < u ∧ rd.location.Inside rt.location)
    (hval : ∀ d x, Event.used d x ∈ E → ∃ par, Event.borrowed d x par ∈ E)
    (hqb : ∀ d r par, E[q]? ≠ some (.borrowed d r par)) {i : Nat}
    (hiq : ∀ p r par, E[p]? = some (.borrowed i r par) → p < q) {d : Nat}
    (hd : Derives E i d) :
    ∀ u x, E[u]? = some (.used d x) → q < u →
      ∃ c u' rc, Derives E i c ∧ (∀ p r par, E[p]? = some (.borrowed c r par) → p < q) ∧
        E[u']? = some (.used c rc) ∧ q < u' ∧ x.location.Inside rc.location := by
  induction hd with
  | refl => exact fun u x hu hqu => ⟨i, u, x, .refl, hiq, hu, hqu, Location.Inside.refl _⟩
  | @step d t rd hb hDt ih =>
    intro u x hu hqu
    obtain ⟨par', hb'⟩ := hval d x (List.mem_of_getElem? hu)
    obtain ⟨hxr, -⟩ := borrowed_unique hnd hb' hb
    by_cases hall : ∀ p r par, E[p]? = some (.borrowed d r par) → p < q
    · exact ⟨d, u, x, .step hb hDt, hall, hu, hqu, Location.Inside.refl _⟩
    · have hex : ∃ p r par, E[p]? = some (.borrowed d r par) ∧ ¬ p < q :=
        Classical.byContradiction fun hne =>
          hall fun p r par hp => Classical.byContradiction fun hlt => hne ⟨p, r, par, hp, hlt⟩
      obtain ⟨p, r, par, hp, hpq⟩ := hex
      obtain ⟨hrr, hpar'⟩ := borrowed_unique hnd (List.mem_of_getElem? hp) hb
      rw [hrr, hpar'] at hp
      have hqp : q < p := by
        rcases Nat.lt_or_ge q p with h | h
        · exact h
        · have : p = q := by omega
          subst this
          exact absurd hp (hqb _ _ _)
      obtain ⟨u', rt, hu', hqu', hin⟩ := hpar p d rd t hp hqp
      rw [← hxr] at hin
      obtain ⟨c, u'', rc, hDc, hcq, hu'', hqu'', hin'⟩ := ih u' rt hu' hqu'
      exact ⟨c, u'', rc, hDc, hcq, hu'', hqu'', hin.trans hin'⟩

/-- **Each reference borrowed after a point has the one it is derived from used
after the point**, when the step of the point borrows nothing. -/
theorem parent_after {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s : State}
    {h : Shadow} {E F : List Event} (h0 : AdmissibleInitial P s₀) (hT : Trace P N ν s₀ s h E)
    (hB : ∀ s h E, Trace P N ν s₀ s h E → ∀ o post evs, TracedStep P N ν s h o post evs →
      BorrowsInside evs)
    (hBF : BorrowsInside F) {q : Nat}
    (hsep : ∀ s₁ h₁ E₁ s₂ h₂ E₂ G, Trace P N ν s₀ s₁ h₁ E₁ →
      TracedStep P N ν s₁ h₁ (.running s₂) (.running h₂) E₂ → Steps P N ν s₂ h₂ s h G →
      E = E₁ ++ E₂ ++ G → E₁.length ≤ q → q < E₁.length + E₂.length →
      ∀ d rd t, Event.borrowed d rd (some t) ∉ E₂)
    (hsepF : E.length ≤ q → ∀ d rd t, Event.borrowed d rd (some t) ∉ F) :
    ∀ p d rd t, (E ++ F)[p]? = some (.borrowed d rd (some t)) → q < p →
      ∃ u rt, (E ++ F)[u]? = some (.used t rt) ∧ q < u ∧ rd.location.Inside rt.location := by
  intro p d rd t hp hqp
  by_cases hpE : p < E.length
  · rw [List.getElem?_append_left hpE] at hp
    obtain ⟨s₁, h₁, E₁, s₂, h₂, E₂, G, hT₁, hst, hG, rfl, hle, hlt⟩ :=
      trace_split' hT hp (derived_not_initial h0)
    have hp₂ : E₂[p - E₁.length]? = some (.borrowed d rd (some t)) := by
      rw [List.append_assoc, List.getElem?_append_right hle,
        List.getElem?_append_left (by omega)] at hp
      exact hp
    have hq₁ : E₁.length > q := by
      refine Nat.lt_of_not_le fun hle' => ?_
      exact hsep _ _ _ _ _ _ _ hT₁ hst hG rfl hle' (by omega) d rd t (List.mem_of_getElem? hp₂)
    obtain ⟨rt, hu, hin, -⟩ := hB _ _ _ hT₁ _ _ _ hst d rd t (List.mem_of_getElem? hp₂)
    obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hu
    have hkl := (List.getElem?_eq_some_iff.1 hk).1
    refine ⟨E₁.length + k, rt, ?_, by omega, hin⟩
    rw [List.getElem?_append_left (by simp; omega), List.append_assoc,
      List.getElem?_append_right (by omega), Nat.add_sub_cancel_left,
      List.getElem?_append_left hkl, hk]
  · rw [List.getElem?_append_right (by omega)] at hp
    have hqE : q < E.length :=
      Nat.lt_of_not_le fun hle => hsepF hle d rd t (List.mem_of_getElem? hp)
    obtain ⟨rt, hu, hin, -⟩ := hBF d rd t (List.mem_of_getElem? hp)
    obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hu
    refine ⟨E.length + k, rt, ?_, by omega, hin⟩
    rw [List.getElem?_append_right (by omega), Nat.add_sub_cancel_left, hk]

/-! ## D4c and D4e on events -/

/-- A write overlapping a location overlaps every location it is inside. -/
theorem overlaps_inside {l m M : Location} (hin : m.Inside M)
    (h : Access.write.overlaps l m = true) : Access.write.overlaps l M = true := by
  simp only [Access.overlaps, Bool.and_eq_true, beq_iff_eq, Bool.or_eq_true,
    List.isPrefixOf_iff_prefix] at h ⊢
  obtain ⟨hr, hp⟩ := h
  refine ⟨hr.trans hin.1, ?_⟩
  rcases hp with hp | hp
  · rcases Nat.le_total l.path.length M.path.length with hle | hle
    · exact Or.inl (List.prefix_of_prefix_length_le hp hin.2 hle)
    · exact Or.inr (List.prefix_of_prefix_length_le hin.2 hp hle)
  · exact Or.inr (hin.2.trans hp)

/-- **D4c on events, by positions**: no write between the creation of a `&`
reference and a later use of it or of one derived from it overlaps the used
location. -/
theorem stable_of {E : List Event}
    (h : ∀ (p q u i : Nat) (r : Reference) (par : Option Nat) (l : Location) (via : Option Nat)
      (d : Nat) (rd : Reference), E[p]? = some (.borrowed i r par) → r.mutable = false →
      E[q]? = some (.accessed .write l via) → E[u]? = some (.used d rd) → p < q → q < u →
      RefSafety.derivesFrom E d i = true → Access.write.overlaps l rd.location = false) :
    RefSafety.stable E = true := by
  unfold RefSafety.stable
  rw [List.all_eq_true]
  rintro ⟨e, p⟩ hep
  have hp : E[p]? = some e := List.mem_zipIdx_iff_getElem?.1 hep
  cases e
  case borrowed i r par =>
    simp only [Event.borrowed?]
    cases hm : r.mutable
    · simp only [Bool.false_or, List.all_eq_true]
      rintro ⟨e', q⟩ heq
      obtain ⟨hpq, hq⟩ := mem_zipIdx_drop heq
      cases e'
      case accessed a l via =>
        cases a
        case write =>
          simp only [Event.accessed?, List.all_eq_true]
          intro m hm'
          unfold RefSafety.liveLocations at hm'
          obtain ⟨e'', he'', hf⟩ := List.mem_filterMap.1 hm'
          obtain ⟨u, hqu, hu⟩ := mem_drop_pos he''
          cases e''
          case used d rd =>
            simp only [Event.used?] at hf
            split at hf
            · next hder =>
              simp only [Option.some.injEq] at hf
              subst hf
              rw [h p q u i r par l via d rd hp hm hq hu hpq hqu hder]
              rfl
            · cases hf
          all_goals simp [Event.used?] at hf
        all_goals rfl
      all_goals simp [Event.accessed?]
    · rfl
  all_goals rfl

/-- **D4e on events, by positions**: no event ending a root comes between the
creation of a reference into it and a later use of it or of one derived from
it. -/
theorem dead_of {E : List Event}
    (h : ∀ (q p u : Nat) (e : Event) (ended : Root → Bool) (i : Nat) (r : Reference)
      (par : Option Nat) (d : Nat) (rd : Reference), E[q]? = some e → e.ends? = some ended →
      E[p]? = some (.borrowed i r par) → p < q → E[u]? = some (.used d rd) → q < u →
      RefSafety.derivesFrom E d i = true → ended r.location.root = false) :
    RefSafety.deadSlotsUnborrowed E = true := by
  unfold RefSafety.deadSlotsUnborrowed
  rw [List.all_eq_true]
  rintro ⟨e, q⟩ heq
  have hq : E[q]? = some e := List.mem_zipIdx_iff_getElem?.1 heq
  dsimp only
  cases hend : e.ends? with
  | none => rfl
  | some ended =>
    simp only [List.all_eq_true]
    intro e' he'
    obtain ⟨p, hp'⟩ := List.mem_iff_getElem?.1 he'
    have hpq : p < q := by
      have := (List.getElem?_eq_some_iff.1 hp').1
      simp at this
      omega
    have hp : E[p]? = some e' := by rw [← hp', List.getElem?_take_of_lt hpq]
    cases e'
    case borrowed i r par =>
      simp only [Event.borrowed?]
      cases hr : ended r.location.root
      · rfl
      · simp only [Bool.not_true, Bool.false_or, Bool.not_eq_true']
        unfold RefSafety.usedAfter
        rw [List.any_eq_false]
        intro e'' he''
        obtain ⟨u, hqu, hu⟩ := mem_drop_pos he''
        cases e''
        case used d rd =>
          simp only [Event.used?, Bool.not_eq_true]
          cases hder : RefSafety.derivesFrom E d i
          · rfl
          · have := h q p u e ended i r par d rd hq hend hp hpq hu hqu hder
            rw [hr] at this
            cases this
        all_goals simp [Event.used?]
    all_goals rfl

/-! ## D4c and D4e from each step -/

section Reductions

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
  {Held : State → Shadow → Nat → Reference → Prop}

/-- An event at a position of a trace's steps is an event of its step. -/
theorem mem_step {E E₁ E₂ G : List Event} (hE : E = E₁ ++ E₂ ++ G) {q : Nat} {e : Event}
    (hq : E[q]? = some e) (hle : E₁.length ≤ q) (hlt : q < E₁.length + E₂.length) :
    E₂[q - E₁.length]? = some e := by
  rw [hE, List.append_assoc, List.getElem?_append_right hle,
    List.getElem?_append_left (by omega)] at hq
  exact hq

/-- The core of `immutableStable_of_steps`: the trace `E` and the last step's
events `F`. -/
theorem stable_core (h0 : AdmissibleInitial P s₀) (hH : HeldFacts P N ν s₀ Held)
    (hB : ∀ s h E, Trace P N ν s₀ s h E → ∀ o post evs, TracedStep P N ν s h o post evs →
      BorrowsInside evs)
    (hVS : StepsStable P N ν s₀ Held) {s : State} {h : Shadow} {E F : List Event}
    (hT : Trace P N ν s₀ s h E) (hF : ∀ d r, Event.used d r ∈ F → Held s h d r)
    (hBF : BorrowsInside F) (hnd : (borrowedIds (E ++ F)).Nodup)
    (hFbw : ∀ d r par l via, Event.borrowed d r par ∈ F → Event.accessed .write l via ∈ F → False)
    (hVSF : ∀ l via, Event.accessed .write l via ∈ F → ∀ i r, Held s h i r →
      r.mutable = false → Access.write.overlaps l r.location = false) :
    RefSafety.stable (E ++ F) = true := by
  apply stable_of
  intro p q u i r par l via d rd hp hm hq hu hpq hqu hder
  have hval : ∀ d x, Event.used d x ∈ E ++ F → ∃ par, Event.borrowed d x par ∈ E ++ F := by
    intro d x hu'
    obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hu'
    obtain ⟨par, hb⟩ := used_value h0 hH hT hF hk
    exact ⟨par, List.mem_append_left _ hb⟩
  have hBI := borrows_inside_all h0 hT hB hBF
  have hiq : ∀ p' r' par', (E ++ F)[p']? = some (.borrowed i r' par') → p' < q :=
    fun p' r' par' hp' => by rw [borrowed_pos_unique hnd hp' hp]; exact hpq
  have hqb : ∀ d r par, (E ++ F)[q]? ≠ some (.borrowed d r par) := by
    intro d r par h'
    rw [hq] at h'
    cases h'
  have hpar := parent_after h0 hT hB hBF (q := q)
    (fun _ _ E₁ _ _ E₂ G _ hst _ hE hle hlt d' rd' t hb => by
      have hqE : q < E.length := by rw [hE]; simp; omega
      rw [List.getElem?_append_left hqE] at hq
      exact traced_borrow_write hst hb (List.mem_of_getElem? (mem_step hE hq hle hlt)))
    (fun hle d' rd' t hb => hFbw d' rd' (some t) l via hb (by
      rw [List.getElem?_append_right hle] at hq
      exact List.mem_of_getElem? hq))
  obtain ⟨c, u', rc, hDc, hcq, hu', hqu', hin⟩ :=
    live_chain hnd hpar hval hqb hiq (derives_of_derivesFrom hder) u rd hu hqu
  obtain ⟨pc, hbc⟩ := hval c rc (List.mem_of_getElem? hu')
  have hrc : rc.mutable = false := (derives_inside hnd hBI hval (List.mem_of_getElem? hp) hDc hbc).2 hm
  cases hov : Access.write.overlaps l rd.location with
  | false => rfl
  | true =>
    exfalso
    have hov' := overlaps_inside hin hov
    by_cases hqE : q < E.length
    · rw [List.getElem?_append_left hqE] at hq
      have hcq' : ∀ p r par, E[p]? = some (.borrowed c r par) → p < q := fun p r par hp' =>
        hcq p r par (by
          rw [List.getElem?_append_left (List.getElem?_eq_some_iff.1 hp').1]; exact hp')
      obtain ⟨s₁, h₁, E₁, s₂, h₂, E₂, G, hT₁, hst, -, hE, hle, hlt, hheld, -⟩ :=
        held_across hH hT hF hq (initial_not_accessed h0) hcq' hu' hqu'
      have := hVS s₁ h₁ E₁ hT₁ _ _ E₂ hst l via
        (List.mem_of_getElem? (mem_step hE hq hle hlt)) c rc hheld hrc
      rw [hov'] at this
      cases this
    · have huF : Event.used c rc ∈ F := by
        rw [List.getElem?_append_right (by omega)] at hu'
        exact List.mem_of_getElem? hu'
      have hqF : Event.accessed .write l via ∈ F := by
        rw [List.getElem?_append_right (by omega)] at hq
        exact List.mem_of_getElem? hq
      have := hVSF l via hqF c rc (hF c rc huF) hrc
      rw [hov'] at this
      cases this

/-- **D4c from what each step guarantees**, from an admissible initial state:
when no write of a step overlaps an immutable reference held before it, for a
predicate of the held references with `HeldFacts`, and derived references
are inside the ones they derive from, no write overlaps a live `&`
reference. -/
theorem immutableStable_of_steps (h0 : AdmissibleInitial P s₀) (hH : HeldFacts P N ν s₀ Held)
    (hB : ∀ s h E, Trace P N ν s₀ s h E → ∀ o post evs, TracedStep P N ν s h o post evs →
      BorrowsInside evs)
    (hVS : StepsStable P N ν s₀ Held) : ImmutableStable P N ν s₀ := by
  intro s h E hT
  have hI := refTraceInv_trace h0 hT
  refine ⟨?_, fun o post evs' hS => ?_⟩
  · have := stable_core h0 hH hB hVS hT (F := []) (fun _ _ h => by cases h)
      (fun _ _ _ h => by cases h) (by simpa using hI.unique) (fun _ _ _ _ _ h => by cases h)
      (fun _ _ h => by cases h)
    simpa using this
  · exact stable_core h0 hH hB hVS hT (fun d r hu => hH.used _ _ _ hT _ _ _ hS d r hu)
      (hB _ _ _ hT _ _ _ hS) (traced_nodup h0 hT hS)
      (fun _ _ _ _ _ hb hw => traced_borrow_write hS hb hw)
      (fun l via hw i r hi hm => hVS s h E hT o post evs' hS l via hw i r (Or.inl hi) hm)

/-- The initial events end nothing. -/
theorem initial_not_ends (h0 : AdmissibleInitial P s₀) {e : Event}
    (he : e ∈ (Shadow.initial s₀).2) : e.ends? = none := by
  rcases initial_events h0 he with ⟨_, _, _, rfl⟩ | ⟨_, _, rfl⟩ | ⟨_, _, rfl⟩ <;> rfl

/-- The core of `deadSlots_of_steps`: the trace `E` and the last step's events
`F`. -/
theorem dead_core (h0 : AdmissibleInitial P s₀) (hH : HeldFacts P N ν s₀ Held)
    (hB : ∀ s h E, Trace P N ν s₀ s h E → ∀ o post evs, TracedStep P N ν s h o post evs →
      BorrowsInside evs)
    (hDead : StepsDead P N ν s₀ Held) {s : State} {h : Shadow} {E F : List Event}
    (hT : Trace P N ν s₀ s h E) (hF : ∀ d r, Event.used d r ∈ F → Held s h d r)
    (hBF : BorrowsInside F) (hnd : (borrowedIds (E ++ F)).Nodup)
    (hFends : ∀ e ∈ F, ∀ ended, e.ends? = some ended →
      ∃ A, F = A ++ [e] ∧ ∀ e' ∈ A, e'.ends? = none) :
    RefSafety.deadSlotsUnborrowed (E ++ F) = true := by
  apply dead_of
  intro q p u e ended i r par d rd hq hend hp hpq hu hqu hder
  have hulen := (List.getElem?_eq_some_iff.1 hu).1
  by_cases hqE : q < E.length
  · rw [List.getElem?_append_left hqE] at hq
    have he : e ∉ (Shadow.initial s₀).2 := fun he => by
      rw [initial_not_ends h0 he] at hend
      cases hend
    have hval : ∀ d x, Event.used d x ∈ E ++ F → ∃ par, Event.borrowed d x par ∈ E ++ F := by
      intro d x hu'
      obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hu'
      obtain ⟨par, hb⟩ := used_value h0 hH hT hF hk
      exact ⟨par, List.mem_append_left _ hb⟩
    have hBI := borrows_inside_all h0 hT hB hBF
    have hiq : ∀ p' r' par', (E ++ F)[p']? = some (.borrowed i r' par') → p' < q :=
      fun p' r' par' hp' => by rw [borrowed_pos_unique hnd hp' hp]; exact hpq
    have hqb : ∀ d r par, (E ++ F)[q]? ≠ some (.borrowed d r par) := by
      intro d r par h'
      rw [List.getElem?_append_left hqE, hq] at h'
      cases h'
      cases hend
    have hpar := parent_after h0 hT hB hBF (q := q)
      (fun _ _ E₁ _ _ E₂ G hT₁ hst _ hE hle hlt d' rd' t hb =>
        (traced_ends (refTraceInv_trace h0 hT₁).pre hst
          (List.mem_of_getElem? (mem_step hE hq hle hlt)) hend).2 d' rd' (some t) hb)
      (fun hle => absurd hle (by omega))
    obtain ⟨c, u', rc, hDc, hcq, hu', hqu', -⟩ :=
      live_chain hnd hpar hval hqb hiq (derives_of_derivesFrom hder) u rd hu hqu
    obtain ⟨pc, hbc⟩ := hval c rc (List.mem_of_getElem? hu')
    have hroot : rc.location.root = r.location.root :=
      (derives_inside hnd hBI hval (List.mem_of_getElem? hp) hDc hbc).1.1
    have hcq' : ∀ p r par, E[p]? = some (.borrowed c r par) → p < q := fun p r par hp' =>
      hcq p r par (by
        rw [List.getElem?_append_left (List.getElem?_eq_some_iff.1 hp').1]; exact hp')
    obtain ⟨s₁, h₁, E₁, s₂, h₂, E₂, G, hT₁, hst, -, hE, hle, hlt, -, hpost⟩ :=
      held_across hH hT hF hq he hcq' hu' hqu'
    have he₂ := mem_step hE hq hle hlt
    obtain ⟨⟨A, hA, hA'⟩, -⟩ := traced_ends (refTraceInv_trace h0 hT₁).pre hst
      (List.mem_of_getElem? he₂) hend
    have hlast := ends_last hA' (hA ▸ he₂) hend
    have hlen : E₂.length = A.length + 1 := by rw [hA]; simp
    have := hDead s₁ h₁ E₁ hT₁ s₂ h₂ E₂ hst e (List.mem_of_getElem? he₂) ended hend c rc
      (hpost (by omega))
    rw [← hroot]
    exact this
  · exfalso
    rw [List.getElem?_append_right (by omega)] at hq
    obtain ⟨A, hA, hA'⟩ := hFends e (List.mem_of_getElem? hq) ended hend
    have hlast := ends_last hA' (hA ▸ hq) hend
    have hlen : F.length = A.length + 1 := by rw [hA]; simp
    simp only [List.length_append] at hulen
    omega

/-- **D4e from what each step guarantees**, from an admissible initial state:
when, after a step that ends a slot, no reference held points into it, for a
predicate of the held references with `HeldFacts`, and derived references
are inside the ones they derive from, no slot ends while a live reference
points into it. -/
theorem deadSlots_of_steps (h0 : AdmissibleInitial P s₀) (hH : HeldFacts P N ν s₀ Held)
    (hB : ∀ s h E, Trace P N ν s₀ s h E → ∀ o post evs, TracedStep P N ν s h o post evs →
      BorrowsInside evs)
    (hDead : StepsDead P N ν s₀ Held) : DeadSlotsUnborrowed P N ν s₀ := by
  intro s h E hT
  have hI := refTraceInv_trace h0 hT
  refine ⟨?_, fun o post evs' hS => ?_⟩
  · have := dead_core h0 hH hB hDead hT (F := []) (fun _ _ h => by cases h)
      (fun _ _ _ h => by cases h) (by simpa using hI.unique) (fun _ h => by cases h)
    simpa using this
  · exact dead_core h0 hH hB hDead hT (fun d r hu => hH.used _ _ _ hT _ _ _ hS d r hu)
      (hB _ _ _ hT _ _ _ hS) (traced_nodup h0 hT hS)
      (fun e he ended hend => (traced_ends hI.pre hS he hend).1)

end Reductions

end MoveBytecode.Proofs
