-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Syntax
import Model.Result

/-!
# The acquires list

UNTRUSTED. Transcribes `move-bytecode-verifier/src/acquires_list_verifier.rs`.
A function must annotate every struct it moves from or borrows globally, and
every struct a function of this module it calls annotates. Every annotation
must be used that way and name a struct with `key`.

Rust collects the annotations and the uses as sets; the model keeps them as
lists and asks only membership, so the order and repetitions do not matter.
The order in which Rust checks the annotations decides only which error it
reports.
-/

namespace MoveBytecode.Model

namespace Acquires

/-- `handle_to_def`: the definition of a function handle of this module, the
last one winning. -/
def handleToDef (m : CompiledModule) (h : Nat) : Option FunctionDefinition :=
  (m.functionDefs.toList.filter (·.function == h)).getLast?

/-- `function_acquired_resources`: the annotations of a callee defined in
this module, and none for a callee of another module. -/
def functionAcquires (m : CompiledModule) (h : Nat) : VResult (List Nat) := do
  let fh ← lookup m.functionHandles h "function_handle_at"
  if fh.module != m.selfModuleHandleIdx then return []
  pure ((handleToDef m h).map (·.acquiresGlobalResources) |>.getD [])

/-- `call_acquire`: the callee's annotations must be the caller's too. -/
def callAcquire (m : CompiledModule) (annotated actual : List Nat) (h : Nat) :
    VResult (List Nat) := do
  let acquired ← functionAcquires m h
  if acquired.any (fun s => !annotated.contains s) then reject "MISSING_ACQUIRES_ANNOTATION"
  pure (acquired ++ actual)

/-- `struct_acquire`. -/
def structAcquire (annotated actual : List Nat) (s : Nat) : VResult (List Nat) :=
  if annotated.contains s then pure (s :: actual) else reject "MISSING_ACQUIRES_ANNOTATION"

/-- `verify_instruction`: the structs acquired so far after an instruction. -/
def instr (m : CompiledModule) (annotated actual : List Nat) : Bytecode → VResult (List Nat)
  | .call h => callAcquire m annotated actual h
  | .callGeneric i => do
    callAcquire m annotated actual
      (← lookup m.functionInstantiations i "function_instantiation_at").handle
  | .moveFrom d | .mutBorrowGlobal d | .immBorrowGlobal d => structAcquire annotated actual d
  | .moveFromGeneric i | .mutBorrowGlobalGeneric i | .immBorrowGlobalGeneric i => do
    structAcquire annotated actual
      (← lookup m.structDefInstantiations i "struct_instantiation_at").def_
  | .packClosure _ _ | .packClosureGeneric _ _ | .callClosure _ | .pop | .brTrue _
  | .brFalse _ | .abort | .abortMsg | .branch _ | .nop | .ret | .stLoc _ | .moveLoc _
  | .copyLoc _ | .immBorrowLoc _ | .mutBorrowLoc _ | .freezeRef | .mutBorrowField _
  | .mutBorrowFieldGeneric _ | .immBorrowField _ | .immBorrowFieldGeneric _
  | .mutBorrowVariantField _ | .mutBorrowVariantFieldGeneric _ | .immBorrowVariantField _
  | .immBorrowVariantFieldGeneric _ | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _
  | .ldU256 _ | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _
  | .ldConst _ | .ldTrue | .ldFalse | .pack _ | .packGeneric _ | .unpack _ | .unpackGeneric _
  | .packVariant _ | .packVariantGeneric _ | .unpackVariant _ | .unpackVariantGeneric _
  | .testVariant _ | .testVariantGeneric _ | .readRef | .writeRef | .castU8 | .castU16
  | .castU32 | .castU64 | .castU128 | .castU256 | .castI8 | .castI16 | .castI32 | .castI64
  | .castI128 | .castI256 | .add | .sub | .mul | .mod | .div | .negate | .bitOr | .bitAnd
  | .xor | .shl | .shr | .or | .and | .not | .eq | .neq | .lt | .gt | .le | .ge | .exists_ _
  | .existsGeneric _ | .moveTo _ | .moveToGeneric _ | .vecPack _ _ | .vecLen _
  | .vecImmBorrow _ | .vecMutBorrow _ | .vecPushBack _ | .vecPopBack _ | .vecUnpack _ _
  | .vecSwap _ => pure actual

/-- The check of one annotation once the code is scanned. -/
def annotation (m : CompiledModule) (actual : List Nat) (s : Nat) : VResult Unit := do
  if !actual.contains s then reject "EXTRANEOUS_ACQUIRES_ANNOTATION"
  let sd ← match m.structDefs[s]? with
    | some sd => pure sd
    | none => panicAt "struct_defs().get"
  let sh ← lookup m.structHandles sd.structHandle "struct_handle_at"
  if !sh.abilities.key then reject "INVALID_ACQUIRES_ANNOTATION"

end Acquires

/-- `AcquiresVerifier::verify`, for a function with code. -/
def verifyAcquires (m : CompiledModule) (fdef : FunctionDefinition) (code : CodeUnit) :
    VResult Unit := do
  let annotated := fdef.acquiresGlobalResources
  let actual ← code.code.toList.foldlM (Acquires.instr m annotated) []
  annotated.forM (Acquires.annotation m actual)

end MoveBytecode.Model
