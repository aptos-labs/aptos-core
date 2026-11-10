-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec

/-!
# The non-generic subset

UNTRUSTED. The supported subset of Phase 3, before stage A of Phase 4
admitted generic functions, structs and enums and the generic instruction
forms: every function and struct handle is non-generic, every type is closed
(`RuntimeType.ofToken`, under no type arguments), and every instruction is of
the core, of data by value, or of references and global storage
(`Bytecode.inSubsetNG`). A program of this subset is supported
(`Program.Supported`).

The theorems proved for it are milestones on the way to generic code, as M1–M3
were on the way to the struct-API borrows. Their shapes are the statements of
`Spec.Properties.Theorems` with `Program.SupportedNonGeneric` for
`Program.Supported`.
-/

namespace MoveBytecode

/-- The instructions of the non-generic subset. -/
def Bytecode.inSubsetNG (i : Bytecode) : Bool := i.inCore || i.inDataByValue || i.inReferences

/-- A signature of closed parameter, return or local types of the subset. -/
def CompiledModule.SupportedSignatureNG (m : CompiledModule) (s : SignatureIndex) : Prop :=
  ∃ sig, m.signatures[s]? = some sig ∧ (RuntimeType.ofSignature m sig).isSome

/-- A signature of closed value types of the subset. -/
def CompiledModule.SupportedValueSignatureNG (m : CompiledModule) (s : SignatureIndex) : Prop :=
  ∃ sig, m.signatures[s]? = some sig ∧ (sig.mapM (RuntimeType.ofToken m)).isSome

/-- The element type of a vector instruction is a closed type of the subset. -/
def CompiledModule.SupportedOperandsNG (m : CompiledModule) : Bytecode → Prop
  | .vecPack s _ | .vecUnpack s _ | .vecLen s | .vecImmBorrow s | .vecMutBorrow s
  | .vecPushBack s | .vecPopBack s | .vecSwap s => (m.vectorElemType s).isSome
  | _ => True

/-- The non-generic subset of Phase 3: every function is non-generic with
closed types of the subset; every native takes and returns values only,
without function types (`Program.Supported` leaves natives over function
values out); every struct handle is non-generic; every struct definition declares fields or
variants of closed types of the subset; every constant has a type of the
subset; and every instruction is in the non-generic subset with its type
operand. -/
def CompiledModule.SupportedNonGeneric (m : CompiledModule) : Prop :=
  (∀ fh ∈ m.functionHandles.toList, fh.typeParameters = [] ∧
    m.SupportedSignatureNG fh.parameters ∧ m.SupportedSignatureNG fh.returns) ∧
  (∀ d ∈ m.functionDefs.toList, d.code = none → ∃ fh, m.functionHandles[d.function]? = some fh ∧
    m.SupportedValueSignatureNG fh.parameters ∧ m.SupportedValueSignatureNG fh.returns ∧
    m.NoFunctionTypes fh.parameters ∧ m.NoFunctionTypes fh.returns) ∧
  (∀ sh ∈ m.structHandles.toList, sh.typeParameters = []) ∧
  (∀ d ∈ m.structDefs.toList, d.fieldInformation ≠ .native ∧
    ∀ f ∈ CompiledModule.StructFieldInformation.allFields d.fieldInformation,
      (RuntimeType.ofToken m f.signature).isSome) ∧
  (∀ c ∈ m.constantPool.toList, (RuntimeType.ofToken m c.type).isSome) ∧
  (∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code →
    m.SupportedSignatureNG code.locals ∧
    ∀ i ∈ code.code.toList, i.inSubsetNG ∧ m.SupportedOperandsNG i)

def Program.SupportedNonGeneric (P : Program) : Prop :=
  ∀ m ∈ P.modules.toList, m.SupportedNonGeneric

/-! ## The milestone statements -/

/-- **L1–L6, L8–L10** from local verification. -/
def LocalPropertiesNonGeneric (V : VerifierInterface) : Prop :=
  ∀ m, V.verifyModule VerifierConfig.production m = true → 6 ≤ m.version → m.version ≤ 10 →
    m.IndexIntegrity ∧ m.Uniqueness ∧
    m.CodeStructure VerifierConfig.production.maxValueStackSize ∧
    m.ConstantsWellFormed ∧ m.InstructionConsistency ∧
    m.TypeFormation ∧ m.StructDefinitionsWellFormed ∧ m.FriendsWellFormed

/-- **D1** for the non-generic subset. -/
def SoundnessNonGeneric (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric →
    NativesRespect P N → AdmissibleInitial P s₀ → NoWrong P N s₀

/-- **D2** for the non-generic subset. -/
def OwnershipAccountingNonGeneric (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy) (auth : List Address)
    (A : List StructId) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric →
    NativesRespect P N → NativesRespectOwnership P N ν π → EntryInitial P auth A s₀ →
    OwnershipAccounted P N ν π s₀

/-- **D3** for the non-generic subset. -/
def EncapsulationNonGeneric (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy) (auth : List Address)
    (A : List StructId) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric →
    NativesRespect P N → NativesRespectOwnership P N ν π → EntryInitial P auth A s₀ →
    Encapsulated P N ν π auth A s₀

/-- **D4** for the non-generic subset. -/
def ReferenceSafetyNonGeneric (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric →
    NativesRespect P N → NativesRespectOwnership P N ν π → AdmissibleInitial P s₀ →
    ReferencesSafe P N ν s₀

end MoveBytecode

namespace MoveBytecode.Proofs

/-! ## Closed types

A token that translates under no type arguments mentions no type parameter,
so it translates to the same type under any type arguments. -/

mutual
theorem ofTokenWith_closed {m : CompiledModule} {tys : List RuntimeType} :
    ∀ {t : SignatureToken} {r : RuntimeType}, RuntimeType.ofTokenWith m [] t = some r →
      RuntimeType.ofTokenWith m tys t = some r
  | .bool, _, h | .u8, _, h | .u16, _, h | .u32, _, h | .u64, _, h | .u128, _, h
  | .u256, _, h | .i8, _, h | .i16, _, h | .i32, _, h | .i64, _, h | .i128, _, h
  | .i256, _, h | .address, _, h | .signer, _, h | .struct _, _, h => by
    simpa only [RuntimeType.ofTokenWith] using h
  | .vector e, r, h => by
    simp only [RuntimeType.ofTokenWith, Option.map_eq_some_iff] at h ⊢
    obtain ⟨r', h', rfl⟩ := h
    exact ⟨r', ofTokenWith_closed h', rfl⟩
  | .structInstantiation hh args, r, h => by
    simp only [RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff] at h ⊢
    obtain ⟨sh, hsh, sid, hsid, targs, htargs, h⟩ := h
    exact ⟨sh, hsh, sid, hsid, targs, ofTokenListWith_closed htargs, h⟩
  | .typeParameter _, _, h => by simp [RuntimeType.ofTokenWith] at h
  | .function ps rs a, r, h => by
    simp only [RuntimeType.ofTokenWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h ⊢
    obtain ⟨ps', hps, rs', hrs, rfl⟩ := h
    exact ⟨ps', ofSlotTokensWith_closed hps, rs', ofSlotTokensWith_closed hrs, rfl⟩
  | .reference _, _, h | .mutableReference _, _, h => by
    simp [RuntimeType.ofTokenWith] at h

theorem ofSlotTokensWith_closed {m : CompiledModule} {tys : List RuntimeType} :
    ∀ {ts : List SignatureToken} {rs : List RuntimeType},
      RuntimeType.ofSlotTokensWith m [] ts = some rs →
      RuntimeType.ofSlotTokensWith m tys ts = some rs
  | [], _, h => by simpa only [RuntimeType.ofSlotTokensWith] using h
  | .reference t :: ts, _, h => by
    simp only [RuntimeType.ofSlotTokensWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h ⊢
    obtain ⟨r, hr, rs, hrs, rfl⟩ := h
    exact ⟨r, ofTokenWith_closed hr, rs, ofSlotTokensWith_closed hrs, rfl⟩
  | .mutableReference t :: ts, _, h => by
    simp only [RuntimeType.ofSlotTokensWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h ⊢
    obtain ⟨r, hr, rs, hrs, rfl⟩ := h
    exact ⟨r, ofTokenWith_closed hr, rs, ofSlotTokensWith_closed hrs, rfl⟩
  | .bool :: ts, _, h | .u8 :: ts, _, h | .u16 :: ts, _, h | .u32 :: ts, _, h
  | .u64 :: ts, _, h | .u128 :: ts, _, h | .u256 :: ts, _, h | .i8 :: ts, _, h
  | .i16 :: ts, _, h | .i32 :: ts, _, h | .i64 :: ts, _, h | .i128 :: ts, _, h
  | .i256 :: ts, _, h | .address :: ts, _, h | .signer :: ts, _, h | .vector _ :: ts, _, h
  | .struct _ :: ts, _, h | .structInstantiation _ _ :: ts, _, h
  | .typeParameter _ :: ts, _, h | .function _ _ _ :: ts, _, h => by
    simp only [RuntimeType.ofSlotTokensWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h ⊢
    obtain ⟨r, hr, rs, hrs, rfl⟩ := h
    exact ⟨r, ofTokenWith_closed hr, rs, ofSlotTokensWith_closed hrs, rfl⟩

theorem ofTokenListWith_closed {m : CompiledModule} {tys : List RuntimeType} :
    ∀ {ts : List SignatureToken} {rs : List RuntimeType},
      RuntimeType.ofTokenListWith m [] ts = some rs → RuntimeType.ofTokenListWith m tys ts = some rs
  | [], _, h => by simpa only [RuntimeType.ofTokenListWith] using h
  | t :: ts, _, h => by
    simp only [RuntimeType.ofTokenListWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h ⊢
    obtain ⟨r, hr, rs, hrs, rfl⟩ := h
    exact ⟨r, ofTokenWith_closed hr, rs, ofTokenListWith_closed hrs, rfl⟩
end

theorem ofTokenListWith_length {m : CompiledModule} {tys : List RuntimeType} :
    ∀ {ts : List SignatureToken} {rs : List RuntimeType},
      RuntimeType.ofTokenListWith m tys ts = some rs → rs.length = ts.length
  | [], _, h => by simp [RuntimeType.ofTokenListWith] at h; subst h; rfl
  | t :: ts, _, h => by
    simp only [RuntimeType.ofTokenListWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨r, -, rs, hrs, rfl⟩ := h
    simp [ofTokenListWith_length hrs]

theorem ofSlotTokenWith_closed {m : CompiledModule} {tys : List RuntimeType} {t : SignatureToken}
    {r : RuntimeType} (h : RuntimeType.ofSlotTokenWith m [] t = some r) :
    RuntimeType.ofSlotTokenWith m tys t = some r := by
  cases t <;> simp only [RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at h ⊢ <;>
    first
    | exact ofTokenWith_closed h
    | (obtain ⟨r', h', rfl⟩ := h; exact ⟨r', ofTokenWith_closed h', rfl⟩)

theorem ofSignatureWith_closed {m : CompiledModule} {tys : List RuntimeType} :
    ∀ {sig : Signature} {rs : List RuntimeType}, RuntimeType.ofSignatureWith m [] sig = some rs →
      RuntimeType.ofSignatureWith m tys sig = some rs
  | [], _, h => by simpa [RuntimeType.ofSignatureWith] using h
  | t :: ts, _, h => by
    simp only [RuntimeType.ofSignatureWith, List.mapM_cons, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h ⊢
    obtain ⟨r, hr, rs, hrs, rfl⟩ := h
    exact ⟨r, ofSlotTokenWith_closed hr, rs,
      (ofSignatureWith_closed (sig := ts) (by simpa [RuntimeType.ofSignatureWith] using hrs) :), rfl⟩

theorem vectorElemTypeWith_closed {m : CompiledModule} {tys : List RuntimeType} {s : Nat}
    {r : RuntimeType} (h : m.vectorElemTypeWith [] s = some r) : m.vectorElemTypeWith tys s = some r := by
  simp only [CompiledModule.vectorElemTypeWith] at h ⊢
  split at h
  · exact ofTokenWith_closed h
  · simp at h
  · simp at h

/-! ## Instances in the non-generic subset

Every struct and function handle of a program of the non-generic subset has no
type parameters and every token is closed, so a struct type that resolves has
no type arguments (`structType?` checks one per type parameter), and a
function's types are the same at every instance. -/

section
variable {P : Program}

theorem nonGeneric_module (hs : P.SupportedNonGeneric) {i : Nat} {m : CompiledModule}
    (h : P.modules[i]? = some m) : m.SupportedNonGeneric :=
  hs m (Array.mem_toList_iff.2 (Array.mem_of_getElem? h))

theorem struct?_parts {r : StructRef} {info : StructInfo} (h : P.struct? r = some info) :
    P.modules[r.module]? = some info.module ∧
      info.module.structHandles[info.definition.structHandle]? = some info.handle := by
  simp only [Program.struct?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
  obtain ⟨m, hm, _, -, d, -, sh, hsh, rfl⟩ := h
  exact ⟨hm, hsh⟩

theorem structType?_parts {s : StructId} {info : StructInfo} (h : P.structType? s = some info) :
    (∃ r, P.struct? r = some info) ∧ s.typeArgs.length = info.handle.typeParameters.length := by
  simp only [Program.structType?, bind, Option.bind_eq_some_iff] at h
  obtain ⟨r, -, a, ha, h⟩ := h
  split at h
  · next hlen =>
    simp only [Option.some.injEq] at h
    subst h
    exact ⟨⟨r, ha⟩, hlen.1⟩
  · simp at h

/-- The type arguments of a struct type that resolves are value types. -/
theorem structType?_args_value {s : StructId} {info : StructInfo}
    (h : P.structType? s = some info) : ∀ x ∈ s.typeArgs, x.isValue = true := by
  simp only [Program.structType?, bind, Option.bind_eq_some_iff] at h
  obtain ⟨r, -, a, -, h⟩ := h
  split at h
  · next hc =>
    intro x hx
    exact List.all_eq_true.1 hc.2 x hx
  · simp at h

/-- A struct type of a program of the non-generic subset that resolves is a
non-generic type, without type arguments. -/
theorem structType?_nonGeneric (hs : P.SupportedNonGeneric) {s : StructId} {info : StructInfo}
    (h : P.structType? s = some info) : info.handle.typeParameters = [] ∧ s.typeArgs = [] := by
  obtain ⟨⟨r, hr⟩, hlen⟩ := structType?_parts h
  obtain ⟨hm, hsh⟩ := struct?_parts hr
  have hnp := (nonGeneric_module hs hm).2.2.1 info.handle
    (Array.mem_toList_iff.2 (Array.mem_of_getElem? hsh))
  rw [hnp] at hlen
  exact ⟨hnp, List.eq_nil_of_length_eq_zero hlen⟩

theorem AbilitySet.all_inter (a : AbilitySet) : AbilitySet.all.inter a = a := by
  cases a; rfl

/-- The abilities of a struct type of the non-generic subset are its
declaration's. -/
theorem abilities_struct_nonGeneric (hs : P.SupportedNonGeneric) {s : StructId}
    {info : StructInfo} (h : P.structType? s = some info) :
    (RuntimeType.struct s).abilities P = info.handle.abilities := by
  obtain ⟨hnp, hargs⟩ := structType?_nonGeneric hs h
  simp only [RuntimeType.abilities, h, hnp, hargs, RuntimeType.argumentAbilities,
    AbilitySet.all_inter]

theorem function?_typeArgs (f : FunctionRef) (tys : List RuntimeType) :
    P.function? { f with typeArgs := tys } = P.function? f := rfl

theorem function?_lookup {f : FunctionRef} {info : FunctionInfo} (h : P.function? f = some info) :
    P.modules[f.module]? = some info.module ∧
      info.module.functionDefs[f.index]? = some info.definition ∧
      info.module.functionHandles[info.definition.function]? = some info.handle := by
  simp only [Program.function?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
  obtain ⟨m, hm, _, -, d, hd, fh, hfh, rfl⟩ := h
  exact ⟨hm, hd, hfh⟩

/-- Resolution names a function without type arguments. -/
theorem resolve_typeArgs {mi h : Nat} {f : FunctionRef} (hr : P.resolve mi h = some f) :
    f.typeArgs = [] := by
  simp only [Program.resolve, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at hr
  obtain ⟨_, -, _, -, _, -, _, -, _, -, _, -, _, -, rfl⟩ := hr
  rfl

/-- A struct handle names its type without type arguments. -/
theorem structIdAt_typeArgs {m : CompiledModule} {h : StructHandleIndex} {s : StructId}
    (hs : m.structIdAt h = some s) : { s with typeArgs := [] } = s := by
  simp only [CompiledModule.structIdAt, bind, Option.bind_eq_some_iff, pure,
    Option.some.injEq] at hs
  obtain ⟨_, -, _, -, _, -, rfl⟩ := hs
  rfl

theorem withTypeArgs_self {f : FunctionRef} (h : f.typeArgs = []) :
    { f with typeArgs := [] } = f := by
  cases f
  simp_all

/-- An instance without type arguments of a function without type parameters
satisfies its constraints. -/
theorem typeArgsOk_nil {f : FunctionRef} {info : FunctionInfo} (hf : P.function? f = some info)
    (hta : f.typeArgs = []) (htp : info.handle.typeParameters = []) : P.typeArgsOk f = true := by
  simp [Program.typeArgsOk, hf, hta, htp, RuntimeType.argumentsWellFormed]

/-- In the non-generic subset, a function's parameter types are the same at
every instance. -/
theorem paramTypes_nonGeneric (hs : P.SupportedNonGeneric) (f : FunctionRef) :
    P.paramTypes f = P.paramTypes { f with typeArgs := [] } := by
  cases hf : P.function? f with
  | none => simp [Program.paramTypes, function?_typeArgs, hf]
  | some info =>
    obtain ⟨hm, -, hfh⟩ := function?_lookup hf
    obtain ⟨sig, hsig, hsome⟩ := ((nonGeneric_module hs hm).1 info.handle
      (Array.mem_toList_iff.2 (Array.mem_of_getElem? hfh))).2.1
    obtain ⟨tys, htys⟩ := Option.isSome_iff_exists.1 hsome
    simp [Program.paramTypes, function?_typeArgs, hf, hsig, ofSignatureWith_closed htys]

/-- In the non-generic subset, a function's return types are the same at every
instance. -/
theorem returnTypes_nonGeneric (hs : P.SupportedNonGeneric) (f : FunctionRef) :
    P.returnTypes f = P.returnTypes { f with typeArgs := [] } := by
  cases hf : P.function? f with
  | none => simp [Program.returnTypes, function?_typeArgs, hf]
  | some info =>
    obtain ⟨hm, -, hfh⟩ := function?_lookup hf
    obtain ⟨sig, hsig, hsome⟩ := ((nonGeneric_module hs hm).1 info.handle
      (Array.mem_toList_iff.2 (Array.mem_of_getElem? hfh))).2.2
    obtain ⟨tys, htys⟩ := Option.isSome_iff_exists.1 hsome
    simp [Program.returnTypes, function?_typeArgs, hf, hsig, ofSignatureWith_closed htys]

/-- In the non-generic subset, a function's local types are the same at every
instance. -/
theorem localTypes_nonGeneric (hs : P.SupportedNonGeneric) (f : FunctionRef) :
    P.localTypes f = P.localTypes { f with typeArgs := [] } := by
  cases hf : P.function? f with
  | none => simp [Program.localTypes, function?_typeArgs, hf]
  | some info =>
    obtain ⟨hm, hd, hfh⟩ := function?_lookup hf
    have hmg := nonGeneric_module hs hm
    obtain ⟨sig, hsig, hsome⟩ := (hmg.1 info.handle
      (Array.mem_toList_iff.2 (Array.mem_of_getElem? hfh))).2.1
    obtain ⟨tys, htys⟩ := Option.isSome_iff_exists.1 hsome
    cases hc : info.definition.code with
    | none => simp [Program.localTypes, function?_typeArgs, hf, hc]
    | some code =>
      obtain ⟨lsig, hlsig, hlsome⟩ := (hmg.2.2.2.2.2 info.definition
        (Array.mem_toList_iff.2 (Array.mem_of_getElem? hd)) code hc).1
      obtain ⟨ltys, hltys⟩ := Option.isSome_iff_exists.1 hlsome
      simp [Program.localTypes, function?_typeArgs, hf, hc, hsig, hlsig,
        ofSignatureWith_closed htys, ofSignatureWith_closed hltys]

end

/-- The type arguments of an instance of a function without type parameters,
when they instantiate it: none. -/
theorem typeArgs_nil_of_wellFormed {P : Program} {args : List RuntimeType}
    (h : RuntimeType.argumentsWellFormed P args [] = true) : args = [] := by
  cases args with
  | nil => rfl
  | cons _ _ => simp [RuntimeType.argumentsWellFormed] at h

end MoveBytecode.Proofs
