-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.StackUsage

/-!
# The type certificate from the type-safety pass

UNTRUSTED (checked by the kernel). When the stack-usage and type-safety
passes accept a function, the abstract stack of signature tokens that type
safety computes is a certificate: empty at every block start, and
transformed by `typeInstr` along every edge. Every instruction type safety
accepts has the specification's arity as its stack effect
(`effect_agrees_typed`), which gives L4 (`stackDiscipline_of_verifyFunction`).

The type-safety pass threads one stack through all blocks and never resets
it. Each instruction changes the stack's length by exactly its stack effect,
so the length tracks the stack-usage heights, which are zero at every block
boundary; hence every block starts and ends with an empty stack.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Pops -/

/-- Two lists related pointwise. -/
inductive AllPairs {α β : Type} (R : α → β → Prop) : List α → List β → Prop where
  | nil : AllPairs R [] []
  | cons {a : α} {b : β} {as : List α} {bs : List β} :
      R a b → AllPairs R as bs → AllPairs R (a :: as) (b :: bs)

theorem allPairs_replicate {α β : Type} {R : α → β → Prop} {b : β} :
    ∀ {l : List α}, (∀ a ∈ l, R a b) → AllPairs R l (List.replicate l.length b)
  | [], _ => .nil
  | _ :: _, h => .cons (h _ (List.mem_cons_self ..))
      (allPairs_replicate fun a ha => h a (List.mem_cons_of_mem _ ha))

theorem popType_ok {st : List SignatureToken} {t : SignatureToken} {rest : List SignatureToken} :
    popType st = .ok (t, rest) ↔ st = t :: rest := by
  cases st <;> simp [popType, reject, pure, Except.pure]

theorem popType_ok' {st : List SignatureToken} {p : SignatureToken × List SignatureToken} :
    popType st = .ok p ↔ st = p.1 :: p.2 := by
  cases st <;> cases p <;> simp [popType, reject, pure, Except.pure] <;> grind

theorem popTwo_ok {st : List SignatureToken} {a b : SignatureToken} {rest : List SignatureToken} :
    popTwo st = .ok (a, b, rest) ↔ st = a :: b :: rest := by
  unfold popTwo
  rcases st with _ | ⟨x, _ | ⟨y, r⟩⟩ <;>
    simp [popType, reject, bind, Except.bind, pure, Except.pure] <;> grind

/-- Peel a successful run of the model in hypothesis `h`: binds, pops,
conditionals and matches, discarding the branches that reject or panic. -/
macro "peel_ok " h:ident : tactic => `(tactic| repeat' (first
  | (simp [reject, panicAt] at $h:ident; done)
  | (simp only [bind_ok] at $h:ident)
  | (obtain ⟨⟨_, _⟩, hp, $h:ident⟩ := $h:ident; rw [popType_ok'] at hp; subst hp)
  | (obtain ⟨_, _, $h:ident⟩ := $h:ident)
  | (split at $h:ident)
  | (simp only [Except.ok.injEq] at $h:ident; subst $h:ident)
  | (dsimp only at $h:ident)))

theorem popTwo_ok' {st : List SignatureToken}
    {p : SignatureToken × SignatureToken × List SignatureToken} :
    popTwo st = .ok p ↔ st = p.1 :: p.2.1 :: p.2.2 := by
  obtain ⟨a, b, r⟩ := p; exact popTwo_ok

/-- `popExpected` pops one argument per expected type, each assignable to it. -/
theorem popExpected_ok {status : String} {st st' : List SignatureToken} :
    ∀ {ex : List SignatureToken}, popExpected status st ex = .ok st' →
      ∃ args, st = args ++ st' ∧ args.length = ex.length ∧
        AllPairs (fun a p => isAssignableFrom p a = true) args ex.reverse := by
  intro ex h
  unfold popExpected at h
  generalize hr : ex.reverse = rev at h
  have hlen : ex.length = rev.length := by rw [← hr, List.length_reverse]
  rw [hlen]
  clear hr hlen
  induction rev generalizing st with
  | nil =>
    simp [List.foldlM, pure, Except.pure] at h
    exact ⟨[], by simp [h], rfl, AllPairs.nil⟩
  | cons p ps ih =>
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨st1, ⟨⟨arg, rest⟩, hp, h1⟩, h2⟩ := h
    rw [popType_ok] at hp
    by_cases ha : isAssignableFrom p arg = true
    · simp [ha, pure, Except.pure] at h1
      subst h1
      obtain ⟨args, hst, hl, hf⟩ := ih h2
      exact ⟨arg :: args, by simp [hp, hst], by simp [hl], AllPairs.cons ha hf⟩
    · simp [ha, reject] at h1

/-! ## Structs and vectors -/

theorem fieldsSignature_ok {sd : StructDefinition} {variant : Option Nat}
    {fs : List SignatureToken} (h : fieldsSignature sd variant = .ok fs) :
    (variant = none ∧ ∃ fields, sd.fieldInformation = .declared fields ∧
        fs = fields.map (·.signature)) ∨
      ∃ v variants vd, variant = some v ∧ sd.fieldInformation = .declaredVariants variants ∧
        variants[v]? = some vd ∧ fs = vd.fields.map (·.signature) := by
  unfold fieldsSignature at h
  cases hfi : sd.fieldInformation with
  | native => rw [hfi] at h; cases variant <;> simp [reject] at h
  | declared fields =>
    rw [hfi] at h
    cases variant with
    | none =>
      simp only [pure, Except.pure, Except.ok.injEq] at h
      exact Or.inl ⟨rfl, fields, rfl, h.symm⟩
    | some v => simp [reject] at h
  | declaredVariants variants =>
    rw [hfi] at h
    cases variant with
    | none => simp [reject] at h
    | some v =>
      cases hv : variants[v]? with
      | none => simp [hv, reject] at h
      | some vd =>
        simp only [hv, pure, Except.pure, Except.ok.injEq] at h
        exact Or.inr ⟨v, variants, vd, rfl, rfl, hv, h.symm⟩

theorem fieldCount_of_fieldsSignature {sd : StructDefinition} {variant : Option Nat}
    {fs : List SignatureToken} (h : fieldsSignature sd variant = .ok fs) :
    fieldCount sd.fieldInformation variant = fs.length := by
  rcases fieldsSignature_ok h with ⟨rfl, fields, hf, rfl⟩ | ⟨v, variants, vd, rfl, hf, hv, rfl⟩
  · simp [fieldCount, hf]
  · simp [fieldCount, hf, hv]

theorem typePack_ok {st st' : List SignatureToken} {sd : StructDefinition}
    {variant : Option Nat} (h : typePack st sd variant = .ok st') :
    ∃ fs args rest, fieldsSignature sd variant = .ok fs ∧ st = args ++ rest ∧
      args.length = fs.length ∧
      AllPairs (fun a p => isAssignableFrom p a = true) args fs.reverse ∧
      st' = .struct sd.structHandle :: rest := by
  unfold typePack at h
  simp only [bind_ok] at h
  obtain ⟨fs, hfs, rest, hpop, h⟩ := h
  simp only [pure, Except.pure, Except.ok.injEq] at h
  obtain ⟨args, hst, hlen, hall⟩ := popExpected_ok hpop
  exact ⟨fs, args, rest, hfs, hst, hlen, hall, h.symm⟩

theorem typeUnpack_ok {st st' : List SignatureToken} {sd : StructDefinition}
    {variant : Option Nat} (h : typeUnpack st sd variant = .ok st') :
    ∃ fs rest, st = .struct sd.structHandle :: rest ∧ fieldsSignature sd variant = .ok fs ∧
      st' = fs.reverse ++ rest := by
  unfold typeUnpack at h
  simp only [bind_ok] at h
  obtain ⟨⟨arg, rest⟩, hp, h⟩ := h
  rw [popType_ok] at hp
  by_cases hc : arg = .struct sd.structHandle
  · subst hc
    simp only [bne_self_eq_false, Bool.false_eq_true, if_false, pure, Except.pure, bind_ok] at h
    obtain ⟨fs, hfs, h⟩ := h
    simp only [Except.ok.injEq] at h
    exact ⟨fs, rest, hp, hfs, h.symm⟩
  · have : (arg != .struct sd.structHandle) = true := by simpa using hc
    simp [this, reject, bind, Except.bind] at h

theorem popElements_ok {elem : SignatureToken} :
    ∀ (l : List Nat) {st st' : List SignatureToken},
      l.foldlM (fun stack _ => do
        let (op, stack) ← popType stack
        if !isAssignableFrom elem op then reject "TYPE_MISMATCH" else pure stack) st = .ok st' →
      ∃ args, st = args ++ st' ∧ args.length = l.length ∧
        ∀ a ∈ args, isAssignableFrom elem a = true
  | [], st, st', h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq] at h
    exact ⟨[], by simp [h], rfl, by simp⟩
  | _ :: l, st, st', h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨st1, ⟨⟨arg, rest⟩, hp, h1⟩, h2⟩ := h
    rw [popType_ok] at hp
    by_cases ha : isAssignableFrom elem arg = true
    · simp only [ha, Bool.not_true, Bool.false_eq_true, if_false, pure, Except.pure,
        Except.ok.injEq] at h1
      subst h1
      obtain ⟨args, hst, hl, hall⟩ := popElements_ok l h2
      refine ⟨arg :: args, by simp [hp, hst], by simp [hl], fun a ha' => ?_⟩
      rcases List.mem_cons.1 ha' with rfl | ha'
      · exact ha
      · exact hall a ha'
    · simp [ha, reject] at h1

/-! ## Type arguments -/

theorem mapM_ok_length {α β ε : Type} {f : α → Except ε β} :
    ∀ {l : List α} {r : List β}, l.mapM f = .ok r → r.length = l.length
  | [], r, h => by simp [pure, Except.pure] at h; subst h; rfl
  | x :: xs, r, h => by
    simp only [List.mapM_cons, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨b, -, bs, hbs, rfl⟩ := h
    simp [mapM_ok_length hbs]

/-- `CallGeneric`'s argument pops: one argument per parameter, each
assignable to its instantiated type. -/
theorem popInstantiated_ok {tys : List SignatureToken} :
    ∀ {rev : List SignatureToken} {st st' : List SignatureToken},
      rev.foldlM (fun stack p => do
        let (arg, rest) ← popType stack
        if !isAssignableFrom (← instantiate tys p) arg then reject "CALL_TYPE_MISMATCH_ERROR"
        else pure rest) st = .ok st' →
      ∃ args, st = args ++ st' ∧ args.length = rev.length ∧
        AllPairs (fun a p => ∃ q, instantiate tys p = .ok q ∧ isAssignableFrom q a = true)
          args rev
  | [], st, st', h => by
    simp [List.foldlM, pure, Except.pure] at h
    exact ⟨[], by simp [h], rfl, AllPairs.nil⟩
  | p :: ps, st, st', h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨st1, ⟨⟨arg, rest⟩, hp, q, hq, h1⟩, h2⟩ := h
    rw [popType_ok] at hp
    by_cases ha : isAssignableFrom q arg = true
    · simp [ha, pure, Except.pure] at h1
      subst h1
      obtain ⟨args, hst, hl, hf⟩ := popInstantiated_ok h2
      exact ⟨arg :: args, by simp [hp, hst], by simp [hl], AllPairs.cons ⟨q, hq, ha⟩ hf⟩
    · simp [ha, reject] at h1

theorem typeCallGeneric_ok {m : CompiledModule} {st st' : List SignatureToken}
    {fh : FunctionHandle} {tys : List SignatureToken} (h : typeCallGeneric m st fh tys = .ok st') :
    ∃ params rets rets' args rest, m.signatures[fh.parameters]? = some params ∧
      m.signatures[fh.returns]? = some rets ∧ rets.mapM (instantiate tys) = .ok rets' ∧
      st = args ++ rest ∧ args.length = params.length ∧
      AllPairs (fun a p => ∃ q, instantiate tys p = .ok q ∧ isAssignableFrom q a = true)
        args params.reverse ∧
      st' = rets'.reverse ++ rest := by
  unfold typeCallGeneric at h
  simp only [bind_ok, lookup_ok, pure, Except.pure, Except.ok.injEq] at h
  obtain ⟨params, hparams, rest, hpop, rets, hrets, rets', hrets', rfl⟩ := h
  obtain ⟨args, hst, hlen, hall⟩ := popInstantiated_ok hpop
  exact ⟨params, rets, rets', args, rest, hparams, hrets, hrets', hst, by simpa using hlen,
    hall, rfl⟩

theorem fieldsSignatureGeneric_ok {sd : StructDefinition} {variant : Option Nat}
    {tys fs : List SignatureToken} (h : fieldsSignatureGeneric sd variant tys = .ok fs) :
    ∃ fs0, fieldsSignature sd variant = .ok fs0 ∧ fs0.mapM (instantiate tys) = .ok fs := by
  unfold fieldsSignatureGeneric at h
  simp only [bind_ok] at h
  exact h

theorem typePackGeneric_ok {st st' : List SignatureToken} {sd : StructDefinition}
    {variant : Option Nat} {tys : List SignatureToken}
    (h : typePackGeneric st sd variant tys = .ok st') :
    ∃ fs0 fs args rest, fieldsSignature sd variant = .ok fs0 ∧
      fs0.mapM (instantiate tys) = .ok fs ∧ st = args ++ rest ∧
      args.length = fs.length ∧
      AllPairs (fun a p => isAssignableFrom p a = true) args fs.reverse ∧
      st' = materializeType sd.structHandle tys :: rest := by
  unfold typePackGeneric at h
  simp only [bind_ok] at h
  obtain ⟨fs, hfs, rest, hpop, h⟩ := h
  simp only [pure, Except.pure, Except.ok.injEq] at h
  obtain ⟨fs0, h0, h1⟩ := fieldsSignatureGeneric_ok hfs
  obtain ⟨args, hst, hlen, hall⟩ := popExpected_ok hpop
  exact ⟨fs0, fs, args, rest, h0, h1, hst, hlen, hall, h.symm⟩

theorem typeUnpackGeneric_ok {st st' : List SignatureToken} {sd : StructDefinition}
    {variant : Option Nat} {tys : List SignatureToken}
    (h : typeUnpackGeneric st sd variant tys = .ok st') :
    ∃ fs0 fs rest, st = materializeType sd.structHandle tys :: rest ∧
      fieldsSignature sd variant = .ok fs0 ∧ fs0.mapM (instantiate tys) = .ok fs ∧
      st' = fs.reverse ++ rest := by
  unfold typeUnpackGeneric at h
  simp only [bind_ok] at h
  obtain ⟨⟨arg, rest⟩, hp, h⟩ := h
  rw [popType_ok] at hp
  by_cases hc : arg = materializeType sd.structHandle tys
  · subst hc
    simp only [bne_self_eq_false, Bool.false_eq_true, if_false, pure, Except.pure, bind_ok] at h
    obtain ⟨fs, hfs, h⟩ := h
    simp only [Except.ok.injEq] at h
    obtain ⟨fs0, h0, h1⟩ := fieldsSignatureGeneric_ok hfs
    exact ⟨fs0, fs, rest, hp, h0, h1, h.symm⟩
  · have : (arg != materializeType sd.structHandle tys) = true := by simpa using hc
    simp [this, reject, bind, Except.bind] at h

theorem structInstParts_ok {m : CompiledModule} {i : Nat} {d : Nat} {sd : StructDefinition}
    {tys : List SignatureToken} :
    structInstParts m i = .ok (d, sd, tys) ↔
      ∃ si, m.structDefInstantiations[i]? = some si ∧ si.def_ = d ∧
        m.structDefs[d]? = some sd ∧ m.signatures[si.typeParameters]? = some tys := by
  unfold structInstParts
  simp only [bind_ok, lookup_ok, pure, Except.pure, Except.ok.injEq, Prod.mk.injEq]
  constructor
  · rintro ⟨si, hsi, sd', hsd, tys', htys, rfl, rfl, rfl⟩
    exact ⟨si, hsi, rfl, hsd, htys⟩
  · rintro ⟨si, hsi, rfl, hsd, htys⟩
    exact ⟨si, hsi, sd, hsd, tys, htys, rfl, rfl, rfl⟩

theorem variantInstParts_ok {m : CompiledModule} {i : Nat} {vh : StructVariantHandle}
    {sd : StructDefinition} {tys : List SignatureToken} :
    variantInstParts m i = .ok (vh, sd, tys) ↔
      ∃ vi, m.structVariantInstantiations[i]? = some vi ∧
        m.structVariantHandles[vi.handle]? = some vh ∧
        m.structDefs[vh.structIndex]? = some sd ∧ m.signatures[vi.typeParameters]? = some tys := by
  unfold variantInstParts
  simp only [bind_ok, lookup_ok, pure, Except.pure, Except.ok.injEq, Prod.mk.injEq]
  constructor
  · rintro ⟨vi, hvi, vh', hvh, sd', hsd, tys', htys, rfl, rfl, rfl⟩
    exact ⟨vi, hvi, hvh, hsd, htys⟩
  · rintro ⟨vi, hvi, hvh, hsd, htys⟩
    exact ⟨vi, hvi, vh, hvh, sd, hsd, tys, htys, rfl, rfl, rfl⟩

theorem instantiateIn_attach {subst : List SignatureToken} (l : List SignatureToken) :
    l.attach.mapM (fun ⟨x, _⟩ => instantiateIn subst x) = l.mapM (instantiateIn subst) := by
  rw [List.mapM_subtype (g := instantiateIn subst) (fun _ _ => rfl), List.unattach_attach]

mutual
/-- The model's substitution agrees with the specification's. -/
theorem instantiateIn_spec {subst : List SignatureToken} :
    ∀ (t : SignatureToken) {q : SignatureToken}, instantiateIn subst t = .ok q →
      t.instantiate subst = some q
  | .bool, _, h | .u8, _, h | .u16, _, h | .u32, _, h | .u64, _, h | .u128, _, h
  | .u256, _, h | .i8, _, h | .i16, _, h | .i32, _, h | .i64, _, h | .i128, _, h
  | .i256, _, h | .address, _, h | .signer, _, h | .struct _, _, h => by
    simp only [instantiateIn, pure, Except.pure, Except.ok.injEq] at h
    subst h
    rfl
  | .vector t, _, h => by
    simp only [instantiateIn, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨q', h', rfl⟩ := h
    simp [SignatureToken.instantiate, instantiateIn_spec t h']
  | .reference t, _, h => by
    simp only [instantiateIn, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨q', h', rfl⟩ := h
    simp [SignatureToken.instantiate, instantiateIn_spec t h']
  | .mutableReference t, _, h => by
    simp only [instantiateIn, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨q', h', rfl⟩ := h
    simp [SignatureToken.instantiate, instantiateIn_spec t h']
  | .function ps rs a, _, h => by
    simp only [instantiateIn, instantiateIn_attach, bind_ok, pure, Except.pure,
      Except.ok.injEq] at h
    obtain ⟨ps', hps, rs', hrs, rfl⟩ := h
    simp [SignatureToken.instantiate, instantiateList_spec ps hps, instantiateList_spec rs hrs]
  | .structInstantiation hh args, _, h => by
    simp only [instantiateIn, instantiateIn_attach, bind_ok, pure, Except.pure,
      Except.ok.injEq] at h
    obtain ⟨args', hargs, rfl⟩ := h
    simp [SignatureToken.instantiate, instantiateList_spec args hargs]
  | .typeParameter i, _, h => by
    simp only [instantiateIn, lookup_ok] at h
    simpa [SignatureToken.instantiate] using h

theorem instantiateList_spec {subst : List SignatureToken} :
    ∀ (ts : List SignatureToken) {qs : List SignatureToken},
      ts.mapM (instantiateIn subst) = .ok qs → SignatureToken.instantiateList subst ts = some qs
  | [], _, h => by
    simp only [List.mapM_nil, pure, Except.pure, Except.ok.injEq] at h
    subst h
    rfl
  | t :: ts, _, h => by
    simp only [List.mapM_cons, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨q, hq, qs', hqs, rfl⟩ := h
    simp [SignatureToken.instantiateList, instantiateIn_spec t hq, instantiateList_spec ts hqs]
end

theorem fieldCount_of_generic {sd : StructDefinition} {variant : Option Nat}
    {tys fs0 fs : List SignatureToken} (h0 : fieldsSignature sd variant = .ok fs0)
    (h1 : fs0.mapM (instantiate tys) = .ok fs) :
    fieldCount sd.fieldInformation variant = fs.length := by
  rw [fieldCount_of_fieldsSignature h0, mapM_ok_length h1]

/-! ## Closures -/

/-- What instruction consistency and the signature checker establish about a
closure instruction (`check_closure_mask`, `verify_code`): a mask captures
only parameters of its target, and `CallClosure`'s signature is a single
function type. -/
def ClosureOk (m : CompiledModule) : Bytecode → Prop
  | .packClosure h mask => ∀ fh params, m.functionHandles[h]? = some fh →
      m.signatures[fh.parameters]? = some params → ∀ i, mask.getLsbD i = true → i < params.length
  | .packClosureGeneric x mask => ∀ fi fh params, m.functionInstantiations[x]? = some fi →
      m.functionHandles[fi.handle]? = some fh → m.signatures[fh.parameters]? = some params →
      ∀ i, mask.getLsbD i = true → i < params.length
  | .callClosure s => ∃ ps rs a, m.signatures[s]? = some [.function ps rs a]
  | _ => True

theorem filter_zipIdx_length {α : Type} (f : Nat → Bool) : ∀ (l : List α) (k : Nat),
    ((l.zipIdx k).filter (fun p => f p.2)).length = (List.range' k l.length).countP f
  | [], _ => rfl
  | a :: l, k => by
    rw [List.zipIdx_cons, List.length_cons, List.range'_succ, List.countP_cons, List.filter_cons]
    have ih := filter_zipIdx_length f l (k + 1)
    split <;> simp_all

theorem countP_range_stable {p : Nat → Bool} {a : Nat} (h : ∀ i, p i = true → i < a) :
    ∀ b, a ≤ b → (List.range b).countP p = (List.range a).countP p
  | 0, hab => by
    have : a = 0 := by omega
    subst this; rfl
  | b + 1, hab => by
    by_cases hb : a = b + 1
    · subst hb; rfl
    · rw [List.range_succ, List.countP_append, countP_range_stable h b (by omega)]
      have : p b = false := by
        cases hp : p b
        · rfl
        · have := h b hp; omega
      simp [this]

/-- A mask that captures only positions of `l` extracts `capturedCount` of
its elements. -/
theorem maskExtract_length {α : Type} {mask : BitVec 64} {l : List α}
    (h : ∀ i, mask.getLsbD i = true → i < l.length) :
    (maskExtract mask l true).length = capturedCount mask := by
  unfold maskExtract capturedCount
  rw [List.length_map]
  have := filter_zipIdx_length (fun i => mask.getLsbD i == true) l 0
  simp only [beq_true] at this ⊢
  rw [this, ← List.range_eq_range']
  have h64 : ∀ i, mask.getLsbD i = true → i < 64 := by
    intro i hi
    rcases Nat.lt_or_ge i 64 with h' | h'
    · exact h'
    · rw [BitVec.getLsbD_of_ge mask i h'] at hi; cases hi
  have hmin : ∀ i, mask.getLsbD i = true → i < min l.length 64 := fun i hi =>
    Nat.lt_min.2 ⟨h i hi, h64 i hi⟩
  rw [countP_range_stable hmin l.length (Nat.min_le_left _ _),
    countP_range_stable hmin 64 (Nat.min_le_right _ _)]

/-- The captures `clos_pack` pops: one per extracted parameter, top of the
stack first, each assignable to its instantiated parameter type, of no
reference type, and with at least the closure's abilities. -/
theorem closFold_ok {m : CompiledModule} {cs : List AbilitySet} {tys : List SignatureToken} :
    ∀ (ts : List SignatureToken) {ab0 ab : AbilitySet} {st st' : List SignatureToken},
      ts.foldlM (closCapture m cs tys) (ab0, st) = .ok (ab, st') →
      ∃ caps, st = caps ++ st' ∧ caps.length = ts.length ∧ ab.subset ab0 = true ∧
        AllPairs (fun a t => (∃ q, instantiate tys t = .ok q ∧ isAssignableFrom q a = true) ∧
          SignatureV2.isReference t = false ∧
          ∃ aa, abilities m cs a = .ok aa ∧ ab.subset aa = true) caps ts
  | [], ab0, ab, st, st', h => by
    simp only [List.foldlM_nil, pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    exact ⟨[], rfl, rfl, by cases ab0; simp [MoveBytecode.AbilitySet.subset], AllPairs.nil⟩
  | t :: ts, ab0, ab, st, st', h => by
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨⟨ab1, st1⟩, h1, h2⟩ := h
    unfold closCapture at h1
    simp only [bind_ok] at h1
    obtain ⟨⟨arg, rest⟩, hp, aa, haa, q, hq, h1⟩ := h1
    rw [popType_ok] at hp
    subst hp
    split at h1
    · simp [reject, bind, Except.bind] at h1
    · next hassign =>
      split at h1
      · simp [reject, bind, Except.bind] at h1
      · next hnref =>
        simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h1
        obtain ⟨rfl, rfl⟩ := h1
        obtain ⟨caps, hst, hlen, hsub, hall⟩ := closFold_ok ts h2
        refine ⟨arg :: caps, by simp [hst], by simp [hlen], ?_, AllPairs.cons ⟨⟨q, hq, by
          simpa using hassign⟩, by simpa using hnref, aa, haa, ?_⟩ hall⟩
        · cases ab; cases ab0; cases aa
          simp_all [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter]
          grind
        · cases ab; cases ab0; cases aa
          simp_all [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter]
          grind

/-- `clos_pack`'s effect: the captures popped and the closure's type
pushed. -/
theorem typeClosPack_ok {m : CompiledModule} {cs : List AbilitySet} {st st' : List SignatureToken}
    {fh : FunctionHandle} {tys : List SignatureToken} {mask : BitVec 64}
    (h : typeClosPack m cs st fh tys mask = .ok st') :
    ∃ params caps rest ab returns t, m.signatures[fh.parameters]? = some params ∧
      st = caps ++ rest ∧ caps.length = (maskExtract mask params true).length ∧
      ab.subset (closureInitAbilities fh) = true ∧
      AllPairs (fun a t => (∃ q, instantiate tys t = .ok q ∧ isAssignableFrom q a = true) ∧
          SignatureV2.isReference t = false ∧
          ∃ aa, abilities m cs a = .ok aa ∧ ab.subset aa = true) caps
        (maskExtract mask params true).reverse ∧
      m.signatures[fh.returns]? = some returns ∧
      instantiate tys (.function (maskExtract mask params false) returns ab) = .ok t ∧
      st' = t :: rest := by
  unfold typeClosPack at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨params, hparams, ⟨ab, rest⟩, hfold, returns, hrets, t, ht, h⟩ := h
  simp only [pure, Except.pure, Except.ok.injEq] at h
  obtain ⟨caps, hst, hlen, hsub, hall⟩ := closFold_ok _ hfold
  exact ⟨params, caps, rest, ab, returns, t, hparams, hst, by simpa using hlen, hsub, hall, hrets,
    ht, h.symm⟩

/-- `call_closure`'s effect: the closure and the arguments popped, the
results pushed. -/
theorem typeCallClosure_ok {st st' : List SignatureToken} {expected : SignatureToken}
    (h : typeCallClosure st expected = .ok st') :
    ∃ params results a clo args rest, expected = .function params results a ∧
      st = clo :: (args ++ rest) ∧ isAssignableFrom expected clo = true ∧
      args.length = params.length ∧
      AllPairs (fun a p => isAssignableFrom p a = true) args params.reverse ∧
      st' = results.reverse ++ rest := by
  unfold typeCallClosure at h
  split at h
  · next params results a =>
    simp only [bind_ok] at h
    obtain ⟨⟨clo, st1⟩, hp, h⟩ := h
    rw [popType_ok] at hp
    split at h
    · simp [reject, bind, Except.bind] at h
    · next hassign =>
      simp only [bind_ok, pure, Except.pure, Except.ok.injEq] at h
      obtain ⟨rest, hpop, rfl⟩ := h
      obtain ⟨args, hargs, hlen, hall⟩ := popExpected_ok hpop
      exact ⟨params, results, a, clo, args, rest, rfl, by rw [hp, hargs], by simpa using hassign,
        hlen, hall, rfl⟩
  · simp [panicAt] at h

set_option maxHeartbeats 1600000 in
/-- A type-checked instruction changes the abstract stack's length by
exactly its stack effect. -/
theorem typeInstr_effect {m : CompiledModule} {view : FunctionView} {st st' : List SignatureToken}
    {i : Bytecode} (h : typeInstr m view st i = .ok st') (hcl : ClosureOk m i) :
    ∃ pops pushes, instructionEffect m view.returns i = .ok (pops, pushes) ∧ pops ≤ st.length ∧
      st'.length = st.length - pops + pushes := by
  cases i
  -- References, vectors through references and global storage.
  case freezeRef | mutBorrowField | immBorrowField | mutBorrowVariantField
    | immBorrowVariantField | mutBorrowLoc | immBorrowLoc | testVariant | readRef | writeRef
    | mutBorrowGlobal | immBorrowGlobal | exists_ | moveFrom | moveTo | vecLen | vecImmBorrow
    | vecMutBorrow | vecPushBack | vecPopBack | vecSwap
    | mutBorrowFieldGeneric | immBorrowFieldGeneric | mutBorrowVariantFieldGeneric
    | immBorrowVariantFieldGeneric | testVariantGeneric | mutBorrowGlobalGeneric
    | immBorrowGlobalGeneric | existsGeneric | moveFromGeneric | moveToGeneric =>
    simp only [typeInstr, instructionEffect, pure, Except.pure, borrowField, borrowGlobal,
      borrowVectorElement, rejectIf, fieldHandleParts, borrowFieldGeneric,
      borrowGlobalGeneric] at h ⊢
    refine ⟨_, _, rfl, ?_⟩
    peel_ok h
    all_goals simp
  all_goals simp only [typeInstr, instructionEffect, Model.castTo, typeCall, bind_ok, pure,
      Except.pure, lookup_ok] at h ⊢
  case call hdl =>
    obtain ⟨fh, hfh, params, hparams, st1, hpop, rets, hrets, hst⟩ := h
    obtain ⟨args, hargs, hlen, _⟩ := popExpected_ok hpop
    simp only [Except.ok.injEq] at hst
    refine ⟨params.length, rets.length, ⟨fh, hfh, params, hparams, rets, hrets, rfl⟩, ?_, ?_⟩
    · rw [hargs]; simp; omega
    · rw [← hst, hargs]; simp; omega
  case ret =>
    obtain ⟨args, hargs, hlen, _⟩ := popExpected_ok h
    refine ⟨_, _, rfl, ?_, ?_⟩
    · rw [hargs]; simp; omega
    · rw [hargs]; simp; omega
  case pack d =>
    obtain ⟨sd, hsd, h⟩ := h
    obtain ⟨fs, args, rest, hfs, rfl, hlen, -, rfl⟩ := typePack_ok h
    refine ⟨_, _, ⟨sd, hsd, rfl⟩, ?_, ?_⟩ <;>
      simp [fieldCount_of_fieldsSignature hfs, ← hlen] <;> omega
  case unpack d =>
    obtain ⟨sd, hsd, h⟩ := h
    obtain ⟨fs, rest, rfl, hfs, rfl⟩ := typeUnpack_ok h
    refine ⟨_, _, ⟨sd, hsd, rfl⟩, ?_, ?_⟩ <;>
      simp [fieldCount_of_fieldsSignature hfs] <;> omega
  case packVariant hdl =>
    obtain ⟨vh, hvh, sd, hsd, h⟩ := h
    obtain ⟨fs, args, rest, hfs, rfl, hlen, -, rfl⟩ := typePack_ok h
    refine ⟨_, _, ⟨vh, hvh, sd, hsd, rfl⟩, ?_, ?_⟩ <;>
      simp [fieldCount_of_fieldsSignature hfs, ← hlen] <;> omega
  case unpackVariant hdl =>
    obtain ⟨vh, hvh, sd, hsd, h⟩ := h
    obtain ⟨fs, rest, rfl, hfs, rfl⟩ := typeUnpack_ok h
    refine ⟨_, _, ⟨vh, hvh, sd, hsd, rfl⟩, ?_, ?_⟩ <;>
      simp [fieldCount_of_fieldsSignature hfs] <;> omega
  case vecPack idx n =>
    obtain ⟨elem, -, st1, hpop, h⟩ := h
    simp only [Except.ok.injEq] at h
    subst h
    obtain ⟨args, rfl, hlen, -⟩ := popElements_ok _ hpop
    refine ⟨_, _, rfl, ?_, ?_⟩ <;> simp [hlen] <;> omega
  case vecUnpack idx n =>
    obtain ⟨⟨op, rest⟩, hp, elem, -, h⟩ := h
    rw [popType_ok] at hp
    subst hp
    split at h
    · simp [reject] at h
    · simp only [Except.ok.injEq] at h
      subst h
      refine ⟨_, _, rfl, ?_, ?_⟩ <;> simp <;> omega
  case abortMsg =>
    obtain ⟨⟨msg, rest⟩, hp, ⟨code, rest'⟩, hp', h⟩ := h
    rw [popType_ok] at hp hp'
    subst hp hp'
    split at h
    · simp [reject] at h
    · simp only [Except.ok.injEq] at h
      subst h
      refine ⟨_, _, rfl, ?_, ?_⟩ <;> simp
  case callGeneric idx =>
    obtain ⟨fi, hfi, fh, hfh, tys, htys, h⟩ := h
    obtain ⟨params, rets, rets', args, rest, hparams, hrets, hrets', rfl, hlen, -, rfl⟩ :=
      typeCallGeneric_ok h
    refine ⟨params.length, rets.length, ⟨fi, hfi, fh, hfh, params, hparams, rets, hrets, rfl⟩,
      ?_, ?_⟩
    · simp; omega
    · simp [mapM_ok_length hrets']; omega
  case packGeneric idx =>
    obtain ⟨⟨d, sd, tys⟩, hparts, h⟩ := h
    obtain ⟨si, hsi, rfl, hsd, -⟩ := structInstParts_ok.1 hparts
    obtain ⟨fs0, fs, args, rest, h0, h1, rfl, hlen, -, rfl⟩ := typePackGeneric_ok h
    refine ⟨_, _, ⟨si, hsi, sd, hsd, rfl⟩, ?_, ?_⟩ <;>
      simp [fieldCount_of_generic h0 h1, ← hlen] <;> omega
  case unpackGeneric idx =>
    obtain ⟨⟨d, sd, tys⟩, hparts, h⟩ := h
    obtain ⟨si, hsi, rfl, hsd, -⟩ := structInstParts_ok.1 hparts
    obtain ⟨fs0, fs, rest, rfl, h0, h1, rfl⟩ := typeUnpackGeneric_ok h
    refine ⟨_, _, ⟨si, hsi, sd, hsd, rfl⟩, ?_, ?_⟩ <;>
      simp [fieldCount_of_generic h0 h1] <;> omega
  case packVariantGeneric idx =>
    obtain ⟨⟨vh, sd, tys⟩, hparts, h⟩ := h
    obtain ⟨vi, hvi, hvh, hsd, -⟩ := variantInstParts_ok.1 hparts
    obtain ⟨fs0, fs, args, rest, h0, h1, rfl, hlen, -, rfl⟩ := typePackGeneric_ok h
    refine ⟨_, _, ⟨vi, hvi, vh, hvh, sd, hsd, rfl⟩, ?_, ?_⟩ <;>
      simp [fieldCount_of_generic h0 h1, ← hlen] <;> omega
  case unpackVariantGeneric idx =>
    obtain ⟨⟨vh, sd, tys⟩, hparts, h⟩ := h
    obtain ⟨vi, hvi, hvh, hsd, -⟩ := variantInstParts_ok.1 hparts
    obtain ⟨fs0, fs, rest, rfl, h0, h1, rfl⟩ := typeUnpackGeneric_ok h
    refine ⟨_, _, ⟨vi, hvi, vh, hvh, sd, hsd, rfl⟩, ?_, ?_⟩ <;>
      simp [fieldCount_of_generic h0 h1] <;> omega
  case packClosure hh mask =>
    obtain ⟨fh, hfh, h⟩ := h
    obtain ⟨params, caps, rest, ab, returns, t, hparams, rfl, hlen, -, -, -, -, rfl⟩ :=
      typeClosPack_ok h
    have hcount := maskExtract_length (hcl fh params hfh hparams)
    refine ⟨_, _, rfl, ?_, ?_⟩ <;> simp [hlen, hcount] <;> omega
  case packClosureGeneric x mask =>
    obtain ⟨fi, hfi, tys, htys, fh, hfh, h⟩ := h
    obtain ⟨params, caps, rest, ab, returns, t, hparams, rfl, hlen, -, -, -, -, rfl⟩ :=
      typeClosPack_ok h
    have hcount := maskExtract_length (hcl fi fh params hfi hfh hparams)
    refine ⟨_, _, rfl, ?_, ?_⟩ <;> simp [hlen, hcount] <;> omega
  case callClosure s =>
    obtain ⟨sig, hsig, h⟩ := h
    cases hhd : sig.head? with
    | none => rw [hhd] at h; simp [panicAt] at h
    | some t =>
      rw [hhd] at h
      obtain ⟨params, results, a, clo, args, rest, rfl, rfl, -, hlen, -, rfl⟩ :=
        typeCallClosure_ok h
      refine ⟨1 + params.length, results.length, ⟨sig, hsig, by simp [hhd]⟩, ?_, ?_⟩ <;>
        simp [hlen] <;> omega
  all_goals (try (simp at h; done))
  all_goals refine ⟨_, _, rfl, ?_, ?_⟩
  all_goals (try simp only [popType_ok', popTwo_ok'] at h)
  all_goals first
    | (cases h; simp; done)
    | (cases h; simp; omega)
    | grind [reject]

/-- For an instruction type safety accepts, the model's stack effect is the
specification's arity. -/
theorem effect_agrees_typed {m : CompiledModule} {view : FunctionView} {fh : FunctionHandle}
    (hret : m.signatures[fh.returns]? = some view.returns) {st st' : List SignatureToken}
    {i : Bytecode} (ht : typeInstr m view st i = .ok st') (hcl : ClosureOk m i) {e : Nat × Nat}
    (h : instructionEffect m view.returns i = .ok e) : i.arity m fh = some e := by
  cases i <;> simp only [instructionEffect, Bytecode.arity, pure, Except.pure, bind_ok,
    lookup_ok, Except.ok.injEq] at h ⊢
  case call hdl =>
    obtain ⟨fh', h1, ps, h2, rs, h3, h4⟩ := h
    subst h4
    simp [CompiledModule.handleArity, h1, h2, h3]
  case ret => subst h; simp [hret]
  case callClosure s =>
    obtain ⟨ps, rs, a, hs⟩ := hcl
    obtain ⟨sig, hsig, h⟩ := h
    rw [hs, Option.some.injEq] at hsig
    subst hsig
    simp only [List.head?_cons, Except.ok.injEq] at h
    subst h
    simp [CompiledModule.closureArity, hs, Nat.add_comm]
  case pack d =>
    obtain ⟨sd, hsd, rfl⟩ := h
    simp only [typeInstr, bind_ok, lookup_ok] at ht
    obtain ⟨sd', hsd', ht⟩ := ht
    rw [hsd] at hsd'; cases hsd'
    obtain ⟨fs, -, -, hfs, -⟩ := typePack_ok ht
    rcases fieldsSignature_ok hfs with ⟨-, fields, hf, rfl⟩ | ⟨v, -, -, hv, -⟩
    · simp [CompiledModule.declaredFieldCount, hsd, hf, fieldCount]
    · cases hv
  case unpack d =>
    obtain ⟨sd, hsd, rfl⟩ := h
    simp only [typeInstr, bind_ok, lookup_ok] at ht
    obtain ⟨sd', hsd', ht⟩ := ht
    rw [hsd] at hsd'; cases hsd'
    obtain ⟨fs, -, -, hfs, -⟩ := typeUnpack_ok ht
    rcases fieldsSignature_ok hfs with ⟨-, fields, hf, rfl⟩ | ⟨v, -, -, hv, -⟩
    · simp [CompiledModule.declaredFieldCount, hsd, hf, fieldCount]
    · cases hv
  case packVariant hdl =>
    obtain ⟨vh, hvh, sd, hsd, rfl⟩ := h
    simp only [typeInstr, bind_ok, lookup_ok] at ht
    obtain ⟨vh', hvh', sd', hsd', ht⟩ := ht
    rw [hvh] at hvh'; cases hvh'
    rw [hsd] at hsd'; cases hsd'
    obtain ⟨fs, -, -, hfs, -⟩ := typePack_ok ht
    rcases fieldsSignature_ok hfs with ⟨hv, -⟩ | ⟨v, variants, vd, hv, hf, hvd, rfl⟩
    · cases hv
    · cases hv
      simp [CompiledModule.variantFieldCount, CompiledModule.variants, hvh, hsd, hf, hvd,
        fieldCount]
  case unpackVariant hdl =>
    obtain ⟨vh, hvh, sd, hsd, rfl⟩ := h
    simp only [typeInstr, bind_ok, lookup_ok] at ht
    obtain ⟨vh', hvh', sd', hsd', ht⟩ := ht
    rw [hvh] at hvh'; cases hvh'
    rw [hsd] at hsd'; cases hsd'
    obtain ⟨fs, -, -, hfs, -⟩ := typeUnpack_ok ht
    rcases fieldsSignature_ok hfs with ⟨hv, -⟩ | ⟨v, variants, vd, hv, hf, hvd, rfl⟩
    · cases hv
    · cases hv
      simp [CompiledModule.variantFieldCount, CompiledModule.variants, hvh, hsd, hf, hvd,
        fieldCount]
  case callGeneric idx =>
    obtain ⟨fi, h1, fh', h2, ps, h3, rs, h4, h5⟩ := h
    subst h5
    simp [CompiledModule.instantiationArity, CompiledModule.handleArity, h1, h2, h3, h4]
  case packGeneric idx =>
    obtain ⟨si, hsi, sd, hsd, rfl⟩ := h
    simp only [typeInstr, bind_ok] at ht
    obtain ⟨⟨d, sd', tys⟩, hparts, ht⟩ := ht
    obtain ⟨si', hsi', rfl, hsd', -⟩ := structInstParts_ok.1 hparts
    rw [hsi] at hsi'; cases hsi'
    rw [hsd] at hsd'; cases hsd'
    dsimp only at ht
    obtain ⟨fs0, -, -, -, hfs, -⟩ := typePackGeneric_ok ht
    rcases fieldsSignature_ok hfs with ⟨-, fields, hf, rfl⟩ | ⟨v, -, -, hv, -⟩
    · simp [CompiledModule.structInstFieldCount, CompiledModule.declaredFieldCount, hsi, hsd, hf,
        fieldCount]
    · cases hv
  case unpackGeneric idx =>
    obtain ⟨si, hsi, sd, hsd, rfl⟩ := h
    simp only [typeInstr, bind_ok] at ht
    obtain ⟨⟨d, sd', tys⟩, hparts, ht⟩ := ht
    obtain ⟨si', hsi', rfl, hsd', -⟩ := structInstParts_ok.1 hparts
    rw [hsi] at hsi'; cases hsi'
    rw [hsd] at hsd'; cases hsd'
    dsimp only at ht
    obtain ⟨fs0, -, -, -, hfs, -⟩ := typeUnpackGeneric_ok ht
    rcases fieldsSignature_ok hfs with ⟨-, fields, hf, rfl⟩ | ⟨v, -, -, hv, -⟩
    · simp [CompiledModule.structInstFieldCount, CompiledModule.declaredFieldCount, hsi, hsd, hf,
        fieldCount]
    · cases hv
  case packVariantGeneric idx =>
    obtain ⟨vi, hvi, vh, hvh, sd, hsd, rfl⟩ := h
    simp only [typeInstr, bind_ok] at ht
    obtain ⟨⟨vh', sd', tys⟩, hparts, ht⟩ := ht
    obtain ⟨vi', hvi', hvh', hsd', -⟩ := variantInstParts_ok.1 hparts
    rw [hvi] at hvi'; cases hvi'
    rw [hvh] at hvh'; cases hvh'
    rw [hsd] at hsd'; cases hsd'
    dsimp only at ht
    obtain ⟨fs0, -, -, -, hfs, -⟩ := typePackGeneric_ok ht
    rcases fieldsSignature_ok hfs with ⟨hv, -⟩ | ⟨v, variants, vd, hv, hf, hvd, rfl⟩
    · cases hv
    · cases hv
      simp [CompiledModule.variantInstFieldCount, CompiledModule.variantFieldCount,
        CompiledModule.variants, hvi, hvh, hsd, hf, hvd, fieldCount]
  case unpackVariantGeneric idx =>
    obtain ⟨vi, hvi, vh, hvh, sd, hsd, rfl⟩ := h
    simp only [typeInstr, bind_ok] at ht
    obtain ⟨⟨vh', sd', tys⟩, hparts, ht⟩ := ht
    obtain ⟨vi', hvi', hvh', hsd', -⟩ := variantInstParts_ok.1 hparts
    rw [hvi] at hvi'; cases hvi'
    rw [hvh] at hvh'; cases hvh'
    rw [hsd] at hsd'; cases hsd'
    dsimp only at ht
    obtain ⟨fs0, -, -, -, hfs, -⟩ := typeUnpackGeneric_ok ht
    rcases fieldsSignature_ok hfs with ⟨hv, -⟩ | ⟨v, variants, vd, hv, hf, hvd, rfl⟩
    · cases hv
    · cases hv
      simp [CompiledModule.variantInstFieldCount, CompiledModule.variantFieldCount,
        CompiledModule.variants, hvi, hvh, hsd, hf, hvd, fieldCount]
  all_goals first
    | (subst h; rfl)
    | (simp [typeInstr, panicAt] at ht; done)
    | (simp [typeInstr, bind_ok, lookup_ok, panicAt] at ht; done)
    | skip

/-! ## The certificate -/

variable (m : CompiledModule) (view : FunctionView)

/-- The certified operand-stack types before `pc`: empty at a block id,
otherwise those after the previous instruction. -/
def typesBefore : Nat → Option (List SignatureToken)
  | 0 => some []
  | pc + 1 =>
    if pc + 1 ∈ blockIds view.code.code then some []
    else (typesBefore pc).bind (fun st => (typeStep m view st pc).toOption)

theorem typesBefore_of_mem {pc : Nat} (h : pc ∈ blockIds view.code.code) :
    typesBefore m view pc = some [] := by
  cases pc with
  | zero => rfl
  | succ p => simp [typesBefore, h]

theorem typesBefore_succ {pc : Nat} {st st' : List SignatureToken}
    (hnot : pc + 1 ∉ blockIds view.code.code) (h : typesBefore m view pc = some st)
    (hs : typeStep m view st pc = .ok st') : typesBefore m view (pc + 1) = some st' := by
  simp [typesBefore, hnot, h, hs, Except.toOption]

variable {m view}

theorem typeStep_ok {st st' : List SignatureToken} {pc : Nat}
    (h : typeStep m view st pc = .ok st') :
    ∃ i, view.code.code[pc]? = some i ∧ typeInstr m view st i = .ok st' := by
  unfold typeStep at h
  simp only [bind_ok, lookup_ok] at h
  exact h

/-- Along a block whose stack usage is verified, the type fold from the empty
stack follows the certificate and has the stack-usage heights as lengths. -/
theorem typeBlock_prefix {fh : FunctionHandle}
    (hagree : EffectsAgree m fh view.returns view.code.code)
    (hcl : ∀ i ∈ view.code.code.toList, ClosureOk m i) {b : BasicBlock}
    (hb : BlockOk view.code.code b) :
    ∀ j st, j ≤ b.exit + 1 - b.start →
      (List.range' b.start j).foldlM (typeStep m view) [] = .ok st →
      (j = 0 → st = []) ∧
      (0 < j → st.length = heightAfter m fh view.code.code (b.start + j - 1)) ∧
      (b.start + j ≤ b.exit → typesBefore m view (b.start + j) = some st) := by
  intro j
  induction j with
  | zero =>
    intro st _ h
    simp [List.range', pure, Except.pure] at h
    subst h
    exact ⟨fun _ => rfl, by omega, fun _ => typesBefore_of_mem m view hb.start_mem⟩
  | succ j ih =>
    intro st hj h
    obtain ⟨st0, h0, h1⟩ := foldlM_last h
    obtain ⟨hz0, hlen0, hcert0⟩ := ih st0 (by omega) h0
    obtain ⟨i, hi, hti⟩ := typeStep_ok h1
    have hmem : i ∈ view.code.code.toList := by
      rw [Array.mem_toList_iff]; exact Array.mem_of_getElem? hi
    obtain ⟨pops, pushes, he, hpops, hlen⟩ := typeInstr_effect hti (hcl i hmem)
    have harity := hagree i hmem _ he
    have heff : eff m fh view.code.code (b.start + j) = (pops, pushes) := by
      simp [eff, hi, harity]
    -- The length before this instruction is the height before it.
    have hbefore : st0.length = heightBefore m fh view.code.code (b.start + j) := by
      rw [heightBefore_in_block hb (by omega) (by omega)]
      cases j with
      | zero => simp [hz0 rfl]
      | succ j' =>
        simp [hlen0 (by omega), show b.start + (j' + 1) - 1 = b.start + j' by omega]
    refine ⟨by omega, fun _ => ?_, fun hle => ?_⟩
    · rw [show b.start + (j + 1) - 1 = b.start + j by omega, hlen, heightAfter, heff, hbefore]
    · rw [show b.start + (j + 1) = b.start + j + 1 by omega]
      exact typesBefore_succ m view (hb.inner _ (by omega) (by omega)) (hcert0 (by omega)) h1

/-- A verified block, from the empty stack, ends with the empty stack. -/
theorem typeBlock_empty {config : VerifierConfig} {fh : FunctionHandle}
    (hagree : EffectsAgree m fh view.returns view.code.code)
    (hcl : ∀ i ∈ view.code.code.toList, ClosureOk m i) {b : BasicBlock}
    (hb : BlockOk view.code.code b)
    (hv : verifyBlock config m view.returns view.code.code b = .ok ()) {r : List SignatureToken}
    (h : typeBlock m view [] b = .ok r) : r = [] := by
  unfold typeBlock at h
  have hle := hb.start_le_exit
  obtain ⟨_, hlen, _⟩ := typeBlock_prefix hagree hcl hb _ r (Nat.le_refl _) h
  have hz := block_exit hagree hb hv
  have := hlen (by omega)
  rw [show b.start + (b.exit + 1 - b.start) - 1 = b.exit by omega, hz] at this
  exact List.eq_nil_of_length_eq_zero this

/-- The threaded fold over the blocks starts every block from the empty stack. -/
theorem typeBlocks_from_empty {config : VerifierConfig} {fh : FunctionHandle}
    (hagree : EffectsAgree m fh view.returns view.code.code)
    (hcl : ∀ i ∈ view.code.code.toList, ClosureOk m i) :
    ∀ (blocks : List BasicBlock) {r : List SignatureToken},
      (∀ b ∈ blocks, BlockOk view.code.code b) →
      (∀ b ∈ blocks, verifyBlock config m view.returns view.code.code b = .ok ()) →
      blocks.foldlM (typeBlock m view) [] = .ok r →
      ∀ b ∈ blocks, ∃ r', typeBlock m view [] b = .ok r' := by
  intro blocks
  induction blocks with
  | nil => simp
  | cons b bs ih =>
    intro r hok hv h
    simp only [List.foldlM_cons, bind_ok] at h
    obtain ⟨r0, h0, hrest⟩ := h
    have hr0 := typeBlock_empty hagree hcl (hok b (by simp)) (hv b (by simp)) h0
    subst hr0
    intro b' hb'
    rcases List.mem_cons.1 hb' with rfl | hb'
    · exact ⟨_, h0⟩
    · exact ih (fun x hx => hok x (by simp [hx])) (fun x hx => hv x (by simp [hx])) hrest b' hb'

/-- **The type certificate.** For a function of the core subset whose stack
usage and type safety are verified, every instruction type-checks from its
certified stack; the result is certified at the next instruction of the
block, and is empty when the instruction ends its block. -/
theorem typeCertificate {config : VerifierConfig} {fh : FunctionHandle}
    (hagree : EffectsAgree m fh view.returns view.code.code)
    (hcl : ∀ i ∈ view.code.code.toList, ClosureOk m i)
    (hblocks : view.cfg.blocks = buildBlocks view.code.code)
    (hsu : verifyStackUsage config m view = .ok ())
    (hts : verifyTypeSafety m view = .ok ()) :
    ∀ pc, pc < view.code.code.size → ∃ st st',
      typesBefore m view pc = some st ∧ typeStep m view st pc = .ok st' ∧
      (pc + 1 < view.code.code.size → pc + 1 ∉ blockIds view.code.code →
        typesBefore m view (pc + 1) = some st') ∧
      (pc + 1 = view.code.code.size ∨ pc + 1 ∈ blockIds view.code.code → st' = []) := by
  intro pc hpc
  obtain ⟨b, hbm, hs, he⟩ := (buildBlocks_spec view.code.code).2 pc hpc
  have hb := (buildBlocks_spec view.code.code).1 b hbm
  have hverify := forM_ok hsu
  rw [hblocks] at hverify
  have hv := hverify b hbm
  -- The block's fold from the empty stack succeeds.
  unfold verifyTypeSafety at hts
  simp only [bind_ok] at hts
  obtain ⟨r, hr, _⟩ := hts
  rw [hblocks] at hr
  obtain ⟨r', hb'⟩ := typeBlocks_from_empty hagree hcl (buildBlocks view.code.code)
    (buildBlocks_spec view.code.code).1 hverify hr b hbm
  have hempty := typeBlock_empty hagree hcl hb hv hb'
  unfold typeBlock at hb'
  -- Split the fold at `pc`.
  have hsplit : b.exit + 1 - b.start = (pc - b.start + 1) + (b.exit - pc) := by omega
  rw [hsplit] at hb'
  obtain ⟨st1, h1⟩ := foldlM_prefix hb'
  obtain ⟨st0, h0, hstep⟩ := foldlM_last h1
  rw [show b.start + (pc - b.start) = pc by omega] at hstep
  obtain ⟨_, _, hcert0⟩ := typeBlock_prefix hagree hcl hb (pc - b.start) st0 (by omega) h0
  rw [show b.start + (pc - b.start) = pc by omega] at hcert0
  refine ⟨st0, st1, hcert0 he, hstep, fun hlt hnot => ?_, fun hend => ?_⟩
  · exact typesBefore_succ m view hnot (hcert0 he) hstep
  · -- `pc` ends its block, so `st1` is the block's result.
    have hexit := ends_at_exit hb hs he hend
    have : pc - b.start + 1 + (b.exit - pc) = pc - b.start + 1 := by omega
    rw [this] at hb'
    rw [hb'] at h1
    cases h1
    exact hempty

/-! ## L4 -/

/-- Type safety type-checks every instruction of the code: it folds over
every block, and every instruction lies in a block. -/
theorem typed_of_typeSafety (hts : verifyTypeSafety m view = .ok ())
    (hblocks : view.cfg.blocks = buildBlocks view.code.code) :
    ∀ i ∈ view.code.code.toList, ∃ st st', typeInstr m view st i = .ok st' := by
  intro i hi
  obtain ⟨pc, hpc⟩ := List.mem_iff_getElem?.1 hi
  rw [Array.getElem?_toList] at hpc
  obtain ⟨b, hb, h1, h2⟩ := (buildBlocks_spec view.code.code).2 pc (lt_of_getElem? hpc)
  unfold verifyTypeSafety at hts
  simp only [bind_ok] at hts
  obtain ⟨_, hfold, -⟩ := hts
  rw [hblocks] at hfold
  obtain ⟨acc, r, hblock⟩ := foldlM_ok_mem hfold b hb
  unfold typeBlock at hblock
  have hmem : pc ∈ List.range' b.start (b.exit + 1 - b.start) := by
    rw [List.mem_range'_1]; omega
  obtain ⟨st, st', hstep⟩ := foldlM_ok_mem hblock pc hmem
  obtain ⟨i', hi', hty⟩ := typeStep_ok hstep
  rw [hpc, Option.some.injEq] at hi'
  subst hi'
  exact ⟨st, st', hty⟩

/-- The stack effects of a type-checked code unit are the specification's
arities. -/
theorem effectsAgree_of_typeSafety {fh : FunctionHandle}
    (hret : m.signatures[fh.returns]? = some view.returns)
    (hcl : ∀ i ∈ view.code.code.toList, ClosureOk m i)
    (hts : verifyTypeSafety m view = .ok ())
    (hblocks : view.cfg.blocks = buildBlocks view.code.code) :
    EffectsAgree m fh view.returns view.code.code := by
  intro i hi e he
  obtain ⟨st, st', ht⟩ := typed_of_typeSafety hts hblocks i hi
  exact effect_agrees_typed hret ht (hcl i hi) he

/-- **L4** for a function the model accepts. -/
theorem stackDiscipline_of_verifyFunction {config : VerifierConfig} {index : Nat}
    {fdef : FunctionDefinition} {n : Nat} {c : CodeUnit} (hcode : fdef.code = some c)
    (hcl : ∀ i ∈ c.code.toList, ClosureOk m i)
    (h : verifyFunction config m index fdef = .ok n) :
    ∃ fh, m.functionHandles[fdef.function]? = some fh ∧
      StackDiscipline m fh c.code config.maxValueStackSize := by
  unfold verifyFunction at h
  rw [hcode] at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨handle, hh, cfg, hcf, view, hview, h⟩ := h
  obtain ⟨hvc, hvcfg, hret⟩ := functionView_ok hview
  refine ⟨handle, hh, ?_⟩
  have hparts : verifyStackUsage config m view = .ok () ∧ verifyTypeSafety m view = .ok () := by
    repeat' split at h
    all_goals first
      | (simp [reject, bind, Except.bind] at h; done)
      | (simp only [bind_ok] at h
         obtain ⟨u, hsu, u', hts, _⟩ := h
         cases u; cases u'
         exact ⟨hsu, hts⟩)
  have hblocks : view.cfg.blocks = buildBlocks view.code.code := by
    rw [hvcfg, verifyControlFlow_blocks hcf, hvc]
  have hcl' : ∀ i ∈ view.code.code.toList, ClosureOk m i := by rw [hvc]; exact hcl
  have hagree := effectsAgree_of_typeSafety hret hcl' hparts.2 hblocks
  have hsu := forM_ok hparts.1
  rw [hblocks] at hsu
  rw [hvc] at hagree hsu
  exact stackDiscipline_of_blocks hagree hsu

end MoveBytecode.Proofs
