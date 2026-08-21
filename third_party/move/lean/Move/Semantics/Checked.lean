-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Semantics.Spec

/-!
# Checked source arithmetic

Faithful computations for Move `u64` arithmetic. The existing functions in
`Move.Basic` remain compiler markers; the source verifier interprets those
markers with these checked operations.
-/

namespace Move.Semantics.Checked

open Move

/-- The VM uses code zero for arithmetic and bounds failures in the modeled
IR. This is an execution failure code, not a user-written `abort` constant. -/
def arithmeticAbortCode : Nat := 0

private def result (value : Nat) : Except Nat U64 :=
  if value < U64.size then .ok (U64.ofNat value) else .error arithmeticAbortCode

def add (lhs rhs : U64) : Except Nat U64 := result (lhs.toNat + rhs.toNat)

def sub (lhs rhs : U64) : Except Nat U64 :=
  if rhs.toNat ≤ lhs.toNat then .ok (U64.ofNat (lhs.toNat - rhs.toNat))
  else .error arithmeticAbortCode

def mul (lhs rhs : U64) : Except Nat U64 := result (lhs.toNat * rhs.toNat)

def div (lhs rhs : U64) : Except Nat U64 :=
  if rhs.toNat = 0 then .error arithmeticAbortCode
  else .ok (U64.ofNat (lhs.toNat / rhs.toNat))

def mod (lhs rhs : U64) : Except Nat U64 :=
  if rhs.toNat = 0 then .error arithmeticAbortCode
  else .ok (U64.ofNat (lhs.toNat % rhs.toNat))

/-- Lift a checked value operation into transaction semantics. -/
def lift (operation : Except Nat α) : Txn σ α := fun state =>
  match operation with
  | .ok value => .ok value state
  | .error code => .abort code

def addM (lhs rhs : U64) : Txn σ U64 := lift (add lhs rhs)
def subM (lhs rhs : U64) : Txn σ U64 := lift (sub lhs rhs)
def mulM (lhs rhs : U64) : Txn σ U64 := lift (mul lhs rhs)
def divM (lhs rhs : U64) : Txn σ U64 := lift (div lhs rhs)
def modM (lhs rhs : U64) : Txn σ U64 := lift (mod lhs rhs)

/-! Relational operations used by direct verification. -/

def addSpec (lhs rhs : U64) : Spec σ U64 where
  ok := fun initial value final =>
    lhs.toNat + rhs.toNat < U64.size ∧
      value = U64.ofNat (lhs.toNat + rhs.toNat) ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬lhs.toNat + rhs.toNat < U64.size

def subSpec (lhs rhs : U64) : Spec σ U64 where
  ok := fun initial value final =>
    rhs.toNat ≤ lhs.toNat ∧
      value = U64.ofNat (lhs.toNat - rhs.toNat) ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬rhs.toNat ≤ lhs.toNat

def mulSpec (lhs rhs : U64) : Spec σ U64 where
  ok := fun initial value final =>
    lhs.toNat * rhs.toNat < U64.size ∧
      value = U64.ofNat (lhs.toNat * rhs.toNat) ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬lhs.toNat * rhs.toNat < U64.size

def divSpec (lhs rhs : U64) : Spec σ U64 where
  ok := fun initial value final =>
    rhs.toNat ≠ 0 ∧ value = U64.ofNat (lhs.toNat / rhs.toNat) ∧ final = initial
  aborts := fun _ code => code = arithmeticAbortCode ∧ rhs.toNat = 0

def modSpec (lhs rhs : U64) : Spec σ U64 where
  ok := fun initial value final =>
    rhs.toNat ≠ 0 ∧ value = U64.ofNat (lhs.toNat % rhs.toNat) ∧ final = initial
  aborts := fun _ code => code = arithmeticAbortCode ∧ rhs.toNat = 0

@[simp] theorem addSpec_eq_pure {lhs rhs : U64}
    (safe : lhs.toNat + rhs.toNat < U64.size) :
    (addSpec lhs rhs : Spec σ U64) =
      Spec.pure (U64.ofNat (lhs.toNat + rhs.toNat)) := by
  apply Spec.extensionality
  · funext initial value final
    simp [addSpec, Spec.pure, safe]
  · funext initial code
    simp [addSpec, Spec.pure, safe]

@[simp] theorem subSpec_eq_pure {lhs rhs : U64} (safe : rhs.toNat ≤ lhs.toNat) :
    (subSpec lhs rhs : Spec σ U64) =
      Spec.pure (U64.ofNat (lhs.toNat - rhs.toNat)) := by
  apply Spec.extensionality
  · funext initial value final
    simp [subSpec, Spec.pure, safe]
  · funext initial code
    simp [subSpec, Spec.pure, safe]

theorem subSpec_one_eq_pure_of_pos {value : U64}
    (positive : 0 < value.toNat) :
    (subSpec value 1 : Spec σ U64) =
      Spec.pure (U64.ofNat (value.toNat - 1)) := by
  apply subSpec_eq_pure
  change 1 ≤ value.toNat
  exact positive

@[simp] theorem divSpec_eq_pure {lhs rhs : U64} (nonzero : rhs.toNat ≠ 0) :
    (divSpec lhs rhs : Spec σ U64) =
      Spec.pure (U64.ofNat (lhs.toNat / rhs.toNat)) := by
  apply Spec.extensionality
  · funext initial value final
    simp [divSpec, Spec.pure, nonzero]
  · funext initial code
    simp [divSpec, Spec.pure, nonzero]

@[simp] theorem add_success {lhs rhs : U64}
    (h : lhs.toNat + rhs.toNat < U64.size) :
    add lhs rhs = .ok (U64.ofNat (lhs.toNat + rhs.toNat)) := by
  simp [add, result, h]

@[simp] theorem add_overflow {lhs rhs : U64}
    (h : ¬lhs.toNat + rhs.toNat < U64.size) :
    add lhs rhs = .error arithmeticAbortCode := by
  simp [add, result, h]

@[simp] theorem sub_success {lhs rhs : U64} (h : rhs.toNat ≤ lhs.toNat) :
    sub lhs rhs = .ok (U64.ofNat (lhs.toNat - rhs.toNat)) := by
  simp [sub, h]

@[simp] theorem sub_underflow {lhs rhs : U64} (h : ¬rhs.toNat ≤ lhs.toNat) :
    sub lhs rhs = .error arithmeticAbortCode := by
  simp [sub, h]

@[simp] theorem div_zero (lhs : U64) :
    div lhs (U64.ofNat 0) = .error arithmeticAbortCode := by
  simp [div]

end Move.Semantics.Checked
