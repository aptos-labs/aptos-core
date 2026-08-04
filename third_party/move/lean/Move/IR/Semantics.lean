-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.IR.Value
import Move.IR.State
import Move.IR.Spec
import Move.IR.Contract
import Move.IR.Syntax
import Move.IR.ValueTyping

/-!
# IR Semantics

This module gives a big-step operational semantics for the bytecode CFG.
`RunFrom` starts at a position inside a block and executes the remaining
instructions followed by the terminator.  A terminating run produces a
`FrameOutcome`: either a normal return or an abort with memory and an abort
code.

The relation describes terminating runs only.  Nontermination has no outcome,
matching the partial-correctness interpretation of verification conditions.

Conventions:

* `Oper.sem` is a deterministic partial function for non-call operations.
  `none` means that the arguments are ill typed and execution is stuck.
  `some .abort` denotes a runtime abort, such as arithmetic overflow,
  division by zero, or a resource error.  Runtime aborts use the fixed code
  `runtimeAbortCode`; `Term.abort` uses the value of its operand.
* Branching on a non-boolean local, jumping to an undeclared block, and
  calling an undeclared function are likewise stuck.
* Local reference roots are frame-qualified.  Calls pass reference values
  unchanged, so a callee can read or update its caller's frame directly.
  Borrow analysis guarantees that no reference rooted in the callee frame
  survives return and that output references derive from input references.
* Move has no references to references.  `read_ref` and `write_ref` therefore
  require reference-free payloads.  `freeze_ref` also checks that its target
  is live and reference free.  `borrow_field` and `borrow_vec_elem` validate
  the referenced aggregate and selected position.  Equality compares
  reference-free values with compatible erased runtime shapes.
* Calls require exact argument arity (`args.length = d.numParams`), as in the
  VM.  Otherwise execution is stuck.
* `Oper.function` calls execute the actual callee body ("calls are real").
  Modular verification against callee *contracts* appears as hypotheses of
  the translation theorems, not in this semantics.
* A call installs the callee at frame `current + 1`.  Ordinary operands use
  that frame, while reference arguments retain their original roots.  Return
  retires the callee and resumes the caller; abort discards local-frame state.
  Borrow analysis prevents callee-local roots from escaping, so frame depths
  can be reused.
-/

namespace Move.IR

/-- Abort code used for all runtime aborts. -/
def runtimeAbortCode : Nat := 0

/-- Result of a non-call operation: result values plus the updated memory,
or a runtime abort. -/
inductive OpOutcome where
  | ok (rets : List Value) (m : Memory)
  | abort

/-- Semantics of the non-call operations, as a deterministic partial
function of the operand values and the current memory.  `none` = ill-typed
(stuck); `some .abort` = runtime abort.  `function` calls and the
reference operations are handled relationally by `RunFrom` (they need the
locals resp. the operand *indices*, not just values).

`deref` resolves reference values, used by `eq`: Move's `==` compares the
values references refer to.  Values stored to global memory or packed into
structs and vectors must be reference-free (`Value.refFree`), as in the
Move VM. -/
def Oper.sem (current : FrameId) (deref : RefTarget → Option Value) :
    Oper → List Value → Memory → Option OpOutcome
  | .add, [.u64 i, .u64 j], m =>
      some (if i + j < U64_SIZE then .ok [.u64 (i + j)] m else .abort)
  | .sub, [.u64 i, .u64 j], m =>
      some (if j ≤ i then .ok [.u64 (i - j)] m else .abort)
  | .mul, [.u64 i, .u64 j], m =>
      some (if i * j < U64_SIZE then .ok [.u64 (i * j)] m else .abort)
  | .div, [.u64 i, .u64 j], m =>
      some (if j = 0 then .abort else .ok [.u64 (i / j)] m)
  | .mod, [.u64 i, .u64 j], m =>
      some (if j = 0 then .abort else .ok [.u64 (i % j)] m)
  | .lt, [.u64 i, .u64 j], m => some (.ok [.bool (decide (i < j))] m)
  | .le, [.u64 i, .u64 j], m => some (.ok [.bool (decide (i ≤ j))] m)
  | .eq, [v₁, v₂], m => do
      let a ← v₁.derefWith deref
      let b ← v₂.derefWith deref
      if a.refFree && b.refFree && a.sameTypeShape b then
        pure (.ok [.bool (a == b)] m)
      else none
  | .and, [.bool a, .bool b], m => some (.ok [.bool (a && b)] m)
  | .or, [.bool a, .bool b], m => some (.ok [.bool (a || b)] m)
  | .not, [.bool a], m => some (.ok [.bool !a] m)
  | .pack, vs, m =>
      if Value.refFreeList vs then some (.ok [.struct vs] m) else none
  | .unpack, [.struct fs], m => some (.ok fs m)
  | .getField i, [.struct fs], m => (fs[i]?).map (fun v => .ok [v] m)
  | .updateField i, [.struct fs, v], m =>
      if !v.refFree then none
      else if i < fs.length then some (.ok [.struct (fs.set i v)] m)
      else none
  | .vecPack, vs, m =>
      if Value.refFreeList vs then some (.ok [.vector vs] m) else none
  | .vecLen, [.vector es], m => some (.ok [.u64 es.length] m)
  | .vecGet, [.vector es, .u64 i], m =>
      some (match es[i]? with
        | some v => .ok [v] m
        | none => .abort)
  | .vecSet, [.vector es, .u64 i, v], m =>
      if !v.refFree then none
      else some (if i < es.length then .ok [.vector (es.set i v)] m
        else .abort)
  | .vecPush, [.vector es, v], m =>
      if !v.refFree then none
      else some (if es.length + 1 < U64_SIZE then .ok [.vector (es ++ [v])] m
        else .abort)
  | .vecPop, [.vector es], m =>
      some (match es.getLast? with
        | some v => .ok [.vector es.dropLast, v] m
        | none => .abort)
  | .mkMutLoc x, [v], m =>
      if v.refFree then some (.ok [.mut ⟨.loc current x, []⟩ v] m) else none
  | .mkMutGlobal r, [.address a], m =>
      some (match m r a with
        | some v => .ok [.mut ⟨.global r a, []⟩ v] m
        | none => .abort)
  | .childMutField i, [.mut t (.struct fs)], m =>
      (fs[i]?).map fun w => .ok [.mut ⟨t.root, t.path ++ [i]⟩ w] m
  | .childMutIndex, [.mut t (.vector es), .u64 n], m =>
      some (match es[n]? with
        | some w => .ok [.mut ⟨t.root, t.path ++ [n]⟩ w] m
        | none => .abort)
  | .getMut, [.mut _ v], m => some (.ok [v] m)
  | .setMut, [.mut t _, w], m =>
      if w.refFree then some (.ok [.mut t w] m) else none
  | .isParent pat, [.mut tp _, .mut tc _], m =>
      some (.ok [.bool (isParentTarget pat tp tc)] m)
  | .mutPathIndex k, [.mut tp _, .mut tc _], m =>
      match tc.path[tp.path.length + k]? with
      | some n => if n < U64_SIZE then some (.ok [.u64 n] m) else none
      | none => none
  | .isMutLoc x, [.mut t _], m =>
      some (.ok [.bool (t.root == .loc current x && t.path == [])] m)
  | .isMutGlobal r, [.mut t _], m =>
      some (.ok [.bool (match t.root with
        | .global r' _ => r' == r && t.path == []
        | .loc _ _ => false)] m)
  | .mutAddr, [.mut t _], m =>
      match t.root with
      | .global _ a => some (.ok [.address a] m)
      | .loc _ _ => none
  | .getGlobal r, [.address a], m =>
      some (match m r a with
        | some v => .ok [v] m
        | none => .abort)
  | .writeGlobal r, [.address a, v], m =>
      if v.refFree then some (.ok [] (memWrite m r a v)) else none
  | .moveTo r, [.address a, v], m =>
      if !v.refFree then none
      else
        some (match m r a with
          | some _ => .abort
          | none => .ok [] (memWrite m r a v))
  | .moveFrom r, [.address a], m =>
      some (match m r a with
        | some v => .ok [v] (memRemove m r a)
        | none => .abort)
  | .exists_ r, [.address a], m => some (.ok [.bool (m r a).isSome] m)
  | _, _, _ => none

/-! ## Frame outcomes -/

/-- Result of executing a whole frame.  A normal result carries the frame
store after retiring the callee so that its caller can resume; aborting local
state is deliberately unobservable. -/
inductive FrameOutcome where
  | ret (world : FrameWorld) (vals : List Value)
  | abort (m : Memory) (code : Nat)

/-- Big-step frame execution: from a position inside a block — the remaining
instructions `rest` and the block's terminator `term` of the enclosing CFG
`G` — to a `FrameOutcome`.  Entering a block `b` is `RunBlock`; a whole call
is `FunExec`.  A callee's execution appears as a premise of the caller's
`callOk`/`callAbort` rules, on the callee's own CFG. -/
inductive RunFrom (P : Program) : Cfg → List Instr → Term → MoveState →
    FrameOutcome → Prop where
  -- straight-line instructions
  | load {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dst : LocalIndex} {v : Value} {o : FrameOutcome}
      (hrest : RunFrom P G rest term (s.writeLocal dst v) o) :
      RunFrom P G (.load dst v :: rest) term s o
  | assign {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dst src : LocalIndex} {v : Value} {o : FrameOutcome}
      (hsrc : s.locals src = some v)
      (hrest : RunFrom P G rest term (s.writeLocal dst v) o) :
      RunFrom P G (.assign dst src :: rest) term s o
  | nop {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {o : FrameOutcome}
      (hrest : RunFrom P G rest term s o) :
      RunFrom P G (.nop :: rest) term s o
  | opOk {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dsts srcs : List LocalIndex} {op : Oper} {vs rets : List Value}
      {m' : Memory} {o : FrameOutcome}
      (hsrcs : srcs.mapM s.locals = some vs)
      (hlen : dsts.length = rets.length)
      (hop : op.sem s.current s.readTarget vs s.memory = some (.ok rets m'))
      (hrest : RunFrom P G rest term
        (MoveState.writeLocals (s.setMemory m') dsts rets) o) :
      RunFrom P G (.call dsts op srcs :: rest) term s o
  | opAbort {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dsts srcs : List LocalIndex} {op : Oper} {vs : List Value}
      (hsrcs : srcs.mapM s.locals = some vs)
      (hop : op.sem s.current s.readTarget vs s.memory = some .abort) :
      RunFrom P G (.call dsts op srcs :: rest) term s
        (.abort s.memory runtimeAbortCode)
  /-- A guarded write-back candidate whose may-parent is absent does not
  match.  Unlike ordinary operations, this internal dispatch test is total
  on an uninitialized first operand. -/
  | isParentMissing {G : Cfg} {rest : List Instr} {term : Term}
      {s : MoveState} {dst p t : LocalIndex} {pat : List (Option Nat)}
      {rt : RefTarget} {v : Value} {o : FrameOutcome}
      (hp : s.locals p = none)
      (ht : s.locals t = some (.mut rt v))
      (hrest : RunFrom P G rest term
        (s.writeLocal dst (.bool false)) o) :
      RunFrom P G (.call [dst] (.isParent pat) [p, t] :: rest) term s o
  -- reference operations (references are runtime values; borrow rules
  -- record locations, read/write follow them)
  | borrowLoc {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dst x : LocalIndex} {v : Value} {o : FrameOutcome}
      (hx : s.locals x = some v)
      (hrest : RunFrom P G rest term
        (s.writeLocal dst (.ref ⟨.loc s.current x, []⟩)) o) :
      RunFrom P G (.call [dst] .borrowLoc [x] :: rest) term s o
  | borrowField {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dst t : LocalIndex} {i : Nat} {rt : RefTarget} {fs : List Value}
      {o : FrameOutcome}
      (ht : s.locals t = some (.ref rt))
      (hs : s.readTarget rt = some (.struct fs))
      (hi : i < fs.length)
      (hrest : RunFrom P G rest term
        (s.writeLocal dst (.ref ⟨rt.root, rt.path ++ [i]⟩)) o) :
      RunFrom P G (.call [dst] (.borrowField i) [t] :: rest) term s o
  | borrowGlobalOk {G : Cfg} {rest : List Instr} {term : Term}
      {s : MoveState} {dst t : LocalIndex} {r : ResourceId} {a : Address}
      {v : Value} {o : FrameOutcome}
      (ha : s.locals t = some (.address a))
      (hpresent : s.memory r a = some v)
      (hrest : RunFrom P G rest term
        (s.writeLocal dst (.ref ⟨.global r a, []⟩)) o) :
      RunFrom P G (.call [dst] (.borrowGlobal r) [t] :: rest) term s o
  | borrowGlobalAbort {G : Cfg} {rest : List Instr} {term : Term}
      {s : MoveState} {dst t : LocalIndex} {r : ResourceId} {a : Address}
      (ha : s.locals t = some (.address a))
      (habsent : s.memory r a = none) :
      RunFrom P G (.call [dst] (.borrowGlobal r) [t] :: rest) term s
        (.abort s.memory runtimeAbortCode)
  | borrowVecElemOk {G : Cfg} {rest : List Instr} {term : Term}
      {s : MoveState} {dst t it : LocalIndex} {rt : RefTarget}
      {es : List Value} {n : Nat} {o : FrameOutcome}
      (ht : s.locals t = some (.ref rt))
      (hv : s.readTarget rt = some (.vector es))
      (hi : s.locals it = some (.u64 n))
      (hlt : n < es.length)
      (hrest : RunFrom P G rest term
        (s.writeLocal dst (.ref ⟨rt.root, rt.path ++ [n]⟩)) o) :
      RunFrom P G (.call [dst] .borrowVecElem [t, it] :: rest) term s o
  | borrowVecElemAbort {G : Cfg} {rest : List Instr} {term : Term}
      {s : MoveState} {dst t it : LocalIndex} {rt : RefTarget}
      {es : List Value} {n : Nat}
      (ht : s.locals t = some (.ref rt))
      (hv : s.readTarget rt = some (.vector es))
      (hi : s.locals it = some (.u64 n))
      (hge : es.length ≤ n) :
      RunFrom P G (.call [dst] .borrowVecElem [t, it] :: rest) term s
        (.abort s.memory runtimeAbortCode)
  | readRef {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dst t : LocalIndex} {rt : RefTarget} {v : Value} {o : FrameOutcome}
      (ht : s.locals t = some (.ref rt))
      (hv : s.readTarget rt = some v)
      (hfree : v.refFree)
      (hrest : RunFrom P G rest term (s.writeLocal dst v) o) :
      RunFrom P G (.call [dst] .readRef [t] :: rest) term s o
  | writeRef {G : Cfg} {rest : List Instr} {term : Term} {s s' : MoveState}
      {t vt : LocalIndex} {rt : RefTarget} {v : Value} {o : FrameOutcome}
      (ht : s.locals t = some (.ref rt))
      (hv : s.locals vt = some v)
      (hfree : v.refFree)
      (hs' : s.writeTarget rt v = some s')
      (hrest : RunFrom P G rest term s' o) :
      RunFrom P G (.call [] .writeRef [t, vt] :: rest) term s o
  | freezeRef {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dst t : LocalIndex} {rt : RefTarget} {v : Value} {o : FrameOutcome}
      (ht : s.locals t = some (.ref rt))
      (hv : s.readTarget rt = some v)
      (hfree : v.refFree)
      (hrest : RunFrom P G rest term (s.writeLocal dst (.ref rt)) o) :
      RunFrom P G (.call [dst] .freezeRef [t] :: rest) term s o
  | callOk {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dsts srcs : List LocalIndex} {f : FunId} {d : FunDecl}
      {args retVals : List Value} {blk : Block} {world : FrameWorld}
      {o : FrameOutcome}
      (hd : P.funs f = some d)
      (hargs : srcs.mapM s.locals = some args)
      (hnargs : args.length = d.numParams)
      (hentry : d.body.blocks d.body.entry = some blk)
      (hcallee : RunFrom P d.body blk.instrs blk.term
        (s.enterCall args) (.ret world retVals))
      (hlen : dsts.length = retVals.length)
      (hrest : RunFrom P G rest term
        (MoveState.writeLocals (world.resume s.current) dsts retVals) o) :
      RunFrom P G (.call dsts (.function f) srcs :: rest) term s o
  | callAbort {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dsts srcs : List LocalIndex} {f : FunId} {d : FunDecl}
      {args : List Value} {blk : Block} {m' : Memory} {code : Nat}
      (hd : P.funs f = some d)
      (hargs : srcs.mapM s.locals = some args)
      (hnargs : args.length = d.numParams)
      (hentry : d.body.blocks d.body.entry = some blk)
      (hcallee : RunFrom P d.body blk.instrs blk.term
        (s.enterCall args) (.abort m' code)) :
      RunFrom P G (.call dsts (.function f) srcs :: rest) term s
        (.abort m' code)
  -- terminators
  | jump {G : Cfg} {s : MoveState} {b : BlockId} {blk : Block}
      {o : FrameOutcome}
      (hb : G.blocks b = some blk)
      (hnext : RunFrom P G blk.instrs blk.term s o) :
      RunFrom P G [] (.jump b) s o
  | branchTrue {G : Cfg} {s : MoveState} {c : LocalIndex} {b₁ b₂ : BlockId}
      {blk : Block} {o : FrameOutcome}
      (hc : s.locals c = some (.bool true))
      (hb : G.blocks b₁ = some blk)
      (hnext : RunFrom P G blk.instrs blk.term s o) :
      RunFrom P G [] (.branch c b₁ b₂) s o
  | branchFalse {G : Cfg} {s : MoveState} {c : LocalIndex} {b₁ b₂ : BlockId}
      {blk : Block} {o : FrameOutcome}
      (hc : s.locals c = some (.bool false))
      (hb : G.blocks b₂ = some blk)
      (hnext : RunFrom P G blk.instrs blk.term s o) :
      RunFrom P G [] (.branch c b₁ b₂) s o
  | ret {G : Cfg} {s : MoveState} {srcs : List LocalIndex} {vals : List Value}
      (hvals : srcs.mapM s.locals = some vals) :
      RunFrom P G [] (.ret srcs) s (.ret s.finishFrame vals)
  | abort {G : Cfg} {s : MoveState} {code : LocalIndex} {n : Nat}
      (hcode : s.locals code = some (.u64 n)) :
      RunFrom P G [] (.abort code) s (.abort s.memory n)

/-- Execution of a frame from the start of block `b`. -/
def RunBlock (P : Program) (G : Cfg) (b : BlockId) (s : MoveState)
    (o : FrameOutcome) : Prop :=
  ∃ blk, G.blocks b = some blk ∧ RunFrom P G blk.instrs blk.term s o

/-- Big-step execution of a function on a fresh activation record. -/
def FunExec (P : Program) (f : FunId) (m : Memory) (args : List Value)
    (o : FrameOutcome) : Prop :=
  ∃ d, P.funs f = some d ∧
    args.length = d.numParams ∧
    RunBlock P d.body d.body.entry (MoveState.initial args m) o

set_option maxHeartbeats 500000 in
/-- Non-equality operations do not inspect the supplied reference dereferencer. -/
theorem Oper.sem_deref_irrel {op : Oper} {current : FrameId}
    {deref₁ deref₂ : RefTarget → Option Value} {vs : List Value}
    {m : Memory} (hne : op ≠ .eq) :
    op.sem current deref₁ vs m = op.sem current deref₂ vs m := by
  generalize hi : op.sem current deref₁ vs m = out
  fun_cases op.sem current deref₁ vs m <;> try rfl
  all_goals simp_all [Oper.sem]

/-- **Semantic contract satisfaction.**  Under the precondition:

* every normal outcome satisfies `ensures` and the `modifies` frame
  condition, and certifies that the `aborts` condition evaluates to *false*
  in both its opaque-call pre-state view and definition-exit view;
* every abort outcome certifies that the `aborts` condition holds in both
  views.

Checking both views makes definition verification strong enough to justify
the opaque-call abstraction used by modular verification.  The exit-state
conjunct matches the production Prover's definition-side interpretation.
For this bytecode language — deterministic, with big-step (terminating) executions — it is
equivalent to the biconditional reading of `aborts_if` (FMCAD'26 §II): "the
function aborts iff the condition holds".  The existence direction of that
biconditional additionally needs a termination argument, which is outside
what assert-based verification establishes and therefore not part of this
definition.

The obligation is relative to **well-typed boundary states** (`TypedArgs`,
`TypedMemory`): the prover verifies bytecode whose typing the bytecode
verifier has established, so its verification conditions assume — rather
than check — well-formedness of the boundary (the injected `WellFormed`
assumptions of the multisorted Boogie encoding). -/
def SatisfiesContract (P : Program) (f : FunId) (d : FunDecl) : Prop :=
  ∀ (m : Memory) (args : List Value),
    TypedArgs P.structs d args → TypedMemory P.structs m →
    Holds (preEnv P.structs m args) d.contract.requires →
      (∀ world rets, FunExec P f m args (.ret world rets) →
        Holds (postEnv P.structs m world.memory args rets)
          d.contract.ensures ∧
        agreesOutside (d.contract.footprint (preEnv P.structs m args)) m
          world.memory ∧
        d.contract.abortsFalseAtExit P.structs m world.memory args) ∧
      (∀ m' code, FunExec P f m args (.abort m' code) →
        d.contract.abortsHoldsAtExit P.structs m m' args)

end Move.IR
