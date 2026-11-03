-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefWrites

/-!
# Global storage, safe

UNTRUSTED (checked by the kernel). `Exists`, `MoveFrom` and `MoveTo` on a type
of the current module with `key` do not go wrong: the operands have the types
the type certificate gives, a resource in well-typed storage conforms to its
type, and `MoveTo` reads the address through a tracked `&signer`. `MoveTo`
keeps storage well typed and the reference invariant; `Exists` and `MoveFrom`
are steps on values (`refInv_valueOnly`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit} {view : FunctionView} {A : RefState} {stys : List SignatureToken}
  {tys : List RuntimeType} {ps : List Reference} {tc : TypeCertificate} {ac : AvailabilityCertificate}

theorem conforms_address {v : Value} (h : v.conforms P .address = true) : ∃ a, v = .address a := by
  have := conforms_type h rfl
  cases v <;> simp_all [Value.type]

theorem conforms_signer {v : Value} (h : v.conforms P .signer = true) : ∃ a, v = .signer a := by
  have := conforms_type h rfl
  cases v <;> simp_all [Value.type]

/-- A type with `key` is a non-generic struct type. -/
theorem structType_of_key {sid : StructId} (h : ((RuntimeType.struct sid).abilities P).key = true) :
    ∃ info, P.structType? sid = some info := by
  cases hs : P.structType? sid with
  | none => simp [RuntimeType.abilities, hs, AbilitySet.empty] at h
  | some info => exact ⟨info, rfl⟩

theorem globalType_of_key {mi : Nat} {d : StructDefinitionIndex} {sid : StructId}
    (h : P.definedStruct? mi d = some sid) (hkey : ((RuntimeType.struct sid).abilities P).key = true) :
    Step.globalType P mi d = .ok sid := by
  obtain ⟨info, hinfo⟩ := structType_of_key hkey
  simp [Step.globalType, localStruct_ok h hinfo, hkey, bind, Except.bind, pure, Except.pure]

/-! ## The type certificate's typing -/

theorem typeAfter_exists {f : FunctionRef} {m : CompiledModule} {d : StructDefinitionIndex}
    {before after : List RuntimeType} (h : typeAfter P f m (.exists_ d) before = some after) :
    ∃ sid brest, P.definedStruct? f.module d = some sid ∧
      ((RuntimeType.struct sid).abilities P).key = true ∧
      before = .address :: brest ∧ after = .bool :: brest := by
  have h' : (do
      let _ ← (do
        let s ← P.definedStruct? f.module d
        if ((RuntimeType.struct s).abilities P).key then some s else none)
      match before with
      | .address :: rest => some (.bool :: rest)
      | _ => none) = some after := h
  clear h
  cases hds : P.definedStruct? f.module d with
  | none => simp [hds] at h'
  | some sid =>
    simp only [hds, bind, Option.bind_some] at h'
    split at h'
    · next hkey =>
      simp only [Option.bind_some] at h'
      split at h'
      · next brest => exact ⟨sid, brest, rfl, hkey, rfl, (Option.some.inj h').symm⟩
      · simp at h'
    · simp at h'

theorem typeAfter_moveFrom {f : FunctionRef} {m : CompiledModule} {d : StructDefinitionIndex}
    {before after : List RuntimeType} (h : typeAfter P f m (.moveFrom d) before = some after) :
    ∃ sid brest, P.definedStruct? f.module d = some sid ∧
      ((RuntimeType.struct sid).abilities P).key = true ∧
      before = .address :: brest ∧ after = .struct sid :: brest := by
  have h' : (do
      let s ← (do
        let s ← P.definedStruct? f.module d
        if ((RuntimeType.struct s).abilities P).key then some s else none)
      match before with
      | .address :: rest => some (.struct s :: rest)
      | _ => none) = some after := h
  clear h
  cases hds : P.definedStruct? f.module d with
  | none => simp [hds] at h'
  | some sid =>
    simp only [hds, bind, Option.bind_some] at h'
    split at h'
    · next hkey =>
      simp only [Option.bind_some] at h'
      split at h'
      · next brest => exact ⟨sid, brest, rfl, hkey, rfl, (Option.some.inj h').symm⟩
      · simp at h'
    · simp at h'

theorem typeAfter_moveTo {f : FunctionRef} {m : CompiledModule} {d : StructDefinitionIndex}
    {before after : List RuntimeType} (h : typeAfter P f m (.moveTo d) before = some after) :
    ∃ sid brest, P.definedStruct? f.module d = some sid ∧
      ((RuntimeType.struct sid).abilities P).key = true ∧
      before = .struct sid :: .ref false .signer :: brest ∧ after = brest := by
  have h' : (do
      let s ← (do
        let s ← P.definedStruct? f.module d
        if ((RuntimeType.struct s).abilities P).key then some s else none)
      match before with
      | .struct s' :: .ref false .signer :: rest => if s' = s then some rest else none
      | _ => none) = some after := h
  clear h
  cases hds : P.definedStruct? f.module d with
  | none => simp [hds] at h'
  | some sid =>
    simp only [hds, bind, Option.bind_some] at h'
    split at h'
    · next hkey =>
      simp only [Option.bind_some] at h'
      split at h'
      · next s' brest =>
        split at h'
        · next hs' =>
          subst hs'
          exact ⟨s', brest, rfl, hkey, rfl, (Option.some.inj h').symm⟩
        · simp at h'
      · simp at h'
    · simp at h'

/-! ## `Exists`, `MoveFrom` -/

theorem popAddress_of {a : Address} {vs : List Value} (hst : fr.stack = .address a :: vs) :
    Step.popAddress fr = .ok (a, { fr with stack := vs }) := by
  simp [Step.popAddress, Step.pop, hst, bind, Except.bind, pure, Except.pure]

/-- **An existence test** of definition `d` at type arguments `targs`,
`Exists` or its generic form, keeps the type and availability invariant. -/
theorem exists_safeAt (hc : Certified P tc ac) {N : NativeRel}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    {instr : Bytecode} {d : StructDefinitionIndex} {targs : List RuntimeType}
    (hi : code.code[fr.pc]? = some instr) (hctl : instr.control = .fallThrough)
    (hav : ∀ a, availabilityAfter instr a = a)
    (htyp : ∀ before after, typeAfter P fr.function info.module instr before = some after →
      ∃ sid brest, Step.globalType P fr.function.module d targs = .ok sid ∧
        before = .address :: brest ∧ after = .bool :: brest)
    (hexec : Step.exec P s fr rest info code instr = Step.existsResource P s fr rest d targs) :
    ExecSafe P N (Inv P tc ac) (Step.exec P s fr rest info code instr) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  obtain ⟨tys, htc, hconf⟩ := hact.types
  rw [hbefore, Option.some.injEq] at htc
  subst htc
  obtain ⟨sid, brest, hglob, rfl, rfl⟩ := htyp _ _ hafter
  obtain ⟨va, vs, hst, hva, hvs⟩ := conformsAll_cons_right hconf
  obtain ⟨a, rfl⟩ := conforms_address hva
  rw [hexec]
  by_cases hl : s.activeCount sid.module > 1
  · simp [Step.existsResource, hglob, popAddress_of hst, Step.unlocked, hl, bind,
      Except.bind, Step.fail]
    trivial
  · simp only [Step.existsResource, hglob, popAddress_of hst, Step.unlocked, hl,
      if_false, bind, Except.bind, pure, Except.pure]
    exact fallThrough_safe (after := .bool :: brest) hc hact hchain hi hctl hav hbefore
      hafter rfl rfl rfl (by simp [Step.advance, Step.push, hvs])

/-- **`Exists`** keeps the type and availability invariant. -/
theorem exists_safe (hc : Certified P tc ac) {N : NativeRel}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    {d : StructDefinitionIndex} (hi : code.code[fr.pc]? = some (.exists_ d)) :
    ExecSafe P N (Inv P tc ac) (Step.exec P s fr rest info code (.exists_ d)) :=
  exists_safeAt (d := d) (targs := []) hc hact hchain hi rfl (fun _ => rfl)
    (fun _ _ hafter => by
      obtain ⟨sid, brest, hds, hkey, h1, h2⟩ := typeAfter_exists hafter
      exact ⟨sid, brest, globalType_of_key hds hkey, h1, h2⟩) rfl

/-- A resource in storage. -/
theorem get?_mem {st : GlobalStorage} {a : Address} {sid : StructId} {v : Value}
    (h : st.get? a sid = some v) : ∃ r ∈ st, r.type = sid ∧ r.value = v := by
  unfold GlobalStorage.get? at h
  simp only [Option.map_eq_some_iff] at h
  obtain ⟨r, hr, rfl⟩ := h
  have hmem := List.mem_of_find?_eq_some hr
  have hp := List.find?_some hr
  simp only [Bool.and_eq_true, beq_iff_eq] at hp
  exact ⟨r, hmem, hp.2, rfl⟩

/-- **A move from storage** of definition `d` at type arguments `targs`,
`MoveFrom` or its generic form, keeps the type and availability invariant,
from well-typed storage. -/
theorem moveFrom_safeAt (hc : Certified P tc ac) {N : NativeRel}
    (hstor : ∀ r ∈ s.storage, r.value.conforms P (.struct r.type) = true)
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    {instr : Bytecode} {d : StructDefinitionIndex} {targs : List RuntimeType}
    (hi : code.code[fr.pc]? = some instr) (hctl : instr.control = .fallThrough)
    (hav : ∀ a, availabilityAfter instr a = a)
    (htyp : ∀ before after, typeAfter P fr.function info.module instr before = some after →
      ∃ sid brest, Step.globalType P fr.function.module d targs = .ok sid ∧
        before = .address :: brest ∧ after = .struct sid :: brest)
    (hexec : Step.exec P s fr rest info code instr = Step.moveResourceFrom P s fr rest d targs) :
    ExecSafe P N (Inv P tc ac) (Step.exec P s fr rest info code instr) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  obtain ⟨tys, htc, hconf⟩ := hact.types
  rw [hbefore, Option.some.injEq] at htc
  subst htc
  obtain ⟨sid, brest, hglob, rfl, rfl⟩ := htyp _ _ hafter
  obtain ⟨va, vs, hst, hva, hvs⟩ := conformsAll_cons_right hconf
  obtain ⟨a, rfl⟩ := conforms_address hva
  rw [hexec]
  by_cases hl : s.activeCount sid.module > 1
  · simp [Step.moveResourceFrom, hglob, popAddress_of hst, Step.unlocked, hl, bind,
      Except.bind, Step.fail]
    trivial
  · cases hg : s.storage.get? a sid with
    | none =>
      simp [Step.moveResourceFrom, hglob, popAddress_of hst, Step.unlocked, hl, hg, bind,
        Except.bind, pure, Except.pure, Step.fail]
      trivial
    | some v =>
      obtain ⟨r, hr, hrt, hrv⟩ := get?_mem hg
      have hv : v.conforms P (.struct sid) = true := by rw [← hrv, ← hrt]; exact hstor r hr
      simp only [Step.moveResourceFrom, hglob, popAddress_of hst, Step.unlocked, hl,
        if_false, bind, Except.bind, pure, Except.pure, hg, Step.finish, State.invalidate]
      exact fallThrough_safe (s := { s with
          storage := s.storage.remove a sid
          invalidations := .removed (.global a sid) :: s.invalidations })
        (after := .struct sid :: brest) hc hact hchain hi hctl hav hbefore
        hafter rfl rfl rfl (by simp [Step.advance, Step.pushAll, hvs, hv])

/-- **`MoveFrom`** keeps the type and availability invariant, from well-typed
storage. -/
theorem moveFrom_safe (hc : Certified P tc ac) {N : NativeRel}
    (hstor : ∀ r ∈ s.storage, r.value.conforms P (.struct r.type) = true)
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    {d : StructDefinitionIndex} (hi : code.code[fr.pc]? = some (.moveFrom d)) :
    ExecSafe P N (Inv P tc ac) (Step.exec P s fr rest info code (.moveFrom d)) :=
  moveFrom_safeAt (d := d) (targs := []) hc hstor hact hchain hi rfl (fun _ => rfl)
    (fun _ _ hafter => by
      obtain ⟨sid, brest, hds, hkey, h1, h2⟩ := typeAfter_moveFrom hafter
      exact ⟨sid, brest, globalType_of_key hds hkey, h1, h2⟩) rfl

/-! ## `MoveTo` -/

theorem insert_mem {st : GlobalStorage} {a : Address} {sid : StructId} {v : Value} {r : Resource}
    (h : r ∈ st.insert a sid v) : r ∈ st ∨ r = ⟨a, sid, v⟩ := by
  unfold GlobalStorage.insert at h
  simpa using h

/-- **A move to storage** of definition `d` at type arguments `targs`,
`MoveTo` or its generic form. -/
theorem refSafe_moveToAt (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {instr : Bytecode}
    {d : StructDefinitionIndex} {targs : List RuntimeType}
    (hctx : TopCtx P s fr info code instr view A stys tys ps)
    (hi : code.code[fr.pc]? = some instr) (hret : instr ≠ .ret)
    (hctl : instr.control = .fallThrough) (hav : ∀ a, availabilityAfter instr a = a)
    (hrun : ∀ A', (ReferenceSafety.executeInner info.module view instr).run A = .ok ((), A') →
      (ReferenceSafety.executeInner info.module view (.moveTo d)).run A = .ok ((), A'))
    (htyp : ∀ before after, typeAfter P fr.function info.module instr before = some after →
      ∃ sid brest, Step.globalType P fr.function.module d targs = .ok sid ∧
        ((RuntimeType.struct sid).abilities P).key = true ∧
        before = .struct sid :: .ref false .signer :: brest ∧ after = brest)
    (hexec : Step.exec P s fr rest info code instr = Step.moveResourceTo P s fr rest d targs) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code instr) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  obtain ⟨sid, brest, hglob, hkey, rfl, rfl⟩ := htyp _ _ hafter
  obtain ⟨v, st1, hst1, hv, h1⟩ := conformsAll_cons_right hctx.conf
  obtain ⟨rv, vs, hst2, hrv, hvs⟩ := conformsAll_cons_right h1
  subst hst2
  obtain ⟨r, rfl, hrm, hrt⟩ := conforms_ref hrv (by simp [rigid])
  have hvkey : (v.abilities P).key = true :=
    AbilitySet.key_of_subset (abilities_of_conforms hc.abilities hv) hkey
  obtain ⟨A', hx, hsucc⟩ := hctx.next hret
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  obtain ⟨va, ra, hsa, hcase⟩ := exec_moveTo (hrun _ hx)
  have hlen : ra.length = vs.length := by
    have := hctx.height; rw [hsa, hst1] at this; simpa using this
  rcases hcase with ⟨rfl, -⟩ | ⟨id, g, rfl, hrel, rfl⟩
  · exact absurd rfl (notRef_of_abs hctx.typed hctx.targs hctx.sig hctx.conf (k := 1) (by rw [hsa]; rfl)
      (by rw [hst1]; rfl) r)
  obtain ⟨t0, stys0, hstys⟩ : ∃ t0 stys0, stys = t0 :: stys0 := by
    have := hctx.typed.nstack
    rw [hsa] at this
    cases stys with
    | nil => simp at this
    | cons t0 stys0 => exact ⟨t0, stys0, rfl⟩
  have ht1 := hctx.typed.popValue hsa hstys
  have hd1 := hd.pop (A' := { A with stack := .reference id :: ra })
    (fr' := { fr with stack := .ref r :: vs }) hsa hst1 (by simp [hlen]) rfl rfl rfl rfl rfl
  obtain ⟨r', hr', hL⟩ := hd1.top rfl rfl hlen
  simp only [Value.ref.injEq] at hr'
  subst hr'
  have hp : id ∈ heldIds A := mem_heldIds.2 (Or.inr (by
    rw [hsa]; exact List.mem_cons_of_mem _ List.mem_cons_self))
  have hok₁ : RefOk P { s with frames := { fr with stack := vs } :: rest }
      { fr with stack := vs } r := (hrefs id r hp hL).same hsf rfl rfl rfl rfl rfl rfl rfl
  obtain ⟨w, hw⟩ := hok₁.deref_some
  have hwc : w.conforms P .signer = true := by
    rw [← hrt]
    exact hok₁.deref_conforms (hctx.roots.replaceTop hsf rfl rfl rfl) hw
  obtain ⟨a, rfl⟩ := conforms_signer hwc
  have hpop : Step.popRefTo { fr with stack := .ref r :: vs } .signer false =
      .ok (r, { fr with stack := vs }) := by
    simp [Step.popRefTo, Step.popRef, Step.pop, bind, Except.bind, pure, Except.pure, hrt]
  have hpre : Step.exec P s fr rest info code instr =
      (Step.unlocked { s with frames := { fr with stack := vs } :: rest } sid >>= fun _ =>
        if (s.storage.get? a sid).isSome then Step.fail .resourceExists
        else Step.finish { s with
          frames := { fr with stack := vs } :: rest
          storage := s.storage.insert a sid v } []) := by
    rw [hexec]
    simp only [Step.moveResourceTo, hglob, Step.pop, hst1, bind, Except.bind,
      pure, Except.pure, hpop, hrm, Bool.false_eq_true, if_false, hv, hvkey, Bool.not_true,
      Step.deref, Step.need, hw]
  rw [hpre]
  have hact1 : State.activeCount { s with frames := { fr with stack := vs } :: rest } sid.module =
      s.activeCount sid.module := rfl
  by_cases hl : s.activeCount sid.module > 1
  · simp only [Step.unlocked, hact1, hl, if_true, bind, Except.bind, Step.fail]
    trivial
  · simp only [Step.unlocked, hact1, hl, if_false, bind, Except.bind, pure, Except.pure]
    cases hg : (s.storage.get? a sid).isSome
    · simp only [Bool.false_eq_true, if_false, Step.finish]
      obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, hctl])
      have hd₀ := hd1.popRelease ht1 rfl rfl hlen hrel (A' := { A with stack := ra, graph := g })
        (fr' := Step.advance (Step.pushAll { fr with stack := vs } [])) rfl rfl rfl
        (by simp [Step.advance, Step.pushAll])
      refine ⟨?_, ?_⟩
      · exact fallThrough_safe (N := N) (s := { s with storage := s.storage.insert a sid v })
          hc hact hchain hi hctl hav hbefore hafter rfl rfl rfl
          (by simpa [Step.advance, Step.pushAll] using hvs)
      · refine refInv_finish hr hsf (hctx.chain _ hsf) rfl rfl rfl rfl rfl rfl (Nat.le_refl _) (fun r' hr' => ?_) (es := []) rfl
          (storageKept_insert rfl) allowedBy_nil hd₀ (fun x r₂ hx hxr => ?_) hA'' hle
          (RefsFrom.next (new := []) (n := 2) (by simp [Step.advance, Step.pushAll, hst1]) rfl
            (by simp))
        · rcases insert_mem hr' with hr' | rfl
          · exact hr.storage r' hr'
          · exact hv
        · have hsub : ∀ y, AbsValue.reference y ∈ ra → AbsValue.reference y ∈ A.stack :=
            fun y hy => by rw [hsa]; exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hy)
          exact (hrefs x r₂ (heldIds_sub (A := A) (A' := { A with stack := ra, graph := g }) rfl
            hsub x hx) hxr).sameKept hsf rfl rfl rfl rfl rfl rfl (storageKept_insert rfl)
    · simp only [if_true, Step.fail]
      trivial

/-- **`MoveTo`.** -/
theorem refSafe_moveTo (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {d : StructDefinitionIndex}
    (hctx : TopCtx P s fr info code (.moveTo d) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.moveTo d)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (.moveTo d)) :=
  refSafe_moveToAt (d := d) (targs := []) hc hr hsf hact hchain hctx hi (by intro h; cases h) rfl
    (fun _ => rfl) (fun _ hx => hx)
    (fun _ _ hafter => by
      obtain ⟨sid, brest, hds, hkey, h1, h2⟩ := typeAfter_moveTo hafter
      exact ⟨sid, brest, globalType_of_key hds hkey, hkey, h1, h2⟩) rfl

end MoveBytecode.Proofs
