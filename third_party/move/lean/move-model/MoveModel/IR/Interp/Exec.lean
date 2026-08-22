-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Value
import MoveModel.IR.State
import MoveModel.IR.Syntax
import MoveModel.IR.Semantics

/-!
# A Computable Interpreter for the IR

A fuel-based interpreter makes frontend-produced programs executable with
`#eval`.  `RunFrom` represents memory and locals as functions.  The interpreter
instead uses an association list for memory and a list for locals.

When the relational semantics is stuck, the interpreter returns
`InterpError.stuck`.  Reference operations follow the corresponding
`RunFrom` rules.

Every recursive call consumes one unit of fuel, making termination structural.
Callers must supply enough fuel. `Interp/Correctness.lean` proves soundness with
respect to `RunFrom`.
-/

namespace MoveModel.IR

/-- Executable global memory: an association list over locations. -/
abbrev IMem := List (ResourceKey × Address × Value)

namespace IMem

/-- Look up one resource in executable association-list memory. -/
def get (m : IMem) (r : ResourceKey) (a : Address) : Option Value :=
  (m.find? (fun e => e.1 == r && e.2.1 == a)).map (·.2.2)

/-- Remove one resource from executable memory. -/
def remove : IMem → ResourceKey → Address → IMem
  | [], _, _ => []
  | (r', a', v) :: m, r, a =>
      if r' == r && a' == a then remove m r a
      else (r', a', v) :: remove m r a

/-- Insert or replace one resource in executable memory. -/
def set (m : IMem) (r : ResourceKey) (a : Address) (v : Value) : IMem :=
  (r, a, v) :: m.remove r a

/-- Interpret executable memory as the function-valued semantic memory. -/
def denote (m : IMem) : Memory := fun r a => m.get r a

end IMem

/-- Executable locals: `none` = uninitialized. -/
abbrev ILocals := List (Option Value)

namespace ILocals

/-- Look up one executable local, returning `none` outside the list. -/
def get (l : ILocals) (i : LocalIndex) : Option Value :=
  (l[i]?).join

/-- Extend as needed and write one executable local. -/
def set : ILocals → LocalIndex → Value → ILocals
  | [], 0, v => [some v]
  | [], i + 1, v => none :: set [] i v
  | _ :: l, 0, v => some v :: l
  | x :: l, i + 1, v => x :: set l i v

/-- Read a list of executable locals, failing if any is unavailable. -/
def getAll (l : ILocals) (idxs : List LocalIndex) : Option (List Value) :=
  idxs.mapM l.get

/-- Write paired local indices and values, stopping when either list ends. -/
def setAll (l : ILocals) : List LocalIndex → List Value → ILocals
  | i :: idxs, v :: vs => (l.set i v).setAll idxs vs
  | _, _ => l

/-- Interpret executable locals as the function-valued semantic locals. -/
def denote (l : ILocals) : Locals := l.get

end ILocals

/-- Executable call-frame store, indexed by call depth. -/
abbrev IFrames := List ILocals

namespace IFrames

/-- Look up an executable frame, returning empty locals when absent. -/
def get (frames : IFrames) (frame : FrameId) : ILocals :=
  (frames[frame]?).getD []

/-- Extend as needed and replace one executable frame. -/
def set : IFrames → FrameId → ILocals → IFrames
  | [], 0, locals => [locals]
  | [], frame + 1, locals => [] :: set [] frame locals
  | _ :: frames, 0, locals => locals :: frames
  | head :: frames, frame + 1, locals => head :: set frames frame locals

end IFrames

/-- Executable counterpart of `MoveState`. -/
structure IState where
  current : FrameId
  frames : IFrames
  memory : IMem
  deriving Repr, BEq

/-- Executable counterpart of `FrameWorld`. -/
structure IWorld where
  frames : IFrames
  memory : IMem
  deriving Repr, BEq

namespace IState

/-- Return the executable locals of the current frame. -/
def locals (s : IState) : ILocals := s.frames.get s.current

/-- Construct the executable state of an external function invocation. -/
def initial (args : List Value) (memory : IMem) : IState :=
  { current := 0, frames := [args.map some], memory }

/-- Replace the locals of the current executable frame. -/
def setLocals (s : IState) (locals : ILocals) : IState :=
  { s with frames := s.frames.set s.current locals }

/-- Read one local from the current executable frame. -/
def getLocal (s : IState) (idx : LocalIndex) : Option Value :=
  s.locals.get idx

/-- Write one local in the current executable frame. -/
def writeLocal (s : IState) (idx : LocalIndex) (value : Value) : IState :=
  s.setLocals (s.locals.set idx value)

/-- Write paired result values into current-frame destination locals. -/
def writeLocals : IState → List LocalIndex → List Value → IState
  | s, idx :: idxs, value :: values =>
      (s.writeLocal idx value).writeLocals idxs values
  | s, _, _ => s

/-- Enter an executable callee frame initialized with arguments. -/
def enterCall (s : IState) (args : List Value) : IState :=
  let child := s.current + 1
  { current := child
    frames := s.frames.set child (args.map some)
    memory := s.memory }

/-- Retire the active executable frame and produce a returned world. -/
def finishFrame (s : IState) : IWorld :=
  { frames := s.frames.set s.current [], memory := s.memory }

/-- Replace executable global memory. -/
def setMemory (s : IState) (memory : IMem) : IState := { s with memory }

end IState

namespace IWorld

/-- Resume an executable caller frame from a returned world. -/
def resume (world : IWorld) (caller : FrameId) : IState :=
  { current := caller, frames := world.frames, memory := world.memory }

end IWorld

/-- Result of a terminated interpretation. -/
inductive IOutcome where
  | ret (world : IWorld) (vals : List Value)
  | abort (m : IMem) (code : Nat)
  deriving Repr, BEq

/-- Interpretation errors: configurations the relational semantics has no
outcome for (`stuck`), and exhausted fuel. -/
inductive InterpError where
  | stuck (reason : String)
  | outOfFuel
  deriving Repr, BEq

/-- Result of one non-call operation. -/
inductive IOpRes where
  | ok (rets : List Value) (m : IMem)
  | abort

/-- Executable counterpart of `Oper.sem` (`function` is handled by the
interpreter loop). -/
def interpOp (current : FrameId) (deref : RefTarget → Option Value) (op : Oper)
    (vs : List Value) (m : IMem) : Except InterpError IOpRes :=
  let arith2 (f : Int → Int → Except InterpError IOpRes) :
      Except InterpError IOpRes :=
    match vs with
    | [.int i, .int j] => f i j
    | _ => throw (.stuck "ill-typed operands")
  match op with
  | .add nt => arith2 fun i j =>
      pure (if nt.lo ≤ i + j ∧ i + j < nt.hi then .ok [.int (i + j)] m else .abort)
  | .sub nt => arith2 fun i j =>
      pure (if nt.lo ≤ i - j ∧ i - j < nt.hi then .ok [.int (i - j)] m else .abort)
  | .mul nt => arith2 fun i j =>
      pure (if nt.lo ≤ i * j ∧ i * j < nt.hi then .ok [.int (i * j)] m else .abort)
  | .div nt => arith2 fun i j =>
      pure (if j = 0 then .abort
        else if nt.lo ≤ i.tdiv j ∧ i.tdiv j < nt.hi then .ok [.int (i.tdiv j)] m
          else .abort)
  | .mod _ => arith2 fun i j =>
      pure (if j = 0 then .abort else .ok [.int (i.tmod j)] m)
  | .bitAnd nt => arith2 fun i j =>
      pure (.ok [.int (nt.fromBits ((nt.toBits i).toNat &&& (nt.toBits j).toNat))] m)
  | .bitOr nt => arith2 fun i j =>
      pure (.ok [.int (nt.fromBits ((nt.toBits i).toNat ||| (nt.toBits j).toNat))] m)
  | .bitXor nt => arith2 fun i j =>
      pure (.ok [.int (nt.fromBits ((nt.toBits i).toNat ^^^ (nt.toBits j).toNat))] m)
  | .shl nt => arith2 fun i k =>
      pure (if k < (nt.width.bits : Int) then
        .ok [.int (nt.fromBits (((nt.toBits i).toNat <<< k.toNat) % nt.size : Nat))] m
      else .abort)
  | .shr nt => arith2 fun i k =>
      pure (if k < (nt.width.bits : Int) then .ok [.int (i.fdiv (2 ^ k.toNat))] m
        else .abort)
  | .cast target =>
    match vs with
    | [.int i] =>
        pure (if target.lo ≤ i ∧ i < target.hi then .ok [.int i] m else .abort)
    | _ => throw (.stuck "ill-typed operands")
  | .lt =>
    match vs with
    | [v₁, v₂] =>
      match v₁.derefWith deref, v₂.derefWith deref with
      | some (.int i), some (.int j) =>
          pure (.ok [.bool (decide (i < j))] m)
      | some a, some b =>
          if a.refFree && b.refFree && a.sameTypeShape b then
            pure (.ok [.bool (compare a b == .lt)] m)
          else throw (.stuck "ill-typed comparison operands")
      | _, _ => throw (.stuck "read through a dangling reference")
    | _ => throw (.stuck "ill-typed operands")
  | .le => arith2 fun i j => pure (.ok [.bool (decide (i ≤ j))] m)
  | .eq =>
    match vs with
    | [v₁, v₂] =>
      match v₁.derefWith deref, v₂.derefWith deref with
      | some a, some b =>
        if a.refFree && b.refFree && a.sameTypeShape b then
          pure (.ok [.bool (a == b)] m)
        else throw (.stuck "ill-typed equality operands")
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
  | .packInst _ =>
    if Value.refFreeList vs then pure (.ok [.struct vs] m)
    else throw (.stuck "reference packed into a struct")
  | .unpack =>
    match vs with
    | [.struct fs] => pure (.ok fs m)
    | _ => throw (.stuck "ill-typed operands")
  | .unpackInst _ =>
    match vs with
    | [.struct fs] => pure (.ok fs m)
    | _ => throw (.stuck "ill-typed operands")
  | .packVariant tag =>
    if Value.refFreeList vs then pure (.ok [.variant tag vs] m)
    else throw (.stuck "reference packed into an enum variant")
  | .packVariantInst tag _ =>
    if Value.refFreeList vs then pure (.ok [.variant tag vs] m)
    else throw (.stuck "reference packed into an enum variant")
  | .unpackVariant tag =>
    match vs with
    | [.variant actual fs] =>
        if actual = tag then pure (.ok fs m) else pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .unpackVariantInst tag _ =>
    match vs with
    | [.variant actual fs] =>
        if actual = tag then pure (.ok fs m) else pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .testVariant tag =>
    match vs with
    | [.variant actual _] => pure (.ok [.bool (actual = tag)] m)
    | _ => throw (.stuck "ill-typed operands")
  | .testVariantInst tag _ =>
    match vs with
    | [.variant actual _] => pure (.ok [.bool (actual = tag)] m)
    | _ => throw (.stuck "ill-typed operands")
  | .getField i =>
    match vs with
    | [.struct fs] =>
      match fs[i]? with
      | some v => pure (.ok [v] m)
      | none => throw (.stuck s!"field {i} out of range")
    | _ => throw (.stuck "ill-typed operands")
  | .getFieldInst i _ =>
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
  | .getGlobalInst r args =>
    match vs with
    | [.address a] =>
      match m.get (resourceKey r args) a with
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
  | .moveToInst r args =>
    match vs with
    | [.address a, v] =>
      if !v.refFree then throw (.stuck "reference stored to global memory")
      else
        match m.get (resourceKey r args) a with
        | some _ => pure .abort
        | none => pure (.ok [] (m.set (resourceKey r args) a v))
    | _ => throw (.stuck "ill-typed operands")
  | .moveFrom r =>
    match vs with
    | [.address a] =>
      match m.get r a with
      | some v => pure (.ok [v] (m.remove r a))
      | none => pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .moveFromInst r args =>
    match vs with
    | [.address a] =>
      match m.get (resourceKey r args) a with
      | some v => pure (.ok [v] (m.remove (resourceKey r args) a))
      | none => pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .exists_ r =>
    match vs with
    | [.address a] => pure (.ok [.bool (m.get r a).isSome] m)
    | _ => throw (.stuck "ill-typed operands")
  | .existsInst r args =>
    match vs with
    | [.address a] =>
      pure (.ok [.bool (m.get (resourceKey r args) a).isSome] m)
    | _ => throw (.stuck "ill-typed operands")
  | .updateField i =>
    match vs with
    | [.struct fs, v] =>
      if !v.refFree then throw (.stuck "reference stored into a struct")
      else if i < fs.length then pure (.ok [.struct (fs.set i v)] m)
      else throw (.stuck s!"field {i} out of range")
    | _ => throw (.stuck "ill-typed operands")
  | .vecPack =>
    if Value.refFreeList vs then pure (.ok [.vector vs] m)
    else throw (.stuck "reference stored into a vector")
  | .vecLen =>
    match vs with
    | [value] =>
      match value.derefWith deref with
      | some (.vector es) => pure (.ok [.u64 es.length] m)
      | some _ => throw (.stuck "vector length of a non-vector")
      | none => throw (.stuck "read through a dangling reference")
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
  | .vecInsert =>
    match vs with
    | [.vector es, .u64 i, v] =>
      if !v.refFree then throw (.stuck "reference stored into a vector")
      else if i ≤ es.length && es.length + 1 < U64_SIZE then
        pure (.ok [.vector (es.take i ++ v :: es.drop i)] m)
      else pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .vecRemove =>
    match vs with
    | [.vector es, .u64 i] =>
      match es[i]? with
      | some v => pure (.ok [.vector (es.take i ++ es.drop (i + 1)), v] m)
      | none => pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .vecSwap =>
    match vs with
    | [.vector es, .u64 i, .u64 j] =>
      match es[i]?, es[j]? with
      | some vi, some vj => pure (.ok [.vector ((es.set i vj).set j vi)] m)
      | _, _ => pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .vecSwapRemove =>
    match vs with
    | [.vector es, .u64 i] =>
      match es[i]?, es.getLast? with
      | some removed, some last => pure (.ok [.vector (es.set i last).dropLast, removed] m)
      | _, _ => pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .vecAppend =>
    match vs with
    | [.vector lhs, .vector rhs] =>
      if lhs.length + rhs.length < U64_SIZE then pure (.ok [.vector (lhs ++ rhs)] m)
      else pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .vecReverse =>
    match vs with
    | [.vector es] => pure (.ok [.vector es.reverse] m)
    | _ => throw (.stuck "ill-typed operands")
  | .vecReverseSlice =>
    match vs with
    | [.vector es, .u64 left, .u64 right] =>
      if left ≤ right && right ≤ es.length then
        pure (.ok [.vector (es.take left ++ ((es.drop left).take (right - left)).reverse ++
          es.drop right)] m)
      else pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .vecContains =>
    match vs with
    | [.vector es, v] => pure (.ok [.bool (es.any (· == v))] m)
    | _ => throw (.stuck "ill-typed operands")
  | .vecIndexOf =>
    match vs with
    | [.vector es, v] =>
      let i := es.findIdx (· == v)
      pure (.ok [.bool (i < es.length), .u64 (if i < es.length then i else 0)] m)
    | _ => throw (.stuck "ill-typed operands")
  | .vecTrim =>
    match vs with
    | [.vector es, .u64 newLen] =>
      if newLen ≤ es.length then
        pure (.ok [.vector (es.take newLen), .vector (es.drop newLen)] m)
      else pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .vecTrimReverse =>
    match vs with
    | [.vector es, .u64 newLen] =>
      if newLen ≤ es.length then
        pure (.ok [.vector (es.take newLen), .vector (es.drop newLen).reverse] m)
      else pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .vecRotate =>
    match vs with
    | [.vector es, .u64 rot] =>
      if rot ≤ es.length then
        pure (.ok [.vector (es.drop rot ++ es.take rot), .u64 (es.length - rot)] m)
      else pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .vecRotateSlice =>
    match vs with
    | [.vector es, .u64 left, .u64 rot, .u64 right] =>
      if left ≤ rot && rot ≤ right && right ≤ es.length then
        pure (.ok [.vector (es.take left ++ (es.drop rot).take (right - rot) ++
          (es.drop left).take (rot - left) ++ es.drop right),
          .u64 (left + (right - rot))] m)
      else pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .vecDestroyEmpty =>
    match vs with
    | [.vector es] => if es.isEmpty then pure (.ok [] m) else pure .abort
    | _ => throw (.stuck "ill-typed operands")
  | .mkMutLoc x =>
    match vs with
    | [v] =>
      if v.refFree then pure (.ok [.mut ⟨.loc current x, []⟩ v] m)
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
      pure (.ok [.bool (t.root == .loc current x && t.path == [])] m)
    | _ => throw (.stuck "ill-typed operands")
  | .isMutGlobal r =>
    match vs with
    | [.mut t _] =>
      pure (.ok [.bool (match t.root with
        | .global r' _ => r' == (r : ResourceKey) && t.path == []
        | .loc _ _ => false)] m)
    | _ => throw (.stuck "ill-typed operands")
  | .mutAddr =>
    match vs with
    | [.mut t _] =>
      match t.root with
      | .global _ a => pure (.ok [.address a] m)
      | .loc _ _ => throw (.stuck "address of a local-rooted mutation")
    | _ => throw (.stuck "ill-typed operands")
  | .function _ => throw (.stuck "internal: function call in interpOp")
  | .functionInst _ _ => throw (.stuck "internal: function call in interpOp")
  | .borrowLoc => throw (.stuck "internal: reference op in interpOp")
  | .borrowField _ => throw (.stuck "internal: reference op in interpOp")
  | .borrowFieldInst _ _ => throw (.stuck "internal: reference op in interpOp")
  | .borrowGlobal _ => throw (.stuck "internal: reference op in interpOp")
  | .borrowGlobalInst _ _ => throw (.stuck "internal: reference op in interpOp")
  | .borrowVecElem => throw (.stuck "internal: reference op in interpOp")
  | .readRef => throw (.stuck "internal: reference op in interpOp")
  | .writeRef => throw (.stuck "internal: reference op in interpOp")
  | .freezeRef => throw (.stuck "internal: reference op in interpOp")

/-- Read the value a stable reference designates. -/
def readTargetI (s : IState) (t : RefTarget) : Option Value :=
  match t.root with
  | .loc frame x => (s.frames.get frame).get x |>.bind (·.getPath t.path)
  | .global r a => (s.memory.get r a).bind (·.getPath t.path)

/-- Write through a stable reference. -/
def writeTargetI (s : IState) (t : RefTarget) (v : Value) : Option IState :=
  match t.root with
  | .loc frame x =>
      let locals := s.frames.get frame
      (locals.get x).bind fun root =>
        (root.setPath t.path v).map fun root' =>
          { s with frames := s.frames.set frame (locals.set x root') }
  | .global r a => (s.memory.get r a).bind fun root =>
      (root.setPath t.path v).map fun root' =>
        { s with memory := s.memory.set r a root' }

/-- Result of running a block's straight-line instructions. -/
inductive IInstrsRes where
  | ok (state : IState)
  | abort (m : IMem) (code : Nat)

mutual

/-- Interprets a function call on a fresh activation record. -/
def interpFun (P : Program) : Nat → FunId → IMem → List Value →
    Except InterpError IOutcome
  | 0, _, _, _ => throw .outOfFuel
  | fuel + 1, f, m, args =>
    match P.funs f with
    | none => throw (.stuck s!"undeclared function {f}")
    | some d =>
        if d.native then
          throw (.stuck s!"native function {f} has no registered interpreter implementation")
        else if args.length = d.numParams then
          interpBlock P d.body fuel d.body.entry (IState.initial args m)
        else throw (.stuck "function argument arity mismatch")

/-- Interprets execution from the start of a block. -/
def interpBlock (P : Program) (G : Cfg) : Nat → BlockId → IState →
    Except InterpError IOutcome
  | 0, _, _ => throw .outOfFuel
  | fuel + 1, b, s => do
    match G.blocks b with
    | none => throw (.stuck s!"undeclared block {b}")
    | some blk =>
      match ← interpInstrs P (fuel + 1) blk.instrs s with
      | .abort m code => pure (.abort m code)
      | .ok s =>
        match blk.term with
        | .jump b' => interpBlock P G fuel b' s
        | .branch c b₁ b₂ =>
          match s.getLocal c with
          | some (.bool true) => interpBlock P G fuel b₁ s
          | some (.bool false) => interpBlock P G fuel b₂ s
          | _ => throw (.stuck "branch on a non-boolean")
        | .ret srcs =>
          match srcs.mapM s.getLocal with
          | some vals => pure (.ret s.finishFrame vals)
          | none => throw (.stuck "uninitialized return value")
        | .abort code =>
          match s.getLocal code with
          | some (.u64 n) => pure (.abort s.memory n)
          | _ => throw (.stuck "abort with a non-u64 code")

/-- Interprets a straight-line instruction list. -/
def interpInstrs (P : Program) : Nat → List Instr → IState →
    Except InterpError IInstrsRes
  | 0, _, _ => throw .outOfFuel
  | _ + 1, [], s => pure (.ok s)
  | fuel + 1, i :: rest, s => do
    match i with
    | .load dst v => interpInstrs P fuel rest (s.writeLocal dst v)
    | .assign dst src =>
      match s.getLocal src with
      | some v => interpInstrs P fuel rest (s.writeLocal dst v)
      | none => throw (.stuck "read of an uninitialized local")
    | .nop => interpInstrs P fuel rest s
    | .call [dst] .borrowLoc [x] =>
      match s.getLocal x with
      | some _ =>
        interpInstrs P fuel rest
          (s.writeLocal dst (.ref ⟨.loc s.current x, []⟩))
      | none => throw (.stuck "borrow of an uninitialized local")
    | .call [dst] (.borrowField i) [t] =>
      match s.getLocal t with
      | some (.ref rt) =>
        match readTargetI s rt with
        | some (.struct fs) =>
          if i < fs.length then
            interpInstrs P fuel rest
              (s.writeLocal dst (.ref ⟨rt.root, rt.path ++ [i]⟩))
          else throw (.stuck "field borrow out of range")
        | some _ => throw (.stuck "field borrow of a non-struct")
        | none => throw (.stuck "field borrow of a dangling reference")
      | _ => throw (.stuck "borrow_field of a non-reference")
    | .call [dst] (.borrowFieldInst i _) [t] =>
      match s.getLocal t with
      | some (.ref rt) =>
        match readTargetI s rt with
        | some (.struct fs) =>
          if i < fs.length then
            interpInstrs P fuel rest
              (s.writeLocal dst (.ref ⟨rt.root, rt.path ++ [i]⟩))
          else throw (.stuck "field borrow out of range")
        | some _ => throw (.stuck "field borrow of a non-struct")
        | none => throw (.stuck "field borrow of a dangling reference")
      | _ => throw (.stuck "borrow_field of a non-reference")
    | .call [dst] (.borrowGlobal r) [t] =>
      match s.getLocal t with
      | some (.address a) =>
        match s.memory.get r a with
        | some _ =>
          interpInstrs P fuel rest
            (s.writeLocal dst (.ref ⟨.global r a, []⟩))
        | none => pure (.abort s.memory runtimeAbortCode)
      | _ => throw (.stuck "borrow_global of a non-address")
    | .call [dst] (.borrowGlobalInst r args) [t] =>
      match s.getLocal t with
      | some (.address a) =>
        match s.memory.get (resourceKey r args) a with
        | some _ =>
          interpInstrs P fuel rest
            (s.writeLocal dst (.ref ⟨.global (resourceKey r args) a, []⟩))
        | none => pure (.abort s.memory runtimeAbortCode)
      | _ => throw (.stuck "borrow_global of a non-address")
    | .call [dst] .borrowVecElem [t, it] =>
      match s.getLocal t, s.getLocal it with
      | some (.ref rt), some (.u64 n) =>
        match readTargetI s rt with
        | some (.vector es) =>
          if n < es.length then
            interpInstrs P fuel rest
              (s.writeLocal dst (.ref ⟨rt.root, rt.path ++ [n]⟩))
          else pure (.abort s.memory Oper.borrowVecElem.abortCode)
        | some _ => throw (.stuck "vector element borrow of a non-vector")
        | none => throw (.stuck "read through a dangling reference")
      | _, _ =>
        throw (.stuck "vector element borrow of a non-reference or bad index")
    | .call [dst] .readRef [t] =>
      match s.getLocal t with
      | some (.ref rt) =>
        match readTargetI s rt with
        | some v =>
          if !v.refFree then
            throw (.stuck "reference read through a reference")
          else interpInstrs P fuel rest (s.writeLocal dst v)
        | none => throw (.stuck "read through a dangling reference")
      | _ => throw (.stuck "read_ref of a non-reference")
    | .call [] .writeRef [t, vt] =>
      match s.getLocal t, s.getLocal vt with
      | some (.ref rt), some v =>
        if !v.refFree then
          throw (.stuck "reference stored through a reference")
        else
        match writeTargetI s rt v with
        | some s => interpInstrs P fuel rest s
        | none => throw (.stuck "write through a dangling reference")
      | _, _ => throw (.stuck "write_ref of a non-reference")
    | .call [dst] (.isParent pat) [p, t] =>
      match s.getLocal p, s.getLocal t with
      | none, some (.mut _ _) =>
          interpInstrs P fuel rest (s.writeLocal dst (.bool false))
      | _, _ => interpGeneric P fuel rest s [dst] (.isParent pat) [p, t]
    | .call [dst] .freezeRef [t] =>
      match s.getLocal t with
      | some (.ref rt) =>
        match readTargetI s rt with
        | some v =>
          if !v.refFree then
            throw (.stuck "reference frozen through a reference")
          else interpInstrs P fuel rest (s.writeLocal dst (.ref rt))
        | none => throw (.stuck "freeze of a dangling reference")
      | _ => throw (.stuck "freeze_ref of a non-reference")
    | .call dsts (.function f) srcs =>
      match srcs.mapM s.getLocal with
      | none => throw (.stuck "uninitialized call argument")
      | some args =>
        match P.funs f with
        | none => throw (.stuck s!"undeclared function {f}")
        | some d =>
          if d.native then
            throw (.stuck s!"native function {f} has no registered interpreter implementation")
          else if args.length ≠ d.numParams then
            throw (.stuck "arity mismatch in call arguments")
          else
          match ← interpBlock P d.body fuel d.body.entry (s.enterCall args) with
          | .abort m code => pure (.abort m code)
          | .ret world rets =>
            if dsts.length = rets.length then
              interpInstrs P fuel rest
                ((world.resume s.current).writeLocals dsts rets)
            else throw (.stuck "arity mismatch in call results")
    | .call dsts (.functionInst f typeArgs) srcs =>
      match srcs.mapM s.getLocal with
      | none => throw (.stuck "uninitialized call argument")
      | some args =>
        match P.funs f with
        | none => throw (.stuck s!"undeclared function {f}")
        | some d =>
          if d.native then
            throw (.stuck s!"native function {f} has no registered interpreter implementation")
          else if typeArgs.length ≠ d.typeParams.length then
            throw (.stuck "arity mismatch in call type arguments")
          else if args.length ≠ d.numParams then
            throw (.stuck "arity mismatch in call arguments")
          else
          let body := d.body.instantiate typeArgs
          match ← interpBlock P body fuel body.entry (s.enterCall args) with
          | .abort m code => pure (.abort m code)
          | .ret world rets =>
            if dsts.length = rets.length then
              interpInstrs P fuel rest
                ((world.resume s.current).writeLocals dsts rets)
            else throw (.stuck "arity mismatch in call results")
    | .call dsts op srcs =>
      interpGeneric P fuel rest s dsts op srcs

/-- Shared interpreter path for primitive (non-call, non-reference)
operations. -/
def interpGeneric (P : Program) (fuel : Nat) (rest : List Instr)
    (s : IState) (dsts : List LocalIndex) (op : Oper)
    (srcs : List LocalIndex) : Except InterpError IInstrsRes := do
  match srcs.mapM s.getLocal with
  | none => throw (.stuck "uninitialized operand")
  | some vs =>
    match ← interpOp s.current (readTargetI s) op vs s.memory with
    | .abort => pure (.abort s.memory op.abortCode)
    | .ok rets m' =>
      if dsts.length = rets.length then
        interpInstrs P fuel rest ((s.setMemory m').writeLocals dsts rets)
      else throw (.stuck "arity mismatch in operation results")

end

end MoveModel.IR
