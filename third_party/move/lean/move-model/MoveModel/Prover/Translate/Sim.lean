-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Prover.Ivl.Syntax
import MoveModel.Prover.Ivl.Semantics
import MoveModel.Prover.Ivl.WpSound
import MoveModel.IR.Semantics
import MoveModel.IR.Execution
import MoveModel.IR.CodeTyping
import MoveModel.Prover.Translate.Compile

/-!
# Forward Simulation

This module proves the forward direction of translation correctness: every
bytecode execution is represented by an IVL execution.

The compiled program contains assertions, including callee preconditions.
It may therefore fail while representing a source run.  Otherwise it
reproduces the source outcome through the abort-flag encoding.  A verified
program rules out the failure branch.

One master induction, `sim_aux`, follows the big-step source derivation.  This
handles mutual recursion between functions and establishes three properties
together:

* the *simulation*: the compiled run, built command by command
  (`ContRun`), with the raised abort flag skipping the rest of a block to
  the abort exit;
* *contract conformance* (`Conforms`): the exit-block assertions recorded
  along the constructed run, translated to the enclosing boundary;
* *type preservation* (`TypedLocals`, `TypedMemory`, and `OutTyped`), used at
  calls to satisfy the typing premises of `SatisfiesContract` and `callRel`.

At a call, the induction hypothesis simulates the callee body.  The entry stub
is then prefixed to this run.  Call-site typing and the asserted precondition
establish its assumptions.  The callee's `Verified` proof gives `wpB_safe`,
which excludes failure and yields the conformance facts required by
`callRel`.

Since source blocks map 1-1 to IVL blocks (source `b` ↦ label `b + 1`),
the public simulation statement (`compile_simulates`) is block-wise.
-/

namespace MoveModel.Prover.Translate

open MoveModel.Prover.Ivl
open MoveModel.IR

/-- Relation between a source frame outcome and a final verification state:
a normal return is represented with the abort flag down, the same final
memory and the returned values in `rets`; an abort is represented by the
raised flag carrying the code (the post-abort `cur` is irrelevant — Move
discards the state on abort). -/
def OutRel (o : FrameOutcome) (v' : VState) : Prop :=
  match o with
  | .ret world vals =>
      v'.aborted = none ∧ v'.cur.memory = world.memory ∧ v'.rets = vals
  | .abort _ code => v'.aborted = some code

/-- The per-outcome payload of `SatisfiesContract`: what an execution's
outcome must satisfy relative to the boundary `(m, args)`. -/
def Conforms (Δ : StructDecls) (d : FunDecl) (m : Memory)
    (args : List Value) : FrameOutcome → Prop
  | .ret world rets =>
      Holds (postEnv Δ m world.memory args rets) d.contract.ensures ∧
      agreesOutside (d.contract.footprint (preEnv Δ m args)) m world.memory ∧
      d.contract.abortsFalseAtExit Δ m world.memory args
  | .abort m' _ => d.contract.abortsHoldsAtExit Δ m m' args

/-- Type preservation at the boundary: a normal outcome delivers well-typed
memory and well-typed results. -/
def OutTyped (Δ : StructDecls) (d : FunDecl) : FrameOutcome → Prop
  | .ret world rets =>
      TypedMemory Δ world.memory ∧ IsValidList Δ d.returns rets
  | .abort _ _ => True

/-- A normal execution of frame `s.current` leaves all caller frames
unchanged.  Aborts discard the frame state, so they carry no frame fact. -/
def LowerFramesEq (s : MoveState) : FrameOutcome → Prop
  | .ret world _ =>
      ∀ frame, frame < s.current → world.frames frame = s.frames frame
  | .abort _ _ => True

theorem LowerFramesEq.ofSameBelow {s s' : MoveState} {o : FrameOutcome}
    (hss : s.SameBelow s') (ho : LowerFramesEq s' o) :
    LowerFramesEq s o := by
  cases o with
  | abort => trivial
  | ret world vals =>
    intro frame hlt
    rw [ho frame (by simpa [hss.1] using hlt), hss.2 frame hlt]

/-- Facts carried by a successful compiled suffix. -/
def CompileFacts (P : Program) (d : FunDecl) (m₀ : Memory)
    (args₀ : List Value) (s : MoveState) (o : FrameOutcome)
    (v : VState) (o' : Outcome VState) : Prop :=
  o' = .fail ∨ ∃ v', o' = .ok v' ∧ OutRel o v' ∧
    v'.snaps = v.snaps ∧ v'.args = v.args ∧
    Conforms P.structs d m₀ args₀ o ∧ OutTyped P.structs d o ∧
    LowerFramesEq s o

theorem CompileFacts.ofSameBelow {P : Program} {d : FunDecl}
    {m₀ : Memory} {args₀ : List Value} {s s' : MoveState}
    {o : FrameOutcome} {v : VState} {o' : Outcome VState}
    (hss : s.SameBelow s')
    (h : CompileFacts P d m₀ args₀ s' o v o') :
    CompileFacts P d m₀ args₀ s o v o' := by
  rcases h with hfail | ⟨v', hv', hrel, hsnaps, hargs, hconf, htyped, hlower⟩
  · exact .inl hfail
  · exact .inr ⟨v', hv', hrel, hsnaps, hargs, hconf, htyped,
      hlower.ofSameBelow hss⟩

/-- Translate an abort-exit assertion into the boundary facts carried by
the simulation motive.  Raising the abort flag preserves snapshots and
arguments, so every aborting handler uses the same conversion. -/
theorem abortOutcomeFacts (P : Program) (d : FunDecl)
    {s : MoveState} {v : VState} {m₀ m : Memory} {args₀ : List Value}
    {code : Nat} {o' : Outcome VState}
    (hsnaps : v.snaps = fun _ => m₀) (hargs : v.args = args₀)
    (hmemory : v.cur.memory = m)
    (hfact : o' = .fail ∨
      (o' = .ok (v.doAbort code) ∧
        d.contract.abortsHoldsAtExit P.structs
          (v.snaps preLabel) v.cur.memory v.args)) :
    o' = .fail ∨ ∃ v', o' = .ok v' ∧ OutRel (.abort m code) v' ∧
      v'.snaps = v.snaps ∧ v'.args = v.args ∧
      Conforms P.structs d m₀ args₀ (.abort m code) ∧
      OutTyped P.structs d (.abort m code) ∧
      LowerFramesEq s (.abort m code) := by
  rcases hfact with rfl | ⟨rfl, hholds⟩
  · exact .inl rfl
  · refine .inr ⟨_, rfl, rfl, rfl, rfl, ?_, trivial, trivial⟩
    change d.contract.abortsHoldsAtExit P.structs m₀ m args₀
    simpa [Contract.abortsHoldsAtExit, abortEnv, hsnaps, hargs, hmemory]
      using hholds

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
      | none =>
        simp only [compileInstr, hfd, refFail, List.mem_singleton] at hc
        subst hc
        exact skipCmd_assert _
      | some d =>
        rw [compileInstr, hfd] at hc
        simp only [List.mem_cons, List.not_mem_nil, or_false] at hc
        rcases hc with rfl | rfl
        · exact skipCmd_assert _
        · exact skipCmd_callRel _ _ _ _
    | functionInst f typeArgs =>
      intro c hc
      simp only [compileInstr, refFail, List.mem_singleton] at hc
      subst hc; exact skipCmd_assert _
    | borrowLoc =>
      intro c hc
      simp only [compileInstr, refFail, List.mem_singleton] at hc
      subst hc; exact skipCmd_assert _
    | borrowField i =>
      intro c hc
      simp only [compileInstr, refFail, List.mem_singleton] at hc
      subst hc; exact skipCmd_assert _
    | borrowFieldInst i typeArgs =>
      intro c hc
      simp only [compileInstr, refFail, List.mem_singleton] at hc
      subst hc; exact skipCmd_assert _
    | borrowGlobal r =>
      intro c hc
      simp only [compileInstr, refFail, List.mem_singleton] at hc
      subst hc; exact skipCmd_assert _
    | borrowGlobalInst r typeArgs =>
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
    | bitAnd => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
                subst hc; exact skipCmd_onOk _
    | bitOr => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
               subst hc; exact skipCmd_onOk _
    | bitXor => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
                subst hc; exact skipCmd_onOk _
    | shl w => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
               subst hc; exact skipCmd_onOk _
    | shr w => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
               subst hc; exact skipCmd_onOk _
    | cast target =>
      intro c hc
      simp only [compileInstr, List.mem_singleton] at hc
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
    | packInst typeArgs => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
                           subst hc; exact skipCmd_onOk _
    | unpack => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
                subst hc; exact skipCmd_onOk _
    | unpackInst typeArgs => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
                             subst hc; exact skipCmd_onOk _
    | packVariant variant => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
                             subst hc; exact skipCmd_onOk _
    | packVariantInst variant typeArgs =>
      intro c hc
      simp only [compileInstr, List.mem_singleton] at hc
      subst hc
      exact skipCmd_onOk _
    | unpackVariant variant => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
                               subst hc; exact skipCmd_onOk _
    | unpackVariantInst variant typeArgs =>
      intro c hc
      simp only [compileInstr, List.mem_singleton] at hc
      subst hc
      exact skipCmd_onOk _
    | testVariant variant => intro c hc; simp only [compileInstr, List.mem_singleton] at hc
                             subst hc; exact skipCmd_onOk _
    | testVariantInst variant typeArgs =>
      intro c hc
      simp only [compileInstr, List.mem_singleton] at hc
      subst hc
      exact skipCmd_onOk _
    | getField i => intro c hc
                    simp only [compileInstr, List.mem_singleton] at hc
                    subst hc; exact skipCmd_onOk _
    | getFieldInst i typeArgs => intro c hc
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
    | vecInsert => intro c hc
                   simp only [compileInstr, List.mem_singleton] at hc
                   subst hc; exact skipCmd_onOk _
    | vecRemove => intro c hc
                   simp only [compileInstr, List.mem_singleton] at hc
                   subst hc; exact skipCmd_onOk _
    | vecSwap => intro c hc
                 simp only [compileInstr, List.mem_singleton] at hc
                 subst hc; exact skipCmd_onOk _
    | vecSwapRemove => intro c hc
                       simp only [compileInstr, List.mem_singleton] at hc
                       subst hc; exact skipCmd_onOk _
    | vecAppend => intro c hc
                   simp only [compileInstr, List.mem_singleton] at hc
                   subst hc; exact skipCmd_onOk _
    | vecReverse => intro c hc
                    simp only [compileInstr, List.mem_singleton] at hc
                    subst hc; exact skipCmd_onOk _
    | vecReverseSlice => intro c hc
                         simp only [compileInstr, List.mem_singleton] at hc
                         subst hc; exact skipCmd_onOk _
    | vecContains => intro c hc
                     simp only [compileInstr, List.mem_singleton] at hc
                     subst hc; exact skipCmd_onOk _
    | vecIndexOf => intro c hc
                    simp only [compileInstr, List.mem_singleton] at hc
                    subst hc; exact skipCmd_onOk _
    | vecTrim => intro c hc
                 simp only [compileInstr, List.mem_singleton] at hc
                 subst hc; exact skipCmd_onOk _
    | vecTrimReverse => intro c hc
                        simp only [compileInstr, List.mem_singleton] at hc
                        subst hc; exact skipCmd_onOk _
    | vecRotate => intro c hc
                   simp only [compileInstr, List.mem_singleton] at hc
                   subst hc; exact skipCmd_onOk _
    | vecRotateSlice => intro c hc
                        simp only [compileInstr, List.mem_singleton] at hc
                        subst hc; exact skipCmd_onOk _
    | vecDestroyEmpty => intro c hc
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
    | getGlobalInst r typeArgs => intro c hc
                                  simp only [compileInstr, List.mem_singleton] at hc
                                  subst hc; exact skipCmd_onOk _
    | writeGlobal r => intro c hc
                       simp only [compileInstr, List.mem_singleton] at hc
                       subst hc; exact skipCmd_onOk _
    | moveTo r => intro c hc
                  simp only [compileInstr, List.mem_singleton] at hc
                  subst hc; exact skipCmd_onOk _
    | moveToInst r typeArgs => intro c hc
                               simp only [compileInstr, List.mem_singleton] at hc
                               subst hc; exact skipCmd_onOk _
    | moveFrom r => intro c hc
                    simp only [compileInstr, List.mem_singleton] at hc
                    subst hc; exact skipCmd_onOk _
    | moveFromInst r typeArgs => intro c hc
                                 simp only [compileInstr, List.mem_singleton] at hc
                                 subst hc; exact skipCmd_onOk _
    | exists_ r => intro c hc
                   simp only [compileInstr, List.mem_singleton] at hc
                   subst hc; exact skipCmd_onOk _
    | existsInst r typeArgs => intro c hc
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
        (o' = .ok v ∧ d.contract.abortsHoldsAtExit P.structs
          (v.snaps preLabel) v.cur.memory v.args)) := by
  by_cases hassert : d.contract.abortsHoldsAtExit P.structs
      (v.snaps preLabel) v.cur.memory v.args
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
        (o' = .ok v ∧ d.contract.abortsHoldsAtExit P.structs
          (v.snaps preLabel) v.cur.memory v.args)) := by
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
theorem Oper.sem_function_none (current : FrameId)
    (deref : RefTarget → Option Value)
    (f : FunId) (vs : List Value) (m : Memory) :
    Oper.sem current deref (.function f) vs m = none := by
  match vs with
  | [] => rfl
  | [_] => rfl
  | [_, _] => rfl
  | _ :: _ :: _ :: _ => rfl

/-- `Oper.sem` is also undefined on type-instantiated calls. -/
theorem Oper.sem_functionInst_none (current : FrameId)
    (deref : RefTarget → Option Value)
    (f : FunId) (typeArgs : List Ty) (vs : List Value) (m : Memory) :
    Oper.sem current deref (.functionInst f typeArgs) vs m = none := by
  match vs with
  | [] => rfl
  | [_] => rfl
  | [_, _] => rfl
  | _ :: _ :: _ :: _ => rfl

/-- `Oper.sem` is undefined on the reference operations (handled by
`refFail`). -/
theorem Oper.sem_refOp_none {op : Oper} (h : op.isRefOp)
    (current : FrameId) (deref : RefTarget → Option Value)
    (vs : List Value) (m : Memory) :
    Oper.sem current deref op vs m = none := by
  cases op <;> simp [Oper.isRefOp] at h <;>
    match vs with
    | [] => rfl
    | [_] => rfl
    | [_, _] => rfl
    | _ :: _ :: _ :: _ => rfl



/-- `compileInstr` on a value operation (neither a call nor a reference
operation) is the single guarded update. -/
theorem compileInstr_op (P : Program) (dsts srcs : List LocalIndex)
    {op : Oper} (hfun : ∀ f, op ≠ .function f)
    (hfunInst : ∀ f args, op ≠ .functionInst f args)
    (href : ¬ op.isRefOp) :
    compileInstr P (.call dsts op srcs) =
      [onOk (applyOper dsts op srcs)] := by
  cases op <;> first
    | rfl
    | (exact absurd rfl (hfun _))
    | (exact absurd rfl (hfunInst _ _))
    | (exact absurd rfl href)

/-! ## The master simulation induction -/

/-- The continuation property proved by compilation simulation at one source
cursor.  Naming the motive lets instruction-local proofs be separated from
the recursive execution induction. -/
abbrev CompileSimAt (P : Program) (G : Cfg) (rest : List Instr)
    (term : Term) (s : MoveState) (o : FrameOutcome) : Prop :=
  ∀ (d : FunDecl), G = d.body → WfFunDecl P d →
    (∀ i ∈ rest, WfInstr P d.locals i) → WfTerm P.structs d term →
    ∀ (v : VState) (m₀ : Memory) (args₀ : List Value),
      v.cur = s → v.aborted = none →
      v.snaps = (fun _ => m₀) → v.args = args₀ →
      TypedLocals P.structs d.locals s.locals →
      TypedMemory P.structs s.memory →
    ∃ o', ContRun (compileFun P d) P d.body.size rest term v o' ∧
      CompileFacts P d m₀ args₀ s o v o'

/-- Local compilation simulation for a continuing non-function
instruction.  The recursive suffix result is an input; this theorem contains
no execution induction of its own. -/
theorem compile_sim_instrNext (P : Program)
    {G : Cfg} {i : Instr} {rest : List Instr} {term : Term}
    {s s' : MoveState} {o : FrameOutcome}
    (hi : InstrNext i s s')
    (_hrest : RunFrom P G rest term s' o)
    (ih : CompileSimAt P G rest term s' o) :
    CompileSimAt P G (i :: rest) term s o := by
  cases hi with
  | @load s dst val =>
    intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    have hwf := hwfR _ List.mem_cons_self
    have hTL' : TypedLocals P.structs d.locals (s.writeLocal dst val).locals := by
      cases hwf with
      | load ht _ hval =>
        exact hTL.writeLocal fun t' ht' => by
          rw [ht] at ht'; cases ht'; exact hval
      | refInstr href => nomatch href
    obtain ⟨o', hcont, hfacts⟩ :=
      ih d hG hwfd (fun i hi => hwfR i (List.mem_cons_of_mem _ hi)) hwfT
        { v with cur := s.writeLocal dst val } m₀ args₀
        rfl hok hsnaps hargs hTL' hTM
    refine ⟨o', ContRun.prepend ?_ hcont rfl,
      hfacts.ofSameBelow (MoveState.sameBelow_writeLocal s dst val)⟩
    have hstep := onOk_run (v := v)
      (f := fun v => { v with cur := v.cur.writeLocal dst val }) hok
    simp only [hcur] at hstep
    exact hstep
  | @assign s dst src val hsrc =>
    intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    have hwf := hwfR _ List.mem_cons_self
    have hTL' : TypedLocals P.structs d.locals (s.writeLocal dst val).locals := by
      cases hwf with
      | assign hts htd hsub =>
        exact hTL.writeLocal fun t' ht' => by
          rw [htd] at ht'; cases ht'
          exact hsub.semantic _ (hTL _ _ _ hts hsrc)
      | refInstr href => nomatch href
    obtain ⟨o', hcont, hfacts⟩ :=
      ih d hG hwfd (fun i hi => hwfR i (List.mem_cons_of_mem _ hi)) hwfT
        { v with cur := s.writeLocal dst val } m₀ args₀
        rfl hok hsnaps hargs hTL' hTM
    refine ⟨o', ContRun.prepend ?_ hcont rfl,
      hfacts.ofSameBelow (MoveState.sameBelow_writeLocal s dst val)⟩
    have hstep := onOk_run (v := v)
      (f := fun v => match v.cur.locals src with
        | some val => { v with cur := v.cur.writeLocal dst val }
        | none => v) hok
    simp only [hcur, hsrc] at hstep
    exact hstep
  | nop =>
    intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    obtain ⟨o', hcont, hfacts⟩ :=
      ih d hG hwfd (fun i hi => hwfR i (List.mem_cons_of_mem _ hi)) hwfT
        v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨o', ContRun.prepend .nil hcont rfl, hfacts⟩
  | @op s dsts srcs op vs rets m' hsrcs hlen hop =>
    intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    have hwf := hwfR _ List.mem_cons_self
    cases hwf with
    | op hsts hdts hwfop hsub hsubr =>
      have hvs := (hTL.mapM_isValidList hsrcs hsts).sub hsub.semantic
      obtain ⟨hrets, hTM'⟩ := hwfop.sem_preserves hvs hTM hop
      have hTL' := hTL.writeLocals (s := s.setMemory m') hdts
        (hrets.sub hsubr.semantic)
      obtain ⟨o', hcont, hfacts⟩ :=
        ih d hG hwfd (fun i hi => hwfR i (List.mem_cons_of_mem _ hi)) hwfT
          { v with cur := MoveState.writeLocals (s.setMemory m') dsts rets }
          m₀ args₀ rfl hok hsnaps hargs hTL'
          (by rw [writeLocals_memory]; exact hTM')
      have hfun : ∀ f', op ≠ .function f' := by
        intro f' h
        subst h
        rw [Oper.sem_function_none] at hop
        cases hop
      have hfunInst : ∀ f' args', op ≠ .functionInst f' args' := by
        intro f' args' h
        subst h
        rw [Oper.sem_functionInst_none] at hop
        cases hop
      have href : ¬ op.isRefOp := by
        intro h
        rw [Oper.sem_refOp_none h] at hop
        cases hop
      refine ⟨o', ContRun.prepend ?_ hcont
        (compileInstr_op P dsts srcs hfun hfunInst href),
        hfacts.ofSameBelow ((MoveState.sameBelow_setMemory s m').trans
          (MoveState.sameBelow_writeLocals (s.setMemory m') dsts rets))⟩
      have hstep := onOk_run (v := v) (f := applyOper dsts op srcs) hok
      simp only [applyOper, hcur, hsrcs, hop, if_pos hlen] at hstep
      exact hstep
    | callFun hd' hsts hlen' hsubp hdts hsubr =>
      exact absurd hop (by rw [Oper.sem_function_none]; simp)
    | callFunInst hd' htyargs hsts hlen' hsubp hsubpSym hdts hsubr hsubrSym =>
      exact absurd hop (by rw [Oper.sem_functionInst_none]; simp)
    | refInstr href =>
      exact absurd hop (by rw [Oper.sem_refOp_none href.isRefOp]; simp)
  | @isParentMissing s dst p t pat rt val hp ht =>
    intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    have hwf := hwfR _ List.mem_cons_self
    have hTL' : TypedLocals P.structs d.locals
        (s.writeLocal dst (.bool false)).locals := by
      cases hwf with
      | op hsts hdts hwfop hsub hsubr =>
        cases hwfop with
        | isParent =>
          simpa [MoveState.writeLocals] using hTL.writeLocals hdts
            ((show IsValidList P.structs [.bool] [.bool false] by simp).sub
              hsubr.semantic)
      | refInstr href => nomatch href
    obtain ⟨o', hcont, hfacts⟩ :=
      ih d hG hwfd (fun i hi => hwfR i (List.mem_cons_of_mem _ hi)) hwfT
        { v with cur := s.writeLocal dst (.bool false) } m₀ args₀
        rfl hok hsnaps hargs hTL' hTM
    refine ⟨o', ContRun.prepend ?_ hcont rfl,
      hfacts.ofSameBelow
        (MoveState.sameBelow_writeLocal s dst (.bool false))⟩
    have hstep := onOk_run (v := v)
      (f := applyOper [dst] (.isParent pat) [p, t]) hok
    have hp' : s.frames s.current p = none := by
      simpa only [MoveState.locals_apply] using hp
    have ht' : s.frames s.current t = some (.mut rt val) := by
      simpa only [MoveState.locals_apply] using ht
    have happly : applyOper [dst] (.isParent pat) [p, t] v =
        { v with cur := s.writeLocal dst (.bool false) } := by
      simp [applyOper, hcur, hp', ht']
    rw [happly] at hstep
    simpa only [compileInstr] using hstep
  | borrowLoc _ | borrowField _ _ _ | borrowFieldInst _ _ _
  | borrowGlobal _ _ | borrowGlobalInst _ _
  | borrowVecElem _ _ _ _ | readRef _ _ _ | writeRef _ _ _ _
  | freezeRef _ _ _ =>
    intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨.fail, ContRun.failPrefix (.assertFail fun h => h), .inl rfl⟩

/-- Local compilation simulation for an immediately stopping non-function
instruction. -/
theorem compile_sim_instrStop (P : Program)
    {G : Cfg} {i : Instr} {rest : List Instr} {term : Term}
    {s : MoveState} {o : FrameOutcome}
    (hi : InstrStop i s o) : CompileSimAt P G (i :: rest) term s o := by
  cases hi with
  | @op s dsts srcs op vs hsrcs hop =>
    intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    have hfun : ∀ f', op ≠ .function f' := by
      intro f' h
      subst h
      rw [Oper.sem_function_none] at hop
      cases hop
    have hfunInst : ∀ f' args', op ≠ .functionInst f' args' := by
      intro f' args' h
      subst h
      rw [Oper.sem_functionInst_none] at hop
      cases hop
    have href : ¬ op.isRefOp := by
      intro h
      rw [Oper.sem_refOp_none h] at hop
      cases hop
    obtain ⟨o', hcont, hfact⟩ :=
      abortPath P d (rest := rest) (t := term)
        (v := v.doAbort op.abortCode) (code := op.abortCode) rfl
    refine ⟨o', ContRun.prepend ?_ hcont
      (compileInstr_op P dsts srcs hfun hfunInst href), ?_⟩
    · have hstep := onOk_run (v := v) (f := applyOper dsts op srcs) hok
      simp only [applyOper, hcur, hsrcs, hop] at hstep
      exact hstep
    · exact abortOutcomeFacts P d hsnaps hargs (by rw [hcur]) hfact
  | borrowGlobal _ _ | borrowGlobalInst _ _ | borrowVecElem _ _ _ _ =>
    intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
    exact ⟨.fail, ContRun.failPrefix (.assertFail fun h => h), .inl rfl⟩

/-- A simulated declaration satisfying its entry assumptions cannot reach
the compiled failure outcome; its source outcome therefore satisfies the
declaration's contract and is well typed.  The declaration need not occur as
a table entry, which is what lets this theorem verify a generic instance. -/
theorem compile_sim_decl (P : Program)
    {d : FunDecl}
    (hwfd : WfFunDecl P d)
    (hver : VerifiedDecl P d)
    (hanns : WfProgram (compileFun P d) (compAnns P d) (fun l => l))
    {s : MoveState} {args : List Value}
    {blk : Block} {o : FrameOutcome}
    (hentry : d.body.blocks d.body.entry = some blk)
    (hargsTyped : TypedArgs P.structs d args)
    (hTM : TypedMemory P.structs s.memory)
    (hreq : Holds ((initVState s.memory args).preEnvOf P.structs)
      d.contract.requires)
    (hlocals : s.locals = initLocals args)
    (ih : CompileSimAt P d.body blk.instrs blk.term
      s o) :
    Conforms P.structs d s.memory args o ∧ OutTyped P.structs d o ∧
      LowerFramesEq s o := by
  have hcurInit :
      (initVStateAt s.current s.frames s.memory args).cur = s := by
    apply MoveState.ext
    · rfl
    · change setFrame s.frames s.current (initLocals args) = s.frames
      rw [← hlocals]
      exact setFrame_self s.frames s.current
    · rfl
  obtain ⟨oc, hcont, hfacts⟩ :=
    ih d rfl hwfd (hwfd.wfInstr _ _ hentry)
      (hwfd.wfTerm _ _ hentry)
      (initVStateAt s.current s.frames s.memory args)
      s.memory args hcurInit
      rfl rfl rfl
      (by rw [hlocals]; exact TypedLocals.initLocals hargsTyped) hTM
  have hentrylt : d.body.entry < d.body.size :=
    hwfd.blocksLt _ (by simp [hentry])
  have hblk₀ : (compileFun P d).blocks 0 =
      some ⟨[.assume (typedEntry P d),
               .assume fun v => Holds (v.preEnvOf P.structs)
                 d.contract.requires],
            .goto [(fun _ => True, d.body.entry + 1)]⟩ := by
    simp [compileFun]
  have hbex : BExec (compileFun P d) 0
      (initVStateAt s.current s.frames s.memory args) oc :=
    BExec.goto (gt := (fun _ => True, d.body.entry + 1)) hblk₀
      (.assume ⟨hargsTyped, hTM⟩ (.assume hreq .nil))
      (List.mem_singleton.mpr rfl) trivial
      (ContRun.toBExec (compileFun_blocks_src P d hentrylt hentry) hcont)
  obtain ⟨fuel, hwp⟩ := hver
  rcases hfacts with rfl |
    ⟨v', rfl, hrel, hsnaps, hargs, hconf, htyped, hlower⟩
  · exact absurd hbex
      (wpB_safe hanns (compAnns_start P d)
        (hwp s.memory args s.current s.frames))
  · exact ⟨hconf, htyped, hlower⟩

/-- Declared-function wrapper around `compile_sim_decl`. -/
theorem compile_sim_callee (P : Program)
    (hwfP : WfProg P)
    (hver : ∀ f d, P.funs f = some d → Verified P f)
    (hanns : ∀ f d, P.funs f = some d →
      WfProgram (compileFun P d) (compAnns P d) (fun l => l))
    {f : FunId} {d : FunDecl} {s : MoveState} {args : List Value}
    {blk : Block} {o : FrameOutcome}
    (hd : P.funs f = some d)
    (hentry : d.body.blocks d.body.entry = some blk)
    (hargsTyped : TypedArgs P.structs d args)
    (hTM : TypedMemory P.structs s.memory)
    (hreq : Holds ((initVState s.memory args).preEnvOf P.structs)
      d.contract.requires)
    (hlocals : s.locals = initLocals args)
    (ih : CompileSimAt P d.body blk.instrs blk.term s o) :
    Conforms P.structs d s.memory args o ∧ OutTyped P.structs d o ∧
      LowerFramesEq s o := by
  obtain ⟨d', hd', hver'⟩ := hver f d hd
  rw [hd] at hd'
  injection hd' with hdd
  subst hdd
  exact compile_sim_decl P (hwfP f d hd) hver' (hanns f d hd)
    hentry hargsTyped hTM hreq hlocals ih

theorem compile_sim_callOk (P : Program)
    (hwfP : WfProg P)
    (hver : ∀ f d, P.funs f = some d → Verified P f)
    (hanns : ∀ f d, P.funs f = some d →
      WfProgram (compileFun P d) (compAnns P d) (fun l => l)) :
    RunFrom.CallOkCase P (CompileSimAt P) := by
  intro G rest term s dsts srcs f₂ d₂ args retVals blk m' o
    hd₂ hargs hnargs hentry hcallee ihcallee hlen hrest ihrest
  intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hvargs hTL hTM
  have hwf := hwfR _ List.mem_cons_self
  cases hwf with
  | op hsts hdts hwfop hsub hsubr => cases hwfop
  | refInstr href => nomatch href
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
        have hargsTyped := hTL.typedArgsOfCall hargs hsts hlenp hsubp
        have hreqE := hreq.holdsAtInitialState hok hcur hargs
        obtain ⟨hconfc, htypc, hlower⟩ :=
          compile_sim_callee P hwfP hver hanns
          (s := s.enterCall args) hd₂ hentry hargsTyped hTM hreqE
          (by simp [MoveState.enterCall, MoveState.locals]) ihcallee
        obtain ⟨hens, hagree, habf⟩ := hconfc
        obtain ⟨hTMm', hretsV⟩ := htypc
        have hcaller : m'.frames s.current = s.frames s.current := by
          calc
            m'.frames s.current =
                (s.enterCall args).frames s.current :=
              hlower s.current (Nat.lt_succ_self s.current)
            _ = s.frames s.current := by
              simp [MoveState.enterCall, setFrame]
        have hcallerLocals :
            (m'.resume s.current).locals = s.locals := hcaller
        have hcallrel : callRel P.structs d₂ dsts srcs v
            { v with
              cur := (m'.resume s.current).writeLocals dsts retVals } := by
          unfold callRel
          rw [if_neg (by simp [hok])]
          refine ⟨args, by rw [hcur]; exact hargs,
            .inr ⟨m'.memory, retVals, hlen.symm, ?_, ?_, ?_, ?_, ?_⟩⟩
          · rw [hcur]; exact habf.1
          · rw [hcur]; exact hens
          · rw [hcur]; exact hagree
          · intro _; exact ⟨hTMm', hretsV⟩
          · rw [hcur]
            refine ⟨by
                simp [MoveState.setMemory, FrameWorld.resume,
                  MoveState.resumeFrame], ?_, ?_,
              rfl, rfl, rfl, rfl⟩
            · apply MoveState.writeLocals_locals_congr
              change s.locals = (m'.resume s.current).locals
              exact hcallerLocals.symm
            · simp [MoveState.setMemory, FrameWorld.resume,
                MoveState.resumeFrame]
        have hTLbase :
            TypedLocals P.structs d.locals (m'.resume s.current).locals := by
          rw [hcallerLocals]
          exact hTL
        have hTL' := hTLbase.writeLocals
          (s := m'.resume s.current) hdts
          (hretsV.sub hsubr.semantic)
        obtain ⟨o', hcont, hfacts⟩ :=
          ihrest d hG hwfd (fun i hi => hwfR i (List.mem_cons_of_mem _ hi))
            hwfT
            { v with
              cur := (m'.resume s.current).writeLocals dsts retVals }
            m₀ args₀ rfl hok hsnaps hvargs hTL'
            (by rw [writeLocals_memory]; exact hTMm')
        have hresumeBelow : s.SameBelow (m'.resume s.current) := by
          refine ⟨rfl, ?_⟩
          intro frame hframe
          calc
            m'.frames frame = (s.enterCall args).frames frame :=
              hlower frame (Nat.lt_trans hframe (Nat.lt_succ_self _))
            _ = s.frames frame := by
              simp [MoveState.enterCall, setFrame,
                Nat.ne_of_lt (Nat.lt_trans hframe (Nat.lt_succ_self _))]
        refine ⟨o', ContRun.prepend
          (CmdsExec.assertOk hreq (CmdsExec.havoc hcallrel .nil)) hcont
          (by simp [compileInstr, hd₂]),
          hfacts.ofSameBelow (hresumeBelow.trans
            (MoveState.sameBelow_writeLocals
              (m'.resume s.current) dsts retVals))⟩

theorem compile_sim_callAbort (P : Program)
    (hwfP : WfProg P)
    (hver : ∀ f d, P.funs f = some d → Verified P f)
    (hanns : ∀ f d, P.funs f = some d →
      WfProgram (compileFun P d) (compAnns P d) (fun l => l)) :
    RunFrom.CallAbortCase P (CompileSimAt P) := by
  intro G rest term s dsts srcs f₂ d₂ args blk m' code
    hd₂ hargs hnargs hentry hcallee ihcallee
  intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hvargs hTL hTM
  have hwf := hwfR _ List.mem_cons_self
  cases hwf with
  | op hsts hdts hwfop hsub hsubr => cases hwfop
  | refInstr href => nomatch href
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
        have hargsTyped := hTL.typedArgsOfCall hargs hsts hlenp hsubp
        have hreqE := hreq.holdsAtInitialState hok hcur hargs
        obtain ⟨hconfc, htypc, _hlower⟩ :=
          compile_sim_callee P hwfP hver hanns
          (s := s.enterCall args) hd₂ hentry hargsTyped hTM hreqE
          (by simp [MoveState.enterCall, MoveState.locals]) ihcallee
        let vabort := ({ v with cur := v.cur.setMemory m' }).doAbort code
        have hcallrel : callRel P.structs d₂ dsts srcs v vabort := by
          unfold callRel
          rw [if_neg (by simp [hok])]
          refine ⟨args, by rw [hcur]; exact hargs,
            .inl ⟨code, m', ?_, rfl⟩⟩
          rw [hcur]
          exact hconfc.1
        obtain ⟨o', hcont, hfact⟩ :=
          abortPath P d (rest := rest) (t := term) (v := vabort)
            (code := code) rfl
        refine ⟨o', ContRun.prepend
          (CmdsExec.assertOk hreq (CmdsExec.havoc hcallrel .nil)) hcont
          (by simp [compileInstr, hd₂]), ?_⟩
        exact abortOutcomeFacts P d
          (v := { v with cur := v.cur.setMemory m' })
          hsnaps hvargs rfl hfact

/-- Generic calls must have been rewritten by `Module.monomorphize`; seeing
one at the IVL boundary is an explicit verification failure. -/
theorem compile_sim_callInstOk (P : Program) :
    RunFrom.CallInstOkCase P (CompileSimAt P) := by
  intro G rest term s dsts srcs f typeArgs d' args retVals blk world o
    hd htypeArity hargs hnargs hentry hcallee ihcallee hlen hrest ihrest
  intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hvargs hTL hTM
  exact ⟨.fail, ContRun.failPrefix (.assertFail fun h => h), .inl rfl⟩

/-- The same monomorphic-boundary check for an aborting generic call. -/
theorem compile_sim_callInstAbort (P : Program) :
    RunFrom.CallInstAbortCase P (CompileSimAt P) := by
  intro G rest term s dsts srcs f typeArgs d' args blk m' code
    hd htypeArity hargs hnargs hentry hcallee ihcallee
  intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hvargs hTL hTM
  exact ⟨.fail, ContRun.failPrefix (.assertFail fun h => h), .inl rfl⟩

/-- Simulation handler for CFG edges.  The recursive block simulation is
reconnected to the compiled CFG edge selected by the source terminator. -/
theorem compile_sim_termNext (P : Program) :
    RunFrom.TermNextCase P (CompileSimAt P) := by
  intro G term s b blk o ht hnext ih
  cases ht with
  | @jump s b blk hb =>
      intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
      subst hG
      have hlt : b < d.body.size := hwfd.blocksLt b (by simp [hb])
      obtain ⟨o', hcont, hfacts⟩ :=
        ih d rfl hwfd (hwfd.wfInstr b blk hb) (hwfd.wfTerm b blk hb)
          v m₀ args₀ hcur hok hsnaps hargs hTL hTM
      exact ⟨o', .inr ⟨v, .nil, (flagClear, b + 1), by simp [termGoto], hok,
        ContRun.toBExec (compileFun_blocks_src P d hlt hb) hcont⟩, hfacts⟩
  | @branch s c b₁ b₂ taken blk hc hb =>
      intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
      subst hG
      let b := if taken then b₁ else b₂
      change d.body.blocks b = some blk at hb
      have hlt : b < d.body.size := hwfd.blocksLt b (by simp [hb])
      obtain ⟨o', hcont, hfacts⟩ :=
        ih d rfl hwfd (hwfd.wfInstr b blk hb) (hwfd.wfTerm b blk hb)
          v m₀ args₀ hcur hok hsnaps hargs hTL hTM
      exact ⟨o', .inr ⟨v, .nil,
        (fun v => v.aborted = none ∧
          v.cur.locals c = some (.bool taken), b + 1),
        by cases taken <;> simp [b, termGoto],
        ⟨hok, by rw [hcur]; exact hc⟩,
        ContRun.toBExec (compileFun_blocks_src P d hlt hb) hcont⟩, hfacts⟩

/-- Simulation handler for return and abort terminators. -/
theorem compile_sim_termStop (P : Program) :
    RunFrom.TermStopCase P (CompileSimAt P) := by
  intro G term s o ht
  cases ht with
  | @ret s srcs vals hvals =>
      intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
      have hcmds : CmdsExec (compiledSuffix P [] (.ret srcs)) v
          (.ok { v with cur := s, rets := vals }) := by
        have hstep := onOk_run (v := v)
          (f := fun v => match srcs.mapM v.cur.locals with
            | some vals => { v with rets := vals }
            | none => v) hok
        simp only [hcur, hvals] at hstep
        exact hstep
      by_cases hassert :
          d.contract.abortsFalseAtExit P.structs
            (VState.snaps { v with cur := s, rets := vals } preLabel)
            (VState.cur { v with cur := s, rets := vals }).memory
            (VState.args { v with cur := s, rets := vals }) ∧
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
            rfl, rfl, ?_, ?_, ?_⟩⟩
        · obtain ⟨h₁, h₂, h₃⟩ := hassert
          show Holds (postEnv P.structs m₀ s.memory args₀ vals)
              d.contract.ensures ∧
            agreesOutside
              (d.contract.footprint (preEnv P.structs m₀ args₀)) m₀ s.memory ∧
            d.contract.abortsFalseAtExit P.structs m₀ s.memory args₀
          refine ⟨?_, ?_, ?_⟩
          · simpa [VState.postEnvOf, hsnaps, hargs] using h₂
          · simpa [VState.preEnvOf, hsnaps, hargs] using h₃
          · simpa [hsnaps, hargs] using h₁
        · show TypedMemory P.structs s.memory ∧
            IsValidList P.structs d.returns vals
          cases hwfT with
          | ret hsts hsub =>
              exact ⟨hTM, (hTL.mapM_isValidList hvals hsts).sub hsub.semantic⟩
        · intro frame hframe
          simp [MoveState.finishFrame, setFrame, Nat.ne_of_lt hframe]
      · exact ⟨.fail, .inr ⟨{ v with cur := s, rets := vals }, hcmds,
          (flagClear, d.body.size + 1), by simp [termGoto], hok,
          .fail (compileFun_blocks_retExit P d) (.assertFail hassert)⟩,
          .inl rfl⟩
  | @abort s code n hcode =>
      intro d hG hwfd hwfR hwfT v m₀ args₀ hcur hok hsnaps hargs hTL hTM
      have hcmds : CmdsExec (compiledSuffix P [] (.abort code)) v
          (.ok (v.doAbort n)) := by
        have hstep := onOk_run (v := v)
          (f := fun v => match v.cur.locals code with
            | some (.u64 n) => v.doAbort n
            | _ => v) hok
        simp only [hcur, hcode] at hstep
        exact hstep
      obtain ⟨o', hterm, hfact⟩ :=
        abortExit_run P d (.abort code) (v := v.doAbort n)
          (by simp [VState.doAbort])
      refine ⟨o', .inr ⟨v.doAbort n, hcmds, hterm⟩, ?_⟩
      exact abortOutcomeFacts P d hsnaps hargs (by rw [hcur]) hfact

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
    RunFrom P G rest term s o → CompileSimAt P G rest term s o := by
  intro G rest term s o hrun
  exact RunFrom.inductGrouped
    (M := CompileSimAt P)
    (compile_sim_instrNext P) (compile_sim_instrStop P)
    (compile_sim_callOk P hwfP hver hanns)
    (compile_sim_callAbort P hwfP hver hanns)
    (compile_sim_callInstOk P)
    (compile_sim_callInstAbort P)
    (compile_sim_termNext P) (compile_sim_termStop P) hrun

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
  obtain ⟨d', hd', _harity, blk, hentry, hrun⟩ := hexec
  rw [hd] at hd'
  injection hd' with hdd
  subst hdd
  exact (compile_sim_callee P hwfP hver hanns
    (s := MoveState.initial args m) hd hentry htyargs htymem
    (by
      change Holds (preEnv P.structs m args) d.contract.requires
      exact hreq)
    rfl (sim_aux P hwfP hver hanns hrun)).1

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
      TypedMemory P.structs m'.memory → IsValidList P.structs d.returns rets →
      ∃ o, CmdsExec (compileInstr P (.call dsts (.function f) srcs)) v o ∧
        (o = .fail ∨ ∃ v', o = .ok v' ∧ v'.aborted = none ∧
          v'.cur = (s.setMemory m'.memory).writeLocals dsts rets ∧
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
            cur := (s.setMemory m'.memory).writeLocals dsts rets } := by
        unfold callRel
        rw [if_neg (by simp [hok])]
        refine ⟨args, by rw [hcur]; exact hargs,
          .inr ⟨m'.memory, rets, hlen, ?_, ?_, ?_, ?_, ?_⟩⟩
        · rw [hcur]; exact habf.1
        · rw [hcur]; exact hens
        · rw [hcur]; exact hagree
        · intro _; exact ⟨hTMm', hretsV⟩
        · rw [hcur]
          exact VState.VisibleEq.refl _
      refine ⟨.ok { v with
          cur := (s.setMemory m'.memory).writeLocals dsts rets },
        by rw [hcomp]; exact .assertOk hreq (.havoc hcallrel .nil),
        .inr ⟨_, rfl, hok, rfl, rfl, rfl, rfl⟩⟩
    · intro m' code hexec
      have habort := hsat'.2 m' code hexec
      let vabort := ({ v with cur := v.cur.setMemory m' }).doAbort code
      have hcallrel : callRel P.structs d dsts srcs v vabort := by
        unfold callRel
        rw [if_neg (by simp [hok])]
        refine ⟨args, by rw [hcur]; exact hargs,
          .inl ⟨code, m', ?_, rfl⟩⟩
        rw [hcur]
        exact habort.1
      exact ⟨.ok vabort,
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
    sim_aux P hwfP hver hanns hrun d rfl
      (hwfP f d hd)
      (hwfd.wfInstr _ _ hb) (hwfd.wfTerm _ _ hb)
      v m₀ args₀ hcur hok hsnaps hargs hTL hTM
  refine ⟨o', ContRun.toBExec (compileFun_blocks_src P d hlt hb) hcont, ?_⟩
  rcases hfacts with rfl | ⟨v', rfl, hrel, hsn, har, -, -, -⟩
  · exact .inl rfl
  · exact .inr ⟨v', rfl, hrel, hsn, har⟩

end MoveModel.Prover.Translate
