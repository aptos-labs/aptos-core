-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefConc

/-!
# The reference-safety analysis, instruction by instruction

UNTRUSTED (checked by the kernel). What the transfer function of the
reference-safety pass (`ReferenceSafety.executeInner`) does to an abstract
state, when it succeeds: the checks it passed, the borrow-graph operations
it performed, and the resulting locals, operand stack and next id.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

theorem run_bind {α β : Type} {x : RefM α} {f : α → RefM β} {st st' : RefState} {b : β} :
    (x >>= f).run st = .ok (b, st') ↔ ∃ a st₁, x.run st = .ok (a, st₁) ∧ (f a).run st₁ = .ok (b, st') := by
  simp only [StateT.run, bind, StateT.bind, Except.bind]
  constructor
  · intro h
    cases hx : x st with
    | error e => rw [hx] at h; cases h
    | ok p => rw [hx] at h; exact ⟨p.1, p.2, rfl, h⟩
  · rintro ⟨a, st₁, hx, hf⟩
    rw [hx]; exact hf

theorem run_pure {α : Type} {a b : α} {st st' : RefState} :
    (pure a : RefM α).run st = .ok (b, st') ↔ a = b ∧ st = st' := by
  simp [StateT.run, pure, StateT.pure, Except.pure]

theorem run_get {st st' s : RefState} : (get : RefM RefState).run st = .ok (s, st') ↔ s = st ∧ st' = st := by
  simp [StateT.run, get, getThe, MonadStateOf.get, StateT.get, pure, Except.pure, eq_comm]

theorem run_set {st st' s : RefState} : (set s : RefM PUnit).run st = .ok ((), st') ↔ st' = s := by
  simp [StateT.run, set, StateT.set, pure, Except.pure, eq_comm]

theorem run_lift {α : Type} {x : VResult α} {a : α} {st st' : RefState} :
    (liftM x : RefM α).run st = .ok (a, st') ↔ x = .ok a ∧ st' = st := by
  simp only [StateT.run, liftM, monadLift, MonadLift.monadLift, StateT.lift, bind, Except.bind]
  cases x <;> simp [pure, Except.pure, eq_comm]

/-! ## The analysis's primitives -/

theorem query_ok {α : Type} {f : Graph → VResult α} {a : α} {st st' : RefState} :
    (query f).run st = .ok (a, st') ↔ f st.graph = .ok a ∧ st' = st := by
  unfold query
  simp only [StateT.run, bind, StateT.bind, get, getThe, MonadStateOf.get, StateT.get, liftM,
    monadLift, MonadLift.monadLift, StateT.lift, Except.bind, pure, Except.pure]
  cases hf : f st.graph <;> simp [eq_comm]

theorem graphOp_ok {f : Graph → VResult Graph} {st st' : RefState} :
    (graphOp f).run st = .ok ((), st') ↔ ∃ g, f st.graph = .ok g ∧ st' = { st with graph := g } := by
  unfold graphOp
  simp only [StateT.run, bind, StateT.bind, get, getThe, MonadStateOf.get, StateT.get, liftM,
    monadLift, MonadLift.monadLift, StateT.lift, Except.bind, pure, Except.pure, set,
    StateT.set]
  cases hf : f st.graph <;> simp [eq_comm]

/-! ## Instructions -/

/-- The definitions every instruction lemma unfolds. -/
macro "unfold_ref" : tactic => `(tactic| simp only [StateT.run, bind, StateT.bind, get, getThe,
  MonadStateOf.get, StateT.get, liftM, monadLift, MonadLift.monadLift, StateT.lift, Except.bind,
  pure, Except.pure, set, StateT.set, modify, modifyGet, MonadStateOf.modifyGet, StateT.modifyGet,
  push, pop, popRef, popValue, newRef, release, releaseValue, addCopy, addBorrow, addFieldBorrow,
  addLocalBorrow, addResourceBorrow, isMutable, isWritable, isReadable, isFreezable,
  isLocalBorrowed, isLocalMutablyBorrowed, isGlobalBorrowed, isGlobalMutablyBorrowed,
  ReferenceSafety.localAt,
  ReferenceSafety.setLocal, frameRoot, graphOp, query, refIdOf, reject, panicAt] at *)

theorem copyLoc_ref {m : CompiledModule} {view : FunctionView} {st st' : RefState} {i id : Nat}
    (hloc : st.locals[i]? = some (.reference id))
    (h : (executeInner m view (.copyLoc i)).run st = .ok ((), st')) :
    ∃ b g1 g2, st.graph.isMutable id = .ok b ∧ st.graph.newRef st.nextId b = .ok g1 ∧
      g1.factor id [] st.nextId = .ok g2 ∧
      st' = { st with
        graph := g2, nextId := st.nextId + 1, stack := .reference st.nextId :: st.stack } := by
  simp only [executeInner, ReferenceSafety.copyLoc] at h
  unfold_ref
  simp only [hloc, StateT.pure, StateT.get, StateT.bind, StateT.lift, StateT.set, pure,
    Except.pure, bind, Except.bind] at h
  cases hm : st.graph.isMutable id with
  | error e => simp [hm] at h
  | ok b =>
    simp only [hm] at h
    cases hn : st.graph.newRef st.nextId b with
    | error e => simp [hn] at h
    | ok g1 =>
      simp only [hn] at h
      cases hf : g1.factor id [] st.nextId with
      | error e => simp [hf] at h
      | ok g2 =>
        simp only [hf, Except.ok.injEq, Prod.mk.injEq] at h
        exact ⟨b, g1, g2, rfl, hn, hf, h.2.symm⟩

/-! ## Primitives, by their effect -/

theorem push_run {v : AbsValue} {st st' : RefState} :
    (ReferenceSafety.push v).run st = .ok ((), st') ↔ st' = { st with stack := v :: st.stack } := by
  simp [ReferenceSafety.push, StateT.run, modify, modifyGet, MonadStateOf.modifyGet,
    StateT.modifyGet, pure, Except.pure, eq_comm]

theorem pop_run {v : AbsValue} {st st' : RefState} :
    ReferenceSafety.pop.run st = .ok (v, st') ↔
      ∃ rest, st.stack = v :: rest ∧ st' = { st with stack := rest } := by
  unfold ReferenceSafety.pop
  simp only [StateT.run, bind, StateT.bind, get, getThe, MonadStateOf.get, StateT.get, pure,
    Except.pure, Except.bind, set]
  cases hs : st.stack with
  | nil => simp [panicAt, liftM, monadLift, MonadLift.monadLift, StateT.lift, Functor.map,
      Except.map]
  | cons x rest =>
    simp only [StateT.bind, StateT.set, StateT.pure, pure, Except.pure, bind, Except.bind,
      Except.ok.injEq, Prod.mk.injEq, List.cons.injEq]
    constructor
    · intro h; exact ⟨rest, ⟨h.1, rfl⟩, h.2.symm⟩
    · rintro ⟨r, ⟨h1, h2⟩, h3⟩; subst h2; exact ⟨h1, h3.symm⟩

theorem popRef_run {id : Nat} {st st' : RefState} :
    ReferenceSafety.popRef.run st = .ok (id, st') ↔
      ∃ rest, st.stack = .reference id :: rest ∧ st' = { st with stack := rest } := by
  unfold ReferenceSafety.popRef
  rw [run_bind]
  constructor
  · rintro ⟨v, st₁, hp, h⟩
    obtain ⟨rest, hs, rfl⟩ := pop_run.1 hp
    cases v with
    | reference id' =>
      simp only [StateT.run, pure, StateT.pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      exact ⟨rest, hs, rfl⟩
    | nonReference => simp [panicAt, StateT.run, liftM, monadLift, MonadLift.monadLift,
        StateT.lift, bind, Except.bind] at h
  · rintro ⟨rest, hs, rfl⟩
    exact ⟨.reference id, _, pop_run.2 ⟨rest, hs, rfl⟩, by simp [StateT.run, pure, StateT.pure,
      Except.pure]⟩

theorem popValue_run {st st' : RefState} :
    ReferenceSafety.popValue.run st = .ok ((), st') ↔
      ∃ rest, st.stack = .nonReference :: rest ∧ st' = { st with stack := rest } := by
  unfold ReferenceSafety.popValue
  rw [run_bind]
  constructor
  · rintro ⟨v, st₁, hp, h⟩
    obtain ⟨rest, hs, rfl⟩ := pop_run.1 hp
    cases v with
    | nonReference =>
      simp only [StateT.run, pure, StateT.pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      exact ⟨rest, hs, h.2.symm⟩
    | reference _ => simp [panicAt, StateT.run, liftM, monadLift, MonadLift.monadLift,
        StateT.lift, bind, Except.bind] at h
  · rintro ⟨rest, hs, rfl⟩
    exact ⟨.nonReference, _, pop_run.2 ⟨rest, hs, rfl⟩, by simp [StateT.run, pure, StateT.pure,
      Except.pure]⟩

theorem newRef_run {m : Bool} {id : Nat} {st st' : RefState} :
    (ReferenceSafety.newRef m).run st = .ok (id, st') ↔
      ∃ g, st.graph.newRef st.nextId m = .ok g ∧ id = st.nextId ∧
        st' = { st with graph := g, nextId := st.nextId + 1 } := by
  unfold ReferenceSafety.newRef
  simp only [StateT.run, bind, StateT.bind, get, getThe, MonadStateOf.get, StateT.get, liftM,
    monadLift, MonadLift.monadLift, StateT.lift, Except.bind, pure, Except.pure, set,
    StateT.set, StateT.pure]
  cases hf : st.graph.newRef st.nextId m <;> simp [eq_comm]

theorem release_run {id : Nat} {st st' : RefState} :
    (ReferenceSafety.release id).run st = .ok ((), st') ↔
      ∃ g, st.graph.release id = .ok g ∧ st' = { st with graph := g } := graphOp_ok

theorem localAt_run {i : Nat} {v : AbsValue} {st st' : RefState} :
    (ReferenceSafety.localAt i).run st = .ok (v, st') ↔ st.locals[i]? = some v ∧ st' = st := by
  unfold ReferenceSafety.localAt
  simp only [StateT.run, bind, StateT.bind, get, getThe, MonadStateOf.get, StateT.get, pure,
    Except.pure, Except.bind]
  cases hs : st.locals[i]? with
  | none => simp [panicAt, liftM, monadLift, MonadLift.monadLift, StateT.lift, Functor.map,
      Except.map]
  | some x => simp [StateT.pure, pure, Except.pure, eq_comm]

theorem setLocal_run {i : Nat} {v : AbsValue} {st st' : RefState} :
    (ReferenceSafety.setLocal i v).run st = .ok ((), st') ↔
      st' = { st with locals := st.locals.set i v } := by
  simp [ReferenceSafety.setLocal, StateT.run, modify, modifyGet, MonadStateOf.modifyGet,
    StateT.modifyGet, pure, Except.pure, eq_comm]

theorem frameRoot_run {r : Nat} {st st' : RefState} :
    ReferenceSafety.frameRoot.run st = .ok (r, st') ↔ r = st.locals.length ∧ st' = st := by
  simp [ReferenceSafety.frameRoot, StateT.run, bind, StateT.bind, get, getThe, MonadStateOf.get,
    StateT.get, pure, Except.pure, Except.bind, StateT.pure, eq_comm]

theorem run_reject {α : Type} {c : String} {a : α} {st st' : RefState} :
    ¬ ((liftM (reject c : VResult α) : RefM α).run st = .ok (a, st')) := by
  simp [reject, StateT.run, liftM, monadLift, MonadLift.monadLift, StateT.lift, bind, Except.bind]

/-! ## Queries in the analysis -/

theorem isLocalMutablyBorrowed_run {i : Nat} {b : Bool} {st st' : RefState} :
    (isLocalMutablyBorrowed i).run st = .ok (b, st') ↔
      st.graph.hasConsistentMutableBorrows st.locals.length (some (.local_ i)) = .ok b ∧
        st' = st := by
  unfold isLocalMutablyBorrowed
  rw [run_bind]
  constructor
  · rintro ⟨r, st₁, hr, hq⟩
    obtain ⟨rfl, rfl⟩ := frameRoot_run.1 hr
    exact query_ok.1 hq
  · rintro ⟨hq, rfl⟩
    exact ⟨_, _, frameRoot_run.2 ⟨rfl, rfl⟩, query_ok.2 ⟨hq, rfl⟩⟩

theorem isLocalBorrowed_run {i : Nat} {b : Bool} {st st' : RefState} :
    (isLocalBorrowed i).run st = .ok (b, st') ↔
      st.graph.hasConsistentBorrows st.locals.length (some (.local_ i)) = .ok b ∧ st' = st := by
  unfold isLocalBorrowed
  rw [run_bind]
  constructor
  · rintro ⟨r, st₁, hr, hq⟩
    obtain ⟨rfl, rfl⟩ := frameRoot_run.1 hr
    exact query_ok.1 hq
  · rintro ⟨hq, rfl⟩
    exact ⟨_, _, frameRoot_run.2 ⟨rfl, rfl⟩, query_ok.2 ⟨hq, rfl⟩⟩

theorem lift_run_ok {α : Type} {x : VResult α} {a : α} {st st' : RefState} :
    (monadLift x : RefM α).run st = .ok (a, st') ↔ x = .ok a ∧ st' = st := by
  simp only [StateT.run, monadLift, MonadLift.monadLift, StateT.lift, bind, Except.bind]
  cases x <;> simp [pure, Except.pure, eq_comm]

/-- An `if` on a query result that rejects when it holds. -/
theorem ifReject_run {α : Type} {b : Bool} {c : String} {k : RefM α} {a : α}
    {st st' : RefState} :
    (if b = true then (monadLift (reject c : VResult α) : RefM α) else k).run st = .ok (a, st') ↔
      b = false ∧ k.run st = .ok (a, st') := by
  cases b <;> simp [reject, StateT.run, monadLift, MonadLift.monadLift, StateT.lift, bind,
    Except.bind]

/-! ## Moving values about -/

theorem exec_pop {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    (h : (executeInner m view .pop).run st = .ok ((), st')) :
    ∃ v rest, st.stack = v :: rest ∧
      ((v = .nonReference ∧ st' = { st with stack := rest }) ∨
        ∃ id g, v = .reference id ∧ st.graph.release id = .ok g ∧
          st' = { st with stack := rest, graph := g }) := by
  simp only [executeInner, run_bind] at h
  obtain ⟨v, st₁, hp, h⟩ := h
  obtain ⟨rest, hs, rfl⟩ := pop_run.1 hp
  refine ⟨v, rest, hs, ?_⟩
  cases v with
  | nonReference =>
    simp only [releaseValue, StateT.run, pure, StateT.pure, Except.pure, Except.ok.injEq,
      Prod.mk.injEq] at h
    exact Or.inl ⟨rfl, h.2.symm⟩
  | reference id =>
    simp only [releaseValue] at h
    obtain ⟨g, hg, rfl⟩ := release_run.1 h
    exact Or.inr ⟨id, g, rfl, hg, rfl⟩

theorem exec_copyLoc_val {m : CompiledModule} {view : FunctionView} {st st' : RefState} {i : Nat}
    (hloc : st.locals[i]? = some .nonReference)
    (h : (executeInner m view (.copyLoc i)).run st = .ok ((), st')) :
    st.graph.hasConsistentMutableBorrows st.locals.length (some (.local_ i)) = .ok false ∧
      st' = { st with stack := .nonReference :: st.stack } := by
  simp only [executeInner, run_bind] at h
  obtain ⟨v, st₁, hc, h⟩ := h
  simp only [ReferenceSafety.copyLoc, run_bind] at hc
  obtain ⟨old, st₂, hl, hc⟩ := hc
  obtain ⟨hold, rfl⟩ := localAt_run.1 hl
  rw [hloc, Option.some.injEq] at hold
  subst hold
  simp only [run_bind] at hc
  obtain ⟨b, st₃, hb, hc⟩ := hc
  obtain ⟨hq, rfl⟩ := isLocalMutablyBorrowed_run.1 hb
  rw [ifReject_run] at hc
  obtain ⟨rfl, hc⟩ := hc
  simp only [StateT.run, pure, StateT.pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at hc
  obtain ⟨rfl, rfl⟩ := hc
  exact ⟨hq, push_run.1 h⟩

theorem exec_moveLoc {m : CompiledModule} {view : FunctionView} {st st' : RefState} {i : Nat}
    (h : (executeInner m view (.moveLoc i)).run st = .ok ((), st')) :
    ∃ v, st.locals[i]? = some v ∧
      st' = { st with locals := st.locals.set i .nonReference, stack := v :: st.stack } ∧
      (v = .nonReference →
        st.graph.hasConsistentBorrows st.locals.length (some (.local_ i)) = .ok false) := by
  simp only [executeInner, run_bind] at h
  obtain ⟨v, st₁, hc, h⟩ := h
  simp only [ReferenceSafety.moveLoc, run_bind] at hc
  obtain ⟨old, st₂, hl, hc⟩ := hc
  obtain ⟨hold, rfl⟩ := localAt_run.1 hl
  obtain ⟨u, st₃, hs, hc⟩ := hc
  rw [setLocal_run] at hs
  subst hs
  refine ⟨old, hold, ?_⟩
  cases old with
  | reference id =>
    simp only [StateT.run, pure, StateT.pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at hc
    obtain ⟨rfl, rfl⟩ := hc
    exact ⟨push_run.1 h, fun h => by cases h⟩
  | nonReference =>
    simp only [run_bind] at hc
    obtain ⟨b, st₄, hb, hc⟩ := hc
    obtain ⟨hq, rfl⟩ := isLocalBorrowed_run.1 hb
    rw [ifReject_run] at hc
    obtain ⟨rfl, hc⟩ := hc
    simp only [StateT.run, pure, StateT.pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at hc
    obtain ⟨rfl, rfl⟩ := hc
    exact ⟨push_run.1 h, fun _ => by simpa using hq⟩

theorem exec_stLoc {m : CompiledModule} {view : FunctionView} {st st' : RefState} {i : Nat}
    (h : (executeInner m view (.stLoc i)).run st = .ok ((), st')) :
    ∃ v rest old, st.stack = v :: rest ∧ st.locals[i]? = some old ∧
      ((old = .nonReference ∧
          st.graph.hasConsistentBorrows st.locals.length (some (.local_ i)) = .ok false ∧
          st' = { st with locals := st.locals.set i v, stack := rest }) ∨
        ∃ id g, old = .reference id ∧ st.graph.release id = .ok g ∧
          st' = { st with locals := st.locals.set i v, stack := rest, graph := g }) := by
  simp only [executeInner, run_bind] at h
  obtain ⟨v, st₁, hp, h⟩ := h
  obtain ⟨rest, hs, rfl⟩ := pop_run.1 hp
  simp only [ReferenceSafety.stLoc, run_bind] at h
  obtain ⟨old, st₂, hl, h⟩ := h
  obtain ⟨hold, rfl⟩ := localAt_run.1 hl
  obtain ⟨u, st₃, hset, h⟩ := h
  rw [setLocal_run] at hset
  subst hset
  refine ⟨v, rest, old, hs, hold, ?_⟩
  cases old with
  | reference id =>
    obtain ⟨g, hg, rfl⟩ := release_run.1 h
    exact Or.inr ⟨id, g, rfl, hg, rfl⟩
  | nonReference =>
    simp only [run_bind] at h
    obtain ⟨b, st₄, hb, h⟩ := h
    obtain ⟨hq, rfl⟩ := isLocalBorrowed_run.1 hb
    rw [ifReject_run] at h
    obtain ⟨rfl, h⟩ := h
    simp only [StateT.run, pure, StateT.pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    exact Or.inl ⟨rfl, by simpa using hq, h.2.symm⟩

theorem isReadable_run {id : Nat} {f : Option Nat} {b : Bool} {st st' : RefState} :
    (ReferenceSafety.isReadable id f).run st = .ok (b, st') ↔
      st.graph.isReadable id (f.map .field) = .ok b ∧ st' = st := query_ok

theorem isWritable_run {id : Nat} {b : Bool} {st st' : RefState} :
    (ReferenceSafety.isWritable id).run st = .ok (b, st') ↔
      st.graph.isWritable id = .ok b ∧ st' = st := query_ok

theorem isFreezable_run {id : Nat} {b : Bool} {st st' : RefState} :
    (ReferenceSafety.isFreezable id).run st = .ok (b, st') ↔
      st.graph.isFreezable id none = .ok b ∧ st' = st := query_ok

theorem isMutable_run {id : Nat} {b : Bool} {st st' : RefState} :
    (ReferenceSafety.isMutable id).run st = .ok (b, st') ↔
      st.graph.isMutable id = .ok b ∧ st' = st := query_ok

/-- A `do`-block `if` that rejects when it holds. -/
theorem ifRejectUnit_run {c : Bool} {s : String} {u : PUnit} {st st' : RefState} :
    (if c = true then (monadLift (reject s : VResult PUnit) : RefM PUnit) else pure PUnit.unit).run st =
        .ok (u, st') ↔ c = false ∧ st' = st := by
  cases c <;> simp [reject, StateT.run, monadLift, MonadLift.monadLift, StateT.lift, bind,
    Except.bind, pure, StateT.pure, Except.pure, eq_comm]

theorem pure_run {α : Type} {a b : α} {st st' : RefState} :
    (pure a : RefM α).run st = .ok (b, st') ↔ a = b ∧ st' = st := by
  simp [StateT.run, pure, StateT.pure, Except.pure, eq_comm]

/-- A block that begins with a rejection does not succeed. -/
theorem reject_bind_run {α β : Type} {s : String} {k : α → RefM β} {b : β} {st st' : RefState} :
    ¬ ((liftM (reject s : VResult α) >>= k : RefM β).run st = .ok (b, st')) := by
  simp [reject, StateT.run, liftM, monadLift, MonadLift.monadLift, StateT.lift, bind, StateT.bind,
    Except.bind]

theorem exec_readRef {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    (h : (executeInner m view .readRef).run st = .ok ((), st')) :
    ∃ id rest g, st.stack = .reference id :: rest ∧ st.graph.isReadable id none = .ok true ∧
      st.graph.release id = .ok g ∧ st' = { st with stack := .nonReference :: rest, graph := g } := by
  simp only [executeInner, run_bind] at h
  obtain ⟨id, st₁, hp, h⟩ := h
  obtain ⟨rest, hs, rfl⟩ := popRef_run.1 hp
  obtain ⟨v, st₂, hr, h⟩ := h
  simp only [ReferenceSafety.readRef, run_bind] at hr
  obtain ⟨b, st₃, hb, hr⟩ := hr
  obtain ⟨hq, rfl⟩ := isReadable_run.1 hb
  split at hr
  · exact absurd hr reject_bind_run
  · next hbf =>
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hbf
    subst hbf
    rw [run_bind] at hr
    obtain ⟨u', st₅, hrel, hr⟩ := hr
    obtain ⟨g, hg, rfl⟩ := release_run.1 hrel
    obtain ⟨rfl, rfl⟩ := pure_run.1 hr
    exact ⟨id, rest, g, hs, hq, hg, push_run.1 h⟩

theorem lookup_run {α : Type} {t : Array α} {i : Nat} {w : String} {a : α} {st st' : RefState} :
    (liftM (lookup t i w) : RefM α).run st = .ok (a, st') ↔ t[i]? = some a ∧ st' = st := by
  simp only [StateT.run, liftM, monadLift, MonadLift.monadLift, StateT.lift, bind, Except.bind]
  unfold lookup
  cases t[i]? <;> simp [panicAt, pure, Except.pure, eq_comm]

theorem fieldOf_run {m : CompiledModule} {h f : Nat} {st st' : RefState} :
    (fieldOf m h).run st = .ok (f, st') ↔
      (∃ fh, m.fieldHandles[h]? = some fh ∧ f = fh.field) ∧ st' = st := by
  unfold fieldOf
  rw [run_bind]
  constructor
  · rintro ⟨fh, st₁, hl, hp⟩
    obtain ⟨hfh, rfl⟩ := lookup_run.1 hl
    obtain ⟨rfl, rfl⟩ := pure_run.1 hp
    exact ⟨⟨fh, hfh, rfl⟩, rfl⟩
  · rintro ⟨⟨fh, hfh, rfl⟩, rfl⟩
    exact ⟨fh, _, lookup_run.2 ⟨hfh, rfl⟩, pure_run.2 ⟨rfl, rfl⟩⟩

theorem variantFieldOf_run {m : CompiledModule} {h f : Nat} {st st' : RefState} :
    (variantFieldOf m h).run st = .ok (f, st') ↔
      (∃ fh, m.variantFieldHandles[h]? = some fh ∧ f = fh.field) ∧ st' = st := by
  unfold variantFieldOf
  rw [run_bind]
  constructor
  · rintro ⟨fh, st₁, hl, hp⟩
    obtain ⟨hfh, rfl⟩ := lookup_run.1 hl
    obtain ⟨rfl, rfl⟩ := pure_run.1 hp
    exact ⟨⟨fh, hfh, rfl⟩, rfl⟩
  · rintro ⟨⟨fh, hfh, rfl⟩, rfl⟩
    exact ⟨fh, _, lookup_run.2 ⟨hfh, rfl⟩, pure_run.2 ⟨rfl, rfl⟩⟩

theorem valueFor_value {ty : SignatureToken} (hk : refKind ty = none) {v : AbsValue}
    {st st' : RefState} : (valueFor ty).run st = .ok (v, st') ↔ v = .nonReference ∧ st' = st := by
  unfold valueFor
  rw [hk]
  exact pure_run.trans (by simp [eq_comm])

/-! ## Writing, freezing and borrowing -/

theorem exec_writeRef {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    (h : (executeInner m view .writeRef).run st = .ok ((), st')) :
    ∃ id rest g, st.stack = .reference id :: .nonReference :: rest ∧
      st.graph.isWritable id = .ok true ∧ st.graph.release id = .ok g ∧
      st' = { st with stack := rest, graph := g } := by
  simp only [executeInner, run_bind] at h
  obtain ⟨id, st₁, hp, h⟩ := h
  obtain ⟨rest₁, hs₁, rfl⟩ := popRef_run.1 hp
  obtain ⟨u, st₂, hv, h⟩ := h
  obtain ⟨rest, hs₂, rfl⟩ := popValue_run.1 hv
  simp only [ReferenceSafety.writeRef, run_bind] at h
  obtain ⟨b, st₃, hb, h⟩ := h
  obtain ⟨hq, rfl⟩ := isWritable_run.1 hb
  split at h
  · exact absurd h reject_bind_run
  · next hbf =>
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hbf
    subst hbf
    obtain ⟨g, hg, rfl⟩ := release_run.1 h
    simp only at hs₂ hq hg
    subst hs₂
    exact ⟨id, rest, g, hs₁, hq, hg, rfl⟩

theorem exec_freezeRef {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    (h : (executeInner m view .freezeRef).run st = .ok ((), st')) :
    ∃ id rest g1 g2 g3, st.stack = .reference id :: rest ∧
      st.graph.isFreezable id none = .ok true ∧ st.graph.newRef st.nextId false = .ok g1 ∧
      g1.factor id [] st.nextId = .ok g2 ∧ g2.release id = .ok g3 ∧
      st' = { st with
        stack := .reference st.nextId :: rest, graph := g3, nextId := st.nextId + 1 } := by
  simp only [executeInner, run_bind] at h
  obtain ⟨id, st₁, hp, h⟩ := h
  obtain ⟨rest, hs, rfl⟩ := popRef_run.1 hp
  obtain ⟨v, st₂, hf, h⟩ := h
  simp only [ReferenceSafety.freezeRef, run_bind] at hf
  obtain ⟨b, st₃, hb, hf⟩ := hf
  obtain ⟨hq, rfl⟩ := isFreezable_run.1 hb
  split at hf
  · exact absurd hf reject_bind_run
  · next hbf =>
    simp only [Bool.not_eq_true', Bool.not_eq_false] at hbf
    subst hbf
    rw [run_bind] at hf
    obtain ⟨fr, st₄, hn, hf⟩ := hf
    obtain ⟨g1, hg1, rfl, rfl⟩ := newRef_run.1 hn
    rw [run_bind] at hf
    obtain ⟨u, st₅, hc, hf⟩ := hf
    obtain ⟨g2, hg2, rfl⟩ := graphOp_ok.1 hc
    rw [run_bind] at hf
    obtain ⟨u', st₆, hrel, hf⟩ := hf
    obtain ⟨g3, hg3, rfl⟩ := release_run.1 hrel
    obtain ⟨rfl, rfl⟩ := pure_run.1 hf
    exact ⟨id, rest, g1, g2, g3, hs, hq, hg1, hg2, hg3, push_run.1 h⟩

theorem frameRoot_bind_run {α : Type} {k : Nat → RefM α} {a : α} {st st' : RefState} :
    (ReferenceSafety.frameRoot >>= k).run st = .ok (a, st') ↔ (k st.locals.length).run st = .ok (a, st') := by
  rw [run_bind]
  constructor
  · rintro ⟨r, st₁, hr, hk⟩
    obtain ⟨rfl, rfl⟩ := frameRoot_run.1 hr
    exact hk
  · intro hk
    exact ⟨_, _, frameRoot_run.2 ⟨rfl, rfl⟩, hk⟩

theorem exec_borrowLoc {m : CompiledModule} {view : FunctionView} {st st' : RefState} {i : Nat}
    {mut_ : Bool}
    (h : (executeInner m view (if mut_ then .mutBorrowLoc i else .immBorrowLoc i)).run st =
      .ok ((), st')) :
    ∃ g1 g2, (mut_ = false →
        st.graph.hasConsistentMutableBorrows st.locals.length (some (.local_ i)) = .ok false) ∧
      (mut_ = true → st.graph.hasFullBorrows st.locals.length = .ok false) ∧
      st.graph.newRef st.nextId mut_ = .ok g1 ∧
      g1.factor st.locals.length [.local_ i] st.nextId = .ok g2 ∧
      st' = { st with
        stack := .reference st.nextId :: st.stack, graph := g2, nextId := st.nextId + 1 } := by
  have h' : (do ReferenceSafety.push (← borrowLoc mut_ i) : RefM Unit).run st = .ok ((), st') := by
    cases mut_ <;> simpa [executeInner] using h
  simp only [run_bind] at h'
  obtain ⟨v, st₁, hb, h'⟩ := h'
  simp only [borrowLoc, run_bind] at hb
  obtain ⟨a, st₂, hl, hb⟩ := hb
  obtain ⟨hq1, rfl⟩ := isLocalMutablyBorrowed_run.1 hl
  split at hb
  · exact absurd hb reject_bind_run
  · next ha =>
    rw [frameRoot_bind_run, run_bind] at hb
    obtain ⟨b, st₃, hq, hb⟩ := hb
    obtain ⟨hq2, rfl⟩ := query_ok.1 hq
    split at hb
    · exact absurd hb reject_bind_run
    · next hb2 =>
      rw [run_bind] at hb
      obtain ⟨newId, st₄, hn, hb⟩ := hb
      obtain ⟨g1, hg1, rfl, rfl⟩ := newRef_run.1 hn
      rw [run_bind] at hb
      obtain ⟨u, st₅, hadd, hb⟩ := hb
      simp only [addLocalBorrow, frameRoot_bind_run] at hadd
      obtain ⟨g2, hg2, rfl⟩ := graphOp_ok.1 hadd
      obtain ⟨rfl, rfl⟩ := pure_run.1 hb
      refine ⟨g1, g2, fun hm => ?_, fun hm => ?_, hg1, hg2, push_run.1 h'⟩
      · subst hm
        simp only [Bool.not_false, Bool.true_and, Bool.not_eq_true] at ha
        rw [ha] at hq1; exact hq1
      · subst hm
        simp only [Bool.true_and, Bool.not_eq_true] at hb2
        rw [hb2] at hq2; exact hq2

theorem borrowField_tail {mut_ : Bool} {id f : Nat} {v : AbsValue} {st st' : RefState}
    (h : (do
      let newId ← ReferenceSafety.newRef mut_
      addFieldBorrow id f newId
      ReferenceSafety.release id
      pure (AbsValue.reference newId) : RefM AbsValue).run st = .ok (v, st')) :
    ∃ g1 g2 g3, st.graph.newRef st.nextId mut_ = .ok g1 ∧ g1.factor id [.field f] st.nextId = .ok g2 ∧
      g2.release id = .ok g3 ∧ v = .reference st.nextId ∧
      st' = { st with graph := g3, nextId := st.nextId + 1 } := by
  rw [run_bind] at h
  obtain ⟨newId, st₂, hn, h⟩ := h
  obtain ⟨g1, hg1, rfl, rfl⟩ := newRef_run.1 hn
  rw [run_bind] at h
  obtain ⟨u, st₃, hadd, h⟩ := h
  obtain ⟨g2, hg2, rfl⟩ := graphOp_ok.1 hadd
  rw [run_bind] at h
  obtain ⟨u', st₄, hrel, h⟩ := h
  obtain ⟨g3, hg3, rfl⟩ := release_run.1 hrel
  obtain ⟨rfl, rfl⟩ := pure_run.1 h
  exact ⟨g1, g2, g3, hg1, hg2, hg3, rfl, rfl⟩

theorem borrowField_run {mut_ : Bool} {id f : Nat} {v : AbsValue} {st st' : RefState}
    (h : (ReferenceSafety.borrowField mut_ id f).run st = .ok (v, st')) :
    ∃ g1 g2 g3, (mut_ = true → st.graph.hasFullBorrows id = .ok false) ∧
      (mut_ = false → st.graph.isReadable id (some (.field f)) = .ok true) ∧
      st.graph.newRef st.nextId mut_ = .ok g1 ∧ g1.factor id [.field f] st.nextId = .ok g2 ∧
      g2.release id = .ok g3 ∧ v = .reference st.nextId ∧
      st' = { st with graph := g3, nextId := st.nextId + 1 } := by
  simp only [ReferenceSafety.borrowField] at h
  cases mut_
  · simp only [Bool.false_eq_true, ↓reduceIte, run_bind] at h
    obtain ⟨r, st₁, hr, h⟩ := h
    obtain ⟨hq, rfl⟩ := isReadable_run.1 hr
    obtain ⟨bad, st₂, hp, h⟩ := h
    obtain ⟨rfl, rfl⟩ := pure_run.1 hp
    split at h
    · exact absurd h reject_bind_run
    · next hb =>
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hb
      subst hb
      obtain ⟨g1, g2, g3, h1, h2, h3, h4, h5⟩ := borrowField_tail h
      exact ⟨g1, g2, g3, fun h => absurd h (by decide), fun _ => by simpa using hq, h1, h2, h3, h4,
        h5⟩
  · simp only [↓reduceIte, run_bind] at h
    obtain ⟨bad, st₁, hq', h⟩ := h
    obtain ⟨hq, rfl⟩ := query_ok.1 hq'
    split at h
    · exact absurd h reject_bind_run
    · next hb =>
      simp only [Bool.not_eq_true] at hb
      subst hb
      obtain ⟨g1, g2, g3, h1, h2, h3, h4, h5⟩ := borrowField_tail h
      exact ⟨g1, g2, g3, fun _ => hq, fun h => absurd h (by decide), h1, h2, h3, h4, h5⟩

/-- The field borrows: through field `f` of the handle, a reference on top of
the stack becomes a new reference to the field. -/
theorem exec_fieldBorrow {st st' : RefState} {mut_ : Bool} {f : Nat}
    (h : (do let id ← popRef; ReferenceSafety.push (← ReferenceSafety.borrowField mut_ id f) :
      RefM Unit).run st = .ok ((), st')) :
    ∃ id rest g1 g2 g3, st.stack = .reference id :: rest ∧
      (mut_ = true → st.graph.hasFullBorrows id = .ok false) ∧
      (mut_ = false → st.graph.isReadable id (some (.field f)) = .ok true) ∧
      st.graph.newRef st.nextId mut_ = .ok g1 ∧ g1.factor id [.field f] st.nextId = .ok g2 ∧
      g2.release id = .ok g3 ∧
      st' = { st with
        stack := .reference st.nextId :: rest, graph := g3, nextId := st.nextId + 1 } := by
  rw [run_bind] at h
  obtain ⟨id, st₁, hp, h⟩ := h
  obtain ⟨rest, hs, rfl⟩ := popRef_run.1 hp
  rw [run_bind] at h
  obtain ⟨v, st₂, hb, h⟩ := h
  obtain ⟨g1, g2, g3, h1, h2, h3, h4, h5, rfl, rfl⟩ := borrowField_run hb
  exact ⟨id, rest, g1, g2, g3, hs, h1, h2, h3, h4, h5, push_run.1 h⟩

theorem refIdOf_run {v : AbsValue} {id : Nat} {st st' : RefState} :
    (refIdOf v).run st = .ok (id, st') ↔ v = .reference id ∧ st' = st := by
  cases v with
  | reference id' => simp [refIdOf, StateT.run, pure, StateT.pure, Except.pure, eq_comm]
  | nonReference => simp [refIdOf, panicAt, StateT.run, liftM, monadLift, MonadLift.monadLift,
      StateT.lift, bind, Except.bind]

theorem vectorOp_run {v : AbsValue} {mut_ : Bool} {st st' : RefState}
    (h : (vectorOp v mut_).run st = .ok ((), st')) :
    ∃ id g, v = .reference id ∧ (mut_ = true → st.graph.isWritable id = .ok true) ∧
      st.graph.release id = .ok g ∧ st' = { st with graph := g } := by
  simp only [vectorOp, run_bind] at h
  obtain ⟨id, st₁, hr, h⟩ := h
  obtain ⟨hv, hst⟩ := refIdOf_run.1 hr
  rw [hst] at h
  subst hv
  cases mut_
  · simp only [Bool.false_eq_true, ↓reduceIte] at h
    obtain ⟨g, hg, rfl⟩ := release_run.1 h
    exact ⟨id, g, rfl, fun h => absurd h (by decide), hg, rfl⟩
  · simp only [↓reduceIte, run_bind] at h
    obtain ⟨b, st₂, hb, h⟩ := h
    obtain ⟨hq, hst₂⟩ := isWritable_run.1 hb
    rw [hst₂] at h
    split at h
    · exact absurd h reject_bind_run
    · next hbf =>
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hbf
      subst hbf
      obtain ⟨g, hg, rfl⟩ := release_run.1 h
      exact ⟨id, g, rfl, fun _ => hq, hg, rfl⟩

theorem elementBorrow_tail {mut_ : Bool} {id : Nat} {r : AbsValue} {st st' : RefState}
    (h : (do
        let elemId ← ReferenceSafety.newRef mut_
        addBorrow id elemId
        ReferenceSafety.release id
        pure (AbsValue.reference elemId) : RefM AbsValue).run st = .ok (r, st')) :
    ∃ g1 g2 g3, st.graph.newRef st.nextId mut_ = .ok g1 ∧
      g1.addEdge id ⟨false, []⟩ st.nextId = .ok g2 ∧ g2.release id = .ok g3 ∧
      r = .reference st.nextId ∧ st' = { st with graph := g3, nextId := st.nextId + 1 } := by
  rw [run_bind] at h
  obtain ⟨newId, st₄, hn, h⟩ := h
  obtain ⟨g1, hg1, rfl, rfl⟩ := newRef_run.1 hn
  rw [run_bind] at h
  obtain ⟨u, st₅, hadd, h⟩ := h
  obtain ⟨g2, hg2, rfl⟩ := graphOp_ok.1 hadd
  rw [run_bind] at h
  obtain ⟨u', st₆, hrel, h⟩ := h
  obtain ⟨g3, hg3, rfl⟩ := release_run.1 hrel
  obtain ⟨rfl, rfl⟩ := pure_run.1 h
  exact ⟨g1, g2, g3, hg1, hg2, hg3, rfl, rfl⟩

theorem elementBorrow_run {v r : AbsValue} {mut_ : Bool} {st st' : RefState}
    (h : (vectorElementBorrow v mut_).run st = .ok (r, st')) :
    ∃ id g1 g2 g3, v = .reference id ∧
      (mut_ = false → st.graph.isReadable id none = .ok true) ∧
      (mut_ = true → st.graph.isWritable id = .ok true) ∧
      st.graph.newRef st.nextId mut_ = .ok g1 ∧ g1.addEdge id ⟨false, []⟩ st.nextId = .ok g2 ∧
      g2.release id = .ok g3 ∧ r = .reference st.nextId ∧
      st' = { st with graph := g3, nextId := st.nextId + 1 } := by
  simp only [vectorElementBorrow, run_bind] at h
  obtain ⟨id, st₁, hr, h⟩ := h
  obtain ⟨hv, hst⟩ := refIdOf_run.1 hr
  rw [hst] at h
  subst hv
  obtain ⟨rd, st₂, hrd, h⟩ := h
  obtain ⟨hq1, hst₂⟩ := isReadable_run.1 hrd
  rw [hst₂] at h
  cases mut_
  · simp only [Bool.not_false, Bool.true_and, Bool.false_eq_true, ↓reduceIte] at h
    split at h
    · exact absurd h reject_bind_run
    · next hbf =>
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hbf
      subst hbf
      obtain ⟨g1, g2, g3, h1, h2, h3, h4, h5⟩ := elementBorrow_tail h
      exact ⟨id, g1, g2, g3, rfl, fun _ => hq1, fun h => absurd h (by decide), h1, h2, h3, h4, h5⟩
  · simp only [Bool.not_true, Bool.false_and, Bool.false_eq_true, ↓reduceIte, run_bind] at h
    obtain ⟨b, st₄, hb, h⟩ := h
    obtain ⟨hq2, hst₄⟩ := isWritable_run.1 hb
    rw [hst₄] at h
    split at h
    · exact absurd h reject_bind_run
    · next hbf =>
      simp only [Bool.not_eq_true', Bool.not_eq_false] at hbf
      subst hbf
      obtain ⟨g1, g2, g3, h1, h2, h3, h4, h5⟩ := elementBorrow_tail h
      exact ⟨id, g1, g2, g3, rfl, fun h => absurd h (by decide), fun _ => hq2, h1, h2, h3, h4, h5⟩

theorem exec_mutBorrowField {m : CompiledModule} {view : FunctionView} {st st' : RefState} {h : Nat}
    (hx : (executeInner m view (.mutBorrowField h)).run st = .ok ((), st')) :
    ∃ fh, m.fieldHandles[h]? = some fh ∧ ∃ id rest g1 g2 g3, st.stack = .reference id :: rest ∧
      st.graph.hasFullBorrows id = .ok false ∧
      st.graph.newRef st.nextId true = .ok g1 ∧ g1.factor id [.field fh.field] st.nextId = .ok g2 ∧
      g2.release id = .ok g3 ∧
      st' = { st with
        stack := .reference st.nextId :: rest, graph := g3, nextId := st.nextId + 1 } := by
  simp only [executeInner, run_bind] at hx
  obtain ⟨id, st₁, hp, hx⟩ := hx
  obtain ⟨rest, hs, rfl⟩ := popRef_run.1 hp
  obtain ⟨f, st₂, hf, hx⟩ := hx
  obtain ⟨⟨fh, hfh, rfl⟩, rfl⟩ := fieldOf_run.1 hf
  obtain ⟨v, st₃, hb, hx⟩ := hx
  obtain ⟨g1, g2, g3, h1, -, h3, h4, h5, rfl, rfl⟩ := borrowField_run hb
  exact ⟨fh, hfh, id, rest, g1, g2, g3, hs, h1 rfl, h3, h4, h5, push_run.1 hx⟩

theorem exec_immBorrowField {m : CompiledModule} {view : FunctionView} {st st' : RefState} {h : Nat}
    (hx : (executeInner m view (.immBorrowField h)).run st = .ok ((), st')) :
    ∃ fh, m.fieldHandles[h]? = some fh ∧ ∃ id rest g1 g2 g3, st.stack = .reference id :: rest ∧
      st.graph.isReadable id (some (.field fh.field)) = .ok true ∧
      st.graph.newRef st.nextId false = .ok g1 ∧ g1.factor id [.field fh.field] st.nextId = .ok g2 ∧
      g2.release id = .ok g3 ∧
      st' = { st with
        stack := .reference st.nextId :: rest, graph := g3, nextId := st.nextId + 1 } := by
  simp only [executeInner, run_bind] at hx
  obtain ⟨id, st₁, hp, hx⟩ := hx
  obtain ⟨rest, hs, rfl⟩ := popRef_run.1 hp
  obtain ⟨f, st₂, hf, hx⟩ := hx
  obtain ⟨⟨fh, hfh, rfl⟩, rfl⟩ := fieldOf_run.1 hf
  obtain ⟨v, st₃, hb, hx⟩ := hx
  obtain ⟨g1, g2, g3, -, h2, h3, h4, h5, rfl, rfl⟩ := borrowField_run hb
  exact ⟨fh, hfh, id, rest, g1, g2, g3, hs, h2 rfl, h3, h4, h5, push_run.1 hx⟩

theorem exec_mutBorrowVariantField {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {h : Nat} (hx : (executeInner m view (.mutBorrowVariantField h)).run st = .ok ((), st')) :
    ∃ fh, m.variantFieldHandles[h]? = some fh ∧ ∃ id rest g1 g2 g3,
      st.stack = .reference id :: rest ∧ st.graph.hasFullBorrows id = .ok false ∧
      st.graph.newRef st.nextId true = .ok g1 ∧ g1.factor id [.field fh.field] st.nextId = .ok g2 ∧
      g2.release id = .ok g3 ∧
      st' = { st with
        stack := .reference st.nextId :: rest, graph := g3, nextId := st.nextId + 1 } := by
  simp only [executeInner, run_bind] at hx
  obtain ⟨id, st₁, hp, hx⟩ := hx
  obtain ⟨rest, hs, rfl⟩ := popRef_run.1 hp
  obtain ⟨f, st₂, hf, hx⟩ := hx
  obtain ⟨⟨fh, hfh, rfl⟩, rfl⟩ := variantFieldOf_run.1 hf
  obtain ⟨v, st₃, hb, hx⟩ := hx
  obtain ⟨g1, g2, g3, h1, -, h3, h4, h5, rfl, rfl⟩ := borrowField_run hb
  exact ⟨fh, hfh, id, rest, g1, g2, g3, hs, h1 rfl, h3, h4, h5, push_run.1 hx⟩

theorem exec_immBorrowVariantField {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {h : Nat} (hx : (executeInner m view (.immBorrowVariantField h)).run st = .ok ((), st')) :
    ∃ fh, m.variantFieldHandles[h]? = some fh ∧ ∃ id rest g1 g2 g3,
      st.stack = .reference id :: rest ∧
      st.graph.isReadable id (some (.field fh.field)) = .ok true ∧
      st.graph.newRef st.nextId false = .ok g1 ∧ g1.factor id [.field fh.field] st.nextId = .ok g2 ∧
      g2.release id = .ok g3 ∧
      st' = { st with
        stack := .reference st.nextId :: rest, graph := g3, nextId := st.nextId + 1 } := by
  simp only [executeInner, run_bind] at hx
  obtain ⟨id, st₁, hp, hx⟩ := hx
  obtain ⟨rest, hs, rfl⟩ := popRef_run.1 hp
  obtain ⟨f, st₂, hf, hx⟩ := hx
  obtain ⟨⟨fh, hfh, rfl⟩, rfl⟩ := variantFieldOf_run.1 hf
  obtain ⟨v, st₃, hb, hx⟩ := hx
  obtain ⟨g1, g2, g3, -, h2, h3, h4, h5, rfl, rfl⟩ := borrowField_run hb
  exact ⟨fh, hfh, id, rest, g1, g2, g3, hs, h2 rfl, h3, h4, h5, push_run.1 hx⟩

theorem exec_vecBorrow {m : CompiledModule} {view : FunctionView} {st st' : RefState} {sig : Nat}
    {mut_ : Bool}
    (hx : (executeInner m view (if mut_ then .vecMutBorrow sig else .vecImmBorrow sig)).run st =
      .ok ((), st')) :
    ∃ id rest g1 g2 g3, st.stack = .nonReference :: .reference id :: rest ∧
      (mut_ = false → st.graph.isReadable id none = .ok true) ∧
      (mut_ = true → st.graph.isWritable id = .ok true) ∧
      st.graph.newRef st.nextId mut_ = .ok g1 ∧ g1.addEdge id ⟨false, []⟩ st.nextId = .ok g2 ∧
      g2.release id = .ok g3 ∧
      st' = { st with
        stack := .reference st.nextId :: rest, graph := g3, nextId := st.nextId + 1 } := by
  have hx' : (do popValue; ReferenceSafety.push (← vectorElementBorrow (← ReferenceSafety.pop) mut_) :
      RefM Unit).run st = .ok ((), st') := by
    cases mut_ <;> simpa [executeInner] using hx
  rw [run_bind] at hx'
  obtain ⟨u, st₁, hv, hx'⟩ := hx'
  obtain ⟨rest₁, hs₁, rfl⟩ := popValue_run.1 hv
  rw [run_bind] at hx'
  obtain ⟨v, st₂, hp, hx'⟩ := hx'
  obtain ⟨rest, hs₂, rfl⟩ := pop_run.1 hp
  rw [run_bind] at hx'
  obtain ⟨r, st₃, hb, hx'⟩ := hx'
  obtain ⟨id, g1, g2, g3, rfl, h1, h2, h3, h4, h5, rfl, rfl⟩ := elementBorrow_run hb
  simp only at hs₂ h1 h2 h3
  subst hs₂
  exact ⟨id, rest, g1, g2, g3, hs₁, h1, h2, h3, h4, h5, push_run.1 hx'⟩

theorem exec_vecLen {m : CompiledModule} {view : FunctionView} {st st' : RefState} {sig : Nat}
    (hx : (executeInner m view (.vecLen sig)).run st = .ok ((), st')) :
    ∃ id rest g, st.stack = .reference id :: rest ∧ st.graph.release id = .ok g ∧
      st' = { st with stack := .nonReference :: rest, graph := g } := by
  simp only [executeInner, run_bind] at hx
  obtain ⟨v, st₁, hp, hx⟩ := hx
  obtain ⟨rest, hs, rfl⟩ := pop_run.1 hp
  obtain ⟨u, st₂, hop, hx⟩ := hx
  obtain ⟨id, g, rfl, -, hg, rfl⟩ := vectorOp_run hop
  obtain ⟨r, st₃, hvf, hx⟩ := hx
  obtain ⟨rfl, rfl⟩ := (valueFor_value (ty := .u64) rfl).1 hvf
  exact ⟨id, rest, g, hs, hg, push_run.1 hx⟩

theorem exec_vecPushBack {m : CompiledModule} {view : FunctionView} {st st' : RefState} {sig : Nat}
    (hx : (executeInner m view (.vecPushBack sig)).run st = .ok ((), st')) :
    ∃ id rest g, st.stack = .nonReference :: .reference id :: rest ∧
      st.graph.isWritable id = .ok true ∧ st.graph.release id = .ok g ∧
      st' = { st with stack := rest, graph := g } := by
  simp only [executeInner, run_bind] at hx
  obtain ⟨u, st₁, hv, hx⟩ := hx
  obtain ⟨rest₁, hs₁, rfl⟩ := popValue_run.1 hv
  obtain ⟨v, st₂, hp, hx⟩ := hx
  obtain ⟨rest, hs₂, rfl⟩ := pop_run.1 hp
  obtain ⟨id, g, rfl, hw, hg, rfl⟩ := vectorOp_run hx
  simp only at hs₂ hg
  subst hs₂
  exact ⟨id, rest, g, hs₁, hw rfl, hg, rfl⟩

theorem exec_vecPopBack {m : CompiledModule} {view : FunctionView} {st st' : RefState} {sig : Nat}
    (hval : ∀ ty, (m.signatures[sig]?.bind (·.head?)) = some ty → refKind ty = none)
    (hx : (executeInner m view (.vecPopBack sig)).run st = .ok ((), st')) :
    ∃ id rest g, st.stack = .reference id :: rest ∧
      st.graph.isWritable id = .ok true ∧ st.graph.release id = .ok g ∧
      st' = { st with stack := .nonReference :: rest, graph := g } := by
  simp only [executeInner, run_bind] at hx
  obtain ⟨v, st₁, hp, hx⟩ := hx
  obtain ⟨rest, hs, rfl⟩ := pop_run.1 hp
  obtain ⟨u, st₂, hop, hx⟩ := hx
  obtain ⟨id, g, rfl, hw, hg, rfl⟩ := vectorOp_run hop
  obtain ⟨ty, st₃, hty, hx⟩ := hx
  -- The element type.
  have hk : refKind ty = none ∧ st₃ = { st with stack := rest, graph := g } := by
    simp only [vecElementType, run_bind] at hty
    obtain ⟨sg, st₄, hl, hty⟩ := hty
    obtain ⟨hsg, rfl⟩ := lookup_run.1 hl
    split at hty
    · next t ht =>
      obtain ⟨rfl, rfl⟩ := pure_run.1 hty
      exact ⟨hval _ (by simp [hsg, ht]), rfl⟩
    · simp [reject, StateT.run, liftM, monadLift, MonadLift.monadLift, StateT.lift, bind,
        Except.bind] at hty
  obtain ⟨hk, rfl⟩ := hk
  obtain ⟨r, st₄, hvf, hx⟩ := hx
  obtain ⟨rfl, rfl⟩ := (valueFor_value hk).1 hvf
  exact ⟨id, rest, g, hs, hw rfl, hg, push_run.1 hx⟩

theorem exec_vecSwap {m : CompiledModule} {view : FunctionView} {st st' : RefState} {sig : Nat}
    (hx : (executeInner m view (.vecSwap sig)).run st = .ok ((), st')) :
    ∃ id rest g, st.stack = .nonReference :: .nonReference :: .reference id :: rest ∧
      st.graph.isWritable id = .ok true ∧ st.graph.release id = .ok g ∧
      st' = { st with stack := rest, graph := g } := by
  simp only [executeInner, run_bind] at hx
  obtain ⟨u, st₁, hv, hx⟩ := hx
  obtain ⟨rest₁, hs₁, rfl⟩ := popValue_run.1 hv
  obtain ⟨u', st₂, hv', hx⟩ := hx
  obtain ⟨rest₂, hs₂, rfl⟩ := popValue_run.1 hv'
  obtain ⟨v, st₃, hp, hx⟩ := hx
  obtain ⟨rest, hs₃, rfl⟩ := pop_run.1 hp
  obtain ⟨id, g, rfl, hw, hg, rfl⟩ := vectorOp_run hx
  simp only at hs₂ hs₃ hg
  subst hs₂ hs₃
  exact ⟨id, rest, g, hs₁, hw rfl, hg, rfl⟩

theorem comparison_run {v1 v2 r : AbsValue} {st st' : RefState}
    (h : (comparison v1 v2).run st = .ok (r, st')) :
    r = .nonReference ∧
      ((v1 = .nonReference ∧ v2 = .nonReference ∧ st' = st) ∨
        ∃ id1 id2 g1 g2, v1 = .reference id1 ∧ v2 = .reference id2 ∧
          st.graph.release id1 = .ok g1 ∧ g1.release id2 = .ok g2 ∧
          st' = { st with graph := g2 }) := by
  unfold comparison at h
  split at h
  · next id1 id2 =>
    simp only [run_bind] at h
    obtain ⟨b1, st₁, hb1, h⟩ := h
    obtain ⟨-, hst₁⟩ := isReadable_run.1 hb1
    rw [hst₁] at h
    obtain ⟨b2, st₂, hb2, h⟩ := h
    obtain ⟨-, hst₂⟩ := isReadable_run.1 hb2
    rw [hst₂] at h
    split at h
    · exact absurd h reject_bind_run
    · rw [run_bind] at h
      obtain ⟨u, st₃, hr1, h⟩ := h
      obtain ⟨g1, hg1, rfl⟩ := release_run.1 hr1
      rw [run_bind] at h
      obtain ⟨u', st₄, hr2, h⟩ := h
      obtain ⟨g2, hg2, rfl⟩ := release_run.1 hr2
      obtain ⟨rfl, rfl⟩ := pure_run.1 h
      exact ⟨rfl, Or.inr ⟨id1, id2, g1, g2, rfl, rfl, hg1, hg2, rfl⟩⟩
  · dsimp only at h
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    exact ⟨rfl, Or.inl ⟨rfl, rfl, rfl⟩⟩
  · simp [panicAt, StateT.run, liftM, monadLift, MonadLift.monadLift, StateT.lift, bind,
      StateT.bind, Except.bind] at h
  · simp [panicAt, StateT.run, liftM, monadLift, MonadLift.monadLift, StateT.lift, bind,
      StateT.bind, Except.bind] at h

theorem exec_eq {m : CompiledModule} {view : FunctionView} {st st' : RefState} {instr : Bytecode}
    (hi : instr = .eq ∨ instr = .neq)
    (hx : (executeInner m view instr).run st = .ok ((), st')) :
    ∃ v1 v2 rest, st.stack = v1 :: v2 :: rest ∧
      ((v1 = .nonReference ∧ v2 = .nonReference ∧
          st' = { st with stack := .nonReference :: rest }) ∨
        ∃ id1 id2 g1 g2, v1 = .reference id1 ∧ v2 = .reference id2 ∧
          st.graph.release id1 = .ok g1 ∧ g1.release id2 = .ok g2 ∧
          st' = { st with stack := .nonReference :: rest, graph := g2 }) := by
  have hx' : (do
      let v1 ← ReferenceSafety.pop
      let v2 ← ReferenceSafety.pop
      ReferenceSafety.push (← comparison v1 v2) : RefM Unit).run st = .ok ((), st') := by
    rcases hi with rfl | rfl <;> simpa [executeInner] using hx
  rw [run_bind] at hx'
  obtain ⟨v1, st₁, hp1, hx'⟩ := hx'
  obtain ⟨rest₁, hs₁, rfl⟩ := pop_run.1 hp1
  rw [run_bind] at hx'
  obtain ⟨v2, st₂, hp2, hx'⟩ := hx'
  obtain ⟨rest, hs₂, rfl⟩ := pop_run.1 hp2
  rw [run_bind] at hx'
  obtain ⟨r, st₃, hc, hx'⟩ := hx'
  obtain ⟨rfl, hcase⟩ := comparison_run hc
  simp only at hs₂
  subst hs₂
  refine ⟨v1, v2, rest, hs₁, ?_⟩
  rcases hcase with ⟨rfl, rfl, rfl⟩ | ⟨id1, id2, g1, g2, rfl, rfl, hg1, hg2, rfl⟩
  · exact Or.inl ⟨rfl, rfl, push_run.1 hx'⟩
  · exact Or.inr ⟨id1, id2, g1, g2, rfl, rfl, hg1, hg2, push_run.1 hx'⟩

theorem exec_testVariant {m : CompiledModule} {view : FunctionView} {st st' : RefState} {h : Nat}
    (hx : (executeInner m view (.testVariant h)).run st = .ok ((), st')) :
    ∃ id rest g, st.stack = .reference id :: rest ∧ st.graph.release id = .ok g ∧
      st' = { st with stack := .nonReference :: rest, graph := g } := by
  simp only [executeInner, run_bind] at hx
  obtain ⟨vh, st₁, hl, hx⟩ := hx
  obtain ⟨-, rfl⟩ := lookup_run.1 hl
  obtain ⟨id, st₂, hp, hx⟩ := hx
  obtain ⟨rest, hs, rfl⟩ := popRef_run.1 hp
  obtain ⟨v, st₃, hr, hx⟩ := hx
  simp only [ReferenceSafety.readRef, run_bind] at hr
  obtain ⟨b, st₄, hb, hr⟩ := hr
  obtain ⟨-, hst⟩ := isReadable_run.1 hb
  rw [hst] at hr
  split at hr
  · exact absurd hr reject_bind_run
  · rw [run_bind] at hr
    obtain ⟨u, st₅, hrel, hr⟩ := hr
    obtain ⟨g, hg, rfl⟩ := release_run.1 hrel
    obtain ⟨rfl, rfl⟩ := pure_run.1 hr
    exact ⟨id, rest, g, hs, hg, push_run.1 hx⟩

/-! ## Instructions on values -/

/-- Popping a value for each element of `l`. -/
theorem popValues_run {α : Type} : ∀ (l : List α) (st st' : RefState),
    (l.forM fun _ => popValue).run st = .ok ((), st') →
      l.length ≤ st.stack.length ∧ (∀ v ∈ st.stack.take l.length, v = .nonReference) ∧
        st' = { st with stack := st.stack.drop l.length }
  | [], st, st', h => by
    simp only [List.forM] at h
    obtain ⟨-, rfl⟩ := pure_run.1 h
    simp
  | a :: l, st, st', h => by
    simp only [List.forM] at h
    rw [run_bind] at h
    obtain ⟨u, st₁, hv, h⟩ := h
    obtain ⟨rest, hs, rfl⟩ := popValue_run.1 hv
    obtain ⟨h1, h2, rfl⟩ := popValues_run l _ st' h
    rw [hs]
    simp only [List.length_cons, List.take_succ_cons, List.mem_cons, List.drop_succ_cons]
    refine ⟨by simp at h1; omega, fun v hv => ?_, by first | rfl | trivial⟩
    rcases hv with rfl | hv
    · rfl
    · exact h2 v hv

/-- Pushing a value for each element of `l`. -/
theorem pushValues_run {α : Type} : ∀ (l : List α) (st st' : RefState),
    (l.forM fun _ => ReferenceSafety.push .nonReference).run st = .ok ((), st') →
      st' = { st with stack := List.replicate l.length .nonReference ++ st.stack }
  | [], st, st', h => by
    simp only [List.forM] at h
    obtain ⟨-, rfl⟩ := pure_run.1 h
    simp
  | a :: l, st, st', h => by
    simp only [List.forM] at h
    rw [run_bind] at h
    obtain ⟨u, st₁, hp, h⟩ := h
    rw [push_run] at hp
    subst hp
    rw [pushValues_run l _ st' h]
    simp [List.replicate_succ']

/-- The instructions the analysis does nothing for. -/
theorem exec_nothing {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {instr : Bytecode}
    (hi : instr ∈ [.branch 0, .nop] ∨ (∃ o, instr = .branch o) ∨ instr = .castU8 ∨
      instr = .castU16 ∨ instr = .castU32 ∨ instr = .castU64 ∨ instr = .castU128 ∨
      instr = .castU256 ∨ instr = .castI8 ∨ instr = .castI16 ∨ instr = .castI32 ∨
      instr = .castI64 ∨ instr = .castI128 ∨ instr = .castI256 ∨ instr = .negate ∨
      instr = .not ∨ (∃ d, instr = .exists_ d))
    (h : (executeInner m view instr).run st = .ok ((), st')) : st' = st := by
  rcases hi with hi | ⟨o, rfl⟩ | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl |
    rfl | rfl | rfl | ⟨d, rfl⟩
  · simp only [List.mem_cons, List.mem_nil_iff, or_false] at hi
    rcases hi with rfl | rfl <;>
      exact (pure_run.1 (by simpa [executeInner] using h)).2
  all_goals exact (pure_run.1 (by simpa [executeInner] using h)).2

/-- The instructions that pop one value. -/
theorem exec_popOne {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {instr : Bytecode} (hi : (∃ o, instr = .brTrue o) ∨ (∃ o, instr = .brFalse o) ∨ instr = .abort)
    (h : (executeInner m view instr).run st = .ok ((), st')) :
    ∃ rest, st.stack = .nonReference :: rest ∧ st' = { st with stack := rest } := by
  rcases hi with ⟨o, rfl⟩ | ⟨o, rfl⟩ | rfl <;>
    exact popValue_run.1 (by simpa [executeInner] using h)

/-- The binary operators: pop two values, push one. -/
theorem exec_binop {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {instr : Bytecode}
    (hi : instr ∈ [.add, .sub, .mul, .mod, .div, .bitOr, .bitAnd, .xor, .shl, .shr, .or, .and,
      .lt, .gt, .le, .ge])
    (h : (executeInner m view instr).run st = .ok ((), st')) :
    ∃ rest, st.stack = .nonReference :: .nonReference :: rest ∧
      st' = { st with stack := .nonReference :: rest } := by
  have h' : (do popValue; popValue; ReferenceSafety.push .nonReference : RefM Unit).run st =
      .ok ((), st') := by
    simp only [List.mem_cons, List.mem_nil_iff, or_false] at hi
    rcases hi with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl |
      rfl | rfl | rfl <;> simpa [executeInner] using h
  rw [run_bind] at h'
  obtain ⟨u, st₁, h1, h'⟩ := h'
  obtain ⟨r1, hs1, rfl⟩ := popValue_run.1 h1
  rw [run_bind] at h'
  obtain ⟨u', st₂, h2, h'⟩ := h'
  obtain ⟨rest, hs2, rfl⟩ := popValue_run.1 h2
  simp only at hs2
  subst hs2
  exact ⟨rest, hs1, push_run.1 h'⟩

/-- The integer and boolean loads: push a value. -/
theorem exec_ld {m : CompiledModule} {view : FunctionView} {st st' : RefState} {instr : Bytecode}
    (hi : instr = .ldTrue ∨ instr = .ldFalse ∨ (∃ v, instr = .ldU8 v) ∨ (∃ v, instr = .ldU16 v) ∨
      (∃ v, instr = .ldU32 v) ∨ (∃ v, instr = .ldU64 v) ∨ (∃ v, instr = .ldU128 v) ∨
      (∃ v, instr = .ldU256 v) ∨ (∃ v, instr = .ldI8 v) ∨ (∃ v, instr = .ldI16 v) ∨
      (∃ v, instr = .ldI32 v) ∨ (∃ v, instr = .ldI64 v) ∨ (∃ v, instr = .ldI128 v) ∨
      (∃ v, instr = .ldI256 v))
    (h : (executeInner m view instr).run st = .ok ((), st')) :
    st' = { st with stack := .nonReference :: st.stack } := by
  have h' : (do ReferenceSafety.push (← valueFor .bool) : RefM Unit).run st = .ok ((), st') ∨
      ∃ ty, refKind ty = none ∧
        (do ReferenceSafety.push (← valueFor ty) : RefM Unit).run st = .ok ((), st') := by
    rcases hi with rfl | rfl | ⟨v, rfl⟩ | ⟨v, rfl⟩ | ⟨v, rfl⟩ | ⟨v, rfl⟩ | ⟨v, rfl⟩ | ⟨v, rfl⟩ |
      ⟨v, rfl⟩ | ⟨v, rfl⟩ | ⟨v, rfl⟩ | ⟨v, rfl⟩ | ⟨v, rfl⟩ | ⟨v, rfl⟩
    all_goals first
      | exact Or.inl (by simpa [executeInner] using h)
      | exact Or.inr ⟨_, rfl, by simpa [executeInner] using h⟩
  rcases h' with h' | ⟨ty, hk, h'⟩
  · rw [run_bind] at h'
    obtain ⟨v, st₁, hv, h'⟩ := h'
    obtain ⟨rfl, rfl⟩ := (valueFor_value (ty := .bool) rfl).1 hv
    exact push_run.1 h'
  · rw [run_bind] at h'
    obtain ⟨v, st₁, hv, h'⟩ := h'
    obtain ⟨rfl, rfl⟩ := (valueFor_value hk).1 hv
    exact push_run.1 h'

/-- A step on values only: the analysis pops `j` non-references and pushes `k`
non-references, and changes nothing else. -/
abbrev ValueStep (j k : Nat) (st st' : RefState) : Prop :=
  j ≤ st.stack.length ∧ (∀ v ∈ st.stack.take j, v = .nonReference) ∧
    st' = { st with stack := List.replicate k .nonReference ++ st.stack.drop j }

theorem ValueStep.refl (st : RefState) : ValueStep 0 0 st st :=
  ⟨Nat.zero_le _, by simp, by simp⟩

theorem ValueStep.pushOne {st : RefState} :
    ValueStep 0 1 st { st with stack := .nonReference :: st.stack } :=
  ⟨Nat.zero_le _, by simp, by simp⟩

theorem ValueStep.popOne {st : RefState} {rest : List AbsValue}
    (hs : st.stack = .nonReference :: rest) : ValueStep 1 0 st { st with stack := rest } :=
  ⟨by simp [hs], by simp [hs], by simp [hs]⟩

theorem packFields_run {n : Nat} {st st' : RefState}
    (h : (packFields n).run st = .ok ((), st')) : ValueStep n 1 st st' := by
  unfold packFields at h
  rw [run_bind] at h
  obtain ⟨u, st₁, hp, h⟩ := h
  obtain ⟨hj, hv, rfl⟩ := popValues_run _ _ _ hp
  rw [push_run] at h
  subst h
  simp only [List.length_range] at hj hv
  exact ⟨hj, hv, by simp⟩

theorem unpackFields_run {n : Nat} {st st' : RefState}
    (h : (unpackFields n).run st = .ok ((), st')) : ValueStep 1 n st st' := by
  unfold unpackFields at h
  rw [run_bind] at h
  obtain ⟨u, st₁, hp, h⟩ := h
  obtain ⟨rest, hs, rfl⟩ := popValue_run.1 hp
  rw [pushValues_run _ _ _ h]
  exact ⟨by simp [hs], by simp [hs], by simp [hs]⟩

/-- A lookup the analysis makes leaves its state alone. -/
theorem lookupBind_run {α β : Type} {t : Array α} {i : Nat} {w : String} {k : α → RefM β} {b : β}
    {st st' : RefState} (h : (liftM (lookup t i w) >>= k : RefM β).run st = .ok (b, st')) :
    ∃ a, t[i]? = some a ∧ (k a).run st = .ok (b, st') := by
  rw [run_bind] at h
  obtain ⟨a, st₁, hl, h⟩ := h
  obtain ⟨ha, rfl⟩ := lookup_run.1 hl
  exact ⟨a, ha, h⟩

theorem structDef_run {m : CompiledModule} {d : Nat} {sd : StructDefinition} {st st' : RefState}
    (h : (structDef m d).run st = .ok (sd, st')) : m.structDefs[d]? = some sd ∧ st' = st :=
  lookup_run.1 h

theorem structFields_run {m : CompiledModule} {d n : Nat} {st st' : RefState}
    (h : (ReferenceSafety.structFields m d).run st = .ok (n, st')) :
    ∃ sd, m.structDefs[d]? = some sd ∧ n = fieldCount sd.fieldInformation none ∧ st' = st := by
  unfold ReferenceSafety.structFields at h
  rw [run_bind] at h
  obtain ⟨sd, st₁, hs, h⟩ := h
  obtain ⟨hsd, rfl⟩ := structDef_run hs
  obtain ⟨rfl, rfl⟩ := pure_run.1 h
  exact ⟨sd, hsd, rfl, rfl⟩

theorem variantFields_run {m : CompiledModule} {h n : Nat} {st st' : RefState}
    (hx : (variantFields m h).run st = .ok (n, st')) :
    ∃ vh sd, m.structVariantHandles[h]? = some vh ∧ m.structDefs[vh.structIndex]? = some sd ∧
      n = fieldCount sd.fieldInformation (some vh.variant) ∧ st' = st := by
  unfold variantFields at hx
  rw [run_bind] at hx
  obtain ⟨vh, st₁, hl, hx⟩ := hx
  obtain ⟨hvh, rfl⟩ := lookup_run.1 hl
  rw [run_bind] at hx
  obtain ⟨sd, st₂, hs, hx⟩ := hx
  obtain ⟨hsd, rfl⟩ := structDef_run hs
  obtain ⟨rfl, rfl⟩ := pure_run.1 hx
  exact ⟨vh, sd, hvh, hsd, rfl, rfl⟩

theorem vecElementType_run {m : CompiledModule} {idx : Nat} {ty : SignatureToken}
    {st st' : RefState} (h : (vecElementType m idx).run st = .ok (ty, st')) :
    (m.signatures[idx]?.bind (·.head?)) = some ty ∧ st' = st := by
  simp only [vecElementType, run_bind] at h
  obtain ⟨sg, st₁, hl, h⟩ := h
  obtain ⟨hsg, rfl⟩ := lookup_run.1 hl
  split at h
  · next t ht =>
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    exact ⟨by simp [hsg, ht], rfl⟩
  · simp [reject, StateT.run, liftM, monadLift, MonadLift.monadLift, StateT.lift, bind,
      Except.bind] at h

theorem exec_pack {m : CompiledModule} {view : FunctionView} {st st' : RefState} {d : Nat}
    (h : (executeInner m view (.pack d)).run st = .ok ((), st')) :
    ∃ sd, m.structDefs[d]? = some sd ∧
      ValueStep (fieldCount sd.fieldInformation none) 1 st st' := by
  simp only [executeInner, run_bind] at h
  obtain ⟨n, st₁, hn, h⟩ := h
  obtain ⟨sd, hsd, rfl, rfl⟩ := structFields_run hn
  exact ⟨sd, hsd, packFields_run h⟩

theorem exec_packVariant {m : CompiledModule} {view : FunctionView} {st st' : RefState} {hh : Nat}
    (h : (executeInner m view (.packVariant hh)).run st = .ok ((), st')) :
    ∃ vh sd, m.structVariantHandles[hh]? = some vh ∧ m.structDefs[vh.structIndex]? = some sd ∧
      ValueStep (fieldCount sd.fieldInformation (some vh.variant)) 1 st st' := by
  simp only [executeInner, run_bind] at h
  obtain ⟨n, st₁, hn, h⟩ := h
  obtain ⟨vh, sd, hvh, hsd, rfl, rfl⟩ := variantFields_run hn
  exact ⟨vh, sd, hvh, hsd, packFields_run h⟩

theorem exec_unpack {m : CompiledModule} {view : FunctionView} {st st' : RefState} {d : Nat}
    (h : (executeInner m view (.unpack d)).run st = .ok ((), st')) :
    ∃ sd, m.structDefs[d]? = some sd ∧
      ValueStep 1 (fieldCount sd.fieldInformation none) st st' := by
  simp only [executeInner, run_bind] at h
  obtain ⟨n, st₁, hn, h⟩ := h
  obtain ⟨sd, hsd, rfl, rfl⟩ := structFields_run hn
  exact ⟨sd, hsd, unpackFields_run h⟩

theorem exec_unpackVariant {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {hh : Nat} (h : (executeInner m view (.unpackVariant hh)).run st = .ok ((), st')) :
    ∃ vh sd, m.structVariantHandles[hh]? = some vh ∧ m.structDefs[vh.structIndex]? = some sd ∧
      ValueStep 1 (fieldCount sd.fieldInformation (some vh.variant)) st st' := by
  simp only [executeInner, run_bind] at h
  obtain ⟨n, st₁, hn, h⟩ := h
  obtain ⟨vh, sd, hvh, hsd, rfl, rfl⟩ := variantFields_run hn
  exact ⟨vh, sd, hvh, hsd, unpackFields_run h⟩

theorem exec_vecPack {m : CompiledModule} {view : FunctionView} {st st' : RefState} {idx n : Nat}
    (h : (executeInner m view (.vecPack idx n)).run st = .ok ((), st')) : ValueStep n 1 st st' := by
  simp only [executeInner, run_bind] at h
  obtain ⟨u, st₁, hp, h⟩ := h
  obtain ⟨hj, hv, rfl⟩ := popValues_run _ _ _ hp
  obtain ⟨ty, st₂, hty, h⟩ := h
  rw [(vecElementType_run hty).2] at h
  obtain ⟨v, st₃, hvf, h⟩ := h
  obtain ⟨rfl, rfl⟩ := (valueFor_value (ty := .vector ty) rfl).1 hvf
  rw [push_run] at h
  subst h
  simp only [List.length_range] at hj hv
  exact ⟨hj, hv, by simp⟩

/-- `clos_pack`: the captures, one per mask bit below the parameter count,
are values, and the closure is a value. -/
theorem closPack_run {m : CompiledModule} {fh : FunctionHandle} {mask : BitVec 64}
    {st st' : RefState} (h : (closPack m fh mask).run st = .ok ((), st')) :
    ∃ params, m.signatures[fh.parameters]? = some params ∧
      ValueStep ((List.range params.length).countP (fun i => mask.getLsbD i)) 1 st st' := by
  simp only [closPack] at h
  obtain ⟨params, hps, h⟩ := lookupBind_run h
  rw [run_bind] at h
  obtain ⟨u, st₁, hp, h⟩ := h
  obtain ⟨hj, hv, rfl⟩ := popValues_run _ _ _ hp
  rw [push_run] at h
  subst h
  simp only [List.length_range] at hj hv
  exact ⟨params, hps, hj, hv, by simp⟩

theorem exec_packClosure {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {h : Nat} {mask : BitVec 64}
    (hx : (executeInner m view (.packClosure h mask)).run st = .ok ((), st')) :
    ∃ fh params, m.functionHandles[h]? = some fh ∧ m.signatures[fh.parameters]? = some params ∧
      ValueStep ((List.range params.length).countP (fun i => mask.getLsbD i)) 1 st st' := by
  simp only [executeInner] at hx
  obtain ⟨fh, hfh, hx⟩ := lookupBind_run hx
  obtain ⟨params, hps, hvs⟩ := closPack_run hx
  exact ⟨fh, params, hfh, hps, hvs⟩

theorem exec_packClosureGeneric {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {i : Nat} {mask : BitVec 64}
    (hx : (executeInner m view (.packClosureGeneric i mask)).run st = .ok ((), st')) :
    ∃ fi fh params, m.functionInstantiations[i]? = some fi ∧
      m.functionHandles[fi.handle]? = some fh ∧ m.signatures[fh.parameters]? = some params ∧
      ValueStep ((List.range params.length).countP (fun i => mask.getLsbD i)) 1 st st' := by
  simp only [executeInner] at hx
  obtain ⟨fi, hfi, hx⟩ := lookupBind_run hx
  obtain ⟨fh, hfh, hx⟩ := lookupBind_run hx
  obtain ⟨params, hps, hvs⟩ := closPack_run hx
  exact ⟨fi, fh, params, hfi, hfh, hps, hvs⟩

theorem pushValueFor_run {ty : SignatureToken} (hk : refKind ty = none) {α : Type} :
    ∀ (l : List α) (st st' : RefState),
      (l.forM fun _ => do ReferenceSafety.push (← valueFor ty)).run st = .ok ((), st') →
        st' = { st with stack := List.replicate l.length .nonReference ++ st.stack }
  | [], st, st', h => by
    simp only [List.forM] at h
    obtain ⟨-, rfl⟩ := pure_run.1 h
    simp
  | a :: l, st, st', h => by
    simp only [List.forM] at h
    rw [run_bind] at h
    obtain ⟨u, st₁, hp, h⟩ := h
    rw [run_bind] at hp
    obtain ⟨v, st₂, hvf, hp⟩ := hp
    obtain ⟨rfl, rfl⟩ := (valueFor_value hk).1 hvf
    rw [push_run] at hp
    subst hp
    rw [pushValueFor_run hk l _ st' h]
    simp [List.replicate_succ']

theorem exec_vecUnpack {m : CompiledModule} {view : FunctionView} {st st' : RefState} {idx n : Nat}
    (hval : ∀ ty, (m.signatures[idx]?.bind (·.head?)) = some ty → refKind ty = none)
    (h : (executeInner m view (.vecUnpack idx n)).run st = .ok ((), st')) :
    ValueStep 1 n st st' := by
  simp only [executeInner, run_bind] at h
  obtain ⟨u, st₁, hp, h⟩ := h
  obtain ⟨rest, hs, rfl⟩ := popValue_run.1 hp
  obtain ⟨ty, st₂, hty, h⟩ := h
  obtain ⟨hty', rfl⟩ := vecElementType_run hty
  rw [pushValueFor_run (hval ty hty') _ _ _ h]
  exact ⟨by simp [hs], by simp [hs], by simp [hs]⟩

theorem exec_ldConst {m : CompiledModule} {view : FunctionView} {st st' : RefState} {i : Nat}
    (hval : ∀ c, m.constantPool[i]? = some c → refKind c.type = none)
    (h : (executeInner m view (.ldConst i)).run st = .ok ((), st')) : ValueStep 0 1 st st' := by
  simp only [executeInner, run_bind] at h
  obtain ⟨c, st₁, hc, h⟩ := h
  obtain ⟨hc', rfl⟩ := lookup_run.1 hc
  obtain ⟨v, st₂, hvf, h⟩ := h
  obtain ⟨rfl, rfl⟩ := (valueFor_value (hval c hc')).1 hvf
  rw [push_run] at h
  subst h
  exact ValueStep.pushOne

theorem exec_abortMsg {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    (h : (executeInner m view .abortMsg).run st = .ok ((), st')) : ValueStep 2 0 st st' := by
  simp only [executeInner, run_bind] at h
  obtain ⟨u, st₁, hp, h⟩ := h
  obtain ⟨r1, hs1, rfl⟩ := popValue_run.1 hp
  obtain ⟨rest, hs2, rfl⟩ := popValue_run.1 h
  simp only at hs2
  exact ⟨by simp [hs1, hs2], by simp [hs1, hs2], by simp [hs1, hs2]⟩

theorem exec_moveFrom {m : CompiledModule} {view : FunctionView} {st st' : RefState} {d : Nat}
    (h : (executeInner m view (.moveFrom d)).run st = .ok ((), st')) :
    st.graph.hasConsistentBorrows st.locals.length (some (.global d)) = .ok false ∧
      ValueStep 1 1 st st' := by
  simp only [executeInner, run_bind] at h
  obtain ⟨u, st₁, hp, h⟩ := h
  obtain ⟨rest, hs, rfl⟩ := popValue_run.1 hp
  obtain ⟨v, st₂, hm, h⟩ := h
  simp only [ReferenceSafety.moveFrom, isGlobalBorrowed, run_bind] at hm
  obtain ⟨b, st₃, hb, hm⟩ := hm
  obtain ⟨r, st₄, hr, hb⟩ := hb
  obtain ⟨rfl, rfl⟩ := frameRoot_run.1 hr
  obtain ⟨hq, rfl⟩ := query_ok.1 hb
  rw [ifReject_run] at hm
  obtain ⟨rfl, hm⟩ := hm
  obtain ⟨rfl, rfl⟩ := pure_run.1 hm
  rw [push_run] at h
  subst h
  exact ⟨hq, ⟨by simp [hs], by simp [hs], by simp [hs]⟩⟩

theorem exec_moveTo {m : CompiledModule} {view : FunctionView} {st st' : RefState} {d : Nat}
    (h : (executeInner m view (.moveTo d)).run st = .ok ((), st')) :
    ∃ v rest, st.stack = .nonReference :: v :: rest ∧
      ((v = .nonReference ∧ st' = { st with stack := rest }) ∨
        ∃ id g, v = .reference id ∧ st.graph.release id = .ok g ∧
          st' = { st with stack := rest, graph := g }) := by
  simp only [executeInner, run_bind] at h
  obtain ⟨u, st₁, hp, h⟩ := h
  obtain ⟨rest₁, hs1, rfl⟩ := popValue_run.1 hp
  obtain ⟨v, st₂, hp2, h⟩ := h
  obtain ⟨rest, hs2, rfl⟩ := pop_run.1 hp2
  simp only at hs2
  subst hs2
  refine ⟨v, rest, hs1, ?_⟩
  cases v with
  | nonReference =>
    simp only [releaseValue] at h
    obtain ⟨-, rfl⟩ := pure_run.1 h
    exact Or.inl ⟨rfl, rfl⟩
  | reference id =>
    simp only [releaseValue] at h
    obtain ⟨g, hg, rfl⟩ := release_run.1 h
    exact Or.inr ⟨id, g, rfl, hg, rfl⟩

/-! ## Calls on values -/

theorem popN_run : ∀ (l : List Nat) (st st' : RefState) (args : List AbsValue),
    (l.mapM fun _ => ReferenceSafety.pop).run st = .ok (args, st') →
      l.length ≤ st.stack.length ∧ args = st.stack.take l.length ∧
        st' = { st with stack := st.stack.drop l.length }
  | [], st, st', args, h => by
    simp only [List.mapM_nil] at h
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    simp
  | a :: l, st, st', args, h => by
    simp only [List.mapM_cons] at h
    rw [run_bind] at h
    obtain ⟨v, st₁, hp, h⟩ := h
    obtain ⟨rest, hs, rfl⟩ := pop_run.1 hp
    rw [run_bind] at h
    obtain ⟨vs, st₂, hvs, h⟩ := h
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    obtain ⟨hl, rfl, rfl⟩ := popN_run l _ _ _ hvs
    simp only at hl
    exact ⟨by simp [hs]; omega, by simp [hs], by simp [hs]⟩

/-- A loop whose body leaves the state alone leaves the state alone. -/
theorem forM_preserve {α : Type} {f : α → RefM PUnit}
    (hf : ∀ a st st' u, (f a).run st = .ok (u, st') → st' = st) :
    ∀ (l : List α) (st st' : RefState) (u : PUnit), (l.forM f).run st = .ok (u, st') → st' = st
  | [], st, st', u, h => by
    simp only [List.forM] at h
    exact (pure_run.1 h).2
  | a :: l, st, st', u, h => by
    simp only [List.forM] at h
    rw [run_bind] at h
    obtain ⟨u₁, st₁, h1, h⟩ := h
    rw [forM_preserve hf l _ _ _ h, hf a _ _ _ h1]

/-- A fold whose step leaves the accumulator and the state alone on values. -/
theorem foldlM_values {β : Type} {f : β → AbsValue → RefM β}
    (hf : ∀ acc st st' r, (f acc .nonReference).run st = .ok (r, st') → r = acc ∧ st' = st) :
    ∀ (args : List AbsValue) (acc : β) (st st' : RefState) (r : β),
      (∀ v ∈ args, v = .nonReference) → (args.foldlM f acc).run st = .ok (r, st') →
        r = acc ∧ st' = st
  | [], acc, st, st', r, _, h => by
    simp only [List.foldlM_nil] at h
    exact ⟨(pure_run.1 h).1.symm, (pure_run.1 h).2⟩
  | v :: args, acc, st, st', r, hv, h => by
    simp only [List.foldlM_cons] at h
    rw [run_bind] at h
    obtain ⟨acc₁, st₁, h1, h⟩ := h
    rw [hv v List.mem_cons_self] at h1
    obtain ⟨rfl, rfl⟩ := hf _ _ _ _ h1
    exact foldlM_values hf args _ _ _ _ (fun w hw => hv w (List.mem_cons_of_mem _ hw)) h

theorem mapM_values {g : SignatureToken → RefM AbsValue}
    (hg : ∀ t st st' v, refKind t = none → (g t).run st = .ok (v, st') →
      v = .nonReference ∧ st' = st) :
    ∀ (tys : List SignatureToken) (st st' : RefState) (vs : List AbsValue),
      (∀ t ∈ tys, refKind t = none) → (tys.mapM g).run st = .ok (vs, st') →
        vs = List.replicate tys.length .nonReference ∧ st' = st
  | [], st, st', vs, _, h => by
    simp only [List.mapM_nil] at h
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    simp
  | t :: tys, st, st', vs, ht, h => by
    simp only [List.mapM_cons] at h
    rw [run_bind] at h
    obtain ⟨v, st₁, h1, h⟩ := h
    obtain ⟨rfl, rfl⟩ := hg t _ _ _ (ht t List.mem_cons_self) h1
    rw [run_bind] at h
    obtain ⟨vs', st₂, h2, h⟩ := h
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    obtain ⟨rfl, rfl⟩ := mapM_values hg tys _ _ _ (fun u hu => ht u (List.mem_cons_of_mem _ hu)) h2
    simp [List.replicate_succ]

theorem coreCall_values {args : List AbsValue} {tys : List SignatureToken}
    {values : List AbsValue} {st st' : RefState} (hargs : ∀ v ∈ args, v = .nonReference)
    (htys : ∀ t ∈ tys, refKind t = none) (h : (coreCall args tys).run st = .ok (values, st')) :
    values = List.replicate tys.length .nonReference ∧ st' = st := by
  unfold coreCall at h
  rw [run_bind] at h
  obtain ⟨⟨all, muts⟩, st₁, hfold, h⟩ := h
  obtain ⟨hacc, rfl⟩ := foldlM_values (fun acc st st' r h => by
      obtain ⟨a, b⟩ := acc
      exact (pure_run.1 h).imp_left Eq.symm) args _ _ _ _ hargs hfold
  simp only [Prod.mk.injEq] at hacc
  obtain ⟨rfl, rfl⟩ := hacc
  rw [run_bind] at h
  obtain ⟨vs, st₂, hmap, h⟩ := h
  obtain ⟨rfl, rfl⟩ := mapM_values (fun t st st' v hk hv => by
      simp only [hk] at hv
      exact (pure_run.1 hv).imp_left Eq.symm) tys _ _ _ htys hmap
  rw [run_bind] at h
  obtain ⟨u, st₃, hrel, h⟩ := h
  simp only [List.forM] at hrel
  obtain ⟨-, rfl⟩ := pure_run.1 hrel
  obtain ⟨rfl, rfl⟩ := pure_run.1 h
  exact ⟨rfl, rfl⟩

theorem pushReplicate_run : ∀ (k : Nat) (st st' : RefState),
    ((List.replicate k AbsValue.nonReference).forM ReferenceSafety.push).run st = .ok ((), st') →
      st' = { st with stack := List.replicate k .nonReference ++ st.stack }
  | 0, st, st', h => by
    simp only [List.replicate, List.forM] at h
    obtain ⟨-, rfl⟩ := pure_run.1 h
    simp
  | k + 1, st, st', h => by
    simp only [List.replicate, List.forM] at h
    rw [run_bind] at h
    obtain ⟨u, st₁, hp, h⟩ := h
    rw [push_run] at hp
    subst hp
    rw [pushReplicate_run k _ _ h]
    simp [List.replicate_succ']

/-- **A call on values**: the arguments are values and the callee returns
values, so the analysis pops the arguments and pushes the results. -/
theorem exec_call_values {m : CompiledModule} {view : FunctionView} {st st' : RefState} {h : Nat}
    (hx : (executeInner m view (.call h)).run st = .ok ((), st')) :
    ∃ fh, m.functionHandles[h]? = some fh ∧
      (borrowFieldMutable fh.attributes = none →
        ∃ ps rs, m.signatures[fh.parameters]? = some ps ∧ m.signatures[fh.returns]? = some rs ∧
          ((∀ v ∈ st.stack.take ps.length, v = .nonReference) → (∀ t ∈ rs, refKind t = none) →
            ValueStep ps.length rs.length st st')) := by
  simp only [executeInner] at hx
  obtain ⟨fh, hfh, hx⟩ := lookupBind_run hx
  refine ⟨fh, hfh, fun hattr => ?_⟩
  simp only [callOrSpecialize, hattr, ReferenceSafety.call] at hx
  obtain ⟨ps, hps, hx⟩ := lookupBind_run hx
  rw [run_bind] at hx
  obtain ⟨args, st₁, hpop, hx⟩ := hx
  obtain ⟨hlen, rfl, rfl⟩ := popN_run _ _ _ _ hpop
  rw [run_bind] at hx
  obtain ⟨acq, st₂, hacq, hx⟩ := hx
  obtain ⟨-, rfl⟩ := lift_run_ok.1 hacq
  obtain ⟨rs, hrs, hx⟩ := lookupBind_run hx
  refine ⟨ps, rs, hps, hrs, fun hargs hr => ?_⟩
  rw [run_bind] at hx
  obtain ⟨u, st₃, hchk, hx⟩ := hx
  have hst₃ := forM_preserve (fun s st st' u h => by
      rw [run_bind] at h
      obtain ⟨b, st₄, hb, h⟩ := h
      simp only [isGlobalBorrowed] at hb
      rw [frameRoot_bind_run] at hb
      obtain ⟨-, rfl⟩ := query_ok.1 hb
      exact (ifRejectUnit_run.1 h).2) _ _ _ _ hchk
  subst hst₃
  rw [run_bind] at hx
  obtain ⟨values, st₄, hcore, hx⟩ := hx
  simp only [List.length_range] at hlen hargs hcore
  obtain ⟨rfl, rfl⟩ := coreCall_values hargs hr hcore
  rw [pushReplicate_run _ _ _ hx]
  simp only
  exact ⟨hlen, hargs, rfl⟩

end MoveBytecode.Proofs
