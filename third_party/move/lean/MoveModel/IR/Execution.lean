-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Semantics

/-!
# Structured execution induction

`RunFrom` gives each concrete Move operation its own constructor.  This is
useful for defining semantics but repetitive for proofs.  This module
separates the first action from the execution that follows and provides an
induction principle over those groups.

The local judgments do not replace `RunFrom` and contain no recursive
execution premises:

* `InstrNext` and `InstrStop` describe one non-function instruction;
* `TermNext` and `TermStop` describe one terminator.

Function calls remain separate because both the callee and the caller
continuation contribute induction hypotheses.  `RunFrom.inductGrouped`
therefore has six cases instead of the 20 concrete `RunFrom` constructors.
The head-action judgments still permit exhaustive local case analysis.

`InstrPath` packages a finite sequence of continuing head actions.  It is the
reusable certificate used when one source instruction becomes several target
instructions.
-/

namespace MoveModel.IR

/-- A non-function instruction which continues in state `s'`. -/
inductive InstrNext : Instr → MoveState → MoveState → Prop where
  | load {s : MoveState} {dst : LocalIndex} {v : Value} :
      InstrNext (.load dst v) s (s.writeLocal dst v)
  | assign {s : MoveState} {dst src : LocalIndex} {v : Value}
      (hsrc : s.locals src = some v) :
      InstrNext (.assign dst src) s (s.writeLocal dst v)
  | nop {s : MoveState} : InstrNext .nop s s
  | op {s : MoveState} {dsts srcs : List LocalIndex} {op : Oper}
      {vs rets : List Value} {m' : Memory}
      (hsrcs : srcs.mapM s.locals = some vs)
      (hlen : dsts.length = rets.length)
      (hop : op.sem s.current s.readTarget vs s.memory = some (.ok rets m')) :
      InstrNext (.call dsts op srcs) s
        (MoveState.writeLocals (s.setMemory m') dsts rets)
  | isParentMissing {s : MoveState} {dst p t : LocalIndex}
      {pat : List (Option Nat)} {rt : RefTarget} {v : Value}
      (hp : s.locals p = none)
      (ht : s.locals t = some (.mut rt v)) :
      InstrNext (.call [dst] (.isParent pat) [p, t]) s
        (s.writeLocal dst (.bool false))
  | borrowLoc {s : MoveState} {dst x : LocalIndex} {v : Value}
      (hx : s.locals x = some v) :
      InstrNext (.call [dst] .borrowLoc [x]) s
        (s.writeLocal dst (.ref ⟨.loc s.current x, []⟩))
  | borrowField {s : MoveState} {dst t : LocalIndex} {i : Nat}
      {rt : RefTarget} {fs : List Value}
      (ht : s.locals t = some (.ref rt))
      (hs : s.readTarget rt = some (.struct fs))
      (hi : i < fs.length) :
      InstrNext (.call [dst] (.borrowField i) [t]) s
        (s.writeLocal dst (.ref ⟨rt.root, rt.path ++ [i]⟩))
  | borrowFieldInst {s : MoveState} {dst t : LocalIndex} {i : Nat}
      {args : List Ty} {rt : RefTarget} {fs : List Value}
      (ht : s.locals t = some (.ref rt))
      (hs : s.readTarget rt = some (.struct fs))
      (hi : i < fs.length) :
      InstrNext (.call [dst] (.borrowFieldInst i args) [t]) s
        (s.writeLocal dst (.ref ⟨rt.root, rt.path ++ [i]⟩))
  | borrowGlobal {s : MoveState} {dst t : LocalIndex} {r : ResourceId}
      {a : Address} {v : Value}
      (ha : s.locals t = some (.address a))
      (hpresent : s.memory r a = some v) :
      InstrNext (.call [dst] (.borrowGlobal r) [t]) s
        (s.writeLocal dst (.ref ⟨.global r a, []⟩))
  | borrowGlobalInst {s : MoveState} {dst t : LocalIndex} {r : ResourceId}
      {args : List Ty} {a : Address} {v : Value}
      (ha : s.locals t = some (.address a))
      (hpresent : s.memory (resourceKey r args) a = some v) :
      InstrNext (.call [dst] (.borrowGlobalInst r args) [t]) s
        (s.writeLocal dst (.ref ⟨.global (resourceKey r args) a, []⟩))
  | borrowVecElem {s : MoveState} {dst t it : LocalIndex}
      {rt : RefTarget} {es : List Value} {n : Nat}
      (ht : s.locals t = some (.ref rt))
      (hv : s.readTarget rt = some (.vector es))
      (hi : s.locals it = some (.u64 n))
      (hlt : n < es.length) :
      InstrNext (.call [dst] .borrowVecElem [t, it]) s
        (s.writeLocal dst (.ref ⟨rt.root, rt.path ++ [n]⟩))
  | readRef {s : MoveState} {dst t : LocalIndex} {rt : RefTarget}
      {v : Value}
      (ht : s.locals t = some (.ref rt))
      (hv : s.readTarget rt = some v)
      (hfree : v.refFree) :
      InstrNext (.call [dst] .readRef [t]) s (s.writeLocal dst v)
  | writeRef {s s' : MoveState} {t vt : LocalIndex} {rt : RefTarget}
      {v : Value}
      (ht : s.locals t = some (.ref rt))
      (hv : s.locals vt = some v)
      (hfree : v.refFree)
      (hs' : s.writeTarget rt v = some s') :
      InstrNext (.call [] .writeRef [t, vt]) s s'
  | freezeRef {s : MoveState} {dst t : LocalIndex} {rt : RefTarget}
      {v : Value}
      (ht : s.locals t = some (.ref rt))
      (hv : s.readTarget rt = some v)
      (hfree : v.refFree) :
      InstrNext (.call [dst] .freezeRef [t]) s
        (s.writeLocal dst (.ref rt))

/-- A non-function instruction which terminates its frame immediately. -/
inductive InstrStop : Instr → MoveState → FrameOutcome → Prop where
  | op {s : MoveState} {dsts srcs : List LocalIndex} {op : Oper}
      {vs : List Value}
      (hsrcs : srcs.mapM s.locals = some vs)
      (hop : op.sem s.current s.readTarget vs s.memory = some .abort) :
      InstrStop (.call dsts op srcs) s
        (.abort s.memory op.abortCode)
  | borrowGlobal {s : MoveState} {dst t : LocalIndex} {r : ResourceId}
      {a : Address}
      (ha : s.locals t = some (.address a))
      (habsent : s.memory r a = none) :
      InstrStop (.call [dst] (.borrowGlobal r) [t]) s
        (.abort s.memory runtimeAbortCode)
  | borrowGlobalInst {s : MoveState} {dst t : LocalIndex} {r : ResourceId}
      {args : List Ty} {a : Address}
      (ha : s.locals t = some (.address a))
      (habsent : s.memory (resourceKey r args) a = none) :
      InstrStop (.call [dst] (.borrowGlobalInst r args) [t]) s
        (.abort s.memory runtimeAbortCode)
  | borrowVecElem {s : MoveState} {dst t it : LocalIndex}
      {rt : RefTarget} {es : List Value} {n : Nat}
      (ht : s.locals t = some (.ref rt))
      (hv : s.readTarget rt = some (.vector es))
      (hi : s.locals it = some (.u64 n))
      (hge : es.length ≤ n) :
      InstrStop (.call [dst] .borrowVecElem [t, it]) s
        (.abort s.memory runtimeAbortCode)

/-- A terminator which selects another block. -/
inductive TermNext (G : Cfg) : Term → MoveState → BlockId → Block → Prop where
  | jump {s : MoveState} {b : BlockId} {blk : Block}
      (hb : G.blocks b = some blk) : TermNext G (.jump b) s b blk
  | branch {s : MoveState} {c : LocalIndex} {b₁ b₂ : BlockId}
      {taken : Bool} {blk : Block}
      (hc : s.locals c = some (.bool taken))
      (hb : G.blocks (if taken then b₁ else b₂) = some blk) :
      TermNext G (.branch c b₁ b₂) s (if taken then b₁ else b₂) blk

/-- A terminator which produces the frame's final outcome. -/
inductive TermStop : Term → MoveState → FrameOutcome → Prop where
  | ret {s : MoveState} {srcs : List LocalIndex} {vals : List Value}
      (hvals : srcs.mapM s.locals = some vals) :
      TermStop (.ret srcs) s (.ret s.finishFrame vals)
  | abort {s : MoveState} {code : LocalIndex} {n : Nat}
      (hcode : s.locals code = some (.u64 n)) :
      TermStop (.abort code) s (.abort s.memory n)

namespace InstrStop

/-- A stopping instruction determines an execution independently of the
unreached suffix and terminator. -/
theorem run {P : Program} {G : Cfg} {i : Instr} {s : MoveState}
    {o : FrameOutcome} (h : InstrStop i s o) {rest : List Instr}
    {term : Term} : RunFrom P G (i :: rest) term s o := by
  cases h with
  | op hsrcs hop => exact .opAbort hsrcs hop
  | borrowGlobal ha habsent => exact .borrowGlobalAbort ha habsent
  | borrowGlobalInst ha habsent => exact .borrowGlobalInstAbort ha habsent
  | borrowVecElem ht hv hi hge =>
      exact .borrowVecElemAbort ht hv hi hge

end InstrStop

namespace InstrNext

/-- Every continuing instruction preserves the active frame identifier. -/
theorem current_eq {i : Instr} {s s' : MoveState}
    (h : InstrNext i s s') : s'.current = s.current := by
  cases h with
  | writeRef _ _ _ hwrite => exact MoveState.writeTarget_current hwrite
  | _ => simp

/-- Prefix a continuing head action to an execution of the suffix. -/
theorem run {P : Program} {G : Cfg} {i : Instr} {s s' : MoveState}
    (h : InstrNext i s s') {rest : List Instr} {term : Term}
    {o : FrameOutcome} (hrest : RunFrom P G rest term s' o) :
    RunFrom P G (i :: rest) term s o := by
  cases h with
  | load => exact .load hrest
  | assign hsrc => exact .assign hsrc hrest
  | nop => exact .nop hrest
  | op hsrcs hlen hop => exact .opOk hsrcs hlen hop hrest
  | isParentMissing hp ht => exact .isParentMissing hp ht hrest
  | borrowLoc hx => exact .borrowLoc hx hrest
  | borrowField ht hs hi => exact .borrowField ht hs hi hrest
  | borrowFieldInst ht hs hi => exact .borrowFieldInst ht hs hi hrest
  | borrowGlobal ha hpresent => exact .borrowGlobalOk ha hpresent hrest
  | borrowGlobalInst ha hpresent => exact .borrowGlobalInstOk ha hpresent hrest
  | borrowVecElem ht hv hi hlt => exact .borrowVecElemOk ht hv hi hlt hrest
  | readRef ht hv hfree => exact .readRef ht hv hfree hrest
  | writeRef ht hv hfree hs' => exact .writeRef ht hv hfree hs' hrest
  | freezeRef ht hv hfree => exact .freezeRef ht hv hfree hrest

end InstrNext

/-- A finite sequence of non-function instructions which all continue.
Transformation proofs use this as the compact semantic certificate for an
instruction rewritten to zero, one, or several target instructions. -/
inductive InstrPath : List Instr → MoveState → MoveState → Prop where
  | nil {s : MoveState} : InstrPath [] s s
  | cons {i : Instr} {is : List Instr} {s s₁ s₂ : MoveState} :
      InstrNext i s s₁ → InstrPath is s₁ s₂ → InstrPath (i :: is) s s₂

namespace InstrPath

/-- Embed one continuing instruction step as a singleton instruction path. -/
theorem one {i : Instr} {s s' : MoveState} (h : InstrNext i s s') :
    InstrPath [i] s s' := .cons h .nil

/-- Concatenate two straight-line instruction paths. -/
theorem append {first second : List Instr} {s₁ s₂ s₃ : MoveState}
    (h₁ : InstrPath first s₁ s₂) (h₂ : InstrPath second s₂ s₃) :
    InstrPath (first ++ second) s₁ s₃ := by
  induction h₁ with
  | nil => exact h₂
  | cons head _ ih => exact .cons head (ih h₂)

/-- A finite continuing instruction path preserves the active frame. -/
theorem current_eq {is : List Instr} {s s' : MoveState}
    (h : InstrPath is s s') : s'.current = s.current := by
  induction h with
  | nil => rfl
  | cons head _ ih => exact ih.trans head.current_eq

/-- Prefix a continuing instruction path to any execution. -/
theorem run {P : Program} {G : Cfg} {pre rest : List Instr}
    {term : Term} {s s' : MoveState} {o : FrameOutcome}
    (h : InstrPath pre s s') (hrest : RunFrom P G rest term s' o) :
    RunFrom P G (pre ++ rest) term s o := by
  induction h with
  | nil => simpa using hrest
  | cons hi _ ih => exact hi.run (ih hrest)

end InstrPath

/-- A finite straight-line instruction execution which stops at its final
instruction.  Transformation proofs use this for rewritten source actions
which abort before the surrounding terminator is reached. -/
inductive InstrStopPath : List Instr → MoveState → FrameOutcome → Prop where
  | stop {i : Instr} {s : MoveState} {o : FrameOutcome} :
      InstrStop i s o → InstrStopPath [i] s o
  | cons {i : Instr} {is : List Instr} {s s₁ : MoveState}
      {o : FrameOutcome} :
      InstrNext i s s₁ → InstrStopPath is s₁ o →
      InstrStopPath (i :: is) s o

namespace InstrStopPath

/-- Prefix a stopping straight-line path to any instruction suffix and
terminator; neither is reached. -/
theorem run {P : Program} {G : Cfg} {pre rest : List Instr}
    {term : Term} {s : MoveState} {o : FrameOutcome}
    (h : InstrStopPath pre s o) : RunFrom P G (pre ++ rest) term s o := by
  induction h with
  | stop head => exact head.run
  | cons head _ ih => exact head.run ih

end InstrStopPath

/-! ## Execution between program points -/

/-- A finite, continuing execution from one instruction/terminator program
point to another in the same CFG.  Unlike `RunFrom`, this relation stops at
an arbitrary suffix rather than at a frame outcome.  It is the generic
certificate used by transformations which expand one source action across
instructions and split blocks. -/
inductive RunTo (G : Cfg) :
    List Instr → Term → MoveState →
    List Instr → Term → MoveState → Prop where
  | refl {is : List Instr} {term : Term} {s : MoveState} :
      RunTo G is term s is term s
  | instr {i : Instr} {is : List Instr} {term : Term} {s s₁ : MoveState}
      {target : List Instr} {targetTerm : Term} {s₂ : MoveState}
      (head : InstrNext i s s₁)
      (rest : RunTo G is term s₁ target targetTerm s₂) :
      RunTo G (i :: is) term s target targetTerm s₂
  | term {term : Term} {s : MoveState} {b : BlockId} {blk : Block}
      {target : List Instr} {targetTerm : Term} {s₂ : MoveState}
      (head : TermNext G term s b blk)
      (next : RunTo G blk.instrs blk.term s target targetTerm s₂) :
      RunTo G [] term s target targetTerm s₂

namespace RunTo

/-- Concatenate two finite executions meeting at the same program point. -/
theorem trans {G : Cfg}
    {is₁ is₂ is₃ : List Instr} {term₁ term₂ term₃ : Term}
    {s₁ s₂ s₃ : MoveState}
    (h₁ : RunTo G is₁ term₁ s₁ is₂ term₂ s₂)
    (h₂ : RunTo G is₂ term₂ s₂ is₃ term₃ s₃) :
    RunTo G is₁ term₁ s₁ is₃ term₃ s₃ := by
  induction h₁ with
  | refl => exact h₂
  | instr head _ ih => exact .instr head (ih h₂)
  | term head _ ih => exact .term head (ih h₂)

/-- Finish a finite program-point execution with a complete frame run. -/
theorem run {P : Program} {G : Cfg}
    {is is' : List Instr} {term term' : Term} {s s' : MoveState}
    {o : FrameOutcome}
    (h : RunTo G is term s is' term' s')
    (hrest : RunFrom P G is' term' s' o) :
    RunFrom P G is term s o := by
  induction h with
  | refl => exact hrest
  | instr head _ ih => exact head.run (ih hrest)
  | term head _ ih =>
      cases head with
      | jump hb => exact .jump hb (ih hrest)
      | @branch _ _ _ _ taken _ hc hb =>
          cases taken with
          | false => exact .branchFalse hc hb (ih hrest)
          | true => exact .branchTrue hc hb (ih hrest)

/-- A straight-line instruction path is a `RunTo` prefix of any suffix. -/
theorem ofInstrPath {G : Cfg} {pre rest : List Instr} {term : Term}
    {s s' : MoveState} (h : InstrPath pre s s') :
    RunTo G (pre ++ rest) term s rest term s' := by
  induction h with
  | nil => exact .refl
  | cons head _ ih => exact .instr head ih

end RunTo

namespace TermNext

/-- The block selected by a continuing terminator is one of its syntactic
successors. -/
theorem succ_mem {G : Cfg} {term : Term} {s : MoveState}
    {b : BlockId} {blk : Block} (h : TermNext G term s b blk) :
    b ∈ termSuccs term := by
  cases h with
  | jump => simp [termSuccs]
  | branch => split <;> simp [termSuccs]

/-- A continuing terminator's selected block is present in its CFG. -/
theorem block {G : Cfg} {term : Term} {s : MoveState}
    {b : BlockId} {blk : Block} (h : TermNext G term s b blk) :
    G.blocks b = some blk := by
  cases h <;> assumption

/-- Transport a selected terminator edge to another CFG and state.  Only a
branch's boolean operand needs to be preserved. -/
theorem transport {G G' : Cfg} {term : Term} {s s' : MoveState}
    {b : BlockId} {blk blk' : Block} (h : TermNext G term s b blk)
    (hblk : G'.blocks b = some blk')
    (hlookup : ∀ x taken, x ∈ termReads term →
      s.locals x = some (.bool taken) → s'.locals x = some (.bool taken)) :
    TermNext G' term s' b blk' := by
  cases h with
  | jump => exact .jump hblk
  | branch hc _ =>
      exact .branch (hlookup _ _ (by simp [termReads]) hc) hblk

/-- Follow a selected CFG edge. -/
theorem run {P : Program} {G : Cfg} {term : Term} {s : MoveState}
    {b : BlockId} {blk : Block} (h : TermNext G term s b blk)
    {o : FrameOutcome} (hnext : RunFrom P G blk.instrs blk.term s o) :
    RunFrom P G [] term s o := by
  cases h with
  | jump hb => exact .jump hb hnext
  | @branch _ _ _ _ taken _ hc hb =>
      cases taken with
      | false => exact .branchFalse hc hb hnext
      | true => exact .branchTrue hc hb hnext

end TermNext

namespace TermStop

/-- A stopping terminator is already a complete execution. -/
theorem run {P : Program} {G : Cfg} {term : Term} {s : MoveState}
    {o : FrameOutcome} (h : TermStop term s o) :
    RunFrom P G [] term s o := by
  cases h with
  | ret hvals => exact .ret hvals
  | abort hcode => exact .abort hcode

end TermStop

/-!
The grouped eliminator is kept as an ordinary theorem rather than a tactic or
macro.  Its motive deliberately does not depend on the proof term: all current
execution metatheory depends on the execution indices, and proof-independent
motives make the interface considerably easier to instantiate and compose.
-/

namespace RunFrom

/-- An execution annotated with a state invariant.  This is the generic
interface for facts established outside the operational semantics, such as
bytecode typing or borrow-checker consistency.  Its six constructors mirror
the grouped execution structure rather than every concrete instruction. -/
inductive Invariant (P : Program) (StateOK : Cfg → MoveState → Prop) :
    Cfg → List Instr → Term → MoveState → FrameOutcome → Prop where
  | instrNext {G i rest term s s' o}
      (ok : StateOK G s) (head : InstrNext i s s')
      (restRun : Invariant P StateOK G rest term s' o) :
      Invariant P StateOK G (i :: rest) term s o
  | instrStop {G i rest term s o}
      (ok : StateOK G s) (head : InstrStop i s o) :
      Invariant P StateOK G (i :: rest) term s o
  | callOk {G rest term s dsts srcs f d args retVals blk world o}
      (ok : StateOK G s)
      (decl : P.funs f = some d)
      (argsRead : srcs.mapM s.locals = some args)
      (arity : args.length = d.numParams)
      (entry : d.body.blocks d.body.entry = some blk)
      (callee : Invariant P StateOK d.body blk.instrs blk.term
        (s.enterCall args) (.ret world retVals))
      (rets : dsts.length = retVals.length)
      (restRun : Invariant P StateOK G rest term
        (MoveState.writeLocals (world.resume s.current) dsts retVals) o) :
      Invariant P StateOK G
        (.call dsts (.function f) srcs :: rest) term s o
  | callAbort {G rest term s dsts srcs f d args blk m' code}
      (ok : StateOK G s)
      (decl : P.funs f = some d)
      (argsRead : srcs.mapM s.locals = some args)
      (arity : args.length = d.numParams)
      (entry : d.body.blocks d.body.entry = some blk)
      (callee : Invariant P StateOK d.body blk.instrs blk.term
        (s.enterCall args) (.abort m' code)) :
      Invariant P StateOK G
        (.call dsts (.function f) srcs :: rest) term s (.abort m' code)
  | termNext {G term s b blk o}
      (ok : StateOK G s) (head : TermNext G term s b blk)
      (next : Invariant P StateOK G blk.instrs blk.term s o) :
      Invariant P StateOK G [] term s o
  | termStop {G term s o}
      (ok : StateOK G s) (head : TermStop term s o) :
      Invariant P StateOK G [] term s o

/-- Erase state-invariant annotations from an execution. -/
theorem Invariant.run {P : Program} {StateOK : Cfg → MoveState → Prop}
    {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
    {o : FrameOutcome} (h : Invariant P StateOK G rest term s o) :
    RunFrom P G rest term s o := by
  induction h with
  | instrNext _ head _ ih => exact head.run ih
  | instrStop _ head => exact head.run
  | callOk _ decl argsRead arity entry _ rets _ ihCallee ihRest =>
      exact .callOk decl argsRead arity entry ihCallee rets ihRest
  | callAbort _ decl argsRead arity entry _ ihCallee =>
      exact .callAbort decl argsRead arity entry ihCallee
  | termNext _ head _ ih => exact head.run ih
  | termStop _ head => exact head.run

/-- The invariant decorating a run holds at its initial state. -/
theorem Invariant.start {P : Program} {StateOK : Cfg → MoveState → Prop}
    {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
    {o : FrameOutcome} (h : Invariant P StateOK G rest term s o) :
    StateOK G s := by
  cases h <;> assumption

/-- Weaken the state invariant decorating an execution. -/
theorem Invariant.mono {P : Program}
    {StateOK StateOK' : Cfg → MoveState → Prop}
    (himp : ∀ G s, StateOK G s → StateOK' G s)
    {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
    {o : FrameOutcome} (h : Invariant P StateOK G rest term s o) :
    Invariant P StateOK' G rest term s o := by
  induction h with
  | instrNext ok head _ ih => exact .instrNext (himp _ _ ok) head ih
  | instrStop ok head => exact .instrStop (himp _ _ ok) head
  | callOk ok decl argsRead arity entry _ rets _ ihCallee ihRest =>
      exact .callOk (himp _ _ ok) decl argsRead arity entry ihCallee rets ihRest
  | callAbort ok decl argsRead arity entry _ ihCallee =>
      exact .callAbort (himp _ _ ok) decl argsRead arity entry ihCallee
  | termNext ok head _ ih => exact .termNext (himp _ _ ok) head ih
  | termStop ok head => exact .termStop (himp _ _ ok) head

/-- Proof obligation for one continuing non-call instruction. -/
abbrev InstrNextCase (P : Program)
    (M : Cfg → List Instr → Term → MoveState → FrameOutcome → Prop) :=
  ∀ {G i rest term s s' o}, InstrNext i s s' →
    RunFrom P G rest term s' o → M G rest term s' o →
    M G (i :: rest) term s o

/-- Proof obligation for one stopping non-call instruction. -/
abbrev InstrStopCase (_P : Program)
    (M : Cfg → List Instr → Term → MoveState → FrameOutcome → Prop) :=
  ∀ {G i rest term s o}, InstrStop i s o → M G (i :: rest) term s o

/-- Proof obligation for a successful function call and caller continuation. -/
abbrev CallOkCase (P : Program)
    (M : Cfg → List Instr → Term → MoveState → FrameOutcome → Prop) :=
  ∀ {G rest term s dsts srcs f d args retVals blk world o},
    P.funs f = some d → srcs.mapM s.locals = some args →
    args.length = d.numParams →
    d.body.blocks d.body.entry = some blk →
    RunFrom P d.body blk.instrs blk.term
      (s.enterCall args) (.ret world retVals) →
    M d.body blk.instrs blk.term
      (s.enterCall args) (.ret world retVals) →
    dsts.length = retVals.length →
    RunFrom P G rest term
      (MoveState.writeLocals (world.resume s.current) dsts retVals) o →
    M G rest term
      (MoveState.writeLocals (world.resume s.current) dsts retVals) o →
    M G (.call dsts (.function f) srcs :: rest) term s o

/-- Proof obligation for an aborting function call. -/
abbrev CallAbortCase (P : Program)
    (M : Cfg → List Instr → Term → MoveState → FrameOutcome → Prop) :=
  ∀ {G rest term s dsts srcs f d args blk m' code},
    P.funs f = some d → srcs.mapM s.locals = some args →
    args.length = d.numParams →
    d.body.blocks d.body.entry = some blk →
    RunFrom P d.body blk.instrs blk.term
      (s.enterCall args) (.abort m' code) →
    M d.body blk.instrs blk.term
      (s.enterCall args) (.abort m' code) →
    M G (.call dsts (.function f) srcs :: rest) term s (.abort m' code)

/-- Proof obligation for a successful type-instantiated function call. -/
abbrev CallInstOkCase (P : Program)
    (M : Cfg → List Instr → Term → MoveState → FrameOutcome → Prop) :=
  ∀ {G rest term s dsts srcs f typeArgs d args retVals blk world o},
    P.funs f = some d → typeArgs.length = d.typeParams.length →
    srcs.mapM s.locals = some args → args.length = d.numParams →
    (d.body.instantiate typeArgs).blocks
      (d.body.instantiate typeArgs).entry = some blk →
    RunFrom P (d.body.instantiate typeArgs) blk.instrs blk.term
      (s.enterCall args) (.ret world retVals) →
    M (d.body.instantiate typeArgs) blk.instrs blk.term
      (s.enterCall args) (.ret world retVals) →
    dsts.length = retVals.length →
    RunFrom P G rest term
      (MoveState.writeLocals (world.resume s.current) dsts retVals) o →
    M G rest term
      (MoveState.writeLocals (world.resume s.current) dsts retVals) o →
    M G (.call dsts (.functionInst f typeArgs) srcs :: rest) term s o

/-- Proof obligation for an aborting type-instantiated function call. -/
abbrev CallInstAbortCase (P : Program)
    (M : Cfg → List Instr → Term → MoveState → FrameOutcome → Prop) :=
  ∀ {G rest term s dsts srcs f typeArgs d args blk m' code},
    P.funs f = some d → typeArgs.length = d.typeParams.length →
    srcs.mapM s.locals = some args → args.length = d.numParams →
    (d.body.instantiate typeArgs).blocks
      (d.body.instantiate typeArgs).entry = some blk →
    RunFrom P (d.body.instantiate typeArgs) blk.instrs blk.term
      (s.enterCall args) (.abort m' code) →
    M (d.body.instantiate typeArgs) blk.instrs blk.term
      (s.enterCall args) (.abort m' code) →
    M G (.call dsts (.functionInst f typeArgs) srcs :: rest) term s (.abort m' code)

/-- Proof obligation for a terminator that follows a CFG edge. -/
abbrev TermNextCase (P : Program)
    (M : Cfg → List Instr → Term → MoveState → FrameOutcome → Prop) :=
  ∀ {G term s b blk o}, TermNext G term s b blk →
    RunFrom P G blk.instrs blk.term s o →
    M G blk.instrs blk.term s o → M G [] term s o

/-- Proof obligation for a stopping terminator. -/
abbrev TermStopCase (_P : Program)
    (M : Cfg → List Instr → Term → MoveState → FrameOutcome → Prop) :=
  ∀ {G term s o}, TermStop term s o → M G [] term s o

/-- Induct over executions using six semantic action classes instead of the
individual `RunFrom` constructors. -/
@[elab_as_elim]
theorem inductGrouped {P : Program}
    {M : Cfg → List Instr → Term → MoveState → FrameOutcome → Prop}
    (instrNext : InstrNextCase P M)
    (instrStop : InstrStopCase P M)
    (callOk : CallOkCase P M)
    (callAbort : CallAbortCase P M)
    (callInstOk : CallInstOkCase P M)
    (callInstAbort : CallInstAbortCase P M)
    (termNext : TermNextCase P M)
    (termStop : TermStopCase P M) :
    ∀ {G rest term s o}, RunFrom P G rest term s o → M G rest term s o := by
  intro G rest term s o h
  induction h with
  | load hrest ih => exact instrNext .load hrest ih
  | assign hsrc hrest ih => exact instrNext (.assign hsrc) hrest ih
  | nop hrest ih => exact instrNext .nop hrest ih
  | opOk hsrcs hlen hop hrest ih =>
      exact instrNext (.op hsrcs hlen hop) hrest ih
  | isParentMissing hp ht hrest ih =>
      exact instrNext (.isParentMissing hp ht) hrest ih
  | opAbort hsrcs hop => exact instrStop (.op hsrcs hop)
  | borrowLoc hx hrest ih => exact instrNext (.borrowLoc hx) hrest ih
  | borrowField ht hs hi hrest ih =>
      exact instrNext (.borrowField ht hs hi) hrest ih
  | borrowFieldInst ht hs hi hrest ih =>
      exact instrNext (.borrowFieldInst ht hs hi) hrest ih
  | borrowGlobalOk ha hpresent hrest ih =>
      exact instrNext (.borrowGlobal ha hpresent) hrest ih
  | borrowGlobalAbort ha habsent =>
      exact instrStop (.borrowGlobal ha habsent)
  | borrowGlobalInstOk ha hpresent hrest ih =>
      exact instrNext (.borrowGlobalInst ha hpresent) hrest ih
  | borrowGlobalInstAbort ha habsent =>
      exact instrStop (.borrowGlobalInst ha habsent)
  | borrowVecElemOk ht hv hi hlt hrest ih =>
      exact instrNext (.borrowVecElem ht hv hi hlt) hrest ih
  | borrowVecElemAbort ht hv hi hge =>
      exact instrStop (.borrowVecElem ht hv hi hge)
  | readRef ht hv hfree hrest ih =>
      exact instrNext (.readRef ht hv hfree) hrest ih
  | writeRef ht hv hfree hs' hrest ih =>
      exact instrNext (.writeRef ht hv hfree hs') hrest ih
  | freezeRef ht hv hfree hrest ih =>
      exact instrNext (.freezeRef ht hv hfree) hrest ih
  | callOk hd hargs hnargs hentry hcallee hlen hrest ihcallee ihrest =>
      exact callOk hd hargs hnargs hentry hcallee ihcallee hlen hrest ihrest
  | callAbort hd hargs hnargs hentry hcallee ihcallee =>
      exact callAbort hd hargs hnargs hentry hcallee ihcallee
  | callInstOk hd htyargs hargs hnargs hentry hcallee hlen hrest ihcallee ihrest =>
      exact callInstOk hd htyargs hargs hnargs hentry hcallee ihcallee hlen hrest ihrest
  | callInstAbort hd htyargs hargs hnargs hentry hcallee ihcallee =>
      exact callInstAbort hd htyargs hargs hnargs hentry hcallee ihcallee
  | jump hb hnext ih => exact termNext (.jump hb) hnext ih
  | branchTrue hc hb hnext ih =>
      exact termNext (.branch (taken := true) hc hb) hnext ih
  | branchFalse hc hb hnext ih =>
      exact termNext (.branch (taken := false) hc hb) hnext ih
  | ret hvals => exact termStop (.ret hvals)
  | abort hcode => exact termStop (.abort hcode)

end RunFrom

/-- Normalize a one-result, memory-preserving operation to `writeLocal`. -/
theorem InstrNext.op_one {s : MoveState} {dst : LocalIndex} {op : Oper}
    {srcs : List LocalIndex} {vs : List Value} {v : Value}
    (hsrcs : srcs.mapM s.locals = some vs)
    (hop : op.sem s.current s.readTarget vs s.memory =
      some (.ok [v] s.memory)) :
    InstrNext (.call [dst] op srcs) s (s.writeLocal dst v) := by
  have hset : s.setMemory s.memory = s := by cases s; rfl
  have hend : (s.setMemory s.memory).writeLocals [dst] [v] =
      s.writeLocal dst v := by rw [hset]; rfl
  rw [← hend]
  exact InstrNext.op hsrcs (by rfl) hop

/-- Normalize a zero-result operation to a memory-only state update. -/
theorem InstrNext.op_zero {s : MoveState} {op : Oper}
    {srcs : List LocalIndex} {vs : List Value} {m' : Memory}
    (hsrcs : srcs.mapM s.locals = some vs)
    (hop : op.sem s.current s.readTarget vs s.memory =
      some (.ok [] m')) :
    InstrNext (.call [] op srcs) s (s.setMemory m') := by
  exact InstrNext.op hsrcs (by rfl) hop

/-- Construct a local-rooted mutation from a reference-free local value. -/
theorem InstrNext.mkMutLoc {s : MoveState} {dst x : LocalIndex} {v : Value}
    (hx : s.locals x = some v) (hfree : v.refFree) :
    InstrNext (.call [dst] (.mkMutLoc x) [x]) s
      (s.writeLocal dst (.mut ⟨.loc s.current x, []⟩ v)) := by
  apply InstrNext.op_one (vs := [v])
  · change [x].mapM s.locals = some [v]
    change s.frames s.current x = some v at hx
    simp [hx]
  · simp [Oper.sem, hfree]

/-- Construct a global-rooted mutation when the resource exists. -/
theorem InstrNext.mkMutGlobal {s : MoveState} {dst aT : LocalIndex}
    {r : ResourceId} {a : Address} {v : Value}
    (ha : s.locals aT = some (.address a))
    (hpresent : s.memory r a = some v) :
    InstrNext (.call [dst] (.mkMutGlobal r) [aT]) s
      (s.writeLocal dst (.mut ⟨.global r a, []⟩ v)) := by
  apply InstrNext.op_one (vs := [.address a])
  · change [aT].mapM s.locals = some [.address a]
    change s.frames s.current aT = some (.address a) at ha
    simp [ha]
  · simp [Oper.sem, hpresent]

/-- Construct a field-child mutation from its parent payload. -/
theorem InstrNext.childMutField {s : MoveState} {dst t : LocalIndex}
    {i : Nat} {rt : RefTarget} {fs : List Value} {v : Value}
    (ht : s.locals t = some (.mut rt (.struct fs)))
    (hfield : fs[i]? = some v) :
    InstrNext (.call [dst] (.childMutField i) [t]) s
      (s.writeLocal dst (.mut ⟨rt.root, rt.path ++ [i]⟩ v)) := by
  apply InstrNext.op_one (vs := [.mut rt (.struct fs)])
  · change [t].mapM s.locals = some [.mut rt (.struct fs)]
    change s.frames s.current t = some (.mut rt (.struct fs)) at ht
    simp [ht]
  · simp [Oper.sem, hfield]

/-- Construct a vector-element child mutation on a successful lookup. -/
theorem InstrNext.childMutIndex {s : MoveState} {dst t iT : LocalIndex}
    {rt : RefTarget} {es : List Value} {n : Nat} {v : Value}
    (ht : s.locals t = some (.mut rt (.vector es)))
    (hi : s.locals iT = some (.u64 n)) (helem : es[n]? = some v) :
    InstrNext (.call [dst] .childMutIndex [t, iT]) s
      (s.writeLocal dst (.mut ⟨rt.root, rt.path ++ [n]⟩ v)) := by
  apply InstrNext.op_one (vs := [.mut rt (.vector es), .u64 n])
  · change [t, iT].mapM s.locals =
      some [.mut rt (.vector es), .u64 n]
    change s.frames s.current t = some (.mut rt (.vector es)) at ht
    change s.frames s.current iT = some (.u64 n) at hi
    simp [ht, hi]
  · simp [Oper.sem, helem]

/-- Read the carried value of a mutation. -/
theorem InstrNext.getMut {s : MoveState} {dst t : LocalIndex}
    {rt : RefTarget} {v : Value} (ht : s.locals t = some (.mut rt v)) :
    InstrNext (.call [dst] .getMut [t]) s (s.writeLocal dst v) := by
  apply InstrNext.op_one (vs := [.mut rt v])
  · change [t].mapM s.locals = some [.mut rt v]
    change s.frames s.current t = some (.mut rt v) at ht
    simp [ht]
  · simp [Oper.sem]

/-- Read an existing struct field. -/
theorem InstrNext.getField {s : MoveState} {dst src i : LocalIndex}
    {fs : List Value} {v : Value}
    (hsrc : s.locals src = some (.struct fs)) (hfield : fs[i]? = some v) :
    InstrNext (.call [dst] (.getField i) [src]) s
      (s.writeLocal dst v) := by
  apply InstrNext.op_one (vs := [.struct fs])
  · change [src].mapM s.locals = some [.struct fs]
    change s.frames s.current src = some (.struct fs) at hsrc
    simp [hsrc]
  · simp [Oper.sem, hfield]

/-- Functionally replace an in-bounds struct field. -/
theorem InstrNext.updateField {s : MoveState} {dst src val i : LocalIndex}
    {fs : List Value} {v : Value}
    (hsrc : s.locals src = some (.struct fs)) (hval : s.locals val = some v)
    (hfree : v.refFree) (hbound : i < fs.length) :
    InstrNext (.call [dst] (.updateField i) [src, val]) s
      (s.writeLocal dst (.struct (fs.set i v))) := by
  apply InstrNext.op_one (vs := [.struct fs, v])
  · change [src, val].mapM s.locals = some [.struct fs, v]
    change s.frames s.current src = some (.struct fs) at hsrc
    change s.frames s.current val = some v at hval
    simp [hsrc, hval]
  · simp [Oper.sem, hfree, hbound]

/-- Read an existing vector element. -/
theorem InstrNext.vecGet {s : MoveState} {dst src idx : LocalIndex}
    {es : List Value} {i : Nat} {v : Value}
    (hsrc : s.locals src = some (.vector es))
    (hidx : s.locals idx = some (.u64 i)) (helem : es[i]? = some v) :
    InstrNext (.call [dst] .vecGet [src, idx]) s
      (s.writeLocal dst v) := by
  apply InstrNext.op_one (vs := [.vector es, .u64 i])
  · change [src, idx].mapM s.locals = some [.vector es, .u64 i]
    change s.frames s.current src = some (.vector es) at hsrc
    change s.frames s.current idx = some (.u64 i) at hidx
    simp [hsrc, hidx]
  · simp [Oper.sem, helem]

/-- Functionally replace an in-bounds vector element. -/
theorem InstrNext.vecSet {s : MoveState} {dst src idx val : LocalIndex}
    {es : List Value} {i : Nat} {v : Value}
    (hsrc : s.locals src = some (.vector es))
    (hidx : s.locals idx = some (.u64 i)) (hval : s.locals val = some v)
    (hfree : v.refFree) (hbound : i < es.length) :
    InstrNext (.call [dst] .vecSet [src, idx, val]) s
      (s.writeLocal dst (.vector (es.set i v))) := by
  apply InstrNext.op_one (vs := [.vector es, .u64 i, v])
  · change [src, idx, val].mapM s.locals =
      some [.vector es, .u64 i, v]
    change s.frames s.current src = some (.vector es) at hsrc
    change s.frames s.current idx = some (.u64 i) at hidx
    change s.frames s.current val = some v at hval
    simp [hsrc, hidx, hval]
  · simp [Oper.sem, hfree, hbound]

/-- Recover one dynamic vector index from a child mutation target. -/
theorem InstrNext.mutPathIndex {s : MoveState} {dst p t : LocalIndex}
    {k n : Nat} {rp rt : RefTarget} {vp vt : Value}
    (hp : s.locals p = some (.mut rp vp))
    (ht : s.locals t = some (.mut rt vt))
    (hindex : rt.path[rp.path.length + k]? = some n)
    (hbound : n < U64_SIZE) :
    InstrNext (.call [dst] (.mutPathIndex k) [p, t]) s
      (s.writeLocal dst (.u64 n)) := by
  apply InstrNext.op_one (vs := [.mut rp vp, .mut rt vt])
  · change [p, t].mapM s.locals = some [.mut rp vp, .mut rt vt]
    change s.frames s.current p = some (.mut rp vp) at hp
    change s.frames s.current t = some (.mut rt vt) at ht
    simp [hp, ht]
  · simp [Oper.sem, hindex, hbound]

/-- Replace the carried value of a mutation while retaining its target. -/
theorem InstrNext.setMut {s : MoveState} {t vt : LocalIndex}
    {rt : RefTarget} {old v : Value}
    (ht : s.locals t = some (.mut rt old)) (hv : s.locals vt = some v)
    (hfree : v.refFree) :
    InstrNext (.call [t] .setMut [t, vt]) s
      (s.writeLocal t (.mut rt v)) := by
  apply InstrNext.op_one (vs := [.mut rt old, v])
  · change [t, vt].mapM s.locals = some [.mut rt old, v]
    change s.frames s.current t = some (.mut rt old) at ht
    change s.frames s.current vt = some v at hv
    simp [ht, hv]
  · simp [Oper.sem, hfree]

/-- Extract the address carried by a global-rooted mutation. -/
theorem InstrNext.mutAddr {s : MoveState} {dst t : LocalIndex}
    {r : ResourceId} {a : Address} {path : List Nat} {v : Value}
    (ht : s.locals t = some (.mut ⟨.global r a, path⟩ v)) :
    InstrNext (.call [dst] .mutAddr [t]) s
      (s.writeLocal dst (.address a)) := by
  apply InstrNext.op_one (vs := [.mut ⟨.global r a, path⟩ v])
  · change [t].mapM s.locals = some [.mut ⟨.global r a, path⟩ v]
    change s.frames s.current t = some (.mut ⟨.global r a, path⟩ v) at ht
    simp [ht]
  · simp [Oper.sem]

/-- Store a reference-free value in global memory. -/
theorem InstrNext.writeGlobal {s : MoveState} {r : ResourceId}
    {aT vT : LocalIndex} {a : Address} {v : Value}
    (ha : s.locals aT = some (.address a))
    (hv : s.locals vT = some v) (hfree : v.refFree) :
    InstrNext (.call [] (.writeGlobal r) [aT, vT]) s
      (s.setMemory (memWrite s.memory r a v)) := by
  apply InstrNext.op_zero (vs := [.address a, v])
  · change [aT, vT].mapM s.locals = some [.address a, v]
    change s.frames s.current aT = some (.address a) at ha
    change s.frames s.current vT = some v at hv
    simp [ha, hv]
  · simp [Oper.sem, hfree]

/-- Test whether one mutation target is the statically described parent of
another. -/
theorem InstrNext.isParent {s : MoveState} {dst p t : LocalIndex}
    {pat : List (Option Nat)} {rp rt : RefTarget} {vp vt : Value}
    (hp : s.locals p = some (.mut rp vp))
    (ht : s.locals t = some (.mut rt vt)) :
    InstrNext (.call [dst] (.isParent pat) [p, t]) s
      (s.writeLocal dst (.bool (isParentTarget pat rp rt))) := by
  apply InstrNext.op_one (vs := [.mut rp vp, .mut rt vt])
  · change [p, t].mapM s.locals = some [.mut rp vp, .mut rt vt]
    change s.frames s.current p = some (.mut rp vp) at hp
    change s.frames s.current t = some (.mut rt vt) at ht
    simp [hp, ht]
  · simp [Oper.sem]

/-- Test whether a mutation denotes the named local root. -/
theorem InstrNext.isMutLoc {s : MoveState} {dst t x : LocalIndex}
    {rt : RefTarget} {v : Value} (ht : s.locals t = some (.mut rt v)) :
    InstrNext (.call [dst] (.isMutLoc x) [t]) s
      (s.writeLocal dst (.bool
        (rt.root == .loc s.current x && rt.path == []))) := by
  apply InstrNext.op_one (vs := [.mut rt v])
  · change [t].mapM s.locals = some [.mut rt v]
    change s.frames s.current t = some (.mut rt v) at ht
    simp [ht]
  · simp [Oper.sem]

/-- Test whether a mutation denotes a root of the named global type. -/
theorem InstrNext.isMutGlobal {s : MoveState} {dst t : LocalIndex}
    {r : ResourceId} {rt : RefTarget} {v : Value}
    (ht : s.locals t = some (.mut rt v)) :
    InstrNext (.call [dst] (.isMutGlobal r) [t]) s
      (s.writeLocal dst (.bool (match rt.root with
        | .global r' _ => r' == r && rt.path.isEmpty
        | .loc _ _ => false))) := by
  apply InstrNext.op_one (vs := [.mut rt v])
  · change [t].mapM s.locals = some [.mut rt v]
    change s.frames s.current t = some (.mut rt v) at ht
    simp [ht]
  · obtain ⟨root, path⟩ := rt
    cases root <;> simp [Oper.sem]

/-- Write a mutation payload back to a local root. -/
theorem InstrPath.writeBackLocal {s : MoveState} {x t : LocalIndex}
    {rt : RefTarget} {v : Value}
    (ht : s.locals t = some (.mut rt v)) :
    InstrPath [.call [x] .getMut [t]] s (s.writeLocal x v) :=
  InstrPath.one (InstrNext.getMut ht)

/-- Copy a child mutation payload back into a directly aliased parent. -/
theorem InstrPath.writeBackParent {s : MoveState} {p t c : LocalIndex}
    {rp rt : RefTarget} {vp v : Value}
    (hp : s.locals p = some (.mut rp vp))
    (ht : s.locals t = some (.mut rt v)) (hfree : v.refFree)
    (hcp : c ≠ p) :
    InstrPath [.call [c] .getMut [t], .call [p] .setMut [p, c]] s
      ((s.writeLocal c v).writeLocal p (.mut rp v)) := by
  let s₁ := s.writeLocal c v
  have hp₁ : s₁.locals p = some (.mut rp vp) := by
    simpa [s₁, MoveState.writeLocal_locals, Ne.symm hcp] using hp
  have hc₁ : s₁.locals c = some v := by simp [s₁]
  exact .cons (InstrNext.getMut ht)
    (.cons (InstrNext.setMut hp₁ hc₁ hfree) .nil)

/-- Write a global-rooted mutation payload back to memory using two fresh
temporary locals. -/
theorem InstrPath.writeBackGlobal {s : MoveState} {r : ResourceId}
    {ad c t : LocalIndex} {a : Address} {path : List Nat} {v : Value}
    (ht : s.locals t = some (.mut ⟨.global r a, path⟩ v))
    (hfree : v.refFree) (hat : ad ≠ t) (hca : c ≠ ad) :
    InstrPath
      [.call [ad] .mutAddr [t], .call [c] .getMut [t],
        .call [] (.writeGlobal r) [ad, c]] s
      (((s.writeLocal ad (.address a)).writeLocal c v).setMemory
        (memWrite s.memory r a v)) := by
  let s₁ := s.writeLocal ad (.address a)
  let s₂ := s₁.writeLocal c v
  have ht₁ : s₁.locals t = some (.mut ⟨.global r a, path⟩ v) := by
    simpa [s₁, MoveState.writeLocal_locals, Ne.symm hat] using ht
  have ha₂ : s₂.locals ad = some (.address a) := by
    simp [s₂, s₁, Ne.symm hca]
  have hv₂ : s₂.locals c = some v := by
    simp [s₂]
  have hmem : memWrite s₂.memory r a v = memWrite s.memory r a v := by
    simp [s₂, s₁]
  have hwrite := InstrNext.writeGlobal (r := r) ha₂ hv₂ hfree
  rw [hmem] at hwrite
  exact .cons (InstrNext.mutAddr ht)
    (.cons (InstrNext.getMut ht₁)
      (.cons hwrite .nil))

/-- A missing resource makes global mutation construction abort. -/
theorem InstrStop.mkMutGlobal {s : MoveState} {dst aT : LocalIndex}
    {r : ResourceId} {a : Address}
    (ha : s.locals aT = some (.address a)) (habsent : s.memory r a = none) :
    InstrStop (.call [dst] (.mkMutGlobal r) [aT]) s
      (.abort s.memory runtimeAbortCode) := by
  apply InstrStop.op (vs := [.address a])
  · change [aT].mapM s.locals = some [.address a]
    change s.frames s.current aT = some (.address a) at ha
    simp [ha]
  · simp [Oper.sem, habsent]

/-- An out-of-bounds child-mutation lookup aborts. -/
theorem InstrStop.childMutIndex {s : MoveState} {dst t iT : LocalIndex}
    {rt : RefTarget} {es : List Value} {n : Nat}
    (ht : s.locals t = some (.mut rt (.vector es)))
    (hi : s.locals iT = some (.u64 n)) (helem : es[n]? = none) :
    InstrStop (.call [dst] .childMutIndex [t, iT]) s
      (.abort s.memory runtimeAbortCode) := by
  apply InstrStop.op (vs := [.mut rt (.vector es), .u64 n])
  · change [t, iT].mapM s.locals =
      some [.mut rt (.vector es), .u64 n]
    change s.frames s.current t = some (.mut rt (.vector es)) at ht
    change s.frames s.current iT = some (.u64 n) at hi
    simp [ht, hi]
  · simp [Oper.sem, helem]

end MoveModel.IR
