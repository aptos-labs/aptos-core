-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Spec

/-!
# Direct contracts over the relational semantics

Contracts are ordinary Lean predicates over the relational verification
semantics.  They deliberately cannot mention reference identities or prophecy
variables.  This is the frozen reference stack's `Move.Verify.Contract`
generalized over the failure vocabulary `ε`.
-/

namespace LeanerIR.Proofs

/-- A contract.  Failure behavior has three components, read off the declared
failure conditions: which failure outcomes the contract permits (completeness
— a failure must match a declared clause), where the function must fail
(sufficiency — a declared condition forces a failure), and where a declared
failure excuses the postcondition.  They coincide for declared conditions,
but not when failure behavior is left uninterpreted: then every failure is
permitted, nothing must fail, and nothing is excused. -/
structure Contract (σ ε Args Result : Type) where
  /-- States in which callers may invoke the function. -/
  requires : Args → σ → Prop
  /-- Relation established by a successful execution. -/
  ensures : Args → σ → Result → σ → Prop
  /-- Failure outcomes the contract permits.  Uninterpreted failure behavior
  permits every outcome. -/
  aborts : Args → σ → ε → Prop
  /-- States where a declared failure excuses the postcondition.  This is the
  disjunction of the declared failure conditions, and `False` when no failure
  condition is declared — uninterpreted failures excuse nothing, so
  successful executions must still establish `ensures`. -/
  mayAbort : Args → σ → Prop
  /-- States in which the function must fail: the disjunction of the declared
  failure conditions (a declared condition is sufficient for a failure), and
  `False` when none is declared. -/
  mustAbort : Args → σ → Prop := fun _ _ => False
  /-- What a successful execution leaves unchanged.  A specification changes
  only the global memory its `modifies` clause lists, so this defaults to
  changing nothing at all and is never written by hand. -/
  frame : Args → σ → σ → Prop := fun _ initial final => final = initial

/-- A relational computation satisfies its contract for every permitted
initial state.  Three things hold of every successful execution: the frame,
unconditionally; the postcondition wherever the declared failures are ruled
out; and that no declared failure condition held (a successful execution
refutes sufficiency).  Every failure outcome is one the contract permits.
These readings are the semantics — not anything written in the clauses. -/
def Satisfies (function : Args → Spec σ ε Result)
    (contract : Contract σ ε Args Result) : Prop :=
  ∀ args initial,
    contract.requires args initial →
      (∀ result final, (function args).ok initial result final →
        (¬contract.mayAbort args initial →
          contract.ensures args initial result final) ∧
        contract.frame args initial final ∧
        ¬contract.mustAbort args initial) ∧
      (∀ error, (function args).aborts initial error →
        contract.aborts args initial error) ∧
      ¬(function args).undefined initial

/-- Contract satisfaction depends only on the three relations of `Spec`.
This is the transport used by a generated denotation's exact agreement
theorem; no one-way simulation is sufficient for exact abort clauses. -/
theorem satisfies_congr {left right : Args → Spec σ ε Result}
    (equivalent : ∀ args, Spec.Equiv (left args) (right args))
    (contract : Contract σ ε Args Result) :
    Satisfies left contract ↔ Satisfies right contract := by
  constructor
  · intro satisfies args initial permitted
    obtain ⟨normal, failing, defined⟩ := satisfies args initial permitted
    refine ⟨?_, ?_, ?_⟩
    · intro result final execution
      exact normal result final ((equivalent args).ok _ _ _ |>.mpr execution)
    · intro error execution
      exact failing error ((equivalent args).aborts _ _ |>.mpr execution)
    · exact fun obligation => defined ((equivalent args).undefined _ |>.mpr obligation)
  · intro satisfies args initial permitted
    obtain ⟨normal, failing, defined⟩ := satisfies args initial permitted
    refine ⟨?_, ?_, ?_⟩
    · intro result final execution
      exact normal result final ((equivalent args).ok _ _ _ |>.mp execution)
    · intro error execution
      exact failing error ((equivalent args).aborts _ _ |>.mp execution)
    · exact fun obligation => defined ((equivalent args).undefined _ |>.mp obligation)

/-- The computation a contract summarizes: every outcome the contract permits.
It is the semantics of a function whose body is not consulted — a native, or
an opaque summary — so callers reason through the contract alone: they must
establish the precondition (the summary is undefined outside it), and receive
the postcondition where no declared failure excuses it, the frame, and the
declared failures. -/
def Contract.summary (contract : Contract σ ε Args Result) (args : Args) :
    Spec σ ε Result where
  ok := fun initial result final =>
    contract.requires args initial ∧
    (¬contract.mayAbort args initial → contract.ensures args initial result final) ∧
    contract.frame args initial final ∧ ¬contract.mustAbort args initial
  aborts := fun initial error =>
    contract.requires args initial ∧ contract.aborts args initial error
  undefined := fun initial => ¬contract.requires args initial

/-- A summary satisfies the contract it summarizes. -/
theorem satisfies_summary (contract : Contract σ ε Args Result) :
    Satisfies (fun args => contract.summary args) contract := by
  intro args initial permitted
  refine ⟨fun result final h => ⟨h.2.1, h.2.2.1, h.2.2.2⟩, fun error h => h.2, ?_⟩
  simp [Contract.summary, permitted]

/-- Range-carrying wrapper on one generated proof obligation: the
proposition of an authored specification clause, tagged with the half-open
byte range of that clause in its source file.  Contract generation wraps
each clause it emits; the verification script keeps the wrapper folded
through symbolic execution and unfolds it only inside the closing attempt,
so an obligation that survives the automatic finish still names the clause
it came from for error reporting. -/
def Obligation (startByte endByte : Nat) (p : Prop) : Prop := p

theorem Obligation.intro {startByte endByte : Nat} {p : Prop} (h : p) :
    Obligation startByte endByte p := h

/-- The only way symbolic execution unfolds an obligation marker: the
closing attempt rewrites with this once the goal is down to arithmetic. -/
theorem Obligation_iff {startByte endByte : Nat} {p : Prop} :
    Obligation startByte endByte p ↔ p := .rfl

/- Sealed so ordinary reduction cannot strip the range off an obligation;
`Obligation_iff` above is the deliberate exit. -/
attribute [irreducible] Obligation

/-! A prophecy is eliminated the moment its reconciliation equation appears:
`∀ future, … → value = future → P future` is `… → P value`.  Lean's
`forall_eq` handles the bare shape; these handle the equation under the
hypotheses a weakest-precondition rule puts in front of it. -/

@[simp] theorem forall_imp_eq_left {α : Sort u} {A : Prop} {b : α}
    {P : α → Prop} : (∀ x, A → b = x → P x) ↔ (A → P b) :=
  ⟨fun h a => h b a rfl, fun h _ a e => e ▸ h a⟩

@[simp] theorem forall_imp_eq_right {α : Sort u} {A : Prop} {b : α}
    {P : α → Prop} : (∀ x, A → x = b → P x) ↔ (A → P b) :=
  ⟨fun h a => h b a rfl, fun h _ a e => e ▸ h a⟩

@[simp] theorem forall_imp_imp_eq_left {α : Sort u} {A B : Prop} {b : α}
    {P : α → Prop} : (∀ x, A → B → b = x → P x) ↔ (A → B → P b) :=
  ⟨fun h a c => h b a c rfl, fun h _ a c e => e ▸ h a c⟩

/-- Weakest precondition of the relational verification semantics.  Besides
the normal and failure outcomes it demands well-definedness: a program point
that owes a proof the language never checks at run time — re-establishing a
data invariant after a mutation — must not be reachable with that proof
outstanding. -/
def wp (action : Spec σ ε Result)
    (ensures : Result → σ → Prop) (aborts : ε → Prop)
    (initial : σ) : Prop :=
  (∀ result final, action.ok initial result final → ensures result final) ∧
  (∀ error, action.aborts initial error → aborts error) ∧
  ¬action.undefined initial

/-- A total operation — one that owes no proof the language does not check —
has the plain two-obligation weakest precondition.  Every primitive is total;
only re-establishing a data invariant after a mutation is not. -/
theorem wp_total_iff {action : Spec σ ε Result}
    {ensures : Result → σ → Prop} {aborts : ε → Prop} {initial : σ}
    (total : ¬action.undefined initial) :
    wp action ensures aborts initial ↔
      (∀ result final, action.ok initial result final → ensures result final) ∧
      (∀ error, action.aborts initial error → aborts error) := by
  simp [wp, total]

@[simp, lir_wp_norm] theorem wp_pure (value : Result) (state : σ)
    (ensures : Result → σ → Prop) (aborts : ε → Prop) :
    wp (Spec.pure value) ensures aborts state ↔ ensures value state := by
  simp [wp, Spec.pure]

@[simp, lir_wp_norm] theorem wp_abort (error : ε) (state : σ)
    (ensures : Result → σ → Prop) (aborts : ε → Prop) :
    wp (Spec.abort error : Spec σ ε Result) ensures aborts state ↔ aborts error := by
  simp [wp, Spec.abort]

/-- The weakest precondition through a summary: the precondition, and the
continuation under what the contract guarantees. -/
@[simp, lir_wp_norm] theorem wp_summary (contract : Contract σ ε Args Result)
    (args : Args) (ensures : Result → σ → Prop) (aborts : ε → Prop)
    (initial : σ) :
    wp (contract.summary args) ensures aborts initial ↔
      contract.requires args initial ∧
      (∀ result final,
        (¬contract.mayAbort args initial → contract.ensures args initial result final) →
        contract.frame args initial final → ¬contract.mustAbort args initial →
        ensures result final) ∧
      (∀ error, contract.aborts args initial error → aborts error) := by
  constructor
  · intro h
    obtain ⟨hok, habort, hdefined⟩ := h
    have permitted : contract.requires args initial :=
      Classical.byContradiction fun h => hdefined h
    exact ⟨permitted,
      fun result final h1 h2 h3 => hok result final ⟨permitted, h1, h2, h3⟩,
      fun error h => habort error ⟨permitted, h⟩⟩
  · intro h
    obtain ⟨permitted, hok, habort⟩ := h
    refine ⟨fun result final h => hok result final h.2.1 h.2.2.1 h.2.2.2,
      fun error h => habort error h.2, ?_⟩
    simp [Contract.summary, permitted]

@[lir_wp_norm] theorem wp_bind (action : Spec σ ε α) (next : α → Spec σ ε β)
    (ensures : β → σ → Prop) (aborts : ε → Prop) (initial : σ) :
    wp (Spec.bind action next) ensures aborts initial ↔
      wp action (fun value state => wp (next value) ensures aborts state)
        aborts initial := by
  constructor
  · rintro ⟨hok, habort, hdefined⟩
    refine ⟨?_, ?_, ?_⟩
    · intro value middle ha
      refine ⟨?_, ?_, ?_⟩
      · intro result final hn
        exact hok result final ⟨value, middle, ha, hn⟩
      · intro error hn
        exact habort error (.inr ⟨value, middle, ha, hn⟩)
      · intro obligation
        exact hdefined (.inr ⟨value, middle, ha, obligation⟩)
    · intro error ha
      exact habort error (.inl ha)
    · intro obligation
      exact hdefined (.inl obligation)
  · rintro ⟨haction, habort, hdefined⟩
    refine ⟨?_, ?_, ?_⟩
    · rintro result final ⟨value, middle, ha, hn⟩
      exact (haction value middle ha).1 result final hn
    · intro error h
      cases h with
      | inl ha => exact habort error ha
      | inr hn =>
          obtain ⟨value, middle, ha, hn⟩ := hn
          exact (haction value middle ha).2.1 error hn
    · intro obligation
      cases obligation with
      | inl ha => exact hdefined ha
      | inr hn =>
          obtain ⟨value, middle, ha, hn⟩ := hn
          exact (haction value middle ha).2.2 hn

theorem satisfies_of_wp (function : Args → Spec σ ε Result)
    (contract : Contract σ ε Args Result)
    (proof : ∀ args initial, contract.requires args initial →
      wp (function args)
        (fun result final =>
          (¬contract.mayAbort args initial →
            contract.ensures args initial result final) ∧
          contract.frame args initial final ∧
          ¬contract.mustAbort args initial)
        (contract.aborts args initial)
        initial) :
    Satisfies function contract := by
  exact proof

/-- The empty finite approximation satisfies every partial-correctness
contract because it has no observable outcome. -/
theorem satisfies_bottom (contract : Contract σ ε Args Result) :
    Satisfies (fun _ => Spec.bottom) contract := by
  intro args initial _
  simp [Spec.bottom]

/-- Fixed-point induction for recursive functions.  The premise is exactly
the proof rule users expect: assuming recursive calls satisfy the contract,
prove that one authored function body satisfies it. -/
theorem satisfies_fix
    (body : (Args → Spec σ ε Result) → Args → Spec σ ε Result)
    (contract : Contract σ ε Args Result)
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
  · intro error execution
    obtain ⟨fuel, execution⟩ := execution
    exact (approximates fuel args initial permitted).2.1 error execution
  · rintro ⟨fuel, obligation⟩
    exact (approximates fuel args initial permitted).2.2 obligation

/-- Fixed-point induction for a heterogeneous mutually recursive SCC.  The
recursive hypothesis supplies every member's contract, so calls across the
family are justified at the same finite approximation. -/
theorem satisfies_fixFamily
    (body : Spec.Family σ ε Index Args Result →
      Spec.Family σ ε Index Args Result)
    (contracts : (index : Index) → Contract σ ε (Args index) (Result index))
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
  · intro error execution
    obtain ⟨fuel, execution⟩ := execution
    exact (approximates fuel index args initial permitted).2.1 error execution
  · rintro ⟨fuel, obligation⟩
    exact (approximates fuel index args initial permitted).2.2 obligation

/-- Use an already established contract as the weakest-precondition fact for
one concrete call. This avoids manually projecting normal and failure halves.
The callee's postcondition speaks only where its declared failures are ruled
out; for a callee without reachable failures the final argument discharges
itself. -/
theorem wp_of_satisfies
    {function : Args → Spec σ ε Result} {contract : Contract σ ε Args Result}
    {args : Args} {initial : σ}
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
      ⟨established.1 noAbort, established.2.1⟩,
    (verified args initial permitted).2.1,
    (verified args initial permitted).2.2⟩

/-- The failure half of an established contract, usable without ruling the
declared failures out. -/
theorem aborts_of_satisfies
    {function : Args → Spec σ ε Result} {contract : Contract σ ε Args Result}
    {args : Args} {initial : σ}
    (verified : Satisfies function contract)
    (permitted : contract.requires args initial) :
    ∀ error, (function args).aborts initial error →
      contract.aborts args initial error :=
  (verified args initial permitted).2.1

/-- Weaken an established weakest-precondition fact to a coarser
postcondition and failure condition. This adapts a callee's contract to the
caller's local obligation without reopening normal and failure halves. -/
theorem wp_mono {action : Spec σ ε Result}
    {ensures ensures' : Result → σ → Prop} {aborts aborts' : ε → Prop}
    {initial : σ}
    (established : wp action ensures aborts initial)
    (weakenEnsures : ∀ result final, ensures result final → ensures' result final)
    (weakenAborts : ∀ error, aborts error → aborts' error) :
    wp action ensures' aborts' initial :=
  ⟨fun result final execution =>
      weakenEnsures result final (established.1 result final execution),
    fun error execution => weakenAborts error (established.2.1 error execution),
    established.2.2⟩

/-- Fixed-point induction with a `wp` step. Most loop proofs naturally reason
about one call and need not duplicate success and failure forwarding. -/
theorem satisfies_fix_of_wp
    (body : (Args → Spec σ ε Result) → Args → Spec σ ε Result)
    (contract : Contract σ ε Args Result)
    (step : ∀ recursive, Satisfies recursive contract →
      ∀ args initial, contract.requires args initial →
        wp (body recursive args)
          (fun result final =>
            (¬contract.mayAbort args initial →
              contract.ensures args initial result final) ∧
            contract.frame args initial final ∧
            ¬contract.mustAbort args initial)
          (contract.aborts args initial)
          initial) :
    Satisfies (Spec.fix body) contract := by
  apply satisfies_fix body contract
  intro recursive recursiveVerified
  exact satisfies_of_wp (body recursive) contract
    (step recursive recursiveVerified)

/-- Loop verification from a stated invariant (`Spec.withInvariant`): the
invariant holds on entry, and every iteration that starts under it is correct
when its recursive occurrence — the next iteration — is assumed correct under
the invariant.  Partial correctness, like `satisfies_fix`. -/
theorem wp_withInvariant_fix {Args Result : Type}
    {invariant : Args → σ → Prop}
    {body : (Args → Spec σ ε Result) → Args → Spec σ ε Result}
    {init : Args} {ensures : Result → σ → Prop} {aborts : ε → Prop}
    {initial : σ}
    (entry : invariant init initial)
    (step : ∀ recursive,
      (∀ args store, invariant args store →
        wp (recursive args) ensures aborts store) →
      ∀ args store, invariant args store →
        wp (body recursive args) ensures aborts store) :
    wp (Spec.withInvariant body init invariant) ensures aborts initial := by
  let contract : Contract σ ε Args Result := {
    requires := invariant
    ensures := fun _ _ result final => ensures result final
    aborts := fun _ _ error => aborts error
    mayAbort := fun _ _ => False
    mustAbort := fun _ _ => False
    frame := fun _ _ _ => True }
  have verified : Satisfies (Spec.fix body) contract := by
    apply satisfies_fix_of_wp body contract
    intro recursive recursiveVerified args store permitted
    have hypothesis : ∀ args store, invariant args store →
        wp (recursive args) ensures aborts store := fun args store holds =>
      wp_mono (wp_of_satisfies recursiveVerified holds (noAbort := fun h => h))
        (fun _ _ h => h.1) (fun _ h => h)
    exact wp_mono (step recursive hypothesis args store permitted)
      (fun _ _ h => ⟨fun _ => h, trivial, fun h' => h'⟩) (fun _ h => h)
  show wp (Spec.fix body init) ensures aborts initial
  exact wp_mono (wp_of_satisfies verified entry (noAbort := fun h => h))
    (fun _ _ h => h.1) (fun _ h => h)

/-- Loop verification for a loop that leaves the store as it found it: the
invariant speaks about the loop state, and the store at every iteration is
the store at entry.  The automatic prover's rule (source invariants range
over locals and referents). -/
theorem wp_withInvariant_fix_frame {Args Result : Type}
    {invariant : Args → σ → Prop}
    {body : (Args → Spec σ ε Result) → Args → Spec σ ε Result}
    {init : Args} {ensures : Result → σ → Prop} {aborts : ε → Prop}
    {initial : σ}
    (entry : invariant init initial)
    (step : ∀ recursive,
      (∀ args, invariant args initial → wp (recursive args) ensures aborts initial) →
      ∀ args, invariant args initial →
        wp (body recursive args) ensures aborts initial) :
    wp (Spec.withInvariant body init invariant) ensures aborts initial := by
  show wp (Spec.fix body init) ensures aborts initial
  refine wp_withInvariant_fix
    (invariant := fun args store => invariant args store ∧ store = initial)
    ⟨entry, rfl⟩ ?_
  rintro recursive hypothesis args _ ⟨holds, rfl⟩
  exact step recursive (fun args holds => hypothesis args _ ⟨holds, rfl⟩) args holds

/-- The weakest-precondition form of mutual fixed-point induction. -/
theorem satisfies_fixFamily_of_wp
    (body : Spec.Family σ ε Index Args Result →
      Spec.Family σ ε Index Args Result)
    (contracts : (index : Index) → Contract σ ε (Args index) (Result index))
    (step : ∀ recursive,
      (∀ index, Satisfies (recursive index) (contracts index)) →
      ∀ index args initial, (contracts index).requires args initial →
        wp (body recursive index args)
          (fun result final =>
            (¬(contracts index).mayAbort args initial →
              (contracts index).ensures args initial result final) ∧
            (contracts index).frame args initial final ∧
            ¬(contracts index).mustAbort args initial)
          ((contracts index).aborts args initial)
          initial) :
    ∀ index, Satisfies (Spec.fixFamily body index) (contracts index) := by
  apply satisfies_fixFamily body contracts
  intro recursive recursiveVerified index
  exact satisfies_of_wp (body recursive index) (contracts index)
    (step recursive recursiveVerified index)

end LeanerIR.Proofs
