-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefSteps

/-!
# The reference-safety analysis on calls and returns

UNTRUSTED (checked by the kernel). What the analysis does on `Call` with
reference arguments and results, and on `Ret`: the arguments it checks, the
references it creates for the results and the borrows it records for them,
the arguments it releases; and on `Ret`, the locals it releases and the
checks on the frame and the returned references.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

/-! ## Graph folds -/

/-- Borrow edges from each of `ps` to `c`, as `addBorrow` records them. -/
def addBorrows (g : Graph) (ps : List Nat) (c : Nat) : VResult Graph :=
  ps.foldlM (fun g p => g.addEdge p ⟨false, []⟩ c) g

/-- Release each of `ps`. -/
def releaseAll (g : Graph) (ps : List Nat) : VResult Graph :=
  ps.foldlM (fun g p => g.release p) g

theorem addBorrows_run : ∀ (ps : List Nat) (c : Nat) (st st' : RefState) (u : PUnit),
    (ps.forM fun p => addBorrow p c).run st = .ok (u, st') →
      ∃ g, addBorrows st.graph ps c = .ok g ∧ st' = { st with graph := g }
  | [], c, st, st', u, h => by
    simp only [List.forM] at h
    obtain ⟨-, rfl⟩ := pure_run.1 h
    exact ⟨_, rfl, rfl⟩
  | p :: ps, c, st, st', u, h => by
    simp only [List.forM] at h
    rw [run_bind] at h
    obtain ⟨u₁, st₁, h1, h⟩ := h
    obtain ⟨g1, hg1, rfl⟩ := graphOp_ok.1 h1
    obtain ⟨g, hg, rfl⟩ := addBorrows_run ps c _ _ _ h
    exact ⟨g, by simp [addBorrows, List.foldlM_cons, hg1, bind, Except.bind]; exact hg, rfl⟩

theorem releaseAll_run : ∀ (ps : List Nat) (st st' : RefState) (u : PUnit),
    (ps.forM ReferenceSafety.release).run st = .ok (u, st') →
      ∃ g, releaseAll st.graph ps = .ok g ∧ st' = { st with graph := g }
  | [], st, st', u, h => by
    simp only [List.forM] at h
    obtain ⟨-, rfl⟩ := pure_run.1 h
    exact ⟨_, rfl, rfl⟩
  | p :: ps, st, st', u, h => by
    simp only [List.forM] at h
    rw [run_bind] at h
    obtain ⟨u₁, st₁, h1, h⟩ := h
    obtain ⟨g1, hg1, rfl⟩ := release_run.1 h1
    obtain ⟨g, hg, rfl⟩ := releaseAll_run ps _ _ _ h
    exact ⟨g, by simp [releaseAll, List.foldlM_cons, hg1, bind, Except.bind]; exact hg, rfl⟩

/-! ## `coreCall` -/

/-- One argument of `coreCall`'s fold. -/
def callArg (acc : List Nat × List Nat) (v : AbsValue) : RefM (List Nat × List Nat) :=
  match acc with
  | (all, muts) => do
    match v with
    | .nonReference => pure (all, muts)
    | .reference id =>
      if ← isMutable id then
        if !(← isWritable id) then reject "CALL_BORROWED_MUTABLE_REFERENCE_ERROR"
        pure (insertNat id all, insertNat id muts)
      else pure (insertNat id all, muts)

/-- One result of `coreCall`. -/
def callResult (all muts : List Nat) (ty : SignatureToken) : RefM AbsValue := do
  match refKind ty with
  | some true =>
    let id ← newRef true
    muts.forM fun p => addBorrow p id
    pure (.reference id)
  | some false =>
    let id ← newRef false
    all.forM fun p => addBorrow p id
    pure (.reference id)
  | none => pure .nonReference

theorem coreCall_eq (args : List AbsValue) (tys : List SignatureToken) :
    coreCall args tys = (do
      let (all, muts) ← args.foldlM callArg ([], [])
      let values ← tys.mapM (callResult all muts)
      all.forM release
      pure values) := rfl

/-- **The arguments' fold**: it checks that every mutable reference argument
is writable, and collects the reference arguments and the mutable ones. -/
theorem callArgs_run : ∀ (args : List AbsValue) (all muts : List Nat) (st st' : RefState)
    (r : List Nat × List Nat), (args.foldlM callArg (all, muts)).run st = .ok (r, st') →
      st' = st ∧ (∀ x, x ∈ r.1 ↔ x ∈ all ∨ .reference x ∈ args) ∧
      (∀ x, x ∈ r.2 ↔ x ∈ muts ∨ (.reference x ∈ args ∧ st.graph.isMutable x = .ok true)) ∧
      (∀ x, .reference x ∈ args → ∃ b, st.graph.isMutable x = .ok b) ∧
      (∀ x, .reference x ∈ args → st.graph.isMutable x = .ok true →
        st.graph.isWritable x = .ok true)
  | [], all, muts, st, st', r, h => by
    simp only [List.foldlM_nil] at h
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    simp
  | v :: args, all, muts, st, st', r, h => by
    simp only [List.foldlM_cons] at h
    rw [run_bind] at h
    obtain ⟨⟨all₁, muts₁⟩, st₁, h1, h⟩ := h
    -- One argument.
    have hstep : st₁ = st ∧ (∀ x, x ∈ all₁ ↔ x ∈ all ∨ v = .reference x) ∧
        (∀ x, x ∈ muts₁ ↔ x ∈ muts ∨ (v = .reference x ∧ st.graph.isMutable x = .ok true)) ∧
        (∀ x, v = .reference x → ∃ b, st.graph.isMutable x = .ok b) ∧
        (∀ x, v = .reference x → st.graph.isMutable x = .ok true →
          st.graph.isWritable x = .ok true) := by
      cases v with
      | nonReference =>
        simp only [callArg] at h1
        obtain ⟨he, rfl⟩ := pure_run.1 h1
        simp only [Prod.mk.injEq] at he
        obtain ⟨rfl, rfl⟩ := he
        simp
      | reference id =>
        simp only [callArg] at h1
        rw [run_bind] at h1
        obtain ⟨b, st₂, hb, h1⟩ := h1
        obtain ⟨hq, rfl⟩ := isMutable_run.1 hb
        cases b with
        | true =>
          simp only [if_true] at h1
          rw [run_bind] at h1
          obtain ⟨w, st₃, hw, h1⟩ := h1
          obtain ⟨hwq, rfl⟩ := isWritable_run.1 hw
          cases w with
          | false => exact absurd h1 reject_bind_run
          | true =>
            simp only [Bool.not_true, Bool.false_eq_true, if_false] at h1
            obtain ⟨he, rfl⟩ := pure_run.1 h1
            simp only [Prod.mk.injEq] at he
            obtain ⟨rfl, rfl⟩ := he
            refine ⟨rfl, fun x => ?_, fun x => ?_, fun x hx => ?_, fun x hx _ => ?_⟩
            · rw [mem_insertNat]; simp [eq_comm, or_comm]
            · rw [mem_insertNat]
              constructor
              · rintro (rfl | h)
                · exact Or.inr ⟨rfl, hq⟩
                · exact Or.inl h
              · rintro (h | ⟨h, -⟩)
                · exact Or.inr h
                · simp only [AbsValue.reference.injEq] at h; exact Or.inl h.symm
            · simp only [AbsValue.reference.injEq] at hx; subst hx; exact ⟨true, hq⟩
            · simp only [AbsValue.reference.injEq] at hx; subst hx; exact hwq
        | false =>
          simp only [Bool.false_eq_true, if_false] at h1
          obtain ⟨he, rfl⟩ := pure_run.1 h1
          simp only [Prod.mk.injEq] at he
          obtain ⟨rfl, rfl⟩ := he
          refine ⟨rfl, fun x => ?_, fun x => ?_, fun x hx => ?_, fun x hx hm => ?_⟩
          · rw [mem_insertNat]; simp [eq_comm, or_comm]
          · constructor
            · exact Or.inl
            · rintro (h | ⟨h, hm⟩)
              · exact h
              · simp only [AbsValue.reference.injEq] at h; subst h
                rw [hq] at hm; cases hm
          · simp only [AbsValue.reference.injEq] at hx; subst hx; exact ⟨false, hq⟩
          · simp only [AbsValue.reference.injEq] at hx; subst hx
            rw [hq] at hm; cases hm
    obtain ⟨rfl, ha1, hm1, hk1, hw1⟩ := hstep
    obtain ⟨rfl, ha, hm, hk, hw⟩ := callArgs_run args all₁ muts₁ _ _ r h
    refine ⟨rfl, fun x => ?_, fun x => ?_, fun x hx => ?_, fun x hx => ?_⟩
    · rw [ha, ha1, List.mem_cons]
      constructor
      · rintro ((h | h) | h)
        · exact Or.inl h
        · exact Or.inr (Or.inl h.symm)
        · exact Or.inr (Or.inr h)
      · rintro (h | h | h)
        · exact Or.inl (Or.inl h)
        · exact Or.inl (Or.inr h.symm)
        · exact Or.inr h
    · rw [hm, hm1, List.mem_cons]
      constructor
      · rintro ((h | ⟨h, h'⟩) | ⟨h, h'⟩)
        · exact Or.inl h
        · exact Or.inr ⟨Or.inl h.symm, h'⟩
        · exact Or.inr ⟨Or.inr h, h'⟩
      · rintro (h | ⟨h | h, h'⟩)
        · exact Or.inl (Or.inl h)
        · exact Or.inl (Or.inr ⟨h.symm, h'⟩)
        · exact Or.inr ⟨h, h'⟩
    · rcases List.mem_cons.1 hx with h | h
      · exact hk1 x h.symm
      · exact hk x h
    · rcases List.mem_cons.1 hx with h | h
      · exact hw1 x h.symm
      · exact hw x h

/-- **One result**: a value, or a new reference borrowed from every argument
it may come from. -/
theorem callResult_run {all muts : List Nat} {ty : SignatureToken} {v : AbsValue}
    {st st' : RefState} (h : (callResult all muts ty).run st = .ok (v, st')) :
    (refKind ty = none ∧ v = .nonReference ∧ st' = st) ∨
      ∃ m g1 g2, refKind ty = some m ∧ v = .reference st.nextId ∧
        st.graph.newRef st.nextId m = .ok g1 ∧
        addBorrows g1 (if m then muts else all) st.nextId = .ok g2 ∧
        st' = { st with graph := g2, nextId := st.nextId + 1 } := by
  unfold callResult at h
  split at h
  · next hk =>
    rw [run_bind] at h
    obtain ⟨id, st₁, hn, h⟩ := h
    obtain ⟨g1, hg1, rfl, rfl⟩ := newRef_run.1 hn
    rw [run_bind] at h
    obtain ⟨u, st₂, hb, h⟩ := h
    obtain ⟨g2, hg2, rfl⟩ := addBorrows_run _ _ _ _ _ hb
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    exact Or.inr ⟨true, g1, g2, hk, rfl, hg1, hg2, rfl⟩
  · next hk =>
    rw [run_bind] at h
    obtain ⟨id, st₁, hn, h⟩ := h
    obtain ⟨g1, hg1, rfl, rfl⟩ := newRef_run.1 hn
    rw [run_bind] at h
    obtain ⟨u, st₂, hb, h⟩ := h
    obtain ⟨g2, hg2, rfl⟩ := addBorrows_run _ _ _ _ _ hb
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    exact Or.inr ⟨false, g1, g2, hk, rfl, hg1, hg2, rfl⟩
  · next hk =>
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    exact Or.inl ⟨hk, rfl, rfl⟩

theorem insertNat_sorted (x : Nat) : ∀ {l : List Nat}, l.Pairwise (· < ·) →
    (insertNat x l).Pairwise (· < ·)
  | [], _ => by simp [insertNat]
  | y :: ys, h => by
    unfold insertNat
    by_cases h1 : x < y
    · simp only [h1, ↓reduceIte]
      refine List.Pairwise.cons (fun z hz => ?_) h
      rcases List.mem_cons.1 hz with rfl | hz
      · exact h1
      · exact Nat.lt_trans h1 (List.rel_of_pairwise_cons h hz)
    · by_cases h2 : x = y
      · subst h2; simpa using h
      · have : (x == y) = false := by simpa using h2
        simp only [h1, ↓reduceIte, this, Bool.false_eq_true]
        refine List.Pairwise.cons (fun z hz => ?_) (insertNat_sorted x (List.Pairwise.of_cons h))
        rcases (mem_insertNat z x ys).1 hz with rfl | hz
        · omega
        · exact List.rel_of_pairwise_cons h hz

theorem callArgs_sorted : ∀ (args : List AbsValue) (all muts : List Nat) (st st' : RefState)
    (r : List Nat × List Nat), all.Pairwise (· < ·) →
      (args.foldlM callArg (all, muts)).run st = .ok (r, st') → r.1.Pairwise (· < ·)
  | [], all, muts, st, st', r, hs, h => by
    simp only [List.foldlM_nil] at h
    obtain ⟨rfl, rfl⟩ := pure_run.1 h
    exact hs
  | v :: args, all, muts, st, st', r, hs, h => by
    simp only [List.foldlM_cons] at h
    rw [run_bind] at h
    obtain ⟨⟨all₁, muts₁⟩, st₁, h1, h⟩ := h
    refine callArgs_sorted args all₁ muts₁ _ _ r ?_ h
    cases v with
    | nonReference =>
      simp only [callArg] at h1
      obtain ⟨he, -⟩ := pure_run.1 h1
      simp only [Prod.mk.injEq] at he
      obtain ⟨rfl, rfl⟩ := he
      exact hs
    | reference id =>
      simp only [callArg] at h1
      rw [run_bind] at h1
      obtain ⟨b, st₂, -, h1⟩ := h1
      cases b with
      | true =>
        simp only [if_true] at h1
        rw [run_bind] at h1
        obtain ⟨w, st₃, -, h1⟩ := h1
        cases w with
        | false => exact absurd h1 reject_bind_run
        | true =>
          simp only [Bool.not_true, Bool.false_eq_true, if_false] at h1
          obtain ⟨he, -⟩ := pure_run.1 h1
          simp only [Prod.mk.injEq] at he
          obtain ⟨rfl, rfl⟩ := he
          exact insertNat_sorted id hs
      | false =>
        simp only [Bool.false_eq_true, if_false] at h1
        obtain ⟨he, -⟩ := pure_run.1 h1
        simp only [Prod.mk.injEq] at he
        obtain ⟨rfl, rfl⟩ := he
        exact insertNat_sorted id hs

theorem pushList_run : ∀ (vals : List AbsValue) (st st' : RefState) (u : PUnit),
    (vals.forM ReferenceSafety.push).run st = .ok (u, st') →
      st' = { st with stack := vals.reverse ++ st.stack }
  | [], st, st', u, h => by
    simp only [List.forM] at h
    obtain ⟨-, rfl⟩ := pure_run.1 h
    simp
  | v :: vals, st, st', u, h => by
    simp only [List.forM] at h
    rw [run_bind] at h
    obtain ⟨u₁, st₁, hp, h⟩ := h
    rw [push_run] at hp
    subst hp
    rw [pushList_run vals _ _ _ h]
    simp

/-- **`Call`**: the analysis pops the arguments, checks that each mutable
reference argument is writable, creates a reference for each reference
result borrowed from the arguments it may come from, releases the arguments,
and pushes the results. -/
theorem exec_call {m : CompiledModule} {view : FunctionView} {st st' : RefState} {h : Nat}
    (hx : (executeInner m view (.call h)).run st = .ok ((), st')) :
    ∃ fh, m.functionHandles[h]? = some fh ∧
      (borrowFieldMutable fh.attributes = none →
        ∃ ps rs all muts vals st₂ g, m.signatures[fh.parameters]? = some ps ∧
          m.signatures[fh.returns]? = some rs ∧ ps.length ≤ st.stack.length ∧
          (∀ x, x ∈ all ↔ .reference x ∈ st.stack.take ps.length) ∧
          (∀ x, x ∈ muts ↔
            .reference x ∈ st.stack.take ps.length ∧ st.graph.isMutable x = .ok true) ∧
          (∀ x, .reference x ∈ st.stack.take ps.length → ∃ b, st.graph.isMutable x = .ok b) ∧
          (∀ x, .reference x ∈ st.stack.take ps.length → st.graph.isMutable x = .ok true →
            st.graph.isWritable x = .ok true) ∧
          all.Pairwise (· < ·) ∧
          (rs.mapM (callResult all muts)).run { st with stack := st.stack.drop ps.length } =
            .ok (vals, st₂) ∧
          releaseAll st₂.graph all = .ok g ∧
          st' = { st₂ with graph := g, stack := vals.reverse ++ st₂.stack }) := by
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
  simp only [List.length_range] at hlen hcore
  rw [coreCall_eq] at hcore
  rw [run_bind] at hcore
  obtain ⟨⟨all, muts⟩, st₅, hfold, hcore⟩ := hcore
  have hsorted := callArgs_sorted _ _ _ _ _ _ List.Pairwise.nil hfold
  obtain ⟨rfl, hall, hmuts, hkind, hwr⟩ := callArgs_run _ _ _ _ _ _ hfold
  rw [run_bind] at hcore
  obtain ⟨vals, st₆, hmap, hcore⟩ := hcore
  rw [run_bind] at hcore
  obtain ⟨u', st₇, hrel, hcore⟩ := hcore
  obtain ⟨g, hg, rfl⟩ := releaseAll_run _ _ _ _ hrel
  obtain ⟨rfl, rfl⟩ := pure_run.1 hcore
  refine ⟨ps, rs, all, muts, vals, st₆, g, hps, hrs, hlen, fun x => ?_, fun x => ?_, hkind, hwr,
    hsorted, hmap, hg, ?_⟩
  · rw [hall]; simp
  · rw [hmuts]; simp
  · rw [pushList_run _ _ _ _ hx]

theorem funType_run {m : CompiledModule} {idx : Nat} {ps rs : List SignatureToken}
    {st st' : RefState} (h : (funType m idx).run st = .ok ((ps, rs), st')) :
    (∃ ab, (m.signatures[idx]?.bind (·.head?)) = some (.function ps rs ab)) ∧ st' = st := by
  simp only [funType, run_bind] at h
  obtain ⟨sg, st₁, hl, h⟩ := h
  obtain ⟨hsg, rfl⟩ := lookup_run.1 hl
  split at h
  · next t ht =>
    split at h
    · next args results ab =>
      simp only [StateT.run, pure, StateT.pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨⟨rfl, rfl⟩, rfl⟩ := h
      exact ⟨⟨ab, by simp [hsg, ht]⟩, rfl⟩
    all_goals simp [reject, StateT.run, liftM, monadLift, MonadLift.monadLift, StateT.lift, bind,
      Except.bind] at h
  · simp [reject, StateT.run, liftM, monadLift, MonadLift.monadLift, StateT.lift, bind,
      Except.bind] at h

/-- **The analysis's core of a call** (`coreCall`) of the `n` arguments on top
of `A`'s stack, with result tokens `rs`: it collects the reference arguments
`all` and the mutable ones `muts`, each mutable one writable, creates the
results, releases the arguments and pushes the results. `Call` runs it on the
state at the call (`exec_call_core`), `CallClosure` once the closure is popped
(`exec_callClosure`). -/
def CoreRun (A A' : RefState) (n : Nat) (rs : List SignatureToken) : Prop :=
  ∃ all muts vals st₂ g, n ≤ A.stack.length ∧
    (∀ x, x ∈ all ↔ .reference x ∈ A.stack.take n) ∧
    (∀ x, x ∈ muts ↔ .reference x ∈ A.stack.take n ∧ A.graph.isMutable x = .ok true) ∧
    (∀ x, .reference x ∈ A.stack.take n → ∃ b, A.graph.isMutable x = .ok b) ∧
    (∀ x, .reference x ∈ A.stack.take n → A.graph.isMutable x = .ok true →
      A.graph.isWritable x = .ok true) ∧
    all.Pairwise (· < ·) ∧
    (rs.mapM (callResult all muts)).run { A with stack := A.stack.drop n } = .ok (vals, st₂) ∧
    releaseAll st₂.graph all = .ok g ∧
    A' = { st₂ with graph := g, stack := vals.reverse ++ st₂.stack }

/-- `Call`'s core. -/
theorem exec_call_core {m : CompiledModule} {view : FunctionView} {st st' : RefState} {h : Nat}
    (hx : (executeInner m view (.call h)).run st = .ok ((), st')) {fh : FunctionHandle}
    (hfh : m.functionHandles[h]? = some fh) (hattr : borrowFieldMutable fh.attributes = none) :
    ∃ ps rs, m.signatures[fh.parameters]? = some ps ∧ m.signatures[fh.returns]? = some rs ∧
      CoreRun st st' ps.length rs := by
  obtain ⟨fh', hfh', hcall⟩ := exec_call hx
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  obtain ⟨ps, rs, all, muts, vals, st₂, g, hps, hrs, h1, h2, h3, h4, h5, h6, h7, h8, h9⟩ :=
    hcall hattr
  exact ⟨ps, rs, hps, hrs, all, muts, vals, st₂, g, h1, h2, h3, h4, h5, h6, h7, h8, h9⟩

/-- **`CallClosure`**: the analysis pops the closure, then runs the core of a
call (`CoreRun`) of the function type the instruction names, with no
acquires check. -/
theorem exec_callClosure {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    {idx : Nat} (hx : (executeInner m view (.callClosure idx)).run st = .ok ((), st')) :
    ∃ ps rs ab clo rest, (m.signatures[idx]?.bind (·.head?)) = some (.function ps rs ab) ∧
      st.stack = clo :: rest ∧ CoreRun { st with stack := rest } st' ps.length rs := by
  simp only [executeInner, run_bind] at hx
  obtain ⟨⟨ps, rs⟩, st₀, hft, hx⟩ := hx
  obtain ⟨⟨ab, hhead⟩, rfl⟩ := funType_run hft
  obtain ⟨clo, st₁, hp, hx⟩ := hx
  obtain ⟨rest, hs, rfl⟩ := pop_run.1 hp
  obtain ⟨args, st₂, hpop, hx⟩ := hx
  obtain ⟨hlen, rfl, rfl⟩ := popN_run _ _ _ _ hpop
  obtain ⟨values, st₄, hcore, hx⟩ := hx
  simp only [List.length_range] at hlen hcore
  rw [coreCall_eq] at hcore
  rw [run_bind] at hcore
  obtain ⟨⟨all, muts⟩, st₅, hfold, hcore⟩ := hcore
  have hsorted := callArgs_sorted _ _ _ _ _ _ List.Pairwise.nil hfold
  obtain ⟨rfl, hall, hmuts, hkind, hwr⟩ := callArgs_run _ _ _ _ _ _ hfold
  rw [run_bind] at hcore
  obtain ⟨vals, st₆, hmap, hcore⟩ := hcore
  rw [run_bind] at hcore
  obtain ⟨u', st₇, hrel, hcore⟩ := hcore
  obtain ⟨g, hg, rfl⟩ := releaseAll_run _ _ _ _ hrel
  obtain ⟨rfl, rfl⟩ := pure_run.1 hcore
  refine ⟨ps, rs, ab, clo, rest, hhead, hs, all, muts, vals, st₆, g, hlen, fun x => ?_,
    fun x => ?_, hkind, hwr, hsorted, hmap, hg, ?_⟩
  · rw [hall]; simp
  · rw [hmuts]; simp
  · rw [pushList_run _ _ _ _ hx]

/-- **A struct-API mutable borrow call**: the analysis pops the reference
argument and borrows field `off` of it mutably, exactly as `MutBorrowField`
does (`exec_mutBorrowField`). -/
theorem exec_callApi {m : CompiledModule} {view : FunctionView} {st st' : RefState} {h : Nat}
    (hx : (executeInner m view (.call h)).run st = .ok ((), st')) {fh : FunctionHandle}
    (hfh : m.functionHandles[h]? = some fh) {off : Nat}
    (hoff : borrowFieldMutable fh.attributes = some off) :
    ∃ id rest g1 g2 g3, st.stack = .reference id :: rest ∧
      st.graph.hasFullBorrows id = .ok false ∧
      st.graph.newRef st.nextId true = .ok g1 ∧ g1.factor id [.field off] st.nextId = .ok g2 ∧
      g2.release id = .ok g3 ∧
      st' = { st with
        stack := .reference st.nextId :: rest, graph := g3, nextId := st.nextId + 1 } := by
  simp only [executeInner] at hx
  obtain ⟨fh', hfh', hx⟩ := lookupBind_run hx
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  simp only [callOrSpecialize, hoff, run_bind] at hx
  obtain ⟨id, st₁, hp, hx⟩ := hx
  obtain ⟨rest, hs, rfl⟩ := popRef_run.1 hp
  obtain ⟨v, st₃, hb, hx⟩ := hx
  obtain ⟨g1, g2, g3, h1, -, h3, h4, h5, rfl, rfl⟩ := borrowField_run hb
  exact ⟨id, rest, g1, g2, g3, hs, h1 rfl, h3, h4, h5, push_run.1 hx⟩

theorem forM_each {α : Type} {f : α → RefM PUnit} {Q : RefState → α → Prop}
    (hf : ∀ a st st' u, (f a).run st = .ok (u, st') → st' = st ∧ Q st a) :
    ∀ (l : List α) (st st' : RefState) (u : PUnit), (l.forM f).run st = .ok (u, st') →
      st' = st ∧ ∀ a ∈ l, Q st a
  | [], st, st', u, h => by
    simp only [List.forM] at h
    exact ⟨(pure_run.1 h).2, fun a ha => by simp at ha⟩
  | a :: l, st, st', u, h => by
    simp only [List.forM] at h
    rw [run_bind] at h
    obtain ⟨u₁, st₁, h1, h⟩ := h
    obtain ⟨hst, hq⟩ := hf a st st₁ u₁ h1
    rw [hst] at h
    obtain ⟨hst', hl⟩ := forM_each hf l st st' u h
    exact ⟨hst', fun b hb => by
      rcases List.mem_cons.1 hb with rfl | hb
      · exact hq
      · exact hl b hb⟩

/-- **The call's check of the callee's acquires**: no type the callee
acquires is globally borrowed by the caller. -/
theorem exec_call_acquires {m : CompiledModule} {view : FunctionView} {st st' : RefState} {h : Nat}
    (hx : (executeInner m view (.call h)).run st = .ok ((), st')) :
    ∃ fh, m.functionHandles[h]? = some fh ∧
      (borrowFieldMutable fh.attributes = none →
        ∃ acq, ReferenceSafety.acquiredBy m fh = .ok acq ∧
          ∀ d ∈ acq, st.graph.hasConsistentBorrows st.locals.length (some (.global d)) = .ok false) := by
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
  obtain ⟨hacq', rfl⟩ := lift_run_ok.1 hacq
  obtain ⟨rs, hrs, hx⟩ := lookupBind_run hx
  rw [run_bind] at hx
  obtain ⟨u, st₃, hchk, hx⟩ := hx
  obtain ⟨-, hq⟩ := forM_each (Q := fun st d =>
      st.graph.hasConsistentBorrows st.locals.length (some (.global d)) = .ok false)
    (fun s st st' u h => by
      rw [run_bind] at h
      obtain ⟨b, st₄, hb, h⟩ := h
      simp only [isGlobalBorrowed] at hb
      rw [frameRoot_bind_run] at hb
      obtain ⟨hq, rfl⟩ := query_ok.1 hb
      obtain ⟨hbf, rfl⟩ := ifRejectUnit_run.1 h
      exact ⟨rfl, by rw [← hbf]; exact hq⟩) _ _ _ _ hchk
  exact ⟨acq, hacq', hq⟩

/-! ## `Ret` -/

/-- One step of collecting the locals' references. -/
def refStep (s : List Nat) (v : AbsValue) : List Nat :=
  match v with
  | .reference id => insertNat id s
  | .nonReference => s

/-- The references the locals hold, as `ret` collects them. -/
def localRefs (locals : List AbsValue) : List Nat :=
  locals.foldl refStep []

/-- `ret`'s check on one returned value. -/
def retValue (v : AbsValue) : RefM Unit := do
  match v with
  | .reference id =>
    if ← isMutable id then
      if !(← isWritable id) then reject "RET_BORROWED_MUTABLE_REFERENCE_ERROR"
  | .nonReference => pure ()

theorem ret_eq (values : List AbsValue) :
    ReferenceSafety.ret values = (do
      let released := localRefs (← get).locals
      released.forM release
      let root ← frameRoot
      if ← query (·.hasConsistentBorrows root none) then
        reject "UNSAFE_RET_LOCAL_OR_RESOURCE_STILL_BORROWED"
      values.forM retValue) := rfl

theorem retValues_run : ∀ (vals : List AbsValue) (st st' : RefState) (u : PUnit),
    (vals.forM retValue).run st = .ok (u, st') →
      st' = st ∧ ∀ id, .reference id ∈ vals → st.graph.isMutable id = .ok true →
        st.graph.isWritable id = .ok true
  | [], st, st', u, h => by
    simp only [List.forM] at h
    obtain ⟨-, rfl⟩ := pure_run.1 h
    simp
  | v :: vals, st, st', u, h => by
    simp only [List.forM] at h
    rw [run_bind] at h
    obtain ⟨u₁, st₁, h1, h⟩ := h
    have hone : st₁ = st ∧ ∀ id, v = .reference id → st.graph.isMutable id = .ok true →
        st.graph.isWritable id = .ok true := by
      cases v with
      | nonReference =>
        simp only [retValue] at h1
        obtain ⟨-, rfl⟩ := pure_run.1 h1
        exact ⟨rfl, fun id h => by cases h⟩
      | reference id =>
        simp only [retValue] at h1
        rw [run_bind] at h1
        obtain ⟨b, st₂, hb, h1⟩ := h1
        obtain ⟨hq, rfl⟩ := isMutable_run.1 hb
        cases b with
        | false =>
          simp only [Bool.false_eq_true, if_false] at h1
          obtain ⟨-, rfl⟩ := pure_run.1 h1
          refine ⟨rfl, fun id' he hm => ?_⟩
          simp only [AbsValue.reference.injEq] at he
          subst he
          rw [hq] at hm; cases hm
        | true =>
          simp only [if_true] at h1
          rw [run_bind] at h1
          obtain ⟨w, st₃, hw, h1⟩ := h1
          obtain ⟨hwq, rfl⟩ := isWritable_run.1 hw
          cases w with
          | false =>
            simp [reject, StateT.run, liftM, monadLift, MonadLift.monadLift, StateT.lift, bind,
              Except.bind] at h1
          | true =>
            simp only [Bool.not_true, Bool.false_eq_true, if_false] at h1
            obtain ⟨-, rfl⟩ := pure_run.1 h1
            refine ⟨rfl, fun id' he _ => ?_⟩
            simp only [AbsValue.reference.injEq] at he
            subst he
            exact hwq
    obtain ⟨rfl, h1⟩ := hone
    obtain ⟨rfl, h2⟩ := retValues_run vals _ _ _ h
    refine ⟨rfl, fun id hid hm => ?_⟩
    rcases List.mem_cons.1 hid with he | he
    · exact h1 id he.symm hm
    · exact h2 id he hm

/-- **`Ret`**: the analysis releases the locals' references; then nothing may
borrow from the frame, and every returned mutable reference is writable. -/
theorem exec_ret {m : CompiledModule} {view : FunctionView} {st st' : RefState}
    (hx : (executeInner m view .ret).run st = .ok ((), st')) :
    view.returns.length ≤ st.stack.length ∧ ∃ g, releaseAll st.graph (localRefs st.locals) = .ok g ∧
      g.hasConsistentBorrows st.locals.length none = .ok false ∧
      ∀ id, .reference id ∈ st.stack.take view.returns.length → g.isMutable id = .ok true →
        g.isWritable id = .ok true := by
  simp only [executeInner] at hx
  rw [run_bind] at hx
  obtain ⟨values, st₁, hpop, hx⟩ := hx
  obtain ⟨hlen, rfl, rfl⟩ := popN_run _ _ _ _ hpop
  simp only [List.length_range] at hlen
  refine ⟨hlen, ?_⟩
  rw [ret_eq] at hx
  rw [run_bind] at hx
  obtain ⟨st₂, st₃, hget, hx⟩ := hx
  have hst : st₂ = { st with stack := st.stack.drop (List.range view.returns.length).length } ∧
      st₃ = { st with stack := st.stack.drop (List.range view.returns.length).length } := by
    simp only [StateT.run, get, getThe, MonadStateOf.get, StateT.get, pure, Except.pure,
      Except.ok.injEq, Prod.mk.injEq] at hget
    exact ⟨hget.1.symm, hget.2.symm⟩
  obtain ⟨rfl, rfl⟩ := hst
  rw [run_bind] at hx
  obtain ⟨u, st₄, hrel, hx⟩ := hx
  obtain ⟨g, hg, rfl⟩ := releaseAll_run _ _ _ _ hrel
  rw [frameRoot_bind_run] at hx
  rw [run_bind] at hx
  obtain ⟨b, st₅, hb, hx⟩ := hx
  obtain ⟨hq, rfl⟩ := query_ok.1 hb
  cases b with
  | true => exact absurd hx reject_bind_run
  | false =>
    simp only [Bool.false_eq_true, if_false] at hx
    obtain ⟨-, hchk⟩ := retValues_run _ _ _ _ hx
    exact ⟨g, hg, hq, fun id hid hm => hchk id (List.mem_reverse.2 (by simpa using hid)) hm⟩

end MoveBytecode.Proofs
