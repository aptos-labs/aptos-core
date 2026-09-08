-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Execution
import MoveModel.IR.CodeTyping

/-!
# Frontend-checked programs and states

This module organizes the frontend facts consumed by semantic and
transformation proofs into three layers:

* static code typing and declaration/CFG consistency;
* runtime type and reference consistency;
* preservation of a runtime invariant throughout an execution.

Analyses may project these facts into narrower, program-point certificates
such as `FrameSafe`.  Those projections are not whole-program checking
judgments.
-/

namespace MoveModel.IR

/-! ## Static consistency -/

/-- Structural facts expected of every frontend-produced function. -/
structure ConsistentFunDecl (d : FunDecl) : Prop where
  paramsBound : d.numParams ≤ d.numLocals
  declared : ∀ x, x < d.numLocals → ∃ ty, d.locals x = some ty
  localsBound : ∀ x ty, d.locals x = some ty → x < d.numLocals
  noNestedRefs : ∀ x t,
    d.locals x = some (.ref t) ∨ d.locals x = some (.mutRef t) →
      t.isRef = false
  entry : ∃ blk, d.body.blocks d.body.entry = some blk
  successors : ∀ b blk succ,
    d.body.blocks b = some blk → succ ∈ termSuccs blk.term →
      ∃ succBlk, d.body.blocks succ = some succBlk

/-- The static frontend checks for one function: code typing plus
declaration/CFG shape. -/
structure CheckedFunDecl (P : Program) (d : FunDecl) : Prop where
  typed : WfFunDecl P d
  consistent : ConsistentFunDecl d

/-- The static frontend checks for every function in a program. -/
def CheckedProgram.Static (P : Program) : Prop :=
  ∀ f d, P.funs f = some d → CheckedFunDecl P d

/-- Forget declaration consistency and retain static IR code typing. -/
theorem CheckedProgram.Static.wfProg {P : Program}
    (h : CheckedProgram.Static P) :
    WfProg P :=
  fun f d hd => (h f d hd).typed

/-! ## Runtime consistency -/

/-- A source runtime value at its bytecode-local type.  Unlike boundary
`IsValid`, reference types here describe an actual `Value.ref` whose target
exists and contains a value valid at the referenced type. -/
def SourceValueValid (Δ : StructDecls) (s : MoveState) : Ty → Value → Prop
  | .ref t, v | .mutRef t, v =>
      ∃ rt w, v = .ref rt ∧ s.readTarget rt = some w ∧ IsValid Δ t w
  | t, v => IsValid Δ t v

/-- Runtime typing for source bytecode, including actual reference locals. -/
structure RuntimeTyped (Δ : StructDecls) (d : FunDecl)
    (s : MoveState) : Prop where
  values : ∀ x ty v, d.locals x = some ty → s.locals x = some v →
    SourceValueValid Δ s ty v
  memory : TypedMemory Δ s.memory

/-- Runtime facts supplied by bytecode verification and borrow checking.
The value-shape component is deliberately independent of semantic typing so
proofs needing only reference-freedom do not depend on `IsValid`. -/
structure RuntimeConsistent (d : FunDecl) (s : MoveState) extends
    ConsistentFunDecl d where
  values : ∀ x ty v, d.locals x = some ty → s.locals x = some v →
    if ty.isRef then
      ∃ rt w, v = .ref rt ∧ s.readTarget rt = some w ∧ w.refFree
    else v.refFree
  memory : ∀ r a v, s.memory r a = some v → v.refFree

/-- The combined runtime typing and reference-consistency certificate carried
for a state accepted by the frontend checks. -/
structure CheckedState (P : Program) (d : FunDecl) (s : MoveState) : Prop where
  consistent : RuntimeConsistent d s
  typed : RuntimeTyped P.structs d s

/-- A non-reference local in a consistent state contains a reference-free value. -/
theorem RuntimeConsistent.plain_free {d : FunDecl} {s : MoveState}
    (h : RuntimeConsistent d s) {x : LocalIndex} {ty : Ty} {v : Value}
    (hty : d.locals x = some ty) (hnref : ty.isRef = false)
    (hv : s.locals x = some v) : v.refFree := by
  simpa [hnref] using h.values x ty v hty hv

/-- Dereferencing a reference local in a consistent state yields a
reference-free value. -/
theorem RuntimeConsistent.refTarget_free {d : FunDecl} {s : MoveState}
    (h : RuntimeConsistent d s) {x : LocalIndex} {ty : Ty}
    {rt : RefTarget} {v : Value} (hty : d.locals x = some ty)
    (href : ty.isRef = true) (hx : s.locals x = some (.ref rt))
    (hv : s.readTarget rt = some v) : v.refFree := by
  have hlocal := h.values x ty (.ref rt) hty hx
  rw [href] at hlocal
  obtain ⟨rt', w, heq, hread, hfree⟩ := hlocal
  cases heq
  rw [hv] at hread
  cases hread
  exact hfree

/-- A declared local containing a runtime reference has a reference type. -/
theorem RuntimeConsistent.ref_decl {d : FunDecl} {s : MoveState}
    (h : RuntimeConsistent d s) {x : LocalIndex} (hrange : x < d.numLocals)
    {rt : RefTarget} (hx : s.locals x = some (.ref rt)) :
    ∃ ty, d.locals x = some ty ∧ ty.isRef = true := by
  obtain ⟨ty, hty⟩ := h.declared x hrange
  refine ⟨ty, hty, ?_⟩
  cases href : ty.isRef
  · have := h.plain_free hty href hx
    simp at this
  · rfl

/-! ## Generic invariant preservation -/

/-- A function execution carrying an arbitrary state invariant. -/
def InvariantFunExec (P : Program) (StateOK : Cfg → MoveState → Prop)
    (f : FunId) (m : Memory) (args : List Value)
    (o : FrameOutcome) : Prop :=
  ∃ d, P.funs f = some d ∧ args.length = d.numParams ∧ ∃ blk,
    d.body.blocks d.body.entry = some blk ∧
    RunFrom.Invariant P StateOK d.body blk.instrs blk.term
      (MoveState.initial args m) o

/-- Every semantic execution starting in a state satisfying `StateOK` can be
decorated with that invariant.  The initial-state premise is essential: the
operational semantics intentionally remains executable on malformed inputs. -/
def ExecutionPreserves (P : Program)
    (StateOK : Cfg → MoveState → Prop) : Prop :=
  ∀ f d m args o, P.funs f = some d →
    StateOK d.body (MoveState.initial args m) →
    FunExec P f m args o →
    InvariantFunExec P StateOK f m args o

/-- Erase the invariant decoration from a function execution. -/
theorem InvariantFunExec.run {P : Program} {StateOK : Cfg → MoveState → Prop}
    {f : FunId} {m : Memory} {args : List Value} {o : FrameOutcome}
    (h : InvariantFunExec P StateOK f m args o) : FunExec P f m args o := by
  obtain ⟨d, hd, harity, blk, hblk, hrun⟩ := h
  exact ⟨d, hd, harity, blk, hblk, hrun.run⟩

/-- Recover the decorated initial state and the declaration selected by a
function execution. -/
theorem InvariantFunExec.initial {P : Program}
    {StateOK : Cfg → MoveState → Prop} {f : FunId} {m : Memory}
    {args : List Value} {o : FrameOutcome}
    (h : InvariantFunExec P StateOK f m args o) :
    ∃ d blk, P.funs f = some d ∧
      d.body.blocks d.body.entry = some blk ∧
      StateOK d.body (MoveState.initial args m) := by
  obtain ⟨d, hd, _harity, blk, hblk, hrun⟩ := h
  exact ⟨d, blk, hd, hblk, hrun.start⟩

/-- Runtime consistency at a CFG belonging to a declaration of `P`. -/
def RuntimeConsistentAt (P : Program) (G : Cfg) (s : MoveState) : Prop :=
  ∀ f d, P.funs f = some d → G = d.body → RuntimeConsistent d s

/-- Full checked-state validity at a CFG belonging to a declaration of `P`. -/
def CheckedStateAt (P : Program) (G : Cfg) (s : MoveState) : Prop :=
  ∀ f d, P.funs f = some d → G = d.body → CheckedState P d s

/-- A checked boundary for starting one function execution.  Argument arity
and value typing are kept alongside the state invariant because the untyped
semantics itself does not impose them at the outermost call boundary. -/
structure CheckedInput (P : Program) (f : FunId) (m : Memory)
    (args : List Value) : Prop where
  typedArgs : ∀ d, P.funs f = some d → TypedArgs P.structs d args
  state : ∀ d, P.funs f = some d →
    CheckedStateAt P d.body (MoveState.initial args m)

/-- A function execution decorated with runtime-consistency certificates. -/
abbrev ConsistentFunExec (P : Program) :=
  InvariantFunExec P (RuntimeConsistentAt P)

/-- A function execution decorated with full checked-state certificates. -/
abbrev CheckedFunExec (P : Program) :=
  InvariantFunExec P (CheckedStateAt P)

/-- The frontend certificate specialized to one concrete execution.  Pass
composition uses this boundary when the next layer needs checked states only
along the execution being simulated, rather than preservation for every
possible execution of the intermediate program. -/
structure CheckedExecution (P : Program) (f : FunId) (m : Memory)
    (args : List Value) (o : FrameOutcome) : Prop where
  static : CheckedProgram.Static P
  input : CheckedInput P f m args
  execution : CheckedFunExec P f m args o

/-- Erase runtime-consistency decorations from a function execution. -/
theorem ConsistentFunExec.run {P : Program} {f : FunId} {m : Memory}
    {args : List Value} {o : FrameOutcome}
    (h : ConsistentFunExec P f m args o) : FunExec P f m args o :=
  InvariantFunExec.run h

/-- Forget runtime typing while retaining runtime consistency on an execution. -/
theorem CheckedFunExec.consistent {P : Program} {f : FunId} {m : Memory}
    {args : List Value} {o : FrameOutcome}
    (h : CheckedFunExec P f m args o) : ConsistentFunExec P f m args o := by
  obtain ⟨d, hd, harity, blk, hblk, hrun⟩ := h
  refine ⟨d, hd, harity, blk, hblk, hrun.mono ?_⟩
  intro G s hs f' d' hf' hG
  exact (hs f' d' hf' hG).consistent

/-- Recover the checked initial state of a checked function execution. -/
theorem CheckedFunExec.initial {P : Program} {f : FunId} {m : Memory}
    {args : List Value} {o : FrameOutcome}
    (h : CheckedFunExec P f m args o) :
    ∃ d blk, P.funs f = some d ∧
      d.body.blocks d.body.entry = some blk ∧
      CheckedState P d (MoveState.initial args m) := by
  obtain ⟨d, blk, hd, hblk, hs⟩ := InvariantFunExec.initial h
  exact ⟨d, blk, hd, hblk, hs f d hd rfl⟩

/-- Project static IR code typing from a checked execution. -/
theorem CheckedExecution.wfProg {P : Program} {f : FunId} {m : Memory}
    {args : List Value} {o : FrameOutcome}
    (h : CheckedExecution P f m args o) : WfProg P :=
  h.static.wfProg

/-- Erase all checked decorations from a checked execution. -/
theorem CheckedExecution.run {P : Program} {f : FunId} {m : Memory}
    {args : List Value} {o : FrameOutcome}
    (h : CheckedExecution P f m args o) : FunExec P f m args o :=
  h.execution.run

/-- Recover the declaration, entry block, and checked initial state. -/
theorem CheckedExecution.initial {P : Program} {f : FunId} {m : Memory}
    {args : List Value} {o : FrameOutcome}
    (h : CheckedExecution P f m args o) :
    ∃ d blk, P.funs f = some d ∧
      d.body.blocks d.body.entry = some blk ∧
      CheckedState P d (MoveState.initial args m) :=
  h.execution.initial

/-- CFG block bounds projected from the static part of a checked execution. -/
theorem CheckedExecution.blocksLt {P : Program} {f : FunId} {m : Memory}
    {args : List Value} {o : FrameOutcome}
    (h : CheckedExecution P f m args o) :
    ∀ g d, P.funs g = some d →
      ∀ b, d.body.blocks b ≠ none → b < d.body.size := by
  intro g d hd b hb
  exact (h.wfProg g d hd).blocksLt b hb

/-- Initial memory typing projected from a checked execution. -/
theorem CheckedExecution.initialMemoryTyped {P : Program} {f : FunId}
    {m : Memory} {args : List Value} {o : FrameOutcome}
    (h : CheckedExecution P f m args o) : TypedMemory P.structs m := by
  obtain ⟨_d, _blk, _hd, _hblk, hs⟩ := h.initial
  simpa [MoveState.initial] using hs.typed.memory

/-- Initial memory contains no reference values in a checked execution. -/
theorem CheckedExecution.initialMemoryRefFree {P : Program} {f : FunId}
    {m : Memory} {args : List Value} {o : FrameOutcome}
    (h : CheckedExecution P f m args o) :
    ∀ r a v, m r a = some v → v.refFree := by
  obtain ⟨_d, _blk, _hd, _hblk, hs⟩ := h.initial
  intro r a v hv
  apply hs.consistent.memory r a v
  simpa [MoveState.initial] using hv

/-- The complete frontend certificate consumed by semantic proofs. -/
structure CheckedProgram (P : Program) : Prop where
  static : CheckedProgram.Static P
  execution : ExecutionPreserves P (CheckedStateAt P)

/-- Project static IR code typing from the complete frontend certificate. -/
theorem CheckedProgram.wfProg {P : Program} (h : CheckedProgram P) : WfProg P :=
  h.static.wfProg

/-- Decorate one execution with all frontend-provided runtime invariants. -/
theorem CheckedProgram.checkedExec {P : Program} (h : CheckedProgram P)
    {f : FunId} {m : Memory} {args : List Value} {o : FrameOutcome}
    (input : CheckedInput P f m args)
    (hexec : FunExec P f m args o) : CheckedFunExec P f m args o := by
  obtain ⟨d, hd, harity, hrun⟩ := hexec
  exact h.execution f d m args o hd (input.state d hd)
    ⟨d, hd, harity, hrun⟩

/-- Specialize a whole-program frontend certificate to one execution. -/
theorem CheckedProgram.executionOf {P : Program} (h : CheckedProgram P)
    {f : FunId} {m : Memory} {args : List Value} {o : FrameOutcome}
    (input : CheckedInput P f m args)
    (hexec : FunExec P f m args o) : CheckedExecution P f m args o :=
  ⟨h.static, input, h.checkedExec input hexec⟩

/-- Recover the checked initial state selected by a concrete execution. -/
theorem CheckedProgram.initial {P : Program} (h : CheckedProgram P)
    {f : FunId} {m : Memory} {args : List Value} {o : FrameOutcome}
    (input : CheckedInput P f m args)
    (hexec : FunExec P f m args o) :
    ∃ d blk, P.funs f = some d ∧
      d.body.blocks d.body.entry = some blk ∧
      CheckedState P d (MoveState.initial args m) :=
  CheckedFunExec.initial (h.checkedExec input hexec)

end MoveModel.IR
