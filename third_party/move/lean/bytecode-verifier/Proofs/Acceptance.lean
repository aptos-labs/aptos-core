-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.StackUsage
import Proofs.Constants

/-!
# What the model's acceptance gives

UNTRUSTED (checked by the kernel). From a program the model accepts
(`WellFormedBy Model.verifier`): each module passed bounds, duplication,
constants and code-unit verification, and each function with code passed
control flow, stack usage, type safety and locals safety. Then the
consequences used by the soundness proof that need only these facts: every
function's code is nonempty and every successor of every instruction is in
the code (L3), and, on the supported subset, every function's locals have
types.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## From the program's acceptance to each module and function -/

theorem accepts_iff {x : VResult Unit} : accepts x = true ↔ x = .ok () := by
  cases x with
  | ok u => cases u; simp [accepts]
  | error e => simp [accepts]

theorem verifyModule_ok {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) :
    verifyBounds m = .ok () ∧ verifyDuplication m = .ok () ∧ verifyConstants m = .ok () ∧
      verifyCodeUnits VerifierConfig.production m = .ok () := by
  unfold verifyModule at h
  simp only [show (VerifierConfig.production.scope == .nothing) = false from rfl,
    Bool.false_eq_true, ↓reduceIte, bind_ok] at h
  obtain ⟨u1, h1, u2, -, u3, -, u4, h4, u5, -, u6, -, u7, h7, u8, -, u9, -, u10, -, h11⟩ := h
  cases u1; cases u4; cases u7
  exact ⟨h1, h4, h7, h11⟩

/-- The instruction-consistency pass of a module the model accepts. -/
theorem verifyModule_instructions {m : CompiledModule}
    (h : verifyModule VerifierConfig.production m = .ok ()) :
    verifyInstructionConsistency m = .ok () := by
  unfold verifyModule at h
  simp only [show (VerifierConfig.production.scope == .nothing) = false from rfl,
    Bool.false_eq_true, ↓reduceIte, bind_ok] at h
  obtain ⟨-, -, -, -, -, -, -, -, -, -, u6, h6, -⟩ := h
  cases u6
  exact h6

theorem wellFormed_module {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {i : Nat} {m : CompiledModule} (hm : P.modules[i]? = some m) :
    verifyModule VerifierConfig.production m = .ok () ∧
      verifyLinking VerifierConfig.production m (P.others i) = .ok () := by
  obtain ⟨h1, h2⟩ := hwf.2 i m hm
  exact ⟨accepts_iff.1 h1, accepts_iff.1 h2⟩

theorem verifyCodeUnits_fn {config : VerifierConfig} {m : CompiledModule}
    (h : verifyCodeUnits config m = .ok ()) {index : Nat} {fdef : FunctionDefinition}
    (hd : m.functionDefs[index]? = some fdef) : ∃ n, verifyFunction config m index fdef = .ok n := by
  unfold verifyCodeUnits at h
  simp only [bind_ok] at h
  obtain ⟨ctx?, -, total, hfold, -⟩ := h
  have hmem : (fdef, index) ∈ m.functionDefs.toList.zipIdx := by
    rw [List.mem_zipIdx_iff_getElem?]
    simpa using hd
  obtain ⟨acc, r, hr⟩ := foldlM_ok_mem hfold _ hmem
  simp only [bind_ok] at hr
  obtain ⟨u, -, n, hn, -⟩ := hr
  exact ⟨n, hn⟩

/-- The passes a function with code passed. -/
structure FunctionVerified (config : VerifierConfig) (m : CompiledModule) (index : Nat)
    (fdef : FunctionDefinition) (code : CodeUnit) (handle : FunctionHandle) : Prop where
  hasHandle : m.functionHandles[fdef.function]? = some handle
  views : ∃ cfg view, verifyControlFlow config m.version code = .ok cfg ∧
    FunctionView.function m index code handle cfg = .ok view ∧
    verifyStackUsage config m view = .ok () ∧ verifyTypeSafety m view = .ok () ∧
    ∃ inv, verifyLocalsSafety m view = .ok inv

theorem verifyFunction_parts {config : VerifierConfig} {m : CompiledModule} {index n : Nat}
    {fdef : FunctionDefinition} {code : CodeUnit} (hcode : fdef.code = some code)
    (h : verifyFunction config m index fdef = .ok n) :
    ∃ handle, FunctionVerified config m index fdef code handle := by
  unfold verifyFunction at h
  rw [hcode] at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨handle, hh, cfg, hcf, view, hview, h⟩ := h
  refine ⟨handle, hh, cfg, view, hcf, hview, ?_⟩
  repeat' split at h
  all_goals first
    | (simp [reject, bind, Except.bind] at h; done)
    | skip
  all_goals
    simp only [bind_ok] at h
    obtain ⟨u1, hsu, u2, hts, inv, hls, -⟩ := h
    cases u1; cases u2
    exact ⟨hsu, hts, inv, hls⟩

/-- A function with code passed the acquires verifier. -/
theorem verifyFunction_acquires {config : VerifierConfig} {m : CompiledModule} {index n : Nat}
    {fdef : FunctionDefinition} {code : CodeUnit} (hcode : fdef.code = some code)
    (h : verifyFunction config m index fdef = .ok n) : verifyAcquires m fdef code = .ok () := by
  unfold verifyFunction at h
  rw [hcode] at h
  simp only [bind_ok, lookup_ok] at h
  obtain ⟨handle, hh, cfg, hcf, view, hview, h⟩ := h
  repeat' split at h
  all_goals first
    | (simp [reject, bind, Except.bind] at h; done)
    | skip
  all_goals
    simp only [bind_ok] at h
    obtain ⟨u1, -, u2, -, inv, -, u3, -, u4, hacq, -⟩ := h
    cases u4
    exact hacq

theorem function?_parts {P : Program} {f : FunctionRef} {info : FunctionInfo}
    (h : P.function? f = some info) :
    P.modules[f.module]? = some info.module ∧ info.module.selfId = some info.moduleId ∧
      info.module.functionDefs[f.index]? = some info.definition ∧
      info.module.functionHandles[info.definition.function]? = some info.handle := by
  simp only [Program.function?, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
  obtain ⟨m, hm, id, hid, d, hd, fh, hfh, rfl⟩ := h
  exact ⟨hm, hid, hd, hfh⟩

/-- Every function with code of an accepted program passed the code-unit
passes, with its own handle. -/
theorem function_verified {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit} (hf : P.function? f = some info)
    (hcode : info.definition.code = some code) :
    FunctionVerified VerifierConfig.production info.module f.index info.definition code
      info.handle := by
  obtain ⟨hm, -, hd, hfh⟩ := function?_parts hf
  obtain ⟨-, -, -, hcu⟩ := verifyModule_ok (wellFormed_module hwf hm).1
  obtain ⟨n, hn⟩ := verifyCodeUnits_fn hcu hd
  obtain ⟨handle, hv⟩ := verifyFunction_parts hcode hn
  have : handle = info.handle := by rw [hv.hasHandle] at hfh; exact Option.some.inj hfh
  subst this
  exact hv

/-- Every function with code of an accepted program passed the acquires
verifier. -/
theorem function_acquires {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {f : FunctionRef} {info : FunctionInfo} {code : CodeUnit} (hf : P.function? f = some info)
    (hcode : info.definition.code = some code) :
    verifyAcquires info.module info.definition code = .ok () := by
  obtain ⟨hm, -, hd, -⟩ := function?_parts hf
  obtain ⟨-, -, -, hcu⟩ := verifyModule_ok (wellFormed_module hwf hm).1
  obtain ⟨n, hn⟩ := verifyCodeUnits_fn hcu hd
  exact verifyFunction_acquires hcode hn

/-! ## The supported subset -/

theorem module_mem {P : Program} {i : Nat} {m : CompiledModule} (h : P.modules[i]? = some m) :
    m ∈ P.modules.toList := by
  rw [Array.mem_toList_iff]
  exact Array.mem_of_getElem? h

theorem function_supported {P : Program} (hs : P.SupportedNonGeneric) {f : FunctionRef}
    {info : FunctionInfo} (hf : P.function? f = some info) : info.module.SupportedNonGeneric :=
  hs _ (module_mem (function?_parts hf).1)

theorem getElem?_mem_toList {α : Type} {a : Array α} {i : Nat} {x : α} (h : a[i]? = some x) :
    x ∈ a.toList := by
  rw [Array.mem_toList_iff]
  exact Array.mem_of_getElem? h

theorem localTypes_some {P : Program} (hs : P.SupportedNonGeneric) {f : FunctionRef} {info : FunctionInfo}
    {code : CodeUnit} (hf : P.function? f = some info) (hcode : info.definition.code = some code) :
    ∃ tys, P.localTypes f = some tys := by
  obtain ⟨hhandles, -, -, -, -, hdefs⟩ := function_supported hs hf
  obtain ⟨-, -, hd, hfh⟩ := function?_parts hf
  obtain ⟨-, ⟨psig, hpsig, hps⟩, -⟩ := hhandles _ (getElem?_mem_toList hfh)
  obtain ⟨⟨lsig, hlsig, hls⟩, -⟩ := hdefs _ (getElem?_mem_toList hd) code hcode
  obtain ⟨ps, hps⟩ := Option.isSome_iff_exists.1 hps
  obtain ⟨ls, hls⟩ := Option.isSome_iff_exists.1 hls
  refine ⟨ps ++ ls, ?_⟩
  rw [localTypes_nonGeneric hs]
  simp [Program.localTypes, function?_typeArgs, hf, hcode, hpsig, hps, hlsig, hls]

/-! ## Code structure -/

theorem verifyControlFlow_fallthrough {config : VerifierConfig} {version : Nat} {code : CodeUnit}
    {cfg : Cfg} (h : verifyControlFlow config version code = .ok cfg) :
    ∃ last, code.code.back? = some last ∧ isUnconditionalBranch last = true := by
  unfold verifyControlFlow at h
  by_cases hv : version ≤ 5
  · simp [hv, panicAt, bind, Except.bind] at h
  · simp only [hv, if_false, bind_ok] at h
    obtain ⟨u, hf, -⟩ := h
    unfold verifyFallthrough at hf
    split at hf
    · simp [reject] at hf
    · next last hlast =>
      refine ⟨last, hlast, ?_⟩
      cases hu : isUnconditionalBranch last <;> simp_all [reject]

theorem nonempty_of_verified {config : VerifierConfig} {m : CompiledModule} {index : Nat}
    {fdef : FunctionDefinition} {code : CodeUnit} {handle : FunctionHandle}
    (hv : FunctionVerified config m index fdef code handle) : 0 < code.code.size := by
  obtain ⟨cfg, -, hcf, -⟩ := hv.views
  obtain ⟨last, hlast, -⟩ := verifyControlFlow_fallthrough hcf
  rcases Nat.eq_zero_or_pos code.code.size with h | h
  · simp [Array.back?, h] at hlast
  · exact h


theorem seq_ok {x y : VResult Unit} (h : (x >>= fun _ => y) = .ok ()) : y = .ok () := by
  cases x <;> simp_all [bind, Except.bind]

theorem seq_ok2 {x y : VResult Unit} (h : (x >>= fun _ => y) = .ok ()) :
    x = .ok () ∧ y = .ok () := by
  cases x with
  | error e => simp [bind, Except.bind] at h
  | ok u => cases u; exact ⟨rfl, h⟩

theorem checkIdx_ok {n i : Nat} (h : checkIdx n i = .ok ()) : i < n := by
  unfold checkIdx at h
  split at h
  · simp [reject] at h
  · next hlt => exact Nat.lt_of_not_le hlt

/-- Bounds checking validates every field handle: its owner is a struct
definition with that many fields. -/
theorem verifyBounds_fieldHandle {m : CompiledModule} (hb : verifyBounds m = .ok ()) {h : Nat}
    {fh : FieldHandle} (hfh : m.fieldHandles[h]? = some fh) :
    ∃ sd, m.structDefs[fh.owner]? = some sd ∧ fh.field < fieldCount sd.fieldInformation none := by
  unfold verifyBounds at hb
  simp only at hb
  split at hb
  · simp [reject, bind, Except.bind] at hb
  iterate 6 obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨hf, -⟩ := seq_ok2 hb
  have h := forM_ok hf fh (getElem?_mem_toList hfh)
  unfold Bounds.checkFieldHandle at h
  obtain ⟨h1, h⟩ := seq_ok2 h
  obtain ⟨sd, hsd⟩ : ∃ sd, m.structDefs[fh.owner]? = some sd :=
    ⟨_, Array.getElem?_eq_getElem (checkIdx_ok h1)⟩
  simp only [hsd] at h
  refine ⟨sd, hsd, ?_⟩
  split at h
  · simp [reject] at h
  · next hlt => exact Nat.lt_of_not_le hlt

/-- Bounds checking validates every variant field handle: it lists at least
one variant, and each is a variant of its struct definition with that many
fields. -/
theorem verifyBounds_variantFieldHandle {m : CompiledModule} (hb : verifyBounds m = .ok ())
    {h : Nat} {fh : VariantFieldHandle} (hfh : m.variantFieldHandles[h]? = some fh) :
    ∃ sd, m.structDefs[fh.structIndex]? = some sd ∧ fh.variants ≠ [] ∧
      ∀ v ∈ fh.variants, v < variantCount sd.fieldInformation ∧
        fh.field < fieldCount sd.fieldInformation (some v) := by
  unfold verifyBounds at hb
  simp only at hb
  split at hb
  · simp [reject, bind, Except.bind] at hb
  iterate 12 obtain ⟨-, hb⟩ := seq_ok2 hb
  obtain ⟨hf, -⟩ := seq_ok2 hb
  have h := forM_ok hf fh (getElem?_mem_toList hfh)
  unfold Bounds.checkVariantFieldHandle at h
  obtain ⟨h1, h⟩ := seq_ok2 h
  obtain ⟨sd, hsd⟩ : ∃ sd, m.structDefs[fh.structIndex]? = some sd :=
    ⟨_, Array.getElem?_eq_getElem (checkIdx_ok h1)⟩
  have hne : fh.variants ≠ [] := by
    intro hn
    simp [hn, reject, bind, Except.bind] at h
  have hne' : fh.variants.isEmpty = false := by simpa using hne
  simp only [hne', Bool.false_eq_true, ↓reduceIte, bind_ok, lookup_ok, hsd] at h
  obtain ⟨sd', hsd', h⟩ := h
  cases hsd'
  refine ⟨sd, hsd, hne, fun v hv => ?_⟩
  have h := forM_ok h v hv
  obtain ⟨hvi, h⟩ := seq_ok2 h
  refine ⟨?_, ?_⟩
  · unfold Bounds.checkVariantIndex at hvi
    split at hvi
    · simp [reject] at hvi
    · next hlt => exact Nat.lt_of_not_le hlt
  · split at h
    · simp [reject] at h
    · next hlt => exact Nat.lt_of_not_le hlt

/-- The bounds check of every function definition. -/
theorem verifyBounds_defs {m : CompiledModule} (hb : verifyBounds m = .ok ()) :
    ∀ fdef ∈ m.functionDefs.toList, Bounds.checkFunctionDef m fdef = .ok () := by
  unfold verifyBounds at hb
  simp only at hb
  split at hb
  · simp [reject, bind, Except.bind] at hb
  repeat replace hb := seq_ok hb
  exact forM_ok hb

theorem checkFunctionDef_code {m : CompiledModule} {fdef : FunctionDefinition} {code : CodeUnit}
    (h : Bounds.checkFunctionDef m fdef = .ok ()) (hcode : fdef.code = some code) :
    ∃ n params, Bounds.checkCode m code n params = .ok () := by
  unfold Bounds.checkFunctionDef at h
  repeat replace h := seq_ok h
  simp only [hcode] at h
  cases hfh : m.functionHandles[fdef.function]? with
  | none => simp [hfh, reject, bind, Except.bind] at h
  | some fh =>
    cases hps : m.signatures[fh.parameters]? with
    | none => simp [hfh, hps, pure, Except.pure, reject, bind, Except.bind] at h
    | some ps =>
      simp only [hfh, hps, pure, Except.pure, bind, Except.bind] at h
      exact ⟨_, _, h⟩

theorem checkCode_instrs {m : CompiledModule} {code : CodeUnit} {n : Nat} {params : Signature}
    (h : Bounds.checkCode m code n params = .ok ()) :
    ∃ localsCount, ∀ i ∈ code.code.toList,
      Bounds.checkInstr m code.code.size localsCount n i = .ok () := by
  unfold Bounds.checkCode at h
  replace h := seq_ok h
  cases hl : m.signatures[code.locals]? with
  | none => simp [hl, reject, bind, Except.bind] at h
  | some locals =>
    simp only [hl, pure_bind] at h
    split at h
    · simp [reject, bind, Except.bind] at h
    · repeat replace h := seq_ok h
      exact ⟨_, forM_ok h⟩

/-- Branch targets are in the code: the bounds check of a function's code. -/
theorem branch_targets {m : CompiledModule} (hb : verifyBounds m = .ok ())
    {fdef : FunctionDefinition} (hd : fdef ∈ m.functionDefs.toList) {code : CodeUnit}
    (hcode : fdef.code = some code) {pc o : Nat} {i : Bytecode} (hi : code.code[pc]? = some i)
    (ho : offset i = some o) : o < code.code.size := by
  obtain ⟨n, params, hcc⟩ := checkFunctionDef_code (verifyBounds_defs hb fdef hd) hcode
  obtain ⟨lc, hins⟩ := checkCode_instrs hcc
  have hc := hins i (getElem?_mem_toList hi)
  cases i <;> simp only [offset, reduceCtorEq, Option.some.injEq] at ho <;> subst ho <;>
    simp only [Bounds.checkInstr, checkIdx] at hc <;> split at hc <;>
    first | omega | (simp [reject] at hc)


theorem not_unconditional {i : Bytecode} (h : i.control = .fallThrough ∨ ∃ o, i.control = .branch o) :
    isUnconditionalBranch i = false := by
  cases i <;> simp_all [Bytecode.control, isUnconditionalBranch]

theorem next_lt {code : Array Bytecode} {pc : Nat} {i : Bytecode} (hi : code[pc]? = some i)
    (hfall : ∃ last, code.back? = some last ∧ isUnconditionalBranch last = true)
    (hc : i.control = .fallThrough ∨ ∃ o, i.control = .branch o) : pc + 1 < code.size := by
  have hpc := lt_of_getElem? hi
  obtain ⟨last, hlast, hu⟩ := hfall
  by_cases h : pc + 1 = code.size
  · exfalso
    simp only [Array.back?] at hlast
    rw [show code.size - 1 = pc by omega, hi, Option.some.injEq] at hlast
    subst hlast
    rw [not_unconditional hc] at hu
    exact Bool.false_ne_true hu
  · omega

/-- Every successor of every instruction is in the code (L3). -/
theorem successors_lt {code : Array Bytecode}
    (hfall : ∃ last, code.back? = some last ∧ isUnconditionalBranch last = true)
    (hbr : ∀ (pc o : Nat) (i : Bytecode), code[pc]? = some i → offset i = some o → o < code.size) :
    ∀ pc, ∀ s ∈ successors code pc, s < code.size := by
  intro pc s hs
  unfold successors at hs
  split at hs
  · simp at hs
  · next i hi =>
    cases hc : i.control with
    | fallThrough =>
      rw [hc] at hs
      simp only [List.mem_singleton] at hs
      subst hs
      exact next_lt hi hfall (Or.inl hc)
    | jump o =>
      rw [hc] at hs
      simp only [List.mem_singleton] at hs
      subst hs
      exact hbr pc _ i hi (control_jump hc).1
    | branch o =>
      rw [hc] at hs
      simp only [List.mem_cons, List.not_mem_nil, or_false] at hs
      rcases hs with rfl | rfl
      · exact hbr pc _ i hi (control_branch hc).1
      · exact next_lt hi hfall (Or.inr ⟨o, hc⟩)
    | exit => rw [hc] at hs; simp at hs

end MoveBytecode.Proofs
