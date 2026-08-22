-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Value
import MoveModel.IR.State
import MoveModel.IR.Spec
import MoveModel.IR.Contract
import MoveModel.IR.Syntax
import MoveModel.IR.ValueTyping

set_option maxHeartbeats 0

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
  division by zero, or a resource error.  `Oper.abortCode` supplies its code;
  `Term.abort` uses the value of its operand.
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

namespace MoveModel.IR

/-- Default code used for runtime execution failures. -/
def runtimeAbortCode : Nat := 0

/-- Observable abort code of a failed primitive operation. Stable vector
insertion and removal mirror `std::vector::EINDEX_OUT_OF_BOUNDS`; the remaining
runtime failures retain the model's generic execution-failure code. -/
def Oper.abortCode : Oper → Nat
  | .vecPop => 0x20000
  | .vecGet => 0x20000
  | .vecSet => 0x20000
  | .borrowVecElem => 0x20000
  | .vecInsert => 0x20000
  | .vecRemove => 0x20000
  | .vecSwap => 0x20000
  | .vecSwapRemove => 0x20000
  | .vecAppend => 0x20000
  | .vecTrim => 0x20000
  | .vecTrimReverse => 0x20000
  | .vecDestroyEmpty => 0x20000
  | .vecReverseSlice => 0x20001
  | .vecRotate => 0x20001
  | .vecRotateSlice => 0x20001
  | _ => runtimeAbortCode

/-- Result of a non-call operation: result values plus the updated memory,
or a runtime abort. -/
inductive OpOutcome where
  | ok (rets : List Value) (m : Memory)
  | abort

/-- Semantics of the extended `std::vector` surface. Keeping these derived
operations behind one fallback keeps the core operation matcher small. -/
def Oper.extendedVectorSem : Oper → List Value → Memory → Option OpOutcome
  | .vecSwap, [.vector es, .u64 i, .u64 j], m =>
      some (match es[i]?, es[j]? with
        | some vi, some vj => .ok [.vector ((es.set i vj).set j vi)] m
        | _, _ => .abort)
  | .vecSwapRemove, [.vector es, .u64 i], m =>
      some (match es[i]?, es.getLast? with
        | some removed, some last =>
            .ok [.vector (es.set i last).dropLast, removed] m
        | _, _ => .abort)
  | .vecAppend, [.vector lhs, .vector rhs], m =>
      some (if lhs.length + rhs.length < U64_SIZE then
        .ok [.vector (lhs ++ rhs)] m else .abort)
  | .vecReverse, [.vector es], m => some (.ok [.vector es.reverse] m)
  | .vecReverseSlice, [.vector es, .u64 left, .u64 right], m =>
      some (if left ≤ right && right ≤ es.length then
        .ok [.vector (es.take left ++ ((es.drop left).take (right - left)).reverse ++
          es.drop right)] m else .abort)
  | .vecContains, [.vector es, v], m =>
      some (.ok [.bool (es.any (· == v))] m)
  | .vecIndexOf, [.vector es, v], m =>
      let i := es.findIdx (· == v)
      some (.ok [.bool (i < es.length), .u64 (if i < es.length then i else 0)] m)
  | .vecTrim, [.vector es, .u64 newLen], m =>
      some (if newLen ≤ es.length then
        .ok [.vector (es.take newLen), .vector (es.drop newLen)] m else .abort)
  | .vecTrimReverse, [.vector es, .u64 newLen], m =>
      some (if newLen ≤ es.length then
        .ok [.vector (es.take newLen), .vector (es.drop newLen).reverse] m else .abort)
  | .vecRotate, [.vector es, .u64 rot], m =>
      some (if rot ≤ es.length then
        .ok [.vector (es.drop rot ++ es.take rot), .u64 (es.length - rot)] m
      else .abort)
  | .vecRotateSlice, [.vector es, .u64 left, .u64 rot, .u64 right], m =>
      some (if left ≤ rot && rot ≤ right && right ≤ es.length then
        .ok [.vector (es.take left ++ (es.drop rot).take (right - rot) ++
          (es.drop left).take (rot - left) ++ es.drop right),
          .u64 (left + (right - rot))] m else .abort)
  | .vecDestroyEmpty, [.vector es], m =>
      some (if es.isEmpty then .ok [] m else .abort)
  | _, _, _ => none

/-- Checked integer outcome: accept the mathematical result `r` iff it lies in
the type's range `[lo, hi)`, else abort.  One rule for both signednesses; the
bounds come from `nt`. -/
@[simp] def NumType.checked (nt : NumType) (r : Int) (m : Memory) : OpOutcome :=
  if nt.lo ≤ r ∧ r < nt.hi then .ok [.int r] m else .abort

/-- Bitwise outcome: apply `f` to the operands' two's-complement bit patterns
and reinterpret the result in range.  For unsigned types `toBits`/`fromBits`
are the identity, recovering the plain `Nat` bit operation. -/
@[simp] def NumType.bitwise (nt : NumType) (f : Nat → Nat → Nat) (i j : Int)
    (m : Memory) : OpOutcome :=
  .ok [.int (nt.fromBits (f (nt.toBits i).toNat (nt.toBits j).toNat))] m

/-- Semantics of the non-call operations, as a deterministic partial
function of the operand values and the current memory.  `none` = ill-typed
(stuck); `some .abort` = runtime abort.  `function` calls and the
reference operations are handled relationally by `RunFrom` (they need the
locals resp. the operand *indices*, not just values).

`deref` resolves reference values for observations: Move equality, structural
ordering, and vector length operate on the values references denote. Values
stored to global memory or packed into structs and vectors must be
reference-free (`Value.refFree`), as in the Move VM. -/
def Oper.sem (current : FrameId) (deref : RefTarget → Option Value) :
    Oper → List Value → Memory → Option OpOutcome
  -- checked integer arithmetic: compute over `Int`, then confine to `nt`'s
  -- range.  Division truncates toward zero (`tdiv`/`tmod`); `mod` never leaves
  -- the range.  Bitwise/shift act on the two's-complement pattern; `shr` is
  -- arithmetic (floor division by `2^k`).  Signed and unsigned share every
  -- rule — only `nt.lo`/`nt.hi` differ.
  | .add nt, [.int i, .int j], m => some (nt.checked (i + j) m)
  | .sub nt, [.int i, .int j], m => some (nt.checked (i - j) m)
  | .mul nt, [.int i, .int j], m => some (nt.checked (i * j) m)
  | .div nt, [.int i, .int j], m =>
      some (if j = 0 then .abort else nt.checked (i.tdiv j) m)
  | .mod _, [.int i, .int j], m =>
      some (if j = 0 then .abort else .ok [.int (i.tmod j)] m)
  | .bitAnd nt, [.int i, .int j], m => some (nt.bitwise (· &&& ·) i j m)
  | .bitOr nt, [.int i, .int j], m => some (nt.bitwise (· ||| ·) i j m)
  | .bitXor nt, [.int i, .int j], m => some (nt.bitwise (· ^^^ ·) i j m)
  | .shl nt, [.int i, .int k], m =>
      some (if k < (nt.width.bits : Int) then
        .ok [.int (nt.fromBits (((nt.toBits i).toNat <<< k.toNat) % nt.size : Nat))] m
      else .abort)
  | .shr nt, [.int i, .int k], m =>
      some (if k < (nt.width.bits : Int) then .ok [.int (i.fdiv (2 ^ k.toNat))] m
        else .abort)
  | .cast target, [.int i], m => some (target.checked i m)
  | .lt, [v₁, v₂], m => do
      let a ← v₁.derefWith deref
      let b ← v₂.derefWith deref
      match a, b with
      | .int i, .int j => pure (.ok [.bool (decide (i < j))] m)
      | a, b =>
          if a.refFree && b.refFree && a.sameTypeShape b then
            pure (.ok [.bool (compare a b == .lt)] m)
          else none
  | .le, [.int i, .int j], m => some (.ok [.bool (decide (i ≤ j))] m)
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
  | .packInst _, vs, m =>
      if Value.refFreeList vs then some (.ok [.struct vs] m) else none
  | .unpack, [.struct fs], m => some (.ok fs m)
  | .unpackInst _, [.struct fs], m => some (.ok fs m)
  | .packVariant tag, vs, m =>
      if Value.refFreeList vs then some (.ok [.variant tag vs] m) else none
  | .packVariantInst tag _, vs, m =>
      if Value.refFreeList vs then some (.ok [.variant tag vs] m) else none
  | .unpackVariant tag, [.variant actual fs], m =>
      if actual = tag then some (.ok fs m) else some .abort
  | .unpackVariantInst tag _, [.variant actual fs], m =>
      if actual = tag then some (.ok fs m) else some .abort
  | .testVariant tag, [.variant actual _], m =>
      some (.ok [.bool (actual = tag)] m)
  | .testVariantInst tag _, [.variant actual _], m =>
      some (.ok [.bool (actual = tag)] m)
  | .getField i, [.struct fs], m => (fs[i]?).map (fun v => .ok [v] m)
  | .getFieldInst i _, [.struct fs], m => (fs[i]?).map (fun v => .ok [v] m)
  | .updateField i, [.struct fs, v], m =>
      if !v.refFree then none
      else if i < fs.length then some (.ok [.struct (fs.set i v)] m)
      else none
  | .vecPack, vs, m =>
      if Value.refFreeList vs then some (.ok [.vector vs] m) else none
  | .vecLen, [value], m => do
      let .vector es ← value.derefWith deref | none
      pure (.ok [.u64 es.length] m)
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
  | .vecInsert, [.vector es, .u64 i, v], m =>
      if !v.refFree then none
      else some (if i ≤ es.length && es.length + 1 < U64_SIZE then
        .ok [.vector (es.take i ++ v :: es.drop i)] m
      else .abort)
  | .vecRemove, [.vector es, .u64 i], m =>
      some (match es[i]? with
        | some v => .ok [.vector (es.take i ++ es.drop (i + 1)), v] m
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
  | .getGlobalInst r args, [.address a], m =>
      some (match m (resourceKey r args) a with
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
  | .moveToInst r args, [.address a, v], m =>
      if !v.refFree then none
      else
        some (match m (resourceKey r args) a with
          | some _ => .abort
          | none => .ok [] (memWrite m (resourceKey r args) a v))
  | .moveFrom r, [.address a], m =>
      some (match m r a with
        | some v => .ok [v] (memRemove m r a)
        | none => .abort)
  | .moveFromInst r args, [.address a], m =>
      some (match m (resourceKey r args) a with
        | some v => .ok [v] (memRemove m (resourceKey r args) a)
        | none => .abort)
  | .exists_ r, [.address a], m => some (.ok [.bool (m r a).isSome] m)
  | .existsInst r args, [.address a], m =>
      some (.ok [.bool (m (resourceKey r args) a).isSome] m)
  | op, vs, m =>
      match op with
      | .vecSwap | .vecSwapRemove | .vecAppend | .vecReverse | .vecReverseSlice
      | .vecContains | .vecIndexOf | .vecTrim | .vecTrimReverse | .vecRotate
      | .vecRotateSlice | .vecDestroyEmpty => op.extendedVectorSem vs m
      | _ => none

@[simp] theorem Oper.sem_vecSwap (current) (deref) (es) (i j) (m) :
    Oper.sem current deref .vecSwap [.vector es, .u64 i, .u64 j] m =
      some (match es[i]?, es[j]? with
        | some vi, some vj => .ok [.vector ((es.set i vj).set j vi)] m
        | _, _ => .abort) := rfl

@[simp] theorem Oper.sem_vecSwapRemove (current) (deref) (es) (i) (m) :
    Oper.sem current deref .vecSwapRemove [.vector es, .u64 i] m =
      some (match es[i]?, es.getLast? with
        | some removed, some last => .ok [.vector (es.set i last).dropLast, removed] m
        | _, _ => .abort) := rfl

@[simp] theorem Oper.sem_vecAppend (current) (deref) (lhs rhs) (m) :
    Oper.sem current deref .vecAppend [.vector lhs, .vector rhs] m =
      some (if lhs.length + rhs.length < U64_SIZE then
        .ok [.vector (lhs ++ rhs)] m else .abort) := rfl

@[simp] theorem Oper.sem_vecReverse (current) (deref) (es) (m) :
    Oper.sem current deref .vecReverse [.vector es] m =
      some (.ok [.vector es.reverse] m) := rfl

@[simp] theorem Oper.sem_vecReverseSlice (current) (deref) (es) (left right) (m) :
    Oper.sem current deref .vecReverseSlice
        [.vector es, .u64 left, .u64 right] m =
      some (if left ≤ right && right ≤ es.length then
        .ok [.vector (es.take left ++ ((es.drop left).take (right - left)).reverse ++
          es.drop right)] m else .abort) := rfl

@[simp] theorem Oper.sem_vecContains (current) (deref) (es) (v) (m) :
    Oper.sem current deref .vecContains [.vector es, v] m =
      some (.ok [.bool (es.any (· == v))] m) := rfl

@[simp] theorem Oper.sem_vecIndexOf (current) (deref) (es) (v) (m) :
    Oper.sem current deref .vecIndexOf [.vector es, v] m =
      let i := es.findIdx (· == v)
      some (.ok [.bool (i < es.length), .u64 (if i < es.length then i else 0)] m) := rfl

@[simp] theorem Oper.sem_vecTrim (current) (deref) (es) (newLen) (m) :
    Oper.sem current deref .vecTrim [.vector es, .u64 newLen] m =
      some (if newLen ≤ es.length then
        .ok [.vector (es.take newLen), .vector (es.drop newLen)] m else .abort) := rfl

@[simp] theorem Oper.sem_vecTrimReverse (current) (deref) (es) (newLen) (m) :
    Oper.sem current deref .vecTrimReverse [.vector es, .u64 newLen] m =
      some (if newLen ≤ es.length then
        .ok [.vector (es.take newLen), .vector (es.drop newLen).reverse] m
      else .abort) := rfl

@[simp] theorem Oper.sem_vecRotate (current) (deref) (es) (rot) (m) :
    Oper.sem current deref .vecRotate [.vector es, .u64 rot] m =
      some (if rot ≤ es.length then
        .ok [.vector (es.drop rot ++ es.take rot), .u64 (es.length - rot)] m
      else .abort) := rfl

@[simp] theorem Oper.sem_vecRotateSlice (current) (deref) (es) (left rot right) (m) :
    Oper.sem current deref .vecRotateSlice
        [.vector es, .u64 left, .u64 rot, .u64 right] m =
      some (if left ≤ rot && rot ≤ right && right ≤ es.length then
        .ok [.vector (es.take left ++ (es.drop rot).take (right - rot) ++
          (es.drop left).take (rot - left) ++ es.drop right),
          .u64 (left + (right - rot))] m else .abort) := rfl

@[simp] theorem Oper.sem_vecDestroyEmpty (current) (deref) (es) (m) :
    Oper.sem current deref .vecDestroyEmpty [.vector es] m =
      some (if es.isEmpty then .ok [] m else .abort) := rfl

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
        (.abort s.memory op.abortCode)
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
  | borrowFieldInst {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dst t : LocalIndex} {i : Nat} {args : List Ty} {rt : RefTarget}
      {fs : List Value} {o : FrameOutcome}
      (ht : s.locals t = some (.ref rt))
      (hs : s.readTarget rt = some (.struct fs))
      (hi : i < fs.length)
      (hrest : RunFrom P G rest term
        (s.writeLocal dst (.ref ⟨rt.root, rt.path ++ [i]⟩)) o) :
      RunFrom P G (.call [dst] (.borrowFieldInst i args) [t] :: rest) term s o
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
  | borrowGlobalInstOk {G : Cfg} {rest : List Instr} {term : Term}
      {s : MoveState} {dst t : LocalIndex} {r : ResourceId} {args : List Ty}
      {a : Address} {v : Value} {o : FrameOutcome}
      (ha : s.locals t = some (.address a))
      (hpresent : s.memory (resourceKey r args) a = some v)
      (hrest : RunFrom P G rest term
        (s.writeLocal dst (.ref ⟨.global (resourceKey r args) a, []⟩)) o) :
      RunFrom P G (.call [dst] (.borrowGlobalInst r args) [t] :: rest) term s o
  | borrowGlobalInstAbort {G : Cfg} {rest : List Instr} {term : Term}
      {s : MoveState} {dst t : LocalIndex} {r : ResourceId} {args : List Ty}
      {a : Address}
      (ha : s.locals t = some (.address a))
      (habsent : s.memory (resourceKey r args) a = none) :
      RunFrom P G (.call [dst] (.borrowGlobalInst r args) [t] :: rest) term s
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
  | callInstOk {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dsts srcs : List LocalIndex} {f : FunId} {typeArgs : List Ty}
      {d : FunDecl} {args retVals : List Value} {blk : Block}
      {world : FrameWorld} {o : FrameOutcome}
      (hd : P.funs f = some d)
      (htyargs : typeArgs.length = d.typeParams.length)
      (hargs : srcs.mapM s.locals = some args)
      (hnargs : args.length = d.numParams)
      (hentry : (d.body.instantiate typeArgs).blocks
        (d.body.instantiate typeArgs).entry = some blk)
      (hcallee : RunFrom P (d.body.instantiate typeArgs) blk.instrs blk.term
        (s.enterCall args) (.ret world retVals))
      (hlen : dsts.length = retVals.length)
      (hrest : RunFrom P G rest term
        (MoveState.writeLocals (world.resume s.current) dsts retVals) o) :
      RunFrom P G (.call dsts (.functionInst f typeArgs) srcs :: rest) term s o
  | callInstAbort {G : Cfg} {rest : List Instr} {term : Term} {s : MoveState}
      {dsts srcs : List LocalIndex} {f : FunId} {typeArgs : List Ty}
      {d : FunDecl} {args : List Value} {blk : Block} {m' : Memory} {code : Nat}
      (hd : P.funs f = some d)
      (htyargs : typeArgs.length = d.typeParams.length)
      (hargs : srcs.mapM s.locals = some args)
      (hnargs : args.length = d.numParams)
      (hentry : (d.body.instantiate typeArgs).blocks
        (d.body.instantiate typeArgs).entry = some blk)
      (hcallee : RunFrom P (d.body.instantiate typeArgs) blk.instrs blk.term
        (s.enterCall args) (.abort m' code)) :
      RunFrom P G (.call dsts (.functionInst f typeArgs) srcs :: rest) term s
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

theorem Oper.sem_deref_irrel {op : Oper} {current : FrameId}
    {deref₁ deref₂ : RefTarget → Option Value} {vs : List Value}
    {m : Memory} (hne : op ≠ .eq) (hnlt : op ≠ .lt)
    (hnlen : op ≠ .vecLen) :
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

end MoveModel.IR
