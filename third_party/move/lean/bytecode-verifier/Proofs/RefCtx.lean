-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefDescOps

/-!
# The current frame against the certificates

UNTRUSTED (checked by the kernel). In a state satisfying the reference
invariant, the
current frame's abstract state from the reference certificate is well typed
for the type certificate's operand types, which the frame's operand stack
conforms to; so the two stacks have the same height, and an abstract value
slot holds no concrete reference (`TopCtx`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph

/-- What a reachable instruction of the current frame has from the
certificates. -/
structure TopCtx (P : Program) (s : State) (fr : Frame) (info : FunctionInfo) (code : CodeUnit)
    (instr : Bytecode) (view : FunctionView) (A : RefState) (stys : List SignatureToken)
    (tys : List RuntimeType) (ps : List Reference) : Prop where
  cert : RefCertOk P fr.function info.module code view
  ref : modelRef P fr.function fr.pc = some A
  typesBefore : typesBefore info.module view fr.pc = some stys
  typed : Typed (view.parameters ++ view.locals) stys (modelAvail P fr.function fr.pc) A
  sig : RuntimeType.ofSignatureWith info.module fr.function.typeArgs stys = some tys
  conf : conformsAll P fr.stack tys = true
  tc : modelTypes P fr.function fr.pc = some tys
  frameOk : FrameOk P s fr ps A
  chain : ∀ rest, s.frames = fr :: rest → Below P s fr ps rest
  roots : RootsOk P s
  exec : ∃ A', (ReferenceSafety.executeInner info.module view instr).run A = .ok ((), A')
  height : A.stack.length = fr.stack.length
  next : instr ≠ .ret → ∃ A', (ReferenceSafety.executeInner info.module view instr).run A =
    .ok ((), A') ∧ ∀ pc' ∈ successors code.code fr.pc, ∃ A'',
      modelRef P fr.function pc' = some A'' ∧ RefLe A' A''
  view : ∃ cfg, FunctionView.function info.module fr.function.index code info.handle cfg = .ok view
  /-- The frame's type arguments are value types. -/
  targs : ∀ x ∈ fr.function.typeArgs, x.isValue = true

/-- The suspended frames' locals conform. -/
theorem ChainRef.locals {P : Program} {s : State} :
    ∀ {cid : FrameId} {cfn : FunctionRef} {cps : List Reference} {Sc : StructId → Prop}
      {frs : List Frame}, ChainRef P s cid cfn cps Sc frs → ∀ g ∈ frs, LocalsOk P g
  | _, _, _, _, [], _, g, hg => by simp at hg
  | _, _, _, _, caller :: frs, ⟨ps, S, hsusp, hrest⟩, g, hg => by
    rcases List.mem_cons.1 hg with rfl | hg
    · exact hsusp.locals
    · exact ChainRef.locals hrest g hg

/-- **The current frame's context** in a state of the invariant, for an
accepted program whose struct-API mutable borrow handles have a wrapper's
signature. -/
theorem topCtxGen {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) (hapi : P.ApiSigs) {s : State} (hinv : InvRefs P s) {fr : Frame}
    {rest : List Frame} (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code) {instr : Bytecode}
    (hi : code.code[fr.pc]? = some instr) :
    ∃ view A stys tys ps, TopCtx P s fr info code instr view A stys tys ps := by
  obtain ⟨cfg, view, -, hview, hcert⟩ := refCert_gen hwf hs hapi hact.fn hact.body hact.typeArgs
  obtain ⟨A, stys, hA, htys, ht, hexec, hnext⟩ := hcert.at_ fr.pc instr hact.reachable hi
  obtain ⟨tys, htc, hconf⟩ := hact.types
  have htc' := htc
  rw [modelTypes_eq hact.fn hact.body hview, htys, Option.bind_some] at htc'
  obtain ⟨ps, hok, hchain⟩ := hinv.2.topOk hsf hA
  have hlen : A.stack.length = fr.stack.length := by
    rw [ht.nstack, conformsAll_length hconf, ofSignature_length htc']
  have hroots : RootsOk P s := by
    intro g hg
    rw [hsf] at hg
    rcases List.mem_cons.1 hg with rfl | hg
    · exact hact.locals
    · obtain ⟨S, -, hch⟩ := hchain
      exact hch.locals g hg
  refine ⟨view, A, stys, tys, ps, ⟨hcert, hA, htys, ht, htc', hconf, htc, hok, fun rest' hsf' => ?_,
    hroots, hexec, hlen, hnext, ⟨cfg, hview⟩, typeArgsOk_values hact.typeArgs⟩⟩
  rw [hsf, List.cons.injEq] at hsf'
  rw [← hsf'.2]
  exact hchain

/-- **The current frame's context** in a state of the invariant, for an
accepted program of the non-generic subset. -/
theorem topCtx' {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.SupportedNonGeneric) (hapi : P.ApiSigs) {s : State} (hinv : InvRefs P s) {fr : Frame}
    {rest : List Frame} (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code) {instr : Bytecode}
    (hi : code.code[fr.pc]? = some instr) :
    ∃ view A stys tys ps, TopCtx P s fr info code instr view A stys tys ps :=
  topCtxGen hwf (supported_of_nonGeneric hwf hs) hapi hinv hsf hact hi

/-- A slot token that is no reference translates to a value type, under type
arguments that are value types. -/
theorem ofSlotToken_isValue {m : CompiledModule} {τ : List RuntimeType}
    (hτ : ∀ x ∈ τ, x.isValue = true) {t : SignatureToken} {u : RuntimeType}
    (h : RuntimeType.ofSlotTokenWith m τ t = some u) (hk : refKind t = none) : u.isValue = true := by
  cases t <;> simp [refKind] at hk <;>
    exact ofTokenWith_isValue hτ (by simpa [RuntimeType.ofSlotTokenWith] using h)

/-- **An abstract value slot holds no concrete reference.** -/
theorem notRef_of_abs {m : CompiledModule} {P : Program} {ltys stys : List SignatureToken}
    {avail : LocalIndex → Availability} {A : RefState} {tys : List RuntimeType}
    {stack : List Value} {τ : List RuntimeType} (ht : Typed ltys stys avail A)
    (hτ : ∀ x ∈ τ, x.isValue = true)
    (hsig : RuntimeType.ofSignatureWith m τ stys = some tys) (hconf : conformsAll P stack tys = true)
    {k : Nat} (hA : A.stack[k]? = some .nonReference) {v : Value} (hv : stack[k]? = some v) :
    ∀ r, v ≠ .ref r := by
  have hlt : k < stys.length := by
    rw [← ht.nstack]; exact (List.getElem?_eq_some_iff.1 hA).1
  obtain ⟨t, htk⟩ : ∃ t, stys[k]? = some t := ⟨_, List.getElem?_eq_getElem hlt⟩
  have hk : refKind t = none := by
    cases hb : refKind t with
    | none => rfl
    | some b =>
      obtain ⟨x, hx⟩ := ht.stkKind k t b htk hb
      rw [hA] at hx
      cases hx
  obtain ⟨u, hu, hut⟩ := ofSigW_getElem hsig htk
  obtain ⟨u', hu', hvu⟩ := conformsAll_getElem hconf hv
  rw [hu, Option.some.injEq] at hu'
  subst hu'
  exact not_ref_of_conforms hvu (ofSlotToken_isValue hτ hut hk)

/-- **The current frame's context** in a state of the invariant, for an
accepted M3 program. -/
theorem topCtx {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.SupportedNonGeneric) (hm3 : P.M3) {s : State} (hinv : InvRefs P s) {fr : Frame}
    {rest : List Frame} (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code) {instr : Bytecode}
    (hi : code.code[fr.pc]? = some instr) :
    ∃ view A stys tys ps, TopCtx P s fr info code instr view A stys tys ps :=
  topCtx' hwf hs hm3.apiSigs hinv hsf hact hi

end MoveBytecode.Proofs
