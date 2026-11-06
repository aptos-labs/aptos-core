-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostExec

/-!
# Steps and the closures they create

UNTRUSTED (checked by the kernel). Facts about the steps, which D5
(`Proofs.FunctionValues`) needs. A `PackClosure` that continues pushes a
well-formed closure (`packClosure_cont`), and a `CallClosure` that continues
enters the closure's target with its captured and provided values
(`callClosure_cont`). Of the steps:

- the annotation of a step creates a value only for the replacing
  instructions (literals, constants, arithmetic, comparisons, and the tests
  that push a result), whose pushed value it creates whole, and for the
  packing ones, whose container it creates (`exec_created`);
- a step other than `PackClosure` pushes no new closure: the replacing
  instructions push a plain value, the other packing ones a struct, an enum
  value or a vector (`exec_ok`);
- it keeps every resource in global storage of a type with `key`: only
  `MoveTo` adds a resource, of a type whose `key` it checks (`exec_ok`);
- a step that returns returns values of the current segment and commits
  global storage unchanged (`exec_ok`).

`exec_ok` walks the semantics of each instruction with `exec_walk`, which
splits its binds, conditionals and matches and closes each result with the
lemmas of this file.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Step

/-- The instructions whose annotation creates a value whole: literals,
constants, arithmetic, comparisons, and the tests that push a result. -/
def Replacing : Bytecode → Bool
  | .ldU8 _ | .ldU16 _ | .ldU32 _ | .ldU64 _ | .ldU128 _ | .ldU256 _ => true
  | .ldI8 _ | .ldI16 _ | .ldI32 _ | .ldI64 _ | .ldI128 _ | .ldI256 _ => true
  | .ldTrue | .ldFalse | .ldConst _ => true
  | .castU8 | .castU16 | .castU32 | .castU64 | .castU128 | .castU256 => true
  | .castI8 | .castI16 | .castI32 | .castI64 | .castI128 | .castI256 => true
  | .not | .negate => true
  | .add | .sub | .mul | .mod | .div | .bitOr | .bitAnd | .xor | .shl | .shr => true
  | .or | .and | .lt | .gt | .le | .ge | .eq | .neq => true
  | .exists_ _ | .existsGeneric _ | .testVariant _ | .testVariantGeneric _ | .vecLen _ => true
  | _ => false

/-- The instructions whose annotation creates a container. -/
def Packing : Bytecode → Bool
  | .pack _ | .packGeneric _ | .packVariant _ | .packVariantGeneric _ | .vecPack _ _ => true
  | .packClosure _ _ | .packClosureGeneric _ _ => true
  | _ => false

theorem fresh_created {c c' : Creator} {v w : Value} {n i : Nat}
    (h : Event.created i w c' ∈ (IdTree.fresh c v n).2.2) : w ∈ subvalues v := by
  rw [(fresh_spec c v n).2.2.2] at h
  obtain ⟨⟨j, x⟩, hx, he⟩ := List.mem_map.1 h
  simp only [Event.created.injEq] at he
  obtain ⟨-, rfl, -⟩ := he
  exact List.of_mem_zip hx |>.2

set_option maxHeartbeats 10000000 in
theorem exec_created {P : Program} {m : ModuleId} {instr : Bytecode} {s : State} {fr : Frame}
    {o : Outcome} {h : Shadow} :
    ∀ p, Shadow.exec P m instr s fr o h = some p → ∀ i w c, Event.created i w c ∈ p.2 →
    ∃ v, Shadow.topAfter o = some v ∧
      ((Replacing instr = true ∧ w ∈ subvalues v) ∨ (Packing instr = true ∧ w = v)) := by
  rcases h with ⟨_ | ⟨g, rest⟩, hst, hhe, f⟩
  · simp [Shadow.exec]
  cases instr
  all_goals (
    simp [Shadow.exec, guard, Option.bind_eq_some_iff]
    repeat' (first
      | split
      | (simp only [and_imp, forall_eq', forall_apply_eq_imp_iff, forall_apply_eq_imp_iff₂,
          Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq, Prod.forall,
          forall_exists_index, Option.bind_some])
      | intro)
    all_goals (
      rename_i he
      subst_vars
      first
      | (simp_all; done)
      | (exact ⟨_, ‹_›, Or.inl ⟨rfl, fresh_created he⟩⟩)
      | (simp at he
         exact ⟨_, ‹_›, Or.inl ⟨rfl, fresh_created he⟩⟩)
      | (simp at he
         obtain ⟨-, rfl, -⟩ := he
         exact ⟨_, ‹_›, Or.inr ⟨rfl, rfl⟩⟩)))


/-! ## The closure instructions -/

section Closures

variable {P : Program}

/-- **A `PackClosure` that continues** runs, keeps global storage, and pushes
a well-formed closure. -/
theorem packClosure_cont {s : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
    {hd : FunctionHandleIndex} {mask : BitVec 64} {tys : List RuntimeType} {o : Outcome}
    (hcont : o.continues = true)
    (h : stepOf (Step.packClosure P s fr rest info hd mask tys) = .done o) :
    ∃ s', o = .running s' ∧ s'.storage = s.storage ∧
      ∃ v, Shadow.topAfter (.running s') = some v ∧ v.wellFormed P = true := by
  unfold Step.packClosure at h
  obtain ⟨target, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨tinfo, htf, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  next hwfa =>
  obtain ⟨paramTys, hpar, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨retTys, hret, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  next hnat =>
  split at h
  · exact (done_wrong h hcont).elim
  next hwithin =>
  split at h
  next capturedTys remaining hsplit =>
  split at h
  · exact (done_wrong h hcont).elim
  next hval =>
  obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
  obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  next hc =>
  refine ⟨_, done_next h, rfl, _, rfl, ?_⟩
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hc hval hwithin hwfa
  obtain ⟨tcode, htcode⟩ : ∃ c, tinfo.definition.code = some c := by
    cases hcd : tinfo.definition.code with
    | none => simp [hcd] at hnat
    | some c => exact ⟨c, rfl⟩
  have htf' : P.function? { target with typeArgs := tys } = some tinfo := by
    rw [function?_typeArgs]; exact need_ok' htf
  have hok : P.typeArgsOk { target with typeArgs := tys } = true := by
    simp only [Program.typeArgsOk, htf']; exact hwfa
  have hct : P.closureType? { target with typeArgs := tys } mask =
      some (capturedTys, remaining, retTys) := by
    simp only [Program.closureType?, htf', htcode, hok, need_ok' hpar, need_ok' hret, hwithin,
      hsplit, hval, bind, Option.bind_some, Bool.not_true, pure]
    rfl
  simp [Value.wellFormed, hct, hc]


/-- **A `CallClosure` that continues** enters the closure on top of the
stack: the target's first frame, on top, runs it from its first
instruction, with the captured and the provided values as its first locals,
conforming to its parameter types; global storage is unchanged. -/
theorem callClosure_cont {s : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
    {sig : SignatureIndex} {o : Outcome} (hcont : o.continues = true)
    (h : stepOf (Step.callClosure P s fr rest info sig) = .done o) :
    ∃ f mask captured cps crs ca st s' callee frs tys,
      fr.stack = .closure f mask captured cps crs ca :: st ∧ o = .running s' ∧
      s'.storage = s.storage ∧ s'.frames = callee :: frs ∧ P.paramTypes f = some tys ∧
      callee.function = f ∧ callee.pc = 0 ∧
      conformsAll P (maskCompose mask captured (st.take cps.length).reverse) tys = true ∧
      callee.locals.toList.take (maskCompose mask captured (st.take cps.length).reverse).length =
        (maskCompose mask captured (st.take cps.length).reverse).map some := by
  unfold Step.callClosure at h
  obtain ⟨sigTys, -, h⟩ := done_bind h hcont (ends_need _ _)
  dsimp only at h
  split at h
  · obtain ⟨ps, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨rs, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨_, hx, h⟩ := done_bind h hcont (ends_pure _)
    simp only [pure, Except.pure, Except.ok.injEq] at hx
    subst hx
    dsimp only at h
    obtain ⟨⟨c, fr₁⟩, hpop, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st, hst, rfl⟩ := pop_ok hpop
    dsimp only at h
    split at h
    · next target mask captured cps crs ca =>
      split at h
      · exact (done_wrong h hcont).elim
      next hchk =>
      have hcps : cps = ps := by
        by_cases e : cps = ps
        · exact e
        · exfalso; apply hchk; simp [e]
      subst hcps
      obtain ⟨⟨provided, fr₂⟩, hpop2, h⟩ := done_bind h hcont popN_ends
      obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop2
      dsimp only at h
      split at h
      · exact (done_wrong h hcont).elim
      obtain ⟨targetInfo, -, h⟩ := done_bind h hcont (ends_need _ _)
      split at h
      · exact (done_wrong h hcont).elim
      obtain ⟨paramTys, hps, h⟩ := done_bind h hcont (ends_need _ _)
      obtain ⟨retTys, -, h⟩ := done_bind h hcont (ends_need _ _)
      split at h
      · exact (done_wrong h hcont).elim
      next hconf =>
      simp only [Bool.or_eq_true, Bool.not_eq_true', not_or, Bool.not_eq_false] at hconf
      obtain ⟨localTys, hlt, h⟩ := done_bind h hcont (ends_need _ _)
      split at h <;> split at h
      all_goals first
        | exact (done_fail h hcont).elim
        | skip
      all_goals
        simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
        subst h
        exact ⟨target, mask, captured, cps, crs, ca, st, _, _, _, paramTys, hst, rfl, rfl, rfl,
          need_ok' hps, rfl, rfl, hconf.1, by simp⟩
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

/-- The closure-packing steps: they replace no value, and one that continues
runs and pushes a well-formed closure, keeping global storage. -/
theorem packClosure_exec {P : Program} {s : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {instr : Bytecode} {o : Outcome}
    (hpk : (∃ h m, instr = .packClosure h m) ∨ ∃ i m, instr = .packClosureGeneric i m)
    (hcont : o.continues = true) (h : stepOf (Step.exec P s fr rest info code instr) = .done o) :
    Replacing instr = false ∧ ∃ s', o = .running s' ∧ s'.storage = s.storage ∧
      ∃ v, Shadow.topAfter (.running s') = some v ∧ v.wellFormed P = true := by
  rcases hpk with ⟨hd, m, rfl⟩ | ⟨i, m, rfl⟩
  · exact ⟨rfl, packClosure_cont hcont h⟩
  · refine ⟨rfl, ?_⟩
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact packClosure_cont hcont h

end Closures

/-! ## What a step does to global storage and to the top of the segment -/

/-- Every resource in global storage is of a type with `key`. -/
@[irreducible] def TypeKeys (P : Program) (s : State) : Prop :=
  ∀ r ∈ s.storage, ((RuntimeType.struct r.type).abilities P).key = true

/-- The value a replacing instruction pushes holds no closure, and a packing
one pushes no closure. -/
@[irreducible] def TopFresh (instr : Bytecode) (o : Outcome) : Prop :=
  ∀ v, Shadow.topAfter o = some v →
    (Replacing instr = true → ∀ w ∈ subvalues v, IsClosure w = false) ∧
    (Packing instr = true → IsClosure v = false)

/-- What a step of `instr` from `s`, with current frame `fr`, establishes. -/
def StepOk (P : Program) (s : State) (fr : Frame) (instr : Bytecode) : Outcome → Prop
  | .running s' => TypeKeys P s' ∧ TopFresh instr (.running s')
  | .returned vs st => (∀ v ∈ vs, v ∈ fr.stack) ∧ st = s.storage
  | .failed _ => True
  | .wrong _ => True

/-- A partial step whose outcome, when the execution continues, satisfies
`StepOk`. -/
@[irreducible] def ExecOk (P : Program) (s : State) (fr : Frame) (instr : Bytecode)
    (x : Exec StepResult) : Prop :=
  ∀ o, stepOf x = .done o → o.continues = true → StepOk P s fr instr o

section ExecOk

variable {P : Program} {s : State} {fr : Frame} {instr : Bytecode}

theorem typeKeys_iff {s₀ : State} :
    TypeKeys P s₀ ↔ ∀ r ∈ s₀.storage, ((RuntimeType.struct r.type).abilities P).key = true := by
  unfold TypeKeys; exact Iff.rfl

theorem topFresh_of {o : Outcome}
    (h : ∀ v, Shadow.topAfter o = some v →
      (Replacing instr = true → ∀ w ∈ subvalues v, IsClosure w = false) ∧
      (Packing instr = true → IsClosure v = false)) : TopFresh instr o := by
  unfold TopFresh; exact h

theorem topFresh_elim {o : Outcome} (h : TopFresh instr o) {v : Value}
    (hv : Shadow.topAfter o = some v) :
    (Replacing instr = true → ∀ w ∈ subvalues v, IsClosure w = false) ∧
      (Packing instr = true → IsClosure v = false) := by
  unfold TopFresh at h; exact h v hv

theorem execOk_elim {x : Exec StepResult} (h : ExecOk P s fr instr x) {o : Outcome}
    (ho : stepOf x = .done o) (hc : o.continues = true) : StepOk P s fr instr o := by
  unfold ExecOk at h; exact h o ho hc

theorem execOk_bind {α : Type} {x : Exec α} {g : α → Exec StepResult} (hx : Ends x)
    (h : ∀ a, x = .ok a → ExecOk P s fr instr (g a)) : ExecOk P s fr instr (x >>= g) := by
  unfold ExecOk
  intro o ho hc
  obtain ⟨a, ha, hg⟩ := done_bind ho hc hx
  exact execOk_elim (h a ha) hg hc

theorem execOk_wrong {r : WrongReason} : ExecOk P s fr instr (wrong r) := by
  unfold ExecOk
  intro o ho hc
  exact (done_wrong ho hc).elim

theorem execOk_fail {f : Failure} : ExecOk P s fr instr (fail f) := by
  unfold ExecOk
  intro o ho hc
  exact (done_fail ho hc).elim

theorem execOk_callNative {s' : State} {f : FunctionRef} {args views : List Value} :
    ExecOk P s fr instr (pure (.callNative s' f args views)) := by
  unfold ExecOk
  intro o ho
  simp [stepOf, pure, Except.pure] at ho

theorem execOk_running {s' : State} (hk : TypeKeys P s') (ht : TopFresh instr (.running s')) :
    ExecOk P s fr instr (pure (.done (.running s'))) := by
  unfold ExecOk
  intro o ho _
  simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at ho
  subst ho
  exact ⟨hk, ht⟩

theorem execOk_returned {vs : List Value} {st : GlobalStorage} (hv : ∀ v ∈ vs, v ∈ fr.stack)
    (hst : st = s.storage) : ExecOk P s fr instr (pure (.done (.returned vs st))) := by
  unfold ExecOk
  intro o ho _
  simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at ho
  subst ho
  exact ⟨hv, hst⟩

theorem execOk_next {s₀ : State} {rest : List Frame} {fr' : Frame} (hk : TypeKeys P s₀)
    (ht : TopFresh instr (.running { s₀ with frames := fr' :: rest })) :
    ExecOk P s fr instr (next s₀ rest fr') :=
  execOk_running (s' := { s₀ with frames := fr' :: rest }) (by
    rw [typeKeys_iff] at hk ⊢; exact hk) ht

theorem execOk_finish {s₀ : State} {vals : List Value} (hk : TypeKeys P s₀)
    (ht : ∀ fr₀ rest₀, s₀.frames = fr₀ :: rest₀ →
      TopFresh instr (.running { s₀ with frames := advance (pushAll fr₀ vals) :: rest₀ })) :
    ExecOk P s fr instr (finish s₀ vals) := by
  unfold finish
  split
  · next fr₀ rest₀ hs =>
    exact execOk_running (by rw [typeKeys_iff] at hk ⊢; exact hk) (ht fr₀ rest₀ hs)
  · exact execOk_wrong

theorem topFresh_other {o : Outcome} (h1 : Replacing instr = false) (h2 : Packing instr = false) :
    TopFresh instr o :=
  topFresh_of fun _ _ => ⟨fun h => absurd h (by rw [h1]; decide),
    fun h => absurd h (by rw [h2]; decide)⟩

theorem topFresh_other' {s₀ : State} {vals : List Value} (h1 : Replacing instr = false)
    (h2 : Packing instr = false) : ∀ fr₀ rest₀, s₀.frames = fr₀ :: rest₀ →
      TopFresh instr (.running { s₀ with frames := advance (pushAll fr₀ vals) :: rest₀ }) :=
  fun _ _ _ => topFresh_other h1 h2

theorem keys_congr {s₀ s₁ : State} (hk : TypeKeys P s₀) (h : s₁.storage = s₀.storage) :
    TypeKeys P s₁ := by
  rw [typeKeys_iff] at hk ⊢; rw [h]; exact hk

theorem keys_update {s₀ s₁ : State} {l : Location} {v : Value} (h : update s₀ l v = .ok s₁)
    (hk : TypeKeys P s₀) : TypeKeys P s₁ := by
  rw [typeKeys_iff] at hk ⊢
  obtain ⟨s₂, hw, rfl⟩ := update_ok h
  unfold State.write? at hw
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def, Option.some.injEq] at hw
  obtain ⟨rv, -, v', -, rfl⟩ := hw
  intro r hr
  cases hroot : l.root with
  | local_ f i => rw [hroot] at hr; exact hk r hr
  | held k => rw [hroot] at hr; exact hk r hr
  | global a t =>
    rw [hroot] at hr
    simp only [State.invalidate, State.setRoot, GlobalStorage.update, List.mem_map] at hr
    obtain ⟨r₀, hr₀, rfl⟩ := hr
    split
    · exact hk r₀ hr₀
    · exact hk r₀ hr₀

theorem leaf_fresh {v : Value} (h : v.children = []) (hc : IsClosure v = false) :
    ∀ w ∈ subvalues v, IsClosure w = false := by
  intro w hw
  rw [subvalues_eq, h] at hw
  simp only [List.flatMap_nil, List.mem_singleton] at hw
  subst hw
  exact hc

theorem plain_fresh {v : Value} (h : ∀ w ∈ subvalues v, IsPlain w = true) :
    ∀ w ∈ subvalues v, IsClosure w = false := by
  intro w hw
  have := h w hw
  cases w <;> simp_all [IsPlain, IsClosure]

theorem fresh_const {t : RuntimeType} {d : List UInt8} {v : Value}
    (h : decodeConstant t d = some v) :
    (Replacing instr = true → ∀ w ∈ subvalues v, IsClosure w = false) ∧
      (Packing instr = true → IsClosure v = false) :=
  ⟨fun _ => plain_fresh (decodeConstant_plain h),
    fun _ => plain_fresh (decodeConstant_plain h) v (self_mem_subvalues v)⟩

theorem fresh_container {v : Value} (h1 : Replacing instr = false) (hv : IsClosure v = false) :
    (Replacing instr = true → ∀ w ∈ subvalues v, IsClosure w = false) ∧
      (Packing instr = true → IsClosure v = false) :=
  ⟨fun h => absurd h (by rw [h1]; decide), fun _ => hv⟩

theorem fresh_leaf {v : Value} (hc : v.children = []) (hv : IsClosure v = false) :
    (Replacing instr = true → ∀ w ∈ subvalues v, IsClosure w = false) ∧
      (Packing instr = true → IsClosure v = false) :=
  ⟨fun _ => leaf_fresh hc hv, fun _ => hv⟩

theorem plainOf {fr₀ fr' : Frame} {n : Nat} (h : ReplacesPlain P fr₀ fr' n) :
    ∃ v, Replaces fr₀ fr' n v ∧ ∀ w ∈ subvalues v, IsPlain w = true :=
  let ⟨v, hr, _, hp⟩ := h; ⟨v, hr, hp⟩

theorem topFresh_replaces {s' : State} {fr₀ fr' : Frame} {n : Nat}
    (h : ∃ v, Replaces fr₀ fr' n v ∧ ∀ w ∈ subvalues v, IsPlain w = true)
    (hf : s'.frames.head? = some fr') : TopFresh instr (.running s') := by
  obtain ⟨v, ⟨-, rfl⟩, hp⟩ := h
  refine topFresh_of fun w hw => ?_
  simp only [Shadow.topAfter, hf, advance, push, List.head?_cons, Option.bind_eq_bind,
    Option.bind_some, Option.some.injEq] at hw
  subst hw
  refine ⟨fun _ => plain_fresh hp, fun _ => ?_⟩
  have := hp v (self_mem_subvalues v)
  cases v <;> simp_all [IsPlain, IsClosure]

theorem equality_plain {positive : Bool} {fr₀ fr' : Frame}
    (h : equality P s fr₀ positive = .ok fr') :
    ∃ v, Replaces fr₀ fr' 2 v ∧ ∀ w ∈ subvalues v, IsPlain w = true := by
  obtain ⟨-, -, -, -, -, -, c, hR⟩ := equality_ok h
  exact ⟨.bool c, hR, by simp [subvalues_eq, Value.children, IsPlain]⟩

theorem globalType_key {mi : Nat} {d : StructDefinitionIndex} {tys : List RuntimeType}
    {sid : StructId} (h : globalType P mi d tys = .ok sid) :
    ((RuntimeType.struct sid).abilities P).key = true := by
  unfold globalType at h
  rw [bind_ok'] at h
  obtain ⟨⟨sid', _⟩, -, h⟩ := h
  dsimp only at h
  split at h
  · next hk =>
    simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact hk
  · simp [wrong] at h

theorem keys_insert {s₀ s₁ : State} {a : Address} {t : StructId} {v : Value} (hk : TypeKeys P s₀)
    (hkey : ((RuntimeType.struct t).abilities P).key = true)
    (h : s₁.storage = s₀.storage.insert a t v) : TypeKeys P s₁ := by
  rw [typeKeys_iff] at hk ⊢
  intro r hr
  rw [h] at hr
  simp only [GlobalStorage.insert, List.mem_append, List.mem_singleton] at hr
  rcases hr with hr | rfl
  · exact hk r hr
  · exact hkey

theorem keys_remove {s₀ s₁ : State} {a : Address} {t : StructId} (hk : TypeKeys P s₀)
    (h : s₁.storage = s₀.storage.remove a t) : TypeKeys P s₁ := by
  rw [typeKeys_iff] at hk ⊢
  intro r hr
  rw [h] at hr
  exact hk r (List.mem_filter.1 hr).1

end ExecOk

/-- Ends of the partial steps the semantics binds. -/
macro "ends_tac" : tactic => `(tactic| first
  | exact ends_need _ _ | exact pop_ends | exact popN_ends | exact popRef_ends
  | exact popRefTo_ends | exact deref_ends | exact update_ends | exact unlocked_ends
  | exact globalType_ends | exact localStruct_ends | exact genericArgs_ends | exact elemType_ends
  | exact popBool_ends | exact popInt_ends | exact popIndex_ends | exact popAddress_ends
  | exact derefVector_ends | exact intBinop_ends | exact intCompare_ends | exact boolBinop_ends
  | exact shift_ends | exact cast_ends | exact equality_ends | exact jump_ends
  | exact ends_pure _ | exact ends_wrong _ | exact ends_fail _
  | exact mapM_ends (fun _ => ends_bind (ends_need _ _) fun _ _ =>
      ends_bind (ends_need _ _) fun _ _ => ends_need _ _) _)

/-- Walks a partial step of the semantics to its results (`ExecOk`). -/
macro "exec_walk" : tactic => `(tactic| (
  unfold Step.exec
  try dsimp only
  try unfold packStruct
  try unfold unpackStruct
  try unfold packEnum
  try unfold unpackEnum
  try unfold testEnum
  try unfold existsResource
  try unfold moveResourceFrom
  try unfold moveResourceTo
  try unfold borrowGlobal
  try unfold borrowField
  try unfold borrowVariantField
  try unfold borrowLoc
  try unfold borrowElement
  try unfold Step.call
  try unfold Step.ret
  repeat' (first
    | exact execOk_wrong
    | exact execOk_fail
    | (refine execOk_bind (by ends_tac) fun _ _ => ?_)
    | (refine execOk_next ?_ ?_)
    | (refine execOk_finish ?_ ?_)
    | (refine execOk_running ?_ ?_)
    | (refine execOk_returned (fun v hv => List.mem_reverse.1 hv) ?_)
    | exact execOk_callNative
    | split
    | dsimp only)
  all_goals first
    | (exact keys_congr (by assumption) (by rfl))
    | (exact topFresh_other rfl rfl)
    | (exact topFresh_other' rfl rfl)
    | (exact keys_update (by assumption) (keys_congr (by assumption) (by rfl)))
    | (exact keys_remove (by assumption) (by rfl))
    | (exact keys_insert (by assumption) (globalType_key (by assumption)) (by rfl))
    | (exact topFresh_replaces (plainOf (P := default) (intBinop_plain ‹_›)) rfl)
    | (exact topFresh_replaces (plainOf (P := default) (cast_plain ‹_›)) rfl)
    | (exact topFresh_replaces (plainOf (P := default) (shift_plain ‹_›)) rfl)
    | (exact topFresh_replaces (plainOf (P := default) (boolBinop_plain ‹_›)) rfl)
    | (exact topFresh_replaces (plainOf (P := default) (intCompare_plain ‹_›)) rfl)
    | (exact topFresh_replaces (equality_plain ‹_›) rfl)
    | ((try intro _ _ _)
       refine topFresh_of fun w hw => ?_
       simp [Shadow.topAfter, advance, push, pushAll, ldInt] at hw
       subst hw
       first
       | exact fresh_const (need_ok' ‹_›)
       | exact fresh_container rfl rfl
       | exact fresh_leaf rfl rfl)
    | exact rfl))

set_option maxHeartbeats 20000000 in
/-- **What a step of an instruction other than the closure-packing ones
establishes**: it keeps every resource in global storage of a type with
`key`, a replacing or packing instruction pushes no new closure, and a
returning step returns values of the segment with global storage unchanged.
`CallClosure` enters a frame and keeps global storage (`callClosure_cont`). -/
theorem exec_ok {P : Program} {s : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
    {code : CodeUnit} {instr : Bytecode} (hk : TypeKeys P s)
    (hnc : ∀ h m, instr ≠ .packClosure h m) (hng : ∀ i m, instr ≠ .packClosureGeneric i m) :
    ExecOk P s fr instr (Step.exec P s fr rest info code instr) := by
  cases instr
  case packClosure h m => exact absurd rfl (hnc h m)
  case packClosureGeneric i m => exact absurd rfl (hng i m)
  case callClosure sig =>
    unfold ExecOk
    intro o ho hc
    obtain ⟨-, -, -, -, -, -, -, s', -, -, -, -, rfl, hstor, -⟩ := callClosure_cont hc ho
    exact ⟨keys_congr hk hstor, topFresh_other rfl rfl⟩
  all_goals exec_walk

end MoveBytecode.Proofs
