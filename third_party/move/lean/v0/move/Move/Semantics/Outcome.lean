-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Basic

/-!
# Source transaction outcomes

Executable semantics for the effects visible at a Move transaction boundary.
The state parameter contains on-chain state only; mutable-reference state is
represented separately by ownership passing.
-/

namespace Move.Semantics

/-- A source computation either returns normally with tentative state or
aborts. Aborting does not expose tentative state because the transaction
boundary rolls it back. -/
inductive Outcome (State Result : Type) where
  | ok (value : Result) (state : State)
  | abort (code : Nat)
  deriving Repr, DecidableEq

namespace Outcome

def map (f : α → β) : Outcome σ α → Outcome σ β
  | .ok value state => .ok (f value) state
  | .abort code => .abort code

def bind (outcome : Outcome σ α) (next : α → σ → Outcome σ β) : Outcome σ β :=
  match outcome with
  | .ok value state => next value state
  | .abort code => .abort code

/-- The state committed at the transaction boundary. Aborts restore the
initial state. -/
def committedState (initial : σ) : Outcome σ α → σ
  | .ok _ final => final
  | .abort _ => initial

@[simp] theorem committedState_ok (initial final : σ) (value : α) :
    committedState initial (.ok value final) = final := rfl

@[simp] theorem committedState_abort (initial : σ) (code : Nat) :
    committedState initial (Outcome.abort (State := σ) (Result := α) code) = initial := rfl

end Outcome

/-- State-and-abort computations used by the faithful source semantics. The
state is generic so the semantic core does not prescribe one global-resource
encoding. -/
abbrev Txn (State Result : Type) := State → Outcome State Result

namespace Txn

def pure (value : α) : Txn σ α := fun state => .ok value state

def bind (action : Txn σ α) (next : α → Txn σ β) : Txn σ β := fun state =>
  (action state).bind next

def abort (code : Nat) : Txn σ α := fun _ => .abort code

def get : Txn σ σ := fun state => .ok state state

def set (state : σ) : Txn σ Unit := fun _ => .ok () state

def modify (f : σ → σ) : Txn σ Unit := fun state => .ok () (f state)

instance : Monad (Txn σ) where
  pure := pure
  bind := bind

@[simp] theorem pure_apply (value : α) (state : σ) :
    (pure value : Txn σ α) state = .ok value state := rfl

@[simp] theorem bind_ok (value : α) (state : σ) (next : α → Txn σ β) :
    Outcome.bind (.ok value state) next = next value state := rfl

@[simp] theorem bind_abort (code : Nat) (next : α → Txn σ β) :
    Outcome.bind (Outcome.abort (State := σ) (Result := α) code) next = .abort code := rfl

end Txn

end Move.Semantics
