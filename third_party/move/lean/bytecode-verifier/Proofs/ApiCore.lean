-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.SoundnessD1

/-!
# Struct-API borrows in progress, generic or not, for D4

UNTRUSTED (checked by the kernel). What D4's proofs need of a struct-API
wrapper in progress is common to a non-generic one (`ApiRun`) and a generic
one (`ApiRunG`), and is `ApiCore`: the caller's state at the call, which
keeps the reference invariant; the wrapper's code (`WrapperBody`), which
acquires nothing; and how the state differs from the call. Every state of an
execution has the reference invariant or such a wrapper in progress
(`invCore_reachable`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-- **The code of a struct-API wrapper**, generic or not: it acquires nothing,
and is `MoveLoc(0)`, a mutable field or variant field borrow, generic or not,
then `Ret`. -/
structure WrapperBody (cinfo : FunctionInfo) : Prop where
  acquires : cinfo.definition.acquiresGlobalResources = []
  code : ∃ code b, cinfo.definition.code = some code ∧ code.code.toList = [.moveLoc 0, b, .ret] ∧
    ((∃ f, b = .mutBorrowField f) ∨ (∃ v, b = .mutBorrowVariantField v) ∨
      (∃ i, b = .mutBorrowFieldGeneric i) ∨ ∃ i, b = .mutBorrowVariantFieldGeneric i)

/-- A wrapper with no global operation and no call acquires nothing, generic
borrows included. -/
theorem acquires_wrapperG {m : CompiledModule} {fdef : FunctionDefinition} {code : CodeUnit}
    (h : verifyAcquires m fdef code = .ok ()) {b : Bytecode}
    (hb : code.code.toList = [.moveLoc 0, b, .ret])
    (hbk : (∃ f, b = .mutBorrowField f) ∨ (∃ v, b = .mutBorrowVariantField v) ∨
      (∃ i, b = .mutBorrowFieldGeneric i) ∨ ∃ i, b = .mutBorrowVariantFieldGeneric i) :
    fdef.acquiresGlobalResources = [] := by
  unfold verifyAcquires at h
  rw [hb] at h
  simp only [bind_ok] at h
  obtain ⟨actual, hfold, hann⟩ := h
  have hact : actual = [] := by
    rcases hbk with ⟨i, rfl⟩ | ⟨i, rfl⟩ | ⟨i, rfl⟩ | ⟨i, rfl⟩ <;>
      simp [List.foldlM, Acquires.instr, pure, Except.pure, bind, Except.bind] at hfold <;>
      exact hfold
  subst hact
  cases hann' : fdef.acquiresGlobalResources with
  | nil => rfl
  | cons s rest =>
    rw [hann'] at hann
    simp [List.forM, Acquires.annotation, reject, bind, Except.bind] at hann

/-- **A struct-API mutable borrow in progress, generic or not**: state `s`
runs the wrapper `w` over the caller `fr`, which called it with the
reference `x` on top of its operand stack, from state `sc`. -/
structure ApiCore (P : Program) (s sc : State) : Prop where
  inv : InvRefs P sc
  call : ∃ (fr : Frame) (rest : List Frame) (info : FunctionInfo) (code : CodeUnit) (off : Nat)
      (x : Reference) (st : List Value) (w : Frame) (cinfo : FunctionInfo),
    sc.frames = fr :: rest ∧ ActiveAt P (modelTypes P) (modelAvail P) fr info code ∧
    fr.stack = .ref x :: st ∧ WrapperBody cinfo ∧
    s.frames = w :: { fr with stack := st } :: rest ∧ P.function? w.function = some cinfo ∧
    w.id = sc.nextFrame ∧ s.storage = sc.storage ∧ s.held = sc.held ∧
    (∃ es : List Invalidation, s.invalidations = es ++ sc.invalidations ∧
      ∀ e ∈ es, e = .removed (.local_ w.id 0)) ∧
    (∀ j v, w.locals[j]? = some (some v) → j = 0) ∧
    ((w.pc = 0 ∧ w.locals[0]? = some (some (.ref x)) ∧ w.stack = []) ∨
     (w.pc = 1 ∧ w.locals[0]? = some none ∧ w.stack = [.ref x]) ∨
     (w.pc = 2 ∧ w.locals[0]? = some none ∧ ∃ (y : Reference) (step : PathStep),
        w.stack = [.ref y] ∧ y.mutable = true ∧ y.location = x.location.child step ∧
        absPath [step] = [.field off] ∧ RefOk P s { fr with stack := st } y))

theorem ApiRun.core {P : Program} {s sc : State} (h : ApiRun P s sc) : ApiCore P s sc := by
  obtain ⟨fr, rest, info, code, hh, fh, off, x, st, w, callee, cinfo, hsc, hact, -, -, -, hst, -,
    hcf, hwc, hsf, hwfun, hwid, -, -, hstor, hheld, -, -, hes, hloc, hpc⟩ := h.call
  subst hwfun
  obtain ⟨ccode, b, sd, sdef, sh, inner, hcc, hcl, -, -, -, hb⟩ := hwc.code
  refine ⟨h.inv, fr, rest, info, code, off, x, st, w, cinfo, hsc, hact, hst,
    ⟨hwc.acquires, ccode, b, hcc, hcl, ?_⟩, hsf, hcf, hwid, hstor, hheld, hes, hloc, hpc⟩
  rcases hb with ⟨f, -, rfl, -⟩ | ⟨v, -, rfl, -⟩
  · exact Or.inl ⟨f, rfl⟩
  · exact Or.inr (Or.inl ⟨v, rfl⟩)

theorem ApiRunG.core {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {s sc : State} (h : ApiRunG P s sc) : ApiCore P s sc := by
  obtain ⟨fr, rest, info, code, i, fi, fh, off, x, st, w, cinfo, hsc, hact, -, -, -, -, hst, -,
    hwc, hsf, hwfn, hwid, -, -, hstor, hheld, -, -, hes, hloc, hpc⟩ := h.call
  obtain ⟨ccode, b, hcc, hcl, hb⟩ := hwc.code
  have hbk : (∃ f, b = .mutBorrowField f) ∨ (∃ v, b = .mutBorrowVariantField v) ∨
      (∃ i, b = .mutBorrowFieldGeneric i) ∨ ∃ i, b = .mutBorrowVariantFieldGeneric i := by
    rcases hb with ⟨f, -, rfl, -⟩ | ⟨v, -, rfl, -⟩ | ⟨j, -, -, rfl, -⟩ | ⟨j, -, -, rfl, -⟩
    · exact Or.inl ⟨f, rfl⟩
    · exact Or.inr (Or.inl ⟨v, rfl⟩)
    · exact Or.inr (Or.inr (Or.inl ⟨j, rfl⟩))
    · exact Or.inr (Or.inr (Or.inr ⟨j, rfl⟩))
  refine ⟨h.inv, fr, rest, info, code, off, x, st, w, cinfo, hsc, hact, hst,
    ⟨acquires_wrapperG (function_acquires hwf hwfn hcc) hcl hbk, ccode, b, hcc, hcl, hbk⟩, hsf,
    hwfn, hwid, hstor, hheld, hes, hloc, hpc⟩

/-- **The invariant of executions, for D4**: types and availability, and the
reference invariant or a struct-API borrow in progress, generic or not. -/
def InvCore (P : Program) (s : State) : Prop :=
  Inv P (modelTypes P) (modelAvail P) s ∧ (RefInvariant P s ∨ ∃ sc, ApiCore P s sc)

theorem InvAllG.core {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    {s : State} (h : InvAllG P s) : InvCore P s := by
  refine ⟨h.1, ?_⟩
  rcases h.2 with hr | ⟨sc, hrun⟩ | ⟨sc, hrun⟩
  · exact Or.inl hr
  · exact Or.inr ⟨sc, hrun.core⟩
  · exact Or.inr ⟨sc, hrun.core hwf⟩

/-- **Every reachable state satisfies the invariant**, generic code included. -/
theorem invCore_reachable {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported)
    (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
      ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
    {N : NativeRel} (hN : NativesRespect P N) {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    ∀ s, Reachable P N s₀ s → InvCore P s := fun s hr =>
  (invAllG_reachable hwf hs hapi (certified_of_wellFormed_gen schedulerSound hwf hs) hN h0 s
    hr).core hwf

/-- **A wrapper in progress runs its own code**: `MoveLoc(0)`, a mutable field
or variant field borrow, generic or not, or `Ret`. -/
theorem apiCore_instr {P : Program} {s sc : State} (hrun : ApiCore P s sc) {w : Frame}
    {rest : List Frame} (hsf : s.frames = w :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hinfo : P.function? w.function = some info) (hcode : info.definition.code = some code)
    {instr : Bytecode} (hi : code.code[w.pc]? = some instr) :
    (w.pc = 0 ∧ instr = .moveLoc 0) ∨
      (w.pc = 1 ∧ ((∃ f, instr = .mutBorrowField f) ∨ (∃ v, instr = .mutBorrowVariantField v) ∨
        (∃ i, instr = .mutBorrowFieldGeneric i) ∨ ∃ i, instr = .mutBorrowVariantFieldGeneric i)) ∨
      (w.pc = 2 ∧ instr = .ret) := by
  obtain ⟨fr, rest₀, info₀, code₀, off, x, st, w', cinfo, -, -, -, hwc, hs'f, hwfun, -⟩ :=
    hrun.call
  rw [hsf, List.cons.injEq] at hs'f
  obtain ⟨rfl, -⟩ := hs'f
  rw [hwfun, Option.some.injEq] at hinfo
  subst hinfo
  obtain ⟨ccode, b, hccode, hcl, hb⟩ := hwc.code
  rw [hccode, Option.some.injEq] at hcode
  subst hcode
  rcases wrapper_at hcl hi with ⟨hp, rfl⟩ | ⟨hp, rfl⟩ | ⟨hp, rfl⟩
  · exact Or.inl ⟨hp, rfl⟩
  · exact Or.inr (Or.inl ⟨hp, hb⟩)
  · exact Or.inr (Or.inr ⟨hp, rfl⟩)

/-- A wrapper's function acquires nothing. -/
theorem apiCore_noAcq {P : Program} {s sc : State} (hrun : ApiCore P s sc) {w : Frame}
    {rest : List Frame} (hsf : s.frames = w :: rest) {t : StructId} :
    ¬ acqTypes P w.function t := by
  obtain ⟨fr, rest₀, info₀, code₀, off, x, st, w', cinfo, -, -, -, hwc, hs'f, hwfun, -⟩ :=
    hrun.call
  rw [hsf, List.cons.injEq] at hs'f
  obtain ⟨rfl, -⟩ := hs'f
  rintro ⟨d, hd, -⟩
  simp only [acqOf, hwfun, Option.map_some, Option.getD_some, hwc.acquires] at hd
  cases hd

end MoveBytecode.Proofs
