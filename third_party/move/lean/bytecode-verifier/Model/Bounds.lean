-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Model.StackUsage

/-!
# Bounds checking

UNTRUSTED. Transcribes `move-binary-format/src/check_bounds.rs` for modules:
every index into a table is in range, struct type tokens have the right number
of type arguments, type-parameter indices are in scope, branch targets and
local indices are within their code unit, and a function has at most 255
locals. Scripts are not modeled yet (plan Phase 5).
-/

namespace MoveBytecode.Model

mutual
/-- `SignatureToken::preorder_traversal`: a token, then the traversals of its
children in order (for a function type, parameters then results). -/
def preorder : SignatureToken → List SignatureToken
  | .vector e => .vector e :: preorder e
  | .reference e => .reference e :: preorder e
  | .mutableReference e => .mutableReference e :: preorder e
  | .structInstantiation h args => .structInstantiation h args :: preorderList args
  | .function ps rs a => .function ps rs a :: (preorderList ps ++ preorderList rs)
  | .bool => [.bool]
  | .u8 => [.u8]
  | .u16 => [.u16]
  | .u32 => [.u32]
  | .u64 => [.u64]
  | .u128 => [.u128]
  | .u256 => [.u256]
  | .i8 => [.i8]
  | .i16 => [.i16]
  | .i32 => [.i32]
  | .i64 => [.i64]
  | .i128 => [.i128]
  | .i256 => [.i256]
  | .address => [.address]
  | .signer => [.signer]
  | .struct h => [.struct h]
  | .typeParameter i => [.typeParameter i]

def preorderList : List SignatureToken → List SignatureToken
  | [] => []
  | t :: ts => preorder t ++ preorderList ts
end

/-- `check_bounds_impl`. -/
def checkIdx (size idx : Nat) : VResult Unit :=
  if idx ≥ size then reject "INDEX_OUT_OF_BOUNDS" else pure ()

/-- `StructFieldInformation::variant_count`. -/
def variantCount : StructFieldInformation → Nat
  | .declaredVariants vs => vs.length
  | .native => 0
  | .declared _ => 0

namespace Bounds

variable (m : CompiledModule)

/-- `check_type`: struct handles in range, with as many type arguments as the
handle declares. -/
def checkType (ty : SignatureToken) : VResult Unit :=
  (preorder ty).forM fun t => match t with
    | .struct idx => do
      checkIdx m.structHandles.size idx
      match m.structHandles[idx]? with
      | some sh =>
        if !sh.typeParameters.isEmpty then reject "NUMBER_OF_TYPE_ARGUMENTS_MISMATCH" else pure ()
      | none => pure ()
    | .structInstantiation idx args => do
      checkIdx m.structHandles.size idx
      match m.structHandles[idx]? with
      | some sh =>
        if sh.typeParameters.length != args.length then reject "NUMBER_OF_TYPE_ARGUMENTS_MISMATCH"
        else pure ()
      | none => pure ()
    | _ => pure ()

/-- `check_type_parameters_in_ty`. -/
def checkTypeParametersInTy (ty : SignatureToken) (count : Nat) : VResult Unit :=
  (preorder ty).forM fun t => match t with
    | .typeParameter i => if i ≥ count then reject "INDEX_OUT_OF_BOUNDS" else pure ()
    | _ => pure ()

/-- `check_type_parameters_in_signature`: the largest type-parameter index in
the signature is in scope. -/
def checkTypeParametersInSignature (sigIdx : Nat) (count : Nat) : VResult Unit := do
  let sig ← lookup m.signatures sigIdx "signature_at"
  let maxIdx := (preorderList sig).foldl (fun acc t => match t with
    | .typeParameter i => some (match acc with
      | some a => Nat.max a i
      | none => i)
    | _ => acc) none
  match maxIdx with
  | some i => if i ≥ count then reject "INDEX_OUT_OF_BOUNDS" else pure ()
  | none => pure ()

def checkModuleHandle (h : ModuleHandle) : VResult Unit := do
  checkIdx m.addressIdentifiers.size h.address
  checkIdx m.identifiers.size h.name

def checkFunctionHandle (h : FunctionHandle) : VResult Unit := do
  checkIdx m.moduleHandles.size h.module
  checkIdx m.identifiers.size h.name
  checkIdx m.signatures.size h.parameters
  checkIdx m.signatures.size h.returns
  checkTypeParametersInSignature m h.parameters h.typeParameters.length
  checkTypeParametersInSignature m h.returns h.typeParameters.length

def checkFieldHandle (h : FieldHandle) : VResult Unit := do
  checkIdx m.structDefs.size h.owner
  match m.structDefs[h.owner]? with
  | some sd =>
    if h.field ≥ fieldCount sd.fieldInformation none then reject "INDEX_OUT_OF_BOUNDS" else pure ()
  | none => pure ()

def checkVariantIndex (sd : StructDefinition) (v : Nat) : VResult Unit :=
  if v ≥ variantCount sd.fieldInformation then reject "INDEX_OUT_OF_BOUNDS" else pure ()

def checkVariantFieldHandle (h : VariantFieldHandle) : VResult Unit := do
  checkIdx m.structDefs.size h.structIndex
  if h.variants.isEmpty then reject "ZERO_VARIANTS_ERROR"
  let sd ← lookup m.structDefs h.structIndex "struct_def_at"
  h.variants.forM fun v => do
    checkVariantIndex sd v
    if h.field ≥ fieldCount sd.fieldInformation (some v) then reject "INDEX_OUT_OF_BOUNDS"

def checkStructVariantHandle (h : StructVariantHandle) : VResult Unit := do
  checkIdx m.structDefs.size h.structIndex
  checkVariantIndex (← lookup m.structDefs h.structIndex "struct_def_at") h.variant

def checkFieldDef (typeParamCount : Nat) (f : FieldDefinition) : VResult Unit := do
  checkIdx m.identifiers.size f.name
  checkType m f.signature
  checkTypeParametersInTy f.signature typeParamCount

def checkStructDef (sd : StructDefinition) : VResult Unit := do
  checkIdx m.structHandles.size sd.structHandle
  let typeParamCount := match m.structHandles[sd.structHandle]? with
    | some sh => sh.typeParameters.length
    | none => 0
  match sd.fieldInformation with
  | .declared fields => fields.forM (checkFieldDef m typeParamCount)
  | .declaredVariants variants => do
    variants.forM fun v => do
      checkIdx m.identifiers.size v.name
      v.fields.forM (checkFieldDef m typeParamCount)
    if variants.isEmpty then reject "ZERO_VARIANTS_ERROR"
  | .native => pure ()

/-- The per-instruction checks of `check_code`. -/
def checkInstr (codeLen localsCount typeParamCount : Nat) : Bytecode → VResult Unit
  | .ldConst i => checkIdx m.constantPool.size i
  | .mutBorrowField h | .immBorrowField h => checkIdx m.fieldHandles.size h
  | .mutBorrowVariantField h | .immBorrowVariantField h => checkIdx m.variantFieldHandles.size h
  | .mutBorrowFieldGeneric i | .immBorrowFieldGeneric i => do
    checkIdx m.fieldInstantiations.size i
    match m.fieldInstantiations[i]? with
    | some fi => checkTypeParametersInSignature m fi.typeParameters typeParamCount
    | none => pure ()
  | .mutBorrowVariantFieldGeneric i | .immBorrowVariantFieldGeneric i => do
    checkIdx m.variantFieldInstantiations.size i
    match m.variantFieldInstantiations[i]? with
    | some fi => checkTypeParametersInSignature m fi.typeParameters typeParamCount
    | none => pure ()
  | .call h | .packClosure h _ => checkIdx m.functionHandles.size h
  | .callGeneric i | .packClosureGeneric i _ => do
    checkIdx m.functionInstantiations.size i
    match m.functionInstantiations[i]? with
    | some fi => checkTypeParametersInSignature m fi.typeParameters typeParamCount
    | none => pure ()
  | .pack d | .unpack d | .exists_ d | .immBorrowGlobal d | .mutBorrowGlobal d | .moveFrom d
  | .moveTo d => checkIdx m.structDefs.size d
  | .packVariant h | .unpackVariant h | .testVariant h => checkIdx m.structVariantHandles.size h
  | .packGeneric i | .unpackGeneric i | .existsGeneric i | .immBorrowGlobalGeneric i
  | .mutBorrowGlobalGeneric i | .moveFromGeneric i | .moveToGeneric i => do
    checkIdx m.structDefInstantiations.size i
    match m.structDefInstantiations[i]? with
    | some si => checkTypeParametersInSignature m si.typeParameters typeParamCount
    | none => pure ()
  | .packVariantGeneric i | .unpackVariantGeneric i | .testVariantGeneric i => do
    checkIdx m.structVariantInstantiations.size i
    match m.structVariantInstantiations[i]? with
    | some si => checkTypeParametersInSignature m si.typeParameters typeParamCount
    | none => pure ()
  | .brTrue o | .brFalse o | .branch o => checkIdx codeLen o
  | .copyLoc i | .moveLoc i | .stLoc i | .mutBorrowLoc i | .immBorrowLoc i => checkIdx localsCount i
  | .callClosure s | .vecPack s _ | .vecLen s | .vecImmBorrow s | .vecMutBorrow s
  | .vecPushBack s | .vecPopBack s | .vecUnpack s _ | .vecSwap s => do
    checkIdx m.signatures.size s
    checkTypeParametersInSignature m s typeParamCount
  | .freezeRef | .pop | .ret | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU256 _ | .ldU128 _
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI256 _ | .ldI128 _
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 | .ldTrue | .ldFalse
  | .readRef | .writeRef | .add | .sub | .mul | .mod | .div | .negate | .bitOr | .bitAnd
  | .xor | .shl | .shr | .or | .and | .not | .eq | .neq | .lt | .gt | .le | .ge | .abort
  | .abortMsg | .nop => pure ()

/-- `check_code`. -/
def checkCode (code : CodeUnit) (typeParamCount : Nat) (params : Signature) : VResult Unit := do
  checkIdx m.signatures.size code.locals
  let locals ← match m.signatures[code.locals]? with
    | some sig => pure sig
    | none => reject "INDEX_OUT_OF_BOUNDS"
  let localsCount := locals.length + params.length
  if localsCount > 255 then reject "TOO_MANY_LOCALS"
  checkTypeParametersInSignature m code.locals typeParamCount
  code.code.toList.forM (checkInstr m code.code.size localsCount typeParamCount)

def checkFunctionDef (fdef : FunctionDefinition) : VResult Unit := do
  checkIdx m.functionHandles.size fdef.function
  fdef.acquiresGlobalResources.forM (checkIdx m.structDefs.size)
  match fdef.code with
  | none => pure ()
  | some code => do
    let fh ← match m.functionHandles[fdef.function]? with
      | some fh => pure fh
      | none => reject "INDEX_OUT_OF_BOUNDS"
    let params ← match m.signatures[fh.parameters]? with
      | some sig => pure sig
      | none => reject "INDEX_OUT_OF_BOUNDS"
    checkCode m code fh.typeParameters.length params

end Bounds

/-- `BoundsChecker::verify_module`. -/
def verifyBounds (m : CompiledModule) : VResult Unit := do
  if m.moduleHandles.isEmpty then reject "NO_MODULE_HANDLES"
  m.signatures.toList.forM (fun sig => sig.forM (Bounds.checkType m))
  m.constantPool.toList.forM (fun c => Bounds.checkType m c.type)
  m.moduleHandles.toList.forM (Bounds.checkModuleHandle m)
  checkIdx m.moduleHandles.size m.selfModuleHandleIdx
  m.structHandles.toList.forM (fun h => do
    checkIdx m.moduleHandles.size h.module
    checkIdx m.identifiers.size h.name)
  m.functionHandles.toList.forM (Bounds.checkFunctionHandle m)
  m.fieldHandles.toList.forM (Bounds.checkFieldHandle m)
  m.friendDecls.toList.forM (Bounds.checkModuleHandle m)
  m.structDefInstantiations.toList.forM (fun i => do
    checkIdx m.structDefs.size i.def_
    checkIdx m.signatures.size i.typeParameters)
  m.functionInstantiations.toList.forM (fun i => do
    checkIdx m.functionHandles.size i.handle
    checkIdx m.signatures.size i.typeParameters)
  m.fieldInstantiations.toList.forM (fun i => do
    checkIdx m.fieldHandles.size i.handle
    checkIdx m.signatures.size i.typeParameters)
  m.structDefs.toList.forM (Bounds.checkStructDef m)
  m.variantFieldHandles.toList.forM (Bounds.checkVariantFieldHandle m)
  m.structVariantHandles.toList.forM (Bounds.checkStructVariantHandle m)
  m.variantFieldInstantiations.toList.forM (fun i => do
    checkIdx m.variantFieldHandles.size i.handle
    checkIdx m.signatures.size i.typeParameters)
  m.structVariantInstantiations.toList.forM (fun i => do
    checkIdx m.structVariantHandles.size i.handle
    checkIdx m.signatures.size i.typeParameters)
  m.functionDefs.toList.forM (Bounds.checkFunctionDef m)

end MoveBytecode.Model
