-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Value
import MoveModel.IR.State
import MoveModel.IR.ValueTyping
import MoveModel.IR.Spec
import MoveModel.IR.Contract

/-!
# IR Syntax: Stackless Bytecode as a CFG of Basic Blocks

The translation consumes a monomorphic fragment of Move stackless bytecode
(`stackless_bytecode.rs`).  Programs are control-flow graphs of basic blocks,
matching the `StacklessControlFlowGraph` view:

* instructions are **three-address**: all operands are locals
  (`LocalIndex`), there are no nested expressions in code;
* a basic block is a list of straight-line instructions plus a terminator
  (`jump`/`branch`/`ret`/`abort`);
* `Instr.call dsts op srcs` mirrors `Bytecode::Call(dsts, Operation, srcs)`,
  with `Oper` the supported `Operation` fragment.

Reference elimination introduces `Oper.writeGlobal`, `Oper.updateField`, and
`Oper.vecSet` (TACAS'22 §3.1).  A write through `&mut global<R>(a)` becomes a
direct store.  Writes through field and vector-element borrows become
functional updates.

A `FunDecl` maps each loop header to a `LoopSpec`.  The specification contains
the user invariant, the member blocks, and the loop targets that an iteration
may modify.

This metadata captures the output of the prover's `fat_loop` recognition and
target analysis.  After `LoopAnalysisProcessor`, the same information appears
as `Prop` and havoc instructions at the header.
-/

namespace MoveModel.IR

/-- Function identifier. -/
abbrev FunId := Nat

/-- Basic-block identifier. -/
abbrev BlockId := Nat

/-- The supported fragment of stackless-bytecode `Operation`s.  Arithmetic
aborts on overflow/underflow and division by zero; `getGlobal`/`moveFrom`
abort if the resource is absent, `moveTo` if it is present.  `unpack` yields
all fields of a struct value.  `function f` is a call to a declared function.

The mutation operations (`mkMutLoc` … `mutAddr`) are the *mutation
algebra* of the full reference elimination — TACAS'22 §3.1's `Mut<T>`
(`Mvp::mklocal/mkglobal/field/get/set/is_*`) resp. the Boogie prelude's
`$Mutation`/`$ChildMutation`/`$Dereference`/`$UpdateMutation`/
`$IsParentMutation`: they create a mutation for a root location (checking
out the current value; `mkMutGlobal` aborts like `borrow_global` if the
resource is absent), derive sub-mutations by static field offset or
dynamic vector index (aborting out of range), read and replace the carried
value, and support the write-back dispatch — `isParent` tests location
derivation against a static edge pattern (`none` = index wildcard, MVP's
`-1`), `mutPathIndex` recovers a dynamic index from the child's path, and
`isMutLoc`/`isMutGlobal`/`mutAddr` identify root locations.  Frontends
never produce them.

The vector operations are the value-level counterparts of the `vector`
natives (in stackless bytecode those are native calls; here they are
operations, with the borrow-based natives expressed through references):
`vecPack` builds a vector from its operands (`vector::empty` + literals),
`vecLen`/`vecGet`/`vecSet` read the length, an element, and functionally
replace an element (aborting out of range, like `vector::borrow`),
`vecPush`/`vecPop` append and remove at the back, while
`vecInsert`/`vecRemove` shift a suffix at a dynamic index. `vecPop` aborts on
the empty vector, `vecInsert` when the index is beyond the end, and
`vecRemove` when it is at or beyond the end. Growth also aborts when the
length would exceed the `u64` range.
`vecGet`/`vecSet` are also the reference-elimination residues of
element borrows, alongside `updateField`/`writeGlobal`.

The reference operations (`borrowLoc` … `freezeRef`, mirroring
`Operation::BorrowLoc/BorrowField/BorrowGlobal/ReadRef/WriteRef/FreezeRef`,
plus `borrowVecElem` for `vector::borrow(_mut)` with its dynamic element
index) **execute** (references are runtime values, `RefTarget`) but do
**not verify**: they compile to a failing assertion.  As in the real
prover, verification requires the bytecode-level *reference elimination*
(`RefElim.lean`), which rewrites them into the value-level operations. -/
inductive Oper where
  -- one integer-arithmetic family: each operation carries the operand's
  -- `NumType` (width + signedness); signed and unsigned differ only in the
  -- range checked/wrapped against.  Comparisons (`lt`/`le`/`eq`) are shared —
  -- a value carries its mathematical magnitude, so ordering is correct for
  -- both signs.
  | add (nt : NumType) | sub (nt : NumType) | mul (nt : NumType)
  | div (nt : NumType) | mod (nt : NumType)
  | bitAnd (nt : NumType) | bitOr (nt : NumType) | bitXor (nt : NumType)
  | shl (nt : NumType) | shr (nt : NumType)
  | cast (target : NumType)
  | lt | le | eq | and | or | not
  | pack
  | packInst (args : List Ty)
  | unpack
  | unpackInst (args : List Ty)
  | packVariant (variant : Nat)
  | packVariantInst (variant : Nat) (args : List Ty)
  | unpackVariant (variant : Nat)
  | unpackVariantInst (variant : Nat) (args : List Ty)
  | testVariant (variant : Nat)
  | testVariantInst (variant : Nat) (args : List Ty)
  | getField (i : Nat)
  | getFieldInst (i : Nat) (args : List Ty)
  | updateField (i : Nat)
  | vecPack
  | vecLen
  | vecGet
  | vecSet
  | vecPush
  | vecPop
  | vecInsert
  | vecRemove
  | vecSwap
  | vecSwapRemove
  | vecAppend
  | vecReverse
  | vecReverseSlice
  | vecContains
  | vecIndexOf
  | vecTrim
  | vecTrimReverse
  | vecRotate
  | vecRotateSlice
  | vecDestroyEmpty
  -- the mutation algebra (`Mvp::*` of TACAS'22 §3.1, `$Mutation` of the
  -- Boogie prelude): residues of the full reference elimination, never
  -- produced by frontends
  | mkMutLoc (x : LocalIndex)
  | mkMutGlobal (r : ResourceId)
  | childMutField (i : Nat)
  | childMutIndex
  | getMut
  | setMut
  | isParent (pat : List (Option Nat))
  | mutPathIndex (k : Nat)
  | isMutLoc (x : LocalIndex)
  | isMutGlobal (r : ResourceId)
  | mutAddr
  | getGlobal (r : ResourceId)
  | getGlobalInst (r : ResourceId) (args : List Ty)
  | writeGlobal (r : ResourceId)
  | moveTo (r : ResourceId)
  | moveToInst (r : ResourceId) (args : List Ty)
  | moveFrom (r : ResourceId)
  | moveFromInst (r : ResourceId) (args : List Ty)
  | exists_ (r : ResourceId)
  | existsInst (r : ResourceId) (args : List Ty)
  | function (f : FunId)
  | functionInst (f : FunId) (args : List Ty)
  | borrowLoc
  | borrowField (i : Nat)
  | borrowFieldInst (i : Nat) (args : List Ty)
  | borrowGlobal (r : ResourceId)
  | borrowGlobalInst (r : ResourceId) (args : List Ty)
  | borrowVecElem
  | readRef
  | writeRef
  | freezeRef
  deriving BEq, Repr

/-- Straight-line instructions (`Bytecode::Load/Assign/Call/Nop`). -/
inductive Instr where
  | load (dst : LocalIndex) (v : Value)
  | assign (dst src : LocalIndex)
  | call (dsts : List LocalIndex) (op : Oper) (srcs : List LocalIndex)
  | nop
  deriving BEq, Repr

/-- Substitute the enclosing function's type parameters in every explicit
operation instantiation.  Runtime values are type-erased, so these are the
only instruction fields which need substitution when entering a generic
function at concrete type arguments. -/
def Oper.instantiate (typeArgs : List Ty) : Oper → Oper
  | .packInst args => .packInst (instantiateTypes typeArgs args)
  | .unpackInst args => .unpackInst (instantiateTypes typeArgs args)
  | .packVariantInst tag args =>
      .packVariantInst tag (instantiateTypes typeArgs args)
  | .unpackVariantInst tag args =>
      .unpackVariantInst tag (instantiateTypes typeArgs args)
  | .testVariantInst tag args =>
      .testVariantInst tag (instantiateTypes typeArgs args)
  | .getFieldInst field args =>
      .getFieldInst field (instantiateTypes typeArgs args)
  | .getGlobalInst resource args =>
      .getGlobalInst resource (instantiateTypes typeArgs args)
  | .moveToInst resource args =>
      .moveToInst resource (instantiateTypes typeArgs args)
  | .moveFromInst resource args =>
      .moveFromInst resource (instantiateTypes typeArgs args)
  | .existsInst resource args =>
      .existsInst resource (instantiateTypes typeArgs args)
  | .functionInst funId args =>
      .functionInst funId (instantiateTypes typeArgs args)
  | .borrowFieldInst field args =>
      .borrowFieldInst field (instantiateTypes typeArgs args)
  | .borrowGlobalInst resource args =>
      .borrowGlobalInst resource (instantiateTypes typeArgs args)
  | op => op

/-- Instantiate all explicit type arguments in an instruction. -/
def Instr.instantiate (typeArgs : List Ty) : Instr → Instr
  | .call dsts op srcs => .call dsts (op.instantiate typeArgs) srcs
  | instr => instr

/-- The locals read by an instruction. -/
def instrUses : Instr → List LocalIndex
  | .load _ _ => []
  | .assign _ src => [src]
  | .call _ _ srcs => srcs
  | .nop => []

/-- The locals defined by an instruction. -/
def instrDefs : Instr → List LocalIndex
  | .load dst _ => [dst]
  | .assign dst _ => [dst]
  | .call dsts _ _ => dsts
  | .nop => []

/-- Block terminators.  `branch` reads a boolean local and never falls
through (`Bytecode::Branch`); `ret` returns the values of `srcs`; `abort`
aborts with the `u64` code held in `code`. -/
inductive Term where
  | jump (b : BlockId)
  | branch (cond : LocalIndex) (thenB elseB : BlockId)
  | ret (srcs : List LocalIndex)
  | abort (code : LocalIndex)
  deriving BEq, Repr

/-- The locals read by a terminator. -/
def termReads : Term → List LocalIndex
  | .ret srcs => srcs
  | .abort code => [code]
  | .branch cond _ _ => [cond]
  | .jump _ => []

/-- The successor blocks of a terminator. -/
def termSuccs : Term → List BlockId
  | .jump b => [b]
  | .branch _ thenB elseB => [thenB, elseB]
  | .ret _ => []
  | .abort _ => []

/-- A basic block: straight-line instructions plus a terminator. -/
structure Block where
  instrs : List Instr
  term : Term
  deriving BEq, Repr

/-- Instantiate all explicit type arguments in a basic block. -/
def Block.instantiate (typeArgs : List Ty) (block : Block) : Block :=
  { block with instrs := block.instrs.map (Instr.instantiate typeArgs) }

/-- A control-flow graph of basic blocks.  `size` bounds the block ids in
use (`blocks b = none` is expected for `b ≥ size`); block ids follow code
layout order, so — as in stackless bytecode, where a back edge is a jump to
a lower code offset — back edges target lower-numbered blocks. -/
structure Cfg where
  blocks : BlockId → Option Block
  entry : BlockId
  size : Nat

/-- Instantiate a generic function CFG at one list of call-site types. -/
def Cfg.instantiate (typeArgs : List Ty) (cfg : Cfg) : Cfg :=
  { cfg with blocks := fun block =>
      (cfg.blocks block).map (Block.instantiate typeArgs) }

/-- Loop metadata attached to a loop-header block: the invariant (a spec
expression over the current locals and memory) and the declared loop
targets — the locals (`valTargets`) and global locations (`memTargets`)
an iteration may modify — plus the blocks belonging to the loop. -/
structure LoopSpec where
  inv : SpecExp
  valTargets : LocalIndex → Prop
  memTargets : Footprint
  members : BlockId → Prop

/-- Specialize the type-bearing portion of loop metadata. -/
def LoopSpec.instantiate (args : List Ty) (spec : LoopSpec) : LoopSpec where
  inv := spec.inv.instantiate args
  valTargets := spec.valTargets
  memTargets := spec.memTargets
  members := spec.members

/-- A function declaration.  Parameters are the locals
`0 .. numParams-1`; results are delivered by the `ret` terminator.
`locals` declares the type of every local (parameters first),
`numLocals` their count (`locals t = none` is expected for
`t ≥ numLocals`), `returns` the result types in `ret`-operand order. -/
structure FunDecl where
  typeParams : List TypeParamDecl := []
  numParams : Nat
  numLocals : Nat
  locals : LocalIndex → Option Ty
  returns : List Ty
  body : Cfg
  loopSpecs : BlockId → Option LoopSpec
  contract : Contract
  /-- Bodyless declaration dispatched by the hosting VM. -/
  native : Bool := false

/-- Construct an executable declaration from the finite lists produced by a
compiler. The semantic IR deliberately uses partial maps; this constructor
keeps that representation at the boundary instead of introducing an exchange
format merely to quote a compiled declaration into Lean. -/
def FunDecl.ofLists (typeParams : List TypeParamDecl) (numParams : Nat)
    (locals : List Ty) (returns : List Ty) (blocks : List Block)
    (entry : BlockId) (contract : Contract) (native : Bool := false) : FunDecl where
  typeParams := typeParams
  numParams := numParams
  numLocals := locals.length
  locals := fun i => locals[i]?
  returns := returns
  body := { blocks := fun b => blocks[b]?, entry := entry, size := blocks.length }
  -- Source lowering currently has no proof-loop metadata. That metadata is
  -- introduced by the prover pipeline, whose input is already semantic IR.
  loopSpecs := fun _ => none
  contract := contract
  native := native

/-- Materialize a declaration's finite local map in index order. -/
def FunDecl.localsList (d : FunDecl) : List Ty :=
  (List.range d.numLocals).filterMap d.locals

/-- Materialize a declaration's finite CFG in block-index order. -/
def FunDecl.blocksList (d : FunDecl) : List Block :=
  (List.range d.body.size).filterMap d.body.blocks

/-- The monomorphic proof/execution view of a generic declaration at one type
instantiation.  Numeric layout and parameter counts are unchanged. -/
def FunDecl.instantiate (args : List Ty) (d : FunDecl) : FunDecl where
  typeParams := []
  numParams := d.numParams
  numLocals := d.numLocals
  locals := fun i => (d.locals i).map (Ty.instantiate args)
  returns := instantiateTypes args d.returns
  body := d.body.instantiate args
  loopSpecs := fun block => (d.loopSpecs block).map (LoopSpec.instantiate args)
  contract := d.contract.instantiate args
  native := d.native

/-- A program: partial maps from function ids to function declarations and
from resource ids to struct declarations. -/
structure Program where
  funs : FunId → Option FunDecl
  structs : StructDecls

/-! ## Typed boundary states -/

/-- Well-typed arguments of a function activation: the arity matches the
parameter count, and every argument is well-formed for the declared type of
its parameter local. -/
def TypedArgs (Δ : StructDecls) (d : FunDecl) (args : List Value) : Prop :=
  args.length = d.numParams ∧
  ∀ i t v, d.locals i = some t → args[i]? = some v → IsValid Δ t v

/-- Reference-free typed arguments cannot inhabit a mutable-reference
parameter.  This is the checked external-boundary form of the bytecode rule
that mutable references are runtime-only values. -/
theorem TypedArgs.not_mutRefParameter {Δ : StructDecls} {d : FunDecl}
    {args : List Value} (htyped : TypedArgs Δ d args)
    (hfree : ∀ v ∈ args, v.refFree) {i : LocalIndex} {t : Ty}
    (hi : i < d.numParams) : d.locals i ≠ some (.mutRef t) := by
  intro hdecl
  have hi' : i < args.length := by simpa [htyped.1] using hi
  have hsome : args[i]? ≠ none := by
    simpa [List.getElem?_eq_none] using hi'
  cases harg : args[i]? with
  | none => exact hsome harg
  | some v =>
      have hvalid := htyped.2 i (.mutRef t) v hdecl harg
      have hvfree := hfree v (List.mem_of_getElem? harg)
      rw [hvalid.mutRef_not_refFree] at hvfree
      contradiction

/-- Well-typed global memory: every stored resource whose type is declared
is well-formed for its declaration — the memory-level `WellFormed`
assumption of the real prover. -/
def TypedMemory (Δ : StructDecls) (m : Memory) : Prop :=
  ∀ r sd a v, Δ r = some sd → m r a = some v → IsValid Δ (.struct r) v

end MoveModel.IR
