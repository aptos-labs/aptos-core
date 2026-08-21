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
}

instance : Monad (Spec σ) where
  pure := pure
  bind := bind

theorem extensionality {left right : Spec σ α}
    (ok : left.ok = right.ok) (aborts : left.aborts = right.aborts) :
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

/-- Embed a deterministic helper computation into the authoritative
relational semantics. This is useful for tests, not required for deployment. -/
def ofTxn (action : Txn σ α) : Spec σ α where
  ok := fun initial result final => action initial = .ok result final
  aborts := fun initial code => action initial = .abort code

@[simp] theorem pure_ok : (pure value : Spec σ α).ok initial result final ↔
    result = value ∧ final = initial := Iff.rfl

@[simp] theorem pure_aborts : ¬(pure value : Spec σ α).aborts initial code := by
  simp [pure]

@[simp] theorem abort_ok : ¬(abort code : Spec σ α).ok initial result final := by
  simp [abort]

@[simp] theorem abort_aborts : (abort code : Spec σ α).aborts initial actual ↔
    actual = code := Iff.rfl

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
