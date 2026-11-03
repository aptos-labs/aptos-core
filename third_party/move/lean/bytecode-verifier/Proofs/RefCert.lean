-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefDesc
import Proofs.RefTypedCall
import Proofs.Availability
import Proofs.Traversal
import Proofs.Reducible

/-!
# The reference-safety analysis's states, instruction by instruction

UNTRUSTED (checked by the kernel). From the pre-states the reference-safety
analysis records for its blocks, the state before each instruction
(`refBefore`): inside a block, the analysis's transfer of the previous
instruction; at the end of a block, its canonical form is covered by the
pre-state of every successor (`ref_at`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

section Before

variable (m : CompiledModule) (view : FunctionView)

/-- The analysis's state before each instruction: the recorded pre-state at a
block id, otherwise the state after the previous instruction. -/
def refBefore (code : Array Bytecode) (inv : List (Nat × RefState)) : Nat → Option RefState
  | 0 => inv.lookup 0
  | pc + 1 =>
    if pc + 1 ∈ blockIds code then inv.lookup (pc + 1)
    else (refBefore code inv pc).bind fun st => match code[pc]? with
      | some instr => match (executeInner m view instr).run st with
        | .ok ((), st') => some st'
        | .error _ => none
      | none => none

theorem refBefore_of_mem {code : Array Bytecode} {inv : List (Nat × RefState)} {pc : Nat}
    (h : pc ∈ blockIds code) : refBefore m view code inv pc = inv.lookup pc := by
  cases pc with
  | zero => rfl
  | succ p => simp [refBefore, h]

/-- What one step of `executeBlock` does. -/
theorem execute_run {st st' : RefState} {instr : Bytecode} {off e : Nat}
    (h : ReferenceSafety.execute m view st instr off e = .ok st') :
    ∃ st₁, (executeInner m view instr).run st = .ok ((), st₁) ∧
      (off = e → st₁.stack = [] ∧ st' = canonState st₁) ∧ (off ≠ e → st' = st₁) := by
  unfold ReferenceSafety.execute at h
  simp only [bind_ok] at h
  obtain ⟨⟨u, st₂⟩, hrun, h⟩ := h
  simp only [pure, Except.pure, Except.ok.injEq] at h
  subst h
  rw [run_bind] at hrun
  obtain ⟨u₁, st₁, hx, hc⟩ := hrun
  refine ⟨st₁, hx, fun hoe => ?_, fun hoe => ?_⟩
  · subst hoe
    simp only [beq_self_eq_true, if_true] at hc
    exact canonicalize_run hc
  · have : (off == e) = false := by simpa using hoe
    simp only [this, Bool.false_eq_true, if_false] at hc
    exact (pure_run.1 hc).2

/-- The step `executeBlock` folds. -/
def refBlockStep (code : Array Bytecode) (e : Nat) (st : RefState) (offset : Nat) :
    VResult RefState := do
  (referenceAnalysis m view).execute st (← lookup code offset "code[offset]") offset e

theorem refBlockStep_ok {code : Array Bytecode} {e : Nat} {st st' : RefState} {pc : Nat}
    (h : refBlockStep m view code e st pc = .ok st') :
    ∃ instr, code[pc]? = some instr ∧ ReferenceSafety.execute m view st instr pc e = .ok st' := by
  simp only [refBlockStep, bind_ok, lookup_ok] at h
  obtain ⟨instr, hi, h⟩ := h
  exact ⟨instr, hi, h⟩

theorem refExecuteBlock_eq {cfg : Cfg} {code : Array Bytecode}
    (hcfg : cfg.blocks = buildBlocks code) {b : BasicBlock} (hb : b ∈ buildBlocks code)
    (pre : RefState) :
    executeBlock (referenceAnalysis m view) cfg code b.start pre =
      (List.range' b.start (b.exit + 1 - b.start)).foldlM (refBlockStep m view code b.exit) pre := by
  simp only [executeBlock, cfg_blockEnd hcfg hb]
  rfl

/-- Along a block that executes from its recorded pre-state, `refBefore`
follows the execution. -/
theorem refBlock_prefix {code : Array Bytecode} {inv : List (Nat × RefState)}
    {b : BasicBlock} (hb : BlockOk code b) {pre : RefState}
    (hpre : inv.lookup b.start = some pre) :
    ∀ j st, j ≤ b.exit + 1 - b.start →
      (List.range' b.start j).foldlM (refBlockStep m view code b.exit) pre = .ok st →
      (b.start + j ≤ b.exit → refBefore m view code inv (b.start + j) = some st) := by
  intro j
  induction j with
  | zero =>
    intro st _ h _
    simp [List.range', pure, Except.pure] at h
    subst h
    rw [Nat.add_zero, refBefore_of_mem m view hb.start_mem]
    simpa using hpre
  | succ j ih =>
    intro st hj h hle
    obtain ⟨st0, h0, h1⟩ := foldlM_last h
    have hcert0 := ih st0 (by omega) h0 (by omega)
    obtain ⟨instr, hi, hex⟩ := refBlockStep_ok m view h1
    obtain ⟨st₁, hx, -, hne⟩ := execute_run m view hex
    have hst : st = st₁ := hne (by omega)
    subst hst
    rw [show b.start + (j + 1) = b.start + j + 1 by omega]
    simp [refBefore, hb.inner (b.start + j + 1) (by omega) (by omega), hcert0, hi, hx]

end Before

section Fixpoint

variable {m : CompiledModule} {view : FunctionView} {cfg : Cfg} {code : Array Bytecode}
  {initial : RefState} {inv : List (Nat × RefState)}

/-- Inside an analyzed block, the state before each instruction executes it;
inside the block the result is the state before the next instruction, and at
the block's end its canonical form is covered by the pre-state of each
successor block. -/
theorem ref_at (hcfg : cfg.blocks = buildBlocks code)
    (hfix : PostFixpointOf (referenceAnalysis m view) RefLe cfg code initial inv)
    {b : BasicBlock} (hb : b ∈ buildBlocks code) {pre : RefState}
    (hpre : inv.lookup b.start = some pre) {pc : Nat} (h1 : b.start ≤ pc) (h2 : pc ≤ b.exit) :
    ∃ st st' instr, code[pc]? = some instr ∧ refBefore m view code inv pc = some st ∧
      (executeInner m view instr).run st = .ok ((), st') ∧
      (pc < b.exit → refBefore m view code inv (pc + 1) = some st') ∧
      (pc = b.exit → st'.stack = [] ∧
        ∀ s ∈ b.successors, ∃ pre', inv.lookup s = some pre' ∧ RefLe (canonState st') pre') := by
  have hok := (buildBlocks_spec code).1 b hb
  obtain ⟨post, hex, hsucc⟩ := hfix.closed b.start pre hpre
  rw [refExecuteBlock_eq m view hcfg hb] at hex
  rw [cfg_successors hcfg hb] at hsucc
  have hsplit : b.exit + 1 - b.start = (pc - b.start + 1) + (b.exit - pc) := by omega
  rw [hsplit] at hex
  obtain ⟨stj, hj⟩ := foldlM_prefix hex
  obtain ⟨st0, h0, hstep⟩ := foldlM_last hj
  have hbefore := refBlock_prefix m view hok hpre (pc - b.start) st0 (by omega) h0 (by omega)
  rw [show b.start + (pc - b.start) = pc by omega] at hbefore hstep
  obtain ⟨instr, hi, hexe⟩ := refBlockStep_ok m view hstep
  obtain ⟨st₁, hx, hend, hne⟩ := execute_run m view hexe
  refine ⟨st0, st₁, instr, hi, hbefore, hx, fun hlt => ?_, fun heq => ?_⟩
  · have := refBlock_prefix m view hok hpre (pc - b.start + 1) stj (by omega) hj (by omega)
    rw [show b.start + (pc - b.start + 1) = pc + 1 by omega] at this
    rw [this, hne (by omega)]
  · obtain ⟨hs, hcanon⟩ := hend heq
    refine ⟨hs, ?_⟩
    have hlast : b.exit - pc = 0 := by omega
    rw [hlast, Nat.add_zero] at hex
    rw [hj, Except.ok.injEq] at hex
    subst hex
    rw [hcanon] at hsucc
    exact hsucc

/-- Every reachable instruction is in an analyzed block. -/
theorem ref_analyzed_at (hcfg : cfg.blocks = buildBlocks code)
    (hfix : PostFixpointOf (referenceAnalysis m view) RefLe cfg code initial inv)
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
    have hylt := hlt x y hy
    obtain ⟨st, st', instr, hi, -, -, -, hend⟩ := ref_at hcfg hfix hb hpre h1 h2
    rcases Nat.lt_or_ge x b.exit with hx | hx
    · rw [inner_successor hok h1 hx hi hy]
      exact ⟨b, hb, by omega, by omega, pre, hpre⟩
    · have hxe : x = b.exit := by omega
      subst hxe
      have hys : y ∈ b.successors := by
        rw [hok.successors]; exact mem_getSuccessors hi hy hylt
      obtain ⟨pre', hpre', -⟩ := (hend rfl).2 y hys
      obtain ⟨b', hb', hs'⟩ := block_of_mem (exit_successor_mem hok hi hy hylt) hylt
      have hok' := (buildBlocks_spec code).1 b' hb'
      have := hok'.start_le_exit
      exact ⟨b', hb', by omega, by omega, pre', hs' ▸ hpre'⟩

end Fixpoint

/-! ## The abstract invariant along blocks -/

theorem reachable_succ {code : Array Bytecode} {pc s : Nat} (h : CodeReachable code pc)
    (hs : s ∈ successors code pc) : CodeReachable code s := by
  obtain ⟨p, hw⟩ := h
  exact ⟨_, walk_snoc hw hs⟩

theorem Typed.mono_avail {ltys stys : List SignatureToken} {avail avail' : LocalIndex → Availability}
    {st : RefState} (ht : Typed ltys stys avail st)
    (h : ∀ i, avail' i = .available → avail i = .available) : Typed ltys stys avail' st :=
  { ht with locKind := fun i t b hi hb hav => ht.locKind i t b hi hb (h i hav) }

/-- What the closure of the abstract invariant needs of a function: what the
earlier passes establish, and the M3 restriction. -/
structure RefSetting (P : Program) (f : FunctionRef) (m : CompiledModule) (view : FunctionView)
    (avail : Nat → LocalIndex → Availability) : Prop where
  agrees : GenCtx P f m view
  bounds : verifyBounds m = .ok ()
  blocks : view.cfg.blocks = buildBlocks view.code.code
  nonempty : 0 < view.code.code.size
  succLt : ∀ pc, ∀ s ∈ successors view.code.code pc, s < view.code.code.size
  fall : ∃ last, view.code.code.back? = some last ∧ isUnconditionalBranch last = true
  types : ∀ pc, pc < view.code.code.size → ∃ stys stys' before,
    typesBefore m view pc = some stys ∧ RuntimeType.ofSignatureWith m f.typeArgs stys = some before ∧
    typeStep m view stys pc = .ok stys' ∧
    (pc + 1 < view.code.code.size → pc + 1 ∉ blockIds view.code.code →
      typesBefore m view (pc + 1) = some stys') ∧
    (pc + 1 = view.code.code.size ∨ pc + 1 ∈ blockIds view.code.code → stys' = [])
  availRead : ∀ (pc : Nat) (instr : Bytecode), CodeReachable view.code.code pc → view.code.code[pc]? = some instr →
    ∀ i, instr.readsLocal = some i → avail pc i = .available
  availSucc : ∀ (pc : Nat) (instr : Bytecode), CodeReachable view.code.code pc → view.code.code[pc]? = some instr →
    ∀ s ∈ successors view.code.code pc, ∀ i,
      Availability.Refines (availabilityAfter instr (avail pc) i) (avail s i)
  m2 : ∀ (pc : Nat) (instr : Bytecode), view.code.code[pc]? = some instr → instr = .ret ∨ M2Instr m instr
  ops : ∀ (pc : Nat) (instr : Bytecode), view.code.code[pc]? = some instr → InstNonRef m instr

/-- The abstract invariant of the pre-states the analysis records. -/
def RefJ (view : FunctionView) (avail : Nat → LocalIndex → Availability) (s : Nat)
    (a : RefState) : Prop :=
  s ∈ blockIds view.code.code ∧ CodeReachable view.code.code s ∧
    Typed (view.parameters ++ view.locals) [] (avail s) a

theorem refines_avail {a b : Availability} (h : Availability.Refines a b) (hb : b = .available) :
    a = .available := by
  rcases h with h | h <;> subst hb <;> simp_all

section Closure

variable {P : Program} {f : FunctionRef} {m : CompiledModule} {view : FunctionView}
  {avail : Nat → LocalIndex → Availability}

/-- One instruction inside or at the end of a block keeps the invariant, for
the types and availability after it. -/
theorem refSetting_step (hs : RefSetting P f m view avail) {pc : Nat} {instr : Bytecode}
    {stys : List SignatureToken} {st st' : RefState}
    (hreach : CodeReachable view.code.code pc) (hi : view.code.code[pc]? = some instr)
    (hnr : instr ≠ .ret) (htys : typesBefore m view pc = some stys)
    (ht : Typed (view.parameters ++ view.locals) stys (avail pc) st)
    (hx : (executeInner m view instr).run st = .ok ((), st')) :
    ∃ stys', typeStep m view stys pc = .ok stys' ∧
      Typed (view.parameters ++ view.locals) stys' (availabilityAfter instr (avail pc)) st' ∧
      (pc + 1 < view.code.code.size → pc + 1 ∉ blockIds view.code.code →
        typesBefore m view (pc + 1) = some stys') ∧
      (pc + 1 = view.code.code.size ∨ pc + 1 ∈ blockIds view.code.code → stys' = []) := by
  have hlt : pc < view.code.code.size := lt_of_getElem? hi
  obtain ⟨stys0, stys', before, h0, hsig, hstep, hnext, hend⟩ := hs.types pc hlt
  rw [htys, Option.some.injEq] at h0
  subst h0
  obtain ⟨i', hi', hty⟩ := typeStep_ok hstep
  rw [hi, Option.some.injEq] at hi'
  subst hi'
  have hm2 : M2Instr m instr := by
    rcases hs.m2 pc instr hi with h | h
    · exact absurd h hnr
    · exact h
  exact ⟨stys', hstep, typed_exec_M2 hs.agrees hs.bounds rfl hsig hm2 (hs.ops pc instr hi) ht hty
    (hs.availRead pc instr hreach hi) hx, hnext, hend⟩

/-- Along a block that executes from a pre-state satisfying the invariant,
the state before each instruction is well typed. -/
theorem refBlock_typed (hs : RefSetting P f m view avail) {b : BasicBlock}
    (hb : BlockOk view.code.code b) {pre : RefState}
    (hJ : RefJ view avail b.start pre) :
    ∀ j st, j ≤ b.exit + 1 - b.start →
      (List.range' b.start j).foldlM (refBlockStep m view view.code.code b.exit) pre = .ok st →
      b.start + j ≤ b.exit →
      CodeReachable view.code.code (b.start + j) ∧ ∃ stys, typesBefore m view (b.start + j) = some stys ∧
        Typed (view.parameters ++ view.locals) stys (avail (b.start + j)) st := by
  obtain ⟨-, hreach0, ht0⟩ := hJ
  intro j
  induction j with
  | zero =>
    intro st _ h _
    simp [List.range', pure, Except.pure] at h
    subst h
    exact ⟨hreach0, [], typesBefore_of_mem m view hb.start_mem, ht0⟩
  | succ j ih =>
    intro st hj h hle
    obtain ⟨st0, h0, h1⟩ := foldlM_last h
    obtain ⟨hreach, stys, htys, ht⟩ := ih st0 (by omega) h0 (by omega)
    obtain ⟨instr, hi, hex⟩ := refBlockStep_ok m view h1
    obtain ⟨st₁, hx, -, hne⟩ := execute_run m view hex
    have hst : st = st₁ := hne (by omega)
    subst hst
    have hnotb : b.start + j + 1 ∉ blockIds view.code.code :=
      hb.inner (b.start + j + 1) (by omega) (by omega)
    -- An inner instruction falls through.
    have hnb : isBranch instr = false := by
      cases hbr : isBranch instr
      · rfl
      · rcases branch_ends hi hbr with h | h
        · have := hb.exit_lt; omega
        · exact absurd h hnotb
    have hsucc : b.start + j + 1 ∈ successors view.code.code (b.start + j) := by
      simp [successors, hi, not_branch_fallThrough hnb]
    have hnr : instr ≠ .ret := by rintro rfl; simp [isBranch, isUnconditionalBranch, isConditionalBranch] at hnb
    obtain ⟨stys', -, ht', hnext, -⟩ := refSetting_step hs hreach hi hnr htys ht hx
    rw [show b.start + (j + 1) = b.start + j + 1 by omega]
    refine ⟨reachable_succ hreach hsucc, stys', hnext (by have := hb.exit_lt; omega) hnotb, ?_⟩
    exact ht'.mono_avail fun i hav =>
      refines_avail (hs.availSucc _ instr hreach hi _ hsucc i) hav

/-- **The invariant is closed** under the analysis's blocks and joins. -/
theorem refJClosed (hs : RefSetting P f m view avail) :
    JClosed (referenceAnalysis m view) view.cfg view.code.code (RefJ view avail) where
  exec := by
    intro bs pre post hJ hex s hsucc
    obtain ⟨hmem, hreach0, ht0⟩ := hJ
    have hlt0 : bs < view.code.code.size := by
      obtain ⟨p, hw⟩ := hreach0
      exact walk_induct (Q := fun pc => pc < view.code.code.size) hw hs.nonempty
        (fun x y _ hy => hs.succLt x y hy)
    obtain ⟨b, hb, rfl⟩ := block_of_mem hmem hlt0
    have hok := (buildBlocks_spec view.code.code).1 b hb
    rw [refExecuteBlock_eq m view hs.blocks hb] at hex
    rw [cfg_successors hs.blocks hb] at hsucc
    have hsplit : b.exit + 1 - b.start = (b.exit - b.start) + 1 := by
      have := hok.start_le_exit; omega
    rw [hsplit] at hex
    obtain ⟨st0, h0, hstep⟩ := foldlM_last hex
    obtain ⟨hreach, stys, htys, ht⟩ := refBlock_typed hs hok ⟨hmem, hreach0, ht0⟩
      (b.exit - b.start) st0 (by omega) h0 (by have := hok.start_le_exit; omega)
    have hse : b.start + (b.exit - b.start) = b.exit := by have := hok.start_le_exit; omega
    rw [hse] at hstep hreach htys ht
    obtain ⟨instr, hi, hexe⟩ := refBlockStep_ok m view hstep
    obtain ⟨st₁, hx, hend, -⟩ := execute_run m view hexe
    obtain ⟨-, hpost⟩ := hend rfl
    subst hpost
    rw [hok.successors] at hsucc
    have hsucc' := (getSuccessors_mem hs.fall).1 hsucc
    have hslt := hs.succLt _ s hsucc'
    by_cases hret : instr = .ret
    · subst hret
      simp [successors, hi, Bytecode.control] at hsucc'
    · obtain ⟨stys', -, ht', -, hlast⟩ := refSetting_step hs hreach hi hret htys ht hx
      have hend' := hok.exit_end
      simp only [isEndOfBlock, Bool.or_eq_true, beq_iff_eq, List.contains_iff_mem] at hend'
      have hnil : stys' = [] := hlast hend'
      subst hnil
      have htc := ht'.canonical
      exact ⟨exit_successor_mem hok hi hsucc' hslt, reachable_succ hreach hsucc',
        htc.mono_avail fun i hav => refines_avail (hs.availSucc _ instr hreach hi _ hsucc' i) hav⟩
  join := by
    intro s a b j changed ha hb hj
    exact ⟨ha.1, ha.2.1, typed_join ha.2.2 hb.2.2 hj⟩

end Closure

/-! ## The entry -/

/-- The analysis's state at entry, before the frame root is added. -/
def newState (n : Nat) (g : Graph) : RefState where
  locals := List.replicate n .nonReference
  graph := g
  nextId := n + 1
  stack := []

/-- The analysis's initial state, for a function whose parameters are values. -/
theorem new_values {view : FunctionView} {st : RefState} (h : RefState.new view = .ok st)
    (hps : ∀ t ∈ view.parameters, refKind t = none) :
    ∃ g, Graph.newRef [] (view.parameters.length + view.locals.length) true = .ok g ∧
      st = newState (view.parameters.length + view.locals.length) g := by
  unfold RefState.new at h
  simp only [bind_ok] at h
  obtain ⟨st₁, hfold, h⟩ := h
  -- The pass over the parameters changes nothing.
  have hst₁ : st₁ = newState (view.parameters.length + view.locals.length) [] := by
    let I : RefState → List (SignatureToken × Nat) → Prop := fun st rest =>
      st = newState (view.parameters.length + view.locals.length) [] ∧
        ∀ x ∈ rest, refKind x.1 = none
    have := foldlM_invariant I (fun acc x rest acc' hI hf => ?_) _ _ _
      ⟨rfl, fun x hx => by
        obtain ⟨ty, i⟩ := x
        exact hps ty (List.mem_of_getElem? (List.mem_zipIdx_iff_getElem?.1 hx))⟩ hfold
    · exact this.1
    · obtain ⟨hacc, hrest⟩ := hI
      obtain ⟨ty, i⟩ := x
      have hk : refKind ty = none := hrest (ty, i) List.mem_cons_self
      simp only [hk, pure, Except.pure, Except.ok.injEq] at hf
      subst hf
      exact ⟨hacc, fun y hy => hrest y (List.mem_cons_of_mem _ hy)⟩
  subst hst₁
  obtain ⟨g, hg, h⟩ := h
  simp only [pure, Except.pure, Except.ok.injEq] at h
  exact ⟨g, hg, by rw [← h]; rfl⟩

/-- **The initial state is well typed** when the parameters are values and
only they are available. -/
theorem typed_new {view : FunctionView} {st : RefState} {avail : LocalIndex → Availability}
    (h : RefState.new view = .ok st) (hps : ∀ t ∈ view.parameters, refKind t = none)
    (havail : ∀ i, avail i = .available → i < view.parameters.length) :
    Typed (view.parameters ++ view.locals) [] avail st := by
  obtain ⟨g, hg, rfl⟩ := new_values h hps
  have hf := Graph.newRef_find hg
  refine {
    wf := Graph.newRef_wf hg Graph.nil_wf
    back := Graph.newRef_back hg Graph.nil_back
    rootMut := by simpa [newState] using Graph.newRef_mutability_self hg
    nlocals := by simp [newState]
    nstack := rfl
    keys := fun x hx => ?_
    distinct := by simp [heldIds, newState, filterMap_refId_replicate]
    rootFree := by simp [heldIds, newState, filterMap_refId_replicate]
    locMut := fun i x hi => by simp [newState, List.getElem?_replicate] at hi
    stkMut := fun k x hk => by simp [newState] at hk
    stkKind := fun k t b hk _ => by simp at hk
    locKind := fun i t b hi hb hav => ?_ }
  · simp only [newState] at hx ⊢
    rw [hf] at hx
    split at hx
    · next hx' => exact Or.inl (by simpa using hx')
    · simp [NatMap.find?] at hx
  · have hlt := havail i hav
    rw [List.getElem?_append_left hlt] at hi
    rw [hps t (List.mem_of_getElem? hi)] at hb
    cases hb

/-! ## The post-fixpoint -/

theorem refJoinSound {view : FunctionView} {avail : Nat → LocalIndex → Availability}
    (m : CompiledModule) : JoinSound (referenceAnalysis m view) RefLe (RefJ view avail) where
  refl := RefLe.refl
  trans := RefLe.trans
  join := fun ha hb h => join_sound ha.2.2 hb.2.2 h

/-- **The reference-safety analysis computes a post-fixpoint** for the
description order, from the initial state, with every recorded pre-state
satisfying the abstract invariant. -/
theorem ref_postFixpoint {P : Program} {f : FunctionRef} {m : CompiledModule}
    {view : FunctionView} {avail : Nat → LocalIndex → Availability}
    (hs : RefSetting P f m view avail) (hbuild : Cfg.build view.code.code = .ok view.cfg)
    (hps : ∀ t ∈ view.parameters, refKind t = none)
    (havail0 : ∀ i, avail 0 i = .available → i < view.parameters.length)
    (h : verifyReferenceSafety m view = .ok ()) :
    ∃ initial inv, RefState.new view = .ok initial ∧
      PostFixpointOf (referenceAnalysis m view) RefLe view.cfg view.code.code initial inv ∧
      ∀ b pre, inv.lookup b = some pre → RefJ view avail b pre := by
  unfold verifyReferenceSafety at h
  simp only [bind_ok] at h
  obtain ⟨initial, hinit, inv, hana, -⟩ := h
  have hJ0 : RefJ view avail 0 initial :=
    ⟨zero_mem_blockIds _, ⟨[0], .nil 0⟩, typed_new hinit hps havail0⟩
  exact ⟨initial, inv, hinit,
    analyze_postFixpoint (refJoinSound m) (refJClosed hs) hJ0 hbuild hana,
    analyze_justified (RefJ view avail) hJ0 (refJClosed hs).exec (refJClosed hs).join hana⟩

/-- **The abstract invariant before each instruction** of an analyzed block. -/
theorem ref_typed_at {P : Program} {f : FunctionRef} {m : CompiledModule} {view : FunctionView}
    {avail : Nat → LocalIndex → Availability} {initial : RefState} {inv : List (Nat × RefState)}
    (hs : RefSetting P f m view avail)
    (hfix : PostFixpointOf (referenceAnalysis m view) RefLe view.cfg view.code.code initial inv)
    {b : BasicBlock} (hb : b ∈ buildBlocks view.code.code) {pre : RefState}
    (hpre : inv.lookup b.start = some pre) (hJ : RefJ view avail b.start pre) {pc : Nat}
    (h1 : b.start ≤ pc) (h2 : pc ≤ b.exit) :
    ∃ st stys, refBefore m view view.code.code inv pc = some st ∧
      typesBefore m view pc = some stys ∧
      Typed (view.parameters ++ view.locals) stys (avail pc) st ∧ CodeReachable view.code.code pc := by
  have hok := (buildBlocks_spec view.code.code).1 b hb
  obtain ⟨post, hex, -⟩ := hfix.closed b.start pre hpre
  rw [refExecuteBlock_eq m view hs.blocks hb] at hex
  have hsplit : b.exit + 1 - b.start = (pc - b.start) + (b.exit + 1 - pc) := by omega
  rw [hsplit] at hex
  obtain ⟨st0, h0⟩ := foldlM_prefix hex
  have hbefore := refBlock_prefix m view hok hpre (pc - b.start) st0 (by omega) h0 (by omega)
  obtain ⟨hreach, stys, htys, ht⟩ := refBlock_typed hs hok hJ (pc - b.start) st0 (by omega) h0
    (by omega)
  rw [show b.start + (pc - b.start) = pc by omega] at hbefore hreach htys ht
  exact ⟨st0, stys, hbefore, htys, ht, hreach⟩

end MoveBytecode.Proofs
