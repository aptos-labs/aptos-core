-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.IR.Value
import Move.IR.State
import Move.IR.Syntax
import Move.IR.Semantics

/-!
# A Computable Interpreter for the IR

A fuel-based executable interpreter, so that programs — in particular those
produced by the Move or MASM frontend — can be run with `#eval`.  The relational
semantics (`RunFrom`) uses function-typed memory and locals; the
interpreter uses executable representations: an association list for
memory, a list for locals.  Where the relational semantics is *stuck*
(ill-typed configurations), the interpreter returns `InterpError.stuck`.
The reference operations execute, mirroring the `RunFrom` rules.

Every recursive call consumes one unit of fuel, so termination is
structural; supply ample fuel.  The agreement theorem with `RunFrom` is a
roadmap item.
-/

namespace Move.IR

/-- Executable global memory: an association list over locations. -/
abbrev IMem := List (ResourceId × Address × Value)

namespace IMem

def get (m : IMem) (r : ResourceId) (a : Address) : Option Value :=
  (m.find? (fun e => e.1 == r && e.2.1 == a)).map (·.2.2)

def remove (m : IMem) (r : ResourceId) (a : Address) : IMem :=
  m.filter (fun e => !(e.1 == r && e.2.1 == a))

def set (m : IMem) (r : ResourceId) (a : Address) (v : Value) : IMem :=
  (r, a, v) :: m.remove r a

end IMem

/-- Executable locals: `none` = uninitialized. -/
abbrev ILocals := List (Option Value)

namespace ILocals

def get (l : ILocals) (i : LocalIndex) : Option Value :=
  (l[i]?).join

def set (l : ILocals) (i : LocalIndex) (v : Value) : ILocals :=
  if i < l.length then List.set l i (some v)
  else l ++ List.replicate (i - l.length) none ++ [some v]

def getAll (l : ILocals) (idxs : List LocalIndex) : Option (List Value) :=
  idxs.mapM l.get

def setAll (l : ILocals) : List LocalIndex → List Value → ILocals
  | i :: idxs, v :: vs => (l.set i v).setAll idxs vs
  | _, _ => l

end ILocals

/-- Result of a terminated interpretation. -/
inductive IOutcome where
  | ret (m : IMem) (vals : List Value)
  | abort (m : IMem) (code : Nat)
  deriving Repr, BEq

/-- Interpretation errors: configurations the relational semantics has no
outcome for (`stuck`), and exhausted fuel. -/
inductive InterpError where
  | stuck (reason : String)
  | outOfFuel
  deriving Repr, BEq

/-- Result of one non-call operation. -/
private inductive IOpRes where
  | ok (rets : List Value) (m : IMem)
  | abort

/-- Executable counterpart of `Oper.sem` (`function` is handled by the
interpreter loop). -/
private def interpOp (deref : RefTarget → Option Value) (op : Oper)
    (vs : List Value) (m : IMem) : Except InterpError IOpRes :=
  let arith2 (f : Nat → Nat → Except InterpError IOpRes) :
      Except InterpError IOpRes :=
    match vs with
    | [.u64 i, .u64 j] => f i j
    | _ => throw (.stuck "ill-typed operands")
  match op with
  | .add => arith2 fun i j =>
      pure (if i + j < U64_SIZE then .ok [.u64 (i + j)] m else .abort)
  | .sub => arith2 fun i j =>
      pure (if j ≤ i then .ok [.u64 (i - j)] m else .abort)
  | .mul => arith2 fun i j =>
      pure (if i * j < U64_SIZE then .ok [.u64 (i * j)] m else .abort)
  | .div => arith2 fun i j =>
      pure (if j = 0 then .abort else .ok [.u64 (i / j)] m)
  | .mod => arith2 fun i j =>
      pure (if j = 0 then .abort else .ok [.u64 (i % j)] m)
  | .lt => arith2 fun i j => pure (.ok [.bool (decide (i < j))] m)
  | .le => arith2 fun i j => pure (.ok [.bool (decide (i ≤ j))] m)
  | .eq =>
    match vs with
    | [v₁, v₂] =>
      match v₁.derefWith deref, v₂.derefWith deref with
      | some a, some b => pure (.ok [.bool (a == b)] m)
      | _, _ => throw (.stuck "read through a dangling reference")
    | _ => throw (.stuck "ill-typed operands")
  | .and =>
    match vs with
    | [.bool a, .bool b] => pure (.ok [.bool (a && b)] m)
    | _ => throw (.stuck "ill-typed operands")
  | .or =>
    match vs with
    | [.bool a, .bool b] => pure (.ok [.bool (a || b)] m)
    | _ => throw (.stuck "ill-typed operands")
  | .not =>
    match vs with
    | [.bool a] => pure (.ok [.bool !a] m)
    | _ => throw (.stuck "ill-typed operands")
  | .pack =>
    if Value.refFreeList vs then pure (.ok [.struct vs] m)
    else throw (.stuck "reference packed into a struct")
  | .unpack =>
    match vs with
    | [.struct fs] => pure (.ok fs m)
    | _ => throw (.stuck "ill-typed operands")
  | .getField i =>
    match vs with
    | [.struct fs] =>
      match fs[i]? with
      | some v => pure (.ok [v] m)
      | none => throw (.stuck s!"field {i} out of range")
    | _ => throw (.stuck "ill-typed operands")
  | .getGlobal r =>
    match vs with
    | [.address a] =>
      match m.get r a with
      | some v => pure (.ok [v] m)
      | none => pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .writeGlobal r =>
    match vs with
    | [.address a, v] =>
      if v.refFree then pure (.ok [] (m.set r a v))
      else throw (.stuck "reference stored to global memory")
    | _ => throw (.stuck "ill-typed operands")
  | .moveTo r =>
    match vs with
    | [.address a, v] =>
      if !v.refFree then throw (.stuck "reference stored to global memory")
      else
        match m.get r a with
        | some _ => pure .abort
        | none => pure (.ok [] (m.set r a v))
    | _ => throw (.stuck "ill-typed operands")
  | .moveFrom r =>
    match vs with
    | [.address a] =>
      match m.get r a with
      | some v => pure (.ok [v] (m.remove r a))
      | none => pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .exists_ r =>
    match vs with
    | [.address a] => pure (.ok [.bool (m.get r a).isSome] m)
    | _ => throw (.stuck "ill-typed operands")
  | .updateField i =>
    match vs with
    | [.struct fs, v] =>
      if i < fs.length then pure (.ok [.struct (fs.set i v)] m)
      else throw (.stuck s!"field {i} out of range")
    | _ => throw (.stuck "ill-typed operands")
  | .vecPack =>
    if Value.refFreeList vs then pure (.ok [.vector vs] m)
    else throw (.stuck "reference stored into a vector")
  | .vecLen =>
    match vs with
    | [.vector es] => pure (.ok [.u64 es.length] m)
    | _ => throw (.stuck "ill-typed operands")
  | .vecGet =>
    match vs with
    | [.vector es, .u64 i] =>
      match es[i]? with
      | some v => pure (.ok [v] m)
      | none => pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .vecSet =>
    match vs with
    | [.vector es, .u64 i, v] =>
      if !v.refFree then throw (.stuck "reference stored into a vector")
      else if i < es.length then pure (.ok [.vector (es.set i v)] m)
      else pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .vecPush =>
    match vs with
    | [.vector es, v] =>
      if !v.refFree then throw (.stuck "reference stored into a vector")
      else if es.length + 1 < U64_SIZE then pure (.ok [.vector (es ++ [v])] m)
      else pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .vecPop =>
    match vs with
    | [.vector es] =>
      match es.getLast? with
      | some v => pure (.ok [.vector es.dropLast, v] m)
      | none => pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .mkMutLoc x =>
    match vs with
    | [v] =>
      if v.refFree then pure (.ok [.mut ⟨.loc x, []⟩ v] m)
      else throw (.stuck "reference checked out into a mutation")
    | _ => throw (.stuck "ill-typed operands")
  | .mkMutGlobal r =>
    match vs with
    | [.address a] =>
      match m.get r a with
      | some v => pure (.ok [.mut ⟨.global r a, []⟩ v] m)
      | none => pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .childMutField i =>
    match vs with
    | [.mut t (.struct fs)] =>
      match fs[i]? with
      | some w => pure (.ok [.mut ⟨t.root, t.path ++ [i]⟩ w] m)
      | none => throw (.stuck s!"field {i} out of range")
    | _ => throw (.stuck "ill-typed operands")
  | .childMutIndex =>
    match vs with
    | [.mut t (.vector es), .u64 n] =>
      match es[n]? with
      | some w => pure (.ok [.mut ⟨t.root, t.path ++ [n]⟩ w] m)
      | none => pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .getMut =>
    match vs with
    | [.mut _ v] => pure (.ok [v] m)
    | _ => throw (.stuck "ill-typed operands")
  | .setMut =>
    match vs with
    | [.mut t _, w] =>
      if w.refFree then pure (.ok [.mut t w] m)
      else throw (.stuck "reference stored into a mutation")
    | _ => throw (.stuck "ill-typed operands")
  | .isParent pat =>
    match vs with
    | [.mut tp _, .mut tc _] =>
      pure (.ok [.bool (isParentTarget pat tp tc)] m)
    | _ => throw (.stuck "ill-typed operands")
  | .mutPathIndex k =>
    match vs with
    | [.mut tp _, .mut tc _] =>
      match tc.path[tp.path.length + k]? with
      | some n =>
        if n < U64_SIZE then pure (.ok [.u64 n] m)
        else throw (.stuck "path index out of the u64 range")
      | none => throw (.stuck "path index out of range")
    | _ => throw (.stuck "ill-typed operands")
  | .isMutLoc x =>
    match vs with
    | [.mut t _] =>
      pure (.ok [.bool (t.root == .loc x && t.path == [])] m)
    | _ => throw (.stuck "ill-typed operands")
  | .isMutGlobal r =>
    match vs with
    | [.mut t _] =>
      pure (.ok [.bool (match t.root with
        | .global r' _ => r' == r && t.path == []
        | .loc _ => false)] m)
    | _ => throw (.stuck "ill-typed operands")
  | .mutAddr =>
    match vs with
    | [.mut t _] =>
      match t.root with
      | .global _ a => pure (.ok [.address a] m)
      | .loc _ => throw (.stuck "address of a local-rooted mutation")
    | _ => throw (.stuck "ill-typed operands")
  | .function _ => throw (.stuck "internal: function call in interpOp")
  | .borrowLoc => throw (.stuck "internal: reference op in interpOp")
  | .borrowField _ => throw (.stuck "internal: reference op in interpOp")
  | .borrowGlobal _ => throw (.stuck "internal: reference op in interpOp")
  | .borrowVecElem => throw (.stuck "internal: reference op in interpOp")
  | .readRef => throw (.stuck "internal: reference op in interpOp")
  | .writeRef => throw (.stuck "internal: reference op in interpOp")
  | .freezeRef => throw (.stuck "internal: reference op in interpOp")

/-- Read the value a reference designates (executable counterpart of
`MoveState.readTarget`). -/
private def readTargetI (locals : ILocals) (m : IMem) (t : RefTarget) :
    Option Value :=
  match t.root with
  | .loc x => (locals.get x).bind (·.getPath t.path)
  | .global r a => (m.get r a).bind (·.getPath t.path)

/-- Write through a reference (executable counterpart of
`MoveState.writeTarget`). -/
private def writeTargetI (locals : ILocals) (m : IMem) (t : RefTarget)
    (v : Value) : Option (ILocals × IMem) :=
  match t.root with
  | .loc x => (locals.get x).bind fun root =>
      (root.setPath t.path v).map fun root' => (locals.set x root', m)
  | .global r a => (m.get r a).bind fun root =>
      (root.setPath t.path v).map fun root' => (locals, m.set r a root')

/-- Write through several references, in order (executable counterpart of
`MoveState.writeTargets`). -/
private def writeTargetsI (locals : ILocals) (m : IMem) :
    List RefTarget → List Value → Option (ILocals × IMem)
  | [], [] => some (locals, m)
  | t :: ts, v :: vs =>
      (writeTargetI locals m t v).bind fun (locals, m) =>
        writeTargetsI locals m ts vs
  | _, _ => none

/-- Result of running a block's straight-line instructions. -/
private inductive IInstrsRes where
  | ok (locals : ILocals) (m : IMem)
  | abort (m : IMem) (code : Nat)

mutual

/-- Interprets a function call on a fresh activation record. -/
def interpFun (P : Program) : Nat → FunId → IMem → List Value →
    Except InterpError IOutcome
  | 0, _, _, _ => throw .outOfFuel
  | fuel + 1, f, m, args =>
    match P.funs f with
    | none => throw (.stuck s!"undeclared function {f}")
    | some d => interpBlock P d.body fuel d.body.entry (args.map some) m

/-- Interprets execution from the start of a block. -/
def interpBlock (P : Program) (G : Cfg) : Nat → BlockId → ILocals → IMem →
    Except InterpError IOutcome
  | 0, _, _, _ => throw .outOfFuel
  | fuel + 1, b, locals, m => do
    match G.blocks b with
    | none => throw (.stuck s!"undeclared block {b}")
    | some blk =>
      match ← interpInstrs P (fuel + 1) blk.instrs locals m with
      | .abort m code => pure (.abort m code)
      | .ok locals m =>
        match blk.term with
        | .jump b' => interpBlock P G fuel b' locals m
        | .branch c b₁ b₂ =>
          match locals.get c with
          | some (.bool true) => interpBlock P G fuel b₁ locals m
          | some (.bool false) => interpBlock P G fuel b₂ locals m
          | _ => throw (.stuck "branch on a non-boolean")
        | .ret srcs =>
          match locals.getAll srcs with
          | some vals => pure (.ret m vals)
          | none => throw (.stuck "uninitialized return value")
        | .abort code =>
          match locals.get code with
          | some (.u64 n) => pure (.abort m n)
          | _ => throw (.stuck "abort with a non-u64 code")

/-- Interprets a straight-line instruction list. -/
def interpInstrs (P : Program) : Nat → List Instr → ILocals → IMem →
    Except InterpError IInstrsRes
  | 0, _, _, _ => throw .outOfFuel
  | _ + 1, [], locals, m => pure (.ok locals m)
  | fuel + 1, i :: rest, locals, m => do
    match i with
    | .load dst v => interpInstrs P fuel rest (locals.set dst v) m
    | .assign dst src =>
      match locals.get src with
      | some v => interpInstrs P fuel rest (locals.set dst v) m
      | none => throw (.stuck "read of an uninitialized local")
    | .nop => interpInstrs P fuel rest locals m
    | .call [dst] .borrowLoc [x] =>
      match locals.get x with
      | some _ =>
        interpInstrs P fuel rest (locals.set dst (.ref ⟨.loc x, []⟩)) m
      | none => throw (.stuck "borrow of an uninitialized local")
    | .call [dst] (.borrowField i) [t] =>
      match locals.get t with
      | some (.ref rt) =>
        interpInstrs P fuel rest
          (locals.set dst (.ref ⟨rt.root, rt.path ++ [i]⟩)) m
      | _ => throw (.stuck "borrow_field of a non-reference")
    | .call [dst] (.borrowGlobal r) [t] =>
      match locals.get t with
      | some (.address a) =>
        match m.get r a with
        | some _ =>
          interpInstrs P fuel rest
            (locals.set dst (.ref ⟨.global r a, []⟩)) m
        | none => pure (.abort m runtimeAbortCode)
      | _ => throw (.stuck "borrow_global of a non-address")
    | .call [dst] .borrowVecElem [t, it] =>
      match locals.get t, locals.get it with
      | some (.ref rt), some (.u64 n) =>
        match readTargetI locals m rt with
        | some (.vector es) =>
          if n < es.length then
            interpInstrs P fuel rest
              (locals.set dst (.ref ⟨rt.root, rt.path ++ [n]⟩)) m
          else pure (.abort m runtimeAbortCode)
        | some _ => throw (.stuck "vector element borrow of a non-vector")
        | none => throw (.stuck "read through a dangling reference")
      | _, _ =>
        throw (.stuck "vector element borrow of a non-reference or bad index")
    | .call [dst] .readRef [t] =>
      match locals.get t with
      | some (.ref rt) =>
        match readTargetI locals m rt with
        | some v => interpInstrs P fuel rest (locals.set dst v) m
        | none => throw (.stuck "read through a dangling reference")
      | _ => throw (.stuck "read_ref of a non-reference")
    | .call [] .writeRef [t, vt] =>
      match locals.get t, locals.get vt with
      | some (.ref rt), some v =>
        match writeTargetI locals m rt v with
        | some (locals, m) => interpInstrs P fuel rest locals m
        | none => throw (.stuck "write through a dangling reference")
      | _, _ => throw (.stuck "write_ref of a non-reference")
    | .call [dst] .freezeRef [t] =>
      match locals.get t with
      | some (.ref rt) =>
        interpInstrs P fuel rest (locals.set dst (.ref rt)) m
      | _ => throw (.stuck "freeze_ref of a non-reference")
    | .call dsts (.function f) srcs =>
      match locals.getAll srcs with
      | none => throw (.stuck "uninitialized call argument")
      | some args =>
        match locRefTargets args with
        | [] =>
          match ← interpFun P fuel f m args with
          | .abort m code => pure (.abort m code)
          | .ret m rets =>
            if dsts.length = rets.length then
              interpInstrs P fuel rest (locals.setAll dsts rets) m
            else throw (.stuck "arity mismatch in call results")
        | targets =>
          -- checkout call (see `Semantics.lean`): loc-rooted reference
          -- arguments are checked out into the callee's shadow slots and
          -- written back on return; returned references are re-rooted.
          match P.funs f with
          | none => throw (.stuck s!"undeclared function {f}")
          | some d =>
            match targets.mapM (readTargetI locals m) with
            | none => throw (.stuck "read through a dangling reference")
            | some checked =>
              let base := d.numLocals
              let ext := d.body.extendRets (shadowSlots base targets.length)
              let init : ILocals :=
                (List.range base).map (fun i => (reRootedArgs base args)[i]?)
                  ++ checked.map some
              match ← interpBlock P ext fuel d.body.entry init m with
              | .abort m code => pure (.abort m code)
              | .ret m' rets' =>
                let retVals := rets'.take (rets'.length - targets.length)
                let finals := rets'.drop (rets'.length - targets.length)
                if finals.length ≠ targets.length then
                  throw (.stuck "missing checkout finals")
                else
                  match writeTargetsI locals m' targets finals with
                  | none =>
                    throw (.stuck "write-back through a dangling reference")
                  | some (locals, m') =>
                    match retVals.mapM (reRootRet base targets) with
                    | none =>
                      throw (.stuck "returned reference into the callee frame")
                    | some retVals' =>
                      if dsts.length = retVals'.length then
                        interpInstrs P fuel rest (locals.setAll dsts retVals')
                          m'
                      else throw (.stuck "arity mismatch in call results")
    | .call dsts op srcs =>
      match locals.getAll srcs with
      | none => throw (.stuck "uninitialized operand")
      | some vs =>
        match ← interpOp (readTargetI locals m) op vs m with
        | .abort => pure (.abort m runtimeAbortCode)
        | .ok rets m' =>
          if dsts.length = rets.length then
            interpInstrs P fuel rest (locals.setAll dsts rets) m'
          else throw (.stuck "arity mismatch in operation results")

end

end Move.IR
