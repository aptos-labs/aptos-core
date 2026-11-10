-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Properties.Structure

/-!
# Instruction consistency (L9)

TRUSTED. Property L9 of one module's text (plan §7, group L): every
instruction's operand is of the kind the instruction's form requires.

- **Generic and non-generic forms.** An instruction that names a function, a
  struct or an enum comes in two forms. The non-generic form names the entity
  directly and supplies no type arguments; the generic form names it through
  an instantiation table entry. The non-generic form must name an entity
  without type parameters. The generic form must name a generic entity, with
  one type argument per type parameter. From `file_format.rs`:
  instantiations "point to a generic handle and its instantiation"; `Call`
  runs its callee with type arguments only "if func.is_generic"; for
  `PackClosureGeneric`, "an uninstantiated generic function cannot be used to
  create a closure"; and a type parameter is an index into its entity's
  type-parameter list, so substitution needs an argument for each. A
  "partial" instantiation in `file_format.rs` is one whose arguments mention
  the enclosing function's type parameters; it still supplies every argument.
- **Struct and enum forms.** `Pack` and `Unpack` (and their generic forms)
  name a struct with declared fields: not an enum, whose values are built
  and taken apart by the variant forms, and not a native struct, which has no
  fields. That the variant and field forms name an existing variant or field
  is already part of L1, for every handle.
- **Signature operands.** The signature of a vector instruction is one type,
  the element type (`elem_ty_idx` in `file_format.rs`). The signature of
  `CallClosure` is one function type, "the given function type".

Function values (Phase 4, stage B) need no more here: `PackClosure` names a
non-generic function and `PackClosureGeneric` a generic one with all its type
arguments, as calls do, and `CallClosure` one function type. That a mask
names only parameters the function has, and captures none of reference type,
are checked by the semantics (`Step.packClosure`); the format's own wording,
"if `m` are the number of bits set in the mask, then `m <= n`", bounds only
their number, so they are not stated as instruction consistency.

Not yet stated:
- `VecPack` and `VecUnpack` counts. The language constrains a count only
  through the operand stack: `VecPack n` pops `n` values and `VecUnpack n`
  pushes `n`, and L4 bounds both. A bound on the count itself would be an
  acceptance limit (L11). Whether the vector has length `n` at run time is
  R2.
- Struct-API attributes (bytecode version 10: `pack`, `unpack`,
  `packVariant`, `unpackVariant`, `testVariant`, `borrowFieldImmutable`,
  `borrowFieldMutable`). `file_format.rs` documents only that each marks "the
  API" performing that operation on a non-private struct or enum. What makes
  such a function well-formed is not fixed by the format documentation or the
  Move book: its signature against the struct's fields, the struct it
  operates on, how a generic struct's type parameters are passed, and which
  variants an enum field borrow covers. These need struct types (Phase 2) and
  a statement agreed with the language owners. The plan expects the
  encapsulation part under D3a.
-/

namespace MoveBytecode

namespace CompiledModule

/-! ## Type-parameter counts of named entities

Each is `none` when a table entry on the way is missing; L1 rules that out. -/

/-- The number of type parameters of function handle `h`. -/
def functionTypeParamCount (m : CompiledModule) (h : FunctionHandleIndex) : Option Nat := do
  pure (← m.functionHandles[h]?).typeParameters.length

/-- The number of type parameters, phantom ones included, of the struct or
enum that struct definition `d` defines. -/
def structTypeParamCount (m : CompiledModule) (d : StructDefinitionIndex) : Option Nat := do
  let sd ← m.structDefs[d]?
  pure (← m.structHandles[sd.structHandle]?).typeParameters.length

/-- The number of type parameters of the struct that owns field handle `h`. -/
def fieldTypeParamCount (m : CompiledModule) (h : FieldHandleIndex) : Option Nat := do
  m.structTypeParamCount (← m.fieldHandles[h]?).owner

/-- The number of type parameters of the enum of variant handle `h`. -/
def variantTypeParamCount (m : CompiledModule) (h : StructVariantHandleIndex) : Option Nat := do
  m.structTypeParamCount (← m.structVariantHandles[h]?).structIndex

/-- The number of type parameters of the enum of variant field handle `h`. -/
def variantFieldTypeParamCount (m : CompiledModule) (h : VariantFieldHandleIndex) :
    Option Nat := do
  m.structTypeParamCount (← m.variantFieldHandles[h]?).structIndex

/-! ## Operand kinds -/

/-- The operand of a generic form: an entity with `count` type parameters, at
least one, instantiated by signature `typeArgs` with one type argument per
type parameter. -/
def GenericInstance (m : CompiledModule) (count : Option Nat) (typeArgs : SignatureIndex) :
    Prop :=
  ∃ n args, count = some n ∧ 0 < n ∧ m.signatures[typeArgs]? = some args ∧ args.length = n

/-- Struct definition `d` defines a struct with declared fields: neither an
enum nor a native struct. -/
def DeclaredStruct (m : CompiledModule) (d : StructDefinitionIndex) : Prop :=
  (m.declaredFieldCount d).isSome

/-- Signature `s` is a single type. -/
def SingleType (m : CompiledModule) (s : SignatureIndex) : Prop :=
  ∃ t, m.signatures[s]? = some [t]

/-- Signature `s` is a single function type. -/
def SingleFunctionType (m : CompiledModule) (s : SignatureIndex) : Prop :=
  ∃ params results abilities, m.signatures[s]? = some [.function params results abilities]

end CompiledModule

/-- **L9** for one instruction of module `m`: its operand is of the kind its
form requires. Instructions that name no function, struct, enum, element
type or closure type require nothing. -/
def Bytecode.OperandsConsistent (m : CompiledModule) : Bytecode → Prop
  -- Functions: direct calls and closure creation.
  | .call h | .packClosure h _ => m.functionTypeParamCount h = some 0
  | .callGeneric i | .packClosureGeneric i _ =>
    ∃ x, m.functionInstantiations[i]? = some x ∧
      m.GenericInstance (m.functionTypeParamCount x.handle) x.typeParameters
  -- Structs: construction and destruction.
  | .pack d | .unpack d => m.structTypeParamCount d = some 0 ∧ m.DeclaredStruct d
  | .packGeneric i | .unpackGeneric i =>
    ∃ x, m.structDefInstantiations[i]? = some x ∧
      m.GenericInstance (m.structTypeParamCount x.def_) x.typeParameters ∧
      m.DeclaredStruct x.def_
  -- Global storage.
  | .mutBorrowGlobal d | .immBorrowGlobal d | .exists_ d | .moveFrom d | .moveTo d =>
    m.structTypeParamCount d = some 0
  | .mutBorrowGlobalGeneric i | .immBorrowGlobalGeneric i | .existsGeneric i
  | .moveFromGeneric i | .moveToGeneric i =>
    ∃ x, m.structDefInstantiations[i]? = some x ∧
      m.GenericInstance (m.structTypeParamCount x.def_) x.typeParameters
  -- Struct fields.
  | .mutBorrowField h | .immBorrowField h => m.fieldTypeParamCount h = some 0
  | .mutBorrowFieldGeneric i | .immBorrowFieldGeneric i =>
    ∃ x, m.fieldInstantiations[i]? = some x ∧
      m.GenericInstance (m.fieldTypeParamCount x.handle) x.typeParameters
  -- Enum variants and their fields.
  | .packVariant h | .unpackVariant h | .testVariant h => m.variantTypeParamCount h = some 0
  | .packVariantGeneric i | .unpackVariantGeneric i | .testVariantGeneric i =>
    ∃ x, m.structVariantInstantiations[i]? = some x ∧
      m.GenericInstance (m.variantTypeParamCount x.handle) x.typeParameters
  | .mutBorrowVariantField h | .immBorrowVariantField h =>
    m.variantFieldTypeParamCount h = some 0
  | .mutBorrowVariantFieldGeneric i | .immBorrowVariantFieldGeneric i =>
    ∃ x, m.variantFieldInstantiations[i]? = some x ∧
      m.GenericInstance (m.variantFieldTypeParamCount x.handle) x.typeParameters
  -- Vectors: the signature is the element type. Counts: see the module docstring.
  | .vecPack s _ | .vecLen s | .vecImmBorrow s | .vecMutBorrow s | .vecPushBack s
  | .vecPopBack s | .vecUnpack s _ | .vecSwap s => m.SingleType s
  -- Closure calls: the signature is the closure's function type.
  | .callClosure s => m.SingleFunctionType s
  -- No named entity or type operand.
  | .pop | .ret | .brTrue _ | .brFalse _ | .branch _ => True
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => True
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => True
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => True
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => True
  | .ldConst _ | .ldTrue | .ldFalse | .copyLoc _ | .moveLoc _ | .stLoc _ => True
  | .readRef | .writeRef | .freezeRef | .mutBorrowLoc _ | .immBorrowLoc _ => True
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => True
  | .eq | .neq | .lt | .gt | .le | .ge | .shl | .shr | .negate => True
  | .abort | .abortMsg | .nop => True

/-- **L9.** Every instruction of every function of the module, reachable or
not, has operands of the kind its form requires. -/
def CompiledModule.InstructionConsistency (m : CompiledModule) : Prop :=
  ∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code →
    ∀ i ∈ code.code.toList, i.OperandsConsistent m

end MoveBytecode
