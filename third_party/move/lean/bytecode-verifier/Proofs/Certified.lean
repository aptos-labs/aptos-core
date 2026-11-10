-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ByValue
import Proofs.Instances
import Proofs.NativeFacts

/-!
# D1 from certificates

UNTRUSTED (checked by the kernel). The main safety argument, stated with the
specification's own notions and independent of any verifier: a program whose
functions have consistent type and availability certificates (C1, C2), whose
reachable calls resolve to visible functions whose definitions match the
caller's handles (K1, K2), whose reachable successors stay in the code (L3),
and whose constants decode, never goes wrong (D1), whatever the natives do
within their contracts.

The verifier's part is to establish `Certified` (see `Proofs.Soundness`).
Because the theorem quantifies over every consistent certificate, it applies
equally to any other consumer, MonoVM included, that computes or checks such
certificates.

The invariant `Inv` says that the current frame is at a reachable instruction
of a function with code, its operand stack conforms to the type certificate,
its locals conform to their declared types and hold a value wherever the
availability certificate says `available`; and every suspended frame resumes
in such a state when its callee returns values of the callee's declared
return types.
-/

namespace MoveBytecode.Proofs

/-- What the soundness argument needs of a program, stated with the
specification's own notions: consistent certificates, and the facts about
reachable instructions that the certificates do not cover. -/
structure Certified (P : Program) (tc : TypeCertificate) (ac : AvailabilityCertificate) :
    Prop where
  types : tc.Consistent P
  availability : ac.Consistent P
  locals : ∀ f info code, P.function? f = some info → info.definition.code = some code →
    P.typeArgsOk f = true → ∃ tys, P.localTypes f = some tys
  nonempty : ∀ f info code, P.function? f = some info → info.definition.code = some code →
    0 < code.code.size
  successors : ∀ f info code, P.function? f = some info → info.definition.code = some code →
    ∀ pc, CodeReachable code.code pc → ∀ s ∈ successors code.code pc, s < code.code.size
  calls : ∀ f info code, P.function? f = some info → info.definition.code = some code →
    ∀ pc h, CodeReachable code.code pc → code.code[pc]? = some (.call h) →
      ∃ callee cinfo, P.resolve f.module h = some callee ∧
        P.visibleFrom info.moduleId callee = true ∧ P.function? callee = some cinfo ∧
        cinfo.handle.typeParameters = [] ∧
        P.paramTypes callee = P.handleParamTypes f.module h ∧
        P.returnTypes callee = P.handleReturnTypes f.module h
  /-- A generic call resolves to a visible function with as many type
  parameters as the caller's handle, whose constraints the handle's include,
  and whose parameter and return types are the handle's at every instance of
  the arity (K1, K2). -/
  callsGeneric : ∀ f info code, P.function? f = some info → info.definition.code = some code →
    ∀ pc i x, CodeReachable code.code pc → code.code[pc]? = some (.callGeneric i) →
      info.module.functionInstantiations[i]? = some x →
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
            P.handleReturnTypesAt f.module x.handle tys
  /-- A closure's target resolves, as a generic call's, to a visible
  function with code, of the handle's arity, whose constraints the handle's
  include, and whose parameter and return types are the handle's at every
  instance of the arity (K1, K2, `Program.ClosureTargetsHaveCode`). -/
  closures : ∀ f info code, P.function? f = some info → info.definition.code = some code →
    ∀ pc instr h, CodeReachable code.code pc → code.code[pc]? = some instr →
      instr.closureHandle? info.module = some h →
      ∃ target tinfo fh, P.resolve f.module h = some target ∧
        P.visibleFrom info.moduleId target = true ∧ P.function? target = some tinfo ∧
        tinfo.definition.code.isSome = true ∧
        info.module.functionHandles[h]? = some fh ∧
        tinfo.handle.typeParameters.length = fh.typeParameters.length ∧
        (∀ (j : Nat) (c d : AbilitySet), fh.typeParameters[j]? = some c →
          tinfo.handle.typeParameters[j]? = some d → d.subset c = true) ∧
        ∀ tys : List RuntimeType, tys.length = fh.typeParameters.length →
          P.paramTypes { target with typeArgs := tys } = P.handleParamTypesAt f.module h tys ∧
          P.returnTypes { target with typeArgs := tys } = P.handleReturnTypesAt f.module h tys
  constants : ∀ f info code, P.function? f = some info → info.definition.code = some code →
    ∀ pc i, CodeReachable code.code pc → code.code[pc]? = some (.ldConst i) →
      ∀ c t, info.module.constantPool[i]? = some c → RuntimeType.ofToken info.module c.type = some t →
        ∃ v, decodeConstant t c.data = some v ∧ v.conforms P t = true
  /-- At entry, a local certified unavailable is one of the function's locals.
  Needed only for the agreement of `unavailable` (C2), not for D1. -/
  entryRange : ∀ f info code, P.function? f = some info → info.definition.code = some code →
    P.typeArgsOk f = true →
    ∀ i, ac f 0 i = .unavailable → ∃ tys, P.localTypes f = some tys ∧ i < tys.length
  /-- Declared abilities are justified by the fields (L6, through K1), so a
  value has the abilities of its type. -/
  abilities : AbilitiesSound P
  /-- Natives other than the modeled ones take and return values only. -/
  nativeValues : ∀ f info, P.function? f = some info → info.definition.code = none →
    P.typeArgsOk f = true → P.nativeShape? f = none →
    (∀ tys, P.paramTypes f = some tys → ∀ t ∈ tys, t.isValue = true) ∧
    (∀ tys, P.returnTypes f = some tys → ∀ t ∈ tys, t.isValue = true)
  /-- A local that `StLoc` may overwrite, or that may still hold a value at
  `Ret`, has a type with `drop`. -/
  drops : ∀ f info code, P.function? f = some info → info.definition.code = some code →
    P.typeArgsOk f = true → ∀ pc i, CodeReachable code.code pc →
      (code.code[pc]? = some .ret ∨ code.code[pc]? = some (.stLoc i)) →
      ac f pc i ≠ .unavailable →
      ∀ tys t, P.localTypes f = some tys → tys[i]? = some t → (t.abilities P).drop = true

variable {P : Program} {tc : TypeCertificate} {ac : AvailabilityCertificate}

/-- The locals of a frame have the declared number of slots, and every value
in a slot conforms to its declared type. -/
def LocalsOk (P : Program) (fr : Frame) : Prop :=
  ∃ tys, P.localTypes fr.function = some tys ∧ fr.locals.size = tys.length ∧
    ∀ (i : Nat) (v : Value), fr.locals[i]? = some (some v) →
      ∃ t : RuntimeType, tys[i]? = some t ∧ v.conforms P t = true

/-- The current frame, about to execute a reachable instruction of function
code `code`, agrees with both certificates. -/
structure ActiveAt (P : Program) (tc : TypeCertificate) (ac : AvailabilityCertificate)
    (fr : Frame) (info : FunctionInfo) (code : CodeUnit) : Prop where
  fn : P.function? fr.function = some info
  body : info.definition.code = some code
  pcLt : fr.pc < code.code.size
  reachable : CodeReachable code.code fr.pc
  types : ∃ tys, tc fr.function fr.pc = some tys ∧ conformsAll P fr.stack tys = true
  locals : LocalsOk P fr
  available : ∀ i, ac fr.function fr.pc i = .available → ∃ v, fr.locals[i]? = some (some v)
  unavailable : ∀ i, ac fr.function fr.pc i = .unavailable → fr.locals[i]? = some none
  /-- The frame runs an instance whose type arguments satisfy its function's
  constraints. -/
  typeArgs : P.typeArgsOk fr.function = true

def ActiveOk (P : Program) (tc : TypeCertificate) (ac : AvailabilityCertificate)
    (fr : Frame) : Prop :=
  ∃ info code, ActiveAt P tc ac fr info code

/-- A suspended caller resumes correctly with any return values of the
declared types. -/
def ResumeOk (P : Program) (tc : TypeCertificate) (ac : AvailabilityCertificate)
    (caller : Frame) (rets : List RuntimeType) : Prop :=
  ∀ vals, conformsAll P vals rets = true →
    ActiveOk P tc ac { caller with stack := vals.reverse ++ caller.stack, pc := caller.pc + 1 }

/-- Each suspended frame resumes correctly when the frame above it returns. -/
def ChainOk (P : Program) (tc : TypeCertificate) (ac : AvailabilityCertificate) :
    FunctionRef → List Frame → Prop
  | _, [] => True
  | callee, caller :: rest =>
    (∃ rets, P.returnTypes callee = some rets ∧ ResumeOk P tc ac caller rets) ∧
      ChainOk P tc ac caller.function rest

/-- The invariant of verified executions. -/
def Inv (P : Program) (tc : TypeCertificate) (ac : AvailabilityCertificate) (s : State) :
    Prop :=
  ∃ fr rest, s.frames = fr :: rest ∧ ActiveOk P tc ac fr ∧ ChainOk P tc ac fr.function rest

/-! ## Moving to a successor -/

theorem refines_available {a b : Availability} (h : Availability.Refines a b)
    (hb : b = .available) : a = .available := by
  rcases h with h | h <;> subst hb <;> simp_all

theorem refines_unavailable {a b : Availability} (h : Availability.Refines a b)
    (hb : b = .unavailable) : a = .unavailable := by
  rcases h with h | h <;> subst hb <;> simp_all

/-- The frame after an instruction, at one of its successors, agrees with the
certificates when its stack conforms to the certified result and its locals
are as the instruction leaves them. -/
theorem activeAt_succ (hc : Certified P tc ac) {fr fr' : Frame} {info : FunctionInfo}
    {code : CodeUnit} {instr : Bytecode} {before after : List RuntimeType}
    (hact : ActiveAt P tc ac fr info code) (hinstr : code.code[fr.pc]? = some instr)
    (hbefore : tc fr.function fr.pc = some before)
    (hafter : typeAfter P fr.function info.module instr before = some after)
    (hfun : fr'.function = fr.function) (hsucc : fr'.pc ∈ successors code.code fr.pc)
    (hstack : conformsAll P fr'.stack after = true) (hlocals : LocalsOk P fr')
    (havail : ∀ i, availabilityAfter instr (ac fr.function fr.pc) i = .available →
      ∃ v, fr'.locals[i]? = some (some v))
    (hunavail : ∀ i, availabilityAfter instr (ac fr.function fr.pc) i = .unavailable →
      fr'.locals[i]? = some none) :
    ActiveAt P tc ac fr' info code := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨b, a, hb, ha, hs⟩ := htypes fr.pc instr hact.reachable hinstr
  rw [hbefore, Option.some.injEq] at hb
  subst hb
  rw [hafter, Option.some.injEq] at ha
  subst ha
  obtain ⟨params, -, -, havl⟩ := hc.availability fr.function info code hact.fn hact.body
  have hav := (havl fr.pc instr hact.reachable hinstr).2 fr'.pc hsucc
  obtain ⟨certified, hcert, hac⟩ := hs fr'.pc hsucc
  refine ⟨hfun ▸ hact.fn, hact.body, ?_, ?_, ⟨certified, ?_, conformsAll_assignable hstack hac⟩,
    hlocals, ?_, ?_, hfun ▸ hact.typeArgs⟩
  · exact hc.successors fr.function info code hact.fn hact.body fr.pc hact.reachable
      fr'.pc hsucc
  · exact CodeReachable.succ hact.reachable hsucc
  · rw [hfun]; exact hcert
  · intro i hi
    rw [hfun] at hi
    exact havail i (refines_available (hav i) hi)
  · intro i hi
    rw [hfun] at hi
    exact hunavail i (refines_unavailable (hav i) hi)

/-! ## Safe outcomes -/

/-- An outcome that is not `wrong`, and whose state, if running, satisfies
`Q`. -/
def Safe (Q : State → Prop) : Outcome → Prop
  | .running s => Q s
  | .returned _ _ => True
  | .failed _ => True
  | .wrong _ => False

/-- A step result that is safe, for every result the natives may produce. -/
def StepSafe (P : Program) (N : NativeRel) (Q : State → Prop) : StepResult → Prop
  | .done o => Safe Q o
  | .callNative s f args views => ∀ r, N f views r → Safe Q (resumeNative P s f args r)

def ExecSafe (P : Program) (N : NativeRel) (Q : State → Prop) : Step.Exec StepResult → Prop
  | .ok r => StepSafe P N Q r
  | .error o => Safe Q o

/-- The callee an instruction names, resolved from module `mi` with the
instantiations of module `m`, is no modeled native (`Program.nativeShape?`):
a call of it behaves as a call of a function with code or of a native on
values. Every instruction but `Call` and `CallGeneric` calls nothing. -/
def NoModeledCall (P : Program) (mi : Nat) (m : CompiledModule) : Bytecode → Prop
  | .call h => ∀ c, P.resolve mi h = some c → P.nativeShape? c = none
  | .callGeneric i => ∀ x c, m.functionInstantiations[i]? = some x →
      P.resolve mi x.handle = some c → P.nativeShape? c = none
  | _ => True

theorem conformsAll_cons_right {vs : List Value} {t : RuntimeType} {ts : List RuntimeType}
    (h : conformsAll P vs (t :: ts) = true) :
    ∃ v rest, vs = v :: rest ∧ v.conforms P t = true ∧ conformsAll P rest ts = true := by
  cases vs with
  | nil => simp [conformsAll] at h
  | cons v rest => exact ⟨v, rest, rfl, (conformsAll_cons.1 h)⟩

/-- The invariant reads only the frames, so continuing from any state with
the same frames is as safe. -/
theorem next_state {N : NativeRel} {s s' : State} {rest : List Frame} {fr : Frame}
    (h : ExecSafe P N (Inv P tc ac) (Step.next s rest fr)) :
    ExecSafe P N (Inv P tc ac) (Step.next s' rest fr) := h

theorem next_safe {N : NativeRel} {s : State} {rest : List Frame} {fr : Frame}
    (hact : ActiveOk P tc ac fr) (hchain : ChainOk P tc ac fr.function rest) :
    ExecSafe P N (Inv P tc ac) (Step.next s rest fr) :=
  ⟨fr, rest, rfl, hact, hchain⟩

/-- A fall-through instruction that leaves the locals alone. -/
theorem fallThrough_safe (hc : Certified P tc ac) {N : NativeRel} {s : State}
    {rest : List Frame} {fr fr' : Frame} {info : FunctionInfo} {code : CodeUnit}
    {instr : Bytecode} {before after : List RuntimeType}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    (hinstr : code.code[fr.pc]? = some instr) (hctrl : instr.control = .fallThrough)
    (hav : ∀ a, availabilityAfter instr a = a)
    (hbefore : tc fr.function fr.pc = some before)
    (hafter : typeAfter P fr.function info.module instr before = some after)
    (hfun : fr'.function = fr.function) (hpc : fr'.pc = fr.pc + 1)
    (hloc : fr'.locals = fr.locals) (hstk : conformsAll P fr'.stack after = true) :
    ExecSafe P N (Inv P tc ac) (Step.next s rest fr') := by
  refine next_safe ⟨info, code, activeAt_succ hc hact hinstr hbefore hafter hfun ?_ hstk ?_ ?_
    ?_⟩ (hfun ▸ hchain)
  · simp [successors, hinstr, hctrl, hpc]
  · obtain ⟨tys, h1, h2, h3⟩ := hact.locals
    exact ⟨tys, hfun ▸ h1, hloc ▸ h2, hloc ▸ h3⟩
  · intro i hi
    rw [hav] at hi
    exact hloc ▸ hact.available i hi
  · intro i hi
    rw [hav] at hi
    exact hloc ▸ hact.unavailable i hi

theorem intBinary_some {allow : IntType → Bool} {result : IntType → RuntimeType}
    {before after : List RuntimeType} (h : intBinary allow result before = some after) :
    ∃ t rest, before = .int t :: .int t :: rest ∧ allow t = true ∧ after = result t :: rest := by
  unfold intBinary at h
  split at h
  · next t' t rest =>
    split at h
    · next hc =>
      simp only [Bool.and_eq_true, decide_eq_true_eq] at hc
      obtain ⟨rfl, ha⟩ := hc
      exact ⟨t, rest, rfl, ha, (Option.some.inj h).symm⟩
    · simp at h
  · simp at h

theorem castTo_some {target : IntType} {before after : List RuntimeType}
    (h : castTo target before = some after) :
    ∃ t rest, before = .int t :: rest ∧ after = .int target :: rest := by
  unfold castTo at h
  split at h
  · next t rest => exact ⟨t, rest, rfl, (Option.some.inj h).symm⟩
  · simp at h

theorem conformsAll_int {vs : List Value} {t : IntType} {ts : List RuntimeType}
    (h : conformsAll P vs (.int t :: ts) = true) :
    ∃ x rest, vs = .int t x :: rest ∧ t.inRange x = true ∧ conformsAll P rest ts = true := by
  obtain ⟨v, rest, rfl, hv, hr⟩ := conformsAll_cons_right h
  obtain ⟨x, rfl, hx⟩ := conforms_int hv
  exact ⟨x, rest, rfl, hx, hr⟩

theorem conformsAll_bool {vs : List Value} {ts : List RuntimeType}
    (h : conformsAll P vs (.bool :: ts) = true) :
    ∃ b rest, vs = .bool b :: rest ∧ conformsAll P rest ts = true := by
  obtain ⟨v, rest, rfl, hv, hr⟩ := conformsAll_cons_right h
  obtain ⟨b, rfl⟩ := conforms_bool hv
  exact ⟨b, rest, rfl, hr⟩

theorem bind_failed {α : Type} {x : Step.Exec α} {g : α → Step.Exec StepResult} {f : Failure}
    {N : NativeRel} {Q : State → Prop} (hx : x = .error (.failed f)) :
    ExecSafe P N Q (x >>= g) := by
  subst hx; trivial

/-- Moving to a successor. -/
theorem succ_safe (hc : Certified P tc ac) {N : NativeRel} {s : State}
    {rest : List Frame} {fr fr' : Frame} {info : FunctionInfo} {code : CodeUnit}
    {instr : Bytecode} {before after : List RuntimeType}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    (hinstr : code.code[fr.pc]? = some instr)
    (hbefore : tc fr.function fr.pc = some before)
    (hafter : typeAfter P fr.function info.module instr before = some after)
    (hfun : fr'.function = fr.function) (hsucc : fr'.pc ∈ successors code.code fr.pc)
    (hstk : conformsAll P fr'.stack after = true) (hlocals : LocalsOk P fr')
    (havail : ∀ i, availabilityAfter instr (ac fr.function fr.pc) i = .available →
      ∃ v, fr'.locals[i]? = some (some v))
    (hunavail : ∀ i, availabilityAfter instr (ac fr.function fr.pc) i = .unavailable →
      fr'.locals[i]? = some none) :
    ExecSafe P N (Inv P tc ac) (Step.next s rest fr') :=
  next_safe ⟨info, code, activeAt_succ hc hact hinstr hbefore hafter hfun hsucc hstk hlocals
    havail hunavail⟩ (hfun ▸ hchain)

theorem localsOk_of {fr fr' : Frame} (h : LocalsOk P fr) (hfun : fr'.function = fr.function)
    (hloc : fr'.locals = fr.locals) : LocalsOk P fr' := by
  obtain ⟨tys, h1, h2, h3⟩ := h
  exact ⟨tys, hfun ▸ h1, hloc ▸ h2, hloc ▸ h3⟩

/-- A local that a reachable instruction reads holds a value of its declared
type. -/
theorem read_local (hc : Certified P tc ac) {fr : Frame} {info : FunctionInfo} {code : CodeUnit}
    {instr : Bytecode} {i : LocalIndex} {tys : List RuntimeType} {t : RuntimeType}
    (hact : ActiveAt P tc ac fr info code) (hinstr : code.code[fr.pc]? = some instr)
    (hread : instr.readsLocal = some i) (htys : P.localTypes fr.function = some tys)
    (hti : tys[i]? = some t) : ∃ v, fr.locals[i]? = some (some v) ∧ v.conforms P t = true := by
  obtain ⟨-, -, -, havl⟩ := hc.availability fr.function info code hact.fn hact.body
  obtain ⟨v, hv⟩ := hact.available i ((havl fr.pc _ hact.reachable hinstr).1 i hread)
  obtain ⟨tys', htys', -, hconf⟩ := hact.locals
  rw [htys, Option.some.injEq] at htys'
  subst htys'
  obtain ⟨t', ht', hvt⟩ := hconf i v hv
  rw [hti, Option.some.injEq] at ht'
  subst ht'
  exact ⟨v, hv, hvt⟩

/-! ## The semantics' helpers on well-typed operands -/

section Helpers

variable {N : NativeRel} {Q : State → Prop} {s : State} {rest : List Frame} {fr : Frame}

theorem intBinop_safe {allow : IntType → Bool} {op : IntType → Int → Int → Option Int}
    {t : IntType} {x y : Int} {vs : List Value}
    (hst : fr.stack = .int t y :: .int t x :: vs) (hallow : allow t = true)
    (k : ∀ r, op t x y = some r →
      ExecSafe P N Q (Step.next s rest (Step.advance (Step.push { fr with stack := vs } (.int t r))))) :
    ExecSafe P N Q (Step.intBinop fr allow op >>= fun fr => Step.next s rest fr) := by
  simp only [Step.intBinop, Step.popInt, Step.pop, hst, bind, Except.bind, pure, Except.pure,
    bne_self_eq_false, hallow, Bool.not_true, Bool.or_false, Bool.false_eq_true, ↓reduceIte]
  cases h : op t x y with
  | none => trivial
  | some r => exact k r h

theorem intCompare_safe {op : Int → Int → Bool} {t : IntType} {x y : Int} {vs : List Value}
    (hst : fr.stack = .int t y :: .int t x :: vs)
    (k : ExecSafe P N Q (Step.next s rest
      (Step.advance (Step.push { fr with stack := vs } (.bool (op x y)))))) :
    ExecSafe P N Q (Step.intCompare fr op >>= fun fr => Step.next s rest fr) := by
  simpa only [Step.intCompare, Step.popInt, Step.pop, hst, bind, Except.bind, pure, Except.pure,
    bne_self_eq_false, Bool.false_eq_true, ↓reduceIte] using k

theorem cast_safe {target t : IntType} {x : Int} {vs : List Value}
    (hst : fr.stack = .int t x :: vs)
    (k : ∀ r, IntType.cast target x = some r → ExecSafe P N Q (Step.next s rest
      (Step.advance (Step.push { fr with stack := vs } (.int target r))))) :
    ExecSafe P N Q (Step.cast fr target >>= fun fr => Step.next s rest fr) := by
  simp only [Step.cast, Step.popInt, Step.pop, hst, bind, Except.bind, pure, Except.pure]
  cases h : IntType.cast target x with
  | none => trivial
  | some r => exact k r h

theorem shift_safe {op : IntType → Int → Nat → Option Int} {t : IntType} {x n : Int}
    {vs : List Value} (hst : fr.stack = .int .u8 n :: .int t x :: vs) (hs : t.isSigned = false)
    (k : ∀ r, op t x n.toNat = some r → ExecSafe P N Q (Step.next s rest
      (Step.advance (Step.push { fr with stack := vs } (.int t r))))) :
    ExecSafe P N Q (Step.shift fr op >>= fun fr => Step.next s rest fr) := by
  simp only [Step.shift, Step.popInt, Step.pop, hst, bind, Except.bind, pure, Except.pure,
    bne_self_eq_false, hs, Bool.or_false, Bool.false_eq_true, ↓reduceIte]
  cases h : op t x n.toNat with
  | none => trivial
  | some r => exact k r h

theorem boolBinop_safe {op : Bool → Bool → Bool} {a b : Bool} {vs : List Value}
    (hst : fr.stack = .bool b :: .bool a :: vs)
    (k : ExecSafe P N Q (Step.next s rest
      (Step.advance (Step.push { fr with stack := vs } (.bool (op a b)))))) :
    ExecSafe P N Q (Step.boolBinop fr op >>= fun fr => Step.next s rest fr) := by
  simpa only [Step.boolBinop, Step.popBool, Step.pop, hst, bind, Except.bind, pure,
    Except.pure] using k

theorem equality_safe {positive : Bool} {a b : Value} {vs : List Value}
    (hst : fr.stack = b :: a :: vs) (hty : RuntimeType.comparable a.type b.type = true)
    (ha : (a.abilities P).drop = true) (hb : (b.abilities P).drop = true)
    (hra : ∀ r, a ≠ .ref r)
    (k : ExecSafe P N Q (Step.next s rest (Step.advance (Step.push { fr with stack := vs }
      (.bool (if positive then a == b else a != b)))))) :
    ExecSafe P N Q (Step.equality P s fr positive >>= fun fr => Step.next s rest fr) := by
  cases a with
  | ref r => exact absurd rfl (hra r)
  | _ =>
    simpa only [Step.equality, Step.pop, hst, bind, Except.bind, pure, Except.pure, hty,
      bne_self_eq_false, Bool.false_eq_true, ↓reduceIte, ha, hb, Bool.not_true,
      Bool.or_self] using k

end Helpers

/-! ## Calls -/

theorem ofSignature_length {m : CompiledModule} {tys : List RuntimeType} {sig : Signature}
    {ps : List RuntimeType} (h : RuntimeType.ofSignatureWith m tys sig = some ps) :
    ps.length = sig.length := by
  unfold RuntimeType.ofSignatureWith at h
  induction sig generalizing ps with
  | nil => simp at h; subst h; rfl
  | cons t ts ih =>
    simp only [List.mapM_cons, bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
    obtain ⟨u, -, us, hus, rfl⟩ := h
    simp [ih hus]

theorem localTypes_split {f : FunctionRef} {tys : List RuntimeType}
    (h : P.localTypes f = some tys) : ∃ ps ls, P.paramTypes f = some ps ∧ tys = ps ++ ls := by
  simp only [Program.localTypes, bind, Option.bind_eq_some_iff, pure,
    Option.some.injEq] at h
  obtain ⟨info, hinfo, code, -, sig, hsig, ps, hps, lsig, -, ls, -, rfl⟩ := h
  exact ⟨ps, ls, by simp [Program.paramTypes, hinfo, hsig, hps], rfl⟩

theorem paramTypes_sig {f : FunctionRef} {info : FunctionInfo} {ps : List RuntimeType}
    (hf : P.function? f = some info) (h : P.paramTypes f = some ps) :
    ∃ sig, info.module.signatures[info.handle.parameters]? = some sig ∧
      RuntimeType.ofSignatureWith info.module f.typeArgs sig = some ps := by
  simp only [Program.paramTypes, hf, bind, Option.bind_eq_some_iff, Option.some.injEq] at h
  obtain ⟨_, rfl, sig, hsig, hps⟩ := h
  exact ⟨sig, hsig, hps⟩

theorem conformsAll_getElem {vs : List Value} {ts : List RuntimeType} {j : Nat} {v : Value}
    (h : conformsAll P vs ts = true) (hv : vs[j]? = some v) :
    ∃ t, ts[j]? = some t ∧ v.conforms P t = true := by
  induction vs generalizing ts j with
  | nil => simp at hv
  | cons w ws ih =>
    cases ts with
    | nil => simp [conformsAll] at h
    | cons t ts =>
      rw [conformsAll_cons] at h
      cases j with
      | zero => simp at hv; subst hv; exact ⟨t, rfl, h.1⟩
      | succ j => simpa using ih (ts := ts) (j := j) h.2 (by simpa using hv)

/-- The locals of a new frame: the arguments, then unavailable slots. -/
theorem entry_locals {args : List Value} {n j : Nat} {v : Value}
    (h : (args.map some ++ List.replicate (n - args.length) none).toArray[j]? = some (some v)) :
    args[j]? = some v := by
  simp only [List.getElem?_toArray] at h
  by_cases hj : j < args.length
  · rw [List.getElem?_append_left (by simpa using hj), List.getElem?_map] at h
    simpa using h
  · rw [List.getElem?_append_right (by simpa using hj), List.getElem?_replicate] at h
    split at h <;> simp at h

theorem entry_locals_of {args : List Value} {n j : Nat} (hj : j < args.length) :
    (args.map some ++ List.replicate (n - args.length) none).toArray[j]? = some args[j]? := by
  simp only [List.getElem?_toArray]
  rw [List.getElem?_append_left (by simpa using hj), List.getElem?_map]
  simp [hj]

theorem entry_locals_none {args : List Value} {n j : Nat} (h1 : args.length ≤ j) (h2 : j < n) :
    (args.map some ++ List.replicate (n - args.length) none).toArray[j]? = some none := by
  simp only [List.getElem?_toArray]
  rw [List.getElem?_append_right (by simpa using h1), List.getElem?_replicate]
  simp only [List.length_map]
  rw [if_pos (by omega)]

/-- The callee's first frame agrees with the certificates. -/
theorem entry_activeAt (hc : Certified P tc ac) {callee : FunctionRef} {cinfo : FunctionInfo}
    {ccode : CodeUnit} {params ls : List RuntimeType} {args : List Value}
    (hcf : P.function? callee = some cinfo) (hcode : cinfo.definition.code = some ccode)
    (htargs : P.typeArgsOk callee = true)
    (hpar : P.paramTypes callee = some params) (hltys : P.localTypes callee = some (params ++ ls))
    (hargs : conformsAll P args params = true) (pc0 : Nat) (hpc0 : pc0 = 0) (stack : List Value)
    (hstack : stack = []) (ca le : Bool) (fid : Nat) :
    ActiveAt P tc ac
      { function := callee, id := fid, pc := pc0,
        locals := (args.map some ++ List.replicate ((params ++ ls).length - args.length) none).toArray,
        stack := stack, countedActive := ca, lockEntered := le } cinfo ccode := by
  subst hpc0 hstack
  have hlen := conformsAll_length hargs
  refine ⟨hcf, hcode, hc.nonempty callee cinfo ccode hcf hcode, ⟨[0], Walk.nil 0⟩,
    ⟨[], (hc.types callee cinfo ccode hcf hcode htargs).1, rfl⟩, ⟨params ++ ls, hltys, ?_, ?_⟩,
    ?_, ?_, htargs⟩
  · simp only [List.size_toArray, List.length_append, List.length_map, List.length_replicate]
    omega
  · intro j v hv
    obtain ⟨t, ht, hvt⟩ := conformsAll_getElem hargs (entry_locals hv)
    have hj : j < params.length := (List.getElem?_eq_some_iff.1 ht).1
    exact ⟨t, by rw [List.getElem?_append_left hj]; exact ht, hvt⟩
  · intro j hj
    obtain ⟨psig, hpsig, hent, -⟩ := hc.availability callee cinfo ccode hcf hcode
    obtain ⟨sig, hsig, hps⟩ := paramTypes_sig hcf hpar
    rw [hsig, Option.some.injEq] at hpsig
    subst hpsig
    have h0 := refines_available (hent j) hj
    have hjs : j < sig.length := by
      split at h0
      · assumption
      · simp at h0
    have hj' : j < args.length := by rw [hlen, ofSignature_length hps]; exact hjs
    refine ⟨args[j], ?_⟩
    dsimp only
    rw [entry_locals_of hj', List.getElem?_eq_getElem hj']
  · intro j hj
    obtain ⟨psig, hpsig, hent, -⟩ := hc.availability callee cinfo ccode hcf hcode
    obtain ⟨sig, hsig, hps⟩ := paramTypes_sig hcf hpar
    rw [hsig, Option.some.injEq] at hpsig
    subst hpsig
    have h0 := refines_unavailable (hent j) hj
    have hjs : sig.length ≤ j := by
      split at h0
      · simp at h0
      · omega
    obtain ⟨tys, htys, hjt⟩ := hc.entryRange callee cinfo ccode hcf hcode htargs j hj
    rw [hltys, Option.some.injEq] at htys
    subst htys
    dsimp only
    exact entry_locals_none (by rw [hlen, ofSignature_length hps]; exact hjs) hjt

/-- A call of handle `h` at type arguments `tys`, which the certificate types
as `typeAfter`'s call does, given the callee's resolution and linking. -/
theorem callWith_safe (hc : Certified P tc ac) {N : NativeRel} (hN : NativesRespect P N)
    {s : State} {rest : List Frame} {fr : Frame} {info : FunctionInfo} {code : CodeUnit}
    {instr : Bytecode} {h : FunctionHandleIndex} {tys : List RuntimeType}
    {callee : FunctionRef} {cinfo : FunctionInfo} {fh : FunctionHandle}
    {before after params returns : List RuntimeType}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    (hinstr : code.code[fr.pc]? = some instr) (hsucc : fr.pc + 1 ∈ successors code.code fr.pc)
    (hav : ∀ a, availabilityAfter instr a = a)
    (hbefore : tc fr.function fr.pc = some before)
    (hafter : typeAfter P fr.function info.module instr before = some after)
    (hstack : conformsAll P fr.stack before = true)
    (hres : P.resolve fr.function.module h = some callee)
    (hvis : P.visibleFrom info.moduleId callee = true) (hcf : P.function? callee = some cinfo)
    (hfh : info.module.functionHandles[h]? = some fh)
    (hlen : fh.typeParameters.length = tys.length)
    (hclen : cinfo.handle.typeParameters.length = tys.length)
    (hwfa : RuntimeType.argumentsWellFormed P tys cinfo.handle.typeParameters = true)
    (hpar' : P.paramTypes { callee with typeArgs := tys } = some params)
    (hret' : P.returnTypes { callee with typeArgs := tys } = some returns)
    (hparams : P.handleParamTypesAt fr.function.module h tys = some params)
    (hreturns : P.handleReturnTypesAt fr.function.module h tys = some returns)
    (hpre : RuntimeType.assignableAll (before.take params.length) params.reverse = true)
    (hta : returns.reverse ++ before.drop params.length = after)
    (hnm : P.nativeShape? callee = none) :
    ExecSafe P N (Inv P tc ac) (Step.call P s fr rest info h tys) := by
  replace hnm : P.nativeShape? { callee with typeArgs := tys } = none := hnm
  have hcf' : P.function? { callee with typeArgs := tys } = some cinfo := by
    rw [function?_typeArgs]; exact hcf
  have htargs : P.typeArgsOk { callee with typeArgs := tys } = true := by
    simp only [Program.typeArgsOk, hcf']; exact hwfa
  have hslen := conformsAll_length hstack
  have htl := assignableAll_length hpre
  have hge : params.length ≤ fr.stack.length := by
    rw [hslen]; simp only [List.length_take, List.length_reverse] at htl; omega
  have hargs : conformsAll P (fr.stack.take params.length).reverse params = true := by
    have := conformsAll_reverse
      (conformsAll_assignable (conformsAll_take params.length hstack) hpre)
    rwa [List.reverse_reverse] at this
  have hdrop : conformsAll P (fr.stack.drop params.length) (before.drop params.length) = true :=
    conformsAll_drop params.length hstack
  have hresume : ResumeOk P tc ac { fr with stack := fr.stack.drop params.length } returns := by
    intro vals hvals
    refine ⟨info, code, activeAt_succ hc hact hinstr hbefore hafter rfl hsucc ?_ ?_ ?_ ?_⟩
    · rw [← hta]
      exact conformsAll_append (conformsAll_reverse hvals) hdrop
    · exact localsOk_of hact.locals rfl rfl
    · intro i hi
      rw [hav] at hi
      exact hact.available i hi
    · intro i hi
      rw [hav] at hi
      exact hact.unavailable i hi
  simp only [Step.call, Step.need, hres, hvis, hcf, hfh, hlen, hclen, hwfa, hpar', hret',
    hparams, hreturns, Step.popN, Nat.not_lt.2 hge, hargs, bne_self_eq_false, Bool.or_false,
    Bool.not_true, Bool.false_eq_true, ↓reduceIte, bind, Except.bind, pure, Except.pure]
  cases hcode : cinfo.definition.code with
  | none =>
    have hval : ((params ++ returns).all RuntimeType.isValue) = true := by
      rw [List.all_eq_true]
      intro t ht
      rcases List.mem_append.1 ht with ht | ht
      · exact (hc.nativeValues _ _ hcf' hcode htargs hnm).1 _ hpar' t ht
      · exact (hc.nativeValues _ _ hcf' hcode htargs hnm).2 _ hret' t ht
    have hnoref : ∀ v ∈ (fr.stack.take params.length).reverse, ∀ r, v ≠ .ref r :=
      noRef_of_conformsAll hargs ((hc.nativeValues _ _ hcf' hcode htargs hnm).1 _ hpar')
    simp only [Option.isNone_none, hval, Bool.not_true, Bool.and_false, Bool.false_eq_true,
      ↓reduceIte, nativeView_of_noRefs _ hnoref, exclusiveArguments_of_noRefs hnoref,
      Bool.not_true]
    intro r hr
    cases r with
    | failed e => trivial
    | returned vals writes places =>
      obtain ⟨tys', htys, hconf⟩ := hN _ _ vals writes places hr
      rw [hret', Option.some.injEq] at htys
      subst htys
      rw [nativeResultFits_none hnm] at hconf
      rw [resumeNative_values rfl hret' hnm hnoref
        ((hc.nativeValues _ _ hcf' hcode htargs hnm).2 _ hret'), hconf]
      simp only [Bool.and_eq_true] at hconf
      exact ⟨_, rest, rfl, hresume vals hconf.1.1, hchain⟩
  | some ccode =>
    simp only [Option.isNone_some, Bool.false_and, Bool.false_eq_true, ↓reduceIte]
    obtain ⟨ltys, hltys⟩ := hc.locals _ cinfo ccode hcf' hcode htargs
    obtain ⟨ps, ls, hps, rfl⟩ := localTypes_split hltys
    rw [hpar', Option.some.injEq] at hps
    subst hps
    simp only [hltys]
    repeat' split
    all_goals first
      | trivial
      | exact ⟨_, _, rfl,
          ⟨cinfo, ccode, entry_activeAt hc hcf' hcode htargs hpar' hltys hargs 0 rfl [] rfl
            _ _ _⟩,
          ⟨returns, hret', hresume⟩, hchain⟩

theorem call_safe (hc : Certified P tc ac) {N : NativeRel} (hN : NativesRespect P N)
    {s : State} {rest : List Frame} {fr : Frame} {info : FunctionInfo} {code : CodeUnit}
    {h : FunctionHandleIndex} {before after : List RuntimeType}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    (hinstr : code.code[fr.pc]? = some (.call h))
    (hbefore : tc fr.function fr.pc = some before)
    (hafter : typeAfter P fr.function info.module (.call h) before = some after)
    (hstack : conformsAll P fr.stack before = true)
    (hnm : NoModeledCall P fr.function.module info.module (.call h)) :
    ExecSafe P N (Inv P tc ac) (Step.call P s fr rest info h) := by
  obtain ⟨callee, cinfo, hres, hvis, hcf, htp, hpar, hret⟩ :=
    hc.calls fr.function info code hact.fn hact.body fr.pc h hact.reachable hinstr
  have hceq := withTypeArgs_self (resolve_typeArgs hres)
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨cfh, hcfh, hta⟩ := hta
  split at hta
  · simp at hta
  next hne =>
  have hcfh0 : cfh.typeParameters = [] := by simpa using hne
  simp only [Option.bind_eq_some_iff] at hta
  obtain ⟨params, hparams, returns, hreturns, hta⟩ := hta
  split at hta
  · next hpre =>
    simp only [Option.some.injEq] at hta
    exact callWith_safe hc hN hact hchain hinstr (by simp [successors, hinstr, Bytecode.control])
      (fun _ => rfl) hbefore hafter hstack hres hvis hcf hcfh (by simp [hcfh0]) (by simp [htp])
      (by simp [htp, RuntimeType.argumentsWellFormed]) (by rw [hceq]; exact hpar.trans hparams)
      (by rw [hceq]; exact hret.trans hreturns) hparams hreturns hpre hta (hnm _ hres)
  · simp at hta

/-- The type arguments a generic form supplies, as `typeAfter` reads them. -/
theorem genericArgs_of_typeArgsWith {m : CompiledModule} {tys : List RuntimeType}
    {sig : SignatureIndex} {targs : List RuntimeType} (h : m.typeArgsWith tys sig = some targs)
    (hne : targs ≠ []) :
    Step.genericArgs m tys sig = .ok targs := by
  unfold CompiledModule.typeArgsWith at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨toks, htoks, h⟩ := h
  cases targs with
  | nil => exact absurd rfl hne
  | cons t ts =>
    simp [Step.genericArgs, Step.need, htoks, h, bind, Except.bind, pure, Except.pure]

/-- `CallGeneric`: a call at the type arguments its instantiation supplies. -/
theorem callGeneric_safe (hc : Certified P tc ac) {N : NativeRel} (hN : NativesRespect P N)
    {s : State} {rest : List Frame} {fr : Frame} {info : FunctionInfo} {code : CodeUnit}
    {i : FunctionInstantiationIndex} {before after : List RuntimeType}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    (hinstr : code.code[fr.pc]? = some (.callGeneric i))
    (hbefore : tc fr.function fr.pc = some before)
    (hafter : typeAfter P fr.function info.module (.callGeneric i) before = some after)
    (hstack : conformsAll P fr.stack before = true)
    (hnm : NoModeledCall P fr.function.module info.module (.callGeneric i)) :
    ExecSafe P N (Inv P tc ac) (Step.exec P s fr rest info code (.callGeneric i)) := by
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨x, hx, fh, hfh, tys, ⟨targs, htargs, hmatch⟩, hta⟩ := hta
  obtain ⟨rfl, hne⟩ := typeArgsMatch_some hmatch
  obtain ⟨callee, cinfo, fh', hres, hvis, hcf, hfh', hclen, hcons, hsig⟩ :=
    hc.callsGeneric fr.function info code hact.fn hact.body fr.pc i x hact.reachable hinstr hx
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  split at hta
  · simp at hta
  next hwf =>
  have hwf' : RuntimeType.argumentsWellFormed P targs fh.typeParameters = true := by
    simpa using hwf
  have hlen := argumentsWellFormed_length hwf'
  simp only [Option.bind_eq_some_iff] at hta
  obtain ⟨params, hparams, returns, hreturns, hta⟩ := hta
  split at hta
  · next hpre =>
    simp only [Option.some.injEq] at hta
    obtain ⟨hpar, hret⟩ := hsig targs hlen
    dsimp only [Step.exec]
    simp only [Step.need, hx, pure, Except.pure, genericArgs_of_typeArgsWith htargs hne, bind,
      Except.bind]
    exact callWith_safe hc hN hact hchain hinstr (by simp [successors, hinstr, Bytecode.control])
      (fun _ => rfl) hbefore hafter hstack hres hvis hcf hfh hlen.symm (hclen.trans hlen.symm)
      (argumentsWellFormed_weaken hwf' hclen hcons) (hpar.trans hparams) (hret.trans hreturns)
      hparams hreturns hpre hta (hnm _ _ hx hres)
  · simp at hta

/-! ## Structs, enums and vectors -/

theorem popPush_some {tys : List RuntimeType} {r : RuntimeType} {before after : List RuntimeType}
    (h : popPush tys r before = some after) :
    RuntimeType.assignableAll (before.take tys.length) tys.reverse = true ∧
      after = r :: before.drop tys.length := by
  unfold popPush at h
  split at h
  · next hp =>
    simp only [Option.some.injEq] at h
    exact ⟨hp, h.symm⟩
  · simp at h

/-- A stack conforming to the types before `popPush` conforms to the popped
types themselves, with the rest below. -/
theorem popPush_conforms {tys : List RuntimeType} {r : RuntimeType}
    {before after : List RuntimeType} {stack : List Value}
    (h : popPush tys r before = some after) (hstack : conformsAll P stack before = true) :
    ∃ tail, after = r :: tail ∧ conformsAll P stack (tys.reverse ++ tail) = true := by
  obtain ⟨hp, rfl⟩ := popPush_some h
  refine ⟨before.drop tys.length, rfl, ?_⟩
  have h1 := conformsAll_assignable (conformsAll_take tys.length hstack) hp
  have h2 := conformsAll_drop tys.length hstack
  have := conformsAll_append h1 h2
  rwa [List.take_append_drop] at this

theorem popPushAll_some {t : RuntimeType} {tys before after : List RuntimeType}
    (h : popPushAll t tys before = some after) :
    ∃ tail, before = t :: tail ∧ after = tys.reverse ++ tail := by
  unfold popPushAll at h
  split at h
  · next t' rest =>
    split at h
    · next he => subst he; simp only [Option.some.injEq] at h; exact ⟨rest, rfl, h.symm⟩
    · simp at h
  · simp at h

/-- Popping the values of `tys`, the last on top, from a stack that conforms
to them. -/
theorem popN_conforms {fr : Frame} {tys tail : List RuntimeType}
    (h : conformsAll P fr.stack (tys.reverse ++ tail) = true) :
    Step.popN fr tys.length =
        .ok ((fr.stack.take tys.length).reverse, { fr with stack := fr.stack.drop tys.length }) ∧
      conformsAll P (fr.stack.take tys.length).reverse tys = true ∧
      conformsAll P (fr.stack.drop tys.length) tail = true := by
  have hn : tys.reverse.length = tys.length := List.length_reverse
  have hlen := conformsAll_length h
  have hge : tys.length ≤ fr.stack.length := by rw [hlen]; simp
  refine ⟨by simp [Step.popN, Nat.not_lt.2 hge, pure, Except.pure], ?_, ?_⟩
  · have := conformsAll_reverse (conformsAll_take tys.length h)
    rwa [List.take_left' hn, List.reverse_reverse] at this
  · have := conformsAll_drop tys.length h
    rwa [List.drop_left' hn] at this

theorem fieldTypes_some {s : StructId} {tys : List RuntimeType} (h : P.fieldTypes s = some tys) :
    ∃ info fields, P.structType? s = some info ∧
      info.definition.fieldInformation = .declared fields := by
  unfold Program.fieldTypes at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, hinfo, h⟩ := h
  split at h
  · next fields hf => exact ⟨info, fields, hinfo, hf⟩
  · simp at h
  · simp at h

theorem variantFieldTypes_some {s : StructId} {tag : VariantIndex} {tys : List RuntimeType}
    (h : P.variantFieldTypes s tag = some tys) :
    ∃ info variants vd, P.structType? s = some info ∧
      info.definition.fieldInformation = .declaredVariants variants ∧ variants[tag]? = some vd := by
  unfold Program.variantFieldTypes at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, hinfo, h⟩ := h
  split at h
  · next variants hf =>
    simp only [Option.bind_eq_some_iff] at h
    obtain ⟨vd, hvd, -⟩ := h
    exact ⟨info, variants, vd, hinfo, hf, hvd⟩
  · simp at h
  · simp at h

/-- A value of a declared struct type is a struct value or a table, with
conforming fields. -/
theorem conforms_struct {v : Value} {s : StructId} {tys : List RuntimeType}
    (hv : v.conforms P (.struct s) = true) (hf : P.fieldTypes s = some tys) :
    ∃ fields, StructLike v s fields ∧ conformsAll P fields tys = true := by
  have hty := conforms_type hv rfl
  have hw := conforms_wellFormed hv
  cases v with
  | struct s' fields =>
    simp only [Value.type, RuntimeType.struct.injEq] at hty
    subst hty
    obtain ⟨-, tys', htys', hc⟩ := wellFormed_struct.1 hw
    rw [hf, Option.some.injEq] at htys'
    subst htys'
    exact ⟨fields, Or.inl rfl, hc⟩
  | table s' fields ks es =>
    simp only [Value.type, RuntimeType.struct.injEq] at hty
    subst hty
    obtain ⟨-, -, ⟨tys', htys', hc⟩, -⟩ := wellFormed_table.1 hw
    rw [hf, Option.some.injEq] at htys'
    subst htys'
    exact ⟨fields, Or.inr ⟨ks, es, rfl⟩, hc⟩
  | variant s' tag fields =>
    simp only [Value.type, RuntimeType.struct.injEq] at hty
    subst hty
    obtain ⟨tys', htys', -⟩ := wellFormed_variant.1 hw
    obtain ⟨info, fields', hinfo, hfi⟩ := fieldTypes_some hf
    obtain ⟨info', variants, -, hinfo', hfi', -⟩ := variantFieldTypes_some htys'
    rw [hinfo, Option.some.injEq] at hinfo'
    subst hinfo'
    rw [hfi] at hfi'
    cases hfi'
  | _ => simp [Value.type] at hty

theorem StructLike.child_field {v : Value} {s : StructId} {fs : List Value} (h : StructLike v s fs)
    (i : Nat) : v.child? (.field i) = fs[i]? := by
  rcases h with rfl | ⟨ks, es, rfl⟩ <;> rfl

/-- A value of an enum type is a variant value whose fields conform to its
variant's. -/
theorem conforms_variant {v : Value} {s : StructId} {tag0 : VariantIndex}
    {tys0 : List RuntimeType} (hv : v.conforms P (.struct s) = true)
    (hf : P.variantFieldTypes s tag0 = some tys0) :
    ∃ tag fields tys, v = .variant s tag fields ∧ P.variantFieldTypes s tag = some tys ∧
      conformsAll P fields tys = true := by
  have hty := conforms_type hv rfl
  have hw := conforms_wellFormed hv
  cases v with
  | variant s' tag fields =>
    simp only [Value.type, RuntimeType.struct.injEq] at hty
    subst hty
    obtain ⟨tys, htys, hc⟩ := wellFormed_variant.1 hw
    exact ⟨tag, fields, tys, rfl, htys, hc⟩
  | struct s' fields =>
    simp only [Value.type, RuntimeType.struct.injEq] at hty
    subst hty
    obtain ⟨-, tys', htys', -⟩ := wellFormed_struct.1 hw
    obtain ⟨info, fields', hinfo, hfi⟩ := fieldTypes_some htys'
    obtain ⟨info', variants, -, hinfo', hfi', -⟩ := variantFieldTypes_some hf
    rw [hinfo, Option.some.injEq] at hinfo'
    subst hinfo'
    rw [hfi] at hfi'
    cases hfi'
  | table s' fields ks es =>
    simp only [Value.type, RuntimeType.struct.injEq] at hty
    subst hty
    obtain ⟨-, -, ⟨tys', htys', -⟩, -⟩ := wellFormed_table.1 hw
    obtain ⟨info, fields', hinfo, hfi⟩ := fieldTypes_some htys'
    obtain ⟨info', variants, -, hinfo', hfi', -⟩ := variantFieldTypes_some hf
    rw [hinfo, Option.some.injEq] at hinfo'
    subst hinfo'
    rw [hfi] at hfi'
    cases hfi'
  | _ => simp [Value.type] at hty

theorem conforms_vector {v : Value} {t : RuntimeType} (hv : v.conforms P (.vector t) = true) :
    ∃ elems, v = .vector t elems ∧ ∀ w ∈ elems, w.conforms P t = true := by
  have hty := conforms_type hv rfl
  have hw := conforms_wellFormed hv
  cases v with
  | vector t' elems =>
    simp only [Value.type, RuntimeType.vector.injEq] at hty
    subst hty
    exact ⟨elems, rfl, wellFormed_vector.1 hw⟩
  | _ => simp [Value.type] at hty

theorem definedStruct_struct {mi : Nat} {d : StructDefinitionIndex} {s : StructId}
    (h : P.definedStruct? mi d = some s) : ∃ sinfo, P.struct? ⟨mi, d⟩ = some sinfo := by
  unfold Program.definedStruct? at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, hinfo, -⟩ := h
  exact ⟨info, hinfo⟩

/-- The instance a generic form names, when it is a type of the program. -/
theorem localStruct_ok_gen {mi : Nat} {d : StructDefinitionIndex} {s : StructId}
    {tys : List RuntimeType} {info : StructInfo} (h : P.definedStruct? mi d = some s)
    (hinfo : P.structType? { s with typeArgs := tys } = some info)
    (hwf : (RuntimeType.struct { s with typeArgs := tys }).wellFormed P = true) :
    Step.localStruct P mi d tys = .ok ({ s with typeArgs := tys }, info) := by
  obtain ⟨sinfo, hs⟩ := definedStruct_struct h
  simp [Step.localStruct, Step.need, hs, h, hinfo, hwf, bind, Except.bind, pure, Except.pure]

theorem localStruct_ok {mi : Nat} {d : StructDefinitionIndex} {s : StructId} {info : StructInfo}
    (h : P.definedStruct? mi d = some s) (hinfo : P.structType? s = some info) :
    Step.localStruct P mi d = .ok (s, info) := by
  obtain ⟨sinfo, hs⟩ := definedStruct_struct h
  have hta := definedStruct?_typeArgs h
  have hseq : { s with typeArgs := [] } = s := by cases s; simp_all
  have hlen := (structType?_parts hinfo).2
  rw [hta] at hlen
  have htp : info.handle.typeParameters = [] := List.eq_nil_of_length_eq_zero hlen.symm
  simp [Step.localStruct, Step.need, hs, h, hseq, hinfo, RuntimeType.wellFormed, htp,
    RuntimeType.argumentsWellFormed, bind, Except.bind, pure, Except.pure]

theorem elemType_ok {m : CompiledModule} {sig : SignatureIndex} {tys : List RuntimeType}
    {t : RuntimeType} (h : m.vectorElemTypeWith tys sig = some t) :
    Step.elemType m sig tys = .ok t := by
  unfold CompiledModule.vectorElemTypeWith at h
  unfold Step.elemType
  cases hs : m.signatures[sig]? with
  | none => simp [hs] at h
  | some l =>
    rw [hs] at h
    rcases l with _ | ⟨a, _ | ⟨b, l⟩⟩ <;>
      simp_all [Step.need, bind, Except.bind, pure, Except.pure]

theorem messageBytes_some :
    ∀ (elems : List Value), (∀ w ∈ elems, w.conforms P (.int .u8) = true) →
      ∃ bytes, Step.messageBytes elems = some bytes
  | [], _ => ⟨[], rfl⟩
  | w :: ws, h => by
    obtain ⟨x, rfl, -⟩ := conforms_int (h w (List.mem_cons_self ..))
    obtain ⟨bytes, hb⟩ := messageBytes_some ws (fun v hv => h v (List.mem_cons_of_mem _ hv))
    exact ⟨UInt8.ofNat x.toNat :: bytes, by simp [Step.messageBytes, hb, bind, pure]⟩

/-! ## Closures -/

theorem filter_zipIdx_split {α : Type} (p : Nat → Bool) : ∀ (xs : List α) (k : Nat),
    ((xs.zipIdx k).filter fun x => p x.2).length +
      ((xs.zipIdx k).filter fun x => !p x.2).length = xs.length
  | [], _ => rfl
  | x :: xs, k => by
    have := filter_zipIdx_split p xs (k + 1)
    cases hb : p k <;> simp [List.zipIdx_cons, hb] <;> omega

theorem conformsAll_maskCompose_go {mask : BitVec 64} : ∀ (ts : List RuntimeType) (k : Nat)
    {cs vs : List Value},
    conformsAll P cs (((ts.zipIdx k).filter fun x => mask.getLsbD x.2).map (·.1)) = true →
    conformsAll P vs (((ts.zipIdx k).filter fun x => !mask.getLsbD x.2).map (·.1)) = true →
    conformsAll P (maskCompose.go ((List.range' k ts.length).map mask.getLsbD) cs vs) ts = true
  | [], k, cs, vs, _, _ => by simp [maskCompose.go]
  | t :: ts, k, cs, vs, hc, hv => by
    simp only [List.zipIdx_cons, List.filter_cons, List.length_cons, List.range'_succ,
      List.map_cons] at hc hv ⊢
    cases hb : mask.getLsbD k
    · simp only [hb, Bool.not_false, ↓reduceIte, Bool.false_eq_true, List.map_cons] at hc hv
      obtain ⟨v, vs', rfl, hv1, hv2⟩ := conformsAll_cons_right hv
      simp only [maskCompose.go, conformsAll_cons]
      exact ⟨hv1, conformsAll_maskCompose_go ts (k + 1) hc hv2⟩
    · simp only [hb, Bool.not_true, ↓reduceIte, Bool.false_eq_true, List.map_cons] at hc hv
      obtain ⟨c, cs', rfl, hc1, hc2⟩ := conformsAll_cons_right hc
      simp only [maskCompose.go, conformsAll_cons]
      exact ⟨hc1, conformsAll_maskCompose_go ts (k + 1) hc2 hv⟩

/-- A closure's captured values and the provided ones, in parameter order,
conform to the parameters. -/
theorem conformsAll_maskCompose {mask : BitVec 64} {ts ctys rem : List RuntimeType}
    {cs vs : List Value} (hsplit : maskSplit mask ts = (ctys, rem))
    (hc : conformsAll P cs ctys = true) (hv : conformsAll P vs rem = true) :
    conformsAll P (maskCompose mask cs vs) ts = true := by
  simp only [maskSplit, Prod.mk.injEq] at hsplit
  obtain ⟨rfl, rfl⟩ := hsplit
  have hlen : cs.length + vs.length = ts.length := by
    rw [conformsAll_length hc, conformsAll_length hv]
    simpa using filter_zipIdx_split mask.getLsbD ts 0
  unfold maskCompose
  rw [hlen, List.range_eq_range']
  exact conformsAll_maskCompose_go ts 0 (by simpa using hc) (by simpa using hv)

/-- A value of a function type is a closure with that type's parameters and
results and at least its abilities. -/
theorem conforms_function {v : Value} {ps rs : List RuntimeType} {a : AbilitySet}
    (h : v.conforms P (.function ps rs a) = true) :
    ∃ target mask captured ca, v = .closure target mask captured ps rs ca ∧
      a.subset ca = true ∧ v.wellFormed P = true := by
  obtain ⟨ht, hw⟩ := conforms_iff.1 h
  cases v <;> simp [Value.type, RuntimeType.assignable] at ht
  case closure target mask captured cps crs ca =>
    obtain ⟨⟨rfl, rfl⟩, hs⟩ := ht
    exact ⟨target, mask, captured, ca, rfl, hs, hw⟩

/-- The parts of a closure's type. -/
theorem closureType?_parts {f : FunctionRef} {mask : BitVec 64}
    {ctys rem rts : List RuntimeType} (h : P.closureType? f mask = some (ctys, rem, rts)) :
    ∃ info code params, P.function? f = some info ∧ info.definition.code = some code ∧
      P.typeArgsOk f = true ∧ P.paramTypes f = some params ∧ P.returnTypes f = some rts ∧
      maskSplit mask params = (ctys, rem) := by
  unfold Program.closureType? at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, hinfo, code, hcode, h⟩ := h
  split at h
  · simp at h
  · next hok =>
    obtain ⟨params, hps, h⟩ := Option.bind_eq_some_iff.1 h
    obtain ⟨results, hrs, h⟩ := Option.bind_eq_some_iff.1 h
    split at h
    · simp at h
    · split at h
      · simp at h
      · simp only [pure, Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, rfl, rfl⟩ := h
        exact ⟨info, code, params, hinfo, hcode, by simpa using hok, hps, hrs, rfl⟩

/-- A closure over handle `h` at type arguments `tys`, which the certificate
types as `typeAfter`'s `closureOf` does, given the target's resolution and
linking. -/
theorem packWith_safe (hc : Certified P tc ac) {N : NativeRel}
    {s : State} {rest : List Frame} {fr : Frame} {info : FunctionInfo} {code : CodeUnit}
    {instr : Bytecode} {h : FunctionHandleIndex} {mask : BitVec 64} {tys : List RuntimeType}
    {target : FunctionRef} {tinfo : FunctionInfo} {fh : FunctionHandle}
    {before after ps rs captured remaining : List RuntimeType}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    (hinstr : code.code[fr.pc]? = some instr) (hctrl : instr.control = .fallThrough)
    (hav : ∀ a, availabilityAfter instr a = a)
    (hbefore : tc fr.function fr.pc = some before)
    (hafter : typeAfter P fr.function info.module instr before = some after)
    (hstack : conformsAll P fr.stack before = true)
    (hres : P.resolve fr.function.module h = some target)
    (hvis : P.visibleFrom info.moduleId target = true) (htf : P.function? target = some tinfo)
    (htcode : tinfo.definition.code.isSome = true)
    (hfh : info.module.functionHandles[h]? = some fh)
    (hlen : fh.typeParameters.length = tys.length)
    (hclen : tinfo.handle.typeParameters.length = tys.length)
    (hwfa : RuntimeType.argumentsWellFormed P tys tinfo.handle.typeParameters = true)
    (hpar : P.paramTypes { target with typeArgs := tys } = some ps)
    (hret : P.returnTypes { target with typeArgs := tys } = some rs)
    (hparams : P.handleParamTypesAt fr.function.module h tys = some ps)
    (hreturns : P.handleReturnTypesAt fr.function.module h tys = some rs)
    (hwithin : maskWithin mask ps.length = true)
    (hsplit : maskSplit mask ps = (captured, remaining))
    (hval : captured.all RuntimeType.isValue = true)
    (hpp : popPush captured (.function remaining rs
        ((P.closureAbilities { target with typeArgs := tys }).inter
          (RuntimeType.capturedAbilities P (before.take captured.length)))) before = some after) :
    ExecSafe P N (Inv P tc ac) (Step.packClosure P s fr rest info h mask tys) := by
  have hpp' := hpp
  obtain ⟨tail, rfl, hconf⟩ := popPush_conforms hpp hstack
  obtain ⟨hpop, hcapt, hrest⟩ := popN_conforms (fr := fr) hconf
  have htn : tinfo.definition.code.isNone = false := by
    cases hcd : tinfo.definition.code <;> simp_all
  obtain ⟨tcode, htcode'⟩ : ∃ c, tinfo.definition.code = some c := by
    cases hcd : tinfo.definition.code with
    | none => simp [hcd] at htcode
    | some c => exact ⟨c, rfl⟩
  have htf' : P.function? { target with typeArgs := tys } = some tinfo := by
    rw [function?_typeArgs]; exact htf
  have hct : P.closureType? { target with typeArgs := tys } mask = some (captured, remaining, rs) := by
    have hok : P.typeArgsOk { target with typeArgs := tys } = true := by
      simp only [Program.typeArgsOk, htf']; exact hwfa
    simp only [Program.closureType?, htf', htcode', hok, hpar, hret, hwithin, hsplit, hval, bind,
      Option.bind_some, Bool.not_true, pure]
    rfl
  -- The closure has at least the abilities the certificate gives it.
  have hab : ((P.closureAbilities { target with typeArgs := tys }).inter
      (RuntimeType.capturedAbilities P (before.take captured.length))).subset
      (P.closureValueAbilities { target with typeArgs := tys }
        (fr.stack.take captured.length).reverse) = true := by
    refine AbilitySet.subset_inter.2 ⟨AbilitySet.inter_subset_left _ _,
      ability_subset_trans (AbilitySet.inter_subset_right _ _) (capturedAbilities_ge ?_)⟩
    intro u hu
    simp only [List.map_reverse, List.mem_reverse, List.mem_map] at hu
    obtain ⟨v, hv, rfl⟩ := hu
    obtain ⟨t, ht, hvt⟩ := conformsAll_mem (conformsAll_take captured.length hstack) v hv
    exact ability_subset_trans (capturedAbilities_sub ht)
      (assignable_abilities (conforms_iff.1 hvt).1)
  simp only [Step.packClosure, Step.need, hres, hvis, htf, hfh, hlen, hclen, hwfa, hpar, hret,
    hparams, hreturns, bne_self_eq_false, Bool.or_false, Bool.not_true, Bool.false_eq_true,
    ↓reduceIte, bind, Except.bind, pure, Except.pure, htn, hwithin, hsplit, hval, hpop, hcapt]
  refine fallThrough_safe hc hact hchain hinstr hctrl hav hbefore hafter rfl rfl rfl ?_
  simp only [Step.advance, Step.push, conformsAll_cons]
  refine ⟨conforms_of ?_ ?_, hrest⟩
  · simp [Value.type, RuntimeType.assignable, hab]
  · simp [Value.wellFormed, hct, hcapt]

theorem packClosure_safe (hc : Certified P tc ac) {N : NativeRel}
    {s : State} {rest : List Frame} {fr : Frame} {info : FunctionInfo} {code : CodeUnit}
    {h : FunctionHandleIndex} {mask : BitVec 64} {before after : List RuntimeType}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    (hinstr : code.code[fr.pc]? = some (.packClosure h mask))
    (hbefore : tc fr.function fr.pc = some before)
    (hafter : typeAfter P fr.function info.module (.packClosure h mask) before = some after)
    (hstack : conformsAll P fr.stack before = true) :
    ExecSafe P N (Inv P tc ac) (Step.exec P s fr rest info code (.packClosure h mask)) := by
  obtain ⟨target, tinfo, fh', hres, hvis, htf, htcode, hfh', hclen, hcons, hsig⟩ :=
    hc.closures fr.function info code hact.fn hact.body fr.pc _ h hact.reachable hinstr rfl
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨fh, hfh, hta⟩ := hta
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  split at hta
  · simp at hta
  next hne =>
  have hfh0 : fh.typeParameters = [] := by simpa using hne
  simp only [Option.bind_eq_some_iff] at hta
  obtain ⟨ps, hparams, rs, hreturns, target', hres', hta⟩ := hta
  rw [hres, Option.some.injEq] at hres'
  subst hres'
  split at hta
  · simp at hta
  next hwithin =>
  rcases hsplit : maskSplit mask ps with ⟨captured, remaining⟩
  rw [hsplit] at hta
  dsimp only at hta
  split at hta
  · simp at hta
  next hval =>
  obtain ⟨hpar, hret⟩ := hsig [] (by simp [hfh0])
  have htp : tinfo.handle.typeParameters = [] :=
    List.eq_nil_of_length_eq_zero (by simp [hclen, hfh0])
  dsimp only [Step.exec]
  exact packWith_safe hc hact hchain hinstr rfl (fun _ => rfl) hbefore hafter hstack hres hvis htf
    htcode hfh (by simp [hfh0]) (by simp [htp]) (by rw [htp]; rfl) (hpar.trans hparams)
    (hret.trans hreturns) hparams hreturns (by simpa using hwithin) hsplit (by simpa using hval) hta

theorem packClosureGeneric_safe (hc : Certified P tc ac) {N : NativeRel}
    {s : State} {rest : List Frame} {fr : Frame} {info : FunctionInfo} {code : CodeUnit}
    {i : FunctionInstantiationIndex} {mask : BitVec 64} {before after : List RuntimeType}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    (hinstr : code.code[fr.pc]? = some (.packClosureGeneric i mask))
    (hbefore : tc fr.function fr.pc = some before)
    (hafter : typeAfter P fr.function info.module (.packClosureGeneric i mask) before = some after)
    (hstack : conformsAll P fr.stack before = true) :
    ExecSafe P N (Inv P tc ac) (Step.exec P s fr rest info code (.packClosureGeneric i mask)) := by
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨x, hx, fh, hfh, tys, ⟨targs, htargs, hmatch⟩, hta⟩ := hta
  obtain ⟨rfl, hne⟩ := typeArgsMatch_some hmatch
  obtain ⟨target, tinfo, fh', hres, hvis, htf, htcode, hfh', hclen, hcons, hsig⟩ :=
    hc.closures fr.function info code hact.fn hact.body fr.pc _ x.handle hact.reachable hinstr
      (by simp [Bytecode.closureHandle?, hx])
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  split at hta
  · simp at hta
  next hwf =>
  have hwf' : RuntimeType.argumentsWellFormed P targs fh.typeParameters = true := by
    simpa using hwf
  have hlen := argumentsWellFormed_length hwf'
  simp only [Option.bind_eq_some_iff] at hta
  obtain ⟨ps, hparams, rs, hreturns, target', hres', hta⟩ := hta
  rw [hres, Option.some.injEq] at hres'
  subst hres'
  split at hta
  · simp at hta
  next hwithin =>
  rcases hsplit : maskSplit mask ps with ⟨captured, remaining⟩
  rw [hsplit] at hta
  dsimp only at hta
  split at hta
  · simp at hta
  next hval =>
  obtain ⟨hpar, hret⟩ := hsig targs hlen
  dsimp only [Step.exec]
  simp only [Step.need, hx, pure, Except.pure, genericArgs_of_typeArgsWith htargs hne, bind,
    Except.bind]
  exact packWith_safe hc hact hchain hinstr rfl (fun _ => rfl) hbefore hafter hstack hres hvis htf
    htcode hfh hlen.symm (hclen.trans hlen.symm) (argumentsWellFormed_weaken hwf' hclen hcons)
    (hpar.trans hparams) (hret.trans hreturns) hparams hreturns (by simpa using hwithin) hsplit
    (by simpa using hval) hta

/-- `CallClosure`: the closure on top, of the instruction's function type,
runs its target, a function with code, with its captured values and the
provided ones, which conform to the target's parameters; the target returns
the closure's results. -/
theorem callClosure_safe (hc : Certified P tc ac) {N : NativeRel}
    {s : State} {rest : List Frame} {fr : Frame} {info : FunctionInfo} {code : CodeUnit}
    {sig : SignatureIndex} {before after : List RuntimeType}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    (hinstr : code.code[fr.pc]? = some (.callClosure sig))
    (hbefore : tc fr.function fr.pc = some before)
    (hafter : typeAfter P fr.function info.module (.callClosure sig) before = some after)
    (hstack : conformsAll P fr.stack before = true) :
    ExecSafe P N (Inv P tc ac) (Step.exec P s fr rest info code (.callClosure sig)) := by
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨toks, htoks, hta⟩ := hta
  split at hta
  rotate_left
  · simp at hta
  next ps0 rs0 a =>
  simp only [Option.bind_eq_some_iff] at hta
  obtain ⟨ps, hps, rs, hrs, hta⟩ := hta
  split at hta
  rotate_left
  · simp at hta
  next ps' rs' a' brest =>
  split at hta
  rotate_left
  · simp at hta
  next hcond =>
  simp only [Option.some.injEq] at hta
  subst hta
  simp only [Bool.and_eq_true, decide_eq_true_eq] at hcond
  obtain ⟨⟨⟨h1, h2⟩, hsub⟩, hpre⟩ := hcond
  subst ps' rs'
  obtain ⟨c, vs, hcs, hc1, hvs⟩ := conformsAll_cons_right hstack
  obtain ⟨target, mask, captured, ca, rfl, hsub', hw⟩ := conforms_function hc1
  simp only [Value.wellFormed] at hw
  split at hw
  rotate_left
  · simp at hw
  next ctys rem rts hct =>
  simp only [Bool.and_eq_true, beq_iff_eq] at hw
  obtain ⟨⟨⟨rfl, rfl⟩, -⟩, hccap⟩ := hw
  obtain ⟨tinfo, tcode, params, htf, htcode, htok, hpar, hret, hsplit⟩ := closureType?_parts hct
  have hprov : conformsAll P vs (ps.reverse ++ brest.drop ps.length) = true := by
    have h1 := conformsAll_assignable (conformsAll_take ps.length hvs) hpre
    have h2 := conformsAll_drop ps.length hvs
    have := conformsAll_append h1 h2
    rwa [List.take_append_drop] at this
  obtain ⟨hpop, hargs, hrest⟩ := popN_conforms (fr := { fr with stack := vs }) hprov
  have hcomp := conformsAll_maskCompose (cs := captured) hsplit hccap hargs
  obtain ⟨ltys, hltys⟩ := hc.locals target tinfo tcode htf htcode htok
  obtain ⟨ps2, ls, hps2, rfl⟩ := localTypes_split hltys
  rw [hpar, Option.some.injEq] at hps2
  subst hps2
  have hresume : ResumeOk P tc ac { fr with stack := vs.drop ps.length } rs := by
    intro vals hvals
    refine ⟨info, code, activeAt_succ hc hact hinstr hbefore hafter rfl
      (by simp [successors, hinstr, Bytecode.control]) ?_ ?_ ?_ ?_⟩
    · exact conformsAll_append (conformsAll_reverse hvals) hrest
    · exact localsOk_of hact.locals rfl rfl
    · intro i hi
      exact hact.available i hi
    · intro i hi
      exact hact.unavailable i hi
  dsimp only [Step.exec]
  simp only [Step.callClosure, Step.need, htoks, hps, hrs, Step.pop, hcs, bind, Except.bind, pure,
    Except.pure]
  simp only [bne_self_eq_false, Bool.or_false, ability_subset_trans hsub hsub', Bool.not_true,
    Bool.false_eq_true, ↓reduceIte, hpop, hargs, htf, htcode, hpar, hret, hcomp, hltys]
  repeat' split
  all_goals first
    | trivial
    | exact ⟨_, _, rfl,
        ⟨tinfo, tcode, entry_activeAt hc htf htcode htok hpar hltys hcomp 0 rfl [] rfl _ _ _⟩,
        ⟨rs, hret, hresume⟩, hchain⟩

/-- **The closure instructions** are safe and keep the invariant. -/
theorem execClosure_safe (hc : Certified P tc ac) {N : NativeRel}
    {s : State} {rest : List Frame} {fr : Frame} {info : FunctionInfo} {code : CodeUnit}
    {instr : Bytecode}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    (hinstr : code.code[fr.pc]? = some instr) (hcl : instr.inClosures = true) :
    ExecSafe P N (Inv P tc ac) (Step.exec P s fr rest info code instr) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc instr hact.reachable hinstr
  obtain ⟨tys, htys, hstack⟩ := hact.types
  rw [hbefore, Option.some.injEq] at htys
  subst htys
  cases instr <;> simp [Bytecode.inClosures] at hcl
  case packClosure h mask => exact packClosure_safe hc hact hchain hinstr hbefore hafter hstack
  case packClosureGeneric i mask =>
    exact packClosureGeneric_safe hc hact hchain hinstr hbefore hafter hstack
  case callClosure sig => exact callClosure_safe hc hact hchain hinstr hbefore hafter hstack

theorem exec_safe (hc : Certified P tc ac) {N : NativeRel} (hN : NativesRespect P N)
    {s : State} {rest : List Frame} {fr : Frame} {info : FunctionInfo} {code : CodeUnit}
    {instr : Bytecode}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    (hinstr : code.code[fr.pc]? = some instr)
    (hbv : (instr.inCore || instr.inDataByValue) = true)
    (heqv : instr = .eq ∨ instr = .neq →
      ∀ tys, tc fr.function fr.pc = some tys → ∀ t ∈ tys, t.isValue = true)
    (hnm : NoModeledCall P fr.function.module info.module instr) :
    ExecSafe P N (Inv P tc ac) (Step.exec P s fr rest info code instr) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, hsucc⟩ := htypes fr.pc instr hact.reachable hinstr
  obtain ⟨tys, htys, hstack⟩ := hact.types
  rw [hbefore, Option.some.injEq] at htys
  subst htys
  have ft := fun {fr' : Frame} (hctrl : instr.control = .fallThrough)
    (hav : ∀ a, availabilityAfter instr a = a) (hfun : fr'.function = fr.function)
    (hpc : fr'.pc = fr.pc + 1) (hloc : fr'.locals = fr.locals)
    (hstk : conformsAll P fr'.stack after = true) =>
    fallThrough_safe (N := N) (s := s) hc hact hchain hinstr hctrl hav hbefore hafter hfun hpc
      hloc hstk
  have sk := fun {fr' : Frame} (hav : ∀ a, availabilityAfter instr a = a)
    (hfun : fr'.function = fr.function) (hsucc : fr'.pc ∈ successors code.code fr.pc)
    (hloc : fr'.locals = fr.locals) (hstk : conformsAll P fr'.stack after = true) =>
    succ_safe (N := N) (s := s) hc hact hchain hinstr hbefore hafter hfun hsucc hstk
      (localsOk_of hact.locals hfun hloc)
    (fun i hi => by rw [hav] at hi; exact hloc ▸ hact.available i hi)
    (fun i hi => by rw [hav] at hi; exact hloc ▸ hact.unavailable i hi)
  cases instr
  case ldTrue | ldFalse =>
    dsimp only [typeAfter] at hafter; simp only [Option.some.injEq] at hafter
    subst hafter
    exact ft rfl (fun _ => rfl) rfl rfl rfl
      (by simpa [Step.advance, Step.push, Value.conforms, Value.type, Value.wellFormed] using hstack)
  case ldU8 v | ldU16 v | ldU32 v | ldU64 v | ldU128 v | ldU256 v
    | ldI8 v | ldI16 v | ldI32 v | ldI64 v | ldI128 v | ldI256 v =>
    dsimp only [typeAfter] at hafter; simp only [Option.some.injEq] at hafter
    subst hafter
    exact ft rfl (fun _ => rfl) rfl rfl rfl
      (by simp only [Step.ldInt, Step.advance, Step.push, conformsAll_cons, conforms_int_iff]
          exact ⟨⟨trivial, inRange_ofBitVec rfl⟩, hstack⟩)
  case add | sub | mul | div | mod =>
    dsimp only [typeAfter] at hafter
    obtain ⟨t, brest, rfl, -, rfl⟩ := intBinary_some hafter
    obtain ⟨y, st1, hst, hy, h1⟩ := conformsAll_int hstack
    obtain ⟨x, st2, rfl, hx, h2⟩ := conformsAll_int h1
    refine intBinop_safe hst rfl fun r hr => ?_
    have hr' : t.inRange r = true := by
      first
        | exact inRange_checked hr
        | exact inRange_rem hr
        | exact inRange_checked (by unfold IntType.div at hr; split at hr; simp at hr; exact hr)
    exact ft rfl (fun _ => rfl) rfl rfl rfl (by simp [Step.advance, Step.push, h2, hr'])
  case bitOr | bitAnd | xor =>
    dsimp only [typeAfter] at hafter
    obtain ⟨t, brest, rfl, hs, rfl⟩ := intBinary_some hafter
    simp only [Bool.not_eq_true'] at hs
    obtain ⟨y, st1, hst, hy, h1⟩ := conformsAll_int hstack
    obtain ⟨x, st2, rfl, hx, h2⟩ := conformsAll_int h1
    refine intBinop_safe hst (by simp [hs]) fun r hr => ?_
    have hr' : t.inRange r = true := by
      simp only [Option.some.injEq] at hr
      subst hr
      first
        | exact inRange_bitOr hs hx hy
        | exact inRange_bitAnd hs hx
        | exact inRange_bitXor hs hx hy
    exact ft rfl (fun _ => rfl) rfl rfl rfl (by simp [Step.advance, Step.push, h2, hr'])
  case lt | gt | le | ge =>
    dsimp only [typeAfter] at hafter
    obtain ⟨t, brest, rfl, -, rfl⟩ := intBinary_some hafter
    obtain ⟨y, st1, hst, hy, h1⟩ := conformsAll_int hstack
    obtain ⟨x, st2, rfl, hx, h2⟩ := conformsAll_int h1
    exact intCompare_safe hst
      (ft rfl (fun _ => rfl) rfl rfl rfl
        (by simp [Step.advance, Step.push, h2, Value.conforms, Value.type, Value.wellFormed]))
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256
    | castI8 | castI16 | castI32 | castI64 | castI128 | castI256 =>
    dsimp only [typeAfter] at hafter
    obtain ⟨t, brest, rfl, rfl⟩ := castTo_some hafter
    obtain ⟨x, st1, hst, hx, h1⟩ := conformsAll_int hstack
    exact cast_safe hst fun r hr => ft rfl (fun _ => rfl) rfl rfl rfl
      (by simp [Step.advance, Step.push, h1, inRange_checked hr])
  case shl | shr =>
    dsimp only [typeAfter] at hafter
    split at hafter
    · next t brest =>
      split at hafter
      · next hs =>
        simp only [Bool.not_eq_true', Option.some.injEq] at hs hafter
        subst hafter
        obtain ⟨n, st1, hst, -, h1⟩ := conformsAll_int hstack
        obtain ⟨x, st2, rfl, hx, h2⟩ := conformsAll_int h1
        refine shift_safe hst hs fun r hr => ?_
        have hr' : t.inRange r = true := by
          first
            | exact inRange_shl hs hr
            | exact inRange_shr hs hx hr
        exact ft rfl (fun _ => rfl) rfl rfl rfl (by simp [Step.advance, Step.push, h2, hr'])
      · simp at hafter
    · simp at hafter
  case or | and =>
    dsimp only [typeAfter] at hafter
    split at hafter
    · next brest =>
      simp only [Option.some.injEq] at hafter
      subst hafter
      obtain ⟨b, st1, hst, h1⟩ := conformsAll_bool hstack
      obtain ⟨a, st2, rfl, h2⟩ := conformsAll_bool h1
      exact boolBinop_safe hst
        (ft rfl (fun _ => rfl) rfl rfl rfl
          (by simp [Step.advance, Step.push, h2, Value.conforms, Value.type, Value.wellFormed]))
    · simp at hafter
  case not =>
    dsimp only [typeAfter] at hafter
    split at hafter
    · next brest =>
      simp only [Option.some.injEq] at hafter
      subst hafter
      obtain ⟨b, st1, hst, h1⟩ := conformsAll_bool hstack
      dsimp only [Step.exec]
      simp only [Step.popBool, Step.pop, hst, bind, Except.bind, pure, Except.pure]
      exact ft rfl (fun _ => rfl) rfl rfl rfl
        (by simp [Step.advance, Step.push, h1, Value.conforms, Value.type, Value.wellFormed])
    · simp at hafter
  case eq | neq =>
    dsimp only [typeAfter] at hafter
    split at hafter
    · next t' t brest =>
      split at hafter
      · next heq =>
        simp only [Bool.and_eq_true] at heq
        obtain ⟨⟨hcmp, hdrop⟩, hdrop'⟩ := heq
        simp only [Option.some.injEq] at hafter
        subst hafter
        obtain ⟨b, st1, hst, hb, h1⟩ := conformsAll_cons_right hstack
        obtain ⟨a, st2, rfl, ha, h2⟩ := conformsAll_cons_right h1
        have htv := heqv (by first | exact Or.inl rfl | exact Or.inr rfl) _ hbefore t (by simp)
        exact equality_safe hst (comparable_of_assignable hcmp (conforms_assignable ha)
            (conforms_assignable hb))
          (AbilitySet.drop_of_subset (abilities_of_conforms hc.abilities ha) hdrop)
          (AbilitySet.drop_of_subset (abilities_of_conforms hc.abilities hb) hdrop')
          (not_ref_of_value ha htv)
          (ft rfl (fun _ => rfl) rfl rfl rfl
            (by simp [Step.advance, Step.push, h2, Value.conforms, Value.type, Value.wellFormed]))
      · simp at hafter
    · simp at hafter
  case negate =>
    dsimp only [typeAfter] at hafter
    split at hafter
    · next t brest =>
      split at hafter
      · next hs =>
        simp only [Option.some.injEq] at hafter
        subst hafter
        obtain ⟨x, st1, hst, hx, h1⟩ := conformsAll_int hstack
        dsimp only [Step.exec]
        simp only [Step.popInt, Step.pop, hst, bind, Except.bind, pure, Except.pure, hs,
          Bool.not_true, Bool.false_eq_true, ↓reduceIte]
        cases h : t.negate x with
        | none => trivial
        | some r =>
          exact ft rfl (fun _ => rfl) rfl rfl rfl
            (by simp [Step.advance, Step.push, h1, inRange_checked h])
      · simp at hafter
    · simp at hafter
  case abort =>
    dsimp only [typeAfter] at hafter
    split at hafter
    · obtain ⟨c, st1, hst, -, -⟩ := conformsAll_int hstack
      dsimp only [Step.exec]
      simp only [Step.pop, hst, bind, Except.bind, pure, Except.pure]
      trivial
    · simp at hafter
  case nop =>
    dsimp only [typeAfter] at hafter
    simp only [Option.some.injEq] at hafter
    subst hafter
    exact ft rfl (fun _ => rfl) rfl rfl rfl hstack
  case pop =>
    dsimp only [typeAfter] at hafter
    split at hafter
    · next t brest =>
      split at hafter
      · next hdrop =>
        simp only [Option.some.injEq] at hafter
        subst hafter
        obtain ⟨v, st1, hst, hv, h1⟩ := conformsAll_cons_right hstack
        have hvd := AbilitySet.drop_of_subset (abilities_of_conforms hc.abilities hv) hdrop
        dsimp only [Step.exec]
        simp only [Step.pop, hst, bind, Except.bind, pure, Except.pure, hvd,
          Bool.not_true, Bool.false_eq_true, ↓reduceIte]
        exact ft rfl (fun _ => rfl) rfl rfl rfl (by simpa [Step.advance] using h1)
      · simp at hafter
    · simp at hafter
  case branch o =>
    dsimp only [typeAfter] at hafter
    simp only [Option.some.injEq] at hafter
    subst hafter
    have ho : o ∈ successors code.code fr.pc := by simp [successors, hinstr, Bytecode.control]
    have hlt := hc.successors fr.function info code hact.fn hact.body fr.pc hact.reachable o ho
    dsimp only [Step.exec]
    simp only [Step.jump, hlt, ↓reduceIte, bind, Except.bind, pure, Except.pure]
    exact sk (fr' := { fr with pc := o }) (fun _ => rfl) rfl ho rfl hstack
  case brTrue o | brFalse o =>
    dsimp only [typeAfter] at hafter
    split at hafter
    · next brest =>
      simp only [Option.some.injEq] at hafter
      subst hafter
      obtain ⟨b, st1, hst, h1⟩ := conformsAll_bool hstack
      have ho : o ∈ successors code.code fr.pc := by simp [successors, hinstr, Bytecode.control]
      have hn : fr.pc + 1 ∈ successors code.code fr.pc := by
        simp [successors, hinstr, Bytecode.control]
      have hlt := hc.successors fr.function info code hact.fn hact.body fr.pc hact.reachable o ho
      dsimp only [Step.exec]
      simp only [Step.popBool, Step.pop, hst, bind, Except.bind, pure, Except.pure]
      cases b <;>
        simp only [Bool.false_eq_true, ↓reduceIte, Step.jump, hlt, Step.advance] <;>
        first
          | exact sk (fun _ => rfl) rfl ho rfl h1
          | exact sk (fun _ => rfl) rfl hn rfl h1
    · simp at hafter
  case ldConst i =>
    dsimp only [typeAfter] at hafter
    simp only [bind, Option.bind_eq_some_iff, Option.some.injEq] at hafter
    obtain ⟨c, hpool, t, htok, rfl⟩ := hafter
    obtain ⟨v, hdec, hv⟩ := hc.constants fr.function info code hact.fn hact.body fr.pc i
      hact.reachable hinstr c t hpool htok
    dsimp only [Step.exec]
    simp only [Step.need, hpool, htok, hdec, bind, Except.bind, pure, Except.pure]
    exact ft rfl (fun _ => rfl) rfl rfl rfl (by simp [Step.advance, Step.push, hv, hstack])
  case copyLoc i =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff] at hta
    obtain ⟨t, ⟨tys, htys, hti⟩, hta⟩ := hta
    split at hta
    · next hcopy =>
      simp only [Option.some.injEq] at hta
      subst hta
      obtain ⟨v, hv, hvt⟩ := read_local hc hact hinstr rfl htys hti
      have hvc := AbilitySet.copy_of_subset (abilities_of_conforms hc.abilities hvt) hcopy
      dsimp only [Step.exec]
      simp only [Step.need, hv, bind, Except.bind, pure, Except.pure, hvc, ↓reduceIte]
      exact ft rfl (fun _ => rfl) rfl rfl rfl (by simp [Step.advance, Step.push, hvt, hstack])
    · simp at hta
  case moveLoc i =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff, Option.some.injEq] at hta
    obtain ⟨t, ⟨tys, htys, hti⟩, rfl⟩ := hta
    obtain ⟨v, hv, hvt⟩ := read_local hc hact hinstr rfl htys hti
    obtain ⟨tys', htys', hsize, hconf⟩ := hact.locals
    rw [htys, Option.some.injEq] at htys'
    subst htys'
    dsimp only [Step.exec]
    simp only [Step.need, hv, bind, Except.bind, pure, Except.pure]
    refine succ_safe hc hact hchain hinstr hbefore hafter rfl
      (by simp [Step.advance, Step.push, successors, hinstr, Bytecode.control])
      (by simp [Step.advance, Step.push, hvt, hstack]) ?_ ?_ ?_
    · refine ⟨tys, htys, by simp [Step.advance, Step.push, hsize], fun j w hw => ?_⟩
      simp only [Step.advance, Step.push, Array.getElem?_setIfInBounds] at hw
      split at hw
      · split at hw <;> simp at hw
      · exact hconf j w hw
    · intro j hj
      simp only [availabilityAfter] at hj
      split at hj
      · simp at hj
      · next hne =>
        obtain ⟨w, hw⟩ := hact.available j hj
        exact ⟨w, by
          simp [Step.advance, Step.push, Array.getElem?_setIfInBounds_ne (Ne.symm hne), hw]⟩
    · intro j hj
      simp only [availabilityAfter] at hj
      split at hj
      · next hji =>
        subst hji
        have hi : j < fr.locals.size := by
          rcases Nat.lt_or_ge j fr.locals.size with h | h
          · exact h
          · simp [Array.getElem?_eq_none h] at hv
        simp [Step.advance, Step.push, hi]
      · next hne =>
        simp [Step.advance, Step.push, Array.getElem?_setIfInBounds_ne (Ne.symm hne),
          hact.unavailable j hj]
  case stLoc i =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff] at hta
    obtain ⟨t, ⟨tys, htys, hti⟩, hta⟩ := hta
    split at hta
    · next t' brest =>
      split at hta
      · next heq =>
        simp only [Option.some.injEq] at hta
        subst hta
        obtain ⟨v, st1, hst, hvt0, h1⟩ := conformsAll_cons_right hstack
        have hvt := conforms_trans hvt0 heq
        obtain ⟨tys', htys', hsize, hconf⟩ := hact.locals
        rw [htys, Option.some.injEq] at htys'
        subst htys'
        have hi : i < fr.locals.size := by rw [hsize]; exact (List.getElem?_eq_some_iff.1 hti).1
        obtain ⟨slot, hslot⟩ : ∃ slot, fr.locals[i]? = some slot := ⟨_, Array.getElem?_eq_getElem hi⟩
        have hnext : ExecSafe P N (Inv P tc ac) (Step.next s rest (Step.advance
            { fr with stack := st1, locals := fr.locals.setIfInBounds i (some v) })) := by
          refine succ_safe hc hact hchain hinstr hbefore hafter rfl
            (by simp [Step.advance, successors, hinstr, Bytecode.control])
            (by simpa [Step.advance] using h1) ?_ ?_ ?_
          · refine ⟨tys, htys, by simp [Step.advance, hsize], fun j w hw => ?_⟩
            simp only [Step.advance, Array.getElem?_setIfInBounds] at hw
            split at hw
            · next hij =>
              subst hij
              simp only [Option.some.injEq] at hw
              subst hw
              exact ⟨t, hti, hvt⟩
            · exact hconf j w hw
          · intro j _
            by_cases hij : i = j
            · subst hij
              exact ⟨v, by simp [Step.advance, hi]⟩
            · have : availabilityAfter (.stLoc i) (ac fr.function fr.pc) j =
                  ac fr.function fr.pc j := by simp [availabilityAfter, Ne.symm hij]
              obtain ⟨w, hw⟩ := hact.available j (by rw [← this]; assumption)
              exact ⟨w, by simp [Step.advance, Array.getElem?_setIfInBounds_ne hij, hw]⟩
          · intro j hj
            by_cases hij : i = j
            · subst hij
              simp [availabilityAfter] at hj
            · have : availabilityAfter (.stLoc i) (ac fr.function fr.pc) j =
                  ac fr.function fr.pc j := by simp [availabilityAfter, Ne.symm hij]
              rw [this] at hj
              simp [Step.advance, Array.getElem?_setIfInBounds_ne hij, hact.unavailable j hj]
        dsimp only [Step.exec]
        simp only [Step.pop, hst, Step.need, htys, hti, hslot, hvt, bind, Except.bind, pure,
          Except.pure, Bool.not_true, Bool.false_eq_true, ↓reduceIte]
        cases slot with
        | none => simpa using next_state hnext
        | some old =>
          -- The overwritten value has `drop`: its local may hold a value, so its
          -- type has `drop`, and the value has its type's abilities.
          have hav : ac fr.function fr.pc i ≠ .unavailable := by
            intro hu; rw [hact.unavailable i hu] at hslot; cases hslot
          obtain ⟨t0, ht0, hold⟩ := hconf i old hslot
          rw [hti, Option.some.injEq] at ht0
          subst ht0
          have hdrop := hc.drops fr.function info code hact.fn hact.body hact.typeArgs fr.pc i
            hact.reachable (Or.inr hinstr) hav tys t htys hti
          have hod := AbilitySet.drop_of_subset (abilities_of_conforms hc.abilities hold) hdrop
          simpa [hod] using next_state hnext
      · simp at hta
    · simp at hta
  case ret =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff] at hta
    obtain ⟨rets, hrets, hta⟩ := hta
    split at hta
    · next heq =>
      have hstack := conformsAll_assignable hstack heq
      have hlen := conformsAll_length hstack
      simp only [List.length_reverse] at hlen
      have hvals := conformsAll_reverse hstack
      rw [List.reverse_reverse] at hvals
      dsimp only [Step.exec, Step.ret]
      simp only [Step.need, hrets, hlen, Nat.lt_irrefl, gt_iff_lt, ↓reduceIte, hvals,
        Bool.not_true, Bool.false_eq_true, bind, Except.bind, pure, Except.pure]
      split
      · next hany =>
        -- Every value still in a local has `drop`: its local may hold a value,
        -- so its type has `drop`, and the value has its type's abilities.
        exfalso
        rw [Array.any_eq_true] at hany
        obtain ⟨i, hi_lt, hi⟩ := hany
        split at hi
        · next v hv =>
          have hslot : fr.locals[i]? = some (some v) := by
            rw [Array.getElem?_eq_getElem hi_lt, hv]
          obtain ⟨tys, htys, -, hconf⟩ := hact.locals
          obtain ⟨t, ht, hvt⟩ := hconf i v hslot
          have hav : ac fr.function fr.pc i ≠ .unavailable := by
            intro hu; rw [hact.unavailable i hu] at hslot; cases hslot
          have hdrop := hc.drops fr.function info code hact.fn hact.body hact.typeArgs fr.pc i
            hact.reachable (Or.inl hinstr) hav tys t htys ht
          have hvd := AbilitySet.drop_of_subset (abilities_of_conforms hc.abilities hvt) hdrop
          simp [hvd] at hi
        · simp at hi
      cases rest with
      | nil => trivial
      | cons caller rest' =>
        obtain ⟨⟨rets', hrets', hres⟩, hchain'⟩ := hchain
        rw [hrets, Option.some.injEq] at hrets'
        subst hrets'
        have hact' := hres _ hvals
        rw [List.reverse_reverse] at hact'
        exact ⟨_, rest', rfl, hact', hchain'⟩
    · simp at hta
  case call h => exact call_safe hc hN hact hchain hinstr hbefore hafter hstack hnm
  case pack d =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff] at hta
    obtain ⟨sid, hdef, tys, hftys, hpp⟩ := hta
    obtain ⟨tail, rfl, hstack⟩ := popPush_conforms hpp hstack
    obtain ⟨sinfo, fields, hst, hfi⟩ := fieldTypes_some hftys
    obtain ⟨hpop, hvals, hrest⟩ := popN_conforms hstack
    dsimp only [Step.exec, Step.packStruct]
    simp only [localStruct_ok hdef hst, hfi, Step.need, hftys, hpop, hvals, bind, Except.bind,
      pure, Except.pure, Bool.not_true, Bool.false_eq_true, ↓reduceIte]
    exact ft rfl (fun _ => rfl) rfl rfl rfl
      (by simpa [Step.advance, Step.push, hrest, packedValue] using
        packedValue_conforms hftys hvals)
  case unpack d =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff] at hta
    obtain ⟨sid, hdef, tys, hftys, hpp⟩ := hta
    obtain ⟨tail, rfl, rfl⟩ := popPushAll_some hpp
    obtain ⟨sinfo, fields, hst, hfi⟩ := fieldTypes_some hftys
    obtain ⟨v, st1, hstk, hv, h1⟩ := conformsAll_cons_right hstack
    obtain ⟨fs, hsl, hfs⟩ := conforms_struct hv hftys
    rcases hsl with rfl | ⟨ks, es, rfl⟩ <;>
    · dsimp only [Step.exec, Step.unpackStruct]
      simp only [localStruct_ok hdef hst, hfi, Step.pop, hstk, bind, Except.bind, pure,
        Except.pure, bne_self_eq_false, Bool.false_eq_true, ↓reduceIte]
      exact ft rfl (fun _ => rfl) rfl rfl rfl
        (by simpa [Step.advance, Step.pushAll] using
          conformsAll_append (conformsAll_reverse hfs) h1)
  case packVariant h =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff] at hta
    obtain ⟨vh, hvh, sid, hdef, tys, hftys, hpp⟩ := hta
    obtain ⟨tail, rfl, hstack⟩ := popPush_conforms hpp hstack
    obtain ⟨sinfo, variants, vd, hst, hfi, hvd⟩ := variantFieldTypes_some hftys
    obtain ⟨hpop, hvals, hrest⟩ := popN_conforms hstack
    dsimp only [Step.exec, Step.packEnum]
    simp only [Step.need, hvh, localStruct_ok hdef hst, hfi, hvd, hftys, hpop, hvals, bind,
      Except.bind, pure, Except.pure, Bool.not_true, Bool.false_eq_true, ↓reduceIte]
    exact ft rfl (fun _ => rfl) rfl rfl rfl
      (by simp [Step.advance, Step.push, hrest, Value.conforms, Value.type, wellFormed_variant,
        hftys, hvals])
  case unpackVariant h =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff] at hta
    obtain ⟨vh, hvh, sid, hdef, tys, hftys, hpp⟩ := hta
    obtain ⟨tail, rfl, rfl⟩ := popPushAll_some hpp
    obtain ⟨sinfo, variants, vd, hst, hfi, hvd⟩ := variantFieldTypes_some hftys
    obtain ⟨v, st1, hstk, hv, h1⟩ := conformsAll_cons_right hstack
    obtain ⟨tag, fs, tys', rfl, htys', hfs⟩ := conforms_variant hv hftys
    dsimp only [Step.exec, Step.unpackEnum]
    simp only [Step.need, hvh, localStruct_ok hdef hst, hfi, hvd, Step.pop, hstk, bind,
      Except.bind, pure, Except.pure, bne_self_eq_false, Bool.false_eq_true, ↓reduceIte]
    by_cases htag : tag = vh.variant
    · subst htag
      rw [hftys, Option.some.injEq] at htys'
      subst htys'
      simp only [bne_self_eq_false, Bool.false_eq_true, ↓reduceIte]
      exact ft rfl (fun _ => rfl) rfl rfl rfl
        (by simpa [Step.advance, Step.pushAll] using
          conformsAll_append (conformsAll_reverse hfs) h1)
    · have : (tag != vh.variant) = true := by simpa using htag
      simp only [this, ↓reduceIte]
      trivial
  case vecPack sig n =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff] at hta
    obtain ⟨t, het, hpp⟩ := hta
    obtain ⟨tail, rfl, hstack⟩ := popPush_conforms hpp hstack
    obtain ⟨hpop, hvals, hrest⟩ := popN_conforms hstack
    simp only [List.length_replicate] at hpop hvals hrest
    have hlen : (fr.stack.take n).reverse.length = n := by
      have := conformsAll_length hvals; simpa using this
    have hall : ∀ v ∈ (fr.stack.take n).reverse, v.conforms P t = true := by
      have h2 : conformsAll P (fr.stack.take n).reverse
          (List.replicate (fr.stack.take n).reverse.length t) = true := by rw [hlen]; exact hvals
      exact conformsAll_replicate.1 h2
    dsimp only [Step.exec]
    simp only [elemType_ok het, hpop, List.all_eq_true.2 hall, bind, Except.bind,
      Bool.not_true, Bool.false_eq_true, ↓reduceIte]
    exact ft rfl (fun _ => rfl) rfl rfl rfl
      (by simp only [Step.advance, Step.push, conformsAll_cons]
          exact ⟨conforms_of (assignable_refl _) (wellFormed_vector.2 hall), hrest⟩)
  case vecUnpack sig n =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff] at hta
    obtain ⟨t, het, hpp⟩ := hta
    obtain ⟨tail, rfl, rfl⟩ := popPushAll_some hpp
    obtain ⟨v, st1, hstk, hv, h1⟩ := conformsAll_cons_right hstack
    obtain ⟨elems, rfl, helems⟩ := conforms_vector hv
    dsimp only [Step.exec]
    simp only [elemType_ok het, Step.pop, hstk, bind, Except.bind, pure, Except.pure,
      bne_self_eq_false, Bool.false_eq_true, ↓reduceIte]
    by_cases hn : elems.length = n
    · subst hn
      simp only [bne_self_eq_false, Bool.false_eq_true, ↓reduceIte]
      exact ft rfl (fun _ => rfl) rfl rfl rfl
        (by simpa [Step.advance, Step.pushAll] using
          conformsAll_append (conformsAll_reverse (conformsAll_replicate.2 helems)) h1)
    · have : (elems.length != n) = true := by simpa using hn
      simp only [this, ↓reduceIte]
      trivial
  case abortMsg =>
    dsimp only [typeAfter] at hafter
    split at hafter
    · obtain ⟨msg, st1, hst, hmsg, h1⟩ := conformsAll_cons_right hstack
      obtain ⟨elems, rfl, helems⟩ := conforms_vector hmsg
      obtain ⟨c, st2, rfl, -, -⟩ := conformsAll_int h1
      obtain ⟨bytes, hb⟩ := messageBytes_some elems helems
      dsimp only [Step.exec]
      simp only [Step.pop, hst, bind, Except.bind, pure, Except.pure, Step.need, hb]
      split <;> trivial
    · simp at hafter
  -- Outside the supported subset the certificate has no result type; references
  -- and global storage are outside programs by value.
  all_goals first
    | (dsimp only [typeAfter] at hafter; simp at hafter; done)
    | (simp [Bytecode.inCore, Bytecode.inDataByValue] at hbv)

/-- The generic forms by value: `CallGeneric`, and the generic forms that
build and take apart struct and enum values, each as its non-generic form at
the instance its instantiation names. -/
theorem execGeneric_safe (hc : Certified P tc ac) {N : NativeRel} (hN : NativesRespect P N)
    {s : State} {rest : List Frame} {fr : Frame} {info : FunctionInfo} {code : CodeUnit}
    {instr : Bytecode}
    (hact : ActiveAt P tc ac fr info code) (hchain : ChainOk P tc ac fr.function rest)
    (hinstr : code.code[fr.pc]? = some instr) (hg : instr.inGenericByValue = true)
    (hnm : NoModeledCall P fr.function.module info.module instr) :
    ExecSafe P N (Inv P tc ac) (Step.exec P s fr rest info code instr) := by
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc instr hact.reachable hinstr
  obtain ⟨tys, htys, hstack⟩ := hact.types
  rw [hbefore, Option.some.injEq] at htys
  subst htys
  have ft := fun {fr' : Frame} (hctrl : instr.control = .fallThrough)
    (hav : ∀ a, availabilityAfter instr a = a) (hfun : fr'.function = fr.function)
    (hpc : fr'.pc = fr.pc + 1) (hloc : fr'.locals = fr.locals)
    (hstk : conformsAll P fr'.stack after = true) =>
    fallThrough_safe (N := N) (s := s) hc hact hchain hinstr hctrl hav hbefore hafter hfun hpc
      hloc hstk
  cases instr
  case callGeneric i => exact callGeneric_safe hc hN hact hchain hinstr hbefore hafter hstack hnm
  case packGeneric i =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff] at hta
    obtain ⟨x, hx, sid, ⟨s0, hdef, targs, ⟨a, htargs, hmatch⟩, hif⟩, tys, hftys, hpp⟩ := hta
    obtain ⟨rfl, hne⟩ := typeArgsMatch_some hmatch
    split at hif
    · next hwf =>
      simp only [Option.some.injEq] at hif
      subst hif
      obtain ⟨tail, rfl, hstack⟩ := popPush_conforms hpp hstack
      obtain ⟨sinfo, fields, hst, hfi⟩ := fieldTypes_some hftys
      obtain ⟨hpop, hvals, hrest⟩ := popN_conforms hstack
      dsimp only [Step.exec, Step.packStruct]
      simp only [Step.need, hx, pure, Except.pure, genericArgs_of_typeArgsWith htargs hne, bind,
        Except.bind, localStruct_ok_gen hdef hst hwf, hfi, hftys, hpop, hvals, Bool.not_true,
        Bool.false_eq_true, ↓reduceIte]
      exact ft rfl (fun _ => rfl) rfl rfl rfl
        (by simpa [Step.advance, Step.push, hrest, packedValue] using
          packedValue_conforms hftys hvals)
    · simp at hif
  case unpackGeneric i =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff] at hta
    obtain ⟨x, hx, sid, ⟨s0, hdef, targs, ⟨a, htargs, hmatch⟩, hif⟩, tys, hftys, hpp⟩ := hta
    obtain ⟨rfl, hne⟩ := typeArgsMatch_some hmatch
    split at hif
    · next hwf =>
      simp only [Option.some.injEq] at hif
      subst hif
      obtain ⟨tail, rfl, rfl⟩ := popPushAll_some hpp
      obtain ⟨sinfo, fields, hst, hfi⟩ := fieldTypes_some hftys
      obtain ⟨v, st1, hstk, hv, h1⟩ := conformsAll_cons_right hstack
      obtain ⟨fs, hsl, hfs⟩ := conforms_struct hv hftys
      rcases hsl with rfl | ⟨ks, es, rfl⟩ <;>
      · dsimp only [Step.exec, Step.unpackStruct]
        simp only [Step.need, hx, pure, Except.pure, genericArgs_of_typeArgsWith htargs hne, bind,
          Except.bind, localStruct_ok_gen hdef hst hwf, hfi, Step.pop, hstk, bne_self_eq_false,
          Bool.false_eq_true, ↓reduceIte]
        exact ft rfl (fun _ => rfl) rfl rfl rfl
          (by simpa [Step.advance, Step.pushAll] using
            conformsAll_append (conformsAll_reverse hfs) h1)
    · simp at hif
  case packVariantGeneric i =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff] at hta
    obtain ⟨x, hx, vh, hvh, sid, ⟨s0, hdef, targs, ⟨a, htargs, hmatch⟩, hif⟩, tys, hftys, hpp⟩ :=
      hta
    obtain ⟨rfl, hne⟩ := typeArgsMatch_some hmatch
    split at hif
    · next hwf =>
      simp only [Option.some.injEq] at hif
      subst hif
      obtain ⟨tail, rfl, hstack⟩ := popPush_conforms hpp hstack
      obtain ⟨sinfo, variants, vd, hst, hfi, hvd⟩ := variantFieldTypes_some hftys
      obtain ⟨hpop, hvals, hrest⟩ := popN_conforms hstack
      dsimp only [Step.exec, Step.packEnum]
      simp only [Step.need, hx, hvh, pure, Except.pure, genericArgs_of_typeArgsWith htargs hne, bind,
        Except.bind, localStruct_ok_gen hdef hst hwf, hfi, hvd, hftys, hpop, hvals,
        Bool.not_true, Bool.false_eq_true, ↓reduceIte]
      exact ft rfl (fun _ => rfl) rfl rfl rfl
        (by simp [Step.advance, Step.push, hrest, Value.conforms, Value.type, wellFormed_variant,
          hftys, hvals])
    · simp at hif
  case unpackVariantGeneric i =>
    have hta := hafter
    dsimp only [typeAfter] at hta
    simp only [bind, Option.bind_eq_some_iff] at hta
    obtain ⟨x, hx, vh, hvh, sid, ⟨s0, hdef, targs, ⟨a, htargs, hmatch⟩, hif⟩, tys, hftys, hpp⟩ :=
      hta
    obtain ⟨rfl, hne⟩ := typeArgsMatch_some hmatch
    split at hif
    · next hwf =>
      simp only [Option.some.injEq] at hif
      subst hif
      obtain ⟨tail, rfl, rfl⟩ := popPushAll_some hpp
      obtain ⟨sinfo, variants, vd, hst, hfi, hvd⟩ := variantFieldTypes_some hftys
      obtain ⟨v, st1, hstk, hv, h1⟩ := conformsAll_cons_right hstack
      obtain ⟨tag, fs, tys', rfl, htys', hfs⟩ := conforms_variant hv hftys
      dsimp only [Step.exec, Step.unpackEnum]
      simp only [Step.need, hx, hvh, pure, Except.pure, genericArgs_of_typeArgsWith htargs hne, bind,
        Except.bind, localStruct_ok_gen hdef hst hwf, hfi, hvd, Step.pop, hstk,
        bne_self_eq_false, Bool.false_eq_true, ↓reduceIte]
      by_cases htag : tag = vh.variant
      · subst htag
        rw [hftys, Option.some.injEq] at htys'
        subst htys'
        simp only [bne_self_eq_false, Bool.false_eq_true, ↓reduceIte]
        exact ft rfl (fun _ => rfl) rfl rfl rfl
          (by simpa [Step.advance, Step.pushAll] using
            conformsAll_append (conformsAll_reverse hfs) h1)
      · have : (tag != vh.variant) = true := by simpa using htag
        simp only [this, ↓reduceIte]
        trivial
    · simp at hif
  all_goals simp [Bytecode.inGenericByValue] at hg

/-! ## The invariant -/

/-- A program by value has no modeled natives: those have a reference
parameter (`modeledNative?_hasRef`). -/
theorem nativeShape?_byValue (hP : P.ByValue) (f : FunctionRef) : P.nativeShape? f = none := by
  cases hf : P.function? f with
  | none => simp [Program.nativeShape?, hf]
  | some info =>
    cases hk : info.module.modeledNative? info.definition with
    | none => simp [Program.nativeShape?, hf, hk]
    | some k =>
      exfalso
      obtain ⟨fh, params, -, hparams, t, ht, htr⟩ := modeledNative?_hasRef hk
      have hmod := (function?_lookup hf).1
      have := (hP _ (Array.mem_toList_iff.2 (Array.mem_of_getElem? hmod))).1 params
        (Array.mem_toList_iff.2 (Array.mem_of_getElem? hparams)) t ht
      rw [htr] at this
      cases this

theorem noModeledCall_byValue (hP : P.ByValue) (mi : Nat) (m : CompiledModule) (instr : Bytecode) :
    NoModeledCall P mi m instr := by
  cases instr <;> first
    | trivial
    | exact fun _ _ => nativeShape?_byValue hP _
    | exact fun _ _ _ _ => nativeShape?_byValue hP _

theorem noModeledCall_nonGeneric (hs : P.SupportedNonGeneric) (mi : Nat) (m : CompiledModule)
    (instr : Bytecode) : NoModeledCall P mi m instr := by
  cases instr <;> first
    | trivial
    | exact fun _ _ => nativeShape?_nonGeneric hs _
    | exact fun _ _ _ _ => nativeShape?_nonGeneric hs _

theorem step_safe (hc : Certified P tc ac) (hP : P.ByValue) {N : NativeRel}
    (hN : NativesRespect P N) {s : State}
    (hs : Inv P tc ac s) : StepSafe P N (Inv P tc ac) (step P s) := by
  obtain ⟨fr, rest, hfr, ⟨info, code, hact⟩, hchain⟩ := hs
  have hlt := hact.pcLt
  have hpc : code.code[fr.pc]? = some code.code[fr.pc] := Array.getElem?_eq_getElem hlt
  have hbv := byValue_instr hP hact.fn hact.body hpc
  have hsafe : ExecSafe P N (Inv P tc ac) (Step.exec P s fr rest info code code.code[fr.pc]) := by
    cases hcd : (code.code[fr.pc].inCore || code.code[fr.pc].inDataByValue)
    · rw [hcd, Bool.false_or] at hbv
      exact execGeneric_safe hc hN hact hchain hpc hbv (noModeledCall_byValue hP _ _ _)
    · exact exec_safe hc hN hact hchain hpc hcd
        (fun _ => certificate_values hP hc.types hact.fn hact.body hact.typeArgs hact.reachable)
        (noModeledCall_byValue hP _ _ _)
  unfold step
  rw [hfr]
  simp only [hact.fn, hact.body, hpc]
  cases hx : Step.exec P s fr rest info code code.code[fr.pc] <;> rw [hx] at hsafe <;>
    exact hsafe

theorem stepRel_safe (hc : Certified P tc ac) (hP : P.ByValue) {N : NativeRel}
    (hN : NativesRespect P N)
    {s : State} {o : Outcome} (hs : Inv P tc ac s) (hstep : StepRel P N s o) :
    Safe (Inv P tc ac) o := by
  have hsafe := step_safe hc hP hN hs
  cases hstep with
  | core h => rw [h] at hsafe; exact hsafe
  | native h hr => rw [h] at hsafe; exact hsafe _ hr

theorem initial_inv (hc : Certified P tc ac) {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    Inv P tc ac s₀ := by
  obtain ⟨f, args, ps, storage, held, hpar, hargs, -, hinit⟩ := h0
  simp only [initialState, bind, Option.bind_eq_some_iff, pure] at hinit
  obtain ⟨info, hf, code, hcode, ltys, hltys, hinit⟩ := hinit
  split at hinit
  · simp at hinit
  next htargs =>
  split at hinit
  · simp at hinit
  simp only [Option.some.injEq] at hinit
  subst hinit
  obtain ⟨ps', ls, hps', rfl⟩ := localTypes_split hltys
  rw [hpar, Option.some.injEq] at hps'
  subst hps'
  exact ⟨_, [], rfl, ⟨info, code, entry_activeAt hc hf hcode (by simpa using htargs) hpar hltys
    hargs 0 rfl [] rfl _ _ _⟩, trivial⟩

/-- Every reachable state satisfies the invariant. -/
theorem inv_reachable (hc : Certified P tc ac) (hP : P.ByValue) {N : NativeRel}
    (hN : NativesRespect P N)
    {s₀ : State} (h0 : AdmissibleInitial P s₀) : ∀ s, Reachable P N s₀ s → Inv P tc ac s := by
  intro s hr
  induction hr with
  | refl => exact initial_inv hc h0
  | step _ hst ih => exact stepRel_safe hc hP hN ih hst

/-- **D1 from certificates.** A program with consistent certificates and the
side conditions of `Certified` never goes wrong. -/
theorem noWrong_of_certified (hc : Certified P tc ac) (hP : P.ByValue) {N : NativeRel}
    (hN : NativesRespect P N)
    {s₀ : State} (h0 : AdmissibleInitial P s₀) : NoWrong P N s₀ := by
  intro s hr r hw
  exact stepRel_safe hc hP hN (inv_reachable hc hP hN h0 s hr) hw

/-- **C1, C2 (agreement) from certificates.** Every execution agrees with
certificates that satisfy `Certified`. -/
theorem certificates_agree (hc : Certified P tc ac) (hP : P.ByValue) {N : NativeRel}
    (hN : NativesRespect P N) :
    StaticTypes P N tc ∧ StaticAvailability P N ac := by
  have hact : ∀ s₀, AdmissibleInitial P s₀ → ∀ s, Reachable P N s₀ s → ∀ fr rest,
      s.frames = fr :: rest → ∃ info code, ActiveAt P tc ac fr info code := by
    intro s₀ h0 s hr fr rest hfr
    obtain ⟨fr', rest', hfr', hok, -⟩ := inv_reachable hc hP hN h0 s hr
    rw [hfr, List.cons.injEq] at hfr'
    obtain ⟨rfl, -⟩ := hfr'
    exact hok
  refine ⟨fun s₀ h0 s hr fr rest hfr => ?_, fun s₀ h0 s hr fr rest hfr => ?_⟩
  · obtain ⟨info, code, ha⟩ := hact s₀ h0 s hr fr rest hfr
    obtain ⟨tys, htys, -, hconf⟩ := ha.locals
    exact ⟨ha.types, tys, htys, hconf⟩
  · obtain ⟨info, code, ha⟩ := hact s₀ h0 s hr fr rest hfr
    exact ⟨ha.available, ha.unavailable⟩

end MoveBytecode.Proofs
