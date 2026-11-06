-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ExclusiveDefs
import Proofs.ReturnsDerived
import Proofs.RefTrackedFacts
import Proofs.RefContainBelow
import Proofs.HeldAll

/-!
# D4b″ for M3 programs, at each step

UNTRUSTED (checked by the kernel). An access of a verified step that is not
through a reference derived from a `&mut` parameter `r` of a frame `k` overlaps
no reference derived from `r` (`stepsExclusive_M3`):

- through a reference: it derives from another parameter of frame `k`, which
  the call found compatible with `r`, or from a root borrow of a frame at or
  above `k` (`idChain_down`): a local of a frame no older than `k`, in which
  `r` is not, or a resource of a type the borrowing frame acquired while the
  type's module was active once, which no parameter below is in (`ParamChain`,
  R3);
- directly: a local of the current frame, in which `r` is not; `MoveFrom` of a
  resource of a type the frame acquires, which no parameter below is in (R3);
  `MoveTo` of a resource not there, into which no tracked reference is.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

/-! ## Locations -/

theorem loc_eq {l m : Location} (h1 : l.root = m.root) (h2 : l.path = m.path) : l = m := by
  cases l; cases m; simp_all

/-- Accesses in another root overlap nothing. -/
theorem overlaps_root_ne (a : Access) {l m : Location} (h : l.root ≠ m.root) :
    a.overlaps l m = false := by
  have key : (l.root == m.root) = false := by simpa using h
  cases a <;> simp [Access.overlaps, key]

/-- Prefixes of one list are comparable. -/
theorem prefix_comparable {α : Type} {a b c : List α} (ha : a <+: c) (hb : b <+: c) :
    a <+: b ∨ b <+: a := by
  rcases Nat.le_total a.length b.length with h | h
  · exact Or.inl (List.prefix_of_prefix_length_le ha hb h)
  · exact Or.inr (List.prefix_of_prefix_length_le hb ha h)

/-- **Locations inside two compatible references, one of them mutable, do not
overlap.** -/
theorem compat_disjoint {p r : Reference} (hc : Compat p r) (hm : r.mutable = true) (a : Access)
    {x y : Location} (hx : x.Inside p.location) (hy : y.Inside r.location) :
    a.overlaps x y = false := by
  cases hov : a.overlaps x y with
  | false => rfl
  | true =>
    exfalso
    have hxy : x.root = y.root ∧ (x.path <+: y.path ∨ y.path <+: x.path) := by
      cases a <;> simp only [Access.overlaps, Bool.and_eq_true, beq_iff_eq, Bool.or_eq_true,
        List.isPrefixOf_iff_prefix] at hov
      · exact hov
      · exact hov
      · exact ⟨hov.1, Or.inr hov.2⟩
    have hroot : p.location.root = r.location.root := by rw [← hx.1, ← hy.1, hxy.1]
    have hcmp : p.location.path <+: r.location.path ∨ r.location.path <+: p.location.path := by
      rcases hxy.2 with h | h
      · exact prefix_comparable (hx.2.trans h) hy.2
      · exact prefix_comparable hx.2 (hy.2.trans h)
    obtain ⟨h1, h2, -, h4⟩ := hc
    rcases hcmp with h | h
    · by_cases e : r.location.path = p.location.path
      · exact (h2 hm).2 (loc_eq hroot e.symm)
      · have hsi : r.location.StrictlyInside p.location := ⟨⟨hroot.symm, h⟩, e⟩
        cases hpm : p.mutable
        · rw [h4 hpm hsi] at hm; cases hm
        · exact (h1 hpm).1 hsi
    · by_cases e : p.location.path = r.location.path
      · exact (h2 hm).2 (loc_eq hroot e)
      · exact (h2 hm).1 ⟨⟨hroot, h⟩, e⟩

/-! ## Down the frames -/

/-- Reference `d` derives from a borrow frame `fr`, over the frames `sk`, made
of a root of its own: one of its locals, or a resource of a type its function
acquires while the type's module is active at most once. -/
def RootOf (P : Program) (E : List Event) (fr : Frame) (sk : List (FunctionRef × Bool))
    (d : Nat) : Prop :=
  ∃ ρ rρ, Event.borrowed ρ rρ none ∈ E ∧ Derives E ρ d ∧
    ((∃ i, rρ.location.root = .local_ fr.id i) ∨
      ∃ a t, rρ.location.root = .global a t ∧ acqTypes P fr.function t ∧
        countActive P sk t.module ≤ 1)

theorem RootOf.derive {P : Program} {E : List Event} {fr : Frame}
    {sk : List (FunctionRef × Bool)} {t d : Nat} (h : RootOf P E fr sk t) (hD : Derives E t d) :
    RootOf P E fr sk d := by
  obtain ⟨ρ, rρ, hb, hD', hc⟩ := h
  exact ⟨ρ, rρ, hb, hD'.trans hD, hc⟩

/-- **Down the frames**: a reference with its origin in the current frame
derives from a parameter of the frame `k` below it, or from a root borrow of a
frame at or above `k`. -/
theorem idChain_down {P : Program} {E : List Event} :
    ∀ (k : Nat) {gs : List ShadowFrame} {fr : Frame} {frs : List Frame}
      {above : Option (List Nat)}, IdChain P E gs (fr :: frs) above →
    ∀ {pids : List Nat} {d : Nat}, RefSafety.params? E fr.id = some pids →
    Origin P E fr ((fr :: frs).map Frame.skel) pids d →
    ∀ {frk : Frame} {psk : List Nat}, (fr :: frs)[k]? = some frk →
    RefSafety.params? E frk.id = some psk →
    (∃ p ∈ psk, Derives E p d) ∨
      ∃ j ≤ k, ∃ frj, (fr :: frs)[j]? = some frj ∧
        RootOf P E frj (((fr :: frs).drop j).map Frame.skel) d
  | 0, _, fr, frs, _, _, pids, d, hp, hO, frk, psk, hk, hpsk => by
    simp only [List.getElem?_cons_zero, Option.some.injEq] at hk
    subst hk
    rw [hp, Option.some.injEq] at hpsk
    subst hpsk
    rcases hO with h | h
    · exact Or.inl h
    · exact Or.inr ⟨0, Nat.le_refl _, _, rfl, h⟩
  | k + 1, gs, fr, frs, above, hC, pids, d, hp, hO, frk, psk, hk, hpsk => by
    rcases hO with ⟨p, hpp, hDp⟩ | h
    · cases gs with
      | nil => simp [IdChain] at hC
      | cons g gs =>
        obtain ⟨pids0, hp0, -, -, -, htail⟩ := hC
        rw [hp, Option.some.injEq] at hp0
        subst hp0
        cases frs with
        | nil => simp at hk
        | cons fr1 frs' =>
          cases gs with
          | nil => simp [IdChain] at htail
          | cons g1 gs' =>
            obtain ⟨pids1, hp1, hb1, hr1, habove1, hrest1⟩ := htail
            have hO1 := habove1 pids rfl p hpp
            have hC1 : IdChain P E (g1 :: gs') (fr1 :: frs') (some pids) :=
              ⟨pids1, hp1, hb1, hr1, habove1, hrest1⟩
            rcases idChain_down k hC1 hp1 hO1 (by simpa using hk) hpsk with
              ⟨p', hp', hD'⟩ | ⟨j, hj, frj, hfrj, hroot⟩
            · exact Or.inl ⟨p', hp', hD'.trans hDp⟩
            · exact Or.inr ⟨j + 1, by omega, frj, by simpa using hfrj,
                by simpa using hroot.derive hDp⟩
    · exact Or.inr ⟨0, Nat.zero_le _, fr, rfl, h⟩

theorem drop_of_getElem? {α : Type} : ∀ {l : List α} {j : Nat} {x : α}, l[j]? = some x →
    l.drop j = x :: l.drop (j + 1)
  | [], _, _, h => by simp at h
  | a :: l, 0, x, h => by
    simp only [List.getElem?_cons_zero, Option.some.injEq] at h
    simp [h]
  | a :: l, j + 1, x, h => by
    simp only [List.getElem?_cons_succ] at h
    simpa using drop_of_getElem? h

theorem paramChain_drop {P : Program} {E : List Event} : ∀ (j : Nat) {frs : List Frame}
    {T : StructId → Prop}, ParamChain P E frs T →
    ∃ T' : StructId → Prop, (∀ t, T' t → T t) ∧ ParamChain P E (frs.drop j) T'
  | 0, _, T, h => ⟨T, fun _ ht => ht, by simpa using h⟩
  | _ + 1, [], T, _ => ⟨T, fun _ ht => ht, by simp [ParamChain]⟩
  | j + 1, _ :: _, _, ⟨_, _, _, _, T1, hsub, hrest⟩ => by
    obtain ⟨T', h1, h2⟩ := paramChain_drop j hrest
    exact ⟨T', fun t ht => hsub t (h1 t ht), by simpa using h2⟩

/-- **The parameter facts of two frames**, the one at `k` no newer than the
one at `j`. -/
theorem paramChain_at {P : Program} {E : List Event} {frs : List Frame} {T : StructId → Prop}
    (h : ParamChain P E frs T) {j k : Nat} (hjk : j ≤ k) {frj frk : Frame}
    (hj : frs[j]? = some frj) (hk : frs[k]? = some frk) :
    ∃ psj Tj psk Tk, RefSafety.params? E frj.id = some psj ∧
      ParamsOk P E frj ((frs.drop j).map Frame.skel) psj Tj ∧
      RefSafety.params? E frk.id = some psk ∧
      ParamsOk P E frk ((frs.drop k).map Frame.skel) psk Tk ∧
      (∀ t, Tk t → Tj t) ∧ frk.id ≤ frj.id := by
  obtain ⟨Tj, -, hcj⟩ := paramChain_drop j h
  have ej := drop_of_getElem? hj
  rw [ej] at hcj
  obtain ⟨psj, hpj, hokj, hidsj, Tj', hsubj, hrestj⟩ := hcj
  rw [← ej] at hokj
  rcases Nat.lt_or_ge j k with hlt | hge
  · have hk' : (frs.drop (j + 1))[k - (j + 1)]? = some frk := by
      rw [List.getElem?_drop]
      have : j + 1 + (k - (j + 1)) = k := by omega
      rw [this]; exact hk
    obtain ⟨Tk, hsubk, hck⟩ := paramChain_drop (k - (j + 1)) hrestj
    have hdrop : (frs.drop (j + 1)).drop (k - (j + 1)) = frs.drop k := by
      rw [List.drop_drop]; congr 1; omega
    rw [hdrop] at hck
    have ek := drop_of_getElem? hk
    rw [ek] at hck
    obtain ⟨psk, hpk, hokk, -, -⟩ := hck
    rw [← ek] at hokk
    exact ⟨psj, Tj, psk, Tk, hpj, hokj, hpk, hokk, fun t ht => hsubj t (hsubk t ht),
      Nat.le_of_lt (hidsj frk (List.mem_of_getElem? hk'))⟩
  · have hjk' : j = k := by omega
    subst hjk'
    rw [hj, Option.some.injEq] at hk
    subst hk
    exact ⟨psj, Tj, psj, Tj, hpj, hokj, hpj, hokj, fun _ ht => ht, Nat.le_refl _⟩

/-! ## A step -/

/-- A native call accesses nothing. -/
theorem nativeEvents_accessed {ν : NativeFlows} {f : FunctionRef} {args vals : List Value}
    {g : ShadowFrame} {m : ModuleId} {fresh : Nat} {a : Access} {l : Location}
    {via : Option Nat} : Event.accessed a l via ∉ nativeEvents ν f args vals g m fresh := by
  intro hb
  simp only [nativeEvents, List.mem_append] at hb
  rcases hb with hb | hb
  · obtain ⟨_, _, _, _, he, -⟩ := consumedFrom_mem _ hb
    cases he
  · obtain ⟨_, _, _, he⟩ := (nativeResults_spec ν f args vals _ m fresh).2.2 _ hb
    cases he

section M3

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
  (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
  (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀)
include hwf hs hapi hN h0

/-- **D4b″ at each step of a verified program**, given the parameter facts
along executions. -/
theorem stepsExclusive'
    (hPC : ∀ s h E, Trace P N ν s₀ s h E → ∃ T, ParamChain P E s.frames T) :
    StepsExclusive P N ν s₀ (Tracked P) := by
  intro s h E hT o post evs hS a l via ha fr hfr ps hps r hr rr par hrr hmut hvia i ri hti hD
  have hI := refTraceInv_trace h0 hT
  have hinvA := invCore_reachable hwf hs hapi hN h0 s (trace_reachable hT)
  have hval : ∀ d x, Event.used d x ∈ E → ∃ par, Event.borrowed d x par ∈ E := by
    intro d x hu'
    obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hu'
    exact used_value (F := []) h0 (heldFacts_frames h0) hT (fun _ _ h => by cases h)
      (by rw [List.append_nil]; exact hk)
  have hBI : BorrowsInside E := by
    have := borrows_inside_all (F := []) h0 hT (fun _ _ _ _ _ _ _ hS' => traced_borrowsInside hS')
      (fun _ _ _ h => by cases h)
    rwa [List.append_nil] at this
  have hinside : ∀ {p d : Nat} {rp rd : Reference} {pp pd : Option Nat},
      Event.borrowed p rp pp ∈ E → Derives E p d → Event.borrowed d rd pd ∈ E →
      rd.location.Inside rp.location :=
    fun hp hD hd => (derives_inside hI.unique hBI hval hp hD hd).1
  obtain ⟨pi, hbi⟩ := hI.refs i ri (tracked_occ hI.pre.fits hti)
  have hin : ri.location.Inside rr.location := hinside hrr hD hbi
  obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hfr
  obtain ⟨T, hPCs⟩ := hPC s h E hT
  have hC := idChain_trace hwf hs hapi hN h0 hT
  cases hS with
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨kk, hshape⟩
    case inr =>
      obtain ⟨g, hs', fr0, rest, n, t, v, hhf, hsf, -, -, hslot, -, hvt, hl, -⟩ :=
        native_access hI.pre hstep hn ha
      have hg : g.fits fr0 = true := by
        have hfit := hI.pre.fits
        rw [shadow_fits, hhf, hsf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
        exact hfit.1.1
      rw [hsf] at hk hPCs
      obtain ⟨ps0, T0, psk, Tk, -, hok0, hpk, hokk, hTk0, hidk0⟩ :=
        paramChain_at hPCs (Nat.zero_le k) rfl hk
      rw [hps, Option.some.injEq] at hpk
      subst hpk
      have hC' := hC
      rw [hhf, hsf] at hC'
      -- An access through a reference of the current frame.
      have key : ∀ t v, StackSlot g fr0 t v → via = some t → l = v.location →
          a.overlaps l ri.location = false := by
        intro t v hslot hvt hl
        subst hl
        have hocc0 : (t, Value.ref v) ∈ g.occurrences fr0 := refSlot_occ hg hslot.refSlot
        obtain ⟨pt, hbt⟩ := hI.refs t v (by
          rw [hhf, hsf, framesOcc_cons]; exact List.mem_append_left _ hocc0)
        have hC'' := hC'
        obtain ⟨pids0, hpids0, -, hrefs0, -, -⟩ := hC''
        rcases idChain_down k hC' hpids0 (hrefs0 t v hocc0) hk hps with
          ⟨p, hpp, hDp⟩ | ⟨j, hjk, frj, hj, hroot⟩
        · -- Derived from another parameter of the frame.
          have hpr : p ≠ r := fun e => hvia t hvt (e ▸ hDp)
          obtain ⟨rp, pp, hbp⟩ := mem_borrowedIds.1 (hokk.mem p hpp)
          exact compat_disjoint (hokk.compat p hpp r hr hpr rp rr pp par hbp hrr) hmut a
            (hinside hbp hDp hbt) hin
        · -- Derived from a root borrow of a frame at or above it.
          obtain ⟨ρ, rρ, hbρ, hDρ, hcond⟩ := hroot
          have hvρ := hinside hbρ hDρ hbt
          obtain ⟨psj, Tj, psk', Tk', -, hokj, hpk', hokk', hTkj, hidkj⟩ :=
            paramChain_at hPCs hjk hj hk
          rw [hps, Option.some.injEq] at hpk'
          subst hpk'
          rcases hcond with ⟨i', hi'⟩ | ⟨a', t', ht', hacq, hcnt⟩
          · apply overlaps_root_ne
            intro e
            have := hokk'.below r hr rr par hrr frj.id i' (by rw [← hin.1, ← e, hvρ.1, hi'])
            exact Nat.lt_irrefl _ (Nat.lt_of_lt_of_le this hidkj)
          · by_cases e : v.location.root = ri.location.root
            · exfalso
              have hTk := hokk'.glob r hr rr par hrr a' t' (by rw [← hin.1, ← e, hvρ.1, ht'])
              have := hokj.acq t' (hTkj t' hTk) hacq
              omega
            · exact overlaps_root_ne a e
      exact key _ _ hslot hvt hl
    obtain ⟨rfl, rfl, rfl, g, hs', info, hh, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at ha
    exact absurd ha nativeEvents_accessed
  | core hstep hc =>
    obtain ⟨fr0, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hex⟩ := coreStep_exec hc
    have hstep' : stepOf (Step.exec P s fr0 rest info code instr) = .done o := by
      simp only [step, hsf, hinfo, hcode, hinstr] at hstep
      exact hstep
    have hcont := coreStep_continues hc
    obtain ⟨g, hs', hhf, -⟩ := shadow_cons hI.pre hsf
    have hg : g.fits fr0 = true := by
      have hfit := hI.pre.fits
      rw [shadow_fits, hhf, hsf, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at hfit
      exact hfit.1.1
    rw [hsf] at hk hPCs
    obtain ⟨ps0, T0, psk, Tk, -, hok0, hpk, hokk, hTk0, hidk0⟩ :=
      paramChain_at hPCs (Nat.zero_le k) rfl hk
    rw [hps, Option.some.injEq] at hpk
    subst hpk
    have hC' := hC
    rw [hhf, hsf] at hC'
    -- An access through a reference of the current frame.
    have key : ∀ t v, StackSlot g fr0 t v → via = some t → l = v.location →
        a.overlaps l ri.location = false := by
      intro t v hslot hvt hl
      subst hl
      have hocc0 : (t, Value.ref v) ∈ g.occurrences fr0 := refSlot_occ hg hslot.refSlot
      obtain ⟨pt, hbt⟩ := hI.refs t v (by
        rw [hhf, hsf, framesOcc_cons]; exact List.mem_append_left _ hocc0)
      have hC'' := hC'
      obtain ⟨pids0, hpids0, -, hrefs0, -, -⟩ := hC''
      rcases idChain_down k hC' hpids0 (hrefs0 t v hocc0) hk hps with
        ⟨p, hpp, hDp⟩ | ⟨j, hjk, frj, hj, hroot⟩
      · -- Derived from another parameter of the frame.
        have hpr : p ≠ r := fun e => hvia t hvt (e ▸ hDp)
        obtain ⟨rp, pp, hbp⟩ := mem_borrowedIds.1 (hokk.mem p hpp)
        exact compat_disjoint (hokk.compat p hpp r hr hpr rp rr pp par hbp hrr) hmut a
          (hinside hbp hDp hbt) hin
      · -- Derived from a root borrow of a frame at or above it.
        obtain ⟨ρ, rρ, hbρ, hDρ, hcond⟩ := hroot
        have hvρ := hinside hbρ hDρ hbt
        obtain ⟨psj, Tj, psk', Tk', -, hokj, hpk', hokk', hTkj, hidkj⟩ :=
          paramChain_at hPCs hjk hj hk
        rw [hps, Option.some.injEq] at hpk'
        subst hpk'
        rcases hcond with ⟨i', hi'⟩ | ⟨a', t', ht', hacq, hcnt⟩
        · apply overlaps_root_ne
          intro e
          have := hokk'.below r hr rr par hrr frj.id i' (by rw [← hin.1, ← e, hvρ.1, hi'])
          exact Nat.lt_irrefl _ (Nat.lt_of_lt_of_le this hidkj)
        · by_cases e : v.location.root = ri.location.root
          · exfalso
            have hTk := hokk'.glob r hr rr par hrr a' t' (by rw [← hin.1, ← e, hvρ.1, ht'])
            have := hokj.acq t' (hTkj t' hTk) hacq
            omega
          · exact overlaps_root_ne a e
    cases exec_accessed hhf hex ha with
    | inspect hslot _ _ => exact key _ _ hslot rfl rfl
    | viaRef hslot _ _ _ => exact key _ _ hslot rfl rfl
    | @direct _ j hop =>
      apply overlaps_root_ne
      intro e
      have := hokk.below r hr rr par hrr fr0.id j (by rw [← hin.1]; exact e.symm)
      exact Nat.lt_irrefl _ (Nat.lt_of_lt_of_le this hidk0)
    | @global addr sid hop =>
      by_cases e : ri.location.root = .global addr sid
      · exfalso
        have hrg : rr.location.root = .global addr sid := by rw [← hin.1, e]
        -- A wrapper in progress does no global storage operation.
        have hinv : InvRefs P s := by
          rcases hinvA.2 with hr' | ⟨sc, hrun⟩
          · exact ⟨hinvA.1, hr'⟩
          · exfalso
            unfold GlobalOperand at hop
            rcases hop with ⟨d, rfl⟩ | ⟨d, rfl⟩ | ⟨d, rfl⟩ | ⟨d, rfl⟩ <;>
            rcases apiCore_instr hrun hsf hinfo hcode hinstr with ⟨-, h⟩ |
              ⟨-, ⟨f, h⟩ | ⟨v, h⟩ | ⟨f, h⟩ | ⟨v, h⟩⟩ | ⟨-, h⟩ <;> cases h
        unfold GlobalOperand at hop
        have hcase : ((∃ d, instr = .moveFrom d) ∨ ∃ i, instr = .moveFromGeneric i) ∨
            ((∃ d, instr = .moveTo d) ∨ ∃ i, instr = .moveToGeneric i) := by
          rcases hop with h | h | h | h
          · exact Or.inl (Or.inl h)
          · exact Or.inr (Or.inl h)
          · exact Or.inl (Or.inr h)
          · exact Or.inr (Or.inr h)
        rcases hcase with hmf | hmt
        · -- `MoveFrom`, generic or not, of a resource of a type the frame acquires: R3.
          obtain ⟨addr', v, sid', hhead, htop, hvt, hl⟩ := ct_moveFrom_loc hmf hhf hex ha
          simp only [Location.mk.injEq, Root.global.injEq, and_true] at hl
          obtain ⟨rfl, rfl⟩ := hl
          obtain ⟨d, tys, hdcase, sid0, a0, v0, st, hsid, hstk, hl1, hget, htop0⟩ :=
            ct_moveFrom_step hmf hstep' hcont
          rw [hstk] at hhead
          simp only [List.head?_cons, Option.some.injEq, Value.address.injEq] at hhead
          subst hhead
          rw [htop0, Option.some.injEq] at htop
          subst htop
          obtain ⟨res, hres, hrt, hrv⟩ := get?_mem hget
          have hvt0 := hI.pre.inv.typed res hres
          rw [hrt, hrv, hvt, RuntimeType.struct.injEq] at hvt0
          subst hvt0
          have hacq : d ∈ info.definition.acquiresGlobalResources := by
            have hacq0 := function_acquires hwf hinfo hcode
            rcases hdcase with ⟨rfl, -⟩ | ⟨i, x, rfl, hx0, rfl, -⟩
            · exact (acquires_instr hacq0 (getElem?_mem_toList hinstr)).1 _ (Or.inl rfl)
            · exact acquires_instrGeneric hacq0 (getElem?_mem_toList hinstr) (Or.inl rfl) hx0
          have hacqT : acqTypes P fr0.function sid :=
            ⟨d, by simp only [acqOf, hinfo, Option.map_some, Option.getD_some]; exact hacq,
              globalType_erase hsid⟩
          have hcount : countActive P ((fr0 :: rest).map Frame.skel) sid.module ≤ 1 := by
            rw [← hsf, ← hinv.2.active.count]; omega
          have hTk := hokk.glob r hr rr par hrr _ _ hrg
          have := hok0.acq sid (hTk0 sid hTk) hacqT
          simp only [List.drop_zero] at this
          omega
        · -- `MoveTo`, generic or not, of a resource not there: no tracked reference is
          -- into it.
          obtain ⟨v, sid', rs, addr', hhead, hvt, hrs, hderef, hl⟩ := ct_moveTo_loc hmt hhf hex ha
          simp only [Location.mk.injEq, Root.global.injEq, and_true] at hl
          obtain ⟨rfl, rfl⟩ := hl
          obtain ⟨sid0, v0, rs0, st, a0, hstk, -, hvt0, hderef0, hget⟩ :=
            ct_moveTo_step hmt hsf hstep' hcont
          rw [hstk] at hhead hrs
          simp only [List.head?_cons, Option.some.injEq] at hhead
          subst hhead
          simp only [List.getElem?_cons_succ, List.getElem?_cons_zero, Option.some.injEq,
            Value.ref.injEq] at hrs
          subst hrs
          rw [hvt0, RuntimeType.struct.injEq] at hvt
          subst hvt
          rw [hderef0, Option.some.injEq, Value.signer.injEq] at hderef
          subst hderef
          obtain ⟨frx, -, hok⟩ := tracked_refOk' hwf hs hapi hinvA hti
          have hread := (deref?_isSome.1 hok.valid).2
          rw [Location.eta ri.location, e] at hread
          simp [State.read?, State.root?, hget] at hread
      · exact overlaps_root_ne _ (fun e' => e e'.symm)

end M3

/-- **D4b″ at each step of an M3 program**, a corollary. -/
theorem stepsExclusive_M3 {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.SupportedNonGeneric) (hm3 : P.M3)
    (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀)
    (hPC : ∀ s h E, Trace P N ν s₀ s h E → ∃ T, ParamChain P E s.frames T) :
    StepsExclusive P N ν s₀ (Tracked P) :=
  stepsExclusive' hwf (supported_of_nonGeneric hwf hs) (apiVersions_M3 hm3) hN h0 hPC

end MoveBytecode.Proofs
