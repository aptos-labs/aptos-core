-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Prover.Ivl.Syntax
import Move.Prover.Ivl.Semantics
import Move.Prover.Ivl.WpSound
import Move.IR.Semantics
import Move.IR.TypedCode
import Move.Prover.Translate.Compile

/-!
# Forward Simulation

The translation-correctness direction: every bytecode execution is
*represented* by an execution of the compiled IVL program.  Because the
compiled program contains assertions (e.g. callee preconditions at call
sites), the representation is "up to failure": the compiled program either
reproduces the source outcome through the abort-flag encoding, or it can
fail an assertion — in which case the verification condition is false and
adequacy is unaffected.

Everything is proven through **one master induction** over the big-step
source derivation (`sim_aux`) — which is also how the mutual recursion
between functions is untangled — establishing three things at once:

* the *simulation*: the compiled run, built command by command
  (`ContRun`), with the raised abort flag skipping the rest of a block to
  the abort exit;
* *contract conformance* (`Conforms`): the exit-block assertions recorded
  along the constructed run, translated to the enclosing boundary;
* *type preservation* (`TypedLocals`/`TypedMemory`/`OutTyped`): the
  runtime typing invariant under the source-typing discipline (`WfProg`,
  `IR/TypedCode.lean`), which is what lets a call site discharge the
  callee's `SatisfiesContract` hypotheses and the `callRel` havoc's typing
  conjunct.

At a call, the induction hypothesis simulates the callee from its entry;
prefixing the callee's entry stub (whose `assume`s hold by the call-site
typing and the asserted `requires`) yields a full callee execution, and
`wpB_safe` — from the callee's `Verified` — excludes its failing branch,
forcing the conformance facts that the `callRel` havoc requires.

Since source blocks map 1-1 to IVL blocks (source `b` ↦ label `b + 1`),
the public simulation statement (`compile_simulates`) is block-wise.
-/

namespace Move.Prover.Translate

open Move.Prover.Ivl
open Move.IR

/-- Relation between a source frame outcome and a final verification state:
a normal return is represented with the abort flag down, the same final
memory and the returned values in `rets`; an abort is represented by the
raised flag carrying the code (the post-abort `cur` is irrelevant — Move
discards the state on abort). -/
def OutRel (o : FrameOutcome) (v' : VState) : Prop :=
  match o with
  | .ret m vals => v'.aborted = none ∧ v'.cur.memory = m ∧ v'.rets = vals
  | .abort _ code => v'.aborted = some code

/-- The per-outcome payload of `SatisfiesContract`: what an execution's
outcome must satisfy relative to the boundary `(m, args)`. -/
def Conforms (Δ : StructDecls) (d : FunDecl) (m : Memory)
    (args : List Value) : FrameOutcome → Prop
  | .ret m' rets =>
      Holds (postEnv Δ m m' args rets) d.contract.ensures ∧
      agreesOutside (d.contract.footprint (preEnv Δ m args)) m m' ∧
      d.contract.abortsFalse (preEnv Δ m args)
  | .abort _ _ => d.contract.abortsHolds (preEnv Δ m args)

/-- Type preservation at the boundary: a normal outcome delivers well-typed
memory and well-typed results. -/
def OutTyped (Δ : StructDecls) (d : FunDecl) : FrameOutcome → Prop
  | .ret m' rets => TypedMemory Δ m' ∧ IsValidList Δ d.returns rets
  | .abort _ _ => True

/-! ## Mid-block executions of the compiled program -/

/-- The compiled commands of a block suffix: the remaining instructions
plus the terminator's contribution. -/
def compiledSuffix (P : Program) (rest : List Instr) (t : Term) :
    List (BCmd VState) :=
  (rest.map (compileInstr P)).flatten ++ termCmds t

theorem compiledSuffix_cons (P : Program) (i : Instr) (rest : List Instr)
    (t : Term) :
    compiledSuffix P (i :: rest) t =
      compileInstr P i ++ compiledSuffix P rest t := by
  simp [compiledSuffix]

/-- The compiled terminator routes from the post-commands state into the
rest of the program. -/
def TermRun (Gc : BProgram VState) (size : Nat) (t : Term) (v : VState)
    (o' : Outcome VState) : Prop :=
  ∃ gt ∈ termGoto size t, gt.1 v ∧ BExec Gc gt.2 v o'

/-- A run of the remainder of a compiled block: the commands either fail,
or complete and the terminator routes onward. -/
def ContRun (Gc : BProgram VState) (P : Program) (size : Nat)
    (rest : List Instr) (t : Term) (v : VState) (o' : Outcome VState) :
    Prop :=
  (o' = .fail ∧ CmdsExec (compiledSuffix P rest t) v .fail) ∨
  (∃ v₁, CmdsExec (compiledSuffix P rest t) v (.ok v₁) ∧
    TermRun Gc size t v₁ o')

/-- Prepend a normal command run to a mid-block run. -/
theorem ContRun.prepend {Gc : BProgram VState} {P : Program} {size : Nat}
    {pre : List (BCmd VState)} {rest : List Instr} {t : Term}
    {v v₁ : VState} {o' : Outcome VState}
    (hpre : CmdsExec pre v (.ok v₁))
    (h : ContRun Gc P size rest t v₁ o')
    {i : Instr} (hcomp : compileInstr P i = pre) :
    ContRun Gc P size (i :: rest) t v o' := by
  subst hcomp
  rcases h with ⟨rfl, hfail⟩ | ⟨v₂, hok, hterm⟩
  · exact .inl ⟨rfl, by
      rw [compiledSuffix_cons]
      exact hpre.append_ok hfail⟩
  · exact .inr ⟨v₂, by
      rw [compiledSuffix_cons]
      exact hpre.append_ok hok, hterm⟩

/-- A failing command run of the compiled instruction fails the block. -/
theorem ContRun.failPrefix {Gc : BProgram VState} {P : Program} {size : Nat}
    {rest : List Instr} {t : Term} {v : VState} {i : Instr}
    (hfail : CmdsExec (compileInstr P i) v .fail) :
    ContRun Gc P size (i :: rest) t v .fail := by
  refine .inl ⟨rfl, ?_⟩
  rw [compiledSuffix_cons]
  exact hfail.append_fail

/-- A block-initial mid-block run is an execution of the block's label. -/
theorem ContRun.toBExec {Gc : BProgram VState} {P : Program} {size : Nat}
    {blk : Block} {l : Label} {v : VState} {o' : Outcome VState}
    (hblk : Gc.blocks l = some (compileBlock P size blk))
    (h : ContRun Gc P size blk.instrs blk.term v o') :
    BExec Gc l v o' := by
  rcases h with ⟨rfl, hfail⟩ | ⟨v₁, hok, gt, hmem, hg, hnext⟩
  · exact .fail hblk hfail
  · exact .goto hblk hok hmem hg hnext

/-! ## Labels of the compiled program -/

theorem compileFun_blocks_src (P : Program) (d : FunDecl) {b : BlockId}
    {blk : Block} (hlt : b < d.body.size)
    (hb : d.body.blocks b = some blk) :
    (compileFun P d).blocks (b + 1) =
      some (compileBlock P d.body.size blk) := by
  have h1 : b ≠ d.body.size := Nat.ne_of_lt hlt
  have h2 : b ≠ d.body.size + 1 := by
    intro h
    subst h
    exact Nat.lt_irrefl _ (Nat.lt_of_succ_lt hlt)
  simp [compileFun, h1, h2, hb]

theorem compileFun_blocks_retExit (P : Program) (d : FunDecl) :
    (compileFun P d).blocks (d.body.size + 1) =
      some (retExitBlock P.structs d.contract) := by
  simp [compileFun]

theorem compileFun_blocks_abortExit (P : Program) (d : FunDecl) :
    (compileFun P d).blocks (d.body.size + 2) =
      some (abortExitBlock P.structs d.contract) := by
  simp [compileFun]

/-- The abort exit is a member of every compiled terminator's targets,
guarded by the raised flag. -/
theorem termGoto_abortExit (size : Nat) (t : Term) :
    (flagSet, size + 2) ∈ termGoto size t := by
  cases t <;> simp [termGoto]

/-- `compAnns` never annotates label 0 (the entry stub), so `wpB_sound`'s
start condition holds there. -/
theorem compAnns_start (P : Program) (d : FunDecl) :
    ∀ h ann, compAnns P d h = some ann → ann.members 0 → h = 0 := by
  intro h ann hh hmem
  match h, hh with
  | 0, _ => rfl
  | b + 1, hh =>
    simp only [compAnns, Option.map_eq_some_iff] at hh
    obtain ⟨ls, -, rfl⟩ := hh
    obtain ⟨b', hb', -⟩ := hmem
    cases hb'

/-! ## Passing compiled code with a raised flag -/

/-- A command a flag-set state passes through unchanged (asserts are
handled separately: they pass or fail). -/
def SkipCmd (c : BCmd VState) : Prop :=
  ∀ v : VState, v.aborted.isSome →
    match c with
    | .assign f => f v = v
    | .havoc R => R v v
    | .assume p => p v
    | .assert _ => True

theorem skipCmd_onOk (f : VState → VState) : SkipCmd (onOk f) := by
  intro v h
  simp [onOk, h]

theorem skipCmd_assert (p : VState → Prop) : SkipCmd (.assert p) := by
  intro v h
  trivial

theorem skipCmd_callRel (Δ : StructDecls) (d : FunDecl)
    (dsts srcs : List LocalIndex) :
    SkipCmd (.havoc (callRel Δ d dsts srcs)) := by
  intro v h
  simp [callRel, h]

theorem compileInstr_skips (P : Program) (i : Instr) :
    ∀ c ∈ compileInstr P i, SkipCmd c := by
  cases i with
  | load dst val =>
    intro c hc
    simp only [compileInstr, List.mem_singleton] at hc
    subst hc; exact skipCmd_onOk _
  | assign dst src =>
    intro c hc
    simp only [compileInstr, List.mem_singleton] at hc
    subst hc; exact skipCmd_onOk _
  | nop => intro c hc; simp [compileInstr] at hc
  | call dsts op srcs =>
    cases op with
    | function f =>
      intro c hc
      cases hfd : P.funs f with
      | none => rw [compileInstr, hfd] at hc; simp at hc
      | some d =>
        rw [compileInstr, hfd] at hc
        simp only [List.mem_cons, List.not_mem_nil, or_false] at hc
        rcases hc with rfl | rfl
        · exact skipCmd_assert _
        · exact skipCmd_callRel _ _ _ _
    | borrowLoc =>
      intro c hc
      simp only [compileInstr, refFail, List.mem_singleton] at hc
      subst hc; exact skipCmd_assert _
    | borrowField i =>
      intro c hc
      simp only [compileInstr, refFail, List.mem_singleton] at hc
      subst hc; exact skipCmd_assert _
    | borrowGlobal r =>
      intro c hc
      simp only [compileInstr, refFail, List.mem_singleton] at hc
      subst hc; exact skipCmd_assert _
    | borrowVecElem =>
      intro c hc
      simp only [compileInstr, refFail, List.mem_singleton] at hc
      subst hc; exact skipCmd_assert _
    | readRef =>
      intro c hc
      simp only [compileInstr, refFail, List.mem_singleton] at hc
      subst hc; exact skipCmd_assert _
    | writeRef =>
      intro c hc
      simp only [compileInstr, refFail, List.mem_singleton] at hc
      subst hc; exact skipCmd_assert _
    | freezeRef =>
      intro c hc
      simp only [compileInstr, refFail, List.mem_singleton] at hc
      subst hc; exact skipCmd_assert _
    | add => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
             subst hc; exact skipCmd_onOk _
    | sub => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
             subst hc; exact skipCmd_onOk _
    | mul => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
             subst hc; exact skipCmd_onOk _
    | div => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
             subst hc; exact skipCmd_onOk _
    | mod => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
             subst hc; exact skipCmd_onOk _
    | lt => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
            subst hc; exact skipCmd_onOk _
    | le => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
            subst hc; exact skipCmd_onOk _
    | eq => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
            subst hc; exact skipCmd_onOk _
    | and => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
             subst hc; exact skipCmd_onOk _
    | or => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
            subst hc; exact skipCmd_onOk _
    | not => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
             subst hc; exact skipCmd_onOk _
    | pack => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
              subst hc; exact skipCmd_onOk _
    | unpack => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
                subst hc; exact skipCmd_onOk _
    | getField i => intro c hc
                    simp only [compileInstr, List.mem_singleton] at hc
                    subst hc; exact skipCmd_onOk _
    | updateField i => intro c hc
                       simp only [compileInstr, List.mem_singleton] at hc
                       subst hc; exact skipCmd_onOk _
    | vecPack => intro c hc
                 simp only [compileInstr, List.mem_singleton] at hc
                 subst hc; exact skipCmd_onOk _
    | vecLen => intro c hc
                simp only [compileInstr, List.mem_singleton] at hc
                subst hc; exact skipCmd_onOk _
    | vecGet => intro c hc
                simp only [compileInstr, List.mem_singleton] at hc
                subst hc; exact skipCmd_onOk _
    | vecSet => intro c hc
                simp only [compileInstr, List.mem_singleton] at hc
                subst hc; exact skipCmd_onOk _
    | vecPush => intro c hc
                 simp only [compileInstr, List.mem_singleton] at hc
                 subst hc; exact skipCmd_onOk _
    | vecPop => intro c hc
                simp only [compileInstr, List.mem_singleton] at hc
                subst hc; exact skipCmd_onOk _
    | mkMutLoc x => intro c hc
                    simp only [compileInstr, List.mem_singleton] at hc
                    subst hc; exact skipCmd_onOk _
    | mkMutGlobal r => intro c hc
                       simp only [compileInstr, List.mem_singleton] at hc
                       subst hc; exact skipCmd_onOk _
    | childMutField i => intro c hc
                         simp only [compileInstr, List.mem_singleton] at hc
                         subst hc; exact skipCmd_onOk _
    | childMutIndex => intro c hc
                       simp only [compileInstr, List.mem_singleton] at hc
                       subst hc; exact skipCmd_onOk _
    | getMut => intro c hc
                simp only [compileInstr, List.mem_singleton] at hc
                subst hc; exact skipCmd_onOk _
    | setMut => intro c hc
                simp only [compileInstr, List.mem_singleton] at hc
                subst hc; exact skipCmd_onOk _
    | isParent pat => intro c hc
                      simp only [compileInstr, List.mem_singleton] at hc
                      subst hc; exact skipCmd_onOk _
    | mutPathIndex k => intro c hc
                        simp only [compileInstr, List.mem_singleton] at hc
                        subst hc; exact skipCmd_onOk _
    | isMutLoc x => intro c hc
                    simp only [compileInstr, List.mem_singleton] at hc
                    subst hc; exact skipCmd_onOk _
    | isMutGlobal r => intro c hc
                       simp only [compileInstr, List.mem_singleton] at hc
                       subst hc; exact skipCmd_onOk _
    | mutAddr => intro c hc
                 simp only [compileInstr, List.mem_singleton] at hc
                 subst hc; exact skipCmd_onOk _
    | getGlobal r => intro c hc
                     simp only [compileInstr, List.mem_singleton] at hc
                     subst hc; exact skipCmd_onOk _
    | writeGlobal r => intro c hc
                       simp only [compileInstr, List.mem_singleton] at hc
                       subst hc; exact skipCmd_onOk _
    | moveTo r => intro c hc
                  simp only [compileInstr, List.mem_singleton] at hc
                  subst hc; exact skipCmd_onOk _
    | moveFrom r => intro c hc
                    simp only [compileInstr, List.mem_singleton] at hc
                    subst hc; exact skipCmd_onOk _
    | exists_ r => intro c hc
                   simp only [compileInstr, List.mem_singleton] at hc
                   subst hc; exact skipCmd_onOk _

theorem termCmds_skips (t : Term) : ∀ c ∈ termCmds t, SkipCmd c := by
  cases t with
  | jump b => intro c hc; simp [termCmds] at hc
  | branch c b₁ b₂ => intro c hc; simp [termCmds] at hc
  | ret srcs =>
    intro c hc
    simp only [termCmds, List.mem_singleton] at hc
    subst hc; exact skipCmd_onOk _
  | abort code =>
    intro c hc
    simp only [termCmds, List.mem_singleton] at hc
    subst hc; exact skipCmd_onOk _

/-- A flag-set state passes any list of skip commands unchanged — or fails
an assertion on the way. -/
theorem skips_run {cs : List (BCmd VState)} (h : ∀ c ∈ cs, SkipCmd c)
    {v : VState} (hab : v.aborted.isSome) :
    CmdsExec cs v (.ok v) ∨ CmdsExec cs v .fail := by
  induction cs with
  | nil => exact .inl .nil
  | cons c cs ih =>
    have hc := h c (by simp)
    have hrest := ih fun c' hc' => h c' (by simp [hc'])
    cases c with
    | assign f =>
      have hf : f v = v := hc v hab
      rcases hrest with hok | hfail
      · exact .inl (.assign (by rw [hf]; exact hok))
      · exact .inr (.assign (by rw [hf]; exact hfail))
    | havoc R =>
      have hR : R v v := hc v hab
      rcases hrest with hok | hfail
      · exact .inl (.havoc hR hok)
      · exact .inr (.havoc hR hfail)
    | assume p =>
      have hp : p v := hc v hab
      rcases hrest with hok | hfail
      · exact .inl (.assume hp hok)
      · exact .inr (.assume hp hfail)
    | assert p =>
      by_cases hp : p v
      · rcases hrest with hok | hfail
        · exact .inl (.assertOk hp hok)
        · exact .inr (.assertOk hp hfail)
      · exact .inr (.assertFail hp)

/-- The compiled suffix of a block consists of skip commands. -/
theorem compiledSuffix_skips (P : Program) (rest : List Instr) (t : Term) :
    ∀ c ∈ compiledSuffix P rest t, SkipCmd c := by
  intro c hc
  rw [compiledSuffix, List.mem_append] at hc
  rcases hc with hc | hc
  · rw [List.mem_flatten] at hc
    obtain ⟨l, hl, hcl⟩ := hc
    rw [List.mem_map] at hl
    obtain ⟨i, -, rfl⟩ := hl
    exact compileInstr_skips P i c hcl
  · exact termCmds_skips t c hc

/-- The abort-exit edge: from a flag-set state at a compiled terminator,
route to the abort exit and pass (recording its assertion) or fail it. -/
theorem abortExit_run (P : Program) (d : FunDecl) (t : Term) {v : VState}
    (habs : v.aborted.isSome) :
    ∃ o', TermRun (compileFun P d) d.body.size t v o' ∧
      (o' = .fail ∨
        (o' = .ok v ∧ d.contract.abortsHolds (v.preEnvOf P.structs))) := by
  by_cases hassert : d.contract.abortsHolds (v.preEnvOf P.structs)
  · refine ⟨.ok v, ⟨(flagSet, d.body.size + 2),
      termGoto_abortExit _ t, habs, ?_⟩, .inr ⟨rfl, hassert⟩⟩
    exact .ret (compileFun_blocks_abortExit P d) (.assertOk hassert .nil)
  · refine ⟨.fail, ⟨(flagSet, d.body.size + 2),
      termGoto_abortExit _ t, habs, ?_⟩, .inl rfl⟩
    exact .fail (compileFun_blocks_abortExit P d) (.assertFail hassert)

/-- **The abort path**: from a flag-set state anywhere in a block, the
compiled program reaches the abort exit — recording its assertion — or
fails an assertion on the way. -/
theorem abortPath (P : Program) (d : FunDecl) {rest : List Instr} {t : Term}
    {v : VState} {code : Nat} (hab : v.aborted = some code) :
    ∃ o', ContRun (compileFun P d) P d.body.size rest t v o' ∧
      (o' = .fail ∨
        (o' = .ok v ∧ d.contract.abortsHolds (v.preEnvOf P.structs))) := by
  have habs : v.aborted.isSome := by simp [hab]
  rcases skips_run (compiledSuffix_skips P rest t) habs with hok | hfail
  · obtain ⟨o', hterm, hfact⟩ := abortExit_run P d t habs
    exact ⟨o', .inr ⟨v, hok, hterm⟩, hfact⟩
  · exact ⟨.fail, .inl ⟨rfl, hfail⟩, .inl rfl⟩

/-! ## Stepping compiled instructions from a flag-clear state -/

/-- Execute one `onOk` command from a flag-clear state. -/
theorem onOk_step {f : VState → VState} {v : VState}
    (hok : v.aborted = none) {cs : List (BCmd VState)} {o : Outcome VState}
    (hrest : CmdsExec cs (f v) o) :
    CmdsExec (onOk f :: cs) v o := by
  refine .assign ?_
  simpa [hok] using hrest

/-- The single-command run of `onOk`. -/
theorem onOk_run {f : VState → VState} {v : VState}
    (hok : v.aborted = none) :
    CmdsExec [onOk f] v (.ok (f v)) :=
  onOk_step hok .nil

/-- `Oper.sem` is undefined on function calls (handled relationally). -/
theorem Oper.sem_function_none (deref : RefTarget → Option Value)
    (f : FunId) (vs : List Value) (m : Memory) :
    Oper.sem deref (.function f) vs m = none := by
  match vs with
  | [] => rfl
  | [_] => rfl
  | [_, _] => rfl
  | _ :: _ :: _ :: _ => rfl

/-- `Oper.sem` is undefined on the reference operations (handled by
`refFail`). -/
theorem Oper.sem_refOp_none {op : Oper} (h : op.isRefOp)
    (deref : RefTarget → Option Value) (vs : List Value) (m : Memory) :
    Oper.sem deref op vs m = none := by
  cases op <;> simp [Oper.isRefOp] at h <;>
    match vs with
    | [] => rfl
    | [_] => rfl
    | [_, _] => rfl
    | _ :: _ :: _ :: _ => rfl



/-- `compileInstr` on a value operation (neither a call nor a reference
operation) is the single guarded update. -/
theorem compileInstr_op (P : Program) (dsts srcs : List LocalIndex)
    {op : Oper} (hfun : ∀ f, op ≠ .function f) (href : ¬ op.isRefOp) :
    compileInstr P (.call dsts op srcs) =
      [onOk fun v =>
        match srcs.mapM v.cur.locals with
        | some vs =>
          match op.sem v.cur.readTarget vs v.cur.memory with
          | some (.ok rets m') =>
              if dsts.length = rets.length then
                { v with
                  cur := MoveState.writeLocals ⟨v.cur.locals, m'⟩ dsts rets }
              else v
          | some .abort => v.doAbort runtimeAbortCode
          | none => v
        | none => v] := by
  cases op <;> first
    | rfl
    | (exact absurd rfl (hfun _))
    | (exact absurd rfl href)

/-! ## The master simulation induction -/

/-- **The master induction** (simulation, contract conformance, and type
preservation, fused): a terminating source execution from a mid-block
position — under the ambient hypotheses that the program is well-typed,
every function's VC holds, and every compiled program satisfies the loop
side conditions — is represented by a run of the compiled program: either
an assertion failure, or a faithful run whose exit assertions record the
enclosing function's contract conformance at its boundary `(m₀, args₀)`,
with well-typed results. -/
theorem sim_aux (P : Program)
    (hwfP : WfProg P)
    (hver : ∀ f d, P.funs f = some d → Verified P f)
    (hanns : ∀ f d, P.funs f = some d →
      WfProgram (compileFun P d) (compAnns P d) (fun l => l)) :
    ∀ {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {o : FrameOutcome},
    RunFrom P G rest term s o →
    ∀ (d : FunDecl), (∃ f, P.funs f = some d) → G = d.body →
    (∀ i ∈ rest, WfInstr P d.locals i) → WfTerm P.structs d term →
    ∀ (v : VState) (m₀ : Memory) (args₀ : List Value),
      v.cur = s → v.aborted = none →
      v.snaps = (fun _ => m₀) → v.args = args₀ →
      TypedLocals P.structs d.locals s.locals →
      TypedMemory P.structs s.memory →
    ∃ o', ContRun (compileFun P d) P d.body.size rest term v o' ∧
      (o' = .fail ∨ ∃ v', o' = .ok v' ∧ OutRel o v' ∧
        v'.snaps = v.snaps ∧ v'.args = v.args ∧
        Conforms P.structs d m₀ args₀ o ∧ OutTyped P.structs d o) := by
  intro G rest term s o hrun
  induction hrun with
  | @load G rest term s dst val o hrest ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    have hwf := hwfR _ List.mem_cons_self
    have hTL' : TypedLocals P.structs d.locals (s.writeLocal dst val).locals := by
      cases hwf with
      | load ht hval =>
        exact hTL.writeLocal fun t' ht' => by
          rw [ht] at ht'; cases ht'; exact hval
    obtain ⟨o', hcont, hfacts⟩ :=
      ih d hd hG (fun i hi => hwfR i (List.mem_cons_of_mem _ hi)) hwfT
        { v with cur := s.writeLocal dst val } m₀ args₀
        rfl hok hsnaps hargs hTL' hTM
    refine ⟨o', ContRun.prepend ?_ hcont rfl, hfacts⟩
    have hstep := onOk_run (v := v)
      (f := fun v => { v with cur := v.cur.writeLocal dst val }) hok
    simp only [hcur] at hstep
    exact hstep
  | @assign G rest term s dst src val o hsrc hrest ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    have hwf := hwfR _ List.mem_cons_self
    have hTL' : TypedLocals P.structs d.locals (s.writeLocal dst val).locals := by
      cases hwf with
      | assign hts htd hsub =>
        exact hTL.writeLocal fun t' ht' => by
          rw [htd] at ht'; cases ht'
          exact hsub _ (hTL _ _ _ hts hsrc)
    obtain ⟨o', hcont, hfacts⟩ :=
      ih d hd hG (fun i hi => hwfR i (List.mem_cons_of_mem _ hi)) hwfT
        { v with cur := s.writeLocal dst val } m₀ args₀
        rfl hok hsnaps hargs hTL' hTM
    refine ⟨o', ContRun.prepend ?_ hcont rfl, hfacts⟩
    have hstep := onOk_run (v := v)
      (f := fun v => match v.cur.locals src with
        | some val => { v with cur := v.cur.writeLocal dst val }
        | none => v) hok
    simp only [hcur, hsrc] at hstep
    exact hstep
  | @nop G rest term s o hrest ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    obtain ⟨o', hcont, hfacts⟩ :=
      ih d hd hG (fun i hi => hwfR i (List.mem_cons_of_mem _ hi)) hwfT
        v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨o', ContRun.prepend .nil hcont rfl, hfacts⟩
  | @opOk G rest term s dsts srcs op vs rets m' o hsrcs hlen hop hrest ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    have hwf := hwfR _ List.mem_cons_self
    cases hwf with
    | op hsts hdts hwfop hsub hsubr =>
      have hvs := (hTL.mapM_isValidList hsrcs hsts).sub hsub
      obtain ⟨hrets, hTM'⟩ := hwfop.sem_preserves hvs hTM hop
      have hTL' := hTL.writeLocals (s := ⟨s.locals, m'⟩) hdts (hrets.sub hsubr)
      obtain ⟨o', hcont, hfacts⟩ :=
        ih d hd hG (fun i hi => hwfR i (List.mem_cons_of_mem _ hi)) hwfT
          { v with cur := MoveState.writeLocals ⟨s.locals, m'⟩ dsts rets }
          m₀ args₀ rfl hok hsnaps hargs hTL'
          (by rw [writeLocals_memory]; exact hTM')
      have hfun : ∀ f', op ≠ .function f' := by
        intro f' h
        subst h
        rw [Oper.sem_function_none] at hop
        cases hop
      have href : ¬ op.isRefOp := by
        intro h
        rw [Oper.sem_refOp_none h] at hop
        cases hop
      refine ⟨o', ContRun.prepend ?_ hcont
        (compileInstr_op P dsts srcs hfun href), hfacts⟩
      have hstep := onOk_run (v := v)
        (f := fun v => match srcs.mapM v.cur.locals with
          | some vs =>
            match op.sem v.cur.readTarget vs v.cur.memory with
            | some (.ok rets m') =>
                if dsts.length = rets.length then
                  { v with
                    cur := MoveState.writeLocals ⟨v.cur.locals, m'⟩ dsts rets }
                else v
            | some .abort => v.doAbort runtimeAbortCode
            | none => v
          | none => v) hok
      simp only [hcur, hsrcs, hop, if_pos hlen] at hstep
      exact hstep
    | callFun hd' hsts hlen' hsubp hdts hsubr =>
      exact absurd hop (by rw [Oper.sem_function_none]; simp)
    | refInstr href =>
      exact absurd hop (by rw [Oper.sem_refOp_none href]; simp)
  | @opAbort G rest term s dsts srcs op vs hsrcs hop =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    have hfun : ∀ f', op ≠ .function f' := by
      intro f' h
      subst h
      rw [Oper.sem_function_none] at hop
      cases hop
    have href : ¬ op.isRefOp := by
      intro h
      rw [Oper.sem_refOp_none h] at hop
      cases hop
    obtain ⟨o', hcont, hfact⟩ :=
      abortPath P d (rest := rest) (t := term)
        (v := v.doAbort runtimeAbortCode) (code := runtimeAbortCode) rfl
    refine ⟨o', ContRun.prepend ?_ hcont
      (compileInstr_op P dsts srcs hfun href), ?_⟩
    · have hstep := onOk_run (v := v)
        (f := fun v => match srcs.mapM v.cur.locals with
          | some vs =>
            match op.sem v.cur.readTarget vs v.cur.memory with
            | some (.ok rets m') =>
                if dsts.length = rets.length then
                  { v with
                    cur := MoveState.writeLocals ⟨v.cur.locals, m'⟩ dsts rets }
                else v
            | some .abort => v.doAbort runtimeAbortCode
            | none => v
          | none => v) hok
      simp only [hcur, hsrcs, hop] at hstep
      exact hstep
    · rcases hfact with rfl | ⟨rfl, hholds⟩
      · exact .inl rfl
      · refine .inr ⟨_, rfl, rfl, rfl, rfl, ?_, trivial⟩
        show d.contract.abortsHolds (preEnv P.structs m₀ args₀)
        simpa [VState.preEnvOf, VState.doAbort, hsnaps, hargs] using hholds
  | borrowLoc hx hrest ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨.fail, ContRun.failPrefix (.assertFail fun h => h), .inl rfl⟩
  | borrowField ht hrest ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨.fail, ContRun.failPrefix (.assertFail fun h => h), .inl rfl⟩
  | borrowGlobalOk ha hpresent hrest ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨.fail, ContRun.failPrefix (.assertFail fun h => h), .inl rfl⟩
  | borrowGlobalAbort ha habsent =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨.fail, ContRun.failPrefix (.assertFail fun h => h), .inl rfl⟩
  | borrowVecElemOk ht hv hi hlt hrest ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨.fail, ContRun.failPrefix (.assertFail fun h => h), .inl rfl⟩
  | borrowVecElemAbort ht hv hi hge =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨.fail, ContRun.failPrefix (.assertFail fun h => h), .inl rfl⟩
  | readRef ht hv hrest ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨.fail, ContRun.failPrefix (.assertFail fun h => h), .inl rfl⟩
  | writeRef ht hv hs' hrest ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨.fail, ContRun.failPrefix (.assertFail fun h => h), .inl rfl⟩
  | freezeRef ht hrest ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨.fail, ContRun.failPrefix (.assertFail fun h => h), .inl rfl⟩
  | @callOk G rest term s dsts srcs f₂ d₂ args retVals blk m' o hd₂ hargs hnoref hentry hcallee hlen hrest ihcallee ihrest =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hvargs hTL hTM
    have hwf := hwfR _ List.mem_cons_self
    cases hwf with
    | op hsts hdts hwfop hsub hsubr => cases hwfop
    | refInstr href => simp [Oper.isRefOp] at href
    | @callFun _ _ _ _ sts dts hd₂' hsts hlenp hsubp hdts hsubr =>
      rw [hd₂] at hd₂'
      injection hd₂' with hdd
      subst hdd
      by_cases hreq : callRequires P.structs d₂.contract srcs v
      case neg =>
        refine ⟨.fail, ContRun.failPrefix ?_, .inl rfl⟩
        rw [show compileInstr P (.call dsts (.function f₂) srcs) =
            [.assert (callRequires P.structs d₂.contract srcs),
             .havoc (callRel P.structs d₂ dsts srcs)] by
          simp [compileInstr, hd₂]]
        exact .assertFail hreq
      case pos =>
        obtain ⟨args', hargs', hreqH⟩ := hreq hok
        rw [hcur, hargs] at hargs'
        injection hargs' with hargs'
        subst hargs'
        have hvsts := hTL.mapM_isValidList hargs hsts
        have hargsTyped : TypedArgs P.structs d₂ args := by
          refine ⟨hvsts.length.trans hlenp, ?_⟩
          intro i t w ht hw
          obtain ⟨t', ht', hval⟩ := hvsts.getElem? hw
          exact hsubp i t' t ht' ht _ hval
        have hwfd₂ := hwfP f₂ d₂ hd₂
        obtain ⟨oc, hcontc, hfactsc⟩ :=
          ihcallee d₂ ⟨f₂, hd₂⟩ rfl (hwfd₂.wfInstr _ _ hentry)
            (hwfd₂.wfTerm _ _ hentry)
            (initVState s.memory args) s.memory args rfl rfl rfl rfl
            (TypedLocals.initLocals hargsTyped) hTM
        have hentrylt : d₂.body.entry < d₂.body.size :=
          hwfd₂.blocksLt _ (by simp [hentry])
        have hreqE : Holds
            ((initVState s.memory args).preEnvOf P.structs)
            d₂.contract.requires := by
          rw [hcur] at hreqH
          exact hreqH
        have hblk0 : (compileFun P d₂).blocks 0 =
            some ⟨[.assume (typedEntry P d₂),
                   .assume fun v => Holds (v.preEnvOf P.structs)
                     d₂.contract.requires],
                  .goto [(fun _ => True, d₂.body.entry + 1)]⟩ := by
          simp [compileFun]
        have hbex : BExec (compileFun P d₂) 0 (initVState s.memory args) oc :=
          BExec.goto (gt := (fun _ => True, d₂.body.entry + 1)) hblk0
            (.assume ⟨hargsTyped, hTM⟩ (.assume hreqE .nil))
            (List.mem_singleton.mpr rfl) trivial
            (ContRun.toBExec (compileFun_blocks_src P d₂ hentrylt hentry)
              hcontc)
        obtain ⟨d₂'', hd₂'', fuel, hwp⟩ := hver f₂ d₂ hd₂
        rw [hd₂] at hd₂''
        injection hd₂'' with hdd
        subst hdd
        rcases hfactsc with rfl | ⟨vc', rfl, hrelc, hsnc, hargc, hconfc, htypc⟩
        · exact absurd hbex
            (wpB_safe (hanns f₂ d₂ hd₂) (compAnns_start P d₂)
              (hwp s.memory args))
        · obtain ⟨hens, hagree, habf⟩ := hconfc
          obtain ⟨hTMm', hretsV⟩ := htypc
          have hcallrel : callRel P.structs d₂ dsts srcs v
              { v with
                cur := MoveState.writeLocals ⟨s.locals, m'⟩ dsts retVals } := by
            unfold callRel
            rw [if_neg (by simp [hok])]
            refine ⟨args, by rw [hcur]; exact hargs,
              .inr ⟨m', retVals, hlen.symm, ?_, ?_, ?_, ?_, by rw [hcur]⟩⟩
            · rw [hcur]; exact habf
            · rw [hcur]; exact hens
            · rw [hcur]; exact hagree
            · intro _; exact ⟨hTMm', hretsV⟩
          have hTL' := hTL.writeLocals (s := ⟨s.locals, m'⟩) hdts
            (hretsV.sub hsubr)
          obtain ⟨o', hcont, hfacts⟩ :=
            ihrest d hd hG (fun i hi => hwfR i (List.mem_cons_of_mem _ hi))
              hwfT
              { v with
                cur := MoveState.writeLocals ⟨s.locals, m'⟩ dsts retVals }
              m₀ args₀ rfl hok hsnaps hvargs hTL'
              (by rw [writeLocals_memory]; exact hTMm')
          refine ⟨o', ContRun.prepend
            (CmdsExec.assertOk hreq (CmdsExec.havoc hcallrel .nil)) hcont
            (by simp [compileInstr, hd₂]), hfacts⟩
  | @callAbort G rest term s dsts srcs f₂ d₂ args blk m' code hd₂ hargs hnoref hentry hcallee ihcallee =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hvargs hTL hTM
    have hwf := hwfR _ List.mem_cons_self
    cases hwf with
    | op hsts hdts hwfop hsub hsubr => cases hwfop
    | refInstr href => simp [Oper.isRefOp] at href
    | @callFun _ _ _ _ sts dts hd₂' hsts hlenp hsubp hdts hsubr =>
      rw [hd₂] at hd₂'
      injection hd₂' with hdd
      subst hdd
      by_cases hreq : callRequires P.structs d₂.contract srcs v
      case neg =>
        refine ⟨.fail, ContRun.failPrefix ?_, .inl rfl⟩
        rw [show compileInstr P (.call dsts (.function f₂) srcs) =
            [.assert (callRequires P.structs d₂.contract srcs),
             .havoc (callRel P.structs d₂ dsts srcs)] by
          simp [compileInstr, hd₂]]
        exact .assertFail hreq
      case pos =>
        obtain ⟨args', hargs', hreqH⟩ := hreq hok
        rw [hcur, hargs] at hargs'
        injection hargs' with hargs'
        subst hargs'
        have hvsts := hTL.mapM_isValidList hargs hsts
        have hargsTyped : TypedArgs P.structs d₂ args := by
          refine ⟨hvsts.length.trans hlenp, ?_⟩
          intro i t w ht hw
          obtain ⟨t', ht', hval⟩ := hvsts.getElem? hw
          exact hsubp i t' t ht' ht _ hval
        have hwfd₂ := hwfP f₂ d₂ hd₂
        obtain ⟨oc, hcontc, hfactsc⟩ :=
          ihcallee d₂ ⟨f₂, hd₂⟩ rfl (hwfd₂.wfInstr _ _ hentry)
            (hwfd₂.wfTerm _ _ hentry)
            (initVState s.memory args) s.memory args rfl rfl rfl rfl
            (TypedLocals.initLocals hargsTyped) hTM
        have hentrylt : d₂.body.entry < d₂.body.size :=
          hwfd₂.blocksLt _ (by simp [hentry])
        have hreqE : Holds
            ((initVState s.memory args).preEnvOf P.structs)
            d₂.contract.requires := by
          rw [hcur] at hreqH
          exact hreqH
        have hblk0 : (compileFun P d₂).blocks 0 =
            some ⟨[.assume (typedEntry P d₂),
                   .assume fun v => Holds (v.preEnvOf P.structs)
                     d₂.contract.requires],
                  .goto [(fun _ => True, d₂.body.entry + 1)]⟩ := by
          simp [compileFun]
        have hbex : BExec (compileFun P d₂) 0 (initVState s.memory args) oc :=
          BExec.goto (gt := (fun _ => True, d₂.body.entry + 1)) hblk0
            (.assume ⟨hargsTyped, hTM⟩ (.assume hreqE .nil))
            (List.mem_singleton.mpr rfl) trivial
            (ContRun.toBExec (compileFun_blocks_src P d₂ hentrylt hentry)
              hcontc)
        obtain ⟨d₂'', hd₂'', fuel, hwp⟩ := hver f₂ d₂ hd₂
        rw [hd₂] at hd₂''
        injection hd₂'' with hdd
        subst hdd
        rcases hfactsc with rfl | ⟨vc', rfl, hrelc, hsnc, hargc, hconfc, htypc⟩
        · exact absurd hbex
            (wpB_safe (hanns f₂ d₂ hd₂) (compAnns_start P d₂)
              (hwp s.memory args))
        · have hcallrel : callRel P.structs d₂ dsts srcs v (v.doAbort code) := by
            unfold callRel
            rw [if_neg (by simp [hok])]
            refine ⟨args, by rw [hcur]; exact hargs,
              .inl ⟨code, ?_, rfl⟩⟩
            rw [hcur]
            exact hconfc
          obtain ⟨o', hcont, hfact⟩ :=
            abortPath P d (rest := rest) (t := term) (v := v.doAbort code)
              (code := code) rfl
          refine ⟨o', ContRun.prepend
            (CmdsExec.assertOk hreq (CmdsExec.havoc hcallrel .nil)) hcont
            (by simp [compileInstr, hd₂]), ?_⟩
          rcases hfact with rfl | ⟨rfl, hholds⟩
          · exact .inl rfl
          · refine .inr ⟨_, rfl, rfl, rfl, rfl, ?_, trivial⟩
            show d.contract.abortsHolds (preEnv P.structs m₀ args₀)
            simpa [VState.preEnvOf, VState.doAbort, hsnaps, hvargs] using hholds
  -- checkout calls pass loc-rooted reference values, which are never
  -- `IsValid` — excluded at well-typed call sites
  | callRefOk hd₂ hargs hasref hchecked hentry hcallee hflen hwb hreroot hlen hrest ihcallee ihrest =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hvargs hTL hTM
    have hwf := hwfR _ List.mem_cons_self
    cases hwf with
    | op hsts hdts hwfop hsub hsubr => cases hwfop
    | refInstr href => simp [Oper.isRefOp] at href
    | callFun hd₂' hsts hlenp hsubp hdts hsubr =>
      exact absurd
        ((hTL.mapM_isValidList hargs hsts).locRefTargets_eq_nil) hasref
  | callRefAbort hd₂ hargs hasref hchecked hentry hcallee ihcallee =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hvargs hTL hTM
    have hwf := hwfR _ List.mem_cons_self
    cases hwf with
    | op hsts hdts hwfop hsub hsubr => cases hwfop
    | refInstr href => simp [Oper.isRefOp] at href
    | callFun hd₂' hsts hlenp hsubp hdts hsubr =>
      exact absurd
        ((hTL.mapM_isValidList hargs hsts).locRefTargets_eq_nil) hasref
  | @jump G s b blk o hb hnext ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    subst hG
    obtain ⟨f, hf⟩ := hd
    have hwfd := hwfP f d hf
    have hlt : b < d.body.size := hwfd.blocksLt b (by simp [hb])
    obtain ⟨o', hcont, hfacts⟩ :=
      ih d ⟨f, hf⟩ rfl (hwfd.wfInstr b blk hb) (hwfd.wfTerm b blk hb)
        v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨o', .inr ⟨v, .nil, (flagClear, b + 1), by simp [termGoto], hok,
      ContRun.toBExec (compileFun_blocks_src P d hlt hb) hcont⟩, hfacts⟩
  | @branchTrue G s c b₁ b₂ blk o hc hb hnext ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    subst hG
    obtain ⟨f, hf⟩ := hd
    have hwfd := hwfP f d hf
    have hlt : b₁ < d.body.size := hwfd.blocksLt b₁ (by simp [hb])
    obtain ⟨o', hcont, hfacts⟩ :=
      ih d ⟨f, hf⟩ rfl (hwfd.wfInstr b₁ blk hb) (hwfd.wfTerm b₁ blk hb)
        v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨o', .inr ⟨v, .nil,
      (fun v => v.aborted = none ∧ v.cur.locals c = some (.bool true), b₁ + 1),
      by simp [termGoto], ⟨hok, by rw [hcur]; exact hc⟩,
      ContRun.toBExec (compileFun_blocks_src P d hlt hb) hcont⟩, hfacts⟩
  | @branchFalse G s c b₁ b₂ blk o hc hb hnext ih =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    subst hG
    obtain ⟨f, hf⟩ := hd
    have hwfd := hwfP f d hf
    have hlt : b₂ < d.body.size := hwfd.blocksLt b₂ (by simp [hb])
    obtain ⟨o', hcont, hfacts⟩ :=
      ih d ⟨f, hf⟩ rfl (hwfd.wfInstr b₂ blk hb) (hwfd.wfTerm b₂ blk hb)
        v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨o', .inr ⟨v, .nil,
      (fun v => v.aborted = none ∧ v.cur.locals c = some (.bool false), b₂ + 1),
      by simp [termGoto], ⟨hok, by rw [hcur]; exact hc⟩,
      ContRun.toBExec (compileFun_blocks_src P d hlt hb) hcont⟩, hfacts⟩
  | @ret G s srcs vals hvals =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    have hcmds : CmdsExec (compiledSuffix P [] (.ret srcs)) v
        (.ok { v with cur := s, rets := vals }) := by
      have hstep := onOk_run (v := v)
        (f := fun v => match srcs.mapM v.cur.locals with
          | some vals => { v with rets := vals }
          | none => v) hok
      simp only [hcur, hvals] at hstep
      exact hstep
    by_cases hassert :
        d.contract.abortsFalse
          (VState.preEnvOf { v with cur := s, rets := vals } P.structs) ∧
        Holds
          (VState.postEnvOf { v with cur := s, rets := vals } P.structs)
          d.contract.ensures ∧
        agreesOutside
          (d.contract.footprint
            (VState.preEnvOf { v with cur := s, rets := vals } P.structs))
          (VState.snaps { v with cur := s, rets := vals } preLabel)
          (VState.cur { v with cur := s, rets := vals }).memory
    · refine ⟨.ok { v with cur := s, rets := vals },
        .inr ⟨{ v with cur := s, rets := vals }, hcmds,
          (flagClear, d.body.size + 1), by simp [termGoto], hok,
          .ret (compileFun_blocks_retExit P d) (.assertOk hassert .nil)⟩,
        .inr ⟨{ v with cur := s, rets := vals }, rfl, ⟨hok, rfl, rfl⟩,
          rfl, rfl, ?_, ?_⟩⟩
      · -- Conforms from the exit assertion
        obtain ⟨h1, h2, h3⟩ := hassert
        show Holds (postEnv P.structs m₀ s.memory args₀ vals)
            d.contract.ensures ∧
          agreesOutside
            (d.contract.footprint (preEnv P.structs m₀ args₀)) m₀ s.memory ∧
          d.contract.abortsFalse (preEnv P.structs m₀ args₀)
        refine ⟨?_, ?_, ?_⟩
        · simpa [VState.postEnvOf, hsnaps, hargs] using h2
        · simpa [VState.preEnvOf, hsnaps, hargs] using h3
        · simpa [VState.preEnvOf, hsnaps, hargs] using h1
      · -- OutTyped
        show TypedMemory P.structs s.memory ∧
          IsValidList P.structs d.returns vals
        cases hwfT with
        | ret hsts hsub =>
          exact ⟨hTM, (hTL.mapM_isValidList hvals hsts).sub hsub⟩
    · exact ⟨.fail, .inr ⟨{ v with cur := s, rets := vals }, hcmds,
        (flagClear, d.body.size + 1), by simp [termGoto], hok,
        .fail (compileFun_blocks_retExit P d) (.assertFail hassert)⟩,
        .inl rfl⟩
  | @abort G s code n hcode =>
    intro d hd hG hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    have hcmds : CmdsExec (compiledSuffix P [] (.abort code)) v
        (.ok (v.doAbort n)) := by
      have hstep := onOk_run (v := v)
        (f := fun v => match v.cur.locals code with
          | some (.u64 n) => v.doAbort n
          | _ => v) hok
      simp only [hcur, hcode] at hstep
      exact hstep
    obtain ⟨o', hterm, hfact⟩ :=
      abortExit_run P d (.abort code) (v := v.doAbort n) (by simp [VState.doAbort])
    refine ⟨o', .inr ⟨v.doAbort n, hcmds, hterm⟩, ?_⟩
    rcases hfact with rfl | ⟨rfl, hholds⟩
    · exact .inl rfl
    · refine .inr ⟨_, rfl, rfl, rfl, rfl, ?_, trivial⟩
      show d.contract.abortsHolds (preEnv P.structs m₀ args₀)
      simpa [VState.preEnvOf, VState.doAbort, hsnaps, hargs] using hholds

/-- **Adequacy per execution**: under the ambient hypotheses, every
terminating execution of a declared function from a well-typed boundary
satisfying `requires` conforms to the function's contract. -/
theorem funExec_conforms (P : Program)
    (hwfP : WfProg P)
    (hver : ∀ f d, P.funs f = some d → Verified P f)
    (hanns : ∀ f d, P.funs f = some d →
      WfProgram (compileFun P d) (compAnns P d) (fun l => l))
    {f : FunId} {d : FunDecl} {m : Memory} {args : List Value}
    {o : FrameOutcome}
    (hd : P.funs f = some d)
    (htyargs : TypedArgs P.structs d args)
    (htymem : TypedMemory P.structs m)
    (hreq : Holds (preEnv P.structs m args) d.contract.requires)
    (hexec : FunExec P f m args o) :
    Conforms P.structs d m args o := by
  obtain ⟨d', hd', blk, hentry, hrun⟩ := hexec
  rw [hd] at hd'
  injection hd' with hdd
  subst hdd
  have hwfd := hwfP f d hd
  obtain ⟨o', hcont, hfacts⟩ :=
    sim_aux P hwfP hver hanns hrun d ⟨f, hd⟩ rfl
      (hwfd.wfInstr _ _ hentry) (hwfd.wfTerm _ _ hentry)
      (initVState m args) m args rfl rfl rfl rfl
      (TypedLocals.initLocals htyargs) htymem
  have hentrylt : d.body.entry < d.body.size :=
    hwfd.blocksLt _ (by simp [hentry])
  have hblk0 : (compileFun P d).blocks 0 =
      some ⟨[.assume (typedEntry P d),
             .assume fun v => Holds (v.preEnvOf P.structs)
               d.contract.requires],
            .goto [(fun _ => True, d.body.entry + 1)]⟩ := by
    simp [compileFun]
  have hbex : BExec (compileFun P d) 0 (initVState m args) o' :=
    BExec.goto (gt := (fun _ => True, d.body.entry + 1)) hblk0
      (.assume ⟨htyargs, htymem⟩ (.assume hreq .nil))
      (List.mem_singleton.mpr rfl) trivial
      (ContRun.toBExec (compileFun_blocks_src P d hentrylt hentry) hcont)
  obtain ⟨d'', hd'', fuel, hwp⟩ := hver f d hd
  rw [hd] at hd''
  injection hd'' with hdd
  subst hdd
  rcases hfacts with rfl | ⟨v', rfl, hrel, hsn, har, hconf, hty⟩
  · exact absurd hbex
      (wpB_safe (hanns f d hd) (compAnns_start P d) (hwp m args))
  · exact hconf

/-- **Abstract calls over-approximate concrete calls**: if the callee
satisfies its contract, every terminating callee execution — from a
well-typed call boundary, and with well-typed results — is represented by
an execution of the two-command opaque schema the call compiles to, or the
schema fails its `requires` assertion. -/
theorem contract_call_overapproximates (P : Program) {f : FunId}
    {d : FunDecl} {dsts srcs : List LocalIndex} {s : MoveState} {v : VState}
    {args : List Value}
    (hd : P.funs f = some d)
    (hsat : SatisfiesContract P f d)
    (hargs : srcs.mapM s.locals = some args)
    (hcur : v.cur = s) (hok : v.aborted = none)
    (htyargs : TypedArgs P.structs d args)
    (htymem : TypedMemory P.structs s.memory) :
    (∀ m' rets, FunExec P f s.memory args (.ret m' rets) →
      rets.length = dsts.length →
      TypedMemory P.structs m' → IsValidList P.structs d.returns rets →
      ∃ o, CmdsExec (compileInstr P (.call dsts (.function f) srcs)) v o ∧
        (o = .fail ∨ ∃ v', o = .ok v' ∧ v'.aborted = none ∧
          v'.cur = MoveState.writeLocals ⟨s.locals, m'⟩ dsts rets ∧
          v'.snaps = v.snaps ∧ v'.args = v.args ∧ v'.rets = v.rets)) ∧
    (∀ m' code, FunExec P f s.memory args (.abort m' code) →
      ∃ o, CmdsExec (compileInstr P (.call dsts (.function f) srcs)) v o ∧
        (o = .fail ∨ ∃ v', o = .ok v' ∧ v'.aborted = some code ∧
          v'.snaps = v.snaps ∧ v'.args = v.args)) := by
  have hcomp : compileInstr P (.call dsts (.function f) srcs) =
      [.assert (callRequires P.structs d.contract srcs),
       .havoc (callRel P.structs d dsts srcs)] := by
    simp [compileInstr, hd]
  by_cases hreq : callRequires P.structs d.contract srcs v
  case neg =>
    constructor
    · intro m' rets _ _ _ _
      exact ⟨.fail, by rw [hcomp]; exact .assertFail hreq, .inl rfl⟩
    · intro m' code _
      exact ⟨.fail, by rw [hcomp]; exact .assertFail hreq, .inl rfl⟩
  case pos =>
    obtain ⟨args', hargs', hreqH⟩ := hreq hok
    rw [hcur, hargs] at hargs'
    injection hargs' with hargs'
    subst hargs'
    rw [hcur] at hreqH
    have hsat' := hsat s.memory args htyargs htymem hreqH
    constructor
    · intro m' rets hexec hlen hTMm' hretsV
      obtain ⟨hens, hagree, habf⟩ := hsat'.1 m' rets hexec
      have hcallrel : callRel P.structs d dsts srcs v
          { v with
            cur := MoveState.writeLocals ⟨s.locals, m'⟩ dsts rets } := by
        unfold callRel
        rw [if_neg (by simp [hok])]
        refine ⟨args, by rw [hcur]; exact hargs,
          .inr ⟨m', rets, hlen, ?_, ?_, ?_, ?_, by rw [hcur]⟩⟩
        · rw [hcur]; exact habf
        · rw [hcur]; exact hens
        · rw [hcur]; exact hagree
        · intro _; exact ⟨hTMm', hretsV⟩
      refine ⟨.ok { v with
          cur := MoveState.writeLocals ⟨s.locals, m'⟩ dsts rets },
        by rw [hcomp]; exact .assertOk hreq (.havoc hcallrel .nil),
        .inr ⟨_, rfl, hok, rfl, rfl, rfl, rfl⟩⟩
    · intro m' code hexec
      have habort := hsat'.2 m' code hexec
      have hcallrel : callRel P.structs d dsts srcs v (v.doAbort code) := by
        unfold callRel
        rw [if_neg (by simp [hok])]
        refine ⟨args, by rw [hcur]; exact hargs, .inl ⟨code, ?_, rfl⟩⟩
        rw [hcur]
        exact habort
      exact ⟨.ok (v.doAbort code),
        by rw [hcomp]; exact .assertOk hreq (.havoc hcallrel .nil),
        .inr ⟨_, rfl, rfl, rfl, rfl⟩⟩

/-- **Forward simulation** (block-wise): under the ambient hypotheses, a
terminating source execution from block `b` of a well-typed function, from
a related well-typed verification state, is represented by an execution of
the compiled program from label `b + 1` — either faithfully, through
`OutRel` and a normal IVL outcome at an exit block, or by an assertion
failure of the compiled program.  The verification-only components `snaps`
and `args` are invariant. -/
theorem compile_simulates (P : Program) (d : FunDecl)
    (hwfP : WfProg P)
    (hver : ∀ f d', P.funs f = some d' → Verified P f)
    (hanns : ∀ f d', P.funs f = some d' →
      WfProgram (compileFun P d') (compAnns P d') (fun l => l))
    {f : FunId} (hd : P.funs f = some d)
    {b : BlockId} {s : MoveState} {o : FrameOutcome} {v : VState}
    {m₀ : Memory} {args₀ : List Value}
    (hexec : RunBlock P d.body b s o)
    (hcur : v.cur = s) (hok : v.aborted = none)
    (hsnaps : v.snaps = fun _ => m₀) (hargs : v.args = args₀)
    (hTL : TypedLocals P.structs d.locals s.locals)
    (hTM : TypedMemory P.structs s.memory) :
    ∃ o', BExec (compileFun P d) (b + 1) v o' ∧
      (o' = .fail ∨ ∃ v', o' = .ok v' ∧ OutRel o v' ∧
        v'.snaps = v.snaps ∧ v'.args = v.args) := by
  obtain ⟨blk, hb, hrun⟩ := hexec
  have hwfd := hwfP f d hd
  have hlt : b < d.body.size := hwfd.blocksLt b (by simp [hb])
  obtain ⟨o', hcont, hfacts⟩ :=
    sim_aux P hwfP hver hanns hrun d ⟨f, hd⟩ rfl
      (hwfd.wfInstr _ _ hb) (hwfd.wfTerm _ _ hb)
      v m₀ args₀ hcur hok hsnaps hargs hTL hTM
  refine ⟨o', ContRun.toBExec (compileFun_blocks_src P d hlt hb) hcont, ?_⟩
  rcases hfacts with rfl | ⟨v', rfl, hrel, hsn, har, -, -⟩
  · exact .inl rfl
  · exact .inr ⟨v', rfl, hrel, hsn, har⟩

end Move.Prover.Translate
