-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Semantics.Checked
import Move.Semantics.Reference

/-!
# Direct source contracts

Contracts are ordinary Lean predicates over source transaction semantics.
They deliberately cannot mention reference identities or prophecy variables.
-/

namespace Move.Verify

open Move.Semantics

structure Contract (State Args Result : Type) where
  requires : Args → State → Prop
  ensures : Args → State → Result → State → Prop
  aborts : Args → State → Nat → Prop

/-- A relational source computation satisfies its contract for every
permitted initial state. -/
def Satisfies (function : Args → Spec State Result)
    (contract : Contract State Args Result) : Prop :=
  ∀ args initial,
    contract.requires args initial →
      (∀ result final, (function args).ok initial result final →
        contract.ensures args initial result final) ∧
      (∀ code, (function args).aborts initial code →
        contract.aborts args initial code)

/-- Weakest precondition of the relational verification semantics. -/
def wp (action : Spec State Result)
    (ensures : Result → State → Prop) (aborts : Nat → Prop)
    (initial : State) : Prop :=
  (∀ result final, action.ok initial result final → ensures result final) ∧
  (∀ code, action.aborts initial code → aborts code)

@[simp] theorem wp_pure (value : Result) (state : State)
    (ensures : Result → State → Prop) (aborts : Nat → Prop) :
    wp (Spec.pure value) ensures aborts state ↔ ensures value state := by
  simp [wp, Spec.pure]

@[simp] theorem wp_abort (code : Nat) (state : State)
    (ensures : Result → State → Prop) (aborts : Nat → Prop) :
    wp (Spec.abort code) ensures aborts state ↔ aborts code := by
  simp [wp, Spec.abort]

theorem wp_bind (action : Spec State α) (next : α → Spec State β)
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
        (contract.ensures args initial)
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
        (contract.ensures args initial)
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
        exact h
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
