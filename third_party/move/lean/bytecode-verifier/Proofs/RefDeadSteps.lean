-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.StepsDefs
import Proofs.DanglingM3

/-!
# D4e at each step, for verified programs

UNTRUSTED (checked by the kernel). After a step of a verified program that
makes a local unavailable or ends a frame, no reference the analysis tracks
points into it (`stepsDead_tracked`). Only `MoveLoc` makes a local
unavailable and only `Ret` ends a frame (`sd_exec_ends`). Every reference the
analysis tracks after the step is valid there (`tracked_refOk`), so its
location resolves; but the local `MoveLoc` moved out holds nothing, and the
frame `Ret` popped is gone.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

/-! ## The steps that end a root -/

/-- What an event of instruction `instr` of frame `fr` ends: a local of `fr`
that `MoveLoc` moves out, or `fr` itself at `Ret`. -/
private def EndsBy (instr : Bytecode) (fr : Frame) : Event → Prop
  | .vacated ρ => ∃ j, instr = .moveLoc j ∧ ρ = .local_ fr.id j
  | .frameEnded f _ => instr = .ret ∧ f = fr.id
  | .created .. | .consumed .. | .borrowed .. | .used .. | .accessed .. | .entered ..
  | .privileged .. => True

/-- Creations end nothing. -/
private theorem sd_all_fresh {instr : Bytecode} {fr : Frame} {c : Creator} {v : Value}
    {n : Nat} : ∀ e ∈ (IdTree.fresh c v n).2.2, EndsBy instr fr e := by
  rw [(fresh_spec c v n).2.2.2]
  intro e he
  obtain ⟨⟨i, w⟩, -, rfl⟩ := List.mem_map.1 he
  trivial

/-- **Only `MoveLoc` makes a local unavailable, and only `Ret` ends a
frame.** -/
private theorem sd_exec_ends {P : Program} {m : ModuleId} {instr : Bytecode} {s : State}
    {fr : Frame} {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event}
    {g : ShadowFrame} {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (post, evs)) {e : Event} (he : e ∈ evs) :
    EndsBy instr fr e := by
  cases instr <;>
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq, Prod.mk.injEq, guard, reduceCtorEq] at hex
  all_goals (try simp_all)
  case ret =>
    split at hex <;>
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      simp only [List.mem_append, List.mem_map, List.mem_singleton] at he
      rcases he with ⟨_, -, rfl⟩ | rfl
      · trivial
      · exact ⟨rfl, rfl⟩
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldConst | ldTrue | ldFalse =>
    obtain ⟨a, -, -, rfl⟩ := hex
    exact sd_all_fresh e he
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32
    | castI64 | castI128 | castI256 | not | negate | add | sub | mul | div | mod | bitOr | bitAnd
    | xor | or | and | lt | gt | le | ge | shl | shr =>
    obtain ⟨-, a, -, -, rfl⟩ := hex
    exact sd_all_fresh e he
  case pack | packVariant | vecPack | packGeneric | packVariantGeneric | packClosure
    | packClosureGeneric =>
    obtain ⟨a, -, -, -, rfl⟩ := hex
    revert e
    simp [EndsBy]
  case callClosure =>
    split at hex
    all_goals (try split at hex)
    all_goals (try split at hex)
    all_goals (try split at hex)
    all_goals first
      | cases hex
      | (obtain ⟨_, -, hex⟩ := Option.bind_eq_some_iff.1 hex
         simp only [Option.some.injEq, Prod.mk.injEq] at hex
         obtain ⟨-, rfl⟩ := hex
         simp only [List.mem_append, List.mem_map, List.mem_cons,
           List.not_mem_nil, or_false] at he
         rcases he with ⟨_, -, rfl⟩ | rfl | rfl <;> trivial)
      | simp at hex
  case unpack | unpackVariant | vecUnpack | unpackGeneric | unpackVariantGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨v, -, -, rfl⟩ := hex
      revert e
      simp [EndsBy]
    · cases hex
  case eq | neq =>
    obtain ⟨a, b, ⟨-, a₁, -, -, rfl⟩, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp only [List.mem_cons] at he
      rcases he with rfl | rfl | rfl | rfl | he
      any_goals trivial
      exact sd_all_fresh e he
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      exact sd_all_fresh e he
  case copyLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      revert e
      split <;> simp [EndsBy]
    · cases hex
  case moveLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      revert e
      split <;> simp [EndsBy]
    · cases hex
  case stLoc j =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      revert e
      split <;> simp [EndsBy]
    · cases hex
  case call | callGeneric =>
    split at hex
    · split at hex
      · obtain ⟨_, -, hex⟩ := Option.bind_eq_some_iff.1 hex
        simp only [Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, rfl⟩ := hex
        simp only [List.mem_append, List.mem_map, List.mem_singleton] at he
        rcases he with ⟨_, -, rfl⟩ | rfl <;> trivial
      · cases hex
    · cases hex
  case readRef =>
    obtain ⟨a, b, hra, a₁, -, -, rfl⟩ := hex
    revert e
    simp [EndsBy]
  case writeRef =>
    obtain ⟨a, b, hra, a₁, -, a₂, -, -, rfl⟩ := hex
    revert e
    simp [EndsBy]
  case freezeRef =>
    obtain ⟨a, b, -, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      revert e
      simp [EndsBy]
    · cases hm
  case testVariant | testVariantGeneric =>
    obtain ⟨a, b, -, hm⟩ := hex
    split at hm
    · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨x, hx, -, rfl⟩ := hm
      obtain ⟨-, -, v, -, rfl⟩ := hx
      simp only [List.mem_cons, List.mem_append, List.not_mem_nil, or_false] at he
      rcases he with rfl | rfl | he | rfl
      any_goals trivial
      exact sd_all_fresh e he
    · cases hm
  case mutBorrowField | immBorrowField | mutBorrowVariantField | immBorrowVariantField
    | mutBorrowFieldGeneric | immBorrowFieldGeneric | mutBorrowVariantFieldGeneric
    | immBorrowVariantFieldGeneric =>
    obtain ⟨a, b, -, hm⟩ := hex
    split at hm
    · obtain ⟨_, -, hm⟩ := Option.bind_eq_some_iff.1 hm
      split at hm
      · simp only [Option.some.injEq, Prod.mk.injEq] at hm
        obtain ⟨-, rfl⟩ := hm
        revert e
        simp [EndsBy]
      · cases hm
    · cases hm
  case vecLen =>
    obtain ⟨a, b, -, a₁, b₁, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    simp only [List.mem_cons] at he
    rcases he with rfl | rfl | he
    any_goals trivial
    exact sd_all_fresh e he
  case vecImmBorrow | vecMutBorrow =>
    obtain ⟨a, b, -, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      revert e
      simp [EndsBy]
    · cases hm
  case vecPushBack =>
    obtain ⟨a, -, a₁, b, hra, a₂, -, -, rfl⟩ := hex
    revert e
    simp [EndsBy]
  case vecPopBack =>
    obtain ⟨a, b, hra, a₁, -, a₂, -, a₃, -, hm⟩ := hex
    split at hm
    · simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      revert e
      simp [EndsBy]
    · cases hm
  case vecSwap =>
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨x, hra, y, -, -, rfl⟩ := hex
        revert e
        simp [EndsBy]
      · cases hex
    · cases hex
  case exists_ =>
    obtain ⟨a, -, a₁, b, ⟨-, a', -, -, rfl⟩, -, rfl⟩ := hex
    simp only [List.mem_append, List.mem_singleton] at he
    rcases he with he | rfl
    · exact sd_all_fresh e he
    · trivial
  case existsGeneric =>
    obtain ⟨_, _, _, _, _, _, _, _, a, b, ⟨_, a', _, _, rfl⟩, _, rfl⟩ := hex
    simp only [List.mem_append, List.mem_singleton] at he
    rcases he with he | rfl
    · exact sd_all_fresh e he
    · trivial
  case moveFrom | moveFromGeneric =>
    split at hex
    · simp only [Option.bind_eq_some_iff] at hex
      obtain ⟨v, -, hex⟩ := hex
      split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨k, -, t, -, -, rfl⟩ := hex
        revert e
        simp [EndsBy]
      · cases hex
    · cases hex
  case moveTo | moveToGeneric =>
    obtain ⟨a, -, a₁, -, hm⟩ := hex
    split at hm
    · simp only [Option.bind_eq_some_iff] at hm
      obtain ⟨x, hra, hm⟩ := hm
      split at hm
      · simp only [Option.some.injEq, Prod.mk.injEq] at hm
        obtain ⟨-, rfl⟩ := hm
        revert e
        simp [EndsBy]
      · cases hm
    · cases hm
  case mutBorrowLoc | immBorrowLoc =>
    split at hex
    · simp only [Option.some.injEq, Prod.mk.injEq] at hex
      obtain ⟨-, rfl⟩ := hex
      revert e
      simp [EndsBy]
    · cases hex
  case mutBorrowGlobal | immBorrowGlobal | mutBorrowGlobalGeneric | immBorrowGlobalGeneric =>
    split at hex
    · split at hex
      · simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hex
        obtain ⟨-, -, -, rfl⟩ := hex
        revert e
        simp [EndsBy]
      · cases hex
    · cases hex


/-! ## What `MoveLoc` and `Ret` leave -/

section Steps

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

/-- The step of the current frame's instruction. -/
private theorem sd_step {instr : Bytecode} (hsf : s.frames = fr :: rest)
    (hinfo : P.function? fr.function = some info) (hcode : info.definition.code = some code)
    (hi : code.code[fr.pc]? = some instr) :
    step P s = stepOf (Step.exec P s fr rest info code instr) := by
  unfold step
  rw [hsf]
  simp only [hinfo, hcode, hi]
  rfl

/-- After `MoveLoc j`, nothing resolves in local `j`. -/
private theorem sd_moveLoc {j : LocalIndex}
    (h : stepOf (Step.exec P s fr rest info code (.moveLoc j)) = .done (.running s'))
    (p : List PathStep) : s'.read? ⟨.local_ fr.id j, p⟩ = none := by
  obtain ⟨slot, hslot, h⟩ := done_bind h rfl (ends_need _ _)
  cases slot with
  | none => exact (done_wrong h rfl).elim
  | some v =>
    obtain rfl := Outcome.running.inj (done_next h)
    have hlt : j < fr.locals.size := (Array.getElem?_eq_some_iff.1 (need_ok hslot)).1
    simp [State.read?, State.root?, State.frame?, advance, push,
      Array.getElem?_setIfInBounds_self_of_lt hlt]

/-- After `Ret` to a caller, nothing resolves in the returning frame's
locals: its identity is no longer a frame's. -/
private theorem sd_ret (hsf : s.frames = fr :: rest) (hnd : (s.frames.map (·.id)).Nodup)
    (h : stepOf (Step.exec P s fr rest info code .ret) = .done (.running s')) (k : Nat)
    (p : List PathStep) : s'.read? ⟨.local_ fr.id k, p⟩ = none := by
  unfold Step.exec Step.ret at h
  obtain ⟨retTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  cases rest with
  | nil => simp [stepOf, pure, Except.pure] at h
  | cons caller rest' =>
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq, Outcome.running.injEq] at h
    rw [hsf] at hnd
    simp only [List.map_cons, List.nodup_cons, List.mem_cons, List.mem_map, not_or,
      not_exists, not_and] at hnd
    have hfind :
        (({ caller with stack := fr.stack ++ caller.stack, pc := caller.pc + 1 } : Frame) ::
          rest').find? (·.id == fr.id) = none := by
      rw [List.find?_eq_none]
      intro g hg
      simp only [List.mem_cons] at hg
      rcases hg with rfl | hg
      · simpa using fun e => hnd.1.1 e.symm
      · simpa using fun e => hnd.1.2 g hg e
    cases hca : fr.countedActive <;> cases hle : fr.lockEntered <;>
      simp only [hca, hle, Bool.false_eq_true, ↓reduceIte] at h <;> subst h <;>
      simp [State.read?, State.root?, State.frame?, hfind]

end Steps

/-- **What a step that ends a root leaves**: after the step, no location in
the root it ends resolves. -/
theorem sd_ends {P : Program} {N : NativeRel} {ν : NativeFlows} {s s' : State}
    {h : Shadow} {E evs : List Event} {o : Outcome} {post : ShadowPost}
    (hI : RefTraceInv P s h E) (hS : TracedStep P N ν s h o post evs) (ho : o = .running s')
    {e : Event} (he : e ∈ evs) {ended : Root → Bool} (hend : e.ends? = some ended)
    {l : Location} (hread : (s'.read? l).isSome = true) : ended l.root = false := by
  cases hS with
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      rw [(nativeStep_quiet hn he).2.2.1] at hend
      cases hend
    obtain ⟨rfl, rfl, rfl, g, hs, info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at he
    simp only [nativeEvents, List.mem_append] at he
    rcases he with he | he
    · obtain ⟨_, _, _, _, rfl, -⟩ := consumedFrom_mem _ he
      simp [Event.ends?] at hend
    · obtain ⟨_, _, _, rfl⟩ := (nativeResults_spec _ _ _ _ _ _ _).2.2 _ he
      simp [Event.ends?] at hend
  | core hstep hc =>
    subst ho
    obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hex⟩ := coreStep_exec hc
    have hstep' := (sd_step hsf hinfo hcode hinstr).symm.trans hstep
    cases hfr : h.frames with
    | nil =>
      have hfit := hI.pre.fits
      rw [shadow_fits, hsf, hfr] at hfit
      simp [framesFit] at hfit
    | cons g hs =>
      have hk := sd_exec_ends hfr hex he
      cases e
      case vacated ρ =>
        obtain ⟨j, rfl, rfl⟩ := hk
        simp only [Event.ends?, Option.some.injEq] at hend
        subst hend
        cases hb : (l.root == Root.local_ fr.id j) with
        | false => exact hb
        | true =>
          rw [beq_iff_eq] at hb
          rw [Location.eta l, hb, sd_moveLoc hstep'] at hread
          cases hread
      case frameEnded f rs =>
        obtain ⟨rfl, rfl⟩ := hk
        simp only [Event.ends?, Option.some.injEq] at hend
        subst hend
        cases hρ : l.root with
        | local_ g' k =>
          cases hb : (g' == fr.id) with
          | false => exact hb
          | true =>
            rw [beq_iff_eq] at hb
            subst hb
            rw [Location.eta l, hρ, sd_ret hsf hI.pre.inv.frameIds hstep'] at hread
            cases hread
        | global a t => rfl
        | held k => rfl
      all_goals simp [Event.ends?] at hend

/-! ## D4e at each step -/

/-- **After a step that makes a local unavailable or ends a frame, no
tracked reference points into it**, in a verified program. -/
theorem stepsDead_tracked {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.SupportedNonGeneric) (hm3 : P.M3)
    (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀) :
    StepsDead P N ν s₀ (Tracked P) := by
  intro s h E hT s' h' evs hS e he ended hend i r ht
  have hinv' := invM3_reachable hwf hs hm3 (certified_of_wellFormed schedulerSound hwf hs) hN h0
    s' (trace_reachable (Trace.step hT hS))
  obtain ⟨fr'', -, hok⟩ := tracked_refOk hwf hs hm3 hinv' ht
  exact sd_ends (refTraceInv_trace h0 hT) hS rfl he hend (deref?_isSome.1 hok.valid).2

end MoveBytecode.Proofs
