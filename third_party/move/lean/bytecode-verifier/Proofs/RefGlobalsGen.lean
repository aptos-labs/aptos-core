-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefGlobalFacts
import Proofs.RefRefsGen

/-!
# The generic global storage operations, safe

UNTRUSTED (checked by the kernel). `ExistsGeneric`, `MoveFromGeneric`,
`MoveToGeneric` and the generic global borrows run as their non-generic forms
at the instance their instantiation names (`exists_safeAt`,
`refSafe_moveFromAt`, `refSafe_moveToAt`, `refSafe_borrowGlobalAt`): the
analysis runs them as the non-generic forms at the instantiation's definition,
whose label stands for every instance of the struct (`StructIdOf.erase`), and
the acquires verifier checks that definition.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

variable {P : Program} {s : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit} {view : FunctionView} {A : RefState} {stys : List SignatureToken}
  {tys : List RuntimeType} {ps : List Reference} {tc : TypeCertificate}
  {ac : AvailabilityCertificate}

/-- **What the acquires verifier checks of a generic global operation**: the
definition its instantiation names is annotated. -/
theorem acquires_instrGeneric {m : CompiledModule} {fdef : FunctionDefinition} {code : CodeUnit}
    (h : verifyAcquires m fdef code = .ok ()) {i : Bytecode} (hi : i ∈ code.code.toList)
    {k : Nat} (hk : i = .moveFromGeneric k ∨ i = .mutBorrowGlobalGeneric k ∨
      i = .immBorrowGlobalGeneric k) {si : StructDefInstantiation}
    (hsi : m.structDefInstantiations[k]? = some si) :
    si.def_ ∈ fdef.acquiresGlobalResources := by
  unfold verifyAcquires at h
  simp only [bind_ok] at h
  obtain ⟨actual, hfold, -⟩ := h
  obtain ⟨b, r', hstep⟩ := foldlM_ok_mem hfold i hi
  have hs : Acquires.structAcquire fdef.acquiresGlobalResources b si.def_ = .ok r' := by
    rcases hk with rfl | rfl | rfl <;>
      simpa only [Acquires.instr, bind_ok, lookup_ok, hsi, Option.some.injEq, exists_eq_left'] using hstep
  simp only [Acquires.structAcquire] at hs
  split at hs
  · next hc => simpa using hc
  · simp [reject] at hs

/-- The instance a generic form names is a type of the program. -/
theorem InstanceAt.structType? {f : FunctionRef} {m : CompiledModule} {d sig : Nat}
    {targs : List RuntimeType} {sid : StructId} (h : InstanceAt P f m d sig targs sid) :
    ∃ sinfo, P.structType? sid = some sinfo := by
  obtain ⟨s0, -, -, -, -, hwf⟩ := h
  simp only [RuntimeType.wellFormed] at hwf
  split at hwf
  · next sinfo hs => exact ⟨sinfo, hs⟩
  · simp at hwf

/-- The struct an instance instantiates is the definition's. -/
theorem InstanceAt.definedStruct {f : FunctionRef} {m : CompiledModule} {d sig : Nat}
    {targs : List RuntimeType} {sid : StructId} (h : InstanceAt P f m d sig targs sid) :
    P.definedStruct? f.module d = some sid.erase := by
  obtain ⟨s0, hdef, -, -, rfl, -⟩ := h
  have h0 := definedStruct?_typeArgs hdef
  rw [hdef]
  cases s0
  simp_all [StructIdOf.erase]

/-- The global type of an instance with `key`, as `Step.globalType` finds it. -/
theorem InstanceAt.globalType {f : FunctionRef} {m : CompiledModule} {d sig : Nat}
    {targs : List RuntimeType} {sid : StructId} (h : InstanceAt P f m d sig targs sid)
    (hkey : ((RuntimeType.struct sid).abilities P).key = true) :
    Step.globalType P f.module d targs = .ok sid := by
  obtain ⟨sinfo, hst⟩ := h.structType?
  simp [Step.globalType, h.localStruct hst, hkey, bind, Except.bind, pure, Except.pure]

/-- The type certificate's key type of a generic global operation. -/
theorem keyType_of {f : FunctionRef} {m : CompiledModule} {d sig : Nat} {targs : List RuntimeType}
    {s1 sid : StructId} (hinst : InstanceAt P f m d sig targs s1)
    (hk : (if ((RuntimeType.struct s1).abilities P).key = true then some s1 else none) = some sid) :
    InstanceAt P f m d sig targs sid ∧ ((RuntimeType.struct sid).abilities P).key = true := by
  split at hk
  · next hkey =>
    simp only [Option.some.injEq] at hk
    subst hk
    exact ⟨hinst, hkey⟩
  · simp at hk

theorem typeAfter_existsGeneric {f : FunctionRef} {m : CompiledModule} {i : Nat}
    {before after : List RuntimeType}
    (h : typeAfter P f m (.existsGeneric i) before = some after) :
    ∃ x targs sid brest, m.structDefInstantiations[i]? = some x ∧
      InstanceAt P f m x.def_ x.typeParameters targs sid ∧
      ((RuntimeType.struct sid).abilities P).key = true ∧
      before = .address :: brest ∧ after = .bool :: brest := by
  have h' := h
  dsimp only [typeAfter] at h'
  simp only [bind, Option.bind_eq_some_iff] at h'
  obtain ⟨sid, ⟨x, hx, s1, ⟨s0, hdef, targs, ⟨a, htargs, hmatch⟩, hif⟩, hk⟩, h'⟩ := h'
  obtain ⟨hinst, hkey⟩ := keyType_of (instanceAt_of hdef hmatch htargs hif) hk
  split at h'
  · next brest => exact ⟨x, targs, sid, brest, hx, hinst, hkey, rfl, (Option.some.inj h').symm⟩
  · simp at h'

theorem typeAfter_moveFromGeneric {f : FunctionRef} {m : CompiledModule} {i : Nat}
    {before after : List RuntimeType}
    (h : typeAfter P f m (.moveFromGeneric i) before = some after) :
    ∃ x targs sid brest, m.structDefInstantiations[i]? = some x ∧
      InstanceAt P f m x.def_ x.typeParameters targs sid ∧
      ((RuntimeType.struct sid).abilities P).key = true ∧
      before = .address :: brest ∧ after = .struct sid :: brest := by
  have h' := h
  dsimp only [typeAfter] at h'
  simp only [bind, Option.bind_eq_some_iff] at h'
  obtain ⟨sid, ⟨x, hx, s1, ⟨s0, hdef, targs, ⟨a, htargs, hmatch⟩, hif⟩, hk⟩, h'⟩ := h'
  obtain ⟨hinst, hkey⟩ := keyType_of (instanceAt_of hdef hmatch htargs hif) hk
  split at h'
  · next brest => exact ⟨x, targs, sid, brest, hx, hinst, hkey, rfl, (Option.some.inj h').symm⟩
  · simp at h'

theorem typeAfter_moveToGeneric {f : FunctionRef} {m : CompiledModule} {i : Nat}
    {before after : List RuntimeType}
    (h : typeAfter P f m (.moveToGeneric i) before = some after) :
    ∃ x targs sid brest, m.structDefInstantiations[i]? = some x ∧
      InstanceAt P f m x.def_ x.typeParameters targs sid ∧
      ((RuntimeType.struct sid).abilities P).key = true ∧
      before = .struct sid :: .ref false .signer :: brest ∧ after = brest := by
  have h' := h
  dsimp only [typeAfter] at h'
  simp only [bind, Option.bind_eq_some_iff] at h'
  obtain ⟨sid, ⟨x, hx, s1, ⟨s0, hdef, targs, ⟨a, htargs, hmatch⟩, hif⟩, hk⟩, h'⟩ := h'
  obtain ⟨hinst, hkey⟩ := keyType_of (instanceAt_of hdef hmatch htargs hif) hk
  split at h'
  · next s' brest =>
    split at h'
    · next hs' =>
      subst hs'
      exact ⟨x, targs, s', brest, hx, hinst, hkey, rfl, (Option.some.inj h').symm⟩
    · simp at h'
  · simp at h'

theorem typeAfter_borrowGlobalGeneric {f : FunctionRef} {m : CompiledModule} {i : Nat}
    {mut_ : Bool} {before after : List RuntimeType}
    (h : typeAfter P f m (if mut_ then .mutBorrowGlobalGeneric i else .immBorrowGlobalGeneric i)
      before = some after) :
    ∃ x targs sid brest, m.structDefInstantiations[i]? = some x ∧
      InstanceAt P f m x.def_ x.typeParameters targs sid ∧
      ((RuntimeType.struct sid).abilities P).key = true ∧
      before = .address :: brest ∧ after = .ref mut_ (.struct sid) :: brest := by
  have h' : (do
      let s ← (do
        let x ← m.structDefInstantiations[i]?
        let s ← (do
          let s ← P.definedStruct? f.module x.def_
          let s := { s with typeArgs := ← (do
            match ← m.typeArgsWith f.typeArgs x.typeParameters with
            | [] => none
            | t :: ts => some (t :: ts)) }
          if (RuntimeType.struct s).wellFormed P then some s else none)
        if ((RuntimeType.struct s).abilities P).key then some s else none)
      match before with
      | .address :: rest => some (.ref mut_ (.struct s) :: rest)
      | _ => none) = some after := by
    cases mut_ <;> exact h
  clear h
  simp only [bind, Option.bind_eq_some_iff] at h'
  obtain ⟨sid, ⟨x, hx, s1, ⟨s0, hdef, targs, ⟨a, htargs, hmatch⟩, hif⟩, hk⟩, h'⟩ := h'
  obtain ⟨hinst, hkey⟩ := keyType_of (instanceAt_of hdef hmatch htargs hif) hk
  split at h'
  · next brest => exact ⟨x, targs, sid, brest, hx, hinst, hkey, rfl, (Option.some.inj h').symm⟩
  · simp at h'

/-- The step of a generic global operation, at the instance. -/
theorem exec_globalGeneric {i : Nat} {x : StructDefInstantiation} {targs : List RuntimeType}
    {sid : StructId} (hx : info.module.structDefInstantiations[i]? = some x)
    (hinst : InstanceAt P fr.function info.module x.def_ x.typeParameters targs sid) :
    Step.exec P s fr rest info code (.existsGeneric i) =
        Step.existsResource P s fr rest x.def_ targs ∧
      Step.exec P s fr rest info code (.moveFromGeneric i) =
        Step.moveResourceFrom P s fr rest x.def_ targs ∧
      Step.exec P s fr rest info code (.moveToGeneric i) =
        Step.moveResourceTo P s fr rest x.def_ targs ∧
      ∀ mut_ : Bool, Step.exec P s fr rest info code
        (if mut_ then .mutBorrowGlobalGeneric i else .immBorrowGlobalGeneric i) =
          Step.borrowGlobal P s fr rest mut_ x.def_ targs := by
  refine ⟨?_, ?_, ?_, fun mut_ => ?_⟩ <;> try cases mut_
  all_goals simp only [Bool.false_eq_true, if_false, if_true, Step.exec, Step.need, hx, bind,
    Except.bind, pure, Except.pure, hinst.genericArgs]

/-- **`ExistsGeneric`** keeps the type and availability invariant. -/
theorem exists_safeGeneric (hc : Certified P tc ac) {N : NativeRel}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    {i : Nat} (hi : code.code[fr.pc]? = some (.existsGeneric i)) :
    ExecSafe P N (Inv P tc ac) (Step.exec P s fr rest info code (.existsGeneric i)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, -, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  obtain ⟨x, targs, sid, -, hx, hinst, -⟩ := typeAfter_existsGeneric hafter
  refine exists_safeAt (d := x.def_) (targs := targs) hc hact hchain hi rfl (fun _ => rfl)
    (fun b a h => ?_) (exec_globalGeneric hx hinst).1
  obtain ⟨x', targs', sid', brest, hx', hinst', hkey, h1, h2⟩ := typeAfter_existsGeneric h
  rw [hx, Option.some.injEq] at hx'
  subst hx'
  obtain ⟨rfl, rfl⟩ := hinst.unique hinst'
  exact ⟨_, brest, hinst.globalType hkey, h1, h2⟩

/-- **`MoveFromGeneric`.** -/
theorem refSafe_moveFromGeneric (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {i : Nat}
    (hctx : TopCtx P s fr info code (.moveFromGeneric i) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.moveFromGeneric i)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (.moveFromGeneric i)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, -, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  obtain ⟨x, targs, sid, -, hx, hinst, -⟩ := typeAfter_moveFromGeneric hafter
  have h1 := acquires_instrGeneric (function_acquires hwf hact.fn hact.body)
    (getElem?_mem_toList hi) (Or.inl rfl) hx
  refine refSafe_moveFromAt (d := x.def_) (targs := targs) hc hr hsf hact hchain hctx hi
    (by intro h; cases h) rfl (fun _ => rfl)
    (by simp only [acqOf, hact.fn, Option.map_some, Option.getD_some]; exact h1)
    (fun A' hrun => ?_) (fun b a h => ?_) (exec_globalGeneric hx hinst).2.1
  · obtain ⟨x', hx', hrun'⟩ := exec_moveFromGeneric' hrun
    rw [hx, Option.some.injEq] at hx'
    subst hx'
    exact hrun'
  · obtain ⟨x', targs', sid', brest, hx', hinst', hkey, h1, h2⟩ := typeAfter_moveFromGeneric h
    rw [hx, Option.some.injEq] at hx'
    subst hx'
    obtain ⟨rfl, rfl⟩ := hinst.unique hinst'
    exact ⟨_, brest, hinst.globalType hkey, hinst.definedStruct, h1, h2⟩

/-- **`MoveToGeneric`.** -/
theorem refSafe_moveToGeneric (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {i : Nat}
    (hctx : TopCtx P s fr info code (.moveToGeneric i) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.moveToGeneric i)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (.moveToGeneric i)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, -, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  obtain ⟨x, targs, sid, -, hx, hinst, -⟩ := typeAfter_moveToGeneric hafter
  refine refSafe_moveToAt (d := x.def_) (targs := targs) hc hr hsf hact hchain hctx hi
    (by intro h; cases h) rfl (fun _ => rfl) (fun A' hrun => hrun) (fun b a h => ?_)
    (exec_globalGeneric hx hinst).2.2.1
  obtain ⟨x', targs', sid', brest, hx', hinst', hkey, h1, h2⟩ := typeAfter_moveToGeneric h
  rw [hx, Option.some.injEq] at hx'
  subst hx'
  obtain ⟨rfl, rfl⟩ := hinst.unique hinst'
  exact ⟨_, brest, hinst.globalType hkey, hkey, h1, h2⟩

/-- **`MutBorrowGlobalGeneric`, `ImmBorrowGlobalGeneric`.** -/
theorem refSafe_borrowGlobalGeneric (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {i : Nat} {mut_ : Bool}
    (hctx : TopCtx P s fr info code
      (if mut_ then .mutBorrowGlobalGeneric i else .immBorrowGlobalGeneric i) view A stys tys ps)
    (hi : code.code[fr.pc]? =
      some (if mut_ then .mutBorrowGlobalGeneric i else .immBorrowGlobalGeneric i)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code
      (if mut_ then .mutBorrowGlobalGeneric i else .immBorrowGlobalGeneric i)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, -, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  obtain ⟨x, targs, sid, -, hx, hinst, -⟩ := typeAfter_borrowGlobalGeneric hafter
  have h1 := acquires_instrGeneric (function_acquires hwf hact.fn hact.body)
    (getElem?_mem_toList hi) (by cases mut_ <;> simp) hx
  refine refSafe_borrowGlobalAt (d := x.def_) (mut_ := mut_) (targs := targs) hc hr hsf hact
    hchain hctx hi (by cases mut_ <;> intro h <;> cases h) (by cases mut_ <;> rfl)
    (fun _ => by cases mut_ <;> rfl)
    (by simp only [acqOf, hact.fn, Option.map_some, Option.getD_some]; exact h1)
    (fun A' hrun => ?_) (fun b a h => ?_) ((exec_globalGeneric hx hinst).2.2.2 mut_)
  · obtain ⟨x', hx', hrun'⟩ := exec_borrowGlobalGeneric' hrun
    rw [hx, Option.some.injEq] at hx'
    subst hx'
    exact hrun'
  · obtain ⟨x', targs', sid', brest, hx', hinst', hkey, h1, h2⟩ := typeAfter_borrowGlobalGeneric h
    rw [hx, Option.some.injEq] at hx'
    subst hx'
    obtain ⟨rfl, rfl⟩ := hinst.unique hinst'
    exact ⟨_, brest, hinst.globalType hkey, hinst.definedStruct, h1, h2⟩

end MoveBytecode.Proofs
