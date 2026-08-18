-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# Leaner source types

Opaque source-level types recognized by the Lean-to-Move compiler.  Their Lean
implementations are not the runtime representation used by MoveModel.
-/

namespace Move

/-- Identity marker used by the `continue f args...` syntax.  The Leaner
normalizer consumes this call and requires the marked application to become a
CFG back edge; it has no Move runtime representation. -/
@[noinline] opaque continueMarker {α : Sort u} (value : α) : α := value

/-- A Move account address.  The private payload exists only so source
declarations pass through Lean's compiler; the Move backend supplies the real
256-bit representation. -/
structure Address where
  private mk ::
  private dummy : Nat
  deriving Inhabited

/-- A Move signer value. -/
structure Signer where
  private mk ::
  private dummy : Nat
  deriving Inhabited

/-- Move's unsigned fixed-width integer types.  Only `U64` is enabled in the
first lowering milestone; the others reserve the source names. -/
structure U8 where private mk :: private dummy : Nat deriving Inhabited
structure U16 where private mk :: private dummy : Nat deriving Inhabited
structure U32 where private mk :: private dummy : Nat deriving Inhabited
structure U64 where
  private mk ::
  private value : Nat
  deriving Inhabited, DecidableEq, Repr
structure U128 where private mk :: private dummy : Nat deriving Inhabited
structure U256 where private mk :: private dummy : Nat deriving Inhabited

/-- A homogeneous Move vector. Its list payload exists only to give source
programs an ordinary Lean type; Move supplies the runtime representation. -/
structure Vector (α : Type) where
  private mk ::
  private elems : List α
  deriving Inhabited

/- Move's built-in structural comparison.  Compiler-v2 implements ordering as
`std::cmp::compare<T>` and equality as the generic bytecode equality operator.
Primitives use their natural ordering, while vectors, structures, and enums
are compared lexicographically.  The host bodies are only markers; source
verification supplies their logical laws. -/
namespace Compare

@[noinline] opaque less {T : Type} (left right : T) : Bool := false
@[noinline] opaque equal {T : Type} (left right : T) : Bool := false

def Less {T : Type} (left right : T) : Prop := less left right = true

instance (priority := low) genericLT {T : Type} : LT T := ⟨Less⟩
instance (priority := low) genericDecidableLT {T : Type} (left right : T) :
    Decidable (@LT.lt T (genericLT (T := T)) left right) :=
  inferInstanceAs (Decidable (less left right = true))

/-- Every Move value supports structural equality. The low priority preserves
ordinary Lean equality instances outside generic Move source while making
`left == right` available without a type-class constraint in generic `fun`s. -/
instance (priority := low) genericBEq {T : Type} : BEq T := ⟨equal⟩

end Compare

/-- An immutable Move reference.  Its payload is only a non-executable source
model used to make the primitives compilable. -/
structure Ref (α : Type) where
  private mk ::
  private value : α

/-- Compiler-facing mutable Move reference. `current` is the checked-out value
and `prophecy` is the ghost value expected at loan death. Marker execution
initializes both equally; the faithful relational semantics in
`Move.Semantics.Reference` introduces a fresh prophecy. -/
structure MutRef (α : Type) where
  private mk ::
  private current : α
  private prophecy : α

/-- Compile-time metadata identifying one structure field. -/
structure Field (Owner FieldTy : Type) where
  private mk ::
  private projection : Owner → FieldTy

/-- Construct field metadata from a genuine Lean structure projection. -/
@[noinline] opaque fieldOfProjection {Owner FieldTy : Type} :
  (Owner → FieldTy) → Field Owner FieldTy := fun projection => ⟨projection⟩

namespace Ref

@[noinline] opaque default [Inhabited α] : Ref α := ⟨(Inhabited.default : α)⟩
@[noinline] opaque ofValue (value : α) : Ref α := ⟨value⟩
@[noinline] opaque get (r : Ref α) : α := r.value

instance [Inhabited α] : Inhabited (Ref α) := ⟨default⟩

end Ref

namespace MutRef

@[noinline] opaque default [Inhabited α] : MutRef α :=
  ⟨(Inhabited.default : α), (Inhabited.default : α)⟩
@[noinline] opaque ofValue (value : α) : MutRef α := ⟨value, value⟩
@[noinline] opaque get (r : MutRef α) : α := r.current

instance [Inhabited α] : Inhabited (MutRef α) := ⟨default⟩

end MutRef

/-- Implicitly view a mutable Move reference as immutable.  Lean inserts this
coercion at Move call sites; the compiler lowers the marker to `freeze_ref`.
Unlike `freeze`, this is not an `Action`: freezing only weakens the reference
permission and has no independently observable effect. -/
@[noinline] opaque freezeRef {α : Type} (ref : MutRef α) : Ref α :=
  Ref.ofValue ref.get

instance {α : Type} : CoeOut (MutRef α) (Ref α) := ⟨freezeRef⟩

namespace Field

@[noinline] opaque get (field : Field Owner FieldTy) (owner : Owner) : FieldTy :=
  field.projection owner

end Field

namespace U64

/-- The exclusive upper bound of Move's `u64` value range. -/
def size : Nat := 2 ^ 64

/-- Expose the mathematical value of a source `u64` for specifications. -/
def toNat (value : U64) : Nat := value.value

/-- Source `u64` values are determined by their mathematical value. -/
@[ext] theorem ext {left right : U64}
    (equal : left.toNat = right.toNat) : left = right := by
  cases left
  cases right
  simp [toNat] at equal
  cases equal
  rfl

/-- Whether a source value belongs to Move's runtime `u64` domain. The
compiler only constructs valid values; keeping this predicate explicit lets
the source semantics state that invariant without identifying `U64` with
Lean's wrapping `UInt64`. -/
def Valid (value : U64) : Prop := value.toNat < size

/-- Compiler-recognized `u64` literal. Kept out of line so LCNF preserves the
named marker while remaining reducible for source-level proofs. -/
@[noinline] def ofNat (n : Nat) : U64 := ⟨n⟩

@[simp] theorem toNat_ofNat (n : Nat) : (ofNat n).toNat = n := rfl

/-- Compiler-recognized Move arithmetic.  Move overflow behavior is supplied
by the backend, not by Lean's native fixed-width arithmetic. -/
@[noinline] def add : U64 → U64 → U64 := fun a b => ⟨a.value + b.value⟩
@[noinline] def sub : U64 → U64 → U64 := fun a b => ⟨a.value - b.value⟩
@[noinline] def mul : U64 → U64 → U64 := fun a b => ⟨a.value * b.value⟩
@[noinline] def div : U64 → U64 → U64 := fun a b => ⟨a.value / b.value⟩
@[noinline] def mod : U64 → U64 → U64 := fun a b => ⟨a.value % b.value⟩

/-- Compiler-recognized comparison, represented by a `Bool` so Lean's ordinary
`if` lowering exposes a direct case split in LCNF. -/
@[noinline] opaque less : U64 → U64 → Bool := fun a b => decide (a.value < b.value)
@[noinline] opaque lessEq : U64 → U64 → Bool := fun a b => decide (a.value ≤ b.value)
@[noinline] opaque equal : U64 → U64 → Bool := fun a b => decide (a.value = b.value)

instance (n : Nat) : OfNat U64 n := ⟨ofNat n⟩

theorem eq_zero_of_not_pos {value : U64} (notPositive : ¬0 < value.toNat) :
    value = 0 := by
  apply ext
  change value.toNat = 0
  exact Nat.eq_zero_of_not_pos notPositive

instance : Add U64 := ⟨add⟩
instance : Sub U64 := ⟨sub⟩
instance : Mul U64 := ⟨mul⟩
instance : Div U64 := ⟨div⟩
instance : Mod U64 := ⟨mod⟩
instance : BEq U64 := ⟨equal⟩

@[simp] theorem add_eq_ofNat (a b : U64) :
    a + b = ofNat (a.toNat + b.toNat) := rfl

@[simp] theorem sub_eq_ofNat (a b : U64) :
    a - b = ofNat (a.toNat - b.toNat) := rfl

@[simp] theorem mul_eq_ofNat (a b : U64) :
    a * b = ofNat (a.toNat * b.toNat) := rfl

@[simp] theorem div_eq_ofNat (a b : U64) :
    a / b = ofNat (a.toNat / b.toNat) := rfl

@[simp] theorem mod_eq_ofNat (a b : U64) :
    a % b = ofNat (a.toNat % b.toNat) := rfl

/-- `<` remains ordinary Lean syntax.  Its decision procedure is the named
`U64.less` primitive, which the backend lowers to `MoveModel.IR.Oper.lt`. -/
instance : LT U64 := ⟨fun a b => less a b = true⟩
instance (a b : U64) : Decidable (a < b) :=
  inferInstanceAs (Decidable (less a b = true))

end U64

namespace Vector

@[noinline] def empty : Vector α := ⟨[]⟩
@[noinline] def push : Vector α → α → Vector α :=
  fun values value => ⟨values.elems ++ [value]⟩
@[noinline] def length : Vector α → U64 :=
  fun values => U64.ofNat values.elems.length
@[noinline] def get [Inhabited α] : Vector α → U64 → α :=
  fun values index => values.elems[index.toNat]?.getD Inhabited.default
@[noinline] def set : Vector α → U64 → α → Vector α :=
  fun values index value => ⟨values.elems.set index.toNat value⟩

/-- Logical contents of a source vector. This is a specification accessor and
is never selected for Move lowering. -/
def toList (values : Vector α) : List α := values.elems

/-- Construct a logical source vector from a list.  This is a verification
helper, not a compiler primitive; deployable source builds vectors with the
ordinary vector operations. -/
def ofList (values : List α) : Vector α := ⟨values⟩

@[simp] theorem toList_empty : (empty : Vector α).toList = [] := rfl
@[simp] theorem toList_push (values : Vector α) (value : α) :
    (push values value).toList = values.toList ++ [value] := rfl
@[simp] theorem length_toNat (values : Vector α) :
    (length values).toNat = values.toList.length := rfl
@[simp] theorem toList_set (values : Vector α) (index : U64) (value : α) :
    (set values index value).toList = values.toList.set index.toNat value := rfl
@[simp] theorem toList_ofList (values : List α) : (ofList values).toList = values := rfl

end Vector

end Move
