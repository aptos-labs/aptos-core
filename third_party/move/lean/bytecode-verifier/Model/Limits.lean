-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Syntax
import Spec.Config
import Model.Result

/-!
# Limits

UNTRUSTED. Transcribes `move-bytecode-verifier/src/limits.rs` for modules:
the configured bounds on type parameters, parameters, return values, type
size and depth, and the numbers of definitions, fields and variants. These
are acceptance limits (plan L11): soundness does not depend on them.
-/

namespace MoveBytecode.Model

/-- The weight of one token in the type-size count: structs and type
parameters may expand, so they weigh more. -/
def typeNodeWeight : SignatureToken → Nat
  | .struct _ | .structInstantiation .. => 4
  | .typeParameter _ => 4
  | .function .. => 1
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 | .i8 | .i16 | .i32 | .i64 | .i128
  | .i256 | .address | .signer | .vector _ | .reference _ | .mutableReference _ => 1

mutual
/-- `preorder_traversal_with_depth`, from depth `d`: a token, then its
children in order (for a function type, parameters then results) one level
deeper. -/
def preorderDepth : SignatureToken → Nat → List (SignatureToken × Nat)
  | .vector e, d => (.vector e, d) :: preorderDepth e (d + 1)
  | .reference e, d => (.reference e, d) :: preorderDepth e (d + 1)
  | .mutableReference e, d => (.mutableReference e, d) :: preorderDepth e (d + 1)
  | .structInstantiation h args, d =>
    (.structInstantiation h args, d) :: preorderDepthList args (d + 1)
  | .function ps rs a, d =>
    (.function ps rs a, d) :: (preorderDepthList ps (d + 1) ++ preorderDepthList rs (d + 1))
  | t, d => [(t, d)]

def preorderDepthList : List SignatureToken → Nat → List (SignatureToken × Nat)
  | [], _ => []
  | t :: ts, d => preorderDepth t d ++ preorderDepthList ts d
end

namespace Limits

variable (config : VerifierConfig)

def over (limit : Option Nat) (n : Nat) : Bool :=
  match limit with
  | some l => n > l
  | none => false

/-- `verify_function_handles`. -/
def functionHandles (m : CompiledModule) : VResult Unit :=
  m.functionHandles.toList.forM fun fh => do
    if over config.maxGenericInstantiationLength fh.typeParameters.length then
      reject "TOO_MANY_TYPE_PARAMETERS"
    if config.maxFunctionParameters.isSome then
      let ps ← lookup m.signatures fh.parameters "signature_at"
      if over config.maxFunctionParameters ps.length then reject "TOO_MANY_PARAMETERS"
    if config.maxFunctionReturnValues.isSome then
      let rs ← lookup m.signatures fh.returns "signature_at"
      if over config.maxFunctionReturnValues rs.length then reject "TOO_MANY_PARAMETERS"

/-- `verify_struct_handles`. -/
def structHandles (m : CompiledModule) : VResult Unit :=
  m.structHandles.toList.forM fun sh =>
    if over config.maxGenericInstantiationLength sh.typeParameters.length then
      reject "TOO_MANY_TYPE_PARAMETERS"
    else pure ()

/-- `verify_type_node`. -/
def typeNode (ty : SignatureToken) : VResult Unit := do
  if config.maxTypeNodes.isNone && config.maxFunctionParameters.isNone &&
      config.maxFunctionReturnValues.isNone && config.maxTypeDepth.isNone then
    return ()
  let size ← (preorderDepth ty 1).foldlM (fun size (tok, depth) => do
    if over config.maxTypeDepth depth then reject "TOO_MANY_TYPE_NODES"
    match tok with
    | .function ps rs _ =>
      if over config.maxFunctionParameters ps.length then reject "TOO_MANY_PARAMETERS"
      if over config.maxFunctionReturnValues rs.length then reject "TOO_MANY_PARAMETERS"
    | _ => pure ()
    pure (size + typeNodeWeight tok)) 0
  if over config.maxTypeNodes size then reject "TOO_MANY_TYPE_NODES"

/-- `verify_type_nodes`. -/
def typeNodes (m : CompiledModule) : VResult Unit := do
  m.signatures.toList.forM fun sig => sig.forM (typeNode config)
  m.constantPool.toList.forM fun c => typeNode config c.type
  m.structDefs.toList.forM fun sd =>
    match sd.fieldInformation with
    | .native => pure ()
    | .declared fields => fields.forM fun f => typeNode config f.signature
    | .declaredVariants variants =>
      variants.forM fun v => v.fields.forM fun f => typeNode config f.signature

/-- `verify_definitions`. -/
def definitions (m : CompiledModule) : VResult Unit := do
  if over config.maxFunctionDefinitions m.functionDefs.size then
    reject "MAX_FUNCTION_DEFINITIONS_REACHED"
  if over config.maxStructDefinitions m.structDefs.size then
    reject "MAX_STRUCT_DEFINITIONS_REACHED"
  if config.maxFieldsInStruct.isSome then
    m.structDefs.toList.forM fun sd => do
      let n := match sd.fieldInformation with
        | .native => 0
        | .declared fields => fields.length
        | .declaredVariants variants => variants.foldl (fun mx v => Nat.max mx v.fields.length) 0
      if over config.maxFieldsInStruct n then reject "MAX_FIELD_DEFINITIONS_REACHED"
  m.structDefs.toList.forM fun sd =>
    match sd.fieldInformation with
    | .declaredVariants variants =>
      if over config.maxStructVariants variants.length then reject "MAX_STRUCT_VARIANTS_REACHED"
      else pure ()
    | .native | .declared _ => pure ()

end Limits

/-- `LimitsVerifier::verify_module`. -/
def verifyLimits (config : VerifierConfig) (m : CompiledModule) : VResult Unit := do
  Limits.functionHandles config m
  Limits.structHandles config m
  Limits.typeNodes config m
  Limits.definitions config m

end MoveBytecode.Model
