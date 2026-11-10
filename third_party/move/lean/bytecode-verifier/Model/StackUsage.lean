-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Config
import Model.Cfg

/-!
# Stack usage

UNTRUSTED. Transcribes `move-bytecode-verifier/src/stack_usage_verifier.rs`
and the `FunctionView` it reads (`move-binary-format/src/binary_views.rs`):
every basic block, reachable or not, starts and ends with an empty operand
stack, never pops more than it has pushed, and stays within the configured
stack size and push bounds.

Rust performs these checks with checked `u64` arithmetic; the model uses
natural numbers. With the configured bounds no `u64` overflow is reachable
before a bound is exceeded, except an overflowing push count, which Rust
rejects and the model rejects through `max_push_size`.
-/

namespace MoveBytecode.Model

/-- The parts of `FunctionView` the code-unit passes read. -/
structure FunctionView where
  index : Option Nat
  code : CodeUnit
  handle : FunctionHandle
  parameters : Signature
  returns : Signature
  locals : Signature
  cfg : Cfg
  deriving Inhabited

/-- `FunctionView::function`, with its control-flow graph. -/
def FunctionView.function (m : CompiledModule) (index : Nat) (code : CodeUnit)
    (handle : FunctionHandle) (cfg : Cfg) : VResult FunctionView := do
  pure { index := some index, code, handle, cfg,
         parameters := ← lookup m.signatures handle.parameters "signature_at(parameters)"
         returns := ← lookup m.signatures handle.returns "signature_at(return_)"
         locals := ← lookup m.signatures code.locals "signature_at(locals)" }

/-- `ClosureMask::captured_count`: the set bits of the mask. -/
def capturedCount (mask : BitVec 64) : Nat :=
  (List.range 64).countP (fun i => mask.getLsbD i)

/-- `StructFieldInformation::field_count`. -/
def fieldCount (info : StructFieldInformation) (variant : Option Nat) : Nat :=
  match info, variant with
  | .native, _ => 0
  | .declared fields, _ => fields.length
  | .declaredVariants _, none => 0
  | .declaredVariants variants, some v =>
    match variants[v]? with
    | some vd => vd.fields.length
    | none => 0

/-- `instruction_effect`: how many values an instruction pops and pushes. -/
def instructionEffect (m : CompiledModule) (returns : Signature) : Bytecode → VResult (Nat × Nat)
  | .pop | .brTrue _ | .brFalse _ | .stLoc _ | .abort => pure (1, 0)
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _
  | .ldTrue | .ldFalse | .ldConst _ | .copyLoc _ | .moveLoc _ | .mutBorrowLoc _
  | .immBorrowLoc _ => pure (0, 1)
  | .not | .negate | .freezeRef | .readRef | .exists_ _ | .existsGeneric _
  | .mutBorrowGlobal _ | .mutBorrowGlobalGeneric _ | .immBorrowGlobal _ | .immBorrowGlobalGeneric _
  | .mutBorrowField _ | .mutBorrowFieldGeneric _ | .immBorrowField _ | .immBorrowFieldGeneric _
  | .mutBorrowVariantField _ | .mutBorrowVariantFieldGeneric _ | .immBorrowVariantField _
  | .immBorrowVariantFieldGeneric _ | .testVariant _ | .testVariantGeneric _
  | .moveFrom _ | .moveFromGeneric _
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256
  | .vecLen _ | .vecPopBack _ => pure (1, 1)
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .shl | .shr | .or | .and
  | .eq | .neq | .lt | .gt | .le | .ge => pure (2, 1)
  | .vecPack _ n => pure (n, 1)
  | .vecUnpack _ n => pure (1, n)
  | .vecImmBorrow _ | .vecMutBorrow _ => pure (2, 1)
  | .moveTo _ | .moveToGeneric _ | .writeRef | .vecPushBack _ | .abortMsg => pure (2, 0)
  | .vecSwap _ => pure (3, 0)
  | .branch _ | .nop => pure (0, 0)
  | .ret => pure (returns.length, 0)
  | .call h => do
    let fh ← lookup m.functionHandles h "function_handle_at"
    pure ((← lookup m.signatures fh.parameters "signature_at").length,
          (← lookup m.signatures fh.returns "signature_at").length)
  | .callGeneric i => do
    let fi ← lookup m.functionInstantiations i "function_instantiation_at"
    let fh ← lookup m.functionHandles fi.handle "function_handle_at"
    pure ((← lookup m.signatures fh.parameters "signature_at").length,
          (← lookup m.signatures fh.returns "signature_at").length)
  | .callClosure s => do
    match (← lookup m.signatures s "signature_at").head? with
    | some (.function args results _) => pure (1 + args.length, results.length)
    -- Rust: "the signature checker v2 ensures we never reach this".
    | _ => pure (0, 0)
  | .packClosure _ mask | .packClosureGeneric _ mask => pure (capturedCount mask, 1)
  | .pack d => do
    pure (fieldCount (← lookup m.structDefs d "struct_def_at").fieldInformation none, 1)
  | .packGeneric i => do
    let si ← lookup m.structDefInstantiations i "struct_instantiation_at"
    pure (fieldCount (← lookup m.structDefs si.def_ "struct_def_at").fieldInformation none, 1)
  | .packVariant h => do
    let vh ← lookup m.structVariantHandles h "struct_variant_handle_at"
    let sd ← lookup m.structDefs vh.structIndex "struct_def_at"
    pure (fieldCount sd.fieldInformation (some vh.variant), 1)
  | .packVariantGeneric i => do
    let vi ← lookup m.structVariantInstantiations i "struct_variant_instantiation_at"
    let vh ← lookup m.structVariantHandles vi.handle "struct_variant_handle_at"
    let sd ← lookup m.structDefs vh.structIndex "struct_def_at"
    pure (fieldCount sd.fieldInformation (some vh.variant), 1)
  | .unpack d => do
    pure (1, fieldCount (← lookup m.structDefs d "struct_def_at").fieldInformation none)
  | .unpackGeneric i => do
    let si ← lookup m.structDefInstantiations i "struct_instantiation_at"
    pure (1, fieldCount (← lookup m.structDefs si.def_ "struct_def_at").fieldInformation none)
  | .unpackVariant h => do
    let vh ← lookup m.structVariantHandles h "struct_variant_handle_at"
    let sd ← lookup m.structDefs vh.structIndex "struct_def_at"
    pure (1, fieldCount sd.fieldInformation (some vh.variant))
  | .unpackVariantGeneric i => do
    let vi ← lookup m.structVariantInstantiations i "struct_variant_instantiation_at"
    let vh ← lookup m.structVariantHandles vi.handle "struct_variant_handle_at"
    let sd ← lookup m.structDefs vh.structIndex "struct_def_at"
    pure (1, fieldCount sd.fieldInformation (some vh.variant))

/-- The running state of `verify_block`: the stack height relative to the
block start, and the pushes so far. -/
structure BlockState where
  height : Nat
  pushes : Nat
  deriving DecidableEq, Repr

/-- One instruction of `verify_block`. -/
def stackStep (config : VerifierConfig) (m : CompiledModule) (returns : Signature)
    (code : Array Bytecode) (st : BlockState) (pc : Nat) : VResult BlockState := do
  let (pops, pushes) ← instructionEffect m returns (← lookup code pc "code[i]")
  let overall := st.pushes + pushes
  match config.maxPushSize with
  | some maxPush => if overall > maxPush then reject "VALUE_STACK_PUSH_OVERFLOW"
  | none => pure ()
  if st.height < pops then reject "NEGATIVE_STACK_SIZE_WITHIN_BLOCK"
  let height := st.height - pops + pushes
  if height > config.maxValueStackSize then reject "VALUE_STACK_OVERFLOW"
  pure ⟨height, overall⟩

/-- `verify_block`: the instructions `start..=exit` from an empty stack, which
must be empty again at the end. -/
def verifyBlock (config : VerifierConfig) (m : CompiledModule) (returns : Signature)
    (code : Array Bytecode) (b : BasicBlock) : VResult Unit := do
  let st ← (List.range' b.start (b.exit + 1 - b.start)).foldlM
    (stackStep config m returns code) ⟨0, 0⟩
  if st.height == 0 then pure () else reject "POSITIVE_STACK_SIZE_AT_BLOCK_END"

/-- `StackUsageVerifier::verify`: every block of the control-flow graph. -/
def verifyStackUsage (config : VerifierConfig) (m : CompiledModule) (view : FunctionView) :
    VResult Unit :=
  view.cfg.blocks.forM (verifyBlock config m view.returns view.code.code)

end MoveBytecode.Model
