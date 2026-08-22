-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Interp.Exec

/-!
# Interpreter Correctness

The executable interpreter represents memory and locals with finite lists;
the relational semantics represents them with functions.  `IMem.denote` and
`ILocals.denote` bridge those representations.  The main theorem below shows
that every successful interpreter result is a relational execution.  Thus
evaluation cannot manufacture behavior absent from the declarative semantics.
-/

namespace MoveModel.IR

/-- Interpret executable frames as semantic frames. -/
def IFrames.denote (frames : IFrames) : FrameStore :=
  fun frame => (frames.get frame).denote

/-- Interpret an executable state as a semantic state. -/
def IState.denote (s : IState) : MoveState :=
  { current := s.current, frames := s.frames.denote, memory := s.memory.denote }

/-- Interpret an executable return world as a semantic return world. -/
def IWorld.denote (world : IWorld) : FrameWorld :=
  { frames := world.frames.denote, memory := world.memory.denote }

/-- Interpret an executable outcome as a relational-semantics outcome. -/
def IOutcome.denote : IOutcome → FrameOutcome
  | .ret world vals => .ret world.denote vals
  | .abort m code => .abort m.denote code

/-! ## Representation lemmas -/

@[simp] theorem IMem.get_cons (m : IMem) (r r' : ResourceKey)
    (a a' : Address) (v : Value) :
    IMem.get ((r, a, v) :: m) r' a' =
      if r = r' ∧ a = a' then some v else m.get r' a' := by
  by_cases hr : r = r' <;> by_cases ha : a = a' <;>
    simp_all [IMem.get]

/-- Executable lookup after removal agrees with deleting the selected key. -/
theorem IMem.get_remove (m : IMem) (r r' : ResourceKey)
    (a a' : Address) :
    (m.remove r a).get r' a' =
      if r' = r ∧ a' = a then none else m.get r' a' := by
  induction m with
  | nil => simp [IMem.remove, IMem.get]
  | cons e m ih =>
    rcases e with ⟨r₀, a₀, v⟩
    simp only [IMem.remove]
    split <;> rename_i hstored
    · rw [ih]
      simp only [Bool.and_eq_true, beq_iff_eq] at hstored
      rcases hstored with ⟨rfl, rfl⟩
      by_cases hr : r' = r₀ <;> by_cases ha : a' = a₀ <;>
        simp_all [IMem.get_cons, eq_comm]
    · rw [IMem.get_cons, ih, IMem.get_cons]
      simp only [Bool.and_eq_true, beq_iff_eq] at hstored
      by_cases hr : r' = r <;> by_cases ha : a' = a <;>
        by_cases hr₀ : r₀ = r' <;> by_cases ha₀ : a₀ = a' <;>
        simp_all [eq_comm]

/-- Denotation maps executable insertion to semantic memory insertion. -/
@[simp] theorem IMem.denote_set (m : IMem) (r : ResourceKey) (a : Address)
    (v : Value) :
    (m.set r a v).denote = memWrite m.denote r a v := by
  funext r' a'
  by_cases h : r' = r ∧ a' = a
  · rcases h with ⟨rfl, rfl⟩
    simp [IMem.denote, IMem.set, memWrite, IMem.get_cons]
  · have h' : ¬(r = r' ∧ a = a') := by
      intro h'
      exact h ⟨h'.1.symm, h'.2.symm⟩
    simp [IMem.denote, IMem.set, memWrite, h, h', IMem.get_cons,
      IMem.get_remove]

/-- Denotation maps executable removal to semantic memory removal. -/
@[simp] theorem IMem.denote_remove (m : IMem) (r : ResourceKey)
    (a : Address) :
    (m.remove r a).denote = memRemove m.denote r a := by
  funext r' a'
  simp only [IMem.denote, IMem.get_remove, memRemove]

/-- Characterize executable local lookup after a local write. -/
theorem ILocals.get_set (l : ILocals) (i : LocalIndex) (v : Value)
    (j : LocalIndex) :
    (l.set i v).get j = if j = i then some v else l.get j := by
  induction i generalizing l j with
  | zero => cases l <;> cases j <;> simp [ILocals.set, ILocals.get]
  | succ i ih =>
    cases l with
    | nil =>
      cases j with
      | zero => simp [ILocals.set, ILocals.get]
      | succ j =>
        simp only [ILocals.set, ILocals.get, List.getElem?_cons_succ]
        simpa [ILocals.get] using ih (l := []) (j := j)
    | cons x l =>
      cases j with
      | zero => simp [ILocals.set, ILocals.get]
      | succ j =>
        simp only [ILocals.set, ILocals.get, List.getElem?_cons_succ]
        simpa [ILocals.get] using ih (l := l) (j := j)

/-- Denotation maps executable local writes to semantic local updates. -/
@[simp] theorem ILocals.denote_set (l : ILocals) (i : LocalIndex)
    (v : Value) :
    (l.set i v).denote = fun j => if j = i then some v else l.denote j := by
  funext j
  simp [ILocals.denote, ILocals.get_set]

/-- Characterize executable frame lookup after replacing one frame. -/
theorem IFrames.get_set (frames : IFrames) (frame : FrameId)
    (locals : ILocals) (other : FrameId) :
    (frames.set frame locals).get other =
      if other = frame then locals else frames.get other := by
  induction frame generalizing frames other with
  | zero => cases frames <;> cases other <;> simp [IFrames.set, IFrames.get]
  | succ frame ih =>
    cases frames with
    | nil =>
      cases other with
      | zero => simp [IFrames.set, IFrames.get]
      | succ other =>
        simpa [IFrames.set, IFrames.get] using
          ih (frames := []) (other := other)
    | cons head tail =>
      cases other with
      | zero => simp [IFrames.set, IFrames.get]
      | succ other =>
        simpa [IFrames.set, IFrames.get] using
          ih (frames := tail) (other := other)

/-- Denotation maps executable frame writes to semantic frame replacement. -/
@[simp] theorem IFrames.denote_set (frames : IFrames) (frame : FrameId)
    (locals : ILocals) :
    (frames.set frame locals).denote =
      setFrame frames.denote frame locals.denote := by
  funext other
  simp only [IFrames.denote]
  rw [IFrames.get_set]
  simp only [setFrame]
  split <;> rfl

/-- A list of executable argument locals denotes semantic initialized locals. -/
@[simp] theorem ILocals.denote_args (args : List Value) :
    ILocals.denote (args.map some) = initLocals args := by
  funext idx
  cases h : args[idx]? <;>
    simp [ILocals.denote, ILocals.get, initLocals, h]

/-- Empty executable locals denote an entirely uninitialized local store. -/
@[simp] theorem ILocals.denote_nil : (ILocals.denote []) = fun _ => none := by
  rfl

/-- Executable and semantic current-local lookup agree under denotation. -/
theorem IState.denote_getLocal (s : IState) (idx : LocalIndex) :
    s.getLocal idx = s.denote.locals idx := by
  rfl

/-- Mapping executable local lookup agrees with mapping semantic lookup. -/
theorem IState.mapM_getLocal (s : IState) (idxs : List LocalIndex) :
    idxs.mapM s.getLocal = idxs.mapM s.denote.locals := by
  rfl

/-- A successful executable local lookup is the same semantic lookup. -/
theorem IState.getLocal_some_denote {s : IState} {idx : LocalIndex}
    {v : Value} (h : s.getLocal idx = some v) :
    s.denote.locals idx = some v :=
  (s.denote_getLocal idx).symm.trans h

/-- Successful executable multi-local lookup is the same semantic lookup. -/
theorem IState.mapM_getLocal_some_denote {s : IState}
    {idxs : List LocalIndex} {vs : List Value}
    (h : idxs.mapM s.getLocal = some vs) :
    idxs.mapM s.denote.locals = some vs :=
  (s.mapM_getLocal idxs).symm.trans h

/-- Denotation commutes with writing one current local. -/
@[simp] theorem IState.denote_writeLocal (s : IState) (idx : LocalIndex)
    (v : Value) :
    (s.writeLocal idx v).denote = s.denote.writeLocal idx v := by
  apply MoveState.ext
  · rfl
  · simp only [IState.writeLocal, IState.setLocals, IState.denote,
      MoveState.writeLocal, IState.locals, MoveState.locals,
      IFrames.denote_set]
    congr 2
    funext j
    simp [ILocals.denote_set, IFrames.denote]
  · rfl

/-- Denotation commutes with writing multiple current locals. -/
@[simp] theorem IState.denote_writeLocals (s : IState)
    (idxs : List LocalIndex) (vs : List Value) :
    (s.writeLocals idxs vs).denote = s.denote.writeLocals idxs vs := by
  induction idxs generalizing s vs with
  | nil => cases vs <;> rfl
  | cons idx idxs ih =>
    cases vs with
    | nil => rfl
    | cons v vs =>
      simp only [IState.writeLocals, MoveState.writeLocals]
      rw [ih, IState.denote_writeLocal]

/-- Denotation commutes with replacing global memory. -/
@[simp] theorem IState.denote_setMemory (s : IState) (m : IMem) :
    (s.setMemory m).denote = s.denote.setMemory m.denote := by
  rfl

/-- Executable and semantic initial states agree under denotation. -/
@[simp] theorem IState.denote_initial (args : List Value) (m : IMem) :
    (IState.initial args m).denote = MoveState.initial args m.denote := by
  simp only [IState.initial, IState.denote, MoveState.initial]
  apply MoveState.ext
  · rfl
  · change IFrames.denote [args.map some] =
      setFrame emptyFrames 0 (initLocals args)
    funext frame
    cases frame with
    | zero =>
      simpa [IFrames.denote, IFrames.get, setFrame] using
        ILocals.denote_args args
    | succ frame => rfl
  · rfl

/-- Denotation commutes with entering a callee frame. -/
@[simp] theorem IState.denote_enterCall (s : IState) (args : List Value) :
    (s.enterCall args).denote = s.denote.enterCall args := by
  apply MoveState.ext
  · rfl
  · simp only [IState.enterCall, IState.denote, MoveState.enterCall,
      IFrames.denote_set]
    congr 2
    exact ILocals.denote_args args
  · rfl

/-- Denotation commutes with retiring the current frame. -/
@[simp] theorem IState.denote_finishFrame (s : IState) :
    s.finishFrame.denote = s.denote.finishFrame := by
  apply FrameWorld.ext
  · simp [IState.finishFrame, IWorld.denote, MoveState.finishFrame,
      IState.denote]
  · rfl

/-- Denotation commutes with resuming a returned caller frame. -/
@[simp] theorem IWorld.denote_resume (world : IWorld) (caller : FrameId) :
    (world.resume caller).denote = world.denote.resume caller := by
  rfl

/-- Executable and semantic reference reads agree under denotation. -/
theorem readTargetI_eq (s : IState) (t : RefTarget) :
    readTargetI s t = s.denote.readTarget t := by
  rcases t with ⟨root, path⟩
  cases root <;> rfl

/-- A successful executable reference read is the corresponding semantic read. -/
theorem readTargetI_some_denote {s : IState} {t : RefTarget} {v : Value}
    (h : readTargetI s t = some v) : s.denote.readTarget t = some v :=
  (readTargetI_eq s t).symm.trans h

/-- Successful executable reference writes denote semantic reference writes. -/
theorem writeTargetI_eq (s : IState) (t : RefTarget) (v : Value) :
    Option.map IState.denote (writeTargetI s t v) =
      s.denote.writeTarget t v := by
  rcases t with ⟨root, path⟩
  cases root with
  | loc frame idx =>
    cases hget : (s.frames.get frame).get idx with
    | none =>
      simp [writeTargetI, MoveState.writeTarget, IState.denote,
        IFrames.denote, ILocals.denote, hget]
    | some root =>
      cases hset : root.setPath path v with
      | none =>
        simp [writeTargetI, MoveState.writeTarget, IState.denote,
          IFrames.denote, ILocals.denote, hget, hset]
      | some root' =>
        have hget' : s.frames.denote frame idx = some root := hget
        simp only [writeTargetI, hget, Option.bind_some, hset,
          Option.map_some, Option.map, MoveState.writeTarget, hget',
          IState.denote, IFrames.denote_set]
        apply congrArg some
        apply MoveState.ext
        · rfl
        · apply congrArg (setFrame s.frames.denote frame)
          exact ILocals.denote_set (s.frames.get frame) idx root'
        · rfl
  | global r a =>
    cases hget : s.memory.get r a <;>
      simp [writeTargetI, MoveState.writeTarget, IState.denote,
        IMem.denote, hget]
    rename_i root
    cases hset : root.setPath path v <;>
      simp [writeTargetI, MoveState.writeTarget, IState.denote,
        IMem.denote, hget, hset, IMem.denote_set, MoveState.writeGlobal]

/-! ## Primitive-operation agreement -/

/-- Interpret the executable result of a primitive operation. -/
def IOpRes.denote : IOpRes → OpOutcome
  | .ok rets m => .ok rets m.denote
  | .abort => .abort

/-- Executable primitive-operation results agree with relational operation semantics. -/
theorem interpOp_sound {s : IState} {op : Oper}
    {vs : List Value} {r : IOpRes}
    (h : interpOp s.current (readTargetI s) op vs s.memory = .ok r) :
    op.sem s.denote.current s.denote.readTarget vs s.memory.denote =
      some r.denote := by
  have hderef : readTargetI s = s.denote.readTarget := by
    funext t
    exact readTargetI_eq s t
  rw [hderef] at h
  cases op <;> simp only [interpOp] at h <;>
    repeat first | split at h
  all_goals
    try simp_all [pure, Except.pure, Oper.sem, NumType.checked, NumType.bitwise,
      IOpRes.denote, IState.denote, IMem.denote]
  all_goals repeat first | split at h
  all_goals try simp only [Except.ok.injEq] at h
  all_goals try subst r
  all_goals
    simp_all [NumType.checked, NumType.bitwise, IMem.denote_set, IMem.denote_remove]

/-- The one-source/one-destination fallback preserves instruction soundness. -/
theorem interp_oneOne_fallback (P : Program) (fuel : Nat)
    (rest : List Instr) (s : IState) (dsts srcs : List Nat)
    (op : Oper)
    (hop : op = .borrowLoc ∨ (∃ i, op = .borrowField i) ∨
      (∃ r, op = .borrowGlobal r) ∨ op = .readRef ∨ op = .freezeRef)
    (hshape : ¬ ∃ dst src, dsts = [dst] ∧ srcs = [src]) :
    interpInstrs P (fuel + 1) (.call dsts op srcs :: rest) s =
      interpGeneric P fuel rest s dsts op srcs := by
  rcases hop with rfl | ⟨i, rfl⟩ | ⟨r, rfl⟩ | rfl | rfl <;>
    unfold interpInstrs <;> split <;> (try simp_all) <;> rfl

/-- Instantiated field/global borrows use the generic path when malformed. -/
theorem interp_oneOneInst_fallback (P : Program) (fuel : Nat)
    (rest : List Instr) (s : IState) (dsts srcs : List Nat)
    (op : Oper)
    (hop : (∃ i args, op = .borrowFieldInst i args) ∨
      (∃ r args, op = .borrowGlobalInst r args))
    (hshape : ¬ ∃ dst src, dsts = [dst] ∧ srcs = [src]) :
    interpInstrs P (fuel + 1) (.call dsts op srcs :: rest) s =
      interpGeneric P fuel rest s dsts op srcs := by
  rcases hop with ⟨i, args, rfl⟩ | ⟨r, args, rfl⟩ <;>
    unfold interpInstrs <;> split <;> (try simp_all) <;> rfl

/-- The two-source/one-destination fallback preserves instruction soundness. -/
theorem interp_oneTwo_fallback (P : Program) (fuel : Nat)
    (rest : List Instr) (s : IState) (dsts srcs : List Nat)
    (op : Oper) (hop : op = .borrowVecElem)
    (hshape : ¬ ∃ dst x y, dsts = [dst] ∧ srcs = [x, y]) :
    interpInstrs P (fuel + 1) (.call dsts op srcs :: rest) s =
      interpGeneric P fuel rest s dsts op srcs := by
  subst op
  unfold interpInstrs
  split <;> (try simp_all) <;> rfl

/-- The two-source/no-destination fallback preserves instruction soundness. -/
theorem interp_zeroTwo_fallback (P : Program) (fuel : Nat)
    (rest : List Instr) (s : IState) (dsts srcs : List Nat)
    (op : Oper) (hop : op = .writeRef)
    (hshape : ¬ ∃ x y, dsts = [] ∧ srcs = [x, y]) :
    interpInstrs P (fuel + 1) (.call dsts op srcs :: rest) s =
      interpGeneric P fuel rest s dsts op srcs := by
  subst op
  unfold interpInstrs
  split <;> (try simp_all) <;> rfl

/-- A malformed optional-parent guard follows the generic operation path. -/
theorem interp_isParent_fallback (P : Program) (fuel : Nat)
    (rest : List Instr) (s : IState) (dsts srcs : List Nat)
    (pat : List (Option Nat))
    (hshape : ¬ ∃ dst p t, dsts = [dst] ∧ srcs = [p, t]) :
    interpInstrs P (fuel + 1) (.call dsts (.isParent pat) srcs :: rest) s =
      interpGeneric P fuel rest s dsts (.isParent pat) srcs := by
  unfold interpInstrs
  split <;> (try simp_all) <;> rfl

/-! ## Frame-level agreement -/

def InstrResultAgrees (P : Program) (G : Cfg) (term : Term)
    (o : FrameOutcome) : IInstrsRes → Prop
  | .ok s => RunFrom P G [] term s.denote o
  | .abort m code => o = .abort m.denote code

/-- Soundness of instruction-list interpretation at a fixed fuel bound. -/
def InstrSoundAt (fuel : Nat) : Prop :=
  ∀ (P : Program) (G : Cfg) (is : List Instr) (term : Term)
    (s : IState) (r : IInstrsRes) (o : FrameOutcome),
    interpInstrs P fuel is s = .ok r →
    InstrResultAgrees P G term o r →
    RunFrom P G is term s.denote o

/-- Soundness of block interpretation at a fixed fuel bound. -/
def BlockSoundAt (fuel : Nat) : Prop :=
  ∀ (P : Program) (G : Cfg) (b : BlockId) (s : IState) (o : IOutcome),
    interpBlock P G fuel b s = .ok o →
    RunBlock P G b s.denote o.denote

/-- Soundness of function interpretation at a fixed fuel bound. -/
def FunSoundAt (fuel : Nat) : Prop :=
  ∀ (P : Program) (f : FunId) (m : IMem) (args : List Value)
    (o : IOutcome),
    interpFun P fuel f m args = .ok o →
    FunExec P f m.denote args o.denote

/-- Functions, blocks, and instruction lists are jointly sound at every fuel. -/
theorem interp_sound_at (fuel : Nat) :
    InstrSoundAt fuel ∧ BlockSoundAt fuel ∧ FunSoundAt fuel := by
  induction fuel using Nat.strongRecOn with
  | ind fuel ih =>
    have his : InstrSoundAt fuel := by
      intro P G is term s r o hexec hcont
      cases fuel with
      | zero => simp [interpInstrs] at hexec
      | succ n =>
        cases is with
        | nil =>
          simp only [interpInstrs, pure, Except.pure,
            Except.ok.injEq] at hexec
          subst r
          exact hcont
        | cons instr rest =>
          have prev := (ih n (by omega)).1
          cases instr with
          | load dst v =>
            simp only [interpInstrs] at hexec
            apply RunFrom.load
            rw [← IState.denote_writeLocal]
            exact prev P G rest term (s.writeLocal dst v) r o hexec hcont
          | assign dst src =>
            simp only [interpInstrs] at hexec
            cases hsrc : s.getLocal src with
            | none => simp only [hsrc] at hexec; cases hexec
            | some v =>
              simp only [hsrc] at hexec
              apply RunFrom.assign (IState.getLocal_some_denote hsrc)
              rw [← IState.denote_writeLocal]
              exact prev P G rest term (s.writeLocal dst v) r o hexec hcont
          | nop =>
            simp only [interpInstrs] at hexec
            exact RunFrom.nop (prev P G rest term s r o hexec hcont)
          | call dsts op srcs =>
            have generic
                (hg : interpGeneric P n rest s dsts op srcs = .ok r) :
                RunFrom P G (.call dsts op srcs :: rest) term s.denote o := by
              unfold interpGeneric at hg
              cases hsrcs : srcs.mapM s.getLocal with
              | none => simp [hsrcs] at hg
              | some vs =>
                simp only [hsrcs] at hg
                cases hop : interpOp s.current (readTargetI s) op vs
                    s.memory with
                | error e =>
                  simp only [hop, bind, Except.bind] at hg
                  contradiction
                | ok opres =>
                  simp only [hop, bind, Except.bind] at hg
                  have hsem := interpOp_sound (s := s) hop
                  cases opres with
                  | abort =>
                    dsimp only [IOpRes.denote] at hsem
                    dsimp only at hg
                    simp only [pure, Except.pure, Except.ok.injEq] at hg
                    subst r
                    simp only [InstrResultAgrees] at hcont
                    subst o
                    exact RunFrom.opAbort
                      (by rw [← s.mapM_getLocal]; exact hsrcs) hsem
                  | ok rets m' =>
                    dsimp only [IOpRes.denote] at hsem
                    dsimp only at hg
                    split at hg <;> rename_i hlen
                    ·
                      refine RunFrom.opOk
                        (by rw [← s.mapM_getLocal]; exact hsrcs)
                        hlen hsem ?_
                      rw [← IState.denote_setMemory,
                        ← IState.denote_writeLocals]
                      exact prev P G rest term
                        ((s.setMemory m').writeLocals dsts rets) r o hg hcont
                    · contradiction
            cases op <;>
              try { apply generic; simpa only [interpInstrs] using hexec }
            · -- isParent
              rename_i pat
              by_cases hs : ∃ dst p t,
                  dsts = [dst] ∧ srcs = [p, t]
              · obtain ⟨dst, p, t, rfl, rfl⟩ := hs
                simp only [interpInstrs] at hexec
                cases hp : s.getLocal p with
                | some parent =>
                    apply generic
                    simpa only [interpInstrs, hp] using hexec
                | none =>
                    cases ht : s.getLocal t with
                    | none => simp [hp, ht, interpGeneric] at hexec
                    | some child =>
                        cases child <;>
                          try { simp [hp, ht, interpGeneric] at hexec }
                        case «mut» rt v =>
                          simp only [hp, ht] at hexec
                          refine RunFrom.isParentMissing (by
                            rw [← s.denote_getLocal]
                            exact hp) (IState.getLocal_some_denote ht) ?_
                          rw [← IState.denote_writeLocal]
                          exact prev P G rest term
                            (s.writeLocal dst (.bool false)) r o hexec hcont
              · apply generic
                rw [← interp_isParent_fallback P n rest s dsts srcs pat hs]
                exact hexec
            · -- function
              rename_i f
              simp only [interpInstrs] at hexec
              cases hargs : srcs.mapM s.getLocal with
              | none => simp [hargs] at hexec
              | some args =>
                cases hd : P.funs f with
                | none => simp [hargs, hd] at hexec
                | some d =>
                  by_cases hnargs : args.length = d.numParams
                  · simp [hargs, hd, hnargs] at hexec
                    cases hcall : interpBlock P d.body n d.body.entry
                        (s.enterCall args) with
                    | error e =>
                      simp only [hcall, bind, Except.bind] at hexec
                      contradiction
                    | ok io =>
                      obtain ⟨blk, hentry, hcallee⟩ :=
                        (ih n (by omega)).2.1 P d.body d.body.entry
                          (s.enterCall args) io hcall
                      have hargsSem : srcs.mapM s.denote.locals = some args := by
                        rw [← s.mapM_getLocal]
                        exact hargs
                      cases io with
                      | abort m' code =>
                        simp only [hcall, bind, Except.bind] at hexec
                        change Except.ok (.abort m' code) = Except.ok r at hexec
                        injection hexec with hexec
                        subst r
                        simp only [InstrResultAgrees] at hcont
                        subst o
                        exact RunFrom.callAbort hd hargsSem hnargs hentry
                          (by simpa [IOutcome.denote] using hcallee)
                      | ret world rets =>
                        simp only [hcall, bind, Except.bind] at hexec
                        by_cases hlen : dsts.length = rets.length
                        · simp only [hlen, ↓reduceIte] at hexec
                          refine RunFrom.callOk hd hargsSem hnargs hentry
                            (by simpa [IOutcome.denote] using hcallee) hlen ?_
                          rw [← IWorld.denote_resume,
                            ← IState.denote_writeLocals]
                          exact prev P G rest term
                            ((world.resume s.current).writeLocals dsts rets)
                            r o hexec hcont
                        · simp [hlen] at hexec
                  · simp [hargs, hd, hnargs] at hexec
            · -- functionInst
              rename_i f typeArgs
              simp only [interpInstrs] at hexec
              cases hargs : srcs.mapM s.getLocal with
              | none => simp [hargs] at hexec
              | some args =>
                cases hd : P.funs f with
                | none => simp [hargs, hd] at hexec
                | some d =>
                  by_cases htyargs : typeArgs.length = d.typeParams.length
                  · by_cases hnargs : args.length = d.numParams
                    · simp [hargs, hd, htyargs, hnargs] at hexec
                      cases hcall : interpBlock P (d.body.instantiate typeArgs) n
                          (d.body.instantiate typeArgs).entry
                          (s.enterCall args) with
                      | error e =>
                        simp only [hcall, bind, Except.bind] at hexec
                        contradiction
                      | ok io =>
                        obtain ⟨blk, hentry, hcallee⟩ :=
                          (ih n (by omega)).2.1 P (d.body.instantiate typeArgs)
                            (d.body.instantiate typeArgs).entry
                            (s.enterCall args) io hcall
                        have hargsSem : srcs.mapM s.denote.locals = some args := by
                          rw [← s.mapM_getLocal]
                          exact hargs
                        cases io with
                        | abort m' code =>
                          simp only [hcall, bind, Except.bind] at hexec
                          change Except.ok (.abort m' code) = Except.ok r at hexec
                          injection hexec with hexec
                          subst r
                          simp only [InstrResultAgrees] at hcont
                          subst o
                          exact RunFrom.callInstAbort hd htyargs hargsSem hnargs
                            hentry (by simpa [IOutcome.denote] using hcallee)
                        | ret world rets =>
                          simp only [hcall, bind, Except.bind] at hexec
                          by_cases hlen : dsts.length = rets.length
                          · simp only [hlen, ↓reduceIte] at hexec
                            refine RunFrom.callInstOk hd htyargs hargsSem hnargs
                              hentry (by simpa [IOutcome.denote] using hcallee)
                              hlen ?_
                            rw [← IWorld.denote_resume,
                              ← IState.denote_writeLocals]
                            exact prev P G rest term
                              ((world.resume s.current).writeLocals dsts rets)
                              r o hexec hcont
                          · simp [hlen] at hexec
                    · simp [hargs, hd, htyargs, hnargs] at hexec
                  · simp [hargs, hd, htyargs] at hexec
            · -- borrowLoc
              by_cases hs : ∃ dst x, dsts = [dst] ∧ srcs = [x]
              · obtain ⟨dst, x, rfl, rfl⟩ := hs
                simp only [interpInstrs] at hexec
                cases hx : s.getLocal x with
                | none => simp [hx] at hexec
                | some v =>
                  simp only [hx] at hexec
                  refine RunFrom.borrowLoc (IState.getLocal_some_denote hx) ?_
                  rw [← IState.denote_writeLocal]
                  exact prev P G rest term
                    (s.writeLocal dst (.ref ⟨.loc s.current x, []⟩))
                    r o hexec hcont
              · apply generic
                rw [← interp_oneOne_fallback P n rest s dsts srcs
                  .borrowLoc (Or.inl rfl) hs]
                exact hexec
            · -- borrowField
              rename_i field
              by_cases hs : ∃ dst t, dsts = [dst] ∧ srcs = [t]
              · obtain ⟨dst, t, rfl, rfl⟩ := hs
                simp only [interpInstrs] at hexec
                cases ht : s.getLocal t with
                | none => simp [ht] at hexec
                | some tv =>
                  cases tv <;> try { simp [ht] at hexec }
                  case ref rt =>
                    cases hv : readTargetI s rt with
                    | none => simp [ht, hv] at hexec
                    | some rv =>
                      cases rv <;> try { simp [ht, hv] at hexec }
                      case struct fs =>
                        by_cases hfield : field < fs.length
                        · simp [ht, hv, hfield] at hexec
                          refine RunFrom.borrowField
                            (IState.getLocal_some_denote ht)
                            (readTargetI_some_denote hv) hfield ?_
                          rw [← IState.denote_writeLocal]
                          exact prev P G rest term
                            (s.writeLocal dst
                              (.ref ⟨rt.root, rt.path ++ [field]⟩))
                            r o hexec hcont
                        · simp [ht, hv, hfield] at hexec
              · apply generic
                rw [← interp_oneOne_fallback P n rest s dsts srcs
                  (.borrowField field) (Or.inr (Or.inl ⟨field, rfl⟩)) hs]
                exact hexec
            · -- borrowFieldInst
              rename_i field typeArgs
              by_cases hs : ∃ dst t, dsts = [dst] ∧ srcs = [t]
              · obtain ⟨dst, t, rfl, rfl⟩ := hs
                simp only [interpInstrs] at hexec
                cases ht : s.getLocal t with
                | none => simp [ht] at hexec
                | some tv =>
                  cases tv <;> try { simp [ht] at hexec }
                  case ref rt =>
                    cases hv : readTargetI s rt with
                    | none => simp [ht, hv] at hexec
                    | some rv =>
                      cases rv <;> try { simp [ht, hv] at hexec }
                      case struct fs =>
                        by_cases hfield : field < fs.length
                        · simp [ht, hv, hfield] at hexec
                          refine RunFrom.borrowFieldInst
                            (IState.getLocal_some_denote ht)
                            (readTargetI_some_denote hv) hfield ?_
                          rw [← IState.denote_writeLocal]
                          exact prev P G rest term
                            (s.writeLocal dst
                              (.ref ⟨rt.root, rt.path ++ [field]⟩))
                            r o hexec hcont
                        · simp [ht, hv, hfield] at hexec
              · apply generic
                rw [← interp_oneOneInst_fallback P n rest s dsts srcs
                  (.borrowFieldInst field typeArgs)
                  (Or.inl ⟨field, typeArgs, rfl⟩) hs]
                exact hexec
            · -- borrowGlobal
              rename_i resource
              by_cases hs : ∃ dst t, dsts = [dst] ∧ srcs = [t]
              · obtain ⟨dst, t, rfl, rfl⟩ := hs
                simp only [interpInstrs] at hexec
                cases ht : s.getLocal t with
                | none => simp [ht] at hexec
                | some tv =>
                  cases tv <;> try { simp [ht] at hexec }
                  case address a =>
                    cases hpresent : s.memory.get resource a with
                    | none =>
                      simp only [ht, hpresent] at hexec
                      change Except.ok (.abort s.memory runtimeAbortCode) =
                        Except.ok r at hexec
                      injection hexec with hexec
                      subst r
                      simp only [InstrResultAgrees] at hcont
                      subst o
                      exact RunFrom.borrowGlobalAbort
                        (IState.getLocal_some_denote ht)
                        (by simpa [IState.denote, IMem.denote] using hpresent)
                    | some v =>
                      simp only [ht, hpresent] at hexec
                      refine RunFrom.borrowGlobalOk
                        (IState.getLocal_some_denote ht)
                        (by simpa [IState.denote, IMem.denote] using hpresent) ?_
                      rw [← IState.denote_writeLocal]
                      exact prev P G rest term
                        (s.writeLocal dst (.ref ⟨.global resource a, []⟩))
                        r o hexec hcont
              · apply generic
                rw [← interp_oneOne_fallback P n rest s dsts srcs
                  (.borrowGlobal resource)
                  (Or.inr (Or.inr (Or.inl ⟨resource, rfl⟩))) hs]
                exact hexec
            · -- borrowGlobalInst
              rename_i resource typeArgs
              by_cases hs : ∃ dst t, dsts = [dst] ∧ srcs = [t]
              · obtain ⟨dst, t, rfl, rfl⟩ := hs
                simp only [interpInstrs] at hexec
                cases ht : s.getLocal t with
                | none => simp [ht] at hexec
                | some tv =>
                  cases tv <;> try { simp [ht] at hexec }
                  case address a =>
                    cases hpresent : s.memory.get (resourceKey resource typeArgs) a with
                    | none =>
                      simp only [ht, hpresent] at hexec
                      change Except.ok (.abort s.memory runtimeAbortCode) =
                        Except.ok r at hexec
                      injection hexec with hexec
                      subst r
                      simp only [InstrResultAgrees] at hcont
                      subst o
                      exact RunFrom.borrowGlobalInstAbort
                        (IState.getLocal_some_denote ht)
                        (by simpa [IState.denote, IMem.denote] using hpresent)
                    | some v =>
                      simp only [ht, hpresent] at hexec
                      refine RunFrom.borrowGlobalInstOk
                        (IState.getLocal_some_denote ht)
                        (by simpa [IState.denote, IMem.denote] using hpresent) ?_
                      rw [← IState.denote_writeLocal]
                      exact prev P G rest term
                        (s.writeLocal dst
                          (.ref ⟨.global (resourceKey resource typeArgs) a, []⟩))
                        r o hexec hcont
              · apply generic
                rw [← interp_oneOneInst_fallback P n rest s dsts srcs
                  (.borrowGlobalInst resource typeArgs)
                  (Or.inr ⟨resource, typeArgs, rfl⟩) hs]
                exact hexec
            · -- borrowVecElem
              by_cases hs : ∃ dst t it, dsts = [dst] ∧ srcs = [t, it]
              · obtain ⟨dst, t, it, rfl, rfl⟩ := hs
                simp only [interpInstrs] at hexec
                cases ht : s.getLocal t with
                | none => simp [ht] at hexec
                | some tv =>
                  cases tv <;> try { simp [ht] at hexec }
                  case ref rt =>
                    cases hi : s.getLocal it with
                    | none => simp [ht, hi] at hexec
                    | some iv =>
                      cases iv <;> try { simp [ht, hi] at hexec }
                      case int idxInt =>
                      cases idxInt with
                      | negSucc _ => simp [ht, hi] at hexec
                      | ofNat idx =>
                        cases hv : readTargetI s rt with
                        | none => simp [ht, hi, hv] at hexec
                        | some rv =>
                          cases rv <;> try { simp [ht, hi, hv] at hexec }
                          case vector es =>
                            by_cases hlt : idx < es.length
                            · simp [ht, hi, hv, hlt] at hexec
                              refine RunFrom.borrowVecElemOk
                                (IState.getLocal_some_denote ht)
                                (readTargetI_some_denote hv)
                                (IState.getLocal_some_denote hi) hlt ?_
                              rw [← IState.denote_writeLocal]
                              exact prev P G rest term
                                (s.writeLocal dst
                                  (.ref ⟨rt.root, rt.path ++ [idx]⟩))
                                r o hexec hcont
                            · simp [ht, hi, hv, hlt] at hexec
                              change Except.ok
                                (.abort s.memory runtimeAbortCode) =
                                  Except.ok r at hexec
                              injection hexec with hexec
                              subst r
                              simp only [InstrResultAgrees] at hcont
                              subst o
                              exact RunFrom.borrowVecElemAbort
                                (IState.getLocal_some_denote ht)
                                (readTargetI_some_denote hv)
                                (IState.getLocal_some_denote hi) (by omega)
              · apply generic
                rw [← interp_oneTwo_fallback P n rest s dsts srcs
                  .borrowVecElem rfl hs]
                exact hexec
            · -- readRef
              by_cases hs : ∃ dst t, dsts = [dst] ∧ srcs = [t]
              · obtain ⟨dst, t, rfl, rfl⟩ := hs
                simp only [interpInstrs] at hexec
                cases ht : s.getLocal t with
                | none => simp [ht] at hexec
                | some tv =>
                  cases tv <;> try { simp [ht] at hexec }
                  case ref rt =>
                    cases hv : readTargetI s rt with
                    | none => simp [ht, hv] at hexec
                    | some v =>
                      cases hfree : v.refFree with
                      | false => simp [ht, hv, hfree] at hexec
                      | true =>
                        simp [ht, hv, hfree] at hexec
                        refine RunFrom.readRef
                          (IState.getLocal_some_denote ht)
                          (readTargetI_some_denote hv)
                          (by simpa using hfree) ?_
                        rw [← IState.denote_writeLocal]
                        exact prev P G rest term (s.writeLocal dst v)
                          r o hexec hcont
              · apply generic
                rw [← interp_oneOne_fallback P n rest s dsts srcs .readRef
                  (Or.inr (Or.inr (Or.inr (Or.inl rfl)))) hs]
                exact hexec
            · -- writeRef
              by_cases hs : ∃ t vt, dsts = [] ∧ srcs = [t, vt]
              · obtain ⟨t, vt, rfl, rfl⟩ := hs
                simp only [interpInstrs] at hexec
                cases ht : s.getLocal t with
                | none => simp [ht] at hexec
                | some tv =>
                  cases tv <;> try { simp [ht] at hexec }
                  case ref rt =>
                    cases hv : s.getLocal vt with
                    | none => simp [ht, hv] at hexec
                    | some v =>
                      cases hfree : v.refFree with
                      | false => simp [ht, hv, hfree] at hexec
                      | true =>
                        cases hw : writeTargetI s rt v with
                        | none => simp [ht, hv, hfree, hw] at hexec
                        | some s' =>
                          simp only [ht, hv, hfree, Bool.not_true, hw] at hexec
                          have hwsem := writeTargetI_eq s rt v
                          simp [hw] at hwsem
                          refine RunFrom.writeRef
                            (IState.getLocal_some_denote ht)
                            (IState.getLocal_some_denote hv)
                            (by simpa using hfree)
                            hwsem.symm ?_
                          exact prev P G rest term s' r o hexec hcont
              · apply generic
                rw [← interp_zeroTwo_fallback P n rest s dsts srcs
                  .writeRef rfl hs]
                exact hexec
            · -- freezeRef
              by_cases hs : ∃ dst t, dsts = [dst] ∧ srcs = [t]
              · obtain ⟨dst, t, rfl, rfl⟩ := hs
                simp only [interpInstrs] at hexec
                cases ht : s.getLocal t with
                | none => simp [ht] at hexec
                | some tv =>
                  cases tv <;> try { simp [ht] at hexec }
                  case ref rt =>
                    cases hv : readTargetI s rt with
                    | none => simp [ht, hv] at hexec
                    | some v =>
                      cases hfree : v.refFree with
                      | false => simp [ht, hv, hfree] at hexec
                      | true =>
                        simp [ht, hv, hfree] at hexec
                        refine RunFrom.freezeRef
                          (IState.getLocal_some_denote ht)
                          (readTargetI_some_denote hv)
                          (by simpa using hfree) ?_
                        rw [← IState.denote_writeLocal]
                        exact prev P G rest term
                          (s.writeLocal dst (.ref rt)) r o hexec hcont
              · apply generic
                rw [← interp_oneOne_fallback P n rest s dsts srcs .freezeRef
                  (Or.inr (Or.inr (Or.inr (Or.inr rfl)))) hs]
                exact hexec
    have hblock : BlockSoundAt fuel := by
      intro P G b s o hexec
      cases fuel with
      | zero => simp [interpBlock] at hexec
      | succ n =>
        simp only [interpBlock] at hexec
        cases hb : G.blocks b with
        | none => simp [hb] at hexec
        | some blk =>
          cases hi : interpInstrs P (n + 1) blk.instrs s with
          | error e =>
            simp only [hb, hi, bind, Except.bind] at hexec
            contradiction
          | ok ir =>
            simp only [hb, hi, bind, Except.bind] at hexec
            cases ir with
            | abort m' code =>
              change Except.ok (.abort m' code) = .ok o at hexec
              injection hexec with hexec
              subst o
              exact ⟨blk, hb, his P G blk.instrs blk.term s
                (.abort m' code) (.abort m'.denote code) hi rfl⟩
            | ok s' =>
              refine ⟨blk, hb, his P G blk.instrs blk.term s (.ok s')
                o.denote hi ?_⟩
              simp only [InstrResultAgrees]
              cases hterm : blk.term with
              | jump b' =>
                simp only [hterm] at hexec
                obtain ⟨next, hbnext, hnext⟩ :=
                  (ih n (by omega)).2.1 P G b' s' o hexec
                exact RunFrom.jump hbnext hnext
              | branch c b₁ b₂ =>
                simp only [hterm] at hexec
                cases hc : s'.getLocal c with
                | none => simp [hc] at hexec
                | some cv =>
                  cases cv <;> try { simp [hc] at hexec }
                  case bool taken =>
                    cases taken <;> simp only [hc] at hexec
                    · obtain ⟨next, hbnext, hnext⟩ :=
                        (ih n (by omega)).2.1 P G b₂ s' o hexec
                      exact RunFrom.branchFalse
                        (IState.getLocal_some_denote hc)
                        hbnext hnext
                    · obtain ⟨next, hbnext, hnext⟩ :=
                        (ih n (by omega)).2.1 P G b₁ s' o hexec
                      exact RunFrom.branchTrue
                        (IState.getLocal_some_denote hc)
                        hbnext hnext
              | ret srcs =>
                simp only [hterm] at hexec
                cases hvals : srcs.mapM s'.getLocal with
                | none => simp [hvals] at hexec
                | some vals =>
                  simp only [hvals] at hexec
                  change Except.ok (.ret s'.finishFrame vals) = .ok o at hexec
                  injection hexec with hexec
                  subst o
                  simpa [IOutcome.denote] using RunFrom.ret
                    (IState.mapM_getLocal_some_denote hvals)
              | abort code =>
                simp only [hterm] at hexec
                cases hcode : s'.getLocal code with
                | none => simp [hcode] at hexec
                | some cv =>
                  cases cv <;> try { simp [hcode] at hexec }
                  case int codeInt =>
                  cases codeInt with
                  | negSucc _ => simp [hcode] at hexec
                  | ofNat code' =>
                    simp only [hcode] at hexec
                    change Except.ok (.abort s'.memory code') = .ok o at hexec
                    injection hexec with hexec
                    subst o
                    exact RunFrom.abort
                      (IState.getLocal_some_denote hcode)
    have hfun : FunSoundAt fuel := by
      intro P f m args o hexec
      cases fuel with
      | zero => simp [interpFun] at hexec
      | succ n =>
        simp only [interpFun] at hexec
        cases hd : P.funs f with
        | none => simp [hd] at hexec
        | some d =>
          simp only [hd] at hexec
          split at hexec
          · rename_i harity
            refine ⟨d, hd, harity, ?_⟩
            simpa using (ih n (by omega)).2.1 P d.body d.body.entry
              (IState.initial args m) o hexec
          · simp at hexec
    exact ⟨his, hblock, hfun⟩

/-- Every successful interpreter result is a relational execution. -/
theorem interpFun_sound {P : Program} {fuel : Nat} {f : FunId} {m : IMem}
    {args : List Value} {o : IOutcome}
    (h : interpFun P fuel f m args = .ok o) :
    FunExec P f m.denote args o.denote :=
  (interp_sound_at fuel).2.2 P f m args o h

end MoveModel.IR
