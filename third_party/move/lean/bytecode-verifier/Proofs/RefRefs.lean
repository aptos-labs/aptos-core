-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefLocals

/-!
# The instructions on references, safe

UNTRUSTED (checked by the kernel). Each instruction that reads, writes or
borrows through a reference, in a state satisfying the reference invariant,
does not
go wrong, and leads to a state satisfying it: the reference it uses is
tracked, so valid, and the analysis's checks keep every other tracked
reference valid.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit} {view : FunctionView} {A : RefState} {stys : List SignatureToken}
  {tys : List RuntimeType} {ps : List Reference}

/-- Every frame's locals conform after the current frame is replaced by one
with the same function and locals. -/
theorem RootsOk.replaceTop (h : RootsOk P s) (hsf : s.frames = fr :: rest) {fr' : Frame}
    (hs'f : s'.frames = fr' :: rest) (hfun : fr'.function = fr.function)
    (hloc : fr'.locals = fr.locals) : RootsOk P s' := by
  intro g hg
  rw [hs'f] at hg
  rcases List.mem_cons.1 hg with rfl | hg
  · obtain ⟨tys, h1, h2, h3⟩ := h fr (by rw [hsf]; exact List.mem_cons_self)
    exact ⟨tys, by rw [hfun]; exact h1, by rw [hloc]; exact h2, by rw [hloc]; exact h3⟩
  · exact h g (by rw [hsf]; exact List.mem_cons_of_mem _ hg)

/-- A type a reference type is assignable to is a reference type of its
mutability. -/
theorem assignable_from_ref {m : Bool} {t T : RuntimeType}
    (h : (RuntimeType.ref m t).assignable T = true) : ∃ u, T = .ref m u := by
  rcases assignable_cases h with ⟨ps, rs, x, x', h1, -⟩ | ⟨v, w, h1, rfl, -⟩ | rfl
  · cases h1
  · cases h1; exact ⟨w, rfl⟩
  · exact ⟨t, rfl⟩

/-- **A valid tracked reference designates a value of its target type.** -/
theorem RefOk.deref_conforms {r : Reference} {frx : Frame} (h : RefOk P s frx r)
    (hroots : RootsOk P s) {w : Value} (hd : s.deref? r.location r.since = some w) :
    w.conforms P r.target = true := by
  obtain ⟨⟨T, T', hT, hpath, hasg⟩, -, -, -, -⟩ := h
  have hread := deref?_read hd
  refine conforms_trans (t := T') ?_ hasg.assignable
  cases hρ : r.location.root with
  | local_ f i =>
    rw [hρ] at hT
    simp only [RootType, Option.bind_eq_some_iff] at hT
    obtain ⟨g, hg, hT⟩ := hT
    have hl : r.location = ⟨.local_ f i, r.location.path⟩ := by
      cases hl : r.location; simp_all
    rw [hl, read?_local hg] at hread
    cases hv : g.locals[i]? with
    | none => rw [hv] at hread; cases hread
    | some o =>
      cases o with
      | none => rw [hv] at hread; cases hread
      | some rv =>
        rw [hv] at hread
        simp only [Option.bind_some, id] at hread
        obtain ⟨tys', htys', -, hconf⟩ := hroots g (frame?_mem hg).1
        obtain ⟨T', hT', hrvT⟩ := hconf i rv hv
        rw [htys'] at hT
        obtain ⟨a, ha, haT⟩ := hT
        simp only [Option.some.injEq] at ha
        subst ha
        rw [hT', Option.some.injEq] at haT
        subst haT
        exact get_conforms hrvT hread hpath
  | held k =>
    rw [hρ] at hT
    simp only [RootType, Option.bind_eq_some_iff] at hT
    obtain ⟨v, hv, hT⟩ := hT
    split at hT
    · next hwf =>
      simp only [Option.some.injEq] at hT
      subst hT
      have hl : r.location = ⟨.held k, r.location.path⟩ := by
        cases hl : r.location; simp_all
      rw [hl, read?_held, hv, Option.bind_some] at hread
      exact get_conforms (conforms_iff.2 ⟨assignable_refl _, hwf⟩) hread hpath
    · cases hT
  | global a t =>
    rw [hρ] at hT
    simp only [RootType, Option.bind_eq_some_iff] at hT
    obtain ⟨v, hv, hT⟩ := hT
    split at hT
    · next hwf =>
      simp only [Option.some.injEq] at hT
      subst hT
      have hl : r.location = ⟨.global a t, r.location.path⟩ := by
        cases hl : r.location; simp_all
      rw [hl] at hread
      simp only [State.read?, State.root?, hv, bind, Option.bind_some] at hread
      exact get_conforms (conforms_iff.2 ⟨assignable_refl _, hwf⟩) hread hpath
    · cases hT

/-- A tracked reference is valid. -/
theorem RefOk.deref_some {r : Reference} (h : RefOk P s fr r) :
    ∃ w, s.deref? r.location r.since = some w :=
  Option.isSome_iff_exists.1 h.valid

/-- The type certificate's types before the current instruction. -/
theorem TopCtx.before {instr : Bytecode} {view : FunctionView} {A : RefState}
    (hctx : TopCtx P s fr info code instr view A stys tys ps) {before : List RuntimeType}
    (h : modelTypes P fr.function fr.pc = some before) : before = tys := by
  rw [hctx.tc, Option.some.injEq] at h
  exact h.symm

/-- A value conforming to a reference type is a reference of that mutability,
whose target is assignable to the type's target. -/
theorem conforms_ref_assignable {v : Value} {m : Bool} {t : RuntimeType}
    (h : v.conforms P (.ref m t) = true) :
    ∃ r, v = .ref r ∧ r.mutable = m ∧ r.target.assignable t = true := by
  rcases assignable_cases (conforms_assignable h) with
    ⟨_, _, _, _, -, hu, -⟩ | ⟨a, b, he, hu, hab⟩ | he
  · cases hu
  · simp only [RuntimeType.ref.injEq] at hu
    obtain ⟨rfl, rfl⟩ := hu
    cases v <;> simp [Value.type] at he
    rename_i r
    exact ⟨r, rfl, he.1, by rw [he.2]; exact hab⟩
  · cases v <;> simp [Value.type] at he
    rename_i r
    exact ⟨r, rfl, he.1, by rw [he.2]; exact assignable_refl _⟩

/-- A value conforming to a mutable reference type, or to a reference type
with a rigid target, is a reference of exactly that type. -/
theorem conforms_ref {v : Value} {m : Bool} {t : RuntimeType}
    (h : v.conforms P (.ref m t) = true) (ht : m = true ∨ rigid t = true) :
    ∃ r, v = .ref r ∧ r.mutable = m ∧ r.target = t := by
  rcases assignable_cases (conforms_assignable h) with
    ⟨_, _, _, _, -, hu, -⟩ | ⟨a, b, he, hu, hab⟩ | he
  · cases hu
  · simp only [RuntimeType.ref.injEq] at hu
    obtain ⟨rfl, rfl⟩ := hu
    rcases ht with ht | ht
    · cases ht
    · cases v <;> simp [Value.type] at he
      rename_i r
      exact ⟨r, rfl, he.1, by rw [he.2]; exact assignable_rigid hab ht⟩
  · cases v <;> simp [Value.type] at he
    rename_i r
    exact ⟨r, rfl, he.1, he.2⟩

/-- The current frame with its operand stack replaced. -/
theorem frame?_withStack {s : State} {fr : Frame} {rest : List Frame} (st : List Value) :
    ({ s with frames := { fr with stack := st } :: rest } : State).frame? fr.id =
      some { fr with stack := st } := by
  simp [State.frame?]

theorem localsOk_withStack {fr : Frame} (h : LocalsOk P fr) (st : List Value) :
    LocalsOk P { fr with stack := st } := h

/-- **`ReadRef`.** -/
theorem refSafe_readRef (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest)
    (hctx : TopCtx P s fr info code .readRef view A stys tys ps)
    (hi : code.code[fr.pc]? = some .readRef) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code .readRef) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc .readRef hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  dsimp only [typeAfter] at hafter
  split at hafter
  · next m t brest =>
    split at hafter
    · next hcopy =>
      simp only [Option.some.injEq] at hafter
      subst hafter
      obtain ⟨v, st, hst, hv, hstc⟩ := conformsAll_cons_right hctx.conf
      obtain ⟨r, rfl, hrm, hrt⟩ := conforms_ref_assignable hv
      obtain ⟨A', hx, hsucc⟩ := hctx.next (by decide)
      obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
      obtain ⟨id, ra, g, hsa, -, hrel, rfl⟩ := exec_readRef hx
      have hlen : ra.length = st.length := by
        have := hctx.height; rw [hsa, hst] at this; simpa using this
      obtain ⟨r', hr', hL⟩ := hd.top hsa hst hlen
      simp only [Value.ref.injEq] at hr'
      subst hr'
      have hid : id ∈ heldIds A := mem_heldIds.2 (Or.inr (by rw [hsa]; exact List.mem_cons_self))
      let fr₁ : Frame := { fr with stack := st }
      let s₁ : State := { s with frames := fr₁ :: rest }
      have hok₁ : RefOk P s₁ fr₁ r := (hrefs id r hid hL).same hsf rfl rfl rfl rfl rfl rfl rfl
      obtain ⟨w, hw⟩ := hok₁.deref_some
      have hwc : w.conforms P t = true :=
        conforms_trans (hok₁.deref_conforms (hctx.roots.replaceTop hsf rfl rfl rfl) hw) hrt
      have hwcopy : (w.abilities P).copy = true :=
        AbilitySet.copy_of_subset (abilities_of_conforms hc.abilities hwc) hcopy
      have hstep : Step.exec P s fr rest info code .readRef =
          .ok (.done (.running { s₁ with frames := Step.advance (Step.pushAll fr₁ [w]) :: rest })) := by
        simp only [Step.exec, Step.popRef, Step.pop, hst, bind, Except.bind, pure, Except.pure,
          Step.deref, Step.need]
        have : s₁.deref? r.location r.since = some w := hw
        simp only [s₁, fr₁] at this
        simp only [this, hwcopy, Bool.not_true, Bool.false_eq_true, if_false, Step.finish]
        rfl
      rw [hstep]
      refine ⟨?_, ?_⟩
      · exact fallThrough_safe (N := N) (s := s₁) (after := t :: brest) hc hact hchain hi rfl
          (fun _ => rfl) hbefore (by dsimp only [typeAfter]; simp [hcopy]) rfl rfl rfl
          (by simp [Step.advance, Step.pushAll, fr₁, hwc, hstc])
      · obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, Bytecode.control])
        have hd₁ := hd.popRelease hctx.typed hsa hst hlen hrel rfl (fr' := fr₁) rfl rfl rfl
        have hd₂ := hd₁.pushVal (v := w) (fr' := Step.advance (Step.pushAll fr₁ [w])) rfl rfl rfl
          (by simp [Step.advance, Step.pushAll, fr₁])
        refine refInv_finish hr hsf (hctx.chain _ hsf) rfl rfl rfl rfl rfl rfl (Nat.le_refl _) hr.storage (es := []) rfl
          (storageKept_of_eq rfl) allowedBy_nil hd₂ (fun x r₂ hx hxr => ?_) hA'' hle
          (RefsFrom.next (new := [w]) (n := 1) (by simp [Step.advance, Step.pushAll, fr₁, hst]) rfl
            (fun v hv' => by
              simp only [List.mem_singleton] at hv'; subst hv'
              exact .of_not_ref (not_ref_of_conforms
                (hok₁.deref_conforms (hctx.roots.replaceTop hsf rfl rfl rfl) hw) hok₁.value)))
        refine (hrefs x r₂ ?_ hxr).same hsf rfl rfl rfl rfl rfl rfl rfl
        simp only [heldIds, List.filterMap_cons, AbsValue.refId?, List.mem_append] at hx ⊢
        rw [hsa]
        simp only [List.filterMap_cons, AbsValue.refId?, List.mem_cons]
        rcases hx with hx | hx
        · exact Or.inl hx
        · exact Or.inr (Or.inr hx)
    · simp at hafter
  · simp at hafter

/-- Bounds checking validates every struct variant handle. -/
theorem verifyBounds_structVariantHandle {m : CompiledModule} (hb : verifyBounds m = .ok ())
    {h : Nat} {vh : StructVariantHandle} (hvh : m.structVariantHandles[h]? = some vh) :
    ∃ sd, m.structDefs[vh.structIndex]? = some sd ∧ vh.variant < variantCount sd.fieldInformation := by
  unfold verifyBounds at hb
  simp only at hb
  split at hb
  · simp [reject, bind, Except.bind] at hb
  iterate 13 obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨hf, -⟩ := seq_ok2 hb
  have h := forM_ok hf vh (getElem?_mem_toList hvh)
  unfold Bounds.checkStructVariantHandle at h
  obtain ⟨h1, h⟩ := seq_ok2 h
  obtain ⟨sd, hsd⟩ : ∃ sd, m.structDefs[vh.structIndex]? = some sd :=
    ⟨_, Array.getElem?_eq_getElem (checkIdx_ok h1)⟩
  simp only [bind_ok, lookup_ok, hsd] at h
  obtain ⟨sd', hsd', h⟩ := h
  cases hsd'
  refine ⟨sd, hsd, ?_⟩
  unfold Bounds.checkVariantIndex at h
  split at h
  · simp [reject] at h
  · next hlt => exact Nat.lt_of_not_le hlt

/-- A value of an enum type is a variant. -/
theorem conforms_enum {v : Value} {s : StructId} {info : StructInfo}
    {variants : List VariantDefinition} (hv : v.conforms P (.struct s) = true)
    (hinfo : P.structType? s = some info)
    (hfi : info.definition.fieldInformation = .declaredVariants variants) :
    ∃ tag fields, v = .variant s tag fields := by
  have hty := conforms_type hv rfl
  have hw := conforms_wellFormed hv
  cases v with
  | variant s' tag fields =>
    simp only [Value.type, RuntimeType.struct.injEq] at hty
    exact ⟨tag, fields, by rw [hty]⟩
  | struct s' fields =>
    simp only [Value.type, RuntimeType.struct.injEq] at hty
    subst hty
    obtain ⟨-, tys, hft, -⟩ := wellFormed_struct.1 hw
    simp [Program.fieldTypes, hinfo, hfi] at hft
  | table s' fields ks es =>
    simp only [Value.type, RuntimeType.struct.injEq] at hty
    subst hty
    obtain ⟨-, -, ⟨tys, hft, -⟩, -⟩ := wellFormed_table.1 hw
    simp [Program.fieldTypes, hinfo, hfi] at hft
  | _ => simp [Value.type] at hty

/-- The common end of the instructions that pop a tracked reference, release
it, and push a value. -/
theorem refInv_popRefPush (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hctx : TopCtx P s fr info code instr view A stys tys ps) {id : Nat} {ra : List AbsValue}
    {g : Graph} {r : Reference} {st : List Value} (hsa : A.stack = .reference id :: ra)
    (hst : fr.stack = .ref r :: st) (hlen : ra.length = st.length)
    (hrel : A.graph.release id = .ok g) {w : Value} {A'' : RefState}
    (hA'' : modelRef P fr.function (fr.pc + 1) = some A'')
    (hle : RefLe { A with stack := .nonReference :: ra, graph := g } A'')
    (hw : ∀ r, w ≠ .ref r := by intro r h; cases h) :
    RefInvariant P { s with frames := Step.advance (Step.pushAll { fr with stack := st } [w]) :: rest } := by
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  have hd₁ := hd.popRelease hctx.typed hsa hst hlen hrel rfl (fr' := { fr with stack := st })
    rfl rfl rfl
  have hd₂ := hd₁.pushVal (v := w) (fr' := Step.advance (Step.pushAll { fr with stack := st } [w]))
    rfl rfl rfl (by simp [Step.advance, Step.pushAll])
  refine refInv_finish hr hsf (hctx.chain _ hsf) rfl rfl rfl rfl rfl rfl (Nat.le_refl _) hr.storage (es := []) rfl
    (storageKept_of_eq rfl) allowedBy_nil hd₂ (fun x r₂ hx hxr => ?_) hA'' hle
    (RefsFrom.next (new := [w]) (n := 1) (by simp [Step.advance, Step.pushAll, hst]) rfl
      (fun v hv' => by simp only [List.mem_singleton] at hv'; subst hv'; exact .of_not_ref hw))
  refine (hrefs x r₂ ?_ hxr).same hsf rfl rfl rfl rfl rfl rfl rfl
  simp only [heldIds, List.filterMap_cons, AbsValue.refId?, List.mem_append] at hx ⊢
  rw [hsa]
  simp only [List.filterMap_cons, AbsValue.refId?, List.mem_cons]
  rcases hx with hx | hx
  · exact Or.inl hx
  · exact Or.inr (Or.inr hx)

/-- The tracked reference on top of the operand stack, valid in the state
with it popped. -/
theorem top_ref_valid (hsf : s.frames = fr :: rest)
    (hctx : TopCtx P s fr info code instr view A stys tys ps) {id : Nat}
    {ra : List AbsValue} {r : Reference} {st : List Value} (hsa : A.stack = .reference id :: ra)
    (hst : fr.stack = .ref r :: st) :
    ∃ w, ({ s with frames := { fr with stack := st } :: rest } : State).deref? r.location r.since =
      some w ∧ w.conforms P r.target = true := by
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  have hlen : ra.length = st.length := by
    have := hctx.height; rw [hsa, hst] at this; simpa using this
  obtain ⟨r', hr', hL⟩ := hd.top hsa hst hlen
  simp only [Value.ref.injEq] at hr'
  subst hr'
  have hid : id ∈ heldIds A := mem_heldIds.2 (Or.inr (by rw [hsa]; exact List.mem_cons_self))
  have hok₁ : RefOk P { s with frames := { fr with stack := st } :: rest } { fr with stack := st } r :=
    (hrefs id r hid hL).same hsf rfl rfl rfl rfl rfl rfl rfl
  obtain ⟨w, hw⟩ := hok₁.deref_some
  exact ⟨w, hw, hok₁.deref_conforms (hctx.roots.replaceTop hsf rfl rfl rfl) hw⟩

/-- **A variant test** of handle `h` at type arguments `targs`: `TestVariant`
or its generic form, given the analysis runs it as the non-generic form and
the type certificate types it as a test of the instance. -/
theorem refSafe_testVariantAt (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {instr : Bytecode}
    {h : Nat} {targs : List RuntimeType}
    (hctx : TopCtx P s fr info code instr view A stys tys ps)
    (hi : code.code[fr.pc]? = some instr) (hret : instr ≠ .ret)
    (hctl : instr.control = .fallThrough) (hav : ∀ a, availabilityAfter instr a = a)
    (hrun : ∀ A', (ReferenceSafety.executeInner info.module view instr).run A = .ok ((), A') →
      (ReferenceSafety.executeInner info.module view (.testVariant h)).run A = .ok ((), A'))
    (htyp : ∀ before after, typeAfter P fr.function info.module instr before = some after →
      ∃ vh sid sinfo variants m0 brest, info.module.structVariantHandles[h]? = some vh ∧
        P.structType? sid = some sinfo ∧
        Step.localStruct P fr.function.module vh.structIndex targs = .ok (sid, sinfo) ∧
        sinfo.definition.fieldInformation = .declaredVariants variants ∧
        vh.variant < variants.length ∧
        before = .ref m0 (.struct sid) :: brest ∧ after = .bool :: brest)
    (hexec : Step.exec P s fr rest info code instr = Step.testEnum P s fr rest info h targs) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code instr) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  obtain ⟨vh, sid, sinfo, variants, m0, brest, hvh, hst₂, hloc, hfi, hv, rfl, rfl⟩ :=
    htyp _ _ hafter
  obtain ⟨v, st, hst, hv', hstc⟩ := conformsAll_cons_right hctx.conf
  obtain ⟨r, rfl, -, hrt⟩ := conforms_ref hv' (by simp [rigid])
  obtain ⟨A', hx, hsucc⟩ := hctx.next hret
  obtain ⟨id, ra, g, hsa', hrel, rfl⟩ := exec_testVariant (hrun _ hx)
  obtain ⟨w, hw, hwc⟩ := top_ref_valid hsf hctx hsa' hst
  rw [hrt] at hwc
  obtain ⟨tag, fields, rfl⟩ := conforms_enum (variants := variants) hwc hst₂ hfi
  have hstep : Step.exec P s fr rest info code instr =
      .ok (.done (.running { s with frames := Step.advance (Step.pushAll { fr with stack := st }
        [.bool (tag == vh.variant)]) :: rest })) := by
    rw [hexec]
    simp only [Step.testEnum, Step.need, hvh, hloc, hfi, bind, Except.bind,
      pure, Except.pure, List.getElem?_eq_getElem hv, Step.popRefTo, Step.popRef, Step.pop,
      hst, hrt, bne_self_eq_false, Bool.false_and, Bool.false_eq_true, if_false, Step.deref,
      hw, Step.finish]
  rw [hstep]
  have hlen : ra.length = st.length := by
    have := hctx.height; rw [hsa', hst] at this; simpa using this
  obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, hctl])
  refine ⟨?_, refInv_popRefPush hr hsf hctx hsa' hst hlen hrel hA'' hle⟩
  exact fallThrough_safe (N := N) (after := .bool :: brest) hc hact hchain hi hctl
    hav hbefore hafter rfl rfl rfl
    (by simp [Step.advance, Step.pushAll, hstc, Value.conforms, Value.type, Value.wellFormed])

/-- **`TestVariant`.** -/
theorem refSafe_testVariant (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hsa : StructsAgree P fr.function.module info.module) (hb : verifyBounds info.module = .ok ())
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {h : Nat}
    (hctx : TopCtx P s fr info code (.testVariant h) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.testVariant h)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (.testVariant h)) := by
  refine refSafe_testVariantAt (targs := []) hc hr hsf hact hchain hctx hi (by intro h; cases h)
    rfl (fun _ => rfl) (fun _ hx => hx) (fun before after hafter => ?_) rfl
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨vh, hvh, sid, hdef, hta⟩ := hta
  split at hta
  · next m0 s' brest =>
    split at hta
    · next hss =>
      subst hss
      simp only [Option.some.injEq] at hta
      subst hta
      obtain ⟨sd, hsd, hvar⟩ := verifyBounds_structVariantHandle hb hvh
      obtain ⟨s₂, id₂, sh, hdef₂, -, -, hst₂⟩ := hsa.defs _ sd hsd
      rw [hdef, Option.some.injEq] at hdef₂
      subst hdef₂
      obtain ⟨variants, hfi, hv⟩ : ∃ variants, sd.fieldInformation = .declaredVariants variants ∧
          vh.variant < variants.length := by
        cases hfi : sd.fieldInformation with
        | declaredVariants vs => rw [hfi] at hvar; exact ⟨vs, rfl, hvar⟩
        | declared _ => rw [hfi] at hvar; simp [variantCount] at hvar
        | native => rw [hfi] at hvar; simp [variantCount] at hvar
      exact ⟨vh, _, _, variants, m0, brest, hvh, hst₂, localStruct_ok hdef hst₂, hfi, hv, rfl,
        rfl⟩
    · simp at hta
  · simp at hta

/-- **`VecLen`.** -/
theorem refSafe_vecLen (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {sig : Nat}
    (hctx : TopCtx P s fr info code (.vecLen sig) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.vecLen sig)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code (.vecLen sig)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨t, het, hta⟩ := hta
  split at hta
  · next m0 t' brest =>
    split at hta
    · next htt =>
      subst htt
      simp only [Option.some.injEq] at hta
      subst hta
      obtain ⟨v, st, hst, hv', hstc⟩ := conformsAll_cons_right hctx.conf
      obtain ⟨r, rfl, -, hrt⟩ := conforms_ref hv' (by simp [rigid])
      obtain ⟨A', hx, hsucc⟩ := hctx.next (by intro h; cases h)
      obtain ⟨id, ra, g, hsa', hrel, rfl⟩ := exec_vecLen hx
      obtain ⟨w, hw, hwc⟩ := top_ref_valid hsf hctx hsa' hst
      rw [hrt] at hwc
      obtain ⟨elems, rfl, -⟩ := conforms_vector hwc
      have hlen : ra.length = st.length := by
        have := hctx.height; rw [hsa', hst] at this; simpa using this
      obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, Bytecode.control])
      by_cases hn : IntType.u64.inRange elems.length = true
      · have hstep : Step.exec P s fr rest info code (.vecLen sig) =
            .ok (.done (.running { s with frames := Step.advance (Step.pushAll
              { fr with stack := st } [.int .u64 elems.length]) :: rest })) := by
          simp only [Step.exec, elemType_ok het, Step.popRefTo, Step.popRef, Step.pop, hst, hrt,
            bind, Except.bind, pure, Except.pure, bne_self_eq_false, Bool.false_and,
            Bool.false_eq_true, if_false, Step.derefVector, Step.deref, Step.need, hw, hn,
            if_true, Step.finish]
        rw [hstep]
        refine ⟨?_, refInv_popRefPush hr hsf hctx hsa' hst hlen hrel hA'' hle⟩
        exact fallThrough_safe (N := N) (after := .int .u64 :: brest) hc hact hchain hi rfl
          (fun _ => rfl) hbefore hafter rfl rfl rfl
          (by simp [Step.advance, Step.pushAll, hstc, hn])
      · have hstep : Step.exec P s fr rest info code (.vecLen sig) = .error (.failed .arithmetic) := by
          simp only [Step.exec, elemType_ok het, Step.popRefTo, Step.popRef, Step.pop, hst, hrt,
            bind, Except.bind, pure, Except.pure, bne_self_eq_false, Bool.false_and,
            Bool.false_eq_true, if_false, Step.derefVector, Step.deref, Step.need, hw, hn,
            Step.fail]
        rw [hstep]
        trivial
    · simp at hta
  · simp at hta

/-- **Replacing the reference on top** by a new reference `c` derived from it,
the old one released. -/
theorem Desc.replaceTop {gsid : Nat → Option StructId} {ps : List Reference}
    {L : Nat → Option Reference} {ltys stys' : List SignatureToken}
    {avail : LocalIndex → Availability} {A' : RefState} {fr' : Frame}
    (hd : Desc A fr gsid ps L) (ht : Typed ltys stys' avail A) {p c : Nat} {ra : List AbsValue}
    {rp r : Reference} {st : List Value} {g3 : Graph} (hsa : A.stack = .reference p :: ra)
    (hst : fr.stack = .ref rp :: st) (hlen : ra.length = st.length)
    (hfresh : A.graph.find? c = none)
    (hc3 : Complete g3 (updL L c r) (fun x => (x ∈ heldIds A ∨ x = c) ∧ x ≠ p) A.locals.length
      fr.id gsid ps)
    (hA' : A'.locals = A.locals ∧ A'.graph = g3 ∧ A'.stack = .reference c :: ra)
    (hid : fr'.id = fr.id) (hloc : fr'.locals = fr.locals) (hs' : fr'.stack = .ref r :: st) :
    Desc A' fr' gsid ps (updL L c r) := by
  obtain ⟨hl, hg, hs⟩ := hA'
  have hcH : c ∉ heldIds A := fun h => by
    have := ht.held_key h; rw [hfresh] at this; cases this
  have hne : ∀ y, y ∈ heldIds A → y ≠ c := fun y hy e => hcH (e ▸ hy)
  have hheld : heldIds A = A.locals.filterMap AbsValue.refId? ++
      p :: ra.filterMap AbsValue.refId? := by simp [heldIds, hsa, AbsValue.refId?]
  have hnd := ht.distinct
  rw [hheld] at hnd
  refine ⟨fun i y hi => ?_, fun q y hq => ?_, ?_⟩
  · rw [hl] at hi
    obtain ⟨r', hr', hL⟩ := hd.locals i y hi
    have hy : y ≠ c := hne y (mem_heldIds.2 (Or.inl (List.mem_of_getElem? hi)))
    exact ⟨r', by rw [hloc]; exact hr', by rw [updL_ne _ hy]; exact hL⟩
  · rw [hs] at hq
    by_cases hq' : q < ra.length
    · rw [reverse_cons_getElem?_lt hq'] at hq
      obtain ⟨r', hr', hL⟩ := hd.stack q y (by rw [hsa, reverse_cons_getElem?_lt hq']; exact hq)
      have hy : y ≠ c := hne y (mem_heldIds.2 (Or.inr (by
        rw [hsa]; exact List.mem_cons_of_mem _ (List.mem_reverse.1 (List.mem_of_getElem? hq)))))
      refine ⟨r', ?_, by rw [updL_ne _ hy]; exact hL⟩
      rw [hst, reverse_cons_getElem?_lt (by omega)] at hr'
      rw [hs', reverse_cons_getElem?_lt (by omega)]
      exact hr'
    · have hqe : q = ra.length := by
        have := (List.getElem?_eq_some_iff.1 hq).1; simp at this; omega
      subst hqe
      rw [reverse_cons_getElem?_top, Option.some.injEq, AbsValue.reference.injEq] at hq
      subst hq
      exact ⟨r, by rw [hs', hlen, reverse_cons_getElem?_top], updL_self _ _ _⟩
  · rw [hl, hg, hid]
    refine hc3.congrH fun y => ?_
    rw [hheld]
    simp only [heldIds, hl, hs, List.filterMap_cons, AbsValue.refId?, List.mem_append,
      List.mem_cons]
    constructor
    · rintro ⟨(h | h | h) | h, hne'⟩
      · exact Or.inl h
      · exact absurd h hne'
      · exact Or.inr (Or.inr h)
      · exact Or.inr (Or.inl h)
    · rintro (h | h | h)
      · refine ⟨Or.inl (Or.inl h), fun e => ?_⟩
        subst e
        exact (List.nodup_append.1 hnd).2.2 _ h _ List.mem_cons_self rfl
      · subst h
        refine ⟨Or.inr rfl, fun e => ?_⟩
        subst e
        exact hcH (by rw [hheld]; simp)
      · refine ⟨Or.inl (Or.inr (Or.inr h)), fun e => ?_⟩
        subst e
        rw [List.nodup_append, List.nodup_cons] at hnd
        exact hnd.2.1.1 h

/-- The common end of the instructions that replace the reference on top by
one derived from it. -/
theorem refInv_replaceTop (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hctx : TopCtx P s fr info code instr view A stys tys ps) {L : Nat → Option Reference}
    (hd : Desc A fr (gsidOf P (s.frames.map Frame.skel)) ps L)
    (hrefs : ∀ x r, x ∈ heldIds A → L x = some r → RefOk P s fr r) {p c : Nat}
    {ra : List AbsValue} {rp r : Reference} {st : List Value} {g3 : Graph}
    (hsa : A.stack = .reference p :: ra) (hst : fr.stack = .ref rp :: st)
    (hfresh : A.graph.find? c = none)
    (hc3 : Complete g3 (updL L c r) (fun x => (x ∈ heldIds A ∨ x = c) ∧ x ≠ p) A.locals.length
      fr.id (gsidOf P (s.frames.map Frame.skel)) ps)
    {n : Nat} {A'' : RefState} (hA'' : modelRef P fr.function (fr.pc + 1) = some A'')
    (hle : RefLe { A with stack := .reference c :: ra, graph := g3, nextId := n } A'')
    (hnew : RefOk P { s with frames := Step.advance (Step.pushAll { fr with stack := st } [.ref r]) :: rest }
      (Step.advance (Step.pushAll { fr with stack := st } [.ref r])) r)
    (hfrom : ValFrom fr (.ref r)) :
    RefInvariant P { s with frames := Step.advance (Step.pushAll { fr with stack := st } [.ref r]) :: rest } := by
  have hlen : ra.length = st.length := by
    have := hctx.height; rw [hsa, hst] at this; simpa using this
  have hd' := hd.replaceTop hctx.typed hsa hst hlen hfresh hc3
    (A' := { A with stack := .reference c :: ra, graph := g3, nextId := n })
    (fr' := Step.advance (Step.pushAll { fr with stack := st } [.ref r])) ⟨rfl, rfl, rfl⟩ rfl rfl
    (by simp [Step.advance, Step.pushAll])
  have hcH : c ∉ heldIds A := fun h => by
    have := hctx.typed.held_key h; rw [hfresh] at this; cases this
  refine refInv_finish hr hsf (hctx.chain _ hsf) rfl rfl rfl rfl rfl rfl (Nat.le_refl _) hr.storage (es := []) rfl
    (storageKept_of_eq rfl) allowedBy_nil hd' (fun x r₂ hx hxr => ?_) hA'' hle
    (RefsFrom.next (new := [.ref r]) (n := 1) (by simp [Step.advance, Step.pushAll, hst]) rfl
      (fun v hv' => by simp only [List.mem_singleton] at hv'; subst hv'; exact hfrom))
  by_cases hxc : x = c
  · subst hxc
    rw [updL_self, Option.some.injEq] at hxr
    subst hxr
    exact hnew
  · rw [updL_ne _ hxc] at hxr
    refine (hrefs x r₂ ?_ hxr).same hsf rfl rfl rfl rfl rfl rfl rfl
    simp only [heldIds, List.filterMap_cons, AbsValue.refId?, List.mem_append, List.mem_cons]
      at hx ⊢
    rw [hsa]
    simp only [List.filterMap_cons, AbsValue.refId?, List.mem_cons]
    rcases hx with hx | hx | hx
    · exact Or.inl hx
    · exact absurd hx hxc
    · exact Or.inr (Or.inr hx)

theorem pushAll_one (fr : Frame) (v : Value) : Step.pushAll fr [v] = Step.push fr v := by
  simp [Step.pushAll, Step.push]

theorem RefOk.freeze {r : Reference} {fr' : Frame} (h : RefOk P s fr' r) :
    RefOk P s fr' { r with mutable := false } :=
  ⟨h.root, h.below, h.value, h.since, h.valid, fun h => by cases h⟩

/-- **`FreezeRef`.** -/
theorem refSafe_freezeRef (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest)
    (hctx : TopCtx P s fr info code .freezeRef view A stys tys ps)
    (hi : code.code[fr.pc]? = some .freezeRef) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code .freezeRef) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  have hta := hafter
  dsimp only [typeAfter] at hta
  split at hta
  · next t brest =>
    simp only [Option.some.injEq] at hta
    subst hta
    obtain ⟨v, st, hst, hv', hstc⟩ := conformsAll_cons_right hctx.conf
    obtain ⟨r, rfl, hrm, hrt⟩ := conforms_ref hv' (by simp [rigid])
    obtain ⟨A', hx, hsucc⟩ := hctx.next (by decide)
    obtain ⟨id, ra, g1, g2, g3, hsa, hfrz, hnew, hfac, hrel, rfl⟩ := exec_freezeRef hx
    obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
    have hlen : ra.length = st.length := by
      have := hctx.height; rw [hsa, hst] at this; simpa using this
    obtain ⟨r', hr', hL⟩ := hd.top hsa hst hlen
    simp only [Value.ref.injEq] at hr'
    subst hr'
    have hp : id ∈ heldIds A := mem_heldIds.2 (Or.inr (by rw [hsa]; exact List.mem_cons_self))
    have hc2 := hd.complete.freezePre (r := { r with mutable := false }) hp hL hrm rfl rfl hfrz
      hnew hfac
    have hc3 := hc2.release (Or.inl hp) hrel
    have hstep : Step.exec P s fr rest info code .freezeRef =
        .ok (.done (.running { s with frames := Step.advance (Step.pushAll { fr with stack := st }
          [.ref { r with mutable := false }]) :: rest })) := by
      simp only [Step.exec, Step.popRef, Step.pop, hst, bind, Except.bind, pure, Except.pure, hrm,
        Bool.not_true, Bool.false_eq_true, if_false, Step.next, pushAll_one]
    rw [hstep]
    obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, Bytecode.control])
    refine ⟨?_, refInv_replaceTop hr hsf hctx hd hrefs hsa hst (Graph.newRef_ok hnew).1 hc3
      hA'' hle (RefOk.freeze ((hrefs id r hp hL).same hsf rfl rfl rfl rfl rfl rfl rfl))
      (.of_same (r₀ := r) (mem_frameVals_stack (by rw [hst]; exact List.mem_cons_self)) rfl rfl)⟩
    exact fallThrough_safe (N := N) (after := .ref false t :: brest) hc hact hchain hi rfl
      (fun _ => rfl) hbefore hafter rfl rfl rfl
      (by simp [Step.advance, Step.pushAll, hstc, Value.conforms, Value.type, Value.wellFormed,
        hrt] <;> simp_all [Value.wellFormed, Value.conforms])
  · simp at hta

/-- **`MutBorrowLoc`, `ImmBorrowLoc`.** -/
theorem refSafe_borrowLoc (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {i : Nat} {mut_ : Bool}
    (hctx : TopCtx P s fr info code (if mut_ then .mutBorrowLoc i else .immBorrowLoc i) view A
      stys tys ps)
    (hi : code.code[fr.pc]? = some (if mut_ then .mutBorrowLoc i else .immBorrowLoc i)) :
    ExecSafe P N (InvRefs P)
      (Step.exec P s fr rest info code (if mut_ then .mutBorrowLoc i else .immBorrowLoc i)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  have hta := hafter
  have hta' : ∃ t, (∃ ltys, P.localTypes fr.function = some ltys ∧ ltys[i]? = some t) ∧
      t.isValue = true ∧ after = .ref mut_ t :: before := by
    cases mut_
    · simp only [Bool.false_eq_true, if_false] at hta
      dsimp only [typeAfter] at hta
      simp only [bind, Option.bind_eq_some_iff] at hta
      obtain ⟨t, ht, hta⟩ := hta
      split at hta
      · next hv => exact ⟨t, ht, hv, (Option.some.inj hta).symm⟩
      · simp at hta
    · simp only [if_true] at hta
      dsimp only [typeAfter] at hta
      simp only [bind, Option.bind_eq_some_iff] at hta
      obtain ⟨t, ht, hta⟩ := hta
      split at hta
      · next hv => exact ⟨t, ht, hv, (Option.some.inj hta).symm⟩
      · simp at hta
  obtain ⟨t, ⟨ltys, hltys, hti⟩, htv, rfl⟩ := hta'
  obtain ⟨v, hv, hvt⟩ := read_local hc hact hi (by cases mut_ <;> rfl) hltys hti
  obtain ⟨A', hx, hsucc⟩ := hctx.next (by cases mut_ <;> intro h <;> cases h)
  obtain ⟨g1, g2, himm, hfull, hnew, hfac, rfl⟩ := exec_borrowLoc hx
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  obtain ⟨rn, hrn⟩ : ∃ rn : Reference,
      rn = ⟨mut_, t, ⟨.local_ fr.id i, []⟩, s.invalidations.length⟩ := ⟨_, rfl⟩
  have hc2 := hd.complete.borrowLoc (r := rn) (by rw [hrn]) (by rw [hrn]; exact himm)
    (by rw [hrn]; exact hfull) (by rw [hrn]; exact hnew) hfac
  have hstep : Step.exec P s fr rest info code (if mut_ then .mutBorrowLoc i else .immBorrowLoc i) =
      .ok (.done (.running { s with frames := Step.advance (Step.push fr (.ref rn)) :: rest })) := by
    subst hrn
    cases mut_ <;>
      simp [Step.exec, Step.borrowLoc, Step.need, hv, hltys, hti, htv, bind, Except.bind, pure,
        Except.pure, Step.next, Step.borrow]
  rw [hstep]
  obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by cases mut_ <;> simp [successors, hi, Bytecode.control])
  refine ⟨?_, ?_⟩
  · exact fallThrough_safe (N := N) (after := .ref mut_ t :: before) hc hact hchain hi
      (by cases mut_ <;> rfl) (fun _ => by cases mut_ <;> rfl) hbefore hafter rfl rfl rfl
      (by
        simp only [Step.advance, Step.push, conformsAll_cons]
        refine ⟨conforms_iff.2 ⟨?_, ?_⟩, hctx.conf⟩
        · rw [hrn]; exact assignable_refl _
        · rw [hrn]; exact htv)
  · have hd' := hd.pushNew (A' := { A with
        stack := .reference A.nextId :: A.stack, graph := g2, nextId := A.nextId + 1 })
      (fr' := Step.advance (Step.push fr (.ref rn))) hctx.typed (Graph.newRef_ok hnew).1
      hctx.height hc2 ⟨rfl, rfl, rfl⟩ rfl rfl rfl
    refine refInv_finish hr hsf (hctx.chain _ hsf) rfl rfl rfl rfl rfl rfl (Nat.le_refl _) hr.storage (es := []) rfl
      (storageKept_of_eq rfl) allowedBy_nil hd' (fun x r₂ hx hxr => ?_) hA'' hle
      (RefsFrom.next (new := [.ref rn]) (n := 0) (by simp [Step.advance, Step.push]) rfl
        (fun v hv' => by
          simp only [List.mem_singleton] at hv'; subst hv'
          exact .of_loc (fun k => by rw [hrn]; simp)))
    by_cases hxc : x = A.nextId
    · subst hxc
      rw [updL_self, Option.some.injEq] at hxr
      subst hxr
      have hrt : rn.target = t := by rw [hrn]
      refine ⟨⟨t, t, by rw [hrn]; simp [RootType, State.frame?, Step.advance, Step.push, hltys, hti],
        by rw [hrn]; simp only [pathType], by rw [hrn]; exact Or.inl rfl⟩,
        by rw [hrn]; exact Nat.le_refl _,
        by rw [hrt]; exact htv, by rw [hrn]; exact Nat.le_refl _, ?_, fun _ => by rw [hrn]; rfl⟩
      rw [deref?_isSome]
      refine ⟨fun e he => ?_, ?_⟩
      · rw [hrn] at he
        simp [State.invalidatedSince] at he
      · rw [show rn.location = ⟨.local_ fr.id i, []⟩ by rw [hrn],
          read?_local (fr := Step.advance (Step.push fr (.ref rn)))
            (by simp [State.frame?, Step.advance, Step.push])]
        simp [Step.advance, Step.push, hv, Value.get?]
    · rw [updL_ne _ hxc] at hxr
      refine (hrefs x r₂ ?_ hxr).same hsf rfl rfl rfl rfl rfl rfl rfl
      simp only [heldIds, List.filterMap_cons, AbsValue.refId?, List.mem_append, List.mem_cons]
        at hx ⊢
      rcases hx with hx | hx | hx
      · exact Or.inl hx
      · exact absurd hx hxc
      · exact Or.inr hx

theorem read?_child {s : State} {l : Location} {st : PathStep} :
    s.read? (l.child st) = (s.read? l).bind (·.child? st) := by
  simp only [State.read?, Location.child, bind, Option.bind]
  cases s.root? l.root with
  | none => rfl
  | some v => exact get_append_step

theorem invalidatedSince_self (s : State) : s.invalidatedSince s.invalidations.length = [] := by
  simp [State.invalidatedSince]

/-- A new reference to a location one step inside a tracked one is tracked,
when the step exists in the value there. -/
theorem RefOk.child {fr' : Frame} {rp : Reference} (h : RefOk P s fr' rp) {st : PathStep}
    {t : RuntimeType} (hst : stepType P rp.target st = some t) (htv : t.isValue = true)
    (hex : ∀ w : Value, s.deref? rp.location rp.since = some w → (w.child? st).isSome = true)
    (mutable : Bool) (hmut : mutable = true → rp.mutable = true) (hstw : st.writable = true) :
    RefOk P s fr' ⟨mutable, t, rp.location.child st, s.invalidations.length⟩ := by
  obtain ⟨w, hw⟩ := h.deref_some
  obtain ⟨⟨T, T', hT, hpath, hasg⟩, hbelow, -, -, -, hwr⟩ := h
  have hT' : T' = rp.target := assignable_rigid hasg.assignable (stepType_rigid hst)
  subst hT'
  refine ⟨⟨T, t, hT, ?_, Or.inl rfl⟩, hbelow, htv, Nat.le_refl _, ?_, fun hm => ?_⟩
  · simp only [Location.child]
    rw [pathType_append, hpath]
    exact hst
  · rw [deref?_isSome]
    refine ⟨fun e he => by simp [invalidatedSince_self] at he, ?_⟩
    simp only
    rw [read?_child, deref?_read hw]
    exact hex w hw
  · simp only [Location.child, List.all_append, List.all_cons, List.all_nil,
      hwr (hmut hm), hstw, Bool.and_self]

/-- The type certificate's typing of a field borrow. -/
theorem typeAfter_fieldBorrow {f : FunctionRef} {m : CompiledModule} {hh : Nat} {mut_ : Bool}
    {before after : List RuntimeType}
    (h : typeAfter P f m (if mut_ then .mutBorrowField hh else .immBorrowField hh) before =
      some after) :
    ∃ fh sid ftys t m' brest, m.fieldHandles[hh]? = some fh ∧
      P.definedStruct? f.module fh.owner = some sid ∧ P.fieldTypes sid = some ftys ∧
      ftys[fh.field]? = some t ∧ before = .ref m' (.struct sid) :: brest ∧
      (mut_ = true → m' = true) ∧ after = .ref mut_ t :: brest := by
  have h' : (do
      let fh ← m.fieldHandles[hh]?
      let s ← P.definedStruct? f.module fh.owner
      let t ← (← P.fieldTypes s)[fh.field]?
      match before with
      | .ref mut' (.struct s') :: rest =>
        if s' = s && (mut' || !mut_) then some (.ref mut_ t :: rest) else none
      | _ => none) = some after := by
    cases mut_ <;> exact h
  clear h
  cases hfh : m.fieldHandles[hh]? with
  | none => simp [hfh] at h'
  | some fh =>
    cases hdef : P.definedStruct? f.module fh.owner with
    | none => simp [hfh, hdef] at h'
    | some sid =>
      cases hft : P.fieldTypes sid with
      | none => simp [hfh, hdef, hft] at h'
      | some ftys =>
        cases ht : ftys[fh.field]? with
        | none => simp [hfh, hdef, hft, ht] at h'
        | some t =>
          refine ⟨fh, sid, ftys, t, ?_⟩
          simp only [hfh, hdef, hft, ht, bind, Option.bind_some] at h'
          split at h'
          · next m' s' brest =>
            split at h'
            · next hc' =>
              simp only [Bool.and_eq_true, decide_eq_true_eq, Bool.or_eq_true,
                Bool.not_eq_true'] at hc'
              obtain ⟨rfl, hm⟩ := hc'
              refine ⟨m', brest, rfl, hdef, hft, ht, rfl, fun hb => ?_, (Option.some.inj h').symm⟩
              subst hb
              rcases hm with hm | hm
              · exact hm
              · cases hm
            · simp at h'
          · simp at h'

theorem exec_fieldBorrow' {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {h : Nat} {mut_ : Bool}
    (hx : (ReferenceSafety.executeInner m view
      (if mut_ then .mutBorrowField h else .immBorrowField h)).run st = .ok ((), st')) :
    ∃ fh, m.fieldHandles[h]? = some fh ∧ ∃ id rest g1 g2 g3, st.stack = .reference id :: rest ∧
      (mut_ = true → st.graph.hasFullBorrows id = .ok false) ∧
      (mut_ = false → st.graph.isReadable id (some (.field fh.field)) = .ok true) ∧
      st.graph.newRef st.nextId mut_ = .ok g1 ∧ g1.factor id [.field fh.field] st.nextId = .ok g2 ∧
      g2.release id = .ok g3 ∧
      st' = { st with
        stack := .reference st.nextId :: rest, graph := g3, nextId := st.nextId + 1 } := by
  cases mut_
  · obtain ⟨fh, hfh, id, rest, g1, g2, g3, hs, hrd, hnew, hfac, hrel, rfl⟩ :=
      exec_immBorrowField (h := h) hx
    exact ⟨fh, hfh, id, rest, g1, g2, g3, hs, fun hb => absurd hb (by decide), fun _ => hrd,
      hnew, hfac, hrel, rfl⟩
  · obtain ⟨fh, hfh, id, rest, g1, g2, g3, hs, hfb, hnew, hfac, hrel, rfl⟩ :=
      exec_mutBorrowField (h := h) hx
    exact ⟨fh, hfh, id, rest, g1, g2, g3, hs, fun _ => hfb, fun hb => absurd hb (by decide),
      hnew, hfac, hrel, rfl⟩

/-- **A field borrow** of handle `h` at type arguments `targs`:
`MutBorrowField`, `ImmBorrowField`, or a generic form, given the analysis runs
it as the non-generic form and the type certificate types it as a borrow of
the instance. -/
theorem refSafe_borrowFieldAt (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {instr : Bytecode}
    {h : Nat} {mut_ : Bool} {targs : List RuntimeType}
    (hctx : TopCtx P s fr info code instr view A stys tys ps)
    (hi : code.code[fr.pc]? = some instr) (hret : instr ≠ .ret)
    (hctl : instr.control = .fallThrough) (hav : ∀ a, availabilityAfter instr a = a)
    (hrun : ∀ A', (ReferenceSafety.executeInner info.module view instr).run A = .ok ((), A') →
      (ReferenceSafety.executeInner info.module view
        (if mut_ then .mutBorrowField h else .immBorrowField h)).run A = .ok ((), A'))
    (htyp : ∀ before after, typeAfter P fr.function info.module instr before = some after →
      ∃ fh sid ftys t m' brest, info.module.fieldHandles[h]? = some fh ∧
        (∀ sinfo, P.structType? sid = some sinfo →
          Step.localStruct P fr.function.module fh.owner targs = .ok (sid, sinfo)) ∧
        P.fieldTypes sid = some ftys ∧ ftys[fh.field]? = some t ∧
        before = .ref m' (.struct sid) :: brest ∧ (mut_ = true → m' = true) ∧
        after = .ref mut_ t :: brest)
    (hexec : Step.exec P s fr rest info code instr =
      Step.borrowField P s fr rest info mut_ h targs) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code instr) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  -- The type certificate's typing of the borrow.
  obtain ⟨fh, sid, ftys, t, m', brest, hfh, hloc, hftys, ht, rfl, hm', rfl⟩ := htyp _ _ hafter
  obtain ⟨sinfo, fields, hsinfo, hfi⟩ := fieldTypes_some hftys
  obtain ⟨v, st, hst, hv', hstc⟩ := conformsAll_cons_right hctx.conf
  obtain ⟨rp, rfl, hrm, hrt⟩ := conforms_ref hv' (by simp [rigid])
  obtain ⟨A', hx, hsucc⟩ := hctx.next hret
  obtain ⟨fh', hfh', id, ra, g1, g2, g3, hsa, hfull, hread, hnew, hfac, hrel, rfl⟩ :=
    exec_fieldBorrow' (hrun _ hx)
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  have hlen : ra.length = st.length := by
    have := hctx.height; rw [hsa, hst] at this; simpa using this
  obtain ⟨r', hr', hL⟩ := hd.top hsa hst hlen
  simp only [Value.ref.injEq] at hr'
  subst hr'
  have hp : id ∈ heldIds A := mem_heldIds.2 (Or.inr (by rw [hsa]; exact List.mem_cons_self))
  obtain ⟨w, hw, hwc⟩ := top_ref_valid hsf hctx hsa hst
  let fr₁ : Frame := { fr with stack := st }
  let s₁ : State := { s with frames := fr₁ :: rest }
  let rn : Reference := ⟨mut_, t, rp.location.child (.field fh.field), s.invalidations.length⟩
  have hc2 := hd.complete.fieldBorrowPre (r := rn) hp hL (s := .field fh.field) rfl rfl
    (fun h => by rw [hrm]; exact hm' h) hfull hread hnew hfac
  have hc3 := hc2.release (Or.inl hp) hrel
  have hstep : Step.exec P s fr rest info code instr =
      .ok (.done (.running { s₁ with
        frames := Step.advance (Step.pushAll fr₁ [.ref rn]) :: rest })) := by
    have hpop : Step.popRefTo fr (.struct sid) mut_ = .ok (rp, fr₁) := by
      simp only [Step.popRefTo, Step.popRef, Step.pop, hst, bind, Except.bind, pure, Except.pure,
        hrt, bne_self_eq_false]
      cases mut_
      · rfl
      · simp [hm' rfl ▸ hrm]; rfl
    have hder : Step.deref s₁ rp = .ok w := by
      simp [Step.deref, Step.need, s₁, fr₁, hw, pure, Except.pure]
    rw [hexec]
    cases mut_ <;>
      simp only [Step.borrowField, Step.need,
        hfh, hloc _ hsinfo, hfi, hftys, ht, bind, Except.bind, pure, Except.pure,
        hpop, hder, Step.finish, Step.borrow, rn, s₁, fr₁] <;> rfl
  rw [hstep]
  obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, hctl])
  have htv : t.isValue = true :=
    fieldTypes_values (fieldTypes_args hftys) hftys t (List.mem_of_getElem? ht)
  refine ⟨?_, refInv_replaceTop hr hsf hctx hd hrefs hsa hst (Graph.newRef_ok hnew).1 hc3
    hA'' hle ?_ (.of_loc fun k h => by simp [rn, Location.child] at h)⟩
  · exact fallThrough_safe (N := N) (after := .ref mut_ t :: brest) hc hact hchain hi
      hctl hav hbefore hafter rfl rfl rfl
      (by
        simp only [Step.advance, Step.pushAll, fr₁, List.reverse_cons, List.reverse_nil,
          List.nil_append, List.cons_append, conformsAll_cons]
        exact ⟨conforms_iff.2 ⟨assignable_refl _, htv⟩, hstc⟩)
  · have hok := (hrefs id rp hp hL).same hsf (s' := { s₁ with
      frames := Step.advance (Step.pushAll fr₁ [.ref rn]) :: rest })
      (fr' := Step.advance (Step.pushAll fr₁ [.ref rn])) rfl rfl rfl rfl rfl rfl rfl
    refine hok.child (st := .field fh.field) ?_ htv ?_ mut_ (fun h => by rw [hrm]; exact hm' h) rfl
    · rw [hrt]; simp [stepType, hftys, ht]
    · intro w' hw'
      have hw'c := hok.deref_conforms (hctx.roots.replaceTop hsf rfl rfl rfl) hw'
      rw [hrt] at hw'c
      obtain ⟨fs, hsl, hfs⟩ := conforms_struct hw'c hftys
      have hl := conformsAll_length hfs
      have hlt : fh.field < fs.length := by
        rw [hl]; exact (List.getElem?_eq_some_iff.1 ht).1
      rcases hsl with rfl | ⟨ks, es, rfl⟩ <;> simp [Value.child?, List.getElem?_eq_getElem hlt]

/-- **`MutBorrowField`, `ImmBorrowField`.** -/
theorem refSafe_borrowField (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {h : Nat} {mut_ : Bool}
    (hctx : TopCtx P s fr info code (if mut_ then .mutBorrowField h else .immBorrowField h) view
      A stys tys ps)
    (hi : code.code[fr.pc]? = some (if mut_ then .mutBorrowField h else .immBorrowField h)) :
    ExecSafe P N (InvRefs P)
      (Step.exec P s fr rest info code (if mut_ then .mutBorrowField h else .immBorrowField h)) :=
  refSafe_borrowFieldAt (targs := []) hc hr hsf hact hchain hctx hi
    (by cases mut_ <;> intro h <;> cases h) (by cases mut_ <;> rfl)
    (fun _ => by cases mut_ <;> rfl) (fun _ hx => hx)
    (fun _ _ hafter => by
      obtain ⟨fh, sid, ftys, t, m', brest, hfh, hdef, hftys, ht, h1, hm', h2⟩ :=
        typeAfter_fieldBorrow hafter
      exact ⟨fh, sid, ftys, t, m', brest, hfh, fun _ hsinfo => localStruct_ok hdef hsinfo, hftys,
        ht, h1, hm', h2⟩)
    (by cases mut_ <;> rfl)

/-- Locations designate the same values in states that agree on their root
and on the invalidations. -/
theorem deref?_congr {s₁ s₂ : State} {l : Location} (hroot : s₂.root? l.root = s₁.root? l.root)
    (hinv : s₂.invalidations = s₁.invalidations) (since : Nat) :
    s₂.deref? l since = s₁.deref? l since := by
  rw [State.deref?, State.deref?, State.invalidatedSince, State.invalidatedSince, hinv,
    read?_eq_of_root hroot]

/-- Roots have the same values after the current frame is replaced by one with
the same identity and locals. -/
theorem root?_replaceTop {s₁ s₂ : State} {fa fb : Frame} (hsf : s₁.frames = fa :: rest)
    (hs₂ : s₂.frames = fb :: rest) (hid : fb.id = fa.id) (hloc : fb.locals = fa.locals)
    (hstor : s₂.storage = s₁.storage) (hheld : s₂.held = s₁.held) (ρ : Root) :
    s₂.root? ρ = s₁.root? ρ := by
  cases ρ with
  | local_ f i =>
    simp only [State.root?]
    by_cases hf : f = fa.id
    · subst hf
      rw [frame?_head hsf, show s₂.frame? fa.id = some fb by rw [← hid]; exact frame?_head hs₂]
      simp [hloc]
    · rw [frame?_cons_ne hsf hs₂ hid hf]
  | global a t => simp only [State.root?, hstor]
  | held k => simp only [State.root?, hheld]

theorem mapM_exec_const {α β : Type} {g : α → Step.Exec β} {c : β} :
    ∀ {l : List α}, (∀ v ∈ l, g v = .ok c) → l.mapM g = .ok (l.map fun _ => c)
  | [], _ => rfl
  | v :: l, h => by
    rw [List.mapM_cons, h v List.mem_cons_self,
      mapM_exec_const fun w hw => h w (List.mem_cons_of_mem _ hw)]
    rfl

theorem mapM_some_of_mem {α β : Type} {f : α → Option β} :
    ∀ {l : List α} {l' : List β}, l.mapM f = some l' → ∀ a ∈ l, ∃ b ∈ l', f a = some b
  | [], _, _, a, ha => by simp at ha
  | x :: l, l', h, a, ha => by
    simp only [List.mapM_cons, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨b, hb, l'', hl'', rfl⟩ := h
    rcases List.mem_cons.1 ha with rfl | ha
    · exact ⟨b, List.mem_cons_self, hb⟩
    · obtain ⟨b', hb', hfb'⟩ := mapM_some_of_mem hl'' a ha
      exact ⟨b', List.mem_cons_of_mem _ hb', hfb'⟩

/-- The fields the variants of a variant field handle share. -/
theorem variantFieldType?_spec {s : StructId} {vfh : VariantFieldHandle} {t : RuntimeType}
    (h : P.variantFieldType? s vfh = some t) :
    ∀ v ∈ vfh.variants, ∃ fields, P.variantFieldTypes s v = some fields ∧
      fields[vfh.field]? = some t := by
  unfold Program.variantFieldType? at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨tys, hmap, h⟩ := h
  intro v hv
  obtain ⟨b, hb, hfb⟩ := mapM_some_of_mem hmap v hv
  simp only [Option.bind_eq_some_iff] at hfb
  obtain ⟨fields, hfs, hb'⟩ := hfb
  refine ⟨fields, hfs, ?_⟩
  rw [hb']
  split at h
  · next t0 ts =>
    split at h
    · next hall =>
      simp only [Option.some.injEq] at h
      subst h
      rcases List.mem_cons.1 hb with rfl | hb
      · rfl
      · have := List.all_eq_true.1 hall b hb
        simp at this
        rw [this]
    · simp at h
  · simp at h

theorem ok_bind' {ε α β : Type} (a : α) (f : α → Except ε β) : (Except.ok a >>= f) = f a := rfl

theorem need_some {α : Type} (a : α) (r : WrongReason) : Step.need (some a) r = .ok a := rfl

/-- The type certificate's typing of a variant field borrow. -/
theorem typeAfter_variantBorrow {f : FunctionRef} {m : CompiledModule} {hh : Nat} {mut_ : Bool}
    {before after : List RuntimeType}
    (h : typeAfter P f m (if mut_ then .mutBorrowVariantField hh else .immBorrowVariantField hh)
      before = some after) :
    ∃ vfh sid t m' brest, m.variantFieldHandles[hh]? = some vfh ∧
      P.definedStruct? f.module vfh.structIndex = some sid ∧ P.variantFieldType? sid vfh = some t ∧
      before = .ref m' (.struct sid) :: brest ∧ (mut_ = true → m' = true) ∧
      after = .ref mut_ t :: brest := by
  have h' : (do
      let vfh ← m.variantFieldHandles[hh]?
      let s ← P.definedStruct? f.module vfh.structIndex
      let t ← P.variantFieldType? s vfh
      match before with
      | .ref mut' (.struct s') :: rest =>
        if s' = s && (mut' || !mut_) then some (.ref mut_ t :: rest) else none
      | _ => none) = some after := by
    cases mut_ <;> exact h
  clear h
  cases hvh : m.variantFieldHandles[hh]? with
  | none => simp [hvh] at h'
  | some vfh =>
    cases hdef : P.definedStruct? f.module vfh.structIndex with
    | none => simp [hvh, hdef] at h'
    | some sid =>
      cases ht : P.variantFieldType? sid vfh with
      | none => simp [hvh, hdef, ht] at h'
      | some t =>
        refine ⟨vfh, sid, t, ?_⟩
        simp only [hvh, hdef, ht, bind, Option.bind_some] at h'
        split at h'
        · next m' s' brest =>
          split at h'
          · next hc' =>
            simp only [Bool.and_eq_true, decide_eq_true_eq, Bool.or_eq_true,
              Bool.not_eq_true'] at hc'
            obtain ⟨rfl, hm⟩ := hc'
            refine ⟨m', brest, rfl, hdef, ht, rfl, fun hb => ?_, (Option.some.inj h').symm⟩
            subst hb
            rcases hm with hm | hm
            · exact hm
            · cases hm
          · simp at h'
        · simp at h'

theorem exec_variantBorrow' {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {h : Nat} {mut_ : Bool}
    (hx : (ReferenceSafety.executeInner m view
      (if mut_ then .mutBorrowVariantField h else .immBorrowVariantField h)).run st =
        .ok ((), st')) :
    ∃ fh, m.variantFieldHandles[h]? = some fh ∧ ∃ id rest g1 g2 g3,
      st.stack = .reference id :: rest ∧
      (mut_ = true → st.graph.hasFullBorrows id = .ok false) ∧
      (mut_ = false → st.graph.isReadable id (some (.field fh.field)) = .ok true) ∧
      st.graph.newRef st.nextId mut_ = .ok g1 ∧ g1.factor id [.field fh.field] st.nextId = .ok g2 ∧
      g2.release id = .ok g3 ∧
      st' = { st with
        stack := .reference st.nextId :: rest, graph := g3, nextId := st.nextId + 1 } := by
  cases mut_
  · obtain ⟨fh, hfh, id, rest, g1, g2, g3, hs, hrd, hnew, hfac, hrel, rfl⟩ :=
      exec_immBorrowVariantField (h := h) hx
    exact ⟨fh, hfh, id, rest, g1, g2, g3, hs, fun hb => absurd hb (by decide), fun _ => hrd,
      hnew, hfac, hrel, rfl⟩
  · obtain ⟨fh, hfh, id, rest, g1, g2, g3, hs, hfb, hnew, hfac, hrel, rfl⟩ :=
      exec_mutBorrowVariantField (h := h) hx
    exact ⟨fh, hfh, id, rest, g1, g2, g3, hs, fun _ => hfb, fun hb => absurd hb (by decide),
      hnew, hfac, hrel, rfl⟩

theorem find?_zip_map {l : List Nat} {c : RuntimeType} {tag : Nat} {p : Nat × RuntimeType}
    (h : (l.zip (l.map fun _ => c)).find? (·.1 == tag) = some p) : p = (tag, c) ∧ tag ∈ l := by
  have hmem := List.mem_of_find?_eq_some h
  have hp := List.find?_some h
  obtain ⟨a, b⟩ := p
  simp only [beq_iff_eq] at hp
  subst hp
  have h1 := List.of_mem_zip hmem
  obtain ⟨-, hb⟩ := h1
  simp only [List.mem_map] at hb
  obtain ⟨_, -, rfl⟩ := hb
  exact ⟨rfl, (List.of_mem_zip hmem).1⟩

/-- **A variant field borrow** of handle `h` at type arguments `targs`:
`MutBorrowVariantField`, `ImmBorrowVariantField`, or a generic form, given the
analysis runs it as the non-generic form and the type certificate types it as
a borrow of the instance. -/
theorem refSafe_borrowVariantFieldAt (hc : Certified P (modelTypes P) (modelAvail P))
    {N : NativeRel} (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {instr : Bytecode}
    {h : Nat} {mut_ : Bool} {targs : List RuntimeType}
    (hctx : TopCtx P s fr info code instr view A stys tys ps)
    (hi : code.code[fr.pc]? = some instr) (hret : instr ≠ .ret)
    (hctl : instr.control = .fallThrough) (hav : ∀ a, availabilityAfter instr a = a)
    (hrun : ∀ A', (ReferenceSafety.executeInner info.module view instr).run A = .ok ((), A') →
      (ReferenceSafety.executeInner info.module view
        (if mut_ then .mutBorrowVariantField h else .immBorrowVariantField h)).run A =
          .ok ((), A'))
    (htyp : ∀ before after, typeAfter P fr.function info.module instr before = some after →
      ∃ vfh sid t m' brest, info.module.variantFieldHandles[h]? = some vfh ∧
        (∀ sinfo, P.structType? sid = some sinfo →
          Step.localStruct P fr.function.module vfh.structIndex targs = .ok (sid, sinfo)) ∧
        P.variantFieldType? sid vfh = some t ∧
        before = .ref m' (.struct sid) :: brest ∧ (mut_ = true → m' = true) ∧
        after = .ref mut_ t :: brest)
    (hexec : Step.exec P s fr rest info code instr =
      Step.borrowVariantField P s fr rest info mut_ h targs) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code instr) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  obtain ⟨vfh, sid, t, m', brest, hvh, hloc, hvt, rfl, hm', rfl⟩ := htyp _ _ hafter
  have hspec := variantFieldType?_spec hvt
  -- The enum's declaration.
  have hne : vfh.variants ≠ [] := by
    intro hn
    simp [Program.variantFieldType?, hn] at hvt
  obtain ⟨v0, hv0⟩ := List.exists_mem_of_ne_nil _ hne
  obtain ⟨fs0, hfs0, -⟩ := hspec v0 hv0
  obtain ⟨sinfo, variants, -, hsinfo, hfi, -⟩ := variantFieldTypes_some hfs0
  obtain ⟨v, st, hst, hv', hstc⟩ := conformsAll_cons_right hctx.conf
  obtain ⟨rp, rfl, hrm, hrt⟩ := conforms_ref hv' (by simp [rigid])
  obtain ⟨A', hx, hsucc⟩ := hctx.next hret
  obtain ⟨fh', hfh', id, ra, g1, g2, g3, hsa, hfull, hread, hnew, hfac, hrel, rfl⟩ :=
    exec_variantBorrow' (hrun _ hx)
  rw [hvh, Option.some.injEq] at hfh'
  subst hfh'
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  have hlen : ra.length = st.length := by
    have := hctx.height; rw [hsa, hst] at this; simpa using this
  obtain ⟨r', hr', hL⟩ := hd.top hsa hst hlen
  simp only [Value.ref.injEq] at hr'
  subst hr'
  have hp : id ∈ heldIds A := mem_heldIds.2 (Or.inr (by rw [hsa]; exact List.mem_cons_self))
  obtain ⟨w, hw, hwc⟩ := top_ref_valid hsf hctx hsa hst
  rw [hrt] at hwc
  obtain ⟨tag, fields, tys', rfl, htys', hfs⟩ := conforms_variant hwc hfs0
  let fr₁ : Frame := { fr with stack := st }
  let s₁ : State := { s with frames := fr₁ :: rest }
  -- The concrete step, up to the lookup of the variant.
  have hmap : vfh.variants.mapM (fun v => do
      let _ ← Step.need variants[v]? .badIndex
      let fields ← Step.need (P.variantFieldTypes sid v) (.unsupported "field types")
      Step.need fields[vfh.field]? .badIndex) = .ok (vfh.variants.map fun _ => t) := by
    refine mapM_exec_const fun v hv => ?_
    obtain ⟨fs, hfs', hft⟩ := hspec v hv
    obtain ⟨sinfo', variants', vd, hsinfo', hfi', hvd⟩ := variantFieldTypes_some hfs'
    rw [hsinfo, Option.some.injEq] at hsinfo'
    subst hsinfo'
    rw [hfi, StructFieldInformation.declaredVariants.injEq] at hfi'
    subst hfi'
    simp [Step.need, hvd, hfs', hft, bind, Except.bind, pure, Except.pure]
  have hpop : Step.popRefTo fr (.struct sid) mut_ = .ok (rp, fr₁) := by
    simp only [Step.popRefTo, Step.popRef, Step.pop, hst, bind, Except.bind, pure, Except.pure,
      hrt, bne_self_eq_false]
    cases mut_
    · rfl
    · simp [hm' rfl ▸ hrm]; rfl
  have hder : Step.deref ({ s with frames := fr₁ :: rest } : State) rp =
      .ok (.variant sid tag fields) := by
    simp [Step.deref, Step.need, fr₁, hw, pure, Except.pure]
  cases hfind : (vfh.variants.zip (vfh.variants.map fun _ => t)).find? (·.1 == tag) with
  | none =>
    have hstep : Step.exec P s fr rest info code instr = .error (.failed .variantMismatch) := by
      rw [hexec]
      cases mut_ <;>
        simp only [Step.borrowVariantField,
          hvh, need_some, ok_bind', hloc _ hsinfo, hfi, hmap, hpop, hder, hfind,
          Step.fail]
    rw [hstep]
    trivial
  | some p =>
    obtain ⟨rfl, htag⟩ := find?_zip_map hfind
    obtain ⟨fst, hfst, hftag⟩ := hspec tag htag
    rw [hfst, Option.some.injEq] at htys'
    subst htys'
    let rn : Reference := ⟨mut_, t, rp.location.child (.variantField tag vfh.field),
      s.invalidations.length⟩
    have hc2 := hd.complete.fieldBorrowPre (r := rn) hp hL (s := .variantField tag vfh.field)
      rfl rfl (fun h => by rw [hrm]; exact hm' h) hfull hread hnew hfac
    have hc3 := hc2.release (Or.inl hp) hrel
    have hstep : Step.exec P s fr rest info code instr =
        .ok (.done (.running { s₁ with
          frames := Step.advance (Step.pushAll fr₁ [.ref rn]) :: rest })) := by
      rw [hexec]
      cases mut_ <;>
        simp only [Step.borrowVariantField,
          hvh, need_some, ok_bind', hloc _ hsinfo, hfi, hmap, hpop, hder, hfind,
          Step.finish] <;> rfl
    rw [hstep]
    obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, hctl])
    have htv : t.isValue = true := variantFieldTypes_values (variantFieldTypes_args hfst) hfst t
      (List.mem_of_getElem? hftag)
    refine ⟨?_, refInv_replaceTop hr hsf hctx hd hrefs hsa hst (Graph.newRef_ok hnew).1 hc3
      hA'' hle ?_ (.of_loc fun k h => by simp [rn, Location.child] at h)⟩
    · exact fallThrough_safe (N := N) (after := .ref mut_ t :: brest) hc hact hchain hi
        hctl hav hbefore hafter rfl rfl rfl
        (by
          simp only [Step.advance, Step.pushAll, fr₁, List.reverse_cons, List.reverse_nil,
            List.nil_append, List.cons_append, conformsAll_cons]
          exact ⟨conforms_iff.2 ⟨assignable_refl _, htv⟩, hstc⟩)
    · have hok := (hrefs id rp hp hL).same hsf (s' := { s₁ with
        frames := Step.advance (Step.pushAll fr₁ [.ref rn]) :: rest })
        (fr' := Step.advance (Step.pushAll fr₁ [.ref rn])) rfl rfl rfl rfl rfl rfl rfl
      refine hok.child (st := .variantField tag vfh.field) ?_ htv ?_ mut_
        (fun h => by rw [hrm]; exact hm' h) rfl
      · rw [hrt]; simp [stepType, hfst, hftag]
      · intro w' hw'
        have heq := deref?_congr (s₁ := s₁) (l := rp.location)
          (s₂ := { s₁ with frames := Step.advance (Step.pushAll fr₁ [.ref rn]) :: rest })
          (root?_replaceTop (s₁ := s₁)
            (s₂ := { s₁ with frames := Step.advance (Step.pushAll fr₁ [.ref rn]) :: rest })
            (rest := rest) rfl rfl rfl rfl rfl rfl rp.location.root) rfl rp.since
        rw [heq, hw, Option.some.injEq] at hw'
        subst hw'
        have hl := conformsAll_length hfs
        have hlt : vfh.field < fields.length := by
          rw [hl]; exact (List.getElem?_eq_some_iff.1 hftag).1
        simp [Value.child?, List.getElem?_eq_getElem hlt]

/-- **`MutBorrowVariantField`, `ImmBorrowVariantField`.** -/
theorem refSafe_borrowVariantField (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {h : Nat} {mut_ : Bool}
    (hctx : TopCtx P s fr info code
      (if mut_ then .mutBorrowVariantField h else .immBorrowVariantField h) view A stys tys ps)
    (hi : code.code[fr.pc]? =
      some (if mut_ then .mutBorrowVariantField h else .immBorrowVariantField h)) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code
      (if mut_ then .mutBorrowVariantField h else .immBorrowVariantField h)) :=
  refSafe_borrowVariantFieldAt (targs := []) hc hr hsf hact hchain hctx hi
    (by cases mut_ <;> intro h <;> cases h) (by cases mut_ <;> rfl)
    (fun _ => by cases mut_ <;> rfl) (fun _ hx => hx)
    (fun _ _ hafter => by
      obtain ⟨vfh, sid, t, m', brest, hvh, hdef, hvt, h1, hm', h2⟩ :=
        typeAfter_variantBorrow hafter
      exact ⟨vfh, sid, t, m', brest, hvh, fun _ hsinfo => localStruct_ok hdef hsinfo, hvt, h1,
        hm', h2⟩)
    (by cases mut_ <;> rfl)

/-- The common end of the instructions that replace the reference on top by
one derived from it, from an abstract state and frame already rid of the
operands above the reference. -/
theorem refInv_replaceTop' (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hchain : Below P s fr ps rest)
    {ltys stys0 : List SignatureToken} {avail : LocalIndex → Availability} {A0 : RefState}
    {fr0 : Frame} (ht0 : Typed ltys stys0 avail A0) {L : Nat → Option Reference}
    (hd : Desc A0 fr0 (gsidOf P (s.frames.map Frame.skel)) ps L)
    (hrefs : ∀ x r, x ∈ heldIds A0 → L x = some r → RefOk P s fr r)
    (hid0 : fr0.id = fr.id) (hloc0 : fr0.locals = fr.locals) {p c : Nat}
    {ra : List AbsValue} {rp r : Reference} {st : List Value} {g3 : Graph}
    (hsa : A0.stack = .reference p :: ra) (hst : fr0.stack = .ref rp :: st)
    (hlen : ra.length = st.length) (hfresh : A0.graph.find? c = none)
    (hc3 : Complete g3 (updL L c r) (fun x => (x ∈ heldIds A0 ∨ x = c) ∧ x ≠ p) A0.locals.length
      fr0.id (gsidOf P (s.frames.map Frame.skel)) ps)
    {n : Nat} {A'' : RefState} (hA'' : modelRef P fr.function (fr.pc + 1) = some A'')
    (hle : RefLe { A0 with stack := .reference c :: ra, graph := g3, nextId := n } A'')
    (hnew : RefOk P { s with frames := Step.advance (Step.pushAll { fr with stack := st } [.ref r]) :: rest }
      (Step.advance (Step.pushAll { fr with stack := st } [.ref r])) r)
    (hfrom : RefsFrom fr (Step.advance (Step.pushAll { fr with stack := st } [.ref r]))) :
    RefInvariant P { s with frames := Step.advance (Step.pushAll { fr with stack := st } [.ref r]) :: rest } := by
  have hd' := hd.replaceTop ht0 hsa hst hlen hfresh hc3
    (A' := { A0 with stack := .reference c :: ra, graph := g3, nextId := n })
    (fr' := Step.advance (Step.pushAll { fr with stack := st } [.ref r])) ⟨rfl, rfl, rfl⟩
    (by simp [Step.advance, Step.pushAll, hid0]) (by simp [Step.advance, Step.pushAll, hloc0])
    (by simp [Step.advance, Step.pushAll])
  refine refInv_finish hr hsf hchain rfl rfl rfl rfl rfl rfl (Nat.le_refl _) hr.storage (es := []) rfl
    (storageKept_of_eq rfl) allowedBy_nil hd' (fun x r₂ hx hxr => ?_) hA'' hle hfrom
  by_cases hxc : x = c
  · subst hxc
    rw [updL_self, Option.some.injEq] at hxr
    subst hxr
    exact hnew
  · rw [updL_ne _ hxc] at hxr
    refine (hrefs x r₂ ?_ hxr).same hsf rfl rfl rfl rfl rfl rfl rfl
    simp only [heldIds, List.filterMap_cons, AbsValue.refId?, List.mem_append, List.mem_cons]
      at hx ⊢
    rw [hsa]
    simp only [List.filterMap_cons, AbsValue.refId?, List.mem_cons]
    rcases hx with hx | hx | hx
    · exact Or.inl hx
    · exact absurd hx hxc
    · exact Or.inr (Or.inr hx)

/-- The type certificate's typing of an element borrow. -/
theorem typeAfter_elemBorrow {f : FunctionRef} {m : CompiledModule} {sig : Nat} {mut_ : Bool}
    {before after : List RuntimeType}
    (h : typeAfter P f m (if mut_ then .vecMutBorrow sig else .vecImmBorrow sig) before =
      some after) :
    ∃ t m' brest, m.vectorElemTypeWith f.typeArgs sig = some t ∧
      before = .int .u64 :: .ref m' (.vector t) :: brest ∧ (mut_ = true → m' = true) ∧
      after = .ref mut_ t :: brest := by
  have h' : (do
      let t ← m.vectorElemTypeWith f.typeArgs sig
      match before with
      | .int .u64 :: .ref mut' (.vector t') :: rest =>
        if t' = t && (mut' || !mut_) then some (.ref mut_ t :: rest) else none
      | _ => none) = some after := by
    cases mut_ <;> exact h
  clear h
  cases het : m.vectorElemTypeWith f.typeArgs sig with
  | none => simp [het] at h'
  | some t =>
    refine ⟨t, ?_⟩
    simp only [het, bind, Option.bind_some] at h'
    split at h'
    · next m' t' brest =>
      split at h'
      · next hc' =>
        simp only [Bool.and_eq_true, decide_eq_true_eq, Bool.or_eq_true, Bool.not_eq_true'] at hc'
        obtain ⟨rfl, hm⟩ := hc'
        refine ⟨m', brest, rfl, rfl, fun hb => ?_, (Option.some.inj h').symm⟩
        subst hb
        rcases hm with hm | hm
        · exact hm
        · cases hm
      · simp at h'
    · simp at h'

/-- **`VecMutBorrow`, `VecImmBorrow`.** -/
theorem refSafe_vecBorrow (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {sig : Nat} {mut_ : Bool}
    (hctx : TopCtx P s fr info code (if mut_ then .vecMutBorrow sig else .vecImmBorrow sig) view
      A stys tys ps)
    (hi : code.code[fr.pc]? = some (if mut_ then .vecMutBorrow sig else .vecImmBorrow sig)) :
    ExecSafe P N (InvRefs P)
      (Step.exec P s fr rest info code (if mut_ then .vecMutBorrow sig else .vecImmBorrow sig)) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  obtain ⟨t, m', brest, het, rfl, hm', rfl⟩ := typeAfter_elemBorrow hafter
  obtain ⟨vi, st1, hst1, hvi, h1⟩ := conformsAll_cons_right hctx.conf
  obtain ⟨x, rfl, -⟩ := conforms_int hvi
  obtain ⟨v, st, hst2, hv', hstc⟩ := conformsAll_cons_right h1
  subst hst2
  obtain ⟨rp, rfl, hrm, hrt⟩ := conforms_ref hv' (by simp [rigid])
  obtain ⟨A', hx, hsucc⟩ := hctx.next (by cases mut_ <;> intro h <;> cases h)
  obtain ⟨id, ra, g1, g2, g3, hsa, hrd, hwr, hnew, hadd, hrel, rfl⟩ := exec_vecBorrow hx
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  have hlen : ra.length = st.length := by
    have := hctx.height; rw [hsa, hst1] at this; simpa using this
  -- Without the index.
  let A0 : RefState := { A with stack := .reference id :: ra }
  let fr0 : Frame := { fr with stack := .ref rp :: st }
  have hd0 : Desc A0 fr0 (gsidOf P (s.frames.map Frame.skel)) ps L :=
    hd.pop hsa hst1 (by simp [hlen]) rfl rfl rfl rfl rfl
  obtain ⟨t0, stys0, hstys⟩ : ∃ t0 stys0, stys = t0 :: stys0 := by
    have := hctx.typed.nstack
    rw [hsa] at this
    cases stys with
    | nil => simp at this
    | cons t0 stys0 => exact ⟨t0, stys0, rfl⟩
  have ht0 : Typed (view.parameters ++ view.locals) stys0 (modelAvail P fr.function fr.pc) A0 :=
    hctx.typed.popValue hsa hstys
  have hp : id ∈ heldIds A0 := mem_heldIds.2 (Or.inr (List.mem_cons_self))
  have hrefs0 : ∀ y r, y ∈ heldIds A0 → L y = some r → RefOk P s fr r := fun y r hy hyr =>
    hrefs y r (by simpa [heldIds, A0, hsa, AbsValue.refId?] using hy) hyr
  obtain ⟨r', hr', hL⟩ := hd0.top rfl rfl hlen
  simp only [Value.ref.injEq] at hr'
  subst hr'
  -- The vector.
  have hok₁ : RefOk P { s with frames := { fr with stack := st } :: rest } { fr with stack := st } rp :=
    (hrefs0 id rp hp hL).same hsf rfl rfl rfl rfl rfl rfl rfl
  obtain ⟨w, hw⟩ := hok₁.deref_some
  have hwc := hok₁.deref_conforms (hctx.roots.replaceTop hsf rfl rfl rfl) hw
  rw [hrt] at hwc
  obtain ⟨elems, rfl, -⟩ := conforms_vector hwc
  have hpop : Step.popRefTo { fr with stack := .ref rp :: st } (.vector t) mut_ =
      .ok (rp, { fr with stack := st }) := by
    simp only [Step.popRefTo, Step.popRef, Step.pop, bind, Except.bind, pure, Except.pure,
      hrt, bne_self_eq_false]
    cases mut_
    · rfl
    · simp [hm' rfl ▸ hrm]
  have hidx : Step.popIndex fr = .ok (x.toNat, { fr with stack := .ref rp :: st }) := by
    simp [Step.popIndex, Step.popInt, Step.pop, hst1, bind, Except.bind, pure, Except.pure]
  have hdv : Step.derefVector ({ s with frames := { fr with stack := st } :: rest } : State) rp =
      .ok elems := by
    simp [Step.derefVector, Step.deref, Step.need, hw, bind, Except.bind, pure, Except.pure]
  by_cases hin : x.toNat < elems.length
  · let rn : Reference := ⟨mut_, t, rp.location.child (.element x.toNat), s.invalidations.length⟩
    have hc2 := hd0.complete.elemPre (r := rn) hp hL rfl (fun h => by rw [hrm]; exact hm' h)
      hwr hrd hnew hadd
    have hc3 := hc2.release (Or.inl hp) hrel
    have hstep : Step.exec P s fr rest info code (if mut_ then .vecMutBorrow sig else .vecImmBorrow sig) =
        .ok (.done (.running { s with
          frames := Step.advance (Step.pushAll { fr with stack := st } [.ref rn]) :: rest })) := by
      cases mut_ <;>
        simp only [Bool.false_eq_true, if_false, if_true, Step.exec, Step.borrowElement,
          elemType_ok het, ok_bind', hidx, hpop, hdv, hin, if_true, Step.finish] <;> rfl
    rw [hstep]
    obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1)
      (by cases mut_ <;> simp [successors, hi, Bytecode.control])
    have htv : t.isValue = true :=
      vectorElemTypeWith_value (typeArgsOk_values hact.typeArgs) het
    refine ⟨?_, refInv_replaceTop' hr hsf (hctx.chain _ hsf) ht0 hd0 hrefs0 rfl rfl (rfl : A0.stack = _) rfl hlen
      (Graph.newRef_ok hnew).1 hc3 hA'' hle ?_
      (RefsFrom.next (new := [.ref rn]) (n := 2) (by simp [Step.advance, Step.pushAll, hst1]) rfl
        (fun v hv' => by
          simp only [List.mem_singleton] at hv'; subst hv'
          exact .of_loc fun k h => by simp [rn, Location.child] at h))⟩
    · exact fallThrough_safe (N := N) (after := .ref mut_ t :: brest) hc hact hchain hi
        (by cases mut_ <;> rfl) (fun _ => by cases mut_ <;> rfl) hbefore hafter rfl rfl rfl
        (by
          simp only [Step.advance, Step.pushAll, List.reverse_cons, List.reverse_nil,
            List.nil_append, List.cons_append, conformsAll_cons]
          exact ⟨conforms_iff.2 ⟨assignable_refl _, htv⟩, hstc⟩)
    · have hok := (hrefs0 id rp hp hL).same hsf (s' := { s with
        frames := Step.advance (Step.pushAll { fr with stack := st } [.ref rn]) :: rest })
        (fr' := Step.advance (Step.pushAll { fr with stack := st } [.ref rn])) rfl rfl rfl rfl rfl rfl rfl
      refine hok.child (st := .element x.toNat) ?_ htv ?_ mut_
        (fun h => by rw [hrm]; exact hm' h) rfl
      · rw [hrt]; rfl
      · intro w' hw'
        have heq := deref?_congr (s₁ := { s with frames := { fr with stack := st } :: rest })
          (l := rp.location)
          (s₂ := { s with frames := Step.advance (Step.pushAll { fr with stack := st } [.ref rn]) ::
            rest })
          (root?_replaceTop (s₁ := { s with frames := { fr with stack := st } :: rest })
            (s₂ := { s with frames := Step.advance (Step.pushAll { fr with stack := st } [.ref rn]) ::
              rest }) (rest := rest) rfl rfl rfl rfl rfl rfl rp.location.root) rfl rp.since
        rw [heq, hw, Option.some.injEq] at hw'
        subst hw'
        simp [Value.child?, List.getElem?_eq_getElem hin]
  · have hstep : Step.exec P s fr rest info code (if mut_ then .vecMutBorrow sig else .vecImmBorrow sig) =
        .error (.failed .vectorIndex) := by
      cases mut_ <;>
        simp only [Bool.false_eq_true, if_false, if_true, Step.exec, Step.borrowElement,
          elemType_ok het, ok_bind', hidx, hpop, hdv, hin, if_false, Step.fail]
    rw [hstep]
    trivial

end MoveBytecode.Proofs
