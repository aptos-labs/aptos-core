-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Soundness
import Proofs.CertificatesGen
import Proofs.AbilitiesGen

/-!
# Soundness of the model on generic code (D1)

UNTRUSTED (checked by the kernel). A program the model accepts, on the whole
supported subset, generic code and closures included, satisfies every
condition of `Certified` with the model's own certificates: the type
certificate at every instance whose type arguments satisfy its function's
constraints (C1), the availability certificate (C2), resolution of every
call, generic call and closure target (K1, K2), declared abilities justified by the fields at every instance (L6
through K1), and `drop` for every local `StLoc` or `Ret` may discard, at the
instance. So a program by value never goes wrong (`soundness_byValue_gen`),
and its executions agree with the certificates (`certificatesHold_byValue_gen`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

variable {P : Program}

/-! ## Locals and natives at an instance -/

/-- An instance whose type arguments satisfy its function's constraints has
local types. -/
theorem localTypes_some_gen (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit}
    (hf : P.function? f = some info) (hcode : info.definition.code = some code)
    (hok : P.typeArgsOk f = true) : ∃ tys, P.localTypes f = some tys := by
  obtain ⟨-, ⟨cfg, view, -, hview, -⟩⟩ := function_verified hwf hf hcode
  obtain ⟨r, hr, -⟩ := (genCtx_of hwf hs hf hcode hok hview).locals
  exact ⟨r, hr⟩

theorem ofTokenListWith_some_mem {m : CompiledModule} {tys : List RuntimeType} :
    ∀ {ts : List SignatureToken} {us : List RuntimeType},
      RuntimeType.ofTokenListWith m tys ts = some us →
        ∀ t ∈ ts, ∃ u, RuntimeType.ofTokenWith m tys t = some u
  | [], _, _, _, ht => by simp at ht
  | t' :: ts, _, h, t, ht => by
    simp only [RuntimeType.ofTokenListWith, bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at h
    obtain ⟨u, hu, us, hus, -⟩ := h
    rcases List.mem_cons.1 ht with rfl | ht
    · exact ⟨u, hu⟩
    · exact ofTokenListWith_some_mem hus t ht

/-- A supported value signature translates to value types, at type arguments
of its arity that are value types. -/
theorem ofSignatureWith_values_of_supported {m : CompiledModule} {s n : Nat}
    (hs : m.SupportedValueSignature s n) {tys : List RuntimeType}
    (hv : ∀ x ∈ tys, x.isValue = true) (hlen : tys.length = n) {sig : Signature}
    (hsig : m.signatures[s]? = some sig) {rs : List RuntimeType}
    (h : RuntimeType.ofSignatureWith m tys sig = some rs) : ∀ t ∈ rs, t.isValue = true := by
  obtain ⟨sig', hsig', hsome⟩ := hs
  rw [hsig, Option.some.injEq] at hsig'
  subst hsig'
  obtain ⟨us, hus⟩ := Option.isSome_iff_exists.1 (hsome tys hlen)
  intro t ht
  obtain ⟨tok, htok, hr⟩ := mapM_some_mem h t ht
  obtain ⟨u, hu⟩ := ofTokenListWith_some_mem hus tok htok
  rw [ofSlotW_of_ofTokenW hu, Option.some.injEq] at hr
  subst hr
  exact ofTokenWith_isValue hv hu

/-- **A supported program's natives that are not modeled take and return
values only**, at every instance whose type arguments satisfy their
constraints. -/
theorem native_types_values_gen (hs : P.Supported) {f : FunctionRef} {info : FunctionInfo}
    (hf : P.function? f = some info) (hnone : info.definition.code = none)
    (hok : P.typeArgsOk f = true) (hshape : P.nativeShape? f = none) :
    (∀ tys, P.paramTypes f = some tys → ∀ t ∈ tys, t.isValue = true) ∧
    (∀ tys, P.returnTypes f = some tys → ∀ t ∈ tys, t.isValue = true) := by
  obtain ⟨hm, -, hd, hfh⟩ := function?_parts hf
  obtain ⟨-, hnat, -⟩ := hs.1 _ (module_mem hm)
  rcases hnat _ (getElem?_mem_toList hd) hnone with hmod | ⟨fh, hfh', hpar, hret, -⟩
  · exfalso
    have : P.nativeShape? f = info.module.modeledNative? info.definition := by
      simp [Program.nativeShape?, hf]
    rw [hshape] at this
    rw [← this] at hmod
    simp at hmod
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  have hargs : RuntimeType.argumentsWellFormed P f.typeArgs info.handle.typeParameters = true := by
    simpa [Program.typeArgsOk, hf] using hok
  have hlen := argumentsWellFormed_length hargs
  have hv := typeArgsOk_values hok
  constructor
  · intro tys htys
    simp only [Program.paramTypes, hf, bind, Option.bind_some, Option.bind_eq_some_iff] at htys
    obtain ⟨sig, hsig, htys⟩ := htys
    exact ofSignatureWith_values_of_supported hpar hv hlen hsig htys
  · intro tys htys
    simp only [Program.returnTypes, hf, bind, Option.bind_some, Option.bind_eq_some_iff] at htys
    obtain ⟨sig, hsig, htys⟩ := htys
    exact ofSignatureWith_values_of_supported hret hv hlen hsig htys

/-! ## Calls -/

/-- **K1, K2** for `Call` in generic code: it names a non-generic handle (L9),
which resolves to a visible non-generic function with the handle's types. -/
theorem calls_certified_gen (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit}
    (hf : P.function? f = some info) (hcode : info.definition.code = some code) {pc : Nat}
    {h : FunctionHandleIndex} (hi : code.code[pc]? = some (.call h)) :
    ∃ callee cinfo, P.resolve f.module h = some callee ∧
      P.visibleFrom info.moduleId callee = true ∧ P.function? callee = some cinfo ∧
      cinfo.handle.typeParameters = [] ∧
      P.paramTypes callee = P.handleParamTypes f.module h ∧
      P.returnTypes callee = P.handleReturnTypes f.module h := by
  obtain ⟨hm, hid, hd, -⟩ := function?_parts hf
  have hl9 := instructionConsistency_of_accepted (wellFormed_module hwf hm).1 _
    (getElem?_mem_toList hd) code hcode _ (getElem?_mem_toList hi)
  simp only [Bytecode.OperandsConsistent, CompiledModule.functionTypeParamCount, bind,
    Option.bind_eq_some_iff, pure, Option.some.injEq] at hl9
  obtain ⟨fh, hfh, hlen0⟩ := hl9
  obtain ⟨callee, cinfo, hres, hcf, hvis, hclen, -, htys⟩ := handle_resolves_all hwf hs hm hid hfh
  have htp : cinfo.handle.typeParameters = [] :=
    List.eq_nil_of_length_eq_zero (hclen.trans hlen0)
  have hceq := withTypeArgs_self (resolve_typeArgs hres)
  obtain ⟨⟨ps, hps1, hps2⟩, ⟨rs, hrs1, hrs2⟩⟩ := htys [] (by simp [hlen0])
  rw [hceq] at hps2 hrs2
  exact ⟨callee, cinfo, hres, hvis, hcf, htp, hps2.trans hps1.symm, hrs2.trans hrs1.symm⟩

/-- **K1, K2** for `CallGeneric`: the instantiation's handle resolves to a
visible function of its arity, whose constraints the handle's include, with
the handle's types at every instance of the arity. -/
theorem callsGeneric_certified (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) {f : FunctionRef} {info : FunctionInfo}
    (hf : P.function? f = some info) {i : FunctionInstantiationIndex}
    {x : FunctionInstantiation} (hx : info.module.functionInstantiations[i]? = some x) :
    ∃ callee cinfo fh, P.resolve f.module x.handle = some callee ∧
      P.visibleFrom info.moduleId callee = true ∧ P.function? callee = some cinfo ∧
      info.module.functionHandles[x.handle]? = some fh ∧
      cinfo.handle.typeParameters.length = fh.typeParameters.length ∧
      (∀ (j : Nat) (c d : AbilitySet), fh.typeParameters[j]? = some c →
        cinfo.handle.typeParameters[j]? = some d → d.subset c = true) ∧
      ∀ tys : List RuntimeType, tys.length = fh.typeParameters.length →
        P.paramTypes { callee with typeArgs := tys } =
          P.handleParamTypesAt f.module x.handle tys ∧
        P.returnTypes { callee with typeArgs := tys } =
          P.handleReturnTypesAt f.module x.handle tys := by
  obtain ⟨hm, hid, -, -⟩ := function?_parts hf
  have hb := (verifyModule_ok (wellFormed_module hwf hm).1).1
  obtain ⟨fh, hfh⟩ :=
    getElem?_of_lt (verifyBounds_functionInstantiations hb x (getElem?_mem_toList hx))
  obtain ⟨callee, cinfo, hres, hcf, hvis, hclen, hcons, htys⟩ :=
    handle_resolves_all hwf hs hm hid hfh
  refine ⟨callee, cinfo, fh, hres, hvis, hcf, hfh, hclen, hcons, fun tys hlen => ?_⟩
  obtain ⟨⟨ps, hps1, hps2⟩, ⟨rs, hrs1, hrs2⟩⟩ := htys tys hlen
  exact ⟨hps2.trans hps1.symm, hrs2.trans hrs1.symm⟩

/-- **K1, K2** for closures: the handle a closure instruction names resolves
to a visible function with code (`Program.ClosureTargetsHaveCode`), of the
handle's arity, whose constraints the handle's include, with the handle's
types at every instance of the arity. -/
theorem closures_certified (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit}
    (hf : P.function? f = some info) (hcode : info.definition.code = some code) {pc : Nat}
    {instr : Bytecode} {h : FunctionHandleIndex} (hi : code.code[pc]? = some instr)
    (hh : instr.closureHandle? info.module = some h) :
    ∃ target tinfo fh, P.resolve f.module h = some target ∧
      P.visibleFrom info.moduleId target = true ∧ P.function? target = some tinfo ∧
      tinfo.definition.code.isSome = true ∧
      info.module.functionHandles[h]? = some fh ∧
      tinfo.handle.typeParameters.length = fh.typeParameters.length ∧
      (∀ (j : Nat) (c d : AbilitySet), fh.typeParameters[j]? = some c →
        tinfo.handle.typeParameters[j]? = some d → d.subset c = true) ∧
      ∀ tys : List RuntimeType, tys.length = fh.typeParameters.length →
        P.paramTypes { target with typeArgs := tys } = P.handleParamTypesAt f.module h tys ∧
        P.returnTypes { target with typeArgs := tys } = P.handleReturnTypesAt f.module h tys := by
  obtain ⟨hm, hid, hd, -⟩ := function?_parts hf
  have hl9 := instructionConsistency_of_accepted (wellFormed_module hwf hm).1 _
    (getElem?_mem_toList hd) code hcode _ (getElem?_mem_toList hi)
  obtain ⟨fh, hfh⟩ : ∃ fh, info.module.functionHandles[h]? = some fh := by
    cases instr <;> simp [Bytecode.closureHandle?] at hh
    case packClosure h' mask =>
      subst hh
      simp only [Bytecode.OperandsConsistent, CompiledModule.functionTypeParamCount, bind,
        Option.bind_eq_some_iff] at hl9
      obtain ⟨fh, hfh, -⟩ := hl9
      exact ⟨fh, hfh⟩
    case packClosureGeneric i mask =>
      obtain ⟨x, hx, rfl⟩ := hh
      simp only [Bytecode.OperandsConsistent] at hl9
      obtain ⟨x', hx', n, -, hcount, -⟩ := hl9
      rw [hx, Option.some.injEq] at hx'
      subst hx'
      simp only [CompiledModule.functionTypeParamCount, bind, Option.bind_eq_some_iff] at hcount
      obtain ⟨fh, hfh, -⟩ := hcount
      exact ⟨fh, hfh⟩
  obtain ⟨target, tinfo, hres, htf, hvis, hclen, hcons, htys⟩ :=
    handle_resolves_all hwf hs hm hid hfh
  have htcode := hs.2 f.module info.module hm _ (getElem?_mem_toList hd) code hcode instr
    (getElem?_mem_toList hi) h hh target tinfo hres htf
  refine ⟨target, tinfo, fh, hres, hvis, htf, htcode, hfh, hclen, hcons, fun tys hlen => ?_⟩
  obtain ⟨⟨ps, hps1, hps2⟩, ⟨rs, hrs1, hrs2⟩⟩ := htys tys hlen
  exact ⟨hps2.trans hps1.symm, hrs2.trans hrs1.symm⟩

/-! ## Availability at an instance -/

theorem modelAvail_entryRange_gen (hsched : SchedulerSound)
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
    {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit} (hf : P.function? f = some info)
    (hcode : info.definition.code = some code) (hok : P.typeArgsOk f = true) (i : Nat)
    (hi : modelAvail P f 0 i = .unavailable) :
    ∃ tys, P.localTypes f = some tys ∧ i < tys.length := by
  obtain ⟨-, ⟨cfg, view, hcf, hview, -, -, inv, hls⟩⟩ := function_verified hwf hf hcode
  obtain ⟨hvc, -, hvcfg, -, -, -, -⟩ := functionView_fields hview
  have hbuild := verifyControlFlow_build hcf
  have hls0 := hls
  unfold verifyLocalsSafety at hls0
  simp only [bind_ok] at hls0
  obtain ⟨initial, hinit, hana⟩ := hls0
  rw [hvcfg, hvc] at hana
  have hfix := hsched code.code cfg initial _ inv hbuild hana
  obtain ⟨pre, hpre, hle⟩ := hfix.entry
  simp only [modelAvail, hf, hcode, hbuild, hview, hls, availCert, availBefore, hpre] at hi
  have hlt : i < pre.states.size := by
    rcases Nat.lt_or_ge i pre.states.size with h | h
    · exact h
    · simp [stateAvail, Array.getElem?_eq_none h, toAvail] at hi
  obtain ⟨r, hr, hsig⟩ := (genCtx_of hwf hs hf hcode hok hview).locals
  refine ⟨r, hr, ?_⟩
  rw [ofSignature_length hsig, List.length_append, ← initial_size hinit, hle.1]
  exact hlt

/-- A local that `StLoc` may overwrite, or that may hold a value at `Ret`, has
a type with `drop` at every instance whose type arguments satisfy the
function's constraints: the locals-safety pass checks `drop` of its token
with the type parameters at their constraints. -/
theorem modelAvail_drops_gen (hsched : SchedulerSound)
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
    {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit} (hf : P.function? f = some info)
    (hcode : info.definition.code = some code) (hok : P.typeArgsOk f = true) {pc i : Nat}
    (hreach : CodeReachable code.code pc)
    (hinstr : code.code[pc]? = some .ret ∨ code.code[pc]? = some (.stLoc i))
    (hav : modelAvail P f pc i ≠ .unavailable) {tys : List RuntimeType} {t : RuntimeType}
    (htys : P.localTypes f = some tys) (ht : tys[i]? = some t) : (t.abilities P).drop = true := by
  obtain ⟨hhandle, ⟨cfg, view, hcf, hview, hsu, hts, inv, hls⟩⟩ := function_verified hwf hf hcode
  obtain ⟨hvc, -, hvcfg, -, -, -, -⟩ := functionView_fields hview
  have hbuild := verifyControlFlow_build hcf
  have hblocks := verifyControlFlow_blocks hcf
  have hls0 := hls
  unfold verifyLocalsSafety at hls0
  simp only [bind_ok] at hls0
  obtain ⟨initial, hinit, hana⟩ := hls0
  rw [hvcfg, hvc] at hana
  have hfix := hsched code.code cfg initial _ inv hbuild hana
  have hne := nonempty_of_verified ⟨hhandle, cfg, view, hcf, hview, hsu, hts, inv, hls⟩
  have hfall := verifyControlFlow_fallthrough hcf
  have hbounds := (verifyModule_ok (wellFormed_module hwf (function?_parts hf).1).1).1
  have hlt : ∀ pc, ∀ s ∈ successors code.code pc, s < code.code.size :=
    successors_lt hfall (fun pc o i hi ho => branch_targets hbounds
      (getElem?_mem_toList (function?_parts hf).2.2.1) hcode hi ho)
  obtain ⟨b, hb, h1, h2, pre, hpre⟩ := analyzed_at hblocks hfix hne hlt hreach
  obtain ⟨st, st', instr, hi, hbefore, hexe, -, -⟩ := avail_at hblocks hfix hb hpre h1 h2
  have hav' : stateAvail st i ≠ .unavailable := by
    simpa [modelAvail, hf, hcode, hbuild, hview, hls, availCert, hbefore] using hav
  -- The abilities the analysis checks are those of the locals' tokens, with
  -- the type parameters at their constraints.
  have hA := availBefore_abilities
    (analyzeFunction_preserves (localsPreserves initial.abilities) rfl hana) pc st hbefore
  obtain ⟨hlenA, hgetA⟩ := mapM_ok_getElem (initial_abilities hinit)
  have hc := genCtx_of hwf hs hf hcode hok hview
  obtain ⟨r, hr, hsig⟩ := hc.locals
  rw [htys, Option.some.injEq] at hr
  subst hr
  have hinstr' : instr = .ret ∨ instr = .stLoc i := by
    rcases hinstr with h | h <;> rw [hi, Option.some.injEq] at h <;> simp [h]
  obtain ⟨ab, hab, hdrop⟩ := execute_drop hexe hinstr' (fun _ => by
    have := (List.getElem?_eq_some_iff.1 ht).1
    rw [ofSignature_length hsig] at this
    rw [hA]
    simpa [← hlenA] using this) hav'
  rw [hA] at hab
  obtain ⟨tok, htok, habs⟩ := hgetA i ab (by simpa using hab)
  obtain ⟨u, hu, htok'⟩ := ofSigW_getElem hsig htok
  rw [ht, Option.some.injEq] at hu
  subst hu
  exact AbilitySet.drop_of_subset
    (abilities_sub_slot_gen (P := P) (cs := view.handle.typeParameters) hc.resolves hc.targs
      tok habs htok') hdrop

/-! ## Certificates -/

/-- A program by value has no closure instructions. -/
theorem noClosureCode_of_byValue (hbv : P.ByValue) : NoClosureCode P := by
  intro f info code hf hcode instr hi
  have h := (hbv _ (getElem?_mem_toList (function?_parts hf).1)).2 _
    (getElem?_mem_toList (function?_parts hf).2.2.1) code hcode instr hi
  refine ⟨fun h' mask he => ?_, fun i' mask he => ?_, fun s he => ?_⟩ <;> subst he <;>
    simp [Bytecode.inCore, Bytecode.inDataByValue, Bytecode.inGenericByValue] at h

/-- A program of the non-generic subset has no closure instructions. -/
theorem noClosureCode_of_nonGeneric (hs : P.SupportedNonGeneric) : NoClosureCode P := by
  intro f info code hf hcode instr hi
  have h := (((hs _ (getElem?_mem_toList (function?_parts hf).1)).2.2.2.2.2 _
    (getElem?_mem_toList (function?_parts hf).2.2.1) code hcode).2 instr hi).1
  refine ⟨fun h' mask he => ?_, fun i' mask he => ?_, fun s he => ?_⟩ <;> subst he <;>
    simp [Bytecode.inSubsetNG, Bytecode.inCore, Bytecode.inDataByValue,
      Bytecode.inReferences] at h

/-- A program the model accepts, on the supported subset, generic code and
closures included, is certified by the model's own analyses. -/
theorem certified_of_wellFormed_gen (hsched : SchedulerSound)
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported) :
    Certified P (modelTypes P) (modelAvail P) where
  types := modelTypes_consistent_gen hwf hs
  availability := modelAvail_consistent hsched hwf
  locals := fun _ _ _ hf hcode hok => localTypes_some_gen hwf hs hf hcode hok
  nonempty := fun _ _ _ hf hcode => nonempty_of_verified (function_verified hwf hf hcode)
  successors := fun _ _ _ hf hcode pc _ s hs' => by
    obtain ⟨-, ⟨cfg, view, hcf, -⟩⟩ := function_verified hwf hf hcode
    have hbounds := (verifyModule_ok (wellFormed_module hwf (function?_parts hf).1).1).1
    exact successors_lt (verifyControlFlow_fallthrough hcf) (fun pc o i hi ho =>
      branch_targets hbounds (getElem?_mem_toList (function?_parts hf).2.2.1) hcode hi ho) pc s hs'
  calls := fun _ _ _ hf hcode _ _ _ hi => calls_certified_gen hwf hs hf hcode hi
  callsGeneric := fun _ _ _ hf _ _ _ _ _ _ hx => callsGeneric_certified hwf hs hf hx
  closures := fun _ _ _ hf hcode _ _ _ _ hi hh => closures_certified hwf hs hf hcode hi hh
  constants := fun _ _ _ hf _ _ i _ _ c t hc ht => by
    have hconst := (verifyModule_ok (wellFormed_module hwf (function?_parts hf).1).1).2.2.1
    obtain ⟨-, rt, v, hrt, hv, hconf⟩ := constants_decode hconst c (getElem?_mem_toList hc)
    rw [hrt, Option.some.injEq] at ht
    subst ht
    exact ⟨v, hv, hconf _⟩
  entryRange := fun _ _ _ hf hcode hok i hi =>
    modelAvail_entryRange_gen hsched hwf hs hf hcode hok i hi
  abilities := abilitiesSound_gen hwf
  drops := fun _ _ _ hf hcode hok _ _ hreach hinstr hav _ _ htys ht =>
    modelAvail_drops_gen hsched hwf hs hf hcode hok hreach hinstr hav htys ht
  nativeValues := fun _ _ hf hn hok hsh => native_types_values_gen hs hf hn hok hsh

/-! ## The non-generic subset is supported -/

theorem mapM_ofToken_eq {m : CompiledModule} :
    ∀ (ts : List SignatureToken),
      ts.mapM (RuntimeType.ofToken m) = RuntimeType.ofTokenListWith m [] ts
  | [] => rfl
  | t :: ts => by
    simp only [List.mapM_cons, RuntimeType.ofTokenListWith, mapM_ofToken_eq ts]

/-- **A program of the non-generic subset is supported**, given its modules'
index integrity (L1), which gives every definition its handle. -/
theorem supported_of_nonGeneric (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.SupportedNonGeneric) : P.Supported := by
  -- The non-generic subset has no closure instructions.
  have hclos : P.ClosureTargetsHaveCode := by
    intro mi m hm d hd code hcode instr hi h hh
    have hin := (((hs m (getElem?_mem_toList hm)).2.2.2.2.2 d hd code hcode).2 instr hi).1
    cases instr <;> simp [Bytecode.closureHandle?] at hh <;>
      simp [Bytecode.inSubsetNG, Bytecode.inCore, Bytecode.inDataByValue,
        Bytecode.inReferences] at hin
  refine ⟨fun m hm => ?_, hclos⟩
  obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 hm
  rw [Array.getElem?_toList] at hi
  have hacc := (wellFormed_module hwf hi).1
  have hl1 := indexIntegrity_of_accepted hacc
  obtain ⟨h1, h2, h3, h4, h5, h6⟩ := hs m hm
  obtain ⟨-, -, -, -, -, -, -, -, -, -, -, -, -, hsdr, -, hfdr⟩ := hl1
  refine ⟨fun fh hfh => ?_, fun d hd hnone => ?_, fun d hd => ?_, fun c hc => h5 c hc,
    fun d hd code hcode => ?_⟩
  · obtain ⟨hnp, ⟨sig, hsig, hp⟩, ⟨rsig, hrsig, hr⟩⟩ := h1 fh hfh
    rw [hnp]
    exact ⟨⟨sig, hsig, fun tys hl => by
        obtain rfl : tys = [] := List.eq_nil_of_length_eq_zero (by simpa using hl); exact hp⟩,
      ⟨rsig, hrsig, fun tys hl => by
        obtain rfl : tys = [] := List.eq_nil_of_length_eq_zero (by simpa using hl); exact hr⟩⟩
  · obtain ⟨fh, hfh, ⟨sig, hsig, hp⟩, ⟨rsig, hrsig, hr⟩, hfp, hfr⟩ := h2 d hd hnone
    have hnp := (h1 fh (getElem?_mem_toList hfh)).1
    refine Or.inr ⟨fh, hfh, ⟨sig, hsig, fun tys hl => ?_⟩, ⟨rsig, hrsig, fun tys hl => ?_⟩, hfp, hfr⟩ <;>
      rw [hnp] at hl <;> obtain rfl : tys = [] := List.eq_nil_of_length_eq_zero (by simpa using hl) <;>
      rw [← mapM_ofToken_eq] <;> assumption
  · obtain ⟨hnat, hf⟩ := h4 d hd
    obtain ⟨sh, hsh, -⟩ := hsdr d hd
    have hnp := h3 sh (getElem?_mem_toList hsh)
    refine ⟨hnat, sh, hsh, fun f hf' tys hl => ?_⟩
    rw [hnp] at hl
    obtain rfl : tys = [] := List.eq_nil_of_length_eq_zero (by simpa using hl)
    exact hf f hf'
  · obtain ⟨fh, hfh, -⟩ := hfdr d hd
    have hnp := (h1 fh (getElem?_mem_toList hfh)).1
    obtain ⟨⟨lsig, hlsig, hl⟩, hins⟩ := h6 d hd code hcode
    refine ⟨fh, hfh, ⟨lsig, hlsig, fun tys htl => ?_⟩, fun instr hinstr => ?_⟩
    · rw [hnp] at htl
      obtain rfl : tys = [] := List.eq_nil_of_length_eq_zero (by simpa using htl)
      exact hl
    · obtain ⟨hsub, hops⟩ := hins instr hinstr
      rw [hnp]
      refine ⟨by simp only [Bytecode.inSubset]; simp only [Bytecode.inSubsetNG] at hsub; simp [hsub], ?_⟩
      cases instr <;> simp only [CompiledModule.SupportedOperandsNG] at hops <;>
        simp only [CompiledModule.SupportedOperands] <;>
        first
        | trivial
        | (intro tys htl
           obtain rfl : tys = [] := List.eq_nil_of_length_eq_zero (by simpa using htl)
           exact hops)
        | (simp [Bytecode.inSubsetNG, Bytecode.inCore, Bytecode.inDataByValue,
            Bytecode.inReferences] at hsub)

/-! ## D1 and C1–C2 for programs by value -/

/-- **D1** for programs by value on the supported subset, generic code
included: `Soundness` with the premise `P.ByValue`. -/
def SoundnessByValueGen (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.Supported →
    P.ByValue → NativesRespect P N → AdmissibleInitial P s₀ → NoWrong P N s₀

/-- **D1** for programs by value, generic code included: a program the model
accepts, by value on the supported subset, never goes wrong, whatever its
natives do within their contracts. -/
theorem soundness_byValue_gen : SoundnessByValueGen verifier :=
  fun _ _ _ hwf _ hs hbv hN h0 =>
    noWrong_of_certified (certified_of_wellFormed_gen schedulerSound hwf hs) hbv hN h0

/-- **C1, C2** for programs by value, generic code included: the
certificates are consistent and parametric, and every execution agrees with
them (`CertificatesHold` with the premise `P.ByValue`). -/
theorem certificatesHold_byValue_gen (P : Program)
    (hwf : WellFormedBy verifier VerifierConfig.production P) (_ : P.SupportedVersions)
    (hs : P.Supported) (hbv : P.ByValue) :
    ∃ (tc : TypeCertificate) (ac : AvailabilityCertificate),
      tc.Consistent P ∧ tc.Parametric P ∧ ac.Consistent P ∧
      ∀ N, NativesRespect P N → StaticTypes P N tc ∧ StaticAvailability P N ac :=
  let hc := certified_of_wellFormed_gen schedulerSound hwf hs
  ⟨modelTypes _, modelAvail _, hc.types, modelTypes_parametric P, hc.availability,
    fun _ hN => certificates_agree hc hbv hN⟩

/-! ## The non-generic milestones, as corollaries -/

/-- A program of the non-generic subset the model accepts is certified by the
model's own analyses. -/
theorem certified_of_wellFormed (hsched : SchedulerSound) {P : Program}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.SupportedNonGeneric) :
    Certified P (modelTypes P) (modelAvail P) :=
  certified_of_wellFormed_gen hsched hwf (supported_of_nonGeneric hwf hs)

/-- **D1** for programs by value of the non-generic subset. -/
def SoundnessByValue (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric →
    P.ByValue → NativesRespect P N → AdmissibleInitial P s₀ → NoWrong P N s₀

theorem soundness_byValue : SoundnessByValue verifier := fun P N s₀ hwf hv hs hbv hN h0 =>
  soundness_byValue_gen P N s₀ hwf hv (supported_of_nonGeneric hwf hs) hbv hN h0

/-- **C1, C2** (static) on the non-generic subset. -/
theorem certificatesConsistent (P : Program)
    (hwf : WellFormedBy verifier VerifierConfig.production P) (_ : P.SupportedVersions)
    (hs : P.SupportedNonGeneric) : (modelTypes P).Consistent P ∧ (modelAvail P).Consistent P :=
  let hc := certified_of_wellFormed schedulerSound hwf hs
  ⟨hc.types, hc.availability⟩

/-- **C1, C2** for programs by value of the non-generic subset. -/
theorem certificatesHold_byValue (P : Program)
    (hwf : WellFormedBy verifier VerifierConfig.production P) (_ : P.SupportedVersions)
    (hs : P.SupportedNonGeneric) (hbv : P.ByValue) :
    ∃ (tc : TypeCertificate) (ac : AvailabilityCertificate),
      tc.Consistent P ∧ ac.Consistent P ∧
      ∀ N, NativesRespect P N → StaticTypes P N tc ∧ StaticAvailability P N ac :=
  let hc := certified_of_wellFormed schedulerSound hwf hs
  ⟨modelTypes _, modelAvail _, hc.types, hc.availability,
    fun _ hN => certificates_agree hc hbv hN⟩

end MoveBytecode.Proofs
