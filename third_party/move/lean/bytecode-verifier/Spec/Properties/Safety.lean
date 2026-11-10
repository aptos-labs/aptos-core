-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Config
import Spec.Semantics.Run
import Spec.Properties.Linking

/-!
# Safety: the supported subset, premises, D1, N1, and the verifier interface

TRUSTED. Premises and conclusions of the soundness theorems (plan §7–§8). The
verifier model and its proofs are untrusted; only these statements, and the
semantics they quantify over, are trusted. `Spec.Properties.Theorems` states
the theorems themselves.

Status: plan Phase 4, stage B. `Supported` names the core subset of Phase 1,
data by value (Phase 2a: signers, vectors, structs and enums), the reference
and global storage instructions (Phase 3), with reference types for
parameters, return values and locals, generics (Phase 4, stage A): generic
functions, structs and enums, type parameters in every signature, and the
generic form of every instruction, and function values (stage B): function
types in every signature, `PackClosure`, `PackClosureGeneric` and
`CallClosure`. Natives take and return values only, may be generic, and name
no function type, except the modeled natives with references
(`Spec.Semantics.ModeledNatives`, 2026-11-03), the table natives among them
where the table module keeps the table rules (2026-11-04). Closures target
functions with code.
-/

namespace MoveBytecode

/-! ## The supported subset -/

/-- Instructions of the core subset (plan Phase 1). -/
def Bytecode.inCore : Bytecode → Bool
  | .pop | .ret | .brTrue _ | .brFalse _ | .branch _ => true
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => true
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => true
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => true
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => true
  | .ldConst _ | .ldTrue | .ldFalse | .copyLoc _ | .moveLoc _ | .stLoc _ | .call _ => true
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => true
  | .eq | .neq | .lt | .gt | .le | .ge | .abort | .nop | .shl | .shr | .negate => true
  | .callGeneric _ | .callClosure _ | .packClosure _ _ | .packClosureGeneric _ _ => false
  | .pack _ | .packGeneric _ | .packVariant _ | .packVariantGeneric _ => false
  | .unpack _ | .unpackGeneric _ | .unpackVariant _ | .unpackVariantGeneric _ => false
  | .testVariant _ | .testVariantGeneric _ => false
  | .readRef | .writeRef | .freezeRef | .mutBorrowLoc _ | .immBorrowLoc _ => false
  | .mutBorrowField _ | .mutBorrowVariantField _ | .mutBorrowFieldGeneric _
  | .mutBorrowVariantFieldGeneric _ => false
  | .immBorrowField _ | .immBorrowVariantField _ | .immBorrowFieldGeneric _
  | .immBorrowVariantFieldGeneric _ => false
  | .mutBorrowGlobal _ | .mutBorrowGlobalGeneric _ | .immBorrowGlobal _
  | .immBorrowGlobalGeneric _ => false
  | .exists_ _ | .existsGeneric _ | .moveFrom _ | .moveFromGeneric _ | .moveTo _
  | .moveToGeneric _ => false
  | .vecPack _ _ | .vecLen _ | .vecImmBorrow _ | .vecMutBorrow _ | .vecPushBack _
  | .vecPopBack _ | .vecUnpack _ _ | .vecSwap _ => false
  | .abortMsg => false

/-- Instructions that Phase 2a (data by value) adds to the core: the
non-generic forms that build and take apart struct, enum and vector values
without references, and `AbortMsg`, whose message is a vector. -/
def Bytecode.inDataByValue : Bytecode → Bool
  | .pack _ | .unpack _ | .packVariant _ | .unpackVariant _ => true
  | .vecPack _ _ | .vecUnpack _ _ | .abortMsg => true
  | .pop | .ret | .brTrue _ | .brFalse _ | .branch _ => false
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => false
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => false
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => false
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => false
  | .ldConst _ | .ldTrue | .ldFalse | .copyLoc _ | .moveLoc _ | .stLoc _ | .call _ => false
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => false
  | .eq | .neq | .lt | .gt | .le | .ge | .abort | .nop | .shl | .shr | .negate => false
  | .callGeneric _ | .callClosure _ | .packClosure _ _ | .packClosureGeneric _ _ => false
  | .packGeneric _ | .packVariantGeneric _ | .unpackGeneric _ | .unpackVariantGeneric _ => false
  | .testVariant _ | .testVariantGeneric _ => false
  | .readRef | .writeRef | .freezeRef | .mutBorrowLoc _ | .immBorrowLoc _ => false
  | .mutBorrowField _ | .mutBorrowVariantField _ | .mutBorrowFieldGeneric _
  | .mutBorrowVariantFieldGeneric _ => false
  | .immBorrowField _ | .immBorrowVariantField _ | .immBorrowFieldGeneric _
  | .immBorrowVariantFieldGeneric _ => false
  | .mutBorrowGlobal _ | .mutBorrowGlobalGeneric _ | .immBorrowGlobal _
  | .immBorrowGlobalGeneric _ => false
  | .exists_ _ | .existsGeneric _ | .moveFrom _ | .moveFromGeneric _ | .moveTo _
  | .moveToGeneric _ => false
  | .vecLen _ | .vecImmBorrow _ | .vecMutBorrow _ | .vecPushBack _ | .vecPopBack _
  | .vecSwap _ => false

/-- Instructions that Phase 3, stage A (references and global storage) adds:
the non-generic borrows of locals, fields and variant fields, reading,
writing and freezing through references, `TestVariant`, the vector
operations through references, and the non-generic global storage
operations. -/
def Bytecode.inReferences : Bytecode → Bool
  | .readRef | .writeRef | .freezeRef | .mutBorrowLoc _ | .immBorrowLoc _ => true
  | .mutBorrowField _ | .immBorrowField _ | .mutBorrowVariantField _
  | .immBorrowVariantField _ | .testVariant _ => true
  | .vecLen _ | .vecImmBorrow _ | .vecMutBorrow _ | .vecPushBack _ | .vecPopBack _
  | .vecSwap _ => true
  | .exists_ _ | .moveFrom _ | .moveTo _ | .mutBorrowGlobal _ | .immBorrowGlobal _ => true
  | .pop | .ret | .brTrue _ | .brFalse _ | .branch _ => false
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => false
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => false
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => false
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => false
  | .ldConst _ | .ldTrue | .ldFalse | .copyLoc _ | .moveLoc _ | .stLoc _ | .call _ => false
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => false
  | .eq | .neq | .lt | .gt | .le | .ge | .abort | .nop | .shl | .shr | .negate => false
  | .callGeneric _ | .callClosure _ | .packClosure _ _ | .packClosureGeneric _ _ => false
  | .pack _ | .packGeneric _ | .packVariant _ | .packVariantGeneric _ => false
  | .unpack _ | .unpackGeneric _ | .unpackVariant _ | .unpackVariantGeneric _ => false
  | .testVariantGeneric _ | .mutBorrowFieldGeneric _ | .mutBorrowVariantFieldGeneric _ => false
  | .immBorrowFieldGeneric _ | .immBorrowVariantFieldGeneric _ => false
  | .mutBorrowGlobalGeneric _ | .immBorrowGlobalGeneric _ | .existsGeneric _
  | .moveFromGeneric _ | .moveToGeneric _ => false
  | .vecPack _ _ | .vecUnpack _ _ | .abortMsg => false

/-- Instructions that Phase 4, stage A (generics) adds: the generic form of
every instruction of the subset that names a function, a struct or an enum.
`PackClosureGeneric` is stage B's (`inClosures`). -/
def Bytecode.inGenerics : Bytecode → Bool
  | .callGeneric _ | .packGeneric _ | .unpackGeneric _ | .packVariantGeneric _
  | .unpackVariantGeneric _ | .testVariantGeneric _ => true
  | .mutBorrowFieldGeneric _ | .immBorrowFieldGeneric _ | .mutBorrowVariantFieldGeneric _
  | .immBorrowVariantFieldGeneric _ => true
  | .mutBorrowGlobalGeneric _ | .immBorrowGlobalGeneric _ | .existsGeneric _
  | .moveFromGeneric _ | .moveToGeneric _ => true
  | .pop | .ret | .brTrue _ | .brFalse _ | .branch _ => false
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => false
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => false
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => false
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => false
  | .ldConst _ | .ldTrue | .ldFalse | .copyLoc _ | .moveLoc _ | .stLoc _ | .call _ => false
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => false
  | .eq | .neq | .lt | .gt | .le | .ge | .abort | .nop | .shl | .shr | .negate => false
  | .callClosure _ | .packClosure _ _ | .packClosureGeneric _ _ => false
  | .pack _ | .packVariant _ | .unpack _ | .unpackVariant _ | .testVariant _ => false
  | .readRef | .writeRef | .freezeRef | .mutBorrowLoc _ | .immBorrowLoc _ => false
  | .mutBorrowField _ | .mutBorrowVariantField _ | .immBorrowField _ | .immBorrowVariantField _ => false
  | .mutBorrowGlobal _ | .immBorrowGlobal _ | .exists_ _ | .moveFrom _ | .moveTo _ => false
  | .vecPack _ _ | .vecLen _ | .vecImmBorrow _ | .vecMutBorrow _ | .vecPushBack _
  | .vecPopBack _ | .vecUnpack _ _ | .vecSwap _ => false
  | .abortMsg => false

/-- Instructions that Phase 4, stage B (function values) adds: closure
creation and closure calls. -/
def Bytecode.inClosures : Bytecode → Bool
  | .packClosure _ _ | .packClosureGeneric _ _ | .callClosure _ => true
  | .pop | .ret | .brTrue _ | .brFalse _ | .branch _ => false
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => false
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => false
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => false
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => false
  | .ldConst _ | .ldTrue | .ldFalse | .copyLoc _ | .moveLoc _ | .stLoc _ | .call _ => false
  | .callGeneric _ => false
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => false
  | .eq | .neq | .lt | .gt | .le | .ge | .abort | .nop | .shl | .shr | .negate => false
  | .pack _ | .packGeneric _ | .packVariant _ | .packVariantGeneric _ => false
  | .unpack _ | .unpackGeneric _ | .unpackVariant _ | .unpackVariantGeneric _ => false
  | .testVariant _ | .testVariantGeneric _ => false
  | .readRef | .writeRef | .freezeRef | .mutBorrowLoc _ | .immBorrowLoc _ => false
  | .mutBorrowField _ | .mutBorrowVariantField _ | .mutBorrowFieldGeneric _
  | .mutBorrowVariantFieldGeneric _ => false
  | .immBorrowField _ | .immBorrowVariantField _ | .immBorrowFieldGeneric _
  | .immBorrowVariantFieldGeneric _ => false
  | .mutBorrowGlobal _ | .mutBorrowGlobalGeneric _ | .immBorrowGlobal _
  | .immBorrowGlobalGeneric _ => false
  | .exists_ _ | .existsGeneric _ | .moveFrom _ | .moveFromGeneric _ | .moveTo _
  | .moveToGeneric _ => false
  | .vecPack _ _ | .vecLen _ | .vecImmBorrow _ | .vecMutBorrow _ | .vecPushBack _
  | .vecPopBack _ | .vecUnpack _ _ | .vecSwap _ => false
  | .abortMsg => false

/-- Instructions of the supported subset: the core, data by value, references
and global storage, their generic forms, and closures. -/
def Bytecode.inSubset (i : Bytecode) : Bool :=
  i.inCore || i.inDataByValue || i.inReferences || i.inGenerics || i.inClosures

mutual
/-- A type token that names a function type anywhere in it. -/
def SignatureToken.mentionsFunction : SignatureToken → Bool
  | .function .. => true
  | .vector elem => elem.mentionsFunction
  | .structInstantiation _ args => SignatureToken.anyMentionsFunction args
  | .reference t | .mutableReference t => t.mentionsFunction
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 => false
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => false
  | .address | .signer | .struct _ | .typeParameter _ => false

def SignatureToken.anyMentionsFunction : List SignatureToken → Bool
  | [] => false
  | t :: ts => t.mentionsFunction || SignatureToken.anyMentionsFunction ts
end

/-- The function handle a closure-creating instruction names: its own for
`PackClosure`, its instantiation's for `PackClosureGeneric`. -/
def Bytecode.closureHandle? (m : CompiledModule) : Bytecode → Option FunctionHandleIndex
  | .packClosure h _ => some h
  | .packClosureGeneric i _ => (m.functionInstantiations[i]?).map (·.handle)
  | .callClosure _ => none
  | .pop | .ret | .brTrue _ | .brFalse _ | .branch _ => none
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => none
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => none
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => none
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => none
  | .ldConst _ | .ldTrue | .ldFalse | .copyLoc _ | .moveLoc _ | .stLoc _ | .call _ => none
  | .callGeneric _ => none
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => none
  | .eq | .neq | .lt | .gt | .le | .ge | .abort | .nop | .shl | .shr | .negate => none
  | .pack _ | .packGeneric _ | .packVariant _ | .packVariantGeneric _ => none
  | .unpack _ | .unpackGeneric _ | .unpackVariant _ | .unpackVariantGeneric _ => none
  | .testVariant _ | .testVariantGeneric _ => none
  | .readRef | .writeRef | .freezeRef | .mutBorrowLoc _ | .immBorrowLoc _ => none
  | .mutBorrowField _ | .mutBorrowVariantField _ | .mutBorrowFieldGeneric _
  | .mutBorrowVariantFieldGeneric _ => none
  | .immBorrowField _ | .immBorrowVariantField _ | .immBorrowFieldGeneric _
  | .immBorrowVariantFieldGeneric _ => none
  | .mutBorrowGlobal _ | .mutBorrowGlobalGeneric _ | .immBorrowGlobal _
  | .immBorrowGlobalGeneric _ => none
  | .exists_ _ | .existsGeneric _ | .moveFrom _ | .moveFromGeneric _ | .moveTo _
  | .moveToGeneric _ => none
  | .vecPack _ _ | .vecLen _ | .vecImmBorrow _ | .vecMutBorrow _ | .vecPushBack _
  | .vecPopBack _ | .vecUnpack _ _ | .vecSwap _ => none
  | .abortMsg => none

/-- A signature of parameter, return or local types, all of the subset where
`n` type parameters are in scope: value types and references to them, which
translate under any `n` closed type arguments (`RuntimeType.ofSlotTokenWith`).
A token is in the subset when it has no function type; that it translates
under every instance also requires what makes it a type at all (struct
handles in range, one type argument per type parameter of a struct handle,
type parameters in scope, references only at the top and at the parameters
and results of function types), which local verification establishes (L1,
L5) but is required here, as in Phases 1–3. -/
def CompiledModule.SupportedSignature (m : CompiledModule) (s : SignatureIndex) (n : Nat := 0) :
    Prop :=
  ∃ sig, m.signatures[s]? = some sig ∧
    ∀ tys : List RuntimeType, tys.length = n → (RuntimeType.ofSignatureWith m tys sig).isSome

/-- A signature of value types of the subset, without references, where `n`
type parameters are in scope (`RuntimeType.ofTokenWith`). -/
def CompiledModule.SupportedValueSignature (m : CompiledModule) (s : SignatureIndex)
    (n : Nat := 0) : Prop :=
  ∃ sig, m.signatures[s]? = some sig ∧
    ∀ tys : List RuntimeType, tys.length = n → (RuntimeType.ofTokenListWith m tys sig).isSome

/-- The type operands of an instruction are in the subset, where `n` type
parameters are in scope: the element type of the vector instructions, the
type arguments of the generic forms, and the function type of `CallClosure`.
Other supported instructions have no type operand. -/
def CompiledModule.SupportedOperands (m : CompiledModule) (i : Bytecode) (n : Nat := 0) : Prop :=
  match i with
  | .vecPack s _ | .vecUnpack s _ | .vecLen s | .vecImmBorrow s | .vecMutBorrow s
  | .vecPushBack s | .vecPopBack s | .vecSwap s =>
    ∀ tys : List RuntimeType, tys.length = n → (m.vectorElemTypeWith tys s).isSome
  | .callGeneric i =>
    ∃ x, m.functionInstantiations[i]? = some x ∧ m.SupportedValueSignature x.typeParameters n
  | .packGeneric i | .unpackGeneric i | .mutBorrowGlobalGeneric i | .immBorrowGlobalGeneric i
  | .existsGeneric i | .moveFromGeneric i | .moveToGeneric i =>
    ∃ x, m.structDefInstantiations[i]? = some x ∧ m.SupportedValueSignature x.typeParameters n
  | .packVariantGeneric i | .unpackVariantGeneric i | .testVariantGeneric i =>
    ∃ x, m.structVariantInstantiations[i]? = some x ∧ m.SupportedValueSignature x.typeParameters n
  | .mutBorrowFieldGeneric i | .immBorrowFieldGeneric i =>
    ∃ x, m.fieldInstantiations[i]? = some x ∧ m.SupportedValueSignature x.typeParameters n
  | .mutBorrowVariantFieldGeneric i | .immBorrowVariantFieldGeneric i =>
    ∃ x, m.variantFieldInstantiations[i]? = some x ∧ m.SupportedValueSignature x.typeParameters n
  | .packClosureGeneric i _ =>
    ∃ x, m.functionInstantiations[i]? = some x ∧ m.SupportedValueSignature x.typeParameters n
  | .callClosure s => m.SupportedValueSignature s n
  | .packClosure _ _ => True
  | .pop | .ret | .brTrue _ | .brFalse _ | .branch _ => True
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => True
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => True
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => True
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => True
  | .ldConst _ | .ldTrue | .ldFalse | .copyLoc _ | .moveLoc _ | .stLoc _ | .call _ => True
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => True
  | .eq | .neq | .lt | .gt | .le | .ge | .abort | .nop | .shl | .shr | .negate => True
  | .pack _ | .packVariant _ | .unpack _ | .unpackVariant _ | .testVariant _ => True
  | .readRef | .writeRef | .freezeRef | .mutBorrowLoc _ | .immBorrowLoc _ => True
  | .mutBorrowField _ | .mutBorrowVariantField _ | .immBorrowField _ | .immBorrowVariantField _ => True
  | .mutBorrowGlobal _ | .immBorrowGlobal _ | .exists_ _ | .moveFrom _ | .moveTo _ => True
  | .abortMsg => True

/-- Signature `s` names no function type. -/
def CompiledModule.NoFunctionTypes (m : CompiledModule) (s : SignatureIndex) : Prop :=
  ∃ sig, m.signatures[s]? = some sig ∧ ∀ t ∈ sig, t.mentionsFunction = false

/-- Every function, generic or not, has types of the subset, with its type
parameters in scope; every native function is a modeled native
(`CompiledModule.modeledNative?`, `Spec.Semantics.ModeledNatives`) or takes and
returns values only, no references and no function types; every struct
definition declares fields or variants (no native structs) of types of the
subset, with its type parameters in scope; every constant has a type of the
subset; and every instruction is in the subset with its type operands, with its
function's type parameters in scope.

Natives with reference parameters or results other than the modeled ones are a
coverage limit, not a soundness rule: the shapes of the modeled natives (read,
update or borrow; language-owner rulings, 2026-11-03, the later one making the
`#[bytecode_instruction]` natives of `std::vector` ordinary shape natives;
`Spec.Semantics.ModeledNatives`) say what a native reads and writes through the
references it receives and which references it returns, and a native they do
not describe, which may reach storage no argument designates or call back into
Move code (an aggregator, a storage slot, a dispatchable native), stays
outside. A table's entries are inside the table (language-owner rulings,
2026-11-04, `Value.table`), so the table natives reach only what their
arguments designate; a module `0x1::table` that breaks the table rules
(`CompiledModule.followsTableRules`) declares them as natives that are not
modeled, and is not in the subset.
Generic natives are in the
subset: a native receives its type arguments with its function
(`FunctionRef.typeArgs`), and its contract is stated at that instance. Natives
whose signatures name a function type are a coverage limit too: a native that
evaluates a function value is dynamic dispatch, whose contract would have to
say which Move code it runs and under which module lock (R4). A native at type
arguments that are function types may still receive closures and pass them
on. -/
def CompiledModule.Supported (m : CompiledModule) : Prop :=
  (∀ fh ∈ m.functionHandles.toList,
    m.SupportedSignature fh.parameters fh.typeParameters.length ∧
    m.SupportedSignature fh.returns fh.typeParameters.length) ∧
  (∀ d ∈ m.functionDefs.toList, d.code = none → (m.modeledNative? d).isSome ∨
    ∃ fh, m.functionHandles[d.function]? = some fh ∧
      m.SupportedValueSignature fh.parameters fh.typeParameters.length ∧
      m.SupportedValueSignature fh.returns fh.typeParameters.length ∧
      m.NoFunctionTypes fh.parameters ∧ m.NoFunctionTypes fh.returns) ∧
  (∀ d ∈ m.structDefs.toList, d.fieldInformation ≠ .native ∧
    ∃ sh, m.structDefHandle d = some sh ∧
      ∀ f ∈ CompiledModule.StructFieldInformation.allFields d.fieldInformation,
        ∀ tys : List RuntimeType, tys.length = sh.typeParameters.length →
          (RuntimeType.ofTokenWith m tys f.signature).isSome) ∧
  (∀ c ∈ m.constantPool.toList, (RuntimeType.ofToken m c.type).isSome) ∧
  (∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code →
    ∃ fh, m.functionHandles[d.function]? = some fh ∧
      m.SupportedSignature code.locals fh.typeParameters.length ∧
      ∀ i ∈ code.code.toList, i.inSubset ∧ m.SupportedOperands i fh.typeParameters.length)

/-- Every closure the program's code creates targets a function with code, when
its handle resolves (`Bytecode.closureHandle?`). A closure over a native is a
coverage limit: calling it would run the native on arguments partly taken from
the closure, which neither the native contracts nor the ghost layer describe
(`Program.closureType?`). Whether a module's handle names a native is known
only in the program, so this premise is the program's. -/
def Program.ClosureTargetsHaveCode (P : Program) : Prop :=
  ∀ mi m, P.modules[mi]? = some m → ∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code →
    ∀ instr ∈ code.code.toList, ∀ h, instr.closureHandle? m = some h →
      ∀ f info, P.resolve mi h = some f → P.function? f = some info → info.definition.code.isSome

/-- Every module is in the subset, and every closure targets a function with
code. -/
def Program.Supported (P : Program) : Prop :=
  (∀ m ∈ P.modules.toList, m.Supported) ∧ P.ClosureTargetsHaveCode

/-- Bytecode versions in scope (plan §4.2), with the version-gated format
rules the production deserializer enforces. The spec has no deserializer, so
it states them here, keeping the theorems to modules that can come from
bytes:

- below version 10, no function handle carries a struct-API attribute
  (`FunctionAttribute.isStructApi`): the deserializer rejects each one as
  `MALFORMED` (`deserialize_function_attribute`, called from
  `load_attribute`, in `move-binary-format/src/deserializer.rs`). -/
def Program.SupportedVersions (P : Program) : Prop :=
  (∀ m ∈ P.modules.toList, 6 ≤ m.version ∧ m.version ≤ 10) ∧
  (∀ m ∈ P.modules.toList, m.version < 10 →
    ∀ fh ∈ m.functionHandles.toList, ∀ a ∈ fh.attributes, ¬ a.isStructApi)

/-! ## Premises -/

/-- Native contracts (supported subset): every result a native returns fits its
shape (`Program.nativeResultFits`), at the instance it was called with (a
generic native's type arguments are part of `f`, `FunctionRef.typeArgs`), on
the arguments as it saw them (`State.nativeView`):

- its values conform, deeply, to its declared return types (to those that are
  not references, for the borrow shape);
- only a native of the update shape writes back, one value for each of its
  mutable reference parameters, conforming to the parameter's target type;
- only a native of the borrow shape returns places, one for each reference
  result, each inside what one of its reference parameters designates, of the
  declared type the result's target needs, never more mutable than that
  parameter (`nativePlaceFits`).

For a native that takes and returns values only, this is what it was before:
its values conform to its return types, and it returns nothing else. A table
native writes back a conforming table, so one with entries keeps the
abilities of its type (`Value.wellFormed`). D1 needs no more: a native
reaches no location but through its reference arguments
(the semantics shows it only what they designate and writes back only through
the mutable ones), it hands out references only to places inside them, and it
does not touch global storage otherwise (`NativeRel` gives it no state). The
contracts for the identity accounting of D2 and the encapsulation of D3 are
`NativesRespectOwnership` (`Spec.Properties.Natives`). -/
def NativesRespect (P : Program) (N : NativeRel) : Prop :=
  ∀ f args vals writes places, N f args (.returned vals writes places) →
    ∃ tys, P.returnTypes f = some tys ∧ P.nativeResultFits f tys args vals writes places = true

/-- An admissible initial state: a call of a function instance of the program,
with type arguments satisfying its constraints (`initialState`,
`Program.typeArgsOk`) and arguments conforming, deeply, to its parameter types
at that instance, over well-typed global storage (plan §7, D: "a well-typed initial state (global values conform to
their declared types)"; `GlobalStorage.wellTyped`). A reference argument
designates a value the transaction holds (`held`), of its target type, which
no other argument designates (`heldArguments`): reference parameters are
distinct roots, as for the `&signer` parameters of entry functions. The held
values are well-formed values of value types, so they contain no reference.

For D1 this admits calling any function with any conforming arguments,
including struct values and signers the transaction could not construct: type
safety does not depend on where a conforming value came from, so the stronger
statement costs nothing. D2 and D3 do depend on it: a forged value of another
module's type, or a forged signer, would falsify them. They are stated for
the initial states of transactions, `EntryInitial` (`Spec.Properties.Entry`),
which the entry premises restrict:

- E1: only entry functions that return nothing, with arguments the
  transaction layer validates.
  For struct and enum arguments the book admits only `public` types with
  `copy` and without `key`, whose fields are themselves valid argument types
  (`structs-and-enums.md`, "Transaction Arguments");
- E2: signers only from transaction authentication, for the leading `signer`
  and `&signer` parameters (`primitive-types.md`, "Signer"). -/
def AdmissibleInitial (P : Program) (s₀ : State) : Prop :=
  ∃ f args tys storage held, P.paramTypes f = some tys ∧ conformsAll P args tys = true ∧
    storage.wellTyped P = true ∧ initialState P f args storage held = some s₀

/-! ## Conclusions -/

/-- **D1.** No reachable state steps to `wrong`. -/
def NoWrong (P : Program) (N : NativeRel) (s₀ : State) : Prop :=
  ∀ s, Reachable P N s₀ s → ∀ r, ¬ StepRel P N s (.wrong r)

/-- **N1.** Progress of the semantics: when natives always produce some result,
every state has a step. The deterministic core is a total function, so this
holds by construction; it guards against a later relational step losing
cases. A call of a modeled native has a next step: it suspends with what the
native sees of its arguments, to which the native relation gives a result
(`StepRel.native`). It does not rule out a rule violation misclassified as `running`; the
semantics tests produce every `WrongReason` reachable in the supported
subset. -/
def Progress (P : Program) (N : NativeRel) : Prop :=
  (∀ f args, ∃ r, N f args r) → ∀ s, ∃ o, StepRel P N s o

/-! ## The verifier whose soundness is proved

The theorems are stated over a decision procedure: local verification of one
module, and link checks of a module against the rest of its program. The
model of the Rust verifier provides it, and differential testing compares its
decisions with the Rust verifier's on the same modules (deliverable A2).
Fixing the interface here keeps the theorems from being stated over an
arbitrary predicate. -/

structure VerifierInterface where
  verifyModule : VerifierConfig → CompiledModule → Bool
  verifyLinking : VerifierConfig → CompiledModule → List CompiledModule → Bool

/-- The modules of a program other than the one at index `i`. -/
def Program.others (P : Program) (i : Nat) : List CompiledModule :=
  (P.modules.toList.zipIdx.filter (fun e => e.2 != i)).map (·.1)

/-- A program accepted by `V` under `config`: module ids are distinct, and
every module passes local verification and link checks against the rest. -/
def WellFormedBy (V : VerifierInterface) (config : VerifierConfig) (P : Program) : Prop :=
  P.DistinctModuleIds ∧
  ∀ i m, P.modules[i]? = some m →
    V.verifyModule config m = true ∧ V.verifyLinking config m (P.others i) = true

end MoveBytecode
