-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefRuntime
import Proofs.Soundness
import Proofs.SoundnessGen
import Proofs.RefTypedGeneric

/-!
# The reference certificate of a program

UNTRUSTED (checked by the kernel). The milestone restrictions M1
(`Program.M1`), M2 (`Program.M2`) and M3 (`Program.M3`), the reference-safety
analysis's state
before each instruction of each function (`modelRef`), and what an accepted
M3 program's certificate says at each reachable instruction (`refCert`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

/-- **Milestone M1** for a module: every function handle takes and returns
values and is no struct-API mutable borrow, and no code borrows global
storage. -/
def _root_.MoveBytecode.CompiledModule.M1 (m : CompiledModule) : Prop :=
  (∀ fh ∈ m.functionHandles.toList,
    (∀ ps, m.signatures[fh.parameters]? = some ps → ∀ t ∈ ps, refKind t = none) ∧
    (∀ rs, m.signatures[fh.returns]? = some rs → ∀ t ∈ rs, refKind t = none) ∧
    borrowFieldMutable fh.attributes = none) ∧
  ∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code → ∀ i ∈ code.code.toList,
    ∀ x, i ≠ .mutBorrowGlobal x ∧ i ≠ .immBorrowGlobal x

/-- **Milestone M1** for a program: every module. -/
def _root_.MoveBytecode.Program.M1 (P : Program) : Prop := ∀ m ∈ P.modules.toList, m.M1

/-- **Milestone M2** for a module: no function handle is a struct-API mutable
borrow, and no code borrows global storage. References may cross calls. -/
def _root_.MoveBytecode.CompiledModule.M2 (m : CompiledModule) : Prop :=
  (∀ fh ∈ m.functionHandles.toList, borrowFieldMutable fh.attributes = none) ∧
  ∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code → ∀ i ∈ code.code.toList,
    ∀ x, i ≠ .mutBorrowGlobal x ∧ i ≠ .immBorrowGlobal x

/-- **Milestone M2** for a program: every module. -/
def _root_.MoveBytecode.Program.M2 (P : Program) : Prop := ∀ m ∈ P.modules.toList, m.M2

theorem _root_.MoveBytecode.Program.M1.toM2 {P : Program} (h : P.M1) : P.M2 :=
  fun m hm => ⟨fun fh hfh => ((h m hm).1 fh hfh).2.2, (h m hm).2⟩

/-- **Milestone M3** for a module: no function handle is a struct-API mutable
borrow. References cross calls, and code borrows global storage. -/
def _root_.MoveBytecode.CompiledModule.M3 (m : CompiledModule) : Prop :=
  ∀ fh ∈ m.functionHandles.toList, borrowFieldMutable fh.attributes = none

/-- **Milestone M3** for a program: every module. -/
def _root_.MoveBytecode.Program.M3 (P : Program) : Prop := ∀ m ∈ P.modules.toList, m.M3

theorem _root_.MoveBytecode.Program.M2.toM3 {P : Program} (h : P.M2) : P.M3 :=
  fun m hm => (h m hm).1

/-- The bound `verifyReferenceSafety` gives the abstract interpreter. -/
def refBound (initial : RefState) : Nat :=
  (initial.locals.length + 1) * (initial.locals.length + 1) * (maxEdgeSetSize + 1) +
    (initial.locals.length + 1)

theorem verifyReferenceSafety_eq (m : CompiledModule) (view : FunctionView) :
    verifyReferenceSafety m view = (do
      let initial ← RefState.new view
      let _ ← analyzeFunction (referenceAnalysis m view) view.cfg view.code.code initial
        (refBound initial)
      pure ()) := rfl

/-- **The reference certificate**: the reference-safety analysis's state
before each instruction of each function with code. -/
def modelRef (P : Program) (f : FunctionRef) (pc : Nat) : Option RefState :=
  match P.function? f with
  | none => none
  | some info =>
    match info.definition.code with
    | none => none
    | some code =>
      match Cfg.build code.code with
      | .error _ => none
      | .ok cfg =>
        match FunctionView.function info.module f.index code info.handle cfg with
        | .error _ => none
        | .ok view =>
          match RefState.new view with
          | .error _ => none
          | .ok initial =>
            match analyzeFunction (referenceAnalysis info.module view) view.cfg view.code.code
                initial (refBound initial) with
            | .error _ => none
            | .ok inv => refBefore info.module view code.code inv pc

/-- A function the model accepts passed reference safety. -/
theorem function_refSafe {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit} (hf : P.function? f = some info)
    (hcode : info.definition.code = some code) :
    ∃ cfg view, verifyControlFlow VerifierConfig.production info.module.version code = .ok cfg ∧
      FunctionView.function info.module f.index code info.handle cfg = .ok view ∧
      verifyReferenceSafety info.module view = .ok () := by
  obtain ⟨hm, -, hd, hfh⟩ := function?_parts hf
  obtain ⟨-, -, -, hcu⟩ := verifyModule_ok (wellFormed_module hwf hm).1
  obtain ⟨n, hn⟩ := verifyCodeUnits_fn hcu hd
  unfold verifyFunction at hn
  rw [hcode] at hn
  simp only [bind_ok, lookup_ok] at hn
  obtain ⟨handle, hh, cfg, hcf, view, hview, hn⟩ := hn
  rw [hfh, Option.some.injEq] at hh
  subst hh
  refine ⟨cfg, view, hcf, hview, ?_⟩
  repeat' split at hn
  all_goals first
    | (simp [reject, bind, Except.bind] at hn; done)
    | skip
  all_goals
    simp only [bind_ok] at hn
    obtain ⟨u1, -, u2, -, inv, -, u3, hrs, -⟩ := hn
    cases u3
    exact hrs

theorem function_m2 {P : Program} (hm2 : P.M2) {f : FunctionRef} {info : FunctionInfo}
    (hf : P.function? f = some info) : info.module.M2 :=
  hm2 _ (module_mem (function?_parts hf).1)

theorem function_m3 {P : Program} (hm3 : P.M3) {f : FunctionRef} {info : FunctionInfo}
    (hf : P.function? f = some info) : info.module.M3 :=
  hm3 _ (module_mem (function?_parts hf).1)

/-- **The struct-API mutable borrow handles of a program have a wrapper's
signature**: one parameter and one mutable reference result (`ApiSig`). -/
def _root_.MoveBytecode.Program.ApiSigs (P : Program) : Prop :=
  ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    borrowFieldMutable fh.attributes = some off → ApiSig m fh

/-- An M3 program has no struct-API mutable borrow handle. -/
theorem _root_.MoveBytecode.Program.M3.apiSigs {P : Program} (h : P.M3) : P.ApiSigs :=
  fun m hm fh hfh off hoff => by rw [h m hm fh hfh] at hoff; cases hoff

section Setting

/-- The type arguments a supported instruction names are no references. -/
theorem instNonRef_of_supported {m : CompiledModule} {instr : Bytecode} {n : Nat}
    (hsub : instr.inSubset = true) (h : m.SupportedOperands instr n) : InstNonRef m instr := by
  intro s hs sig hsig t ht
  have key : ∀ {x : SignatureIndex}, m.SupportedValueSignature x n → x = s →
      refKind t = none := by
    rintro x ⟨sig', hsig', hall⟩ rfl
    rw [hsig, Option.some.injEq] at hsig'
    subst hsig'
    obtain ⟨us, hus⟩ := Option.isSome_iff_exists.1 (hall (List.replicate n .bool) (by simp))
    obtain ⟨u, hu⟩ := ofTokenListWith_some_mem hus t ht
    exact refKind_of_ofTokenWith hu
  cases instr <;> simp only [instSig?, Option.map_eq_some_iff] at hs <;>
    simp only [CompiledModule.SupportedOperands] at h <;>
    first
    | (simp at hs; done)
    | (obtain ⟨x, hx, rfl⟩ := hs
       obtain ⟨x', hx', hv⟩ := h
       rw [hx, Option.some.injEq] at hx'
       subst hx'
       exact key hv rfl)
    | (simp [Bytecode.inSubset, Bytecode.inCore, Bytecode.inDataByValue, Bytecode.inReferences,
        Bytecode.inGenerics] at hsub)

variable {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
  (hs : P.Supported) (hapi : P.ApiSigs)

include hwf hs hapi in
/-- **The setting of the abstract invariant** for an instance of a function of
an accepted program whose struct-API mutable borrow handles have a wrapper's
signature, generic code included. -/
theorem refSetting_gen {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit}
    (hf : P.function? f = some info) (hcode : info.definition.code = some code)
    (hok : P.typeArgsOk f = true) {cfg : Cfg} {view : FunctionView}
    (hcf : verifyControlFlow VerifierConfig.production info.module.version code = .ok cfg)
    (hview : FunctionView.function info.module f.index code info.handle cfg = .ok view) :
    RefSetting P f info.module view (modelAvail P f) := by
  obtain ⟨hvc, -, hvcfg, -, hrs, -, -⟩ := functionView_fields hview
  obtain ⟨-, ⟨cfg', view', hcf', hview', hsu, hts, -⟩⟩ := function_verified hwf hf hcode
  rw [hcf, Except.ok.injEq] at hcf'
  subst hcf'
  rw [hview, Except.ok.injEq] at hview'
  subst hview'
  obtain ⟨hvc', hvh, -, -, -, -, -⟩ := functionView_fields hview
  have ha := genCtx_of hwf hs hf hcode hok hview
  have hacc := (wellFormed_module hwf (function?_parts hf).1).1
  have hbounds := (verifyModule_ok hacc).1
  have hblocks : view.cfg.blocks = buildBlocks view.code.code := by
    rw [hvcfg, verifyControlFlow_blocks hcf, hvc]
  have hfall := verifyControlFlow_fallthrough hcf
  have hlt : ∀ pc, ∀ s ∈ successors code.code pc, s < code.code.size :=
    successors_lt hfall (fun pc o i hi ho => branch_targets hbounds
      (getElem?_mem_toList (function?_parts hf).2.2.1) hcode hi ho)
  have hsub : ∀ i ∈ view.code.code.toList,
      i.inSubset = true ∧ info.module.SupportedOperands i view.handle.typeParameters.length := by
    obtain ⟨-, -, -, -, hdefs⟩ := hs.1 _ (module_mem (function?_parts hf).1)
    obtain ⟨fh, hfh, -, hcodes⟩ :=
      hdefs _ (getElem?_mem_toList (function?_parts hf).2.2.1) code hcode
    rw [(function?_parts hf).2.2.2, Option.some.injEq] at hfh
    subst hfh
    rw [hvc, hvh]
    exact hcodes
  have hl9 : ∀ i ∈ view.code.code.toList, i.OperandsConsistent info.module := by
    rw [hvc]
    exact instructionConsistency_of_accepted hacc _ (getElem?_mem_toList (function?_parts hf).2.2.1)
      code hcode
  have hl5 : ∀ i ∈ view.code.code.toList,
      i.TypeOperandsWellFormed info.module view.handle.typeParameters := by
    obtain ⟨-, -, -, htf⟩ := typeFormation_of_accepted hacc
    obtain ⟨fh, hfh, -, hops⟩ := htf _ (getElem?_mem_toList (function?_parts hf).2.2.1) code hcode
    rw [(function?_parts hf).2.2.2, Option.some.injEq] at hfh
    subst hfh
    rw [hvc, hvh]
    exact hops
  have hcl : ∀ i ∈ view.code.code.toList, ClosureOk info.module i := by
    rw [hvc]
    exact closureOk_of_accepted hacc (getElem?_mem_toList (function?_parts hf).2.2.1) hcode
  have hcert := typeCertificate (effectsAgree_of_typeSafety hrs hcl hts hblocks) hcl hblocks hsu hts
  have hat := types_at_gen ha hbounds hsub hcl hl9 hl5 hcert
  obtain ⟨-, -, -, havl⟩ := modelAvail_consistent schedulerSound hwf f info code hf hcode
  subst hvc
  refine {
    agrees := ha
    bounds := hbounds
    blocks := hblocks
    nonempty := nonempty_of_verified (function_verified hwf hf hcode)
    succLt := hlt
    fall := hfall
    types := fun pc hpc => ?_
    availRead := fun pc instr hr hi => (havl pc instr hr hi).1
    availSucc := fun pc instr hr hi => (havl pc instr hr hi).2
    m2 := fun pc instr hi => ?_
    ops := fun pc instr hi =>
      instNonRef_of_supported (hsub _ (getElem?_mem_toList hi)).1
        (hsub _ (getElem?_mem_toList hi)).2 }
  · obtain ⟨st, st', before, -, -, -, -, h1, hb, h2, -, -, -, hfl, hend⟩ := hat pc hpc
    exact ⟨st, st', before, h1, hb, h2, hfl, hend⟩
  · cases instr
    case ret => exact Or.inl rfl
    case call h =>
      right
      intro fh hfh
      exact hapi _ (module_mem (function?_parts hf).1) fh (getElem?_mem_toList hfh)
    case callGeneric i =>
      right
      intro fi fh _ hfh
      exact hapi _ (module_mem (function?_parts hf).1) fh (getElem?_mem_toList hfh)
    all_goals exact Or.inr trivial

/-- What the reference certificate says of one function of an accepted M3
program: its entry covers the analysis's initial state, and before each
reachable instruction it holds a well-typed state whose transfer, at each
successor, is covered by the certificate there. -/
structure RefCertOk (P : Program) (f : FunctionRef) (m : CompiledModule) (code : CodeUnit)
    (view : FunctionView) : Prop where
  setting : RefSetting P f m view (modelAvail P f)
  code_eq : view.code = code
  entry : ∃ initial A0, RefState.new view = .ok initial ∧ modelRef P f 0 = some A0 ∧
    RefLe initial A0
  at_ : ∀ pc instr, CodeReachable code.code pc → code.code[pc]? = some instr →
    ∃ A stys, modelRef P f pc = some A ∧ typesBefore m view pc = some stys ∧
      Typed (view.parameters ++ view.locals) stys (modelAvail P f pc) A ∧
      (∃ A', (executeInner m view instr).run A = .ok ((), A')) ∧
      (instr ≠ .ret → ∃ A', (executeInner m view instr).run A = .ok ((), A') ∧
        ∀ pc' ∈ successors code.code pc, ∃ A'', modelRef P f pc' = some A'' ∧ RefLe A' A'')

include hwf hs hapi in
/-- **The reference certificate of an instance** of a function of an accepted
program, generic code included, whose struct-API
mutable borrow handles have a wrapper's signature. -/
theorem refCert_gen {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit}
    (hf : P.function? f = some info) (hcode : info.definition.code = some code)
    (hok : P.typeArgsOk f = true) :
    ∃ cfg view, verifyControlFlow VerifierConfig.production info.module.version code = .ok cfg ∧
      FunctionView.function info.module f.index code info.handle cfg = .ok view ∧
      RefCertOk P f info.module code view := by
  obtain ⟨cfg, view, hcf, hview, hrs⟩ := function_refSafe hwf hf hcode
  have hset := refSetting_gen hwf hs hapi hf hcode hok hcf hview
  obtain ⟨hvc, -, hvcfg, hps, -, -, -⟩ := functionView_fields hview
  have hbuild0 := verifyControlFlow_build hcf
  have hbuild : Cfg.build view.code.code = .ok view.cfg := by rw [hvc, hvcfg]; exact hbuild0
  obtain ⟨params, hparams, hent, havl⟩ := modelAvail_consistent schedulerSound hwf f info code hf hcode
  rw [hps, Option.some.injEq] at hparams
  subst hparams
  have havail0 : ∀ i, modelAvail P f 0 i = .available → i < view.parameters.length := by
    intro i hi
    have := refines_avail (hent i) hi
    split at this
    · assumption
    · simp at this
  rw [verifyReferenceSafety_eq] at hrs
  simp only [bind_ok] at hrs
  obtain ⟨initial, hinit, inv, hana, -⟩ := hrs
  have hJ0 : RefJ view (modelAvail P f) 0 initial :=
    ⟨zero_mem_blockIds _, ⟨[0], .nil 0⟩, typed_new_general hinit havail0⟩
  have hfix := analyze_postFixpoint (refJoinSound info.module) (refJClosed hset) hJ0 hbuild hana
  have hJ := analyze_justified (RefJ view (modelAvail P f)) hJ0 (refJClosed hset).exec
    (refJClosed hset).join hana
  have hmodel : ∀ pc, modelRef P f pc = refBefore info.module view code.code inv pc := by
    intro pc
    simp only [modelRef, hf, hcode, hbuild0, hview, hinit, hana]
  refine ⟨cfg, view, hcf, hview, hset, hvc, ?_, fun pc instr hreach hi => ?_⟩
  · obtain ⟨pre, hpre, hle⟩ := hfix.entry
    exact ⟨initial, pre, hinit, by rw [hmodel]; exact hpre, hle⟩
  · subst hvc
    obtain ⟨b, hb, h1, h2, pre, hpre⟩ :=
      ref_analyzed_at hset.blocks hfix hset.nonempty hset.succLt hreach
    obtain ⟨st, stys, hbefore, htys, ht, -⟩ :=
      ref_typed_at hset hfix hb hpre (hJ _ _ hpre) h1 h2
    obtain ⟨st0, st', instr', hi', hbefore', hx, hnext, hend⟩ := ref_at hset.blocks hfix hb hpre h1 h2
    rw [hi, Option.some.injEq] at hi'
    subst hi'
    rw [hbefore, Option.some.injEq] at hbefore'
    subst hbefore'
    refine ⟨st, stys, by rw [hmodel]; exact hbefore, htys, ht, ⟨st', hx⟩, fun hnr => ⟨st', hx, ?_⟩⟩
    have hok := (buildBlocks_spec view.code.code).1 b hb
    intro pc' hpc'
    rcases Nat.lt_or_ge pc b.exit with hlt | hge
    · rw [inner_successor hok h1 hlt hi hpc']
      exact ⟨st', by rw [hmodel]; exact hnext hlt, RefLe.refl _⟩
    · have hpe : pc = b.exit := by omega
      obtain ⟨-, hsucc⟩ := hend hpe
      subst hpe
      have hslt := hset.succLt _ pc' hpc'
      have hys : pc' ∈ b.successors := by
        rw [hok.successors]; exact (getSuccessors_mem hset.fall).2 hpc'
      obtain ⟨pre', hpre', hle⟩ := hsucc pc' hys
      obtain ⟨stys', -, ht', -, hlast⟩ := refSetting_step hset hreach hi hnr htys ht hx
      have hend' := hok.exit_end
      simp only [isEndOfBlock, Bool.or_eq_true, beq_iff_eq, List.contains_iff_mem] at hend'
      have hnil : stys' = [] := hlast hend'
      subst hnil
      refine ⟨pre', ?_, (canon_le ht').trans hle⟩
      rw [hmodel, refBefore_of_mem _ _ (exit_successor_mem hok hi hpc' hslt)]
      exact hpre'

end Setting

/-- **The setting of the abstract invariant** for an instance of a function of
an accepted program of the non-generic subset. -/
theorem refSetting_of' {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.SupportedNonGeneric) (hapi : P.ApiSigs) {f : FunctionRef} {info : FunctionInfo}
    {code : CodeUnit} (hf : P.function? f = some info) (hcode : info.definition.code = some code)
    (hok : P.typeArgsOk f = true) {cfg : Cfg} {view : FunctionView}
    (hcf : verifyControlFlow VerifierConfig.production info.module.version code = .ok cfg)
    (hview : FunctionView.function info.module f.index code info.handle cfg = .ok view) :
    RefSetting P f info.module view (modelAvail P f) :=
  refSetting_gen hwf (supported_of_nonGeneric hwf hs) hapi hf hcode hok hcf hview

/-- **The reference certificate of an instance** of a function of an accepted
program of the non-generic subset. -/
theorem refCert' {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.SupportedNonGeneric) (hapi : P.ApiSigs) {f : FunctionRef} {info : FunctionInfo}
    {code : CodeUnit} (hf : P.function? f = some info) (hcode : info.definition.code = some code)
    (hok : P.typeArgsOk f = true) :
    ∃ cfg view, verifyControlFlow VerifierConfig.production info.module.version code = .ok cfg ∧
      FunctionView.function info.module f.index code info.handle cfg = .ok view ∧
      RefCertOk P f info.module code view :=
  refCert_gen hwf (supported_of_nonGeneric hwf hs) hapi hf hcode hok

/-- **The setting of the abstract invariant** for a function of an accepted M3
program. -/
theorem refSetting_of {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.SupportedNonGeneric) (hm3 : P.M3) {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit}
    (hf : P.function? f = some info) (hcode : info.definition.code = some code)
    (hok : P.typeArgsOk f = true) {cfg : Cfg} {view : FunctionView}
    (hcf : verifyControlFlow VerifierConfig.production info.module.version code = .ok cfg)
    (hview : FunctionView.function info.module f.index code info.handle cfg = .ok view) :
    RefSetting P f info.module view (modelAvail P f) :=
  refSetting_of' hwf hs hm3.apiSigs hf hcode hok hcf hview

/-- **The reference certificate of an accepted M3 program.** -/
theorem refCert {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.SupportedNonGeneric) (hm3 : P.M3) {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit}
    (hf : P.function? f = some info) (hcode : info.definition.code = some code)
    (hok : P.typeArgsOk f = true) :
    ∃ cfg view, verifyControlFlow VerifierConfig.production info.module.version code = .ok cfg ∧
      FunctionView.function info.module f.index code info.handle cfg = .ok view ∧
      RefCertOk P f info.module code view :=
  refCert' hwf hs hm3.apiSigs hf hcode hok

end MoveBytecode.Proofs
