-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.StaticTyping
import LeanerIR.Semantics.Typing

/-!
# Typing values at semantic types

A runtime value inhabits a semantic type (`SemanticTypes.lean`). Loans are
typed by an environment: a live mutable loan's hole and its borrow are typed
at the loan's referent type, so writing a borrow's current value back into
its hole keeps every position typed.
-/

namespace LeanerIR

/-- The referent type of each live mutable loan. -/
abbrev LoanTypes := Nat → Option SemTy

open Validation in
/-- The types a function's frames are required to read faithfully through
their instantiation (`StaticTyping.requiredTypes`). -/
def requiredAt (unit : ValidatedUnit) (function : FunctionHandle) : Array TypeId :=
  ((StaticTyping.requiredTypes unit).getD #[]).at
    (function.namespaceId.index, function.functionId.index)

open Validation in
/-- A body's environment at semantic arguments: each type binder its
argument. -/
def frameEnv (generics : Array GenericBinder) (arguments : Array SemArg) : Array SemArg :=
  (StaticTyping.staticEnv generics).map (·.subst arguments)

open Validation in
/-- The closed type arguments a frame's instantiation table was built from,
each resolving to the frame's environment at its position. -/
def ArgumentsClosed (ns : ValidatedNamespace) (typeArguments : Array GenericArgument)
    (env : Array SemArg) : Prop :=
  ∀ (index : Nat) (value : TypeUse), typeArguments[index]? = some (GenericArgument.typeArg value) →
    ∃ type, env[index]? = some (SemArg.type type) ∧ Resolves ns.tables #[] value.typeId type

open Validation SemanticOperations in
/-- A frame reads its instantiation faithfully at the types it requires:
without an instantiation every such type is closed, and otherwise the
instantiation is the table of closed type arguments that instantiate each
such type (`Semantics/FrameInstantiation.lean`). -/
def FrameInstantiation (ns : ValidatedNamespace) (required : Array TypeId)
    (instantiation : Array (TypeId × TypeId)) (env : Array SemArg) : Prop :=
  (instantiation = #[] ∧ ∀ typeId ∈ required, ∀ type,
      Resolves ns.tables env typeId type → Resolves ns.tables #[] typeId type) ∨
  ∃ outer typeArguments, instantiation = invocationTypeInstantiation ns outer typeArguments ∧
    ArgumentsClosed ns (instantiateGenericArguments outer typeArguments) env ∧
    ∀ typeId ∈ required,
      (instantiatePlaceFieldType? ns (instantiateGenericArguments outer typeArguments) typeId).isSome

/-- A type without its outer shared references: a shared reference is the
value it observes. -/
def SemTy.unshared : SemTy → SemTy
  | .reference .shared referent => referent.unshared
  | type => type

mutual
  /-- A value inhabits a semantic type, its loans typed by `loans`. -/
  inductive HasType (unit : Validation.ValidatedUnit) (loans : LoanTypes) :
      RuntimeValue → SemTy → Prop where
    | unit : HasType unit loans .unit .unit
    | bool (value : Bool) : HasType unit loans (.bool value) .bool
    | character (value : Nat) (valid : isUnicodeScalar value = true) :
        HasType unit loans (.character value) .character
    | string (value : String) : HasType unit loans (.string value) .string
    | bytes (value : Array UInt8) : HasType unit loans (.bytes value) .bytes
    /-- An integer fits its type at the unit's target width. -/
    | integer (value : Int) (width : IntWidth) (signed : Bool)
        (fits : Validation.StaticTyping.holdsAt (Validation.targetPointerWidth? unit) width signed
          value = true) :
        HasType unit loans (.integer value) (.integer width signed)
    | address (value : String) : HasType unit loans (.address value) .address
    | signer (value : String) : HasType unit loans (.signer value) .signer
    | tuple (values : Array RuntimeValue) (types : List SemTy)
        (elements : HasTypes unit loans values.toList types) :
        HasType unit loans (.tuple values) (.tuple types)
    | vector (values : Array RuntimeValue) (element : SemTy) (length : Option ConstValue)
        (length_matches : VectorLengthMatches length values.size)
        (elements : HasTypeEach unit loans values.toList element) :
        HasType unit loans (.vector values) (.vector element length)
    /-- A nominal value inhabits its declaration's type at generic arguments
    under which each field inhabits its declared type. -/
    | nominal (source : StructHandle) (variant : Option String) (fields : Array RuntimeValue)
        (name : QualifiedName) (arguments : List SemArg)
        (declaringNamespace : Validation.ValidatedNamespace) (declared : Array FieldDecl)
        (fieldTypes : List SemTy)
        (name_eq : SemanticOperations.structName? unit source = some name)
        (declaration_eq : SemanticOperations.handleFields? unit source variant =
          some (declaringNamespace, declared))
        (fieldTypes_resolve : ResolvesAll declaringNamespace.tables arguments.toArray
          (declared.toList.map (·.type.typeId)) fieldTypes)
        (fields_typed : HasTypes unit loans fields.toList fieldTypes) :
        HasType unit loans (.nominal source variant fields) (.nominal name arguments)
    /-- A closure inhabits the function type of its target's open
    parameters and packed results, under the semantic arguments its
    instantiation is faithful to; its captures inhabit the captured
    parameters' types. -/
    | closure (function : FunctionHandle) (mask : Nat)
        (instantiation : Array (TypeId × TypeId)) (captures : Array RuntimeValue)
        (parameters : List SemTy) (result : SemTy)
        (targetNs : Validation.ValidatedNamespace)
        (declaration : FunctionDecl Validation.FunctionBody) (arguments : Array SemArg)
        (allParameters results : List SemTy)
        (namespace_eq : unit.namespaces[function.namespaceId.index]? = some targetNs)
        (declaration_eq : targetNs.functions[function.functionId.index]? = some declaration)
        (signature_eq : Validation.StaticTyping.signatureTypes? targetNs declaration
          (frameEnv declaration.signature.generics arguments) = some (allParameters, results))
        (faithful : FrameInstantiation targetNs (requiredAt unit function) instantiation
          (frameEnv declaration.signature.generics arguments))
        (mask_bound : mask < 2 ^ declaration.signature.parameters.size)
        (captures_typed : HasTypes unit loans captures.toList
          (ClosureMask.extract mask true allParameters))
        (parameters_eq : parameters = ClosureMask.extract mask false allParameters)
        (packs : Validation.StaticTyping.packs results result = true) :
        HasType unit loans (.closure function mask instantiation captures)
          (.function parameters result)
    /-- A live mutable loan's borrow holds a value of the loan's referent type. -/
    | borrow (loan : Nat) (current : RuntimeValue) (referent : SemTy)
        (loan_eq : loans loan = some referent)
        (current_typed : HasType unit loans current referent) :
        HasType unit loans (.borrow loan current) (.reference .mutable referent)
    /-- A shared reference is the value it observes. -/
    | shared (value : RuntimeValue) (referent : SemTy)
        (value_typed : HasType unit loans value referent) :
        HasType unit loans value (.reference .shared referent)
    /-- What a settled borrow leaves where it rested: a reference whose loan
    has ended, which nothing dereferences (`dereferenceBorrow?`). -/
    | dead (referent : SemTy) : HasType unit loans .unit (.reference .mutable referent)
    /-- A hole stands where a live mutable loan took its value from, at the
    loan's referent type, observed through shared references or not. -/
    | hole (loan : Nat) (referent type : SemTy) (loan_eq : loans loan = some referent)
        (same : referent.unshared = type.unshared) (inhabited : referent.unshared ≠ .never) :
        HasType unit loans (.loanHole loan) type
    /-- The unit value and the empty tuple are one value (`packResults`). -/
    | unitEmptyTuple : HasType unit loans .unit (.tuple [])
    | emptyTupleUnit : HasType unit loans (.tuple #[]) .unit

  /-- Pointwise typing of a row. -/
  inductive HasTypes (unit : Validation.ValidatedUnit) (loans : LoanTypes) :
      List RuntimeValue → List SemTy → Prop where
    | nil : HasTypes unit loans [] []
    | cons {value : RuntimeValue} {type : SemTy} {values : List RuntimeValue}
        {types : List SemTy} :
        HasType unit loans value type → HasTypes unit loans values types →
          HasTypes unit loans (value :: values) (type :: types)

  /-- Homogeneous typing of a vector's elements. -/
  inductive HasTypeEach (unit : Validation.ValidatedUnit) (loans : LoanTypes) :
      List RuntimeValue → SemTy → Prop where
    | nil {type : SemTy} : HasTypeEach unit loans [] type
    | cons {value : RuntimeValue} {values : List RuntimeValue} {type : SemTy} :
        HasType unit loans value type → HasTypeEach unit loans values type →
          HasTypeEach unit loans (value :: values) type
end

/-- One loan environment extends another: it types each loan the other
types, alike. -/
def LoanTypes.Extends (larger smaller : LoanTypes) : Prop :=
  ∀ loan type, smaller loan = some type → larger loan = some type

theorem LoanTypes.Extends.refl (loans : LoanTypes) : loans.Extends loans :=
  fun _ _ h => h

theorem LoanTypes.Extends.trans {first second third : LoanTypes}
    (outer : third.Extends second) (inner : second.Extends first) : third.Extends first :=
  fun loan type h => outer loan type (inner loan type h)

mutual
/-- A value typed under some loans is typed under any extension of them. -/
theorem HasType.weaken {unit : Validation.ValidatedUnit} {smaller larger : LoanTypes}
    (extends_ : larger.Extends smaller) {value : RuntimeValue} {type : SemTy} :
    HasType unit smaller value type → HasType unit larger value type
  | .unit => .unit
  | .bool value => .bool value
  | .character value valid => .character value valid
  | .string value => .string value
  | .bytes value => .bytes value
  | .integer value width signed fits => .integer value width signed fits
  | .address value => .address value
  | .signer value => .signer value
  | .tuple values types elements => .tuple values types (HasTypes.weaken extends_ elements)
  | .vector values element length length_matches elements =>
      .vector values element length length_matches (HasTypeEach.weaken extends_ elements)
  | .nominal source variant fields name arguments declaringNamespace declared fieldTypes
      name_eq declaration_eq fieldTypes_resolve fields_typed =>
      .nominal source variant fields name arguments declaringNamespace declared fieldTypes
        name_eq declaration_eq fieldTypes_resolve (HasTypes.weaken extends_ fields_typed)
  | .closure function mask instantiation captures parameters result targetNs declaration
      arguments allParameters results namespace_eq declaration_eq signature_eq faithful
      mask_bound captures_typed parameters_eq packs =>
      .closure function mask instantiation captures parameters result targetNs declaration
        arguments allParameters results namespace_eq declaration_eq signature_eq faithful
        mask_bound (HasTypes.weaken extends_ captures_typed) parameters_eq packs
  | .borrow loan current referent loan_eq current_typed =>
      .borrow loan current referent (extends_ loan referent loan_eq)
        (HasType.weaken extends_ current_typed)
  | .shared value referent value_typed => .shared value referent (HasType.weaken extends_ value_typed)
  | .dead referent => .dead referent
  | .hole loan referent type loan_eq same inhabited =>
      .hole loan referent type (extends_ loan referent loan_eq) same inhabited
  | .unitEmptyTuple => .unitEmptyTuple
  | .emptyTupleUnit => .emptyTupleUnit

theorem HasTypes.weaken {unit : Validation.ValidatedUnit} {smaller larger : LoanTypes}
    (extends_ : larger.Extends smaller) {values : List RuntimeValue} {types : List SemTy} :
    HasTypes unit smaller values types → HasTypes unit larger values types
  | .nil => .nil
  | .cons head tail => .cons (HasType.weaken extends_ head) (HasTypes.weaken extends_ tail)

theorem HasTypeEach.weaken {unit : Validation.ValidatedUnit} {smaller larger : LoanTypes}
    (extends_ : larger.Extends smaller) {values : List RuntimeValue} {type : SemTy} :
    HasTypeEach unit smaller values type → HasTypeEach unit larger values type
  | .nil => .nil
  | .cons head tail => .cons (HasType.weaken extends_ head) (HasTypeEach.weaken extends_ tail)
end

/-- A value's type is inhabited: not `never`, through shared references or
not. -/
theorem HasType.inhabited {unit : Validation.ValidatedUnit} {loans : LoanTypes}
    {value : RuntimeValue} : ∀ {type : SemTy}, HasType unit loans value type →
      type.unshared ≠ .never
  | _, .shared _ _ typed => by
      simp only [SemTy.unshared]
      exact HasType.inhabited typed
  | _, .hole _ _ _ _ same inhabited => same ▸ inhabited
  | _, .unit | _, .bool _ | _, .character _ _ | _, .string _ | _, .bytes _
  | _, .integer _ _ _ _ | _, .address _ | _, .signer _ | _, .tuple _ _ _
  | _, .vector _ _ _ _ _ | _, .nominal .. | _, .closure .. | _, .borrow .. | _, .dead _
  | _, .unitEmptyTuple | _, .emptyTupleUnit => by simp [SemTy.unshared]

end LeanerIR
