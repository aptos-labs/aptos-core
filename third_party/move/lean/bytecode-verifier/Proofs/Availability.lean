-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Linking
import Proofs.Certified

/-!
# The availability certificate C2 from the model

UNTRUSTED (checked by the kernel). The locals-safety pass runs the abstract
interpreter over the blocks of a function and returns each analyzed block's
pre-state. When that map is a post-fixpoint of the analysis (the entry's
pre-state is at least the initial state, and every analyzed block executes
to a post-state at most the pre-state of each successor), the states before
each instruction form an availability certificate that is consistent in the
specification's sense (`AvailabilityCertificate.Consistent`).

The model's transfer function is exactly the specification's
`availabilityAfter`, and its checks are exactly the specification's
availability requirements on reads. That the scheduler of the abstract
interpreter computes a post-fixpoint is `SchedulerSound`, proved separately
from the properties of the depth-first traversal.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Abstract local states as availability -/

/-- The availability an abstract local state stands for. A local outside the
state is `maybe`. -/
def toAvail : Option LocalState → Availability
  | some .available => .available
  | some .unavailable => .unavailable
  | some .maybeAvailable => .maybe
  | none => .maybe

def stateAvail (st : LocalsState) (i : Nat) : Availability := toAvail st.states[i]?

/-- `a` is at least as precise as `b`, local by local. -/
def StateLe (a b : LocalsState) : Prop :=
  a.states.size = b.states.size ∧ ∀ i, Availability.Refines (stateAvail a i) (stateAvail b i)

theorem Refines.refl (a : Availability) : Availability.Refines a a := Or.inl rfl

theorem Refines.trans {a b c : Availability} (h1 : Availability.Refines a b)
    (h2 : Availability.Refines b c) : Availability.Refines a c := by
  rcases h1 with rfl | rfl <;> rcases h2 with rfl | rfl
  · exact Or.inl rfl
  · exact Or.inr rfl
  · exact Or.inr rfl
  · exact Or.inr rfl

theorem stateLe_refl (a : LocalsState) : StateLe a a := ⟨rfl, fun _ => Refines.refl _⟩

theorem stateLe_trans {a b c : LocalsState} (h1 : StateLe a b) (h2 : StateLe b c) : StateLe a c :=
  ⟨h1.1.trans h2.1, fun i => Refines.trans (h1.2 i) (h2.2 i)⟩

theorem joinLocal_refines_left (x y : LocalState) :
    Availability.Refines (toAvail (some x)) (toAvail (some (joinLocal x y))) := by
  cases x <;> cases y <;> simp [joinLocal, toAvail, Availability.Refines]

theorem joinLocal_refines_right (x y : LocalState) :
    Availability.Refines (toAvail (some y)) (toAvail (some (joinLocal x y))) := by
  cases x <;> cases y <;> simp [joinLocal, toAvail, Availability.Refines]

/-- The pointwise join of two lists of local states. -/
theorem getElem?_zip_map : ∀ {xs ys : List LocalState}, xs.length = ys.length → ∀ (i : Nat),
    ((xs.zip ys).map (fun p => joinLocal p.1 p.2))[i]? =
      (xs[i]?).bind (fun x => (ys[i]?).map (joinLocal x))
  | [], [], _, i => by simp
  | x :: xs, y :: ys, hl, 0 => by simp
  | x :: xs, y :: ys, hl, i + 1 => by
    simpa using getElem?_zip_map (xs := xs) (ys := ys) (by simpa using hl) i

theorem refines_join_left (x : Option LocalState) (y : Option LocalState) (hxy : x.isSome = y.isSome) :
    Availability.Refines (toAvail x) (toAvail (x.bind (fun x => y.map (joinLocal x)))) := by
  cases x <;> cases y <;> simp_all [Refines.refl, joinLocal_refines_left]

theorem refines_join_right (x : Option LocalState) (y : Option LocalState) (hxy : x.isSome = y.isSome) :
    Availability.Refines (toAvail y) (toAvail (x.bind (fun x => y.map (joinLocal x)))) := by
  cases x <;> cases y <;> simp_all [Refines.refl, joinLocal_refines_right]

/-- The join is an upper bound, and returns the old state when nothing
changed. -/
theorem join_ok {a b j : LocalsState} {changed : Bool} (h : a.join b = .ok (j, changed)) :
    StateLe a j ∧ StateLe b j ∧ (changed = false → j = a) := by
  unfold LocalsState.join at h
  simp only at h
  split at h
  · simp [panicAt] at h
  · next hsz =>
    simp only [Bool.or_eq_true, bne_iff_ne, ne_eq, not_or, Decidable.not_not] at hsz
    obtain ⟨hs, -⟩ := hsz
    have hl : a.states.toList.length = b.states.toList.length := by simpa using hs
    have hget := getElem?_zip_map hl
    simp only [Array.getElem?_toList] at hget
    have hsome : ∀ i : Nat, (a.states[i]?).isSome = (b.states[i]?).isSome := by
      intro i
      rcases Nat.lt_or_ge i a.states.size with hi | hi
      · rw [Array.getElem?_eq_getElem hi, Array.getElem?_eq_getElem (hs ▸ hi)]; rfl
      · rw [Array.getElem?_eq_none hi, Array.getElem?_eq_none (hs ▸ hi)]
    split at h
    · next heq =>
      simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      refine ⟨stateLe_refl _, ⟨hs.symm, fun i => ?_⟩, fun _ => rfl⟩
      have := congrArg (·[i]?) (beq_iff_eq.1 heq)
      simp only [hget, Array.getElem?_toList] at this
      simp only [stateAvail]
      rw [← this]
      exact refines_join_right _ _ (hsome i)
    · simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      refine ⟨⟨by simp [hl]; omega, fun i => ?_⟩, ⟨by simp [hl], fun i => ?_⟩, by simp⟩
      · simp only [stateAvail, List.getElem?_toArray, hget]
        exact refines_join_left _ _ (hsome i)
      · simp only [stateAvail, List.getElem?_toArray, hget]
        exact refines_join_right _ _ (hsome i)

theorem stateAvail_set {st : LocalsState} {i : Nat} (hi : i < st.states.size) (s : LocalState)
    (j : Nat) :
    stateAvail { st with states := st.states.setIfInBounds i s } j =
      if j = i then toAvail (some s) else stateAvail st j := by
  simp only [stateAvail, Array.getElem?_setIfInBounds]
  by_cases hj : j = i
  · subst hj; simp [hi]
  · simp [hj, Ne.symm hj]

theorem lookup_lt {α : Type} {a : Array α} {i : Nat} {w : String} {x : α}
    (h : lookup a i w = .ok x) : i < a.size ∧ a[i]? = some x := by
  rw [lookup_ok] at h
  exact ⟨lt_of_getElem? h, h⟩

/-- The model's transfer function is the specification's availability
transfer, and it requires every local an instruction reads to be available. -/
theorem execute_ok {st st' : LocalsState} {instr : Bytecode} {o e : Nat}
    (h : st.execute instr o e = .ok st') :
    st'.states.size = st.states.size ∧
      (∀ i, stateAvail st' i = availabilityAfter instr (stateAvail st) i) ∧
      (∀ i, instr.readsLocal = some i → stateAvail st i = .available) := by
  cases instr
  case stLoc i =>
    simp only [LocalsState.execute, bind_ok] at h
    obtain ⟨s, hs, h⟩ := h
    obtain ⟨hi, -⟩ := lookup_lt hs
    have hres : st' = { st with states := st.states.setIfInBounds i .available } := by
      cases s
      · simp only [pure, Except.pure, Except.ok.injEq, Array.set!_eq_setIfInBounds] at h
        exact h.symm
      all_goals
        simp only [bind_ok] at h
        obtain ⟨ab, -, h⟩ := h
        split at h
        · simp [reject] at h
        · simp only [pure, Except.pure, Except.ok.injEq, Array.set!_eq_setIfInBounds] at h
          exact h.symm
    subst hres
    refine ⟨by simp, fun j => ?_, fun j hj => by simp [Bytecode.readsLocal] at hj⟩
    rw [stateAvail_set hi]
    simp [availabilityAfter, toAvail]
  case moveLoc i =>
    simp only [LocalsState.execute, bind_ok] at h
    obtain ⟨s, hs, h⟩ := h
    obtain ⟨hi, hsi⟩ := lookup_lt hs
    cases s <;> simp only [reject, pure, Except.pure, Except.ok.injEq, reduceCtorEq,
      Array.set!_eq_setIfInBounds] at h
    subst h
    refine ⟨by simp, fun j => ?_, fun j hj => ?_⟩
    · rw [stateAvail_set hi]
      simp [availabilityAfter, toAvail]
    · simp only [Bytecode.readsLocal, Option.some.injEq] at hj
      subst hj
      simp [stateAvail, hsi, toAvail]
  case copyLoc i | mutBorrowLoc i | immBorrowLoc i =>
    simp only [LocalsState.execute, bind_ok] at h
    obtain ⟨s, hs, h⟩ := h
    obtain ⟨hi, hsi⟩ := lookup_lt hs
    cases s <;> simp only [reject, pure, Except.pure, Except.ok.injEq, reduceCtorEq] at h
    subst h
    refine ⟨rfl, fun j => by simp [availabilityAfter], fun j hj => ?_⟩
    simp only [Bytecode.readsLocal, Option.some.injEq] at hj
    subst hj
    simp [stateAvail, hsi, toAvail]
  case ret =>
    simp only [LocalsState.execute] at h
    split at h
    · simp [panicAt, bind, Except.bind] at h
    · simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at h
      obtain ⟨_, -, rfl⟩ := h
      exact ⟨rfl, fun j => by simp [availabilityAfter], fun j hj => by
        simp [Bytecode.readsLocal] at hj⟩
  all_goals
    simp only [LocalsState.execute, pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact ⟨rfl, fun j => by simp [availabilityAfter], fun j hj => by
      simp [Bytecode.readsLocal] at hj⟩

/-! ## From blocks to instructions -/

/-- The analysis' state before each instruction: the recorded pre-state at a
block id, otherwise the state after the previous instruction. -/
def availBefore (code : Array Bytecode) (inv : List (Nat × LocalsState)) : Nat → Option LocalsState
  | 0 => inv.lookup 0
  | pc + 1 =>
    if pc + 1 ∈ blockIds code then inv.lookup (pc + 1)
    else (availBefore code inv pc).bind fun st => match code[pc]? with
      | some instr => (st.execute instr pc 0).toOption
      | none => none

theorem availBefore_of_mem {code : Array Bytecode} {inv : List (Nat × LocalsState)} {pc : Nat}
    (h : pc ∈ blockIds code) : availBefore code inv pc = inv.lookup pc := by
  cases pc with
  | zero => rfl
  | succ p => simp [availBefore, h]

theorem execute_irrelevant (st : LocalsState) (instr : Bytecode) (o e o' e' : Nat) :
    st.execute instr o e = st.execute instr o' e' := rfl

/-- The step `executeBlock` folds. -/
def blockStep (code : Array Bytecode) (e : Nat) (st : LocalsState) (offset : Nat) :
    VResult LocalsState := do
  localsAnalysis.execute st (← lookup code offset "code[offset]") offset e

theorem blockStep_ok {code : Array Bytecode} {e : Nat} {st st' : LocalsState} {pc : Nat}
    (h : blockStep code e st pc = .ok st') :
    ∃ instr, code[pc]? = some instr ∧ st.execute instr pc 0 = .ok st' := by
  simp only [blockStep, bind_ok, lookup_ok] at h
  obtain ⟨instr, hi, h⟩ := h
  exact ⟨instr, hi, h⟩

/-- Along a block that executes from its recorded pre-state, `availBefore`
follows the execution. -/
theorem availBlock_prefix {code : Array Bytecode} {inv : List (Nat × LocalsState)}
    {b : BasicBlock} (hb : BlockOk code b) {pre : LocalsState}
    (hpre : inv.lookup b.start = some pre) :
    ∀ j st, j ≤ b.exit + 1 - b.start →
      (List.range' b.start j).foldlM (blockStep code b.exit) pre = .ok st →
      (b.start + j ≤ b.exit → availBefore code inv (b.start + j) = some st) := by
  intro j
  induction j with
  | zero =>
    intro st _ h _
    simp [List.range', pure, Except.pure] at h
    subst h
    rw [Nat.add_zero, availBefore_of_mem hb.start_mem]
    simpa using hpre
  | succ j ih =>
    intro st hj h hle
    obtain ⟨st0, h0, h1⟩ := foldlM_last h
    have hcert0 := ih st0 (by omega) h0 (by omega)
    obtain ⟨instr, hi, hex⟩ := blockStep_ok h1
    rw [show b.start + (j + 1) = b.start + j + 1 by omega]
    simp [availBefore, hb.inner (b.start + j + 1) (by omega) (by omega), hcert0, hi, hex,
      Except.toOption]

/-! ## Blocks of the control-flow graph -/

theorem block_exit_unique {code : Array Bytecode} {b1 b2 : BasicBlock} (h1 : BlockOk code b1)
    (h2 : BlockOk code b2) (hs : b1.start = b2.start) : b1.exit ≤ b2.exit := by
  rcases Nat.lt_or_ge b2.exit b1.exit with hlt | hge
  · exfalso
    have hend := h2.exit_end
    simp only [isEndOfBlock, Bool.or_eq_true, beq_iff_eq, List.contains_iff_mem] at hend
    rcases hend with hend | hend
    · have := h1.exit_lt; omega
    · exact h1.inner (b2.exit + 1) (by have := h2.start_le_exit; omega) (by omega) hend
  · exact hge

/-- Blocks are determined by their start. -/
theorem block_start_unique {code : Array Bytecode} {b1 b2 : BasicBlock}
    (h1 : b1 ∈ buildBlocks code) (h2 : b2 ∈ buildBlocks code) (hs : b1.start = b2.start) :
    b1 = b2 := by
  have o1 := (buildBlocks_spec code).1 b1 h1
  have o2 := (buildBlocks_spec code).1 b2 h2
  have he : b1.exit = b2.exit :=
    Nat.le_antisymm (block_exit_unique o1 o2 hs) (block_exit_unique o2 o1 hs.symm)
  have e1 := o1.successors
  have e2 := o2.successors
  cases b1; cases b2
  simp only at hs he e1 e2
  subst hs he
  rw [e1, e2]

/-- The block containing an instruction. -/
theorem block_of_pc {code : Array Bytecode} {pc : Nat} (hpc : pc < code.size) :
    ∃ b ∈ buildBlocks code, b.start ≤ pc ∧ pc ≤ b.exit :=
  (buildBlocks_spec code).2 pc hpc

/-- A block id starts its block. -/
theorem block_of_mem {code : Array Bytecode} {s : Nat} (hs : s ∈ blockIds code)
    (hlt : s < code.size) : ∃ b ∈ buildBlocks code, b.start = s := by
  obtain ⟨b, hb, hle, hge⟩ := block_of_pc hlt
  refine ⟨b, hb, ?_⟩
  rcases Nat.eq_or_lt_of_le hle with h | h
  · exact h
  · exact absurd hs (((buildBlocks_spec code).1 b hb).inner s h hge)

theorem find_block {code : Array Bytecode} {b : BasicBlock} (hb : b ∈ buildBlocks code) :
    (buildBlocks code).find? (·.start == b.start) = some b := by
  cases hf : (buildBlocks code).find? (·.start == b.start) with
  | none =>
    rw [List.find?_eq_none] at hf
    exact absurd (by simp) (hf b hb)
  | some b' =>
    have hmem := List.mem_of_find?_eq_some hf
    have hp := List.find?_some hf
    simp only [beq_iff_eq] at hp
    rw [block_start_unique hmem hb hp]

theorem cfg_blockEnd {cfg : Cfg} {code : Array Bytecode} (hcfg : cfg.blocks = buildBlocks code)
    {b : BasicBlock} (hb : b ∈ buildBlocks code) : cfg.blockEnd b.start = b.exit := by
  simp [Cfg.blockEnd, hcfg, find_block hb]

theorem cfg_successors {cfg : Cfg} {code : Array Bytecode} (hcfg : cfg.blocks = buildBlocks code)
    {b : BasicBlock} (hb : b ∈ buildBlocks code) : cfg.successors b.start = b.successors := by
  simp [Cfg.successors, blockSuccessors, hcfg, find_block hb]

theorem executeBlock_eq {cfg : Cfg} {code : Array Bytecode} (hcfg : cfg.blocks = buildBlocks code)
    {b : BasicBlock} (hb : b ∈ buildBlocks code) (pre : LocalsState) :
    executeBlock localsAnalysis cfg code b.start pre =
      (List.range' b.start (b.exit + 1 - b.start)).foldlM (blockStep code b.exit) pre := by
  simp only [executeBlock, cfg_blockEnd hcfg hb]
  rfl

/-! ## Successors -/

theorem not_branch_fallThrough {i : Bytecode} (h : isBranch i = false) :
    i.control = .fallThrough := by
  cases hc : i.control with
  | fallThrough => rfl
  | jump o => rw [(control_jump hc).2] at h; cases h
  | branch o => rw [(control_branch hc).2] at h; cases h
  | exit => rw [(control_exit hc).2] at h; cases h

theorem mem_getSuccessors {code : Array Bytecode} {pc s : Nat} {i : Bytecode}
    (hi : code[pc]? = some i) (hs : s ∈ successors code pc) (hlt : s < code.size) :
    s ∈ getSuccessors pc code := by
  -- Membership before the final ordering of two successors.
  have hsorted : ∀ v : List Nat, s ∈ v → (pc + 1 < code.size) →
      s ∈ (match v with
        | [a, c] => if a > c then [c, a] else v
        | _ => v) := by
    intro v hv _
    split
    · next a c =>
      split
      · simp at hv ⊢; omega
      · exact hv
    · exact hv
  have hoff : offset i = some s → s ∈ getSuccessors pc code := by
    intro ho
    unfold getSuccessors
    simp only [hi, ho]
    split
    · simp
    · next hge =>
      apply hsorted _ _ (by omega)
      split <;> simp
  have hnext : s = pc + 1 → isUnconditionalBranch i = false → s ∈ getSuccessors pc code := by
    intro hs1 hu
    subst hs1
    unfold getSuccessors
    simp only [hi]
    split
    · omega
    · apply hsorted _ _ (by omega)
      split
      · split
        · simp
        · next hc =>
          simp only [hu] at hc
          simp only [Bool.not_false, Bool.true_and, Bool.not_eq_true', List.contains_cons,
            List.contains_nil, Bool.or_false, beq_eq_false_iff_ne, ne_eq,
            Decidable.not_not] at hc
          simp [hc]
      · simp [hu]
  simp only [successors, hi] at hs
  cases hc : i.control with
  | fallThrough =>
    rw [hc] at hs
    simp only [List.mem_singleton] at hs
    exact hnext hs (not_unconditional (Or.inl hc))
  | jump o =>
    rw [hc] at hs
    simp only [List.mem_singleton] at hs
    subst hs
    exact hoff (control_jump hc).1
  | branch o =>
    rw [hc] at hs
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hs
    rcases hs with rfl | rfl
    · exact hoff (control_branch hc).1
    · exact hnext rfl (not_unconditional (Or.inr ⟨o, hc⟩))
  | exit => rw [hc] at hs; simp at hs

/-! ## The analysis' fixpoint gives the certificate -/

/-- The invariant map of the locals analysis is a post-fixpoint: the entry's
pre-state is at least the initial state, and every analyzed block executes
from its pre-state to a post-state that is at most the pre-state of each
successor, which is analyzed too. -/
structure PostFixpoint (cfg : Cfg) (code : Array Bytecode) (initial : LocalsState)
    (inv : List (Nat × LocalsState)) : Prop where
  entry : ∃ pre, inv.lookup 0 = some pre ∧ StateLe initial pre
  closed : ∀ b pre, inv.lookup b = some pre → ∃ post,
    executeBlock localsAnalysis cfg code b pre = .ok post ∧
    ∀ s ∈ cfg.successors b, ∃ pre', inv.lookup s = some pre' ∧ StateLe post pre'

/-- The availability certificate of one function: the analysis' state before
each instruction, `maybe` where there is none. -/
def availCert (code : Array Bytecode) (inv : List (Nat × LocalsState)) (pc : Nat) (i : Nat) :
    Availability :=
  match availBefore code inv pc with
  | some st => stateAvail st i
  | none => .maybe

theorem walk_induct {succ : Nat → List Nat} {Q : Nat → Prop} {a c : Nat} {p : List Nat}
    (hw : Walk succ a c p) (ha : Q a) (hstep : ∀ x y, Q x → y ∈ succ x → Q y) : Q c := by
  induction hw with
  | nil => exact ha
  | cons hab _ ih => exact ih (hstep _ _ ha hab)

section Fixpoint

variable {cfg : Cfg} {code : Array Bytecode} {initial : LocalsState}
  {inv : List (Nat × LocalsState)}

/-- Inside an analyzed block, the state before each instruction executes it;
inside the block the result is the state before the next instruction, and at
the block's end it is at most the pre-state of each successor block. -/
theorem avail_at (hcfg : cfg.blocks = buildBlocks code) (hfix : PostFixpoint cfg code initial inv)
    {b : BasicBlock} (hb : b ∈ buildBlocks code) {pre : LocalsState}
    (hpre : inv.lookup b.start = some pre) {pc : Nat} (h1 : b.start ≤ pc) (h2 : pc ≤ b.exit) :
    ∃ st st' instr, code[pc]? = some instr ∧ availBefore code inv pc = some st ∧
      st.execute instr pc 0 = .ok st' ∧
      (pc < b.exit → availBefore code inv (pc + 1) = some st') ∧
      (pc = b.exit → ∀ s ∈ b.successors, ∃ pre', inv.lookup s = some pre' ∧ StateLe st' pre') := by
  have hok := (buildBlocks_spec code).1 b hb
  obtain ⟨post, hex, hsucc⟩ := hfix.closed b.start pre hpre
  rw [executeBlock_eq hcfg hb] at hex
  rw [cfg_successors hcfg hb] at hsucc
  have hsplit : b.exit + 1 - b.start = (pc - b.start + 1) + (b.exit - pc) := by omega
  rw [hsplit] at hex
  obtain ⟨stj, hj⟩ := foldlM_prefix hex
  obtain ⟨st0, h0, hstep⟩ := foldlM_last hj
  have hbefore := availBlock_prefix hok hpre (pc - b.start) st0 (by omega) h0 (by omega)
  rw [show b.start + (pc - b.start) = pc by omega] at hbefore hstep
  obtain ⟨instr, hi, hexe⟩ := blockStep_ok hstep
  refine ⟨st0, stj, instr, hi, hbefore, hexe, fun hlt => ?_, fun heq => ?_⟩
  · have := availBlock_prefix hok hpre (pc - b.start + 1) stj (by omega) hj (by omega)
    rwa [show b.start + (pc - b.start + 1) = pc + 1 by omega] at this
  · have hlast : b.exit - pc = 0 := by omega
    rw [hlast, Nat.add_zero] at hex
    rw [hj, Except.ok.injEq] at hex
    subst hex
    exact hsucc

/-- Every reachable instruction is in an analyzed block. -/
theorem analyzed_at (hcfg : cfg.blocks = buildBlocks code) (hfix : PostFixpoint cfg code initial inv)
    (hne : 0 < code.size) (hlt : ∀ pc, ∀ s ∈ successors code pc, s < code.size) {pc : Nat}
    (hreach : CodeReachable code pc) :
    ∃ b ∈ buildBlocks code, b.start ≤ pc ∧ pc ≤ b.exit ∧ ∃ pre, inv.lookup b.start = some pre := by
  obtain ⟨p, hw⟩ := hreach
  refine walk_induct (Q := fun pc => ∃ b ∈ buildBlocks code, b.start ≤ pc ∧ pc ≤ b.exit ∧
    ∃ pre, inv.lookup b.start = some pre) hw ?_ ?_
  · obtain ⟨b, hb, hs⟩ := block_of_mem (zero_mem_blockIds code) hne
    obtain ⟨pre, hpre, -⟩ := hfix.entry
    have hok := (buildBlocks_spec code).1 b hb
    exact ⟨b, hb, by omega, by omega, pre, hs ▸ hpre⟩
  · intro x y ⟨b, hb, h1, h2, pre, hpre⟩ hy
    have hok := (buildBlocks_spec code).1 b hb
    have hexit := hok.exit_lt
    have hylt := hlt x y hy
    obtain ⟨st, st', instr, hi, -, -, hin, hend⟩ := avail_at hcfg hfix hb hpre h1 h2
    rcases Nat.lt_or_ge x b.exit with hx | hx
    · -- Inside the block the only successor is the next instruction.
      have hnb : isBranch instr = false := by
        cases hbr : isBranch instr
        · rfl
        · rcases branch_ends hi hbr with h | h
          · omega
          · exact absurd h (hok.inner (x + 1) (by omega) (by omega))
      simp only [successors, hi, not_branch_fallThrough hnb, List.mem_singleton] at hy
      subst hy
      exact ⟨b, hb, by omega, by omega, pre, hpre⟩
    · have hxe : x = b.exit := by omega
      subst hxe
      have hys : y ∈ b.successors := by
        rw [hok.successors]; exact mem_getSuccessors hi hy hylt
      obtain ⟨pre', hpre', -⟩ := hend rfl y hys
      have hmem : y ∈ blockIds code := by
        have hend' := hok.exit_end
        simp only [isEndOfBlock, Bool.or_eq_true, beq_iff_eq, List.contains_iff_mem] at hend'
        simp only [successors, hi] at hy
        cases hc : instr.control with
        | fallThrough =>
          rw [hc] at hy
          simp only [List.mem_singleton] at hy
          subst hy
          rcases hend' with h | h
          · omega
          · exact h
        | jump o =>
          rw [hc] at hy
          simp only [List.mem_singleton] at hy
          subst hy
          exact offset_mem hi (control_jump hc).1
        | branch o =>
          rw [hc] at hy
          simp only [List.mem_cons, List.not_mem_nil, or_false] at hy
          rcases hy with rfl | rfl
          · exact offset_mem hi (control_branch hc).1
          · rcases hend' with h | h
            · omega
            · exact h
        | exit => rw [hc] at hy; simp at hy
      obtain ⟨b', hb', hs'⟩ := block_of_mem hmem hylt
      have hok' := (buildBlocks_spec code).1 b' hb'
      have := hok'.start_le_exit
      exact ⟨b', hb', by omega, by omega, pre', hs' ▸ hpre'⟩

/-- Inside a block the only successor is the next instruction. -/
theorem inner_successor {b : BasicBlock} (hok : BlockOk code b) {x y : Nat} {instr : Bytecode}
    (h1 : b.start ≤ x) (hx : x < b.exit) (hi : code[x]? = some instr)
    (hy : y ∈ successors code x) : y = x + 1 := by
  have hexit := hok.exit_lt
  have hnb : isBranch instr = false := by
    cases hbr : isBranch instr
    · rfl
    · rcases branch_ends hi hbr with h | h
      · omega
      · exact absurd h (hok.inner (x + 1) (by omega) (by omega))
  simpa [successors, hi, not_branch_fallThrough hnb] using hy

/-- A successor of a block's end in the code starts a block. -/
theorem exit_successor_mem {b : BasicBlock} (hok : BlockOk code b) {instr : Bytecode} {y : Nat}
    (hi : code[b.exit]? = some instr) (hy : y ∈ successors code b.exit) (hylt : y < code.size) :
    y ∈ blockIds code := by
  have hend' := hok.exit_end
  simp only [isEndOfBlock, Bool.or_eq_true, beq_iff_eq, List.contains_iff_mem] at hend'
  simp only [successors, hi] at hy
  cases hc : instr.control with
  | fallThrough =>
    rw [hc] at hy
    simp only [List.mem_singleton] at hy
    subst hy
    rcases hend' with h | h
    · omega
    · exact h
  | jump o =>
    rw [hc] at hy
    simp only [List.mem_singleton] at hy
    subst hy
    exact offset_mem hi (control_jump hc).1
  | branch o =>
    rw [hc] at hy
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hy
    rcases hy with rfl | rfl
    · exact offset_mem hi (control_branch hc).1
    · rcases hend' with h | h
      · omega
      · exact h
  | exit => rw [hc] at hy; simp at hy

/-- **C2** (static) for one function: the analysis' states form a consistent
availability certificate. -/
theorem availCert_consistent (hcfg : cfg.blocks = buildBlocks code)
    (hfix : PostFixpoint cfg code initial inv) (hne : 0 < code.size)
    (hlt : ∀ pc, ∀ s ∈ successors code pc, s < code.size) {np : Nat}
    (hinit : ∀ i, Availability.Refines (if i < np then .available else .unavailable)
      (stateAvail initial i)) :
    (∀ i, Availability.Refines (if i < np then .available else .unavailable)
      (availCert code inv 0 i)) ∧
    ∀ pc instr, CodeReachable code pc → code[pc]? = some instr →
      (∀ i, instr.readsLocal = some i → availCert code inv pc i = .available) ∧
      ∀ s ∈ successors code pc, ∀ i,
        Availability.Refines (availabilityAfter instr (availCert code inv pc) i)
          (availCert code inv s i) := by
  refine ⟨fun i => ?_, fun pc instr hreach hinstr => ?_⟩
  · obtain ⟨pre, hpre, hle⟩ := hfix.entry
    simp only [availCert, availBefore, hpre]
    exact Refines.trans (hinit i) (hle.2 i)
  · obtain ⟨b, hb, h1, h2, pre, hpre⟩ := analyzed_at hcfg hfix hne hlt hreach
    have hok := (buildBlocks_spec code).1 b hb
    obtain ⟨st, st', instr', hi, hbefore, hexe, hnext, hend⟩ := avail_at hcfg hfix hb hpre h1 h2
    rw [hinstr, Option.some.injEq] at hi
    subst hi
    obtain ⟨-, htrans, hreads⟩ := execute_ok hexe
    have hcert : availCert code inv pc = stateAvail st := by
      funext i; simp [availCert, hbefore]
    refine ⟨fun i hr => by rw [hcert]; exact hreads i hr, fun s hs i => ?_⟩
    rw [hcert, ← htrans]
    rcases Nat.lt_or_ge pc b.exit with hx | hx
    · rw [inner_successor hok h1 hx hinstr hs]
      simp only [availCert, hnext hx]
      exact Refines.refl _
    · have hxe : pc = b.exit := by omega
      subst hxe
      have hslt := hlt _ s hs
      have hys : s ∈ b.successors := by
        rw [hok.successors]; exact mem_getSuccessors hinstr hs hslt
      obtain ⟨pre', hpre', hle⟩ := hend rfl s hys
      simp only [availCert, availBefore_of_mem (exit_successor_mem hok hinstr hs hslt), hpre']
      exact hle.2 i

end Fixpoint

/-! ## The model's availability certificate -/

/-- The abstract interpreter computes a post-fixpoint whenever it succeeds on
a control-flow graph built from the code. -/
def SchedulerSound : Prop :=
  ∀ (code : Array Bytecode) (cfg : Cfg) (initial : LocalsState) (height : Nat)
    (inv : List (Nat × LocalsState)), Cfg.build code = .ok cfg →
    analyzeFunction localsAnalysis cfg code initial height = .ok inv →
    PostFixpoint cfg code initial inv

/-- The model's availability certificate: what the locals-safety pass
computes for each function with code. -/
def modelAvail (P : Program) : AvailabilityCertificate := fun f pc i =>
  match P.function? f with
  | none => .maybe
  | some info =>
    match info.definition.code with
    | none => .maybe
    | some code =>
      match Cfg.build code.code with
      | .error _ => .maybe
      | .ok cfg =>
        match FunctionView.function info.module f.index code info.handle cfg with
        | .error _ => .maybe
        | .ok view =>
          match verifyLocalsSafety info.module view with
          | .error _ => .maybe
          | .ok inv => availCert code.code inv pc i

theorem verifyControlFlow_build {config : VerifierConfig} {version : Nat} {c : CodeUnit}
    {cfg : Cfg} (h : verifyControlFlow config version c = .ok cfg) : Cfg.build c.code = .ok cfg := by
  unfold verifyControlFlow at h
  by_cases hv : version ≤ 5
  · simp [hv, panicAt, bind, Except.bind] at h
  · simp only [hv, if_false, bind_ok] at h
    obtain ⟨_, _, cfg', hb, _, _, h⟩ := h
    simp [pure, Except.pure] at h
    subst h
    exact hb

theorem initial_refines {m : CompiledModule} {view : FunctionView} {initial : LocalsState}
    (h : LocalsState.new m view = .ok initial) (i : Nat) :
    Availability.Refines (if i < view.parameters.length then .available else .unavailable)
      (stateAvail initial i) := by
  unfold LocalsState.new at h
  simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at h
  obtain ⟨abs, -, rfl⟩ := h
  simp only [stateAvail, List.getElem?_toArray, List.getElem?_map]
  by_cases hn : i < view.parameters.length + view.locals.length
  · have hr : (List.range (view.parameters.length + view.locals.length))[i]? = some i := by
      simp [hn]
    rw [hr]
    by_cases hp : i < view.parameters.length <;> simp [hp, toAvail, Refines.refl]
  · have hr : (List.range (view.parameters.length + view.locals.length))[i]? = none := by
      simp; omega
    rw [hr]
    exact Or.inr rfl

/-- **C2** (static): the model's availability certificate is consistent,
given that the scheduler computes a post-fixpoint. -/
theorem modelAvail_consistent (hsched : SchedulerSound) {P : Program}
    (hwf : WellFormedBy verifier VerifierConfig.production P) :
    (modelAvail P).Consistent P := by
  intro f info code hf hcode
  obtain ⟨hhandle, ⟨cfg, view, hcf, hview, -, -, inv, hls⟩⟩ := function_verified hwf hf hcode
  obtain ⟨hvc, hvh, hvcfg, hps, -, -, -⟩ := functionView_fields hview
  have hbuild := verifyControlFlow_build hcf
  have hblocks := verifyControlFlow_blocks hcf
  unfold verifyLocalsSafety at hls
  simp only [bind_ok] at hls
  obtain ⟨initial, hinit, hana⟩ := hls
  rw [hvcfg, hvc] at hana
  have hfix := hsched code.code cfg initial _ inv hbuild hana
  have hne := nonempty_of_verified ⟨hhandle, cfg, view, hcf, hview, by
    obtain ⟨_, ⟨cfg', view', hcf', hview', hsu, hts, inv', hls'⟩⟩ := function_verified hwf hf hcode
    rw [hcf, Except.ok.injEq] at hcf'
    subst hcf'
    rw [hview, Except.ok.injEq] at hview'
    subst hview'
    exact ⟨hsu, hts, inv', hls'⟩⟩
  have hfall := verifyControlFlow_fallthrough hcf
  have hbounds := (verifyModule_ok (wellFormed_module hwf (function?_parts hf).1).1).1
  have hlt : ∀ pc, ∀ s ∈ successors code.code pc, s < code.code.size :=
    successors_lt hfall (fun pc o i hi ho => branch_targets hbounds
      (getElem?_mem_toList (function?_parts hf).2.2.1) hcode hi ho)
  have hcert : ∀ pc i, modelAvail P f pc i = availCert code.code inv pc i := by
    intro pc i
    have hls' : verifyLocalsSafety info.module view = .ok inv := by
      unfold verifyLocalsSafety
      simp only [bind_ok]
      exact ⟨initial, hinit, by rw [hvcfg, hvc]; exact hana⟩
    simp only [modelAvail, hf, hcode, hbuild, hview, hls']
  obtain ⟨hentry, hsteps⟩ := availCert_consistent hblocks hfix hne hlt (initial_refines hinit)
  refine ⟨view.parameters, hps, fun i => ?_, fun pc instr hreach hinstr => ?_⟩
  · rw [hcert]; exact hentry i
  · obtain ⟨hr, hs⟩ := hsteps pc instr hreach hinstr
    refine ⟨fun i hi => by rw [hcert]; exact hr i hi, fun s hs' i => ?_⟩
    have hfun : modelAvail P f pc = availCert code.code inv pc := by
      funext j; exact hcert pc j
    rw [hfun, hcert]
    exact hs s hs' i

theorem initial_size {m : CompiledModule} {view : FunctionView} {initial : LocalsState}
    (h : LocalsState.new m view = .ok initial) :
    initial.states.size = view.parameters.length + view.locals.length := by
  unfold LocalsState.new at h
  simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at h
  obtain ⟨abs, -, rfl⟩ := h
  simp

/-! ## Invariants of the abstract interpreter -/

theorem lookup_mem {S : Type} : ∀ {l : List (Nat × S)} {k : Nat} {v : S},
    l.lookup k = some v → (k, v) ∈ l
  | [], _, _, h => by simp at h
  | (k', v') :: rest, k, v, h => by
    simp only [List.lookup] at h
    split at h
    · next hk =>
      simp only [beq_iff_eq] at hk
      simp only [Option.some.injEq] at h
      subst hk h
      exact List.mem_cons_self ..
    · exact List.mem_cons_of_mem _ (lookup_mem h)

section Preserve

variable {S : Type} {a : Analysis S} {cfg : Cfg} {code : Array Bytecode} {Q : S → Prop}

/-- A property of abstract states that the join and the transfer function
keep. -/
structure Preserves (a : Analysis S) (Q : S → Prop) : Prop where
  join : ∀ old new j c, Q old → Q new → a.join old new = .ok (j, c) → Q j
  execute : ∀ st instr o e st', Q st → a.execute st instr o e = .ok st' → Q st'

theorem executeBlock_preserves (hp : Preserves a Q) {b : Nat} {pre post : S} (hq : Q pre)
    (h : executeBlock a cfg code b pre = .ok post) : Q post := by
  unfold executeBlock at h
  refine foldlM_inv Q hq (fun st offset r _ hst hr => ?_) h
  simp only [bind_ok] at hr
  obtain ⟨instr, -, hr⟩ := hr
  exact hp.execute _ _ _ _ _ hst hr

theorem propagate_preserves (hp : Preserves a Q) {b : Nat} {post : S} (hpost : Q post)
    {acc acc' : List (Nat × S) × List Nat} {succ : Nat} (hacc : ∀ e ∈ acc.1, Q e.2)
    (h : propagate a cfg b post acc succ = .ok acc') : ∀ e ∈ acc'.1, Q e.2 := by
  obtain ⟨inv, cands⟩ := acc
  unfold propagate at h
  simp only at h
  split at h
  · simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    intro e he
    rcases List.mem_append.1 he with he | he
    · exact hacc e he
    · simp only [List.mem_singleton] at he
      subst he
      exact hpost
  · next old hold =>
    simp only [bind_ok] at h
    obtain ⟨⟨joined, changed⟩, hj, h⟩ := h
    have hqj := hp.join _ _ _ _ (hacc _ (lookup_mem hold)) hpost hj
    simp only at h
    split at h
    · simp only [pure, Except.pure, Except.ok.injEq] at h
      subst h
      intro e he
      simp only [setPre, List.mem_map] at he
      obtain ⟨e', he', rfl⟩ := he
      split
      · exact hqj
      · exact hacc e' he'
    · simp only [pure, Except.pure, Except.ok.injEq] at h
      subst h
      exact hacc

theorem analyzeLoop_preserves (hp : Preserves a Q) :
    ∀ (fuel : Nat) (inv : List (Nat × S)) (b : Option Nat) {r : List (Nat × S)},
      (∀ e ∈ inv, Q e.2) → analyzeLoop a cfg code fuel inv b = .ok r → ∀ e ∈ r, Q e.2
  | _, inv, none, r, hinv, h => by
    simp only [analyzeLoop, pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact hinv
  | 0, _, some _, _, _, h => by simp [analyzeLoop, outOfFuel] at h
  | fuel + 1, inv, some b, r, hinv, h => by
    simp only [analyzeLoop] at h
    split at h
    · exact analyzeLoop_preserves hp fuel inv _ hinv h
    · next pre hpre =>
      simp only [bind_ok] at h
      obtain ⟨post, hpost, ⟨inv', cands'⟩, hfold, h⟩ := h
      have hqpost := executeBlock_preserves hp (hinv _ (lookup_mem hpre)) hpost
      have hinv' : ∀ e ∈ inv', Q e.2 :=
        foldlM_inv (fun acc : List (Nat × S) × List Nat => ∀ e ∈ acc.1, Q e.2) hinv
          (fun acc s acc' _ hacc hr => propagate_preserves hp hqpost hacc hr) hfold
      exact analyzeLoop_preserves hp fuel inv' _ hinv' h

/-- Every state the abstract interpreter records has a property that the
initial state has and that the join and the transfer function keep. -/
theorem analyzeFunction_preserves (hp : Preserves a Q) {initial : S} {height : Nat}
    {inv : List (Nat × S)} (hq : Q initial)
    (h : analyzeFunction a cfg code initial height = .ok inv) : ∀ e ∈ inv, Q e.2 :=
  analyzeLoop_preserves hp _ _ _ (fun e he => by
    simp only [List.mem_singleton] at he
    subst he
    exact hq) h

end Preserve

/-! ## Locals that may be discarded have `drop` -/

theorem join_abilities {a b j : LocalsState} {c : Bool} (h : a.join b = .ok (j, c)) :
    j.abilities = a.abilities := by
  unfold LocalsState.join at h
  simp only at h
  split at h
  · simp [panicAt] at h
  · split at h <;> simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h <;>
      rw [← h.1]

theorem execute_abilities {st st' : LocalsState} {instr : Bytecode} {o e : Nat}
    (h : st.execute instr o e = .ok st') : st'.abilities = st.abilities := by
  cases instr
  case stLoc i =>
    simp only [LocalsState.execute, bind_ok] at h
    obtain ⟨s, -, h⟩ := h
    cases s
    · simp only [pure, Except.pure, Except.ok.injEq] at h
      rw [← h]
    all_goals
      simp only [bind_ok] at h
      obtain ⟨ab, -, h⟩ := h
      split at h
      · simp [reject] at h
      · simp only [pure, Except.pure, Except.ok.injEq] at h
        rw [← h]
  case moveLoc i =>
    simp only [LocalsState.execute, bind_ok] at h
    obtain ⟨s, -, h⟩ := h
    cases s <;> simp only [reject, pure, Except.pure, Except.ok.injEq, reduceCtorEq] at h
    rw [← h]
  case copyLoc i | mutBorrowLoc i | immBorrowLoc i =>
    simp only [LocalsState.execute, bind_ok] at h
    obtain ⟨s, -, h⟩ := h
    cases s <;> simp only [reject, pure, Except.pure, Except.ok.injEq, reduceCtorEq] at h
    rw [← h]
  case ret =>
    simp only [LocalsState.execute] at h
    split at h
    · simp [panicAt, bind, Except.bind] at h
    · simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at h
      obtain ⟨_, -, rfl⟩ := h
      rfl
  all_goals
    simp only [LocalsState.execute, pure, Except.pure, Except.ok.injEq] at h
    rw [← h]

/-- The locals analysis keeps each local's abilities. -/
theorem localsPreserves (A : Array AbilitySet) :
    Preserves localsAnalysis (fun st : LocalsState => st.abilities = A) :=
  ⟨fun _ _ _ _ hold _ hj => (join_abilities hj).trans hold,
    fun _ _ o e _ hst h => (execute_abilities (o := o) (e := e) h).trans hst⟩

theorem availBefore_abilities {code : Array Bytecode} {inv : List (Nat × LocalsState)}
    {A : Array AbilitySet} (hinv : ∀ e ∈ inv, e.2.abilities = A) :
    ∀ pc st, availBefore code inv pc = some st → st.abilities = A
  | 0, st, h => hinv _ (lookup_mem h)
  | pc + 1, st, h => by
    simp only [availBefore] at h
    split at h
    · exact hinv _ (lookup_mem h)
    · simp only [Option.bind_eq_some_iff] at h
      obtain ⟨st0, h0, h⟩ := h
      split at h
      · next instr _ =>
        cases hex : st0.execute instr pc 0 with
        | error => simp [hex, Except.toOption] at h
        | ok st1 =>
          simp only [hex, Except.toOption, Option.some.injEq] at h
          subst h
          exact (execute_abilities hex).trans (availBefore_abilities hinv pc st0 h0)
      · simp at h

theorem initial_abilities {m : CompiledModule} {view : FunctionView} {initial : LocalsState}
    (h : LocalsState.new m view = .ok initial) :
    (view.parameters ++ view.locals).mapM (Model.abilities m view.handle.typeParameters) =
      .ok initial.abilities.toList := by
  unfold LocalsState.new at h
  simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at h
  obtain ⟨abs, habs, rfl⟩ := h
  simpa using habs

theorem mapM_ok_getElem {α β ε : Type} {f : α → Except ε β} :
    ∀ {l : List α} {r : List β}, l.mapM f = .ok r →
      r.length = l.length ∧
        ∀ (i : Nat) (b : β), r[i]? = some b → ∃ a, l[i]? = some a ∧ f a = .ok b
  | [], r, h => by simp [pure, Except.pure] at h; subst h; simp
  | x :: xs, r, h => by
    simp only [List.mapM_cons, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨b, hb, bs, hbs, rfl⟩ := h
    obtain ⟨hlen, hget⟩ := mapM_ok_getElem hbs
    refine ⟨by simp [hlen], fun i c hc => ?_⟩
    cases i with
    | zero => simp at hc; subst hc; exact ⟨x, rfl, hb⟩
    | succ i => simpa using hget i c (by simpa using hc)

theorem forIn_ok_mem {α : Type} {l : List α} {f : α → PUnit → VResult (ForInStep PUnit)}
    {u : PUnit} (hy : ∀ x r, f x PUnit.unit = .ok r → r = .yield PUnit.unit)
    (h : forIn l PUnit.unit f = .ok u) : ∀ x ∈ l, ∃ r, f x PUnit.unit = .ok r := by
  induction l with
  | nil => simp
  | cons y ys ih =>
    cases hf : f y PUnit.unit with
    | error err => simp [List.forIn_cons, hf, bind, Except.bind] at h
    | ok r =>
    rw [hy y r hf] at hf
    simp only [List.forIn_cons, hf, bind, Except.bind] at h
    intro x hx
    rcases List.mem_cons.1 hx with rfl | hx
    · exact ⟨_, hf⟩
    · exact ih h x hx

/-- `StLoc` and `Ret` succeed only if each local they may discard has `drop`
in the analysis' abilities. -/
theorem execute_drop {st st' : LocalsState} {instr : Bytecode} {o e i : Nat}
    (h : st.execute instr o e = .ok st') (hi : instr = .ret ∨ instr = .stLoc i)
    (hlt : instr = .ret → i < st.abilities.size) (hav : stateAvail st i ≠ .unavailable) :
    ∃ a, st.abilities[i]? = some a ∧ a.drop = true := by
  rcases hi with rfl | rfl
  · simp only [LocalsState.execute] at h
    split at h
    · simp [panicAt, bind, Except.bind] at h
    · next hsize =>
      simp only [bne_iff_ne, ne_eq, Decidable.not_not] at hsize
      have hlt' := hlt rfl
      obtain ⟨s, hs⟩ : ∃ s, st.states[i]? = some s :=
        ⟨_, Array.getElem?_eq_getElem (by omega)⟩
      have hsu : s ≠ .unavailable := by
        rintro rfl; simp [stateAvail, hs, toAvail] at hav
      simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at h
      obtain ⟨u, hu, -⟩ := h
      obtain ⟨a, ha⟩ : ∃ a, st.abilities[i]? = some a := ⟨_, Array.getElem?_eq_getElem hlt'⟩
      have hmem : (s, a) ∈ st.states.toList.zip st.abilities.toList := by
        rw [List.mem_iff_getElem?]
        exact ⟨i, by simp [List.getElem?_zip_eq_some, hs, ha]⟩
      refine ⟨a, ha, ?_⟩
      obtain ⟨r, hr⟩ := forIn_ok_mem (fun x r hr => by
        split at hr <;> (try split at hr) <;>
          simp_all [reject, bind, Except.bind]) hu (s, a) hmem
      cases s
      · exact absurd rfl hsu
      all_goals
        simp only at hr
        split at hr
        · simp [reject, bind, Except.bind] at hr
        · next hd => simpa using hd
  · simp only [LocalsState.execute, bind_ok] at h
    obtain ⟨s, hs, h⟩ := h
    rw [lookup_ok] at hs
    have hsu : s ≠ .unavailable := by
      rintro rfl; simp [stateAvail, hs, toAvail] at hav
    cases s
    · exact absurd rfl hsu
    all_goals
      simp only [bind_ok] at h
      obtain ⟨ab, hab, h⟩ := h
      rw [lookup_ok] at hab
      split at h
      · simp [reject] at h
      · next hd => exact ⟨ab, hab, by simpa using hd⟩

end MoveBytecode.Proofs
