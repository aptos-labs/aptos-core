-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.IR.Value
import Move.IR.State
import Move.IR.Spec
import Move.IR.Contract
import Move.IR.Syntax
import Move.IR.Typing

/-!
# IR Semantics

Big-step operational semantics of the bytecode CFG.  A terminating execution
of a frame — starting inside a block, given the remaining instructions and
the block's terminator — relates a pre-state to a `FrameOutcome`: either a
normal return carrying the final memory and return values, or an abort
carrying the memory at the abort point and an abort code.  Nontermination is
the absence of an outcome (the relation captures terminating runs only,
matching the partial-correctness reading of the verification conditions).

Conventions:

* Non-call operations are given by the deterministic partial function
  `Oper.sem`: `none` means the operation is *ill-typed* at its arguments and
  execution is **stuck** (no rule, no outcome) — well-typed Move cannot get
  stuck, and stuckness harmlessly removes ill-typed configurations from all
  theorems; `some .abort` is a *runtime abort* (arithmetic
  overflow/underflow, division by zero, missing resource, resource
  collision).  Since contracts do not constrain abort codes, all runtime
  aborts carry the fixed code `runtimeAbortCode`; `Term.abort` carries its
  operand's value.
* Branching on a non-boolean local, jumping to an undeclared block, and
  calling an undeclared function are likewise stuck.
* `Oper.function` calls execute the actual callee body ("calls are real").
  Modular verification against callee *contracts* appears as hypotheses of
  the translation theorems, not in this semantics.
* Calls **check out** loc-rooted reference arguments (the `Mut` threading
  of the Move Prover's encoding, promoted to the semantics): a `RefTarget`
  rooted at a local names a slot of the *caller's* frame, so the callee
  receives it re-rooted at a fresh *shadow slot* above its declared locals,
  preloaded with the referenced value (`checkoutLocals`).  The callee runs
  against a view of its body whose `ret`s additionally return the shadow
  slots (`Cfg.extendRets`); on return, the caller writes the shadow finals
  back through the argument targets (`MoveState.writeTargets`) and
  re-roots returned references — a reference rooted at shadow slot `k` is
  a continued borrow of the `k`-th argument target (`reRootRet`).
  Global-rooted references pass verbatim: memory is shared across frames.
  This is observationally faithful for borrow-checked code — locals are
  invisible in frame outcomes, exclusive borrows make the deferred
  write-back unobservable, and aborts correctly discard the write-backs.
  A reference rooted at a genuine callee local escaping through `ret` is
  dangling and stuck (the borrow checker rules it out).
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
def Oper.sem (deref : RefTarget → Option Value) :
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
      pure (.ok [.bool (a == b)] m)
  | .and, [.bool a, .bool b], m => some (.ok [.bool (a && b)] m)
  | .or, [.bool a, .bool b], m => some (.ok [.bool (a || b)] m)
  | .not, [.bool a], m => some (.ok [.bool !a] m)
  | .pack, vs, m =>
      if Value.refFreeList vs then some (.ok [.struct vs] m) else none
  | .unpack, [.struct fs], m => some (.ok fs m)
  | .getField i, [.struct fs], m => (fs[i]?).map (fun v => .ok [v] m)
  | .updateField i, [.struct fs, v], m =>
      if i < fs.length then some (.ok [.struct (fs.set i v)] m) else none
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
      if v.refFree then some (.ok [.mut ⟨.loc x, []⟩ v] m) else none
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
      some (.ok [.bool (t.root == .loc x && t.path == [])] m)
  | .isMutGlobal r, [.mut t _], m =>
      some (.ok [.bool (match t.root with
        | .global r' _ => r' == r && t.path == []
        | .loc _ => false)] m)
  | .mutAddr, [.mut t _], m =>
      match t.root with
      | .global _ a => some (.ok [.address a] m)
      | .loc _ => none
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

/-! ## Call-boundary checkout of local references -/

/-- The target of a loc-rooted reference value (`none` for every other
value, including global-rooted references). -/
def Value.locRefTarget? : Value → Option RefTarget
  | .ref ⟨.loc x, p⟩ => some ⟨.loc x, p⟩
  | _ => none

/-- The targets of the loc-rooted reference arguments of a call, in order
of occurrence.  These are checked out at the call boundary; global-rooted
references resolve in the callee as they stand. -/
def locRefTargets (args : List Value) : List RefTarget :=
  args.filterMap Value.locRefTarget?

/-- The argument values as the callee receives them: the `k`-th loc-rooted
reference is re-rooted at shadow slot `base + k`. -/
def reRootedArgs (base : Nat) (args : List Value) : List Value :=
  go 0 args
where
  go (k : Nat) : List Value → List Value
    | [] => []
    | v :: rest =>
      match v.locRefTarget? with
      | some _ => .ref ⟨.loc (base + k), []⟩ :: go (k + 1) rest
      | none => v :: go k rest

/-- The callee's initial locals for a checkout call: the declared slots
from the re-rooted arguments, the shadow slots (above the callee's
declared locals) preloaded with the checked-out values. -/
def checkoutLocals (base : Nat) (args checked : List Value) : Locals :=
  fun i =>
    if i < base then (reRootedArgs base args)[i]? else checked[i - base]?

/-- The shadow-slot indices of a checkout call with `n` checked-out
references. -/
def shadowSlots (base n : Nat) : List LocalIndex :=
  (List.range n).map (base + ·)

/-- Re-root a returned value: a reference rooted at shadow slot `base + k`
is a continued borrow of the `k`-th checked-out target, so it is
transplanted onto it; global-rooted references pass unchanged; a reference
rooted at a genuine callee local is dangling (`none` — the borrow checker
rules it out). -/
def reRootRet (base : Nat) (targets : List RefTarget) : Value → Option Value
  | .ref ⟨.loc j, p⟩ =>
      if j < base then none
      else (targets[j - base]?).map fun t => .ref ⟨t.root, t.path ++ p⟩
  | v => some v

/-- Only reference values have a loc-rooted target. -/
theorem Value.locRefTarget?_some {v : Value} {rt : RefTarget}
    (h : v.locRefTarget? = some rt) : v = .ref rt := by
  cases v with
  | ref rt' =>
    obtain ⟨root, p⟩ := rt'
    cases root with
    | loc x =>
      simp only [locRefTarget?, Option.some.injEq] at h
      rw [← h]
    | global r a => simp [locRefTarget?] at h
  | u64 n => simp [locRefTarget?] at h
  | bool b => simp [locRefTarget?] at h
  | address a => simp [locRefTarget?] at h
  | struct fs => simp [locRefTarget?] at h
  | vector es => simp [locRefTarget?] at h
  | «mut» rt' w => simp [locRefTarget?] at h

/-- A nonempty checkout list exhibits a loc-rooted reference argument. -/
theorem locRefTargets_exists {args : List Value}
    (h : locRefTargets args ≠ []) :
    ∃ v ∈ args, ∃ rt, v.locRefTarget? = some rt := by
  induction args with
  | nil => exact absurd rfl h
  | cons v vs ih =>
    cases hv : v.locRefTarget? with
    | some rt => exact ⟨v, List.mem_cons_self, rt, hv⟩
    | none =>
      have h' : locRefTargets vs ≠ [] := by
        simpa [locRefTargets, List.filterMap_cons, hv] using h
      obtain ⟨w, hmem, rt, hw⟩ := ih h'
      exact ⟨w, List.mem_cons_of_mem _ hmem, rt, hw⟩

/-- Result of executing a whole frame: the boundary state is global memory
plus return values (caller locals are invisible). -/
inductive FrameOutcome where
  | ret (m : Memory) (vals : List Value)
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
      (hop : op.sem s.readTarget vs s.memory = some (.ok rets m'))
      (hrest : RunFrom P G rest term
        (MoveState.writeLocals ⟨s.locals, m'⟩ dsts rets) o) :
      RunFrom P G (.call dsts op srcs :: rest) term s o
  | opAbort {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dsts srcs : List LocalIndex} {op : Oper} {vs : List Value}
      (hsrcs : srcs.mapM s.locals = some vs)
      (hop : op.sem s.readTarget vs s.memory = some .abort) :
      RunFrom P G (.call dsts op srcs :: rest) term s
        (.abort s.memory runtimeAbortCode)
  -- reference operations (references are runtime values; borrow rules
  -- record locations, read/write follow them)
  | borrowLoc {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dst x : LocalIndex} {v : Value} {o : FrameOutcome}
      (hx : s.locals x = some v)
      (hrest : RunFrom P G rest term
        (s.writeLocal dst (.ref ⟨.loc x, []⟩)) o) :
      RunFrom P G (.call [dst] .borrowLoc [x] :: rest) term s o
  | borrowField {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dst t : LocalIndex} {i : Nat} {rt : RefTarget} {o : FrameOutcome}
      (ht : s.locals t = some (.ref rt))
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
      (hrest : RunFrom P G rest term (s.writeLocal dst v) o) :
      RunFrom P G (.call [dst] .readRef [t] :: rest) term s o
  | writeRef {G : Cfg} {rest : List Instr} {term : Term} {s s' : MoveState}
      {t vt : LocalIndex} {rt : RefTarget} {v : Value} {o : FrameOutcome}
      (ht : s.locals t = some (.ref rt))
      (hv : s.locals vt = some v)
      (hs' : s.writeTarget rt v = some s')
      (hrest : RunFrom P G rest term s' o) :
      RunFrom P G (.call [] .writeRef [t, vt] :: rest) term s o
  | freezeRef {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dst t : LocalIndex} {rt : RefTarget} {o : FrameOutcome}
      (ht : s.locals t = some (.ref rt))
      (hrest : RunFrom P G rest term (s.writeLocal dst (.ref rt)) o) :
      RunFrom P G (.call [dst] .freezeRef [t] :: rest) term s o
  | callOk {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dsts srcs : List LocalIndex} {f : FunId} {d : FunDecl}
      {args retVals : List Value} {blk : Block} {m' : Memory}
      {o : FrameOutcome}
      (hd : P.funs f = some d)
      (hargs : srcs.mapM s.locals = some args)
      (hnoref : locRefTargets args = [])
      (hentry : d.body.blocks d.body.entry = some blk)
      (hcallee : RunFrom P d.body blk.instrs blk.term
        ⟨initLocals args, s.memory⟩ (.ret m' retVals))
      (hlen : dsts.length = retVals.length)
      (hrest : RunFrom P G rest term
        (MoveState.writeLocals ⟨s.locals, m'⟩ dsts retVals) o) :
      RunFrom P G (.call dsts (.function f) srcs :: rest) term s o
  | callAbort {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dsts srcs : List LocalIndex} {f : FunId} {d : FunDecl}
      {args : List Value} {blk : Block} {m' : Memory} {code : Nat}
      (hd : P.funs f = some d)
      (hargs : srcs.mapM s.locals = some args)
      (hnoref : locRefTargets args = [])
      (hentry : d.body.blocks d.body.entry = some blk)
      (hcallee : RunFrom P d.body blk.instrs blk.term
        ⟨initLocals args, s.memory⟩ (.abort m' code)) :
      RunFrom P G (.call dsts (.function f) srcs :: rest) term s
        (.abort m' code)
  -- checkout calls: some argument is a loc-rooted reference (see the
  -- module docs)
  | callRefOk {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dsts srcs : List LocalIndex} {f : FunId} {d : FunDecl}
      {args checked retVals finals retVals' : List Value} {blk : Block}
      {m' : Memory} {s' : MoveState} {o : FrameOutcome}
      (hd : P.funs f = some d)
      (hargs : srcs.mapM s.locals = some args)
      (hasref : locRefTargets args ≠ [])
      (hchecked : (locRefTargets args).mapM s.readTarget = some checked)
      (hentry : (d.body.extendRets
          (shadowSlots d.numLocals (locRefTargets args).length)).blocks
          d.body.entry = some blk)
      (hcallee : RunFrom P
        (d.body.extendRets
          (shadowSlots d.numLocals (locRefTargets args).length))
        blk.instrs blk.term
        ⟨checkoutLocals d.numLocals args checked, s.memory⟩
        (.ret m' (retVals ++ finals)))
      (hflen : finals.length = (locRefTargets args).length)
      (hwb : MoveState.writeTargets ⟨s.locals, m'⟩ (locRefTargets args)
        finals = some s')
      (hreroot : retVals.mapM (reRootRet d.numLocals (locRefTargets args))
        = some retVals')
      (hlen : dsts.length = retVals'.length)
      (hrest : RunFrom P G rest term (s'.writeLocals dsts retVals') o) :
      RunFrom P G (.call dsts (.function f) srcs :: rest) term s o
  | callRefAbort {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dsts srcs : List LocalIndex} {f : FunId} {d : FunDecl}
      {args checked : List Value} {blk : Block} {m' : Memory} {code : Nat}
      (hd : P.funs f = some d)
      (hargs : srcs.mapM s.locals = some args)
      (hasref : locRefTargets args ≠ [])
      (hchecked : (locRefTargets args).mapM s.readTarget = some checked)
      (hentry : (d.body.extendRets
          (shadowSlots d.numLocals (locRefTargets args).length)).blocks
          d.body.entry = some blk)
      (hcallee : RunFrom P
        (d.body.extendRets
          (shadowSlots d.numLocals (locRefTargets args).length))
        blk.instrs blk.term
        ⟨checkoutLocals d.numLocals args checked, s.memory⟩
        (.abort m' code)) :
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
      RunFrom P G [] (.ret srcs) s (.ret s.memory vals)
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
    RunBlock P d.body d.body.entry ⟨initLocals args, m⟩ o

/-- **Semantic contract satisfaction.**  Under the precondition:

* every normal outcome satisfies `ensures` and the `modifies` frame
  condition, and certifies that the `aborts` condition evaluates to *false*
  (tightness of `aborts_if`);
* every abort outcome certifies that the `aborts` condition holds
  (completeness of `aborts_if`).

This per-execution form is what the TACAS'22 Fig. 8 exit assertions check.
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
      (∀ m' rets, FunExec P f m args (.ret m' rets) →
        Holds (postEnv P.structs m m' args rets) d.contract.ensures ∧
        agreesOutside (d.contract.footprint (preEnv P.structs m args)) m m' ∧
        d.contract.abortsFalse (preEnv P.structs m args)) ∧
      (∀ m' code, FunExec P f m args (.abort m' code) →
        d.contract.abortsHolds (preEnv P.structs m args))

end Move.IR
