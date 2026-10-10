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

/-! ## Relational operations used by direct verification

One family for both signednesses: each operation computes over `Int` and is
accepted exactly when the result lies in the operand type's range.  Only
`nt.lo`/`nt.hi` distinguish signed from unsigned, so there is one definition,
one `_eq_pure` equation, and one totality lemma per operation.  The per-sign
*views* (`Nat` for unsigned, `Int` for signed) are recovered by the
normalization lemmas at the end of this section. -/

/-- Range membership at a numeric type: the single predicate every checked
operation consults. -/
def inRange (nt : NumType) (n : Int) : Prop := nt.lo ≤ n ∧ n < nt.hi

instance (nt : NumType) (n : Int) : Decidable (inRange nt n) := by
  unfold inRange; infer_instance

/-- The unsigned view of the range condition. -/
theorem inRange_unsigned_iff {n : Int} :
    inRange (numTypeOf Unsigned W) n ↔ (0 ≤ n ∧ n < ((widthOf W).size : Int)) :=
  Iff.rfl


variable {S : Type} [Sign S]

def addSpec (lhs rhs : MoveInt S W) : Spec σ (MoveInt S W) where
  ok := fun initial value final =>
    inRange (numTypeOf S W) (lhs.toInt + rhs.toInt) ∧
      value = MoveInt.ofInt (lhs.toInt + rhs.toInt) ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬ inRange (numTypeOf S W) (lhs.toInt + rhs.toInt)

def subSpec (lhs rhs : MoveInt S W) : Spec σ (MoveInt S W) where
  ok := fun initial value final =>
    inRange (numTypeOf S W) (lhs.toInt - rhs.toInt) ∧
      value = MoveInt.ofInt (lhs.toInt - rhs.toInt) ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬ inRange (numTypeOf S W) (lhs.toInt - rhs.toInt)

def mulSpec (lhs rhs : MoveInt S W) : Spec σ (MoveInt S W) where
  ok := fun initial value final =>
    inRange (numTypeOf S W) (lhs.toInt * rhs.toInt) ∧
      value = MoveInt.ofInt (lhs.toInt * rhs.toInt) ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬ inRange (numTypeOf S W) (lhs.toInt * rhs.toInt)

def divSpec (lhs rhs : MoveInt S W) : Spec σ (MoveInt S W) where
  ok := fun initial value final =>
    rhs.toInt ≠ 0 ∧ inRange (numTypeOf S W) (lhs.toInt.tdiv rhs.toInt) ∧
      value = MoveInt.ofInt (lhs.toInt.tdiv rhs.toInt) ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧
      (rhs.toInt = 0 ∨ ¬ inRange (numTypeOf S W) (lhs.toInt.tdiv rhs.toInt))

def modSpec (lhs rhs : MoveInt S W) : Spec σ (MoveInt S W) where
  ok := fun initial value final =>
    rhs.toInt ≠ 0 ∧ value = MoveInt.ofInt (lhs.toInt.tmod rhs.toInt) ∧ final = initial
  aborts := fun _ code => code = arithmeticAbortCode ∧ rhs.toInt = 0

@[simp] theorem addSpec_eq_pure {lhs rhs : MoveInt S W}
    (safe : inRange (numTypeOf S W) (lhs.toInt + rhs.toInt)) :
    (addSpec lhs rhs : Spec σ (MoveInt S W)) =
      Spec.pure (MoveInt.ofInt (lhs.toInt + rhs.toInt)) := by
  apply Spec.extensionality
  · funext initial value final; simp [addSpec, Spec.pure, safe]
  · funext initial code; simp [addSpec, Spec.pure, safe]
  · rfl

@[simp] theorem subSpec_eq_pure {lhs rhs : MoveInt S W}
    (safe : inRange (numTypeOf S W) (lhs.toInt - rhs.toInt)) :
    (subSpec lhs rhs : Spec σ (MoveInt S W)) =
      Spec.pure (MoveInt.ofInt (lhs.toInt - rhs.toInt)) := by
  apply Spec.extensionality
  · funext initial value final; simp [subSpec, Spec.pure, safe]
  · funext initial code; simp [subSpec, Spec.pure, safe]
  · rfl

@[simp] theorem mulSpec_eq_pure {lhs rhs : MoveInt S W}
    (safe : inRange (numTypeOf S W) (lhs.toInt * rhs.toInt)) :
    (mulSpec lhs rhs : Spec σ (MoveInt S W)) =
      Spec.pure (MoveInt.ofInt (lhs.toInt * rhs.toInt)) := by
  apply Spec.extensionality
  · funext initial value final; simp [mulSpec, Spec.pure, safe]
  · funext initial code; simp [mulSpec, Spec.pure, safe]
  · rfl

@[simp] theorem divSpec_eq_pure {lhs rhs : MoveInt S W} (nonzero : rhs.toInt ≠ 0)
    (safe : inRange (numTypeOf S W) (lhs.toInt.tdiv rhs.toInt)) :
    (divSpec lhs rhs : Spec σ (MoveInt S W)) =
      Spec.pure (MoveInt.ofInt (lhs.toInt.tdiv rhs.toInt)) := by
  apply Spec.extensionality
  · funext initial value final; simp [divSpec, Spec.pure, nonzero, safe]
  · funext initial code; simp [divSpec, Spec.pure, nonzero, safe]
  · rfl

@[simp] theorem modSpec_eq_pure {lhs rhs : MoveInt S W} (nonzero : rhs.toInt ≠ 0) :
    (modSpec lhs rhs : Spec σ (MoveInt S W)) =
      Spec.pure (MoveInt.ofInt (lhs.toInt.tmod rhs.toInt)) := by
  apply Spec.extensionality
  · funext initial value final; simp [modSpec, Spec.pure, nonzero]
  · funext initial code; simp [modSpec, Spec.pure, nonzero]
  · rfl

/-- Decrementing a positive unsigned value never aborts.  Stated in the
natural-number view, which is how loop proofs carry the guard. -/
theorem subSpec_one_eq_pure_of_pos {value : UInt W} (positive : 0 < value.toNat) :
    (subSpec value 1 : Spec σ (UInt W)) =
      Spec.pure (MoveInt.ofInt (value.toInt - (1 : UInt W).toInt)) := by
  refine subSpec_eq_pure ?_
  have hlt := value.toNat_lt
  have hv := UInt.toInt_eq_toNat value
  have h1 : (1 : UInt W).toInt = 1 := by
    rw [UInt.toInt_eq_toNat, UInt.toNat_one]; rfl
  rw [inRange_unsigned_iff]
  omega

/-! ## The unsigned view of the range condition

`inRange` is stated over `Int`; for unsigned operands it reduces to the
natural-number bound that unsigned specifications are written in.  These are
the only place the two views meet. -/

@[simp high] theorem inRange_add (a b : UInt W) :
    inRange (numTypeOf Unsigned W) (a.toInt + b.toInt) ↔
      a.toNat + b.toNat < (widthOf W).size := by
  rw [inRange_unsigned_iff, UInt.toInt_eq_toNat, UInt.toInt_eq_toNat]
  omega

/-- The range condition for a nonnegative `Int` addend (covers literals). -/
theorem inRange_add_nonneg (a : UInt W) {m : Int} (h : 0 ≤ m) :
    inRange (numTypeOf Unsigned W) (a.toInt + m) ↔
      a.toNat + m.toNat < (widthOf W).size := by
  rw [inRange_unsigned_iff, UInt.toInt_eq_toNat]
  omega

/-- The range condition for a bare `Int` numeral addend. -/
theorem inRange_add_intLit (a : UInt W) (n : Nat) :
    inRange (numTypeOf Unsigned W) (a.toInt + (no_index (OfNat.ofNat n) : Int)) ↔
      a.toNat + n < (widthOf W).size := by
  rw [inRange_unsigned_iff, UInt.toInt_eq_toNat]
  have hlit : ((OfNat.ofNat n : Int)) = ((n : Nat) : Int) := rfl
  rw [hlit]; omega

/-- The range condition after one operand has normalized to a natural cast:
unsigned checked results reach `inRange` in that mixed shape. -/
@[simp high] theorem inRange_add_natCast (a : UInt W) (n : Nat) :
    inRange (numTypeOf Unsigned W) (a.toInt + (n : Int)) ↔
      a.toNat + n < (widthOf W).size := by
  rw [inRange_unsigned_iff, UInt.toInt_eq_toNat]
  omega

@[simp high] theorem inRange_natCast_add (n : Nat) (a : UInt W) :
    inRange (numTypeOf Unsigned W) ((n : Int) + a.toInt) ↔
      n + a.toNat < (widthOf W).size := by
  rw [inRange_unsigned_iff, UInt.toInt_eq_toNat]
  omega

@[simp] theorem inRange_natCast_pair (m n : Nat) :
    inRange (numTypeOf Unsigned W) ((m : Int) + (n : Int)) ↔
      m + n < (widthOf W).size := by
  rw [inRange_unsigned_iff]
  omega

@[simp high] theorem inRange_sub (a b : UInt W) :
    inRange (numTypeOf Unsigned W) (a.toInt - b.toInt) ↔ b.toNat ≤ a.toNat := by
  have ha := a.toNat_lt
  rw [inRange_unsigned_iff, UInt.toInt_eq_toNat, UInt.toInt_eq_toNat]
  omega

@[simp high] theorem inRange_mul (a b : UInt W) :
    inRange (numTypeOf Unsigned W) (a.toInt * b.toInt) ↔
      a.toNat * b.toNat < (widthOf W).size := by
  rw [inRange_unsigned_iff, UInt.toInt_eq_toNat, UInt.toInt_eq_toNat,
    ← Int.natCast_mul]
  constructor
  · rintro ⟨-, h⟩; exact_mod_cast h
  · intro h; exact ⟨Int.natCast_nonneg _, by exact_mod_cast h⟩

/-- The unsigned range condition on a natural-number result. -/
@[simp] theorem inRange_natCast {n : Nat} :
    inRange (numTypeOf Unsigned W) ((n : Nat) : Int) ↔ n < (widthOf W).size := by
  rw [inRange_unsigned_iff]
  omega

@[simp high] theorem inRange_tdiv (a b : UInt W) :
    inRange (numTypeOf Unsigned W) (a.toInt.tdiv b.toInt) := by
  have ha := a.toNat_lt
  rw [UInt.toInt_tdiv, inRange_natCast]
  exact Nat.lt_of_le_of_lt (Nat.div_le_self _ _) ha

/-- After cast normalization the unsigned quotient appears as an `Int`
division of casts; it is still always in range. -/
@[simp] theorem inRange_natCast_div (a b : UInt W) :
    inRange (numTypeOf Unsigned W) ((a.toNat : Int) / (b.toNat : Int)) := by
  have ha := a.toNat_lt
  rw [← Int.natCast_ediv, inRange_natCast]
  exact Nat.lt_of_le_of_lt (Nat.div_le_self _ _) ha

@[simp] theorem inRange_natCast_emod (a b : UInt W) :
    inRange (numTypeOf Unsigned W) ((a.toNat : Int) % (b.toNat : Int)) := by
  have ha := a.toNat_lt
  rw [← Int.natCast_emod, inRange_natCast]
  exact Nat.lt_of_le_of_lt (Nat.mod_le _ _) ha

@[simp] theorem inRange_cast {W' : Type} [Width W'] (a : UInt W) :
    inRange (numTypeOf Unsigned W') a.toInt ↔ a.toNat < (widthOf W').size := by
  rw [inRange_unsigned_iff, UInt.toInt_eq_toNat]
  omega

/-- The unsigned view of a zero divisor. -/
@[simp] theorem toInt_ne_zero_iff (a : UInt W) : a.toInt ≠ 0 ↔ a.toNat ≠ 0 := by
  rw [UInt.toInt_eq_toNat]; omega

@[simp] theorem toInt_eq_zero_iff (a : UInt W) : a.toInt = 0 ↔ a.toNat = 0 := by
  rw [UInt.toInt_eq_toNat]; omega

/-- The unsigned view of checked subtraction: its range condition is exactly
the non-underflow bound, in the natural-number shape unsigned proofs carry. -/
@[simp high] theorem subSpec_eq_pure_unsigned {lhs rhs : UInt W}
    (safe : rhs.toNat ≤ lhs.toNat) :
    (subSpec lhs rhs : Spec σ (UInt W)) =
      Spec.pure (UInt.ofNat (lhs.toNat - rhs.toNat)) := by
  rw [subSpec_eq_pure ((inRange_sub lhs rhs).mpr safe), UInt.ofInt_sub _ _ safe]

/-- The unsigned view of checked addition, in the natural-number shape. -/
@[simp high] theorem addSpec_eq_pure_unsigned {lhs rhs : UInt W}
    (safe : lhs.toNat + rhs.toNat < (widthOf W).size) :
    (addSpec lhs rhs : Spec σ (UInt W)) =
      Spec.pure (UInt.ofNat (lhs.toNat + rhs.toNat)) := by
  rw [addSpec_eq_pure ((inRange_add lhs rhs).mpr safe), UInt.ofInt_add]

/-- The unsigned view of checked division.  Its range condition is vacuous — a
quotient of nonnegatives is at most the dividend — so only the zero-divisor
side condition remains.  A single side condition also keeps `simp`'s
conditional rewriting effective, which a two-hypothesis rule is not. -/
@[simp high] theorem divSpec_eq_pure_unsigned {lhs rhs : UInt W}
    (nonzero : rhs.toInt ≠ 0) :
    (divSpec lhs rhs : Spec σ (UInt W)) =
      Spec.pure (UInt.ofNat (lhs.toNat / rhs.toNat)) := by
  rw [divSpec_eq_pure nonzero (inRange_tdiv lhs rhs), UInt.ofInt_tdiv]

/-- The unsigned view of checked multiplication and remainder, completing the
per-view `_eq_pure` set (add / sub / div are above).  Every rule a proof can
reach thus produces the `ofNat`-of-`toNat` value that unsigned specifications
are written in; the generic rules serve the signed view only. -/
@[simp high] theorem mulSpec_eq_pure_unsigned {lhs rhs : UInt W}
    (safe : lhs.toNat * rhs.toNat < (widthOf W).size) :
    (mulSpec lhs rhs : Spec σ (UInt W)) =
      Spec.pure (UInt.ofNat (lhs.toNat * rhs.toNat)) := by
  rw [mulSpec_eq_pure ((inRange_mul lhs rhs).mpr safe), UInt.ofInt_mul]

@[simp high] theorem modSpec_eq_pure_unsigned {lhs rhs : UInt W}
    (nonzero : rhs.toNat ≠ 0) :
    (modSpec lhs rhs : Spec σ (UInt W)) =
      Spec.pure (UInt.ofNat (lhs.toNat % rhs.toNat)) := by
  rw [modSpec_eq_pure ((toInt_ne_zero_iff rhs).mpr nonzero), UInt.ofInt_tmod]

-- `inRange` is opaque to decision procedures: unfolding it turns every
-- checked-arithmetic side condition into raw `Int` arithmetic over quotients
-- and remainders, which no linear procedure can close.  The reduction lemmas
-- above are its interface — one per view, as intended.
attribute [irreducible] inRange

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

def shlSpec (lhs : MoveInt S W) (amount : UInt W8) : Spec σ (MoveInt S W) where
  ok := fun initial value final =>
    amount.toNat < (widthOf W).bits ∧ value = MoveInt.shl lhs amount ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬amount.toNat < (widthOf W).bits

def shrSpec (lhs : MoveInt S W) (amount : UInt W8) : Spec σ (MoveInt S W) where
  ok := fun initial value final =>
    amount.toNat < (widthOf W).bits ∧ value = MoveInt.shr lhs amount ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬amount.toNat < (widthOf W).bits

/-- Casting confines the value to the *target* type's range, so a negative
signed source aborts at an unsigned target exactly as an oversized unsigned
source does. -/
def castSpec {S' W' : Type} [Sign S'] [Width W'] (value : MoveInt S W) :
    Spec σ (MoveInt S' W') where
  ok := fun initial result final =>
    inRange (numTypeOf S' W') value.toInt ∧
      result = MoveInt.cast value ∧ final = initial
  aborts := fun _ code =>
    code = arithmeticAbortCode ∧ ¬ inRange (numTypeOf S' W') value.toInt

@[simp] theorem shlSpec_eq_pure {lhs : MoveInt S W} {amount : UInt W8}
    (safe : amount.toNat < (widthOf W).bits) :
    (shlSpec lhs amount : Spec σ (MoveInt S W)) =
      Spec.pure (MoveInt.shl lhs amount) := by
  apply Spec.extensionality
  · funext initial value final
    simp [shlSpec, Spec.pure, safe]
  · funext initial code
    simp [shlSpec, Spec.pure, safe]

  · rfl
@[simp] theorem shrSpec_eq_pure {lhs : MoveInt S W} {amount : UInt W8}
    (safe : amount.toNat < (widthOf W).bits) :
    (shrSpec lhs amount : Spec σ (MoveInt S W)) =
      Spec.pure (MoveInt.shr lhs amount) := by
  apply Spec.extensionality
  · funext initial value final
    simp [shrSpec, Spec.pure, safe]
  · funext initial code
    simp [shrSpec, Spec.pure, safe]

  · rfl
@[simp] theorem castSpec_eq_pure {S' W' : Type} [Sign S'] [Width W']
    {value : MoveInt S W} (safe : inRange (numTypeOf S' W') value.toInt) :
    (castSpec value : Spec σ (MoveInt S' W')) =
      Spec.pure (MoveInt.cast value) := by
  apply Spec.extensionality
  · funext initial result final
    simp [castSpec, Spec.pure, safe]
  · funext initial code
    simp [castSpec, Spec.pure, safe]
  · rfl

/-- The unsigned view of the checked cast. -/
@[simp high] theorem castSpec_eq_pure_unsigned {W' : Type} [Width W']
    {value : UInt W} (safe : value.toNat < (widthOf W').size) :
    (castSpec value : Spec σ (UInt W')) = Spec.pure (MoveInt.cast value) :=
  castSpec_eq_pure ((inRange_cast value).mpr safe)

/-- The checked operations abort rather than leaving an obligation. -/
@[simp] theorem total_addSpec (lhs rhs : MoveInt S W) :
    Spec.Total (addSpec lhs rhs : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_subSpec (lhs rhs : MoveInt S W) :
    Spec.Total (subSpec lhs rhs : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_mulSpec (lhs rhs : MoveInt S W) :
    Spec.Total (mulSpec lhs rhs : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_divSpec (lhs rhs : MoveInt S W) :
    Spec.Total (divSpec lhs rhs : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_modSpec (lhs rhs : MoveInt S W) :
    Spec.Total (modSpec lhs rhs : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_shlSpec (lhs : MoveInt S W) (amount : Move.U8) :
    Spec.Total (shlSpec lhs amount : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_shrSpec (lhs : MoveInt S W) (amount : Move.U8) :
    Spec.Total (shrSpec lhs amount : Spec σ _) := fun _ h => h.elim

@[simp] theorem total_castSpec {S' W' : Type} [Sign S'] [Move.Width W']
    (value : MoveInt S W) :
    Spec.Total (castSpec value : Spec σ (MoveInt S' W')) := fun _ h => h.elim

end Move.Semantics.Checked
