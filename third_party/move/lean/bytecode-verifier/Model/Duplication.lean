-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Syntax
import Model.Result

/-!
# Duplication checking

UNTRUSTED. Transcribes `move-bytecode-verifier/src/check_duplication.rs` for
modules: no table has duplicate entries (handles by module and name), structs
have at least one field and no duplicate field or variant names, every
definition belongs to this module, and every handle of this module has a
definition.
-/

namespace MoveBytecode.Model

/-- `first_duplicate_element`: whether some element repeats an earlier one. -/
def hasDuplicate {α : Type} [BEq α] (xs : List α) : Bool :=
  let rec go : List α → List α → Bool
    | [], _ => false
    | x :: rest, seen => if seen.contains x then true else go rest (x :: seen)
  go xs []

def noDuplicate {α : Type} [BEq α] (xs : List α) : VResult Unit :=
  if hasDuplicate xs then reject "DUPLICATE_ELEMENT" else pure ()

def checkStructDefinitions (m : CompiledModule) : VResult Unit := do
  noDuplicate (m.structDefs.toList.map (·.structHandle))
  m.structDefs.toList.forM fun sd =>
    match sd.fieldInformation with
    | .native => pure ()
    | .declared fields => do
      if fields.isEmpty then reject "ZERO_SIZED_STRUCT"
      noDuplicate (fields.map (·.name))
    | .declaredVariants variants => do
      noDuplicate (variants.map (·.name))
      variants.forM fun v => noDuplicate (v.fields.map (·.name))
  m.structDefs.toList.forM fun sd => do
    if (← lookup m.structHandles sd.structHandle "struct_handle_at").module != m.selfModuleHandleIdx
    then reject "INVALID_MODULE_HANDLE"
  let implemented := m.structDefs.toList.map (·.structHandle)
  m.structHandles.toList.zipIdx.forM fun (sh, i) => do
    if sh.module == m.selfModuleHandleIdx && !implemented.contains i then
      reject "UNIMPLEMENTED_HANDLE"

def checkFunctionDefinitions (m : CompiledModule) : VResult Unit := do
  noDuplicate (m.functionDefs.toList.map (·.function))
  m.functionDefs.toList.forM fun fd => do
    if hasDuplicate fd.acquiresGlobalResources then reject "DUPLICATE_ACQUIRES_ANNOTATION"
  m.functionDefs.toList.forM fun fd => do
    if (← lookup m.functionHandles fd.function "function_handle_at").module != m.selfModuleHandleIdx
    then reject "INVALID_MODULE_HANDLE"
  let implemented := m.functionDefs.toList.map (·.function)
  m.functionHandles.toList.zipIdx.forM fun (fh, i) => do
    if fh.module == m.selfModuleHandleIdx && !implemented.contains i then
      reject "UNIMPLEMENTED_HANDLE"

/-- `DuplicationChecker::verify_module`. -/
def verifyDuplication (m : CompiledModule) : VResult Unit := do
  noDuplicate m.identifiers.toList
  noDuplicate m.addressIdentifiers.toList
  noDuplicate m.constantPool.toList
  noDuplicate m.signatures.toList
  noDuplicate m.moduleHandles.toList
  noDuplicate m.friendDecls.toList
  noDuplicate (m.structHandles.toList.map (fun h => (h.module, h.name)))
  noDuplicate (m.functionHandles.toList.map (fun h => (h.module, h.name)))
  noDuplicate m.functionInstantiations.toList
  noDuplicate m.fieldHandles.toList
  noDuplicate m.fieldInstantiations.toList
  noDuplicate m.variantFieldHandles.toList
  noDuplicate m.variantFieldInstantiations.toList
  noDuplicate m.structVariantHandles.toList
  noDuplicate m.structVariantInstantiations.toList
  checkFunctionDefinitions m
  checkStructDefinitions m
  noDuplicate m.structDefInstantiations.toList

end MoveBytecode.Model
