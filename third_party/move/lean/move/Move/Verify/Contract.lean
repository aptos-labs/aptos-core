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
        contract.aborts args initial code) ∧
      ¬(function args).undefined initial

/-! A prophecy is eliminated the moment its reconciliation equation appears:
`∀ future, … → value = future → P future` is `… → P value`.  Lean's
`forall_eq` handles the bare shape; these handle the equation under the
hypotheses a weakest-precondition rule puts in front of it. -/

@[simp] theorem forall_imp_eq_left {α : Sort u} {A : Prop} {b : α}
    {P : α → Prop} : (∀ x, A → b = x → P x) ↔ (A → P b) :=
  ⟨fun h a => h b a rfl, fun h x a e => e ▸ h a⟩

@[simp] theorem forall_imp_eq_right {α : Sort u} {A : Prop} {b : α}
    {P : α → Prop} : (∀ x, A → x = b → P x) ↔ (A → P b) :=
  ⟨fun h a => h b a rfl, fun h x a e => e ▸ h a⟩

@[simp] theorem forall_imp_imp_eq_left {α : Sort u} {A B : Prop} {b : α}
    {P : α → Prop} : (∀ x, A → B → b = x → P x) ↔ (A → B → P b) :=
  ⟨fun h a c => h b a c rfl, fun h x a c e => e ▸ h a c⟩

/-- Weakest precondition of the relational verification semantics.  Besides
the normal and abort outcomes it demands well-definedness: a program point
that owes a proof the language never checks at run time — re-establishing a
data invariant after a mutation — must not be reachable with that proof
outstanding. -/
def wp (action : Spec State Result)
    (ensures : Result → State → Prop) (aborts : Nat → Prop)
    (initial : State) : Prop :=
  (∀ result final, action.ok initial result final → ensures result final) ∧
  (∀ code, action.aborts initial code → aborts code) ∧
  ¬action.undefined initial

/-- A total operation — one that owes no proof the language does not check —
has the plain two-obligation weakest precondition.  Every primitive is total;
only re-establishing a data invariant after a mutation is not. -/
theorem wp_total_iff {action : Spec State Result}
    {ensures : Result → State → Prop} {aborts : Nat → Prop} {initial : State}
    (total : ¬action.undefined initial) :
    wp action ensures aborts initial ↔
      (∀ result final, action.ok initial result final → ensures result final) ∧
      (∀ code, action.aborts initial code → aborts code) := by
  simp [wp, total]

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
  · rintro ⟨hok, habort, hdefined⟩
    refine ⟨?_, ?_, ?_⟩
    · intro value middle ha
      refine ⟨?_, ?_, ?_⟩
      · intro result final hn
        exact hok result final ⟨value, middle, ha, hn⟩
      · intro code hn
        exact habort code (.inr ⟨value, middle, ha, hn⟩)
      · intro obligation
        exact hdefined (.inr ⟨value, middle, ha, obligation⟩)
    · intro code ha
      exact habort code (.inl ha)
    · intro obligation
      exact hdefined (.inl obligation)
  · rintro ⟨haction, habort, hdefined⟩
    refine ⟨?_, ?_, ?_⟩
    · rintro result final ⟨value, middle, ha, hn⟩
      exact (haction value middle ha).1 result final hn
    · intro code h
      cases h with
      | inl ha => exact habort code ha
      | inr hn =>
          obtain ⟨value, middle, ha, hn⟩ := hn
          exact (haction value middle ha).2.1 code hn
    · intro obligation
      cases obligation with
      | inl ha => exact hdefined ha
      | inr hn =>
          obtain ⟨value, middle, ha, hn⟩ := hn
          exact (haction value middle ha).2.2 hn

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
  refine ⟨?_, ?_, ?_⟩
  · intro result final execution
    obtain ⟨fuel, execution⟩ := execution
    exact (approximates fuel args initial permitted).1 result final execution
  · intro code execution
    obtain ⟨fuel, execution⟩ := execution
    exact (approximates fuel args initial permitted).2.1 code execution
  · rintro ⟨fuel, obligation⟩
    exact (approximates fuel args initial permitted).2.2 obligation

/-- Fixed-point induction for a heterogeneous mutually recursive SCC.  The
recursive hypothesis supplies every member's contract, so calls across the
family are justified at the same finite approximation. -/
theorem satisfies_fixFamily
    (body : Spec.Family State Index Args Result →
      Spec.Family State Index Args Result)
    (contracts : (index : Index) → Contract State (Args index) (Result index))
    (step : ∀ recursive,
      (∀ index, Satisfies (recursive index) (contracts index)) →
      ∀ index, Satisfies (body recursive index) (contracts index)) :
    ∀ index, Satisfies (Spec.fixFamily body index) (contracts index) := by
  have approximates : ∀ fuel index,
      Satisfies (Spec.fixFamilyApprox body fuel index) (contracts index) := by
    intro fuel
    induction fuel with
    | zero =>
        intro index
        exact satisfies_bottom (contracts index)
    | succ fuel induction =>
        simpa [Spec.fixFamilyApprox] using
          step (Spec.fixFamilyApprox body fuel) induction
  intro index args initial permitted
  refine ⟨?_, ?_, ?_⟩
  · intro result final execution
    obtain ⟨fuel, execution⟩ := execution
    exact (approximates fuel index args initial permitted).1 result final execution
  · intro code execution
    obtain ⟨fuel, execution⟩ := execution
    exact (approximates fuel index args initial permitted).2.1 code execution
  · rintro ⟨fuel, obligation⟩
    exact (approximates fuel index args initial permitted).2.2 obligation

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
    (verified args initial permitted).2.1,
    (verified args initial permitted).2.2⟩

/-- The abort half of an established contract, usable without ruling the
declared aborts out. -/
theorem aborts_of_satisfies
    (verified : Satisfies function contract)
    (permitted : contract.requires args initial) :
    ∀ code, (function args).aborts initial code →
      contract.aborts args initial code :=
  (verified args initial permitted).2.1

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
    fun code execution => weakenAborts code (established.2.1 code execution),
    established.2.2⟩

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

/-- The weakest-precondition form of mutual fixed-point induction. -/
theorem satisfies_fixFamily_of_wp
    (body : Spec.Family State Index Args Result →
      Spec.Family State Index Args Result)
    (contracts : (index : Index) → Contract State (Args index) (Result index))
    (step : ∀ recursive,
      (∀ index, Satisfies (recursive index) (contracts index)) →
      ∀ index args initial, (contracts index).requires args initial →
        wp (body recursive index args)
          (fun result final =>
            (¬(contracts index).mayAbort args initial →
              (contracts index).ensures args initial result final) ∧
            (contracts index).frame args initial final)
          ((contracts index).aborts args initial)
          initial) :
    ∀ index, Satisfies (Spec.fixFamily body index) (contracts index) := by
  apply satisfies_fixFamily body contracts
  intro recursive recursiveVerified index
  exact satisfies_of_wp (body recursive index) (contracts index)
    (step recursive recursiveVerified index)

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
      refine ⟨?_, ?_, ?_⟩
      · intro actual actualFinal heq
        simp [Spec.ofTxn, houtcome] at heq
        obtain ⟨rfl, rfl⟩ := heq
        exact ⟨fun _ => h.1, h.2⟩
      · intro code
        simp [Spec.ofTxn, houtcome]
      · simp [Spec.ofTxn]
  | abort code =>
      simp [txnWP, houtcome] at h
      refine ⟨?_, ?_, ?_⟩
      · intro result final
        simp [Spec.ofTxn, houtcome]
      · intro actual heq
        simp [Spec.ofTxn, houtcome] at heq
        subst actual
        exact h
      · simp [Spec.ofTxn]

end Move.Verify
