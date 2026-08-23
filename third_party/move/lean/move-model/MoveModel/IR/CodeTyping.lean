-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Syntax
import MoveModel.IR.Semantics
import MoveModel.IR.Util

set_option maxHeartbeats 0

/-!
# Source Code Typing and Preservation

This module defines the semantic typing discipline for the bytecode accepted
by the prover.  It represents the guarantees expected from the bytecode
verifier.

The adequacy proof uses these facts at opaque calls.  `SatisfiesContract` and
the `callRel` havoc apply only at well-typed boundaries: arguments and memory
must be well typed before the call, and results must be well typed afterward.
The source typing rules establish this preservation property.

The judgment is semantic where possible.  `TySub` defines subtyping as
inclusion between `IsValid` value sets.  This accounts for transparent
reference types and the runtime identification of signers with addresses.

Reference instructions have source-level typing rules even though the direct
prover translation maps them to failing assertions (`refFail`).  The rules
record the verifier facts later consumed by reference elimination.  In the
direct simulation, reference operations take the assertion-failure branch.

`TypedLocals` is the runtime invariant threaded through the simulation:
every *defined* local is well-formed for its declared type.
-/

namespace MoveModel.IR

/-- Semantic subtyping: every value valid at `t` is valid at `t'`. -/
def TySub (Δ : StructDecls) (t t' : Ty) : Prop :=
  ∀ v, IsValid Δ t v → IsValid Δ t' v

/-- Semantic type inclusion is reflexive. -/
theorem TySub.refl (Δ : StructDecls) (t : Ty) : TySub Δ t t := fun _ h => h

/-- Bytecode-level assignment compatibility.  `TySub` is deliberately
reference-transparent because it models the prover boundary; within
bytecode, however, a reference-bearing type must match exactly. -/
structure CodeTySub (Δ : StructDecls) (t t' : Ty) : Prop where
  semantic : TySub Δ t t'
  refExact : t.isRef || t'.isRef → t = t'

/-- Code-level type inclusion is reflexive. -/
theorem CodeTySub.refl (Δ : StructDecls) (t : Ty) : CodeTySub Δ t t :=
  ⟨TySub.refl Δ t, fun _ => rfl⟩

/-- Pointwise semantic subtyping of type lists. -/
inductive TySubs (Δ : StructDecls) : List Ty → List Ty → Prop where
  | nil : TySubs Δ [] []
  | cons {t t' : Ty} {ts ts' : List Ty} :
      TySub Δ t t' → TySubs Δ ts ts' → TySubs Δ (t :: ts) (t' :: ts')

/-- Pointwise bytecode-level assignment compatibility. -/
inductive CodeTySubs (Δ : StructDecls) : List Ty → List Ty → Prop where
  | nil : CodeTySubs Δ [] []
  | cons {t t' : Ty} {ts ts' : List Ty} :
      CodeTySub Δ t t' → CodeTySubs Δ ts ts' →
      CodeTySubs Δ (t :: ts) (t' :: ts')

/-- Pointwise code-type inclusion implies semantic list-type inclusion. -/
theorem CodeTySubs.semantic { Δ : StructDecls } :
    ∀ {ts ts' : List Ty}, CodeTySubs Δ ts ts' → TySubs Δ ts ts'
  | [], [], .nil => .nil
  | _ :: _, _ :: _, .cons h hs => .cons h.semantic hs.semantic

/-- Operation typing: operand types and result types of the non-call,
non-reference operations. -/
inductive WfOp (Δ : StructDecls) : Oper → List Ty → List Ty → Prop where
  -- one rule per integer operation: operands and result share the operand's
  -- `NumType`; shifts take a `u8` amount; `cast` maps any source `NumType` to
  -- any target (cross-sign casts included — the checked semantics enforces the
  -- target range).
  | add (nt : NumType) : WfOp Δ (.add nt) [.int nt, .int nt] [.int nt]
  | sub (nt : NumType) : WfOp Δ (.sub nt) [.int nt, .int nt] [.int nt]
  | mul (nt : NumType) : WfOp Δ (.mul nt) [.int nt, .int nt] [.int nt]
  | div (nt : NumType) : WfOp Δ (.div nt) [.int nt, .int nt] [.int nt]
  | mod (nt : NumType) : WfOp Δ (.mod nt) [.int nt, .int nt] [.int nt]
  | bitAnd (nt : NumType) : WfOp Δ (.bitAnd nt) [.int nt, .int nt] [.int nt]
  | bitOr (nt : NumType) : WfOp Δ (.bitOr nt) [.int nt, .int nt] [.int nt]
  | bitXor (nt : NumType) : WfOp Δ (.bitXor nt) [.int nt, .int nt] [.int nt]
  | shl (nt : NumType) : WfOp Δ (.shl nt) [.int nt, .uint .w8] [.int nt]
  | shr (nt : NumType) : WfOp Δ (.shr nt) [.int nt, .uint .w8] [.int nt]
  | cast (src target : NumType) :
      WfOp Δ (.cast target) [.int src] [.int target]
  | lt (nt : NumType) : WfOp Δ .lt [.int nt, .int nt] [.bool]
  | le (nt : NumType) : WfOp Δ .le [.int nt, .int nt] [.bool]
  | eq (t : Ty) : WfOp Δ .eq [t, t] [.bool]
  | and : WfOp Δ .and [.bool, .bool] [.bool]
  | or : WfOp Δ .or [.bool, .bool] [.bool]
  | not : WfOp Δ .not [.bool] [.bool]
  | pack {r : ResourceId} {sd : StructDecl} :
      Δ r = some sd → WfOp Δ .pack sd.fields [.struct r]
  | packInst {r : ResourceId} {sd : StructDecl} {args : List Ty} :
      Δ r = some sd → args.length = sd.typeParams.length →
      WfOp Δ (.packInst args) (instantiateTypes args sd.fields)
        [.structInst r args]
  | unpack {r : ResourceId} {sd : StructDecl} :
      Δ r = some sd → WfOp Δ .unpack [.struct r] sd.fields
  | unpackInst {r : ResourceId} {sd : StructDecl} {args : List Ty} :
      Δ r = some sd → args.length = sd.typeParams.length →
      WfOp Δ (.unpackInst args) [.structInst r args]
        (instantiateTypes args sd.fields)
  | packVariant {r : ResourceId} {sd : StructDecl} {variants : List (List Ty)}
      {variant : Nat} {fields : List Ty} :
      Δ r = some sd → sd.variants = some variants → variants[variant]? = some fields →
      WfOp Δ (.packVariant variant) fields [.enum r]
  | unpackVariant {r : ResourceId} {sd : StructDecl} {variants : List (List Ty)}
      {variant : Nat} {fields : List Ty} :
      Δ r = some sd → sd.variants = some variants → variants[variant]? = some fields →
      WfOp Δ (.unpackVariant variant) [.enum r] fields
  | testVariant {r : ResourceId} {sd : StructDecl} {variants : List (List Ty)}
      {variant : Nat} {fields : List Ty} :
      Δ r = some sd → sd.variants = some variants → variants[variant]? = some fields →
      WfOp Δ (.testVariant variant) [.enum r] [.bool]
  | packVariantInst {r : ResourceId} {sd : StructDecl}
      {variants : List (List Ty)} {variant : Nat} {fields args : List Ty} :
      Δ r = some sd → args.length = sd.typeParams.length →
      sd.variants = some variants → variants[variant]? = some fields →
      WfOp Δ (.packVariantInst variant args)
        (instantiateTypes args fields) [.enumInst r args]
  | unpackVariantInst {r : ResourceId} {sd : StructDecl}
      {variants : List (List Ty)} {variant : Nat} {fields args : List Ty} :
      Δ r = some sd → args.length = sd.typeParams.length →
      sd.variants = some variants → variants[variant]? = some fields →
      WfOp Δ (.unpackVariantInst variant args) [.enumInst r args]
        (instantiateTypes args fields)
  | testVariantInst {r : ResourceId} {sd : StructDecl}
      {variants : List (List Ty)} {variant : Nat} {fields args : List Ty} :
      Δ r = some sd → args.length = sd.typeParams.length →
      sd.variants = some variants → variants[variant]? = some fields →
      WfOp Δ (.testVariantInst variant args) [.enumInst r args] [.bool]
  | getField {r : ResourceId} {sd : StructDecl} {i : Nat} {t : Ty} :
      Δ r = some sd → sd.fields[i]? = some t →
      WfOp Δ (.getField i) [.struct r] [t]
  | getFieldInst {r : ResourceId} {sd : StructDecl} {i : Nat} {t : Ty}
      {args : List Ty} :
      Δ r = some sd → args.length = sd.typeParams.length →
      sd.fields[i]? = some t →
      WfOp Δ (.getFieldInst i args) [.structInst r args]
        [t.instantiate args]
  | updateField {r : ResourceId} {sd : StructDecl} {i : Nat} {t : Ty} :
      Δ r = some sd → sd.fields[i]? = some t →
      WfOp Δ (.updateField i) [.struct r, t] [.struct r]
  | vecPack {t : Ty} {n : Nat} :
      n < U64_SIZE → WfOp Δ .vecPack (List.replicate n t) [.vector t]
  | vecLen (t : Ty) : WfOp Δ .vecLen [.vector t] [.u64]
  | vecGet (t : Ty) : WfOp Δ .vecGet [.vector t, .u64] [t]
  | vecSet (t : Ty) : WfOp Δ .vecSet [.vector t, .u64, t] [.vector t]
  | vecPush (t : Ty) : WfOp Δ .vecPush [.vector t, t] [.vector t]
  | vecPop (t : Ty) : WfOp Δ .vecPop [.vector t] [.vector t, t]
  | vecInsert (t : Ty) :
      WfOp Δ .vecInsert [.vector t, .u64, t] [.vector t]
  | vecRemove (t : Ty) :
      WfOp Δ .vecRemove [.vector t, .u64] [.vector t, t]
  | vecSwap (t : Ty) :
      WfOp Δ .vecSwap [.vector t, .u64, .u64] [.vector t]
  | vecSwapRemove (t : Ty) :
      WfOp Δ .vecSwapRemove [.vector t, .u64] [.vector t, t]
  | vecAppend (t : Ty) :
      WfOp Δ .vecAppend [.vector t, .vector t] [.vector t]
  | vecReverse (t : Ty) : WfOp Δ .vecReverse [.vector t] [.vector t]
  | vecReverseSlice (t : Ty) :
      WfOp Δ .vecReverseSlice [.vector t, .u64, .u64] [.vector t]
  | vecContains (t : Ty) : WfOp Δ .vecContains [.vector t, t] [.bool]
  | vecIndexOf (t : Ty) : WfOp Δ .vecIndexOf [.vector t, t] [.bool, .u64]
  | vecTrim (t : Ty) :
      WfOp Δ .vecTrim [.vector t, .u64] [.vector t, .vector t]
  | vecTrimReverse (t : Ty) :
      WfOp Δ .vecTrimReverse [.vector t, .u64] [.vector t, .vector t]
  | vecRotate (t : Ty) :
      WfOp Δ .vecRotate [.vector t, .u64] [.vector t, .u64]
  | vecRotateSlice (t : Ty) :
      WfOp Δ .vecRotateSlice [.vector t, .u64, .u64, .u64] [.vector t, .u64]
  | vecDestroyEmpty (t : Ty) : WfOp Δ .vecDestroyEmpty [.vector t] []
  | mkMutLoc (x : LocalIndex) (t : Ty) :
      WfOp Δ (.mkMutLoc x) [t] [.mutRef t]
  | mkMutGlobal {r : ResourceId} {sd : StructDecl} :
      Δ r = some sd →
      WfOp Δ (.mkMutGlobal r) [.address] [.mutRef (.struct r)]
  | childMutField {r : ResourceId} {sd : StructDecl} {i : Nat} {t : Ty} :
      Δ r = some sd → sd.fields[i]? = some t →
      WfOp Δ (.childMutField i) [.mutRef (.struct r)] [.mutRef t]
  | childMutIndex (t : Ty) :
      WfOp Δ .childMutIndex [.mutRef (.vector t), .u64] [.mutRef t]
  | getMut (t : Ty) : WfOp Δ .getMut [.mutRef t] [t]
  | setMut (t : Ty) : WfOp Δ .setMut [.mutRef t, t] [.mutRef t]
  | isParent (pat : List (Option Nat)) (t₁ t₂ : Ty) :
      WfOp Δ (.isParent pat) [.mutRef t₁, .mutRef t₂] [.bool]
  | mutPathIndex (k : Nat) (t₁ t₂ : Ty) :
      WfOp Δ (.mutPathIndex k) [.mutRef t₁, .mutRef t₂] [.u64]
  | isMutLoc (x : LocalIndex) (t : Ty) :
      WfOp Δ (.isMutLoc x) [.mutRef t] [.bool]
  | isMutGlobal (r : ResourceId) (t : Ty) :
      WfOp Δ (.isMutGlobal r) [.mutRef t] [.bool]
  | mutAddr (t : Ty) : WfOp Δ .mutAddr [.mutRef t] [.address]
  | getGlobal {r : ResourceId} {sd : StructDecl} :
      Δ r = some sd → WfOp Δ (.getGlobal r) [.address] [.struct r]
  | writeGlobal {r : ResourceId} {sd : StructDecl} :
      Δ r = some sd → WfOp Δ (.writeGlobal r) [.address, .struct r] []
  | moveTo {r : ResourceId} {sd : StructDecl} :
      Δ r = some sd → WfOp Δ (.moveTo r) [.signer, .struct r] []
  | moveFrom {r : ResourceId} {sd : StructDecl} :
      Δ r = some sd → WfOp Δ (.moveFrom r) [.address] [.struct r]
  | exists_ (r : ResourceId) : WfOp Δ (.exists_ r) [.address] [.bool]

/-- The reference operations (which compile to failing assertions and
therefore need no typing). -/
def Oper.isRefOp : Oper → Bool
  | .borrowLoc | .borrowField _ | .borrowFieldInst _ _
  | .borrowGlobal _ | .borrowGlobalInst _ _ | .borrowVecElem
  | .readRef | .writeRef | .freezeRef => true
  | .add _ | .sub _ | .mul _ | .div _ | .mod _
  | .bitAnd _ | .bitOr _ | .bitXor _ | .shl _ | .shr _ | .cast _
  | .lt | .le | .eq | .and | .or | .not
  | .pack | .packInst _ | .unpack | .unpackInst _
  | .packVariant _ | .packVariantInst _ _
  | .unpackVariant _ | .unpackVariantInst _ _
  | .testVariant _ | .testVariantInst _ _
  | .getField _ | .getFieldInst _ _ | .updateField _
  | .vecPack | .vecLen | .vecGet | .vecSet | .vecPush | .vecPop
  | .vecInsert | .vecRemove | .vecSwap | .vecSwapRemove | .vecAppend
  | .vecReverse | .vecReverseSlice | .vecContains | .vecIndexOf
  | .vecTrim | .vecTrimReverse | .vecRotate | .vecRotateSlice | .vecDestroyEmpty
  | .mkMutLoc _ | .mkMutGlobal _ | .childMutField _ | .childMutIndex
  | .getMut | .setMut | .isParent _ | .mutPathIndex _
  | .isMutLoc _ | .isMutGlobal _ | .mutAddr
  | .getGlobal _ | .getGlobalInst _ _ | .writeGlobal _
  | .moveTo _ | .moveToInst _ _ | .moveFrom _ | .moveFromInst _ _
  | .exists_ _ | .existsInst _ _
  | .function _ | .functionInst _ _ => false

/-- Typing of the source reference operations.  Runtime reference values are
not described by `IsValid`: declared reference types are transparent at
external/prover boundaries, whereas these rules describe the bytecode locals
before reference elimination. -/
inductive WfRefInstr (Δ : StructDecls) (decl : LocalIndex → Option Ty) :
    Instr → Prop where
  | borrowLocImm {dst x : LocalIndex} {t : Ty} :
      decl x = some t → decl dst = some (.ref t) →
      WfRefInstr Δ decl (.call [dst] .borrowLoc [x])
  | borrowLocMut {dst x : LocalIndex} {t : Ty} :
      decl x = some t → decl dst = some (.mutRef t) →
      WfRefInstr Δ decl (.call [dst] .borrowLoc [x])
  | borrowFieldImm {dst src : LocalIndex} {r : ResourceId}
      {sd : StructDecl} {i : Nat} {t : Ty} :
      Δ r = some sd → sd.fields[i]? = some t →
      decl src = some (.ref (.struct r)) →
      decl dst = some (.ref t) →
      WfRefInstr Δ decl (.call [dst] (.borrowField i) [src])
  | borrowFieldMut {dst src : LocalIndex} {r : ResourceId}
      {sd : StructDecl} {i : Nat} {t : Ty} :
      Δ r = some sd → sd.fields[i]? = some t →
      decl src = some (.mutRef (.struct r)) →
      decl dst = some (.mutRef t) →
      WfRefInstr Δ decl (.call [dst] (.borrowField i) [src])
  | borrowFieldInstImm {dst src : LocalIndex} {r : ResourceId}
      {sd : StructDecl} {i : Nat} {t : Ty} {args : List Ty} :
      Δ r = some sd → args.length = sd.typeParams.length →
      sd.fields[i]? = some t →
      decl src = some (.ref (.structInst r args)) →
      decl dst = some (.ref (t.instantiate args)) →
      WfRefInstr Δ decl (.call [dst] (.borrowFieldInst i args) [src])
  | borrowFieldInstMut {dst src : LocalIndex} {r : ResourceId}
      {sd : StructDecl} {i : Nat} {t : Ty} {args : List Ty} :
      Δ r = some sd → args.length = sd.typeParams.length →
      sd.fields[i]? = some t →
      decl src = some (.mutRef (.structInst r args)) →
      decl dst = some (.mutRef (t.instantiate args)) →
      WfRefInstr Δ decl (.call [dst] (.borrowFieldInst i args) [src])
  | borrowGlobalImm {dst addr : LocalIndex} {r : ResourceId}
      {sd : StructDecl} :
      Δ r = some sd → decl addr = some .address →
      decl dst = some (.ref (.struct r)) →
      WfRefInstr Δ decl (.call [dst] (.borrowGlobal r) [addr])
  | borrowGlobalMut {dst addr : LocalIndex} {r : ResourceId}
      {sd : StructDecl} :
      Δ r = some sd → decl addr = some .address →
      decl dst = some (.mutRef (.struct r)) →
      WfRefInstr Δ decl (.call [dst] (.borrowGlobal r) [addr])
  | borrowGlobalInstImm {dst addr : LocalIndex} {r : ResourceId}
      {sd : StructDecl} {args : List Ty} :
      Δ r = some sd → args.length = sd.typeParams.length →
      decl addr = some .address →
      decl dst = some (.ref (.structInst r args)) →
      WfRefInstr Δ decl (.call [dst] (.borrowGlobalInst r args) [addr])
  | borrowGlobalInstMut {dst addr : LocalIndex} {r : ResourceId}
      {sd : StructDecl} {args : List Ty} :
      Δ r = some sd → args.length = sd.typeParams.length →
      decl addr = some .address →
      decl dst = some (.mutRef (.structInst r args)) →
      WfRefInstr Δ decl (.call [dst] (.borrowGlobalInst r args) [addr])
  | borrowVecElemImm {dst src idx : LocalIndex} {t : Ty} :
      decl src = some (.ref (.vector t)) → decl idx = some .u64 →
      decl dst = some (.ref t) →
      WfRefInstr Δ decl (.call [dst] .borrowVecElem [src, idx])
  | borrowVecElemMut {dst src idx : LocalIndex} {t : Ty} :
      decl src = some (.mutRef (.vector t)) → decl idx = some .u64 →
      decl dst = some (.mutRef t) →
      WfRefInstr Δ decl (.call [dst] .borrowVecElem [src, idx])
  | readImm {dst src : LocalIndex} {t : Ty} :
      decl src = some (.ref t) → decl dst = some t →
      WfRefInstr Δ decl (.call [dst] .readRef [src])
  | readMut {dst src : LocalIndex} {t : Ty} :
      decl src = some (.mutRef t) → decl dst = some t →
      WfRefInstr Δ decl (.call [dst] .readRef [src])
  | writeMut {dst src : LocalIndex} {t : Ty} :
      decl dst = some (.mutRef t) → decl src = some t →
      WfRefInstr Δ decl (.call [] .writeRef [dst, src])
  | freeze {dst src : LocalIndex} {t : Ty} :
      decl src = some (.mutRef t) → decl dst = some (.ref t) →
      WfRefInstr Δ decl (.call [dst] .freezeRef [src])

/-- Every well-typed reference instruction is classified as a reference operation. -/
theorem WfRefInstr.isRefOp { Δ : StructDecls }
    {decl : LocalIndex → Option Ty} {dsts srcs : List LocalIndex}
    {op : Oper} (h : WfRefInstr Δ decl (.call dsts op srcs)) :
    op.isRefOp := by
  cases h <;> rfl

/-- Instruction typing, relative to the declared local types `decl` of the
enclosing function and the program (for callee lookups). -/
inductive WfInstr (P : Program) (decl : LocalIndex → Option Ty) :
    Instr → Prop where
  | load {dst : LocalIndex} {v : Value} {t : Ty} :
      decl dst = some t → t.isRef = false → IsValid P.structs t v →
      WfInstr P decl (.load dst v)
  | assign {dst src : LocalIndex} {ts td : Ty} :
      decl src = some ts → decl dst = some td →
      CodeTySub P.structs ts td →
      WfInstr P decl (.assign dst src)
  | op {dsts srcs : List LocalIndex} {oper : Oper}
      {sts dts ots rts : List Ty} :
      srcs.mapM decl = some sts → dsts.mapM decl = some dts →
      WfOp P.structs oper ots rts →
      CodeTySubs P.structs sts ots → CodeTySubs P.structs rts dts →
      WfInstr P decl (.call dsts oper srcs)
  | callFun {f : FunId} {d' : FunDecl} {dsts srcs : List LocalIndex}
      {sts dts : List Ty} :
      P.funs f = some d' →
      srcs.mapM decl = some sts →
      sts.length = d'.numParams →
      (∀ i t t', sts[i]? = some t → d'.locals i = some t' →
        CodeTySub P.structs t t') →
      dsts.mapM decl = some dts →
      CodeTySubs P.structs d'.returns dts →
      WfInstr P decl (.call dsts (.function f) srcs)
  | callFunInst {f : FunId} {d' : FunDecl} {typeArgs : List Ty}
      {dsts srcs : List LocalIndex} {sts dts : List Ty} :
      P.funs f = some d' →
      typeArgs.length = d'.typeParams.length →
      srcs.mapM decl = some sts →
      sts.length = d'.numParams →
      (∀ i t t', sts[i]? = some t → d'.locals i = some t' →
        CodeTySub P.structs t (t'.instantiate typeArgs)) →
      (∀ i t t', sts[i]? = some t → d'.locals i = some t' →
        CodeTySub P.structs t t') →
      dsts.mapM decl = some dts →
      CodeTySubs P.structs (instantiateTypes typeArgs d'.returns) dts →
      CodeTySubs P.structs d'.returns dts →
      WfInstr P decl (.call dsts (.functionInst f typeArgs) srcs)
  | refInstr {i : Instr} :
      WfRefInstr P.structs decl i → WfInstr P decl i
  | nop : WfInstr P decl .nop

/-- Terminator typing: only `ret` carries an obligation (the returned
locals must be well-typed for the declared result types, feeding the
type preservation at the boundary). -/
inductive WfTerm (Δ : StructDecls) (d : FunDecl) : Term → Prop where
  | jump {b : BlockId} : WfTerm Δ d (.jump b)
  | branch {c : LocalIndex} {b₁ b₂ : BlockId} :
      d.locals c = some .bool → WfTerm Δ d (.branch c b₁ b₂)
  | ret {srcs : List LocalIndex} {sts : List Ty} :
      srcs.mapM d.locals = some sts → CodeTySubs Δ sts d.returns →
      WfTerm Δ d (.ret srcs)
  | abort {code : LocalIndex} :
      d.locals code = some .u64 → WfTerm Δ d (.abort code)

/-- A well-typed function declaration: every declared block is within the
declared size (so the compiled label `b + 1` never collides with the exit
labels), and its instructions and terminator are well-typed. -/
structure WfFunDecl (P : Program) (d : FunDecl) : Prop where
  blocksLt : ∀ b, d.body.blocks b ≠ none → b < d.body.size
  wfInstr : ∀ b blk, d.body.blocks b = some blk →
    ∀ i ∈ blk.instrs, WfInstr P d.locals i
  wfTerm : ∀ b blk, d.body.blocks b = some blk → WfTerm P.structs d blk.term

/-- A well-typed program: what the bytecode verifier guarantees for the
code the prover verifies. -/
def WfProg (P : Program) : Prop :=
  ∀ f d, P.funs f = some d → WfFunDecl P d

/-! ## The runtime typing invariant -/

/-- Every *defined* local is well-formed for its declared type. -/
def TypedLocals (Δ : StructDecls) (decl : LocalIndex → Option Ty)
    (locals : Locals) : Prop :=
  ∀ i t v, decl i = some t → locals i = some v → IsValid Δ t v

/-- Typed function arguments initialize typed locals. -/
theorem TypedLocals.initLocals {Δ : StructDecls} {d : FunDecl}
    {args : List Value} (h : TypedArgs Δ d args) :
    TypedLocals Δ d.locals (MoveModel.IR.initLocals args) :=
  fun i t v ht hv => h.2 i t v ht hv

/-- Writing a valid declared value preserves local typing. -/
theorem TypedLocals.writeLocal {Δ : StructDecls}
    {decl : LocalIndex → Option Ty} {s : MoveState} {x : LocalIndex}
    {v : Value}
    (h : TypedLocals Δ decl s.locals)
    (hv : ∀ t, decl x = some t → IsValid Δ t v) :
    TypedLocals Δ decl (s.writeLocal x v).locals := by
  intro i t w ht hw
  rw [MoveState.writeLocal_locals] at hw
  by_cases hix : i = x
  · rw [if_pos hix] at hw
    cases hw
    exact hv t (hix ▸ ht)
  · rw [if_neg hix] at hw
    exact h i t w ht hw

/-- Writing an in-range integer to a local declared at the same width
preserves local typing. -/
theorem TypedLocals.writeUInt {Δ : StructDecls}
    {decl : LocalIndex → Option Ty} {s : MoveState} {x : Nat} {i : Int}
    {w : IntWidth}
    (h : TypedLocals Δ decl s.locals) (hx : decl x = some (.uint w))
    (h0 : 0 ≤ i) (hi : i < (w.size : Int)) :
    TypedLocals Δ decl (s.writeLocal x (.int i)).locals :=
  h.writeLocal fun t ht => by
    rw [hx] at ht
    cases ht
    exact .uintv h0 hi

/-- Writing an in-range `u64` to a local declared as `u64` preserves local
typing. -/
theorem TypedLocals.writeU64 {Δ : StructDecls}
    {decl : LocalIndex → Option Ty} {s : MoveState} {x n : Nat}
    (h : TypedLocals Δ decl s.locals) (hx : decl x = some .u64)
    (hn : n < U64_SIZE) :
    TypedLocals Δ decl (s.writeLocal x (.u64 n)).locals :=
  h.writeUInt hx (by omega) (by rw [u64_size_eq]; exact_mod_cast hn)

/-- Writing a boolean to a local declared as `bool` preserves local typing. -/
theorem TypedLocals.writeBool {Δ : StructDecls}
    {decl : LocalIndex → Option Ty} {s : MoveState} {x : Nat} {b : Bool}
    (h : TypedLocals Δ decl s.locals) (hx : decl x = some .bool) :
    TypedLocals Δ decl (s.writeLocal x (.bool b)).locals :=
  h.writeLocal fun t ht => by
    rw [hx] at ht
    cases ht
    exact .bool b

/-- Writing values that are valid at the declared types of their
destinations preserves the invariant. -/
theorem TypedLocals.writeLocals {Δ : StructDecls}
    {decl : LocalIndex → Option Ty} :
    ∀ {s : MoveState} {xs : List LocalIndex} {vs : List Value}
      {ts : List Ty},
    TypedLocals Δ decl s.locals →
    xs.mapM decl = some ts → IsValidList Δ ts vs →
    TypedLocals Δ decl (MoveState.writeLocals s xs vs).locals := by
  intro s xs
  induction xs generalizing s with
  | nil =>
    intro vs ts h _ _
    match vs with
    | [] => simpa [MoveState.writeLocals] using h
    | _ :: _ => simpa [MoveState.writeLocals] using h
  | cons x xs ih =>
    intro vs ts h hts hvs
    rw [mapM_cons_eq_some] at hts
    obtain ⟨t, ts', hxt, hts', rfl⟩ := hts
    cases hvs with
    | cons hv hvs =>
      simp only [MoveState.writeLocals]
      exact ih (h.writeLocal fun t' ht' => by rw [hxt] at ht'; cases ht'; exact hv)
        hts' hvs

/-- Extract listwise validity of looked-up operand values from the locals
invariant and their declared types. -/
theorem TypedLocals.mapM_isValidList {Δ : StructDecls}
    {decl : LocalIndex → Option Ty} {locals : Locals}
    (h : TypedLocals Δ decl locals) :
    ∀ {srcs : List LocalIndex} {vs : List Value} {ts : List Ty},
    srcs.mapM locals = some vs → srcs.mapM decl = some ts →
    IsValidList Δ ts vs := by
  intro srcs
  induction srcs with
  | nil =>
    intro vs ts hv ht
    cases hv; cases ht; exact .nil
  | cons x xs ih =>
    intro vs ts hv ht
    rw [mapM_cons_eq_some] at hv ht
    obtain ⟨v, vs', hxv, hvs, rfl⟩ := hv
    obtain ⟨t, ts', hxt, hts, rfl⟩ := ht
    exact .cons (h x t v hxt hxv) (ih hvs hts)

/-! ## `IsValidList` utilities -/

theorem IsValidList.length {Δ : StructDecls} :
    ∀ {ts : List Ty} {vs : List Value}, IsValidList Δ ts vs →
      vs.length = ts.length := by
  intro ts
  induction ts with
  | nil => intro vs h; cases h; rfl
  | cons t ts ih =>
    intro vs h
    cases h with
    | cons _ hvs => simpa using ih hvs

/-- Corresponding indexed elements of a valid typed list are valid. -/
theorem IsValidList.getElem? {Δ : StructDecls} :
    ∀ {ts : List Ty} {vs : List Value}, IsValidList Δ ts vs →
    ∀ {i : Nat} {v : Value}, vs[i]? = some v →
    ∃ t, ts[i]? = some t ∧ IsValid Δ t v := by
  intro ts
  induction ts with
  | nil => intro vs h; cases h; intro i v hv; simp at hv
  | cons t ts ih =>
    intro vs h
    cases h with
    | cons hv hvs =>
      intro i w hw
      match i with
      | 0 => cases hw; exact ⟨_, rfl, hv⟩
      | i + 1 => simpa using ih hvs (by simpa using hw)

/-- Values read from well-typed caller locals form well-typed callee
arguments when the call-site types are assignment-compatible with the
callee parameters. -/
theorem TypedLocals.typedArgsOfCall {Δ : StructDecls}
    {callerDecl : LocalIndex → Option Ty} {callee : FunDecl}
    {locals : Locals} {srcs : List LocalIndex}
    {args : List Value} {srcTys : List Ty}
    (h : TypedLocals Δ callerDecl locals)
    (hargs : srcs.mapM locals = some args)
    (htys : srcs.mapM callerDecl = some srcTys)
    (hlen : srcTys.length = callee.numParams)
    (hsub : ∀ i t t', srcTys[i]? = some t →
      callee.locals i = some t' → CodeTySub Δ t t') :
    TypedArgs Δ callee args := by
  have hvalid := h.mapM_isValidList hargs htys
  refine ⟨hvalid.length.trans hlen, ?_⟩
  intro i t v ht hv
  obtain ⟨t', ht', hval⟩ := hvalid.getElem? hv
  exact (hsub i t' t ht' ht).semantic _ hval

/-- Instantiated calls use the callee parameter types after substituting the
call's explicit type arguments. -/
theorem TypedLocals.typedArgsOfCallInst {Δ : StructDecls}
    {callerDecl : LocalIndex → Option Ty} {callee : FunDecl}
    {typeArgs : List Ty} {locals : Locals} {srcs : List LocalIndex}
    {args : List Value} {srcTys : List Ty}
    (h : TypedLocals Δ callerDecl locals)
    (hargs : srcs.mapM locals = some args)
    (htys : srcs.mapM callerDecl = some srcTys)
    (hlen : srcTys.length = callee.numParams)
    (hsub : ∀ i t t', srcTys[i]? = some t → callee.locals i = some t' →
      CodeTySub Δ t (t'.instantiate typeArgs)) :
    TypedArgs Δ (callee.instantiate typeArgs) args := by
  have hvalid := h.mapM_isValidList hargs htys
  refine ⟨hvalid.length.trans hlen, ?_⟩
  intro i t v ht hv
  obtain ⟨sourceTy, hsourceTy, hval⟩ := hvalid.getElem? hv
  simp only [FunDecl.instantiate, Option.map_eq_some_iff] at ht
  obtain ⟨declTy, hdeclTy, rfl⟩ := ht
  exact (hsub i sourceTy declTy hsourceTy hdeclTy).semantic _ hval

/-- Replacing an element by a value valid at its type preserves list validity. -/
theorem IsValidList.set {Δ : StructDecls} :
    ∀ {ts : List Ty} {vs : List Value}, IsValidList Δ ts vs →
    ∀ {i : Nat} {t : Ty} {v : Value}, ts[i]? = some t → IsValid Δ t v →
    IsValidList Δ ts (vs.set i v) := by
  intro ts
  induction ts with
  | nil => intro vs h; cases h; intro i t v ht _; simp at ht
  | cons t₀ ts ih =>
    intro vs h
    cases h with
    | cons hv hvs =>
      intro i t v ht hval
      match i with
      | 0 => cases ht; exact .cons hval hvs
      | i + 1 =>
        simp only [List.set_cons_succ]
        exact .cons hv (ih hvs (by simpa using ht) hval)

/-- Validity at a replicated (homogeneous) type list: the length matches
and every element is valid at the element type. -/
theorem IsValidList.replicate {Δ : StructDecls} :
    ∀ {n : Nat} {t : Ty} {vs : List Value},
    IsValidList Δ (List.replicate n t) vs →
    vs.length = n ∧ ∀ v ∈ vs, IsValid Δ t v := by
  intro n
  induction n with
  | zero =>
    intro t vs h
    rw [List.replicate_zero] at h
    cases h
    exact ⟨rfl, by intro v hv; cases hv⟩
  | succ n ih =>
    intro t vs h
    rw [List.replicate_succ] at h
    cases h with
    | cons hv hvs =>
      obtain ⟨hlen, hes⟩ := ih hvs
      refine ⟨by simp [hlen], ?_⟩
      intro w hw
      rcases List.mem_cons.mp hw with rfl | hw
      · exact hv
      · exact hes w hw

/-- Transport listwise validity along pointwise subtyping. -/
theorem IsValidList.sub {Δ : StructDecls} :
    ∀ {ts ts' : List Ty} {vs : List Value},
    TySubs Δ ts ts' → IsValidList Δ ts vs → IsValidList Δ ts' vs := by
  intro ts ts' vs hsub
  induction hsub generalizing vs with
  | nil => intro h; cases h; exact .nil
  | cons hst _ ih =>
    intro h
    cases h with
    | cons hv hvs => exact .cons (hst _ hv) (ih hvs)

/-! ## Memory typing utilities -/

theorem TypedMemory.memWrite {Δ : StructDecls} {m : Memory}
    {r : ResourceId} {a : Address} {v : Value}
    (h : TypedMemory Δ m) (hv : IsValid Δ (.struct r) v) :
    TypedMemory Δ (memWrite m r a v) := by
  intro r' sd a' w hsd hw
  simp only [MoveModel.IR.memWrite] at hw
  by_cases hcase : (r' : ResourceKey) = (r : ResourceKey) ∧ a' = a
  · rw [if_pos hcase] at hw
    cases hw
    have hr : r' = r := by simpa using hcase.1
    exact hr ▸ hv
  · rw [if_neg hcase] at hw
    exact h r' sd a' w hsd hw

/-- Removing a resource preserves memory typing. -/
theorem TypedMemory.memRemove {Δ : StructDecls} {m : Memory}
    {r : ResourceId} {a : Address}
    (h : TypedMemory Δ m) : TypedMemory Δ (memRemove m r a) := by
  intro r' sd a' w hsd hw
  simp only [MoveModel.IR.memRemove] at hw
  by_cases hcase : (r' : ResourceKey) = (r : ResourceKey) ∧ a' = a
  · rw [if_pos hcase] at hw; cases hw
  · rw [if_neg hcase] at hw
    exact h r' sd a' w hsd hw

/-! ## Semantic preservation of the operations -/

set_option maxHeartbeats 0 in
theorem WfOp.sem_preserves {Δ : StructDecls} {op : Oper}
    {ots rts : List Ty} {vs rets : List Value} {m m' : Memory}
    {current : FrameId} {deref : RefTarget → Option Value}
    (hop : WfOp Δ op ots rts)
    (hvs : IsValidList Δ ots vs)
    (hm : TypedMemory Δ m)
    (hsem : op.sem current deref vs m = some (.ok rets m')) :
    IsValidList Δ rts rets ∧ TypedMemory Δ m' := by
  cases hop with
  -- one case per integer operation, over the operand's `NumType`; validity is
  -- `IsValid.intv` and the semantics `NumType.checked`/`bitwise`, with
  -- `tmod_mem`/`fdiv_mem`/`fromBits_mem` for the always-in-range results.
  | add nt =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_int_iff] at hv₁ hv₂
    obtain ⟨i, rfl, hloi, hhii⟩ := hv₁
    obtain ⟨j, rfl, hloj, hhij⟩ := hv₂
    simp only [Oper.sem, NumType.checked] at hsem
    split at hsem
    next h => cases hsem; exact ⟨.cons (.intv h.1 h.2) .nil, hm⟩
    next => cases hsem
  | sub nt =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_int_iff] at hv₁ hv₂
    obtain ⟨i, rfl, hloi, hhii⟩ := hv₁
    obtain ⟨j, rfl, hloj, hhij⟩ := hv₂
    simp only [Oper.sem, NumType.checked] at hsem
    split at hsem
    next h => cases hsem; exact ⟨.cons (.intv h.1 h.2) .nil, hm⟩
    next => cases hsem
  | mul nt =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_int_iff] at hv₁ hv₂
    obtain ⟨i, rfl, hloi, hhii⟩ := hv₁
    obtain ⟨j, rfl, hloj, hhij⟩ := hv₂
    simp only [Oper.sem, NumType.checked] at hsem
    split at hsem
    next h => cases hsem; exact ⟨.cons (.intv h.1 h.2) .nil, hm⟩
    next => cases hsem
  | div nt =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_int_iff] at hv₁ hv₂
    obtain ⟨i, rfl, hloi, hhii⟩ := hv₁
    obtain ⟨j, rfl, hloj, hhij⟩ := hv₂
    simp only [Oper.sem, NumType.checked] at hsem
    split at hsem
    next => cases hsem
    next =>
      split at hsem
      next h => cases hsem; exact ⟨.cons (.intv h.1 h.2) .nil, hm⟩
      next => cases hsem
  | mod nt =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_int_iff] at hv₁ hv₂
    obtain ⟨i, rfl, hloi, hhii⟩ := hv₁
    obtain ⟨j, rfl, hloj, hhij⟩ := hv₂
    simp only [Oper.sem] at hsem
    split at hsem
    next => cases hsem
    next hj0 =>
      cases hsem
      obtain ⟨hlo', hhi'⟩ := nt.tmod_mem hloi hloj hhij hj0
      exact ⟨.cons (.intv hlo' hhi') .nil, hm⟩
  | bitAnd nt =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_int_iff] at hv₁ hv₂
    obtain ⟨i, rfl, hloi, hhii⟩ := hv₁
    obtain ⟨j, rfl, hloj, hhij⟩ := hv₂
    simp only [Oper.sem, NumType.bitwise] at hsem
    cases hsem
    have hi' : (nt.toBits i).toNat < nt.size := by have := nt.toBits_mem i; omega
    have hb : (nt.toBits i).toNat &&& (nt.toBits j).toNat < nt.size :=
      Nat.lt_of_le_of_lt Nat.and_le_left hi'
    obtain ⟨hlo', hhi'⟩ := nt.fromBits_mem (Int.natCast_nonneg _) (by exact_mod_cast hb)
    exact ⟨.cons (.intv hlo' hhi') .nil, hm⟩
  | bitOr nt =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_int_iff] at hv₁ hv₂
    obtain ⟨i, rfl, hloi, hhii⟩ := hv₁
    obtain ⟨j, rfl, hloj, hhij⟩ := hv₂
    simp only [Oper.sem, NumType.bitwise] at hsem
    cases hsem
    have hsz : nt.size = 2 ^ nt.width.bits := rfl
    have hi' : (nt.toBits i).toNat < 2 ^ nt.width.bits := by have := nt.toBits_mem i; omega
    have hj' : (nt.toBits j).toNat < 2 ^ nt.width.bits := by have := nt.toBits_mem j; omega
    have hb : (nt.toBits i).toNat ||| (nt.toBits j).toNat < nt.size :=
      Nat.or_lt_two_pow hi' hj'
    obtain ⟨hlo', hhi'⟩ := nt.fromBits_mem (Int.natCast_nonneg _) (by exact_mod_cast hb)
    exact ⟨.cons (.intv hlo' hhi') .nil, hm⟩
  | bitXor nt =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_int_iff] at hv₁ hv₂
    obtain ⟨i, rfl, hloi, hhii⟩ := hv₁
    obtain ⟨j, rfl, hloj, hhij⟩ := hv₂
    simp only [Oper.sem, NumType.bitwise] at hsem
    cases hsem
    have hsz : nt.size = 2 ^ nt.width.bits := rfl
    have hi' : (nt.toBits i).toNat < 2 ^ nt.width.bits := by have := nt.toBits_mem i; omega
    have hj' : (nt.toBits j).toNat < 2 ^ nt.width.bits := by have := nt.toBits_mem j; omega
    have hb : (nt.toBits i).toNat ^^^ (nt.toBits j).toNat < nt.size :=
      Nat.xor_lt_two_pow hi' hj'
    obtain ⟨hlo', hhi'⟩ := nt.fromBits_mem (Int.natCast_nonneg _) (by exact_mod_cast hb)
    exact ⟨.cons (.intv hlo' hhi') .nil, hm⟩
  | shl nt =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_int_iff] at hv₁
    rw [isValid_uint_iff] at hv₂
    obtain ⟨i, rfl, hloi, hhii⟩ := hv₁
    obtain ⟨k, rfl, h0k, hk⟩ := hv₂
    simp only [Oper.sem] at hsem
    split at hsem
    next =>
      cases hsem
      have hmod : ((nt.toBits i).toNat <<< k.toNat) % nt.size < nt.size :=
        Nat.mod_lt _ nt.size_pos
      obtain ⟨hlo', hhi'⟩ := nt.fromBits_mem (Int.natCast_nonneg _) (by exact_mod_cast hmod)
      exact ⟨.cons (.intv hlo' hhi') .nil, hm⟩
    next => cases hsem
  | shr nt =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_int_iff] at hv₁
    rw [isValid_uint_iff] at hv₂
    obtain ⟨i, rfl, hloi, hhii⟩ := hv₁
    obtain ⟨k, rfl, h0k, hk⟩ := hv₂
    simp only [Oper.sem] at hsem
    split at hsem
    next =>
      cases hsem
      have hd1 : (1 : Int) ≤ 2 ^ k.toNat := by
        have hnat : 1 ≤ 2 ^ k.toNat := Nat.two_pow_pos k.toNat
        exact_mod_cast hnat
      obtain ⟨hlo', hhi'⟩ := nt.fdiv_mem hloi hhii hd1
      exact ⟨.cons (.intv hlo' hhi') .nil, hm⟩
    next => cases hsem
  | cast src target =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_int_iff] at hv₁
    obtain ⟨i, rfl, hloi, hhii⟩ := hv₁
    simp only [Oper.sem, NumType.checked] at hsem
    split at hsem
    next h => cases hsem; exact ⟨.cons (.intv h.1 h.2) .nil, hm⟩
    next => cases hsem
  | lt nt =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_int_iff] at hv₁ hv₂
    obtain ⟨i, rfl, _, _⟩ := hv₁
    obtain ⟨j, rfl, _, _⟩ := hv₂
    simp only [Oper.sem] at hsem
    cases hsem
    exact ⟨.cons (.bool _) .nil, hm⟩
  | le nt =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_int_iff] at hv₁ hv₂
    obtain ⟨i, rfl, _, _⟩ := hv₁
    obtain ⟨j, rfl, _, _⟩ := hv₂
    simp only [Oper.sem] at hsem
    cases hsem
    exact ⟨.cons (.bool _) .nil, hm⟩
  | eq =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    simp only [Oper.sem, Option.bind_eq_bind, Option.bind_eq_some_iff,
      Option.pure_def] at hsem
    obtain ⟨a, -, b, -, hsem⟩ := hsem
    split at hsem
    · cases hsem
      exact ⟨.cons (.bool _) .nil, hm⟩
    · cases hsem
  | and =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_bool_iff] at hv₁ hv₂
    obtain ⟨a, rfl⟩ := hv₁
    obtain ⟨b, rfl⟩ := hv₂
    simp only [Oper.sem] at hsem
    cases hsem
    exact ⟨.cons (.bool _) .nil, hm⟩
  | or =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_bool_iff] at hv₁ hv₂
    obtain ⟨a, rfl⟩ := hv₁
    obtain ⟨b, rfl⟩ := hv₂
    simp only [Oper.sem] at hsem
    cases hsem
    exact ⟨.cons (.bool _) .nil, hm⟩
  | not =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_bool_iff] at hv₁
    obtain ⟨a, rfl⟩ := hv₁
    simp only [Oper.sem] at hsem
    cases hsem
    exact ⟨.cons (.bool _) .nil, hm⟩
  | pack hsd =>
    simp only [Oper.sem] at hsem
    split at hsem
    next =>
      cases hsem
      exact ⟨.cons (.struct hsd hvs) .nil, hm⟩
    next => cases hsem
  | packInst hsd hargs =>
    simp only [Oper.sem] at hsem
    split at hsem
    next =>
      cases hsem
      exact ⟨.cons (.structInst hsd hargs hvs) .nil, hm⟩
    next => cases hsem
  | unpack hsd =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_struct_iff] at hv₁
    obtain ⟨sd', fs, hsd', rfl, hfs⟩ := hv₁
    rw [hsd] at hsd'
    cases hsd'
    simp only [Oper.sem] at hsem
    cases hsem
    exact ⟨hfs, hm⟩
  | unpackInst hsd hargs =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_structInst_iff] at hv₁
    obtain ⟨sd', fs, hsd', hargs', rfl, hfs⟩ := hv₁
    rw [hsd] at hsd'; cases hsd'
    simp only [Oper.sem] at hsem
    cases hsem
    exact ⟨hfs, hm⟩
  | packVariant hsd hvariants htag =>
    simp only [Oper.sem] at hsem
    split at hsem
    next =>
      cases hsem
      exact ⟨.cons (.variant hsd hvariants htag hvs) .nil, hm⟩
    next => cases hsem
  | unpackVariant hsd hvariants htag =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_enum_iff] at hv₁
    obtain ⟨sd', variants', actual, fields', fs, hsd', hvariants', hactual, rfl, hfs⟩ := hv₁
    rw [hsd] at hsd'; cases hsd'
    rw [hvariants] at hvariants'; cases hvariants'
    simp only [Oper.sem] at hsem
    split at hsem
    next heq =>
      subst actual
      rw [htag] at hactual
      cases hactual
      cases hsem
      exact ⟨hfs, hm⟩
    next => cases hsem
  | testVariant hsd hvariants htag =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_enum_iff] at hv₁
    obtain ⟨_, _, actual, _, fs, _, _, _, rfl, _⟩ := hv₁
    simp only [Oper.sem] at hsem
    cases hsem
    exact ⟨.cons (.bool (actual = _)) .nil, hm⟩
  | packVariantInst hsd hargs hvariants htag =>
    simp only [Oper.sem] at hsem
    split at hsem
    next =>
      cases hsem
      exact ⟨.cons (.variantInst hsd hargs hvariants htag hvs) .nil, hm⟩
    next => cases hsem
  | unpackVariantInst hsd hargs hvariants htag =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_enumInst_iff] at hv₁
    obtain ⟨sd', variants', actual, fields', fs, hsd', hargs',
      hvariants', hactual, rfl, hfs⟩ := hv₁
    rw [hsd] at hsd'; cases hsd'
    rw [hvariants] at hvariants'; cases hvariants'
    simp only [Oper.sem] at hsem
    split at hsem
    next heq =>
      subst actual
      rw [htag] at hactual
      cases hactual
      cases hsem
      exact ⟨hfs, hm⟩
    next => cases hsem
  | testVariantInst hsd hargs hvariants htag =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_enumInst_iff] at hv₁
    obtain ⟨_, _, actual, _, fs, _, _, _, _, rfl, _⟩ := hv₁
    simp only [Oper.sem] at hsem
    cases hsem
    exact ⟨.cons (.bool (actual = _)) .nil, hm⟩
  | @getField r sd i t hsd hf =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_struct_iff] at hv₁
    obtain ⟨sd', fs, hsd', rfl, hfs⟩ := hv₁
    rw [hsd] at hsd'
    cases hsd'
    simp only [Oper.sem] at hsem
    cases hfv : fs[i]? with
    | none => rw [hfv] at hsem; simp only [Option.map] at hsem; cases hsem
    | some v =>
      rw [hfv] at hsem
      simp only [Option.map] at hsem
      cases hsem
      obtain ⟨t', ht', hval⟩ := hfs.getElem? hfv
      rw [hf] at ht'
      cases ht'
      exact ⟨.cons hval .nil, hm⟩
  | @getFieldInst r sd i t args hsd hargs hf =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_structInst_iff] at hv₁
    obtain ⟨sd', fs, hsd', hargs', rfl, hfs⟩ := hv₁
    rw [hsd] at hsd'; cases hsd'
    simp only [Oper.sem] at hsem
    cases hfv : fs[i]? with
    | none => rw [hfv] at hsem; simp only [Option.map] at hsem; cases hsem
    | some v =>
      rw [hfv] at hsem
      simp only [Option.map] at hsem
      cases hsem
      obtain ⟨t', ht', hval⟩ := hfs.getElem? hfv
      simp only [instantiateTypes, List.getElem?_map, hf, Option.map_some] at ht'
      cases ht'
      exact ⟨.cons hval .nil, hm⟩
  | updateField hsd hf =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_struct_iff] at hv₁
    obtain ⟨sd', fs, hsd', rfl, hfs⟩ := hv₁
    rw [hsd] at hsd'
    cases hsd'
    simp only [Oper.sem] at hsem
    split at hsem
    next => cases hsem
    next =>
      split at hsem
      next =>
        cases hsem
        exact ⟨.cons (.struct hsd (hfs.set hf hv₂)) .nil, hm⟩
      next => cases hsem
  | vecPack hn =>
    simp only [Oper.sem] at hsem
    split at hsem
    next =>
      cases hsem
      obtain ⟨hlen, hes⟩ := hvs.replicate
      exact ⟨.cons (.vector (by rw [hlen]; exact hn) hes) .nil, hm⟩
    next => cases hsem
  | vecLen =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    simp only [Oper.sem] at hsem
    cases hsem
    exact ⟨.cons (.u64 hlen) .nil, hm⟩
  | vecGet =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [isValid_u64_iff] at hv₂
    obtain ⟨i, rfl, hi⟩ := hv₂
    simp only [Oper.sem] at hsem
    split at hsem
    next v hv =>
      cases hsem
      exact ⟨.cons (hes v (List.mem_of_getElem? hv)) .nil, hm⟩
    next => cases hsem
  | vecSet =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl with | cons hv₃ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [isValid_u64_iff] at hv₂
    obtain ⟨i, rfl, hi⟩ := hv₂
    simp only [Oper.sem] at hsem
    split at hsem
    next => cases hsem
    next =>
      split at hsem
      next =>
        cases hsem
        refine ⟨.cons (.vector (by simpa using hlen) ?_) .nil, hm⟩
        intro w hw
        rcases List.mem_or_eq_of_mem_set hw with hw | rfl
        · exact hes w hw
        · exact hv₃
      next => cases hsem
  | vecPush =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    simp only [Oper.sem] at hsem
    split at hsem
    next => cases hsem
    next =>
      split at hsem
      next h =>
        cases hsem
        refine ⟨.cons (.vector (by simpa using h) ?_) .nil, hm⟩
        intro w hw
        rcases List.mem_append.mp hw with hw | hw
        · exact hes w hw
        · rw [List.mem_singleton] at hw
          subst hw
          exact hv₂
      next => cases hsem
  | vecPop =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    simp only [Oper.sem] at hsem
    split at hsem
    next v hv =>
      cases hsem
      refine ⟨.cons (.vector ?_ ?_) (.cons (hes v ?_) .nil), hm⟩
      · rw [List.length_dropLast]
        exact Nat.lt_of_le_of_lt (Nat.sub_le _ _) hlen
      · intro w hw
        exact hes w (List.dropLast_subset _ hw)
      · exact List.mem_of_getElem? (List.getLast?_eq_getElem? ▸ hv)
    next => cases hsem
  | vecInsert =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl with | cons hv₃ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [isValid_u64_iff] at hv₂
    obtain ⟨i, rfl, hi⟩ := hv₂
    simp only [Oper.sem] at hsem
    split at hsem
    next => cases hsem
    next hgrowth =>
      split at hsem
      next hin =>
        cases hsem
        have hin' : i ≤ es.length ∧ es.length + 1 < U64_SIZE := by
          simpa using hin
        have hindex := hin'.1
        have hsize := hin'.2
        refine ⟨.cons (.vector ?_ ?_) .nil, hm⟩
        · simp [List.length_append]
          omega
        · intro w hw
          rcases List.mem_append.mp hw with hw | hw
          · exact hes w (List.take_subset _ _ hw)
          · simp only [List.mem_cons] at hw
            rcases hw with rfl | hw
            · exact hv₃
            · exact hes w (List.drop_subset _ _ hw)
      next => cases hsem
  | vecRemove =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [isValid_u64_iff] at hv₂
    obtain ⟨i, rfl, hi⟩ := hv₂
    simp only [Oper.sem] at hsem
    split at hsem
    next removed hremoved =>
      cases hsem
      refine ⟨.cons (.vector ?_ ?_) (.cons (hes removed ?_) .nil), hm⟩
      · exact Nat.lt_of_le_of_lt
          (by simp [List.length_append]; omega) hlen
      · intro w hw
        rcases List.mem_append.mp hw with hw | hw
        · exact hes w (List.take_subset _ _ hw)
        · exact hes w (List.drop_subset _ _ hw)
      · exact List.mem_of_getElem? hremoved
    next => cases hsem
  | vecSwap =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl with | cons hv₃ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [isValid_u64_iff] at hv₂ hv₃
    obtain ⟨i, rfl, hi⟩ := hv₂
    obtain ⟨j, rfl, hj⟩ := hv₃
    rw [Oper.sem_vecSwap] at hsem
    split at hsem
    next =>
      rename_i _ _ vi vj hvi hvj
      have hviValid := hes vi (List.mem_of_getElem? hvi)
      have hvjValid := hes vj (List.mem_of_getElem? hvj)
      cases hsem
      refine ⟨.cons (.vector (by simpa using hlen) ?_) .nil, hm⟩
      intro w hw
      rcases List.mem_or_eq_of_mem_set hw with hw | rfl
      · rcases List.mem_or_eq_of_mem_set hw with hw | rfl
        · exact hes w hw
        · exact hvjValid
      · exact hviValid
    next => cases hsem
  | vecSwapRemove =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [isValid_u64_iff] at hv₂
    obtain ⟨i, rfl, hi⟩ := hv₂
    rw [Oper.sem_vecSwapRemove] at hsem
    split at hsem
    next =>
      rename_i _ _ removed last hremoved hlast
      have hlastValid := hes last
        (List.mem_of_getElem? (List.getLast?_eq_getElem? ▸ hlast))
      cases hsem
      refine ⟨.cons (.vector ?_ ?_) (.cons (hes removed
        (List.mem_of_getElem? hremoved)) .nil), hm⟩
      · simp only [List.length_dropLast]
        exact Nat.lt_of_le_of_lt (Nat.sub_le _ _) (by simpa using hlen)
      · intro w hw
        have hw' := List.dropLast_subset _ hw
        rcases List.mem_or_eq_of_mem_set hw' with hw' | rfl
        · exact hes w hw'
        · exact hlastValid
    next => cases hsem
  | vecAppend =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁ hv₂
    obtain ⟨lhs, rfl, hlhs, hvlhs⟩ := hv₁
    obtain ⟨rhs, rfl, hrhs, hvrhs⟩ := hv₂
    rw [Oper.sem_vecAppend] at hsem
    split at hsem
    next hroom =>
      cases hsem
      refine ⟨.cons (.vector (by simpa using hroom) ?_) .nil, hm⟩
      intro w hw
      rcases List.mem_append.mp hw with hw | hw
      · exact hvlhs w hw
      · exact hvrhs w hw
    next => cases hsem
  | vecReverse =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [Oper.sem_vecReverse] at hsem
    cases hsem
    refine ⟨.cons (.vector (by simpa) ?_) .nil, hm⟩
    intro w hw
    exact hes w (List.mem_reverse.mp hw)
  | vecReverseSlice =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl with | cons hv₃ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [isValid_u64_iff] at hv₂ hv₃
    obtain ⟨left, rfl, hleft⟩ := hv₂
    obtain ⟨right, rfl, hright⟩ := hv₃
    rw [Oper.sem_vecReverseSlice] at hsem
    split at hsem
    next hrange =>
      cases hsem
      refine ⟨.cons (.vector ?_ ?_) .nil, hm⟩
      · have hrange' : _ := hrange
        simp only [Bool.and_eq_true, decide_eq_true_eq] at hrange'
        simp [List.length_append]
        omega
      · intro w hw
        rcases List.mem_append.mp hw with hw | hw
        · rcases List.mem_append.mp hw with hw | hw
          · exact hes w (List.take_subset _ _ hw)
          · exact hes w (List.drop_subset _ _
              (List.take_subset _ _ (List.mem_reverse.mp hw)))
        · exact hes w (List.drop_subset _ _ hw)
    next => cases hsem
  | vecContains =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [Oper.sem_vecContains] at hsem
    cases hsem
    exact ⟨.cons (.bool _) .nil, hm⟩
  | vecIndexOf =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [Oper.sem_vecIndexOf] at hsem
    cases hsem
    refine ⟨.cons (.bool _) (.cons ?_ .nil), hm⟩
    split
    · exact .u64 (Nat.lt_trans (by assumption) hlen)
    · exact .u64 (by simp [U64_SIZE])
  | vecTrim =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [isValid_u64_iff] at hv₂
    obtain ⟨newLen, rfl, hnewLen⟩ := hv₂
    rw [Oper.sem_vecTrim] at hsem
    split at hsem
    next =>
      cases hsem
      exact ⟨.cons (.vector (Nat.lt_of_le_of_lt (List.length_take_le ..) hnewLen)
          (fun w hw => hes w (List.take_subset _ _ hw)))
        (.cons (.vector (by simp only [List.length_drop]; omega)
          (fun w hw => hes w (List.drop_subset _ _ hw))) .nil), hm⟩
    next => cases hsem
  | vecTrimReverse =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [isValid_u64_iff] at hv₂
    obtain ⟨newLen, rfl, hnewLen⟩ := hv₂
    rw [Oper.sem_vecTrimReverse] at hsem
    split at hsem
    next =>
      cases hsem
      refine ⟨.cons (.vector (Nat.lt_of_le_of_lt (List.length_take_le ..) hnewLen)
          (fun w hw => hes w (List.take_subset _ _ hw)))
        (.cons (.vector ?_ ?_) .nil), hm⟩
      · simp only [List.length_reverse, List.length_drop]
        omega
      · intro w hw
        exact hes w (List.drop_subset _ _ (List.mem_reverse.mp hw))
    next => cases hsem
  | vecRotate =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [isValid_u64_iff] at hv₂
    obtain ⟨rot, rfl, hrotBound⟩ := hv₂
    rw [Oper.sem_vecRotate] at hsem
    split at hsem
    next hrot =>
      cases hsem
      refine ⟨.cons (.vector ?_ ?_) (.cons ?_ .nil), hm⟩
      · simp only [List.length_append, List.length_drop, List.length_take]
        rw [Nat.min_eq_left hrot]
        omega
      · intro w hw
        rcases List.mem_append.mp hw with hw | hw
        · exact hes w (List.drop_subset _ _ hw)
        · exact hes w (List.take_subset _ _ hw)
      · exact .u64 (by omega)
    next => cases hsem
  | vecRotateSlice =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl with | cons hv₃ htl =>
    cases htl with | cons hv₄ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [isValid_u64_iff] at hv₂ hv₃ hv₄
    obtain ⟨left, rfl, hleft⟩ := hv₂
    obtain ⟨rot, rfl, hrot⟩ := hv₃
    obtain ⟨right, rfl, hright⟩ := hv₄
    rw [Oper.sem_vecRotateSlice] at hsem
    by_cases hrange : (left ≤ rot ∧ rot ≤ right) ∧ right ≤ es.length
    · have hsem' : some (OpOutcome.ok
          [.vector (es.take left ++ (es.drop rot).take (right - rot) ++
            (es.drop left).take (rot - left) ++ es.drop right),
            .u64 (left + (right - rot))] m) = some (OpOutcome.ok rets m') := by
          simpa [hrange] using hsem
      cases hsem'
      refine ⟨.cons (.vector ?_ ?_) (.cons ?_ .nil), hm⟩
      · simp only [List.length_append, List.length_take, List.length_drop]
        rw [Nat.min_eq_left (by omega : left ≤ es.length)]
        rw [Nat.min_eq_left (by omega : right - rot ≤ es.length - rot)]
        rw [Nat.min_eq_left (by omega : rot - left ≤ es.length - left)]
        omega
      · intro w hw
        rcases List.mem_append.mp hw with hw | hw
        · rcases List.mem_append.mp hw with hw | hw
          · rcases List.mem_append.mp hw with hw | hw
            · exact hes w (List.take_subset _ _ hw)
            · exact hes w (List.drop_subset _ _ (List.take_subset _ _ hw))
          · exact hes w (List.drop_subset _ _ (List.take_subset _ _ hw))
        · exact hes w (List.drop_subset _ _ hw)
      · exact .u64 (by omega)
    · simp [hrange] at hsem
  | vecDestroyEmpty =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_vector_iff] at hv₁
    obtain ⟨es, rfl, hlen, hes⟩ := hv₁
    rw [Oper.sem_vecDestroyEmpty] at hsem
    split at hsem
    next => cases hsem; exact ⟨.nil, hm⟩
    next => cases hsem
  | mkMutLoc x t =>
    cases hvs with | @cons _ v _ _ hv₁ htl =>
    cases htl
    simp only [Oper.sem] at hsem
    split at hsem
    next =>
      cases hsem
      exact ⟨.cons (.mutRef hv₁) .nil, hm⟩
    next => cases hsem
  | mkMutGlobal hsd =>
    cases hvs with | @cons _ v _ _ hv₁ htl =>
    cases htl
    rw [isValid_address_iff] at hv₁
    obtain ⟨a, rfl⟩ := hv₁
    simp only [Oper.sem] at hsem
    split at hsem
    next v hv =>
      cases hsem
      exact ⟨.cons (.mutRef (hm _ _ _ _ hsd hv)) .nil, hm⟩
    next => cases hsem
  | childMutField hsd hf =>
    cases hvs with | @cons _ v _ _ hv₁ htl =>
    cases htl
    cases v <;> try simp [Oper.sem] at hsem
    rename_i rt w
    cases w <;> try simp at hsem
    rename_i fs
    obtain ⟨w, hfv, hrets, rfl⟩ := hsem
    subst hrets
    have hpay := hv₁.mutRef_payload
    rw [isValid_struct_iff] at hpay
    obtain ⟨sd', fs', hsd', heq, hfs⟩ := hpay
    cases heq
    rw [hsd] at hsd'
    cases hsd'
    obtain ⟨t', ht', hval⟩ := hfs.getElem? hfv
    rw [hf] at ht'
    cases ht'
    exact ⟨.cons (.mutRef hval) .nil, hm⟩
  | childMutIndex =>
    cases hvs with | @cons _ v₁ _ _ hv₁ htl =>
    cases htl with | @cons _ v₂ _ _ hv₂ htl =>
    cases htl
    rw [isValid_u64_iff] at hv₂
    obtain ⟨n, rfl, hn⟩ := hv₂
    cases v₁ <;> try simp [Oper.sem] at hsem
    rename_i rt w
    cases w <;> try simp at hsem
    rename_i es
    have hpay := hv₁.mutRef_payload
    rw [isValid_vector_iff] at hpay
    obtain ⟨es', heq, -, hes⟩ := hpay
    cases heq
    split at hsem
    next u hu =>
      cases hsem
      exact ⟨.cons (.mutRef (hes u (List.mem_of_getElem? hu))) .nil, hm⟩
    next => cases hsem
  | getMut =>
    cases hvs with | @cons _ v _ _ hv₁ htl =>
    cases htl
    cases v <;> try simp [Oper.sem] at hsem
    obtain ⟨hrets, rfl⟩ := hsem
    subst hrets
    exact ⟨.cons hv₁.mutRef_payload .nil, hm⟩
  | setMut =>
    cases hvs with | @cons _ v₁ _ _ hv₁ htl =>
    cases htl with | @cons _ v₂ _ _ hv₂ htl =>
    cases htl
    cases v₁ <;> try simp [Oper.sem] at hsem
    obtain ⟨-, hrets, rfl⟩ := hsem
    subst hrets
    exact ⟨.cons (.mutRef hv₂) .nil, hm⟩
  | isParent pat t₁ t₂ =>
    cases hvs with | @cons _ v₁ _ _ hv₁ htl =>
    cases htl with | @cons _ v₂ _ _ hv₂ htl =>
    cases htl
    cases v₁ <;> try simp [Oper.sem] at hsem
    cases v₂ <;> try simp at hsem
    obtain ⟨hrets, rfl⟩ := hsem
    subst hrets
    exact ⟨.cons (.bool _) .nil, hm⟩
  | mutPathIndex k t₁ t₂ =>
    cases hvs with | @cons _ v₁ _ _ hv₁ htl =>
    cases htl with | @cons _ v₂ _ _ hv₂ htl =>
    cases htl
    cases v₁ <;> try simp [Oper.sem] at hsem
    cases v₂ <;> try simp at hsem
    split at hsem
    next n hn =>
      split at hsem
      next h =>
        cases hsem
        exact ⟨.cons (.u64 h) .nil, hm⟩
      next => cases hsem
    next => cases hsem
  | isMutLoc x t =>
    cases hvs with | @cons _ v _ _ hv₁ htl =>
    cases htl
    cases v <;> try simp [Oper.sem] at hsem
    obtain ⟨hrets, rfl⟩ := hsem
    subst hrets
    exact ⟨.cons (.bool _) .nil, hm⟩
  | isMutGlobal r t =>
    cases hvs with | @cons _ v _ _ hv₁ htl =>
    cases htl
    cases v <;> try simp [Oper.sem] at hsem
    obtain ⟨hrets, rfl⟩ := hsem
    subst hrets
    exact ⟨.cons (.bool _) .nil, hm⟩
  | mutAddr =>
    cases hvs with | @cons _ v _ _ hv₁ htl =>
    cases htl
    cases v <;> try simp [Oper.sem] at hsem
    rename_i rt w
    obtain ⟨root, p⟩ := rt
    cases root with
    | loc x => simp at hsem
    | global r' a =>
      simp at hsem
      obtain ⟨hrets, rfl⟩ := hsem
      subst hrets
      exact ⟨.cons (.address a) .nil, hm⟩
  | getGlobal hsd =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_address_iff] at hv₁
    obtain ⟨a, rfl⟩ := hv₁
    simp only [Oper.sem] at hsem
    split at hsem
    next v hv => cases hsem; exact ⟨.cons (hm _ _ _ _ hsd hv) .nil, hm⟩
    next => cases hsem
  | writeGlobal hsd =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_address_iff] at hv₁
    obtain ⟨a, rfl⟩ := hv₁
    simp only [Oper.sem] at hsem
    split at hsem
    next =>
      cases hsem
      exact ⟨.nil, hm.memWrite hv₂⟩
    next => cases hsem
  | moveTo hsd =>
    cases hvs with | cons hv₁ htl =>
    cases htl with | cons hv₂ htl =>
    cases htl
    rw [isValid_signer_iff] at hv₁
    obtain ⟨a, rfl⟩ := hv₁
    simp only [Oper.sem] at hsem
    split at hsem
    next => cases hsem
    next =>
      split at hsem
      next => cases hsem
      next =>
        cases hsem
        exact ⟨.nil, hm.memWrite hv₂⟩
  | moveFrom hsd =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_address_iff] at hv₁
    obtain ⟨a, rfl⟩ := hv₁
    simp only [Oper.sem] at hsem
    split at hsem
    next v hv =>
      cases hsem
      exact ⟨.cons (hm _ _ _ _ hsd hv) .nil, hm.memRemove⟩
    next => cases hsem
  | exists_ =>
    cases hvs with | cons hv₁ htl =>
    cases htl
    rw [isValid_address_iff] at hv₁
    obtain ⟨a, rfl⟩ := hv₁
    simp only [Oper.sem] at hsem
    cases hsem
    exact ⟨.cons (.bool _) .nil, hm⟩

end MoveModel.IR
