-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Config
import Model.Abilities
import Model.Bounds

/-!
# Signatures

UNTRUSTED. Transcribes `move-bytecode-verifier/src/signature_v2.rs` for
modules: every type is well formed (references only where allowed, type
arguments of the right number satisfying their constraints, function types
with valid abilities) and every use of a type parameter respects the
constraints of the context it appears in; struct fields have the abilities
the struct's own abilities require, and phantom parameters appear only in
phantom positions.

Rust collects the abilities each check requires of type parameters in a
bitset and checks them against the context afterwards; the model collects a
list of (parameter, abilities) pairs. Rust caches intermediate results; every
cache entry holds what the same check would compute again, so the model
recomputes. The one cached result computed under different arguments is a
signature checked with references allowed, then reused where they are not;
each such use rejects a top-level reference first, so the decisions agree.
-/

namespace MoveBytecode.Model

namespace SignatureV2

/-- `AbilitySet::SIGNER`, `REFERENCES`, `VECTOR`, `ALL` and `EMPTY`. -/
def signerSet : AbilitySet := ⟨false, true, false, false⟩
def referencesSet : AbilitySet := ⟨true, true, false, false⟩
def vectorSet : AbilitySet := ⟨true, true, true, false⟩
def allSet : AbilitySet := ⟨true, true, true, true⟩
def emptySet : AbilitySet := ⟨false, false, false, false⟩

/-- The abilities required of type parameters. -/
abbrev Constraints := List (Nat × AbilitySet)

/-- `check_in_context`: every requirement is among the parameter's abilities
in the context, a parameter outside the context having none. -/
def checkInContext (cs : Constraints) (context : Nat → AbilitySet) : VResult Unit :=
  if cs.all fun (i, req) => req.subset (context i) then pure ()
  else reject "CONSTRAINT_NOT_SATISFIED"

def listContext (abilities : List AbilitySet) (i : Nat) : AbilitySet :=
  abilities[i]?.getD emptySet

def assertAbilities (abilities required : AbilitySet) : VResult Unit :=
  if required.subset abilities then pure () else reject "CONSTRAINT_NOT_SATISFIED"

def isReference : SignatureToken → Bool
  | .reference _ | .mutableReference _ => true
  | _ => false

variable (config : VerifierConfig) (m : CompiledModule)

mutual
/-- `check_ty`: the type is well formed, and the abilities it requires of
type parameters. -/
def checkTy : SignatureToken → Bool → AbilitySet → VResult Constraints
  | .typeParameter i, _, req => pure [(i, req)]
  | .bool, _, req | .u8, _, req | .u16, _, req | .u32, _, req | .u64, _, req | .u128, _, req
  | .u256, _, req | .i8, _, req | .i16, _, req | .i32, _, req | .i64, _, req | .i128, _, req
  | .i256, _, req | .address, _, req => do
    assertAbilities AbilitySet.primitives req
    pure []
  | .signer, _, req => do
    assertAbilities signerSet req
    pure []
  | .reference t, allowRef, req | .mutableReference t, allowRef, req =>
    if allowRef then do
      assertAbilities referencesSet req
      checkTy t false emptySet
    else reject "INVALID_SIGNATURE_TOKEN"
  | .vector t, _, req => do
    assertAbilities vectorSet req
    checkTy t false (requires req)
  | .function params results abilities, _, req => do
    assertAbilities abilities req
    if config.checkFunctionTypeAbilities && !abilities.subset publicFunctionAbilities then
      reject "CONSTRAINT_NOT_SATISFIED"
    if config.sigCheckerV2FixFunctionSignatures then
      pure ((← checkTyList params true emptySet) ++ (← checkTyList results true emptySet))
    else pure []
  | .struct h, _, req => do
    let sh ← lookup m.structHandles h "struct_handles[idx]"
    assertAbilities sh.abilities req
    pure []
  | .structInstantiation h args, _, req => do
    let sh ← lookup m.structHandles h "struct_handles[idx]"
    assertAbilities sh.abilities req
    if sh.typeParameters.length != args.length then reject "NUMBER_OF_TYPE_ARGUMENTS_MISMATCH"
    checkTyArgs sh.typeParameters args req

/-- Each type in a list, with the same arguments. -/
def checkTyList : List SignatureToken → Bool → AbilitySet → VResult Constraints
  | [], _, _ => pure []
  | t :: ts, allowRef, req => do
    pure ((← checkTy t allowRef req) ++ (← checkTyList ts allowRef req))

/-- The type arguments of a struct instantiation, each required to have its
parameter's constraints and, unless phantom, what the struct's required
abilities require. -/
def checkTyArgs : List StructTypeParameter → List SignatureToken → AbilitySet →
    VResult Constraints
  | p :: ps, a :: as, req => do
    let argReq := if p.isPhantom then p.constraints else p.constraints.union (requires req)
    pure ((← checkTy a false argReq) ++ (← checkTyArgs ps as req))
  | _, _, _ => pure []
end

/-- `check_ty_in_context`. -/
def checkTyInContext (context : Nat → AbilitySet) (ty : SignatureToken) (allowRef : Bool)
    (req : AbilitySet) : VResult Unit := do
  checkInContext (← checkTy config m ty allowRef req) context

/-- `verify_signature_contextless`, with references allowed (see the module
comment on caching). -/
def signatureConstraints (s : Nat) : VResult Constraints := do
  let sig ← lookup m.signatures s "signature_at"
  checkTyList config m sig true emptySet

/-- `verify_type_in_signature_contextless` for a type argument, after
rejecting a top-level reference. -/
def typeArg (ty : SignatureToken) (req : AbilitySet) : VResult Constraints := do
  if isReference ty then reject "INVALID_SIGNATURE_TOKEN"
  checkTy config m ty false req

/-- `verify_function_instantiation_contextless`. -/
def functionInstantiation (i : Nat) : VResult Constraints := do
  let fi ← lookup m.functionInstantiations i "function_instantiation_at"
  let fh ← lookup m.functionHandles fi.handle "function_handle_at"
  let args ← lookup m.signatures fi.typeParameters "signature_at"
  if fh.typeParameters.length != args.length then reject "NUMBER_OF_TYPE_ARGUMENTS_MISMATCH"
  let cs ← (args.zip fh.typeParameters).mapM fun (ty, req) => typeArg config m ty req
  pure cs.flatten

/-- `verify_struct_type_params`. -/
def structTypeParams (req : AbilitySet) (d : Nat) (argsIdx : Nat) : VResult Constraints := do
  let sd ← lookup m.structDefs d "struct_def_at"
  let sh ← lookup m.structHandles sd.structHandle "struct_handle_at"
  let args ← lookup m.signatures argsIdx "signature_at"
  if sh.typeParameters.length != args.length then reject "NUMBER_OF_TYPE_ARGUMENTS_MISMATCH"
  if !req.subset sh.abilities then reject "CONSTRAINT_NOT_SATISFIED"
  let cs ← (args.zip sh.typeParameters).mapM fun (ty, p) =>
    typeArg config m ty (if p.isPhantom then p.constraints
      else p.constraints.union (requires req))
  pure cs.flatten

def structInstantiation (i : Nat) (req : AbilitySet) : VResult Constraints := do
  let si ← lookup m.structDefInstantiations i "struct_instantiation_at"
  structTypeParams config m req si.def_ si.typeParameters

def structVariantInstantiation (i : Nat) (req : AbilitySet) : VResult Constraints := do
  let si ← lookup m.structVariantInstantiations i "struct_variant_instantiation_at"
  let vh ← lookup m.structVariantHandles si.handle "struct_variant_handle_at"
  structTypeParams config m req vh.structIndex si.typeParameters

def fieldInstantiation (i : Nat) : VResult Constraints := do
  let fi ← lookup m.fieldInstantiations i "field_instantiation_at"
  let fh ← lookup m.fieldHandles fi.handle "field_handle_at"
  structTypeParams config m emptySet fh.owner fi.typeParameters

def variantFieldInstantiation (i : Nat) : VResult Constraints := do
  let fi ← lookup m.variantFieldInstantiations i "variant_field_instantiation_at"
  let fh ← lookup m.variantFieldHandles fi.handle "variant_field_handle_at"
  structTypeParams config m emptySet fh.structIndex fi.typeParameters

/-- `verify_code`: the locals and every type an instruction instantiates,
in the context of the function's type parameters. -/
def verifyCode (abilities : List AbilitySet) (code : CodeUnit) : VResult Unit := do
  let ctx := listContext abilities
  checkInContext (← signatureConstraints config m code.locals) ctx
  code.code.toList.forM fun instr => match instr with
    | .callGeneric i | .packClosureGeneric i _ => do
      checkInContext (← functionInstantiation config m i) ctx
    | .packGeneric i | .unpackGeneric i => do
      checkInContext (← structInstantiation config m i emptySet) ctx
    | .packVariantGeneric i | .unpackVariantGeneric i | .testVariantGeneric i => do
      checkInContext (← structVariantInstantiation config m i emptySet) ctx
    | .existsGeneric i | .moveFromGeneric i | .moveToGeneric i | .immBorrowGlobalGeneric i
    | .mutBorrowGlobalGeneric i => do
      checkInContext (← structInstantiation config m i ⟨false, false, false, true⟩) ctx
    | .immBorrowFieldGeneric i | .mutBorrowFieldGeneric i => do
      checkInContext (← fieldInstantiation config m i) ctx
    | .immBorrowVariantFieldGeneric i | .mutBorrowVariantFieldGeneric i => do
      checkInContext (← variantFieldInstantiation config m i) ctx
    | .callClosure s => do
      match ← lookup m.signatures s "signature_at" with
      | [.function ..] => pure ()
      | _ => reject "CLOSURE_CALL_REQUIRES_FUNCTION"
    | .vecPack s _ | .vecLen s | .vecImmBorrow s | .vecMutBorrow s | .vecPushBack s
    | .vecPopBack s | .vecUnpack s _ | .vecSwap s => do
      let args ← lookup m.signatures s "signature_at"
      match args with
      | [ty] =>
        if isReference ty then reject "INVALID_SIGNATURE_TOKEN"
        checkInContext (← signatureConstraints config m s) ctx
      | _ => reject "NUMBER_OF_TYPE_ARGUMENTS_MISMATCH"
    | .pop | .ret | .branch _ | .brTrue _ | .brFalse _ | .ldU8 _ | .ldU16 _ | .ldU32 _
    | .ldU64 _ | .ldU128 _ | .ldU256 _ | .ldConst _ | .castU8 | .castU16 | .castU32 | .castU64
    | .castU128 | .castU256 | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _
    | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 | .ldTrue | .ldFalse
    | .call _ | .packClosure .. | .pack _ | .unpack _ | .testVariant _ | .packVariant _
    | .unpackVariant _ | .readRef | .writeRef | .freezeRef | .add | .sub | .mul | .mod | .div
    | .negate | .bitOr | .bitAnd | .xor | .shl | .shr | .or | .and | .not | .eq | .neq | .lt
    | .gt | .le | .ge | .copyLoc _ | .moveLoc _ | .stLoc _ | .mutBorrowLoc _ | .immBorrowLoc _
    | .mutBorrowField _ | .immBorrowField _ | .mutBorrowVariantField _
    | .immBorrowVariantField _ | .mutBorrowGlobal _ | .immBorrowGlobal _ | .exists_ _
    | .moveTo _ | .moveFrom _ | .abort | .abortMsg | .nop => pure ()

/-- Whether the phantom-position check looks through references. Rust's
`check_phantom_params` treats `Reference` and `MutableReference` as leaves,
so a phantom type parameter behind a reference in a field's function type is
not checked, unlike the Move compiler and the book's rule. The fix is expected
to come with a `VerifierConfig` switch; until that field exists, the model
follows the current verifier. -/
def phantomThroughReferences (_config : VerifierConfig) : Bool := false

mutual
/-- `check_phantom_params`. With `throughRefs`, a reference's target is checked
in a non-phantom position, like a vector's element. -/
def checkPhantom (context : List StructTypeParameter) (throughRefs : Bool) :
    SignatureToken → Bool → VResult Unit
  | .vector t, _ => checkPhantom context throughRefs t false
  | .function args results _, _ => do
    checkPhantomList context throughRefs args
    checkPhantomList context throughRefs results
  | .structInstantiation h args, _ => do
    let sh ← lookup m.structHandles h "struct_handles[idx]"
    checkPhantomArgs context throughRefs sh.typeParameters args 0
  | .typeParameter i, phantomPos => do
    let p ← lookup context.toArray i "context[idx]"
    if p.isPhantom && !phantomPos then reject "INVALID_PHANTOM_TYPE_PARAM_POSITION"
    else pure ()
  | .reference t, _ | .mutableReference t, _ =>
    if throughRefs then checkPhantom context throughRefs t false else pure ()
  | .struct _, _ | .bool, _ | .u8, _ | .u16, _ | .u32, _ | .u64, _ | .u128, _ | .u256, _
  | .i8, _ | .i16, _ | .i32, _ | .i64, _ | .i128, _ | .i256, _ | .address, _
  | .signer, _ => pure ()

/-- Each type of a list, in a non-phantom position. -/
def checkPhantomList (context : List StructTypeParameter) (throughRefs : Bool) :
    List SignatureToken → VResult Unit
  | [] => pure ()
  | t :: ts => do
    checkPhantom context throughRefs t false
    checkPhantomList context throughRefs ts

/-- The type arguments of a struct instantiation, each in a phantom position
exactly when its parameter is phantom. -/
def checkPhantomArgs (context : List StructTypeParameter) (throughRefs : Bool)
    (params : List StructTypeParameter) : List SignatureToken → Nat → VResult Unit
  | [], _ => pure ()
  | t :: ts, i => do
    let p ← lookup params.toArray i "type_parameters[i]"
    checkPhantom context throughRefs t p.isPhantom
    checkPhantomArgs context throughRefs params ts (i + 1)
end

/-- `verify_struct_def`. -/
def verifyStructDef (sd : StructDefinition) : VResult Unit := do
  let sh ← lookup m.structHandles sd.structHandle "struct_handle_at"
  let context := listContext (sh.typeParameters.map (·.constraints))
  let reqConditional := requires sh.abilities
  let contextAll (i : Nat) := if i < sh.typeParameters.length then allSet else emptySet
  let fields := match sd.fieldInformation with
    | .native => []
    | .declared fs => fs
    | .declaredVariants vs => vs.flatMap (fun (v : VariantDefinition) => v.fields)
  fields.forM fun f => do
    checkTyInContext config m context f.signature false emptySet
    match checkTyInContext config m contextAll f.signature false reqConditional with
    | .ok () => pure ()
    | .error _ => reject "FIELD_MISSING_TYPE_ABILITY"
    checkPhantom m sh.typeParameters (phantomThroughReferences config) f.signature false

/-- `max_num_of_ty_params_or_args`. -/
def maxTypeParams (m : CompiledModule) : Nat :=
  let tp (ts : List SignatureToken) := (preorderList ts).foldl (fun n t => match t with
    | .typeParameter i => Nat.max n (i + 1)
    | _ => n) 0
  let handles := m.functionHandles.toList.foldl (fun n fh => Nat.max n fh.typeParameters.length) 0
  let structs := m.structHandles.toList.foldl (fun n sh => Nat.max n sh.typeParameters.length) 0
  let sigs := m.signatures.toList.foldl (fun n sig => Nat.max n (tp sig)) 0
  let fields := m.structDefs.toList.foldl (fun n sd => Nat.max n (tp (match sd.fieldInformation with
    | .native => []
    | .declared fs => fs.map (·.signature)
    | .declaredVariants vs => vs.flatMap (·.fields.map (·.signature))))) 0
  Nat.max (Nat.max handles structs) (Nat.max sigs fields)


/-- `verify_module_impl`, in Rust's order. -/
def verify (config : VerifierConfig) (m : CompiledModule) : VResult Unit := do
  if maxTypeParams m > 256 then reject "UNKNOWN_INVARIANT_VIOLATION_ERROR"
  (List.range m.signatures.size).forM fun s => do
    let _ ← signatureConstraints config m s
  (List.range m.functionInstantiations.size).forM fun i => do
    let _ ← functionInstantiation config m i
  (List.range m.structDefInstantiations.size).forM fun i => do
    let _ ← structInstantiation config m i emptySet
  (List.range m.fieldInstantiations.size).forM fun i => do
    let _ ← fieldInstantiation config m i
  (List.range m.structVariantInstantiations.size).forM fun i => do
    let _ ← structVariantInstantiation config m i emptySet
  (List.range m.variantFieldInstantiations.size).forM fun i => do
    let _ ← variantFieldInstantiation config m i
  m.functionHandles.toList.forM fun fh => do
    let ctx := listContext fh.typeParameters
    checkInContext (← signatureConstraints config m fh.parameters) ctx
    checkInContext (← signatureConstraints config m fh.returns) ctx
  m.functionDefs.toList.forM fun fd =>
    match fd.code with
    | none => pure ()
    | some code => do
      let fh ← lookup m.functionHandles fd.function "function_handle_at"
      verifyCode config m fh.typeParameters code
  m.structDefs.toList.forM (verifyStructDef config m)

end SignatureV2

/-- `signature_v2::verify_module`. -/
def verifySignatures (config : VerifierConfig) (m : CompiledModule) : VResult Unit :=
  SignatureV2.verify config m

end MoveBytecode.Model
