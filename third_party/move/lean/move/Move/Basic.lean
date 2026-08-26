-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Value

/-!
# Leaner source types

Opaque source-level types recognized by the Lean-to-Move compiler.  Their Lean
implementations are not the runtime representation used by MoveModel.
-/

namespace Move

export MoveModel.IR (IntWidth NumType)

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
  deriving Inhabited, DecidableEq

namespace Address

/-- Compiler-recognized account-address literal.  The backend checks the
payload fits Move's 256-bit address domain. -/
@[noinline] def ofNat (value : Nat) : Address := ⟨value⟩

end Address

/-- Move's `@0x1` address-literal spelling. -/
scoped macro:max (name := addressLiteral) (priority := high) "@" value:num : term =>
  `(Address.ofNat $value)

/-- A Move signer value. The account address is represented explicitly so
source verification and `moveTo` share exactly the same publication address. -/
structure Signer where
  private mk ::
  address : Address
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

/-- Type-level names for signedness, alongside the width tags: the Lean
compiler's intermediate representation erases value indices, so signedness —
like width — is carried as a type. -/
structure Unsigned where private mk ::
structure Signed where private mk ::

/-- The signedness named by a sign-tag type. -/
class Sign (S : Type) where
  signed : Bool

instance : Sign Unsigned := ⟨false⟩
instance : Sign Signed := ⟨true⟩

/-- The signedness of a sign-tag type. -/
abbrev signOf (S : Type) [Sign S] : Bool := Sign.signed S

/-- The numeric type named by a sign tag and a width tag: the pair of bounds
every operation consults. -/
abbrev numTypeOf (S W : Type) [Sign S] [Width W] : NumType := ⟨widthOf W, signOf S⟩

/-- Resolve the signedness of each tag, and with it the bounds each operation
consults: unsigned is `[0, size)`, signed the two's-complement
`[-halfSize, halfSize)`.  These are the only places signedness is decided. -/
@[simp] theorem signOf_unsigned : signOf Unsigned = false := rfl
@[simp] theorem signOf_signed : signOf Signed = true := rfl

section Bounds
variable {W : Type} [Width W]

@[simp] theorem unsigned_lo : (numTypeOf Unsigned W).lo = 0 := rfl
@[simp] theorem unsigned_hi : (numTypeOf Unsigned W).hi = ((widthOf W).size : Int) := rfl
@[simp] theorem unsigned_size : (numTypeOf Unsigned W).size = (widthOf W).size := rfl
@[simp] theorem signed_lo :
    (numTypeOf Signed W).lo = -((widthOf W).halfSize : Int) := rfl
@[simp] theorem signed_hi :
    (numTypeOf Signed W).hi = ((widthOf W).halfSize : Int) := rfl
@[simp] theorem signed_size : (numTypeOf Signed W).size = (widthOf W).size := rfl

end Bounds

/-- A Move integer of the signedness named by `S` and the width named by `W`:
the subtype of unbounded integers within that type's range
(`{ x : Int // lo ≤ x ∧ x < hi }`, the model's `IsValid` bound as a type).
Signed and unsigned integers are one carrier differing only in `lo`/`hi`:
unsigned is `[0, 2^bits)`, signed the two's-complement `[-2^(bits-1),
2^(bits-1))`.  `U8` ... `U256` and `I8` ... `I256` abbreviate the twelve Move
integer types. -/
structure MoveInt (S W : Type) [Sign S] [Width W] where
  private mk ::
  val : Int
  isGe : (numTypeOf S W).lo ≤ val
  isLt : val < (numTypeOf S W).hi

/-- Move's unsigned resp. signed integers of a given width. -/
abbrev UInt (W : Type) [Width W] := MoveInt Unsigned W
abbrev SInt (W : Type) [Width W] := MoveInt Signed W

abbrev U8 := UInt W8
abbrev U16 := UInt W16
abbrev U32 := UInt W32
abbrev U64 := UInt W64
abbrev U128 := UInt W128
abbrev U256 := UInt W256

abbrev I8 := SInt W8
abbrev I16 := SInt W16
abbrev I32 := SInt W32
abbrev I64 := SInt W64
abbrev I128 := SInt W128
abbrev I256 := SInt W256

/-- The logical domain in which a Move source value is specified.  Types whose
host representation is already their logical domain use the identity instance;
integers, vectors, and references refine that default below. -/
class ModelDomain (α : Type u) (β : outParam (Type v)) where
  project : α → β

/-- Project a Move source value into its logical specification domain. -/
def model (value : α) [ModelDomain α β] : β :=
  ModelDomain.project value

/-- Source values without a more specific model retain their host domain.  This
covers booleans, addresses, signers, and user-defined Move structures; a user
can provide a higher-priority instance when a structure has its own model. -/
instance (priority := low) : ModelDomain α α where
  project := id

/-- ASCII and Unicode postfix spellings for projecting into the model domain.
`value^` is convenient in ASCII-only source (write `((value)^)` when followed
by another operator, to disambiguate Lean's infix exponentiation); `value↑` is
its Unicode alias. -/
scoped macro:max value:term:max noWs "^" : term => `(model $value)
scoped macro:max value:term:max noWs "↑" : term => `(model $value)

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

/-- Address equality in Move source is bytecode structural equality.  Give it
an exact instance so Lean does not synthesize equality through `DecidableEq`,
whose implementation is intentionally outside the Move source language. -/
instance : BEq Address := Compare.genericBEq

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

/-- `signer::address_of` on the reference Move actually passes.  A signer is
only ever used through `&signer`, so this is the spelling contracts see, and
dot notation (`account.address`) resolves to it. -/
def address (signer : Ref Signer) : Address := signer.value.address

instance [Inhabited α] : Inhabited (Ref α) := ⟨default⟩

end Ref

namespace MutRef

@[noinline] opaque default [Inhabited α] : MutRef α :=
  ⟨(Inhabited.default : α), (Inhabited.default : α)⟩
@[noinline] opaque ofValue (value : α) : MutRef α := ⟨value, value⟩
@[noinline] opaque get (r : MutRef α) : α := r.current

instance [Inhabited α] : Inhabited (MutRef α) := ⟨default⟩

end MutRef

/-- References are specified by the logical value they observe. -/
instance [ModelDomain α β] : ModelDomain (Ref α) β where
  project ref := model ref.get

/-- Mutable references expose their current logical value in specifications. -/
instance [ModelDomain α β] : ModelDomain (MutRef α) β where
  project ref := model ref.get

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

namespace MoveInt

variable {S W : Type} [Sign S] [Width W]

/-- Expose the mathematical value of a source integer for specifications.  For
signed integers this *is* the value one reasons about; for unsigned integers
`UInt.toNat` gives the natural-number view. -/
def toInt (value : MoveInt S W) : Int := value.val

theorem lo_le_toInt (value : MoveInt S W) : (numTypeOf S W).lo ≤ value.toInt :=
  value.isGe
theorem toInt_lt_hi (value : MoveInt S W) : value.toInt < (numTypeOf S W).hi :=
  value.isLt

/-- Source integers are determined by their mathematical value. -/
@[ext] theorem ext {left right : MoveInt S W} (equal : left.toInt = right.toInt) :
    left = right := by
  cases left; cases right
  simp only [toInt] at equal
  simp only [MoveInt.mk.injEq]
  exact equal

/-- Compiler-recognized integer literal, wrapping into the type's range;
checked semantics never constructs an out-of-range argument.  Kept out of line
so LCNF preserves the named marker while remaining reducible for source-level
proofs. -/
@[noinline, nospecialize] def ofInt (n : Int) : MoveInt S W :=
  ⟨(numTypeOf S W).wrap n,
   ((numTypeOf S W).wrap_mem n).1, ((numTypeOf S W).wrap_mem n).2⟩

@[simp] theorem toInt_ofInt (n : Int) :
    (ofInt (S := S) (W := W) n).toInt = (numTypeOf S W).wrap n := rfl

/-- The literal view without wrapping, available whenever it is in range. -/
theorem toInt_ofInt_of_mem {n : Int} (hlo : (numTypeOf S W).lo ≤ n)
    (hhi : n < (numTypeOf S W).hi) : (ofInt (S := S) (W := W) n).toInt = n := by
  rw [toInt_ofInt, (numTypeOf S W).wrap_of_mem hlo hhi]

/-- The unsigned (natural-number) view of a source integer.  Faithful exactly
when the type is unsigned, where the carrier is nonnegative. -/
def toNat (value : MoveInt S W) : Nat := value.val.toNat

/-- Compiler-recognized Move arithmetic.  Move abort behavior is supplied by
the backend; the wrapping host values agree with the checked semantics on
every non-aborting execution.  Division truncates toward zero, matching Move
(and agreeing with natural-number division on unsigned operands). -/
@[noinline, nospecialize] def add : MoveInt S W → MoveInt S W → MoveInt S W :=
  fun a b => ofInt (a.toInt + b.toInt)
@[noinline, nospecialize] def sub : MoveInt S W → MoveInt S W → MoveInt S W :=
  fun a b => ofInt (a.toInt - b.toInt)
@[noinline, nospecialize] def mul : MoveInt S W → MoveInt S W → MoveInt S W :=
  fun a b => ofInt (a.toInt * b.toInt)
@[noinline, nospecialize] def div : MoveInt S W → MoveInt S W → MoveInt S W :=
  fun a b => ofInt (a.toInt.tdiv b.toInt)
@[noinline, nospecialize] def mod : MoveInt S W → MoveInt S W → MoveInt S W :=
  fun a b => ofInt (a.toInt.tmod b.toInt)
@[noinline, nospecialize] def neg : MoveInt S W → MoveInt S W :=
  fun a => ofInt (-a.toInt)

/-- Compiler-recognized Move bit operations, on the type's two's-complement
bit pattern (the identity for unsigned types).  Shifts take a `u8` amount,
truncate (`shl`) resp. drop (`shr`) shifted-out bits, and abort in checked
semantics when the amount reaches the width; `shr` is arithmetic, i.e. floor
division by `2 ^ k`. -/
@[noinline, nospecialize] def land : MoveInt S W → MoveInt S W → MoveInt S W :=
  fun a b => ofInt ((numTypeOf S W).fromBits
    (((numTypeOf S W).toBits a.toInt).toNat &&& ((numTypeOf S W).toBits b.toInt).toNat))
@[noinline, nospecialize] def lor : MoveInt S W → MoveInt S W → MoveInt S W :=
  fun a b => ofInt ((numTypeOf S W).fromBits
    (((numTypeOf S W).toBits a.toInt).toNat ||| ((numTypeOf S W).toBits b.toInt).toNat))
@[noinline, nospecialize] def lxor : MoveInt S W → MoveInt S W → MoveInt S W :=
  fun a b => ofInt ((numTypeOf S W).fromBits
    (((numTypeOf S W).toBits a.toInt).toNat ^^^ ((numTypeOf S W).toBits b.toInt).toNat))
@[noinline, nospecialize] def shl : MoveInt S W → UInt W8 → MoveInt S W :=
  fun a k => ofInt ((numTypeOf S W).fromBits
    ((((numTypeOf S W).toBits a.toInt).toNat <<< k.toNat) % (numTypeOf S W).size))
@[noinline, nospecialize] def shr : MoveInt S W → UInt W8 → MoveInt S W :=
  fun a k => ofInt (a.toInt.fdiv (2 ^ k.toNat))

/-- Compiler-recognized integer cast (Move's `as`).  The host value wraps; the
checked semantics aborts whenever the value does not fit the target range, so
both agree on non-aborting executions.  The target is directed by the expected
type: `(x.cast : U8)`.  Cross-sign casts are ordinary casts — the target's
bounds decide. -/
@[noinline, nospecialize] def cast {S' W' : Type} [Sign S'] [Width W']
    (a : MoveInt S W) : MoveInt S' W' :=
  ofInt a.toInt

/-- Compiler-recognized comparison, represented by a `Bool` so Lean's ordinary
`if` lowering exposes a direct case split in LCNF.  Ordering is on the
mathematical value, hence correct for both signednesses. -/
@[noinline, nospecialize] opaque less : MoveInt S W → MoveInt S W → Bool :=
  fun a b => decide (a.toInt < b.toInt)
@[noinline, nospecialize] opaque lessEq : MoveInt S W → MoveInt S W → Bool :=
  fun a b => decide (a.toInt ≤ b.toInt)
@[noinline, nospecialize] opaque equal : MoveInt S W → MoveInt S W → Bool :=
  fun a b => decide (a.toInt = b.toInt)

instance (n : Nat) : OfNat (MoveInt S W) n := ⟨ofInt (n : Int)⟩
instance : Neg (MoveInt S W) := ⟨neg⟩
instance : Inhabited (MoveInt S W) := ⟨ofInt 0⟩
instance : DecidableEq (MoveInt S W) := fun a b =>
  decidable_of_iff (a.toInt = b.toInt) ⟨ext, fun h => h ▸ rfl⟩
instance : Repr (MoveInt S W) where
  reprPrec value prec := reprPrec value.toInt prec

instance : Add (MoveInt S W) := ⟨add⟩
instance : Sub (MoveInt S W) := ⟨sub⟩
instance : Mul (MoveInt S W) := ⟨mul⟩
instance : Div (MoveInt S W) := ⟨div⟩
instance : Mod (MoveInt S W) := ⟨mod⟩
instance : BEq (MoveInt S W) := ⟨equal⟩
instance : AndOp (MoveInt S W) := ⟨land⟩
instance : OrOp (MoveInt S W) := ⟨lor⟩
instance : XorOp (MoveInt S W) := ⟨lxor⟩
instance : HShiftLeft (MoveInt S W) (UInt W8) (MoveInt S W) := ⟨shl⟩
instance : HShiftRight (MoveInt S W) (UInt W8) (MoveInt S W) := ⟨shr⟩

/-- Zero and one are in every type's range. -/
theorem zero_mem : (numTypeOf S W).lo ≤ (0 : Int) ∧ (0 : Int) < (numTypeOf S W).hi :=
  ⟨(numTypeOf S W).lo_nonpos, (numTypeOf S W).pos_lt_hi⟩

theorem one_lt_hi : (1 : Int) < (numTypeOf S W).hi := by
  have hw : 1 < (widthOf W).halfSize := (widthOf W).one_lt_halfSize
  have hz : 1 < (widthOf W).size := (widthOf W).one_lt_size
  show (1 : Int) < MoveModel.IR.NumType.hi _
  simp only [MoveModel.IR.NumType.hi]
  split
  · exact_mod_cast hw
  · exact_mod_cast hz

theorem toInt_zero : (0 : MoveInt S W).toInt = 0 := by
  show (ofInt (0 : Int)).toInt = 0
  exact toInt_ofInt_of_mem zero_mem.1 zero_mem.2

theorem toInt_one : (1 : MoveInt S W).toInt = 1 := by
  show (ofInt (1 : Int)).toInt = 1
  exact toInt_ofInt_of_mem (by have := (numTypeOf S W).lo_nonpos; omega) one_lt_hi

@[simp] theorem toInt_default : (default : MoveInt S W).toInt = 0 := by
  show (ofInt (0 : Int)).toInt = 0
  exact toInt_ofInt_of_mem zero_mem.1 zero_mem.2

/-- Expose the value of an integer numeral literal directly. -/
theorem toInt_ofNat_numeral (n : Nat) :
    (no_index (OfNat.ofNat n) : MoveInt S W).toInt = (numTypeOf S W).wrap (n : Int) := rfl

/-- Numeric literals are the literal constructor, definitionally. -/
theorem numeral_eq_ofInt (n : Nat) :
    (no_index (OfNat.ofNat n) : MoveInt S W) = ofInt (n : Int) := rfl

@[simp] theorem add_eq_ofInt (a b : MoveInt S W) :
    a + b = ofInt (a.toInt + b.toInt) := rfl
@[simp] theorem sub_eq_ofInt (a b : MoveInt S W) :
    a - b = ofInt (a.toInt - b.toInt) := rfl
@[simp] theorem mul_eq_ofInt (a b : MoveInt S W) :
    a * b = ofInt (a.toInt * b.toInt) := rfl
@[simp] theorem div_eq_ofInt (a b : MoveInt S W) :
    a / b = ofInt (a.toInt.tdiv b.toInt) := rfl
@[simp] theorem mod_eq_ofInt (a b : MoveInt S W) :
    a % b = ofInt (a.toInt.tmod b.toInt) := rfl
@[simp] theorem neg_eq_ofInt (a : MoveInt S W) :
    -a = ofInt (-a.toInt) := rfl

/-! The raw operation names appear in checked-semantics propositions; expose
the same `ofInt`-of-view equations for them as for the notations. -/

@[simp] theorem add_def (a b : MoveInt S W) :
    add a b = ofInt (a.toInt + b.toInt) := rfl
@[simp] theorem sub_def (a b : MoveInt S W) :
    sub a b = ofInt (a.toInt - b.toInt) := rfl
@[simp] theorem mul_def (a b : MoveInt S W) :
    mul a b = ofInt (a.toInt * b.toInt) := rfl
@[simp] theorem div_def (a b : MoveInt S W) :
    div a b = ofInt (a.toInt.tdiv b.toInt) := rfl
@[simp] theorem mod_def (a b : MoveInt S W) :
    mod a b = ofInt (a.toInt.tmod b.toInt) := rfl
@[simp] theorem neg_def (a : MoveInt S W) : neg a = ofInt (-a.toInt) := rfl
@[simp] theorem cast_def {S' W' : Type} [Sign S'] [Width W'] (a : MoveInt S W) :
    (cast a : MoveInt S' W') = ofInt a.toInt := rfl

/-- `<` remains ordinary Lean syntax.  Its decision procedure is the named
`MoveInt.less` primitive, which the backend lowers to
`MoveModel.IR.Oper.lt`. -/
instance : LT (MoveInt S W) := ⟨fun a b => less a b = true⟩
instance (a b : MoveInt S W) : Decidable (a < b) :=
  inferInstanceAs (Decidable (less a b = true))

/-- Trust base: the integer comparison primitive the backend lowers to
`MoveModel.IR.Oper.lt` is numeric on the mathematical value.  This is the
integer-specific counterpart of the verification axiom `logicalLT_uint` for
the generic structural order; both state what the compiler implements. -/
axiom less_eq_true_iff (a b : MoveInt S W) : less a b = true ↔ a.toInt < b.toInt

theorem lt_iff_toInt_lt (a b : MoveInt S W) : a < b ↔ a.toInt < b.toInt :=
  less_eq_true_iff a b

/-- `≤` mirrors `<`. -/
instance : LE (MoveInt S W) := ⟨fun a b => lessEq a b = true⟩
instance (a b : MoveInt S W) : Decidable (a ≤ b) :=
  inferInstanceAs (Decidable (lessEq a b = true))

/-- Trust base: the `≤` primitive is numeric (companion to
`less_eq_true_iff`). -/
axiom lessEq_eq_true_iff (a b : MoveInt S W) : lessEq a b = true ↔ a.toInt ≤ b.toInt

theorem le_iff_toInt_le (a b : MoveInt S W) : a ≤ b ↔ a.toInt ≤ b.toInt :=
  lessEq_eq_true_iff a b

end MoveInt

/-- Unsigned Move integers are specified as natural numbers. -/
instance [Width W] : ModelDomain (UInt W) Nat where
  project := MoveInt.toNat

@[simp] theorem model_uint [Width W] (value : UInt W) : model value = value.toNat := rfl

/-- Signed Move integers are specified as mathematical integers. -/
instance [Width W] : ModelDomain (SInt W) Int where
  project := MoveInt.toInt

@[simp] theorem model_sint [Width W] (value : SInt W) : model value = value.toInt := rfl

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



/-! ## Unsigned view

Unsigned Move integers are `MoveInt Unsigned W`.  Their range is `[0, size)`,
so the carrier is nonnegative and the natural-number view `toNat` is faithful;
specifications are written in it.  The lemmas below are the unsigned instances
of the generic operations, phrased in that view. -/

namespace UInt

variable {W : Type} [Width W]

/-- The carrier of an unsigned integer is nonnegative. -/
theorem nonneg (value : UInt W) : 0 ≤ value.toInt := by
  have h := value.isGe
  simpa [MoveInt.toInt] using h

/-- The subtype carrier is the cast of its natural-number view. -/
theorem toInt_eq_toNat (value : UInt W) : value.toInt = (value.toNat : Int) :=
  (Int.toNat_of_nonneg value.nonneg).symm

theorem val_eq_toNat (value : UInt W) : value.val = (value.toNat : Int) :=
  toInt_eq_toNat value

/-- Unsigned integers are determined by their natural-number view. -/
@[ext] theorem ext {left right : UInt W} (equal : left.toNat = right.toNat) :
    left = right :=
  MoveInt.ext (by rw [toInt_eq_toNat, toInt_eq_toNat, equal])

/-- The natural-number view is bounded by the width's range. -/
theorem toNat_lt (value : UInt W) : value.toNat < (widthOf W).size := by
  have hlt := value.isLt
  have h0 : (0 : Int) ≤ value.val := value.nonneg
  simp only [unsigned_hi] at hlt
  simp only [MoveInt.toNat]
  omega

/-- Compiler-recognized unsigned literal, wrapping into the width's range. -/
@[noinline, nospecialize] def ofNat (n : Nat) : UInt W := MoveInt.ofInt (n : Int)

theorem ofNat_eq_ofInt (n : Nat) : (ofNat n : UInt W) = MoveInt.ofInt (n : Int) := rfl

/-- Unsigned wrapping is `mod size` on the natural-number view. -/
@[simp] theorem toNat_ofNat (n : Nat) :
    (ofNat (W := W) n).toNat = n % (widthOf W).size := by
  have hpos : (0 : Int) < ((widthOf W).size : Int) := by
    exact_mod_cast (widthOf W).size_pos
  show ((numTypeOf Unsigned W).wrap (n : Int)).toNat = _
  simp only [MoveModel.IR.NumType.wrap, unsigned_lo, unsigned_size,
    Int.sub_zero, Int.zero_add]
  omega

/-- The literal view without wrapping, available whenever it is in range. -/
theorem toNat_ofNat_of_lt {n : Nat} (h : n < (widthOf W).size) :
    (ofNat (W := W) n).toNat = n := by
  rw [toNat_ofNat, Nat.mod_eq_of_lt h]

/-- The unsigned operations, as the generic ones at `Unsigned`. -/
abbrev add : UInt W → UInt W → UInt W := MoveInt.add
abbrev sub : UInt W → UInt W → UInt W := MoveInt.sub
abbrev mul : UInt W → UInt W → UInt W := MoveInt.mul
abbrev div : UInt W → UInt W → UInt W := MoveInt.div
abbrev mod : UInt W → UInt W → UInt W := MoveInt.mod
abbrev land : UInt W → UInt W → UInt W := MoveInt.land
abbrev lor : UInt W → UInt W → UInt W := MoveInt.lor
abbrev lxor : UInt W → UInt W → UInt W := MoveInt.lxor
abbrev shl : UInt W → UInt W8 → UInt W := MoveInt.shl
abbrev shr : UInt W → UInt W8 → UInt W := MoveInt.shr
abbrev cast {W' : Type} [Width W'] (a : UInt W) : UInt W' := MoveInt.cast a
abbrev less : UInt W → UInt W → Bool := MoveInt.less
abbrev lessEq : UInt W → UInt W → Bool := MoveInt.lessEq
abbrev equal : UInt W → UInt W → Bool := MoveInt.equal

/-- The unsigned bit pattern is the value itself. -/
@[simp] theorem toBits_toInt (a : UInt W) :
    (numTypeOf Unsigned W).toBits a.toInt = a.toInt := by
  have h0 : (0 : Int) ≤ a.val := a.nonneg
  have hlt := a.isLt
  simp only [unsigned_hi] at hlt
  simp only [MoveModel.IR.NumType.toBits, unsigned_size, MoveInt.toInt]
  exact Int.emod_eq_of_lt h0 hlt

/-- Reinterpreting an in-range unsigned pattern is the identity. -/
theorem fromBits_of_lt {u : Int} (h0 : 0 ≤ u) (h : u < ((widthOf W).size : Int)) :
    (numTypeOf Unsigned W).fromBits u = u := by
  simp only [MoveModel.IR.NumType.fromBits, unsigned_hi, unsigned_size]
  omega

/-- The default integer is zero, which a data invariant's inhabitant proof
needs to see. -/
@[simp] theorem toNat_default : (default : UInt W).toNat = 0 := by
  show (MoveInt.toInt (default : UInt W)).toNat = 0
  rw [MoveInt.toInt_default]; rfl

/-- Expose the mathematical value of an integer numeral literal directly, so
proofs need no per-literal `rfl` facts. -/
@[simp] theorem toNat_ofNat_numeral (n : Nat) :
    (no_index (OfNat.ofNat n) : UInt W).toNat = n % (widthOf W).size :=
  toNat_ofNat n

/-- The `Int` view of an unsigned numeral, in the natural-number spelling the
unsigned lemmas use (at high priority, over the generic `wrap` form). -/
theorem toInt_ofNat_numeral (n : Nat) :
    (no_index (OfNat.ofNat n) : UInt W).toInt = ((n % (widthOf W).size : Nat) : Int) := by
  rw [toInt_eq_toNat, toNat_ofNat_numeral]

/-- Numeric literals are the literal constructor, definitionally. -/
theorem numeral_eq_ofNat (n : Nat) :
    (no_index (OfNat.ofNat n) : UInt W) = ofNat n := rfl

/-- The same identity read the other way: `ofNat` of a literal *is* that
literal.  Neither orientation can be a simp lemma — one side is always a
discrimination-tree wildcard, and left-to-right this one would also fire on the
compound `ofNat (a.toNat + b.toNat)` shapes the view lemmas deliberately
produce.  The `Move.UInt.ofNatLit` simproc applies it to literal arguments
only, which is where the two spellings actually meet. -/
theorem ofNat_eq_numeral (n : Nat) : (ofNat n : UInt W) = OfNat.ofNat n := rfl

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

/-! The unsigned instances of the generic operations, in the natural-number
view: each is the generic `ofInt` form with the carrier's nonnegativity
discharged. -/

theorem ofInt_natCast (n : Nat) : MoveInt.ofInt (n : Int) = (ofNat n : UInt W) := rfl

@[simp high] theorem add_eq_ofNat (a b : UInt W) :
    a + b = ofNat (a.toNat + b.toNat) := by
  show MoveInt.ofInt (a.toInt + b.toInt) = _
  rw [toInt_eq_toNat, toInt_eq_toNat, ← Int.natCast_add, ofInt_natCast]

/-- Unsigned subtraction, in the natural-number view.  The hypothesis is the
non-underflow condition: below it the host marker wraps (as the model does),
while `Nat` subtraction truncates, so the two agree exactly where checked
semantics does not abort. -/
@[simp] theorem sub_eq_ofNat (a b : UInt W) (h : b.toNat ≤ a.toNat) :
    a - b = ofNat (a.toNat - b.toNat) := by
  show MoveInt.ofInt (a.toInt - b.toInt) = _
  rw [toInt_eq_toNat, toInt_eq_toNat, ← ofInt_natCast]
  congr 1
  omega

@[simp high] theorem mul_eq_ofNat (a b : UInt W) :
    a * b = ofNat (a.toNat * b.toNat) := by
  show MoveInt.ofInt (a.toInt * b.toInt) = _
  rw [toInt_eq_toNat, toInt_eq_toNat, ← Int.natCast_mul, ofInt_natCast]

@[simp high] theorem div_eq_ofNat (a b : UInt W) :
    a / b = ofNat (a.toNat / b.toNat) := by
  show MoveInt.ofInt (a.toInt.tdiv b.toInt) = _
  rw [toInt_eq_toNat, toInt_eq_toNat, ← Int.ofNat_tdiv, ofInt_natCast]

@[simp high] theorem mod_eq_ofNat (a b : UInt W) :
    a % b = ofNat (a.toNat % b.toNat) := by
  show MoveInt.ofInt (a.toInt.tmod b.toInt) = _
  rw [toInt_eq_toNat, toInt_eq_toNat, ← Int.ofNat_tmod, ofInt_natCast]

@[simp] theorem land_eq_ofNat (a b : UInt W) :
    a &&& b = ofNat (a.toNat &&& b.toNat) := by
  show MoveInt.ofInt _ = _
  rw [toBits_toInt, toBits_toInt, toInt_eq_toNat, toInt_eq_toNat]
  simp only [Int.toNat_natCast]
  rw [fromBits_of_lt (Int.natCast_nonneg _)
    (by exact_mod_cast Nat.lt_of_le_of_lt Nat.and_le_left a.toNat_lt), ofInt_natCast]

@[simp] theorem lor_eq_ofNat (a b : UInt W) :
    a ||| b = ofNat (a.toNat ||| b.toNat) := by
  show MoveInt.ofInt _ = _
  rw [toBits_toInt, toBits_toInt, toInt_eq_toNat, toInt_eq_toNat]
  simp only [Int.toNat_natCast]
  rw [fromBits_of_lt (Int.natCast_nonneg _)
    (by exact_mod_cast Nat.or_lt_two_pow a.toNat_lt b.toNat_lt), ofInt_natCast]

@[simp] theorem lxor_eq_ofNat (a b : UInt W) :
    a ^^^ b = ofNat (a.toNat ^^^ b.toNat) := by
  show MoveInt.ofInt _ = _
  rw [toBits_toInt, toBits_toInt, toInt_eq_toNat, toInt_eq_toNat]
  simp only [Int.toNat_natCast]
  rw [fromBits_of_lt (Int.natCast_nonneg _)
    (by exact_mod_cast Nat.xor_lt_two_pow a.toNat_lt b.toNat_lt), ofInt_natCast]

@[simp] theorem shl_eq_ofNat (a : UInt W) (k : UInt W8) :
    a <<< k = ofNat ((a.toNat <<< k.toNat) % (widthOf W).size) := by
  show MoveInt.ofInt _ = _
  rw [toBits_toInt, toInt_eq_toNat]
  simp only [Int.toNat_natCast, unsigned_size]
  rw [← Int.natCast_emod, fromBits_of_lt (Int.natCast_nonneg _)
    (by exact_mod_cast Nat.mod_lt _ (widthOf W).size_pos), ofInt_natCast]

@[simp] theorem shr_eq_ofNat (a : UInt W) (k : UInt W8) :
    a >>> k = ofNat (a.toNat >>> k.toNat) := by
  show MoveInt.ofInt (a.toInt.fdiv (2 ^ k.toNat)) = _
  rw [toInt_eq_toNat]
  have hpow : (2 : Int) ^ k.toNat = ((2 ^ k.toNat : Nat) : Int) := by
    simp
  rw [hpow, ← Int.ofNat_fdiv, ofInt_natCast, Nat.shiftRight_eq_div_pow]

@[simp] theorem toNat_cast {W' : Type} [Width W'] (a : UInt W) :
    (cast (W' := W') a).toNat = a.toNat % (widthOf W').size := by
  show (MoveInt.ofInt a.toInt : UInt W').toNat = _
  rw [toInt_eq_toNat, ofInt_natCast, toNat_ofNat]

/-- The natural-number view of a wrapped value: unsigned wrapping is `mod`. -/
@[simp] theorem toNat_ofInt (n : Int) (h : 0 ≤ n) :
    ((MoveInt.ofInt n : UInt W)).toNat = n.toNat % (widthOf W).size := by
  obtain ⟨m, rfl⟩ : ∃ m : Nat, n = (m : Int) :=
    ⟨n.toNat, (Int.toNat_of_nonneg h).symm⟩
  rw [ofInt_natCast, toNat_ofNat, Int.toNat_natCast]

/-- The unsigned view of a generic `ofInt` result.  Checked operations are
specified over `Int`; on unsigned operands the value is the familiar
`ofNat`-of-`toNat` one, which is the form unsigned specifications and their
`toNat` collapse lemmas are written in. -/
@[simp high] theorem ofInt_add (a b : UInt W) :
    (MoveInt.ofInt (a.toInt + b.toInt) : UInt W) = ofNat (a.toNat + b.toNat) :=
  add_eq_ofNat a b

@[simp high] theorem ofInt_mul (a b : UInt W) :
    (MoveInt.ofInt (a.toInt * b.toInt) : UInt W) = ofNat (a.toNat * b.toNat) :=
  mul_eq_ofNat a b

/-- Unsigned subtraction under the non-underflow condition (below it the
marker wraps and `Nat` subtraction truncates; checked semantics aborts there,
so the two agree wherever it matters). -/
@[simp high] theorem ofInt_sub (a b : UInt W) (h : b.toNat ≤ a.toNat) :
    (MoveInt.ofInt (a.toInt - b.toInt) : UInt W) = ofNat (a.toNat - b.toNat) :=
  sub_eq_ofNat a b h

@[simp] theorem ofInt_toInt {W' : Type} [Width W'] (a : UInt W) :
    (MoveInt.ofInt a.toInt : UInt W') = ofNat a.toNat := by
  rw [toInt_eq_toNat, ofInt_natCast]

/-- Adding a bare `Int` numeral (the shape decision procedures normalize
literals to) to an unsigned value. -/
theorem ofInt_add_intLit (a : UInt W) (n : Nat) :
    (MoveInt.ofInt (a.toInt + (no_index (OfNat.ofNat n) : Int)) : UInt W) =
      ofNat (a.toNat + n) := by
  rw [toInt_eq_toNat]
  have hlit : (OfNat.ofNat n : Int) = ((n : Nat) : Int) := rfl
  have hcast : (a.toNat : Int) + (OfNat.ofNat n : Int) = ((a.toNat + n : Nat) : Int) := by
    rw [hlit]; omega
  rw [hcast, ofInt_natCast]

/-- An `Int` numeral wrapped into an unsigned type is that unsigned literal. -/
theorem ofInt_intLit (n : Nat) :
    (MoveInt.ofInt (no_index (OfNat.ofNat n) : Int) : UInt W) = ofNat n := rfl

/-- Adding a nonnegative `Int` amount to an unsigned value: the general form
that also covers literal addends (`+ 1`), which the generic `toInt_one`
produces. -/
theorem ofInt_add_nonneg (a : UInt W) {m : Int} (h : 0 ≤ m) :
    (MoveInt.ofInt (a.toInt + m) : UInt W) = ofNat (a.toNat + m.toNat) := by
  rw [toInt_eq_toNat]
  have hcast : (a.toNat : Int) + m = ((a.toNat + m.toNat : Nat) : Int) := by omega
  rw [hcast, ofInt_natCast]

/-- After cast normalization a quotient appears as an `Int` division of
natural casts; it is still the unsigned quotient. -/
@[simp] theorem ofInt_natCast_div (m n : Nat) :
    (MoveInt.ofInt ((m : Int) / (n : Int)) : UInt W) = ofNat (m / n) := by
  rw [← Int.natCast_ediv, ofInt_natCast]

@[simp] theorem ofInt_natCast_emod (m n : Nat) :
    (MoveInt.ofInt ((m : Int) % (n : Int)) : UInt W) = ofNat (m % n) := by
  rw [← Int.natCast_emod, ofInt_natCast]

@[simp high] theorem ofInt_tdiv (a b : UInt W) :
    (MoveInt.ofInt (a.toInt.tdiv b.toInt) : UInt W) = ofNat (a.toNat / b.toNat) :=
  div_eq_ofNat a b

@[simp high] theorem ofInt_tmod (a b : UInt W) :
    (MoveInt.ofInt (a.toInt.tmod b.toInt) : UInt W) = ofNat (a.toNat % b.toNat) :=
  mod_eq_ofNat a b

@[simp high] theorem ofInt_add_natCast (a : UInt W) (n : Nat) :
    (MoveInt.ofInt (a.toInt + (n : Int)) : UInt W) = ofNat (a.toNat + n) := by
  rw [toInt_eq_toNat, ← Int.natCast_add, ofInt_natCast]

@[simp high] theorem ofInt_natCast_add (n : Nat) (a : UInt W) :
    (MoveInt.ofInt ((n : Int) + a.toInt) : UInt W) = ofNat (n + a.toNat) := by
  rw [toInt_eq_toNat, ← Int.natCast_add, ofInt_natCast]

@[simp] theorem ofInt_natCast_eq (n : Nat) :
    (MoveInt.ofInt (n : Int) : UInt W) = ofNat n := ofInt_natCast n

/-- Truncated division and remainder on unsigned operands are the ordinary
natural-number ones: normalizing to that view keeps decision procedures out of
`Int.tdiv`'s sign correction. -/
@[simp] theorem toInt_tdiv (a b : UInt W) :
    a.toInt.tdiv b.toInt = ((a.toNat / b.toNat : Nat) : Int) := by
  rw [toInt_eq_toNat, toInt_eq_toNat, ← Int.ofNat_tdiv]

@[simp] theorem toInt_tmod (a b : UInt W) :
    a.toInt.tmod b.toInt = ((a.toNat % b.toNat : Nat) : Int) := by
  rw [toInt_eq_toNat, toInt_eq_toNat, ← Int.ofNat_tmod]

/-- Quotients and remainders of unsigned operands stay below the width's
bound: decision procedures need this in the `Int` view the checked division
specification produces. -/
theorem natCast_div_lt_size (a b : UInt W) :
    ((a.toNat : Int) / (b.toNat : Int)) < ((widthOf W).size : Int) := by
  have ha := a.toNat_lt
  rw [← Int.natCast_ediv]
  exact_mod_cast Nat.lt_of_le_of_lt (Nat.div_le_self _ _) ha

theorem natCast_emod_lt_size (a b : UInt W) :
    ((a.toNat : Int) % (b.toNat : Int)) < ((widthOf W).size : Int) := by
  have ha := a.toNat_lt
  rw [← Int.natCast_emod]
  exact_mod_cast Nat.lt_of_le_of_lt (Nat.mod_le _ _) ha

theorem natCast_div_nonneg (a b : UInt W) :
    (0 : Int) ≤ ((a.toNat : Int) / (b.toNat : Int)) :=
  Int.ediv_nonneg (Int.natCast_nonneg _) (Int.natCast_nonneg _)

theorem natCast_emod_nonneg (a b : UInt W) :
    (0 : Int) ≤ ((a.toNat : Int) % (b.toNat : Int)) := by
  rw [← Int.natCast_emod]; exact Int.natCast_nonneg _

theorem natCast_div_lt_u8 (a b : U8) :
    ((a.toNat : Int) / (b.toNat : Int)) < 256 := natCast_div_lt_size a b
theorem natCast_emod_lt_u8 (a b : U8) :
    ((a.toNat : Int) % (b.toNat : Int)) < 256 := natCast_emod_lt_size a b
theorem natCast_div_lt_u16 (a b : U16) :
    ((a.toNat : Int) / (b.toNat : Int)) < 65536 := natCast_div_lt_size a b
theorem natCast_emod_lt_u16 (a b : U16) :
    ((a.toNat : Int) % (b.toNat : Int)) < 65536 := natCast_emod_lt_size a b
theorem natCast_div_lt_u32 (a b : U32) :
    ((a.toNat : Int) / (b.toNat : Int)) < 4294967296 := natCast_div_lt_size a b
theorem natCast_emod_lt_u32 (a b : U32) :
    ((a.toNat : Int) % (b.toNat : Int)) < 4294967296 := natCast_emod_lt_size a b
theorem natCast_div_lt_u64 (a b : U64) :
    ((a.toNat : Int) / (b.toNat : Int)) < 18446744073709551616 := natCast_div_lt_size a b
theorem natCast_emod_lt_u64 (a b : U64) :
    ((a.toNat : Int) % (b.toNat : Int)) < 18446744073709551616 := natCast_emod_lt_size a b
theorem natCast_div_lt_u128 (a b : U128) :
    ((a.toNat : Int) / (b.toNat : Int)) < 340282366920938463463374607431768211456 := natCast_div_lt_size a b
theorem natCast_emod_lt_u128 (a b : U128) :
    ((a.toNat : Int) % (b.toNat : Int)) < 340282366920938463463374607431768211456 := natCast_emod_lt_size a b
theorem natCast_div_lt_u256 (a b : U256) :
    ((a.toNat : Int) / (b.toNat : Int)) < 115792089237316195423570985008687907853269984665640564039457584007913129639936 := natCast_div_lt_size a b
theorem natCast_emod_lt_u256 (a b : U256) :
    ((a.toNat : Int) % (b.toNat : Int)) < 115792089237316195423570985008687907853269984665640564039457584007913129639936 := natCast_emod_lt_size a b

/-- Bitwise results of unsigned operands stay below the width's bound: the
operation only clears or copies bits already present. -/
@[simp] theorem and_lt_size (a b : UInt W) :
    a.toNat &&& b.toNat < (widthOf W).size :=
  Nat.lt_of_le_of_lt Nat.and_le_left a.toNat_lt

@[simp] theorem or_lt_size (a b : UInt W) :
    a.toNat ||| b.toNat < (widthOf W).size :=
  Nat.or_lt_two_pow a.toNat_lt b.toNat_lt

@[simp] theorem xor_lt_size (a b : UInt W) :
    a.toNat ^^^ b.toNat < (widthOf W).size :=
  Nat.xor_lt_two_pow a.toNat_lt b.toNat_lt

@[simp] theorem shiftRight_lt_size (a : UInt W) (n : Nat) :
    a.toNat >>> n < (widthOf W).size :=
  Nat.lt_of_le_of_lt (Nat.shiftRight_le _ _) a.toNat_lt

/-- The `Int` view of the unsigned range, per width with the bound as a
numeral: checked operations are specified over `Int`, so decision procedures
need the operand bounds in that view (the `toNat` view is `toNat_lt`). -/
theorem toInt_lt_size (a : UInt W) : a.toInt < ((widthOf W).size : Int) := by
  have := a.isLt; simpa [MoveInt.toInt] using this

theorem toNat_lt_u8 (a : U8) : a.toNat < 256 := a.toNat_lt
theorem toNat_lt_u16 (a : U16) : a.toNat < 65536 := a.toNat_lt
theorem toNat_lt_u32 (a : U32) : a.toNat < 4294967296 := a.toNat_lt
theorem toNat_lt_u64 (a : U64) : a.toNat < 18446744073709551616 := a.toNat_lt
theorem toNat_lt_u128 (a : U128) :
    a.toNat < 340282366920938463463374607431768211456 := a.toNat_lt
theorem toNat_lt_u256 (a : U256) :
    a.toNat <
      115792089237316195423570985008687907853269984665640564039457584007913129639936 :=
  a.toNat_lt

theorem toInt_lt_u8 (a : U8) : a.toInt < 256 := toInt_lt_size a
theorem toInt_lt_u16 (a : U16) : a.toInt < 65536 := toInt_lt_size a
theorem toInt_lt_u32 (a : U32) : a.toInt < 4294967296 := toInt_lt_size a
theorem toInt_lt_u64 (a : U64) : a.toInt < 18446744073709551616 := toInt_lt_size a
theorem toInt_lt_u128 (a : U128) :
    a.toInt < 340282366920938463463374607431768211456 := toInt_lt_size a
theorem toInt_lt_u256 (a : U256) :
    a.toInt <
      115792089237316195423570985008687907853269984665640564039457584007913129639936 :=
  toInt_lt_size a

/-- The `Int` view of an unsigned literal, and of truncated division and
remainder on natural casts: the remaining links that carry a checked result
from the generic `Int` form back into the unsigned `toNat` view. -/
@[simp] theorem toInt_ofNat (n : Nat) :
    (ofNat (W := W) n).toInt = ((n % (widthOf W).size : Nat) : Int) := by
  rw [toInt_eq_toNat, toNat_ofNat]

@[simp] theorem natCast_tdiv_toInt (m : Nat) (b : UInt W) :
    ((m : Int)).tdiv b.toInt = ((m / b.toNat : Nat) : Int) := by
  rw [toInt_eq_toNat]; exact (Int.ofNat_tdiv _ _).symm

@[simp] theorem natCast_tmod_toInt (m : Nat) (b : UInt W) :
    ((m : Int)).tmod b.toInt = ((m % b.toNat : Nat) : Int) := by
  rw [toInt_eq_toNat]; exact (Int.ofNat_tmod _ _).symm

@[simp] theorem natCast_tdiv (m n : Nat) :
    ((m : Int)).tdiv ((n : Int)) = ((m / n : Nat) : Int) := (Int.ofNat_tdiv m n).symm

@[simp] theorem natCast_tmod (m n : Nat) :
    ((m : Int)).tmod ((n : Int)) = ((m % n : Nat) : Int) := (Int.ofNat_tmod m n).symm

/-! Unsigned comparisons and raw-operation equations in the natural-number
view: the unsigned instances of the generic lemmas, at high `simp` priority so
they win over the generic `toInt` forms. -/

@[simp] theorem lt_iff_toNat_lt (a b : UInt W) : a < b ↔ a.toNat < b.toNat := by
  rw [MoveInt.lt_iff_toInt_lt, toInt_eq_toNat, toInt_eq_toNat]
  exact Int.ofNat_lt

@[simp] theorem le_iff_toNat_le (a b : UInt W) : a ≤ b ↔ a.toNat ≤ b.toNat := by
  rw [MoveInt.le_iff_toInt_le, toInt_eq_toNat, toInt_eq_toNat]
  exact Int.ofNat_le

@[simp high] theorem add_def (a b : UInt W) :
    MoveInt.add a b = ofNat (a.toNat + b.toNat) := add_eq_ofNat a b
@[simp high] theorem mul_def (a b : UInt W) :
    MoveInt.mul a b = ofNat (a.toNat * b.toNat) := mul_eq_ofNat a b
@[simp high] theorem div_def (a b : UInt W) :
    MoveInt.div a b = ofNat (a.toNat / b.toNat) := div_eq_ofNat a b
@[simp high] theorem mod_def (a b : UInt W) :
    MoveInt.mod a b = ofNat (a.toNat % b.toNat) := mod_eq_ofNat a b
@[simp] theorem land_def (a b : UInt W) :
    MoveInt.land a b = ofNat (a.toNat &&& b.toNat) := land_eq_ofNat a b
@[simp] theorem lor_def (a b : UInt W) :
    MoveInt.lor a b = ofNat (a.toNat ||| b.toNat) := lor_eq_ofNat a b
@[simp] theorem lxor_def (a b : UInt W) :
    MoveInt.lxor a b = ofNat (a.toNat ^^^ b.toNat) := lxor_eq_ofNat a b
@[simp] theorem shl_def (a : UInt W) (k : UInt W8) :
    MoveInt.shl a k = ofNat ((a.toNat <<< k.toNat) % (widthOf W).size) := shl_eq_ofNat a k
@[simp] theorem shr_def (a : UInt W) (k : UInt W8) :
    MoveInt.shr a k = ofNat (a.toNat >>> k.toNat) := shr_eq_ofNat a k
@[simp high] theorem cast_def {W' : Type} [Width W'] (a : UInt W) :
    (MoveInt.cast a : UInt W') = ofNat a.toNat := by
  show MoveInt.ofInt a.toInt = _
  rw [toInt_eq_toNat, ofInt_natCast]

/-- Unsigned subtraction in the natural-number view, under the non-underflow
condition (below it the marker wraps and `Nat` subtraction truncates; checked
semantics aborts there, so the two agree wherever it matters). -/
@[simp high] theorem sub_def (a b : UInt W) (h : b.toNat ≤ a.toNat) :
    MoveInt.sub a b = ofNat (a.toNat - b.toNat) := sub_eq_ofNat a b h

end UInt

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

/-! ## Signed view

Signed Move integers are `MoveInt Signed W`; their value is `toInt`, and their
range is the two's-complement `[-halfSize, halfSize)`. -/

namespace SInt

variable {W : Type} [Width W]

theorem neg_halfSize_le_toInt (value : SInt W) :
    -((widthOf W).halfSize : Int) ≤ value.toInt := by
  have h := value.isGe
  simpa [MoveInt.toInt] using h
theorem toInt_lt_halfSize (value : SInt W) :
    value.toInt < ((widthOf W).halfSize : Int) := by
  have h := value.isLt
  simpa [MoveInt.toInt] using h

/-- The signed operations, as the generic ones at `Signed`. -/
abbrev ofInt : Int → SInt W := MoveInt.ofInt
abbrev add : SInt W → SInt W → SInt W := MoveInt.add
abbrev sub : SInt W → SInt W → SInt W := MoveInt.sub
abbrev mul : SInt W → SInt W → SInt W := MoveInt.mul
abbrev div : SInt W → SInt W → SInt W := MoveInt.div
abbrev mod : SInt W → SInt W → SInt W := MoveInt.mod
abbrev neg : SInt W → SInt W := MoveInt.neg
abbrev land : SInt W → SInt W → SInt W := MoveInt.land
abbrev lor : SInt W → SInt W → SInt W := MoveInt.lor
abbrev lxor : SInt W → SInt W → SInt W := MoveInt.lxor
abbrev shl : SInt W → UInt W8 → SInt W := MoveInt.shl
abbrev shr : SInt W → UInt W8 → SInt W := MoveInt.shr
abbrev cast {W' : Type} [Width W'] (a : SInt W) : SInt W' := MoveInt.cast a
abbrev less : SInt W → SInt W → Bool := MoveInt.less
abbrev lessEq : SInt W → SInt W → Bool := MoveInt.lessEq
abbrev equal : SInt W → SInt W → Bool := MoveInt.equal

/-- The literal view without wrapping, available whenever it is in range. -/
theorem toInt_ofInt_of_mem {n : Int}
    (hlo : -((widthOf W).halfSize : Int) ≤ n)
    (hhi : n < ((widthOf W).halfSize : Int)) :
    (ofInt (W := W) n).toInt = n :=
  MoveInt.toInt_ofInt_of_mem (by simpa using hlo) (by simpa using hhi)

/-! The signed instances of the generic operation equations, in the `toInt`
view (mirroring the unsigned `toNat` ones). -/

@[simp] theorem add_eq_ofInt (a b : SInt W) :
    a + b = ofInt (a.toInt + b.toInt) := rfl
@[simp] theorem sub_eq_ofInt (a b : SInt W) :
    a - b = ofInt (a.toInt - b.toInt) := rfl
@[simp] theorem mul_eq_ofInt (a b : SInt W) :
    a * b = ofInt (a.toInt * b.toInt) := rfl
@[simp] theorem div_eq_ofInt (a b : SInt W) :
    a / b = ofInt (a.toInt.tdiv b.toInt) := rfl
@[simp] theorem mod_eq_ofInt (a b : SInt W) :
    a % b = ofInt (a.toInt.tmod b.toInt) := rfl
@[simp] theorem neg_eq_ofInt (a : SInt W) :
    -a = ofInt (-a.toInt) := rfl

@[simp] theorem add_def (a b : SInt W) :
    MoveInt.add a b = ofInt (a.toInt + b.toInt) := rfl
@[simp] theorem sub_def (a b : SInt W) :
    MoveInt.sub a b = ofInt (a.toInt - b.toInt) := rfl
@[simp] theorem mul_def (a b : SInt W) :
    MoveInt.mul a b = ofInt (a.toInt * b.toInt) := rfl
@[simp] theorem div_def (a b : SInt W) :
    MoveInt.div a b = ofInt (a.toInt.tdiv b.toInt) := rfl
@[simp] theorem mod_def (a b : SInt W) :
    MoveInt.mod a b = ofInt (a.toInt.tmod b.toInt) := rfl
@[simp] theorem neg_def (a : SInt W) :
    MoveInt.neg a = ofInt (-a.toInt) := rfl
@[simp] theorem cast_def {W' : Type} [Width W'] (a : SInt W) :
    (MoveInt.cast a : SInt W') = ofInt a.toInt := rfl

/-- The signed view of the ubiquitous numerals. -/
@[simp] theorem toInt_zero (a : SInt W) : (0 : SInt W).toInt = 0 :=
  MoveInt.toInt_zero
@[simp] theorem toInt_one (a : SInt W) : (1 : SInt W).toInt = 1 :=
  MoveInt.toInt_one

/-- The signed view of the comparisons. -/
@[simp] theorem lt_iff_toInt_lt (a b : SInt W) : a < b ↔ a.toInt < b.toInt :=
  MoveInt.lt_iff_toInt_lt a b

@[simp] theorem le_iff_toInt_le (a b : SInt W) : a ≤ b ↔ a.toInt ≤ b.toInt :=
  MoveInt.le_iff_toInt_le a b

end SInt

/-- Resolve signed width-directed literal constructors as named specification
surface. -/
abbrev I8.ofInt : Int → I8 := SInt.ofInt
abbrev I16.ofInt : Int → I16 := SInt.ofInt
abbrev I32.ofInt : Int → I32 := SInt.ofInt
abbrev I64.ofInt : Int → I64 := SInt.ofInt
abbrev I128.ofInt : Int → I128 := SInt.ofInt
abbrev I256.ofInt : Int → I256 := SInt.ofInt

/-- Half the value count of each signed width, as named specification surface;
the signed range is `[-halfSize, halfSize)`. -/
abbrev I8.halfSize : Int := (MoveModel.IR.IntWidth.halfSize .w8 : Int)
abbrev I16.halfSize : Int := (MoveModel.IR.IntWidth.halfSize .w16 : Int)
abbrev I32.halfSize : Int := (MoveModel.IR.IntWidth.halfSize .w32 : Int)
abbrev I64.halfSize : Int := (MoveModel.IR.IntWidth.halfSize .w64 : Int)
abbrev I128.halfSize : Int := (MoveModel.IR.IntWidth.halfSize .w128 : Int)
abbrev I256.halfSize : Int := (MoveModel.IR.IntWidth.halfSize .w256 : Int)

namespace Vector

@[noinline] def empty : Vector α :=
  ⟨[], MoveModel.IR.IntWidth.size_pos _⟩

@[noinline] def singleton (value : α) : Vector α :=
  ⟨[value], by simpa using MoveModel.IR.IntWidth.one_lt_size .w64⟩

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
@[noinline] def isEmpty : Vector α → Bool :=
  fun values => values.elems.isEmpty
@[noinline] def get [Inhabited α] : Vector α → U64 → α :=
  fun values index => values.elems[index.toNat]?.getD Inhabited.default
@[noinline] def set : Vector α → U64 → α → Vector α :=
  fun values index value =>
    ⟨values.elems.set index.toNat value, by simpa using values.bounded⟩

instance : Inhabited (Vector α) := ⟨empty⟩

/-- Logical contents of a source vector. This is a specification accessor and
is never selected for Move lowering. -/
def toList (values : Vector α) : List α := values.elems

/-- Vectors are specified as lists of their elements' logical domains. -/
instance [ModelDomain α β] : ModelDomain (Vector α) (List β) where
  project values := values.toList.map (fun value => model value)

@[simp] theorem model_vector [ModelDomain α β] (values : Vector α) :
    model values = values.toList.map (fun value => model value) := rfl

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

@[simp] theorem toList_singleton (value : α) :
    (singleton value).toList = [value] := rfl

@[simp] theorem isEmpty_eq (values : Vector α) :
    isEmpty values = values.toList.isEmpty := rfl

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
