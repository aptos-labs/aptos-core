-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# Abstract syntax of Move bytecode

TRUSTED. The abstract syntax of `CompiledModule` and `CompiledScript`, taken
from `third_party/move/move-binary-format/src/file_format.rs`. The syntax
describes the object being checked; it contains no judgment about it, so it
may follow the Rust definitions closely (plan §5.1).

The binary encoding is not modeled. Modules enter Lean through the exporter
and `Import.Json`. Table indices are `Nat`; their `u16` bounds are a property
of the binary format and are not relied upon.
-/

namespace MoveBytecode

abbrev ModuleHandleIndex := Nat
abbrev StructHandleIndex := Nat
abbrev FunctionHandleIndex := Nat
abbrev FieldHandleIndex := Nat
abbrev VariantFieldHandleIndex := Nat
abbrev StructVariantHandleIndex := Nat
abbrev StructDefInstantiationIndex := Nat
abbrev StructVariantInstantiationIndex := Nat
abbrev FunctionInstantiationIndex := Nat
abbrev FieldInstantiationIndex := Nat
abbrev VariantFieldInstantiationIndex := Nat
abbrev IdentifierIndex := Nat
abbrev AddressIdentifierIndex := Nat
abbrev ConstantPoolIndex := Nat
abbrev SignatureIndex := Nat
abbrev StructDefinitionIndex := Nat
abbrev FunctionDefinitionIndex := Nat
abbrev LocalIndex := Nat
abbrev CodeOffset := Nat
abbrev MemberCount := Nat
abbrev VariantIndex := Nat
abbrev TypeParameterIndex := Nat

/-- A 32-byte account address. -/
abbrev Address := BitVec 256

/-- The four abilities, kept as a record of flags so that sets compare by
value. The binary format admits no other bits. -/
structure AbilitySet where
  copy : Bool := false
  drop : Bool := false
  store : Bool := false
  key : Bool := false
  deriving DecidableEq, Repr, Inhabited, Hashable

namespace AbilitySet

def empty : AbilitySet := {}

def all : AbilitySet := ⟨true, true, true, true⟩

/-- The abilities of the primitive types: `copy`, `drop` and `store`. -/
def primitives : AbilitySet := ⟨true, true, true, false⟩

def subset (a b : AbilitySet) : Bool :=
  (!a.copy || b.copy) && (!a.drop || b.drop) && (!a.store || b.store) && (!a.key || b.key)

def inter (a b : AbilitySet) : AbilitySet :=
  ⟨a.copy && b.copy, a.drop && b.drop, a.store && b.store, a.key && b.key⟩

def union (a b : AbilitySet) : AbilitySet :=
  ⟨a.copy || b.copy, a.drop || b.drop, a.store || b.store, a.key || b.key⟩

end AbilitySet

/-- A type token as it appears in signatures. -/
inductive SignatureToken where
  | bool
  | u8
  | u16
  | u32
  | u64
  | u128
  | u256
  | i8
  | i16
  | i32
  | i64
  | i128
  | i256
  | address
  | signer
  | vector (elem : SignatureToken)
  | function (params results : List SignatureToken) (abilities : AbilitySet)
  | struct (handle : StructHandleIndex)
  | structInstantiation (handle : StructHandleIndex) (typeArgs : List SignatureToken)
  | reference (inner : SignatureToken)
  | mutableReference (inner : SignatureToken)
  | typeParameter (index : TypeParameterIndex)
  deriving Repr, Inhabited, Hashable

/-! Structural equality of signature tokens. `DecidableEq` cannot be derived
for a nested inductive, and a derived `BEq` is an opaque `partial` function
about which nothing can be proved, so equality is defined by structural
recursion and shown lawful. -/

mutual
def SignatureToken.beq : SignatureToken → SignatureToken → Bool
  | .bool, .bool | .u8, .u8 | .u16, .u16 | .u32, .u32 | .u64, .u64 | .u128, .u128
  | .u256, .u256 | .i8, .i8 | .i16, .i16 | .i32, .i32 | .i64, .i64 | .i128, .i128
  | .i256, .i256 | .address, .address | .signer, .signer => true
  | .vector a, .vector b => SignatureToken.beq a b
  | .function p r ab, .function p' r' ab' =>
    SignatureToken.beqList p p' && SignatureToken.beqList r r' && decide (ab = ab')
  | .struct h, .struct h' => decide (h = h')
  | .structInstantiation h as, .structInstantiation h' as' =>
    decide (h = h') && SignatureToken.beqList as as'
  | .reference a, .reference b => SignatureToken.beq a b
  | .mutableReference a, .mutableReference b => SignatureToken.beq a b
  | .typeParameter i, .typeParameter j => decide (i = j)
  | _, _ => false

def SignatureToken.beqList : List SignatureToken → List SignatureToken → Bool
  | [], [] => true
  | a :: as, b :: bs => SignatureToken.beq a b && SignatureToken.beqList as bs
  | _, _ => false
end

theorem SignatureToken.beq_iff (a b : SignatureToken) : SignatureToken.beq a b = true ↔ a = b := by
  induction a using SignatureToken.rec
    (motive_2 := fun as => ∀ bs, SignatureToken.beqList as bs = true ↔ as = bs) generalizing b with
  | vector a ih => cases b <;> simp [SignatureToken.beq, ih]
  | function p r ab ihp ihr => cases b <;> simp [SignatureToken.beq, ihp, ihr, and_assoc]
  | structInstantiation h as ih => cases b <;> simp [SignatureToken.beq, ih]
  | reference a ih => cases b <;> simp [SignatureToken.beq, ih]
  | mutableReference a ih => cases b <;> simp [SignatureToken.beq, ih]
  | nil => rename_i bs; cases bs <;> simp [SignatureToken.beqList]
  | cons x xs ihx ihxs => rename_i bs; cases bs <;> simp [SignatureToken.beqList, ihx, ihxs]
  | _ => cases b <;> simp [SignatureToken.beq]

instance : DecidableEq SignatureToken := fun a b =>
  decidable_of_iff _ (SignatureToken.beq_iff a b)

abbrev Signature := List SignatureToken

structure ModuleHandle where
  address : AddressIdentifierIndex
  name : IdentifierIndex
  deriving DecidableEq, Repr, Inhabited

structure StructTypeParameter where
  constraints : AbilitySet
  isPhantom : Bool
  deriving DecidableEq, Repr, Inhabited

structure StructHandle where
  module : ModuleHandleIndex
  name : IdentifierIndex
  abilities : AbilitySet
  typeParameters : List StructTypeParameter
  deriving DecidableEq, Repr, Inhabited

inductive FunctionAttribute where
  | persistent
  | moduleLock
  | pack
  | packVariant (variant : VariantIndex)
  | unpack
  | unpackVariant (variant : VariantIndex)
  | testVariant (variant : VariantIndex)
  | borrowFieldImmutable (offset : MemberCount)
  | borrowFieldMutable (offset : MemberCount)
  deriving DecidableEq, Repr, Inhabited

/-- The struct-API attributes: each marks a function as the API performing
one operation on a non-private struct or enum (`file_format.rs`). The
production deserializer accepts them only from bytecode version 10
(`load_attribute` in `move-binary-format/src/deserializer.rs`); `persistent`
and `moduleLock` are accepted from version 8, where attributes enter the
format. -/
def FunctionAttribute.isStructApi : FunctionAttribute → Bool
  | .pack | .packVariant _ | .unpack | .unpackVariant _ | .testVariant _
  | .borrowFieldImmutable _ | .borrowFieldMutable _ => true
  | .persistent | .moduleLock => false

inductive AccessKind where
  | reads
  | writes
  deriving DecidableEq, Repr, Inhabited

inductive ResourceSpecifier where
  | any
  | declaredAtAddress (address : AddressIdentifierIndex)
  | declaredInModule (module : ModuleHandleIndex)
  | resource (handle : StructHandleIndex)
  | resourceInstantiation (handle : StructHandleIndex) (typeArgs : SignatureIndex)
  deriving DecidableEq, Repr, Inhabited

inductive AddressSpecifier where
  | any
  | literal (address : AddressIdentifierIndex)
  | parameter (local_ : LocalIndex) (function : Option FunctionInstantiationIndex)
  deriving DecidableEq, Repr, Inhabited

/-- Access specifiers are part of the format, but the production verifier
rejects them (plan §4.4). -/
structure AccessSpecifier where
  kind : AccessKind
  negated : Bool
  resource : ResourceSpecifier
  address : AddressSpecifier
  deriving DecidableEq, Repr, Inhabited

structure FunctionHandle where
  module : ModuleHandleIndex
  name : IdentifierIndex
  parameters : SignatureIndex
  returns : SignatureIndex
  typeParameters : List AbilitySet
  accessSpecifiers : Option (List AccessSpecifier)
  attributes : List FunctionAttribute
  deriving DecidableEq, Repr, Inhabited

structure FieldHandle where
  owner : StructDefinitionIndex
  field : MemberCount
  deriving DecidableEq, Repr, Inhabited

structure VariantFieldHandle where
  structIndex : StructDefinitionIndex
  variants : List VariantIndex
  field : MemberCount
  deriving DecidableEq, Repr, Inhabited

structure StructVariantHandle where
  structIndex : StructDefinitionIndex
  variant : VariantIndex
  deriving DecidableEq, Repr, Inhabited

structure FieldDefinition where
  name : IdentifierIndex
  signature : SignatureToken
  deriving BEq, Repr, Inhabited

structure VariantDefinition where
  name : IdentifierIndex
  fields : List FieldDefinition
  deriving BEq, Repr, Inhabited

inductive StructFieldInformation where
  | native
  | declared (fields : List FieldDefinition)
  | declaredVariants (variants : List VariantDefinition)
  deriving BEq, Repr, Inhabited

structure StructDefinition where
  structHandle : StructHandleIndex
  fieldInformation : StructFieldInformation
  deriving BEq, Repr, Inhabited

structure StructDefInstantiation where
  def_ : StructDefinitionIndex
  typeParameters : SignatureIndex
  deriving DecidableEq, Repr, Inhabited

structure StructVariantInstantiation where
  handle : StructVariantHandleIndex
  typeParameters : SignatureIndex
  deriving DecidableEq, Repr, Inhabited

structure FunctionInstantiation where
  handle : FunctionHandleIndex
  typeParameters : SignatureIndex
  deriving DecidableEq, Repr, Inhabited

structure FieldInstantiation where
  handle : FieldHandleIndex
  typeParameters : SignatureIndex
  deriving DecidableEq, Repr, Inhabited

structure VariantFieldInstantiation where
  handle : VariantFieldHandleIndex
  typeParameters : SignatureIndex
  deriving DecidableEq, Repr, Inhabited

/-- Function visibility. `private_` and `public_` carry a trailing underscore
because `private` and `public` are Lean keywords. -/
inductive Visibility where
  | private_
  | public_
  | friend
  deriving DecidableEq, Repr, Inhabited

/-- The instruction set of `file_format::Bytecode`, in the Rust declaration
order. Literal payloads are bit vectors of their exact width; signed literals
are two's complement. -/
inductive Bytecode where
  | pop
  | ret
  | brTrue (offset : CodeOffset)
  | brFalse (offset : CodeOffset)
  | branch (offset : CodeOffset)
  | ldU8 (value : BitVec 8)
  | ldU64 (value : BitVec 64)
  | ldU128 (value : BitVec 128)
  | castU8
  | castU64
  | castU128
  | ldConst (index : ConstantPoolIndex)
  | ldTrue
  | ldFalse
  | copyLoc (index : LocalIndex)
  | moveLoc (index : LocalIndex)
  | stLoc (index : LocalIndex)
  | call (handle : FunctionHandleIndex)
  | callGeneric (inst : FunctionInstantiationIndex)
  | pack (def_ : StructDefinitionIndex)
  | packGeneric (inst : StructDefInstantiationIndex)
  | packVariant (handle : StructVariantHandleIndex)
  | packVariantGeneric (inst : StructVariantInstantiationIndex)
  | unpack (def_ : StructDefinitionIndex)
  | unpackGeneric (inst : StructDefInstantiationIndex)
  | unpackVariant (handle : StructVariantHandleIndex)
  | unpackVariantGeneric (inst : StructVariantInstantiationIndex)
  | testVariant (handle : StructVariantHandleIndex)
  | testVariantGeneric (inst : StructVariantInstantiationIndex)
  | readRef
  | writeRef
  | freezeRef
  | mutBorrowLoc (index : LocalIndex)
  | immBorrowLoc (index : LocalIndex)
  | mutBorrowField (handle : FieldHandleIndex)
  | mutBorrowVariantField (handle : VariantFieldHandleIndex)
  | mutBorrowFieldGeneric (inst : FieldInstantiationIndex)
  | mutBorrowVariantFieldGeneric (inst : VariantFieldInstantiationIndex)
  | immBorrowField (handle : FieldHandleIndex)
  | immBorrowVariantField (handle : VariantFieldHandleIndex)
  | immBorrowFieldGeneric (inst : FieldInstantiationIndex)
  | immBorrowVariantFieldGeneric (inst : VariantFieldInstantiationIndex)
  | mutBorrowGlobal (def_ : StructDefinitionIndex)
  | mutBorrowGlobalGeneric (inst : StructDefInstantiationIndex)
  | immBorrowGlobal (def_ : StructDefinitionIndex)
  | immBorrowGlobalGeneric (inst : StructDefInstantiationIndex)
  | add
  | sub
  | mul
  | mod
  | div
  | bitOr
  | bitAnd
  | xor
  | or
  | and
  | not
  | eq
  | neq
  | lt
  | gt
  | le
  | ge
  | abort
  | nop
  | exists_ (def_ : StructDefinitionIndex)
  | existsGeneric (inst : StructDefInstantiationIndex)
  | moveFrom (def_ : StructDefinitionIndex)
  | moveFromGeneric (inst : StructDefInstantiationIndex)
  | moveTo (def_ : StructDefinitionIndex)
  | moveToGeneric (inst : StructDefInstantiationIndex)
  | shl
  | shr
  | vecPack (elem : SignatureIndex) (count : Nat)
  | vecLen (elem : SignatureIndex)
  | vecImmBorrow (elem : SignatureIndex)
  | vecMutBorrow (elem : SignatureIndex)
  | vecPushBack (elem : SignatureIndex)
  | vecPopBack (elem : SignatureIndex)
  | vecUnpack (elem : SignatureIndex) (count : Nat)
  | vecSwap (elem : SignatureIndex)
  | packClosure (handle : FunctionHandleIndex) (mask : BitVec 64)
  | packClosureGeneric (inst : FunctionInstantiationIndex) (mask : BitVec 64)
  | callClosure (signature : SignatureIndex)
  | ldU16 (value : BitVec 16)
  | ldU32 (value : BitVec 32)
  | ldU256 (value : BitVec 256)
  | castU16
  | castU32
  | castU256
  | ldI8 (value : BitVec 8)
  | ldI16 (value : BitVec 16)
  | ldI32 (value : BitVec 32)
  | ldI64 (value : BitVec 64)
  | ldI128 (value : BitVec 128)
  | ldI256 (value : BitVec 256)
  | castI8
  | castI16
  | castI32
  | castI64
  | castI128
  | castI256
  | negate
  | abortMsg
  deriving DecidableEq, Repr, Inhabited

structure CodeUnit where
  /-- The non-parameter locals; a function's locals are its parameters
  followed by these. -/
  locals : SignatureIndex
  code : Array Bytecode
  deriving DecidableEq, Repr, Inhabited

structure FunctionDefinition where
  function : FunctionHandleIndex
  visibility : Visibility
  isEntry : Bool
  acquiresGlobalResources : List StructDefinitionIndex
  /-- `none` for a native function. -/
  code : Option CodeUnit
  deriving DecidableEq, Repr, Inhabited

/-- A constant: its type and its BCS-encoded value. -/
structure Constant where
  type : SignatureToken
  data : List UInt8
  deriving BEq, Repr, Inhabited

structure Metadata where
  key : List UInt8
  value : List UInt8
  deriving DecidableEq, Repr, Inhabited

structure CompiledModule where
  version : Nat
  selfModuleHandleIdx : ModuleHandleIndex
  moduleHandles : Array ModuleHandle
  structHandles : Array StructHandle
  functionHandles : Array FunctionHandle
  fieldHandles : Array FieldHandle
  friendDecls : Array ModuleHandle
  structDefInstantiations : Array StructDefInstantiation
  functionInstantiations : Array FunctionInstantiation
  fieldInstantiations : Array FieldInstantiation
  signatures : Array Signature
  identifiers : Array String
  addressIdentifiers : Array Address
  constantPool : Array Constant
  metadata : Array Metadata
  structDefs : Array StructDefinition
  functionDefs : Array FunctionDefinition
  structVariantHandles : Array StructVariantHandle
  structVariantInstantiations : Array StructVariantInstantiation
  variantFieldHandles : Array VariantFieldHandle
  variantFieldInstantiations : Array VariantFieldInstantiation
  deriving BEq, Repr, Inhabited

structure CompiledScript where
  version : Nat
  moduleHandles : Array ModuleHandle
  structHandles : Array StructHandle
  functionHandles : Array FunctionHandle
  functionInstantiations : Array FunctionInstantiation
  signatures : Array Signature
  identifiers : Array String
  addressIdentifiers : Array Address
  constantPool : Array Constant
  metadata : Array Metadata
  code : CodeUnit
  typeParameters : List AbilitySet
  parameters : SignatureIndex
  accessSpecifiers : Option (List AccessSpecifier)
  deriving BEq, Repr, Inhabited

end MoveBytecode
