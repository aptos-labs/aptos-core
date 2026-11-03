-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostWrites
import Proofs.ByValue
import Proofs.TableRules

/-!
# The ghost layer annotates every instruction that continues

UNTRUSTED (checked by the kernel). For an instruction of the deterministic
core that continues (`running` or `returned`), from a state whose shadow fits
it and that satisfies the state invariant (`Pre`), `Shadow.exec` annotates
the step, and the annotation establishes `StepGhost`. The semantics' helpers
give the concrete outcome's shape (`Proofs.GhostSemantics`); the patterns of
`Proofs.GhostSteps` and `Proofs.GhostWrites` give the ghost facts for each
shape.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Step

/-- The annotation of instruction `instr` of module `m`, executed from state
`s` with current frame `fr`, whose shadow is `h`, exists and establishes
`StepGhost`. -/
def Annotated (σ : Bool) (P : Program) (π : NativePolicy) (A : List StructId) (auth : List Address)
    (m : ModuleId) (instr : Bytecode) (s : State) (fr : Frame) (o : Outcome) (h : Shadow) : Prop :=
  ∃ post evs, Shadow.exec P m instr s fr o h = some (post, evs) ∧
    StepGhost σ P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh o post evs

theorem primitive_drop {P : Program} {v : Value} (h : Primitive v = true) :
    (v.abilities P).drop = true := by
  cases v <;> simp_all [Primitive, Value.abilities, RuntimeType.abilities, AbilitySet.primitives]

theorem primitive_plain {v : Value} (h : Primitive v = true) :
    ∀ w ∈ subvalues v, IsPlain w = true := by
  cases v <;> simp_all [Primitive, subvalues, Value.nodes, IsPlain]

/-- Silent events of D4 respect D3. -/
theorem silent_ok {π : NativePolicy} {A : List StructId} {es : List Event}
    (h : ∀ e ∈ es, e.Silent = true ∧ ∀ m s i, e ≠ .privileged m s i) :
    ∀ e ∈ es, e.Silent = true ∧ Encapsulation.structsRespected π A e = true := by
  intro e he
  obtain ⟨h1, h2⟩ := h e he
  refine ⟨h1, ?_⟩
  cases e with
  | privileged m s i => exact absurd rfl (h2 m s i)
  | _ => simp_all [Event.Silent, Encapsulation.structsRespected]

section Cases

variable {P : Program} {π : NativePolicy} {A : List StructId} {auth : List Address}
  {s s₂ : State} {fr fr' : Frame} {rest : List Frame} {g g' : ShadowFrame} {hs : List ShadowFrame}
  {hst hhe : List IdTree} {f f' : Nat} {m : ModuleId} {instr : Bytecode} {o : Outcome}
  {evs : List Event}

/-- The current frames of a state and its shadow. -/
theorem pre_head (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest) :
    g.fits fr = true ∧ framesFit hs rest = true := by
  have := hp.fits
  rw [shadow_fits, hsf] at this
  simp only [framesFit_cons, Bool.and_eq_true] at this
  exact this.1

/-- A step that changes only the current frame. -/
theorem frame_case (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hshadow : Shadow.exec P m instr s fr o ⟨g :: hs, hst, hhe, f⟩ =
      some (.running ⟨g' :: hs, hst, hhe, f'⟩, evs))
    (ho : o = .running { s₂ with frames := fr' :: rest }) (h2st : s₂.storage = s.storage)
    (h2he : s₂.held = s.held) (h2nf : s₂.nextFrame = s.nextFrame) (hid : fr'.id = fr.id)
    (hG : FrameGhost σ P π A auth g fr f g' fr' f' evs) :
    Annotated σ P π A auth m instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  subst ho
  exact ⟨_, _, hshadow, lift_frame hp hsf rfl h2st h2he h2nf hid hG⟩

/-- A step that changes only the current frame, with silent events around its
ghost facts' events. -/
theorem frame_case' {pre post : List Event}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hshadow : Shadow.exec P m instr s fr o ⟨g :: hs, hst, hhe, f⟩ =
      some (.running ⟨g' :: hs, hst, hhe, f'⟩, pre ++ evs ++ post))
    (hpre : ∀ e ∈ pre, e.Silent = true ∧ Encapsulation.structsRespected π A e = true)
    (hpost : ∀ e ∈ post, e.Silent = true ∧ Encapsulation.structsRespected π A e = true)
    (ho : o = .running { s₂ with frames := fr' :: rest }) (h2st : s₂.storage = s.storage)
    (h2he : s₂.held = s.held) (h2nf : s₂.nextFrame = s.nextFrame) (hid : fr'.id = fr.id)
    (hG : FrameGhost σ P π A auth g fr f g' fr' f' evs) :
    Annotated σ P π A auth m instr s fr o ⟨g :: hs, hst, hhe, f⟩ :=
  frame_case hp hsf hshadow ho h2st h2he h2nf hid (hG.silent hpre hpost)

/-- The key of a reference on the current frame's segment satisfies the key
invariant. -/
theorem stack_key_ok (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest) {k : Nat}
    {r : Reference} (hk : fr.stack[k]? = some (.ref r)) :
    (Value.ref r).key.Ok σ ∧ ∃ t, g.stack[k]? = some t ∧ t.fits (.ref r) = true := by
  have hg := (pre_head hp hsf).1
  obtain ⟨t, htk, ht⟩ := fitsAll_get (frame_stack_fits hg) hk
  refine ⟨?_, t, htk, ht⟩
  have hmem : (t.id, Value.ref r) ∈ framesOcc (g :: hs) s.frames := by
    rw [hsf, framesOcc_cons, frame_occurrences]
    have h1 : (t.id, Value.ref r) ∈ t.occurrences (.ref r) := by rw [ref_occ ht]; simp
    have h2 := occurrencesAll_eq_flatMap g.stack fr.stack
    refine List.mem_append_left _ (List.mem_append_right _ ?_)
    rw [h2]
    exact List.mem_flatMap.2 ⟨(t, .ref r), List.mem_of_getElem? (zip_getElem? htk hk), h1⟩
  exact hp.keysOk _ (mem_keys hmem)

/-- A reference on the current frame's segment whose location is valid, to
a target without subtypes, designates a value of the target's type. -/
theorem stack_typed_rigid (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    {k : Nat} {r : Reference} (hk : fr.stack[k]? = some (.ref r)) {v : Value}
    (hv : s.deref? r.location r.since = some v) (hr : rigid r.target = true) : v.type = r.target :=
  hp.typed fr (by rw [hsf]; exact List.mem_cons_self ..) r (Or.inl (List.mem_of_getElem? hk)) v hv hr

end Cases

/-! ## Instructions -/

theorem need_ok {α : Type} {x : Option α} {r : WrongReason} {a : α} (h : need x r = .ok a) :
    x = some a := by
  cases x <;> simp_all [need, pure, Except.pure, wrong]

theorem done_pure_callNative {s : State} {f : FunctionRef} {args views : List Value} {o : Outcome}
    (h : stepOf (pure (StepResult.callNative s f args views) : Exec StepResult) = .done o) :
    False := by
  simp [stepOf, pure, Except.pure] at h

/-- The native branch of a call suspends or goes wrong: it is done only
wrong. -/
theorem done_native {s : State} {f : FunctionRef} {args : List Value}
    {x : Option (List Value)} {c : Bool} {o : Outcome}
    (h : stepOf (do
        let views ← need x .invalidReference
        if c = true then wrong .overlappingReferences
        else pure (StepResult.callNative s f args views)) = .done o)
    (hcont : o.continues = true) : False := by
  obtain ⟨views, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact done_wrong h hcont
  · exact done_pure_callNative h

section Instructions

variable {P : Program} {π : NativePolicy} {A : List StructId} {auth : List Address}
  {s : State} {fr : Frame} {rest : List Frame} {g : ShadowFrame} {hs : List ShadowFrame}
  {hst hhe : List IdTree} {f : Nat} {info : FunctionInfo} {code : CodeUnit} {o : Outcome}

/-- The instructions that pop the top value and discard it. -/
theorem pop_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (h : stepOf (Step.exec P s fr rest info code .pop) = .done o) :
    Annotated σ P π A auth info.moduleId .pop s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h hcont pop_ends
  obtain ⟨st, hst', rfl⟩ := pop_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  · next hd =>
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hd
    obtain ⟨t, ts, hgst, hG⟩ := discard_ghost (P := P) (π := π) (A := A) (auth := auth) (f := f)
      hg hst' hd (fr.pc + 1)
    exact frame_case hp hsf (by simp [Shadow.exec, guard, hgst]) (done_next h) rfl rfl rfl rfl hG

theorem brTrue_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) {off : CodeOffset}
    (h : stepOf (Step.exec P s fr rest info code (.brTrue off)) = .done o) :
    Annotated σ P π A auth info.moduleId (.brTrue off) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  obtain ⟨⟨b, fr₁⟩, hpb, h⟩ := done_bind h hcont popBool_ends
  obtain ⟨st, hst', rfl⟩ := popBool_ok hpb
  have hd : ((Value.bool b).abilities P).drop = true := primitive_drop rfl
  cases b
  · obtain ⟨t, ts, hgst, hG⟩ := discard_ghost (P := P) (π := π) (A := A) (auth := auth) (f := f)
      hg hst' hd (fr.pc + 1)
    exact frame_case hp hsf (by simp [Shadow.exec, guard, hgst])
      (done_next (s := s) (rest := rest) (fr := advance { fr with stack := st }) h) rfl rfl rfl rfl hG
  · obtain ⟨fr₂, hj, h⟩ := done_bind h hcont jump_ends
    rw [jump_ok hj] at h
    obtain ⟨t, ts, hgst, hG⟩ := discard_ghost (P := P) (π := π) (A := A) (auth := auth) (f := f)
      hg hst' hd off
    exact frame_case hp hsf (by simp [Shadow.exec, guard, hgst]) (done_next h) rfl rfl rfl rfl hG

theorem brFalse_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) {off : CodeOffset}
    (h : stepOf (Step.exec P s fr rest info code (.brFalse off)) = .done o) :
    Annotated σ P π A auth info.moduleId (.brFalse off) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  obtain ⟨⟨b, fr₁⟩, hpb, h⟩ := done_bind h hcont popBool_ends
  obtain ⟨st, hst', rfl⟩ := popBool_ok hpb
  have hd : ((Value.bool b).abilities P).drop = true := primitive_drop rfl
  cases b
  · obtain ⟨fr₂, hj, h⟩ := done_bind h hcont jump_ends
    rw [jump_ok hj] at h
    obtain ⟨t, ts, hgst, hG⟩ := discard_ghost (P := P) (π := π) (A := A) (auth := auth) (f := f)
      hg hst' hd off
    exact frame_case hp hsf (by simp [Shadow.exec, guard, hgst]) (done_next h) rfl rfl rfl rfl hG
  · obtain ⟨t, ts, hgst, hG⟩ := discard_ghost (P := P) (π := π) (A := A) (auth := auth) (f := f)
      hg hst' hd (fr.pc + 1)
    exact frame_case hp hsf (by simp [Shadow.exec, guard, hgst])
      (done_next (s := s) (rest := rest) (fr := advance { fr with stack := st }) h) rfl rfl rfl rfl hG

theorem branch_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) {off : CodeOffset}
    (h : stepOf (Step.exec P s fr rest info code (.branch off)) = .done o) :
    Annotated σ P π A auth info.moduleId (.branch off) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨fr₂, hj, h⟩ := done_bind h hcont jump_ends
  rw [jump_ok hj] at h
  exact frame_case hp hsf (by simp [Shadow.exec]) (done_next h) rfl rfl rfl rfl
    (keep_ghost (pre_head hp hsf).1 off)

theorem nop_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code .nop) = .done o) :
    Annotated σ P π A auth info.moduleId .nop s fr o ⟨g :: hs, hst, hhe, f⟩ :=
  frame_case hp hsf (by simp [Shadow.exec]) (done_next h) rfl rfl rfl rfl
    (keep_ghost (pre_head hp hsf).1 _)

/-- What the helpers of the replacing instructions establish. -/
def ReplacesPlain (P : Program) (fr fr' : Frame) (n : Nat) : Prop :=
  ∃ v, Replaces fr fr' n v ∧ (∀ w ∈ fr.stack.take n, (w.abilities P).drop = true) ∧
    ∀ w ∈ subvalues v, IsPlain w = true

/-- The shadow after replacing the top `n` slots of `g` by a value `v`
created by `c`. -/
def replaced (g : ShadowFrame) (hs : List ShadowFrame) (hst hhe : List IdTree) (c : Creator)
    (v : Value) (f n : Nat) : ShadowPost :=
  .running ⟨{ g with stack := (IdTree.fresh c v f).1 :: g.stack.drop n } :: hs, hst, hhe,
    (IdTree.fresh c v f).2.1⟩

/-- The literal, constant, cast, arithmetic, comparison and boolean
instructions, with silent events before and after the replacement's. -/
theorem replace_annot {instr : Bytecode} {n : Nat} {pre post : List Event} {v : Value}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hshadow : n ≤ g.stack.length →
      Shadow.exec P info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ =
        some (replaced g hs hst hhe (.instruction info.moduleId instr) v f n,
          pre ++ (IdTree.fresh (.instruction info.moduleId instr) v f).2.2 ++ post))
    (hpre : ∀ e ∈ pre, e.Silent = true ∧ Encapsulation.structsRespected π A e = true)
    (hpost : ∀ e ∈ post, e.Silent = true ∧ Encapsulation.structsRespected π A e = true)
    (hn : n ≤ fr.stack.length) (hdrop : ∀ w ∈ fr.stack.take n, (w.abilities P).drop = true)
    (hplain : ∀ w ∈ subvalues v, IsPlain w = true)
    (ho : o = .running { s with frames := { fr with stack := v :: fr.stack.drop n, pc := fr.pc + 1 } :: rest }) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  have hlen := fitsAll_length (frame_stack_fits hg)
  exact frame_case' hp hsf (hshadow (by omega)) hpre hpost ho rfl rfl rfl rfl
    (replace_ghost hg hdrop hplain _)

/-- `replace_annot` without other events. -/
theorem replace_annot0 {instr : Bytecode} {n : Nat} {v : Value}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hshadow : n ≤ g.stack.length →
      Shadow.exec P info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ =
        some (replaced g hs hst hhe (.instruction info.moduleId instr) v f n,
          (IdTree.fresh (.instruction info.moduleId instr) v f).2.2))
    (hn : n ≤ fr.stack.length) (hdrop : ∀ w ∈ fr.stack.take n, (w.abilities P).drop = true)
    (hplain : ∀ w ∈ subvalues v, IsPlain w = true)
    (ho : o = .running { s with frames := { fr with stack := v :: fr.stack.drop n, pc := fr.pc + 1 } :: rest }) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ :=
  replace_annot (pre := []) (post := []) hp hsf (by simpa using hshadow) (by simp) (by simp) hn hdrop
    hplain ho

theorem replace_bind {instr : Bytecode} {n : Nat}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (hshadow : ∀ v st, n ≤ g.stack.length →
      Shadow.exec P info.moduleId instr s fr
        (.running { s with frames := { fr with stack := v :: st, pc := fr.pc + 1 } :: rest })
        ⟨g :: hs, hst, hhe, f⟩ =
        some (replaced g hs hst hhe (.instruction info.moduleId instr) v f n,
          (IdTree.fresh (.instruction info.moduleId instr) v f).2.2))
    {x : Exec Frame} (hx : Ends x) (hok : ∀ fr', x = .ok fr' → ReplacesPlain P fr fr' n)
    (h : stepOf (x >>= fun fr' => next s rest fr') = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨fr', hx', h⟩ := done_bind h hcont hx
  obtain ⟨v, ⟨hn, rfl⟩, hdrop, hplain⟩ := hok fr' hx'
  have ho := done_next h
  subst ho
  exact replace_annot0 hp hsf (hshadow v _) hn hdrop hplain rfl

theorem intBinop_plain {fr' : Frame} {allow : IntType → Bool}
    {op : IntType → Int → Int → Option Int} (h : intBinop fr allow op = .ok fr') :
    ReplacesPlain P fr fr' 2 := by
  obtain ⟨t, r, hR, hp⟩ := intBinop_ok h
  exact ⟨_, hR, fun w hw => primitive_drop (hp w hw), primitive_plain rfl⟩

theorem intCompare_plain {fr' : Frame} {op : Int → Int → Bool}
    (h : intCompare fr op = .ok fr') :
    ReplacesPlain P fr fr' 2 := by
  obtain ⟨b, hR, hp⟩ := intCompare_ok h
  exact ⟨_, hR, fun w hw => primitive_drop (hp w hw), primitive_plain rfl⟩

theorem boolBinop_plain {fr' : Frame} {op : Bool → Bool → Bool}
    (h : boolBinop fr op = .ok fr') :
    ReplacesPlain P fr fr' 2 := by
  obtain ⟨b, hR, hp⟩ := boolBinop_ok h
  exact ⟨_, hR, fun w hw => primitive_drop (hp w hw), primitive_plain rfl⟩

theorem shift_plain {fr' : Frame} {op : IntType → Int → Nat → Option Int}
    (h : shift fr op = .ok fr') : ReplacesPlain P fr fr' 2 := by
  obtain ⟨t, r, hR, hp⟩ := shift_ok h
  exact ⟨_, hR, fun w hw => primitive_drop (hp w hw), primitive_plain rfl⟩

theorem cast_plain {fr' : Frame} {target : IntType} (h : Step.cast fr target = .ok fr') :
    ReplacesPlain P fr fr' 1 := by
  obtain ⟨r, hR, hp⟩ := cast_ok h
  exact ⟨_, hR, fun w hw => primitive_drop (hp w hw), primitive_plain rfl⟩

theorem top_drop {v : Value} {st : List Value} (hst : fr.stack = v :: st)
    (hd : (v.abilities P).drop = true) :
    ∀ w ∈ fr.stack.take 1, (w.abilities P).drop = true := by
  simp only [hst, List.take_succ_cons, List.take_zero, List.mem_singleton]
  rintro w rfl
  exact hd

theorem ldConst_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) {i : ConstantPoolIndex}
    (h : stepOf (Step.exec P s fr rest info code (.ldConst i)) = .done o) :
    Annotated σ P π A auth info.moduleId (.ldConst i) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨c, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨t, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨v, hv, h⟩ := done_bind h hcont (ends_need _ _)
  have ho := done_next h
  exact replace_annot0 hp hsf
    (fun _ => by rw [ho]; simp [Shadow.exec, guard, Shadow.topAfter, replaced, advance, push])
    (Nat.zero_le _) (by simp) (decodeConstant_plain (need_ok hv)) (by simpa [advance, push] using ho)

theorem lit_annot {instr : Bytecode} {v : Value} (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩)
    (hsf : s.frames = fr :: rest) (hplain : ∀ w ∈ subvalues v, IsPlain w = true)
    (hshadow : Shadow.exec P info.moduleId instr s fr
        (.running { s with frames := { fr with stack := v :: fr.stack, pc := fr.pc + 1 } :: rest })
        ⟨g :: hs, hst, hhe, f⟩ =
      some (replaced g hs hst hhe (.instruction info.moduleId instr) v f 0,
        (IdTree.fresh (.instruction info.moduleId instr) v f).2.2))
    (h : stepOf (next s rest (advance (push fr v))) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have ho := done_next h
  subst ho
  exact replace_annot0 hp hsf (fun _ => hshadow) (Nat.zero_le _) (by simp) hplain
    (by simp [advance, push])

theorem not_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (Step.exec P s fr rest info code .not) = .done o) :
    Annotated σ P π A auth info.moduleId .not s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨⟨b, fr₁⟩, hpb, h⟩ := done_bind h hcont popBool_ends
  obtain ⟨st, hst', rfl⟩ := popBool_ok hpb
  have ho := done_next h
  rw [show st = fr.stack.drop 1 by simp [hst']] at ho
  exact replace_annot0 hp hsf
    (fun hn => by rw [ho]; simp [Shadow.exec, guard, Shadow.topAfter, replaced, hn, advance, push])
    (by simp [hst']) (top_drop hst' (primitive_drop rfl)) (primitive_plain (v := .bool !b) rfl) ho

theorem negate_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (Step.exec P s fr rest info code .negate) = .done o) :
    Annotated σ P π A auth info.moduleId .negate s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨⟨t, x, fr₁⟩, hpi, h⟩ := done_bind h hcont popInt_ends
  obtain ⟨st, hst', rfl⟩ := popInt_ok hpi
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  · split at h
    · next r _ =>
      have ho := done_next h
      rw [show st = fr.stack.drop 1 by simp [hst']] at ho
      exact replace_annot0 hp hsf
        (fun hn => by rw [ho]; simp [Shadow.exec, guard, Shadow.topAfter, replaced, hn, advance, push])
        (by simp [hst']) (top_drop hst' (primitive_drop rfl)) (primitive_plain (v := .int t r) rfl) ho
    · exact (done_fail h hcont).elim

/-- D4's records of uses, accesses, locals and frames. -/
def plainEvent : Event → Bool
  | .used .. | .accessed .. | .vacated .. | .entered .. | .frameEnded .. => true
  | .created .. | .consumed .. | .borrowed .. | .privileged .. => false

/-- Silent events of D4 that are not D3a's records. -/
def Plain4 (es : List Event) : Prop := es.all plainEvent = true

omit P auth s fr rest g hs hst hhe f info code o in
theorem plain4_ok {es : List Event} (h : Plain4 es) :
    ∀ e ∈ es, e.Silent = true ∧ Encapsulation.structsRespected π A e = true := by
  intro e he
  have := List.all_eq_true.1 h e he
  cases e <;> simp_all [plainEvent, Event.Silent, Encapsulation.structsRespected]

omit P π A auth s fr rest g hs hst hhe f info code o in
@[simp high] theorem all_const_true {α : Type} (l : List α) : l.all (fun _ => true) = true := by
  simp [List.all_eq_true]

/-- Proves `Plain4` of an event list built from D4's records, splitting the
matches that choose them. -/
macro "plain4" : tactic =>
  `(tactic| (simp only [Plain4, List.all_append, List.all_cons, List.all_nil,
      Bool.and_true, Bool.true_and, List.all_map, Function.comp_def]
             repeat' split
             all_goals simp [plainEvent, List.all_map, Function.comp_def]))

/-- A step that changes only the current frame, whose events are all silent. -/
theorem frame_annot0 {s₂ : State} {fr' : Frame} {g' : ShadowFrame} {f' : Nat} {instr : Bytecode}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hshadow : ∃ pre, Shadow.exec P info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ =
      some (.running ⟨g' :: hs, hst, hhe, f'⟩, pre) ∧ Plain4 pre)
    (ho : o = .running { s₂ with frames := fr' :: rest }) (h2st : s₂.storage = s.storage)
    (h2he : s₂.held = s.held) (h2nf : s₂.nextFrame = s.nextFrame) (hid : fr'.id = fr.id)
    (hG : FrameGhost σ P π A auth g fr f g' fr' f' []) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨pre, hex, hpre⟩ := hshadow
  exact frame_case' (pre := pre) (post := []) hp hsf (by simpa using hex) (plain4_ok hpre) (by simp)
    ho h2st h2he h2nf hid hG

theorem equality_annot {positive : Bool} {instr : Bytecode}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (hshadow : ∀ v st, 2 ≤ g.stack.length → ∃ pre,
      Shadow.exec P info.moduleId instr s fr
        (.running { s with frames := { fr with stack := v :: st, pc := fr.pc + 1 } :: rest })
        ⟨g :: hs, hst, hhe, f⟩ =
        some (replaced g hs hst hhe (.instruction info.moduleId instr) v f 2,
          pre ++ (IdTree.fresh (.instruction info.moduleId instr) v f).2.2 ++ []) ∧ Plain4 pre)
    (h : stepOf (equality P s fr positive >>= fun fr' => next s rest fr') = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨fr', hx, h⟩ := done_bind h hcont equality_ends
  obtain ⟨a, b, st, hst', ha, hb, c, hn, rfl⟩ := equality_ok hx
  have ho := done_next h
  subst ho
  have hg := (pre_head hp hsf).1
  have hlen := fitsAll_length (frame_stack_fits hg)
  obtain ⟨pre, hex, hpre⟩ := hshadow (.bool c) (fr.stack.drop 2) (by omega)
  refine replace_annot hp hsf (fun _ => hex) (plain4_ok hpre) (by simp) hn ?_
    (primitive_plain rfl) rfl
  intro w hw
  simp only [hst', List.take_succ_cons, List.take_zero, List.mem_cons, List.not_mem_nil,
    or_false] at hw
  rcases hw with rfl | rfl
  · exact hb
  · exact ha

/-- A step that changes only the current frame, with silent events before its
ghost facts' events. -/
theorem frame_annot {s₂ : State} {fr' : Frame} {g' : ShadowFrame} {f' : Nat} {instr : Bytecode}
    {evs : List Event} (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hshadow : ∃ pre, Shadow.exec P info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ =
      some (.running ⟨g' :: hs, hst, hhe, f'⟩, pre ++ evs ++ []) ∧ Plain4 pre)
    (ho : o = .running { s₂ with frames := fr' :: rest }) (h2st : s₂.storage = s.storage)
    (h2he : s₂.held = s.held) (h2nf : s₂.nextFrame = s.nextFrame) (hid : fr'.id = fr.id)
    (hG : FrameGhost σ P π A auth g fr f g' fr' f' evs) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨pre, hex, hpre⟩ := hshadow
  exact frame_case' hp hsf hex (plain4_ok hpre) (by simp) ho h2st h2he h2nf hid hG

theorem copyLoc_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) {i : LocalIndex}
    (h : stepOf (Step.exec P s fr rest info code (.copyLoc i)) = .done o) :
    Annotated σ P π A auth info.moduleId (.copyLoc i) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨slot, hslot, h⟩ := done_bind h hcont (ends_need _ _)
  have hslot' := need_ok hslot
  obtain ⟨hi, hfi⟩ := Array.getElem?_eq_some_iff.1 hslot'
  cases slot with
  | none => exact (done_wrong h hcont).elim
  | some v =>
    dsimp only at h
    split at h
    · next hc =>
      obtain ⟨t, ht, hG⟩ := copy_ghost (P := P) (π := π) (A := A) (auth := auth) (f := f)
        (pre_head hp hsf).1 hi hfi hc
      refine frame_annot0 hp hsf ?_ (done_next h) rfl rfl rfl rfl hG
      refine ⟨_, by simp [Shadow.exec, ht, hslot']; rfl, ?_⟩
      plain4
    · exact (done_wrong h hcont).elim

theorem moveLoc_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) {i : LocalIndex}
    (h : stepOf (Step.exec P s fr rest info code (.moveLoc i)) = .done o) :
    Annotated σ P π A auth info.moduleId (.moveLoc i) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨slot, hslot, h⟩ := done_bind h hcont (ends_need _ _)
  have hslot' := need_ok hslot
  obtain ⟨hi, hfi⟩ := Array.getElem?_eq_some_iff.1 hslot'
  cases slot with
  | none => exact (done_wrong h hcont).elim
  | some v =>
    obtain ⟨t, ht, hG⟩ := move_ghost (P := P) (π := π) (A := A) (auth := auth) (f := f)
      (pre_head hp hsf).1 hi hfi
    refine frame_annot0 hp hsf ?_ (done_next h) rfl rfl rfl rfl hG
    refine ⟨_, by simp [Shadow.exec, ht, hslot']; rfl, ?_⟩
    plain4

theorem stLoc_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) {i : LocalIndex}
    (h : stepOf (Step.exec P s fr rest info code (.stLoc i)) = .done o) :
    Annotated σ P π A auth info.moduleId (.stLoc i) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h hcont pop_ends
  obtain ⟨st, hst', rfl⟩ := pop_ok hpop
  dsimp only at h
  obtain ⟨tys, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨t, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨slot, hslot, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨hi, hfi⟩ := Array.getElem?_eq_some_iff.1 (need_ok hslot)
  have hex : ∀ tt ts, g.stack = tt :: ts → ∃ pre, Shadow.exec P info.moduleId (.stLoc i) s fr o
      ⟨g :: hs, hst, hhe, f⟩ = some (.running ⟨{ g with locals := g.locals.setIfInBounds i (some tt), stack := ts } :: hs, hst, hhe, f⟩, pre) ∧ Plain4 pre := by
    intro tt ts hgst
    refine ⟨_, by simp [Shadow.exec, hgst, hst']; rfl, ?_⟩
    plain4
  split at h
  · exact (done_wrong h hcont).elim
  · cases slot with
    | none =>
      obtain ⟨tt, ts, hgst, hG⟩ := stLoc_ghost (P := P) (π := π) (A := A) (auth := auth) (f := f)
        hg hi hst' (fun old hold => by simp_all)
      exact frame_annot0 hp hsf (hex tt ts hgst) (done_next h) rfl rfl rfl rfl hG
    | some old =>
      dsimp only at h
      split at h
      · exact (done_wrong h hcont).elim
      · next hd =>
        simp only [Bool.not_eq_true', Bool.not_eq_false] at hd
        obtain ⟨tt, ts, hgst, hG⟩ := stLoc_ghost (P := P) (π := π) (A := A) (auth := auth) (f := f)
          hg hi hst' (fun old' hold' => by
            have : old' = old := by simp_all
            rw [this]; exact hd)
        exact frame_annot0 hp hsf (hex tt ts hgst) (done_next h) rfl rfl rfl rfl hG

theorem call_annot_of {s₂ : State} {n : Nat} {instr : Bytecode}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest) (hn : n ≤ fr.stack.length)
    (callee : Frame) (m : Nat)
    (hloc : callee.locals = ((fr.stack.take n).reverse.map some ++ List.replicate m none).toArray)
    (hstk : callee.stack = []) (hid : callee.id = s.nextFrame)
    (h2f : s₂.frames = callee :: { fr with stack := fr.stack.drop n } :: rest)
    (h2st : s₂.storage = s.storage) (h2he : s₂.held = s.held)
    (h2nf : s₂.nextFrame = s.nextFrame + 1)
    (hex : ∃ evs, Shadow.exec P info.moduleId instr s fr (.running s₂) ⟨g :: hs, hst, hhe, f⟩ =
      some (.running ⟨{ locals := ((g.stack.take n).reverse.map some ++
          List.replicate (callee.locals.size - n) none).toArray, stack := [] } ::
        { g with stack := g.stack.drop n } :: hs, hst, hhe, f⟩, evs) ∧ Plain4 evs) :
    Annotated σ P π A auth info.moduleId instr s fr (.running s₂) ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨evs, hex, hevs⟩ := hex
  exact ⟨_, _, hex, call_ghost (π := π) (A := A) (auth := auth) hp hsf rfl hn callee m hloc hstk hid
    h2f h2st h2he h2nf (plain4_ok hevs)⟩

theorem call_annot {instr : Bytecode} (hi : (∃ hd, instr = .call hd) ∨ ∃ i, instr = .callGeneric i)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) {hd : FunctionHandleIndex} {tys : List RuntimeType}
    (h : stepOf (Step.call P s fr rest info hd tys) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  have hlen := fitsAll_length (frame_stack_fits hg)
  unfold Step.call at h
  obtain ⟨callee, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨calleeInfo, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨fh, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨paramTys, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨retTys, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨hps, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨hrs, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨⟨args, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
  obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_native h hcont).elim
  obtain ⟨localTys, -, h⟩ := done_bind h hcont (ends_need _ _)
  have hk : fr.stack.length - (fr.stack.drop paramTys.length).length = paramTys.length := by
    simp; omega
  split at h <;> split at h
  all_goals first
    | exact (done_fail h hcont).elim
    | skip
  all_goals
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
    subst h
    refine call_annot_of hp hsf hn _ _ rfl rfl ?_ rfl ?_ ?_ ?_
      ⟨_, by
        rcases hi with ⟨_, rfl⟩ | ⟨_, rfl⟩ <;>
          (simp only [Shadow.exec, hk, guard, hlen, if_pos hn]; rfl), by plain4⟩
    all_goals split <;> rfl

theorem locals_drop {P : Program} {fr : Frame}
    (h : (fr.locals.any fun slot => match slot with
      | some v => !(v.abilities P).drop
      | none => false) = false) :
    ∀ v, some v ∈ fr.locals.toList → (v.abilities P).drop = true := by
  intro v hv
  rw [← Array.any_toList, List.any_eq_false] at h
  simpa using h _ hv

theorem ret_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (hret : RetOk P s o)
    (h : stepOf (Step.exec P s fr rest info code .ret) = .done o) :
    Annotated σ P π A auth info.moduleId .ret s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  unfold Step.exec Step.ret at h
  obtain ⟨retTys, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  next hany =>
  have hl := locals_drop (Bool.not_eq_true _ ▸ hany)
  cases rest with
  | nil =>
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
    have hhs : hs = [] := by
      have := (pre_head hp hsf).2
      cases hs with
      | nil => rfl
      | cons => simp [framesFit] at this
    subst hhs
    obtain ⟨evs, hex, hevs⟩ : ∃ evs, Shadow.exec P info.moduleId .ret s fr o ⟨[g], hst, hhe, f⟩ =
        some (.returned g.stack.reverse hst, evs) ∧ Plain4 evs :=
      ⟨_, by rw [← h]; rfl, by plain4⟩
    subst h
    exact ⟨_, _, hex, ret_last_ghost (π := π) (A := A) (auth := auth) hp hsf rfl (hret _ _ rfl) hl
      (by split <;> (try split) <;> rfl) (plain4_ok hevs)⟩
  | cons caller rest' =>
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
    obtain ⟨cg, hs', hhs⟩ : ∃ cg hs', hs = cg :: hs' := by
      have := (pre_head hp hsf).2
      cases hs with
      | nil => simp [framesFit] at this
      | cons cg hs' => exact ⟨cg, hs', rfl⟩
    subst hhs
    obtain ⟨evs, hex, hevs⟩ : ∃ evs, Shadow.exec P info.moduleId .ret s fr o
        ⟨g :: cg :: hs', hst, hhe, f⟩ =
          some (.running ⟨{ cg with stack := g.stack ++ cg.stack } :: hs', hst, hhe, f⟩, evs) ∧
          Plain4 evs :=
      ⟨_, by rw [← h]; rfl, by plain4⟩
    subst h
    exact ⟨_, _, hex, ret_caller_ghost (π := π) (A := A) (auth := auth) hp hsf rfl hl
      (by split <;> (try split) <;> rfl) (by split <;> (try split) <;> rfl)
      (by split <;> (try split) <;> rfl) (plain4_ok hevs)⟩

/-- **`Ret` is annotated**, whatever the values the transaction holds: the
annotation exists and fits the outcome. -/
theorem ret_exists (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (Step.exec P s fr rest info code .ret) = .done o) :
    ∃ post evs, Shadow.exec P info.moduleId .ret s fr o ⟨g :: hs, hst, hhe, f⟩ =
      some (post, evs) ∧ post.fits o = true := by
  unfold Step.exec Step.ret at h
  obtain ⟨retTys, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  next hany =>
  have hl := locals_drop (Bool.not_eq_true _ ▸ hany)
  cases rest with
  | nil =>
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
    have hhs : hs = [] := by
      have := (pre_head hp hsf).2
      cases hs with
      | nil => rfl
      | cons => simp [framesFit] at this
    subst hhs
    obtain ⟨evs, hex⟩ : ∃ evs, Shadow.exec P info.moduleId .ret s fr o ⟨[g], hst, hhe, f⟩ =
        some (.returned g.stack.reverse hst, evs) := ⟨_, by rw [← h]; rfl⟩
    subst h
    refine ⟨_, _, hex, ?_⟩
    have hfit := hp.fits
    rw [shadow_fits, hsf, Bool.and_eq_true] at hfit
    obtain ⟨hfr, hext⟩ := hfit
    rw [framesFit_cons, Bool.and_eq_true] at hfr
    simp only [extFit, Bool.and_eq_true] at hext
    simp only [ShadowPost.fits, Bool.and_eq_true]
    refine ⟨fitsAll_reverse (frame_stack_fits hfr.1), ?_⟩
    split <;> (try split) <;> exact hext.1
  | cons caller rest' =>
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
    obtain ⟨cg, hs', hhs⟩ : ∃ cg hs', hs = cg :: hs' := by
      have := (pre_head hp hsf).2
      cases hs with
      | nil => simp [framesFit] at this
      | cons cg hs' => exact ⟨cg, hs', rfl⟩
    subst hhs
    obtain ⟨evs, hex, hevs⟩ : ∃ evs, Shadow.exec P info.moduleId .ret s fr o
        ⟨g :: cg :: hs', hst, hhe, f⟩ =
          some (.running ⟨{ cg with stack := g.stack ++ cg.stack } :: hs', hst, hhe, f⟩, evs) ∧
          Plain4 evs :=
      ⟨_, by rw [← h]; rfl, by plain4⟩
    subst h
    exact ⟨_, _, hex, (ret_caller_ghost (π := {}) (A := []) (auth := []) hp hsf rfl hl
      (by split <;> (try split) <;> rfl) (by split <;> (try split) <;> rfl)
      (by split <;> (try split) <;> rfl) (plain4_ok hevs)).fits⟩

theorem pack_annot' {c : Value} {n : Nat} {instr : Bytecode}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest) (hn : n ≤ fr.stack.length)
    (hch : c.children = (fr.stack.take n).reverse) (hcr : ∀ r, c ≠ .ref r)
    (hresp : Encapsulation.structsRespected π A (.created f c (.instruction info.moduleId instr)) = true ∧
      Encapsulation.signersRespected π auth (.created f c (.instruction info.moduleId instr)) = true)
    (hclo : Encapsulation.closuresRespected P (.created f c (.instruction info.moduleId instr)) = true)
    (hshadow : Shadow.exec P info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ = (do
      let v ← Shadow.topAfter o
      let n := v.children.length
      guard (n ≤ g.stack.length)
      pure (ShadowPost.running
        ⟨{ g with stack := .node f (g.stack.take n).reverse :: g.stack.drop n } :: hs, hst, hhe, f + 1⟩,
        [Event.created f v (Creator.instruction info.moduleId instr)])))
    (ho : o = .running { s with frames := { fr with stack := c :: fr.stack.drop n, pc := fr.pc + 1 } :: rest }) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  have hlen := fitsAll_length (frame_stack_fits hg)
  have hcl : c.children.length = n := by simp [hch]; omega
  refine frame_case hp hsf ?_ ho rfl rfl rfl rfl (pack_ghost hg hch hcr _ hresp hclo)
  rw [hshadow, ho]
  simp [guard, Shadow.topAfter, hcl, hlen, hn]

theorem unpack_annot' {c : Value} {st : List Value} {instr : Bytecode} (cons : Consumer)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest) (hst' : fr.stack = c :: st)
    (hlegit : (c.type.abilities P).drop = true ∨ cons.Legitimate π c = true)
    (hresp : ∀ i, Encapsulation.structsRespected π A (.consumed i c cons) = true)
    (hshadow : Shadow.exec P info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ = (do
      let t :: st := g.stack | none
      let v ← fr.stack.head?
      pure (ShadowPost.running ⟨{ g with stack := t.children.reverse ++ st } :: hs, hst, hhe, f⟩,
        [Event.consumed t.id v cons])))
    (ho : o = .running { s with frames := { fr with stack := c.children.reverse ++ st, pc := fr.pc + 1 } :: rest }) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨t, ts, hgst, hG⟩ := unpack_ghost (P := P) (auth := auth) (f := f) (pre_head hp hsf).1 hst'
    cons hlegit hresp
  refine frame_case hp hsf ?_ ho rfl rfl rfl rfl hG
  rw [hshadow]
  simp [hgst, hst']

theorem localStruct_ends {P : Program} {mi : Nat} {d : StructDefinitionIndex}
    {tys : List RuntimeType} : Ends (localStruct P mi d tys) := by
  unfold localStruct
  refine ends_bind (ends_need _ _) fun _ _ => ends_bind (ends_need _ _) fun _ _ =>
    ends_bind (ends_need _ _) fun _ _ => ?_
  split
  · exact ends_pure _
  · exact ends_wrong _

/-- The type arguments of a generic form end the step or give a list. -/
theorem genericArgs_ends {m : CompiledModule} {tys : List RuntimeType} {sig : SignatureIndex} :
    Ends (genericArgs m tys sig) := by
  unfold genericArgs
  refine ends_bind (ends_need _ _) fun _ _ => ends_bind (ends_need _ _) fun l _ => ?_
  rcases l with _ | ⟨t, ts⟩
  · exact ends_wrong _
  · exact ends_pure _

theorem genericArgs_ok {m : CompiledModule} {tys : List RuntimeType} {sig : SignatureIndex}
    {l : List RuntimeType} (h : genericArgs m tys sig = .ok l) : m.typeArgsWith tys sig = some l := by
  unfold genericArgs at h
  rw [bind_ok'] at h
  obtain ⟨toks, htoks, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨l', hl', h⟩ := h
  rcases l' with _ | ⟨t, ts⟩
  · simp [wrong] at h
  · simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    simp [CompiledModule.typeArgsWith, need_ok htoks, need_ok hl']

theorem elemType_ends {m : CompiledModule} {sig : SignatureIndex} {tys : List RuntimeType} :
    Ends (elemType m sig tys) := by
  unfold elemType
  refine ends_bind (ends_need _ _) fun l _ => ?_
  rcases l with _ | ⟨t, _ | ⟨t', l⟩⟩
  · exact ends_wrong _
  · exact ends_need _ _
  · exact ends_wrong _

/-- The struct type of a struct operation of the current module: the type
its definition defines, at the instruction's type arguments. -/
theorem localStruct_defined {P : Program} {mi : Nat} {d : StructDefinitionIndex}
    {tys : List RuntimeType} {sid : StructId} {sinfo : StructInfo}
    (h : localStruct P mi d tys = .ok (sid, sinfo)) :
    ∃ sid₀, P.definedStruct? mi d = some sid₀ ∧ sid = { sid₀ with typeArgs := tys } := by
  unfold localStruct at h
  rw [bind_ok'] at h
  obtain ⟨_, -, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨sid', h1, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨info', -, h⟩ := h
  split at h
  · simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    exact ⟨sid', need_ok h1, h.1.symm⟩
  · simp [wrong] at h

/-- A struct type defined by the current function's module belongs to that
module. -/
theorem definedStruct_module {P : Program} {fref : FunctionRef} {d : StructDefinitionIndex}
    {sid : StructId} {info : FunctionInfo} (hd : P.definedStruct? fref.module d = some sid)
    (hf : P.function? fref = some info) : sid.module = info.moduleId := by
  unfold Program.definedStruct? Program.struct? at hd
  unfold Program.function? at hf
  cases hm : P.modules[fref.module]? with
  | none => simp [hm] at hf
  | some m =>
    cases hid : m.selfId with
    | none => simp [hm, hid] at hf
    | some id =>
      simp only [hm, hid, Option.bind_eq_bind, Option.bind_some, Option.bind_eq_some_iff,
        Option.pure_def, Option.some.injEq] at hd hf
      obtain ⟨_, _, _, _, rfl⟩ := hf
      obtain ⟨_, ⟨_, _, _, _, rfl⟩, _, _, hs⟩ := hd
      split at hs
      · simp only [Option.some.injEq] at hs
        rw [← hs]
      · simp at hs

/-- A struct type of a struct operation belongs to the current function's
module. -/
theorem localStruct_module {P : Program} {fref : FunctionRef} {d : StructDefinitionIndex}
    {tys : List RuntimeType} {sid : StructId} {sinfo : StructInfo} {info : FunctionInfo}
    (h : localStruct P fref.module d tys = .ok (sid, sinfo)) (hf : P.function? fref = some info) :
    sid.module = info.moduleId := by
  obtain ⟨sid₀, hd, rfl⟩ := localStruct_defined h
  exact definedStruct_module (sid := sid₀) hd hf

theorem pack_annot {instr : Bytecode} {d : StructDefinitionIndex} {tys : List RuntimeType}
    (hi : instr = .pack d ∨ ∃ i, instr = .packGeneric i)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (hinfo : P.function? fr.function = some info)
    (h : stepOf (packStruct P s fr rest d tys) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  have hmod := localStruct_module hls hinfo
  dsimp only at h
  split at h
  · obtain ⟨tys, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
    obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    · refine pack_annot' (c := packedValue sid _) hp hsf hn (packedValue_children _ _)
        (packedValue_ne_ref _ _)
        (by unfold packedValue; split <;> simp [Encapsulation.structsRespected,
          Encapsulation.signersRespected, Creator.MayCreateStruct, hmod, Value.type])
        (by unfold packedValue; split <;> simp [Encapsulation.closuresRespected])
        (by rcases hi with rfl | ⟨_, rfl⟩ <;> rfl) ?_
      have := done_next h
      unfold packedValue
      exact this
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

/-- `PackClosure` and `PackClosureGeneric` move the captured values into the
closure, which the instruction creates. -/
theorem packClosure_annot {instr : Bytecode} {hd : FunctionHandleIndex} {mask : BitVec 64}
    {tys : List RuntimeType}
    (hi : instr = .packClosure hd mask ∨ ∃ i, instr = .packClosureGeneric i mask)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (Step.packClosure P s fr rest info hd mask tys) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  unfold Step.packClosure at h
  obtain ⟨target, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  next hvis =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hvis
  obtain ⟨targetInfo, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨fh, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨paramTys, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨retTys, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨hps, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨hrs, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
  obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  · exact pack_annot' hp hsf hn rfl (by simp)
      (by simp [Encapsulation.structsRespected, Encapsulation.signersRespected, Value.type])
      (by
        simp only [Encapsulation.closuresRespected, Encapsulation.closureCreatorOk]
        exact hvis)
      (by rcases hi with rfl | ⟨_, rfl⟩ <;> rfl) (done_next h)

theorem packVariant_annot {instr : Bytecode} {hv : StructVariantHandleIndex}
    {tys : List RuntimeType} (hi : instr = .packVariant hv ∨ ∃ i, instr = .packVariantGeneric i)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (hinfo : P.function? fr.function = some info)
    (h : stepOf (packEnum P s fr rest info hv tys) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨vh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  have hmod := localStruct_module hls hinfo
  dsimp only at h
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
    obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
    dsimp only at h
    split at h
    · exact (done_wrong h hcont).elim
    · exact pack_annot' hp hsf hn rfl (by simp)
        (by simp [Encapsulation.structsRespected, Encapsulation.signersRespected, Value.type,
          Creator.MayCreateStruct, hmod]) (by simp [Encapsulation.closuresRespected])
        (by rcases hi with rfl | ⟨_, rfl⟩ <;> rfl)
        (done_next h)
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem vecPack_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) {sig : SignatureIndex} {n : Nat}
    (h : stepOf (Step.exec P s fr rest info code (.vecPack sig n)) = .done o) :
    Annotated σ P π A auth info.moduleId (.vecPack sig n) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨t, -, h⟩ := done_bind h hcont elemType_ends
  obtain ⟨⟨vals, fr₁⟩, hpop, h⟩ := done_bind h hcont popN_ends
  obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  · exact pack_annot' hp hsf hn rfl (by simp)
      (by simp [Encapsulation.structsRespected, Encapsulation.signersRespected, Value.type])
      (by simp [Encapsulation.closuresRespected]) rfl (done_next h)

theorem unpack_annot {instr : Bytecode} {d : StructDefinitionIndex} {tys : List RuntimeType}
    {code : CodeUnit}
    (hi : instr = .unpack d ∨ ∃ i x, instr = .unpackGeneric i ∧
      info.module.structDefInstantiations[i]? = some x ∧ x.def_ = d)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (hinfo : P.function? fr.function = some info)
    (hcode : info.definition.code = some code) (hin : instr ∈ code.code.toList)
    (h : stepOf (unpackStruct P s fr rest d tys) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  have hmod := localStruct_module hls hinfo
  dsimp only at h
  have hinstr : instr = .unpack d ∨ ∃ i, instr = .unpackGeneric i := by
    rcases hi with h | ⟨i, -, h, -⟩
    · exact Or.inl h
    · exact Or.inr ⟨i, h⟩
  split at h
  · obtain ⟨⟨v, fr₁⟩, hpp, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st, hst', rfl⟩ := pop_ok hpp
    dsimp only at h
    split at h
    · next sid' fields =>
      split at h
      · exact (done_wrong h hcont).elim
      · next hne =>
        have hmod' : sid'.module = info.moduleId := by
          rw [show sid' = sid by simpa using hne]; exact hmod
        exact unpack_annot' (.unpack info.moduleId) hp hsf hst'
          (Or.inr (by simp [Consumer.Legitimate, hmod']))
          (fun i => by
            simp [Encapsulation.structsRespected, Value.type, Consumer.MayTakeApart, hmod'])
          (by rcases hinstr with rfl | ⟨_, rfl⟩ <;> rfl) (done_next h)
    · next sid' fields ks es =>
      split at h
      · exact (done_wrong h hcont).elim
      · next hne =>
        have hsid : sid' = sid := by simpa using hne
        have hmod' : sid'.module = info.moduleId := by rw [hsid]; exact hmod
        -- An unpacked table has no entries.
        have hes : es = [] := by
          cases es with
          | nil => rfl
          | cons e es =>
            exfalso
            have hcomp : Composes P (.table sid' fields ks (e :: es)) :=
              hp.composes fr (by rw [hsf]; exact List.mem_cons_self ..) _
                (by rw [hst']; exact List.mem_cons_self ..)
            obtain ⟨htab, htt⟩ := hcomp (List.cons_ne_nil _ _)
            obtain ⟨sid₀, hd, rfl⟩ := localStruct_defined hls
            have ht₀ : sid₀.isTable = true := by
              rw [hsid] at htt; simpa [StructIdOf.isTable] using htt
            rw [unpack_noEntries hinfo hcode hin hi hd ht₀] at htab
            cases htab
        subst hes
        have hch : (Value.table sid' fields ks []).children = fields := by simp [Value.children]
        refine unpack_annot' (.unpack info.moduleId) hp hsf hst'
          (Or.inr (by simp [Consumer.Legitimate, hmod']))
          (fun i => by
            simp [Encapsulation.structsRespected, Value.type, Consumer.MayTakeApart, hmod'])
          (by rcases hinstr with rfl | ⟨_, rfl⟩ <;> rfl) ?_
        rw [hch]
        exact done_next h
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem unpackVariant_annot {instr : Bytecode} {hv : StructVariantHandleIndex}
    {tys : List RuntimeType} (hi : instr = .unpackVariant hv ∨ ∃ i, instr = .unpackVariantGeneric i)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (hinfo : P.function? fr.function = some info)
    (h : stepOf (unpackEnum P s fr rest info hv tys) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨vh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  have hmod := localStruct_module hls hinfo
  dsimp only at h
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨v, fr₁⟩, hpp, h⟩ := done_bind h hcont pop_ends
    obtain ⟨st, hst', rfl⟩ := pop_ok hpp
    dsimp only at h
    split at h
    · next sid' tag fields =>
      split at h
      · exact (done_wrong h hcont).elim
      · next hne =>
        split at h
        · exact (done_fail h hcont).elim
        · have hmod' : sid'.module = info.moduleId := by
            rw [show sid' = sid by simpa using hne]; exact hmod
          exact unpack_annot' (.unpack info.moduleId) hp hsf hst'
            (Or.inr (by simp [Consumer.Legitimate, hmod']))
            (fun i => by
              simp [Encapsulation.structsRespected, Value.type, Consumer.MayTakeApart, hmod'])
            (by rcases hi with rfl | ⟨_, rfl⟩ <;> rfl) (done_next h)
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem vecUnpack_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) {sig : SignatureIndex} {n : Nat}
    (h : stepOf (Step.exec P s fr rest info code (.vecUnpack sig n)) = .done o) :
    Annotated σ P π A auth info.moduleId (.vecUnpack sig n) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨t, -, h⟩ := done_bind h hcont elemType_ends
  obtain ⟨⟨v, fr₁⟩, hpp, h⟩ := done_bind h hcont pop_ends
  obtain ⟨st, hst', rfl⟩ := pop_ok hpp
  dsimp only at h
  split at h
  · split at h
    · exact (done_wrong h hcont).elim
    · split at h
      · exact (done_fail h hcont).elim
      · exact unpack_annot' .vecUnpack hp hsf hst' (Or.inr (by simp [Consumer.Legitimate]))
          (fun i => by simp [Encapsulation.structsRespected, Value.type]) rfl (done_next h)
  · exact (done_wrong h hcont).elim

theorem abort_absurd (hcont : o.continues = true)
    (h : stepOf (Step.exec P s fr rest info code .abort) = .done o) : False := by
  obtain ⟨⟨v, fr₁⟩, -, h⟩ := done_bind h hcont pop_ends
  dsimp only at h
  split at h
  · exact done_fail h hcont
  · exact done_wrong h hcont

theorem abortMsg_absurd (hcont : o.continues = true)
    (h : stepOf (Step.exec P s fr rest info code .abortMsg) = .done o) : False := by
  obtain ⟨⟨msg, fr₁⟩, -, h⟩ := done_bind h hcont pop_ends
  dsimp only at h
  obtain ⟨⟨c, fr₂⟩, -, h⟩ := done_bind h hcont pop_ends
  dsimp only at h
  split at h
  · obtain ⟨bytes, -, h⟩ := done_bind h hcont (ends_need _ _)
    split at h
    · exact done_fail h hcont
    · exact done_fail h hcont
  · exact done_wrong h hcont

/-! ## References -/

theorem done_finish {s₁ : State} {fr₁ : Frame} {rest₁ : List Frame} {vals : List Value}
    (hs : s₁.frames = fr₁ :: rest₁) (h : stepOf (finish s₁ vals) = .done o) :
    o = .running { s₁ with frames := advance (pushAll fr₁ vals) :: rest₁ } := by
  unfold finish at h
  rw [hs] at h
  simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
  exact h.symm

theorem globalType_ends {mi : Nat} {d : StructDefinitionIndex} {tys : List RuntimeType} :
    Ends (globalType P mi d tys) := by
  unfold globalType
  refine ends_bind localStruct_ends fun p _ => ?_
  rcases p with ⟨_, _⟩
  dsimp only
  split <;> first | trivial | rfl

theorem globalType_ok {mi : Nat} {d : StructDefinitionIndex} {tys : List RuntimeType}
    {sid : StructId} (h : globalType P mi d tys = .ok sid) :
    ∃ sid₀, P.definedStruct? mi d = some sid₀ ∧ sid = { sid₀ with typeArgs := tys } := by
  unfold globalType at h
  rw [bind_ok'] at h
  obtain ⟨⟨sid', sinfo⟩, hls, h⟩ := h
  dsimp only at h
  split at h
  · simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact localStruct_defined hls
  · simp [wrong] at h

theorem globalType_module {fref : FunctionRef} {d : StructDefinitionIndex}
    {tys : List RuntimeType} {sid : StructId} {info : FunctionInfo}
    (h : globalType P fref.module d tys = .ok sid) (hf : P.function? fref = some info) :
    sid.module = info.moduleId := by
  obtain ⟨sid₀, hd, rfl⟩ := globalType_ok h
  exact definedStruct_module (sid := sid₀) hd hf

theorem ref_drop' {r : Reference} : ((Value.ref r).abilities P).drop = true := by
  simp [Value.abilities, RuntimeType.abilities]

/-- The borrows and `FreezeRef`, with silent events around the borrow. -/
theorem borrow_annot {s₂ : State} {n : Nat} {r : Reference} {parent : Option Nat}
    {pre post : List Event} {instr : Bytecode}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hshadow : n ≤ g.stack.length → Shadow.exec P info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ =
      some (.running ⟨{ g with stack := .node f [] :: g.stack.drop n } :: hs, hst, hhe, f + 1⟩,
        pre ++ [.borrowed f r parent] ++ post))
    (hpre : ∀ e ∈ pre, e.Silent = true ∧ Encapsulation.structsRespected π A e = true)
    (hpost : ∀ e ∈ post, e.Silent = true ∧ Encapsulation.structsRespected π A e = true)
    (hn : n ≤ fr.stack.length) (hdrop : ∀ w ∈ fr.stack.take n, (w.abilities P).drop = true)
    (hok : (Value.ref r).key.Ok σ)
    (ho : o = .running { s₂ with frames := { fr with stack := .ref r :: fr.stack.drop n, pc := fr.pc + 1 } :: rest })
    (h2st : s₂.storage = s.storage) (h2he : s₂.held = s.held) (h2nf : s₂.nextFrame = s.nextFrame) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  have hlen := fitsAll_length (frame_stack_fits hg)
  exact frame_case' hp hsf (hshadow (by omega)) hpre hpost ho h2st h2he h2nf rfl
    (borrow_ghost hg hdrop hok parent)

theorem readRef_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (Step.exec P s fr rest info code .readRef) = .done o) :
    Annotated σ P π A auth info.moduleId .readRef s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRef_ends
  obtain ⟨st, hst', rfl⟩ := popRef_ok hpr
  dsimp only at h
  obtain ⟨v, hv, h⟩ := done_bind h hcont deref_ends
  split at h
  · exact (done_wrong h hcont).elim
  · next hc =>
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hc
    have ho := done_finish rfl h
    have hv' : s.read? r.location = some v := by
      have := deref?_read (deref_ok hv)
      rwa [read_eq (s' := { s with frames := { fr with stack := st } :: rest }) (s := s)
        (by simp [hsf]) rfl rfl] at this
    have hfa := frame_stack_fits hg
    rw [hst'] at hfa
    obtain ⟨t, ts₀, hgst₀, -, -⟩ := fitsAll_cons_right hfa
    obtain ⟨t', ts, tv, hgst, htv, hG⟩ := readRef_ghost (π := π) (A := A) (auth := auth)
      (evs := [.accessed .read r.location (some t.id), .used t.id r]) hp hsf rfl hst' hv' hc rfl rfl rfl
      (plain4_ok (by plain4))
    rw [hgst₀, List.cons.injEq] at hgst
    obtain ⟨rfl, rfl⟩ := hgst
    subst ho
    refine ⟨_, _, ?_, hG⟩
    simp [Shadow.exec, hgst₀, hst', htv]

theorem freezeRef_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (Step.exec P s fr rest info code .freezeRef) = .done o) :
    Annotated σ P π A auth info.moduleId .freezeRef s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRef_ends
  obtain ⟨st, hst', rfl⟩ := popRef_ok hpr
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  · have ho := done_next h
    obtain ⟨hok, t, ht, -⟩ := stack_key_ok hp hsf (k := 0) (r := r) (by simp [hst'])
    rw [show st = fr.stack.drop 1 by simp [hst']] at ho
    refine borrow_annot (pre := [.used t.id r]) (post := []) (parent := some t.id) hp hsf
      (fun hn => ?_) (plain4_ok (by plain4)) (by simp) (by simp [hst'])
      (by simp [hst', ref_drop']) ?_ ho rfl rfl rfl
    · rw [ho]
      simp [Shadow.exec, guard, hn, hst', ht, Shadow.topAfter, advance, push]
    · simp only [Value.key, Key.Ok] at hok ⊢
      exact ⟨fun _ _ => trivial, hok.2⟩

theorem borrowLoc_annot {mutable : Bool} {i : LocalIndex} {instr : Bytecode}
    (hi : instr = .mutBorrowLoc i ∧ mutable = true ∨ instr = .immBorrowLoc i ∧ mutable = false)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (borrowLoc P s fr rest mutable i) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  unfold borrowLoc at h
  obtain ⟨slot, -, h⟩ := done_bind h hcont (ends_need _ _)
  cases slot with
  | none => exact (done_wrong h hcont).elim
  | some v =>
    dsimp only at h
    obtain ⟨tys, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨t, -, h⟩ := done_bind h hcont (ends_need _ _)
    split at h
    · exact (done_wrong h hcont).elim
    · have ho := done_next h
      refine borrow_annot (n := 0) (r := ⟨mutable, t, ⟨.local_ fr.id i, []⟩, s.invalidations.length⟩)
        (pre := []) (post := []) (parent := none)
        hp hsf (fun _ => ?_) (by simp) (by simp)
        (Nat.zero_le _) (by simp) ?_ (by simpa [advance, push, borrow] using ho) rfl rfl rfl
      · rw [ho]
        rcases hi with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩ <;>
          simp [Shadow.exec, guard, Shadow.topAfter, advance, push, borrow]
      · simp [Value.key, Key.Ok, Root.isHeld]

theorem borrowGlobal_annot {mutable : Bool} {d : StructDefinitionIndex} {instr : Bytecode}
    {tys : List RuntimeType}
    (hi : (instr = .mutBorrowGlobal d ∨ ∃ i, instr = .mutBorrowGlobalGeneric i) ∧ mutable = true ∨
      (instr = .immBorrowGlobal d ∨ ∃ i, instr = .immBorrowGlobalGeneric i) ∧ mutable = false)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (hinfo : P.function? fr.function = some info)
    (h : stepOf (borrowGlobal P s fr rest mutable d tys) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  have hlen := fitsAll_length (frame_stack_fits hg)
  unfold borrowGlobal at h
  obtain ⟨sid, hsid, h⟩ := done_bind h hcont globalType_ends
  have hmod := globalType_module hsid hinfo
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h hcont popAddress_ends
  obtain ⟨st, hst', rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h hcont unlocked_ends
  split at h
  · have ho := done_next h
    rw [show st = fr.stack.drop 1 by simp [hst']] at ho
    refine borrow_annot (n := 1) (r := ⟨mutable, .struct sid, ⟨.global a sid, []⟩, s.invalidations.length⟩)
      (pre := []) (post := [.privileged info.moduleId sid instr]) (parent := none) hp hsf
      (fun hn => ?_) (by simp)
      (by simp [Event.Silent, Encapsulation.structsRespected, hmod]) (by simp [hst'])
      (by simp [hst', Value.abilities, RuntimeType.abilities, AbilitySet.primitives]) ?_
      (by simpa [advance, push, borrow] using ho) rfl rfl rfl
    · rw [ho]
      rcases hi with ⟨rfl | ⟨_, rfl⟩, rfl⟩ | ⟨rfl | ⟨_, rfl⟩, rfl⟩ <;>
        simp [Shadow.exec, guard, Shadow.topAfter, advance, push, borrow, hn]
    · simp [Value.key, Key.Ok, Root.isHeld]
  · exact (done_fail h hcont).elim

theorem mapM_ends {α β : Type} {g : α → Exec β} (hg : ∀ a, Ends (g a)) :
    ∀ (l : List α), Ends (l.mapM g)
  | [] => by simp [List.mapM_nil]; trivial
  | a :: l => by
    rw [List.mapM_cons]
    exact ends_bind (hg a) fun _ _ => ends_bind (mapM_ends hg l) fun _ _ => trivial

/-- A reference borrowed inside another keeps the key invariant: it is
mutable only if the other is, and its path is not empty. -/
theorem child_ok {r : Reference} (hok : (Value.ref r).key.Ok σ) {mutable : Bool}
    (hmut : mutable = true → r.mutable = true) (t : RuntimeType) (step : PathStep) (n : Nat) :
    (Value.ref ⟨mutable, t, r.location.child step, n⟩).key.Ok σ := by
  simp only [Value.key, Key.Ok, Location.child] at hok ⊢
  refine ⟨fun hσ hh => ?_, fun a ty hl => ?_⟩
  · cases hm : mutable
    · rfl
    · have := hok.1 hσ hh
      rw [hmut hm] at this
      exact absurd this (by simp)
  · simp only [Location.mk.injEq] at hl
    exact absurd hl.2 (by simp)

theorem borrowField_annot {mutable : Bool} {hh : FieldHandleIndex} {instr : Bytecode}
    (hi : (instr = .mutBorrowField hh ∨ ∃ i, instr = .mutBorrowFieldGeneric i) ∧ mutable = true ∨
      (instr = .immBorrowField hh ∨ ∃ i, instr = .immBorrowFieldGeneric i) ∧ mutable = false)
    {tys : List RuntimeType}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (hinfo : P.function? fr.function = some info)
    (h : stepOf (borrowField P s fr rest info mutable hh tys) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  unfold borrowField at h
  obtain ⟨fh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  have hmod := localStruct_module hls hinfo
  dsimp only at h
  split at h
  · obtain ⟨tys, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨t, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hst', rfl, htarget, hmut⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨_, -, h⟩ := done_bind h hcont deref_ends
    have ho := done_finish rfl h
    obtain ⟨hok, tr, htr, -⟩ := stack_key_ok hp hsf (k := 0) (r := r) (by simp [hst'])
    refine borrow_annot (n := 1) (s₂ := { s with frames := { fr with stack := st } :: rest })
      (r := ⟨mutable, t, r.location.child (.field fh.field), s.invalidations.length⟩)
      (pre := [.used tr.id r]) (post := [.privileged info.moduleId sid instr]) (parent := some tr.id)
      hp hsf (fun hn => ?_) (plain4_ok (by plain4))
      (by simp [Event.Silent, Encapsulation.structsRespected, hmod]) (by simp [hst'])
      (by simp [hst', ref_drop']) (child_ok hok hmut _ _ _) ?_ rfl rfl rfl
    · rw [ho]
      rcases hi with ⟨rfl | ⟨_, rfl⟩, rfl⟩ | ⟨rfl | ⟨_, rfl⟩, rfl⟩ <;>
        simp [Shadow.exec, guard, hn, hst', htr, htarget, Shadow.topAfter, advance, pushAll, borrow]
    · rw [ho]; simp [advance, pushAll, borrow, hst']
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem borrowVariantField_annot {mutable : Bool} {hh : VariantFieldHandleIndex} {instr : Bytecode}
    (hi : (instr = .mutBorrowVariantField hh ∨ ∃ i, instr = .mutBorrowVariantFieldGeneric i) ∧ mutable = true ∨
      (instr = .immBorrowVariantField hh ∨ ∃ i, instr = .immBorrowVariantFieldGeneric i) ∧ mutable = false)
    {tys : List RuntimeType}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (hinfo : P.function? fr.function = some info)
    (h : stepOf (borrowVariantField P s fr rest info mutable hh tys) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  unfold borrowVariantField at h
  obtain ⟨vfh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  have hmod := localStruct_module hls hinfo
  dsimp only at h
  split at h
  · obtain ⟨tys, -, h⟩ := done_bind h hcont (mapM_ends (fun _ =>
      ends_bind (ends_need _ _) fun _ _ => ends_bind (ends_need _ _) fun _ _ => ends_need _ _) _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hst', rfl, htarget, hmut⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨v, -, h⟩ := done_bind h hcont deref_ends
    split at h
    · next tag _ =>
      split at h
      · next t _ =>
        have ho := done_finish rfl h
        obtain ⟨hok, tr, htr, -⟩ := stack_key_ok hp hsf (k := 0) (r := r) (by simp [hst'])
        refine borrow_annot (n := 1) (s₂ := { s with frames := { fr with stack := st } :: rest })
          (r := ⟨mutable, t, r.location.child (.variantField tag vfh.field), s.invalidations.length⟩)
          (pre := [.accessed .inspect r.location (some tr.id), .used tr.id r])
          (post := [.privileged info.moduleId sid instr]) (parent := some tr.id)
          hp hsf (fun hn => ?_) (plain4_ok (by plain4))
          (by simp [Event.Silent, Encapsulation.structsRespected, hmod]) (by simp [hst'])
          (by simp [hst', ref_drop']) (child_ok hok hmut _ _ _) ?_ rfl rfl rfl
        · rw [ho]
          rcases hi with ⟨rfl | ⟨_, rfl⟩, rfl⟩ | ⟨rfl | ⟨_, rfl⟩, rfl⟩ <;>
            simp [Shadow.exec, guard, hn, hst', htr, htarget, Shadow.topAfter, advance, pushAll,
              borrow]
        · rw [ho]; simp [advance, pushAll, borrow, hst']
      · exact (done_fail h hcont).elim
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem testVariant_annot {instr : Bytecode} {hh : StructVariantHandleIndex}
    {tys : List RuntimeType} (hi : instr = .testVariant hh ∨ ∃ i, instr = .testVariantGeneric i)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (hinfo : P.function? fr.function = some info)
    (h : stepOf (testEnum P s fr rest info hh tys) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨vh, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨⟨sid, sinfo⟩, hls, h⟩ := done_bind h hcont localStruct_ends
  have hmod := localStruct_module hls hinfo
  dsimp only at h
  split at h
  · obtain ⟨_, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
    obtain ⟨st, hst', rfl, htarget, -⟩ := popRefTo_ok hpr
    dsimp only at h
    obtain ⟨v, -, h⟩ := done_bind h hcont deref_ends
    split at h
    · next tag _ =>
      have ho := done_finish rfl h
      obtain ⟨-, tr, htr, -⟩ := stack_key_ok hp hsf (k := 0) (r := r) (by simp [hst'])
      rw [show st = fr.stack.drop 1 by simp [hst']] at ho
      refine replace_annot (n := 1) (v := .bool (tag == vh.variant))
        (pre := [.accessed .inspect r.location (some tr.id), .used tr.id r])
        (post := [.privileged info.moduleId sid instr]) hp hsf (fun hn => ?_)
        (plain4_ok (by plain4)) (by simp [Event.Silent, Encapsulation.structsRespected, hmod])
        (by simp [hst']) (by simp [hst', ref_drop']) (primitive_plain rfl)
        (by simpa [advance, pushAll] using ho)
      rw [ho]
      rcases hi with rfl | ⟨_, rfl⟩ <;>
        simp [Shadow.exec, guard, hn, hst', htr, htarget, Shadow.topAfter, advance, pushAll,
          replaced]
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

theorem vecLen_annot {sig : SignatureIndex}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (Step.exec P s fr rest info code (.vecLen sig)) = .done o) :
    Annotated σ P π A auth info.moduleId (.vecLen sig) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨t, -, h⟩ := done_bind h hcont elemType_ends
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
  obtain ⟨st, hst', rfl, -, -⟩ := popRefTo_ok hpr
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h hcont derefVector_ends
  split at h
  · have ho := done_finish rfl h
    obtain ⟨-, tr, htr, -⟩ := stack_key_ok hp hsf (k := 0) (r := r) (by simp [hst'])
    rw [show st = fr.stack.drop 1 by simp [hst']] at ho
    refine replace_annot (n := 1) (v := .int .u64 elems.length)
      (pre := [.accessed .inspect r.location (some tr.id), .used tr.id r]) (post := [])
      hp hsf (fun hn => ?_) (plain4_ok (by plain4)) (by simp) (by simp [hst'])
      (by simp [hst', ref_drop']) (primitive_plain rfl) (by simpa [advance, pushAll] using ho)
    rw [ho]
    simp [Shadow.exec, guard, hn, hst', htr, Shadow.topAfter, advance, pushAll, replaced]
  · exact (done_fail h hcont).elim

theorem borrowElement_annot {mutable : Bool} {sig : SignatureIndex} {instr : Bytecode}
    (hi : instr = .vecMutBorrow sig ∧ mutable = true ∨ instr = .vecImmBorrow sig ∧ mutable = false)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (borrowElement s fr rest info mutable sig) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  unfold borrowElement at h
  obtain ⟨t, -, h⟩ := done_bind h hcont elemType_ends
  obtain ⟨⟨i, fr₁⟩, hpi, h⟩ := done_bind h hcont popIndex_ends
  obtain ⟨x, st₁, hst₁, rfl, rfl⟩ := popIndex_ok hpi
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
  obtain ⟨st, hst', rfl, -, hmut⟩ := popRefTo_ok hpr
  simp only at hst'
  dsimp only at h
  obtain ⟨elems, -, h⟩ := done_bind h hcont derefVector_ends
  split at h
  · have ho := done_finish rfl h
    have hst2 : fr.stack = .int .u64 x :: .ref r :: st := by rw [hst₁, hst']
    obtain ⟨hok, tr, htr, -⟩ := stack_key_ok hp hsf (k := 1) (r := r) (by simp [hst2])
    refine borrow_annot (n := 2) (s₂ := { s with frames := { fr with stack := st } :: rest })
      (r := ⟨mutable, t, r.location.child (.element x.toNat), s.invalidations.length⟩)
      (pre := [.accessed .inspect r.location (some tr.id), .used tr.id r]) (post := [])
      (parent := some tr.id) hp hsf (fun hn => ?_) (plain4_ok (by plain4)) (by simp)
      (by simp [hst2]) ?_ (child_ok hok hmut _ _ _) ?_ rfl rfl rfl
    · rw [ho]
      rcases hi with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩ <;>
        simp [Shadow.exec, guard, hn, hst2, htr, Shadow.topAfter, advance, pushAll, borrow]
    · intro w hw
      simp only [hst2, List.take_succ_cons, List.take_zero, List.mem_cons, List.not_mem_nil,
        or_false] at hw
      rcases hw with rfl | rfl
      · simp [Value.abilities, RuntimeType.abilities, AbilitySet.primitives]
      · exact ref_drop'
    · rw [ho]; simp [advance, pushAll, borrow, hst2]
  · exact (done_fail h hcont).elim

/-! ## Writes through references -/

theorem occurrencesAll_swap {cs : List IdTree} {es : List Value} {i j : Nat} {a b : IdTree}
    {x y : Value} (hci : cs[i]? = some a) (hcj : cs[j]? = some b) (hei : es[i]? = some x)
    (hej : es[j]? = some y) :
    (IdTree.occurrencesAll ((cs.set i b).set j a) ((es.set i y).set j x)).Perm
      (IdTree.occurrencesAll cs es) := by
  obtain ⟨R, h1, h2⟩ := occurrencesAll_set hci hei b y
  have hcj' : (cs.set i b)[j]? = some b := by
    rw [List.getElem?_set]; split
    · next hij => subst hij; simp [(List.getElem?_eq_some_iff.1 hci).1]
    · exact hcj
  have hej' : (es.set i y)[j]? = some y := by
    rw [List.getElem?_set]; split
    · next hij => subst hij; simp [(List.getElem?_eq_some_iff.1 hei).1]
    · exact hej
  obtain ⟨R', h3, h4⟩ := occurrencesAll_set hcj' hej' a x
  have hRR : R.Perm R' := (List.perm_append_left_iff _).1 (h2.symm.trans h3)
  exact h4.trans ((hRR.symm.append_left _).trans h1.symm)

theorem conforms_type' {v : Value} {t : RuntimeType} (h : v.conforms P t = true)
    (ht : rigid t = true) : v.type = t :=
  conforms_type h ht

/-- The reference at depth `k` of the current segment, when mutable, is not
into a held value. -/
theorem stack_not_held (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest) {k : Nat}
    {r : Reference} (hk : fr.stack[k]? = some (.ref r)) (hmut : r.mutable = true) :
    σ = true → r.location.root.isHeld = false := by
  intro hσ
  obtain ⟨hok, -⟩ := stack_key_ok hp hsf hk
  simp only [Value.key, Key.Ok] at hok
  cases hh : r.location.root.isHeld
  · rfl
  · have := hok.1 hσ hh; rw [hmut] at this; exact absurd this (by simp)

theorem elemTypeWith_value {m : CompiledModule} {sig : SignatureIndex} {tys : List RuntimeType}
    (htys : ∀ x ∈ tys, x.isValue = true) {t : RuntimeType}
    (h : elemType m sig tys = .ok t) : t.isValue = true := by
  unfold elemType at h
  rw [bind_ok'] at h
  obtain ⟨l, -, h⟩ := h
  rcases l with _ | ⟨t', _ | ⟨_, _⟩⟩
  · simp [wrong] at h
  · exact ofTokenWith_isValue htys (need_ok' h)
  · simp [wrong] at h

theorem elemType_value {m : CompiledModule} {sig : SignatureIndex} {t : RuntimeType}
    (h : elemType m sig = .ok t) : t.isValue = true :=
  elemTypeWith_value (by simp) h

/-- A struct type with field types has type arguments of value types. -/
theorem fieldTypes_args {s : StructId} {tys : List RuntimeType} (h : P.fieldTypes s = some tys) :
    ∀ x ∈ s.typeArgs, x.isValue = true := by
  unfold Program.fieldTypes at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, hinfo, -⟩ := h
  exact structType?_args_value hinfo

/-- An enum type with variant field types has type arguments of value types. -/
theorem variantFieldTypes_args {s : StructId} {tag : VariantIndex} {tys : List RuntimeType}
    (h : P.variantFieldTypes s tag = some tys) : ∀ x ∈ s.typeArgs, x.isValue = true := by
  unfold Program.variantFieldTypes at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, hinfo, -⟩ := h
  exact structType?_args_value hinfo

/-- The captured parameters of a closure type are value types. -/
theorem closureType?_captured_value {f : FunctionRef} {mask : BitVec 64}
    {ctys rem rts : List RuntimeType}
    (h : P.closureType? f mask = some (ctys, rem, rts)) : ∀ t ∈ ctys, t.isValue = true := by
  unfold Program.closureType? at h
  simp only [bind, Option.bind_eq_some_iff] at h
  obtain ⟨info, -, _, -, h⟩ := h
  split at h
  · simp at h
  · obtain ⟨params, -, h⟩ := Option.bind_eq_some_iff.1 h
    obtain ⟨results, -, h⟩ := Option.bind_eq_some_iff.1 h
    split at h
    · simp at h
    · split at h
      · simp at h
      · next hall =>
        simp only [pure, Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨rfl, -, -⟩ := h
        intro t ht
        simp only [Bool.not_eq_true', Bool.not_eq_false] at hall
        exact List.all_eq_true.1 (by simpa using hall) t ht

/-- A value holds no reference below its top when its children hold none. -/
theorem plain_children {v : Value} (hv : v.isRef = false)
    (hc : ∀ c ∈ v.children, ∀ u ∈ subvalues c, u.isRef = false) :
    ∀ u ∈ subvalues v, u.isRef = false := by
  intro u hu
  rw [subvalues_eq, List.mem_cons, List.mem_flatMap] at hu
  rcases hu with rfl | ⟨c, hc', hu⟩
  · exact hv
  · exact hc c hc' u hu

theorem plain_child {v c : Value} (hv : ∀ u ∈ subvalues v, u.isRef = false) (hc : c ∈ v.children) :
    ∀ u ∈ subvalues c, u.isRef = false := fun u hu =>
  hv u (by rw [subvalues_eq, List.mem_cons, List.mem_flatMap]; exact Or.inr ⟨c, hc, hu⟩)

theorem allConform_mem' {vs : List Value} {t : RuntimeType} (h : Value.allConform P vs t = true)
    {c : Value} (hc : c ∈ vs) : c.type.assignable t = true ∧ c.wellFormed P = true := by
  induction vs with
  | nil => simp at hc
  | cons v vs ih =>
    simp only [Value.allConform, Bool.and_eq_true] at h
    rcases List.mem_cons.1 hc with rfl | hc
    · exact h.1
    · exact ih h.2 hc

theorem conformsAll_mem' {vs : List Value} {ts : List RuntimeType} (h : conformsAll P vs ts = true)
    {c : Value} (hc : c ∈ vs) : ∃ t ∈ ts, c.type.assignable t = true ∧ c.wellFormed P = true := by
  induction vs generalizing ts with
  | nil => simp at hc
  | cons v vs ih =>
    cases ts with
    | nil => simp [conformsAll] at h
    | cons t ts =>
      simp only [conformsAll, Bool.and_eq_true] at h
      rcases List.mem_cons.1 hc with rfl | hc
      · exact ⟨t, List.mem_cons_self .., h.1⟩
      · obtain ⟨t', ht', h'⟩ := ih h.2 hc
        exact ⟨t', List.mem_cons_of_mem _ ht', h'⟩

/-- **A well-formed value of a value type holds no reference.** -/
theorem wellFormed_plain (v : Value) (hw : v.wellFormed P = true) (hv : v.type.isValue = true) :
    ∀ u ∈ subvalues v, u.isRef = false := by
  induction v using Value.rec
    (motive_2 := fun vs => ∀ c ∈ vs, c.wellFormed P = true → c.type.isValue = true →
      ∀ u ∈ subvalues c, u.isRef = false) with
  | vector e vs ih =>
    intro u hu
    rw [subvalues_eq, List.mem_cons] at hu
    rcases hu with rfl | hu
    · rfl
    · obtain ⟨c, hc, huc⟩ := List.mem_flatMap.1 hu
      simp only [Value.wellFormed] at hw
      obtain ⟨hct, hcw⟩ := allConform_mem' hw hc
      exact ih c hc hcw (by rw [assignable_isValue hct]; simpa [Value.type, RuntimeType.isValue] using hv)
        u huc
  | struct st fs ih =>
    intro u hu
    rw [subvalues_eq, List.mem_cons] at hu
    rcases hu with rfl | hu
    · rfl
    · obtain ⟨c, hc, huc⟩ := List.mem_flatMap.1 hu
      obtain ⟨-, tys, htys, hw⟩ := wellFormed_struct.1 hw
      obtain ⟨t, ht, hct, hcw⟩ := conformsAll_mem' hw hc
      exact ih c hc hcw (by
        rw [assignable_isValue hct]; exact fieldTypes_values (fieldTypes_args htys) htys t ht) u huc
  | table st fs ks es ihf _ ihe =>
    intro u hu
    rw [subvalues_eq, List.mem_cons] at hu
    rcases hu with rfl | hu
    · rfl
    · obtain ⟨c, hc, huc⟩ := List.mem_flatMap.1 hu
      obtain ⟨-, -, ⟨tys, htys, hwf⟩, hes⟩ := wellFormed_table.1 hw
      simp only [Value.children, List.mem_append] at hc
      rcases hc with hc | hc
      · obtain ⟨t, ht, hct, hcw⟩ := conformsAll_mem' hwf hc
        exact ihf c hc hcw (by
          rw [assignable_isValue hct]; exact fieldTypes_values (fieldTypes_args htys) htys t ht) u huc
      · rcases hes with rfl | ⟨-, k, v, -, -, hce, -⟩
        · simp at hc
        · obtain ⟨hct, hcw⟩ := allConform_mem' hce hc
          exact ihe c hc hcw (by rw [assignable_isValue hct]; rfl) u huc
  | variant st tag fs ih =>
    intro u hu
    rw [subvalues_eq, List.mem_cons] at hu
    rcases hu with rfl | hu
    · rfl
    · obtain ⟨c, hc, huc⟩ := List.mem_flatMap.1 hu
      simp only [Value.wellFormed] at hw
      split at hw
      · next tys htys =>
        obtain ⟨t, ht, hct, hcw⟩ := conformsAll_mem' hw hc
        exact ih c hc hcw (by
          rw [assignable_isValue hct]
          exact variantFieldTypes_values (variantFieldTypes_args htys) htys t ht) u huc
      · cases hw
  | ref r => simp [Value.type, RuntimeType.isValue] at hv
  | closure fn mask cs ps rs a ih =>
    intro u hu
    rw [subvalues_eq, List.mem_cons] at hu
    rcases hu with rfl | hu
    · rfl
    · obtain ⟨c, hc, huc⟩ := List.mem_flatMap.1 hu
      simp only [Value.children] at hc
      simp only [Value.wellFormed] at hw
      split at hw
      · next ctys rem rts hct =>
        simp only [Bool.and_eq_true] at hw
        obtain ⟨t, ht, hca, hcw⟩ := conformsAll_mem' hw.2 hc
        exact ih c hc hcw (by rw [assignable_isValue hca]; exact closureType?_captured_value hct t ht)
          u huc
      · cases hw
  | bool b => intro u hu; simp [subvalues, Value.nodes] at hu; subst hu; rfl
  | int t x => intro u hu; simp [subvalues, Value.nodes] at hu; subst hu; rfl
  | address a => intro u hu; simp [subvalues, Value.nodes] at hu; subst hu; rfl
  | signer a => intro u hu; simp [subvalues, Value.nodes] at hu; rcases hu with rfl | rfl <;> rfl
  | nil =>
    rename_i c hc _ _ _ _
    simp at hc
  | cons v vs ihv ihvs =>
    rename_i c hc hcw hct u hu
    rcases List.mem_cons.1 hc with rfl | hc
    · exact ihv hcw hct u hu
    · exact ihvs c hc hcw hct u hu

/-- A value that conforms to a value type holds no reference. -/
theorem conforms_plain {v : Value} {t : RuntimeType} (h : v.conforms P t = true) (ht : t.isValue = true) :
    ∀ u ∈ subvalues v, u.isRef = false := by
  simp only [Value.conforms, Bool.and_eq_true] at h
  exact wellFormed_plain v h.2 (by rw [assignable_isValue h.1]; exact ht)

/-- The target of a reference on the current segment is a value type. -/
theorem stack_target (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest) {k : Nat}
    {r : Reference} (hk : fr.stack[k]? = some (.ref r)) : r.target.isValue = true :=
  hp.refsTop.targets fr (by rw [hsf]; exact List.mem_cons_self ..) r
    (Or.inl (List.mem_of_getElem? hk))

/-- The value a reference on the current segment designates holds no
reference. -/
theorem deref_plain (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest) {k : Nat}
    {r : Reference} (hk : fr.stack[k]? = some (.ref r)) {v : Value}
    (hv : s.read? r.location = some v) : ∀ u ∈ subvalues v, u.isRef = false :=
  hp.refsTop.deref fr (by rw [hsf]; exact List.mem_cons_self ..) r
    (Or.inl (List.mem_of_getElem? hk)) v hv

theorem isRef_of_type_eq {v w : Value} (h : v.type = w.type) (hw : w.isRef = false) :
    v.isRef = false := by
  cases v <;> cases w <;> simp_all [Value.type, Value.isRef]

theorem isRef_of_value {v : Value} {t : RuntimeType} (h : v.type = t) (ht : t.isValue = true) :
    v.isRef = false := by
  cases v with
  | ref r => subst h; simp [Value.type, RuntimeType.isValue] at ht
  | _ => rfl

theorem writeRef_annot (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (Step.exec P s fr rest info code .writeRef) = .done o) :
    Annotated σ P π A auth info.moduleId .writeRef s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  have hlen := fitsAll_length (frame_stack_fits hg)
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRef_ends
  obtain ⟨st₁, hst₁, rfl⟩ := popRef_ok hpr
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  next hmut =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hmut
  obtain ⟨⟨v, fr₂⟩, hpv, h⟩ := done_bind h hcont pop_ends
  obtain ⟨st, hst₂, rfl⟩ := pop_ok hpv
  simp only at hst₂
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  next hconf =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hconf
  obtain ⟨old, hold, h⟩ := done_bind h hcont deref_ends
  split at h
  · exact (done_wrong h hcont).elim
  next hdrop =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hdrop
  obtain ⟨s₂', hu, h⟩ := done_bind h hcont update_ends
  obtain ⟨s₂, hwr, rfl⟩ := update_ok hu
  have hstk : fr.stack = .ref r :: v :: st := by rw [hst₁, hst₂]
  have hfr₁ : ({ s with frames := { { fr with stack := st₁ } with stack := st } :: rest } : State) =
      { s with frames := { fr with stack := fr.stack.drop 2 } :: rest } := by simp [hstk]
  rw [hfr₁] at hold hwr
  have hd := deref_ok hold
  rw [deref_eq (s' := { s with frames := { fr with stack := fr.stack.drop 2 } :: rest }) (s := s)
    (by simp [hsf]) rfl rfl rfl] at hd
  have hread := deref?_read hd
  have hheld := stack_not_held hp hsf (k := 0) (by simp [hstk]) hmut
  have hfa := frame_stack_fits hg
  rw [hstk] at hfa
  obtain ⟨tr, ts₁, hgst₁, htr, hts₁⟩ := fitsAll_cons_right hfa
  obtain ⟨tv, ts, rfl, htv, -⟩ := fitsAll_cons_right hts₁
  obtain ⟨tw, htw, hfw⟩ := tree_at hp hsf rfl 2 hread
  obtain ⟨h₂, s₂'', fr₂, rest₂, hwith, hwrite, hs₂⟩ := write_exists hp hsf rfl 2 hread htv (write?_writable hwr)
  rw [hwr, Option.some.injEq] at hwrite
  subst hwrite
  obtain ⟨g₂, hs₂', hh₂, -, hG⟩ := mutate_ghost (π := π) (A := A) (auth := auth)
    (evs := [.accessed .write r.location (some tr.id), .used tr.id r])
    (vals := []) (tvs := []) (keep := keys (tv.occurrences v))
    (dropped := [(tr.id, (Value.ref r).key)] ++ keys (tw.occurrences old))
    (s₃ := s₂.invalidate (.mutated r.location)) hp hsf rfl (n := 2) (by simp [hstk]) hheld hread htw htv
    (fun hpath a t hroot => by
      obtain ⟨hglob, -⟩ := stack_key_ok hp hsf (k := 0) (r := r) (by simp [hstk])
      simp only [Value.key, Key.Ok] at hglob
      have hrt : r.target = .struct t := hglob.2 a t (by
        cases hl : r.location with
        | mk root path => simp_all)
      rw [conforms_type hconf (by rw [hrt]; rfl), (stack_typed_rigid hp hsf (k := 0) (r := r)
        (by simp [hstk]) hd (by rw [hrt]; rfl))])
    hwith (by simpa using hwr) hs₂ (by simp [IdTree.fitsAll])
    (conforms_plain hconf (stack_target hp hsf (k := 0) (by simp [hstk])))
    (by simp)
    (by simp [hgst₁, hstk, IdTree.occurrencesAll, ref_occ htr]; perm_parts)
    (by simp [IdTree.occurrencesAll])
    (fun x hx => by
      simp only [List.mem_append, List.mem_singleton] at hx
      rcases hx with rfl | hx
      · simp [Value.type, RuntimeType.abilities, key_type]
      · obtain ⟨o', ho', -, h2⟩ := mem_keys_iff.1 hx
        rw [← h2, key_type]
        exact deep_drop old hdrop o'.2 (occurrences_mem hfw ho'))
    (fun x hx _ => by
      simp only [IdTree.occurrencesAll, List.nil_append] at hx
      rw [hgst₁, hstk]
      simp only [List.take_succ_cons, List.take_zero, IdTree.occurrencesAll, List.append_nil,
        List.mem_append]
      simp [hx])
    (plain4_ok (by plain4)) rfl rfl rfl (fr₂.pc + 1)
  have ho := done_finish (s₁ := s₂.invalidate (.mutated r.location)) hs₂ h
  subst ho
  rw [shadow_eta hh₂] at hG
  refine ⟨_, _, ?_, hG⟩
  simp only [hgst₁, List.drop_succ_cons, List.drop_zero] at htw hwith
  simp [Shadow.exec, guard, hstk, hgst₁, htw, hwith]

theorem vector_tree {t : IdTree} {e : RuntimeType} {elems : List Value}
    (ht : t.fits (.vector e elems) = true) :
    IdTree.fitsAll t.children elems = true ∧
      t.occurrences (.vector e elems) = (t.id, .vector e elems) :: IdTree.occurrencesAll t.children elems := by
  obtain ⟨i, cs⟩ := t
  simp only [IdTree.fits, Value.children] at ht
  exact ⟨ht, rfl⟩

theorem vecPushBack_annot {sig : SignatureIndex}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (Step.exec P s fr rest info code (.vecPushBack sig)) = .done o) :
    Annotated σ P π A auth info.moduleId (.vecPushBack sig) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  have hlen := fitsAll_length (frame_stack_fits hg)
  obtain ⟨t, hte, h⟩ := done_bind h hcont elemType_ends
  obtain ⟨⟨v, fr₁⟩, hpv, h⟩ := done_bind h hcont pop_ends
  obtain ⟨st₁, hst₁, rfl⟩ := pop_ok hpv
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
  obtain ⟨st, hst₂, rfl, htarget, hmut⟩ := popRefTo_ok hpr
  simp only at hst₂
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  next hconf =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hconf
  obtain ⟨elems, helems, h⟩ := done_bind h hcont derefVector_ends
  obtain ⟨s₂', hu, hfin⟩ := done_bind h hcont update_ends
  obtain ⟨s₂, hwr, rfl⟩ := update_ok hu
  have hstk : fr.stack = v :: .ref r :: st := by rw [hst₁, hst₂]
  have hfr₁ : ({ s with frames := { { fr with stack := st₁ } with stack := st } :: rest } : State) =
      { s with frames := { fr with stack := fr.stack.drop 2 } :: rest } := by simp [hstk]
  rw [hfr₁] at helems hwr
  obtain ⟨t', hd⟩ := derefVector_ok helems
  rw [deref_eq (s' := { s with frames := { fr with stack := fr.stack.drop 2 } :: rest }) (s := s)
    (by simp [hsf]) rfl rfl rfl] at hd
  have hread := deref?_read hd
  have htyped := stack_typed_rigid hp hsf (k := 1) (r := r) (by simp [hstk]) hd (by rw [htarget]; rfl)
  rw [htarget] at htyped
  simp only [Value.type, RuntimeType.vector.injEq] at htyped
  subst htyped
  have hheld := stack_not_held hp hsf (k := 1) (by simp [hstk]) (hmut rfl)
  have hfa := frame_stack_fits hg
  rw [hstk] at hfa
  obtain ⟨tv, ts₁, hgst₁, htv, hts₁⟩ := fitsAll_cons_right hfa
  obtain ⟨tr, ts, rfl, htr, -⟩ := fitsAll_cons_right hts₁
  obtain ⟨vt, htw, hfw⟩ := tree_at hp hsf rfl 2 hread
  obtain ⟨hcs, hvocc⟩ := vector_tree hfw
  have hfw' : (IdTree.node vt.id (vt.children ++ [tv])).fits (.vector t' (elems ++ [v])) = true := by
    simp only [IdTree.fits, Value.children]
    rw [fitsAll_append (fitsAll_length hcs)]
    simp [hcs, IdTree.fitsAll, htv]
  obtain ⟨h₂, s₂'', fr₂, rest₂, hwith, hwrite, hs₂⟩ := write_exists hp hsf rfl 2 hread hfw' (write?_writable hwr)
  rw [hwr, Option.some.injEq] at hwrite
  subst hwrite
  obtain ⟨g₂, hs₂', hh₂, -, hG⟩ := mutate_ghost (π := π) (A := A) (auth := auth)
    (evs := [.accessed .write r.location (some tr.id), .used tr.id r])
    (vals := []) (tvs := [])
    (keep := keys (tv.occurrences v) ++ (vt.id, (Value.vector t' elems).key) ::
      keys (IdTree.occurrencesAll vt.children elems))
    (dropped := [(tr.id, (Value.ref r).key)])
    (s₃ := s₂.invalidate (.mutated r.location)) hp hsf rfl (n := 2) (by simp [hstk]) hheld hread htw
    hfw' (fun _ _ _ _ => rfl) hwith (by simpa using hwr) hs₂ (by simp [IdTree.fitsAll])
    (plain_children rfl fun c hc => by
      simp only [Value.children, List.mem_append, List.mem_singleton] at hc
      rcases hc with hc | rfl
      · exact plain_child (deref_plain hp hsf (k := 1) (by simp [hstk]) hread)
          (by simpa [Value.children] using hc)
      · have htv := stack_target hp hsf (k := 1) (r := r) (by simp [hstk])
        rw [htarget] at htv
        exact conforms_plain hconf (by simpa [RuntimeType.isValue] using htv))
    (by simp)
    (by simp [hgst₁, hstk, IdTree.occurrencesAll, ref_occ htr, hvocc]; perm_parts)
    (by
      simp only [IdTree.occurrencesAll, occurrences_node, Value.children, keys_cons, keys_nil,
        List.nil_append]
      rw [occurrencesAll_append (fitsAll_length hcs)]
      simp only [IdTree.occurrencesAll, List.append_nil, keys_append]
      simp only [Value.key]
      perm_parts)
    (by simp [Value.type, RuntimeType.abilities, key_type])
    (fun x hx hc => by
      simp only [IdTree.occurrencesAll, List.nil_append, occurrences_node, Value.children,
        List.mem_cons] at hx
      rw [occurrencesAll_append (fitsAll_length hcs)] at hx
      rw [hgst₁, hstk, hvocc]
      simp only [IdTree.occurrencesAll, List.take_succ_cons, List.take_zero, List.append_nil,
        List.mem_append, List.mem_cons] at hx ⊢
      rcases hx with rfl | hx | hx <;> first | (simp [IsClosure] at hc; done) | simp [hx])
    (plain4_ok (by plain4)) rfl rfl rfl (fr₂.pc + 1)
  have ho := done_finish (s₁ := s₂.invalidate (.mutated r.location)) hs₂ hfin
  subst ho
  rw [shadow_eta hh₂] at hG
  refine ⟨_, _, ?_, hG⟩
  simp only [hgst₁, List.drop_succ_cons, List.drop_zero] at htw hwith
  simp [Shadow.exec, guard, hstk, hgst₁, htw, hwith]

theorem vecPopBack_annot {sig : SignatureIndex}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (Step.exec P s fr rest info code (.vecPopBack sig)) = .done o) :
    Annotated σ P π A auth info.moduleId (.vecPopBack sig) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  have hlen := fitsAll_length (frame_stack_fits hg)
  obtain ⟨t, -, h⟩ := done_bind h hcont elemType_ends
  obtain ⟨⟨r, fr₁⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
  obtain ⟨st, hstk, rfl, htarget, hmut⟩ := popRefTo_ok hpr
  dsimp only at h
  obtain ⟨elems, helems, h⟩ := done_bind h hcont derefVector_ends
  split at h
  · next last init hrev =>
    obtain ⟨s₂', hu, hfin⟩ := done_bind h hcont update_ends
    obtain ⟨s₂, hwr, rfl⟩ := update_ok hu
    have hfr₁ : ({ s with frames := { fr with stack := st } :: rest } : State) =
        { s with frames := { fr with stack := fr.stack.drop 1 } :: rest } := by simp [hstk]
    rw [hfr₁] at helems hwr
    obtain ⟨t', hd⟩ := derefVector_ok helems
    rw [deref_eq (s' := { s with frames := { fr with stack := fr.stack.drop 1 } :: rest }) (s := s)
      (by simp [hsf]) rfl rfl rfl] at hd
    have hread := deref?_read hd
    have htyped := stack_typed_rigid hp hsf (k := 0) (r := r) (by simp [hstk]) hd
      (by rw [htarget]; rfl)
    rw [htarget] at htyped
    simp only [Value.type, RuntimeType.vector.injEq] at htyped
    subst htyped
    have hheld := stack_not_held hp hsf (k := 0) (by simp [hstk]) (hmut rfl)
    have hfa := frame_stack_fits hg
    rw [hstk] at hfa
    obtain ⟨tr, ts, hgst, htr, -⟩ := fitsAll_cons_right hfa
    obtain ⟨vt, htw, hfw⟩ := tree_at hp hsf rfl 1 hread
    obtain ⟨hcs, hvocc⟩ := vector_tree hfw
    have helems' : elems = init.reverse ++ [last] := by
      have := congrArg List.reverse hrev
      simpa using this
    rw [helems'] at hcs hvocc hfw hread
    obtain ⟨cs', tl, hcs'⟩ : ∃ cs' tl, vt.children = cs' ++ [tl] := by
      have hl := fitsAll_length hcs
      rcases List.eq_nil_or_concat vt.children with hn | ⟨cs', tl, hc⟩
      · rw [hn] at hl; simp at hl
      · exact ⟨cs', tl, by rw [hc, List.concat_eq_append]⟩
    rw [hcs'] at hcs
    rw [fitsAll_append (by have := fitsAll_length hcs; simp at this ⊢; omega)] at hcs
    simp only [Bool.and_eq_true, IdTree.fitsAll, Bool.and_true] at hcs
    obtain ⟨hcs₁, htl⟩ := hcs
    have hfw' : (IdTree.node vt.id cs').fits (.vector t' init.reverse) = true := by
      simpa [IdTree.fits, Value.children] using hcs₁
    obtain ⟨h₂, s₂'', fr₂, rest₂, hwith, hwrite, hs₂⟩ := write_exists hp hsf rfl 1 hread hfw' (write?_writable hwr)
    rw [hwr, Option.some.injEq] at hwrite
    subst hwrite
    obtain ⟨g₂, hs₂', hh₂, -, hG⟩ := mutate_ghost (π := π) (A := A) (auth := auth)
      (evs := [.accessed .write r.location (some tr.id), .used tr.id r])
      (vals := [last]) (tvs := [tl])
      (keep := (vt.id, (Value.vector t' (init.reverse ++ [last])).key) ::
        keys (IdTree.occurrencesAll cs' init.reverse) ++ keys (tl.occurrences last))
      (dropped := [(tr.id, (Value.ref r).key)])
      (s₃ := s₂.invalidate (.mutated r.location)) hp hsf rfl (n := 1) (by simp [hstk]) hheld hread
      htw hfw' (fun _ _ _ _ => rfl) hwith (by simpa using hwr) hs₂ (by simp [IdTree.fitsAll, htl])
      (plain_children rfl fun c hc => plain_child (deref_plain hp hsf (k := 0) (by simp [hstk]) hread)
        (by simp only [Value.children] at hc ⊢; exact List.mem_append_left _ hc))
      (fun w hw => by
        simp only [List.mem_singleton] at hw
        subst hw
        exact plain_child (deref_plain hp hsf (k := 0) (by simp [hstk]) hread)
          (by simp [Value.children]))
      (by
        simp only [hgst, hstk, List.take_succ_cons, List.take_zero, IdTree.occurrencesAll,
          ref_occ htr, hvocc, hcs', List.append_nil]
        rw [occurrencesAll_append (fitsAll_length hcs₁)]
        simp only [IdTree.occurrencesAll, List.append_nil, keys_append, keys_cons, keys_nil]
        perm_parts)
      (by
        simp only [IdTree.occurrencesAll, occurrences_node, Value.children, keys_cons,
          List.append_nil, Value.key]
        perm_parts)
      (by simp [Value.type, RuntimeType.abilities, key_type])
      (fun x hx hc => by
        rw [hvocc, hcs', occurrencesAll_append (fitsAll_length hcs₁)]
        simp only [IdTree.occurrencesAll, occurrences_node, Value.children, List.append_nil,
          List.mem_append, List.mem_cons] at hx ⊢
        rcases hx with hx | rfl | hx <;> first | (simp [IsClosure] at hc; done) | simp [hx])
      (plain4_ok (by plain4)) rfl rfl rfl (fr₂.pc + 1)
    have ho := done_finish (s₁ := s₂.invalidate (.mutated r.location)) hs₂ hfin
    subst ho
    refine ⟨_, _, ?_, hG⟩
    have hfull : Shadow.tree? ⟨g :: hs, hst, hhe, f⟩ s r.location = some vt := by
      rw [tree_eq (h := { (⟨g :: hs, hst, hhe, f⟩ : Shadow) with
        frames := { g with stack := g.stack.drop 1 } :: hs }) (h' := ⟨g :: hs, hst, hhe, f⟩)
        (s := s) (s' := s) rfl rfl (by simp) rfl rfl _ rfl]
      exact htw
    simp only [hgst, List.drop_succ_cons, List.drop_zero] at htw hwith
    simp [Shadow.exec, guard, hstk, hgst, htw, hwith, hcs', hh₂, hfull]
  · exact (done_fail h hcont).elim

theorem vecSwap_annot {sig : SignatureIndex}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true)
    (h : stepOf (Step.exec P s fr rest info code (.vecSwap sig)) = .done o) :
    Annotated σ P π A auth info.moduleId (.vecSwap sig) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
  have hlen := fitsAll_length (frame_stack_fits hg)
  obtain ⟨t, -, h⟩ := done_bind h hcont elemType_ends
  obtain ⟨⟨j, fr₁⟩, hpj, h⟩ := done_bind h hcont popIndex_ends
  obtain ⟨jx, st₁, hst₁, rfl, rfl⟩ := popIndex_ok hpj
  obtain ⟨⟨i, fr₂⟩, hpi, h⟩ := done_bind h hcont popIndex_ends
  obtain ⟨ix, st₂, hst₂, rfl, rfl⟩ := popIndex_ok hpi
  obtain ⟨⟨r, fr₃⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
  obtain ⟨st, hst₃, rfl, htarget, hmut⟩ := popRefTo_ok hpr
  simp only at hst₂ hst₃
  dsimp only at h
  obtain ⟨elems, helems, h⟩ := done_bind h hcont derefVector_ends
  split at h
  · next x y hx hy =>
    obtain ⟨s₂', hu, hfin⟩ := done_bind h hcont update_ends
    obtain ⟨s₂, hwr, rfl⟩ := update_ok hu
    have hstk : fr.stack = .int .u64 jx :: .int .u64 ix :: .ref r :: st := by rw [hst₁, hst₂, hst₃]
    have hfr₁ : ({ s with frames := { { { fr with stack := st₁ } with stack := st₂ } with stack := st } :: rest } : State) =
        { s with frames := { fr with stack := fr.stack.drop 3 } :: rest } := by simp [hstk]
    rw [hfr₁] at helems hwr
    obtain ⟨t', hd⟩ := derefVector_ok helems
    rw [deref_eq (s' := { s with frames := { fr with stack := fr.stack.drop 3 } :: rest }) (s := s)
      (by simp [hsf]) rfl rfl rfl] at hd
    have hread := deref?_read hd
    have htyped := stack_typed_rigid hp hsf (k := 2) (r := r) (by simp [hstk]) hd
      (by rw [htarget]; rfl)
    rw [htarget] at htyped
    simp only [Value.type, RuntimeType.vector.injEq] at htyped
    subst htyped
    have hheld := stack_not_held hp hsf (k := 2) (by simp [hstk]) (hmut rfl)
    have hfa := frame_stack_fits hg
    rw [hstk] at hfa
    obtain ⟨tj, ts₁, hgst₁, htj, hts₁⟩ := fitsAll_cons_right hfa
    obtain ⟨ti, ts₂, rfl, hti, hts₂⟩ := fitsAll_cons_right hts₁
    obtain ⟨tr, ts, rfl, htr, -⟩ := fitsAll_cons_right hts₂
    obtain ⟨vt, htw, hfw⟩ := tree_at hp hsf rfl 3 hread
    obtain ⟨hcs, hvocc⟩ := vector_tree hfw
    obtain ⟨ta, hta, hfa'⟩ := fitsAll_get hcs hx
    obtain ⟨tb, htb, hfb⟩ := fitsAll_get hcs hy
    have hfw' : (IdTree.node vt.id ((vt.children.set ix.toNat tb).set jx.toNat ta)).fits
        (.vector t' ((elems.set ix.toNat y).set jx.toNat x)) = true := by
      simp only [IdTree.fits, Value.children]
      exact fitsAll_set (fitsAll_set hcs _ hfb) _ hfa'
    obtain ⟨h₂, s₂'', fr₂, rest₂, hwith, hwrite, hs₂⟩ := write_exists hp hsf rfl 3 hread hfw' (write?_writable hwr)
    rw [hwr, Option.some.injEq] at hwrite
    subst hwrite
    have hswap := occurrencesAll_swap hta htb hx hy
    obtain ⟨g₂, hs₂', hh₂, -, hG⟩ := mutate_ghost (π := π) (A := A) (auth := auth)
      (evs := [.accessed .write r.location (some tr.id), .used tr.id r])
      (vals := []) (tvs := [])
      (keep := keys (vt.occurrences (.vector t' elems)))
      (dropped := [(tj.id, (Value.int .u64 jx).key), (ti.id, (Value.int .u64 ix).key),
        (tr.id, (Value.ref r).key)])
      (s₃ := s₂.invalidate (.mutated r.location)) hp hsf rfl (n := 3) (by simp [hstk]) hheld hread
      htw hfw' (fun _ _ _ _ => rfl) hwith (by simpa using hwr) hs₂ (by simp [IdTree.fitsAll])
      (plain_children rfl fun c hc => by
        have hplain := deref_plain hp hsf (k := 2) (by simp [hstk]) hread
        simp only [Value.children] at hc
        have hcm : c ∈ elems := by
          rcases List.mem_or_eq_of_mem_set hc with hc | rfl
          · rcases List.mem_or_eq_of_mem_set hc with hc | rfl
            · exact hc
            · exact List.mem_of_getElem? hy
          · exact List.mem_of_getElem? hx
        exact plain_child hplain (by simpa [Value.children] using hcm))
      (by simp)
      (by
        simp only [hgst₁, hstk, List.take_succ_cons, List.take_zero, IdTree.occurrencesAll,
          ref_occ htr, List.append_nil, keys_append, keys_cons, keys_nil]
        have h1 : tj.occurrences (.int .u64 jx) = [(tj.id, .int .u64 jx)] := by
          obtain ⟨_, cs⟩ := tj; cases cs <;> simp_all [IdTree.fits, IdTree.fitsAll, Value.children,
            IdTree.occurrencesAll, IdTree.id]
        have h2 : ti.occurrences (.int .u64 ix) = [(ti.id, .int .u64 ix)] := by
          obtain ⟨_, cs⟩ := ti; cases cs <;> simp_all [IdTree.fits, IdTree.fitsAll, Value.children,
            IdTree.occurrencesAll, IdTree.id]
        rw [h1, h2]
        simp only [keys_cons, keys_nil]
        perm_parts)
      (by
        simp only [IdTree.occurrencesAll, occurrences_node, Value.children, keys_cons, keys_nil,
          List.nil_append, hvocc]
        simp only [Value.key]
        exact List.Perm.cons _ (keys_perm hswap))
      (by
        intro z hz
        simp only [List.mem_cons, List.not_mem_nil, or_false] at hz
        rcases hz with rfl | rfl | rfl <;>
          simp [Value.type, RuntimeType.abilities, key_type, AbilitySet.primitives])
      (fun z hz hc => by
        rw [hvocc]
        simp only [IdTree.occurrencesAll, occurrences_node, Value.children, List.nil_append,
          List.mem_cons, List.mem_append] at hz ⊢
        rcases hz with rfl | hz
        · simp [IsClosure] at hc
        · simp [hswap.subset hz])
      (plain4_ok (by plain4)) rfl rfl rfl (fr₂.pc + 1)
    have ho := done_finish (s₁ := s₂.invalidate (.mutated r.location)) hs₂ hfin
    subst ho
    rw [shadow_eta hh₂] at hG
    refine ⟨_, _, ?_, hG⟩
    simp only [hgst₁, List.drop_succ_cons, List.drop_zero] at htw hwith
    simp [Shadow.exec, guard, hstk, hgst₁, htw, hwith, hta, htb]
  · exact (done_fail h hcont).elim
  · exact (done_fail h hcont).elim

/-! ## Global storage -/

theorem exists_annot {instr : Bytecode} {d : StructDefinitionIndex} {tys : List RuntimeType}
    (hi : instr = .exists_ d ∧ tys = [] ∨ ∃ i x, instr = .existsGeneric i ∧
      info.module.structDefInstantiations[i]? = some x ∧ x.def_ = d ∧
      info.module.typeArgsWith fr.function.typeArgs x.typeParameters = some tys)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (hinfo : P.function? fr.function = some info)
    (h : stepOf (existsResource P s fr rest d tys) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨sid, hsid, h⟩ := done_bind h hcont globalType_ends
  obtain ⟨sid₀, hdef, rfl⟩ := globalType_ok hsid
  have hmod := definedStruct_module (sid := sid₀) hdef hinfo
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h hcont popAddress_ends
  obtain ⟨st, hstk, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h hcont unlocked_ends
  have ho := done_next h
  rw [show st = fr.stack.drop 1 by simp [hstk]] at ho
  refine replace_annot (n := 1) (v := .bool (s.storage.get? a { sid₀ with typeArgs := tys }).isSome)
    (pre := []) (post := [.privileged info.moduleId { sid₀ with typeArgs := tys } instr]) hp hsf
    (fun hn => ?_) (by simp)
    (by simp [Event.Silent, Encapsulation.structsRespected, hmod]) (by simp [hstk])
    (by simp [hstk, Value.abilities, RuntimeType.abilities, AbilitySet.primitives])
    (primitive_plain rfl) (by simpa [advance, push] using ho)
  rw [ho]
  rcases hi with ⟨rfl, rfl⟩ | ⟨i, x, rfl, hx, rfl, hta⟩
  · have h0 := definedStruct?_typeArgs hdef
    have hsid₀ : ({ sid₀ with typeArgs := [] } : StructId) = sid₀ := by
      cases sid₀; simp_all
    simp [Shadow.exec, guard, hn, hdef, Shadow.topAfter, advance, push, replaced, hsid₀]
  · have hm := (function?_lookup hinfo).1
    simp [Shadow.exec, guard, hn, hdef, hm, hx, hta, Shadow.topAfter, advance, push, replaced]

theorem moveFrom_annot {instr : Bytecode} {d : StructDefinitionIndex} {tys : List RuntimeType}
    (hi : instr = .moveFrom d ∨ ∃ i, instr = .moveFromGeneric i)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (hinfo : P.function? fr.function = some info)
    (h : stepOf (moveResourceFrom P s fr rest d tys) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨sid, hsid, h⟩ := done_bind h hcont globalType_ends
  have hmod := globalType_module hsid hinfo
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h hcont popAddress_ends
  obtain ⟨st, hstk, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, -, h⟩ := done_bind h hcont unlocked_ends
  split at h
  · next v hget =>
    have ho := done_finish rfl h
    obtain ⟨htype, k, t, hk, htk, hG⟩ := moveFrom_ghost (π := π) (A := A) (auth := auth)
      (evs := [.accessed .write ⟨.global a sid, []⟩ none, .privileged info.moduleId sid instr])
      (s₂ := (({ s with frames := { fr with stack := st } :: rest, storage := s.storage.remove a sid } : State).invalidate (.removed (.global a sid))))
      hp hsf rfl hstk hget rfl rfl rfl
      (fun e he => by
        simp only [List.mem_cons, List.not_mem_nil, or_false] at he
        rcases he with rfl | rfl <;> simp [Event.Silent, Encapsulation.structsRespected, hmod])
    subst ho
    refine ⟨_, _, ?_, hG⟩
    simp only at htk
    rcases hi with rfl | ⟨_, rfl⟩ <;>
      simp [Shadow.exec, hstk, Shadow.topAfter, advance, pushAll, htype, hk, htk]
  · exact (done_fail h hcont).elim

theorem moveTo_annot {instr : Bytecode} {d : StructDefinitionIndex} {tys : List RuntimeType}
    (hi : instr = .moveTo d ∨ ∃ i, instr = .moveToGeneric i)
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (hinfo : P.function? fr.function = some info)
    (h : stepOf (moveResourceTo P s fr rest d tys) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨sid, hsid, h⟩ := done_bind h hcont globalType_ends
  have hmod := globalType_module hsid hinfo
  obtain ⟨⟨v, fr₁⟩, hpv, h⟩ := done_bind h hcont pop_ends
  obtain ⟨st₁, hst₁, rfl⟩ := pop_ok hpv
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
  obtain ⟨st, hst₂, rfl, -, -⟩ := popRefTo_ok hpr
  simp only at hst₂
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  next hconf =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hconf
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨sv, hsv, h⟩ := done_bind h hcont deref_ends
  split at h
  · next a =>
    obtain ⟨_, -, h⟩ := done_bind h hcont unlocked_ends
    split at h
    · exact (done_fail h hcont).elim
    · next hnone =>
      have hget : s.storage.get? a sid = none := by simpa using hnone
      have hstk : fr.stack = v :: .ref r :: st := by rw [hst₁, hst₂]
      have hfin := done_finish (s₁ := { ({ s with frames := { fr with stack := st } :: rest } : State) with
        storage := s.storage.insert a sid v }) rfl h
      have hd := deref_ok hsv
      rw [deref_eq (s' := { s with frames := { fr with stack := st } :: rest }) (s := s)
        (by simp [hsf]) rfl rfl rfl] at hd
      obtain ⟨-, tr, htr, -⟩ := stack_key_ok hp hsf (k := 1) (r := r) (by simp [hstk])
      obtain ⟨tv, tr', ts, hgst, hG⟩ := moveTo_ghost (π := π) (A := A) (auth := auth)
        (evs := [.accessed .read r.location (some tr.id), .accessed .write ⟨.global a sid, []⟩ none,
          .used tr.id r, .privileged info.moduleId sid instr])
        (s₂ := ({ ({ s with frames := { fr with stack := st } :: rest } : State) with storage := s.storage.insert a sid v } : State))
        hp hsf rfl hstk hget (conforms_type' hconf rfl) rfl rfl rfl
        (fun e he => by
          simp only [List.mem_cons, List.not_mem_nil, or_false] at he
          rcases he with rfl | rfl | rfl | rfl <;>
            simp [Event.Silent, Encapsulation.structsRespected, hmod])
      have htr' : tr' = tr := by simpa [hgst] using htr
      subst htr'
      subst hfin
      refine ⟨_, _, ?_, hG⟩
      rcases hi with rfl | ⟨_, rfl⟩ <;> simp [Shadow.exec, hstk, hgst, conforms_type' hconf rfl, hd]
  · exact (done_wrong h hcont).elim

theorem callClosure_annot_of {s₂ : State} {c : Value} {st : List Value} {sig : SignatureIndex}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hstk0 : fr.stack = c :: st) {target : FunctionRef} {mask : BitVec 64} {cs : List Value}
    {cps crs : List RuntimeType} {ca : AbilitySet} (hc : c = .closure target mask cs cps crs ca)
    {i : Nat} {cts gst : List IdTree} (hgst : g.stack = .node i cts :: gst)
    {k : Nat} (hk : k = cps.length) (hn : k ≤ st.length) (callee : Frame) (m : Nat)
    (hloc : callee.locals = ((maskCompose mask cs (st.take k).reverse).map some ++
      List.replicate m none).toArray)
    (hstk : callee.stack = []) (hid : callee.id = s.nextFrame)
    (h2f : s₂.frames = callee :: { fr with stack := st.drop k } :: rest)
    (h2st : s₂.storage = s.storage) (h2he : s₂.held = s.held)
    (h2nf : s₂.nextFrame = s.nextFrame + 1)
    (hex : ∃ evs, Shadow.exec P info.moduleId (.callClosure sig) s fr (.running s₂)
        ⟨g :: hs, hst, hhe, f⟩ =
      some (.running ⟨{ locals := ((maskCompose mask cts (gst.take k).reverse).map some ++
          List.replicate (callee.locals.size - (maskCompose mask cts (gst.take k).reverse).length)
            none).toArray, stack := [] } :: { g with stack := gst.drop k } :: hs, hst, hhe, f⟩,
        evs ++ [.consumed i c (.callClosure info.moduleId)]) ∧ Plain4 evs) :
    Annotated σ P π A auth info.moduleId (.callClosure sig) s fr (.running s₂) ⟨g :: hs, hst, hhe, f⟩ := by
  obtain ⟨evs, hex, hevs⟩ := hex
  exact ⟨_, _, hex, callClosure_ghost (π := π) (A := A) (auth := auth) hp hsf rfl hstk0 hc hgst hk hn
    callee m hloc hstk hid h2f h2st h2he h2nf (plain4_ok hevs) info.moduleId⟩

/-- `CallClosure` consumes the closure on top, and moves its captured values
and the provided arguments into the callee's first locals. -/
theorem callClosure_annot {sig : SignatureIndex}
    (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩) (hsf : s.frames = fr :: rest)
    (hcont : o.continues = true) (_hinfo : P.function? fr.function = some info)
    (h : stepOf (Step.callClosure P s fr rest info sig) = .done o) :
    Annotated σ P π A auth info.moduleId (.callClosure sig) s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  have hg := (pre_head hp hsf).1
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
    obtain ⟨st, hstk0, rfl⟩ := pop_ok hpop
    dsimp only at h
    split at h
    · next target mask captured cps crs ca =>
      split at h
      · exact (done_wrong h hcont).elim
      next htys =>
      have hcps : cps = ps := by
        simp only [Bool.or_eq_true, bne_iff_ne, ne_eq, not_or, Decidable.not_not] at htys
        exact htys.1.1
      obtain ⟨⟨provided, fr₂⟩, hpop2, h⟩ := done_bind h hcont popN_ends
      obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop2
      dsimp only at h
      split at h
      · exact (done_wrong h hcont).elim
      obtain ⟨targetInfo, -, h⟩ := done_bind h hcont (ends_need _ _)
      split at h
      · exact (done_wrong h hcont).elim
      obtain ⟨paramTys, -, h⟩ := done_bind h hcont (ends_need _ _)
      obtain ⟨retTys, -, h⟩ := done_bind h hcont (ends_need _ _)
      split at h
      · exact (done_wrong h hcont).elim
      obtain ⟨localTys, -, h⟩ := done_bind h hcont (ends_need _ _)
      have hfa := frame_stack_fits hg
      rw [hstk0] at hfa
      obtain ⟨ct, gst, hgst, -, hgsf⟩ := fitsAll_cons_right hfa
      obtain ⟨i, cts⟩ := ct
      have hglen := fitsAll_length hgsf
      have hn' : ps.length ≤ st.length := hn
      have hk : st.length - (st.length - ps.length) = ps.length := by omega
      split at h <;> split at h
      all_goals first
        | exact (done_fail h hcont).elim
        | skip
      all_goals
        simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
        subst h
        refine callClosure_annot_of hp hsf hstk0 rfl hgst (k := ps.length) (by rw [hcps]) hn' _ _ rfl
          rfl ?_ rfl ?_ ?_ ?_ ⟨?evs, ?hex, ?hpl⟩
        case hex =>
          simp only [Shadow.exec, hstk0, hgst, List.head?_cons, List.length_cons, List.length_drop,
            Nat.add_sub_cancel, hk, guard, hglen, if_pos hn']
          rw [show ∀ (l : List Event) (a b : Event), l ++ [a, b] = (l ++ [a]) ++ [b] from
            fun l a b => by simp]
          rfl
        case hpl => plain4
        all_goals first
          | rfl
          | (split <;> rfl)
    · exact (done_wrong h hcont).elim
  · exact (done_wrong h hcont).elim

/-- **Every instruction that continues is annotated**, and its annotation
establishes `StepGhost`. -/
theorem exec_annot {instr : Bytecode} (hp : Pre σ P s ⟨g :: hs, hst, hhe, f⟩)
    (hsf : s.frames = fr :: rest) (hcont : o.continues = true)
    (hret : instr = .ret → RetOk P s o) (hinfo : P.function? fr.function = some info)
    (hcode : info.definition.code = some code) (hin : instr ∈ code.code.toList)
    (h : stepOf (Step.exec P s fr rest info code instr) = .done o) :
    Annotated σ P π A auth info.moduleId instr s fr o ⟨g :: hs, hst, hhe, f⟩ := by
  cases instr
  case pop => exact pop_annot hp hsf hcont h
  case ret => exact ret_annot hp hsf hcont (hret rfl) h
  case brTrue => exact brTrue_annot hp hsf hcont h
  case brFalse => exact brFalse_annot hp hsf hcont h
  case branch => exact branch_annot hp hsf hcont h
  case nop => exact nop_annot hp hsf h
  case ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8 | ldI16 | ldI32 | ldI64 | ldI128
    | ldI256 | ldTrue | ldFalse =>
    exact lit_annot hp hsf (primitive_plain rfl)
      (by simp [Shadow.exec, guard, Shadow.topAfter, replaced]) h
  case ldConst => exact ldConst_annot hp hsf hcont h
  case castU8 | castU16 | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32
    | castI64 | castI128 | castI256 =>
    exact replace_bind hp hsf hcont
      (fun _ _ hn => by simp [Shadow.exec, guard, Shadow.topAfter, replaced, hn]) cast_ends
      (fun _ hx => cast_plain hx) h
  case add | sub | mul | div | mod | bitOr | bitAnd | xor =>
    exact replace_bind hp hsf hcont
      (fun _ _ hn => by simp [Shadow.exec, guard, Shadow.topAfter, replaced, hn]) intBinop_ends
      (fun _ hx => intBinop_plain hx) h
  case shl | shr =>
    exact replace_bind hp hsf hcont
      (fun _ _ hn => by simp [Shadow.exec, guard, Shadow.topAfter, replaced, hn]) shift_ends
      (fun _ hx => shift_plain hx) h
  case or | and =>
    exact replace_bind hp hsf hcont
      (fun _ _ hn => by simp [Shadow.exec, guard, Shadow.topAfter, replaced, hn]) boolBinop_ends
      (fun _ hx => boolBinop_plain hx) h
  case lt | gt | le | ge =>
    exact replace_bind hp hsf hcont
      (fun _ _ hn => by simp [Shadow.exec, guard, Shadow.topAfter, replaced, hn]) intCompare_ends
      (fun _ hx => intCompare_plain hx) h
  case eq | neq =>
    refine equality_annot hp hsf hcont (fun _ _ hn => ?_) h
    simp [Shadow.exec, guard, Shadow.topAfter, replaced, hn]
    split
    · exact ⟨[_, _, _, _], rfl, by plain4⟩
    · exact ⟨[], by simp, by plain4⟩
  case not => exact not_annot hp hsf hcont h
  case negate => exact negate_annot hp hsf hcont h
  case copyLoc => exact copyLoc_annot hp hsf hcont h
  case moveLoc => exact moveLoc_annot hp hsf hcont h
  case stLoc => exact stLoc_annot hp hsf hcont h
  case call => exact call_annot (Or.inl ⟨_, rfl⟩) hp hsf hcont h
  case callGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact call_annot (Or.inr ⟨i, rfl⟩) hp hsf hcont h
  case pack => exact pack_annot (Or.inl rfl) hp hsf hcont hinfo h
  case packGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact pack_annot (Or.inr ⟨i, rfl⟩) hp hsf hcont hinfo h
  case packVariant => exact packVariant_annot (Or.inl rfl) hp hsf hcont hinfo h
  case packVariantGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact packVariant_annot (Or.inr ⟨i, rfl⟩) hp hsf hcont hinfo h
  case vecPack => exact vecPack_annot hp hsf hcont h
  case unpack => exact unpack_annot (Or.inl rfl) hp hsf hcont hinfo hcode hin h
  case unpackGeneric i =>
    obtain ⟨x, hx, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact unpack_annot (Or.inr ⟨i, x, rfl, need_ok hx, rfl⟩) hp hsf hcont hinfo hcode hin h
  case unpackVariant => exact unpackVariant_annot (Or.inl rfl) hp hsf hcont hinfo h
  case unpackVariantGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact unpackVariant_annot (Or.inr ⟨i, rfl⟩) hp hsf hcont hinfo h
  case vecUnpack => exact vecUnpack_annot hp hsf hcont h
  case abort => exact (abort_absurd hcont h).elim
  case abortMsg => exact (abortMsg_absurd hcont h).elim
  case readRef => exact readRef_annot hp hsf hcont h
  case writeRef => exact writeRef_annot hp hsf hcont h
  case freezeRef => exact freezeRef_annot hp hsf hcont h
  case mutBorrowLoc => exact borrowLoc_annot (Or.inl ⟨rfl, rfl⟩) hp hsf hcont h
  case immBorrowLoc => exact borrowLoc_annot (Or.inr ⟨rfl, rfl⟩) hp hsf hcont h
  case mutBorrowField => exact borrowField_annot (Or.inl ⟨Or.inl rfl, rfl⟩) hp hsf hcont hinfo h
  case immBorrowField => exact borrowField_annot (Or.inr ⟨Or.inl rfl, rfl⟩) hp hsf hcont hinfo h
  case mutBorrowFieldGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact borrowField_annot (Or.inl ⟨Or.inr ⟨i, rfl⟩, rfl⟩) hp hsf hcont hinfo h
  case immBorrowFieldGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact borrowField_annot (Or.inr ⟨Or.inr ⟨i, rfl⟩, rfl⟩) hp hsf hcont hinfo h
  case mutBorrowVariantField =>
    exact borrowVariantField_annot (Or.inl ⟨Or.inl rfl, rfl⟩) hp hsf hcont hinfo h
  case immBorrowVariantField =>
    exact borrowVariantField_annot (Or.inr ⟨Or.inl rfl, rfl⟩) hp hsf hcont hinfo h
  case mutBorrowVariantFieldGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact borrowVariantField_annot (Or.inl ⟨Or.inr ⟨i, rfl⟩, rfl⟩) hp hsf hcont hinfo h
  case immBorrowVariantFieldGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact borrowVariantField_annot (Or.inr ⟨Or.inr ⟨i, rfl⟩, rfl⟩) hp hsf hcont hinfo h
  case testVariant => exact testVariant_annot (Or.inl rfl) hp hsf hcont hinfo h
  case testVariantGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact testVariant_annot (Or.inr ⟨i, rfl⟩) hp hsf hcont hinfo h
  case vecLen => exact vecLen_annot hp hsf hcont h
  case vecImmBorrow => exact borrowElement_annot (Or.inr ⟨rfl, rfl⟩) hp hsf hcont h
  case vecMutBorrow => exact borrowElement_annot (Or.inl ⟨rfl, rfl⟩) hp hsf hcont h
  case vecPushBack => exact vecPushBack_annot hp hsf hcont h
  case vecPopBack => exact vecPopBack_annot hp hsf hcont h
  case vecSwap => exact vecSwap_annot hp hsf hcont h
  case exists_ => exact exists_annot (Or.inl ⟨rfl, rfl⟩) hp hsf hcont hinfo h
  case existsGeneric i =>
    obtain ⟨x, hx, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, htys, h⟩ := done_bind h hcont genericArgs_ends
    exact exists_annot (Or.inr ⟨i, x, rfl, need_ok hx, rfl, genericArgs_ok htys⟩) hp hsf hcont
      hinfo h
  case moveFrom => exact moveFrom_annot (Or.inl rfl) hp hsf hcont hinfo h
  case moveFromGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact moveFrom_annot (Or.inr ⟨i, rfl⟩) hp hsf hcont hinfo h
  case moveTo => exact moveTo_annot (Or.inl rfl) hp hsf hcont hinfo h
  case moveToGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact moveTo_annot (Or.inr ⟨i, rfl⟩) hp hsf hcont hinfo h
  case mutBorrowGlobal => exact borrowGlobal_annot (Or.inl ⟨Or.inl rfl, rfl⟩) hp hsf hcont hinfo h
  case immBorrowGlobal => exact borrowGlobal_annot (Or.inr ⟨Or.inl rfl, rfl⟩) hp hsf hcont hinfo h
  case mutBorrowGlobalGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact borrowGlobal_annot (Or.inl ⟨Or.inr ⟨i, rfl⟩, rfl⟩) hp hsf hcont hinfo h
  case immBorrowGlobalGeneric i =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact borrowGlobal_annot (Or.inr ⟨Or.inr ⟨i, rfl⟩, rfl⟩) hp hsf hcont hinfo h
  case packClosure hd mask => exact packClosure_annot (Or.inl rfl) hp hsf hcont h
  case packClosureGeneric i mask =>
    obtain ⟨x, -, h⟩ := done_bind h hcont (ends_need _ _)
    obtain ⟨tys, -, h⟩ := done_bind h hcont genericArgs_ends
    exact packClosure_annot (Or.inr ⟨i, rfl⟩) hp hsf hcont h
  case callClosure sig => exact callClosure_annot hp hsf hcont hinfo h
  all_goals exact (done_wrong h hcont).elim

end Instructions

/-! ## Core steps -/

/-- **A core step that continues is annotated**, from a state that satisfies
`Pre` with its shadow, and its annotation establishes `StepGhost`. -/
theorem coreStep_annot {P : Program} {π : NativePolicy} {A : List StructId}
    {auth : List Address} {s : State} {h : Shadow} {o : Outcome} (hp : Pre σ P s h)
    (hstep : step P s = .done o) (hcont : o.continues = true) (hret : RetOk P s o) :
    ∃ post evs, Shadow.coreStep P s o h = some (post, evs) ∧
      StepGhost σ P π A auth (h.occurrences s) (framesOcc h.frames s.frames) (h.frames.zip s.frames) h.fresh o post evs := by
  obtain ⟨hframes, hst, hhe, fresh⟩ := h
  have hfit := hp.fits
  unfold step at hstep
  split at hstep
  · next hsf =>
    simp only [StepResult.done.injEq] at hstep
    subst hstep
    simp [Outcome.continues] at hcont
  · next fr rest hsf =>
    cases hframes with
    | nil => rw [shadow_fits, hsf] at hfit; simp [framesFit] at hfit
    | cons g hs =>
      split at hstep
      · simp only [StepResult.done.injEq] at hstep
        subst hstep
        simp [Outcome.continues] at hcont
      · next info hinfo =>
        split at hstep
        · simp only [StepResult.done.injEq] at hstep
          subst hstep
          simp [Outcome.continues] at hcont
        · next code hcode =>
          split at hstep
          · simp only [StepResult.done.injEq] at hstep
            subst hstep
            simp [Outcome.continues] at hcont
          · next instr hinstr =>
            have hstep' : stepOf (Step.exec P s fr rest info code instr) = .done o := hstep
            obtain ⟨post, evs, hex, hgh⟩ := exec_annot (π := π) (A := A) (auth := auth)
              hp hsf hcont (fun _ => hret) hinfo hcode
              (Array.mem_toList_iff.2 (Array.mem_of_getElem? hinstr)) hstep'
            refine ⟨post, evs, ?_, hgh⟩
            simp [Shadow.coreStep, hsf, hinfo, hcode, hinstr, hex, hgh.fits]

/-- **A core step that continues has an annotation**, from a state that
satisfies `Pre`, whatever the values the transaction holds (`ShadowAccounts`
needs only this). -/
theorem coreStep_exists {P : Program} {s : State} {h : Shadow} {o : Outcome} (hp : Pre σ P s h)
    (hstep : step P s = .done o) (hcont : o.continues = true) :
    ∃ post evs, Shadow.coreStep P s o h = some (post, evs) := by
  obtain ⟨hframes, hst, hhe, fresh⟩ := h
  have hfit := hp.fits
  unfold step at hstep
  split at hstep
  · next hsf =>
    simp only [StepResult.done.injEq] at hstep
    subst hstep
    simp [Outcome.continues] at hcont
  · next fr rest hsf =>
    cases hframes with
    | nil => rw [shadow_fits, hsf] at hfit; simp [framesFit] at hfit
    | cons g hs =>
      split at hstep
      · simp only [StepResult.done.injEq] at hstep
        subst hstep
        simp [Outcome.continues] at hcont
      · next info hinfo =>
        split at hstep
        · simp only [StepResult.done.injEq] at hstep
          subst hstep
          simp [Outcome.continues] at hcont
        · next code hcode =>
          split at hstep
          · simp only [StepResult.done.injEq] at hstep
            subst hstep
            simp [Outcome.continues] at hcont
          · next instr hinstr =>
            have hstep' : stepOf (Step.exec P s fr rest info code instr) = .done o := hstep
            have hex : ∃ post evs, Shadow.exec P info.moduleId instr s fr o ⟨g :: hs, hst, hhe, fresh⟩ =
                some (post, evs) ∧ post.fits o = true := by
              by_cases hr : instr = .ret
              · subst hr
                exact ret_exists hp hsf hcont hstep'
              · obtain ⟨post, evs, hex, hgh⟩ := exec_annot (π := {}) (A := []) (auth := [])
                  hp hsf hcont (fun h => absurd h hr) hinfo hcode
                  (Array.mem_toList_iff.2 (Array.mem_of_getElem? hinstr)) hstep'
                exact ⟨post, evs, hex, hgh.fits⟩
            obtain ⟨post, evs, hex, hfits⟩ := hex
            refine ⟨post, evs, ?_⟩
            simp [Shadow.coreStep, hsf, hinfo, hcode, hinstr, hex, hfits]

/-- An annotated core step continues: the annotation fits only `running`
and `returned` outcomes. -/
theorem coreStep_continues {P : Program} {s : State} {h : Shadow} {o : Outcome}
    {post : ShadowPost} {evs : List Event} (hc : Shadow.coreStep P s o h = some (post, evs)) :
    o.continues = true := by
  unfold Shadow.coreStep at hc
  split at hc
  · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at hc
    obtain ⟨_, -, _, -, _, -, ⟨post', evs'⟩, -, hc⟩ := hc
    split at hc
    · next hf =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hc
      obtain ⟨rfl, -⟩ := hc
      revert hf
      cases post' <;> cases o <;> simp [ShadowPost.fits, Outcome.continues]
    · simp at hc
  · simp at hc

end MoveBytecode.Proofs
