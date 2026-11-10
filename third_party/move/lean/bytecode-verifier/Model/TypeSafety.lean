-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Model.Abilities
import Model.SignatureV2
import Model.StackUsage

/-!
# Type safety

UNTRUSTED. Transcribes `move-bytecode-verifier/src/type_safety.rs`: an
abstract stack of signature tokens, threaded through the blocks in order and
never reset (the stack-usage pass has made every block balanced), with each
instruction checking and transforming it.

Modeled: every instruction: the core subset, the data instructions of Phase 2
(`Pack`, `Unpack`, their variant forms, `VecPack`, `VecUnpack`, `AbortMsg`),
the references, vector operations and global-storage operations of Phase 3,
and the generic forms and closures of Phase 4 (`clos_pack` and
`call_closure`, with `ClosureMask::extract` from
`move-core/types/src/function.rs`). The verifier meter is not modeled.
-/

namespace MoveBytecode.Model

/-- `SignatureToken::is_integer`. -/
def isInteger : SignatureToken → Bool
  | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => true
  | _ => false

/-- `SignatureToken::is_unsigned_integer`. -/
def isUnsignedInteger : SignatureToken → Bool
  | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 => true
  | _ => false

/-- `SignatureToken::is_signed_integer`. -/
def isSignedInteger : SignatureToken → Bool
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => true
  | _ => false

/-- `SignatureToken::is_assignable_from`: function types may lose abilities,
immutable references are covariant, and otherwise the types are equal. -/
def isAssignableFrom : SignatureToken → SignatureToken → Bool
  | .function args1 results1 ab1, .function args2 results2 ab2 =>
    args1 == args2 && results1 == results2 && ab1.subset ab2
  | .reference t1, .reference t2 => isAssignableFrom t1 t2
  | t1, t2 => t1 == t2

/-- `Locals::local_at`: parameters, then the other locals. -/
def localAt (view : FunctionView) (i : Nat) : VResult SignatureToken :=
  if i < view.parameters.length then lookup view.parameters.toArray i "parameters[idx]"
  else lookup view.locals.toArray (i - view.parameters.length) "locals[idx]"

/-- `safe_unwrap!(stack.pop())`: an empty stack is an invariant violation. -/
def popType : List SignatureToken → VResult (SignatureToken × List SignatureToken)
  | t :: rest => pure (t, rest)
  | [] => reject "UNKNOWN_INVARIANT_VIOLATION_ERROR"

/-- Pop one value per expected type, last expected type first; each must be
assignable to its expected type. -/
def popExpected (status : String) (stack : List SignatureToken) (expected : List SignatureToken) :
    VResult (List SignatureToken) :=
  expected.reverse.foldlM (fun stack p => do
    let (arg, rest) ← popType stack
    if !isAssignableFrom p arg then reject status else pure rest) stack

/-- The `call` helper for a non-generic call: pop and check the arguments,
last parameter first, then push the returns. -/
def typeCall (m : CompiledModule) (stack : List SignatureToken) (fh : FunctionHandle) :
    VResult (List SignatureToken) := do
  let params ← lookup m.signatures fh.parameters "signature_at"
  let stack ← popExpected "CALL_TYPE_MISMATCH_ERROR" stack params
  let returns ← lookup m.signatures fh.returns "signature_at"
  pure (returns.reverse ++ stack)

/-- Pop two operands (the first is the top). -/
def popTwo (stack : List SignatureToken) :
    VResult (SignatureToken × SignatureToken × List SignatureToken) := do
  let (op1, stack) ← popType stack
  let (op2, stack) ← popType stack
  pure (op1, op2, stack)

def castTo (stack : List SignatureToken) (target : SignatureToken) :
    VResult (List SignatureToken) := do
  let (op, stack) ← popType stack
  if !isInteger op then reject "INTEGER_OP_TYPE_MISMATCH_ERROR" else pure (target :: stack)

/-- `type_fields_signature` without type arguments: a struct's declared
fields, or one variant's. -/
def fieldsSignature (sd : StructDefinition) (variant : Option Nat) :
    VResult (List SignatureToken) :=
  match sd.fieldInformation, variant with
  | .declared fields, none => pure (fields.map (·.signature))
  | .declaredVariants variants, some v =>
    match variants[v]? with
    | some vd => pure (vd.fields.map (·.signature))
    | none => reject "PACK_TYPE_MISMATCH_ERROR"
  | .declared _, some _ | .declaredVariants _, none | .native, _ =>
    reject "PACK_TYPE_MISMATCH_ERROR"

/-- `pack` without type arguments: pop the fields, last first, each
assignable to its declared type, and push the struct. -/
def typePack (stack : List SignatureToken) (sd : StructDefinition) (variant : Option Nat) :
    VResult (List SignatureToken) := do
  let fields ← fieldsSignature sd variant
  let stack ← popExpected "PACK_TYPE_MISMATCH_ERROR" stack fields
  pure (.struct sd.structHandle :: stack)

/-- `unpack` without type arguments: pop exactly the struct, push its fields,
last on top. -/
def typeUnpack (stack : List SignatureToken) (sd : StructDefinition) (variant : Option Nat) :
    VResult (List SignatureToken) := do
  let (arg, stack) ← popType stack
  if arg != .struct sd.structHandle then reject "UNPACK_TYPE_MISMATCH_ERROR"
  let fields ← fieldsSignature sd variant
  pure (fields.reverse ++ stack)

/-- `signature_at(idx).0[0]`, which panics on an empty signature. -/
def elementType (m : CompiledModule) (idx : Nat) : VResult SignatureToken := do
  match (← lookup m.signatures idx "signature_at").head? with
  | some t => pure t
  | none => panicAt "signature_at(idx).0[0]"

/-- `VecPack`: pop `n` operands, each assignable to the element type. -/
def popElements (elem : SignatureToken) (stack : List SignatureToken) (n : Nat) :
    VResult (List SignatureToken) :=
  (List.range n).foldlM (fun stack _ => do
    let (op, stack) ← popType stack
    if !isAssignableFrom elem op then reject "TYPE_MISMATCH" else pure stack) stack

/-- `get_vector_element_type`. -/
def vectorElement : SignatureToken → Option SignatureToken
  | .vector t => some t
  | _ => none

/-- `SignatureToken::is_mutable_reference`. -/
def isMutableReference : SignatureToken → Bool
  | .mutableReference _ => true
  | _ => false

/-- A reference of the given mutability to `t`. -/
def refTo (mut_ : Bool) (t : SignatureToken) : SignatureToken :=
  if mut_ then .mutableReference t else .reference t

/-- `StructFieldInformation::fields`: a struct's fields, whatever the variant;
an enum's variant's fields, or none without a valid variant. -/
def structFields (fi : StructFieldInformation) (variant : Option Nat) : List FieldDefinition :=
  match fi, variant with
  | .native, _ => []
  | .declared fields, _ => fields
  | .declaredVariants variants, some v =>
    match variants[v]? with
    | some vd => vd.fields
    | none => []
  | .declaredVariants _, none => []

/-- The type of field `field` that every variant of `variants` declares, which
must be one type. -/
def variantFieldType (sd : StructDefinition) (field : Nat) (variants : List Nat) :
    VResult (Option SignatureToken) :=
  variants.foldlM (fun acc v => do
    match (structFields sd.fieldInformation (some v))[field]? with
    | some fd =>
      match acc with
      | some t => if fd.signature != t then reject "BORROWFIELD_TYPE_MISMATCH_ERROR" else pure acc
      | none => pure (some fd.signature)
    | none => reject "BORROWFIELD_BAD_FIELD_ERROR") none

/-- The target of a reference type. -/
def refTarget : SignatureToken → Option SignatureToken
  | .reference t | .mutableReference t => some t
  | _ => none

/-- The owner, the variants (for a variant field handle) and the offset of a
field handle. -/
def fieldHandleParts (m : CompiledModule) (variant : Bool) (h : Nat) :
    VResult (Nat × Option (List Nat) × Nat) :=
  if variant then do
    let fh ← lookup m.variantFieldHandles h "variant_field_handle_at"
    pure (fh.structIndex, some fh.variants, fh.field)
  else do
    let fh ← lookup m.fieldHandles h "field_handle_at"
    pure (fh.owner, none, fh.field)

/-- `borrow_field` without type arguments: pop a reference to the owner and
push a reference to the field, of field handle `h` or, if `variant`, of
variant field handle `h`. A field that does not exist pushes a placeholder:
the bounds checker reports it. -/
def borrowField (m : CompiledModule) (stack : List SignatureToken) (mut_ : Bool)
    (variant : Bool) (h : Nat) : VResult (List SignatureToken) := do
  let (operand, stack) ← popType stack
  rejectIf (mut_ && !isMutableReference operand) "BORROWFIELD_TYPE_MISMATCH_ERROR"
  let (owner, variants, field) ← fieldHandleParts m variant h
  let sd ← lookup m.structDefs owner "struct_def_at"
  rejectIf (refTarget operand != some (.struct sd.structHandle)) "BORROWFIELD_TYPE_MISMATCH_ERROR"
  let fieldTy ← match variants with
    | some vs =>
      if vs.isEmpty then reject "ZERO_VARIANTS_ERROR" else variantFieldType sd field vs
    | none => pure ((structFields sd.fieldInformation none)[field]?.map (·.signature))
  match fieldTy with
  | some t => pure (refTo mut_ t :: stack)
  | none => pure (.bool :: stack)

/-- `borrow_global` without type arguments. -/
def borrowGlobal (m : CompiledModule) (constraints : List AbilitySet)
    (stack : List SignatureToken) (mut_ : Bool) (idx : StructDefinitionIndex) :
    VResult (List SignatureToken) := do
  let (operand, stack) ← popType stack
  rejectIf (operand != .address) "BORROWGLOBAL_TYPE_MISMATCH_ERROR"
  let sd ← lookup m.structDefs idx "struct_def_at"
  let a ← abilities m constraints (.struct sd.structHandle)
  rejectIf (!a.key) "BORROWGLOBAL_WITHOUT_KEY_ABILITY"
  pure (refTo mut_ (.struct sd.structHandle) :: stack)

/-- `get_vector_ref_element_type`: the element type of a reference to a
vector, of a mutable one if `mutOnly`. -/
def vectorRefElement (t : SignatureToken) (mutOnly : Bool) : Option SignatureToken :=
  match t with
  | .reference (.vector e) => if mutOnly then none else some e
  | .mutableReference (.vector e) => some e
  | _ => none

/-- `borrow_vector_element`. -/
def borrowVectorElement (stack : List SignatureToken) (declared : SignatureToken) (mut_ : Bool) :
    VResult (List SignatureToken) := do
  let (idx, stack) ← popType stack
  let (vec, stack) ← popType stack
  rejectIf (idx != .u64) "TYPE_MISMATCH"
  match vectorRefElement vec mut_ with
  | some e => if declared == e then pure (refTo mut_ e :: stack) else reject "TYPE_MISMATCH"
  | none => reject "TYPE_MISMATCH"

/-! ## Type arguments

Rust shares one helper between an instruction's non-generic form, which passes
an empty signature, and its generic form, which passes the instantiation's
type arguments. The helpers above are the non-generic forms; those below take
the type arguments. With no type arguments they behave as the ones above:
`instantiate` and `materialize_type` change nothing. -/

/-- `instantiate`, for a non-empty substitution: `subst` for each type
parameter. A type parameter outside `subst` panics: Rust indexes
`subst.0[idx]`. -/
def instantiateIn (subst : List SignatureToken) : SignatureToken → VResult SignatureToken
  | .bool => pure .bool
  | .u8 => pure .u8
  | .u16 => pure .u16
  | .u32 => pure .u32
  | .u64 => pure .u64
  | .u128 => pure .u128
  | .u256 => pure .u256
  | .i8 => pure .i8
  | .i16 => pure .i16
  | .i32 => pure .i32
  | .i64 => pure .i64
  | .i128 => pure .i128
  | .i256 => pure .i256
  | .address => pure .address
  | .signer => pure .signer
  | .vector t => do pure (.vector (← instantiateIn subst t))
  | .function args results a => do
    let args ← args.attach.mapM (fun ⟨x, _⟩ => instantiateIn subst x)
    let results ← results.attach.mapM (fun ⟨x, _⟩ => instantiateIn subst x)
    pure (.function args results a)
  | .struct h => pure (.struct h)
  | .structInstantiation h args => do
    pure (.structInstantiation h (← args.attach.mapM (fun ⟨x, _⟩ => instantiateIn subst x)))
  | .reference t => do pure (.reference (← instantiateIn subst t))
  | .mutableReference t => do pure (.mutableReference (← instantiateIn subst t))
  | .typeParameter i => lookup subst.toArray i "subst[idx]"

/-- `instantiate`: `token` unchanged when `subst` is empty. -/
def instantiate (subst : List SignatureToken) (token : SignatureToken) : VResult SignatureToken :=
  if subst.isEmpty then pure token else instantiateIn subst token

/-- `materialize_type`: the struct type of a handle under type arguments. -/
def materializeType (h : StructHandleIndex) (typeArgs : List SignatureToken) : SignatureToken :=
  if typeArgs.isEmpty then .struct h else .structInstantiation h typeArgs

/-- `call` with type actuals (`CallGeneric`): pop the arguments, last
parameter first, each assignable to its instantiated parameter type, then
push the instantiated returns. -/
def typeCallGeneric (m : CompiledModule) (stack : List SignatureToken) (fh : FunctionHandle)
    (tys : List SignatureToken) : VResult (List SignatureToken) := do
  let params ← lookup m.signatures fh.parameters "signature_at"
  let stack ← params.reverse.foldlM (fun stack p => do
    let (arg, rest) ← popType stack
    if !isAssignableFrom (← instantiate tys p) arg then reject "CALL_TYPE_MISMATCH_ERROR"
    else pure rest) stack
  let returns ← lookup m.signatures fh.returns "signature_at"
  pure ((← returns.mapM (instantiate tys)).reverse ++ stack)

/-- `type_fields_signature` with type arguments. -/
def fieldsSignatureGeneric (sd : StructDefinition) (variant : Option Nat)
    (tys : List SignatureToken) : VResult (List SignatureToken) := do
  (← fieldsSignature sd variant).mapM (instantiate tys)

/-- `pack` with type arguments. -/
def typePackGeneric (stack : List SignatureToken) (sd : StructDefinition) (variant : Option Nat)
    (tys : List SignatureToken) : VResult (List SignatureToken) := do
  let fields ← fieldsSignatureGeneric sd variant tys
  let stack ← popExpected "PACK_TYPE_MISMATCH_ERROR" stack fields
  pure (materializeType sd.structHandle tys :: stack)

/-- `unpack` with type arguments. -/
def typeUnpackGeneric (stack : List SignatureToken) (sd : StructDefinition)
    (variant : Option Nat) (tys : List SignatureToken) : VResult (List SignatureToken) := do
  let (arg, stack) ← popType stack
  if arg != materializeType sd.structHandle tys then reject "UNPACK_TYPE_MISMATCH_ERROR"
  let fields ← fieldsSignatureGeneric sd variant tys
  pure (fields.reverse ++ stack)

/-- `variantFieldType` with type arguments: the field types are compared
after instantiation. -/
def variantFieldTypeGeneric (sd : StructDefinition) (field : Nat) (variants : List Nat)
    (tys : List SignatureToken) : VResult (Option SignatureToken) :=
  variants.foldlM (fun acc v => do
    match (structFields sd.fieldInformation (some v))[field]? with
    | some fd =>
      let ty ← instantiate tys fd.signature
      match acc with
      | some t => if ty != t then reject "BORROWFIELD_TYPE_MISMATCH_ERROR" else pure acc
      | none => pure (some ty)
    | none => reject "BORROWFIELD_BAD_FIELD_ERROR") none

/-- `borrow_field` with type arguments, of field instantiation's handle `h`
or, if `variant`, of a variant field instantiation's handle `h`. -/
def borrowFieldGeneric (m : CompiledModule) (stack : List SignatureToken) (mut_ : Bool)
    (variant : Bool) (h : Nat) (tys : List SignatureToken) : VResult (List SignatureToken) := do
  let (operand, stack) ← popType stack
  rejectIf (mut_ && !isMutableReference operand) "BORROWFIELD_TYPE_MISMATCH_ERROR"
  let (owner, variants, field) ← fieldHandleParts m variant h
  let sd ← lookup m.structDefs owner "struct_def_at"
  rejectIf (refTarget operand != some (materializeType sd.structHandle tys))
    "BORROWFIELD_TYPE_MISMATCH_ERROR"
  let fieldTy ← match variants with
    | some vs =>
      if vs.isEmpty then reject "ZERO_VARIANTS_ERROR" else variantFieldTypeGeneric sd field vs tys
    | none =>
      match (structFields sd.fieldInformation none)[field]? with
      | some fd => some <$> instantiate tys fd.signature
      | none => pure none
  match fieldTy with
  | some t => pure (refTo mut_ t :: stack)
  | none => pure (.bool :: stack)

/-- `borrow_global` with type arguments. -/
def borrowGlobalGeneric (m : CompiledModule) (constraints : List AbilitySet)
    (stack : List SignatureToken) (mut_ : Bool) (idx : StructDefinitionIndex)
    (tys : List SignatureToken) : VResult (List SignatureToken) := do
  let (operand, stack) ← popType stack
  rejectIf (operand != .address) "BORROWGLOBAL_TYPE_MISMATCH_ERROR"
  let sd ← lookup m.structDefs idx "struct_def_at"
  let a ← abilities m constraints (materializeType sd.structHandle tys)
  rejectIf (!a.key) "BORROWGLOBAL_WITHOUT_KEY_ABILITY"
  pure (refTo mut_ (materializeType sd.structHandle tys) :: stack)

/-- A struct instantiation's definition and type arguments
(`struct_instantiation_at`, `struct_def_at`, `signature_at`). -/
def structInstParts (m : CompiledModule) (i : StructDefInstantiationIndex) :
    VResult (StructDefinitionIndex × StructDefinition × List SignatureToken) := do
  let si ← lookup m.structDefInstantiations i "struct_instantiation_at"
  let sd ← lookup m.structDefs si.def_ "struct_def_at"
  pure (si.def_, sd, ← lookup m.signatures si.typeParameters "signature_at")

/-- A variant instantiation's variant handle, definition and type arguments
(`struct_variant_instantiation_at`, `struct_variant_handle_at`,
`struct_def_at`, `signature_at`). -/
def variantInstParts (m : CompiledModule) (i : StructVariantInstantiationIndex) :
    VResult (StructVariantHandle × StructDefinition × List SignatureToken) := do
  let vi ← lookup m.structVariantInstantiations i "struct_variant_instantiation_at"
  let vh ← lookup m.structVariantHandles vi.handle "struct_variant_handle_at"
  let sd ← lookup m.structDefs vh.structIndex "struct_def_at"
  pure (vh, sd, ← lookup m.signatures vi.typeParameters "signature_at")

/-! ## Closures -/

/-- `ClosureMask::extract`: the elements at captured positions (bit `i` of
the mask set for position `i`) when `captured`, the others otherwise. A
position at or beyond 64 is never captured. -/
def maskExtract {α : Type} (mask : BitVec 64) (l : List α) (captured : Bool) : List α :=
  (l.zipIdx.filter (fun p => mask.getLsbD p.2 == captured)).map (·.1)

/-- The abilities `clos_pack` starts from: a closure may be stored only when
the target's handle carries `persistent`. -/
def closureInitAbilities (fh : FunctionHandle) : AbilitySet :=
  if fh.attributes.contains .persistent then publicFunctionAbilities else privateFunctionAbilities

/-- One captured argument of `clos_pack`: pop it, intersect the closure's
abilities with its own, and check it is assignable to the instantiated
parameter type, which must not be a reference. -/
def closCapture (m : CompiledModule) (constraints : List AbilitySet) (tys : List SignatureToken)
    (acc : AbilitySet × List SignatureToken) (ty : SignatureToken) :
    VResult (AbilitySet × List SignatureToken) := do
  let (arg, stack) ← popType acc.2
  let ab := acc.1.inter (← abilities m constraints arg)
  if !isAssignableFrom (← instantiate tys ty) arg then reject "PACK_TYPE_MISMATCH_ERROR"
  if SignatureV2.isReference ty then reject "PACK_TYPE_MISMATCH_ERROR"
  pure (ab, stack)

/-- `clos_pack`: pop the captured arguments, last first, each assignable to
its instantiated parameter type and not of a reference type, intersecting the
closure's abilities with each; push the function type of the parameters not
captured, instantiated. The closure may be stored only when the handle
carries `persistent`. -/
def typeClosPack (m : CompiledModule) (constraints : List AbilitySet)
    (stack : List SignatureToken) (fh : FunctionHandle) (tys : List SignatureToken)
    (mask : BitVec 64) : VResult (List SignatureToken) := do
  let init := closureInitAbilities fh
  let params ← lookup m.signatures fh.parameters "signature_at"
  let (ab, stack) ← (maskExtract mask params true).reverse.foldlM
    (closCapture m constraints tys) (init, stack)
  let returns ← lookup m.signatures fh.returns "signature_at"
  pure ((← instantiate tys (.function (maskExtract mask params false) returns ab)) :: stack)

/-- `call_closure`: pop the closure, assignable to the expected function
type, then the arguments, last first, each assignable to its parameter type;
push the results. -/
def typeCallClosure (stack : List SignatureToken) (expected : SignatureToken) :
    VResult (List SignatureToken) := do
  match expected with
  | .function params results _ =>
    let (closure, stack) ← popType stack
    if !isAssignableFrom expected closure then reject "CALL_TYPE_MISMATCH_ERROR"
    let stack ← popExpected "CALL_TYPE_MISMATCH_ERROR" stack params
    pure (results.reverse ++ stack)
  -- Rust: "the signature checker has ensured this is a function".
  | _ => panicAt "safe_assert!(false)"

/-- `verify_instr`. -/
def typeInstr (m : CompiledModule) (view : FunctionView) (stack : List SignatureToken) :
    Bytecode → VResult (List SignatureToken)
  | .pop => do
    let (op, stack) ← popType stack
    if !(← abilities m view.handle.typeParameters op).drop then reject "POP_WITHOUT_DROP_ABILITY"
    else pure stack
  | .brTrue _ | .brFalse _ => do
    let (op, stack) ← popType stack
    if op != .bool then reject "BR_TYPE_MISMATCH_ERROR" else pure stack
  | .stLoc i => do
    let (op, stack) ← popType stack
    if !isAssignableFrom (← localAt view i) op then reject "STLOC_TYPE_MISMATCH_ERROR"
    else pure stack
  | .abort => do
    let (op, stack) ← popType stack
    if op != .u64 then reject "ABORT_TYPE_MISMATCH_ERROR" else pure stack
  | .ret => popExpected "RET_TYPE_MISMATCH_ERROR" stack view.returns
  | .branch _ | .nop => pure stack
  | .ldU8 _ => pure (.u8 :: stack)
  | .ldU16 _ => pure (.u16 :: stack)
  | .ldU32 _ => pure (.u32 :: stack)
  | .ldU64 _ => pure (.u64 :: stack)
  | .ldU128 _ => pure (.u128 :: stack)
  | .ldU256 _ => pure (.u256 :: stack)
  | .ldI8 _ => pure (.i8 :: stack)
  | .ldI16 _ => pure (.i16 :: stack)
  | .ldI32 _ => pure (.i32 :: stack)
  | .ldI64 _ => pure (.i64 :: stack)
  | .ldI128 _ => pure (.i128 :: stack)
  | .ldI256 _ => pure (.i256 :: stack)
  | .ldConst i => do pure ((← lookup m.constantPool i "constant_at").type :: stack)
  | .ldTrue | .ldFalse => pure (.bool :: stack)
  | .copyLoc i => do
    let t ← localAt view i
    if !(← abilities m view.handle.typeParameters t).copy then
      reject "COPYLOC_WITHOUT_COPY_ABILITY"
    else pure (t :: stack)
  | .moveLoc i => do pure ((← localAt view i) :: stack)
  | .call h => do typeCall m stack (← lookup m.functionHandles h "function_handle_at")
  | .castU8 => castTo stack .u8
  | .castU16 => castTo stack .u16
  | .castU32 => castTo stack .u32
  | .castU64 => castTo stack .u64
  | .castU128 => castTo stack .u128
  | .castU256 => castTo stack .u256
  | .castI8 => castTo stack .i8
  | .castI16 => castTo stack .i16
  | .castI32 => castTo stack .i32
  | .castI64 => castTo stack .i64
  | .castI128 => castTo stack .i128
  | .castI256 => castTo stack .i256
  | .add | .sub | .mul | .mod | .div => do
    let (op1, op2, stack) ← popTwo stack
    if isInteger op1 && op1 == op2 then pure (op1 :: stack)
    else reject "INTEGER_OP_TYPE_MISMATCH_ERROR"
  | .bitOr | .bitAnd | .xor => do
    let (op1, op2, stack) ← popTwo stack
    if isUnsignedInteger op1 && op1 == op2 then pure (op1 :: stack)
    else reject "INTEGER_OP_TYPE_MISMATCH_ERROR"
  | .negate => do
    let (op, stack) ← popType stack
    if isSignedInteger op then pure (op :: stack) else reject "INTEGER_OP_TYPE_MISMATCH_ERROR"
  | .shl | .shr => do
    let (op1, op2, stack) ← popTwo stack
    if isUnsignedInteger op2 && op1 == .u8 then pure (op2 :: stack)
    else reject "INTEGER_OP_TYPE_MISMATCH_ERROR"
  | .or | .and => do
    let (op1, op2, stack) ← popTwo stack
    if op1 == .bool && op2 == .bool then pure (.bool :: stack)
    else reject "BOOLEAN_OP_TYPE_MISMATCH_ERROR"
  | .not => do
    let (op, stack) ← popType stack
    if op == .bool then pure (.bool :: stack) else reject "BOOLEAN_OP_TYPE_MISMATCH_ERROR"
  | .eq | .neq => do
    let (op1, op2, stack) ← popTwo stack
    if (← abilities m view.handle.typeParameters op1).drop && op1 == op2 then
      pure (.bool :: stack)
    else reject "EQUALITY_OP_TYPE_MISMATCH_ERROR"
  | .lt | .gt | .le | .ge => do
    let (op1, op2, stack) ← popTwo stack
    if isInteger op1 && op1 == op2 then pure (.bool :: stack)
    else reject "INTEGER_OP_TYPE_MISMATCH_ERROR"
  | .pack d => do typePack stack (← lookup m.structDefs d "struct_def_at") none
  | .unpack d => do typeUnpack stack (← lookup m.structDefs d "struct_def_at") none
  | .packVariant h => do
    let vh ← lookup m.structVariantHandles h "struct_variant_handle_at"
    typePack stack (← lookup m.structDefs vh.structIndex "struct_def_at") (some vh.variant)
  | .unpackVariant h => do
    let vh ← lookup m.structVariantHandles h "struct_variant_handle_at"
    typeUnpack stack (← lookup m.structDefs vh.structIndex "struct_def_at") (some vh.variant)
  | .vecPack idx n => do
    let elem ← elementType m idx
    pure (.vector elem :: (← popElements elem stack n))
  | .vecUnpack idx n => do
    let (op, stack) ← popType stack
    let elem ← elementType m idx
    if op != .vector elem then reject "TYPE_MISMATCH"
    else pure (List.replicate n elem ++ stack)
  | .abortMsg => do
    let (message, stack) ← popType stack
    let (code, stack) ← popType stack
    if code != .u64 || vectorElement message != some .u8 then reject "ABORT_TYPE_MISMATCH_ERROR"
    else pure stack
  | .freezeRef => do
    let (op, stack) ← popType stack
    match op with
    | .mutableReference inner => pure (.reference inner :: stack)
    | _ => reject "FREEZEREF_TYPE_MISMATCH_ERROR"
  | .mutBorrowField h => borrowField m stack true false h
  | .immBorrowField h => borrowField m stack false false h
  | .mutBorrowVariantField h => borrowField m stack true true h
  | .immBorrowVariantField h => borrowField m stack false true h
  | .mutBorrowLoc i => do
    let t ← localAt view i
    if SignatureV2.isReference t then reject "BORROWLOC_REFERENCE_ERROR"
    else pure (.mutableReference t :: stack)
  | .immBorrowLoc i => do
    let t ← localAt view i
    if SignatureV2.isReference t then reject "BORROWLOC_REFERENCE_ERROR"
    else pure (.reference t :: stack)
  | .testVariant h => do
    let vh ← lookup m.structVariantHandles h "struct_variant_handle_at"
    let sd ← lookup m.structDefs vh.structIndex "struct_def_at"
    let (arg, stack) ← popType stack
    match arg with
    | .reference inner | .mutableReference inner =>
      if inner == .struct sd.structHandle then pure (.bool :: stack)
      else reject "TEST_VARIANT_TYPE_MISMATCH_ERROR"
    | _ => reject "TEST_VARIANT_TYPE_MISMATCH_ERROR"
  | .readRef => do
    let (op, stack) ← popType stack
    match op with
    | .reference inner | .mutableReference inner =>
      if !(← abilities m view.handle.typeParameters inner).copy then
        reject "READREF_WITHOUT_COPY_ABILITY"
      else pure (inner :: stack)
    | _ => reject "READREF_TYPE_MISMATCH_ERROR"
  | .writeRef => do
    let (refOp, stack) ← popType stack
    let (val, stack) ← popType stack
    match refOp with
    | .mutableReference inner =>
      if !(← abilities m view.handle.typeParameters inner).drop then
        reject "WRITEREF_WITHOUT_DROP_ABILITY"
      else if !isAssignableFrom inner val then reject "WRITEREF_TYPE_MISMATCH_ERROR"
      else pure stack
    | _ => reject "WRITEREF_NO_MUTABLE_REFERENCE_ERROR"
  | .mutBorrowGlobal d => borrowGlobal m view.handle.typeParameters stack true d
  | .immBorrowGlobal d => borrowGlobal m view.handle.typeParameters stack false d
  | .exists_ d => do
    let sd ← lookup m.structDefs d "struct_def_at"
    let a ← abilities m view.handle.typeParameters (.struct sd.structHandle)
    rejectIf (!a.key) "EXISTS_WITHOUT_KEY_ABILITY_OR_BAD_ARGUMENT"
    let (op, stack) ← popType stack
    rejectIf (op != .address) "EXISTS_WITHOUT_KEY_ABILITY_OR_BAD_ARGUMENT"
    pure (.bool :: stack)
  | .moveFrom d => do
    let sd ← lookup m.structDefs d "struct_def_at"
    let a ← abilities m view.handle.typeParameters (.struct sd.structHandle)
    rejectIf (!a.key) "MOVEFROM_WITHOUT_KEY_ABILITY"
    let (op, stack) ← popType stack
    rejectIf (op != .address) "MOVEFROM_TYPE_MISMATCH_ERROR"
    pure (.struct sd.structHandle :: stack)
  | .moveTo d => do
    let sd ← lookup m.structDefs d "struct_def_at"
    let a ← abilities m view.handle.typeParameters (.struct sd.structHandle)
    rejectIf (!a.key) "MOVETO_WITHOUT_KEY_ABILITY"
    let (keyOp, stack) ← popType stack
    let (signerOp, stack) ← popType stack
    rejectIf (keyOp != .struct sd.structHandle) "MOVETO_TYPE_MISMATCH_ERROR"
    rejectIf (signerOp != .reference .signer) "MOVETO_TYPE_MISMATCH_ERROR"
    pure stack
  | .vecLen idx => do
    let (op, stack) ← popType stack
    let elem ← elementType m idx
    if vectorRefElement op false == some elem then pure (.u64 :: stack) else reject "TYPE_MISMATCH"
  | .vecImmBorrow idx => do borrowVectorElement stack (← elementType m idx) false
  | .vecMutBorrow idx => do borrowVectorElement stack (← elementType m idx) true
  | .vecPushBack idx => do
    let (elemOp, stack) ← popType stack
    let (vecOp, stack) ← popType stack
    let elem ← elementType m idx
    rejectIf (!isAssignableFrom elem elemOp) "TYPE_MISMATCH"
    rejectIf (vectorRefElement vecOp true != some elem) "TYPE_MISMATCH"
    pure stack
  | .vecPopBack idx => do
    let (vecOp, stack) ← popType stack
    let elem ← elementType m idx
    if vectorRefElement vecOp true == some elem then pure (elem :: stack)
    else reject "TYPE_MISMATCH"
  | .vecSwap idx => do
    let (i2, stack) ← popType stack
    let (i1, stack) ← popType stack
    let (vecOp, stack) ← popType stack
    rejectIf (i1 != .u64 || i2 != .u64) "TYPE_MISMATCH"
    let elem ← elementType m idx
    rejectIf (vectorRefElement vecOp true != some elem) "TYPE_MISMATCH"
    pure stack
  | .callGeneric i => do
    let fi ← lookup m.functionInstantiations i "function_instantiation_at"
    let fh ← lookup m.functionHandles fi.handle "function_handle_at"
    typeCallGeneric m stack fh (← lookup m.signatures fi.typeParameters "signature_at")
  | .packGeneric i => do
    let (_, sd, tys) ← structInstParts m i
    typePackGeneric stack sd none tys
  | .unpackGeneric i => do
    let (_, sd, tys) ← structInstParts m i
    typeUnpackGeneric stack sd none tys
  | .packVariantGeneric i => do
    let (vh, sd, tys) ← variantInstParts m i
    typePackGeneric stack sd (some vh.variant) tys
  | .unpackVariantGeneric i => do
    let (vh, sd, tys) ← variantInstParts m i
    typeUnpackGeneric stack sd (some vh.variant) tys
  | .testVariantGeneric i => do
    let (_, sd, tys) ← variantInstParts m i
    let (arg, stack) ← popType stack
    match arg with
    | .reference inner | .mutableReference inner =>
      if inner == materializeType sd.structHandle tys then pure (.bool :: stack)
      else reject "TEST_VARIANT_TYPE_MISMATCH_ERROR"
    | _ => reject "TEST_VARIANT_TYPE_MISMATCH_ERROR"
  | .mutBorrowFieldGeneric i => do
    let fi ← lookup m.fieldInstantiations i "field_instantiation_at"
    borrowFieldGeneric m stack true false fi.handle
      (← lookup m.signatures fi.typeParameters "signature_at")
  | .immBorrowFieldGeneric i => do
    let fi ← lookup m.fieldInstantiations i "field_instantiation_at"
    borrowFieldGeneric m stack false false fi.handle
      (← lookup m.signatures fi.typeParameters "signature_at")
  | .mutBorrowVariantFieldGeneric i => do
    let fi ← lookup m.variantFieldInstantiations i "variant_field_instantiation_at"
    borrowFieldGeneric m stack true true fi.handle
      (← lookup m.signatures fi.typeParameters "signature_at")
  | .immBorrowVariantFieldGeneric i => do
    let fi ← lookup m.variantFieldInstantiations i "variant_field_instantiation_at"
    borrowFieldGeneric m stack false true fi.handle
      (← lookup m.signatures fi.typeParameters "signature_at")
  | .mutBorrowGlobalGeneric i => do
    let (d, _, tys) ← structInstParts m i
    borrowGlobalGeneric m view.handle.typeParameters stack true d tys
  | .immBorrowGlobalGeneric i => do
    let (d, _, tys) ← structInstParts m i
    borrowGlobalGeneric m view.handle.typeParameters stack false d tys
  | .existsGeneric i => do
    let (_, sd, tys) ← structInstParts m i
    let a ← abilities m view.handle.typeParameters (materializeType sd.structHandle tys)
    rejectIf (!a.key) "EXISTS_WITHOUT_KEY_ABILITY_OR_BAD_ARGUMENT"
    let (op, stack) ← popType stack
    rejectIf (op != .address) "EXISTS_WITHOUT_KEY_ABILITY_OR_BAD_ARGUMENT"
    pure (.bool :: stack)
  | .moveFromGeneric i => do
    let (_, sd, tys) ← structInstParts m i
    let a ← abilities m view.handle.typeParameters (materializeType sd.structHandle tys)
    rejectIf (!a.key) "MOVEFROM_WITHOUT_KEY_ABILITY"
    let (op, stack) ← popType stack
    rejectIf (op != .address) "MOVEFROM_TYPE_MISMATCH_ERROR"
    pure (materializeType sd.structHandle tys :: stack)
  | .moveToGeneric i => do
    let (_, sd, tys) ← structInstParts m i
    let a ← abilities m view.handle.typeParameters (materializeType sd.structHandle tys)
    rejectIf (!a.key) "MOVETO_WITHOUT_KEY_ABILITY"
    let (keyOp, stack) ← popType stack
    let (signerOp, stack) ← popType stack
    rejectIf (keyOp != materializeType sd.structHandle tys) "MOVETO_TYPE_MISMATCH_ERROR"
    rejectIf (signerOp != .reference .signer) "MOVETO_TYPE_MISMATCH_ERROR"
    pure stack
  | .packClosure h mask => do
    typeClosPack m view.handle.typeParameters stack
      (← lookup m.functionHandles h "function_handle_at") [] mask
  | .packClosureGeneric i mask => do
    let fi ← lookup m.functionInstantiations i "function_instantiation_at"
    let tys ← lookup m.signatures fi.typeParameters "signature_at"
    typeClosPack m view.handle.typeParameters stack
      (← lookup m.functionHandles fi.handle "function_handle_at") tys mask
  | .callClosure idx => do
    match (← lookup m.signatures idx "signature_at").head? with
    | some t => typeCallClosure stack t
    | none => panicAt "safe_unwrap!(signature_at(idx).0.first())"

/-- One instruction of `type_safety::verify`. -/
def typeStep (m : CompiledModule) (view : FunctionView) (stack : List SignatureToken)
    (offset : Nat) : VResult (List SignatureToken) := do
  typeInstr m view stack (← lookup view.code.code offset "code[offset]")

/-- The instructions of one block, from the stack the previous block left. -/
def typeBlock (m : CompiledModule) (view : FunctionView) (stack : List SignatureToken)
    (b : BasicBlock) : VResult (List SignatureToken) :=
  (List.range' b.start (b.exit + 1 - b.start)).foldlM (typeStep m view) stack

/-- `type_safety::verify`: every block in order, with one shared stack. -/
def verifyTypeSafety (m : CompiledModule) (view : FunctionView) : VResult Unit := do
  let _ ← view.cfg.blocks.foldlM (typeBlock m view) []
  pure ()

end MoveBytecode.Model
