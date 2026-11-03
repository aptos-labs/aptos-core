-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Syntax
import Model.Result

/-!
# Instruction consistency

UNTRUSTED. Transcribes `move-bytecode-verifier/src/instruction_consistency.rs`
for modules: an instruction that comes in a generic and a non-generic form
uses the generic form exactly for a generic function, struct or enum; a
closure mask captures only parameters; and `VecPack`/`VecUnpack` counts fit
in a `u16`.

Rust's `*_at` accessors either panic or return an index error on an index out
of range; the bounds checker has ruled both out, and the model treats both as
a `panic` result.
-/

namespace MoveBytecode.Model

/-- `ClosureMask::max_captured`: the highest captured parameter, if any. -/
def maxCaptured (mask : BitVec 64) : Option Nat :=
  (List.range 64).foldl (fun acc i => if mask.getLsbD i then some i else acc) none

namespace InstructionConsistency

variable (m : CompiledModule)

/-- `check_struct_op`. -/
def checkStructOp (d : Nat) (generic : Bool) : VResult Unit := do
  let sd ← lookup m.structDefs d "struct_def_at"
  let sh ← lookup m.structHandles sd.structHandle "struct_handle_at"
  if sh.typeParameters.isEmpty == generic then reject "GENERIC_MEMBER_OPCODE_MISMATCH"
  else pure ()

/-- `check_variant_op`. -/
def checkVariantOp (h : Nat) (generic : Bool) : VResult Unit := do
  let vh ← lookup m.structVariantHandles h "struct_variant_handle_at"
  checkStructOp m vh.structIndex generic

/-- `check_function_op`. -/
def checkFunctionOp (h : Nat) (generic : Bool) : VResult Unit := do
  let fh ← lookup m.functionHandles h "function_handle_at"
  if fh.typeParameters.isEmpty == generic then reject "GENERIC_MEMBER_OPCODE_MISMATCH"
  else pure ()

/-- `check_closure_mask`. -/
def checkClosureMask (h : Nat) (mask : BitVec 64) : VResult Unit := do
  let fh ← lookup m.functionHandles h "function_handle_at"
  let sig ← lookup m.signatures fh.parameters "signature_at"
  match maxCaptured mask with
  | some mx => if mx ≥ sig.length then reject "INVALID_CLOSURE_MASK" else pure ()
  | none => pure ()

/-- The check of one instruction in `check_instructions`. -/
def checkInstr : Bytecode → VResult Unit
  | .mutBorrowField h | .immBorrowField h => do
    checkStructOp m (← lookup m.fieldHandles h "field_handle_at").owner false
  | .mutBorrowFieldGeneric i | .immBorrowFieldGeneric i => do
    let fi ← lookup m.fieldInstantiations i "field_instantiation_at"
    checkStructOp m (← lookup m.fieldHandles fi.handle "field_handle_at").owner true
  | .mutBorrowVariantField h | .immBorrowVariantField h => do
    checkStructOp m (← lookup m.variantFieldHandles h "variant_field_handle_at").structIndex false
  | .mutBorrowVariantFieldGeneric i | .immBorrowVariantFieldGeneric i => do
    let fi ← lookup m.variantFieldInstantiations i "variant_field_instantiation_at"
    checkStructOp m
      (← lookup m.variantFieldHandles fi.handle "variant_field_handle_at").structIndex true
  | .call h => checkFunctionOp m h false
  | .callGeneric i => do
    checkFunctionOp m (← lookup m.functionInstantiations i "function_instantiation_at").handle true
  | .packClosure h mask => do
    checkFunctionOp m h false
    checkClosureMask m h mask
  | .packClosureGeneric i mask => do
    let fi ← lookup m.functionInstantiations i "function_instantiation_at"
    checkFunctionOp m fi.handle true
    checkClosureMask m fi.handle mask
  | .pack d | .unpack d => checkStructOp m d false
  | .packGeneric i | .unpackGeneric i => do
    checkStructOp m (← lookup m.structDefInstantiations i "struct_instantiation_at").def_ true
  | .packVariant h | .unpackVariant h | .testVariant h => checkVariantOp m h false
  | .packVariantGeneric i | .unpackVariantGeneric i | .testVariantGeneric i => do
    checkVariantOp m
      (← lookup m.structVariantInstantiations i "struct_variant_instantiation_at").handle true
  | .mutBorrowGlobal d | .immBorrowGlobal d => checkStructOp m d false
  | .mutBorrowGlobalGeneric i | .immBorrowGlobalGeneric i => do
    checkStructOp m (← lookup m.structDefInstantiations i "struct_instantiation_at").def_ true
  | .exists_ d | .moveFrom d | .moveTo d => checkStructOp m d false
  | .existsGeneric i | .moveFromGeneric i | .moveToGeneric i => do
    checkStructOp m (← lookup m.structDefInstantiations i "struct_instantiation_at").def_ true
  | .vecPack _ n | .vecUnpack _ n =>
    if n > 65535 then reject "CONSTRAINT_NOT_SATISFIED" else pure ()
  | .callClosure _ | .freezeRef | .pop | .ret | .branch _ | .brTrue _ | .brFalse _
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ | .ldConst _
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 | .ldTrue | .ldFalse
  | .readRef | .writeRef | .add | .sub | .mul | .mod | .div | .negate | .bitOr | .bitAnd
  | .xor | .shl | .shr | .or | .and | .not | .eq | .neq | .lt | .gt | .le | .ge | .copyLoc _
  | .moveLoc _ | .stLoc _ | .mutBorrowLoc _ | .immBorrowLoc _ | .vecLen _
  | .vecImmBorrow _ | .vecMutBorrow _ | .vecPushBack _ | .vecPopBack _
  | .vecSwap _ | .abort | .abortMsg | .nop | .ldI8 _ | .ldI16 _ | .ldI32 _
  | .ldI64 _ | .ldI128 _ | .ldI256 _ | .castI8 | .castI16 | .castI32 | .castI64
  | .castI128 | .castI256 => pure ()

end InstructionConsistency

/-- `InstructionConsistency::verify_module`. -/
def verifyInstructionConsistency (m : CompiledModule) : VResult Unit :=
  m.functionDefs.toList.forM fun fd =>
    match fd.code with
    | none => pure ()
    | some code => code.code.toList.forM (InstructionConsistency.checkInstr m)

end MoveBytecode.Model
