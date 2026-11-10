-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefTyped

/-!
# Each instruction keeps the analysis's states well typed

UNTRUSTED (checked by the kernel). `typed_exec`: an instruction the type
checker and the reference-safety analysis both accept takes a well-typed
abstract state (`Typed`) to a well-typed one, for the type-checker's operand
types after it and the availability after it.

Milestone M1 of the D1-with-references plan: no function signature mentions a
reference type (`M1Instr` on calls), and no `BorrowGlobal`. `Ret` ends the
frame and is handled by the block lemma.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

/-! ## Reference kinds of the type checker's tokens -/

theorem refKind_of_ofToken {m : CompiledModule} {t : SignatureToken} {u : RuntimeType}
    (h : RuntimeType.ofToken m t = some u) : refKind t = none := by
  cases t <;> simp_all [refKind, RuntimeType.ofToken, RuntimeType.ofTokenWith]

theorem refKind_of_ofTokenWith {m : CompiledModule} {τ : List RuntimeType} {t : SignatureToken}
    {u : RuntimeType} (h : RuntimeType.ofTokenWith m τ t = some u) : refKind t = none := by
  cases t <;> simp_all [refKind, RuntimeType.ofTokenWith]

theorem refKind_inner {m : CompiledModule} {τ : List RuntimeType} {t inner : SignatureToken}
    {u : RuntimeType} (h : RuntimeType.ofSlotTokenWith m τ t = some u)
    (ht : t = .reference inner ∨ t = .mutableReference inner) : refKind inner = none := by
  rcases ht with rfl | rfl <;>
    simp only [RuntimeType.ofSlotTokenWith, Option.map_eq_some_iff] at h <;>
    obtain ⟨v, hv, -⟩ := h <;> exact refKind_of_ofTokenWith hv

theorem refKind_of_isInteger {t : SignatureToken} (h : isInteger t = true) : refKind t = none := by
  cases t <;> simp_all [isInteger, refKind]

theorem refKind_of_isUnsigned {t : SignatureToken} (h : isUnsignedInteger t = true) :
    refKind t = none := by
  cases t <;> simp_all [isUnsignedInteger, refKind]

theorem refKind_of_isSigned {t : SignatureToken} (h : isSignedInteger t = true) :
    refKind t = none := by
  cases t <;> simp_all [isSignedInteger, refKind]

theorem refKind_refTo (mut_ : Bool) (t : SignatureToken) : refKind (refTo mut_ t) = some mut_ := by
  cases mut_ <;> rfl

theorem castTo_ok {stack st' : List SignatureToken} {target : SignatureToken}
    (h : Model.castTo stack target = .ok st') :
    ∃ op rest, stack = op :: rest ∧ isInteger op = true ∧ st' = target :: rest := by
  simp only [Model.castTo, bind_ok] at h
  obtain ⟨⟨op, rest⟩, hp, h⟩ := h
  rw [popType_ok] at hp
  split at h
  · simp [reject] at h
  · next hc =>
    simp only [pure, Except.pure, Except.ok.injEq] at h
    exact ⟨op, rest, hp, by simpa using hc, h.symm⟩

/-- An instantiated function type is no reference. -/
theorem refKind_instantiate_function {tys ps rs : List SignatureToken} {ab : AbilitySet}
    {t : SignatureToken} (h : instantiate tys (.function ps rs ab) = .ok t) : refKind t = none := by
  unfold instantiate at h
  split at h
  · simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    rfl
  · simp only [instantiateIn, bind_ok, pure, Except.pure, Except.ok.injEq] at h
    obtain ⟨_, -, _, -, rfl⟩ := h
    rfl

/-- The captured parameters' count, as the reference analysis counts them. -/
theorem maskExtract_length_countP {α : Type} (mask : BitVec 64) (l : List α) :
    (maskExtract mask l true).length = (List.range l.length).countP (fun i => mask.getLsbD i) := by
  unfold maskExtract
  rw [List.length_map]
  have := filter_zipIdx_length (fun i => mask.getLsbD i == true) l 0
  simp only [beq_true] at this ⊢
  rw [this, ← List.range_eq_range']

/-- The M1 restriction on an instruction: no global borrows, a call's callee
takes and returns values only and is no struct-API borrow, and no closure
call, which `typed_exec_M2` handles with the calls. -/
def M1Instr (m : CompiledModule) : Bytecode → Prop
  | .mutBorrowGlobal _ | .immBorrowGlobal _ | .ret => False
  -- The generic forms: Phase 4.
  | .callGeneric _ | .packGeneric _ | .packVariantGeneric _ | .unpackGeneric _
  | .unpackVariantGeneric _ | .testVariantGeneric _ | .mutBorrowFieldGeneric _
  | .mutBorrowVariantFieldGeneric _ | .immBorrowFieldGeneric _ | .immBorrowVariantFieldGeneric _
  | .mutBorrowGlobalGeneric _ | .immBorrowGlobalGeneric _ | .existsGeneric _ | .moveFromGeneric _
  | .moveToGeneric _ => False
  | .call h => ∀ fh, m.functionHandles[h]? = some fh →
      borrowFieldMutable fh.attributes = none ∧
      (∀ ps, m.signatures[fh.parameters]? = some ps → ∀ t ∈ ps, refKind t = none) ∧
      (∀ rs, m.signatures[fh.returns]? = some rs → ∀ t ∈ rs, refKind t = none)
  -- A closure call, like a call, is handled with the calls (`typed_exec_M2`).
  | .callClosure _ => False
  | _ => True

namespace Typed

variable {ltys stys : List SignatureToken} {avail : LocalIndex → Availability} {st : RefState}

/-- A step leaving the analysis's state alone retypes the top operand. -/
theorem retype' (ht : Typed ltys stys avail st) {st' : RefState} (hst : st' = st)
    {t t' : SignatureToken} {rest : List SignatureToken} (hstys : stys = t :: rest)
    (ht1 : refKind t = none) (ht2 : refKind t' = none) : Typed ltys (t' :: rest) avail st' := by
  subst hst; exact ht.retype hstys ht1 ht2

end Typed

theorem elementType_ok {m : CompiledModule} {idx : Nat} {t : SignatureToken}
    (h : elementType m idx = .ok t) : (m.signatures[idx]?.bind (·.head?)) = some t := by
  unfold elementType at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨sg, hsg, h⟩ := h
  split at h
  · next t' ht' =>
    simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    simp [hsg, ht']
  · simp [panicAt] at h

theorem newFactor {g g1 g2 : Graph} {c p : Nat} {mm : Bool} {path : List Label}
    (hnew : g.newRef c mm = .ok g1) (hfac : g1.factor p path c = .ok g2) (hw : g.WF)
    (hb : g.Back) : g2.WF ∧ g2.Back ∧ ∀ x, g2.mutability x = g1.mutability x := by
  have hw1 := Graph.newRef_wf hnew hw
  obtain ⟨hw2, hm2, -⟩ := Graph.factor_ok hfac hw1
  exact ⟨hw2, Graph.factor_back hfac hw1 (Graph.newRef_back hnew hb), hm2⟩

theorem newAddEdge {g g1 g2 : Graph} {c p : Nat} {mm : Bool} {e : BorrowGraph.Edge}
    (hnew : g.newRef c mm = .ok g1) (hadd : g1.addEdge p e c = .ok g2) (hw : g.WF)
    (hb : g.Back) : g2.WF ∧ g2.Back ∧ ∀ x, g2.mutability x = g1.mutability x :=
  ⟨Graph.addEdge_wf hadd (Graph.newRef_wf hnew hw),
    Graph.addEdge_back hadd (Graph.newRef_back hnew hb), Graph.addEdge_mutability hadd⟩

theorem AllPairs.exists_of_mem {α β : Type} {R : α → β → Prop} :
    ∀ {l₁ : List α} {l₂ : List β}, AllPairs R l₁ l₂ → ∀ a ∈ l₁, ∃ b ∈ l₂, R a b
  | _, _, .nil, a, ha => by simp at ha
  | _, _, .cons hr hrest, a, ha => by
    rcases List.mem_cons.1 ha with rfl | ha
    · exact ⟨_, List.mem_cons_self, hr⟩
    · obtain ⟨b, hb, hab⟩ := AllPairs.exists_of_mem hrest a ha
      exact ⟨b, List.mem_cons_of_mem _ hb, hab⟩

/-- The fields of a struct or variant are value types. -/
theorem fieldsSignature_value {P : Program} {f : FunctionRef} {m : CompiledModule}
    {view : FunctionView} (ha : GenCtx P f m view) {d : Nat} {sd : StructDefinition}
    (hsd : m.structDefs[d]? = some sd) {variant : Option Nat} {fs : List SignatureToken}
    (h : fieldsSignature sd variant = .ok fs) : ∀ t ∈ fs, refKind t = none := by
  intro t ht
  have hall : ∀ fd ∈ CompiledModule.StructFieldInformation.allFields sd.fieldInformation,
      refKind fd.signature = none := fun fd hfd => by
    obtain ⟨sh, -, hsh⟩ := ha.fields sd (getElem?_mem_toList hsd)
    obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1
      (hsh fd hfd (List.replicate sh.typeParameters.length .bool) (by simp))
    exact refKind_of_ofTokenWith hu
  rcases fieldsSignature_ok h with ⟨-, fields, hfi, rfl⟩ | ⟨v, variants, vd, -, hfi, hv, rfl⟩
  · obtain ⟨fd, hfd, rfl⟩ := List.mem_map.1 ht
    exact hall fd (by simp [CompiledModule.StructFieldInformation.allFields, hfi, hfd])
  · obtain ⟨fd, hfd, rfl⟩ := List.mem_map.1 ht
    exact hall fd (by
      simp only [CompiledModule.StructFieldInformation.allFields, hfi, List.mem_flatMap]
      exact ⟨vd, List.mem_of_getElem? hv, hfd⟩)

/-- The element type of a reference to a vector is a value type. -/
theorem refKind_vecElem {m : CompiledModule} {τ : List RuntimeType} {mut' : Bool}
    {e : SignatureToken} {u : RuntimeType}
    (hu : RuntimeType.ofSlotTokenWith m τ (refTo mut' (.vector e)) = some u) : refKind e = none := by
  cases mut' <;> simp only [refTo, Bool.false_eq_true, ↓reduceIte, RuntimeType.ofSlotTokenWith,
    RuntimeType.ofTokenWith, Option.map_eq_some_iff] at hu <;>
  · obtain ⟨_, ⟨w, hw, -⟩, -⟩ := hu
    exact refKind_of_ofTokenWith hw

namespace Typed

variable {ltys stys : List SignatureToken} {avail : LocalIndex → Availability} {st : RefState}

/-- **A reference derived from the one on top**, which is released: field,
element and freeze borrows. -/
theorem borrowFrom (ht : Typed ltys stys avail st) {id c : Nat} {mm : Bool}
    {rest : List AbsValue} {g1 g2 g3 : Graph} {op T : SignatureToken}
    {rest_t : List SignatureToken} {st' : RefState}
    (hs : st.stack = .reference id :: rest) (hstys : stys = op :: rest_t)
    (hnew : st.graph.newRef c mm = .ok g1)
    (hg2 : g2.WF ∧ g2.Back ∧ ∀ x, g2.mutability x = g1.mutability x)
    (hrel : g2.release id = .ok g3) (hm : refKind T = some mm)
    (hl : st'.locals = st.locals) (hg : st'.graph = g3)
    (hs' : st'.stack = .reference c :: rest) : Typed ltys (T :: rest_t) avail st' := by
  have h1 := ht.pushNew hnew hg2.1 hg2.2.1 hg2.2.2 hm
  have h2 := h1.releaseAt (pre := [.reference c]) (rest := rest) (id := id) (tpre := [T])
    (t := op) (stys' := rest_t) (by simp [hs]) hrel (by simp [hstys]) rfl
  exact h2.ext hl hg hs'

/-- **Reading through the reference on top**, which is released, and pushing
a value. -/
theorem releasePush (ht : Typed ltys stys avail st) {id : Nat} {rest : List AbsValue}
    {g : Graph} {op T : SignatureToken} {rest_t : List SignatureToken} {st' : RefState}
    (hs : st.stack = .reference id :: rest) (hrel : st.graph.release id = .ok g)
    (hstys : stys = op :: rest_t) (hT : refKind T = none)
    (hl : st'.locals = st.locals) (hg : st'.graph = g)
    (hs' : st'.stack = .nonReference :: rest) : Typed ltys (T :: rest_t) avail st' := by
  have h1 := ht.popRelease hs hrel hstys
  have h2 := h1.ofValueStep (P := [T]) (stys' := T :: rest_t) ValueStep.pushOne rfl
    (by simpa using hT) (by simp)
  exact h2.ext hl hg hs'

/-- Pushing a value. -/
theorem pushValue (ht : Typed ltys stys avail st) {T : SignatureToken} (hT : refKind T = none) :
    Typed ltys (T :: stys) avail { st with stack := .nonReference :: st.stack } :=
  ht.ofValueStep (P := [T]) (stys' := T :: stys) ValueStep.pushOne rfl (by simpa using hT)
    (by simp)

/-- Popping two values and pushing one. -/
theorem popTwoPush (ht : Typed ltys stys avail st) {st' : RefState} {a b T : SignatureToken}
    {rest : List SignatureToken} (hvs : ValueStep 2 1 st st') (hstys : stys = a :: b :: rest)
    (hT : refKind T = none) : Typed ltys (T :: rest) avail st' :=
  ht.ofValueStep (P := [T]) hvs rfl (by simpa using hT) (by simp [hstys])

/-- Popping a value. -/
theorem popValue (ht : Typed ltys stys avail st) {rest : List AbsValue} {t : SignatureToken}
    {rest_t : List SignatureToken} (hs : st.stack = .nonReference :: rest)
    (hstys : stys = t :: rest_t) : Typed ltys rest_t avail { st with stack := rest } :=
  ht.ofValueStep (P := []) (stys' := rest_t) (ValueStep.popOne hs) rfl (by simp)
    (by simp [hstys])

end Typed

set_option maxHeartbeats 4000000 in
/-- **Each instruction keeps the analysis's states well typed** (M1). -/
theorem typed_exec {P : Program} {f : FunctionRef} {m : CompiledModule} {view : FunctionView}
    {ltys stys stys' : List SignatureToken} {avail : LocalIndex → Availability}
    {st st' : RefState} {instr : Bytecode} {before : List RuntimeType}
    (ha : GenCtx P f m view) (hb : verifyBounds m = .ok ())
    (hltys : ltys = view.parameters ++ view.locals)
    (hsig : RuntimeType.ofSignatureWith m f.typeArgs stys = some before) (hM1 : M1Instr m instr)
    (ht : Typed ltys stys avail st) (hty : typeInstr m view stys instr = .ok stys')
    (hread : ∀ i, instr.readsLocal = some i → avail i = .available)
    (hx : (executeInner m view instr).run st = .ok ((), st')) :
    Typed ltys stys' (availabilityAfter instr avail) st' := by
  have hslot : ∀ t ∈ stys, (RuntimeType.ofSlotTokenWith m f.typeArgs t).isSome := ofSigW_tokens hsig
  cases instr
  case ret | mutBorrowGlobal | immBorrowGlobal => exact (hM1 : False).elim
  case callGeneric | packGeneric | packVariantGeneric | unpackGeneric | unpackVariantGeneric
    | testVariantGeneric | mutBorrowFieldGeneric | mutBorrowVariantFieldGeneric
    | immBorrowFieldGeneric | immBorrowVariantFieldGeneric | mutBorrowGlobalGeneric
    | immBorrowGlobalGeneric | existsGeneric | moveFromGeneric | moveToGeneric =>
    exact (hM1 : False).elim
  case callClosure => exact (hM1 : False).elim
  case packClosure h mask =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨fh, hfh, hty⟩ := hty
    obtain ⟨params, caps, rest, ab, returns, t, hps, rfl, hlen, -, -, -, hinst, rfl⟩ :=
      typeClosPack_ok hty
    obtain ⟨fh', params', hfh', hps', hvs⟩ := exec_packClosure hx
    rw [hfh, Option.some.injEq] at hfh'
    subst hfh'
    rw [hps, Option.some.injEq] at hps'
    subst hps'
    rw [← maskExtract_length_countP, ← hlen] at hvs
    exact ht.ofValueStep (P := [t]) hvs rfl (by simp [refKind_instantiate_function hinst])
      (by simp)
  case packClosureGeneric i mask =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨fi, hfi, tys, -, fh, hfh, hty⟩ := hty
    obtain ⟨params, caps, rest, ab, returns, t, hps, rfl, hlen, -, -, -, hinst, rfl⟩ :=
      typeClosPack_ok hty
    obtain ⟨fi', fh', params', hfi', hfh', hps', hvs⟩ := exec_packClosureGeneric hx
    rw [hfi, Option.some.injEq] at hfi'
    subst hfi'
    rw [hfh, Option.some.injEq] at hfh'
    subst hfh'
    rw [hps, Option.some.injEq] at hps'
    subst hps'
    rw [← maskExtract_length_countP, ← hlen] at hvs
    exact ht.ofValueStep (P := [t]) hvs rfl (by simp [refKind_instantiate_function hinst])
      (by simp)
  case nop | branch =>
    simp only [typeInstr, pure, Except.pure, Except.ok.injEq] at hty
    subst hty
    rw [exec_nothing (by simp) hx]
    exact ht
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32
    | castI64 | castI128 | castI256 =>
    simp only [typeInstr] at hty
    obtain ⟨op, rest, hs, hi, rfl⟩ := castTo_ok hty
    exact ht.retype' (exec_nothing (by simp) hx) hs (refKind_of_isInteger hi) rfl
  case negate =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨op, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    split at hty
    · next hc =>
      simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      exact ht.retype' (exec_nothing (by simp) hx) hp (refKind_of_isSigned hc)
        (refKind_of_isSigned hc)
    · simp [reject] at hty
  case not =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨op, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    split at hty
    · next hc =>
      simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      simp only [beq_iff_eq] at hc
      subst hc
      exact ht.retype' (exec_nothing (by simp) hx) hp rfl rfl
    · simp [reject] at hty
  case exists_ d =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨sd, -, a, -, u, -, ⟨op, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    obtain ⟨u', hop, hty⟩ := hty
    rw [rejectIf_ok] at hop
    simp only [bne_eq_false_iff_eq] at hop
    subst hop
    simp only [pure, Except.pure, Except.ok.injEq] at hty
    subst hty
    exact ht.retype' (exec_nothing (by simp) hx) hp rfl rfl
  case brTrue | brFalse | abort =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨op, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    split at hty
    · simp [reject] at hty
    · simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      obtain ⟨r, hs, rfl⟩ := exec_popOne (by simp) hx
      exact ht.popValue hs hp
  case add | sub | mul | mod | div | lt | gt | le | ge | bitOr | bitAnd | xor | shl | shr | or
    | and =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨op1, op2, rest⟩, hp, hty⟩ := hty
    rw [popTwo_ok] at hp
    try dsimp only at hty
    obtain ⟨r, hs, rfl⟩ := exec_binop (by simp) hx
    have hvs : ValueStep 2 1 st { st with stack := .nonReference :: r } :=
      ⟨by simp [hs], by simp [hs], by simp [hs]⟩
    split at hty
    · next hc =>
      simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      simp only [Bool.and_eq_true, beq_iff_eq] at hc
      refine ht.popTwoPush hvs hp ?_
      first
        | rfl
        | exact refKind_of_isInteger hc.1
        | exact refKind_of_isUnsigned hc.1
    · simp [reject] at hty
  case eq | neq =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨op1, op2, rest⟩, hp, hty⟩ := hty
    rw [popTwo_ok] at hp
    try dsimp only at hty
    obtain ⟨a, -, hty⟩ := hty
    split at hty
    · next hc =>
      simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      simp only [Bool.and_eq_true, beq_iff_eq] at hc
      obtain ⟨-, rfl⟩ := hc
      obtain ⟨v1, v2, r, hs, hcase⟩ := exec_eq (by simp) hx
      rcases hcase with ⟨rfl, rfl, rfl⟩ | ⟨id1, id2, g1, g2, rfl, rfl, hr1, hr2, rfl⟩
      · exact ht.popTwoPush ⟨by simp [hs], by simp [hs], by simp [hs]⟩ hp rfl
      · have h1 := ht.popRelease hs hr1 hp
        have h2 := h1.popRelease (id := id2) (rest := r) (g := g2) rfl hr2 rfl
        exact (h2.pushValue (T := .bool) rfl).ext rfl rfl rfl
    · simp [reject] at hty
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldTrue | ldFalse =>
    simp only [typeInstr, pure, Except.pure, Except.ok.injEq] at hty
    subst hty
    rw [exec_ld (by simp) hx]
    exact ht.pushValue rfl
  case ldConst i =>
    simp only [typeInstr, bind_ok, lookup_ok, pure, Except.pure, Except.ok.injEq] at hty
    obtain ⟨c, hc, rfl⟩ := hty
    have hval : ∀ c, m.constantPool[i]? = some c → refKind c.type = none := fun c hc => by
      obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1 (ha.constants c (getElem?_mem_toList hc))
      exact refKind_of_ofToken hu
    exact ht.ofValueStep (P := [c.type]) (exec_ldConst hval hx) rfl (by simpa using hval c hc)
      (by simp)
  case abortMsg =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨msg, rest⟩, hp, ⟨code, rest'⟩, hp', hty⟩ := hty
    rw [popType_ok] at hp hp'
    try dsimp only at hty
    split at hty
    · simp [reject] at hty
    · simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      subst hp' hp
      exact ht.ofValueStep (P := []) (exec_abortMsg hx) rfl (by simp) (by simp)
  case pop =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨op, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    obtain ⟨a, -, hty⟩ := hty
    split at hty
    · simp [reject] at hty
    · simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      obtain ⟨v, r, hs, hcase⟩ := exec_pop hx
      rcases hcase with ⟨rfl, rfl⟩ | ⟨id, g, rfl, hrel, rfl⟩
      · exact ht.popValue hs hp
      · exact ht.popRelease hs hrel hp
  case stLoc i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨op, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    obtain ⟨t, hloc, hty⟩ := hty
    split at hty
    · simp [reject] at hty
    · next hassign =>
      simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      have hti : ltys[i]? = some t := by rw [hltys]; exact localAt_ok hloc
      have hassign' : isAssignableFrom t op = true := by simpa using hassign
      obtain ⟨v, r, old, hs, hold, hcase⟩ := exec_stLoc hx
      rcases hcase with ⟨rfl, -, rfl⟩ | ⟨id, g, rfl, hrel, rfl⟩
      · exact (ht.storeLocal (g := st.graph) hs hold hti hp hassign' ht.wf ht.back
          (by simp [AbsValue.refId?]) (fun _ _ => rfl)).ext rfl rfl rfl
      · obtain ⟨hwg, hgone, hmut, -, -⟩ := Graph.release_ok hrel ht.wf
        exact ht.storeLocal hs hold hti hp hassign' hwg (Graph.release_back hrel ht.back)
          (by simpa [AbsValue.refId?] using hgone)
          (fun x hx => hmut x (by simpa [AbsValue.refId?] using hx))
  case moveLoc i =>
    simp only [typeInstr, bind_ok, pure, Except.pure] at hty
    obtain ⟨t, hloc, hty⟩ := hty
    simp only [Except.ok.injEq] at hty
    subst hty
    have hti : ltys[i]? = some t := by rw [hltys]; exact localAt_ok hloc
    obtain ⟨v, hv, rfl, -⟩ := exec_moveLoc hx
    exact ht.moveLocal hv hti (hread i rfl)
  case copyLoc i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨t, hloc, a, -, hty⟩ := hty
    split at hty
    · simp [reject] at hty
    · simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      have hti : ltys[i]? = some t := by rw [hltys]; exact localAt_ok hloc
      have hav := hread i rfl
      obtain ⟨v, hv⟩ : ∃ v, st.locals[i]? = some v := by
        have hlt : i < ltys.length := (List.getElem?_eq_some_iff.1 hti).1
        rw [← ht.nlocals] at hlt
        exact ⟨_, List.getElem?_eq_getElem hlt⟩
      cases v with
      | nonReference =>
        obtain ⟨-, rfl⟩ := exec_copyLoc_val hv hx
        have hk : refKind t = none := by
          cases hk : refKind t with
          | none => rfl
          | some b =>
            obtain ⟨x, hx'⟩ := ht.locKind i t b hti hk hav
            rw [hv] at hx'
            cases hx'
        exact ht.pushValue hk
      | reference id =>
        obtain ⟨b, g1, g2, hmut, hnew, hfac, rfl⟩ := copyLoc_ref hv hx
        obtain ⟨t', b', h1, h2, h3⟩ := ht.locMut i id hv
        rw [hti, Option.some.injEq] at h1
        subst h1
        have hbb : b = b' := by
          have := Graph.isMutable_ok hmut
          rw [h3] at this
          exact (Option.some.inj this).symm
        subst hbb
        obtain ⟨hw2, hb2, hm2⟩ := newFactor hnew hfac ht.wf ht.back
        exact (ht.pushNew hnew hw2 hb2 hm2 h2).ext rfl rfl rfl
  case pack d =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨sd, hsd, hty⟩ := hty
    obtain ⟨fs, args, rest, hfs, rfl, hlen, -, rfl⟩ := typePack_ok hty
    obtain ⟨sd', hsd', hvs⟩ := exec_pack hx
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    rw [fieldCount_of_fieldsSignature hfs, ← hlen] at hvs
    exact ht.ofValueStep (P := [.struct sd.structHandle]) hvs rfl (by simp [refKind]) (by simp)
  case packVariant hh =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨vh, hvh, sd, hsd, hty⟩ := hty
    obtain ⟨fs, args, rest, hfs, rfl, hlen, -, rfl⟩ := typePack_ok hty
    obtain ⟨vh', sd', hvh', hsd', hvs⟩ := exec_packVariant hx
    rw [hvh, Option.some.injEq] at hvh'
    subst hvh'
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    rw [fieldCount_of_fieldsSignature hfs, ← hlen] at hvs
    exact ht.ofValueStep (P := [.struct sd.structHandle]) hvs rfl (by simp [refKind]) (by simp)
  case unpack d =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨sd, hsd, hty⟩ := hty
    obtain ⟨fs, rest, rfl, hfs, rfl⟩ := typeUnpack_ok hty
    obtain ⟨sd', hsd', hvs⟩ := exec_unpack hx
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    rw [fieldCount_of_fieldsSignature hfs] at hvs
    exact ht.ofValueStep (P := fs.reverse) hvs (by simp)
      (fun t htm => fieldsSignature_value ha hsd hfs t (List.mem_reverse.1 htm)) (by simp)
  case unpackVariant hh =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨vh, hvh, sd, hsd, hty⟩ := hty
    obtain ⟨fs, rest, rfl, hfs, rfl⟩ := typeUnpack_ok hty
    obtain ⟨vh', sd', hvh', hsd', hvs⟩ := exec_unpackVariant hx
    rw [hvh, Option.some.injEq] at hvh'
    subst hvh'
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    rw [fieldCount_of_fieldsSignature hfs] at hvs
    exact ht.ofValueStep (P := fs.reverse) hvs (by simp)
      (fun t htm => fieldsSignature_value ha hsd hfs t (List.mem_reverse.1 htm)) (by simp)
  case vecPack idx n =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨elem, -, st1, hpop, hty⟩ := hty
    simp only [pure, Except.pure, Except.ok.injEq] at hty
    subst hty
    obtain ⟨args, rfl, hlen, -⟩ := popElements_ok _ hpop
    have hvs := exec_vecPack hx
    simp only [List.length_range] at hlen
    rw [← hlen] at hvs
    exact ht.ofValueStep (P := [.vector elem]) hvs rfl (by simp [refKind]) (by simp)
  case vecUnpack idx n =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨op, rest⟩, hp, elem, hel, hty⟩ := hty
    rw [popType_ok] at hp
    split at hty
    · simp [reject] at hty
    · next hc =>
      simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      have hop : op = .vector elem := by simpa using hc
      subst hop
      have helem : refKind elem = none := by
        obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1 (hslot (.vector elem) (by simp [hp]))
        simp only [RuntimeType.ofSlotTokenWith, RuntimeType.ofTokenWith, Option.map_eq_some_iff] at hu
        obtain ⟨v, hv, -⟩ := hu
        exact refKind_of_ofTokenWith hv
      have hval : ∀ ty, (m.signatures[idx]?.bind (·.head?)) = some ty → refKind ty = none := by
        intro ty hty'
        rw [elementType_ok hel, Option.some.injEq] at hty'
        subst hty'
        exact helem
      exact ht.ofValueStep (P := List.replicate n elem) (exec_vecUnpack hval hx) (by simp)
        (fun t htm => by rw [(List.mem_replicate.1 htm).2]; exact helem) (by simp [hp])
  case moveFrom d =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨sd, -, a, -, u, -, ⟨op, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    obtain ⟨u', -, hty⟩ := hty
    simp only [pure, Except.pure, Except.ok.injEq] at hty
    subst hty
    obtain ⟨-, hvs⟩ := exec_moveFrom hx
    exact ht.ofValueStep (P := [.struct sd.structHandle]) hvs rfl (by simp [refKind])
      (by simp [hp])
  case moveTo d =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨sd, -, a, -, u, -, ⟨keyOp, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    obtain ⟨⟨signerOp, rest'⟩, hp', hty⟩ := hty
    rw [popType_ok] at hp'
    try dsimp only at hty
    obtain ⟨u', -, u'', -, hty⟩ := hty
    simp only [pure, Except.pure, Except.ok.injEq] at hty
    subst hty hp'
    obtain ⟨v, r, hs, hcase⟩ := exec_moveTo hx
    have h1 := ht.popValue hs hp
    rcases hcase with ⟨rfl, rfl⟩ | ⟨id, g, rfl, hrel, rfl⟩
    · exact (h1.popValue (rest := r) rfl rfl).ext rfl rfl rfl
    · exact (h1.popRelease (id := id) (rest := r) (g := g) rfl hrel rfl).ext rfl rfl rfl
  case readRef =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨op, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    obtain ⟨id, r, g, hs, -, hrel, rfl⟩ := exec_readRef hx
    obtain ⟨u, hu⟩ := Option.isSome_iff_exists.1 (hslot op (by simp [hp]))
    cases op
    all_goals try (simp [reject] at hty; done)
    all_goals
      try dsimp only at hty
      simp only [bind_ok] at hty
      obtain ⟨a, -, hty⟩ := hty
      split at hty
      · simp [reject] at hty
      · simp only [pure, Except.pure, Except.ok.injEq] at hty
        subst hty
        exact ht.releasePush hs hrel hp (refKind_inner hu (by simp)) rfl rfl rfl
  case writeRef =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨refOp, rest⟩, hp, ⟨val, rest'⟩, hp', hty⟩ := hty
    rw [popType_ok] at hp hp'
    subst hp'
    obtain ⟨id, r, g, hs, -, hrel, rfl⟩ := exec_writeRef hx
    have hst : stys' = rest' := by
      cases refOp
      all_goals try (simp [reject] at hty; done)
      try dsimp only at hty
      simp only [bind_ok] at hty
      obtain ⟨a, -, hty⟩ := hty
      split at hty
      · simp [reject] at hty
      · split at hty
        · simp [reject] at hty
        · simp only [pure, Except.pure, Except.ok.injEq] at hty
          exact hty.symm
    subst hst
    have h1 := ht.popRelease hs hrel hp
    exact (h1.popValue (rest := r) rfl rfl).ext rfl rfl rfl
  case freezeRef =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨op, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    split at hty
    · next inner =>
      simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      obtain ⟨id, r, g1, g2, g3, hs, -, hnew, hfac, hrel, rfl⟩ := exec_freezeRef hx
      exact ht.borrowFrom hs hp hnew (newFactor hnew hfac ht.wf ht.back) hrel rfl rfl rfl rfl
    · simp [reject] at hty
  case mutBorrowLoc i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨t, hloc, hty⟩ := hty
    split at hty
    · simp [reject] at hty
    · simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      obtain ⟨g1, g2, -, -, hnew, hfac, rfl⟩ := exec_borrowLoc (mut_ := true) hx
      obtain ⟨hw2, hb2, hm2⟩ := newFactor hnew hfac ht.wf ht.back
      exact (ht.pushNew hnew hw2 hb2 hm2 rfl).ext rfl rfl rfl
  case immBorrowLoc i =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨t, hloc, hty⟩ := hty
    split at hty
    · simp [reject] at hty
    · simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      obtain ⟨g1, g2, -, -, hnew, hfac, rfl⟩ := exec_borrowLoc (mut_ := false) hx
      obtain ⟨hw2, hb2, hm2⟩ := newFactor hnew hfac ht.wf ht.back
      exact (ht.pushNew hnew hw2 hb2 hm2 rfl).ext rfl rfl rfl
  case mutBorrowField hh | immBorrowField hh =>
    simp only [typeInstr] at hty
    obtain ⟨op, rest, fh, sd, rfl, -, hfh, hsd, -, rfl⟩ := fieldBorrow_ok hty
    obtain ⟨sd', hsd', hlt⟩ := verifyBounds_fieldHandle hb hfh
    rw [hsd, Option.some.injEq] at hsd'
    subst hsd'
    obtain ⟨fd, hfd⟩ : ∃ fd, (Model.structFields sd.fieldInformation none)[fh.field]? = some fd := by
      cases hfi : sd.fieldInformation with
      | declared fields =>
        rw [hfi] at hlt
        simp only [fieldCount] at hlt
        exact ⟨fields[fh.field], by simp [Model.structFields, List.getElem?_eq_getElem hlt]⟩
      | declaredVariants _ => rw [hfi] at hlt; simp [fieldCount] at hlt
      | native => rw [hfi] at hlt; simp [fieldCount] at hlt
    simp only [hfd]
    first
      | obtain ⟨fh', hfh', id, r, g1, g2, g3, hs, -, hnew, hfac, hrel, rfl⟩ :=
          exec_mutBorrowField hx
        rw [hfh, Option.some.injEq] at hfh'
        subst hfh'
        exact ht.borrowFrom hs rfl hnew (newFactor hnew hfac ht.wf ht.back) hrel
          (refKind_refTo _ _) rfl rfl rfl
      | obtain ⟨fh', hfh', id, r, g1, g2, g3, hs, -, hnew, hfac, hrel, rfl⟩ :=
          exec_immBorrowField hx
        rw [hfh, Option.some.injEq] at hfh'
        subst hfh'
        exact ht.borrowFrom hs rfl hnew (newFactor hnew hfac ht.wf ht.back) hrel
          (refKind_refTo _ _) rfl rfl rfl
  case mutBorrowVariantField hh | immBorrowVariantField hh =>
    simp only [typeInstr] at hty
    obtain ⟨op, rest, fh, sd, rfl, -, hfh, hsd, -, -, ft, rfl, -⟩ := variantBorrow_ok hty
    first
      | obtain ⟨fh', hfh', id, r, g1, g2, g3, hs, -, hnew, hfac, hrel, rfl⟩ :=
          exec_mutBorrowVariantField hx
        exact ht.borrowFrom hs rfl hnew (newFactor hnew hfac ht.wf ht.back) hrel
          (refKind_refTo _ _) rfl rfl rfl
      | obtain ⟨fh', hfh', id, r, g1, g2, g3, hs, -, hnew, hfac, hrel, rfl⟩ :=
          exec_immBorrowVariantField hx
        exact ht.borrowFrom hs rfl hnew (newFactor hnew hfac ht.wf ht.back) hrel
          (refKind_refTo _ _) rfl rfl rfl
  case testVariant hh =>
    simp only [typeInstr, bind_ok, lookup_ok] at hty
    obtain ⟨vh, -, sd, -, ⟨arg, rest⟩, hp, hty⟩ := hty
    rw [popType_ok] at hp
    try dsimp only at hty
    have hst : stys' = .bool :: rest := by
      split at hty
      · split at hty
        · simp only [pure, Except.pure, Except.ok.injEq] at hty; exact hty.symm
        · simp [reject] at hty
      · split at hty
        · simp only [pure, Except.pure, Except.ok.injEq] at hty; exact hty.symm
        · simp [reject] at hty
      · simp [reject] at hty
    subst hst
    obtain ⟨id, r, g, hs, hrel, rfl⟩ := exec_testVariant hx
    exact ht.releasePush hs hrel hp rfl rfl rfl rfl
  case vecLen idx =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨op, rest⟩, hp, elem, -, hty⟩ := hty
    rw [popType_ok] at hp
    split at hty
    · simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      obtain ⟨id, r, g, hs, hrel, rfl⟩ := exec_vecLen hx
      exact ht.releasePush hs hrel hp rfl rfl rfl rfl
    · simp [reject] at hty
  case vecPopBack idx =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨op, rest⟩, hp, elem, hel, hty⟩ := hty
    rw [popType_ok] at hp
    split at hty
    · next hc =>
      simp only [pure, Except.pure, Except.ok.injEq] at hty
      subst hty
      have hc' : vectorRefElement op true = some elem := by simpa using hc
      obtain ⟨mut', rfl, -⟩ := vectorRefElement_some hc'
      obtain ⟨u, hu⟩ :=
        Option.isSome_iff_exists.1 (hslot (refTo mut' (.vector elem)) (by simp [hp]))
      have helem := refKind_vecElem hu
      have hval : ∀ ty, (m.signatures[idx]?.bind (·.head?)) = some ty → refKind ty = none := by
        intro ty hty'
        rw [elementType_ok hel, Option.some.injEq] at hty'
        subst hty'
        exact helem
      obtain ⟨id, r, g, hs, -, hrel, rfl⟩ := exec_vecPopBack hval hx
      exact ht.releasePush hs hrel hp helem rfl rfl rfl
    · simp [reject] at hty
  case vecPushBack idx =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨elemOp, rest⟩, hp, ⟨vecOp, rest'⟩, hp', elem, -, u, -, u', -, hty⟩ := hty
    rw [popType_ok] at hp hp'
    simp only [pure, Except.pure, Except.ok.injEq] at hty
    subst hty hp'
    obtain ⟨id, r, g, hs, -, hrel, rfl⟩ := exec_vecPushBack hx
    have h1 := ht.popValue (rest := .reference id :: r) hs hp
    exact (h1.popRelease (id := id) (rest := r) (g := g) rfl hrel rfl).ext rfl rfl rfl
  case vecSwap idx =>
    simp only [typeInstr, bind_ok] at hty
    obtain ⟨⟨i2, rest⟩, hp, ⟨i1, rest'⟩, hp', ⟨vecOp, rest''⟩, hp'', u, -, elem, -, u', -, hty⟩ :=
      hty
    rw [popType_ok] at hp hp' hp''
    simp only [pure, Except.pure, Except.ok.injEq] at hty
    subst hty hp'' hp'
    obtain ⟨id, r, g, hs, -, hrel, rfl⟩ := exec_vecSwap hx
    have h1 := ht.popValue hs hp
    have h2 := h1.popValue (rest := .reference id :: r) rfl rfl
    exact (h2.popRelease (id := id) (rest := r) (g := g) rfl hrel rfl).ext rfl rfl rfl
  case vecImmBorrow idx =>
    simp only [typeInstr, bind_ok, borrowVectorElement] at hty
    obtain ⟨elem, -, ⟨iop, rest⟩, hp, ⟨vop, rest'⟩, hp', u, -, hty⟩ := hty
    rw [popType_ok] at hp hp'
    subst hp'
    have hst : stys' = refTo false elem :: rest' := by
      split at hty
      · split at hty
        · next hc =>
          simp only [beq_iff_eq] at hc
          subst hc
          simp only [pure, Except.pure, Except.ok.injEq] at hty
          exact hty.symm
        · simp [reject] at hty
      · simp [reject] at hty
    subst hst
    obtain ⟨id, r, g1, g2, g3, hs, -, -, hnew, hadd, hrel, rfl⟩ :=
      exec_vecBorrow (mut_ := false) hx
    have h1 := ht.popValue (rest := .reference id :: r) hs hp
    exact h1.borrowFrom (id := id) (rest := r) rfl rfl hnew
      (newAddEdge hnew hadd ht.wf ht.back) hrel (refKind_refTo _ _) rfl rfl rfl
  case vecMutBorrow idx =>
    simp only [typeInstr, bind_ok, borrowVectorElement] at hty
    obtain ⟨elem, -, ⟨iop, rest⟩, hp, ⟨vop, rest'⟩, hp', u, -, hty⟩ := hty
    rw [popType_ok] at hp hp'
    subst hp'
    have hst : stys' = refTo true elem :: rest' := by
      split at hty
      · split at hty
        · next hc =>
          simp only [beq_iff_eq] at hc
          subst hc
          simp only [pure, Except.pure, Except.ok.injEq] at hty
          exact hty.symm
        · simp [reject] at hty
      · simp [reject] at hty
    subst hst
    obtain ⟨id, r, g1, g2, g3, hs, -, -, hnew, hadd, hrel, rfl⟩ :=
      exec_vecBorrow (mut_ := true) hx
    have h1 := ht.popValue (rest := .reference id :: r) hs hp
    exact h1.borrowFrom (id := id) (rest := r) rfl rfl hnew
      (newAddEdge hnew hadd ht.wf ht.back) hrel (refKind_refTo _ _) rfl rfl rfl
  case call hh =>
    simp only [typeInstr, typeCall, bind_ok, lookup_ok] at hty
    obtain ⟨fh, hfh, ps, hps, st1, hpop, rs, hrs, hty⟩ := hty
    simp only [pure, Except.pure, Except.ok.injEq] at hty
    subst hty
    obtain ⟨args, rfl, hlen, hall⟩ := popExpected_ok hpop
    obtain ⟨hattr, hpv, hrv⟩ := hM1 fh hfh
    obtain ⟨fh', hfh', hcall⟩ := exec_call_values hx
    rw [hfh, Option.some.injEq] at hfh'
    subst hfh'
    obtain ⟨ps', rs', hps', hrs', hvs⟩ := hcall hattr
    rw [hps, Option.some.injEq] at hps'
    subst hps'
    rw [hrs, Option.some.injEq] at hrs'
    subst hrs'
    have hargsv : ∀ a ∈ args, refKind a = none := by
      intro a ha
      obtain ⟨p, hp, hpa⟩ := AllPairs.exists_of_mem hall a ha
      rw [← refKind_of_assignable hpa]
      exact hpv ps hps p (List.mem_reverse.1 hp)
    have hnr : ∀ v ∈ st.stack.take ps.length, v = .nonReference := by
      intro v hv
      obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hv
      rw [List.getElem?_take] at hk
      split at hk
      · next hlt =>
        have hka : k < args.length := by omega
        have htk : (args ++ st1)[k]? = some args[k] := by
          rw [List.getElem?_append_left hka]; exact List.getElem?_eq_getElem hka
        exact ht.nonRef_of_value htk (hargsv _ (List.getElem_mem hka)) hk
      · cases hk
    exact ht.ofValueStep (P := rs.reverse) (hvs hnr (hrv rs hrs)) (by simp)
      (fun t htm => hrv rs hrs t (List.mem_reverse.1 htm)) (by simp [← hlen])

end MoveBytecode.Proofs
