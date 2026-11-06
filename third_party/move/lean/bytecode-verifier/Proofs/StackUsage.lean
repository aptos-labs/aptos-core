-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec
import Proofs.Blocks

/-!
# L4 from the stack-usage pass

UNTRUSTED (checked by the kernel). If the model's stack-usage pass accepts a
function whose instructions' stack effects are the specification's arities
(`EffectsAgree`, which type safety provides), the function satisfies the
specification's stack discipline `StackDiscipline` (property L4): a single
operand-stack height before every instruction, consistent along every edge,
within bounds, zero at every block boundary and after every exit.

The height function is the one the stack-usage pass computes: zero at each
block id, and otherwise the height after the previous instruction.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## `Except` and table lookups -/

theorem bind_ok {ε α β : Type} {x : Except ε α} {f : α → Except ε β} {b : β} :
    (x >>= f) = .ok b ↔ ∃ a, x = .ok a ∧ f a = .ok b := by
  cases x <;> simp [bind, Except.bind]

theorem lookup_ok {α : Type} {t : Array α} {i : Nat} {w : String} {a : α} :
    lookup t i w = .ok a ↔ t[i]? = some a := by
  unfold lookup; cases t[i]? <;> simp [panicAt, pure, Except.pure]

theorem foldlM_ok_mem {α β ε : Type} {f : β → α → Except ε β} {l : List α} {b r : β}
    (h : l.foldlM f b = .ok r) : ∀ x ∈ l, ∃ acc r', f acc x = .ok r' := by
  induction l generalizing b with
  | nil => simp
  | cons y l ih =>
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨b', hy, hl⟩ := h
    intro x hx
    rcases List.mem_cons.1 hx with rfl | hx
    · exact ⟨b, b', hy⟩
    · exact ih hl x hx

theorem forM_ok {α ε : Type} {l : List α} {f : α → Except ε Unit} (h : l.forM f = .ok ()) :
    ∀ x ∈ l, f x = .ok () := by
  induction l with
  | nil => simp
  | cons y l ih =>
    simp only [List.forM, bind_ok] at h
    obtain ⟨u, hy, hl⟩ := h
    intro x hx
    rcases List.mem_cons.1 hx with rfl | hx
    · cases u; exact hy
    · exact ih hl x hx

/-! ## Instruction classifications -/

theorem control_fallThrough {i : Bytecode} (h : i.control = .fallThrough) :
    offset i = none ∧ isBranch i = false := by
  cases i <;> simp_all [Bytecode.control, offset, isBranch, isConditionalBranch,
    isUnconditionalBranch]

theorem control_jump {i : Bytecode} {o : Nat} (h : i.control = .jump o) :
    offset i = some o ∧ isBranch i = true := by
  cases i <;> simp_all [Bytecode.control, offset, isBranch, isConditionalBranch,
    isUnconditionalBranch]

theorem control_branch {i : Bytecode} {o : Nat} (h : i.control = .branch o) :
    offset i = some o ∧ isBranch i = true := by
  cases i <;> simp_all [Bytecode.control, offset, isBranch, isConditionalBranch,
    isUnconditionalBranch]

theorem control_exit {i : Bytecode} (h : i.control = .exit) :
    offset i = none ∧ isBranch i = true := by
  cases i <;> simp_all [Bytecode.control, offset, isBranch, isConditionalBranch,
    isUnconditionalBranch]

/-- The model's stack effect of every instruction of the code is the
specification's arity. Type safety establishes it for the instructions it
accepts (`Proofs/TypeSafety.lean`). -/
def EffectsAgree (m : CompiledModule) (fh : FunctionHandle) (returns : Signature)
    (code : Array Bytecode) : Prop :=
  ∀ i ∈ code.toList, ∀ e, instructionEffect m returns i = .ok e → i.arity m fh = some e

/-! ## Heights -/

variable (m : CompiledModule) (fh : FunctionHandle) (code : Array Bytecode)

/-- The specification's stack effect of the instruction at `pc`. -/
def eff (pc : Nat) : Nat × Nat :=
  match code[pc]? with
  | some i => (i.arity m fh).getD (0, 0)
  | none => (0, 0)

/-- The height before `pc`: zero at a block id, otherwise the height after the
previous instruction. -/
def heightBefore : Nat → Nat
  | 0 => 0
  | pc + 1 =>
    if pc + 1 ∈ blockIds code then 0
    else heightBefore pc - (eff m fh code pc).1 + (eff m fh code pc).2

def heightAfter (pc : Nat) : Nat :=
  heightBefore m fh code pc - (eff m fh code pc).1 + (eff m fh code pc).2

theorem heightBefore_of_mem {pc : Nat} (h : pc ∈ blockIds code) :
    heightBefore m fh code pc = 0 := by
  cases pc with
  | zero => rfl
  | succ p => simp [heightBefore, h]

theorem heightBefore_succ {pc : Nat} (h : pc + 1 ∉ blockIds code) :
    heightBefore m fh code (pc + 1) = heightAfter m fh code pc := by
  simp [heightBefore, h, heightAfter]

variable {m fh code}

/-! ## One instruction and one block of the pass -/

theorem stackStep_ok {config : VerifierConfig} {returns : Signature} {st st' : BlockState}
    {pc : Nat} (h : stackStep config m returns code st pc = .ok st') :
    ∃ i e, code[pc]? = some i ∧ instructionEffect m returns i = .ok e ∧ e.1 ≤ st.height ∧
      st'.height = st.height - e.1 + e.2 ∧ st'.height ≤ config.maxValueStackSize := by
  unfold stackStep at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨i, hi, ⟨pops, pushes⟩, he, h⟩ := h
  refine ⟨i, (pops, pushes), hi, he, ?_⟩
  cases hmp : config.maxPushSize <;>
    by_cases c2 : st.height < pops <;>
    by_cases c3 : config.maxValueStackSize < st.height - pops + pushes <;>
    simp_all [reject, bind, Except.bind, pure, Except.pure]
  all_goals (try split at h) <;> (try simp_all) <;> (try subst h) <;> (try simp) <;> omega

theorem verifyBlock_ok {config : VerifierConfig} {returns : Signature} {b : BasicBlock}
    (h : verifyBlock config m returns code b = .ok ()) :
    ∃ st, (List.range' b.start (b.exit + 1 - b.start)).foldlM
      (stackStep config m returns code) ⟨0, 0⟩ = .ok st ∧ st.height = 0 := by
  unfold verifyBlock at h
  simp only [bind_ok] at h
  obtain ⟨st, hf, h⟩ := h
  refine ⟨st, hf, ?_⟩
  by_cases hz : st.height = 0
  · exact hz
  · simp [hz, reject] at h

theorem foldlM_prefix {σ : Type} {f : σ → Nat → VResult σ} {s j k : Nat}
    {init st : σ} (h : (List.range' s (j + k)).foldlM f init = .ok st) :
    ∃ st', (List.range' s j).foldlM f init = .ok st' := by
  rw [← List.range'_append, List.foldlM_append, bind_ok] at h
  obtain ⟨st', h1, _⟩ := h
  exact ⟨st', h1⟩

theorem foldlM_last {σ : Type} {f : σ → Nat → VResult σ} {s j : Nat}
    {init st : σ} (h : (List.range' s (j + 1)).foldlM f init = .ok st) :
    ∃ st0, (List.range' s j).foldlM f init = .ok st0 ∧ f st0 (s + j) = .ok st := by
  rw [List.range'_concat, List.foldlM_append, bind_ok] at h
  obtain ⟨st0, h0, h1⟩ := h
  refine ⟨st0, h0, ?_⟩
  simpa [List.foldlM, bind_ok] using h1

/-- Inside a block, the height before an instruction is zero at the start and
the height after the previous instruction elsewhere. -/
theorem heightBefore_in_block {b : BasicBlock} (hb : BlockOk code b) {pc : Nat}
    (h1 : b.start ≤ pc) (h2 : pc ≤ b.exit) :
    heightBefore m fh code pc =
      if pc = b.start then 0 else heightAfter m fh code (pc - 1) := by
  by_cases hs : pc = b.start
  · subst hs; simp [heightBefore_of_mem m fh code hb.start_mem]
  · simp only [hs, if_false]
    obtain ⟨q, rfl⟩ : ∃ q, pc = q + 1 := ⟨pc - 1, by omega⟩
    rw [heightBefore_succ m fh code (hb.inner (q + 1) (by omega) h2)]
    simp

/-- The heights the pass computes along a block. -/
theorem block_heights {config : VerifierConfig} {returns : Signature}
    (hagree : EffectsAgree m fh returns code) {b : BasicBlock} (hb : BlockOk code b) :
    ∀ j st, j ≤ b.exit + 1 - b.start →
      (List.range' b.start j).foldlM (stackStep config m returns code) ⟨0, 0⟩ = .ok st →
      st.height ≤ config.maxValueStackSize ∧
      st.height = (if j = 0 then 0 else heightAfter m fh code (b.start + j - 1)) := by
  intro j
  induction j with
  | zero =>
    intro st _ h
    simp [List.range', pure, Except.pure] at h
    subst h; simp
  | succ j ih =>
    intro st hj h
    obtain ⟨st0, h0, h1⟩ := foldlM_last h
    obtain ⟨_, hst0⟩ := ih st0 (by omega) h0
    obtain ⟨i, e, hi, he, _, hst, hmax⟩ := stackStep_ok h1
    have hmem : i ∈ code.toList := by
      rw [Array.mem_toList_iff]; exact Array.mem_of_getElem? hi
    have harity := hagree i hmem e he
    have heff : eff m fh code (b.start + j) = e := by simp [eff, hi, harity]
    have hbefore : heightBefore m fh code (b.start + j) = st0.height := by
      rw [heightBefore_in_block hb (by omega) (by omega), hst0]
      cases j with
      | zero => simp
      | succ j' => simp
    refine ⟨hmax, ?_⟩
    simp only [Nat.add_one_ne_zero, if_false, show b.start + (j + 1) - 1 = b.start + j by omega]
    rw [hst, heightAfter, heff, hbefore]

/-- What the pass guarantees at one instruction of a verified block. -/
theorem block_pc {config : VerifierConfig} {returns : Signature}
    (hagree : EffectsAgree m fh returns code) {b : BasicBlock} (hb : BlockOk code b)
    (hv : verifyBlock config m returns code b = .ok ()) {pc : Nat}
    (h1 : b.start ≤ pc) (h2 : pc ≤ b.exit) :
    ∃ i, code[pc]? = some i ∧ i.arity m fh = some (eff m fh code pc) ∧
      (eff m fh code pc).1 ≤ heightBefore m fh code pc ∧
      heightBefore m fh code pc ≤ config.maxValueStackSize ∧
      heightAfter m fh code pc ≤ config.maxValueStackSize := by
  obtain ⟨stF, hF, _⟩ := verifyBlock_ok hv
  rw [show b.exit + 1 - b.start = (pc - b.start + 1) + (b.exit - pc) by omega] at hF
  obtain ⟨st1, h1'⟩ := foldlM_prefix hF
  obtain ⟨st0, h0, hstep⟩ := foldlM_last h1'
  obtain ⟨hmax0, hst0⟩ := block_heights hagree hb (pc - b.start) st0 (by omega) h0
  rw [show b.start + (pc - b.start) = pc by omega] at hstep
  obtain ⟨i, e, hi, he, hpops, hst, hmax⟩ := stackStep_ok hstep
  have hmem : i ∈ code.toList := by
    rw [Array.mem_toList_iff]; exact Array.mem_of_getElem? hi
  have harity := hagree i hmem e he
  have heff : eff m fh code pc = e := by simp [eff, hi, harity]
  have hbefore : heightBefore m fh code pc = st0.height := by
    rw [heightBefore_in_block hb h1 h2, hst0]
    by_cases hs : pc = b.start
    · simp [hs]
    · simp [hs, show pc - b.start ≠ 0 by omega, show b.start + (pc - b.start) - 1 = pc - 1 by omega]
  refine ⟨i, hi, by rw [heff]; exact harity, by rw [heff, hbefore]; exact hpops,
    by rw [hbefore]; exact hmax0, ?_⟩
  rw [heightAfter, heff, hbefore, ← hst]; exact hmax

/-- The last instruction of a verified block leaves the stack empty. -/
theorem block_exit {config : VerifierConfig} {returns : Signature}
    (hagree : EffectsAgree m fh returns code) {b : BasicBlock} (hb : BlockOk code b)
    (hv : verifyBlock config m returns code b = .ok ()) :
    heightAfter m fh code b.exit = 0 := by
  obtain ⟨stF, hF, hz⟩ := verifyBlock_ok hv
  obtain ⟨_, hst⟩ := block_heights hagree hb _ stF (Nat.le_refl _) hF
  have := hb.start_le_exit
  rw [hz] at hst
  simp [show b.exit + 1 - b.start ≠ 0 by omega,
    show b.start + (b.exit + 1 - b.start) - 1 = b.exit by omega] at hst
  exact hst.symm

/-! ## Block boundaries -/

theorem lt_of_getElem? {α : Type} {a : Array α} {i : Nat} {x : α} (h : a[i]? = some x) :
    i < a.size := by
  rcases Nat.lt_or_ge i a.size with h' | h'
  · exact h'
  · simp [Array.getElem?_eq_none h'] at h

/-- The specification's block leaders are the model's block ids. -/
theorem isLeader_mem {pc : Nat} (hpc : pc < code.size) (h : IsLeader code pc) :
    pc ∈ blockIds code := by
  rcases h with rfl | ⟨q, i, hq, hm⟩
  · exact zero_mem_blockIds code
  · rw [mem_blockIds]; right
    refine ⟨q, lt_of_getElem? hq, i, hq, ?_⟩
    cases hc : i.control with
    | fallThrough => rw [hc] at hm; exact hm.elim
    | jump o =>
      obtain ⟨ho, hbr⟩ := control_jump hc
      rw [hc] at hm
      rcases hm with rfl | rfl
      · exact Or.inl ho
      · exact Or.inr ⟨hbr, hpc, rfl⟩
    | branch o =>
      obtain ⟨ho, hbr⟩ := control_branch hc
      rw [hc] at hm
      rcases hm with rfl | rfl
      · exact Or.inl ho
      · exact Or.inr ⟨hbr, hpc, rfl⟩
    | exit =>
      obtain ⟨_, hbr⟩ := control_exit hc
      rw [hc] at hm
      subst hm
      exact Or.inr ⟨hbr, hpc, rfl⟩

/-- A branch ends its block. -/
theorem branch_ends {pc : Nat} {i : Bytecode} (hi : code[pc]? = some i)
    (hbr : isBranch i = true) : pc + 1 = code.size ∨ pc + 1 ∈ blockIds code := by
  have hpc := lt_of_getElem? hi
  by_cases h : pc + 1 < code.size
  · right; rw [mem_blockIds]; right; exact ⟨pc, hpc, i, hi, Or.inr ⟨hbr, h, rfl⟩⟩
  · left; omega

/-- A jump target is a block id. -/
theorem offset_mem {pc o : Nat} {i : Bytecode} (hi : code[pc]? = some i)
    (ho : offset i = some o) : o ∈ blockIds code := by
  rw [mem_blockIds]; right; exact ⟨pc, lt_of_getElem? hi, i, hi, Or.inl ho⟩

/-- An instruction that ends a block is its block's last instruction. -/
theorem ends_at_exit {b : BasicBlock} (hb : BlockOk code b) {pc : Nat} (h1 : b.start ≤ pc)
    (h2 : pc ≤ b.exit) (hend : pc + 1 = code.size ∨ pc + 1 ∈ blockIds code) : pc = b.exit := by
  rcases Nat.lt_or_ge pc b.exit with hlt | hge
  · have := hb.exit_lt
    rcases hend with h | h
    · omega
    · exact absurd h (hb.inner (pc + 1) (by omega) (by omega))
  · omega

/-! ## L4 -/

/-- **L4** for a code unit whose blocks the stack-usage pass accepts. -/
theorem stackDiscipline_of_blocks {config : VerifierConfig} {returns : Signature}
    (hagree : EffectsAgree m fh returns code)
    (h : ∀ b ∈ buildBlocks code, verifyBlock config m returns code b = .ok ()) :
    StackDiscipline m fh code config.maxValueStackSize := by
  let H : Nat → Nat := fun pc => if pc < code.size then heightBefore m fh code pc else 0
  have Hmem : ∀ x, x ∈ blockIds code → H x = 0 := by
    intro x hx; simp only [H]; split <;> simp [heightBefore_of_mem m fh code hx]
  refine ⟨H, by simp [H, heightBefore], ?_⟩
  intro pc i hi
  have hpc := lt_of_getElem? hi
  obtain ⟨b, hbm, hs, he⟩ := (buildBlocks_spec code).2 pc hpc
  have hb := (buildBlocks_spec code).1 b hbm
  obtain ⟨i', hi', harity, hpops, hmaxb, hmaxa⟩ := block_pc hagree hb (h b hbm) hs he
  rw [hi] at hi'
  cases hi'
  have hzero : (pc + 1 = code.size ∨ pc + 1 ∈ blockIds code) → heightAfter m fh code pc = 0 := by
    intro hend
    rw [ends_at_exit hb hs he hend]
    exact block_exit hagree hb (h b hbm)
  have Hpc : H pc = heightBefore m fh code pc := by simp [H, hpc]
  have Hafter : H pc - (eff m fh code pc).1 + (eff m fh code pc).2 = heightAfter m fh code pc := by
    rw [Hpc]; rfl
  refine ⟨(eff m fh code pc).1, (eff m fh code pc).2, harity, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · rw [Hpc]; exact hpops
  · rw [Hpc]; exact hmaxb
  · rw [Hafter]; exact hmaxa
  · intro s hs'
    rw [Hafter]
    simp only [successors, hi] at hs'
    cases hc : i.control with
    | fallThrough =>
      rw [hc] at hs'
      simp at hs'
      subst hs'
      by_cases hend : pc + 1 = code.size ∨ pc + 1 ∈ blockIds code
      · rw [hzero hend]
        rcases hend with h' | h'
        · simp [H, h']
        · exact Hmem _ h'
      · simp only [not_or] at hend
        have hlt : pc + 1 < code.size := by omega
        simp [H, hlt, heightBefore_succ m fh code hend.2]
    | jump o =>
      rw [hc] at hs'
      simp at hs'
      subst hs'
      obtain ⟨ho, hbr⟩ := control_jump hc
      rw [hzero (branch_ends hi hbr)]
      exact Hmem _ (offset_mem hi ho)
    | branch o =>
      rw [hc] at hs'
      obtain ⟨ho, hbr⟩ := control_branch hc
      rw [hzero (branch_ends hi hbr)]
      simp at hs'
      rcases hs' with rfl | rfl
      · exact Hmem _ (offset_mem hi ho)
      · rcases branch_ends hi hbr with h' | h'
        · simp [H, h']
        · exact Hmem _ h'
    | exit =>
      rw [hc] at hs'
      simp at hs'
  · intro hl
    rw [Hpc]; exact heightBefore_of_mem m fh code (isLeader_mem hpc hl)
  · intro hc
    rw [Hafter]
    exact hzero (branch_ends hi (control_exit hc).2)

/-! ## From the model's acceptance -/

theorem cfgBuild_blocks {c : Array Bytecode} {cfg : Cfg} (h : Cfg.build c = .ok cfg) :
    cfg.blocks = buildBlocks c := by
  unfold Cfg.build at h
  simp only [bind_ok] at h
  obtain ⟨t, _, h⟩ := h
  simp [pure, Except.pure] at h
  rw [← h]

theorem verifyControlFlow_blocks {config : VerifierConfig} {version : Nat} {c : CodeUnit}
    {cfg : Cfg} (h : verifyControlFlow config version c = .ok cfg) :
    cfg.blocks = buildBlocks c.code := by
  unfold verifyControlFlow at h
  by_cases hv : version ≤ 5
  · simp [hv, panicAt, bind, Except.bind] at h
  · simp only [hv, if_false, bind_ok] at h
    obtain ⟨_, _, cfg', hb, _, _, h⟩ := h
    simp [pure, Except.pure] at h
    subst h
    exact cfgBuild_blocks hb

theorem functionView_ok {c : CodeUnit} {handle : FunctionHandle} {cfg : Cfg} {index : Nat}
    {view : FunctionView} (h : FunctionView.function m index c handle cfg = .ok view) :
    view.code = c ∧ view.cfg = cfg ∧ m.signatures[handle.returns]? = some view.returns := by
  unfold FunctionView.function at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨_, _, rs, hrs, _, _, h⟩ := h
  simp [pure, Except.pure] at h
  subst h
  exact ⟨rfl, rfl, hrs⟩

end MoveBytecode.Proofs
