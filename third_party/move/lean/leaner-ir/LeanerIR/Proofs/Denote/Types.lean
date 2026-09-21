-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Typed
import LeanerIR.Proofs.IntegerArithmetic
import LeanerIR.Proofs.IntegerEvaluation
import LeanerIR.Proofs.Meaning
import LeanerIR.Proofs.Denote.Attr
import LeanerIR.Semantics.Focus

/-!
# Native types of the denotation

The denotation of validated LIR (`designs/denotation.md`) is typed: a term
denotes a `Spec` over a native carrier, never over `RuntimeValue`.  This
module owns the closed universe of those carriers, the environments locals
live in, the codecs that connect carriers to the runtime representation at
the function boundary, and the native operations with their
weakest-precondition rules.

Every carrier is the type a specification clause reads: a certified
integer is a `SpecInt` whose `.val` is the mathematical value, a boolean is
a `Bool`.  No frame, row, registry, or arena occurs in anything a goal
mentions.
-/

namespace LeanerIR.Proofs.Denote

open LeanerIR.SemanticOperations

mutual
/-- The types the denotation carries: scalars, tuples, and nominal data
with its declared field rows.  An enum names its variants separately from
their rows and carries the distinctness its codec needs. -/
inductive NTy where
  | unit
  | bool
  | int (width : Nat) (signed : Bool)
  | address
  | signer
  | string
  | bytes
  | tuple (elements : NRow)
  | struct (source : StructHandle) (fields : NRow)
  | enum (source : StructHandle) (names : List String) (rows : NRows)
      (distinct : names.Nodup)
  /-- A growable vector of one element type. -/
  | vector (element : NTy)
  /-- A mutable reference: its loan and the current value it owns. -/
  | ref (referent : NTy)
  /-- A type parameter of a generic function: its skolem family's carrier. -/
  | param (index : Nat)

/-- A row of types: tuple elements, declared fields, or the locals of a body. -/
inductive NRow where
  | nil
  | cons (head : NTy) (tail : NRow)

/-- The field rows of an enum's variants, in declaration order. -/
inductive NRows where
  | nil
  | cons (fields : NRow) (tail : NRows)
end

deriving instance DecidableEq for NTy, NRow, NRows
deriving instance Repr for NTy, NRow, NRows
instance : Inhabited NTy := ⟨.unit⟩
instance : Inhabited NRow := ⟨.nil⟩

/-- Whether a type is a scalar: one whose values a clause compares directly. -/
def NTy.isScalar : NTy → Bool
  | .tuple _ | .struct _ _ | .enum _ _ _ _ | .vector _ | .ref _ | .param _ => false
  | _ => true

def NRow.length : NRow → Nat
  | .nil => 0
  | .cons _ rest => rest.length + 1

def NRow.append : NRow → NRow → NRow
  | .nil, rest => rest
  | .cons τ Γ, rest => .cons τ (Γ.append rest)

instance : Append NRow := ⟨NRow.append⟩

@[simp] theorem NRow.nil_append (rest : NRow) : (NRow.nil ++ rest) = rest := rfl
@[simp] theorem NRow.cons_append (τ : NTy) (Γ rest : NRow) :
    (NRow.cons τ Γ ++ rest) = .cons τ (Γ ++ rest) := rfl

def NRow.ofList : List NTy → NRow
  | [] => .nil
  | τ :: rest => .cons τ (NRow.ofList rest)

def NRow.toList : NRow → List NTy
  | .nil => []
  | .cons τ rest => τ :: rest.toList

def NRows.length : NRows → Nat
  | .nil => 0
  | .cons _ rest => rest.length + 1

def NRows.ofList : List NRow → NRows
  | [] => .nil
  | row :: rest => .cons row (NRows.ofList rest)

def NRows.toList : NRows → List NRow
  | .nil => []
  | .cons row rest => row :: rest.toList

/-- A codec is tight when a runtime value that decodes is the encoding of
the value it decodes to: decoding and encoding are inverse on the image. -/
def _root_.LeanerIR.Proofs.Codec.Tight {Native Runtime : Type} (codec : Codec Native Runtime) : Prop :=
  ∀ raw value, codec.decode? raw = some value → codec.encode value = raw

/-- The carriers of a generic function's type parameters: per index an
inhabited type, a tight codec whose encodings are loan-free, and decidable
equality.  A generic function is proved over every family
(`designs/denotation.md`, Generics). -/
class Skolems where
  carrier : Nat → Type
  codec : (index : Nat) → Codec (carrier index) RuntimeValue
  decEq : (index : Nat) → DecidableEq (carrier index)
  inhabited : (index : Nat) → Inhabited (carrier index)
  tight : ∀ index, (codec index).Tight
  plain : ∀ index value, LeanerIR.SemanticOperations.Plain ((codec index).encode value)

instance [Θ : Skolems] (index : Nat) : Inhabited (Skolems.carrier index) := Θ.inhabited index

/-- The family of literals, which never have a type parameter's type. -/
@[reducible] def Skolems.ground : Skolems where
  carrier := fun _ => Unit
  codec := fun _ => Codec.unit
  decEq := fun _ => inferInstanceAs (DecidableEq Unit)
  inhabited := fun _ => ⟨()⟩
  tight := fun _ raw value decoded => by
    cases raw <;> simp [Codec.unit, decodeUnit?] at decoded ⊢
  plain := fun _ _ => .unit

open Classical in
/-- The family of the public theorem: every type parameter carried as a
loan-free runtime value. -/
@[reducible] noncomputable def Skolems.runtime : Skolems where
  carrier := fun _ => { value : RuntimeValue // Plain value }
  codec := fun _ =>
    ⟨Subtype.val, fun raw => if plain : Plain raw then some ⟨raw, plain⟩ else none,
      fun value => by simp [value.property]⟩
  decEq := fun _ left right => Classical.propDecidable (left = right)
  inhabited := fun _ => ⟨⟨.unit, .unit⟩⟩
  tight := fun _ raw value decoded => by
    by_cases plain : Plain raw <;> simp [plain] at decoded
    subst decoded
    rfl
  plain := fun _ value => value.property

section Carriers
variable [Skolems]

mutual
/-- The Lean type a native value of one type inhabits. -/
@[reducible] def NTy.carrier : NTy → Type
  | .unit => Unit
  | .bool => Bool
  | .int width signed => SpecInt (.bits width) signed
  | .address => String
  | .signer => String
  | .string => String
  | .bytes => Array UInt8
  | .tuple elements => HList elements
  | .struct _ fields => HList fields
  | .enum _ names rows _ => variantCarrier names rows
  | .vector element => SpecVector element.carrier
  | .ref referent => referent.carrier × referent.carrier
  | .param index => Skolems.carrier index

/-- A row of values, one per type.  Not reducible: a row type is a key of
the lemmas that rewrite values of it. -/
def HList : NRow → Type
  | .nil => Unit
  | .cons τ rest => τ.carrier × HList rest

/-- A value of one of the named variants: the nested sum of their rows.
Rows beyond the names, or names beyond the rows, have no values. -/
def variantCarrier : List String → NRows → Type
  | _, .nil => Empty
  | [], .cons _ _ => Empty
  | _ :: names, .cons fields rest => HList fields ⊕ variantCarrier names rest
end

/-- The carrier a literal's value inhabits: the ground family's, where a
type parameter has no values. -/
@[reducible] def NTy.groundCarrier (τ : NTy) : Type := @NTy.carrier Skolems.ground τ

mutual
/-- A literal's value at the current family. -/
def NTy.ofGround : (τ : NTy) → τ.groundCarrier → τ.carrier
  | .unit, value => value
  | .bool, value => value
  | .int _ _, value => value
  | .address, value => value
  | .signer, value => value
  | .string, value => value
  | .bytes, value => value
  | .tuple elements, values => HList.ofGround elements values
  | .struct _ fields, values => HList.ofGround fields values
  | .enum _ names rows _, value => variantCarrier.ofGround names rows value
  | .vector element, vector =>
      ⟨vector.values.map element.ofGround, by simpa using vector.bounded⟩
  | .ref referent, value => (referent.ofGround value.1, referent.ofGround value.2)
  | .param _, _ => default

def HList.ofGround : (row : NRow) → @HList Skolems.ground row → HList row
  | .nil, _ => ()
  | .cons τ rest, values => (τ.ofGround values.1, HList.ofGround rest values.2)

def variantCarrier.ofGround : (names : List String) → (rows : NRows) →
    @variantCarrier Skolems.ground names rows → variantCarrier names rows
  | _, .nil, value => (value : Empty).elim
  | [], .cons _ _, value => (value : Empty).elim
  | _ :: _, .cons fields _, .inl value => .inl (HList.ofGround fields value)
  | _ :: names, .cons _ rest, .inr value => .inr (variantCarrier.ofGround names rest value)
end

/-- The injections of a variant value, declared at the folded carrier type
so that an `Option` of decoded variants keeps that type; applied, they are
the sum constructors. -/
def variantCarrier.first {name : String} {names : List String} {fields : NRow} {rest : NRows}
    (value : HList fields) : variantCarrier (name :: names) (.cons fields rest) := Sum.inl value
def variantCarrier.later {name : String} {names : List String} {fields : NRow} {rest : NRows}
    (value : variantCarrier names rest) : variantCarrier (name :: names) (.cons fields rest) :=
  Sum.inr value
@[simp] theorem variantCarrier.first_eq (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (value : HList fields) :
    (variantCarrier.first (name := name) (names := names) (fields := fields) (rest := rest) value) =
      (Sum.inl value : HList fields ⊕ variantCarrier names rest) := rfl
@[simp] theorem variantCarrier.later_eq (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (value : variantCarrier names rest) :
    (variantCarrier.later (name := name) (names := names) (fields := fields) (rest := rest) value) =
      (Sum.inr value : HList fields ⊕ variantCarrier names rest) := rfl

/-- A bounded vector decides equality by its elements. -/
instance instDecidableEqSpecVector {α : Type} [DecidableEq α] : DecidableEq (SpecVector α) := fun a b =>
  if h : a.values = b.values then isTrue (SpecVector.ext h)
  else isFalse fun equal => h (congrArg SpecVector.values equal)

/-- A certified integer decides equality by its value. -/
instance {width : IntWidth} {signed : Bool} : DecidableEq (SpecInt width signed) := fun a b =>
  if h : a.val = b.val then isTrue (SpecInt.ext h)
  else isFalse fun equal => h (congrArg SpecInt.val equal)

mutual
/-- Decidable equality of native values, for the structural `==`. -/
def NTy.decEq : (τ : NTy) → DecidableEq τ.carrier
  | .unit => inferInstanceAs (DecidableEq Unit)
  | .bool => inferInstanceAs (DecidableEq Bool)
  | .int width signed => inferInstanceAs (DecidableEq (SpecInt (.bits width) signed))
  | .address => inferInstanceAs (DecidableEq String)
  | .signer => inferInstanceAs (DecidableEq String)
  | .string => inferInstanceAs (DecidableEq String)
  | .bytes => inferInstanceAs (DecidableEq (Array UInt8))
  | .tuple elements => HList.decEq elements
  | .struct _ fields => HList.decEq fields
  | .enum _ names rows _ => variantCarrier.decEq names rows
  | .vector element => @instDecidableEqSpecVector _ element.decEq
  | .ref referent => @instDecidableEqProd _ _ referent.decEq referent.decEq
  | .param index => Skolems.decEq index

def HList.decEq : (row : NRow) → DecidableEq (HList row)
  | .nil => inferInstanceAs (DecidableEq Unit)
  | .cons τ rest => @instDecidableEqProd _ _ τ.decEq (HList.decEq rest)

def variantCarrier.decEq : (names : List String) → (rows : NRows) →
    DecidableEq (variantCarrier names rows)
  | _, .nil => inferInstanceAs (DecidableEq Empty)
  | [], .cons _ _ => inferInstanceAs (DecidableEq Empty)
  | _ :: names, .cons fields rest =>
      @instDecidableEqSum _ _ (HList.decEq fields) (variantCarrier.decEq names rest)
end

/-- A nominal codec: a row of fields under the declaration's handle and
the chosen variant. -/
def Codec.nominalRow (source : StructHandle) (variant : Option String)
    (row : Codec Native (List RuntimeValue)) : Codec Native RuntimeValue where
  encode := fun value => .nominal source variant (row.encode value).toArray
  decode?
    | .nominal actualSource actualVariant fields =>
        if actualSource = source ∧ actualVariant = variant then row.decode? fields.toList
        else none
    | _ => none
  decode_encode := by
    intro value
    simp [row.decode_encode]

/-- A mutable reference's carrier, its current value and its prophecy, as a
pair. The runtime has no value for a reference apart from a loan: the
agreement lends each argument reference under a loan it assigns
(`Denote.Agreement`). This codec only keeps `NTy.codec` total; no
denotation encodes a reference with it. -/
def Codec.prophecyPair (codec : Codec Native RuntimeValue) :
    Codec (Native × Native) RuntimeValue where
  encode := fun value => .tuple #[codec.encode value.1, codec.encode value.2]
  decode?
    | .tuple values => match values.toList with
      | [current, prophecy] =>
          (codec.decode? current).bind fun current =>
            (codec.decode? prophecy).map fun prophecy => (current, prophecy)
      | _ => none
    | _ => none
  decode_encode := by
    intro value
    simp [codec.decode_encode]

/-- The row codecs of an enum's variants. -/
@[reducible] def RowCodecs : NRows → Type
  | .nil => Unit
  | .cons fields rest => Codec (HList fields) (List RuntimeValue) × RowCodecs rest

/-- Encode a variant value under its declaration's handle and name. -/
def variantEncode (source : StructHandle) : (names : List String) → (rows : NRows) →
    RowCodecs rows → variantCarrier names rows → RuntimeValue
  | _, .nil, _, value => nomatch value
  | [], .cons _ _, _, value => nomatch value
  | name :: names, .cons _ rest, (codec, codecs), value =>
      match value with
      | .inl fields => (Codec.nominalRow source (some name) codec).encode fields
      | .inr later => variantEncode source names rest codecs later

/-- Decode a variant value by its name. -/
def variantDecode? (source : StructHandle) : (names : List String) → (rows : NRows) →
    RowCodecs rows → RuntimeValue → Option (variantCarrier names rows)
  | _, .nil, _, _ => none
  | [], .cons _ _, _, _ => none
  | name :: names, .cons _ rest, (codec, codecs), runtime =>
      match runtime with
      | .nominal actualSource (some actualVariant) _ =>
          if actualSource = source ∧ actualVariant = name then
            ((Codec.nominalRow source (some name) codec).decode? runtime).map .inl
          else (variantDecode? source names rest codecs runtime).map .inr
      | _ => none

theorem variantEncode_shape (source : StructHandle) : (names : List String) → (rows : NRows) →
    (codecs : RowCodecs rows) → (value : variantCarrier names rows) →
    ∃ name fields, variantEncode source names rows codecs value =
      .nominal source (some name) fields ∧ name ∈ names
  | _, .nil, _, value => nomatch value
  | [], .cons _ _, _, value => nomatch value
  | name :: names, .cons _ rest, (codec, codecs), .inl fields =>
      ⟨name, _, rfl, List.mem_cons_self⟩
  | _ :: names, .cons _ rest, (_, codecs), .inr later => by
      obtain ⟨name, fields, encoded, member⟩ :=
        variantEncode_shape source names rest codecs later
      exact ⟨name, fields, encoded, List.mem_cons_of_mem _ member⟩

theorem variantDecode?_encode (source : StructHandle) : (names : List String) → (rows : NRows) →
    (codecs : RowCodecs rows) → names.Nodup → (value : variantCarrier names rows) →
    variantDecode? source names rows codecs (variantEncode source names rows codecs value) =
      some value
  | _, .nil, _, _, value => nomatch value
  | [], .cons _ _, _, _, value => nomatch value
  | name :: names, .cons _ rest, (codec, codecs), _, .inl fields => by
      simp [variantEncode, variantDecode?, Codec.nominalRow, codec.decode_encode]
  | name :: names, .cons _ rest, (codec, codecs), nodup, .inr later => by
      obtain ⟨actual, fields, encoded, member⟩ :=
        variantEncode_shape source names rest codecs later
      have distinct : actual ≠ name := fun equal =>
        (List.nodup_cons.mp nodup).1 (equal ▸ member)
      simp only [variantEncode]
      rw [encoded]
      simp only [variantDecode?, distinct, and_false, ↓reduceIte]
      rw [← encoded, variantDecode?_encode source names rest codecs
        (List.nodup_cons.mp nodup).2 later]
      rfl

/-- The codec of an enum's values. -/
def variantCodec (source : StructHandle) (names : List String) (rows : NRows)
    (codecs : RowCodecs rows) (distinct : names.Nodup) :
    Codec (variantCarrier names rows) RuntimeValue where
  encode := variantEncode source names rows codecs
  decode? := variantDecode? source names rows codecs
  decode_encode := variantDecode?_encode source names rows codecs distinct

mutual
/-- The certified codec of one type. -/
def NTy.codec : (τ : NTy) → Codec τ.carrier RuntimeValue
  | .unit => Codec.unit
  | .bool => Codec.bool
  | .int width signed => Codec.specInt (.bits width) signed
  | .address => Codec.address
  | .signer => Codec.signer
  | .string => Codec.string
  | .bytes => Codec.bytes
  | .tuple elements => Codec.tuple (rowCodec elements)
  | .struct source fields => Codec.nominalRow source none (rowCodec fields)
  | .enum source names rows distinct => variantCodec source names rows (rowCodecs rows) distinct
  | .vector element => Codec.boundedVector element.codec
  | .ref referent => Codec.prophecyPair referent.codec
  | .param index => Skolems.codec index

/-- The codec of a row of values. -/
def rowCodec : (row : NRow) → Codec (HList row) (List RuntimeValue)
  | .nil => Codec.tupleNil
  | .cons τ rest => Codec.tupleCons τ.codec (rowCodec rest)

def rowCodecs : (rows : NRows) → RowCodecs rows
  | .nil => ()
  | .cons fields rest => (rowCodec fields, rowCodecs rest)
end

/-- The runtime representation of a native value. -/
def NTy.encode (τ : NTy) (value : τ.carrier) : RuntimeValue := τ.codec.encode value

@[simp] theorem NTy.encode_unit (value : Unit) : NTy.encode .unit value = .unit := rfl
@[simp] theorem NTy.encode_bool (value : Bool) : NTy.encode .bool value = .bool value := rfl
@[simp] theorem NTy.encode_int (width : Nat) (signed : Bool)
    (value : SpecInt (.bits width) signed) :
    NTy.encode (.int width signed) value = .integer value.val := rfl
@[simp] theorem NTy.encode_address (value : String) :
    NTy.encode .address value = .address value := rfl
@[simp] theorem NTy.encode_signer (value : String) :
    NTy.encode .signer value = .signer value := rfl
@[simp] theorem NTy.encode_string (value : String) :
    NTy.encode .string value = .string value := rfl
@[simp] theorem NTy.encode_bytes (value : Array UInt8) :
    NTy.encode .bytes value = .bytes value := rfl
@[simp] theorem NTy.encode_vector (element : NTy) (value : SpecVector element.carrier) :
    NTy.encode (.vector element) value = .vector (value.values.map element.codec.encode) := rfl

/-- The runtime row a row of values denotes. -/
def HList.encode {Γ : NRow} (values : HList Γ) : List RuntimeValue := (rowCodec Γ).encode values

@[simp] theorem HList.encode_nil (values : HList .nil) : HList.encode values = [] := rfl
@[simp] theorem HList.encode_cons {τ : NTy} {Γ : NRow} (values : HList (.cons τ Γ)) :
    HList.encode values = τ.encode values.1 :: HList.encode values.2 := rfl

@[simp] theorem NTy.encode_tuple (elements : NRow) (value : HList elements) :
    NTy.encode (.tuple elements) value = .tuple value.encode.toArray := rfl
@[simp] theorem NTy.encode_struct (source : StructHandle) (fields : NRow)
    (value : HList fields) :
    NTy.encode (.struct source fields) value = .nominal source none value.encode.toArray := rfl
@[simp] theorem NTy.encode_enum_inl (source : StructHandle) (name : String)
    (names : List String) (fields : NRow) (rest : NRows) (distinct : (name :: names).Nodup)
    (value : HList fields) :
    NTy.encode (.enum source (name :: names) (.cons fields rest) distinct) (.inl value) =
      .nominal source (some name) value.encode.toArray := rfl
@[simp] theorem NTy.encode_ref (referent : NTy) (value : referent.carrier × referent.carrier) :
    NTy.encode (.ref referent) value =
      .tuple #[referent.encode value.1, referent.encode value.2] := rfl
@[simp] theorem NTy.encode_enum_inr (source : StructHandle) (name : String)
    (names : List String) (fields : NRow) (rest : NRows) (distinct : (name :: names).Nodup)
    (value : variantCarrier names rest) :
    NTy.encode (.enum source (name :: names) (.cons fields rest) distinct) (.inr value) =
      NTy.encode (.enum source names rest (List.nodup_cons.mp distinct).2) value := rfl

theorem NTy.encode_injective (τ : NTy) {left right : τ.carrier}
    (equal : τ.encode left = τ.encode right) : left = right :=
  τ.codec.encode_injective equal

@[simp] theorem NTy.decode_encode (τ : NTy) (value : τ.carrier) :
    τ.codec.decode? (τ.encode value) = some value :=
  τ.codec.decode_encode value

/-- A native scalar is never a borrow: the encoded frame carries no loan. -/
theorem NTy.encode_not_borrow (τ : NTy) (scalar : τ.isScalar = true) (value : τ.carrier)
    (loan : Nat) (current : RuntimeValue) : τ.encode value ≠ .borrow loan current := by
  cases τ <;> simp [NTy.isScalar] at scalar <;> simp

/-! ## Structural equality

The `==` primitive decides structural equality of runtime values.  On the
encoding of a native value it is the native decision. -/

omit [Skolems] in
private theorem beq_eq_decide {α : Type} [BEq α] [LawfulBEq α] [DecidableEq α]
    (left right : α) : (left == right) = decide (left = right) := by
  by_cases h : left = right
  · subst h
    simp
  · simp [h]

omit [Skolems] in
theorem RuntimeValue.beq_unit : (RuntimeValue.unit == RuntimeValue.unit) = true := by
  show RuntimeValue.beq _ _ = true
  unfold RuntimeValue.beq
  rfl

omit [Skolems] in
theorem RuntimeValue.beq_signer (left right : String) :
    (RuntimeValue.signer left == RuntimeValue.signer right) = decide (left = right) := by
  show RuntimeValue.beq _ _ = _
  unfold RuntimeValue.beq
  exact beq_eq_decide _ _

omit [Skolems] in
theorem RuntimeValue.beq_string (left right : String) :
    (RuntimeValue.string left == RuntimeValue.string right) = decide (left = right) := by
  show RuntimeValue.beq _ _ = _
  unfold RuntimeValue.beq
  exact beq_eq_decide _ _

omit [Skolems] in
theorem RuntimeValue.beq_bytes (left right : Array UInt8) :
    (RuntimeValue.bytes left == RuntimeValue.bytes right) = decide (left = right) := by
  show RuntimeValue.beq _ _ = _
  unfold RuntimeValue.beq
  exact beq_eq_decide _ _

mutual
/-- Native structural equality at one type: the value a clause compares. -/
def NTy.eqb : (τ : NTy) → τ.carrier → τ.carrier → Bool
  | .unit, _, _ => true
  | .bool, left, right => @decide (left = right) (NTy.decEq .bool left right)
  | .int width signed, left, right =>
      @decide ((left : SpecInt (.bits width) signed).val = (right : SpecInt (.bits width) signed).val)
        inferInstance
  | .address, left, right => @decide (left = right) (NTy.decEq .address left right)
  | .signer, left, right => @decide (left = right) (NTy.decEq .signer left right)
  | .string, left, right => @decide (left = right) (NTy.decEq .string left right)
  | .bytes, left, right => @decide (left = right) (NTy.decEq .bytes left right)
  | .tuple elements, left, right => rowEqb elements left right
  | .struct _ fields, left, right => rowEqb fields left right
  | .enum _ names rows _, left, right => variantEqb names rows left right
  | .vector element, left, right =>
      left.values.size == right.values.size &&
        (left.values.toList.zip right.values.toList).all fun pair => element.eqb pair.1 pair.2
  | .ref _, _, _ => false
  | .param index, left, right => @decide (left = right) (Skolems.decEq index left right)

/-- Structural equality of rows, component by component. -/
def rowEqb : (row : NRow) → HList row → HList row → Bool
  | .nil, _, _ => true
  | .cons τ rest, left, right => τ.eqb left.1 right.1 && rowEqb rest left.2 right.2

/-- Structural equality of variant values: the same variant with equal rows. -/
def variantEqb : (names : List String) → (rows : NRows) →
    variantCarrier names rows → variantCarrier names rows → Bool
  | _, .nil, left, _ => nomatch left
  | [], .cons _ _, left, _ => nomatch left
  | _ :: names, .cons fields rest, left, right =>
      match left, right with
      | .inl l, .inl r => rowEqb fields l r
      | .inr l, .inr r => variantEqb names rest l r
      | .inl _, .inr _ => false
      | .inr _, .inl _ => false
end

@[simp] theorem NTy.eqb_tuple (elements : NRow) (left right : HList elements) :
    NTy.eqb (.tuple elements) left right = rowEqb elements left right := rfl
@[simp] theorem NTy.eqb_struct (source : StructHandle) (fields : NRow)
    (left right : HList fields) :
    NTy.eqb (.struct source fields) left right = rowEqb fields left right := rfl
@[simp] theorem NTy.eqb_enum (source : StructHandle) (names : List String) (rows : NRows)
    (distinct : names.Nodup) (left right : variantCarrier names rows) :
    NTy.eqb (.enum source names rows distinct) left right = variantEqb names rows left right := rfl
@[simp] theorem NTy.eqb_vector (element : NTy) (left right : SpecVector element.carrier) :
    NTy.eqb (.vector element) left right =
      (left.values.size == right.values.size &&
        (left.values.toList.zip right.values.toList).all fun pair => element.eqb pair.1 pair.2) := rfl
@[simp] theorem rowEqb_nil (left right : HList .nil) : rowEqb .nil left right = true := rfl
@[simp] theorem rowEqb_cons (τ : NTy) (rest : NRow) (left right : HList (.cons τ rest)) :
    rowEqb (.cons τ rest) left right = (τ.eqb left.1 right.1 && rowEqb rest left.2 right.2) := rfl
@[simp] theorem variantEqb_inl_inl (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (left right : HList fields) :
    variantEqb (name :: names) (.cons fields rest) (.inl left) (.inl right) =
      rowEqb fields left right := rfl
@[simp] theorem variantEqb_inr_inr (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (left right : variantCarrier names rest) :
    variantEqb (name :: names) (.cons fields rest) (.inr left) (.inr right) =
      variantEqb names rest left right := rfl
@[simp] theorem variantEqb_inl_inr (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (left : HList fields) (right : variantCarrier names rest) :
    variantEqb (name :: names) (.cons fields rest) (.inl left) (.inr right) = false := rfl
@[simp] theorem variantEqb_inr_inl (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (left : variantCarrier names rest) (right : HList fields) :
    variantEqb (name :: names) (.cons fields rest) (.inr left) (.inl right) = false := rfl

/-- Whether a name is among the tested names. -/
def namedIn (name : String) : List String → Bool
  | [] => false
  | test :: rest => name == test || namedIn name rest

omit [Skolems] in
@[simp] theorem namedIn_nil (name : String) : namedIn name [] = false := rfl
omit [Skolems] in
@[simp] theorem namedIn_cons (name test : String) (rest : List String) :
    namedIn name (test :: rest) = (name == test || namedIn name rest) := rfl

/-- At a scalar type, runtime structural equality of encodings is the
native decision. -/
theorem NTy.beq_encode (τ : NTy) (scalar : τ.isScalar = true) (left right : τ.carrier) :
    (τ.encode left == τ.encode right) = τ.eqb left right := by
  cases τ <;> simp [NTy.isScalar] at scalar <;> simp [NTy.eqb, RuntimeValue.beq_unit,
    RuntimeValue.beq_signer, RuntimeValue.beq_string, RuntimeValue.beq_bytes] <;>
    exact Iff.rfl

@[simp] theorem NTy.eqb_int (width : Nat) (signed : Bool)
    (left right : SpecInt (.bits width) signed) :
    NTy.eqb (.int width signed) left right = decide (left.val = right.val) := rfl
@[simp] theorem NTy.eqb_bool (left right : Bool) :
    NTy.eqb .bool left right = decide (left = right) := rfl
@[simp] theorem NTy.eqb_address (left right : String) :
    NTy.eqb .address left right = decide (left = right) := rfl
@[simp] theorem NTy.eqb_unit (left right : Unit) : NTy.eqb .unit left right = true := rfl
@[simp] theorem NTy.eqb_param (index : Nat) (left right : (NTy.param index).carrier) :
    NTy.eqb (.param index) left right = @decide (left = right) (Skolems.decEq index left right) :=
  rfl

/-! ## Environments

A function's locals are one heterogeneous row indexed by their declared
types.  A slot is `none` before its first assignment, as in the runtime
frame; a term binds a local by writing its slot.  Variables are
intrinsically typed positions, so reading a slot needs no cast and no
junk value. -/

/-- The locals of a body, in declaration order. -/
@[reducible] def HEnv : NRow → Type
  | .nil => Unit
  | .cons τ Γ => Option τ.carrier × HEnv Γ

/-- A typed position in a row. -/
inductive Var : NRow → NTy → Type where
  | here {Γ : NRow} {τ : NTy} : Var (.cons τ Γ) τ
  | there {Γ : NRow} {σ τ : NTy} (rest : Var Γ τ) : Var (.cons σ Γ) τ

/-- The declaration index of a variable. -/
def Var.index : {Γ : NRow} → {τ : NTy} → Var Γ τ → Nat
  | _, _, .here => 0
  | _, _, .there rest => rest.index + 1

def Var.get : {Γ : NRow} → {τ : NTy} → Var Γ τ → HEnv Γ → Option τ.carrier
  | _, _, .here, env => env.1
  | _, _, .there rest, env => rest.get env.2

def Var.set : {Γ : NRow} → {τ : NTy} → Var Γ τ → τ.carrier → HEnv Γ → HEnv Γ
  | _, _, .here, value, env => (some value, env.2)
  | _, _, .there rest, value, env => (env.1, rest.set value env.2)

/-- Empty a slot: the value has moved out. -/
def Var.clear : {Γ : NRow} → {τ : NTy} → Var Γ τ → HEnv Γ → HEnv Γ
  | _, _, .here, env => (none, env.2)
  | _, _, .there rest, env => (env.1, rest.clear env.2)

/-- The component of a row at a position. -/
def Var.select : {Γ : NRow} → {τ : NTy} → Var Γ τ → HList Γ → τ.carrier
  | _, _, .here, values => values.1
  | _, _, .there rest, values => rest.select values.2

@[simp] theorem Var.get_here {Γ : NRow} {τ : NTy} (env : HEnv (.cons τ Γ)) :
    (Var.here : Var (.cons τ Γ) τ).get env = env.1 := rfl
@[simp] theorem Var.get_there {Γ : NRow} {σ τ : NTy} (rest : Var Γ τ)
    (env : HEnv (.cons σ Γ)) : (Var.there rest).get env = rest.get env.2 := rfl
@[simp] theorem Var.set_here {Γ : NRow} {τ : NTy} (value : τ.carrier)
    (env : HEnv (.cons τ Γ)) :
    (Var.here : Var (.cons τ Γ) τ).set value env = (some value, env.2) := rfl
@[simp] theorem Var.set_there {Γ : NRow} {σ τ : NTy} (rest : Var Γ τ)
    (value : τ.carrier) (env : HEnv (.cons σ Γ)) :
    (Var.there rest).set value env = (env.1, rest.set value env.2) := rfl
@[simp] theorem Var.clear_here {Γ : NRow} {τ : NTy} (env : HEnv (.cons τ Γ)) :
    (Var.here : Var (.cons τ Γ) τ).clear env = (none, env.2) := rfl
@[simp] theorem Var.clear_there {Γ : NRow} {σ τ : NTy} (rest : Var Γ τ)
    (env : HEnv (.cons σ Γ)) : (Var.there rest).clear env = (env.1, rest.clear env.2) := rfl
@[simp] theorem Var.select_here {Γ : NRow} {τ : NTy} (values : HList (.cons τ Γ)) :
    (Var.here : Var (.cons τ Γ) τ).select values = values.1 := rfl
@[simp] theorem Var.select_there {Γ : NRow} {σ τ : NTy} (rest : Var Γ τ)
    (values : HList (.cons σ Γ)) : (Var.there rest).select values = rest.select values.2 := rfl

/-- The runtime locals an environment denotes. -/
def HEnv.encode : {Γ : NRow} → HEnv Γ → List (Option RuntimeValue)
  | .nil, _ => []
  | .cons τ _, env => env.1.map τ.encode :: HEnv.encode env.2

/-- The runtime frame an environment denotes: its locals, and no loan. -/
def envFrame {Γ : NRow} (env : HEnv Γ) : RuntimeFrame :=
  { locals := env.encode.toArray }

theorem HEnv.encode_injective : {Γ : NRow} → {left right : HEnv Γ} →
    left.encode = right.encode → left = right
  | .nil, _, _, _ => rfl
  | .cons τ Γ, (some a, restA), (some b, restB), equal => by
      simp only [HEnv.encode, Option.map_some, List.cons.injEq, Option.some.injEq] at equal
      obtain ⟨head, tail⟩ := equal
      cases τ.encode_injective head
      cases HEnv.encode_injective tail
      rfl
  | .cons τ Γ, (none, restA), (none, restB), equal => by
      simp only [HEnv.encode, Option.map_none, List.cons.injEq, true_and] at equal
      cases HEnv.encode_injective equal
      rfl
  | .cons τ Γ, (some _, _), (none, _), equal => by
      simp [HEnv.encode] at equal
  | .cons τ Γ, (none, _), (some _, _), equal => by
      simp [HEnv.encode] at equal

theorem HEnv.encode_length : {Γ : NRow} → (env : HEnv Γ) → env.encode.length = Γ.length
  | .nil, _ => rfl
  | .cons _ _, env => by simp [HEnv.encode, HEnv.encode_length env.2, NRow.length]

theorem envFrame_injective {Γ : NRow} {left right : HEnv Γ}
    (equal : envFrame left = envFrame right) : left = right := by
  apply HEnv.encode_injective
  have := congrArg (fun frame : RuntimeFrame => frame.locals.toList) equal
  simpa [envFrame] using this

/-- Reading a slot of the encoded frame is the encoded slot read. -/
theorem readLocal?_envFrame : {Γ : NRow} → {τ : NTy} → (x : Var Γ τ) → (env : HEnv Γ) →
    readLocal? (envFrame env) ⟨x.index⟩ = (x.get env).map τ.encode
  | _, _, .here, env => by
      simp only [readLocal?, envFrame, HEnv.encode, Var.index, List.getElem?_toArray,
        List.getElem?_cons_zero, Var.get_here]
      cases env.1 <;> rfl
  | _, _, .there rest, env => by
      have ih := readLocal?_envFrame rest env.2
      simp only [readLocal?, envFrame, HEnv.encode, Var.index, List.getElem?_toArray,
        List.getElem?_cons_succ, Var.get_there] at ih ⊢
      exact ih

omit [Skolems] in
theorem Var.index_lt : {Γ : NRow} → {τ : NTy} → (x : Var Γ τ) → x.index < Γ.length
  | _, _, .here => by simp [Var.index, NRow.length]
  | _, _, .there rest => by simp [Var.index, Var.index_lt rest, NRow.length]

/-- Writing a slot of the encoded row is the encoded slot write. -/
theorem HEnv.encode_set : {Γ : NRow} → {τ : NTy} → (x : Var Γ τ) → (value : τ.carrier) →
    (env : HEnv Γ) →
    (x.set value env).encode = env.encode.set x.index (some (τ.encode value))
  | _, _, .here, value, env => by
      simp [HEnv.encode, Var.index, Var.set]
  | _, _, .there rest, value, env => by
      simp [HEnv.encode, Var.index, Var.set, HEnv.encode_set rest value env.2]

/-- Writing a slot of the encoded frame is the encoded slot write. -/
theorem envFrame_set {Γ : NRow} {τ : NTy} (x : Var Γ τ) (value : τ.carrier)
    (env : HEnv Γ) :
    { envFrame env with locals := (envFrame env).locals.set! x.index (some (τ.encode value)) } =
      envFrame (x.set value env) := by
  simp [envFrame, HEnv.encode_set, Array.set!_eq_setIfInBounds, List.setIfInBounds_toArray]

/-- The argument-row codec of a function with the given parameter types. -/
def hlistCodec (Γ : NRow) : Codec (HList Γ) (Array RuntimeValue) where
  encode := fun values => values.encode.toArray
  decode? := fun values => (rowCodec Γ).decode? values.toList
  decode_encode := by
    intro values
    simp [HList.encode, (rowCodec Γ).decode_encode]

/-- The declared results of a function: none, or one native value.  Move
lowers a multi-result as one tuple; that shape is not carried yet. -/
inductive ResultShape where
  | none
  | one (τ : NTy)
  deriving DecidableEq, Repr, Inhabited

@[reducible] def ResultShape.carrier : ResultShape → Type
  | .none => Unit
  | .one τ => τ.carrier

/-- The type the body of a function must produce. -/
def ResultShape.bodyType : ResultShape → NTy
  | .none => .unit
  | .one τ => τ

/-- The result-row codec of a function. -/
def resultCodec : (shape : ResultShape) → Codec shape.carrier (Array RuntimeValue)
  | .none => {
      encode := fun _ => #[]
      decode? := fun values => if values.isEmpty then some () else none
      decode_encode := by intro value; cases value; rfl }
  | .one τ => {
      encode := fun value => #[τ.encode value]
      decode? := fun values => match values.toList with
        | [value] => τ.codec.decode? value
        | _ => none
      decode_encode := by intro value; simp }

/-- The initial locals of a body: its arguments, then uninitialized slots. -/
def initialEnv : (params rest : NRow) → HList params → HEnv (params ++ rest)
  | .nil, .nil, _ => ()
  | .nil, .cons _ rest, _ => (none, initialEnv .nil rest ())
  | .cons _ params, rest, values => (some values.1, initialEnv params rest values.2)

end Carriers

/-! ## Native operations

Each operation is the `Spec` a checked primitive denotes on native
carriers, and each has one weakest-precondition rule.  Failures carry the
runtime payload the primitive throws, so a declared abort code is checked
exactly. -/

/-- A `Spec` over the runtime state with LIR failures. -/
abbrev Comp (α : Type) := Spec RuntimeState Failure α

/-- A certificate from bounds, at a nonzero width. -/
theorem fits_of_bounds {width : Nat} {signed : Bool} {value lower upper : Int}
    (bounds : (Ty.integer (.bits width) signed).integerBounds? = some (lower, upper))
    (inRange : lower ≤ value ∧ value ≤ upper) :
    IntegerValueFits (.bits width) signed value := by
  simp [IntegerValueFits, Ty.integerValueFits?, bounds, inRange.1, inRange.2]

/-- The bounds a fixed-width type has, at a nonzero width. -/
theorem integerBounds?_of_nonzero {width : Nat} (signed : Bool) (nonzero : width ≠ 0) :
    (Ty.integer (.bits width) signed).integerBounds? =
      some (if signed then (-(2 : Int) ^ (width - 1), (2 : Int) ^ (width - 1) - 1)
        else (0, (2 : Int) ^ width - 1)) := by
  cases signed <;> simp [Ty.integerBounds?, nonzero]

theorem IntegerValueFits_unsigned_succ (n : Nat) (value : Int) :
    IntegerValueFits (.bits (n + 1)) false value ↔ 0 ≤ value ∧ value ≤ 2 ^ (n + 1) - 1 := by
  simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]

theorem IntegerValueFits_signed_succ (n : Nat) (value : Int) :
    IntegerValueFits (.bits (n + 1)) true value ↔
      -(2 : Int) ^ n ≤ value ∧ value ≤ 2 ^ n - 1 := by
  simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]

/-- The length of a bounded vector, which fits `u64` by its bound. -/
def vectorLength {α : Type} (vector : SpecVector α) : SpecInt (.bits 64) false :=
  ⟨vector.values.size, (IntegerValueFits_unsigned_succ 63 _).mpr
    ⟨Int.natCast_nonneg _, by have := vector.bounded; omega⟩⟩

@[simp] theorem vectorLength_val {α : Type} (vector : SpecVector α) :
    (vectorLength vector).val = vector.values.size := rfl

/-- An array as a bounded vector, when its size is below the bound. -/
def _root_.LeanerIR.SpecVector.ofArray? {α : Type} (values : Array α) : Option (SpecVector α) :=
  if bounded : values.size < 2 ^ 64 then some ⟨values, bounded⟩ else none

theorem _root_.LeanerIR.SpecVector.ofArray?_eq {α : Type} (values : Array α) :
    SpecVector.ofArray? values =
      if bounded : values.size < 2 ^ 64 then some ⟨values, bounded⟩ else none := rfl

/-- The runtime's in-place range reversal (`reverseVectorRange`), on any
element type: a swap schedule over the range, from both ends inward. -/
def reverseRange {α : Type} : Nat → Nat → Nat → Array α → Array α
  | 0, _, _, elements => elements
  | count + 1, left, right, elements =>
      reverseRange count (left + 1) (right - 1) (elements.swapIfInBounds left right)

@[simp] theorem reverseRange_zero {α : Type} (left right : Nat) (elements : Array α) :
    reverseRange 0 left right elements = elements := rfl

/-- The runtime's search (`findVectorIndex`): the first position whose
element the equality accepts. -/
def findIndex? {α : Type} (eq : α → α → Bool) (elements : Array α) (needle : α) :
    Nat → Nat → Option Nat
  | 0, _ => none
  | count + 1, index =>
      if elements[index]?.any (eq · needle) then some index
      else findIndex? eq elements needle count (index + 1)

theorem findIndex?_lt {α : Type} (eq : α → α → Bool) (elements : Array α) (needle : α) :
    ∀ (count index found : Nat), findIndex? eq elements needle count index = some found →
      found < elements.size := by
  intro count
  induction count with
  | zero => intro _ _ h; simp [findIndex?] at h
  | succ n ih =>
      intro index found h
      simp only [findIndex?] at h
      split at h
      · rename_i hit
        cases h
        cases present : elements[index]? with
        | none => rw [present] at hit; simp [Option.any] at hit
        | some _ => exact (Array.getElem?_eq_some_iff.mp present).1
      · exact ih _ _ h

private theorem findIndex?_eq_none_from {α : Type} (eq : α → α → Bool) (elements : Array α) (needle : α) :
    ∀ (count index : Nat), index + count = elements.size →
      (findIndex? eq elements needle count index = none ↔
        ∀ i (h : i < elements.size), index ≤ i → eq elements[i] needle = false) := by
  intro count
  induction count with
  | zero =>
      intro index sum
      simp only [findIndex?, true_iff]
      intro i h low
      omega
  | succ n ih =>
      intro index sum
      have inBounds : index < elements.size := by omega
      simp only [findIndex?, Array.getElem?_eq_getElem inBounds, Option.any_some]
      split
      · rename_i hit
        simp only [reduceCtorEq, false_iff, Classical.not_forall]
        exact ⟨index, inBounds, Nat.le_refl _, by simp [hit]⟩
      · rename_i miss
        rw [ih (index + 1) (by omega)]
        constructor
        · intro rest i h low
          by_cases same : i = index
          · subst same; simpa using miss
          · exact rest i h (by omega)
        · intro all i h low
          exact all i h (by omega)

/-- An insertion at a position within the vector (the end included). -/
theorem insertIdxIfInBounds_of_le {α : Type} (xs : Array α) (i : Nat) (a : α) (h : i ≤ xs.size) :
    xs.insertIdxIfInBounds i a = xs.insertIdx i a h := by
  simp [Array.insertIdxIfInBounds, h]

/-- A removal at a position within the vector. -/
theorem eraseIdxIfInBounds_of_lt {α : Type} (xs : Array α) (i : Nat) (h : i < xs.size) :
    xs.eraseIdxIfInBounds i = xs.eraseIdx i h := by
  simp [Array.eraseIdxIfInBounds, h]

/-- A field of an element looked up in a vector is looked up as that field
of the element, so that it reduces once the element's encoding does. -/
theorem getD_map_field {α : Type} (element : Option α) (encode : α → RuntimeValue)
    (index : Nat) :
    ((element.map encode).getD .unit).field index =
      (element.map fun value => (encode value).field index).getD .unit := by
  cases element <;> rfl

/-- A search over the whole vector fails exactly when no element is accepted. -/
theorem findIndex?_eq_none_iff {α : Type} (eq : α → α → Bool) (elements : Array α) (needle : α) :
    findIndex? eq elements needle elements.size 0 = none ↔
      ∀ x ∈ elements, eq x needle = false := by
  rw [findIndex?_eq_none_from eq elements needle elements.size 0 (by omega)]
  constructor
  · intro all x mem
    obtain ⟨i, h, rfl⟩ := Array.getElem_of_mem mem
    exact all i h (Nat.zero_le _)
  · intro all i h _
    exact all _ (Array.getElem_mem h)

/-- A search over the whole vector succeeds exactly when some element is
accepted. -/
theorem findIndex?_isSome_iff {α : Type} (eq : α → α → Bool) (elements : Array α) (needle : α) :
    (findIndex? eq elements needle elements.size 0).isSome = true ↔
      ∃ x ∈ elements, eq x needle = true := by
  rw [← Bool.not_eq_false, Option.isSome_eq_false_iff, Option.isNone_iff_eq_none,
    findIndex?_eq_none_iff]
  simp

/-- The found position of a search, as a `u64`: below the vector's size,
which is below the bound. -/
def foundIndex {α : Type} (vector : SpecVector α) (found : Option Nat)
    (bound : ∀ i, found = some i → i < vector.values.size) : SpecInt (.bits 64) false :=
  ⟨found.getD 0, (IntegerValueFits_unsigned_succ 63 _).mpr ⟨Int.natCast_nonneg _, by
    have := vector.bounded
    cases h : found with
    | none => simp
    | some i => have := bound i h; simp; omega⟩⟩

/-- The first position of the needle in a bounded vector, as a `u64`. -/
def foundIndexOf {α : Type} (eq : α → α → Bool) (vector : SpecVector α) (needle : α) :
    SpecInt (.bits 64) false :=
  foundIndex vector (findIndex? eq vector.values needle vector.values.size 0)
    (fun _ h => findIndex?_lt eq vector.values needle _ _ _ h)

@[simp] theorem foundIndex_val {α : Type} (vector : SpecVector α) (found : Option Nat)
    (bound : ∀ i, found = some i → i < vector.values.size) :
    (foundIndex vector found bound).val = found.getD 0 := rfl

/-- The structural order of two runtime values as the `i8` `compare`
returns. -/
def compareResult (orders : Array (Array (Array String))) (left right : RuntimeValue) :
    SpecInt (.bits 8) true :=
  ⟨orderValue (RuntimeValue.order (SemanticOperations.variantRank orders) left right), by
    rw [IntegerValueFits_signed_succ]
    cases RuntimeValue.order (SemanticOperations.variantRank orders) left right <;>
      simp [orderValue]⟩

@[simp] theorem compareResult_val (orders : Array (Array (Array String)))
    (left right : RuntimeValue) :
    (compareResult orders left right).val =
      orderValue (RuntimeValue.order (SemanticOperations.variantRank orders) left right) := rfl

/-- A bounded vector with one element replaced; the size is unchanged. -/
def _root_.LeanerIR.SpecVector.set {α : Type} (vector : SpecVector α) (index : Nat) (value : α) : SpecVector α :=
  ⟨vector.values.set! index value, by rw [Array.size_set!]; exact vector.bounded⟩

@[simp] theorem _root_.LeanerIR.SpecVector.values_set {α : Type} (vector : SpecVector α) (index : Nat) (value : α) :
    (vector.set index value).values = vector.values.set! index value := rfl

@[simp] theorem _root_.LeanerIR.SpecVector.values_mk {α : Type} (values : Array α) (bounded : values.size < 2 ^ 64) :
    (SpecVector.mk values bounded).values = values := rfl

/-- The bounds a range certificate carries, as facts a leaf adds beside it
rather than rewrites into it: a term's certificate keeps its type. -/
theorem bounds_of_fits_unsigned {width : Nat} {value : Int}
    (fits : IntegerValueFits (.bits width) false value) : 0 ≤ value ∧ value ≤ 2 ^ width - 1 := by
  cases width with
  | zero => simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?] at fits
  | succ n => exact (IntegerValueFits_unsigned_succ n value).mp fits

theorem bounds_of_fits_signed {width : Nat} {value : Int}
    (fits : IntegerValueFits (.bits width) true value) :
    -(2 : Int) ^ (width - 1) ≤ value ∧ value ≤ 2 ^ (width - 1) - 1 := by
  cases width with
  | zero => simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?] at fits
  | succ n => simpa using (IntegerValueFits_signed_succ n value).mp fits

theorem bounds_of_not_fits_unsigned {width : Nat} {value : Int} (nonzero : width ≠ 0)
    (unfit : ¬IntegerValueFits (.bits width) false value) :
    ¬(0 ≤ value ∧ value ≤ 2 ^ width - 1) := by
  cases width with
  | zero => exact absurd rfl nonzero
  | succ n => exact fun bounds => unfit ((IntegerValueFits_unsigned_succ n value).mpr bounds)

theorem bounds_of_not_fits_signed {width : Nat} {value : Int} (nonzero : width ≠ 0)
    (unfit : ¬IntegerValueFits (.bits width) true value) :
    ¬(-(2 : Int) ^ (width - 1) ≤ value ∧ value ≤ 2 ^ (width - 1) - 1) := by
  cases width with
  | zero => exact absurd rfl nonzero
  | succ n => exact fun bounds => unfit ((IntegerValueFits_signed_succ n value).mpr (by simpa using bounds))

/-- Zero-width integers have no inhabitants. -/
theorem _root_.LeanerIR.SpecInt.width_nonzero {width : Nat} {signed : Bool}
    (value : SpecInt (.bits width) signed) : width ≠ 0 := by
  intro zero
  subst zero
  have := value.fits
  simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?] at this

/-- A certified integer built from bounds, at the width of a witness. -/
def _root_.LeanerIR.SpecInt.ofBounds {width : Nat} (witness : SpecInt (.bits width) false) (value : Int)
    (inRange : 0 ≤ value ∧ value ≤ 2 ^ width - 1) : SpecInt (.bits width) false :=
  ⟨value, fits_of_bounds (integerBounds?_of_nonzero false witness.width_nonzero) inRange⟩

@[simp] theorem _root_.LeanerIR.SpecInt.ofBounds_val {width : Nat} (witness : SpecInt (.bits width) false)
    (value : Int) (inRange : 0 ≤ value ∧ value ≤ 2 ^ width - 1) :
    (SpecInt.ofBounds witness value inRange).val = value := rfl

/-- The fit of a value at a nonzero width, as its bounds. -/
theorem fits_iff_of_bounds {width : Nat} {signed : Bool} {value lower upper : Int}
    (bounds : (Ty.integer (.bits width) signed).integerBounds? = some (lower, upper)) :
    IntegerValueFits (.bits width) signed value ↔ lower ≤ value ∧ value ≤ upper := by
  simp [IntegerValueFits, Ty.integerValueFits?, bounds]

/-- Nothing fits a width without bounds. -/
theorem not_fits_of_none {width : Nat} {signed : Bool} {value : Int}
    (bounds : (Ty.integer (.bits width) signed).integerBounds? = none) :
    ¬IntegerValueFits (.bits width) signed value := by
  simp [IntegerValueFits, Ty.integerValueFits?, bounds]

/-- The checked result of an integer computation: the value, certified,
or the failure carrying the out-of-range value.  Matches `checkedInteger`,
including the payload-free failure of a width without bounds. -/
def checkedInt (failure : ThrowKind) (width : Nat) (signed : Bool) (value : Int) :
    Comp (SpecInt (.bits width) signed) :=
  if fits : IntegerValueFits (.bits width) signed value then Spec.pure ⟨value, fits⟩
  else match (Ty.integer (.bits width) signed).integerBounds? with
    | some _ => Spec.abort (failure, #[.integer value])
    | none => Spec.abort (failure, #[])

theorem checkedInt_eq (failure : ThrowKind) (width : Nat) (signed : Bool) (value : Int)
    (nonzero : width ≠ 0) :
    checkedInt failure width signed value =
      if fits : IntegerValueFits (.bits width) signed value then Spec.pure ⟨value, fits⟩
      else Spec.abort (failure, #[.integer value]) := by
  unfold checkedInt
  rw [integerBounds?_of_nonzero signed nonzero]

/-- The checked operation agrees with the runtime evaluator. -/
theorem checkedInt_ok_iff (failure : ThrowKind) (width : Nat) (signed : Bool) (value : Int)
    (state final : RuntimeState) (result : SpecInt (.bits width) signed) :
    (checkedInt failure width signed value).ok state result final ↔
      checkedInteger failure (.integer (.bits width) signed) value = .ok (.integer result.val) ∧
        final = state := by
  unfold checkedInt checkedInteger
  cases bounds : (Ty.integer (.bits width) signed).integerBounds? with
  | none => simp [not_fits_of_none bounds]
  | some pair =>
      obtain ⟨lower, upper⟩ := pair
      by_cases fits : IntegerValueFits (.bits width) signed value
      · have inRange := (fits_iff_of_bounds bounds).mp fits
        simp only [fits, ↓reduceDIte, Spec.pure_ok, inRange.1, inRange.2, decide_true,
          Bool.and_self, ↓reduceIte, Except.ok.injEq, RuntimeValue.integer.injEq]
        constructor
        · rintro ⟨rfl, rfl⟩
          exact ⟨rfl, rfl⟩
        · rintro ⟨equal, rfl⟩
          exact ⟨SpecInt.ext equal.symm, rfl⟩
      · have outOfRange : ¬(lower ≤ value ∧ value ≤ upper) :=
          fun inRange => fits ((fits_iff_of_bounds bounds).mpr inRange)
        simp only [fits, ↓reduceDIte, Spec.abort_ok, false_iff, not_and]
        intro equal
        split at equal
        · rename_i inRange
          exact absurd ⟨of_decide_eq_true (Bool.and_eq_true_iff.mp inRange).1,
            of_decide_eq_true (Bool.and_eq_true_iff.mp inRange).2⟩ outOfRange
        · cases equal

theorem checkedInt_aborts_iff (failure : ThrowKind) (width : Nat) (signed : Bool)
    (value : Int) (state : RuntimeState) (error : Failure) :
    (checkedInt failure width signed value).aborts state error ↔
      ∃ kind thrown, checkedInteger failure (.integer (.bits width) signed) value =
        .error (kind, thrown) ∧ error = (kind, thrown) := by
  unfold checkedInt checkedInteger
  cases bounds : (Ty.integer (.bits width) signed).integerBounds? with
  | none =>
      simp only [not_fits_of_none bounds, ↓reduceDIte, Spec.abort_aborts, Except.error.injEq]
      constructor
      · rintro rfl
        exact ⟨_, _, rfl, rfl⟩
      · rintro ⟨_, _, equal, rfl⟩
        cases equal
        rfl
  | some pair =>
      obtain ⟨lower, upper⟩ := pair
      by_cases fits : IntegerValueFits (.bits width) signed value
      · have inRange := (fits_iff_of_bounds bounds).mp fits
        simp [fits, inRange.1, inRange.2]
      · have outOfRange : ¬(lower ≤ value ∧ value ≤ upper) :=
          fun inRange => fits ((fits_iff_of_bounds bounds).mpr inRange)
        have failed : ¬ (decide (lower ≤ value) && decide (value ≤ upper)) = true := by
          intro inRange
          exact outOfRange ⟨of_decide_eq_true (Bool.and_eq_true_iff.mp inRange).1,
            of_decide_eq_true (Bool.and_eq_true_iff.mp inRange).2⟩
        simp only [fits, ↓reduceDIte, Spec.abort_aborts, failed, ↓reduceIte, Except.error.injEq]
        constructor
        · rintro rfl
          exact ⟨_, _, rfl, rfl⟩
        · rintro ⟨_, _, equal, rfl⟩
          cases equal
          rfl

/-- Native comparison of two integers. -/
inductive CompareOp where
  | less
  | greater
  | lessEqual
  | greaterEqual
  deriving DecidableEq, Repr, Inhabited

def CompareOp.decide : CompareOp → Int → Int → Bool
  | .less, left, right => Decidable.decide (left < right)
  | .greater, left, right => Decidable.decide (left > right)
  | .lessEqual, left, right => Decidable.decide (left ≤ right)
  | .greaterEqual, left, right => Decidable.decide (left ≥ right)

@[simp] theorem CompareOp.decide_less (left right : Int) :
    CompareOp.less.decide left right = Decidable.decide (left < right) := rfl
@[simp] theorem CompareOp.decide_greater (left right : Int) :
    CompareOp.greater.decide left right = Decidable.decide (left > right) := rfl
@[simp] theorem CompareOp.decide_lessEqual (left right : Int) :
    CompareOp.lessEqual.decide left right = Decidable.decide (left ≤ right) := rfl
@[simp] theorem CompareOp.decide_greaterEqual (left right : Int) :
    CompareOp.greaterEqual.decide left right = Decidable.decide (left ≥ right) := rfl

/-- Checked integer arithmetic. -/
inductive CheckedOp where
  | add
  | subtract
  | multiply
  | divide
  | modulo
  deriving DecidableEq, Repr, Inhabited

/-- The checked arithmetic on certified integers.  Division by zero throws
with no payload, as the runtime does; every other failure carries the
out-of-range value. -/
def CheckedOp.run (op : CheckedOp) (failure : ThrowKind) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed) : Comp (SpecInt (.bits width) signed) :=
  match op with
  | .add => checkedInt failure width signed (left.val + right.val)
  | .subtract => checkedInt failure width signed (left.val - right.val)
  | .multiply => checkedInt failure width signed (left.val * right.val)
  | .divide =>
      if right.val = 0 then Spec.abort (failure, #[])
      else checkedInt failure width signed (left.val.tdiv right.val)
  | .modulo =>
      if right.val = 0 then Spec.abort (failure, #[])
      else Spec.bind (checkedInt failure width signed (left.val.tdiv right.val)) fun _ =>
        checkedInt failure width signed (left.val.tmod right.val)

/-- Modular arithmetic: the operations the runtime evaluates with
`modularInteger`, wrapping the mathematical result into the width. -/
inductive ModularOp where
  | add
  | subtract
  | multiply
  deriving DecidableEq, Repr, Inhabited

/-- The mathematical value of a modular operation before wrapping. -/
def ModularOp.eval : ModularOp → Int → Int → Int
  | .add, left, right => left + right
  | .subtract, left, right => left - right
  | .multiply, left, right => left * right

@[simp] theorem ModularOp.eval_add (left right : Int) :
    ModularOp.add.eval left right = left + right := rfl
@[simp] theorem ModularOp.eval_subtract (left right : Int) :
    ModularOp.subtract.eval left right = left - right := rfl
@[simp] theorem ModularOp.eval_multiply (left right : Int) :
    ModularOp.multiply.eval left right = left * right := rfl

/-- The residue of a value in the representable range of a width, as the
runtime's `modularInteger` computes it: the least nonnegative residue
modulo `2 ^ width`, shifted below zero for a signed width. -/
def wrapInt (width : Nat) (signed : Bool) (value : Int) : Int :=
  let modulus : Int := (2 : Int) ^ width
  let residue := ((value % modulus) + modulus) % modulus
  if signed && residue ≥ (2 : Int) ^ (width - 1) then residue - modulus else residue

theorem wrapInt_unsigned (width : Nat) (value : Int) :
    wrapInt width false value =
      ((value % (2 : Int) ^ width) + (2 : Int) ^ width) % (2 : Int) ^ width := by
  simp [wrapInt]

theorem wrapInt_signed (width : Nat) (value : Int) :
    wrapInt width true value =
      if ((value % (2 : Int) ^ width) + (2 : Int) ^ width) % (2 : Int) ^ width ≥
          (2 : Int) ^ (width - 1) then
        ((value % (2 : Int) ^ width) + (2 : Int) ^ width) % (2 : Int) ^ width - (2 : Int) ^ width
      else ((value % (2 : Int) ^ width) + (2 : Int) ^ width) % (2 : Int) ^ width := by
  simp [wrapInt]

theorem residue_bounds (modulus value : Int) (positive : 0 < modulus) :
    0 ≤ ((value % modulus) + modulus) % modulus ∧
      ((value % modulus) + modulus) % modulus < modulus :=
  ⟨Int.emod_nonneg _ (Int.ne_of_gt positive), Int.emod_lt_of_pos _ positive⟩

theorem wrapInt_fits (width : Nat) (signed : Bool) (value : Int) (nonzero : width ≠ 0) :
    IntegerValueFits (.bits width) signed (wrapInt width signed value) := by
  obtain ⟨n, rfl⟩ : ∃ n, width = n + 1 := ⟨width - 1, by omega⟩
  have half : (0 : Int) < 2 ^ n := by
    have := Nat.two_pow_pos n
    have cast : ((2 ^ n : Nat) : Int) = (2 : Int) ^ n := Int.natCast_pow 2 n
    omega
  have modulus : (2 : Int) ^ (n + 1) = 2 * 2 ^ n := by
    rw [Int.pow_succ]; exact Int.mul_comm _ _
  have residue := residue_bounds ((2 : Int) ^ (n + 1)) value (by omega)
  cases signed with
  | false =>
      rw [IntegerValueFits_unsigned_succ, wrapInt_unsigned]
      omega
  | true =>
      rw [IntegerValueFits_signed_succ, wrapInt_signed]
      simp only [Nat.add_sub_cancel]
      split <;> omega

/-- Zero fits every integer width. -/
theorem zero_fits (width : Nat) (signed : Bool) (nonzero : width ≠ 0) :
    IntegerValueFits (.bits width) signed 0 := by
  obtain ⟨n, rfl⟩ : ∃ n, width = n + 1 := ⟨width - 1, by omega⟩
  have half : (0 : Int) < 2 ^ n := by
    have := Nat.two_pow_pos n
    have cast : ((2 ^ n : Nat) : Int) = (2 : Int) ^ n := Int.natCast_pow 2 n
    omega
  have modulus : (2 : Int) ^ (n + 1) = 2 * 2 ^ n := by
    rw [Int.pow_succ]; exact Int.mul_comm _ _
  cases signed with
  | false => rw [IntegerValueFits_unsigned_succ]; omega
  | true => rw [IntegerValueFits_signed_succ]; omega

/-- A modular operation on certified integers, wrapped into their width. -/
def ModularOp.run (op : ModularOp) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed) : SpecInt (.bits width) signed :=
  ⟨wrapInt width signed (op.eval left.val right.val), wrapInt_fits _ _ _ left.width_nonzero⟩

@[simp] theorem ModularOp.run_val (op : ModularOp) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed) :
    (op.run left right).val = wrapInt width signed (op.eval left.val right.val) := rfl

/-- Bitwise operations on unsigned integers. -/
inductive BitOp where
  | and
  deriving DecidableEq, Repr, Inhabited

/-- The mathematical value of a bitwise operation. -/
def BitOp.eval : BitOp → Int → Int → Int
  | .and => IntegerArithmetic.bitwiseAnd

@[simp] theorem BitOp.eval_and (left right : Int) :
    BitOp.and.eval left right = IntegerArithmetic.bitwiseAnd left right := rfl

theorem BitOp.eval_bounds (op : BitOp) {width : Nat}
    (left right : SpecInt (.bits width) false) :
    0 ≤ op.eval left.val right.val ∧ op.eval left.val right.val ≤ 2 ^ width - 1 := by
  obtain ⟨leftLower, leftUpper⟩ := left.unsigned_bounds
  obtain ⟨rightLower, rightUpper⟩ := right.unsigned_bounds
  have pow : ((2 ^ width : Nat) : Int) = (2 : Int) ^ width := Int.natCast_pow 2 width
  cases op
  simp only [BitOp.eval, IntegerArithmetic.bitwiseAnd_nonnegative _ _ leftLower rightLower,
    Int.ofNat_eq_natCast]
  have := Nat.and_le_left (n := left.val.toNat) (m := right.val.toNat)
  omega

/-- Bitwise operation on certified unsigned integers. -/
def BitOp.run (op : BitOp) {width : Nat} (left right : SpecInt (.bits width) false) :
    SpecInt (.bits width) false :=
  SpecInt.ofBounds left (op.eval left.val right.val) (op.eval_bounds left right)

@[simp] theorem BitOp.run_val (op : BitOp) {width : Nat}
    (left right : SpecInt (.bits width) false) :
    (op.run left right).val = op.eval left.val right.val := rfl

theorem shiftLeft_mod_bounds {width : Nat} (value : Int) (distance : Nat)
    (nonzero : width ≠ 0) :
    0 ≤ (Int.shiftLeft value distance) % (2 : Int) ^ width ∧
      (Int.shiftLeft value distance) % (2 : Int) ^ width ≤ 2 ^ width - 1 := by
  have pow : ((2 ^ width : Nat) : Int) = (2 : Int) ^ width := Int.natCast_pow 2 width
  have positiveNat := Nat.two_pow_pos width
  have positive : (0 : Int) < 2 ^ width := by omega
  have := Int.emod_nonneg (Int.shiftLeft value distance) (Int.ne_of_gt positive)
  have := Int.emod_lt_of_pos (Int.shiftLeft value distance) positive
  omega

/-- Truncating quotient of nonnegative operands: nonnegative, at most the dividend. -/
theorem tdiv_bounds_of_nonneg {left right : Int} (leftNonnegative : 0 ≤ left)
    (rightNonnegative : 0 ≤ right) : 0 ≤ left.tdiv right ∧ left.tdiv right ≤ left :=
  ⟨Int.tdiv_nonneg leftNonnegative rightNonnegative, Int.tdiv_le_self right leftNonnegative⟩

/-- Truncating quotient of any operands: its magnitude is at most the
dividend's, it is the dividend or its negation at a unit divisor, and at
most half the dividend's magnitude otherwise.  These decide the range of
a signed quotient: only the minimum divided by `-1` leaves the width. -/
theorem tdiv_facts (left right : Int) :
    (left.tdiv right).natAbs ≤ left.natAbs ∧ (right = 0 → left.tdiv right = 0) ∧
    (right = 1 → left.tdiv right = left) ∧ (right = -1 → left.tdiv right = -left) ∧
    (2 ≤ right.natAbs → 2 * (left.tdiv right).natAbs ≤ left.natAbs) := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · rw [Int.natAbs_tdiv]; exact Nat.div_le_self _ _
  · rintro rfl; simp
  · rintro rfl; simp
  · rintro rfl; simp [Int.tdiv_neg]
  · intro atLeastTwo
    rw [Int.natAbs_tdiv]
    change 2 * (left.natAbs / right.natAbs) ≤ left.natAbs
    have := Nat.div_le_div_left atLeastTwo Nat.zero_lt_two (a := left.natAbs)
    have := Nat.div_mul_le_self left.natAbs 2
    omega

/-- Truncating remainder of any operands: smaller in magnitude than a
nonzero divisor. -/
theorem tmod_facts (left right : Int) : right ≠ 0 → (left.tmod right).natAbs < right.natAbs := by
  intro nonzero
  rw [Int.natAbs_tmod]
  exact Nat.mod_lt _ (Int.natAbs_pos.mpr nonzero)

/-- The range of a signed quotient at its width: it fits, unless it is the
minimum divided by `-1`.  Stated without magnitudes so that a leaf reads
it linearly. -/
theorem tdiv_signed_range {width : Nat} (left right : SpecInt (.bits width) true) :
    (-(2 ^ (width - 1)) ≤ left.val.tdiv right.val ∧
      left.val.tdiv right.val ≤ 2 ^ (width - 1) - 1) ∨
    (left.val = -(2 ^ (width - 1)) ∧ right.val = -1 ∧ left.val.tdiv right.val = 2 ^ (width - 1)) := by
  have facts := tdiv_facts left.val right.val
  have leftBounds := left.signed_bounds
  have rightBounds := right.signed_bounds
  omega

/-- The range of a signed remainder at its width: it always fits. -/
theorem tmod_signed_range {width : Nat} (left right : SpecInt (.bits width) true) :
    -(2 ^ (width - 1)) ≤ left.val.tmod right.val ∧
      left.val.tmod right.val ≤ 2 ^ (width - 1) - 1 := by
  have leftBounds := left.signed_bounds
  have rightBounds := right.signed_bounds
  by_cases zero : right.val = 0
  · rw [zero, Int.tmod_zero]
    exact leftBounds
  · have facts := tmod_facts left.val right.val zero
    omega

/-- Truncating remainder of nonnegative operands: nonnegative, at most the dividend. -/
theorem tmod_bounds_of_nonneg {left right : Int} (leftNonnegative : 0 ≤ left)
    (rightNonnegative : 0 ≤ right) : 0 ≤ left.tmod right ∧ left.tmod right ≤ left := by
  have nonnegative := Int.tmod_nonneg right leftNonnegative
  have product : 0 ≤ right * left.tdiv right :=
    Int.mul_nonneg rightNonnegative (Int.tdiv_nonneg leftNonnegative rightNonnegative)
  have definition := Int.tmod_def left right
  omega

/-- A left shift of a nonnegative value is nonnegative. -/
theorem shiftLeft_nonneg (value : Int) (distance : Nat) (nonnegative : 0 ≤ value) :
    0 ≤ Int.shiftLeft value distance := by
  show 0 ≤ value <<< distance
  rw [Int.shiftLeft_eq]
  exact Int.mul_nonneg nonnegative (Int.pow_nonneg (by decide))

/-- Checked left shift: the distance must be below the width; the result
is the shifted value modulo the width, as the runtime bit pattern. -/
def checkedShiftLeft (failure : ThrowKind) {width distanceWidth : Nat}
    (value : SpecInt (.bits width) false) (distance : SpecInt (.bits distanceWidth) false) :
    Comp (SpecInt (.bits width) false) :=
  if distance.val < width then
    Spec.pure (SpecInt.ofBounds value (Int.shiftLeft value.val distance.val.toNat % 2 ^ width)
      (shiftLeft_mod_bounds _ _ value.width_nonzero))
  else Spec.abort (failure, #[.integer distance.val])

theorem shiftRight_bounds {width : Nat} (value : SpecInt (.bits width) false) (distance : Nat) :
    0 ≤ Int.shiftRight value.val distance ∧ Int.shiftRight value.val distance ≤ 2 ^ width - 1 := by
  obtain ⟨lower, upper⟩ := value.unsigned_bounds
  have nonnegative := @Int.le_shiftRight_of_nonneg value.val distance lower
  have bounded := @Int.shiftRight_le_of_nonneg value.val distance lower
  rw [Int.shiftRight_eq] at nonnegative bounded
  omega

/-- Checked right shift of an unsigned integer. -/
def checkedShiftRight (failure : ThrowKind) {width distanceWidth : Nat}
    (value : SpecInt (.bits width) false) (distance : SpecInt (.bits distanceWidth) false) :
    Comp (SpecInt (.bits width) false) :=
  if distance.val < width then
    Spec.pure (SpecInt.ofBounds value (Int.shiftRight value.val distance.val.toNat)
      (shiftRight_bounds value _))
  else Spec.abort (failure, #[.integer distance.val])

/-! ## Weakest preconditions

One rule per operation, in the vocabulary a clause reads: certificates as
bounds, results by their value. -/

theorem wp_ite (c : Prop) [Decidable c] (left right : Comp α)
    (ensures : α → RuntimeState → Prop) (aborts : Failure → Prop) (state : RuntimeState) :
    wp (if c then left else right) ensures aborts state ↔
      (c → wp left ensures aborts state) ∧ (¬c → wp right ensures aborts state) := by
  by_cases h : c <;> simp [h]

theorem wp_bottom (ensures : α → RuntimeState → Prop) (aborts : Failure → Prop)
    (state : RuntimeState) : wp (Spec.bottom : Comp α) ensures aborts state := by
  simp [wp, Spec.bottom]

theorem wp_checkedInt (failure : ThrowKind) (width : Nat) (signed : Bool) (value : Int)
    (nonzero : width ≠ 0) (ensures : SpecInt (.bits width) signed → RuntimeState → Prop)
    (aborts : Failure → Prop) (state : RuntimeState) :
    wp (checkedInt failure width signed value) ensures aborts state ↔
      (∀ fits : IntegerValueFits (.bits width) signed value, ensures ⟨value, fits⟩ state) ∧
      (¬IntegerValueFits (.bits width) signed value → aborts (failure, #[.integer value])) := by
  rw [checkedInt_eq _ _ _ _ nonzero]
  by_cases fits : IntegerValueFits (.bits width) signed value <;> simp [fits]

theorem wp_checked_add (failure : ThrowKind) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed)
    (ensures : SpecInt (.bits width) signed → RuntimeState → Prop)
    (aborts : Failure → Prop) (state : RuntimeState) :
    wp (CheckedOp.add.run failure left right) ensures aborts state ↔
      (∀ fits : IntegerValueFits (.bits width) signed (left.val + right.val),
        ensures ⟨left.val + right.val, fits⟩ state) ∧
      (¬IntegerValueFits (.bits width) signed (left.val + right.val) →
        aborts (failure, #[.integer (left.val + right.val)])) :=
  wp_checkedInt _ _ _ _ left.width_nonzero _ _ _

theorem wp_checked_subtract (failure : ThrowKind) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed)
    (ensures : SpecInt (.bits width) signed → RuntimeState → Prop)
    (aborts : Failure → Prop) (state : RuntimeState) :
    wp (CheckedOp.subtract.run failure left right) ensures aborts state ↔
      (∀ fits : IntegerValueFits (.bits width) signed (left.val - right.val),
        ensures ⟨left.val - right.val, fits⟩ state) ∧
      (¬IntegerValueFits (.bits width) signed (left.val - right.val) →
        aborts (failure, #[.integer (left.val - right.val)])) :=
  wp_checkedInt _ _ _ _ left.width_nonzero _ _ _

theorem wp_checked_multiply (failure : ThrowKind) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed)
    (ensures : SpecInt (.bits width) signed → RuntimeState → Prop)
    (aborts : Failure → Prop) (state : RuntimeState) :
    wp (CheckedOp.multiply.run failure left right) ensures aborts state ↔
      (∀ fits : IntegerValueFits (.bits width) signed (left.val * right.val),
        ensures ⟨left.val * right.val, fits⟩ state) ∧
      (¬IntegerValueFits (.bits width) signed (left.val * right.val) →
        aborts (failure, #[.integer (left.val * right.val)])) :=
  wp_checkedInt _ _ _ _ left.width_nonzero _ _ _

theorem wp_checked_divide (failure : ThrowKind) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed)
    (ensures : SpecInt (.bits width) signed → RuntimeState → Prop)
    (aborts : Failure → Prop) (state : RuntimeState) :
    wp (CheckedOp.divide.run failure left right) ensures aborts state ↔
      (right.val = 0 → aborts (failure, #[])) ∧
      (right.val ≠ 0 →
        (∀ fits : IntegerValueFits (.bits width) signed (left.val.tdiv right.val),
          ensures ⟨left.val.tdiv right.val, fits⟩ state) ∧
        (¬IntegerValueFits (.bits width) signed (left.val.tdiv right.val) →
          aborts (failure, #[.integer (left.val.tdiv right.val)]))) := by
  simp only [CheckedOp.run, wp_ite, wp_abort, wp_checkedInt _ _ _ _ left.width_nonzero]

theorem wp_checked_modulo (failure : ThrowKind) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed)
    (ensures : SpecInt (.bits width) signed → RuntimeState → Prop)
    (aborts : Failure → Prop) (state : RuntimeState) :
    wp (CheckedOp.modulo.run failure left right) ensures aborts state ↔
      (right.val = 0 → aborts (failure, #[])) ∧
      (right.val ≠ 0 →
        (IntegerValueFits (.bits width) signed (left.val.tdiv right.val) →
          (∀ fits : IntegerValueFits (.bits width) signed (left.val.tmod right.val),
            ensures ⟨left.val.tmod right.val, fits⟩ state) ∧
          (¬IntegerValueFits (.bits width) signed (left.val.tmod right.val) →
            aborts (failure, #[.integer (left.val.tmod right.val)]))) ∧
        (¬IntegerValueFits (.bits width) signed (left.val.tdiv right.val) →
          aborts (failure, #[.integer (left.val.tdiv right.val)]))) := by
  simp only [CheckedOp.run, wp_ite, wp_abort, wp_bind,
    wp_checkedInt _ _ _ _ left.width_nonzero]

theorem wp_checkedShiftLeft (failure : ThrowKind) {width distanceWidth : Nat}
    (value : SpecInt (.bits width) false) (distance : SpecInt (.bits distanceWidth) false)
    (ensures : SpecInt (.bits width) false → RuntimeState → Prop)
    (aborts : Failure → Prop) (state : RuntimeState) :
    wp (checkedShiftLeft failure value distance) ensures aborts state ↔
      (distance.val < width →
        ensures (SpecInt.ofBounds value (Int.shiftLeft value.val distance.val.toNat % 2 ^ width)
          (shiftLeft_mod_bounds _ _ value.width_nonzero)) state) ∧
      (¬distance.val < width → aborts (failure, #[.integer distance.val])) := by
  simp only [checkedShiftLeft, wp_ite, wp_pure, wp_abort]

theorem wp_checkedShiftRight (failure : ThrowKind) {width distanceWidth : Nat}
    (value : SpecInt (.bits width) false) (distance : SpecInt (.bits distanceWidth) false)
    (ensures : SpecInt (.bits width) false → RuntimeState → Prop)
    (aborts : Failure → Prop) (state : RuntimeState) :
    wp (checkedShiftRight failure value distance) ensures aborts state ↔
      (distance.val < width →
        ensures (SpecInt.ofBounds value (Int.shiftRight value.val distance.val.toNat)
          (shiftRight_bounds value _)) state) ∧
      (¬distance.val < width → aborts (failure, #[.integer distance.val])) := by
  simp only [checkedShiftRight, wp_ite, wp_pure, wp_abort]

/-- The checked conversion at a literal nonzero width. -/
theorem wp_checkedInt_succ (failure : ThrowKind) (n : Nat) (signed : Bool) (value : Int)
    (ensures : SpecInt (.bits (n + 1)) signed → RuntimeState → Prop)
    (aborts : Failure → Prop) (state : RuntimeState) :
    wp (checkedInt failure (n + 1) signed value) ensures aborts state ↔
      (∀ fits : IntegerValueFits (.bits (n + 1)) signed value, ensures ⟨value, fits⟩ state) ∧
      (¬IntegerValueFits (.bits (n + 1)) signed value → aborts (failure, #[.integer value])) :=
  wp_checkedInt _ _ _ _ (Nat.succ_ne_zero n) _ _ _

attribute [lir_denote] wp_ite wp_bottom wp_checked_add wp_checked_subtract wp_checked_multiply
  wp_checked_divide wp_checked_modulo wp_checkedShiftLeft wp_checkedShiftRight wp_checkedInt_succ
  Var.get_here Var.get_there Var.set_here Var.set_there SpecInt.ofBounds_val BitOp.run_val
  ModularOp.run_val ModularOp.eval_add ModularOp.eval_subtract ModularOp.eval_multiply
  foundIndex_val foundIndexOf reverseRange_zero
  wrapInt_unsigned wrapInt_signed
  BitOp.eval_and CompareOp.decide_less CompareOp.decide_greater CompareOp.decide_lessEqual
  CompareOp.decide_greaterEqual NTy.eqb_int NTy.eqb_bool NTy.eqb_address NTy.eqb_unit
  NTy.eqb_param
  IntegerValueFits_unsigned_succ IntegerValueFits_signed_succ initialEnv

/-! ## State operations

The denotation reads and updates the runtime state through `Spec.get` and
`Spec.modify`; each has one weakest-precondition rule. -/

theorem wp_get {σ ε : Type} (ensures : σ → σ → Prop) (aborts : ε → Prop) (state : σ) :
    wp (Spec.get : Spec σ ε σ) ensures aborts state ↔ ensures state state := by
  constructor
  · intro h; exact h.1 state state ⟨rfl, rfl⟩
  · intro h
    refine ⟨fun result final ⟨r, f⟩ => ?_, fun _ h => h.elim, fun h => h⟩
    subst r; subst f; exact h

theorem wp_modify {σ ε : Type} (f : σ → σ) (ensures : Unit → σ → Prop) (aborts : ε → Prop)
    (state : σ) : wp (Spec.modify f : Spec σ ε Unit) ensures aborts state ↔ ensures () (f state) := by
  constructor
  · intro h; exact h.1 () (f state) ⟨rfl, rfl⟩
  · intro h
    refine ⟨fun result final ⟨r, fEq⟩ => ?_, fun _ h => h.elim, fun h => h⟩
    subst r; subst fEq; exact h

theorem wp_set {σ ε : Type} (next : σ) (ensures : Unit → σ → Prop) (aborts : ε → Prop)
    (state : σ) : wp (Spec.set next : Spec σ ε Unit) ensures aborts state ↔ ensures () next := by
  constructor
  · intro h; exact h.1 () next ⟨rfl, rfl⟩
  · intro h
    refine ⟨fun result final ⟨r, fEq⟩ => ?_, fun _ h => h.elim, fun h => h⟩
    subst r; subst fEq; exact h

attribute [lir_denote] wp_get wp_modify wp_set

/-- A resource family: the declaring namespace and the resource type. -/
structure Family where
  namespaceId : NamespaceId
  typeId : TypeId
  deriving DecidableEq, Repr, Inhabited

/-- The storage key of a family at a key value. -/
def Family.key (family : Family) (key : RuntimeValue) : GlobalKey :=
  LeanerIR.SemanticOperations.globalKey family.namespaceId family.typeId key

/-- A family of a generic frame at its type instantiation, as the runtime
keys it. -/
def Family.instantiate (typeInstantiation : Array (TypeId × TypeId)) (family : Family) : Family :=
  { family with typeId := instantiatedTypeId typeInstantiation family.typeId }

@[simp] theorem Family.instantiate_empty (family : Family) : family.instantiate #[] = family := by
  simp [Family.instantiate]

/-- An instantiation given as a literal is looked up entry by entry. -/
theorem instantiatedTypeId_cons (entry : TypeId × TypeId) (rest : List (TypeId × TypeId))
    (typeId : TypeId) :
    instantiatedTypeId (entry :: rest).toArray typeId =
      if entry.1 = typeId then entry.2 else instantiatedTypeId rest.toArray typeId := by
  unfold instantiatedTypeId
  obtain ⟨⟨source⟩, target⟩ := entry
  obtain ⟨index⟩ := typeId
  have beq : ((⟨source⟩ : TypeId) == ⟨index⟩) = (source == index) := rfl
  by_cases same : source = index
  · subst same
    simp [List.find?_cons, beq]
  · simp [List.find?_cons, beq, same]

theorem instantiatedTypeId_nil (typeId : TypeId) :
    instantiatedTypeId ([] : List (TypeId × TypeId)).toArray typeId = typeId :=
  instantiatedTypeId_empty typeId

@[simp] theorem Family.instantiate_namespaceId (typeInstantiation : Array (TypeId × TypeId))
    (family : Family) : (family.instantiate typeInstantiation).namespaceId = family.namespaceId :=
  rfl
@[simp] theorem Family.instantiate_typeId (typeInstantiation : Array (TypeId × TypeId))
    (family : Family) :
    (family.instantiate typeInstantiation).typeId =
      instantiatedTypeId typeInstantiation family.typeId :=
  rfl

attribute [lir_denote] Family.instantiate_empty Family.instantiate_namespaceId
  Family.instantiate_typeId

@[simp] theorem Family.key_eq (family : Family) (key : RuntimeValue) :
    family.key key = ⟨family.namespaceId, family.typeId, key.storageKey⟩ := rfl
@[simp] theorem storageKey_address (address : String) :
    (RuntimeValue.address address).storageKey = .address address := rfl
@[simp] theorem storageKey_signer (address : String) :
    (RuntimeValue.signer address).storageKey = .address address := rfl


/-- The write-backs a callee appended past the pending set it inherited. -/
def exportsAfter (inherited final : Array (Nat × RuntimeValue)) : List (Nat × RuntimeValue) :=
  final.toList.drop inherited.size

@[simp] theorem exportsAfter_self (pending : Array (Nat × RuntimeValue)) :
    exportsAfter pending pending = [] := by
  simp [exportsAfter]

@[simp] theorem exportsAfter_push (pending : Array (Nat × RuntimeValue))
    (entry : Nat × RuntimeValue) : exportsAfter pending (pending.push entry) = [entry] := by
  simp [exportsAfter]

@[simp] theorem exportsAfter_push_push (pending : Array (Nat × RuntimeValue))
    (first second : Nat × RuntimeValue) :
    exportsAfter pending ((pending.push first).push second) = [first, second] := by
  simp [exportsAfter]

/-- The runtime value a callee exported for a loan. -/
def exportedRaw? (loan : Nat) : List (Nat × RuntimeValue) → Option RuntimeValue
  | [] => none
  | (exported, value) :: rest => if exported = loan then some value else exportedRaw? loan rest

@[simp] theorem exportedRaw?_nil (loan : Nat) : exportedRaw? loan [] = none := rfl
@[simp] theorem exportedRaw?_cons (loan exported : Nat) (value : RuntimeValue)
    (rest : List (Nat × RuntimeValue)) :
    exportedRaw? loan ((exported, value) :: rest) =
      if exported = loan then some value else exportedRaw? loan rest := rfl

/-- The native value a runtime value encodes at a type: any value whose
encoding is the runtime value, undefined when none decodes.  A caller
reasons about a callee's export through this relation, never through a
decoder's certificate. -/
def decodeOr {α : Type} (codec : Codec α RuntimeValue) (runtime : RuntimeValue) : Comp α where
  ok := fun initial value final => codec.encode value = runtime ∧ final = initial
  aborts := fun _ _ => False
  undefined := fun _ => (codec.decode? runtime).isNone

theorem wp_decodeOr {α : Type} (codec : Codec α RuntimeValue) (runtime : RuntimeValue)
    (ensures : α → RuntimeState → Prop) (aborts : Failure → Prop) (state : RuntimeState) :
    wp (decodeOr codec runtime) ensures aborts state ↔
      (∀ value, codec.encode value = runtime → ensures value state) ∧
        (codec.decode? runtime).isSome = true := by
  unfold wp decodeOr
  constructor
  · intro h
    refine ⟨fun value encoded => h.1 value state ⟨encoded, rfl⟩, ?_⟩
    have := h.2.2
    simp only [Option.isNone_iff_eq_none] at this
    exact Option.isSome_iff_ne_none.mpr this
  · intro h
    refine ⟨fun value final ⟨encoded, equal⟩ => ?_, fun _ h => h.elim, ?_⟩
    · subst equal; exact h.1 value encoded
    · simp only [Option.isNone_iff_eq_none]
      exact Option.isSome_iff_ne_none.mp h.2

@[simp] theorem Option.isSome_dite_some {p : Prop} [Decidable p] {α : Type} (f : p → α) :
    (if h : p then some (f h) else none).isSome = decide p := by
  by_cases h : p <;> simp [h]

attribute [lir_denote] wp_decodeOr Option.isSome_dite_some exportedRaw?_nil exportedRaw?_cons

/-- Pushing is injective in both the array and the pushed entry. -/
theorem Array.push_eq_push_iff {α : Type} (left right : Array α) (x y : α) :
    (left.push x = right.push y) ↔ left = right ∧ x = y := by
  constructor
  · intro equal
    have arrays := congrArg Array.pop equal
    have entries := congrArg (fun (a : Array α) => a[a.size - 1]?) equal
    simp at arrays entries
    exact ⟨arrays, entries⟩
  · intro ⟨arrays, entries⟩
    rw [arrays, entries]

@[simp] theorem specInt_encode (width : IntWidth) (signed : Bool) (value : SpecInt width signed) :
    (Codec.specInt width signed).encode value = .integer value.val := rfl
@[simp] theorem bool_encode (value : Bool) : Codec.bool.encode value = .bool value := rfl
@[simp] theorem address_encode (value : String) : Codec.address.encode value = .address value := rfl

@[simp] theorem NTy.codec_int [Skolems] (width : Nat) (signed : Bool) :
    (NTy.int width signed).codec = Codec.specInt (.bits width) signed := rfl
@[simp] theorem NTy.codec_bool [Skolems] : NTy.bool.codec = Codec.bool := rfl
@[simp] theorem NTy.codec_address [Skolems] : NTy.address.codec = Codec.address := rfl
@[simp] theorem NTy.codec_unit [Skolems] : NTy.unit.codec = Codec.unit := rfl
@[simp] theorem NTy.codec_ref [Skolems] (referent : NTy) :
    (NTy.ref referent).codec = Codec.prophecyPair referent.codec := rfl

/-- Decoding a runtime integer at a certified width: the value, when it fits. -/
theorem specInt_decode_integer (width : IntWidth) (signed : Bool) (value : Int) :
    (Codec.specInt width signed).decode? (.integer value) =
      if fits : IntegerValueFits width signed value then some ⟨value, fits⟩ else none := rfl

/-- Decoding a runtime boolean or address: the value itself. -/
theorem bool_decode_bool (value : Bool) : Codec.bool.decode? (.bool value) = some value := rfl

theorem address_decode_address (value : String) :
    Codec.address.decode? (.address value) = some value := rfl

theorem signer_decode_signer (value : String) :
    Codec.signer.decode? (.signer value) = some value := rfl

/-- An encoding equation read as a decoding: the runtime value a native
value encodes to decodes back to it, so a literal encoding names the value. -/
theorem NTy.decode?_of_encode [Skolems] (τ : NTy) {value : τ.carrier} {raw : RuntimeValue}
    (encoded : τ.encode value = raw) : τ.codec.decode? raw = some value :=
  encoded ▸ τ.codec.decode_encode value

/-- The same, once an encoding of a vector has been split element-wise. -/
theorem NTy.decode?_vector_of_map [Skolems] (τ : NTy) {value : SpecVector τ.carrier} {raw : Array RuntimeValue}
    (encoded : value.values.map τ.encode = raw) :
    (NTy.vector τ).codec.decode? (.vector raw) = some value :=
  NTy.decode?_of_encode (.vector τ) (congrArg RuntimeValue.vector encoded)

/-- An array is the array of its list. -/
theorem array_eq_of_toList_eq {α : Type} {xs : Array α} {l : List α} (h : xs.toList = l) :
    xs = l.toArray :=
  Array.toList_inj.mp (h.trans (List.toList_toArray (as := l)).symm)

@[simp] theorem NTy.codec_vector [Skolems] (element : NTy) :
    (NTy.vector element).codec = Codec.boundedVector element.codec := rfl

/-- Decoding the elements of a vector one by one, as explicit binds. -/
def decodeElements? (codec : Codec Native RuntimeValue) : List RuntimeValue → Option (List Native)
  | [] => some []
  | value :: values =>
      (codec.decode? value).bind fun head => (decodeElements? codec values).map fun tail => head :: tail

@[simp] theorem decodeElements?_nil (codec : Codec Native RuntimeValue) :
    decodeElements? codec [] = some [] := rfl
@[simp] theorem decodeElements?_cons (codec : Codec Native RuntimeValue) (value : RuntimeValue)
    (values : List RuntimeValue) :
    decodeElements? codec (value :: values) =
      (codec.decode? value).bind fun head =>
        (decodeElements? codec values).map fun tail => head :: tail := rfl

theorem mapM_eq_decodeElements? (codec : Codec Native RuntimeValue) (values : List RuntimeValue) :
    values.mapM codec.decode? = decodeElements? codec values := by
  induction values with
  | nil => rfl
  | cons value values ih =>
      rw [List.mapM_cons, ih, decodeElements?_cons]
      rcases codec.decode? value with _ | head <;>
        rcases decodeElements? codec values with _ | tail <;> rfl

/-- Decoding a vector literal: its elements, then the bound. -/
theorem boundedVector_decode?_vector (codec : Codec Native RuntimeValue) (values : Array RuntimeValue) :
    (Codec.boundedVector codec).decode? (.vector values) =
      (decodeElements? codec values.toList).bind fun decoded =>
        if bounded : decoded.length < 2 ^ 64 then
          some ⟨decoded.toArray, by simpa only [List.size_toArray] using bounded⟩
        else none := by
  show (values.toList.mapM codec.decode? >>= fun decoded => _) = _
  rw [mapM_eq_decodeElements?]
  rcases decodeElements? codec values.toList with _ | decoded <;> rfl

theorem toArray_inj_iff {α : Type} (as bs : List α) : as.toArray = bs.toArray ↔ as = bs :=
  ⟨List.toArray_inj, fun equal => equal ▸ rfl⟩

theorem SpecInt.val_ne_of_ne {width : IntWidth} {signed : Bool} {a b : SpecInt width signed}
    (different : a ≠ b) : a.val ≠ b.val := fun h => different (SpecInt.ext h)

theorem SpecVector.values_ne_of_ne {α : Type} {a b : SpecVector α} (different : a ≠ b) :
    a.values ≠ b.values := fun h => different (SpecVector.ext h)

/-- A frontier never equals itself advanced. -/
@[simp] theorem nat_self_eq_add_iff (n m : Nat) : (n = n + m) ↔ m = 0 := by omega
@[simp] theorem nat_add_eq_self_iff (n m : Nat) : (n + m = n) ↔ m = 0 := by omega

/-- A minted loan differs from every later one. -/
@[simp] theorem nat_eq_add_succ_iff (n k : Nat) : (n = n + (k + 1)) ↔ False :=
  iff_false_intro (by omega)
@[simp] theorem nat_add_succ_eq_iff (n k : Nat) : (n + (k + 1) = n) ↔ False :=
  iff_false_intro (by omega)
@[simp] theorem nat_eq_succ_iff (n : Nat) : (n = n + 1) ↔ False := iff_false_intro (by omega)
@[simp] theorem nat_succ_eq_iff (n : Nat) : (n + 1 = n) ↔ False := iff_false_intro (by omega)

/-- A codec's encoding at a type is the type's encoding. -/
@[simp] theorem NTy.codec_encode [Skolems] (τ : NTy) (value : τ.carrier) :
    τ.codec.encode value = τ.encode value := rfl

/-- Encodings at one type are equal exactly when the values are. -/
@[simp] theorem NTy.encode_inj [Skolems] (τ : NTy) (left right : τ.carrier) :
    (τ.encode left = τ.encode right) ↔ left = right :=
  ⟨τ.encode_injective, fun equal => equal ▸ rfl⟩

/-- Encoded rows are equal exactly when the rows are. -/
@[simp] theorem HList.encode_inj [Skolems] {Γ : NRow} (left right : HList Γ) :
    (HList.encode left = HList.encode right) ↔ left = right :=
  ⟨fun equal => (rowCodec Γ).encode_injective equal, fun equal => equal ▸ rfl⟩

/-- Decoding a nominal literal at a struct type: the row decodes the fields. -/
@[simp] theorem NTy.decode?_struct_nominal [Skolems] (source : StructHandle) (fields : NRow)
    (values : Array RuntimeValue) :
    (NTy.struct source fields).codec.decode? (.nominal source none values) =
      (rowCodec fields).decode? values.toList := by
  simp [NTy.codec, Codec.nominalRow]

/-- Decoding a row literal, component by component. -/
@[simp] theorem rowCodec_decode?_cons [Skolems] (τ : NTy) (rest : NRow) (value : RuntimeValue)
    (values : List RuntimeValue) :
    (rowCodec (.cons τ rest)).decode? (value :: values) =
      (τ.codec.decode? value).bind fun head =>
        ((rowCodec rest).decode? values).bind fun tail =>
          (some (head, tail) : Option (HList (.cons τ rest))) := rfl

@[simp] theorem rowCodec_decode?_nil [Skolems] :
    (rowCodec .nil).decode? [] = @some (HList .nil) () := rfl

/-- Decoding the unfolded encoding of a struct value. -/
@[simp] theorem NTy.decode?_struct_literal [Skolems] (source : StructHandle) (fields : NRow)
    (value : HList fields) :
    (NTy.struct source fields).codec.decode? (.nominal source none value.encode.toArray) =
      some value := (NTy.struct source fields).codec.decode_encode value

/-- Decoding a nominal literal at an enum type, variant by variant: the
named variant decodes its row, any other is looked up among the later
variants. -/
@[simp] theorem NTy.decode?_enum_cons_nominal [Skolems] (source : StructHandle) (name : String)
    (names : List String) (fields : NRow) (rest : NRows) (distinct : (name :: names).Nodup)
    (variant : String) (values : Array RuntimeValue) :
    (NTy.enum source (name :: names) (.cons fields rest) distinct).codec.decode?
        (.nominal source (some variant) values) =
      if variant = name then
        ((rowCodec fields).decode? values.toList).map variantCarrier.first
      else ((NTy.enum source names rest (List.nodup_cons.mp distinct).2).codec.decode?
          (.nominal source (some variant) values)).map variantCarrier.later := by
  by_cases equal : variant = name <;>
    simp [NTy.codec, variantCodec, variantDecode?, Codec.nominalRow, equal,
      variantCarrier.first, variantCarrier.later] <;> rfl

/-- No literal decodes at an enum type without variants. -/
@[simp] theorem NTy.decode?_enum_nil [Skolems] (source : StructHandle) (names : List String)
    (distinct : names.Nodup) (runtime : RuntimeValue) :
    (NTy.enum source names .nil distinct).codec.decode? runtime = none := by
  cases names <;> rfl

/-- Decoding an integer literal at its own width, in the form the integer
codec unfolds to. -/
@[simp] theorem Codec.specInt_decode?_val (width : IntWidth) (signed : Bool)
    (value : SpecInt width signed) :
    (Codec.specInt width signed).decode? (.integer value.val) = some value :=
  (Codec.specInt width signed).decode_encode value

/-- Decoding the unfolded encoding of a tuple value. -/
@[simp] theorem NTy.decode?_tuple_literal [Skolems] (elements : NRow) (value : HList elements) :
    (NTy.tuple elements).codec.decode? (.tuple value.encode.toArray) = some value :=
  (NTy.tuple elements).codec.decode_encode value

/-- Decoding an encoded value at its own type. -/
@[simp] theorem NTy.decode?_encode [Skolems] (τ : NTy) (value : τ.carrier) :
    τ.codec.decode? (τ.encode value) = some value := τ.codec.decode_encode value

/-! ## Tightness of the codecs -/

theorem specInt_tight (width : IntWidth) (signed : Bool) : (Codec.specInt width signed).Tight := by
  intro raw value h
  cases raw <;> simp only [Codec.specInt, decodeInt?, reduceCtorEq] at h
  split at h
  · cases h; rfl
  · cases h

theorem bool_tight : Codec.bool.Tight := by
  intro raw value h; cases raw <;> simp only [Codec.bool, decodeBool?, reduceCtorEq] at h; cases h; rfl
theorem string_tight : Codec.string.Tight := by
  intro raw value h; cases raw <;> simp only [Codec.string, decodeString?, reduceCtorEq] at h; cases h; rfl
theorem address_tight : Codec.address.Tight := by
  intro raw value h; cases raw <;> simp only [Codec.address, decodeAddress?, reduceCtorEq] at h; cases h; rfl
theorem signer_tight : Codec.signer.Tight := by
  intro raw value h; cases raw <;> simp only [Codec.signer, decodeSigner?, reduceCtorEq] at h; cases h; rfl
theorem bytes_tight : Codec.bytes.Tight := by
  intro raw value h; cases raw <;> simp only [Codec.bytes, decodeBytes?, reduceCtorEq] at h; cases h; rfl
theorem unit_tight : Codec.unit.Tight := by
  intro raw value h; cases raw <;> simp only [Codec.unit, decodeUnit?, reduceCtorEq] at h; rfl

theorem tupleNil_tight : Codec.tupleNil.Tight := by
  intro raw value h
  cases raw <;> simp only [Codec.tupleNil, reduceCtorEq] at h
  rfl

theorem tupleCons_tight {Head Tail : Type} {head : Codec Head RuntimeValue}
    {tail : Codec Tail (List RuntimeValue)} (headTight : head.Tight) (tailTight : tail.Tight) :
    (Codec.tupleCons head tail).Tight := by
  intro raw value h
  cases raw with
  | nil => simp [Codec.tupleCons] at h
  | cons first rest =>
      simp only [Codec.tupleCons, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
        Option.some.injEq] at h
      obtain ⟨dh, hh, dt, ht, rfl⟩ := h
      simp only [Codec.tupleCons, headTight first dh hh, tailTight rest dt ht]

theorem tuple_tight {Native : Type} {row : Codec Native (List RuntimeValue)} (rowTight : row.Tight) :
    (Codec.tuple row).Tight := by
  intro raw value h
  cases raw <;> simp only [Codec.tuple, reduceCtorEq] at h
  simp only [Codec.tuple, rowTight _ _ h, Array.toArray_toList]

theorem nominalRow_tight {Native : Type} (source : StructHandle) (variant : Option String)
    {row : Codec Native (List RuntimeValue)} (rowTight : row.Tight) :
    (Codec.nominalRow source variant row).Tight := by
  intro raw value h
  cases raw <;> simp only [Codec.nominalRow, reduceCtorEq] at h
  split at h
  · rename_i equal
    obtain ⟨rfl, rfl⟩ := equal
    simp only [Codec.nominalRow, rowTight _ _ h, Array.toArray_toList]
  · cases h

theorem prophecyPair_tight {Native : Type} {codec : Codec Native RuntimeValue}
    (tight : codec.Tight) : (Codec.prophecyPair codec).Tight := by
  intro raw value h
  cases raw with
  | tuple values =>
      simp only [Codec.prophecyPair] at h
      split at h
      · rename_i current prophecy listed
        simp only [Option.bind_eq_some_iff, Option.map_eq_some_iff] at h
        obtain ⟨c, hc, p, hp, rfl⟩ := h
        simp only [Codec.prophecyPair, tight _ _ hc, tight _ _ hp]
        exact congrArg RuntimeValue.tuple (Array.toList_inj.mp (by simp [listed]))
      · cases h
  | _ => simp [Codec.prophecyPair] at h

theorem decodeElements?_tight {Native : Type} {codec : Codec Native RuntimeValue} (tight : codec.Tight) :
    ∀ (values : List RuntimeValue) (decoded : List Native),
      decodeElements? codec values = some decoded → decoded.map codec.encode = values := by
  intro values
  induction values with
  | nil => intro decoded h; simp only [decodeElements?_nil, Option.some.injEq] at h; subst h; rfl
  | cons value rest ih =>
      intro decoded h
      simp only [decodeElements?_cons, Option.bind_eq_some_iff, Option.map_eq_some_iff] at h
      obtain ⟨head, hh, tail, ht, rfl⟩ := h
      simp only [List.map_cons, tight _ _ hh, ih tail ht]

theorem boundedVector_tight {Native : Type} {codec : Codec Native RuntimeValue} (tight : codec.Tight) :
    (Codec.boundedVector codec).Tight := by
  intro raw value h
  cases raw
  case vector values =>
    rw [boundedVector_decode?_vector] at h
    simp only [Option.bind_eq_some_iff] at h
    obtain ⟨decoded, hd, hv⟩ := h
    split at hv
    · cases hv
      simp only [Codec.boundedVector, List.map_toArray, decodeElements?_tight tight _ _ hd,
        Array.toArray_toList]
    · cases hv
  all_goals simp [Codec.boundedVector] at h

/-- Tightness of the row codecs of an enum's variants. -/
def RowCodecsTight [Skolems] : (rows : NRows) → RowCodecs rows → Prop
  | .nil, _ => True
  | .cons _ rest, (codec, codecs) => codec.Tight ∧ RowCodecsTight rest codecs

theorem variantDecode?_tight [Skolems] (source : StructHandle) : (names : List String) → (rows : NRows) →
    (codecs : RowCodecs rows) → RowCodecsTight rows codecs →
    ∀ raw value, variantDecode? source names rows codecs raw = some value →
      variantEncode source names rows codecs value = raw
  | _, .nil, _, _, _, value, _ => nomatch value
  | [], .cons _ _, _, _, _, value, _ => nomatch value
  | name :: names, .cons fields rest, (codec, codecs), ⟨tight, tights⟩, raw, value, h => by
      unfold variantDecode? at h
      split at h
      · rename_i actualSource actualVariant runtimeFields
        split at h
        · rename_i equal
          obtain ⟨rfl, rfl⟩ := equal
          simp only [Option.map_eq_some_iff] at h
          obtain ⟨decoded, hd, rfl⟩ := h
          exact nominalRow_tight _ _ tight _ _ hd
        · simp only [Option.map_eq_some_iff] at h
          obtain ⟨decoded, hd, rfl⟩ := h
          exact variantDecode?_tight source names rest codecs tights _ _ hd
      · cases h

mutual
theorem NTy.codec_tight [Skolems] : (τ : NTy) → τ.codec.Tight
  | .unit => unit_tight
  | .bool => bool_tight
  | .int width signed => specInt_tight (.bits width) signed
  | .address => address_tight
  | .signer => signer_tight
  | .string => string_tight
  | .bytes => bytes_tight
  | .tuple elements => tuple_tight (rowCodec_tight elements)
  | .struct source fields => nominalRow_tight source none (rowCodec_tight fields)
  | .enum source names rows _ => fun raw value h =>
      variantDecode?_tight source names rows (rowCodecs rows) (rowCodecs_tight rows) raw value h
  | .vector element => boundedVector_tight (NTy.codec_tight element)
  | .ref referent => prophecyPair_tight (NTy.codec_tight referent)
  | .param index => Skolems.tight index

theorem rowCodec_tight [Skolems] : (row : NRow) → (rowCodec row).Tight
  | .nil => tupleNil_tight
  | .cons τ rest => tupleCons_tight (NTy.codec_tight τ) (rowCodec_tight rest)

theorem rowCodecs_tight [Skolems] : (rows : NRows) → RowCodecsTight rows (rowCodecs rows)
  | .nil => trivial
  | .cons fields rest => ⟨rowCodec_tight fields, rowCodecs_tight rest⟩
end

/-- An encoding equation is a decoding equation: the codecs are tight. -/
theorem NTy.encode_eq_iff [Skolems] (τ : NTy) (value : τ.carrier) (raw : RuntimeValue) :
    τ.encode value = raw ↔ τ.codec.decode? raw = some value :=
  ⟨τ.decode?_of_encode, τ.codec_tight raw value⟩


section Equality
variable [Skolems]

/-! ## Structural equality decides equality -/

mutual
/-- Whether a type holds no reference: structural equality of its values
then decides their equality. -/
def NTy.refFree : NTy → Bool
  | .ref _ => false
  | .tuple elements => elements.refFree
  | .struct _ fields => fields.refFree
  | .enum _ _ rows _ => rows.refFree
  | .vector element => element.refFree
  | _ => true

def NRow.refFree : NRow → Bool
  | .nil => true
  | .cons τ rest => τ.refFree && rest.refFree

def NRows.refFree : NRows → Bool
  | .nil => true
  | .cons fields rest => fields.refFree && rest.refFree
end

omit [Skolems] in
theorem eqb_zip_iff {α : Type} (eqb : α → α → Bool) (sound : ∀ a b, eqb a b = true ↔ a = b) :
    ∀ (l r : List α), ((l.length == r.length) && (l.zip r).all fun p => eqb p.1 p.2) = true ↔ l = r
  | [], [] => by simp
  | [], _ :: _ => by simp
  | _ :: _, [] => by simp
  | a :: l, b :: r => by
      have ih := eqb_zip_iff eqb sound l r
      simp only [List.length_cons, List.zip_cons_cons, List.all_cons, Bool.and_eq_true, beq_iff_eq,
        Nat.add_right_cancel_iff, List.cons.injEq, sound] at ih ⊢
      constructor
      · rintro ⟨hlen, hab, hall⟩
        exact ⟨hab, ih.mp ⟨hlen, hall⟩⟩
      · rintro ⟨rfl, rfl⟩
        exact ⟨rfl, rfl, (ih.mpr rfl).2⟩

mutual
theorem NTy.eqb_iff : (τ : NTy) → τ.refFree = true → ∀ l r : τ.carrier, τ.eqb l r = true ↔ l = r
  | .unit, _, l, r => by simp [NTy.eqb]
  | .bool, _, l, r => by simp [NTy.eqb]
  | .int width signed, _, l, r => by
      simp only [NTy.eqb, decide_eq_true_eq]
      exact ⟨fun h => SpecInt.ext h, fun h => h ▸ rfl⟩
  | .address, _, l, r => by simp [NTy.eqb]
  | .signer, _, l, r => by simp [NTy.eqb]
  | .string, _, l, r => by simp [NTy.eqb]
  | .bytes, _, l, r => by simp [NTy.eqb]
  | .tuple elements, free, l, r => rowEqb_iff elements free l r
  | .struct _ fields, free, l, r => rowEqb_iff fields free l r
  | .enum _ names rows _, free, l, r => variantEqb_iff names rows free l r
  | .vector element, free, l, r => by
      rw [NTy.eqb_vector]
      have sound := NTy.eqb_iff element free
      constructor
      · intro h
        have := (eqb_zip_iff element.eqb sound l.values.toList r.values.toList).mp (by
          simpa only [Array.length_toList] using h)
        exact SpecVector.ext (Array.toList_inj.mp this)
      · rintro rfl
        simpa only [Array.length_toList] using
          (eqb_zip_iff element.eqb sound l.values.toList l.values.toList).mpr rfl
  | .ref _, free, _, _ => by simp [NTy.refFree] at free
  | .param _, _, l, r => by simp [NTy.eqb]

theorem rowEqb_iff : (row : NRow) → row.refFree = true → ∀ l r : HList row, rowEqb row l r = true ↔ l = r
  | .nil, _, l, r => by
      cases l; cases r; simp [rowEqb]
  | .cons τ rest, free, l, r => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      obtain ⟨l1, l2⟩ := l
      obtain ⟨r1, r2⟩ := r
      simp only [rowEqb, Bool.and_eq_true, NTy.eqb_iff τ free.1, rowEqb_iff rest free.2]
      constructor
      · rintro ⟨rfl, rfl⟩; rfl
      · intro h; injection h with a b; exact ⟨a, b⟩

theorem variantEqb_iff : (names : List String) → (rows : NRows) → rows.refFree = true →
    ∀ l r : variantCarrier names rows, variantEqb names rows l r = true ↔ l = r
  | _, .nil, _, l, _ => nomatch l
  | [], .cons _ _, _, l, _ => nomatch l
  | _ :: names, .cons fields rest, free, l, r => by
      simp only [NRows.refFree, Bool.and_eq_true] at free
      cases l <;> cases r <;>
        simp only [variantEqb, rowEqb_iff fields free.1, variantEqb_iff names rest free.2,
          Bool.false_eq_true, reduceCtorEq] <;>
        (constructor <;> intro h) <;> first | (subst h; rfl) | (injection h)
end


/-- Structural equality of vectors over a reference-free element type
decides equality; a spec literal then meets a symbolic vector as a
decoding. -/
theorem NTy.eqb_vector_decide (element : NTy) (free : element.refFree = true)
    (left right : SpecVector element.carrier) :
    NTy.eqb (.vector element) left right =
      @decide (left = right) (NTy.decEq (.vector element) left right) := by
  have := NTy.eqb_iff (.vector element) free left right
  by_cases equal : left = right
  · rw [@decide_eq_true _ (NTy.decEq (.vector element) left right) equal]; exact this.mpr equal
  · rw [@decide_eq_false _ (NTy.decEq (.vector element) left right) equal]
    exact Bool.eq_false_iff.mpr (fun h => equal (this.mp h))

/-- A value that does not encode to a runtime value is not what it decodes to. -/
theorem NTy.decode?_ne_of_encode_ne (τ : NTy) {value : τ.carrier} {raw : RuntimeValue}
    (different : τ.encode value ≠ raw) : τ.codec.decode? raw ≠ some value :=
  fun decoded => different (τ.codec_tight raw value decoded)

theorem NTy.decode?_vector_ne_of_map_ne (τ : NTy) {value : SpecVector τ.carrier}
    {raw : Array RuntimeValue} (different : value.values.map τ.codec.encode ≠ raw) :
    (NTy.vector τ).codec.decode? (.vector raw) ≠ some value :=
  fun decoded => different (RuntimeValue.vector.inj ((NTy.vector τ).codec_tight (.vector raw) value decoded))


end Equality

/-! ## Instantiation

A generic call carries its type arguments as a row: the callee's
parameter `i` is the caller's `i`-th argument.  The callee's meaning is
taken at the family they induce, and values cross the call through the
transports between the two views of one runtime value. -/

/-- The `index`-th type of a row, or `default` beyond it. -/
@[reducible] def NRow.getD : NRow → Nat → NTy → NTy
  | .nil, _, default => default
  | .cons τ _, 0, _ => τ
  | .cons _ rest, index + 1, default => rest.getD index default

mutual
/-- A type with its parameters replaced by type arguments. -/
@[reducible] def NTy.subst (θ : NRow) : NTy → NTy
  | .tuple elements => .tuple (NRow.subst θ elements)
  | .struct source fields => .struct source (NRow.subst θ fields)
  | .enum source names rows distinct => .enum source names (rows.subst θ) distinct
  | .vector element => .vector (element.subst θ)
  | .ref referent => .ref (referent.subst θ)
  | .param index => θ.getD index (.param index)
  | τ => τ

@[reducible] def NRow.subst (θ : NRow) : NRow → NRow
  | .nil => .nil
  | .cons τ rest => .cons (τ.subst θ) (NRow.subst θ rest)

@[reducible] def NRows.subst (θ : NRow) : NRows → NRows
  | .nil => .nil
  | .cons fields rest => .cons (NRow.subst θ fields) (rest.subst θ)
end

mutual
/-- Whether a type mentions no type parameter. -/
def NTy.paramFree : NTy → Bool
  | .param _ => false
  | .tuple elements => elements.paramFree
  | .struct _ fields => fields.paramFree
  | .enum _ _ rows _ => rows.paramFree
  | .vector element => element.paramFree
  | .ref referent => referent.paramFree
  | _ => true

def NRow.paramFree : NRow → Bool
  | .nil => true
  | .cons τ rest => τ.paramFree && rest.paramFree

def NRows.paramFree : NRows → Bool
  | .nil => true
  | .cons fields rest => fields.paramFree && rest.paramFree
end

/-- A shape with its parameters replaced by type arguments. -/
@[reducible] def ResultShape.subst (θ : NRow) : ResultShape → ResultShape
  | .none => .none
  | .one τ => .one (τ.subst θ)

section Plain
variable [Skolems]

mutual
/-- Every encoding is loan-free: a reference's codec encodes its current
value and prophecy as a pair. -/
theorem NTy.encode_plain : (τ : NTy) → (value : τ.carrier) → Plain (τ.codec.encode value)
  | .unit, _ => .unit
  | .bool, value => .bool value
  | .int _ _, value => .integer value.val
  | .address, value => .address value
  | .signer, value => .signer value
  | .string, value => .string value
  | .bytes, value => .bytes value
  | .tuple elements, values => .tuple _ fun element member =>
      rowCodec_plain elements values element (by simpa using member)
  | .struct source fields, values => .nominal source none _ fun field member =>
      rowCodec_plain fields values field (by simpa using member)
  | .enum source names rows _, value => variantEncode_plain source names rows value
  | .vector element, value => .vector _ fun encoded member => by
      obtain ⟨item, _, rfl⟩ := Array.mem_map.mp member
      exact NTy.encode_plain element item
  | .ref referent, value => .tuple _ fun encoded member => by
      simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
      rcases member with rfl | rfl
      · exact NTy.encode_plain referent value.1
      · exact NTy.encode_plain referent value.2
  | .param index, value => Skolems.plain index value

theorem rowCodec_plain : (row : NRow) → (values : HList row) →
    ∀ encoded ∈ (rowCodec row).encode values, Plain encoded
  | .nil, _, _, member => nomatch member
  | .cons τ rest, values, encoded, member => by
      rcases List.mem_cons.mp member with rfl | later
      · exact NTy.encode_plain τ values.1
      · exact rowCodec_plain rest values.2 encoded later

theorem variantEncode_plain (source : StructHandle) : (names : List String) → (rows : NRows) →
    (value : variantCarrier names rows) →
    Plain (variantEncode source names rows (rowCodecs rows) value)
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | name :: names, .cons fields rest, .inl values => .nominal source (some name) _ fun field member =>
      rowCodec_plain fields values field (by simpa using member)
  | _ :: names, .cons _ rest, .inr value => variantEncode_plain source names rest value
end

end Plain

mutual
/-- Whether a type has values: an integer has a width, and an enum a first
variant whose fields have values.  Every type the compiler builds has. -/
def NTy.inhabitable : NTy → Bool
  | .int width _ => width != 0
  | .tuple elements => elements.inhabitable
  | .struct _ fields => fields.inhabitable
  | .enum _ names rows _ => NRows.firstInhabitable names rows
  | .ref referent => referent.inhabitable
  | _ => true

def NRow.inhabitable : NRow → Bool
  | .nil => true
  | .cons τ rest => τ.inhabitable && rest.inhabitable

def NRows.firstInhabitable : List String → NRows → Bool
  | _ :: _, .cons fields _ => fields.inhabitable
  | _, _ => false
end

/-- Type arguments of a call: a row whose types have values. -/
abbrev TypeArgs : Type := { row : NRow // row.inhabitable = true }

theorem NRow.getD_inhabitable : (row : NRow) → row.inhabitable = true → (index : Nat) →
    (default : NTy) → default.inhabitable = true → (row.getD index default).inhabitable = true
  | .nil, _, _, _, inhabited => inhabited
  | .cons _ _, inhabitable, 0, _, _ => by
      simp only [NRow.inhabitable, Bool.and_eq_true] at inhabitable
      exact inhabitable.1
  | .cons _ rest, inhabitable, index + 1, default, inhabited => by
      simp only [NRow.inhabitable, Bool.and_eq_true] at inhabitable
      exact NRow.getD_inhabitable rest inhabitable.2 index default inhabited

section Inhabitant
variable [Skolems]

mutual
/-- A value of a type that has values. -/
def NTy.inhabitant : (τ : NTy) → τ.inhabitable = true → τ.carrier
  | .unit, _ => ()
  | .bool, _ => false
  | .int width signed, inhabitable =>
      ⟨0, zero_fits width signed (by simpa [NTy.inhabitable] using inhabitable)⟩
  | .address, _ => ""
  | .signer, _ => ""
  | .string, _ => ""
  | .bytes, _ => #[]
  | .tuple elements, inhabitable =>
      HList.inhabitant elements (by simpa [NTy.inhabitable] using inhabitable)
  | .struct _ fields, inhabitable =>
      HList.inhabitant fields (by simpa [NTy.inhabitable] using inhabitable)
  | .enum _ names rows _, inhabitable =>
      variantCarrier.inhabitant names rows (by simpa [NTy.inhabitable] using inhabitable)
  | .vector _, _ => default
  | .ref referent, inhabitable =>
      let value := referent.inhabitant (by simpa [NTy.inhabitable] using inhabitable)
      (value, value)
  | .param _, _ => default

def HList.inhabitant : (row : NRow) → row.inhabitable = true → HList row
  | .nil, _ => ()
  | .cons τ rest, inhabitable =>
      (τ.inhabitant (by simp_all [NRow.inhabitable]),
        HList.inhabitant rest (by simp_all [NRow.inhabitable]))

def variantCarrier.inhabitant : (names : List String) → (rows : NRows) →
    NRows.firstInhabitable names rows = true → variantCarrier names rows
  | _ :: _, .cons fields _, inhabitable => .inl (HList.inhabitant fields inhabitable)
  | [], _, inhabitable => absurd inhabitable (by simp [NRows.firstInhabitable])
  | _ :: _, .nil, inhabitable => absurd inhabitable (by simp [NRows.firstInhabitable])
end

end Inhabitant

/-- The family a generic call's type arguments induce: parameter `i` is
carried as the `i`-th argument at the caller's family. -/
@[reducible] def Skolems.instantiate (θ : TypeArgs) (outer : Skolems) : Skolems where
  carrier := fun index => @NTy.carrier outer ((NTy.param index).subst θ.1)
  codec := fun index => @NTy.codec outer ((NTy.param index).subst θ.1)
  decEq := fun index => @NTy.decEq outer ((NTy.param index).subst θ.1)
  inhabited := fun index =>
    ⟨@NTy.inhabitant outer ((NTy.param index).subst θ.1)
      (NRow.getD_inhabitable θ.1 θ.2 index (.param index) rfl)⟩
  tight := fun index => @NTy.codec_tight outer ((NTy.param index).subst θ.1)
  plain := fun index => @NTy.encode_plain outer ((NTy.param index).subst θ.1)

section Transport
variable [Θ : Skolems]

mutual
/-- A caller's value in the callee's view: the same runtime value, carried
at the family the call's type arguments induce. -/
def NTy.toSkolem (θ : TypeArgs) : (τ : NTy) → (τ.subst θ.1).carrier →
    @NTy.carrier (Skolems.instantiate θ Θ) τ
  | .unit, value => value
  | .bool, value => value
  | .int _ _, value => value
  | .address, value => value
  | .signer, value => value
  | .string, value => value
  | .bytes, value => value
  | .tuple elements, values => HList.toSkolem θ elements values
  | .struct _ fields, values => HList.toSkolem θ fields values
  | .enum _ names rows _, value => variantCarrier.toSkolem θ names rows value
  | .vector element, vector =>
      ⟨vector.values.map (NTy.toSkolem θ element), by simpa using vector.bounded⟩
  | .ref referent, value => (NTy.toSkolem θ referent value.1, NTy.toSkolem θ referent value.2)
  | .param _, value => value

def HList.toSkolem (θ : TypeArgs) : (row : NRow) → HList (NRow.subst θ.1 row) →
    @HList (Skolems.instantiate θ Θ) row
  | .nil, _ => ()
  | .cons τ rest, values => (NTy.toSkolem θ τ values.1, HList.toSkolem θ rest values.2)

def variantCarrier.toSkolem (θ : TypeArgs) : (names : List String) → (rows : NRows) →
    variantCarrier names (NRows.subst θ.1 rows) → @variantCarrier (Skolems.instantiate θ Θ) names rows
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl values => .inl (HList.toSkolem θ fields values)
  | _ :: names, .cons _ rest, .inr value => .inr (variantCarrier.toSkolem θ names rest value)
end

mutual
/-- A callee's value in the caller's view. -/
def NTy.ofSkolem (θ : TypeArgs) : (τ : NTy) → @NTy.carrier (Skolems.instantiate θ Θ) τ →
    (τ.subst θ.1).carrier
  | .unit, value => value
  | .bool, value => value
  | .int _ _, value => value
  | .address, value => value
  | .signer, value => value
  | .string, value => value
  | .bytes, value => value
  | .tuple elements, values => HList.ofSkolem θ elements values
  | .struct _ fields, values => HList.ofSkolem θ fields values
  | .enum _ names rows _, value => variantCarrier.ofSkolem θ names rows value
  | .vector element, vector =>
      ⟨vector.values.map (NTy.ofSkolem θ element), by simpa using vector.bounded⟩
  | .ref referent, value => (NTy.ofSkolem θ referent value.1, NTy.ofSkolem θ referent value.2)
  | .param _, value => value

def HList.ofSkolem (θ : TypeArgs) : (row : NRow) → @HList (Skolems.instantiate θ Θ) row →
    HList (NRow.subst θ.1 row)
  | .nil, _ => ()
  | .cons τ rest, values => (NTy.ofSkolem θ τ values.1, HList.ofSkolem θ rest values.2)

def variantCarrier.ofSkolem (θ : TypeArgs) : (names : List String) → (rows : NRows) →
    @variantCarrier (Skolems.instantiate θ Θ) names rows → variantCarrier names (NRows.subst θ.1 rows)
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl values => .inl (HList.ofSkolem θ fields values)
  | _ :: names, .cons _ rest, .inr value => .inr (variantCarrier.ofSkolem θ names rest value)
end

/-- A callee's result in the caller's view. -/
def ResultShape.ofSkolem (θ : TypeArgs) : (shape : ResultShape) →
    @ResultShape.carrier (Skolems.instantiate θ Θ) shape → (shape.subst θ.1).carrier
  | .none, value => value
  | .one τ, value => NTy.ofSkolem θ τ value

/-- The default of a parameter under an induced family is its argument's
value, the one a twin of the argument's type defaults to. -/
theorem Skolems.default_instantiate (θ : TypeArgs) (index : Nat) :
    (default : (Skolems.instantiate θ Θ).carrier index) =
      NTy.inhabitant ((NTy.param index).subst θ.1)
        (NRow.getD_inhabitable θ.1 θ.2 index (.param index) rfl) := rfl

/-- A type parameter's codec is its family's. -/
theorem NTy.codec_param (index : Nat) : (NTy.param index).codec = Skolems.codec index := rfl

/-- A type parameter's encoding is its family's codec. -/
theorem NTy.encode_param (index : Nat) (value : (NTy.param index).carrier) :
    NTy.encode (.param index) value = (Skolems.codec index).encode value := rfl

/-- The codec an induced family gives a parameter is its argument's. -/
theorem Skolems.codec_instantiate (θ : TypeArgs) (index : Nat) :
    (Skolems.instantiate θ Θ).codec index = NTy.codec ((NTy.param index).subst θ.1) := rfl

end Transport

attribute [lir_denote] NTy.inhabitant HList.inhabitant variantCarrier.inhabitant
  Skolems.default_instantiate
attribute [lir_denote] NTy.toSkolem HList.toSkolem variantCarrier.toSkolem NTy.ofSkolem
  HList.ofSkolem variantCarrier.ofSkolem ResultShape.ofSkolem NTy.codec_param NTy.encode_param
  Skolems.codec_instantiate

end LeanerIR.Proofs.Denote
