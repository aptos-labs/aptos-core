-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Operations
import LeanerIR.Validation.Profiles

/-!
# Runtime store, frame, and state typing

The M2 preservation/no-stuck groundwork over the M1 `ValueHasType` layer: a
`StateTyping` assigns types to heap slots, `TypedFrame`/`TypedState` relate a
declaration's local table and the shared stores to runtime storage, and the
leaf lemmas connect validation's literal checker and the frame constructor to
the typing judgment.
-/

namespace LeanerIR

open Validation

/-- Interpret a fixed vector length. The core deliberately accepts only a
nonnegative integer length; profile constants require profile typing rules. -/
def VectorLengthMatches (length : Option ConstValue) (size : Nat) : Prop :=
  match length with
  | none => True
  | some (.integer value) => value = Int.ofNat size
  | _ => False

/-- A runtime integer inhabits a core integer type exactly when the neutral
range decision can establish it. Pointer-sized values require target-profile
resolution before this judgment can be constructed. -/
def IntegerValueFits (width : IntWidth) (signed : Bool) (value : Int) : Prop :=
  Ty.integerValueFits? (.integer width signed) value = some true

mutual
  /-- Declarative M1 typing judgment for one value.  A `TypeId` is resolved
  in the namespace-local table named by the judgment. -/
  inductive ValueHasType (unit : ValidatedUnit) :
      Tables → RuntimeValue → TypeId → Prop where
    | unit {tables : Tables} (typeId : TypeId)
        (type_eq : tables.types[typeId.index]? = some .unit) :
        ValueHasType unit tables .unit typeId
    | bool {tables : Tables} (typeId : TypeId) (value : Bool)
        (type_eq : tables.types[typeId.index]? = some .bool) :
        ValueHasType unit tables (.bool value) typeId
    | character {tables : Tables} (typeId : TypeId) (value : Nat)
        (type_eq : tables.types[typeId.index]? = some .character)
        (value_valid : isUnicodeScalar value = true) :
        ValueHasType unit tables (.character value) typeId
    | string {tables : Tables} (typeId : TypeId) (value : String)
        (type_eq : tables.types[typeId.index]? = some .string) :
        ValueHasType unit tables (.string value) typeId
    | bytes {tables : Tables} (typeId : TypeId) (value : Array UInt8)
        (type_eq : tables.types[typeId.index]? = some .bytes) :
        ValueHasType unit tables (.bytes value) typeId
    | integer {tables : Tables} (typeId : TypeId) (value : Int) (width : IntWidth) (signed : Bool)
        (type_eq : tables.types[typeId.index]? = some (.integer width signed))
        (value_fits : IntegerValueFits width signed value) :
        ValueHasType unit tables (.integer value) typeId
    | address {tables : Tables} (typeId : TypeId) (value : String)
        (type_eq : tables.types[typeId.index]? = some .address) :
        ValueHasType unit tables (.address value) typeId
    | signer {tables : Tables} (typeId : TypeId) (value : String)
        (type_eq : tables.types[typeId.index]? = some .signer) :
        ValueHasType unit tables (.signer value) typeId
    | tuple {tables : Tables} (typeId : TypeId) (values : Array RuntimeValue) (types : Array TypeId)
        (type_eq : tables.types[typeId.index]? = some (.tuple types))
        (elements_typed : ValuesHaveTypes unit tables values.toList types.toList) :
        ValueHasType unit tables (.tuple values) typeId
    | vector {tables : Tables} (typeId elementType : TypeId) (values : Array RuntimeValue)
        (length : Option ConstValue)
        (type_eq : tables.types[typeId.index]? = some (.vector elementType length))
        (length_matches : VectorLengthMatches length values.size)
        (elements_typed : ValuesHaveType unit tables values.toList elementType) :
        ValueHasType unit tables (.vector values) typeId
    /-- A nominal value is typed against its declaration: the spelling the
    type names, the variant it declares, and every field at the type the
    declaration gives it under this use's generic arguments.  Field types are
    read in the declaring namespace, which is where they are interned. -/
    | nominal {tables : Tables} (typeId : TypeId) (nameId : NameId)
        (name : QualifiedName) (source : StructHandle) (variant : Option String)
        (fields : Array RuntimeValue) (arguments : Array GenericArgument)
        (declaringNamespace : ValidatedNamespace) (declared : Array FieldDecl)
        (fieldTypes : List TypeId)
        (type_eq : tables.types[typeId.index]? = some (.nominal nameId arguments))
        (name_eq : tables.names[nameId.index]? = some name)
        (source_eq : SemanticOperations.structName? unit source = some name)
        (declaration_eq : SemanticOperations.handleFields? unit source variant
          = some (declaringNamespace, declared))
        (fieldTypes_eq : declared.toList.mapM (fun field =>
            Validation.instantiatePlaceFieldType? declaringNamespace arguments
              field.type.typeId) = some fieldTypes)
        (fields_typed : ValuesHaveTypes unit declaringNamespace.tables
          fields.toList fieldTypes) :
        ValueHasType unit tables (.nominal source variant fields) typeId
    | closure {tables : Tables} (typeId : TypeId) (function : FunctionHandle)
        (captures : Array RuntimeValue) (arguments : Array TypeId) (result : TypeId)
        (abilities : Array Ability)
        (type_eq : tables.types[typeId.index]? = some (.function arguments result abilities)) :
        ValueHasType unit tables (.closure function captures) typeId
    | borrow {tables : Tables} (typeId : TypeId) (loan : Nat) (current : RuntimeValue)
        (referenceType : ReferenceType)
        (type_eq : tables.types[typeId.index]? = some (.reference referenceType))
        (kind_eq : referenceType.kind = .mutable)
        (current_typed : ValueHasType unit tables current referenceType.referent) :
        ValueHasType unit tables (.borrow loan current) typeId
    /-- A shared reference is transparent: certified exclusivity erases it
    to the observed value, so a value typed at the referent is typed at the
    shared reference type. -/
    | sharedReference {tables : Tables} (typeId : TypeId) (value : RuntimeValue)
        (referenceType : ReferenceType)
        (type_eq : tables.types[typeId.index]? = some (.reference referenceType))
        (kind_eq : referenceType.kind = .shared)
        (value_typed : ValueHasType unit tables value referenceType.referent) :
        ValueHasType unit tables value typeId
    /-- A hole stands for the value a live mutable loan took; it is typed
    wherever that value was, so store typing is stable across a loan's
    lifetime. -/
    | loanHole {tables : Tables} (typeId : TypeId) (loan : Nat) :
        ValueHasType unit tables (.loanHole loan) typeId
    /-- Packed-result equivalence: the empty tuple type and `unit` share the
    unit value, matching `packResults` and validation's `isUnitType`. -/
    | unitEmptyTuple {tables : Tables} (typeId : TypeId)
        (type_eq : tables.types[typeId.index]? = some (.tuple #[])) :
        ValueHasType unit tables .unit typeId
    | emptyTupleUnit {tables : Tables} (typeId : TypeId)
        (type_eq : tables.types[typeId.index]? = some .unit) :
        ValueHasType unit tables (.tuple #[]) typeId

  /-- Pointwise M1 typing for parallel value and type-ID lists. -/
  inductive ValuesHaveTypes (unit : ValidatedUnit) :
      Tables → List RuntimeValue → List TypeId → Prop where
    | nil {tables : Tables} : ValuesHaveTypes unit tables [] []
    | cons {tables : Tables} (value : RuntimeValue) (typeId : TypeId)
        (values : List RuntimeValue) (typeIds : List TypeId)
        (head_typed : ValueHasType unit tables value typeId)
        (tail_typed : ValuesHaveTypes unit tables values typeIds) :
        ValuesHaveTypes unit tables (value :: values) (typeId :: typeIds)

  /-- Homogeneous pointwise typing used by vectors. -/
  inductive ValuesHaveType (unit : ValidatedUnit) :
      Tables → List RuntimeValue → TypeId → Prop where
    | nil {tables : Tables} (typeId : TypeId) : ValuesHaveType unit tables [] typeId
    | cons {tables : Tables} (value : RuntimeValue) (values : List RuntimeValue)
        (typeId : TypeId)
        (head_typed : ValueHasType unit tables value typeId)
        (tail_typed : ValuesHaveType unit tables values typeId) :
        ValuesHaveType unit tables (value :: values) typeId
end


namespace SemanticTyping

open Validation
open SemanticOperations

variable {unit : ValidatedUnit}

/-- Frame typing against one declaration's local table: every initialized
local holds a value of its declared type. Under prophetic ownership there is
no heap; borrows and holes are values and type structurally. -/
def TypedFrame (unit : ValidatedUnit) (tables : Tables)
    (locals : Array LocalDecl) (frame : RuntimeFrame) : Prop :=
  frame.locals.size = locals.size ∧
    ∀ (index : Nat) (declaration : LocalDecl), locals[index]? = some declaration →
      ∀ value, frame.locals[index]? = some (some value) →
        ValueHasType unit tables value declaration.type.typeId

/-- State typing: global memory is well formed.  Every published resource
holds a value of the resource type its key names, and holds a value rather
than the hole a live mutable loan left behind — borrow discipline admits one
mutable borrow of global memory at a time and forbids every other access
while it is out, so no caller can present a state holding one.

This is what makes a program that reads storage meaningful: without it a
resource read back out of memory has no known shape, and selecting a field
of it is not a defined operation. -/
def TypedState (unit : ValidatedUnit) (state : RuntimeState) : Prop :=
  ∀ (key : GlobalKey) (value : RuntimeValue), state.globals.lookup key = some value →
    ∃ ns : ValidatedNamespace,
      unit.namespaces[key.namespaceId.index]? = some ns ∧
      ValueHasType unit ns.tables value key.typeId ∧
      ∀ loan, value ≠ .loanHole loan

/-- Inversion at a nominal type: a typed value that is not a hole is the
nominal value of the declared spelling.  This is the whole reason state
typing is assumed — it is what gives a resource read out of memory the shape
a field selection needs. -/
theorem nominal_of_valueHasType {tables : Tables} {value : RuntimeValue}
    {typeId : TypeId} {nameId : NameId} {arguments : Array GenericArgument}
    {name : QualifiedName}
    (typed : ValueHasType unit tables value typeId)
    (type_eq : tables.types[typeId.index]? = some (.nominal nameId arguments))
    (name_eq : tables.names[nameId.index]? = some name)
    (present : ∀ loan, value ≠ .loanHole loan) :
    ∃ source variant fields, value = .nominal source variant fields ∧
      SemanticOperations.structName? unit source = some name := by
  cases typed with
  | nominal _ actualNameId actualName source variant fields _ _ _ _
      actualType_eq actualName_eq source_eq _ _ _ =>
      rw [type_eq, Option.some.injEq] at actualType_eq
      cases actualType_eq
      rw [name_eq, Option.some.injEq] at actualName_eq
      cases actualName_eq
      exact ⟨source, variant, fields, rfl, source_eq⟩
  | loanHole _ loan => exact absurd rfl (present loan)
  | _ => simp_all

/-- A declaration with no variants has one payload, and a value of it carries
no variant.  This is what lets a resource read out of memory be matched
against the fields its declaration lists. -/
theorem handleFields?_of_structDecl {source : StructHandle}
    {variant : Option String} {declaringNamespace target : ValidatedNamespace}
    {declaration : StructDecl} {declared : Array FieldDecl}
    (namespace_eq : unit.namespaces[source.namespaceId.index]?
      = some declaringNamespace)
    (decl_eq : declaringNamespace.structs[source.structId]? = some declaration)
    (no_variants : declaration.variants.isEmpty = true)
    (found : handleFields? unit source variant = some (target, declared)) :
    variant = none ∧ target = declaringNamespace ∧ declared = declaration.fields := by
  cases variant with
  | none =>
      simp [handleFields?, namespace_eq, decl_eq] at found
      exact ⟨rfl, found.2.1.symm, found.2.2.symm⟩
  | some variantName =>
      rw [Array.isEmpty_iff] at no_variants
      simp [handleFields?, namespace_eq, decl_eq, no_variants] at found

/-- Inversion of pointwise typing at a nonempty type row. -/
theorem valuesHaveTypes_cons {tables : Tables} {values : List RuntimeValue}
    {typeId : TypeId} {typeIds : List TypeId}
    (typed : ValuesHaveTypes unit tables values (typeId :: typeIds)) :
    ∃ value rest, values = value :: rest ∧
      ValueHasType unit tables value typeId ∧
      ValuesHaveTypes unit tables rest typeIds := by
  cases typed with
  | cons value _ rest _ head_typed tail_typed =>
      exact ⟨value, rest, rfl, head_typed, tail_typed⟩

/-- Inversion of pointwise typing at an exhausted type row. -/
theorem valuesHaveTypes_nil {tables : Tables} {values : List RuntimeValue}
    (typed : ValuesHaveTypes unit tables values []) : values = [] := by
  cases typed with
  | nil => rfl


/-- Reading an initialized local yields a value of its declared type. -/
theorem readLocal?_typed {tables : Tables}
    {locals : Array LocalDecl} {frame : RuntimeFrame}
    {localId : LocalId} {declaration : LocalDecl} {value : RuntimeValue}
    (frame_typed : TypedFrame unit tables locals frame)
    (declaration_eq : locals[localId.index]? = some declaration)
    (read_eq : readLocal? frame localId = some value) :
    ValueHasType unit tables value declaration.type.typeId := by
  obtain ⟨-, entries⟩ := frame_typed
  simp only [readLocal?] at read_eq
  rw [Option.join_eq_some_iff] at read_eq
  exact entries localId.index declaration declaration_eq value read_eq

/-- The initial frame of a call is typed when the arguments are typed at the
declared parameter types, which occupy the leading locals. -/
theorem initialFrame?_typed {tables : Tables}
    {declaration : FunctionDecl Validation.FunctionBody}
    {arguments : Array RuntimeValue} {frame : RuntimeFrame}
    (locals_parameters : ∀ (index : Nat) (parameter : Parameter),
      declaration.signature.parameters[index]? = some parameter →
      ∃ localDecl : LocalDecl, declaration.locals[index]? = some localDecl ∧
        localDecl.type.typeId = parameter.typeUse.typeId)
    (arguments_typed : ∀ (index : Nat) (argument : RuntimeValue)
        (parameter : Parameter),
      arguments[index]? = some argument →
      declaration.signature.parameters[index]? = some parameter →
      ValueHasType unit tables argument parameter.typeUse.typeId)
    (frame_eq : initialFrame? declaration arguments = some frame) :
    TypedFrame unit tables declaration.locals frame := by
  simp only [initialFrame?] at frame_eq
  cases arity_eq : arguments.size != declaration.signature.parameters.size with
  | true => simp [arity_eq] at frame_eq
  | false =>
    simp only [arity_eq, Bool.false_eq_true, if_false] at frame_eq
    cases bound_eq : decide (declaration.locals.size < arguments.size) with
    | true => simp [of_decide_eq_true bound_eq] at frame_eq
    | false =>
      have bound := of_decide_eq_false bound_eq
      simp only [if_neg bound, Option.some.injEq] at frame_eq
      subst frame_eq
      have arity : arguments.size = declaration.signature.parameters.size := by
        simpa using arity_eq
      refine ⟨by simp [SemanticOperations.initialLocals], ?_⟩
      intro index localDecl local_eq value value_eq
      rw [SemanticOperations.initialLocals, Array.getElem?_ofFn] at value_eq
      split at value_eq
      case isTrue index_lt_locals =>
          rw [Option.some.injEq] at value_eq
          split at value_eq
          case isTrue index_lt_arguments =>
              rw [Option.some.injEq] at value_eq
              have parameter_lt : index < declaration.signature.parameters.size :=
                arity ▸ index_lt_arguments
              obtain ⟨parameter, parameter_eq⟩ :
                  ∃ parameter,
                    declaration.signature.parameters[index]? = some parameter :=
                ⟨_, Array.getElem?_eq_getElem parameter_lt⟩
              obtain ⟨localDecl', local_eq', type_eq⟩ :=
                locals_parameters index parameter parameter_eq
              rw [local_eq] at local_eq'
              cases local_eq'
              rw [type_eq, ← value_eq]
              exact arguments_typed index _ parameter
                (Array.getElem?_eq_getElem index_lt_arguments) parameter_eq
          case isFalse => simp at value_eq
      case isFalse => simp at value_eq

/-! ## Reference vocabulary

Leaf lemmas of the prophetic ownership operations
([`designs/prophetic-references.md`](../../../designs/prophetic-references.md)):
dereference inversion, borrow introduction, freeze, and the hole writes.
The walker-level preservation statements — write-back and mutation through
a live borrow — need a loan-typing environment tying each hole's position
to its loan's referent (a hole types at every type on its own); that
environment is recorded in the LIR design's deferred-work register. -/

/-- Dereference: a borrow typed at a mutable reference type observes a
current value typed at the referent. -/
theorem borrow_current_typed {tables : Tables} {loan : Nat}
    {current : RuntimeValue} {typeId : TypeId}
    {referenceType : ReferenceType}
    (borrow_typed : ValueHasType unit tables (.borrow loan current) typeId)
    (type_eq : tables.types[typeId.index]? = some (.reference referenceType))
    (kind_eq : referenceType.kind = .mutable) :
    ValueHasType unit tables current referenceType.referent := by
  cases borrow_typed with
  | borrow _ _ _ derived derived_eq _ current_typed =>
      rw [type_eq, Option.some.injEq, Ty.reference.injEq] at derived_eq
      rw [derived_eq]
      exact current_typed
  | sharedReference _ _ derived derived_eq derived_kind _ =>
      rw [type_eq, Option.some.injEq, Ty.reference.injEq] at derived_eq
      rw [← derived_eq, kind_eq] at derived_kind
      cases derived_kind

/-- Borrow introduction: borrowing a value typed at the referent of a
mutable reference type yields a typed borrow, for any loan instance. -/
theorem borrow_value_typed {tables : Tables} {loan : Nat}
    {value : RuntimeValue} {typeId : TypeId} {referenceType : ReferenceType}
    (type_eq : tables.types[typeId.index]? = some (.reference referenceType))
    (kind_eq : referenceType.kind = .mutable)
    (value_typed : ValueHasType unit tables value referenceType.referent) :
    ValueHasType unit tables (.borrow loan value) typeId :=
  .borrow typeId loan value referenceType type_eq kind_eq value_typed

/-- Shared-reference introduction: certified exclusivity erases the
reference, so the observed value itself is typed at the shared type. -/
theorem shared_value_typed {tables : Tables} {value : RuntimeValue}
    {typeId : TypeId} {referenceType : ReferenceType}
    (type_eq : tables.types[typeId.index]? = some (.reference referenceType))
    (kind_eq : referenceType.kind = .shared)
    (value_typed : ValueHasType unit tables value referenceType.referent) :
    ValueHasType unit tables value typeId :=
  .sharedReference typeId value referenceType type_eq kind_eq value_typed

/-- Shared-reference inversion: a value typed at a shared reference type is
typed at the referent — the erased reference is transparent both ways. -/
theorem shared_referent_typed {tables : Tables} {value : RuntimeValue}
    {typeId : TypeId} {referenceType : ReferenceType}
    (typed : ValueHasType unit tables value typeId)
    (type_eq : tables.types[typeId.index]? = some (.reference referenceType))
    (kind_eq : referenceType.kind = .shared) :
    ValueHasType unit tables value referenceType.referent := by
  cases typed
  case sharedReference derived derived_kind value_typed derived_eq =>
    rw [type_eq, Option.some.injEq, Ty.reference.injEq] at derived_eq
    rw [derived_eq]
    exact value_typed
  case borrow loan current derived derived_kind current_typed derived_eq =>
    rw [type_eq, Option.some.injEq, Ty.reference.injEq] at derived_eq
    rw [← derived_eq, kind_eq] at derived_kind
    cases derived_kind
  case loanHole loan => exact .loanHole referenceType.referent loan
  all_goals simp_all

/-- Freeze: consuming a mutable borrow yields its current value, typed at
any shared reference type with the same referent. -/
theorem freeze_typed {tables : Tables} {loan : Nat}
    {current : RuntimeValue} {sourceId targetId : TypeId}
    {source target : ReferenceType}
    (source_typed : ValueHasType unit tables (.borrow loan current) sourceId)
    (source_eq : tables.types[sourceId.index]? = some (.reference source))
    (source_kind : source.kind = .mutable)
    (target_eq : tables.types[targetId.index]? = some (.reference target))
    (referent_eq : target.referent = source.referent)
    (kind_eq : target.kind = .shared) :
    ValueHasType unit tables current targetId :=
  .sharedReference targetId current target target_eq kind_eq
    (referent_eq ▸ borrow_current_typed source_typed source_eq source_kind)

/-- Leaving a hole in an initialized local preserves frame typing: a hole
is typed wherever the taken value was. -/
theorem typedFrame_set_loanHole {tables : Tables} {locals : Array LocalDecl}
    {frame : RuntimeFrame} {index loan : Nat}
    (frame_typed : TypedFrame unit tables locals frame) :
    TypedFrame unit tables locals
      { frame with locals := frame.locals.set! index (some (.loanHole loan)) } := by
  obtain ⟨size_eq, entries⟩ := frame_typed
  refine ⟨by simpa using size_eq, ?_⟩
  intro slot declaration declaration_eq value value_eq
  by_cases slot_eq : slot = index
  · subst slot_eq
    rw [Array.set!, Array.getElem?_setIfInBounds_self] at value_eq
    split at value_eq
    · rw [Option.some.injEq, Option.some.injEq] at value_eq
      rw [← value_eq]
      exact .loanHole declaration.type.typeId loan
    · cases value_eq
  · rw [Array.set!, Array.getElem?_setIfInBounds_ne (by omega)] at value_eq
    exact entries slot declaration declaration_eq value value_eq

/-- Taking a resource out preserves state typing. -/
theorem typedState_erase {unit : ValidatedUnit} {state : RuntimeState}
    {key : GlobalKey} (state_typed : TypedState unit state) :
    TypedState unit { state with globals := state.globals.erase key } := by
  intro query value lookup_eq
  by_cases hit : query = key
  · subst hit; rw [GlobalMap.lookup_erase_self] at lookup_eq; cases lookup_eq
  · rw [GlobalMap.lookup_erase_other _ _ _ hit] at lookup_eq
    exact state_typed query value lookup_eq

/-- Publishing a well-typed resource preserves state typing. -/
theorem typedState_insert {unit : ValidatedUnit} {state : RuntimeState}
    {key : GlobalKey} {value : RuntimeValue} {ns : ValidatedNamespace}
    (state_typed : TypedState unit state)
    (ns_eq : unit.namespaces[key.namespaceId.index]? = some ns)
    (value_typed : ValueHasType unit ns.tables value key.typeId)
    (present : ∀ loan, value ≠ .loanHole loan) :
    TypedState unit { state with globals := state.globals.insert key value } := by
  intro query published lookup_eq
  by_cases hit : query = key
  · subst hit
    rw [GlobalMap.lookup_insert_self] at lookup_eq
    cases lookup_eq
    exact ⟨ns, ns_eq, value_typed, present⟩
  · rw [GlobalMap.lookup_insert_other _ _ _ _ hit] at lookup_eq
    exact state_typed query published lookup_eq

/-! ## Literal typing -/

private theorem mapM_cons_eq_some {α β : Type _} {f : α → Option β} {x : α}
    {xs : List α} {ys : List β} :
    (x :: xs).mapM f = some ys ↔
      ∃ y ys', f x = some y ∧ xs.mapM f = some ys' ∧ ys = y :: ys' := by
  simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
    Option.pure_def, Option.some.injEq]
  constructor
  · rintro ⟨y, hy, ys', hys, rfl⟩
    exact ⟨y, ys', hy, hys, rfl⟩
  · rintro ⟨y, ys', hy, hys, rfl⟩
    exact ⟨y, hy, ys', hys, rfl⟩

private theorem attachWith_mapM {α β : Type _} {f : α → Option β} :
    ∀ {xs : List α} {P : α → Prop} {H : ∀ x ∈ xs, P x},
    (xs.attachWith P H).mapM (fun x => f x.val) = xs.mapM f := by
  intro xs
  induction xs with
  | nil => intro P H; rfl
  | cons x xs ih =>
      intro P H
      simp only [List.attachWith, List.pmap, List.mapM_cons]
      rw [show (xs.pmap Subtype.mk _) = xs.attachWith P (fun a h => H a (by simp [h])) from rfl,
        ih]

/-- Reifying an attached array of literals is reifying its list. -/
private theorem constValues?_toList {elements : Array ConstValue}
    {out : Array RuntimeValue}
    (h : elements.attach.mapM (fun ⟨element, _⟩ => constValue? element) = some out) :
    elements.toList.mapM constValue? = some out.toList := by
  rw [Array.mapM_eq_mapM_toList, Array.toList_attach, attachWith_mapM] at h
  cases m : elements.toList.mapM constValue? with
  | none => simp [m] at h
  | some ys =>
      rw [m] at h
      have out_def : ys.toArray = out := by simpa using h
      subst out_def
      simp

private theorem combineFirstSliceChecks_true {left right : Option Bool}
    (h : combineFirstSliceChecks left right = some true) :
    left = some true ∧ right = some true := by
  cases left with
  | none =>
      exfalso
      cases right with
      | none => simp [combineFirstSliceChecks] at h
      | some r => cases r <;> simp [combineFirstSliceChecks] at h
  | some l =>
      cases l with
      | false =>
          exfalso
          cases right with
          | none => simp [combineFirstSliceChecks] at h
          | some r => cases r <;> simp [combineFirstSliceChecks] at h
      | true =>
          cases right with
          | none => exact absurd h (by simp [combineFirstSliceChecks])
          | some r =>
              cases r with
              | false => exact absurd h (by simp [combineFirstSliceChecks])
              | true => exact ⟨rfl, rfl⟩

private theorem mapM_constValue?_length :
    ∀ {literals : List ConstValue} {values : List RuntimeValue},
    literals.mapM constValue? = some values → values.length = literals.length := by
  intro literals
  induction literals with
  | nil => intro values h; cases h; rfl
  | cons literal literals ih =>
      intro values h
      rw [mapM_cons_eq_some] at h
      obtain ⟨value, values', -, tail_eq, rfl⟩ := h
      simpa using ih tail_eq

/-- Values reified from checker-approved literals are well typed. -/
theorem constValue?_hasType {tables : Tables} {literal : ConstValue}
    {typeId : TypeId} {value : RuntimeValue}
    (matches_eq : firstSliceConstMatchesType tables literal typeId = some true)
    (value_eq : constValue? literal = some value) :
    ValueHasType unit tables value typeId := by
  suffices general : ∀ (fuel : Nat) (literal : ConstValue), sizeOf literal ≤ fuel →
      ∀ {typeId : TypeId} {value : RuntimeValue},
        firstSliceConstMatchesType tables literal typeId = some true →
        constValue? literal = some value →
        ValueHasType unit tables value typeId from
    general (sizeOf literal) literal (Nat.le_refl _) matches_eq value_eq
  intro fuel
  induction fuel with
  | zero =>
      intro literal size_le
      cases literal <;> simp at size_le
  | succ fuel ih =>
      intro literal size_le typeId value matches_eq value_eq
      rw [firstSliceConstMatchesType] at matches_eq
      cases ty_eq : tables.types[typeId.index]? with
      | none => simp [ty_eq] at matches_eq
      | some ty =>
          simp only [ty_eq] at matches_eq
          cases literal with
          | unit =>
              simp only [constValue?, Option.some.injEq] at value_eq
              subst value_eq
              cases ty <;> simp_all [isFirstSliceConst, isFirstSliceType]
              case unit => exact .unit typeId ty_eq
              case tuple types =>
                  have : types = #[] := by
                    cases types with
                    | mk l => cases l <;> simp_all
                  subst this
                  exact .unitEmptyTuple typeId ty_eq
          | bool b =>
              simp only [constValue?, Option.some.injEq] at value_eq
              subst value_eq
              cases ty <;> simp_all [isFirstSliceConst, isFirstSliceType]
              exact .bool typeId b ty_eq
          | character c =>
              simp only [constValue?] at value_eq
              cases valid : isUnicodeScalar c with
              | false => simp [valid] at value_eq
              | true =>
                  simp only [valid, if_true, Option.some.injEq] at value_eq
                  subst value_eq
                  cases ty <;> simp_all [isFirstSliceConst, isFirstSliceType]
                  exact .character typeId c ty_eq valid
          | integer i =>
              simp only [constValue?, Option.some.injEq] at value_eq
              subst value_eq
              cases ty <;> simp_all [isFirstSliceConst, isFirstSliceType]
              case integer width signed =>
                exact .integer typeId i width signed ty_eq matches_eq
          | address a =>
              simp only [constValue?, Option.some.injEq] at value_eq
              subst value_eq
              cases ty <;> simp_all [isFirstSliceConst, isFirstSliceType]
              exact .address typeId a ty_eq
          | string str =>
              simp only [constValue?, Option.some.injEq] at value_eq
              subst value_eq
              cases ty <;> simp_all [isFirstSliceConst, isFirstSliceType]
              exact .string typeId str ty_eq
          | bytes bs =>
              simp only [constValue?, Option.some.injEq] at value_eq
              subst value_eq
              cases ty <;> simp_all [isFirstSliceConst, isFirstSliceType]
              exact .bytes typeId bs ty_eq
          | profile p => simp [constValue?] at value_eq
          | vector elements =>
              simp only [constValue?] at value_eq
              cases out_eq : Array.mapM (fun x => constValue? x.val) elements.attach with
              | none => simp [out_eq] at value_eq
              | some out =>
              rw [out_eq] at value_eq
              have value_def : RuntimeValue.vector out = value := by simpa using value_eq
              subst value_def
              have list_eq := constValues?_toList out_eq
              have member_size : ∀ member ∈ elements.toList, sizeOf member ≤ fuel := by
                intro member member_in
                have member_lt := List.sizeOf_lt_of_mem member_in
                have list_lt : sizeOf elements.toList <
                    sizeOf (ConstValue.vector elements) := by
                  cases elements with
                  | mk l => simp; omega
                omega
              have all_typed : ∀ (vs : List ConstValue),
                  (∀ m ∈ vs, sizeOf m ≤ fuel) →
                  ∀ (elementType : TypeId) (os : List RuntimeValue),
                  firstSliceAllMatch tables vs elementType = some true →
                  vs.mapM constValue? = some os →
                  ValuesHaveType unit tables os elementType := by
                intro vs
                induction vs with
                | nil =>
                    intro _ elementType os _ mapm_eq
                    cases mapm_eq
                    exact .nil elementType
                | cons v vs inner_ih =>
                    intro bounds elementType os all_eq mapm_eq
                    rw [mapM_cons_eq_some] at mapm_eq
                    obtain ⟨o, os', o_eq, tail_eq, rfl⟩ := mapm_eq
                    simp only [firstSliceAllMatch] at all_eq
                    obtain ⟨head_true, tail_true⟩ := combineFirstSliceChecks_true all_eq
                    exact .cons o os' elementType
                      (ih v (bounds v (by simp)) head_true o_eq)
                      (inner_ih (fun m hm => bounds m (by simp [hm])) elementType os'
                        tail_true tail_eq)
              cases ty
              case vector elementType length =>
                dsimp only at matches_eq
                cases length_bool :
                    firstSliceVectorLengthMatches length elements.size with
                | false => rw [length_bool] at matches_eq; simp at matches_eq
                | true =>
                    rw [length_bool] at matches_eq
                    simp only [Bool.not_true, Bool.false_eq_true, if_false] at matches_eq
                    have sizes : out.size = elements.size := by
                      have := mapM_constValue?_length list_eq
                      simpa using this
                    refine .vector typeId elementType out length ty_eq ?_
                      (all_typed elements.toList member_size elementType out.toList
                        matches_eq list_eq)
                    cases length with
                    | none => trivial
                    | some c =>
                        cases c
                        case integer expected =>
                            simp only [firstSliceVectorLengthMatches] at length_bool
                            simp only [VectorLengthMatches, sizes]
                            exact eq_of_beq length_bool
                        all_goals
                          simp [firstSliceVectorLengthMatches] at length_bool
              all_goals dsimp only at matches_eq
              all_goals simp [isFirstSliceConst, isFirstSliceType] at matches_eq
          | tuple elements =>
              simp only [constValue?] at value_eq
              cases out_eq : Array.mapM (fun x => constValue? x.val) elements.attach with
              | none => simp [out_eq] at value_eq
              | some out =>
              rw [out_eq] at value_eq
              have value_def : RuntimeValue.tuple out = value := by simpa using value_eq
              subst value_def
              have list_eq := constValues?_toList out_eq
              have member_size : ∀ member ∈ elements.toList, sizeOf member ≤ fuel := by
                intro member member_in
                have member_lt := List.sizeOf_lt_of_mem member_in
                have list_lt : sizeOf elements.toList <
                    sizeOf (ConstValue.tuple elements) := by
                  cases elements with
                  | mk l => simp; omega
                omega
              have pairs_typed : ∀ (vs : List ConstValue),
                  (∀ m ∈ vs, sizeOf m ≤ fuel) →
                  ∀ (ts : List TypeId) (os : List RuntimeValue),
                  firstSlicePairsMatch tables vs ts = some true →
                  vs.mapM constValue? = some os →
                  ValuesHaveTypes unit tables os ts := by
                intro vs
                induction vs with
                | nil =>
                    intro _ ts os pairs_eq mapm_eq
                    cases mapm_eq
                    cases ts with
                    | nil => exact .nil
                    | cons t ts => simp [firstSlicePairsMatch] at pairs_eq
                | cons v vs inner_ih =>
                    intro bounds ts os pairs_eq mapm_eq
                    rw [mapM_cons_eq_some] at mapm_eq
                    obtain ⟨o, os', o_eq, tail_eq, rfl⟩ := mapm_eq
                    cases ts with
                    | nil => simp [firstSlicePairsMatch] at pairs_eq
                    | cons t ts =>
                        simp only [firstSlicePairsMatch] at pairs_eq
                        obtain ⟨head_true, tail_true⟩ :=
                          combineFirstSliceChecks_true pairs_eq
                        exact .cons o t os' ts
                          (ih v (bounds v (by simp)) head_true o_eq)
                          (inner_ih (fun m hm => bounds m (by simp [hm])) ts os'
                            tail_true tail_eq)
              cases ty
              case unit =>
                  dsimp only at matches_eq
                  have empty : elements = #[] := by
                    cases elements with
                    | mk l => cases l <;> simp_all
                  subst empty
                  have : out = #[] := by
                    cases out with
                    | mk l =>
                        cases l with
                        | nil => rfl
                        | cons o os => simp_all
                  subst this
                  exact .emptyTupleUnit typeId ty_eq
              case tuple types =>
                  dsimp only at matches_eq
                  cases size_bool : elements.size != types.size with
                  | true => rw [size_bool] at matches_eq; simp at matches_eq
                  | false =>
                      rw [size_bool] at matches_eq
                      simp only [Bool.false_eq_true, if_false] at matches_eq
                      exact .tuple typeId out types ty_eq
                        (pairs_typed elements.toList member_size types.toList out.toList
                          matches_eq list_eq)
              all_goals dsimp only at matches_eq
              all_goals simp [isFirstSliceConst, isFirstSliceType] at matches_eq

/-! ## Primitive operation typing -/

private theorem integerBounds_shape {ty : Ty} {lower upper : Int}
    (bounds_eq : ty.integerBounds? = some (lower, upper)) :
    ∃ width signed, ty = .integer (.bits width) signed ∧ width ≠ 0 := by
  cases ty
  case integer width signed =>
      cases width
      case bits width =>
          refine ⟨width, signed, rfl, fun h => ?_⟩
          subst h
          simp [Ty.integerBounds?] at bounds_eq
      all_goals simp [Ty.integerBounds?] at bounds_eq
  all_goals simp [Ty.integerBounds?] at bounds_eq

private theorem integerValueFits_of_bounds {width : Nat} {signed : Bool}
    {value lower upper : Int}
    (bounds_eq : (Ty.integer (.bits width) signed).integerBounds? = some (lower, upper))
    (lower_le : lower ≤ value) (le_upper : value ≤ upper) :
    IntegerValueFits (.bits width) signed value := by
  simp only [IntegerValueFits, Ty.integerValueFits?, bounds_eq, bind, Option.bind,
    Option.some.injEq]
  simp [lower_le, le_upper]

/-- A checked-integer success is typed at the checked type. -/
theorem checkedInteger_ok_typed {tables : Tables} {failure : ThrowKind} {ty : Ty}
    {value : Int} {out : RuntimeValue} {typeId : TypeId}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : checkedInteger failure ty value = .ok out) :
    ValueHasType unit tables out typeId := by
  unfold checkedInteger at ok_eq
  cases bounds_eq : ty.integerBounds? with
  | none => simp [bounds_eq] at ok_eq
  | some bounds =>
      obtain ⟨lower, upper⟩ := bounds
      obtain ⟨width, signed, ty_def, -⟩ := integerBounds_shape bounds_eq
      subst ty_def
      simp only [bounds_eq] at ok_eq
      cases in_range : lower ≤ value && value ≤ upper with
      | false => simp [in_range] at ok_eq
      | true =>
          simp only [in_range, if_true, Except.ok.injEq] at ok_eq
          subst ok_eq
          simp only [Bool.and_eq_true, decide_eq_true_eq] at in_range
          exact .integer typeId value (.bits width) signed type_eq
            (integerValueFits_of_bounds bounds_eq in_range.1 in_range.2)

/-- A modular wrap is typed at the fixed-width type it wraps into. -/
theorem modularInteger_typed {tables : Tables} {ty : Ty} {value : Int}
    {out : RuntimeValue} {typeId : TypeId}
    (type_eq : tables.types[typeId.index]? = some ty)
    (mod_eq : modularInteger ty value = some out) :
    ValueHasType unit tables out typeId := by
  unfold modularInteger at mod_eq
  cases ty
  case integer width signed =>
    cases width
    case bits width =>
      cases zero : width == 0 with
      | true => simp [zero] at mod_eq
      | false =>
          have width_pos : 0 < width := Nat.pos_of_ne_zero (by simpa using zero)
          simp only [zero, Bool.false_eq_true, if_false, Option.some.injEq] at mod_eq
          have two_pow_pos : ∀ n : Nat, (0 : Int) < (2 : Int) ^ n := by
            intro n
            induction n with
            | zero => simp
            | succ m ih => rw [Int.pow_succ]; omega
          have modulus_pos : (0 : Int) < (2 : Int) ^ width := two_pow_pos width
          have residue_nonneg :
              0 ≤ ((value % (2 : Int) ^ width) + (2 : Int) ^ width) % (2 : Int) ^ width :=
            Int.emod_nonneg _ (by omega)
          have residue_lt :
              ((value % (2 : Int) ^ width) + (2 : Int) ^ width) % (2 : Int) ^ width
                < (2 : Int) ^ width :=
            Int.emod_lt_of_pos _ modulus_pos
          have split_pow : (2 : Int) ^ width = 2 * (2 : Int) ^ (width - 1) := by
            cases width with
            | zero => omega
            | succ m => rw [Int.pow_succ]; simp; omega
          rw [← mod_eq]
          refine .integer typeId _ (.bits width) signed type_eq ?_
          cases signed with
          | false =>
              simp only [Bool.false_and, Bool.false_eq_true, if_false]
              apply integerValueFits_of_bounds
                (lower := 0) (upper := (2 : Int) ^ width - 1)
                (bounds_eq := by
                  simp [Ty.integerBounds?, Nat.pos_iff_ne_zero.mp width_pos])
              · exact residue_nonneg
              · omega
          | true =>
              simp only [Bool.true_and]
              by_cases big :
                  ((value % (2 : Int) ^ width) + (2 : Int) ^ width) % (2 : Int) ^ width
                    ≥ (2 : Int) ^ (width - 1)
              · rw [if_pos (decide_eq_true big)]
                apply integerValueFits_of_bounds
                  (lower := -(2 : Int) ^ (width - 1))
                  (upper := (2 : Int) ^ (width - 1) - 1)
                  (bounds_eq := by
                    simp [Ty.integerBounds?, Nat.pos_iff_ne_zero.mp width_pos])
                · omega
                · omega
              · rw [if_neg (by simpa using big)]
                apply integerValueFits_of_bounds
                  (lower := -(2 : Int) ^ (width - 1))
                  (upper := (2 : Int) ^ (width - 1) - 1)
                  (bounds_eq := by
                    simp [Ty.integerBounds?, Nat.pos_iff_ne_zero.mp width_pos])
                · omega
                · omega
    all_goals simp at mod_eq
  all_goals simp at mod_eq

/-- Successful helper applications are typed at the fixed-width node. -/
private theorem modularWrapped_ok_typed {tables : Tables} {ty : Ty} {typeId : TypeId}
    {value : RuntimeValue} {wrapped : Option RuntimeValue}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : (Except.ok (ε := ThrowKind × Array RuntimeValue)) <$> wrapped
      = some (.ok value))
    (wrapped_eq : ∀ out, wrapped = some out → ∃ raw, modularInteger ty raw = some out) :
    ValueHasType unit tables value typeId := by
  cases w : wrapped with
  | none => rw [w] at ok_eq; simp at ok_eq
  | some out =>
      rw [w] at ok_eq
      have : out = value := by simpa using ok_eq
      subst this
      obtain ⟨raw, raw_eq⟩ := wrapped_eq _ w
      exact modularInteger_typed type_eq raw_eq

private theorem modularBinaryInteger_ok_typed {tables : Tables} {ty : Ty}
    {typeId : TypeId} {arguments : Array RuntimeValue} {value : RuntimeValue}
    {operation : Int → Int → Int}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : modularBinaryInteger ty arguments operation = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  unfold modularBinaryInteger at ok_eq
  split at ok_eq
  · exact modularWrapped_ok_typed type_eq ok_eq (fun out h => ⟨_, h⟩)
  · cases ok_eq

private theorem modularUnaryInteger_ok_typed {tables : Tables} {ty : Ty}
    {typeId : TypeId} {arguments : Array RuntimeValue} {value : RuntimeValue}
    {operation : Int → Int}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : modularUnaryInteger ty arguments operation = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  unfold modularUnaryInteger at ok_eq
  split at ok_eq
  · exact modularWrapped_ok_typed type_eq ok_eq (fun out h => ⟨_, h⟩)
  · cases ok_eq

private theorem checkedBinaryInteger_ok_typed {tables : Tables} {ty : Ty}
    {typeId : TypeId} {failure : ThrowKind} {arguments : Array RuntimeValue}
    {value : RuntimeValue} {operation : Int → Int → Int}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : checkedBinaryInteger failure ty arguments operation = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  unfold checkedBinaryInteger at ok_eq
  split at ok_eq
  · have := Option.some.inj ok_eq
    exact checkedInteger_ok_typed type_eq this
  · cases ok_eq

private theorem checkedUnaryInteger_ok_typed {tables : Tables} {ty : Ty}
    {typeId : TypeId} {failure : ThrowKind} {arguments : Array RuntimeValue}
    {value : RuntimeValue} {operation : Int → Int}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : checkedUnaryInteger failure ty arguments operation = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  unfold checkedUnaryInteger at ok_eq
  split at ok_eq
  · have := Option.some.inj ok_eq
    exact checkedInteger_ok_typed type_eq this
  · cases ok_eq

private theorem bitwiseBinaryInteger_ok_typed {tables : Tables} {ty : Ty}
    {typeId : TypeId} {arguments : Array RuntimeValue} {value : RuntimeValue}
    {operation : Nat → Nat → Nat}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : bitwiseBinaryInteger ty arguments operation = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  unfold bitwiseBinaryInteger at ok_eq
  split at ok_eq
  · simp only [bind, Option.bind_eq_some_iff, Option.some.injEq] at ok_eq
    obtain ⟨leftBits, -, rightBits, -, out, wrap_eq, out_def⟩ := ok_eq
    have : out = value := by simpa using out_def
    subst this
    exact modularInteger_typed type_eq wrap_eq
  · cases ok_eq

private theorem bitwiseNotInteger_ok_typed {tables : Tables} {ty : Ty}
    {typeId : TypeId} {arguments : Array RuntimeValue} {value : RuntimeValue}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : bitwiseNotInteger ty arguments = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  unfold bitwiseNotInteger at ok_eq
  split at ok_eq
  · split at ok_eq
    · cases ok_eq
    · split at ok_eq
      · simp only [bind, Option.bind_eq_some_iff, Option.some.injEq] at ok_eq
        obtain ⟨bits, -, out, wrap_eq, out_def⟩ := ok_eq
        have : out = value := by simpa using out_def
        subst this
        exact modularInteger_typed type_eq wrap_eq
      · cases ok_eq
  · cases ok_eq

private theorem checkedShiftInteger_ok_typed {tables : Tables} {ty : Ty}
    {typeId : TypeId} {failure : ThrowKind} {left : Bool}
    {arguments : Array RuntimeValue} {value : RuntimeValue}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : checkedShiftInteger failure left ty arguments = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  unfold checkedShiftInteger at ok_eq
  split at ok_eq
  · split at ok_eq
    · cases ok_eq
    · split at ok_eq
      · split at ok_eq
        · simp at ok_eq
        · simp only [bind, Option.bind_eq_some_iff, Option.some.injEq] at ok_eq
          obtain ⟨bits, -, out, wrap_eq, out_def⟩ := ok_eq
          have : out = value := by simpa using out_def
          subst this
          exact modularInteger_typed type_eq wrap_eq
      · cases ok_eq
  · cases ok_eq

private theorem shiftInteger_ok_typed {tables : Tables} {ty : Ty}
    {typeId : TypeId} {left : Bool}
    {arguments : Array RuntimeValue} {value : RuntimeValue}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : shiftInteger left ty arguments = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  unfold shiftInteger at ok_eq
  split at ok_eq
  · rename_i inner_eq
    cases ok_eq
    exact checkedShiftInteger_ok_typed type_eq inner_eq
  · cases ok_eq

/-- Comparison results are Boolean whatever operand family matched. -/
private theorem compareOrdered_ok_typed {tables : Tables} {typeId : TypeId}
    {arguments : Array RuntimeValue} {value : RuntimeValue}
    {integerRelation : Int → Int → Bool} {booleanRelation : Bool → Bool → Bool}
    {characterRelation : Nat → Nat → Bool}
    (result_eq : tables.types[typeId.index]? = some .bool)
    (ok_eq : compareOrdered arguments integerRelation booleanRelation characterRelation
      = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  unfold compareOrdered at ok_eq
  split at ok_eq <;> cases ok_eq <;> exact .bool typeId _ result_eq

/-- Scalar operands rule the Boolean arm of bitwise operations out. -/
private theorem bitwiseBinary_ok_typed {tables : Tables} {ty : Ty} {typeId : TypeId}
    {arguments : Array RuntimeValue} {value : RuntimeValue}
    {integerOperation : Nat → Nat → Nat} {booleanOperation : Bool → Bool → Bool}
    (type_eq : tables.types[typeId.index]? = some ty)
    (arguments_scalar : ∀ argument ∈ arguments,
      (∃ i, argument = RuntimeValue.integer i) ∨ ∃ c, argument = RuntimeValue.character c)
    (ok_eq : bitwiseBinary ty arguments integerOperation booleanOperation
      = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  unfold bitwiseBinary at ok_eq
  cases shape : arguments.toList with
  | nil =>
      rw [shape] at ok_eq
      exact bitwiseBinaryInteger_ok_typed type_eq ok_eq
  | cons head tail =>
      rw [shape] at ok_eq
      cases head
      case bool b =>
        cases tail with
        | nil => exact bitwiseBinaryInteger_ok_typed type_eq ok_eq
        | cons second rest =>
            cases second
            case bool b2 =>
              cases rest with
              | nil =>
                  exfalso
                  have head_scalar := arguments_scalar (.bool b)
                    (by rw [Array.mem_def, shape]; simp)
                  rcases head_scalar with ⟨i, hi⟩ | ⟨c, hc⟩
                  · simp at hi
                  · simp at hc
              | cons third more => exact bitwiseBinaryInteger_ok_typed type_eq ok_eq
            all_goals exact bitwiseBinaryInteger_ok_typed type_eq ok_eq
      all_goals exact bitwiseBinaryInteger_ok_typed type_eq ok_eq

private theorem overflowingBinaryInteger_ok_typed {ns : ValidatedNamespace}
    {target : Option Nat} {resultType valueType overflowType : TypeId}
    {width : Nat} {signed : Bool} {arguments : Array RuntimeValue}
    {value : RuntimeValue} {operation : Int → Int → Int}
    (result_eq : ns.tables.types[resultType.index]? =
      some (.tuple #[valueType, overflowType]))
    (value_eq : ns.tables.types[valueType.index]? = some (.integer (.bits width) signed))
    (overflow_eq : ns.tables.types[overflowType.index]? = some .bool)
    (ok_eq : overflowingBinaryInteger ns target (.tuple #[valueType, overflowType])
      arguments operation = some (.ok value)) :
    ValueHasType unit ns.tables value resultType := by
  unfold overflowingBinaryInteger at ok_eq
  dsimp only [List.toList_toArray] at ok_eq
  rw [value_eq, overflow_eq] at ok_eq
  simp [resolveTargetIntegerType?] at ok_eq
  obtain ⟨-, ok_eq⟩ := ok_eq
  split at ok_eq
  · simp only [Option.bind_eq_some_iff, Option.some.injEq, Except.ok.injEq] at ok_eq
    obtain ⟨bounds, -, out, wrap_eq, out_def⟩ := ok_eq
    rw [← out_def]
    refine .tuple resultType _ #[valueType, overflowType] result_eq ?_
    exact .cons _ _ _ _ (modularInteger_typed value_eq wrap_eq)
      (.cons _ _ _ _ (.bool overflowType _ overflow_eq) .nil)
  · cases ok_eq

/-- The primitive operations whose validated result is a nonzero fixed-width
integer. -/
def isFixedIntegerResultPrimitive : PrimitiveOperation → Bool
  | .add | .subtract | .multiply | .divide | .modulo => true
  | .checkedAdd _ | .checkedSubtract _ | .checkedMultiply _
  | .checkedDivide _ | .checkedModulo _ => true
  | .negate | .checkedNegate _ | .bitwiseNot => true
  | .bitwiseOr | .bitwiseAnd | .bitwiseXor => true
  | .shiftLeft | .shiftRight | .checkedShiftLeft _ | .checkedShiftRight _ => true
  | .cast | .checkedCast _ => true
  | _ => false

/-- The primitive operations whose validated result is Boolean. -/
def isBooleanResultPrimitive : PrimitiveOperation → Bool
  | .logicalAnd | .logicalOr | .logicalNot => true
  | .equal | .notEqual => true
  | .less | .greater | .lessEqual | .greaterEqual => true
  | .bitwiseOr | .bitwiseAnd | .bitwiseXor => true
  | _ => false

/-- The well-formed scalar primitive shapes admitted by validation, as the
premises the operation preservation theorem consumes. The aggregate
vocabulary (tuple, vector, length, index, slice, repeat) joins with the
aggregate slice. -/
inductive WfPrimitive (tables : Tables) : PrimitiveOperation → TypeId → Prop where
  | fixedInteger (operation : PrimitiveOperation) (resultType : TypeId)
      (width : Nat) (signed : Bool)
      (operation_scalar : isFixedIntegerResultPrimitive operation = true)
      (result_eq : tables.types[resultType.index]? =
        some (.integer (.bits width) signed)) :
      WfPrimitive tables operation resultType
  | boolean (operation : PrimitiveOperation) (resultType : TypeId)
      (operation_boolean : isBooleanResultPrimitive operation = true)
      (result_eq : tables.types[resultType.index]? = some .bool) :
      WfPrimitive tables operation resultType
  | characterCast (operation : PrimitiveOperation) (resultType : TypeId)
      (operation_cast : operation = .cast ∨ ∃ failure, operation = .checkedCast failure)
      (result_eq : tables.types[resultType.index]? = some .character) :
      WfPrimitive tables operation resultType
  | overflowing (operation : PrimitiveOperation)
      (resultType valueType overflowType : TypeId) (width : Nat) (signed : Bool)
      (operation_overflowing : operation = .overflowingAdd ∨
        operation = .overflowingSubtract ∨ operation = .overflowingMultiply)
      (result_eq : tables.types[resultType.index]? =
        some (.tuple #[valueType, overflowType]))
      (value_eq : tables.types[valueType.index]? =
        some (.integer (.bits width) signed))
      (overflow_eq : tables.types[overflowType.index]? = some .bool) :
      WfPrimitive tables operation resultType

private theorem divideIntegers?_ok_typed {tables : Tables} {ty : Ty}
    {typeId : TypeId} {arguments : Array RuntimeValue} {value : RuntimeValue}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : divideIntegers? ty arguments = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  rw [divideIntegers?] at ok_eq
  split at ok_eq
  · simp only [bind, Option.bind_eq_some_iff] at ok_eq
    obtain ⟨quotient, -, out, wrap_eq, out_def⟩ := ok_eq
    have : out = value := by simpa using out_def
    subst this
    exact modularInteger_typed type_eq wrap_eq
  · cases ok_eq

private theorem moduloIntegers?_ok_typed {tables : Tables} {ty : Ty}
    {typeId : TypeId} {arguments : Array RuntimeValue} {value : RuntimeValue}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : moduloIntegers? ty arguments = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  rw [moduloIntegers?] at ok_eq
  split at ok_eq
  · simp only [bind, Option.bind_eq_some_iff] at ok_eq
    obtain ⟨remainder, -, out, wrap_eq, out_def⟩ := ok_eq
    have : out = value := by simpa using out_def
    subst this
    exact modularInteger_typed type_eq wrap_eq
  · cases ok_eq

/- The checked helpers' literal-zero pattern overlaps their general one, so
they have no single equation to rewrite with; they are unfolded, and the
proof follows the exposed matches. -/
private theorem checkedDivideIntegers?_ok_typed {tables : Tables} {ty : Ty}
    {typeId : TypeId} {failure : ThrowKind} {arguments : Array RuntimeValue}
    {value : RuntimeValue}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : checkedDivideIntegers? failure ty arguments = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  unfold checkedDivideIntegers? at ok_eq
  split at ok_eq
  · cases ok_eq
  · simp only [bind, pure, Option.bind_eq_some_iff, Option.some.injEq] at ok_eq
    obtain ⟨quotient, -, checked_eq⟩ := ok_eq
    exact checkedInteger_ok_typed type_eq checked_eq
  · cases ok_eq

private theorem checkedModuloIntegers?_ok_typed {tables : Tables} {ty : Ty}
    {typeId : TypeId} {failure : ThrowKind} {arguments : Array RuntimeValue}
    {value : RuntimeValue}
    (type_eq : tables.types[typeId.index]? = some ty)
    (ok_eq : checkedModuloIntegers? failure ty arguments = some (.ok value)) :
    ValueHasType unit tables value typeId := by
  unfold checkedModuloIntegers? at ok_eq
  split at ok_eq
  · cases ok_eq
  · simp only [bind, pure, Option.bind_eq_some_iff] at ok_eq
    obtain ⟨quotient, -, checked_eq⟩ := ok_eq
    split at checked_eq
    · cases checked_eq
    · simp only [Option.some.injEq] at checked_eq
      exact checkedInteger_ok_typed type_eq checked_eq
  · cases ok_eq

/-- Preservation for the scalar primitive vocabulary: a successful evaluation
of a well-formed primitive over scalar operands is typed at the validated
result type. -/
theorem WfPrimitive.eval_typed {ns : ValidatedNamespace}
    {operation : PrimitiveOperation} {resultType : TypeId}
    {arguments : Array RuntimeValue} {target : Option Nat} {value : RuntimeValue}
    (wf : WfPrimitive ns.tables operation resultType)
    (arguments_scalar : ∀ argument ∈ arguments,
      (∃ i, argument = RuntimeValue.integer i) ∨ ∃ c, argument = RuntimeValue.character c)
    (eval_eq : evaluatePrimitiveOperation? ns resultType operation arguments target
      = some (.ok value)) :
    ValueHasType unit ns.tables value resultType := by
  cases wf with
  | fixedInteger width signed operation_scalar result_eq =>
      unfold evaluatePrimitiveOperation? at eval_eq
      simp only [result_eq, resolveTargetIntegerType?, bind, Option.bind_eq_some_iff,
        Option.some.injEq, exists_eq_left'] at eval_eq
      cases operation
      case add => exact modularBinaryInteger_ok_typed result_eq eval_eq
      case subtract => exact modularBinaryInteger_ok_typed result_eq eval_eq
      case multiply => exact modularBinaryInteger_ok_typed result_eq eval_eq
      case checkedAdd failure => exact checkedBinaryInteger_ok_typed result_eq eval_eq
      case checkedSubtract failure =>
        exact checkedBinaryInteger_ok_typed result_eq eval_eq
      case checkedMultiply failure =>
        exact checkedBinaryInteger_ok_typed result_eq eval_eq
      case negate => exact modularUnaryInteger_ok_typed result_eq eval_eq
      case checkedNegate failure => exact checkedUnaryInteger_ok_typed result_eq eval_eq
      case bitwiseNot => exact bitwiseNotInteger_ok_typed result_eq eval_eq
      case bitwiseOr => exact bitwiseBinary_ok_typed result_eq arguments_scalar eval_eq
      case bitwiseAnd => exact bitwiseBinary_ok_typed result_eq arguments_scalar eval_eq
      case bitwiseXor => exact bitwiseBinary_ok_typed result_eq arguments_scalar eval_eq
      case shiftLeft => exact shiftInteger_ok_typed result_eq eval_eq
      case shiftRight => exact shiftInteger_ok_typed result_eq eval_eq
      case checkedShiftLeft failure =>
        exact checkedShiftInteger_ok_typed result_eq eval_eq
      case checkedShiftRight failure =>
        exact checkedShiftInteger_ok_typed result_eq eval_eq
      case divide =>
        try dsimp only at eval_eq
        exact divideIntegers?_ok_typed result_eq eval_eq
      case modulo =>
        try dsimp only at eval_eq
        exact moduloIntegers?_ok_typed result_eq eval_eq
      case checkedDivide failure =>
        try dsimp only at eval_eq
        exact checkedDivideIntegers?_ok_typed result_eq eval_eq
      case checkedModulo failure =>
        try dsimp only at eval_eq
        exact checkedModuloIntegers?_ok_typed result_eq eval_eq
      case cast =>
        dsimp only at eval_eq
        split at eval_eq
        · exact modularWrapped_ok_typed result_eq eval_eq (fun out h => ⟨_, h⟩)
        · exact modularWrapped_ok_typed result_eq eval_eq (fun out h => ⟨_, h⟩)
        · cases eval_eq
      case checkedCast failure =>
        dsimp only at eval_eq
        split at eval_eq
        · simp only [Option.some.injEq] at eval_eq
          exact checkedInteger_ok_typed result_eq eval_eq
        · simp only [Option.some.injEq] at eval_eq
          exact checkedInteger_ok_typed result_eq eval_eq
        · cases eval_eq
      all_goals simp [isFixedIntegerResultPrimitive] at operation_scalar
  | boolean operation_boolean result_eq =>
      unfold evaluatePrimitiveOperation? at eval_eq
      simp only [result_eq, resolveTargetIntegerType?, bind, Option.bind_eq_some_iff,
        Option.some.injEq, exists_eq_left'] at eval_eq
      cases operation
      case logicalAnd =>
        dsimp only at eval_eq
        split at eval_eq <;> cases eval_eq <;> exact .bool _ _ result_eq
      case logicalOr =>
        dsimp only at eval_eq
        split at eval_eq <;> cases eval_eq <;> exact .bool _ _ result_eq
      case logicalNot =>
        dsimp only at eval_eq
        split at eval_eq <;> cases eval_eq <;> exact .bool _ _ result_eq
      case equal =>
        try dsimp only at eval_eq
        rw [equalValues?] at eval_eq
        split at eval_eq <;> cases eval_eq <;> exact .bool _ _ result_eq
      case notEqual =>
        try dsimp only at eval_eq
        rw [notEqualValues?] at eval_eq
        split at eval_eq <;> cases eval_eq <;> exact .bool _ _ result_eq
      case less => exact compareOrdered_ok_typed result_eq eval_eq
      case greater => exact compareOrdered_ok_typed result_eq eval_eq
      case lessEqual => exact compareOrdered_ok_typed result_eq eval_eq
      case greaterEqual => exact compareOrdered_ok_typed result_eq eval_eq
      case bitwiseOr =>
        dsimp only at eval_eq
        unfold bitwiseBinary at eval_eq
        split at eval_eq
        · cases eval_eq; exact .bool _ _ result_eq
        · exact bitwiseBinaryInteger_ok_typed result_eq eval_eq
      case bitwiseAnd =>
        dsimp only at eval_eq
        unfold bitwiseBinary at eval_eq
        split at eval_eq
        · cases eval_eq; exact .bool _ _ result_eq
        · exact bitwiseBinaryInteger_ok_typed result_eq eval_eq
      case bitwiseXor =>
        dsimp only at eval_eq
        unfold bitwiseBinary at eval_eq
        split at eval_eq
        · cases eval_eq; exact .bool _ _ result_eq
        · exact bitwiseBinaryInteger_ok_typed result_eq eval_eq
      all_goals simp [isBooleanResultPrimitive] at operation_boolean
  | characterCast operation_cast result_eq =>
      unfold evaluatePrimitiveOperation? at eval_eq
      simp only [result_eq, resolveTargetIntegerType?, bind, Option.bind_eq_some_iff,
        Option.some.injEq, exists_eq_left'] at eval_eq
      rcases operation_cast with rfl | ⟨failure, rfl⟩
      · dsimp only at eval_eq
        split at eval_eq
        · split at eval_eq
          · cases eval_eq
          · cases eval_eq
            rename_i guard
            refine .character resultType _ result_eq ?_
            simp only [Bool.or_eq_true, not_or, Bool.not_eq_true] at guard
            simpa using guard.2
        · exact modularWrapped_ok_typed result_eq eval_eq (fun out h => ⟨_, h⟩)
        · cases eval_eq
      · dsimp only at eval_eq
        split at eval_eq
        · split at eval_eq
          · cases eval_eq
          · cases eval_eq
            rename_i guard
            refine .character resultType _ result_eq ?_
            simp only [Bool.or_eq_true, not_or, Bool.not_eq_true] at guard
            simpa using guard.2
        · simp only [Option.some.injEq] at eval_eq
          exact checkedInteger_ok_typed result_eq eval_eq
        · cases eval_eq
  | overflowing valueType overflowType width signed
      operation_overflowing result_eq value_eq overflow_eq =>
      unfold evaluatePrimitiveOperation? at eval_eq
      simp only [result_eq, resolveTargetIntegerType?, bind, Option.bind_eq_some_iff,
        Option.some.injEq, exists_eq_left'] at eval_eq
      rcases operation_overflowing with rfl | rfl | rfl <;>
        exact overflowingBinaryInteger_ok_typed result_eq value_eq overflow_eq eval_eq

/-- A member of a homogeneously typed list is typed at the element type. -/
theorem _root_.LeanerIR.ValuesHaveType.mem_typed {tables : Tables} :
    ∀ {values : List RuntimeValue} {typeId : TypeId} {value : RuntimeValue},
      ValuesHaveType unit tables values typeId → value ∈ values →
      ValueHasType unit tables value typeId
  | _, _, _, .cons _ _ _ head_typed _, .head _ => head_typed
  | _, _, _, .cons _ _ _ _ tail_typed, .tail _ tail_mem =>
      mem_typed tail_typed tail_mem

/-- Pointwise membership typing builds the homogeneous list judgment. -/
theorem valuesHaveType_of_forall_mem {tables : Tables} {values : List RuntimeValue}
    {typeId : TypeId}
    (typed : ∀ value ∈ values, ValueHasType unit tables value typeId) :
    ValuesHaveType unit tables values typeId := by
  induction values with
  | nil => exact .nil _
  | cons head tail ih =>
      exact .cons _ _ _ (typed _ (by simp))
        (ih fun v mem => typed _ (by simp [mem]))

/-- Preservation for `tuple`: packing typed arguments yields a typed tuple. -/
theorem tuple_eval_typed {ns : ValidatedNamespace} {resultType : TypeId}
    {elementTypes : Array TypeId} {arguments : Array RuntimeValue}
    {target : Option Nat} {value : RuntimeValue}
    (result_eq : ns.tables.types[resultType.index]? = some (.tuple elementTypes))
    (arguments_typed : ValuesHaveTypes unit ns.tables arguments.toList elementTypes.toList)
    (eval_eq : evaluatePrimitiveOperation? ns resultType .tuple arguments target
      = some (.ok value)) :
    ValueHasType unit ns.tables value resultType := by
  unfold evaluatePrimitiveOperation? at eval_eq
  simp only [result_eq, resolveTargetIntegerType?, bind, Option.bind_eq_some_iff,
    Option.some.injEq, exists_eq_left'] at eval_eq
  cases eval_eq
  exact .tuple resultType _ elementTypes result_eq arguments_typed

/-- Preservation for `vector`: packing typed arguments yields a typed vector. -/
theorem vector_eval_typed {ns : ValidatedNamespace} {resultType : TypeId}
    {elementType : TypeId} {length? : Option ConstValue}
    {arguments : Array RuntimeValue} {target : Option Nat} {value : RuntimeValue}
    (result_eq : ns.tables.types[resultType.index]? =
      some (.vector elementType length?))
    (length_matches : VectorLengthMatches length? arguments.size)
    (arguments_typed : ValuesHaveType unit ns.tables arguments.toList elementType)
    (eval_eq : evaluatePrimitiveOperation? ns resultType .vector arguments target
      = some (.ok value)) :
    ValueHasType unit ns.tables value resultType := by
  unfold evaluatePrimitiveOperation? at eval_eq
  simp only [result_eq, resolveTargetIntegerType?, bind, Option.bind_eq_some_iff,
    Option.some.injEq, exists_eq_left'] at eval_eq
  cases eval_eq
  exact .vector resultType elementType _ length? result_eq length_matches arguments_typed

/-- Preservation for `repeatVector`: replicating the typed seed yields a
vector of the annotated constant length. -/
theorem repeatVector_eval_typed {ns : ValidatedNamespace} {resultType : TypeId}
    {elementType : TypeId} {length : Int} {arguments : Array RuntimeValue}
    {target : Option Nat} {value : RuntimeValue}
    (result_eq : ns.tables.types[resultType.index]? =
      some (.vector elementType (some (.integer length))))
    (arguments_typed : ValuesHaveType unit ns.tables arguments.toList elementType)
    (eval_eq : evaluatePrimitiveOperation? ns resultType .repeatVector arguments target
      = some (.ok value)) :
    ValueHasType unit ns.tables value resultType := by
  unfold evaluatePrimitiveOperation? at eval_eq
  simp only [result_eq, resolveTargetIntegerType?, bind, Option.bind_eq_some_iff,
    Option.some.injEq, exists_eq_left'] at eval_eq
  split at eval_eq
  · rename_i element' length' seed node_eq shape
    cases node_eq
    split at eval_eq
    · cases eval_eq
    · rename_i nonnegative
      cases eval_eq
      refine .vector resultType elementType _ _ result_eq ?_ ?_
      · simp only [VectorLengthMatches, Array.size_replicate]
        exact (Int.toNat_of_nonneg (by omega)).symm
      · refine valuesHaveType_of_forall_mem fun v mem => ?_
        have seed_typed : ValueHasType unit ns.tables seed elementType :=
          arguments_typed.mem_typed (by rw [shape]; simp)
        have : v = seed := by
          simp only [Array.toList_replicate, List.mem_replicate] at mem
          exact mem.2
        rw [this]
        exact seed_typed
  · cases eval_eq

/-- Preservation for `length`: the reported size inhabits the validated
integer result node (unbounded or nonzero fixed width). -/
theorem length_eval_typed {ns : ValidatedNamespace} {resultType : TypeId}
    {width : IntWidth} {signed : Bool} {arguments : Array RuntimeValue}
    {target : Option Nat} {value : RuntimeValue}
    (result_eq : ns.tables.types[resultType.index]? = some (.integer width signed))
    (width_shape : width = .unbounded ∨ ∃ bits, width = .bits bits)
    (eval_eq : evaluatePrimitiveOperation? ns resultType .length arguments target
      = some (.ok value)) :
    ValueHasType unit ns.tables value resultType := by
  unfold evaluatePrimitiveOperation? at eval_eq
  rcases width_shape with rfl | ⟨bits, rfl⟩
  · simp only [result_eq, resolveTargetIntegerType?, bind, Option.bind_eq_some_iff,
      Option.some.injEq, exists_eq_left'] at eval_eq
    split at eval_eq <;> cases eval_eq <;>
      exact .integer resultType _ _ signed result_eq
        (by simp [IntegerValueFits, Ty.integerValueFits?])
  · simp only [result_eq, resolveTargetIntegerType?, bind, Option.bind_eq_some_iff,
      Option.some.injEq, exists_eq_left'] at eval_eq
    split at eval_eq <;>
      first
        | cases eval_eq
        | exact modularWrapped_ok_typed result_eq eval_eq (fun out h => ⟨_, h⟩)

/-- Preservation for `index`: a selected element carries the element typing
of the indexed vector. -/
theorem index_eval_typed {ns : ValidatedNamespace} {resultType : TypeId}
    {arguments : Array RuntimeValue} {target : Option Nat} {value : RuntimeValue}
    (elements_typed : ∀ elements, RuntimeValue.vector elements ∈ arguments →
      ValuesHaveType unit ns.tables elements.toList resultType)
    (eval_eq : evaluatePrimitiveOperation? ns resultType .index arguments target
      = some (.ok value)) :
    ValueHasType unit ns.tables value resultType := by
  unfold evaluatePrimitiveOperation? at eval_eq
  simp only [bind, Option.bind_eq_some_iff] at eval_eq
  obtain ⟨resolved, -, eval_eq⟩ := eval_eq
  split at eval_eq
  · rename_i elements index shape
    split at eval_eq
    · cases eval_eq
    · cases get_eq : elements[index.toNat]? with
      | some element =>
          rw [get_eq] at eval_eq
          have element_eq : element = value := by simpa using eval_eq
          subst element_eq
          have vector_mem : RuntimeValue.vector elements ∈ arguments := by
            rw [Array.mem_def, shape]; simp
          have element_mem : element ∈ elements.toList := by
            rw [Array.getElem?_eq_some_iff] at get_eq
            obtain ⟨lt, rfl⟩ := get_eq
            simp
          exact (elements_typed elements vector_mem).mem_typed element_mem
      | none => rw [get_eq] at eval_eq; cases eval_eq
  · cases eval_eq

/-- Preservation for `slice`: an extracted subvector keeps the element typing
of its source at the unconstrained-length vector result node. -/
theorem slice_eval_typed {ns : ValidatedNamespace} {resultType : TypeId}
    {elementType : TypeId} {arguments : Array RuntimeValue}
    {target : Option Nat} {value : RuntimeValue}
    (result_eq : ns.tables.types[resultType.index]? =
      some (.vector elementType none))
    (elements_typed : ∀ elements, RuntimeValue.vector elements ∈ arguments →
      ValuesHaveType unit ns.tables elements.toList elementType)
    (eval_eq : evaluatePrimitiveOperation? ns resultType .slice arguments target
      = some (.ok value)) :
    ValueHasType unit ns.tables value resultType := by
  unfold evaluatePrimitiveOperation? at eval_eq
  simp only [result_eq, resolveTargetIntegerType?, bind, Option.bind_eq_some_iff,
    Option.some.injEq, exists_eq_left'] at eval_eq
  split at eval_eq
  · rename_i elements start stop shape
    split at eval_eq
    · cases eval_eq
    · cases eval_eq
      have vector_mem : RuntimeValue.vector elements ∈ arguments := by
        rw [Array.mem_def, shape]; simp
      refine .vector resultType elementType _ none result_eq trivial ?_
      refine valuesHaveType_of_forall_mem fun v mem => ?_
      refine (elements_typed elements vector_mem).mem_typed ?_
      rw [Array.toList_extract, List.extract_eq_take_drop] at mem
      exact List.mem_of_mem_drop (List.mem_of_mem_take mem)
  · cases eval_eq

/-- Preservation for `copyValue`/`moveValue`: the forwarded argument keeps
its typing at the result node. -/
theorem copyMove_eval_typed {ns : ValidatedNamespace} {resultType : TypeId}
    {operation : PrimitiveOperation} {arguments : Array RuntimeValue}
    {target : Option Nat} {value : RuntimeValue}
    (operation_copy : operation = .copyValue ∨ operation = .moveValue)
    (arguments_typed : ∀ argument ∈ arguments,
      ValueHasType unit ns.tables argument resultType)
    (eval_eq : evaluatePrimitiveOperation? ns resultType operation arguments target
      = some (.ok value)) :
    ValueHasType unit ns.tables value resultType := by
  rcases operation_copy with rfl | rfl <;>
    · unfold evaluatePrimitiveOperation? at eval_eq
      simp only [bind, Option.bind_eq_some_iff] at eval_eq
      obtain ⟨resolved, -, eval_eq⟩ := eval_eq
      split at eval_eq
      · rename_i head shape
        cases eval_eq
        exact arguments_typed _ (by rw [Array.mem_def, shape]; simp)
      · cases eval_eq

end SemanticTyping
end LeanerIR
