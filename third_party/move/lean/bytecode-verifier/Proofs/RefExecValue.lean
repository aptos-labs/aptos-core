-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefInv

/-!
# Steps that touch no reference

UNTRUSTED (checked by the kernel). An instruction on values only
(`Bytecode.ValueOnly`) changes nothing of the current frame but its operand
stack, from which it pops no reference, and its position; it records no
invalidation of a local and creates no frame (`exec_value`). The reference
invariant follows (`refInv_value`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

/-- The instructions on values only. -/
def _root_.MoveBytecode.Bytecode.ValueOnly : Bytecode → Bool
  | .brTrue _ | .brFalse _ | .branch _ | .nop => true
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => true
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => true
  | .ldTrue | .ldFalse | .ldConst _ => true
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => true
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => true
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => true
  | .lt | .gt | .le | .ge | .shl | .shr | .negate | .abort | .abortMsg => true
  | .pack _ | .unpack _ | .packVariant _ | .unpackVariant _ | .vecPack _ _ | .vecUnpack _ _ => true
  | .exists_ _ => true
  | .pop | .ret | .copyLoc _ | .moveLoc _ | .stLoc _ | .call _ | .eq | .neq => false
  | .callGeneric _ | .callClosure _ | .packClosure _ _ | .packClosureGeneric _ _ => false
  | .packGeneric _ | .packVariantGeneric _ | .unpackGeneric _ | .unpackVariantGeneric _ => false
  | .testVariant _ | .testVariantGeneric _ | .readRef | .writeRef | .freezeRef => false
  | .mutBorrowLoc _ | .immBorrowLoc _ | .mutBorrowField _ | .immBorrowField _ => false
  | .mutBorrowVariantField _ | .immBorrowVariantField _ => false
  | .mutBorrowFieldGeneric _ | .mutBorrowVariantFieldGeneric _ => false
  | .immBorrowFieldGeneric _ | .immBorrowVariantFieldGeneric _ => false
  | .mutBorrowGlobal _ | .immBorrowGlobal _ | .mutBorrowGlobalGeneric _ => false
  | .immBorrowGlobalGeneric _ | .existsGeneric _ | .moveFromGeneric _ | .moveTo _ => false
  | .moveFrom _ => false
  | .moveToGeneric _ | .vecLen _ | .vecImmBorrow _ | .vecMutBorrow _ | .vecPushBack _ => false
  | .vecPopBack _ | .vecSwap _ => false

/-- What a step on values does. -/
structure ValueShape (P : Program) (s s' : State) (fr : Frame) (rest : List Frame) (code : CodeUnit) :
    Prop where
  frame : ∃ fr' new j, s'.frames = fr' :: rest ∧ fr'.id = fr.id ∧ fr'.function = fr.function ∧
    fr'.locals = fr.locals ∧ fr'.stack = new ++ fr.stack.drop j ∧
    ((∀ x ∈ fr.function.typeArgs, x.isValue = true) → ∀ v ∈ fr.stack.take j, ∀ r, v ≠ .ref r) ∧
    fr'.pc ∈ successors code.code fr.pc ∧
    fr'.countedActive = fr.countedActive ∧
    /- The values pushed are no references, when the values popped are well formed. -/
    ((∀ x ∈ fr.function.typeArgs, x.isValue = true) →
      (∀ v ∈ fr.stack.take j, v.wellFormed P = true) → ∀ v ∈ new, ∀ r, v ≠ .ref r)
  invalidations : s'.invalidations = s.invalidations
  nextFrame : s'.nextFrame = s.nextFrame
  storage : s'.storage = s.storage
  held : s'.held = s.held
  active : s'.active = s.active

theorem ValueShape.ofNext {P : Program} {s : State} {fr fr' : Frame} {rest : List Frame}
    {code : CodeUnit}
    {new : List Value} {j : Nat} (hid : fr'.id = fr.id) (hfun : fr'.function = fr.function)
    (hloc : fr'.locals = fr.locals) (hstk : fr'.stack = new ++ fr.stack.drop j)
    (hpop : ∀ v ∈ fr.stack.take j, ∀ r, v ≠ .ref r) (hpc : fr'.pc ∈ successors code.code fr.pc)
    (hnew : (∀ x ∈ fr.function.typeArgs, x.isValue = true) →
      (∀ v ∈ fr.stack.take j, v.wellFormed P = true) → ∀ v ∈ new, ∀ r, v ≠ .ref r := by
      intro _ _ v hv r hr; subst hr; simp_all)
    (hca : fr'.countedActive = fr.countedActive := by rfl) :
    ValueShape P s { s with frames := fr' :: rest } fr rest code :=
  ⟨⟨fr', new, j, rfl, hid, hfun, hloc, hstk, fun _ => hpop, hpc, hca, hnew⟩, rfl, rfl, rfl, rfl, rfl⟩

/-- `ofNext` for a step that pops no reference when the frame's type
arguments are value types. -/
theorem ValueShape.ofNext' {P : Program} {s : State} {fr fr' : Frame} {rest : List Frame}
    {code : CodeUnit}
    {new : List Value} {j : Nat} (hid : fr'.id = fr.id) (hfun : fr'.function = fr.function)
    (hloc : fr'.locals = fr.locals) (hstk : fr'.stack = new ++ fr.stack.drop j)
    (hpop : (∀ x ∈ fr.function.typeArgs, x.isValue = true) →
      ∀ v ∈ fr.stack.take j, ∀ r, v ≠ .ref r) (hpc : fr'.pc ∈ successors code.code fr.pc)
    (hnew : (∀ x ∈ fr.function.typeArgs, x.isValue = true) →
      (∀ v ∈ fr.stack.take j, v.wellFormed P = true) → ∀ v ∈ new, ∀ r, v ≠ .ref r := by
      intro _ _ v hv r hr; subst hr; simp_all)
    (hca : fr'.countedActive = fr.countedActive := by rfl) :
    ValueShape P s { s with frames := fr' :: rest } fr rest code :=
  ⟨⟨fr', new, j, rfl, hid, hfun, hloc, hstk, hpop, hpc, hca, hnew⟩, rfl, rfl, rfl, rfl, rfl⟩

theorem primitive_notRef {w : Value} (h : Primitive w = true) : ∀ r, w ≠ .ref r := by
  intro r hr; subst hr; simp [Primitive] at h

theorem take_one_notRef {v : Value} {st : List Value} (hv : ∀ r, v ≠ .ref r) :
    ∀ w ∈ (v :: st).take 1, ∀ r, w ≠ .ref r := by
  intro w hw; simp at hw; subst hw; exact hv

theorem Replaces.shape {P : Program} {s : State} {fr fr' : Frame} {rest : List Frame}
    {code : CodeUnit} {n : Nat} {v : Value} (hR : Replaces fr fr' n v)
    (hprim : ∀ w ∈ fr.stack.take n, Primitive w = true)
    (hpc : fr.pc + 1 ∈ successors code.code fr.pc)
    (hv : ∀ r, v ≠ .ref r := by intro r h; cases h) :
    ValueShape P s { s with frames := fr' :: rest } fr rest code := by
  obtain ⟨-, rfl⟩ := hR
  exact ValueShape.ofNext (new := [v]) (j := n) rfl rfl rfl (by simp [advance, push]; try rfl)
    (fun w hw => primitive_notRef (hprim w hw)) (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hpc)
    (fun _ _ w hw => by simp only [List.mem_singleton] at hw; subst hw; exact hv)

/-- The fields of a well-formed struct, variant or vector of a value type are no
references. -/
theorem fields_notRef {P : Program} {sid : StructId} {fields : List Value}
    (h : (Value.struct sid fields).wellFormed P = true) : ∀ v ∈ fields, ∀ r, v ≠ .ref r := by
  obtain ⟨-, tys, htys, h⟩ := wellFormed_struct.1 h
  intro v hv
  obtain ⟨t, ht, hvt⟩ := conformsAll_mem h v hv
  exact not_ref_of_conforms hvt (fieldTypes_values (fieldTypes_args_value htys) htys t ht)

/-- The fields of a well-formed table are no references. -/
theorem tableFields_notRef {P : Program} {sid : StructId} {fields ks es : List Value}
    (h : (Value.table sid fields ks es).wellFormed P = true) : ∀ v ∈ fields, ∀ r, v ≠ .ref r := by
  obtain ⟨-, -, ⟨tys, htys, h⟩, -⟩ := wellFormed_table.1 h
  intro v hv
  obtain ⟨t, ht, hvt⟩ := conformsAll_mem h v hv
  exact not_ref_of_conforms hvt (fieldTypes_values (fieldTypes_args_value htys) htys t ht)

theorem variantFields_notRef {P : Program} {sid : StructId} {tag : VariantIndex}
    {fields : List Value} (h : (Value.variant sid tag fields).wellFormed P = true) :
    ∀ v ∈ fields, ∀ r, v ≠ .ref r := by
  simp only [Value.wellFormed] at h
  split at h
  · next tys htys =>
    intro v hv
    obtain ⟨t, ht, hvt⟩ := conformsAll_mem h v hv
    exact not_ref_of_conforms hvt
      (variantFieldTypes_values (variantFieldTypes_args_value htys) htys t ht)
  · cases h

theorem elems_notRef {P : Program} {t : RuntimeType} {elems : List Value}
    (h : (Value.vector t elems).wellFormed P = true) (ht : t.isValue = true) :
    ∀ v ∈ elems, ∀ r, v ≠ .ref r := by
  simp only [Value.wellFormed] at h
  intro v hv
  exact not_ref_of_conforms (allConform_iff.1 h v hv) ht

/-- A decoded constant is no reference. -/
theorem decodeConstant_notRef {P : Program} {t : RuntimeType} {d : List UInt8} {v : Value}
    (h : decodeConstant t d = some v) : ∀ r, v ≠ .ref r := by
  intro r hr
  subst hr
  unfold decodeConstant at h
  split at h
  · next v' heq =>
    simp only [Option.some.injEq] at h
    subst h
    have := (bcsDecode_shaped (P := P) t heq).2
    simp only [Value.type] at this
    subst this
    simp [bcsDecode] at heq
  · simp at h
  · simp at h

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

theorem value_packStruct {d : StructDefinitionIndex} {tys : List RuntimeType}
    (hpc : fr.pc + 1 ∈ successors code.code fr.pc)
    (h : stepOf (Step.packStruct P s fr rest d tys) = .done (.running s')) :
    ValueShape P s s' fr rest code := by
  have hcont : (Outcome.running s').continues = true := rfl
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨tys, htys, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
    obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    · next hc =>
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hc
      rw [Outcome.running.inj (done_next h)]
      refine ValueShape.ofNext (new := [_]) (j := tys.length) rfl rfl rfl
        (by simp [advance, push]; try rfl) (fun w hw => ?_) (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hpc)
        (fun _ _ v hv r hr => by
          simp only [List.mem_singleton] at hv; subst hv; split at hr <;> cases hr)
      obtain ⟨t, ht, hwt⟩ := conformsAll_mem hc w (List.mem_reverse.2 hw)
      exact not_ref_of_conforms hwt
        (fieldTypes_values (fieldTypes_args (need_ok' htys)) (need_ok' htys) t ht)
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem value_packEnum {hv' : StructVariantHandleIndex} {tys : List RuntimeType}
    (hpc : fr.pc + 1 ∈ successors code.code fr.pc)
    (h : stepOf (Step.packEnum P s fr rest info hv' tys) = .done (.running s')) :
    ValueShape P s s' fr rest code := by
  have hcont : (Outcome.running s').continues = true := rfl
  obtain ⟨vh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, htys, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
    obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    · next hc =>
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hc
      rw [Outcome.running.inj (done_next h)]
      refine ValueShape.ofNext (new := [_]) (j := tys.length) rfl rfl rfl
        (by simp [advance, push]; try rfl) (fun w hw => ?_) (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hpc)
      obtain ⟨t, ht, hwt⟩ := conformsAll_mem hc w (List.mem_reverse.2 hw)
      exact not_ref_of_conforms hwt
        (variantFieldTypes_values (variantFieldTypes_args (need_ok' htys)) (need_ok' htys) t ht)
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

/-- **`PackClosure`**: the captures are values (`.capturedReference`
otherwise), and so is the closure. -/
theorem value_packClosure {hd : FunctionHandleIndex} {mask : BitVec 64}
    {tys : List RuntimeType} (hpc : fr.pc + 1 ∈ successors code.code fr.pc)
    (h : stepOf (Step.packClosure P s fr rest info hd mask tys) = .done (.running s')) :
    ValueShape P s s' fr rest code := by
  have hcont : (Outcome.running s').continues = true := rfl
  unfold Step.packClosure at h
  obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  next capturedTys remaining _ =>
  split at h
  · exact (done_wrong h hcont).elim
  next hval =>
  obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
  obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  next hc =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hc hval
  rw [Outcome.running.inj (done_next h)]
  refine ValueShape.ofNext (new := [_]) (j := capturedTys.length) rfl rfl rfl
    (by simp [advance, push]; try rfl) (fun w hw => ?_)
    (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hpc)
    (hnew := fun _ _ v hv r hr => by rw [List.mem_singleton] at hv; subst hv; cases hr)
  obtain ⟨t, ht, hwt⟩ := conformsAll_mem hc w (List.mem_reverse.2 hw)
  exact not_ref_of_conforms hwt (List.all_eq_true.1 hval t ht)

theorem value_unpackStruct {d : StructDefinitionIndex} {tys : List RuntimeType}
    (hpc : fr.pc + 1 ∈ successors code.code fr.pc)
    (h : stepOf (Step.unpackStruct P s fr rest d tys) = .done (.running s')) :
    ValueShape P s s' fr rest code := by
  have hcont : (Outcome.running s').continues = true := rfl
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨⟨v, fr₁⟩, hpp, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st, hst, rfl⟩ := pop_ok hpp
    dsimp only at h
    split at h
    · next sid' fields =>
      split at h
      · exact (done_wrong h hcont).elim
      · rw [Outcome.running.inj (done_next h)]
        exact ValueShape.ofNext (new := fields.reverse) (j := 1) rfl rfl rfl
          (by simp [advance, pushAll, hst]) (by rw [hst]; exact take_one_notRef (by simp))
          (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hpc)
          (fun _ hwf w hw => fields_notRef (sid := sid') (hwf _ (by rw [hst]; simp)) w
            (List.mem_reverse.1 hw))
    · next sid' fields ks es =>
      split at h
      · exact (done_wrong h hcont).elim
      · rw [Outcome.running.inj (done_next h)]
        exact ValueShape.ofNext (new := fields.reverse) (j := 1) rfl rfl rfl
          (by simp [advance, pushAll, hst]) (by rw [hst]; exact take_one_notRef (by simp))
          (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hpc)
          (fun _ hwf w hw => tableFields_notRef (sid := sid') (ks := ks) (es := es)
            (hwf _ (by rw [hst]; simp)) w
            (List.mem_reverse.1 hw))
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem value_unpackEnum {hv' : StructVariantHandleIndex} {tys : List RuntimeType}
    (hpc : fr.pc + 1 ∈ successors code.code fr.pc)
    (h : stepOf (Step.unpackEnum P s fr rest info hv' tys) = .done (.running s')) :
    ValueShape P s s' fr rest code := by
  have hcont : (Outcome.running s').continues = true := rfl
  obtain ⟨vh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  dsimp only at h
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨v, fr₁⟩, hpp, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st, hst, rfl⟩ := pop_ok hpp
    dsimp only at h
    split at h
    · next sid' tag fields =>
      split at h
      · exact (done_wrong h hcont).elim
      · split at h
        · exact (done_fail h hcont).elim
        · rw [Outcome.running.inj (done_next h)]
          exact ValueShape.ofNext (new := fields.reverse) (j := 1) rfl rfl rfl
            (by simp [advance, pushAll, hst]) (by rw [hst]; exact take_one_notRef (by simp))
            (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hpc)
            (fun _ hwf w hw =>
              variantFields_notRef (sid := sid') (tag := tag) (hwf _ (by rw [hst]; simp)) w
                (List.mem_reverse.1 hw))
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem value_existsResource {d : StructDefinitionIndex} {tys : List RuntimeType}
    (hpc : fr.pc + 1 ∈ successors code.code fr.pc)
    (h : stepOf (Step.existsResource P s fr rest d tys) = .done (.running s')) :
    ValueShape P s s' fr rest code := by
  have hcont : (Outcome.running s').continues = true := rfl
  obtain ⟨sid, -, h⟩ := done_bind h hcont globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h hcont popAddress_ends
  obtain ⟨st, hstk, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h hcont unlocked_ends
  rw [Outcome.running.inj (done_next h)]
  exact ValueShape.ofNext (new := [_]) (j := 1) rfl rfl rfl (by simp [advance, push, hstk]; try rfl)
    (by rw [hstk]; exact take_one_notRef (by simp)) (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hpc)

/-- The generic forms of the steps on values. -/
def _root_.MoveBytecode.Bytecode.ValueOnlyGeneric : Bytecode → Bool
  | .packGeneric _ | .unpackGeneric _ | .packVariantGeneric _ | .unpackVariantGeneric _
  | .existsGeneric _ => true
  -- A closure's captures are values, and so is the closure.
  | .packClosure _ _ | .packClosureGeneric _ _ => true
  | _ => false

/-- **A generic step on values** has the shape of its non-generic form. -/
theorem exec_valueGeneric {instr : Bytecode} (hi : code.code[fr.pc]? = some instr)
    (hv : instr.ValueOnlyGeneric = true)
    (h : stepOf (Step.exec P s fr rest info code instr) = .done (.running s')) :
    ValueShape P s s' fr rest code := by
  have hcont : (Outcome.running s').continues = true := rfl
  have hft : instr.control = .fallThrough → fr.pc + 1 ∈ successors code.code fr.pc :=
    fun hc => by simp [successors, hi, hc]
  cases instr <;> simp only [Bytecode.ValueOnlyGeneric] at hv <;> try (exact absurd hv (by decide))
  case packGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact value_packStruct (hft rfl) h
  case unpackGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact value_unpackStruct (hft rfl) h
  case packVariantGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact value_packEnum (hft rfl) h
  case unpackVariantGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact value_unpackEnum (hft rfl) h
  case existsGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact value_existsResource (hft rfl) h
  case packClosure hd mask => exact value_packClosure (hft rfl) h
  case packClosureGeneric i mask =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact value_packClosure (hft rfl) h

theorem vectorElemTypeWith_head {m : CompiledModule} {tys : List RuntimeType}
    {sig : SignatureIndex} (h : (m.vectorElemTypeWith tys sig).isSome) :
    ∀ ty, (m.signatures[sig]?.bind (·.head?)) = some ty → refKind ty = none := by
  intro ty hty
  unfold CompiledModule.vectorElemTypeWith at h
  split at h
  · next t ht =>
    rw [ht] at hty
    simp only [Option.bind_some, List.head?_cons, Option.some.injEq] at hty
    subst hty
    obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1 h
    cases t <;> simp_all [refKind, RuntimeType.ofTokenWith]
  · simp at h
  · simp at h

theorem vectorElemType_head {m : CompiledModule} {sig : SignatureIndex}
    (h : (m.vectorElemType sig).isSome) :
    ∀ ty, (m.signatures[sig]?.bind (·.head?)) = some ty → refKind ty = none := by
  intro ty hty
  unfold CompiledModule.vectorElemType CompiledModule.vectorElemTypeWith at h
  split at h
  · next t ht =>
    rw [ht] at hty
    simp only [Option.bind_some, List.head?_cons, Option.some.injEq] at hty
    subst hty
    obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1 h
    exact refKind_of_ofToken hu
  · simp at h
  · simp at h


set_option maxHeartbeats 1600000 in
/-- **A step on values** keeps the current frame's locals and the frames
below, pops no reference, and invalidates no local. -/
theorem exec_value {instr : Bytecode} (hi : code.code[fr.pc]? = some instr)
    (hv : instr.ValueOnly = true)
    (h : stepOf (Step.exec P s fr rest info code instr) = .done (.running s')) :
    ValueShape P s s' fr rest code := by
  have hcont : (Outcome.running s').continues = true := rfl
  have hft : instr.control = .fallThrough → fr.pc + 1 ∈ successors code.code fr.pc :=
    fun hc => by simp [successors, hi, hc]
  cases instr <;> simp only [Bytecode.ValueOnly] at hv <;> try (exact absurd hv (by decide))
  case brTrue off =>
    obtain ⟨⟨b, fr₁⟩, hpb, h⟩ := done_bind h hcont popBool_ends
    obtain ⟨st, hst, rfl⟩ := popBool_ok hpb
    have hpop := take_one_notRef (v := .bool b) (st := st) (by intro r; simp)
    rw [← hst] at hpop
    cases b
    · rw [Outcome.running.inj (done_next h)]
      exact ValueShape.ofNext (new := []) (j := 1) rfl rfl rfl (by simp [advance, hst])
        hpop (by simp [successors, hi, Bytecode.control, Step.advance])
    · obtain ⟨fr₂, hj, h⟩ := done_bind h hcont jump_ends
      rw [jump_ok hj] at h
      rw [Outcome.running.inj (done_next h)]
      exact ValueShape.ofNext (new := []) (j := 1) rfl rfl rfl (by simp [hst]) hpop
        (by simp [successors, hi, Bytecode.control])
  case brFalse off =>
    obtain ⟨⟨b, fr₁⟩, hpb, h⟩ := done_bind h hcont popBool_ends
    obtain ⟨st, hst, rfl⟩ := popBool_ok hpb
    have hpop := take_one_notRef (v := .bool b) (st := st) (by intro r; simp)
    rw [← hst] at hpop
    cases b
    · obtain ⟨fr₂, hj, h⟩ := done_bind h hcont jump_ends
      rw [jump_ok hj] at h
      rw [Outcome.running.inj (done_next h)]
      exact ValueShape.ofNext (new := []) (j := 1) rfl rfl rfl (by simp [hst]) hpop
        (by simp [successors, hi, Bytecode.control])
    · rw [Outcome.running.inj (done_next h)]
      exact ValueShape.ofNext (new := []) (j := 1) rfl rfl rfl (by simp [advance, hst])
        hpop (by simp [successors, hi, Bytecode.control, Step.advance])
  case branch off =>
    obtain ⟨fr₂, hj, h⟩ := done_bind h hcont jump_ends
    rw [jump_ok hj] at h
    rw [Outcome.running.inj (done_next h)]
    exact ValueShape.ofNext (new := []) (j := 0) rfl rfl rfl (by simp) (by simp)
      (by simp [successors, hi, Bytecode.control])
  case nop =>
    rw [Outcome.running.inj (done_next h)]
    exact ValueShape.ofNext (new := []) (j := 0) rfl rfl rfl (by simp [advance]) (by simp)
      (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hft rfl)
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldTrue | ldFalse =>
    rw [Outcome.running.inj (done_next h)]
    exact ValueShape.ofNext (new := [_]) (j := 0) rfl rfl rfl (by simp [advance, push, ldInt]; try rfl)
      (by simp) (by simpa [Step.advance, Step.push, Step.ldInt] using hft rfl)
  case ldConst i =>
    obtain ⟨c, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨t, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨v, hv, h⟩ := done_bind h hcont (ends_need _ _)
    rw [Outcome.running.inj (done_next h)]
    exact ValueShape.ofNext (new := [v]) (j := 0) rfl rfl rfl (by simp [advance, push]; try rfl)
      (by simp) (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hft rfl)
      (fun _ _ w hw => by
        simp only [List.mem_singleton] at hw; subst hw; exact decodeConstant_notRef (P := P) (need_ok' hv))
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32
    | castI64 | castI128 | castI256 =>
    obtain ⟨fr', hx, h⟩ := done_bind h hcont cast_ends
    obtain ⟨r, hR, hprim⟩ := cast_ok hx
    rw [Outcome.running.inj (done_next h)]
    exact hR.shape hprim (hft rfl)
  case add | sub | mul | div | mod | bitOr | bitAnd | xor =>
    obtain ⟨fr', hx, h⟩ := done_bind h hcont intBinop_ends
    obtain ⟨t, r, hR, hprim⟩ := intBinop_ok hx
    rw [Outcome.running.inj (done_next h)]
    exact hR.shape hprim (hft rfl)
  case shl | shr =>
    obtain ⟨fr', hx, h⟩ := done_bind h hcont shift_ends
    obtain ⟨t, r, hR, hprim⟩ := shift_ok hx
    rw [Outcome.running.inj (done_next h)]
    exact hR.shape hprim (hft rfl)
  case or | and =>
    obtain ⟨fr', hx, h⟩ := done_bind h hcont boolBinop_ends
    obtain ⟨b, hR, hprim⟩ := boolBinop_ok hx
    rw [Outcome.running.inj (done_next h)]
    exact hR.shape hprim (hft rfl)
  case lt | gt | le | ge =>
    obtain ⟨fr', hx, h⟩ := done_bind h hcont intCompare_ends
    obtain ⟨b, hR, hprim⟩ := intCompare_ok hx
    rw [Outcome.running.inj (done_next h)]
    exact hR.shape hprim (hft rfl)
  case not =>
    obtain ⟨⟨b, fr₁⟩, hpb, h⟩ := done_bind h hcont popBool_ends
    obtain ⟨st, hst, rfl⟩ := popBool_ok hpb
    have hpop := take_one_notRef (v := .bool b) (st := st) (by intro r; simp)
    rw [← hst] at hpop
    rw [Outcome.running.inj (done_next h)]
    exact ValueShape.ofNext (new := [.bool !b]) (j := 1) rfl rfl rfl
      (by simp [advance, push, hst]; try rfl) hpop (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hft rfl)
  case negate =>
    obtain ⟨⟨t, x, fr₁⟩, hpi, h⟩ := done_bind h hcont popInt_ends
    obtain ⟨st, hst, rfl⟩ := popInt_ok hpi
    have hpop := take_one_notRef (v := .int t x) (st := st) (by intro r; simp)
    rw [← hst] at hpop
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    · split at h
      · next r _ =>
        rw [Outcome.running.inj (done_next h)]
        exact ValueShape.ofNext (new := [.int t r]) (j := 1) rfl rfl rfl
          (by simp [advance, push, hst]; try rfl) hpop (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hft rfl)
      · exact (done_fail h hcont).elim
  case pack d => exact value_packStruct (hft rfl) h
  case packVariant hv' => exact value_packEnum (hft rfl) h
  case vecPack sig n =>
    obtain ⟨t, het, h⟩ := done_bind h hcont elemType_ends
    obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
    obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    · next hc =>
      simp only [Bool.not_eq_true', Bool.not_eq_false, List.all_eq_true] at hc
      rw [Outcome.running.inj (done_next h)]
      refine ValueShape.ofNext' (new := [_]) (j := n) rfl rfl rfl
        (by simp [advance, push]; try rfl) (fun hargs w hw => ?_) (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hft rfl)
      exact not_ref_of_conforms (hc w (List.mem_reverse.2 hw))
        (elemTypeWith_value hargs het)
  case unpack d => exact value_unpackStruct (hft rfl) h
  case unpackVariant hv' => exact value_unpackEnum (hft rfl) h
  case vecUnpack sig n =>
    obtain ⟨t, het, h⟩ := done_bind h hcont elemType_ends
    obtain ⟨⟨v, fr₁⟩, hpp, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st, hst, rfl⟩ := pop_ok hpp
    dsimp only at h
    split at h
    · next t' elems =>
      split at h
      · exact (done_wrong h hcont).elim
      · next hne =>
        split at h
        · exact (done_fail h hcont).elim
        · rw [Outcome.running.inj (done_next h)]
          exact ValueShape.ofNext (new := elems.reverse) (j := 1) rfl rfl rfl
            (by simp [advance, pushAll, hst]) (by rw [hst]; exact take_one_notRef (by simp))
            (by simpa [Step.advance, Step.push, Step.pushAll, Step.ldInt] using hft rfl)
            (fun hargs hwf w hw => by
              have ht : t' = t := by simpa using hne
              subst ht
              exact elems_notRef (hwf _ (by rw [hst]; simp)) (elemTypeWith_value hargs het) w
                (List.mem_reverse.1 hw))
    · exact (done_wrong h hcont).elim
  case abort => exact (abort_absurd hcont h).elim
  case abortMsg => exact (abortMsg_absurd hcont h).elim
  case exists_ d => exact value_existsResource (hft rfl) h

/-- **The analysis's step on values**: it pops and pushes values. -/
theorem exec_valueStep {m : CompiledModule} {view : FunctionView} {A A' : RefState}
    {instr : Bytecode} (hv : instr.ValueOnly = true)
    (hops : ∀ sig k, instr = .vecUnpack sig k →
      ∀ ty, (m.signatures[sig]?.bind (·.head?)) = some ty → refKind ty = none)
    (hconst : ∀ c ∈ m.constantPool.toList, (RuntimeType.ofToken m c.type).isSome)
    (hx : (ReferenceSafety.executeInner m view instr).run A = .ok ((), A')) :
    ∃ j k, ValueStep j k A A' := by
  cases instr <;> simp only [Bytecode.ValueOnly] at hv <;> try (exact absurd hv (by decide))
  case branch | nop | castU8 | castU16 | castU32 | castU64 | castU128 | castU256 | castI8
    | castI16 | castI32 | castI64 | castI128 | castI256 | negate | not | exists_ =>
    rw [exec_nothing (by simp) hx]
    exact ⟨0, 0, ValueStep.refl _⟩
  case brTrue | brFalse | abort =>
    obtain ⟨r, hs, rfl⟩ := exec_popOne (by simp) hx
    exact ⟨1, 0, ValueStep.popOne hs⟩
  case add | sub | mul | mod | div | bitOr | bitAnd | xor | shl | shr | or | and | lt | gt | le
    | ge =>
    obtain ⟨r, hs, rfl⟩ := exec_binop (by simp) hx
    exact ⟨2, 1, by simp [hs], by simp [hs], by simp [hs]⟩
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldTrue | ldFalse =>
    rw [exec_ld (by simp) hx]
    exact ⟨0, 1, ValueStep.pushOne⟩
  case ldConst i =>
    exact ⟨0, 1, exec_ldConst (fun c hc => by
      obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1 (hconst c (getElem?_mem_toList hc))
      exact refKind_of_ofToken hu) hx⟩
  case abortMsg => exact ⟨2, 0, exec_abortMsg hx⟩
  case pack d => obtain ⟨sd, -, h⟩ := exec_pack hx; exact ⟨_, _, h⟩
  case packVariant d => obtain ⟨vh, sd, -, -, h⟩ := exec_packVariant hx; exact ⟨_, _, h⟩
  case unpack d => obtain ⟨sd, -, h⟩ := exec_unpack hx; exact ⟨_, _, h⟩
  case unpackVariant d => obtain ⟨vh, sd, -, -, h⟩ := exec_unpackVariant hx; exact ⟨_, _, h⟩
  case vecPack sig n => exact ⟨_, _, exec_vecPack hx⟩
  case vecUnpack sig n => exact ⟨_, _, exec_vecUnpack (hops sig n rfl) hx⟩

end MoveBytecode.Proofs
