-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Values

/-!
# Programs by value

UNTRUSTED (checked by the kernel). Phase 3 is proved in steps. A program *by
value* uses none of the instructions Phase 3 adds (references and global
storage) and no reference type in any signature: the subset of Phase 2a and
its generic forms (`Bytecode.inGenericByValue`), under the Phase 3 semantics.
The theorems are proved first for programs by value (`Proofs.Soundness`,
`Proofs.SoundnessGen`), then for the whole subset.

In a program by value, every parameter, return and local type is a value
type, and so is every type a consistent type certificate gives a reachable
instruction's operand stack (`certificate_values`): the instructions by value
only ever push value types.
-/

namespace MoveBytecode

/-- A reference token at the top of a signature token. -/
def SignatureToken.isRefToken : SignatureToken → Bool
  | .reference _ | .mutableReference _ => true
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 | .i8 | .i16 | .i32 | .i64 | .i128
  | .i256 | .address | .signer | .vector _ | .function _ _ _ | .struct _
  | .structInstantiation _ _ | .typeParameter _ => false

/-- The generic forms of the instructions by value: `CallGeneric`, and the
generic forms that build and take apart struct and enum values. -/
def Bytecode.inGenericByValue : Bytecode → Bool
  | .callGeneric _ | .packGeneric _ | .unpackGeneric _ | .packVariantGeneric _
  | .unpackVariantGeneric _ => true
  | .testVariantGeneric _ => false
  | .mutBorrowFieldGeneric _ | .immBorrowFieldGeneric _ | .mutBorrowVariantFieldGeneric _
  | .immBorrowVariantFieldGeneric _ => false
  | .mutBorrowGlobalGeneric _ | .immBorrowGlobalGeneric _ | .existsGeneric _
  | .moveFromGeneric _ | .moveToGeneric _ => false
  | .pop | .ret | .brTrue _ | .brFalse _ | .branch _ => false
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => false
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => false
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => false
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => false
  | .ldConst _ | .ldTrue | .ldFalse | .copyLoc _ | .moveLoc _ | .stLoc _ | .call _ => false
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .or | .and | .not => false
  | .eq | .neq | .lt | .gt | .le | .ge | .abort | .nop | .shl | .shr | .negate => false
  | .callClosure _ | .packClosure _ _ | .packClosureGeneric _ _ => false
  | .pack _ | .packVariant _ | .unpack _ | .unpackVariant _ | .testVariant _ => false
  | .readRef | .writeRef | .freezeRef | .mutBorrowLoc _ | .immBorrowLoc _ => false
  | .mutBorrowField _ | .mutBorrowVariantField _ | .immBorrowField _ | .immBorrowVariantField _ => false
  | .mutBorrowGlobal _ | .immBorrowGlobal _ | .exists_ _ | .moveFrom _ | .moveTo _ => false
  | .vecPack _ _ | .vecLen _ | .vecImmBorrow _ | .vecMutBorrow _ | .vecPushBack _
  | .vecPopBack _ | .vecUnpack _ _ | .vecSwap _ => false
  | .abortMsg => false

/-- A module by value: no signature of its pool has a reference type, and
every instruction of its code is of the core, of data by value, or a generic
form of these. -/
def CompiledModule.ByValue (m : CompiledModule) : Prop :=
  (∀ sig ∈ m.signatures.toList, ∀ t ∈ sig, t.isRefToken = false) ∧
  ∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code →
    ∀ i ∈ code.code.toList, (i.inCore || i.inDataByValue || i.inGenericByValue) = true

/-- A program by value: every module is. -/
def Program.ByValue (P : Program) : Prop :=
  ∀ m ∈ P.modules.toList, m.ByValue

end MoveBytecode

namespace MoveBytecode.Proofs

variable {P : Program}

/-- A value token translates to a value type, under type arguments that are
value types. -/
theorem ofTokenWith_isValue {m : CompiledModule} {tys : List RuntimeType}
    (htys : ∀ x ∈ tys, x.isValue = true) :
    ∀ {t : SignatureToken} {r : RuntimeType}, RuntimeType.ofTokenWith m tys t = some r →
      r.isValue = true
  | .bool, _, h | .u8, _, h | .u16, _, h | .u32, _, h | .u64, _, h | .u128, _, h | .u256, _, h
  | .i8, _, h | .i16, _, h | .i32, _, h | .i64, _, h | .i128, _, h | .i256, _, h
  | .address, _, h | .signer, _, h => by
    simp only [RuntimeType.ofTokenWith, Option.some.injEq] at h; subst h; rfl
  | .vector e, r, h => by
    simp only [RuntimeType.ofTokenWith, Option.map_eq_some_iff] at h
    obtain ⟨r', h', rfl⟩ := h
    simpa [RuntimeType.isValue] using ofTokenWith_isValue htys h'
  | .struct sh, r, h => by
    simp only [RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff] at h
    obtain ⟨_, -, h⟩ := h
    split at h
    · simp only [pure, Option.bind_eq_some_iff, Option.some.injEq] at h
      obtain ⟨_, -, rfl⟩ := h
      rfl
    · simp at h
  | .structInstantiation .., r, h => by
    simp only [RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff] at h
    obtain ⟨_, -, _, -, _, -, h⟩ := h
    split at h
    · simp only [pure, Option.some.injEq] at h; subst h; rfl
    · simp at h
  | .typeParameter _, r, h => by
    simp only [RuntimeType.ofTokenWith] at h
    exact htys r (List.mem_of_getElem? h)
  | .function .., r, h => by
    simp only [RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨_, -, _, -, rfl⟩ := h
    rfl
  | .reference _, _, h | .mutableReference _, _, h => by
    simp [RuntimeType.ofTokenWith] at h

theorem ofToken_isValue {m : CompiledModule} {t : SignatureToken} {r : RuntimeType}
    (h : RuntimeType.ofToken m t = some r) : r.isValue = true :=
  ofTokenWith_isValue (by simp) h

theorem ofSlotTokenWith_value {m : CompiledModule} {tys : List RuntimeType}
    (htys : ∀ x ∈ tys, x.isValue = true) {t : SignatureToken} (h : t.isRefToken = false)
    {r : RuntimeType} (hr : RuntimeType.ofSlotTokenWith m tys t = some r) : r.isValue = true := by
  cases t <;> simp only [SignatureToken.isRefToken] at h <;>
    simp only [RuntimeType.ofSlotTokenWith] at hr <;> first
      | exact ofTokenWith_isValue htys hr
      | simp at h

theorem ofSlotToken_value {m : CompiledModule} {t : SignatureToken} (h : t.isRefToken = false)
    {r : RuntimeType} (hr : RuntimeType.ofSlotToken m t = some r) : r.isValue = true :=
  ofSlotTokenWith_value (by simp) h hr

theorem mapM_some_mem {α β : Type} {f : α → Option β} :
    ∀ {l : List α} {l' : List β}, l.mapM f = some l' → ∀ b ∈ l', ∃ a ∈ l, f a = some b
  | [], l', h, b, hb => by simp at h; subst h; simp at hb
  | a :: l, l', h, b, hb => by
    simp only [List.mapM_cons, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨b', hb', l'', hl'', rfl⟩ := h
    rcases List.mem_cons.1 hb with rfl | hb
    · exact ⟨a, List.mem_cons_self .., hb'⟩
    · obtain ⟨a', ha', hfa'⟩ := mapM_some_mem hl'' b hb
      exact ⟨a', List.mem_cons_of_mem _ ha', hfa'⟩

/-- A signature of a module by value translates to value types. -/
theorem ofSignature_values {m : CompiledModule} (hm : m.ByValue) {targs : List RuntimeType}
    (htargs : ∀ x ∈ targs, x.isValue = true) {s : Nat} {sig : Signature}
    (hs : m.signatures[s]? = some sig) {tys : List RuntimeType}
    (h : RuntimeType.ofSignatureWith m targs sig = some tys) : ∀ t ∈ tys, t.isValue = true := by
  intro t ht
  obtain ⟨tok, htok, hr⟩ := mapM_some_mem h t ht
  exact ofSlotTokenWith_value htargs
    (hm.1 sig (Array.mem_toList_iff.2 (Array.mem_of_getElem? hs)) tok htok) hr

theorem byValue_module (hP : P.ByValue) {mi : Nat} {m : CompiledModule}
    (h : P.modules[mi]? = some m) : m.ByValue :=
  hP m (Array.mem_toList_iff.2 (Array.mem_of_getElem? h))

theorem function_module {f : FunctionRef} {info : FunctionInfo} (h : P.function? f = some info) :
    P.modules[f.module]? = some info.module := by
  simp only [Program.function?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
  obtain ⟨m, hm, _, -, _, -, _, -, rfl⟩ := h
  exact hm

theorem returnTypes_values (hP : P.ByValue) {f : FunctionRef}
    (hf : ∀ x ∈ f.typeArgs, x.isValue = true) {tys : List RuntimeType}
    (h : P.returnTypes f = some tys) : ∀ t ∈ tys, t.isValue = true := by
  simp only [Program.returnTypes, bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, hinfo, sig, hsig, h⟩ := h
  exact ofSignature_values (byValue_module hP (function_module hinfo)) hf hsig h

theorem paramTypes_values (hP : P.ByValue) {f : FunctionRef}
    (hf : ∀ x ∈ f.typeArgs, x.isValue = true) {tys : List RuntimeType}
    (h : P.paramTypes f = some tys) : ∀ t ∈ tys, t.isValue = true := by
  simp only [Program.paramTypes, bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, hinfo, sig, hsig, h⟩ := h
  exact ofSignature_values (byValue_module hP (function_module hinfo)) hf hsig h

theorem localTypes_values (hP : P.ByValue) {f : FunctionRef}
    (hf : ∀ x ∈ f.typeArgs, x.isValue = true) {tys : List RuntimeType}
    (h : P.localTypes f = some tys) : ∀ t ∈ tys, t.isValue = true := by
  simp only [Program.localTypes, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
  obtain ⟨info, hinfo, code, -, sig, hsig, ps, hps, lsig, hlsig, ls, hls, rfl⟩ := h
  have hm := byValue_module hP (function_module hinfo)
  intro t ht
  rcases List.mem_append.1 ht with ht | ht
  · exact ofSignature_values hm hf hsig hps t ht
  · exact ofSignature_values hm hf hlsig hls t ht

theorem handleReturnTypes_values (hP : P.ByValue) {mi h : Nat} {targs : List RuntimeType}
    (htargs : ∀ x ∈ targs, x.isValue = true) {tys : List RuntimeType}
    (hr : P.handleReturnTypesAt mi h targs = some tys) : ∀ t ∈ tys, t.isValue = true := by
  simp only [Program.handleReturnTypesAt, bind, Option.bind_eq_some_iff] at hr
  obtain ⟨m, hm, fh, -, sig, hsig, hr⟩ := hr
  exact ofSignature_values (byValue_module hP hm) htargs hsig hr

theorem function_def_mem {f : FunctionRef} {info : FunctionInfo} (h : P.function? f = some info) :
    info.definition ∈ info.module.functionDefs := by
  simp only [Program.function?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
  obtain ⟨m, -, _, -, d, hd, _, -, rfl⟩ := h
  exact Array.mem_of_getElem? hd

/-- Every instruction of a program by value is of the core, data by value, or
a generic form of these. -/
theorem byValue_instr (hP : P.ByValue) {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit}
    (hinfo : P.function? f = some info) (hcode : info.definition.code = some code) {pc : Nat}
    {instr : Bytecode} (hi : code.code[pc]? = some instr) :
    (instr.inCore || instr.inDataByValue || instr.inGenericByValue) = true :=
  (byValue_module hP (function_module hinfo)).2 info.definition
    (Array.mem_toList_iff.2 (function_def_mem hinfo)) code hcode instr
    (Array.mem_toList_iff.2 (Array.mem_of_getElem? hi))

/-- A value of a value type is not a reference. -/
theorem not_ref_of_value {v : Value} {t : RuntimeType} (hv : v.conforms P t = true)
    (ht : t.isValue = true) : ∀ r, v ≠ .ref r := by
  rintro r rfl
  rw [← assignable_isValue (conforms_assignable hv)] at ht
  simp [Value.type, RuntimeType.isValue] at ht

/-! ## Walks -/

theorem walk_snoc {succ : Nat → List Nat} {a b c : Nat} {p : List Nat}
    (hw : Walk succ a b p) (hc : c ∈ succ b) : Walk succ a c (p ++ [c]) := by
  induction hw with
  | nil n => exact Walk.cons hc (Walk.nil c)
  | cons hab _ ih => exact Walk.cons hab (ih hc)

theorem CodeReachable.succ {code : Array Bytecode} {pc s : Nat} (h : CodeReachable code pc)
    (hs : s ∈ successors code pc) : CodeReachable code s := by
  obtain ⟨p, hp⟩ := h
  exact ⟨_, walk_snoc hp hs⟩

theorem successors_instr {code : Array Bytecode} {pc s : Nat} (hs : s ∈ successors code pc) :
    ∃ i, code[pc]? = some i := by
  unfold successors at hs
  split at hs
  · simp at hs
  · next i _ => exact ⟨i, by assumption⟩

/-! ## Certificates of programs by value -/

/-- A struct type a definition defines is named without type arguments. -/
theorem definedStruct?_typeArgs {mi d : Nat} {s : StructId} (h : P.definedStruct? mi d = some s) :
    s.typeArgs = [] := by
  simp only [Program.definedStruct?, bind, Option.bind_eq_some_iff] at h
  obtain ⟨_, -, _, -, h⟩ := h
  split at h
  · simp only [Option.some.injEq] at h
    subst h
    rfl
  · simp at h

/-- Type arguments that instantiate type parameters are value types. -/
theorem argumentsWellFormed_values :
    ∀ {args : List RuntimeType} {cs : List AbilitySet},
      RuntimeType.argumentsWellFormed P args cs = true → ∀ x ∈ args, x.isValue = true
  | [], _, _ => by simp
  | a :: as, c :: cs, h => by
    simp only [RuntimeType.argumentsWellFormed, Bool.and_eq_true] at h
    intro x hx
    rcases List.mem_cons.1 hx with rfl | hx
    · exact h.1.1.1
    · exact argumentsWellFormed_values h.2 x hx
  | _ :: _, [], h => by simp [RuntimeType.argumentsWellFormed] at h

/-- The type arguments of an instance that satisfies its function's
constraints are value types. -/
theorem typeArgsOk_values {f : FunctionRef} (h : P.typeArgsOk f = true) :
    ∀ x ∈ f.typeArgs, x.isValue = true := by
  unfold Program.typeArgsOk at h
  split at h
  · exact argumentsWellFormed_values h
  · simp at h

theorem fieldTypes_values {s : StructId} (hs : ∀ x ∈ s.typeArgs, x.isValue = true)
    {tys : List RuntimeType} (h : P.fieldTypes s = some tys) :
    ∀ t ∈ tys, t.isValue = true := by
  simp only [Program.fieldTypes, bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, -, h⟩ := h
  split at h
  · intro t ht
    obtain ⟨fd, -, hfd⟩ := mapM_some_mem h t ht
    exact ofTokenWith_isValue hs hfd
  · simp at h
  · simp at h

theorem variantFieldTypes_values {s : StructId} (hs : ∀ x ∈ s.typeArgs, x.isValue = true)
    {tag : VariantIndex} {tys : List RuntimeType}
    (h : P.variantFieldTypes s tag = some tys) : ∀ t ∈ tys, t.isValue = true := by
  simp only [Program.variantFieldTypes, bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, -, h⟩ := h
  split at h
  · simp only [Option.bind_eq_some_iff] at h
    obtain ⟨vd, -, h⟩ := h
    intro t ht
    obtain ⟨fd, -, hfd⟩ := mapM_some_mem h t ht
    exact ofTokenWith_isValue hs hfd
  · simp at h
  · simp at h

theorem fieldTypes_args_value {s : StructId} {tys : List RuntimeType}
    (h : P.fieldTypes s = some tys) : ∀ x ∈ s.typeArgs, x.isValue = true := by
  simp only [Program.fieldTypes, bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, hinfo, -⟩ := h
  exact structType?_args_value hinfo

theorem variantFieldTypes_args_value {s : StructId} {tag : VariantIndex} {tys : List RuntimeType}
    (h : P.variantFieldTypes s tag = some tys) : ∀ x ∈ s.typeArgs, x.isValue = true := by
  simp only [Program.variantFieldTypes, bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, hinfo, -⟩ := h
  exact structType?_args_value hinfo

/-- The type arguments a generic form supplies: at least one. -/
theorem typeArgsMatch_some {l tys : List RuntimeType}
    (h : (match l with
      | [] => none
      | t :: ts => some (t :: ts) : Option (List RuntimeType)) = some tys) :
    l = tys ∧ tys ≠ [] := by
  cases l with
  | nil => simp at h
  | cons t ts =>
    simp only [Option.some.injEq] at h
    subst h
    exact ⟨rfl, List.cons_ne_nil t ts⟩

theorem vectorElemTypeWith_value {m : CompiledModule} {tys : List RuntimeType}
    (htys : ∀ x ∈ tys, x.isValue = true) {sig : SignatureIndex} {t : RuntimeType}
    (h : m.vectorElemTypeWith tys sig = some t) : t.isValue = true := by
  unfold CompiledModule.vectorElemTypeWith at h
  split at h
  · exact ofTokenWith_isValue htys h
  · simp at h
  · simp at h

theorem vectorElemType_value {m : CompiledModule} {sig : SignatureIndex} {t : RuntimeType}
    (h : m.vectorElemType sig = some t) : t.isValue = true :=
  vectorElemTypeWith_value (by simp) h

theorem drop_values {l : List RuntimeType} (h : ∀ t ∈ l, t.isValue = true) (n : Nat) :
    ∀ t ∈ l.drop n, t.isValue = true := fun t ht => h t (List.mem_of_mem_drop ht)

theorem cons_values {t : RuntimeType} {l : List RuntimeType} (ht : t.isValue = true)
    (h : ∀ t ∈ l, t.isValue = true) : ∀ u ∈ t :: l, u.isValue = true := by
  intro u hu
  rcases List.mem_cons.1 hu with rfl | hu
  · exact ht
  · exact h u hu

theorem tail_values {t : RuntimeType} {l : List RuntimeType}
    (h : ∀ u ∈ t :: l, u.isValue = true) : ∀ u ∈ l, u.isValue = true :=
  fun u hu => h u (List.mem_cons_of_mem _ hu)

theorem tail2_values {t t' : RuntimeType} {l : List RuntimeType}
    (h : ∀ u ∈ t :: t' :: l, u.isValue = true) : ∀ u ∈ l, u.isValue = true :=
  tail_values (tail_values h)

theorem popPush_values {tys : List RuntimeType} {r : RuntimeType}
    {before after : List RuntimeType} (h : popPush tys r before = some after) (hr : r.isValue = true)
    (hb : ∀ t ∈ before, t.isValue = true) : ∀ t ∈ after, t.isValue = true := by
  unfold popPush at h
  split at h
  · simp only [Option.some.injEq] at h
    subst h
    exact cons_values hr (drop_values hb _)
  · simp at h

theorem popPushAll_values {t : RuntimeType} {tys : List RuntimeType}
    {before after : List RuntimeType} (h : popPushAll t tys before = some after)
    (htys : ∀ u ∈ tys, u.isValue = true) (hb : ∀ t ∈ before, t.isValue = true) :
    ∀ u ∈ after, u.isValue = true := by
  unfold popPushAll at h
  split at h
  · split at h
    · simp only [Option.some.injEq] at h
      subst h
      intro u hu
      rcases List.mem_append.1 hu with hu | hu
      · exact htys u (List.mem_reverse.1 hu)
      · exact tail_values hb u hu
    · simp at h
  · simp at h

theorem intBinary_values {allow : IntType → Bool} {result : IntType → RuntimeType}
    (hres : ∀ t, (result t).isValue = true) {before after : List RuntimeType}
    (h : intBinary allow result before = some after) (hb : ∀ t ∈ before, t.isValue = true) :
    ∀ t ∈ after, t.isValue = true := by
  unfold intBinary at h
  split at h
  · split at h
    · simp only [Option.some.injEq] at h
      subst h
      exact cons_values (hres _) (tail2_values hb)
    · simp at h
  · simp at h

theorem castTo_values {target : IntType} {before after : List RuntimeType}
    (h : castTo target before = some after) (hb : ∀ t ∈ before, t.isValue = true) :
    ∀ t ∈ after, t.isValue = true := by
  unfold castTo at h
  split at h
  · simp only [Option.some.injEq] at h
    subst h
    exact cons_values rfl (tail_values hb)
  · simp at h

/-- Value tokens translate to value types, under type arguments that are value
types. -/
theorem ofTokenListWith_values {m : CompiledModule} {tys : List RuntimeType}
    (htys : ∀ x ∈ tys, x.isValue = true) :
    ∀ {ts : List SignatureToken} {rs : List RuntimeType},
      RuntimeType.ofTokenListWith m tys ts = some rs → ∀ x ∈ rs, x.isValue = true
  | [], _, h => by simp [RuntimeType.ofTokenListWith] at h; subst h; simp
  | t :: ts, _, h => by
    simp only [RuntimeType.ofTokenListWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨r, hr, rs, hrs, rfl⟩ := h
    intro x hx
    rcases List.mem_cons.1 hx with rfl | hx
    · exact ofTokenWith_isValue htys hr
    · exact ofTokenListWith_values htys hrs x hx

/-- The type arguments an instantiation supplies are value types, under type
arguments that are value types. -/
theorem typeArgsWith_values {m : CompiledModule} {tys : List RuntimeType}
    (htys : ∀ x ∈ tys, x.isValue = true) {sig : SignatureIndex} {rs : List RuntimeType}
    (h : m.typeArgsWith tys sig = some rs) : ∀ x ∈ rs, x.isValue = true := by
  unfold CompiledModule.typeArgsWith at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨toks, -, h⟩ := h
  exact ofTokenListWith_values htys h

/-- An instruction by value maps value types to value types. -/
theorem typeAfter_values (hP : P.ByValue) {f : FunctionRef}
    (hf : ∀ x ∈ f.typeArgs, x.isValue = true) {info : FunctionInfo}
    (hinfo : P.function? f = some info) {instr : Bytecode}
    (hi : (instr.inCore || instr.inDataByValue || instr.inGenericByValue) = true)
    {before after : List RuntimeType} (hb : ∀ t ∈ before, t.isValue = true)
    (h : typeAfter P f info.module instr before = some after) : ∀ t ∈ after, t.isValue = true := by
  have hm := byValue_module hP (function_module hinfo)
  cases instr <;> simp only [Bytecode.inCore, Bytecode.inDataByValue, Bytecode.inGenericByValue,
    Bool.or_false, Bool.false_or, Bool.false_eq_true] at hi <;> dsimp only [typeAfter] at h
  -- Pushing a primitive, or popping.
  all_goals first
    | exact castTo_values h hb
    | exact intBinary_values (fun _ => rfl) h hb
    | (simp only [Option.some.injEq] at h; subst h; exact cons_values rfl hb)
    | (simp only [Option.some.injEq] at h; subst h; exact hb)
    | skip
  case pop | brTrue | brFalse | abort | abortMsg | shl | shr | or | and | not | eq | neq
    | negate | stLoc =>
    repeat' (first
      | (simp only [Option.bind_eq_some_iff, bind] at h; obtain ⟨_, _, h⟩ := h)
      | split at h)
    all_goals first
      | (simp at h; done)
      | (simp only [Option.some.injEq] at h; subst h
         first
          | exact tail_values hb
          | exact tail2_values hb
          | exact cons_values rfl (tail_values hb)
          | exact cons_values rfl (tail2_values hb)
          | exact cons_values rfl (tail_values (tail2_values hb))
          | exact tail_values (tail_values (tail_values hb)))
  case ret =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨_, -, h⟩ := h
    split at h
    · simp only [Option.some.injEq] at h; subst h; simp
    · simp at h
  case ldConst i =>
    simp only [bind, Option.bind_eq_some_iff, Option.some.injEq] at h
    obtain ⟨_, -, t, ht, rfl⟩ := h
    exact cons_values (ofToken_isValue ht) hb
  case copyLoc i | moveLoc i =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨t, ht, h⟩ := h
    obtain ⟨tys, htys, hti⟩ := ht
    have htv := localTypes_values hP hf htys t (List.mem_of_getElem? hti)
    first
      | (split at h
         · simp only [Option.some.injEq] at h; subst h; exact cons_values htv hb
         · simp at h)
      | (simp only [Option.some.injEq] at h; subst h; exact cons_values htv hb)
  case call hdl =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨_, -, h⟩ := h
    split at h
    · simp at h
    · simp only [Option.bind_eq_some_iff] at h
      obtain ⟨params, -, rets, hrets, h⟩ := h
      split at h
      · simp only [Option.some.injEq] at h
        subst h
        intro t ht
        rcases List.mem_append.1 ht with ht | ht
        · exact handleReturnTypes_values hP (by simp) hrets t (List.mem_reverse.1 ht)
        · exact drop_values hb _ t ht
      · simp at h
  case pack d =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨st, -, tys, -, h⟩ := h
    exact popPush_values h rfl hb
  case unpack d =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨st, hst, tys, htys, h⟩ := h
    exact popPushAll_values h
      (fieldTypes_values (by simp [definedStruct?_typeArgs hst]) htys) hb
  case packVariant hdl =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨_, -, st, -, tys, -, h⟩ := h
    exact popPush_values h rfl hb
  case unpackVariant hdl =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨_, -, st, hst, tys, htys, h⟩ := h
    exact popPushAll_values h
      (variantFieldTypes_values (by simp [definedStruct?_typeArgs hst]) htys) hb
  case callGeneric i =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨x, -, fh, -, tys, ⟨targs, htargs, hmatch⟩, h⟩ := h
    obtain ⟨rfl, -⟩ := typeArgsMatch_some hmatch
    split at h
    · simp at h
    · simp only [Option.bind_eq_some_iff] at h
      obtain ⟨params, -, rets, hrets, h⟩ := h
      split at h
      · simp only [Option.some.injEq] at h
        subst h
        intro t ht
        rcases List.mem_append.1 ht with ht | ht
        · exact handleReturnTypes_values hP (typeArgsWith_values hf htargs) hrets t
            (List.mem_reverse.1 ht)
        · exact drop_values hb _ t ht
      · simp at h
  case packGeneric i =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨_, -, _, -, _, -, h⟩ := h
    exact popPush_values h rfl hb
  case unpackGeneric i =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨_, -, _, -, tys, htys, h⟩ := h
    exact popPushAll_values h (fieldTypes_values (fieldTypes_args_value htys) htys) hb
  case packVariantGeneric i =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨_, -, _, -, _, -, _, -, h⟩ := h
    exact popPush_values h rfl hb
  case unpackVariantGeneric i =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨_, -, _, -, _, -, tys, htys, h⟩ := h
    exact popPushAll_values h
      (variantFieldTypes_values (variantFieldTypes_args_value htys) htys) hb
  case vecPack sig n =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨t, ht, h⟩ := h
    exact popPush_values h
      (by simpa [RuntimeType.isValue] using vectorElemTypeWith_value hf ht) hb
  case vecUnpack sig n =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨t, ht, h⟩ := h
    exact popPushAll_values h (fun u hu => by
      rw [List.eq_of_mem_replicate hu]; exact vectorElemTypeWith_value hf ht) hb

/-- Types a list of value types is assignable to are value types. -/
theorem assignableAll_values : ∀ {ts us : List RuntimeType},
    RuntimeType.assignableAll ts us = true → (∀ t ∈ ts, t.isValue = true) →
      ∀ u ∈ us, u.isValue = true
  | [], [], _, _, _, hu => by simp at hu
  | t :: ts, u :: us, h, hv, w, hw => by
    simp only [RuntimeType.assignableAll, Bool.and_eq_true] at h
    rcases List.mem_cons.1 hw with rfl | hw
    · rw [← assignable_isValue h.1]; exact hv t (List.mem_cons_self ..)
    · exact assignableAll_values h.2 (fun x hx => hv x (List.mem_cons_of_mem _ hx)) w hw
  | [], _ :: _, h, _, _, _ | _ :: _, [], h, _, _, _ => by simp [RuntimeType.assignableAll] at h

/-- **The certificate of a program by value has value types**: at every
reachable instruction, by induction along the walk from the entry. -/
theorem certificate_values (hP : P.ByValue) {tc : TypeCertificate} (htc : tc.Consistent P)
    {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit} (hinfo : P.function? f = some info)
    (hcode : info.definition.code = some code) (hok : P.typeArgsOk f = true) {pc : Nat}
    (hr : CodeReachable code.code pc)
    (tys : List RuntimeType) (htys : tc f pc = some tys) : ∀ t ∈ tys, t.isValue = true := by
  have hm := byValue_module hP (function_module hinfo)
  have key : ∀ a c p, Walk (successors code.code) a c p → CodeReachable code.code a →
      (∀ tys, tc f a = some tys → ∀ t ∈ tys, t.isValue = true) →
      ∀ tys, tc f c = some tys → ∀ t ∈ tys, t.isValue = true := by
    intro a c p hw
    induction hw with
    | nil n => intro _ ha; exact ha
    | @cons a b c p hab _ ih =>
      intro hra ha
      obtain ⟨instr, hinstr⟩ := successors_instr hab
      obtain ⟨before, after, hbefore, hafter, hsucc⟩ := (htc f info code hinfo hcode hok).2 a instr
        hra hinstr
      have hiv : (instr.inCore || instr.inDataByValue || instr.inGenericByValue) = true :=
        hm.2 info.definition (Array.mem_toList_iff.2 (function_def_mem hinfo)) code hcode instr
          (Array.mem_toList_iff.2 (Array.mem_of_getElem? hinstr))
      refine ih (CodeReachable.succ hra hab) fun tys htys => ?_
      obtain ⟨c, hc, hac⟩ := hsucc b hab
      rw [hc, Option.some.injEq] at htys
      subst htys
      exact assignableAll_values hac
        (typeAfter_values hP (typeArgsOk_values hok) hinfo hiv (ha before hbefore) hafter)
  obtain ⟨p, hw⟩ := hr
  refine key 0 pc p hw ⟨[0], Walk.nil 0⟩ (fun tys h0 => ?_) tys htys
  rw [(htc f info code hinfo hcode hok).1, Option.some.injEq] at h0
  subst h0
  simp

/-! ## Natives -/

theorem mapM_isSome_mem {α β : Type} {f : α → Option β} :
    ∀ {l : List α}, (l.mapM f).isSome = true → ∀ a ∈ l, (f a).isSome = true
  | [], _, a, ha => by simp at ha
  | b :: l, h, a, ha => by
    simp only [List.mapM_cons, bind, Option.isSome_bind, pure] at h
    cases hb : f b with
    | none => rw [hb] at h; simp at h
    | some y =>
      rw [hb] at h
      simp only [Option.any_some] at h
      rcases List.mem_cons.1 ha with rfl | ha
      · rw [hb]; rfl
      · cases hl : l.mapM f with
        | none => rw [hl] at h; simp at h
        | some ys => exact mapM_isSome_mem (l := l) (by rw [hl]; rfl) a ha

/-- A slot token that is a value token translates to a value type. -/
theorem ofSlotToken_value_of_token {m : CompiledModule} {t : SignatureToken}
    (ht : (RuntimeType.ofToken m t).isSome = true) {r : RuntimeType}
    (hr : RuntimeType.ofSlotToken m t = some r) : r.isValue = true := by
  cases t <;> first
    | (simp [RuntimeType.ofToken, RuntimeType.ofTokenWith] at ht; done)
    | (simp only [RuntimeType.ofSlotToken, RuntimeType.ofSlotTokenWith] at hr
       exact ofToken_isValue hr)

/-- A supported value signature translates to value types. -/
theorem ofSignature_values_of_supported {m : CompiledModule} {s : Nat}
    (hs : m.SupportedValueSignatureNG s) {sig : Signature} (hsig : m.signatures[s]? = some sig)
    {tys : List RuntimeType} (h : RuntimeType.ofSignature m sig = some tys) :
    ∀ t ∈ tys, t.isValue = true := by
  obtain ⟨sig', hsig', hsome⟩ := hs
  rw [hsig, Option.some.injEq] at hsig'
  subst hsig'
  intro t ht
  obtain ⟨tok, htok, hr⟩ := mapM_some_mem h t ht
  exact ofSlotToken_value_of_token (mapM_isSome_mem hsome tok htok) hr

/-- **A supported program's natives take and return values only.** -/
theorem native_types_values (hs : P.SupportedNonGeneric) {f : FunctionRef} {info : FunctionInfo}
    (hf : P.function? f = some info) (hnone : info.definition.code = none) :
    (∀ tys, P.paramTypes f = some tys → ∀ t ∈ tys, t.isValue = true) ∧
    (∀ tys, P.returnTypes f = some tys → ∀ t ∈ tys, t.isValue = true) := by
  have hm := function_module hf
  have hsup := hs info.module (Array.mem_toList_iff.2 (Array.mem_of_getElem? hm))
  have hf' := hf
  simp only [Program.function?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hf'
  obtain ⟨m, hm', id, -, d, hd, h, hh, rfl⟩ := hf'
  simp only [CompiledModule.functionDefHandle] at hh
  obtain ⟨fh, hfh, hpar, hret, -⟩ := hsup.2.1 d (Array.mem_toList_iff.2 (Array.mem_of_getElem? hd))
    hnone
  rw [hfh, Option.some.injEq] at hh
  subst hh
  constructor
  · intro tys htys
    rw [paramTypes_nonGeneric hs] at htys
    simp only [Program.paramTypes, function?_typeArgs, hf, bind, Option.bind_some,
      Option.bind_eq_some_iff] at htys
    obtain ⟨sig, hsig, htys⟩ := htys
    exact ofSignature_values_of_supported hpar hsig htys
  · intro tys htys
    rw [returnTypes_nonGeneric hs] at htys
    simp only [Program.returnTypes, function?_typeArgs, hf, bind, Option.bind_some,
      Option.bind_eq_some_iff] at htys
    obtain ⟨sig, hsig, htys⟩ := htys
    exact ofSignature_values_of_supported hret hsig htys

/-! ## The non-generic subset -/

/-- An instruction of a function of a program of the non-generic subset is in
the non-generic subset. -/
theorem instr_inSubsetNG (hs : P.SupportedNonGeneric) {f : FunctionRef} {info : FunctionInfo}
    {code : CodeUnit} (hf : P.function? f = some info) (hcode : info.definition.code = some code)
    {pc : Nat} {instr : Bytecode} (hi : code.code[pc]? = some instr) : instr.inSubsetNG = true := by
  obtain ⟨hm, hd, -⟩ := function?_lookup hf
  exact ((nonGeneric_module hs hm).2.2.2.2.2 info.definition
    (Array.mem_toList_iff.2 (Array.mem_of_getElem? hd)) code hcode).2 instr
    (Array.mem_toList_iff.2 (Array.mem_of_getElem? hi)) |>.1

/-- The struct type a definition defines, at no type arguments, is the defined
type. -/
theorem definedStruct?_nil {mi d : Nat} {sid : StructId}
    (h : ∃ sid₀, P.definedStruct? mi d = some sid₀ ∧ sid = { sid₀ with typeArgs := [] }) :
    P.definedStruct? mi d = some sid := by
  obtain ⟨sid₀, h1, rfl⟩ := h
  have h0 := definedStruct?_typeArgs h1
  cases sid₀
  simp only at h0
  subst h0
  exact h1


end MoveBytecode.Proofs
