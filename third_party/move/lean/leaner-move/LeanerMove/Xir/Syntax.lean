-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# Deployable XIR modules

The wire representation compiler-v2 loads as stackless bytecode
(`move-model-exchange`'s `XirModule`, schema `move-xir-module`). Every
declaration is addressed positionally: struct ids index the module's structs
and then its external structs, function ids its functions and then its
external functions, and locals start with the parameters.
-/

namespace LeanerIR.Move.Xir

/-- The XIR module wrapper version this backend writes. -/
def version : Nat := 8

/-- A Move integer type, as width-annotated operations carry it. -/
inductive IntType where
  | u8 | u16 | u32 | u64 | u128 | u256
  | i8 | i16 | i32 | i64 | i128 | i256
  deriving Repr, BEq, Inhabited

/-- A Move type. Struct ids address local structs, then external ones. -/
inductive Ty where
  | bool
  | int (type : IntType)
  | address
  | signer
  | typeParameter (index : Nat)
  | struct (id : Nat) (arguments : Array Ty)
  | enum (id : Nat) (arguments : Array Ty)
  | vector (element : Ty)
  | ref (referent : Ty)
  | mutRef (referent : Ty)
  | function (parameters results : Array Ty) (abilities : Array String)
  deriving Repr, BEq, Inhabited

/-- A declaration-scoped type parameter. -/
structure TypeParameter where
  name : String
  abilities : Array String := #[]
  phantom : Bool := false
  deriving Repr, BEq, Inhabited

structure Field where
  name : String
  ty : Ty
  deriving Repr, BEq, Inhabited

structure Variant where
  name : String
  fields : Array Field
  deriving Repr, BEq, Inhabited

/-- An argument of a source attribute: a name applied to arguments, a `u64`
constant, or a boolean. -/
inductive AttributeArg where
  | name (name : String) (args : Array AttributeArg)
  | num (value : Nat)
  | bool (value : Bool)
  deriving Repr, BEq, Inhabited

/-- A source attribute of a struct or function, such as `module_lock`, as the
bytecode carries it. -/
structure Attribute where
  name : String
  args : Array AttributeArg := #[]
  deriving Repr, BEq, Inhabited

/-- A struct, or an enum when `variants` is present. -/
structure Struct where
  name : String
  abilities : Array String := #[]
  typeParameters : Array TypeParameter := #[]
  fields : Array Field := #[]
  variants : Option (Array Variant) := none
  attributes : Array Attribute := #[]
  deriving Repr, BEq, Inhabited

/-- A constant a `load` writes. -/
inductive Value where
  | bool (value : Bool)
  | num (value : Nat)
  | address (value : String)
  | vector (elements : Array Value)
  deriving Repr, BEq, Inhabited

/-- The operation of a `call` instruction. Instantiated forms carry the type
arguments of a generic struct, enum, or function. -/
inductive Oper where
  | add (type : IntType) | sub (type : IntType) | mul (type : IntType)
  | div (type : IntType) | mod (type : IntType)
  | bitAnd (type : IntType) | bitOr (type : IntType) | bitXor (type : IntType)
  | shl (type : IntType) | shr (type : IntType) | cast (type : IntType)
  | lt | le | eq | and | or | not
  | pack (arguments : Array Ty)
  | unpack (arguments : Array Ty)
  | packVariant (variant : Nat) (arguments : Array Ty)
  | unpackVariant (variant : Nat) (arguments : Array Ty)
  | testVariant (variant : Nat) (arguments : Array Ty)
  | getField (field : Nat) (arguments : Array Ty)
  | vecPack | vecLen | vecGet | vecSet | vecPush | vecPop | vecInsert | vecRemove | vecSwap
  | moveTo (struct : Nat) (arguments : Array Ty)
  | moveFrom (struct : Nat) (arguments : Array Ty)
  | «exists» (struct : Nat) (arguments : Array Ty)
  | function (function : Nat) (arguments : Array Ty)
  /-- A function value of `function` over the captured operands; bit `i` of
  `mask` marks parameter `i` as captured. -/
  | closure (function : Nat) (mask : Nat) (arguments : Array Ty)
  /-- Call the function value in the last operand with the preceding ones. -/
  | invoke
  | borrowLoc
  | borrowField (field : Nat) (arguments : Array Ty)
  | borrowGlobal (struct : Nat) (arguments : Array Ty)
  | borrowVecElem
  | borrowVariantField (variants : Array Nat) (field : Nat) (arguments : Array Ty)
  | testVariantRef (variant : Nat) (arguments : Array Ty)
  | readRef | writeRef | freezeRef
  deriving Repr, BEq, Inhabited

inductive Instr where
  | load (destination : Nat) (value : Value)
  | assign (destination source : Nat)
  | call (destinations : Array Nat) (operation : Oper) (sources : Array Nat)
  deriving Repr, BEq, Inhabited

inductive Term where
  | jump (block : Nat)
  | branch (condition thenBlock elseBlock : Nat)
  | ret (sources : Array Nat)
  | abort (code : Nat)
  deriving Repr, BEq, Inhabited

/-- A half-open UTF-8 byte range of the source. -/
structure Span where
  start : Nat
  stop : Nat
  deriving Repr, BEq, Inhabited

structure Block where
  instrs : Array Instr := #[]
  term : Term
  /-- Source spans aligned with `instrs`, and the terminator's. -/
  instrSpans : Array (Option Span) := #[]
  termSpan : Option Span := none
  deriving Repr, BEq, Inhabited

inductive Visibility where
  | private_ | public_ | friend
  deriving Repr, BEq, Inhabited

structure Function where
  name : String
  typeParameters : Array TypeParameter := #[]
  visibility : Visibility
  isEntry : Bool := false
  isNative : Bool := false
  acquires : Array Nat := #[]
  params : Nat
  locals : Array Ty
  localNames : Array (Option String) := #[]
  returns : Array Ty
  blocks : Array Block
  span : Option Span := none
  attributes : Array Attribute := #[]
  deriving Repr, BEq, Inhabited

/-- A declaration of another module this module refers to. -/
structure External where
  address : String
  module : String
  name : String
  deriving Repr, BEq, Inhabited

structure Module where
  address : String
  name : String
  structs : Array Struct
  functions : Array Function
  externalFunctions : Array External := #[]
  externalStructs : Array External := #[]
  deriving Repr, BEq, Inhabited

end LeanerIR.Move.Xir
