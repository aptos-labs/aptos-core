-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Verify.SimpAttrs
import Move.Semantics.Checked
import Move.Semantics.Reference

/-!
# Direct source contracts

Contracts are ordinary Lean predicates over source transaction semantics.
They deliberately cannot mention reference identities or prophecy variables.
-/

namespace Move.Verify

open Move.Semantics

/-- A source contract.  Abort behavior has two independent components: which
abort outcomes the contract permits, and where a declared abort excuses the
postcondition.  They coincide for declared conditions, but not when abort
behavior is left uninterpreted: then every code is permitted while nothing is
excused. -/
structure Contract (State Args Result : Type) where
  /-- States in which callers may invoke the function. -/
  requires : Args → State → Prop
  /-- Relation established by a successful execution. -/
  ensures : Args → State → Result → State → Prop
  /-- Abort outcomes the contract permits.  Uninterpreted abort behavior
  permits every code. -/
  aborts : Args → State → Nat → Prop
  /-- States where a declared abort excuses the postcondition.  This is the
  disjunction of the declared abort conditions, and `False` when no abort
  condition is declared — uninterpreted aborts excuse nothing, so successful
  executions must still establish `ensures`. -/
  mayAbort : Args → State → Prop
  /-- What a successful execution leaves unchanged.  A specification changes
  only the global memory its `modifies` clause lists, so this defaults to
  changing nothing at all and is never written by hand. -/
  frame : Args → State → State → Prop := fun _ initial final => final = initial

/-- A relational source computation satisfies its contract for every
permitted initial state.  Two things hold of every successful execution: the
frame, unconditionally, and the postcondition wherever the declared aborts
are ruled out.  Both readings are these semantics — not anything written in
the clauses. -/
def Satisfies (function : Args → Spec State Result)
    (contract : Contract State Args Result) : Prop :=
  ∀ args initial,
    contract.requires args initial →
      (∀ result final, (function args).ok initial result final →
        (¬contract.mayAbort args initial →
          contract.ensures args initial result final) ∧
        contract.frame args initial final) ∧
      (∀ code, (function args).aborts initial code →
        contract.aborts args initial code)

/-- Weakest precondition of the relational verification semantics. -/
def wp (action : Spec State Result)
    (ensures : Result → State → Prop) (aborts : Nat → Prop)
    (initial : State) : Prop :=
  (∀ result final, action.ok initial result final → ensures result final) ∧
  (∀ code, action.aborts initial code → aborts code)

@[simp, wp_norm] theorem wp_pure (value : Result) (state : State)
    (ensures : Result → State → Prop) (aborts : Nat → Prop) :
    wp (Spec.pure value) ensures aborts state ↔ ensures value state := by
  simp [wp, Spec.pure]

@[simp, wp_norm] theorem wp_abort (code : Nat) (state : State)
    (ensures : Result → State → Prop) (aborts : Nat → Prop) :
    wp (Spec.abort code) ensures aborts state ↔ aborts code := by
  simp [wp, Spec.abort]

@[wp_norm] theorem wp_bind (action : Spec State α) (next : α → Spec State β)
    (ensures : β → State → Prop) (aborts : Nat → Prop) (initial : State) :
    wp (Spec.bind action next) ensures aborts initial ↔
      wp action (fun value state => wp (next value) ensures aborts state) aborts initial := by
  constructor
  · rintro ⟨hok, habort⟩
    refine ⟨?_, ?_⟩
    · intro value middle ha
      refine ⟨?_, ?_⟩
      · intro result final hn
        exact hok result final ⟨value, middle, ha, hn⟩
      · intro code hn
        exact habort code (.inr ⟨value, middle, ha, hn⟩)
    · intro code ha
      exact habort code (.inl ha)
  · rintro ⟨haction, habort⟩
    refine ⟨?_, ?_⟩
    · rintro result final ⟨value, middle, ha, hn⟩
      exact (haction value middle ha).1 result final hn
    · intro code h
      cases h with
      | inl ha => exact habort code ha
      | inr hn =>
          obtain ⟨value, middle, ha, hn⟩ := hn
          exact (haction value middle ha).2 code hn

theorem satisfies_of_wp (function : Args → Spec State Result)
    (contract : Contract State Args Result)
    (proof : ∀ args initial, contract.requires args initial →
      wp (function args)
        (fun result final =>
          (¬contract.mayAbort args initial →
            contract.ensures args initial result final) ∧
          contract.frame args initial final)
        (contract.aborts args initial)
        initial) :
    Satisfies function contract := by
  exact proof

/-- The empty finite approximation satisfies every partial-correctness
contract because it has no observable outcome. -/
theorem satisfies_bottom (contract : Contract State Args Result) :
    Satisfies (fun _ => Spec.bottom) contract := by
  intro args initial _
  simp [Spec.bottom]

/-- Fixed-point induction for recursive source functions.  The premise is
exactly the proof rule users expect: assuming recursive calls satisfy the
contract, prove that one authored function body satisfies it. -/
theorem satisfies_fix
    (body : (Args → Spec State Result) → Args → Spec State Result)
    (contract : Contract State Args Result)
    (step : ∀ recursive, Satisfies recursive contract →
      Satisfies (body recursive) contract) :
    Satisfies (Spec.fix body) contract := by
  have approximates : ∀ fuel, Satisfies (Spec.fixApprox body fuel) contract := by
    intro fuel
    induction fuel with
    | zero => exact satisfies_bottom contract
    | succ fuel induction =>
        simpa [Spec.fixApprox] using step (Spec.fixApprox body fuel) induction
  intro args initial permitted
  constructor
  · intro result final execution
    obtain ⟨fuel, execution⟩ := execution
    exact (approximates fuel args initial permitted).1 result final execution
  · intro code execution
    obtain ⟨fuel, execution⟩ := execution
    exact (approximates fuel args initial permitted).2 code execution

/-- Use an already established contract as the weakest-precondition fact for
one concrete call. This avoids manually projecting normal and abort halves.
The callee's postcondition speaks only where its declared aborts are ruled
out; for a callee without reachable aborts the final argument discharges
itself. -/
theorem wp_of_satisfies
    (verified : Satisfies function contract)
    (permitted : contract.requires args initial)
    (noAbort : ¬contract.mayAbort args initial := by simp) :
    wp (function args)
      (fun result final =>
        contract.ensures args initial result final ∧
        contract.frame args initial final)
      (contract.aborts args initial)
      initial :=
  ⟨fun result final execution =>
      let established := (verified args initial permitted).1 result final execution
      ⟨established.1 noAbort, established.2⟩,
    (verified args initial permitted).2⟩

/-- The abort half of an established contract, usable without ruling the
declared aborts out. -/
theorem aborts_of_satisfies
    (verified : Satisfies function contract)
    (permitted : contract.requires args initial) :
    ∀ code, (function args).aborts initial code →
      contract.aborts args initial code :=
  (verified args initial permitted).2

/-- Weaken an established weakest-precondition fact to a coarser
postcondition and abort condition. This adapts a callee's contract to the
caller's local obligation without reopening normal and abort halves. -/
theorem wp_mono {action : Spec State Result}
    {ensures ensures' : Result → State → Prop} {aborts aborts' : Nat → Prop}
    {initial : State}
    (established : wp action ensures aborts initial)
    (weakenEnsures : ∀ result final, ensures result final → ensures' result final)
    (weakenAborts : ∀ code, aborts code → aborts' code) :
    wp action ensures' aborts' initial :=
  ⟨fun result final execution =>
      weakenEnsures result final (established.1 result final execution),
    fun code execution => weakenAborts code (established.2 code execution)⟩

/-- Fixed-point induction with a `wp` step. Most Leaner loop proofs naturally
reason about one call and need not duplicate success and abort forwarding. -/
theorem satisfies_fix_of_wp
    (body : (Args → Spec State Result) → Args → Spec State Result)
    (contract : Contract State Args Result)
    (step : ∀ recursive, Satisfies recursive contract →
      ∀ args initial, contract.requires args initial →
        wp (body recursive args)
          (fun result final =>
            (¬contract.mayAbort args initial →
              contract.ensures args initial result final) ∧
            contract.frame args initial final)
          (contract.aborts args initial)
          initial) :
    Satisfies (Spec.fix body) contract := by
  apply satisfies_fix body contract
  intro recursive recursiveVerified
  exact satisfies_of_wp (body recursive) contract
    (step recursive recursiveVerified)

/-- Deterministic helper WP, used only to embed test computations into the
authoritative relational semantics. -/
def txnWP (action : Txn State Result)
    (ensures : Result → State → Prop) (aborts : Nat → Prop)
    (initial : State) : Prop :=
  match action initial with
  | .ok result final => ensures result final
  | .abort code => aborts code

theorem satisfies_of_txnWP (function : Args → Txn State Result)
    (contract : Contract State Args Result)
    (proof : ∀ args initial, contract.requires args initial →
      txnWP (function args)
        (fun result final =>
          contract.ensures args initial result final ∧
          contract.frame args initial final)
        (contract.aborts args initial)
        initial) :
    Satisfies (fun args => Spec.ofTxn (function args)) contract := by
  intro args initial hpre
  have h := proof args initial hpre
  cases houtcome : function args initial with
  | ok result final =>
      simp [txnWP, houtcome] at h
      refine ⟨?_, ?_⟩
      · intro actual actualFinal heq
        simp [Spec.ofTxn, houtcome] at heq
        obtain ⟨rfl, rfl⟩ := heq
        exact ⟨fun _ => h.1, h.2⟩
      · intro code
        simp [Spec.ofTxn, houtcome]
  | abort code =>
      simp [txnWP, houtcome] at h
      refine ⟨?_, ?_⟩
      · intro result final
        simp [Spec.ofTxn, houtcome]
      · intro actual heq
        simp [Spec.ofTxn, houtcome] at heq
        subst actual
        exact h

end Move.Verify
