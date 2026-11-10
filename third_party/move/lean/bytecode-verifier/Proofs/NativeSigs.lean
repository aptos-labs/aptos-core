-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.NativeFacts

/-!
# The types of the modeled natives

UNTRUSTED (checked by the kernel). A modeled native (`Program.nativeShape?`)
declares the parameter and return tokens of its entry of `modeledNatives`, so
its parameter and return types at an instance are the entry's patterns
translated under the instance's type arguments (`nativeShape_entry`). In
particular a borrow-shape native is `vector::borrow`, `vector::borrow_mut` or
`signer::borrow_address` (`borrow_sigs`), and a native of another shape
returns values only (`returns_values`).
-/

namespace MoveBytecode.Proofs

variable {P : Program}

mutual
/-- The value type a pattern stands for, under type arguments `τs`. -/
def _root_.MoveBytecode.TokenPattern.valueType (τs : List RuntimeType) :
    TokenPattern → Option RuntimeType
  | .bool => some .bool
  | .u8 => some (.int .u8)
  | .u64 => some (.int .u64)
  | .address => some .address
  | .signer => some .signer
  | .vector p => (p.valueType τs).map .vector
  | .typeParameter i => τs[i]?
  | .struct module name => some (.struct { module := ⟨stdAddress, module⟩, name })
  | .structInstantiation module name ps =>
    (TokenPattern.valueTypes τs ps).map fun args =>
      .struct { module := ⟨stdAddress, module⟩, name, typeArgs := args }
  | .reference _ => none
  | .mutableReference _ => none

/-- The value types patterns stand for, under type arguments `τs`. -/
def _root_.MoveBytecode.TokenPattern.valueTypes (τs : List RuntimeType) :
    List TokenPattern → Option (List RuntimeType)
  | [] => some []
  | p :: ps => do pure ((← p.valueType τs) :: (← TokenPattern.valueTypes τs ps))
end

/-- The slot type a pattern stands for: a value type, or a reference to one. -/
def _root_.MoveBytecode.TokenPattern.slotType (τs : List RuntimeType) :
    TokenPattern → Option RuntimeType
  | .reference p => (p.valueType τs).map (.ref false)
  | .mutableReference p => (p.valueType τs).map (.ref true)
  | p => p.valueType τs

theorem valueTypes_length {τs : List RuntimeType} :
    ∀ {ps : List TokenPattern} {ts : List RuntimeType}, TokenPattern.valueTypes τs ps = some ts →
      ts.length = ps.length
  | [], ts, h => by simp [TokenPattern.valueTypes] at h; subst h; rfl
  | p :: ps, ts, h => by
    simp only [TokenPattern.valueTypes, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨t, -, ts', hts', rfl⟩ := h
    simp [valueTypes_length hts']

mutual
theorem valueType_of_matches {m : CompiledModule} {τs : List RuntimeType} :
    ∀ {p : TokenPattern} {t : SignatureToken}, TokenPattern.matches m p t = true →
      RuntimeType.ofTokenWith m τs t = p.valueType τs
  | .vector p, t, h => by
    cases t <;> simp [TokenPattern.matches] at h
    simp [RuntimeType.ofTokenWith, TokenPattern.valueType, valueType_of_matches h]
  | .struct module name, t, hm => by
    cases t <;> simp only [TokenPattern.matches, Bool.false_eq_true] at hm
    rename_i h
    simp only [Bool.and_eq_true, beq_iff_eq] at hm
    obtain ⟨hsh, hid⟩ := hm
    cases hs : m.structHandles[h]? with
    | none => rw [hs] at hsh; simp at hsh
    | some sh =>
      rw [hs] at hsh
      simp only [Option.any_some] at hsh
      simp [RuntimeType.ofTokenWith, hs, List.isEmpty_iff.1 hsh, hid, TokenPattern.valueType]
  | .structInstantiation module name ps, t, hm => by
    cases t <;> simp only [TokenPattern.matches, Bool.false_eq_true] at hm
    rename_i h ts
    simp only [Bool.and_eq_true, beq_iff_eq] at hm
    obtain ⟨⟨hsh, hid⟩, hall⟩ := hm
    have ih := valueTypes_of_matchAll (τs := τs) hall
    cases hs : m.structHandles[h]? with
    | none => rw [hs] at hsh; simp at hsh
    | some sh =>
      rw [hs] at hsh
      simp only [Option.any_some, beq_iff_eq] at hsh
      simp only [RuntimeType.ofTokenWith, hs, hid, ih, TokenPattern.valueType, bind,
        Option.bind_some]
      cases hv : TokenPattern.valueTypes τs ps with
      | none => rfl
      | some args =>
        simp [valueTypes_length hv, hsh, pure]
  | .typeParameter i, t, h => by
    cases t <;> simp [TokenPattern.matches] at h
    subst h; rfl
  | .bool, t, h | .u8, t, h | .u64, t, h | .address, t, h | .signer, t, h => by
    cases t <;> simp_all [TokenPattern.matches, RuntimeType.ofTokenWith, TokenPattern.valueType]
  | .reference _, t, h | .mutableReference _, t, h => by
    cases t <;> simp_all [TokenPattern.matches, RuntimeType.ofTokenWith, TokenPattern.valueType]

theorem valueTypes_of_matchAll {m : CompiledModule} {τs : List RuntimeType} :
    ∀ {ps : List TokenPattern} {ts : List SignatureToken}, TokenPattern.matchAll m ps ts = true →
      RuntimeType.ofTokenListWith m τs ts = TokenPattern.valueTypes τs ps
  | [], [], _ => rfl
  | p :: ps, t :: ts, h => by
    simp only [TokenPattern.matchAll, Bool.and_eq_true] at h
    simp only [RuntimeType.ofTokenListWith, TokenPattern.valueTypes, valueType_of_matches h.1,
      valueTypes_of_matchAll h.2]
  | [], _ :: _, h => by simp [TokenPattern.matchAll] at h
  | _ :: _, [], h => by simp [TokenPattern.matchAll] at h
end

theorem slotType_of_matches {m : CompiledModule} {τs : List RuntimeType} {p : TokenPattern}
    {t : SignatureToken} (h : TokenPattern.matches m p t = true) :
    RuntimeType.ofSlotTokenWith m τs t = p.slotType τs := by
  cases p with
  | reference q =>
    cases t <;> simp only [TokenPattern.matches, Bool.false_eq_true] at h
    simp [RuntimeType.ofSlotTokenWith, TokenPattern.slotType, valueType_of_matches (τs := τs) h]
  | mutableReference q =>
    cases t <;> simp only [TokenPattern.matches, Bool.false_eq_true] at h
    simp [RuntimeType.ofSlotTokenWith, TokenPattern.slotType, valueType_of_matches (τs := τs) h]
  | _ =>
    have hv := valueType_of_matches (τs := τs) h
    cases t <;> (try simp only [TokenPattern.matches, Bool.false_eq_true] at h) <;>
      simp only [RuntimeType.ofSlotTokenWith, TokenPattern.slotType] <;> exact hv

theorem ofSignatureWith_of_matchAll {m : CompiledModule} {τs : List RuntimeType} :
    ∀ {ps : List TokenPattern} {ts : List SignatureToken}, TokenPattern.matchAll m ps ts = true →
      RuntimeType.ofSignatureWith m τs ts = ps.mapM (·.slotType τs)
  | [], [], _ => rfl
  | p :: ps, t :: ts, h => by
    simp only [TokenPattern.matchAll, Bool.and_eq_true] at h
    have ih := ofSignatureWith_of_matchAll (τs := τs) h.2
    unfold RuntimeType.ofSignatureWith at ih ⊢
    rw [List.mapM_cons, List.mapM_cons, ih, slotType_of_matches h.1]
  | [], _ :: _, h => by simp [TokenPattern.matchAll] at h
  | _ :: _, [], h => by simp [TokenPattern.matchAll] at h

/-- **A modeled native's types are its entry's**, at its instance. -/
theorem nativeShape_entry {f : FunctionRef} {k : NativeShape} (h : P.nativeShape? f = some k) :
    ∃ info e, P.function? f = some info ∧ info.definition.code = none ∧ e ∈ modeledNatives ∧
      e.shape = k ∧ info.handle.typeParameters = e.typeParameters ∧
      P.paramTypes f = e.params.mapM (·.slotType f.typeArgs) ∧
      P.returnTypes f = e.returns.mapM (·.slotType f.typeArgs) := by
  simp only [Program.nativeShape?, bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, hf, h⟩ := h
  have hfl := function?_lookup hf
  unfold CompiledModule.modeledNative? at h
  split at h
  · simp at h
  next hcode =>
  simp only [bind, Option.bind_eq_some_iff, pure] at h
  obtain ⟨_, -, _, -, fh, hfh, params, hparams, returns, hreturns, e, he, hk⟩ := h
  split at hk
  · cases hk
  simp only [Option.some.injEq] at hk
  subst hk
  have hmem := List.mem_of_find?_eq_some he
  have hp := List.find?_some he
  simp only [Bool.and_eq_true, beq_iff_eq] at hp
  have hfh' : fh = info.handle := by
    simp only [CompiledModule.functionDefHandle] at hfh
    rw [hfl.2.2, Option.some.injEq] at hfh
    exact hfh.symm
  subst hfh'
  refine ⟨info, e, hf, by simpa using hcode, hmem, rfl, hp.1.1.2, ?_, ?_⟩
  · simp only [Program.paramTypes, hf, hparams, bind, Option.bind_some]
    exact ofSignatureWith_of_matchAll hp.1.2
  · simp only [Program.returnTypes, hf, hreturns, bind, Option.bind_some]
    exact ofSignatureWith_of_matchAll hp.2

/-- The table type `0x1::table::Table<κ, ν>`. -/
def tableStructId (κ ν : RuntimeType) : StructId :=
  { module := tableModuleId, name := "Table", typeArgs := [κ, ν] }

theorem tableStructId_isTable (κ ν : RuntimeType) : (tableStructId κ ν).isTable = true := by
  simp [tableStructId, StructIdOf.isTable]

/-- **The borrow-shape natives' types**: `vector::borrow`, `vector::borrow_mut`,
`signer::borrow_address`, `table::borrow_box` and `table::borrow_box_mut`. -/
theorem borrow_sigs {f : FunctionRef} {ptys tys : List RuntimeType}
    (h : P.nativeShape? f = some .borrow) (hp : P.paramTypes f = some ptys)
    (hr : P.returnTypes f = some tys) :
    (∃ τ, ptys = [.ref false (.vector τ), .int .u64] ∧ tys = [.ref false τ]) ∨
    (∃ τ, ptys = [.ref true (.vector τ), .int .u64] ∧ tys = [.ref true τ]) ∨
    (ptys = [.ref false .signer] ∧ tys = [.ref false .address]) ∨
    (∃ κ ν, f.typeArgs[0]? = some κ ∧
      ptys = [.ref false (.struct (tableStructId κ ν)), κ] ∧ tys = [.ref false (tableEntryType ν)]) ∨
    (∃ κ ν, f.typeArgs[0]? = some κ ∧
      ptys = [.ref true (.struct (tableStructId κ ν)), κ] ∧ tys = [.ref true (tableEntryType ν)]) := by
  obtain ⟨info, e, -, -, he, hk, -, hpe, hre⟩ := nativeShape_entry h
  rw [hp] at hpe
  rw [hr] at hre
  simp only [modeledNatives, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl |
    rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp at hk
  · -- `vector::borrow`
    left
    cases hτ : f.typeArgs[0]? with
    | none => simp [TokenPattern.slotType, TokenPattern.valueType, hτ] at hpe
    | some τ =>
      simp [TokenPattern.slotType, TokenPattern.valueType, hτ] at hpe hre
      exact ⟨τ, hpe, hre⟩
  · -- `vector::borrow_mut`
    right; left
    cases hτ : f.typeArgs[0]? with
    | none => simp [TokenPattern.slotType, TokenPattern.valueType, hτ] at hpe
    | some τ =>
      simp [TokenPattern.slotType, TokenPattern.valueType, hτ] at hpe hre
      exact ⟨τ, hpe, hre⟩
  · -- `signer::borrow_address`
    right; right; left
    simp [TokenPattern.slotType, TokenPattern.valueType] at hpe hre
    exact ⟨hpe, hre⟩
  all_goals
    right; right; right
    cases hκ : f.typeArgs[0]? with
    | none =>
      simp [TokenPattern.slotType, TokenPattern.valueType, TokenPattern.valueTypes, hκ] at hpe
    | some κ =>
      cases hν : f.typeArgs[1]? with
      | none =>
        simp [TokenPattern.slotType, TokenPattern.valueType, TokenPattern.valueTypes, hκ, hν] at hpe
      | some ν =>
        simp [TokenPattern.slotType, TokenPattern.valueType, TokenPattern.valueTypes, hκ, hν]
          at hpe hre
        first
        | exact Or.inl ⟨κ, ν, rfl, hpe, hre⟩
        | exact Or.inr ⟨κ, ν, rfl, hpe, hre⟩

theorem slotType_values {τs : List RuntimeType} (hτ : ∀ x ∈ τs, x.isValue = true) :
    ∀ {p : TokenPattern} {t : RuntimeType}, p.valueType τs = some t → t.isValue = true
  | .bool, _, h | .u8, _, h | .u64, _, h | .address, _, h | .signer, _, h
  | .struct _ _, _, h => by simp [TokenPattern.valueType] at h; subst h; rfl
  | .structInstantiation _ _ _, _, h => by
    simp only [TokenPattern.valueType, Option.map_eq_some_iff] at h
    obtain ⟨_, -, rfl⟩ := h
    rfl
  | .vector p, t, h => by
    simp only [TokenPattern.valueType, Option.map_eq_some_iff] at h
    obtain ⟨u, hu, rfl⟩ := h
    simp only [RuntimeType.isValue]
    exact slotType_values hτ hu
  | .typeParameter i, _, h => hτ _ (List.mem_of_getElem? h)
  | .reference _, _, h | .mutableReference _, _, h => by simp [TokenPattern.valueType] at h

/-- **A modeled native that is not of the borrow shape returns values**, at an
instance whose type arguments are value types. -/
theorem returns_values {f : FunctionRef} {k : NativeShape} {tys : List RuntimeType}
    (h : P.nativeShape? f = some k) (hk : k ≠ .borrow) (hr : P.returnTypes f = some tys)
    (hτ : ∀ x ∈ f.typeArgs, x.isValue = true) : ∀ t ∈ tys, t.isValue = true := by
  obtain ⟨info, e, -, -, he, rfl, -, -, hre⟩ := nativeShape_entry h
  rw [hr] at hre
  intro t ht
  obtain ⟨p, hp, hpt⟩ := mapM_some_mem hre.symm t ht
  simp only [modeledNatives, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl |
    rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp at hk <;> simp at hp <;> subst hp <;>
    exact slotType_values hτ hpt

/-! ## Places -/

theorem placeType?_nil (t : RuntimeType) : RuntimeType.placeType? P t [] = some t := by
  simp [RuntimeType.placeType?]

theorem placeType?_cons {t : RuntimeType} {st : PathStep} {q : List PathStep} {u : RuntimeType}
    (h : RuntimeType.placeType? P t (st :: q) = some u) :
    ∃ t', (match t, st with
      | .struct s, .field i => (P.fieldTypes s).bind (·[i]?)
      | .struct s, .variantField tag i => (P.variantFieldTypes s tag).bind (·[i]?)
      | .vector elem, .element _ => some elem
      | .signer, .signerAddress => some .address
      | .struct s, .tableEntry _ =>
        match s.isTable, s.typeArgs with
        | true, [_, v] => some (tableEntryType v)
        | _, _ => none
      | _, _ => none) = some t' ∧ RuntimeType.placeType? P t' q = some u := by
  simp only [RuntimeType.placeType?, Option.bind_eq_some_iff] at h
  exact h

/-- An address has no place inside it. -/
theorem placeType?_address {q : List PathStep} {u : RuntimeType}
    (h : RuntimeType.placeType? P .address q = some u) : q = [] := by
  cases q with
  | nil => rfl
  | cons st q =>
    obtain ⟨t', ht', -⟩ := placeType?_cons h
    cases st <;> simp at ht'

/-- A function type has no place inside it. -/
theorem placeType?_function {ps rs : List RuntimeType} {a : AbilitySet} {q : List PathStep}
    {u : RuntimeType} (h : RuntimeType.placeType? P (.function ps rs a) q = some u) : q = [] := by
  cases q with
  | nil => rfl
  | cons st q =>
    obtain ⟨t', ht', -⟩ := placeType?_cons h
    cases st <;> simp at ht'

/-- **A path through a signer's address ends there, at an address.** -/
theorem placeType?_writable : ∀ {t u : RuntimeType} {q : List PathStep},
    RuntimeType.placeType? P t q = some u → u ≠ .address → q.all PathStep.writable = true
  | _, _, [], _, _ => rfl
  | t, u, st :: q, h, hu => by
    obtain ⟨t', ht', hq⟩ := placeType?_cons h
    cases st with
    | signerAddress =>
      cases t <;> simp at ht'
      subst ht'
      have := placeType?_address hq
      subst this
      simp [RuntimeType.placeType?] at hq
      exact absurd hq.symm hu
    | field _ => simpa [PathStep.writable] using placeType?_writable hq hu
    | variantField _ _ => simpa [PathStep.writable] using placeType?_writable hq hu
    | element _ => simpa [PathStep.writable] using placeType?_writable hq hu
    | tableEntry _ => simpa [PathStep.writable] using placeType?_writable hq hu

theorem vector_ne (τ : RuntimeType) : RuntimeType.vector τ ≠ τ := by
  intro h
  have := congrArg sizeOf h
  simp at this

/-- **A place of a vector borrow is at its type**: an element's place whose type
is assignable to the element type is of the element type. -/
theorem vector_place_type {τ t : RuntimeType} {q : List PathStep}
    (h : RuntimeType.placeType? P (.vector τ) q = some t) (ha : t.assignable τ = true) :
    t = τ := by
  cases q with
  | nil =>
    simp only [RuntimeType.placeType?, Option.some.injEq] at h
    subst h
    cases τ with
    | function ps rs a => simp [RuntimeType.assignable] at ha
    | ref m u => cases m <;> simp [RuntimeType.assignable] at ha
    | _ => exact absurd (assignable_rigid ha rfl) (vector_ne _)
  | cons st q =>
    obtain ⟨t', ht', hq⟩ := placeType?_cons h
    cases st <;> simp at ht'
    subst ht'
    cases τ with
    | function ps rs a =>
      have := placeType?_function hq
      subst this
      simp [RuntimeType.placeType?] at hq
      exact hq.symm
    | ref m u =>
      have : q = [] := by
        cases q with
        | nil => rfl
        | cons st q =>
          obtain ⟨t', ht', -⟩ := placeType?_cons hq
          cases st <;> simp at ht'
      subst this
      simp [RuntimeType.placeType?] at hq
      exact hq.symm
    | _ => exact assignable_rigid ha rfl

/-- The place of a mutable vector borrow is writable. -/
theorem vector_place_writable {τ : RuntimeType} {q : List PathStep}
    (h : RuntimeType.placeType? P (.vector τ) q = some τ) : q.all PathStep.writable = true := by
  cases q with
  | nil => rfl
  | cons st q =>
    obtain ⟨t', ht', hq⟩ := placeType?_cons h
    cases st <;> simp at ht'
    subst ht'
    simp only [List.all_cons, PathStep.writable, Bool.true_and]
    by_cases hτ : τ = .address
    · subst hτ
      rw [placeType?_address hq]; rfl
    · exact placeType?_writable hq hτ

theorem conformsAll_nil_right {vs : List Value} (h : conformsAll P vs [] = true) : vs = [] := by
  cases vs with
  | nil => rfl
  | cons v vs => simp [conformsAll] at h

theorem tableEntryType_ne_address (ν : RuntimeType) : tableEntryType ν ≠ .address := by
  simp [tableEntryType]

/-- The type a borrow-shape native borrows inside is not its result's target. -/
theorem borrow_ne {X tgt : RuntimeType}
    (h : X = .vector tgt ∨ (X = .signer ∧ tgt = .address) ∨
      ∃ κ ν, X = .struct (tableStructId κ ν) ∧ tgt = tableEntryType ν) : X ≠ tgt := by
  rcases h with rfl | ⟨rfl, rfl⟩ | ⟨κ, ν, rfl, rfl⟩
  · exact vector_ne _
  · simp
  · simp [tableStructId, tableEntryType]

/-- The result target of a borrow-shape native is a value type when what it
borrows inside is. -/
theorem borrow_value {X tgt : RuntimeType}
    (h : X = .vector tgt ∨ (X = .signer ∧ tgt = .address) ∨
      ∃ κ ν, X = .struct (tableStructId κ ν) ∧ tgt = tableEntryType ν) (hX : X.isValue = true) :
    tgt.isValue = true := by
  rcases h with rfl | ⟨rfl, rfl⟩ | ⟨κ, ν, rfl, rfl⟩
  · simpa [RuntimeType.isValue] using hX
  · rfl
  · rfl

/-- **What a borrow-shape native returns**: one reference, to a place inside
its first argument, a reference, of exactly the place's declared type there,
never more mutable than the argument, writable when mutable; and nothing
else. -/
theorem borrow_result {f : FunctionRef} {ptys tys : List RuntimeType}
    {views vals writes : List Value} {places : List NativePlace}
    (hshape : P.nativeShape? f = some .borrow) (hpar : P.paramTypes f = some ptys)
    (hret : P.returnTypes f = some tys) (hτ : ∀ x ∈ f.typeArgs, x.isValue = true)
    (hfits : P.nativeResultFits f tys views vals writes places = true) :
    writes = [] ∧ vals = [] ∧ ∃ m am X tgt p, tys = [.ref m tgt] ∧ places = [p] ∧ p.arg = 0 ∧
      ptys[0]? = some (.ref am X) ∧ (m = true → am = true) ∧ rigid X = true ∧
      (X = .vector tgt ∨ (X = .signer ∧ tgt = .address) ∨
        ∃ κ ν, X = .struct (tableStructId κ ν) ∧ tgt = tableEntryType ν) ∧
      RuntimeType.placeType? P X p.path = some tgt ∧
      (m = true → p.path.all PathStep.writable = true) ∧
      ∃ view, views[0]? = some view ∧ (view.get? p.path).isSome = true := by
  simp only [Program.nativeResultFits, hshape, hpar, Bool.and_eq_true, List.isEmpty_iff,
    beq_iff_eq] at hfits
  obtain ⟨⟨⟨hw, hv⟩, hpl⟩, hall⟩ := hfits
  have hrv : ∀ i t, ptys[i + 1]? = some t → t.isValue = true := by
    rcases borrow_sigs hshape hpar hret with ⟨τ, rfl, -⟩ | ⟨τ, rfl, -⟩ | ⟨rfl, -⟩ |
      ⟨κ, ν, hκ, rfl, -⟩ | ⟨κ, ν, hκ, rfl, -⟩ <;> intro i t ht <;> (cases i <;> simp at ht) <;>
      subst ht <;> first | rfl | exact hτ _ (List.mem_of_getElem? hκ)
  rcases borrow_sigs hshape hpar hret with ⟨τ, hp, rfl⟩ | ⟨τ, hp, rfl⟩ | ⟨hp, rfl⟩ |
    ⟨κ, ν, -, hp, rfl⟩ | ⟨κ, ν, -, hp, rfl⟩ <;>
  · simp only [List.filter_cons, List.filter_nil, RuntimeType.isValue, Bool.not_false,
      Bool.false_eq_true, ↓reduceIte, List.length_cons, List.length_nil] at hv hpl hall
    refine ⟨hw, conformsAll_nil_right hv, ?_⟩
    match places, hpl, hall with
    | [p], _, hall =>
      simp only [List.zip_cons_cons, List.zip_nil_right, List.all_cons, List.all_nil,
        Bool.and_true] at hall
      have harg : p.arg = 0 := by
        cases hk : p.arg with
        | zero => rfl
        | succ k =>
          exfalso
          unfold nativePlaceFits at hall
          rw [hk] at hall
          cases hpk : ptys[k + 1]? with
          | none => simp [hpk] at hall
          | some t =>
            have := hrv k t hpk
            cases t <;> simp [hpk, RuntimeType.isValue] at hall this
      subst hp
      cases hv0 : views[0]? with
      | none => simp [nativePlaceFits, harg, hv0] at hall
      | some view =>
      simp only [nativePlaceFits, harg, hv0, List.getElem?_cons_zero, Bool.and_eq_true,
        Bool.or_eq_true, Bool.not_eq_true'] at hall
      obtain ⟨⟨hmut, hget⟩, hty⟩ := hall
      split at hty
      case h_2 => cases hty
      next t ht =>
      have hteq := hty
      first
      | (replace hteq := vector_place_type ht (by simpa [RuntimeType.assignable] using hteq))
      | (replace hteq := assignable_rigid (by simpa [RuntimeType.assignable] using hteq) rfl)
      | (simp [RuntimeType.assignable] at hteq)
      subst hteq
      refine ⟨_, _, _, _, p, rfl, rfl, harg, rfl, by simp, rfl,
        by first | exact Or.inr (Or.inr ⟨_, _, rfl, rfl⟩) | simp, ht, ?_, view, rfl, hget⟩
      intro hm
      first
      | exact vector_place_writable ht
      | exact placeType?_writable ht (tableEntryType_ne_address _)
      | exact absurd hm (by decide)

/-- The mutable references among the arguments. -/
def mutRefsOf (args : List Value) : List Reference :=
  args.filterMap fun v => match v with
    | .ref r => if r.mutable then some r else none
    | _ => none

theorem writtenReferences_update {f : FunctionRef} (h : P.nativeShape? f = some .update)
    (args : List Value) : P.writtenReferences f args = mutRefsOf args := by
  simp only [Program.writtenReferences, h, beq_self_eq_true, ↓reduceIte]
  rfl

theorem writtenReferences_other {f : FunctionRef} (h : P.nativeShape? f ≠ some .update)
    (args : List Value) : P.writtenReferences f args = [] := by
  simp [Program.writtenReferences, h]

/-- **The written-back references are typed by the mutable parameter types**:
a mutable reference conforms only to a mutable reference type with its
target, and nothing else conforms to one. -/
theorem mutRefs_targets :
    ∀ {args : List Value} {ptys : List RuntimeType}, conformsAll P args ptys = true →
      (mutRefsOf args).map (·.target) = RuntimeType.mutableTargets ptys
  | [], [], _ => rfl
  | v :: args, t :: ptys, h => by
    simp only [conformsAll, Bool.and_eq_true] at h
    obtain ⟨⟨hva, -⟩, hrest⟩ := h
    have ih := mutRefs_targets hrest
    simp only [mutRefsOf, RuntimeType.mutableTargets, List.filterMap_cons] at ih ⊢
    cases v with
    | ref r =>
      cases hm : r.mutable with
      | true =>
        simp only [Value.type, hm] at hva
        have : t = .ref true r.target := by
          cases t <;> simp_all [RuntimeType.assignable]
        subst this
        simp [hm, ih]
      | false =>
        simp only [Value.type, hm] at hva
        have : ∀ u, t ≠ .ref true u := by
          intro u hu; subst hu; simp [RuntimeType.assignable] at hva
        cases t with
        | ref b u =>
          cases b
          · simpa [hm] using ih
          · exact absurd rfl (this u)
        | _ => simpa [hm] using ih
    | _ =>
      have : ∀ u, t ≠ .ref true u := by
        intro u hu; subst hu; simp [Value.type, RuntimeType.assignable] at hva
      cases t with
      | ref b u =>
        cases b
        · simpa using ih
        · exact absurd rfl (this u)
      | _ => simpa using ih
  | [], _ :: _, h => by simp [conformsAll] at h
  | _ :: _, [], h => by simp [conformsAll] at h

/-- The references among the arguments. -/
def refsOf (args : List Value) : List Reference :=
  args.filterMap fun v => match v with
    | .ref r => some r
    | _ => none

theorem mutRefsOf_eq : ∀ args : List Value, mutRefsOf args = (refsOf args).filter (·.mutable)
  | [] => rfl
  | v :: args => by
    have ih := mutRefsOf_eq args
    simp only [mutRefsOf, refsOf, List.filterMap_cons] at ih ⊢
    cases v with
    | ref r => cases hm : r.mutable <;> simp [hm, ih]
    | _ => exact ih

theorem pairwise_of_zipIdx {α : Type} {R : α → α → Prop} :
    ∀ {L : List α} {n : Nat}, (∀ x ∈ L.zipIdx n, ∀ y ∈ L.zipIdx n, x.2 ≠ y.2 → R x.1 y.1) →
      L.Pairwise R
  | [], _, _ => List.Pairwise.nil
  | a :: l, n, h => by
    rw [List.zipIdx_cons] at h
    refine List.Pairwise.cons (fun b hb => ?_) (pairwise_of_zipIdx (n := n + 1) fun x hx y hy hne =>
      h x (List.mem_cons_of_mem _ hx) y (List.mem_cons_of_mem _ hy) hne)
    obtain ⟨k, hk, rfl⟩ := List.getElem_of_mem hb
    have hmem : (l[k], n + 1 + k) ∈ l.zipIdx (n + 1) := by
      rw [List.mem_iff_getElem?]
      exact ⟨k, by simp [hk]⟩
    exact h (a, n) List.mem_cons_self _ (List.mem_cons_of_mem _ hmem) (by simp; omega)

/-- **The mutable reference arguments of a native call overlap no other
reference argument**, as the semantics checked. -/
theorem exclusive_pairwise {args : List Value} (h : Step.exclusiveArguments args = true) :
    (refsOf args).Pairwise (fun a b => a.mutable = true → a.location.overlaps b.location = false) := by
  unfold Step.exclusiveArguments at h
  rw [List.all_eq_true] at h
  refine pairwise_of_zipIdx (n := 0) fun x hx y hy hne hm => ?_
  have := h x hx
  simp only [hm, Bool.not_true, Bool.false_or, List.all_eq_true] at this
  have := this y hy
  simp only [Bool.or_eq_true, beq_iff_eq, Bool.not_eq_true'] at this
  rcases this with h1 | h1
  · exact absurd h1 hne
  · exact h1

/-- The written-back references overlap one another nowhere. -/
theorem mutRefs_disjoint {args : List Value} (h : Step.exclusiveArguments args = true) :
    (mutRefsOf args).Pairwise (fun a b => a.location.overlaps b.location = false) := by
  rw [mutRefsOf_eq]
  have := (exclusive_pairwise h).filter (fun r : Reference => r.mutable)
  refine this.imp_of_mem fun {a b} ha _ hab => hab ?_
  simpa using (List.mem_filter.1 ha).2

theorem mem_mutRefsOf {args : List Value} {r : Reference} (h : r ∈ mutRefsOf args) :
    Value.ref r ∈ args ∧ r.mutable = true := by
  simp only [mutRefsOf, List.mem_filterMap] at h
  obtain ⟨v, hv, hvr⟩ := h
  cases v with
  | ref r' =>
    simp only at hvr
    split at hvr
    · next hm => simp only [Option.some.injEq] at hvr; subst hvr; exact ⟨hv, hm⟩
    · cases hvr
  | _ => cases hvr

/-- An argument at a reference parameter type is a reference, of that
mutability, with a target assignable to the parameter's. -/
theorem conformsAll_ref_at {args : List Value} {ptys : List RuntimeType} {k : Nat} {m : Bool}
    {X : RuntimeType} (h : conformsAll P args ptys = true) (hk : ptys[k]? = some (.ref m X)) :
    ∃ r, args[k]? = some (.ref r) ∧ r.mutable = m ∧ r.target.assignable X = true := by
  induction args generalizing ptys k with
  | nil => cases ptys <;> simp_all [conformsAll]
  | cons v args ih =>
    cases ptys with
    | nil => simp at hk
    | cons t ptys =>
      simp only [conformsAll, Bool.and_eq_true] at h
      cases k with
      | succ k => simpa using ih h.2 (by simpa using hk)
      | zero =>
        simp only [List.getElem?_cons_zero, Option.some.injEq] at hk
        subst hk
        have hva := h.1.1
        cases v with
        | ref r =>
          refine ⟨r, rfl, ?_⟩
          simp only [Value.type] at hva
          cases hm : r.mutable <;> cases m <;> simp_all [RuntimeType.assignable]
        | _ => simp [Value.type, RuntimeType.assignable] at hva

/-! ## What the native returns -/

theorem fits_writes_nil {f : FunctionRef} {k : NativeShape} (hshape : P.nativeShape? f = some k)
    (hk : k ≠ .update) {tys : List RuntimeType} {views vals writes : List Value}
    {places : List NativePlace} (h : P.nativeResultFits f tys views vals writes places = true) :
    writes = [] := by
  cases k <;> simp_all [Program.nativeResultFits]
  split at h <;> simp_all

theorem fits_values {f : FunctionRef} {k : NativeShape} (hshape : P.nativeShape? f = some k)
    (hk : k ≠ .borrow) {tys : List RuntimeType} {views vals writes : List Value}
    {places : List NativePlace} (h : P.nativeResultFits f tys views vals writes places = true) :
    conformsAll P vals tys = true ∧ places = [] := by
  cases k <;> simp_all [Program.nativeResultFits]
  split at h <;> simp_all

theorem get?_append : ∀ (p q : List PathStep) (v : Value), v.get? (p ++ q) = (v.get? p).bind (·.get? q)
  | [], q, v => by simp [Value.get?]
  | st :: p, q, v => by
    simp only [List.cons_append, Value.get?, bind, Option.bind]
    cases v.child? st with
    | none => rfl
    | some c => exact get?_append p q c

theorem read?_descend {s : State} {l : Location} {q : List PathStep} :
    s.read? (l.descend q) = (s.read? l).bind (·.get? q) := by
  simp only [State.read?, Location.descend, bind, Option.bind]
  cases s.root? l.root with
  | none => rfl
  | some v => exact get?_append _ _ v

end MoveBytecode.Proofs
