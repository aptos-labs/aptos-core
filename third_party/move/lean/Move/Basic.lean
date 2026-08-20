-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Value

/-!
# Leaner source types

Opaque source-level types recognized by the Lean-to-Move compiler.  Their Lean
implementations are not the runtime representation used by MoveModel.
-/

namespace Move

export MoveModel.IR (IntWidth)

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

/-- Type-level names for Move's integer widths.  The Lean compiler's
intermediate representation erases value indices from types, so `UInt` is
indexed by a tag *type*; `Width` maps the tag back to the semantic width. -/
structure W8 where private mk ::
structure W16 where private mk ::
structure W32 where private mk ::
structure W64 where private mk ::
structure W128 where private mk ::
structure W256 where private mk ::

/-- The semantic width named by a width-tag type. -/
class Width (W : Type) where
  width : IntWidth

instance : Width W8 := ⟨.w8⟩
instance : Width W16 := ⟨.w16⟩
instance : Width W32 := ⟨.w32⟩
instance : Width W64 := ⟨.w64⟩
instance : Width W128 := ⟨.w128⟩
instance : Width W256 := ⟨.w256⟩

/-- The semantic width of a width-tag type. -/
abbrev widthOf (W : Type) [Width W] : IntWidth := Width.width W

/-- Move's unsigned integer of the width named by `W`: the subtype of signed
unbounded integers within the width's range
(`{ x : Int // 0 ≤ x ∧ x < 2 ^ bits }`, the model's `IsValid` bound as a
type).  One generic carrier serves every width; `U8` ... `U256` abbreviate
the six Move widths. -/
structure UInt (W : Type) [Width W] where
  private mk ::
  val : Int
  nonneg : 0 ≤ val
  isLt : val < ((widthOf W).size : Int)

abbrev U8 := UInt W8
abbrev U16 := UInt W16
abbrev U32 := UInt W32
abbrev U64 := UInt W64
abbrev U128 := UInt W128
abbrev U256 := UInt W256

/-- A homogeneous Move vector: a list of elements certified to fit Move's
`u64` length domain, as every runtime vector does by construction. The
certificate is carried by the type, like the integer subtype bound; the
list payload exists only to give source programs an ordinary Lean type,
and Move supplies the runtime representation. -/
structure Vector (α : Type) where
  private mk ::
  private elems : List α
  private bounded : elems.length < MoveModel.IR.IntWidth.size .w64

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

namespace UInt

variable {W : Type} [Width W]

/-- Expose the mathematical value of a source integer for specifications. -/
def toNat (value : UInt W) : Nat := value.val.toNat

/-- The subtype carrier is the cast of its natural-number view. -/
theorem val_eq_toNat (value : UInt W) : value.val = (value.toNat : Int) :=
  (Int.toNat_of_nonneg value.nonneg).symm

/-- Source integers are determined by their mathematical value. -/
@[ext] theorem ext {left right : UInt W}
    (equal : left.toNat = right.toNat) : left = right := by
  cases left
  cases right
  simp only [toNat] at equal
  simp only [UInt.mk.injEq]
  omega

/-- The natural-number view is bounded by the width's range. -/
theorem toNat_lt (value : UInt W) : value.toNat < (widthOf W).size := by
  have hlt := value.isLt
  have h0 := value.nonneg
  simp only [toNat]
  omega

/-- Compiler-recognized integer literal, wrapping into the width's range;
checked semantics never constructs an out-of-range argument. Kept out of
line so LCNF preserves the named marker while remaining reducible for
source-level proofs. -/
@[noinline, nospecialize] def ofNat (n : Nat) : UInt W :=
  ⟨(n : Int) % ((widthOf W).size : Int),
   Int.emod_nonneg _ (by have := (widthOf W).size_pos; omega),
   Int.emod_lt_of_pos _ (by have := (widthOf W).size_pos; omega)⟩

@[simp] theorem toNat_ofNat (n : Nat) :
    (ofNat (W := W) n).toNat = n % (widthOf W).size := by
  have hpos : 0 < (widthOf W).size := (widthOf W).size_pos
  simp only [toNat, ofNat]
  omega

/-- The literal view without wrapping, available whenever the literal is in
range. -/
theorem toNat_ofNat_of_lt {n : Nat} (h : n < (widthOf W).size) :
    (ofNat (W := W) n).toNat = n := by
  rw [toNat_ofNat, Nat.mod_eq_of_lt h]

/-- Compiler-recognized Move arithmetic.  Move abort behavior is supplied
by the backend; the wrapping host values agree with the checked semantics
on every non-aborting execution. -/
@[noinline, nospecialize] def add : UInt W → UInt W → UInt W :=
  fun a b => ofNat (a.toNat + b.toNat)
@[noinline, nospecialize] def sub : UInt W → UInt W → UInt W :=
  fun a b => ofNat (a.toNat - b.toNat)
@[noinline, nospecialize] def mul : UInt W → UInt W → UInt W :=
  fun a b => ofNat (a.toNat * b.toNat)
@[noinline, nospecialize] def div : UInt W → UInt W → UInt W :=
  fun a b => ofNat (a.toNat / b.toNat)
@[noinline, nospecialize] def mod : UInt W → UInt W → UInt W :=
  fun a b => ofNat (a.toNat % b.toNat)

/-- Compiler-recognized Move bit operations. `land`, `lor`, and `lxor` are
width-preserving; shifts take a `u8` amount, truncate (`shl`) resp. drop
(`shr`) shifted-out bits, and abort in checked semantics when the amount
reaches the width. -/
@[noinline, nospecialize] def land : UInt W → UInt W → UInt W :=
  fun a b => ofNat (a.toNat &&& b.toNat)
@[noinline, nospecialize] def lor : UInt W → UInt W → UInt W :=
  fun a b => ofNat (a.toNat ||| b.toNat)
@[noinline, nospecialize] def lxor : UInt W → UInt W → UInt W :=
  fun a b => ofNat (a.toNat ^^^ b.toNat)
@[noinline, nospecialize] def shl : UInt W → UInt W8 → UInt W :=
  fun a k => ofNat (a.toNat <<< k.toNat)
@[noinline, nospecialize] def shr : UInt W → UInt W8 → UInt W :=
  fun a k => ofNat (a.toNat >>> k.toNat)

/-- Compiler-recognized integer cast (Move's `as`).  The host value wraps;
the checked semantics aborts whenever the value does not fit, so both agree
on non-aborting executions. The target width is directed by the expected
type: `(x.cast : U8)`. -/
@[noinline, nospecialize] def cast {W' : Type} [Width W'] (a : UInt W) : UInt W' :=
  ofNat a.toNat

/-- Compiler-recognized comparison, represented by a `Bool` so Lean's ordinary
`if` lowering exposes a direct case split in LCNF. -/
@[noinline, nospecialize] opaque less : UInt W → UInt W → Bool :=
  fun a b => decide (a.toNat < b.toNat)
@[noinline, nospecialize] opaque lessEq : UInt W → UInt W → Bool :=
  fun a b => decide (a.toNat ≤ b.toNat)
@[noinline, nospecialize] opaque equal : UInt W → UInt W → Bool :=
  fun a b => decide (a.toNat = b.toNat)

instance (n : Nat) : OfNat (UInt W) n := ⟨ofNat n⟩
instance : Inhabited (UInt W) := ⟨ofNat 0⟩
instance : DecidableEq (UInt W) := fun a b =>
  decidable_of_iff (a.toNat = b.toNat) ⟨ext, fun h => h ▸ rfl⟩
instance : Repr (UInt W) where
  reprPrec value prec := reprPrec value.toNat prec

/-- Expose the mathematical value of an integer numeral literal directly, so
proofs need no per-literal `rfl` facts. -/
@[simp] theorem toNat_ofNat_numeral (n : Nat) :
    (no_index (OfNat.ofNat n) : UInt W).toNat = n % (widthOf W).size :=
  toNat_ofNat n

/-! The operations whose results always stay in range collapse without side
conditions: the wrap in `ofNat` is invisible for them.  Stated on the
`ofNat`-of-view shapes the operation lemmas produce, at high priority so they
win over the generic `toNat_ofNat`. -/

@[simp high] theorem toNat_ofNat_sub (a b : UInt W) :
    (ofNat (a.toNat - b.toNat) : UInt W).toNat = a.toNat - b.toNat := by
  rw [toNat_ofNat, Nat.mod_eq_of_lt
    (Nat.lt_of_le_of_lt (Nat.sub_le _ _) a.toNat_lt)]

@[simp high] theorem toNat_ofNat_div (a : UInt W) (n : Nat) :
    (ofNat (a.toNat / n) : UInt W).toNat = a.toNat / n := by
  rw [toNat_ofNat, Nat.mod_eq_of_lt
    (Nat.lt_of_le_of_lt (Nat.div_le_self _ _) a.toNat_lt)]

@[simp high] theorem toNat_ofNat_mod (a : UInt W) (n : Nat) :
    (ofNat (a.toNat % n) : UInt W).toNat = a.toNat % n := by
  rw [toNat_ofNat, Nat.mod_eq_of_lt
    (Nat.lt_of_le_of_lt (Nat.mod_le _ _) a.toNat_lt)]

@[simp high] theorem toNat_ofNat_land (a b : UInt W) :
    (ofNat (a.toNat &&& b.toNat) : UInt W).toNat = a.toNat &&& b.toNat := by
  rw [toNat_ofNat, Nat.mod_eq_of_lt
    (Nat.lt_of_le_of_lt Nat.and_le_left a.toNat_lt)]

@[simp high] theorem toNat_ofNat_lor (a b : UInt W) :
    (ofNat (a.toNat ||| b.toNat) : UInt W).toNat = a.toNat ||| b.toNat := by
  have hlt : a.toNat ||| b.toNat < (widthOf W).size :=
    Nat.or_lt_two_pow a.toNat_lt b.toNat_lt
  rw [toNat_ofNat, Nat.mod_eq_of_lt hlt]

@[simp high] theorem toNat_ofNat_lxor (a b : UInt W) :
    (ofNat (a.toNat ^^^ b.toNat) : UInt W).toNat = a.toNat ^^^ b.toNat := by
  have hlt : a.toNat ^^^ b.toNat < (widthOf W).size :=
    Nat.xor_lt_two_pow a.toNat_lt b.toNat_lt
  rw [toNat_ofNat, Nat.mod_eq_of_lt hlt]

@[simp high] theorem toNat_ofNat_shr (a : UInt W) (n : Nat) :
    (ofNat (a.toNat >>> n) : UInt W).toNat = a.toNat >>> n := by
  rw [toNat_ofNat, Nat.mod_eq_of_lt
    (Nat.lt_of_le_of_lt (Nat.shiftRight_le _ _) a.toNat_lt)]

/-- Numeric literals are the literal constructor, definitionally. -/
theorem numeral_eq_ofNat (n : Nat) :
    (no_index (OfNat.ofNat n) : UInt W) = UInt.ofNat n := rfl

/-- Per-width literal views with the range as a numeral, giving decision
procedures ground arithmetic to work with. -/
theorem toNat_ofNat_u8 (n : Nat) :
    (ofNat n : U8).toNat = n % 256 := toNat_ofNat n
theorem toNat_ofNat_u16 (n : Nat) :
    (ofNat n : U16).toNat = n % 65536 := toNat_ofNat n
theorem toNat_ofNat_u32 (n : Nat) :
    (ofNat n : U32).toNat = n % 4294967296 := toNat_ofNat n
theorem toNat_ofNat_u64 (n : Nat) :
    (ofNat n : U64).toNat = n % 18446744073709551616 := toNat_ofNat n
theorem toNat_ofNat_u128 (n : Nat) :
    (ofNat n : U128).toNat =
      n % 340282366920938463463374607431768211456 := toNat_ofNat n
theorem toNat_ofNat_u256 (n : Nat) :
    (ofNat n : U256).toNat =
      n % 115792089237316195423570985008687907853269984665640564039457584007913129639936 :=
  toNat_ofNat n

/-- The two ubiquitous numerals are always in range. -/
@[simp] theorem toNat_zero : (0 : UInt W).toNat = 0 := by
  rw [toNat_ofNat_numeral, Nat.zero_mod]

@[simp] theorem toNat_one : (1 : UInt W).toNat = 1 := by
  rw [toNat_ofNat_numeral, Nat.mod_eq_of_lt (widthOf W).one_lt_size]

theorem eq_zero_of_not_pos {value : UInt W}
    (notPositive : ¬0 < value.toNat) : value = 0 := by
  apply ext
  rw [toNat_ofNat_numeral, Nat.zero_mod]
  omega

instance : Add (UInt W) := ⟨add⟩
instance : Sub (UInt W) := ⟨sub⟩
instance : Mul (UInt W) := ⟨mul⟩
instance : Div (UInt W) := ⟨div⟩
instance : Mod (UInt W) := ⟨mod⟩
instance : BEq (UInt W) := ⟨equal⟩
instance : AndOp (UInt W) := ⟨land⟩
instance : OrOp (UInt W) := ⟨lor⟩
instance : XorOp (UInt W) := ⟨lxor⟩
instance : HShiftLeft (UInt W) (UInt W8) (UInt W) := ⟨shl⟩
instance : HShiftRight (UInt W) (UInt W8) (UInt W) := ⟨shr⟩

@[simp] theorem add_eq_ofNat (a b : UInt W) :
    a + b = ofNat (a.toNat + b.toNat) := rfl

@[simp] theorem sub_eq_ofNat (a b : UInt W) :
    a - b = ofNat (a.toNat - b.toNat) := rfl

@[simp] theorem mul_eq_ofNat (a b : UInt W) :
    a * b = ofNat (a.toNat * b.toNat) := rfl

@[simp] theorem div_eq_ofNat (a b : UInt W) :
    a / b = ofNat (a.toNat / b.toNat) := rfl

@[simp] theorem mod_eq_ofNat (a b : UInt W) :
    a % b = ofNat (a.toNat % b.toNat) := rfl

@[simp] theorem land_eq_ofNat (a b : UInt W) :
    a &&& b = ofNat (a.toNat &&& b.toNat) := rfl

@[simp] theorem lor_eq_ofNat (a b : UInt W) :
    a ||| b = ofNat (a.toNat ||| b.toNat) := rfl

@[simp] theorem lxor_eq_ofNat (a b : UInt W) :
    a ^^^ b = ofNat (a.toNat ^^^ b.toNat) := rfl

@[simp] theorem shl_eq_ofNat (a : UInt W) (k : UInt W8) :
    a <<< k = ofNat (a.toNat <<< k.toNat) := rfl

@[simp] theorem shr_eq_ofNat (a : UInt W) (k : UInt W8) :
    a >>> k = ofNat (a.toNat >>> k.toNat) := rfl

@[simp] theorem toNat_cast {W' : Type} [Width W'] (a : UInt W) :
    (cast (W' := W') a).toNat = a.toNat % (widthOf W').size :=
  toNat_ofNat a.toNat

/-! The raw operation names appear in checked-semantics propositions; expose
the same `ofNat`-of-view equations for them as for the notations. -/

@[simp] theorem add_def (a b : UInt W) :
    add a b = ofNat (a.toNat + b.toNat) := rfl
@[simp] theorem sub_def (a b : UInt W) :
    sub a b = ofNat (a.toNat - b.toNat) := rfl
@[simp] theorem mul_def (a b : UInt W) :
    mul a b = ofNat (a.toNat * b.toNat) := rfl
@[simp] theorem div_def (a b : UInt W) :
    div a b = ofNat (a.toNat / b.toNat) := rfl
@[simp] theorem mod_def (a b : UInt W) :
    mod a b = ofNat (a.toNat % b.toNat) := rfl
@[simp] theorem land_def (a b : UInt W) :
    land a b = ofNat (a.toNat &&& b.toNat) := rfl
@[simp] theorem lor_def (a b : UInt W) :
    lor a b = ofNat (a.toNat ||| b.toNat) := rfl
@[simp] theorem lxor_def (a b : UInt W) :
    lxor a b = ofNat (a.toNat ^^^ b.toNat) := rfl
@[simp] theorem shl_def (a : UInt W) (k : UInt W8) :
    shl a k = ofNat (a.toNat <<< k.toNat) := rfl
@[simp] theorem shr_def (a : UInt W) (k : UInt W8) :
    shr a k = ofNat (a.toNat >>> k.toNat) := rfl
@[simp] theorem cast_def {W' : Type} [Width W'] (a : UInt W) :
    (cast a : UInt W') = ofNat a.toNat := rfl

/-- `<` remains ordinary Lean syntax.  Its decision procedure is the named
`UInt.less` primitive, which the backend lowers to `MoveModel.IR.Oper.lt`. -/
instance : LT (UInt W) := ⟨fun a b => less a b = true⟩
instance (a b : UInt W) : Decidable (a < b) :=
  inferInstanceAs (Decidable (less a b = true))

end UInt

/-- Resolve the width of each tag for specification normalization, in both
the `widthOf` and the raw instance-projection spelling. -/
@[simp] theorem widthOf_W8 : widthOf W8 = .w8 := rfl
@[simp] theorem widthOf_W16 : widthOf W16 = .w16 := rfl
@[simp] theorem widthOf_W32 : widthOf W32 = .w32 := rfl
@[simp] theorem widthOf_W64 : widthOf W64 = .w64 := rfl
@[simp] theorem widthOf_W128 : widthOf W128 = .w128 := rfl
@[simp] theorem widthOf_W256 : widthOf W256 = .w256 := rfl

@[simp] theorem width_W8 : Width.width W8 = .w8 := rfl
@[simp] theorem width_W16 : Width.width W16 = .w16 := rfl
@[simp] theorem width_W32 : Width.width W32 = .w32 := rfl
@[simp] theorem width_W64 : Width.width W64 = .w64 := rfl
@[simp] theorem width_W128 : Width.width W128 = .w128 := rfl
@[simp] theorem width_W256 : Width.width W256 = .w256 := rfl

/-- Width-directed literal constructors, as named specification surface. -/
abbrev U8.ofNat : Nat → U8 := UInt.ofNat
abbrev U16.ofNat : Nat → U16 := UInt.ofNat
abbrev U32.ofNat : Nat → U32 := UInt.ofNat
abbrev U64.ofNat : Nat → U64 := UInt.ofNat
abbrev U128.ofNat : Nat → U128 := UInt.ofNat
abbrev U256.ofNat : Nat → U256 := UInt.ofNat

/-- The exclusive upper bounds of the Move integer value ranges, as named
specification constants. -/
abbrev U8.size : Nat := MoveModel.IR.IntWidth.size .w8
abbrev U16.size : Nat := MoveModel.IR.IntWidth.size .w16
abbrev U32.size : Nat := MoveModel.IR.IntWidth.size .w32
abbrev U64.size : Nat := MoveModel.IR.IntWidth.size .w64
abbrev U128.size : Nat := MoveModel.IR.IntWidth.size .w128
abbrev U256.size : Nat := MoveModel.IR.IntWidth.size .w256

namespace Vector

@[noinline] def empty : Vector α :=
  ⟨[], MoveModel.IR.IntWidth.size_pos _⟩

/-- Push is total on the certified type: at the (unreachable without an
abort) maximum length it leaves the vector unchanged, agreeing with the
checked semantics on every non-aborting execution. -/
@[noinline] def push : Vector α → α → Vector α :=
  fun values value =>
    if grows : values.elems.length + 1 < MoveModel.IR.IntWidth.size .w64 then
      ⟨values.elems ++ [value], by simpa using grows⟩
    else
      values
@[noinline] def length : Vector α → U64 :=
  fun values => UInt.ofNat values.elems.length
@[noinline] def get [Inhabited α] : Vector α → U64 → α :=
  fun values index => values.elems[index.toNat]?.getD Inhabited.default
@[noinline] def set : Vector α → U64 → α → Vector α :=
  fun values index value =>
    ⟨values.elems.set index.toNat value, by simpa using values.bounded⟩

instance : Inhabited (Vector α) := ⟨empty⟩

/-- Logical contents of a source vector. This is a specification accessor and
is never selected for Move lowering. -/
def toList (values : Vector α) : List α := values.elems

/-- Construct a logical source vector from a list.  This is a verification
helper, not a compiler primitive; deployable source builds vectors with the
ordinary vector operations. -/
def ofList (values : List α)
    (bounded : values.length < MoveModel.IR.IntWidth.size .w64 := by
      decide) : Vector α :=
  ⟨values, bounded⟩

/-- Every vector fits Move's `u64` length domain, by construction. -/
theorem toList_length_lt (values : Vector α) :
    values.toList.length < U64.size := values.bounded

/-- Source vectors are determined by their logical contents. -/
@[ext] theorem ext {left right : Vector α}
    (equal : left.toList = right.toList) : left = right := by
  cases left
  cases right
  simp only [toList] at equal
  subst equal
  rfl

@[simp] theorem toList_mk (values : List α)
    (bounded : values.length < MoveModel.IR.IntWidth.size .w64) :
    (⟨values, bounded⟩ : Vector α).toList = values := rfl

@[simp] theorem toList_empty : (empty : Vector α).toList = [] := rfl

@[simp] theorem toList_push (values : Vector α) (value : α)
    (grows : values.toList.length + 1 < U64.size) :
    (push values value).toList = values.toList ++ [value] := by
  have grows' : values.elems.length + 1 < MoveModel.IR.IntWidth.size .w64 :=
    grows
  rw [push, dif_pos grows']
  rfl

@[simp] theorem length_toNat (values : Vector α) :
    (length values).toNat = values.toList.length := by
  have bounded : values.elems.length < (widthOf W64).size := values.bounded
  rw [length, UInt.toNat_ofNat, Nat.mod_eq_of_lt bounded]
  rfl

@[simp] theorem toList_set (values : Vector α) (index : U64) (value : α) :
    (set values index value).toList = values.toList.set index.toNat value := rfl

@[simp] theorem toList_ofList (values : List α)
    (bounded : values.length < MoveModel.IR.IntWidth.size .w64) :
    (ofList values bounded).toList = values := rfl

end Vector

end Move
