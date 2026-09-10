-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Typed
import LeanerIR.Proofs.IntegerArithmetic
import LeanerIR.Proofs.IntegerEvaluation
import LeanerIR.Proofs.Meaning
import LeanerIR.Proofs.Denote.Attr

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
  /-- A mutable reference: its loan and the current value it owns. -/
  | ref (referent : NTy)

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
  | .tuple _ | .struct _ _ | .enum _ _ _ _ | .ref _ => false
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
  | .ref referent => Nat × referent.carrier

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
  | .ref referent => @instDecidableEqProd _ _ inferInstance referent.decEq

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

/-- A mutable reference: the loan it holds and the encoded current value. -/
def Codec.mutRef (codec : Codec Native RuntimeValue) : Codec (Nat × Native) RuntimeValue where
  encode := fun value => .borrow value.1 (codec.encode value.2)
  decode?
    | .borrow loan current => (codec.decode? current).map fun decoded => (loan, decoded)
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
  | .ref referent => Codec.mutRef referent.codec

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
@[simp] theorem NTy.encode_ref (referent : NTy) (value : Nat × referent.carrier) :
    NTy.encode (.ref referent) value = .borrow value.1 (referent.encode value.2) := rfl
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

private theorem beq_eq_decide {α : Type} [BEq α] [LawfulBEq α] [DecidableEq α]
    (left right : α) : (left == right) = decide (left = right) := by
  by_cases h : left = right
  · subst h
    simp
  · simp [h]

theorem RuntimeValue.beq_unit : (RuntimeValue.unit == RuntimeValue.unit) = true := by
  show RuntimeValue.beq _ _ = true
  unfold RuntimeValue.beq
  rfl

theorem RuntimeValue.beq_signer (left right : String) :
    (RuntimeValue.signer left == RuntimeValue.signer right) = decide (left = right) := by
  show RuntimeValue.beq _ _ = _
  unfold RuntimeValue.beq
  exact beq_eq_decide _ _

theorem RuntimeValue.beq_string (left right : String) :
    (RuntimeValue.string left == RuntimeValue.string right) = decide (left = right) := by
  show RuntimeValue.beq _ _ = _
  unfold RuntimeValue.beq
  exact beq_eq_decide _ _

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
  | .ref _, _, _ => false

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

@[simp] theorem namedIn_nil (name : String) : namedIn name [] = false := rfl
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
  BitOp.eval_and CompareOp.decide_less CompareOp.decide_greater CompareOp.decide_lessEqual
  CompareOp.decide_greaterEqual NTy.eqb_int NTy.eqb_bool NTy.eqb_address NTy.eqb_unit
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

@[simp] theorem Family.key_eq (family : Family) (key : RuntimeValue) :
    family.key key = ⟨family.namespaceId, family.typeId, key.storageKey⟩ := rfl
@[simp] theorem storageKey_address (address : String) :
    (RuntimeValue.address address).storageKey = .address address := rfl
@[simp] theorem storageKey_signer (address : String) :
    (RuntimeValue.signer address).storageKey = .address address := rfl

/-- Mint a fresh loan: its identity, with the frontier advanced. -/
def mintLoan : Comp Nat :=
  Spec.bind Spec.get fun state =>
    Spec.bind (Spec.modify fun state => { state with nextLoan := state.nextLoan + 1 }) fun _ =>
      Spec.pure state.nextLoan

theorem wp_mintLoan (ensures : Nat → RuntimeState → Prop) (aborts : Failure → Prop)
    (state : RuntimeState) :
    wp mintLoan ensures aborts state ↔
      ensures state.nextLoan { state with nextLoan := state.nextLoan + 1 } := by
  simp [mintLoan, wp_bind, wp_get, wp_modify, wp_pure]

attribute [lir_denote] wp_mintLoan

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

@[simp] theorem NTy.codec_int (width : Nat) (signed : Bool) :
    (NTy.int width signed).codec = Codec.specInt (.bits width) signed := rfl
@[simp] theorem NTy.codec_bool : NTy.bool.codec = Codec.bool := rfl
@[simp] theorem NTy.codec_address : NTy.address.codec = Codec.address := rfl
@[simp] theorem NTy.codec_unit : NTy.unit.codec = Codec.unit := rfl
@[simp] theorem NTy.codec_ref (referent : NTy) :
    (NTy.ref referent).codec = Codec.mutRef referent.codec := rfl

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
@[simp] theorem NTy.codec_encode (τ : NTy) (value : τ.carrier) :
    τ.codec.encode value = τ.encode value := rfl

/-- Encodings at one type are equal exactly when the values are. -/
@[simp] theorem NTy.encode_inj (τ : NTy) (left right : τ.carrier) :
    (τ.encode left = τ.encode right) ↔ left = right :=
  ⟨τ.encode_injective, fun equal => equal ▸ rfl⟩

/-- Encoded rows are equal exactly when the rows are. -/
@[simp] theorem HList.encode_inj {Γ : NRow} (left right : HList Γ) :
    (HList.encode left = HList.encode right) ↔ left = right :=
  ⟨fun equal => (rowCodec Γ).encode_injective equal, fun equal => equal ▸ rfl⟩

/-- Decoding a nominal literal at a struct type: the row decodes the fields. -/
@[simp] theorem NTy.decode?_struct_nominal (source : StructHandle) (fields : NRow)
    (values : Array RuntimeValue) :
    (NTy.struct source fields).codec.decode? (.nominal source none values) =
      (rowCodec fields).decode? values.toList := by
  simp [NTy.codec, Codec.nominalRow]

/-- Decoding a row literal, component by component. -/
@[simp] theorem rowCodec_decode?_cons (τ : NTy) (rest : NRow) (value : RuntimeValue)
    (values : List RuntimeValue) :
    (rowCodec (.cons τ rest)).decode? (value :: values) =
      (τ.codec.decode? value).bind fun head =>
        ((rowCodec rest).decode? values).bind fun tail =>
          (some (head, tail) : Option (HList (.cons τ rest))) := rfl

@[simp] theorem rowCodec_decode?_nil :
    (rowCodec .nil).decode? [] = @some (HList .nil) () := rfl

/-- Decoding the unfolded encoding of a struct value. -/
@[simp] theorem NTy.decode?_struct_literal (source : StructHandle) (fields : NRow)
    (value : HList fields) :
    (NTy.struct source fields).codec.decode? (.nominal source none value.encode.toArray) =
      some value := (NTy.struct source fields).codec.decode_encode value

/-- Decoding the unfolded encoding of a tuple value. -/
@[simp] theorem NTy.decode?_tuple_literal (elements : NRow) (value : HList elements) :
    (NTy.tuple elements).codec.decode? (.tuple value.encode.toArray) = some value :=
  (NTy.tuple elements).codec.decode_encode value

/-- Decoding an encoded value at its own type. -/
@[simp] theorem NTy.decode?_encode (τ : NTy) (value : τ.carrier) :
    τ.codec.decode? (τ.encode value) = some value := τ.codec.decode_encode value

/-- A registry lookup steps through a registration. -/
@[simp] theorem globalLoanKeyIn?_cons (registered loan : Nat) (key : GlobalKey)
    (rest : List (Nat × GlobalKey)) :
    LeanerIR.SemanticOperations.globalLoanKeyIn? ((registered, key) :: rest) loan =
      if registered = loan then some key
      else LeanerIR.SemanticOperations.globalLoanKeyIn? rest loan := by
  by_cases equal : registered = loan
  · subst equal
    simp [LeanerIR.SemanticOperations.globalLoanKeyIn?]
  · simp [LeanerIR.SemanticOperations.globalLoanKeyIn?, List.find?_cons, equal]

/-- The loan discipline through a global loan that a body registers, keeps
across a callee, and retires: from the entry state to the state after the
retirement, whatever the callee did to the registry. -/
theorem LoanDiscipline.through_global_loan {initial registered final retired : RuntimeState}
    {loan : Nat} {key : GlobalKey}
    (registry : registered.globalLoans = (loan, key) :: initial.globalLoans)
    (minted : initial.nextLoan ≤ loan) (live : loan < registered.nextLoan)
    (discipline : LeanerIR.SemanticOperations.LoanDiscipline registered final)
    (retiredRegistry : retired.globalLoans =
      LeanerIR.SemanticOperations.removeGlobalLoan final.globalLoans loan)
    (retiredFrontier : retired.nextLoan = final.nextLoan) :
    LeanerIR.SemanticOperations.LoanDiscipline initial retired := by
  obtain ⟨fresh, stable, monotone⟩ := discipline
  refine ⟨fun freshInitial other bound => ?_, fun other bound => ?_, by omega⟩
  · rw [retiredRegistry, LeanerIR.SemanticOperations.globalLoanKeyIn?_remove_other _ _ _ (by omega)]
    have freshRegistered : LeanerIR.SemanticOperations.FreshGlobalLoanIds registered := by
      intro candidate above
      rw [registry]
      exact LeanerIR.SemanticOperations.globalLoanKeyIn?_cons_none (by omega)
        (freshInitial candidate (by omega))
    exact fresh freshRegistered other (by omega)
  · rw [retiredRegistry, LeanerIR.SemanticOperations.globalLoanKeyIn?_remove_other _ _ _ (by omega),
      stable other (by omega), registry]
    have different : (loan == other) = false := by
      simp only [beq_eq_false_iff_ne]
      omega
    simp only [LeanerIR.SemanticOperations.globalLoanKeyIn?, List.find?_cons, different]

/-- The loan discipline holds between a state and any state that keeps its
registry and does not lower its frontier. -/
theorem LoanDiscipline_of_same_registry {initial final : RuntimeState}
    (registry : final.globalLoans = initial.globalLoans)
    (frontier : initial.nextLoan ≤ final.nextLoan) :
    LeanerIR.SemanticOperations.LoanDiscipline initial final :=
  LeanerIR.SemanticOperations.LoanDiscipline.of_eq registry frontier

end LeanerIR.Proofs.Denote
