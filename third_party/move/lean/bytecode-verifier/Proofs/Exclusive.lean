-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ExclusiveDefs
import Proofs.Derivations
import Proofs.FrameSteps
import Proofs.RefAccess

/-!
# D4b″ from each step

UNTRUSTED (checked by the kernel). D4b″ reduces to what each step guarantees
(`parametersExclusive_of_steps`): for a `&mut` reference parameter `r` of a
frame entered and not yet returned, an access not through a reference derived
from `r` overlaps no reference derived from `r` held before the access's step
(`StepsExclusive`). A reference derived from `r` and used after the access is
inside one derived from `r`, borrowed before the access and used after it,
which is held at the access's step (`live_chain'`, `held_across`). None of the
references derived from `r` is borrowed by the access's own step: a step that
accesses and borrows borrows from the reference it accesses through, which
does not derive from `r`.
-/

namespace MoveBytecode.Proofs

/-! ## Steps that access -/

/-- **A step that accesses and borrows** borrows from the reference it
accesses through. -/
theorem exec_access_borrow {P : Program} {m : ModuleId} {instr : Bytecode} {s : State}
    {fr : Frame} {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event}
    {g : ShadowFrame} {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) {a : Access} {l : Location}
    {via : Option Nat} (ha : Event.accessed a l via ∈ evs) {d t : Nat} {rd : Reference}
    (hb : Event.borrowed d rd (some t) ∈ evs) : via = some t := by
  obtain ⟨-, hnew⟩ := exec_borrowed hh hex hb
  rcases hnew with ⟨rfl, -⟩ | ⟨j, rfl | rfl⟩ | ⟨j, rfl | rfl | rfl | rfl⟩ |
    ⟨j, rfl | rfl | rfl | rfl⟩ | ⟨j, rfl | rfl⟩ | ⟨j, rfl | rfl | rfl | rfl⟩ <;>
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      guard] at hex
  all_goals first
    | (obtain ⟨x, -, _, -, hm⟩ := hex
       split at hm
       · simp only [Option.some.injEq, Prod.mk.injEq] at hm
         obtain ⟨-, rfl⟩ := hm
         simp at ha hb
         first
           | (obtain ⟨-, -, rfl⟩ := ha; obtain ⟨-, -, rfl⟩ := hb; rfl)
           | skip
       · cases hm)
    | (obtain ⟨_, -, hm⟩ := hex
       split at hm
       · simp only [Option.some.injEq, Prod.mk.injEq] at hm
         obtain ⟨-, rfl⟩ := hm
         simp at hb
       · cases hm)
    | (obtain ⟨x, -, hm⟩ := hex
       split at hm
       · obtain ⟨_, -, hm⟩ := Option.bind_eq_some_iff.1 hm
         split at hm
         · simp only [Option.some.injEq, Prod.mk.injEq] at hm
           obtain ⟨-, rfl⟩ := hm
           simp at ha hb
           first
             | (obtain ⟨-, -, rfl⟩ := ha; obtain ⟨-, -, rfl⟩ := hb; rfl)
             | skip
         · cases hm
       · cases hm)
    | (split at hex
       · split at hex
         · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
           obtain ⟨-, -, -, rfl⟩ := hex
           simp at hb
         · cases hex
       · cases hex)

/-- A `Call` or `Ret` accesses nothing. -/
theorem exec_access_not_call {P : Program} {m : ModuleId} {instr : Bytecode} {s : State}
    {fr : Frame} {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event}
    {g : ShadowFrame} {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) {a : Access} {l : Location}
    {via : Option Nat} (ha : Event.accessed a l via ∈ evs) :
    (∀ hd, instr ≠ .call hd) ∧ (∀ i, instr ≠ .callGeneric i) ∧ instr ≠ .ret ∧
      ∀ sig, instr ≠ .callClosure sig := by
  refine ⟨fun hd hi => ?_, fun hd hi => ?_, fun hi => ?_, fun sig hi => ?_⟩
  all_goals try
    subst hi
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.pure_def, guard] at hex
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, -, -, rfl⟩ := hex
        simp at ha
      · cases hex
    · cases hex
  · subst hi
    simp only [Shadow.exec, hh] at hex
    split at hex <;>
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp at ha
  · subst hi
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.pure_def, guard] at hex
    split at hex
    all_goals (try split at hex)
    all_goals (try split at hex)
    all_goals (try split at hex)
    all_goals first
      | cases hex
      | (obtain ⟨_, -, hex⟩ := Option.bind_eq_some_iff.1 hex
         simp only [Option.some.injEq, Prod.mk.injEq] at hex
         obtain ⟨-, rfl⟩ := hex
         simp at ha)
      | simp at hex

section Traced

variable {P : Program} {N : NativeRel} {ν : NativeFlows}

/-- **A traced step that accesses and borrows** borrows from the reference it
accesses through. -/
theorem traced_access_borrow {s : State} {h : Shadow} {o : Outcome} {post : ShadowPost}
    {evs : List Event} (hS : TracedStep P N ν s h o post evs) {a : Access} {l : Location}
    {via : Option Nat} (ha : Event.accessed a l via ∈ evs) {d t : Nat} {rd : Reference}
    (hb : Event.borrowed d rd (some t) ∈ evs) : via = some t := by
  cases hS with
  | core _ hc =>
    obtain ⟨fr, rest, info, code, instr, -, -, -, -, hex⟩ := coreStep_exec hc
    obtain ⟨hframes, hst, hhe, fresh⟩ := h
    cases hframes with
    | nil => simp [Shadow.exec] at hex
    | cons g hs => exact exec_access_borrow rfl hex ha hb
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      obtain ⟨tys, -, hfits⟩ := nativeStep_fits hstep hn
      obtain ⟨-, -, -, -, p, hp, -⟩ := nativeStep_borrowed hn hb
      have hkb := places_borrow hfits (List.ne_nil_of_mem hp)
      obtain ⟨-, -, -, -, r0, t0, m, tgt, p, d', -, -, -, -, -, -, -, -, -, hall, -⟩ :=
        nativeStep_borrowEvents hstep hn hkb
      rcases hall _ ha with he | he | he | ⟨_, _, _, he⟩ | ⟨_, _, _, he⟩ <;> cases he
      rcases hall _ hb with he | he | he | ⟨_, _, _, he⟩ | ⟨_, _, _, he⟩ <;> cases he
      rfl
    obtain ⟨rfl, rfl, rfl, g, hs, info, hh, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at hb
    have := mem_borrowedIds.2 ⟨_, _, hb⟩
    rw [nativeEvents_borrowedIds] at this
    cases this

/-- **A traced step that accesses** enters and ends no frame. -/
theorem traced_access_quiet {s : State} {h : Shadow} {o : Outcome} {post : ShadowPost}
    {evs : List Event} (hS : TracedStep P N ν s h o post evs) {a : Access} {l : Location}
    {via : Option Nat} (ha : Event.accessed a l via ∈ evs) :
    ∀ e ∈ evs, e.entered? = none ∧ e.returned? = none := by
  intro e he
  cases hS with
  | core _ hc =>
    obtain ⟨fr, rest, info, code, instr, -, -, -, -, hex⟩ := coreStep_exec hc
    obtain ⟨hframes, hst, hhe, fresh⟩ := h
    cases hframes with
    | nil => simp [Shadow.exec] at hex
    | cons g hs =>
      obtain ⟨hcall, hcallg, hret, hcallc⟩ := exec_access_not_call rfl hex ha
      refine ⟨?_, ?_⟩
      · cases hen : e.entered? with
        | none => rfl
        | some x => exact (exec_entered rfl hex hcall hcallg hcallc he hen).elim
      · cases hr : e.returned? with
        | none => rfl
        | some x => exact (exec_returned rfl hex hret he hr).elim
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      exact ⟨(nativeStep_quiet hn he).1, (nativeStep_quiet hn he).2.1⟩
    obtain ⟨rfl, rfl, rfl, g, hs, info, hh, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at he
    exact nativeEvents_quiet he

/-- In a traced step, a reference is borrowed after the one it derives from. -/
theorem traced_parent_lt {s : State} {h : Shadow} {E : List Event} {o : Outcome}
    {post : ShadowPost} {evs : List Event} (hI : RefTraceInv P s h E)
    (hS : TracedStep P N ν s h o post evs) :
    ∀ d rd t, Event.borrowed d rd (some t) ∈ evs → t < d := by
  intro d rd t hb
  obtain ⟨s', rfl⟩ : ∃ s', o = .running s' := by
    cases hS with
    | core _ hc =>
      obtain ⟨fr, rest, info, code, instr, -, -, -, -, hex⟩ := coreStep_exec hc
      obtain ⟨hframes, hst, hhe, fresh⟩ := h
      cases hframes with
      | nil => simp [Shadow.exec] at hex
      | cons g hs =>
        obtain ⟨htop, -⟩ := exec_borrowed rfl hex hb
        cases o with
        | running s' => exact ⟨s', rfl⟩
        | _ => simp [Shadow.topAfter] at htop
    | native hstep _ hn =>
      rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
      case inr =>
        obtain ⟨-, -, -, -, -, -, s', -, -, -, -, -, -, -, -, -, -, -, -, -, -, ho, -⟩ :=
          nativeStep_spec hn
        exact ⟨s', ho⟩
      obtain ⟨rfl, rfl, rfl, g, hs, info, hh, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
      rw [hevs] at hb
      have := mem_borrowedIds.2 ⟨_, _, hb⟩
      rw [nativeEvents_borrowedIds] at this
      cases this
  have hd := traced_borrowed_fresh hI hS rfl hb
  obtain ⟨rt, hu, -, -⟩ := traced_borrowsInside hS d rd t hb
  obtain ⟨par, hbt⟩ := hI.refs t rt (traced_used hI.pre hS hu)
  have := hI.below t (mem_borrowedIds.2 ⟨_, _, hbt⟩)
  omega

end Traced

/-! ## Traces -/

section Traces

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}

/-- Every trace starts with the initial events. -/
theorem trace_initial_prefix {s : State} {h : Shadow} {E : List Event}
    (hT : Trace P N ν s₀ s h E) : (Shadow.initial s₀).2 <+: E := by
  induction hT with
  | start => exact List.prefix_refl _
  | step _ _ ih => exact ih.trans (List.prefix_append _ _)

/-- **The step of a position** of a trace after its initial events. -/
theorem trace_split_pos {s : State} {h : Shadow} {E : List Event}
    (hT : Trace P N ν s₀ s h E) {q : Nat} (hq0 : (Shadow.initial s₀).2.length ≤ q)
    (hqE : q < E.length) :
    ∃ s₁ h₁ E₁ s₂ h₂ E₂ G, Trace P N ν s₀ s₁ h₁ E₁ ∧
      TracedStep P N ν s₁ h₁ (.running s₂) (.running h₂) E₂ ∧ Steps P N ν s₂ h₂ s h G ∧
      E = E₁ ++ E₂ ++ G ∧ E₁.length ≤ q ∧ q < E₁.length + E₂.length := by
  induction hT generalizing q with
  | start => omega
  | @step s₁ s₂ h₁ h₂ E₁ E₂ hT₀ hS ih =>
    by_cases hlt : q < E₁.length
    · obtain ⟨t₁, k₁, F₁, t₂, k₂, F₂, G, hT₁, hS₁, hG, hE, hle, hlt'⟩ := ih hq0 hlt
      exact ⟨t₁, k₁, F₁, t₂, k₂, F₂, G ++ E₂, hT₁, hS₁, .step hG hS, by simp [hE], hle, hlt'⟩
    · rw [List.length_append] at hqE
      exact ⟨s₁, h₁, E₁, s₂, h₂, E₂, [], hT₀, hS, .refl, by simp, by omega, by omega⟩

/-- **A frame entered and not yet returned is in the frames**, with the
parameters it was entered with. -/
theorem trace_alive (h0 : AdmissibleInitial P s₀) {s : State} {h : Shadow} {E : List Event}
    (hT : Trace P N ν s₀ s h E) {p : Nat} {f : FrameId} {ps : List Nat}
    (hp : E[p]? = some (.entered f ps))
    (hne : ∀ k, p < k → ∀ rets, E[k]? ≠ some (.frameEnded f rets)) :
    (∃ fr ∈ s.frames, fr.id = f) ∧ RefSafety.params? E f = some ps := by
  induction hT generalizing p with
  | start =>
    obtain ⟨fr₀, g₀, pids₀, hsf, -, hpar, -, -, hent, -⟩ := initial_entry h0
    obtain ⟨rfl, rfl⟩ := hent f ps (List.mem_of_getElem? hp)
    exact ⟨⟨fr₀, by rw [hsf]; exact List.mem_singleton_self _, rfl⟩, hpar⟩
  | @step s₁ s₂ h₁ h₂ E₁ E₂ hT₀ hS ih =>
    have hI := refTraceInv_trace h0 hT₀
    by_cases hpE : p < E₁.length
    · rw [List.getElem?_append_left hpE] at hp
      obtain ⟨⟨fr, hfr, rfl⟩, hpar⟩ := ih hp fun k hk rets he => hne k hk rets (by
        rw [List.getElem?_append_left (List.getElem?_eq_some_iff.1 he).1]; exact he)
      refine ⟨?_, params?_append hpar⟩
      rcases traced_shape hI.pre hI.typed.args hI.typed.valsShaped hS rfl rfl with ⟨hkeys, -, -⟩ |
        ⟨fr', rest, g, hs', callee, gc, k, pids, hsf, -, hs'f, -⟩ |
        ⟨fr', caller, rest, g, gcaller, hs', caller', gc', rets, hsf, -, hs'f, -, hkey, hR, -⟩
      · have : fr.key ∈ s₂.frames.map Frame.key := by
          rw [hkeys]; exact List.mem_map_of_mem hfr
        obtain ⟨fr₂, hfr₂, hk⟩ := List.mem_map.1 this
        exact ⟨fr₂, hfr₂, by
          simp only [Frame.key, Prod.mk.injEq] at hk; exact hk.1⟩
      · rw [hsf, List.mem_cons] at hfr
        rw [hs'f]
        rcases hfr with rfl | hfr
        · exact ⟨_, List.mem_cons_of_mem _ (List.mem_cons_self ..), rfl⟩
        · exact ⟨fr, List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hfr), rfl⟩
      · rw [hsf] at hfr
        rw [hs'f]
        rcases List.mem_cons.1 hfr with rfl | hfr
        · exfalso
          obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hR.ended
          exact hne (E₁.length + k) (by omega) rets (by
            rw [List.getElem?_append_right (by omega), Nat.add_sub_cancel_left, hk])
        · rcases List.mem_cons.1 hfr with rfl | hfr
          · exact ⟨caller', List.mem_cons_self .., by
              have := hkey; simp only [Frame.key, Prod.mk.injEq] at this; exact this.1⟩
          · exact ⟨fr, List.mem_cons_of_mem _ hfr, rfl⟩
    · rw [List.getElem?_append_right (by omega)] at hp
      have he := List.mem_of_getElem? hp
      rcases traced_shape hI.pre hI.typed.args hI.typed.valsShaped hS rfl rfl with ⟨-, -, hev⟩ |
        ⟨fr', rest, g, hs', callee, gc, k, pids, hsf, -, hs'f, -, hcid, hent, hcev, -⟩ |
        ⟨fr', caller, rest, g, gcaller, hs', caller', gc', rets, hsf, -, hs'f, -, hkey, hR, -⟩
      · exact absurd (hev _ he).1 (by simp [Event.entered?])
      · obtain ⟨rfl, rfl⟩ := (hcev _ he).2 f ps rfl
        refine ⟨⟨callee, by rw [hs'f]; exact List.mem_cons_self .., rfl⟩, ?_⟩
        rw [params?_append_none (params?_none_of fun f' ps' he' => by
          rw [hcid]; exact trace_entered_lt h0 hT₀ f' ps' he')]
        exact params?_of_mem (fun e he' f' ps' hef hf' => by
          subst hef
          exact ((hcev _ he').2 f' ps' rfl).2) hent
      · exact absurd (hR.only _ he).1 (by simp [Event.entered?])

/-- **A frame's parameters were borrowed before the step that entered it**:
with the initial events, or before the entering step. -/
theorem param_borrowed_before (h0 : AdmissibleInitial P s₀) {s : State} {h : Shadow}
    {E : List Event} (hT : Trace P N ν s₀ s h E) {pe : Nat} {f : FrameId} {ps : List Nat}
    (hpe : E[pe]? = some (.entered f ps)) {r : Nat} (hr : r ∈ ps) :
    ∃ k rr par, E[k]? = some (.borrowed r rr par) ∧
      (k < (Shadow.initial s₀).2.length ∨ k < pe) := by
  have hpre := trace_initial_prefix hT
  by_cases hp0 : pe < (Shadow.initial s₀).2.length
  · obtain ⟨fr₀, g₀, pids₀, -, -, -, -, hb₀, hent, -⟩ := initial_entry h0
    have he : Event.entered f ps ∈ (Shadow.initial s₀).2 := by
      obtain ⟨G, hG⟩ := hpre
      rw [← hG, List.getElem?_append_left hp0] at hpe
      exact List.mem_of_getElem? hpe
    obtain ⟨-, rfl⟩ := hent f ps he
    obtain ⟨rr, par, hb⟩ := mem_borrowedIds.1 (hb₀ r hr)
    obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hb
    have hkl := (List.getElem?_eq_some_iff.1 hk).1
    obtain ⟨G, hG⟩ := hpre
    exact ⟨k, rr, par, by rw [← hG, List.getElem?_append_left hkl, hk], Or.inl hkl⟩
  · obtain ⟨s₁, h₁, E₁, s₂, h₂, E₂, G, hT₁, hst, -, rfl, hle, hlt⟩ :=
      trace_split_pos hT (by omega) (List.getElem?_eq_some_iff.1 hpe).1
    have hI₁ := refTraceInv_trace h0 hT₁
    have he₂ := List.mem_of_getElem? (mem_step rfl hpe hle hlt)
    rcases traced_shape hI₁.pre hI₁.typed.args hI₁.typed.valsShaped hst rfl rfl with ⟨-, -, hev⟩ |
      ⟨fr, rest, g, hs', callee, gc, k, pids, hsf, hhf, -, -, -, -, hcev, -, hpids, -⟩ |
      ⟨_, _, _, _, _, _, _, _, _, -, -, -, -, -, hR, -⟩
    · exact absurd (hev _ he₂).1 (by simp [Event.entered?])
    · obtain ⟨-, rfl⟩ := (hcev _ he₂).2 f ps rfl
      obtain ⟨rr, hm⟩ := hpids r hr
      obtain ⟨par, hb⟩ := hI₁.refs r rr (by
        rw [hhf, hsf, framesOcc_cons]; exact List.mem_append_left _ hm)
      obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hb
      have hkl := (List.getElem?_eq_some_iff.1 hk).1
      refine ⟨k, rr, par, ?_, Or.inr (by omega)⟩
      rw [List.append_assoc, List.getElem?_append_left hkl, hk]
    · exact absurd (hR.only _ he₂).1 (by simp [Event.entered?])

end Traces

/-! ## D4b″ on events -/

/-- A member of a list's longest prefix of elements satisfying `pred`: its
position, and every element up to it satisfies `pred`. -/
theorem mem_takeWhile_idx {α : Type} {pred : α → Bool} :
    ∀ {l : List α} {x : α}, x ∈ l.takeWhile pred →
      ∃ i : Nat, l[i]? = some x ∧ ∀ (j : Nat) (y : α), j ≤ i → l[j]? = some y → pred y = true
  | [], x, h => by simp at h
  | a :: l, x, h => by
    cases ha : pred a with
    | false => simp [List.takeWhile, ha] at h
    | true =>
      simp only [List.takeWhile, ha] at h
      rcases List.mem_cons.1 h with rfl | h
      · refine ⟨0, rfl, fun j y hj hy => ?_⟩
        obtain rfl : j = 0 := by omega
        simp only [List.getElem?_cons_zero, Option.some.injEq] at hy
        subst hy
        exact ha
      · obtain ⟨i, hi, hall⟩ := mem_takeWhile_idx h
        refine ⟨i + 1, by simpa using hi, fun j y hj hy => ?_⟩
        cases j with
        | zero =>
          simp only [List.getElem?_cons_zero, Option.some.injEq] at hy
          subst hy
          exact ha
        | succ j => exact hall j y (by omega) (by simpa using hy)

/-- An event while frame `f`, entered at position `p`, is active: after the
entry, and with no return of `f` up to it. -/
theorem mem_whileActive {E : List Event} {f : FrameId} {p q : Nat} {e : Event}
    (h : (e, q) ∈ RefSafety.whileActive E f p) :
    p < q ∧ E[q]? = some e ∧ ∀ k, p < k → k ≤ q → ∀ rets, E[k]? ≠ some (.frameEnded f rets) := by
  unfold RefSafety.whileActive at h
  obtain ⟨i, hi, hall⟩ := mem_takeWhile_idx h
  rw [List.getElem?_drop, List.getElem?_zipIdx] at hi
  cases he : E[p + 1 + i]? with
  | none => rw [he] at hi; cases hi
  | some e' =>
    rw [he] at hi
    simp only [Option.map_some, Option.some.injEq, Prod.mk.injEq] at hi
    obtain ⟨rfl, hq⟩ := hi
    refine ⟨by omega, by rw [← he]; congr 1; omega, fun k hpk hkq rets hk => ?_⟩
    have := hall (k - (p + 1)) (.frameEnded f rets, k) (by omega) (by
      rw [List.getElem?_drop, List.getElem?_zipIdx, show p + 1 + (k - (p + 1)) = k by omega, hk]
      simp)
    simp [Event.returned?] at this

/-- **D4b″ on events, by positions**: while a frame entered at `p` is active,
no access at `q` that is not through a reference derived from a `&mut`
parameter `r` overlaps the location of a later use of a reference derived
from `r`. -/
theorem exclusive_of {E : List Event}
    (h : ∀ (p q u : Nat) (f : FrameId) (ps : List Nat) (r : Nat) (a : Access) (l : Location)
      (via : Option Nat) (d : Nat) (x : Reference),
      E[p]? = some (.entered f ps) → r ∈ ps → RefSafety.isMutable E r = true →
      E[q]? = some (.accessed a l via) → p < q →
      (∀ k, p < k → k ≤ q → ∀ rets, E[k]? ≠ some (.frameEnded f rets)) →
      (∀ v, via = some v → RefSafety.derivesFrom E v r = false) →
      E[u]? = some (.used d x) → q < u → RefSafety.derivesFrom E d r = true →
      a.overlaps l x.location = false) :
    RefSafety.paramsExclusive E = true := by
  unfold RefSafety.paramsExclusive
  rw [List.all_eq_true]
  rintro ⟨e, p⟩ hep
  have hp : E[p]? = some e := List.mem_zipIdx_iff_getElem?.1 hep
  cases e
  case entered f ps =>
    simp only [Event.entered?]
    rw [List.all_eq_true]
    intro r hr
    cases hm : RefSafety.isMutable E r
    · rfl
    · simp only [Bool.not_true, Bool.false_or, List.all_eq_true]
      rintro ⟨e', q⟩ heq
      obtain ⟨hpq, hq, hne⟩ := mem_whileActive heq
      cases e'
      case accessed a l via =>
        simp only [Event.accessed?]
        cases hv : via.any (fun v => RefSafety.derivesFrom E v r)
        · simp only [Bool.false_or, List.all_eq_true]
          intro m hm'
          unfold RefSafety.liveLocations at hm'
          obtain ⟨e'', he'', hf⟩ := List.mem_filterMap.1 hm'
          obtain ⟨u, hqu, hu⟩ := mem_drop_pos he''
          have hvia : ∀ v, via = some v → RefSafety.derivesFrom E v r = false := by
            intro v hv'
            subst hv'
            simpa using hv
          cases e''
          case used d x =>
            simp only [Event.used?] at hf
            split at hf
            · next hder =>
              simp only [Option.some.injEq] at hf
              subst hf
              rw [h p q u f ps r a l via d x hp hr hm hq hpq hne hvia hu hqu hder]
              rfl
            · cases hf
          all_goals simp [Event.used?] at hf
        · rfl
      all_goals rfl
  all_goals rfl

/-- An access overlapping a location overlaps every location it is inside. -/
theorem overlaps_inside' {a : Access} {l m M : Location} (hin : m.Inside M)
    (h : a.overlaps l m = true) : a.overlaps l M = true := by
  cases a with
  | inspect =>
    simp only [Access.overlaps, Bool.and_eq_true, beq_iff_eq, List.isPrefixOf_iff_prefix] at h ⊢
    exact ⟨h.1.trans hin.1, hin.2.trans h.2⟩
  | read =>
    simp only [Access.overlaps, Bool.and_eq_true, beq_iff_eq, Bool.or_eq_true,
      List.isPrefixOf_iff_prefix] at h ⊢
    obtain ⟨hr, hp⟩ := h
    refine ⟨hr.trans hin.1, ?_⟩
    rcases hp with hp | hp
    · rcases Nat.le_total l.path.length M.path.length with hle | hle
      · exact Or.inl (List.prefix_of_prefix_length_le hp hin.2 hle)
      · exact Or.inr (List.prefix_of_prefix_length_le hin.2 hp hle)
    · exact Or.inr (hin.2.trans hp)
  | write => exact overlaps_inside hin h

/-! ## Derivations -/

/-- **Along a derivation from `i`, some reference borrowed before a point is
used after it**, when each reference derived from `i` and borrowed after the
point has the one it is derived from used after it. -/
theorem live_chain' {E : List Event} (hnd : (borrowedIds E).Nodup) {q i : Nat}
    (hpar : ∀ p d rd t, E[p]? = some (.borrowed d rd (some t)) → q < p → Derives E i d →
      ∃ u rt, E[u]? = some (.used t rt) ∧ q < u ∧ rd.location.Inside rt.location)
    (hval : ∀ d x, Event.used d x ∈ E → ∃ par, Event.borrowed d x par ∈ E)
    (hqb : ∀ d r par, E[q]? ≠ some (.borrowed d r par))
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
      obtain ⟨u', rt, hu', hqu', hin⟩ := hpar p d rd t hp hqp (.step hb hDt)
      rw [← hxr] at hin
      obtain ⟨c, u'', rc, hDc, hcq, hu'', hqu'', hin'⟩ := ih u' rt hu' hqu'
      exact ⟨c, u'', rc, hDc, hcq, hu'', hqu'', hin.trans hin'⟩

/-- **A derivation of a reference borrowed in a prefix** whose borrows are
from references it borrows is a derivation in the prefix. -/
theorem derives_prefix {E F : List Event} (hnd : (borrowedIds (E ++ F)).Nodup)
    (hcl : ∀ d rd t, Event.borrowed d rd (some t) ∈ E → t ∈ borrowedIds E) {r : Nat} :
    ∀ {c : Nat}, Derives (E ++ F) r c → c ∈ borrowedIds E ∨ c = r → Derives E r c := by
  intro c hD
  induction hD with
  | refl => exact fun _ => .refl
  | @step d t rd hb hDt ih =>
    rintro (hc | rfl)
    · obtain ⟨rd', par', hb'⟩ := mem_borrowedIds.1 hc
      obtain ⟨rfl, rfl⟩ := borrowed_unique hnd (List.mem_append_left _ hb') hb
      exact .step hb' (ih (Or.inl (hcl _ _ _ hb')))
    · exact .refl

section Core

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
  {Held : State → Shadow → Nat → Reference → Prop}

/-- A trace's references borrowed from others are borrowed from references it
borrows. -/
theorem trace_parent_borrowed (h0 : AdmissibleInitial P s₀) {s : State} {h : Shadow}
    {E : List Event} (hT : Trace P N ν s₀ s h E) :
    ∀ d rd t, Event.borrowed d rd (some t) ∈ E → t ∈ borrowedIds E := by
  intro d rd t hb
  have hBI := borrows_inside_all (F := []) h0 hT (fun _ _ _ _ _ _ _ hS => traced_borrowsInside hS)
    (fun _ _ _ h => by cases h)
  rw [List.append_nil] at hBI
  obtain ⟨rt, hu, -, -⟩ := hBI d rd t hb
  obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hu
  obtain ⟨par, hbt⟩ := used_value (F := []) h0 (heldFacts_frames h0) hT (fun _ _ h => by cases h)
    (by rw [List.append_nil]; exact hk)
  exact mem_borrowedIds.2 ⟨_, _, hbt⟩

/-- **The step of an access while a frame is active**: the frame was entered
before it, and its parameter `r` borrowed before it; the step borrows no
reference derived from `r`, as it borrows only from the reference it accesses
through, which does not derive from `r`. -/
theorem access_step_facts (h0 : AdmissibleInitial P s₀) {s₁ : State} {h₁ : Shadow}
    {E₁ : List Event} (hT₁ : Trace P N ν s₀ s₁ h₁ E₁) {o : Outcome} {post : ShadowPost}
    {E₂ : List Event} (hst : TracedStep P N ν s₁ h₁ o post E₂) {E' G : List Event}
    (hE' : E' = E₁ ++ E₂ ++ G) (hnd : (borrowedIds E').Nodup) {p q : Nat} {f : FrameId}
    {ps : List Nat} {r : Nat} {a : Access} {l : Location} {via : Option Nat}
    (hp : E'[p]? = some (.entered f ps)) (hr : r ∈ ps)
    (hq : E'[q]? = some (.accessed a l via)) (hpq : p < q) (hle : E₁.length ≤ q)
    (hlt : q < E₁.length + E₂.length) (hviaD : ∀ v, via = some v → ¬ Derives E' r v) :
    Event.accessed a l via ∈ E₂ ∧ p < E₁.length ∧ E₁[p]? = some (.entered f ps) ∧
      (∃ k rr par, k < E₁.length ∧ E₁[k]? = some (.borrowed r rr par)) ∧
      ∀ y ry par, Event.borrowed y ry par ∈ E₂ → ¬ Derives E' r y := by
  have ha := List.mem_of_getElem? (mem_step hE' hq hle hlt)
  have hpE₁ : p < E₁.length := by
    refine Nat.lt_of_not_le fun hle' => ?_
    have := (traced_access_quiet hst ha _
      (List.mem_of_getElem? (mem_step hE' hp hle' (by omega)))).1
    simp [Event.entered?] at this
  have hp₁ : E₁[p]? = some (.entered f ps) := by
    rw [hE', List.append_assoc, List.getElem?_append_left hpE₁] at hp
    exact hp
  obtain ⟨k, rr, par, hk, hk'⟩ := param_borrowed_before h0 hT₁ hp₁ hr
  have hkl : k < E₁.length := by
    have := (trace_initial_prefix hT₁).length_le
    omega
  refine ⟨ha, hpE₁, hp₁, ⟨k, rr, par, hkl, hk⟩, fun y ry par' hb hD => ?_⟩
  have hkE' : E'[k]? = some (.borrowed r rr par) := by
    rw [hE', List.append_assoc, List.getElem?_append_left hkl, hk]
  obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hb
  have hjl := (List.getElem?_eq_some_iff.1 hj).1
  have hjE' : E'[E₁.length + j]? = some (.borrowed y ry par') := by
    rw [hE', List.append_assoc, List.getElem?_append_right (by omega), Nat.add_sub_cancel_left,
      List.getElem?_append_left hjl, hj]
  have hbE' : Event.borrowed y ry par' ∈ E' := List.mem_of_getElem? hjE'
  by_cases hyr : y = r
  · subst hyr
    have := borrowed_pos_unique hnd hkE' hjE'
    omega
  · cases hD with
    | refl => exact hyr rfl
    | step hb' hDt =>
      obtain ⟨-, rfl⟩ := borrowed_unique hnd hbE' hb'
      exact hviaD _ (traced_access_borrow hst ha hb) hDt

end Core

/-! ## D4b″ from each step -/

section Steps

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
  {Held : State → Shadow → Nat → Reference → Prop}

/-- **The step of an access** of a trace, or of the step after it. -/
theorem access_split (h0 : AdmissibleInitial P s₀) {s : State} {h : Shadow} {E F : List Event}
    (hT : Trace P N ν s₀ s h E) (hFstep : F ≠ [] → ∃ o post, TracedStep P N ν s h o post F)
    {q : Nat} {a : Access} {l : Location} {via : Option Nat}
    (hq : (E ++ F)[q]? = some (.accessed a l via)) :
    ∃ s₁ h₁ E₁ o post E₂ G, Trace P N ν s₀ s₁ h₁ E₁ ∧ TracedStep P N ν s₁ h₁ o post E₂ ∧
      E ++ F = E₁ ++ E₂ ++ G ∧ E₁.length ≤ q ∧ q < E₁.length + E₂.length := by
  by_cases hqE : q < E.length
  · rw [List.getElem?_append_left hqE] at hq
    obtain ⟨s₁, h₁, E₁, s₂, h₂, E₂, G, hT₁, hst, -, rfl, hle, hlt⟩ := trace_split h0 hT hq
    exact ⟨s₁, h₁, E₁, _, _, E₂, G ++ F, hT₁, hst, by simp, hle, hlt⟩
  · have hFne : F ≠ [] := by
      rintro rfl
      rw [List.append_nil] at hq
      exact hqE (List.getElem?_eq_some_iff.1 hq).1
    obtain ⟨o, post, hS⟩ := hFstep hFne
    have hqlt := (List.getElem?_eq_some_iff.1 hq).1
    rw [List.length_append] at hqlt
    exact ⟨s, h, E, o, post, F, [], hT, hS, by simp, by omega, by omega⟩

/-- The core of `parametersExclusive_of_steps`: an access of the trace `E`
or of the step after it, with events `F`, and a later use. -/
theorem exclusive_core (h0 : AdmissibleInitial P s₀) (hH : HeldFacts P N ν s₀ Held)
    (hB : ∀ s h E, Trace P N ν s₀ s h E → ∀ o post evs, TracedStep P N ν s h o post evs →
      BorrowsInside evs)
    (hX : StepsExclusive P N ν s₀ Held) {s : State} {h : Shadow} {E F : List Event}
    (hT : Trace P N ν s₀ s h E) (hF : ∀ d r, Event.used d r ∈ F → Held s h d r)
    (hBF : BorrowsInside F) (hnd : (borrowedIds (E ++ F)).Nodup)
    (hlt : ∀ d rd t, Event.borrowed d rd (some t) ∈ E ++ F → t < d)
    (hFstep : F ≠ [] → ∃ o post, TracedStep P N ν s h o post F)
    {p q u : Nat} {f : FrameId} {ps : List Nat} {r : Nat} {a : Access} {l : Location}
    {via : Option Nat} {d : Nat} {x : Reference}
    (hp : (E ++ F)[p]? = some (.entered f ps)) (hr : r ∈ ps)
    (hmut : RefSafety.isMutable (E ++ F) r = true)
    (hq : (E ++ F)[q]? = some (.accessed a l via)) (hpq : p < q)
    (hne : ∀ k, p < k → k ≤ q → ∀ rets, (E ++ F)[k]? ≠ some (.frameEnded f rets))
    (hvia : ∀ v, via = some v → RefSafety.derivesFrom (E ++ F) v r = false)
    (hu : (E ++ F)[u]? = some (.used d x)) (hqu : q < u)
    (hder : RefSafety.derivesFrom (E ++ F) d r = true) :
    a.overlaps l x.location = false := by
  -- `r`'s creation.
  obtain ⟨rr, par, hbr, hrm⟩ : ∃ rr par, Event.borrowed r rr par ∈ E ++ F ∧ rr.mutable = true := by
    unfold RefSafety.isMutable at hmut
    cases hc : RefSafety.creation? (E ++ F) r with
    | none => rw [hc] at hmut; cases hmut
    | some y =>
      obtain ⟨rr, par⟩ := y
      rw [hc] at hmut
      exact ⟨rr, par, creation?_mem hc, hmut⟩
  have hrB : r ∈ borrowedIds (E ++ F) := mem_borrowedIds.2 ⟨rr, par, hbr⟩
  have hviaD : ∀ v, via = some v → ¬ Derives (E ++ F) r v := fun v hv hD => by
    have := hvia v hv
    rw [derivesFrom_of_derives hnd hlt hrB hD] at this
    cases this
  have hval : ∀ d x, Event.used d x ∈ E ++ F → ∃ par, Event.borrowed d x par ∈ E ++ F := by
    intro d x hu'
    obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hu'
    obtain ⟨par, hb⟩ := used_value h0 hH hT hF hk
    exact ⟨par, List.mem_append_left _ hb⟩
  -- `r` is borrowed before the access.
  have hiq : ∀ p' r' par', (E ++ F)[p']? = some (.borrowed r r' par') → p' < q := by
    intro p' r' par' hp'
    obtain ⟨s₁, h₁, E₁, o, post, E₂, G, hT₁, hst, hE', hle, hlt'⟩ := access_split h0 hT hFstep hq
    obtain ⟨-, -, -, ⟨k, rk, park, hkl, hk⟩, -⟩ :=
      access_step_facts h0 hT₁ hst hE' hnd hp hr hq hpq hle hlt' hviaD
    have hkE : (E ++ F)[k]? = some (.borrowed r rk park) := by
      rw [hE', List.append_assoc, List.getElem?_append_left hkl, hk]
    rw [borrowed_pos_unique hnd hp' hkE]
    omega
  -- A reference derived from `r` and borrowed after the access has its parent
  -- used after it: the access's own step borrows nothing derived from `r`.
  have hpar : ∀ p' d' rd t, (E ++ F)[p']? = some (.borrowed d' rd (some t)) → q < p' →
      Derives (E ++ F) r d' →
      ∃ u rt, (E ++ F)[u]? = some (.used t rt) ∧ q < u ∧ rd.location.Inside rt.location := by
    intro p' d' rd t hp' hqp hD
    by_cases hpE : p' < E.length
    · have hp'' := hp'
      rw [List.getElem?_append_left hpE] at hp''
      obtain ⟨s₁, h₁, E₁, s₂, h₂, E₂, G, hT₁, hst, -, hEs, hle, hlt'⟩ :=
        trace_split' hT hp'' (derived_not_initial h0)
      have hE' : E ++ F = E₁ ++ E₂ ++ (G ++ F) := by rw [hEs]; simp
      by_cases hq₁ : E₁.length ≤ q
      · exfalso
        obtain ⟨-, -, -, -, hno⟩ := access_step_facts h0 hT₁ hst hE' hnd hp hr hq hpq hq₁
          (by omega) hviaD
        exact hno _ _ _ (List.mem_of_getElem? (mem_step hE' hp' (by omega) (by omega))) hD
      · obtain ⟨rt, hu', hin, -⟩ := hB _ _ _ hT₁ _ _ _ hst d' rd t
          (List.mem_of_getElem? (mem_step hE' hp' (by omega) (by omega)))
        obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hu'
        have hkl := (List.getElem?_eq_some_iff.1 hk).1
        refine ⟨E₁.length + k, rt, ?_, by omega, hin⟩
        rw [hE', List.append_assoc, List.getElem?_append_right (by omega),
          Nat.add_sub_cancel_left, List.getElem?_append_left hkl, hk]
    · have hbF : Event.borrowed d' rd (some t) ∈ F := by
        rw [List.getElem?_append_right (by omega)] at hp'
        exact List.mem_of_getElem? hp'
      by_cases hqE : q < E.length
      · obtain ⟨rt, hu', hin, -⟩ := hBF d' rd t hbF
        obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hu'
        refine ⟨E.length + k, rt, ?_, by omega, hin⟩
        rw [List.getElem?_append_right (by omega), Nat.add_sub_cancel_left, hk]
      · exfalso
        have hFne : F ≠ [] := by rintro rfl; simp at hbF
        obtain ⟨o, post, hS⟩ := hFstep hFne
        have hqlt := (List.getElem?_eq_some_iff.1 hq).1
        rw [List.length_append] at hqlt
        obtain ⟨-, -, -, -, hno⟩ := access_step_facts h0 hT hS (G := []) (by simp) hnd hp hr hq
          hpq (by omega) (by omega) hviaD
        exact hno _ _ _ hbF hD
  have hqb : ∀ d r par, (E ++ F)[q]? ≠ some (.borrowed d r par) := by
    intro d r par h'
    rw [hq] at h'
    cases h'
  obtain ⟨c, u', rc, hDc, hcq, hu', hqu', hin⟩ :=
    live_chain' hnd hpar hval hqb hiq (derives_of_derivesFrom hder) u x hu hqu
  -- `c` is held at the access's step.
  obtain ⟨s₁, h₁, E₁, o, post, E₂, G, hT₁, hst, hE', hle, hlt', hheld⟩ :
      ∃ s₁ h₁ E₁ o post E₂ G, Trace P N ν s₀ s₁ h₁ E₁ ∧ TracedStep P N ν s₁ h₁ o post E₂ ∧
        E ++ F = E₁ ++ E₂ ++ G ∧ E₁.length ≤ q ∧ q < E₁.length + E₂.length ∧
        (Held s₁ h₁ c rc ∨ ∃ par, Event.borrowed c rc par ∈ E₂) := by
    by_cases hqE : q < E.length
    · have hq' := hq
      rw [List.getElem?_append_left hqE] at hq'
      have hcq' : ∀ p r par, E[p]? = some (.borrowed c r par) → p < q := fun p r par hp' =>
        hcq p r par (by
          rw [List.getElem?_append_left (List.getElem?_eq_some_iff.1 hp').1]; exact hp')
      obtain ⟨s₁, h₁, E₁, s₂, h₂, E₂, G, hT₁, hst, -, hEs, hle, hlt', hh, -⟩ :=
        held_across hH hT hF hq' (initial_not_accessed h0) hcq' hu' hqu'
      exact ⟨s₁, h₁, E₁, _, _, E₂, G ++ F, hT₁, hst, by rw [hEs]; simp, hle, hlt', hh⟩
    · have hFne : F ≠ [] := by
        rintro rfl
        rw [List.append_nil] at hq
        exact hqE (List.getElem?_eq_some_iff.1 hq).1
      obtain ⟨o, post, hS⟩ := hFstep hFne
      have hqlt := (List.getElem?_eq_some_iff.1 hq).1
      rw [List.length_append] at hqlt
      have huF : Event.used c rc ∈ F := by
        rw [List.getElem?_append_right (by omega)] at hu'
        exact List.mem_of_getElem? hu'
      exact ⟨s, h, E, o, post, F, [], hT, hS, by simp, by omega, by omega, Or.inl (hF c rc huF)⟩
  obtain ⟨ha, -, hp₁, ⟨k, rk, park, hkl, hk⟩, hno⟩ :=
    access_step_facts h0 hT₁ hst hE' hnd hp hr hq hpq hle hlt' hviaD
  have hheld' : Held s₁ h₁ c rc := by
    rcases hheld with hh | ⟨par', hb⟩
    · exact hh
    · exact (hno _ _ _ hb hDc).elim
  -- The frame is in the state, with its parameters.
  obtain ⟨⟨fr, hfr, hfid⟩, hps⟩ := trace_alive h0 hT₁ hp₁ fun k' hk' rets he => by
    have hk'l := (List.getElem?_eq_some_iff.1 he).1
    exact hne k' hk' (by omega) rets (by
      rw [hE', List.append_assoc, List.getElem?_append_left hk'l, he])
  -- `r`, as borrowed before the step.
  have hkE : (E ++ F)[k]? = some (.borrowed r rk park) := by
    rw [hE', List.append_assoc, List.getElem?_append_left hkl, hk]
  obtain ⟨h1, h2⟩ := borrowed_unique hnd hbr (List.mem_of_getElem? hkE)
  rw [← h1, ← h2] at hk
  -- `c`, borrowed before the step.
  have hcB : c ∈ borrowedIds E₁ := by
    obtain ⟨pc, hpc⟩ := hval c rc (List.mem_of_getElem? hu')
    obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hpc
    have hjq := hcq j rc pc hj
    by_cases hjE : j < E₁.length
    · rw [hE', List.append_assoc, List.getElem?_append_left hjE] at hj
      exact mem_borrowedIds.2 ⟨_, _, List.mem_of_getElem? hj⟩
    · exact (hno _ _ _ (List.mem_of_getElem? (mem_step hE' hj (by omega) (by omega))) hDc).elim
  have hnd₁ : (borrowedIds (E₁ ++ (E₂ ++ G))).Nodup := by
    rw [← List.append_assoc, ← hE']; exact hnd
  have hDc₁ : Derives E₁ r c := derives_prefix hnd₁ (trace_parent_borrowed h0 hT₁)
    (by rw [← List.append_assoc, ← hE']; exact hDc) (Or.inl hcB)
  have hvia₁ : ∀ v, via = some v → ¬ Derives E₁ r v := fun v hv hD =>
    hviaD v hv (by rw [hE', List.append_assoc]; exact hD.append)
  have hX' := hX s₁ h₁ E₁ hT₁ o post E₂ hst a l via ha fr hfr ps (by rw [hfid]; exact hps) r hr
    rr par (List.mem_of_getElem? hk) hrm hvia₁ c rc hheld' hDc₁
  cases hov : a.overlaps l x.location with
  | false => rfl
  | true => rw [overlaps_inside' hin hov] at hX'; cases hX'

/-- **D4b″ from what each step guarantees**, from an admissible initial state:
when, for a `&mut` reference parameter `r` of a frame of the state, no access
of a step that is not through a reference derived from `r` overlaps a
reference derived from `r` held before the step, for a predicate of the held
references with `HeldFacts`, and derived references are inside the ones they
derive from, no access while a frame is active that is not through a
reference derived from one of its `&mut` parameters overlaps a location on
which that parameter is live. -/
theorem parametersExclusive_of_steps (h0 : AdmissibleInitial P s₀)
    (hH : HeldFacts P N ν s₀ Held)
    (hB : ∀ s h E, Trace P N ν s₀ s h E → ∀ o post evs, TracedStep P N ν s h o post evs →
      BorrowsInside evs)
    (hX : StepsExclusive P N ν s₀ Held) : ParametersExclusive P N ν s₀ := by
  intro s h E hT
  have hI := refTraceInv_trace h0 hT
  have hltE := trace_parent_lt h0 hT
  refine ⟨?_, fun o post evs' hS => ?_⟩
  · have := exclusive_of (E := E ++ [])
      fun p q u f ps r a l via d x hp hr hm hq hpq hne hvia hu hqu hder =>
        exclusive_core h0 hH hB hX hT (fun _ _ h => by cases h) (fun _ _ _ h => by cases h)
          (by simpa using hI.unique) (by simpa using hltE) (fun hne => absurd rfl hne)
          hp hr hm hq hpq hne hvia hu hqu hder
    simpa using this
  · exact exclusive_of fun p q u f ps r a l via d x hp hr hm hq hpq hne hvia hu hqu hder =>
      exclusive_core h0 hH hB hX hT (fun d r hu => hH.used _ _ _ hT _ _ _ hS d r hu)
        (hB _ _ _ hT _ _ _ hS) (traced_nodup h0 hT hS)
        (fun d rd t hb => by
          rcases List.mem_append.1 hb with hb | hb
          · exact hltE d rd t hb
          · exact traced_parent_lt hI hS d rd t hb)
        (fun _ => ⟨o, post, hS⟩) hp hr hm hq hpq hne hvia hu hqu hder

end Steps

end MoveBytecode.Proofs
