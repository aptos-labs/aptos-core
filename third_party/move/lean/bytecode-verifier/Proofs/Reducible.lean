-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.LoopSummary
import Proofs.TypeFormation

/-!
# L3: reducibility

UNTRUSTED (checked by the kernel). If the model accepts a function's control
flow, the specification's instruction graph is reducible: no cycle of
forward edges passes through a reachable instruction.

A ranking decreases along every forward edge from a reachable instruction:
the finishing order of the loop summary's search for the instruction's
block, then the instruction's distance to the end of the code. Inside a
block, control falls through to the next instruction. Across blocks, an edge
the search did not record as a back edge leads to a block that finished
earlier (`summary_facts`); and a back edge's target dominates its source
(`back_dominated`), so it is not a forward edge. Dominance carries over from
the summary's nodes to instructions because a walk enters a block only at
its first instruction.

Finally (`localPropertiesNonGeneric_of_phantom`), the model meets every per-module
property of the specification, given the phantom-position rule for field
types that mention a reference (`PhantomUnderReferences`), which its
acceptance does not establish.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Walks -/

theorem walk_head {succ : Nat → List Nat} {a c : Nat} {p : List Nat} (h : Walk succ a c p) :
    a ∈ p := by
  cases h <;> simp

/-- A property kept along edges holds along walks. -/
theorem walk_preserve {succ : Nat → List Nat} {P : Nat → Prop}
    (step : ∀ a b, P a → b ∈ succ a → P b) :
    ∀ {a c : Nat} {p : List Nat}, Walk succ a c p → P a → P c
  | _, _, _, .nil _, h => h
  | _, _, _, .cons hab w, h => walk_preserve step w (step _ _ h hab)

/-- A property kept along the edges that avoid `x` holds along a walk that
avoids `x`. -/
theorem walk_avoid {succ : Nat → List Nat} {P : Nat → Prop} {x : Nat}
    (step : ∀ a b, P a → b ∈ succ a → b ≠ x → P b) :
    ∀ {a c : Nat} {p : List Nat}, Walk succ a c p → x ∉ p → P a → P c
  | _, _, _, .nil _, _, h => h
  | _, _, _, .cons hab w, hx, h =>
    walk_avoid step w (fun h' => hx (List.mem_cons_of_mem _ h'))
      (step _ _ h hab fun hb => hx (List.mem_cons_of_mem _ (hb ▸ walk_head w)))

/-- **Reducibility from a ranking** that decreases along every forward edge
from a reachable node. -/
theorem reducible_of_rank {succ : Nat → List Nat} (r : Nat → Nat)
    (h : ∀ a b, (∃ p, Walk succ 0 a p) → ForwardEdge succ a b → r b < r a) :
    Reducible succ := by
  intro n hn hplus
  have key : ∀ a c, Plus (ForwardEdge succ) a c → (∃ p, Walk succ 0 a p) →
      r c < r a ∧ ∃ p, Walk succ 0 c p := by
    intro a c hp
    induction hp with
    | single hfe =>
      rintro ⟨p, hw⟩
      exact ⟨h _ _ ⟨p, hw⟩ hfe, _, walk_snoc hw hfe.1⟩
    | tail _ hbc ih =>
      intro ha
      obtain ⟨h1, p, hw⟩ := ih ha
      exact ⟨Nat.lt_trans (h _ _ ⟨p, hw⟩ hbc) h1, _, walk_snoc hw hbc.1⟩
  exact Nat.lt_irrefl _ (key n n hplus hn).1

theorem rwalk_snoc {s : LoopSummary} :
    ∀ {a b : Nat} {p : List Nat}, RWalk s a b p → ∀ {c : Nat}, Recorded s b c →
      RWalk s a c (p ++ [c])
  | _, _, _, .nil _, _, hc => RWalk.cons hc (RWalk.nil _)
  | _, _, _, .cons hab w, _, hc => RWalk.cons hab (rwalk_snoc w hc)

/-! ## Blocks -/

section Blocks

variable {code : Array Bytecode}

/-- Two blocks holding the same instruction are the same block. -/
theorem block_eq {b b' : BasicBlock} (hb : BlockOk code b) (hb' : BlockOk code b') {pc : Nat}
    (h1 : b.start ≤ pc) (h2 : pc ≤ b.exit) (h1' : b'.start ≤ pc) (h2' : pc ≤ b'.exit) :
    b = b' := by
  have hs : b.start = b'.start := by
    rcases Nat.lt_trichotomy b.start b'.start with h | h | h
    · exact absurd hb'.start_mem (hb.inner _ h (by omega))
    · exact h
    · exact absurd hb.start_mem (hb'.inner _ h (by omega))
  have he : b.exit = b'.exit := by
    rcases Nat.lt_trichotomy b.exit b'.exit with h | h | h
    · rcases (isEndOfBlock_iff code b.exit).1 hb.exit_end with h' | h'
      · have := hb'.exit_lt; omega
      · exact absurd h' (hb'.inner _ (by omega) (by omega))
    · exact h
    · rcases (isEndOfBlock_iff code b'.exit).1 hb'.exit_end with h' | h'
      · have := hb.exit_lt; omega
      · exact absurd h' (hb.inner _ (by omega) (by omega))
  have hsucc := hb.successors
  have hsucc' := hb'.successors
  cases b
  cases b'
  simp only at hs he hsucc hsucc'
  subst hs he
  rw [hsucc, hsucc']

/-- A block id inside the code starts a block. -/
theorem blockId_start {c : Nat} (hc : c < code.size) (hid : c ∈ blockIds code) :
    ∃ b ∈ buildBlocks code, b.start = c := by
  obtain ⟨b, hb, h1, h2⟩ := (buildBlocks_spec code).2 c hc
  refine ⟨b, hb, ?_⟩
  rcases Nat.lt_or_ge b.start c with h | h
  · exact absurd hid (((buildBlocks_spec code).1 b hb).inner c h h2)
  · omega

/-- The model's successors of a block are those of its own record. -/
theorem blockSuccessors_eq {b : BasicBlock} (hb : b ∈ buildBlocks code) :
    blockSuccessors (buildBlocks code) b.start = b.successors := by
  unfold blockSuccessors
  cases hf : (buildBlocks code).find? (·.start == b.start) with
  | none =>
    rw [List.find?_eq_none] at hf
    exact absurd (by simp) (hf b hb)
  | some bb =>
    have hbb := List.mem_of_find?_eq_some hf
    have hst := List.find?_some hf
    simp only [beq_iff_eq] at hst
    have hok := (buildBlocks_spec code).1
    have := block_eq (hok bb hbb) (hok b hb) (pc := b.start) (by omega)
      (by have := (hok bb hbb).start_le_exit; omega) (Nat.le_refl _) (hok b hb).start_le_exit
    rw [this]

theorem sortPair_mem {v : List Nat} {c : Nat} :
    c ∈ (match v with
      | [a, b] => if a > b then [b, a] else v
      | _ => v) ↔ c ∈ v := by
  split
  · split <;> simp [or_comm]
  · rfl

theorem unconditional_of_control {i : Bytecode}
    (h : i.control = .exit ∨ ∃ o, i.control = .jump o) : isUnconditionalBranch i = true := by
  rcases h with h | ⟨o, h⟩ <;> cases i <;> simp_all [Bytecode.control, isUnconditionalBranch]

variable (hfall : ∃ last, code.back? = some last ∧ isUnconditionalBranch last = true)
  (hbr : ∀ (pc o : Nat) (i : Bytecode), code[pc]? = some i → offset i = some o → o < code.size)

include hfall in
/-- The model's successors of an instruction are the specification's. -/
theorem getSuccessors_mem {pc c : Nat} : c ∈ getSuccessors pc code ↔ c ∈ successors code pc := by
  cases hi : code[pc]? with
  | none => simp [getSuccessors, successors, hi]
  | some i =>
    cases hc : i.control with
    | fallThrough =>
      obtain ⟨ho, -⟩ := control_fallThrough hc
      have hlt := next_lt hi hfall (Or.inl hc)
      have hu := not_unconditional (Or.inl hc)
      simp [getSuccessors, successors, hi, hc, ho, hu, Nat.not_le.2 hlt]
    | jump o =>
      obtain ⟨ho, -⟩ := control_jump hc
      have hu := unconditional_of_control (Or.inr ⟨o, hc⟩)
      simp [getSuccessors, successors, hi, hc, ho, hu]
    | branch o =>
      obtain ⟨ho, -⟩ := control_branch hc
      have hlt := next_lt hi hfall (Or.inr ⟨o, hc⟩)
      have hu := not_unconditional (Or.inr ⟨o, hc⟩)
      by_cases hop : o = pc + 1
      · subst hop
        simp [getSuccessors, successors, hi, hc, ho, hu, Nat.not_le.2 hlt]
      · have hop' : (pc + 1 == o) = false := by simp [Ne.symm hop]
        simp only [getSuccessors, successors, hi, hc, ho, hu, Nat.not_le.2 hlt, if_false,
          Bool.not_false, Bool.true_and, List.contains_cons, List.contains_nil, Bool.or_false,
          hop', if_true, List.singleton_append]
        split <;> simp [or_comm]
    | exit =>
      obtain ⟨ho, -⟩ := control_exit hc
      have hu := unconditional_of_control (Or.inl hc)
      simp [getSuccessors, successors, hi, hc, ho, hu]

/-- Inside a block, control falls through to the next instruction. -/
theorem inner_successors_eq {b : BasicBlock} (hb : BlockOk code b) {pc : Nat} (h1 : b.start ≤ pc)
    (h2 : pc < b.exit) : successors code pc = [pc + 1] := by
  have hpc : pc < code.size := Nat.lt_trans h2 hb.exit_lt
  obtain ⟨i, hi⟩ : ∃ i, code[pc]? = some i := ⟨code[pc], Array.getElem?_eq_getElem hpc⟩
  have hend := fun hbr => ends_at_exit hb h1 (Nat.le_of_lt h2) (branch_ends hi hbr)
  unfold successors
  rw [hi]
  cases hc : i.control with
  | fallThrough => simp [hc]
  | jump o => exact absurd (hend (control_jump hc).2) (Nat.ne_of_lt h2)
  | branch o => exact absurd (hend (control_branch hc).2) (Nat.ne_of_lt h2)
  | exit => exact absurd (hend (control_exit hc).2) (Nat.ne_of_lt h2)

include hfall hbr in
/-- A successor of a block's last instruction starts a block. -/
theorem exit_successor {b : BasicBlock} (hb : BlockOk code b) {s : Nat}
    (hs : s ∈ successors code b.exit) : ∃ c ∈ buildBlocks code, c.start = s := by
  have hslt := successors_lt hfall hbr b.exit s hs
  refine blockId_start hslt ?_
  obtain ⟨i, hi⟩ : ∃ i, code[b.exit]? = some i :=
    ⟨code[b.exit]'hb.exit_lt, Array.getElem?_eq_getElem hb.exit_lt⟩
  have hnext : b.exit + 1 < code.size → b.exit + 1 ∈ blockIds code := fun hlt => by
    rcases (isEndOfBlock_iff code b.exit).1 hb.exit_end with h | h
    · omega
    · exact h
  cases hc : i.control with
  | fallThrough =>
    simp only [successors, hi, hc, List.mem_singleton] at hs
    subst hs; exact hnext hslt
  | jump o =>
    simp only [successors, hi, hc, List.mem_singleton] at hs
    subst hs; exact offset_mem hi (control_jump hc).1
  | branch o =>
    simp only [successors, hi, hc, List.mem_cons, List.not_mem_nil, or_false] at hs
    rcases hs with rfl | rfl
    · exact offset_mem hi (control_branch hc).1
    · exact hnext hslt
  | exit => simp [successors, hi, hc] at hs

end Blocks

/-! ## The ranking -/

theorem rank_lt {x y m r r' : Nat} (hxy : x < y) (hr : r < m) : x * m + r < y * m + r' := by
  have : (x + 1) * m ≤ y * m := Nat.mul_le_mul_right m hxy
  rw [Nat.succ_mul] at this
  omega

theorem verifyReducibility_build {config : VerifierConfig} {cfg : Cfg}
    (h : verifyReducibility config cfg = .ok ()) : ∃ s, LoopSummary.build cfg = .ok s := by
  unfold verifyReducibility at h
  simp only [bind_ok] at h
  obtain ⟨s, hs, -⟩ := h
  exact ⟨s, hs⟩

theorem verifyControlFlow_parts {config : VerifierConfig} {version : Nat} {c : CodeUnit}
    {cfg : Cfg} (h : verifyControlFlow config version c = .ok cfg) :
    cfg.blocks = buildBlocks c.code ∧ verifyReducibility config cfg = .ok () := by
  refine ⟨verifyControlFlow_blocks h, ?_⟩
  unfold verifyControlFlow at h
  by_cases hv : version ≤ 5
  · simp [hv, panicAt, bind, Except.bind] at h
  · simp only [hv, if_false, bind_ok] at h
    obtain ⟨_, _, cfg', _, u, hr, h⟩ := h
    simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    cases u
    exact hr

/-- **L3 reducibility** for a code unit whose control flow the model
accepts, given the bounds check of its branch targets. -/
theorem reducible_of_verified {config : VerifierConfig} {version : Nat} {c : CodeUnit}
    {cfg : Cfg} (h : verifyControlFlow config version c = .ok cfg)
    (hbr : ∀ (pc o : Nat) (i : Bytecode), c.code[pc]? = some i → offset i = some o →
      o < c.code.size) :
    Reducible (successors c.code) := by
  have hfall := verifyControlFlow_fallthrough h
  obtain ⟨hblocks, hred⟩ := verifyControlFlow_parts h
  obtain ⟨s, hbuild⟩ := verifyReducibility_build hred
  have hok := (buildBlocks_spec c.code).1
  have hsize : 0 < c.code.size := by
    obtain ⟨last, hlast, -⟩ := hfall
    rcases Nat.eq_zero_or_pos c.code.size with h0 | h0
    · simp [Array.back?, h0] at hlast
    · exact h0
  -- The blocks' starts are closed under the model's successors.
  have hsucc : ∀ b ∈ buildBlocks c.code, cfg.successors b.start = b.successors := by
    intro b hb; simp only [Cfg.successors, hblocks]; exact blockSuccessors_eq hb
  have hS : ∀ b ∈ cfg.blocks.map (·.start), ∀ x ∈ cfg.successors b,
      x ∈ cfg.blocks.map (·.start) := by
    intro b hb x hx
    rw [hblocks] at hb ⊢
    obtain ⟨B, hB, rfl⟩ := List.mem_map.1 hb
    rw [hsucc B hB, (hok B hB).successors, getSuccessors_mem hfall] at hx
    obtain ⟨C, hC, hCs⟩ := exit_successor hfall hbr (hok B hB) hx
    exact List.mem_map.2 ⟨C, hC, hCs⟩
  obtain ⟨B0, hB0, hB0s, -⟩ := (buildBlocks_spec c.code).2 0 hsize
  have h0 : 0 ∈ cfg.blocks.map (·.start) := by
    rw [hblocks]; exact List.mem_map.2 ⟨B0, hB0, by omega⟩
  obtain ⟨node, μ, hnode0, hinj, hedge, hsok, hμ⟩ :=
    summary_facts hbuild (by simp) h0 hS
  have hdom := back_dominated hred hbuild hsok
  -- The block holding an instruction.
  let blockAt (pc : Nat) : Option BasicBlock :=
    cfg.blocks.find? fun B => decide (B.start ≤ pc) && decide (pc ≤ B.exit)
  have hblockAt : ∀ {B : BasicBlock} {pc : Nat}, B ∈ buildBlocks c.code → B.start ≤ pc →
      pc ≤ B.exit → blockAt pc = some B := by
    intro B pc hB h1 h2
    simp only [blockAt, hblocks]
    cases hf : (buildBlocks c.code).find?
        (fun B => decide (B.start ≤ pc) && decide (pc ≤ B.exit)) with
    | none =>
      rw [List.find?_eq_none] at hf
      exact absurd (by simp [h1, h2]) (hf B hB)
    | some B' =>
      have hB' := List.mem_of_find?_eq_some hf
      have hp := List.find?_some hf
      simp only [Bool.and_eq_true, decide_eq_true_eq] at hp
      rw [block_eq (hok B' hB') (hok B hB) hp.1 hp.2 h1 h2]
  let N := c.code.size + 1
  let rank (pc : Nat) : Nat :=
    match blockAt pc with
    | some B => (match node B.start with | some k => μ k | none => 0) * N + (c.code.size - pc)
    | none => 0
  -- Reachable instructions lie in explored blocks.
  let InBlock (pc : Nat) : Prop := pc < c.code.size ∧
    ∃ B ∈ buildBlocks c.code, B.start ≤ pc ∧ pc ≤ B.exit ∧ ∃ k, node B.start = some k
  have hstep : ∀ a b, InBlock a → b ∈ successors c.code a → InBlock b := by
    rintro a b ⟨ha, B, hB, h1, h2, k, hk⟩ hb
    refine ⟨successors_lt hfall hbr a b hb, ?_⟩
    rcases Nat.lt_or_ge a B.exit with hlt | hge
    · rw [inner_successors_eq (hok B hB) h1 hlt, List.mem_singleton] at hb
      subst hb
      exact ⟨B, hB, by omega, hlt, k, hk⟩
    · have hae : a = B.exit := by omega
      subst hae
      obtain ⟨C, hC, rfl⟩ := exit_successor hfall hbr (hok B hB) hb
      have hbs : C.start ∈ cfg.successors B.start := by
        rw [hsucc B hB, (hok B hB).successors, getSuccessors_mem hfall]; exact hb
      obtain ⟨kc, hkc, -⟩ := hedge _ _ hk _ hbs
      exact ⟨C, hC, Nat.le_refl _, (hok C hC).start_le_exit, kc, hkc⟩
  have hreach : ∀ pc p, Walk (successors c.code) 0 pc p → InBlock pc := fun pc p hw =>
    walk_preserve hstep hw ⟨hsize, B0, hB0, by omega, Nat.zero_le _, 0, by
      rw [show B0.start = 0 by omega]; exact hnode0⟩
  -- A back edge's target dominates its source.
  have hback : ∀ {B C : BasicBlock} {kB kC : Nat}, B ∈ buildBlocks c.code →
      node B.start = some kB → C ∈ buildBlocks c.code → node C.start = some kC →
      kB ∈ s.backEdges kC → Dominates (successors c.code) C.start B.exit := by
    intro B C kB kC hB hkB hC hkC hbk p hw
    refine Classical.byContradiction fun hp => ?_
    let P (pc : Nat) : Prop := InBlock pc ∧ ∃ D ∈ buildBlocks c.code, D.start ≤ pc ∧
      pc ≤ D.exit ∧ ∃ kD, node D.start = some kD ∧ ∃ path, RWalk s 0 kD path ∧ kC ∉ path
    have hP : ∀ a b, P a → b ∈ successors c.code a → b ≠ C.start → P b := by
      rintro a b ⟨hin, D, hD, h1, h2, kD, hkD, path, hpath, hkp⟩ hb hbC
      refine ⟨hstep a b hin hb, ?_⟩
      rcases Nat.lt_or_ge a D.exit with hlt | hge
      · rw [inner_successors_eq (hok D hD) h1 hlt, List.mem_singleton] at hb
        subst hb
        exact ⟨D, hD, by omega, hlt, kD, hkD, path, hpath, hkp⟩
      · have hae : a = D.exit := by omega
        subst hae
        obtain ⟨E, hE, rfl⟩ := exit_successor hfall hbr (hok D hD) hb
        have hbs : E.start ∈ cfg.successors D.start := by
          rw [hsucc D hD, (hok D hD).successors, getSuccessors_mem hfall]; exact hb
        obtain ⟨kE, hkE, hrec⟩ := hedge _ _ hkD _ hbs
        refine ⟨E, hE, Nat.le_refl _, (hok E hE).start_le_exit, kE, hkE, _, rwalk_snoc hpath hrec,
          fun hm => ?_⟩
        rcases List.mem_append.1 hm with hm | hm
        · exact hkp hm
        · simp only [List.mem_singleton] at hm
          subst hm
          exact hbC (hinj _ _ _ hkE hkC)
    have hC0 : kC ≠ 0 := by
      intro h0'
      subst h0'
      have := hinj _ _ _ hnode0 hkC
      rw [← this] at hp
      exact hp (walk_head hw)
    have hP0 : P 0 := ⟨⟨hsize, B0, hB0, by omega, Nat.zero_le _, 0, by
        rw [show B0.start = 0 by omega]; exact hnode0⟩,
      B0, hB0, by omega, Nat.zero_le _, 0, by rw [show B0.start = 0 by omega]; exact hnode0,
      [0], RWalk.nil 0, by simpa using hC0⟩
    obtain ⟨-, D, hD, h1, h2, kD, hkD, path, hpath, hkp⟩ := walk_avoid hP hw hp hP0
    have := block_eq (hok D hD) (hok B hB) h1 h2 (hok B hB).start_le_exit (Nat.le_refl _)
    subst this
    have hkk : kD = kB := by rw [hkB] at hkD; exact (Option.some.inj hkD).symm
    subst hkk
    exact hkp (hdom kC _ hbk path hpath)
  refine reducible_of_rank rank fun a b ⟨p, hw⟩ hfe => ?_
  obtain ⟨ha, B, hB, h1, h2, kB, hkB⟩ := hreach a p hw
  have hra : rank a = μ kB * N + (c.code.size - a) := by
    simp only [rank, hblockAt hB h1 h2, hkB]
  rcases Nat.lt_or_ge a B.exit with hlt | hge
  · have hb := hfe.1
    rw [inner_successors_eq (hok B hB) h1 hlt, List.mem_singleton] at hb
    subst hb
    have hrb : rank (a + 1) = μ kB * N + (c.code.size - (a + 1)) := by
      simp only [rank, hblockAt hB (by omega) hlt, hkB]
    rw [hra, hrb]
    have := hok B hB |>.exit_lt
    omega
  · have hae : a = B.exit := by omega
    subst hae
    obtain ⟨C, hC, rfl⟩ := exit_successor hfall hbr (hok B hB) hfe.1
    have hbs : C.start ∈ cfg.successors B.start := by
      rw [hsucc B hB, (hok B hB).successors, getSuccessors_mem hfall]; exact hfe.1
    obtain ⟨kC, hkC, hrec⟩ := hedge _ _ hkB _ hbs
    rcases hrec with hpred | hbk
    · have hrb : rank C.start = μ kC * N + (c.code.size - C.start) := by
        simp only [rank, hblockAt hC (Nat.le_refl _) (hok C hC).start_le_exit, hkC]
      rw [hra, hrb]
      exact rank_lt (hμ kC kB hpred) (by omega)
    · exact absurd (hback hB hkB hC hkC hbk) hfe.2

/-! ## The local properties -/

/-- **L3** for every function of a module the model accepts. -/
theorem controlFlowWellFormed_of_accepted {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) {d : FunctionDefinition}
    (hd : d ∈ m.functionDefs.toList) {code : CodeUnit} (hcode : d.code = some code) :
    ControlFlowWellFormed code.code := by
  obtain ⟨index, hidx⟩ := List.mem_iff_getElem?.1 hd
  rw [Array.getElem?_toList] at hidx
  obtain ⟨hb, -, -, hcu⟩ := verifyModule_ok h
  obtain ⟨n, hn⟩ := verifyCodeUnits_fn hcu hidx
  obtain ⟨handle, hv⟩ := verifyFunction_parts hcode hn
  obtain ⟨cfg, -, hcf, -⟩ := hv.views
  have hbr := fun pc o i hi ho => branch_targets hb hd hcode (pc := pc) (o := o) (i := i) hi ho
  exact ⟨nonempty_of_verified hv, fun pc _ => successors_lt (verifyControlFlow_fallthrough hcf) hbr pc,
    reducible_of_verified hcf hbr⟩

/-- **L3** and **L4** for every module the model accepts. -/
theorem codeStructure_of_accepted {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) :
    m.CodeStructure VerifierConfig.production.maxValueStackSize := by
  intro d hd code hcode
  obtain ⟨fh, hfh, hsd⟩ := stackDiscipline_of_accepted h hd hcode
  exact ⟨fh, hfh, controlFlowWellFormed_of_accepted h hd hcode, hsd⟩

/-- **The local properties** (L1–L6, L8–L10) of every module the model
accepts, given the phantom-position rule for field types that mention a
reference. Every other part of every property is proved from the model's
acceptance. L7 and L11, which `LocalProperties` adds, are not proved yet:
this is the milestone statement `LocalPropertiesNonGeneric`. -/
theorem localPropertiesNonGeneric_of_phantom (hph : PhantomUnderReferences) :
    LocalPropertiesNonGeneric verifier := by
  intro m hacc _ _
  have h : verifyModule VerifierConfig.production m = .ok () := by
    simp only [verifier] at hacc
    cases hm : verifyModule VerifierConfig.production m with
    | ok u => rfl
    | error e => rw [hm] at hacc; simp [accepts] at hacc
  obtain ⟨-, hdup, -, -⟩ := verifyModule_ok h
  exact ⟨indexIntegrity_of_accepted h, uniqueness_of_duplication hdup,
    codeStructure_of_accepted h, constantsWellFormed_of_accepted h,
    instructionConsistency_of_accepted h, typeFormation_of_accepted h,
    structDefinitionsWellFormed_of_accepted hph h, friendsWellFormed_of_accepted h⟩

/-- **The local properties**, unconditionally, once the model's phantom check
looks through references in the production configuration: the expected fix
of `check_phantom_params`, behind its configuration switch. -/
theorem localPropertiesNonGeneric_of_switch
    (hsw : SignatureV2.phantomThroughReferences VerifierConfig.production = true) :
    LocalPropertiesNonGeneric verifier :=
  localPropertiesNonGeneric_of_phantom (phantomUnderReferences_of_switch hsw)

end MoveBytecode.Proofs
