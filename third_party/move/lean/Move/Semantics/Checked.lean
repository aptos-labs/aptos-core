-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Semantics.Spec

/-!
# Checked source arithmetic

Faithful computations for Move integer arithmetic, generic over the width.
The existing functions in `Move.Basic` remain compiler markers; the source
verifier interprets those markers with these checked operations.
-/

namespace Move.Semantics.Checked

open Move

variable {W : Type} [Width W]

/-- The VM uses code zero for arithmetic and bounds failures in the modeled
IR. This is an execution failure code, not a user-written `abort` constant. -/
def arithmeticAbortCode : Nat := 0

private def result (value : Nat) : Except Nat (UInt W) :=
  if value < (widthOf W).size then .ok (UInt.ofNat value) else .error arithmeticAbortCode

def add (lhs rhs : UInt W) : Except Nat (UInt W) := result (lhs.toNat + rhs.toNat)

def sub (lhs rhs : UInt W) : Except Nat (UInt W) :=
  if rhs.toNat ≤ lhs.toNat then .ok (UInt.ofNat (lhs.toNat - rhs.toNat))
  else .error arithmeticAbortCode

def mul (lhs rhs : UInt W) : Except Nat (UInt W) := result (lhs.toNat * rhs.toNat)

def div (lhs rhs : UInt W) : Except Nat (UInt W) :=
  if rhs.toNat = 0 then .error arithmeticAbortCode
  else .ok (UInt.ofNat (lhs.toNat / rhs.toNat))

def mod (lhs rhs : UInt W) : Except Nat (UInt W) :=
  if rhs.toNat = 0 then .error arithmeticAbortCode
  else .ok (UInt.ofNat (lhs.toNat % rhs.toNat))

/-- Lift a checked value operation into transaction semantics. -/
def lift (operation : Except Nat α) : Txn σ α := fun state =>
  match operation with
  | .ok value => .ok value state
  | .error code => .abort code

def addM (lhs rhs : UInt W) : Txn σ (UInt W) := lift (add lhs rhs)
def subM (lhs rhs : UInt W) : Txn σ (UInt W) := lift (sub lhs rhs)
def mulM (lhs rhs : UInt W) : Txn σ (UInt W) := lift (mul lhs rhs)
def divM (lhs rhs : UInt W) : Txn σ (UInt W) := lift (div lhs rhs)
def modM (lhs rhs : UInt W) : Txn σ (UInt W) := lift (mod lhs rhs)

/-! Relational operations used by direct verification. -/

def addSpec (lhs rhs : UInt W) : Spec σ (UInt W) where
  ok := fun initial value final =>
    lhs.toNat + rhs.toNat < (widthOf W).size ∧
      value = UInt.ofNat (lhs.toNat + rhs.toNat) ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬lhs.toNat + rhs.toNat < (widthOf W).size

def subSpec (lhs rhs : UInt W) : Spec σ (UInt W) where
  ok := fun initial value final =>
    rhs.toNat ≤ lhs.toNat ∧
      value = UInt.ofNat (lhs.toNat - rhs.toNat) ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬rhs.toNat ≤ lhs.toNat

def mulSpec (lhs rhs : UInt W) : Spec σ (UInt W) where
  ok := fun initial value final =>
    lhs.toNat * rhs.toNat < (widthOf W).size ∧
      value = UInt.ofNat (lhs.toNat * rhs.toNat) ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬lhs.toNat * rhs.toNat < (widthOf W).size

def divSpec (lhs rhs : UInt W) : Spec σ (UInt W) where
  ok := fun initial value final =>
    rhs.toNat ≠ 0 ∧ value = UInt.ofNat (lhs.toNat / rhs.toNat) ∧ final = initial
  aborts := fun _ code => code = arithmeticAbortCode ∧ rhs.toNat = 0

def modSpec (lhs rhs : UInt W) : Spec σ (UInt W) where
  ok := fun initial value final =>
    rhs.toNat ≠ 0 ∧ value = UInt.ofNat (lhs.toNat % rhs.toNat) ∧ final = initial
  aborts := fun _ code => code = arithmeticAbortCode ∧ rhs.toNat = 0

@[simp] theorem addSpec_eq_pure {lhs rhs : UInt W}
    (safe : lhs.toNat + rhs.toNat < (widthOf W).size) :
    (addSpec lhs rhs : Spec σ (UInt W)) =
      Spec.pure (UInt.ofNat (lhs.toNat + rhs.toNat)) := by
  apply Spec.extensionality
  · funext initial value final
    simp [addSpec, Spec.pure, safe]
  · funext initial code
    simp [addSpec, Spec.pure, safe]

  · rfl
@[simp] theorem subSpec_eq_pure {lhs rhs : UInt W} (safe : rhs.toNat ≤ lhs.toNat) :
    (subSpec lhs rhs : Spec σ (UInt W)) =
      Spec.pure (UInt.ofNat (lhs.toNat - rhs.toNat)) := by
  apply Spec.extensionality
  · funext initial value final
    simp [subSpec, Spec.pure, safe]
  · funext initial code
    simp [subSpec, Spec.pure, safe]

  · rfl
theorem subSpec_one_eq_pure_of_pos {value : UInt W}
    (positive : 0 < value.toNat) :
    (subSpec value 1 : Spec σ (UInt W)) =
      Spec.pure (UInt.ofNat (value.toNat - 1)) := by
  have hone : (1 : UInt W).toNat = 1 := by
    rw [UInt.toNat_ofNat_numeral, Nat.mod_eq_of_lt (widthOf W).one_lt_size]
  rw [show UInt.ofNat (value.toNat - 1)
      = UInt.ofNat (value.toNat - (1 : UInt W).toNat) by rw [hone]]
  apply subSpec_eq_pure
  omega

@[simp] theorem mulSpec_eq_pure {lhs rhs : UInt W}
    (safe : lhs.toNat * rhs.toNat < (widthOf W).size) :
    (mulSpec lhs rhs : Spec σ (UInt W)) =
      Spec.pure (UInt.ofNat (lhs.toNat * rhs.toNat)) := by
  apply Spec.extensionality
  · funext initial value final
    simp [mulSpec, Spec.pure, safe]
  · funext initial code
    simp [mulSpec, Spec.pure, safe]

  · rfl
@[simp] theorem divSpec_eq_pure {lhs rhs : UInt W} (nonzero : rhs.toNat ≠ 0) :
    (divSpec lhs rhs : Spec σ (UInt W)) =
      Spec.pure (UInt.ofNat (lhs.toNat / rhs.toNat)) := by
  apply Spec.extensionality
  · funext initial value final
    simp [divSpec, Spec.pure, nonzero]
  · funext initial code
    simp [divSpec, Spec.pure, nonzero]

  · rfl
@[simp] theorem modSpec_eq_pure {lhs rhs : UInt W} (nonzero : rhs.toNat ≠ 0) :
    (modSpec lhs rhs : Spec σ (UInt W)) =
      Spec.pure (UInt.ofNat (lhs.toNat % rhs.toNat)) := by
  apply Spec.extensionality
  · funext initial value final
    simp [modSpec, Spec.pure, nonzero]
  · funext initial code
    simp [modSpec, Spec.pure, nonzero]

  · rfl
@[simp] theorem add_success {lhs rhs : UInt W}
    (h : lhs.toNat + rhs.toNat < (widthOf W).size) :
    add lhs rhs = .ok (UInt.ofNat (lhs.toNat + rhs.toNat)) := by
  simp [add, result, h]

@[simp] theorem add_overflow {lhs rhs : UInt W}
    (h : ¬lhs.toNat + rhs.toNat < (widthOf W).size) :
    add lhs rhs = .error arithmeticAbortCode := by
  simp [add, result, h]

@[simp] theorem sub_success {lhs rhs : UInt W} (h : rhs.toNat ≤ lhs.toNat) :
    sub lhs rhs = .ok (UInt.ofNat (lhs.toNat - rhs.toNat)) := by
  simp [sub, h]

@[simp] theorem sub_underflow {lhs rhs : UInt W} (h : ¬rhs.toNat ≤ lhs.toNat) :
    sub lhs rhs = .error arithmeticAbortCode := by
  simp [sub, h]

@[simp] theorem div_zero (lhs : UInt W) :
    div lhs (UInt.ofNat 0) = .error arithmeticAbortCode := by
  simp [div]

/-! Checked shifts and casts. Bitwise `&&&`, `|||`, and `^^^` never abort, so
the pure markers of `Move.Basic` are already their faithful semantics. -/

def shl (lhs : UInt W) (amount : UInt W8) : Except Nat (UInt W) :=
  if amount.toNat < (widthOf W).bits then .ok (UInt.shl lhs amount)
  else .error arithmeticAbortCode

def shr (lhs : UInt W) (amount : UInt W8) : Except Nat (UInt W) :=
  if amount.toNat < (widthOf W).bits then .ok (UInt.shr lhs amount)
  else .error arithmeticAbortCode

def cast {W' : Type} [Width W'] (value : UInt W) : Except Nat (UInt W') :=
  if value.toNat < (widthOf W').size then .ok (UInt.cast value)
  else .error arithmeticAbortCode

def shlSpec (lhs : UInt W) (amount : UInt W8) : Spec σ (UInt W) where
  ok := fun initial value final =>
    amount.toNat < (widthOf W).bits ∧ value = UInt.shl lhs amount ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬amount.toNat < (widthOf W).bits

def shrSpec (lhs : UInt W) (amount : UInt W8) : Spec σ (UInt W) where
  ok := fun initial value final =>
    amount.toNat < (widthOf W).bits ∧ value = UInt.shr lhs amount ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬amount.toNat < (widthOf W).bits

def castSpec {W' : Type} [Width W'] (value : UInt W) : Spec σ (UInt W') where
  ok := fun initial result final =>
    value.toNat < (widthOf W').size ∧ result = UInt.cast value ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬value.toNat < (widthOf W').size

@[simp] theorem shlSpec_eq_pure {lhs : UInt W} {amount : UInt W8}
    (safe : amount.toNat < (widthOf W).bits) :
    (shlSpec lhs amount : Spec σ (UInt W)) = Spec.pure (UInt.shl lhs amount) := by
  apply Spec.extensionality
  · funext initial value final
    simp [shlSpec, Spec.pure, safe]
  · funext initial code
    simp [shlSpec, Spec.pure, safe]

  · rfl
@[simp] theorem shrSpec_eq_pure {lhs : UInt W} {amount : UInt W8}
    (safe : amount.toNat < (widthOf W).bits) :
    (shrSpec lhs amount : Spec σ (UInt W)) = Spec.pure (UInt.shr lhs amount) := by
  apply Spec.extensionality
  · funext initial value final
    simp [shrSpec, Spec.pure, safe]
  · funext initial code
    simp [shrSpec, Spec.pure, safe]

  · rfl
@[simp] theorem castSpec_eq_pure {W' : Type} [Width W'] {value : UInt W}
    (safe : value.toNat < (widthOf W').size) :
    (castSpec value : Spec σ (UInt W')) = Spec.pure (UInt.cast value) := by
  apply Spec.extensionality
  · funext initial result final
    simp [castSpec, Spec.pure, safe]
  · funext initial code
    simp [castSpec, Spec.pure, safe]

  · rfl
/-- The checked operations abort rather than leaving an obligation. -/
@[simp] theorem total_addSpec (lhs rhs : Move.UInt W) :
    Spec.Total (addSpec lhs rhs : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_subSpec (lhs rhs : Move.UInt W) :
    Spec.Total (subSpec lhs rhs : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_mulSpec (lhs rhs : Move.UInt W) :
    Spec.Total (mulSpec lhs rhs : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_divSpec (lhs rhs : Move.UInt W) :
    Spec.Total (divSpec lhs rhs : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_modSpec (lhs rhs : Move.UInt W) :
    Spec.Total (modSpec lhs rhs : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_shlSpec (lhs : Move.UInt W) (amount : Move.U8) :
    Spec.Total (shlSpec lhs amount : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_shrSpec (lhs : Move.UInt W) (amount : Move.U8) :
    Spec.Total (shrSpec lhs amount : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_castSpec {W' : Type} [Move.Width W'] (value : Move.UInt W) :
    Spec.Total (castSpec value : Spec σ (Move.UInt W')) := fun _ h => h.elim

end Move.Semantics.Checked
