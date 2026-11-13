-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Slices
import VerexIR.Proofs.Typed
import VerexIR.Proofs.Maps.Model
import VerexIR.Proofs.Order
import VerexIR.Proofs.IntegerArithmetic
import VerexIR.Proofs.IntegerEvaluation
import VerexIR.Proofs.Meaning
import VerexIR.Proofs.Denote.Attr
import VerexIR.Semantics.Focus
import VerexIR.Semantics.ValueTyping

/-!
# Native types of the denotation

The denotation of validated VIR (`doc/designs/denotation.md`) is typed: a term
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

namespace VerexIR.Proofs.Denote

open VerexIR.SemanticOperations

/-- Which positions of a function type are shared references, each the
value it observes natively: its parameters and its results. -/
structure Sharing where
  parameters : List Bool
  results : List Bool
  deriving DecidableEq, Repr, Inhabited

mutual
/-- The types the denotation carries: scalars, tuples, and nominal data
with its declared field rows.  An enum names its variants separately from
their rows and carries the distinctness its codec needs. -/
inductive NTy where
  | unit
  | bool
  | int (width : Nat) (signed : Bool)
  /-- An unbounded integer, a specification's `num`: only a ghost slot holds
  one (`doc/designs/ghost-fields.md`). -/
  | num
  | address
  | signer
  | string
  | bytes
  /-- A Unicode scalar value. -/
  | char
  | tuple (elements : NRow)
  /-- A structure at its type arguments, which its rows need not mention. -/
  | struct (source : StructHandle) (arguments : NRow) (fields : NRow)
  | enum (source : StructHandle) (arguments : NRow) (names : List String) (rows : NRows)
      (distinct : names.Nodup)
  /-- A growable vector of one element type. -/
  | vector (element : NTy)
  /-- The logical snapshot of an unsized slice, distinct from an owned vector. -/
  | slice (element : NTy)
  /-- A fixed-length array: a vector with the fact of its length. -/
  | array (element : NTy) (length : Nat)
  /-- The entries of a handle-backed intrinsic map, a value of their own
  beside the handle: the hidden last field of the map's owner
  (`doc/designs/intrinsic-maps.md`, "Tables in the runtime semantics"). The runtime
  keeps them in native storage, the denotation in the value; a declared field
  never has this type. -/
  | table (key value : NTy)
  /-- A ghost field's slot (`doc/designs/ghost-fields.md`): a value of its type,
  encoded in a ghost slot after the declared fields, which no equality
  reads. -/
  | ghost (slot : NTy)
  /-- A mutable reference: its loan and the current value it owns. -/
  | ref (referent : NTy)
  /-- A mutable slice reference with one extent for current contents and prophecy. -/
  | sliceRef (element : NTy)
  /-- A type parameter of a generic function: its skolem family's carrier. -/
  | param (index : Nat)
  /-- A function value taking the parameters and returning the results;
  `shared` marks each parameter and each result that is a shared
  reference, passed or returned as the value it observes. -/
  | function (parameters : NRow) (shared : Sharing) (results : NRow)
  /-- A substitution's entry at a shared reference: the erased referent, marked so
  that a slot of a function type declared at the parameter reads shared. Only a
  substitution's row holds one; no type of a value is marked, and substitution
  strips the mark wherever a parameter stands for a type. -/
  | observed (referent : NTy)
  /-- A nominal whose declaration reaches itself, at its type arguments: no row of it is
  written here, as `NTy` has no binder. `productive` says whether the unit's declaration has
  values at inhabited arguments; the typed values of `.nominal` are read at the unit's
  productivity (`doc/designs/rust-verification.md`, "Inhabitants"). -/
  | nominal (source : StructHandle) (arguments : NRow) (productive : Bool)

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
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char => true
  | .tuple _ | .struct _ _ _ | .enum _ _ _ _ _ | .slice _ | .vector _ | .array _ _ | .table _ _ | .ghost _
  | .ref _ | .sliceRef _ | .param _ | .function _ _ _ | .nominal _ _ _ => false
  -- A substitution's marked entry is no type of a value, so no clause compares one.
  | .observed _ => false

/-- Whether a type is a map's contents: the hidden last field of its owner. -/
def NTy.isTable : NTy → Bool
  | .table _ _ => true
  | _ => false

@[simp] theorem NTy.isTable_table (key value : NTy) : (NTy.table key value).isTable = true := rfl

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
def _root_.VerexIR.Proofs.Codec.Tight {Native Runtime : Type} (codec : Codec Native Runtime) : Prop :=
  ∀ raw value, codec.decode? raw = some value → codec.encode value = raw

/-- A function value: the closure the runtime holds, whose captures hold no
loan. -/
structure ClosureValue where
  function : FunctionHandle
  mask : Nat
  typeInstantiation : Array (TypeId × TypeId)
  captures : Array RuntimeValue
  plain : ∀ capture ∈ captures, Plain capture

theorem ClosureValue.ext {left right : ClosureValue} (function : left.function = right.function)
    (mask : left.mask = right.mask)
    (typeInstantiation : left.typeInstantiation = right.typeInstantiation)
    (captures : left.captures = right.captures) : left = right := by
  cases left; cases right; cases function; cases mask; cases typeInstantiation; cases captures
  rfl

instance : DecidableEq ClosureValue := fun left right =>
  if same : left.function = right.function ∧ left.mask = right.mask ∧
      left.typeInstantiation = right.typeInstantiation ∧ left.captures = right.captures then
    isTrue (ClosureValue.ext same.1 same.2.1 same.2.2.1 same.2.2.2)
  else isFalse fun equal => same (equal ▸ ⟨rfl, rfl, rfl, rfl⟩)

instance : Inhabited ClosureValue :=
  ⟨⟨default, 0, #[], #[], fun _ member => nomatch member⟩⟩

/-- The runtime closure a function value is. -/
def ClosureValue.encode (value : ClosureValue) : RuntimeValue :=
  .closure value.function value.mask value.typeInstantiation value.captures

/-- The order of function values, as Move orders them: by the function's
rank, its handle, its type arguments, its mask, then the captured values. -/
def ClosureValue.order (rank : ValueRanks) (left right : ClosureValue) : Ordering :=
  ((compare (rank.function left.function) (rank.function right.function)).then
    ((compare left.function.namespaceId.index right.function.namespaceId.index).then
      (compare left.function.functionId.index right.function.functionId.index))).then
  ((compareInstantiation left.typeInstantiation right.typeInstantiation).then
    ((compare left.mask right.mask).then
      (RuntimeValue.orderList rank left.captures.toList right.captures.toList)))

/-- Function values order as their runtime closures do. -/
theorem ClosureValue.order_encode (rank : ValueRanks) (left right : ClosureValue) :
    RuntimeValue.order rank left.encode right.encode = ClosureValue.order rank left right :=
  RuntimeValue.order_closure ..

@[simp] theorem ClosureValue.order_self (rank : ValueRanks) (value : ClosureValue) :
    ClosureValue.order rank value value = .eq := by
  rw [← ClosureValue.order_encode, RuntimeValue.order_self]

/-- A runtime closure is the function value it encodes. -/
theorem ClosureValue.encode_injective {left right : ClosureValue}
    (same : left.encode = right.encode) : left = right := by
  simp only [ClosureValue.encode, RuntimeValue.closure.injEq] at same
  exact ClosureValue.ext same.1 same.2.1 same.2.2.1 same.2.2.2

/-- The function value a runtime closure without loans is. -/
def ClosureValue.decode? : RuntimeValue → Option ClosureValue
  | .closure function mask typeInstantiation captures =>
      if plain : ∀ capture ∈ captures, Plain capture then
        some ⟨function, mask, typeInstantiation, captures, plain⟩
      else none
  | _ => none

/-- The codec of function values. -/
def Codec.closure : Codec ClosureValue RuntimeValue where
  encode := ClosureValue.encode
  decode? := ClosureValue.decode?
  decode_encode := fun value => by
    obtain ⟨function, mask, typeInstantiation, captures, plain⟩ := value
    simp only [ClosureValue.encode, ClosureValue.decode?, dif_pos plain]

theorem Codec.closure_tight : Codec.closure.Tight := fun raw value decoded => by
  cases raw
  case closure function mask typeInstantiation captures =>
    simp only [Codec.closure, ClosureValue.decode?] at decoded
    split at decoded
    · cases decoded; rfl
    · cases decoded
  all_goals simp [Codec.closure, ClosureValue.decode?] at decoded

/-! ## Native types of a unit

The native types the unit's type tables denote, as the compiler reads them,
and the type arguments that instantiate them. -/

/-- The `index`-th type of a row, or `default` beyond it. -/
@[reducible] def NRow.getD : NRow → Nat → NTy → NTy
  | .nil, _, default => default
  | .cons τ _, 0, _ => τ
  | .cons _ rest, index + 1, default => rest.getD index default

/-- Whether a substitution's entry marks a shared reference: a slot declared at
the parameter reads shared, and the entry stands for the referent. -/
@[reducible] def NTy.isObserved : NTy → Bool
  | .observed _ => true
  | _ => false

/-- The type a substitution's entry stands for, its mark dropped: what a
parameter is replaced by wherever it stands for a type. -/
@[reducible] def NTy.unmark : NTy → NTy
  | .observed referent => referent.unmark
  | τ => τ

/-- Whether a slot of a function type is a shared reference at a family's
reading of the parameters: as declared, or declared at a parameter the family
reads as a shared reference. -/
@[reducible] def NTy.sharedAtWith (types : Nat → NTy) (declared : Bool) : NTy → Bool
  | .param index => declared || (types index).isObserved
  | _ => declared

/-- The sharing of a row of slots, slot by slot: a flag beyond the row stays as
declared. -/
@[reducible] def Sharing.atRow (types : Nat → NTy) : List Bool → NRow → List Bool
  | [], _ => []
  | declared :: rest, .nil => declared :: Sharing.atRow types rest .nil
  | declared :: rest, .cons τ row =>
      τ.sharedAtWith types declared :: Sharing.atRow types rest row

/-- The sharing of a function type's slots at a family's reading of the
parameters: each slot declared at a parameter standing for a shared reference
reads shared, the others as declared. A family marking none leaves the sharing
(`Sharing.atWith_unmarked`). -/
@[reducible] def Sharing.atWith (types : Nat → NTy) (parameters : NRow) (shared : Sharing)
    (results : NRow) : Sharing where
  parameters := Sharing.atRow types shared.parameters parameters
  results := Sharing.atRow types shared.results results

/-- What a substitution reads a parameter as: its entry, marked where the
argument is a shared reference, or the parameter itself beyond the row. -/
@[reducible] def NRow.argument (θ : NRow) (index : Nat) : NTy := θ.getD index (.param index)

/-- The sharing of a function type's slots at a substitution. -/
@[reducible] def Sharing.at (θ : NRow) (parameters : NRow) (shared : Sharing)
    (results : NRow) : Sharing :=
  Sharing.atWith θ.argument parameters shared results

/-- A family marking no parameter leaves a row of slots as declared. -/
theorem Sharing.atRow_unmarked {types : Nat → NTy}
    (unmarked : ∀ index, (types index).isObserved = false) :
    (flags : List Bool) → (row : NRow) → Sharing.atRow types flags row = flags
  | [], _ => rfl
  | _ :: rest, .nil => by
      simp only [Sharing.atRow, Sharing.atRow_unmarked unmarked rest .nil]
  | _ :: rest, .cons τ row => by
      cases τ <;>
        simp only [Sharing.atRow, NTy.sharedAtWith, unmarked, Bool.or_false,
          Sharing.atRow_unmarked unmarked rest row]

/-- A family marking no parameter leaves a function type's sharing. -/
theorem Sharing.atWith_unmarked {types : Nat → NTy}
    (unmarked : ∀ index, (types index).isObserved = false) (parameters : NRow) (shared : Sharing)
    (results : NRow) : Sharing.atWith types parameters shared results = shared := by
  simp only [Sharing.atWith, Sharing.atRow_unmarked unmarked]

/-- The parameters themselves mark nothing. -/
theorem NTy.isObserved_param (index : Nat) : (NTy.param index).isObserved = false := rfl

/-- A mark is dropped once: the type a parameter stands for is never marked. -/
theorem NTy.unmark_unmark : (τ : NTy) → τ.unmark.unmark = τ.unmark
  | .observed referent => NTy.unmark_unmark referent
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char
  | .tuple _ | .struct _ _ _ | .enum _ _ _ _ _ | .slice _ | .vector _ | .array _ _
  | .table _ _ | .ghost _ | .ref _ | .sliceRef _ | .param _ | .function _ _ _
  | .nominal _ _ _ => rfl

mutual
/-- A type with its parameters replaced by type arguments. -/
@[reducible] def NTy.subst (θ : NRow) : NTy → NTy
  | .tuple elements => .tuple (NRow.subst θ elements)
  | .struct source arguments fields =>
      .struct source (NRow.subst θ arguments) (NRow.subst θ fields)
  | .enum source arguments names rows distinct =>
      .enum source (NRow.subst θ arguments) names (rows.subst θ) distinct
  | .slice element => .slice (element.subst θ)
  | .vector element => .vector (element.subst θ)
  | .array element length => .array (element.subst θ) length
  | .table key value => .table (key.subst θ) (value.subst θ)
  | .ghost slot => .ghost (slot.subst θ)
  | .sliceRef referent => .sliceRef (referent.subst θ)
  | .ref referent => .ref (referent.subst θ)
  | .param index => (θ.getD index (.param index)).unmark
  | .function parameters shared results =>
      .function (NRow.subst θ parameters) shared (NRow.subst θ results)
  | .nominal source arguments productive => .nominal source (NRow.subst θ arguments) productive
  | .observed referent => .observed (referent.subst θ)
  | τ => τ

@[reducible] def NRow.subst (θ : NRow) : NRow → NRow
  | .nil => .nil
  | .cons τ rest => .cons (τ.subst θ) (NRow.subst θ rest)

@[reducible] def NRows.subst (θ : NRow) : NRows → NRows
  | .nil => .nil
  | .cons fields rest => .cons (NRow.subst θ fields) (rest.subst θ)
end

mutual
/-- A type with its parameters replaced by what a family says they stand
for (`Carriers.type`). -/
def NTy.substWith (types : Nat → NTy) : NTy → NTy
  | .unit => .unit
  | .bool => .bool
  | .int width signed => .int width signed
  | .num => .num
  | .address => .address
  | .signer => .signer
  | .string => .string
  | .bytes => .bytes
  | .char => .char
  | .tuple elements => .tuple (NRow.substWith types elements)
  | .struct source arguments fields =>
      .struct source (NRow.substWith types arguments) (NRow.substWith types fields)
  | .enum source arguments names rows distinct =>
      .enum source (NRow.substWith types arguments) names (NRows.substWith types rows) distinct
  | .slice element => .slice (element.substWith types)
  | .vector element => .vector (element.substWith types)
  | .array element length => .array (element.substWith types) length
  | .table key value => .table (key.substWith types) (value.substWith types)
  | .ghost slot => .ghost (slot.substWith types)
  | .sliceRef referent => .sliceRef (referent.substWith types)
  | .ref referent => .ref (referent.substWith types)
  | .param index => (types index).unmark
  | .function parameters shared results =>
      .function (NRow.substWith types parameters) shared (NRow.substWith types results)
  | .nominal source arguments productive =>
      .nominal source (NRow.substWith types arguments) productive
  | .observed referent => .observed (referent.substWith types)

def NRow.substWith (types : Nat → NTy) : NRow → NRow
  | .nil => .nil
  | .cons τ rest => .cons (τ.substWith types) (NRow.substWith types rest)

def NRows.substWith (types : Nat → NTy) : NRows → NRows
  | .nil => .nil
  | .cons fields rest => .cons (NRow.substWith types fields) (NRows.substWith types rest)
end

/-- A substitution's entry read at a family, its mark kept: what a family
composed with a substitution reads a parameter as. -/
@[reducible] def NTy.argumentWith (types : Nat → NTy) : NTy → NTy
  | .param index => types index
  | .observed referent => .observed (NTy.substWith types referent)
  | τ => NTy.substWith types τ

/-- Dropping a mark commutes with a family's substitution, which never marks. -/
theorem NTy.substWith_unmark (types : Nat → NTy) :
    (τ : NTy) → (τ.unmark).substWith types = (τ.substWith types).unmark
  | .observed referent => by
      simp only [NTy.unmark, NTy.substWith, NTy.substWith_unmark types referent]
  | .param index => (NTy.unmark_unmark (types index)).symm
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char
  | .tuple _ | .struct _ _ _ | .enum _ _ _ _ _ | .slice _ | .vector _ | .array _ _
  | .table _ _ | .ghost _ | .ref _ | .sliceRef _ | .function _ _ _ | .nominal _ _ _ => rfl

mutual
/-- Families reading each parameter as the same type, its mark aside,
substitute alike: substitution drops the mark where a parameter stands for a
type. -/
theorem NTy.substWith_unmarked_congr {left right : Nat → NTy}
    (agree : ∀ index, (left index).unmark = (right index).unmark) :
    (τ : NTy) → τ.substWith left = τ.substWith right
  | .param index => agree index
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char => rfl
  | .tuple elements => congrArg NTy.tuple (NRow.substWith_unmarked_congr agree elements)
  | .struct source arguments fields => by
      simp only [NTy.substWith, NRow.substWith_unmarked_congr agree arguments,
        NRow.substWith_unmarked_congr agree fields]
  | .enum source arguments names rows distinct => by
      simp only [NTy.substWith, NRow.substWith_unmarked_congr agree arguments,
        NRows.substWith_unmarked_congr agree rows]
  | .slice element => congrArg NTy.slice (NTy.substWith_unmarked_congr agree element)
  | .vector element => congrArg NTy.vector (NTy.substWith_unmarked_congr agree element)
  | .array element length =>
      congrArg (NTy.array · length) (NTy.substWith_unmarked_congr agree element)
  | .table key value => by
      simp only [NTy.substWith, NTy.substWith_unmarked_congr agree key,
        NTy.substWith_unmarked_congr agree value]
  | .ghost slot => congrArg NTy.ghost (NTy.substWith_unmarked_congr agree slot)
  | .observed referent => congrArg NTy.observed (NTy.substWith_unmarked_congr agree referent)
  | .sliceRef referent => congrArg NTy.sliceRef (NTy.substWith_unmarked_congr agree referent)
  | .ref referent => congrArg NTy.ref (NTy.substWith_unmarked_congr agree referent)
  | .function parameters shared results => by
      simp only [NTy.substWith, NRow.substWith_unmarked_congr agree parameters,
        NRow.substWith_unmarked_congr agree results]
  | .nominal source arguments productive => by
      simp only [NTy.substWith, NRow.substWith_unmarked_congr agree arguments]

theorem NRow.substWith_unmarked_congr {left right : Nat → NTy}
    (agree : ∀ index, (left index).unmark = (right index).unmark) :
    (row : NRow) → row.substWith left = row.substWith right
  | .nil => rfl
  | .cons τ rest => by
      simp only [NRow.substWith, NTy.substWith_unmarked_congr agree τ,
        NRow.substWith_unmarked_congr agree rest]

theorem NRows.substWith_unmarked_congr {left right : Nat → NTy}
    (agree : ∀ index, (left index).unmark = (right index).unmark) :
    (rows : NRows) → rows.substWith left = rows.substWith right
  | .nil => rfl
  | .cons fields rest => by
      simp only [NRows.substWith, NRow.substWith_unmarked_congr agree fields,
        NRows.substWith_unmarked_congr agree rest]
end

mutual
/-- Whether every parameter a type mentions satisfies a predicate. -/
def NTy.paramsAll (allowed : Nat → Bool) : NTy → Bool
  | .param index => allowed index
  | .tuple elements => NRow.paramsAll allowed elements
  | .struct _ arguments fields =>
      NRow.paramsAll allowed arguments && NRow.paramsAll allowed fields
  | .enum _ arguments _ rows _ =>
      NRow.paramsAll allowed arguments && NRows.paramsAll allowed rows
  | .slice element => NTy.paramsAll allowed element
  | .vector element => NTy.paramsAll allowed element
  | .array element _ => NTy.paramsAll allowed element
  | .table key value => NTy.paramsAll allowed key && NTy.paramsAll allowed value
  | .ghost slot => NTy.paramsAll allowed slot
  | .sliceRef referent => NTy.paramsAll allowed referent
  | .ref referent => NTy.paramsAll allowed referent
  | .function parameters _ results =>
      NRow.paramsAll allowed parameters && NRow.paramsAll allowed results
  | .nominal _ arguments _ => NRow.paramsAll allowed arguments
  | .observed referent => NTy.paramsAll allowed referent
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char => true

def NRow.paramsAll (allowed : Nat → Bool) : NRow → Bool
  | .nil => true
  | .cons τ rest => NTy.paramsAll allowed τ && NRow.paramsAll allowed rest

def NRows.paramsAll (allowed : Nat → Bool) : NRows → Bool
  | .nil => true
  | .cons fields rest => NRow.paramsAll allowed fields && NRows.paramsAll allowed rest
end

/-- Families agreeing on the parameters a slot mentions agree on its sharing. -/
theorem NTy.sharedAtWith_congr {allowed : Nat → Bool} {left right : Nat → NTy}
    (agree : ∀ index, allowed index = true → left index = right index) {declared : Bool} :
    (τ : NTy) → τ.paramsAll allowed = true →
      τ.sharedAtWith left declared = τ.sharedAtWith right declared
  | .param index, mentioned => by
      simp only [NTy.sharedAtWith, agree index mentioned]
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _
  | .bytes, _ | .char, _ | .tuple _, _ | .struct _ _ _, _ | .enum _ _ _ _ _, _ | .slice _, _
  | .vector _, _ | .array _ _, _ | .table _ _, _ | .ghost _, _ | .ref _, _ | .sliceRef _, _
  | .observed _, _ | .function _ _ _, _ | .nominal _ _ _, _ => rfl

/-- Families agreeing on the parameters a row mentions agree on its sharing. -/
theorem Sharing.atRow_congr {allowed : Nat → Bool} {left right : Nat → NTy}
    (agree : ∀ index, allowed index = true → left index = right index) :
    (flags : List Bool) → (row : NRow) → row.paramsAll allowed = true →
      Sharing.atRow left flags row = Sharing.atRow right flags row
  | [], _, _ => rfl
  | _ :: rest, .nil, _ => by
      simp only [Sharing.atRow, Sharing.atRow_congr agree rest .nil rfl]
  | _ :: rest, .cons τ row, mentioned => by
      simp only [NRow.paramsAll, Bool.and_eq_true] at mentioned
      simp only [Sharing.atRow, NTy.sharedAtWith_congr agree τ mentioned.1,
        Sharing.atRow_congr agree rest row mentioned.2]

/-- Families agreeing on the parameters a function type mentions agree on its
sharing. -/
theorem Sharing.atWith_congr {allowed : Nat → Bool} {left right : Nat → NTy}
    (agree : ∀ index, allowed index = true → left index = right index) {parameters results : NRow}
    {shared : Sharing} (mentions : parameters.paramsAll allowed = true)
    (mentionsResults : results.paramsAll allowed = true) :
    Sharing.atWith left parameters shared results =
      Sharing.atWith right parameters shared results := by
  simp only [Sharing.atWith, Sharing.atRow_congr agree _ parameters mentions,
    Sharing.atRow_congr agree _ results mentionsResults]

mutual
/-- Substitutions agreeing on the parameters a type mentions agree on it. -/
theorem NTy.substWith_congr {allowed : Nat → Bool} {left right : Nat → NTy}
    (agree : ∀ index, allowed index = true → left index = right index) :
    (τ : NTy) → τ.paramsAll allowed = true → τ.substWith left = τ.substWith right
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _ | .bytes, _ | .char, _ => rfl
  | .param index, mentioned => congrArg NTy.unmark (agree index mentioned)
  | .tuple elements, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, NRow.substWith_congr agree elements mentioned]
  | .struct source arguments fields, mentioned => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.substWith, NRow.substWith_congr agree arguments mentioned.1, NRow.substWith_congr agree fields mentioned.2]
  | .enum source arguments names rows distinct, mentioned => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.substWith, NRow.substWith_congr agree arguments mentioned.1,
        NRows.substWith_congr agree rows mentioned.2]
  | .slice element, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, NTy.substWith_congr agree element mentioned]
  | .vector element, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, NTy.substWith_congr agree element mentioned]
  | .array element _, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, NTy.substWith_congr agree element mentioned]
  | .table key value, mentioned => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.substWith, NTy.substWith_congr agree key mentioned.1, NTy.substWith_congr agree value mentioned.2]
  | .ghost slot, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, NTy.substWith_congr agree slot mentioned]
  | .sliceRef referent, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, NTy.substWith_congr agree referent mentioned]
  | .ref referent, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, NTy.substWith_congr agree referent mentioned]
  | .observed referent, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, NTy.substWith_congr agree referent mentioned]
  | .function parameters shared results, mentioned => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.substWith, NRow.substWith_congr agree parameters mentioned.1,
        NRow.substWith_congr agree results mentioned.2,
        Sharing.atWith_congr agree mentioned.1 mentioned.2]
  | .nominal source arguments productive, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, NRow.substWith_congr agree arguments mentioned]

theorem NRow.substWith_congr {allowed : Nat → Bool} {left right : Nat → NTy}
    (agree : ∀ index, allowed index = true → left index = right index) :
    (row : NRow) → row.paramsAll allowed = true → row.substWith left = row.substWith right
  | .nil, _ => rfl
  | .cons τ rest, mentioned => by
      simp only [NRow.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NRow.substWith, NTy.substWith_congr agree τ mentioned.1, NRow.substWith_congr agree rest mentioned.2]

theorem NRows.substWith_congr {allowed : Nat → Bool} {left right : Nat → NTy}
    (agree : ∀ index, allowed index = true → left index = right index) :
    (rows : NRows) → rows.paramsAll allowed = true → rows.substWith left = rows.substWith right
  | .nil, _ => rfl
  | .cons fields rest, mentioned => by
      simp only [NRows.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NRows.substWith, NRow.substWith_congr agree fields mentioned.1, NRows.substWith_congr agree rest mentioned.2]
end

mutual
/-- Whether a type has values: an integer has a width, and an enum a first
variant whose fields have values.  Every type the compiler builds has. -/
def NTy.inhabitable : NTy → Bool
  | .int width _ => width != 0
  | .tuple elements => elements.inhabitable
  | .struct _ _ fields => fields.inhabitable
  | .enum _ _ names rows _ => NRows.firstInhabitable names rows
  | .ref referent => referent.inhabitable
  | .ghost slot => slot.inhabitable
  -- A function type's values are the unit's typed closures, which a type
  -- alone does not provide.
  | .function _ _ _ => false
  -- A nominal's values are read at the unit's productivity, which its flag states.
  | .nominal _ arguments productive => productive && arguments.inhabitable
  | .array element length => decide (length < 2 ^ 64) && element.inhabitable
  | .observed referent => referent.inhabitable
  | .unit | .bool | .num | .address | .signer | .string | .bytes | .char | .slice _ | .vector _ | .table _ _
  | .param _ | .sliceRef _ => true

def NRow.inhabitable : NRow → Bool
  | .nil => true
  | .cons τ rest => τ.inhabitable && rest.inhabitable

def NRows.firstInhabitable : List String → NRows → Bool
  | _ :: _, .cons fields _ => fields.inhabitable
  | _, _ => false
end

/-- A mark says nothing about values: what a marked entry stands for has values
exactly where the entry does. -/
@[simp] theorem NTy.inhabitable_unmark : (τ : NTy) → τ.unmark.inhabitable = τ.inhabitable
  | .observed referent => NTy.inhabitable_unmark referent
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char
  | .tuple _ | .struct _ _ _ | .enum _ _ _ _ _ | .slice _ | .vector _ | .array _ _
  | .table _ _ | .ghost _ | .ref _ | .sliceRef _ | .param _ | .function _ _ _
  | .nominal _ _ _ => rfl

section NativeTypes

open VerexIR.Validation

/-- A lookup of the type table entry a type identifier names, per namespace. -/
abbrev TypeLookup := NamespaceId → TypeId → Option Ty

/-- The type tables of a unit, read directly. -/
def unitTypes (unit : ValidatedUnit) : TypeLookup := fun namespaceId typeId =>
  (unit.namespaces[namespaceId.index]?).bind (·.tables.types[typeId.index]?)

mutual
/-- Whether a type holds no ghost slot: structural equality, which reads no
ghost, then decides the equality of its reference-free values. -/
def NTy.ghostFree : NTy → Bool
  | .ghost _ => false
  | .tuple elements => elements.ghostFree
  | .struct _ _ fields => fields.ghostFree
  | .enum _ _ _ rows _ => rows.ghostFree
  | .slice element => element.ghostFree
  | .vector element => element.ghostFree
  | .array element _ => element.ghostFree
  | .table key value => key.ghostFree && value.ghostFree
  | .sliceRef referent => referent.ghostFree
  | .ref referent => referent.ghostFree
  -- A function value's captures may hold ghost slots.
  | .function _ _ _ => false
  | .observed referent => referent.ghostFree
  -- The rows of a nominal, which its type does not show, may hold ghost slots.
  | .nominal _ _ _ => false
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char | .param _ => true

def NRow.ghostFree : NRow → Bool
  | .nil => true
  | .cons τ rest => τ.ghostFree && rest.ghostFree

def NRows.ghostFree : NRows → Bool
  | .nil => true
  | .cons fields rest => fields.ghostFree && rest.ghostFree
end

mutual
/-- Whether a type holds no reference: structural equality of its values
then decides their equality, where it holds no ghost slot either. -/
def NTy.refFree : NTy → Bool
  | .ref _ | .sliceRef _ => false
  | .tuple elements => elements.refFree
  | .struct _ _ fields => fields.refFree
  | .enum _ _ _ rows _ => rows.refFree
  | .slice element => element.refFree
  | .vector element => element.refFree
  | .array element _ => element.refFree
  | .table key value => key.refFree && value.refFree
  | .ghost slot => slot.refFree
  -- A closure's captures hold no loan, whatever its parameters take.
  | .nominal _ arguments _ => arguments.refFree
  | .observed referent => referent.refFree
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char | .param _
  | .function _ _ _ => true

def NRow.refFree : NRow → Bool
  | .nil => true
  | .cons τ rest => τ.refFree && rest.refFree

def NRows.refFree : NRows → Bool
  | .nil => true
  | .cons fields rest => fields.refFree && rest.refFree
end

/-- A mark says nothing about references: what a marked entry stands for holds
one exactly where the entry does. -/
@[simp] theorem NTy.refFree_unmark : (τ : NTy) → τ.unmark.refFree = τ.refFree
  | .observed referent => NTy.refFree_unmark referent
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char
  | .tuple _ | .struct _ _ _ | .enum _ _ _ _ _ | .slice _ | .vector _ | .array _ _
  | .table _ _ | .ghost _ | .ref _ | .sliceRef _ | .param _ | .function _ _ _
  | .nominal _ _ _ => rfl

mutual
/-- Whether a type mentions no mutable reference, in a nominal's type arguments too, which
`refFree` does not read. A view of it is itself, so every type read at such type arguments is
its declared reading substituted (`CompileNamespace.transfers`). -/
def NTy.refAbsent : NTy → Bool
  | .ref _ | .sliceRef _ => false
  | .tuple elements => elements.refAbsent
  | .struct _ arguments fields => arguments.refAbsent && fields.refAbsent
  | .enum _ arguments _ rows _ => arguments.refAbsent && rows.refAbsent
  | .slice element => element.refAbsent
  | .vector element => element.refAbsent
  | .array element _ => element.refAbsent
  | .table key value => key.refAbsent && value.refAbsent
  | .ghost slot => slot.refAbsent
  | .nominal _ arguments _ => arguments.refAbsent
  | .observed referent => referent.refAbsent
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char | .param _
  | .function _ _ _ => true

def NRow.refAbsent : NRow → Bool
  | .nil => true
  | .cons τ rest => τ.refAbsent && rest.refAbsent

def NRows.refAbsent : NRows → Bool
  | .nil => true
  | .cons fields rest => fields.refAbsent && rest.refAbsent
end

mutual
/-- What a shared view of a type holds: each mutable reference as what it refers
to. The identity on a type holding none. -/
@[reducible] def NTy.erase : NTy → NTy
  | .sliceRef element => .slice element.erase
  | .ref referent => referent.erase
  | .tuple elements => .tuple elements.erase
  | .struct source arguments fields => .struct source arguments.erase fields.erase
  | .enum source arguments names rows distinct =>
      .enum source arguments.erase names rows.erase distinct
  | .slice element => .slice element.erase
  | .vector element => .vector element.erase
  | .array element length => .array element.erase length
  | .table key value => .table key.erase value.erase
  | .ghost slot => .ghost slot.erase
  | .unit => .unit
  | .bool => .bool
  | .int width signed => .int width signed
  | .num => .num
  | .address => .address
  | .signer => .signer
  | .string => .string
  | .bytes => .bytes
  | .char => .char
  | .param index => .param index
  | .function parameters shared results => .function parameters shared results
  | .nominal source arguments productive => .nominal source arguments productive
  | .observed referent => .observed referent.erase

@[reducible] def NRow.erase : NRow → NRow
  | .nil => .nil
  | .cons τ rest => .cons τ.erase rest.erase

@[reducible] def NRows.erase : NRows → NRows
  | .nil => .nil
  | .cons fields rest => .cons fields.erase rest.erase
end

/-- Whether a type identifier names a reference. -/
def referenceType (types : TypeLookup) (namespaceId : NamespaceId) (typeId : TypeId) : Bool :=
  match types namespaceId typeId with
  | some (.reference _) => true
  | _ => false

/-- Whether a type identifier names a shared reference, which a function
type marks on its parameter. -/
def sharedReference (types : TypeLookup) (namespaceId : NamespaceId) (typeId : TypeId) : Bool :=
  match types namespaceId typeId with
  | some (.reference reference) => match reference.kind with
    | .shared => true
    | .mutable => false
  | _ => false

/-- Whether a slot of a function type is a shared reference in a scope: as the
type states, or at a parameter whose type argument is a shared reference, which
the scope's row marks (`doc/designs/denotation.md`, "Instances at a reference"). -/
def slotShared (types : TypeLookup) (env : NRow) (namespaceId : NamespaceId)
    (typeId : TypeId) : Bool :=
  sharedReference types namespaceId typeId ||
    match types namespaceId typeId with
    | some (.typeParameter index) => (env.argument index).isObserved
    | _ => false

/-- Type arguments that leave each parameter read as none mark no slot: a slot is
shared as its type states. -/
theorem slotShared_identity (types : TypeLookup) (env : NRow)
    (identity : ∀ index, env.getD index (.param index) = .param index) :
    slotShared types env = sharedReference types := by
  funext namespaceId typeId
  simp only [slotShared]
  split
  · simp only [NRow.argument, identity, NTy.isObserved, Bool.or_false]
  · simp only [Bool.or_false]

mutual
/-- The native type an VIR type denotes.  A shared reference denotes its
referent: certified exclusivity erases it at run time.  A nominal type
denotes its declaration's rows, read at its type arguments in the declaring
namespace, so that a view of a type argument erases it; `env` holds the type
arguments of the scope read, and `.nil` leaves its parameters. The fuel bounds
the nesting of declarations. -/
def ntyOfFuel (types : TypeLookup) (unit : ValidatedUnit) :
    NRow → Nat → NamespaceId → TypeId → Option NTy
  | _, 0, _, _ => none
  | env, fuel + 1, namespaceId, typeId => do
      let ns ← unit.namespaces[namespaceId.index]?
      let ty ← types namespaceId typeId
      match ty with
      | .unit => some .unit
      | .bool => some .bool
      | .integer (.bits width) signed => if width == 0 then none else some (.int width signed)
      | .integer .pointer signed => (.int · signed) <$> targetPointerWidth? unit
      | .integer .unbounded true => some .num
      -- `!` has no values, so no carrier is read.
      | .never => some .unit
      | .address => some .address
      | .signer => some .signer
      | .string => some .string
      | .bytes => some .bytes
      | .character => some .char
      | .reference reference =>
          match reference.kind with
          -- A shared reference is what it observes, no loan among it.
          | .shared => NTy.erase <$> ntyOfFuel types unit env fuel namespaceId reference.referent
          | .mutable =>
              match types namespaceId reference.referent with
              | some (.slice element) => .sliceRef <$> ntyOfFuel types unit env fuel namespaceId element
              | _ => .ref <$> ntyOfFuel types unit env fuel namespaceId reference.referent
      | .tuple elements =>
          (.tuple ∘ NRow.ofList) <$> elements.toList.mapM (ntyOfFuel types unit env fuel namespaceId)
      | .slice element => .slice <$> ntyOfFuel types unit env fuel namespaceId element
      | .vector element none => .vector <$> ntyOfFuel types unit env fuel namespaceId element
      -- A length past the vectors' bound has no carrier, as the runtime does
      -- not stop at it.
      | .vector element (some (.integer length)) =>
          if length < 0 ∨ 2 ^ 64 ≤ length then none
          else (.array · length.toNat) <$> ntyOfFuel types unit env fuel namespaceId element
      | .nominal name arguments => do
          let handle ← resolveNominal? unit ns name
          if arguments.isEmpty then structNTyFuel types unit .nil fuel handle else
            -- A binder's position is its index; a lifetime or constant
            -- argument has no carrier, and no carrier reads its position.
            let θ ← arguments.toList.mapM fun argument => match argument with
              | .typeArg value => ntyOfFuel types unit env fuel namespaceId value.typeId
              | .lifetime _ | .const _ => some .unit
              | .evidence _ => none
            structNTyFuel types unit (NRow.ofList θ) fuel handle
      | .typeParameter index => some (env.argument index).unmark
      | .function arguments result _ => do
          let parameters ← arguments.toList.mapM (ntyOfFuel types unit env fuel namespaceId)
          -- A generic result instantiated with Unit has the same empty row as
          -- an explicitly Unit-returning callable. A reference to Unit still
          -- returns one value, even though its native carrier erases to Unit.
          let isUnit : Bool := match ← types namespaceId result with
            | .unit => true
            | .typeParameter index => (env.argument index).unmark == .unit
            | _ => false
          let results ← if isUnit then some [] else (fun τ => [τ]) <$> ntyOfFuel types unit env fuel namespaceId result
          let resultIds := if isUnit then [] else [result]
          some (.function (NRow.ofList parameters)
            { parameters := arguments.toList.map (sharedReference types namespaceId)
              results := resultIds.map (sharedReference types namespaceId) }
            (NRow.ofList results))
      | _ => none

/-- The native types of an abstract owner's leading fields: its handle or
placeholder, and its cached length (`OwnerLayout.leadingTypes`). -/
def _root_.VerexIR.Validation.OwnerLayout.leadingRow (layout : Validation.OwnerLayout) : NRow :=
  NRow.ofList ((if layout.canonical.handle then [NTy.address]
      else if layout.canonical.dummy then [NTy.bool] else []) ++
    (if layout.canonical.length then [NTy.int 64 false] else []))

/-- The native type of a nominal declaration: its field row, or its named
variant rows, at the type arguments `env`. The owner of a handle-backed map holds its
entries as a hidden last field at its key and value parameters. -/
def structNTyFuel (types : TypeLookup) (unit : ValidatedUnit) :
    NRow → Nat → StructHandle → Option NTy
  | _, 0, _ => none
  | env, fuel + 1, handle => do
      let targetNs ← unit.namespaces[handle.namespaceId.index]?
      let declaration ← targetNs.structs[handle.structId]?
      let row := fun (fields : Array FieldDecl) =>
        NRow.ofList <$> fields.toList.mapM fun field =>
          ntyOfFuel types unit env fuel handle.namespaceId (targetNs.typeId field.type.typeId)
      let arguments := NRow.ofList
        ((List.range declaration.generics.size).map fun index => env.getD index (.param index))
      -- After the declared fields, a slot per ghost field (`doc/designs/ghost-fields.md`).
      let ghosts ← NRow.ofList <$> declaration.ghosts.toList.mapM fun ghost =>
        NTy.ghost <$> ntyOfFuel types unit env fuel handle.namespaceId (targetNs.typeId ghost.type.typeId)
      let withGhosts := fun (fields : NRow) =>
        if declaration.ghosts.isEmpty then fields else fields ++ ghosts
      -- An intrinsic map owner's native type is its abstract map: its
      -- leading fields, its ghost slots, and its entries, whatever its
      -- declaration holds (`doc/designs/intrinsic-maps.md`, "Abstract
      -- representation").
      match Validation.abstractLayout? unit handle.namespaceId handle.structId with
      | some layout =>
          some (.struct handle arguments
            (withGhosts layout.leadingRow ++
              .cons (.table (env.getD 0 (.param 0)) (env.getD 1 (.param 1))) .nil))
      | none =>
      if declaration.variants.isEmpty then
        some (.struct handle arguments (withGhosts (← row declaration.fields)))
      else
        let names ← declaration.variants.toList.mapM fun variant =>
          (targetNs.nameOf variant.name).map (·.name)
        let rows ← NRows.ofList <$> declaration.variants.toList.mapM fun variant =>
          withGhosts <$> row variant.fields
        if distinct : names.Nodup then some (.enum handle arguments names rows distinct)
        else none
end

/-- The fuel that bounds the nesting of every type in a unit: two steps per
type-table entry, since validated types are not recursive and a nominal
type costs a step of its own. -/
def typeFuel (unit : ValidatedUnit) : Nat :=
  2 * unit.namespaces.foldl (fun total ns => total + ns.tables.types.size) 0 + 2

/-- The fuel that bounds every body in a unit.  A term visits each node at
most once and each node costs at most four steps of the mutual recursion,
so four times the expression count suffices. -/
def unitFuel (unit : ValidatedUnit) : Nat :=
  4 * unit.namespaces.foldl (fun total ns => total + ns.expressions.size) 0 + 4

def ntyOf (unit : ValidatedUnit) (namespaceId : NamespaceId) (typeId : TypeId) : Option NTy :=
  ntyOfFuel (unitTypes unit) unit .nil (typeFuel unit) namespaceId typeId

/-- The native type of a nominal declaration. -/
def structNTy (unit : ValidatedUnit) (handle : StructHandle) : Option NTy :=
  structNTyFuel (unitTypes unit) unit .nil (typeFuel unit) handle

mutual
/-- Whether a type mentions no type parameter. -/
def NTy.paramFree : NTy → Bool
  | .param _ => false
  | .tuple elements => elements.paramFree
  | .struct _ arguments fields => arguments.paramFree && fields.paramFree
  | .enum _ arguments _ rows _ => arguments.paramFree && rows.paramFree
  | .slice element => element.paramFree
  | .vector element => element.paramFree
  | .array element _ => element.paramFree
  | .table key value => key.paramFree && value.paramFree
  | .ghost slot => slot.paramFree
  | .sliceRef referent => referent.paramFree
  | .ref referent => referent.paramFree
  | .function parameters _ results => parameters.paramFree && results.paramFree
  | .nominal _ arguments _ => arguments.paramFree
  | .observed referent => referent.paramFree
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char => true

def NRow.paramFree : NRow → Bool
  | .nil => true
  | .cons τ rest => τ.paramFree && rest.paramFree

def NRows.paramFree : NRows → Bool
  | .nil => true
  | .cons fields rest => fields.paramFree && rest.paramFree
end

/-- A mark says nothing about parameters: what a marked entry stands for mentions
one exactly where the entry does. -/
@[simp] theorem NTy.paramFree_unmark : (τ : NTy) → τ.unmark.paramFree = τ.paramFree
  | .observed referent => NTy.paramFree_unmark referent
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char
  | .tuple _ | .struct _ _ _ | .enum _ _ _ _ _ | .slice _ | .vector _ | .array _ _
  | .table _ _ | .ghost _ | .ref _ | .sliceRef _ | .param _ | .function _ _ _
  | .nominal _ _ _ => rfl

/-- The type arguments of a nominal type. -/
def NTy.nominalArguments? : NTy → Option NRow
  | .struct _ arguments _ | .enum _ arguments _ _ _ => some arguments
  | _ => none

/-- The variant row an enum's name selects. -/
def NRows.variant? : List String → NRows → String → Option NRow
  | candidate :: names, .cons row rows, name =>
      if candidate == name then some row else NRows.variant? names rows name
  | _, _, _ => none

/-- The node of a type parameter in a namespace's type table. -/
def paramNodeIn? (unit : ValidatedUnit) (namespaceId : NamespaceId) (index : Nat) :
    Option TypeId := do
  let ns ← unit.namespaces[namespaceId.index]?
  let node ← ns.tables.types.findIdx? fun type => match type with
    | .typeParameter other => other == index
    | _ => false
  some ⟨node⟩

/-- Type arguments that leave each parameter read as none. -/
theorem ntyOfFuel_identity (types : TypeLookup) (unit : ValidatedUnit) (env : NRow)
    (identity : ∀ index, env.getD index (.param index) = .param index) (fuel : Nat) :
    ∀ namespaceId, ntyOfFuel types unit env fuel namespaceId =
      ntyOfFuel types unit .nil fuel namespaceId := by
  induction fuel with
  | zero => intro _; funext _; simp only [ntyOfFuel]
  | succ fuel ih =>
      intro namespaceId; funext typeId
      simp only [ntyOfFuel, NRow.argument, identity, NTy.unmark, NRow.getD, ih]

/-- A type parameter's node reads as the parameter. -/
theorem ntyOf_paramNode {unit : ValidatedUnit} {namespaceId : NamespaceId} {index : Nat}
    {node : TypeId} (found : paramNodeIn? unit namespaceId index = some node) :
    ntyOf unit namespaceId node = some (.param index) := by
  simp only [paramNodeIn?, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.some.injEq] at found
  obtain ⟨ns, ns_eq, position, position_eq, rfl⟩ := found
  obtain ⟨inBounds, isParameter, _⟩ := Array.findIdx?_eq_some_iff_getElem.mp position_eq
  have entry : unitTypes unit namespaceId ⟨position⟩ = some (.typeParameter index) := by
    simp only [unitTypes, ns_eq, Option.bind_some, Array.getElem?_eq_getElem inBounds]
    split at isParameter
    · next other same => simp only [beq_iff_eq] at isParameter; rw [same, isParameter]
    · cases isParameter
  show ntyOfFuel (unitTypes unit) unit .nil (2 * _ + 1 + 1) namespaceId ⟨position⟩ = _
  simp only [ntyOfFuel, ns_eq, entry, Option.bind_eq_bind, Option.bind_some]

mutual
/-- Whether a semantic type states a shared reference, outside a function
type, whose own sharing marks say where its references are. -/
def _root_.VerexIR.SemTy.holdsShared : SemTy → Bool
  | .reference .shared _ => true
  | .reference .mutable referent => referent.holdsShared
  | .tuple elements => SemTy.holdsSharedList elements
  | .slice element => element.holdsShared
  | .vector element _ => element.holdsShared
  | .nominal _ arguments => SemArg.holdsSharedList arguments
  | .ghost slot => slot.holdsShared
  | .unit | .never | .bool | .character | .string | .bytes | .address | .signer | .integer _ _
  | .function _ _ | .profile _ | .param _ => false

def _root_.VerexIR.SemTy.holdsSharedList : List SemTy → Bool
  | [] => false
  | type :: types => type.holdsShared || SemTy.holdsSharedList types

def _root_.VerexIR.SemArg.holdsSharedList : List SemArg → Bool
  | [] => false
  | .type type :: rest => type.holdsShared || SemArg.holdsSharedList rest
  | _ :: rest => SemArg.holdsSharedList rest
end

/-- Whether a type states a shared reference only at its top level, where a
function type's sharing marks can say so. -/
def _root_.VerexIR.SemTy.sharingAtTop : SemTy → Bool
  | .reference .shared type => !type.holdsShared
  | type => !type.holdsShared

/-- Whether a function's signature states shared references only at the top
level of its parameters and results. -/
def signatureSharingAtTop (targetNs : ValidatedNamespace)
    (declaration : FunctionDecl FunctionBody) : Bool :=
  match StaticTyping.signatureTypes? targetNs declaration
      (StaticTyping.staticEnv declaration.signature.generics) with
  | some (parameterTypes, resultTypes) => (parameterTypes ++ resultTypes).all SemTy.sharingAtTop
  | none => true

/-- A closure target's own parameter and result types, as the compiler
reads its signature; `none` for a function that is not a closure target. A
target stating a shared reference below the top level of a parameter or result
is not one: a native type erases the reference there, so no sharing mark can
record it. -/
def closureSignature? (unit : ValidatedUnit) (function : FunctionHandle) :
    Option (List NTy × List NTy) := do
  let targetNs ← unit.namespaces[function.namespaceId.index]?
  let declaration ← targetNs.functions[function.functionId.index]?
  let parameters ← declaration.signature.parameters.toList.mapM fun parameter =>
    ntyOf unit function.namespaceId (targetNs.typeId parameter.typeUse.typeId)
  let results ← declaration.signature.results.toList.mapM fun result =>
    ntyOf unit function.namespaceId (targetNs.typeId result.typeId)
  -- Its types mention only its own type parameters, each read at its node.
  guard ((parameters ++ results).all fun τ => τ.paramsAll fun index =>
    (declaration.signature.generics[index]?.any fun binder => binder.kind matches .typeArg) &&
      (paramNodeIn? unit function.namespaceId index).isSome)
  guard (signatureSharingAtTop targetNs declaration)
  some (parameters, results)

/-! ### Native and semantic readings -/

/-- A function value has the same result row as its target: no value for
unit, otherwise one value, including a tuple. Keeping tuples packed preserves
single-element tuples and substitution of a generic result by a tuple. -/
def _root_.VerexIR.SemTy.resultRow : SemTy → List SemTy
  | .unit => []
  | type => [type]

/-- A function's results packed into one type, as a function type states
them: the unit for none, the one type, or a tuple. -/
def _root_.VerexIR.SemTy.pack : List SemTy → SemTy
  | [] => .unit
  | [type] => type
  | types => .tuple types


/-- The number of generic binders a structure or enum declares. -/
def declarationArity? (unit : ValidatedUnit) (source : StructHandle) : Option Nat := do
  let ns ← unit.namespaces[source.namespaceId.index]?
  let declaration ← ns.structs[source.structId]?
  some declaration.generics.size

/-- The names of an enum declaration's variants, in order. -/
def declarationVariants? (unit : ValidatedUnit) (source : StructHandle) : Option (List String) := do
  let ns ← unit.namespaces[source.namespaceId.index]?
  let declaration ← ns.structs[source.structId]?
  declaration.variants.toList.mapM fun variant =>
    (ns.nameOf variant.name).map (·.name)

/-- The field types of a declaration under semantic arguments, by
evaluation. -/
def declaredTypes? (ns : ValidatedNamespace) (arguments : List SemArg)
    (declared : Array FieldDecl) : Option (List SemTy) :=
  (declared.toList.map (ns.typeId ·.type.typeId)).mapM (StaticTyping.resolveAt ns arguments.toArray)

/-- The ghost field types of a declaration under semantic arguments, by
evaluation. -/
def ghostDeclaredTypes? (ns : ValidatedNamespace) (arguments : List SemArg)
    (ghosts : List TypeId) : Option (List SemTy) :=
  ghosts.mapM (StaticTyping.resolveAt ns arguments.toArray)

mutual
/-- Whether a native type reads as a semantic type that is not a shared
reference, decided by evaluation over the unit's declarations. -/
def NTy.typedAsCore (unit : ValidatedUnit) : NTy → SemTy → Bool
  | .unit, type => type == .unit
  | .bool, type => type == .bool
  | .int width signed, type => type == .integer (.bits width) signed
  | .num, type => type == .integer .unbounded true
  | .address, type => type == .address
  | .signer, type => type == .signer
  | .string, type => type == .string
  | .bytes, type => type == .bytes
  | .char, type => type == .character
  | .tuple elements, .tuple types => NRow.typedAsCheck unit elements types
  | .slice element, .slice type => NTy.typedAsCore unit element type
  | .sliceRef element, .reference .mutable (.slice type) => NTy.typedAsCore unit element type
  | .vector element, .vector type none => NTy.typedAsCore unit element type
  | .array element length, .vector type (some (.integer actual)) =>
      actual == (length : Int) && NTy.typedAsCore unit element type
  | .table key value, .vector (.tuple [keyType, valueType]) none =>
      NTy.typedAsCore unit key keyType && NTy.typedAsCore unit value valueType
  -- The declared fields read as the declaration's, then its ghost slots, and
  -- a table owner's hidden contents field as its entries (`contentsTypes`).
  | .struct source nativeArguments fields, .nominal name arguments =>
      SemanticOperations.structName? unit source == some name &&
        declarationArity? unit source == some arguments.length &&
        NRow.argumentsCheck unit nativeArguments arguments &&
        match Validation.abstractLayout? unit source.namespaceId source.structId with
        | some layout =>
          -- An abstract owner's fields are its canonical value's.
          match arguments, unit.namespaces[source.namespaceId.index]? with
          | [.type key, .type value], some declaringNs =>
            match ghostDeclaredTypes? declaringNs arguments (ghostTypeIds unit source) with
            | some ghostTypes =>
                NRow.typedAsCheck unit fields (layout.canonicalTypes ghostTypes key value)
            | none => false
          | _, _ => false
        | none =>
        match SemanticOperations.handleFields? unit source none with
        | some (declaringNs, declared) =>
          match declaredTypes? declaringNs arguments declared,
              ghostDeclaredTypes? declaringNs arguments (ghostTypeIds unit source) with
          | some fieldTypes, some ghostTypes =>
              NRow.typedAsCheck unit fields
                (fieldTypes ++ (ghostTypes.map .ghost ++ contentsTypes unit source none arguments))
          | _, _ => false
        | none => false
  | .enum source nativeArguments names rows _, .nominal name arguments =>
      SemanticOperations.structName? unit source == some name &&
        declarationArity? unit source == some arguments.length &&
        NRow.argumentsCheck unit nativeArguments arguments && !names.isEmpty &&
        declarationVariants? unit source == some names &&
        NRows.variantsCheck unit source arguments names rows
  | .ref referent, .reference .mutable type => NTy.typedAsCore unit referent type
  | .ghost slot, .ghost type => NTy.typedAsCore unit slot type
  | .param index, .param other => index == other
  | .function parameters shared returned, .function parameterTypes result =>
      NRow.parametersCheck unit parameters shared.parameters parameterTypes &&
        SemTy.pack result.resultRow == result &&
        NRow.parametersCheck unit returned shared.results result.resultRow
  | _, _ => false

/-- Whether a row of native types reads as a row of semantic types. -/
def NRow.typedAsCheck (unit : ValidatedUnit) : NRow → List SemTy → Bool
  | .nil, [] => true
  | .cons τ rest, type :: types =>
      NTy.typedAsCore unit τ type && NRow.typedAsCheck unit rest types
  | _, _ => false

/-- Whether a row of native types reads as type arguments. -/
def NRow.argumentsCheck (unit : ValidatedUnit) : NRow → List SemArg → Bool
  | .nil, [] => true
  | .cons τ rest, .type type :: arguments =>
      NTy.typedAsCore unit τ type && NRow.argumentsCheck unit rest arguments
  | _, _ => false

/-- Whether a function type's parameter row reads as semantic parameters. -/
def NRow.parametersCheck (unit : ValidatedUnit) : NRow → List Bool → List SemTy → Bool
  | .nil, [], [] => true
  | .cons τ rest, false :: shared, type :: types =>
      type.unshared == type && NTy.typedAsCore unit τ type &&
        NRow.parametersCheck unit rest shared types
  | .cons τ rest, true :: shared, .reference .shared type :: types =>
      type.unshared == type && NTy.typedAsCore unit τ type &&
        NRow.parametersCheck unit rest shared types
  | _, _, _ => false

/-- Whether an enum's variant rows read as its variants' declared fields. -/
def NRows.variantsCheck (unit : ValidatedUnit) (source : StructHandle)
    (arguments : List SemArg) : List String → NRows → Bool
  | [], .nil => true
  | name :: names, .cons fields rest =>
      (match SemanticOperations.handleFields? unit source (some name) with
        | some (declaringNs, declared) =>
          match declaredTypes? declaringNs arguments declared,
              ghostDeclaredTypes? declaringNs arguments (ghostTypeIds unit source) with
          | some fieldTypes, some ghostTypes =>
              NRow.typedAsCheck unit fields (fieldTypes ++ ghostTypes.map .ghost)
          | _, _ => false
        | none => false) &&
        NRows.variantsCheck unit source arguments names rest
  | _, _ => false
end

/-- Whether a native type reads as a semantic type. -/
def NTy.typedAsCheck (unit : ValidatedUnit) (τ : NTy) (type : SemTy) : Bool :=
  NTy.typedAsCore unit τ type

/-- Which of a closure target's supplied parameters, split by its mask, and
which of its results are shared references. -/
def closureShared (unit : ValidatedUnit) (function : FunctionHandle) (mask : Nat) : Sharing :=
  match unit.namespaces[function.namespaceId.index]? with
  | some targetNs => match targetNs.functions[function.functionId.index]? with
    | some declaration =>
        { parameters := ClosureMask.extract mask false
            (declaration.signature.parameters.toList.map fun parameter =>
              sharedReference (unitTypes unit) function.namespaceId
                (targetNs.typeId parameter.typeUse.typeId))
          results := declaration.signature.results.toList.map fun result =>
            sharedReference (unitTypes unit) function.namespaceId (targetNs.typeId result.typeId) }
    | none => ⟨[], []⟩
  | none => ⟨[], []⟩

/-- A closure target's own rows: the parameters it captures and those it is
supplied, split by its mask, and its results. -/
def closureRows? (unit : ValidatedUnit) (function : FunctionHandle) (mask : Nat) :
    Option (NRow × NRow × NRow) := do
  let (parameters, results) ← closureSignature? unit function
  guard (mask < 2 ^ parameters.length)
  -- A closure captures no reference.
  guard (NRow.ofList (ClosureMask.extract mask true parameters)).refFree
  some (NRow.ofList (ClosureMask.extract mask true parameters),
    NRow.ofList (ClosureMask.extract mask false parameters), NRow.ofList results)

theorem closureRows?_eq_some {unit : ValidatedUnit} {function : FunctionHandle} {mask : Nat}
    {captured supplied results : NRow} :
    closureRows? unit function mask = some (captured, supplied, results) ↔
      ∃ parameters returned, closureSignature? unit function = some (parameters, returned) ∧
        mask < 2 ^ parameters.length ∧
        (NRow.ofList (ClosureMask.extract mask true parameters)).refFree = true ∧
        captured = NRow.ofList (ClosureMask.extract mask true parameters) ∧
        supplied = NRow.ofList (ClosureMask.extract mask false parameters) ∧
        results = NRow.ofList returned := by
  have failure_none : (failure : Option Unit) = none := rfl
  rw [closureRows?]
  match signature : closureSignature? unit function with
  | none => simp
  | some (parameters, returned) =>
      simp only [Option.bind_eq_bind, Option.bind_some, Option.some.injEq, Prod.mk.injEq]
      by_cases bounded : mask < 2 ^ parameters.length
      · by_cases free : (NRow.ofList (ClosureMask.extract mask true parameters)).refFree = true
        · simp only [guard, bounded, free, if_true, Option.pure_def, Option.bind_some, Option.some.injEq, Prod.mk.injEq]
          constructor
          · rintro ⟨rfl, rfl, rfl⟩
            exact ⟨parameters, returned, ⟨rfl, rfl⟩, bounded, free, rfl, rfl, rfl⟩
          · rintro ⟨_, _, ⟨rfl, rfl⟩, -, -, rfl, rfl, rfl⟩
            exact ⟨rfl, rfl, rfl⟩
        · simp [guard, bounded, free, failure_none]
      · simp [guard, bounded, failure_none]

/-- The argument a runtime type instantiation gives a closure target's type
parameter: the native type of what it maps the parameter's node to. A
parameter without a node, or whose argument has no native type, stays. -/
def closureArgument (unit : ValidatedUnit) (function : FunctionHandle)
    (typeInstantiation : Array (TypeId × TypeId)) (index : Nat) : NTy :=
  match paramNodeIn? unit function.namespaceId index with
  | some node =>
      (ntyOf unit function.namespaceId (instantiatedTypeId typeInstantiation node)).getD
        (.param index)
  | none => .param index

mutual
/-- Whether a semantic type states neither a reference outside a function
type's parameters nor `!`. -/
def _root_.VerexIR.SemTy.referenceFree : SemTy → Bool
  | .reference _ _ => false
  | .tuple elements => SemTy.referenceFreeList elements
  | .slice element => element.referenceFree
  | .vector element _ => element.referenceFree
  | .nominal _ arguments => SemArg.referenceFreeList arguments
  | .never => false
  | .unit | .bool | .character | .string | .bytes | .address | .signer | .integer _ _
  | .function _ _ | .profile _ | .param _ => true
  | .ghost slot => slot.referenceFree

def _root_.VerexIR.SemTy.referenceFreeList : List SemTy → Bool
  | [] => true
  | type :: types => type.referenceFree && SemTy.referenceFreeList types

def _root_.VerexIR.SemArg.referenceFreeList : List SemArg → Bool
  | [] => true
  | .type type :: rest => type.referenceFree && SemArg.referenceFreeList rest
  | _ :: rest => SemArg.referenceFreeList rest
end

/-- The type arguments a runtime type instantiation gives a function's
generic binders, read at its type parameters' nodes; a binder without a
node is given none, as no type reads it. -/
def frameArguments (unit : ValidatedUnit) (function : FunctionHandle)
    (typeInstantiation : Array (TypeId × TypeId)) (generics : Array GenericBinder) :
    Array GenericArgument :=
  generics.mapIdx fun index binder =>
    match binder.kind, paramNodeIn? unit function.namespaceId index with
    | .typeArg, some node => .typeArg ⟨instantiatedTypeId typeInstantiation node, ⟨0⟩⟩
    | _, _ => .lifetime ⟨0⟩

/-- Whether a runtime type instantiation is faithful to a function's frame,
as the runtime builds one (`FrameInstantiation`): for a function without
type parameters, the empty one; for another, the instantiation of the
closed arguments it gives the function's type parameters, which state no
reference and have closed native types, at which every type the function
requires is interned. -/
def closureFaithful (unit : ValidatedUnit) (function : FunctionHandle)
    (typeInstantiation : Array (TypeId × TypeId)) : Bool :=
  match unit.namespaces[function.namespaceId.index]? with
  | none => false
  | some targetNs => match targetNs.functions[function.functionId.index]? with
    | none => false
    | some declaration =>
        if declaration.signature.generics.all (·.kind != .typeArg) then typeInstantiation.isEmpty
        else
        let arguments := frameArguments unit function typeInstantiation
          declaration.signature.generics
        typeInstantiation ==
            SemanticOperations.invocationTypeInstantiation targetNs #[] arguments &&
          arguments.all (fun argument => match argument with
            | .typeArg value => match StaticTyping.resolveAt targetNs #[] value.typeId with
              | some type => type.sized && type.referenceFree &&
                  (ntyOf unit function.namespaceId value.typeId).any
                    fun τ => τ.paramFree && τ.refFree
              | none => false
            | _ => true) &&
          (requiredAt unit function).all fun typeId =>
            (instantiatePlaceFieldType? targetNs arguments typeId).isSome

/-- The rows of a closure's target at its runtime type instantiation: its
own rows with each type parameter replaced by its argument. -/
def closureRowsIn? (unit : ValidatedUnit) (function : FunctionHandle) (mask : Nat)
    (typeInstantiation : Array (TypeId × TypeId)) : Option (NRow × NRow × NRow) :=
  (closureRows? unit function mask).map fun (captured, supplied, results) =>
    let argument := closureArgument unit function typeInstantiation
    (captured.substWith argument, supplied.substWith argument, results.substWith argument)

/-- The rows of a nominal at its type arguments: its declaration read as `structNTy` reads it. -/
def NTy.nominalBody? (unit : ValidatedUnit) (source : StructHandle) (arguments : NRow) :
    Option NTy :=
  structNTyFuel (unitTypes unit) unit arguments (typeFuel unit) source

/-- The row of a nominal's body a value's variant selects: a structure's, or the named variant's. -/
def NTy.nominalRow? : Option String → Option NTy → Option NRow
  | none, some (.struct _ _ fields) => some fields
  | some name, some (.enum _ _ names rows _) => NRows.variant? names rows name
  | _, _ => none

mutual
/-- A value of a type, as its encoding holds it, searched for by its structure: an enum by
the first variant that has one, a type parameter assumed inhabited, a function type without
one, as its values are the unit's typed closures. The fuel bounds the unfolding of nominals. -/
def NTy.witnessAt (unit : ValidatedUnit) : Nat → NTy → Option RuntimeValue
  | _, .unit => some .unit
  | _, .bool => some (.bool false)
  | _, .int width _ => if width == 0 then none else some (.integer 0)
  | _, .num => some (.integer 0)
  | _, .address => some (.address "")
  | _, .signer => some (.signer "")
  | _, .string => some (.string "")
  | _, .bytes => some (.bytes #[])
  | _, .char => some (.character 0)
  | fuel, .tuple elements =>
      match NRow.witnessAt unit fuel elements with
      | some values => some (.tuple values.toArray)
      | none => none
  | fuel, .struct source _ fields =>
      match NRow.witnessAt unit fuel fields with
      | some values => some (.nominal source none values.toArray)
      | none => none
  | fuel, .enum source _ names rows _ => NRows.witnessAt unit fuel source names rows
  | _, .sliceRef _ => some (.tuple #[.vector #[], .vector #[]])
  | _, .slice _ => some (.vector #[])
  | _, .vector _ => some (.vector #[])
  | fuel, .array element length =>
      if 2 ^ 64 ≤ length then none
      else if length = 0 then some (.vector #[])
      else match NTy.witnessAt unit fuel element with
        | some value => some (.vector (Array.replicate length value))
        | none => none
  | _, .table _ _ => some (.vector #[])
  | fuel, .ghost slot =>
      match NTy.witnessAt unit fuel slot with
      | some value => some (.ghost value)
      | none => none
  | fuel, .ref referent =>
      match NTy.witnessAt unit fuel referent with
      | some value => some (.tuple #[value, value])
      | none => none
  | fuel, .observed referent => NTy.witnessAt unit fuel referent
  | _, .param _ => some .unit
  | _, .function _ _ _ => none
  | _, .nominal _ _ false => none
  | 0, .nominal _ _ true => none
  | fuel + 1, .nominal source arguments true =>
      match NTy.nominalBody? unit source arguments with
      | some body => NTy.witnessAt unit fuel body
      | none => none
termination_by fuel τ => (fuel, sizeOf τ)

/-- Values of a row, in order. -/
def NRow.witnessAt (unit : ValidatedUnit) : Nat → NRow → Option (List RuntimeValue)
  | _, .nil => some []
  | fuel, .cons τ rest =>
      match NTy.witnessAt unit fuel τ, NRow.witnessAt unit fuel rest with
      | some value, some values => some (value :: values)
      | _, _ => none
termination_by fuel row => (fuel, sizeOf row)

/-- A value of the first variant that has one. -/
def NRows.witnessAt (unit : ValidatedUnit) :
    Nat → StructHandle → List String → NRows → Option RuntimeValue
  | fuel, source, name :: names, .cons fields rest =>
      match NRow.witnessAt unit fuel fields with
      | some values => some (.nominal source (some name) values.toArray)
      | none => NRows.witnessAt unit fuel source names rest
  | _, _, _, _ => none
termination_by fuel _ _ rows => (fuel, sizeOf rows)
end

/-- A value of a nominal at its type arguments, if its declaration has one. -/
def productiveWitness? (unit : ValidatedUnit) (source : StructHandle) (arguments : NRow) :
    Option RuntimeValue :=
  match NTy.nominalBody? unit source arguments with
  | some body => NTy.witnessAt unit (typeFuel unit) body
  | none => none

/-- The value a nominal flagged productive admits whatever its rows say: its witness, which
its rows admit when the flag is the unit's productivity, or `.unit` for a flag no typed type
carries (`doc/designs/rust-verification.md`, "Inhabitants"). -/
def witnessOrUnit (unit : ValidatedUnit) (source : StructHandle) (arguments : NRow) : RuntimeValue :=
  match productiveWitness? unit source arguments with
  | some witness => if plain? witness then witness else .unit
  | none => .unit

theorem witnessOrUnit_plain (unit : ValidatedUnit) (source : StructHandle) (arguments : NRow) :
    Plain (witnessOrUnit unit source arguments) := by
  unfold witnessOrUnit
  split
  · next witness _ =>
      split
      · next plain => exact (plain?_iff witness).mp plain
      · exact .unit
  · exact .unit
private theorem sizeOf_toList_lt (values : Array RuntimeValue) :
    sizeOf values.toList < 1 + sizeOf values := by
  cases values
  simp only [Array.mk.sizeOf_spec]
  omega

mutual
/-- Whether a runtime value encodes a value of a native type at the unit's
runtime family, decided on the value's structure: a closure by its target's
rows at its frame (`closureRowsIn?`) and its captures at the captured row. -/
def NTy.admits (unit : ValidatedUnit) : NTy → RuntimeValue → Bool
  | .unit, .unit => true
  | .bool, .bool _ => true
  | .int width signed, .integer value => (decodeInt? (.bits width) signed (.integer value)).isSome
  | .num, .integer _ => true
  | .address, .address _ => true
  | .signer, .signer _ => true
  | .string, .string _ => true
  | .bytes, .bytes _ => true
  | .char, .character value => isUnicodeScalar value
  | .tuple elements, .tuple values => NRow.admits unit elements values.toList
  | .struct source _ fields, .nominal actual none values =>
      actual == source && NRow.admits unit fields values.toList
  | .enum source _ names rows _, .nominal actual (some name) values =>
      actual == source && match NRows.variant? names rows name with
        | some row => NRow.admits unit row values.toList
        | none => false
  | .slice element, .vector values => NTy.admitsEach unit element values.toList
  | .vector element, .vector values => NTy.admitsEach unit element values.toList
  | .array element length, .vector values =>
      values.size == length && NTy.admitsEach unit element values.toList
  | .table key value, .vector entries => NTy.admitsEntries unit key value entries.toList
  | .sliceRef element, .tuple values =>
      (match values.toList with
        | [.vector current, .vector final] => current.size == final.size
        | _ => false) && NTy.admitsPair unit (.vector element) values.toList
  | .ref referent, .tuple values => NTy.admitsPair unit referent values.toList
  -- The runtime ignores a ghost slot's contents (`doc/designs/ghost-fields.md`).
  | .ghost _, .ghost contents => SemanticOperations.plain? contents
  | .param _, _ => true
  -- A substitution's marked entry is no type of a value, and like a type
  -- parameter it constrains no runtime value: the family decides.
  | .observed _, _ => true
  | .function parameters shared results, .closure function mask typeInstantiation captures =>
      closureFaithful unit function typeInstantiation &&
        match closureRowsIn? unit function mask typeInstantiation with
        | some (captured, supplied, returned) =>
            supplied == parameters && closureShared unit function mask == shared &&
              returned == results && NRow.admits unit captured captures.toList
        | none => false
  | .nominal source arguments productive, .nominal actual variant values =>
      (plain? (.nominal actual variant values) && actual == source &&
        match NTy.nominalRow? variant (NTy.nominalBody? unit source arguments) with
        | some row => NRow.admits unit row values.toList
        | none => false) ||
      (productive && RuntimeValue.nominal actual variant values == witnessOrUnit unit source arguments)
  | .nominal source arguments productive, value =>
      productive && value == witnessOrUnit unit source arguments
  | _, _ => false
-- A marked entry is read at its referent, the value unchanged, so the type
-- orders the recursion after the value.
termination_by _ value => sizeOf value
decreasing_by
  all_goals simp_wf
  all_goals first
    | omega
    | (have := sizeOf_toList_lt ‹Array RuntimeValue›; omega)

/-- Whether runtime values encode the values of a row, in order. -/
def NRow.admits (unit : ValidatedUnit) : NRow → List RuntimeValue → Bool
  | .nil, [] => true
  | .cons τ rest, value :: values => NTy.admits unit τ value && NRow.admits unit rest values
  | _, _ => false
termination_by _ values => sizeOf values

/-- Whether runtime values each encode a value of one type. -/
def NTy.admitsEach (unit : ValidatedUnit) (τ : NTy) : List RuntimeValue → Bool
  | [] => true
  | value :: values => NTy.admits unit τ value && NTy.admitsEach unit τ values
termination_by values => sizeOf values

/-- Whether runtime values are a reference's current value and prophecy. -/
def NTy.admitsPair (unit : ValidatedUnit) (τ : NTy) : List RuntimeValue → Bool
  | [current, prophecy] => NTy.admits unit τ current && NTy.admits unit τ prophecy
  | _ => false
termination_by values => sizeOf values

/-- Whether runtime values are the entries of a map: each a key and a value. -/
def NTy.admitsEntries (unit : ValidatedUnit) (key value : NTy) : List RuntimeValue → Bool
  | [] => true
  | .tuple entry :: entries =>
      NTy.admitsEntry unit key value entry.toList && NTy.admitsEntries unit key value entries
  | _ :: _ => false
termination_by entries => sizeOf entries
decreasing_by
  all_goals simp_wf
  all_goals first
    | omega
    | (have := sizeOf_toList_lt ‹Array RuntimeValue›; omega)

/-- Whether runtime values are an entry: a key and a value. -/
def NTy.admitsEntry (unit : ValidatedUnit) (key value : NTy) : List RuntimeValue → Bool
  | [k, v] => NTy.admits unit key k && NTy.admits unit value v
  | _ => false
termination_by values => sizeOf values
end

end NativeTypes

/-- The typed values of a type's values (`NTy.read`): their type, the read
of a native value, the runtime value of a typed value and the typed value a
runtime value reads back to, inverse to each other and keeping the type's
encoding. -/
structure TypedReads (carrier : Type) (encode : carrier → RuntimeValue) where
  typed : Type
  read : carrier → typed
  decEq : DecidableEq typed
  inhabited : Inhabited typed
  write : typed → RuntimeValue
  back : RuntimeValue → typed
  back_write : ∀ value, back (write value) = value
  back_unit : back .unit = inhabited.default
  write_read : ∀ value, write (read value) = encode value
  /-- The native value a typed value stands for, inverse to the read. -/
  unread : typed → carrier
  unread_read : ∀ value, unread (read value) = value
  /-- The order of the typed values, as an ordered map's typed keys compare. -/
  order : ValueRanks → SpecMap.KeyOrder typed

/-- The key order of values by the runtime order of their images, where
the images are injective: the order of runtime values themselves, which
the runtime family's type parameters are. -/
def SpecMap.KeyOrder.byRuntime {α : Type} (image : α → RuntimeValue)
    (injective : Function.Injective image) (rank : ValueRanks) : SpecMap.KeyOrder α where
  compare left right := RuntimeValue.order rank (image left) (image right)
  lawful := {
    compare_self := RuntimeValue.order_self _ _
    eq_of_compare := fun h => injective (RuntimeValue.eq_of_order h) }
  trans := {
    eq_swap := RuntimeValue.order_swap _ _ _
    isLE_trans := fun h₁ h₂ => RuntimeValue.order_isLE_trans _ _ _ _ h₁ h₂ }

/-- The reads of units: the unit itself. The bundles are reducible, as
instances are: a family's reads unfold to the type's under unification. -/
@[reducible] def TypedReads.unit : TypedReads Unit Codec.unit.encode where
  typed := Unit
  read := fun _ => ()
  decEq := inferInstanceAs (DecidableEq Unit)
  inhabited := ⟨()⟩
  write := fun _ => .unit
  back := fun _ => ()
  back_write := fun _ => rfl
  back_unit := rfl
  write_read := fun _ => rfl
  unread := fun _ => ()
  unread_read := fun _ => rfl
  order := SpecMap.KeyOrder.byRuntime (fun _ => .unit) (fun _ _ _ => rfl)

/-- A loan-free runtime value: the runtime family's value of a type
parameter. -/
abbrev PlainValue : Type := { value : RuntimeValue // Plain value }

open Classical in
/-- The plain value a runtime value reads back to: itself where it is plain,
the unit otherwise. -/
noncomputable def PlainValue.ofRuntime (raw : RuntimeValue) : PlainValue :=
  if plain : Plain raw then ⟨raw, plain⟩ else ⟨.unit, .unit⟩

theorem PlainValue.ofRuntime_val (value : PlainValue) : PlainValue.ofRuntime value.val = value := by
  simp [PlainValue.ofRuntime, value.property]

theorem PlainValue.ofRuntime_unit : PlainValue.ofRuntime .unit = ⟨.unit, .unit⟩ := by
  rw [PlainValue.ofRuntime, dif_pos .unit]

/-- The runtime family's reads: its values as they are. -/
@[reducible] noncomputable def PlainValue.reads : TypedReads PlainValue Subtype.val where
  typed := PlainValue
  read := fun value => value
  decEq := fun left right => Classical.propDecidable (left = right)
  inhabited := ⟨⟨.unit, .unit⟩⟩
  write := Subtype.val
  back := PlainValue.ofRuntime
  back_write := PlainValue.ofRuntime_val
  back_unit := PlainValue.ofRuntime_unit
  write_read := fun _ => rfl
  unread := fun value => value
  unread_read := fun _ => rfl
  order := SpecMap.KeyOrder.byRuntime Subtype.val fun _ _ same => Subtype.ext same

mutual
/-- A runtime value with its ghost slots' contents dropped: the value the
runtime holds where the denotation carries ghost contents
(`doc/designs/ghost-fields.md`). A function value's captures are untyped, so
its equality compares them so. -/
def _root_.VerexIR.RuntimeValue.eraseGhosts : RuntimeValue → RuntimeValue
  | .ghost _ => .ghost .unit
  | .vector values => .vector (RuntimeValue.eraseGhostsAll values.toList).toArray
  | .tuple values => .tuple (RuntimeValue.eraseGhostsAll values.toList).toArray
  | .nominal source variant values =>
      .nominal source variant (RuntimeValue.eraseGhostsAll values.toList).toArray
  | .closure function mask instantiation captures =>
      .closure function mask instantiation (RuntimeValue.eraseGhostsAll captures.toList).toArray
  | .borrow loan current => .borrow loan current.eraseGhosts
  | .unit => .unit
  | .bool value => .bool value
  | .character value => .character value
  | .integer value => .integer value
  | .address value => .address value
  | .signer value => .signer value
  | .string value => .string value
  | .bytes value => .bytes value
  | .loanHole loan => .loanHole loan
  | .retired => .retired
termination_by value => sizeOf value
decreasing_by
  all_goals simp_wf
  all_goals first
    | omega
    | (have := sizeOf_toList_lt ‹Array RuntimeValue›; omega)

def _root_.VerexIR.RuntimeValue.eraseGhostsAll : List RuntimeValue → List RuntimeValue
  | [] => []
  | value :: values => value.eraseGhosts :: RuntimeValue.eraseGhostsAll values
termination_by values => sizeOf values
end

attribute [simp] RuntimeValue.eraseGhostsAll

/-- An equality of a carrier that is the equality of its encodings with
their ghost slots' contents dropped: how a type parameter's values compare
where ghost slots are not read (`NTy.equal`). -/
structure TypedErasure (carrier : Type) (encode : carrier → RuntimeValue) where
  same : carrier → carrier → Bool
  same_eq : ∀ left right,
    same left right = decide ((encode left).eraseGhosts = (encode right).eraseGhosts)

/-- The erasure of units: all equal. -/
@[reducible] def TypedErasure.unit : TypedErasure Unit Codec.unit.encode where
  same := fun _ _ => true
  same_eq := fun _ _ => by simp [Codec.unit, RuntimeValue.eraseGhosts]

/-- The runtime family's erasure: its values' encodings without ghosts. -/
@[reducible] def PlainValue.erasures : TypedErasure PlainValue Subtype.val where
  same := fun left right => decide (left.val.eraseGhosts = right.val.eraseGhosts)
  same_eq := fun _ _ => rfl

/-- An order of a carrier that is the structural order of its encodings: how
a type parameter's values compare. -/
structure TypedOrder (carrier : Type) (encode : carrier → RuntimeValue) where
  order : ValueRanks → carrier → carrier → Ordering
  order_encode : ∀ rank left right,
    RuntimeValue.order rank (encode left) (encode right) = order rank left right

/-- A typed order is a total order: the runtime order of the encodings. -/
instance {carrier : Type} {encode : carrier → RuntimeValue} {order : TypedOrder carrier encode}
    {rank : ValueRanks} : Std.TransCmp (order.order rank) where
  eq_swap := by
    intro left right
    rw [← order.order_encode, ← order.order_encode]
    exact RuntimeValue.order_swap _ _ _
  isLE_trans := by
    intro first second third below above
    rw [← order.order_encode] at below above ⊢
    exact RuntimeValue.order_isLE_trans _ _ _ _ below above

/-- The order of units: all equal. -/
@[reducible] def TypedOrder.unit : TypedOrder Unit Codec.unit.encode where
  order := fun _ _ _ => .eq
  order_encode := fun rank _ _ => RuntimeValue.order_unit rank

/-- The runtime family's order: the structural order of its values. -/
@[reducible] def PlainValue.orders : TypedOrder PlainValue Subtype.val where
  order := fun rank left right => RuntimeValue.order rank left.val right.val
  order_encode := fun _ _ _ => rfl

/-- What a function value's equality compares: its runtime closure without
ghost slots, behind a type of its own, so that verification states the
equality of function values as an equation of these keys and never reads
the runtime closure. -/
structure ErasedClosure where
  value : RuntimeValue
  deriving DecidableEq

/-- A closure's equality key (`ErasedClosure`). -/
def ClosureValue.erased (closure : ClosureValue) : ErasedClosure := ⟨closure.encode.eraseGhosts⟩

/-- The values of a function type as a family holds them: each is a runtime
closure holding no loan (`closure`), and a runtime closure is one where the
family types it (`ofClosure?`). A function value is constructed, invoked,
compared, and ordered through these alone, and verification reads none of
them: its equality is its key without ghost slots (`erased`), its order
Move's order of closures (`doc/designs/higher-order-functions.md`). -/
structure TypedFunctions (carrier : Type) where
  closure : carrier → ClosureValue
  ofClosure? : ClosureValue → Option carrier
  ofClosure?_closure : ∀ value, ofClosure? (closure value) = some value
  closure_of_ofClosure? : ∀ raw value, ofClosure? raw = some value → closure value = raw

namespace TypedFunctions
variable {carrier : Type} (functions : TypedFunctions carrier)

theorem closure_injective {left right : carrier}
    (same : functions.closure left = functions.closure right) : left = right := by
  have read := functions.ofClosure?_closure left
  rw [same, functions.ofClosure?_closure right] at read
  exact (Option.some.inj read).symm

def decEq : DecidableEq carrier := fun left right =>
  decidable_of_iff (functions.closure left = functions.closure right)
    ⟨functions.closure_injective, fun same => same ▸ rfl⟩

/-- The codec of function values: a runtime closure without loans decodes
where the family types it. -/
def codec : Codec carrier RuntimeValue where
  encode := fun value => (functions.closure value).encode
  decode? := fun raw => (ClosureValue.decode? raw).bind functions.ofClosure?
  decode_encode := fun value => by
    have decoded : ClosureValue.decode? (functions.closure value).encode =
        some (functions.closure value) := Codec.closure.decode_encode _
    simp only [decoded, Option.bind_some, functions.ofClosure?_closure]

theorem tight : functions.codec.Tight := fun raw value decoded => by
  simp only [codec, Option.bind_eq_some_iff] at decoded
  obtain ⟨closure, decodedClosure, typed⟩ := decoded
  show (functions.closure value).encode = raw
  rw [functions.closure_of_ofClosure? closure value typed]
  exact Codec.closure_tight raw closure decodedClosure

theorem plain (value : carrier) : Plain (functions.codec.encode value) :=
  .closure _ _ _ _ (functions.closure value).plain

/-- Move's order of function values: their closures'. -/
def order (rank : ValueRanks) (left right : carrier) : Ordering :=
  ClosureValue.order rank (functions.closure left) (functions.closure right)

def orders : TypedOrder carrier functions.codec.encode where
  order := functions.order
  order_encode := fun rank left right => ClosureValue.order_encode rank _ _

/-- A function value's equality key: its closure's without ghost slots. -/
def erased (value : carrier) : ErasedClosure := (functions.closure value).erased

def erasures : TypedErasure carrier functions.codec.encode where
  same := fun left right => decide (functions.erased left = functions.erased right)
  same_eq := fun _ _ => by
    simp only [erased, ClosureValue.erased, ErasedClosure.mk.injEq]
    rfl

/-- The family without function values. -/
@[reducible] def empty : TypedFunctions Empty where
  closure := Empty.elim
  ofClosure? := fun _ => none
  ofClosure?_closure := fun value => value.elim
  closure_of_ofClosure? := fun _ _ decoded => nomatch decoded

end TypedFunctions

/-- The values of a recursive nominal as a family holds them: each is a loan-free runtime
value (`value`), and a runtime value is one where the family types it (`ofValue?`). -/
structure TypedNominals (carrier : Type) where
  value : carrier → RuntimeValue
  ofValue? : RuntimeValue → Option carrier
  ofValue?_value : ∀ held, ofValue? (value held) = some held
  value_of_ofValue? : ∀ raw held, ofValue? raw = some held → value held = raw
  plain : ∀ held, Plain (value held)

namespace TypedNominals
variable {carrier : Type} (nominals : TypedNominals carrier)

theorem value_injective {left right : carrier} (same : nominals.value left = nominals.value right) :
    left = right := by
  have read := nominals.ofValue?_value left
  rw [same, nominals.ofValue?_value right] at read
  exact (Option.some.inj read).symm

def decEq : DecidableEq carrier := fun left right =>
  decidable_of_iff (nominals.value left = nominals.value right)
    ⟨nominals.value_injective, fun same => same ▸ rfl⟩

def codec : Codec carrier RuntimeValue where
  encode := nominals.value
  decode? := nominals.ofValue?
  decode_encode := nominals.ofValue?_value

theorem tight : nominals.codec.Tight := nominals.value_of_ofValue?

/-- The order of a nominal's values: their encodings'. -/
def order (rank : ValueRanks) (left right : carrier) : Ordering :=
  RuntimeValue.order rank (nominals.value left) (nominals.value right)

def orders : TypedOrder carrier nominals.codec.encode where
  order := nominals.order
  order_encode := fun _ _ _ => rfl

def erasures : TypedErasure carrier nominals.codec.encode where
  same := fun left right => decide ((nominals.value left).eraseGhosts = (nominals.value right).eraseGhosts)
  same_eq := fun _ _ => rfl

/-- The one value of a proposition that holds, encoded as `.unit`. -/
@[reducible] def proof (held : Prop) [Decidable held] : TypedNominals (PLift held) where
  value := fun _ => .unit
  ofValue? := fun raw => match raw with
    | .unit => if proved : held then some ⟨proved⟩ else none
    | _ => none
  ofValue?_value := fun value => by simp only [dif_pos value.down]
  value_of_ofValue? := fun raw value decoded => by
    cases raw <;> first | rfl | (simp at decoded)
  plain := fun _ => .unit

end TypedNominals


mutual
/-- Whether a literal can have a type: a scalar, or a tuple or a vector of
literal types. A literal's value is read at the ground family, which has
no certificates, so no struct, variant or type parameter is one. -/
def NTy.literal : NTy → Bool
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char => true
  | .tuple elements => elements.literal
  | .vector element => element.literal
  | _ => false

def NRow.literal : NRow → Bool
  | .nil => true
  | .cons τ rest => τ.literal && rest.literal
end

open Classical in
/-- The codec of a certified value: the subtype of a codec's natives by a
predicate, decoded only where the predicate holds of the decoded value
(classically decided), as a data invariant certifies a struct's row
(`NTy.carrier`). -/
noncomputable def Codec.certified (codec : Codec Native RuntimeValue) (holds : Native → Prop) :
    Codec { value : Native // holds value } RuntimeValue where
  encode := fun value => codec.encode value.val
  decode? := fun raw => (codec.decode? raw).bind fun value =>
    if held : holds value then some ⟨value, held⟩ else none
  decode_encode := by
    intro value
    simp [codec.decode_encode, value.property]

/-- The codec of a pair of natives under two codecs, as a runtime tuple: a
mutable reference's current value and its prophecy. -/
def Codec.pair (left : Codec Left RuntimeValue) (right : Codec Right RuntimeValue) :
    Codec (Left × Right) RuntimeValue where
  encode := fun value => .tuple #[left.encode value.1, right.encode value.2]
  decode?
    | .tuple pair => match pair.toList with
      | [l, r] => (left.decode? l).bind fun l => (right.decode? r).map fun r => (l, r)
      | _ => none
    | _ => none
  decode_encode := by
    intro value
    simp [left.decode_encode, right.decode_encode]

/-- A type parameter's values as a lent current holds them: the parameter's
values plain at the family that instantiates it, with the map that forgets
a certified value's certificates and keeps its encoding.  A parameter whose
carrier holds no certificates of its own is its own lent carrier, with the
identity map and the operations of the certified side
(`doc/designs/data-invariants.md`). -/
structure ParamLent where
  carrier : Type
  codec : Codec carrier RuntimeValue
  decEq : DecidableEq carrier
  inhabited : Inhabited carrier
  tight : codec.Tight
  plain : ∀ value, VerexIR.SemanticOperations.Plain (codec.encode value)
  orders : TypedOrder carrier codec.encode
  erasures : TypedErasure carrier codec.encode

/-- A type parameter's lent values with what ties them to its certified
ones: the map that forgets a certified value's certificates, and the data
invariants the parameter's values satisfy. -/
structure ParamPlain (Certified : Type) (certified : Codec Certified RuntimeValue)
    extends ParamLent where
  /-- A certified value as a lent one: its certificates removed. -/
  of : Certified → carrier
  /-- Forgetting the certificates keeps the encoding. -/
  encode_of : ∀ value, codec.encode (of value) = certified.encode value
  /-- The data invariants the parameter's values satisfy, which a certified
  value of the parameter's type satisfies (`holds_of`): what the parameter
  stands for carries them, and a family that states none takes `True`
  (`NTy.deepHolds`). -/
  holds : carrier → Prop
  holds_of : ∀ value, holds (of value)
  /-- A lent value that satisfies the invariants as a certified one. -/
  certify : (value : carrier) → holds value → Certified
  of_certify : ∀ value held, of (certify value held) = value
  certify_of : ∀ value, certify (of value) (holds_of value) = value

/-- What the lent layer of a family reads: a type parameter's lent values
and a function type's values.  A declaration's data invariants are not part
of it, so the lent carriers, codecs and operations of two families that
differ only in `Carriers.own` are the same term, which a verification
condition at a unit's real family needs of the ones stated at its base
family (`doc/designs/data-invariants.md`). -/
structure PlainFamily where
  lent : Nat → ParamLent
  functionCarrier : NRow → Sharing → NRow → Type
  functions : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
    TypedFunctions (functionCarrier parameters shared results)
  /-- The values of a recursive nominal, which carry no certificates of
  their own: their typing certificate is the family's (`TypedNominals`). -/
  nominalCarrier : StructHandle → NRow → Bool → Type
  nominals : (source : StructHandle) → (arguments : NRow) → (productive : Bool) →
    TypedNominals (nominalCarrier source arguments productive)
  nominalInhabitant : ∀ source arguments, arguments.inhabitable = true →
    nominalCarrier source arguments true

/-- The carriers of a generic function's type parameters: per index an
inhabited type, a tight codec whose encodings are loan-free, and decidable
equality.  A generic function is proved over every family
(`doc/designs/denotation.md`, Generics). -/
class Carriers where
  carrier : Nat → Type
  codec : (index : Nat) → Codec (carrier index) RuntimeValue
  decEq : (index : Nat) → DecidableEq (carrier index)
  inhabited : (index : Nat) → Inhabited (carrier index)
  tight : ∀ index, (codec index).Tight
  plain : ∀ index value, VerexIR.SemanticOperations.Plain ((codec index).encode value)
  /-- The values of a function type at its parameter row, which of its
  parameters are shared references, and its result row. -/
  functionCarrier : NRow → Sharing → NRow → Type
  functions : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
    TypedFunctions (functionCarrier parameters shared results)
  /-- The values of a recursive nominal at its type arguments and productivity flag, a value
  wherever the flag says productive and the arguments are inhabitable (`NTy.inhabitable`). -/
  nominalCarrier : StructHandle → NRow → Bool → Type
  nominals : (source : StructHandle) → (arguments : NRow) → (productive : Bool) →
    TypedNominals (nominalCarrier source arguments productive)
  nominalInhabitant : (source : StructHandle) → (arguments : NRow) →
    arguments.inhabitable = true → nominalCarrier source arguments true
  /-- A nominal's value encoded as `.unit` is its inhabitant, which a map's default reads. -/
  nominalInhabitant_unit : ∀ source arguments held (value : nominalCarrier source arguments true),
    (nominals source arguments true).value value = .unit →
      value = nominalInhabitant source arguments held
  /-- The typed values of a type parameter's values, as a specification
  holds them in a map: an integer argument's by their values, any other
  argument's as they are (`NTy.read`). -/
  reads : (index : Nat) → TypedReads (carrier index) (codec index).encode
  /-- The order of a type parameter's values, as the runtime orders their
  encodings (`NTy.order`). -/
  orders : (index : Nat) → TypedOrder (carrier index) (codec index).encode
  /-- The equality of a type parameter's values without their ghost slots
  (`NTy.equal`). -/
  erasures : (index : Nat) → TypedErasure (carrier index) (codec index).encode
  /-- A type parameter's lent values: what the current of a mutable
  reference to the parameter holds, with the operations over it
  (`ParamPlain`).  A family whose parameters carry no certificates of their
  own keeps the certified side, which this default is. -/
  lent : (index : Nat) → ParamPlain (carrier index) (codec index) :=
    fun index =>
      { carrier := carrier index, codec := codec index, of := id, encode_of := fun _ => rfl,
        holds := fun _ => True, holds_of := fun _ => trivial,
        certify := fun value _ => value, of_certify := fun _ _ => rfl,
        certify_of := fun _ => rfl,
        decEq := decEq index, inhabited := inhabited index, tight := tight index,
        plain := plain index, orders := orders index, erasures := erasures index }
  /-- The plain layer's canonical typed reader. Its law retains the total
  codec interpretation while an instantiated family stores the reader of
  the concrete argument directly. -/
  plainRead : (index : Nat) → { read : (lent index).carrier → (reads index).typed //
      ∀ value, read value = (reads index).back ((lent index).codec.encode value) } :=
    fun index => ⟨fun value => (reads index).back ((lent index).codec.encode value), fun _ => rfl⟩
  /-- The data invariant a declaration states of its values, of the
  encoding of a value at the given type arguments; `True` for a
  declaration without one. A struct's or a variant's carrier is the subtype
  of its plain row by it (`NTy.carrier`,
  `doc/designs/data-invariants.md`). -/
  own : StructHandle → NRow → RuntimeValue → Prop := fun _ _ _ => True

/-- The lent layer of a family (`PlainFamily`). -/
@[reducible] def Carriers.toPlain [Θ : Carriers] : PlainFamily where
  lent := fun index => (Carriers.lent index).toParamLent
  functionCarrier := Carriers.functionCarrier
  functions := Carriers.functions
  nominalCarrier := Carriers.nominalCarrier
  nominals := Carriers.nominals
  nominalInhabitant := Carriers.nominalInhabitant

instance [Θ : Carriers] (index : Nat) : Inhabited (Carriers.carrier index) := Θ.inhabited index

/-- A type parameter's lent values are inhabited, as its certified ones are. -/
instance [Θ : Carriers] (index : Nat) : Inhabited ((Carriers.lent index).carrier) :=
  (Θ.lent index).inhabited

/-- A type parameter's lent values are inhabited at any lent layer. -/
instance (F : PlainFamily) (index : Nat) : Inhabited ((F.lent index).carrier) :=
  (F.lent index).inhabited

/-- A type parameter's order compares equal only equal values: its codec's
encodings are, by the runtime order. -/
instance [Θ : Carriers] {index : Nat} {rank : ValueRanks} :
    Std.LawfulEqCmp ((Carriers.orders index).order rank) where
  compare_self := by
    intro value
    rw [← TypedOrder.order_encode]
    exact RuntimeValue.order_self _ _
  eq_of_compare := by
    intro left right same
    rw [← TypedOrder.order_encode] at same
    exact (Carriers.codec index).encode_injective (RuntimeValue.eq_of_order same)

/-- The family of literals, which never have a type parameter's or a
function's type. -/
@[reducible] def Carriers.ground : Carriers where
  carrier := fun _ => Unit
  codec := fun _ => Codec.unit
  decEq := fun _ => inferInstanceAs (DecidableEq Unit)
  inhabited := fun _ => ⟨()⟩
  tight := fun _ raw value decoded => by
    cases raw <;> simp [Codec.unit, decodeUnit?] at decoded ⊢
  plain := fun _ _ => .unit
  functionCarrier := fun _ _ _ => Empty
  functions := fun _ _ _ => TypedFunctions.empty
  nominalCarrier := fun _ arguments productive =>
    PLift (productive = true ∧ arguments.inhabitable = true)
  nominals := fun _ _ _ => TypedNominals.proof _
  nominalInhabitant := fun _ _ held => ⟨rfl, held⟩
  nominalInhabitant_unit := fun _ _ _ _ _ => rfl
  reads := fun _ => TypedReads.unit
  orders := fun _ => TypedOrder.unit
  erasures := fun _ => TypedErasure.unit
  plainRead := fun _ => ⟨id, fun value => by cases value; rfl⟩

/-- A function value of the runtime family: a runtime closure the unit types
at the function type. A type of its own, so that no verification condition
unfolds a function value to its closure. -/
structure RuntimeFunction (unit : Validation.ValidatedUnit) (parameters : NRow)
    (shared : Sharing) (results : NRow) where
  closure : ClosureValue
  typed : NTy.admits unit (.function parameters shared results) closure.encode = true

/-- The runtime family's function values: the closures the unit types. -/
@[reducible] def RuntimeFunction.functions (unit : Validation.ValidatedUnit) (parameters : NRow)
    (shared : Sharing) (results : NRow) :
    TypedFunctions (RuntimeFunction unit parameters shared results) where
  closure := RuntimeFunction.closure
  ofClosure? := fun raw =>
    if typed : NTy.admits unit (.function parameters shared results) raw.encode = true then
      some ⟨raw, typed⟩
    else none
  ofClosure?_closure := fun value => by simp only [dif_pos value.typed]
  closure_of_ofClosure? := fun raw value decoded => by
    split at decoded
    · cases decoded; rfl
    · cases decoded

/-- A unit's function values: a carrier per function type and its interface
(`TypedFunctions`). One instance per unit, declared where the unit is
registered: the runtime family and storage hold these values. -/
class FunctionCarriers (unit : Validation.ValidatedUnit) where
  carrier : NRow → Sharing → NRow → Type
  functions : (parameters : NRow) → (shared : Sharing) → (results : NRow) →
    TypedFunctions (carrier parameters shared results)
  /-- Every value's closure is one the unit types at its function type. -/
  admits : ∀ parameters shared results (value : carrier parameters shared results),
    NTy.admits unit (.function parameters shared results)
      ((functions parameters shared results).closure value).encode = true
  /-- Every closure the unit types at a function type decodes. -/
  decodes : ∀ parameters shared results (closure : ClosureValue),
    NTy.admits unit (.function parameters shared results) closure.encode = true →
      ((functions parameters shared results).ofClosure? closure).isSome = true
  /-- The data invariants of the unit's declarations, of a value's encoding
  at the given type arguments (`Carriers.own`); `True` for a declaration
  without one. -/
  own : StructHandle → NRow → RuntimeValue → Prop := fun _ _ _ => True

/-- The library's function values, the closures the unit types
(`RuntimeFunction`): the instance of a unit without generated function
values. A definition, not an instance: a unit's instance is declared with
the unit. -/
@[reducible] def FunctionCarriers.runtime (unit : Validation.ValidatedUnit) : FunctionCarriers unit where
  carrier := RuntimeFunction unit
  functions := RuntimeFunction.functions unit
  admits := fun _ _ _ value => value.typed
  decodes := fun _ _ _ _ typed => by
    simp only [RuntimeFunction.functions, dif_pos typed, Option.isSome_some]

/-- A value of a recursive nominal at the runtime family: a runtime value the unit types at
the nominal. A type of its own, so that no verification condition unfolds it to its encoding. -/
structure RuntimeNominal (unit : Validation.ValidatedUnit) (source : StructHandle)
    (arguments : NRow) (productive : Bool) where
  value : RuntimeValue
  typed : NTy.admits unit (.nominal source arguments productive) value = true

/-- What a recursive nominal admits holds no loan. -/
theorem NTy.plain_of_admits_nominal {unit : Validation.ValidatedUnit} {source : StructHandle}
    {arguments : NRow} {productive : Bool} {value : RuntimeValue}
    (admitted : NTy.admits unit (.nominal source arguments productive) value = true) :
    Plain value := by
  have point : productive = true ∧ value = witnessOrUnit unit source arguments →
      Plain value := fun held => held.2 ▸ witnessOrUnit_plain unit source arguments
  cases value <;> first
    | (simp only [NTy.admits, Bool.and_eq_true, beq_iff_eq] at admitted; exact point admitted)
    | (rw [NTy.admits] at admitted
       simp only [Bool.or_eq_true, Bool.and_eq_true, beq_iff_eq] at admitted
       rcases admitted with admitted | admitted
       · exact (plain?_iff _).mp admitted.1.1
       · exact point admitted)

/-- The runtime family's values of a recursive nominal: the values the unit types. -/
@[reducible] def RuntimeNominal.nominals (unit : Validation.ValidatedUnit) (source : StructHandle)
    (arguments : NRow) (productive : Bool) :
    TypedNominals (RuntimeNominal unit source arguments productive) where
  value := RuntimeNominal.value
  ofValue? := fun raw =>
    if typed : NTy.admits unit (.nominal source arguments productive) raw = true then
      some ⟨raw, typed⟩
    else none
  ofValue?_value := fun value => by simp only [dif_pos value.typed]
  value_of_ofValue? := fun raw value decoded => by
    split at decoded
    · cases decoded; rfl
    · cases decoded
  plain := fun value => NTy.plain_of_admits_nominal value.typed

/-- A nominal flagged productive admits its witness. -/
theorem NTy.admits_witnessOrUnit (unit : Validation.ValidatedUnit) (source : StructHandle)
    (arguments : NRow) :
    NTy.admits unit (.nominal source arguments true) (witnessOrUnit unit source arguments) = true := by
  have point : ∀ value, value = witnessOrUnit unit source arguments →
      NTy.admits unit (.nominal source arguments true) value = true := by
    intro value same
    cases value <;> rw [NTy.admits] <;> simp [← same]
  exact point _ rfl

/-- The only `.unit` a nominal admits is its witness. -/
theorem NTy.witnessOrUnit_of_admits_unit {unit : Validation.ValidatedUnit} {source : StructHandle}
    {arguments : NRow} (admitted : NTy.admits unit (.nominal source arguments true) .unit = true) :
    witnessOrUnit unit source arguments = .unit := by
  simp only [NTy.admits, Bool.true_and, beq_iff_eq] at admitted
  exact admitted.symm
open Classical in
/-- The family of the public theorem: every type parameter carried as a
loan-free runtime value, and the unit's function values. -/
@[reducible] noncomputable def Carriers.runtime (unit : Validation.ValidatedUnit)
    [FunctionCarriers unit] : Carriers where
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
  functionCarrier := FunctionCarriers.carrier unit
  functions := FunctionCarriers.functions
  nominalCarrier := RuntimeNominal unit
  nominals := RuntimeNominal.nominals unit
  nominalInhabitant := fun source arguments _ =>
    ⟨witnessOrUnit unit source arguments, NTy.admits_witnessOrUnit unit source arguments⟩
  nominalInhabitant_unit := fun source arguments _ value encoded => by
    obtain ⟨raw, typed⟩ := value
    cases (show raw = .unit from encoded)
    exact congrArg (fun held => RuntimeNominal.mk (unit := unit) (source := source)
      (arguments := arguments) (productive := true) held.1 held.2)
      (Subtype.ext (NTy.witnessOrUnit_of_admits_unit typed).symm :
        (⟨.unit, typed⟩ : { value // NTy.admits unit (.nominal source arguments true) value = true }) =
          ⟨witnessOrUnit unit source arguments, NTy.admits_witnessOrUnit unit source arguments⟩)
  reads := fun _ => PlainValue.reads
  orders := fun _ => PlainValue.orders
  erasures := fun _ => PlainValue.erasures
  plainRead := fun _ => ⟨id, fun value => (PlainValue.ofRuntime_val value).symm⟩
  own := FunctionCarriers.own unit

/-- The data invariants of the runtime family: the unit's, as its function
values carry them. -/
@[simp, vir_denote_norm] theorem Carriers.own_runtime (unit : Validation.ValidatedUnit)
    [functions : FunctionCarriers unit] :
    (Carriers.runtime unit).own = FunctionCarriers.own unit := rfl

section Carriers

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

/-- Decode one entry of a map: a tuple of a key and a value. -/
def Codec.decodeEntry? (key : Codec Key RuntimeValue) (value : Codec Value RuntimeValue) :
    RuntimeValue → Option (Key × Value)
  | .tuple pair => match pair.toList with
    | [k, v] => (key.decode? k).bind fun k => (value.decode? v).map fun v => (k, v)
    | _ => none
  | _ => none

@[simp] theorem Codec.decodeEntry?_encode (key : Codec Key RuntimeValue)
    (value : Codec Value RuntimeValue) (k : Key) (v : Value) :
    Codec.decodeEntry? key value (.tuple #[key.encode k, value.encode v]) = some (k, v) := by
  simp [Codec.decodeEntry?, key.decode_encode, value.decode_encode]

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

/-- A ghost field's slot: the value of its type in a ghost slot
(`doc/designs/ghost-fields.md`). -/
def Codec.ghost (codec : Codec Native RuntimeValue) : Codec Native RuntimeValue where
  encode := fun value => .ghost (codec.encode value)
  decode?
    | .ghost contents => codec.decode? contents
    | _ => none
  decode_encode := by
    intro value
    simp [codec.decode_encode]

@[simp] theorem Codec.ghost_encode (codec : Codec Native RuntimeValue)
    (value : Native) : (Codec.ghost codec).encode value = .ghost (codec.encode value) := rfl

/-- A pair of slice readings decodes only when both have the same extent. -/
def Codec.sliceRef (codec : Codec Native RuntimeValue) : Codec (SpecSliceRef Native) RuntimeValue where
  encode := fun value => .tuple #[(Codec.boundedVector codec).encode value.currentVector,
    (Codec.boundedVector codec).encode value.finalVector]
  decode? := fun raw => do
    let (current, final) ← (Codec.prophecyPair (Codec.boundedVector codec)).decode? raw
    SpecSliceRef.ofVectors? current final
  decode_encode := by
    intro value
    change ((Codec.prophecyPair (Codec.boundedVector codec)).decode?
      ((Codec.prophecyPair (Codec.boundedVector codec)).encode
        (value.currentVector, value.finalVector)) >>= _) = _
    rw [Codec.decode_encode]
    exact SpecSliceRef.ofVectors?_readings value

@[simp] theorem Codec.ghost_decode? (codec : Codec Native RuntimeValue)
    (contents : RuntimeValue) : (Codec.ghost codec).decode? (.ghost contents) = codec.decode? contents :=
  rfl

/-- The entries of a map: a runtime vector of key–value tuples, as many as a
Move vector holds (`SpecEntries`). -/
def Codec.entries (key : Codec Key RuntimeValue) (value : Codec Value RuntimeValue) :
    Codec (SpecEntries (Key × Value)) RuntimeValue where
  encode := fun entries =>
    .vector (entries.entries.map fun entry =>
      .tuple #[key.encode entry.1, value.encode entry.2]).toArray
  decode?
    | .vector entries => do
        let decoded ← entries.toList.mapM (Codec.decodeEntry? key value)
        if bound : decoded.length < 2 ^ 64 then some ⟨decoded, bound⟩ else none
    | _ => none
  decode_encode := by
    intro ⟨entries, bounded⟩
    have decoded : ∀ entries : List (Key × Value),
        List.mapM (Codec.decodeEntry? key value)
          (List.toArray (entries.map fun entry =>
            RuntimeValue.tuple #[key.encode entry.1, value.encode entry.2])).toList =
          some entries := by
      intro entries
      rw [List.toList_toArray, List.mapM_map]
      induction entries with
      | nil => rfl
      | cons entry rest ih =>
          rw [List.mapM_cons, ih]
          simp [Codec.decodeEntry?, key.decode_encode, value.decode_encode]
    simp [decoded, bounded]


mutual
/-- The plain carrier of a type: a struct is its row, a variant its row under
its name, with no certificate anywhere — the shape a data invariant is
stated over, and the current of a mutable reference
(`doc/designs/data-invariants.md`). -/
@[reducible] def NTy.plainOf (F : PlainFamily) : NTy → Type
  | .unit => Unit
  | .bool => Bool
  | .int width signed => SpecInt (.bits width) signed
  | .num => SpecInt .unbounded true
  | .address => String
  | .signer => String
  | .string => String
  | .bytes => Array UInt8
  | .char => SpecChar
  | .tuple elements => HListPlainOf F elements
  | .struct _ _ fields => HListPlainOf F fields
  | .enum _ _ names rows _ => variantPlainOf F names rows
  | .slice element => SpecVector (NTy.plainOf F element)
  | .vector element => SpecVector (NTy.plainOf F element)
  | .array element length => SpecArray (NTy.plainOf F element) length
  | .table key value => SpecEntries (NTy.plainOf F key × NTy.plainOf F value)
  | .ghost slot => NTy.plainOf F slot
  | .sliceRef element => SpecSliceRef (NTy.plainOf F element)
  | .ref referent => NTy.plainOf F referent × NTy.plainOf F referent
  | .param index => (F.lent index).carrier
  | .function parameters shared results => F.functionCarrier parameters shared results
  | .nominal source arguments productive => F.nominalCarrier source arguments productive
  | .observed referent => NTy.plainOf F referent

/-- A plain row of values, one per type. -/
def HListPlainOf (F : PlainFamily) : NRow → Type
  | .nil => Unit
  | .cons τ rest => NTy.plainOf F τ × HListPlainOf F rest

/-- A plain value of one of the named variants: the nested sum of their
plain rows.

A function over a variant value reads it with `Sum.elim`, never with a
`match`: a match on a variant value compiles to a matcher that carries the
family, and a matcher does not reduce at an abstract value, so the same
declaration's value at two families that agree — an induced family and the
outer family at the substituted type — gets stuck at two different matchers
and the two carriers stop being definitionally equal.  A struct's row has
no such match, which is why a struct identifies and a variant would not.
The vacuous cases, where no variant is left and the value's type is empty,
state a constant for the same reason. -/
def variantPlainOf (F : PlainFamily) : List String → NRows → Type
  | _, .nil => Empty
  | [], .cons _ _ => Empty
  | _ :: names, .cons fields rest => HListPlainOf F fields ⊕ variantPlainOf F names rest
end

/-- The row codecs of an enum's variants. -/
@[reducible] def RowCodecsPlainOf (F : PlainFamily) : NRows → Type
  | .nil => Unit
  | .cons fields rest => Codec (HListPlainOf F fields) (List RuntimeValue) × RowCodecsPlainOf F rest

/-- Encode a variant value under its declaration's handle and name. -/
def variantEncodePlainOf (F : PlainFamily) (source : StructHandle) : (names : List String) → (rows : NRows) →
    RowCodecsPlainOf F rows → variantPlainOf F names rows → RuntimeValue
  -- A declaration with no variant left has no value: the case is vacuous,
  -- and it states a constant rather than eliminating the value, so that the
  -- encoder does not match on it.  A match on the value compiles to a
  -- matcher carrying the family, which would make an encoding differ
  -- between two families that encode alike
  -- (`doc/designs/data-invariants.md`).
  | _, .nil, _, _ => .unit
  | [], .cons _ _, _, _ => .unit
  | name :: names, .cons _ rest, (codec, codecs), value =>
      Sum.elim (fun fields => (Codec.nominalRow source (some name) codec).encode fields)
        (fun later => variantEncodePlainOf F source names rest codecs later) value

/-- Decode a variant value by its name. -/
def variantDecodePlainOf? (F : PlainFamily) (source : StructHandle) : (names : List String) → (rows : NRows) →
    RowCodecsPlainOf F rows → RuntimeValue → Option (variantPlainOf F names rows)
  | _, .nil, _, _ => none
  | [], .cons _ _, _, _ => none
  | name :: names, .cons _ rest, (codec, codecs), runtime =>
      match runtime with
      | .nominal actualSource (some actualVariant) _ =>
          if actualSource = source ∧ actualVariant = name then
            ((Codec.nominalRow source (some name) codec).decode? runtime).map .inl
          else (variantDecodePlainOf? F source names rest codecs runtime).map .inr
      | _ => none

/-- No enum value is decoded from the unit. -/
theorem variantDecodePlainOf?_unit (F : PlainFamily) (source : StructHandle) : (names : List String) → (rows : NRows) →
    (codecs : RowCodecsPlainOf F rows) → variantDecodePlainOf? F source names rows codecs .unit = none
  | [], .nil, _ => by simp [variantDecodePlainOf?]
  | _ :: _, .nil, _ => by simp [variantDecodePlainOf?]
  | [], .cons _ _, _ => by simp [variantDecodePlainOf?]
  | _ :: _, .cons _ _, _ => by simp [variantDecodePlainOf?]

theorem variantEncodePlainOf_shape (F : PlainFamily) (source : StructHandle) : (names : List String) → (rows : NRows) →
    (codecs : RowCodecsPlainOf F rows) → (value : variantPlainOf F names rows) →
    ∃ name fields, variantEncodePlainOf F source names rows codecs value =
      .nominal source (some name) fields ∧ name ∈ names
  | _, .nil, _, value => nomatch value
  | [], .cons _ _, _, value => nomatch value
  | name :: names, .cons _ rest, (codec, codecs), .inl fields =>
      ⟨name, _, rfl, List.mem_cons_self⟩
  | _ :: names, .cons _ rest, (_, codecs), .inr later => by
      obtain ⟨name, fields, encoded, member⟩ :=
        variantEncodePlainOf_shape F source names rest codecs later
      exact ⟨name, fields, encoded, List.mem_cons_of_mem _ member⟩

theorem variantDecodePlainOf?_encode (F : PlainFamily) (source : StructHandle) : (names : List String) → (rows : NRows) →
    (codecs : RowCodecsPlainOf F rows) → names.Nodup → (value : variantPlainOf F names rows) →
    variantDecodePlainOf? F source names rows codecs (variantEncodePlainOf F source names rows codecs value) =
      some value
  | _, .nil, _, _, value => nomatch value
  | [], .cons _ _, _, _, value => nomatch value
  | name :: names, .cons _ rest, (codec, codecs), _, .inl fields => by
      simp [variantEncodePlainOf, Sum.elim_inl, Sum.elim_inr, variantDecodePlainOf?, Codec.nominalRow, codec.decode_encode]
  | name :: names, .cons _ rest, (codec, codecs), nodup, .inr later => by
      obtain ⟨actual, fields, encoded, member⟩ :=
        variantEncodePlainOf_shape F source names rest codecs later
      have distinct : actual ≠ name := fun equal =>
        (List.nodup_cons.mp nodup).1 (equal ▸ member)
      simp only [variantEncodePlainOf, Sum.elim_inr]
      rw [encoded]
      simp only [variantDecodePlainOf?, distinct, and_false, ↓reduceIte]
      rw [← encoded, variantDecodePlainOf?_encode F source names rest codecs
        (List.nodup_cons.mp nodup).2 later]
      rfl

/-- The codec of an enum's values. -/
def variantCodecPlainOf (F : PlainFamily) (source : StructHandle) (names : List String) (rows : NRows)
    (codecs : RowCodecsPlainOf F rows) (distinct : names.Nodup) :
    Codec (variantPlainOf F names rows) RuntimeValue where
  encode := variantEncodePlainOf F source names rows codecs
  decode? := variantDecodePlainOf? F source names rows codecs
  decode_encode := variantDecodePlainOf?_encode F source names rows codecs distinct

mutual
/-- The codec of a plain value of one type. -/
def NTy.plainCodecOf (F : PlainFamily) : (τ : NTy) → Codec (NTy.plainOf F τ) RuntimeValue
  | .unit => Codec.unit
  | .bool => Codec.bool
  | .int width signed => Codec.specInt (.bits width) signed
  | .num => Codec.specInt .unbounded true
  | .address => Codec.address
  | .signer => Codec.signer
  | .string => Codec.string
  | .bytes => Codec.bytes
  | .char => Codec.character
  | .tuple elements => Codec.tuple (rowCodecPlainOf F elements)
  | .struct source _ fields => Codec.nominalRow source none (rowCodecPlainOf F fields)
  | .enum source _ names rows distinct => variantCodecPlainOf F source names rows (rowCodecsPlainOf F rows) distinct
  | .slice element => Codec.boundedVector (NTy.plainCodecOf F element)
  | .vector element => Codec.boundedVector (NTy.plainCodecOf F element)
  | .array element length => Codec.array (NTy.plainCodecOf F element) length
  | .table key value => Codec.entries (NTy.plainCodecOf F key) (NTy.plainCodecOf F value)
  | .ghost slot => Codec.ghost (NTy.plainCodecOf F slot)
  | .sliceRef element => Codec.sliceRef (NTy.plainCodecOf F element)
  | .ref referent => Codec.prophecyPair (NTy.plainCodecOf F referent)
  | .param index => (F.lent index).codec
  | .function parameters shared results => (F.functions parameters shared results).codec
  | .nominal source arguments productive => (F.nominals source arguments productive).codec
  | .observed referent => NTy.plainCodecOf F referent
termination_by structural τ => τ

/-- The codec of a plain row of values. -/
def rowCodecPlainOf (F : PlainFamily) : (row : NRow) → Codec (HListPlainOf F row) (List RuntimeValue)
  | .nil => Codec.tupleNil
  | .cons τ rest => Codec.tupleCons (NTy.plainCodecOf F τ) (rowCodecPlainOf F rest)
termination_by structural row => row

def rowCodecsPlainOf (F : PlainFamily) : (rows : NRows) → RowCodecsPlainOf F rows
  | .nil => ()
  | .cons fields rest => (rowCodecPlainOf F fields, rowCodecsPlainOf F rest)
termination_by structural rows => rows
end


variable [Carriers]

/-- The plain carrier of a type at a family's lent layer. -/
@[reducible] def NTy.plain (τ : NTy) : Type := NTy.plainOf Carriers.toPlain τ

/-- A plain row of values at a family's lent layer. -/
@[reducible] def HListPlain (row : NRow) : Type := HListPlainOf Carriers.toPlain row

/-- A plain variant value at a family's lent layer. -/
@[reducible] def variantPlain (names : List String) (rows : NRows) : Type :=
  variantPlainOf Carriers.toPlain names rows

/-- The row codecs of an enum's variants at a family's lent layer. -/
@[reducible] def RowCodecsPlain (rows : NRows) : Type := RowCodecsPlainOf Carriers.toPlain rows

/-- Encode a variant value under its declaration's handle and name. -/
@[reducible] def variantEncodePlain (source : StructHandle) (names : List String) (rows : NRows)
    (codecs : RowCodecsPlain rows) (value : variantPlain names rows) : RuntimeValue :=
  variantEncodePlainOf Carriers.toPlain source names rows codecs value

/-- Decode a variant value by its name. -/
@[reducible] def variantDecodePlain? (source : StructHandle) (names : List String) (rows : NRows)
    (codecs : RowCodecsPlain rows) (runtime : RuntimeValue) : Option (variantPlain names rows) :=
  variantDecodePlainOf? Carriers.toPlain source names rows codecs runtime

/-- No enum value is decoded from the unit. -/
theorem variantDecodePlain?_unit (source : StructHandle) (names : List String) (rows : NRows)
    (codecs : RowCodecsPlain rows) : variantDecodePlain? source names rows codecs .unit = none :=
  variantDecodePlainOf?_unit Carriers.toPlain source names rows codecs

theorem variantEncodePlain_shape (source : StructHandle) (names : List String) (rows : NRows)
    (codecs : RowCodecsPlain rows) (value : variantPlain names rows) :
    ∃ name fields, variantEncodePlain source names rows codecs value =
      .nominal source (some name) fields ∧ name ∈ names :=
  variantEncodePlainOf_shape Carriers.toPlain source names rows codecs value

theorem variantDecodePlain?_encode (source : StructHandle) (names : List String) (rows : NRows)
    (codecs : RowCodecsPlain rows) (distinct : names.Nodup) (value : variantPlain names rows) :
    variantDecodePlain? source names rows codecs
        (variantEncodePlain source names rows codecs value) = some value :=
  variantDecodePlainOf?_encode Carriers.toPlain source names rows codecs distinct value

/-- The codec of an enum's values. -/
@[reducible] def variantCodecPlain (source : StructHandle) (names : List String) (rows : NRows)
    (codecs : RowCodecsPlain rows) (distinct : names.Nodup) :
    Codec (variantPlain names rows) RuntimeValue :=
  variantCodecPlainOf Carriers.toPlain source names rows codecs distinct

/-- The codec of a plain value of one type at a family's lent layer. -/
@[reducible] def NTy.plainCodec (τ : NTy) : Codec τ.plain RuntimeValue :=
  NTy.plainCodecOf Carriers.toPlain τ

/-- The runtime representation of a plain value. -/
def NTy.plainEncode (τ : NTy) (value : τ.plain) : RuntimeValue := τ.plainCodec.encode value

/-- The codec of a plain row of values at a family's lent layer. -/
@[reducible] def rowCodecPlain (row : NRow) : Codec (HListPlain row) (List RuntimeValue) :=
  rowCodecPlainOf Carriers.toPlain row

/-- The codecs of an enum's variant rows at a family's lent layer. -/
@[reducible] def rowCodecsPlain (rows : NRows) : RowCodecsPlain rows :=
  rowCodecsPlainOf Carriers.toPlain rows

/-- The lent layer reads no data invariant: two families that differ only in
what their declarations state (`Carriers.own`) give a type the same plain
codec, which a verification condition at a unit's real family needs of the
ones stated at its base family. -/
example (base : Carriers) (τ : NTy) :
    @NTy.plainCodec { base with own := fun _ _ _ => True } τ =
      @NTy.plainCodec { base with own := fun _ _ _ => False } τ := rfl

mutual
/-- The data invariants a plain value satisfies, at every depth: at a
struct or a variant, its declaration's (`Carriers.own`) of its encoding,
and its fields'; through tuples, vectors, tables and ghost slots; of a
reference, its prophecy's, the current being open while lent
(`doc/designs/data-invariants.md`); nothing
of a scalar, a type parameter's value or a function value, whose carriers
certify themselves. A certified carrier is the subtype of the plain one
by it (`NTy.carrier`). -/
def NTy.deepHolds : (τ : NTy) → τ.plain → Prop
  | .tuple elements, values => HListPlain.deepHolds elements values
  | .struct source arguments fields, row =>
      Carriers.own source arguments (NTy.plainEncode (.struct source arguments fields) row) ∧
        HListPlain.deepHolds fields row
  | .enum source arguments names rows distinct, value =>
      Carriers.own source arguments
          (NTy.plainEncode (.enum source arguments names rows distinct) value) ∧
        variantPlain.deepHolds names rows value
  | .vector element, vector => ∀ value ∈ vector.values, NTy.deepHolds element value
  | .slice element, vector => ∀ value ∈ vector.values, NTy.deepHolds element value
  | .array element _, array => ∀ value ∈ array.vector.values, NTy.deepHolds element value
  | .table key value, entries =>
      ∀ entry ∈ entries.entries, NTy.deepHolds key entry.1 ∧ NTy.deepHolds value entry.2
  | .ghost slot, value => NTy.deepHolds slot value
  | .ref referent, value => NTy.deepHolds referent value.2
  -- A slice reference's carrier certifies both of its readings, the extent
  -- being shared: unlike `.ref`, whose current is open while lent, neither
  -- reading of a borrowed extent is plainer than the other.
  | .sliceRef element, value =>
      (∀ current ∈ value.currentVector.values, NTy.deepHolds element current) ∧
        (∀ final ∈ value.finalVector.values, NTy.deepHolds element final)
  | .observed referent, value => NTy.deepHolds referent value
  | .param index, value => (Carriers.lent index).holds value
  | _, _ => True
termination_by structural τ _ => τ

def HListPlain.deepHolds : (row : NRow) → HListPlain row → Prop
  | .nil, _ => True
  | .cons τ rest, values => NTy.deepHolds τ values.1 ∧ HListPlain.deepHolds rest values.2
termination_by structural row _ => row

def variantPlain.deepHolds : (names : List String) → (rows : NRows) → variantPlain names rows → Prop
  -- A declaration with no variant left has no value: the case is vacuous,
  -- and it states a constant rather than eliminating the value, so that the
  -- function does not match on it.  A match on the value compiles to a
  -- matcher carrying the family, which would make a variant's certificate
  -- differ between two families that state the same invariants
  -- (`doc/designs/data-invariants.md`, the carrier of an induced
  -- family is the outer family's at the substituted type).
  | _, .nil, _ => True
  | [], .cons _ _, _ => True
  | _ :: names, .cons fields rest, value =>
      Sum.elim (HListPlain.deepHolds fields) (variantPlain.deepHolds names rest) value
termination_by structural names rows _ => rows
end

mutual
/-- The Lean type a native value of one type inhabits: a struct or a variant
is its plain value certified by the data invariants at every depth
(`NTy.deepHolds`), a mutable reference its plain current and its certified
prophecy, every other type structural over the certified carriers
(`doc/designs/data-invariants.md`).

The certified cases name the plain layer at the family's lent layer
spelled out (`NTy.plainOf Carriers.toPlain`, `HListPlainOf
Carriers.toPlain`, `variantPlainOf Carriers.toPlain`), not through the
wrapper abbreviations: a subtype stores its carrier type, so every `.val`
of a certified value carries that spelling into the implicit arguments of
the projections read off it.  One type read at two spellings is two terms
to a congruence closure, which no equation of the leaf relates. -/
@[reducible] def NTy.carrier : (τ : NTy) → Type
  | .unit => Unit
  | .bool => Bool
  | .int width signed => SpecInt (.bits width) signed
  | .num => SpecInt .unbounded true
  | .address => String
  | .signer => String
  | .string => String
  | .bytes => Array UInt8
  | .char => SpecChar
  | .tuple elements => HList elements
  | .struct source arguments fields =>
      { row : HListPlainOf Carriers.toPlain fields //
        NTy.deepHolds (.struct source arguments fields) row }
  | .enum source arguments names rows distinct =>
      { value : variantPlainOf Carriers.toPlain names rows //
        NTy.deepHolds (.enum source arguments names rows distinct) value }
  | .slice element => SpecVector element.carrier
  | .vector element => SpecVector element.carrier
  | .array element length => SpecArray element.carrier length
  | .table key value => SpecEntries (key.carrier × value.carrier)
  | .ghost slot => slot.carrier
  | .sliceRef element => SpecSliceRef element.carrier
  | .ref referent => referent.plain × referent.carrier
  | .param index => Carriers.carrier index
  | .function parameters shared results => Carriers.functionCarrier parameters shared results
  | .nominal source arguments productive => Carriers.nominalCarrier source arguments productive
  | .observed referent => referent.carrier
termination_by structural τ => τ

/-- A row of values, one per type.  Not reducible: a row type is a key of
the lemmas that rewrite values of it. -/
def HList : (row : NRow) → Type
  | .nil => Unit
  | .cons τ rest => τ.carrier × HList rest
termination_by structural row => row

/-- A value of one of the named variants: the nested sum of their rows.
Rows beyond the names, or names beyond the rows, have no values. -/
def variantCarrier : (names : List String) → (rows : NRows) → Type
  | _, .nil => Empty
  | [], .cons _ _ => Empty
  | _ :: names, .cons fields rest => HList fields ⊕ variantCarrier names rest
termination_by structural names rows => rows
end

/-- The empty row's one value, at its row type: a term a specification
builds a row's end from, type-correct without unfolding `HList`, which the
unit `()` is not. -/
abbrev HList.nil : HList .nil := ()

/-- A row's value from its head and the rest, at the row type: as
`HList.nil`, a term type-correct without unfolding `HList`, which the pair
is not. -/
abbrev HList.cons {τ : NTy} {rest : NRow} (head : τ.carrier) (tail : HList rest) :
    HList (.cons τ rest) := (head, tail)

/-- The carrier a literal's value inhabits: the ground family's, where a
type parameter has no values. -/
@[reducible] def NTy.groundCarrier (τ : NTy) : Type := @NTy.carrier Carriers.ground τ

mutual
/-- A literal's value at the current family: of a literal type, which has
no certificates to transport. -/
def NTy.ofGround : (τ : NTy) → τ.literal = true → τ.groundCarrier → τ.carrier
  | .unit, _, value => value
  | .bool, _, value => value
  | .int _ _, _, value => value
  | .num, _, value => value
  | .address, _, value => value
  | .signer, _, value => value
  | .string, _, value => value
  | .bytes, _, value => value
  | .char, _, value => value
  | .tuple elements, literal, values => HList.ofGround elements literal values
  | .vector element, literal, vector =>
      ⟨vector.values.map (element.ofGround literal), by simpa using vector.bounded⟩
  | .struct _ _ _, literal, _ => nomatch literal
  | .enum _ _ _ _ _, literal, _ => nomatch literal
  | .slice _, literal, _ => nomatch literal
  | .array _ _, literal, _ => nomatch literal
  | .table _ _, literal, _ => nomatch literal
  | .ghost _, literal, _ => nomatch literal
  | .ref _, literal, _ => nomatch literal
  | .sliceRef _, literal, _ => nomatch literal
  | .param _, literal, _ => nomatch literal
  | .function _ _ _, literal, _ => nomatch literal
  | .nominal _ _ _, literal, _ => nomatch literal
  | .observed _, literal, _ => nomatch literal

def HList.ofGround : (row : NRow) → row.literal = true → @HList Carriers.ground row → HList row
  | .nil, _, _ => ()
  | .cons τ rest, literal, values =>
      (τ.ofGround (Bool.and_eq_true_iff.mp literal).1 values.1,
        HList.ofGround rest (Bool.and_eq_true_iff.mp literal).2 values.2)

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

/-- The injections of a variant value, declared at the folded carrier type
so that an `Option` of decoded variants keeps that type; applied, they are
the sum constructors. -/
def variantPlain.first {name : String} {names : List String} {fields : NRow} {rest : NRows}
    (value : HListPlain fields) : variantPlain (name :: names) (.cons fields rest) := Sum.inl value
def variantPlain.later {name : String} {names : List String} {fields : NRow} {rest : NRows}
    (value : variantPlain names rest) : variantPlain (name :: names) (.cons fields rest) :=
  Sum.inr value
@[simp] theorem variantPlain.first_eq (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (value : HListPlain fields) :
    (variantPlain.first (name := name) (names := names) (fields := fields) (rest := rest) value) =
      (Sum.inl value : HListPlain fields ⊕ variantPlain names rest) := rfl
@[simp] theorem variantPlain.later_eq (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (value : variantPlain names rest) :
    (variantPlain.later (name := name) (names := names) (fields := fields) (rest := rest) value) =
      (Sum.inr value : HListPlain fields ⊕ variantPlain names rest) := rfl

/-- A bounded vector decides equality by its elements. -/
instance instDecidableEqSpecEntries {α : Type} [DecidableEq α] : DecidableEq (SpecEntries α) :=
  fun a b =>
    if h : a.entries = b.entries then isTrue (SpecEntries.ext h)
    else isFalse fun equal => h (equal ▸ rfl)

instance instDecidableEqSpecVector {α : Type} [DecidableEq α] : DecidableEq (SpecVector α) := fun a b =>
  if h : a.values = b.values then isTrue (SpecVector.ext h)
  else isFalse fun equal => h (congrArg SpecVector.values equal)

/-- A certified integer decides equality by its value. -/
instance {width : IntWidth} {signed : Bool} : DecidableEq (SpecInt width signed) := fun a b =>
  if h : a.val = b.val then isTrue (SpecInt.ext h)
  else isFalse fun equal => h (congrArg SpecInt.val equal)

omit [Carriers] in
mutual
/-- Decidable equality of plain values. -/
def NTy.plainDecEqOf (F : PlainFamily) : (τ : NTy) → DecidableEq (NTy.plainOf F τ)
  | .unit => inferInstanceAs (DecidableEq Unit)
  | .bool => inferInstanceAs (DecidableEq Bool)
  | .int width signed => inferInstanceAs (DecidableEq (SpecInt (.bits width) signed))
  | .num => inferInstanceAs (DecidableEq (SpecInt .unbounded true))
  | .address => inferInstanceAs (DecidableEq String)
  | .signer => inferInstanceAs (DecidableEq String)
  | .string => inferInstanceAs (DecidableEq String)
  | .bytes => inferInstanceAs (DecidableEq (Array UInt8))
  | .char => inferInstanceAs (DecidableEq SpecChar)
  | .tuple elements => HListPlainOf.decEq F elements
  | .struct _ _ fields => HListPlainOf.decEq F fields
  | .enum _ _ names rows _ => variantPlainOf.decEq F names rows
  | .slice element => @instDecidableEqSpecVector _ (NTy.plainDecEqOf F element)
  | .vector element => @instDecidableEqSpecVector _ (NTy.plainDecEqOf F element)
  | .array element _ => fun left right =>
      have := @instDecidableEqSpecVector _ (NTy.plainDecEqOf F element)
      decidable_of_iff (left.vector = right.vector) ⟨SpecArray.ext, fun h => h ▸ rfl⟩
  | .table key value => @instDecidableEqSpecEntries _ (@instDecidableEqProd _ _ (NTy.plainDecEqOf F key) (NTy.plainDecEqOf F value))
  | .ghost slot => (NTy.plainDecEqOf F slot)
  | .sliceRef element => fun left right =>
      letI := NTy.plainDecEqOf F element
      decidable_of_iff (left.currentVector = right.currentVector ∧ left.finalVector = right.finalVector)
        ⟨fun h => SpecSliceRef.ext h.1 h.2, fun h => h ▸ ⟨rfl, rfl⟩⟩
  | .ref referent => @instDecidableEqProd _ _ (NTy.plainDecEqOf F referent) (NTy.plainDecEqOf F referent)
  | .param index => (F.lent index).decEq
  | .function parameters shared results => (F.functions parameters shared results).decEq
  | .nominal source arguments productive => (F.nominals source arguments productive).decEq
  | .observed referent => NTy.plainDecEqOf F referent

def HListPlainOf.decEq (F : PlainFamily) : (row : NRow) → DecidableEq (HListPlainOf F row)

  | .nil => inferInstanceAs (DecidableEq Unit)
  | .cons τ rest => @instDecidableEqProd _ _ (NTy.plainDecEqOf F τ) (HListPlainOf.decEq F rest)

def variantPlainOf.decEq (F : PlainFamily) : (names : List String) → (rows : NRows) →
    DecidableEq (variantPlainOf F names rows)
  | _, .nil => inferInstanceAs (DecidableEq Empty)
  | [], .cons _ _ => inferInstanceAs (DecidableEq Empty)
  | _ :: names, .cons fields rest =>
      @instDecidableEqSum _ _ (HListPlainOf.decEq F fields) (variantPlainOf.decEq F names rest)
end

/-- Decidable equality of plain values at a family's lent layer. -/
def NTy.plainDecEq (τ : NTy) : DecidableEq τ.plain := NTy.plainDecEqOf Carriers.toPlain τ

/-- Decidable equality of plain rows at a family's lent layer. -/
def HListPlain.decEq (row : NRow) : DecidableEq (HListPlain row) :=
  HListPlainOf.decEq Carriers.toPlain row

/-- Decidable equality of plain variant values at a family's lent layer. -/
def variantPlain.decEq (names : List String) (rows : NRows) :
    DecidableEq (variantPlain names rows) :=
  variantPlainOf.decEq Carriers.toPlain names rows

mutual
/-- Decidable equality of native values, for the structural `==`. -/
def NTy.decEq : (τ : NTy) → DecidableEq τ.carrier
  | .unit => inferInstanceAs (DecidableEq Unit)
  | .bool => inferInstanceAs (DecidableEq Bool)
  | .int width signed => inferInstanceAs (DecidableEq (SpecInt (.bits width) signed))
  | .num => inferInstanceAs (DecidableEq (SpecInt .unbounded true))
  | .address => inferInstanceAs (DecidableEq String)
  | .signer => inferInstanceAs (DecidableEq String)
  | .string => inferInstanceAs (DecidableEq String)
  | .bytes => inferInstanceAs (DecidableEq (Array UInt8))
  | .char => inferInstanceAs (DecidableEq SpecChar)
  | .tuple elements => HList.decEq elements
  | .struct _ _ fields => @Subtype.instDecidableEq _ _ (HListPlain.decEq fields)
  | .enum _ _ names rows _ => @Subtype.instDecidableEq _ _ (variantPlain.decEq names rows)
  | .slice element => @instDecidableEqSpecVector _ element.decEq
  | .vector element => @instDecidableEqSpecVector _ element.decEq
  | .array element _ => fun left right =>
      have := @instDecidableEqSpecVector _ element.decEq
      decidable_of_iff (left.vector = right.vector) ⟨SpecArray.ext, fun h => h ▸ rfl⟩
  | .table key value => @instDecidableEqSpecEntries _ (@instDecidableEqProd _ _ key.decEq value.decEq)
  | .ghost slot => slot.decEq
  | .sliceRef element => fun left right =>
      letI := element.decEq
      decidable_of_iff (left.currentVector = right.currentVector ∧ left.finalVector = right.finalVector)
        ⟨fun h => SpecSliceRef.ext h.1 h.2, fun h => h ▸ ⟨rfl, rfl⟩⟩
  | .ref referent => @instDecidableEqProd _ _ referent.plainDecEq referent.decEq
  | .param index => Carriers.decEq index
  | .function parameters shared results => (Carriers.functions parameters shared results).decEq
  | .nominal source arguments productive => (Carriers.nominals source arguments productive).decEq
  | .observed referent => referent.decEq


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

/-- The row codecs of an enum's variants. -/
@[reducible] def RowCodecs : NRows → Type
  | .nil => Unit
  | .cons fields rest => Codec (HList fields) (List RuntimeValue) × RowCodecs rest

/-- Encode a variant value under its declaration's handle and name. -/
def variantEncode (source : StructHandle) : (names : List String) → (rows : NRows) →
    RowCodecs rows → variantCarrier names rows → RuntimeValue
  -- Read with `Sum.elim`, and a constant where no variant is left
  -- (`variantPlainOf`).
  | _, .nil, _, _ => .unit
  | [], .cons _ _, _, _ => .unit
  | name :: names, .cons _ rest, (codec, codecs), value =>
      Sum.elim (fun fields => (Codec.nominalRow source (some name) codec).encode fields)
        (fun later => variantEncode source names rest codecs later) value

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

/-- No enum value is decoded from the unit. -/
theorem variantDecode?_unit (source : StructHandle) : (names : List String) → (rows : NRows) →
    (codecs : RowCodecs rows) → variantDecode? source names rows codecs .unit = none
  | [], .nil, _ => by simp [variantDecode?]
  | _ :: _, .nil, _ => by simp [variantDecode?]
  | [], .cons _ _, _ => by simp [variantDecode?]
  | _ :: _, .cons _ _, _ => by simp [variantDecode?]

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
      simp [variantEncode, Sum.elim_inl, Sum.elim_inr, variantDecode?, Codec.nominalRow, codec.decode_encode]
  | name :: names, .cons _ rest, (codec, codecs), nodup, .inr later => by
      obtain ⟨actual, fields, encoded, member⟩ :=
        variantEncode_shape source names rest codecs later
      have distinct : actual ≠ name := fun equal =>
        (List.nodup_cons.mp nodup).1 (equal ▸ member)
      simp only [variantEncode, Sum.elim_inl, Sum.elim_inr]
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
noncomputable def NTy.codec : (τ : NTy) → Codec τ.carrier RuntimeValue
  | .unit => Codec.unit
  | .bool => Codec.bool
  | .int width signed => Codec.specInt (.bits width) signed
  | .num => Codec.specInt .unbounded true
  | .address => Codec.address
  | .signer => Codec.signer
  | .string => Codec.string
  | .bytes => Codec.bytes
  | .char => Codec.character
  | .tuple elements => Codec.tuple (rowCodec elements)
  | .struct source arguments fields =>
      Codec.certified (NTy.plainCodec (.struct source arguments fields))
        (NTy.deepHolds (.struct source arguments fields))
  | .enum source arguments names rows distinct =>
      Codec.certified (NTy.plainCodec (.enum source arguments names rows distinct))
        (NTy.deepHolds (.enum source arguments names rows distinct))
  | .slice element => Codec.boundedVector element.codec

  | .vector element => Codec.boundedVector element.codec
  | .array element length => Codec.array element.codec length
  | .table key value => Codec.entries key.codec value.codec
  | .ghost slot => Codec.ghost slot.codec
  | .sliceRef element => Codec.sliceRef element.codec
  | .ref referent => Codec.pair referent.plainCodec referent.codec
  | .param index => Carriers.codec index
  | .function parameters shared results => (Carriers.functions parameters shared results).codec
  | .nominal source arguments productive => (Carriers.nominals source arguments productive).codec
  | .observed referent => referent.codec

/-- The codec of a row of values. -/
noncomputable def rowCodec : (row : NRow) → Codec (HList row) (List RuntimeValue)
  | .nil => Codec.tupleNil
  | .cons τ rest => Codec.tupleCons τ.codec (rowCodec rest)

noncomputable def rowCodecs : (rows : NRows) → RowCodecs rows
  | .nil => ()
  | .cons fields rest => (rowCodec fields, rowCodecs rest)
end

/-- The runtime representation of a native value. -/
noncomputable def NTy.encode (τ : NTy) (value : τ.carrier) : RuntimeValue := τ.codec.encode value


mutual
/-- What a shared view of a value holds, at the plain layer: each mutable
reference as its current value (`RuntimeValue.observe`).  The current of a
reference is plain (`NTy.carrier`), so the observation is a plain value; a
shared view's certified value is made where the view is taken, a copy out
of a plain current that owes the invariants there (`Term.observe`,
`certifyingAt`). -/
def NTy.plainObserve : (τ : NTy) → τ.plain → τ.erase.plain
  | .sliceRef element, value => value.currentVector.map element.plainObserve
  | .ref σ, value => σ.plainObserve value.1
  | .tuple elements, values => HListPlain.plainObserve elements values
  | .struct _ _ fields, values => HListPlain.plainObserve fields values
  | .enum _ _ names rows _, value => variantPlainObserve names rows value
  | .slice element, vector =>
      ⟨vector.values.map (NTy.plainObserve element), by simpa using vector.bounded⟩
  | .vector element, vector =>
      ⟨vector.values.map (NTy.plainObserve element), by simpa using vector.bounded⟩
  | .array element _, array =>
      ⟨⟨array.vector.values.map (NTy.plainObserve element), by simpa using array.vector.bounded⟩,
        by simpa using array.size_eq⟩
  | .table key value, entries =>
      ⟨entries.entries.map fun entry => (NTy.plainObserve key entry.1, NTy.plainObserve value entry.2),
        by simpa using entries.bounded⟩
  | .ghost slot, value => NTy.plainObserve slot value
  | .unit, value | .bool, value | .int _ _, value | .num, value | .address, value | .signer, value
  | .string, value | .bytes, value | .char, value | .param _, value | .function _ _ _, value | .nominal _ _ _, value => value
  | .observed referent, value => NTy.plainObserve referent value

def HListPlain.plainObserve : (row : NRow) → HListPlain row → HListPlain row.erase
  | .nil, values => values
  | .cons τ rest, values => (τ.plainObserve values.1, HListPlain.plainObserve rest values.2)

def variantPlainObserve : (names : List String) → (rows : NRows) → variantPlain names rows →
    variantPlain names rows.erase
  | _, .nil, value => value
  | [], .cons _ _, value => value
  | _ :: _, .cons fields _, .inl values => .inl (HListPlain.plainObserve fields values)
  | _ :: names, .cons _ rest, .inr later => .inr (variantPlainObserve names rest later)
end

mutual
/-- What a shared view holds as a clause reads it, at the plain layer: each
mutable reference as one whose current value is its prophecy, as
`NTy.plainViewed` has it. -/
def NTy.plainUnerase : (τ : NTy) → τ.erase.plain → τ.plain
  | .sliceRef element, value => SpecSliceRef.settled (value.map element.plainUnerase)
  | .ref σ, value => let seen := σ.plainUnerase value; (seen, seen)
  | .tuple elements, values => HListPlain.plainUnerase elements values
  | .struct _ _ fields, values => HListPlain.plainUnerase fields values
  | .enum _ _ names rows _, value => variantPlainUnerase names rows value
  | .slice element, vector =>
      ⟨vector.values.map (NTy.plainUnerase element), by simpa using vector.bounded⟩
  | .vector element, vector =>
      ⟨vector.values.map (NTy.plainUnerase element), by simpa using vector.bounded⟩
  | .array element _, array =>
      ⟨⟨array.vector.values.map (NTy.plainUnerase element), by simpa using array.vector.bounded⟩,
        by simpa using array.size_eq⟩
  | .table key value, entries =>
      ⟨entries.entries.map fun entry => (NTy.plainUnerase key entry.1, NTy.plainUnerase value entry.2),
        by simpa using entries.bounded⟩
  | .ghost slot, value => NTy.plainUnerase slot value
  | .unit, value | .bool, value | .int _ _, value | .num, value | .address, value | .signer, value
  | .string, value | .bytes, value | .char, value | .param _, value | .function _ _ _, value | .nominal _ _ _, value => value
  | .observed referent, value => NTy.plainUnerase referent value

def HListPlain.plainUnerase : (row : NRow) → HListPlain row.erase → HListPlain row
  | .nil, values => values
  | .cons τ rest, values => (τ.plainUnerase values.1, HListPlain.plainUnerase rest values.2)

def variantPlainUnerase : (names : List String) → (rows : NRows) →
    variantPlain names rows.erase → variantPlain names rows
  | _, .nil, value => value
  | [], .cons _ _, value => value
  | _ :: _, .cons fields _, .inl values => .inl (HListPlain.plainUnerase fields values)
  | _ :: names, .cons _ rest, .inr later => .inr (variantPlainUnerase names rest later)
end

@[simp] theorem NTy.encode_unit (value : Unit) : NTy.encode .unit value = .unit := rfl
@[simp] theorem NTy.encode_bool (value : Bool) : NTy.encode .bool value = .bool value := rfl
@[simp] theorem NTy.encode_int (width : Nat) (signed : Bool)
    (value : SpecInt (.bits width) signed) :
    NTy.encode (.int width signed) value = .integer value.val := rfl
@[simp] theorem NTy.encode_num (value : SpecInt .unbounded true) :
    NTy.encode .num value = .integer value.val := rfl
@[simp] theorem NTy.encode_address (value : String) :
    NTy.encode .address value = .address value := rfl
@[simp] theorem NTy.encode_signer (value : String) :
    NTy.encode .signer value = .signer value := rfl
@[simp] theorem NTy.encode_string (value : String) :
    NTy.encode .string value = .string value := rfl
@[simp] theorem NTy.encode_bytes (value : Array UInt8) :
    NTy.encode .bytes value = .bytes value := rfl
@[simp] theorem NTy.encode_char (value : SpecChar) :
    NTy.encode .char value = .character value.val := rfl
@[simp] theorem NTy.encode_function (parameters results : NRow)
    (value : (NTy.function parameters shared results).carrier) :
    NTy.encode (.function parameters shared results) value =
      ((Carriers.functions parameters shared results).closure value).encode := rfl
@[simp] theorem NTy.encode_nominal (source : StructHandle) (arguments : NRow) (productive : Bool)
    (value : (NTy.nominal source arguments productive).carrier) :
    NTy.encode (.nominal source arguments productive) value =
      (Carriers.nominals source arguments productive).value value := rfl

/-- The storage key a native value keys memory by: an address or a signer by
its address, a scalar by its value, as the runtime keys storage by the
value's encoding (`RuntimeValue.storageKey`). A value of another type keys
storage as its encoding does; validation lets no such value key storage. -/
noncomputable def NTy.storageKey : (τ : NTy) → τ.carrier → StorageKey
  | .unit, _ => .unit
  | .bool, value => .bool value
  | .int _ _, value => .integer value.val
  | .num, value => .integer value.val
  | .address, value => .address value
  | .signer, value => .address value
  | .string, value => .string value
  | .bytes, value => .bytes value
  | .char, value => .character value.val
  | .tuple elements, value => (NTy.encode (.tuple elements) value).storageKey
  | .struct source arguments fields, value =>
      (NTy.encode (.struct source arguments fields) value).storageKey
  | .enum source arguments names rows distinct, value =>
      (NTy.encode (.enum source arguments names rows distinct) value).storageKey
  | .slice element, value => (NTy.encode (.slice element) value).storageKey
  | .vector element, value => (NTy.encode (.vector element) value).storageKey
  | .array element length, value => (NTy.encode (.array element length) value).storageKey
  | .table key value, entries => (NTy.encode (.table key value) entries).storageKey
  | .ghost slot, value => (NTy.encode (.ghost slot) value).storageKey
  | .sliceRef element, value => (NTy.encode (.sliceRef element) value).storageKey
  | .ref referent, value => (NTy.encode (.ref referent) value).storageKey
  | .param index, value => (NTy.encode (.param index) value).storageKey
  | .function parameters shared results, value =>
      (NTy.encode (.function parameters shared results) value).storageKey
  | .nominal source arguments productive, value =>
      (NTy.encode (.nominal source arguments productive) value).storageKey
  | .observed referent, value => (NTy.encode (.observed referent) value).storageKey

/-- A native value keys storage as its encoding does. -/
theorem NTy.storageKey_encode (τ : NTy) (value : τ.carrier) :
    (NTy.encode τ value).storageKey = τ.storageKey value := by
  cases τ <;> rfl

@[simp] theorem NTy.storageKey_address (address : String) :
    NTy.storageKey .address address = .address address := rfl
@[simp] theorem NTy.storageKey_signer (address : String) :
    NTy.storageKey .signer address = .address address := rfl
@[simp] theorem NTy.storageKey_int (width : Nat) (signed : Bool)
    (value : SpecInt (.bits width) signed) :
    NTy.storageKey (.int width signed) value = .integer value.val := rfl
@[simp] theorem NTy.storageKey_bool (value : Bool) : NTy.storageKey .bool value = .bool value := rfl
@[simp] theorem NTy.storageKey_string (value : String) :
    NTy.storageKey .string value = .string value := rfl

/-- A value's encoding does not see a cast between equal types. -/
theorem NTy.encode_cast {τ τ' : NTy} (same : τ = τ') (value : τ.carrier) :
    NTy.encode τ' (same ▸ value) = NTy.encode τ value := by
  subst same
  rfl
@[simp] theorem NTy.encode_array (element : NTy) {length : Nat}
    (value : SpecArray element.carrier length) :
    NTy.encode (.array element length) value =
      .vector (value.vector.values.map element.codec.encode) := rfl
@[simp] theorem NTy.encode_slice (element : NTy) (value : SpecVector element.carrier) :
    NTy.encode (.slice element) value = .vector (value.values.map element.codec.encode) := rfl
@[simp] theorem NTy.encode_vector (element : NTy) (value : SpecVector element.carrier) :
    NTy.encode (.vector element) value = .vector (value.values.map element.codec.encode) := rfl
@[simp] theorem NTy.encode_sliceRef (element : NTy) (value : SpecSliceRef element.carrier) :
    NTy.encode (.sliceRef element) value =
      .tuple #[NTy.encode (.vector element) value.currentVector,
        NTy.encode (.vector element) value.finalVector] := rfl
@[simp] theorem NTy.encode_table (key value : NTy)
    (entries : SpecEntries (key.carrier × value.carrier)) :
    NTy.encode (.table key value) entries =
      .vector (entries.entries.map fun entry =>
        .tuple #[key.encode entry.1, value.encode entry.2]).toArray :=
  rfl

/-- The runtime row a row of values denotes. -/
noncomputable def HList.encode {Γ : NRow} (values : HList Γ) : List RuntimeValue :=
  (rowCodec Γ).encode values

/-- The runtime row a plain row of values denotes. -/
def HListPlain.encode {Γ : NRow} (values : HListPlain Γ) : List RuntimeValue :=
  (rowCodecPlain Γ).encode values

@[simp] theorem HList.encode_nil (values : HList .nil) : HList.encode values = [] := rfl
@[simp] theorem HList.encode_cons {τ : NTy} {Γ : NRow} (values : HList (.cons τ Γ)) :
    HList.encode values = τ.encode values.1 :: HList.encode values.2 := rfl
@[simp] theorem HListPlain.encode_nil (values : HListPlain .nil) : HListPlain.encode values = [] := rfl
@[simp] theorem HListPlain.encode_cons {τ : NTy} {Γ : NRow} (values : HListPlain (.cons τ Γ)) :
    HListPlain.encode values = τ.plainEncode values.1 :: HListPlain.encode values.2 := rfl

@[simp] theorem NTy.plainEncode_table (key value : NTy)
    (entries : SpecEntries (key.plain × value.plain)) :
    NTy.plainEncode (.table key value) entries =
      .vector (entries.entries.map fun entry =>
        .tuple #[key.plainEncode entry.1, value.plainEncode entry.2]).toArray :=
  rfl

@[simp] theorem NTy.encode_tuple (elements : NRow) (value : HList elements) :
    NTy.encode (.tuple elements) value = .tuple value.encode.toArray := rfl
/-- A certified struct encodes as its plain row does. -/
@[simp] theorem NTy.encode_struct (source : StructHandle) (arguments fields : NRow)
    (value : (NTy.struct source arguments fields).carrier) :
    NTy.encode (.struct source arguments fields) value =
      .nominal source none (HListPlain.encode value.val).toArray := rfl
/-- A certified struct encodes as the plain row it certifies. -/
theorem NTy.encode_struct_val (source : StructHandle) (arguments fields : NRow)
    (value : (NTy.struct source arguments fields).carrier) :
    NTy.encode (.struct source arguments fields) value =
      NTy.plainEncode (.struct source arguments fields) value.val := rfl
@[simp] theorem NTy.plainEncode_struct (source : StructHandle) (arguments fields : NRow)
    (value : HListPlain fields) :
    NTy.plainEncode (.struct source arguments fields) value =
      .nominal source none value.encode.toArray := rfl
@[simp] theorem NTy.plainEncode_enum_inl (source : StructHandle) (name : String)
    (names : List String) (fields : NRow) (rest : NRows) (distinct : (name :: names).Nodup)
    (value : HListPlain fields) :
    NTy.plainEncode (.enum source arguments (name :: names) (.cons fields rest) distinct)
        (.inl value) =
      .nominal source (some name) value.encode.toArray := rfl
/-- A certified variant encodes as its plain value does. -/
@[simp] theorem NTy.encode_enum (source : StructHandle) (arguments : NRow) (names : List String)
    (rows : NRows) (distinct : names.Nodup)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    NTy.encode (.enum source arguments names rows distinct) value =
      NTy.plainEncode (.enum source arguments names rows distinct) value.val := rfl
@[simp] theorem NTy.encode_ref (referent : NTy) (value : referent.plain × referent.carrier) :
    NTy.encode (.ref referent) value =
      .tuple #[referent.plainEncode value.1, referent.encode value.2] := rfl
@[simp] theorem NTy.encode_ghost (slot : NTy) (value : slot.carrier) :
    NTy.encode (.ghost slot) value = .ghost (slot.encode value) := rfl
@[simp] theorem NTy.plainEncode_enum_inr (source : StructHandle) (name : String)
    (names : List String) (fields : NRow) (rest : NRows) (distinct : (name :: names).Nodup)
    (value : variantPlain names rest) :
    NTy.plainEncode (.enum source arguments (name :: names) (.cons fields rest) distinct)
        (.inr value) =
      NTy.plainEncode (.enum source arguments names rest (List.nodup_cons.mp distinct).2) value := rfl

theorem NTy.encode_injective (τ : NTy) {left right : τ.carrier}
    (equal : τ.encode left = τ.encode right) : left = right :=
  τ.codec.encode_injective equal

/-- A plain value is determined by its encoding. -/
theorem NTy.plainEncode_injective (τ : NTy) {left right : τ.plain}
    (equal : τ.plainEncode left = τ.plainEncode right) : left = right :=
  τ.plainCodec.encode_injective equal

@[simp] theorem NTy.plainEncode_tuple (elements : NRow) (value : HListPlain elements) :
    NTy.plainEncode (.tuple elements) value = .tuple value.encode.toArray := rfl
@[simp] theorem NTy.plainEncode_vector (element : NTy) (value : SpecVector element.plain) :
    NTy.plainEncode (.vector element) value =
      .vector (value.values.map element.plainCodec.encode) := rfl
/-- A plain slice encodes as a vector of its elements' encodings, as its
certified value does (`NTy.encode_slice`). -/
theorem NTy.plainEncode_slice (element : NTy) (value : SpecVector element.plain) :
    NTy.plainEncode (.slice element) value =
      .vector (value.values.map element.plainCodec.encode) := rfl
/-- A plain array encodes as the vector it holds (`NTy.encode_array`). -/
theorem NTy.plainEncode_array (element : NTy) {length : Nat}
    (value : SpecArray element.plain length) :
    NTy.plainEncode (.array element length) value =
      .vector (value.vector.values.map element.plainCodec.encode) := rfl
/-- A plain slice reference encodes as the pair of its readings
(`NTy.encode_sliceRef`). -/
theorem NTy.plainEncode_sliceRef (element : NTy) (value : SpecSliceRef element.plain) :
    NTy.plainEncode (.sliceRef element) value =
      .tuple #[NTy.plainEncode (.vector element) value.currentVector,
        NTy.plainEncode (.vector element) value.finalVector] := rfl
/-- A mark says nothing about an encoding: an observed plain value encodes
as the value it marks. -/
theorem NTy.plainEncode_observed (referent : NTy) (value : (NTy.observed referent).plain) :
    NTy.plainEncode (.observed referent) value = NTy.plainEncode referent value := rfl
@[simp] theorem NTy.plainEncode_ghost (slot : NTy) (value : slot.plain) :
    NTy.plainEncode (.ghost slot) value = .ghost (slot.plainEncode value) := rfl
@[simp] theorem NTy.plainEncode_ref (referent : NTy) (value : referent.plain × referent.plain) :
    NTy.plainEncode (.ref referent) value =
      .tuple #[referent.plainEncode value.1, referent.plainEncode value.2] := rfl
theorem NTy.plainEncode_param (index : Nat) (value : (NTy.param index).plain) :
    NTy.plainEncode (.param index) value = (Carriers.lent index).codec.encode value := rfl

mutual
/-- A certified value as a lent one: its certificates removed at every
depth, which keeps its encoding (`NTy.plainEncode_forget`).  This is what
a borrow hands a mutable reference as its current
(`doc/designs/data-invariants.md`). -/
def NTy.forget : (τ : NTy) → τ.carrier → τ.plain
  | .unit, value => value
  | .bool, value => value
  | .int _ _, value => value
  | .num, value => value
  | .address, value => value
  | .signer, value => value
  | .string, value => value
  | .bytes, value => value
  | .char, value => value
  | .tuple elements, values => HList.forget elements values
  | .struct _ _ _, value => value.val
  | .enum _ _ _ _ _, value => value.val
  | .vector element, vector =>
      ⟨vector.values.map (NTy.forget element), by simpa using vector.bounded⟩
  | .slice element, vector =>
      ⟨vector.values.map (NTy.forget element), by simpa using vector.bounded⟩
  | .array element _, array =>
      ⟨⟨array.vector.values.map (NTy.forget element), by simpa using array.vector.bounded⟩,
        by simpa using array.size_eq⟩
  | .table key value, entries =>
      ⟨entries.entries.map fun entry => (NTy.forget key entry.1, NTy.forget value entry.2),
        by simpa using entries.bounded⟩
  | .ghost slot, value => NTy.forget slot value
  | .ref referent, value => (value.1, NTy.forget referent value.2)
  | .sliceRef element, value => value.map (NTy.forget element)
  | .observed referent, value => NTy.forget referent value
  | .param index, value => (Carriers.lent index).of value
  | .function _ _ _, value => value
  | .nominal _ _ _, value => value
termination_by structural τ _ => τ

/-- A row of certified values as a row of lent ones. -/
def HList.forget : (row : NRow) → HList row → HListPlain row
  | .nil, _ => ()
  | .cons τ rest, values => (NTy.forget τ values.1, HList.forget rest values.2)
termination_by structural row _ => row
end

mutual
/-- Forgetting a value's certificates keeps its encoding. -/
theorem NTy.plainEncode_forget : (τ : NTy) → (value : τ.carrier) →
    NTy.plainEncode τ (NTy.forget τ value) = NTy.encode τ value
  | .unit, _ => rfl
  | .bool, _ => rfl
  | .int _ _, _ => rfl
  | .num, _ => rfl
  | .address, _ => rfl
  | .signer, _ => rfl
  | .string, _ => rfl
  | .bytes, _ => rfl
  | .char, _ => rfl
  | .struct _ _ _, _ => rfl
  | .enum _ _ _ _ _, _ => rfl
  | .function _ _ _, _ => rfl
  | .nominal _ _ _, _ => rfl
  | .observed referent, value => NTy.plainEncode_forget referent value
  | .param index, value => (Carriers.lent index).encode_of value
  | .tuple elements, values => by
      rw [NTy.plainEncode_tuple, NTy.encode_tuple]
      simp only [NTy.forget]
      rw [HList.encode_forget elements values]
  | .vector element, vector => by
      rw [NTy.plainEncode_vector, NTy.encode_vector]
      simp only [NTy.forget, Array.map_map]
      congr 1
      apply Array.map_congr_left
      intro value _
      exact NTy.plainEncode_forget element value
  | .slice element, vector => by
      rw [NTy.plainEncode_slice, NTy.encode_slice]
      simp only [NTy.forget, Array.map_map]
      congr 1
      apply Array.map_congr_left
      intro value _
      exact NTy.plainEncode_forget element value
  | .array element _, array => by
      rw [NTy.plainEncode_array, NTy.encode_array]
      simp only [NTy.forget, Array.map_map]
      congr 1
      apply Array.map_congr_left
      intro value _
      exact NTy.plainEncode_forget element value
  | .sliceRef element, value => by
      have step : ∀ reading : SpecVector element.carrier,
          NTy.plainEncode (.vector element) (reading.map (NTy.forget element)) =
            NTy.encode (.vector element) reading := by
        intro reading
        rw [NTy.plainEncode_vector, NTy.encode_vector]
        simp only [SpecVector.map_values, Array.map_map]
        congr 1
        apply Array.map_congr_left
        intro value _
        exact NTy.plainEncode_forget element value
      rw [NTy.plainEncode_sliceRef, NTy.encode_sliceRef]
      simp only [NTy.forget, SpecSliceRef.map_currentVector, SpecSliceRef.map_finalVector]
      rw [step value.currentVector, step value.finalVector]
  | .table key value, entries => by
      rw [NTy.plainEncode_table, NTy.encode_table]
      simp only [NTy.forget, List.map_map]
      congr 2
      apply List.map_congr_left
      intro entry _
      simp only [Function.comp_apply, NTy.plainEncode_forget key entry.1, NTy.plainEncode_forget value entry.2]
  | .ghost slot, value => by
      rw [NTy.plainEncode_ghost, NTy.encode_ghost]
      simp only [NTy.forget]
      rw [NTy.plainEncode_forget slot value]
  | .ref referent, value => by
      rw [NTy.plainEncode_ref, NTy.encode_ref]
      simp only [NTy.forget]
      rw [NTy.plainEncode_forget referent value.2]

/-- Forgetting a row's certificates keeps its encoding. -/
theorem HList.encode_forget : (row : NRow) → (values : HList row) →
    HListPlain.encode (HList.forget row values) = HList.encode values
  | .nil, _ => rfl
  | .cons τ rest, values => by
      rw [HList.forget, HListPlain.encode_cons, HList.encode_cons,
        NTy.plainEncode_forget τ values.1, HList.encode_forget rest values.2]
end

/-! ## The structural order of native values

`NTy.plainOrder` orders plain values by recursion on their type: scalars by
value, rows, vectors and entries lexicographically, variants by declaration
position and name and then their fields, a type parameter's values by the
family's order, and function values as closures. `NTy.order` observes native
values through this same order after forgetting their certificates.
`NTy.order_encode` states agreement with `RuntimeValue.order`. -/

omit [Carriers] in
/-- The name of the variant a plain value holds. -/
def variantPlainNameOf (F : PlainFamily) : (names : List String) → (rows : NRows) →
    variantPlainOf F names rows → String
  -- Read with `Sum.elim`, and a constant where no variant is left
  -- (`variantPlainOf`).
  | _, .nil, _ => ""
  | [], .cons _ _, _ => ""
  | name :: names, .cons _ rest, value =>
      Sum.elim (fun _ => name) (variantPlainNameOf F names rest) value
termination_by structural names rows _ => rows

omit [Carriers] in
theorem variantPlainNameOf_mem (F : PlainFamily) : (names : List String) → (rows : NRows) →
    (value : variantPlainOf F names rows) → variantPlainNameOf F names rows value ∈ names
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons _ _, .inl _ => List.mem_cons_self
  | _ :: names, .cons _ rest, .inr later =>
      List.mem_cons_of_mem _ (variantPlainNameOf_mem F names rest later)

/-- The name of the variant a plain value holds, at a family's lent layer. -/
def variantPlainName (names : List String) (rows : NRows) (value : variantPlain names rows) :
    String := variantPlainNameOf Carriers.toPlain names rows value

theorem variantPlainName_mem (names : List String) (rows : NRows)
    (value : variantPlain names rows) : variantPlainName names rows value ∈ names :=
  variantPlainNameOf_mem Carriers.toPlain names rows value

/-- The name of the variant a value holds. -/
def variantName : (names : List String) → (rows : NRows) → variantCarrier names rows → String
  -- Read with `Sum.elim`, and a constant where no variant is left
  -- (`variantPlainOf`).
  | _, .nil, _ => ""
  | [], .cons _ _, _ => ""
  | name :: names, .cons _ rest, value =>
      Sum.elim (fun _ => name) (variantName names rest) value
termination_by structural names rows _ => rows

theorem variantName_mem : (names : List String) → (rows : NRows) →
    (value : variantCarrier names rows) → variantName names rows value ∈ names
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons _ _, .inl _ => List.mem_cons_self
  | _ :: names, .cons _ rest, .inr later =>
      List.mem_cons_of_mem _ (variantName_mem names rest later)

omit [Carriers] in
mutual
/-- The structural order of two plain values of one type. -/
def NTy.plainOrderOf (F : PlainFamily) (rank : ValueRanks) : (τ : NTy) →
    NTy.plainOf F τ → NTy.plainOf F τ → Ordering
  | .unit, _, _ => .eq
  | .bool, left, right => compare left right
  | .int _ _, left, right => compare left.val right.val
  | .num, left, right => compare left.val right.val
  | .address, left, right => compareAddress left right
  | .signer, left, right => compareAddress left right
  | .string, left, right => compare left right
  | .bytes, left, right => List.compareLex compare left.toList right.toList
  | .char, left, right => compare left.val right.val
  | .tuple elements, left, right => HListPlainOf.order F rank elements left right
  | .struct _ _ fields, left, right => HListPlainOf.order F rank fields left right
  | .enum source _ names rows _, left, right => variantPlainOf.order F rank source names rows left right
  | .vector element, left, right =>
      List.compareLex (NTy.plainOrderOf F rank element) left.values.toList right.values.toList
  | .slice element, left, right =>
      List.compareLex (NTy.plainOrderOf F rank element) left.values.toList right.values.toList
  | .array element _, left, right =>
      List.compareLex (NTy.plainOrderOf F rank element) left.vector.values.toList
        right.vector.values.toList
  | .table key value, left, right =>
      List.compareLex (fun l r => (NTy.plainOrderOf F rank key l.1 r.1).then (NTy.plainOrderOf F rank value l.2 r.2))
        left.entries right.entries
  | .ghost slot, left, right => NTy.plainOrderOf F rank slot left right
  | .sliceRef element, left, right =>
      (List.compareLex (NTy.plainOrderOf F rank element) left.currentVector.values.toList
          right.currentVector.values.toList).then
        (List.compareLex (NTy.plainOrderOf F rank element) left.finalVector.values.toList
          right.finalVector.values.toList)
  | .ref referent, left, right =>
      (NTy.plainOrderOf F rank referent left.1 right.1).then (NTy.plainOrderOf F rank referent left.2 right.2)
  | .observed referent, left, right => NTy.plainOrderOf F rank referent left right
  | .param index, left, right => (F.lent index).orders.order rank left right
  | .function parameters shared results, left, right =>
      (F.functions parameters shared results).order rank left right
  | .nominal source arguments productive, left, right =>
      (F.nominals source arguments productive).order rank left right

/-- The lexicographic order of two rows of one row type. -/
def HListPlainOf.order (F : PlainFamily) (rank : ValueRanks) : (row : NRow) →
    HListPlainOf F row → HListPlainOf F row → Ordering
  | .nil, _, _ => .eq
  | .cons τ rest, left, right =>
      (NTy.plainOrderOf F rank τ left.1 right.1).then (HListPlainOf.order F rank rest left.2 right.2)

/-- The order of two variant values of one declaration: the same variant's
by their fields, different variants' as `compareVariant` orders them. -/
def variantPlainOf.order (F : PlainFamily) (rank : ValueRanks) (source : StructHandle) :
    (names : List String) → (rows : NRows) → variantPlainOf F names rows → variantPlainOf F names rows →
      Ordering
  -- Read with `Sum.elim`, and a constant where no variant is left
  -- (`variantPlainOf`).
  | _, .nil, _, _ => .eq
  | [], .cons _ _, _, _ => .eq
  | name :: names, .cons fields rest, left, right =>
      Sum.elim
        (fun leftRow =>
          Sum.elim (HListPlainOf.order F rank fields leftRow)
            (fun rightLater => compareVariant rank source source (some name)
              (some (variantPlainNameOf F names rest rightLater)))
            right)
        (fun leftLater =>
          Sum.elim
            (fun _ => compareVariant rank source source
              (some (variantPlainNameOf F names rest leftLater)) (some name))
            (variantPlainOf.order F rank source names rest leftLater)
            right)
        left
end

/-- The structural order of two plain values at a family's lent layer. -/
def NTy.plainOrder (rank : ValueRanks) (τ : NTy) (left right : τ.plain) : Ordering :=
  NTy.plainOrderOf Carriers.toPlain rank τ left right

/-- The lexicographic order of two plain rows at a family's lent layer. -/
def HListPlain.order (rank : ValueRanks) (row : NRow) (left right : HListPlain row) : Ordering :=
  HListPlainOf.order Carriers.toPlain rank row left right

/-- The order of plain variant values at a family's lent layer. -/
def variantPlain.order (rank : ValueRanks) (source : StructHandle) (names : List String)
    (rows : NRows) (left right : variantPlain names rows) : Ordering :=
  variantPlainOf.order Carriers.toPlain rank source names rows left right

/-- The structural order of native values. A type parameter uses its
family's certified order; constructed types observe their plain values. -/
def NTy.order (rank : ValueRanks) : (τ : NTy) → τ.carrier → τ.carrier → Ordering
  | .param index, left, right => (Carriers.orders index).order rank left right
  | τ, left, right => NTy.plainOrder rank τ (NTy.forget τ left) (NTy.forget τ right)

mutual
/-- The lexicographic order of two rows of one row type. -/
def HList.order (rank : ValueRanks) : (row : NRow) → HList row → HList row → Ordering
  | .nil, _, _ => .eq
  | .cons τ rest, left, right =>
      (NTy.order rank τ left.1 right.1).then (HList.order rank rest left.2 right.2)

/-- The order of two variant values of one declaration: the same variant's
by their fields, different variants' as `compareVariant` orders them. -/
def variantCarrier.order (rank : ValueRanks) (source : StructHandle) :
    (names : List String) → (rows : NRows) → variantCarrier names rows → variantCarrier names rows →
      Ordering
  -- Read with `Sum.elim`, and a constant where no variant is left
  -- (`variantPlainOf`).
  | _, .nil, _, _ => .eq
  | [], .cons _ _, _, _ => .eq
  | name :: names, .cons fields rest, left, right =>
      Sum.elim
        (fun leftRow =>
          Sum.elim (HList.order rank fields leftRow)
            (fun rightLater => compareVariant rank source source (some name)
              (some (variantName names rest rightLater)))
            right)
        (fun leftLater =>
          Sum.elim
            (fun _ => compareVariant rank source source
              (some (variantName names rest leftLater)) (some name))
            (variantCarrier.order rank source names rest leftLater)
            right)
        left
end

/-- An encoded variant value is a nominal value of its declaration, tagged
with the variant it holds. -/
theorem variantEncodePlain_nominal (source : StructHandle) : (names : List String) → (rows : NRows) →
    (codecs : RowCodecsPlain rows) → (value : variantPlain names rows) →
    ∃ fields, variantEncodePlain source names rows codecs value =
      .nominal source (some (variantPlainName names rows value)) fields
  | _, .nil, _, value => nomatch value
  | [], .cons _ _, _, value => nomatch value
  | _ :: _, .cons _ _, _, .inl _ => ⟨_, rfl⟩
  | _ :: names, .cons _ rest, (_, codecs), .inr later =>
      variantEncodePlain_nominal source names rest codecs later

/-- An encoded variant value is a nominal value of its declaration, tagged
with the variant it holds. -/
theorem variantEncode_nominal (source : StructHandle) : (names : List String) → (rows : NRows) →
    (codecs : RowCodecs rows) → (value : variantCarrier names rows) →
    ∃ fields, variantEncode source names rows codecs value =
      .nominal source (some (variantName names rows value)) fields
  | _, .nil, _, value => nomatch value
  | [], .cons _ _, _, value => nomatch value
  | _ :: _, .cons _ _, _, .inl _ => ⟨_, rfl⟩
  | _ :: names, .cons _ rest, (_, codecs), .inr later =>
      variantEncode_nominal source names rest codecs later

omit [Carriers] in
private theorem then_compare_self_nat (a : Nat) (after : Ordering) :
    (compare a a).then after = after := by
  simp

omit [Carriers] in
private theorem then_compare_self_string (a : String) (after : Ordering) :
    (compare a a).then after = after := by
  rw [Std.ReflOrd.compare_self]
  rfl

mutual
/-- Plain values order as their encodings do. -/
theorem NTy.plainOrder_codec (rank : ValueRanks) : (τ : NTy) → (left right : τ.plain) →
    RuntimeValue.order rank (τ.plainCodec.encode left) (τ.plainCodec.encode right) = NTy.plainOrder rank τ left right
  | .unit, _, _ => RuntimeValue.order_unit rank
  | .bool, left, right => RuntimeValue.order_bool rank left right
  | .int _ _, left, right => RuntimeValue.order_integer rank left.val right.val
  | .num, left, right => RuntimeValue.order_integer rank left.val right.val
  | .address, left, right => RuntimeValue.order_address rank left right
  | .signer, left, right => RuntimeValue.order_signer rank left right
  | .string, left, right => RuntimeValue.order_string rank left right
  | .bytes, left, right => RuntimeValue.order_bytes rank left right
  | .char, left, right => RuntimeValue.order_character rank left.val right.val
  | .tuple elements, left, right => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.tuple, RuntimeValue.order_tuple, NTy.plainOrder, NTy.plainOrderOf]
      exact HListPlain.order_codec rank elements left right
  | .struct source _ fields, left, right => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.nominalRow, RuntimeValue.order_nominal, then_compare_self_nat, compareVariant, Ordering.eq_then, NTy.plainOrder, NTy.plainOrderOf]
      exact HListPlain.order_codec rank fields left right
  | .enum source _ names rows distinct, left, right => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, variantCodecPlain, variantCodecPlainOf, NTy.plainOrder, NTy.plainOrderOf]
      exact variantPlain.order_codec rank source names rows distinct left right
  | .vector element, left, right => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.boundedVector, RuntimeValue.order_vector, Array.toList_map, RuntimeValue.orderList_map, NTy.plainOrder, NTy.plainOrderOf]
      congr 1
      funext a b
      exact NTy.plainOrder_codec rank element a b
  | .slice element, left, right => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.boundedVector, RuntimeValue.order_vector, Array.toList_map, RuntimeValue.orderList_map, NTy.plainOrder, NTy.plainOrderOf]
      congr 1
      funext a b
      exact NTy.plainOrder_codec rank element a b
  | .array element _, left, right => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.array, Codec.boundedVector, RuntimeValue.order_vector, Array.toList_map, RuntimeValue.orderList_map, NTy.plainOrder, NTy.plainOrderOf]
      congr 1
      funext a b
      exact NTy.plainOrder_codec rank element a b
  | .table key value, left, right => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.entries, RuntimeValue.order_vector, RuntimeValue.orderList_map, RuntimeValue.order_tuple, RuntimeValue.orderList_cons_cons, RuntimeValue.orderList_nil_nil, Ordering.then_eq, NTy.plainOrder, NTy.plainOrderOf]
      congr 1
      funext a b
      rw [NTy.plainOrder_codec rank key a.1 b.1, NTy.plainOrder_codec rank value a.2 b.2]
      rfl
  | .ghost slot, left, right => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.ghost, RuntimeValue.order_ghost, NTy.plainOrder, NTy.plainOrderOf]
      exact NTy.plainOrder_codec rank slot left right
  | .ref referent, left, right => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.prophecyPair, RuntimeValue.order_tuple, RuntimeValue.orderList_cons_cons, RuntimeValue.orderList_nil_nil, Ordering.then_eq, NTy.plainOrder, NTy.plainOrderOf]
      rw [NTy.plainOrder_codec rank referent left.1 right.1,
        NTy.plainOrder_codec rank referent left.2 right.2]
      rfl
  | .sliceRef element, left, right => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.sliceRef, Codec.boundedVector, RuntimeValue.order_tuple, RuntimeValue.orderList_cons_cons, RuntimeValue.orderList_nil_nil, Ordering.then_eq, RuntimeValue.order_vector, Array.toList_map, RuntimeValue.orderList_map, NTy.plainOrder, NTy.plainOrderOf]
      congr 1 <;> congr 1 <;> funext a b <;> exact NTy.plainOrder_codec rank element a b
  | .observed referent, left, right => NTy.plainOrder_codec rank referent left right
  | .param index, left, right => (Carriers.lent index).orders.order_encode rank left right
  | .function parameters shared results, left, right =>
      (Carriers.functions parameters shared results).orders.order_encode rank left right
  | .nominal source arguments productive, left, right =>
      (Carriers.nominals source arguments productive).orders.order_encode rank left right

/-- Rows order as the lists of their encodings do. -/
theorem HListPlain.order_codec (rank : ValueRanks) : (row : NRow) → (left right : HListPlain row) →
    RuntimeValue.orderList rank ((rowCodecPlain row).encode left) ((rowCodecPlain row).encode right) =
      HListPlain.order rank row left right
  | .nil, _, _ => RuntimeValue.orderList_nil_nil rank
  | .cons τ rest, left, right => by
      simp only [rowCodecPlain, rowCodecPlainOf, Codec.tupleCons, RuntimeValue.orderList_cons_cons, HListPlain.order, HListPlainOf.order]
      rw [NTy.plainOrder_codec rank τ left.1 right.1,
        HListPlain.order_codec rank rest left.2 right.2]
      rfl

/-- Variant values order as their encodings do. -/
theorem variantPlain.order_codec (rank : ValueRanks) (source : StructHandle) :
    (names : List String) → (rows : NRows) → names.Nodup → (left right : variantPlain names rows) →
    RuntimeValue.order rank (variantEncodePlain source names rows (rowCodecsPlain rows) left)
        (variantEncodePlain source names rows (rowCodecsPlain rows) right) =
      variantPlain.order rank source names rows left right
  | _, .nil, _, left, _ => nomatch left
  | [], .cons _ _, _, left, _ => nomatch left
  | _ :: _, .cons fields _, _, .inl left, .inl right => by
      simp only [variantEncodePlain, variantEncodePlainOf, Sum.elim_inl, Sum.elim_inr, rowCodecsPlain, rowCodecsPlainOf, Codec.nominalRow, RuntimeValue.order_nominal, then_compare_self_nat, then_compare_self_string, compareVariant, variantPlain.order, variantPlainOf.order]
      exact HListPlain.order_codec rank fields left right
  | name :: names, .cons _ rest, distinct, .inl left, .inr right => by
      obtain ⟨fields, encoded⟩ := variantEncodePlain_nominal source names rest (rowCodecsPlain rest) right
      have different : name ≠ variantPlainName names rest right := fun same =>
        (List.nodup_cons.mp distinct).1 (same ▸ variantPlainName_mem names rest right)
      simp only [variantEncodePlain, variantEncodePlainOf, Sum.elim_inl, Sum.elim_inr, rowCodecsPlain, rowCodecsPlainOf, Codec.nominalRow, encoded, RuntimeValue.order_nominal, then_compare_self_nat, compareVariant_then_of_ne rank source different, variantPlain.order, variantPlainOf.order]
      rfl
  | name :: names, .cons _ rest, distinct, .inr left, .inl right => by
      obtain ⟨fields, encoded⟩ := variantEncodePlain_nominal source names rest (rowCodecsPlain rest) left
      have different : variantPlainName names rest left ≠ name := fun same =>
        (List.nodup_cons.mp distinct).1 (same ▸ variantPlainName_mem names rest left)
      simp only [variantEncodePlain, variantEncodePlainOf, Sum.elim_inl, Sum.elim_inr, rowCodecsPlain, rowCodecsPlainOf, Codec.nominalRow, encoded, RuntimeValue.order_nominal, then_compare_self_nat, compareVariant_then_of_ne rank source different, variantPlain.order, variantPlainOf.order]
      rfl
  | _ :: names, .cons _ rest, distinct, .inr left, .inr right => by
      simp only [variantEncodePlain, variantEncodePlainOf, Sum.elim_inl, Sum.elim_inr, rowCodecsPlain, rowCodecsPlainOf, variantPlain.order, variantPlainOf.order]
      exact variantPlain.order_codec rank source names rest (List.nodup_cons.mp distinct).2 left right
end

/-- Native values order as their encodings do. -/
theorem NTy.order_codec (rank : ValueRanks) (τ : NTy) (left right : τ.carrier) :
    RuntimeValue.order rank (τ.codec.encode left) (τ.codec.encode right) = NTy.order rank τ left right := by
  have plain : RuntimeValue.order rank (τ.codec.encode left) (τ.codec.encode right) =
      NTy.plainOrder rank τ (NTy.forget τ left) (NTy.forget τ right) := by
    change RuntimeValue.order rank (τ.encode left) (τ.encode right) = _
    rw [← NTy.plainEncode_forget τ left, ← NTy.plainEncode_forget τ right]
    exact NTy.plainOrder_codec rank τ (NTy.forget τ left) (NTy.forget τ right)
  cases τ with
  | param index => exact (Carriers.orders index).order_encode rank left right
  | _ => exact plain

mutual
/-- Rows order as the lists of their encodings do. -/
theorem HList.order_codec (rank : ValueRanks) : (row : NRow) → (left right : HList row) →
    RuntimeValue.orderList rank ((rowCodec row).encode left) ((rowCodec row).encode right) =
      HList.order rank row left right
  | .nil, _, _ => RuntimeValue.orderList_nil_nil rank
  | .cons τ rest, left, right => by
      simp only [rowCodec, Codec.tupleCons, RuntimeValue.orderList_cons_cons, HList.order]
      rw [NTy.order_codec rank τ left.1 right.1, HList.order_codec rank rest left.2 right.2]

/-- Variant values order as their encodings do. -/
theorem variantCarrier.order_codec (rank : ValueRanks) (source : StructHandle) :
    (names : List String) → (rows : NRows) → names.Nodup → (left right : variantCarrier names rows) →
    RuntimeValue.order rank (variantEncode source names rows (rowCodecs rows) left)
        (variantEncode source names rows (rowCodecs rows) right) =
      variantCarrier.order rank source names rows left right
  | _, .nil, _, left, _ => nomatch left
  | [], .cons _ _, _, left, _ => nomatch left
  | _ :: _, .cons fields _, _, .inl left, .inl right => by
      simp only [variantEncode, Sum.elim_inl, Sum.elim_inr, rowCodecs, Codec.nominalRow, RuntimeValue.order_nominal, then_compare_self_nat, then_compare_self_string, compareVariant, variantCarrier.order]
      exact HList.order_codec rank fields left right
  | name :: names, .cons _ rest, distinct, .inl left, .inr right => by
      obtain ⟨fields, encoded⟩ := variantEncode_nominal source names rest (rowCodecs rest) right
      have different : name ≠ variantName names rest right := fun same =>
        (List.nodup_cons.mp distinct).1 (same ▸ variantName_mem names rest right)
      simp only [variantEncode, Sum.elim_inl, Sum.elim_inr, rowCodecs, Codec.nominalRow, encoded, RuntimeValue.order_nominal, then_compare_self_nat, compareVariant_then_of_ne rank source different, variantCarrier.order]
  | name :: names, .cons _ rest, distinct, .inr left, .inl right => by
      obtain ⟨fields, encoded⟩ := variantEncode_nominal source names rest (rowCodecs rest) left
      have different : variantName names rest left ≠ name := fun same =>
        (List.nodup_cons.mp distinct).1 (same ▸ variantName_mem names rest left)
      simp only [variantEncode, Sum.elim_inl, Sum.elim_inr, rowCodecs, Codec.nominalRow, encoded, RuntimeValue.order_nominal, then_compare_self_nat, compareVariant_then_of_ne rank source different, variantCarrier.order]
  | _ :: names, .cons _ rest, distinct, .inr left, .inr right => by
      simp only [variantEncode, Sum.elim_inl, Sum.elim_inr, rowCodecs, variantCarrier.order]
      exact variantCarrier.order_codec rank source names rest (List.nodup_cons.mp distinct).2 left right
end

/-- Native values order as their encodings do. -/
theorem NTy.order_encode (rank : ValueRanks) (τ : NTy) (left right : τ.carrier) :
    RuntimeValue.order rank (τ.encode left) (τ.encode right) = NTy.order rank τ left right :=
  NTy.order_codec rank τ left right

/-- The structural order of a native type's values, as an ordered map's
keys compare. -/
def NTy.keyOrder (rank : ValueRanks) (τ : NTy) : SpecMap.KeyOrder τ.carrier where
  compare := NTy.order rank τ
  lawful := {
    compare_self := fun {a} => by rw [← NTy.order_encode]; exact RuntimeValue.order_self _ _
    eq_of_compare := fun {a b} h => by
      rw [← NTy.order_encode] at h
      exact τ.codec.encode_injective (RuntimeValue.eq_of_order h) }
  trans := {
    eq_swap := fun {a b} => by
      rw [← NTy.order_encode, ← NTy.order_encode]; exact RuntimeValue.order_swap _ _ _
    isLE_trans := fun {a b c} h₁ h₂ => by
      rw [← NTy.order_encode] at h₁ h₂ ⊢
      exact RuntimeValue.order_isLE_trans _ _ _ _ h₁ h₂ }

@[simp, vir_denote_norm] theorem NTy.keyOrder_compare (rank : ValueRanks) (τ : NTy) :
    (NTy.keyOrder rank τ).compare = NTy.order rank τ := rfl

/-- A type's order as a family's order of a type parameter. -/
@[reducible] def NTy.typedOrder (τ : NTy) : TypedOrder τ.carrier τ.codec.encode where
  order := fun rank => NTy.order rank τ
  order_encode := fun rank => NTy.order_codec rank τ

/-- A type's order of lent values as a family's order of a type parameter's
(`Carriers.lent`). -/
@[reducible] def NTy.plainTypedOrder (τ : NTy) : TypedOrder τ.plain τ.plainCodec.encode where
  order := fun rank => NTy.plainOrder rank τ
  order_encode := fun rank => NTy.plainOrder_codec rank τ


/-- A struct's certificate: its declaration's data invariant of its
encoding, and its fields' certificates. -/
theorem NTy.deepHolds_struct (source : StructHandle) (arguments fields : NRow)
    (row : HListPlain fields) :
    NTy.deepHolds (.struct source arguments fields) row ↔
      (Carriers.own source arguments (NTy.plainEncode (.struct source arguments fields) row) ∧
        HListPlain.deepHolds fields row) := Iff.rfl

/-- The row of a certified struct value satisfies the data invariants: that
is what the value's certificate says.  A borrow of a global, of a table
entry or of a certified local hands the reference this row as its current
(`NTy.forget`), so the invariant owed where the reference is handed to a
call, or where the loan dies with the row unchanged, is the borrowed value's
own (`doc/designs/data-invariants.md`).  Stated of the row, not of
`NTy.forget`, because the normal form removes the forgetting first
(`NTy.forget_struct`), and ahead of `NTy.deepHolds_struct`, which would
spell the clauses of a row that already carries them. -/
@[simp high, vir_denote_norm high] theorem NTy.deepHolds_struct_val (source : StructHandle)
    (arguments fields : NRow) (value : (NTy.struct source arguments fields).carrier) :
    NTy.deepHolds (.struct source arguments fields) value.val = True :=
  eq_true value.property

/-- The value of a certified variant satisfies the data invariants: that is
what its certificate says (`NTy.deepHolds_struct_val`). -/
@[simp high, vir_denote_norm high] theorem NTy.deepHolds_enum_val (source : StructHandle)
    (arguments : NRow) (names : List String) (rows : NRows) (distinct : names.Nodup)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    NTy.deepHolds (.enum source arguments names rows distinct) value.val = True :=
  eq_true value.property

/-- A reference's certificate: its prophecy's, the current being open while
it is lent (`doc/designs/data-invariants.md`). -/
@[simp, vir_denote_norm] theorem NTy.deepHolds_ref (referent : NTy)
    (value : (NTy.ref referent).plain) :
    NTy.deepHolds (.ref referent) value = NTy.deepHolds referent value.2 := rfl

/-- A variant value's certificate: its declaration's data invariant of its
encoding, and its fields' certificates. -/
theorem NTy.deepHolds_enum (source : StructHandle) (arguments : NRow) (names : List String)
    (rows : NRows) (distinct : names.Nodup) (value : variantPlain names rows) :
    NTy.deepHolds (.enum source arguments names rows distinct) value ↔
      (Carriers.own source arguments
          (NTy.plainEncode (.enum source arguments names rows distinct) value) ∧
        variantPlain.deepHolds names rows value) := Iff.rfl

/-! ### Certificates, constructor by constructor

A value's certificate is the conjunction of the clauses of the declarations
it reaches: at a type with no declaration under it there is nothing to
state.  Registered as the solver's normal form, so that a certificate of a
spelled-out type becomes the clauses of its parts
(`doc/designs/data-invariants.md`). -/

attribute [vir_denote_norm] NTy.deepHolds_struct NTy.deepHolds_enum

@[vir_denote_norm] theorem NTy.deepHolds_unit (value : (NTy.unit).plain) :
    NTy.deepHolds .unit value = True := rfl
@[vir_denote_norm] theorem NTy.deepHolds_bool (value : (NTy.bool).plain) :
    NTy.deepHolds .bool value = True := rfl
@[vir_denote_norm] theorem NTy.deepHolds_int (width : Nat) (signed : Bool)
    (value : (NTy.int width signed).plain) :
    NTy.deepHolds (.int width signed) value = True := rfl
@[vir_denote_norm] theorem NTy.deepHolds_num (value : (NTy.num).plain) :
    NTy.deepHolds .num value = True := rfl
@[vir_denote_norm] theorem NTy.deepHolds_address (value : (NTy.address).plain) :
    NTy.deepHolds .address value = True := rfl
@[vir_denote_norm] theorem NTy.deepHolds_signer (value : (NTy.signer).plain) :
    NTy.deepHolds .signer value = True := rfl
@[vir_denote_norm] theorem NTy.deepHolds_string (value : (NTy.string).plain) :
    NTy.deepHolds .string value = True := rfl
@[vir_denote_norm] theorem NTy.deepHolds_bytes (value : (NTy.bytes).plain) :
    NTy.deepHolds .bytes value = True := rfl
@[vir_denote_norm] theorem NTy.deepHolds_function (parameters : NRow) (shared : Sharing)
    (results : NRow) (value : (NTy.function parameters shared results).plain) :
    NTy.deepHolds (.function parameters shared results) value = True := rfl
@[vir_denote_norm] theorem NTy.deepHolds_tuple (elements : NRow)
    (values : (NTy.tuple elements).plain) :
    NTy.deepHolds (.tuple elements) values = HListPlain.deepHolds elements values := rfl
@[vir_denote_norm] theorem NTy.deepHolds_vector (element : NTy)
    (vector : (NTy.vector element).plain) :
    NTy.deepHolds (.vector element) vector =
      ∀ value ∈ vector.values, NTy.deepHolds element value := rfl
@[vir_denote_norm] theorem NTy.deepHolds_slice (element : NTy)
    (vector : (NTy.slice element).plain) :
    NTy.deepHolds (.slice element) vector =
      ∀ value ∈ vector.values, NTy.deepHolds element value := rfl
@[vir_denote_norm] theorem NTy.deepHolds_array (element : NTy) (length : Nat)
    (array : (NTy.array element length).plain) :
    NTy.deepHolds (.array element length) array =
      ∀ value ∈ array.vector.values, NTy.deepHolds element value := rfl
@[vir_denote_norm] theorem NTy.deepHolds_sliceRef (element : NTy)
    (value : (NTy.sliceRef element).plain) :
    NTy.deepHolds (.sliceRef element) value =
      ((∀ current ∈ value.currentVector.values, NTy.deepHolds element current) ∧
        (∀ final ∈ value.finalVector.values, NTy.deepHolds element final)) := rfl
@[vir_denote_norm] theorem NTy.deepHolds_table (key value : NTy)
    (entries : (NTy.table key value).plain) :
    NTy.deepHolds (.table key value) entries =
      ∀ entry ∈ entries.entries,
        NTy.deepHolds key entry.1 ∧ NTy.deepHolds value entry.2 := rfl
@[vir_denote_norm] theorem NTy.deepHolds_ghost (slot : NTy) (value : (NTy.ghost slot).plain) :
    NTy.deepHolds (.ghost slot) value = NTy.deepHolds slot value := rfl
@[vir_denote_norm] theorem NTy.deepHolds_param (index : Nat) (value : (NTy.param index).plain) :
    NTy.deepHolds (.param index) value = (Carriers.lent index).holds value := rfl

@[vir_denote_norm] theorem HListPlain.deepHolds_nil (values : HListPlain .nil) :
    HListPlain.deepHolds .nil values = True := rfl
@[vir_denote_norm] theorem HListPlain.deepHolds_cons (τ : NTy) (rest : NRow)
    (values : HListPlain (.cons τ rest)) :
    HListPlain.deepHolds (.cons τ rest) values =
      (NTy.deepHolds τ values.1 ∧ HListPlain.deepHolds rest values.2) := rfl

@[vir_denote_norm] theorem variantPlain.deepHolds_inl (name : String) (names : List String)
    (fields : NRow) (rest : NRows) (values : HListPlain fields) :
    variantPlain.deepHolds (name :: names) (.cons fields rest) (.inl values) =
      HListPlain.deepHolds fields values := rfl
@[vir_denote_norm] theorem variantPlain.deepHolds_inr (name : String) (names : List String)
    (fields : NRow) (rest : NRows) (value : variantPlain names rest) :
    variantPlain.deepHolds (name :: names) (.cons fields rest) (.inr value) =
      variantPlain.deepHolds names rest value := rfl


mutual
/-- A certified value satisfies the data invariants once its certificates
are forgotten: they are what its certificate says.

In the normal form, at every type at once: it is the step by which the cut
at a hand-over of a `&mut` whose current is a borrowed value, and the cut
at a mutation end whose root was rebuilt from certified parts, reach what
the typing of storage gives (`StorageEncodes`).  At a table and a vector it
reads the collection entrywise, which no case lemma does, and
`NTy.forget_table` is kept out of the normal form so that the forgetting of
a table is still here to be read (`SpecMap.ofEntries_forget_table`).  At a
struct or a variant the normal form removes the forgetting first
(`NTy.forget_struct`), where `NTy.deepHolds_struct_val` states the same
step of the row. -/
@[simp, vir_denote_norm] theorem NTy.deepHolds_forget : (τ : NTy) → (value : τ.carrier) →
    NTy.deepHolds τ (NTy.forget τ value)
  | .unit, _ => trivial
  | .bool, _ => trivial
  | .int _ _, _ => trivial
  | .num, _ => trivial
  | .address, _ => trivial
  | .signer, _ => trivial
  | .string, _ => trivial
  | .bytes, _ => trivial
  | .char, _ => trivial
  | .function _ _ _, _ => trivial
  | .nominal _ _ _, _ => trivial
  | .param index, value => (Carriers.lent index).holds_of value
  | .ref referent, value => NTy.deepHolds_forget referent value.2
  | .observed referent, value => NTy.deepHolds_forget referent value
  | .tuple elements, values => HList.deepHolds_forget elements values
  | .struct _ _ _, value => value.property
  | .enum _ _ _ _ _, value => value.property
  | .vector element, vector => by
      intro value member
      simp only [NTy.forget, SpecVector.values, Array.mem_map] at member
      obtain ⟨source, _, equal⟩ := member
      exact equal ▸ NTy.deepHolds_forget element source
  | .slice element, vector => by
      intro value member
      simp only [NTy.forget, SpecVector.values, Array.mem_map] at member
      obtain ⟨source, _, equal⟩ := member
      exact equal ▸ NTy.deepHolds_forget element source
  | .array element _, array => by
      intro value member
      simp only [NTy.forget, SpecVector.values, Array.mem_map] at member
      obtain ⟨source, _, equal⟩ := member
      exact equal ▸ NTy.deepHolds_forget element source
  | .sliceRef element, value =>
      ⟨by
        intro current member
        simp only [NTy.forget, SpecSliceRef.map_currentVector, SpecVector.map_values,
          Array.mem_map] at member
        obtain ⟨source, _, equal⟩ := member
        exact equal ▸ NTy.deepHolds_forget element source,
      by
        intro final member
        simp only [NTy.forget, SpecSliceRef.map_finalVector, SpecVector.map_values,
          Array.mem_map] at member
        obtain ⟨source, _, equal⟩ := member
        exact equal ▸ NTy.deepHolds_forget element source⟩
  | .table key value, entries => by
      intro entry member
      simp only [NTy.forget, SpecEntries.entries, List.mem_map] at member
      obtain ⟨source, _, equal⟩ := member
      exact equal ▸ ⟨NTy.deepHolds_forget key source.1, NTy.deepHolds_forget value source.2⟩
  | .ghost slot, value => NTy.deepHolds_forget slot value

/-- A row of certified values satisfies the data invariants once its
certificates are forgotten (`NTy.deepHolds_forget`, in the normal form with
it, so that a row crosses the same step as one value). -/
@[simp, vir_denote_norm] theorem HList.deepHolds_forget : (row : NRow) → (values : HList row) →
    HListPlain.deepHolds row (HList.forget row values)
  | .nil, _ => trivial
  | .cons τ rest, values =>
      ⟨NTy.deepHolds_forget τ values.1, HList.deepHolds_forget rest values.2⟩
end

mutual
/-- A lent value that satisfies the data invariants as a certified one: a
struct's or a variant's row with the certificate it is given, the rest
structural, and a type parameter's by its family (`ParamPlain.certify`).
Total, so a value read from anywhere is certified by what it satisfies and
a read owes nothing (`doc/designs/data-invariants.md`). -/
def NTy.ofDeepHolds : (τ : NTy) → (value : τ.plain) → NTy.deepHolds τ value → τ.carrier
  | .unit, value, _ => value
  | .bool, value, _ => value
  | .int _ _, value, _ => value
  | .num, value, _ => value
  | .address, value, _ => value
  | .signer, value, _ => value
  | .string, value, _ => value
  | .bytes, value, _ => value
  | .char, value, _ => value
  | .function _ _ _, value, _ => value
  | .nominal _ _ _, value, _ => value
  | .tuple elements, values, holds => HListPlain.ofDeepHolds elements values holds
  | .struct _ _ _, row, holds => ⟨row, holds⟩
  | .enum _ _ _ _ _, value, holds => ⟨value, holds⟩
  | .vector element, vector, holds =>
      ⟨vector.values.attach.map fun value =>
          NTy.ofDeepHolds element value.val (holds value.val value.property),
        by simpa using vector.bounded⟩
  | .slice element, vector, holds =>
      ⟨vector.values.attach.map fun value =>
          NTy.ofDeepHolds element value.val (holds value.val value.property),
        by simpa using vector.bounded⟩
  | .array element _, array, holds =>
      ⟨⟨array.vector.values.attach.map fun value =>
            NTy.ofDeepHolds element value.val (holds value.val value.property),
          by simpa using array.vector.bounded⟩,
        by simpa using array.size_eq⟩
  | .table key value, entries, holds =>
      ⟨entries.entries.attach.map fun entry =>
          (NTy.ofDeepHolds key entry.val.1 (holds entry.val entry.property).1,
            NTy.ofDeepHolds value entry.val.2 (holds entry.val entry.property).2),
        by simpa using entries.bounded⟩
  | .ghost slot, value, holds => NTy.ofDeepHolds slot value holds
  | .ref referent, value, holds => (value.1, NTy.ofDeepHolds referent value.2 holds)
  | .sliceRef element, value, holds =>
      { extent := value.extent
        current :=
          ⟨⟨value.currentVector.values.attach.map fun current =>
                NTy.ofDeepHolds element current.val (holds.1 current.val current.property),
              by simpa using value.currentVector.bounded⟩,
            by simp⟩
        final :=
          ⟨⟨value.finalVector.values.attach.map fun final =>
                NTy.ofDeepHolds element final.val (holds.2 final.val final.property),
              by simpa using value.finalVector.bounded⟩,
            by simp⟩ }
  | .observed referent, value, holds => NTy.ofDeepHolds referent value holds
  | .param index, value, holds => (Carriers.lent index).certify value holds
termination_by structural τ _ _ => τ

/-- A row of lent values that satisfy the data invariants as certified ones. -/
def HListPlain.ofDeepHolds : (row : NRow) → (values : HListPlain row) →
    HListPlain.deepHolds row values → HList row
  | .nil, _, _ => ()
  | .cons τ rest, values, holds =>
      (NTy.ofDeepHolds τ values.1 holds.1, HListPlain.ofDeepHolds rest values.2 holds.2)
termination_by structural row _ _ => row
end

mutual
/-- Certifying a lent value and forgetting the certificates is the value. -/
theorem NTy.forget_ofDeepHolds : (τ : NTy) → (value : τ.plain) →
    (holds : NTy.deepHolds τ value) → NTy.forget τ (NTy.ofDeepHolds τ value holds) = value
  | .unit, _, _ => rfl
  | .bool, _, _ => rfl
  | .int _ _, _, _ => rfl
  | .num, _, _ => rfl
  | .address, _, _ => rfl
  | .signer, _, _ => rfl
  | .string, _, _ => rfl
  | .bytes, _, _ => rfl
  | .char, _, _ => rfl
  | .function _ _ _, _, _ => rfl
  | .nominal _ _ _, _, _ => rfl
  | .struct _ _ _, _, _ => rfl
  | .enum _ _ _ _ _, _, _ => rfl
  | .tuple elements, values, holds => HListPlain.forget_ofDeepHolds elements values holds
  | .ghost slot, value, holds => NTy.forget_ofDeepHolds slot value holds
  | .observed referent, value, holds => NTy.forget_ofDeepHolds referent value holds
  | .ref referent, value, holds => by
      simp only [NTy.ofDeepHolds, NTy.forget, NTy.forget_ofDeepHolds referent value.2 holds]
  | .param index, value, holds => by
      simp only [NTy.ofDeepHolds, NTy.forget, (Carriers.lent index).of_certify value holds]
  | .vector element, vector, holds => by
      apply SpecVector.ext
      simp only [NTy.ofDeepHolds, NTy.forget, Array.map_map]
      rw [show ((NTy.forget element) ∘ fun value : { x // x ∈ vector.values } =>
            NTy.ofDeepHolds element value.val (holds value.val value.property)) =
          fun value => value.val from
        funext fun value =>
          NTy.forget_ofDeepHolds element value.val (holds value.val value.property)]
      simp [Array.attach_map_subtype_val]
  | .slice element, vector, holds => by
      apply SpecVector.ext
      simp only [NTy.ofDeepHolds, NTy.forget, Array.map_map]
      rw [show ((NTy.forget element) ∘ fun value : { x // x ∈ vector.values } =>
            NTy.ofDeepHolds element value.val (holds value.val value.property)) =
          fun value => value.val from
        funext fun value =>
          NTy.forget_ofDeepHolds element value.val (holds value.val value.property)]
      simp [Array.attach_map_subtype_val]
  | .array element _, array, holds => by
      apply SpecArray.ext
      apply SpecVector.ext
      simp only [NTy.ofDeepHolds, NTy.forget, Array.map_map]
      rw [show ((NTy.forget element) ∘ fun value : { x // x ∈ array.vector.values } =>
            NTy.ofDeepHolds element value.val (holds value.val value.property)) =
          fun value => value.val from
        funext fun value =>
          NTy.forget_ofDeepHolds element value.val (holds value.val value.property)]
      simp [Array.attach_map_subtype_val]
  | .sliceRef element, value, holds => by
      -- Both readings of a borrowed extent are certified, so each is the
      -- reading of a vector, and `SpecSliceRef.ext` splits the two.
      have step : ∀ (values : Array element.plain)
          (certify : ∀ source ∈ values, NTy.deepHolds element source),
          (values.attach.map fun source =>
              NTy.ofDeepHolds element source.val (certify source.val source.property)).map
            (NTy.forget element) = values := by
        intro values certify
        rw [Array.map_map]
        rw [show ((NTy.forget element) ∘ fun source : { x // x ∈ values } =>
              NTy.ofDeepHolds element source.val (certify source.val source.property)) =
            fun source => source.val from
          funext fun source =>
            NTy.forget_ofDeepHolds element source.val (certify source.val source.property)]
        simp [Array.attach_map_subtype_val]
      apply SpecSliceRef.ext
      · exact SpecVector.ext (step value.currentVector.values holds.1)
      · exact SpecVector.ext (step value.finalVector.values holds.2)
  | .table key value, entries, holds => by
      apply SpecEntries.ext
      simp only [NTy.ofDeepHolds, NTy.forget, List.map_map]
      rw [show ((fun entry => (NTy.forget key entry.1, NTy.forget value entry.2)) ∘
            fun entry : { x // x ∈ entries.entries } =>
              (NTy.ofDeepHolds key entry.val.1 (holds entry.val entry.property).1,
                NTy.ofDeepHolds value entry.val.2 (holds entry.val entry.property).2)) =
          fun entry => entry.val from
        funext fun entry => by
          simp only [Function.comp_apply,
            NTy.forget_ofDeepHolds key entry.val.1 (holds entry.val entry.property).1,
            NTy.forget_ofDeepHolds value entry.val.2 (holds entry.val entry.property).2]]
      simp [List.attach_map_subtype_val]

/-- Certifying a row and forgetting the certificates is the row. -/
theorem HListPlain.forget_ofDeepHolds : (row : NRow) → (values : HListPlain row) →
    (holds : HListPlain.deepHolds row values) →
    HList.forget row (HListPlain.ofDeepHolds row values holds) = values
  | .nil, values, _ => by cases values; rfl
  | .cons τ rest, values, holds => by
      simp only [HListPlain.ofDeepHolds, HList.forget,
        NTy.forget_ofDeepHolds τ values.1 holds.1,
        HListPlain.forget_ofDeepHolds rest values.2 holds.2]
      rfl
end

/-- A certified struct is the row it certifies: every operation on it reads
the row (`NTy.encode_struct`). -/
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_struct_val (source : StructHandle)
    (arguments fields : NRow) (row : HListPlain fields)
    (holds : NTy.deepHolds (.struct source arguments fields) row) :
    (NTy.ofDeepHolds (.struct source arguments fields) row holds).val = row := rfl

/-- And a certified variant value is the variant value it certifies. -/
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_enum_val (source : StructHandle)
    (arguments : NRow) (names : List String) (rows : NRows) (distinct : names.Nodup)
    (value : variantPlain names rows)
    (holds : NTy.deepHolds (.enum source arguments names rows distinct) value) :
    (NTy.ofDeepHolds (.enum source arguments names rows distinct) value holds).val = value := rfl

/-- A type parameter's value certified from its lent form is the value, and
a lent value certified and forgotten is the lent value: the family's own
round trips (`ParamLent`), as the solver's normal form. -/
@[simp, vir_denote_norm] theorem Carriers.lent_certify_of (index : Nat)
    (value : Carriers.carrier index) (held : (Carriers.lent index).holds
      ((Carriers.lent index).of value)) :
    (Carriers.lent index).certify ((Carriers.lent index).of value) held = value :=
  (Carriers.lent index).certify_of value

@[simp, vir_denote_norm] theorem Carriers.lent_of_certify (index : Nat)
    (value : (Carriers.lent index).carrier) (held : (Carriers.lent index).holds value) :
    (Carriers.lent index).of ((Carriers.lent index).certify value held) = value :=
  (Carriers.lent index).of_certify value held

/-- And a type parameter's certified value satisfies the parameter's data
invariants in its lent form (`ParamPlain.holds_of`).  `NTy.deepHolds_forget`
states this of every type, but at a parameter its statement is gone by the
time a leaf reads it: the normal form has already taken both maps through
the family (`NTy.deepHolds_param`, `NTy.forget_param`), so the fact is
needed in the form they leave. -/
@[simp, vir_denote_norm] theorem Carriers.lent_holds_of (index : Nat)
    (value : Carriers.carrier index) :
    (Carriers.lent index).holds ((Carriers.lent index).of value) :=
  (Carriers.lent index).holds_of value

/-! ### Certifying and forgetting, constructor by constructor

At a type with no declaration under it a value is its own certified and its
own lent form, so a read through either map is the read of the value.
Registered as the solver's normal form
(`doc/designs/data-invariants.md`). -/

section Bridges
variable (τ : NTy)

@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_unit (value : (NTy.unit).plain) (holds) :
    NTy.ofDeepHolds .unit value holds = value := rfl
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_bool (value : (NTy.bool).plain) (holds) :
    NTy.ofDeepHolds .bool value holds = value := rfl
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_int (width : Nat) (signed : Bool)
    (value : (NTy.int width signed).plain) (holds) :
    NTy.ofDeepHolds (.int width signed) value holds = value := rfl
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_num (value : (NTy.num).plain) (holds) :
    NTy.ofDeepHolds .num value holds = value := rfl
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_address (value : (NTy.address).plain) (holds) :
    NTy.ofDeepHolds .address value holds = value := rfl
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_signer (value : (NTy.signer).plain) (holds) :
    NTy.ofDeepHolds .signer value holds = value := rfl
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_string (value : (NTy.string).plain) (holds) :
    NTy.ofDeepHolds .string value holds = value := rfl
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_bytes (value : (NTy.bytes).plain) (holds) :
    NTy.ofDeepHolds .bytes value holds = value := rfl
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_function (parameters : NRow) (shared : Sharing)
    (results : NRow) (value : (NTy.function parameters shared results).plain) (holds) :
    NTy.ofDeepHolds (.function parameters shared results) value holds = value := rfl
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_tuple (elements : NRow)
    (values : (NTy.tuple elements).plain) (holds) :
    NTy.ofDeepHolds (.tuple elements) values holds =
      HListPlain.ofDeepHolds elements values holds := rfl
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_ghost (slot : NTy)
    (value : (NTy.ghost slot).plain) (holds) :
    NTy.ofDeepHolds (.ghost slot) value holds = NTy.ofDeepHolds slot value holds := rfl
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_ref (referent : NTy)
    (value : (NTy.ref referent).plain) (holds) :
    NTy.ofDeepHolds (.ref referent) value holds =
      (value.1, NTy.ofDeepHolds referent value.2 holds) := rfl
@[simp, vir_denote_norm] theorem NTy.ofDeepHolds_param (index : Nat)
    (value : (NTy.param index).plain) (holds) :
    NTy.ofDeepHolds (.param index) value holds = (Carriers.lent index).certify value holds := rfl
@[simp, vir_denote_norm] theorem HListPlain.ofDeepHolds_nil (values : HListPlain .nil) (holds) :
    HListPlain.ofDeepHolds .nil values holds = () := rfl
@[simp, vir_denote_norm] theorem HListPlain.ofDeepHolds_cons (head : NTy) (rest : NRow)
    (values : HListPlain (.cons head rest)) (holds) :
    HListPlain.ofDeepHolds (.cons head rest) values holds =
      (NTy.ofDeepHolds head values.1 holds.1,
        HListPlain.ofDeepHolds rest values.2 holds.2) := rfl

@[simp, vir_denote_norm] theorem NTy.forget_unit (value : (NTy.unit).carrier) :
    NTy.forget .unit value = value := rfl
@[simp, vir_denote_norm] theorem NTy.forget_bool (value : (NTy.bool).carrier) :
    NTy.forget .bool value = value := rfl
@[simp, vir_denote_norm] theorem NTy.forget_int (width : Nat) (signed : Bool)
    (value : (NTy.int width signed).carrier) :
    NTy.forget (.int width signed) value = value := rfl
@[simp, vir_denote_norm] theorem NTy.forget_num (value : (NTy.num).carrier) :
    NTy.forget .num value = value := rfl
@[simp, vir_denote_norm] theorem NTy.forget_address (value : (NTy.address).carrier) :
    NTy.forget .address value = value := rfl
@[simp, vir_denote_norm] theorem NTy.forget_signer (value : (NTy.signer).carrier) :
    NTy.forget .signer value = value := rfl
@[simp, vir_denote_norm] theorem NTy.forget_string (value : (NTy.string).carrier) :
    NTy.forget .string value = value := rfl
@[simp, vir_denote_norm] theorem NTy.forget_bytes (value : (NTy.bytes).carrier) :
    NTy.forget .bytes value = value := rfl
@[simp, vir_denote_norm] theorem NTy.forget_function (parameters : NRow) (shared : Sharing)
    (results : NRow) (value : (NTy.function parameters shared results).carrier) :
    NTy.forget (.function parameters shared results) value = value := rfl
@[simp, vir_denote_norm] theorem NTy.forget_tuple (elements : NRow)
    (values : (NTy.tuple elements).carrier) :
    NTy.forget (.tuple elements) values = HList.forget elements values := rfl
@[simp, vir_denote_norm] theorem NTy.forget_struct (source : StructHandle)
    (arguments fields : NRow) (value : (NTy.struct source arguments fields).carrier) :
    NTy.forget (.struct source arguments fields) value = value.val := rfl
@[simp, vir_denote_norm] theorem NTy.forget_enum (source : StructHandle) (arguments : NRow)
    (names : List String) (rows : NRows) (distinct : names.Nodup)
    (value : (NTy.enum source arguments names rows distinct).carrier) :
    NTy.forget (.enum source arguments names rows distinct) value = value.val := rfl
/-- Keep a forgotten vector whole until an operation reads its entries. -/
theorem NTy.forget_vector (element : NTy)
    (values : (NTy.vector element).carrier) :
    NTy.forget (.vector element) values =
      ⟨values.values.map (NTy.forget element), by simpa using values.bounded⟩ := rfl
@[simp, vir_denote_norm] theorem NTy.forget_vector_values (element : NTy)
    (values : (NTy.vector element).carrier) :
    (NTy.forget (.vector element) values).values = values.values.map (NTy.forget element) := rfl
@[simp, vir_denote_norm] theorem NTy.forget_char (value : (NTy.char).carrier) :
    NTy.forget .char value = value := rfl
/-- A forgotten slice's entries, as a forgotten vector's (`NTy.forget_vector_values`). -/
@[simp, vir_denote_norm] theorem NTy.forget_slice_values (element : NTy)
    (values : (NTy.slice element).carrier) :
    (NTy.forget (.slice element) values).values = values.values.map (NTy.forget element) := rfl
/-- A forgotten array's entries, as a forgotten vector's (`NTy.forget_vector_values`). -/
@[simp, vir_denote_norm] theorem NTy.forget_array_values (element : NTy) {length : Nat}
    (array : (NTy.array element length).carrier) :
    (NTy.forget (.array element length) array).vector.values =
      array.vector.values.map (NTy.forget element) := rfl
@[simp, vir_denote_norm] theorem NTy.forget_observed (referent : NTy)
    (value : (NTy.observed referent).carrier) :
    NTy.forget (.observed referent) value = NTy.forget referent value := rfl
@[simp, vir_denote_norm] theorem NTy.forget_nominal (source : StructHandle) (arguments : NRow)
    (productive : Bool) (value : (NTy.nominal source arguments productive).carrier) :
    NTy.forget (.nominal source arguments productive) value = value := rfl

/-- A forgotten table is its entries forgotten.  Deliberately in no
simp set: the normal form reads a forgotten table
through `SpecMap.ofEntries_forget_table`, whose left side holds this
equation's left side, and a child is rewritten before its parent, so this
equation in a simp set would pre-empt that one and leave the two readings
of one table in two shapes. -/
theorem NTy.forget_table (key value : NTy)
    (entries : (NTy.table key value).carrier) :
    NTy.forget (.table key value) entries =
      ⟨entries.entries.map fun entry => (NTy.forget key entry.1, NTy.forget value entry.2),
        by simpa using entries.bounded⟩ := rfl
@[simp, vir_denote_norm] theorem NTy.forget_ghost (slot : NTy)
    (value : (NTy.ghost slot).carrier) :
    NTy.forget (.ghost slot) value = NTy.forget slot value := rfl
@[simp, vir_denote_norm] theorem NTy.forget_ref (referent : NTy)
    (value : (NTy.ref referent).carrier) :
    NTy.forget (.ref referent) value = (value.1, NTy.forget referent value.2) := rfl
@[simp, vir_denote_norm] theorem NTy.forget_param (index : Nat)
    (value : (NTy.param index).carrier) :
    NTy.forget (.param index) value = (Carriers.lent index).of value := rfl
@[simp, vir_denote_norm] theorem HList.forget_nil (values : HList .nil) :
    HList.forget .nil values = () := rfl
@[simp, vir_denote_norm] theorem HList.forget_cons (head : NTy) (rest : NRow)
    (values : HList (.cons head rest)) :
    HList.forget (.cons head rest) values =
      (NTy.forget head values.1, HList.forget rest values.2) := rfl

end Bridges

/-- A certified value is determined by the lent value it certifies. -/
theorem NTy.forget_injective (τ : NTy) {left right : τ.carrier}
    (equal : τ.forget left = τ.forget right) : left = right := by
  apply NTy.encode_injective τ
  rw [← NTy.plainEncode_forget, ← NTy.plainEncode_forget, equal]

/-- A row of certified values is determined by the row of lent values it
certifies: entry by entry, each value by the one it certifies
(`NTy.forget_injective`). -/
theorem HList.forget_injective : (row : NRow) → ∀ {left right : HList row},
    HList.forget row left = HList.forget row right → left = right
  | .nil, _, _, _ => rfl
  | .cons τ rest, _, _, equal =>
      Prod.ext (NTy.forget_injective τ (congrArg Prod.fst equal))
        (HList.forget_injective rest (congrArg Prod.snd equal))

/-- Forgetting a certified value and certifying it again is the value. -/
theorem NTy.ofDeepHolds_forget (τ : NTy) (value : τ.carrier) :
    NTy.ofDeepHolds τ (τ.forget value) (τ.deepHolds_forget value) = value :=
  NTy.forget_injective τ (NTy.forget_ofDeepHolds τ (τ.forget value) (τ.deepHolds_forget value))

mutual
/-- A plain value as a clause reads it: each reference settled at its current
value, or with `prophecies` at its prophecy.  The plain pair of a reference is
homogeneous, so settling needs no certificate (`NTy.plain`). -/
def NTy.plainViewed (prophecies : Bool) : (τ : NTy) → τ.plain → τ.plain
  | .sliceRef _, value =>
      SpecSliceRef.settled (if prophecies then value.finalVector else value.currentVector)
  | .ref σ, value =>
      let seen := σ.plainViewed prophecies (if prophecies then value.2 else value.1)
      (seen, seen)
  | .tuple elements, values => HListPlain.plainViewed prophecies elements values
  | .struct _ _ fields, values => HListPlain.plainViewed prophecies fields values
  | .enum _ _ names rows _, value => variantPlainViewed prophecies names rows value
  | .unit, value | .bool, value | .int _ _, value | .num, value | .address, value | .signer, value
  | .string, value | .bytes, value | .char, value | .slice _, value | .vector _, value
  | .array _ _, value | .table _ _, value | .ghost _, value | .param _, value
  | .function _ _ _, value | .nominal _ _ _, value => value
  | .observed referent, value => NTy.plainViewed prophecies referent value

/-- An enum's plain value with its active variant read as a clause reads it. -/
def variantPlainViewed (prophecies : Bool) : (names : List String) → (rows : NRows) →
    variantPlain names rows → variantPlain names rows
  | _, .nil, value => value
  | [], .cons _ _, value => value
  | _ :: _, .cons fields _, .inl values => .inl (HListPlain.plainViewed prophecies fields values)
  | _ :: names, .cons _ rest, .inr later => .inr (variantPlainViewed prophecies names rest later)

/-- A plain row of values as a clause reads it. -/
def HListPlain.plainViewed (prophecies : Bool) : (row : NRow) → HListPlain row → HListPlain row
  | .nil, values => values
  | .cons τ rest, values =>
      (τ.plainViewed prophecies values.1, HListPlain.plainViewed prophecies rest values.2)
end

mutual
/-- A value as a clause reads it at entry, or with `prophecies` at exit: each
reference it holds as one whose current value is its current value, or its
prophecy, and so its referent's.  A reference's current is plain and its
prophecy certified (`NTy.carrier`), so the exit view settles the pair at the
prophecy; the entry view settles the plain current and keeps the certified
side, which only the exit reading consults. -/
def NTy.viewed (prophecies : Bool) : (τ : NTy) → τ.carrier → τ.carrier
  | .sliceRef _, value =>
      SpecSliceRef.settled (if prophecies then value.finalVector else value.currentVector)
  | .ref σ, value =>
      if prophecies then
        let seen := σ.viewed true value.2
        (σ.forget seen, seen)
      else
        (σ.plainViewed false value.1, σ.viewed false value.2)
  | .tuple elements, values => HList.viewed prophecies elements values
  -- A certified aggregate is viewed at the plain layer (`NTy.plainViewed`),
  -- where the clause reads its row; settling a reference inside the
  -- certificate would owe a new one (`NTy.deepHolds`), which no view does.
  | .struct _ _ _, value => value
  | .enum _ _ _ _ _, value => value
  | .unit, value | .bool, value | .int _ _, value | .num, value | .address, value | .signer, value
  | .string, value | .bytes, value | .char, value | .slice _, value | .vector _, value
  | .array _ _, value | .table _ _, value | .ghost _, value | .param _, value
  | .function _ _ _, value | .nominal _ _ _, value => value
  | .observed referent, value => NTy.viewed prophecies referent value

/-- An enum's value with its active variant read as a clause reads it. -/
def variantViewed (prophecies : Bool) : (names : List String) → (rows : NRows) →
    variantCarrier names rows → variantCarrier names rows
  | _, .nil, value => value
  | [], .cons _ _, value => value
  | _ :: _, .cons fields _, .inl values => .inl (HList.viewed prophecies fields values)
  | _ :: names, .cons _ rest, .inr later => .inr (variantViewed prophecies names rest later)

/-- A row of values as a clause reads it. -/
def HList.viewed (prophecies : Bool) : (row : NRow) → HList row → HList row
  | .nil, values => values
  | .cons τ rest, values => (τ.viewed prophecies values.1, HList.viewed prophecies rest values.2)
end

/-- A certified value encodes as the lent value it certifies. -/
theorem NTy.plainEncode_ofDeepHolds (τ : NTy) (value : τ.plain)
    (holds : NTy.deepHolds τ value) :
    NTy.encode τ (NTy.ofDeepHolds τ value holds) = NTy.plainEncode τ value := by
  rw [← NTy.plainEncode_forget, NTy.forget_ofDeepHolds]

/-! ## Table identity

A table owner's value holds its entries in a hidden last field, at the
runtime as in the denotation (`doc/designs/intrinsic-maps.md`, "Tables in the
runtime semantics"). A specification's equality compares such values by
their identity. -/

/-- Whether a row ends in a hidden contents field, a map owner's. -/
def NRow.holdsContents : NRow → Bool
  | .cons (.table _ _) .nil => true
  | .cons _ (.cons σ rest) => NRow.holdsContents (.cons σ rest)
  | _ => false

omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.holdsContents_single (key value : NTy) :
    NRow.holdsContents (.cons (.table key value) .nil) = true := by
  simp [NRow.holdsContents]
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.holdsContents_cons (τ σ : NTy) (rest : NRow) :
    NRow.holdsContents (.cons τ (.cons σ rest)) = NRow.holdsContents (.cons σ rest) := by
  simp [NRow.holdsContents]


@[simp, grind =] theorem RuntimeValue.eraseGhosts_ghost (value : RuntimeValue) :
    (RuntimeValue.ghost value).eraseGhosts = .ghost .unit := by
  simp [RuntimeValue.eraseGhosts]
@[simp] theorem RuntimeValue.eraseGhosts_nominal (source : StructHandle)
    (variant : Option String) (values : Array RuntimeValue) :
    (RuntimeValue.nominal source variant values).eraseGhosts =
      .nominal source variant (RuntimeValue.eraseGhostsAll values.toList).toArray := by
  simp [RuntimeValue.eraseGhosts]
@[simp] theorem RuntimeValue.eraseGhosts_tuple (values : Array RuntimeValue) :
    (RuntimeValue.tuple values).eraseGhosts =
      .tuple (RuntimeValue.eraseGhostsAll values.toList).toArray := by
  simp [RuntimeValue.eraseGhosts]
@[simp] theorem RuntimeValue.eraseGhosts_vector (values : Array RuntimeValue) :
    (RuntimeValue.vector values).eraseGhosts =
      .vector (RuntimeValue.eraseGhostsAll values.toList).toArray := by
  simp [RuntimeValue.eraseGhosts]
@[simp] theorem RuntimeValue.eraseGhosts_closure (function : FunctionHandle)
    (mask : Nat) (instantiation : Array (TypeId × TypeId)) (captures : Array RuntimeValue) :
    (RuntimeValue.closure function mask instantiation captures).eraseGhosts =
      .closure function mask instantiation (RuntimeValue.eraseGhostsAll captures.toList).toArray := by
  simp [RuntimeValue.eraseGhosts]
@[simp] theorem RuntimeValue.eraseGhosts_borrow (loan : Nat)
    (current : RuntimeValue) :
    (RuntimeValue.borrow loan current).eraseGhosts = .borrow loan current.eraseGhosts := by
  simp [RuntimeValue.eraseGhosts]
@[simp, grind =] theorem RuntimeValue.eraseGhosts_unit :
    RuntimeValue.unit.eraseGhosts = .unit := by simp [RuntimeValue.eraseGhosts]
@[simp, grind =] theorem RuntimeValue.eraseGhosts_bool (value : Bool) :
    (RuntimeValue.bool value).eraseGhosts = .bool value := by simp [RuntimeValue.eraseGhosts]
@[simp, grind =] theorem RuntimeValue.eraseGhosts_character (value : Nat) :
    (RuntimeValue.character value).eraseGhosts = .character value := by
  simp [RuntimeValue.eraseGhosts]
@[simp, grind =] theorem RuntimeValue.eraseGhosts_integer (value : Int) :
    (RuntimeValue.integer value).eraseGhosts = .integer value := by
  simp [RuntimeValue.eraseGhosts]
@[simp, grind =] theorem RuntimeValue.eraseGhosts_address (value : String) :
    (RuntimeValue.address value).eraseGhosts = .address value := by
  simp [RuntimeValue.eraseGhosts]
@[simp, grind =] theorem RuntimeValue.eraseGhosts_signer (value : String) :
    (RuntimeValue.signer value).eraseGhosts = .signer value := by
  simp [RuntimeValue.eraseGhosts]
@[simp, grind =] theorem RuntimeValue.eraseGhosts_string (value : String) :
    (RuntimeValue.string value).eraseGhosts = .string value := by
  simp [RuntimeValue.eraseGhosts]
@[simp, grind =] theorem RuntimeValue.eraseGhosts_bytes (value : Array UInt8) :
    (RuntimeValue.bytes value).eraseGhosts = .bytes value := by
  simp [RuntimeValue.eraseGhosts]
@[simp] theorem RuntimeValue.eraseGhosts_loanHole (loan : Nat) :
    (RuntimeValue.loanHole loan).eraseGhosts = .loanHole loan := by
  simp [RuntimeValue.eraseGhosts]

mutual
/-- The identity a specification's equality compares values of a type holding
map contents by: a table owner with a handle is its handle alone, allocation
identity (`doc/designs/intrinsic-maps.md`, "Equality and order"); any other value
is read field by field, a table without a handle by its entries. -/
def NTy.identityRuntime : NTy → RuntimeValue → RuntimeValue
  | .struct _ _ fields, .nominal actual variant values =>
      match fields with
      | .cons .address rest =>
          if rest.holdsContents then .nominal actual variant #[values[0]?.getD .unit]
          else .nominal actual variant (NRow.identityRuntimeAll fields values.toList).toArray
      | _ => .nominal actual variant (NRow.identityRuntimeAll fields values.toList).toArray
  | .tuple elements, .tuple values =>
      .tuple (NRow.identityRuntimeAll elements values.toList).toArray
  | .enum _ _ names rows _, .nominal actual (some name) values =>
      match NRows.variant? names rows name with
      | some row => .nominal actual (some name) (NRow.identityRuntimeAll row values.toList).toArray
      | none => .nominal actual (some name) values
  | .slice element, .vector values =>
      .vector (NTy.identityRuntimeEach element values.toList).toArray
  | .vector element, .vector values =>
      .vector (NTy.identityRuntimeEach element values.toList).toArray
  | .array element _, .vector values =>
      .vector (NTy.identityRuntimeEach element values.toList).toArray
  | .table key value, .vector entries =>
      .vector (NTy.identityRuntimeEntries key value entries.toList).toArray
  | .sliceRef element, .tuple values =>
      .tuple (NTy.identityRuntimeEach (.vector element) values.toList).toArray
  | .ref referent, .tuple values => .tuple (NTy.identityRuntimeEach referent values.toList).toArray
  -- No equality reads a ghost (`doc/designs/ghost-fields.md`).
  | .ghost _, .ghost _ => .ghost .unit
  | .function _ _ _, value | .nominal _ _ _, value => value.eraseGhosts
  -- A type parameter's instance may hold ghost slots.
  | .param _, value => value.eraseGhosts
  | _, value => value
termination_by _ value => sizeOf value
decreasing_by
  all_goals simp_wf
  all_goals first
    | omega
    | (have := sizeOf_toList_lt ‹Array RuntimeValue›; omega)

def NRow.identityRuntimeAll : NRow → List RuntimeValue → List RuntimeValue
  | .cons τ rest, value :: values =>
      NTy.identityRuntime τ value :: NRow.identityRuntimeAll rest values
  | _, values => values
termination_by _ values => sizeOf values

def NTy.identityRuntimeEach (τ : NTy) : List RuntimeValue → List RuntimeValue
  | [] => []
  | value :: values => NTy.identityRuntime τ value :: NTy.identityRuntimeEach τ values
termination_by values => sizeOf values

def NTy.identityRuntimeEntries (key value : NTy) : List RuntimeValue → List RuntimeValue
  | [] => []
  | .tuple entry :: entries =>
      .tuple (NTy.identityRuntimePair key value entry.toList).toArray ::
        NTy.identityRuntimeEntries key value entries
  | entry :: entries => entry :: NTy.identityRuntimeEntries key value entries
termination_by entries => sizeOf entries
decreasing_by
  all_goals simp_wf
  all_goals first
    | omega
    | (have := sizeOf_toList_lt ‹Array RuntimeValue›; omega)

def NTy.identityRuntimePair (key value : NTy) : List RuntimeValue → List RuntimeValue
  | [k, v] => [NTy.identityRuntime key k, NTy.identityRuntime value v]
  | values => values
termination_by values => sizeOf values
end

omit [Carriers] in
/-- A table owner with a handle is identified by its handle, the first field. -/
@[simp] theorem NTy.identityRuntime_handle (source : StructHandle)
    (arguments rest : NRow) (hidden : rest.holdsContents = true) (actual : StructHandle)
    (variant : Option String) (values : Array RuntimeValue) :
    NTy.identityRuntime (.struct source arguments (.cons .address rest))
        (.nominal actual variant values) =
      .nominal actual variant #[(RuntimeValue.nominal actual variant values).field 0] := by
  rw [NTy.identityRuntime]
  simp [hidden, RuntimeValue.field]

/-- Whether a row is a table owner's identified by its handle: an address,
then the hidden contents. -/
def NRow.ownsHandle : NRow → Bool
  | .cons .address rest => rest.holdsContents
  | _ => false

omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_address (rest : NRow) :
    NRow.ownsHandle (.cons .address rest) = rest.holdsContents := rfl

omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_other (τ : NTy) (rest : NRow) (other : τ ≠ .address) :
    NRow.ownsHandle (.cons τ rest) = false := by
  cases τ <;> simp_all [NRow.ownsHandle]

/-! ## Native equality without ghosts

`NTy.equal` compares native values without reading their ghost slots: the
program's equality of values that may hold them (`Term.equalErased`), and,
with `handles`, a specification's equality, which reads a table owner with a
handle by its handle alone (`doc/designs/intrinsic-maps.md`, "Equality and
order"). `NTy.equal_codec` is the agreement with the encodings. -/

/-- Two lists of one length, related element by element. -/
def listEqual {α : Type} (equal : α → α → Bool) : List α → List α → Bool
  | [], [] => true
  | a :: as, b :: bs => equal a b && listEqual equal as bs
  | _, _ => false

omit [Carriers] in
/-- Lists of encodings compare without ghosts as their elements do. -/
theorem listEqual_eraseGhosts {α : Type} (encode : α → RuntimeValue) (equal : α → α → Bool)
    (agrees : ∀ a b, equal a b = decide ((encode a).eraseGhosts = (encode b).eraseGhosts)) :
    (left right : List α) →
    listEqual equal left right =
      decide (RuntimeValue.eraseGhostsAll (left.map encode) =
        RuntimeValue.eraseGhostsAll (right.map encode))
  | [], [] => by simp [listEqual]
  | [], _ :: _ => by simp [listEqual]
  | _ :: _, [] => by simp [listEqual]
  | a :: as, b :: bs => by
      simp only [listEqual, agrees, listEqual_eraseGhosts encode equal agrees as bs, List.map_cons, RuntimeValue.eraseGhostsAll, List.cons.injEq, Bool.decide_and]

omit [Carriers] in
mutual
/-- Plain equality without ghost slots; with `handles`, a table owner with
a handle compares by its handle. -/
def NTy.plainEqualOf (F : PlainFamily) (handles : Bool) : (τ : NTy) →
    NTy.plainOf F τ → NTy.plainOf F τ → Bool
  | .unit, _, _ => true
  | .bool, left, right => decide (left = right)
  | .int _ _, left, right => decide (left.val = right.val)
  | .num, left, right => decide (left.val = right.val)
  | .address, left, right => decide (left = right)
  | .signer, left, right => decide (left = right)
  | .string, left, right => decide (left = right)
  | .bytes, left, right => decide (left = right)
  | .char, left, right => decide (left = right)
  | .tuple elements, left, right => HListPlainOf.equal F handles elements left right
  | .struct _ _ fields, left, right =>
      if handles && fields.ownsHandle then HListPlainOf.headEqual F handles fields left right
      else HListPlainOf.equal F handles fields left right
  | .enum _ _ names rows _, left, right => variantPlainOf.equal F handles names rows left right
  | .vector element, left, right =>
      listEqual (NTy.plainEqualOf F handles element) left.values.toList right.values.toList
  | .slice element, left, right =>
      listEqual (NTy.plainEqualOf F handles element) left.values.toList right.values.toList
  | .array element _, left, right =>
      listEqual (NTy.plainEqualOf F handles element) left.vector.values.toList
        right.vector.values.toList
  | .table key value, left, right =>
      listEqual (fun l r => NTy.plainEqualOf F handles key l.1 r.1 && NTy.plainEqualOf F handles value l.2 r.2)
        left.entries right.entries
  | .ghost _, _, _ => true
  | .sliceRef element, left, right =>
      listEqual (NTy.plainEqualOf F handles element) left.currentVector.values.toList
          right.currentVector.values.toList &&
        listEqual (NTy.plainEqualOf F handles element) left.finalVector.values.toList
          right.finalVector.values.toList
  | .ref referent, left, right =>
      NTy.plainEqualOf F handles referent left.1 right.1 && NTy.plainEqualOf F handles referent left.2 right.2
  | .observed referent, left, right => NTy.plainEqualOf F handles referent left right
  | .param index, left, right => (F.lent index).erasures.same left right
  | .function parameters shared results, left, right =>
      (F.functions parameters shared results).erasures.same left right
  | .nominal source arguments productive, left, right =>
      (F.nominals source arguments productive).erasures.same left right

/-- Rows compare component by component. -/
def HListPlainOf.equal (F : PlainFamily) (handles : Bool) : (row : NRow) →
    HListPlainOf F row → HListPlainOf F row → Bool
  | .nil, _, _ => true
  | .cons τ rest, left, right =>
      NTy.plainEqualOf F handles τ left.1 right.1 && HListPlainOf.equal F handles rest left.2 right.2

/-- Rows compare by their first component. -/
def HListPlainOf.headEqual (F : PlainFamily) (handles : Bool) : (row : NRow) →
    HListPlainOf F row → HListPlainOf F row → Bool
  | .nil, _, _ => true
  | .cons τ _, left, right => NTy.plainEqualOf F handles τ left.1 right.1

/-- Variant values compare as the same variant with equal rows. -/
def variantPlainOf.equal (F : PlainFamily) (handles : Bool) : (names : List String) →
    (rows : NRows) → variantPlainOf F names rows → variantPlainOf F names rows → Bool
  -- Read with `Sum.elim`, and a constant where no variant is left
  -- (`variantPlainOf`).
  | _, .nil, _, _ => true
  | [], .cons _ _, _, _ => true
  | _ :: names, .cons fields rest, left, right =>
      Sum.elim
        (fun leftRow =>
          Sum.elim (HListPlainOf.equal F handles fields leftRow) (fun _ => false) right)
        (fun leftLater =>
          Sum.elim (fun _ => false)
            (variantPlainOf.equal F handles names rest leftLater) right)
        left
end

/-- Plain equality without ghost slots at a family's lent layer. -/
def NTy.plainEqual (handles : Bool) (τ : NTy) (left right : τ.plain) : Bool :=
  NTy.plainEqualOf Carriers.toPlain handles τ left right

/-- Plain rows compare component by component. -/
def HListPlain.equal (handles : Bool) (row : NRow) (left right : HListPlain row) : Bool :=
  HListPlainOf.equal Carriers.toPlain handles row left right

/-- Plain rows compare by their first component. -/
def HListPlain.headEqual (handles : Bool) (row : NRow) (left right : HListPlain row) : Bool :=
  HListPlainOf.headEqual Carriers.toPlain handles row left right

/-- Plain variant values compare as the same variant with equal rows. -/
def variantPlain.equal (handles : Bool) (names : List String) (rows : NRows)
    (left right : variantPlain names rows) : Bool :=
  variantPlainOf.equal Carriers.toPlain handles names rows left right

mutual
/-- Native equality without ghost slots; with `handles`, a table owner with
a handle compares by its handle. -/
def NTy.equal (handles : Bool) : (τ : NTy) → τ.carrier → τ.carrier → Bool
  | .unit, _, _ => true
  | .bool, left, right => decide (left = right)
  | .int _ _, left, right => decide (left.val = right.val)
  | .num, left, right => decide (left.val = right.val)
  | .address, left, right => decide (left = right)
  | .signer, left, right => decide (left = right)
  | .string, left, right => decide (left = right)
  | .bytes, left, right => decide (left = right)
  | .char, left, right => decide (left = right)
  | .tuple elements, left, right => HList.equal handles elements left right
  | .struct _ _ fields, left, right =>
      if handles && fields.ownsHandle then HListPlain.headEqual handles fields left.val right.val
      else HListPlain.equal handles fields left.val right.val
  | .enum _ _ names rows _, left, right => variantPlain.equal handles names rows left.val right.val
  | .slice element, left, right =>
      listEqual (NTy.equal handles element) left.values.toList right.values.toList

  | .vector element, left, right =>
      listEqual (NTy.equal handles element) left.values.toList right.values.toList
  | .array element _, left, right =>
      listEqual (NTy.equal handles element) left.vector.values.toList right.vector.values.toList
  | .table key value, left, right =>
      listEqual (fun l r => NTy.equal handles key l.1 r.1 && NTy.equal handles value l.2 r.2)
        left.entries right.entries
  | .ghost _, _, _ => true
  | .sliceRef element, left, right =>
      listEqual (NTy.equal handles element) left.currentVector.values.toList right.currentVector.values.toList &&
        listEqual (NTy.equal handles element) left.finalVector.values.toList right.finalVector.values.toList
  | .ref referent, left, right =>
      NTy.plainEqual handles referent left.1 right.1 && NTy.equal handles referent left.2 right.2
  | .param index, left, right => (Carriers.erasures index).same left right
  | .function parameters shared results, left, right =>
      (Carriers.functions parameters shared results).erasures.same left right
  | .nominal source arguments productive, left, right =>
      (Carriers.nominals source arguments productive).erasures.same left right
  | .observed referent, left, right => NTy.equal handles referent left right

/-- Rows compare component by component. -/
def HList.equal (handles : Bool) : (row : NRow) → HList row → HList row → Bool
  | .nil, _, _ => true
  | .cons τ rest, left, right =>
      NTy.equal handles τ left.1 right.1 && HList.equal handles rest left.2 right.2

/-- Rows compare by their first component. -/
def HList.headEqual (handles : Bool) : (row : NRow) → HList row → HList row → Bool
  | .nil, _, _ => true
  | .cons τ _, left, right => NTy.equal handles τ left.1 right.1

/-- Variant values compare as the same variant with equal rows. -/
def variantCarrier.equal (handles : Bool) : (names : List String) → (rows : NRows) →
    variantCarrier names rows → variantCarrier names rows → Bool
  -- Read with `Sum.elim`, and a constant where no variant is left
  -- (`variantPlainOf`).
  | _, .nil, _, _ => true
  | [], .cons _ _, _, _ => true
  | _ :: names, .cons fields rest, left, right =>
      Sum.elim
        (fun leftRow =>
          Sum.elim (HList.equal handles fields leftRow) (fun _ => false) right)
        (fun leftLater =>
          Sum.elim (fun _ => false)
            (variantCarrier.equal handles names rest leftLater) right)
        left
end

/-! The native equality at constructed types, as the normal form reads it.
A type parameter's equality stays its family's, and a function value's its
closure's. -/

section EqualEquations
variable (handles : Bool)

omit [Carriers] in
@[simp, vir_denote_norm] theorem listEqual_nil {α : Type} (equal : α → α → Bool) :
    listEqual equal [] [] = true := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem listEqual_cons {α : Type} (equal : α → α → Bool)
    (a b : α) (as bs : List α) :
    listEqual equal (a :: as) (b :: bs) = (equal a b && listEqual equal as bs) := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem listEqual_nil_cons {α : Type} (equal : α → α → Bool)
    (b : α) (bs : List α) : listEqual equal [] (b :: bs) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem listEqual_cons_nil {α : Type} (equal : α → α → Bool)
    (a : α) (as : List α) : listEqual equal (a :: as) [] = false := rfl

@[simp, vir_denote_norm] theorem NTy.equal_unit (left right : NTy.unit.carrier) :
    NTy.equal handles .unit left right = true := rfl
@[simp, vir_denote_norm] theorem NTy.equal_bool (left right : NTy.bool.carrier) :
    NTy.equal handles .bool left right = decide (left = right) := rfl
@[simp, vir_denote_norm] theorem NTy.equal_int (width : Nat) (signed : Bool)
    (left right : (NTy.int width signed).carrier) :
    NTy.equal handles (.int width signed) left right = decide (left.val = right.val) := rfl
@[simp, vir_denote_norm] theorem NTy.equal_num (left right : NTy.num.carrier) :
    NTy.equal handles .num left right = decide (left.val = right.val) := rfl
@[simp, vir_denote_norm] theorem NTy.equal_address (left right : NTy.address.carrier) :
    NTy.equal handles .address left right = decide (left = right) := rfl
@[simp, vir_denote_norm] theorem NTy.equal_signer (left right : NTy.signer.carrier) :
    NTy.equal handles .signer left right = decide (left = right) := rfl
@[simp, vir_denote_norm] theorem NTy.equal_string (left right : NTy.string.carrier) :
    NTy.equal handles .string left right = decide (left = right) := rfl
@[simp, vir_denote_norm] theorem NTy.equal_bytes (left right : NTy.bytes.carrier) :
    NTy.equal handles .bytes left right = decide (left = right) := rfl
@[simp, vir_denote_norm] theorem NTy.equal_tuple (elements : NRow)
    (left right : (NTy.tuple elements).carrier) :
    NTy.equal handles (.tuple elements) left right = HList.equal handles elements left right := rfl
@[simp, vir_denote_norm] theorem NTy.equal_struct_handle (owner : StructHandle) (arguments fields : NRow)
    (owns : fields.ownsHandle = true) (left right : (NTy.struct owner arguments fields).carrier) :
    NTy.equal true (.struct owner arguments fields) left right =
      HListPlain.headEqual true fields left.val right.val := by
  simp [NTy.equal, owns]
@[simp, vir_denote_norm] theorem NTy.equal_struct (owner : StructHandle) (arguments fields : NRow)
    (plain : (handles && fields.ownsHandle) = false)
    (left right : (NTy.struct owner arguments fields).carrier) :
    NTy.equal handles (.struct owner arguments fields) left right =
      HListPlain.equal handles fields left.val right.val := by
  simp [NTy.equal, plain]
@[simp, vir_denote_norm] theorem NTy.equal_enum (owner : StructHandle) (arguments : NRow)
    (names : List String) (rows : NRows) (distinct : names.Nodup)
    (left right : (NTy.enum owner arguments names rows distinct).carrier) :
    NTy.equal handles (.enum owner arguments names rows distinct) left right =
      variantPlain.equal handles names rows left.val right.val := rfl
@[simp, vir_denote_norm] theorem NTy.equal_vector (element : NTy)
    (left right : (NTy.vector element).carrier) :
    NTy.equal handles (.vector element) left right =
      listEqual (NTy.equal handles element) left.values.toList right.values.toList := rfl
theorem NTy.equal_table (key value : NTy)
    (left right : (NTy.table key value).carrier) :
    NTy.equal handles (.table key value) left right =
      listEqual (fun l r => NTy.equal handles key l.1 r.1 && NTy.equal handles value l.2 r.2)
        left.entries right.entries := rfl
/-- Tables of scalar keys and values compare as their typed maps
(`NTy.equal_table_scalar`); others entry by entry. -/
@[simp, vir_denote_norm] theorem NTy.equal_table_entries (key value : NTy)
    (compound : (key.isScalar && value.isScalar) = false)
    (left right : (NTy.table key value).carrier) :
    NTy.equal handles (.table key value) left right =
      listEqual (fun l r => NTy.equal handles key l.1 r.1 && NTy.equal handles value l.2 r.2)
        left.entries right.entries := rfl
@[simp, vir_denote_norm] theorem NTy.equal_ghost (slot : NTy) (left right : (NTy.ghost slot).carrier) :
    NTy.equal handles (.ghost slot) left right = true := rfl
@[simp, vir_denote_norm] theorem NTy.equal_function (parameters : NRow) (shared : Sharing) (results : NRow)
    (left right : (NTy.function parameters shared results).carrier) :
    NTy.equal handles (.function parameters shared results) left right =
      decide ((Carriers.functions parameters shared results).erased left =
        (Carriers.functions parameters shared results).erased right) := rfl
@[simp, vir_denote_norm] theorem NTy.equal_nominal (source : StructHandle) (arguments : NRow)
    (productive : Bool) (left right : (NTy.nominal source arguments productive).carrier) :
    NTy.equal handles (.nominal source arguments productive) left right =
      decide (((Carriers.nominals source arguments productive).value left).eraseGhosts =
        ((Carriers.nominals source arguments productive).value right).eraseGhosts) := rfl
/-- A type parameter's equality stays folded, at one flag: its values hold
no handle. -/
@[simp, vir_denote_norm] theorem NTy.equal_param_handles (index : Nat)
    (left right : (NTy.param index).carrier) :
    NTy.equal true (.param index) left right = NTy.equal false (.param index) left right := rfl
@[simp, vir_denote_norm] theorem NTy.equal_ref (referent : NTy)
    (left right : (NTy.ref referent).carrier) :
    NTy.equal handles (.ref referent) left right =
      (NTy.plainEqual handles referent left.1 right.1 && NTy.equal handles referent left.2 right.2) :=
  rfl

@[simp, vir_denote_norm] theorem HList.equal_nil (left right : HList .nil) :
    HList.equal handles .nil left right = true := rfl
@[simp, vir_denote_norm] theorem HList.equal_cons (τ : NTy) (rest : NRow)
    (left right : HList (.cons τ rest)) :
    HList.equal handles (.cons τ rest) left right =
      (NTy.equal handles τ left.1 right.1 && HList.equal handles rest left.2 right.2) := rfl
@[simp, vir_denote_norm] theorem HList.headEqual_cons (τ : NTy) (rest : NRow)
    (left right : HList (.cons τ rest)) :
    HList.headEqual handles (.cons τ rest) left right = NTy.equal handles τ left.1 right.1 := rfl

@[simp, vir_denote_norm] theorem variantCarrier.equal_inl_inl (name : String) (names : List String)
    (fields : NRow) (rest : NRows) (left right : HList fields) :
    variantCarrier.equal handles (name :: names) (.cons fields rest) (.inl left) (.inl right) =
      HList.equal handles fields left right := rfl
@[simp, vir_denote_norm] theorem variantCarrier.equal_inl_inr (name : String) (names : List String)
    (fields : NRow) (rest : NRows) (left : HList fields) (right : variantCarrier names rest) :
    variantCarrier.equal handles (name :: names) (.cons fields rest) (.inl left) (.inr right) =
      false := rfl
@[simp, vir_denote_norm] theorem variantCarrier.equal_inr_inl (name : String) (names : List String)
    (fields : NRow) (rest : NRows) (left : variantCarrier names rest) (right : HList fields) :
    variantCarrier.equal handles (name :: names) (.cons fields rest) (.inr left) (.inl right) =
      false := rfl
@[simp, vir_denote_norm] theorem variantCarrier.equal_inr_inr (name : String) (names : List String)
    (fields : NRow) (rest : NRows) (left right : variantCarrier names rest) :
    variantCarrier.equal handles (name :: names) (.cons fields rest) (.inr left) (.inr right) =
      variantCarrier.equal handles names rest left right := rfl

/-! The plain equality at each type, mirroring the certified equations
(`NTy.equal_int`, `HList.equal_cons`, `variantCarrier.equal_inl_inl`, …):
one normal form at either layer, so that a clause comparing the current of
a mutable reference reduces as one comparing a value of its type does. -/

@[simp, vir_denote_norm] theorem NTy.plainEqual_unit (left right : (NTy.unit).plain) :
    NTy.plainEqual handles .unit left right = true := rfl
@[simp, vir_denote_norm] theorem NTy.plainEqual_bool (left right : (NTy.bool).plain) :
    NTy.plainEqual handles .bool left right = decide (left = right) := rfl
@[simp, vir_denote_norm] theorem NTy.plainEqual_int (width : Nat) (signed : Bool)
    (left right : (NTy.int width signed).plain) :
    NTy.plainEqual handles (.int width signed) left right = decide (left.val = right.val) := rfl
@[simp, vir_denote_norm] theorem NTy.plainEqual_num (left right : (NTy.num).plain) :
    NTy.plainEqual handles .num left right = decide (left.val = right.val) := rfl
@[simp, vir_denote_norm] theorem NTy.plainEqual_address (left right : (NTy.address).plain) :
    NTy.plainEqual handles .address left right = decide (left = right) := rfl
@[simp, vir_denote_norm] theorem NTy.plainEqual_signer (left right : (NTy.signer).plain) :
    NTy.plainEqual handles .signer left right = decide (left = right) := rfl
@[simp, vir_denote_norm] theorem NTy.plainEqual_string (left right : (NTy.string).plain) :
    NTy.plainEqual handles .string left right = decide (left = right) := rfl
@[simp, vir_denote_norm] theorem NTy.plainEqual_bytes (left right : (NTy.bytes).plain) :
    NTy.plainEqual handles .bytes left right = decide (left = right) := rfl
@[simp, vir_denote_norm] theorem NTy.plainEqual_tuple (elements : NRow)
    (left right : (NTy.tuple elements).plain) :
    NTy.plainEqual handles (.tuple elements) left right =
      HListPlain.equal handles elements left right := rfl
@[simp, vir_denote_norm] theorem NTy.plainEqual_ghost (slot : NTy)
    (left right : (NTy.ghost slot).plain) :
    NTy.plainEqual handles (.ghost slot) left right = true := rfl
@[simp, vir_denote_norm] theorem NTy.plainEqual_ref (referent : NTy)
    (left right : (NTy.ref referent).plain) :
    NTy.plainEqual handles (.ref referent) left right =
      (NTy.plainEqual handles referent left.1 right.1 &&
        NTy.plainEqual handles referent left.2 right.2) := rfl

@[simp, vir_denote_norm] theorem NTy.plainEqual_struct_handle (owner : StructHandle)
    (arguments fields : NRow) (owns : fields.ownsHandle = true)
    (left right : (NTy.struct owner arguments fields).plain) :
    NTy.plainEqual true (.struct owner arguments fields) left right =
      HListPlain.headEqual true fields left right := by
  simp [NTy.plainEqual, NTy.plainEqualOf, HListPlain.headEqual, owns]
@[simp, vir_denote_norm] theorem NTy.plainEqual_struct (owner : StructHandle)
    (arguments fields : NRow) (plain : (handles && fields.ownsHandle) = false)
    (left right : (NTy.struct owner arguments fields).plain) :
    NTy.plainEqual handles (.struct owner arguments fields) left right =
      HListPlain.equal handles fields left right := by
  simp [NTy.plainEqual, NTy.plainEqualOf, HListPlain.equal, plain]
@[simp, vir_denote_norm] theorem NTy.plainEqual_enum (owner : StructHandle) (arguments : NRow)
    (names : List String) (rows : NRows) (distinct : names.Nodup)
    (left right : (NTy.enum owner arguments names rows distinct).plain) :
    NTy.plainEqual handles (.enum owner arguments names rows distinct) left right =
      variantPlain.equal handles names rows left right := rfl

@[simp, vir_denote_norm] theorem HListPlain.equal_nil (left right : HListPlain .nil) :
    HListPlain.equal handles .nil left right = true := rfl
@[simp, vir_denote_norm] theorem HListPlain.equal_cons (τ : NTy) (rest : NRow)
    (left right : HListPlain (.cons τ rest)) :
    HListPlain.equal handles (.cons τ rest) left right =
      (NTy.plainEqual handles τ left.1 right.1 &&
        HListPlain.equal handles rest left.2 right.2) := rfl
@[simp, vir_denote_norm] theorem HListPlain.headEqual_cons (τ : NTy) (rest : NRow)
    (left right : HListPlain (.cons τ rest)) :
    HListPlain.headEqual handles (.cons τ rest) left right =
      NTy.plainEqual handles τ left.1 right.1 := rfl

@[simp, vir_denote_norm] theorem variantPlain.equal_inl_inl (name : String)
    (names : List String) (fields : NRow) (rest : NRows) (left right : HListPlain fields) :
    variantPlain.equal handles (name :: names) (.cons fields rest) (.inl left) (.inl right) =
      HListPlain.equal handles fields left right := rfl
@[simp, vir_denote_norm] theorem variantPlain.equal_inl_inr (name : String)
    (names : List String) (fields : NRow) (rest : NRows) (left : HListPlain fields)
    (right : variantPlain names rest) :
    variantPlain.equal handles (name :: names) (.cons fields rest) (.inl left) (.inr right) =
      false := rfl
@[simp, vir_denote_norm] theorem variantPlain.equal_inr_inl (name : String)
    (names : List String) (fields : NRow) (rest : NRows) (left : variantPlain names rest)
    (right : HListPlain fields) :
    variantPlain.equal handles (name :: names) (.cons fields rest) (.inr left) (.inl right) =
      false := rfl
@[simp, vir_denote_norm] theorem variantPlain.equal_inr_inr (name : String)
    (names : List String) (fields : NRow) (rest : NRows)
    (left right : variantPlain names rest) :
    variantPlain.equal handles (name :: names) (.cons fields rest) (.inr left) (.inr right) =
      variantPlain.equal handles names rest left right := rfl

end EqualEquations

mutual
/-- Plain values compare without ghosts as their encodings do. -/
theorem NTy.plainEqual_codec : (τ : NTy) → (left right : τ.plain) →
    NTy.plainEqual false τ left right =
      decide ((τ.plainCodec.encode left).eraseGhosts = (τ.plainCodec.encode right).eraseGhosts)
  | .unit, _, _ => by simp [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.unit]
  | .bool, left, right => by simp [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.bool]
  | .int _ _, left, right => by simp [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.specInt]
  | .num, left, right => by simp [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.specInt]
  | .address, left, right => by simp [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.address]
  | .signer, left, right => by simp [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.signer]
  | .string, left, right => by simp [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.string]
  | .bytes, left, right => by simp [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.bytes]
  | .char, left, right => by
      simp [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.character,
        SpecChar.eq_iff_val]
  | .tuple elements, left, right => by
      simp only [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.tuple, RuntimeValue.eraseGhosts_tuple, List.toList_toArray, RuntimeValue.tuple.injEq, Array.mk.injEq]
      exact HListPlain.equal_codec elements left right
  | .struct source _ fields, left, right => by
      simp only [NTy.plainEqual, NTy.plainEqualOf, Bool.false_and, Bool.false_eq_true, ↓reduceIte, NTy.plainCodec, NTy.plainCodecOf, Codec.nominalRow, RuntimeValue.eraseGhosts_nominal, List.toList_toArray, RuntimeValue.nominal.injEq, true_and, Array.mk.injEq]
      exact HListPlain.equal_codec fields left right
  | .enum source _ names rows distinct, left, right => by
      simp only [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, variantCodecPlain, variantCodecPlainOf]
      exact variantPlain.equal_codec source names rows distinct left right
  | .vector element, left, right => by
      simp only [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.boundedVector, RuntimeValue.eraseGhosts_vector, Array.toList_map, RuntimeValue.vector.injEq, Array.mk.injEq]
      exact listEqual_eraseGhosts _ _ (NTy.plainEqual_codec element) _ _
  | .slice element, left, right => by
      simp only [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.boundedVector, RuntimeValue.eraseGhosts_vector, Array.toList_map, RuntimeValue.vector.injEq, Array.mk.injEq]
      exact listEqual_eraseGhosts _ _ (NTy.plainEqual_codec element) _ _
  | .array element _, left, right => by
      simp only [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.array, Codec.boundedVector, RuntimeValue.eraseGhosts_vector, Array.toList_map, RuntimeValue.vector.injEq, Array.mk.injEq]
      exact listEqual_eraseGhosts _ _ (NTy.plainEqual_codec element) _ _
  | .table key value, left, right => by
      simp only [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.entries, RuntimeValue.eraseGhosts_vector, List.toList_toArray, RuntimeValue.vector.injEq, Array.mk.injEq]
      refine listEqual_eraseGhosts _ _ (fun a b => ?_) _ _
      show (NTy.plainEqual false key a.1 b.1 && NTy.plainEqual false value a.2 b.2) = _
      simp only [RuntimeValue.eraseGhosts_tuple, List.toList_toArray, RuntimeValue.eraseGhostsAll, RuntimeValue.tuple.injEq, Array.mk.injEq, List.cons.injEq, and_true, Bool.decide_and, NTy.plainEqual_codec key a.1 b.1, NTy.plainEqual_codec value a.2 b.2]
  | .ghost slot, left, right => by
      simp [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.ghost]
  | .ref referent, left, right => by
      show (NTy.plainEqual false referent left.1 right.1 &&
        NTy.plainEqual false referent left.2 right.2) = _
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.prophecyPair, RuntimeValue.eraseGhosts_tuple, List.toList_toArray, RuntimeValue.eraseGhostsAll, RuntimeValue.tuple.injEq, Array.mk.injEq, List.cons.injEq, and_true, Bool.decide_and, NTy.plainEqual_codec referent left.1 right.1, NTy.plainEqual_codec referent left.2 right.2]
  | .sliceRef element, left, right => by
      simp only [NTy.plainEqual, NTy.plainEqualOf, NTy.plainCodec, NTy.plainCodecOf, Codec.sliceRef, Codec.boundedVector, RuntimeValue.eraseGhosts_tuple, List.toList_toArray, RuntimeValue.eraseGhostsAll, RuntimeValue.tuple.injEq, Array.mk.injEq, List.cons.injEq, and_true, Bool.decide_and, RuntimeValue.eraseGhosts_vector, Array.toList_map, RuntimeValue.vector.injEq]
      congr 1 <;> exact listEqual_eraseGhosts _ _ (NTy.plainEqual_codec element) _ _
  | .observed referent, left, right => NTy.plainEqual_codec referent left right
  | .param index, left, right => (Carriers.lent index).erasures.same_eq left right
  | .function parameters shared results, left, right =>
      (Carriers.functions parameters shared results).erasures.same_eq left right
  | .nominal source arguments productive, left, right =>
      (Carriers.nominals source arguments productive).erasures.same_eq left right

/-- Rows compare without ghosts as the lists of their encodings do. -/
theorem HListPlain.equal_codec : (row : NRow) → (left right : HListPlain row) →
    HListPlain.equal false row left right =
      decide (RuntimeValue.eraseGhostsAll ((rowCodecPlain row).encode left) =
        RuntimeValue.eraseGhostsAll ((rowCodecPlain row).encode right))
  | .nil, _, _ => by simp [HListPlain.equal, HListPlainOf.equal, rowCodecPlain, rowCodecPlainOf, Codec.tupleNil]
  | .cons τ rest, left, right => by
      show (NTy.plainEqual false τ left.1 right.1 &&
        HListPlain.equal false rest left.2 right.2) = _
      simp only [rowCodecPlain, rowCodecPlainOf, Codec.tupleCons, RuntimeValue.eraseGhostsAll, List.cons.injEq, Bool.decide_and, NTy.plainEqual_codec τ left.1 right.1, HListPlain.equal_codec rest left.2 right.2]

/-- Variant values compare without ghosts as their encodings do. -/
theorem variantPlain.equal_codec (source : StructHandle) :
    (names : List String) → (rows : NRows) → names.Nodup → (left right : variantPlain names rows) →
    variantPlain.equal false names rows left right =
      decide ((variantEncodePlain source names rows (rowCodecsPlain rows) left).eraseGhosts =
        (variantEncodePlain source names rows (rowCodecsPlain rows) right).eraseGhosts)
  | _, .nil, _, left, _ => nomatch left
  | [], .cons _ _, _, left, _ => nomatch left
  | _ :: _, .cons fields _, _, .inl left, .inl right => by
      simp only [variantPlain.equal, variantPlainOf.equal, variantEncodePlain, variantEncodePlainOf, Sum.elim_inl, Sum.elim_inr, rowCodecsPlain, rowCodecsPlainOf, Codec.nominalRow, RuntimeValue.eraseGhosts_nominal, List.toList_toArray, RuntimeValue.nominal.injEq, true_and, Array.mk.injEq]
      exact HListPlain.equal_codec fields left right
  | name :: names, .cons _ rest, distinct, .inl left, .inr right => by
      obtain ⟨fields, encoded⟩ := variantEncodePlain_nominal source names rest (rowCodecsPlain rest) right
      have different : name ≠ variantPlainName names rest right := fun same =>
        (List.nodup_cons.mp distinct).1 (same ▸ variantPlainName_mem names rest right)
      simp only [variantPlain.equal, variantPlainOf.equal, variantEncodePlain, variantEncodePlainOf, Sum.elim_inl, Sum.elim_inr, rowCodecsPlain, rowCodecsPlainOf, Codec.nominalRow, encoded, RuntimeValue.eraseGhosts_nominal, RuntimeValue.nominal.injEq, Option.some.injEq, different, false_and, and_false, decide_false]
  | name :: names, .cons _ rest, distinct, .inr left, .inl right => by
      obtain ⟨fields, encoded⟩ := variantEncodePlain_nominal source names rest (rowCodecsPlain rest) left
      have different : variantPlainName names rest left ≠ name := fun same =>
        (List.nodup_cons.mp distinct).1 (same ▸ variantPlainName_mem names rest left)
      simp only [variantPlain.equal, variantPlainOf.equal, variantEncodePlain, variantEncodePlainOf, Sum.elim_inl, Sum.elim_inr, rowCodecsPlain, rowCodecsPlainOf, Codec.nominalRow, encoded, RuntimeValue.eraseGhosts_nominal, RuntimeValue.nominal.injEq, Option.some.injEq, different, false_and, and_false, decide_false]
  | _ :: names, .cons _ rest, distinct, .inr left, .inr right => by
      simp only [variantPlain.equal, variantPlainOf.equal, variantEncodePlain, variantEncodePlainOf, Sum.elim_inl, Sum.elim_inr, rowCodecsPlain, rowCodecsPlainOf]
      exact variantPlain.equal_codec source names rest (List.nodup_cons.mp distinct).2 left right
end

mutual
/-- Native values compare without ghosts as their encodings do. -/
theorem NTy.equal_codec : (τ : NTy) → (left right : τ.carrier) →
    NTy.equal false τ left right =
      decide ((τ.codec.encode left).eraseGhosts = (τ.codec.encode right).eraseGhosts)
  | .unit, _, _ => by simp [NTy.equal, NTy.codec, Codec.unit]
  | .bool, left, right => by simp [NTy.equal, NTy.codec, Codec.bool]
  | .int _ _, left, right => by simp [NTy.equal, NTy.codec, Codec.specInt]
  | .num, left, right => by simp [NTy.equal, NTy.codec, Codec.specInt]
  | .address, left, right => by simp [NTy.equal, NTy.codec, Codec.address]
  | .signer, left, right => by simp [NTy.equal, NTy.codec, Codec.signer]
  | .string, left, right => by simp [NTy.equal, NTy.codec, Codec.string]
  | .bytes, left, right => by simp [NTy.equal, NTy.codec, Codec.bytes]
  | .char, left, right => by simp [NTy.equal, NTy.codec, Codec.character, SpecChar.eq_iff_val]
  | .tuple elements, left, right => by
      simp only [NTy.equal, NTy.codec, Codec.tuple, RuntimeValue.eraseGhosts_tuple, List.toList_toArray, RuntimeValue.tuple.injEq, Array.mk.injEq]
      exact HList.equal_codec elements left right
  | .struct source arguments fields, left, right => by
      have plain := NTy.plainEqual_codec (.struct source arguments fields) left.val right.val
      simp only [NTy.plainEqual, NTy.plainEqualOf, Bool.false_and, Bool.false_eq_true, ↓reduceIte] at plain
      simp only [NTy.equal, Bool.false_and, Bool.false_eq_true, ↓reduceIte, NTy.codec, Codec.certified]
      exact plain
  | .enum source arguments names rows distinct, left, right => by
      simp only [NTy.equal, NTy.codec, Codec.certified]
      exact NTy.plainEqual_codec (.enum source arguments names rows distinct) left.val right.val
  | .slice element, left, right => by
      simp only [NTy.equal, NTy.codec, Codec.boundedVector, RuntimeValue.eraseGhosts_vector,
        Array.toList_map, RuntimeValue.vector.injEq, Array.mk.injEq]
      exact listEqual_eraseGhosts _ _ (NTy.equal_codec element) _ _

  | .vector element, left, right => by
      simp only [NTy.equal, NTy.codec, Codec.boundedVector, RuntimeValue.eraseGhosts_vector, Array.toList_map, RuntimeValue.vector.injEq, Array.mk.injEq]
      exact listEqual_eraseGhosts _ _ (NTy.equal_codec element) _ _
  | .array element _, left, right => by
      simp only [NTy.equal, NTy.codec, Codec.array, Codec.boundedVector,
        RuntimeValue.eraseGhosts_vector, Array.toList_map, RuntimeValue.vector.injEq, Array.mk.injEq]
      exact listEqual_eraseGhosts _ _ (NTy.equal_codec element) _ _
  | .table key value, left, right => by
      simp only [NTy.equal, NTy.codec, Codec.entries, RuntimeValue.eraseGhosts_vector, List.toList_toArray, RuntimeValue.vector.injEq, Array.mk.injEq]
      refine listEqual_eraseGhosts _ _ (fun a b => ?_) _ _
      simp only [RuntimeValue.eraseGhosts_tuple, List.toList_toArray, RuntimeValue.eraseGhostsAll, RuntimeValue.tuple.injEq, Array.mk.injEq, List.cons.injEq, and_true, Bool.decide_and, NTy.equal_codec key a.1 b.1, NTy.equal_codec value a.2 b.2]
  | .ghost slot, left, right => by
      simp [NTy.equal, NTy.codec, Codec.ghost]
  | .observed referent, left, right => NTy.equal_codec referent left right
  | .sliceRef element, left, right => by
      simp only [NTy.equal, NTy.codec, Codec.sliceRef, Codec.boundedVector,
        RuntimeValue.eraseGhosts_tuple, List.toList_toArray, RuntimeValue.eraseGhostsAll,
        RuntimeValue.tuple.injEq, Array.mk.injEq, List.cons.injEq, and_true, Bool.decide_and,
        RuntimeValue.eraseGhosts_vector, Array.toList_map, RuntimeValue.vector.injEq]
      congr 1 <;> exact listEqual_eraseGhosts _ _ (NTy.equal_codec element) _ _
  | .ref referent, left, right => by
      simp only [NTy.equal, NTy.codec, Codec.pair, RuntimeValue.eraseGhosts_tuple, List.toList_toArray, RuntimeValue.eraseGhostsAll, RuntimeValue.tuple.injEq, Array.mk.injEq, List.cons.injEq, and_true, Bool.decide_and, NTy.plainEqual_codec referent left.1 right.1, NTy.equal_codec referent left.2 right.2]
  | .param index, left, right => (Carriers.erasures index).same_eq left right
  | .function parameters shared results, left, right =>
      (Carriers.functions parameters shared results).erasures.same_eq left right
  | .nominal source arguments productive, left, right =>
      (Carriers.nominals source arguments productive).erasures.same_eq left right

/-- Rows compare without ghosts as the lists of their encodings do. -/
theorem HList.equal_codec : (row : NRow) → (left right : HList row) →
    HList.equal false row left right =
      decide (RuntimeValue.eraseGhostsAll ((rowCodec row).encode left) =
        RuntimeValue.eraseGhostsAll ((rowCodec row).encode right))
  | .nil, _, _ => by simp [HList.equal, rowCodec, Codec.tupleNil]
  | .cons τ rest, left, right => by
      simp only [HList.equal, rowCodec, Codec.tupleCons, RuntimeValue.eraseGhostsAll, List.cons.injEq, Bool.decide_and, NTy.equal_codec τ left.1 right.1, HList.equal_codec rest left.2 right.2]

/-- Variant values compare without ghosts as their encodings do. -/
theorem variantCarrier.equal_codec (source : StructHandle) :
    (names : List String) → (rows : NRows) → names.Nodup → (left right : variantCarrier names rows) →
    variantCarrier.equal false names rows left right =
      decide ((variantEncode source names rows (rowCodecs rows) left).eraseGhosts =
        (variantEncode source names rows (rowCodecs rows) right).eraseGhosts)
  | _, .nil, _, left, _ => nomatch left
  | [], .cons _ _, _, left, _ => nomatch left
  | _ :: _, .cons fields _, _, .inl left, .inl right => by
      simp only [variantCarrier.equal, variantEncode, Sum.elim_inl, Sum.elim_inr, rowCodecs, Codec.nominalRow, RuntimeValue.eraseGhosts_nominal, List.toList_toArray, RuntimeValue.nominal.injEq, true_and, Array.mk.injEq]
      exact HList.equal_codec fields left right
  | name :: names, .cons _ rest, distinct, .inl left, .inr right => by
      obtain ⟨fields, encoded⟩ := variantEncode_nominal source names rest (rowCodecs rest) right
      have different : name ≠ variantName names rest right := fun same =>
        (List.nodup_cons.mp distinct).1 (same ▸ variantName_mem names rest right)
      simp only [variantCarrier.equal, variantEncode, Sum.elim_inl, Sum.elim_inr, rowCodecs, Codec.nominalRow, encoded, RuntimeValue.eraseGhosts_nominal, RuntimeValue.nominal.injEq, Option.some.injEq, different, false_and, and_false, decide_false]
  | name :: names, .cons _ rest, distinct, .inr left, .inl right => by
      obtain ⟨fields, encoded⟩ := variantEncode_nominal source names rest (rowCodecs rest) left
      have different : variantName names rest left ≠ name := fun same =>
        (List.nodup_cons.mp distinct).1 (same ▸ variantName_mem names rest left)
      simp only [variantCarrier.equal, variantEncode, Sum.elim_inl, Sum.elim_inr, rowCodecs, Codec.nominalRow, encoded, RuntimeValue.eraseGhosts_nominal, RuntimeValue.nominal.injEq, Option.some.injEq, different, false_and, and_false, decide_false]
  | _ :: names, .cons _ rest, distinct, .inr left, .inr right => by
      simp only [variantCarrier.equal, variantEncode, Sum.elim_inl, Sum.elim_inr, rowCodecs]
      exact variantCarrier.equal_codec source names rest (List.nodup_cons.mp distinct).2 left right
end

/-- Native values compare without ghosts as their encodings do. -/
theorem NTy.equal_encode (τ : NTy) (left right : τ.carrier) :
    NTy.equal false τ left right =
      decide ((τ.encode left).eraseGhosts = (τ.encode right).eraseGhosts) :=
  NTy.equal_codec τ left right

/-- Native equality is reflexive, at a type parameter's values too. -/
theorem NTy.equal_self (τ : NTy) (value : τ.carrier) : NTy.equal false τ value value = true := by
  rw [NTy.equal_codec]; simp

attribute [simp, vir_denote_norm high] NTy.equal_self

/-- A type's erasure as a family's erasure of a type parameter. -/
@[reducible] def NTy.typedErasure (τ : NTy) : TypedErasure τ.carrier τ.codec.encode where
  same := NTy.equal false τ
  same_eq := NTy.equal_codec τ

/-- A type's equality of lent values as a family's erasure of a type
parameter's (`Carriers.lent`). -/
@[reducible] def NTy.plainTypedErasure (τ : NTy) : TypedErasure τ.plain τ.plainCodec.encode where
  same := NTy.plainEqual false τ
  same_eq := NTy.plainEqual_codec τ

/-! `ownsHandle` at each other leading field, without a side condition: a
normalizer whose discharger reads the context alone decides these. -/
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_unit (rest : NRow) :
    NRow.ownsHandle (.cons .unit rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_bool (rest : NRow) :
    NRow.ownsHandle (.cons .bool rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_int (width : Nat) (signed : Bool) (rest : NRow) :
    NRow.ownsHandle (.cons (.int width signed) rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_num (rest : NRow) :
    NRow.ownsHandle (.cons .num rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_signer (rest : NRow) :
    NRow.ownsHandle (.cons .signer rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_string (rest : NRow) :
    NRow.ownsHandle (.cons .string rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_bytes (rest : NRow) :
    NRow.ownsHandle (.cons .bytes rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_tuple (elements rest : NRow) :
    NRow.ownsHandle (.cons (.tuple elements) rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_struct (source : StructHandle)
    (arguments fields rest : NRow) :
    NRow.ownsHandle (.cons (.struct source arguments fields) rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_enum (source : StructHandle) (arguments : NRow)
    (names : List String) (rows : NRows) (distinct : names.Nodup) (rest : NRow) :
    NRow.ownsHandle (.cons (.enum source arguments names rows distinct) rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_vector (element : NTy) (rest : NRow) :
    NRow.ownsHandle (.cons (.vector element) rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_table (key value : NTy) (rest : NRow) :
    NRow.ownsHandle (.cons (.table key value) rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_ghost (slot : NTy) (rest : NRow) :
    NRow.ownsHandle (.cons (.ghost slot) rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_ref (referent : NTy) (rest : NRow) :
    NRow.ownsHandle (.cons (.ref referent) rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_param (index : Nat) (rest : NRow) :
    NRow.ownsHandle (.cons (.param index) rest) = false := rfl
omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_function (parameters : NRow) (shared : Sharing)
    (results rest : NRow) :
    NRow.ownsHandle (.cons (.function parameters shared results) rest) = false := rfl

omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_nominal (source : StructHandle) (arguments : NRow)
    (productive : Bool) (rest : NRow) :
    NRow.ownsHandle (.cons (.nominal source arguments productive) rest) = false := rfl

omit [Carriers] in
@[simp, vir_denote_norm] theorem NRow.ownsHandle_nil : NRow.ownsHandle .nil = false := rfl

omit [Carriers] in
/-- A struct not identified by a handle is read field by field. -/
@[simp] theorem NTy.identityRuntime_fieldwise (source : StructHandle)
    (arguments fields : NRow) (actual : StructHandle) (variant : Option String)
    (values : Array RuntimeValue) (fieldwise : fields.ownsHandle = false) :
    NTy.identityRuntime (.struct source arguments fields) (.nominal actual variant values) =
      .nominal actual variant (NRow.identityRuntimeAll fields values.toList).toArray := by
  cases fields with
  | nil => simp [NTy.identityRuntime]
  | cons τ rest => cases τ <;> simp_all [NRow.ownsHandle, NTy.identityRuntime]

omit [Carriers] in
/-- No equality reads a ghost slot (`doc/designs/ghost-fields.md`). -/
@[simp] theorem NTy.identityRuntime_ghost (slot : NTy) (contents : RuntimeValue) :
    NTy.identityRuntime (.ghost slot) (.ghost contents) = .ghost .unit := by
  rw [NTy.identityRuntime]

omit [Carriers] in
/-- A function value's identity is its runtime value with ghost contents
dropped. -/
@[simp] theorem NTy.identityRuntime_function (parameters : NRow)
    (shared : Sharing) (results : NRow) (value : RuntimeValue) :
    NTy.identityRuntime (.function parameters shared results) value = value.eraseGhosts := by
  cases value <;> rw [NTy.identityRuntime]

omit [Carriers] in
/-- A recursive nominal's identity is its runtime value with ghost contents dropped. -/
@[simp] theorem NTy.identityRuntime_nominal (source : StructHandle) (arguments : NRow)
    (productive : Bool) (value : RuntimeValue) :
    NTy.identityRuntime (.nominal source arguments productive) value = value.eraseGhosts := by
  cases value <;> rw [NTy.identityRuntime]

omit [Carriers] in
/-- A type parameter's value's identity drops its ghost contents. -/
@[simp] theorem NTy.identityRuntime_param (index : Nat) (value : RuntimeValue) :
    NTy.identityRuntime (.param index) value = value.eraseGhosts := by
  cases value <;> rw [NTy.identityRuntime]

omit [Carriers] in
/-- A scalar is its own identity. -/
@[simp] theorem NTy.identityRuntime_scalar (τ : NTy) (scalar : τ.isScalar = true)
    (value : RuntimeValue) : NTy.identityRuntime τ value = value := by
  cases τ <;> simp [NTy.isScalar] at scalar <;> cases value <;> simp [NTy.identityRuntime]

omit [Carriers] in
@[simp] theorem NRow.identityRuntimeAll_cons (τ : NTy) (rest : NRow)
    (value : RuntimeValue) (values : List RuntimeValue) :
    NRow.identityRuntimeAll (.cons τ rest) (value :: values) =
      NTy.identityRuntime τ value :: NRow.identityRuntimeAll rest values := by
  rw [NRow.identityRuntimeAll]

omit [Carriers] in
@[simp] theorem NRow.identityRuntimeAll_nil (values : List RuntimeValue) :
    NRow.identityRuntimeAll .nil values = values := by
  cases values <;> simp [NRow.identityRuntimeAll]

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

omit [Carriers] in
private theorem beq_eq_decide {α : Type} [BEq α] [LawfulBEq α] [DecidableEq α]
    (left right : α) : (left == right) = decide (left = right) := by
  by_cases h : left = right
  · subst h
    simp
  · simp [h]

omit [Carriers] in
theorem RuntimeValue.beq_unit : (RuntimeValue.unit == RuntimeValue.unit) = true := by
  show RuntimeValue.beq _ _ = true
  unfold RuntimeValue.beq
  rfl

omit [Carriers] in
theorem RuntimeValue.beq_signer (left right : String) :
    (RuntimeValue.signer left == RuntimeValue.signer right) = decide (left = right) := by
  show RuntimeValue.beq _ _ = _
  unfold RuntimeValue.beq
  exact beq_eq_decide _ _

omit [Carriers] in
theorem RuntimeValue.beq_string (left right : String) :
    (RuntimeValue.string left == RuntimeValue.string right) = decide (left = right) := by
  show RuntimeValue.beq _ _ = _
  unfold RuntimeValue.beq
  exact beq_eq_decide _ _

omit [Carriers] in
theorem RuntimeValue.beq_character (left right : Nat) :
    (RuntimeValue.character left == RuntimeValue.character right) = decide (left = right) := by
  show RuntimeValue.beq _ _ = _
  unfold RuntimeValue.beq
  exact beq_eq_decide _ _

omit [Carriers] in
theorem RuntimeValue.beq_bytes (left right : Array UInt8) :
    (RuntimeValue.bytes left == RuntimeValue.bytes right) = decide (left = right) := by
  show RuntimeValue.beq _ _ = _
  unfold RuntimeValue.beq
  exact beq_eq_decide _ _

omit [Carriers] in
mutual
/-- Structural equality of plain values at one type. -/
def NTy.plainEqbOf (F : PlainFamily) : (τ : NTy) → NTy.plainOf F τ → NTy.plainOf F τ → Bool
  | .unit, _, _ => true
  | .bool, left, right => @decide (left = right) (NTy.plainDecEqOf F .bool left right)
  | .int width signed, left, right =>
      @decide ((left : SpecInt (.bits width) signed).val = (right : SpecInt (.bits width) signed).val)
        inferInstance
  | .num, left, right =>
      @decide ((left : SpecInt .unbounded true).val = (right : SpecInt .unbounded true).val)
        inferInstance
  | .address, left, right => @decide (left = right) (NTy.plainDecEqOf F .address left right)
  | .signer, left, right => @decide (left = right) (NTy.plainDecEqOf F .signer left right)
  | .string, left, right => @decide (left = right) (NTy.plainDecEqOf F .string left right)
  | .bytes, left, right => @decide (left = right) (NTy.plainDecEqOf F .bytes left right)
  | .char, left, right => @decide (left = right) (NTy.plainDecEqOf F .char left right)
  | .tuple elements, left, right => rowEqbPlainOf F elements left right
  | .struct _ _ fields, left, right => rowEqbPlainOf F fields left right
  | .enum _ _ names rows _, left, right => variantEqbPlainOf F names rows left right
  | .vector element, left, right =>
      left.values.size == right.values.size &&
        (left.values.toList.zip right.values.toList).all fun pair => NTy.plainEqbOf F element pair.1 pair.2
  | .slice element, left, right =>
      left.values.size == right.values.size &&
        (left.values.toList.zip right.values.toList).all fun pair => NTy.plainEqbOf F element pair.1 pair.2
  | .array element _, left, right =>
      left.vector.values.size == right.vector.values.size &&
        (left.vector.values.toList.zip right.vector.values.toList).all fun pair =>
          NTy.plainEqbOf F element pair.1 pair.2
  | .table key value, left, right =>
      left.entries.length == right.entries.length &&
        (left.entries.zip right.entries).all fun pair =>
          NTy.plainEqbOf F key pair.1.1 pair.2.1 && NTy.plainEqbOf F value pair.1.2 pair.2.2
  -- No equality reads a ghost (`doc/designs/ghost-fields.md`).
  | .ghost _, _, _ => true
  | .sliceRef _, _, _ => false
  | .ref _, _, _ => false
  | .observed referent, left, right => NTy.plainEqbOf F referent left right
  | .param index, left, right => @decide (left = right) ((F.lent index).decEq left right)
  -- A function value's captures are compared as the runtime holds them.
  | .function parameters shared results, left, right =>
      (F.functions parameters shared results).erasures.same left right
  | .nominal source arguments productive, left, right =>
      (F.nominals source arguments productive).erasures.same left right
termination_by structural τ _ _ => τ

/-- Structural equality of rows, component by component. -/
def rowEqbPlainOf (F : PlainFamily) : (row : NRow) →
    HListPlainOf F row → HListPlainOf F row → Bool
  | .nil, _, _ => true
  | .cons τ rest, left, right =>
      NTy.plainEqbOf F τ left.1 right.1 && rowEqbPlainOf F rest left.2 right.2
termination_by structural row _ _ => row

/-- Structural equality of variant values: the same variant with equal rows. -/
def variantEqbPlainOf (F : PlainFamily) : (names : List String) → (rows : NRows) →
    variantPlainOf F names rows → variantPlainOf F names rows → Bool
  -- Read with `Sum.elim`, and a constant where no variant is left
  -- (`variantPlainOf`).
  | _, .nil, _, _ => true
  | [], .cons _ _, _, _ => true
  | _ :: names, .cons fields rest, left, right =>
      Sum.elim
        (fun leftRow => Sum.elim (rowEqbPlainOf F fields leftRow) (fun _ => false) right)
        (fun leftLater =>
          Sum.elim (fun _ => false) (variantEqbPlainOf F names rest leftLater) right)
        left
termination_by structural names rows _ _ => rows
end

/-- Structural equality of plain values at a family's lent layer. -/
def NTy.plainEqb (τ : NTy) (left right : τ.plain) : Bool :=
  NTy.plainEqbOf Carriers.toPlain τ left right

/-- Structural equality of plain rows at a family's lent layer. -/
def rowEqbPlain (row : NRow) (left right : HListPlain row) : Bool :=
  rowEqbPlainOf Carriers.toPlain row left right

/-- Structural equality of plain variant values at a family's lent layer. -/
def variantEqbPlain (names : List String) (rows : NRows)
    (left right : variantPlain names rows) : Bool :=
  variantEqbPlainOf Carriers.toPlain names rows left right

mutual
/-- Native structural equality at one type: the value a clause compares. -/
def NTy.eqb : (τ : NTy) → τ.carrier → τ.carrier → Bool
  | .unit, _, _ => true
  | .bool, left, right => @decide (left = right) (NTy.decEq .bool left right)
  | .int width signed, left, right =>
      @decide ((left : SpecInt (.bits width) signed).val = (right : SpecInt (.bits width) signed).val)
        inferInstance
  | .num, left, right =>
      @decide ((left : SpecInt .unbounded true).val = (right : SpecInt .unbounded true).val)
        inferInstance
  | .address, left, right => @decide (left = right) (NTy.decEq .address left right)
  | .signer, left, right => @decide (left = right) (NTy.decEq .signer left right)
  | .string, left, right => @decide (left = right) (NTy.decEq .string left right)
  | .bytes, left, right => @decide (left = right) (NTy.decEq .bytes left right)
  | .char, left, right => @decide (left = right) (NTy.decEq .char left right)
  | .tuple elements, left, right => rowEqb elements left right
  | .struct _ _ fields, left, right => rowEqbPlain fields left.val right.val
  | .enum _ _ names rows _, left, right => variantEqbPlain names rows left.val right.val
  | .slice element, left, right =>
      left.values.size == right.values.size &&
        (left.values.toList.zip right.values.toList).all fun pair => element.eqb pair.1 pair.2

  | .vector element, left, right =>
      left.values.size == right.values.size &&
        (left.values.toList.zip right.values.toList).all fun pair => element.eqb pair.1 pair.2
  | .array element _, left, right =>
      left.vector.values.size == right.vector.values.size &&
        (left.vector.values.toList.zip right.vector.values.toList).all fun pair =>
          element.eqb pair.1 pair.2
  | .table key value, left, right =>
      left.entries.length == right.entries.length &&
        (left.entries.zip right.entries).all fun pair =>
          key.eqb pair.1.1 pair.2.1 && value.eqb pair.1.2 pair.2.2
  -- No equality reads a ghost (`doc/designs/ghost-fields.md`).
  | .ghost _, _, _ => true
  | .sliceRef _, _, _ => false
  | .ref _, _, _ => false
  | .param index, left, right => @decide (left = right) (Carriers.decEq index left right)
  -- A function value's captures are compared as the runtime holds them.
  | .function parameters shared results, left, right =>
      (Carriers.functions parameters shared results).erasures.same left right
  | .observed referent, left, right => NTy.eqb referent left right
  | .nominal source arguments productive, left, right =>
      (Carriers.nominals source arguments productive).erasures.same left right

/-- Structural equality of rows, component by component. -/
def rowEqb : (row : NRow) → HList row → HList row → Bool
  | .nil, _, _ => true
  | .cons τ rest, left, right => τ.eqb left.1 right.1 && rowEqb rest left.2 right.2

/-- Structural equality of variant values: the same variant with equal rows. -/
def variantEqb : (names : List String) → (rows : NRows) →
    variantCarrier names rows → variantCarrier names rows → Bool
  -- Read with `Sum.elim`, and a constant where no variant is left
  -- (`variantPlainOf`).
  | _, .nil, _, _ => true
  | [], .cons _ _, _, _ => true
  | _ :: names, .cons fields rest, left, right =>
      Sum.elim
        (fun leftRow => Sum.elim (rowEqb fields leftRow) (fun _ => false) right)
        (fun leftLater =>
          Sum.elim (fun _ => false) (variantEqb names rest leftLater) right)
        left
end

@[simp] theorem NTy.eqb_tuple (elements : NRow) (left right : HList elements) :
    NTy.eqb (.tuple elements) left right = rowEqb elements left right := rfl
@[simp] theorem NTy.eqb_struct (source : StructHandle) (arguments fields : NRow)
    (left right : (NTy.struct source arguments fields).carrier) :
    NTy.eqb (.struct source arguments fields) left right = rowEqbPlain fields left.val right.val :=
  rfl
@[simp] theorem NTy.eqb_enum (source : StructHandle) (arguments : NRow) (names : List String)
    (rows : NRows) (distinct : names.Nodup)
    (left right : (NTy.enum source arguments names rows distinct).carrier) :
    NTy.eqb (.enum source arguments names rows distinct) left right =
      variantEqbPlain names rows left.val right.val := rfl
@[simp] theorem NTy.eqb_array (element : NTy) {length : Nat}
    (left right : SpecArray element.carrier length) :
    NTy.eqb (.array element length) left right =
      (left.vector.values.size == right.vector.values.size &&
        (left.vector.values.toList.zip right.vector.values.toList).all fun pair =>
          element.eqb pair.1 pair.2) := rfl
@[simp] theorem NTy.eqb_slice (element : NTy) (left right : SpecVector element.carrier) :
    NTy.eqb (.slice element) left right =
      (left.values.size == right.values.size &&
        (left.values.toList.zip right.values.toList).all fun pair => element.eqb pair.1 pair.2) := rfl

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

@[simp] theorem NTy.plainEqb_vector (element : NTy) (left right : SpecVector element.plain) :
    NTy.plainEqb (.vector element) left right =
      (left.values.size == right.values.size &&
        (left.values.toList.zip right.values.toList).all fun pair => element.plainEqb pair.1 pair.2) := rfl
@[simp] theorem NTy.plainEqb_slice (element : NTy) (left right : SpecVector element.plain) :
    NTy.plainEqb (.slice element) left right =
      (left.values.size == right.values.size &&
        (left.values.toList.zip right.values.toList).all fun pair => element.plainEqb pair.1 pair.2) := rfl
@[simp] theorem NTy.plainEqb_array (element : NTy) {length : Nat}
    (left right : SpecArray element.plain length) :
    NTy.plainEqb (.array element length) left right =
      (left.vector.values.size == right.vector.values.size &&
        (left.vector.values.toList.zip right.vector.values.toList).all fun pair =>
          element.plainEqb pair.1 pair.2) := rfl
@[simp] theorem rowEqbPlain_nil (left right : HListPlain .nil) : rowEqbPlain .nil left right = true := rfl
@[simp] theorem rowEqbPlain_cons (τ : NTy) (rest : NRow) (left right : HListPlain (.cons τ rest)) :
    rowEqbPlain (.cons τ rest) left right = (τ.plainEqb left.1 right.1 && rowEqbPlain rest left.2 right.2) := rfl
@[simp] theorem variantEqbPlain_inl_inl (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (left right : HListPlain fields) :
    variantEqbPlain (name :: names) (.cons fields rest) (.inl left) (.inl right) =
      rowEqbPlain fields left right := rfl
@[simp] theorem variantEqbPlain_inr_inr (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (left right : variantPlain names rest) :
    variantEqbPlain (name :: names) (.cons fields rest) (.inr left) (.inr right) =
      variantEqbPlain names rest left right := rfl
@[simp] theorem variantEqbPlain_inl_inr (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (left : HListPlain fields) (right : variantPlain names rest) :
    variantEqbPlain (name :: names) (.cons fields rest) (.inl left) (.inr right) = false := rfl
@[simp] theorem variantEqbPlain_inr_inl (name : String) (names : List String) (fields : NRow)
    (rest : NRows) (left : variantPlain names rest) (right : HListPlain fields) :
    variantEqbPlain (name :: names) (.cons fields rest) (.inr left) (.inl right) = false := rfl

/-- Whether a name is among the tested names. -/
def namedIn (name : String) : List String → Bool
  | [] => false
  | test :: rest => name == test || namedIn name rest

omit [Carriers] in
@[simp] theorem namedIn_nil (name : String) : namedIn name [] = false := rfl
omit [Carriers] in
@[simp] theorem namedIn_cons (name test : String) (rest : List String) :
    namedIn name (test :: rest) = (name == test || namedIn name rest) := rfl

/-- At a scalar type, runtime structural equality of encodings is the
native decision. -/
theorem NTy.beq_encode (τ : NTy) (scalar : τ.isScalar = true) (left right : τ.carrier) :
    (τ.encode left == τ.encode right) = τ.eqb left right := by
  cases τ <;> simp [NTy.isScalar] at scalar <;> simp [NTy.eqb, RuntimeValue.beq_unit,
    RuntimeValue.beq_signer, RuntimeValue.beq_string, RuntimeValue.beq_bytes,
    RuntimeValue.beq_character, SpecChar.eq_iff_val] <;>
    exact Iff.rfl

@[simp] theorem NTy.eqb_char (left right : SpecChar) :
    NTy.eqb .char left right = decide (left = right) := rfl
@[simp] theorem NTy.eqb_int (width : Nat) (signed : Bool)
    (left right : SpecInt (.bits width) signed) :
    NTy.eqb (.int width signed) left right = decide (left.val = right.val) := rfl
@[simp] theorem NTy.eqb_num (left right : SpecInt .unbounded true) :
    NTy.eqb .num left right = decide (left.val = right.val) := rfl
@[simp] theorem NTy.eqb_bool (left right : Bool) :
    NTy.eqb .bool left right = decide (left = right) := rfl
@[simp] theorem NTy.eqb_address (left right : String) :
    NTy.eqb .address left right = decide (left = right) := rfl
@[simp] theorem NTy.eqb_unit (left right : Unit) : NTy.eqb .unit left right = true := rfl
@[simp] theorem NTy.eqb_param (index : Nat) (left right : (NTy.param index).carrier) :
    NTy.eqb (.param index) left right = @decide (left = right) (Carriers.decEq index left right) :=
  rfl
@[simp] theorem NTy.eqb_function (parameters results : NRow)
    (left right : (NTy.function parameters shared results).carrier) :
    NTy.eqb (.function parameters shared results) left right =
      decide ((Carriers.functions parameters shared results).erased left =
        (Carriers.functions parameters shared results).erased right) := rfl

@[simp] theorem NTy.eqb_nominal (source : StructHandle) (arguments : NRow) (productive : Bool)
    (left right : (NTy.nominal source arguments productive).carrier) :
    NTy.eqb (.nominal source arguments productive) left right =
      decide (((Carriers.nominals source arguments productive).value left).eraseGhosts =
        ((Carriers.nominals source arguments productive).value right).eraseGhosts) := rfl
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
noncomputable def HEnv.encode : {Γ : NRow} → HEnv Γ → List (Option RuntimeValue)
  | .nil, _ => []
  | .cons τ _, env => env.1.map τ.encode :: HEnv.encode env.2

/-- The runtime frame an environment denotes: its locals, and no loan. -/
noncomputable def envFrame {Γ : NRow} (env : HEnv Γ) : RuntimeFrame :=
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
      simp only [readLocal?, envFrame, HEnv.encode, Var.index, List.getElem?_toArray, List.getElem?_cons_zero, Var.get_here]
      cases env.1 <;> rfl
  | _, _, .there rest, env => by
      have ih := readLocal?_envFrame rest env.2
      simp only [readLocal?, envFrame, HEnv.encode, Var.index, List.getElem?_toArray, List.getElem?_cons_succ, Var.get_there] at ih ⊢
      exact ih

omit [Carriers] in
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
noncomputable def hlistCodec (Γ : NRow) : Codec (HList Γ) (Array RuntimeValue) where
  encode := fun values => values.encode.toArray
  decode? := fun values => (rowCodec Γ).decode? values.toList
  decode_encode := by
    intro values
    simp [HList.encode, (rowCodec Γ).decode_encode]

/-- The declared results of a function: none, or one native value. A
tuple result stays one tuple value, including at a function-value call. -/
inductive ResultShape where
  | none
  | one (τ : NTy)
  deriving DecidableEq, Repr, Inhabited

@[reducible] def ResultShape.carrier : ResultShape → Type
  | .none => Unit
  | .one τ => τ.carrier

/-- The type the body of a function must produce. Reducible, as
`ResultShape.carrier` is: a callee's result type reaches instance arguments
(a lookup's `GetElem?` instance), which simp matches at instance
transparency only. -/
@[reducible] def ResultShape.bodyType : ResultShape → NTy
  | .none => .unit
  | .one τ => τ

/-- The result row of a function type returning a shape. -/
@[reducible] def ResultShape.row : ResultShape → NRow
  | .none => .nil
  | .one τ => .cons τ .nil

/-- The result-row codec of a function. -/
noncomputable def resultCodec : (shape : ResultShape) → Codec shape.carrier (Array RuntimeValue)
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

/-- The initial locals of a body over its frame, the row its arguments and
remaining locals spell: its arguments, then uninitialized slots. -/
def initialEnv : (params rest : NRow) → {frame : NRow} → params ++ rest = frame →
    HList params → HEnv frame
  | .nil, .nil, .nil, _, _ => ()
  | .nil, .nil, .cons _ _, framed, _ => nomatch framed
  | .nil, .cons _ _, .nil, framed, _ => nomatch framed
  | .nil, .cons _ rest, .cons _ _, framed, _ =>
      (none, initialEnv .nil rest (NRow.cons.inj framed).2 ())
  | .cons _ _, _, .nil, framed, _ => nomatch framed
  | .cons _ params, rest, .cons _ _, framed, values =>
      (some ((NRow.cons.inj framed).1 ▸ values.1), initialEnv params rest (NRow.cons.inj framed).2 values.2)

/-! The equations at a frame spelled as its row: no cast remains. -/

theorem initialEnv_nil (framed : NRow.nil ++ NRow.nil = NRow.nil) (values : HList .nil) :
    initialEnv .nil .nil framed values = () := rfl

theorem initialEnv_nil_cons {τ : NTy} {rest frame : NRow}
    (framed : NRow.nil ++ NRow.cons τ rest = .cons τ frame) (values : HList .nil) :
    initialEnv .nil (.cons τ rest) framed values =
      (none, initialEnv .nil rest (NRow.cons.inj framed).2 ()) := rfl

theorem initialEnv_cons {τ : NTy} {params rest frame : NRow}
    (framed : NRow.cons τ params ++ rest = .cons τ frame) (values : HList (.cons τ params)) :
    initialEnv (.cons τ params) rest framed values =
      (some values.1, initialEnv params rest (NRow.cons.inj framed).2 values.2) := rfl

end Carriers

section Memory
variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]

/-! ## Native operations

Each operation is the `Spec` a checked primitive denotes on native
carriers, and each has one weakest-precondition rule.  Failures carry the
runtime payload the primitive throws, so a declared abort code is checked
exactly. -/

/-! ## Global memory

Global memory is typed (`doc/designs/static-memory.md`): one partial map per
resource type, holding values at the runtime family, which every frame of a
verification shares.  Nothing decodes a stored value. -/

/-- Ordinary stored values retain their language type's bounds. Native
collections hold an arbitrary finite array of typed elements: their storage
is not a Move vector and imposes no vector length bound. -/
inductive ResourceKind where
  | value
  | collection
  deriving DecidableEq, Repr, Inhabited

/-- A resource type: the stored value's (or collection element's) native type
at the runtime family, its scope including phantom arguments, and storage kind. -/
structure ResourceType where
  type : NTy
  arguments : NRow
  kind : ResourceKind := .value
  deriving DecidableEq, Repr, Inhabited

/-- Instantiate a resource's native types without changing its storage domain. -/
@[reducible] def ResourceType.subst (resource : ResourceType) (types : NRow) : ResourceType :=
  { resource with
    type := resource.type.subst types
    arguments := NRow.subst types resource.arguments }

/-- The values stored at a resource type. -/
@[reducible] def ResourceType.carrier (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (resource : ResourceType) :
    Type :=
  match resource.kind with
  | .value => @NTy.carrier (Carriers.runtime unit) resource.type
  | .collection => Array (@NTy.carrier (Carriers.runtime unit) resource.type)

/-- The runtime representation of a typed storage slot. Native collections
use the runtime's unbounded array representation without a Move vector codec. -/
@[reducible] noncomputable def ResourceType.encode (resource : ResourceType)
    (value : resource.carrier unit) : RuntimeValue :=
  match resource with
  | ⟨type, _, .value⟩ => @NTy.encode (Carriers.runtime unit) type value
  | ⟨type, _, .collection⟩ =>
      .vector (value.map (@NTy.encode (Carriers.runtime unit) type))

theorem ResourceType.encode_injective (resource : ResourceType) :
    Function.Injective (resource.encode (unit := unit)) := by
  rcases resource with ⟨type, arguments, kind⟩
  cases kind with
  | value => exact @NTy.encode_injective (Carriers.runtime unit) type
  | collection =>
      intro left right equal
      have mapped := RuntimeValue.vector.inj equal
      exact (Array.map_inj_right fun _ _ =>
        @NTy.encode_injective (Carriers.runtime unit) type _ _).mp mapped

/-- Global memory: per resource type, the value stored under each key. -/
def Memory (unit : Validation.ValidatedUnit) [FunctionCarriers unit] : Type :=
  (resource : ResourceType) → StorageKey → Option (resource.carrier unit)

/-- The memory with one slot replaced. -/
def Memory.set (memory : Memory unit) (resource : ResourceType) (key : StorageKey)
    (value : Option (resource.carrier unit)) : Memory unit :=
  fun other otherKey =>
    if same : other = resource then
      if otherKey = key then same ▸ value else memory other otherKey
    else memory other otherKey

@[simp] theorem Memory.set_same (memory : Memory unit) (resource : ResourceType)
    (key otherKey : StorageKey)
    (value : Option (resource.carrier unit)) :
    memory.set resource key value resource otherKey =
      if otherKey = key then value else memory resource otherKey := by
  simp only [Memory.set, dite_true]

@[simp] theorem Memory.set_other (memory : Memory unit) {resource other : ResourceType}
    (key otherKey : StorageKey) (value : Option (resource.carrier unit))
    (distinct : other ≠ resource) :
    memory.set resource key value other otherKey = memory other otherKey := by
  simp only [Memory.set, distinct, dite_false]

/-- The memory with one slot replaced leaves every other resource type's
slots as they were. -/
@[simp] theorem Memory.set_other_resource (memory : Memory unit) {resource other : ResourceType}
    (key : StorageKey) (value : Option (resource.carrier unit)) (distinct : other ≠ resource) :
    memory.set resource key value other = memory other := by
  funext otherKey
  exact Memory.set_other memory key otherKey value distinct

/-- A slot read after a write at an undecided key holds a value either way. -/
@[simp] theorem ite_some_some {α : Type} {condition : Prop} [Decidable condition] (left right : α) :
    (if condition then some left else some right) = some (if condition then left else right) := by
  split <;> rfl

/-- Whether a slot read after a write at an undecided key holds a value,
by the key. -/
@[simp] theorem isSome_ite {α : Type} {condition : Prop} [Decidable condition]
    (left right : Option α) :
    (if condition then left else right).isSome =
      if condition then left.isSome else right.isSome := by
  split <;> rfl

/-- A condition that holds where `p` does: a case split rather than an
implication. -/
theorem ite_true_or {p q : Prop} [Decidable p] : (if p then True else q) ↔ p ∨ q := by
  by_cases h : p <;> simp [h]

/-- An array that is not empty has an element. -/
theorem array_isEmpty_eq_false {α : Type} {xs : Array α} :
    xs.isEmpty = false ↔ 0 < xs.size := by
  rw [← Bool.not_eq_true, Array.isEmpty_iff_size_eq_zero]
  omega

/-- An unsigned element a read finds, or none, as an integer: within the
element's width. -/
theorem asInt_getD_map_unsigned {width : Nat} (element : Option (SpecInt (.bits width) false)) :
    0 ≤ ((element.map (Codec.specInt (.bits width) false).encode).getD .unit).asInt ∧
      ((element.map (Codec.specInt (.bits width) false).encode).getD .unit).asInt ≤
        2 ^ width - 1 := by
  cases element with
  | none =>
      have positive : (1 : Int) ≤ 2 ^ width := by
        have := Nat.one_le_two_pow (n := width)
        exact_mod_cast this
      simp only [Option.map_none, Option.getD_none, RuntimeValue.asInt]
      omega
  | some value =>
      simpa only [Option.map_some, Option.getD_some, Codec.specInt, RuntimeValue.asInt]
        using SpecInt.unsigned_bounds value

/-- An unsigned integer an entry a read finds holds, or none, as an
integer: within its width. -/
theorem asInt_getD_map_integer_unsigned {α : Type} {width : Nat}
    (read : α → SpecInt (.bits width) false) (entry : Option α) :
    0 ≤ ((entry.map fun value => RuntimeValue.integer (read value).val).getD .unit).asInt ∧
      ((entry.map fun value => RuntimeValue.integer (read value).val).getD .unit).asInt ≤
        2 ^ width - 1 := by
  cases entry with
  | none =>
      have positive : (1 : Int) ≤ 2 ^ width := by
        have := Nat.one_le_two_pow (n := width)
        exact_mod_cast this
      simp only [Option.map_none, Option.getD_none, RuntimeValue.asInt]
      omega
  | some value =>
      simpa only [Option.map_some, Option.getD_some, RuntimeValue.asInt]
        using SpecInt.unsigned_bounds (read value)

/-- A read of a sequence of integers, as an integer: the read of the
sequence of their values. -/
theorem asInt_getD_getElem?_map_integer (values : List Int) (index : Nat) :
    (((values.map RuntimeValue.integer)[index]?).getD .unit).asInt = (values[index]?).getD 0 := by
  rw [List.getElem?_map]
  cases values[index]? <;> rfl

/-- An encoded read of a sequence of certified integers, as an integer: the
read of the sequence of their values. -/
theorem asInt_getD_map_encode_getElem? {width : IntWidth} {signed : Bool}
    (elements : List (SpecInt width signed)) (index : Nat) :
    ((elements[index]?.map (Codec.specInt width signed).encode).getD .unit).asInt =
      ((elements.map SpecInt.val)[index]?).getD 0 := by
  rw [List.getElem?_map]
  cases elements[index]? <;> rfl

/-- The value of an element of a sequence of certified integers: the read of
the sequence of their values. -/
theorem val_getElem_eq_getD_map_val {width : IntWidth} {signed : Bool}
    (elements : List (SpecInt width signed)) (index : Nat) (inBounds : index < elements.length) :
    (elements[index]'inBounds).val = ((elements.map SpecInt.val)[index]?).getD 0 := by
  rw [List.getElem?_map, List.getElem?_eq_getElem inBounds]
  rfl

/-- An optional certified read preserves the supplied fallback when projecting
to integers; no in-bounds premise is needed. -/
theorem val_getD_getElem?_map_val {width : IntWidth} {signed : Bool}
    (elements : List (SpecInt width signed)) (index : Nat)
    (fallback : SpecInt width signed) :
    (elements[index]?.getD fallback).val =
      ((elements.map SpecInt.val)[index]?).getD fallback.val := by
  rw [List.getElem?_map]
  cases elements[index]? <;> rfl

/-- A position an array reads a value at lies within it. -/
theorem lt_size_of_getElem?_eq_some {α : Type} {xs : Array α} {i : Nat} {value : α}
    (read : xs[i]? = some value) : i < xs.size :=
  (Array.getElem?_eq_some_iff.mp read).1

instance : Inhabited (Memory unit) := ⟨fun _ _ => none⟩

/-- A `Spec` over global memory with VIR failures. -/
abbrev Comp (unit : Validation.ValidatedUnit) [FunctionCarriers unit] (α : Type) :=
  Spec (Memory unit) Failure α

end Memory

section Operations
variable {σ : Type}

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

@[vir_denote_side] theorem IntegerValueFits_unsigned_succ (n : Nat) (value : Int) :
    IntegerValueFits (.bits (n + 1)) false value ↔ 0 ≤ value ∧ value ≤ 2 ^ (n + 1) - 1 := by
  simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]

@[vir_denote_side] theorem IntegerValueFits_signed_succ (n : Nat) (value : Int) :
    IntegerValueFits (.bits (n + 1)) true value ↔
      -(2 : Int) ^ n ≤ value ∧ value ≤ 2 ^ n - 1 := by
  simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]

/-! A specification integer cut to a fixed width (`SpecInt.ofInt`) is the
integer where it fits, with the bounds stated as the normalization's
discharger decides them from a leaf's facts. -/

@[vir_denote_norm] theorem SpecInt.val_ofInt_unsigned (n : Nat)
    (default : SpecInt (.bits (n + 1)) false) {value : Int} (low : 0 ≤ value)
    (high : value ≤ 2 ^ (n + 1) - 1) : (SpecInt.ofInt default value).val = value :=
  SpecInt.val_ofInt default ((IntegerValueFits_unsigned_succ n value).mpr ⟨low, high⟩)

@[vir_denote_norm] theorem SpecInt.val_ofInt_signed (n : Nat)
    (default : SpecInt (.bits (n + 1)) true) {value : Int} (low : -(2 : Int) ^ n ≤ value)
    (high : value ≤ 2 ^ n - 1) : (SpecInt.ofInt default value).val = value :=
  SpecInt.val_ofInt default ((IntegerValueFits_signed_succ n value).mpr ⟨low, high⟩)

@[vir_denote_norm] theorem SpecInt.val_ofInt_unbounded (default : SpecInt .unbounded true)
    (value : Int) : (SpecInt.ofInt default value).val = value :=
  SpecInt.val_ofInt default (by simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?])

/-! A certified integer a clause defines by its value (`∃ x, x.val = e ∧ p x`)
is a certified integer of that value at hand, or the value where it fits,
with the bounds stated as for `SpecInt.ofInt`. -/

theorem SpecInt.exists_val_eq {width : IntWidth} {signed : Bool} {value : Int}
    {p : SpecInt width signed → Prop} (x : SpecInt width signed) (named : x.val = value) :
    (∃ c : SpecInt width signed, c.val = value ∧ p c) ↔ p x :=
  ⟨fun ⟨c, defined, held⟩ => SpecInt.ext (defined.trans named.symm) ▸ held,
    fun held => ⟨x, named, held⟩⟩

@[vir_denote_norm] theorem SpecInt.exists_val {width : IntWidth} {signed : Bool}
    {p : SpecInt width signed → Prop} (x : SpecInt width signed) :
    (∃ c : SpecInt width signed, c.val = x.val ∧ p c) ↔ p x :=
  SpecInt.exists_val_eq x rfl

@[vir_denote_norm] theorem SpecInt.exists_unbounded {value : Int}
    {p : SpecInt .unbounded true → Prop} :
    (∃ c : SpecInt .unbounded true, c.val = value ∧ p c) ↔
      p ⟨value, by simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]⟩ :=
  SpecInt.exists_val_eq _ rfl

/-- A certified integer of a value exists where the value fits. -/
@[vir_denote_norm] theorem SpecInt.exists_val_unsigned (n : Nat) {value : Int} :
    (∃ c : SpecInt (.bits (n + 1)) false, c.val = value) ↔
      0 ≤ value ∧ value ≤ 2 ^ (n + 1) - 1 :=
  ⟨fun ⟨c, defined⟩ => (IntegerValueFits_unsigned_succ n value).mp (defined ▸ c.fits),
    fun bounds => ⟨⟨value, (IntegerValueFits_unsigned_succ n value).mpr bounds⟩, rfl⟩⟩

@[vir_denote_norm] theorem SpecInt.exists_val_signed (n : Nat) {value : Int} :
    (∃ c : SpecInt (.bits (n + 1)) true, c.val = value) ↔
      -(2 : Int) ^ n ≤ value ∧ value ≤ 2 ^ n - 1 :=
  ⟨fun ⟨c, defined⟩ => (IntegerValueFits_signed_succ n value).mp (defined ▸ c.fits),
    fun bounds => ⟨⟨value, (IntegerValueFits_signed_succ n value).mpr bounds⟩, rfl⟩⟩

@[vir_denote_norm] theorem SpecInt.exists_val_unbounded {value : Int} :
    (∃ c : SpecInt .unbounded true, c.val = value) ↔ True :=
  iff_true_intro ⟨⟨value, by simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]⟩, rfl⟩

/-! A quantifier over certified integers, as a clause quantifies over an
integer type: over the integers its type admits, each certified by its
membership. -/

theorem SpecInt.forall_mk {width : IntWidth} {signed : Bool} (Q : SpecInt width signed → Prop) :
    (∀ x : SpecInt width signed, Q x) ↔
      ∀ value : Int, (fits : IntegerValueFits width signed value) → Q ⟨value, fits⟩ :=
  ⟨fun held value fits => held ⟨value, fits⟩, fun held x => held x.val x.fits⟩

theorem SpecInt.forall_mk_unbounded (Q : SpecInt .unbounded true → Prop) :
    (∀ x : SpecInt .unbounded true, Q x) ↔ ∀ value : Int, Q ⟨value, rfl⟩ :=
  ⟨fun held value => held ⟨value, rfl⟩, fun held x => held x.val⟩

/-- The length of a bounded vector, which fits `u64` by its bound. -/
def vectorLength {α : Type} (vector : SpecVector α) : SpecInt (.bits 64) false :=
  ⟨vector.values.size, (IntegerValueFits_unsigned_succ 63 _).mpr
    ⟨Int.natCast_nonneg _, by have := vector.bounded; omega⟩⟩

@[simp] theorem vectorLength_val {α : Type} (vector : SpecVector α) :
    (vectorLength vector).val = vector.values.size := rfl

/-- An array as a bounded vector, when its size is below the bound. -/
def _root_.VerexIR.SpecVector.ofArray? {α : Type} (values : Array α) : Option (SpecVector α) :=
  if bounded : values.size < 2 ^ 64 then some ⟨values, bounded⟩ else none

theorem _root_.VerexIR.SpecVector.ofArray?_eq {α : Type} (values : Array α) :
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

/-- A write, slice, extension, or concatenation of images of arrays is the
image of the array it makes. -/
theorem setIfInBounds_map {α β : Type} (f : α → β) (xs : Array α) (i : Nat) (a : α) :
    (xs.map f).setIfInBounds i (f a) = (xs.setIfInBounds i a).map f :=
  Array.map_setIfInBounds.symm

theorem extract_map {α β : Type} (f : α → β) (xs : Array α) (i j : Nat) :
    (xs.map f).extract i j = (xs.extract i j).map f :=
  Array.map_extract.symm

theorem push_map {α β : Type} (f : α → β) (xs : Array α) (a : α) :
    (xs.map f).push (f a) = (xs.push a).map f :=
  Array.map_push.symm

theorem append_map {α β : Type} (f : α → β) (xs ys : Array α) :
    xs.map f ++ ys.map f = (xs ++ ys).map f :=
  Array.map_append.symm

/-- A range reversal in closed form: the elements of `[left, right)` in
reverse order, the others in place; the array itself when the range passes
its end. -/
def reverseSlice {α : Type} (elements : Array α) (left right : Nat) : Array α :=
  if right ≤ elements.size then
    elements.mapIdx fun index element =>
      if left ≤ index ∧ index < right then elements[left + right - 1 - index]?.getD element
      else element
  else elements

private theorem getElem?_swapIfInBounds_of_lt {α : Type} {elements : Array α} {i j k : Nat}
    (hi : i < elements.size) (hj : j < elements.size) :
    (elements.swapIfInBounds i j)[k]? =
      if k = i then elements[j]? else if k = j then elements[i]? else elements[k]? := by
  by_cases hk : k < elements.size
  · rw [Array.getElem?_eq_getElem (by simpa using hk), Array.getElem_swapIfInBounds]
    by_cases ki : k = i
    · subst ki; simp [hj]
    · by_cases kj : k = j
      · subst kj; simp [ki, hi]
      · simp [ki, kj, hk]
  · rw [Array.getElem?_eq_none (by simpa using hk)]
    have ki : k ≠ i := by omega
    have kj : k ≠ j := by omega
    simp [ki, kj, hk]

private theorem getElem?_reverseRange {α : Type} (count left right : Nat) (elements : Array α)
    (index : Nat) (within : right < elements.size) (fits : left + 2 * count ≤ right + 1) :
    (reverseRange count left right elements)[index]? =
      if (left ≤ index ∧ index < left + count) ∨ (right + 1 ≤ index + count ∧ index ≤ right)
      then elements[left + right - index]? else elements[index]? := by
  induction count generalizing left right elements with
  | zero =>
      have : ¬((left ≤ index ∧ index < left + 0) ∨ (right + 1 ≤ index + 0 ∧ index ≤ right)) := by
        omega
      show elements[index]? = _
      rw [if_neg this]
  | succ count ih =>
      simp only [reverseRange]
      rw [ih (left + 1) (right - 1) _ (by simp; omega) (by omega)]
      have left_within : left < elements.size := by omega
      simp only [getElem?_swapIfInBounds_of_lt (i := left) (j := right) left_within within]
      by_cases outer : (left ≤ index ∧ index < left + (count + 1)) ∨
          (right + 1 ≤ index + (count + 1) ∧ index ≤ right)
      · rw [if_pos outer]
        by_cases inner : (left + 1 ≤ index ∧ index < left + 1 + count) ∨
            (right - 1 + 1 ≤ index + count ∧ index ≤ right - 1)
        · rw [if_pos inner]
          have m1 : left + 1 + (right - 1) - index ≠ left := by omega
          have m2 : left + 1 + (right - 1) - index ≠ right := by omega
          rw [if_neg m1, if_neg m2]
          congr 1; omega
        · rw [if_neg inner]
          by_cases il : index = left
          · subst il; simp
          · have ir : index = right := by omega
            subst ir; simp [il]
      · rw [if_neg outer]
        have inner : ¬((left + 1 ≤ index ∧ index < left + 1 + count) ∨
            (right - 1 + 1 ≤ index + count ∧ index ≤ right - 1)) := by omega
        rw [if_neg inner]
        have il : index ≠ left := by omega
        have ir : index ≠ right := by omega
        simp [il, ir]

theorem size_reverseSlice {α : Type} (elements : Array α) (left right : Nat) :
    (reverseSlice elements left right).size = elements.size := by
  unfold reverseSlice; split <;> simp

theorem getElem?_reverseSlice {α : Type} (elements : Array α) (left right index : Nat) :
    (reverseSlice elements left right)[index]? =
      if left ≤ index ∧ index < right ∧ right ≤ elements.size
      then elements[left + right - 1 - index]? else elements[index]? := by
  unfold reverseSlice
  by_cases within : right ≤ elements.size
  · simp only [within, if_true, Array.getElem?_mapIdx, and_true]
    by_cases range : left ≤ index ∧ index < right
    · have bound : index < elements.size := by omega
      have mirror : left + right - 1 - index < elements.size := by omega
      rw [Array.getElem?_eq_getElem bound, Array.getElem?_eq_getElem mirror]
      simp [range]
    · rw [if_neg range]
      cases elements[index]? <;> simp [range]
  · simp [within]

/-- A range reversed twice is the array. -/
theorem reverseSlice_reverseSlice {α : Type} (elements : Array α) (left right : Nat) :
    reverseSlice (reverseSlice elements left right) left right = elements := by
  apply Array.ext_getElem?
  intro index
  simp only [getElem?_reverseSlice, size_reverseSlice]
  by_cases range : left ≤ index ∧ index < right ∧ right ≤ elements.size
  · rw [if_pos range, if_pos (by omega)]
    congr 1; omega
  · rw [if_neg range, if_neg range]

/-- The closed form is the runtime's swap schedule on a range within the
array. -/
theorem reverseRange_eq_reverseSlice {α : Type} (elements : Array α) (left right : Nat)
    (ordered : left ≤ right) (within : right ≤ elements.size) :
    reverseRange ((right - left) / 2) left (right - 1) elements =
      reverseSlice elements left right := by
  apply Array.ext_getElem?
  intro index
  rw [getElem?_reverseSlice]
  by_cases short : right - left < 2
  · have : (right - left) / 2 = 0 := by omega
    rw [this]
    simp only [reverseRange]
    by_cases range : left ≤ index ∧ index < right ∧ right ≤ elements.size
    · rw [if_pos range]; congr 1; omega
    · rw [if_neg range]
  · rw [getElem?_reverseRange _ _ _ _ _ (by omega) (by omega)]
    by_cases range : left ≤ index ∧ index < right ∧ right ≤ elements.size
    · rw [if_pos range]
      by_cases middle : (left ≤ index ∧ index < left + (right - left) / 2) ∨
          (right - 1 + 1 ≤ index + (right - left) / 2 ∧ index ≤ right - 1)
      · rw [if_pos middle]; congr 1; omega
      · rw [if_neg middle]; congr 1; omega
    · rw [if_neg range]
      have : ¬((left ≤ index ∧ index < left + (right - left) / 2) ∨
          (right - 1 + 1 ≤ index + (right - left) / 2 ∧ index ≤ right - 1)) := by omega
      rw [if_neg this]

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

/-- A lookup below the size of a pushed array reads the array before the
push, as a lookup. -/
theorem getElem?_push_of_lt {α : Type} (xs : Array α) (a : α) (i : Nat) (h : i < xs.size) :
    (xs.push a)[i]? = xs[i]? := by
  rw [Array.getElem?_push, if_neg (Nat.ne_of_lt h)]

private theorem findIndex?_eq_some_from {α : Type} (eq : α → α → Bool) (elements : Array α) (needle : α) :
    ∀ (count index i : Nat), index + count = elements.size →
      (findIndex? eq elements needle count index = some i ↔
        index ≤ i ∧ i < elements.size ∧ (∃ x, elements[i]? = some x ∧ eq x needle = true) ∧
          ∀ j, index ≤ j → j < i → ∀ x, elements[j]? = some x → eq x needle = false) := by
  intro count
  induction count with
  | zero =>
      intro index i sum
      simp only [findIndex?, reduceCtorEq, false_iff]
      rintro ⟨low, high, -, -⟩
      omega
  | succ n ih =>
      intro index i sum
      have inBounds : index < elements.size := by omega
      simp only [findIndex?, Array.getElem?_eq_getElem inBounds, Option.any_some]
      split
      · rename_i hit
        simp only [Option.some.injEq]
        constructor
        · rintro rfl
          exact ⟨Nat.le_refl _, inBounds, ⟨elements[index], Array.getElem?_eq_getElem inBounds, hit⟩,
            fun _ lo hi _ _ => absurd hi (Nat.not_lt.mpr lo)⟩
        · rintro ⟨low, high, -, first⟩
          refine Classical.byContradiction fun ne => ?_
          have := first index (Nat.le_refl _) (by omega) elements[index]
            (Array.getElem?_eq_getElem inBounds)
          rw [hit] at this
          exact absurd this (by decide)
      · rename_i miss
        rw [ih (index + 1) i (by omega)]
        constructor
        · rintro ⟨low, high, found, first⟩
          refine ⟨by omega, high, found, fun j lo hi x hx => ?_⟩
          by_cases same : j = index
          · subst same
            rw [Array.getElem?_eq_getElem inBounds, Option.some.injEq] at hx
            subst hx
            simpa using miss
          · exact first j (by omega) hi x hx
        · rintro ⟨low, high, found, first⟩
          refine ⟨?_, high, found, fun j lo hi x hx => first j (by omega) hi x hx⟩
          refine Classical.byContradiction fun lt => ?_
          have same : i = index := by omega
          subst same
          obtain ⟨x, hx, hit⟩ := found
          rw [Array.getElem?_eq_getElem inBounds, Option.some.injEq] at hx
          subst hx
          exact miss hit

/-- A search over the whole vector finds a position exactly when the
element there is accepted and none before it is, positions before it
stated as a specification's range quantifier states them. -/
theorem findIndex?_eq_some_iff {α : Type} (eq : α → α → Bool) (elements : Array α) (needle : α)
    (i : Nat) :
    findIndex? eq elements needle elements.size 0 = some i ↔
      i < elements.size ∧ (∃ x, elements[i]? = some x ∧ eq x needle = true) ∧
        ∀ j : Int, 0 ≤ j → j < ↑i → ∀ x, elements[j.toNat]? = some x → eq x needle = false := by
  rw [findIndex?_eq_some_from eq elements needle elements.size 0 i (by omega)]
  constructor
  · rintro ⟨-, high, found, first⟩
    exact ⟨high, found, fun j _ hi x hx => first j.toNat (Nat.zero_le _) (by omega) x hx⟩
  · rintro ⟨high, found, first⟩
    exact ⟨Nat.zero_le _, high, found,
      fun j _ hi x hx => first ↑j (by omega) (by omega) x (by simpa using hx)⟩

/-- Lookups after a removal, at integer positions as the denotation states
them: before the removed position the array is read there, from it on one
position further. -/
theorem getElem?_eraseIdx_toNat_of_lt {α : Type} (xs : Array α) (i : Nat) (h : i < xs.size)
    (k : Int) (h0 : 0 ≤ k) (hk : k < ↑i) : (xs.eraseIdx i h)[k.toNat]? = xs[k.toNat]? :=
  Array.getElem?_eraseIdx_of_lt h (by omega)

theorem getElem?_eraseIdx_toNat_of_ge {α : Type} (xs : Array α) (i : Nat) (h : i < xs.size)
    (k : Int) (hk : ↑i ≤ k) : (xs.eraseIdx i h)[k.toNat]? = xs[(k + 1).toNat]? := by
  rw [Array.getElem?_eraseIdx_of_ge h (by omega)]
  congr 1
  omega

/-- Lookups after an insertion, at integer positions: before the inserted
position the array is read there, past it one position back. -/
theorem getElem?_insertIdx_toNat_of_lt {α : Type} (xs : Array α) (x : α) (i : Nat)
    (w : i ≤ xs.size) (k : Int) (h0 : 0 ≤ k) (hk : k < ↑i) :
    (xs.insertIdx i x w)[k.toNat]? = xs[k.toNat]? :=
  Array.getElem?_insertIdx_of_lt w (by omega)

theorem getElem?_insertIdx_toNat_of_gt {α : Type} (xs : Array α) (x : α) (i : Nat)
    (w : i ≤ xs.size) (k : Int) (hk : ↑i < k) (hs : k ≤ ↑xs.size) :
    (xs.insertIdx i x w)[k.toNat]? = xs[(k - 1).toNat]? := by
  rw [Array.getElem?_insertIdx_of_ge (by omega : i < k.toNat) (by omega)]
  congr 1
  omega

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
theorem findIndex?_eq_none_mem_iff {α : Type} (eq : α → α → Bool) (elements : Array α) (needle : α) :
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
theorem findIndex?_isSome_mem_iff {α : Type} (eq : α → α → Bool) (elements : Array α) (needle : α) :
    (findIndex? eq elements needle elements.size 0).isSome = true ↔
      ∃ x ∈ elements, eq x needle = true := by
  rw [← Bool.not_eq_false, Option.isSome_eq_false_iff, Option.isNone_iff_eq_none,
    findIndex?_eq_none_mem_iff]
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

section CompareResult
variable [Carriers]

/-- The result of `compare` on two native values: the structural order as
an `i8`. -/
def NTy.compareResult (orders : Validation.ValueOrders) (τ : NTy) (left right : τ.carrier) :
    SpecInt (.bits 8) true :=
  ⟨orderValue (NTy.order (SemanticOperations.valueRanks orders) τ left right), by
    rw [IntegerValueFits_signed_succ]
    cases NTy.order (SemanticOperations.valueRanks orders) τ left right <;> simp [orderValue]⟩

@[simp] theorem NTy.compareResult_val (orders : Validation.ValueOrders) (τ : NTy)
    (left right : τ.carrier) :
    (NTy.compareResult orders τ left right).val =
      orderValue (NTy.order (SemanticOperations.valueRanks orders) τ left right) := rfl

/-- `compare` on native values is the runtime's on their encodings. -/
theorem NTy.compareResult_encode (orders : Validation.ValueOrders) (τ : NTy)
    (left right : τ.carrier) :
    (NTy.compareResult orders τ left right).val =
      orderValue (RuntimeValue.order (SemanticOperations.valueRanks orders)
        (τ.encode left) (τ.encode right)) := by
  rw [NTy.compareResult_val, NTy.order_encode]

end CompareResult

/-- A bounded vector with one element replaced; the size is unchanged. -/
def _root_.VerexIR.SpecVector.set {α : Type} (vector : SpecVector α) (index : Nat) (value : α) : SpecVector α :=
  ⟨vector.values.set! index value, by rw [Array.size_set!]; exact vector.bounded⟩

@[simp] theorem _root_.VerexIR.SpecVector.values_set {α : Type} (vector : SpecVector α) (index : Nat) (value : α) :
    (vector.set index value).values = vector.values.set! index value := rfl

/-- An array's vector with one element replaced keeps the array's length. -/
theorem _root_.VerexIR.SpecArray.size_set {α : Type} {length : Nat} (array : SpecArray α length)
    (index : Nat) (value : α) : (array.vector.set index value).values.size = length := by
  rw [SpecVector.values_set, Array.size_set!]; exact array.size_eq

@[simp] theorem _root_.VerexIR.SpecVector.values_mk {α : Type} (values : Array α) (bounded : values.size < 2 ^ 64) :
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
theorem _root_.VerexIR.SpecInt.width_nonzero {width : Nat} {signed : Bool}
    (value : SpecInt (.bits width) signed) : width ≠ 0 := by
  intro zero
  subst zero
  have := value.fits
  simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?] at this

/-- A certified integer built from bounds, at the width of a witness. -/
def _root_.VerexIR.SpecInt.ofBounds {width : Nat} (witness : SpecInt (.bits width) false) (value : Int)
    (inRange : 0 ≤ value ∧ value ≤ 2 ^ width - 1) : SpecInt (.bits width) false :=
  ⟨value, fits_of_bounds (integerBounds?_of_nonzero false witness.width_nonzero) inRange⟩

@[simp] theorem _root_.VerexIR.SpecInt.ofBounds_val {width : Nat} (witness : SpecInt (.bits width) false)
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
    Spec σ Failure (SpecInt (.bits width) signed) :=
  if fits : IntegerValueFits (.bits width) signed value then Spec.pure ⟨value, fits⟩
  else match (Ty.integer (.bits width) signed).integerBounds? with
    | some _ => Spec.abort (Failure.ofInts failure #[value])
    | none => Spec.abort (Failure.ofInts failure #[])

theorem checkedInt_eq (failure : ThrowKind) (width : Nat) (signed : Bool) (value : Int)
    (nonzero : width ≠ 0) :
    (checkedInt failure width signed value : Spec σ Failure _) =
      if fits : IntegerValueFits (.bits width) signed value then Spec.pure ⟨value, fits⟩
      else Spec.abort (Failure.ofInts failure #[value]) := by
  unfold checkedInt
  rw [integerBounds?_of_nonzero signed nonzero]

/-- The checked operation agrees with the runtime evaluator. -/
theorem checkedInt_ok_iff (failure : ThrowKind) (width : Nat) (signed : Bool) (value : Int)
    (state final : σ) (result : SpecInt (.bits width) signed) :
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
        simp only [fits, ↓reduceDIte, Spec.pure_ok, inRange.1, inRange.2, decide_true, Bool.and_self, ↓reduceIte, Except.ok.injEq, RuntimeValue.integer.injEq]
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
    (value : Int) (state : σ) (error : Failure) :
    (checkedInt failure width signed value).aborts state error ↔
      ∃ kind thrown, checkedInteger failure (.integer (.bits width) signed) value =
        .error (kind, thrown) ∧ error.encode = (kind, thrown) := by
  unfold checkedInt checkedInteger
  cases bounds : (Ty.integer (.bits width) signed).integerBounds? with
  | none =>
      simp only [not_fits_of_none bounds, ↓reduceDIte, Spec.abort_aborts, Except.error.injEq]
      constructor
      · rintro rfl
        exact ⟨_, _, rfl, by simp [Failure.encode, Failure.ofInts]⟩
      · rintro ⟨_, _, equal, encoded⟩
        cases equal
        exact Failure.encode_injective (encoded.trans (by simp [Failure.encode, Failure.ofInts]))
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
          exact ⟨_, _, rfl, by simp [Failure.encode, Failure.ofInts]⟩
        · rintro ⟨_, _, equal, encoded⟩
          cases equal
          exact Failure.encode_injective (encoded.trans (by simp [Failure.encode, Failure.ofInts]))

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
    (left right : SpecInt (.bits width) signed) : Spec σ Failure (SpecInt (.bits width) signed) :=
  match op with
  | .add => checkedInt failure width signed (left.val + right.val)
  | .subtract => checkedInt failure width signed (left.val - right.val)
  | .multiply => checkedInt failure width signed (left.val * right.val)
  | .divide =>
      if right.val = 0 then Spec.abort (Failure.ofInts failure #[])
      else checkedInt failure width signed (left.val.tdiv right.val)
  | .modulo =>
      if right.val = 0 then Spec.abort (Failure.ofInts failure #[])
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

/-- The residue `wrapInt` computes is the remainder: `%` of integers is
already the least nonnegative residue. -/
theorem residue_eq (modulus value : Int) :
    ((value % modulus) + modulus) % modulus = value % modulus := by
  rw [Int.add_emod_right, Int.emod_emod_of_dvd _ (Int.dvd_refl _)]

theorem wrapInt_unsigned (width : Nat) (value : Int) :
    wrapInt width false value = value % (2 : Int) ^ width := by
  simp [wrapInt, residue_eq]

theorem wrapInt_signed (width : Nat) (value : Int) :
    wrapInt width true value =
      if value % (2 : Int) ^ width ≥ (2 : Int) ^ (width - 1) then
        value % (2 : Int) ^ width - (2 : Int) ^ width
      else value % (2 : Int) ^ width := by
  simp [wrapInt, residue_eq]

theorem residue_bounds (modulus value : Int) (positive : 0 < modulus) :
    0 ≤ value % modulus ∧ value % modulus < modulus :=
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

/-- A certified integer of a nonzero width defaults to zero. -/
instance {n : Nat} {signed : Bool} : Inhabited (SpecInt (.bits (n + 1)) signed) :=
  ⟨⟨0, zero_fits (n + 1) signed (Nat.succ_ne_zero n)⟩⟩

/-- A certified integer a clause defines by its value (`∃ x, x.val = e ∧ p x`)
is the value cut to the width, the value's range a conjunct of its own: the
cut is then read as a cut the context bounds. A leaf's step
(`verex_defined_values`), after the path has named its locals. -/
theorem SpecInt.exists_val_cut_unsigned (n : Nat) {value : Int}
    {p : SpecInt (.bits (n + 1)) false → Prop} :
    (∃ c : SpecInt (.bits (n + 1)) false, c.val = value ∧ p c) ↔
      (0 ≤ value ∧ value ≤ 2 ^ (n + 1) - 1) ∧ p (SpecInt.ofInt default value) :=
  ⟨fun ⟨c, defined, held⟩ =>
    ⟨(IntegerValueFits_unsigned_succ n value).mp (defined ▸ c.fits),
      (SpecInt.ofInt_eq_of_val default c defined) ▸ held⟩,
    fun ⟨bounds, held⟩ => ⟨_, SpecInt.val_ofInt default
      ((IntegerValueFits_unsigned_succ n value).mpr bounds), held⟩⟩

theorem SpecInt.exists_val_cut_signed (n : Nat) {value : Int}
    {p : SpecInt (.bits (n + 1)) true → Prop} :
    (∃ c : SpecInt (.bits (n + 1)) true, c.val = value ∧ p c) ↔
      (-(2 : Int) ^ n ≤ value ∧ value ≤ 2 ^ n - 1) ∧ p (SpecInt.ofInt default value) :=
  ⟨fun ⟨c, defined, held⟩ =>
    ⟨(IntegerValueFits_signed_succ n value).mp (defined ▸ c.fits),
      (SpecInt.ofInt_eq_of_val default c defined) ▸ held⟩,
    fun ⟨bounds, held⟩ => ⟨_, SpecInt.val_ofInt default
      ((IntegerValueFits_signed_succ n value).mpr bounds), held⟩⟩

/-- Presence of an optional value in one form: `isSome`, as its absence is
`= none`. -/
theorem not_eq_none_iff_isSome {α : Type} (o : Option α) : (¬o = none) ↔ o.isSome = true := by
  cases o <;> simp

/-- A modular operation on certified integers, wrapped into their width. -/
def ModularOp.run (op : ModularOp) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed) : SpecInt (.bits width) signed :=
  ⟨wrapInt width signed (op.eval left.val right.val), wrapInt_fits _ _ _ left.width_nonzero⟩

@[simp] theorem ModularOp.run_val (op : ModularOp) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed) :
    (op.run left right).val = wrapInt width signed (op.eval left.val right.val) := rfl

/-- A code point's validity in the normal form: its range, outside the
surrogates. -/
theorem isUnicodeScalar_eq_true (value : Nat) :
    isUnicodeScalar value = true ↔ value ≤ 0x10ffff ∧ (value < 0xd800 ∨ 0xdfff < value) := by
  simp only [isUnicodeScalar, Bool.and_eq_true, decide_eq_true_eq, Bool.not_eq_true',
    Bool.and_eq_false_iff, decide_eq_false_iff_not]
  omega

/-- The bound a character carries by its type. -/
theorem _root_.VerexIR.SpecChar.bounds (value : SpecChar) :
    value.val ≤ 0x10ffff ∧ (value.val < 0xd800 ∨ 0xdfff < value.val) :=
  (isUnicodeScalar_eq_true _).mp value.valid

/-- A character's code point as an integer. -/
def _root_.VerexIR.SpecChar.code (value : SpecChar) : SpecInt (.bits 32) false :=
  ⟨value.val, (IntegerValueFits_unsigned_succ 31 _).mpr (by
    have valid := value.valid
    simp only [isUnicodeScalar, Bool.and_eq_true, decide_eq_true_eq] at valid
    omega)⟩

@[simp] theorem _root_.VerexIR.SpecChar.code_val (value : SpecChar) : value.code.val = value.val := rfl

/-- Whether a modular operation's exact result leaves the width. -/
def ModularOp.overflows (op : ModularOp) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed) : Bool :=
  !decide (IntegerValueFits (.bits width) signed (op.eval left.val right.val))

/-- A flag in the normal form: the exact result outside the width's range, as
a checked operation's failure condition states it. -/
theorem ModularOp.overflows_unsigned_succ (op : ModularOp) (n : Nat)
    (left right : SpecInt (.bits (n + 1)) false) :
    op.overflows left right =
      !decide (0 ≤ op.eval left.val right.val ∧ op.eval left.val right.val ≤ 2 ^ (n + 1) - 1) :=
  congrArg not (decide_eq_decide.mpr (IntegerValueFits_unsigned_succ n _))

theorem ModularOp.overflows_signed_succ (op : ModularOp) (n : Nat)
    (left right : SpecInt (.bits (n + 1)) true) :
    op.overflows left right =
      !decide (-(2 : Int) ^ n ≤ op.eval left.val right.val ∧
        op.eval left.val right.val ≤ 2 ^ n - 1) :=
  congrArg not (decide_eq_decide.mpr (IntegerValueFits_signed_succ n _))

/-- Bitwise operations on unsigned integers. -/
inductive BitOp where
  | and
  | or
  | xor
  deriving DecidableEq, Repr, Inhabited

/-- The mathematical value of a bitwise operation: on the bit patterns of
nonnegative operands, as the runtime computes it. -/
def BitOp.eval : BitOp → Int → Int → Int
  | .and => IntegerArithmetic.bitwiseAnd
  | .or => fun left right => Int.ofNat (left.toNat ||| right.toNat)
  | .xor => fun left right => Int.ofNat (left.toNat ^^^ right.toNat)

@[simp] theorem BitOp.eval_and (left right : Int) :
    BitOp.and.eval left right = IntegerArithmetic.bitwiseAnd left right := rfl

@[simp] theorem BitOp.eval_or (left right : Int) :
    BitOp.or.eval left right = Int.ofNat (left.toNat ||| right.toNat) := rfl

@[simp] theorem BitOp.eval_xor (left right : Int) :
    BitOp.xor.eval left right = Int.ofNat (left.toNat ^^^ right.toNat) := rfl

/-- An unsigned value's bits exclusively disjoined with the ones of a width
at least its own: its complement there. -/
theorem SpecInt.toNat_xor_ones {width bits : Nat} (value : SpecInt (.bits width) false)
    (wide : width ≤ bits) : value.val.toNat ^^^ (2 ^ bits - 1) = 2 ^ bits - 1 - value.val.toNat := by
  obtain ⟨lower, upper⟩ := value.unsigned_bounds
  have pow : ((2 ^ width : Nat) : Int) = (2 : Int) ^ width := Int.natCast_pow 2 width
  have positive := Nat.two_pow_pos width
  have widen := Nat.pow_le_pow_right (by decide : 0 < 2) wide
  apply VerexIR.Proofs.IntegerArithmetic.xor_two_pow_sub_one
  omega

theorem BitOp.eval_bounds (op : BitOp) {width : Nat}
    (left right : SpecInt (.bits width) false) :
    0 ≤ op.eval left.val right.val ∧ op.eval left.val right.val ≤ 2 ^ width - 1 := by
  obtain ⟨leftLower, leftUpper⟩ := left.unsigned_bounds
  obtain ⟨rightLower, rightUpper⟩ := right.unsigned_bounds
  have pow : ((2 ^ width : Nat) : Int) = (2 : Int) ^ width := Int.natCast_pow 2 width
  have positive := Nat.two_pow_pos width
  have leftBelow : left.val.toNat < 2 ^ width := by omega
  have rightBelow : right.val.toNat < 2 ^ width := by omega
  cases op with
  | and =>
      simp only [BitOp.eval, IntegerArithmetic.bitwiseAnd_nonnegative _ _ leftLower rightLower, Int.ofNat_eq_natCast]
      have := Nat.and_le_left (n := left.val.toNat) (m := right.val.toNat)
      omega
  | or =>
      simp only [BitOp.eval, Int.ofNat_eq_natCast]
      have := Nat.or_lt_two_pow leftBelow rightBelow
      omega
  | xor =>
      simp only [BitOp.eval, Int.ofNat_eq_natCast]
      have := Nat.xor_lt_two_pow leftBelow rightBelow
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

/-- Truncating division of a nonnegative dividend: the division algorithm,
and the remainder's range at a positive divisor. With the quotient times
the divisor read as one term, these are linear. -/
theorem tdiv_algorithm {left : Int} (nonnegative : 0 ≤ left) (right : Int) :
    left.tdiv right * right + left.tmod right = left ∧ 0 ≤ left.tmod right ∧
      (0 < right → left.tmod right < right) :=
  ⟨Int.tdiv_mul_add_tmod left right, Int.tmod_nonneg right nonnegative, fun positive =>
    Int.tmod_lt_of_pos left positive⟩

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
    Spec σ Failure (SpecInt (.bits width) false) :=
  if distance.val < width then
    Spec.pure (SpecInt.ofBounds value (Int.shiftLeft value.val distance.val.toNat % 2 ^ width)
      (shiftLeft_mod_bounds _ _ value.width_nonzero))
  else Spec.abort (Failure.ofInts failure #[distance.val])

/-- A certified unsigned value shifted left stays below a modulus at least
the power of two its width and the distance reach: the modulus is idle. -/
theorem shiftLeft_emod_of_fits {width : Nat} (value : SpecInt (.bits width) false)
    (distance : Nat) (modulus : Int) (fits : (2 : Int) ^ (width + distance) ≤ modulus) :
    Int.shiftLeft value.val distance % modulus = Int.shiftLeft value.val distance := by
  obtain ⟨lower, upper⟩ := value.unsigned_bounds
  have shifted : Int.shiftLeft value.val distance = value.val * 2 ^ distance := by
    show value.val <<< distance = _
    rw [Int.shiftLeft_eq]
  have positive : (0 : Int) < 2 ^ distance := Int.pow_pos (by decide)
  have below : value.val * 2 ^ distance < 2 ^ (width + distance) := by
    rw [Int.pow_add]
    exact Int.mul_lt_mul_of_pos_right (by omega) positive
  rw [shifted]
  exact Int.emod_eq_of_lt (Int.mul_nonneg lower (Int.le_of_lt positive)) (by omega)

/-- The truncating form of `shiftLeft_emod_of_fits`: on a nonnegative
value both remainders agree. -/
theorem shiftLeft_tmod_of_fits {width : Nat} (value : SpecInt (.bits width) false)
    (distance : Nat) (modulus : Int) (fits : (2 : Int) ^ (width + distance) ≤ modulus) :
    (Int.shiftLeft value.val distance).tmod modulus = Int.shiftLeft value.val distance := by
  have nonnegative : 0 ≤ Int.shiftLeft value.val distance := by
    show 0 ≤ value.val <<< distance
    rw [Int.shiftLeft_eq]
    exact Int.mul_nonneg value.unsigned_bounds.1 (Int.pow_nonneg (by decide))
  rw [Int.tmod_eq_emod_of_nonneg nonnegative]
  exact shiftLeft_emod_of_fits value distance modulus fits

/-- The truncating remainder of a shifted nonnegative value is its
remainder: a specification's remainder meets the runtime's. -/
theorem shiftLeft_tmod_of_nonneg (value : Int) (distance : Nat) (modulus : Int)
    (nonnegative : 0 ≤ value) :
    (Int.shiftLeft value distance).tmod modulus = Int.shiftLeft value distance % modulus :=
  Int.tmod_eq_emod_of_nonneg (shiftLeft_nonneg value distance nonnegative)

/-- A shift left is a multiplication by a power of two, which `omega` reads. -/
theorem shiftLeft_eq_mul (value : Int) (distance : Nat) :
    Int.shiftLeft value distance = value * 2 ^ distance := by
  show value <<< distance = _
  rw [Int.shiftLeft_eq]

/-- A shift right is a division by a power of two, which `omega` reads. -/
theorem shiftRight_eq_div (value : Int) (distance : Nat) :
    Int.shiftRight value distance = value / 2 ^ distance := by
  rw [← Int.shiftRight_eq, Int.shiftRight_eq_div_pow]
  simp

/-- A product of consecutive integers is even and nonnegative, in each order
of its factors: what a triangular sum `n * (n - 1) / 2` needs, which `omega`
reads with the product as an atom. -/
theorem mul_succ_facts (n : Int) : 0 ≤ n * (n + 1) ∧ n * (n + 1) % 2 = 0 := by
  refine ⟨?_, ?_⟩
  · by_cases h : n ≤ -1
    · exact Int.mul_nonneg_of_nonpos_of_nonpos (by omega) (by omega)
    · exact Int.mul_nonneg (by omega) (by omega)
  · rcases Int.emod_two_eq n with h | h <;> simp [Int.mul_emod, Int.add_emod, h]

theorem mul_pred_facts (n : Int) : 0 ≤ n * (n - 1) ∧ n * (n - 1) % 2 = 0 := by
  refine ⟨?_, ?_⟩
  · by_cases h : n ≤ 0
    · exact Int.mul_nonneg_of_nonpos_of_nonpos h (by omega)
    · exact Int.mul_nonneg (by omega) (by omega)
  · rcases Int.emod_two_eq n with h | h <;> simp [Int.mul_emod, Int.sub_emod, h]

theorem succ_mul_facts (n : Int) : 0 ≤ (n + 1) * n ∧ (n + 1) * n % 2 = 0 := by
  rw [Int.mul_comm]; exact mul_succ_facts n

theorem pred_mul_facts (n : Int) : 0 ≤ (n - 1) * n ∧ (n - 1) * n % 2 = 0 := by
  rw [Int.mul_comm]; exact mul_pred_facts n

/-- The bounds of a product of unsigned certified integers, in the form
`omega` reads with the product as an atom. -/
theorem mul_unsigned_bounds {leftWidth rightWidth : Nat} (left : SpecInt (.bits leftWidth) false)
    (right : SpecInt (.bits rightWidth) false) :
    0 ≤ left.val * right.val ∧
      left.val * right.val ≤ (2 ^ leftWidth - 1) * (2 ^ rightWidth - 1) := by
  obtain ⟨leftLower, leftUpper⟩ := left.unsigned_bounds
  obtain ⟨rightLower, rightUpper⟩ := right.unsigned_bounds
  exact ⟨Int.mul_nonneg leftLower rightLower,
    Int.mul_le_mul leftUpper rightUpper rightLower (Int.le_trans leftLower leftUpper)⟩

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
    Spec σ Failure (SpecInt (.bits width) false) :=
  if distance.val < width then
    Spec.pure (SpecInt.ofBounds value (Int.shiftRight value.val distance.val.toNat)
      (shiftRight_bounds value _))
  else Spec.abort (Failure.ofInts failure #[distance.val])

/-! ## Weakest preconditions

One rule per operation, in the vocabulary a clause reads: certificates as
bounds, results by their value. -/

theorem wp_ite (c : Prop) [Decidable c] (left right : Spec σ Failure α)
    (ensures : α → σ → Prop) (aborts : Failure → Prop) (state : σ) :
    wp (if c then left else right) ensures aborts state ↔
      (c → wp left ensures aborts state) ∧ (¬c → wp right ensures aborts state) := by
  by_cases h : c <;> simp [h]

theorem wp_bottom (ensures : α → σ → Prop) (aborts : Failure → Prop)
    (state : σ) : wp (Spec.bottom : Spec σ Failure α) ensures aborts state := by
  simp [wp, Spec.bottom]

theorem wp_stuck (ensures : α → σ → Prop) (aborts : Failure → Prop) (state : σ) :
    wp (Spec.stuck : Spec σ Failure α) ensures aborts state ↔ False := by
  simp [wp]

/-- A computed integer is named: the continuation receives a variable and
the value it is defined by, so a value read several times is one variable in
the goal, as a temporary is in the Move Prover's translation. -/
theorem forall_named {width : IntWidth} {signed : Bool} {value : Int}
    {p : SpecInt width signed → Prop} :
    (∀ named : SpecInt width signed, named.val = value →
        IntegerValueFits width signed value → p named) ↔
      ∀ fits : IntegerValueFits width signed value, p ⟨value, fits⟩ := by
  constructor
  · intro h fits
    exact h ⟨value, fits⟩ rfl fits
  · intro h certified definition _
    obtain ⟨v, fits⟩ := certified
    simp only at definition
    subst definition
    exact h fits

/-- `forall_named` for a continuation that discards the value. -/
theorem forall_named_discard {width : IntWidth} {signed : Bool} {value : Int} {p : Prop} :
    (∀ named : SpecInt width signed, named.val = value →
        IntegerValueFits width signed value → p) ↔
      (IntegerValueFits width signed value → p) :=
  forall_named (p := fun _ => p)

/-- `forall_named` at a value built from bounds. -/
theorem forall_named_ofBounds {width : Nat} {witness : SpecInt (.bits width) false} {value : Int}
    {inRange : 0 ≤ value ∧ value ≤ 2 ^ width - 1} {p : SpecInt (.bits width) false → Prop} :
    (∀ named : SpecInt (.bits width) false, named.val = value → p named) ↔
      p (SpecInt.ofBounds witness value inRange) := by
  constructor
  · intro h
    exact h _ rfl
  · intro h named definition
    have : named = SpecInt.ofBounds witness value inRange := SpecInt.ext (by simp [definition])
    subst this
    exact h

/-- A name defined by a value that fits is that value. -/
theorem forall_named_fits {width : IntWidth} {signed : Bool} {value : Int}
    {p : SpecInt width signed → Prop} (fits : IntegerValueFits width signed value) :
    (∀ named : SpecInt width signed, named.val = value → p named) ↔ p ⟨value, fits⟩ :=
  ⟨fun h => h _ rfl, fun h named definition => by
    obtain ⟨v, _⟩ := named
    simp only at definition
    subst definition
    exact h⟩

open Lean Meta in
/-- Whether a proposition is closed as the kernel reads it: no variable, no
metavariable, and nothing a frame resolves.  A decision over a term a frame
resolves must not be taken: `Skolems.resolve` reduces for the elaborator,
being a projection of a frame, and not for the kernel, which then rejects
the `Eq.refl` the decision leaves (`Solve.rowInhabitable?`).  The frames
come later than this module, so the resolution is named and not quoted. -/
def frameFreeDecision (statement : Lean.Expr) : Bool :=
  !statement.hasFVar && !statement.hasMVar && !statement.hasLooseBVars &&
    (statement.find? (·.isAppOf `VerexIR.Proofs.Denote.Skolems.resolve)).isNone

open Lean Meta in
/-- A decidable proposition proved by evaluation, where the proposition is
frame-free (`frameFreeDecision`) and the decision evaluates to `true`; none
otherwise, so that no step builds a certificate the kernel refuses.  Every
decision a step takes of a proposition it did not build from literals alone
goes through this. -/
def decidedFrameFree? (statement : Lean.Expr) : MetaM (Option Lean.Expr) := do
  unless frameFreeDecision statement do return none
  try
    unless (← withDefault (whnf (← mkDecide statement))).isConstOf ``Bool.true do return none
    return some (← mkDecideProof statement)
  catch _ => return none

open Lean Meta Simp in
/-- A literal is not named: a computed value that is a literal, as in a loop
unrolled over literals, decides the conditions that read it. -/
simproc [vir_denote] namedLiteral (∀ _named : SpecInt _ _, _) := fun e => do
  let .forallE name type body info := e | return .continue
  unless type.isAppOfArity ``SpecInt 2 do return .continue
  let .forallE _ premise rest _ := body | return .continue
  if rest.hasLooseBVar 0 then return .continue
  let some (_, lhs, value) := premise.eq? | return .continue
  unless lhs.isAppOfArity ``SpecInt.val 3 && lhs.appArg! == .bvar 0 do return .continue
  if value.hasLooseBVars then return .continue
  let some _ ← getIntValue? value | return .continue
  let fits := mkApp3 (mkConst ``IntegerValueFits) (type.getArg! 0) (type.getArg! 1) value
  let some certificate ← decidedFrameFree? fits | return .continue
  let continuation := Expr.lam name type (rest.lowerLooseBVars 1 1) info
  let equivalence ← mkAppOptM ``forall_named_fits
    #[type.getArg! 0, type.getArg! 1, value, continuation, certificate]
  let literal := mkApp4 (mkConst ``SpecInt.mk) (type.getArg! 0) (type.getArg! 1) value certificate
  return .visit { expr := continuation.beta #[literal], proof? := some (← mkPropExt equivalence) }

theorem wp_checkedInt (failure : ThrowKind) (width : Nat) (signed : Bool) (value : Int)
    (nonzero : width ≠ 0) (ensures : SpecInt (.bits width) signed → σ → Prop)
    (aborts : Failure → Prop) (state : σ) :
    wp (checkedInt failure width signed value) ensures aborts state ↔
      (∀ named : SpecInt (.bits width) signed, named.val = value →
        IntegerValueFits (.bits width) signed value → ensures named state) ∧
      (¬IntegerValueFits (.bits width) signed value → aborts (Failure.ofInts failure #[value])) := by
  rw [checkedInt_eq _ _ _ _ nonzero, forall_named]
  by_cases fits : IntegerValueFits (.bits width) signed value <;> simp [fits]

theorem wp_checked_add (failure : ThrowKind) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed)
    (ensures : SpecInt (.bits width) signed → σ → Prop)
    (aborts : Failure → Prop) (state : σ) :
    wp (CheckedOp.add.run failure left right) ensures aborts state ↔
      (∀ named : SpecInt (.bits width) signed, named.val = left.val + right.val →
        IntegerValueFits (.bits width) signed (left.val + right.val) → ensures named state) ∧
      (¬IntegerValueFits (.bits width) signed (left.val + right.val) →
        aborts (Failure.ofInts failure #[(left.val + right.val)])) :=
  wp_checkedInt _ _ _ _ left.width_nonzero _ _ _

theorem wp_checked_subtract (failure : ThrowKind) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed)
    (ensures : SpecInt (.bits width) signed → σ → Prop)
    (aborts : Failure → Prop) (state : σ) :
    wp (CheckedOp.subtract.run failure left right) ensures aborts state ↔
      (∀ named : SpecInt (.bits width) signed, named.val = left.val - right.val →
        IntegerValueFits (.bits width) signed (left.val - right.val) → ensures named state) ∧
      (¬IntegerValueFits (.bits width) signed (left.val - right.val) →
        aborts (Failure.ofInts failure #[(left.val - right.val)])) :=
  wp_checkedInt _ _ _ _ left.width_nonzero _ _ _

theorem wp_checked_multiply (failure : ThrowKind) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed)
    (ensures : SpecInt (.bits width) signed → σ → Prop)
    (aborts : Failure → Prop) (state : σ) :
    wp (CheckedOp.multiply.run failure left right) ensures aborts state ↔
      (∀ named : SpecInt (.bits width) signed, named.val = left.val * right.val →
        IntegerValueFits (.bits width) signed (left.val * right.val) → ensures named state) ∧
      (¬IntegerValueFits (.bits width) signed (left.val * right.val) →
        aborts (Failure.ofInts failure #[(left.val * right.val)])) :=
  wp_checkedInt _ _ _ _ left.width_nonzero _ _ _

theorem wp_checked_divide (failure : ThrowKind) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed)
    (ensures : SpecInt (.bits width) signed → σ → Prop)
    (aborts : Failure → Prop) (state : σ) :
    wp (CheckedOp.divide.run failure left right) ensures aborts state ↔
      (right.val = 0 → aborts (Failure.ofInts failure #[])) ∧
      (right.val ≠ 0 →
        (∀ named : SpecInt (.bits width) signed, named.val = left.val.tdiv right.val →
          IntegerValueFits (.bits width) signed (left.val.tdiv right.val) → ensures named state) ∧
        (¬IntegerValueFits (.bits width) signed (left.val.tdiv right.val) →
          aborts (Failure.ofInts failure #[(left.val.tdiv right.val)]))) := by
  simp only [CheckedOp.run, wp_ite, wp_abort, wp_checkedInt _ _ _ _ left.width_nonzero]

theorem wp_checked_modulo (failure : ThrowKind) {width : Nat} {signed : Bool}
    (left right : SpecInt (.bits width) signed)
    (ensures : SpecInt (.bits width) signed → σ → Prop)
    (aborts : Failure → Prop) (state : σ) :
    wp (CheckedOp.modulo.run failure left right) ensures aborts state ↔
      (right.val = 0 → aborts (Failure.ofInts failure #[])) ∧
      (right.val ≠ 0 →
        (IntegerValueFits (.bits width) signed (left.val.tdiv right.val) →
          (∀ named : SpecInt (.bits width) signed, named.val = left.val.tmod right.val →
            IntegerValueFits (.bits width) signed (left.val.tmod right.val) → ensures named state) ∧
          (¬IntegerValueFits (.bits width) signed (left.val.tmod right.val) →
            aborts (Failure.ofInts failure #[(left.val.tmod right.val)]))) ∧
        (¬IntegerValueFits (.bits width) signed (left.val.tdiv right.val) →
          aborts (Failure.ofInts failure #[(left.val.tdiv right.val)]))) := by
  simp only [CheckedOp.run, wp_ite, wp_abort, wp_bind, wp_checkedInt _ _ _ _ left.width_nonzero, forall_named_discard]

theorem wp_checkedShiftLeft (failure : ThrowKind) {width distanceWidth : Nat}
    (value : SpecInt (.bits width) false) (distance : SpecInt (.bits distanceWidth) false)
    (ensures : SpecInt (.bits width) false → σ → Prop)
    (aborts : Failure → Prop) (state : σ) :
    wp (checkedShiftLeft failure value distance) ensures aborts state ↔
      (distance.val < width →
        ∀ named : SpecInt (.bits width) false,
          named.val = Int.shiftLeft value.val distance.val.toNat % 2 ^ width →
            ensures named state) ∧
      (¬distance.val < width → aborts (Failure.ofInts failure #[distance.val])) := by
  simp only [checkedShiftLeft, wp_ite, wp_pure, wp_abort]
  exact and_congr (imp_congr_right fun _ =>
    (forall_named_ofBounds (p := fun named => ensures named state)).symm) Iff.rfl

theorem wp_checkedShiftRight (failure : ThrowKind) {width distanceWidth : Nat}
    (value : SpecInt (.bits width) false) (distance : SpecInt (.bits distanceWidth) false)
    (ensures : SpecInt (.bits width) false → σ → Prop)
    (aborts : Failure → Prop) (state : σ) :
    wp (checkedShiftRight failure value distance) ensures aborts state ↔
      (distance.val < width →
        ∀ named : SpecInt (.bits width) false,
          named.val = Int.shiftRight value.val distance.val.toNat → ensures named state) ∧
      (¬distance.val < width → aborts (Failure.ofInts failure #[distance.val])) := by
  simp only [checkedShiftRight, wp_ite, wp_pure, wp_abort]
  exact and_congr (imp_congr_right fun _ =>
    (forall_named_ofBounds (p := fun named => ensures named state)).symm) Iff.rfl

/-- The checked conversion at a literal nonzero width. -/
theorem wp_checkedInt_succ (failure : ThrowKind) (n : Nat) (signed : Bool) (value : Int)
    (ensures : SpecInt (.bits (n + 1)) signed → σ → Prop)
    (aborts : Failure → Prop) (state : σ) :
    wp (checkedInt failure (n + 1) signed value) ensures aborts state ↔
      (∀ fits : IntegerValueFits (.bits (n + 1)) signed value, ensures ⟨value, fits⟩ state) ∧
      (¬IntegerValueFits (.bits (n + 1)) signed value → aborts (Failure.ofInts failure #[value])) :=
  (wp_checkedInt _ _ _ _ (Nat.succ_ne_zero n) _ _ _).trans (and_congr forall_named Iff.rfl)

attribute [vir_denote] wp_ite wp_bottom wp_stuck SpecChar.code_val NTy.eqb_char wp_checked_add wp_checked_subtract wp_checked_multiply
  wp_checked_divide wp_checked_modulo wp_checkedShiftLeft wp_checkedShiftRight wp_checkedInt_succ
  Var.get_here Var.get_there Var.set_here Var.set_there SpecInt.ofBounds_val BitOp.run_val
  ModularOp.run_val ModularOp.eval_add ModularOp.eval_subtract ModularOp.eval_multiply
  ModularOp.overflows_unsigned_succ ModularOp.overflows_signed_succ isUnicodeScalar_eq_true
  foundIndex_val foundIndexOf
  wrapInt_unsigned wrapInt_signed
  BitOp.eval_and BitOp.eval_or BitOp.eval_xor CompareOp.decide_less CompareOp.decide_greater CompareOp.decide_lessEqual
  CompareOp.decide_greaterEqual NTy.eqb_int NTy.eqb_bool NTy.eqb_num NTy.eqb_address NTy.eqb_unit
  NTy.eqb_param NTy.eqb_function
  IntegerValueFits_unsigned_succ IntegerValueFits_signed_succ initialEnv_nil initialEnv_nil_cons
  initialEnv_cons

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

attribute [vir_denote] wp_get wp_modify wp_set

/-- An instantiation given as a literal is looked up entry by entry. -/
theorem instantiatedTypeId_cons (entry : TypeId × TypeId) (rest : List (TypeId × TypeId))
    (typeId : TypeId) :
    instantiatedTypeId (entry :: rest).toArray typeId =
      if entry.1 = typeId then entry.2 else instantiatedTypeId rest.toArray typeId := by
  unfold instantiatedTypeId
  obtain ⟨⟨source⟩, target⟩ := entry
  obtain ⟨index⟩ := typeId
  by_cases same : source = index
  · subst same
    simp [List.find?_cons]
  · simp [List.find?_cons, TypeId.mk.injEq, same]

theorem instantiatedTypeId_nil (typeId : TypeId) :
    instantiatedTypeId ([] : List (TypeId × TypeId)).toArray typeId = typeId :=
  instantiatedTypeId_empty typeId

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

@[simp] theorem Option.isSome_dite_some {p : Prop} [Decidable p] {α : Type} (f : p → α) :
    (if h : p then some (f h) else none).isSome = decide p := by
  by_cases h : p <;> simp [h]

attribute [vir_denote] Option.isSome_dite_some exportedRaw?_nil exportedRaw?_cons

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

@[simp, grind =] theorem specInt_encode (width : IntWidth) (signed : Bool) (value : SpecInt width signed) :
    (Codec.specInt width signed).encode value = .integer value.val := rfl
@[simp, grind =] theorem bool_encode (value : Bool) : Codec.bool.encode value = .bool value := rfl
@[simp, grind =] theorem address_encode (value : String) : Codec.address.encode value = .address value := rfl

@[simp] theorem NTy.codec_int [Carriers] (width : Nat) (signed : Bool) :
    (NTy.int width signed).codec = Codec.specInt (.bits width) signed := rfl
@[simp] theorem NTy.codec_num [Carriers] : NTy.num.codec = Codec.specInt .unbounded true := rfl

/-- Every integer is a `num`: decoding one never fails. -/
@[simp] theorem NTy.num_decode_integer [Carriers] (value : Int) :
    NTy.num.codec.decode? (.integer value) = some ⟨value, rfl⟩ := rfl
@[simp] theorem NTy.codec_bool [Carriers] : NTy.bool.codec = Codec.bool := rfl
@[simp] theorem NTy.codec_address [Carriers] : NTy.address.codec = Codec.address := rfl
@[simp] theorem NTy.codec_unit [Carriers] : NTy.unit.codec = Codec.unit := rfl
@[simp] theorem NTy.codec_ref [Carriers] (referent : NTy) :
    (NTy.ref referent).codec = Codec.pair referent.plainCodec referent.codec := rfl
@[simp] theorem NTy.plainCodec_ref [Carriers] (referent : NTy) :
    (NTy.ref referent).plainCodec = Codec.prophecyPair referent.plainCodec := rfl

/-- A native integer already carries its range certificate. Decoding its
value projection must not introduce another range branch. -/
theorem specInt_decode_val (width : IntWidth) (signed : Bool) (value : SpecInt width signed) :
    (Codec.specInt width signed).decode? (.integer value.val) = some value :=
  (Codec.specInt width signed).decode_encode value

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
theorem NTy.decode?_of_encode [Carriers] (τ : NTy) {value : τ.carrier} {raw : RuntimeValue}
    (encoded : τ.encode value = raw) : τ.codec.decode? raw = some value :=
  encoded ▸ τ.codec.decode_encode value

/-- Erasing ghost fields preserves integer observations, including borrowed values. -/
theorem asInt_eraseGhosts (value : RuntimeValue) : value.eraseGhosts.asInt = value.asInt := by
  cases value with
  | borrow loan current =>
    simpa only [RuntimeValue.eraseGhosts, RuntimeValue.asInt] using asInt_eraseGhosts current
  | _ => simp only [RuntimeValue.eraseGhosts, RuntimeValue.asInt]
termination_by sizeOf value

/-- An integer equated to a runtime value is what the value reads as. -/
theorem asInt_of_integer_eq {value : Int} {raw : RuntimeValue}
    (encoded : RuntimeValue.integer value = raw) : raw.asInt = value := by
  subst encoded; rfl

/-- The same for a boolean. -/
theorem asBool_of_bool_eq {value : Bool} {raw : RuntimeValue}
    (encoded : RuntimeValue.bool value = raw) : raw.asBool = value := by
  subst encoded; rfl

/-- The same, once an encoding of a vector has been split element-wise. -/
theorem NTy.decode?_vector_of_map [Carriers] (τ : NTy) {value : SpecVector τ.carrier} {raw : Array RuntimeValue}
    (encoded : value.values.map τ.encode = raw) :
    (NTy.vector τ).codec.decode? (.vector raw) = some value :=
  NTy.decode?_of_encode (.vector τ) (congrArg RuntimeValue.vector encoded)

/-- An array is the array of its list. -/
theorem array_eq_of_toList_eq {α : Type} {xs : Array α} {l : List α} (h : xs.toList = l) :
    xs = l.toArray :=
  Array.toList_inj.mp (h.trans (List.toList_toArray (as := l)).symm)

@[simp] theorem NTy.codec_slice [Carriers] (element : NTy) :
    (NTy.slice element).codec = Codec.boundedVector element.codec := rfl

@[simp] theorem NTy.codec_vector [Carriers] (element : NTy) :
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

/-- Membership in an array, as the position the element is at: an integer
in the array's range, as a specification's range quantifier states one. -/
theorem mem_iff_exists_int_index {α : Type} (a : Array α) (x : α) :
    x ∈ a ↔ ∃ i : Int, 0 ≤ i ∧ i < ↑a.size ∧ a[i.toNat]? = some x := by
  constructor
  · intro mem
    obtain ⟨i, h, rfl⟩ := Array.getElem_of_mem mem
    exact ⟨i, by omega, by omega, by simp [h]⟩
  · rintro ⟨i, _, _, h⟩
    exact Array.mem_of_getElem? h

/-- A search over the whole vector fails exactly when the element at no
position is accepted, positions stated as a specification's range
quantifier states them. -/
theorem findIndex?_eq_none_iff {α : Type} (eq : α → α → Bool) (elements : Array α) (needle : α) :
    findIndex? eq elements needle elements.size 0 = none ↔
      ∀ i : Int, 0 ≤ i → i < ↑elements.size →
        ∀ x, elements[i.toNat]? = some x → eq x needle = false := by
  rw [findIndex?_eq_none_mem_iff]
  constructor
  · intro all _ _ _ x h
    exact all x (Array.mem_of_getElem? h)
  · intro all x mem
    obtain ⟨i, low, high, h⟩ := (mem_iff_exists_int_index _ _).mp mem
    exact all i low high x h

/-- A search over the whole vector succeeds exactly when the element at
some position is accepted. -/
theorem findIndex?_isSome_iff {α : Type} (eq : α → α → Bool) (elements : Array α) (needle : α) :
    (findIndex? eq elements needle elements.size 0).isSome = true ↔
      ∃ i : Int, 0 ≤ i ∧ i < ↑elements.size ∧
        ∃ x, elements[i.toNat]? = some x ∧ eq x needle = true := by
  rw [findIndex?_isSome_mem_iff]
  constructor
  · rintro ⟨x, mem, hx⟩
    obtain ⟨i, low, high, h⟩ := (mem_iff_exists_int_index _ _).mp mem
    exact ⟨i, low, high, x, h, hx⟩
  · rintro ⟨i, _, _, x, h, hx⟩
    exact ⟨x, Array.mem_of_getElem? h, hx⟩

/-- A search over a literal vector computes: its steps unfold at literal
counts. -/
theorem findIndex?_zero {α : Type} (eq : α → α → Bool) (elements : Array α) (needle : α)
    (index : Nat) : findIndex? eq elements needle 0 index = none := rfl

theorem findIndex?_succ {α : Type} (eq : α → α → Bool) (elements : Array α) (needle : α)
    (count index : Nat) :
    findIndex? eq elements needle (count + 1) index =
      if elements[index]?.any (eq · needle) then some index
      else findIndex? eq elements needle count (index + 1) := rfl

/-- An element a lookup yields whose value is a given integer's: the lookup
yields that integer, certificates being proof-irrelevant. -/
theorem exists_some_val_eq_iff {width : IntWidth} {signed : Bool}
    (o : Option (SpecInt width signed)) (n : SpecInt width signed) :
    (∃ x, o = some x ∧ x.val = n.val) ↔ o = some n := by
  constructor
  · rintro ⟨x, ho, hv⟩
    rw [ho, SpecInt.ext hv]
  · intro h
    exact ⟨n, h, rfl⟩

theorem forall_some_val_ne_iff {width : IntWidth} {signed : Bool}
    (o : Option (SpecInt width signed)) (n : SpecInt width signed) :
    (∀ x, o = some x → ¬x.val = n.val) ↔ ¬o = some n := by
  constructor
  · intro h ho
    exact h n ho rfl
  · intro h x ho hv
    exact h (ho.trans (congrArg some (SpecInt.ext hv)))

/-- Every element with a property differs from a given one exactly when
that one lacks the property: an element bound by a lookup and equated to
a value is the lookup at the value. -/
theorem forall_imp_ne_iff {α : Type} (q : α → Prop) (a : α) :
    (∀ x, q x → ¬x = a) ↔ ¬q a :=
  ⟨fun h qa => h a qa rfl, fun h _ qx xa => h (xa ▸ qx)⟩

theorem forall_imp_ne_iff' {α : Type} (q : α → Prop) (a : α) :
    (∀ x, q x → ¬a = x) ↔ ¬q a :=
  ⟨fun h qa => h a qa rfl, fun h _ qx ax => h (ax ▸ qx)⟩

/-- A range quantifier negated, quantified over, or bound: the logical
connectives are pushed only through the shape a specification's range
quantifier and a vector's membership take, not through every statement of a
verification condition. -/
theorem not_exists_range_iff (n : Int) (P : Int → Prop) :
    (¬∃ i : Int, 0 ≤ i ∧ i < n ∧ P i) ↔ ∀ i : Int, 0 ≤ i → i < n → ¬P i := by
  constructor
  · intro h i low high holds
    exact h ⟨i, low, high, holds⟩
  · rintro h ⟨i, low, high, holds⟩
    exact h i low high holds

theorem not_forall_range_iff (n : Int) (P : Int → Prop) :
    (¬∀ i : Int, 0 ≤ i → i < n → ¬P i) ↔ ∃ i : Int, 0 ≤ i ∧ i < n ∧ P i := by
  rw [← not_exists_range_iff, Classical.not_not]

theorem forall_exists_range_iff {α : Type} (n : Int) (P : Int → α → Prop) (Q : α → Prop) :
    (∀ x, (∃ i : Int, 0 ≤ i ∧ i < n ∧ P i x) → Q x) ↔
      ∀ i : Int, 0 ≤ i → i < n → ∀ x, P i x → Q x := by
  constructor
  · intro h i low high x holds
    exact h x ⟨i, low, high, holds⟩
  · rintro h x ⟨i, low, high, holds⟩
    exact h i low high x holds

theorem exists_range_eq_iff {α : Type} (n : Int) (P : Int → α → Prop) (y : α) :
    (∃ x, (∃ i : Int, 0 ≤ i ∧ i < n ∧ P i x) ∧ x = y) ↔ ∃ i : Int, 0 ≤ i ∧ i < n ∧ P i y := by
  constructor
  · rintro ⟨x, ⟨i, low, high, holds⟩, rfl⟩
    exact ⟨i, low, high, holds⟩
  · rintro ⟨i, low, high, holds⟩
    exact ⟨y, ⟨i, low, high, holds⟩, rfl⟩

/-- An integer in a nonempty range is its lower bound or one above it: the
step a literal range's existential takes to the disjunction of its
instances. -/
theorem exists_range_step (lo hi : Int) (P : Int → Prop) (nonempty : lo < hi) :
    (∃ i : Int, lo ≤ i ∧ i < hi ∧ P i) ↔ P lo ∨ ∃ i : Int, lo + 1 ≤ i ∧ i < hi ∧ P i := by
  constructor
  · rintro ⟨i, low, high, holds⟩
    by_cases at_lo : i = lo
    · exact .inl (at_lo ▸ holds)
    · exact .inr ⟨i, by omega, high, holds⟩
  · rintro (holds | ⟨i, low, high, holds⟩)
    · exact ⟨lo, Int.le_refl lo, nonempty, holds⟩
    · exact ⟨i, by omega, high, holds⟩

/-- No integer is in an empty range. -/
theorem exists_range_empty (lo hi : Int) (P : Int → Prop) (empty : hi ≤ lo) :
    (∃ i : Int, lo ≤ i ∧ i < hi ∧ P i) ↔ False :=
  ⟨fun ⟨_, low, high, _⟩ => by omega, False.elim⟩

/-- A property of encoded vector elements is a property of the native
elements. Restrict the rewrite to runtime-value predicates so normalization
does not try it on every unrelated universal proposition. -/
theorem forall_runtime_mem_map_iff {α : Type}
    {encode : α → RuntimeValue} {values : List α}
    {predicate : RuntimeValue → Prop} :
    (∀ value, value ∈ values.map encode → predicate value) ↔
      ∀ value, value ∈ values → predicate (encode value) :=
  List.forall_mem_map

/-- An element of a mapped list under an existential: the element it maps. -/
theorem exists_mem_map_iff {α β : Type} (f : α → β) (l : List α) (P : β → Prop) :
    (∃ r, r ∈ l.map f ∧ P r) ↔ ∃ x, x ∈ l ∧ P (f x) := by
  constructor
  · rintro ⟨r, hr, hP⟩
    obtain ⟨x, hx, rfl⟩ := List.mem_map.mp hr
    exact ⟨x, hx, hP⟩
  · rintro ⟨x, hx, hP⟩
    exact ⟨f x, List.mem_map.mpr ⟨x, hx, rfl⟩, hP⟩

/-- Decoding the encoding of every element gives the elements back. -/
@[simp] theorem decodeElements?_map_encode (codec : Codec Native RuntimeValue)
    (values : List Native) :
    decodeElements? codec (values.map codec.encode) = some values := by
  induction values with
  | nil => rfl
  | cons value values ih => simp [decodeElements?_cons, ih]

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

/-- Decoding a vector's encoding, as the encoding unfolds to the runtime
vector of encoded elements, gives the vector back in one step. -/
theorem boundedVector_decode?_map_encode (codec : Codec Native RuntimeValue)
    (vector : SpecVector Native) :
    (Codec.boundedVector codec).decode? (.vector (vector.values.map codec.encode)) = some vector := by
  rw [boundedVector_decode?_vector, Array.toList_map, decodeElements?_map_encode, Option.bind_some,
    dif_pos (by simpa only [Array.length_toList] using vector.bounded)]

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
@[simp] theorem NTy.codec_encode [Carriers] (τ : NTy) (value : τ.carrier) :
    τ.codec.encode value = τ.encode value := rfl

/-- Encodings at one type are equal exactly when the values are. -/
@[simp] theorem NTy.encode_inj [Carriers] (τ : NTy) (left right : τ.carrier) :
    (τ.encode left = τ.encode right) ↔ left = right :=
  ⟨τ.encode_injective, fun equal => equal ▸ rfl⟩

/-- Encoded rows are equal exactly when the rows are. -/
@[simp] theorem HList.encode_inj [Carriers] {Γ : NRow} (left right : HList Γ) :
    (HList.encode left = HList.encode right) ↔ left = right :=
  ⟨fun equal => (rowCodec Γ).encode_injective equal, fun equal => equal ▸ rfl⟩

open Classical in
/-- Decoding a nominal literal at a struct type: the plain row decodes the
fields, and the value is certified where the data invariants hold of it. -/
@[simp] theorem NTy.decode?_struct_nominal [Carriers] (source : StructHandle)
    (arguments fields : NRow) (values : Array RuntimeValue) :
    (NTy.struct source arguments fields).codec.decode? (.nominal source none values) =
      ((rowCodecPlain fields).decode? values.toList).bind fun row =>
        if held : NTy.deepHolds (.struct source arguments fields) row then some ⟨row, held⟩
        else none := by
  simp [NTy.codec, Codec.certified, NTy.plainCodec, NTy.plainCodecOf, Codec.nominalRow]
@[simp] theorem NTy.plainCodec_decode?_struct_nominal [Carriers] (source : StructHandle)
    (arguments fields : NRow) (values : Array RuntimeValue) :
    (NTy.struct source arguments fields).plainCodec.decode? (.nominal source none values) =
      (rowCodecPlain fields).decode? values.toList := by
  simp [NTy.plainCodec, NTy.plainCodecOf, Codec.nominalRow]

/-- Decoding a row literal, component by component. -/
@[simp] theorem rowCodec_decode?_cons [Carriers] (τ : NTy) (rest : NRow) (value : RuntimeValue)
    (values : List RuntimeValue) :
    (rowCodec (.cons τ rest)).decode? (value :: values) =
      (τ.codec.decode? value).bind fun head =>
        ((rowCodec rest).decode? values).bind fun tail =>
          (some (head, tail) : Option (HList (.cons τ rest))) := rfl

@[simp] theorem rowCodec_decode?_nil [Carriers] :
    (rowCodec .nil).decode? [] = @some (HList .nil) () := rfl

@[simp] theorem rowCodecPlain_decode?_cons [Carriers] (τ : NTy) (rest : NRow) (value : RuntimeValue)
    (values : List RuntimeValue) :
    (rowCodecPlain (.cons τ rest)).decode? (value :: values) =
      (τ.plainCodec.decode? value).bind fun head =>
        ((rowCodecPlain rest).decode? values).bind fun tail =>
          (some (head, tail) : Option (HListPlain (.cons τ rest))) := rfl

@[simp] theorem rowCodecPlain_decode?_nil [Carriers] :
    (rowCodecPlain .nil).decode? [] = @some (HListPlain .nil) () := rfl

/-- Decoding the unfolded encoding of a struct value. -/
@[simp] theorem NTy.decode?_struct_literal [Carriers] (source : StructHandle)
    (arguments fields : NRow) (value : (NTy.struct source arguments fields).carrier) :
    (NTy.struct source arguments fields).codec.decode?
        (.nominal source none (HListPlain.encode value.val).toArray) =
      some value := (NTy.struct source arguments fields).codec.decode_encode value
@[simp] theorem NTy.plainCodec_decode?_struct_literal [Carriers] (source : StructHandle)
    (arguments fields : NRow) (value : HListPlain fields) :
    (NTy.struct source arguments fields).plainCodec.decode?
        (.nominal source none value.encode.toArray) =
      some value := (NTy.struct source arguments fields).plainCodec.decode_encode value

/-- Decoding a nominal literal at an enum type, variant by variant: the
named variant decodes its row, any other is looked up among the later
variants. -/
@[simp] theorem NTy.plainCodec_decode?_enum_cons_nominal [Carriers] (source : StructHandle)
    (name : String) (names : List String) (fields : NRow) (rest : NRows)
    (distinct : (name :: names).Nodup) (variant : String) (values : Array RuntimeValue) :
    (NTy.enum source arguments (name :: names) (.cons fields rest) distinct).plainCodec.decode?
        (.nominal source (some variant) values) =
      if variant = name then
        ((rowCodecPlain fields).decode? values.toList).map variantPlain.first
      else ((NTy.enum source arguments names rest (List.nodup_cons.mp distinct).2).plainCodec.decode?
          (.nominal source (some variant) values)).map variantPlain.later := by
  by_cases equal : variant = name <;>
    simp [NTy.plainCodec, NTy.plainCodecOf, variantCodecPlain, variantCodecPlainOf, variantDecodePlain?, variantDecodePlainOf?, Codec.nominalRow, equal, variantPlain.first, variantPlain.later] <;> rfl

open Classical in
/-- Decoding at an enum type: the plain value, certified where the data
invariants hold of it. -/
@[simp] theorem NTy.decode?_enum [Carriers] (source : StructHandle) (arguments : NRow)
    (names : List String) (rows : NRows) (distinct : names.Nodup) (runtime : RuntimeValue) :
    (NTy.enum source arguments names rows distinct).codec.decode? runtime =
      ((NTy.enum source arguments names rows distinct).plainCodec.decode? runtime).bind fun value =>
        if held : NTy.deepHolds (.enum source arguments names rows distinct) value then some ⟨value, held⟩
        else none := rfl

/-- No literal decodes at an enum type without variants. -/
@[simp] theorem NTy.decode?_enum_nil [Carriers] (source : StructHandle) (names : List String)
    (distinct : names.Nodup) (runtime : RuntimeValue) :
    (NTy.enum source arguments names .nil distinct).codec.decode? runtime = none := by
  cases names <;> rfl

/-- Decoding an integer literal at its own width, in the form the integer
codec unfolds to. -/
@[simp] theorem Codec.specInt_decode?_val (width : IntWidth) (signed : Bool)
    (value : SpecInt width signed) :
    (Codec.specInt width signed).decode? (.integer value.val) = some value :=
  (Codec.specInt width signed).decode_encode value

/-- Decoding the unfolded encoding of a tuple value. -/
@[simp] theorem NTy.decode?_tuple_literal [Carriers] (elements : NRow) (value : HList elements) :
    (NTy.tuple elements).codec.decode? (.tuple value.encode.toArray) = some value :=
  (NTy.tuple elements).codec.decode_encode value

/-- Decoding an encoded value at its own type. -/
@[simp] theorem NTy.decode?_encode [Carriers] (τ : NTy) (value : τ.carrier) :
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
theorem character_tight : Codec.character.Tight := by
  intro raw value h
  cases raw <;> simp only [Codec.character, reduceCtorEq] at h
  split at h
  · cases h; rfl
  · cases h

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
      simp only [Codec.tupleCons, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def, Option.some.injEq] at h
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

theorem ghost_tight {Native : Type} {codec : Codec Native RuntimeValue}
    (tight : codec.Tight) : (Codec.ghost codec).Tight := by
  intro raw value h
  cases raw <;> simp only [Codec.ghost, reduceCtorEq] at h
  case ghost contents => simp only [Codec.ghost, tight _ _ h]

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

/-- A certified codec is tight where its base codec is. -/
theorem certified_tight {Native : Type} {codec : Codec Native RuntimeValue} {holds : Native → Prop}
    (tight : codec.Tight) : (Codec.certified codec holds).Tight := by
  intro raw value h
  simp only [Codec.certified, Option.bind_eq_some_iff] at h
  obtain ⟨decoded, hd, hv⟩ := h
  split at hv
  · cases hv; exact tight _ _ hd
  · cases hv

/-- A pair codec is tight where its components are. -/
theorem pair_tight {Left Right : Type} {left : Codec Left RuntimeValue}
    {right : Codec Right RuntimeValue} (tightLeft : left.Tight) (tightRight : right.Tight) :
    (Codec.pair left right).Tight := by
  intro raw value h
  cases raw with
  | tuple values =>
      simp only [Codec.pair] at h
      split at h
      · rename_i current prophecy listed
        simp only [Option.bind_eq_some_iff, Option.map_eq_some_iff] at h
        obtain ⟨c, hc, p, hp, rfl⟩ := h
        simp only [Codec.pair, tightLeft _ _ hc, tightRight _ _ hp]
        exact congrArg RuntimeValue.tuple (Array.toList_inj.mp (by simp [listed]))
      · cases h
  | _ => cases h

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

/-- Decoding an entry tightly: the decoded entry encodes to the runtime entry. -/
theorem decodeEntry?_tight {Key Value : Type} {key : Codec Key RuntimeValue}
    {value : Codec Value RuntimeValue} (keyTight : key.Tight) (valueTight : value.Tight)
    (raw : RuntimeValue) (entry : Key × Value)
    (decoded : Codec.decodeEntry? key value raw = some entry) :
    RuntimeValue.tuple #[key.encode entry.1, value.encode entry.2] = raw := by
  cases raw <;> simp only [Codec.decodeEntry?, reduceCtorEq] at decoded
  case tuple pair =>
    split at decoded
    · next k v pairList =>
        simp only [Option.bind_eq_some_iff, Option.map_eq_some_iff] at decoded
        obtain ⟨dk, hk, dv, hv, rfl⟩ := decoded
        have pairEq : pair = #[k, v] := by
          rw [← (Array.toArray_toList : pair.toList.toArray = pair), pairList]
        subst pairEq
        simp only [keyTight k dk hk, valueTight v dv hv]
    · cases decoded

theorem entries_tight {Key Value : Type} {key : Codec Key RuntimeValue}
    {value : Codec Value RuntimeValue} (keyTight : key.Tight) (valueTight : value.Tight) :
    (Codec.entries key value).Tight := by
  intro raw entries h
  cases raw <;> simp only [Codec.entries, reduceCtorEq] at h
  case vector values =>
    simp only [bind, Option.bind_eq_some_iff] at h
    obtain ⟨decoded, h, bound⟩ := h
    split at bound <;> try contradiction
    cases bound
    suffices decoded : ∀ (raws : List RuntimeValue) (entries : List (Key × Value)),
        raws.mapM (Codec.decodeEntry? key value) = some entries →
        entries.map (fun entry => RuntimeValue.tuple #[key.encode entry.1, value.encode entry.2]) =
          raws by
      simp only [Codec.entries, decoded values.toList _ h, Array.toArray_toList]
    intro raws
    induction raws with
    | nil => intro entries h; cases h; rfl
    | cons first rest ih =>
        intro entries h
        rw [List.mapM_cons] at h
        simp only [Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def, Option.some.injEq] at h
        obtain ⟨entry, hd, decoded, hrest, rfl⟩ := h
        simp only [List.map_cons, decodeEntry?_tight keyTight valueTight first entry hd, ih decoded hrest]

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
      simp only [Codec.boundedVector, List.map_toArray, decodeElements?_tight tight _ _ hd, Array.toArray_toList]
    · cases hv
  all_goals simp [Codec.boundedVector] at h

/-- Equal-extent checking does not change a successfully decoded representation. -/
theorem sliceRef_tight {Native : Type} {codec : Codec Native RuntimeValue}
    (tight : codec.Tight) : (Codec.sliceRef codec).Tight := by
  intro raw value h
  change ((Codec.prophecyPair (Codec.boundedVector codec)).decode? raw).bind
    (fun pair => SpecSliceRef.ofVectors? pair.1 pair.2) = some value at h
  obtain ⟨⟨current, final⟩, decoded, sized⟩ := Option.bind_eq_some_iff.mp h
  obtain ⟨hc, hf⟩ := SpecSliceRef.ofVectors?_sound sized
  change RuntimeValue.tuple _ = raw
  rw [hc, hf]
  exact prophecyPair_tight (boundedVector_tight tight) raw (current, final) decoded

/-- Tightness of the row codecs of an enum's variants. -/
def RowCodecsTight [Carriers] : (rows : NRows) → RowCodecs rows → Prop
  | .nil, _ => True
  | .cons _ rest, (codec, codecs) => codec.Tight ∧ RowCodecsTight rest codecs

theorem variantDecode?_tight [Carriers] (source : StructHandle) : (names : List String) → (rows : NRows) →
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

/-- Tightness of the row codecs of an enum's variants. -/
def RowCodecsPlainTight [Carriers] : (rows : NRows) → RowCodecsPlain rows → Prop
  | .nil, _ => True
  | .cons _ rest, (codec, codecs) => codec.Tight ∧ RowCodecsPlainTight rest codecs

theorem variantDecodePlain?_tight [Carriers] (source : StructHandle) : (names : List String) → (rows : NRows) →
    (codecs : RowCodecsPlain rows) → RowCodecsPlainTight rows codecs →
    ∀ raw value, variantDecodePlain? source names rows codecs raw = some value →
      variantEncodePlain source names rows codecs value = raw
  | _, .nil, _, _, _, value, _ => nomatch value
  | [], .cons _ _, _, _, _, value, _ => nomatch value
  | name :: names, .cons fields rest, (codec, codecs), ⟨tight, tights⟩, raw, value, h => by
      unfold variantDecodePlain? variantDecodePlainOf? at h
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
          exact variantDecodePlain?_tight source names rest codecs tights _ _ hd
      · cases h

end Operations

mutual
theorem array_tight {Native : Type} {codec : Codec Native RuntimeValue} {length : Nat}
    (tight : codec.Tight) : (Codec.array codec length).Tight := by
  intro raw value h
  simp only [Codec.array, Option.bind_eq_some_iff] at h
  obtain ⟨vector, decoded, sized⟩ := h
  split at sized
  · cases sized
    exact boundedVector_tight tight raw vector decoded
  · cases sized


theorem NTy.plainCodec_tight [Carriers] : (τ : NTy) → τ.plainCodec.Tight
  | .unit => unit_tight
  | .bool => bool_tight
  | .int width signed => specInt_tight (.bits width) signed
  | .num => specInt_tight .unbounded true
  | .address => address_tight
  | .signer => signer_tight
  | .string => string_tight
  | .bytes => bytes_tight
  | .tuple elements => tuple_tight (rowCodecPlain_tight elements)
  | .struct source _ fields => nominalRow_tight source none (rowCodecPlain_tight fields)
  | .enum source _ names rows _ => fun raw value h =>
      variantDecodePlain?_tight source names rows (rowCodecsPlain rows) (rowCodecsPlain_tight rows) raw value h
  | .vector element => boundedVector_tight (NTy.plainCodec_tight element)
  | .table key value => entries_tight (NTy.plainCodec_tight key) (NTy.plainCodec_tight value)
  | .ghost slot => ghost_tight (NTy.plainCodec_tight slot)
  | .ref referent => prophecyPair_tight (NTy.plainCodec_tight referent)
  | .param index => (Carriers.lent index).tight
  | .function parameters shared results => (Carriers.functions parameters shared results).tight
  | .char => character_tight
  | .slice element => boundedVector_tight (NTy.plainCodec_tight element)
  | .array element _ => array_tight (NTy.plainCodec_tight element)
  | .sliceRef element => sliceRef_tight (NTy.plainCodec_tight element)
  | .nominal source arguments productive => (Carriers.nominals source arguments productive).tight
  | .observed referent => NTy.plainCodec_tight referent

theorem rowCodecPlain_tight [Carriers] : (row : NRow) → (rowCodecPlain row).Tight
  | .nil => tupleNil_tight
  | .cons τ rest => tupleCons_tight (NTy.plainCodec_tight τ) (rowCodecPlain_tight rest)

theorem rowCodecsPlain_tight [Carriers] : (rows : NRows) → RowCodecsPlainTight rows (rowCodecsPlain rows)
  | .nil => trivial
  | .cons fields rest => ⟨rowCodecPlain_tight fields, rowCodecsPlain_tight rest⟩
end

mutual
theorem NTy.codec_tight [Carriers] : (τ : NTy) → τ.codec.Tight
  | .unit => unit_tight
  | .bool => bool_tight
  | .int width signed => specInt_tight (.bits width) signed
  | .num => specInt_tight .unbounded true
  | .address => address_tight
  | .signer => signer_tight
  | .string => string_tight
  | .bytes => bytes_tight
  | .char => character_tight
  | .tuple elements => tuple_tight (rowCodec_tight elements)
  | .struct source arguments fields =>
      certified_tight (NTy.plainCodec_tight (.struct source arguments fields))
  | .enum source arguments names rows distinct =>
      certified_tight (NTy.plainCodec_tight (.enum source arguments names rows distinct))
  | .slice element => boundedVector_tight (NTy.codec_tight element)

  | .vector element => boundedVector_tight (NTy.codec_tight element)
  | .array element _ => array_tight (NTy.codec_tight element)
  | .table key value => entries_tight (NTy.codec_tight key) (NTy.codec_tight value)
  | .ghost slot => ghost_tight (NTy.codec_tight slot)
  | .observed referent => NTy.codec_tight referent
  | .sliceRef element => sliceRef_tight (NTy.codec_tight element)
  | .ref referent => pair_tight (NTy.plainCodec_tight referent) (NTy.codec_tight referent)

  | .param index => Carriers.tight index
  | .function parameters shared results => (Carriers.functions parameters shared results).tight
  | .nominal source arguments productive => (Carriers.nominals source arguments productive).tight

theorem rowCodec_tight [Carriers] : (row : NRow) → (rowCodec row).Tight
  | .nil => tupleNil_tight
  | .cons τ rest => tupleCons_tight (NTy.codec_tight τ) (rowCodec_tight rest)

theorem rowCodecs_tight [Carriers] : (rows : NRows) → RowCodecsTight rows (rowCodecs rows)
  | .nil => trivial
  | .cons fields rest => ⟨rowCodec_tight fields, rowCodecs_tight rest⟩
end

/-- The certified codec decodes a lent value's encoding exactly where the
data invariants hold of it, and then to its certification: Term's
`NTy.certify` of such a value is `some (NTy.ofDeepHolds …)`. -/
theorem NTy.decode?_plainEncode [Carriers] (τ : NTy) (value : τ.plain)
    (holds : NTy.deepHolds τ value) :
    τ.codec.decode? (NTy.plainEncode τ value) = some (NTy.ofDeepHolds τ value holds) := by
  rw [← NTy.plainEncode_ofDeepHolds τ value holds]
  exact τ.codec.decode_encode _

/-- And it decodes only there: the obligation a certifying step owes is the
data invariants, at every type and not only at a struct or a variant. -/
theorem NTy.decode?_plainEncode_iff [Carriers] (τ : NTy) (value : τ.plain) :
    (∃ certified, τ.codec.decode? (NTy.plainEncode τ value) = some certified) ↔
      NTy.deepHolds τ value := by
  constructor
  · intro ⟨certified, decoded⟩
    have encoded : τ.codec.encode certified = NTy.plainEncode τ value :=
      NTy.codec_tight τ _ _ decoded
    have forgotten : τ.forget certified = value := by
      apply NTy.plainEncode_injective τ
      rw [NTy.plainEncode_forget, NTy.codec_encode] at *
      exact encoded
    exact forgotten ▸ τ.deepHolds_forget certified
  · intro holds
    exact ⟨NTy.ofDeepHolds τ value holds, NTy.decode?_plainEncode τ value holds⟩


/-! ## Encoded vectors in clauses

A specification function speaks about a vector's encoding: its values
mapped through the element codec, compared with another such encoding or
with a runtime literal. Both are equalities of the native values: two
encodings agree exactly when the vectors do, and an encoding is a literal
exactly when the literal decodes to the values, which holds for a tight
codec and lets the normalizer decode the literal element by element. -/

theorem map_encode_eq_map_encode_iff (codec : Codec Native RuntimeValue)
    (left right : Array Native) :
    left.map codec.encode = right.map codec.encode ↔ left = right :=
  Array.map_inj_right fun _ _ h => codec.encode_injective h

/-- What decodes element by element under a tight codec is the encoding of
what it decodes to. -/
theorem eq_map_encode_of_decodeElements? {codec : Codec Native RuntimeValue} (tight : codec.Tight) :
    ∀ (raw : List RuntimeValue) (decoded : List Native),
      decodeElements? codec raw = some decoded → raw = decoded.map codec.encode
  | [], decoded, h => by
      simp only [decodeElements?_nil, Option.some.injEq] at h
      subst h; rfl
  | value :: rest, decoded, h => by
      simp only [decodeElements?_cons, Option.bind_eq_some_iff, Option.map_eq_some_iff] at h
      obtain ⟨head, decodedHead, tail, decodedTail, rfl⟩ := h
      rw [List.map_cons, ← eq_map_encode_of_decodeElements? tight rest tail decodedTail,
        tight value head decodedHead]

theorem map_encode_eq_toArray_iff_of_tight {codec : Codec Native RuntimeValue} (tight : codec.Tight)
    (values : Array Native) (raw : List RuntimeValue) :
    values.map codec.encode = raw.toArray ↔
      ∃ decoded, decodeElements? codec raw = some decoded ∧ values = decoded.toArray := by
  constructor
  · intro h
    refine ⟨values.toList, ?_, by simp⟩
    have : raw = values.toList.map codec.encode := by
      rw [← Array.toList_map, h, List.toList_toArray]
    rw [this, decodeElements?_map_encode]
  · rintro ⟨decoded, h, rfl⟩
    rw [eq_map_encode_of_decodeElements? tight raw decoded h, List.map_toArray]

theorem toArray_eq_map_encode_iff_of_tight {codec : Codec Native RuntimeValue} (tight : codec.Tight)
    (values : Array Native) (raw : List RuntimeValue) :
    raw.toArray = values.map codec.encode ↔
      ∃ decoded, decodeElements? codec raw = some decoded ∧ values = decoded.toArray := by
  rw [eq_comm, map_encode_eq_toArray_iff_of_tight tight]

/-- The literal cases the normalizer meets: the scalar codecs a clause's
type unfolds to, and the codec of any other type. The conclusion is at the
values, the form the deciders read vectors in. -/
theorem map_specInt_encode_eq_toArray_iff (width : IntWidth) (signed : Bool)
    (values : Array (SpecInt width signed)) (raw : List RuntimeValue) :
    values.map (Codec.specInt width signed).encode = raw.toArray ↔
      ∃ decoded, decodeElements? (Codec.specInt width signed) raw = some decoded ∧
        values = decoded.toArray :=
  map_encode_eq_toArray_iff_of_tight (specInt_tight width signed) values raw

theorem toArray_eq_map_specInt_encode_iff (width : IntWidth) (signed : Bool)
    (values : Array (SpecInt width signed)) (raw : List RuntimeValue) :
    raw.toArray = values.map (Codec.specInt width signed).encode ↔
      ∃ decoded, decodeElements? (Codec.specInt width signed) raw = some decoded ∧
        values = decoded.toArray :=
  toArray_eq_map_encode_iff_of_tight (specInt_tight width signed) values raw

theorem map_bool_encode_eq_toArray_iff (values : Array Bool) (raw : List RuntimeValue) :
    values.map Codec.bool.encode = raw.toArray ↔
      ∃ decoded, decodeElements? Codec.bool raw = some decoded ∧ values = decoded.toArray :=
  map_encode_eq_toArray_iff_of_tight bool_tight values raw

theorem map_address_encode_eq_toArray_iff (values : Array String) (raw : List RuntimeValue) :
    values.map Codec.address.encode = raw.toArray ↔
      ∃ decoded, decodeElements? Codec.address raw = some decoded ∧ values = decoded.toArray :=
  map_encode_eq_toArray_iff_of_tight address_tight values raw

theorem map_codec_encode_eq_toArray_iff [Carriers] (τ : NTy) (values : Array τ.carrier)
    (raw : List RuntimeValue) :
    values.map τ.codec.encode = raw.toArray ↔
      ∃ decoded, decodeElements? τ.codec raw = some decoded ∧ values = decoded.toArray :=
  map_encode_eq_toArray_iff_of_tight (NTy.codec_tight τ) values raw

theorem toArray_eq_map_codec_encode_iff [Carriers] (τ : NTy) (values : Array τ.carrier)
    (raw : List RuntimeValue) :
    raw.toArray = values.map τ.codec.encode ↔
      ∃ decoded, decodeElements? τ.codec raw = some decoded ∧ values = decoded.toArray :=
  toArray_eq_map_encode_iff_of_tight (NTy.codec_tight τ) values raw

/-! ## Quantifiers over a literal range

A clause quantifying over a range with literal bounds is its instances,
one per position, when the range is short: the normalizer splits the first
position off until the range is empty, deciding the literal comparisons. -/

theorem forall_int_range_split {P : Int → Prop} {low high : Int} (lt : low < high)
    (short : high ≤ low + 64) :
    (∀ i, low ≤ i → i < high → P i) ↔ P low ∧ ∀ i, low + 1 ≤ i → i < high → P i := by
  constructor
  · intro h
    exact ⟨h low (Int.le_refl low) lt, fun i above below => h i (by omega) below⟩
  · rintro ⟨first, rest⟩ i above below
    by_cases at_low : i = low
    · subst at_low; exact first
    · exact rest i (by omega) below

theorem forall_int_range_empty {P : Int → Prop} {low high : Int} (le : high ≤ low) :
    (∀ i, low ≤ i → i < high → P i) ↔ True :=
  iff_true_intro fun _ above below => absurd (Int.lt_of_le_of_lt above below) (Int.not_lt.mpr le)

/-- An encoding equation is a decoding equation: the codecs are tight. -/
theorem NTy.encode_eq_iff [Carriers] (τ : NTy) (value : τ.carrier) (raw : RuntimeValue) :
    τ.encode value = raw ↔ τ.codec.decode? raw = some value :=
  ⟨τ.decode?_of_encode, τ.codec_tight raw value⟩


section Equality
variable [Carriers]

/-! ## Structural equality decides equality -/


mutual
/-- Whether a value of a type carries a data invariant at any depth: a
struct or enum whose declaration states one (`declares`), or one its
fields, elements, or components hold. A type parameter's is not known, and
a reference's referent belongs to the value it borrows. -/
def NTy.carriesInvariant (declares : StructHandle → Bool) : NTy → Bool
  | .tuple elements => elements.carriesInvariant declares
  | .struct source _ fields => declares source || fields.carriesInvariant declares
  | .enum source _ _ rows _ => declares source || rows.carriesInvariant declares
  | .slice element => element.carriesInvariant declares
  | .vector element => element.carriesInvariant declares
  | .array element _ => element.carriesInvariant declares
  | .table key value => key.carriesInvariant declares || value.carriesInvariant declares
  | .ghost slot => slot.carriesInvariant declares
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char | .param _ | .ref _ | .sliceRef _
  | .function _ _ _ => false
  -- The rows of a nominal are read where it is produced.
  | .nominal source arguments _ => declares source || arguments.carriesInvariant declares
  | .observed referent => referent.carriesInvariant declares

def NRow.carriesInvariant (declares : StructHandle → Bool) : NRow → Bool
  | .nil => false
  | .cons τ rest => τ.carriesInvariant declares || rest.carriesInvariant declares

def NRows.carriesInvariant (declares : StructHandle → Bool) : NRows → Bool
  | .nil => false
  | .cons fields rest => fields.carriesInvariant declares || rest.carriesInvariant declares
end

mutual
/-- Whether a value of a type owns, at any depth, what `wanted` asks for: a nominal,
a type parameter, or a function value, whose captures its type does not show.
Ownership goes through fields, elements and components, not through type
arguments its fields do not hold. A shared reference denotes its referent, so
its referent counts, though dropping it drops nothing. -/
def NTy.owns (wanted : Option StructHandle → Option Nat → Bool) : NTy → Bool
  | .param index => wanted none (some index)
  | .tuple elements => elements.owns wanted
  | .struct source _ fields => wanted (some source) none || fields.owns wanted
  | .enum source _ _ rows _ => wanted (some source) none || rows.owns wanted
  | .slice element => element.owns wanted
  | .vector element => element.owns wanted
  | .array element _ => element.owns wanted
  | .table key value => key.owns wanted || value.owns wanted
  | .ghost slot => slot.owns wanted
  | .observed referent => referent.owns wanted
  | .function _ _ _ => wanted none none
  -- Its rows are not shown either.
  | .nominal source arguments _ =>
      wanted (some source) none || wanted none none || arguments.owns wanted
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char | .ref _ | .sliceRef _ =>
      false

def NRow.owns (wanted : Option StructHandle → Option Nat → Bool) : NRow → Bool
  | .nil => false
  | .cons τ rest => τ.owns wanted || rest.owns wanted

def NRows.owns (wanted : Option StructHandle → Option Nat → Bool) : NRows → Bool
  | .nil => false
  | .cons fields rest => fields.owns wanted || rest.owns wanted
end

omit [Carriers] in
theorem eqb_zip_iff {α : Type} (eqb : α → α → Bool) (sound : ∀ a b, eqb a b = true ↔ a = b) :
    ∀ (l r : List α), ((l.length == r.length) && (l.zip r).all fun p => eqb p.1 p.2) = true ↔ l = r
  | [], [] => by simp
  | [], _ :: _ => by simp
  | _ :: _, [] => by simp
  | a :: l, b :: r => by
      have ih := eqb_zip_iff eqb sound l r
      simp only [List.length_cons, List.zip_cons_cons, List.all_cons, Bool.and_eq_true, beq_iff_eq, Nat.add_right_cancel_iff, List.cons.injEq, sound] at ih ⊢
      constructor
      · rintro ⟨hlen, hab, hall⟩
        exact ⟨hab, ih.mp ⟨hlen, hall⟩⟩
      · rintro ⟨rfl, rfl⟩
        exact ⟨rfl, rfl, (ih.mpr rfl).2⟩

mutual
theorem NTy.plainEqb_iff : (τ : NTy) → τ.refFree = true → τ.ghostFree = true →
    ∀ l r : τ.plain, τ.plainEqb l r = true ↔ l = r
  | .unit, _, _, l, r => by simp [NTy.plainEqb, NTy.plainEqbOf]
  | .bool, _, _, l, r => by simp [NTy.plainEqb, NTy.plainEqbOf]
  | .int width signed, _, _, l, r => by
      simp only [NTy.plainEqb, NTy.plainEqbOf, decide_eq_true_eq]
      exact ⟨fun h => SpecInt.ext h, fun h => h ▸ rfl⟩
  | .num, _, _, l, r => by
      simp only [NTy.plainEqb, NTy.plainEqbOf, decide_eq_true_eq]
      exact ⟨fun h => SpecInt.ext h, fun h => h ▸ rfl⟩
  | .address, _, _, l, r => by simp [NTy.plainEqb, NTy.plainEqbOf]
  | .signer, _, _, l, r => by simp [NTy.plainEqb, NTy.plainEqbOf]
  | .string, _, _, l, r => by simp [NTy.plainEqb, NTy.plainEqbOf]
  | .bytes, _, _, l, r => by simp [NTy.plainEqb, NTy.plainEqbOf]
  | .char, _, _, l, r => by simp [NTy.plainEqb, NTy.plainEqbOf]
  | .tuple elements, free, plain, l, r => rowEqbPlain_iff elements free plain l r
  | .struct _ _ fields, free, plain, l, r => rowEqbPlain_iff fields free plain l r
  | .enum _ _ names rows _, free, plain, l, r => variantEqbPlain_iff names rows free plain l r
  | .vector element, free, plain, l, r => by
      rw [NTy.plainEqb_vector]
      have sound := NTy.plainEqb_iff element free plain
      constructor
      · intro h
        have := (eqb_zip_iff element.plainEqb sound l.values.toList r.values.toList).mp (by
          simpa only [Array.length_toList] using h)
        exact SpecVector.ext (Array.toList_inj.mp this)
      · rintro rfl
        simpa only [Array.length_toList] using
          (eqb_zip_iff element.plainEqb sound l.values.toList l.values.toList).mpr rfl
  | .slice element, free, plain, l, r => by
      rw [NTy.plainEqb_slice]
      have sound := NTy.plainEqb_iff element free plain
      constructor
      · intro h
        have := (eqb_zip_iff element.plainEqb sound l.values.toList r.values.toList).mp (by
          simpa only [Array.length_toList] using h)
        exact SpecVector.ext (Array.toList_inj.mp this)
      · rintro rfl
        simpa only [Array.length_toList] using
          (eqb_zip_iff element.plainEqb sound l.values.toList l.values.toList).mpr rfl
  | .array element length, free, plain, l, r => by
      rw [NTy.plainEqb_array]
      have sound := NTy.plainEqb_iff element free plain
      constructor
      · intro h
        have := (eqb_zip_iff element.plainEqb sound l.vector.values.toList
          r.vector.values.toList).mp (by simpa only [Array.length_toList] using h)
        exact SpecArray.ext (SpecVector.ext (Array.toList_inj.mp this))
      · rintro rfl
        simpa only [Array.length_toList] using
          (eqb_zip_iff element.plainEqb sound l.vector.values.toList
            l.vector.values.toList).mpr rfl
  | .table key value, free, plain, l, r => by
      simp only [NTy.refFree, Bool.and_eq_true] at free
      simp only [NTy.ghostFree, Bool.and_eq_true] at plain
      show (l.entries.length == r.entries.length && (l.entries.zip r.entries).all fun pair =>
        key.plainEqb pair.1.1 pair.2.1 && value.plainEqb pair.1.2 pair.2.2) = true ↔ l = r
      rw [SpecEntries.eq_iff_entries]
      refine eqb_zip_iff (fun a b => key.plainEqb a.1 b.1 && value.plainEqb a.2 b.2) ?_ l.entries r.entries
      intro a b
      rw [Bool.and_eq_true, NTy.plainEqb_iff key free.1 plain.1, NTy.plainEqb_iff value free.2 plain.2]
      exact ⟨fun ⟨k, v⟩ => Prod.ext k v, fun equal => by cases equal; exact ⟨rfl, rfl⟩⟩
  | .ghost _, _, plain, _, _ => by simp [NTy.ghostFree] at plain
  | .observed referent, free, plain, l, r => NTy.plainEqb_iff referent free plain l r
  | .sliceRef _, free, _, _, _ => by simp [NTy.refFree] at free
  | .ref _, free, _, _, _ => by simp [NTy.refFree] at free
  | .param _, _, _, l, r => by simp [NTy.plainEqb, NTy.plainEqbOf]
  | .function _ _ _, _, plain, _, _ => by simp [NTy.ghostFree] at plain
  | .nominal _ _ _, _, plain, _, _ => by simp [NTy.ghostFree] at plain

theorem rowEqbPlain_iff : (row : NRow) → row.refFree = true → row.ghostFree = true →
    ∀ l r : HListPlain row, rowEqbPlain row l r = true ↔ l = r
  | .nil, _, _, l, r => by
      cases l; cases r; simp [rowEqbPlain, rowEqbPlainOf]
  | .cons τ rest, free, plain, l, r => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      simp only [NRow.ghostFree, Bool.and_eq_true] at plain
      obtain ⟨l1, l2⟩ := l
      obtain ⟨r1, r2⟩ := r
      simp only [rowEqbPlain_cons, Bool.and_eq_true, NTy.plainEqb_iff τ free.1 plain.1,
        rowEqbPlain_iff rest free.2 plain.2]
      constructor
      · rintro ⟨rfl, rfl⟩; rfl
      · intro h; injection h with a b; exact ⟨a, b⟩

theorem variantEqbPlain_iff : (names : List String) → (rows : NRows) → rows.refFree = true →
    rows.ghostFree = true →
    ∀ l r : variantPlain names rows, variantEqbPlain names rows l r = true ↔ l = r
  | _, .nil, _, _, l, _ => nomatch l
  | [], .cons _ _, _, _, l, _ => nomatch l
  | _ :: names, .cons fields rest, free, plain, l, r => by
      simp only [NRows.refFree, Bool.and_eq_true] at free
      simp only [NRows.ghostFree, Bool.and_eq_true] at plain
      cases l <;> cases r <;>
        simp only [variantEqbPlain_inl_inl, variantEqbPlain_inr_inr, variantEqbPlain_inl_inr,
          variantEqbPlain_inr_inl, rowEqbPlain_iff fields free.1 plain.1,
          variantEqbPlain_iff names rest free.2 plain.2, Bool.false_eq_true, reduceCtorEq] <;>
        (constructor <;> intro h) <;> first | (subst h; rfl) | (injection h)
end

mutual
theorem NTy.eqb_iff : (τ : NTy) → τ.refFree = true → τ.ghostFree = true →
    ∀ l r : τ.carrier, τ.eqb l r = true ↔ l = r
  | .unit, _, _, l, r => by simp [NTy.eqb]
  | .bool, _, _, l, r => by simp [NTy.eqb]
  | .int width signed, _, _, l, r => by
      simp only [NTy.eqb, decide_eq_true_eq]
      exact ⟨fun h => SpecInt.ext h, fun h => h ▸ rfl⟩
  | .num, _, _, l, r => by
      simp only [NTy.eqb, decide_eq_true_eq]
      exact ⟨fun h => SpecInt.ext h, fun h => h ▸ rfl⟩
  | .address, _, _, l, r => by simp [NTy.eqb]
  | .signer, _, _, l, r => by simp [NTy.eqb]
  | .string, _, _, l, r => by simp [NTy.eqb]
  | .bytes, _, _, l, r => by simp [NTy.eqb]
  | .char, _, _, l, r => by simp [NTy.eqb]
  | .tuple elements, free, plain, l, r => rowEqb_iff elements free plain l r
  | .struct _ _ fields, free, plain, l, r =>
      (rowEqbPlain_iff fields free plain l.val r.val).trans Subtype.ext_iff.symm
  | .enum _ _ names rows _, free, plain, l, r =>
      (variantEqbPlain_iff names rows free plain l.val r.val).trans Subtype.ext_iff.symm
  | .slice element, free, plain, l, r => by
      rw [NTy.eqb_slice]
      have sound := NTy.eqb_iff element free plain
      constructor
      · intro h
        have := (eqb_zip_iff element.eqb sound l.values.toList r.values.toList).mp (by
          simpa only [Array.length_toList] using h)
        exact SpecVector.ext (Array.toList_inj.mp this)
      · rintro rfl
        simpa only [Array.length_toList] using
          (eqb_zip_iff element.eqb sound l.values.toList l.values.toList).mpr rfl

  | .vector element, free, plain, l, r => by
      rw [NTy.eqb_vector]
      have sound := NTy.eqb_iff element free plain
      constructor
      · intro h
        have := (eqb_zip_iff element.eqb sound l.values.toList r.values.toList).mp (by
          simpa only [Array.length_toList] using h)
        exact SpecVector.ext (Array.toList_inj.mp this)
      · rintro rfl
        simpa only [Array.length_toList] using
          (eqb_zip_iff element.eqb sound l.values.toList l.values.toList).mpr rfl
  | .array element length, free, plain, l, r => by
      rw [NTy.eqb_array]
      have sound := NTy.eqb_iff element free plain
      constructor
      · intro h
        have := (eqb_zip_iff element.eqb sound l.vector.values.toList r.vector.values.toList).mp (by
          simpa only [Array.length_toList] using h)
        exact SpecArray.ext (SpecVector.ext (Array.toList_inj.mp this))
      · rintro rfl
        simpa only [Array.length_toList] using
          (eqb_zip_iff element.eqb sound l.vector.values.toList l.vector.values.toList).mpr rfl
  | .table key value, free, plain, l, r => by
      simp only [NTy.refFree, Bool.and_eq_true] at free
      simp only [NTy.ghostFree, Bool.and_eq_true] at plain
      show (l.entries.length == r.entries.length && (l.entries.zip r.entries).all fun pair =>
        key.eqb pair.1.1 pair.2.1 && value.eqb pair.1.2 pair.2.2) = true ↔ l = r
      rw [SpecEntries.eq_iff_entries]
      refine eqb_zip_iff (fun a b => key.eqb a.1 b.1 && value.eqb a.2 b.2) ?_ l.entries r.entries
      intro a b
      rw [Bool.and_eq_true, NTy.eqb_iff key free.1 plain.1, NTy.eqb_iff value free.2 plain.2]
      exact ⟨fun ⟨k, v⟩ => Prod.ext k v, fun equal => by cases equal; exact ⟨rfl, rfl⟩⟩
  | .ghost _, _, plain, _, _ => by simp [NTy.ghostFree] at plain
  | .observed referent, free, plain, l, r => NTy.eqb_iff referent free plain l r
  | .sliceRef _, free, _, _, _ => by simp [NTy.refFree] at free
  | .ref _, free, _, _, _ => by simp [NTy.refFree] at free
  | .param _, _, _, l, r => by simp [NTy.eqb]
  | .function _ _ _, _, plain, _, _ | .nominal _ _ _, _, plain, _, _ => by simp [NTy.ghostFree] at plain

theorem rowEqb_iff : (row : NRow) → row.refFree = true → row.ghostFree = true →
    ∀ l r : HList row, rowEqb row l r = true ↔ l = r
  | .nil, _, _, l, r => by
      cases l; cases r; simp [rowEqb]
  | .cons τ rest, free, plain, l, r => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      simp only [NRow.ghostFree, Bool.and_eq_true] at plain
      obtain ⟨l1, l2⟩ := l
      obtain ⟨r1, r2⟩ := r
      simp only [rowEqb, Bool.and_eq_true, NTy.eqb_iff τ free.1 plain.1, rowEqb_iff rest free.2 plain.2]
      constructor
      · rintro ⟨rfl, rfl⟩; rfl
      · intro h; injection h with a b; exact ⟨a, b⟩

theorem variantEqb_iff : (names : List String) → (rows : NRows) → rows.refFree = true →
    rows.ghostFree = true →
    ∀ l r : variantCarrier names rows, variantEqb names rows l r = true ↔ l = r
  | _, .nil, _, _, l, _ => nomatch l
  | [], .cons _ _, _, _, l, _ => nomatch l
  | _ :: names, .cons fields rest, free, plain, l, r => by
      simp only [NRows.refFree, Bool.and_eq_true] at free
      simp only [NRows.ghostFree, Bool.and_eq_true] at plain
      cases l <;> cases r <;>
        simp only [variantEqb, Sum.elim_inl, Sum.elim_inr, rowEqb_iff fields free.1 plain.1,
          variantEqb_iff names rest free.2 plain.2, Bool.false_eq_true, reduceCtorEq] <;>
        (constructor <;> intro h) <;> first | (subst h; rfl) | (injection h)
end


/-- Structural equality of vectors over a reference-free element type
holding no ghost slot decides equality; a spec literal then meets a
symbolic vector as a decoding. -/
theorem NTy.eqb_vector_decide (element : NTy) (free : element.refFree = true)
    (plain : element.ghostFree = true) (left right : SpecVector element.carrier) :
    NTy.eqb (.vector element) left right =
      @decide (left = right) (NTy.decEq (.vector element) left right) := by
  have := NTy.eqb_iff (.vector element) free plain left right
  by_cases equal : left = right
  · rw [@decide_eq_true _ (NTy.decEq (.vector element) left right) equal]; exact this.mpr equal
  · rw [@decide_eq_false _ (NTy.decEq (.vector element) left right) equal]
    exact Bool.eq_false_iff.mpr (fun h => equal (this.mp h))

/-- The same for tables over reference-free key and value types. -/
theorem NTy.eqb_table_decide (key value : NTy) (free : (NTy.table key value).refFree = true)
    (plain : (NTy.table key value).ghostFree = true)
    (left right : (NTy.table key value).carrier) :
    NTy.eqb (.table key value) left right =
      @decide (left = right) (NTy.decEq (.table key value) left right) := by
  have := NTy.eqb_iff (.table key value) free plain left right
  by_cases equal : left = right
  · rw [@decide_eq_true _ (NTy.decEq (.table key value) left right) equal]; exact this.mpr equal
  · rw [@decide_eq_false _ (NTy.decEq (.table key value) left right) equal]
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

mutual
/-- At an abstract family every parameter stands for itself. -/
theorem NTy.substWith_param : (τ : NTy) → τ.substWith (fun index => .param index) = τ
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char | .param _ => rfl
  | .tuple elements => congrArg NTy.tuple (NRow.substWith_param elements)
  | .struct source arguments fields => by
      simp only [NTy.substWith, NRow.substWith_param arguments, NRow.substWith_param fields]
  | .enum source arguments names rows distinct => by
      simp only [NTy.substWith, NRow.substWith_param arguments, NRows.substWith_param rows]
  | .slice element => congrArg NTy.slice (NTy.substWith_param element)
  | .vector element => congrArg NTy.vector (NTy.substWith_param element)
  | .array element length => congrArg (NTy.array · length) (NTy.substWith_param element)
  | .table key value => by
      simp only [NTy.substWith, NTy.substWith_param key, NTy.substWith_param value]
  | .ghost slot => congrArg NTy.ghost (NTy.substWith_param slot)
  | .observed referent => congrArg NTy.observed (NTy.substWith_param referent)
  | .sliceRef referent => congrArg NTy.sliceRef (NTy.substWith_param referent)
  | .ref referent => congrArg NTy.ref (NTy.substWith_param referent)
  | .function parameters shared results => by
      simp only [NTy.substWith, NRow.substWith_param parameters, NRow.substWith_param results,
        Sharing.atWith_unmarked NTy.isObserved_param]
  | .nominal source arguments productive => by
      simp only [NTy.substWith, NRow.substWith_param arguments]

/-- An argument read at the parameters is itself, its mark kept. -/
theorem NTy.argumentWith_param : (τ : NTy) → τ.argumentWith (fun index => .param index) = τ
  | .param _ => rfl
  | .observed referent => congrArg NTy.observed (NTy.substWith_param referent)
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char
  | .tuple _ | .struct _ _ _ | .enum _ _ _ _ _ | .slice _ | .vector _ | .array _ _
  | .table _ _ | .ghost _ | .ref _ | .sliceRef _ | .function _ _ _
  | .nominal _ _ _ => NTy.substWith_param _

theorem NRow.substWith_param : (row : NRow) → row.substWith (fun index => .param index) = row
  | .nil => rfl
  | .cons τ rest => by
      simp only [NRow.substWith, NTy.substWith_param τ, NRow.substWith_param rest]

theorem NRows.substWith_param : (rows : NRows) → rows.substWith (fun index => .param index) = rows
  | .nil => rfl
  | .cons fields rest => by
      simp only [NRows.substWith, NRow.substWith_param fields, NRows.substWith_param rest]
end

/-- A slot mentioning no parameter is shared as declared. -/
theorem NTy.sharedAtWith_paramFree {types : Nat → NTy} {declared : Bool} :
    (τ : NTy) → τ.paramFree = true → τ.sharedAtWith types declared = declared
  | .param _, free => by simp [NTy.paramFree] at free
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _
  | .bytes, _ | .char, _ | .tuple _, _ | .struct _ _ _, _ | .enum _ _ _ _ _, _ | .slice _, _
  | .vector _, _ | .array _ _, _ | .table _ _, _ | .ghost _, _ | .ref _, _ | .sliceRef _, _
  | .observed _, _ | .function _ _ _, _ | .nominal _ _ _, _ => rfl

/-- A row of slots mentioning no parameter is shared as declared. -/
theorem Sharing.atRow_paramFree {types : Nat → NTy} : (flags : List Bool) → (row : NRow) →
    row.paramFree = true → Sharing.atRow types flags row = flags
  | [], _, _ => rfl
  | _ :: rest, .nil, _ => by
      simp only [Sharing.atRow, Sharing.atRow_paramFree rest .nil rfl]
  | _ :: rest, .cons τ row, free => by
      simp only [NRow.paramFree, Bool.and_eq_true] at free
      simp only [Sharing.atRow, NTy.sharedAtWith_paramFree τ free.1,
        Sharing.atRow_paramFree rest row free.2]

/-- A function type mentioning no parameter keeps its sharing at a family. -/
theorem Sharing.atWith_paramFree {types : Nat → NTy} {parameters results : NRow}
    {shared : Sharing} (left : parameters.paramFree = true) (right : results.paramFree = true) :
    Sharing.atWith types parameters shared results = shared := by
  simp only [Sharing.atWith, Sharing.atRow_paramFree _ parameters left,
    Sharing.atRow_paramFree _ results right]

mutual
/-- A type without parameters is what every family says it is. -/
theorem NTy.substWith_paramFree (types : Nat → NTy) :
    (τ : NTy) → τ.paramFree = true → τ.substWith types = τ
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _ | .bytes, _ | .char, _ => rfl
  | .param _, free => by simp [NTy.paramFree] at free
  | .tuple elements, free => by
      simp only [NTy.paramFree] at free
      simp only [NTy.substWith, NRow.substWith_paramFree types elements free]
  | .struct source arguments fields, free => by
      simp only [NTy.paramFree, Bool.and_eq_true] at free
      simp only [NTy.substWith, NRow.substWith_paramFree types arguments free.1, NRow.substWith_paramFree types fields free.2]
  | .enum source arguments names rows distinct, free => by
      simp only [NTy.paramFree, Bool.and_eq_true] at free
      simp only [NTy.substWith, NRow.substWith_paramFree types arguments free.1,
        NRows.substWith_paramFree types rows free.2]
  | .slice element, free => by
      simp only [NTy.paramFree] at free
      simp only [NTy.substWith, NTy.substWith_paramFree types element free]
  | .vector element, free => by
      simp only [NTy.paramFree] at free
      simp only [NTy.substWith, NTy.substWith_paramFree types element free]
  | .array element _, free => by
      simp only [NTy.paramFree] at free
      simp only [NTy.substWith, NTy.substWith_paramFree types element free]
  | .table key value, free => by
      simp only [NTy.paramFree, Bool.and_eq_true] at free
      simp only [NTy.substWith, NTy.substWith_paramFree types key free.1, NTy.substWith_paramFree types value free.2]
  | .ghost slot, free => by
      simp only [NTy.paramFree] at free
      simp only [NTy.substWith, NTy.substWith_paramFree types slot free]
  | .observed referent, free => by
      simp only [NTy.paramFree] at free
      simp only [NTy.substWith, NTy.substWith_paramFree types referent free]
  | .sliceRef referent, free => by
      simp only [NTy.paramFree] at free
      simp only [NTy.substWith, NTy.substWith_paramFree types referent free]
  | .ref referent, free => by
      simp only [NTy.paramFree] at free
      simp only [NTy.substWith, NTy.substWith_paramFree types referent free]
  | .function parameters shared results, free => by
      simp only [NTy.paramFree, Bool.and_eq_true] at free
      simp only [NTy.substWith, NRow.substWith_paramFree types parameters free.1,
        NRow.substWith_paramFree types results free.2,
        Sharing.atWith_paramFree free.1 free.2]
  | .nominal source arguments productive, free => by
      simp only [NTy.paramFree] at free
      simp only [NTy.substWith, NRow.substWith_paramFree types arguments free]

theorem NRow.substWith_paramFree (types : Nat → NTy) :
    (row : NRow) → row.paramFree = true → row.substWith types = row
  | .nil, _ => rfl
  | .cons τ rest, free => by
      simp only [NRow.paramFree, Bool.and_eq_true] at free
      simp only [NRow.substWith, NTy.substWith_paramFree types τ free.1, NRow.substWith_paramFree types rest free.2]

theorem NRows.substWith_paramFree (types : Nat → NTy) :
    (rows : NRows) → rows.paramFree = true → rows.substWith types = rows
  | .nil, _ => rfl
  | .cons fields rest, free => by
      simp only [NRows.paramFree, Bool.and_eq_true] at free
      simp only [NRows.substWith, NRow.substWith_paramFree types fields free.1, NRows.substWith_paramFree types rest free.2]
end

/-- A shape with its parameters replaced by type arguments. -/
@[reducible] def ResultShape.subst (θ : NRow) : ResultShape → ResultShape
  | .none => .none
  | .one τ => .one (τ.subst θ)

theorem ResultShape.row_subst (θ : NRow) : (shape : ResultShape) →
    (shape.subst θ).row = NRow.subst θ shape.row
  | .none => rfl
  | .one _ => rfl

section Plain
variable [Carriers]

mutual
/-- Every encoding is loan-free: a reference's codec encodes its current
value and prophecy as a pair. -/
theorem NTy.plainEncode_plain : (τ : NTy) → (value : τ.plain) → Plain (τ.plainCodec.encode value)
  | .unit, _ => .unit
  | .bool, value => .bool value
  | .int _ _, value => .integer value.val
  | .num, value => .integer value.val
  | .address, value => .address value
  | .signer, value => .signer value
  | .string, value => .string value
  | .bytes, value => .bytes value
  | .char, value => .character value.val
  | .tuple elements, values => .tuple _ fun element member =>
      rowCodecPlain_plain elements values element (by simpa using member)
  | .struct source arguments fields, values => .nominal source none _ fun field member =>
      rowCodecPlain_plain fields values field (by simpa using member)
  | .enum source arguments names rows _, value => variantEncodePlain_plain source names rows value
  | .vector element, value => .vector _ fun encoded member => by
      obtain ⟨item, _, rfl⟩ := Array.mem_map.mp member
      exact NTy.plainEncode_plain element item
  | .slice element, value => .vector _ fun encoded member => by
      obtain ⟨item, _, rfl⟩ := Array.mem_map.mp member
      exact NTy.plainEncode_plain element item
  | .array element _, value => .vector _ fun encoded member => by
      obtain ⟨item, _, rfl⟩ := Array.mem_map.mp member
      exact NTy.plainEncode_plain element item
  | .table key value, entries => .vector _ fun encoded member => by
      obtain ⟨entry, _, rfl⟩ := List.mem_map.mp (List.mem_toArray.mp member)
      exact .tuple _ fun component member => by
        simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
        rcases member with rfl | rfl
        · exact NTy.plainEncode_plain key entry.1
        · exact NTy.plainEncode_plain value entry.2
  | .ghost slot, value => .ghost _ (NTy.plainEncode_plain slot value)
  | .observed referent, value => NTy.plainEncode_plain referent value
  | .sliceRef element, value => .tuple _ fun encoded member => by
      simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
      rcases member with rfl | rfl <;>
        exact .vector _ fun item member => by
          obtain ⟨item, _, rfl⟩ := Array.mem_map.mp member
          exact NTy.plainEncode_plain element item
  | .ref referent, value => .tuple _ fun encoded member => by
      simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
      rcases member with rfl | rfl
      · exact NTy.plainEncode_plain referent value.1
      · exact NTy.plainEncode_plain referent value.2
  | .param index, value => (Carriers.lent index).plain value
  | .function parameters shared results, value =>
      (Carriers.functions parameters shared results).plain value
  | .nominal source arguments productive, value =>
      (Carriers.nominals source arguments productive).plain value

theorem rowCodecPlain_plain : (row : NRow) → (values : HListPlain row) →
    ∀ encoded ∈ (rowCodecPlain row).encode values, Plain encoded
  | .nil, _, _, member => nomatch member
  | .cons τ rest, values, encoded, member => by
      rcases List.mem_cons.mp member with rfl | later
      · exact NTy.plainEncode_plain τ values.1
      · exact rowCodecPlain_plain rest values.2 encoded later

theorem variantEncodePlain_plain (source : StructHandle) : (names : List String) → (rows : NRows) →
    (value : variantPlain names rows) →
    Plain (variantEncodePlain source names rows (rowCodecsPlain rows) value)
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | name :: names, .cons fields rest, .inl values => .nominal source (some name) _ fun field member =>
      rowCodecPlain_plain fields values field (by simpa using member)
  | _ :: names, .cons _ rest, .inr value => variantEncodePlain_plain source names rest value
end

mutual
/-- Every encoding is loan-free: a reference's codec encodes its current
value and prophecy as a pair. -/
theorem NTy.encode_plain : (τ : NTy) → (value : τ.carrier) → Plain (τ.codec.encode value)
  | .unit, _ => .unit
  | .bool, value => .bool value
  | .int _ _, value => .integer value.val
  | .num, value => .integer value.val
  | .address, value => .address value
  | .signer, value => .signer value
  | .string, value => .string value
  | .bytes, value => .bytes value
  | .char, value => .character value.val
  | .tuple elements, values => .tuple _ fun element member =>
      rowCodec_plain elements values element (by simpa using member)
  | .struct source arguments fields, values =>
      NTy.plainEncode_plain (.struct source arguments fields) values.val
  | .enum source arguments names rows distinct, value =>
      NTy.plainEncode_plain (.enum source arguments names rows distinct) value.val
  | .slice element, value => .vector _ fun encoded member => by
      obtain ⟨item, _, rfl⟩ := Array.mem_map.mp member
      exact NTy.encode_plain element item

  | .vector element, value => .vector _ fun encoded member => by
      obtain ⟨item, _, rfl⟩ := Array.mem_map.mp member
      exact NTy.encode_plain element item
  | .array element _, value => .vector _ fun encoded member => by
      obtain ⟨item, _, rfl⟩ := Array.mem_map.mp member
      exact NTy.encode_plain element item
  | .table key value, entries => .vector _ fun encoded member => by
      obtain ⟨entry, _, rfl⟩ := List.mem_map.mp (List.mem_toArray.mp member)
      exact .tuple _ fun component member => by
        simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
        rcases member with rfl | rfl
        · exact NTy.encode_plain key entry.1
        · exact NTy.encode_plain value entry.2
  | .ghost slot, value => .ghost _ (NTy.encode_plain slot value)
  | .observed referent, value => NTy.encode_plain referent value
  | .sliceRef element, value => .tuple _ fun encoded member => by
      simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
      rcases member with rfl | rfl <;>
        exact .vector _ fun item member => by
          obtain ⟨item, _, rfl⟩ := Array.mem_map.mp member
          exact NTy.encode_plain element item
  | .ref referent, value => .tuple _ fun encoded member => by
      simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
      rcases member with rfl | rfl
      · exact NTy.plainEncode_plain referent value.1
      · exact NTy.encode_plain referent value.2
  | .param index, value => Carriers.plain index value
  | .function parameters shared results, value =>
      (Carriers.functions parameters shared results).plain value
  | .nominal source arguments productive, value =>
      (Carriers.nominals source arguments productive).plain value

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

/-- Both ordinary slots and native collection slots encode loan-free values. -/
theorem ResourceType.encode_plain {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
    (resource : ResourceType) (value : resource.carrier unit) :
    Plain (resource.encode value) := by
  rcases resource with ⟨type, arguments, kind⟩
  cases kind with
  | value => exact @NTy.encode_plain (Carriers.runtime unit) type value
  | collection =>
      refine .vector _ fun encoded member => ?_
      obtain ⟨element, _, rfl⟩ := Array.mem_map.mp member
      exact @NTy.encode_plain (Carriers.runtime unit) type element

section Admits

variable (unit : Validation.ValidatedUnit) [FunctionCarriers unit]

theorem NTy.admitsEach_of_forall {τ : NTy} : (values : List RuntimeValue) →
    (∀ value ∈ values, NTy.admits unit τ value = true) → NTy.admitsEach unit τ values = true
  | [], _ => by simp [NTy.admitsEach]
  | value :: values, admitted => by
      simp only [NTy.admitsEach, admitted value List.mem_cons_self, Bool.true_and]
      exact NTy.admitsEach_of_forall values fun other member =>
        admitted other (List.mem_cons_of_mem _ member)

mutual
/-- The runtime family's encoding of a value admits its type. -/
theorem NTy.plainAdmits_encode : (τ : NTy) → (value : @NTy.plain (Carriers.runtime unit) τ) →
    NTy.admits unit τ ((@NTy.plainCodec (Carriers.runtime unit) τ).encode value) = true
  | .unit, _ => by simp [NTy.plainCodec, NTy.plainCodecOf, Codec.unit, NTy.admits]
  | .bool, _ => by simp [NTy.plainCodec, NTy.plainCodecOf, Codec.bool, NTy.admits]
  | .int width signed, value => by
      simp [NTy.plainCodec, NTy.plainCodecOf, Codec.specInt, NTy.admits, decodeInt?_val]
  | .num, _ => by simp [NTy.plainCodec, NTy.plainCodecOf, Codec.specInt, NTy.admits]
  | .address, _ => by simp [NTy.plainCodec, NTy.plainCodecOf, Codec.address, NTy.admits]
  | .signer, _ => by simp [NTy.plainCodec, NTy.plainCodecOf, Codec.signer, NTy.admits]
  | .string, _ => by simp [NTy.plainCodec, NTy.plainCodecOf, Codec.string, NTy.admits]
  | .bytes, _ => by simp [NTy.plainCodec, NTy.plainCodecOf, Codec.bytes, NTy.admits]
  | .char, value => by
      simpa [NTy.plainCodec, NTy.plainCodecOf, Codec.character, NTy.admits] using value.valid
  | .tuple elements, values => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.tuple, NTy.admits]
      exact NRow.plainAdmits_encode elements values
  | .struct source arguments fields, values => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.nominalRow, NTy.admits, beq_self_eq_true, Bool.true_and]
      exact NRow.plainAdmits_encode fields values
  | .enum source arguments names rows distinct, value => by
      have admitted := variantEncodePlain_admits source names rows distinct value
      simp only [variantEncodePlain] at admitted
      simp only [NTy.plainCodec, NTy.plainCodecOf, variantCodecPlain, variantCodecPlainOf]
      split at admitted
      · next actual name fields encoded =>
          rw [encoded]
          obtain ⟨rfl, -, row, selected, rowAdmitted⟩ := admitted
          simp only [NTy.admits, beq_self_eq_true, selected, rowAdmitted, Bool.true_and]
      · exact admitted.elim
  | .vector element, value => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.boundedVector, NTy.admits, Array.toList_map]
      exact NTy.admitsEach_of_forall unit _ fun encoded member => by
        obtain ⟨item, _, rfl⟩ := List.mem_map.mp member
        exact NTy.plainAdmits_encode element item
  | .slice element, value => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.boundedVector, NTy.admits, Array.toList_map]
      exact NTy.admitsEach_of_forall unit _ fun encoded member => by
        obtain ⟨item, _, rfl⟩ := List.mem_map.mp member
        exact NTy.plainAdmits_encode element item
  | .array element _, value => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.array, Codec.boundedVector, NTy.admits,
        Array.toList_map, Array.size_map, value.size_eq, beq_self_eq_true, Bool.true_and]
      exact NTy.admitsEach_of_forall unit _ fun encoded member => by
        obtain ⟨item, _, rfl⟩ := List.mem_map.mp member
        exact NTy.plainAdmits_encode element item
  | .table key value, entries => by
      show NTy.admits unit (.table key value)
        (@NTy.plainEncode (Carriers.runtime unit) (.table key value) entries) = true
      rw [@NTy.plainEncode_table (Carriers.runtime unit)]
      simp only [NTy.admits, List.toList_toArray]
      induction entries.entries with
      | nil => simp [NTy.admitsEntries]
      | cons entry rest ih =>
          simp only [List.map_cons, NTy.admitsEntries, NTy.admitsEntry, ih, Bool.and_true, Bool.and_eq_true, List.toList_toArray]
          exact ⟨NTy.plainAdmits_encode key entry.1, NTy.plainAdmits_encode value entry.2⟩
  | .sliceRef element, value => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.sliceRef, Codec.boundedVector, NTy.admits,
        List.toList_toArray, NTy.admitsPair, Array.size_map, value.currentVector_size,
        value.finalVector_size, beq_self_eq_true, Bool.true_and, Bool.and_eq_true, Array.toList_map]
      constructor <;> exact NTy.admitsEach_of_forall unit _ fun encoded member => by
        obtain ⟨item, _, rfl⟩ := List.mem_map.mp member
        exact NTy.plainAdmits_encode element item
  | .ref referent, value => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.prophecyPair, NTy.admits, NTy.admitsPair, NTy.plainAdmits_encode referent value.1, NTy.plainAdmits_encode referent value.2, Bool.and_self]
  | .ghost slot, value => by
      simp only [NTy.plainCodec, NTy.plainCodecOf, Codec.ghost, NTy.admits]
      exact (SemanticOperations.plain?_iff _).mpr (@NTy.plainEncode_plain (Carriers.runtime unit) slot value)
  | .observed _, _ => by simp only [NTy.admits]
  | .param _, _ => by simp [NTy.admits]
  | .function _ _ _, value => FunctionCarriers.admits _ _ _ value
  | .nominal _ _ _, value => value.typed

/-- The runtime family's encoding of a row admits it. -/
theorem NRow.plainAdmits_encode : (row : NRow) → (values : @HListPlain (Carriers.runtime unit) row) →
    NRow.admits unit row ((@rowCodecPlain (Carriers.runtime unit) row).encode values) = true
  | .nil, _ => by simp [rowCodecPlain, rowCodecPlainOf, Codec.tupleNil, NRow.admits]
  | .cons τ rest, values => by
      simp only [rowCodecPlain, rowCodecPlainOf, Codec.tupleCons, NRow.admits, NTy.plainAdmits_encode τ values.1, NRow.plainAdmits_encode rest values.2, Bool.and_self]

/-- The runtime family's encoding of an enum value is a variant the enum's
names select, whose fields admit its row. -/
theorem variantEncodePlain_admits (source : StructHandle) : (names : List String) → (rows : NRows) →
    names.Nodup → (value : @variantPlain (Carriers.runtime unit) names rows) →
    match (@variantEncodePlain (Carriers.runtime unit) source names rows
        (@rowCodecsPlain (Carriers.runtime unit) rows) value : RuntimeValue) with
    | RuntimeValue.nominal actual (some name) fields => actual = source ∧ name ∈ names ∧
        ∃ row, NRows.variant? names rows name = some row ∧ NRow.admits unit row fields.toList = true
    | _ => False
  | _, .nil, _, value => nomatch value
  | [], .cons _ _, _, value => nomatch value
  | name :: names, .cons fields rest, _, .inl values => by
      simp only [variantEncodePlain, variantEncodePlainOf, Sum.elim_inl, Sum.elim_inr, Codec.nominalRow, List.mem_cons, true_or, NRows.variant?, beq_self_eq_true, if_true, Option.some.injEq, exists_eq_left', true_and]
      exact NRow.plainAdmits_encode fields values
  | first :: names, .cons fields rest, distinct, .inr later => by
      have admitted := variantEncodePlain_admits source names rest (List.nodup_cons.mp distinct).2 later
      simp only [variantEncodePlain, rowCodecsPlain] at admitted
      simp only [variantEncodePlain, variantEncodePlainOf, Sum.elim_inl, Sum.elim_inr, rowCodecsPlain, rowCodecsPlainOf]
      split at admitted
      · next actual name values encoded =>
          obtain ⟨rfl, member, row, selected, rowAdmitted⟩ := admitted
          have different : (first == name) = false := by
            simp only [beq_eq_false_iff_ne]
            rintro rfl
            exact (List.nodup_cons.mp distinct).1 member
          simp only [List.mem_cons, member, or_true, true_and, NRows.variant?, different]
          exact ⟨row, selected, rowAdmitted⟩
      · exact admitted.elim
end

mutual
/-- The runtime family's encoding of a value admits its type. -/
theorem NTy.admits_encode : (τ : NTy) → (value : @NTy.carrier (Carriers.runtime unit) τ) →
    NTy.admits unit τ ((@NTy.codec (Carriers.runtime unit) τ).encode value) = true
  | .unit, _ => by simp [NTy.codec, Codec.unit, NTy.admits]
  | .bool, _ => by simp [NTy.codec, Codec.bool, NTy.admits]
  | .int width signed, value => by
      simp [NTy.codec, Codec.specInt, NTy.admits, decodeInt?_val]
  | .num, _ => by simp [NTy.codec, Codec.specInt, NTy.admits]
  | .address, _ => by simp [NTy.codec, Codec.address, NTy.admits]
  | .signer, _ => by simp [NTy.codec, Codec.signer, NTy.admits]
  | .string, _ => by simp [NTy.codec, Codec.string, NTy.admits]
  | .bytes, _ => by simp [NTy.codec, Codec.bytes, NTy.admits]
  | .char, value => by simpa [NTy.codec, Codec.character, NTy.admits] using value.valid
  | .tuple elements, values => by
      simp only [NTy.codec, Codec.tuple, NTy.admits]
      exact NRow.admits_encode elements values
  | .struct source arguments fields, values =>
      NTy.plainAdmits_encode (unit := unit) (.struct source arguments fields) values.val
  | .enum source arguments names rows distinct, value =>
      NTy.plainAdmits_encode (unit := unit) (.enum source arguments names rows distinct) value.val
  | .slice element, value => by
      simp only [NTy.codec, Codec.boundedVector, NTy.admits, Array.toList_map]
      exact NTy.admitsEach_of_forall unit _ fun encoded member => by
        obtain ⟨item, _, rfl⟩ := List.mem_map.mp member
        exact NTy.admits_encode element item

  | .vector element, value => by
      simp only [NTy.codec, Codec.boundedVector, NTy.admits, Array.toList_map]
      exact NTy.admitsEach_of_forall unit _ fun encoded member => by
        obtain ⟨item, _, rfl⟩ := List.mem_map.mp member
        exact NTy.admits_encode element item
  | .array element _, value => by
      simp only [NTy.codec, Codec.array, Codec.boundedVector, NTy.admits, Array.toList_map,
        Array.size_map, value.size_eq, beq_self_eq_true, Bool.true_and]
      exact NTy.admitsEach_of_forall unit _ fun encoded member => by
        obtain ⟨item, _, rfl⟩ := List.mem_map.mp member
        exact NTy.admits_encode element item
  | .table key value, entries => by
      show NTy.admits unit (.table key value)
        (@NTy.encode (Carriers.runtime unit) (.table key value) entries) = true
      rw [@NTy.encode_table (Carriers.runtime unit)]
      simp only [NTy.admits, List.toList_toArray]
      induction entries.entries with
      | nil => simp [NTy.admitsEntries]
      | cons entry rest ih =>
          simp only [List.map_cons, NTy.admitsEntries, NTy.admitsEntry, ih, Bool.and_true, Bool.and_eq_true, List.toList_toArray]
          exact ⟨NTy.admits_encode key entry.1, NTy.admits_encode value entry.2⟩
  | .sliceRef element, value => by
      simp only [NTy.codec, Codec.sliceRef, Codec.boundedVector, NTy.admits,
        List.toList_toArray, NTy.admitsPair, NTy.admits, Array.size_map, value.currentVector_size, value.finalVector_size,
        beq_self_eq_true, Bool.true_and, Bool.and_eq_true, Array.toList_map]
      constructor <;> exact NTy.admitsEach_of_forall unit _ fun encoded member => by
        obtain ⟨item, _, rfl⟩ := List.mem_map.mp member
        exact NTy.admits_encode element item
  | .ref referent, value => by
      simp only [NTy.codec, Codec.pair, NTy.admits, NTy.admitsPair, NTy.plainAdmits_encode (unit := unit) referent value.1, NTy.admits_encode referent value.2, Bool.and_self]
  | .ghost slot, value => by
      simp only [NTy.codec, Codec.ghost, NTy.admits]
      exact (SemanticOperations.plain?_iff _).mpr (@NTy.encode_plain (Carriers.runtime unit) slot value)
  | .observed _, _ => by simp only [NTy.admits]
  | .param _, _ => by simp [NTy.admits]
  | .function _ _ _, value => FunctionCarriers.admits _ _ _ value
  | .nominal _ _ _, value => value.typed

/-- The runtime family's encoding of a row admits it. -/
theorem NRow.admits_encode : (row : NRow) → (values : @HList (Carriers.runtime unit) row) →
    NRow.admits unit row ((@rowCodec (Carriers.runtime unit) row).encode values) = true
  | .nil, _ => by simp [rowCodec, Codec.tupleNil, NRow.admits]
  | .cons τ rest, values => by
      simp only [rowCodec, Codec.tupleCons, NRow.admits, NTy.admits_encode τ values.1, NRow.admits_encode rest values.2, Bool.and_self]

/-- The runtime family's encoding of an enum value is a variant the enum's
names select, whose fields admit its row. -/
theorem variantEncode_admits (source : StructHandle) : (names : List String) → (rows : NRows) →
    names.Nodup → (value : @variantCarrier (Carriers.runtime unit) names rows) →
    match (@variantEncode (Carriers.runtime unit) source names rows
        (@rowCodecs (Carriers.runtime unit) rows) value : RuntimeValue) with
    | RuntimeValue.nominal actual (some name) fields => actual = source ∧ name ∈ names ∧
        ∃ row, NRows.variant? names rows name = some row ∧ NRow.admits unit row fields.toList = true
    | _ => False
  | _, .nil, _, value => nomatch value
  | [], .cons _ _, _, value => nomatch value
  | name :: names, .cons fields rest, _, .inl values => by
      simp only [variantEncode, Sum.elim_inl, Sum.elim_inr, Codec.nominalRow, List.mem_cons, true_or, NRows.variant?, beq_self_eq_true, if_true, Option.some.injEq, exists_eq_left', true_and]
      exact NRow.admits_encode fields values
  | first :: names, .cons fields rest, distinct, .inr later => by
      have admitted := variantEncode_admits source names rest (List.nodup_cons.mp distinct).2 later
      simp only [variantEncode, Sum.elim_inl, Sum.elim_inr, rowCodecs]
      split at admitted
      · next actual name values encoded =>
          obtain ⟨rfl, member, row, selected, rowAdmitted⟩ := admitted
          have different : (first == name) = false := by
            simp only [beq_eq_false_iff_ne]
            rintro rfl
            exact (List.nodup_cons.mp distinct).1 member
          simp only [List.mem_cons, member, or_true, true_and, NRows.variant?, different]
          exact ⟨row, selected, rowAdmitted⟩
      · exact admitted.elim
end

end Admits


mutual
/-- Whether a type carries no certificate anywhere: a plain value of such a
type satisfies the data invariants of its type by its type alone
(`NTy.deepHolds_of_certificateFree`), so a value copied out of a mutable
reference's current is certified without a cut where its type is
certificate-free.  A declaration (`.struct`, `.enum`) states clauses of its
own and a type parameter's values satisfy what its family lends, so neither
is certificate-free; a ghost slot is certificate-free exactly where the
slot is, and a function type always is — its plain and certified values are
the same closures and no invariant reaches them (`NTy.deepHolds`). -/
def NTy.certificateFree : NTy → Bool
  | .struct _ _ _ | .enum _ _ _ _ _ | .param _ | .nominal _ _ _ => false
  | .tuple elements => elements.certificateFree
  | .vector element => element.certificateFree
  | .slice element => element.certificateFree
  | .array element _ => element.certificateFree
  | .table key value => key.certificateFree && value.certificateFree
  | .ref referent => referent.certificateFree
  | .sliceRef element => element.certificateFree
  | .observed referent => referent.certificateFree
  | .ghost slot => slot.certificateFree
  | _ => true

/-- Whether every type of a row carries no certificate
(`NTy.certificateFree`). -/
def NRow.certificateFree : NRow → Bool
  | .nil => true
  | .cons τ rest => τ.certificateFree && rest.certificateFree
end

/-- Whether every row of a variant declaration carries no certificate
(`NRow.certificateFree`). -/
def NRows.certificateFree : NRows → Bool
  | .nil => true
  | .cons row rest => row.certificateFree && rest.certificateFree

mutual
/-- A plain value of a certificate-free type satisfies the data invariants
of its type: at such a type `NTy.deepHolds` is `True` or the conjunction of
its components', and no component states a declaration's clauses
(`NTy.certificateFree`). -/
theorem NTy.deepHolds_of_certificateFree [Carriers] : (τ : NTy) → τ.certificateFree = true →
    (value : τ.plain) → NTy.deepHolds τ value
  | .tuple elements, free, values => HListPlain.deepHolds_of_certificateFree elements free values
  | .vector element, free, vector => fun value _ =>
      NTy.deepHolds_of_certificateFree element free value
  | .slice element, free, vector => fun value _ =>
      NTy.deepHolds_of_certificateFree element free value
  | .array element _, free, array => fun value _ =>
      NTy.deepHolds_of_certificateFree element free value
  | .table key value, free, entries => by
      have parts := Bool.and_eq_true _ _ |>.mp free
      exact fun entry _ => ⟨NTy.deepHolds_of_certificateFree key parts.1 entry.1,
        NTy.deepHolds_of_certificateFree value parts.2 entry.2⟩
  | .ref referent, free, value => NTy.deepHolds_of_certificateFree referent free value.2
  | .sliceRef element, free, value =>
      ⟨fun v _ => NTy.deepHolds_of_certificateFree element free v,
        fun v _ => NTy.deepHolds_of_certificateFree element free v⟩
  | .observed referent, free, value => NTy.deepHolds_of_certificateFree referent free value
  | .ghost slot, free, value => NTy.deepHolds_of_certificateFree slot free value
  | .unit, _, _ | .bool, _, _ | .int _ _, _, _ | .num, _, _ | .address, _, _ | .signer, _, _
  | .string, _, _ | .bytes, _, _ | .char, _, _ | .function _ _ _, _, _ => trivial

/-- Every plain value of a row of certificate-free types satisfies the data
invariants of its row (`NTy.deepHolds_of_certificateFree`). -/
theorem HListPlain.deepHolds_of_certificateFree [Carriers] : (row : NRow) → row.certificateFree = true →
    (values : HListPlain row) → HListPlain.deepHolds row values
  | .nil, _, _ => trivial
  | .cons τ rest, free, values => by
      have parts := Bool.and_eq_true _ _ |>.mp free
      exact ⟨NTy.deepHolds_of_certificateFree τ parts.1 values.1,
        HListPlain.deepHolds_of_certificateFree rest parts.2 values.2⟩
end

/-- Every plain value of a variant declaration whose rows carry no
certificate satisfies the data invariants of those rows: the invariants of a
variant value are the fields' of the variant it is, and a field of a
certificate-free type states none
(`HListPlain.deepHolds_of_certificateFree`). -/
theorem variantPlain.deepHolds_of_certificateFree [Carriers] : (names : List String) →
    (rows : NRows) → rows.certificateFree = true →
    (value : variantPlain names rows) → variantPlain.deepHolds names rows value
  | _, .nil, _, value => nomatch value
  | [], .cons _ _, _, value => nomatch value
  | _ :: _, .cons fields _, free, .inl row => by
      simp only [NRows.certificateFree, Bool.and_eq_true] at free
      exact HListPlain.deepHolds_of_certificateFree fields free.1 row
  | _ :: names, .cons _ rest, free, .inr later => by
      simp only [NRows.certificateFree, Bool.and_eq_true] at free
      exact variantPlain.deepHolds_of_certificateFree names rest free.2 later

/-- A component of a pair whose type is cast: the component cast along its
factor's equation.  The proof a cast carries is immaterial — two proofs of
one equation are the same — so a caller states the equation it has. -/
private theorem cast_fst {A B C D : Type} (pair : (A × B) = (C × D)) (left : A = C)
    (right : B = D) (value : A × B) : (cast pair value).1 = cast left value.1 := by
  subst left; subst right; rfl

private theorem cast_snd {A B C D : Type} (pair : (A × B) = (C × D)) (left : A = C)
    (right : B = D) (value : A × B) : (cast pair value).2 = cast right value.2 := by
  subst left; subst right; rfl

/-- The elements of a vector whose element type is cast: each element
through any map the cast agrees with. -/
private theorem cast_values {A B : Type} (vectors : SpecVector A = SpecVector B) (element : A = B)
    (map : A → B) (agrees : ∀ value, map value = cast element value) (value : SpecVector A) :
    (cast vectors value).values = value.values.map map := by
  subst element
  simp only [cast_eq] at agrees
  simp [funext agrees]

/-- The elements of a fixed-extent array whose element type is cast
(`cast_values`). -/
private theorem cast_array {A B : Type} {length : Nat}
    (arrays : SpecArray A length = SpecArray B length) (element : A = B)
    (map : A → B) (agrees : ∀ value, map value = cast element value)
    (value : SpecArray A length) :
    (cast arrays value).vector.values = value.vector.values.map map := by
  subst element
  simp only [cast_eq] at agrees
  simp [funext agrees]

/-- A slice reference whose element type is cast: the value mapped
(`cast_values`). -/
private theorem cast_sliceRef {A B : Type} (slices : SpecSliceRef A = SpecSliceRef B)
    (element : A = B) (map : A → B) (agrees : ∀ value, map value = cast element value)
    (value : SpecSliceRef A) : cast slices value = value.map map := by
  subst element
  simp only [cast_eq] at agrees
  have identity : map = id := funext fun value => agrees value
  rw [identity, SpecSliceRef.map_id, cast_eq]

/-- The entries of a table whose entry type is cast (`cast_values`). -/
private theorem cast_entries {A B : Type} (tables : SpecEntries A = SpecEntries B) (entry : A = B)
    (map : A → B) (agrees : ∀ value, map value = cast entry value) (value : SpecEntries A) :
    (cast tables value).entries = value.entries.map map := by
  subst entry
  simp only [cast_eq] at agrees
  simp [funext agrees]

mutual
/-- The certified carrier of a certificate-free type is its plain one: the
two families differ only at a declaration and at a type parameter, neither
of which is certificate-free, and at a mutable reference, whose referent is
certificate-free exactly where the reference is (`NTy.certificateFree`).  A
value copied out of a plain current is therefore the plain value cast along
this equation — one proof term whatever the value's size, where certifying
it level by level rebuilds it (`Term.copy`,
`NTy.cast_eq_ofDeepHolds_of_certificateFree`). -/
theorem NTy.carrier_eq_plain_of_certificateFree [Θ : Carriers] : (τ : NTy) →
    τ.certificateFree = true → @NTy.carrier Θ τ = @NTy.plain Θ τ
  | .tuple elements, free => HList.eq_plain_of_certificateFree elements free
  | .vector element, free => by
      show SpecVector (@NTy.carrier Θ element) = SpecVector (@NTy.plain Θ element)
      rw [NTy.carrier_eq_plain_of_certificateFree element free]
  | .slice element, free => by
      show SpecVector (@NTy.carrier Θ element) = SpecVector (@NTy.plain Θ element)
      rw [NTy.carrier_eq_plain_of_certificateFree element free]
  | .array element length, free => by
      show SpecArray (@NTy.carrier Θ element) length = SpecArray (@NTy.plain Θ element) length
      rw [NTy.carrier_eq_plain_of_certificateFree element free]
  | .sliceRef element, free => by
      show SpecSliceRef (@NTy.carrier Θ element) = SpecSliceRef (@NTy.plain Θ element)
      rw [NTy.carrier_eq_plain_of_certificateFree element free]
  | .observed referent, free => NTy.carrier_eq_plain_of_certificateFree referent free
  | .table key value, free => by
      have parts := Bool.and_eq_true _ _ |>.mp free
      show SpecEntries (@NTy.carrier Θ key × @NTy.carrier Θ value) =
        SpecEntries (@NTy.plain Θ key × @NTy.plain Θ value)
      rw [NTy.carrier_eq_plain_of_certificateFree key parts.1,
        NTy.carrier_eq_plain_of_certificateFree value parts.2]
  | .ghost slot, free => NTy.carrier_eq_plain_of_certificateFree slot free
  | .ref referent, free => by
      show (@NTy.plain Θ referent × @NTy.carrier Θ referent) =
        (@NTy.plain Θ referent × @NTy.plain Θ referent)
      rw [NTy.carrier_eq_plain_of_certificateFree referent free]
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _
  | .string, _ | .bytes, _ | .char, _ | .function _ _ _, _ => rfl

/-- The row of certified carriers of a row of certificate-free types is its
row of plain ones (`NTy.carrier_eq_plain_of_certificateFree`). -/
theorem HList.eq_plain_of_certificateFree [Θ : Carriers] : (row : NRow) →
    row.certificateFree = true → @HList Θ row = @HListPlain Θ row
  | .nil, _ => rfl
  | .cons τ rest, free => by
      have parts := Bool.and_eq_true _ _ |>.mp free
      show (@NTy.carrier Θ τ × @HList Θ rest) = (@NTy.plain Θ τ × @HListPlain Θ rest)
      rw [NTy.carrier_eq_plain_of_certificateFree τ parts.1,
        HList.eq_plain_of_certificateFree rest parts.2]
end

mutual
/-- Forgetting the certificates of a value of a certificate-free type is
the value cast along `NTy.carrier_eq_plain_of_certificateFree`: at such a
type the certified and the plain carrier are the same type and `NTy.forget`
is the identity on it.  This is what lets a reading of a copy out of a
plain current be a reading of the value copied
(`NTy.forget_cast_of_certificateFree`). -/
theorem NTy.forget_eq_cast_of_certificateFree [Θ : Carriers] : (τ : NTy) →
    (free : τ.certificateFree = true) → (value : @NTy.carrier Θ τ) →
    NTy.forget τ value = cast (NTy.carrier_eq_plain_of_certificateFree τ free) value
  | .tuple elements, free, values => HList.forget_eq_cast_of_certificateFree elements free values
  | .vector element, free, value => by
      have elements := NTy.carrier_eq_plain_of_certificateFree element free
      have vectors : SpecVector (@NTy.carrier Θ element) = SpecVector (@NTy.plain Θ element) := by
        rw [elements]
      show NTy.forget (.vector element) value = cast vectors value
      refine SpecVector.ext ?_
      rw [cast_values vectors elements (NTy.forget element)
        (NTy.forget_eq_cast_of_certificateFree element free) value]
      rfl
  | .slice element, free, value => by
      have elements := NTy.carrier_eq_plain_of_certificateFree element free
      have vectors : SpecVector (@NTy.carrier Θ element) = SpecVector (@NTy.plain Θ element) := by
        rw [elements]
      show NTy.forget (.slice element) value = cast vectors value
      refine SpecVector.ext ?_
      rw [cast_values vectors elements (NTy.forget element)
        (NTy.forget_eq_cast_of_certificateFree element free) value]
      rfl
  | .array element length, free, value => by
      have elements := NTy.carrier_eq_plain_of_certificateFree element free
      have arrays : SpecArray (@NTy.carrier Θ element) length =
          SpecArray (@NTy.plain Θ element) length := by rw [elements]
      show NTy.forget (.array element length) value = cast arrays value
      refine SpecArray.ext (SpecVector.ext ?_)
      rw [cast_array arrays elements (NTy.forget element)
        (NTy.forget_eq_cast_of_certificateFree element free) value]
      rfl
  | .sliceRef element, free, value => by
      have elements := NTy.carrier_eq_plain_of_certificateFree element free
      have slices : SpecSliceRef (@NTy.carrier Θ element) =
          SpecSliceRef (@NTy.plain Θ element) := by rw [elements]
      show NTy.forget (.sliceRef element) value = cast slices value
      rw [cast_sliceRef slices elements (NTy.forget element)
        (NTy.forget_eq_cast_of_certificateFree element free) value]
      rfl
  | .observed referent, free, value =>
      NTy.forget_eq_cast_of_certificateFree referent free value
  | .table key value, free, entries => by
      have parts := Bool.and_eq_true _ _ |>.mp free
      have keys := NTy.carrier_eq_plain_of_certificateFree key parts.1
      have values := NTy.carrier_eq_plain_of_certificateFree value parts.2
      have pairs : (@NTy.carrier Θ key × @NTy.carrier Θ value) =
          (@NTy.plain Θ key × @NTy.plain Θ value) := by rw [keys, values]
      have tables : SpecEntries (@NTy.carrier Θ key × @NTy.carrier Θ value) =
          SpecEntries (@NTy.plain Θ key × @NTy.plain Θ value) := by rw [pairs]
      have agrees : ∀ entry : @NTy.carrier Θ key × @NTy.carrier Θ value,
          (NTy.forget key entry.1, NTy.forget value entry.2) = cast pairs entry := by
        refine fun entry => Prod.ext ?_ ?_
        · rw [cast_fst pairs keys values]
          exact NTy.forget_eq_cast_of_certificateFree key parts.1 entry.1
        · rw [cast_snd pairs keys values]
          exact NTy.forget_eq_cast_of_certificateFree value parts.2 entry.2
      show NTy.forget (.table key value) entries = cast tables entries
      refine SpecEntries.ext ?_
      rw [cast_entries tables pairs _ agrees entries]
      rfl
  | .ghost slot, free, value => NTy.forget_eq_cast_of_certificateFree slot free value
  | .ref referent, free, value => by
      have referents := NTy.carrier_eq_plain_of_certificateFree referent free
      have pairs : (@NTy.plain Θ referent × @NTy.carrier Θ referent) =
          (@NTy.plain Θ referent × @NTy.plain Θ referent) := by rw [referents]
      show (value.1, NTy.forget referent value.2) = cast pairs value
      refine Prod.ext ?_ ?_
      · rw [cast_fst pairs rfl referents, cast_eq]
      · rw [cast_snd pairs rfl referents]
        exact NTy.forget_eq_cast_of_certificateFree referent free value.2
  | .unit, _, value | .bool, _, value | .int _ _, _, value | .num, _, value
  | .address, _, value | .signer, _, value | .string, _, value | .bytes, _, value
  | .char, _, value | .function _ _ _, _, value => (cast_eq _ value).symm

/-- Forgetting the certificates of a row of values of certificate-free
types is the row cast along `HList.eq_plain_of_certificateFree`. -/
theorem HList.forget_eq_cast_of_certificateFree [Θ : Carriers] : (row : NRow) →
    (free : row.certificateFree = true) → (values : @HList Θ row) →
    HList.forget row values = cast (HList.eq_plain_of_certificateFree row free) values
  | .nil, _, _ => rfl
  | .cons τ rest, free, values => by
      have parts := Bool.and_eq_true _ _ |>.mp free
      have heads := NTy.carrier_eq_plain_of_certificateFree τ parts.1
      have tails := HList.eq_plain_of_certificateFree rest parts.2
      have pairs : (@NTy.carrier Θ τ × @HList Θ rest) =
          (@NTy.plain Θ τ × @HListPlain Θ rest) := by rw [heads, tails]
      show (NTy.forget τ values.1, HList.forget rest values.2) = cast pairs values
      refine Prod.ext ?_ ?_
      · rw [cast_fst pairs heads tails]
        exact NTy.forget_eq_cast_of_certificateFree τ parts.1 values.1
      · rw [cast_snd pairs heads tails]
        exact HList.forget_eq_cast_of_certificateFree rest parts.2 values.2
end

/-- The plain value a copy out of a plain current carries: the cast is the
identity read through `NTy.forget`
(`NTy.forget_eq_cast_of_certificateFree`), so a reading of the copy is a
reading of the value copied (`Term.copy`). -/
theorem NTy.forget_cast_of_certificateFree [Carriers] (τ : NTy)
    (free : τ.certificateFree = true) (value : τ.plain) :
    NTy.forget τ (cast (NTy.carrier_eq_plain_of_certificateFree τ free).symm value) = value := by
  rw [NTy.forget_eq_cast_of_certificateFree τ free, cast_cast]
  exact cast_eq _ value

/-- The cast and the certification of a plain value of a certificate-free
type are the same value: both forget to it and `NTy.forget` is injective.
The cast is the denotation of a copy out of a plain current because it is
one proof term, not a rebuilt value (`Term.copy`). -/
theorem NTy.cast_eq_ofDeepHolds_of_certificateFree [Carriers] (τ : NTy)
    (free : τ.certificateFree = true) (value : τ.plain) :
    cast (NTy.carrier_eq_plain_of_certificateFree τ free).symm value =
      NTy.ofDeepHolds τ value (NTy.deepHolds_of_certificateFree τ free value) :=
  NTy.forget_injective τ (by
    rw [NTy.forget_cast_of_certificateFree τ free value, NTy.forget_ofDeepHolds])
/-- The empty row has values. -/
theorem NRow.inhabitable_nil : NRow.inhabitable .nil = true := rfl

/-- A row has values where its first type and the rest do: the step a proof
of a row's inhabitability is built by, entry by entry, where the row is not
ground and no decision reaches it (`Skolems.type_inhabitable`). -/
theorem NRow.inhabitable_cons (τ : NTy) (rest : NRow) (head : τ.inhabitable = true)
    (tail : rest.inhabitable = true) : (NRow.cons τ rest).inhabitable = true := by
  simp only [NRow.inhabitable, head, tail, Bool.and_self]

/-- Type arguments of a call: a row whose types have values. -/
abbrev TypeArgs : Type := { row : NRow // row.inhabitable = true }

theorem TypeArgs.inhabitable (θ : TypeArgs) : θ.1.inhabitable = true := θ.2

theorem NRow.getD_inhabitable : (row : NRow) → row.inhabitable = true → (index : Nat) →
    (default : NTy) → default.inhabitable = true → (row.getD index default).inhabitable = true
  | .nil, _, _, _, inhabited => inhabited
  | .cons _ _, inhabitable, 0, _, _ => by
      simp only [NRow.inhabitable, Bool.and_eq_true] at inhabitable
      exact inhabitable.1
  | .cons _ rest, inhabitable, index + 1, default, inhabited => by
      simp only [NRow.inhabitable, Bool.and_eq_true] at inhabitable
      exact NRow.getD_inhabitable rest inhabitable.2 index default inhabited

mutual
/-- A substitution by inhabitable types keeps a type inhabitable. -/
theorem NTy.inhabitable_subst (θ : NRow) (fits : θ.inhabitable = true) :
    (τ : NTy) → τ.inhabitable = true → (τ.subst θ).inhabitable = true
  | .tuple elements, held => NRow.inhabitable_subst θ fits elements held
  | .struct _ _ fields, held => NRow.inhabitable_subst θ fits fields held
  | .enum _ _ names rows _, held => NRows.firstInhabitable_subst θ fits names rows held
  | .ref referent, held => NTy.inhabitable_subst θ fits referent held
  | .ghost slot, held => NTy.inhabitable_subst θ fits slot held
  | .observed referent, held => NTy.inhabitable_subst θ fits referent held
  | .param index, _ =>
      (NTy.inhabitable_unmark _).trans (NRow.getD_inhabitable θ fits index (.param index) rfl)
  | .function _ _ _, held => by simp [NTy.inhabitable] at held
  | .nominal _ arguments _, held => by
      simp only [NTy.subst, NTy.inhabitable, Bool.and_eq_true] at held ⊢
      exact ⟨held.1, NRow.inhabitable_subst θ fits arguments held.2⟩
  | .array element _, held => by
      simp only [NTy.subst, NTy.inhabitable, Bool.and_eq_true] at held ⊢
      exact ⟨held.1, NTy.inhabitable_subst θ fits element held.2⟩
  | .sliceRef _, _ | .slice _, _ | .vector _, _ | .table _ _, _ | .unit, _ | .bool, _ | .num, _ | .address, _ | .signer, _
  | .string, _ | .bytes, _ | .char, _ => rfl
  | .int _ _, held => held

theorem NRow.inhabitable_subst (θ : NRow) (fits : θ.inhabitable = true) :
    (row : NRow) → row.inhabitable = true → (NRow.subst θ row).inhabitable = true
  | .nil, _ => rfl
  | .cons τ rest, held => by
      simp only [NRow.subst, NRow.inhabitable, Bool.and_eq_true] at held ⊢
      exact ⟨NTy.inhabitable_subst θ fits τ held.1, NRow.inhabitable_subst θ fits rest held.2⟩

theorem NRows.firstInhabitable_subst (θ : NRow) (fits : θ.inhabitable = true) :
    (names : List String) → (rows : NRows) → NRows.firstInhabitable names rows = true →
      NRows.firstInhabitable names (NRows.subst θ rows) = true
  | _ :: _, .cons fields _, held => NRow.inhabitable_subst θ fits fields held
  | [], _, held => by simp [NRows.firstInhabitable] at held
  | _ :: _, .nil, held => by simp [NRows.firstInhabitable] at held
end
section Inhabitant
variable [Θ : Carriers]

omit [Carriers] in
mutual
/-- A plain value of a type that has values. -/
def NTy.plainInhabitantOf (F : PlainFamily) : (τ : NTy) → τ.inhabitable = true →
    NTy.plainOf F τ
  | .unit, _ => ()
  | .bool, _ => false
  | .int width signed, inhabitable =>
      ⟨0, zero_fits width signed (by simpa [NTy.inhabitable] using inhabitable)⟩
  | .num, _ => ⟨0, rfl⟩
  | .address, _ => ""
  | .signer, _ => ""
  | .string, _ => ""
  | .bytes, _ => #[]
  | .char, _ => default
  | .tuple elements, inhabitable =>
      HListPlainOf.inhabitant F elements (by simpa [NTy.inhabitable] using inhabitable)
  | .struct _ _ fields, inhabitable =>
      HListPlainOf.inhabitant F fields (by simpa [NTy.inhabitable] using inhabitable)
  | .enum _ _ names rows _, inhabitable =>
      variantPlainOf.inhabitant F names rows (by simpa [NTy.inhabitable] using inhabitable)
  | .sliceRef _, _ => default
  | .slice _, _ => default
  | .vector _, _ => default
  | .array element length, inhabitable =>
      have held : length < 2 ^ 64 ∧ element.inhabitable = true := by
        simpa [NTy.inhabitable] using inhabitable
      ⟨⟨Array.replicate length (NTy.plainInhabitantOf F element held.2), by simpa using held.1⟩, by simp⟩
  | .table _ _, _ => default
  | .ghost slot, inhabitable =>
      NTy.plainInhabitantOf F slot (by simpa [NTy.inhabitable] using inhabitable)
  | .ref referent, inhabitable =>
      let value := NTy.plainInhabitantOf F referent (by simpa [NTy.inhabitable] using inhabitable)
      (value, value)
  | .param _, _ => default
  | .function _ _ _, inhabitable => absurd inhabitable (by simp [NTy.inhabitable])
  | .nominal source arguments true, inhabitable =>
      F.nominalInhabitant source arguments (by simpa [NTy.inhabitable] using inhabitable)
  | .nominal _ _ false, inhabitable => absurd inhabitable (by simp [NTy.inhabitable])
  | .observed referent, inhabitable => NTy.plainInhabitantOf F referent inhabitable
termination_by structural τ _ => τ


def HListPlainOf.inhabitant (F : PlainFamily) : (row : NRow) → row.inhabitable = true →
    HListPlainOf F row
  | .nil, _ => ()
  | .cons τ rest, inhabitable =>
      (NTy.plainInhabitantOf F τ (by simp_all [NRow.inhabitable]),
        HListPlainOf.inhabitant F rest (by simp_all [NRow.inhabitable]))
termination_by structural row _ => row

def variantPlainOf.inhabitant (F : PlainFamily) : (names : List String) → (rows : NRows) →
    NRows.firstInhabitable names rows = true → variantPlainOf F names rows
  | _ :: _, .cons fields _, inhabitable => .inl (HListPlainOf.inhabitant F fields inhabitable)
  | [], _, inhabitable => absurd inhabitable (by simp [NRows.firstInhabitable])
  | _ :: _, .nil, inhabitable => absurd inhabitable (by simp [NRows.firstInhabitable])
termination_by structural names rows _ => rows
end

/-- A plain value of a type that has values, at a family's lent layer. -/
def NTy.plainInhabitant (τ : NTy) (inhabitable : τ.inhabitable = true) : τ.plain :=
  NTy.plainInhabitantOf Carriers.toPlain τ inhabitable

/-- A plain row of a row whose types have values. -/
def HListPlain.inhabitant (row : NRow) (inhabitable : row.inhabitable = true) : HListPlain row :=
  HListPlainOf.inhabitant Carriers.toPlain row inhabitable

/-- A plain value of the first inhabitable variant. -/
def variantPlain.inhabitant (names : List String) (rows : NRows)
    (inhabitable : NRows.firstInhabitable names rows = true) : variantPlain names rows :=
  variantPlainOf.inhabitant Carriers.toPlain names rows inhabitable

mutual
/-- Every plain value satisfies the data invariants where every
declaration's and every type parameter's is trivially true. -/
theorem NTy.deepHolds_of_trivial
    (own : ∀ source arguments value, Carriers.own source arguments value)
    (params : ∀ index value, (Carriers.lent index).holds value) :
    (τ : NTy) → (value : τ.plain) → NTy.deepHolds τ value
  | .tuple elements, values => HListPlain.deepHolds_of_trivial own params elements values
  | .struct _ _ fields, row => ⟨own _ _ _, HListPlain.deepHolds_of_trivial own params fields row⟩
  | .enum _ _ names rows _, value =>
      ⟨own _ _ _, variantPlain.deepHolds_of_trivial own params names rows value⟩
  | .vector element, vector => fun value _ => NTy.deepHolds_of_trivial own params element value
  | .slice element, vector => fun value _ => NTy.deepHolds_of_trivial own params element value
  | .array element _, array => fun value _ => NTy.deepHolds_of_trivial own params element value
  | .table key value, entries => fun entry _ =>
      ⟨NTy.deepHolds_of_trivial own params key entry.1, NTy.deepHolds_of_trivial own params value entry.2⟩
  | .ghost slot, value => NTy.deepHolds_of_trivial own params slot value
  | .ref referent, value => NTy.deepHolds_of_trivial own params referent value.2
  | .sliceRef element, value =>
      ⟨fun current _ => NTy.deepHolds_of_trivial own params element current,
        fun final _ => NTy.deepHolds_of_trivial own params element final⟩
  | .observed referent, value => NTy.deepHolds_of_trivial own params referent value
  | .unit, _ => trivial
  | .bool, _ => trivial
  | .int _ _, _ => trivial
  | .num, _ => trivial
  | .address, _ => trivial
  | .signer, _ => trivial
  | .string, _ => trivial
  | .bytes, _ => trivial
  | .char, _ => trivial
  | .param index, value => params index value
  | .function _ _ _, _ => trivial
  | .nominal _ _ _, _ => trivial

theorem HListPlain.deepHolds_of_trivial
    (own : ∀ source arguments value, Carriers.own source arguments value)
    (params : ∀ index value, (Carriers.lent index).holds value) :
    (row : NRow) → (values : HListPlain row) → HListPlain.deepHolds row values
  | .nil, _ => trivial
  | .cons τ rest, values =>
      ⟨NTy.deepHolds_of_trivial own params τ values.1, HListPlain.deepHolds_of_trivial own params rest values.2⟩

theorem variantPlain.deepHolds_of_trivial
    (own : ∀ source arguments value, Carriers.own source arguments value)
    (params : ∀ index value, (Carriers.lent index).holds value) :
    (names : List String) → (rows : NRows) → (value : variantPlain names rows) →
    variantPlain.deepHolds names rows value
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl row => HListPlain.deepHolds_of_trivial own params fields row
  | _ :: names, .cons _ rest, .inr later => variantPlain.deepHolds_of_trivial own params names rest later
end

mutual
/-- A plain value as a certified one, where every declaration's and every
type parameter's data invariant is trivially true, with a type parameter's
values certified by `certifyParam`: at a family whose parameters carry no
certificates of their own that map is the identity
(`doc/designs/data-invariants.md`). -/
def NTy.certifyTrivial (own : ∀ source arguments value, Carriers.own source arguments value)
    (params : ∀ index value, (Carriers.lent index).holds value)
    (certifyParam : ∀ index, (Carriers.lent index).carrier → Carriers.carrier index) :
    (τ : NTy) → τ.plain → τ.carrier
  | .unit, value => value
  | .bool, value => value
  | .int _ _, value => value
  | .num, value => value
  | .address, value => value
  | .signer, value => value
  | .string, value => value
  | .bytes, value => value
  | .char, value => value
  | .tuple elements, values => HListPlain.certifyTrivial own params certifyParam elements values
  | .struct source arguments fields, row =>
      ⟨row, NTy.deepHolds_of_trivial own params (.struct source arguments fields) row⟩
  | .enum source arguments names rows distinct, value =>
      ⟨value, NTy.deepHolds_of_trivial own params (.enum source arguments names rows distinct) value⟩
  | .vector element, vector =>
      ⟨vector.values.map (NTy.certifyTrivial own params certifyParam element),
        by simpa using vector.bounded⟩
  | .slice element, vector =>
      ⟨vector.values.map (NTy.certifyTrivial own params certifyParam element),
        by simpa using vector.bounded⟩
  | .array element _, array =>
      ⟨⟨array.vector.values.map (NTy.certifyTrivial own params certifyParam element),
          by simpa using array.vector.bounded⟩,
        by simpa using array.size_eq⟩
  | .table key value, entries =>
      ⟨entries.entries.map fun entry =>
          (NTy.certifyTrivial own params certifyParam key entry.1,
            NTy.certifyTrivial own params certifyParam value entry.2),
        by simpa using entries.bounded⟩
  | .ghost slot, value => NTy.certifyTrivial own params certifyParam slot value
  | .ref referent, reference =>
      (reference.1, NTy.certifyTrivial own params certifyParam referent reference.2)
  | .sliceRef element, value => value.map (NTy.certifyTrivial own params certifyParam element)
  | .observed referent, value => NTy.certifyTrivial own params certifyParam referent value
  | .param index, value => certifyParam index value
  | .function _ _ _, value => value
  | .nominal _ _ _, value => value

def HListPlain.certifyTrivial (own : ∀ source arguments value, Carriers.own source arguments value)
    (params : ∀ index value, (Carriers.lent index).holds value)
    (certifyParam : ∀ index, (Carriers.lent index).carrier → Carriers.carrier index) :
    (row : NRow) → HListPlain row → HList row
  | .nil, _ => ()
  | .cons τ rest, values =>
      (NTy.certifyTrivial own params certifyParam τ values.1,
        HListPlain.certifyTrivial own params certifyParam rest values.2)
end

/-! ### The trivial certification, constructor by constructor

No clause reads a certificate, so a certified value built this way is the
plain value again wherever a reader projects it.  Registered as the
solver's normal form, so that a read of a field of a trivially certified
row is the field of the row (`doc/designs/data-invariants.md`). -/

section CertifyTrivial
variable (own : ∀ source arguments value, Carriers.own source arguments value)
  (params : ∀ index value, (Carriers.lent index).holds value)
  (certifyParam : ∀ index, (Carriers.lent index).carrier → Carriers.carrier index)

@[vir_denote_norm] theorem NTy.certifyTrivial_unit (value : (NTy.unit).plain) :
    NTy.certifyTrivial own params certifyParam .unit value = value := rfl
@[vir_denote_norm] theorem NTy.certifyTrivial_bool (value : (NTy.bool).plain) :
    NTy.certifyTrivial own params certifyParam .bool value = value := rfl
@[vir_denote_norm] theorem NTy.certifyTrivial_int (width : Nat) (signed : Bool)
    (value : (NTy.int width signed).plain) :
    NTy.certifyTrivial own params certifyParam (.int width signed) value = value := rfl
@[vir_denote_norm] theorem NTy.certifyTrivial_num (value : (NTy.num).plain) :
    NTy.certifyTrivial own params certifyParam .num value = value := rfl
@[vir_denote_norm] theorem NTy.certifyTrivial_address (value : (NTy.address).plain) :
    NTy.certifyTrivial own params certifyParam .address value = value := rfl
@[vir_denote_norm] theorem NTy.certifyTrivial_signer (value : (NTy.signer).plain) :
    NTy.certifyTrivial own params certifyParam .signer value = value := rfl
@[vir_denote_norm] theorem NTy.certifyTrivial_string (value : (NTy.string).plain) :
    NTy.certifyTrivial own params certifyParam .string value = value := rfl
@[vir_denote_norm] theorem NTy.certifyTrivial_bytes (value : (NTy.bytes).plain) :
    NTy.certifyTrivial own params certifyParam .bytes value = value := rfl
@[vir_denote_norm] theorem NTy.certifyTrivial_function (parameters : NRow) (shared : Sharing)
    (results : NRow) (value : (NTy.function parameters shared results).plain) :
    NTy.certifyTrivial own params certifyParam (.function parameters shared results) value =
      value := rfl
@[vir_denote_norm] theorem NTy.certifyTrivial_tuple (elements : NRow)
    (values : (NTy.tuple elements).plain) :
    NTy.certifyTrivial own params certifyParam (.tuple elements) values =
      HListPlain.certifyTrivial own params certifyParam elements values := rfl

/-- A trivially certified struct value is its row: the certificate is the
trivial one, which no reader inspects. -/
@[vir_denote_norm] theorem NTy.certifyTrivial_struct_val (source : StructHandle)
    (arguments fields : NRow) (row : HListPlain fields) :
    (NTy.certifyTrivial own params certifyParam (.struct source arguments fields) row).val =
      row := rfl

/-- A trivially certified variant value is its payload. -/
@[vir_denote_norm] theorem NTy.certifyTrivial_enum_val (source : StructHandle)
    (arguments : NRow) (names : List String) (rows : NRows) (distinct : names.Nodup)
    (value : variantPlain names rows) :
    (NTy.certifyTrivial own params certifyParam
        (.enum source arguments names rows distinct) value).val = value := rfl

@[vir_denote_norm] theorem NTy.certifyTrivial_vector_values (element : NTy)
    (vector : (NTy.vector element).plain) :
    (NTy.certifyTrivial own params certifyParam (.vector element) vector).values =
      vector.values.map (NTy.certifyTrivial own params certifyParam element) := rfl
@[vir_denote_norm] theorem NTy.certifyTrivial_table_entries (key value : NTy)
    (entries : (NTy.table key value).plain) :
    (NTy.certifyTrivial own params certifyParam (.table key value) entries).entries =
      entries.entries.map (fun entry =>
        (NTy.certifyTrivial own params certifyParam key entry.1,
          NTy.certifyTrivial own params certifyParam value entry.2)) := rfl
@[vir_denote_norm] theorem NTy.certifyTrivial_ghost (slot : NTy)
    (value : (NTy.ghost slot).plain) :
    NTy.certifyTrivial own params certifyParam (.ghost slot) value =
      NTy.certifyTrivial own params certifyParam slot value := rfl
@[vir_denote_norm] theorem NTy.certifyTrivial_ref (referent : NTy)
    (reference : (NTy.ref referent).plain) :
    NTy.certifyTrivial own params certifyParam (.ref referent) reference =
      (reference.1, NTy.certifyTrivial own params certifyParam referent reference.2) := rfl
@[vir_denote_norm] theorem NTy.certifyTrivial_param (index : Nat)
    (value : (NTy.param index).plain) :
    NTy.certifyTrivial own params certifyParam (.param index) value =
      certifyParam index value := rfl

@[vir_denote_norm] theorem HListPlain.certifyTrivial_nil (values : HListPlain .nil) :
    HListPlain.certifyTrivial own params certifyParam .nil values = () := rfl
@[vir_denote_norm] theorem HListPlain.certifyTrivial_cons (τ : NTy) (rest : NRow)
    (values : HListPlain (.cons τ rest)) :
    HListPlain.certifyTrivial own params certifyParam (.cons τ rest) values =
      (NTy.certifyTrivial own params certifyParam τ values.1,
        HListPlain.certifyTrivial own params certifyParam rest values.2) := rfl

end CertifyTrivial


/-- Inhabitants of the certified carriers of a family: of every type that
has values, a value. A declaration's data invariant may be unsatisfiable,
so nothing constructs these in general; they are the content of the
assumption that a unit's data invariants are satisfiable, a hypothesis of
the public theorem (`doc/designs/data-invariants.md`). Where every
declaration's invariant is trivially true they are the plain inhabitants
(`CarriersNonempty.ofTrivial`). -/
class CarriersNonempty (Θ : Carriers) where
  inhabitant : (τ : NTy) → τ.inhabitable = true → @NTy.carrier Θ τ
  -- MERGE-REVIEW: the parent made inhabitants abstract while a nominal's
  -- inhabitant is the family's, which `NTy.readBack_unit_nominal` reads;
  -- a nominal's values carry no certificate of their own, so there is
  -- nothing here to choose (`Carriers.nominalInhabitant`).
  /-- A nominal's inhabitant is its family's. -/
  inhabitant_nominal : ∀ (source : StructHandle) (arguments : NRow)
    (inhabitable : (NTy.nominal source arguments true).inhabitable = true)
    (held : arguments.inhabitable = true),
    inhabitant (.nominal source arguments true) inhabitable =
      @Carriers.nominalInhabitant Θ source arguments held

/-- The inhabitants where no declaration and no type parameter states a
data invariant. -/
def CarriersNonempty.ofTrivial
    (own : ∀ source arguments value, Carriers.own source arguments value)
    (params : ∀ index value, (Carriers.lent index).holds value)
    (certifyParam : ∀ index, (Carriers.lent index).carrier → Carriers.carrier index) :
    CarriersNonempty Θ where
  inhabitant := fun τ inhabitable =>
    NTy.certifyTrivial own params certifyParam τ (NTy.plainInhabitant τ inhabitable)
  inhabitant_nominal := fun _ _ _ _ => rfl

/-- A value of a type that has values. -/
def NTy.inhabitant [CarriersNonempty Θ] (τ : NTy) (inhabitable : τ.inhabitable = true) :
    τ.carrier :=
  CarriersNonempty.inhabitant (Θ := Θ) τ inhabitable

end Inhabitant

section Satisfiable
variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]

/-- That a unit's data invariants are satisfiable: every struct and every
enum its declarations give values has a value at the public family.  A
declaration's invariant may be unsatisfiable, and the denotation needs
inhabitants — a read of an absent resource, a map read of a missing key, a
total variant field — so this is a hypothesis of the public theorem beside
`GlobalsPreserved` and `FailuresIntegral`, from which the unit's
`CarriersNonempty` instance is built (`CarriersNonempty.ofSatisfiable`,
`doc/designs/data-invariants.md`, "Inhabitants, by hypothesis").
Every other type's inhabitant is structural over these, so the nominal
types are all it states. -/
def DataInvariantsSatisfiable (unit : Validation.ValidatedUnit)
    [FunctionCarriers unit] : Prop :=
  (∀ (source : StructHandle) (arguments fields : NRow),
      (NTy.struct source arguments fields).inhabitable = true →
      Nonempty (@NTy.carrier (Carriers.runtime unit) (.struct source arguments fields))) ∧
    ∀ (source : StructHandle) (arguments : NRow) (names : List String) (rows : NRows)
      (distinct : names.Nodup),
      (NTy.enum source arguments names rows distinct).inhabitable = true →
      Nonempty (@NTy.carrier (Carriers.runtime unit)
        (.enum source arguments names rows distinct))

mutual
/-- A value of a type that has values, at the public family, from the
satisfiability of the unit's data invariants: a struct's or a variant's by
the hypothesis, every other type's structurally. -/
noncomputable def NTy.inhabitantSatisfiable (satisfiable : DataInvariantsSatisfiable unit) :
    (τ : NTy) → τ.inhabitable = true → @NTy.carrier (Carriers.runtime unit) τ
  | .unit, _ => ()
  | .bool, _ => false
  | .int width signed, inhabitable =>
      ⟨0, zero_fits width signed (by simpa [NTy.inhabitable] using inhabitable)⟩
  | .num, _ => ⟨0, rfl⟩
  | .address, _ => ""
  | .signer, _ => ""
  | .string, _ => ""
  | .bytes, _ => #[]
  | .char, _ => default
  | .tuple elements, inhabitable =>
      HList.inhabitantSatisfiable satisfiable elements
        (by simpa [NTy.inhabitable] using inhabitable)
  | .struct source arguments fields, inhabitable =>
      Classical.choice (satisfiable.1 source arguments fields inhabitable)
  | .enum source arguments names rows distinct, inhabitable =>
      Classical.choice (satisfiable.2 source arguments names rows distinct inhabitable)
  | .vector _, _ => default
  | .slice _, _ => default
  | .array element length, inhabitable =>
      have held : length < 2 ^ 64 ∧ element.inhabitable = true := by
        simpa [NTy.inhabitable] using inhabitable
      ⟨⟨Array.replicate length (NTy.inhabitantSatisfiable satisfiable element held.2),
          by simpa using held.1⟩,
        by simp⟩
  | .table _ _, _ => default
  | .ghost slot, inhabitable =>
      NTy.inhabitantSatisfiable satisfiable slot (by simpa [NTy.inhabitable] using inhabitable)
  | .ref referent, inhabitable =>
      (@NTy.plainInhabitant (Carriers.runtime unit) referent
          (by simpa [NTy.inhabitable] using inhabitable),
        NTy.inhabitantSatisfiable satisfiable referent
          (by simpa [NTy.inhabitable] using inhabitable))
  | .sliceRef _, _ => default
  | .observed referent, inhabitable => NTy.inhabitantSatisfiable satisfiable referent inhabitable
  | .param _, _ => default
  | .function _ _ _, inhabitable => absurd inhabitable (by simp [NTy.inhabitable])
  | .nominal source arguments true, inhabitable =>
      @Carriers.nominalInhabitant (Carriers.runtime unit) source arguments
        (by simpa [NTy.inhabitable] using inhabitable)
  | .nominal _ _ false, inhabitable => absurd inhabitable (by simp [NTy.inhabitable])
termination_by structural τ _ => τ

/-- A row of values at the public family, from the same hypothesis. -/
noncomputable def HList.inhabitantSatisfiable (satisfiable : DataInvariantsSatisfiable unit) :
    (row : NRow) → row.inhabitable = true → @HList (Carriers.runtime unit) row
  | .nil, _ => ()
  | .cons τ rest, inhabitable =>
      (NTy.inhabitantSatisfiable satisfiable τ (by simp_all [NRow.inhabitable]),
        HList.inhabitantSatisfiable satisfiable rest (by simp_all [NRow.inhabitable]))
termination_by structural row _ => row
end

/-- The inhabitants of a unit's certified carriers, from the satisfiability
of its data invariants: the content of the ledger hypothesis
(`DataInvariantsSatisfiable`). -/
noncomputable def CarriersNonempty.ofSatisfiable (satisfiable : DataInvariantsSatisfiable unit) :
    @CarriersNonempty (Carriers.runtime unit) where
  inhabitant := NTy.inhabitantSatisfiable satisfiable
  inhabitant_nominal := fun _ _ _ _ => rfl

/-- A unit whose declarations state no data invariant satisfies the
hypothesis: every plain value is certified there
(`CarriersNonempty.ofTrivial`), so a generated artifact discharges it rather
than carrying it. -/
theorem DataInvariantsSatisfiable.of_trivial
    (own : ∀ source arguments value, FunctionCarriers.own unit source arguments value) :
    DataInvariantsSatisfiable unit :=
  have inhabitants : @CarriersNonempty (Carriers.runtime unit) :=
    @CarriersNonempty.ofTrivial (Carriers.runtime unit) own (fun _ _ => trivial)
      (fun _ value => value)
  ⟨fun source arguments fields inhabitable =>
      ⟨inhabitants.inhabitant (.struct source arguments fields) inhabitable⟩,
    fun source arguments names rows distinct inhabitable =>
      ⟨inhabitants.inhabitant (.enum source arguments names rows distinct) inhabitable⟩⟩

end Satisfiable

section TypedReads
variable [Θ : Carriers] [CarriersNonempty Θ]

/-- The default of a native type's typed values: the type's inhabitant. -/
@[reducible] def nativeInhabited (τ : NTy) (fallback : τ.carrier) : Inhabited τ.carrier :=
  ⟨fallback⟩

/-- A native type's inhabitant, as a view's instance holds it: irreducible,
so that the instance keeps one spelling under normalization. -/
@[irreducible] def NTy.inhabitantOf (τ : NTy) (inhabitable : τ.inhabitable = true) : τ.carrier :=
  NTy.inhabitant τ inhabitable

/-! The typed values of a type, as a specification holds them in a map:
integers by their values, a type parameter's as its family reads them,
every other type's as they are. -/

/-- The type of a type's typed values. -/
@[reducible] def NTy.readCarrier : NTy → Type
  | .int _ _ | .num => Int
  | .param index => (Carriers.reads index).typed
  | .observed referent => referent.readCarrier
  | τ => τ.carrier

/-- The typed value of a native value. -/
def NTy.read : (τ : NTy) → τ.carrier → τ.readCarrier
  | .int _ _, value => value.val
  | .num, value => value.val
  | .param index, value => (Carriers.reads index).read value
  | .unit, value => value
  | .bool, value => value
  | .address, value => value
  | .signer, value => value
  | .string, value => value
  | .bytes, value => value
  | .char, value => value
  | .tuple elements, value => value
  | .struct source arguments fields, value => value
  | .enum source arguments names rows distinct, value => value
  | .slice element, value => value
  | .vector element, value => value
  | .array _ _, value => value
  | .table keyType valueType, value => value
  | .ghost slot, value => value
  | .observed referent, value => NTy.read referent value
  | .sliceRef referent, value => value
  | .ref referent, value => value
  | .function parameters shared results, value => value
  | .nominal source arguments productive, value => value

@[reducible] def NTy.readDecEq : (τ : NTy) → DecidableEq τ.readCarrier
  | .int _ _ => inferInstanceAs (DecidableEq Int)
  | .num => inferInstanceAs (DecidableEq Int)
  | .param index => (Carriers.reads index).decEq
  | .unit => NTy.decEq (NTy.unit)
  | .bool => NTy.decEq (NTy.bool)
  | .address => NTy.decEq (NTy.address)
  | .signer => NTy.decEq (NTy.signer)
  | .string => NTy.decEq (NTy.string)
  | .bytes => NTy.decEq (NTy.bytes)
  | .char => NTy.decEq (NTy.char)
  | .tuple elements => NTy.decEq (NTy.tuple elements)
  | .struct source arguments fields => NTy.decEq (NTy.struct source arguments fields)
  | .enum source arguments names rows distinct => NTy.decEq (NTy.enum source arguments names rows distinct)
  | .slice element => NTy.decEq (NTy.slice element)
  | .vector element => NTy.decEq (NTy.vector element)
  | .array element length => NTy.decEq (NTy.array element length)
  | .table keyType valueType => NTy.decEq (NTy.table keyType valueType)
  | .ghost slot => NTy.decEq (NTy.ghost slot)
  | .observed referent => referent.readDecEq
  | .sliceRef referent => NTy.decEq (NTy.sliceRef referent)
  | .ref referent => NTy.decEq (NTy.ref referent)
  | .function parameters shared results => NTy.decEq (NTy.function parameters shared results)
  | .nominal source arguments productive => NTy.decEq (NTy.nominal source arguments productive)

@[reducible] def NTy.readInhabited : (τ : NTy) → τ.inhabitable = true → Inhabited τ.readCarrier
  | .int _ _, _ => inferInstanceAs (Inhabited Int)
  | .num, _ => inferInstanceAs (Inhabited Int)
  | .param index, _ => (Carriers.reads index).inhabited
  | .unit, inhabitable => nativeInhabited (NTy.unit) (NTy.inhabitantOf (NTy.unit) inhabitable)
  | .bool, inhabitable => nativeInhabited (NTy.bool) (NTy.inhabitantOf (NTy.bool) inhabitable)
  | .address, inhabitable => nativeInhabited (NTy.address) (NTy.inhabitantOf (NTy.address) inhabitable)
  | .signer, inhabitable => nativeInhabited (NTy.signer) (NTy.inhabitantOf (NTy.signer) inhabitable)
  | .string, inhabitable => nativeInhabited (NTy.string) (NTy.inhabitantOf (NTy.string) inhabitable)
  | .bytes, inhabitable => nativeInhabited (NTy.bytes) (NTy.inhabitantOf (NTy.bytes) inhabitable)
  | .char, inhabitable => nativeInhabited (NTy.char) (NTy.inhabitantOf (NTy.char) inhabitable)
  | .tuple elements, inhabitable => nativeInhabited (NTy.tuple elements) (NTy.inhabitantOf (NTy.tuple elements) inhabitable)
  | .struct source arguments fields, inhabitable => nativeInhabited (NTy.struct source arguments fields) (NTy.inhabitantOf (NTy.struct source arguments fields) inhabitable)
  | .enum source arguments names rows distinct, inhabitable => nativeInhabited (NTy.enum source arguments names rows distinct) (NTy.inhabitantOf (NTy.enum source arguments names rows distinct) inhabitable)
  | .slice element, inhabitable => nativeInhabited (NTy.slice element) (NTy.inhabitantOf (NTy.slice element) inhabitable)
  | .vector element, inhabitable => nativeInhabited (NTy.vector element) (NTy.inhabitantOf (NTy.vector element) inhabitable)
  | .array element length, inhabitable => nativeInhabited (NTy.array element length) (NTy.inhabitantOf (NTy.array element length) inhabitable)
  | .table keyType valueType, inhabitable => nativeInhabited (NTy.table keyType valueType) (NTy.inhabitantOf (NTy.table keyType valueType) inhabitable)
  | .ghost slot, inhabitable => nativeInhabited (NTy.ghost slot) (NTy.inhabitantOf (NTy.ghost slot) inhabitable)
  | .observed referent, inhabitable => referent.readInhabited inhabitable
  | .sliceRef referent, inhabitable => nativeInhabited (NTy.sliceRef referent) (NTy.inhabitantOf (NTy.sliceRef referent) inhabitable)
  | .ref referent, inhabitable => nativeInhabited (NTy.ref referent) (NTy.inhabitantOf (NTy.ref referent) inhabitable)
  | .function parameters shared results, inhabitable => nativeInhabited (NTy.function parameters shared results) (NTy.inhabitantOf (NTy.function parameters shared results) inhabitable)
  | .nominal source arguments productive, inhabitable => nativeInhabited (NTy.nominal source arguments productive) (NTy.inhabitantOf (NTy.nominal source arguments productive) inhabitable)

/-- The runtime value of a typed value. -/
noncomputable def NTy.readWrite : (τ : NTy) → τ.readCarrier → RuntimeValue
  | .int _ _ => RuntimeValue.integer
  | .num => RuntimeValue.integer
  | .param index => (Carriers.reads index).write
  | .unit => NTy.encode (NTy.unit)
  | .bool => NTy.encode (NTy.bool)
  | .address => NTy.encode (NTy.address)
  | .signer => NTy.encode (NTy.signer)
  | .string => NTy.encode (NTy.string)
  | .bytes => NTy.encode (NTy.bytes)
  | .char => NTy.encode (NTy.char)
  | .tuple elements => NTy.encode (NTy.tuple elements)
  | .struct source arguments fields => NTy.encode (NTy.struct source arguments fields)
  | .enum source arguments names rows distinct => NTy.encode (NTy.enum source arguments names rows distinct)
  | .slice element => NTy.encode (NTy.slice element)
  | .vector element => NTy.encode (NTy.vector element)
  | .array element length => NTy.encode (NTy.array element length)
  | .table keyType valueType => NTy.encode (NTy.table keyType valueType)
  | .ghost slot => NTy.encode (NTy.ghost slot)
  | .observed referent => referent.readWrite
  | .sliceRef referent => NTy.encode (NTy.sliceRef referent)
  | .ref referent => NTy.encode (NTy.ref referent)
  | .function parameters shared results => NTy.encode (NTy.function parameters shared results)
  | .nominal source arguments productive => NTy.encode (NTy.nominal source arguments productive)

/-- The typed value a runtime value reads back to. -/
noncomputable def NTy.readBack : (τ : NTy) → τ.inhabitable = true → RuntimeValue → τ.readCarrier
  | .int _ _, _ => RuntimeValue.asInt
  | .num, _ => RuntimeValue.asInt
  | .param index, _ => (Carriers.reads index).back
  | .unit, inhabitable => fun raw =>
      ((NTy.unit).codec.decode? raw).getD (nativeInhabited (NTy.unit) (NTy.inhabitantOf (NTy.unit) inhabitable)).default
  | .bool, inhabitable => fun raw =>
      ((NTy.bool).codec.decode? raw).getD (nativeInhabited (NTy.bool) (NTy.inhabitantOf (NTy.bool) inhabitable)).default
  | .address, inhabitable => fun raw =>
      ((NTy.address).codec.decode? raw).getD (nativeInhabited (NTy.address) (NTy.inhabitantOf (NTy.address) inhabitable)).default
  | .signer, inhabitable => fun raw =>
      ((NTy.signer).codec.decode? raw).getD (nativeInhabited (NTy.signer) (NTy.inhabitantOf (NTy.signer) inhabitable)).default
  | .string, inhabitable => fun raw =>
      ((NTy.string).codec.decode? raw).getD (nativeInhabited (NTy.string) (NTy.inhabitantOf (NTy.string) inhabitable)).default
  | .bytes, inhabitable => fun raw =>
      ((NTy.bytes).codec.decode? raw).getD (nativeInhabited (NTy.bytes) (NTy.inhabitantOf (NTy.bytes) inhabitable)).default
  | .char, inhabitable => fun raw =>
      ((NTy.char).codec.decode? raw).getD (nativeInhabited (NTy.char) (NTy.inhabitantOf (NTy.char) inhabitable)).default
  | .tuple elements, inhabitable => fun raw =>
      ((NTy.tuple elements).codec.decode? raw).getD (nativeInhabited (NTy.tuple elements) (NTy.inhabitantOf (NTy.tuple elements) inhabitable)).default
  | .struct source arguments fields, inhabitable => fun raw =>
      ((NTy.struct source arguments fields).codec.decode? raw).getD (nativeInhabited (NTy.struct source arguments fields) (NTy.inhabitantOf (NTy.struct source arguments fields) inhabitable)).default
  | .enum source arguments names rows distinct, inhabitable => fun raw =>
      ((NTy.enum source arguments names rows distinct).codec.decode? raw).getD (nativeInhabited (NTy.enum source arguments names rows distinct) (NTy.inhabitantOf (NTy.enum source arguments names rows distinct) inhabitable)).default
  | .array element length, inhabitable => fun raw =>
      ((NTy.array element length).codec.decode? raw).getD (nativeInhabited (NTy.array element length) (NTy.inhabitantOf (NTy.array element length) inhabitable)).default
  | .slice element, inhabitable => fun raw =>
      ((NTy.slice element).codec.decode? raw).getD (nativeInhabited (NTy.slice element) (NTy.inhabitantOf (NTy.slice element) inhabitable)).default
  | .vector element, inhabitable => fun raw =>
      ((NTy.vector element).codec.decode? raw).getD (nativeInhabited (NTy.vector element) (NTy.inhabitantOf (NTy.vector element) inhabitable)).default
  | .table keyType valueType, inhabitable => fun raw =>
      ((NTy.table keyType valueType).codec.decode? raw).getD (nativeInhabited (NTy.table keyType valueType) (NTy.inhabitantOf (NTy.table keyType valueType) inhabitable)).default
  | .ghost slot, inhabitable => fun raw =>
      ((NTy.ghost slot).codec.decode? raw).getD (nativeInhabited (NTy.ghost slot) (NTy.inhabitantOf (NTy.ghost slot) inhabitable)).default
  | .observed referent, inhabitable => referent.readBack inhabitable
  | .sliceRef referent, inhabitable => fun raw =>
      ((NTy.sliceRef referent).codec.decode? raw).getD (nativeInhabited (NTy.sliceRef referent) (NTy.inhabitantOf (NTy.sliceRef referent) inhabitable)).default
  | .ref referent, inhabitable => fun raw =>
      ((NTy.ref referent).codec.decode? raw).getD (nativeInhabited (NTy.ref referent) (NTy.inhabitantOf (NTy.ref referent) inhabitable)).default
  | .function parameters shared results, inhabitable => fun raw =>
      ((NTy.function parameters shared results).codec.decode? raw).getD (nativeInhabited (NTy.function parameters shared results) (NTy.inhabitantOf (NTy.function parameters shared results) inhabitable)).default
  | .nominal source arguments productive, inhabitable => fun raw =>
      ((NTy.nominal source arguments productive).codec.decode? raw).getD (nativeInhabited (NTy.nominal source arguments productive) (NTy.inhabitantOf (NTy.nominal source arguments productive) inhabitable)).default

theorem NTy.readBack_readWrite : (τ : NTy) → (inhabitable : τ.inhabitable = true) →
    (value : τ.readCarrier) → NTy.readBack τ inhabitable (NTy.readWrite τ value) = value
  | .int _ _, _, _ => rfl
  | .num, _, _ => rfl
  | .param index, _, value => (Carriers.reads index).back_write value
  | .unit, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .bool, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .address, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .signer, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .string, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .bytes, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .char, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .tuple elements, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .struct source arguments fields, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .enum source arguments names rows distinct, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .slice element, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .vector element, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .array element length, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .table keyType valueType, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .ghost slot, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .observed referent, inhabitable, value => NTy.readBack_readWrite referent inhabitable value
  | .sliceRef referent, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .ref referent, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .function parameters shared results, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]
  | .nominal source arguments productive, _, value => by simp only [NTy.readBack, NTy.readWrite, NTy.decode?_encode, Option.getD_some]

/-- `.unit` reads back as a nominal's inhabitant: it is what `.unit` decodes to, if anything. -/
theorem NTy.readBack_unit_nominal {source : StructHandle} {arguments : NRow}
    (inhabitable : (NTy.nominal source arguments true).inhabitable = true) :
    ((Carriers.nominals source arguments true).ofValue? .unit).getD
        (NTy.inhabitantOf (.nominal source arguments true) inhabitable) =
      NTy.inhabitantOf (.nominal source arguments true) inhabitable := by
  have held : arguments.inhabitable = true := by simpa [NTy.inhabitable] using inhabitable
  cases decoded : (Carriers.nominals source arguments true).ofValue? .unit with
  | none => rfl
  | some value =>
      have same := Carriers.nominalInhabitant_unit source arguments held value
        ((Carriers.nominals source arguments true).value_of_ofValue? _ _ decoded)
      simp only [Option.getD_some, same]
      rw [NTy.inhabitantOf]
      exact (CarriersNonempty.inhabitant_nominal source arguments inhabitable held).symm

theorem NTy.readBack_unit : (τ : NTy) → (inhabitable : τ.inhabitable = true) →
    NTy.readBack τ inhabitable .unit = (NTy.readInhabited τ inhabitable).default
  | .int _ _, _ => rfl
  | .num, _ => rfl
  | .param index, _ => (Carriers.reads index).back_unit
  | .unit, inhabitable => rfl
  | .bool, inhabitable => rfl
  | .address, inhabitable => rfl
  | .signer, inhabitable => rfl
  | .string, inhabitable => rfl
  | .bytes, inhabitable => rfl
  | .char, inhabitable => rfl
  | .tuple elements, inhabitable => rfl
  | .struct source arguments fields, inhabitable => rfl
  | .enum source arguments names rows distinct, inhabitable => by
      simp only [NTy.readBack, NTy.readInhabited, NTy.codec, Codec.certified, NTy.plainCodec, NTy.plainCodecOf, variantCodecPlain, variantCodecPlainOf, variantDecodePlain?_unit, Option.bind_none, Option.getD_none]
  | .slice element, inhabitable => rfl

  | .vector element, inhabitable => rfl
  | .array element length, inhabitable => rfl
  | .table keyType valueType, inhabitable => rfl
  | .ghost slot, inhabitable => rfl
  | .observed referent, inhabitable => NTy.readBack_unit referent inhabitable
  | .sliceRef referent, inhabitable => rfl
  | .ref referent, inhabitable => rfl
  | .function parameters shared results, inhabitable => rfl
  | .nominal _ _ true, inhabitable => NTy.readBack_unit_nominal inhabitable
  | .nominal _ _ false, inhabitable => absurd inhabitable (by simp [NTy.inhabitable])

theorem NTy.readWrite_read : (τ : NTy) → (value : τ.carrier) →
    NTy.readWrite τ (NTy.read τ value) = NTy.encode τ value
  | .int _ _, _ => rfl
  | .num, _ => rfl
  | .param index, value => (Carriers.reads index).write_read value
  | .unit, _ => rfl
  | .bool, _ => rfl
  | .address, _ => rfl
  | .signer, _ => rfl
  | .string, _ => rfl
  | .bytes, _ => rfl
  | .char, _ => rfl
  | .tuple elements, _ => rfl
  | .struct source arguments fields, _ => rfl
  | .enum source arguments names rows distinct, _ => rfl
  | .slice element, _ => rfl
  | .vector element, _ => rfl
  | .array element length, _ => rfl
  | .table keyType valueType, _ => rfl
  | .ghost slot, _ => rfl
  | .observed referent, value => NTy.readWrite_read referent value
  | .sliceRef referent, _ => rfl
  | .ref referent, _ => rfl
  | .function parameters shared results, _ => rfl
  | .nominal source arguments productive, _ => rfl

/-- The native value a typed value stands for: an integer cut to its type, a
type parameter's as its family reads it back, any other as it is. -/
def NTy.unread : (τ : NTy) → τ.inhabitable = true → τ.readCarrier → τ.carrier
  | .int width signed, inhabitable, value =>
      SpecInt.ofInt (NTy.inhabitant (.int width signed) inhabitable) value
  | .num, _, value => ⟨value, rfl⟩
  | .param index, _, value => (Carriers.reads index).unread value
  | .unit, _, value => value
  | .bool, _, value => value
  | .address, _, value => value
  | .signer, _, value => value
  | .string, _, value => value
  | .bytes, _, value => value
  | .char, _, value => value
  | .tuple _, _, value => value
  | .struct _ _ _, _, value => value
  | .enum _ _ _ _ _, _, value => value
  | .slice _, _, value => value
  | .vector _, _, value => value
  | .array _ _, _, value => value
  | .table _ _, _, value => value
  | .ghost _, _, value => value
  | .observed referent, inhabitable, value => referent.unread inhabitable value
  | .sliceRef _, _, value => value
  | .ref _, _, value => value
  | .function _ _ _, _, value | .nominal _ _ _, _, value => value

/-! A typed value read back as a native value at a constructed type: an
integer cut to its width (`SpecInt.ofInt`), any other the value itself; a
type parameter's stays its family's. -/

@[simp, vir_denote_norm] theorem NTy.unread_int (width : Nat) (signed : Bool)
    (inhabitable : (NTy.int width signed).inhabitable = true) :
    NTy.unread (.int width signed) inhabitable =
      SpecInt.ofInt (NTy.inhabitant (.int width signed) inhabitable) := rfl
@[simp, vir_denote_norm] theorem NTy.unread_num (inhabitable : NTy.num.inhabitable = true) :
    NTy.unread .num inhabitable = fun value => ⟨value, rfl⟩ := rfl
@[simp, vir_denote_norm] theorem NTy.unread_unit (inhabitable) :
    NTy.unread .unit inhabitable = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.unread_bool (inhabitable) :
    NTy.unread .bool inhabitable = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.unread_address (inhabitable) :
    NTy.unread .address inhabitable = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.unread_signer (inhabitable) :
    NTy.unread .signer inhabitable = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.unread_string (inhabitable) :
    NTy.unread .string inhabitable = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.unread_bytes (inhabitable) :
    NTy.unread .bytes inhabitable = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.unread_tuple (elements : NRow) (inhabitable) :
    NTy.unread (.tuple elements) inhabitable = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.unread_struct (source : StructHandle) (arguments fields : NRow)
    (inhabitable) : NTy.unread (.struct source arguments fields) inhabitable = fun value => value :=
  rfl
@[simp, vir_denote_norm] theorem NTy.unread_enum (source : StructHandle) (arguments : NRow)
    (names : List String) (rows : NRows) (distinct : names.Nodup) (inhabitable) :
    NTy.unread (.enum source arguments names rows distinct) inhabitable = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.unread_vector (element : NTy) (inhabitable) :
    NTy.unread (.vector element) inhabitable = fun value => value := rfl
@[simp, vir_denote_norm] theorem NTy.unread_table (key value : NTy) (inhabitable) :
    NTy.unread (.table key value) inhabitable = fun entries => entries := rfl

@[simp, vir_denote_norm] theorem NTy.unread_read : (τ : NTy) → (inhabitable : τ.inhabitable = true) → (value : τ.carrier) →
    NTy.unread τ inhabitable (NTy.read τ value) = value
  | .int _ _, _, value => SpecInt.ofInt_val _ value
  | .num, _, _ => rfl
  | .param index, _, value => (Carriers.reads index).unread_read value
  | .unit, _, _ => rfl
  | .bool, _, _ => rfl
  | .address, _, _ => rfl
  | .signer, _, _ => rfl
  | .string, _, _ => rfl
  | .bytes, _, _ => rfl
  | .char, _, _ => rfl
  | .tuple _, _, _ => rfl
  | .struct _ _ _, _, _ => rfl
  | .enum _ _ _ _ _, _, _ => rfl
  | .slice _, _, _ => rfl
  | .vector _, _, _ => rfl
  | .array _ _, _, _ => rfl
  | .table _ _, _, _ => rfl
  | .ghost _, _, _ => rfl
  | .observed referent, inhabitable, value => NTy.unread_read referent inhabitable value
  | .sliceRef _, _, _ => rfl
  | .ref _, _, _ => rfl
  | .function _ _ _, _, _ | .nominal _ _ _, _, _ => rfl

/-- The order of a type's typed values, as an ordered map's typed keys
compare: integers by their values, a type parameter's as its family orders
them, any other type's by the structural order (`NTy.keyOrder`). -/
@[reducible] def NTy.readOrder (rank : ValueRanks) : (τ : NTy) → SpecMap.KeyOrder τ.readCarrier
  | .int _ _ => SpecMap.intOrder
  | .num => SpecMap.intOrder
  | .param index => (Carriers.reads index).order rank
  | .unit => NTy.keyOrder rank .unit
  | .bool => NTy.keyOrder rank .bool
  | .address => NTy.keyOrder rank .address
  | .signer => NTy.keyOrder rank .signer
  | .string => NTy.keyOrder rank .string
  | .bytes => NTy.keyOrder rank .bytes
  | .char => NTy.keyOrder rank .char
  | .tuple elements => NTy.keyOrder rank (.tuple elements)
  | .struct source arguments fields => NTy.keyOrder rank (.struct source arguments fields)
  | .enum source arguments names rows distinct =>
      NTy.keyOrder rank (.enum source arguments names rows distinct)
  | .slice element => NTy.keyOrder rank (.slice element)
  | .vector element => NTy.keyOrder rank (.vector element)
  | .array element length => NTy.keyOrder rank (.array element length)
  | .table key value => NTy.keyOrder rank (.table key value)
  | .ghost slot => NTy.keyOrder rank (.ghost slot)
  | .observed referent => referent.readOrder rank
  | .sliceRef referent => NTy.keyOrder rank (.sliceRef referent)
  | .ref referent => NTy.keyOrder rank (.ref referent)
  | .function parameters shared results => NTy.keyOrder rank (.function parameters shared results)
  | .nominal source arguments productive => NTy.keyOrder rank (.nominal source arguments productive)

/-- The typed reads of a type, bundled: what a family instantiated at the
type reads a parameter by. -/
@[reducible] noncomputable def NTy.reads (τ : NTy) (inhabitable : τ.inhabitable = true) :
    TypedReads τ.carrier (NTy.encode τ) where
  typed := τ.readCarrier
  read := τ.read
  decEq := τ.readDecEq
  inhabited := τ.readInhabited inhabitable
  write := τ.readWrite
  back := τ.readBack inhabitable
  back_write := τ.readBack_readWrite inhabitable
  back_unit := τ.readBack_unit inhabitable
  write_read := τ.readWrite_read
  unread := τ.unread inhabitable
  unread_read := τ.unread_read inhabitable
  order := fun rank => NTy.readOrder rank τ

end TypedReads

/-- Decoding the encoding of a certificate-free plain value reads the
same value.  The two layers agree by the actual forgetting and codec laws,
not merely because their Lean types happen to agree. -/
theorem NTy.readBack_plainEncode_certificateFree [Θ : Carriers]
    [CarriersNonempty Θ] (τ : NTy) (held : τ.inhabitable = true)
    (free : τ.certificateFree = true) (value : τ.plain) :
    NTy.readBack τ held (NTy.plainEncode τ value) =
      NTy.read τ (cast (NTy.carrier_eq_plain_of_certificateFree τ free).symm value) := by
  conv => lhs; rw [← NTy.forget_cast_of_certificateFree τ free value]
  rw [NTy.plainEncode_forget, ← NTy.readWrite_read, NTy.readBack_readWrite]

/-- The typed value of a plain value. Certificate-free types read the same
value through the canonical cast; this agrees with decoding its encoding
(`NTy.readBack_plainEncode_certificateFree`). A type which can carry a
certificate keeps the total decoder, including its invalid-value fallback.
Thus instantiating a plain parameter at a certificate-free type produces the
same read as a concrete typed view without reconstructing its values. -/
noncomputable def NTy.readPlain [Θ : Carriers]
    [CarriersNonempty Θ] (τ : NTy) (held : τ.inhabitable = true) (value : τ.plain) :
    τ.readCarrier :=
  match τ with
  | .param index => (Carriers.plainRead index).val value
  | τ => if free : τ.certificateFree = true then
      NTy.read τ (cast (NTy.carrier_eq_plain_of_certificateFree τ free).symm value)
    else NTy.readBack τ held (NTy.plainEncode τ value)

/-- The canonical plain reading preserves the original total codec reading. -/
theorem NTy.readPlain_eq_readBack [Θ : Carriers]
    [CarriersNonempty Θ] (τ : NTy) (held : τ.inhabitable = true) (value : τ.plain) :
    NTy.readPlain τ held value = NTy.readBack τ held (NTy.plainEncode τ value) := by
  cases τ with
  | param index => exact (Carriers.plainRead index).property value
  | _ =>
    simp only [NTy.readPlain]
    all_goals first
      | rfl
      | (split
         · exact (NTy.readBack_plainEncode_certificateFree _ held ‹_› value).symm
         · rfl)

/-- The family a generic call's type arguments induce: parameter `i` is
carried as the `i`-th argument at the caller's family. -/
@[reducible] noncomputable def Carriers.instantiate (θ : TypeArgs) (outer : Carriers)
    [@CarriersNonempty outer] : Carriers where
  carrier := fun index => @NTy.carrier outer ((NTy.param index).subst θ.1)
  codec := fun index => @NTy.codec outer ((NTy.param index).subst θ.1)
  decEq := fun index => @NTy.decEq outer ((NTy.param index).subst θ.1)
  inhabited := fun index =>
    ⟨@NTy.inhabitant outer inferInstance ((NTy.param index).subst θ.1)
      ((NTy.inhabitable_unmark _).trans
        (NRow.getD_inhabitable θ.1 θ.inhabitable index (.param index) rfl))⟩
  tight := fun index => @NTy.codec_tight outer ((NTy.param index).subst θ.1)
  plain := fun index => @NTy.encode_plain outer ((NTy.param index).subst θ.1)
  functionCarrier := fun parameters shared results =>
    @Carriers.functionCarrier outer (NRow.subst θ.1 parameters) shared (NRow.subst θ.1 results)
  functions := fun parameters shared results =>
    @Carriers.functions outer (NRow.subst θ.1 parameters) shared (NRow.subst θ.1 results)
  nominalCarrier := fun source arguments productive =>
    @Carriers.nominalCarrier outer source (NRow.subst θ.1 arguments) productive
  nominals := fun source arguments productive =>
    @Carriers.nominals outer source (NRow.subst θ.1 arguments) productive
  nominalInhabitant := fun source arguments held =>
    @Carriers.nominalInhabitant outer source (NRow.subst θ.1 arguments)
      (NRow.inhabitable_subst θ.1 θ.inhabitable arguments held)
  nominalInhabitant_unit := fun source arguments _ =>
    @Carriers.nominalInhabitant_unit outer source (NRow.subst θ.1 arguments) _
  reads := fun index => @NTy.reads outer inferInstance ((NTy.param index).subst θ.1)
    ((NTy.inhabitable_unmark _).trans
        (NRow.getD_inhabitable θ.1 θ.inhabitable index (.param index) rfl))
  orders := fun index => @NTy.typedOrder outer ((NTy.param index).subst θ.1)
  erasures := fun index => @NTy.typedErasure outer ((NTy.param index).subst θ.1)
  lent := fun index =>
    { carrier := @NTy.plain outer ((NTy.param index).subst θ.1),
      codec := @NTy.plainCodec outer ((NTy.param index).subst θ.1),
      of := @NTy.forget outer ((NTy.param index).subst θ.1),
      encode_of := @NTy.plainEncode_forget outer ((NTy.param index).subst θ.1),
      holds := @NTy.deepHolds outer ((NTy.param index).subst θ.1),
      holds_of := @NTy.deepHolds_forget outer ((NTy.param index).subst θ.1),
      certify := @NTy.ofDeepHolds outer ((NTy.param index).subst θ.1),
      of_certify := @NTy.forget_ofDeepHolds outer ((NTy.param index).subst θ.1),
      certify_of := @NTy.ofDeepHolds_forget outer ((NTy.param index).subst θ.1),
      decEq := @NTy.plainDecEq outer ((NTy.param index).subst θ.1),
      inhabited := ⟨@NTy.plainInhabitant outer ((NTy.param index).subst θ.1)
        ((NTy.inhabitable_unmark _).trans
        (NRow.getD_inhabitable θ.1 θ.inhabitable index (.param index) rfl))⟩,
      tight := @NTy.plainCodec_tight outer ((NTy.param index).subst θ.1),
      plain := @NTy.plainEncode_plain outer ((NTy.param index).subst θ.1),
      orders := @NTy.plainTypedOrder outer ((NTy.param index).subst θ.1),
      erasures := @NTy.plainTypedErasure outer ((NTy.param index).subst θ.1) }
  plainRead := fun index => ⟨@NTy.readPlain outer inferInstance ((NTy.param index).subst θ.1)
    ((NTy.inhabitable_unmark _).trans
      (NRow.getD_inhabitable θ.1 θ.inhabitable index (.param index) rfl)),
    fun value => @NTy.readPlain_eq_readBack outer inferInstance ((NTy.param index).subst θ.1)
      ((NTy.inhabitable_unmark _).trans
        (NRow.getD_inhabitable θ.1 θ.inhabitable index (.param index) rfl)) value⟩
  own := fun source arguments value => outer.own source (NRow.subst θ.1 arguments) value

/-- The data invariants an induced family states of a declaration: the
caller's, at the arguments it substitutes (`Carriers.instantiate`). -/
@[simp, vir_denote_norm] theorem Carriers.own_instantiate (θ : TypeArgs) (outer : Carriers)
    [@CarriersNonempty outer] (source : StructHandle) (arguments : NRow)
    (value : RuntimeValue) :
    @Carriers.own (Carriers.instantiate θ outer) source arguments value =
      outer.own source (NRow.subst θ.1 arguments) value := rfl

/-- A type parameter's order at the family a call's type arguments induce:
the outer family's order at the argument's type. -/
@[vir_denote_norm] theorem Carriers.orders_instantiate (θ : TypeArgs) (outer : Carriers) [@CarriersNonempty outer]
    (index : Nat)
    (rank : ValueRanks) (left right : @Carriers.carrier (Carriers.instantiate θ outer) index) :
    (@Carriers.orders (Carriers.instantiate θ outer) index).order rank left right =
      @NTy.order outer rank ((NTy.param index).subst θ.1) left right := rfl

/-- A type parameter's equality at the family a call's type arguments
induce: the outer family's equality at the argument's type. -/
@[vir_denote_norm] theorem Carriers.erasures_instantiate (θ : TypeArgs) (outer : Carriers) [@CarriersNonempty outer]
    (index : Nat) (left right : @Carriers.carrier (Carriers.instantiate θ outer) index) :
    (@Carriers.erasures (Carriers.instantiate θ outer) index).same left right =
      @NTy.equal outer false ((NTy.param index).subst θ.1) left right := rfl

section Transport
variable [Θ : Carriers] [Ξ : @CarriersNonempty Θ]


/-! ## Lent values across a call

A type parameter's lent values are the argument's lent values at the
caller's family (`Carriers.lent`), so the plain family transports as the
certified one does and a parameter's own value is itself. -/

mutual
/-- A caller's lent value in the callee's view: the same runtime value,
carried at the family the call's type arguments induce. -/
def NTy.plainToSkolem (θ : TypeArgs) : (τ : NTy) → (τ.subst θ.1).plain →
    @NTy.plain (Carriers.instantiate θ Θ) τ
  | .unit, value => value
  | .bool, value => value
  | .int _ _, value => value
  | .num, value => value
  | .address, value => value
  | .signer, value => value
  | .string, value => value
  | .bytes, value => value
  | .char, value => value
  | .tuple elements, values => HListPlain.toSkolem θ elements values
  | .struct _ _ fields, values => HListPlain.toSkolem θ fields values
  | .enum _ _ names rows _, value => variantPlain.toSkolem θ names rows value
  | .vector element, vector =>
      ⟨vector.values.map (NTy.plainToSkolem θ element), by simpa using vector.bounded⟩
  | .slice element, vector =>
      ⟨vector.values.map (NTy.plainToSkolem θ element), by simpa using vector.bounded⟩
  | .array element _, array =>
      ⟨⟨array.vector.values.map (NTy.plainToSkolem θ element), by simpa using array.vector.bounded⟩,
        by simpa using array.size_eq⟩
  | .table key value, entries =>
      ⟨entries.entries.map fun entry =>
          (NTy.plainToSkolem θ key entry.1, NTy.plainToSkolem θ value entry.2),
        by simpa using entries.bounded⟩
  | .ghost slot, value => NTy.plainToSkolem θ slot value
  | .ref referent, value =>
      (NTy.plainToSkolem θ referent value.1, NTy.plainToSkolem θ referent value.2)
  | .sliceRef element, value => value.map (NTy.plainToSkolem θ element)
  | .observed referent, value => NTy.plainToSkolem θ referent value
  | .param _, value => value
  | .function _ _ _, value => value
  | .nominal _ _ _, value => value

/-- A caller's row of lent values in the callee's view. -/
def HListPlain.toSkolem (θ : TypeArgs) : (row : NRow) → HListPlain (NRow.subst θ.1 row) →
    @HListPlain (Carriers.instantiate θ Θ) row
  | .nil, _ => ()
  | .cons τ rest, values =>
      (NTy.plainToSkolem θ τ values.1, HListPlain.toSkolem θ rest values.2)

/-- A caller's lent variant value in the callee's view. -/
def variantPlain.toSkolem (θ : TypeArgs) : (names : List String) → (rows : NRows) →
    variantPlain names (NRows.subst θ.1 rows) →
    @variantPlain (Carriers.instantiate θ Θ) names rows
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl values => .inl (HListPlain.toSkolem θ fields values)
  | _ :: names, .cons _ rest, .inr value => .inr (variantPlain.toSkolem θ names rest value)
end

mutual
/-- A callee's lent value in the caller's view. -/
def NTy.plainOfSkolem (θ : TypeArgs) : (τ : NTy) →
    @NTy.plain (Carriers.instantiate θ Θ) τ → (τ.subst θ.1).plain
  | .unit, value => value
  | .bool, value => value
  | .int _ _, value => value
  | .num, value => value
  | .address, value => value
  | .signer, value => value
  | .string, value => value
  | .bytes, value => value
  | .char, value => value
  | .tuple elements, values => HListPlain.ofSkolem θ elements values
  | .struct _ _ fields, values => HListPlain.ofSkolem θ fields values
  | .enum _ _ names rows _, value => variantPlain.ofSkolem θ names rows value
  | .vector element, vector =>
      ⟨vector.values.map (NTy.plainOfSkolem θ element), by simpa using vector.bounded⟩
  | .slice element, vector =>
      ⟨vector.values.map (NTy.plainOfSkolem θ element), by simpa using vector.bounded⟩
  | .array element _, array =>
      ⟨⟨array.vector.values.map (NTy.plainOfSkolem θ element), by simpa using array.vector.bounded⟩,
        by simpa using array.size_eq⟩
  | .table key value, entries =>
      ⟨entries.entries.map fun entry =>
          (NTy.plainOfSkolem θ key entry.1, NTy.plainOfSkolem θ value entry.2),
        by simpa using entries.bounded⟩
  | .ghost slot, value => NTy.plainOfSkolem θ slot value
  | .ref referent, value =>
      (NTy.plainOfSkolem θ referent value.1, NTy.plainOfSkolem θ referent value.2)
  | .sliceRef element, value => value.map (NTy.plainOfSkolem θ element)
  | .observed referent, value => NTy.plainOfSkolem θ referent value
  | .param _, value => value
  | .function _ _ _, value => value
  | .nominal _ _ _, value => value

/-- A callee's row of lent values in the caller's view. -/
def HListPlain.ofSkolem (θ : TypeArgs) : (row : NRow) →
    @HListPlain (Carriers.instantiate θ Θ) row → HListPlain (NRow.subst θ.1 row)
  | .nil, _ => ()
  | .cons τ rest, values =>
      (NTy.plainOfSkolem θ τ values.1, HListPlain.ofSkolem θ rest values.2)

/-- A callee's lent variant value in the caller's view. -/
def variantPlain.ofSkolem (θ : TypeArgs) : (names : List String) → (rows : NRows) →
    @variantPlain (Carriers.instantiate θ Θ) names rows →
    variantPlain names (NRows.subst θ.1 rows)
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl values => .inl (HListPlain.ofSkolem θ fields values)
  | _ :: names, .cons _ rest, .inr value => .inr (variantPlain.ofSkolem θ names rest value)
end

mutual
/-- A caller's lent value read back in the caller's view is the value. -/
theorem NTy.plainOfSkolem_toSkolem (θ : TypeArgs) : (τ : NTy) → (value : (τ.subst θ.1).plain) →
    NTy.plainOfSkolem θ τ (NTy.plainToSkolem θ τ value) = value
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _
  | .bytes, _ | .char, _ | .param _, _ | .function _ _ _, _ | .nominal _ _ _, _ => rfl
  | .tuple elements, values => HListPlain.ofSkolem_toSkolem θ elements values
  | .struct _ _ fields, values => HListPlain.ofSkolem_toSkolem θ fields values
  | .enum _ _ names rows _, value => variantPlain.ofSkolem_toSkolem θ names rows value
  | .vector element, vector => by
      apply SpecVector.ext
      simp only [NTy.plainToSkolem, NTy.plainOfSkolem, Array.map_map]
      conv => rhs; rw [← Array.map_id vector.values]
      apply Array.map_congr_left
      intro value _
      exact NTy.plainOfSkolem_toSkolem θ element value
  | .slice element, vector => by
      apply SpecVector.ext
      simp only [NTy.plainToSkolem, NTy.plainOfSkolem, Array.map_map]
      conv => rhs; rw [← Array.map_id vector.values]
      apply Array.map_congr_left
      intro value _
      exact NTy.plainOfSkolem_toSkolem θ element value
  | .array element _, array => by
      apply SpecArray.ext
      apply SpecVector.ext
      simp only [NTy.plainToSkolem, NTy.plainOfSkolem, Array.map_map]
      conv => rhs; rw [← Array.map_id array.vector.values]
      apply Array.map_congr_left
      intro value _
      exact NTy.plainOfSkolem_toSkolem θ element value
  | .table key value, entries => by
      apply SpecEntries.ext
      simp only [NTy.plainToSkolem, NTy.plainOfSkolem, List.map_map]
      conv => rhs; rw [← List.map_id entries.entries]
      apply List.map_congr_left
      intro entry _
      simp only [Function.comp_apply, NTy.plainOfSkolem_toSkolem θ key, NTy.plainOfSkolem_toSkolem θ value, id_eq]
  | .ghost slot, value => NTy.plainOfSkolem_toSkolem θ slot value
  | .ref referent, value => by
      simp only [NTy.plainToSkolem, NTy.plainOfSkolem, NTy.plainOfSkolem_toSkolem θ referent]
  | .sliceRef element, value => by
      simp only [NTy.plainToSkolem, NTy.plainOfSkolem, SpecSliceRef.map_map]
      have same : (fun x => NTy.plainOfSkolem θ element (NTy.plainToSkolem θ element x)) = id :=
        funext (NTy.plainOfSkolem_toSkolem θ element)
      rw [show _ ∘ _ = id from same, SpecSliceRef.map_id]
  | .observed referent, value => NTy.plainOfSkolem_toSkolem θ referent value

theorem HListPlain.ofSkolem_toSkolem (θ : TypeArgs) : (row : NRow) →
    (values : HListPlain (NRow.subst θ.1 row)) →
    HListPlain.ofSkolem θ row (HListPlain.toSkolem θ row values) = values
  | .nil, _ => rfl
  | .cons τ rest, values => by
      simp only [HListPlain.toSkolem, HListPlain.ofSkolem, NTy.plainOfSkolem_toSkolem θ τ, HListPlain.ofSkolem_toSkolem θ rest]
      rfl

theorem variantPlain.ofSkolem_toSkolem (θ : TypeArgs) : (names : List String) → (rows : NRows) →
    (value : variantPlain names (NRows.subst θ.1 rows)) →
    variantPlain.ofSkolem θ names rows (variantPlain.toSkolem θ names rows value) = value
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl values => by
      simp only [variantPlain.toSkolem, variantPlain.ofSkolem, HListPlain.ofSkolem_toSkolem θ fields]
  | _ :: names, .cons _ rest, .inr value => by
      simp only [variantPlain.toSkolem, variantPlain.ofSkolem, variantPlain.ofSkolem_toSkolem θ names rest]
end

mutual
/-- A callee's lent value read back in the callee's view is the value. -/
theorem NTy.plainToSkolem_ofSkolem (θ : TypeArgs) : (τ : NTy) →
    (value : @NTy.plain (Carriers.instantiate θ Θ) τ) →
    NTy.plainToSkolem θ τ (NTy.plainOfSkolem θ τ value) = value
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _
  | .bytes, _ | .char, _ | .param _, _ | .function _ _ _, _ | .nominal _ _ _, _ => rfl
  | .tuple elements, values => HListPlain.toSkolem_ofSkolem θ elements values
  | .struct _ _ fields, values => HListPlain.toSkolem_ofSkolem θ fields values
  | .enum _ _ names rows _, value => variantPlain.toSkolem_ofSkolem θ names rows value
  | .vector element, vector => by
      apply SpecVector.ext
      simp only [NTy.plainToSkolem, NTy.plainOfSkolem, Array.map_map]
      conv => rhs; rw [← Array.map_id vector.values]
      apply Array.map_congr_left
      intro value _
      exact NTy.plainToSkolem_ofSkolem θ element value
  | .slice element, vector => by
      apply SpecVector.ext
      simp only [NTy.plainToSkolem, NTy.plainOfSkolem, Array.map_map]
      conv => rhs; rw [← Array.map_id vector.values]
      apply Array.map_congr_left
      intro value _
      exact NTy.plainToSkolem_ofSkolem θ element value
  | .array element _, array => by
      apply SpecArray.ext
      apply SpecVector.ext
      simp only [NTy.plainToSkolem, NTy.plainOfSkolem, Array.map_map]
      conv => rhs; rw [← Array.map_id array.vector.values]
      apply Array.map_congr_left
      intro value _
      exact NTy.plainToSkolem_ofSkolem θ element value
  | .table key value, entries => by
      apply SpecEntries.ext
      simp only [NTy.plainToSkolem, NTy.plainOfSkolem, List.map_map]
      conv => rhs; rw [← List.map_id entries.entries]
      apply List.map_congr_left
      intro entry _
      simp only [Function.comp_apply, NTy.plainToSkolem_ofSkolem θ key, NTy.plainToSkolem_ofSkolem θ value, id_eq]
  | .ghost slot, value => NTy.plainToSkolem_ofSkolem θ slot value
  | .ref referent, value => by
      simp only [NTy.plainToSkolem, NTy.plainOfSkolem, NTy.plainToSkolem_ofSkolem θ referent]
  | .sliceRef element, value => by
      simp only [NTy.plainToSkolem, NTy.plainOfSkolem, SpecSliceRef.map_map]
      have same : (fun x => NTy.plainToSkolem θ element (NTy.plainOfSkolem θ element x)) = id :=
        funext (NTy.plainToSkolem_ofSkolem θ element)
      rw [show _ ∘ _ = id from same, SpecSliceRef.map_id]
  | .observed referent, value => NTy.plainToSkolem_ofSkolem θ referent value

theorem HListPlain.toSkolem_ofSkolem (θ : TypeArgs) : (row : NRow) →
    (values : @HListPlain (Carriers.instantiate θ Θ) row) →
    HListPlain.toSkolem θ row (HListPlain.ofSkolem θ row values) = values
  | .nil, _ => rfl
  | .cons τ rest, values => by
      simp only [HListPlain.toSkolem, HListPlain.ofSkolem, NTy.plainToSkolem_ofSkolem θ τ, HListPlain.toSkolem_ofSkolem θ rest]
      rfl

theorem variantPlain.toSkolem_ofSkolem (θ : TypeArgs) : (names : List String) → (rows : NRows) →
    (value : @variantPlain (Carriers.instantiate θ Θ) names rows) →
    variantPlain.toSkolem θ names rows (variantPlain.ofSkolem θ names rows value) = value
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl values => by
      simp only [variantPlain.toSkolem, variantPlain.ofSkolem, HListPlain.toSkolem_ofSkolem θ fields]
  | _ :: names, .cons _ rest, .inr value => by
      simp only [variantPlain.toSkolem, variantPlain.ofSkolem, variantPlain.toSkolem_ofSkolem θ names rest]
end

mutual
/-- Transport of a lent value to the callee's view keeps its encoding. -/
theorem NTy.plainEncode_toSkolem (θ : TypeArgs) : (τ : NTy) → (value : (τ.subst θ.1).plain) →
    @NTy.plainEncode (Carriers.instantiate θ Θ) τ (NTy.plainToSkolem θ τ value) =
      NTy.plainEncode (τ.subst θ.1) value
  | .unit, _ => rfl
  | .bool, _ => rfl
  | .int _ _, _ => rfl
  | .num, _ => rfl
  | .address, _ => rfl
  | .signer, _ => rfl
  | .string, _ => rfl
  | .bytes, _ => rfl
  | .char, _ => rfl
  | .param _, _ => rfl
  | .function _ _ _, _ => rfl
  | .nominal _ _ _, _ => rfl
  | .observed referent, value => NTy.plainEncode_toSkolem θ referent value
  | .tuple elements, values => by
      show @NTy.plainEncode (Carriers.instantiate θ Θ) (.tuple elements)
          (HListPlain.toSkolem θ elements values) =
        NTy.plainEncode (.tuple (NRow.subst θ.1 elements)) values
      rw [@NTy.plainEncode_tuple (Carriers.instantiate θ Θ), NTy.plainEncode_tuple,
        HListPlain.encode_toSkolem θ elements values]
  | .struct source arguments fields, values => by
      show @NTy.plainEncode (Carriers.instantiate θ Θ) (.struct source arguments fields)
          (HListPlain.toSkolem θ fields values) =
        NTy.plainEncode (.struct source (NRow.subst θ.1 arguments) (NRow.subst θ.1 fields)) values
      rw [@NTy.plainEncode_struct (Carriers.instantiate θ Θ), NTy.plainEncode_struct,
        HListPlain.encode_toSkolem θ fields values]
  | .enum source arguments names rows distinct, value =>
      variantPlain.encode_toSkolem θ source names rows distinct value
  | .vector element, vector => by
      show @NTy.plainEncode (Carriers.instantiate θ Θ) (.vector element)
          (NTy.plainToSkolem θ (.vector element) vector) =
        NTy.plainEncode (.vector (element.subst θ.1)) vector
      rw [@NTy.plainEncode_vector (Carriers.instantiate θ Θ), NTy.plainEncode_vector]
      simp only [NTy.plainToSkolem, Array.map_map]
      congr 1
      apply Array.map_congr_left
      intro value _
      exact NTy.plainEncode_toSkolem θ element value
  | .slice element, vector => by
      show @NTy.plainEncode (Carriers.instantiate θ Θ) (.slice element)
          (NTy.plainToSkolem θ (.slice element) vector) =
        NTy.plainEncode (.slice (element.subst θ.1)) vector
      rw [@NTy.plainEncode_slice (Carriers.instantiate θ Θ), NTy.plainEncode_slice]
      simp only [NTy.plainToSkolem, Array.map_map]
      congr 1
      apply Array.map_congr_left
      intro value _
      exact NTy.plainEncode_toSkolem θ element value
  | .array element length, array => by
      show @NTy.plainEncode (Carriers.instantiate θ Θ) (.array element length)
          (NTy.plainToSkolem θ (.array element length) array) =
        NTy.plainEncode (.array (element.subst θ.1) length) array
      rw [@NTy.plainEncode_array (Carriers.instantiate θ Θ), NTy.plainEncode_array]
      simp only [NTy.plainToSkolem, Array.map_map]
      congr 1
      apply Array.map_congr_left
      intro value _
      exact NTy.plainEncode_toSkolem θ element value
  | .sliceRef element, value => by
      show @NTy.plainEncode (Carriers.instantiate θ Θ) (.sliceRef element)
          (NTy.plainToSkolem θ (.sliceRef element) value) =
        NTy.plainEncode (.sliceRef (element.subst θ.1)) value
      rw [@NTy.plainEncode_sliceRef (Carriers.instantiate θ Θ), NTy.plainEncode_sliceRef,
        @NTy.plainEncode_vector (Carriers.instantiate θ Θ),
        @NTy.plainEncode_vector (Carriers.instantiate θ Θ), NTy.plainEncode_vector,
        NTy.plainEncode_vector]
      simp only [NTy.plainToSkolem, SpecSliceRef.map_currentVector,
        SpecSliceRef.map_finalVector, SpecVector.map_values, Array.map_map]
      have same : (@NTy.plainCodec (Carriers.instantiate θ Θ) element).encode ∘
          NTy.plainToSkolem θ element = (element.subst θ.1).plainCodec.encode :=
        funext (NTy.plainEncode_toSkolem θ element)
      rw [same]
  | .table key value, entries => by
      show @NTy.plainEncode (Carriers.instantiate θ Θ) (.table key value)
          (NTy.plainToSkolem θ (.table key value) entries) =
        NTy.plainEncode (.table (key.subst θ.1) (value.subst θ.1)) entries
      rw [@NTy.plainEncode_table (Carriers.instantiate θ Θ), NTy.plainEncode_table]
      simp only [NTy.plainToSkolem, List.map_map]
      congr 2
      apply List.map_congr_left
      intro entry _
      simp only [Function.comp_apply, NTy.plainEncode_toSkolem θ key entry.1, NTy.plainEncode_toSkolem θ value entry.2]
  | .ghost slot, value => by
      show @NTy.plainEncode (Carriers.instantiate θ Θ) (.ghost slot)
          (NTy.plainToSkolem θ (.ghost slot) value) = NTy.plainEncode (.ghost (slot.subst θ.1)) value
      rw [@NTy.plainEncode_ghost (Carriers.instantiate θ Θ), NTy.plainEncode_ghost]
      simp only [NTy.plainToSkolem]
      rw [NTy.plainEncode_toSkolem θ slot value]
  | .ref referent, value => by
      show @NTy.plainEncode (Carriers.instantiate θ Θ) (.ref referent)
          (NTy.plainToSkolem θ (.ref referent) value) =
        NTy.plainEncode (.ref (referent.subst θ.1)) value
      rw [@NTy.plainEncode_ref (Carriers.instantiate θ Θ), NTy.plainEncode_ref]
      simp only [NTy.plainToSkolem]
      rw [NTy.plainEncode_toSkolem θ referent value.1, NTy.plainEncode_toSkolem θ referent value.2]

theorem HListPlain.encode_toSkolem (θ : TypeArgs) : (row : NRow) →
    (values : HListPlain (NRow.subst θ.1 row)) →
    @HListPlain.encode (Carriers.instantiate θ Θ) row (HListPlain.toSkolem θ row values) =
      HListPlain.encode values
  | .nil, _ => rfl
  | .cons τ rest, values => by
      show @HListPlain.encode (Carriers.instantiate θ Θ) (.cons τ rest)
          (NTy.plainToSkolem θ τ values.1, HListPlain.toSkolem θ rest values.2) =
        HListPlain.encode (Γ := .cons (τ.subst θ.1) (NRow.subst θ.1 rest)) values
      rw [@HListPlain.encode_cons (Carriers.instantiate θ Θ), HListPlain.encode_cons]
      simp only
      rw [NTy.plainEncode_toSkolem θ τ values.1, HListPlain.encode_toSkolem θ rest values.2]

theorem variantPlain.encode_toSkolem (θ : TypeArgs) (source : StructHandle) :
    (names : List String) → (rows : NRows) → (distinct : names.Nodup) →
    (value : variantPlain names (rows.subst θ.1)) →
    @NTy.plainEncode (Carriers.instantiate θ Θ) (.enum source arguments names rows distinct)
        (variantPlain.toSkolem θ names rows value) =
      NTy.plainEncode (.enum source (NRow.subst θ.1 arguments) names (rows.subst θ.1) distinct)
        value
  | _, .nil, _, value => nomatch value
  | [], .cons _ _, _, value => nomatch value
  | name :: names, .cons fields rest, distinct, .inl values => by
      show @NTy.plainEncode (Carriers.instantiate θ Θ)
            (.enum source arguments (name :: names) (.cons fields rest) distinct)
          (.inl (HListPlain.toSkolem θ fields values)) =
        NTy.plainEncode (.enum source (NRow.subst θ.1 arguments) (name :: names)
          (.cons (NRow.subst θ.1 fields) (rest.subst θ.1)) distinct) (.inl values)
      rw [@NTy.plainEncode_enum_inl (Carriers.instantiate θ Θ), NTy.plainEncode_enum_inl,
        HListPlain.encode_toSkolem θ fields values]
  | name :: names, .cons fields rest, distinct, .inr value => by
      show @NTy.plainEncode (Carriers.instantiate θ Θ)
            (.enum source arguments (name :: names) (.cons fields rest) distinct)
          (.inr (variantPlain.toSkolem θ names rest value)) =
        NTy.plainEncode (.enum source (NRow.subst θ.1 arguments) (name :: names)
          (.cons (NRow.subst θ.1 fields) (rest.subst θ.1)) distinct) (.inr value)
      rw [@NTy.plainEncode_enum_inr (Carriers.instantiate θ Θ), NTy.plainEncode_enum_inr]
      exact variantPlain.encode_toSkolem θ source names rest _ value
end

/-- Transport of a lent value to the caller's view keeps its encoding
(`NTy.plainEncode_toSkolem`). -/
theorem NTy.plainEncode_ofSkolem (θ : TypeArgs) (τ : NTy)
    (value : @NTy.plain (Carriers.instantiate θ Θ) τ) :
    NTy.plainEncode (τ.subst θ.1) (NTy.plainOfSkolem θ τ value) =
      @NTy.plainEncode (Carriers.instantiate θ Θ) τ value := by
  have step := NTy.plainEncode_toSkolem θ τ (NTy.plainOfSkolem θ τ value)
  rw [NTy.plainToSkolem_ofSkolem θ τ value] at step
  exact step.symm

attribute [vir_denote_norm] NTy.plainEncode_ofSkolem

/-- Transport of a row of lent values to the caller's view keeps its
encoding (`HListPlain.encode_toSkolem`). -/
theorem HListPlain.encode_ofSkolem (θ : TypeArgs) (row : NRow)
    (values : @HListPlain (Carriers.instantiate θ Θ) row) :
    HListPlain.encode (HListPlain.ofSkolem θ row values) =
      @HListPlain.encode (Carriers.instantiate θ Θ) row values := by
  have step := HListPlain.encode_toSkolem θ row (HListPlain.ofSkolem θ row values)
  rw [HListPlain.toSkolem_ofSkolem θ row values] at step
  exact step.symm

mutual
/-- A transported lent value satisfies the data invariants exactly where
the caller's value does: a declaration's clauses read the value's encoding,
which the transport keeps, and the clauses an induced family states are the
caller's at the substituted arguments. -/
theorem NTy.deepHolds_toSkolem (θ : TypeArgs) : (τ : NTy) → (value : (τ.subst θ.1).plain) →
    (@NTy.deepHolds (Carriers.instantiate θ Θ) τ (NTy.plainToSkolem θ τ value) ↔
      @NTy.deepHolds Θ (τ.subst θ.1) value)
  | .unit, _ => Iff.rfl
  | .bool, _ => Iff.rfl
  | .int _ _, _ => Iff.rfl
  | .num, _ => Iff.rfl
  | .address, _ => Iff.rfl
  | .signer, _ => Iff.rfl
  | .string, _ => Iff.rfl
  | .bytes, _ => Iff.rfl
  | .char, _ => Iff.rfl
  | .param _, _ => Iff.rfl
  | .function _ _ _, _ => Iff.rfl
  | .nominal _ _ _, _ => Iff.rfl
  | .tuple elements, values => HListPlain.deepHolds_toSkolem θ elements values
  | .ghost slot, value => NTy.deepHolds_toSkolem θ slot value
  | .ref referent, value => NTy.deepHolds_toSkolem θ referent value.2
  | .observed referent, value => NTy.deepHolds_toSkolem θ referent value
  | .struct source arguments fields, values => by
      simp only [NTy.deepHolds_struct]
      rw [NTy.plainEncode_toSkolem θ (.struct source arguments fields) values]
      exact and_congr Iff.rfl (HListPlain.deepHolds_toSkolem θ fields values)
  | .enum source arguments names rows distinct, value => by
      simp only [NTy.deepHolds_enum]
      rw [NTy.plainEncode_toSkolem θ (.enum source arguments names rows distinct) value]
      exact and_congr Iff.rfl (variantPlain.deepHolds_toSkolem θ names rows value)
  | .vector element, vector => by
      show (∀ value ∈ (vector.values.map (NTy.plainToSkolem θ element)),
          @NTy.deepHolds (Carriers.instantiate θ Θ) element value) ↔
        ∀ value ∈ vector.values, @NTy.deepHolds Θ (element.subst θ.1) value
      refine ⟨fun holds value member => ?_, fun holds value member => ?_⟩
      · exact (NTy.deepHolds_toSkolem θ element value).mp
          (holds _ (Array.mem_map.mpr ⟨value, member, rfl⟩))
      · obtain ⟨source, inside, equal⟩ := Array.mem_map.mp member
        exact equal ▸ (NTy.deepHolds_toSkolem θ element source).mpr (holds source inside)
  | .slice element, vector => by
      show (∀ value ∈ (vector.values.map (NTy.plainToSkolem θ element)),
          @NTy.deepHolds (Carriers.instantiate θ Θ) element value) ↔
        ∀ value ∈ vector.values, @NTy.deepHolds Θ (element.subst θ.1) value
      refine ⟨fun holds value member => ?_, fun holds value member => ?_⟩
      · exact (NTy.deepHolds_toSkolem θ element value).mp
          (holds _ (Array.mem_map.mpr ⟨value, member, rfl⟩))
      · obtain ⟨source, inside, equal⟩ := Array.mem_map.mp member
        exact equal ▸ (NTy.deepHolds_toSkolem θ element source).mpr (holds source inside)
  | .array element _, array => by
      show (∀ value ∈ (array.vector.values.map (NTy.plainToSkolem θ element)),
          @NTy.deepHolds (Carriers.instantiate θ Θ) element value) ↔
        ∀ value ∈ array.vector.values, @NTy.deepHolds Θ (element.subst θ.1) value
      refine ⟨fun holds value member => ?_, fun holds value member => ?_⟩
      · exact (NTy.deepHolds_toSkolem θ element value).mp
          (holds _ (Array.mem_map.mpr ⟨value, member, rfl⟩))
      · obtain ⟨source, inside, equal⟩ := Array.mem_map.mp member
        exact equal ▸ (NTy.deepHolds_toSkolem θ element source).mpr (holds source inside)
  | .sliceRef element, value => by
      have reading : ∀ values : Array (element.subst θ.1).plain,
          (∀ transported ∈ (values.map (NTy.plainToSkolem θ element)),
              @NTy.deepHolds (Carriers.instantiate θ Θ) element transported) ↔
            ∀ source ∈ values, @NTy.deepHolds Θ (element.subst θ.1) source := by
        intro values
        refine ⟨fun holds source member => ?_, fun holds transported member => ?_⟩
        · exact (NTy.deepHolds_toSkolem θ element source).mp
            (holds _ (Array.mem_map.mpr ⟨source, member, rfl⟩))
        · obtain ⟨source, inside, equal⟩ := Array.mem_map.mp member
          exact equal ▸ (NTy.deepHolds_toSkolem θ element source).mpr (holds source inside)
      show ((∀ current ∈ (value.currentVector.values.map (NTy.plainToSkolem θ element)),
            @NTy.deepHolds (Carriers.instantiate θ Θ) element current) ∧
          ∀ final ∈ (value.finalVector.values.map (NTy.plainToSkolem θ element)),
            @NTy.deepHolds (Carriers.instantiate θ Θ) element final) ↔
        ((∀ current ∈ value.currentVector.values,
              @NTy.deepHolds Θ (element.subst θ.1) current) ∧
          ∀ final ∈ value.finalVector.values, @NTy.deepHolds Θ (element.subst θ.1) final)
      exact and_congr (reading value.currentVector.values) (reading value.finalVector.values)
  | .table key value, entries => by
      show (∀ entry ∈ (entries.entries.map fun entry =>
            (NTy.plainToSkolem θ key entry.1, NTy.plainToSkolem θ value entry.2)),
          @NTy.deepHolds (Carriers.instantiate θ Θ) key entry.1 ∧
            @NTy.deepHolds (Carriers.instantiate θ Θ) value entry.2) ↔
        ∀ entry ∈ entries.entries, @NTy.deepHolds Θ (key.subst θ.1) entry.1 ∧
          @NTy.deepHolds Θ (value.subst θ.1) entry.2
      refine ⟨fun holds entry member => ?_, fun holds entry member => ?_⟩
      · obtain ⟨first, second⟩ := holds _ (List.mem_map.mpr ⟨entry, member, rfl⟩)
        exact ⟨(NTy.deepHolds_toSkolem θ key entry.1).mp first,
          (NTy.deepHolds_toSkolem θ value entry.2).mp second⟩
      · obtain ⟨source, inside, equal⟩ := List.mem_map.mp member
        obtain ⟨first, second⟩ := holds source inside
        exact equal ▸ ⟨(NTy.deepHolds_toSkolem θ key source.1).mpr first,
          (NTy.deepHolds_toSkolem θ value source.2).mpr second⟩

theorem HListPlain.deepHolds_toSkolem (θ : TypeArgs) : (row : NRow) →
    (values : HListPlain (NRow.subst θ.1 row)) →
    (@HListPlain.deepHolds (Carriers.instantiate θ Θ) row (HListPlain.toSkolem θ row values) ↔
      @HListPlain.deepHolds Θ (NRow.subst θ.1 row) values)
  | .nil, _ => Iff.rfl
  | .cons τ rest, values =>
      and_congr (NTy.deepHolds_toSkolem θ τ values.1)
        (HListPlain.deepHolds_toSkolem θ rest values.2)

theorem variantPlain.deepHolds_toSkolem (θ : TypeArgs) : (names : List String) → (rows : NRows) →
    (value : variantPlain names (NRows.subst θ.1 rows)) →
    (@variantPlain.deepHolds (Carriers.instantiate θ Θ) names rows
        (variantPlain.toSkolem θ names rows value) ↔
      @variantPlain.deepHolds Θ names (NRows.subst θ.1 rows) value)
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl values => HListPlain.deepHolds_toSkolem θ fields values
  | _ :: names, .cons _ rest, .inr value => variantPlain.deepHolds_toSkolem θ names rest value
end

/-- A transported lent value satisfies the data invariants in the callee's
view exactly where the caller's view does (`NTy.deepHolds_toSkolem`). -/
theorem NTy.deepHolds_ofSkolem (θ : TypeArgs) (τ : NTy)
    (value : @NTy.plain (Carriers.instantiate θ Θ) τ) :
    (@NTy.deepHolds Θ (τ.subst θ.1) (NTy.plainOfSkolem θ τ value) ↔
      @NTy.deepHolds (Carriers.instantiate θ Θ) τ value) := by
  have step := NTy.deepHolds_toSkolem θ τ (NTy.plainOfSkolem θ τ value)
  rw [NTy.plainToSkolem_ofSkolem θ τ value] at step
  exact step.symm

mutual
/-- A caller's value in the callee's view: the same runtime value, carried
at the family the call's type arguments induce. -/
def NTy.toSkolem (θ : TypeArgs) : (τ : NTy) → (τ.subst θ.1).carrier →
    @NTy.carrier (Carriers.instantiate θ Θ) τ
  | .unit, value => value
  | .bool, value => value
  | .int _ _, value => value
  | .num, value => value
  | .address, value => value
  | .signer, value => value
  | .string, value => value
  | .bytes, value => value
  | .char, value => value
  | .tuple elements, values => HList.toSkolem θ elements values
  | .struct source arguments fields, values =>
      ⟨HListPlain.toSkolem θ fields values.val,
        (NTy.deepHolds_toSkolem θ (.struct source arguments fields) values.val).mpr
          values.property⟩
  | .enum source arguments names rows distinct, value =>
      ⟨variantPlain.toSkolem θ names rows value.val,
        (NTy.deepHolds_toSkolem θ (.enum source arguments names rows distinct) value.val).mpr
          value.property⟩
  | .slice element, vector =>
      ⟨vector.values.map (NTy.toSkolem θ element), by simpa using vector.bounded⟩

  | .vector element, vector =>
      ⟨vector.values.map (NTy.toSkolem θ element), by simpa using vector.bounded⟩
  | .array element _, array =>
      ⟨⟨array.vector.values.map (NTy.toSkolem θ element), by simpa using array.vector.bounded⟩,
        by simpa using array.size_eq⟩
  | .table key value, entries =>
      ⟨entries.entries.map fun entry => (NTy.toSkolem θ key entry.1, NTy.toSkolem θ value entry.2),
        by simpa using entries.bounded⟩
  | .ghost slot, value => NTy.toSkolem θ slot value
  | .observed referent, value => NTy.toSkolem θ referent value
  | .sliceRef element, value => value.map (NTy.toSkolem θ element)
  | .ref referent, value =>
      (NTy.plainToSkolem θ referent value.1, NTy.toSkolem θ referent value.2)

  | .param _, value => value
  | .function _ _ _, value | .nominal _ _ _, value => value

def HList.toSkolem (θ : TypeArgs) : (row : NRow) → HList (NRow.subst θ.1 row) →
    @HList (Carriers.instantiate θ Θ) row
  | .nil, _ => ()
  | .cons τ rest, values => (NTy.toSkolem θ τ values.1, HList.toSkolem θ rest values.2)

def variantCarrier.toSkolem (θ : TypeArgs) : (names : List String) → (rows : NRows) →
    variantCarrier names (NRows.subst θ.1 rows) → @variantCarrier (Carriers.instantiate θ Θ) names rows
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl values => .inl (HList.toSkolem θ fields values)
  | _ :: names, .cons _ rest, .inr value => .inr (variantCarrier.toSkolem θ names rest value)
end

mutual
/-- A callee's value in the caller's view. -/
def NTy.ofSkolem (θ : TypeArgs) : (τ : NTy) → @NTy.carrier (Carriers.instantiate θ Θ) τ →
    (τ.subst θ.1).carrier
  | .unit, value => value
  | .bool, value => value
  | .int _ _, value => value
  | .num, value => value
  | .address, value => value
  | .signer, value => value
  | .string, value => value
  | .bytes, value => value
  | .char, value => value
  | .tuple elements, values => HList.ofSkolem θ elements values
  | .struct source arguments fields, values =>
      ⟨HListPlain.ofSkolem θ fields values.val,
        (NTy.deepHolds_ofSkolem θ (.struct source arguments fields) values.val).mpr
          values.property⟩
  | .enum source arguments names rows distinct, value =>
      ⟨variantPlain.ofSkolem θ names rows value.val,
        (NTy.deepHolds_ofSkolem θ (.enum source arguments names rows distinct) value.val).mpr
          value.property⟩
  | .slice element, vector =>
      ⟨vector.values.map (NTy.ofSkolem θ element), by simpa using vector.bounded⟩

  | .vector element, vector =>
      ⟨vector.values.map (NTy.ofSkolem θ element), by simpa using vector.bounded⟩
  | .array element _, array =>
      ⟨⟨array.vector.values.map (NTy.ofSkolem θ element), by simpa using array.vector.bounded⟩,
        by simpa using array.size_eq⟩
  | .table key value, entries =>
      ⟨entries.entries.map fun entry => (NTy.ofSkolem θ key entry.1, NTy.ofSkolem θ value entry.2),
        by simpa using entries.bounded⟩
  | .ghost slot, value => NTy.ofSkolem θ slot value
  | .observed referent, value => NTy.ofSkolem θ referent value
  | .sliceRef element, value => value.map (NTy.ofSkolem θ element)
  | .ref referent, value =>
      (NTy.plainOfSkolem θ referent value.1, NTy.ofSkolem θ referent value.2)

  | .param _, value => value
  | .function _ _ _, value | .nominal _ _ _, value => value

def HList.ofSkolem (θ : TypeArgs) : (row : NRow) → @HList (Carriers.instantiate θ Θ) row →
    HList (NRow.subst θ.1 row)
  | .nil, _ => ()
  | .cons τ rest, values => (NTy.ofSkolem θ τ values.1, HList.ofSkolem θ rest values.2)

def variantCarrier.ofSkolem (θ : TypeArgs) : (names : List String) → (rows : NRows) →
    @variantCarrier (Carriers.instantiate θ Θ) names rows → variantCarrier names (NRows.subst θ.1 rows)
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl values => .inl (HList.ofSkolem θ fields values)
  | _ :: names, .cons _ rest, .inr value => .inr (variantCarrier.ofSkolem θ names rest value)
end

/-- The inhabitants of an induced family: the caller's inhabitant of the
argument's type, in the callee's view. -/
instance CarriersNonempty.instantiate (θ : TypeArgs) (outer : Carriers)
    [inner : @CarriersNonempty outer] : @CarriersNonempty (Carriers.instantiate θ outer) where
  inhabitant := fun τ inhabitable =>
    @NTy.toSkolem outer inner θ τ
      (@NTy.inhabitant outer inner (τ.subst θ.1) (NTy.inhabitable_subst θ.1 θ.inhabitable τ inhabitable))
  inhabitant_nominal := fun source arguments _ held =>
    inner.inhabitant_nominal source (NRow.subst θ.1 arguments) _
      (NRow.inhabitable_subst θ.1 θ.inhabitable arguments held)

/-- A callee's result in the caller's view. -/
def ResultShape.ofSkolem (θ : TypeArgs) : (shape : ResultShape) →
    @ResultShape.carrier (Carriers.instantiate θ Θ) shape → (shape.subst θ.1).carrier
  | .none, value => value
  | .one τ, value => NTy.ofSkolem θ τ value

/-- The default of a parameter under an induced family is its argument's
value, the one a twin of the argument's type defaults to. -/
theorem Carriers.default_instantiate (θ : TypeArgs) (index : Nat) :
    (default : (Carriers.instantiate θ Θ).carrier index) =
      NTy.inhabitant ((NTy.param index).subst θ.1)
        ((NTy.inhabitable_unmark _).trans
          (NRow.getD_inhabitable θ.1 θ.inhabitable index (.param index) rfl)) := rfl

/-- A type parameter's codec is its family's. -/
theorem NTy.codec_param (index : Nat) : (NTy.param index).codec = Carriers.codec index := rfl

/-- A type parameter's encoding is its family's codec. -/
theorem NTy.encode_param (index : Nat) (value : (NTy.param index).carrier) :
    NTy.encode (.param index) value = (Carriers.codec index).encode value := rfl

/-- The codec an induced family gives a parameter is its argument's. -/
theorem Carriers.codec_instantiate (θ : TypeArgs) (index : Nat) :
    (Carriers.instantiate θ Θ).codec index = NTy.codec ((NTy.param index).subst θ.1) := rfl

/-! A callee's value in the caller's view, constructor by constructor: the
normalizer reduces the transport of a value it knows the shape of, so a
variant name or a field read through it is the one the callee's contract
states. -/

theorem NTy.ofSkolem_unit (θ : TypeArgs) (value : @NTy.carrier (Carriers.instantiate θ Θ) .unit) :
    NTy.ofSkolem θ .unit value = value := rfl
theorem NTy.ofSkolem_bool (θ : TypeArgs) (value : @NTy.carrier (Carriers.instantiate θ Θ) .bool) :
    NTy.ofSkolem θ .bool value = value := rfl
theorem NTy.ofSkolem_int (θ : TypeArgs) (width : Nat) (signed : Bool)
    (value : @NTy.carrier (Carriers.instantiate θ Θ) (.int width signed)) :
    NTy.ofSkolem θ (.int width signed) value = value := rfl
theorem NTy.ofSkolem_num (θ : TypeArgs) (value : @NTy.carrier (Carriers.instantiate θ Θ) .num) :
    NTy.ofSkolem θ .num value = value := rfl
theorem NTy.ofSkolem_address (θ : TypeArgs)
    (value : @NTy.carrier (Carriers.instantiate θ Θ) .address) :
    NTy.ofSkolem θ .address value = value := rfl
theorem NTy.ofSkolem_signer (θ : TypeArgs)
    (value : @NTy.carrier (Carriers.instantiate θ Θ) .signer) :
    NTy.ofSkolem θ .signer value = value := rfl
theorem NTy.ofSkolem_string (θ : TypeArgs)
    (value : @NTy.carrier (Carriers.instantiate θ Θ) .string) :
    NTy.ofSkolem θ .string value = value := rfl
theorem NTy.ofSkolem_bytes (θ : TypeArgs) (value : @NTy.carrier (Carriers.instantiate θ Θ) .bytes) :
    NTy.ofSkolem θ .bytes value = value := rfl
theorem NTy.ofSkolem_param (θ : TypeArgs) (index : Nat)
    (value : @NTy.carrier (Carriers.instantiate θ Θ) (.param index)) :
    NTy.ofSkolem θ (.param index) value = value := rfl
theorem NTy.ofSkolem_tuple (θ : TypeArgs) (elements : NRow)
    (values : @NTy.carrier (Carriers.instantiate θ Θ) (.tuple elements)) :
    NTy.ofSkolem θ (.tuple elements) values = HList.ofSkolem θ elements values := rfl
theorem NTy.ofSkolem_struct (θ : TypeArgs) (source : StructHandle) (fields : NRow)
    (values : @NTy.carrier (Carriers.instantiate θ Θ) (.struct source arguments fields)) :
    (NTy.ofSkolem θ (.struct source arguments fields) values).val =
      HListPlain.ofSkolem θ fields values.val := rfl
theorem NTy.ofSkolem_enum (θ : TypeArgs) (source : StructHandle) (names : List String)
    (rows : NRows) (distinct : names.Nodup)
    (value : @NTy.carrier (Carriers.instantiate θ Θ) (.enum source arguments names rows distinct)) :
    (NTy.ofSkolem θ (.enum source arguments names rows distinct) value).val =
      variantPlain.ofSkolem θ names rows value.val := rfl
theorem NTy.ofSkolem_vector (θ : TypeArgs) (element : NTy)
    (vector : @NTy.carrier (Carriers.instantiate θ Θ) (.vector element)) :
    NTy.ofSkolem θ (.vector element) vector =
      ⟨vector.values.map (NTy.ofSkolem θ element), by simpa using vector.bounded⟩ := rfl
theorem NTy.ofSkolem_ref (θ : TypeArgs) (referent : NTy)
    (value : @NTy.carrier (Carriers.instantiate θ Θ) (.ref referent)) :
    NTy.ofSkolem θ (.ref referent) value =
      (NTy.plainOfSkolem θ referent value.1, NTy.ofSkolem θ referent value.2) := rfl
theorem NTy.ofSkolem_ghost (θ : TypeArgs) (slot : NTy)
    (value : @NTy.carrier (Carriers.instantiate θ Θ) (.ghost slot)) :
    NTy.ofSkolem θ (.ghost slot) value = NTy.ofSkolem θ slot value := rfl
/-! A callee's transport unapplied, as a vector maps it over its elements:
the function a constructor's transport is, so that a mapped transport of
scalars is the identity and one of rows is the rows'. A table's transport
stays folded (`encodeTransport`). -/

@[vir_denote] theorem NTy.ofSkolem_unit_fun (θ : TypeArgs) :
    @NTy.ofSkolem Θ inferInstance θ .unit = fun value => value := rfl
@[vir_denote] theorem NTy.ofSkolem_bool_fun (θ : TypeArgs) :
    @NTy.ofSkolem Θ inferInstance θ .bool = fun value => value := rfl
@[vir_denote] theorem NTy.ofSkolem_int_fun (θ : TypeArgs) (width : Nat) (signed : Bool) :
    @NTy.ofSkolem Θ inferInstance θ (.int width signed) = fun value => value := rfl
@[vir_denote] theorem NTy.ofSkolem_num_fun (θ : TypeArgs) :
    @NTy.ofSkolem Θ inferInstance θ .num = fun value => value := rfl
@[vir_denote] theorem NTy.ofSkolem_address_fun (θ : TypeArgs) :
    @NTy.ofSkolem Θ inferInstance θ .address = fun value => value := rfl
@[vir_denote] theorem NTy.ofSkolem_signer_fun (θ : TypeArgs) :
    @NTy.ofSkolem Θ inferInstance θ .signer = fun value => value := rfl
@[vir_denote] theorem NTy.ofSkolem_string_fun (θ : TypeArgs) :
    @NTy.ofSkolem Θ inferInstance θ .string = fun value => value := rfl
@[vir_denote] theorem NTy.ofSkolem_bytes_fun (θ : TypeArgs) :
    @NTy.ofSkolem Θ inferInstance θ .bytes = fun value => value := rfl
@[vir_denote] theorem NTy.ofSkolem_param_fun (θ : TypeArgs) (index : Nat) :
    @NTy.ofSkolem Θ inferInstance θ (.param index) = fun value => value := rfl
@[vir_denote] theorem NTy.ofSkolem_tuple_fun (θ : TypeArgs) (elements : NRow) :
    @NTy.ofSkolem Θ inferInstance θ (.tuple elements) = fun values => HList.ofSkolem θ elements values := rfl
@[vir_denote] theorem NTy.ofSkolem_struct_fun (θ : TypeArgs) (source : StructHandle)
    (fields : NRow) :
    @NTy.ofSkolem Θ inferInstance θ (.struct source arguments fields) =
      fun values => ⟨HListPlain.ofSkolem θ fields values.val,
        (NTy.deepHolds_ofSkolem θ (.struct source arguments fields) values.val).mpr
          values.property⟩ := rfl
@[vir_denote] theorem NTy.ofSkolem_enum_fun (θ : TypeArgs) (source : StructHandle)
    (names : List String) (rows : NRows) (distinct : names.Nodup) :
    @NTy.ofSkolem Θ inferInstance θ (.enum source arguments names rows distinct) =
      fun value => ⟨variantPlain.ofSkolem θ names rows value.val,
        (NTy.deepHolds_ofSkolem θ (.enum source arguments names rows distinct) value.val).mpr
          value.property⟩ := rfl
@[vir_denote] theorem NTy.ofSkolem_vector_fun (θ : TypeArgs) (element : NTy) :
    @NTy.ofSkolem Θ inferInstance θ (.vector element) =
      fun vector => ⟨vector.values.map (NTy.ofSkolem θ element), by simpa using vector.bounded⟩ :=
  rfl
@[vir_denote] theorem NTy.ofSkolem_ref_fun (θ : TypeArgs) (referent : NTy) :
    @NTy.ofSkolem Θ inferInstance θ (.ref referent) =
      fun value => (NTy.plainOfSkolem θ referent value.1, NTy.ofSkolem θ referent value.2) := rfl

theorem HList.ofSkolem_nil (θ : TypeArgs) (values : @HList (Carriers.instantiate θ Θ) .nil) :
    HList.ofSkolem θ .nil values = () := rfl
theorem HList.ofSkolem_cons (θ : TypeArgs) (τ : NTy) (rest : NRow)
    (values : @HList (Carriers.instantiate θ Θ) (.cons τ rest)) :
    HList.ofSkolem θ (.cons τ rest) values =
      (NTy.ofSkolem θ τ values.1, HList.ofSkolem θ rest values.2) := rfl
theorem variantPlain.ofSkolem_inl (θ : TypeArgs) (name : String) (names : List String)
    (fields : NRow) (rest : NRows) (values : @HListPlain (Carriers.instantiate θ Θ) fields) :
    variantPlain.ofSkolem θ (name :: names) (.cons fields rest) (.inl values) =
      .inl (HListPlain.ofSkolem θ fields values) := rfl
theorem variantPlain.ofSkolem_inr (θ : TypeArgs) (name : String) (names : List String)
    (fields : NRow) (rest : NRows)
    (value : @variantPlain (Carriers.instantiate θ Θ) names rest) :
    variantPlain.ofSkolem θ (name :: names) (.cons fields rest) (.inr value) =
      .inr (variantPlain.ofSkolem θ names rest value) := rfl
theorem variantPlain.toSkolem_inl (θ : TypeArgs) (name : String) (names : List String)
    (fields : NRow) (rest : NRows) (values : HListPlain (NRow.subst θ.1 fields)) :
    variantPlain.toSkolem θ (name :: names) (.cons fields rest) (.inl values) =
      .inl (HListPlain.toSkolem θ fields values) := rfl
theorem variantPlain.toSkolem_inr (θ : TypeArgs) (name : String) (names : List String)
    (fields : NRow) (rest : NRows) (value : variantPlain names (NRows.subst θ.1 rest)) :
    variantPlain.toSkolem θ (name :: names) (.cons fields rest) (.inr value) =
      .inr (variantPlain.toSkolem θ names rest value) := rfl
theorem variantCarrier.ofSkolem_inl (θ : TypeArgs) (name : String) (names : List String)
    (fields : NRow) (rest : NRows) (values : @HList (Carriers.instantiate θ Θ) fields) :
    variantCarrier.ofSkolem θ (name :: names) (.cons fields rest) (.inl values) =
      .inl (HList.ofSkolem θ fields values) := rfl
theorem variantCarrier.ofSkolem_inr (θ : TypeArgs) (name : String) (names : List String)
    (fields : NRow) (rest : NRows) (value : @variantCarrier (Carriers.instantiate θ Θ) names rest) :
    variantCarrier.ofSkolem θ (name :: names) (.cons fields rest) (.inr value) =
      .inr (variantCarrier.ofSkolem θ names rest value) := rfl
theorem ResultShape.ofSkolem_none (θ : TypeArgs)
    (value : @ResultShape.carrier (Carriers.instantiate θ Θ) .none) :
    ResultShape.ofSkolem θ .none value = value := rfl
theorem ResultShape.ofSkolem_one (θ : TypeArgs) (τ : NTy)
    (value : @ResultShape.carrier (Carriers.instantiate θ Θ) (.one τ)) :
    ResultShape.ofSkolem θ (.one τ) value = NTy.ofSkolem θ τ value := rfl

/-! Transport to the caller's view keeps a value's runtime encoding: the
callee and the caller see the same runtime value. -/

mutual
theorem NTy.encode_ofSkolem (θ : TypeArgs) : (τ : NTy) →
    (value : @NTy.carrier (Carriers.instantiate θ Θ) τ) →
    NTy.encode (τ.subst θ.1) (NTy.ofSkolem θ τ value) =
      @NTy.encode (Carriers.instantiate θ Θ) τ value
  | .unit, _ => rfl
  | .bool, _ => rfl
  | .int _ _, _ => rfl
  | .num, _ => rfl
  | .address, _ => rfl
  | .signer, _ => rfl
  | .string, _ => rfl
  | .bytes, _ => rfl
  | .char, _ => rfl
  | .param _, _ => rfl
  | .function _ _ _, _ => rfl
  | .tuple elements, values => by
      show NTy.encode (.tuple (NRow.subst θ.1 elements)) (HList.ofSkolem θ elements values) = _
      rw [NTy.encode_tuple, @NTy.encode_tuple (Carriers.instantiate θ Θ),
        HList.encode_ofSkolem θ elements values]
  | .struct source arguments fields, values => by
      rw [NTy.encode_struct_val, @NTy.encode_struct_val (Carriers.instantiate θ Θ)]
      exact NTy.plainEncode_ofSkolem θ (.struct source arguments fields) values.val
  | .enum source arguments names rows distinct, value => by
      rw [NTy.encode_enum, @NTy.encode_enum (Carriers.instantiate θ Θ)]
      exact NTy.plainEncode_ofSkolem θ (.enum source arguments names rows distinct) value.val
  | .slice element, vector => by
      show NTy.encode (.slice (element.subst θ.1)) _ = _
      rw [NTy.encode_slice, @NTy.encode_slice (Carriers.instantiate θ Θ)]
      simp only [NTy.ofSkolem, Array.map_map]
      congr 1
      apply Array.map_congr_left
      intro x _
      exact NTy.encode_ofSkolem θ element x

  | .vector element, vector => by
      show NTy.encode (.vector (element.subst θ.1)) _ = _
      rw [NTy.encode_vector, @NTy.encode_vector (Carriers.instantiate θ Θ)]
      simp only [NTy.ofSkolem, Array.map_map]
      congr 1
      apply Array.map_congr_left
      intro x _
      exact NTy.encode_ofSkolem θ element x
  | .array element length, array => by
      show NTy.encode (.array (element.subst θ.1) length) _ = _
      rw [NTy.encode_array, @NTy.encode_array (Carriers.instantiate θ Θ)]
      simp only [NTy.ofSkolem, Array.map_map]
      congr 1
      apply Array.map_congr_left
      intro x _
      exact NTy.encode_ofSkolem θ element x
  | .table key value, entries => by
      show NTy.encode (.table (key.subst θ.1) (value.subst θ.1)) _ = _
      rw [NTy.encode_table, @NTy.encode_table (Carriers.instantiate θ Θ)]
      simp only [NTy.ofSkolem, List.map_map]
      congr 2
      apply List.map_congr_left
      intro entry _
      simp only [Function.comp_apply, NTy.encode_ofSkolem θ key entry.1,
        NTy.encode_ofSkolem θ value entry.2]
  | .sliceRef element, value => by
      simp only [NTy.encode, NTy.codec, Codec.sliceRef, Codec.boundedVector, NTy.ofSkolem,
        SpecSliceRef.map_currentVector, SpecSliceRef.map_finalVector, SpecVector.map, Array.map_map]
      have same : (element.subst θ.1).codec.encode ∘ NTy.ofSkolem θ element =
          (@NTy.codec (Carriers.instantiate θ Θ) element).encode :=
        funext (NTy.encode_ofSkolem θ element)
      rw [same]
  | .ref referent, value => by
      show NTy.encode (.ref (referent.subst θ.1)) _ = _
      rw [NTy.encode_ref, @NTy.encode_ref (Carriers.instantiate θ Θ)]
      simp only [NTy.ofSkolem]
      rw [NTy.plainEncode_ofSkolem θ referent value.1, NTy.encode_ofSkolem θ referent value.2]
  | .ghost slot, value => by
      show NTy.encode (.ghost (slot.subst θ.1)) _ = _
      rw [NTy.encode_ghost, @NTy.encode_ghost (Carriers.instantiate θ Θ)]
      simp only [NTy.ofSkolem]
      rw [NTy.encode_ofSkolem θ slot value]
  | .observed referent, value => NTy.encode_ofSkolem θ referent value
  | .nominal _ _ _, _ => rfl


theorem HList.encode_ofSkolem (θ : TypeArgs) : (row : NRow) →
    (values : @HList (Carriers.instantiate θ Θ) row) →
    HList.encode (HList.ofSkolem θ row values) = @HList.encode (Carriers.instantiate θ Θ) row values
  | .nil, _ => rfl
  | .cons τ rest, values => by
      show HList.encode (Γ := .cons (τ.subst θ.1) (NRow.subst θ.1 rest))
        (NTy.ofSkolem θ τ values.1, HList.ofSkolem θ rest values.2) = _
      rw [HList.encode_cons, @HList.encode_cons (Carriers.instantiate θ Θ)]
      simp only
      rw [NTy.encode_ofSkolem θ τ values.1, HList.encode_ofSkolem θ rest values.2]
end

/-- Equality at the family a call's type arguments induce, without handle
identity, is equality of the values read in the caller's view: both compare
encodings (`NTy.equal_codec`), which the transport keeps. -/
theorem NTy.equal_ofSkolem (θ : TypeArgs) (τ : NTy)
    (left right : @NTy.carrier (Carriers.instantiate θ Θ) τ) :
    @NTy.equal (Carriers.instantiate θ Θ) false τ left right =
      NTy.equal false (τ.subst θ.1) (NTy.ofSkolem θ τ left) (NTy.ofSkolem θ τ right) := by
  rw [@NTy.equal_codec (Carriers.instantiate θ Θ), NTy.equal_codec]
  simp only [NTy.codec_encode, NTy.encode_ofSkolem]
  rfl

mutual
theorem NTy.encode_toSkolem (θ : TypeArgs) : (τ : NTy) → (value : (τ.subst θ.1).carrier) →
    @NTy.encode (Carriers.instantiate θ Θ) τ (NTy.toSkolem θ τ value) =
      NTy.encode (τ.subst θ.1) value
  | .unit, _ => rfl
  | .bool, _ => rfl
  | .int _ _, _ => rfl
  | .num, _ => rfl
  | .address, _ => rfl
  | .signer, _ => rfl
  | .string, _ => rfl
  | .bytes, _ => rfl
  | .char, _ => rfl
  | .param _, _ => rfl
  | .function _ _ _, _ => rfl
  | .tuple elements, values => by
      show @NTy.encode (Carriers.instantiate θ Θ) (.tuple elements) (HList.toSkolem θ elements values) =
        NTy.encode (.tuple (NRow.subst θ.1 elements)) values
      rw [@NTy.encode_tuple (Carriers.instantiate θ Θ), NTy.encode_tuple,
        HList.encode_toSkolem θ elements values]
  | .struct source arguments fields, values => by
      rw [@NTy.encode_struct_val (Carriers.instantiate θ Θ), NTy.encode_struct_val]
      exact NTy.plainEncode_toSkolem θ (.struct source arguments fields) values.val
  | .enum source arguments names rows distinct, value => by
      rw [@NTy.encode_enum (Carriers.instantiate θ Θ), NTy.encode_enum]
      exact NTy.plainEncode_toSkolem θ (.enum source arguments names rows distinct) value.val
  | .slice element, vector => by
      show @NTy.encode (Carriers.instantiate θ Θ) (.slice element)
          (NTy.toSkolem θ (.slice element) vector) = NTy.encode (.slice (element.subst θ.1)) vector
      rw [@NTy.encode_slice (Carriers.instantiate θ Θ), NTy.encode_slice]
      simp only [NTy.toSkolem, Array.map_map]
      congr 1
      apply Array.map_congr_left
      intro x _
      exact NTy.encode_toSkolem θ element x

  | .vector element, vector => by
      show @NTy.encode (Carriers.instantiate θ Θ) (.vector element)
          (NTy.toSkolem θ (.vector element) vector) = NTy.encode (.vector (element.subst θ.1)) vector
      rw [@NTy.encode_vector (Carriers.instantiate θ Θ), NTy.encode_vector]
      simp only [NTy.toSkolem, Array.map_map]
      congr 1
      apply Array.map_congr_left
      intro x _
      exact NTy.encode_toSkolem θ element x
  | .array element length, array => by
      show @NTy.encode (Carriers.instantiate θ Θ) (.array element length)
          (NTy.toSkolem θ (.array element length) array) =
        NTy.encode (.array (element.subst θ.1) length) array
      rw [@NTy.encode_array (Carriers.instantiate θ Θ), NTy.encode_array]
      simp only [NTy.toSkolem, Array.map_map]
      congr 1
      apply Array.map_congr_left
      intro x _
      exact NTy.encode_toSkolem θ element x
  | .table key value, entries => by
      show @NTy.encode (Carriers.instantiate θ Θ) (.table key value)
          (NTy.toSkolem θ (.table key value) entries) =
        NTy.encode (.table (key.subst θ.1) (value.subst θ.1)) entries
      rw [@NTy.encode_table (Carriers.instantiate θ Θ), NTy.encode_table]
      simp only [NTy.toSkolem, List.map_map]
      congr 2
      apply List.map_congr_left
      intro entry _
      simp only [Function.comp_apply, NTy.encode_toSkolem θ key entry.1,
        NTy.encode_toSkolem θ value entry.2]
  | .sliceRef element, value => by
      simp only [NTy.encode, NTy.codec, Codec.sliceRef, Codec.boundedVector, NTy.toSkolem,
        SpecSliceRef.map_currentVector, SpecSliceRef.map_finalVector, SpecVector.map, Array.map_map]
      have same : (@NTy.codec (Carriers.instantiate θ Θ) element).encode ∘ NTy.toSkolem θ element =
          (element.subst θ.1).codec.encode :=
        funext (NTy.encode_toSkolem θ element)
      rw [same]
  | .ref referent, value => by
      show @NTy.encode (Carriers.instantiate θ Θ) (.ref referent)
          (NTy.toSkolem θ (.ref referent) value) = NTy.encode (.ref (referent.subst θ.1)) value
      rw [@NTy.encode_ref (Carriers.instantiate θ Θ), NTy.encode_ref]
      simp only [NTy.toSkolem]
      rw [NTy.plainEncode_toSkolem θ referent value.1, NTy.encode_toSkolem θ referent value.2]
  | .ghost slot, value => by
      show @NTy.encode (Carriers.instantiate θ Θ) (.ghost slot)
          (NTy.toSkolem θ (.ghost slot) value) = NTy.encode (.ghost (slot.subst θ.1)) value
      rw [@NTy.encode_ghost (Carriers.instantiate θ Θ), NTy.encode_ghost]
      simp only [NTy.toSkolem]
      rw [NTy.encode_toSkolem θ slot value]
  | .observed referent, value => NTy.encode_toSkolem θ referent value
  | .nominal _ _ _, _ => rfl


theorem HList.encode_toSkolem (θ : TypeArgs) : (row : NRow) →
    (values : HList (NRow.subst θ.1 row)) →
    @HList.encode (Carriers.instantiate θ Θ) row (HList.toSkolem θ row values) =
      HList.encode values
  | .nil, _ => rfl
  | .cons τ rest, values => by
      show @HList.encode (Carriers.instantiate θ Θ) (.cons τ rest)
          (NTy.toSkolem θ τ values.1, HList.toSkolem θ rest values.2) =
        HList.encode (Γ := .cons (τ.subst θ.1) (NRow.subst θ.1 rest)) values
      rw [@HList.encode_cons (Carriers.instantiate θ Θ), HList.encode_cons]
      simp only
      rw [NTy.encode_toSkolem θ τ values.1, HList.encode_toSkolem θ rest values.2]
end

/-- Forgetting a callee's certificates and reading the value in the caller's
view is the caller's view of the value with its certificates forgotten. -/
theorem NTy.forget_ofSkolem (θ : TypeArgs) (τ : NTy)
    (value : @NTy.carrier (Carriers.instantiate θ Θ) τ) :
    NTy.plainOfSkolem θ τ (@NTy.forget (Carriers.instantiate θ Θ) τ value) =
      @NTy.forget Θ (τ.subst θ.1) (NTy.ofSkolem θ τ value) := by
  apply NTy.plainEncode_injective (τ.subst θ.1)
  simp only [NTy.plainEncode_ofSkolem, NTy.plainEncode_forget, NTy.encode_ofSkolem]

/-- And the same in the callee's view. -/
theorem NTy.forget_toSkolem (θ : TypeArgs) (τ : NTy) (value : (τ.subst θ.1).carrier) :
    NTy.plainToSkolem θ τ (@NTy.forget Θ (τ.subst θ.1) value) =
      @NTy.forget (Carriers.instantiate θ Θ) τ (NTy.toSkolem θ τ value) := by
  apply @NTy.plainEncode_injective (Carriers.instantiate θ Θ) τ
  simp only [NTy.plainEncode_toSkolem, NTy.plainEncode_forget, NTy.encode_toSkolem]

attribute [vir_denote, vir_denote_norm] Array.map_id' Array.map_id

/-! A caller's value in the callee's view, as far as the callee looks into
it: a scalar is itself, and a row, a vector, and a variant are transported
lazily, component by component as they are projected, so that the encoding
of a transported value meets the caller's encoding (`NTy.encode_toSkolem`)
rather than a rebuilt copy of it. -/

section LazyTransport
variable (θ : TypeArgs)

@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_unit (value : (NTy.unit.subst θ.1).carrier) :
    NTy.toSkolem θ .unit value = value := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_bool (value : (NTy.bool.subst θ.1).carrier) :
    NTy.toSkolem θ .bool value = value := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_int (width : Nat) (signed : Bool)
    (value : ((NTy.int width signed).subst θ.1).carrier) :
    NTy.toSkolem θ (.int width signed) value = value := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_num (value : (NTy.num.subst θ.1).carrier) :
    NTy.toSkolem θ .num value = value := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_address
    (value : (NTy.address.subst θ.1).carrier) : NTy.toSkolem θ .address value = value := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_signer
    (value : (NTy.signer.subst θ.1).carrier) : NTy.toSkolem θ .signer value = value := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_string
    (value : (NTy.string.subst θ.1).carrier) : NTy.toSkolem θ .string value = value := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_bytes
    (value : (NTy.bytes.subst θ.1).carrier) : NTy.toSkolem θ .bytes value = value := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_param (index : Nat)
    (value : ((NTy.param index).subst θ.1).carrier) :
    NTy.toSkolem θ (.param index) value = value := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_tuple (elements : NRow)
    (values : ((NTy.tuple elements).subst θ.1).carrier) :
    NTy.toSkolem θ (.tuple elements) values = HList.toSkolem θ elements values := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_struct (source : StructHandle) (fields : NRow)
    (values : ((NTy.struct source arguments fields).subst θ.1).carrier) :
    (NTy.toSkolem θ (.struct source arguments fields) values).val =
      HListPlain.toSkolem θ fields values.val := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_enum (source : StructHandle)
    (names : List String) (rows : NRows) (distinct : names.Nodup)
    (value : ((NTy.enum source arguments names rows distinct).subst θ.1).carrier) :
    (NTy.toSkolem θ (.enum source arguments names rows distinct) value).val =
      variantPlain.toSkolem θ names rows value.val := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_ref (referent : NTy)
    (value : ((NTy.ref referent).subst θ.1).carrier) :
    NTy.toSkolem θ (.ref referent) value =
      (NTy.plainToSkolem θ referent value.1, NTy.toSkolem θ referent value.2) := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_ghost (slot : NTy)
    (value : ((NTy.ghost slot).subst θ.1).carrier) :
    NTy.toSkolem θ (.ghost slot) value = NTy.toSkolem θ slot value := rfl
@[vir_denote, vir_denote_norm] theorem NTy.toSkolem_vector_values (element : NTy)
    (vector : ((NTy.vector element).subst θ.1).carrier) :
    (NTy.toSkolem θ (.vector element) vector).values =
      vector.values.map (NTy.toSkolem θ element) := rfl
@[vir_denote, vir_denote_norm] theorem HList.toSkolem_nil (values : HList (NRow.subst θ.1 .nil)) :
    HList.toSkolem θ .nil values = () := rfl
@[vir_denote, vir_denote_norm] theorem HList.toSkolem_fst (τ : NTy) (rest : NRow)
    (values : HList (NRow.subst θ.1 (.cons τ rest))) :
    (HList.toSkolem θ (.cons τ rest) values).1 = NTy.toSkolem θ τ values.1 := rfl
@[vir_denote, vir_denote_norm] theorem HList.toSkolem_snd (τ : NTy) (rest : NRow)
    (values : HList (NRow.subst θ.1 (.cons τ rest))) :
    (HList.toSkolem θ (.cons τ rest) values).2 = HList.toSkolem θ rest values.2 := rfl
@[vir_denote, vir_denote_norm] theorem HList.toSkolem_mk (τ : NTy) (rest : NRow)
    (head : (τ.subst θ.1).carrier) (tail : HList (NRow.subst θ.1 rest)) :
    HList.toSkolem θ (.cons τ rest) ((head, tail) : HList (NRow.subst θ.1 (.cons τ rest))) =
      (NTy.toSkolem θ τ head, HList.toSkolem θ rest tail) := rfl
@[vir_denote, vir_denote_norm] theorem variantCarrier.toSkolem_inl (name : String)
    (names : List String) (fields : NRow) (rest : NRows) (values : HList (NRow.subst θ.1 fields)) :
    variantCarrier.toSkolem θ (name :: names) (.cons fields rest) (.inl values) =
      .inl (HList.toSkolem θ fields values) := rfl
@[vir_denote, vir_denote_norm] theorem variantCarrier.toSkolem_inr (name : String)
    (names : List String) (fields : NRow) (rest : NRows)
    (value : variantCarrier names (NRows.subst θ.1 rest)) :
    variantCarrier.toSkolem θ (name :: names) (.cons fields rest) (.inr value) =
      .inr (variantCarrier.toSkolem θ names rest value) := rfl

end LazyTransport

/-! ## Transports are identities

A value's carrier at the family a call's type arguments induce is its
carrier at the caller's family, at the substituted type; the transports
between them carry a value to itself. At a concrete type the two carriers
unfold to the same type, so a transport of a value is the value
(`ofSkolemIdentity`). -/

section TransportIdentity
variable (θ : TypeArgs)

private theorem prod_type_congr {A A' B B' : Type} (first : A = A') (second : B = B') :
    (A × B) = (A' × B') := by
  subst first; subst second; rfl

private theorem sum_type_congr {A A' B B' : Type} (first : A = A') (second : B = B') :
    (A ⊕ B) = (A' ⊕ B') := by
  subst first; subst second; rfl

private theorem prod_mk_heq {A A' B B' : Type} (ha : A = A') (hb : B = B') (a : A) (b : B)
    (p : A' × B') (first : HEq a p.1) (second : HEq b p.2) : HEq (a, b) p := by
  subst ha; subst hb
  obtain ⟨x, y⟩ := p
  rw [eq_of_heq first, eq_of_heq second]

private theorem sum_inl_heq {A A' B B' : Type} (ha : A = A') (hb : B = B') (a : A) (a' : A')
    (same : HEq a a') : HEq (Sum.inl a : A ⊕ B) (Sum.inl a' : A' ⊕ B') := by
  subst ha; subst hb
  rw [eq_of_heq same]

private theorem sum_inr_heq {A A' B B' : Type} (ha : A = A') (hb : B = B') (b : B) (b' : B')
    (same : HEq b b') : HEq (Sum.inr b : A ⊕ B) (Sum.inr b' : A' ⊕ B') := by
  subst ha; subst hb
  rw [eq_of_heq same]

private theorem specArray_map_heq {A B : Type} {length : Nat} (h : A = B) (f : A → B)
    (array : SpecArray A length) (bounded : (array.vector.values.map f).size < 2 ^ 64)
    (sized : (⟨array.vector.values.map f, bounded⟩ : SpecVector B).values.size = length)
    (same : ∀ x, HEq (f x) x) :
    HEq (⟨⟨array.vector.values.map f, bounded⟩, sized⟩ : SpecArray B length) array := by
  subst h
  have identity : f = id := funext fun x => eq_of_heq (same x)
  obtain ⟨⟨values, _⟩, _⟩ := array
  simp only [identity, Array.map_id]
  exact HEq.rfl

private theorem specVector_map_heq {A B : Type} (h : A = B) (f : A → B) (vector : SpecVector A)
    (bounded : (vector.values.map f).size < 2 ^ 64) (same : ∀ x, HEq (f x) x) :
    HEq (⟨vector.values.map f, bounded⟩ : SpecVector B) vector := by
  subst h
  have identity : f = id := funext fun x => eq_of_heq (same x)
  obtain ⟨values, _⟩ := vector
  simp only [identity, Array.map_id]
  exact HEq.rfl

private theorem specEntries_map_heq {K K' V V' : Type} (hk : K = K') (hv : V = V')
    (fk : K → K') (fv : V → V') (table : SpecEntries (K × V))
    (bounded : (table.entries.map fun entry => (fk entry.1, fv entry.2)).length < 2 ^ 64)
    (sameKeys : ∀ k, HEq (fk k) k) (sameValues : ∀ v, HEq (fv v) v) :
    HEq (⟨table.entries.map fun entry => (fk entry.1, fv entry.2), bounded⟩ : SpecEntries (K' × V'))
      table := by
  subst hk; subst hv
  have identity : (fun entry : K × V => (fk entry.1, fv entry.2)) = id :=
    funext fun entry => by simp only [eq_of_heq (sameKeys entry.1), eq_of_heq (sameValues entry.2), id]
  obtain ⟨entries, _⟩ := table
  simp only [identity, List.map_id]
  exact HEq.rfl

private theorem specSliceRef_map_heq {A B : Type} (h : A = B) (f : A → B)
    (value : SpecSliceRef A) (same : ∀ x, HEq (f x) x) : HEq (value.map f) value := by
  subst h
  have identity : f = id := funext fun x => eq_of_heq (same x)
  rw [identity]
  exact heq_of_eq (SpecSliceRef.map_id value)


/-- Two certified carriers are the same type where their plain carriers are
and their certificates agree, the agreement read through a map that carries
a value to itself. -/
private theorem subtype_congr {A B : Type} (same : A = B) (f : A → B)
    (identity : ∀ value, HEq (f value) value) {P : A → Prop} {Q : B → Prop}
    (agree : ∀ value, P value ↔ Q (f value)) :
    { value : A // P value } = { value : B // Q value } := by
  subst same
  have carried : ∀ value, f value = value := fun value => eq_of_heq (identity value)
  have predicates : P = Q :=
    funext fun value => propext ((agree value).trans (by rw [carried value]))
  subst predicates
  rfl

/-- Two certified values of such carriers are the same value where their
plain values are. -/
private theorem subtype_heq {A B : Type} (same : A = B) (f : A → B)
    (identity : ∀ value, HEq (f value) value) {P : A → Prop} {Q : B → Prop}
    (agree : ∀ value, P value ↔ Q (f value))
    (left : { value : A // P value }) (right : { value : B // Q value })
    (values : HEq left.val right.val) : HEq left right := by
  subst same
  have carried : ∀ value, f value = value := fun value => eq_of_heq (identity value)
  have predicates : P = Q :=
    funext fun value => propext ((agree value).trans (by rw [carried value]))
  subst predicates
  exact heq_of_eq (Subtype.ext (eq_of_heq values))

mutual
/-- The lent carrier at the induced family is the lent carrier at the
substituted type: a type parameter's lent values are the argument's
(`Carriers.lent`). -/
theorem NTy.plain_instantiate : (τ : NTy) →
    @NTy.plain (Carriers.instantiate θ Θ) τ = @NTy.plain Θ (τ.subst θ.1)
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char | .param _
  | .function _ _ _ | .nominal _ _ _ => rfl
  | .tuple elements => HListPlain.plain_instantiate elements
  | .struct _ _ fields => HListPlain.plain_instantiate fields
  | .enum _ _ names rows _ => variantPlain.plain_instantiate names rows
  | .slice element => congrArg SpecVector (NTy.plain_instantiate element)
  | .vector element => congrArg SpecVector (NTy.plain_instantiate element)
  | .array element length =>
      congrArg (fun T => SpecArray T length) (NTy.plain_instantiate element)
  | .table key value =>
      congrArg SpecEntries
        (prod_type_congr (NTy.plain_instantiate key) (NTy.plain_instantiate value))
  | .ghost slot => NTy.plain_instantiate slot
  | .sliceRef element => congrArg SpecSliceRef (NTy.plain_instantiate element)
  | .observed referent => NTy.plain_instantiate referent
  | .ref referent =>
      prod_type_congr (NTy.plain_instantiate referent) (NTy.plain_instantiate referent)
theorem HListPlain.plain_instantiate : (row : NRow) →
    @HListPlain (Carriers.instantiate θ Θ) row = @HListPlain Θ (NRow.subst θ.1 row)
  | .nil => rfl
  | .cons τ rest =>
      prod_type_congr (NTy.plain_instantiate τ) (HListPlain.plain_instantiate rest)
theorem variantPlain.plain_instantiate : (names : List String) → (rows : NRows) →
    @variantPlain (Carriers.instantiate θ Θ) names rows =
      @variantPlain Θ names (NRows.subst θ.1 rows)
  | _, .nil => rfl
  | [], .cons _ _ => rfl
  | _ :: names, .cons fields rest =>
      sum_type_congr (HListPlain.plain_instantiate fields)
        (variantPlain.plain_instantiate names rest)
end

mutual
/-- A callee's lent value in the caller's view is the value. -/
theorem NTy.plainOfSkolem_heq : (τ : NTy) → (value : @NTy.plain (Carriers.instantiate θ Θ) τ) →
    HEq (NTy.plainOfSkolem θ τ value) value
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _
  | .bytes, _ | .char, _ | .param _, _ | .function _ _ _, _ | .nominal _ _ _, _ => HEq.rfl
  | .tuple elements, values => HListPlain.ofSkolem_heq elements values
  | .struct _ _ fields, values => HListPlain.ofSkolem_heq fields values
  | .enum _ _ names rows _, value => variantPlain.ofSkolem_heq names rows value
  | .slice element, vector =>
      specVector_map_heq (NTy.plain_instantiate θ element) _ vector _
        (NTy.plainOfSkolem_heq element)
  | .vector element, vector =>
      specVector_map_heq (NTy.plain_instantiate θ element) _ vector _
        (NTy.plainOfSkolem_heq element)
  | .array element _, array =>
      specArray_map_heq (NTy.plain_instantiate θ element) _ array _ _
        (NTy.plainOfSkolem_heq element)
  | .table key value, table =>
      specEntries_map_heq (NTy.plain_instantiate θ key) (NTy.plain_instantiate θ value) _ _
        table _ (NTy.plainOfSkolem_heq key) (NTy.plainOfSkolem_heq value)
  | .ghost slot, value => NTy.plainOfSkolem_heq slot value
  | .observed referent, value => NTy.plainOfSkolem_heq referent value
  | .sliceRef element, value =>
      specSliceRef_map_heq (NTy.plain_instantiate θ element) _ value
        (NTy.plainOfSkolem_heq element)
  | .ref referent, value =>
      prod_mk_heq (NTy.plain_instantiate θ referent).symm (NTy.plain_instantiate θ referent).symm
        _ _ value (NTy.plainOfSkolem_heq referent value.1)
        (NTy.plainOfSkolem_heq referent value.2)
theorem HListPlain.ofSkolem_heq : (row : NRow) →
    (values : @HListPlain (Carriers.instantiate θ Θ) row) →
    HEq (HListPlain.ofSkolem θ row values) values
  | .nil, values => by cases values; exact HEq.rfl
  | .cons τ rest, values =>
      prod_mk_heq (NTy.plain_instantiate θ τ).symm (HListPlain.plain_instantiate θ rest).symm _ _
        values (NTy.plainOfSkolem_heq τ values.1) (HListPlain.ofSkolem_heq rest values.2)
theorem variantPlain.ofSkolem_heq : (names : List String) → (rows : NRows) →
    (value : @variantPlain (Carriers.instantiate θ Θ) names rows) →
    HEq (variantPlain.ofSkolem θ names rows value) value
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: names, .cons fields rest, .inl values =>
      sum_inl_heq (HListPlain.plain_instantiate θ fields).symm
        (variantPlain.plain_instantiate θ names rest).symm _ _
        (HListPlain.ofSkolem_heq fields values)
  | _ :: names, .cons fields rest, .inr value =>
      sum_inr_heq (HListPlain.plain_instantiate θ fields).symm
        (variantPlain.plain_instantiate θ names rest).symm _ _
        (variantPlain.ofSkolem_heq names rest value)
end

mutual
/-- A caller's lent value in the callee's view is the value. -/
theorem NTy.plainToSkolem_heq : (τ : NTy) → (value : (τ.subst θ.1).plain) →
    HEq (NTy.plainToSkolem θ τ value) value
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _
  | .bytes, _ | .char, _ | .param _, _ | .function _ _ _, _ | .nominal _ _ _, _ => HEq.rfl
  | .tuple elements, values => HListPlain.toSkolem_heq elements values
  | .struct _ _ fields, values => HListPlain.toSkolem_heq fields values
  | .enum _ _ names rows _, value => variantPlain.toSkolem_heq names rows value
  | .slice element, vector =>
      specVector_map_heq (NTy.plain_instantiate θ element).symm _ vector _
        (NTy.plainToSkolem_heq element)
  | .vector element, vector =>
      specVector_map_heq (NTy.plain_instantiate θ element).symm _ vector _
        (NTy.plainToSkolem_heq element)
  | .array element _, array =>
      specArray_map_heq (NTy.plain_instantiate θ element).symm _ array _ _
        (NTy.plainToSkolem_heq element)
  | .table key value, table =>
      specEntries_map_heq (NTy.plain_instantiate θ key).symm
        (NTy.plain_instantiate θ value).symm _ _ table _
        (NTy.plainToSkolem_heq key) (NTy.plainToSkolem_heq value)
  | .ghost slot, value => NTy.plainToSkolem_heq slot value
  | .observed referent, value => NTy.plainToSkolem_heq referent value
  | .sliceRef element, value =>
      specSliceRef_map_heq (NTy.plain_instantiate θ element).symm _ value
        (NTy.plainToSkolem_heq element)
  | .ref referent, value =>
      prod_mk_heq (NTy.plain_instantiate θ referent) (NTy.plain_instantiate θ referent) _ _ value
        (NTy.plainToSkolem_heq referent value.1) (NTy.plainToSkolem_heq referent value.2)
theorem HListPlain.toSkolem_heq : (row : NRow) → (values : HListPlain (NRow.subst θ.1 row)) →
    HEq (HListPlain.toSkolem θ row values) values
  | .nil, values => by cases values; exact HEq.rfl
  | .cons τ rest, values =>
      prod_mk_heq (NTy.plain_instantiate θ τ) (HListPlain.plain_instantiate θ rest) _ _
        values (NTy.plainToSkolem_heq τ values.1) (HListPlain.toSkolem_heq rest values.2)
theorem variantPlain.toSkolem_heq : (names : List String) → (rows : NRows) →
    (value : variantPlain names (NRows.subst θ.1 rows)) →
    HEq (variantPlain.toSkolem θ names rows value) value
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: names, .cons fields rest, .inl values =>
      sum_inl_heq (HListPlain.plain_instantiate θ fields)
        (variantPlain.plain_instantiate θ names rest) _ _ (HListPlain.toSkolem_heq fields values)
  | _ :: names, .cons fields rest, .inr value =>
      sum_inr_heq (HListPlain.plain_instantiate θ fields)
        (variantPlain.plain_instantiate θ names rest) _ _
        (variantPlain.toSkolem_heq names rest value)
end

mutual
/-- The carrier at the induced family is the carrier at the substituted
type: at a struct or a variant the plain carriers agree and so do the
certificates (`NTy.deepHolds_ofSkolem`). -/
theorem NTy.carrier_instantiate : (τ : NTy) →
    @NTy.carrier (Carriers.instantiate θ Θ) τ = @NTy.carrier Θ (τ.subst θ.1)
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char | .param _
  | .function _ _ _ | .nominal _ _ _ => rfl
  | .tuple elements => HList.carrier_instantiate elements
  | .struct source arguments fields =>
      subtype_congr (HListPlain.plain_instantiate θ fields) (HListPlain.ofSkolem θ fields)
        (HListPlain.ofSkolem_heq θ fields)
        (fun row => (NTy.deepHolds_ofSkolem θ (.struct source arguments fields) row).symm)
  | .enum source arguments names rows distinct =>
      subtype_congr (variantPlain.plain_instantiate θ names rows)
        (variantPlain.ofSkolem θ names rows) (variantPlain.ofSkolem_heq θ names rows)
        (fun value =>
          (NTy.deepHolds_ofSkolem θ (.enum source arguments names rows distinct) value).symm)
  | .slice element => by
      show SpecVector _ = SpecVector _
      rw [NTy.carrier_instantiate element]
  | .vector element => by
      show SpecVector _ = SpecVector _
      rw [NTy.carrier_instantiate element]
  | .array element _ => by
      show SpecArray _ _ = SpecArray _ _
      rw [NTy.carrier_instantiate element]
  | .table key value => by
      show SpecEntries (_ × _) = SpecEntries (_ × _)
      rw [NTy.carrier_instantiate key, NTy.carrier_instantiate value]
  | .ghost slot => NTy.carrier_instantiate slot
  | .sliceRef element => by
      show SpecSliceRef _ = SpecSliceRef _
      rw [NTy.carrier_instantiate element]
  | .observed referent => NTy.carrier_instantiate referent
  | .ref referent => by
      show (_ × _) = (_ × _)
      rw [NTy.plain_instantiate θ referent, NTy.carrier_instantiate referent]
theorem HList.carrier_instantiate : (row : NRow) →
    @HList (Carriers.instantiate θ Θ) row = @HList Θ (NRow.subst θ.1 row)
  | .nil => rfl
  | .cons τ rest => by
      show (_ × _) = (_ × _)
      rw [NTy.carrier_instantiate τ, HList.carrier_instantiate rest]
theorem variantCarrier.carrier_instantiate : (names : List String) → (rows : NRows) →
    @variantCarrier (Carriers.instantiate θ Θ) names rows =
      @variantCarrier Θ names (NRows.subst θ.1 rows)
  | _, .nil => rfl
  | [], .cons _ _ => rfl
  | _ :: names, .cons fields rest => by
      show (_ ⊕ _) = (_ ⊕ _)
      rw [HList.carrier_instantiate fields, variantCarrier.carrier_instantiate names rest]
end

mutual
/-- A callee's value in the caller's view is the value. -/
theorem NTy.ofSkolem_heq : (τ : NTy) → (value : @NTy.carrier (Carriers.instantiate θ Θ) τ) →
    HEq (NTy.ofSkolem θ τ value) value
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _
  | .bytes, _ | .char, _ | .param _, _ | .function _ _ _, _ | .nominal _ _ _, _ => HEq.rfl
  | .tuple elements, values => HList.ofSkolem_heq elements values
  | .struct source arguments fields, value =>
      (subtype_heq (HListPlain.plain_instantiate θ fields) (HListPlain.ofSkolem θ fields)
        (HListPlain.ofSkolem_heq θ fields)
        (fun row => (NTy.deepHolds_ofSkolem θ (.struct source arguments fields) row).symm)
        value (NTy.ofSkolem θ (.struct source arguments fields) value)
        (HListPlain.ofSkolem_heq θ fields value.val).symm).symm
  | .enum source arguments names rows distinct, value =>
      (subtype_heq (variantPlain.plain_instantiate θ names rows)
        (variantPlain.ofSkolem θ names rows) (variantPlain.ofSkolem_heq θ names rows)
        (fun inner =>
          (NTy.deepHolds_ofSkolem θ (.enum source arguments names rows distinct) inner).symm)
        value (NTy.ofSkolem θ (.enum source arguments names rows distinct) value)
        (variantPlain.ofSkolem_heq θ names rows value.val).symm).symm
  | .slice element, vector =>
      specVector_map_heq (NTy.carrier_instantiate θ element) _ vector _
        (NTy.ofSkolem_heq element)

  | .vector element, vector =>
      specVector_map_heq (NTy.carrier_instantiate θ element) _ vector _
        (NTy.ofSkolem_heq element)
  | .array element _, array =>
      specArray_map_heq (NTy.carrier_instantiate θ element) _ array _ _
        (NTy.ofSkolem_heq element)
  | .table key value, table =>
      specEntries_map_heq (NTy.carrier_instantiate θ key) (NTy.carrier_instantiate θ value) _ _
        table _ (NTy.ofSkolem_heq key) (NTy.ofSkolem_heq value)
  | .ghost slot, value => NTy.ofSkolem_heq slot value
  | .observed referent, value => NTy.ofSkolem_heq referent value
  | .sliceRef element, value =>
      specSliceRef_map_heq (NTy.carrier_instantiate θ element) _ value (NTy.ofSkolem_heq element)
  | .ref referent, value =>
      prod_mk_heq (NTy.plain_instantiate θ referent).symm
        (NTy.carrier_instantiate θ referent).symm _ _ value
        (NTy.plainOfSkolem_heq θ referent value.1) (NTy.ofSkolem_heq referent value.2)
theorem HList.ofSkolem_heq : (row : NRow) → (values : @HList (Carriers.instantiate θ Θ) row) →
    HEq (HList.ofSkolem θ row values) values
  | .nil, values => by cases values; exact HEq.rfl
  | .cons τ rest, values =>
      prod_mk_heq (NTy.carrier_instantiate θ τ).symm (HList.carrier_instantiate θ rest).symm _ _
        values (NTy.ofSkolem_heq τ values.1) (HList.ofSkolem_heq rest values.2)
theorem variantCarrier.ofSkolem_heq : (names : List String) → (rows : NRows) →
    (value : @variantCarrier (Carriers.instantiate θ Θ) names rows) →
    HEq (variantCarrier.ofSkolem θ names rows value) value
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: names, .cons fields rest, .inl values =>
      sum_inl_heq (HList.carrier_instantiate θ fields).symm
        (variantCarrier.carrier_instantiate θ names rest).symm _ _ (HList.ofSkolem_heq fields values)
  | _ :: names, .cons fields rest, .inr value =>
      sum_inr_heq (HList.carrier_instantiate θ fields).symm
        (variantCarrier.carrier_instantiate θ names rest).symm _ _
        (variantCarrier.ofSkolem_heq names rest value)
end

mutual
/-- A caller's value in the callee's view is the value. -/
theorem NTy.toSkolem_heq : (τ : NTy) → (value : (τ.subst θ.1).carrier) →
    HEq (NTy.toSkolem θ τ value) value
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _
  | .bytes, _ | .char, _ | .param _, _ | .function _ _ _, _ | .nominal _ _ _, _ => HEq.rfl
  | .tuple elements, values => HList.toSkolem_heq elements values
  | .struct source arguments fields, value =>
      subtype_heq (HListPlain.plain_instantiate θ fields) (HListPlain.ofSkolem θ fields)
        (HListPlain.ofSkolem_heq θ fields)
        (fun row => (NTy.deepHolds_ofSkolem θ (.struct source arguments fields) row).symm)
        (NTy.toSkolem θ (.struct source arguments fields) value) value
        (HListPlain.toSkolem_heq θ fields value.val)
  | .enum source arguments names rows distinct, value =>
      subtype_heq (variantPlain.plain_instantiate θ names rows)
        (variantPlain.ofSkolem θ names rows) (variantPlain.ofSkolem_heq θ names rows)
        (fun inner =>
          (NTy.deepHolds_ofSkolem θ (.enum source arguments names rows distinct) inner).symm)
        (NTy.toSkolem θ (.enum source arguments names rows distinct) value) value
        (variantPlain.toSkolem_heq θ names rows value.val)
  | .slice element, vector =>
      specVector_map_heq (NTy.carrier_instantiate θ element).symm _ vector _
        (NTy.toSkolem_heq element)

  | .vector element, vector =>
      specVector_map_heq (NTy.carrier_instantiate θ element).symm _ vector _
        (NTy.toSkolem_heq element)
  | .array element _, array =>
      specArray_map_heq (NTy.carrier_instantiate θ element).symm _ array _ _
        (NTy.toSkolem_heq element)
  | .table key value, table =>
      specEntries_map_heq (NTy.carrier_instantiate θ key).symm
        (NTy.carrier_instantiate θ value).symm _ _ table _
        (NTy.toSkolem_heq key) (NTy.toSkolem_heq value)
  | .ghost slot, value => NTy.toSkolem_heq slot value
  | .observed referent, value => NTy.toSkolem_heq referent value
  | .sliceRef element, value =>
      specSliceRef_map_heq (NTy.carrier_instantiate θ element).symm _ value (NTy.toSkolem_heq element)
  | .ref referent, value =>
      prod_mk_heq (NTy.plain_instantiate θ referent) (NTy.carrier_instantiate θ referent) _ _ value
        (NTy.plainToSkolem_heq θ referent value.1) (NTy.toSkolem_heq referent value.2)
theorem HList.toSkolem_heq : (row : NRow) → (values : HList (NRow.subst θ.1 row)) →
    HEq (HList.toSkolem θ row values) values
  | .nil, values => by cases values; exact HEq.rfl
  | .cons τ rest, values =>
      prod_mk_heq (NTy.carrier_instantiate θ τ) (HList.carrier_instantiate θ rest) _ _
        values (NTy.toSkolem_heq τ values.1) (HList.toSkolem_heq rest values.2)
theorem variantCarrier.toSkolem_heq : (names : List String) → (rows : NRows) →
    (value : variantCarrier names (NRows.subst θ.1 rows)) →
    HEq (variantCarrier.toSkolem θ names rows value) value
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: names, .cons fields rest, .inl values =>
      sum_inl_heq (HList.carrier_instantiate θ fields)
        (variantCarrier.carrier_instantiate θ names rest) _ _
        (HList.toSkolem_heq fields values)
  | _ :: names, .cons fields rest, .inr value =>
      sum_inr_heq (HList.carrier_instantiate θ fields)
        (variantCarrier.carrier_instantiate θ names rest) _ _
        (variantCarrier.toSkolem_heq names rest value)
end

/-- The inhabitant of a type at the induced family is the caller's
inhabitant of the substituted type, in the callee's view
(`CarriersNonempty.instantiate`). -/
theorem NTy.inhabitant_instantiate (τ : NTy) (inhabitable : τ.inhabitable = true) :
    @NTy.inhabitant (Carriers.instantiate θ Θ) (CarriersNonempty.instantiate θ Θ) τ inhabitable =
      NTy.toSkolem θ τ (NTy.inhabitant (τ.subst θ.1) (NTy.inhabitable_subst θ.1 θ.inhabitable τ inhabitable)) :=
  rfl

/-- The default of a type at the induced family is the default of the
substituted type, in the callee's view (`NTy.inhabitant_instantiate`). -/
theorem NTy.inhabitantOf_instantiate (τ : NTy) (inhabitable : τ.inhabitable = true) :
    @NTy.inhabitantOf (Carriers.instantiate θ Θ) (CarriersNonempty.instantiate θ Θ) τ inhabitable =
      NTy.toSkolem θ τ
        (NTy.inhabitantOf (τ.subst θ.1) (NTy.inhabitable_subst θ.1 θ.inhabitable τ inhabitable)) := by
  unfold NTy.inhabitantOf
  exact NTy.inhabitant_instantiate θ τ inhabitable

end TransportIdentity

mutual
theorem NTy.ofSkolem_toSkolem (θ : TypeArgs) : (τ : NTy) → (value : (τ.subst θ.1).carrier) →
    NTy.ofSkolem θ τ (NTy.toSkolem θ τ value) = value
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _ | .bytes, _ | .char, _
  | .param _, _ | .function _ _ _, _ | .nominal _ _ _, _ => rfl
  | .tuple elements, values => HList.ofSkolem_toSkolem θ elements values
  | .struct _ _ fields, values => Subtype.ext (HListPlain.ofSkolem_toSkolem θ fields values.val)
  | .enum _ _ names rows _, value =>
      Subtype.ext (variantPlain.ofSkolem_toSkolem θ names rows value.val)
  | .slice element, vector => by
      apply SpecVector.ext
      simp only [NTy.toSkolem, NTy.ofSkolem, Array.map_map]
      conv => rhs; rw [← Array.map_id vector.values]
      apply Array.map_congr_left
      intro value _
      exact NTy.ofSkolem_toSkolem θ element value

  | .vector element, vector => by
      apply SpecVector.ext
      simp only [NTy.toSkolem, NTy.ofSkolem, Array.map_map]
      conv => rhs; rw [← Array.map_id vector.values]
      apply Array.map_congr_left
      intro value _
      exact NTy.ofSkolem_toSkolem θ element value
  | .array element _, array => by
      apply SpecArray.ext
      apply SpecVector.ext
      simp only [NTy.toSkolem, NTy.ofSkolem, Array.map_map]
      conv => rhs; rw [← Array.map_id array.vector.values]
      apply Array.map_congr_left
      intro value _
      exact NTy.ofSkolem_toSkolem θ element value
  | .table key value, entries => by
      apply SpecEntries.ext
      simp only [NTy.toSkolem, NTy.ofSkolem, List.map_map]
      conv => rhs; rw [← List.map_id entries.entries]
      apply List.map_congr_left
      intro entry _
      simp only [Function.comp_apply, NTy.ofSkolem_toSkolem θ key, NTy.ofSkolem_toSkolem θ value,
        id_eq]
  | .sliceRef element, value => by
      simp only [NTy.toSkolem, NTy.ofSkolem, SpecSliceRef.map_map]
      have same : (fun x => NTy.ofSkolem θ element
          (NTy.toSkolem θ element x)) = id :=
        funext (NTy.ofSkolem_toSkolem θ element)
      rw [show _ ∘ _ = id from same, SpecSliceRef.map_id]
  | .ref referent, value => by
      simp only [NTy.toSkolem, NTy.ofSkolem, NTy.plainOfSkolem_toSkolem θ referent, NTy.ofSkolem_toSkolem θ referent]
  | .ghost slot, value => NTy.ofSkolem_toSkolem θ slot value
  | .observed referent, value => NTy.ofSkolem_toSkolem θ referent value

theorem HList.ofSkolem_toSkolem (θ : TypeArgs) : (row : NRow) →
    (values : HList (NRow.subst θ.1 row)) → HList.ofSkolem θ row (HList.toSkolem θ row values) = values
  | .nil, () => rfl
  | .cons τ rest, values => by
      simp only [HList.toSkolem, HList.ofSkolem, NTy.ofSkolem_toSkolem θ τ, HList.ofSkolem_toSkolem θ rest]
      rfl

theorem variantCarrier.ofSkolem_toSkolem (θ : TypeArgs) : (names : List String) →
    (rows : NRows) → (value : variantCarrier names (NRows.subst θ.1 rows)) →
    variantCarrier.ofSkolem θ names rows (variantCarrier.toSkolem θ names rows value) = value
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl values => by
      simp only [variantCarrier.toSkolem, variantCarrier.ofSkolem, HList.ofSkolem_toSkolem θ fields]
  | _ :: names, .cons _ rest, .inr value => by
      simp only [variantCarrier.toSkolem, variantCarrier.ofSkolem, variantCarrier.ofSkolem_toSkolem θ names rest]
end

mutual
theorem NTy.toSkolem_ofSkolem (θ : TypeArgs) : (τ : NTy) →
    (value : @NTy.carrier (Carriers.instantiate θ Θ) τ) →
    NTy.toSkolem θ τ (NTy.ofSkolem θ τ value) = value
  | .unit, _ | .bool, _ | .int _ _, _ | .num, _ | .address, _ | .signer, _ | .string, _ | .bytes, _ | .char, _
  | .param _, _ | .function _ _ _, _ | .nominal _ _ _, _ => rfl
  | .tuple elements, values => HList.toSkolem_ofSkolem θ elements values
  | .struct _ _ fields, values => Subtype.ext (HListPlain.toSkolem_ofSkolem θ fields values.val)
  | .enum _ _ names rows _, value =>
      Subtype.ext (variantPlain.toSkolem_ofSkolem θ names rows value.val)
  | .slice element, vector => by
      apply SpecVector.ext
      simp only [NTy.toSkolem, NTy.ofSkolem, Array.map_map]
      conv => rhs; rw [← Array.map_id vector.values]
      apply Array.map_congr_left
      intro value _
      exact NTy.toSkolem_ofSkolem θ element value

  | .vector element, vector => by
      apply SpecVector.ext
      simp only [NTy.toSkolem, NTy.ofSkolem, Array.map_map]
      conv => rhs; rw [← Array.map_id vector.values]
      apply Array.map_congr_left
      intro value _
      exact NTy.toSkolem_ofSkolem θ element value
  | .array element _, array => by
      apply SpecArray.ext
      apply SpecVector.ext
      simp only [NTy.toSkolem, NTy.ofSkolem, Array.map_map]
      conv => rhs; rw [← Array.map_id array.vector.values]
      apply Array.map_congr_left
      intro value _
      exact NTy.toSkolem_ofSkolem θ element value
  | .table key value, entries => by
      apply SpecEntries.ext
      simp only [NTy.toSkolem, NTy.ofSkolem, List.map_map]
      conv => rhs; rw [← List.map_id entries.entries]
      apply List.map_congr_left
      intro entry _
      simp only [Function.comp_apply, NTy.toSkolem_ofSkolem θ key, NTy.toSkolem_ofSkolem θ value,
        id_eq]
  | .sliceRef element, value => by
      simp only [NTy.toSkolem, NTy.ofSkolem, SpecSliceRef.map_map]
      have same : (fun x => NTy.toSkolem θ element
          (NTy.ofSkolem θ element x)) = id :=
        funext (NTy.toSkolem_ofSkolem θ element)
      rw [show _ ∘ _ = id from same, SpecSliceRef.map_id]
  | .ref referent, value => by
      simp only [NTy.toSkolem, NTy.ofSkolem, NTy.plainToSkolem_ofSkolem θ referent, NTy.toSkolem_ofSkolem θ referent]
  | .ghost slot, value => NTy.toSkolem_ofSkolem θ slot value
  | .observed referent, value => NTy.toSkolem_ofSkolem θ referent value

theorem HList.toSkolem_ofSkolem (θ : TypeArgs) : (row : NRow) →
    (values : @HList (Carriers.instantiate θ Θ) row) →
    HList.toSkolem θ row (HList.ofSkolem θ row values) = values
  | .nil, () => rfl
  | .cons τ rest, values => by
      simp only [HList.toSkolem, HList.ofSkolem, NTy.toSkolem_ofSkolem θ τ, HList.toSkolem_ofSkolem θ rest]
      rfl

theorem variantCarrier.toSkolem_ofSkolem (θ : TypeArgs) : (names : List String) →
    (rows : NRows) → (value : @variantCarrier (Carriers.instantiate θ Θ) names rows) →
    variantCarrier.toSkolem θ names rows (variantCarrier.ofSkolem θ names rows value) = value
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl values => by
      simp only [variantCarrier.toSkolem, variantCarrier.ofSkolem, HList.toSkolem_ofSkolem θ fields]
  | _ :: names, .cons _ rest, .inr value => by
      simp only [variantCarrier.toSkolem, variantCarrier.ofSkolem, variantCarrier.toSkolem_ofSkolem θ names rest]
end

/-! An equation to a caller's value in the callee's view is stated in the
caller's view, where the caller's value is a local to substitute. -/

theorem NTy.eq_toSkolem_iff (θ : TypeArgs) (τ : NTy)
    (value : @NTy.carrier (Carriers.instantiate θ Θ) τ) (caller : (τ.subst θ.1).carrier) :
    value = NTy.toSkolem θ τ caller ↔ NTy.ofSkolem θ τ value = caller :=
  ⟨fun h => h ▸ NTy.ofSkolem_toSkolem θ τ caller,
    fun h => h ▸ (NTy.toSkolem_ofSkolem θ τ value).symm⟩

theorem NTy.toSkolem_eq_iff (θ : TypeArgs) (τ : NTy)
    (value : @NTy.carrier (Carriers.instantiate θ Θ) τ) (caller : (τ.subst θ.1).carrier) :
    NTy.toSkolem θ τ caller = value ↔ caller = NTy.ofSkolem θ τ value := by
  rw [eq_comm, NTy.eq_toSkolem_iff, eq_comm]

theorem HList.eq_toSkolem_iff (θ : TypeArgs) (row : NRow)
    (values : @HList (Carriers.instantiate θ Θ) row) (caller : HList (NRow.subst θ.1 row)) :
    values = HList.toSkolem θ row caller ↔ HList.ofSkolem θ row values = caller :=
  ⟨fun h => h ▸ HList.ofSkolem_toSkolem θ row caller,
    fun h => h ▸ (HList.toSkolem_ofSkolem θ row values).symm⟩

theorem HList.toSkolem_eq_iff (θ : TypeArgs) (row : NRow)
    (values : @HList (Carriers.instantiate θ Θ) row) (caller : HList (NRow.subst θ.1 row)) :
    HList.toSkolem θ row caller = values ↔ caller = HList.ofSkolem θ row values := by
  rw [eq_comm, HList.eq_toSkolem_iff, eq_comm]

/-- Two callee values are equal exactly when their caller views are. -/
theorem NTy.ofSkolem_inj (θ : TypeArgs) (τ : NTy)
    (left right : @NTy.carrier (Carriers.instantiate θ Θ) τ) :
    NTy.ofSkolem θ τ left = NTy.ofSkolem θ τ right ↔ left = right :=
  ⟨fun same => by rw [← NTy.toSkolem_ofSkolem θ τ left, same, NTy.toSkolem_ofSkolem],
    fun same => same ▸ rfl⟩

/-- Two optional callee values are equal exactly when their caller views
are: the view of a read that may be absent, such as a variant's field
(`Choices.selectCertified?`). -/
theorem NTy.ofSkolem_inj_option (θ : TypeArgs) (τ : NTy)
    (left right : Option (@NTy.carrier (Carriers.instantiate θ Θ) τ)) :
    left.map (NTy.ofSkolem θ τ) = right.map (NTy.ofSkolem θ τ) ↔ left = right := by
  constructor
  · intro same
    cases left <;> cases right <;>
      simp_all [Option.map_some, Option.map_none, NTy.ofSkolem_inj]
  · intro same
    rw [same]

theorem variantCarrier.eq_toSkolem_iff (θ : TypeArgs) (names : List String) (rows : NRows)
    (value : @variantCarrier (Carriers.instantiate θ Θ) names rows)
    (caller : variantCarrier names (NRows.subst θ.1 rows)) :
    value = variantCarrier.toSkolem θ names rows caller ↔
      variantCarrier.ofSkolem θ names rows value = caller :=
  ⟨fun h => h ▸ variantCarrier.ofSkolem_toSkolem θ names rows caller,
    fun h => h ▸ (variantCarrier.toSkolem_ofSkolem θ names rows value).symm⟩

theorem variantCarrier.toSkolem_eq_iff (θ : TypeArgs) (names : List String) (rows : NRows)
    (value : @variantCarrier (Carriers.instantiate θ Θ) names rows)
    (caller : variantCarrier names (NRows.subst θ.1 rows)) :
    variantCarrier.toSkolem θ names rows caller = value ↔
      caller = variantCarrier.ofSkolem θ names rows value := by
  rw [eq_comm, variantCarrier.eq_toSkolem_iff, eq_comm]

end Transport


/-! ## Frames

A frame's family gives the carriers of its type parameters and says what
its types are at the runtime family, whose values global memory holds, with
its values carried across.  At the runtime frame, the frame of every
function without type parameters, both are identities by definition, so a
goal over such a function carries no transport. -/

/-- A slot consulted at a substitution and then at a family is the slot
consulted at their composition. -/
theorem NTy.sharedAtWith_subst (θ : NRow) (types : Nat → NTy) (declared : Bool) :
    (τ : NTy) → (τ.subst θ).sharedAtWith types (τ.sharedAtWith θ.argument declared) =
      τ.sharedAtWith (fun index => (θ.argument index).argumentWith types) declared
  | .param index => by
      simp only [NTy.subst, NTy.sharedAtWith, NRow.argument]
      cases θ.getD index (.param index) <;>
        try simp only [NTy.unmark, NTy.isObserved, NTy.sharedAtWith, NTy.argumentWith,
          NTy.substWith, NTy.unmark_unmark, Bool.or_false, Bool.or_true]
      all_goals try (split <;> rfl)
      all_goals rfl
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char
  | .tuple _ | .struct _ _ _ | .enum _ _ _ _ _ | .slice _ | .vector _ | .array _ _
  | .table _ _ | .ghost _ | .ref _ | .sliceRef _ | .observed _ | .function _ _ _
  | .nominal _ _ _ => rfl

/-- A row of slots consulted at a substitution and then at a family. -/
theorem Sharing.atRow_subst (θ : NRow) (types : Nat → NTy) :
    (flags : List Bool) → (row : NRow) →
      Sharing.atRow types (Sharing.atRow θ.argument flags row) (NRow.subst θ row) =
        Sharing.atRow (fun index => (θ.argument index).argumentWith types) flags row
  | [], _ => rfl
  | _ :: rest, .nil => by
      simp only [NRow.subst, Sharing.atRow, NTy.sharedAtWith,
        Sharing.atRow_subst θ types rest .nil]
  | declared :: rest, .cons τ row => by
      simp only [NRow.subst, Sharing.atRow, NTy.sharedAtWith_subst θ types declared τ,
        Sharing.atRow_subst θ types rest row]

/-- A function type's sharing consulted at a substitution and then at a family. -/
theorem Sharing.atWith_subst (θ : NRow) (types : Nat → NTy) (parameters : NRow)
    (shared : Sharing) (results : NRow) :
    Sharing.atWith types (NRow.subst θ parameters) (Sharing.at θ parameters shared results)
        (NRow.subst θ results) =
      Sharing.atWith (fun index => (θ.argument index).argumentWith types) parameters shared
        results := by
  simp only [Sharing.at, Sharing.atWith, Sharing.atRow_subst]

mutual
/-- Resolving the parameters after a substitution resolves each parameter's
substituted type. -/
theorem NTy.substWith_subst (θ : NRow) (types : Nat → NTy) : (τ : NTy) →
    (τ.subst θ).substWith types =
      τ.substWith fun index => (θ.argument index).argumentWith types
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .char => rfl
  | .param index => by
      simp only [NTy.subst, NTy.substWith, NRow.argument]
      cases entry : θ.getD index (.param index) <;>
        simp only [entry, NTy.unmark, NTy.argumentWith, NTy.substWith, NTy.unmark_unmark,
          NTy.substWith_unmark types]
  | .tuple elements => congrArg NTy.tuple (NRow.substWith_subst θ types elements)
  | .struct source arguments fields => by
      simp only [NTy.subst, NTy.substWith, NRow.substWith_subst θ types arguments, NRow.substWith_subst θ types fields]
  | .enum source arguments names rows distinct => by
      simp only [NTy.subst, NTy.substWith, NRow.substWith_subst θ types arguments,
        NRows.substWith_subst θ types rows]
  | .slice element => congrArg NTy.slice (NTy.substWith_subst θ types element)
  | .vector element => congrArg NTy.vector (NTy.substWith_subst θ types element)
  | .array element length => congrArg (NTy.array · length) (NTy.substWith_subst θ types element)
  | .table key value => by
      simp only [NTy.subst, NTy.substWith, NTy.substWith_subst θ types key,
        NTy.substWith_subst θ types value]
  | .sliceRef referent => congrArg NTy.sliceRef (NTy.substWith_subst θ types referent)
  | .ref referent => congrArg NTy.ref (NTy.substWith_subst θ types referent)
  | .ghost slot => congrArg NTy.ghost (NTy.substWith_subst θ types slot)
  | .observed referent => congrArg NTy.observed (NTy.substWith_subst θ types referent)
  | .function parameters shared results => by
      simp only [NTy.subst, NTy.substWith, NRow.substWith_subst θ types parameters,
        NRow.substWith_subst θ types results, Sharing.atWith_subst]
  | .nominal source arguments productive => by
      simp only [NTy.subst, NTy.substWith, NRow.substWith_subst θ types arguments]

theorem NRow.substWith_subst (θ : NRow) (types : Nat → NTy) : (row : NRow) →
    (NRow.subst θ row).substWith types =
      row.substWith fun index => (θ.argument index).argumentWith types
  | .nil => rfl
  | .cons τ rest => by
      simp only [NRow.substWith, NTy.substWith_subst θ types τ, NRow.substWith_subst θ types rest]

theorem NRows.substWith_subst (θ : NRow) (types : Nat → NTy) : (rows : NRows) →
    (NRows.subst θ rows).substWith types =
      rows.substWith fun index => (θ.argument index).argumentWith types
  | .nil => rfl
  | .cons fields rest => by
      simp only [NRows.substWith, NRow.substWith_subst θ types fields, NRows.substWith_subst θ types rest]
end

mutual
/-- A substitution by inhabitable types keeps a type inhabitable. -/
theorem NTy.inhabitable_substWith (f : Nat → NTy) (fits : ∀ index, (f index).inhabitable = true) :
    (τ : NTy) → τ.inhabitable = true → (τ.substWith f).inhabitable = true
  | .tuple elements, held => by
      simp only [NTy.substWith, NTy.inhabitable] at held ⊢
      exact NRow.inhabitable_substWith f fits elements held
  | .struct _ _ fields, held => by
      simp only [NTy.substWith, NTy.inhabitable] at held ⊢
      exact NRow.inhabitable_substWith f fits fields held
  | .enum _ _ names rows _, held => by
      simp only [NTy.substWith, NTy.inhabitable] at held ⊢
      exact NRows.firstInhabitable_substWith f fits names rows held
  | .ref referent, held => by
      simp only [NTy.substWith, NTy.inhabitable] at held ⊢
      exact NTy.inhabitable_substWith f fits referent held
  | .ghost slot, held => by
      simp only [NTy.substWith, NTy.inhabitable] at held ⊢
      exact NTy.inhabitable_substWith f fits slot held
  | .observed referent, held => by
      simp only [NTy.substWith, NTy.inhabitable] at held ⊢
      exact NTy.inhabitable_substWith f fits referent held
  | .param index, _ => (NTy.inhabitable_unmark _).trans (fits index)
  | .function _ _ _, held => by simp [NTy.inhabitable] at held
  | .nominal _ arguments _, held => by
      simp only [NTy.substWith, NTy.inhabitable, Bool.and_eq_true] at held ⊢
      exact ⟨held.1, NRow.inhabitable_substWith f fits arguments held.2⟩
  | .sliceRef _, _ => rfl
  | .slice _, _ => rfl
  | .vector _, _ => rfl
  | .table _ _, _ => rfl
  | .unit, _ => rfl
  | .array element _, held => by
      simp only [NTy.substWith, NTy.inhabitable, Bool.and_eq_true] at held ⊢
      exact ⟨held.1, NTy.inhabitable_substWith f fits element held.2⟩
  | .bool, _ => rfl
  | .int _ _, held => held
  | .num, _ => rfl
  | .address, _ => rfl
  | .signer, _ => rfl
  | .string, _ => rfl
  | .bytes, _ => rfl
  | .char, _ => rfl

theorem NRow.inhabitable_substWith (f : Nat → NTy) (fits : ∀ index, (f index).inhabitable = true) :
    (row : NRow) → row.inhabitable = true → (NRow.substWith f row).inhabitable = true
  | .nil, _ => rfl
  | .cons τ rest, held => by
      simp only [NRow.substWith, NRow.inhabitable, Bool.and_eq_true] at held ⊢
      exact ⟨NTy.inhabitable_substWith f fits τ held.1, NRow.inhabitable_substWith f fits rest held.2⟩

theorem NRows.firstInhabitable_substWith (f : Nat → NTy)
    (fits : ∀ index, (f index).inhabitable = true) :
    (names : List String) → (rows : NRows) → NRows.firstInhabitable names rows = true →
      NRows.firstInhabitable names (NRows.substWith f rows) = true
  | _ :: _, .cons fields _, held => by
      simp only [NRows.substWith, NRows.firstInhabitable] at held ⊢
      exact NRow.inhabitable_substWith f fits fields held
  | [], _, held => by simp [NRows.firstInhabitable] at held
  | _ :: _, .nil, held => by simp [NRows.firstInhabitable] at held
end

/-- A frame reads a parameter as an inhabitable type where the substitution's
entry has values: dropping or keeping a mark says nothing about them. -/
theorem NTy.inhabitable_argumentWith {types : Nat → NTy}
    (fits : ∀ index, (types index).inhabitable = true) (τ : NTy) (held : τ.inhabitable = true) :
    (τ.argumentWith types).inhabitable = true := by
  cases τ <;>
    first
      | exact fits _
      | exact NTy.inhabitable_substWith types fits _ held

/-- The family of a frame: the carriers of its type parameters, each of its
types as a runtime-family type, and its values carried to and from that
family, inverse to each other and keeping their encodings. -/
class Skolems (unit : outParam Validation.ValidatedUnit) [FunctionCarriers unit] extends Carriers where
  /-- A type of the frame at the runtime family. -/
  resolve : NTy → NTy
  /-- A type parameter as the frame reads it, marked where it stands for a
  shared reference: what a function type's slot declared at it consults
  (`doc/designs/denotation.md`, "Instances at a reference"). -/
  argument : Nat → NTy
  /-- Resolution replaces each type parameter by its reading, a mark dropped
  wherever the parameter stands for a type. -/
  resolve_eq : ∀ τ, resolve τ = τ.substWith argument
  toRuntime : (τ : NTy) → @NTy.carrier toCarriers τ → @NTy.carrier (Carriers.runtime unit) (resolve τ)
  ofRuntime : (τ : NTy) → @NTy.carrier (Carriers.runtime unit) (resolve τ) → @NTy.carrier toCarriers τ
  ofRuntime_toRuntime : ∀ τ value, ofRuntime τ (toRuntime τ value) = value
  toRuntime_ofRuntime : ∀ τ value, toRuntime τ (ofRuntime τ value) = value
  encode_toRuntime : ∀ τ value,
    @NTy.encode (Carriers.runtime unit) (resolve τ) (toRuntime τ value) = @NTy.encode toCarriers τ value
  /-- A frame reads its type parameters as inhabitable types, as the type
  arguments of a resource are (`TypeArgs`). -/
  argument_inhabitable : ∀ index, (argument index).inhabitable = true
  /-- The inhabitants of the runtime family: the content of the ledger
  hypothesis that the unit's data invariants are satisfiable
  (`DataInvariantsSatisfiable`).  Every frame's inhabitants follow from it
  (`Skolems.carriersNonempty`), so it is carried here and assumed once. -/
  runtimeNonempty : CarriersNonempty (Carriers.runtime unit)
  /-- A frame's lent value of a type at the runtime family: what the current
  of a mutable reference holds, which carries across frames as a certified
  value does (`toRuntime`). -/
  plainToRuntime : (τ : NTy) → @NTy.plain toCarriers τ →
    @NTy.plain (Carriers.runtime unit) (resolve τ)
  plainOfRuntime : (τ : NTy) → @NTy.plain (Carriers.runtime unit) (resolve τ) →
    @NTy.plain toCarriers τ
  plainOfRuntime_plainToRuntime : ∀ τ value, plainOfRuntime τ (plainToRuntime τ value) = value
  plainToRuntime_plainOfRuntime : ∀ τ value, plainToRuntime τ (plainOfRuntime τ value) = value
  plainEncode_plainToRuntime : ∀ τ value,
    @NTy.plainEncode (Carriers.runtime unit) (resolve τ) (plainToRuntime τ value) =
      @NTy.plainEncode toCarriers τ value
  /-- Forgetting a value's certificates commutes with moving it to the
  runtime family. -/
  forget_toRuntime : ∀ τ value,
    plainToRuntime τ (@NTy.forget toCarriers τ value) =
      @NTy.forget (Carriers.runtime unit) (resolve τ) (toRuntime τ value)
  /-- The data invariants a frame states of a declaration are the unit's, at
  the declaration's arguments as the frame resolves them.  A data invariant
  is a property of an encoded value, and a value keeps its encoding across
  frames (`encode_toRuntime`), so the certificate transports with it
  (`doc/designs/data-invariants.md`, "Data invariants";
  `StoredCase.atFrame` is the precedent).  Stated with `NRow.substWith`, as
  `resolve_eq` is, because `NRow.map` is declared later; the normal form is
  the mapped row (`Carriers.own_frame`). -/
  own_resolve : ∀ source arguments value,
    @Carriers.own toCarriers source arguments value =
      FunctionCarriers.own unit source
        (NRow.substWith (fun index => resolve (.param index)) arguments) value


variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
  [CarriersNonempty (Carriers.runtime unit)]

/-- The type parameter `index` of a frame at the runtime family.  It names
the type arguments of an opaque specification function, which nothing else
identifies. -/
@[reducible] def Skolems.type (Θ : Skolems unit) (index : Nat) : NTy := Θ.resolve (.param index)

omit [CarriersNonempty (Carriers.runtime unit)] in
/-- A frame's reading of a parameter and its resolution of it agree, the mark
aside: resolution drops it. -/
theorem Skolems.unmark_argument (Θ : Skolems unit) (index : Nat) :
    (Θ.argument index).unmark = (Θ.type index).unmark := by
  show _ = (Θ.resolve (.param index)).unmark
  rw [Θ.resolve_eq]
  exact (NTy.unmark_unmark _).symm

omit [CarriersNonempty (Carriers.runtime unit)] in
/-- What a frame's parameter stands for carries no mark: resolution drops it.
In the normal form, so that a type argument read through a substitution and
one read off the frame are one spelling. -/
@[simp, vir_denote_norm] theorem Skolems.unmark_type (Θ : Skolems unit) (index : Nat) :
    (Θ.type index).unmark = Θ.type index := by
  show (Θ.resolve (.param index)).unmark = Θ.resolve (.param index)
  rw [Θ.resolve_eq]
  exact NTy.unmark_unmark _

omit [CarriersNonempty (Carriers.runtime unit)] in
/-- A frame resolves a type by substituting what its parameters stand for. -/
theorem Skolems.resolve_eq_type (Θ : Skolems unit) (τ : NTy) :
    Θ.resolve τ = τ.substWith Θ.type :=
  (Θ.resolve_eq τ).trans (NTy.substWith_unmarked_congr Θ.unmark_argument τ)

/-- The inhabitants of a frame's carriers, from the runtime family's: a
value of a type at the frame is the runtime family's value of the type the
frame resolves it to (`Skolems.ofRuntime`), and a frame reads every type
parameter as an inhabitable type.  Only the runtime family's inhabitants
are assumed (`Skolems.runtimeNonempty`); every frame's follow. -/
noncomputable instance Skolems.carriersNonempty (Θ : Skolems unit) :
    CarriersNonempty Θ.toCarriers where
  -- MERGE-REVIEW: a nominal's inhabitant is the frame's own
  -- (`Carriers.nominalInhabitant`), which `NTy.readBack_unit_nominal`
  -- reads; nothing relates a carry-back from the runtime family to it, and
  -- a nominal's values carry no certificate, so there is nothing to carry.
  inhabitant := fun τ inhabitable =>
    match τ, inhabitable with
    | .nominal source arguments true, held =>
        @Carriers.nominalInhabitant Θ.toCarriers source arguments
          (by simpa [NTy.inhabitable] using held)
    | τ, inhabitable =>
        Θ.ofRuntime τ (@NTy.inhabitant (Carriers.runtime unit) Θ.runtimeNonempty (Θ.resolve τ)
          (by
            rw [Θ.resolve_eq τ]
            exact NTy.inhabitable_substWith _ Θ.argument_inhabitable τ inhabitable))
  inhabitant_nominal := fun _ _ _ _ => rfl

/-- The inhabitants of the runtime family, as a frame carries them. -/
instance (priority := low) Skolems.runtimeCarriersNonempty [Θ : Skolems unit] :
    CarriersNonempty (Carriers.runtime unit) := Θ.runtimeNonempty


/-- The frame of the public theorem and of every function without type
parameters: the runtime family itself. -/
@[reducible] noncomputable def Skolems.runtime (unit : Validation.ValidatedUnit)
    [FunctionCarriers unit] [CarriersNonempty (Carriers.runtime unit)] : Skolems unit where
  toCarriers := Carriers.runtime unit
  resolve := fun τ => τ
  argument := fun index => .param index
  resolve_eq := fun τ => (NTy.substWith_param τ).symm
  argument_inhabitable := fun _ => rfl
  toRuntime := fun _ value => value
  ofRuntime := fun _ value => value
  ofRuntime_toRuntime := fun _ _ => rfl
  toRuntime_ofRuntime := fun _ _ => rfl
  encode_toRuntime := fun _ _ => rfl
  runtimeNonempty := inferInstance
  plainToRuntime := fun _ value => value
  plainOfRuntime := fun _ value => value
  plainOfRuntime_plainToRuntime := fun _ _ => rfl
  plainToRuntime_plainOfRuntime := fun _ _ => rfl
  plainEncode_plainToRuntime := fun _ _ => rfl
  forget_toRuntime := fun _ _ => rfl
  own_resolve := fun _ arguments _ => by rw [NRow.substWith_param arguments]; rfl


/-- The frame a generic call's type arguments induce: its carriers are the
arguments' at the caller's family, and its types resolve through the
caller's. -/
@[reducible] noncomputable def Skolems.instantiate (θ : TypeArgs) (outer : Skolems unit) :
    Skolems unit where
  toCarriers := Carriers.instantiate θ outer.toCarriers
  resolve := fun τ => outer.resolve (τ.subst θ.1)
  argument := fun index => (θ.1.argument index).argumentWith outer.argument
  resolve_eq := fun τ => by
    rw [outer.resolve_eq (τ.subst θ.1), NTy.substWith_subst]
  toRuntime := fun τ value =>
    outer.toRuntime (τ.subst θ.1) (@NTy.ofSkolem outer.toCarriers inferInstance θ τ value)
  ofRuntime := fun τ value =>
    @NTy.toSkolem outer.toCarriers inferInstance θ τ (outer.ofRuntime (τ.subst θ.1) value)
  ofRuntime_toRuntime := fun τ value => by
    simp only [outer.ofRuntime_toRuntime, @NTy.toSkolem_ofSkolem outer.toCarriers inferInstance θ τ value]
  toRuntime_ofRuntime := fun τ value => by
    simp only [@NTy.ofSkolem_toSkolem outer.toCarriers inferInstance θ τ, outer.toRuntime_ofRuntime]
  encode_toRuntime := fun τ value => by
    simp only [outer.encode_toRuntime, @NTy.encode_ofSkolem outer.toCarriers inferInstance θ τ value]
  argument_inhabitable := fun index =>
    NTy.inhabitable_argumentWith outer.argument_inhabitable _

      (NRow.getD_inhabitable θ.1 θ.2 index (.param index) rfl)
  runtimeNonempty := outer.runtimeNonempty
  plainToRuntime := fun τ value =>
    outer.plainToRuntime (τ.subst θ.1)
      (@NTy.plainOfSkolem outer.toCarriers inferInstance θ τ value)
  plainOfRuntime := fun τ value =>
    @NTy.plainToSkolem outer.toCarriers inferInstance θ τ
      (outer.plainOfRuntime (τ.subst θ.1) value)
  plainOfRuntime_plainToRuntime := fun τ value => by
    simp only [outer.plainOfRuntime_plainToRuntime,
      @NTy.plainToSkolem_ofSkolem outer.toCarriers inferInstance θ τ value]
  plainToRuntime_plainOfRuntime := fun τ value => by
    simp only [@NTy.plainOfSkolem_toSkolem outer.toCarriers inferInstance θ τ,
      outer.plainToRuntime_plainOfRuntime]
  plainEncode_plainToRuntime := fun τ value => by
    simp only [outer.plainEncode_plainToRuntime,
      @NTy.plainEncode_ofSkolem outer.toCarriers inferInstance θ τ value]
  forget_toRuntime := fun τ value => by
    rw [@NTy.forget_ofSkolem outer.toCarriers inferInstance θ τ value]
    exact outer.forget_toRuntime (τ.subst θ.1) _
  own_resolve := fun source arguments value => by
    -- A substitution's entry is read with its mark (`NTy.argumentWith`) and
    -- resolved without it, and a row's substitution drops the mark
    -- (`NRow.substWith_unmarked_congr`), so the two readings of the row agree.
    have marked : ∀ (types : Nat → NTy) (entry : NTy),
        (entry.argumentWith types).unmark = (entry.substWith types).unmark := by
      intro types entry
      cases entry <;>
        simp only [NTy.argumentWith, NTy.substWith, NTy.unmark, NTy.unmark_unmark]
    have row : NRow.substWith (fun index => outer.resolve (.param index))
          (NRow.subst θ.1 arguments) =
        NRow.substWith (fun index => outer.resolve ((NTy.param index).subst θ.1)) arguments := by
      rw [NRow.substWith_subst θ.1 (fun index => outer.resolve (.param index)) arguments]
      refine NRow.substWith_unmarked_congr (fun index => ?_) arguments
      have resolved : outer.resolve ((NTy.param index).subst θ.1) =
          ((θ.1.argument index).substWith (fun i => outer.resolve (.param i))).unmark := by
        rw [outer.resolve_eq_type]
        show ((θ.1.argument index).unmark).substWith outer.type = _
        rw [NTy.substWith_unmark]
      rw [marked, resolved, NTy.unmark_unmark]
    show @Carriers.own outer.toCarriers source (NRow.subst θ.1 arguments) value = _
    rw [outer.own_resolve source (NRow.subst θ.1 arguments) value, row]

attribute [vir_denote, vir_denote_norm] Skolems.ofRuntime_toRuntime Skolems.toRuntime_ofRuntime

/-- A frame types the closures the runtime family types at the rows it
resolves, as its transports to the runtime family keep encodings. -/
theorem Skolems.ofClosure?_of_runtime [Θ : Skolems unit] {parameters results : NRow}
    {shared : Sharing} {closure : ClosureValue}
    (typed : NTy.admits unit (.function (parameters.substWith Θ.type) shared
      (results.substWith Θ.type)) closure.encode = true) :
    ((Carriers.functions parameters shared results).ofClosure? closure).isSome = true := by
  have resolved : NTy.function (parameters.substWith Θ.type) shared
      (results.substWith Θ.type) = Θ.resolve (.function parameters shared results) :=
    (Θ.resolve_eq_type (.function parameters shared results)).symm
  -- The closure at the unit's function values, carried to the frame.
  let functions := FunctionCarriers.functions (unit := unit) (parameters.substWith Θ.type)
    shared (results.substWith Θ.type)
  have decoded := FunctionCarriers.decodes _ _ _ closure typed
  let value := Θ.ofRuntime (.function parameters shared results)
    (resolved ▸ (functions.ofClosure? closure).get decoded)
  have encoded := Θ.encode_toRuntime (.function parameters shared results) value
  rw [Θ.toRuntime_ofRuntime, @NTy.encode_cast (Carriers.runtime unit) _ _ resolved] at encoded
  simp only [NTy.encode_function] at encoded
  have back := functions.closure_of_ofClosure? closure _ (Option.some_get decoded).symm
  have encoded : closure.encode =
      ((Carriers.functions parameters shared results).closure value).encode := by
    rw [← back]; exact encoded
  have same := ClosureValue.encode_injective encoded
  rw [show closure = (Carriers.functions parameters shared results).closure value from same,
    (Carriers.functions parameters shared results).ofClosure?_closure]
  rfl

/-- A type parameter of an induced frame is its argument with the caller's
type parameters in place: spelled by its structure, as a contract spells the
types it applies. -/
theorem Skolems.type_instantiate (θ : TypeArgs) (Θ : Skolems unit) (index : Nat) :
    (Skolems.instantiate θ Θ).type index = ((NTy.param index).subst θ.1).substWith Θ.type :=
  Θ.resolve_eq_type _

theorem Skolems.type_runtime (index : Nat) : (Skolems.runtime unit).type index = .param index := rfl

/-- A frame resolves its type parameters to inhabitable types: the reading of a
parameter has values, and dropping its mark keeps them. -/
theorem Skolems.type_inhabitable (Θ : Skolems unit) (index : Nat) :
    (Θ.type index).inhabitable = true := by
  show (Θ.resolve (.param index)).inhabitable = true
  rw [Θ.resolve_eq]
  exact (NTy.inhabitable_unmark _).trans (Θ.argument_inhabitable index)

/-- A frame resolves a constructor's type through its parameters, without
unfolding a parameter the frame leaves opaque. -/
@[vir_denote_norm] theorem Skolems.resolve_int (frame : Skolems unit) (width : Nat) (signed : Bool) :
    frame.resolve (.int width signed) = .int width signed := frame.resolve_eq _
@[vir_denote_norm] theorem Skolems.resolve_num (frame : Skolems unit) :
    frame.resolve .num = .num := frame.resolve_eq _
@[vir_denote_norm] theorem Skolems.resolve_unit (frame : Skolems unit) :
    frame.resolve .unit = .unit := frame.resolve_eq _
@[vir_denote_norm] theorem Skolems.resolve_bool (frame : Skolems unit) :
    frame.resolve .bool = .bool := frame.resolve_eq _
@[vir_denote_norm] theorem Skolems.resolve_address (frame : Skolems unit) :
    frame.resolve .address = .address := frame.resolve_eq _
@[vir_denote_norm] theorem Skolems.resolve_signer (frame : Skolems unit) :
    frame.resolve .signer = .signer := frame.resolve_eq _
@[vir_denote_norm] theorem Skolems.resolve_string (frame : Skolems unit) :
    frame.resolve .string = .string := frame.resolve_eq _
@[vir_denote_norm] theorem Skolems.resolve_bytes (frame : Skolems unit) :
    frame.resolve .bytes = .bytes := frame.resolve_eq _
@[vir_denote_norm] theorem Skolems.resolve_struct (frame : Skolems unit) (owner : StructHandle)
    (arguments fields : NRow) :
    frame.resolve (.struct owner arguments fields) =
      .struct owner (NRow.substWith frame.type arguments) (NRow.substWith frame.type fields) :=
  frame.resolve_eq_type _

/-- A frame resolves a type without parameters to itself. -/
theorem Skolems.resolve_paramFree [Θ : Skolems unit] {τ : NTy} (free : τ.paramFree = true) :
    Θ.resolve τ = τ := by
  rw [Θ.resolve_eq τ, NTy.substWith_paramFree _ τ free]

open Lean Meta Simp in
/-- A frame resolves a type without parameters to itself
(`Skolems.resolve_paramFree`), decided where the type is frame-free
(`decidedFrameFree?`): the constructor rules' normal form for any closed
type, a nominal descriptor of a declaration among them. -/
simproc [vir_denote_norm] resolveParamFree (@Skolems.resolve _ _ ?frame _) := fun e => do
  unless e.isAppOfArity ``Skolems.resolve 4 do return .continue
  let arguments := e.getAppArgs
  let τ := arguments[3]!
  let some free ← decidedFrameFree? (← mkEq (mkApp (mkConst ``NTy.paramFree) τ)
      (mkConst ``Bool.true)) | return .continue
  let nonempty := mkApp3 (mkConst ``Skolems.runtimeNonempty) arguments[0]! arguments[1]! arguments[2]!
  return .done { expr := τ, proof? := some (mkAppN (mkConst ``Skolems.resolve_paramFree)
    #[arguments[0]!, arguments[1]!, nonempty, arguments[2]!, τ, free]) }


/-- At the public frame a type is itself. -/
theorem NTy.substWith_type_runtime (τ : NTy) : τ.substWith (Skolems.runtime unit).type = τ :=
  NTy.substWith_param τ

/-- At the public frame a parameter's reading is the parameter. -/
@[simp] theorem Skolems.argument_runtime (index : Nat) :
    (Skolems.runtime unit).argument index = .param index := rfl

/-- An induced frame resolves a type, and carries its values, through the
caller's frame at the type's substitution. -/
theorem Skolems.resolve_instantiate (θ : TypeArgs) (Θ : Skolems unit) (τ : NTy) :
    (Skolems.instantiate θ Θ).resolve τ = Θ.resolve (τ.subst θ.1) := rfl
theorem Skolems.ofRuntime_instantiate (θ : TypeArgs) (Θ : Skolems unit) (τ : NTy)
    (value : @NTy.carrier (Carriers.runtime unit) ((Skolems.instantiate θ Θ).resolve τ)) :
    (Skolems.instantiate θ Θ).ofRuntime τ value =
      @NTy.toSkolem Θ.toCarriers inferInstance θ τ (Θ.ofRuntime (τ.subst θ.1) value) := rfl
theorem Skolems.toRuntime_instantiate (θ : TypeArgs) (Θ : Skolems unit) (τ : NTy)
    (value : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers τ) :
    (Skolems.instantiate θ Θ).toRuntime τ value =
      Θ.toRuntime (τ.subst θ.1) (@NTy.ofSkolem Θ.toCarriers inferInstance θ τ value) := rfl

/-- The runtime frame resolves a type to itself, and carries values as they are. -/
theorem Skolems.resolve_runtime (τ : NTy) : (Skolems.runtime unit).resolve τ = τ := rfl
theorem Skolems.ofRuntime_runtime (τ : NTy)
    (value : @NTy.carrier (Carriers.runtime unit) τ) :
    (Skolems.runtime unit).ofRuntime τ value = value := rfl
theorem Skolems.toRuntime_runtime (τ : NTy)
    (value : @NTy.carrier (Carriers.runtime unit) τ) :
    (Skolems.runtime unit).toRuntime τ value = value := rfl

open Lean Meta in
/-- A frame as the equations above spell it: `Skolems.runtime` or
`Skolems.instantiate` applied, reached from a frame a definition induces
(`Skolems.instance`) by unfolding reducible heads one at a time; none at
anything else, in particular at a function's own frame, which is a variable
and unfolds to nothing.  Nothing else is reduced, so the test is of the
head and never of what a frame carries. -/
private def skolemsFrame? : Nat → Lean.Expr → MetaM (Option Lean.Expr)
  | 0, _ => return none
  | fuel + 1, frame => do
    let .const name _ := frame.getAppFn | return none
    if name == ``Skolems.runtime || name == ``Skolems.instantiate then return some frame
    let some unfolded ← withReducible (unfoldDefinition? frame) | return none
    skolemsFrame? fuel unfolded

open Lean Meta in
/-- What a `Skolems` operation reads at the frame one of the equations above
names, decided by that frame's head.

A frame is an instance argument, which a discrimination tree stores as a
wildcard: simp offers every `Skolems` operation to each of those equations,
and an operation at a frame they do not name learns that only by unifying
the operation's value argument, whose type names the frame's carrier
family.  That unification walks the inlined rows of both families, and one
failed match of `Skolems.toRuntime_runtime` at a function's own frame costs
tens of millions of heartbeats.  This reader decides on the frame's head
instead and reads nothing at a head it does not know, so that the failure
is the test and not the walk.

At the two heads it knows the reading is the equation's own right side, at
the operation's own arguments, which is why `arity` is the operation's and
the equation is instantiated and not matched: the equations are `rfl`, so
every reader of this is definitional (`dsimproc`), as simp's own use of a
`rfl` equation is. -/
private def skolemsFrameReading? (runtime instantiate : Option Name) (arity : Nat)
    (operation : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let arguments := operation.getAppArgs
  unless arguments.size == arity do return none
  let unit := arguments[0]!
  let carriers := arguments[1]!
  let some frame ← skolemsFrame? 4 arguments[2]! | return none
  let read := arguments.extract 3 arguments.size
  let .const frameName _ := frame.getAppFn | return none
  let frameArguments := frame.getAppArgs
  -- The frame's unit and function carriers are the operation's own, so that
  -- the equation's left side is the operation itself.
  unless frameArguments.size ≥ 2 && frameArguments[0]! == unit && frameArguments[1]! == carriers do
    return none
  let equation? : Option Lean.Expr :=
    if frameName == ``Skolems.runtime && frameArguments.size == 3 then
      runtime.map fun equation =>
        mkAppN (mkConst equation) (#[unit, carriers, frameArguments[2]!] ++ read)
    else if frameName == ``Skolems.instantiate && frameArguments.size == 4 then
      instantiate.map fun equation =>
        -- An induced frame's equations take the runtime family's inhabitants,
        -- which their statements do not name; the frame carries them.
        mkAppN (mkConst equation)
          (#[unit, carriers,
              mkAppN (mkConst ``Skolems.runtimeCarriersNonempty)
                #[unit, carriers, frameArguments[3]!],
              frameArguments[2]!, frameArguments[3]!] ++ read)
    else none
  let some equation := equation? | return none
  let some (_, left, right) := (← inferType equation).eq? | return none
  -- The equation is of the operation at the frame as the equation spells it,
  -- which is the operation itself unless a definition induced the frame; the
  -- reading is definitionally the operation's either way.
  unless left == mkAppN operation.getAppFn (arguments.set! 2 frame) do return none
  return some right

open Lean Meta Simp in
dsimproc [simp, vir_denote_norm] skolemsResolveFrame (@Skolems.resolve _ _ ?frame _) := fun e => do
  let some read ← skolemsFrameReading? ``Skolems.resolve_runtime ``Skolems.resolve_instantiate 4 e
    | return .continue
  return .visit read

open Lean Meta Simp in
dsimproc [simp, vir_denote_norm] skolemsOfRuntimeFrame (@Skolems.ofRuntime _ _ ?frame _ _) :=
  fun e => do
  let some read ←
    skolemsFrameReading? ``Skolems.ofRuntime_runtime ``Skolems.ofRuntime_instantiate 5 e
    | return .continue
  return .visit read

open Lean Meta Simp in
dsimproc [simp, vir_denote_norm] skolemsToRuntimeFrame (@Skolems.toRuntime _ _ ?frame _ _) :=
  fun e => do
  let some read ←
    skolemsFrameReading? ``Skolems.toRuntime_runtime ``Skolems.toRuntime_instantiate 5 e
    | return .continue
  return .visit read

-- A transported operation stands at an induced frame and reads at the
-- caller's; the runtime frame is not one a transport meets.
open Lean Meta Simp in
dsimproc [vir_transport] skolemsResolveInduced (@Skolems.resolve _ _ ?frame _) := fun e => do
  let some read ← skolemsFrameReading? none ``Skolems.resolve_instantiate 4 e | return .continue
  return .visit read

open Lean Meta Simp in
dsimproc [vir_transport] skolemsOfRuntimeInduced (@Skolems.ofRuntime _ _ ?frame _ _) := fun e => do
  let some read ← skolemsFrameReading? none ``Skolems.ofRuntime_instantiate 5 e | return .continue
  return .visit read

open Lean Meta Simp in
dsimproc [vir_transport] skolemsToRuntimeInduced (@Skolems.toRuntime _ _ ?frame _ _) := fun e => do
  let some read ← skolemsFrameReading? none ``Skolems.toRuntime_instantiate 5 e | return .continue
  return .visit read

-- `NTy.inhabitant` stays folded: it is the one spelling of a certified
-- family's default, which a clause's read and a library statement of the same
-- read both take. Unfolding it replaces it by the class projection it reads,
-- so one goal holds the same value under two spellings and the deciders see
-- two atoms. The structural defaults below do compute, and unfold.
attribute [vir_denote] NTy.plainInhabitant HListPlain.inhabitant
  variantPlain.inhabitant
  Carriers.default_instantiate
-- A variant's transport unfolds: an enum value is taken apart by its
-- variants anyway, and a match reads the variant it holds. A table's
-- transport stays folded, so that the encoding of a transported table meets
-- the callee's encoding of it (`encodeTransport`), as a vector's does.
attribute [vir_denote] variantCarrier.toSkolem variantPlain.toSkolem NTy.ofSkolem_unit
  NTy.ofSkolem_bool
  NTy.ofSkolem_int NTy.ofSkolem_address NTy.ofSkolem_num NTy.ofSkolem_signer NTy.ofSkolem_string
  NTy.ofSkolem_bytes NTy.ofSkolem_param NTy.ofSkolem_tuple NTy.ofSkolem_struct
  NTy.ofSkolem_enum NTy.ofSkolem_vector NTy.ofSkolem_ref NTy.ofSkolem_ghost
  HList.ofSkolem variantCarrier.ofSkolem HListPlain.ofSkolem variantPlain.ofSkolem
  ResultShape.ofSkolem


section Resolved
variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit] [Skolems unit]

/-- A row of a frame, as types of the runtime family. -/
def NRow.resolved : NRow → NRow
  | .nil => .nil
  | .cons τ rest => .cons (Skolems.resolve τ) (NRow.resolved rest)

/-- Values of a row of a frame, at the runtime family. -/
def HList.toRuntime : (row : NRow) → HList row → @HList (Carriers.runtime unit) row.resolved
  | .nil, _ => ()
  | .cons τ rest, values => (Skolems.toRuntime τ values.1, HList.toRuntime rest values.2)

/-- Values of a row encode as their runtime-family values do. -/
theorem HList.encode_toRuntime : (row : NRow) → (values : HList row) →
    @HList.encode (Carriers.runtime unit) row.resolved (HList.toRuntime row values) =
      HList.encode values
  | .nil, _ => rfl
  | .cons τ rest, values => by
      show @NTy.encode (Carriers.runtime unit) (Skolems.resolve τ) (Skolems.toRuntime τ values.1) ::
          @HList.encode (Carriers.runtime unit) rest.resolved (HList.toRuntime rest values.2) =
        τ.encode values.1 :: HList.encode values.2
      rw [Skolems.encode_toRuntime, HList.encode_toRuntime rest values.2]

/-- A row without type parameters is its own resolution. -/
theorem NRow.resolved_paramFree : (row : NRow) → row.paramFree = true → row.resolved = row
  | .nil, _ => rfl
  | .cons τ rest, free => by
      simp only [NRow.paramFree, Bool.and_eq_true] at free
      simp only [NRow.resolved, Skolems.resolve_paramFree free.1, NRow.resolved_paramFree rest free.2]

/-- A row's resolution is its substitution by what the frame's parameters
stand for. -/
theorem NRow.resolved_eq_substWith : (row : NRow) →
    row.resolved = row.substWith (Skolems.type ‹_›)
  | .nil => rfl
  | .cons τ rest => by
      simp only [NRow.resolved, NRow.substWith, NRow.resolved_eq_substWith rest]
      rw [Skolems.resolve_eq_type]

/-- A row resolved at the frame type arguments induce is the row with them
substituted, resolved at the outer frame. -/
theorem NRow.resolved_instantiate (θ : TypeArgs) (Θ : Skolems unit) : (row : NRow) →
    @NRow.resolved unit _ (Skolems.instantiate θ Θ) row = (NRow.subst θ.1 row).resolved
  | .nil => rfl
  | .cons τ rest => by
      simp only [NRow.resolved, Skolems.resolve_instantiate, NRow.resolved_instantiate θ Θ rest]

omit [Skolems unit] in
private theorem heq_pair {α β γ : Type} (a : α) (same : β = γ) (x : β) (y : γ)
    (sameValue : HEq x y) : HEq (a, x) (a, y) := by
  subst same; cases sameValue; rfl

omit [Skolems unit] in
/-- An induced frame carries a row's values to the runtime family as the
outer frame carries them read in its view. -/
theorem HList.toRuntime_instantiate (θ : TypeArgs) (Θ : Skolems unit) : (row : NRow) →
    (values : @HList (Skolems.instantiate θ Θ).toCarriers row) →
    HEq (@HList.toRuntime unit _ (Skolems.instantiate θ Θ) row values)
      (@HList.toRuntime unit _ Θ (NRow.subst θ.1 row) (HList.ofSkolem θ row values))
  | .nil, _ => HEq.rfl
  | .cons _ rest, values =>
      heq_pair _ (congrArg (@HList (Carriers.runtime unit)) (@NRow.resolved_instantiate unit _ _ θ Θ rest))
        _ _ (HList.toRuntime_instantiate θ Θ rest values.2)

omit [Skolems unit] in
/-- At the base frame a row resolves to itself. -/
theorem NRow.resolved_runtime [CarriersNonempty (Carriers.runtime unit)] : (row : NRow) →
    @NRow.resolved unit _ (Skolems.runtime unit) row = row
  | .nil => rfl
  | .cons _ rest => congrArg (NRow.cons _) (NRow.resolved_runtime rest)

omit [Skolems unit] in
/-- The base frame carries a row's values to themselves. -/
theorem HList.toRuntime_runtime [CarriersNonempty (Carriers.runtime unit)] : (row : NRow) →
    (values : @HList (Carriers.runtime unit) row) →
    HEq (@HList.toRuntime unit _ (Skolems.runtime unit) row values) values
  | .nil, _ => HEq.rfl
  | .cons _ rest, values =>
      heq_pair _ (congrArg (@HList (Carriers.runtime unit))
          (@NRow.resolved_runtime unit _ _ rest))
        _ _ (HList.toRuntime_runtime rest values.2)

end Resolved

end VerexIR.Proofs.Denote
