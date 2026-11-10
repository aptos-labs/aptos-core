-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Semantics.Program

/-!
# Local structural properties L1–L4

TRUSTED. Properties of one module's text, established by local verification
alone (plan §7, group L). They mention no execution.

- L1 `IndexIntegrity`: every index into the module's own tables is in range.
- L2 `Uniqueness`: an index names one entity.
- L3 `ControlFlowWellFormed`: no jump leaves the code, no path falls off the
  end, and the control-flow graph is reducible.
- L4 `StackDiscipline`: a unique operand-stack height at every instruction,
  balanced at block boundaries.

L5 (type formation), L6 (struct definitions) and L10 (friend declarations)
are in `Spec.Properties.Declarations`, L7 (local instantiation) in
`Spec.Properties.Instantiation`, L8 (constants) in
`Spec.Properties.Constants`, L9 (instruction consistency) in
`Spec.Properties.Instructions`, L11 (syntactic bounds, loop depth included)
in `Spec.Properties.Limits`.
-/

namespace MoveBytecode

/-! ## References to tables -/

/-- A reference from a type token to a table or type-parameter list. -/
inductive TokenRef where
  | structHandle (i : StructHandleIndex)
  | typeParameter (i : TypeParameterIndex)
  deriving DecidableEq, Repr

mutual
def SignatureToken.refs : SignatureToken → List TokenRef
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 => []
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => []
  | .address | .signer => []
  | .vector e => e.refs
  | .function ps rs _ => SignatureToken.refsList ps ++ SignatureToken.refsList rs
  | .struct h => [.structHandle h]
  | .structInstantiation h args => .structHandle h :: SignatureToken.refsList args
  | .reference t => t.refs
  | .mutableReference t => t.refs
  | .typeParameter i => [.typeParameter i]

def SignatureToken.refsList : List SignatureToken → List TokenRef
  | [] => []
  | t :: ts => t.refs ++ SignatureToken.refsList ts
end

/-- A reference from an instruction to a table, a local, or a code offset. -/
inductive IndexRef where
  | local_ (i : LocalIndex)
  | codeOffset (o : CodeOffset)
  | constant (i : ConstantPoolIndex)
  | functionHandle (i : FunctionHandleIndex)
  | functionInstantiation (i : FunctionInstantiationIndex)
  | structDef (i : StructDefinitionIndex)
  | structDefInstantiation (i : StructDefInstantiationIndex)
  | structVariantHandle (i : StructVariantHandleIndex)
  | structVariantInstantiation (i : StructVariantInstantiationIndex)
  | fieldHandle (i : FieldHandleIndex)
  | fieldInstantiation (i : FieldInstantiationIndex)
  | variantFieldHandle (i : VariantFieldHandleIndex)
  | variantFieldInstantiation (i : VariantFieldInstantiationIndex)
  | signature (i : SignatureIndex)
  deriving DecidableEq, Repr

def Bytecode.indexRefs : Bytecode → List IndexRef
  | .pop | .ret | .readRef | .writeRef | .freezeRef => []
  | .brTrue o | .brFalse o | .branch o => [.codeOffset o]
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => []
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => []
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => []
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => []
  | .ldConst i => [.constant i]
  | .ldTrue | .ldFalse => []
  | .copyLoc i | .moveLoc i | .stLoc i | .mutBorrowLoc i | .immBorrowLoc i => [.local_ i]
  | .call h => [.functionHandle h]
  | .callGeneric i => [.functionInstantiation i]
  | .pack d | .unpack d => [.structDef d]
  | .packGeneric i | .unpackGeneric i => [.structDefInstantiation i]
  | .packVariant h | .unpackVariant h | .testVariant h => [.structVariantHandle h]
  | .packVariantGeneric i | .unpackVariantGeneric i | .testVariantGeneric i =>
    [.structVariantInstantiation i]
  | .mutBorrowField h | .immBorrowField h => [.fieldHandle h]
  | .mutBorrowFieldGeneric i | .immBorrowFieldGeneric i => [.fieldInstantiation i]
  | .mutBorrowVariantField h | .immBorrowVariantField h => [.variantFieldHandle h]
  | .mutBorrowVariantFieldGeneric i | .immBorrowVariantFieldGeneric i =>
    [.variantFieldInstantiation i]
  | .mutBorrowGlobal d | .immBorrowGlobal d | .exists_ d | .moveFrom d | .moveTo d =>
    [.structDef d]
  | .mutBorrowGlobalGeneric i | .immBorrowGlobalGeneric i | .existsGeneric i
  | .moveFromGeneric i | .moveToGeneric i => [.structDefInstantiation i]
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => []
  | .eq | .neq | .lt | .gt | .le | .ge | .abort | .nop | .shl | .shr => []
  | .negate | .abortMsg => []
  | .vecPack s _ | .vecLen s | .vecImmBorrow s | .vecMutBorrow s | .vecPushBack s
  | .vecPopBack s | .vecUnpack s _ | .vecSwap s => [.signature s]
  | .packClosure h _ => [.functionHandle h]
  | .packClosureGeneric i _ => [.functionInstantiation i]
  | .callClosure s => [.signature s]

namespace CompiledModule

/-- All tokens of a struct definition's fields, across variants. -/
def StructFieldInformation.allFields : StructFieldInformation → List FieldDefinition
  | .native => []
  | .declared fields => fields
  | .declaredVariants variants => variants.flatMap (·.fields)

/-- The number of fields of a struct (not an enum) definition. -/
def declaredFieldCount (m : CompiledModule) (d : StructDefinitionIndex) : Option Nat := do
  match (← m.structDefs[d]?).fieldInformation with
  | .declared fields => some fields.length
  | .native => none
  | .declaredVariants _ => none

/-- The variants of an enum definition. -/
def variants (m : CompiledModule) (d : StructDefinitionIndex) : Option (List VariantDefinition) := do
  match (← m.structDefs[d]?).fieldInformation with
  | .declaredVariants vs => some vs
  | .native => none
  | .declared _ => none

/-- Struct handles and type parameters referenced by a token are in range;
`typeParams` is the number of type parameters in scope. -/
def TokenInRange (m : CompiledModule) (typeParams : Nat) (t : SignatureToken) : Prop :=
  ∀ r ∈ t.refs, match r with
    | .structHandle i => i < m.structHandles.size
    | .typeParameter i => i < typeParams

def SignatureInRange (m : CompiledModule) (typeParams : Nat) (s : SignatureIndex) : Prop :=
  ∃ sig, m.signatures[s]? = some sig ∧ ∀ t ∈ sig, m.TokenInRange typeParams t

/-- An instruction's reference is in range, in a function with
`localsCount` locals, `codeLen` instructions and `typeParams` type
parameters. Type arguments of instantiations must be in scope. -/
def RefInRange (m : CompiledModule) (localsCount codeLen typeParams : Nat) : IndexRef → Prop
  | .local_ i => i < localsCount
  | .codeOffset o => o < codeLen
  | .constant i => i < m.constantPool.size
  | .functionHandle i => i < m.functionHandles.size
  | .functionInstantiation i =>
    ∃ x, m.functionInstantiations[i]? = some x ∧ m.SignatureInRange typeParams x.typeParameters
  | .structDef i => i < m.structDefs.size
  | .structDefInstantiation i =>
    ∃ x, m.structDefInstantiations[i]? = some x ∧ m.SignatureInRange typeParams x.typeParameters
  | .structVariantHandle i => i < m.structVariantHandles.size
  | .structVariantInstantiation i =>
    ∃ x, m.structVariantInstantiations[i]? = some x ∧
      m.SignatureInRange typeParams x.typeParameters
  | .fieldHandle i => i < m.fieldHandles.size
  | .fieldInstantiation i =>
    ∃ x, m.fieldInstantiations[i]? = some x ∧ m.SignatureInRange typeParams x.typeParameters
  | .variantFieldHandle i => i < m.variantFieldHandles.size
  | .variantFieldInstantiation i =>
    ∃ x, m.variantFieldInstantiations[i]? = some x ∧
      m.SignatureInRange typeParams x.typeParameters
  | .signature i => m.SignatureInRange typeParams i

/-- **L1.** Every index into the module's own tables is in range, as are
local, code-offset and type-parameter indices in code. Whether imported
handles resolve is a linking property (K1). Access specifiers are not
covered: the production verifier rejects them. -/
def IndexIntegrity (m : CompiledModule) : Prop :=
  m.selfModuleHandleIdx < m.moduleHandles.size ∧
  (∀ mh ∈ m.moduleHandles.toList ++ m.friendDecls.toList,
    mh.address < m.addressIdentifiers.size ∧ mh.name < m.identifiers.size) ∧
  (∀ sh ∈ m.structHandles.toList,
    sh.module < m.moduleHandles.size ∧ sh.name < m.identifiers.size) ∧
  (∀ fh ∈ m.functionHandles.toList,
    fh.module < m.moduleHandles.size ∧ fh.name < m.identifiers.size ∧
    m.SignatureInRange fh.typeParameters.length fh.parameters ∧
    m.SignatureInRange fh.typeParameters.length fh.returns) ∧
  (∀ h ∈ m.fieldHandles.toList, ∃ n, m.declaredFieldCount h.owner = some n ∧ h.field < n) ∧
  (∀ h ∈ m.variantFieldHandles.toList, ∃ vs, m.variants h.structIndex = some vs ∧
    ∀ v ∈ h.variants, ∃ vd, vs[v]? = some vd ∧ h.field < vd.fields.length) ∧
  (∀ h ∈ m.structVariantHandles.toList, ∃ vs, m.variants h.structIndex = some vs ∧
    h.variant < vs.length) ∧
  (∀ x ∈ m.structDefInstantiations.toList, x.def_ < m.structDefs.size) ∧
  (∀ x ∈ m.functionInstantiations.toList, x.handle < m.functionHandles.size) ∧
  (∀ x ∈ m.fieldInstantiations.toList, x.handle < m.fieldHandles.size) ∧
  (∀ x ∈ m.structVariantInstantiations.toList, x.handle < m.structVariantHandles.size) ∧
  (∀ x ∈ m.variantFieldInstantiations.toList, x.handle < m.variantFieldHandles.size) ∧
  (∀ c ∈ m.constantPool.toList, m.TokenInRange 0 c.type) ∧
  (∀ d ∈ m.structDefs.toList, ∃ sh, m.structHandles[d.structHandle]? = some sh ∧
    ∀ f ∈ StructFieldInformation.allFields d.fieldInformation,
      f.name < m.identifiers.size ∧ m.TokenInRange sh.typeParameters.length f.signature) ∧
  (∀ d ∈ m.structDefs.toList, ∀ vs, d.fieldInformation = .declaredVariants vs →
    ∀ v ∈ vs, v.name < m.identifiers.size) ∧
  (∀ d ∈ m.functionDefs.toList, ∃ fh, m.functionHandles[d.function]? = some fh ∧
    (∀ a ∈ d.acquiresGlobalResources, a < m.structDefs.size) ∧
    ∀ code, d.code = some code →
      ∃ params locals, m.signatures[fh.parameters]? = some params ∧
        m.signatures[code.locals]? = some locals ∧
        m.SignatureInRange fh.typeParameters.length code.locals ∧
        ∀ instr ∈ code.code.toList, ∀ r ∈ instr.indexRefs,
          m.RefInRange (params.length + locals.length) code.code.size
            fh.typeParameters.length r)

/-- **L2.** An index names one entity: pools and handles have no duplicates,
no two handles share a module and name, no two definitions share a handle,
and every definition's handle names this module. -/
def Uniqueness (m : CompiledModule) : Prop :=
  m.identifiers.toList.Nodup ∧
  m.addressIdentifiers.toList.Nodup ∧
  m.moduleHandles.toList.Nodup ∧
  m.friendDecls.toList.Nodup ∧
  (m.structHandles.toList.map (fun h => (h.module, h.name))).Nodup ∧
  (m.functionHandles.toList.map (fun h => (h.module, h.name))).Nodup ∧
  (m.structDefs.toList.map (·.structHandle)).Nodup ∧
  (m.functionDefs.toList.map (·.function)).Nodup ∧
  (∀ d ∈ m.structDefs.toList, ∃ sh, m.structHandles[d.structHandle]? = some sh ∧
    sh.module = m.selfModuleHandleIdx) ∧
  (∀ d ∈ m.functionDefs.toList, ∃ fh, m.functionHandles[d.function]? = some fh ∧
    fh.module = m.selfModuleHandleIdx)

end CompiledModule

/-! ## Control flow (L3) -/

/-- How an instruction transfers control. -/
inductive Control where
  /-- Continues with the next instruction. -/
  | fallThrough
  /-- Unconditional jump. -/
  | jump (target : CodeOffset)
  /-- Conditional jump: the target or the next instruction. -/
  | branch (target : CodeOffset)
  /-- Leaves the function. -/
  | exit
  deriving DecidableEq, Repr

def Bytecode.control : Bytecode → Control
  | .ret | .abort | .abortMsg => .exit
  | .branch o => .jump o
  | .brTrue o | .brFalse o => .branch o
  | .pop | .readRef | .writeRef | .freezeRef => .fallThrough
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => .fallThrough
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => .fallThrough
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => .fallThrough
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => .fallThrough
  | .ldConst _ | .ldTrue | .ldFalse => .fallThrough
  | .copyLoc _ | .moveLoc _ | .stLoc _ | .mutBorrowLoc _ | .immBorrowLoc _ => .fallThrough
  | .call _ | .callGeneric _ | .callClosure _ => .fallThrough
  | .pack _ | .packGeneric _ | .packVariant _ | .packVariantGeneric _ => .fallThrough
  | .unpack _ | .unpackGeneric _ | .unpackVariant _ | .unpackVariantGeneric _ => .fallThrough
  | .testVariant _ | .testVariantGeneric _ => .fallThrough
  | .mutBorrowField _ | .mutBorrowVariantField _ | .mutBorrowFieldGeneric _
  | .mutBorrowVariantFieldGeneric _ => .fallThrough
  | .immBorrowField _ | .immBorrowVariantField _ | .immBorrowFieldGeneric _
  | .immBorrowVariantFieldGeneric _ => .fallThrough
  | .mutBorrowGlobal _ | .mutBorrowGlobalGeneric _ | .immBorrowGlobal _
  | .immBorrowGlobalGeneric _ => .fallThrough
  | .exists_ _ | .existsGeneric _ | .moveFrom _ | .moveFromGeneric _ | .moveTo _
  | .moveToGeneric _ => .fallThrough
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => .fallThrough
  | .eq | .neq | .lt | .gt | .le | .ge | .nop | .shl | .shr | .negate => .fallThrough
  | .vecPack _ _ | .vecLen _ | .vecImmBorrow _ | .vecMutBorrow _ | .vecPushBack _
  | .vecPopBack _ | .vecUnpack _ _ | .vecSwap _ => .fallThrough
  | .packClosure _ _ | .packClosureGeneric _ _ => .fallThrough

/-- The successors of instruction `pc`. -/
def successors (code : Array Bytecode) (pc : Nat) : List Nat :=
  match code[pc]? with
  | none => []
  | some i =>
    match i.control with
    | .fallThrough => [pc + 1]
    | .jump o => [o]
    | .branch o => [o, pc + 1]
    | .exit => []

/-- A walk in a graph given by its successor function, listing its nodes. -/
inductive Walk (succ : Nat → List Nat) : Nat → Nat → List Nat → Prop where
  | nil (n : Nat) : Walk succ n n [n]
  | cons {a b c : Nat} {p : List Nat} : b ∈ succ a → Walk succ b c p → Walk succ a c (a :: p)

/-- `d` dominates `n`: every walk from the entry to `n` passes through `d`. -/
def Dominates (succ : Nat → List Nat) (d n : Nat) : Prop :=
  ∀ p, Walk succ 0 n p → d ∈ p

/-- A forward edge: an edge whose target does not dominate its source. The
remaining edges are back edges. -/
def ForwardEdge (succ : Nat → List Nat) (a b : Nat) : Prop :=
  b ∈ succ a ∧ ¬ Dominates succ b a

/-- Transitive closure. -/
inductive Plus (r : Nat → Nat → Prop) : Nat → Nat → Prop where
  | single {a b : Nat} : r a b → Plus r a b
  | tail {a b c : Nat} : Plus r a b → r b c → Plus r a c

/-- A graph is reducible when its forward edges form no cycle through a node
reachable from the entry: every cycle is entered through a node that
dominates it. -/
def Reducible (succ : Nat → List Nat) : Prop :=
  ∀ n, (∃ p, Walk succ 0 n p) → ¬ Plus (ForwardEdge succ) n n

/-- **L3** for one code unit. A fall-through from the last instruction would
leave the code, so "no path falls off the end" is part of "every successor is
in range". -/
def ControlFlowWellFormed (code : Array Bytecode) : Prop :=
  0 < code.size ∧
  (∀ pc, pc < code.size → ∀ s ∈ successors code pc, s < code.size) ∧
  Reducible (successors code)

/-! ## Stack discipline (L4) -/

namespace CompiledModule

/-- Parameter and return counts of a function handle. -/
def handleArity (m : CompiledModule) (h : FunctionHandleIndex) : Option (Nat × Nat) := do
  let fh ← m.functionHandles[h]?
  pure ((← m.signatures[fh.parameters]?).length, (← m.signatures[fh.returns]?).length)

def instantiationArity (m : CompiledModule) (i : FunctionInstantiationIndex) :
    Option (Nat × Nat) := do
  m.handleArity (← m.functionInstantiations[i]?).handle

def structInstFieldCount (m : CompiledModule) (i : StructDefInstantiationIndex) : Option Nat := do
  m.declaredFieldCount (← m.structDefInstantiations[i]?).def_

def variantFieldCount (m : CompiledModule) (h : StructVariantHandleIndex) : Option Nat := do
  let vh ← m.structVariantHandles[h]?
  let vs ← m.variants vh.structIndex
  pure (← vs[vh.variant]?).fields.length

def variantInstFieldCount (m : CompiledModule) (i : StructVariantInstantiationIndex) :
    Option Nat := do
  m.variantFieldCount (← m.structVariantInstantiations[i]?).handle

/-- The function type named by a `CallClosure` signature. -/
def closureArity (m : CompiledModule) (s : SignatureIndex) : Option (Nat × Nat) := do
  match ← m.signatures[s]? with
  | [.function params results _] => some (params.length + 1, results.length)
  | _ => none

end CompiledModule

/-- The number of captured arguments named by a closure mask. -/
def maskCount (mask : BitVec 64) : Nat :=
  (List.range 64).countP (fun i => mask.getLsbD i)

/-- How many values an instruction pops and pushes, in function `fh` of module
`m`. `none` when a table entry it depends on is missing. -/
def Bytecode.arity (m : CompiledModule) (fh : FunctionHandle) : Bytecode → Option (Nat × Nat)
  | .pop => some (1, 0)
  | .ret => do pure ((← m.signatures[fh.returns]?).length, 0)
  | .brTrue _ | .brFalse _ => some (1, 0)
  | .branch _ => some (0, 0)
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => some (0, 1)
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => some (0, 1)
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => some (1, 1)
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => some (1, 1)
  | .ldConst _ | .ldTrue | .ldFalse => some (0, 1)
  | .copyLoc _ | .moveLoc _ | .mutBorrowLoc _ | .immBorrowLoc _ => some (0, 1)
  | .stLoc _ => some (1, 0)
  | .call h => m.handleArity h
  | .callGeneric i => m.instantiationArity i
  | .callClosure s => m.closureArity s
  | .pack d => do pure (← m.declaredFieldCount d, 1)
  | .packGeneric i => do pure (← m.structInstFieldCount i, 1)
  | .packVariant h => do pure (← m.variantFieldCount h, 1)
  | .packVariantGeneric i => do pure (← m.variantInstFieldCount i, 1)
  | .unpack d => do pure (1, ← m.declaredFieldCount d)
  | .unpackGeneric i => do pure (1, ← m.structInstFieldCount i)
  | .unpackVariant h => do pure (1, ← m.variantFieldCount h)
  | .unpackVariantGeneric i => do pure (1, ← m.variantInstFieldCount i)
  | .testVariant _ | .testVariantGeneric _ => some (1, 1)
  | .readRef | .freezeRef => some (1, 1)
  | .writeRef => some (2, 0)
  | .mutBorrowField _ | .mutBorrowVariantField _ | .mutBorrowFieldGeneric _
  | .mutBorrowVariantFieldGeneric _ => some (1, 1)
  | .immBorrowField _ | .immBorrowVariantField _ | .immBorrowFieldGeneric _
  | .immBorrowVariantFieldGeneric _ => some (1, 1)
  | .mutBorrowGlobal _ | .mutBorrowGlobalGeneric _ | .immBorrowGlobal _
  | .immBorrowGlobalGeneric _ => some (1, 1)
  | .exists_ _ | .existsGeneric _ | .moveFrom _ | .moveFromGeneric _ => some (1, 1)
  | .moveTo _ | .moveToGeneric _ => some (2, 0)
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and => some (2, 1)
  | .eq | .neq | .lt | .gt | .le | .ge | .shl | .shr => some (2, 1)
  | .not | .negate => some (1, 1)
  | .abort => some (1, 0)
  | .abortMsg => some (2, 0)
  | .nop => some (0, 0)
  | .vecPack _ n => some (n, 1)
  | .vecLen _ | .vecPopBack _ => some (1, 1)
  | .vecImmBorrow _ | .vecMutBorrow _ => some (2, 1)
  | .vecPushBack _ => some (2, 0)
  | .vecUnpack _ n => some (1, n)
  | .vecSwap _ => some (3, 0)
  | .packClosure _ mask | .packClosureGeneric _ mask => some (maskCount mask, 1)

/-- The first instruction of a basic block: the entry, a jump target, or the
instruction after one that ends a block. -/
def IsLeader (code : Array Bytecode) (pc : Nat) : Prop :=
  pc = 0 ∨
  ∃ q, ∃ i : Bytecode, code[q]? = some i ∧
    match i.control with
    | .jump o => pc = o ∨ pc = q + 1
    | .branch o => pc = o ∨ pc = q + 1
    | .exit => pc = q + 1
    | .fallThrough => False

/-- **L4** for one code unit of function `fh` in module `m`: a height function
`h` gives the operand-stack height before every instruction, consistent along
every edge, never underflowing or exceeding `maxStack`, zero at every block
boundary, and zero after an exit (so `Ret` runs with exactly the declared
return values). It covers every instruction, reachable or not. -/
def StackDiscipline (m : CompiledModule) (fh : FunctionHandle) (code : Array Bytecode)
    (maxStack : Nat) : Prop :=
  ∃ h : Nat → Nat, h 0 = 0 ∧
    ∀ pc i, code[pc]? = some i →
      ∃ pops pushes, i.arity m fh = some (pops, pushes) ∧
        pops ≤ h pc ∧ h pc ≤ maxStack ∧ h pc - pops + pushes ≤ maxStack ∧
        (∀ s ∈ successors code pc, h s = h pc - pops + pushes) ∧
        (IsLeader code pc → h pc = 0) ∧
        (i.control = .exit → h pc - pops + pushes = 0)

namespace CompiledModule

/-- **L3** and **L4** for every function of a module. -/
def CodeStructure (m : CompiledModule) (maxStack : Nat) : Prop :=
  ∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code →
    ∃ fh, m.functionHandles[d.function]? = some fh ∧
      ControlFlowWellFormed code.code ∧ StackDiscipline m fh code.code maxStack

end CompiledModule

end MoveBytecode
