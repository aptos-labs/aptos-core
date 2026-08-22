-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Semantics.Outcome

/-!
# Relational source semantics

`Spec` is the authoritative semantics for direct source verification. It need
not select an executable result: normal results and aborts are relations. The
deployed execution semantics is the compiled `MoveModel.IR` program.
-/

namespace Move.Semantics

/-- A relational state-and-abort computation. The abort relation mentions only
the initial state because transaction effects are rolled back. -/
structure Spec (State Result : Type) where
  ok : State → Result → State → Prop
  aborts : State → Nat → Prop
  /-- States in which the computation owes a proof that the source language
  never checks at run time — currently only re-establishing a data invariant
  after a mutation.  Every primitive operation is total, so this defaults to
  never, and `wp` demands its negation, which is what makes such an
  obligation positive instead of vacuous. -/
  undefined : State → Prop := fun _ => False

namespace Spec

/-- The computation with no terminating execution.  This is the zeroth
finite approximation used to interpret recursive Move functions. -/
def bottom : Spec σ α where
  ok := fun _ _ _ => False
  aborts := fun _ _ => False

def pure (value : α) : Spec σ α where
  ok := fun initial result final => result = value ∧ final = initial
  aborts := fun _ _ => False

def bind (action : Spec σ α) (next : α → Spec σ β) : Spec σ β where
  ok := fun initial result final =>
    ∃ value middle, action.ok initial value middle ∧ (next value).ok middle result final
  aborts := fun initial code =>
    action.aborts initial code ∨
      ∃ value middle, action.ok initial value middle ∧ (next value).aborts middle code
  undefined := fun initial =>
    action.undefined initial ∨
      ∃ value middle, action.ok initial value middle ∧
        (next value).undefined middle

def abort (code : Nat) : Spec σ α where
  ok := fun _ _ _ => False
  aborts := fun _ actual => actual = code

def get : Spec σ σ where
  ok := fun initial result final => result = initial ∧ final = initial
  aborts := fun _ _ => False

def set (state : σ) : Spec σ Unit where
  ok := fun _ result final => result = () ∧ final = state
  aborts := fun _ _ => False

def modify (f : σ → σ) : Spec σ Unit where
  ok := fun initial result final => result = () ∧ final = f initial
  aborts := fun _ _ => False

/-- Execute a recursive specification with at most `fuel` unfoldings.

This fuel is semantic proof machinery, not a source or runtime bound.  The
public `fix` relation below existentially quantifies it, so it contains every
finite terminating or aborting execution and no arbitrary timeout outcome. -/
def fixApprox (body : (Args → Spec σ Result) → Args → Spec σ Result) :
    Nat → Args → Spec σ Result
  | 0, _ => bottom
  | fuel + 1, args => body (fixApprox body fuel) args

/-- Least finite-unfolding semantics of a recursive Move function.  Divergent
executions produce neither an `ok` result nor an abort, as expected for
partial-correctness verification. -/
def fix (body : (Args → Spec σ Result) → Args → Spec σ Result) :
    Args → Spec σ Result := fun args => {
  ok := fun initial result final =>
    ∃ fuel, (fixApprox body fuel args).ok initial result final
  aborts := fun initial code =>
    ∃ fuel, (fixApprox body fuel args).aborts initial code
  undefined := fun initial =>
    ∃ fuel, (fixApprox body fuel args).undefined initial
}

/-- One heterogeneous family of mutually recursive source functions.  The
index selects both the argument and result type, so an SCC is not forced to
give every member the same signature. -/
abbrev Family (σ : Type) (Index : Type) (Args Result : Index → Type) :=
  (index : Index) → Args index → Spec σ (Result index)

/-- The finite approximants of a mutually recursive family advance every
member in lockstep. -/
def fixFamilyApprox
    (body : Family σ Index Args Result → Family σ Index Args Result) :
    Nat → Family σ Index Args Result
  | 0, _, _ => bottom
  | fuel + 1, index, args => body (fixFamilyApprox body fuel) index args

/-- Least finite-unfolding semantics of a mutually recursive SCC. -/
def fixFamily
    (body : Family σ Index Args Result → Family σ Index Args Result) :
    Family σ Index Args Result := fun index args => {
  ok := fun initial result final =>
    ∃ fuel, (fixFamilyApprox body fuel index args).ok initial result final
  aborts := fun initial code =>
    ∃ fuel, (fixFamilyApprox body fuel index args).aborts initial code
  undefined := fun initial =>
    ∃ fuel, (fixFamilyApprox body fuel index args).undefined initial
}

instance : Monad (Spec σ) where
  pure := pure
  bind := bind

theorem extensionality {left right : Spec σ α}
    (ok : left.ok = right.ok) (aborts : left.aborts = right.aborts)
    (undefined : left.undefined = right.undefined) :
    left = right := by
  cases left
  cases right
  simp_all

/-- A fixed point whose body does not call its recursive argument is just that
body. This is the common semantic shape of a `loop` which exits directly via
`break`, and keeps proofs independent of the finite-approximation encoding. -/
@[simp] theorem fix_const (body : Args → Spec σ Result) :
    fix (fun _ => body) = body := by
  funext args
  apply extensionality
  · funext initial result final
    apply propext
    constructor
    · rintro ⟨fuel, execution⟩
      cases fuel with
      | zero => exact execution.elim
      | succ fuel => exact execution
    · intro execution
      exact ⟨1, execution⟩
  · funext initial code
    apply propext
    constructor
    · rintro ⟨fuel, execution⟩
      cases fuel with
      | zero => exact execution.elim
      | succ fuel => exact execution
    · intro execution
      exact ⟨1, execution⟩
  · funext initial
    apply propext
    constructor
    · rintro ⟨fuel, obligation⟩
      cases fuel with
      | zero => exact obligation.elim
      | succ fuel => exact obligation
    · intro obligation
      exact ⟨1, obligation⟩

@[simp] theorem pure_bind (value : α) (next : α → Spec σ β) :
    bind (pure value) next = next value := by
  apply extensionality
  · funext initial result final
    apply propext
    constructor
    · rintro ⟨actual, middle, ⟨rfl, rfl⟩, execution⟩
      exact execution
    · intro execution
      exact ⟨value, initial, ⟨rfl, rfl⟩, execution⟩
  · funext initial code
    apply propext
    constructor
    · intro execution
      rcases execution with impossible | ⟨actual, middle, ⟨rfl, rfl⟩, execution⟩
      · exact impossible.elim
      · exact execution
    · intro execution
      exact .inr ⟨value, initial, ⟨rfl, rfl⟩, execution⟩
  · funext initial
    apply propext
    constructor
    · intro obligation
      rcases obligation with impossible | ⟨actual, middle, ⟨rfl, rfl⟩, obligation⟩
      · exact impossible.elim
      · exact obligation
    · intro obligation
      exact .inr ⟨value, initial, ⟨rfl, rfl⟩, obligation⟩

/-- Binding after an abort stays that abort. -/
@[simp] theorem abort_bind (code : Nat) (next : α → Spec σ β) :
    (Spec.abort code : Spec σ α).bind next = Spec.abort code := by
  apply Spec.extensionality
  · funext initial value final
    simp [Spec.bind, Spec.abort]
  · funext initial actual
    simp [Spec.bind, Spec.abort]
  · funext initial
    simp [Spec.bind, Spec.abort]

@[simp] theorem bind_pure (action : Spec σ α) :
    bind action pure = action := by
  apply extensionality
  · funext initial result final
    apply propext
    constructor
    · rintro ⟨actual, middle, execution, rfl, rfl⟩
      exact execution
    · intro execution
      exact ⟨result, final, execution, rfl, rfl⟩
  · funext initial code
    simp [bind, pure]
  · funext initial
    simp [bind, pure]

/-- A computation which owes no proof: defined in every state.  Every
primitive is total, so totality is established structurally, without ever
expanding `undefined` into a copy of the `ok` relation. -/
def Total (action : Spec σ α) : Prop := ∀ state, ¬action.undefined state

/-- Well-definedness of a sequence, pointwise and reachability-aware: the
continuation only owes a proof in states the prefix can actually reach.  The
pointwise shape matches proof goals directly, so establishing it never has to
unfold a whole body. -/
theorem bind_defined {action : Spec σ α} {next : α → Spec σ β} {state : σ}
    (head : ¬action.undefined state)
    (tail : ∀ value middle, action.ok state value middle →
      ¬(next value).undefined middle) :
    ¬(action.bind next).undefined state := by
  rintro (obligation | ⟨value, middle, reachable, obligation⟩)
  · exact head obligation
  · exact tail value middle reachable obligation

theorem Total.bind {action : Spec σ α} {next : α → Spec σ β}
    (total : Total action) (continuation : ∀ value, Total (next value)) :
    Total (action.bind next) := by
  rintro state (obligation | ⟨value, middle, -, obligation⟩)
  · exact total state obligation
  · exact continuation value middle obligation

@[simp] theorem total_pure (value : α) : Total (pure value : Spec σ α) :=
  fun _ obligation => obligation.elim

@[simp] theorem total_abort (code : Nat) : Total (abort code : Spec σ α) :=
  fun _ obligation => obligation.elim

@[simp] theorem total_bottom : Total (bottom : Spec σ α) :=
  fun _ obligation => obligation.elim

@[simp] theorem total_get : Total (get : Spec σ σ) :=
  fun _ obligation => obligation.elim

@[simp] theorem total_set (state : σ) : Total (set state : Spec σ Unit) :=
  fun _ obligation => obligation.elim

@[simp] theorem total_modify (f : σ → σ) : Total (modify f : Spec σ Unit) :=
  fun _ obligation => obligation.elim

/-- Well-definedness of a recursive function: every finite unfolding is
well-defined when one step of the body is, assuming it of the recursive
calls. -/
theorem fixApprox_defined {body : (Args → Spec σ Result) → Args → Spec σ Result}
    (step : ∀ recursive, (∀ a s, ¬(recursive a).undefined s) →
      ∀ a s, ¬(body recursive a).undefined s) :
    ∀ fuel args state, ¬(fixApprox body fuel args).undefined state
  | 0, _, _ => fun obligation => obligation
  | fuel + 1, args, state =>
      step (fixApprox body fuel) (fixApprox_defined step fuel) args state

theorem fix_defined {body : (Args → Spec σ Result) → Args → Spec σ Result}
    {args : Args} {state : σ}
    (step : ∀ recursive, (∀ a s, ¬(recursive a).undefined s) →
      ∀ a s, ¬(body recursive a).undefined s) :
    ¬(fix body args).undefined state :=
  fun ⟨fuel, obligation⟩ => fixApprox_defined step fuel args state obligation

/-- Create a value which certifies a data invariant.  Creating one is the
only place an invariant is owed: nothing checks it at run time, so the
operation is undefined exactly where the invariant fails, and `wp` turns that
into a positive proof obligation at the creation site. -/
def certified {Invariant : Prop} (build : Invariant → α) : Spec σ α where
  ok := fun initial result final =>
    ∃ holds : Invariant, result = build holds ∧ final = initial
  aborts := fun _ _ => False
  undefined := fun _ => ¬Invariant

/-- The only well-definedness obligation a source program owes: the data
invariant of a value it creates. -/
theorem certified_defined_iff {Invariant : Prop} (build : Invariant → α)
    (state : σ) :
    ¬(certified build : Spec σ α).undefined state ↔ Invariant := by
  simp only [certified, Classical.not_not]

/-- Assert a predicate over the current global state and hand it to the
continuation.  This certifies the state at the point a global invariant must
hold: `undefined` where the predicate fails makes it a positive obligation
(checked here, at the update, not at the function end), and the `ok` relation
carries the predicate forward so downstream reads may assume it.  It is the
state-level analogue of `certified` for values. -/
def certifyState (invariant : State → Prop) : Spec State Unit where
  ok := fun initial result final => invariant initial ∧ result = () ∧ final = initial
  aborts := fun _ _ => False
  undefined := fun initial => ¬ invariant initial

@[simp] theorem certifyState_ok (invariant : State → Prop) :
    (certifyState invariant).ok initial result final ↔
      invariant initial ∧ result = () ∧ final = initial := Iff.rfl

@[simp] theorem certifyState_aborts (invariant : State → Prop) :
    ¬(certifyState invariant).aborts initial code := fun h => h

@[simp] theorem certifyState_undefined (invariant : State → Prop) :
    (certifyState invariant).undefined initial ↔ ¬ invariant initial := Iff.rfl

/-- Assert a relation between the state before and after an operation.  This
certifies an *update* global invariant, which — unlike a regular invariant —
constrains a state transition rather than a single state: it is asserted at
each update (`rel initial final`) but never assumed on entry.  `op` is the
state-changing operation the invariant is injected around. -/
def certifyUpdate (relation : State → State → Prop) (op : Spec State α) :
    Spec State α where
  ok := fun initial result final => op.ok initial result final ∧ relation initial final
  aborts := op.aborts
  undefined := fun initial =>
    op.undefined initial ∨
      ∃ result final, op.ok initial result final ∧ ¬ relation initial final

@[simp] theorem certifyUpdate_ok (relation : State → State → Prop)
    (op : Spec State α) :
    (certifyUpdate relation op).ok initial result final ↔
      op.ok initial result final ∧ relation initial final := Iff.rfl

@[simp] theorem certifyUpdate_aborts (relation : State → State → Prop)
    (op : Spec State α) :
    (certifyUpdate relation op).aborts initial code ↔ op.aborts initial code :=
  Iff.rfl

@[simp] theorem certifyUpdate_undefined (relation : State → State → Prop)
    (op : Spec State α) :
    (certifyUpdate relation op).undefined initial ↔
      op.undefined initial ∨
        ∃ result final, op.ok initial result final ∧ ¬ relation initial final :=
  Iff.rfl

/-- Embed a deterministic helper computation into the authoritative
relational semantics. This is useful for tests, not required for deployment. -/
def ofTxn (action : Txn σ α) : Spec σ α where
  ok := fun initial result final => action initial = .ok result final
  aborts := fun initial code => action initial = .abort code
  undefined := fun _ => False

@[simp] theorem total_ofTxn (action : Txn σ α) : Total (ofTxn action) :=
  fun _ obligation => obligation.elim

@[simp] theorem bind_ok (action : Spec σ α) (next : α → Spec σ β) :
    (action.bind next).ok initial result final ↔
      ∃ value middle, action.ok initial value middle ∧
        (next value).ok middle result final := Iff.rfl

@[simp] theorem bind_aborts (action : Spec σ α) (next : α → Spec σ β) :
    (action.bind next).aborts initial code ↔
      action.aborts initial code ∨
        ∃ value middle, action.ok initial value middle ∧
          (next value).aborts middle code := Iff.rfl

@[simp] theorem certified_ok {Invariant : Prop} (build : Invariant → α) :
    (certified build : Spec σ α).ok initial result final ↔
      ∃ holds : Invariant, result = build holds ∧ final = initial := Iff.rfl

@[simp] theorem certified_aborts {Invariant : Prop} (build : Invariant → α) :
    ¬(certified build : Spec σ α).aborts initial code := fun h => h

/-! Projections distribute over source conditionals, so symbolic execution
never has to unfold a branch it has not yet decided. -/

@[simp] theorem ok_ite (c : Prop) [Decidable c] (a b : Spec σ α) :
    (if c then a else b).ok initial result final ↔
      if c then a.ok initial result final else b.ok initial result final := by
  split <;> rfl

@[simp] theorem aborts_ite (c : Prop) [Decidable c] (a b : Spec σ α) :
    (if c then a else b).aborts initial code ↔
      if c then a.aborts initial code else b.aborts initial code := by
  split <;> rfl

@[simp] theorem undefined_ite (c : Prop) [Decidable c] (a b : Spec σ α) :
    (if c then a else b).undefined initial ↔
      if c then a.undefined initial else b.undefined initial := by
  split <;> rfl

@[simp] theorem ok_dite (c : Prop) [Decidable c] (a : c → Spec σ α)
    (b : ¬c → Spec σ α) :
    (if h : c then a h else b h).ok initial result final ↔
      if h : c then (a h).ok initial result final
      else (b h).ok initial result final := by
  split <;> rfl

@[simp] theorem aborts_dite (c : Prop) [Decidable c] (a : c → Spec σ α)
    (b : ¬c → Spec σ α) :
    (if h : c then a h else b h).aborts initial code ↔
      if h : c then (a h).aborts initial code else (b h).aborts initial code := by
  split <;> rfl

@[simp] theorem undefined_dite (c : Prop) [Decidable c] (a : c → Spec σ α)
    (b : ¬c → Spec σ α) :
    (if h : c then a h else b h).undefined initial ↔
      if h : c then (a h).undefined initial else (b h).undefined initial := by
  split <;> rfl

@[simp] theorem pure_ok : (pure value : Spec σ α).ok initial result final ↔
    result = value ∧ final = initial := Iff.rfl

@[simp] theorem pure_aborts : ¬(pure value : Spec σ α).aborts initial code := by
  simp [pure]

@[simp] theorem abort_ok : ¬(abort code : Spec σ α).ok initial result final := by
  simp [abort]

@[simp] theorem abort_aborts : (abort code : Spec σ α).aborts initial actual ↔
    actual = code := Iff.rfl

@[simp] theorem pure_undefined : ¬(pure value : Spec σ α).undefined initial := by
  simp [pure]

@[simp] theorem abort_undefined : ¬(abort code : Spec σ α).undefined initial := by
  simp [abort]

@[simp] theorem bottom_undefined : ¬(bottom : Spec σ α).undefined initial := by
  simp [bottom]

/-- Deliberately not `@[simp]`: expanding this copies the whole prefix's `ok`
relation into the goal.  Totality is established structurally instead, with
`Total.bind`. -/
theorem bind_undefined (action : Spec σ α) (next : α → Spec σ β) :
    (action.bind next).undefined initial ↔
      action.undefined initial ∨
        ∃ value middle, action.ok initial value middle ∧
          (next value).undefined middle := Iff.rfl

@[simp] theorem bottom_ok : ¬(bottom : Spec σ α).ok initial result final := by
  simp [bottom]

@[simp] theorem bottom_aborts : ¬(bottom : Spec σ α).aborts initial code := by
  simp [bottom]

@[simp] theorem fixApprox_zero
    (body : (Args → Spec σ Result) → Args → Spec σ Result) (args : Args) :
    fixApprox body 0 args = (bottom : Spec σ Result) := rfl

@[simp] theorem fixApprox_succ
    (body : (Args → Spec σ Result) → Args → Spec σ Result)
    (fuel : Nat) (args : Args) :
    fixApprox body (fuel + 1) args = body (fixApprox body fuel) args := rfl

@[simp] theorem fix_ok
    (body : (Args → Spec σ Result) → Args → Spec σ Result)
    (args : Args) :
    (fix body args).ok initial result final ↔
      ∃ fuel, (fixApprox body fuel args).ok initial result final := Iff.rfl

@[simp] theorem fix_aborts
    (body : (Args → Spec σ Result) → Args → Spec σ Result)
    (args : Args) :
    (fix body args).aborts initial code ↔
      ∃ fuel, (fixApprox body fuel args).aborts initial code := Iff.rfl

end Spec

end Move.Semantics
