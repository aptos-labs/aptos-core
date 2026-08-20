-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import MoveModel.IR.Module

/-!
# Leaner named compiler IR

This is the stable boundary between Lean's compiler representation and the
positional identifiers used by `MoveModel.IR`.  It deliberately keeps declaration,
local, block, resource, and function names until the final erasure pass.
-/

namespace Move.Compiler.LIR

open Lean

abbrev AbilitySet := MoveModel.IR.AbilitySet
abbrev TypeParamDecl := MoveModel.IR.TypeParamDecl

inductive Ty where
  | bool | u64 | address | signer
  | typeParam (index : Nat)
  | struct (name : Name)
  | structInst (name : Name) (args : Array Ty)
  | enum (name : Name)
  | enumInst (name : Name) (args : Array Ty)
  | vector (elem : Ty)
  | ref (elem : Ty)
  | mutRef (elem : Ty)
  deriving BEq, Repr

partial def Ty.instantiate (args : Array Ty) : Ty → Ty
  | .typeParam index => args[index]?.getD (.typeParam index)
  | .structInst name inner => .structInst name (inner.map (·.instantiate args))
  | .enumInst name inner => .enumInst name (inner.map (·.instantiate args))
  | .vector elem => .vector (elem.instantiate args)
  | .ref elem => .ref (elem.instantiate args)
  | .mutRef elem => .mutRef (elem.instantiate args)
  | ty => ty

inductive Visibility where
  | private_ | public_ | friend_ | entry
  deriving BEq, Repr

structure LocalDecl where
  name : String
  ty : Ty
  deriving BEq, Repr

inductive Oper where
  | add | sub | mul | div | mod | lt | le | eq
  | vecPack | vecLen | vecGet | vecSet | vecPush | vecPop
  | vecInsert | vecRemove
  | pack (structName : Name) (typeArgs : Array Ty)
  | unpack (structName : Name) (typeArgs : Array Ty)
  | packVariant (enumName : Name) (variant : Nat) (typeArgs : Array Ty)
  | unpackVariant (enumName : Name) (variant : Nat) (typeArgs : Array Ty)
  | testVariant (enumName : Name) (variant : Nat) (typeArgs : Array Ty)
  | getField (structName : Name) (field : Nat) (typeArgs : Array Ty)
  | borrowLoc
  | borrowGlobal (resource : Name) (typeArgs : Array Ty)
  | borrowField (field : Nat) (typeArgs : Array Ty)
  | borrowVecElem
  | readRef | writeRef | freezeRef
  | exists_ (resource : Name) (typeArgs : Array Ty)
  | moveFrom (resource : Name) (typeArgs : Array Ty)
  | moveTo (resource : Name) (typeArgs : Array Ty)
  | function (name : Name) (typeArgs : Array Ty)
  deriving BEq, Repr

inductive Instr where
  | loadBool (dst : String) (value : Bool)
  | loadU64 (dst : String) (value : Nat)
  | assign (dst src : String)
  | call (dsts : Array String) (op : Oper) (srcs : Array String)
  deriving BEq, Repr

inductive Terminator where
  | jump (block : String)
  | branch (cond thenBlock elseBlock : String)
  | ret (srcs : Array String)
  | abort (code : String)
  deriving BEq, Repr

structure Block where
  name : String
  instrs : Array Instr
  term : Terminator
  deriving BEq, Repr

structure FieldDecl where
  leanName : Name
  moveName : String
  ty : Ty
  deriving BEq, Repr

structure VariantDecl where
  leanName : Name
  moveName : String
  fields : Array FieldDecl
  deriving BEq, Repr

structure StructDecl where
  leanName : Name
  moveName : String
  typeParams : Array TypeParamDecl := #[]
  abilities : AbilitySet
  fields : Array FieldDecl
  variants : Option (Array VariantDecl) := none
  deriving BEq, Repr

structure FunDecl where
  leanName : Name
  moveName : String
  typeParams : Array TypeParamDecl := #[]
  visibility : Visibility
  params : Array LocalDecl
  returns : Array Ty
  locals : Array LocalDecl
  blocks : Array Block
  calls : Array Name
  acquires : Array Name
  deriving BEq, Repr

/-- A callable function owned by another Move module. `leanName` resolves the
source call while the remaining fields are the stable bytecode identity. -/
structure ExternalFunRef where
  leanName : Name
  address : String
  moduleName : String
  functionName : String
  deriving BEq, Repr

structure Module where
  address : String := "0x0"
  name : String
  structs : Array StructDecl
  functions : Array FunDecl
  externalFuns : Array ExternalFunRef := #[]
  deriving BEq, Repr

private def lookup (kind name : String) (names : Array String) : Except String Nat := do
  let i := names.findIdx (· == name)
  if i < names.size then return i
  throw s!"unknown {kind} `{name}`"

private def ensureUnique (kind : String) (names : Array String) : Except String Unit := do
  let mut seen : Array String := #[]
  for name in names do
    if seen.any (· == name) then throw s!"duplicate {kind} `{name}`"
    seen := seen.push name

private def hexDigit? : Char → Option Nat
  | '0' => some 0 | '1' => some 1 | '2' => some 2 | '3' => some 3
  | '4' => some 4 | '5' => some 5 | '6' => some 6 | '7' => some 7
  | '8' => some 8 | '9' => some 9
  | 'a' => some 10 | 'A' => some 10
  | 'b' => some 11 | 'B' => some 11
  | 'c' => some 12 | 'C' => some 12
  | 'd' => some 13 | 'D' => some 13
  | 'e' => some 14 | 'E' => some 14
  | 'f' => some 15 | 'F' => some 15
  | _ => none

private def parseAddress (address : String) : Except String MoveModel.IR.Address := do
  unless address.startsWith "0x" do
    throw s!"module address `{address}` must start with `0x`"
  let digits := (address.drop 2).toString
  if digits.isEmpty then throw "module address has no hexadecimal digits"
  let mut value := 0
  for digit in digits.toList do
    let some n := hexDigit? digit
      | throw s!"module address `{address}` contains a non-hexadecimal digit"
    value := value * 16 + n
  if value < 2 ^ 256 then return value
  throw s!"module address `{address}` does not fit in 256 bits"

private partial def lowerTy (structNames : Array (Name × String)) :
    Move.Compiler.LIR.Ty → Except String MoveModel.IR.Ty
  | .bool => pure .bool
  | .u64 => pure .u64
  | .address => pure .address
  | .signer => pure .signer
  | .typeParam index => pure (.typeParam index)
  | .struct name => do
      let i := structNames.findIdx (·.1 == name)
      if i < structNames.size then return .struct i
      throw s!"unknown structure `{name}`"
  | .structInst name args => do
      let i := structNames.findIdx (·.1 == name)
      if i < structNames.size then
        return .structInst i (← args.toList.mapM (lowerTy structNames))
      throw s!"unknown structure `{name}`"
  | .enum name => do
      let i := structNames.findIdx (·.1 == name)
      if i < structNames.size then return .enum i
      throw s!"unknown enum `{name}`"
  | .enumInst name args => do
      let i := structNames.findIdx (·.1 == name)
      if i < structNames.size then
        return .enumInst i (← args.toList.mapM (lowerTy structNames))
      throw s!"unknown enum `{name}`"
  | .vector elem => return .vector (← lowerTy structNames elem)
  | .ref elem => return .ref (← lowerTy structNames elem)
  | .mutRef elem => return .mutRef (← lowerTy structNames elem)

private def lowerOper (structNames : Array (Name × String))
    (funNames : Array (Name × String)) (externalFunNames : Array Name) :
    Move.Compiler.LIR.Oper → Except String MoveModel.IR.Oper
  | .add => pure .add | .sub => pure .sub | .mul => pure .mul
  | .div => pure .div | .mod => pure .mod | .lt => pure .lt
  | .le => pure .le | .eq => pure .eq
  | .vecPack => pure .vecPack | .vecLen => pure .vecLen
  | .vecGet => pure .vecGet | .vecSet => pure .vecSet
  | .vecPush => pure .vecPush | .vecPop => pure .vecPop
  | .vecInsert => pure .vecInsert | .vecRemove => pure .vecRemove
  | .pack _ args =>
      if args.isEmpty then pure .pack
      else return .packInst (← args.toList.mapM (lowerTy structNames))
  | .unpack _ args =>
      if args.isEmpty then pure .unpack
      else return .unpackInst (← args.toList.mapM (lowerTy structNames))
  | .packVariant _ variant args =>
      if args.isEmpty then pure (.packVariant variant)
      else return .packVariantInst variant (← args.toList.mapM (lowerTy structNames))
  | .unpackVariant _ variant args =>
      if args.isEmpty then pure (.unpackVariant variant)
      else return .unpackVariantInst variant (← args.toList.mapM (lowerTy structNames))
  | .testVariant _ variant args =>
      if args.isEmpty then pure (.testVariant variant)
      else return .testVariantInst variant (← args.toList.mapM (lowerTy structNames))
  | .getField _ field args =>
      if args.isEmpty then pure (.getField field)
      else return .getFieldInst field (← args.toList.mapM (lowerTy structNames))
  | .borrowGlobal name args => do
      let i := structNames.findIdx (·.1 == name)
      if i < structNames.size then
        if args.isEmpty then return .borrowGlobal i
        else return .borrowGlobalInst i (← args.toList.mapM (lowerTy structNames))
      throw s!"unknown resource `{name}`"
  | .borrowLoc => pure .borrowLoc
  | .borrowField field args =>
      if args.isEmpty then pure (.borrowField field)
      else return .borrowFieldInst field (← args.toList.mapM (lowerTy structNames))
  | .borrowVecElem => pure .borrowVecElem
  | .readRef => pure .readRef
  | .writeRef => pure .writeRef
  | .freezeRef => pure .freezeRef
  | .exists_ name args => do
      let i := structNames.findIdx (·.1 == name)
      if i < structNames.size then
        if args.isEmpty then return .exists_ i
        else return .existsInst i (← args.toList.mapM (lowerTy structNames))
      throw s!"unknown resource `{name}`"
  | .moveFrom name args => do
      let i := structNames.findIdx (·.1 == name)
      if i < structNames.size then
        if args.isEmpty then return .moveFrom i
        else return .moveFromInst i (← args.toList.mapM (lowerTy structNames))
      throw s!"unknown resource `{name}`"
  | .moveTo name args => do
      let i := structNames.findIdx (·.1 == name)
      if i < structNames.size then
        if args.isEmpty then return .moveTo i
        else return .moveToInst i (← args.toList.mapM (lowerTy structNames))
      throw s!"unknown resource `{name}`"
  | .function name args => do
      let i := funNames.findIdx (·.1 == name)
      if i < funNames.size then
        if args.isEmpty then return .function i
        else return .functionInst i (← args.toList.mapM (lowerTy structNames))
      let external := externalFunNames.findIdx (· == name)
      if external < externalFunNames.size then
        let id := funNames.size + external
        if args.isEmpty then return .function id
        else return .functionInst id (← args.toList.mapM (lowerTy structNames))
      throw s!"unknown function `{name}`"

private def lowerFun (structNames : Array (Name × String))
    (funNames : Array (Name × String)) (externalFunNames : Array Name)
    (funDecl : FunDecl) :
    Except String (MoveModel.IR.FunDecl × MoveModel.IR.FunMeta) := do
  let allLocals := funDecl.params ++ funDecl.locals
  let localNames := allLocals.map (·.name)
  let blockNames := funDecl.blocks.map (·.name)
  ensureUnique "local name" localNames
  ensureUnique "block name" blockNames
  let localId (name : String) := lookup "local" name localNames
  let block (name : String) := lookup "block" name blockNames
  let mut blocks := #[]
  for sourceBlock in funDecl.blocks do
    let mut instrs := []
    for instr in sourceBlock.instrs do
      match instr with
      | .loadBool dst value =>
          instrs := instrs ++ [.load (← localId dst) (.bool value)]
      | .loadU64 dst value =>
          unless value < 2 ^ 64 do
            throw s!"u64 literal `{value}` does not fit in 64 bits"
          instrs := instrs ++ [.load (← localId dst) (.u64 value)]
      | .assign dst src =>
          instrs := instrs ++ [.assign (← localId dst) (← localId src)]
      | .call dsts op srcs =>
          instrs := instrs ++ [.call (← dsts.toList.mapM localId)
            (← lowerOper structNames funNames externalFunNames op)
            (← srcs.toList.mapM localId)]
    let term ← match sourceBlock.term with
      | .jump target => pure (.jump (← block target))
      | .branch cond thenBlock elseBlock =>
          pure (.branch (← localId cond) (← block thenBlock) (← block elseBlock))
      | .ret srcs => pure (.ret (← srcs.toList.mapM localId))
      | .abort code => pure (.abort (← localId code))
    blocks := blocks.push { instrs := instrs, term := term }
  let entry ← block "entry"
  let locals ← allLocals.toList.mapM fun localDecl => lowerTy structNames localDecl.ty
  let returns ← funDecl.returns.toList.mapM (lowerTy structNames)
  let acquires ← funDecl.acquires.toList.mapM fun name => do
    let i := structNames.findIdx (·.1 == name)
    if i < structNames.size then return i
    throw s!"unknown acquired resource `{name}`"
  let (visibility, isEntry) := match funDecl.visibility with
    | .private_ => (MoveModel.IR.Visibility.private_, false)
    | .public_ => (MoveModel.IR.Visibility.public_, false)
    | .friend_ => (MoveModel.IR.Visibility.friend, false)
    | .entry => (MoveModel.IR.Visibility.public_, true)
  let decl : MoveModel.IR.FunDecl := {
    typeParams := funDecl.typeParams.toList
    numParams := funDecl.params.size
    numLocals := locals.length
    locals := fun t => locals[t]?
    returns := returns
    body := { blocks := fun b => blocks[b]?, entry := entry, size := blocks.size }
    loopSpecs := fun _ => none
    contract := {
      requires := .value (.bool true)
      aborts := none
      ensures := .value (.bool true)
      modifies := []
    }
  }
  let info : MoveModel.IR.FunMeta := {
    name := funDecl.moveName
    visibility := visibility
    isEntry := isEntry
    acquires := acquires
  }
  return (decl, info)

/-- Resolve names to deterministic positional identifiers and construct the
canonical semantic Move IR module. -/
def Module.toIR (module : Module) : Except String MoveModel.IR.Module := do
  let structNames := module.structs.map fun s => (s.leanName, s.moveName)
  let funNames := module.functions.map fun f => (f.leanName, f.moveName)
  let externalFunNames := module.externalFuns.map (·.leanName)
  ensureUnique "struct name" (module.structs.map (·.moveName))
  ensureUnique "function name" (module.functions.map (·.moveName))
  let mut structs : Array MoveModel.IR.StructDecl := #[]
  let mut structMeta : Array MoveModel.IR.StructMeta := #[]
  for structDecl in module.structs do
    ensureUnique "field name" (structDecl.fields.map (·.moveName))
    let fields ← structDecl.fields.toList.mapM fun field => lowerTy structNames field.ty
    let variants ← structDecl.variants.mapM fun variants =>
      variants.toList.mapM fun variant =>
        variant.fields.toList.mapM fun field => lowerTy structNames field.ty
    structs := structs.push {
      typeParams := structDecl.typeParams.toList
      fields := fields
      variants := variants
    }
    structMeta := structMeta.push {
      name := structDecl.moveName
      fieldNames := structDecl.fields.toList.map (·.moveName)
      variantNames := structDecl.variants.map fun variants =>
        variants.toList.map fun variant =>
          (variant.moveName, variant.fields.toList.map (·.moveName))
      abilities := structDecl.abilities
    }
  let mut funs : Array MoveModel.IR.FunDecl := #[]
  let mut funMeta : Array MoveModel.IR.FunMeta := #[]
  for funDecl in module.functions do
    let (decl, info) ← lowerFun structNames funNames externalFunNames funDecl
    funs := funs.push decl
    funMeta := funMeta.push info
  return {
    address := ← parseAddress module.address
    name := module.name
    program := {
      structs := fun r => structs[r]?
      funs := fun f => funs[f]?
    }
    numStructs := structs.size
    numFuns := funs.size
    structMeta := fun r => structMeta[r]?
    funMeta := fun f => funMeta[f]?
    externalFuns := ← module.externalFuns.toList.mapM fun reference => do
      return {
        address := ← parseAddress reference.address
        moduleName := reference.moduleName
        functionName := reference.functionName
      }
    dialect := .stackless
  }

end Move.Compiler.LIR
