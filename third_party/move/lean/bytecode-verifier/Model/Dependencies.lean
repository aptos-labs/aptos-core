-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Semantics.Program
import Model.Result

/-!
# Link checks

UNTRUSTED. Transcribes `move-bytecode-verifier/src/dependencies.rs` for
modules of bytecode version 5 and later: every imported module is among the
dependencies; every imported struct exists with compatible abilities and type
parameters; every imported function exists, may be called from this module
(public, or friend of a module that declares this one a friend), and has the
same type-parameter constraints, parameter and return types (structs compared
by module and name), and attributes. Handles of the module's own functions
and structs are not checked here.

Rust's maps become lists. Collecting the dependencies into a `BTreeMap`
keeps the last module with a given id; the model's lookups do the same.
-/

namespace MoveBytecode.Model

/-- `module_id_for_handle`, panicking on an index out of range. -/
def moduleIdForHandle (m : CompiledModule) (h : ModuleHandle) : VResult ModuleId := do
  pure ⟨← lookup m.addressIdentifiers h.address "address_identifier_at",
        ← lookup m.identifiers h.name "identifier_at"⟩

def selfModuleId (m : CompiledModule) : VResult ModuleId := do
  moduleIdForHandle m (← lookup m.moduleHandles m.selfModuleHandleIdx "self_handle")

/-- The dependency map: the dependencies other than the module itself, by id,
the last one winning. -/
def dependencyLookup (deps : List (ModuleId × CompiledModule)) (id : ModuleId) :
    Option CompiledModule :=
  (deps.filter (·.1 == id)).getLast?.map (·.2)

/-- `compatible_type_parameter_constraints`: the defined constraints are a
subset of the local ones. -/
def compatibleConstraints (local_ defined : AbilitySet) : Bool := defined.subset local_

def compatibleFunTypeParameters (local_ defined : List AbilitySet) : Bool :=
  local_.length == defined.length &&
    (local_.zip defined).all (fun (l, d) => compatibleConstraints l d)

def compatibleStructTypeParameters (local_ defined : List StructTypeParameter) : Bool :=
  local_.length == defined.length &&
    (local_.zip defined).all (fun (l, d) =>
      (!l.isPhantom || d.isPhantom) && compatibleConstraints l.constraints d.constraints)

/-- `compare_structs`: the same module id and struct name on both sides. -/
def compareStructs (m : CompiledModule) (idx1 : Nat) (defModule : CompiledModule) (idx2 : Nat) :
    VResult Unit := do
  let sh ← lookup m.structHandles idx1 "struct_handle_at"
  let mid ← moduleIdForHandle m (← lookup m.moduleHandles sh.module "module_handle_at")
  let name ← lookup m.identifiers sh.name "identifier_at"
  let dsh ← lookup defModule.structHandles idx2 "struct_handle_at"
  let dmid ← moduleIdForHandle defModule
    (← lookup defModule.moduleHandles dsh.module "module_handle_at")
  let dname ← lookup defModule.identifiers dsh.name "identifier_at"
  if mid != dmid || name != dname then reject "TYPE_MISMATCH" else pure ()

mutual
/-- `compare_types`. -/
def compareTypes (m defModule : CompiledModule) : SignatureToken → SignatureToken → VResult Unit
  | .bool, .bool | .u8, .u8 | .u16, .u16 | .u32, .u32 | .u64, .u64 | .u128, .u128
  | .u256, .u256 | .i8, .i8 | .i16, .i16 | .i32, .i32 | .i64, .i64 | .i128, .i128
  | .i256, .i256 | .address, .address | .signer, .signer => pure ()
  | .vector h, .vector d => compareTypes m defModule h d
  | .function ha hr hab, .function da dr dab => do
    compareSignatures m defModule ha da
    compareSignatures m defModule hr dr
    if hab != dab then reject "TYPE_MISMATCH"
  | .struct i1, .struct i2 => compareStructs m i1 defModule i2
  | .structInstantiation i1 a1, .structInstantiation i2 a2 => do
    compareStructs m i1 defModule i2
    compareSignatures m defModule a1 a2
  | .reference h, .reference d => compareTypes m defModule h d
  | .mutableReference h, .mutableReference d => compareTypes m defModule h d
  | .typeParameter i1, .typeParameter i2 => if i1 != i2 then reject "TYPE_MISMATCH" else pure ()
  | _, _ => reject "TYPE_MISMATCH"

/-- `compare_cross_module_signatures`. -/
def compareSignatures (m defModule : CompiledModule) :
    List SignatureToken → List SignatureToken → VResult Unit
  | [], [] => pure ()
  | h :: hs, d :: ds => do
    compareTypes m defModule h d
    compareSignatures m defModule hs ds
  | [], _ :: _ | _ :: _, [] => reject "TYPE_MISMATCH"
end

/-- `FunctionAttribute::cannot_be_removed`. -/
def cannotBeRemoved : FunctionAttribute → Bool
  | .moduleLock => false
  | .persistent | .pack | .packVariant _ | .unpack | .unpackVariant _ | .testVariant _
  | .borrowFieldImmutable _ | .borrowFieldMutable _ => true

/-- `FunctionAttribute::is_compatible_with`. -/
def attributesCompatible (handle defined : List FunctionAttribute) : Bool :=
  handle.all (fun a => !cannotBeRemoved a || defined.contains a)

/-- A function of a dependency this module may call: its handle and
definition indices. Like Rust's loop, a later match overrides an earlier
one. -/
def callableFunction (self : ModuleId) (dep : CompiledModule) (name : String) :
    VResult (Option (Nat × Nat)) := do
  let friends ← dep.friendDecls.toList.mapM (moduleIdForHandle dep)
  dep.functionDefs.toList.zipIdx.foldlM (fun found (fdef, idx) => do
    let fh ← lookup dep.functionHandles fdef.function "function_handle_at"
    let fname ← lookup dep.identifiers fh.name "identifier_at"
    let mayBeCalled := match fdef.visibility with
      | .public_ => true
      | .friend => friends.contains self
      | .private_ => false
    pure (if fname == name && mayBeCalled then some (fdef.function, idx) else found)) none

/-- The struct handle of a struct a dependency defines. -/
def definedStruct (dep : CompiledModule) (name : String) : VResult (Option Nat) :=
  dep.structDefs.toList.foldlM (fun found sd => do
    let sh ← lookup dep.structHandles sd.structHandle "struct_handle_at"
    let sname ← lookup dep.identifiers sh.name "identifier_at"
    pure (if sname == name then some sd.structHandle else found)) none

/-- `dependencies::verify_module` for bytecode version 5 and later. -/
def verifyDependencies (m : CompiledModule) (dependencies : List CompiledModule) :
    VResult Unit := do
  let self ← selfModuleId m
  let deps ← dependencies.foldlM (fun deps d => do
    let id ← selfModuleId d
    pure (if id != self then deps ++ [(id, d)] else deps)) []
  -- `Context::new` reads the version of the module of every imported function.
  m.functionHandles.toList.forM fun fh => do
    if fh.module != m.selfModuleHandleIdx then
      let id ← moduleIdForHandle m (← lookup m.moduleHandles fh.module "module_handle_at")
      if (dependencyLookup deps id).isNone then panicAt "dependency_map.get(..).unwrap()"
  -- `verify_imported_modules`
  m.moduleHandles.toList.zipIdx.forM fun (mh, idx) => do
    let id ← moduleIdForHandle m mh
    if idx != m.selfModuleHandleIdx && (dependencyLookup deps id).isNone then
      reject "MISSING_DEPENDENCY"
  -- `verify_imported_structs`
  m.structHandles.toList.forM fun sh => do
    if sh.module != m.selfModuleHandleIdx then
      let ownerId ← moduleIdForHandle m (← lookup m.moduleHandles sh.module "module_handle_at")
      let some owner := dependencyLookup deps ownerId | reject "UNKNOWN_INVARIANT_VIOLATION_ERROR"
      let name ← lookup m.identifiers sh.name "identifier_at"
      match ← definedStruct owner name with
      | some defIdx => do
        let dsh ← lookup owner.structHandles defIdx "struct_handle_at"
        if !sh.abilities.subset dsh.abilities ||
            !compatibleStructTypeParameters sh.typeParameters dsh.typeParameters then
          reject "TYPE_MISMATCH"
      | none => reject "LOOKUP_FAILED"
  -- `verify_imported_functions`
  m.functionHandles.toList.forM fun fh => do
    if fh.module != m.selfModuleHandleIdx then
      let ownerId ← moduleIdForHandle m (← lookup m.moduleHandles fh.module "module_handle_at")
      let name ← lookup m.identifiers fh.name "identifier_at"
      let some owner := dependencyLookup deps ownerId | reject "UNKNOWN_INVARIANT_VIOLATION_ERROR"
      match ← callableFunction self owner name with
      | some (ownerHandle, ownerDef) => do
        let dfh ← lookup owner.functionHandles ownerHandle "function_handle_at"
        if !compatibleFunTypeParameters fh.typeParameters dfh.typeParameters then
          reject "TYPE_MISMATCH"
        compareSignatures m owner (← lookup m.signatures fh.parameters "signature_at")
          (← lookup owner.signatures dfh.parameters "signature_at")
        compareSignatures m owner (← lookup m.signatures fh.returns "signature_at")
          (← lookup owner.signatures dfh.returns "signature_at")
        -- A public definition without attributes counts as `persistent`.
        let defAttrs ←
          if !fh.attributes.isEmpty && dfh.attributes.isEmpty then do
            if (← lookup owner.functionDefs ownerDef "function_def_at").visibility == .public_ then
              pure [.persistent]
            else pure dfh.attributes
          else pure dfh.attributes
        if !attributesCompatible fh.attributes defAttrs then reject "LINKER_ERROR"
      | none => reject "LOOKUP_FAILED"

end MoveBytecode.Model
