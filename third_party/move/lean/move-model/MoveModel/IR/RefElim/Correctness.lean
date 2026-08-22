-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.RefElim.Transform
import MoveModel.IR.CodeTyping
import MoveModel.IR.Execution

/-!
# Correctness of Reference Elimination

Reference elimination is proved in the same two layers as the pass:

1. `elimImm_correct` replaces immutable references by stable copies.
2. `elimCore_correct` replaces mutable references by mutation values and
   explicit write-backs.

Local roots are pairs `(frame, local)`.  A reference passed to a callee keeps
its identity and continues to address the caller frame.  This removes the need
for checkout locals, shadow slots, and root-renaming maps.

The borrow certificate ensures that callee-local roots do not escape and that
returned references derive from input references.  The correctness theorem
covers the isolated, summary-free `refElimFun` relation (`ElimProgram` below).
The frontend instead runs interprocedural `refElimProg`.  Its summary
computation and cross-call rewriting are executable and tested, but are not
yet covered by `refElim_correct`.

The two layer lemmas are the roadmap proof obligations, and their composition
is proved below.  Both layer proofs use the six-case execution induction
directly; the older checkout simulation is no longer needed.
-/

namespace MoveModel.IR

/-! ## Program relations and observable outcomes -/

/-- `P'` is the function-by-function reference elimination of `P`. -/
def ElimProgram (P P' : Program) : Prop :=
  P'.structs = P.structs ∧
  ∀ f, (P.funs f = none ∧ P'.funs f = none) ∨
    ∃ d d', P.funs f = some d ∧
      refElimFun P.funs P.structs d = .ok d' ∧
      P'.funs f = some d'

/-- External outcome agreement.  Retired frame stores are not observable;
normal executions agree on memory and the source return is the prefix before
the mutation-parameter finals introduced by the core pass.  Abort effects are
discarded, so only the abort code agrees there. -/
inductive AgreeOutcome : FrameOutcome → FrameOutcome → Prop where
  | ret {world world' : FrameWorld} {vals finals : List Value}
      (hmem : world'.memory = world.memory) :
      AgreeOutcome (.ret world vals) (.ret world' (vals ++ finals))
  | abort {m m' : Memory} {code : Nat} :
      AgreeOutcome (.abort m code) (.abort m' code)

/-- The intermediate program after immutable-reference elimination. -/
def immProgram (P : Program) : Program :=
  { P with
    funs := fun f =>
      (P.funs f).bind fun d => (elimImmRefs P.funs d).toOption }

/-- `P'` is core elimination of the post-immutable program `P₁`.
The origin witness exposes the boundary checks performed by the immutable
pass; `summarize` is the escape check for returned references. -/
def CoreProgram (P₁ P' : Program) : Prop :=
  (∃ P, P₁ = immProgram P) ∧
  P'.structs = P₁.structs ∧
  ∀ f, (P₁.funs f = none ∧ P'.funs f = none) ∨
    ∃ d d' s, P₁.funs f = some d ∧
      summarize noSummaries d = .ok s ∧
      elimCore noSummaries P₁.structs d = .ok d' ∧
      P'.funs f = some d'

/-- Recover the original program from a core-elimination program relation. -/
theorem CoreProgram.imm_origin {P₁ P' : Program} (h : CoreProgram P₁ P')
    {f : FunId} {d : FunDecl} (hd : P₁.funs f = some d) :
    ∃ P e, P₁ = immProgram P ∧ P.funs f = some e ∧
      elimImmRefs P.funs e = .ok d := by
  obtain ⟨P, rfl⟩ := h.1
  simp only [immProgram, Option.bind_eq_some_iff] at hd
  obtain ⟨e, he, hout⟩ := hd
  cases helim : elimImmRefs P.funs e with
  | error err => rw [helim] at hout; cases hout
  | ok d' =>
      rw [helim] at hout
      simp only [Except.toOption, Option.some.injEq] at hout
      subst d'
      exact ⟨P, e, rfl, he, helim⟩

/-- A successful immutable transformation installs its result in the
intermediate program at the same function id. -/
theorem immProgram_fun {P : Program} {f : FunId} {d d' : FunDecl}
    (hd : P.funs f = some d) (helim : elimImmRefs P.funs d = .ok d') :
    (immProgram P).funs f = some d' := by
  simp [immProgram, hd, helim, Except.toOption]

/-! ## Structural certificate for the core pass -/

/-- Emitter state after the optional parent-written bookkeeping of one
write-back edge. -/
def coreAfterWrite (e : BEdge) (st : EmitSt) : EmitSt :=
  match e.parent with
  | .refNode p =>
      if st.written.contains p then st
      else { st with written := p :: st.written }
  | _ => st

/-- Parent-written bookkeeping does not alter emitted blocks or the active
cursor. -/
theorem coreAfterWrite_projection (e : BEdge) (st : EmitSt) :
    (coreAfterWrite e st).done = st.done ∧
    (coreAfterWrite e st).curId = st.curId ∧
    (coreAfterWrite e st).cur = st.cur := by
  rcases e with ⟨parent, child, path⟩
  cases parent with
  | refNode p =>
      by_cases hp : p ∈ st.written <;>
        simp [coreAfterWrite, hp]
  | _ => exact ⟨rfl, rfl, rfl⟩

/-- Fresh-local allocation leaves the active emitter cursor unchanged. -/
theorem alloc_cursor {ty : Ty} {st st' : EmitSt} {x : LocalIndex}
    (h : alloc ty st = .ok (x, st')) :
    st'.curId = st.curId ∧ st'.cur = st.cur := by
  obtain ⟨-, rfl⟩ := alloc_inv h
  exact ⟨rfl, rfl⟩

/-- Appending instructions changes only the current instruction prefix. -/
theorem emitAll_cursor {is : List Instr} {st st' : EmitSt}
    (h : emitAll is st = .ok ((), st')) :
    st'.curId = st.curId ∧ st'.cur = st.cur ++ is := by
  unfold emitAll at h
  simp only [modify] at h
  obtain ⟨-, rfl⟩ := h
  exact ⟨rfl, rfl⟩

/-- Appending instructions does not allocate locals. -/
theorem emitAll_nextLocal {is : List Instr} {st st' : EmitSt}
    (h : emitAll is st = .ok ((), st')) :
    st'.nextLocal = st.nextLocal := by
  unfold emitAll at h
  simp only [modify] at h
  obtain ⟨-, rfl⟩ := h
  rfl

/-- Closing a block records its current prefix and resets the requested
continuation cursor. -/
theorem closeBlock_inv {term : Term} {contId : BlockId} {st st' : EmitSt}
    (h : closeBlock term contId st = .ok ((), st')) :
    st' = { st with
      done := (st.curId, ⟨st.cur, term⟩) :: st.done
      curId := contId
      cur := [] } := by
  unfold closeBlock at h
  simp only [modify] at h
  cases h
  rfl

/-- Allocating a block identifier does not move the active cursor. -/
theorem newBlockId_cursor {src result : BlockId} {st st' : EmitSt}
    (h : newBlockId src st = .ok (result, st')) :
    st'.curId = st.curId ∧ st'.cur = st.cur := by
  unfold newBlockId at h
  simp only [get, getThe, MonadStateOf.get, StateT.get, bind, StateT.bind,
    set, StateT.set, pure, StateT.pure, Except.pure, Except.bind] at h
  cases h
  exact ⟨rfl, rfl⟩

/-- Successor-edge splitting restores the source block's active cursor after
emitting any synthetic edge block. -/
theorem splitCoreEdge_cursor {Δ : StructDecls} {d : FunDecl}
    {liveIn : Array LiveSet} {b : BlockId} {g : BGraph}
    {pending : List LocalIndex} {target result : BlockId} {st st' : EmitSt}
    (h : splitCoreEdge Δ d liveIn b g pending target st =
      .ok (result, st')) :
    st'.curId = st.curId ∧ st'.cur = st.cur := by
  unfold splitCoreEdge at h
  split at h
  · simp only [pure, StateT.pure, Except.pure, Except.ok.injEq,
      Prod.mk.injEq] at h
    obtain ⟨-, rfl⟩ := h
    exact ⟨rfl, rfl⟩
  · simp only [bind, StateT.bind] at h
    change (newBlockId b st).bind _ = .ok (result, st') at h
    obtain ⟨first, hnew, h⟩ := Except.bind_ok_inv h
    obtain ⟨w, stNew⟩ := first
    have hnewCur := newBlockId_cursor hnew
    simp only [get, getThe, MonadStateOf.get, StateT.get, pure,
      StateT.pure, Except.pure, modify, modifyGet, MonadStateOf.modifyGet,
      StateT.modifyGet, Except.bind] at h
    let stEdge : EmitSt := { stNew with curId := w, cur := [] }
    change (processDeaths Δ d b g (liveIn.getD target ∅) pending
      stEdge).bind _ = .ok (result, st') at h
    obtain ⟨deaths, -, h⟩ := Except.bind_ok_inv h
    obtain ⟨pending', stDeaths⟩ := deaths
    cases hclose : closeBlock (.jump target) 0 stDeaths with
    | error e => simp [hclose] at h
    | ok closed =>
        obtain ⟨u, stClosed⟩ := closed
        cases u
        simp only [hclose, Except.ok.injEq, Prod.mk.injEq] at h
        obtain ⟨-, rfl⟩ := h
        exact ⟨by simpa using hnewCur.1, by simpa using hnewCur.2⟩

/-- Root-exclusivity checks inspect but never change emitter state. -/
theorem checkRoots_state {g : BGraph} {pending : List LocalIndex} :
    ∀ {xs : List LocalIndex} {st st' : EmitSt},
      checkRoots g pending xs st = .ok ((), st') → st' = st
  | [], st, st', h => by
      simp [checkRoots, pure, StateT.pure, Except.pure] at h
      exact h.symm
  | x :: xs, st, st', h => by
      rw [checkRoots] at h
      split at h
      · simp [throw, throwThe, MonadExceptOf.throw, StateT.lift,
          bind, StateT.bind, Except.bind] at h
      · exact checkRoots_state h

/-- Mutation bookkeeping leaves the emitter cursor and local frontier
unchanged. -/
theorem markWritten_projection {t : LocalIndex} {st st' : EmitSt}
    (h : markWritten t st = .ok ((), st')) :
    st'.curId = st.curId ∧ st'.cur = st.cur ∧
      st'.nextLocal = st.nextLocal := by
  unfold markWritten at h
  simp only [modify] at h
  obtain ⟨-, rfl⟩ := h
  split <;> exact ⟨rfl, rfl, rfl⟩

/-- Guarded block splitting does not allocate locals. -/
theorem emitGuarded_nextLocal {src : BlockId} {guard : LocalIndex}
    {body : List Instr} {st st' : EmitSt}
    (h : emitGuarded src guard body st = .ok ((), st')) :
    st'.nextLocal = st.nextLocal := by
  unfold emitGuarded newBlockId closeBlock at h
  simp only [get, bind, StateT.bind, set, StateT.set, pure, StateT.pure,
    Except.pure, modify] at h
  cases h
  rfl

/-- Parent-written bookkeeping does not change the fresh-local frontier. -/
theorem coreAfterWrite_nextLocal (e : BEdge) (st : EmitSt) :
    (coreAfterWrite e st).nextLocal = st.nextLocal := by
  rcases e with ⟨parent, child, path⟩
  cases parent with
  | refNode p =>
      by_cases hp : p ∈ st.written <;>
        simp [coreAfterWrite, hp, EmitSt.nextLocal]
  | _ => rfl

/-- The two blocks introduced by one guarded diamond are retained exactly,
and its continuation becomes the new emitter cursor. -/
theorem emitGuarded_blocks {src : BlockId} {guard : LocalIndex}
    {body : List Instr} {st st' : EmitSt}
    (h : emitGuarded src guard body st = .ok ((), st')) :
    (st.curId, ⟨st.cur,
      .branch guard st.nextId (st.nextId + 1)⟩) ∈ st'.done ∧
    (st.nextId, ⟨body, .jump (st.nextId + 1)⟩) ∈ st'.done ∧
    st'.curId = st.nextId + 1 ∧ st'.cur = [] := by
  unfold emitGuarded newBlockId closeBlock at h
  simp only [get, bind, StateT.bind, set, StateT.set,
    pure, StateT.pure, Except.pure, modify] at h
  cases h
  simp

/-- Guard generation allocates exactly one Boolean temporary and emits the
test selected by the edge's parent shape. -/
theorem wbGuard_inv {t : LocalIndex} {e : BEdge} {st st' : EmitSt}
    {guardIs : List Instr} {guard : LocalIndex}
    (h : wbGuard t e st = .ok ((guardIs, guard), st')) :
    alloc .bool st = .ok (guard, st') ∧
      (match e.parent with
      | .refNode p =>
          guardIs = [.call [guard] (.isParent (bPathPattern e.path)) [p, t]]
      | .localRoot x =>
          guardIs = [.call [guard] (.isMutLoc x) [t]]
      | .globalRoot r =>
          guardIs = [.call [guard] (.isMutGlobal r) [t]]
      | .anyRoot => False) := by
  unfold wbGuard at h
  simp only [bind, StateT.bind] at h
  change (alloc .bool st).bind _ = .ok ((guardIs, guard), st') at h
  obtain ⟨allocated, halloc, hout⟩ := Except.bind_ok_inv h
  obtain ⟨g, stAlloc⟩ := allocated
  cases hparent : e.parent <;> simp only [hparent] at hout ⊢
  all_goals
    simp only [pure] at hout
  · obtain ⟨⟨rfl, rfl⟩, rfl⟩ := hout
    exact ⟨halloc, rfl⟩
  · obtain ⟨⟨rfl, rfl⟩, rfl⟩ := hout
    exact ⟨halloc, rfl⟩
  · obtain ⟨⟨rfl, rfl⟩, rfl⟩ := hout
    exact ⟨halloc, rfl⟩
  · contradiction

/-- Exact emitter decomposition of a guarded candidate list.  Each step
records guard generation, body generation, and the emitted diamond; the
deterministic bookkeeping is represented by `coreAfterWrite`. -/
inductive CoreGuardedWriteBackTrace (Δ : StructDecls) (d : FunDecl)
    (src : BlockId) (t : LocalIndex) :
    List BEdge → EmitSt → EmitSt → Prop where
  | nil {st : EmitSt} : CoreGuardedWriteBackTrace Δ d src t [] st st
  | cons {e : BEdge} {rest : List BEdge}
      {st stGuard stAppend stBody stSplit stEnd : EmitSt}
      {guardIs body : List Instr} {guard : LocalIndex}
      (guardEmit : wbGuard t e st = .ok ((guardIs, guard), stGuard))
      (guardAppend : emitAll guardIs stGuard = .ok ((), stAppend))
      (bodyEmit : wbBody Δ d t e stAppend = .ok (body, stBody))
      (diamondEmit : emitGuarded src guard body stBody = .ok ((), stSplit))
      (tail : CoreGuardedWriteBackTrace Δ d src t rest
        (coreAfterWrite e stSplit) stEnd) :
      CoreGuardedWriteBackTrace Δ d src t (e :: rest) st stEnd

/-- Successful guarded emission yields its exact candidate-by-candidate
trace. -/
theorem emitGuardedWriteBacks_trace {Δ : StructDecls} {d : FunDecl}
    {src : BlockId} {t : LocalIndex} :
    ∀ {es : List BEdge} {st stEnd : EmitSt},
      emitGuardedWriteBacks Δ d src t es st = .ok ((), stEnd) →
      CoreGuardedWriteBackTrace Δ d src t es st stEnd
  | [], st, stEnd, h => by
      simp only [emitGuardedWriteBacks, pure, StateT.pure, Except.pure,
        Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨-, rfl⟩ := h
      exact .nil
  | e :: rest, st, stEnd, h => by
      rw [emitGuardedWriteBacks] at h
      change (wbGuard t e st).bind _ = .ok ((), stEnd) at h
      obtain ⟨guardResult, hguard, h⟩ := Except.bind_ok_inv h
      obtain ⟨⟨guardIs, guard⟩, stGuard⟩ := guardResult
      simp only [bind, StateT.bind] at h
      change (emitAll guardIs stGuard).bind _ = .ok ((), stEnd) at h
      obtain ⟨appended, happend, h⟩ := Except.bind_ok_inv h
      obtain ⟨u, stAppend⟩ := appended
      cases u
      change (wbBody Δ d t e stAppend).bind _ = .ok ((), stEnd) at h
      obtain ⟨bodyResult, hbody, h⟩ := Except.bind_ok_inv h
      obtain ⟨body, stBody⟩ := bodyResult
      change (emitGuarded src guard body stBody).bind _ =
        .ok ((), stEnd) at h
      obtain ⟨split, hsplit, h⟩ := Except.bind_ok_inv h
      obtain ⟨u, stSplit⟩ := split
      cases u
      cases hp : e.parent with
      | refNode p =>
          rw [hp] at h
          simp only [markWritten, modify, bind, StateT.bind] at h
          exact .cons hguard happend hbody hsplit (by
            simpa [coreAfterWrite, hp] using
              (emitGuardedWriteBacks_trace h))
      | localRoot x =>
          rw [hp] at h
          exact .cons hguard happend hbody hsplit (by
            simpa [coreAfterWrite, hp] using
              (emitGuardedWriteBacks_trace h))
      | globalRoot r =>
          rw [hp] at h
          exact .cons hguard happend hbody hsplit (by
            simpa [coreAfterWrite, hp] using
              (emitGuardedWriteBacks_trace h))
      | anyRoot =>
          rw [hp] at h
          exact .cons hguard happend hbody hsplit (by
            simpa [coreAfterWrite, hp] using
              (emitGuardedWriteBacks_trace h))

/-- Structural alternatives for emitting all write-backs of one dying
reference. -/
inductive CoreWriteBackTrace (Δ : StructDecls) (d : FunDecl)
    (src : BlockId) (g : BGraph) (t : LocalIndex) :
    EmitSt → EmitSt → Prop where
  | none {st : EmitSt} (edges : inEdges g t = []) :
      CoreWriteBackTrace Δ d src g t st st
  | single {e : BEdge} {st stBody stAppend stEnd : EmitSt}
      {body : List Instr}
      (edges : inEdges g t = [e])
      (bodyEmit : wbBody Δ d t e st = .ok (body, stBody))
      (append : emitAll body stBody = .ok ((), stAppend))
      (marked : (match e.parent with
        | .refNode p => markWritten p
        | _ => pure ()) stAppend = .ok ((), stEnd)) :
      CoreWriteBackTrace Δ d src g t st stEnd
  | guarded {e₁ e₂ : BEdge} {rest : List BEdge} {st stEnd : EmitSt}
      (edges : inEdges g t = e₁ :: e₂ :: rest)
      (trace : CoreGuardedWriteBackTrace Δ d src t
        (e₁ :: e₂ :: rest) st stEnd) :
      CoreWriteBackTrace Δ d src g t st stEnd

/-- Successful write-back emission yields its empty, singleton, or guarded
trace. -/
theorem emitWriteBacks_trace {Δ : StructDecls} {d : FunDecl}
    {src : BlockId} {g : BGraph} {t : LocalIndex} {st stEnd : EmitSt}
    (h : emitWriteBacks Δ d src g t st = .ok ((), stEnd)) :
    CoreWriteBackTrace Δ d src g t st stEnd := by
  unfold emitWriteBacks at h
  cases hedges : inEdges g t with
  | nil =>
      rw [hedges] at h
      simp only [pure, StateT.pure, Except.pure, Except.ok.injEq,
        Prod.mk.injEq] at h
      obtain ⟨-, rfl⟩ := h
      exact .none hedges
  | cons e rest =>
      rw [hedges] at h
      cases rest with
      | nil =>
          simp only at h
          change (wbBody Δ d t e st).bind _ = .ok ((), stEnd) at h
          obtain ⟨bodyResult, hbody, h⟩ := Except.bind_ok_inv h
          obtain ⟨body, stBody⟩ := bodyResult
          simp only [bind, StateT.bind] at h
          change (emitAll body stBody).bind _ = .ok ((), stEnd) at h
          obtain ⟨appended, happend, hmarked⟩ := Except.bind_ok_inv h
          obtain ⟨u, stAppend⟩ := appended
          cases u
          exact .single hedges hbody happend hmarked
      | cons e₂ rest =>
          exact .guarded hedges (emitGuardedWriteBacks_trace h)

/-- Guarded write-back traces retain every block finished before them. -/
theorem CoreGuardedWriteBackTrace.done_subset
    {Δ : StructDecls} {d : FunDecl} {src : BlockId} {t : LocalIndex}
    {es : List BEdge} {st stEnd : EmitSt}
    (h : CoreGuardedWriteBackTrace Δ d src t es st stEnd) :
    ∀ p ∈ st.done, p ∈ stEnd.done := by
  induction h with
  | nil => exact fun _ hp => hp
  | @cons e rest st stGuard stAppend stBody stSplit stEnd
      guardIs body guard hguard happend hbody hsplit tail ih =>
      intro p hp
      apply ih p
      have hpGuard := wbGuard_preservesDone t e st (guardIs, guard)
        stGuard hguard p hp
      have hpAppend := emitAll_preservesDone guardIs stGuard () stAppend
        happend p hpGuard
      have hpBody := wbBody_preservesDone Δ d t e stAppend body stBody
        hbody p hpAppend
      have hpSplit := emitGuarded_preservesDone src guard body
        stBody () stSplit hsplit p hpBody
      rcases e with ⟨parent, child, path⟩
      cases parent with
      | refNode q => simp only [coreAfterWrite]; split <;> exact hpSplit
      | _ => exact hpSplit

/-- A complete write-back trace retains all previously finished blocks. -/
theorem CoreWriteBackTrace.done_subset
    {Δ : StructDecls} {d : FunDecl} {src : BlockId}
    {g : BGraph} {t : LocalIndex} {st stEnd : EmitSt}
    (h : CoreWriteBackTrace Δ d src g t st stEnd) :
    ∀ p ∈ st.done, p ∈ stEnd.done := by
  cases h with
  | none => exact fun _ hp => hp
  | @single e st stBody stAppend stEnd body _ hbody happend hmarked =>
      intro p hp
      have hpBody := wbBody_preservesDone Δ d t e st body stBody
        hbody p hp
      have hpAppend := emitAll_preservesDone body stBody () stAppend
        happend p hpBody
      cases hparent : e.parent with
      | refNode parent =>
          exact markWritten_preservesDone parent stAppend () stEnd
            (by simpa [hparent] using hmarked) p hpAppend
      | localRoot x =>
          have hstate : stAppend = stEnd := by
            simpa [hparent, pure, StateT.pure, Except.pure] using hmarked
          rwa [← hstate]
      | globalRoot r =>
          have hstate : stAppend = stEnd := by
            simpa [hparent, pure, StateT.pure, Except.pure] using hmarked
          rwa [← hstate]
      | anyRoot =>
          have hstate : stAppend = stEnd := by
            simpa [hparent, pure, StateT.pure, Except.pure] using hmarked
          rwa [← hstate]
  | guarded _ trace => exact trace.done_subset

/-- Structural trace of one death-processing cascade.  It records only the
chosen leaf and its emitter transition; all monadic control-flow inversion is
performed once when this certificate is built. -/
inductive CoreDeathTrace (Δ : StructDecls) (d : FunDecl) (src : BlockId)
    (g : BGraph) (liveNow : LiveSet) :
    Nat → List LocalIndex → EmitSt →
      List LocalIndex → EmitSt → Prop where
  | fuelZero {pending : List LocalIndex} {st : EmitSt} :
      CoreDeathTrace Δ d src g liveNow 0 pending st pending st
  | noLeaf {fuel : Nat} {pending : List LocalIndex} {st : EmitSt}
      (find : pending.find? (fun t =>
        !liveNow.contains t && !hasPendingChild g pending t) = none) :
      CoreDeathTrace Δ d src g liveNow (fuel + 1) pending st pending st
  | leaf {fuel : Nat} {pending pendingEnd : List LocalIndex}
      {t : LocalIndex} {st st' stEnd : EmitSt}
      (find : pending.find? (fun t =>
        !liveNow.contains t && !hasPendingChild g pending t) = some t)
      (writeBack : CoreWriteBackTrace Δ d src g t st st')
      (rest : CoreDeathTrace Δ d src g liveNow fuel
        (pending.filter (· ≠ t)) st' pendingEnd stEnd) :
      CoreDeathTrace Δ d src g liveNow (fuel + 1) pending st
        pendingEnd stEnd

/-- A successful internal death-processing loop yields its structural trace. -/
theorem processDeaths_go_trace {Δ : StructDecls} {d : FunDecl}
    {src : BlockId} {g : BGraph} {liveNow : LiveSet} :
    ∀ {fuel : Nat} {pending pendingEnd : List LocalIndex} {st stEnd : EmitSt},
      processDeaths.go Δ d src g liveNow fuel pending st =
        .ok (pendingEnd, stEnd) →
      CoreDeathTrace Δ d src g liveNow fuel pending st pendingEnd stEnd
  | 0, pending, pendingEnd, st, stEnd, h => by
      simp only [processDeaths.go, pure, StateT.pure, Except.pure,
        Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      exact .fuelZero
  | fuel + 1, pending, pendingEnd, st, stEnd, h => by
      rw [processDeaths.go] at h
      cases hfind : pending.find? (fun t =>
          !liveNow.contains t && !hasPendingChild g pending t) with
      | none =>
          simp only [hfind, pure, StateT.pure, Except.pure,
            Except.ok.injEq, Prod.mk.injEq] at h
          obtain ⟨rfl, rfl⟩ := h
          exact .noLeaf hfind
      | some t =>
          rw [hfind] at h
          change (emitWriteBacks Δ d src g t st).bind (fun p =>
            processDeaths.go Δ d src g liveNow fuel
              (pending.filter (· ≠ t)) p.2) =
              .ok (pendingEnd, stEnd) at h
          obtain ⟨p, hwrite, hrest⟩ := Except.bind_ok_inv h
          obtain ⟨u, st'⟩ := p
          cases u
          exact .leaf hfind (emitWriteBacks_trace hwrite)
            (processDeaths_go_trace hrest)

/-- A successful public death phase yields a trace starting with exactly one
unit of fuel per pending reference. -/
theorem processDeaths_trace {Δ : StructDecls} {d : FunDecl}
    {src : BlockId} {g : BGraph} {liveNow : LiveSet}
    {pending pendingEnd : List LocalIndex} {st stEnd : EmitSt}
    (h : processDeaths Δ d src g liveNow pending st =
      .ok (pendingEnd, stEnd)) :
    CoreDeathTrace Δ d src g liveNow pending.length pending st
      pendingEnd stEnd := by
  exact processDeaths_go_trace h

/-- Invert the leaf search used by death processing. -/
theorem deathCandidate_of_find {g : BGraph} {liveNow : LiveSet}
    {pending : List LocalIndex} {t : LocalIndex}
    (h : pending.find? (fun t =>
      !liveNow.contains t && !hasPendingChild g pending t) = some t) :
    t ∈ pending ∧ liveNow.contains t = false ∧
      hasPendingChild g pending t = false := by
  refine ⟨List.mem_of_find?_eq_some h, ?_⟩
  have hselected := List.find?_some h
  cases hlive : liveNow.contains t <;>
    cases hchild : hasPendingChild g pending t <;> simp_all

/-- Death processing can only remove pending references. -/
theorem CoreDeathTrace.pending_subset {Δ : StructDecls} {d : FunDecl}
    {src : BlockId} {g : BGraph} {liveNow : LiveSet}
    {fuel : Nat} {pending pendingEnd : List LocalIndex}
    {st stEnd : EmitSt}
    (h : CoreDeathTrace Δ d src g liveNow fuel pending st pendingEnd stEnd) :
    ∀ t, t ∈ pendingEnd → t ∈ pending := by
  induction h with
  | fuelZero => exact fun _ ht => ht
  | noLeaf => exact fun _ ht => ht
  | leaf _ _ _ ih =>
      intro t ht
      exact (List.mem_filter.mp (ih t ht)).1

/-- A death cascade retains all blocks finished before it. -/
theorem CoreDeathTrace.done_subset {Δ : StructDecls} {d : FunDecl}
    {src : BlockId} {g : BGraph} {liveNow : LiveSet}
    {fuel : Nat} {pending pendingEnd : List LocalIndex}
    {st stEnd : EmitSt}
    (h : CoreDeathTrace Δ d src g liveNow fuel pending st pendingEnd stEnd) :
    ∀ p ∈ st.done, p ∈ stEnd.done := by
  induction h with
  | fuelZero => exact fun _ hp => hp
  | noLeaf => exact fun _ hp => hp
  | leaf _ write _ ih =>
      intro p hp
      exact ih p (write.done_subset p hp)

/-- Per-instruction trace of the core rewrite.  Each step records both the
local instruction rewrite and the death/write-back phase immediately after
it; these are the two semantic splices associated with one source action. -/
inductive CoreInstrTrace (sums : Summaries) (Δ : StructDecls) (d : FunDecl)
    (src : BlockId) : List (Instr × LiveSet) →
    BGraph → List LocalIndex → EmitSt →
    BGraph → List LocalIndex → EmitSt → Prop where
  | nil {g : BGraph} {pending : List LocalIndex} {st : EmitSt} :
      CoreInstrTrace sums Δ d src [] g pending st g pending st
  | cons {i : Instr} {liveAfter : LiveSet}
      {points : List (Instr × LiveSet)}
      {g g' gEnd : BGraph} {pending pending' pending'' pendingEnd : List LocalIndex}
      {st st' st'' stEnd : EmitSt}
      (rewrite : rewriteInstr sums d g pending i st =
        .ok ((g', pending'), st'))
      (deaths : CoreDeathTrace Δ d src g' liveAfter pending'.length
        pending' st' pending'' st'')
      (rest : CoreInstrTrace sums Δ d src points g' pending'' st''
        gEnd pendingEnd stEnd) :
      CoreInstrTrace sums Δ d src ((i, liveAfter) :: points) g pending st
        gEnd pendingEnd stEnd

/-- Successful instruction emission yields its per-source-instruction trace. -/
theorem rewriteCoreInstrs_trace {sums : Summaries} {Δ : StructDecls}
    {d : FunDecl} {src : BlockId} :
    ∀ {points : List (Instr × LiveSet)} {g : BGraph}
      {pending : List LocalIndex} {st : EmitSt}
      {gEnd : BGraph} {pendingEnd : List LocalIndex} {stEnd : EmitSt},
      rewriteCoreInstrs sums Δ d src points g pending st =
        .ok ((gEnd, pendingEnd), stEnd) →
      CoreInstrTrace sums Δ d src points g pending st
        gEnd pendingEnd stEnd
  | [], g, pending, st, gEnd, pendingEnd, stEnd, h => by
      simp only [rewriteCoreInstrs, pure, StateT.pure, Except.pure,
        Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨⟨rfl, rfl⟩, rfl⟩ := h
      exact .nil
  | (i, liveAfter) :: points, g, pending, st, gEnd, pendingEnd, stEnd, h => by
      rw [rewriteCoreInstrs] at h
      change (rewriteInstr sums d g pending i st).bind (fun first =>
        (processDeaths Δ d src first.1.1 liveAfter first.1.2 first.2).bind
          (fun second => rewriteCoreInstrs sums Δ d src points
            first.1.1 second.1 second.2)) =
        .ok ((gEnd, pendingEnd), stEnd) at h
      obtain ⟨first, hrewrite, h⟩ := Except.bind_ok_inv h
      obtain ⟨⟨g', pending'⟩, st'⟩ := first
      obtain ⟨second, hdeaths, hrest⟩ := Except.bind_ok_inv h
      obtain ⟨pending'', st''⟩ := second
      exact .cons hrewrite (processDeaths_trace hdeaths)
        (rewriteCoreInstrs_trace hrest)

/-- An instruction trace retains every block finished before it. -/
theorem CoreInstrTrace.done_subset {sums : Summaries} {Δ : StructDecls}
    {d : FunDecl} {src : BlockId} {points : List (Instr × LiveSet)}
    {g gEnd : BGraph} {pending pendingEnd : List LocalIndex}
    {st stEnd : EmitSt}
    (trace : CoreInstrTrace sums Δ d src points g pending st
      gEnd pendingEnd stEnd) :
    ∀ p ∈ st.done, p ∈ stEnd.done := by
  induction trace with
  | nil => exact fun _ hp => hp
  | @cons i liveAfter points g g' gEnd pending pending' pending''
      pendingEnd st st' st'' stEnd rewrite deaths rest ih =>
      intro p hp
      have hp' := rewriteInstr_preservesDone sums d g pending i
        st (g', pending') st' rewrite p hp
      exact ih p (deaths.done_subset p hp')

/-- Decomposition of one successful `rewriteBlock`: preparation, the
instruction trace boundary, and the independently certified terminator
phase. -/
structure CoreBlockRewriteInv (sums : Summaries) (Δ : StructDecls)
    (d : FunDecl) (liveIn : Array LiveSet) (b : BlockId) (blk : Block)
    (g₀ : BGraph) (st stEnd : EmitSt) : Prop where
  output : ∃ g : BGraph, ∃ pending : List LocalIndex, ∃ stInstr : EmitSt,
    rewriteCoreInstrs sums Δ d b
        (blk.instrs.zip
          (liveAfterEach (liveAtTermIn liveIn blk) blk.instrs))
        g₀ (coreEntryPending d liveIn b g₀)
        (prepareCoreBlock d liveIn b g₀ st) = .ok ((g, pending), stInstr) ∧
      finishCoreBlock Δ d liveIn b g pending blk.term stInstr =
        .ok ((), stEnd) ∧
      blockEmitted b stEnd = true

/-- Extract the block decomposition from a successful stateful rewrite. -/
theorem rewriteBlock_core_inv {sums : Summaries} {Δ : StructDecls}
    {d : FunDecl} {liveIn : Array LiveSet} {b : BlockId} {blk : Block}
    {g₀ : BGraph} {st stEnd : EmitSt}
    (h : rewriteBlock sums Δ d liveIn b blk g₀ st = .ok ((), stEnd)) :
    CoreBlockRewriteInv sums Δ d liveIn b blk g₀ st stEnd := by
  unfold rewriteBlock at h
  simp only [bind, StateT.bind, EM.lift] at h
  have hcok : checkJoinedRefParents d (liveIn.getD b ∅) g₀ = .ok () := by
    cases hc : checkJoinedRefParents d (liveIn.getD b ∅) g₀ with
    | error e =>
      rw [hc] at h
      simp only [Except.map, Except.bind] at h
      cases h
    | ok u =>
      cases u
      exact rfl
  rw [hcok] at h
  simp only [Except.map, Except.bind] at h
  change (rewriteCoreInstrs sums Δ d b
      (blk.instrs.zip
        (liveAfterEach (liveAtTermIn liveIn blk) blk.instrs))
      g₀ (coreEntryPending d liveIn b g₀)
      (prepareCoreBlock d liveIn b g₀ st)).bind (fun result =>
        (finishCoreBlock Δ d liveIn b result.1.1 result.1.2 blk.term
          result.2).bind (fun finished => ensureBlockEmitted b finished.2)) =
            .ok ((), stEnd) at h
  obtain ⟨result, hinstrs, h⟩ := Except.bind_ok_inv h
  obtain ⟨⟨g, pending⟩, stInstr⟩ := result
  obtain ⟨finished, hfinish, hemitted⟩ := Except.bind_ok_inv h
  obtain ⟨u, stFinished⟩ := finished
  cases u
  unfold ensureBlockEmitted at hemitted
  by_cases hyes : blockEmitted b stFinished = true
  · have hstate : stFinished = stEnd := by
      simpa [hyes] using hemitted
    subst stEnd
    exact ⟨g, pending, stInstr, hinstrs, hfinish, hyes⟩
  · simp [hyes] at hemitted

/-- Project the instruction-level trace and successful terminator phase from
one block rewrite certificate. -/
theorem CoreBlockRewriteInv.instrTrace {sums : Summaries} {Δ : StructDecls}
    {d : FunDecl} {liveIn : Array LiveSet} {b : BlockId} {blk : Block}
    {g₀ : BGraph} {st stEnd : EmitSt}
    (h : CoreBlockRewriteInv sums Δ d liveIn b blk g₀ st stEnd) :
    ∃ g : BGraph, ∃ pending : List LocalIndex, ∃ stInstr : EmitSt,
      CoreInstrTrace sums Δ d b
        (blk.instrs.zip
          (liveAfterEach (liveAtTermIn liveIn blk) blk.instrs))
        g₀ (coreEntryPending d liveIn b g₀)
        (prepareCoreBlock d liveIn b g₀ st) g pending stInstr ∧
      finishCoreBlock Δ d liveIn b g pending blk.term stInstr =
        .ok ((), stEnd) := by
  obtain ⟨g, pending, stInstr, hinstrs, hfinish, -⟩ := h.output
  exact ⟨g, pending, stInstr, rewriteCoreInstrs_trace hinstrs, hfinish⟩

/-- Proof trace for the stateful block traversal.  A constructor records
either a source CFG gap or the exact successful `rewriteBlock` transition for
one declared block; the tail starts in the state produced by that transition.
This is the core-pass analogue of `ImmSuffix`, at block granularity. -/
inductive CoreBlockTrace (sums : Summaries) (Δ : StructDecls) (d : FunDecl)
    (liveIn : Array LiveSet) (graphs : Array BGraph) :
    List BlockId → EmitSt → EmitSt → Prop where
  | nil {st : EmitSt} : CoreBlockTrace sums Δ d liveIn graphs [] st st
  | gap {b : BlockId} {bs : List BlockId} {st stEnd : EmitSt}
      (block : d.body.blocks b = none)
      (rest : CoreBlockTrace sums Δ d liveIn graphs bs st stEnd) :
      CoreBlockTrace sums Δ d liveIn graphs (b :: bs) st stEnd
  | block {b : BlockId} {bs : List BlockId} {blk : Block}
      {st st' stEnd : EmitSt}
      (source : d.body.blocks b = some blk)
      (rewrite : rewriteBlock sums Δ d liveIn b blk (graphs.getD b []) st =
        .ok ((), st'))
      (rest : CoreBlockTrace sums Δ d liveIn graphs bs st' stEnd) :
      CoreBlockTrace sums Δ d liveIn graphs (b :: bs) st stEnd

/-- A successful block-emitter traversal yields its compact proof trace. -/
theorem emitCoreBlocks_trace {sums : Summaries} {Δ : StructDecls}
    {d : FunDecl} {liveIn : Array LiveSet} {graphs : Array BGraph} :
    ∀ {bs : List BlockId} {st stEnd : EmitSt},
      emitCoreBlocks sums Δ d liveIn graphs bs st = .ok ((), stEnd) →
      CoreBlockTrace sums Δ d liveIn graphs bs st stEnd
  | [], st, stEnd, h => by
      simp only [emitCoreBlocks, pure, StateT.pure, Except.pure,
        Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨-, rfl⟩ := h
      exact .nil
  | b :: bs, st, stEnd, h => by
      rw [emitCoreBlocks] at h
      cases hb : d.body.blocks b with
      | none =>
          rw [hb] at h
          exact .gap hb (emitCoreBlocks_trace h)
      | some blk =>
          rw [hb] at h
          change (rewriteBlock sums Δ d liveIn b blk (graphs.getD b []) st).bind
            (fun p => emitCoreBlocks sums Δ d liveIn graphs bs p.2) =
              .ok ((), stEnd) at h
          obtain ⟨p, hp, hrest⟩ := Except.bind_ok_inv h
          obtain ⟨u, st'⟩ := p
          cases u
          exact .block hb hp (emitCoreBlocks_trace hrest)

/-- Project the exact emitter transition for any declared block visited by a
traversal trace. -/
theorem CoreBlockTrace.rewrite_of_mem {sums : Summaries} {Δ : StructDecls}
    {d : FunDecl} {liveIn : Array LiveSet} {graphs : Array BGraph}
    {bs : List BlockId} {st stEnd : EmitSt}
    (h : CoreBlockTrace sums Δ d liveIn graphs bs st stEnd)
    {b : BlockId} {blk : Block} (hmem : b ∈ bs)
    (hblk : d.body.blocks b = some blk) :
    ∃ before after : EmitSt,
      rewriteBlock sums Δ d liveIn b blk (graphs.getD b []) before =
        .ok ((), after) := by
  induction h with
  | nil => simp at hmem
  | gap hgap rest ih =>
      rcases List.mem_cons.mp hmem with rfl | htail
      · rw [hgap] at hblk
        cases hblk
      · exact ih htail
  | block hsource hrewrite rest ih =>
      rcases List.mem_cons.mp hmem with rfl | htail
      · rw [hsource] at hblk
        cases hblk
        exact ⟨_, _, hrewrite⟩
      · exact ih htail

/-- A traversal trace retains every block which was finished before the
traversal began. -/
theorem CoreBlockTrace.done_subset {sums : Summaries} {Δ : StructDecls}
    {d : FunDecl} {liveIn : Array LiveSet} {graphs : Array BGraph}
    {bs : List BlockId} {st stEnd : EmitSt}
    (h : CoreBlockTrace sums Δ d liveIn graphs bs st stEnd) :
    ∀ p ∈ st.done, p ∈ stEnd.done := by
  induction h with
  | nil => exact fun _ hp => hp
  | gap _ _ ih => exact ih
  | block _ hrewrite _ ih =>
      intro p hp
      apply ih p
      exact (rewriteBlock_preservesDone sums Δ d liveIn _ _ _)
        _ _ _ hrewrite p hp

/-- Project a declared block rewrite together with the fact that all blocks
it finishes remain in the traversal's final emitter state. -/
theorem CoreBlockTrace.rewrite_of_mem_retained
    {sums : Summaries} {Δ : StructDecls}
    {d : FunDecl} {liveIn : Array LiveSet} {graphs : Array BGraph}
    {bs : List BlockId} {st stEnd : EmitSt}
    (h : CoreBlockTrace sums Δ d liveIn graphs bs st stEnd)
    {b : BlockId} {blk : Block} (hmem : b ∈ bs)
    (hblk : d.body.blocks b = some blk) :
    ∃ before after : EmitSt,
      rewriteBlock sums Δ d liveIn b blk (graphs.getD b []) before =
        .ok ((), after) ∧
      ∀ p ∈ after.done, p ∈ stEnd.done := by
  induction h with
  | nil => simp at hmem
  | gap hgap rest ih =>
      rcases List.mem_cons.mp hmem with rfl | htail
      · rw [hgap] at hblk
        cases hblk
      · exact ih htail
  | block hsource hrewrite rest ih =>
      rcases List.mem_cons.mp hmem with rfl | htail
      · rw [hsource] at hblk
        cases hblk
        exact ⟨_, _, hrewrite, rest.done_subset⟩
      · exact ih htail

/-- Densification preserves the block selected at every in-range block id. -/
theorem denseCoreBlocks_at {d : FunDecl} {st : EmitSt} {blocks : List Block}
    (h : denseCoreBlocks d st = .ok blocks) {b : BlockId}
    (hlt : b < max d.body.size st.nextId) :
    ∃ blk, blocks[b]? = some blk ∧ denseCoreBlock d st.done b = .ok blk := by
  unfold denseCoreBlocks at h
  have hrange : (List.range (max d.body.size st.nextId))[b]? = some b := by
    simp [hlt]
  exact Except.mapM_getElem? h hrange

/-- A finished emitter entry is selected verbatim by densification. -/
theorem denseCoreBlock_of_find {d : FunDecl}
    {done : List (BlockId × Block)} {b : BlockId} {blk : Block}
    (h : done.find? (·.1 == b) = some (b, blk)) :
    denseCoreBlock d done b = .ok blk := by
  simp [denseCoreBlock, h, pure, Except.pure]

/-- In a validated append-only emitter trace, lookup by a recorded block id
returns that exact block. -/
theorem emittedBlocksValid_find {done : List (BlockId × Block)}
    (hvalid : emittedBlocksValid done = true)
    {b : BlockId} {blk : Block} (hmem : (b, blk) ∈ done) :
    done.find? (·.1 == b) = some (b, blk) := by
  induction done with
  | nil => simp at hmem
  | cons p rest ih =>
      simp only [emittedBlocksValid, Bool.and_eq_true] at hvalid
      rcases List.mem_cons.mp hmem with heq | hrest
      · subst p
        simp
      · have hp : p.1 ≠ b := by
          intro heq
          subst b
          have : rest.any (fun q => q.1 == p.1) = true :=
            List.any_eq_true.mpr ⟨(p.1, blk), hrest, by simp⟩
          have hany : rest.any (fun q => q.1 == p.1) = false := by
            cases heq : rest.any (fun q => q.1 == p.1) <;>
              simp_all
          rw [hany] at this
          contradiction
        have hpbeq : (p.1 == b) = false := by simp [hp]
        rw [List.find?, hpbeq]
        exact ih hvalid.2 hrest

/-- A successful mutation-value elimination retained at its first-order
`ElimOut` boundary.  Keeping this witness avoids repeatedly unfolding the
stateful emitter after `ElimOut.toFunDecl` has hidden block provenance and
fresh-local allocation behind function fields. -/
structure ElimCoreInv (sums : Summaries) (Δ : StructDecls)
    (d d' : FunDecl) : Prop where
  output : ∃ out : ElimOut,
    elimCoreOut sums Δ d = .ok out ∧
      d' = out.toFunDecl d.loopSpecs

/-- Extract the structural core-pass certificate from a successful result. -/
theorem elimCore_inv {sums : Summaries} {Δ : StructDecls}
    {d d' : FunDecl} (h : elimCore sums Δ d = .ok d') :
    ElimCoreInv sums Δ d d' := by
  unfold elimCore at h
  cases hout : elimCoreOut sums Δ d with
  | error e => simp [hout, Except.bind, bind] at h
  | ok out =>
      simp only [hout, Except.bind, bind, pure, Except.pure,
        Except.ok.injEq] at h
      exact ⟨out, hout, h.symm⟩

/-- Analysis, traversal, and densification facts retained from a successful
first-order core emission.  This is the proof-facing certificate hidden by
the final `ElimOut.toFunDecl` conversion. -/
structure ElimCoreOutInv (sums : Summaries) (Δ : StructDecls)
    (d : FunDecl) (out : ElimOut) : Prop where
  numParams_eq : out.numParams = d.numParams
  entry_eq : out.entry = d.body.entry
  live_stable : liveStable d (liveAnalysis d) = true
  graph_stable :
    graphStable d (graphThroughBlock sums d) (borrowAnalysis sums d) = true
  emission : ∃ st : EmitSt,
    emitCoreBlocks sums Δ d (liveAnalysis d) (borrowAnalysis sums d)
        (List.range d.body.size) (initialEmitState d) = .ok ((), st) ∧
      denseCoreBlocks d st = .ok out.blocks ∧
      emittedBlocksValid st.done = true ∧
      out.blockSrc = st.blockSrc

/-- Extract the proof-facing certificate from successful first-order output. -/
theorem elimCoreOut_inv {sums : Summaries} {Δ : StructDecls}
    {d : FunDecl} {out : ElimOut}
    (h : elimCoreOut sums Δ d = .ok out) :
    ElimCoreOutInv sums Δ d out := by
  unfold elimCoreOut at h
  simp only [pure, Except.pure, Except.bind, bind] at h
  split at h
  · simp_all
  · by_cases hlive : liveStable d (liveAnalysis d) = true
    · simp only [hlive, if_true] at h
      by_cases hgraph :
          graphStable d (graphThroughBlock sums d)
            (borrowAnalysis sums d) = true
      · simp only [hgraph, if_true] at h
        cases hemit : emitCoreBlocks sums Δ d (liveAnalysis d)
            (borrowAnalysis sums d) (List.range d.body.size)
            (initialEmitState d) with
        | error e => simp [hemit] at h
        | ok p =>
          obtain ⟨u, st⟩ := p
          cases u
          cases hdense : denseCoreBlocks d st with
          | error e => simp [hemit, hdense] at h
          | ok dense =>
            by_cases hvalid : emittedBlocksValid st.done = true
            · cases horig : (List.range d.numLocals).mapM (localTy d) with
              | error e => simp [hemit, hdense, hvalid, horig] at h
              | ok origTys =>
                cases hfinals : (mutParamsOf d).mapM (localTy d) with
                | error e =>
                    simp [hemit, hdense, hvalid, horig, hfinals] at h
                | ok finalsTys =>
                  simp only [hemit, hdense, hvalid, horig, hfinals,
                    if_true] at h
                  cases h
                  exact ⟨rfl, rfl, hlive, hgraph, st, hemit, hdense,
                    hvalid, rfl⟩
            · simp [hemit, hdense, hvalid] at h
      · simp [hgraph] at h
    · simp [hlive] at h

/-- Core elimination preserves the external parameter count. -/
theorem ElimCoreInv.numParams_eq {sums : Summaries} {Δ : StructDecls}
    {d d' : FunDecl} (h : ElimCoreInv sums Δ d d') :
    d'.numParams = d.numParams := by
  obtain ⟨out, hemitted, hout⟩ := h.output
  rw [hout]
  exact (elimCoreOut_inv hemitted).numParams_eq

/-- Core elimination preserves the entry block identifier. -/
theorem ElimCoreInv.entry_eq {sums : Summaries} {Δ : StructDecls}
    {d d' : FunDecl} (h : ElimCoreInv sums Δ d d') :
    d'.body.entry = d.body.entry := by
  obtain ⟨out, hemitted, hout⟩ := h.output
  rw [hout]
  exact (elimCoreOut_inv hemitted).entry_eq

/-- Expose the successful stateful traversal together with its block-by-block
proof trace and the densification equation used by the final CFG. -/
theorem ElimCoreInv.emission {sums : Summaries} {Δ : StructDecls}
    {d d' : FunDecl} (h : ElimCoreInv sums Δ d d') :
    ∃ out : ElimOut, ∃ st : EmitSt,
      d' = out.toFunDecl d.loopSpecs ∧
      denseCoreBlocks d st = .ok out.blocks ∧
      out.blockSrc = st.blockSrc ∧
      CoreBlockTrace sums Δ d (liveAnalysis d) (borrowAnalysis sums d)
        (List.range d.body.size) (initialEmitState d) st := by
  obtain ⟨out, hemitted, hout⟩ := h.output
  obtain ⟨st, hemit, hdense, -, hsrc⟩ :=
    (elimCoreOut_inv hemitted).emission
  exact ⟨out, st, hout, hdense, hsrc, emitCoreBlocks_trace hemit⟩

/-- Expose the stateful traversal together with the uniqueness validation
which makes every retained emitter entry an exact output-CFG block. -/
theorem ElimCoreInv.emissionValidated {sums : Summaries} {Δ : StructDecls}
    {d d' : FunDecl} (h : ElimCoreInv sums Δ d d') :
    ∃ out : ElimOut, ∃ st : EmitSt,
      d' = out.toFunDecl d.loopSpecs ∧
      denseCoreBlocks d st = .ok out.blocks ∧
      emittedBlocksValid st.done = true ∧
      CoreBlockTrace sums Δ d (liveAnalysis d) (borrowAnalysis sums d)
        (List.range d.body.size) (initialEmitState d) st := by
  obtain ⟨out, hemitted, hout⟩ := h.output
  obtain ⟨st, hemit, hdense, hvalid, -⟩ :=
    (elimCoreOut_inv hemitted).emission
  exact ⟨out, st, hout, hdense, hvalid, emitCoreBlocks_trace hemit⟩

/-- Every block retained in the final emitter state is selected verbatim by
the output CFG's dense block list. -/
theorem ElimCoreInv.emittedValid {sums : Summaries} {Δ : StructDecls}
    {d d' : FunDecl} (h : ElimCoreInv sums Δ d d') :
    ∃ out : ElimOut, ∃ st : EmitSt,
      d' = out.toFunDecl d.loopSpecs ∧
      emittedBlocksValid st.done = true := by
  obtain ⟨out, hemitted, hout⟩ := h.output
  obtain ⟨st, -, -, hvalid, -⟩ := (elimCoreOut_inv hemitted).emission
  exact ⟨out, st, hout, hvalid⟩

/-- The successful core certificate contains convergence of both analyses
used by the stateful rewrite. -/
theorem ElimCoreInv.analysisStable {sums : Summaries} {Δ : StructDecls}
    {d d' : FunDecl} (h : ElimCoreInv sums Δ d d') :
    liveStable d (liveAnalysis d) = true ∧
      graphStable d (graphThroughBlock sums d) (borrowAnalysis sums d) = true := by
  obtain ⟨out, hemitted, -⟩ := h.output
  exact ⟨(elimCoreOut_inv hemitted).live_stable,
    (elimCoreOut_inv hemitted).graph_stable⟩

/-- Project the successful state transition for one declared source block. -/
theorem ElimCoreInv.rewriteBlock {sums : Summaries} {Δ : StructDecls}
    {d d' : FunDecl} (h : ElimCoreInv sums Δ d d')
    {b : BlockId} {blk : Block} (hlt : b < d.body.size)
    (hblk : d.body.blocks b = some blk) :
    ∃ before after : EmitSt,
      MoveModel.IR.rewriteBlock sums Δ d (liveAnalysis d) b blk
        ((borrowAnalysis sums d).getD b []) before = .ok ((), after) := by
  obtain ⟨out, st, -, -, -, htrace⟩ := h.emission
  exact htrace.rewrite_of_mem (List.mem_range.mpr hlt) hblk

/-- Project the complete instruction/terminator decomposition for a declared
source block directly from the whole-function certificate. -/
theorem ElimCoreInv.blockRewrite {sums : Summaries} {Δ : StructDecls}
    {d d' : FunDecl} (h : ElimCoreInv sums Δ d d')
    {b : BlockId} {blk : Block} (hlt : b < d.body.size)
    (hblk : d.body.blocks b = some blk) :
    ∃ before after : EmitSt,
      CoreBlockRewriteInv sums Δ d (liveAnalysis d) b blk
        ((borrowAnalysis sums d).getD b []) before after := by
  obtain ⟨before, after, hrewrite⟩ := h.rewriteBlock hlt hblk
  exact ⟨before, after, rewriteBlock_core_inv hrewrite⟩

/-- Entering a declared source block exposes both its verbatim dense target
block and the complete rewrite certificate which emitted it. -/
theorem ElimCoreInv.block {sums : Summaries} {Δ : StructDecls}
    {d d' : FunDecl} (h : ElimCoreInv sums Δ d d')
    {b : BlockId} {blk : Block} (hlt : b < d.body.size)
    (hblk : d.body.blocks b = some blk) :
    ∃ target : Block, ∃ before after : EmitSt,
      d'.body.blocks b = some target ∧
      CoreBlockRewriteInv sums Δ d (liveAnalysis d) b blk
        ((borrowAnalysis sums d).getD b []) before after := by
  obtain ⟨out, st, hout, hdense, hvalid, htrace⟩ := h.emissionValidated
  have hlt' : b < max d.body.size st.nextId :=
    Nat.lt_of_lt_of_le hlt (Nat.le_max_left _ _)
  obtain ⟨before, after, hrewrite, hretained⟩ :=
    htrace.rewrite_of_mem_retained (List.mem_range.mpr hlt) hblk
  have hinv := rewriteBlock_core_inv hrewrite
  obtain ⟨_, _, _, _, _, hemitted⟩ := hinv.output
  obtain ⟨target, htargetAfter⟩ := blockEmitted_mem hemitted
  have htargetFinal : (b, target) ∈ st.done :=
    hretained (b, target) htargetAfter
  have hfind := emittedBlocksValid_find hvalid htargetFinal
  have hdenseTarget := denseCoreBlock_of_find (d := d) hfind
  obtain ⟨selected, hselected, hdenseSelected⟩ :=
    denseCoreBlocks_at hdense hlt'
  rw [hdenseTarget] at hdenseSelected
  cases hdenseSelected
  refine ⟨target, before, after, ?_, hinv⟩
  rw [hout]
  exact hselected

/-- Expose one stable first-order result for all block-level reasoning. -/
theorem ElimCoreInv.withOutput {sums : Summaries} {Δ : StructDecls}
    {d d' : FunDecl} (h : ElimCoreInv sums Δ d d') :
    ∃ out : ElimOut,
      elimCoreOut sums Δ d = .ok out ∧
      d'.body.size = out.blocks.length ∧
      (∀ b, d'.body.blocks b = out.blocks[b]?) ∧
      d'.returns = out.returns ∧
      d' = out.toFunDecl d.loopSpecs := by
  obtain ⟨out, hemitted, hout⟩ := h.output
  refine ⟨out, hemitted, ?_, ?_, ?_, hout⟩ <;>
    rw [hout] <;> simp [ElimOut.toFunDecl]

/-- A split block belongs to the source block recorded by the retained
first-order emitter result. -/
def ElimCoreInv.SplitOrigin {sums : Summaries} {Δ : StructDecls}
    {d d' : FunDecl} (split source : BlockId) : Prop :=
  ∃ out, elimCoreOut sums Δ d = .ok out ∧
    d' = out.toFunDecl d.loopSpecs ∧
    (split, source) ∈ out.blockSrc

/-! ## Mutation-value state relation -/

/-- A concrete source reference target realizes one static borrow edge.
Reference-parent edges compare the dynamic path suffix; root edges compare
the frame-qualified root and their complete path pattern. -/
def CoreEdgeMatches (s : MoveState) (rt : RefTarget) (e : BEdge) : Prop :=
  match e.parent with
  | .refNode p => ∃ parent,
      s.locals p = some (.ref parent) ∧
      isParentTarget (bPathPattern e.path) parent rt = true
  | .localRoot x =>
      rt.root = .loc s.current x ∧
      pathMatches (bPathPattern e.path) rt.path = true
  | .globalRoot r => ∃ a,
      rt.root = .global r a ∧
      pathMatches (bPathPattern e.path) rt.path = true
  | .anyRoot => True

/-- A pending child dynamically realizes a reference-parent edge from `p`.
Unlike `hasPendingChild`, this semantic predicate ignores may-edges which do
not match the concrete runtime targets. -/
def CoreHasPendingChild (g : BGraph) (pending : List LocalIndex)
    (s : MoveState) (p : LocalIndex) : Prop :=
  ∃ c ∈ pending, c ≠ p ∧ ∃ e ∈ inEdges g c,
    e.parent = .refNode p ∧ ∃ rt,
      s.locals c = some (.ref rt) ∧ CoreEdgeMatches s rt e

/-- Exactly one may-edge realizes a concrete reference at runtime.  This is
the dynamic uniqueness projection of borrow correctness and rules out two
overlapping guarded write-backs along different patterns of the same parent. -/
def CoreOriginUnique (g : BGraph) (s : MoveState)
    (t : LocalIndex) : Prop :=
  ∀ e₁, e₁ ∈ inEdges g t → ∀ e₂, e₂ ∈ inEdges g t →
    ∀ rt, CoreEdgeMatches s rt e₁ → CoreEdgeMatches s rt e₂ →
      e₁ = e₂

/-- Some pending source reference has the given concrete root.  This runtime
coverage is needed for input references, whose frame-qualified roots are not
known to the callee's static borrow graph. -/
def PendingRoot (pending : List LocalIndex) (s : MoveState)
    (root : RefRoot) : Prop :=
  ∃ t ∈ pending, ∃ rt, s.locals t = some (.ref rt) ∧ rt.root = root

/-- Writing a non-pending slot leaves the concrete roots represented by the
pending reference slots unchanged. -/
theorem PendingRoot.writeLocal_iff {pending : List LocalIndex}
    {s : MoveState} {x : LocalIndex} {v : Value} (hx : x ∉ pending)
    {root : RefRoot} :
    PendingRoot pending (s.writeLocal x v) root ↔
      PendingRoot pending s root := by
  constructor <;> rintro ⟨t, ht, rt, href, hroot⟩
  · refine ⟨t, ht, rt, ?_, hroot⟩
    simpa [MoveState.writeLocal_locals, ne_of_mem_of_not_mem ht hx] using href
  · refine ⟨t, ht, rt, ?_, hroot⟩
    simpa [MoveState.writeLocal_locals, ne_of_mem_of_not_mem ht hx] using href

/-- Active-frame relation for the core pass.  Unborrowed ordinary roots and
global resource types agree directly.  Each pending source reference is a
target mutation at the same frame-qualified location; when it has no pending
child, its carried value is the current source dereference.  Parent payloads
may be stale precisely while a child is pending. -/
structure CoreFrameRel (d : FunDecl) (g : BGraph)
    (pending : List LocalIndex) (s s' : MoveState) : Prop where
  current_eq : s'.current = s.current
  pending_bound : ∀ t, t ∈ pending → t < d.numLocals
  pending_mut : ∀ t, t ∈ pending → isMutLocal d t = true
  plain : ∀ x, x < d.numLocals → isMutLocal d x = false →
    ¬PendingRoot pending s (.loc s.current x) →
    s'.locals x = s.locals x
  mutation : ∀ t, t ∈ pending →
    (s.locals t = none ∧ s'.locals t = none) ∨
      ∃ rt v, s.locals t = some (.ref rt) ∧
        s'.locals t = some (.mut rt v) ∧
        (t < d.numParams ∨
          ∃ e ∈ inEdges g t, CoreEdgeMatches s rt e) ∧
        (¬CoreHasPendingChild g pending s t →
          s.readTarget rt = some v)
  globals : ∀ r a, ¬PendingRoot pending s (.global r a) →
    s'.memory r a = s.memory r a

/-- Identical states are related when no mutation is pending. -/
theorem CoreFrameRel.refl_empty (d : FunDecl) (g : BGraph) (s : MoveState) :
    CoreFrameRel d g [] s s := by
  refine ⟨rfl, ?_, ?_, ?_, ?_, ?_⟩
  · simp
  · simp
  · intro x _ _ _
    rfl
  · simp
  · intro r a _
    rfl

/-- Identical initial states are related for a may-pending set whose entries
are declared mutable references beyond the initialized argument range. -/
theorem CoreFrameRel.initial (d : FunDecl) (g : BGraph)
    (pending : List LocalIndex) (args : List Value) (m : Memory)
    (hbound : ∀ t ∈ pending, t < d.numLocals)
    (hmut : ∀ t ∈ pending, isMutLocal d t = true)
    (huninit : ∀ t ∈ pending, args.length ≤ t) :
    CoreFrameRel d g pending (MoveState.initial args m)
      (MoveState.initial args m) := by
  refine ⟨rfl, hbound, hmut, ?_, ?_, ?_⟩
  · intro x _ _ _
    rfl
  · intro t ht
    left
    have hnone : args[t]? = none :=
      List.getElem?_eq_none (huninit t ht)
    constructor <;> simpa [MoveState.initial, initLocals] using hnone
  · intro r a _
    rfl

/-- An unborrowed ordinary local has the same lookup in related states. -/
theorem CoreFrameRel.lookup_plain {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {s s' : MoveState}
    (h : CoreFrameRel d g pending s s') {x : LocalIndex}
    (hrange : x < d.numLocals)
    (hplain : isMutLocal d x = false)
    (hroot : ¬PendingRoot pending s (.loc s.current x)) :
    s'.locals x = s.locals x :=
  h.plain x hrange hplain hroot

/-- A pending leaf reference exposes a target mutation carrying its current
source dereference and a static origin (unless it is an input reference). -/
theorem CoreFrameRel.leaf {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {s s' : MoveState}
    (h : CoreFrameRel d g pending s s') {t : LocalIndex}
    (ht : t ∈ pending) (hleaf : hasPendingChild g pending t = false)
    {rt₀ : RefTarget} (hpresent : s.locals t = some (.ref rt₀)) :
    ∃ rt v, s.locals t = some (.ref rt) ∧
      s'.locals t = some (.mut rt v) ∧
      (t < d.numParams ∨ ∃ e ∈ inEdges g t, CoreEdgeMatches s rt e) ∧
      s.readTarget rt = some v := by
  rcases h.mutation t ht with habsent |
      ⟨rt, v, href, hmut, horigin, hread⟩
  · rw [hpresent] at habsent
    cases habsent.1
  · refine ⟨rt, v, href, hmut, horigin, hread ?_⟩
    rintro ⟨c, hc, hcne, e, he, heparent, -⟩
    exact noPendingChild_of_false hleaf hc hcne he heparent

/-- An unborrowed global resource type has the same memory in related states. -/
theorem CoreFrameRel.memory_plain {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {s s' : MoveState}
    (h : CoreFrameRel d g pending s s') {r : ResourceId} {a : Address}
    (hroot : ¬PendingRoot pending s (.global r a)) :
    s'.memory r a = s.memory r a :=
  h.globals r a hroot

/-- The core frame relation depends on the target only through its current
frame, source-local range, and memory.  Emitter temporaries above that range
are therefore observationally irrelevant to the relation. -/
theorem CoreFrameRel.target_congr {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {s target target' : MoveState}
    (h : CoreFrameRel d g pending s target)
    (hcurrent : target'.current = target.current)
    (hlocals : ∀ x, x < d.numLocals →
      target'.locals x = target.locals x)
    (hmemory : target'.memory = target.memory) :
    CoreFrameRel d g pending s target' := by
  refine ⟨hcurrent.trans h.current_eq, h.pending_bound, h.pending_mut,
    ?_, ?_, ?_⟩
  · intro x hx hplain hroot
    rw [hlocals x hx]
    exact h.plain x hx hplain hroot
  · intro t ht
    rcases h.mutation t ht with ⟨hsrc, htgt⟩ |
        ⟨rt, v, href, hmut, horigin, hread⟩
    · exact Or.inl ⟨hsrc, by
        rw [hlocals t (h.pending_bound t ht)]
        exact htgt⟩
    · exact Or.inr ⟨rt, v, href, by
        rw [hlocals t (h.pending_bound t ht)]
        exact hmut, horigin, hread⟩
  · intro r a hroot
    rw [hmemory]
    exact h.globals r a hroot

/-- Writing an emitter temporary above the source-local range preserves the
core relation. -/
theorem CoreFrameRel.writeTargetFresh {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {s s' : MoveState}
    (h : CoreFrameRel d g pending s s') {x : LocalIndex} {v : Value}
    (hfresh : d.numLocals ≤ x) :
    CoreFrameRel d g pending s (s'.writeLocal x v) := by
  refine ⟨by simp [h.current_eq], h.pending_bound, h.pending_mut,
    ?_, ?_, ?_⟩
  · intro y hyrange hyplain hyroot
    have hyx : y ≠ x := Nat.ne_of_lt (Nat.lt_of_lt_of_le hyrange hfresh)
    simpa [MoveState.writeLocal_locals, hyx] using
      h.plain y hyrange hyplain hyroot
  · intro t ht
    have htx : t ≠ x := Nat.ne_of_lt
      (Nat.lt_of_lt_of_le (h.pending_bound t ht) hfresh)
    rcases h.mutation t ht with ⟨hsrc, htgt⟩ |
        ⟨rt, w, href, hmut, horigin, hread⟩
    · exact Or.inl ⟨hsrc, by
        simpa [MoveState.writeLocal_locals, htx] using htgt⟩
    · exact Or.inr ⟨rt, w, href, by
        simpa [MoveState.writeLocal_locals, htx] using hmut,
        horigin, hread⟩
  · intro r a hroot
    simpa using h.globals r a hroot

/-- Synchronized writes of a reference-free ordinary local preserve the core
relation when that local is not the root of a pending mutation. -/
theorem CoreFrameRel.writeLocalSame {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {s s' : MoveState}
    (h : CoreFrameRel d g pending s s') {x : LocalIndex} {v : Value}
    (hplain : isMutLocal d x = false)
    (hroot : ¬PendingRoot pending s (.loc s.current x))
    (hmatches : ∀ t, t ∈ pending → ∀ rt e,
      e ∈ inEdges g t → CoreEdgeMatches s rt e →
        CoreEdgeMatches (s.writeLocal x v) rt e) :
    CoreFrameRel d g pending (s.writeLocal x v) (s'.writeLocal x v) := by
  have hxPending : x ∉ pending := by
    intro hx
    have := h.pending_mut x hx
    simp [hplain] at this
  refine ⟨by simp [h.current_eq], h.pending_bound, h.pending_mut,
    ?_, ?_, ?_⟩
  · intro y hyrange hyplain hyroot
    by_cases hyx : y = x
    · subst y
      simp
    · have holdRoot : ¬PendingRoot pending s (.loc s.current y) := by
        intro hold
        apply hyroot
        rw [PendingRoot.writeLocal_iff hxPending]
        simpa using hold
      simpa [MoveState.writeLocal_locals, hyx] using
        h.plain y hyrange hyplain holdRoot
  · intro t ht
    have htx : t ≠ x := ne_of_mem_of_not_mem ht hxPending
    rcases h.mutation t ht with ⟨hsrc, htgt⟩ |
        ⟨rt, w, href, hmut, horigin, hread⟩
    · exact Or.inl ⟨by
        simpa [MoveState.writeLocal_locals, htx] using hsrc, by
        simpa [MoveState.writeLocal_locals, htx] using htgt⟩
    · refine Or.inr ⟨rt, w, by
        simpa [MoveState.writeLocal_locals, htx] using href, by
        simpa [MoveState.writeLocal_locals, htx] using hmut,
        ?_, ?_⟩
      · exact horigin.elim Or.inl fun ⟨e, he, hmatch⟩ =>
          Or.inr ⟨e, he, hmatches t ht rt e he hmatch⟩
      intro hleaf
      have hrt : rt.root ≠ .loc s.current x := by
        intro heq
        exact hroot ⟨t, ht, rt, href, heq⟩
      rw [MoveState.readTarget_writeLocal_of_root_ne _ _ _ _ hrt]
      apply hread
      rintro ⟨c, hc, hcne, e, he, heparent, rc, hrc, hmatch⟩
      apply hleaf
      refine ⟨c, hc, hcne, e, he, heparent, rc, ?_, ?_⟩
      · simpa [MoveState.writeLocal_locals,
          ne_of_mem_of_not_mem hc hxPending] using hrc
      · exact hmatches c hc rc e he hmatch
  · intro r a hnot
    apply h.globals r a
    intro hold
    apply hnot
    rw [PendingRoot.writeLocal_iff hxPending]
    simpa using hold

/-- Writing back a pending leaf to its local root and removing that leaf from
the pending set preserves the core frame relation. -/
theorem CoreFrameRel.writeBackLocal {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {s s' : MoveState}
    (h : CoreFrameRel d g pending s s') {t x : LocalIndex}
    {rt : RefTarget} {v : Value}
    (ht : t ∈ pending) (hleaf : hasPendingChild g pending t = false)
    (hsrc : s.locals t = some (.ref rt))
    (htgt : s'.locals t = some (.mut rt v))
    (hroot : rt.root = .loc s.current x) (hpath : rt.path = [])
    (hunique : ∀ e ∈ inEdges g t, CoreEdgeMatches s rt e →
      e.parent = .localRoot x)
    (hxplain : isMutLocal d x = false) :
    CoreFrameRel d g (pending.filter (· ≠ t)) s
      (s'.writeLocal x v) := by
  rcases rt with ⟨root, path⟩
  change root = .loc s.current x at hroot
  change path = [] at hpath
  subst root
  subst path
  have hread : s.readTarget ⟨.loc s.current x, []⟩ = some v := by
    obtain ⟨rt', v', href, hmut, -, hread⟩ := h.leaf ht hleaf hsrc
    rw [hsrc] at href
    cases href
    rw [htgt] at hmut
    cases hmut
    exact hread
  have hx : s.locals x = some v := by
    simpa [MoveState.readTarget, Value.getPath] using hread
  have xtNotPending : x ∉ pending := by
    intro hxmem
    have hxmut := h.pending_mut x hxmem
    rw [hxplain] at hxmut
    contradiction
  refine ⟨by simp [h.current_eq], ?_, ?_, ?_, ?_, ?_⟩
  · intro u hu
    exact h.pending_bound u (List.mem_filter.mp hu).1
  · intro u hu
    exact h.pending_mut u (List.mem_filter.mp hu).1
  · intro y hyrange hyplain hyroot
    by_cases hyx : y = x
    · subst y
      rw [MoveState.writeLocal_locals, if_pos rfl]
      exact hx.symm
    · have holdRoot : ¬PendingRoot pending s (.loc s.current y) := by
        rintro ⟨u, hu, ru, href, hru⟩
        by_cases hut : u = t
        · subst u
          rw [hsrc] at href
          cases href
          simp only [RefRoot.loc.injEq] at hru
          exact hyx hru.2.symm
        · apply hyroot
          exact ⟨u, by simp [hu, hut], ru, href, hru⟩
      simpa [MoveState.writeLocal_locals, hyx] using
        h.plain y hyrange hyplain holdRoot
  · intro u hu
    have huPending := (List.mem_filter.mp hu).1
    have hut : u ≠ t := by simpa using (List.mem_filter.mp hu).2
    have hux : u ≠ x := ne_of_mem_of_not_mem huPending xtNotPending
    rcases h.mutation u huPending with ⟨hsrcNone, htgtNone⟩ |
        ⟨ru, w, href, hmut, huOrigin, hleafRead⟩
    · exact Or.inl ⟨hsrcNone, by
        simpa [MoveState.writeLocal_locals, hux] using htgtNone⟩
    · exact Or.inr ⟨ru, w, href, by
        simpa [MoveState.writeLocal_locals, hux] using hmut,
        huOrigin, fun huLeaf => hleafRead (by
          rintro ⟨c, hc, hcne, e, he, heparent, rc, hrc, hmatch⟩
          by_cases hct : c = t
          · subst c
            rw [hsrc] at hrc
            cases hrc
            have hactual := hunique e he hmatch
            rw [heparent] at hactual
            cases hactual
          · apply huLeaf
            exact ⟨c, by simp [hc, hct], hcne, e, he, heparent,
              rc, hrc, hmatch⟩)⟩
  · intro r a hglobal
    apply h.globals r a
    rintro ⟨u, hu, ru, href, hru⟩
    by_cases hut : u = t
    · subst u
      rw [hsrc] at href
      cases href
      cases hru
    · apply hglobal
      exact ⟨u, by simp [hu, hut], ru, href, hru⟩

/-- Writing back a pending leaf to its global root and removing that leaf
from the pending set preserves the core frame relation. -/
theorem CoreFrameRel.writeBackGlobal {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {s s' : MoveState}
    (h : CoreFrameRel d g pending s s') {t : LocalIndex}
    {r : ResourceId} {a : Address} {rt : RefTarget} {v : Value}
    (ht : t ∈ pending) (hleaf : hasPendingChild g pending t = false)
    (hsrc : s.locals t = some (.ref rt))
    (htgt : s'.locals t = some (.mut rt v))
    (hroot : rt.root = .global r a) (hpath : rt.path = [])
    (hunique : ∀ e ∈ inEdges g t, CoreEdgeMatches s rt e →
      e.parent = .globalRoot r) :
    CoreFrameRel d g (pending.filter (· ≠ t)) s
      (s'.writeGlobal r a v) := by
  rcases rt with ⟨root, path⟩
  change root = .global r a at hroot
  change path = [] at hpath
  subst root
  subst path
  have hread : s.readTarget ⟨.global r a, []⟩ = some v := by
    obtain ⟨rt', v', href, hmut, -, hread⟩ := h.leaf ht hleaf hsrc
    rw [hsrc] at href
    cases href
    rw [htgt] at hmut
    cases hmut
    exact hread
  have hmemory : s.memory r a = some v := by
    simpa [MoveState.readTarget, Value.getPath] using hread
  refine ⟨h.current_eq, ?_, ?_, ?_, ?_, ?_⟩
  · intro u hu
    exact h.pending_bound u (List.mem_filter.mp hu).1
  · intro u hu
    exact h.pending_mut u (List.mem_filter.mp hu).1
  · intro x hxrange hxplain hxroot
    apply h.plain x hxrange hxplain
    rintro ⟨u, hu, ru, href, hru⟩
    by_cases hut : u = t
    · subst u
      rw [hsrc] at href
      cases href
      cases hru
    · apply hxroot
      exact ⟨u, by simp [hu, hut], ru, href, hru⟩
  · intro u hu
    have huPending := (List.mem_filter.mp hu).1
    rcases h.mutation u huPending with ⟨hsrcNone, htgtNone⟩ |
        ⟨ru, w, href, hmut, huOrigin, hleafRead⟩
    · exact Or.inl ⟨hsrcNone, by simpa [MoveState.writeGlobal] using htgtNone⟩
    · exact Or.inr ⟨ru, w, href, by
        simpa [MoveState.writeGlobal] using hmut,
        huOrigin, fun huLeaf => hleafRead (by
          rintro ⟨c, hc, hcne, e, he, heparent, rc, hrc, hmatch⟩
          by_cases hct : c = t
          · subst c
            rw [hsrc] at hrc
            cases hrc
            have hactual := hunique e he hmatch
            rw [heparent] at hactual
            cases hactual
          · apply huLeaf
            exact ⟨c, by simp [hc, hct], hcne, e, he, heparent,
              rc, hrc, hmatch⟩)⟩
  · intro r' a' hglobal
    by_cases hsame : r' = r ∧ a' = a
    · obtain ⟨rfl, rfl⟩ := hsame
      simp [MoveState.writeGlobal, memWrite, hmemory]
    · rw [MoveState.writeGlobal]
      simp only [memWrite, hsame, ↓reduceIte]
      apply h.globals r' a'
      rintro ⟨u, hu, ru, href, hru⟩
      by_cases hut : u = t
      · subst u
        rw [hsrc] at href
        cases href
        simp only [RefRoot.global.injEq] at hru
        exact hsame ⟨hru.1.symm, hru.2.symm⟩
      · apply hglobal
        exact ⟨u, by simp [hu, hut], ru, href, hru⟩

/-- Simulate the emitted mutation constructor for a local borrow.  Relation
preservation is kept separate from this operational splice. -/
theorem CoreFrameRel.simulate_borrowLoc_step {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {s s' : MoveState}
    (h : CoreFrameRel d g pending s s') {dst x : LocalIndex} {v : Value}
    (hx : s.locals x = some v)
    (hrange : x < d.numLocals)
    (hplain : isMutLocal d x = false)
    (hroot : ¬PendingRoot pending s (.loc s.current x))
    (hfree : v.refFree) :
    InstrPath [(.call [dst] (.mkMutLoc x) [x])] s'
      (s'.writeLocal dst (.mut ⟨.loc s'.current x, []⟩ v)) := by
  have hx' : s'.locals x = some v := by
    rw [h.lookup_plain hrange hplain hroot, hx]
  exact InstrPath.one (InstrNext.mkMutLoc hx' hfree)

/-- Simulate the emitted mutation read for a pending leaf reference. -/
theorem CoreFrameRel.simulate_readRef_step {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {s s' : MoveState}
    (h : CoreFrameRel d g pending s s') {dst t : LocalIndex}
    {rt : RefTarget} {v : Value} (htPending : t ∈ pending)
    (hleaf : hasPendingChild g pending t = false)
    (ht : s.locals t = some (.ref rt)) (hv : s.readTarget rt = some v) :
    InstrPath [(.call [dst] .getMut [t])] s' (s'.writeLocal dst v) := by
  obtain ⟨rt', v', ht', hmut, -, hread⟩ :=
    h.leaf htPending hleaf ht
  rw [ht] at ht'
  cases ht'
  rw [hv] at hread
  cases hread
  exact InstrPath.one (InstrNext.getMut hmut)

/-- Simulate the emitted mutation update for a source write through a pending
leaf reference. -/
theorem CoreFrameRel.simulate_writeRef_step {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {s s' : MoveState}
    (h : CoreFrameRel d g pending s s') {t vt : LocalIndex}
    {rt : RefTarget} {v : Value} (htPending : t ∈ pending)
    (hleaf : hasPendingChild g pending t = false)
    (ht : s.locals t = some (.ref rt)) (hv : s.locals vt = some v)
    (hrange : vt < d.numLocals)
    (hplain : isMutLocal d vt = false)
    (hroot : ¬PendingRoot pending s (.loc s.current vt))
    (hfree : v.refFree) :
    InstrPath [(.call [t] .setMut [t, vt])] s'
      (s'.writeLocal t (.mut rt v)) := by
  obtain ⟨rt', old', ht', hmut, -, -⟩ :=
    h.leaf htPending hleaf ht
  rw [ht] at ht'
  cases ht'
  have hv' : s'.locals vt = some v := by
    rw [h.lookup_plain hrange hplain hroot, hv]
  exact InstrPath.one (InstrNext.setMut hmut hv' hfree)

/-! ## Write-back semantics -/

/-- Invert emission of a write-back to a local root. -/
theorem wbBody_localRoot_inv {Δ : StructDecls} {d : FunDecl}
    {t x child : LocalIndex} {path : List BStep} {st st' : EmitSt}
    {body : List Instr}
    (h : wbBody Δ d t ⟨.localRoot x, child, path⟩ st =
      .ok (body, st')) :
    path = [] ∧ st' = st ∧ body = [.call [x] .getMut [t]] := by
  cases path with
  | nil =>
      simp only [wbBody, List.isEmpty_nil, Bool.not_true, Bool.false_eq_true,
        ↓reduceIte, pure, StateT.pure, Except.pure, Except.ok.injEq,
        Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      exact ⟨rfl, rfl, rfl⟩
  | cons step rest =>
      simp [wbBody, StateT.lift, throw, throwThe,
        MonadExceptOf.throw] at h
      change Except.error "internal: a root borrow edge carries a path" =
        Except.ok (body, st') at h
      cases h

/-- The emitted local-root write-back installs the carried mutation value in
that local. -/
theorem wbBody_localRoot_path {Δ : StructDecls} {d : FunDecl}
    {t x child : LocalIndex} {path : List BStep} {emit emit' : EmitSt}
    {body : List Instr} {s : MoveState} {rt : RefTarget} {v : Value}
    (hemits : wbBody Δ d t ⟨.localRoot x, child, path⟩ emit =
      .ok (body, emit'))
    (ht : s.locals t = some (.mut rt v)) :
    InstrPath body s (s.writeLocal x v) := by
  obtain ⟨-, -, rfl⟩ := wbBody_localRoot_inv hemits
  exact InstrPath.writeBackLocal ht

/-- Decompose emission of a write-back to a global root into its type query
and two fresh-local allocations. -/
theorem wbBody_globalRoot_inv {Δ : StructDecls} {d : FunDecl}
    {t child : LocalIndex} {r : ResourceId} {path : List BStep}
    {st st' : EmitSt} {body : List Instr}
    (h : wbBody Δ d t ⟨.globalRoot r, child, path⟩ st =
      .ok (body, st')) :
    ∃ ty ad c st₁,
      path = [] ∧ mutPayloadTy d t = .ok ty ∧
      alloc .address st = .ok (ad, st₁) ∧
      alloc ty st₁ = .ok (c, st') ∧
      body = [.call [ad] .mutAddr [t], .call [c] .getMut [t],
        .call [] (.writeGlobal r) [ad, c]] := by
  cases path with
  | cons step rest =>
      simp [wbBody, StateT.lift, throw, throwThe,
        MonadExceptOf.throw] at h
      change Except.error "internal: a root borrow edge carries a path" =
        Except.ok (body, st') at h
      cases h
  | nil =>
      simp only [wbBody, List.isEmpty_nil, Bool.not_true,
        Bool.false_eq_true, ↓reduceIte] at h
      change ((mutPayloadTy d t).map (·, st)).bind (fun first =>
          (alloc .address first.2).bind (fun second =>
            (alloc first.1 second.2).bind (fun third =>
              .ok
                ([.call [second.1] .mutAddr [t],
                  .call [third.1] .getMut [t],
                  .call [] (.writeGlobal r) [second.1, third.1]],
                  third.2)))) = .ok (body, st') at h
      obtain ⟨first, hty, h⟩ := Except.bind_ok_inv h
      obtain ⟨ty, sty⟩ := first
      obtain ⟨second, had, h⟩ := Except.bind_ok_inv h
      obtain ⟨ad, st₁⟩ := second
      obtain ⟨third, hc, hout⟩ := Except.bind_ok_inv h
      obtain ⟨c, st₂⟩ := third
      simp only [Except.ok.injEq, Prod.mk.injEq] at hout
      obtain ⟨rfl, rfl⟩ := hout
      obtain ⟨hty', hsty⟩ := Except.map_pair_ok_inv hty
      subst sty
      exact ⟨ty, ad, c, st₁, rfl, hty', had, hc, rfl⟩

/-- The emitted global-root write-back stores the carried mutation value at
its target address.  Freshness follows solely from the emitter allocation
bound, so later simulations need not reason about temporary locals. -/
theorem wbBody_globalRoot_path { Δ : StructDecls} {d : FunDecl}
    {t child : LocalIndex} {r : ResourceId} {path : List BStep}
    {emit emit' : EmitSt} {body : List Instr} {s : MoveState}
    {a : Address} {v : Value}
    (hemits : wbBody Δ d t ⟨.globalRoot r, child, path⟩ emit =
      .ok (body, emit'))
    (hbase : d.numLocals ≤ emit.nextLocal) (htlt : t < d.numLocals)
    (ht : s.locals t = some (.mut ⟨.global r a, []⟩ v))
    (hfree : v.refFree) :
    ∃ s', InstrPath body s s' ∧
      (∀ x, x < emit.nextLocal → s'.locals x = s.locals x) ∧
      s'.memory = memWrite s.memory r a v := by
  obtain ⟨ty, ad, c, st₁, rfl, -, had, hc, rfl⟩ :=
    wbBody_globalRoot_inv hemits
  obtain ⟨hadEq, hst₁⟩ := alloc_inv had
  obtain ⟨hcEq, -⟩ := alloc_inv hc
  have hat : ad ≠ t := by
    rw [(alloc_nextLocal had).1]
    exact Ne.symm (Nat.ne_of_lt (Nat.lt_of_lt_of_le htlt hbase))
  have hca : c ≠ ad := by
    rw [hadEq, hcEq, hst₁]
    simp
  refine ⟨_, InstrPath.writeBackGlobal ht hfree hat hca, ?_, by simp⟩
  intro x hx
  have hxad : x ≠ ad := by
    rw [(alloc_nextLocal had).1]
    exact Nat.ne_of_lt hx
  have hxc : x ≠ c := by
    rw [(alloc_nextLocal hc).1]
    exact Nat.ne_of_lt
      (Nat.lt_of_lt_of_le hx (alloc_nextLocal_le had))
  simp [hxad, hxc]

/-- Decompose emission of a direct child-to-parent mutation write-back. -/
theorem wbBody_refDirect_inv { Δ : StructDecls} {d : FunDecl}
    {p t child : LocalIndex} {st st' : EmitSt} {body : List Instr}
    (h : wbBody Δ d t ⟨.refNode p, child, []⟩ st =
      .ok (body, st')) :
    ∃ tp tc c,
      mutPayloadTy d p = .ok tp ∧ mutPayloadTy d t = .ok tc ∧
      alloc tc st = .ok (c, st') ∧
      body = [.call [c] .getMut [t], .call [p] .setMut [p, c]] := by
  simp only [wbBody] at h
  change ((mutPayloadTy d p).map (·, st)).bind (fun first =>
      ((mutPayloadTy d t).map (·, first.2)).bind (fun second =>
        (alloc second.1 second.2).bind (fun third =>
          .ok (([.call [third.1] .getMut [t],
            .call [p] .setMut [p, third.1]]), third.2)))) =
    .ok (body, st') at h
  obtain ⟨first, hp, h⟩ := Except.bind_ok_inv h
  obtain ⟨tp, stp⟩ := first
  obtain ⟨hpTy, hstp⟩ := Except.map_pair_ok_inv hp
  subst stp
  obtain ⟨second, ht, h⟩ := Except.bind_ok_inv h
  obtain ⟨tc, stt⟩ := second
  obtain ⟨htTy, hstt⟩ := Except.map_pair_ok_inv ht
  subst stt
  obtain ⟨third, hc, hout⟩ := Except.bind_ok_inv h
  obtain ⟨c, stc⟩ := third
  simp only [Except.ok.injEq, Prod.mk.injEq] at hout
  obtain ⟨hbody, hstate⟩ := hout
  subst body
  subst stc
  exact ⟨tp, tc, c, hpTy, htTy, hc, rfl⟩

/-- A direct parent write-back replaces the parent mutation payload. -/
theorem wbBody_refDirect_path { Δ : StructDecls} {d : FunDecl}
    {p t child : LocalIndex} {emit emit' : EmitSt} {body : List Instr}
    {s : MoveState} {rp rt : RefTarget} {vp v : Value}
    (hemits : wbBody Δ d t ⟨.refNode p, child, []⟩ emit =
      .ok (body, emit'))
    (hbase : d.numLocals ≤ emit.nextLocal) (hplt : p < d.numLocals)
    (hp : s.locals p = some (.mut rp vp))
    (ht : s.locals t = some (.mut rt v)) (hfree : v.refFree) :
    ∃ s', InstrPath body s s' ∧
      s'.locals p = some (.mut rp v) ∧
      (∀ x, x < emit.nextLocal → x ≠ p →
        s'.locals x = s.locals x) ∧
      s'.memory = s.memory := by
  obtain ⟨tp, tc, c, hpTy, htTy, hc, hbody⟩ :=
    wbBody_refDirect_inv hemits
  subst body
  have hcp : c ≠ p := by
    have hcEq := (alloc_nextLocal hc).1
    intro heq
    rw [heq] at hcEq
    have hpbase : p < emit.nextLocal := Nat.lt_of_lt_of_le hplt hbase
    exact (Nat.ne_of_lt hpbase) hcEq
  have hcFresh := (alloc_nextLocal hc).1
  refine ⟨_, InstrPath.writeBackParent hp ht hfree hcp, by simp, ?_, rfl⟩
  intro x hx hxp
  have hxc : x ≠ c := by
    intro heq
    rw [← heq] at hcFresh
    exact (Nat.ne_of_lt hx) hcFresh
  simp [hxp, hxc]

/-- Proof-facing trace of the recursive functional-update emitter.  It hides
monadic bind plumbing while retaining every allocation and generated
instruction needed by the semantic proof. -/
inductive BuildUpdateTrace (Δ : StructDecls) (p t : LocalIndex) :
    Nat → LocalIndex → Ty → List BStep → EmitSt →
      List Instr → LocalIndex → EmitSt → Prop where
  | nil {k cur : Nat} {curTy : Ty} {st st' : EmitSt} {c : LocalIndex}
      (allocLeaf : alloc curTy st = .ok (c, st')) :
      BuildUpdateTrace Δ p t k cur curTy [] st
        [.call [c] .getMut [t]] c st'
  | field {k cur i sub out' out : Nat} {curTy ft : Ty}
      {rest : List BStep} {st st₁ st₂ st₃ : EmitSt}
      {instrs : List Instr}
      (fieldTyOk : fieldTy Δ curTy i = .ok ft)
      (allocSub : alloc ft st = .ok (sub, st₁))
      (nested : BuildUpdateTrace Δ p t (k + 1) sub ft rest st₁
        instrs out' st₂)
      (allocOut : alloc curTy st₂ = .ok (out, st₃)) :
      BuildUpdateTrace Δ p t k cur curTy (.field i :: rest) st
        (.call [sub] (.getField i) [cur] :: instrs ++
          [.call [out] (.updateField i) [cur, out']]) out st₃
  | index {k cur idx sub out' out : Nat} {curTy et : Ty}
      {rest : List BStep} {st st₁ st₂ st₃ st₄ : EmitSt}
      {instrs : List Instr}
      (elemTyOk : elemTy curTy = .ok et)
      (allocIndex : alloc .u64 st = .ok (idx, st₁))
      (allocSub : alloc et st₁ = .ok (sub, st₂))
      (nested : BuildUpdateTrace Δ p t (k + 1) sub et rest st₂
        instrs out' st₃)
      (allocOut : alloc curTy st₃ = .ok (out, st₄)) :
      BuildUpdateTrace Δ p t k cur curTy (.index :: rest) st
        (.call [idx] (.mutPathIndex k) [p, t] ::
          .call [sub] .vecGet [cur, idx] :: instrs ++
          [.call [out] .vecSet [cur, idx, out']]) out st₄

/-- Extract the compact structural trace from successful functional-update
emission. -/
theorem buildUpdate_trace { Δ : StructDecls} {p t k cur : LocalIndex}
    {curTy : Ty} {path : List BStep} {st st' : EmitSt}
    {body : List Instr} {out : LocalIndex}
    (h : buildUpdate Δ p t k cur curTy path st =
      .ok ((body, out), st')) :
    BuildUpdateTrace Δ p t k cur curTy path st body out st' := by
  induction path generalizing k cur curTy st body out st' with
  | nil =>
      change (alloc curTy st).bind (fun pair =>
          .ok (([.call [pair.1] .getMut [t]], pair.1), pair.2)) =
        .ok ((body, out), st') at h
      obtain ⟨pair, hc, hout⟩ := Except.bind_ok_inv h
      obtain ⟨c, stc⟩ := pair
      simp only [Except.ok.injEq, Prod.mk.injEq] at hout
      obtain ⟨⟨hbody, hout⟩, hstate⟩ := hout
      rw [hstate] at hc
      rw [← hbody, ← hout]
      exact .nil hc
  | cons step rest ih =>
      cases step with
      | field i =>
          change ((fieldTy Δ curTy i).map (·, st)).bind (fun first =>
              (alloc first.1 first.2).bind (fun second =>
                (buildUpdate Δ p t (k + 1) second.1 first.1 rest
                    second.2).bind (fun third =>
                  (alloc curTy third.2).bind (fun fourth =>
                    .ok ((.call [second.1] (.getField i) [cur] ::
                      third.1.1 ++ [.call [fourth.1] (.updateField i)
                        [cur, third.1.2]], fourth.1), fourth.2))))) =
            .ok ((body, out), st') at h
          obtain ⟨first, hftMap, h⟩ := Except.bind_ok_inv h
          obtain ⟨ft, stft⟩ := first
          obtain ⟨hft, hstft⟩ := Except.map_pair_ok_inv hftMap
          subst stft
          obtain ⟨second, hsub, h⟩ := Except.bind_ok_inv h
          obtain ⟨sub, st₁⟩ := second
          obtain ⟨third, hrec, h⟩ := Except.bind_ok_inv h
          obtain ⟨result, st₂⟩ := third
          obtain ⟨instrs, out'⟩ := result
          obtain ⟨fourth, houtAlloc, h⟩ := Except.bind_ok_inv h
          obtain ⟨out₀, st₃⟩ := fourth
          simp only [Except.ok.injEq, Prod.mk.injEq] at h
          obtain ⟨⟨hbody, hout⟩, hstate⟩ := h
          rw [hstate] at houtAlloc
          rw [← hbody, ← hout]
          exact .field hft hsub (ih hrec) houtAlloc
      | index =>
          change ((elemTy curTy).map (·, st)).bind (fun first =>
              (alloc .u64 first.2).bind (fun second =>
                (alloc first.1 second.2).bind (fun third =>
                  (buildUpdate Δ p t (k + 1) third.1 first.1 rest
                      third.2).bind (fun fourth =>
                    (alloc curTy fourth.2).bind (fun fifth =>
                      .ok ((.call [second.1] (.mutPathIndex k) [p, t] ::
                        .call [third.1] .vecGet [cur, second.1] ::
                        fourth.1.1 ++ [.call [fifth.1] .vecSet
                          [cur, second.1, fourth.1.2]], fifth.1),
                        fifth.2)))))) = .ok ((body, out), st') at h
          obtain ⟨first, hetMap, h⟩ := Except.bind_ok_inv h
          obtain ⟨et, stet⟩ := first
          obtain ⟨het, hstet⟩ := Except.map_pair_ok_inv hetMap
          subst stet
          obtain ⟨second, hidx, h⟩ := Except.bind_ok_inv h
          obtain ⟨idx, st₁⟩ := second
          obtain ⟨third, hsub, h⟩ := Except.bind_ok_inv h
          obtain ⟨sub, st₂⟩ := third
          obtain ⟨fourth, hrec, h⟩ := Except.bind_ok_inv h
          obtain ⟨result, st₃⟩ := fourth
          obtain ⟨instrs, out'⟩ := result
          obtain ⟨fifth, houtAlloc, h⟩ := Except.bind_ok_inv h
          obtain ⟨out₀, st₄⟩ := fifth
          simp only [Except.ok.injEq, Prod.mk.injEq] at h
          obtain ⟨⟨hbody, hout⟩, hstate⟩ := h
          rw [hstate] at houtAlloc
          rw [← hbody, ← hout]
          exact .index het hidx hsub (ih hrec) houtAlloc

/-- Functional-update emission advances (or preserves) the fresh-local
frontier. -/
theorem BuildUpdateTrace.frontier_le { Δ : StructDecls} {p t k cur : Nat}
    {curTy : Ty} {path : List BStep} {st st' : EmitSt}
    {body : List Instr} {out : LocalIndex}
    (h : BuildUpdateTrace Δ p t k cur curTy path st body out st') :
    st.nextLocal ≤ st'.nextLocal := by
  induction h with
  | nil hc =>
      exact alloc_nextLocal_le hc
  | field _ hsub _ hout ih =>
      exact Nat.le_trans (alloc_nextLocal_le hsub)
        (Nat.le_trans ih (alloc_nextLocal_le hout))
  | index _ hidx hsub _ hout ih =>
      exact Nat.le_trans (alloc_nextLocal_le hidx)
        (Nat.le_trans (alloc_nextLocal_le hsub)
          (Nat.le_trans ih (alloc_nextLocal_le hout)))

/-- Functional-update generation allocates temporaries without moving the
active emitter cursor. -/
theorem BuildUpdateTrace.cursor_eq { Δ : StructDecls} {p t k cur : Nat}
    {curTy : Ty} {path : List BStep} {st st' : EmitSt}
    {body : List Instr} {out : LocalIndex}
    (h : BuildUpdateTrace Δ p t k cur curTy path st body out st') :
    st'.curId = st.curId ∧ st'.cur = st.cur := by
  induction h with
  | nil hc => exact alloc_cursor hc
  | field _ hsub _ hout ih =>
      exact ⟨(alloc_cursor hout).1.trans
          (ih.1.trans (alloc_cursor hsub).1),
        (alloc_cursor hout).2.trans
          (ih.2.trans (alloc_cursor hsub).2)⟩
  | index _ hidx hsub _ hout ih =>
      exact ⟨(alloc_cursor hout).1.trans
          (ih.1.trans ((alloc_cursor hsub).1.trans (alloc_cursor hidx).1)),
        (alloc_cursor hout).2.trans
          (ih.2.trans ((alloc_cursor hsub).2.trans (alloc_cursor hidx).2))⟩

/-- Concrete, reference-free functional update along a static borrow path.
Field steps carry their fixed offset; vector steps carry the dynamic index
recovered from the child mutation target. -/
inductive PathUpdate : List BStep → List Nat → Value → Value → Value → Prop where
  | nil {old new : Value} (hfree : new.refFree) :
      PathUpdate [] [] old new new
  | field {i : Nat} {rest : List BStep} {ns : List Nat}
      {fs : List Value} {old new sub : Value}
      (hfree : Value.refFreeList fs) (hfield : fs[i]? = some old)
      (nested : PathUpdate rest ns old new sub) :
      PathUpdate (.field i :: rest) (i :: ns) (.struct fs) new
        (.struct (fs.set i sub))
  | index {i : Nat} {rest : List BStep} {ns : List Nat}
      {es : List Value} {old new sub : Value}
      (hfree : Value.refFreeList es) (helem : es[i]? = some old)
      (hbound : i < U64_SIZE) (nested : PathUpdate rest ns old new sub) :
      PathUpdate (.index :: rest) (i :: ns) (.vector es) new
        (.vector (es.set i sub))

/-- The result produced by a concrete path update is reference-free. -/
theorem PathUpdate.result_free {path : List BStep} {ns : List Nat}
    {old new updated : Value} (h : PathUpdate path ns old new updated) :
    updated.refFree := by
  induction h with
  | nil hfree => exact hfree
  | field hfree _ _ ih => exact Value.refFreeList_set hfree ih
  | index hfree _ _ _ ih => exact Value.refFreeList_set hfree ih

/-- An empty static update path replaces the complete old value. -/
theorem PathUpdate.nil_eq {ns : List Nat} {old new updated : Value}
    (h : PathUpdate [] ns old new updated) : ns = [] ∧ updated = new := by
  cases h
  exact ⟨rfl, rfl⟩

/-! ## Core stack invariant -/

/-- Every dynamically matching pending child can be functionally written
back into its pending parent mutation. -/
def CoreWriteReady (g : BGraph) (pending : List LocalIndex)
    (s : MoveState) : Prop :=
  ∀ t, t ∈ pending → ∀ e, e ∈ inEdges g t →
    ∀ p, e.parent = .refNode p → p ∈ pending →
    ∀ rp vp rt vt,
      s.locals p = some (.mut rp vp) →
      s.locals t = some (.mut rt vt) →
      isParentTarget (bPathPattern e.path) rp rt = true →
      ∃ updated,
        PathUpdate e.path (rt.path.drop rp.path.length) vp vt updated

/-- Writing outside the pending mutation set preserves all parent update
certificates. -/
theorem CoreWriteReady.writeLocal_of_not_mem {g : BGraph}
    {pending : List LocalIndex} {s : MoveState} {x : LocalIndex}
    {v : Value} (h : CoreWriteReady g pending s) (hx : x ∉ pending) :
    CoreWriteReady g pending (s.writeLocal x v) := by
  intro t ht e he p hparent hp rp vp rt vt hpMut htMut hmatches
  have htx : t ≠ x := ne_of_mem_of_not_mem ht hx
  have hpx : p ≠ x := ne_of_mem_of_not_mem hp hx
  apply h t ht e he p hparent hp rp vp rt vt
  · simpa [MoveState.writeLocal_locals, hpx] using hpMut
  · simpa [MoveState.writeLocal_locals, htx] using htMut
  · exact hmatches

/-- Restrict write readiness to a smaller pending set. -/
theorem CoreWriteReady.mono {g : BGraph}
    {small large : List LocalIndex} {s : MoveState}
    (h : CoreWriteReady g large s)
    (hsub : ∀ x, x ∈ small → x ∈ large) :
    CoreWriteReady g small s := by
  intro t ht e he p hparent hp
  exact h t (hsub t ht) e he p hparent (hsub p hp)

/-- Write readiness is unchanged by target-state updates outside the pending
local range. -/
theorem CoreWriteReady.target_congr {g : BGraph}
    {pending : List LocalIndex} {s s' : MoveState}
    (h : CoreWriteReady g pending s)
    (hbound : ∀ x, x ∈ pending → x < n)
    (hlocals : ∀ x, x < n → s'.locals x = s.locals x) :
    CoreWriteReady g pending s' := by
  intro t ht e he p hparent hp rp vp rt vt hpMut htMut hmatch
  apply h t ht e he p hparent hp rp vp rt vt
  · rw [← hlocals p (hbound p hp)]
    exact hpMut
  · rw [← hlocals t (hbound t ht)]
    exact htMut
  · exact hmatch

/-- Borrow-correctness facts consumed when one pending leaf dies.  Matching
may-origins are unique; updating a reference parent exposes its current
source value when the parent becomes a leaf and preserves the update
certificates for disjoint siblings. -/
structure CoreDeathSafe (g : BGraph) (pending : List LocalIndex)
    (s s' : MoveState) (t : LocalIndex) : Prop where
  child_ref : ∃ rt, s.locals t = some (.ref rt)
  origin : ∀ rt, s.locals t = some (.ref rt) →
    ∃ e ∈ inEdges g t, CoreEdgeMatches s rt e
  candidates_nodup : (inEdges g t).Nodup
  origin_unique : CoreOriginUnique g s t
  parent_pending : ∀ e ∈ inEdges g t, ∀ p,
    e.parent = .refNode p → p ∈ pending
  parent_ne : ∀ e ∈ inEdges g t, ∀ p,
    e.parent = .refNode p → p ≠ t
  local_plain : ∀ e ∈ inEdges g t, ∀ x,
    e.parent = .localRoot x → isMutLocal d x = false
  parent_current : ∀ e ∈ inEdges g t, ∀ p,
    e.parent = .refNode p → p ∈ pending → ∀ rp vp rt vt updated,
    s.locals p = some (.ref rp) →
    s.locals t = some (.ref rt) →
    s'.locals p = some (.mut rp vp) →
    s'.locals t = some (.mut rt vt) →
    CoreEdgeMatches s rt e →
    PathUpdate e.path (rt.path.drop rp.path.length) vp vt updated →
    ¬CoreHasPendingChild g (pending.filter (· ≠ t)) s p →
    s.readTarget rp = some updated
  parent_ready : ∀ e ∈ inEdges g t, ∀ p,
    e.parent = .refNode p → p ∈ pending → ∀ rp vp rt vt updated,
    s.locals p = some (.ref rp) →
    s.locals t = some (.ref rt) →
    s'.locals p = some (.mut rp vp) →
    s'.locals t = some (.mut rt vt) →
    CoreEdgeMatches s rt e →
    PathUpdate e.path (rt.path.drop rp.path.length) vp vt updated →
    CoreWriteReady g (pending.filter (· ≠ t))
      (s'.writeLocal p (.mut rp updated))

/-- Changing only emitter-temporary locals preserves a death certificate. -/
theorem CoreDeathSafe.target_congr {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {source target target' : MoveState}
    {t : LocalIndex}
    (h : CoreDeathSafe g pending source target t)
    (hbound : ∀ x, x ∈ pending → x < d.numLocals)
    (ht : t ∈ pending)
    (hlocals : MoveState.LocalsEqBelow d.numLocals target target') :
    CoreDeathSafe g pending source target' t := by
  refine ⟨h.child_ref, h.origin, h.candidates_nodup, h.origin_unique,
    h.parent_pending, h.parent_ne, h.local_plain, ?_, ?_⟩
  · intro e he p hpParent hp rp vp rt vt updated hsrcP hsrcT
      htgtP htgtT hmatch hupdate hleaf
    apply h.parent_current e he p hpParent hp rp vp rt vt updated
      hsrcP hsrcT
    · rw [← hlocals p (hbound p hp)]
      exact htgtP
    · rw [← hlocals t (hbound t ht)]
      exact htgtT
    · exact hmatch
    · exact hupdate
    · exact hleaf
  · intro e he p hpParent hp rp vp rt vt updated hsrcP hsrcT
      htgtP htgtT hmatch hupdate
    have hready := h.parent_ready e he p hpParent hp rp vp rt vt updated
      hsrcP hsrcT (by
        rw [← hlocals p (hbound p hp)]
        exact htgtP) (by
        rw [← hlocals t (hbound t ht)]
        exact htgtT) hmatch hupdate
    apply hready.target_congr (n := d.numLocals)
    · intro x hx
      exact hbound x (List.mem_filter.mp hx).1
    · intro x hx
      by_cases hxp : x = p
      · subst x
        simp
      · simpa [MoveState.writeLocal_locals, hxp] using hlocals x hx

/-! ## Emitter-cursor execution -/

/-- Execution from the suffix which follows an emitter's accumulated current
prefix in the final CFG.  This is the continuation interface shared by
instruction rewrites, write-back diamonds, and terminator emission. -/
def RunAfterPrefix (P : Program) (G : Cfg) (st : EmitSt)
    (s : MoveState) (o : FrameOutcome) : Prop :=
  ∃ rest term, G.blocks st.curId = some ⟨st.cur ++ rest, term⟩ ∧
    RunFrom P G rest term s o

/-- A final CFG contains every block recorded by an emitter state. -/
def EmitDoneIn (G : Cfg) (st : EmitSt) : Prop :=
  ∀ b blk, (b, blk) ∈ st.done → G.blocks b = some blk

/-- The active emitter prefix occurs at its cursor in the final CFG.  Unlike
`RunAfterPrefix`, this is purely structural and makes no claim that the
remaining code is executable from a particular state. -/
def EmitCursorIn (G : Cfg) (st : EmitSt) : Prop :=
  ∃ rest term, G.blocks st.curId = some ⟨st.cur ++ rest, term⟩

/-- Restrict final-CFG containment to an earlier finished-block list. -/
theorem EmitDoneIn.of_subset {G : Cfg} {later earlier : EmitSt}
    (h : EmitDoneIn G later)
    (hsub : ∀ p ∈ earlier.done, p ∈ later.done) : EmitDoneIn G earlier := by
  intro b blk hb
  exact h b blk (hsub (b, blk) hb)

/-- Transfer cursor containment across emitter states with the same cursor. -/
theorem EmitCursorIn.of_cursor_eq {G : Cfg} {before after : EmitSt}
    (hid : after.curId = before.curId) (hcur : after.cur = before.cur)
    (h : EmitCursorIn G after) : EmitCursorIn G before := by
  obtain ⟨rest, term, hblock⟩ := h
  exact ⟨rest, term, by simpa [hid, hcur] using hblock⟩

/-- An executable cursor continuation contains the corresponding structural
cursor fact. -/
theorem RunAfterPrefix.cursorIn {P : Program} {G : Cfg} {st : EmitSt}
    {s : MoveState} {o : FrameOutcome}
    (h : RunAfterPrefix P G st s o) : EmitCursorIn G st := by
  obtain ⟨rest, term, hblock, -⟩ := h
  exact ⟨rest, term, hblock⟩

/-- Recover an earlier emitter cursor after a straight-line append. -/
theorem EmitCursorIn.prepend {G : Cfg} {before after : EmitSt}
    {code : List Instr}
    (hid : after.curId = before.curId)
    (hcur : after.cur = before.cur ++ code)
    (h : EmitCursorIn G after) : EmitCursorIn G before := by
  obtain ⟨rest, term, hblock⟩ := h
  refine ⟨code ++ rest, term, ?_⟩
  calc
    G.blocks before.curId = G.blocks after.curId := congrArg G.blocks hid.symm
    _ = some ⟨after.cur ++ rest, term⟩ := hblock
    _ = some ⟨before.cur ++ (code ++ rest), term⟩ := by
      rw [hcur, List.append_assoc]

/-- The cursor closed by a successful `closeBlock` occurs in any final CFG
which contains the resulting finished-block list. -/
theorem EmitCursorIn.of_close {G : Cfg} {st st' : EmitSt}
    {term : Term} {contId : BlockId}
    (hclose : closeBlock term contId st = .ok ((), st'))
    (hcfg : EmitDoneIn G st') : EmitCursorIn G st := by
  have hstate := closeBlock_inv hclose
  have hblock : G.blocks st.curId = some ⟨st.cur, term⟩ := by
    apply hcfg
    rw [hstate]
    simp
  exact ⟨[], term, by simpa using hblock⟩

/-- Local semantic certificate for one continuing core rewrite.  It exposes
only the straight-line code appended by `rewriteInstr`, its execution, and
the frame invariants at the resulting source/target states; death processing
and all CFG control flow remain outside this certificate. -/
structure CoreInstrNextSafe (d : FunDecl) (g : BGraph)
    (pending : List LocalIndex) (source target : MoveState)
    (before after : EmitSt) (targetNext : MoveState)
    (code : List Instr) : Prop where
  cursorId : after.curId = before.curId
  cursor : after.cur = before.cur ++ code
  frontier : before.nextLocal ≤ after.nextLocal
  path : InstrPath code target targetNext
  frame : CoreFrameRel d g pending source targetNext
  ready : CoreWriteReady g pending targetNext

/-- Local semantic certificate for one stopping core rewrite.  The generated
straight-line path determines the target outcome before any death or
terminator phase can run. -/
structure CoreInstrStopSafe (before after : EmitSt)
    (sourceOutcome targetOutcome : FrameOutcome) (target : MoveState)
    (code : List Instr) : Prop where
  cursorId : after.curId = before.curId
  cursor : after.cur = before.cur ++ code
  path : InstrStopPath code target targetOutcome
  outcome : AgreeOutcome sourceOutcome targetOutcome

/-- A certified stopping rewrite aborts the containing target block at the
same emitter prefix, irrespective of its unreachable suffix. -/
theorem CoreInstrStopSafe.run {P' : Program} {G : Cfg}
    {before after : EmitSt} {sourceOutcome targetOutcome : FrameOutcome}
    {target : MoveState} {code : List Instr}
    (h : CoreInstrStopSafe before after sourceOutcome targetOutcome
      target code)
    (hcursor : EmitCursorIn G after) :
    RunAfterPrefix P' G before target targetOutcome := by
  obtain ⟨rest, term, hblock⟩ := hcursor
  refine ⟨code ++ rest, term, ?_, ?_⟩
  · calc
      G.blocks before.curId = G.blocks after.curId :=
        congrArg G.blocks h.cursorId.symm
      _ = some ⟨after.cur ++ rest, term⟩ := hblock
      _ = some ⟨before.cur ++ (code ++ rest), term⟩ := by
        rw [h.cursor, List.append_assoc]
  · simpa [List.append_assoc] using
      (h.path.run (P := P') (G := G) (rest := rest) (term := term))

/-- Simulation goal at an arbitrary suffix of one core-rewritten source
block.  The instruction trace identifies the exact emitter cursor belonging
to the suffix; the final-CFG premise supplies all generated split blocks. -/
def CoreSimAt (P P' : Program) (G : Cfg) (is : List Instr) (term : Term)
    (source : MoveState) (sourceOutcome : FrameOutcome) : Prop :=
  ∀ {f : FunId} {d d' : FunDecl} {b : BlockId} {blk : Block}
    {points : List (Instr × LiveSet)} {g gEnd : BGraph}
    {pending pendingEnd : List LocalIndex} {st stInstr stEnd : EmitSt}
    {target : MoveState},
    CoreProgram P P' → P.funs f = some d → P'.funs f = some d' →
    ElimCoreInv noSummaries P.structs d d' →
    G = d.body → d.body.blocks b = some blk → term = blk.term →
    points.map Prod.fst = is →
    CoreInstrTrace noSummaries P.structs d b points g pending st
      gEnd pendingEnd stInstr →
    finishCoreBlock P.structs d (liveAnalysis d) b gEnd pendingEnd
      blk.term stInstr = .ok ((), stEnd) →
    EmitDoneIn d'.body stEnd → d.numLocals ≤ st.nextLocal →
    CheckedState P d source →
    CoreFrameRel d g pending source target →
    CoreWriteReady g pending target →
    ∃ targetOutcome,
      RunAfterPrefix P' d'.body st target targetOutcome ∧
      AgreeOutcome sourceOutcome targetOutcome

/-- Frontend certificate consumed by mutation-value correctness.  Its local
fields expose initialization, origin, instruction, and grouped control-flow
facts.  Whole-block simulation is derived from these cases below. -/
structure CoreCheckedFacts (P : Program) : Prop where
  /-- Establish the entry mutation relation for reference-free arguments. -/
  initial {d : FunDecl} {args : List Value} {m : Memory}
      {g : BGraph} {pending : List LocalIndex} :
    CheckedState P d (MoveState.initial args m) →
    (∀ v ∈ args, v.refFree) →
    g = (borrowAnalysis noSummaries d).getD d.body.entry [] →
    pending = coreEntryPending d (liveAnalysis d) d.body.entry g →
      CoreFrameRel d g pending (MoveState.initial args m)
        (MoveState.initial args m) ∧
      CoreWriteReady g pending (MoveState.initial args m)
  /-- Supply the unique coherent origin of one selected dying mutation. -/
  death {d : FunDecl} {g : BGraph} {pending : List LocalIndex}
      {s s' : MoveState} {t : LocalIndex} :
    CheckedState P d s → CoreFrameRel d g pending s s' →
    CoreWriteReady g pending s' → t ∈ pending →
    hasPendingChild g pending t = false →
    CoreDeathSafe g pending s s' t
  /-- Execute one complete death cascade against an outcome-indexed
  continuation.  This is the local closure of the already proved write-back
  trace lemmas and does not quantify over a source block execution. -/
  deaths {P' : Program} {G : Cfg} {d : FunDecl} {src : BlockId}
      {g : BGraph} {liveNow : LiveSet} {fuel : Nat}
      {pending pendingEnd : List LocalIndex} {st stEnd : EmitSt}
      {source target : MoveState} {sourceOutcome : FrameOutcome} :
    CoreDeathTrace P.structs d src g liveNow fuel pending st pendingEnd stEnd →
    EmitDoneIn G stEnd → CheckedState P d source →
    CoreFrameRel d g pending source target → CoreWriteReady g pending target →
    d.numLocals ≤ st.nextLocal →
    (∀ target', CoreFrameRel d g pendingEnd source target' →
      CoreWriteReady g pendingEnd target' →
      ∃ targetOutcome,
        RunAfterPrefix P' G stEnd target' targetOutcome ∧
        AgreeOutcome sourceOutcome targetOutcome) →
    ∃ targetOutcome,
      RunAfterPrefix P' G st target targetOutcome ∧
      AgreeOutcome sourceOutcome targetOutcome
  /-- Certify the target splice for one continuing ordinary instruction. -/
  instrNext {d : FunDecl} {g g' : BGraph}
      {pending pending' : List LocalIndex} {source sourceNext target : MoveState}
      {i : Instr} {before after : EmitSt} :
    CheckedState P d source → CheckedState P d sourceNext →
    CoreFrameRel d g pending source target →
    CoreWriteReady g pending target →
    InstrNext i source sourceNext →
    rewriteInstr noSummaries d g pending i before =
      .ok ((g', pending'), after) →
    ∃ targetNext code,
      CoreInstrNextSafe d g' pending' sourceNext target before after
        targetNext code
  /-- Certify the target splice for one stopping ordinary instruction. -/
  instrStop {d : FunDecl} {g g' : BGraph}
      {pending pending' : List LocalIndex} {source target : MoveState}
      {i : Instr} {sourceOutcome : FrameOutcome} {before after : EmitSt} :
    CheckedState P d source →
    CoreFrameRel d g pending source target →
    CoreWriteReady g pending target →
    InstrStop i source sourceOutcome →
    rewriteInstr noSummaries d g pending i before =
      .ok ((g', pending'), after) →
    ∃ targetOutcome code,
      CoreInstrStopSafe before after sourceOutcome targetOutcome target code
  /-- Structural block facts: every emitted block is present verbatim in the
  final dense target CFG, and fresh locals start beyond source locals. -/
  emitted {d d' : FunDecl} {b : BlockId} {blk : Block}
      {g : BGraph} {before after : EmitSt} :
    ElimCoreInv noSummaries P.structs d d' →
    CoreBlockRewriteInv noSummaries P.structs d (liveAnalysis d) b blk
      g before after →
    EmitDoneIn d'.body after ∧
      d.numLocals ≤ (prepareCoreBlock d (liveAnalysis d) b g before).nextLocal
  /-- Local grouped case for a normally returning concrete call. -/
  callOk {P' : Program} :
    RunFrom.CallOkCase P (CoreSimAt P P')
  /-- Local grouped case for an aborting concrete call. -/
  callAbort {P' : Program} :
    RunFrom.CallAbortCase P (CoreSimAt P P')
  /-- Local grouped case for following a source CFG edge. -/
  termNext {P' : Program} :
    RunFrom.TermNextCase P (CoreSimAt P P')
  /-- Local grouped case for a returning or aborting terminator. -/
  termStop {P' : Program} :
    RunFrom.TermStopCase P (CoreSimAt P P')

/-- Installing a pending leaf into its reference parent and removing the
leaf preserves the frame relation.  Payload coherence and sibling
disjointness are exactly the two projections supplied by `CoreDeathSafe`. -/
theorem CoreFrameRel.writeBackParent {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {s s' : MoveState}
    (h : CoreFrameRel d g pending s s') {t p : LocalIndex} {e : BEdge}
    {rp rt : RefTarget} {vp vt updated : Value}
    (safe : CoreDeathSafe g pending s s' t)
    (hp : p ∈ pending) (hpt : p ≠ t)
    (he : e ∈ inEdges g t) (heParent : e.parent = .refNode p)
    (hsrcP : s.locals p = some (.ref rp))
    (hsrcT : s.locals t = some (.ref rt))
    (htgtP : s'.locals p = some (.mut rp vp))
    (htgtT : s'.locals t = some (.mut rt vt))
    (hmatch : CoreEdgeMatches s rt e)
    (hupdate : PathUpdate e.path (rt.path.drop rp.path.length)
      vp vt updated) :
    CoreFrameRel d g (pending.filter (· ≠ t)) s
      (s'.writeLocal p (.mut rp updated)) := by
  have hpFiltered : p ∈ pending.filter (· ≠ t) := by
    simp [hp, hpt]
  have hroots : rp.root = rt.root := by
    rw [CoreEdgeMatches, heParent] at hmatch
    obtain ⟨parent, hparent, hisParent⟩ := hmatch
    rw [hsrcP] at hparent
    cases hparent
    exact (isParentTarget_parts hisParent).1
  refine ⟨by simp [h.current_eq], ?_, ?_, ?_, ?_, ?_⟩
  · intro u hu
    exact h.pending_bound u (List.mem_filter.mp hu).1
  · intro u hu
    exact h.pending_mut u (List.mem_filter.mp hu).1
  · intro x hxrange hxplain hxroot
    have hxp : x ≠ p := by
      intro hxp
      subst x
      have := h.pending_mut p hp
      rw [hxplain] at this
      contradiction
    have holdRoot : ¬PendingRoot pending s (.loc s.current x) := by
      rintro ⟨u, hu, ru, href, hru⟩
      by_cases hut : u = t
      · subst u
        rw [hsrcT] at href
        cases href
        apply hxroot
        exact ⟨p, hpFiltered, rp, hsrcP, hroots.trans hru⟩
      · apply hxroot
        exact ⟨u, by simp [hu, hut], ru, href, hru⟩
    simpa [MoveState.writeLocal_locals, hxp] using
      h.plain x hxrange hxplain holdRoot
  · intro u hu
    have huPending := (List.mem_filter.mp hu).1
    have hut : u ≠ t := by simpa using (List.mem_filter.mp hu).2
    by_cases hup : u = p
    · subst u
      rcases h.mutation p hp with ⟨hsrcNone, -⟩ |
          ⟨rp', old, href, hmut, horigin, -⟩
      · rw [hsrcP] at hsrcNone
        simp at hsrcNone
      · rw [hsrcP] at href
        cases href
        rw [htgtP] at hmut
        cases hmut
        exact Or.inr ⟨rp, updated, hsrcP, by simp,
          horigin, safe.parent_current e he p heParent hp rp vp rt vt
            updated hsrcP hsrcT htgtP htgtT hmatch hupdate⟩
    · have hup' : u ≠ p := hup
      rcases h.mutation u huPending with ⟨hsrcNone, htgtNone⟩ |
          ⟨ru, w, href, hmut, horigin, hleafRead⟩
      · exact Or.inl ⟨hsrcNone, by
          simpa [MoveState.writeLocal_locals, hup'] using htgtNone⟩
      · exact Or.inr ⟨ru, w, href, by
          simpa [MoveState.writeLocal_locals, hup'] using hmut,
          horigin, fun huLeaf => hleafRead (by
            rintro ⟨c, hc, hcne, ec, hec, hecParent, rc, hrc, hcmatch⟩
            by_cases hct : c = t
            · subst c
              rw [hsrcT] at hrc
              cases hrc
              have hparentEq := safe.origin_unique e he ec hec rt
                hmatch hcmatch
              subst ec
              rw [heParent] at hecParent
              cases hecParent
              exact hup rfl
            · apply huLeaf
              exact ⟨c, by simp [hc, hct], hcne, ec, hec, hecParent,
                rc, hrc, hcmatch⟩)⟩
  · intro r a hglobal
    apply h.globals r a
    rintro ⟨u, hu, ru, href, hru⟩
    by_cases hut : u = t
    · subst u
      rw [hsrcT] at href
      cases href
      apply hglobal
      exact ⟨p, hpFiltered, rp, hsrcP, hroots.trans hru⟩
    · apply hglobal
      exact ⟨u, by simp [hu, hut], ru, href, hru⟩

/-- Static core-simulation information retained for one active or suspended
frame. -/
structure CorePoint where
  frame : FrameId
  decl : FunDecl
  graph : BGraph
  pending : List LocalIndex

/-- Apply the core frame and write-readiness relations to one selected frame
of the whole machine state. -/
structure CorePoint.Rel (p : CorePoint) (s s' : MoveState) : Prop where
  frame : CoreFrameRel p.decl p.graph p.pending
    (s.focusFrame p.frame) (s'.focusFrame p.frame)
  ready : CoreWriteReady p.graph p.pending (s'.focusFrame p.frame)

/-- Stack-indexed core relation.  Tracked frames may differ exactly as
described by their mutation relations; every other frame agrees.  Memory is
governed by each tracked frame's root relation, rather than unconditional
equality, because a checked-out global mutation delays its write-back. -/
structure CoreStackRel (points : List CorePoint)
    (s s' : MoveState) : Prop where
  current_eq : s'.current = s.current
  tracked : ∀ p ∈ points, p.Rel s s'
  untracked : ∀ frame, (∀ p ∈ points, p.frame ≠ frame) →
    s'.frames frame = s.frames frame

/-- The active core point relation is the head of a core stack relation. -/
theorem CoreStackRel.head {p : CorePoint} {points : List CorePoint}
    {s s' : MoveState} (h : CoreStackRel (p :: points) s s') :
    p.Rel s s' :=
  h.tracked p (by simp)

/-- Initially identical states satisfy a singleton core stack relation when
all may-pending entries are uninitialized. -/
theorem CoreStackRel.initial (d : FunDecl) (g : BGraph)
    (pending : List LocalIndex) (args : List Value) (m : Memory)
    (hbound : ∀ t ∈ pending, t < d.numLocals)
    (hmut : ∀ t ∈ pending, isMutLocal d t = true)
    (huninit : ∀ t ∈ pending, args.length ≤ t) :
    CoreStackRel [⟨0, d, g, pending⟩]
      (MoveState.initial args m) (MoveState.initial args m) := by
  have hframe := CoreFrameRel.initial d g pending args m
    hbound hmut huninit
  refine ⟨rfl, ?_, ?_⟩
  · intro p hp
    simp only [List.mem_singleton] at hp
    subst p
    refine ⟨?_, ?_⟩
    · change CoreFrameRel d g pending (MoveState.initial args m)
        (MoveState.initial args m)
      exact hframe
    intro t ht e he p _ hp rp vp rt vt _ htMut _
    have htMut' : (MoveState.initial args m).locals t =
        some (.mut rt vt) := by
      simpa [MoveState.focusFrame, MoveState.initial] using htMut
    rcases hframe.mutation t ht with habsent | hpresent
    · rw [htMut'] at habsent
      cases habsent.2
    · obtain ⟨rt', v, href, hmut', -⟩ := hpresent
      rw [htMut'] at hmut'
      cases hmut'
      change (MoveState.initial args m).locals t = some (.ref rt) at href
      rw [htMut'] at href
      cases href
  · intro frame _
    rfl

/-- Execute the code described by a functional-update trace.  All generated
locals are above the supplied frontier; the result local contains the
updated value, and pre-existing locals and memory are preserved. -/
theorem BuildUpdateTrace.run { Δ : StructDecls} {p t : LocalIndex}
    {k cur : Nat} {curTy : Ty} {path : List BStep} {emit emit' : EmitSt}
    {body : List Instr} {out : LocalIndex}
    (trace : BuildUpdateTrace Δ p t k cur curTy path emit body out emit') :
    ∀ {s : MoveState} {rp rt : RefTarget} {vp vt curVal updated : Value}
      {pre ns : List Nat},
      p < emit.nextLocal → t < emit.nextLocal → cur < emit.nextLocal →
      s.locals p = some (.mut rp vp) →
      s.locals t = some (.mut rt vt) → s.locals cur = some curVal →
      k = pre.length → rt.path = rp.path ++ pre ++ ns →
      PathUpdate path ns curVal vt updated →
      ∃ s', InstrPath body s s' ∧
        s'.locals out = some updated ∧
        MoveState.LocalsEqBelow emit.nextLocal s s' ∧
        s'.memory = s.memory := by
  induction trace with
  | @nil k cur curTy st st' c hc =>
      intro s rp rt vp vt curVal updated pre ns hp ht hcur hpv htv
        hcurv hk hpath hupdate
      cases hupdate with
      | nil hfree =>
          obtain ⟨hcEq, -⟩ := alloc_nextLocal hc
          let s' := s.writeLocal c vt
          refine ⟨s', InstrPath.one (InstrNext.getMut htv), ?_, ?_, rfl⟩
          · simp [s']
          · exact MoveState.LocalsEqBelow.writeLocal (by simp [hcEq])
  | @field k cur i sub out' out curTy ft rest st st₁ st₂ st₃ instrs
      hfieldTy hsub nested hout ih =>
      intro s rp rt vp vt curVal updated pre ns hp ht hcur hpv htv
        hcurv hk hpath hupdate
      cases hupdate with
      | @field _ _ ns fs old _ subVal hfree hfield hrest =>
          obtain ⟨hsubEq, hsubNext⟩ := alloc_nextLocal hsub
          let s₁ := s.writeLocal sub old
          have hpSub : p ≠ sub := by rw [hsubEq]; exact Nat.ne_of_lt hp
          have htSub : t ≠ sub := by rw [hsubEq]; exact Nat.ne_of_lt ht
          have hcurSub : cur ≠ sub := by
            rw [hsubEq]
            exact Nat.ne_of_lt hcur
          have hp₁ : s₁.locals p = some (.mut rp vp) := by
            simpa [s₁, hpSub] using hpv
          have ht₁ : s₁.locals t = some (.mut rt vt) := by
            simpa [s₁, htSub] using htv
          have hsub₁ : s₁.locals sub = some old := by simp [s₁]
          have hpNext : p < st₁.nextLocal :=
            Nat.lt_of_lt_of_le hp (alloc_nextLocal_le hsub)
          have htNext : t < st₁.nextLocal :=
            Nat.lt_of_lt_of_le ht (alloc_nextLocal_le hsub)
          have hsubLt : sub < st₁.nextLocal := by
            rw [hsubEq, hsubNext]
            exact Nat.lt_succ_self _
          have hk' : k + 1 = (pre ++ [i]).length := by simp [hk]
          have hpath' : rt.path = rp.path ++ (pre ++ [i]) ++ ns := by
            simpa [List.append_assoc] using hpath
          obtain ⟨s₂, hrun, houtVal, hbelow, hmem⟩ :=
            ih hpNext htNext hsubLt hp₁ ht₁ hsub₁ hk' hpath' hrest
          have hcur₂ : s₂.locals cur = some (.struct fs) := by
            rw [hbelow cur (Nat.lt_of_lt_of_le hcur (alloc_nextLocal_le hsub))]
            simpa [s₁, hcurSub] using hcurv
          have hbound : i < fs.length := (List.getElem?_eq_some_iff.mp hfield).1
          let s₃ := s₂.writeLocal out (.struct (fs.set i subVal))
          have hlast : InstrPath
              [.call [out] (.updateField i) [cur, out']] s₂ s₃ :=
            InstrPath.one (InstrNext.updateField hcur₂ houtVal
              hrest.result_free hbound)
          have houtFresh : st.nextLocal ≤ out := by
            rw [(alloc_nextLocal hout).1]
            exact Nat.le_trans (alloc_nextLocal_le hsub) nested.frontier_le
          refine ⟨s₃, ?_, by simp [s₃], ?_, by simp [s₃, hmem, s₁]⟩
          · exact (InstrPath.one (InstrNext.getField hcurv hfield)).append
              (hrun.append hlast)
          · exact (MoveState.LocalsEqBelow.writeLocal
                (by simp [hsubEq])).trans
              ((hbelow.mono (alloc_nextLocal_le hsub)).trans
                (MoveState.LocalsEqBelow.writeLocal houtFresh))
  | @index k cur idx sub out' out curTy et rest st st₁ st₂ st₃ st₄
      instrs helemTy hidx hsub nested hout ih =>
      intro s rp rt vp vt curVal updated pre ns hp ht hcur hpv htv
        hcurv hk hpath hupdate
      cases hupdate with
      | @index i _ ns es old _ subVal hfree helem hibound hrest =>
          obtain ⟨hidxEq, hidxNext⟩ := alloc_nextLocal hidx
          obtain ⟨hsubEq, hsubNext⟩ := alloc_nextLocal hsub
          have htargetIndex : rt.path[rp.path.length + k]? = some i := by
            rw [hpath, hk]
            simp
          let s₁ := s.writeLocal idx (.u64 i)
          have hpIdx : p ≠ idx := by
            rw [hidxEq]
            exact Nat.ne_of_lt hp
          have htIdx : t ≠ idx := by
            rw [hidxEq]
            exact Nat.ne_of_lt ht
          have hcurIdx : cur ≠ idx := by
            rw [hidxEq]
            exact Nat.ne_of_lt hcur
          have hp₁ : s₁.locals p = some (.mut rp vp) := by
            simpa [s₁, hpIdx] using hpv
          have ht₁ : s₁.locals t = some (.mut rt vt) := by
            simpa [s₁, htIdx] using htv
          have hcur₁ : s₁.locals cur = some (.vector es) := by
            simpa [s₁, hcurIdx] using hcurv
          have hidx₁ : s₁.locals idx = some (.u64 i) := by simp [s₁]
          let s₂ := s₁.writeLocal sub old
          have hpSt₁ : p < st₁.nextLocal :=
            Nat.lt_of_lt_of_le hp (alloc_nextLocal_le hidx)
          have htSt₁ : t < st₁.nextLocal :=
            Nat.lt_of_lt_of_le ht (alloc_nextLocal_le hidx)
          have hcurSt₁ : cur < st₁.nextLocal :=
            Nat.lt_of_lt_of_le hcur (alloc_nextLocal_le hidx)
          have hidxSt₁ : idx < st₁.nextLocal := by
            rw [hidxEq, hidxNext]
            exact Nat.lt_succ_self _
          have hpSub : p ≠ sub := by rw [hsubEq]; exact Nat.ne_of_lt hpSt₁
          have htSub : t ≠ sub := by rw [hsubEq]; exact Nat.ne_of_lt htSt₁
          have hp₂ : s₂.locals p = some (.mut rp vp) := by
            simpa [s₂, hpSub] using hp₁
          have ht₂ : s₂.locals t = some (.mut rt vt) := by
            simpa [s₂, htSub] using ht₁
          have hsub₂ : s₂.locals sub = some old := by simp [s₂]
          have hpNext : p < st₂.nextLocal :=
            Nat.lt_of_lt_of_le hpSt₁ (alloc_nextLocal_le hsub)
          have htNext : t < st₂.nextLocal :=
            Nat.lt_of_lt_of_le htSt₁ (alloc_nextLocal_le hsub)
          have hsubLt : sub < st₂.nextLocal := by
            rw [hsubEq, hsubNext]
            exact Nat.lt_succ_self _
          have hk' : k + 1 = (pre ++ [i]).length := by simp [hk]
          have hpath' : rt.path = rp.path ++ (pre ++ [i]) ++ ns := by
            simpa [List.append_assoc] using hpath
          obtain ⟨s₃, hrun, houtVal, hbelow, hmem⟩ :=
            ih hpNext htNext hsubLt hp₂ ht₂ hsub₂ hk' hpath' hrest
          have hcur₃ : s₃.locals cur = some (.vector es) := by
            rw [hbelow cur (Nat.lt_of_lt_of_le hcur
              (Nat.le_trans (alloc_nextLocal_le hidx) (alloc_nextLocal_le hsub)))]
            have hcurSub : cur ≠ sub := by
              rw [hsubEq]
              exact Nat.ne_of_lt hcurSt₁
            simpa [s₂, hcurSub] using hcur₁
          have hidx₃ : s₃.locals idx = some (.u64 i) := by
            have hidxLt : idx < st₂.nextLocal :=
              Nat.lt_of_lt_of_le hidxSt₁ (alloc_nextLocal_le hsub)
            rw [hbelow idx hidxLt]
            have hidxSub : idx ≠ sub := by
              rw [hsubEq]
              exact Nat.ne_of_lt hidxSt₁
            simpa [s₂, hidxSub] using hidx₁
          have hbound : i < es.length := (List.getElem?_eq_some_iff.mp helem).1
          let s₄ := s₃.writeLocal out (.vector (es.set i subVal))
          have hlast : InstrPath [.call [out] .vecSet [cur, idx, out']]
              s₃ s₄ := InstrPath.one
            (InstrNext.vecSet hcur₃ hidx₃ houtVal hrest.result_free hbound)
          have houtFresh : st.nextLocal ≤ out := by
            rw [(alloc_nextLocal hout).1]
            exact Nat.le_trans (alloc_nextLocal_le hidx)
              (Nat.le_trans (alloc_nextLocal_le hsub) nested.frontier_le)
          refine ⟨s₄, ?_, by simp [s₄], ?_, by simp [s₄, hmem, s₂, s₁]⟩
          · exact (InstrPath.one
                (InstrNext.mutPathIndex hpv htv htargetIndex hibound)).append
              ((InstrPath.one (InstrNext.vecGet hcur₁ hidx₁ helem)).append
                (hrun.append hlast))
          · exact (MoveState.LocalsEqBelow.writeLocal
                (by simp [hidxEq])).trans
              ((MoveState.LocalsEqBelow.writeLocal (by
                  rw [hsubEq]
                  exact alloc_nextLocal_le hidx)).trans
                ((hbelow.mono (Nat.le_trans (alloc_nextLocal_le hidx)
                    (alloc_nextLocal_le hsub))).trans
                  (MoveState.LocalsEqBelow.writeLocal houtFresh)))

/-- Decompose emission of a nonempty child-to-parent write-back into its
initial parent read, recursive functional update, and final mutation write. -/
theorem wbBody_refPath_inv { Δ : StructDecls} {d : FunDecl}
    {p t child : LocalIndex} {step : BStep} {rest : List BStep}
    {st st' : EmitSt} {body : List Instr}
    (h : wbBody Δ d t ⟨.refNode p, child, step :: rest⟩ st =
      .ok (body, st')) :
    ∃ tp a instrs out st₁,
      mutPayloadTy d p = .ok tp ∧ alloc tp st = .ok (a, st₁) ∧
      BuildUpdateTrace Δ p t 0 a tp (step :: rest) st₁
        instrs out st' ∧
      body = .call [a] .getMut [p] :: instrs ++
        [.call [p] .setMut [p, out]] := by
  simp only [wbBody] at h
  change ((mutPayloadTy d p).map (·, st)).bind (fun first =>
      (alloc first.1 first.2).bind (fun second =>
        (buildUpdate Δ p t 0 second.1 first.1 (step :: rest)
            second.2).bind (fun third =>
          .ok ((.call [second.1] .getMut [p] :: third.1.1 ++
            [.call [p] .setMut [p, third.1.2]]), third.2)))) =
    .ok (body, st') at h
  obtain ⟨first, htpMap, h⟩ := Except.bind_ok_inv h
  obtain ⟨tp, sttp⟩ := first
  obtain ⟨htp, hsttp⟩ := Except.map_pair_ok_inv htpMap
  subst sttp
  obtain ⟨second, ha, h⟩ := Except.bind_ok_inv h
  obtain ⟨a, st₁⟩ := second
  obtain ⟨third, hupdate, hout⟩ := Except.bind_ok_inv h
  obtain ⟨result, stEnd⟩ := third
  obtain ⟨instrs, out⟩ := result
  simp only [Except.ok.injEq, Prod.mk.injEq] at hout
  obtain ⟨hbody, hstate⟩ := hout
  rw [hstate] at hupdate
  exact ⟨tp, a, instrs, out, st₁, htp, ha,
    buildUpdate_trace hupdate, hbody.symm⟩

/-- Generating a write-back body may allocate temporaries but never moves or
extends the active emitter cursor. -/
theorem wbBody_cursor { Δ : StructDecls} {d : FunDecl}
    {t : LocalIndex} {e : BEdge} {st st' : EmitSt} {body : List Instr}
    (h : wbBody Δ d t e st = .ok (body, st')) :
    st'.curId = st.curId ∧ st'.cur = st.cur := by
  rcases e with ⟨parent, child, path⟩
  cases parent with
  | refNode p =>
      cases path with
      | nil =>
          obtain ⟨tp, tc, c, htp, htt, hc, hbody⟩ :=
            wbBody_refDirect_inv h
          exact alloc_cursor hc
      | cons step rest =>
          obtain ⟨tp, a, instrs, out, st₁, htp, ha, trace, hbody⟩ :=
            wbBody_refPath_inv h
          exact ⟨(BuildUpdateTrace.cursor_eq trace).1.trans
              (alloc_cursor ha).1,
            (BuildUpdateTrace.cursor_eq trace).2.trans
              (alloc_cursor ha).2⟩
  | localRoot x =>
      obtain ⟨-, rfl, -⟩ := wbBody_localRoot_inv h
      exact ⟨rfl, rfl⟩
  | globalRoot r =>
      obtain ⟨ty, ad, c, st₁, hpath, hty, had, hc, hbody⟩ :=
        wbBody_globalRoot_inv h
      exact ⟨(alloc_cursor hc).1.trans (alloc_cursor had).1,
        (alloc_cursor hc).2.trans (alloc_cursor had).2⟩
  | anyRoot =>
      change Except.error "internal: anyRoot in a borrow graph" =
        Except.ok (body, st') at h
      cases h

/-- Write-back generation only advances the fresh-local frontier. -/
theorem wbBody_frontier_le { Δ : StructDecls} {d : FunDecl}
    {t : LocalIndex} {e : BEdge} {st st' : EmitSt} {body : List Instr}
    (h : wbBody Δ d t e st = .ok (body, st')) :
    st.nextLocal ≤ st'.nextLocal := by
  rcases e with ⟨parent, child, path⟩
  cases parent with
  | refNode p =>
      cases path with
      | nil =>
          obtain ⟨tp, tc, c, htp, htt, hc, hbody⟩ :=
            wbBody_refDirect_inv h
          exact alloc_nextLocal_le hc
      | cons step rest =>
          obtain ⟨tp, a, instrs, out, st₁, htp, ha, trace, hbody⟩ :=
            wbBody_refPath_inv h
          exact Nat.le_trans (alloc_nextLocal_le ha) trace.frontier_le
  | localRoot x =>
      obtain ⟨-, rfl, -⟩ := wbBody_localRoot_inv h
      exact Nat.le_refl _
  | globalRoot r =>
      obtain ⟨ty, ad, c, st₁, hpath, hty, had, hc, hbody⟩ :=
        wbBody_globalRoot_inv h
      exact Nat.le_trans (alloc_nextLocal_le had) (alloc_nextLocal_le hc)
  | anyRoot =>
      change Except.error "internal: anyRoot in a borrow graph" =
        Except.ok (body, st') at h
      cases h

/-- Execute a nonempty parent-path write-back from its concrete path-update
certificate. -/
theorem wbBody_refPath_path { Δ : StructDecls} {d : FunDecl}
    {p t child : LocalIndex} {step : BStep} {rest : List BStep}
    {emit emit' : EmitSt} {body : List Instr} {s : MoveState}
    {rp rt : RefTarget} {vp vt updated : Value}
    (hemits : wbBody Δ d t ⟨.refNode p, child, step :: rest⟩ emit =
      .ok (body, emit'))
    (hbase : d.numLocals ≤ emit.nextLocal)
    (hplt : p < d.numLocals) (htlt : t < d.numLocals)
    (hp : s.locals p = some (.mut rp vp))
    (ht : s.locals t = some (.mut rt vt))
    (hparent : isParentTarget (bPathPattern (step :: rest)) rp rt = true)
    (hupdate : PathUpdate (step :: rest)
      (rt.path.drop rp.path.length) vp vt updated) :
    ∃ s', InstrPath body s s' ∧
      s'.locals p = some (.mut rp updated) ∧
      (∀ x, x < emit.nextLocal → x ≠ p → s'.locals x = s.locals x) ∧
      s'.memory = s.memory := by
  obtain ⟨tp, a, instrs, out, st₁, htp, ha, trace, rfl⟩ :=
    wbBody_refPath_inv hemits
  obtain ⟨haEq, haNext⟩ := alloc_nextLocal ha
  have hpFresh : p < emit.nextLocal := Nat.lt_of_lt_of_le hplt hbase
  have htFresh : t < emit.nextLocal := Nat.lt_of_lt_of_le htlt hbase
  have hpa : p ≠ a := by rw [haEq]; exact Nat.ne_of_lt hpFresh
  have hta : t ≠ a := by rw [haEq]; exact Nat.ne_of_lt htFresh
  let s₁ := s.writeLocal a vp
  have hp₁ : s₁.locals p = some (.mut rp vp) := by
    simpa [s₁, hpa] using hp
  have ht₁ : s₁.locals t = some (.mut rt vt) := by
    simpa [s₁, hta] using ht
  have ha₁ : s₁.locals a = some vp := by simp [s₁]
  have hpNext : p < st₁.nextLocal :=
    Nat.lt_of_lt_of_le hpFresh (alloc_nextLocal_le ha)
  have htNext : t < st₁.nextLocal :=
    Nat.lt_of_lt_of_le htFresh (alloc_nextLocal_le ha)
  have haLt : a < st₁.nextLocal := by
    rw [haEq, haNext]
    exact Nat.lt_succ_self _
  obtain ⟨-, hpath, -⟩ := isParentTarget_parts hparent
  obtain ⟨s₂, hrun, houtVal, hbelow, hmem⟩ :=
    trace.run (pre := []) (ns := rt.path.drop rp.path.length)
      hpNext htNext haLt hp₁ ht₁ ha₁ (by simp)
      (by simpa using hpath) hupdate
  have hp₂ : s₂.locals p = some (.mut rp vp) := by
    rw [hbelow p hpNext]
    exact hp₁
  let s₃ := s₂.writeLocal p (.mut rp updated)
  have hlast : InstrPath [.call [p] .setMut [p, out]] s₂ s₃ :=
    InstrPath.one (InstrNext.setMut hp₂ houtVal hupdate.result_free)
  refine ⟨s₃, ?_, by simp [s₃], ?_, by simp [s₃, hmem, s₁]⟩
  · exact (InstrPath.one (InstrNext.getMut hp)).append (hrun.append hlast)
  · intro x hx hxp
    rw [MoveState.writeLocal_locals, if_neg hxp,
      hbelow x (Nat.lt_of_lt_of_le hx (alloc_nextLocal_le ha))]
    simp [s₁, haEq, Nat.ne_of_lt hx]

/-- Execute either shape of a reference-parent write-back and re-establish
both components of the core frame invariant, hiding all emitter temporaries. -/
theorem wbBody_refParent_rel { Δ : StructDecls} {d : FunDecl}
    {g : BGraph} {pending : List LocalIndex} {source target : MoveState}
    {t p : LocalIndex} {e : BEdge} {emit emit' : EmitSt}
    {body : List Instr} {rp rt : RefTarget} {vp vt : Value}
    (hframe : CoreFrameRel d g pending source target)
    (hready : CoreWriteReady g pending target)
    (safe : CoreDeathSafe g pending source target t)
    (ht : t ∈ pending) (hp : p ∈ pending) (hpt : p ≠ t)
    (he : e ∈ inEdges g t) (heParent : e.parent = .refNode p)
    (hsrcP : source.locals p = some (.ref rp))
    (hsrcT : source.locals t = some (.ref rt))
    (htgtP : target.locals p = some (.mut rp vp))
    (htgtT : target.locals t = some (.mut rt vt))
    (hmatch : CoreEdgeMatches source rt e)
    (hemits : wbBody Δ d t e emit = .ok (body, emit'))
    (hbase : d.numLocals ≤ emit.nextLocal) :
    ∃ target', InstrPath body target target' ∧
      CoreFrameRel d g (pending.filter (· ≠ t)) source target' ∧
      CoreWriteReady g (pending.filter (· ≠ t)) target' ∧
      target'.locals t = target.locals t := by
  rcases e with ⟨parent, child, path⟩
  simp only at heParent
  subst parent
  have hparentTarget :
      isParentTarget (bPathPattern path) rp rt = true := by
    rw [CoreEdgeMatches] at hmatch
    obtain ⟨rp', href, hparent⟩ := hmatch
    rw [hsrcP] at href
    cases href
    exact hparent
  obtain ⟨updated, hupdate⟩ := hready t ht
    ⟨.refNode p, child, path⟩ he p rfl hp rp vp rt vt
      htgtP htgtT hparentTarget
  have hplt := hframe.pending_bound p hp
  have htlt := hframe.pending_bound t ht
  obtain ⟨target', hrun, hpUpdated, hlocals, hmemory⟩ :
      ∃ target', InstrPath body target target' ∧
        target'.locals p = some (.mut rp updated) ∧
        (∀ x, x < emit.nextLocal → x ≠ p →
          target'.locals x = target.locals x) ∧
        target'.memory = target.memory := by
    cases path with
    | nil =>
        obtain ⟨-, hupdated⟩ := PathUpdate.nil_eq hupdate
        subst updated
        simpa using wbBody_refDirect_path hemits hbase hplt
          htgtP htgtT hupdate.result_free
    | cons step rest =>
        exact wbBody_refPath_path hemits hbase hplt htlt htgtP htgtT
          hparentTarget hupdate
  let baseTarget := target.writeLocal p (.mut rp updated)
  have hbelow : ∀ x, x < d.numLocals →
      target'.locals x = baseTarget.locals x := by
    intro x hx
    by_cases hxp : x = p
    · subst x
      simpa [baseTarget] using hpUpdated
    · rw [hlocals x (Nat.lt_of_lt_of_le hx hbase) hxp]
      simp [baseTarget, hxp]
  have hbaseFrame : CoreFrameRel d g (pending.filter (· ≠ t))
      source baseTarget := by
    apply hframe.writeBackParent safe hp hpt he rfl hsrcP hsrcT
      htgtP htgtT hmatch hupdate
  have hframe' := hbaseFrame.target_congr
    (by simpa [baseTarget] using hrun.current_eq) hbelow
    (by simpa [baseTarget] using hmemory)
  have hbaseReady : CoreWriteReady g (pending.filter (· ≠ t))
      baseTarget := safe.parent_ready ⟨.refNode p, child, path⟩ he p rfl hp
        rp vp rt vt updated hsrcP hsrcT htgtP htgtT hmatch hupdate
  have hready' := hbaseReady.target_congr
    (n := d.numLocals)
    (fun x hx => hframe'.pending_bound x hx) hbelow
  exact ⟨target', hrun, hframe', hready',
    hlocals t (Nat.lt_of_lt_of_le htlt hbase) hpt.symm⟩

/-- Execute a local-root write-back and re-establish the core frame
invariant after removing the dying mutation. -/
theorem wbBody_localRoot_rel { Δ : StructDecls} {d : FunDecl}
    {g : BGraph} {pending : List LocalIndex} {source target : MoveState}
    {t x child : LocalIndex} {path : List BStep} {emit emit' : EmitSt}
    {body : List Instr} {rt : RefTarget}
    (hframe : CoreFrameRel d g pending source target)
    (hready : CoreWriteReady g pending target)
    (safe : CoreDeathSafe g pending source target t)
    (ht : t ∈ pending) (hleaf : hasPendingChild g pending t = false)
    (he : ⟨.localRoot x, child, path⟩ ∈ inEdges g t)
    (hsrc : source.locals t = some (.ref rt))
    (hmatch : CoreEdgeMatches source rt ⟨.localRoot x, child, path⟩)
    (hxplain : isMutLocal d x = false)
    (hemits : wbBody Δ d t ⟨.localRoot x, child, path⟩ emit =
      .ok (body, emit')) :
    ∃ target', InstrPath body target target' ∧
      CoreFrameRel d g (pending.filter (· ≠ t)) source target' ∧
      CoreWriteReady g (pending.filter (· ≠ t)) target' ∧
      target'.locals t = target.locals t := by
  rcases rt with ⟨rtRoot, rtPath⟩
  obtain ⟨rfl, -, -⟩ := wbBody_localRoot_inv hemits
  have hmatch₀ := hmatch
  rw [CoreEdgeMatches] at hmatch
  obtain ⟨hroot, hpath⟩ := hmatch
  have hpathEq : rtPath = [] := pathMatches_nil (by
    simpa only [bPathPattern, List.map_nil] using hpath)
  subst rtPath
  change rtRoot = .loc source.current x at hroot
  subst rtRoot
  obtain ⟨rt', v, href, htgt, -, -⟩ := hframe.leaf ht hleaf hsrc
  rw [hsrc] at href
  cases href
  have hrun := wbBody_localRoot_path hemits htgt
  have hunique : ∀ e ∈ inEdges g t,
      CoreEdgeMatches source ⟨.loc source.current x, []⟩ e →
      e.parent = .localRoot x := by
    intro e' he' hmatch'
    exact congrArg BEdge.parent
      (safe.origin_unique ⟨.localRoot x, child, []⟩ he e' he'
        ⟨.loc source.current x, []⟩ hmatch₀ hmatch').symm
  have hframe' := hframe.writeBackLocal ht hleaf hsrc htgt
    rfl rfl hunique hxplain
  have hxNotPending : x ∉ pending := by
    intro hx
    have := hframe.pending_mut x hx
    rw [hxplain] at this
    contradiction
  have hreadyWrite : CoreWriteReady g pending (target.writeLocal x v) :=
    hready.writeLocal_of_not_mem (v := v) hxNotPending
  have hready' : CoreWriteReady g (pending.filter (· ≠ t))
      (target.writeLocal x v) := hreadyWrite.mono
    (small := pending.filter (· ≠ t))
    (fun u hu => (List.mem_filter.mp hu).1)
  refine ⟨target.writeLocal x v, hrun, hframe', hready', ?_⟩
  simp [ne_of_mem_of_not_mem ht hxNotPending]

/-- Execute a global-root write-back, hide its fresh temporaries, and
re-establish the core frame invariant. -/
theorem wbBody_globalRoot_rel { Δ : StructDecls} {d : FunDecl}
    {g : BGraph} {pending : List LocalIndex} {source target : MoveState}
    {t child : LocalIndex} {r : ResourceId} {path : List BStep}
    {emit emit' : EmitSt} {body : List Instr} {rt : RefTarget}
    (hframe : CoreFrameRel d g pending source target)
    (hready : CoreWriteReady g pending target)
    (safe : CoreDeathSafe g pending source target t)
    (ht : t ∈ pending) (hleaf : hasPendingChild g pending t = false)
    (he : ⟨.globalRoot r, child, path⟩ ∈ inEdges g t)
    (hsrc : source.locals t = some (.ref rt))
    (hmatch : CoreEdgeMatches source rt ⟨.globalRoot r, child, path⟩)
    (hfree : ∀ v, source.readTarget rt = some v → v.refFree)
    (hemits : wbBody Δ d t ⟨.globalRoot r, child, path⟩ emit =
      .ok (body, emit'))
    (hbase : d.numLocals ≤ emit.nextLocal) :
    ∃ target', InstrPath body target target' ∧
      CoreFrameRel d g (pending.filter (· ≠ t)) source target' ∧
      CoreWriteReady g (pending.filter (· ≠ t)) target' ∧
      target'.locals t = target.locals t := by
  rcases rt with ⟨rtRoot, rtPath⟩
  obtain ⟨_, _, _, _, rfl, -, -, -, -⟩ :=
    wbBody_globalRoot_inv hemits
  have hmatch₀ := hmatch
  rw [CoreEdgeMatches] at hmatch
  obtain ⟨a, hroot, hpath⟩ := hmatch
  have hpathEq : rtPath = [] := pathMatches_nil (by
    simpa only [bPathPattern, List.map_nil] using hpath)
  subst rtPath
  change rtRoot = .global r a at hroot
  subst rtRoot
  obtain ⟨rt', v, href, htgt, -, hread⟩ := hframe.leaf ht hleaf hsrc
  rw [hsrc] at href
  cases href
  have hvFree := hfree v hread
  obtain ⟨target', hrun, hlocals, hmemory⟩ :=
    wbBody_globalRoot_path hemits hbase (hframe.pending_bound t ht)
      htgt hvFree
  have hunique : ∀ e ∈ inEdges g t,
      CoreEdgeMatches source ⟨.global r a, []⟩ e →
      e.parent = .globalRoot r := by
    intro e' he' hmatch'
    exact congrArg BEdge.parent
      (safe.origin_unique ⟨.globalRoot r, child, []⟩ he e' he'
        ⟨.global r a, []⟩ hmatch₀ hmatch').symm
  let baseTarget := target.writeGlobal r a v
  have hbaseFrame : CoreFrameRel d g (pending.filter (· ≠ t))
      source baseTarget := hframe.writeBackGlobal ht hleaf hsrc
        htgt rfl rfl hunique
  have hbelow : ∀ x, x < d.numLocals →
      target'.locals x = baseTarget.locals x := by
    intro x hx
    simpa [baseTarget, MoveState.writeGlobal] using
      hlocals x (Nat.lt_of_lt_of_le hx hbase)
  have hframe' := hbaseFrame.target_congr
    (by change target'.current = target.current; exact hrun.current_eq) hbelow
    (by simpa [baseTarget, MoveState.writeGlobal] using hmemory)
  have hreadyBase : CoreWriteReady g (pending.filter (· ≠ t))
      baseTarget := (hready.mono
        (fun u hu => (List.mem_filter.mp hu).1)).target_congr
          (n := d.numLocals)
          (fun u hu => hframe'.pending_bound u hu) (by
            intro x hx
            simp [baseTarget, MoveState.writeGlobal])
  have hready' := hreadyBase.target_congr
    (n := d.numLocals) (fun u hu => hframe'.pending_bound u hu) hbelow
  exact ⟨target', hrun, hframe', hready',
    hlocals t (Nat.lt_of_lt_of_le (hframe.pending_bound t ht) hbase)⟩

/-- Execute the write-back body for the concrete origin selected by the
dying reference.  This is the sole dispatch over the four borrow-node shapes
used by death processing. -/
theorem wbBody_matching_rel {P : Program} {Δ : StructDecls} {d : FunDecl}
    {g : BGraph} {pending : List LocalIndex} {source target : MoveState}
    {t : LocalIndex} {e : BEdge} {emit emit' : EmitSt}
    {body : List Instr}
    (checked : CheckedState P d source)
    (hframe : CoreFrameRel d g pending source target)
    (hready : CoreWriteReady g pending target)
    (safe : CoreDeathSafe g pending source target t)
    (ht : t ∈ pending) (hleaf : hasPendingChild g pending t = false)
    (he : e ∈ inEdges g t)
    {rt : RefTarget} (hsrc : source.locals t = some (.ref rt))
    (hmatch : CoreEdgeMatches source rt e)
    (hemits : wbBody Δ d t e emit = .ok (body, emit'))
    (hbase : d.numLocals ≤ emit.nextLocal) :
    ∃ target', InstrPath body target target' ∧
      CoreFrameRel d g (pending.filter (· ≠ t)) source target' ∧
      CoreWriteReady g (pending.filter (· ≠ t)) target' ∧
      target'.locals t = target.locals t := by
  obtain ⟨rt', v, href, htgt, -, hread⟩ :=
    hframe.leaf ht hleaf hsrc
  rw [hsrc] at href
  cases href
  rcases e with ⟨parent, child, path⟩
  cases parent with
  | refNode p =>
      have hmatch' := hmatch
      rw [CoreEdgeMatches] at hmatch'
      obtain ⟨rp, hsrcP, hparent⟩ := hmatch'
      have hp : p ∈ pending := safe.parent_pending
        ⟨.refNode p, child, path⟩ he p rfl
      have hpt : p ≠ t := safe.parent_ne
        ⟨.refNode p, child, path⟩ he p rfl
      rcases hframe.mutation p hp with habsent |
          ⟨rp', vp, hrefP, htgtP, -, -⟩
      · rw [hsrcP] at habsent
        cases habsent.1
      · rw [hsrcP] at hrefP
        cases hrefP
        exact wbBody_refParent_rel hframe hready safe ht hp hpt he rfl
          hsrcP hsrc htgtP htgt hmatch hemits hbase
  | localRoot x =>
      exact wbBody_localRoot_rel hframe hready safe ht hleaf he hsrc hmatch
        (safe.local_plain ⟨.localRoot x, child, path⟩ he x rfl) hemits
  | globalRoot r =>
      obtain ⟨ty, hty⟩ := isMutLocal_eq_true.mp
        (hframe.pending_mut t ht)
      have hfree : ∀ w, source.readTarget rt = some w → w.refFree := by
        intro w hw
        exact checked.consistent.refTarget_free hty rfl hsrc hw
      exact wbBody_globalRoot_rel hframe hready safe ht hleaf he hsrc hmatch
        hfree hemits hbase
  | anyRoot =>
      change Except.error "internal: anyRoot in a borrow graph" =
        Except.ok (body, emit') at hemits
      cases hemits

/-- The cursor-level CFG shape contributed by one emitted guarded diamond. -/
theorem guardedDiamond_cfg {G : Cfg} {Δ : StructDecls}
    {d : FunDecl} {src : BlockId} {t : LocalIndex} {e : BEdge}
    {st stGuard stAppend stBody stSplit stFinal : EmitSt}
    {guardIs body : List Instr} {guard : LocalIndex}
    (hguard : wbGuard t e st = .ok ((guardIs, guard), stGuard))
    (happend : emitAll guardIs stGuard = .ok ((), stAppend))
    (hbody : wbBody Δ d t e stAppend = .ok (body, stBody))
    (hsplit : emitGuarded src guard body stBody = .ok ((), stSplit))
    (hfinal : EmitDoneIn G stFinal)
    (hretain : ∀ p ∈ stSplit.done, p ∈ stFinal.done) :
    ∃ doId contId,
      G.blocks st.curId =
        some ⟨st.cur ++ guardIs, .branch guard doId contId⟩ ∧
      G.blocks doId = some ⟨body, .jump contId⟩ ∧
      (coreAfterWrite e stSplit).curId = contId ∧
      (coreAfterWrite e stSplit).cur = [] := by
  have hguardCursor := alloc_cursor (wbGuard_inv hguard).1
  have happendCursor := emitAll_cursor happend
  have hbodyCursor := wbBody_cursor hbody
  obtain ⟨hbranch, hdo, hcontId, hcontCur⟩ := emitGuarded_blocks hsplit
  have hcursorId : stBody.curId = st.curId :=
    hbodyCursor.1.trans (happendCursor.1.trans hguardCursor.1)
  have hcursor : stBody.cur = st.cur ++ guardIs := by
    rw [hbodyCursor.2, happendCursor.2, hguardCursor.2]
  have hafter := coreAfterWrite_projection e stSplit
  refine ⟨stBody.nextId, stBody.nextId + 1, ?_, ?_, ?_, ?_⟩
  · simpa [hcursorId, hcursor] using
      hfinal _ _ (hretain _ hbranch)
  · exact hfinal _ _ (hretain _ hdo)
  · exact hafter.2.1.trans hcontId
  · exact hafter.2.2.trans hcontCur

/-- A guarded write-back trace leaves its initial cursor in the final CFG.
The first candidate closes that cursor; an empty trace preserves it. -/
theorem CoreGuardedWriteBackTrace.cursorIn {G : Cfg}
    {Δ : StructDecls} {d : FunDecl} {src : BlockId} {t : LocalIndex}
    {es : List BEdge} {st stEnd : EmitSt}
    (trace : CoreGuardedWriteBackTrace Δ d src t es st stEnd)
    (hcfg : EmitDoneIn G stEnd) (hend : EmitCursorIn G stEnd) :
    EmitCursorIn G st := by
  cases trace with
  | nil => exact hend
  | @cons e rest st stGuard stAppend stBody stSplit stEnd
      guardIs body guard hguard happend hbody hsplit tail =>
      have hretain : ∀ p ∈ stSplit.done, p ∈ stEnd.done := by
        intro p hp
        apply tail.done_subset p
        rw [(coreAfterWrite_projection e stSplit).1]
        exact hp
      obtain ⟨doId, contId, hbranch, -, -, -⟩ :=
        guardedDiamond_cfg hguard happend hbody hsplit hcfg hretain
      exact ⟨guardIs, .branch guard doId contId, hbranch⟩

/-- A complete write-back trace leaves its initial cursor in the final CFG. -/
theorem CoreWriteBackTrace.cursorIn {G : Cfg}
    {Δ : StructDecls} {d : FunDecl} {src : BlockId} {g : BGraph}
    {t : LocalIndex} {st stEnd : EmitSt}
    (trace : CoreWriteBackTrace Δ d src g t st stEnd)
    (hcfg : EmitDoneIn G stEnd) (hend : EmitCursorIn G stEnd) :
    EmitCursorIn G st := by
  cases trace with
  | none => exact hend
  | @single e st stBody stAppend stEnd body _ hbody happend hmarked =>
      have hmark : stEnd.curId = stAppend.curId ∧
          stEnd.cur = stAppend.cur := by
        cases hp : e.parent with
        | refNode p =>
            have hprojection := markWritten_projection
              (by simpa [hp] using hmarked)
            exact ⟨hprojection.1, hprojection.2.1⟩
        | _ =>
            have heq : stEnd = stAppend := by
              symm
              simpa [hp, pure, StateT.pure, Except.pure] using hmarked
            subst stEnd
            exact ⟨rfl, rfl⟩
      have happendCur := emitAll_cursor happend
      have hbodyCur := wbBody_cursor hbody
      exact EmitCursorIn.prepend
        (hmark.1.trans (happendCur.1.trans hbodyCur.1))
        (by rw [hmark.2, happendCur.2, hbodyCur.2]) hend
  | guarded _ guarded => exact guarded.cursorIn hcfg hend

/-- Death processing leaves its initial cursor in the final CFG. -/
theorem CoreDeathTrace.cursorIn {G : Cfg}
    {Δ : StructDecls} {d : FunDecl} {src : BlockId} {g : BGraph}
    {liveNow : LiveSet} {fuel : Nat} {pending pendingEnd : List LocalIndex}
    {st stEnd : EmitSt}
    (trace : CoreDeathTrace Δ d src g liveNow fuel pending st
      pendingEnd stEnd)
    (hcfg : EmitDoneIn G stEnd) (hend : EmitCursorIn G stEnd) :
    EmitCursorIn G st := by
  induction trace with
  | fuelZero | noLeaf => exact hend
  | @leaf fuel pending pendingEnd t st st' stEnd _ write rest ih =>
      have hcfg' : EmitDoneIn G st' := by
        intro b blk hb
        exact hcfg b blk (rest.done_subset (b, blk) hb)
      exact write.cursorIn hcfg' (ih hcfg hend)

/-- Rewriting one source instruction only appends straight-line code at the
active emitter cursor. -/
theorem rewriteInstr_cursor {sums : Summaries} {d : FunDecl} {g g' : BGraph}
    {pending pending' : List LocalIndex} {i : Instr} {st st' : EmitSt}
    (h : rewriteInstr sums d g pending i st =
      .ok ((g', pending'), st')) :
    st'.curId = st.curId ∧ ∃ code, st'.cur = st.cur ++ code := by
  unfold rewriteInstr at h
  cases hcore : rewriteInstrCore sums d g pending i st with
  | error e => simp [hcore] at h
  | ok result =>
      obtain ⟨value, raw⟩ := result
      simp only [hcore, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨-, rfl⟩ := h
      exact ⟨rfl, raw.cur.drop st.cur.length, rfl⟩

/-- A complete instruction trace leaves its initial cursor in the final CFG. -/
theorem CoreInstrTrace.cursorIn {G : Cfg}
    {sums : Summaries} {Δ : StructDecls} {d : FunDecl} {src : BlockId}
    {points : List (Instr × LiveSet)} {g gEnd : BGraph}
    {pending pendingEnd : List LocalIndex} {st stEnd : EmitSt}
    (trace : CoreInstrTrace sums Δ d src points g pending st
      gEnd pendingEnd stEnd)
    (hcfg : EmitDoneIn G stEnd) (hend : EmitCursorIn G stEnd) :
    EmitCursorIn G st := by
  induction trace with
  | nil => exact hend
  | @cons i liveAfter points g g' gEnd pending pending' pending''
      pendingEnd st st' st'' stEnd rewrite deaths rest ih =>
      have hcfg'' : EmitDoneIn G st'' := by
        intro b blk hb
        exact hcfg b blk (rest.done_subset (b, blk) hb)
      have hst' := deaths.cursorIn hcfg'' (ih hcfg hend)
      obtain ⟨hid, code, hcur⟩ := rewriteInstr_cursor rewrite
      exact EmitCursorIn.prepend hid hcur hst'

/-- Successful terminator emission leaves its incoming cursor in the final
CFG, including through terminator deaths and successor-edge splits. -/
theorem finishCoreBlock_cursorIn {G : Cfg} {Δ : StructDecls} {d : FunDecl}
    {liveIn : Array LiveSet} {b : BlockId} {g : BGraph}
    {pending : List LocalIndex} {term : Term} {st stEnd : EmitSt}
    (hfinish : finishCoreBlock Δ d liveIn b g pending term st =
      .ok ((), stEnd))
    (hcfg : EmitDoneIn G stEnd) : EmitCursorIn G st := by
  cases term with
  | abort code =>
      exact EmitCursorIn.of_close (by simpa [finishCoreBlock] using hfinish)
        hcfg
  | jump target =>
      simp only [finishCoreBlock] at hfinish
      change (processDeaths Δ d b g (liveIn.getD target ∅) pending st).bind _ =
        .ok ((), stEnd) at hfinish
      obtain ⟨result, hdeaths, hclose⟩ := Except.bind_ok_inv hfinish
      obtain ⟨pendingEnd, stDeaths⟩ := result
      have trace := processDeaths_trace hdeaths
      have hcfgDeaths : EmitDoneIn G stDeaths :=
        hcfg.of_subset (closeBlock_preservesDone (.jump target) 0
          stDeaths () stEnd hclose)
      exact trace.cursorIn hcfgDeaths
        (EmitCursorIn.of_close hclose hcfg)
  | ret srcs =>
      simp only [finishCoreBlock] at hfinish
      let keep := srcs.filter (isMutLocal d) ++ mutParamsOf d
      change (processDeaths Δ d b g (LiveSet.ofList keep) pending st).bind _ =
        .ok ((), stEnd) at hfinish
      obtain ⟨result, hdeaths, hrest⟩ := Except.bind_ok_inv hfinish
      obtain ⟨pendingEnd, stDeaths⟩ := result
      simp only [get, getThe, MonadStateOf.get, StateT.get, bind,
        StateT.bind, pure, Except.pure, Except.bind] at hrest
      split at hrest
      · simp [throw, throwThe, MonadExceptOf.throw, StateT.lift,
          bind, StateT.bind, Except.bind] at hrest
      · have trace := processDeaths_trace hdeaths
        have hcfgDeaths : EmitDoneIn G stDeaths :=
          hcfg.of_subset (closeBlock_preservesDone
            (.ret (srcs ++ mutParamsOf d)) 0 stDeaths () stEnd hrest)
        exact trace.cursorIn hcfgDeaths
          (EmitCursorIn.of_close hrest hcfg)
  | branch c left right =>
      simp only [finishCoreBlock] at hfinish
      change (checkRoots g pending [c] st).bind _ = .ok ((), stEnd) at hfinish
      obtain ⟨checked, hcheck, hrest⟩ := Except.bind_ok_inv hfinish
      obtain ⟨u, stChecked⟩ := checked
      cases u
      have hcheckedEq := checkRoots_state hcheck
      subst stChecked
      let liveBoth := (liveIn.getD left ∅).union (liveIn.getD right ∅)
      change (processDeaths Δ d b g liveBoth pending st).bind _ =
        .ok ((), stEnd) at hrest
      obtain ⟨deathResult, hdeaths, hrest⟩ := Except.bind_ok_inv hrest
      obtain ⟨pendingB, stDeaths⟩ := deathResult
      change (splitCoreEdge Δ d liveIn b g pendingB left stDeaths).bind _ =
        .ok ((), stEnd) at hrest
      obtain ⟨leftResult, hleft, hrest⟩ := Except.bind_ok_inv hrest
      obtain ⟨left', stLeft⟩ := leftResult
      change (splitCoreEdge Δ d liveIn b g pendingB right stLeft).bind _ =
        .ok ((), stEnd) at hrest
      obtain ⟨rightResult, hright, hclose⟩ := Except.bind_ok_inv hrest
      obtain ⟨right', stRight⟩ := rightResult
      have hcfgDeaths : EmitDoneIn G stDeaths := hcfg.of_subset (by
        intro p hp
        apply closeBlock_preservesDone (.branch c left' right') 0
          stRight () stEnd hclose p
        apply splitCoreEdge_preservesDone Δ d liveIn b g pendingB right
          stLeft right' stRight hright p
        exact splitCoreEdge_preservesDone Δ d liveIn b g pendingB left
          stDeaths left' stLeft hleft p hp)
      have hcursorRight := EmitCursorIn.of_close hclose hcfg
      have hcursorLeft := EmitCursorIn.of_cursor_eq
        (splitCoreEdge_cursor hright).1 (splitCoreEdge_cursor hright).2
        hcursorRight
      have hcursorDeaths := EmitCursorIn.of_cursor_eq
        (splitCoreEdge_cursor hleft).1 (splitCoreEdge_cursor hleft).2
        hcursorLeft
      exact (processDeaths_trace hdeaths).cursorIn hcfgDeaths hcursorDeaths

/-- One guarded candidate advances the fresh-local frontier monotonically. -/
theorem guardedWriteBack_frontier_le {Δ : StructDecls} {d : FunDecl}
    {src : BlockId} {t : LocalIndex} {e : BEdge}
    {st stGuard stAppend stBody stSplit : EmitSt}
    {guardIs body : List Instr} {guard : LocalIndex}
    (hguard : wbGuard t e st = .ok ((guardIs, guard), stGuard))
    (happend : emitAll guardIs stGuard = .ok ((), stAppend))
    (hbody : wbBody Δ d t e stAppend = .ok (body, stBody))
    (hsplit : emitGuarded src guard body stBody = .ok ((), stSplit)) :
    st.nextLocal ≤ (coreAfterWrite e stSplit).nextLocal := by
  rw [coreAfterWrite_nextLocal, emitGuarded_nextLocal hsplit]
  have hbodyFrontier := wbBody_frontier_le hbody
  rw [emitAll_nextLocal happend] at hbodyFrontier
  exact Nat.le_trans (alloc_nextLocal_le (wbGuard_inv hguard).1)
    hbodyFrontier

/-- Guarded write-back traces monotonically advance the fresh-local
frontier. -/
theorem CoreGuardedWriteBackTrace.frontier_le
    {Δ : StructDecls} {d : FunDecl} {src : BlockId} {t : LocalIndex}
    {es : List BEdge} {st stEnd : EmitSt}
    (h : CoreGuardedWriteBackTrace Δ d src t es st stEnd) :
    st.nextLocal ≤ stEnd.nextLocal := by
  induction h with
  | nil => exact Nat.le_refl _
  | cons hguard happend hbody hsplit tail ih =>
      exact Nat.le_trans
        (guardedWriteBack_frontier_le hguard happend hbody hsplit) ih

/-- Every write-back trace monotonically advances the fresh-local frontier. -/
theorem CoreWriteBackTrace.frontier_le
    {Δ : StructDecls} {d : FunDecl} {src : BlockId}
    {g : BGraph} {t : LocalIndex} {st stEnd : EmitSt}
    (h : CoreWriteBackTrace Δ d src g t st stEnd) :
    st.nextLocal ≤ stEnd.nextLocal := by
  cases h with
  | none => exact Nat.le_refl _
  | @single e st stBody stAppend stEnd body _ hbody happend hmarked =>
      have hfrontier := wbBody_frontier_le hbody
      cases hp : e.parent with
      | refNode p =>
          have hprojection := markWritten_projection
            (by simpa [hp] using hmarked)
          rw [hprojection.2.2, emitAll_nextLocal happend]
          exact hfrontier
      | _ =>
          have hstate : stAppend = stEnd := by
            simpa [hp, pure, StateT.pure, Except.pure] using hmarked
          rw [← hstate, emitAll_nextLocal happend]
          exact hfrontier
  | guarded _ trace => exact trace.frontier_le

/-- Follow the false edge of a generated guarded diamond and resume at its
continuation cursor. -/
theorem RunAfterPrefix.guardFalse {P : Program} {G : Cfg}
    {st tail : EmitSt} {guardIs : List Instr} {guard doId contId : Nat}
    {s s' : MoveState} {o : FrameOutcome}
    (hbranch : G.blocks st.curId =
      some ⟨st.cur ++ guardIs, .branch guard doId contId⟩)
    (htailId : tail.curId = contId) (htailCur : tail.cur = [])
    (hguard : InstrPath guardIs s s')
    (hfalse : s'.locals guard = some (.bool false))
    (hrest : RunAfterPrefix P G tail s' o) :
    RunAfterPrefix P G st s o := by
  obtain ⟨rest, term, hnext, hrun⟩ := hrest
  rw [htailId, htailCur] at hnext
  simp only [List.nil_append] at hnext
  refine ⟨guardIs, .branch guard doId contId, hbranch, ?_⟩
  simpa using hguard.run
    (.branchFalse (b₁ := doId) hfalse hnext hrun)

/-- Follow the true edge of a generated guarded diamond, execute its body,
and resume at its continuation cursor. -/
theorem RunAfterPrefix.guardTrue {P : Program} {G : Cfg}
    {st tail : EmitSt} {guardIs body : List Instr} {guard doId contId : Nat}
    {s sGuard sBody : MoveState} {o : FrameOutcome}
    (hbranch : G.blocks st.curId =
      some ⟨st.cur ++ guardIs, .branch guard doId contId⟩)
    (hbody : G.blocks doId = some ⟨body, .jump contId⟩)
    (htailId : tail.curId = contId) (htailCur : tail.cur = [])
    (hguard : InstrPath guardIs s sGuard)
    (htrue : sGuard.locals guard = some (.bool true))
    (hbodyRun : InstrPath body sGuard sBody)
    (hrest : RunAfterPrefix P G tail sBody o) :
    RunAfterPrefix P G st s o := by
  obtain ⟨rest, term, hnext, hrun⟩ := hrest
  rw [htailId, htailCur] at hnext
  simp only [List.nil_append] at hnext
  have hdo : RunFrom P G body (.jump contId) sGuard o :=
    by simpa using hbodyRun.run (.jump hnext hrun)
  refine ⟨guardIs, .branch guard doId contId, hbranch, ?_⟩
  simpa using hguard.run
    (.branchTrue (b₂ := contId) htrue hbody hdo)

/-- Prepend newly emitted straight-line code to a cursor continuation. -/
theorem RunAfterPrefix.prepend {P : Program} {G : Cfg}
    {before after : EmitSt} {code : List Instr} {s s' : MoveState}
    {o : FrameOutcome}
    (hid : after.curId = before.curId)
    (hcur : after.cur = before.cur ++ code)
    (hcode : InstrPath code s s')
    (hrest : RunAfterPrefix P G after s' o) :
    RunAfterPrefix P G before s o := by
  obtain ⟨rest, term, hblock, hrun⟩ := hrest
  refine ⟨code ++ rest, term, ?_, ?_⟩
  · simpa [hid, hcur, List.append_assoc] using hblock
  · exact hcode.run hrun

/-- Execute a generated dispatch guard.  Its Boolean result is true exactly
for the edge realized by the concrete source reference; the fresh guard local
is hidden from both core invariants. -/
theorem wbGuard_rel {Δ : StructDecls} {d : FunDecl} {g : BGraph}
    {pending : List LocalIndex} {source target : MoveState}
    {t : LocalIndex} {e : BEdge}
    {emit emitGuard emitAppend emitBody : EmitSt}
    {guardIs body : List Instr} {guard : LocalIndex} {rt : RefTarget}
    {vt : Value}
    (hframe : CoreFrameRel d g pending source target)
    (hready : CoreWriteReady g pending target)
    (hparent : ∀ p, e.parent = .refNode p → p ∈ pending)
    (htgt : target.locals t = some (.mut rt vt))
    (hguard : wbGuard t e emit = .ok ((guardIs, guard), emitGuard))
    (hbody : wbBody Δ d t e emitAppend =
      .ok (body, emitBody))
    (hbase : d.numLocals ≤ emit.nextLocal) :
    ∃ taken target', InstrPath guardIs target target' ∧
      target'.locals guard = some (.bool taken) ∧
      (taken = true ↔ CoreEdgeMatches source rt e) ∧
      CoreFrameRel d g pending source target' ∧
      CoreWriteReady g pending target' ∧
      MoveState.LocalsEqBelow d.numLocals target target' := by
  obtain ⟨halloc, hshape⟩ := wbGuard_inv hguard
  have hguardEq := (alloc_nextLocal halloc).1
  have hguardFresh : d.numLocals ≤ guard := by
    rw [hguardEq]
    exact hbase
  have hguardNotPending : guard ∉ pending := by
    intro hmem
    exact (Nat.not_lt_of_ge hguardFresh) (hframe.pending_bound guard hmem)
  have finish : ∀ {taken : Bool} {instr : Instr},
      guardIs = [instr] →
      InstrNext instr target (target.writeLocal guard (.bool taken)) →
      (taken = true ↔ CoreEdgeMatches source rt e) →
      ∃ taken target', InstrPath guardIs target target' ∧
        target'.locals guard = some (.bool taken) ∧
        (taken = true ↔ CoreEdgeMatches source rt e) ∧
        CoreFrameRel d g pending source target' ∧
        CoreWriteReady g pending target' ∧
        MoveState.LocalsEqBelow d.numLocals target target' := by
    intro taken instr hsingle hnext hiff
    rw [hsingle]
    refine ⟨taken, target.writeLocal guard (.bool taken),
      InstrPath.one hnext, by simp, hiff,
      hframe.writeTargetFresh hguardFresh, ?_⟩
    refine ⟨hready.writeLocal_of_not_mem hguardNotPending, ?_⟩
    exact MoveState.LocalsEqBelow.writeLocal hguardFresh
  rcases e with ⟨parent, child, path⟩
  cases parent with
  | refNode p =>
      have hp : p ∈ pending := hparent p rfl
      rcases hframe.mutation p hp with ⟨hsrcP, htgtP⟩ |
          ⟨rp, vp, hsrcP, htgtP, -, -⟩
      · simp only at hshape
        apply finish hshape (.isParentMissing htgtP htgt)
        constructor
        · simp
        · rintro ⟨parent, hpresent, -⟩
          rw [hsrcP] at hpresent
          cases hpresent
      · simp only at hshape
        apply finish hshape (InstrNext.isParent htgtP htgt)
        rw [CoreEdgeMatches]
        constructor
        · intro hmatch
          exact ⟨rp, hsrcP, hmatch⟩
        · rintro ⟨rp', hpresent, hmatch⟩
          rw [hsrcP] at hpresent
          cases hpresent
          exact hmatch
  | localRoot x =>
      obtain ⟨rfl, -, -⟩ := wbBody_localRoot_inv hbody
      simp only at hshape
      apply finish hshape (InstrNext.isMutLoc htgt)
      simp [CoreEdgeMatches, bPathPattern, hframe.current_eq]
      intro _
      exact ⟨fun h => h ▸ rfl, pathMatches_nil⟩
  | globalRoot r =>
      obtain ⟨-, -, -, -, rfl, -, -, -, -⟩ :=
        wbBody_globalRoot_inv hbody
      simp only at hshape
      apply finish hshape (InstrNext.isMutGlobal htgt)
      rcases rt with ⟨root, rtPath⟩
      cases root <;> simp [CoreEdgeMatches, bPathPattern]
      intro _
      exact ⟨fun h => h ▸ rfl, pathMatches_nil⟩
  | anyRoot => exact False.elim (by simp at hshape)

/-- If none of a guarded candidate list realizes the concrete reference,
every guard follows its false edge and the core invariants are unchanged. -/
theorem CoreGuardedWriteBackTrace.run_none {P : Program} {G : Cfg}
    {Δ : StructDecls} {d : FunDecl} {src : BlockId} {t : LocalIndex}
    {es : List BEdge} {st stEnd : EmitSt} {g : BGraph}
    {pending : List LocalIndex} {source target : MoveState}
    {rt : RefTarget} {vt : Value} {o : FrameOutcome}
    (trace : CoreGuardedWriteBackTrace Δ d src t es st stEnd)
    (hcfg : EmitDoneIn G stEnd)
    (hframe : CoreFrameRel d g pending source target)
    (hready : CoreWriteReady g pending target)
    (hparents : ∀ e ∈ es, ∀ p, e.parent = .refNode p → p ∈ pending)
    (htBound : t < d.numLocals)
    (htgt : target.locals t = some (.mut rt vt))
    (hnone : ∀ e ∈ es, ¬CoreEdgeMatches source rt e)
    (hbase : d.numLocals ≤ st.nextLocal)
    (hcont : ∀ target', CoreFrameRel d g pending source target' →
      CoreWriteReady g pending target' →
      MoveState.LocalsEqBelow d.numLocals target target' →
      RunAfterPrefix P G stEnd target' o) :
    RunAfterPrefix P G st target o := by
  induction trace generalizing target with
  | nil =>
      exact hcont target hframe hready
        (MoveState.LocalsEqBelow.refl _ _)
  | @cons e rest st stGuard stAppend stBody stSplit stEnd
      guardIs body guard hguard happend hbody hsplit tail ih =>
      have hretain : ∀ p ∈ stSplit.done, p ∈ stEnd.done := by
        intro p hp
        apply tail.done_subset p
        rw [(coreAfterWrite_projection e stSplit).1]
        exact hp
      obtain ⟨doId, contId, hbranch, hdo, htailId, htailCur⟩ :=
        guardedDiamond_cfg hguard happend hbody hsplit hcfg hretain
      obtain ⟨taken, targetGuard, hguardRun, hguardVal, htaken,
          hframeGuard, hreadyGuard, hbelowGuard⟩ :=
        wbGuard_rel hframe hready
          (fun p hp => hparents e (by simp) p hp) htgt
          hguard hbody hbase
      have hbaseTail : d.numLocals ≤
          (coreAfterWrite e stSplit).nextLocal :=
        Nat.le_trans hbase
          (guardedWriteBack_frontier_le hguard happend hbody hsplit)
      cases htakenEq : taken with
      | false =>
          have htail := ih hcfg hframeGuard hreadyGuard
            (fun e' he' p hp =>
              hparents e' (List.mem_cons_of_mem e he') p hp)
            ((hbelowGuard t htBound).trans htgt)
            (fun e' he' => hnone e' (List.mem_cons_of_mem e he'))
            hbaseTail (fun target' hf hr hsame =>
              hcont target' hf hr (hbelowGuard.trans hsame))
          exact RunAfterPrefix.guardFalse hbranch htailId htailCur
            hguardRun (by simpa [htakenEq] using hguardVal) htail
      | true =>
          have hmatch : CoreEdgeMatches source rt e :=
            htaken.mp (by simp [htakenEq])
          exact False.elim (hnone e (by simp) hmatch)

/-- A guarded candidate list containing the unique concrete origin executes
exactly that write-back; all earlier and later candidates take their false
edges. -/
theorem CoreGuardedWriteBackTrace.run_match {P P' : Program} {G : Cfg}
    {Δ : StructDecls} {d : FunDecl} {src : BlockId} {t : LocalIndex}
    {es : List BEdge} {st stEnd : EmitSt} {g : BGraph}
    {pending : List LocalIndex} {source target : MoveState}
    {rt : RefTarget} {o : FrameOutcome}
    (trace : CoreGuardedWriteBackTrace Δ d src t es st stEnd)
    (hcfg : EmitDoneIn G stEnd)
    (checked : CheckedState P d source)
    (hframe : CoreFrameRel d g pending source target)
    (hready : CoreWriteReady g pending target)
    (safe : CoreDeathSafe g pending source target t)
    (ht : t ∈ pending) (hleaf : hasPendingChild g pending t = false)
    (hsubset : ∀ e ∈ es, e ∈ inEdges g t)
    (hnodup : es.Nodup)
    (matched : BEdge) (hmatched : matched ∈ es)
    (hsrc : source.locals t = some (.ref rt))
    (hmatch : CoreEdgeMatches source rt matched)
    (hbase : d.numLocals ≤ st.nextLocal)
    (hcont : ∀ target',
      CoreFrameRel d g (pending.filter (· ≠ t)) source target' →
      CoreWriteReady g (pending.filter (· ≠ t)) target' →
      RunAfterPrefix P' G stEnd target' o) :
    RunAfterPrefix P' G st target o := by
  induction trace generalizing target with
  | nil => simp at hmatched
  | @cons e rest st stGuard stAppend stBody stSplit stEnd
      guardIs body guard hguard happend hbody hsplit tail ih =>
      have he : e ∈ inEdges g t := hsubset e (by simp)
      have htailSubset : ∀ e' ∈ rest, e' ∈ inEdges g t := by
        intro e' he'
        exact hsubset e' (List.mem_cons_of_mem e he')
      have htailNodup := (List.nodup_cons.mp hnodup).2
      have hretain : ∀ p ∈ stSplit.done, p ∈ stEnd.done := by
        intro p hp
        apply tail.done_subset p
        rw [(coreAfterWrite_projection e stSplit).1]
        exact hp
      obtain ⟨doId, contId, hbranch, hdo, htailId, htailCur⟩ :=
        guardedDiamond_cfg hguard happend hbody hsplit hcfg hretain
      obtain ⟨rt', vt, href, htgt, -, -⟩ :=
        hframe.leaf ht hleaf hsrc
      rw [hsrc] at href
      cases href
      obtain ⟨taken, targetGuard, hguardRun, hguardVal, htaken,
          hframeGuard, hreadyGuard, hbelowGuard⟩ :=
        wbGuard_rel hframe hready
          (fun p hp => safe.parent_pending e he p hp)
          htgt hguard hbody hbase
      have safeGuard : CoreDeathSafe g pending source targetGuard t :=
        safe.target_congr hframe.pending_bound ht hbelowGuard
      have hbaseGuard : d.numLocals ≤ stGuard.nextLocal :=
        Nat.le_trans hbase (alloc_nextLocal_le (wbGuard_inv hguard).1)
      have hbaseAppend : d.numLocals ≤ stAppend.nextLocal := by
        rw [emitAll_nextLocal happend]
        exact hbaseGuard
      have hbaseTail : d.numLocals ≤
          (coreAfterWrite e stSplit).nextLocal :=
        Nat.le_trans hbase
          (guardedWriteBack_frontier_le hguard happend hbody hsplit)
      cases htakenEq : taken with
      | false =>
          have hheadNone : ¬CoreEdgeMatches source rt e := by
            intro hm
            have := htaken.mpr hm
            simp [htakenEq] at this
          have hmatchedRest : matched ∈ rest := by
            rcases List.mem_cons.mp hmatched with heq | hrest
            · subst matched
              exact False.elim (hheadNone hmatch)
            · exact hrest
          have htail := ih hcfg hframeGuard hreadyGuard safeGuard
            htailSubset htailNodup hmatchedRest hbaseTail hcont
          exact RunAfterPrefix.guardFalse hbranch htailId htailCur
            hguardRun (by simpa [htakenEq] using hguardVal) htail
      | true =>
          have hheadMatch : CoreEdgeMatches source rt e :=
            htaken.mp (by simp [htakenEq])
          obtain ⟨targetBody, hbodyRun, hframeBody, hreadyBody,
              hbodyT⟩ :=
            wbBody_matching_rel checked hframeGuard hreadyGuard safeGuard
              ht hleaf he hsrc hheadMatch hbody hbaseAppend
          have htgtBody : targetBody.locals t = some (.mut rt vt) :=
            hbodyT.trans ((hbelowGuard t
              (hframe.pending_bound t ht)).trans htgt)
          have hparentsFiltered : ∀ e' ∈ rest, ∀ p,
              e'.parent = .refNode p → p ∈ pending.filter (· ≠ t) := by
            intro e' he' p hp
            have hpMem := safe.parent_pending e' (htailSubset e' he') p hp
            have hpNe := safe.parent_ne e' (htailSubset e' he') p hp
            simp [hpMem, hpNe]
          have hnone : ∀ e' ∈ rest,
              ¬CoreEdgeMatches source rt e' := by
            intro e' he' hm
            have heq := safe.origin_unique e he e'
              (htailSubset e' he') rt hheadMatch hm
            subst e'
            exact (List.nodup_cons.mp hnodup).1 he'
          have htail := tail.run_none hcfg hframeBody hreadyBody
            hparentsFiltered (hframe.pending_bound t ht) htgtBody hnone
            hbaseTail (fun target' hf hr _ => hcont target' hf hr)
          exact RunAfterPrefix.guardTrue hbranch hdo htailId htailCur
            hguardRun (by simpa [htakenEq] using hguardVal) hbodyRun htail

/-- Execute any write-back trace and continue with the dying reference
removed from the pending set. -/
theorem CoreWriteBackTrace.run {P P' : Program} {G : Cfg}
    {Δ : StructDecls} {d : FunDecl} {src : BlockId} {g : BGraph}
    {t : LocalIndex} {st stEnd : EmitSt}
    {pending : List LocalIndex} {source target : MoveState}
    {o : FrameOutcome}
    (trace : CoreWriteBackTrace Δ d src g t st stEnd)
    (hcfg : EmitDoneIn G stEnd)
    (checked : CheckedState P d source)
    (hframe : CoreFrameRel d g pending source target)
    (hready : CoreWriteReady g pending target)
    (safe : CoreDeathSafe g pending source target t)
    (ht : t ∈ pending) (hleaf : hasPendingChild g pending t = false)
    (hbase : d.numLocals ≤ st.nextLocal)
    (hcont : ∀ target',
      CoreFrameRel d g (pending.filter (· ≠ t)) source target' →
      CoreWriteReady g (pending.filter (· ≠ t)) target' →
      RunAfterPrefix P' G stEnd target' o) :
    RunAfterPrefix P' G st target o := by
  obtain ⟨rt, hsrc⟩ := safe.child_ref
  obtain ⟨matched, hmatched, hmatch⟩ := safe.origin rt hsrc
  cases trace with
  | none hedges =>
      rw [hedges] at hmatched
      simp at hmatched
  | @single e st stBody stAppend stEnd body hedges hbody happend hmarked =>
      have heq : matched = e := by simpa [hedges] using hmatched
      subst matched
      obtain ⟨target', hrun, hframe', hready', -⟩ :=
        wbBody_matching_rel checked hframe hready safe ht hleaf
          (by simp [hedges]) hsrc hmatch hbody hbase
      have hmarkCursor : stEnd.curId = stAppend.curId ∧
          stEnd.cur = stAppend.cur := by
        cases hp : e.parent with
        | refNode p =>
            have hprojection := markWritten_projection
              (by simpa [hp] using hmarked)
            exact ⟨hprojection.1, hprojection.2.1⟩
        | _ =>
            have hstate : stEnd = stAppend := by
              symm
              simpa [hp, pure, StateT.pure, Except.pure] using hmarked
            subst stEnd
            exact ⟨rfl, rfl⟩
      have hbodyCursor := wbBody_cursor hbody
      have happendCursor := emitAll_cursor happend
      apply RunAfterPrefix.prepend
        (hmarkCursor.1.trans
          (happendCursor.1.trans hbodyCursor.1))
        (by rw [hmarkCursor.2, happendCursor.2, hbodyCursor.2]) hrun
      exact hcont target' hframe' hready'
  | guarded hedges guarded =>
      exact guarded.run_match hcfg checked hframe hready safe ht hleaf
        (fun e he => by rw [hedges]; exact he)
        (by rw [← hedges]; exact safe.candidates_nodup)
        matched (by simpa [hedges] using hmatched) hsrc hmatch hbase hcont

/-- A death cascade monotonically advances the fresh-local frontier. -/
theorem CoreDeathTrace.frontier_le
    {Δ : StructDecls} {d : FunDecl} {src : BlockId} {g : BGraph}
    {liveNow : LiveSet} {fuel : Nat} {pending pendingEnd : List LocalIndex}
    {st stEnd : EmitSt}
    (h : CoreDeathTrace Δ d src g liveNow fuel pending st
      pendingEnd stEnd) :
    st.nextLocal ≤ stEnd.nextLocal := by
  induction h with
  | fuelZero | noLeaf => exact Nat.le_refl _
  | leaf _ write rest ih =>
      exact Nat.le_trans write.frontier_le ih

/-- Execute a complete death cascade using a continuation phrased at its
final emitter cursor. -/
theorem CoreDeathTrace.run {P P' : Program} {G : Cfg}
    {Δ : StructDecls} {d : FunDecl} {src : BlockId} {g : BGraph}
    {liveNow : LiveSet} {fuel : Nat} {pending pendingEnd : List LocalIndex}
    {st stEnd : EmitSt} {source target : MoveState} {o : FrameOutcome}
    (trace : CoreDeathTrace Δ d src g liveNow fuel pending st
      pendingEnd stEnd)
    (hcfg : EmitDoneIn G stEnd)
    (checked : CheckedState P d source)
    (facts : CoreCheckedFacts P)
    (hframe : CoreFrameRel d g pending source target)
    (hready : CoreWriteReady g pending target)
    (hbase : d.numLocals ≤ st.nextLocal)
    (hcont : ∀ target', CoreFrameRel d g pendingEnd source target' →
      CoreWriteReady g pendingEnd target' →
      RunAfterPrefix P' G stEnd target' o) :
    RunAfterPrefix P' G st target o := by
  induction trace generalizing target with
  | fuelZero => exact hcont target hframe hready
  | noLeaf => exact hcont target hframe hready
  | @leaf fuel pending pendingEnd t st st' stEnd hfind write rest ih =>
      obtain ⟨ht, -, hleaf⟩ := deathCandidate_of_find hfind
      have safe := facts.death checked hframe hready ht hleaf
      have hcfgWrite : EmitDoneIn G st' := by
        intro b blk hb
        exact hcfg b blk (rest.done_subset (b, blk) hb)
      have hbase' : d.numLocals ≤ st'.nextLocal :=
        Nat.le_trans hbase write.frontier_le
      apply write.run hcfgWrite checked hframe hready safe ht hleaf hbase
      intro target' hframe' hready'
      exact ih hcfg hframe' hready' hbase' hcont

/-- Compose one certified continuing rewrite with its immediately following
death cascade.  This is the common per-instruction splice used by the master
execution induction. -/
theorem CoreInstrNextSafe.runDeaths {P P' : Program} {G : Cfg}
    {d : FunDecl} {g : BGraph} {pending : List LocalIndex}
    {source target : MoveState} {before after deathEnd : EmitSt}
    {targetNext : MoveState} {code : List Instr}
    {src : BlockId} {liveAfter : LiveSet} {pendingEnd : List LocalIndex}
    {o : FrameOutcome}
    (safe : CoreInstrNextSafe d g pending source target before after
      targetNext code)
    (deaths : CoreDeathTrace P.structs d src g liveAfter
      pending.length pending after pendingEnd deathEnd)
    (hcfg : EmitDoneIn G deathEnd)
    (checked : CheckedState P d source)
    (facts : CoreCheckedFacts P)
    (hbase : d.numLocals ≤ before.nextLocal)
    (hcont : ∀ target', CoreFrameRel d g pendingEnd source target' →
      CoreWriteReady g pendingEnd target' →
      RunAfterPrefix P' G deathEnd target' o) :
    RunAfterPrefix P' G before target o := by
  have hafterBase : d.numLocals ≤ after.nextLocal :=
    Nat.le_trans hbase safe.frontier
  have hdeath := deaths.run hcfg checked facts safe.frame safe.ready
    hafterBase hcont
  exact RunAfterPrefix.prepend safe.cursorId safe.cursor safe.path hdeath

/-- Close the local core certificates under the six grouped execution cases.
Only ordinary instructions are discharged here; concrete calls and
terminators use the corresponding local obligations in `CoreCheckedFacts`. -/
theorem coreSimAt {P P' : Program} (facts : CoreCheckedFacts P)
    {G : Cfg} {is : List Instr} {term : Term} {source : MoveState}
    {sourceOutcome : FrameOutcome}
    (hrun : RunFrom.Invariant P (CheckedStateAt P) G is term source
      sourceOutcome) :
    CoreSimAt P P' G is term source sourceOutcome := by
  induction hrun with
  | @instrNext G i rest term source sourceNext sourceOutcome ok head restRun ih =>
      intro f d d' b blk points g gEnd pending pendingEnd st stInstr stEnd
        target hprogram hd hd' hinv hG hblk hterm hpoints trace hfinish hcfg
        hbase hchecked hframe hready
      cases points with
      | nil => simp at hpoints
      | cons point points =>
        obtain ⟨i', liveAfter⟩ := point
        simp only [List.map_cons, List.cons.injEq] at hpoints
        obtain ⟨rfl, hpoints⟩ := hpoints
        cases trace with
        | cons rewrite deaths restTrace =>
          have hcheckedNext : CheckedState P d sourceNext :=
            restRun.start f d hd hG
          obtain ⟨targetNext, code, safe⟩ := facts.instrNext hchecked
            hcheckedNext hframe hready head rewrite
          have hcfgInstr : EmitDoneIn d'.body stInstr :=
            hcfg.of_subset
              (finishCoreBlock_preservesDone P.structs d (liveAnalysis d) b
                gEnd pendingEnd blk.term stInstr () stEnd hfinish)
          have hcfgDeaths := hcfgInstr.of_subset restTrace.done_subset
          obtain ⟨targetOutcome, hdeath, hagree⟩ := facts.deaths deaths
            hcfgDeaths hcheckedNext safe.frame safe.ready
            (Nat.le_trans hbase safe.frontier) (fun target' hframe' hready' =>
              ih hprogram hd hd' hinv hG hblk hterm hpoints restTrace hfinish
                hcfg (Nat.le_trans hbase
                  (Nat.le_trans safe.frontier deaths.frontier_le))
                hcheckedNext hframe' hready')
          exact ⟨targetOutcome,
            RunAfterPrefix.prepend safe.cursorId safe.cursor safe.path hdeath,
            hagree⟩
  | @instrStop G i rest term source sourceOutcome ok head =>
      intro f d d' b blk points g gEnd pending pendingEnd st stInstr stEnd
        target hprogram hd hd' hinv hG hblk hterm hpoints trace hfinish hcfg
        hbase hchecked hframe hready
      cases points with
      | nil => simp at hpoints
      | cons point points =>
        obtain ⟨i', liveAfter⟩ := point
        simp only [List.map_cons, List.cons.injEq] at hpoints
        obtain ⟨rfl, hpoints⟩ := hpoints
        cases trace with
        | cons rewrite deaths restTrace =>
          obtain ⟨targetOutcome, code, safe⟩ := facts.instrStop hchecked
            hframe hready head rewrite
          have hcfgInstr : EmitDoneIn d'.body stInstr :=
            hcfg.of_subset
              (finishCoreBlock_preservesDone P.structs d (liveAnalysis d) b
                gEnd pendingEnd blk.term stInstr () stEnd hfinish)
          have hfinishCursor := finishCoreBlock_cursorIn hfinish hcfg
          have hrestCursor := restTrace.cursorIn hcfgInstr hfinishCursor
          have hcfgDeaths := hcfgInstr.of_subset restTrace.done_subset
          have hcursor := deaths.cursorIn hcfgDeaths hrestCursor
          exact ⟨targetOutcome, safe.run hcursor, safe.outcome⟩
  | @callOk runG rest term runState dsts srcs calleeId callee args retVals
      entryBlk world runOutcome ok decl argsRead arity entry calleeRun rets
      restRun ihCallee ihRest =>
      exact facts.callOk decl argsRead arity entry calleeRun.run ihCallee rets
        restRun.run ihRest
  | @callAbort runG rest term runState dsts srcs calleeId callee args
      entryBlk abortMem abortCode ok decl argsRead arity entry calleeRun
      ihCallee =>
      exact facts.callAbort decl argsRead arity entry calleeRun.run ihCallee
  | termNext ok head next ih => exact facts.termNext head next.run ih
  | termStop ok head => exact facts.termStop head

/-- A death with one static origin executes that write-back directly.  The
emitter bookkeeping after the body is intentionally hidden from the semantic
result. -/
theorem emitWriteBacks_single_rel {P : Program} {Δ : StructDecls}
    {d : FunDecl} {src : BlockId} {g : BGraph}
    {pending : List LocalIndex} {source target : MoveState}
    {t : LocalIndex} {e : BEdge} {emit emit' : EmitSt}
    (checked : CheckedState P d source)
    (hframe : CoreFrameRel d g pending source target)
    (hready : CoreWriteReady g pending target)
    (safe : CoreDeathSafe g pending source target t)
    (ht : t ∈ pending) (hleaf : hasPendingChild g pending t = false)
    (hedges : inEdges g t = [e])
    (hemits : CoreWriteBackTrace Δ d src g t emit emit')
    (hbase : d.numLocals ≤ emit.nextLocal) :
    ∃ body wbEnd target',
      wbBody Δ d t e emit = .ok (body, wbEnd) ∧
      InstrPath body target target' ∧
      CoreFrameRel d g (pending.filter (· ≠ t)) source target' ∧
      CoreWriteReady g (pending.filter (· ≠ t)) target' := by
  obtain ⟨rt, hsrc⟩ := safe.child_ref
  obtain ⟨matched, hmatched, hmatch⟩ := safe.origin rt hsrc
  have hmatchedEq : matched = e := by
    simpa [hedges] using hmatched
  subst matched
  cases hemits with
  | none hnone => simp [hedges] at hnone
  | single hedges' hbody _ _ =>
      have heq := congrArg List.head? (hedges'.symm.trans hedges)
      simp only [List.head?_cons, Option.some.injEq] at heq
      subst e
      obtain ⟨target', hrun, hframe', hready', -⟩ :=
        wbBody_matching_rel checked hframe hready safe ht hleaf
          (by simp [hedges]) hsrc hmatch hbody hbase
      exact ⟨_, _, target', hbody, hrun, hframe', hready'⟩
  | guarded hmany _ =>
      rw [hedges] at hmany
      cases hmany

/-! ## Small structural facts about the immutable pass -/

/-- `elimImmBlocks` emits exactly one optional target block for every source
block id it visits. -/
theorem elimImmBlocks_length {sigs : FunId → Option FunDecl}
    {d : FunDecl} {liveIn : Array LiveSet} {graphs : Array BGraph} :
    ∀ {bs : List BlockId} {st : ElimSt} {blocks : List (Option Block)}
      {st' : ElimSt},
      elimImmBlocks sigs d liveIn graphs bs st = .ok (blocks, st') →
      blocks.length = bs.length
  | [], st, blocks, st', h => by
      simp only [elimImmBlocks, pure, Except.pure, Except.ok.injEq,
        Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      rfl
  | b :: bs, st, blocks, st', h => by
      rw [elimImmBlocks] at h
      cases hblk : d.body.blocks b with
      | none =>
          rw [hblk] at h
          obtain ⟨p, hp, h⟩ := Except.bind_ok_inv h
          obtain ⟨rest, stEnd⟩ := p
          simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq]
            at h
          obtain ⟨rfl, rfl⟩ := h
          simp [elimImmBlocks_length hp]
      | some blk =>
          rw [hblk] at h
          obtain ⟨p, hp, h⟩ := Except.bind_ok_inv h
          obtain ⟨instrs, st₁, gEnd⟩ := p
          obtain ⟨u, -, h⟩ := Except.bind_ok_inv h
          cases u
          obtain ⟨u, -, h⟩ := Except.bind_ok_inv h
          cases u
          obtain ⟨p, hp, h⟩ := Except.bind_ok_inv h
          obtain ⟨rest, stEnd⟩ := p
          simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq]
            at h
          obtain ⟨rfl, rfl⟩ := h
          simp [elimImmBlocks_length hp]

/-- Every block produced by immutable elimination lies below the unchanged
CFG size. -/
theorem elimImmRefs_blocksLt {sigs : FunId → Option FunDecl}
    {d d' : FunDecl} (h : elimImmRefs sigs d = .ok d') :
    ∀ b, d'.body.blocks b ≠ none → b < d'.body.size := by
  unfold elimImmRefs at h
  cases hpl : decide (d.numParams ≤ d.numLocals) with
  | false =>
      have hnpl : ¬d.numParams ≤ d.numLocals := of_decide_eq_false hpl
      simp [hnpl, Except.bind, bind] at h
  | true =>
  have hpl' : d.numParams ≤ d.numLocals := of_decide_eq_true hpl
  cases hls : liveStable d (liveAnalysis d) with
  | false => simp [hpl', hls, Except.bind, bind] at h
  | true =>
  cases hgs : graphStable d (immThroughBlock d) (immAnalysis d) with
  | false => simp [hpl', hls, hgs, Except.bind, bind] at h
  | true =>
  cases hsd : gSub (paramSeeds d)
      ((immAnalysis d).getD d.body.entry []) with
  | false =>
      have hsd' : gSub (paramSeeds d)
          ((immAnalysis d)[d.body.entry]?.getD []) = false := by
        rw [← Array.getD_eq_getD_getElem?]
        exact hsd
      simp [hpl', hls, hgs, hsd', Except.bind, bind] at h
  | true =>
  have hsd' : gSub (paramSeeds d)
      ((immAnalysis d)[d.body.entry]?.getD []) = true := by
    rw [← Array.getD_eq_getD_getElem?]
    exact hsd
  cases hblocks : elimImmBlocks sigs d (liveAnalysis d) (immAnalysis d)
      (List.range d.body.size) ⟨d.numLocals, []⟩ with
  | error e =>
      simp [hpl', hls, hgs, hsd', hblocks, Except.bind, bind] at h
  | ok p =>
      obtain ⟨blocks, st⟩ := p
      simp only [hpl', hls, hgs, hsd, hblocks, Except.bind, bind,
        reduceIte, pure, Except.pure, Except.ok.injEq] at h
      subst d'
      intro b hb
      have hlen := elimImmBlocks_length hblocks
      change blocks[b]?.join ≠ none at hb
      have hb' : blocks[b]? ≠ none := by
        intro hnone
        apply hb
        rw [hnone]
        rfl
      have hlt : b < blocks.length := by
        apply Nat.lt_of_not_ge
        intro hge
        apply hb'
        rw [List.getElem?_eq_none]
        exact hge
      change b < d.body.size
      simpa [hlen] using hlt

/-- A successful whole-function pass, retained in the same compact form in
which `elimImmRefs` computed it.  Per-block semantic certificates are derived
from `blocks_ok` only when execution enters that block. -/
structure ElimImmInv (sigs : FunId → Option FunDecl) (d d' : FunDecl) :
    Prop where
  params_le : d.numParams ≤ d.numLocals
  live_stable : liveStable d (liveAnalysis d) = true
  graph_stable :
    graphStable d (immThroughBlock d) (immAnalysis d) = true
  seeds_sub : gSub (paramSeeds d)
    ((immAnalysis d).getD d.body.entry []) = true
  output : ∃ blocks : List (Option Block), ∃ state : ElimSt,
    elimImmBlocks sigs d (liveAnalysis d) (immAnalysis d)
        (List.range d.body.size) ⟨d.numLocals, []⟩ = .ok (blocks, state) ∧
      d' = { d with
        numLocals := d.numLocals + state.newTys.length
        locals := fun t =>
          if t < d.numLocals then (d.locals t).map Ty.stripImm
          else state.newTys[t - d.numLocals]?
        returns := d.returns.map Ty.stripImm
        body := { d.body with blocks := fun b => (blocks[b]?).join } }

/-- Extract the structural invariant produced by successful immutable elimination. -/
theorem elimImmRefs_inv {sigs : FunId → Option FunDecl}
    {d d' : FunDecl} (h : elimImmRefs sigs d = .ok d') :
    ElimImmInv sigs d d' := by
  unfold elimImmRefs at h
  cases hp : decide (d.numParams ≤ d.numLocals) with
  | false =>
      have hp' : ¬d.numParams ≤ d.numLocals := of_decide_eq_false hp
      simp [hp', Except.bind, bind] at h
  | true =>
    have hp' : d.numParams ≤ d.numLocals := of_decide_eq_true hp
    cases hlive : liveStable d (liveAnalysis d) with
    | false => simp [hp', hlive, Except.bind, bind] at h
    | true =>
      cases hgraph : graphStable d (immThroughBlock d) (immAnalysis d) with
      | false => simp [hp', hlive, hgraph, Except.bind, bind] at h
      | true =>
        cases hseeds : gSub (paramSeeds d)
            ((immAnalysis d).getD d.body.entry []) with
        | false =>
            have hseeds' : gSub (paramSeeds d)
                ((immAnalysis d)[d.body.entry]?.getD []) = false := by
              rw [← Array.getD_eq_getD_getElem?]
              exact hseeds
            simp [hp', hlive, hgraph, hseeds', Except.bind, bind] at h
        | true =>
          have hseeds' : gSub (paramSeeds d)
              ((immAnalysis d)[d.body.entry]?.getD []) = true := by
            rw [← Array.getD_eq_getD_getElem?]
            exact hseeds
          cases hblocks : elimImmBlocks sigs d (liveAnalysis d)
              (immAnalysis d) (List.range d.body.size)
              ⟨d.numLocals, []⟩ with
          | error e =>
              simp [hp', hlive, hgraph, hseeds', hblocks, Except.bind,
                bind] at h
          | ok result =>
            obtain ⟨blocks, st⟩ := result
            simp only [hp', hlive, hgraph, hseeds, hblocks, Except.bind,
              bind, reduceIte, pure, Except.pure, Except.ok.injEq] at h
            subst d'
            exact ⟨hp', hlive, hgraph, hseeds, blocks, st, hblocks, rfl⟩

/-- Immutable elimination preserves the function entry block identifier. -/
theorem ElimImmInv.entry_eq {sigs : FunId → Option FunDecl}
    {d d' : FunDecl} (h : ElimImmInv sigs d d') :
    d'.body.entry = d.body.entry := by
  obtain ⟨blocks, st, -, hout⟩ := h.output
  rw [hout]

/-- Immutable elimination preserves the CFG size. -/
theorem ElimImmInv.size_eq {sigs : FunId → Option FunDecl}
    {d d' : FunDecl} (h : ElimImmInv sigs d d') :
    d'.body.size = d.body.size := by
  obtain ⟨blocks, st, -, hout⟩ := h.output
  rw [hout]

/-- Immutable elimination preserves the parameter count. -/
theorem ElimImmInv.numParams_eq {sigs : FunId → Option FunDecl}
    {d d' : FunDecl} (h : ElimImmInv sigs d d') :
    d'.numParams = d.numParams := by
  obtain ⟨blocks, st, -, hout⟩ := h.output
  rw [hout]

set_option linter.unusedSimpArgs false in
/-- Rewriting one instruction never changes the fresh-local base. -/
theorem elimImmInstr_base {d : FunDecl} {st st' : ElimSt} {i : Instr}
    {tgt : List Instr} (h : elimImmInstr d st i = .ok (st', tgt)) :
    st'.base = st.base := by
  have hinv : match elimImmInstr d st i with
      | .error _ => True
      | .ok (next, _) => next.base = st.base := by
    fun_cases elimImmInstr d st i
    case case1 dst x =>
      cases hdst : localTy d dst with
      | error e => simp [hdst, pure, Except.pure, Except.bind, bind]
      | ok ty =>
        cases ty <;> simp [hdst, pure, Except.pure, Except.bind, bind]
        case ref ty =>
          cases hsrc : localTy d x with
          | error e => simp [hdst, hsrc, Except.bind, bind]
          | ok srcTy =>
            cases srcTy <;> simp [hdst, hsrc, Ty.isRef, pure, Except.pure,
              throw, throwThe, MonadExceptOf.throw, Except.bind, bind]
    case case3 dst idx t jp hboundary =>
      cases hdst : localTy d dst with
      | error e => simp [jp, hdst, Except.bind, bind]
      | ok ty =>
        cases ty <;> simp [jp, hdst, pure, Except.pure, Except.bind, bind]
        case ref ty =>
          cases hsrc : localTy d t with
          | error e => simp [jp, hdst, hsrc, Except.bind, bind]
          | ok srcTy =>
            cases srcTy <;> simp [jp, hdst, hsrc, ElimSt.alloc, pure,
              Except.pure, throw, throwThe, MonadExceptOf.throw,
              Except.bind, bind]
    case case5 dst t idx jp hboundary =>
      cases hdst : localTy d dst with
      | error e => simp [jp, hdst, Except.bind, bind]
      | ok ty =>
        cases ty <;> simp [jp, hdst, pure, Except.pure, Except.bind, bind]
        case ref ty =>
          cases hsrc : localTy d t with
          | error e => simp [jp, hdst, hsrc, Except.bind, bind]
          | ok srcTy =>
            cases srcTy <;> simp [jp, hdst, hsrc, ElimSt.alloc, pure,
              Except.pure, throw, throwThe, MonadExceptOf.throw,
              Except.bind, bind]
    case case6 dst r a =>
      cases hdst : localTy d dst with
      | error e => simp [hdst, Except.bind, bind]
      | ok ty => cases ty <;> simp [hdst, pure, Except.pure,
          Except.bind, bind]
    case case8 dst t hdstImm =>
      cases hsrc : localTy d t with
      | error e => simp [hsrc, Except.bind, bind]
      | ok ty => cases ty <;> simp [hsrc, pure, Except.pure, throw,
          throwThe, MonadExceptOf.throw, Except.bind, bind]
    case case9 dst t =>
      cases hsrc : localTy d t with
      | error e => simp [hsrc, Except.bind, bind]
      | ok ty => cases ty <;> simp [hsrc, pure, Except.pure,
          Except.bind, bind]
    case case10 t vt =>
      cases hsrc : localTy d t with
      | error e => simp [hsrc, Except.bind, bind]
      | ok ty => cases ty <;> simp [hsrc, pure, Except.pure, throw,
          throwThe, MonadExceptOf.throw, Except.bind, bind]
    all_goals
      try simp [ElimSt.alloc, pure, Except.pure, throw, throwThe,
        MonadExceptOf.throw, Except.bind, bind]
  rw [h] at hinv
  exact hinv

set_option linter.unusedSimpArgs false in
/-- Rewriting an instruction sequence never changes the fresh-local base. -/
theorem elimImmBlock_base {sigs : FunId → Option FunDecl} {d : FunDecl}
    {lat : LiveSet} : ∀ {is : List Instr} {g : BGraph} {st stEnd : ElimSt}
      {tgt : List Instr} {gEnd : BGraph},
    elimImmBlock sigs d lat is g st = .ok (tgt, stEnd, gEnd) →
      stEnd.base = st.base
  | [], _, _, _, _, _, h => by
      simp only [elimImmBlock, pure, Except.pure, Except.ok.injEq,
        Prod.mk.injEq] at h
      exact congrArg ElimSt.base h.2.1.symm
  | i :: is, g, st, stEnd, tgt, gEnd, h => by
      simp only [elimImmBlock] at h
      split at h <;> rename_i hrange
      · split at h <;> rename_i hboundary
        · obtain ⟨u, _, h⟩ := Except.bind_ok_inv h
          cases u
          obtain ⟨step, hstep, h⟩ := Except.bind_ok_inv h
          obtain ⟨st', one⟩ := step
          obtain ⟨rest, hrest, h⟩ := Except.bind_ok_inv h
          obtain ⟨restIs, restSt, restGraph⟩ := rest
          simp only [pure, Except.pure, Except.ok.injEq,
            Prod.mk.injEq] at h
          obtain ⟨_, rfl, _⟩ := h
          exact (elimImmBlock_base hrest).trans (elimImmInstr_base hstep)
        · simp [hrange, hboundary, throw, throwThe, MonadExceptOf.throw,
            Except.bind, bind] at h
      · simp [hrange, throw, throwThe, MonadExceptOf.throw,
          Except.bind, bind] at h

/-- A successfully transformed source block has a corresponding target block. -/
theorem elimImmBlocks_some_at {sigs : FunId → Option FunDecl}
    {d : FunDecl} {liveIn : Array LiveSet} {graphs : Array BGraph} :
    ∀ {bs : List BlockId} {st : ElimSt} {res : List (Option Block)}
      {stEnd : ElimSt} {k : Nat} {b : BlockId} {blk : Block},
    elimImmBlocks sigs d liveIn graphs bs st = .ok (res, stEnd) →
    bs[k]? = some b → d.body.blocks b = some blk →
    ∃ st₀ instrs st₁ gEnd,
      elimImmBlock sigs d (liveAtTermIn liveIn blk) blk.instrs
          (graphs.getD b []) st₀ = .ok (instrs, st₁, gEnd) ∧
      st₀.base = st.base ∧
      checkTermRange d blk.term = .ok () ∧
      checkRetEscape d gEnd blk.term = .ok () ∧
      res[k]? = some (some ⟨instrs, blk.term⟩)
  | [], st, res, stEnd, k, b, blk, h, hk, hb => by
      simp at hk
  | head :: tail, st, res, stEnd, k, b, blk, h, hk, hb => by
      rw [elimImmBlocks] at h
      cases hhead : d.body.blocks head with
      | none =>
          rw [hhead] at h
          obtain ⟨pair, htail, h⟩ := Except.bind_ok_inv h
          obtain ⟨rest, st'⟩ := pair
          simp only [pure, Except.pure, Except.ok.injEq,
            Prod.mk.injEq] at h
          obtain ⟨rfl, rfl⟩ := h
          cases k with
          | zero =>
              simp only [List.getElem?_cons_zero,
                Option.some.injEq] at hk
              subst b
              rw [hhead] at hb
              cases hb
          | succ k =>
              simpa using (elimImmBlocks_some_at
                (bs := tail) (st := st) (res := rest) (stEnd := st')
                (k := k) htail (by simpa using hk) hb)
      | some headBlk =>
          rw [hhead] at h
          obtain ⟨one, hone, h⟩ := Except.bind_ok_inv h
          obtain ⟨instrs, st', gEnd⟩ := one
          obtain ⟨u, hterm, h⟩ := Except.bind_ok_inv h
          cases u
          obtain ⟨u, hescape, h⟩ := Except.bind_ok_inv h
          cases u
          obtain ⟨pair, htail, h⟩ := Except.bind_ok_inv h
          obtain ⟨rest, stE⟩ := pair
          simp only [pure, Except.pure, Except.ok.injEq,
            Prod.mk.injEq] at h
          obtain ⟨rfl, rfl⟩ := h
          cases k with
          | zero =>
              simp only [List.getElem?_cons_zero,
                Option.some.injEq] at hk
              subst b
              rw [hhead] at hb
              cases hb
              exact ⟨st, instrs, st', gEnd, hone, rfl, hterm, hescape, rfl⟩
          | succ k =>
              obtain ⟨st₀, instrs, st₁, gEnd, hone₀, hbase, hterm₀,
                  hescape₀, hget⟩ := elimImmBlocks_some_at
                (bs := tail) (st := st') (res := rest) (stEnd := stE)
                (k := k) htail (by simpa using hk) hb
              exact ⟨st₀, instrs, st₁, gEnd, hone₀,
                hbase.trans (elimImmBlock_base hone), hterm₀, hescape₀,
                by simpa using hget⟩

/-! ## Per-block rewrite certificates -/

/-- A compact certificate for the image of an instruction suffix.  Each
constructor exposes exactly the three checks and one local rewrite needed by
the corresponding semantic instruction case. -/
inductive ImmSuffix (sigs : FunId → Option FunDecl) (d : FunDecl)
    (liveAtTerm : LiveSet) :
    List Instr → LiveSet → BGraph → ElimSt →
      List Instr → ElimSt → Prop where
  | nil {g : BGraph} {st : ElimSt} :
      ImmSuffix sigs d liveAtTerm [] liveAtTerm g st [] st
  | cons {i : Instr} {is : List Instr} {live : LiveSet}
      {g : BGraph} {st st' stEnd : ElimSt} {tgt rest' : List Instr}
      (range : (instrDefs i ++ instrUses i).all (· < d.numLocals) = true)
      (boundary : immBoundaryInstr sigs d i = true)
      (check : immCheck d g live i = .ok ())
      (rewrite : elimImmInstr d st i = .ok (st', tgt))
      (rest : ImmSuffix sigs d liveAtTerm is live (immStep d g i) st'
        rest' stEnd) :
      ImmSuffix sigs d liveAtTerm (i :: is) (liveThroughInstr i live) g
        st (tgt ++ rest') stEnd

/-- Relate an immutable-elimination block output to any source instruction suffix. -/
theorem elimImmBlock_suffix {sigs : FunId → Option FunDecl}
    {d : FunDecl} {lat : LiveSet} :
    ∀ {is : List Instr} {g : BGraph} {st : ElimSt} {tgt : List Instr}
      {stEnd : ElimSt} {gEnd : BGraph},
    elimImmBlock sigs d lat is g st = .ok (tgt, stEnd, gEnd) →
    ImmSuffix sigs d lat is (liveBeforeSuffix lat is) g st tgt stEnd
  | [], g, st, tgt, stEnd, gEnd, h => by
      simp only [elimImmBlock, pure, Except.pure, Except.ok.injEq,
        Prod.mk.injEq] at h
      obtain ⟨rfl, rfl, rfl⟩ := h
      exact .nil
  | i :: is, g, st, tgt, stEnd, gEnd, h => by
      cases hrange : (instrDefs i ++ instrUses i).all
          (· < d.numLocals) with
      | false => simp [elimImmBlock, hrange, Except.bind, bind] at h
      | true =>
      cases hboundary : immBoundaryInstr sigs d i with
      | false =>
          simp [elimImmBlock, hrange, hboundary, Except.bind, bind] at h
      | true =>
      cases hcheck : immCheck d g (liveBeforeSuffix lat is) i with
      | error e =>
          simp [elimImmBlock, hrange, hboundary, hcheck, Except.bind,
            bind] at h
      | ok u =>
        cases u
        cases hrewrite : elimImmInstr d st i with
        | error e =>
            simp [elimImmBlock, hrange, hboundary, hcheck, hrewrite,
              Except.bind, bind] at h
        | ok p =>
          obtain ⟨st', tgtI⟩ := p
          cases hrest : elimImmBlock sigs d lat is (immStep d g i) st' with
          | error e =>
              simp [elimImmBlock, hrange, hboundary, hcheck, hrewrite,
                hrest, Except.bind, bind] at h
          | ok q =>
            obtain ⟨rest, stE, gE⟩ := q
            have hsuffix := elimImmBlock_suffix hrest
            simp only [elimImmBlock, hrange, hboundary, hcheck, hrewrite,
              hrest, Except.bind, bind, pure, Except.pure,
              Except.ok.injEq, Prod.mk.injEq, reduceIte] at h
            obtain ⟨rfl, rfl, rfl⟩ := h
            exact .cons hrange hboundary hcheck hrewrite hsuffix

/-- Immutable block elimination finishes at the graph computed by its transfer. -/
theorem elimImmBlock_graph {sigs : FunId → Option FunDecl}
    {d : FunDecl} {lat : LiveSet} :
    ∀ {is : List Instr} {g : BGraph} {st : ElimSt} {tgt : List Instr}
      {stEnd : ElimSt} {gEnd : BGraph},
    elimImmBlock sigs d lat is g st = .ok (tgt, stEnd, gEnd) →
    gEnd = is.foldl (immStep d) g
  | [], g, st, tgt, stEnd, gEnd, h => by
      simp only [elimImmBlock, pure, Except.pure, Except.ok.injEq,
        Prod.mk.injEq] at h
      obtain ⟨-, -, rfl⟩ := h
      rfl
  | i :: is, g, st, tgt, stEnd, gEnd, h => by
      cases hrange : (instrDefs i ++ instrUses i).all
          (· < d.numLocals) with
      | false => simp [elimImmBlock, hrange, Except.bind, bind] at h
      | true =>
      cases hboundary : immBoundaryInstr sigs d i with
      | false =>
          simp [elimImmBlock, hrange, hboundary, Except.bind, bind] at h
      | true =>
      cases hcheck : immCheck d g (liveBeforeSuffix lat is) i with
      | error e =>
          simp [elimImmBlock, hrange, hboundary, hcheck, Except.bind,
            bind] at h
      | ok u =>
        cases u
        cases hrewrite : elimImmInstr d st i with
        | error e =>
            simp [elimImmBlock, hrange, hboundary, hcheck, hrewrite,
              Except.bind, bind] at h
        | ok p =>
          obtain ⟨st', tgtI⟩ := p
          cases hrest : elimImmBlock sigs d lat is (immStep d g i) st' with
          | error e =>
              simp [elimImmBlock, hrange, hboundary, hcheck, hrewrite,
                hrest, Except.bind, bind] at h
          | ok q =>
            obtain ⟨rest, stE, gE⟩ := q
            have hg := elimImmBlock_graph hrest
            simp only [elimImmBlock, hrange, hboundary, hcheck, hrewrite,
              hrest, Except.bind, bind, pure, Except.pure,
              Except.ok.injEq, Prod.mk.injEq, reduceIte] at h
            obtain ⟨-, -, rfl⟩ := h
            exact hg

/-- Invert immutable elimination of an assignment instruction. -/
theorem elimImmInstr_assign_inv {d : FunDecl} {st st' : ElimSt}
    {dst src : LocalIndex} {tgt : List Instr}
    (h : elimImmInstr d st (.assign dst src) = .ok (st', tgt)) :
    st' = st ∧ tgt = [.assign dst src] ∧
      isImmLocal d dst = isImmLocal d src := by
  simp only [elimImmInstr] at h
  cases hd : isImmLocal d dst <;> cases hs : isImmLocal d src <;>
    simp [hd, hs, pure, Except.pure, throw, throwThe,
      MonadExceptOf.throw] at h ⊢
  all_goals exact ⟨h.1.symm, h.2.symm⟩

/-- Invert immutable elimination of a load instruction. -/
theorem elimImmInstr_load_inv {d : FunDecl} {st st' : ElimSt}
    {dst : LocalIndex} {v : Value} {tgt : List Instr}
    (h : elimImmInstr d st (.load dst v) = .ok (st', tgt)) :
    st' = st ∧ tgt = [.load dst v] ∧ v.refFree := by
  simp only [elimImmInstr] at h
  cases hfree : v.refFree <;>
    simp [hfree, pure, Except.pure, throw, throwThe,
      MonadExceptOf.throw] at h ⊢
  exact ⟨h.1.symm, h.2.symm⟩

/-- Invert immutable elimination of a no-op instruction. -/
theorem elimImmInstr_nop_inv {d : FunDecl} {st st' : ElimSt}
    {tgt : List Instr} (h : elimImmInstr d st .nop = .ok (st', tgt)) :
    st' = st ∧ tgt = [.nop] := by
  simp only [elimImmInstr, pure, Except.pure, Except.ok.injEq,
    Prod.mk.injEq] at h
  exact ⟨h.1.symm, h.2.symm⟩

/-- A successful local-type query identifies the declared local type. -/
theorem localTy_ok {d : FunDecl} {x : LocalIndex} {ty : Ty}
    (h : localTy d x = .ok ty) : d.locals x = some ty := by
  unfold localTy at h
  cases hx : d.locals x with
  | none => simp [hx] at h
  | some ty' =>
      simp only [hx, pure, Except.pure, Except.ok.injEq] at h
      rw [h]

/-- Invert immutable elimination of a local borrow. -/
theorem elimImmInstr_borrowLoc_inv {d : FunDecl} {st st' : ElimSt}
    {dst x : LocalIndex} {tgt : List Instr}
    (h : elimImmInstr d st (.call [dst] .borrowLoc [x]) = .ok (st', tgt)) :
    st' = st ∧
      ((isImmLocal d dst = true ∧ isImmLocal d x = false ∧
          isMutLocal d x = false ∧ tgt = [.assign dst x]) ∨
       (isImmLocal d dst = false ∧
          tgt = [.call [dst] .borrowLoc [x]])) := by
  simp only [elimImmInstr] at h
  obtain ⟨tyd, htyd, h⟩ := Except.bind_ok_inv h
  have hlocd := localTy_ok htyd
  cases tyd
  case ref u =>
      obtain ⟨tyx, htyx, h⟩ := Except.bind_ok_inv h
      have hlocx := localTy_ok htyx
      cases href : tyx.isRef with
      | true =>
          rw [href] at h
          simp [throw, throwThe, MonadExceptOf.throw, Except.bind,
            bind] at h
      | false =>
          rw [href] at h
          simp only [Bool.false_eq_true, reduceIte, Except.bind, bind,
            pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
          refine ⟨h.1.symm, Or.inl ⟨by simp [isImmLocal, hlocd], ?_, ?_,
            h.2.symm⟩⟩
          · unfold isImmLocal
            rw [hlocx]
            cases tyx <;> simp_all [Ty.isRef]
          · unfold isMutLocal
            rw [hlocx]
            cases tyx <;> simp_all [Ty.isRef]
  all_goals
    simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    exact ⟨h.1.symm, Or.inr ⟨by simp [isImmLocal, hlocd], h.2.symm⟩⟩

/-- Invert immutable elimination of a reference read. -/
theorem elimImmInstr_readRef_inv {d : FunDecl} {st st' : ElimSt}
    {dst t : LocalIndex} {tgt : List Instr}
    (h : elimImmInstr d st (.call [dst] .readRef [t]) = .ok (st', tgt)) :
    st' = st ∧ ((isImmLocal d t = true ∧ tgt = [.assign dst t]) ∨
      (isImmLocal d t = false ∧
        tgt = [.call [dst] .readRef [t]])) := by
  simp only [elimImmInstr] at h
  obtain ⟨ty, hty, h⟩ := Except.bind_ok_inv h
  have hloc := localTy_ok hty
  match ty, h with
  | .ref u, h =>
      simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      exact ⟨h.1.symm, Or.inl ⟨by simp [isImmLocal, hloc], h.2.symm⟩⟩
  | .mutRef u, h | .uint _, h | .sint _, h | .bool, h | .address, h | .signer, h
  | .typeParam _, h | .struct _, h | .structInst _ _, h
  | .enum _, h | .enumInst _ _, h | .vector _, h =>
      simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      exact ⟨h.1.symm, Or.inr ⟨by simp [isImmLocal, hloc], h.2.symm⟩⟩

/-- Invert immutable elimination of a reference freeze. -/
theorem elimImmInstr_freezeRef_inv {d : FunDecl} {st st' : ElimSt}
    {dst t : LocalIndex} {tgt : List Instr}
    (h : elimImmInstr d st (.call [dst] .freezeRef [t]) =
      .ok (st', tgt)) :
    st' = st ∧ isImmLocal d dst = true ∧
      ((isImmLocal d t = true ∧ tgt = [.assign dst t]) ∨
       (isMutLocal d t = true ∧
        tgt = [.call [dst] .readRef [t]])) := by
  simp only [elimImmInstr] at h
  cases hdimm : isImmLocal d dst with
  | false => rw [hdimm] at h; simp at h
  | true =>
      rw [hdimm] at h
      simp only [Bool.not_true, Bool.false_eq_true, reduceIte] at h
      obtain ⟨ty, hty, h⟩ := Except.bind_ok_inv h
      have hloc := localTy_ok hty
      cases ty
      case ref u =>
          simp only [pure, Except.pure, Except.ok.injEq,
            Prod.mk.injEq] at h
          exact ⟨h.1.symm, rfl,
            Or.inl ⟨by simp [isImmLocal, hloc], h.2.symm⟩⟩
      case mutRef u =>
          simp only [pure, Except.pure, Except.ok.injEq,
            Prod.mk.injEq] at h
          exact ⟨h.1.symm, rfl,
            Or.inr ⟨by simp [isMutLocal, hloc], h.2.symm⟩⟩
      all_goals simp at h

/-- Invert immutable elimination of a reference write. -/
theorem elimImmInstr_writeRef_inv {d : FunDecl} {st st' : ElimSt}
    {t vt : LocalIndex} {tgt : List Instr}
    (h : elimImmInstr d st (.call [] .writeRef [t, vt]) =
      .ok (st', tgt)) :
    st' = st ∧ isImmLocal d t = false ∧
      tgt = [.call [] .writeRef [t, vt]] := by
  simp only [elimImmInstr] at h
  obtain ⟨ty, hty, h⟩ := Except.bind_ok_inv h
  have hloc := localTy_ok hty
  cases ty
  case ref u => simp [throw, throwThe, MonadExceptOf.throw] at h
  all_goals
    simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    exact ⟨h.1.symm, by simp [isImmLocal, hloc], h.2.symm⟩

/-- Invert immutable elimination of a global borrow. -/
theorem elimImmInstr_borrowGlobal_inv {d : FunDecl} {st st' : ElimSt}
    {dst t : LocalIndex} {r : ResourceId} {tgt : List Instr}
    (h : elimImmInstr d st (.call [dst] (.borrowGlobal r) [t]) =
      .ok (st', tgt)) :
    st' = st ∧
      ((isImmLocal d dst = true ∧
          tgt = [.call [dst] (.getGlobal r) [t]]) ∨
       (isImmLocal d dst = false ∧
          tgt = [.call [dst] (.borrowGlobal r) [t]])) := by
  simp only [elimImmInstr] at h
  obtain ⟨ty, hty, h⟩ := Except.bind_ok_inv h
  have hloc := localTy_ok hty
  cases ty
  case ref u =>
      simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      exact ⟨h.1.symm, Or.inl ⟨by simp [isImmLocal, hloc], h.2.symm⟩⟩
  all_goals
    simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    exact ⟨h.1.symm, Or.inr ⟨by simp [isImmLocal, hloc], h.2.symm⟩⟩

/-- A non-reference operation with a defined semantic result passes through
the immutable rewrite unchanged. -/
theorem elimImmInstr_function_inv {d : FunDecl} {st st' : ElimSt}
    {dsts srcs : List LocalIndex} {f : FunId} {tgt : List Instr}
    (h : elimImmInstr d st (.call dsts (.function f) srcs) =
      .ok (st', tgt)) :
    st' = st ∧ tgt = [.call dsts (.function f) srcs] := by
  simp only [elimImmInstr, pure, Except.pure, Except.ok.injEq,
    Prod.mk.injEq] at h
  exact ⟨h.1.symm, h.2.symm⟩

/-- A non-reference operation with a defined semantic result passes through
the immutable rewrite unchanged. -/
theorem elimImmInstr_op_inv {d : FunDecl} {st st' : ElimSt}
    {dsts srcs : List LocalIndex} {op : Oper} {tgt : List Instr}
    {current : FrameId} {deref : RefTarget → Option Value}
    {vs : List Value} {m : Memory} {oo : OpOutcome}
    (hsem : op.sem current deref vs m = some oo)
    (h : elimImmInstr d st (.call dsts op srcs) = .ok (st', tgt)) :
    st' = st ∧ tgt = [.call dsts op srcs] := by
  revert h
  generalize hi : Instr.call dsts op srcs = i
  fun_cases elimImmInstr d st i <;> intro h <;>
    first
      | exact Instr.noConfusion hi
      | (simp [throw, throwThe, MonadExceptOf.throw] at h; done)
      | (simp only [pure, Except.pure, Except.ok.injEq,
           Prod.mk.injEq] at h
         exact ⟨h.1.symm, h.2.symm⟩)
      | (cases hi
         exact nomatch hsem)

/-- Ordinary operations leave the immutable borrow graph unchanged. -/
theorem immStep_op {d : FunDecl} {g : BGraph}
    {dsts srcs : List LocalIndex} {op : Oper}
    {current : FrameId} {deref : RefTarget → Option Value}
    {vs : List Value} {m : Memory} {oo : OpOutcome}
    (hsem : op.sem current deref vs m = some oo) :
    immStep d g (.call dsts op srcs) = g := by
  generalize hi : Instr.call dsts op srcs = i
  fun_cases immStep d g i <;>
    first
      | exact Instr.noConfusion hi
      | rfl
      | (cases hi
         exact nomatch hsem)

/-- Parameter-kind agreement exposed by the call-boundary check. -/
theorem immBoundaryInstr_function_param {sigs : FunId → Option FunDecl}
    {caller callee : FunDecl} {f : FunId}
    {dsts srcs : List LocalIndex}
    (hcallee : sigs f = some callee)
    (hboundary : immBoundaryInstr sigs caller
      (.call dsts (.function f) srcs) = true)
    {i src : LocalIndex} (hsrc : srcs[i]? = some src) :
    isImmLocal caller src = isImmLocal callee i ∧
      (caller.locals src).any Ty.isRef = (callee.locals i).any Ty.isRef := by
  have hi : i < srcs.length := by
    apply Nat.lt_of_not_ge
    intro hge
    rw [List.getElem?_eq_none hge] at hsrc
    cases hsrc
  simp only [immBoundaryInstr, hcallee, Bool.and_eq_true] at hboundary
  have hparam := List.all_eq_true.mp hboundary.1 i (by simp [hi])
  rw [hsrc] at hparam
  simpa using hparam

/-- Decided borrow-graph inclusion implies edge membership inclusion. -/
theorem gSub_mem {a b : BGraph} (h : gSub a b = true)
    {e : BEdge} (he : e ∈ a) : e ∈ b := by
  simp only [gSub] at h
  have := List.all_eq_true.mp h e he
  simpa [List.contains_iff_mem] using this

/-- Invert immutable elimination of a field borrow. -/
theorem elimImmInstr_borrowField_inv {d : FunDecl} {st st' : ElimSt}
    {dst t : LocalIndex} {i : Nat} {tgt : List Instr}
    (h : elimImmInstr d st (.call [dst] (.borrowField i) [t]) =
      .ok (st', tgt)) :
    (isImmLocal d dst = true ∧ isImmLocal d t = true ∧ st' = st ∧
      tgt = [.call [dst] (.getField i) [t]]) ∨
    (isImmLocal d dst = true ∧ isMutLocal d t = true ∧
      (∃ ty, d.locals t = some (.mutRef ty) ∧
        st' = (st.alloc ty).1 ∧
        tgt = [.call [(st.alloc ty).2] .readRef [t],
               .call [dst] (.getField i) [(st.alloc ty).2]])) ∨
    (isImmLocal d dst = false ∧ isImmLocal d t = false ∧ st' = st ∧
      tgt = [.call [dst] (.borrowField i) [t]]) := by
  simp only [elimImmInstr] at h
  cases hcnd : (!isImmLocal d dst && isImmLocal d t) with
  | true =>
      rw [hcnd] at h
      simp [throw, throwThe, MonadExceptOf.throw, Except.bind, bind] at h
  | false =>
    rw [hcnd] at h
    simp only [Bool.false_eq_true, reduceIte] at h
    obtain ⟨tyd, htyd, h⟩ := Except.bind_ok_inv h
    have hlocd := localTy_ok htyd
    cases tyd
    case ref u =>
        obtain ⟨tyt, htyt, h⟩ := Except.bind_ok_inv h
        have hloct := localTy_ok htyt
        cases tyt
        case ref u' =>
            simp only [pure, Except.pure, Except.ok.injEq,
              Prod.mk.injEq] at h
            exact Or.inl ⟨by simp [isImmLocal, hlocd],
              by simp [isImmLocal, hloct], h.1.symm, h.2.symm⟩
        case mutRef ty =>
            simp only [pure, Except.pure, Except.ok.injEq,
              Prod.mk.injEq] at h
            exact Or.inr (Or.inl ⟨by simp [isImmLocal, hlocd],
              by simp [isMutLocal, hloct],
              ty, hloct, h.1.symm, h.2.symm⟩)
        all_goals simp [throw, throwThe, MonadExceptOf.throw] at h
    all_goals
      simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      refine Or.inr (Or.inr ⟨by simp [isImmLocal, hlocd], ?_, h.1.symm,
        h.2.symm⟩)
      have hd : isImmLocal d dst = false := by simp [isImmLocal, hlocd]
      rw [hd] at hcnd
      simpa using hcnd

/-- Invert immutable elimination of a vector-element borrow. -/
theorem elimImmInstr_borrowVecElem_inv {d : FunDecl} {st st' : ElimSt}
    {dst t it : LocalIndex} {tgt : List Instr}
    (h : elimImmInstr d st (.call [dst] .borrowVecElem [t, it]) =
      .ok (st', tgt)) :
    (isImmLocal d dst = true ∧ isImmLocal d t = true ∧ st' = st ∧
      tgt = [.call [dst] .vecGet [t, it]]) ∨
    (isImmLocal d dst = true ∧ isMutLocal d t = true ∧
      (∃ ty, d.locals t = some (.mutRef ty) ∧
        st' = (st.alloc ty).1 ∧
        tgt = [.call [(st.alloc ty).2] .readRef [t],
               .call [dst] .vecGet [(st.alloc ty).2, it]])) ∨
    (isImmLocal d dst = false ∧ isImmLocal d t = false ∧ st' = st ∧
      tgt = [.call [dst] .borrowVecElem [t, it]]) := by
  simp only [elimImmInstr] at h
  cases hcnd : (!isImmLocal d dst && isImmLocal d t) with
  | true =>
      rw [hcnd] at h
      simp [throw, throwThe, MonadExceptOf.throw, Except.bind, bind] at h
  | false =>
    rw [hcnd] at h
    simp only [Bool.false_eq_true, reduceIte] at h
    obtain ⟨tyd, htyd, h⟩ := Except.bind_ok_inv h
    have hlocd := localTy_ok htyd
    cases tyd
    case ref u =>
        obtain ⟨tyt, htyt, h⟩ := Except.bind_ok_inv h
        have hloct := localTy_ok htyt
        cases tyt
        case ref u' =>
            simp only [pure, Except.pure, Except.ok.injEq,
              Prod.mk.injEq] at h
            exact Or.inl ⟨by simp [isImmLocal, hlocd],
              by simp [isImmLocal, hloct], h.1.symm, h.2.symm⟩
        case mutRef ty =>
            simp only [pure, Except.pure, Except.ok.injEq,
              Prod.mk.injEq] at h
            exact Or.inr (Or.inl ⟨by simp [isImmLocal, hlocd],
              by simp [isMutLocal, hloct],
              ty, hloct, h.1.symm, h.2.symm⟩)
        all_goals simp [throw, throwThe, MonadExceptOf.throw] at h
    all_goals
      simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      refine Or.inr (Or.inr ⟨by simp [isImmLocal, hlocd], ?_, h.1.symm,
        h.2.symm⟩)
      have hd : isImmLocal d dst = false := by simp [isImmLocal, hlocd]
      rw [hd] at hcnd
      simpa using hcnd

/-- Entering a source block exposes its target instruction suffix and all
terminator checks, without materializing a whole-function proof object. -/
theorem ElimImmInv.block {sigs : FunId → Option FunDecl}
    {d d' : FunDecl} (h : ElimImmInv sigs d d') {b : BlockId}
    {blk : Block} (hlt : b < d.body.size) (hb : d.body.blocks b = some blk) :
    ∃ tgt st stEnd gEnd,
      ImmSuffix sigs d (liveAtTermOf d blk) blk.instrs
        (liveBeforeSuffix (liveAtTermOf d blk) blk.instrs)
        ((immAnalysis d).getD b []) st tgt stEnd ∧
      d.numLocals ≤ st.base ∧
      d'.body.blocks b = some ⟨tgt, blk.term⟩ ∧
      checkTermRange d blk.term = .ok () ∧
      checkRetEscape d gEnd blk.term = .ok () ∧
      gEnd = blk.instrs.foldl (immStep d) ((immAnalysis d).getD b []) := by
  obtain ⟨blocks, final, hblocks, hout⟩ := h.output
  have hrange : (List.range d.body.size)[b]? = some b := by
    simp [hlt]
  obtain ⟨st, tgt, stEnd, gEnd, hone, hbase, hterm, hescape, hget⟩ :=
    elimImmBlocks_some_at hblocks hrange hb
  refine ⟨tgt, st, stEnd, gEnd, elimImmBlock_suffix hone,
    by rw [hbase]; simp, ?_, hterm,
    hescape, elimImmBlock_graph hone⟩
  rw [hout]
  change blocks[b]?.join = some ⟨tgt, blk.term⟩
  rw [hget]
  rfl

/-- Successful terminator range checking bounds every referenced local. -/
theorem checkTermRange_lt {d : FunDecl} {term : Term}
    (h : checkTermRange d term = .ok ()) {x : LocalIndex}
    (hx : x ∈ termReads term) : x < d.numLocals := by
  unfold checkTermRange at h
  split at h
  · next hall =>
      exact of_decide_eq_true (List.all_eq_true.mp hall x hx)
  · simp [throw, throwThe, MonadExceptOf.throw] at h

/-- Successful return escape checking leaves no immutable ancestor for returns. -/
theorem checkRetEscape_imm_empty {d : FunDecl} {g : BGraph}
    {srcs : List LocalIndex}
    (h : checkRetEscape d g (.ret srcs) = .ok ())
    {x : LocalIndex} (hx : x ∈ srcs) (himm : isImmLocal d x = true) :
    (immAncestors g x).isEmpty := by
  simp only [checkRetEscape] at h
  split at h
  · next hall =>
      have hxall := List.all_eq_true.mp hall x hx
      simp only [himm, Bool.not_true, Bool.false_or, Bool.and_eq_true,
        Bool.not_eq_true'] at hxall
      exact hxall.2
  · simp [throw, throwThe, MonadExceptOf.throw] at h

/-- Characterize membership in the immutable-ancestor collection. -/
theorem mem_immAncestors {g : BGraph} {x : LocalIndex} {n : BNode} :
    n ∈ immAncestors g x ↔
      ∃ e ∈ g, e.child = x ∧ e.parent = n := by
  simp only [immAncestors, List.mem_eraseDups, List.mem_map, inEdges,
    List.mem_filter, beq_iff_eq]
  constructor
  · rintro ⟨e, ⟨he, hc⟩, hp⟩
    exact ⟨e, he, hc, hp⟩
  · rintro ⟨e, he, hc, hp⟩
    exact ⟨e, ⟨he, hc⟩, hp⟩

/-- A reference parameter contributes its abstract-root seed edge. -/
theorem anyRoot_edge_mem_paramSeeds {d : FunDecl} {x : LocalIndex}
    (hx : x < d.numParams) (href : (d.locals x).any Ty.isRef = true) :
    ⟨.anyRoot, x, []⟩ ∈ paramSeeds d := by
  simp [paramSeeds, hx, href]

/-- An inserted borrow edge belongs to the resulting graph. -/
theorem mem_gInsert_self {e : BEdge} {g : BGraph} : e ∈ gInsert e g := by
  unfold gInsert
  split
  · next h => simpa [List.contains_iff_mem] using h
  · exact List.mem_cons_self

/-- Existing borrow edges survive insertion. -/
theorem mem_gInsert {e e' : BEdge} {g : BGraph} (h : e ∈ g) :
    e ∈ gInsert e' g := by
  unfold gInsert
  split <;> simp [h]

/-- An edge from the folded list belongs to the insertion fold result. -/
theorem mem_foldl_gInsert {e : BEdge} :
    ∀ {es : List BEdge} {g : BGraph}, e ∈ g →
      e ∈ es.foldl (fun g edge => gInsert edge g) g
  | [], _, h => h
  | edge :: rest, g, h => by
      simp only [List.foldl_cons]
      exact mem_foldl_gInsert (mem_gInsert h)

/-- An edge from the initial graph survives an insertion fold. -/
theorem mem_foldl_gInsert_of_mem {e : BEdge} :
    ∀ {es : List BEdge} {g : BGraph}, e ∈ es →
      e ∈ es.foldl (fun g edge => gInsert edge g) g
  | [], _, h => nomatch h
  | edge :: rest, g, h => by
      simp only [List.foldl_cons]
      rcases List.mem_cons.mp h with rfl | hrest
      · exact mem_foldl_gInsert mem_gInsert_self
      · exact mem_foldl_gInsert_of_mem hrest

/-- Closed insertion contains its newly inserted borrow edge. -/
theorem mem_gInsertClosed_self {e : BEdge} {g : BGraph} :
    e ∈ gInsertClosed e g := by
  unfold gInsertClosed
  exact mem_foldl_gInsert mem_gInsert_self

/-- Closed insertion preserves every existing borrow edge. -/
theorem mem_gInsertClosed {e e' : BEdge} {g : BGraph} (h : e ∈ g) :
    e ∈ gInsertClosed e' g := by
  unfold gInsertClosed
  exact mem_foldl_gInsert (mem_gInsert h)

/-- Compute immutable ancestors after inserting a local-borrow edge. -/
theorem immAncestors_borrowLoc {g : BGraph} {x dst : LocalIndex} :
    .localRoot x ∈
      immAncestors (gInsertClosed ⟨.localRoot x, dst, []⟩ g) dst := by
  apply mem_immAncestors.mpr
  exact ⟨⟨.localRoot x, dst, []⟩, mem_gInsertClosed_self, rfl, rfl⟩

/-- Compute immutable ancestors after inserting a global-borrow edge. -/
theorem immAncestors_borrowGlobal {g : BGraph} {r : ResourceId}
    {dst : LocalIndex} :
    .globalRoot r ∈
      immAncestors (gInsertClosed ⟨.globalRoot r, dst, []⟩ g) dst := by
  apply mem_immAncestors.mpr
  exact ⟨⟨.globalRoot r, dst, []⟩, mem_gInsertClosed_self, rfl, rfl⟩

/-- Copying a reference transfers its immutable ancestors to the destination. -/
theorem immAncestors_copy {g : BGraph} {src dst : LocalIndex}
    {n : BNode} (h : n ∈ immAncestors g src) :
    n ∈ immAncestors
      (gInsertClosed ⟨.refNode src, dst, []⟩ g) dst := by
  obtain ⟨edge, hedge, hchild, hparent⟩ := mem_immAncestors.mp h
  apply mem_immAncestors.mpr
  refine ⟨⟨n, dst, []⟩, ?_, rfl, rfl⟩
  unfold gInsertClosed
  apply mem_foldl_gInsert_of_mem
  simp only [List.mem_map]
  have hin : edge ∈ inEdges g src := by
    simp [inEdges, hedge, hchild]
  exact ⟨edge, hin, by simp [hparent]⟩

/-- A stable graph analysis contains the transfer result at each declared block. -/
theorem graphStable_entry {d : FunDecl} {b : BlockId} {blk : Block}
    {succ : BlockId}
    (hstable : graphStable d (immThroughBlock d) (immAnalysis d) = true)
    (hb : d.body.blocks b = some blk) (hlt : b < d.body.size)
    (hsucc : succ ∈ termSuccs blk.term) {e : BEdge}
    (he : e ∈ immThroughBlock d blk ((immAnalysis d).getD b [])) :
    e ∈ (immAnalysis d).getD succ [] := by
  have hall := List.all_eq_true.mp hstable b
    (by simp [List.mem_range, hlt])
  simp only [hb] at hall
  have hs := List.all_eq_true.mp hall succ (by simpa using hsucc)
  simp only [gSub] at hs
  have he' := List.all_eq_true.mp hs e (by simpa using he)
  simpa [List.contains_iff_mem] using he'

/-- Characterize membership in the bounded ancestor-collection traversal. -/
theorem mem_collectAnc {g : BGraph} {n : BNode} :
    ∀ {ts : List LocalIndex} {acc : List BNode},
    n ∈ collectAnc g ts acc ↔
      (∃ t ∈ ts, n ∈ immAncestors g t) ∨ n ∈ acc
  | [], acc => by simp [collectAnc]
  | t :: ts, acc => by
      simp only [collectAnc, List.foldl_cons]
      rw [show (List.foldl _ _ ts : List BNode) = collectAnc g ts _ from
        rfl, mem_collectAnc, mem_foldl_dedupInsert]
      constructor
      · rintro (⟨u, hu, hn⟩ | hn | ha)
        · exact Or.inl ⟨u, List.mem_cons_of_mem _ hu, hn⟩
        · exact Or.inl ⟨t, List.mem_cons_self, hn⟩
        · exact Or.inr ha
      · rintro (⟨u, hu, hn⟩ | ha)
        · rcases List.mem_cons.mp hu with rfl | hu
          · exact Or.inr (Or.inl hn)
          · exact Or.inl ⟨u, hu, hn⟩
        · exact Or.inr (Or.inr ha)

/-- Protected ancestors arise from a live mutable-reference local. -/
theorem mem_protectedAnc {d : FunDecl} {g : BGraph} {live : LiveSet}
    {i : Instr} {t : LocalIndex} {n : BNode} (ht : t ∈ live)
    (himm : isImmLocal d t = true) (hn : n ∈ immAncestors g t) :
    n ∈ protectedAnc d g live i := by
  have hbase : n ∈ collectAnc g (live.toList.filter (isImmLocal d)) [] :=
    mem_collectAnc.mpr
      (Or.inl ⟨t, List.mem_filter.mpr
        ⟨Std.ExtTreeSet.mem_toList.mpr ht, himm⟩, hn⟩)
  unfold protectedAnc
  split
  · exact mem_collectAnc.mpr (Or.inr hbase)
  · exact hbase

/-- Immutable checking prevents protected ancestors from being overwritten. -/
theorem immCheck_ok_defs {d : FunDecl} {g : BGraph} {live : LiveSet}
    {i : Instr} (h : immCheck d g live i = .ok ()) {x : LocalIndex}
    (hx : x ∈ instrDefs i)
    (hmem : .localRoot x ∈ protectedAnc d g live i ∨
      .localRoot x ∈ collectAnc g ((instrUses i).filter (fun u =>
        isImmLocal d u || isMutLocal d u)) []) : False := by
  have hdefs : (instrDefs i).any (fun y =>
      (protectedAnc d g live i).contains (.localRoot y) ||
      (collectAnc g ((instrUses i).filter (fun u =>
        isImmLocal d u || isMutLocal d u)) []).contains
        (.localRoot y)) = true := by
    refine List.any_eq_true.mpr ⟨x, hx, ?_⟩
    rcases hmem with hm | hm <;> simp [hm]
  unfold immCheck at h
  simp only [hdefs, reduceIte, throw, throwThe,
    MonadExceptOf.throw] at h
  split at h <;> (try split at h) <;> simp at h

/-- A checked local borrow never borrows an immutable-reference slot itself. -/
theorem immCheck_borrowLoc_source_plain {d : FunDecl} {g : BGraph}
    {live : LiveSet} {dst x : LocalIndex}
    (h : immCheck d g live (.call [dst] .borrowLoc [x]) = .ok ()) :
    isImmLocal d x = false := by
  cases hx : isImmLocal d x with
  | false => rfl
  | true =>
      simp [immCheck, hx, throw, throwThe, MonadExceptOf.throw] at h

/-- A mutable-reference local is not classified as immutable. -/
theorem isImmLocal_false_of_isMutLocal {d : FunDecl} {x : LocalIndex}
    (h : isMutLocal d x = true) : isImmLocal d x = false := by
  unfold isMutLocal at h
  unfold isImmLocal
  cases hx : d.locals x with
  | none => simp [hx] at h
  | some ty =>
      rw [hx] at h
      cases ty <;> simp at h ⊢

/-- Immutable-local classification identifies an immutable-reference declaration. -/
theorem isImmLocal_decl {d : FunDecl} {x : LocalIndex}
    (h : isImmLocal d x = true) : ∃ ty, d.locals x = some (.ref ty) := by
  unfold isImmLocal at h
  cases hx : d.locals x with
  | none => simp [hx] at h
  | some ty =>
      rw [hx] at h
      cases ty <;> simp_all

/-! ## Stack-indexed immutable simulation -/

/-- Static coverage of a dynamic reference in the current activation.
Local borrows created in this frame retain their precise local-root node;
roots inherited through parameters or calls are represented by `anyRoot`.
Unlike the former checkout relation, no root renaming is involved. -/
def ImmCovers (frame : FrameId) (g : BGraph) (x : LocalIndex)
    (rt : RefTarget) : Prop :=
  match rt.root with
  | .loc rootFrame y =>
      if rootFrame = frame then .localRoot y ∈ immAncestors g x
      else .anyRoot ∈ immAncestors g x
  | .global r _ =>
      .globalRoot r.resource ∈ immAncestors g x ∨
        .anyRoot ∈ immAncestors g x

/-- Coverage of an immutable reference implies a nonempty ancestor set. -/
theorem ImmCovers.not_empty {frame : FrameId} {g : BGraph}
    {x : LocalIndex} {rt : RefTarget} (h : ImmCovers frame g x rt) :
    (immAncestors g x).isEmpty = false := by
  have of_mem : ∀ {n : BNode}, n ∈ immAncestors g x →
      (immAncestors g x).isEmpty = false := by
    intro n hn
    cases ha : immAncestors g x with
    | nil => rw [ha] at hn; cases hn
    | cons a rest => rfl
  cases rt with
  | mk root path =>
    cases root with
    | loc rootFrame y =>
        simp only [ImmCovers] at h
        split at h <;> exact of_mem h
    | global r a =>
        simp only [ImmCovers] at h
        rcases h with h | h <;> exact of_mem h

/-- Immutable coverage is monotone under borrow-graph inclusion. -/
theorem ImmCovers.mono {frame : FrameId} {g g' : BGraph}
    {x : LocalIndex} {rt : RefTarget}
    (hsub : ∀ e, e ∈ g → e ∈ g') (h : ImmCovers frame g x rt) :
    ImmCovers frame g' x rt := by
  have hanc : ∀ n, n ∈ immAncestors g x →
      n ∈ immAncestors g' x := by
    intro n hn
    obtain ⟨e, he, hc, hp⟩ := mem_immAncestors.mp hn
    exact mem_immAncestors.mpr ⟨e, hsub e he, hc, hp⟩
  unfold ImmCovers at h ⊢
  cases rt with
  | mk root path =>
    cases root with
    | loc rootFrame y =>
        by_cases heq : rootFrame = frame
        · simp only [heq, if_pos] at h ⊢
          exact hanc _ h
        · simp only [heq] at h ⊢
          exact hanc _ h
    | global r a => exact h.imp (hanc _) (hanc _)

/-- Copy-edge insertion transfers immutable coverage to the destination. -/
theorem ImmCovers.copy {frame : FrameId} {g : BGraph}
    {src dst : LocalIndex} {rt : RefTarget} (h : ImmCovers frame g src rt) :
    ImmCovers frame (gInsertClosed ⟨.refNode src, dst, []⟩ g) dst rt := by
  unfold ImmCovers at h ⊢
  cases rt with
  | mk root path =>
    cases root with
    | loc rootFrame y =>
        by_cases heq : rootFrame = frame
        · simp only [heq, if_pos] at h ⊢
          exact immAncestors_copy h
        · simp only [heq] at h ⊢
          exact immAncestors_copy h
    | global r a =>
        exact h.imp immAncestors_copy immAncestors_copy

/-- A covered reference root cannot be an instruction-defined local. -/
theorem ImmCovers.root_ne_def {d : FunDecl} {g : BGraph}
    {live : LiveSet} {i : Instr}
    (hcheck : immCheck d g live i = .ok ())
    {frame : FrameId} {x : LocalIndex} (hx : x ∈ live)
    (himm : isImmLocal d x = true) {rt : RefTarget}
    (hcov : ImmCovers frame g x rt) {dst : LocalIndex}
    (hdst : dst ∈ instrDefs i) : rt.root ≠ .loc frame dst := by
  intro hroot
  have hanc : .localRoot dst ∈ immAncestors g x := by
    unfold ImmCovers at hcov
    rw [hroot] at hcov
    simpa using hcov
  exact immCheck_ok_defs hcheck hdst
    (Or.inl (mem_protectedAnc hx himm hanc))

/-- The three value-level situations seen by ordinary operations: an
unchanged reference-free value, a retained mutable reference, or an
immutable reference replaced by the value it denotes. -/
inductive ImmViewRel (s s' : MoveState) : Value → Value → Prop where
  | free {v : Value} (hfree : v.refFree) : ImmViewRel s s' v v
  | kept {rt : RefTarget}
      (hread : s'.readTarget rt = s.readTarget rt) :
      ImmViewRel s s' (.ref rt) (.ref rt)
  | copied {rt : RefTarget} {v : Value}
      (hfree : v.refFree) (hread : s.readTarget rt = some v) :
      ImmViewRel s s' (.ref rt) v

/-- Related immutable views dereference to the same source value. -/
theorem ImmViewRel.derefWith {s s' : MoveState} {v v' : Value}
    (h : ImmViewRel s s' v v') :
    v'.derefWith s'.readTarget = v.derefWith s.readTarget := by
  cases h with
  | free hfree =>
      cases v <;> simp_all [Value.derefWith]
  | kept hread => exact hread
  | copied hfree hread =>
      cases v' <;> simp_all [Value.derefWith]

/-- Reference-free related views are equal. -/
theorem ImmViewRel.eq_of_refFree {s s' : MoveState} {v v' : Value}
    (h : ImmViewRel s s' v v') (hfree : v.refFree) : v' = v := by
  cases h with
  | free => rfl
  | kept => simp at hfree
  | copied => simp at hfree

/-- A related source value that is not a reference equals its target view. -/
theorem ImmViewRel.eq_of_not_ref {s s' : MoveState} {v v' : Value}
    (h : ImmViewRel s s' v v') (hnref : ∀ rt, v ≠ .ref rt) : v' = v := by
  cases h with
  | free => rfl
  | kept => exact (hnref _ rfl).elim
  | copied => exact (hnref _ rfl).elim

/-- Pointwise lifting of immutable-view relatedness to value lists. -/
inductive ImmViews (s s' : MoveState) : List Value → List Value → Prop where
  | nil : ImmViews s s' [] []
  | cons {v v' : Value} {vs vs' : List Value} :
      ImmViewRel s s' v v' → ImmViews s s' vs vs' →
      ImmViews s s' (v :: vs) (v' :: vs')

/-- Pointwise immutable-view related lists have equal length. -/
theorem ImmViews.length {s s' : MoveState} {vs vs' : List Value}
    (h : ImmViews s s' vs vs') : vs'.length = vs.length := by
  induction h <;> simp_all

/-- Related lists are equal when every source value is reference-free. -/
theorem ImmViews.eq_of_refFree {s s' : MoveState} {vs vs' : List Value}
    (h : ImmViews s s' vs vs') (hfree : Value.refFreeList vs) :
    vs' = vs := by
  induction h with
  | nil => rfl
  | cons hv hvs ih =>
      simp only [Value.refFreeList, Bool.and_eq_true] at hfree
      rw [hv.eq_of_refFree hfree.1, ih hfree.2]

/-- Related lists are equal when no source value is a reference. -/
theorem ImmViews.eq_of_not_ref {s s' : MoveState} {vs vs' : List Value}
    (h : ImmViews s s' vs vs')
    (hnref : ∀ v ∈ vs, ∀ rt, v ≠ .ref rt) : vs' = vs := by
  induction h with
  | nil => rfl
  | cons hv hvs ih =>
      rw [hv.eq_of_not_ref (fun rt => hnref _ (by simp) rt),
        ih (fun v hv rt => hnref v (by simp [hv]) rt)]

/-- Dereferencing a list of related views recovers the source values. -/
theorem ImmViews.mapM_deref {s s' : MoveState} {vs vs' : List Value}
    (h : ImmViews s s' vs vs') :
    vs'.mapM (Value.derefWith s'.readTarget) =
      vs.mapM (Value.derefWith s.readTarget) := by
  induction h with
  | nil => rfl
  | cons hv hvs ih =>
      simp only [List.mapM_cons, hv.derefWith, ih]

/-- Equality-operation semantics agrees on immutable-view related operands. -/
theorem ImmViews.eq_sem {s s' : MoveState} {vs vs' : List Value}
    (h : ImmViews s s' vs vs') (hcurrent : s'.current = s.current)
    (hmemory : s'.memory = s.memory) :
    Oper.eq.sem s'.current s'.readTarget vs' s'.memory =
      Oper.eq.sem s.current s.readTarget vs s.memory := by
  rw [hcurrent, hmemory]
  cases h with
  | nil => rfl
  | cons h₁ hs =>
      cases hs with
      | nil => rfl
      | cons h₂ hs =>
          cases hs with
          | nil => simp only [Oper.sem, h₁.derefWith, h₂.derefWith]
          | cons => rfl

def UsesDeref (op : Oper) : Prop :=
  op = .eq ∨ op = .lt ∨ op = .vecLen

/- Type correctness says only dereferencing observations may consume an
immutable reference operand. -/
def ImmOperandsSafe (op : Oper) (vs : List Value) : Prop :=
  UsesDeref op ∨ ∀ v ∈ vs, ∀ rt, v ≠ .ref rt

set_option maxHeartbeats 1000000 in
/-- A semantically defined ordinary operation has admissible immutable-copy
operands. Equality, ordering, and vector length observe through references;
every other semantic clause either pattern-matches on non-reference values or
explicitly rejects reference-bearing aggregate operands.  The signed operations
delegate to `Oper.semSigned`, which likewise only accepts integer operands. -/
theorem Oper.sem_immOperandsSafe {op : Oper} {current : FrameId}
    {deref : RefTarget → Option Value} {vs : List Value} {m : Memory}
    {out : OpOutcome} (hsem : op.sem current deref vs m = some out) :
    ImmOperandsSafe op vs := by
  by_cases heq : op = .eq
  · exact Or.inl (Or.inl heq)
  · by_cases hlt : op = .lt
    · exact Or.inl (Or.inr (Or.inl hlt))
    · by_cases hlen : op = .vecLen
      · exact Or.inl (Or.inr (Or.inr hlen))
      · right
        generalize hs : op.sem current deref vs m = result at hsem
        fun_cases op.sem current deref vs m <;>
          simp_all [Oper.sem, Value.refFreeList_iff_forall]
        all_goals
          first
            | constructor <;>
                exact fun rt => Value.refFree_ne_ref (by simp_all) rt
            | exact fun v hv rt => Value.refFree_ne_ref (by simp_all) rt
            | exact fun rt => Value.refFree_ne_ref (by simp_all) rt
            | (split at hs <;> simp_all)

/- Immutable-copy views are observationally indistinguishable to an ordinary
well-typed operation: dereferencing observations see the same underlying
values, and every other operation receives no reference operands. -/
theorem ImmViews.op_sem {op : Oper} {s s' : MoveState}
    {vs vs' : List Value} {oo : OpOutcome}
    (h : ImmViews s s' vs vs') (hcurrent : s'.current = s.current)
    (hmemory : s'.memory = s.memory)
    (hsem : op.sem s.current s.readTarget vs s.memory = some oo)
    (hsafe : ImmOperandsSafe op vs) :
    op.sem s'.current s'.readTarget vs' s'.memory = some oo := by
  rcases hsafe with huses | hnref
  · rcases huses with heq | hlt | hlen
    · subst op
      rw [h.eq_sem hcurrent hmemory]
      exact hsem
    · subst op
      rw [hcurrent, hmemory]
      cases h with
      | nil => exact hsem
      | cons h₁ hs =>
          cases hs with
          | nil => exact hsem
          | cons h₂ hs =>
              cases hs with
              | nil => simpa only [Oper.sem, h₁.derefWith, h₂.derefWith] using hsem
              | cons => exact hsem
    · subst op
      rw [hcurrent, hmemory]
      cases h with
      | nil => exact hsem
      | cons h₁ hs =>
          cases hs with
          | nil => simpa only [Oper.sem, h₁.derefWith] using hsem
          | cons => exact hsem
  · by_cases heq : op = .eq
    · subst op
      rw [h.eq_sem hcurrent hmemory]
      exact hsem
    · by_cases hlt : op = .lt
      · subst op
        rw [hcurrent, hmemory]
        cases h with
        | nil => exact hsem
        | cons h₁ hs =>
            cases hs with
            | nil => exact hsem
            | cons h₂ hs =>
                cases hs with
                | nil => simpa only [Oper.sem, h₁.derefWith, h₂.derefWith] using hsem
                | cons => exact hsem
      · by_cases hlen : op = .vecLen
        · subst op
          rw [hcurrent, hmemory]
          cases h with
          | nil => exact hsem
          | cons h₁ hs =>
              cases hs with
              | nil => simpa only [Oper.sem, h₁.derefWith] using hsem
              | cons => exact hsem
        · rw [h.eq_of_not_ref hnref, hcurrent, hmemory]
          rw [Oper.sem_deref_irrel heq hlt hlen]
          exact hsem

/-- Genuinely relational state at one immutable-pass program point.  Source
typing and borrow validity live in generic `FrameSafe`; this relation only
records what may differ between source and target. -/
structure ImmFrameRel (d : FunDecl) (live : LiveSet) (g : BGraph)
    (frame : FrameId) (s s' : MoveState) : Prop where
  locals_rel : ∀ x ∈ live, x < d.numLocals → ∀ v,
    s.frames frame x = some v → ∃ v', s'.frames frame x = some v' ∧
    if isImmLocal d x then
      (v.refFree ∧ v' = v) ∨
        ∃ rt, v = .ref rt ∧ v'.refFree ∧ s.readTarget rt = some v'
    else v' = v
  plain_rel : ∀ x, x < d.numLocals → isImmLocal d x = false →
    ∀ v, s.frames frame x = some v → s'.frames frame x = some v
  refs_agree : ∀ x ∈ live, ∀ rt,
    s.frames frame x = some (.ref rt) →
    s'.readTarget rt = s.readTarget rt
  source_safe : FrameSafe live
    (fun rootLocal =>
      rootLocal < d.numLocals ∧ isImmLocal d rootLocal = false)
    (ImmCovers frame g) frame s

/-- Project value-shape safety from an immutable frame relation. -/
theorem ImmFrameRel.shape {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {s s' : MoveState}
    (h : ImmFrameRel d live g frame s s') :
    ∀ x ∈ live, ∀ v, s.frames frame x = some v →
      v.refFree ∨ ∃ rt, v = .ref rt :=
  h.source_safe.shape

/-- Project root-order safety from an immutable frame relation. -/
theorem ImmFrameRel.roots_below {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {s s' : MoveState}
    (h : ImmFrameRel d live g frame s s') :
    ∀ x ∈ live, ∀ rt, s.frames frame x = some (.ref rt) →
      ∀ rootFrame rootLocal, rt.root = .loc rootFrame rootLocal →
        rootFrame ≤ frame :=
  h.source_safe.roots_below

/-- Project plain-root classification from an immutable frame relation. -/
theorem ImmFrameRel.roots_plain {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {s s' : MoveState}
    (h : ImmFrameRel d live g frame s s') :
    ∀ x ∈ live, ∀ rt, s.frames frame x = some (.ref rt) →
      ∀ rootLocal, rt.root = .loc frame rootLocal →
        rootLocal < d.numLocals ∧ isImmLocal d rootLocal = false :=
  h.source_safe.roots_plain

/-- Project borrow-graph coverage from an immutable frame relation. -/
theorem ImmFrameRel.covers {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {s s' : MoveState}
    (h : ImmFrameRel d live g frame s s') :
    ∀ x ∈ live, ∀ rt, s.frames frame x = some (.ref rt) →
      ImmCovers frame g x rt :=
  h.source_safe.covers

/-- Restrict an immutable frame relation to a smaller live set. -/
theorem ImmFrameRel.restrict {d : FunDecl} {live live' : LiveSet}
    {g : BGraph} {frame : FrameId} {s s' : MoveState}
    (h : ImmFrameRel d live g frame s s')
    (hsub : ∀ x, x ∈ live' → x ∈ live) :
    ImmFrameRel d live' g frame s s' :=
  { h with
    locals_rel := fun x hx => h.locals_rel x (hsub x hx)
    refs_agree := fun x hx => h.refs_agree x (hsub x hx)
    source_safe := h.source_safe.restrict hsub }

/-- Change source and target memory in lockstep.  Borrow checking supplies
the only non-structural premise: values denoted by live immutable references
remain stable across the memory transition. -/
theorem ImmFrameRel.setMemory {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {s s' : MoveState} (h :
      ImmFrameRel d live g frame s s') (m' : Memory)
    (hstable : ∀ x ∈ live, ∀ rt,
      s.frames frame x = some (.ref rt) → isImmLocal d x = true →
      (s.setMemory m').readTarget rt = s.readTarget rt) :
    ImmFrameRel d live g frame (s.setMemory m') (s'.setMemory m') := by
  refine ⟨?_, ?_, ?_, h.source_safe.setMemory m'⟩
  · intro x hx hrange v hv
    obtain ⟨v', hv', hval⟩ := h.locals_rel x hx hrange v (by simpa using hv)
    refine ⟨v', by simpa using hv', ?_⟩
    by_cases himm : isImmLocal d x = true
    · rw [himm] at hval ⊢
      rcases hval with hfree | ⟨rt, rfl, hvfree, hread⟩
      · exact Or.inl hfree
      · exact Or.inr ⟨rt, rfl, hvfree,
          (hstable x hx rt (by simpa using hv) himm).trans hread⟩
    · simpa [himm] using hval
  · intro x hrange himm v hv
    simpa using h.plain_rel x hrange himm v (by simpa using hv)
  · intro x hx rt href
    have hold := h.refs_agree x hx rt (by simpa using href)
    cases rt with
    | mk root path =>
      cases root <;> simp [MoveState.readTarget] at hold ⊢ <;> exact hold

/-- Transport a frame relation across an arbitrary pair of state changes.
The named frame must be unchanged; callers provide stability for copied
immutable views and synchronized agreement for retained references. -/
theorem ImmFrameRel.changeState {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {s s' u u' : MoveState}
    (h : ImmFrameRel d live g frame s s')
    (hsource : u.frames frame = s.frames frame)
    (htarget : u'.frames frame = s'.frames frame)
    (hstable : ∀ x ∈ live, ∀ rt,
      s.frames frame x = some (.ref rt) → isImmLocal d x = true →
      u.readTarget rt = s.readTarget rt)
    (hagree : ∀ x ∈ live, ∀ rt,
      s.frames frame x = some (.ref rt) →
      u'.readTarget rt = u.readTarget rt) :
    ImmFrameRel d live g frame u u' := by
  refine ⟨?_, ?_, ?_, h.source_safe.changeState hsource⟩
  · intro x hx hrange v hv
    rw [hsource] at hv
    obtain ⟨v', hv', hval⟩ := h.locals_rel x hx hrange v hv
    refine ⟨v', by simpa [htarget] using hv', ?_⟩
    by_cases himm : isImmLocal d x = true
    · rw [himm] at hval ⊢
      rcases hval with hval | ⟨rt, rfl, hvfree, hread⟩
      · exact Or.inl hval
      · exact Or.inr ⟨rt, rfl, hvfree,
          (hstable x hx rt hv himm).trans hread⟩
    · simpa [himm] using hval
  · intro x hrange himm v hv
    rw [hsource] at hv
    simpa [htarget] using h.plain_rel x hrange himm v hv
  · intro x hx rt href
    rw [hsource] at href
    exact hagree x hx rt href

/-- Entering a child activation preserves every relation on a caller frame.
Frame-qualified roots make this a direct above-frame state transport. -/
theorem ImmFrameRel.enterCall_below {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {s s' : MoveState} (h : ImmFrameRel d live g frame s s')
    (hle : frame ≤ s.current) (hcurrent : s'.current = s.current)
    (args args' : List Value) :
    ImmFrameRel d live g frame (s.enterCall args) (s'.enterCall args') := by
  have hle' : frame ≤ s'.current := by simpa [hcurrent] using hle
  have sourceRead : ∀ x ∈ live, ∀ rt,
      s.frames frame x = some (.ref rt) →
      (s.enterCall args).readTarget rt = s.readTarget rt := by
    intro x hx rt href
    apply MoveState.readTarget_enterCall_of_root_le
    intro rootFrame rootLocal hroot
    exact Nat.le_trans (h.roots_below x hx rt href
      rootFrame rootLocal hroot) hle
  apply h.changeState
  · exact MoveState.enterCall_frames_of_le s args hle
  · exact MoveState.enterCall_frames_of_le s' args' hle'
  · exact fun x hx rt href _ => sourceRead x hx rt href
  · intro x hx rt href
    rw [MoveState.readTarget_enterCall_of_root_le,
      sourceRead x hx rt href]
    · exact h.refs_agree x hx rt href
    · intro rootFrame rootLocal hroot
      exact Nat.le_trans (h.roots_below x hx rt href
        rootFrame rootLocal hroot) hle'

/-- Preserve an immutable frame relation when both states receive the same frame write. -/
theorem ImmFrameRel.writeFrameLocal_same {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame rootFrame : FrameId}
    {s s' : MoveState} (h : ImmFrameRel d live g frame s s')
    (rootLocal : LocalIndex) (w : Value) (hfree : w.refFree)
    (hstable : ∀ x ∈ live, ∀ rt,
      s.frames frame x = some (.ref rt) → isImmLocal d x = true →
      (s.writeFrameLocal rootFrame rootLocal w).readTarget rt =
        s.readTarget rt) :
    ImmFrameRel d live g frame
      (s.writeFrameLocal rootFrame rootLocal w)
      (s'.writeFrameLocal rootFrame rootLocal w) := by
  by_cases hf : frame = rootFrame
  · subst rootFrame
    have oldSource : ∀ x v,
        (s.writeFrameLocal frame rootLocal w).frames frame x = some v →
        x ≠ rootLocal → s.frames frame x = some v := by
      intro x v hv hne
      simpa [MoveState.writeFrameLocal_frames, hne] using hv
    have oldTarget : ∀ x v, s'.frames frame x = some v →
        x ≠ rootLocal →
        (s'.writeFrameLocal frame rootLocal w).frames frame x = some v := by
      intro x v hv hne
      simpa [MoveState.writeFrameLocal_frames, hne] using hv
    refine ⟨?_, ?_, ?_,
      h.source_safe.writeFrameLocal rootLocal w hfree⟩
    · intro x hx hrange v hv
      by_cases hroot : x = rootLocal
      · subst x
        have : v = w := by
          symm
          simpa [MoveState.writeFrameLocal_frames] using hv
        subst v
        refine ⟨w, by simp, ?_⟩
        split <;> simp [hfree]
      · have hv0 := oldSource x v hv hroot
        obtain ⟨v', hv', hval⟩ := h.locals_rel x hx hrange v hv0
        refine ⟨v', oldTarget x v' hv' hroot, ?_⟩
        by_cases himm : isImmLocal d x = true
        · rw [himm] at hval ⊢
          rcases hval with hval | ⟨rt, rfl, hvfree, hread⟩
          · exact Or.inl hval
          · exact Or.inr ⟨rt, rfl, hvfree,
              (hstable x hx rt hv0 himm).trans hread⟩
        · simpa [himm] using hval
    · intro x hrange himm v hv
      by_cases hroot : x = rootLocal
      · subst x
        simpa [MoveState.writeFrameLocal_frames] using hv
      · exact oldTarget x v
          (h.plain_rel x hrange himm v (oldSource x v hv hroot)) hroot
    · intro x hx rt href
      by_cases hroot : x = rootLocal
      · subst x
        have : w = .ref rt := by
          simpa [MoveState.writeFrameLocal_frames] using href
        subst w
        simp at hfree
      · exact MoveState.readTarget_writeFrameLocal_eq frame rootLocal w rt
          (h.refs_agree x hx rt (oldSource x (.ref rt) href hroot))
  · apply h.changeState
    · simp [MoveState.writeFrameLocal_frames, hf]
    · simp [MoveState.writeFrameLocal_frames, hf]
    · exact hstable
    · intro x hx rt href
      exact MoveState.readTarget_writeFrameLocal_eq rootFrame rootLocal w rt
        (h.refs_agree x hx rt href)

/-- Transport an immutable frame relation across liveness and graph inclusion. -/
theorem ImmFrameRel.transport {d : FunDecl} {live live' : LiveSet}
    {g g' : BGraph} {frame : FrameId} {s s' : MoveState}
    (h : ImmFrameRel d live g frame s s')
    (hlive : ∀ x, x ∈ live' → x ∈ live)
    (hgraph : ∀ e, e ∈ g → e ∈ g') :
    ImmFrameRel d live' g' frame s s' :=
  { h.restrict hlive with
    source_safe :=
      { (h.source_safe.restrict hlive) with
        covers := fun x hx rt href =>
          (h.covers x (hlive x hx) rt href).mono hgraph } }

/-- Transport an immutable frame relation to a CFG successor entry. -/
theorem ImmFrameRel.to_successor {sigs : FunId → Option FunDecl}
    {d d' : FunDecl} (hinv : ElimImmInv sigs d d')
    {b succ : BlockId} {blk succBlk : Block}
    (hb : d.body.blocks b = some blk) (hblt : b < d.body.size)
    (hsucc : succ ∈ termSuccs blk.term)
    (hsuccBlk : d.body.blocks succ = some succBlk)
    (hsuccLt : succ < d.body.size)
    {frame : FrameId} {s s' : MoveState} {gEnd : BGraph}
    (h : ImmFrameRel d (liveAtTermOf d blk) gEnd frame s s')
    (hgEnd : gEnd = blk.instrs.foldl (immStep d)
      ((immAnalysis d).getD b [])) :
    ImmFrameRel d
      (liveBeforeSuffix (liveAtTermOf d succBlk) succBlk.instrs)
      ((immAnalysis d).getD succ []) frame s s' := by
  apply h.transport
  · intro x hx
    apply liveIn_subset_liveAtTermIn hsucc
    exact liveStable_entry hinv.live_stable hsuccBlk hsuccLt hx
  · intro e he
    apply graphStable_entry hinv.graph_stable hb hblt hsucc
    simpa [immThroughBlock, hgEnd] using he

/-- A reference-free state relates to itself at every immutable-pass program
point.  This is the external-entry instance and the base case for callees
whose arguments contain no runtime references. -/
theorem ImmFrameRel.refl_of_refFree {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {s : MoveState}
    (hloc : ∀ x v, s.frames frame x = some v → v.refFree) :
    ImmFrameRel d live g frame s s := by
  refine ⟨?_, ?_, ?_, FrameSafe.of_refFree hloc⟩
  · intro x hlive hx v hv
    refine ⟨v, hv, ?_⟩
    split
    · exact Or.inl ⟨hloc x v hv, rfl⟩
    · rfl
  · intro x hx himm v hv
    exact hv
  · intro x hx rt href
    exact absurd (hloc x (.ref rt) href) (by simp)

/-- Writing a local in a higher frame preserves a lower immutable frame relation. -/
theorem ImmFrameRel.writeLocal_above {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {s s' : MoveState} (h : ImmFrameRel d live g frame s s')
    (hlt : frame < s.current) (hcurrent : s'.current = s.current)
    (x : LocalIndex) (v v' : Value) :
    ImmFrameRel d live g frame (s.writeLocal x v) (s'.writeLocal x v') := by
  have hframe : frame ≠ s.current := Nat.ne_of_lt hlt
  have hframe' : frame ≠ s'.current := by simpa [hcurrent] using hframe
  have sourceRead : ∀ y ∈ live, ∀ rt,
      s.frames frame y = some (.ref rt) →
      (s.writeLocal x v).readTarget rt = s.readTarget rt := by
    intro y hy rt href
    apply MoveState.readTarget_writeLocal_of_rootFrame_ne
    intro rootFrame rootLocal hroot
    exact Nat.ne_of_lt (Nat.lt_of_le_of_lt
      (h.roots_below y hy rt href rootFrame rootLocal hroot) hlt)
  have targetRead : ∀ y ∈ live, ∀ rt,
      s.frames frame y = some (.ref rt) →
      (s'.writeLocal x v').readTarget rt = s'.readTarget rt := by
    intro y hy rt href
    apply MoveState.readTarget_writeLocal_of_rootFrame_ne
    intro rootFrame rootLocal hroot
    have hrootLe := h.roots_below y hy rt href rootFrame rootLocal hroot
    have hrootLt : rootFrame < s'.current := by
      rw [hcurrent]
      exact Nat.lt_of_le_of_lt hrootLe hlt
    exact Nat.ne_of_lt hrootLt
  apply h.changeState
  · simp [MoveState.writeLocal_frames, hframe]
  · simp [MoveState.writeLocal_frames, hframe']
  · exact fun y hy rt href _ => sourceRead y hy rt href
  · intro y hy rt href
    rw [targetRead y hy rt href, sourceRead y hy rt href]
    exact h.refs_agree y hy rt href

/-- Writing locals in a higher frame preserves a lower immutable frame relation. -/
theorem ImmFrameRel.writeLocals_above {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {s s' : MoveState} (h : ImmFrameRel d live g frame s s')
    (hlt : frame < s.current) (hcurrent : s'.current = s.current) :
    ∀ (xs : List LocalIndex) (vs : List Value),
      ImmFrameRel d live g frame (s.writeLocals xs vs)
        (s'.writeLocals xs vs) := by
  intro xs
  induction xs generalizing s s' with
  | nil => intro vs; cases vs <;> exact h
  | cons x xs ih =>
      intro vs
      cases vs with
      | nil => exact h
      | cons v vs =>
        exact ih (h.writeLocal_above hlt hcurrent x v v)
          (by simpa) (by simpa using hcurrent) vs

/-- A target-only write to a freshly allocated local cannot affect the
source-local relation or any well-defined reference rooted in the active
source frame. -/
theorem ImmFrameRel.writeTargetScratch {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {s s' : MoveState} (h : ImmFrameRel d live g frame s s')
    (hframe : frame = s.current) (hcurrent : s'.current = s.current)
    {tmp : LocalIndex} (htmp : d.numLocals ≤ tmp) (v : Value) :
    ImmFrameRel d live g frame s (s'.writeLocal tmp v) := by
  have hframe' : frame = s'.current := by simpa [hcurrent] using hframe
  have targetRead : ∀ y ∈ live, ∀ rt,
      s.frames frame y = some (.ref rt) →
      (s'.writeLocal tmp v).readTarget rt = s'.readTarget rt := by
    intro y hy rt href
    apply MoveState.readTarget_writeLocal_of_root_ne
    intro hroot
    have hrange := (h.roots_plain y hy rt href tmp
      (by simpa [hframe'] using hroot)).1
    exact (Nat.not_lt_of_ge htmp) hrange
  refine ⟨?_, ?_, ?_, h.source_safe⟩
  · intro y hy hyrange w hw
    obtain ⟨w', hw', hval⟩ := h.locals_rel y hy hyrange w hw
    have hyne : y ≠ tmp := fun he =>
      (Nat.not_lt_of_ge htmp) (he ▸ hyrange)
    refine ⟨w', by simpa [MoveState.writeLocal_frames, hframe', hyne]
      using hw', ?_⟩
    by_cases himm : isImmLocal d y = true
    · rw [himm] at hval ⊢
      rcases hval with hfree | ⟨rt, rfl, hwfree, hread⟩
      · exact Or.inl hfree
      · exact Or.inr ⟨rt, rfl, hwfree, hread⟩
    · simpa [himm] using hval
  · intro y hyrange himm w hw
    have hyne : y ≠ tmp := fun he =>
      (Nat.not_lt_of_ge htmp) (he ▸ hyrange)
    simpa [MoveState.writeLocal_frames, hframe', hyne] using
      h.plain_rel y hyrange himm w hw
  · intro y hy rt href
    rw [targetRead y hy rt href]
    exact h.refs_agree y hy rt href

/-- Writing through a target rooted above a tracked frame preserves its relation. -/
theorem ImmFrameRel.writeTarget_above {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {s s' : MoveState} (h : ImmFrameRel d live g frame s s')
    (hlt : frame < s.current) (hcurrent : s'.current = s.current)
    (x : LocalIndex) (v : Value) :
    ImmFrameRel d live g frame s (s'.writeLocal x v) := by
  have hframe' : frame ≠ s'.current := by
    rw [hcurrent]
    exact Nat.ne_of_lt hlt
  have targetRead : ∀ y ∈ live, ∀ rt,
      s.frames frame y = some (.ref rt) →
      (s'.writeLocal x v).readTarget rt = s'.readTarget rt := by
    intro y hy rt href
    apply MoveState.readTarget_writeLocal_of_rootFrame_ne
    intro rootFrame rootLocal hroot
    have hrootLe := h.roots_below y hy rt href rootFrame rootLocal hroot
    rw [hcurrent]
    exact Nat.ne_of_lt (Nat.lt_of_le_of_lt hrootLe hlt)
  refine ⟨?_, ?_, ?_, h.source_safe⟩
  · intro y hy hyrange w hw
    obtain ⟨w', hw', hval⟩ := h.locals_rel y hy hyrange w hw
    refine ⟨w', by simpa [MoveState.writeLocal_frames, hframe'] using hw',
      hval⟩
  · intro y hyrange himm w hw
    simpa [MoveState.writeLocal_frames, hframe'] using
      h.plain_rel y hyrange himm w hw
  · intro y hy rt href
    rw [targetRead y hy rt href]
    exact h.refs_agree y hy rt href

/-- Update a local pair while establishing the required immutable view relation. -/
theorem ImmFrameRel.writeLocal_view {d : FunDecl}
    {before live : LiveSet} {g g' : BGraph} {frame : FrameId}
    {s s' : MoveState} (h : ImmFrameRel d before g frame s s')
    (hframe : frame = s.current) (hcurrent : s'.current = s.current)
    {i : Instr} {dst : LocalIndex} (hdef : dst ∈ instrDefs i)
    (hcheck : immCheck d g live i = .ok ())
    (hsurvive : ∀ y, y ∈ live → y ≠ dst → y ∈ before)
    (hgraph : ∀ e, e ∈ g → e ∈ g')
    {v v' : Value}
    (hval : if isImmLocal d dst then
        (v.refFree ∧ v' = v) ∨
          ∃ rt, v = .ref rt ∧ v'.refFree ∧ s.readTarget rt = some v'
      else v' = v)
    (hshapeNew : v.refFree ∨ ∃ rt, v = .ref rt)
    (hread : ∀ rt, v = .ref rt →
      s'.readTarget rt = s.readTarget rt)
    (hbelow : ∀ rt, v = .ref rt → ∀ rootFrame rootLocal,
      rt.root = .loc rootFrame rootLocal → rootFrame ≤ frame)
    (hplain : ∀ rt, v = .ref rt → ∀ rootLocal,
      rt.root = .loc frame rootLocal →
        rootLocal < d.numLocals ∧ isImmLocal d rootLocal = false)
    (hcover : ∀ rt, v = .ref rt → ImmCovers frame g' dst rt) :
    ImmFrameRel d live g' frame (s.writeLocal dst v)
      (s'.writeLocal dst v') := by
  have hframe' : frame = s'.current := by rw [hcurrent]; exact hframe
  have rootNe : isImmLocal d dst = true → ∀ rt, v = .ref rt →
      rt.root ≠ .loc frame dst := by
    intro himm rt href hroot
    exact Bool.noConfusion (himm.symm.trans (hplain rt href dst hroot).2)
  have sourceStable : isImmLocal d dst = true →
      ∀ rt, v = .ref rt →
      (s.writeLocal dst v).readTarget rt = s.readTarget rt := by
    intro himm rt href
    apply MoveState.readTarget_writeLocal_of_root_ne
    simpa [hframe] using rootNe himm rt href
  have targetStable : isImmLocal d dst = true →
      ∀ rt, v = .ref rt →
      (s'.writeLocal dst v').readTarget rt = s'.readTarget rt := by
    intro himm rt href
    apply MoveState.readTarget_writeLocal_of_root_ne
    simpa [hframe'] using rootNe himm rt href
  refine ⟨?_, ?_, ?_, h.source_safe.writeLocal hframe hsurvive
    (fun _ _ hcov => hcov.mono hgraph) hshapeNew hbelow hplain hcover⟩
  · intro y hy hyrange w hw
    by_cases hydst : y = dst
    · subst y
      have hwEq : v = w := by
        simpa [MoveState.writeLocal_frames, hframe] using hw
      subst w
      refine ⟨v', by simp [hframe'], ?_⟩
      by_cases himm : isImmLocal d dst = true
      · rw [himm] at hval ⊢
        rcases hval with hfree | ⟨rt, href, hvfree, htarget⟩
        · exact Or.inl hfree
        · exact Or.inr ⟨rt, href, hvfree,
            (sourceStable himm rt href).trans htarget⟩
      · simpa [himm] using hval
    · have hbefore := hsurvive y hy hydst
      have hw0 : s.frames frame y = some w := by
        simpa [MoveState.writeLocal_frames, hframe, hydst] using hw
      obtain ⟨w', hw', hwrel⟩ := h.locals_rel y hbefore hyrange w hw0
      refine ⟨w', ?_, ?_⟩
      · simpa [MoveState.writeLocal_frames, hframe', hydst] using hw'
      · by_cases himm : isImmLocal d y = true
        · rw [himm] at hwrel ⊢
          rcases hwrel with hfree | ⟨rt, rfl, hwfree, htarget⟩
          · exact Or.inl hfree
          · have hstable : (s.writeLocal dst v).readTarget rt =
                s.readTarget rt := by
              apply MoveState.readTarget_writeLocal_of_root_ne
              simpa [hframe] using
                (h.covers y hbefore rt hw0).root_ne_def
                  hcheck hy himm hdef
            exact Or.inr ⟨rt, rfl, hwfree, hstable.trans htarget⟩
        · simpa [himm] using hwrel
  · intro y hyrange himm w hw
    by_cases hydst : y = dst
    · subst y
      have hwEq : v = w := by
        simpa [MoveState.writeLocal_frames, hframe] using hw
      subst w
      have hvEq : v' = v := by simpa [himm] using hval
      subst v'
      simp [hframe']
    · have hw0 : s.frames frame y = some w := by
        simpa [MoveState.writeLocal_frames, hframe, hydst] using hw
      have hw' := h.plain_rel y hyrange himm w hw0
      simpa [MoveState.writeLocal_frames, hframe', hydst] using hw'
  · intro y hy rt href
    by_cases hydst : y = dst
    · subst y
      have hvref : v = .ref rt := by
        simpa [MoveState.writeLocal_frames, hframe] using href
      by_cases himm : isImmLocal d dst = true
      · rw [targetStable himm rt hvref, sourceStable himm rt hvref]
        exact hread rt hvref
      · have hvEq : v' = v := by simpa [himm] using hval
        subst v'
        exact MoveState.readTarget_writeLocal_eq hcurrent dst v rt
          (hread rt hvref)
    · have hbefore := hsurvive y hy hydst
      have href0 : s.frames frame y = some (.ref rt) := by
        simpa [MoveState.writeLocal_frames, hframe, hydst] using href
      by_cases himmDst : isImmLocal d dst = true
      · have hrootNe : rt.root ≠ .loc frame dst := by
          by_cases himmY : isImmLocal d y = true
          · exact (h.covers y hbefore rt href0).root_ne_def
              hcheck hy himmY hdef
          · intro hroot
            have hp := h.roots_plain y hbefore rt href0 dst hroot
            exact Bool.noConfusion (himmDst.symm.trans hp.2)
        rw [MoveState.readTarget_writeLocal_of_root_ne s dst v rt
              (by simpa [hframe] using hrootNe),
          MoveState.readTarget_writeLocal_of_root_ne s' dst v' rt
              (by simpa [hframe'] using hrootNe)]
        exact h.refs_agree y hbefore rt href0
      · have hvEq : v' = v := by simpa [himmDst] using hval
        subst v'
        exact MoveState.readTarget_writeLocal_eq hcurrent dst v rt
          (h.refs_agree y hbefore rt href0)

/-- Generic active-frame rule for an instruction that writes the same
reference-free value in source and target. -/
theorem ImmFrameRel.writeLocal_same_refFree {d : FunDecl}
    {before live : LiveSet} {g g' : BGraph} {frame : FrameId}
    {s s' : MoveState} (h : ImmFrameRel d before g frame s s')
    (hframe : frame = s.current) (hcurrent : s'.current = s.current)
    {i : Instr} {dst : LocalIndex} (hdef : dst ∈ instrDefs i)
    (hcheck : immCheck d g live i = .ok ())
    (hsurvive : ∀ y, y ∈ live → y ≠ dst → y ∈ before)
    (hgraph : ∀ e, e ∈ g → e ∈ g')
    {v : Value} (hfree : v.refFree) :
    ImmFrameRel d live g' frame (s.writeLocal dst v)
      (s'.writeLocal dst v) := by
  apply h.writeLocal_view hframe hcurrent hdef hcheck hsurvive hgraph
  · split
    · exact Or.inl ⟨hfree, rfl⟩
    · rfl
  · exact Or.inl hfree
  · intro rt href
    subst v
    simp at hfree
  · intro rt href
    subst v
    simp at hfree
  · intro rt href
    subst v
    simp at hfree
  · intro rt href
    subst v
    simp at hfree

/-- Batch form used by ordinary operations.  All destinations are updated at
once, so multi-result instructions do not require an artificial invariant
between two writes. -/
theorem ImmFrameRel.writeLocals_same_refFree {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {dsts srcs : List LocalIndex} {op : Oper} {rets : List Value}
    {s s' : MoveState} (h : ImmFrameRel d
      (liveThroughInstr (.call dsts op srcs) live) g frame s s')
    (hframe : frame = s.current) (hcurrent : s'.current = s.current)
    (hcheck : immCheck d g live (.call dsts op srcs) = .ok ())
    (hlen : dsts.length = rets.length)
    (hfree : ∀ v ∈ rets, v.refFree)
    (hsourceSafe : FrameSafe live
      (fun rootLocal =>
        rootLocal < d.numLocals ∧ isImmLocal d rootLocal = false)
      (ImmCovers frame g) frame (s.writeLocals dsts rets)) :
    ImmFrameRel d live g frame
      (s.writeLocals dsts rets) (s'.writeLocals dsts rets) := by
  have hframe' : frame = s'.current := by simpa [hcurrent] using hframe
  have sourceBefore : ∀ {y v},
      (s.writeLocals dsts rets).locals y = s.locals y →
      (s.writeLocals dsts rets).frames frame y = some v →
      s.frames frame y = some v := by
    intro y v hs hv
    have hv' : (s.writeLocals dsts rets).locals y = some v := by
      simpa [MoveState.locals, hframe] using hv
    rw [hs] at hv'
    simpa [MoveState.locals, hframe] using hv'
  have targetAfter : ∀ {y v},
      (s'.writeLocals dsts rets).locals y = s'.locals y →
      s'.frames frame y = some v →
      (s'.writeLocals dsts rets).frames frame y = some v := by
    intro y v hs hv
    have hv' : s'.locals y = some v := by
      simpa [MoveState.locals, hframe'] using hv
    rw [← hs] at hv'
    simpa [MoveState.locals, hframe'] using hv'
  have writtenSource : ∀ {y v w},
      (s.writeLocals dsts rets).locals y = some w →
      (s.writeLocals dsts rets).frames frame y = some v → v = w := by
    intro y v w hw hv
    have hv' : (s.writeLocals dsts rets).locals y = some v := by
      simpa [MoveState.locals, hframe] using hv
    rw [hw] at hv'
    exact (Option.some.inj hv').symm
  have sourceStable : ∀ y ∈ live, y ∉ dsts → ∀ rt,
      s.frames frame y = some (.ref rt) → isImmLocal d y = true →
      (s.writeLocals dsts rets).readTarget rt = s.readTarget rt := by
    intro y hy hnot rt href himm
    apply MoveState.readTarget_writeLocals_of_root_ne
    intro dst hdst
    have hbefore : y ∈
        liveThroughInstr (.call dsts op srcs) live :=
      live_liveThroughInstr hy (by simpa [instrDefs] using hnot)
    simpa [hframe] using
      (h.covers y hbefore rt href).root_ne_def hcheck hy himm
        (by simpa [instrDefs] using hdst)
  have agreeAfter : ∀ y ∈ live, ∀ rt,
      (s.writeLocals dsts rets).frames frame y = some (.ref rt) →
      (s'.writeLocals dsts rets).readTarget rt =
        (s.writeLocals dsts rets).readTarget rt := by
    intro y hy rt href
    rcases MoveState.writeLocals_free_lookup (s := s) (s' := s')
        hlen hfree y with ⟨hnot, hs, -⟩ | ⟨v, hvfree, hs, -⟩
    · have href0 := sourceBefore hs href
      have hbefore := live_liveThroughInstr
        (i := .call dsts op srcs) hy
        (by simpa [instrDefs] using hnot)
      exact MoveState.readTarget_writeLocals_eq hcurrent dsts rets rt
        (h.refs_agree y hbefore rt href0)
    · have heq := writtenSource hs href
      subst v
      simp at hvfree
  refine ⟨?_, ?_, ?_, hsourceSafe⟩
  · intro y hy hrange v hv
    rcases MoveState.writeLocals_free_lookup (s := s) (s' := s')
        hlen hfree y with ⟨hnot, hs, hs'⟩ | ⟨w, hwfree, hs, hs'⟩
    · have hv0 := sourceBefore hs hv
      have hbefore := live_liveThroughInstr
        (i := .call dsts op srcs) hy (by simpa [instrDefs] using hnot)
      obtain ⟨v', hv', hval⟩ := h.locals_rel y hbefore hrange v hv0
      refine ⟨v', targetAfter hs' hv', ?_⟩
      by_cases himm : isImmLocal d y = true
      · rw [himm] at hval ⊢
        rcases hval with hsame | ⟨rt, rfl, hvfree, hread⟩
        · exact Or.inl hsame
        · exact Or.inr ⟨rt, rfl, hvfree,
            (sourceStable y hy hnot rt hv0 himm).trans hread⟩
      · simpa [himm] using hval
    · have heq := writtenSource hs hv
      subst v
      refine ⟨w, by simpa [MoveState.locals, hframe'] using hs', ?_⟩
      split
      · exact Or.inl ⟨hwfree, rfl⟩
      · rfl
  · intro y hrange himm v hv
    rcases MoveState.writeLocals_free_lookup (s := s) (s' := s')
        hlen hfree y with ⟨hnot, hs, hs'⟩ | ⟨w, -, hs, hs'⟩
    · have hv0 := sourceBefore hs hv
      have hv' := h.plain_rel y hrange himm v hv0
      exact targetAfter hs' hv'
    · have : (s.writeLocals dsts rets).locals y = some v := by
        simpa [MoveState.locals, hframe] using hv
      rw [hs] at this
      cases this
      simpa [MoveState.locals, hframe'] using hs'
  · intro y hy rt href
    exact agreeAfter y hy rt href

/-- Generic rule for an eliminated immutable-reference constructor.  The
source writes a reference derived from live local `src`; the target writes
the reference-free value already read through that reference. -/
theorem ImmFrameRel.writeCopiedRef {d : FunDecl}
    {before live : LiveSet} {g g' : BGraph} {frame : FrameId}
    {s s' : MoveState} (h : ImmFrameRel d before g frame s s')
    (hframe : frame = s.current) (hcurrent : s'.current = s.current)
    {i : Instr} {dst src : LocalIndex} (hdef : dst ∈ instrDefs i)
    (hcheck : immCheck d g live i = .ok ())
    (hsurvive : ∀ y, y ∈ live → y ≠ dst → y ∈ before)
    (hgraph : ∀ e, e ∈ g → e ∈ g')
    (hsrcLive : src ∈ before) {rt : RefTarget} {v : Value}
    (hsrc : s.frames frame src = some (.ref rt))
    (himm : isImmLocal d dst = true)
    (hfree : v.refFree) (hvalue : s.readTarget rt = some v)
    (hcover : ImmCovers frame g' dst rt) :
    ImmFrameRel d live g' frame
      (s.writeLocal dst (.ref rt)) (s'.writeLocal dst v) := by
  apply h.writeLocal_view hframe hcurrent hdef hcheck hsurvive hgraph
  · simp [himm, hfree, hvalue]
  · exact Or.inr ⟨rt, rfl⟩
  · intro target href
    cases href
    exact h.refs_agree src hsrcLive rt hsrc
  · intro target href rootFrame rootLocal hroot
    cases href
    exact h.roots_below src hsrcLive rt hsrc rootFrame rootLocal hroot
  · intro target href rootLocal hroot
    cases href
    exact h.roots_plain src hsrcLive rt hsrc rootLocal hroot
  · intro target href
    cases href
    exact hcover

/-- Shared state rule for field and vector-element borrows.  Both retain the
parent root and add one path component; only their target instruction paths
differ. -/
theorem ImmFrameRel.writeDerivedRef {d : FunDecl}
    {before live : LiveSet} {g g' : BGraph} {frame : FrameId}
    {s s' : MoveState} (h : ImmFrameRel d before g frame s s')
    (hframe : frame = s.current) (hcurrent : s'.current = s.current)
    {i : Instr} {dst src : LocalIndex} (hdef : dst ∈ instrDefs i)
    (hcheck : immCheck d g live i = .ok ())
    (hsurvive : ∀ y, y ∈ live → y ≠ dst → y ∈ before)
    (hgraph : ∀ e, e ∈ g → e ∈ g')
    (hsrcLive : src ∈ before) {parent child : RefTarget} {v : Value}
    (hsrc : s.frames frame src = some (.ref parent))
    (hroot : child.root = parent.root)
    (hread : s'.readTarget child = s.readTarget child)
    (hvalue : s.readTarget child = some v)
    (hfree : isImmLocal d dst = true → v.refFree)
    (hcover : ImmCovers frame g' dst child) :
    ImmFrameRel d live g' frame
      (s.writeLocal dst (.ref child))
      (s'.writeLocal dst
        (if isImmLocal d dst then v else .ref child)) := by
  apply h.writeLocal_view hframe hcurrent hdef hcheck hsurvive hgraph
  · by_cases himm : isImmLocal d dst = true
    · simp [himm, hfree himm, hvalue]
    · have hplainDst : isImmLocal d dst = false := by
        cases heq : isImmLocal d dst
        · rfl
        · exact absurd heq himm
      simp [hplainDst]
  · exact Or.inr ⟨child, rfl⟩
  · intro target href
    cases href
    exact hread
  · intro target href rootFrame rootLocal hroot'
    cases href
    exact h.roots_below src hsrcLive parent hsrc rootFrame rootLocal
      (hroot ▸ hroot')
  · intro target href rootLocal hroot'
    cases href
    exact h.roots_plain src hsrcLive parent hsrc rootLocal
      (hroot ▸ hroot')
  · intro target href
    cases href
    exact hcover

/-- Assignment is the canonical related-write instance: the destination
inherits both the source's dynamic view and its static derivation roots. -/
theorem ImmFrameRel.assign {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {s s' : MoveState}
    {dst src : LocalIndex} {v : Value}
    (h : ImmFrameRel d (liveThroughInstr (.assign dst src) live)
      g frame s s')
    (hframe : frame = s.current) (hcurrent : s'.current = s.current)
    (hrange : (instrDefs (.assign dst src) ++
      instrUses (.assign dst src)).all (· < d.numLocals) = true)
    (hcheck : immCheck d g live (.assign dst src) = .ok ())
    (hkinds : isImmLocal d dst = isImmLocal d src)
    (hsrc : s.locals src = some v) :
    ∃ v', s'.locals src = some v' ∧
      ImmFrameRel d live (immStep d g (.assign dst src)) frame
        (s.writeLocal dst v) (s'.writeLocal dst v') := by
  have hsrcLive : src ∈ liveThroughInstr (.assign dst src) live :=
    uses_mem_liveThroughInstr (by simp [instrUses])
  have hsrcRange : src < d.numLocals := by
    have hall := List.all_eq_true.mp hrange
    exact of_decide_eq_true (hall src (by simp [instrDefs, instrUses]))
  have hsrcFrame : s.frames frame src = some v := by
    simpa [MoveState.locals, hframe] using hsrc
  obtain ⟨v', hv', hval⟩ :=
    h.locals_rel src hsrcLive hsrcRange v hsrcFrame
  have hframe' : frame = s'.current := by simpa [hcurrent] using hframe
  refine ⟨v', by simpa [MoveState.locals, hframe'] using hv', ?_⟩
  apply h.writeLocal_view hframe hcurrent (i := .assign dst src)
    (dst := dst) (by simp [instrDefs]) hcheck
  · intro y hy hydst
    exact live_liveThroughInstr hy (by simp [instrDefs, hydst])
  · intro e he
    simp only [immStep]
    split
    · unfold gInsertClosed
      exact mem_foldl_gInsert (mem_gInsert he)
    · exact he
  · simpa [hkinds] using hval
  · exact h.shape src hsrcLive v hsrcFrame
  · intro rt href
    exact h.refs_agree src hsrcLive rt (href ▸ hsrcFrame)
  · intro rt href rootFrame rootLocal hroot
    exact h.roots_below src hsrcLive rt (href ▸ hsrcFrame)
      rootFrame rootLocal hroot
  · intro rt href rootLocal hroot
    exact h.roots_plain src hsrcLive rt (href ▸ hsrcFrame)
      rootLocal hroot
  · intro rt href
    have hcov := h.covers src hsrcLive rt (href ▸ hsrcFrame)
    simpa [immStep, hcov.not_empty] using hcov.copy

/-- A local borrow is retained for an ordinary destination and implemented by
copying the borrowed value for an immutable destination.  The conditional
reference-free premise is the single dynamic typing fact this rule needs. -/
theorem ImmFrameRel.borrowLoc {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {s s' : MoveState}
    {dst x : LocalIndex} {v : Value}
    (h : ImmFrameRel d
      (liveThroughInstr (.call [dst] .borrowLoc [x]) live) g frame s s')
    (hframe : frame = s.current) (hcurrent : s'.current = s.current)
    (hrange : (instrDefs (.call [dst] .borrowLoc [x]) ++
      instrUses (.call [dst] .borrowLoc [x])).all (· < d.numLocals) = true)
    (hcheck : immCheck d g live (.call [dst] .borrowLoc [x]) = .ok ())
    (hxPlain : isImmLocal d x = false)
    (hfree : isImmLocal d dst = true → v.refFree)
    (hx : s.locals x = some v) :
    s'.locals x = some v ∧
      ImmFrameRel d live
        (immStep d g (.call [dst] .borrowLoc [x])) frame
        (s.writeLocal dst (.ref ⟨.loc s.current x, []⟩))
        (s'.writeLocal dst
          (if isImmLocal d dst then v
           else .ref ⟨.loc s.current x, []⟩)) := by
  have hxLive : x ∈ liveThroughInstr (.call [dst] .borrowLoc [x]) live :=
    uses_mem_liveThroughInstr (by simp [instrUses])
  have hxRange : x < d.numLocals := by
    exact of_decide_eq_true (List.all_eq_true.mp hrange x
      (by simp [instrDefs, instrUses]))
  have hxFrame : s.frames frame x = some v := by
    simpa [MoveState.locals, hframe] using hx
  have hxFrame' := h.plain_rel x hxRange hxPlain v hxFrame
  have hframe' : frame = s'.current := by simpa [hcurrent] using hframe
  have hx' : s'.locals x = some v := by
    simpa [MoveState.locals, hframe'] using hxFrame'
  refine ⟨hx', ?_⟩
  let rt : RefTarget := ⟨.loc frame x, []⟩
  have hrt : (RefTarget.mk (.loc s.current x) []) = rt := by
    simp [rt, hframe]
  rw [hrt]
  apply h.writeLocal_view hframe hcurrent
    (i := .call [dst] .borrowLoc [x]) (dst := dst)
    (by simp [instrDefs]) hcheck
  · intro y hy hydst
    exact live_liveThroughInstr hy (by simp [instrDefs, hydst])
  · intro e he
    simpa [immStep] using (mem_gInsertClosed (e' :=
      ⟨.localRoot x, dst, []⟩) he)
  · by_cases himm : isImmLocal d dst = true
    · simp only [himm, ↓reduceIte]
      exact Or.inr ⟨rt, rfl, hfree himm, by
        simp [rt, MoveState.readTarget, Value.getPath, hxFrame]⟩
    · have hplainDst : isImmLocal d dst = false := by
        cases heq : isImmLocal d dst
        · rfl
        · exact absurd heq himm
      simp [hplainDst]
  · exact Or.inr ⟨rt, rfl⟩
  · intro target href
    cases href
    simp [rt, MoveState.readTarget, hxFrame, hxFrame']
  · intro target href rootFrame rootLocal hroot
    cases href
    simp [rt] at hroot
    exact hroot.1 ▸ Nat.le_refl frame
  · intro target href rootLocal hroot
    cases href
    simp [rt] at hroot
    exact ⟨by simpa [← hroot] using hxRange,
      by simpa [← hroot] using hxPlain⟩
  · intro target href
    cases href
    simpa [rt, ImmCovers, immStep] using
      (immAncestors_borrowLoc (g := g) (x := x) (dst := dst))

/-- Extend an immutable frame relation after a global borrow. -/
theorem ImmFrameRel.borrowGlobal {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {s s' : MoveState}
    {dst t : LocalIndex} {r : ResourceId} {a : Address} {v : Value}
    (h : ImmFrameRel d
      (liveThroughInstr (.call [dst] (.borrowGlobal r) [t]) live)
      g frame s s')
    (hframe : frame = s.current) (hcurrent : s'.current = s.current)
    (hmemory : s'.memory = s.memory)
    (hcheck : immCheck d g live
      (.call [dst] (.borrowGlobal r) [t]) = .ok ())
    (hfree : v.refFree) (hpresent : s.memory r a = some v) :
    ImmFrameRel d live
      (immStep d g (.call [dst] (.borrowGlobal r) [t])) frame
      (s.writeLocal dst (.ref ⟨.global r a, []⟩))
      (s'.writeLocal dst
        (if isImmLocal d dst then v else .ref ⟨.global r a, []⟩)) := by
  let rt : RefTarget := ⟨.global r a, []⟩
  apply h.writeLocal_view hframe hcurrent
    (i := .call [dst] (.borrowGlobal r) [t]) (dst := dst)
    (by simp [instrDefs]) hcheck
  · intro y hy hydst
    exact live_liveThroughInstr hy (by simp [instrDefs, hydst])
  · intro e he
    simpa [immStep] using (mem_gInsertClosed (e' :=
      ⟨.globalRoot r, dst, []⟩) he)
  · by_cases himm : isImmLocal d dst = true
    · simp only [himm, ↓reduceIte]
      exact Or.inr ⟨rt, rfl, hfree, by
        simp [rt, MoveState.readTarget, Value.getPath, hpresent]⟩
    · have hplainDst : isImmLocal d dst = false := by
        cases heq : isImmLocal d dst
        · rfl
        · exact absurd heq himm
      simp [hplainDst]
  · exact Or.inr ⟨rt, rfl⟩
  · intro target href
    cases href
    simp [MoveState.readTarget, hmemory]
  · intro target href rootFrame rootLocal hroot
    cases href
    simp at hroot
  · intro target href rootLocal hroot
    cases href
    simp at hroot
  · intro target href
    cases href
    exact Or.inl (by simpa [immStep] using
      (immAncestors_borrowGlobal (g := g) (r := r) (dst := dst)))

/-- Extend an immutable frame relation after loading a reference-free value. -/
theorem ImmFrameRel.load {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {s s' : MoveState}
    {dst : LocalIndex} {v : Value}
    (h : ImmFrameRel d (liveThroughInstr (.load dst v) live) g frame s s')
    (hframe : frame = s.current) (hcurrent : s'.current = s.current)
    (hcheck : immCheck d g live (.load dst v) = .ok ())
    (hfree : v.refFree) :
    ImmFrameRel d live (immStep d g (.load dst v)) frame
      (s.writeLocal dst v) (s'.writeLocal dst v) := by
  apply h.writeLocal_same_refFree hframe hcurrent
    (i := .load dst v) (dst := dst) (by simp [instrDefs]) hcheck
  · intro y hy hydst
    apply live_liveThroughInstr hy
    simp [instrDefs, hydst]
  · intro e he
    simpa [immStep] using he
  · exact hfree

/-- The static information carried for one active or suspended frame. -/
structure ImmPoint where
  frame : FrameId
  decl : FunDecl
  live : LiveSet
  graph : BGraph

/-- State relation carried by one immutable-simulation program point. -/
def ImmPoint.Rel (p : ImmPoint) (s s' : MoveState) : Prop :=
  ImmFrameRel p.decl p.live p.graph p.frame s s'

/-- The generic stack relation instantiated with immutable-pass points. -/
abbrev ImmStackRel (points : List ImmPoint) (s s' : MoveState) : Prop :=
  FrameStackRel ImmPoint.frame ImmPoint.Rel points s s'

/-- The semantic part of borrow correctness needed by a memory-changing
instruction: every live immutable borrow in every active or suspended frame
continues to denote the same value. -/
def ImmMemorySafe (points : List ImmPoint) (s : MoveState)
    (m' : Memory) : Prop :=
  ∀ p ∈ points, ∀ x ∈ p.live, ∀ rt,
    s.frames p.frame x = some (.ref rt) → isImmLocal p.decl x = true →
    (s.setMemory m').readTarget rt = s.readTarget rt

/-- Borrow stability for a write through a reference rooted in a local of
any active or suspended frame. -/
def ImmLocalWriteSafe (points : List ImmPoint) (s : MoveState)
    (rootFrame : FrameId) (rootLocal : LocalIndex) (w : Value) : Prop :=
  ∀ p ∈ points, ∀ x ∈ p.live, ∀ rt,
    s.frames p.frame x = some (.ref rt) → isImmLocal p.decl x = true →
    (s.writeFrameLocal rootFrame rootLocal w).readTarget rt =
      s.readTarget rt

/-- Type- and borrow-checker facts for an ordinary successful operation. -/
structure ImmOpSafe (points : List ImmPoint) (s : MoveState)
    (op : Oper) (vs rets : List Value) (m' : Memory) : Prop where
  operands : ImmOperandsSafe op vs
  stable : ImmMemorySafe points s m'
  results : ∀ v ∈ rets, v.refFree

/-- The type- and borrow-checker facts needed by a successful `writeRef`.
They are stated at the root update exposed by `writeTarget`, so the semantic
proof does not reproduce either frontend analysis. -/
structure ImmWriteSafe (points : List ImmPoint) (s s' : MoveState)
    (rt : RefTarget) (v : Value) : Prop where
  localRoot : ∀ frame x path root root',
    rt = ⟨.loc frame x, path⟩ → s.frames frame x = some root →
    root.setPath path v = some root' →
    s'.frames frame x = some root ∧ root'.refFree ∧
      ImmLocalWriteSafe points s frame x root'
  globalRoot : ∀ r a path root root',
    rt = ⟨.global r a, path⟩ → s.memory r a = some root →
    root.setPath path v = some root' →
    ImmMemorySafe points s (memWrite s.memory r a root')

/-- The type- and borrow-checker facts consumed by the immutable semantic
simulation.  This certificate deliberately contains only the three dynamic
projections that are not consequences of the operational rules themselves:
stability across an ordinary operation, safety of a write through a mutable
reference, and validity of a callee's newly entered frame. -/
structure ImmCheckedFacts (P : Program) : Prop where
  op {d : FunDecl} {live : LiveSet} {g : BGraph} {frame : FrameId}
      {points : List ImmPoint} {s s' : MoveState}
      {dsts srcs : List LocalIndex} {oper : Oper}
      {vs rets : List Value} {m' : Memory} :
    ImmStackRel
        (⟨frame, d, liveThroughInstr (.call dsts oper srcs) live, g⟩ ::
          points) s s' →
    frame = s.current →
    immCheck d g live (.call dsts oper srcs) = .ok () →
    oper.sem s.current s.readTarget vs s.memory = some (.ok rets m') →
    ImmOpSafe
        (⟨frame, d, liveThroughInstr (.call dsts oper srcs) live, g⟩ ::
          points) s oper vs rets m' ∧
      FrameSafe live
        (fun rootLocal =>
          rootLocal < d.numLocals ∧ isImmLocal d rootLocal = false)
        (ImmCovers frame g) frame
        ((s.setMemory m').writeLocals dsts rets)
  writeRef {d : FunDecl} {live : LiveSet} {g : BGraph}
      {frame : FrameId} {points : List ImmPoint} {s s' sNext : MoveState}
      {t vt : LocalIndex} {rt : RefTarget} {v : Value} :
    ImmStackRel
        (⟨frame, d, liveThroughInstr (.call [] .writeRef [t, vt]) live,
          g⟩ :: points) s s' →
    frame = s.current →
    immCheck d g live (.call [] .writeRef [t, vt]) = .ok () →
    s.writeTarget rt v = some sNext →
    ImmWriteSafe
      (⟨frame, d, liveThroughInstr (.call [] .writeRef [t, vt]) live,
        g⟩ :: points) s s' rt v
  callEntry {caller callee : FunDecl} {f : FunId}
      {callerLive live : LiveSet} {callerGraph graph : BGraph}
      {frame : FrameId} {points : List ImmPoint} {s s' : MoveState}
      {dsts srcs : List LocalIndex} {args : List Value} :
    ImmStackRel
        (⟨frame, caller, callerLive, callerGraph⟩ :: points) s s' →
    frame = s.current →
    immBoundaryInstr P.funs caller
        (.call dsts (.function f) srcs) = true →
    P.funs f = some callee →
    srcs.mapM s.locals = some args →
    FrameSafe live
      (fun rootLocal =>
        rootLocal < callee.numLocals ∧ isImmLocal callee rootLocal = false)
      (ImmCovers (s.current + 1) graph) (s.current + 1)
      (s.enterCall args)
  callReturn {caller : FunDecl} {live : LiveSet} {graph : BGraph}
      {frame : FrameId} {points : List ImmPoint} {s s' : MoveState}
      {dsts srcs : List LocalIndex} {f : FunId} {vals : List Value} :
    ImmStackRel
        (⟨frame, caller,
          liveThroughInstr (.call dsts (.function f) srcs) live, graph⟩ ::
          points) s s' →
    frame = s.current →
    immCheck caller graph live (.call dsts (.function f) srcs) = .ok () →
    CheckedState P caller (s.writeLocals dsts vals) →
    ImmFrameRel caller live
      (immStep caller graph (.call dsts (.function f) srcs)) frame
      (s.writeLocals dsts vals) (s'.writeLocals dsts vals)

/-- Replacing both memories equally preserves an immutable stack relation. -/
theorem ImmStackRel.setMemory {points : List ImmPoint} {s s' : MoveState}
    (h : ImmStackRel points s s') (m' : Memory)
    (hsafe : ImmMemorySafe points s m') :
    ImmStackRel points (s.setMemory m') (s'.setMemory m') := by
  refine ⟨h.current_eq, rfl, ?_, ?_⟩
  · intro p hp
    exact (h.tracked p hp).setMemory m'
      (fun x hx rt href himm => hsafe p hp x hx rt href himm)
  · intro frame hnone
    simpa using h.untracked frame hnone

/-- Equal frame-local writes preserve an immutable stack relation. -/
theorem ImmStackRel.writeFrameLocal_same {points : List ImmPoint}
    {s s' : MoveState} (h : ImmStackRel points s s')
    (rootFrame : FrameId) (rootLocal : LocalIndex) (w : Value)
    (hfree : w.refFree)
    (hsafe : ImmLocalWriteSafe points s rootFrame rootLocal w) :
    ImmStackRel points (s.writeFrameLocal rootFrame rootLocal w)
      (s'.writeFrameLocal rootFrame rootLocal w) := by
  refine ⟨by simpa using h.current_eq, by simpa using h.memory_eq,
    ?_, ?_⟩
  · intro p hp
    exact (h.tracked p hp).writeFrameLocal_same rootLocal w hfree
      (fun x hx rt href himm => hsafe p hp x hx rt href himm)
  · intro frame hnone
    by_cases hf : frame = rootFrame
    · subst rootFrame
      simp [MoveState.writeFrameLocal_frames, h.untracked frame hnone]
    · simp [MoveState.writeFrameLocal_frames, hf, h.untracked frame hnone]

/-- Push a callee relation over all suspended caller relations.  Establishing
the callee's parameter relation is deliberately a separate call-boundary
obligation; this theorem handles only frame-stack plumbing. -/
theorem ImmStackRel.enterCall {points : List ImmPoint} {s s' : MoveState}
    (h : ImmStackRel points s s')
    (hbelow : ∀ p ∈ points, p.frame ≤ s.current)
    (args args' : List Value) {child : ImmPoint}
    (hchild : child.frame = s.current + 1)
    (hactive : child.Rel (s.enterCall args) (s'.enterCall args')) :
    ImmStackRel (child :: points) (s.enterCall args) (s'.enterCall args') := by
  refine ⟨by simp [h.current_eq], by simpa using h.memory_eq,
    ?_, ?_⟩
  · intro p hp
    rcases List.mem_cons.mp hp with rfl | hp
    · exact hactive
    · exact (h.tracked p hp).enterCall_below (hbelow p hp)
        h.current_eq args args'
  · intro frame hnone
    have hne : frame ≠ s.current + 1 := by
      intro heq
      exact hnone child (by simp) (by simpa [hchild] using heq.symm)
    have hne' : frame ≠ s'.current + 1 := by
      simpa [h.current_eq] using hne
    simp [MoveState.enterCall, setFrame, hne, hne']
    apply h.untracked frame
    intro p hp
    exact hnone p (by simp [hp])

/-- Look up the target view corresponding to a live source local. -/
theorem ImmStackRel.lookup {p : ImmPoint} {points : List ImmPoint}
    {s s' : MoveState} (h : ImmStackRel (p :: points) s s')
    (hp : p.frame = s.current) {x : LocalIndex} (hlive : x ∈ p.live)
    (hlt : x < p.decl.numLocals) {v : Value}
    (hv : s.locals x = some v) :
    ∃ v', s'.locals x = some v' ∧
      if isImmLocal p.decl x then
        (v.refFree ∧ v' = v) ∨
          ∃ rt, v = .ref rt ∧ v'.refFree ∧ s.readTarget rt = some v'
      else v' = v := by
  have hvFrame : s.frames p.frame x = some v := by
    simpa [MoveState.locals, hp] using hv
  obtain ⟨v', hv', hval⟩ := h.head.locals_rel x hlive hlt v hvFrame
  refine ⟨v', ?_, hval⟩
  simpa [MoveState.locals, hp, h.current_eq] using hv'

/-- A live ordinary local has equal source and target values. -/
theorem ImmStackRel.lookup_eq {p : ImmPoint} {points : List ImmPoint}
    {s s' : MoveState} (h : ImmStackRel (p :: points) s s')
    (hp : p.frame = s.current) {x : LocalIndex} (hlive : x ∈ p.live)
    (hlt : x < p.decl.numLocals) (hnimm : isImmLocal p.decl x = false)
    {v : Value} (hv : s.locals x = some v) : s'.locals x = some v := by
  obtain ⟨v', hv', hval⟩ := h.lookup hp hlive hlt hv
  rw [hnimm] at hval
  subst v'
  exact hv'

/-- A live local with no immutable ancestors has equal source and target values. -/
theorem ImmStackRel.lookup_eq_of_imm_empty
    {p : ImmPoint} {points : List ImmPoint} {s s' : MoveState}
    (h : ImmStackRel (p :: points) s s') (hp : p.frame = s.current)
    {x : LocalIndex} (hlive : x ∈ p.live)
    (hlt : x < p.decl.numLocals)
    (hempty : isImmLocal p.decl x = true →
      (immAncestors p.graph x).isEmpty = true)
    {v : Value} (hv : s.locals x = some v) : s'.locals x = some v := by
  obtain ⟨v', hv', hval⟩ := h.lookup hp hlive hlt hv
  by_cases himm : isImmLocal p.decl x = true
  · rw [himm] at hval
    rcases hval with ⟨-, rfl⟩ | ⟨rt, rfl, -, -⟩
    · exact hv'
    · have hcover := h.head.covers x hlive rt
          (by simpa [MoveState.locals, hp] using hv)
      have hne := ImmCovers.not_empty hcover
      rw [hempty himm] at hne
      cases hne
  · have himmFalse : isImmLocal p.decl x = false := by
      cases heq : isImmLocal p.decl x
      · rfl
      · exact absurd heq himm
    rw [himmFalse] at hval
    subst v'
    exact hv'

/-- A live reference-free local has equal source and target values. -/
theorem ImmStackRel.lookup_eq_of_refFree
    {p : ImmPoint} {points : List ImmPoint} {s s' : MoveState}
    (h : ImmStackRel (p :: points) s s') (hp : p.frame = s.current)
    {x : LocalIndex} (hlive : x ∈ p.live)
    (hlt : x < p.decl.numLocals) {v : Value} (hfree : v.refFree)
    (hv : s.locals x = some v) : s'.locals x = some v := by
  obtain ⟨v', hv', hval⟩ := h.lookup hp hlive hlt hv
  by_cases himm : isImmLocal p.decl x = true
  · rw [himm] at hval
    rcases hval with ⟨-, rfl⟩ | ⟨rt, href, -⟩
    · exact hv'
    · subst v
      simp at hfree
  · have himmFalse : isImmLocal p.decl x = false := by
      cases heq : isImmLocal p.decl x
      · rfl
      · exact absurd heq himm
    rw [himmFalse] at hval
    subst v'
    exact hv'

/-- Look up the immutable-view relation for one live local. -/
theorem ImmStackRel.lookup_view {p : ImmPoint} {points : List ImmPoint}
    {s s' : MoveState} (h : ImmStackRel (p :: points) s s')
    (hp : p.frame = s.current) {x : LocalIndex} (hlive : x ∈ p.live)
    (hlt : x < p.decl.numLocals) {v : Value}
    (hv : s.locals x = some v) :
    ∃ v', s'.locals x = some v' ∧ ImmViewRel s s' v v' := by
  obtain ⟨v', hv', hval⟩ := h.lookup hp hlive hlt hv
  refine ⟨v', hv', ?_⟩
  cases h.head.shape x hlive v
      (by simpa [MoveState.locals, hp] using hv) with
  | inl hfree =>
      by_cases himm : isImmLocal p.decl x = true
      · rw [himm] at hval
        rcases hval with ⟨-, rfl⟩ | ⟨rt, hvref, -⟩
        · exact .free hfree
        · subst v
          simp at hfree
      · simp only [himm, Bool.false_eq_true, ↓reduceIte] at hval
        subst v'
        exact .free hfree
  | inr href =>
      obtain ⟨rt, rfl⟩ := href
      by_cases himm : isImmLocal p.decl x = true
      · rw [himm] at hval
        rcases hval with ⟨hfree, -⟩ |
            ⟨rt', href, hfree, hread⟩
        · simp at hfree
        · cases href
          exact .copied hfree hread
      · simp only [himm, Bool.false_eq_true, ↓reduceIte] at hval
        subst v'
        apply ImmViewRel.kept
        have hagree := h.head.refs_agree x hlive rt
          (by simpa [MoveState.locals, hp] using hv)
        exact hagree

/-- Lift live-local lookup to immutable-view related operand lists. -/
theorem ImmStackRel.lookup_views {p : ImmPoint} {points : List ImmPoint}
    {s s' : MoveState} (h : ImmStackRel (p :: points) s s')
    (hp : p.frame = s.current) {xs : List LocalIndex} {vs : List Value}
    (hlive : ∀ x ∈ xs, x ∈ p.live)
    (hrange : ∀ x ∈ xs, x < p.decl.numLocals)
    (hvals : xs.mapM s.locals = some vs) :
    ∃ vs', xs.mapM s'.locals = some vs' ∧ ImmViews s s' vs vs' := by
  induction xs generalizing vs with
  | nil =>
      have hvs : [] = vs := by
        simpa only [List.mapM_nil, pure, Option.pure_def,
          Option.some.injEq] using hvals
      subst vs
      exact ⟨[], rfl, .nil⟩
  | cons x xs ih =>
      rw [mapM_cons_eq_some] at hvals
      obtain ⟨v, rest, hv, hrest, rfl⟩ := hvals
      obtain ⟨v', hv', hview⟩ := h.lookup_view hp
        (hlive x (by simp)) (hrange x (by simp)) hv
      obtain ⟨rest', hrest', hviews⟩ := ih
        (fun y hy => hlive y (by simp [hy]))
        (fun y hy => hrange y (by simp [hy])) hrest
      refine ⟨v' :: rest', ?_, .cons hview hviews⟩
      rw [List.mapM_cons, hv', hrest']
      rfl

/-- Establish a callee entry relation from the caller's operand relation and
the immutable pass's checked parameter-kind agreement.  Source validity at
the new program point is supplied by the frontend execution certificate. -/
theorem ImmStackRel.callEntry {sigs : FunId → Option FunDecl}
    {caller callee : FunDecl} {f : FunId}
    {callerLive live : LiveSet} {callerGraph graph : BGraph}
    {frame : FrameId} {points : List ImmPoint} {s s' : MoveState}
    {dsts srcs : List LocalIndex} {args : List Value}
    (h : ImmStackRel (⟨frame, caller, callerLive, callerGraph⟩ :: points)
      s s')
    (hframe : frame = s.current)
    (hbelow : ∀ p ∈ points, p.frame < s.current)
    (srcLive : ∀ x ∈ srcs, x ∈ callerLive)
    (hrange : (instrDefs (.call dsts (.function f) srcs) ++
      instrUses (.call dsts (.function f) srcs)).all
        (· < caller.numLocals) = true)
    (hboundary : immBoundaryInstr sigs caller
      (.call dsts (.function f) srcs) = true)
    (hcallee : sigs f = some callee)
    (hargs : srcs.mapM s.locals = some args)
    (hcalleeSafe : FrameSafe live
      (fun rootLocal => rootLocal < callee.numLocals ∧
        isImmLocal callee rootLocal = false)
      (ImmCovers (s.current + 1) graph) (s.current + 1)
      (s.enterCall args)) :
    ∃ args', srcs.mapM s'.locals = some args' ∧
      args'.length = args.length ∧
      ImmStackRel
        (⟨s.current + 1, callee, live, graph⟩ ::
          ⟨frame, caller, callerLive, callerGraph⟩ :: points)
        (s.enterCall args) (s'.enterCall args') := by
  have srcRange : ∀ x ∈ srcs, x < caller.numLocals := by
    intro x hx
    exact of_decide_eq_true (List.all_eq_true.mp hrange x (by
      simp only [instrDefs, instrUses, List.mem_append]
      exact Or.inr hx))
  obtain ⟨args', hargs', hviews⟩ :=
    h.lookup_views hframe srcLive srcRange hargs
  have sourceArg : ∀ x v,
      (s.enterCall args).frames (s.current + 1) x = some v →
      ∃ src, srcs[x]? = some src ∧ s.locals src = some v := by
    intro x v hv
    have harg : args[x]? = some v := by
      simpa [MoveState.enterCall, setFrame, initLocals] using hv
    exact getElem?_of_mapM hargs harg
  have targetArg : ∀ x src v,
      srcs[x]? = some src → s'.locals src = some v →
      (s'.enterCall args').frames (s.current + 1) x = some v := by
    intro x src v hsrc hv
    obtain ⟨v', harg, hv'⟩ := mapM_getElem? hargs' hsrc
    rw [hv] at hv'
    cases hv'
    simpa [MoveState.enterCall, setFrame, initLocals, h.current_eq] using harg
  have hactive : ImmFrameRel callee live graph (s.current + 1)
      (s.enterCall args) (s'.enterCall args') := by
    refine ⟨?_, ?_, ?_, hcalleeSafe⟩
    · intro x hx xrange v hv
      obtain ⟨src, hsrc, hlocal⟩ := sourceArg x v hv
      have hsrcMem : src ∈ srcs := List.mem_of_getElem? hsrc
      obtain ⟨v', hv', hval⟩ := h.lookup hframe
        (srcLive src hsrcMem) (srcRange src hsrcMem) hlocal
      refine ⟨v', targetArg x src v' hsrc hv', ?_⟩
      have hkinds := (immBoundaryInstr_function_param hcallee hboundary hsrc).1
      rw [← hkinds]
      by_cases himm : isImmLocal caller src = true
      · rw [himm] at hval ⊢
        rcases hval with hval | ⟨rt, rfl, hvfree, hread⟩
        · exact Or.inl hval
        · refine Or.inr ⟨rt, rfl, hvfree, ?_⟩
          rw [MoveState.readTarget_enterCall_of_root_le]
          · exact hread
          · intro rootFrame rootLocal hroot
            exact Nat.le_trans (h.head.roots_below src
              (srcLive src hsrcMem) rt
              (by simpa [MoveState.locals, hframe] using hlocal)
              rootFrame rootLocal hroot) (by simp [hframe])
      · simpa [himm] using hval
    · intro x xrange himm v hv
      obtain ⟨src, hsrc, hlocal⟩ := sourceArg x v hv
      have hsrcMem : src ∈ srcs := List.mem_of_getElem? hsrc
      have hkinds := (immBoundaryInstr_function_param hcallee hboundary hsrc).1
      have hsimm : isImmLocal caller src = false := by
        rw [hkinds]
        exact himm
      exact targetArg x src v hsrc
        (h.lookup_eq hframe (srcLive src hsrcMem) (srcRange src hsrcMem)
          hsimm hlocal)
    · intro x hx rt href
      obtain ⟨src, hsrc, hlocal⟩ := sourceArg x (.ref rt) href
      have hsrcMem : src ∈ srcs := List.mem_of_getElem? hsrc
      have hold := h.head.refs_agree src (srcLive src hsrcMem) rt
        (by simpa [MoveState.locals, hframe] using hlocal)
      rw [MoveState.readTarget_enterCall_of_root_le s' args',
        MoveState.readTarget_enterCall_of_root_le s args]
      · exact hold
      · intro rootFrame rootLocal hroot
        have hle := h.head.roots_below src
          (srcLive src hsrcMem) rt
          (by simpa [MoveState.locals, hframe] using hlocal)
          rootFrame rootLocal hroot
        exact Nat.le_trans hle (Nat.le_of_eq hframe)
      · intro rootFrame rootLocal hroot
        have hle := h.head.roots_below src (srcLive src hsrcMem) rt
          (by simpa [MoveState.locals, hframe] using hlocal)
          rootFrame rootLocal hroot
        exact Nat.le_trans hle
          (Nat.le_of_eq (hframe.trans h.current_eq.symm))
  refine ⟨args', hargs', hviews.length, h.enterCall ?_ args args' rfl hactive⟩
  intro p hp
  rcases List.mem_cons.mp hp with rfl | hp
  · simp [hframe]
  · exact Nat.le_of_lt (hbelow p hp)


/-- A list of live locals without immutable ancestors has equal lookups. -/
theorem ImmStackRel.lookup_eqs_of_imm_empty
    {p : ImmPoint} {points : List ImmPoint} {s s' : MoveState}
    (h : ImmStackRel (p :: points) s s') (hp : p.frame = s.current)
    {xs : List LocalIndex} {vs : List Value}
    (hlive : ∀ x ∈ xs, x ∈ p.live)
    (hrange : ∀ x ∈ xs, x < p.decl.numLocals)
    (hempty : ∀ x ∈ xs, isImmLocal p.decl x = true →
      (immAncestors p.graph x).isEmpty = true)
    (hvals : xs.mapM s.locals = some vs) :
    xs.mapM s'.locals = some vs := by
  induction xs generalizing vs with
  | nil => simpa using hvals
  | cons x xs ih =>
      rw [mapM_cons_eq_some] at hvals
      obtain ⟨v, rest, hv, hrest, rfl⟩ := hvals
      have hv' := h.lookup_eq_of_imm_empty hp
        (hlive x (by simp)) (hrange x (by simp))
        (hempty x (by simp)) hv
      rw [List.mapM_cons, hv', ih
        (fun y hy => hlive y (by simp [hy]))
        (fun y hy => hrange y (by simp [hy]))
        (fun y hy => hempty y (by simp [hy])) hrest]
      rfl

/-- Lift a newly proved active-frame relation over a synchronized local write;
all suspended frames are preserved by `ImmFrameRel.writeLocal_above`. -/
theorem ImmStackRel.writeLocal {p p' : ImmPoint} {points : List ImmPoint}
    {s s' : MoveState} (h : ImmStackRel (p :: points) s s')
    (hp : p.frame = s.current) (hp' : p'.frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (x : LocalIndex) (v v' : Value)
    (hactive : p'.Rel (s.writeLocal x v) (s'.writeLocal x v')) :
    ImmStackRel (p' :: points) (s.writeLocal x v)
      (s'.writeLocal x v') := by
  refine ⟨by simpa using h.current_eq, by simpa using h.memory_eq,
    ?_, ?_⟩
  · intro q hq
    rcases List.mem_cons.mp hq with rfl | hq
    · exact hactive
    · exact (h.tracked q (by simp [hq])).writeLocal_above
        (hbelow q hq) h.current_eq x v v'
  · intro frame hnone
    have hne : frame ≠ s.current := by
      intro hframe
      exact hnone p' (by simp) (by simpa [hp'] using hframe.symm)
    have hne' : frame ≠ s'.current := by
      simpa [h.current_eq] using hne
    rw [MoveState.writeLocal_frames, MoveState.writeLocal_frames,
      if_neg hne', if_neg hne]
    apply h.untracked frame
    intro q hq
    rcases List.mem_cons.mp hq with rfl | hq
    · simpa [hp] using Ne.symm hne
    · exact hnone q (by simp [hq])

/-- Preserve an immutable stack relation across paired local-result writes. -/
theorem ImmStackRel.writeLocals {p p' : ImmPoint}
    {points : List ImmPoint} {s s' : MoveState}
    (h : ImmStackRel (p :: points) s s')
    (hp : p.frame = s.current) (hp' : p'.frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (xs : List LocalIndex) (vs : List Value)
    (hactive : p'.Rel (s.writeLocals xs vs) (s'.writeLocals xs vs)) :
    ImmStackRel (p' :: points) (s.writeLocals xs vs)
      (s'.writeLocals xs vs) := by
  refine ⟨by simpa using h.current_eq, by simpa using h.memory_eq,
    ?_, ?_⟩
  · intro q hq
    rcases List.mem_cons.mp hq with rfl | hq
    · exact hactive
    · exact (h.tracked q (by simp [hq])).writeLocals_above
        (hbelow q hq) h.current_eq xs vs
  · intro frame hnone
    have hne : frame ≠ s.current := by
      intro hframe
      exact hnone p' (by simp) (by simpa [hp'] using hframe.symm)
    have hne' : frame ≠ s'.current := by simpa [h.current_eq] using hne
    rw [MoveState.writeLocals_frames_of_ne s' xs vs hne',
      MoveState.writeLocals_frames_of_ne s xs vs hne]
    apply h.untracked frame
    intro q hq
    rcases List.mem_cons.mp hq with rfl | hq
    · simpa [hp] using Ne.symm hne
    · exact hnone q (by simp [hq])

/-- Store a reference-free scratch value above every declared local. -/
theorem ImmStackRel.writeTargetScratch {p : ImmPoint}
    {points : List ImmPoint} {s s' : MoveState}
    (h : ImmStackRel (p :: points) s s') (hp : p.frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    {tmp : LocalIndex} (htmp : p.decl.numLocals ≤ tmp) (v : Value) :
    ImmStackRel (p :: points) s (s'.writeLocal tmp v) := by
  refine ⟨by simpa using h.current_eq, h.memory_eq, ?_, ?_⟩
  · intro q hq
    rcases List.mem_cons.mp hq with rfl | hq
    · exact h.head.writeTargetScratch hp h.current_eq htmp v
    · exact (h.tracked q (by simp [hq])).writeTarget_above
        (hbelow q hq) h.current_eq tmp v
  · intro frame hnone
    have hne : frame ≠ s.current := by
      intro hframe
      exact hnone p (by simp) (by simpa [hp] using hframe.symm)
    have hne' : frame ≠ s'.current := by simpa [h.current_eq] using hne
    rw [MoveState.writeLocal_frames, if_neg hne']
    apply h.untracked frame
    intro q hq
    rcases List.mem_cons.mp hq with rfl | hq
    · simpa [hp] using Ne.symm hne
    · exact hnone q (by simp [hq])

/-- Equal reference-free local writes preserve the immutable stack relation. -/
theorem ImmStackRel.writeLocal_same_refFree {d : FunDecl}
    {before live : LiveSet} {g g' : BGraph} {frame : FrameId}
    {points : List ImmPoint} {s s' : MoveState} {i : Instr}
    {dst : LocalIndex} {v : Value}
    (h : ImmStackRel (⟨frame, d, before, g⟩ :: points) s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hdef : dst ∈ instrDefs i)
    (hcheck : immCheck d g live i = .ok ())
    (hsurvive : ∀ y, y ∈ live → y ≠ dst → y ∈ before)
    (hgraph : ∀ e, e ∈ g → e ∈ g')
    (hfree : v.refFree) :
    ImmStackRel (⟨frame, d, live, g'⟩ :: points)
      (s.writeLocal dst v) (s'.writeLocal dst v) := by
  apply h.writeLocal hframe hframe hbelow dst v v
  exact h.head.writeLocal_same_refFree hframe h.current_eq hdef hcheck
    hsurvive hgraph hfree

/-- Update an immutable stack relation for a source and target load. -/
theorem ImmStackRel.load {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {points : List ImmPoint}
    {s s' : MoveState} {dst : LocalIndex} {v : Value}
    (h : ImmStackRel
      (⟨frame, d, liveThroughInstr (.load dst v) live, g⟩ :: points) s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hcheck : immCheck d g live (.load dst v) = .ok ())
    (hfree : v.refFree) :
    ImmStackRel
      (⟨frame, d, live, immStep d g (.load dst v)⟩ :: points)
      (s.writeLocal dst v) (s'.writeLocal dst v) := by
  apply h.writeLocal hframe hframe hbelow dst v v
  exact h.head.load hframe h.current_eq hcheck hfree

/-- Update an immutable stack relation for a source and target assignment. -/
theorem ImmStackRel.assign {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {points : List ImmPoint}
    {s s' : MoveState} {dst src : LocalIndex} {v : Value}
    (h : ImmStackRel
      (⟨frame, d, liveThroughInstr (.assign dst src) live, g⟩ :: points)
      s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hrange : (instrDefs (.assign dst src) ++
      instrUses (.assign dst src)).all (· < d.numLocals) = true)
    (hcheck : immCheck d g live (.assign dst src) = .ok ())
    (hkinds : isImmLocal d dst = isImmLocal d src)
    (hsrc : s.locals src = some v) :
    ∃ v', s'.locals src = some v' ∧
      ImmStackRel
        (⟨frame, d, live, immStep d g (.assign dst src)⟩ :: points)
        (s.writeLocal dst v) (s'.writeLocal dst v') := by
  obtain ⟨v', hsrc', hactive⟩ := h.head.assign hframe h.current_eq
    hrange hcheck hkinds hsrc
  exact ⟨v', hsrc',
    h.writeLocal hframe hframe hbelow dst v v' hactive⟩

/-- Update an immutable stack relation for a local borrow. -/
theorem ImmStackRel.borrowLoc {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {points : List ImmPoint}
    {s s' : MoveState} {dst x : LocalIndex} {v : Value}
    (h : ImmStackRel
      (⟨frame, d, liveThroughInstr (.call [dst] .borrowLoc [x]) live, g⟩ ::
        points) s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hrange : (instrDefs (.call [dst] .borrowLoc [x]) ++
      instrUses (.call [dst] .borrowLoc [x])).all (· < d.numLocals) = true)
    (hcheck : immCheck d g live (.call [dst] .borrowLoc [x]) = .ok ())
    (hxPlain : isImmLocal d x = false)
    (hfree : isImmLocal d dst = true → v.refFree)
    (hx : s.locals x = some v) :
    s'.locals x = some v ∧
      ImmStackRel
        (⟨frame, d, live,
          immStep d g (.call [dst] .borrowLoc [x])⟩ :: points)
        (s.writeLocal dst (.ref ⟨.loc s.current x, []⟩))
        (s'.writeLocal dst
          (if isImmLocal d dst then v
           else .ref ⟨.loc s.current x, []⟩)) := by
  obtain ⟨hx', hactive⟩ := h.head.borrowLoc hframe h.current_eq
    hrange hcheck hxPlain hfree hx
  exact ⟨hx', h.writeLocal hframe hframe hbelow dst _ _ hactive⟩

/-- Simulate one immutable-eliminated load instruction. -/
theorem ImmStackRel.simulate_load {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {points : List ImmPoint}
    {s s' : MoveState} {dst : LocalIndex} {v : Value}
    {st st' : ElimSt} {tgt : List Instr}
    (h : ImmStackRel
      (⟨frame, d, liveThroughInstr (.load dst v) live, g⟩ :: points) s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hcheck : immCheck d g live (.load dst v) = .ok ())
    (hrewrite : elimImmInstr d st (.load dst v) = .ok (st', tgt)) :
    ∃ sNext', InstrPath tgt s' sNext' ∧
      ImmStackRel
        (⟨frame, d, live, immStep d g (.load dst v)⟩ :: points)
        (s.writeLocal dst v) sNext' := by
  obtain ⟨-, rfl, hfree⟩ := elimImmInstr_load_inv hrewrite
  exact ⟨s'.writeLocal dst v, InstrPath.one .load,
    h.load hframe hbelow hcheck hfree⟩

/-- Simulate one immutable-eliminated no-op instruction. -/
theorem ImmStackRel.simulate_nop {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {points : List ImmPoint}
    {s s' : MoveState} {st st' : ElimSt} {tgt : List Instr}
    (h : ImmStackRel
      (⟨frame, d, liveThroughInstr .nop live, g⟩ :: points) s s')
    (hrewrite : elimImmInstr d st .nop = .ok (st', tgt)) :
    ∃ sNext', InstrPath tgt s' sNext' ∧
      ImmStackRel (⟨frame, d, live, immStep d g .nop⟩ :: points)
        s sNext' := by
  obtain ⟨-, rfl⟩ := elimImmInstr_nop_inv hrewrite
  have hactive : ImmFrameRel d live (immStep d g .nop) frame s s' := by
    apply h.head.transport
    · intro y hy
      exact live_liveThroughInstr hy (by simp [instrDefs])
    · intro e he
      simpa [immStep] using he
  exact ⟨s', InstrPath.one .nop, h.replaceHead rfl hactive⟩

/-- Complete one-step simulation certificate for an assignment rewrite. -/
theorem ImmStackRel.simulate_assign {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {points : List ImmPoint}
    {s s' : MoveState} {dst src : LocalIndex} {v : Value}
    {st st' : ElimSt} {tgt : List Instr}
    (h : ImmStackRel
      (⟨frame, d, liveThroughInstr (.assign dst src) live, g⟩ :: points)
      s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hrange : (instrDefs (.assign dst src) ++
      instrUses (.assign dst src)).all (· < d.numLocals) = true)
    (hcheck : immCheck d g live (.assign dst src) = .ok ())
    (hrewrite : elimImmInstr d st (.assign dst src) = .ok (st', tgt))
    (hsrc : s.locals src = some v) :
    ∃ sNext', InstrPath tgt s' sNext' ∧
      ImmStackRel
        (⟨frame, d, live, immStep d g (.assign dst src)⟩ :: points)
        (s.writeLocal dst v) sNext' := by
  obtain ⟨-, rfl, hkinds⟩ := elimImmInstr_assign_inv hrewrite
  obtain ⟨v', hsrc', hrel⟩ := h.assign hframe hbelow hrange hcheck
    hkinds hsrc
  exact ⟨s'.writeLocal dst v', InstrPath.one (.assign hsrc'), hrel⟩

/-- Simulate a normally returning ordinary operation. -/
theorem ImmStackRel.simulate_op {d : FunDecl} {live : LiveSet}
    {g : BGraph} {frame : FrameId} {points : List ImmPoint}
    {s s' : MoveState} {dsts srcs : List LocalIndex} {op : Oper}
    {vs rets : List Value} {m' : Memory} {st st' : ElimSt}
    {tgt : List Instr}
    (h : ImmStackRel
      (⟨frame, d, liveThroughInstr (.call dsts op srcs) live, g⟩ ::
        points) s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hrange : (instrDefs (.call dsts op srcs) ++
      instrUses (.call dsts op srcs)).all (· < d.numLocals) = true)
    (hcheck : immCheck d g live (.call dsts op srcs) = .ok ())
    (hrewrite : elimImmInstr d st (.call dsts op srcs) = .ok (st', tgt))
    (hsrcs : srcs.mapM s.locals = some vs)
    (hlen : dsts.length = rets.length)
    (hop : op.sem s.current s.readTarget vs s.memory = some (.ok rets m'))
    (hsafe : ImmOpSafe
      (⟨frame, d, liveThroughInstr (.call dsts op srcs) live, g⟩ ::
        points) s op vs rets m')
    (hsourceSafe : FrameSafe live
      (fun rootLocal =>
        rootLocal < d.numLocals ∧ isImmLocal d rootLocal = false)
      (ImmCovers frame g) frame
      ((s.setMemory m').writeLocals dsts rets)) :
    ∃ sNext', InstrPath tgt s' sNext' ∧
      ImmStackRel
        (⟨frame, d, live, immStep d g (.call dsts op srcs)⟩ :: points)
        ((s.setMemory m').writeLocals dsts rets) sNext' := by
  obtain ⟨vs', hsrcs', hviews⟩ := h.lookup_views hframe
    (fun x hx => uses_mem_liveThroughInstr (by
      simpa [instrUses] using hx))
    (fun x hx => of_decide_eq_true (List.all_eq_true.mp hrange x (by
      simp only [instrDefs, instrUses, List.mem_append]
      exact Or.inr hx))) hsrcs
  have hop' := hviews.op_sem h.current_eq h.memory_eq hop hsafe.operands
  obtain ⟨-, rfl⟩ := elimImmInstr_op_inv hop hrewrite
  have hmem := h.setMemory m' hsafe.stable
  have hactive := hmem.head.writeLocals_same_refFree hframe
    (by simpa using hmem.current_eq) hcheck hlen hsafe.results hsourceSafe
  have hafter := hmem.writeLocals
    (p' := ⟨frame, d, live, g⟩) hframe hframe hbelow dsts rets
    (by simpa [ImmPoint.Rel] using hactive)
  refine ⟨(s'.setMemory m').writeLocals dsts rets,
    InstrPath.one (.op hsrcs' hlen hop'), ?_⟩
  simpa [immStep_op hop] using hafter

/-- Simulate an aborting ordinary operation. -/
theorem ImmStackRel.simulate_op_abort {P' : Program} {G : Cfg}
    {d : FunDecl} {live : LiveSet} {g : BGraph}
    {frame : FrameId} {points : List ImmPoint} {s s' : MoveState}
    {dsts srcs : List LocalIndex} {op : Oper} {vs : List Value}
    {st st' : ElimSt} {tgt rest : List Instr} {term : Term}
    (h : ImmStackRel
      (⟨frame, d, liveThroughInstr (.call dsts op srcs) live, g⟩ ::
        points) s s')
    (hframe : frame = s.current)
    (hrange : (instrDefs (.call dsts op srcs) ++
      instrUses (.call dsts op srcs)).all (· < d.numLocals) = true)
    (hrewrite : elimImmInstr d st (.call dsts op srcs) = .ok (st', tgt))
    (hsrcs : srcs.mapM s.locals = some vs)
    (hop : op.sem s.current s.readTarget vs s.memory = some .abort)
    (hsafe : ImmOperandsSafe op vs) :
    RunFrom P' G (tgt ++ rest) term s'
      (.abort s.memory op.abortCode) := by
  obtain ⟨vs', hsrcs', hviews⟩ := h.lookup_views hframe
    (fun x hx => uses_mem_liveThroughInstr (by
      simpa [instrUses] using hx))
    (fun x hx => of_decide_eq_true (List.all_eq_true.mp hrange x (by
      simp only [instrDefs, instrUses, List.mem_append]
      exact Or.inr hx))) hsrcs
  have hop' := hviews.op_sem h.current_eq h.memory_eq hop hsafe
  obtain ⟨-, rfl⟩ := elimImmInstr_op_inv hop hrewrite
  simpa [h.memory_eq] using
    (InstrStop.run (rest := rest) (term := term) (.op hsrcs' hop'))

/-- Complete one-step simulation certificate for a local-borrow rewrite. -/
theorem ImmStackRel.simulate_borrowLoc {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {points : List ImmPoint} {s s' : MoveState} {dst x : LocalIndex}
    {v : Value} {st st' : ElimSt} {tgt : List Instr}
    (h : ImmStackRel
      (⟨frame, d, liveThroughInstr (.call [dst] .borrowLoc [x]) live, g⟩ ::
        points) s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hrange : (instrDefs (.call [dst] .borrowLoc [x]) ++
      instrUses (.call [dst] .borrowLoc [x])).all (· < d.numLocals) = true)
    (hcheck : immCheck d g live (.call [dst] .borrowLoc [x]) = .ok ())
    (hrewrite : elimImmInstr d st (.call [dst] .borrowLoc [x]) =
      .ok (st', tgt))
    (hsafe : isImmLocal d x = false ∧
      (isImmLocal d dst = true → v.refFree))
    (hx : s.locals x = some v) :
    ∃ sNext', InstrPath tgt s' sNext' ∧
      ImmStackRel
        (⟨frame, d, live,
          immStep d g (.call [dst] .borrowLoc [x])⟩ :: points)
        (s.writeLocal dst (.ref ⟨.loc s.current x, []⟩)) sNext' := by
  obtain ⟨-, hcases⟩ := elimImmInstr_borrowLoc_inv hrewrite
  obtain ⟨hx', hrel⟩ := h.borrowLoc hframe hbelow hrange hcheck
    hsafe.1 hsafe.2 hx
  rcases hcases with ⟨himm, -, -, rfl⟩ | ⟨himm, rfl⟩
  · exact ⟨s'.writeLocal dst v, InstrPath.one (.assign hx'),
      by simpa [himm] using hrel⟩
  · refine ⟨s'.writeLocal dst (.ref ⟨.loc s'.current x, []⟩),
      InstrPath.one (.borrowLoc hx'), ?_⟩
    simpa [himm, h.current_eq] using hrel

/-- Simulate a successful immutable-eliminated global borrow. -/
theorem ImmStackRel.simulate_borrowGlobal {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {points : List ImmPoint} {s s' : MoveState} {dst t : LocalIndex}
    {r : ResourceId} {a : Address} {v : Value}
    {st st' : ElimSt} {tgt : List Instr}
    (h : ImmStackRel
      (⟨frame, d,
        liveThroughInstr (.call [dst] (.borrowGlobal r) [t]) live, g⟩ ::
        points) s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hrange : (instrDefs (.call [dst] (.borrowGlobal r) [t]) ++
      instrUses (.call [dst] (.borrowGlobal r) [t])).all
        (· < d.numLocals) = true)
    (hcheck : immCheck d g live
      (.call [dst] (.borrowGlobal r) [t]) = .ok ())
    (hrewrite : elimImmInstr d st (.call [dst] (.borrowGlobal r) [t]) =
      .ok (st', tgt))
    (ha : s.locals t = some (.address a)) (hfree : v.refFree)
    (hpresent : s.memory r a = some v) :
    ∃ sNext', InstrPath tgt s' sNext' ∧
      ImmStackRel
        (⟨frame, d, live,
          immStep d g (.call [dst] (.borrowGlobal r) [t])⟩ :: points)
        (s.writeLocal dst (.ref ⟨.global r a, []⟩)) sNext' := by
  have htLive : t ∈
      liveThroughInstr (.call [dst] (.borrowGlobal r) [t]) live :=
    uses_mem_liveThroughInstr (by simp [instrUses])
  have htRange : t < d.numLocals :=
    of_decide_eq_true (List.all_eq_true.mp hrange t
      (by simp [instrDefs, instrUses]))
  have ha' := h.lookup_eq_of_refFree hframe htLive htRange
    (v := .address a) (by simp) ha
  have hpresent' : s'.memory r a = some v := by
    rw [h.memory_eq]
    exact hpresent
  have hactive := h.head.borrowGlobal hframe h.current_eq h.memory_eq
    hcheck hfree hpresent
  have hafter := h.writeLocal
    (p' := ⟨frame, d, live,
      immStep d g (.call [dst] (.borrowGlobal r) [t])⟩)
    hframe hframe hbelow dst (.ref ⟨.global r a, []⟩)
      (if isImmLocal d dst then v else .ref ⟨.global r a, []⟩) hactive
  obtain ⟨-, hcases⟩ := elimImmInstr_borrowGlobal_inv hrewrite
  rcases hcases with ⟨himm, rfl⟩ | ⟨himm, rfl⟩
  · have hsrcs : [t].mapM s'.locals = some [.address a] := by
      rw [List.mapM_cons, ha']
      rfl
    have hop : (Oper.getGlobal r).sem s'.current s'.readTarget
        [.address a] s'.memory = some (.ok [v] s'.memory) := by
      simp [Oper.sem, hpresent']
    have hnext : InstrNext (.call [dst] (.getGlobal r) [t]) s'
        (s'.writeLocal dst v) := by
      have hset : s'.setMemory s'.memory = s' := by cases s'; rfl
      have hend : (s'.setMemory s'.memory).writeLocals [dst] [v] =
          s'.writeLocal dst v := by rw [hset]; rfl
      rw [← hend]
      exact InstrNext.op hsrcs (by rfl) hop
    exact ⟨s'.writeLocal dst v, InstrPath.one hnext,
      by simpa [himm] using hafter⟩
  · exact ⟨s'.writeLocal dst (.ref ⟨.global r a, []⟩),
      InstrPath.one (.borrowGlobal ha' hpresent'),
      by simpa [himm] using hafter⟩

/-- Simulate an aborting global borrow. -/
theorem ImmStackRel.simulate_borrowGlobal_abort {P' : Program} {G : Cfg}
    {d : FunDecl} {live : LiveSet} {g : BGraph}
    {frame : FrameId} {points : List ImmPoint} {s s' : MoveState}
    {dst t : LocalIndex} {r : ResourceId} {a : Address}
    {st st' : ElimSt} {tgt rest : List Instr} {term : Term}
    (h : ImmStackRel
      (⟨frame, d,
        liveThroughInstr (.call [dst] (.borrowGlobal r) [t]) live, g⟩ ::
        points) s s')
    (hframe : frame = s.current)
    (hrange : (instrDefs (.call [dst] (.borrowGlobal r) [t]) ++
      instrUses (.call [dst] (.borrowGlobal r) [t])).all
        (· < d.numLocals) = true)
    (hrewrite : elimImmInstr d st (.call [dst] (.borrowGlobal r) [t]) =
      .ok (st', tgt))
    (ha : s.locals t = some (.address a))
    (habsent : s.memory r a = none) :
    RunFrom P' G (tgt ++ rest) term s'
      (.abort s.memory runtimeAbortCode) := by
  have htLive : t ∈
      liveThroughInstr (.call [dst] (.borrowGlobal r) [t]) live :=
    uses_mem_liveThroughInstr (by simp [instrUses])
  have htRange : t < d.numLocals :=
    of_decide_eq_true (List.all_eq_true.mp hrange t
      (by simp [instrDefs, instrUses]))
  have ha' := h.lookup_eq_of_refFree hframe htLive htRange
    (v := .address a) (by simp) ha
  have habsent' : s'.memory r a = none := by
    rw [h.memory_eq]
    exact habsent
  obtain ⟨-, hcases⟩ := elimImmInstr_borrowGlobal_inv hrewrite
  rcases hcases with ⟨-, rfl⟩ | ⟨-, rfl⟩
  · have hsrcs : [t].mapM s'.locals = some [.address a] := by
      rw [List.mapM_cons, ha']
      rfl
    have hop : (Oper.getGlobal r).sem s'.current s'.readTarget
        [.address a] s'.memory = some .abort := by
      simp [Oper.sem, habsent']
    simpa [h.memory_eq, Oper.abortCode] using
      (InstrStop.run (rest := rest) (term := term) (.op hsrcs hop))
  · simpa [h.memory_eq] using
      (InstrStop.run (rest := rest) (term := term)
        (.borrowGlobal ha' habsent'))

/-- Simulate an immutable-eliminated reference read. -/
theorem ImmStackRel.simulate_readRef {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {points : List ImmPoint} {s s' : MoveState} {dst t : LocalIndex}
    {rt : RefTarget} {v : Value} {st st' : ElimSt} {tgt : List Instr}
    (h : ImmStackRel
      (⟨frame, d, liveThroughInstr (.call [dst] .readRef [t]) live, g⟩ ::
        points) s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hrange : (instrDefs (.call [dst] .readRef [t]) ++
      instrUses (.call [dst] .readRef [t])).all (· < d.numLocals) = true)
    (hcheck : immCheck d g live (.call [dst] .readRef [t]) = .ok ())
    (hrewrite : elimImmInstr d st (.call [dst] .readRef [t]) =
      .ok (st', tgt))
    (ht : s.locals t = some (.ref rt))
    (hv : s.readTarget rt = some v) (hfree : v.refFree) :
    ∃ sNext', InstrPath tgt s' sNext' ∧
      ImmStackRel
        (⟨frame, d, live,
          immStep d g (.call [dst] .readRef [t])⟩ :: points)
        (s.writeLocal dst v) sNext' := by
  have htLive : t ∈
      liveThroughInstr (.call [dst] .readRef [t]) live :=
    uses_mem_liveThroughInstr (by simp [instrUses])
  have htRange : t < d.numLocals :=
    of_decide_eq_true (List.all_eq_true.mp hrange t
      (by simp [instrDefs, instrUses]))
  obtain ⟨tv, ht', hval⟩ := h.lookup hframe htLive htRange ht
  obtain ⟨-, hcases⟩ := elimImmInstr_readRef_inv hrewrite
  have hafter := h.writeLocal_same_refFree hframe hbelow
    (i := .call [dst] .readRef [t]) (dst := dst) (by simp [instrDefs])
    hcheck
    (fun y hy hydst => live_liveThroughInstr hy
      (by simp [instrDefs, hydst]))
    (fun e he => by simpa [immStep] using he) hfree
  rcases hcases with ⟨himm, rfl⟩ | ⟨himm, rfl⟩
  · rw [himm] at hval
    rcases hval with ⟨hrefFree, -⟩ | ⟨rt', href, tvFree, hread⟩
    · simp at hrefFree
    · cases href
      rw [hv] at hread
      cases hread
      exact ⟨s'.writeLocal dst v, InstrPath.one (.assign ht'), hafter⟩
  · rw [himm] at hval
    subst tv
    have hread := h.head.refs_agree t htLive rt
      (by simpa [MoveState.locals, hframe] using ht)
    exact ⟨s'.writeLocal dst v,
      InstrPath.one (.readRef ht' (hread.trans hv) hfree), hafter⟩

/-- Simulate an immutable-eliminated reference freeze. -/
theorem ImmStackRel.simulate_freezeRef {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {points : List ImmPoint} {s s' : MoveState} {dst t : LocalIndex}
    {rt : RefTarget} {v : Value} {st st' : ElimSt} {tgt : List Instr}
    (h : ImmStackRel
      (⟨frame, d, liveThroughInstr (.call [dst] .freezeRef [t]) live, g⟩ ::
        points) s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hrange : (instrDefs (.call [dst] .freezeRef [t]) ++
      instrUses (.call [dst] .freezeRef [t])).all (· < d.numLocals) = true)
    (hcheck : immCheck d g live (.call [dst] .freezeRef [t]) = .ok ())
    (hrewrite : elimImmInstr d st (.call [dst] .freezeRef [t]) =
      .ok (st', tgt))
    (ht : s.locals t = some (.ref rt))
    (hv : s.readTarget rt = some v) (hfree : v.refFree) :
    ∃ sNext', InstrPath tgt s' sNext' ∧
      ImmStackRel
        (⟨frame, d, live,
          immStep d g (.call [dst] .freezeRef [t])⟩ :: points)
        (s.writeLocal dst (.ref rt)) sNext' := by
  have htLive : t ∈
      liveThroughInstr (.call [dst] .freezeRef [t]) live :=
    uses_mem_liveThroughInstr (by simp [instrUses])
  have htRange : t < d.numLocals :=
    of_decide_eq_true (List.all_eq_true.mp hrange t
      (by simp [instrDefs, instrUses]))
  have htFrame : s.frames frame t = some (.ref rt) := by
    simpa [MoveState.locals, hframe] using ht
  obtain ⟨tv, ht', hval⟩ := h.lookup hframe htLive htRange ht
  obtain ⟨-, hdimm, hcases⟩ := elimImmInstr_freezeRef_inv hrewrite
  have hcov := h.head.covers t htLive rt htFrame
  have hactive : ImmFrameRel d live
      (immStep d g (.call [dst] .freezeRef [t])) frame
      (s.writeLocal dst (.ref rt)) (s'.writeLocal dst v) := by
    apply h.head.writeCopiedRef hframe h.current_eq
      (i := .call [dst] .freezeRef [t]) (dst := dst) (src := t)
      (by simp [instrDefs]) hcheck
    · intro y hy hydst
      exact live_liveThroughInstr hy (by simp [instrDefs, hydst])
    · intro e he
      simpa [immStep] using (mem_gInsertClosed (e' :=
        ⟨.refNode t, dst, []⟩) he)
    · exact htLive
    · exact htFrame
    · exact hdimm
    · exact hfree
    · exact hv
    · simpa [immStep] using hcov.copy
  have hafter := h.writeLocal
    (p' := ⟨frame, d, live,
      immStep d g (.call [dst] .freezeRef [t])⟩)
    hframe hframe hbelow dst (.ref rt) v hactive
  rcases hcases with ⟨htimm, rfl⟩ | ⟨htmut, rfl⟩
  · rw [htimm] at hval
    rcases hval with ⟨hrefFree, -⟩ | ⟨rt', href, tvFree, hread⟩
    · simp at hrefFree
    · cases href
      rw [hv] at hread
      cases hread
      exact ⟨s'.writeLocal dst v, InstrPath.one (.assign ht'), hafter⟩
  · have htimm := isImmLocal_false_of_isMutLocal htmut
    rw [htimm] at hval
    subst tv
    have hread := h.head.refs_agree t htLive rt htFrame
    exact ⟨s'.writeLocal dst v,
      InstrPath.one (.readRef ht' (hread.trans hv) hfree), hafter⟩

/-- Simulate an immutable-eliminated reference write. -/
theorem ImmStackRel.simulate_writeRef {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {points : List ImmPoint} {s s' sNext : MoveState}
    {t vt : LocalIndex} {rt : RefTarget} {v : Value}
    {st st' : ElimSt} {tgt : List Instr}
    (h : ImmStackRel
      (⟨frame, d, liveThroughInstr (.call [] .writeRef [t, vt]) live, g⟩ ::
        points) s s')
    (hframe : frame = s.current)
    (hrange : (instrDefs (.call [] .writeRef [t, vt]) ++
      instrUses (.call [] .writeRef [t, vt])).all
        (· < d.numLocals) = true)
    (hrewrite : elimImmInstr d st (.call [] .writeRef [t, vt]) =
      .ok (st', tgt))
    (ht : s.locals t = some (.ref rt)) (hv : s.locals vt = some v)
    (hfree : v.refFree) (hwrite : s.writeTarget rt v = some sNext)
    (hsafe : ImmWriteSafe
      (⟨frame, d, liveThroughInstr (.call [] .writeRef [t, vt]) live, g⟩ ::
        points) s s' rt v) :
    ∃ sNext', InstrPath tgt s' sNext' ∧
      ImmStackRel
        (⟨frame, d, live,
          immStep d g (.call [] .writeRef [t, vt])⟩ :: points)
        sNext sNext' := by
  obtain ⟨rfl, htPlain, rfl⟩ := elimImmInstr_writeRef_inv hrewrite
  have htLive : t ∈
      liveThroughInstr (.call [] .writeRef [t, vt]) live :=
    uses_mem_liveThroughInstr (by simp [instrUses])
  have hvLive : vt ∈
      liveThroughInstr (.call [] .writeRef [t, vt]) live :=
    uses_mem_liveThroughInstr (by simp [instrUses])
  have htRange : t < d.numLocals :=
    of_decide_eq_true (List.all_eq_true.mp hrange t
      (by simp [instrDefs, instrUses]))
  have hvRange : vt < d.numLocals :=
    of_decide_eq_true (List.all_eq_true.mp hrange vt
      (by simp [instrDefs, instrUses]))
  have ht' := h.lookup_eq hframe htLive htRange htPlain ht
  have hv' := h.lookup_eq_of_refFree hframe hvLive hvRange hfree hv
  have finish : ∀ {source target : MoveState},
      ImmStackRel
        (⟨frame, d, liveThroughInstr (.call [] .writeRef [t, vt]) live, g⟩ ::
          points) source target →
      ImmStackRel
        (⟨frame, d, live,
          immStep d g (.call [] .writeRef [t, vt])⟩ :: points)
        source target := by
    intro source target hafter
    apply hafter.replaceHead
      (p' := ⟨frame, d, live,
        immStep d g (.call [] .writeRef [t, vt])⟩) rfl
    apply hafter.head.transport
    · intro x hx
      exact live_liveThroughInstr hx (by simp [instrDefs])
    · intro e he
      simpa [immStep] using he
  cases rt with
  | mk root path =>
    cases root with
    | loc rootFrame rootLocal =>
      cases hroot : s.frames rootFrame rootLocal with
      | none => simp [MoveState.writeTarget, hroot] at hwrite
      | some root =>
        cases hset : root.setPath path v with
        | none => simp [MoveState.writeTarget, hroot, hset] at hwrite
        | some root' =>
          have hsource := MoveState.writeTarget_loc hroot hset
          rw [hsource] at hwrite
          cases hwrite
          obtain ⟨hroot', hrootFree, hstable⟩ :=
            hsafe.localRoot rootFrame rootLocal path root root' rfl hroot hset
          have htarget := MoveState.writeTarget_loc hroot' hset
          have hafter := h.writeFrameLocal_same rootFrame rootLocal root'
            hrootFree hstable
          exact ⟨_, .one (.writeRef ht' hv' hfree htarget),
            finish hafter⟩
    | global r a =>
      cases hroot : s.memory r a with
      | none => simp [MoveState.writeTarget, hroot] at hwrite
      | some root =>
        cases hset : root.setPath path v with
        | none => simp [MoveState.writeTarget, hroot, hset] at hwrite
        | some root' =>
          have hsource := MoveState.writeTarget_global hroot hset
          rw [hsource] at hwrite
          cases hwrite
          have hstable :=
            hsafe.globalRoot r a path root root' rfl hroot hset
          have hroot' : s'.memory r a = some root := by
            rw [h.memory_eq]
            exact hroot
          have htarget := MoveState.writeTarget_global hroot' hset
          have hafter : ImmStackRel
              (⟨frame, d,
                liveThroughInstr (.call [] .writeRef [t, vt]) live, g⟩ ::
                points)
              (s.writeGlobal r a root') (s'.writeGlobal r a root') := by
            simpa [MoveState.writeGlobal, MoveState.setMemory, h.memory_eq] using
              h.setMemory (memWrite s.memory r a root') hstable
          exact ⟨_, .one (.writeRef ht' hv' hfree htarget),
            finish hafter⟩

/-- Shared execution rule for field and vector-element borrows.  The
container-specific wrappers only establish the child lookup and the
`getField`/`vecGet` head steps; immutable-copy, mutable-scratch, and retained
reference handling is centralized here. -/
theorem ImmStackRel.simulate_derivedBorrow {P : Program} {d : FunDecl}
    {live before : LiveSet} {g g' : BGraph} {frame : FrameId}
    {points : List ImmPoint} {s s' : MoveState} {dst src : LocalIndex}
    {parent child : RefTarget} {container w : Value} {st st' : ElimSt}
    {sourceInstr : Instr} {getInstr : LocalIndex → Instr}
    {tgt : List Instr}
    (h : ImmStackRel (⟨frame, d, before, g⟩ :: points) s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hbase : d.numLocals ≤ st.base)
    (hcheck : immCheck d g live sourceInstr = .ok ())
    (hdef : dst ∈ instrDefs sourceInstr)
    (hsurvive : ∀ y, y ∈ live → y ≠ dst → y ∈ before)
    (hgraph : ∀ e, e ∈ g → e ∈ g')
    (hsrcLive : src ∈ before) (hsrcRange : src < d.numLocals)
    (hsrc : s.locals src = some (.ref parent))
    (hsrcFrame : s.frames frame src = some (.ref parent))
    (hcontainer : s.readTarget parent = some container)
    (hroot : child.root = parent.root)
    (hchildRead : s'.readTarget child = s.readTarget child)
    (hchildValue : s.readTarget child = some w)
    (hvalueFree : container.refFree → w.refFree)
    (hcover : ImmCovers frame g' dst child)
    (hcases :
      (isImmLocal d dst = true ∧ isImmLocal d src = true ∧ st' = st ∧
        tgt = [getInstr src]) ∨
      (isImmLocal d dst = true ∧ isMutLocal d src = true ∧
        (∃ ty, d.locals src = some (.mutRef ty) ∧
          st' = (st.alloc ty).1 ∧
          tgt = [.call [(st.alloc ty).2] .readRef [src],
            getInstr (st.alloc ty).2])) ∨
      (isImmLocal d dst = false ∧ isImmLocal d src = false ∧
        st' = st ∧ tgt = [sourceInstr]))
    (hget : s'.locals src = some container →
      InstrNext (getInstr src) s' (s'.writeLocal dst w))
    (hgetScratch : ∀ tmp, d.numLocals ≤ tmp →
      InstrNext (getInstr tmp) (s'.writeLocal tmp container)
        ((s'.writeLocal tmp container).writeLocal dst w))
    (hkeep : s'.locals src = some (.ref parent) →
      s'.readTarget parent = some container →
      InstrNext sourceInstr s' (s'.writeLocal dst (.ref child)))
    (hchecked : CheckedState P d s) :
    ∃ sNext', InstrPath tgt s' sNext' ∧
      ImmStackRel (⟨frame, d, live, g'⟩ :: points)
        (s.writeLocal dst (.ref child)) sNext' := by
  obtain ⟨srcValue, hsrc', hview⟩ :=
    h.lookup hframe hsrcLive hsrcRange hsrc
  have hparentRead := h.head.refs_agree src hsrcLive parent hsrcFrame
  rcases hcases with hcase | hcase | hcase
  · obtain ⟨hdimm, hsimm, -, rfl⟩ := hcase
    obtain ⟨ty, hty⟩ := isImmLocal_decl hsimm
    have hcontainerFree := hchecked.consistent.refTarget_free
      hty rfl hsrc hcontainer
    rw [hsimm] at hview
    rcases hview with ⟨hrefFree, -⟩ | ⟨parent', href, -, hread⟩
    · simp at hrefFree
    · cases href
      rw [hcontainer] at hread
      cases hread
      have hactive := h.head.writeDerivedRef hframe h.current_eq
        (i := sourceInstr) (dst := dst) (src := src)
        (parent := parent) (child := child) (v := w)
        hdef hcheck hsurvive hgraph hsrcLive hsrcFrame hroot
        hchildRead hchildValue (fun _ => hvalueFree hcontainerFree) hcover
      have hafter := h.writeLocal
        (p' := ⟨frame, d, live, g'⟩) hframe hframe hbelow
        dst (.ref child) w
        (by simpa [ImmPoint.Rel, hdimm] using hactive)
      exact ⟨_, InstrPath.one (hget hsrc'), hafter⟩
  · obtain ⟨hdimm, hsmut, ty, hty, -, rfl⟩ := hcase
    have hcontainerFree := hchecked.consistent.refTarget_free
      hty rfl hsrc hcontainer
    have hsimm := isImmLocal_false_of_isMutLocal hsmut
    rw [hsimm] at hview
    subst srcValue
    let tmp := (st.alloc ty).2
    have htmp : d.numLocals ≤ tmp :=
      Nat.le_trans hbase (by simp [tmp, ElimSt.alloc])
    have hscratch := h.writeTargetScratch hframe hbelow htmp container
    have hchildScratch : (s'.writeLocal tmp container).readTarget child =
        s.readTarget child := by
      rw [MoveState.readTarget_writeLocal_of_root_ne]
      · exact hchildRead
      · intro hchildRoot
        have hparentRoot : parent.root = .loc frame tmp := by
          rw [← hroot]
          simpa [h.current_eq, hframe] using hchildRoot
        have hrange :=
          (h.head.roots_plain src hsrcLive parent hsrcFrame tmp hparentRoot).1
        exact (Nat.not_lt_of_ge htmp) hrange
    have hactive := hscratch.head.writeDerivedRef hframe
      hscratch.current_eq (i := sourceInstr) (dst := dst) (src := src)
      (parent := parent) (child := child) (v := w)
      hdef hcheck hsurvive hgraph hsrcLive hsrcFrame hroot
      hchildScratch hchildValue (fun _ => hvalueFree hcontainerFree) hcover
    have hafter := hscratch.writeLocal
      (p' := ⟨frame, d, live, g'⟩) hframe hframe hbelow
      dst (.ref child) w
      (by simpa [ImmPoint.Rel, hdimm] using hactive)
    have hread : s'.readTarget parent = some container :=
      hparentRead.trans hcontainer
    have hfirst : InstrNext (.call [tmp] .readRef [src]) s'
        (s'.writeLocal tmp container) :=
      .readRef hsrc' hread hcontainerFree
    exact ⟨_, .cons hfirst (.one (hgetScratch tmp htmp)),
      by simpa [tmp] using hafter⟩
  · obtain ⟨hdimm, hsimm, -, rfl⟩ := hcase
    rw [hsimm] at hview
    subst srcValue
    have hactive := h.head.writeDerivedRef hframe h.current_eq
      (i := sourceInstr) (dst := dst) (src := src)
      (parent := parent) (child := child) (v := w)
      hdef hcheck hsurvive hgraph hsrcLive hsrcFrame hroot
      hchildRead hchildValue
      (fun himm => by rw [hdimm] at himm; cases himm) hcover
    have hafter := h.writeLocal
      (p' := ⟨frame, d, live, g'⟩) hframe hframe hbelow
      dst (.ref child) (.ref child)
      (by simpa [ImmPoint.Rel, hdimm] using hactive)
    exact ⟨_, InstrPath.one (hkeep hsrc' (hparentRead.trans hcontainer)),
      hafter⟩

/-- Simulate an immutable-eliminated field borrow. -/
theorem ImmStackRel.simulate_borrowField {P : Program} {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {points : List ImmPoint} {s s' : MoveState} {dst t i : Nat}
    {rt : RefTarget} {fs : List Value} {st st' : ElimSt}
    {tgt : List Instr}
    (h : ImmStackRel
      (⟨frame, d,
        liveThroughInstr (.call [dst] (.borrowField i) [t]) live, g⟩ ::
        points) s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hbase : d.numLocals ≤ st.base)
    (hrange : (instrDefs (.call [dst] (.borrowField i) [t]) ++
      instrUses (.call [dst] (.borrowField i) [t])).all
        (· < d.numLocals) = true)
    (hcheck : immCheck d g live
      (.call [dst] (.borrowField i) [t]) = .ok ())
    (hrewrite : elimImmInstr d st (.call [dst] (.borrowField i) [t]) =
      .ok (st', tgt))
    (ht : s.locals t = some (.ref rt))
    (hs : s.readTarget rt = some (.struct fs))
    (hi : i < fs.length)
    (hchecked : CheckedState P d s) :
    ∃ sNext', InstrPath tgt s' sNext' ∧
      ImmStackRel
        (⟨frame, d, live,
          immStep d g (.call [dst] (.borrowField i) [t])⟩ :: points)
        (s.writeLocal dst (.ref ⟨rt.root, rt.path ++ [i]⟩)) sNext' := by
  let child : RefTarget := ⟨rt.root, rt.path ++ [i]⟩
  let w : Value := fs[i]
  have hfi : fs[i]? = some w := by simp [w, hi]
  have htLive : t ∈
      liveThroughInstr (.call [dst] (.borrowField i) [t]) live :=
    uses_mem_liveThroughInstr (by simp [instrUses])
  have htRange : t < d.numLocals :=
    of_decide_eq_true (List.all_eq_true.mp hrange t
      (by simp [instrDefs, instrUses]))
  have htFrame : s.frames frame t = some (.ref rt) := by
    simpa [MoveState.locals, hframe] using ht
  have hparentRead := h.head.refs_agree t htLive rt htFrame
  have hchildValue : s.readTarget child = some w := by
    rw [show child = ⟨rt.root, rt.path ++ [i]⟩ from rfl,
      MoveState.readTarget_snoc, hs]
    simp [Value.getPath, hfi]
  have hchildRead : s'.readTarget child = s.readTarget child := by
    rw [show child = ⟨rt.root, rt.path ++ [i]⟩ from rfl,
      MoveState.readTarget_snoc, MoveState.readTarget_snoc, hparentRead]
  have graphSub : ∀ e, e ∈ g →
      e ∈ immStep d g (.call [dst] (.borrowField i) [t]) := by
    intro e he
    simpa [immStep] using
      (mem_gInsertClosed (e' := ⟨.refNode t, dst, []⟩) he)
  have childCover : ImmCovers frame
      (immStep d g (.call [dst] (.borrowField i) [t])) dst child := by
    simpa [child, ImmCovers, immStep] using
      (h.head.covers t htLive rt htFrame).copy
  apply h.simulate_derivedBorrow (dst := dst) (src := t) (parent := rt)
      (child := child) (container := .struct fs) (w := w) (st := st)
      (st' := st') (sourceInstr := .call [dst] (.borrowField i) [t])
      (getInstr := fun x => .call [dst] (.getField i) [x]) (tgt := tgt)
      hframe hbelow hbase hcheck
      (by simp [instrDefs])
      (fun y hy hydst => live_liveThroughInstr hy
        (by simp [instrDefs, hydst])) graphSub htLive htRange ht htFrame hs
      rfl hchildRead hchildValue
      (fun hfree => Value.refFree_of_getElem? hfree hfi) childCover
      (by simpa using elimImmInstr_borrowField_inv hrewrite)
  · intro ht'
    apply InstrNext.op_one (vs := [.struct fs])
    · rw [List.mapM_cons, ht']
      rfl
    · simp [Oper.sem, hfi]
  · intro tmp _
    apply InstrNext.op_one (vs := [.struct fs])
    · rw [List.mapM_cons, show (s'.writeLocal tmp (.struct fs)).locals tmp =
          some (.struct fs) by simp]
      rfl
    · simp [Oper.sem, hfi]
  · intro ht' hread
    exact .borrowField ht' hread hi
  · exact hchecked

/-- Simulate a successful immutable-eliminated vector-element borrow. -/
theorem ImmStackRel.simulate_borrowVecElem {P : Program} {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {points : List ImmPoint} {s s' : MoveState} {dst t it n : Nat}
    {rt : RefTarget} {es : List Value} {st st' : ElimSt}
    {tgt : List Instr}
    (h : ImmStackRel
      (⟨frame, d,
        liveThroughInstr (.call [dst] .borrowVecElem [t, it]) live, g⟩ ::
        points) s s')
    (hframe : frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    (hbase : d.numLocals ≤ st.base)
    (hrange : (instrDefs (.call [dst] .borrowVecElem [t, it]) ++
      instrUses (.call [dst] .borrowVecElem [t, it])).all
        (· < d.numLocals) = true)
    (hcheck : immCheck d g live
      (.call [dst] .borrowVecElem [t, it]) = .ok ())
    (hrewrite : elimImmInstr d st (.call [dst] .borrowVecElem [t, it]) =
      .ok (st', tgt))
    (ht : s.locals t = some (.ref rt))
    (hv : s.readTarget rt = some (.vector es))
    (hi : s.locals it = some (.u64 n))
    (hlt : n < es.length)
    (hchecked : CheckedState P d s) :
    ∃ sNext', InstrPath tgt s' sNext' ∧
      ImmStackRel
        (⟨frame, d, live,
          immStep d g (.call [dst] .borrowVecElem [t, it])⟩ :: points)
        (s.writeLocal dst (.ref ⟨rt.root, rt.path ++ [n]⟩)) sNext' := by
  let child : RefTarget := ⟨rt.root, rt.path ++ [n]⟩
  let w : Value := es[n]
  have hni : es[n]? = some w := by simp [w, hlt]
  have htLive : t ∈
      liveThroughInstr (.call [dst] .borrowVecElem [t, it]) live :=
    uses_mem_liveThroughInstr (by simp [instrUses])
  have hiLive : it ∈
      liveThroughInstr (.call [dst] .borrowVecElem [t, it]) live :=
    uses_mem_liveThroughInstr (by simp [instrUses])
  have htRange : t < d.numLocals :=
    of_decide_eq_true (List.all_eq_true.mp hrange t
      (by simp [instrDefs, instrUses]))
  have hiRange : it < d.numLocals :=
    of_decide_eq_true (List.all_eq_true.mp hrange it
      (by simp [instrDefs, instrUses]))
  have htFrame : s.frames frame t = some (.ref rt) := by
    simpa [MoveState.locals, hframe] using ht
  have hi' := h.lookup_eq_of_refFree hframe hiLive hiRange
    (v := .u64 n) (by simp) hi
  have hparentRead := h.head.refs_agree t htLive rt htFrame
  have hchildValue : s.readTarget child = some w := by
    rw [show child = ⟨rt.root, rt.path ++ [n]⟩ from rfl,
      MoveState.readTarget_snoc, hv]
    simp [Value.getPath, hni]
  have hchildRead : s'.readTarget child = s.readTarget child := by
    rw [show child = ⟨rt.root, rt.path ++ [n]⟩ from rfl,
      MoveState.readTarget_snoc, MoveState.readTarget_snoc, hparentRead]
  have graphSub : ∀ e, e ∈ g →
      e ∈ immStep d g (.call [dst] .borrowVecElem [t, it]) := by
    intro e he
    simpa [immStep] using
      (mem_gInsertClosed (e' := ⟨.refNode t, dst, []⟩) he)
  have childCover : ImmCovers frame
      (immStep d g (.call [dst] .borrowVecElem [t, it])) dst child := by
    simpa [child, ImmCovers, immStep] using
      (h.head.covers t htLive rt htFrame).copy
  apply h.simulate_derivedBorrow (dst := dst) (src := t) (parent := rt)
      (child := child) (container := .vector es) (w := w) (st := st)
      (st' := st') (sourceInstr := .call [dst] .borrowVecElem [t, it])
      (getInstr := fun x => .call [dst] .vecGet [x, it]) (tgt := tgt)
      hframe hbelow hbase hcheck
      (by simp [instrDefs])
      (fun y hy hydst => live_liveThroughInstr hy
        (by simp [instrDefs, hydst])) graphSub htLive htRange ht htFrame hv
      rfl hchildRead hchildValue
      (fun hfree => Value.refFree_of_getElem? hfree hni) childCover
      (by simpa using elimImmInstr_borrowVecElem_inv hrewrite)
  · intro ht'
    apply InstrNext.op_one (vs := [.vector es, .u64 n])
    · rw [List.mapM_cons, ht', List.mapM_cons, hi']
      rfl
    · simp [Oper.sem, hni]
  · intro tmp htmp
    have hne : it ≠ tmp := by
      intro heq
      subst it
      exact (Nat.not_lt_of_ge htmp) hiRange
    have hiScratch : (s'.writeLocal tmp (.vector es)).locals it =
        some (.u64 n) := by
      rw [MoveState.writeLocal_locals, if_neg hne]
      exact hi'
    apply InstrNext.op_one (vs := [.vector es, .u64 n])
    · rw [List.mapM_cons, show (s'.writeLocal tmp (.vector es)).locals tmp =
          some (.vector es) by simp, List.mapM_cons, hiScratch]
      rfl
    · simp [Oper.sem, hni]
  · intro ht' hread
    exact .borrowVecElem ht' hread hi' hlt
  · exact hchecked

/-- Simulate an aborting immutable-eliminated vector-element borrow. -/
theorem ImmStackRel.simulate_borrowVecElem_abort {P P' : Program} {G : Cfg}
    {d : FunDecl} {live : LiveSet} {g : BGraph}
    {frame : FrameId} {points : List ImmPoint} {s s' : MoveState}
    {dst t it n : Nat} {rt : RefTarget} {es : List Value}
    {st st' : ElimSt} {tgt rest : List Instr} {term : Term}
    (h : ImmStackRel
      (⟨frame, d,
        liveThroughInstr (.call [dst] .borrowVecElem [t, it]) live, g⟩ ::
        points) s s')
    (hframe : frame = s.current)
    (hbase : d.numLocals ≤ st.base)
    (hrange : (instrDefs (.call [dst] .borrowVecElem [t, it]) ++
      instrUses (.call [dst] .borrowVecElem [t, it])).all
        (· < d.numLocals) = true)
    (hrewrite : elimImmInstr d st (.call [dst] .borrowVecElem [t, it]) =
      .ok (st', tgt))
    (ht : s.locals t = some (.ref rt))
    (hv : s.readTarget rt = some (.vector es))
    (hi : s.locals it = some (.u64 n))
    (hge : es.length ≤ n)
    (hchecked : CheckedState P d s) :
    RunFrom P' G (tgt ++ rest) term s'
      (.abort s.memory runtimeAbortCode) := by
  have htLive : t ∈
      liveThroughInstr (.call [dst] .borrowVecElem [t, it]) live :=
    uses_mem_liveThroughInstr (by simp [instrUses])
  have hiLive : it ∈
      liveThroughInstr (.call [dst] .borrowVecElem [t, it]) live :=
    uses_mem_liveThroughInstr (by simp [instrUses])
  have htRange : t < d.numLocals :=
    of_decide_eq_true (List.all_eq_true.mp hrange t
      (by simp [instrDefs, instrUses]))
  have hiRange : it < d.numLocals :=
    of_decide_eq_true (List.all_eq_true.mp hrange it
      (by simp [instrDefs, instrUses]))
  obtain ⟨tv, ht', htval⟩ := h.lookup hframe htLive htRange ht
  have hi' := h.lookup_eq_of_refFree hframe hiLive hiRange
    (v := .u64 n) (by simp) hi
  have hparentRead := h.head.refs_agree t htLive rt
    (by simpa [MoveState.locals, hframe] using ht)
  have hread : s'.readTarget rt = some (.vector es) :=
    hparentRead.trans hv
  have hnone : es[n]? = none := List.getElem?_eq_none hge
  obtain hcase | hcase | hcase := elimImmInstr_borrowVecElem_inv hrewrite
  · obtain ⟨-, htimm, -, rfl⟩ := hcase
    obtain ⟨ty, hty⟩ := isImmLocal_decl htimm
    have hvectorFree := hchecked.consistent.refTarget_free hty rfl ht hv
    rw [htimm] at htval
    rcases htval with ⟨hrefFree, -⟩ | ⟨rt', href, -, hcopied⟩
    · simp at hrefFree
    · cases href
      rw [hv] at hcopied
      cases hcopied
      have hsrcs : [t, it].mapM s'.locals =
          some [.vector es, .u64 n] := by
        rw [List.mapM_cons, ht', List.mapM_cons, hi']
        rfl
      have hop : Oper.vecGet.sem s'.current s'.readTarget
          [.vector es, .u64 n] s'.memory = some .abort := by
        simp [Oper.sem, hnone]
      simpa [h.memory_eq, Oper.abortCode] using
        (InstrStop.run (rest := rest) (term := term) (.op hsrcs hop))
  · obtain ⟨-, htmut, ty, hty, -, rfl⟩ := hcase
    have hvectorFree := hchecked.consistent.refTarget_free hty rfl ht hv
    have htimm := isImmLocal_false_of_isMutLocal htmut
    rw [htimm] at htval
    subst tv
    let tmp := (st.alloc ty).2
    have htmp : d.numLocals ≤ tmp :=
      Nat.le_trans hbase (by simp [tmp, ElimSt.alloc])
    have hfirst : InstrNext (.call [tmp] .readRef [t]) s'
        (s'.writeLocal tmp (.vector es)) :=
      .readRef ht' hread hvectorFree
    have hiScratch : (s'.writeLocal tmp (.vector es)).locals it =
        some (.u64 n) := by
      rw [MoveState.writeLocal_locals, if_neg]
      · exact hi'
      · intro heq
        subst it
        exact (Nat.not_lt_of_ge htmp) hiRange
    have htmpVal : (s'.writeLocal tmp (.vector es)).locals tmp =
        some (.vector es) := by simp
    have hsrcs : [tmp, it].mapM
        (s'.writeLocal tmp (.vector es)).locals =
          some [.vector es, .u64 n] := by
      rw [List.mapM_cons, htmpVal, List.mapM_cons, hiScratch]
      rfl
    have hop : Oper.vecGet.sem (s'.writeLocal tmp (.vector es)).current
        (s'.writeLocal tmp (.vector es)).readTarget
        [.vector es, .u64 n] (s'.writeLocal tmp (.vector es)).memory =
          some .abort := by simp [Oper.sem, hnone]
    exact hfirst.run (by
      simpa [h.memory_eq, Oper.abortCode] using
        (InstrStop.run (rest := rest) (term := term) (.op hsrcs hop)))
  · obtain ⟨-, htimm, -, rfl⟩ := hcase
    rw [htimm] at htval
    subst tv
    simpa [h.memory_eq] using
      (InstrStop.run (rest := rest) (term := term)
        (.borrowVecElem ht' hread hi' hge))

/-- Immutable stack relation specialized to returned frame worlds. -/
abbrev ImmWorldRel (points : List ImmPoint) (w w' : FrameWorld) : Prop :=
  FrameStackWorldRel ImmPoint.frame ImmPoint.Rel points w w'

/-- Resuming related returned worlds produces related running states. -/
theorem ImmWorldRel.resume {points : List ImmPoint} {w w' : FrameWorld}
    (h : ImmWorldRel points w w') (frame : FrameId) :
    ImmStackRel points (w.resume frame) (w'.resume frame) := by
  refine ⟨rfl, h.memory_eq, ?_, ?_⟩
  · intro p hp
    have hpRel := h.tracked p hp
    apply hpRel.changeState
    · rfl
    · rfl
    · intro x hx rt href himm
      rfl
    · intro x hx rt href
      exact hpRel.refs_agree x hx rt href
  · intro other hnone
    simpa [ImmWorldRel, FrameWorld.asState, FrameWorld.resume,
      MoveState.resumeFrame] using h.untracked other hnone

/-- Resume related worlds and write related caller results. -/
theorem ImmWorldRel.resume_writeLocals {p p' : ImmPoint}
    {points : List ImmPoint} {w w' : FrameWorld}
    (h : ImmWorldRel (p :: points) w w') (frame : FrameId)
    (hp : p.frame = frame) (hp' : p'.frame = frame)
    (hbelow : ∀ q ∈ points, q.frame < frame)
    (dsts : List LocalIndex) (vals : List Value)
    (hactive : p'.Rel ((w.resume frame).writeLocals dsts vals)
      ((w'.resume frame).writeLocals dsts vals)) :
    ImmStackRel (p' :: points)
      ((w.resume frame).writeLocals dsts vals)
      ((w'.resume frame).writeLocals dsts vals) :=
  (h.resume frame).writeLocals hp hp' hbelow dsts vals hactive

/-- Finishing a higher active frame preserves a lower immutable frame relation. -/
theorem ImmFrameRel.finish_above {d : FunDecl}
    {live : LiveSet} {g : BGraph} {frame : FrameId}
    {s s' : MoveState} (h : ImmFrameRel d live g frame s s')
    (hlt : frame < s.current) (hcurrent : s'.current = s.current) :
    ImmFrameRel d live g frame s.finishFrame.asState
      s'.finishFrame.asState := by
  have hframe : frame ≠ s.current := Nat.ne_of_lt hlt
  have hframe' : frame ≠ s'.current := by simpa [hcurrent] using hframe
  have sourceRead : ∀ y ∈ live, ∀ rt,
      s.frames frame y = some (.ref rt) →
      s.finishFrame.asState.readTarget rt = s.readTarget rt := by
    intro y hy rt href
    apply FrameWorld.asState_finishFrame_readTarget_of_rootFrame_ne
    intro rootFrame rootLocal hroot
    exact Nat.ne_of_lt (Nat.lt_of_le_of_lt
      (h.roots_below y hy rt href rootFrame rootLocal hroot) hlt)
  have targetRead : ∀ y ∈ live, ∀ rt,
      s.frames frame y = some (.ref rt) →
      s'.finishFrame.asState.readTarget rt = s'.readTarget rt := by
    intro y hy rt href
    apply FrameWorld.asState_finishFrame_readTarget_of_rootFrame_ne
    intro rootFrame rootLocal hroot
    have hrootLe := h.roots_below y hy rt href rootFrame rootLocal hroot
    have : rootFrame < s'.current := by
      rw [hcurrent]
      exact Nat.lt_of_le_of_lt hrootLe hlt
    exact Nat.ne_of_lt this
  apply h.changeState
  · simp [FrameWorld.asState, MoveState.finishFrame, setFrame, hframe]
  · simp [FrameWorld.asState, MoveState.finishFrame, setFrame, hframe']
  · exact fun y hy rt href _ => sourceRead y hy rt href
  · intro y hy rt href
    rw [targetRead y hy rt href, sourceRead y hy rt href]
    exact h.refs_agree y hy rt href

/-- Finish the active immutable-related frame and retain suspended-frame relations. -/
theorem ImmStackRel.finish {p : ImmPoint} {points : List ImmPoint}
    {s s' : MoveState} (h : ImmStackRel (p :: points) s s')
    (hp : p.frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current) :
    ImmWorldRel points s.finishFrame s'.finishFrame := by
  refine ⟨rfl, h.memory_eq, ?_, ?_⟩
  · intro q hq
    exact (h.tracked q (by simp [hq])).finish_above
      (hbelow q hq) h.current_eq
  · intro frame hnone
    by_cases hactive : frame = s.current
    · subst frame
      simp [FrameWorld.asState, MoveState.finishFrame, h.current_eq]
    · have hactive' : frame ≠ s'.current := by
        simpa [h.current_eq] using hactive
      have hold := h.untracked frame (by
        intro q hq
        rcases List.mem_cons.mp hq with rfl | hq
        · simpa [hp] using Ne.symm hactive
        · exact hnone q hq)
      simpa [FrameWorld.asState, MoveState.finishFrame, setFrame,
        hactive, hactive'] using hold

/-- Internal outcomes retain the relation on suspended frames.  Return values
are equal for the immutable pass; the escape check rules out returning an
eliminated immutable reference. -/
inductive ImmOutcomeRel (points : List ImmPoint) :
    FrameOutcome → FrameOutcome → Prop where
  | ret {w w' : FrameWorld} {vals : List Value} :
      ImmWorldRel points w w' →
      ImmOutcomeRel points (.ret w vals) (.ret w' vals)
  | abort {m : Memory} {code : Nat} :
      ImmOutcomeRel points (.abort m code) (.abort m code)

/-- With no suspended frames, related immutable outcomes are equal. -/
theorem ImmOutcomeRel.nil_eq {o o' : FrameOutcome}
    (h : ImmOutcomeRel [] o o') : o' = o := by
  cases h with
  | ret hw => rw [FrameStackWorldRel.nil_eq hw]
  | abort => rfl

/-- Reusable return rule for the immutable simulation.  The caller supplies
only the three static facts produced by liveness, range checking, and the
escape check. -/
theorem ImmStackRel.simulate_ret {P' : Program} {G : Cfg}
    {p : ImmPoint} {points : List ImmPoint} {s s' : MoveState}
    (h : ImmStackRel (p :: points) s s') (hp : p.frame = s.current)
    (hbelow : ∀ q ∈ points, q.frame < s.current)
    {srcs : List LocalIndex} {vals : List Value}
    (hlive : ∀ x ∈ srcs, x ∈ p.live)
    (hrange : ∀ x ∈ srcs, x < p.decl.numLocals)
    (hempty : ∀ x ∈ srcs, isImmLocal p.decl x = true →
      (immAncestors p.graph x).isEmpty = true)
    (hvals : srcs.mapM s.locals = some vals) :
    ∃ o', RunFrom P' G [] (.ret srcs) s' o' ∧
      ImmOutcomeRel points (.ret s.finishFrame vals) o' := by
  have hvals' := h.lookup_eqs_of_imm_empty hp hlive hrange hempty hvals
  exact ⟨.ret s'.finishFrame vals, .ret hvals', .ret (h.finish hp hbelow)⟩

/-- Simulate an explicit abort terminator. -/
theorem ImmStackRel.simulate_abort {P' : Program} {G : Cfg}
    {p : ImmPoint} {points : List ImmPoint} {s s' : MoveState}
    (h : ImmStackRel (p :: points) s s') (hp : p.frame = s.current)
    {code : LocalIndex} (hlive : code ∈ p.live)
    (hrange : code < p.decl.numLocals) {n : Nat}
    (hcode : s.locals code = some (.u64 n)) :
    RunFrom P' G [] (.abort code) s' (.abort s.memory n) ∧
      ImmOutcomeRel points (.abort s.memory n) (.abort s.memory n) := by
  have hcode' := h.lookup_eq_of_refFree hp hlive hrange (v := .u64 n)
    (by simp) hcode
  refine ⟨?_, .abort⟩
  simpa [h.memory_eq] using (RunFrom.abort (P := P') (G := G) hcode')

/-- Reference-free arguments establish the initial immutable frame relation. -/
theorem ImmFrameRel.initial {d : FunDecl} {live : LiveSet}
    {g : BGraph} {m : Memory} {args : List Value}
    (hargs : ∀ v ∈ args, v.refFree) :
    ImmFrameRel d live g 0 (MoveState.initial args m)
      (MoveState.initial args m) := by
  apply ImmFrameRel.refl_of_refFree
  intro x v hv
  apply hargs v
  simp only [MoveState.initial, setFrame_same, initLocals] at hv
  exact List.mem_of_getElem? hv

/-- Reference-free arguments establish the singleton immutable stack relation. -/
theorem ImmStackRel.initial {d : FunDecl} {live : LiveSet}
    {g : BGraph} {m : Memory} {args : List Value}
    (hargs : ∀ v ∈ args, v.refFree) :
    ImmStackRel [⟨0, d, live, g⟩] (MoveState.initial args m)
      (MoveState.initial args m) := by
  refine ⟨rfl, rfl, ?_, ?_⟩
  · intro p hp
    simp only [List.mem_singleton] at hp
    subst p
    exact ImmFrameRel.initial hargs
  · intro frame hnone
    rfl

set_option maxHeartbeats 1000000 in
/-- Master immutable-layer simulation.  The induction follows the six grouped
execution forms; concrete instruction cases delegate to the one-step
certificates above. -/
theorem elimImm_sim {P : Program} (facts : ImmCheckedFacts P)
    (himm : ∀ f d, P.funs f = some d → (elimImmRefs P.funs d).isOk)
    (hsized : ∀ f d, P.funs f = some d →
      ∀ b, d.body.blocks b ≠ none → b < d.body.size)
    {G : Cfg} {is₀ : List Instr} {term₀ : Term} {s : MoveState}
    {o : FrameOutcome}
    (hrun : RunFrom.Invariant P (CheckedStateAt P) G is₀ term₀ s o) :
    ∀ {f : FunId} {d d' : FunDecl} {blk : Block} {b : BlockId}
      {is tgt : List Instr} {live : LiveSet} {g : BGraph}
      {st stEnd : ElimSt} {gEnd : BGraph} {frame : FrameId}
      {points : List ImmPoint} {s' : MoveState},
      P.funs f = some d →
      elimImmRefs P.funs d = .ok d' →
      G = d.body → is₀ = is → term₀ = blk.term →
      d.body.blocks b = some blk →
      ImmSuffix P.funs d (liveAtTermOf d blk) is live g st tgt stEnd →
      d.numLocals ≤ st.base →
      checkTermRange d blk.term = .ok () →
      checkRetEscape d gEnd blk.term = .ok () →
      gEnd = is.foldl (immStep d) g →
      gEnd = blk.instrs.foldl (immStep d)
        ((immAnalysis d).getD b []) →
      ImmStackRel (⟨frame, d, live, g⟩ :: points) s s' →
      frame = s.current →
      (∀ p ∈ points, p.frame < s.current) →
      ∃ o', RunFrom (immProgram P) d'.body tgt blk.term s' o' ∧
        ImmOutcomeRel points o o' := by
  induction hrun with
  | instrNext ok head restRun ih =>
      intro f d d' blk b is tgt live g st stEnd gEnd frame points s'
        hf helim hG his hterm hblk hsuffix hbase htermRange hescape hfinish
        hgEnd
        hrel hframe hbelow
      subst is
      cases hsuffix with
      | @cons i rest liveNext graph state stateNext stateEnd targetHead
          targetRest hrange hboundary hcheck hrewrite hsuffixRest =>
        cases head with
        | load =>
            have hstate := (elimImmInstr_load_inv hrewrite).1
            subst stateNext
            obtain ⟨targetNext, hpath, hrelNext⟩ :=
              hrel.simulate_load hframe hbelow hcheck hrewrite
            obtain ⟨targetOutcome, htarget, hout⟩ := ih hf helim hG rfl hterm
              hblk hsuffixRest hbase
              htermRange hescape (by simpa using hfinish) hgEnd hrelNext
              (by simpa using hframe)
              (by simpa using hbelow)
            exact ⟨targetOutcome, hpath.run htarget, hout⟩
        | assign hsrc =>
            have hstate := (elimImmInstr_assign_inv hrewrite).1
            subst stateNext
            obtain ⟨targetNext, hpath, hrelNext⟩ :=
              hrel.simulate_assign hframe hbelow hrange hcheck hrewrite hsrc
            obtain ⟨targetOutcome, htarget, hout⟩ := ih hf helim hG rfl hterm
              hblk hsuffixRest hbase
              htermRange hescape (by simpa using hfinish) hgEnd hrelNext
              (by simpa using hframe)
              (by simpa using hbelow)
            exact ⟨targetOutcome, hpath.run htarget, hout⟩
        | nop =>
            have hstate := (elimImmInstr_nop_inv hrewrite).1
            subst stateNext
            obtain ⟨targetNext, hpath, hrelNext⟩ :=
              hrel.simulate_nop hrewrite
            obtain ⟨targetOutcome, htarget, hout⟩ := ih hf helim hG rfl hterm
              hblk hsuffixRest hbase
              htermRange hescape (by simpa using hfinish) hgEnd hrelNext
              hframe hbelow
            exact ⟨targetOutcome, hpath.run htarget, hout⟩
        | op hsrcs hlen hop =>
            obtain ⟨hsafe, hsourceSafe⟩ :=
              facts.op hrel hframe hcheck hop
            have hstate := (elimImmInstr_op_inv hop hrewrite).1
            have hbaseNext : d.numLocals ≤ stateNext.base := by
              rw [hstate]
              exact hbase
            obtain ⟨targetNext, hpath, hrelNext⟩ :=
              hrel.simulate_op hframe hbelow hrange hcheck hrewrite hsrcs
                hlen hop hsafe hsourceSafe
            obtain ⟨targetOutcome, htarget, hout⟩ := ih hf helim hG rfl hterm
              hblk hsuffixRest hbaseNext htermRange hescape
              (by simpa using hfinish) hgEnd hrelNext
              (by simpa using hframe) (by simpa using hbelow)
            exact ⟨targetOutcome, hpath.run htarget, hout⟩
        | isParentMissing hp ht =>
            simp [elimImmInstr, throw, throwThe, MonadExceptOf.throw] at hrewrite
        | @borrowLoc _ dst x v hx =>
            have hchecked := ok f d hf hG
            have hxPlain := immCheck_borrowLoc_source_plain hcheck
            have hdstFree : isImmLocal d dst = true → v.refFree := by
              intro hdstImm
              rcases (elimImmInstr_borrowLoc_inv hrewrite).2 with hcase | hcase
              · obtain ⟨_, _, hxMut, -⟩ := hcase
                have hxRange : x < d.numLocals :=
                  of_decide_eq_true (List.all_eq_true.mp hrange x
                    (by simp [instrDefs, instrUses]))
                obtain ⟨ty, hty⟩ := hchecked.consistent.declared x hxRange
                have href : ty.isRef = false := by
                  unfold isImmLocal at hxPlain
                  unfold isMutLocal at hxMut
                  rw [hty] at hxPlain hxMut
                  cases ty <;> simp_all [Ty.isRef]
                exact hchecked.consistent.plain_free hty href hx
              · rw [hcase.1] at hdstImm
                cases hdstImm
            have hstate := (elimImmInstr_borrowLoc_inv hrewrite).1
            obtain ⟨targetNext, hpath, hrelNext⟩ :=
              hrel.simulate_borrowLoc hframe hbelow hrange hcheck hrewrite
                ⟨hxPlain, hdstFree⟩ hx
            obtain ⟨targetOutcome, htarget, hout⟩ := ih hf helim hG rfl hterm
              hblk hsuffixRest (by rw [hstate]; exact hbase) htermRange
              hescape (by simpa using hfinish) hgEnd hrelNext
              (by simpa using hframe)
              (by simpa using hbelow)
            exact ⟨targetOutcome, hpath.run htarget, hout⟩
        | borrowField ht hs hi =>
            have hchecked := ok f d hf hG
            obtain ⟨targetNext, hpath, hrelNext⟩ :=
              hrel.simulate_borrowField hframe hbelow hbase hrange hcheck
                hrewrite ht hs hi hchecked
            have hbaseNext : d.numLocals ≤ stateNext.base := by
              rcases elimImmInstr_borrowField_inv hrewrite with h | h | h
              · rw [h.2.2.1]; exact hbase
              · obtain ⟨_, _, ty, _, hstate, -⟩ := h
                rw [hstate]
                simpa [ElimSt.alloc] using hbase
              · rw [h.2.2.1]; exact hbase
            obtain ⟨targetOutcome, htarget, hout⟩ := ih hf helim hG rfl hterm
              hblk hsuffixRest hbaseNext htermRange hescape
              (by simpa using hfinish) hgEnd hrelNext
              (by simpa using hframe) (by simpa using hbelow)
            exact ⟨targetOutcome, hpath.run htarget, hout⟩
        | borrowFieldInst ht hs hi =>
            simp [elimImmInstr, throw, throwThe, MonadExceptOf.throw] at hrewrite
        | borrowGlobal ha hpresent =>
            have hfree := (ok f d hf hG).consistent.memory _ _ _ hpresent
            have hstate := (elimImmInstr_borrowGlobal_inv hrewrite).1
            obtain ⟨targetNext, hpath, hrelNext⟩ :=
              hrel.simulate_borrowGlobal hframe hbelow hrange hcheck hrewrite
                ha hfree hpresent
            obtain ⟨targetOutcome, htarget, hout⟩ := ih hf helim hG rfl hterm
              hblk hsuffixRest (by rw [hstate]; exact hbase) htermRange
              hescape (by simpa using hfinish) hgEnd hrelNext
              (by simpa using hframe)
              (by simpa using hbelow)
            exact ⟨targetOutcome, hpath.run htarget, hout⟩
        | borrowGlobalInst ha hpresent =>
            simp [elimImmInstr, throw, throwThe, MonadExceptOf.throw] at hrewrite
        | borrowVecElem ht hv hi hlt =>
            have hchecked := ok f d hf hG
            obtain ⟨targetNext, hpath, hrelNext⟩ :=
              hrel.simulate_borrowVecElem hframe hbelow hbase hrange hcheck
                hrewrite ht hv hi hlt hchecked
            have hbaseNext : d.numLocals ≤ stateNext.base := by
              rcases elimImmInstr_borrowVecElem_inv hrewrite with h | h | h
              · rw [h.2.2.1]; exact hbase
              · obtain ⟨_, _, ty, _, hstate, -⟩ := h
                rw [hstate]
                simpa [ElimSt.alloc] using hbase
              · rw [h.2.2.1]; exact hbase
            obtain ⟨targetOutcome, htarget, hout⟩ := ih hf helim hG rfl hterm
              hblk hsuffixRest hbaseNext htermRange hescape
              (by simpa using hfinish) hgEnd hrelNext
              (by simpa using hframe) (by simpa using hbelow)
            exact ⟨targetOutcome, hpath.run htarget, hout⟩
        | readRef ht hv hfree =>
            have hstate := (elimImmInstr_readRef_inv hrewrite).1
            obtain ⟨targetNext, hpath, hrelNext⟩ :=
              hrel.simulate_readRef hframe hbelow hrange hcheck hrewrite ht hv
                hfree
            obtain ⟨targetOutcome, htarget, hout⟩ := ih hf helim hG rfl hterm
              hblk hsuffixRest (by rw [hstate]; exact hbase) htermRange
              hescape (by simpa using hfinish) hgEnd hrelNext
              (by simpa using hframe)
              (by simpa using hbelow)
            exact ⟨targetOutcome, hpath.run htarget, hout⟩
        | writeRef ht hv hfree hwrite =>
            have hsafe := facts.writeRef hrel hframe hcheck hwrite
            have hstate := (elimImmInstr_writeRef_inv hrewrite).1
            have hcurrent := MoveState.writeTarget_current hwrite
            obtain ⟨targetNext, hpath, hrelNext⟩ :=
              hrel.simulate_writeRef hframe hrange hrewrite ht hv hfree hwrite
                hsafe
            obtain ⟨targetOutcome, htarget, hout⟩ := ih hf helim hG rfl hterm
              hblk hsuffixRest (by rw [hstate]; exact hbase) htermRange
              hescape (by simpa using hfinish) hgEnd hrelNext
              (by rw [hcurrent]; exact hframe)
              (by rw [hcurrent]; exact hbelow)
            exact ⟨targetOutcome, hpath.run htarget, hout⟩
        | freezeRef ht hv hfree =>
            have hstate := (elimImmInstr_freezeRef_inv hrewrite).1
            obtain ⟨targetNext, hpath, hrelNext⟩ :=
              hrel.simulate_freezeRef hframe hbelow hrange hcheck hrewrite ht
                hv hfree
            obtain ⟨targetOutcome, htarget, hout⟩ := ih hf helim hG rfl hterm
              hblk hsuffixRest (by rw [hstate]; exact hbase) htermRange
              hescape (by simpa using hfinish) hgEnd hrelNext
              (by simpa using hframe)
              (by simpa using hbelow)
            exact ⟨targetOutcome, hpath.run htarget, hout⟩
  | instrStop ok head =>
      intro f d d' blk b is tgt live g st stEnd gEnd frame points s'
        hf helim hG his hterm hblk hsuffix hbase htermRange hescape hfinish
        hgEnd
        hrel hframe hbelow
      subst is
      cases hsuffix with
      | @cons i rest liveNext graph state stateNext stateEnd targetHead
          targetRest hrange hboundary hcheck hrewrite hsuffixRest =>
        cases head with
        | op hsrcs hop =>
            have htarget := hrel.simulate_op_abort
              (P' := immProgram P) (G := d'.body) (rest := targetRest)
              (term := blk.term) hframe hrange hrewrite
              hsrcs hop (Oper.sem_immOperandsSafe hop)
            exact ⟨_, htarget, .abort⟩
        | borrowGlobal ha habsent =>
            have htarget := hrel.simulate_borrowGlobal_abort
              (P' := immProgram P) (G := d'.body) (rest := targetRest)
              (term := blk.term) hframe hrange hrewrite
              ha habsent
            exact ⟨_, htarget, .abort⟩
        | borrowGlobalInst ha habsent =>
            simp [elimImmInstr, throw, throwThe, MonadExceptOf.throw] at hrewrite
        | borrowVecElem ht hv hi hge =>
            have hchecked := ok f d hf hG
            have htarget := hrel.simulate_borrowVecElem_abort
              (P' := immProgram P) (G := d'.body) (rest := targetRest)
              (term := blk.term) hframe hbase hrange
              hrewrite ht hv hi hge hchecked
            exact ⟨_, htarget, .abort⟩
  | @callOk runG rest term runState dsts srcs calleeId callee args retVals
      entryBlk world runOutcome ok decl argsRead arity entry calleeRun rets
      restRun ihCallee ihRest =>
      intro f d d' blk b is tgt live g st stEnd gEnd frame points s'
        hf helim hG his hterm hblk hsuffix hbase htermRange hescape hfinish
        hgEnd hrel hframe hbelow
      subst is
      cases hsuffix with
      | @cons i rest' liveNext graph state stateNext stateEnd targetHead
          targetRest hrange hboundary hcheck hrewrite hsuffixRest =>
        obtain ⟨rfl, rfl⟩ := elimImmInstr_function_inv hrewrite
        have hcalleeOk := himm calleeId callee decl
        cases helimC : elimImmRefs P.funs callee with
        | error e => simp [helimC, Except.isOk, Except.toBool] at hcalleeOk
        | ok callee' =>
          have hinvC := elimImmRefs_inv helimC
          have hentryLt := hsized calleeId callee decl callee.body.entry
            (by simp [entry])
          obtain ⟨entryTarget, entrySt, entryEnd, entryGraph, entrySuffix,
              entryBase, entryTargetBlk, entryRange, entryEscape,
              entryFinish⟩ := hinvC.block hentryLt entry
          have hentrySafe := facts.callEntry
            (live := liveBeforeSuffix (liveAtTermOf callee entryBlk)
              entryBlk.instrs)
            (graph := (immAnalysis callee).getD callee.body.entry [])
            hrel hframe hboundary decl argsRead
          obtain ⟨args', hargs', hargsLen, hrelEntry⟩ :=
            hrel.callEntry hframe hbelow
            (fun x hx => uses_mem_liveThroughInstr (by
              simpa [instrUses] using hx)) hrange hboundary decl argsRead
            hentrySafe
          have hcalleeBelow : ∀ p ∈
              (⟨frame, d,
                liveThroughInstr (.call dsts (.function calleeId) srcs)
                  liveNext, g⟩ :: points),
              p.frame < (runState.enterCall args).current := by
            intro p hp
            rcases List.mem_cons.mp hp with rfl | hp
            · simp [hframe]
            · simpa [MoveState.enterCall] using
                Nat.lt_trans (hbelow p hp) (Nat.lt_succ_self _)
          obtain ⟨calleeOutcome, hcalleeTarget, hcalleeRel⟩ :=
            ihCallee decl helimC rfl rfl rfl entry entrySuffix entryBase
              entryRange entryEscape entryFinish entryFinish hrelEntry
              (by simp) hcalleeBelow
          cases hcalleeRel with
          | @ret sourceWorld targetWorld values hworld =>
            have hresume := hworld.resume frame
            have hcheckedAfter := restRun.start f d hf hG
            have hcheckedAfter' : CheckedState P d
                ((world.resume frame).writeLocals dsts retVals) := by
              simpa [hframe] using hcheckedAfter
            have hactive := facts.callReturn (vals := retVals) hresume
              (by simp [FrameWorld.resume, MoveState.resumeFrame]) hcheck
              hcheckedAfter'
            have hrelAfter := hworld.resume_writeLocals
              (p := ⟨frame, d,
                liveThroughInstr (.call dsts (.function calleeId) srcs)
                  liveNext, g⟩)
              (p' := ⟨frame, d, liveNext,
                immStep d g (.call dsts (.function calleeId) srcs)⟩)
              frame rfl rfl
              (by intro q hq; simpa [hframe] using hbelow q hq)
              dsts retVals hactive
            have hrelAfter' : ImmStackRel
                (⟨runState.current, d, liveNext,
                  immStep d g (.call dsts (.function calleeId) srcs)⟩ ::
                  points)
                ((world.resume runState.current).writeLocals dsts retVals)
                ((targetWorld.resume frame).writeLocals dsts retVals) := by
              simpa [hframe] using hrelAfter
            obtain ⟨targetOutcome, htargetRest, hout⟩ := ihRest hf helim hG
              rfl hterm hblk hsuffixRest hbase htermRange hescape
              (by simpa using hfinish) hgEnd hrelAfter'
              (by simp [FrameWorld.resume, MoveState.resumeFrame])
              (by simpa [FrameWorld.resume, MoveState.resumeFrame] using
                hbelow)
            have hframeTarget : frame = s'.current :=
              hframe.trans hrel.current_eq.symm
            have htargetRest' : RunFrom (immProgram P) d'.body targetRest
                blk.term
                ((targetWorld.resume s'.current).writeLocals dsts retVals)
                targetOutcome := by
              simpa [hframeTarget] using htargetRest
            refine ⟨targetOutcome, ?_, hout⟩
            exact .callOk (immProgram_fun decl helimC) hargs'
              (by rw [hargsLen, arity, hinvC.numParams_eq])
              (by simpa [hinvC.entry_eq] using entryTargetBlk)
              hcalleeTarget rets htargetRest'
  | @callAbort runG rest term runState dsts srcs calleeId callee args
      entryBlk abortMem abortCode ok decl argsRead arity entry calleeRun
      ihCallee =>
      intro f d d' blk b is tgt live g st stEnd gEnd frame points s'
        hf helim hG his hterm hblk hsuffix hbase htermRange hescape hfinish
        hgEnd hrel hframe hbelow
      subst is
      cases hsuffix with
      | @cons i rest' liveNext graph state stateNext stateEnd targetHead
          targetRest hrange hboundary hcheck hrewrite hsuffixRest =>
        obtain ⟨rfl, rfl⟩ := elimImmInstr_function_inv hrewrite
        have hcalleeOk := himm calleeId callee decl
        cases helimC : elimImmRefs P.funs callee with
        | error e => simp [helimC, Except.isOk, Except.toBool] at hcalleeOk
        | ok callee' =>
          have hinvC := elimImmRefs_inv helimC
          have hentryLt := hsized calleeId callee decl callee.body.entry
            (by simp [entry])
          obtain ⟨entryTarget, entrySt, entryEnd, entryGraph, entrySuffix,
              entryBase, entryTargetBlk, entryRange, entryEscape,
              entryFinish⟩ := hinvC.block hentryLt entry
          have hentrySafe := facts.callEntry
            (live := liveBeforeSuffix (liveAtTermOf callee entryBlk)
              entryBlk.instrs)
            (graph := (immAnalysis callee).getD callee.body.entry [])
            hrel hframe hboundary decl argsRead
          obtain ⟨args', hargs', hargsLen, hrelEntry⟩ :=
            hrel.callEntry hframe hbelow
              (fun x hx => uses_mem_liveThroughInstr (by
                simpa [instrUses] using hx)) hrange hboundary decl argsRead
              hentrySafe
          have hcalleeBelow : ∀ p ∈
              (⟨frame, d,
                liveThroughInstr (.call dsts (.function calleeId) srcs)
                  liveNext, g⟩ :: points),
              p.frame < (runState.enterCall args).current := by
            intro p hp
            rcases List.mem_cons.mp hp with rfl | hp
            · simp [hframe]
            · simpa [MoveState.enterCall] using
                Nat.lt_trans (hbelow p hp) (Nat.lt_succ_self _)
          obtain ⟨calleeOutcome, hcalleeTarget, hcalleeRel⟩ :=
            ihCallee decl helimC rfl rfl rfl entry entrySuffix entryBase
              entryRange entryEscape entryFinish entryFinish hrelEntry
              (by simp) hcalleeBelow
          cases hcalleeRel with
          | abort =>
            refine ⟨.abort abortMem abortCode, ?_, .abort⟩
            exact .callAbort (immProgram_fun decl helimC) hargs'
              (by rw [hargsLen, arity, hinvC.numParams_eq])
              (by simpa [hinvC.entry_eq] using entryTargetBlk)
              hcalleeTarget
  | @termNext runG runTerm runState nextB nextBlk runOutcome ok head next ih =>
      intro f d d' blk b is tgt live g st stEnd gEnd frame points s'
        hf helim hG his hterm hblk hsuffix hbase htermRange hescape hfinish
        hgEnd hrel hframe hbelow
      subst is
      cases hsuffix with
      | nil =>
        have hfinish' : gEnd = g := by simpa using hfinish
        have hinv := elimImmRefs_inv helim
        have hblt := hsized f d hf b (by simp [hblk])
        have hsucc := head.succ_mem
        have hsuccBlk' : d.body.blocks nextB = some nextBlk := by
          simpa [hG] using head.block
        have hslt := hsized f d hf nextB (by simp [hsuccBlk'])
        obtain ⟨target, succSt, succEnd, succGraph, succSuffix,
            succBase, targetBlk, succRange, succEscape, succFinish⟩ :=
          hinv.block hslt hsuccBlk'
        have hactive := hrel.head.to_successor hinv hblk hblt
          (by simpa [hterm] using hsucc) hsuccBlk' hslt
          (hfinish'.symm.trans hgEnd)
        have hactive' : (⟨frame, d,
            liveBeforeSuffix (liveAtTermOf d nextBlk) nextBlk.instrs,
            (immAnalysis d).getD nextB []⟩ : ImmPoint).Rel runState s' :=
          hactive
        have hrelNext := hrel.replaceHead (p' :=
          ⟨frame, d,
            liveBeforeSuffix (liveAtTermOf d nextBlk) nextBlk.instrs,
            (immAnalysis d).getD nextB []⟩) rfl hactive'
        obtain ⟨targetOutcome, htarget, hout⟩ := ih hf helim hG rfl rfl
          hsuccBlk' succSuffix succBase succRange succEscape succFinish
          succFinish hrelNext hframe hbelow
        refine ⟨targetOutcome, ?_, hout⟩
        rw [← hterm]
        have htargetHead := head.transport targetBlk (by
          intro x taken hx hvalue
          apply hrel.lookup_eq_of_refFree hframe
          · apply termReads_mem_liveAtTermIn
            simpa [hterm] using hx
          · apply checkTermRange_lt htermRange
            simpa [hterm] using hx
          · simp
          · exact hvalue)
        exact htargetHead.run htarget
  | termStop ok head =>
      intro f d d' blk b is tgt live g st stEnd gEnd frame points s'
        hf helim hG his hterm hblk hsuffix hbase htermRange hescape hfinish
        hgEnd hrel hframe hbelow
      subst is
      cases hsuffix with
      | nil =>
        have hfinish' : gEnd = g := by simpa using hfinish
        rw [hfinish'] at hescape
        cases head with
        | @ret _ srcs vals hvals =>
            have hescape' := hescape
            rw [← hterm] at hescape'
            have hlive : ∀ x ∈ srcs, x ∈ liveAtTermOf d blk := by
              intro x hx
              apply termReads_mem_liveAtTermIn
              simpa [termReads, ← hterm] using hx
            have hrange : ∀ x ∈ srcs, x < d.numLocals := by
              intro x hx
              apply checkTermRange_lt htermRange
              rw [← hterm]
              simpa [termReads] using hx
            have hempty : ∀ x ∈ srcs, isImmLocal d x = true →
                (immAncestors g x).isEmpty = true := by
              intro x hx himm
              exact checkRetEscape_imm_empty hescape' hx himm
            obtain ⟨targetOutcome, htarget, hout⟩ := hrel.simulate_ret
              (P' := immProgram P) (G := d'.body) (srcs := srcs)
              (vals := vals) hframe hbelow hlive hrange hempty hvals
            refine ⟨targetOutcome, ?_, hout⟩
            rw [← hterm]
            exact htarget
        | @abort _ code n hcode =>
            obtain ⟨htarget, hout⟩ := hrel.simulate_abort
              (P' := immProgram P) (G := d'.body) (code := code) hframe
              (by
                apply termReads_mem_liveAtTermIn
                simp [termReads, ← hterm])
              (checkTermRange_lt htermRange (by
                rw [← hterm]
                simp [termReads])) hcode
            refine ⟨_, ?_, hout⟩
            rw [← hterm]
            exact htarget

/-! ## Layer theorems -/

/-- Immutable-reference elimination preserves a boundary execution exactly
and transports its checked-state decoration to the intermediate program.
The internal simulation relates immutable-reference locals to copies and
tracks passed reference roots by their stable frame-qualified identity. -/
theorem elimImm_correct (P : Program)
    (himm : ∀ g d, P.funs g = some d → (elimImmRefs P.funs d).isOk)
    (facts : ImmCheckedFacts P)
    (checked' : CheckedProgram (immProgram P))
    (f : FunId) (m : Memory) (args : List Value)
    (input' : CheckedInput (immProgram P) f m args)
    (hargs : ∀ v ∈ args, v.refFree)
    (o : FrameOutcome) (hexec : CheckedExecution P f m args o) :
    CheckedExecution (immProgram P) f m args o := by
  have hsized := hexec.blocksLt
  obtain ⟨d, entryBlk, hd, hentry, hinitial⟩ := hexec.initial
  obtain ⟨runDecl, runDeclEq, runArity, runBlk, runBlkEq, hrun⟩ :=
    hexec.execution
  rw [hd] at runDeclEq
  cases runDeclEq
  rw [hentry] at runBlkEq
  cases runBlkEq
  cases helim : elimImmRefs P.funs d with
  | error e =>
      have := himm f d hd
      simp [helim, Except.isOk, Except.toBool] at this
  | ok d' =>
    have hinv := elimImmRefs_inv helim
    obtain ⟨tgt, st, stEnd, gEnd, hsuffix, hbase, htargetBlk,
        htermRange, hescape, hfinish⟩ :=
      hinv.block (hsized f d hd d.body.entry (by simp [hentry])) hentry
    obtain ⟨targetOutcome, htarget, hout⟩ := elimImm_sim facts himm hsized
      hrun hd helim rfl rfl rfl hentry hsuffix hbase
      htermRange hescape hfinish hfinish (ImmStackRel.initial hargs) rfl
      (by simp)
    have heq := hout.nil_eq
    subst targetOutcome
    have hraw : FunExec (immProgram P) f m args o :=
      ⟨d', immProgram_fun hd helim, by
        rw [hinv.numParams_eq]
        exact runArity, ⟨⟨tgt, entryBlk.term⟩,
        by simpa [hinv.entry_eq] using htargetBlk, htarget⟩⟩
    exact checked'.executionOf input' hraw

/-- Mutation-value elimination simulates a checked intermediate execution.
Normal returns have completed every write-back; on abort, only the code is
observable. -/
theorem elimCore_correct (P₁ P' : Program) (h : CoreProgram P₁ P')
    (facts : CoreCheckedFacts P₁)
    (f : FunId) (m : Memory) (args : List Value)
    (hargs : ∀ v ∈ args, v.refFree)
    (o : FrameOutcome) (hexec : CheckedExecution P₁ f m args o) :
    ∃ o', FunExec P' f m args o' ∧ AgreeOutcome o o' := by
  have hsized := hexec.blocksLt
  obtain ⟨d, hd, harity, entryBlk, hentry, hrun⟩ := hexec.execution
  rcases h.2.2 f with ⟨hnone, -⟩ |
      ⟨sourceDecl, targetDecl, summary, hsource, hsummary, helim, htarget⟩
  · rw [hd] at hnone
    contradiction
  · rw [hd] at hsource
    cases hsource
    have hinv := elimCore_inv helim
    obtain ⟨targetBlk, before, after, htargetBlk, hblock⟩ :=
      hinv.block (hsized f d hd d.body.entry (by simp [hentry])) hentry
    let graph := (borrowAnalysis noSummaries d).getD d.body.entry []
    let pending := coreEntryPending d (liveAnalysis d) d.body.entry graph
    have hchecked : CheckedState P₁ d (MoveState.initial args m) :=
      hrun.start f d hd rfl
    obtain ⟨hframe, hready⟩ := facts.initial hchecked hargs
      (g := graph) (pending := pending) rfl rfl
    obtain ⟨gEnd, pendingEnd, stInstr, htrace, hfinish⟩ :=
      hblock.instrTrace
    obtain ⟨hcfg, hbase⟩ := facts.emitted hinv hblock
    have hpoints :
        (entryBlk.instrs.zip
          (liveAfterEach (liveAtTermIn (liveAnalysis d) entryBlk)
            entryBlk.instrs)).map Prod.fst = entryBlk.instrs := by
      exact List.map_fst_zip (l₁ := entryBlk.instrs) (l₂ :=
          liveAfterEach (liveAtTermIn (liveAnalysis d) entryBlk)
            entryBlk.instrs) (by simp)
    obtain ⟨targetOutcome, hprefix, hagree⟩ := coreSimAt facts hrun h hd
      htarget hinv rfl hentry rfl hpoints htrace hfinish hcfg hbase hchecked
      hframe hready
    obtain ⟨targetRest, targetTerm, htargetAt, htargetRun⟩ := hprefix
    simp only [prepareCoreBlock, List.nil_append] at htargetAt
    have htargetEq : targetBlk = ⟨targetRest, targetTerm⟩ :=
      Option.some.inj (htargetBlk.symm.trans htargetAt)
    subst targetBlk
    have htargetEntry : targetDecl.body.blocks targetDecl.body.entry =
        some ⟨targetRest, targetTerm⟩ := by
      rw [hinv.entry_eq]
      exact htargetBlk
    exact ⟨targetOutcome,
      ⟨targetDecl, htarget,
        by rw [hinv.numParams_eq]; exact harity,
        ⟨⟨targetRest, targetTerm⟩, htargetEntry, htargetRun⟩⟩,
      hagree⟩

/-! ## Layer composition -/

theorem refElimFun_inv {sigs : FunId → Option FunDecl}
    {Δ : StructDecls} {d d' : FunDecl}
    (h : refElimFun sigs Δ d = .ok d') :
    ∃ d₁ s, elimImmRefs sigs d = .ok d₁ ∧
      summarize noSummaries d₁ = .ok s ∧
      elimCore noSummaries Δ d₁ = .ok d' := by
  unfold refElimFun at h
  cases helim : elimImmRefs sigs d with
  | error e => simp [helim, Except.bind, bind] at h
  | ok d₁ =>
    cases hsum : summarize noSummaries d₁ with
    | error e => simp [helim, hsum, Except.bind, bind] at h
    | ok s =>
      exact ⟨d₁, s, rfl, hsum, by
        simpa [helim, hsum, Except.bind, bind] using h⟩

/-- Reference elimination is a forward simulation, obtained by composing
the immutable-copy and mutation-value layers. -/
theorem refElim_correct (P P' : Program) (h : ElimProgram P P')
    (checked : CheckedProgram P)
    (checkedImm : CheckedProgram (immProgram P))
    (facts : ImmCheckedFacts P)
    (coreFacts : CoreCheckedFacts (immProgram P))
    (f : FunId) (m : Memory) (args : List Value)
    (input : CheckedInput P f m args)
    (inputImm : CheckedInput (immProgram P) f m args)
    (hargs : ∀ v ∈ args, v.refFree)
    (o : FrameOutcome) (hexec : FunExec P f m args o) :
    ∃ o', FunExec P' f m args o' ∧ AgreeOutcome o o' := by
  have himm : ∀ g d, P.funs g = some d →
      (elimImmRefs P.funs d).isOk := by
    intro g d hg
    rcases h.2 g with ⟨hnone, -⟩ | ⟨e, e', he, hok, -⟩
    · rw [hg] at hnone
      exact absurd hnone (by simp)
    · rw [hg] at he
      cases he
      obtain ⟨d₁, -, helim, -, -⟩ := refElimFun_inv hok
      simp [helim, Except.isOk, Except.toBool]
  have checked₁ : CheckedExecution (immProgram P) f m args o :=
    elimImm_correct P himm facts checkedImm f m args inputImm hargs o
      (checked.executionOf input hexec)
  have hcore : CoreProgram (immProgram P) P' := by
    refine ⟨⟨P, rfl⟩, h.1.trans ?_, ?_⟩
    · rfl
    · intro g
      rcases h.2 g with ⟨hnone, hnone'⟩ | ⟨d, d', hd, helim, hd'⟩
      · left
        simp [immProgram, hnone, hnone']
      · right
        obtain ⟨d₁, s, himm₁, hsum, hcore₁⟩ := refElimFun_inv helim
        exact ⟨d₁, d', s, by
          simp [immProgram, hd, himm₁, Except.toOption], hsum, hcore₁,
          hd'⟩
  exact elimCore_correct (immProgram P) P' hcore coreFacts f m args hargs o
    checked₁

end MoveModel.IR
