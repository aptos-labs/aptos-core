-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Invocation

/-!
# Typed memory is typed at run time

An invocation of a function value a proof does not see runs from global
memory whose every slot is typed at its key's type (`designs/static-typing.md`,
Phase 5). Typed memory (`Denote.Memory`) holds a native value per resource
type, so every runtime encoding of every memory is typed once the unit's
resource types correspond to their keys' types (`ResourcesTyped`), which is
decided by evaluation over the unit (`resourcesTypedCheck`) and holds of the
unit, not of a state.
-/

namespace VerexIR.Proofs

open VerexIR.Validation
open Denote

theorem _root_.VerexIR.SemTy.packs_pack : (results : List SemTy) →
    StaticTyping.packs results (SemTy.pack results) = true
  | [] => by simp [StaticTyping.packs, SemTy.pack]
  | [_] => by simp [StaticTyping.packs, SemTy.pack]
  | _ :: _ :: _ => by simp [StaticTyping.packs, SemTy.pack]

mutual
/-- A native type a semantic type reads as: the compiler's and the checker's
readings of one type identifier agree. A nominal type is read at its
arguments' readings, as many as its declaration binds, and a shared reference
is read only as a function type's parameter, which the type marks. Its encodings inhabit the
semantic type where it is closed and holds no reference (`TypedAs.encode`). -/
inductive Denote.NTy.TypedAs (unit : ValidatedUnit) : NTy → SemTy → Prop
  | scalar {τ : NTy} {type : SemTy} (scalar : τ.scalarType? = some type) :
      Denote.NTy.TypedAs unit τ type
  | unit : Denote.NTy.TypedAs unit .unit .unit
  | tuple {elements : NRow} {types : List SemTy} (typed : Denote.NRow.TypedAs unit elements types) :
      Denote.NTy.TypedAs unit (.tuple elements) (.tuple types)
  | vector {element : NTy} {type : SemTy} (typed : Denote.NTy.TypedAs unit element type) :
      Denote.NTy.TypedAs unit (.vector element) (.vector type none)
  /-- A table owner's hidden contents: its entries, pairs of a key and a value. -/
  | table {key value : NTy} {keyType valueType : SemTy}
      (typedKey : Denote.NTy.TypedAs unit key keyType)
      (typedValue : Denote.NTy.TypedAs unit value valueType) :
      Denote.NTy.TypedAs unit (.table key value) (.vector (.tuple [keyType, valueType]) none)
  /-- A structure at the types its declaration's fields resolve to under its
  arguments, a table owner's hidden contents at its entries' type. -/
  | struct {source : StructHandle} {nativeArguments fields : NRow} {name : QualifiedName}
      {arguments : List SemArg}
      {declaringNs : ValidatedNamespace} {declared : Array FieldDecl}
      {fieldTypes ghostTypes : List SemTy}
      (name_eq : SemanticOperations.structName? unit source = some name)
      (arity : declarationArity? unit source = some arguments.length)
      (typedArguments : Denote.NRow.ArgumentsTypedAs unit nativeArguments arguments)
      (declaration_eq : SemanticOperations.handleFields? unit source none =
        some (declaringNs, declared))
      (resolve : ResolvesAll declaringNs.tables arguments.toArray
        (declared.toList.map (·.type.typeId)) fieldTypes)
      (ghostsResolve : ResolvesAll declaringNs.tables arguments.toArray
        (ghostTypeIds unit source) ghostTypes)
      (typed : Denote.NRow.TypedAs unit fields
        (fieldTypes ++ (ghostTypes.map .ghost ++ contentsTypes unit source none arguments))) :
      Denote.NTy.TypedAs unit (.struct source nativeArguments fields) (.nominal name arguments)
  /-- An intrinsic map owner at its key and value arguments: the fields of
  its abstract value, its leading fields, its ghost slots, and its entries
  (`OwnerLayout.canonicalTypes`; `designs/intrinsic-maps.md`, "Abstract
  representation"). -/
  | abstract {source : StructHandle} {nativeArguments fields : NRow} {name : QualifiedName}
      {key value : SemTy} {layout : OwnerLayout} {declaringNs : ValidatedNamespace}
      {ghostTypes : List SemTy}
      (name_eq : SemanticOperations.structName? unit source = some name)
      (arity : declarationArity? unit source = some 2)
      (typedArguments : Denote.NRow.ArgumentsTypedAs unit nativeArguments
        [.type key, .type value])
      (layout_eq : abstractLayout? unit source.namespaceId source.structId = some layout)
      (namespace_eq : unit.namespaces[source.namespaceId.index]? = some declaringNs)
      (ghostsResolve : ResolvesAll declaringNs.tables #[.type key, .type value]
        (ghostTypeIds unit source) ghostTypes)
      (typed : Denote.NRow.TypedAs unit fields (layout.canonicalTypes ghostTypes key value)) :
      Denote.NTy.TypedAs unit (.struct source nativeArguments fields)
        (.nominal name [.type key, .type value])
  /-- An enum whose every variant's row reads as that variant's fields. -/
  | enum {source : StructHandle} {nativeArguments : NRow} {names : List String} {rows : NRows}
      {distinct : names.Nodup} {name : QualifiedName} {arguments : List SemArg}
      (name_eq : SemanticOperations.structName? unit source = some name)
      (arity : declarationArity? unit source = some arguments.length)
      (typedArguments : Denote.NRow.ArgumentsTypedAs unit nativeArguments arguments)
      (nonempty : names ≠ [])
      (names_eq : declarationVariants? unit source = some names)
      (variants : Denote.NRows.VariantsTypedAs unit source arguments names rows) :
      Denote.NTy.TypedAs unit (.enum source nativeArguments names rows distinct)
        (.nominal name arguments)
  | ref {referent : NTy} {type : SemTy} (typed : Denote.NTy.TypedAs unit referent type) :
      Denote.NTy.TypedAs unit (.ref referent) (.reference .mutable type)
  /-- A ghost field's slot reads as a slot of its type (`designs/ghost-fields.md`). -/
  | ghost {slot : NTy} {type : SemTy} (typed : Denote.NTy.TypedAs unit slot type) :
      Denote.NTy.TypedAs unit (.ghost slot) (.ghost type)
  | param (index : Nat) : Denote.NTy.TypedAs unit (.param index) (.param index)
  /-- A function type at its rows, each parameter a shared reference where it
  is marked so, the results packed as the checker packs them. -/
  | function {parameters returned : NRow} {shared : List Bool} {parameterTypes results : List SemTy}
      {result : SemTy}
      (typedParameters : Denote.NRow.ParametersTypedAs unit parameters shared parameterTypes)
      (typedResults : Denote.NRow.TypedAs unit returned results)
      (packed : result = SemTy.pack results) :
      Denote.NTy.TypedAs unit (.function parameters shared returned)
        (.function parameterTypes result)

/-- A row of native types reading as a row of semantic types. -/
inductive Denote.NRow.TypedAs (unit : ValidatedUnit) : NRow → List SemTy → Prop
  | nil : Denote.NRow.TypedAs unit .nil []
  | cons {τ : NTy} {rest : NRow} {type : SemTy} {types : List SemTy} :
      Denote.NTy.TypedAs unit τ type → Denote.NRow.TypedAs unit rest types →
        Denote.NRow.TypedAs unit (.cons τ rest) (type :: types)

/-- A row of native types reading as type arguments. -/
inductive Denote.NRow.ArgumentsTypedAs (unit : ValidatedUnit) : NRow → List SemArg → Prop
  | nil : Denote.NRow.ArgumentsTypedAs unit .nil []
  | cons {τ : NTy} {rest : NRow} {type : SemTy} {arguments : List SemArg} :
      Denote.NTy.TypedAs unit τ type → Denote.NRow.ArgumentsTypedAs unit rest arguments →
        Denote.NRow.ArgumentsTypedAs unit (.cons τ rest) (.type type :: arguments)

/-- A function type's parameter row reading as its semantic parameters: a
parameter marked shared as a shared reference to what it reads as, any
other as a type that is not one. -/
inductive Denote.NRow.ParametersTypedAs (unit : ValidatedUnit) :
    NRow → List Bool → List SemTy → Prop
  | nil : Denote.NRow.ParametersTypedAs unit .nil [] []
  | value {τ : NTy} {rest : NRow} {shared : List Bool} {type : SemTy} {types : List SemTy}
      (typed : Denote.NTy.TypedAs unit τ type) (observed : type.unshared = type)
      (later : Denote.NRow.ParametersTypedAs unit rest shared types) :
      Denote.NRow.ParametersTypedAs unit (.cons τ rest) (false :: shared) (type :: types)
  | shared {τ : NTy} {rest : NRow} {shared : List Bool} {type : SemTy} {types : List SemTy}
      (typed : Denote.NTy.TypedAs unit τ type) (observed : type.unshared = type)
      (later : Denote.NRow.ParametersTypedAs unit rest shared types) :
      Denote.NRow.ParametersTypedAs unit (.cons τ rest) (true :: shared)
        (.reference .shared type :: types)

/-- An enum's variant rows, each reading as its variant's declared fields
under the enum's arguments. -/
inductive Denote.NRows.VariantsTypedAs (unit : ValidatedUnit) :
    StructHandle → List SemArg → List String → NRows → Prop
  | nil {source : StructHandle} {arguments : List SemArg} :
      Denote.NRows.VariantsTypedAs unit source arguments [] .nil
  | cons {source : StructHandle} {arguments : List SemArg} {name : String} {names : List String}
      {fields : NRow} {rest : NRows}
      {declaringNs : ValidatedNamespace} {declared : Array FieldDecl}
      {fieldTypes ghostTypes : List SemTy}
      (declaration_eq : SemanticOperations.handleFields? unit source (some name) =
        some (declaringNs, declared))
      (resolve : ResolvesAll declaringNs.tables arguments.toArray
        (declared.toList.map (·.type.typeId)) fieldTypes)
      (ghostsResolve : ResolvesAll declaringNs.tables arguments.toArray
        (ghostTypeIds unit source) ghostTypes)
      (typed : Denote.NRow.TypedAs unit fields (fieldTypes ++ ghostTypes.map .ghost))
      (later : Denote.NRows.VariantsTypedAs unit source arguments names rest) :
      Denote.NRows.VariantsTypedAs unit source arguments (name :: names) (.cons fields rest)
end

theorem HasTypeEach.of_forall {unit : ValidatedUnit} {loans : LoanTypes} {type : SemTy} :
    (values : List RuntimeValue) → (∀ value ∈ values, HasType unit loans value type) →
      HasTypeEach unit loans values type
  | [], _ => .nil
  | value :: values, typed =>
      .cons (typed value List.mem_cons_self)
        (HasTypeEach.of_forall values fun value member => typed value (List.mem_cons_of_mem _ member))

theorem declaredTypes?_resolves {ns : ValidatedNamespace} {arguments : List SemArg}
    {declared : Array FieldDecl} {fieldTypes : List SemTy}
    (resolved : declaredTypes? ns arguments declared = some fieldTypes) :
    ResolvesAll ns.tables arguments.toArray (declared.toList.map (·.type.typeId)) fieldTypes :=
  ResolvesAll.of_mapM resolved

mutual
theorem Denote.NTy.typedAs_of_core {unit : ValidatedUnit} :
    (τ : NTy) → (type : SemTy) → τ.typedAsCore unit type = true → τ.TypedAs unit type
  | .unit, type, checked => by
      simp only [NTy.typedAsCore, beq_iff_eq] at checked
      subst checked; exact .unit
  | .bool, type, checked => by
      simp only [NTy.typedAsCore, beq_iff_eq] at checked
      subst checked; exact .scalar rfl
  | .int width signed, type, checked => by
      simp only [NTy.typedAsCore, beq_iff_eq] at checked
      subst checked; exact .scalar rfl
  | .num, type, checked => by
      simp only [NTy.typedAsCore, beq_iff_eq] at checked
      subst checked; exact .scalar rfl
  | .address, type, checked => by
      simp only [NTy.typedAsCore, beq_iff_eq] at checked
      subst checked; exact .scalar rfl
  | .signer, type, checked => by
      simp only [NTy.typedAsCore, beq_iff_eq] at checked
      subst checked; exact .scalar rfl
  | .string, type, checked => by
      simp only [NTy.typedAsCore, beq_iff_eq] at checked
      subst checked; exact .scalar rfl
  | .bytes, type, checked => by
      simp only [NTy.typedAsCore, beq_iff_eq] at checked
      subst checked; exact .scalar rfl
  | .tuple elements, type, checked => by
      cases type <;> simp only [NTy.typedAsCore, Bool.false_eq_true] at checked
      exact .tuple (NRow.typedAs_of_check elements _ checked)
  | .ghost slot, type, checked => by
      cases type <;> simp only [NTy.typedAsCore, Bool.false_eq_true] at checked
      exact .ghost (NTy.typedAs_of_core slot _ checked)
  | .vector element, type, checked => by
      cases type with
      | vector type length =>
          cases length with
          | none =>
              simp only [NTy.typedAsCore] at checked
              exact .vector (NTy.typedAs_of_core element _ checked)
          | some _ => simp only [NTy.typedAsCore, Bool.false_eq_true] at checked
      | _ => simp only [NTy.typedAsCore, Bool.false_eq_true] at checked
  | .table key value, type, checked => by
      cases type with
      | vector element length =>
          cases length with
          | none =>
              cases element with
              | tuple types =>
                  match types, checked with
                  | [keyType, valueType], checked =>
                      simp only [NTy.typedAsCore, Bool.and_eq_true] at checked
                      exact .table (NTy.typedAs_of_core key _ checked.1)
                        (NTy.typedAs_of_core value _ checked.2)
                  | [], checked | [_], checked | _ :: _ :: _ :: _, checked =>
                      simp [NTy.typedAsCore] at checked
              | _ => simp [NTy.typedAsCore] at checked
          | some _ => simp [NTy.typedAsCore] at checked
      | _ => simp [NTy.typedAsCore] at checked
  | .struct source nativeArguments fields, type, checked => by
      cases type with
      | nominal name arguments =>
          simp only [NTy.typedAsCore, Bool.and_eq_true, beq_iff_eq] at checked
          obtain ⟨⟨⟨name_eq, arity⟩, typedArguments⟩, checked⟩ := checked
          split at checked
          · next layout layout_eq =>
              split at checked
              · next key value declaringNs namespace_eq =>
                  split at checked
                  · next ghostTypes ghostsResolved =>
                      exact .abstract name_eq arity
                        (NRow.argumentsTypedAs_of_check nativeArguments _ typedArguments) layout_eq
                        namespace_eq (ResolvesAll.of_mapM ghostsResolved)
                        (NRow.typedAs_of_check fields _ checked)
                  · cases checked
              · cases checked
          · next layout_eq =>
          split at checked
          · next declaringNs declared declaration_eq =>
              split at checked
              · next fieldTypes ghostTypes resolved ghostsResolved =>
                  exact .struct name_eq arity
                    (NRow.argumentsTypedAs_of_check nativeArguments arguments typedArguments)
                    declaration_eq
                    (declaredTypes?_resolves resolved)
                    (ResolvesAll.of_mapM ghostsResolved)
                    (NRow.typedAs_of_check fields _ checked)
              · cases checked
          · cases checked
      | _ => simp only [NTy.typedAsCore, Bool.false_eq_true] at checked
  | .enum source nativeArguments names rows distinct, type, checked => by
      cases type with
      | nominal name arguments =>
          simp only [NTy.typedAsCore, Bool.and_eq_true, beq_iff_eq] at checked
          obtain ⟨⟨⟨⟨⟨name_eq, arity⟩, typedArguments⟩, nonempty⟩, names_eq⟩, variants⟩ :=
            checked
          exact .enum name_eq arity
            (NRow.argumentsTypedAs_of_check nativeArguments arguments typedArguments)
            (by simpa [List.isEmpty_iff] using nonempty) names_eq
            (NRows.variantsTypedAs_of_check source arguments names rows variants)
      | _ => simp only [NTy.typedAsCore, Bool.false_eq_true] at checked
  | .ref referent, type, checked => by
      cases type with
      | reference kind type =>
          cases kind with
          | mutable =>
              simp only [NTy.typedAsCore] at checked
              exact .ref (NTy.typedAs_of_core referent _ checked)
          | shared => simp only [NTy.typedAsCore, Bool.false_eq_true] at checked
      | _ => simp only [NTy.typedAsCore, Bool.false_eq_true] at checked
  | .param index, type, checked => by
      cases type with
      | param other =>
          simp only [NTy.typedAsCore, beq_iff_eq] at checked
          subst checked; exact .param index
      | _ => simp only [NTy.typedAsCore, Bool.false_eq_true] at checked
  | .function parameters shared returned, type, checked => by
      cases type with
      | function parameterTypes result =>
          simp only [NTy.typedAsCore, Bool.and_eq_true, beq_iff_eq] at checked
          obtain ⟨⟨typedParameters, packed⟩, typedResults⟩ := checked
          exact .function (NRow.parametersTypedAs_of_check parameters shared parameterTypes
              typedParameters)
            (NRow.typedAs_of_check returned result.resultRow typedResults) packed.symm
      | _ => simp only [NTy.typedAsCore, Bool.false_eq_true] at checked

theorem Denote.NRow.typedAs_of_check {unit : ValidatedUnit} :
    (row : NRow) → (types : List SemTy) → row.typedAsCheck unit types = true →
      row.TypedAs unit types
  | .nil, [], _ => .nil
  | .cons τ rest, type :: types, checked => by
      simp only [NRow.typedAsCheck, Bool.and_eq_true] at checked
      exact .cons (NTy.typedAs_of_core τ _ checked.1)
        (NRow.typedAs_of_check rest types checked.2)
  | .nil, _ :: _, checked | .cons _ _, [], checked => by
      simp only [NRow.typedAsCheck, Bool.false_eq_true] at checked

theorem Denote.NRow.argumentsTypedAs_of_check {unit : ValidatedUnit} :
    (row : NRow) → (arguments : List SemArg) → row.argumentsCheck unit arguments = true →
      row.ArgumentsTypedAs unit arguments
  | .nil, [], _ => .nil
  | .cons τ rest, .type type :: arguments, checked => by
      simp only [NRow.argumentsCheck, Bool.and_eq_true] at checked
      exact .cons (NTy.typedAs_of_core τ _ checked.1)
        (NRow.argumentsTypedAs_of_check rest arguments checked.2)
  | .nil, _ :: _, checked | .cons _ _, [], checked | .cons _ _, .const _ :: _, checked
  | .cons _ _, .erased :: _, checked => by
      simp only [NRow.argumentsCheck, Bool.false_eq_true] at checked

theorem Denote.NRow.parametersTypedAs_of_check {unit : ValidatedUnit} :
    (row : NRow) → (shared : List Bool) → (types : List SemTy) →
      row.parametersCheck unit shared types = true → row.ParametersTypedAs unit shared types
  | .nil, [], [], _ => .nil
  | .cons τ rest, false :: shared, type :: types, checked => by
      simp only [NRow.parametersCheck, Bool.and_eq_true, beq_iff_eq] at checked
      exact .value (NTy.typedAs_of_core τ type checked.1.2) checked.1.1
        (NRow.parametersTypedAs_of_check rest shared types checked.2)
  | .cons τ rest, true :: shared, type :: types, checked => by
      match type, checked with
      | .reference .shared referent, checked =>
          simp only [NRow.parametersCheck, Bool.and_eq_true, beq_iff_eq] at checked
          exact .shared (NTy.typedAs_of_core τ referent checked.1.2) checked.1.1
            (NRow.parametersTypedAs_of_check rest shared types checked.2)
      | .reference .mutable _, checked | .unit, checked | .never, checked | .bool, checked
      | .character, checked | .string, checked | .bytes, checked | .address, checked
      | .signer, checked | .integer _ _, checked | .tuple _, checked | .vector _ _, checked
      | .nominal _ _, checked | .function _ _, checked | .profile _, checked
      | .param _, checked => simp only [NRow.parametersCheck, Bool.false_eq_true] at checked
  | .nil, _ :: _, _, checked | .nil, [], _ :: _, checked | .cons _ _, [], _, checked
  | .cons _ _, _ :: _, [], checked => by
      simp only [NRow.parametersCheck, Bool.false_eq_true] at checked

theorem Denote.NRows.variantsTypedAs_of_check {unit : ValidatedUnit} (source : StructHandle)
    (arguments : List SemArg) :
    (names : List String) → (rows : NRows) →
      NRows.variantsCheck unit source arguments names rows = true →
      NRows.VariantsTypedAs unit source arguments names rows
  | [], .nil, _ => .nil
  | name :: names, .cons fields rest, checked => by
      simp only [NRows.variantsCheck, Bool.and_eq_true] at checked
      obtain ⟨variant, later⟩ := checked
      split at variant
      · next declaringNs declared declaration_eq =>
          split at variant
          · next fieldTypes ghostTypes resolved ghostsResolved =>
              exact .cons declaration_eq (declaredTypes?_resolves resolved)
                (ResolvesAll.of_mapM ghostsResolved)
                (NRow.typedAs_of_check fields (fieldTypes ++ ghostTypes.map .ghost) variant)
                (NRows.variantsTypedAs_of_check source arguments names rest later)
          · cases variant
      · cases variant
  | [], .cons _ _, checked | _ :: _, .nil, checked => by
      simp only [NRows.variantsCheck, Bool.false_eq_true] at checked
end

theorem Denote.NTy.typedAs_of_check {unit : ValidatedUnit} {τ : NTy} {type : SemTy}
    (checked : τ.typedAsCheck unit type = true) : τ.TypedAs unit type :=
  NTy.typedAs_of_core τ _ checked


theorem _root_.VerexIR.SemTy.sizeOf_unshared_le : (type : SemTy) →
    sizeOf type.unshared ≤ sizeOf type
  | .reference .shared referent => by
      have := SemTy.sizeOf_unshared_le referent
      simp only [SemTy.unshared, SemTy.reference.sizeOf_spec]
      omega
  | .reference .mutable _ | .unit | .never | .bool | .character | .string | .bytes | .address
  | .signer | .integer _ _ | .tuple _ | .vector _ _ | .nominal _ _ | .function _ _ | .profile _
  | .param _ | .ghost _ => Nat.le_refl _

/-- Ghost slot types substitute through their slots. -/
theorem _root_.VerexIR.SemTy.ghosts_subst (types : List SemTy) (arguments : Array SemArg) :
    (types.map SemTy.ghost).map (·.subst arguments) =
      (types.map (·.subst arguments)).map SemTy.ghost := by
  simp [SemTy.subst]

theorem ResolvesAll.subst {tables : Tables} {env : Array SemArg} (arguments : Array SemArg) :
    {typeIds : List TypeId} → {types : List SemTy} → ResolvesAll tables env typeIds types →
      ResolvesAll tables (env.map (·.subst arguments)) typeIds (types.map (·.subst arguments))
  | _, _, .nil => .nil
  | _, _, .cons head tail => .cons (head.subst arguments) (ResolvesAll.subst arguments tail)

theorem SemArg.toArray_substList (arguments : Array SemArg) (semArguments : List SemArg) :
    semArguments.toArray.map (·.subst arguments) =
      (SemArg.substList arguments semArguments).toArray := by
  rw [SemArg.substList_eq_map, List.map_toArray]

theorem _root_.VerexIR.SemTy.subst_pack (arguments : Array SemArg) :
    (results : List SemTy) →
      (SemTy.pack results).subst arguments = SemTy.pack (results.map (·.subst arguments))
  | [] => rfl
  | [_] => rfl
  | _ :: _ :: _ => by simp only [SemTy.pack, SemTy.subst, SemTy.substList_eq_map, List.map_cons]

/-- The arguments a substitution and an instantiation agree on: at each
allowed parameter, a type a native argument reads as, and not a shared
reference. -/
def ArgumentsAgree (unit : ValidatedUnit) (allowed : Nat → Bool) (types : Nat → NTy)
    (arguments : Array SemArg) : Prop :=
  ∀ index, allowed index = true → ∃ type, arguments[index]? = some (.type type) ∧
    (types index).TypedAs unit type ∧ type.unshared = type

/-- A type read as one that is not a type parameter and not a shared
reference stays one that is not a shared reference under substitution. -/
theorem _root_.VerexIR.SemTy.unshared_subst {arguments : Array SemArg} :
    (type : SemTy) → type.unshared = type → (∀ index, type ≠ .param index) →
      (type.subst arguments).unshared = type.subst arguments
  | .reference .shared referent, observed, _ => by
      have bounded := SemTy.sizeOf_unshared_le referent
      simp only [SemTy.unshared] at observed
      rw [observed] at bounded
      simp only [SemTy.reference.sizeOf_spec] at bounded
      omega
  | .param index, _, notParam => absurd rfl (notParam index)
  | .reference .mutable _, _, _ | .unit, _, _ | .never, _, _ | .bool, _, _ | .character, _, _
  | .string, _, _ | .bytes, _, _ | .address, _, _ | .signer, _, _ | .integer _ _, _, _
  | .tuple _, _, _ | .vector _ _, _, _ | .nominal _ _, _, _ | .function _ _, _, _
  | .profile _, _, _ | .ghost _, _, _ => rfl

/-- A type parameter is read only as itself. -/
theorem Denote.NTy.TypedAs.param_of {unit : ValidatedUnit} {τ : NTy} {index : Nat}
    (typed : τ.TypedAs unit (.param index)) : τ = .param index := by
  cases typed with
  | scalar scalar => cases τ <;> simp [NTy.scalarType?] at scalar
  | param => rfl

/-- What a type is read as stays no shared reference under a substitution
whose arguments are none. -/
theorem _root_.VerexIR.SemTy.unshared_subst_typed {unit : ValidatedUnit}
    {allowed : Nat → Bool} {types : Nat → NTy} {arguments : Array SemArg}
    (agree : ArgumentsAgree unit allowed types arguments) {τ : NTy} {type : SemTy}
    (typed : τ.TypedAs unit type) (mentioned : τ.paramsAll allowed = true)
    (observed : type.unshared = type) :
    (type.subst arguments).unshared = type.subst arguments := by
  by_cases isParam : ∃ index, type = .param index
  · obtain ⟨index, rfl⟩ := isParam
    have := typed.param_of
    subst this
    simp only [NTy.paramsAll] at mentioned
    obtain ⟨argument, found, _, unshared⟩ := agree index mentioned
    simp only [SemTy.subst, found]
    exact unshared
  · exact SemTy.unshared_subst _ observed fun index same => isParam ⟨index, same⟩

/-- An owner's canonical field types substitute through their key, value, and
ghost types; its leading fields are closed. -/
theorem _root_.VerexIR.Validation.OwnerLayout.canonicalTypes_subst (layout : OwnerLayout)
    (ghostTypes : List SemTy) (key value : SemTy) (arguments : Array SemArg) :
    (layout.canonicalTypes ghostTypes key value).map (·.subst arguments) =
      layout.canonicalTypes (ghostTypes.map (·.subst arguments)) (key.subst arguments)
        (value.subst arguments) := by
  simp only [OwnerLayout.canonicalTypes, OwnerLayout.leadingTypes, List.map_append,
    SemTy.ghosts_subst]
  congr 1
  by_cases handle : layout.canonical.handle = true <;>
    by_cases dummy : layout.canonical.dummy = true <;>
    by_cases length : layout.canonical.length = true <;> simp [handle, dummy, length, SemTy.subst]

mutual
/-- A native type reads, under a substitution, as its reading under the
arguments it agrees with. -/
theorem Denote.NTy.TypedAs.subst {unit : ValidatedUnit} {allowed : Nat → Bool}
    {types : Nat → NTy} {arguments : Array SemArg}
    (agree : ArgumentsAgree unit allowed types arguments) :
    {τ : NTy} → {type : SemTy} → τ.TypedAs unit type → τ.paramsAll allowed = true →
      (τ.substWith types).TypedAs unit (type.subst arguments)
  | .bool, _, .scalar scalar, _ | .int _ _, _, .scalar scalar, _ | .num, _, .scalar scalar, _ | .address, _, .scalar scalar, _
  | .signer, _, .scalar scalar, _ | .string, _, .scalar scalar, _
  | .bytes, _, .scalar scalar, _ => by
      simp only [NTy.scalarType?, Option.some.injEq] at scalar
      subst scalar
      exact .scalar rfl
  | .unit, _, .scalar scalar, _ | .tuple _, _, .scalar scalar, _
  | .struct _ _ _, _, .scalar scalar, _
  | .enum _ _ _ _ _, _, .scalar scalar, _ | .vector _, _, .scalar scalar, _
  | .table _ _, _, .scalar scalar, _ | .ghost _, _, .scalar scalar, _
  | .ref _, _, .scalar scalar, _ | .param _, _, .scalar scalar, _
  | .function _ _ _, _, .scalar scalar, _ => by simp [NTy.scalarType?] at scalar
  | _, _, .unit, _ => .unit
  | _, _, .tuple typed, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, SemTy.subst, SemTy.substList_eq_map]
      exact .tuple (NRow.TypedAs.subst agree typed mentioned)
  | _, _, .vector typed, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      exact .vector (NTy.TypedAs.subst agree typed mentioned)
  | _, _, .table typedKey typedValue, mentioned => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.substWith, SemTy.subst, SemTy.substList]
      exact .table (NTy.TypedAs.subst agree typedKey mentioned.1)
        (NTy.TypedAs.subst agree typedValue mentioned.2)
  | _, _, .struct name_eq arity typedArguments declaration_eq resolve ghostsResolve typed,
      mentioned => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.substWith, SemTy.subst]
      have typedSubst := NRow.TypedAs.subst agree typed mentioned.2
      rw [List.map_append, List.map_append, SemTy.ghosts_subst, ← contentsTypes_substList]
        at typedSubst
      refine .struct name_eq ?_ (NRow.ArgumentsTypedAs.subst agree typedArguments mentioned.1)
        declaration_eq ?_ ?_ typedSubst
      · rw [SemArg.substList_eq_map, List.length_map]; exact arity
      · rw [← SemArg.toArray_substList]; exact ResolvesAll.subst arguments resolve
      · rw [← SemArg.toArray_substList]; exact ResolvesAll.subst arguments ghostsResolve
  | _, _, .abstract name_eq arity typedArguments layout_eq namespace_eq ghostsResolve typed,
      mentioned => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.substWith, SemTy.subst, SemArg.substList, SemArg.subst]
      have typedSubst := NRow.TypedAs.subst agree typed mentioned.2
      rw [OwnerLayout.canonicalTypes_subst] at typedSubst
      refine .abstract name_eq arity ?_ layout_eq namespace_eq ?_ typedSubst
      · simpa [SemArg.substList, SemArg.subst] using
          NRow.ArgumentsTypedAs.subst agree typedArguments mentioned.1
      · simpa [SemArg.subst] using ResolvesAll.subst arguments ghostsResolve
  | _, _, .enum name_eq arity typedArguments nonempty names_eq variants, mentioned => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      simp only [NTy.substWith, SemTy.subst]
      refine .enum name_eq ?_ (NRow.ArgumentsTypedAs.subst agree typedArguments mentioned.1)
        nonempty names_eq (NRows.VariantsTypedAs.subst agree variants mentioned.2)
      rw [SemArg.substList_eq_map, List.length_map]; exact arity
  | _, _, .ref typed, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      exact .ref (NTy.TypedAs.subst agree typed mentioned)
  | _, _, .ghost typed, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      simp only [NTy.substWith, SemTy.subst]
      exact .ghost (NTy.TypedAs.subst agree typed mentioned)
  | _, _, .param index, mentioned => by
      simp only [NTy.paramsAll] at mentioned
      obtain ⟨type, argument, typed, _⟩ := agree index mentioned
      simp only [NTy.substWith, SemTy.subst, argument]
      exact typed
  | _, _, .function typedParameters typedResults packed, mentioned => by
      simp only [NTy.paramsAll, Bool.and_eq_true] at mentioned
      subst packed
      simp only [NTy.substWith, SemTy.subst, SemTy.substList_eq_map]
      exact .function (NRow.ParametersTypedAs.subst agree typedParameters mentioned.1)
        (NRow.TypedAs.subst agree typedResults mentioned.2) (SemTy.subst_pack arguments _)

theorem Denote.NRow.TypedAs.subst {unit : ValidatedUnit} {allowed : Nat → Bool}
    {types : Nat → NTy} {arguments : Array SemArg}
    (agree : ArgumentsAgree unit allowed types arguments) :
    {row : NRow} → {semTypes : List SemTy} → row.TypedAs unit semTypes →
      row.paramsAll allowed = true →
      (row.substWith types).TypedAs unit (semTypes.map (·.subst arguments))
  | _, _, .nil, _ => .nil
  | _, _, .cons head tail, mentioned => by
      simp only [NRow.paramsAll, Bool.and_eq_true] at mentioned
      exact .cons (NTy.TypedAs.subst agree head mentioned.1)
        (NRow.TypedAs.subst agree tail mentioned.2)

theorem Denote.NRow.ArgumentsTypedAs.subst {unit : ValidatedUnit} {allowed : Nat → Bool}
    {types : Nat → NTy} {arguments : Array SemArg}
    (agree : ArgumentsAgree unit allowed types arguments) :
    {row : NRow} → {semArguments : List SemArg} → row.ArgumentsTypedAs unit semArguments →
      row.paramsAll allowed = true →
      (row.substWith types).ArgumentsTypedAs unit (SemArg.substList arguments semArguments)
  | _, _, .nil, _ => .nil
  | _, _, .cons head tail, mentioned => by
      simp only [NRow.paramsAll, Bool.and_eq_true] at mentioned
      exact .cons (NTy.TypedAs.subst agree head mentioned.1)
        (NRow.ArgumentsTypedAs.subst agree tail mentioned.2)

theorem Denote.NRow.ParametersTypedAs.subst {unit : ValidatedUnit} {allowed : Nat → Bool}
    {types : Nat → NTy} {arguments : Array SemArg}
    (agree : ArgumentsAgree unit allowed types arguments) :
    {row : NRow} → {shared : List Bool} → {semTypes : List SemTy} →
      row.ParametersTypedAs unit shared semTypes → row.paramsAll allowed = true →
      (row.substWith types).ParametersTypedAs unit shared (semTypes.map (·.subst arguments))
  | _, _, _, .nil, _ => .nil
  | .cons τ _, _, _, .value (type := type) typed observed later, mentioned => by
      simp only [NRow.paramsAll, Bool.and_eq_true] at mentioned
      refine .value (NTy.TypedAs.subst agree typed mentioned.1) ?_
        (NRow.ParametersTypedAs.subst agree later mentioned.2)
      exact SemTy.unshared_subst_typed agree typed mentioned.1 observed
  | .cons τ _, _, _, .shared (type := type) typed observed later, mentioned => by
      simp only [NRow.paramsAll, Bool.and_eq_true] at mentioned
      simp only [List.map_cons, SemTy.subst]
      refine .shared (NTy.TypedAs.subst agree typed mentioned.1) ?_
        (NRow.ParametersTypedAs.subst agree later mentioned.2)
      exact SemTy.unshared_subst_typed agree typed mentioned.1 observed

theorem Denote.NRows.VariantsTypedAs.subst {unit : ValidatedUnit} {allowed : Nat → Bool}
    {types : Nat → NTy} {arguments : Array SemArg}
    (agree : ArgumentsAgree unit allowed types arguments) :
    {source : StructHandle} → {semArguments : List SemArg} → {names : List String} →
      {rows : NRows} → NRows.VariantsTypedAs unit source semArguments names rows →
      rows.paramsAll allowed = true →
      NRows.VariantsTypedAs unit source (SemArg.substList arguments semArguments) names
        (rows.substWith types)
  | _, _, _, _, .nil, _ => .nil
  | _, _, _, _, .cons declaration_eq resolve ghostsResolve typed later, mentioned => by
      simp only [NRows.paramsAll, Bool.and_eq_true] at mentioned
      have typedSubst := NRow.TypedAs.subst agree typed mentioned.1
      rw [List.map_append, SemTy.ghosts_subst] at typedSubst
      refine .cons declaration_eq ?_ ?_ typedSubst
        (NRows.VariantsTypedAs.subst agree later mentioned.2)
      · rw [← SemArg.toArray_substList]; exact ResolvesAll.subst arguments resolve
      · rw [← SemArg.toArray_substList]; exact ResolvesAll.subst arguments ghostsResolve
end

mutual
/-- A native type reads as at most one semantic type. -/
theorem Denote.NTy.readsUnique {unit : ValidatedUnit} :
    (τ : NTy) → {left right : SemTy} →
    τ.TypedAs unit left → τ.TypedAs unit right → left = right
  | .unit, _, _, leftTyped, rightTyped => by
      cases leftTyped with
      | scalar scalar => simp [NTy.scalarType?] at scalar
      | unit => cases rightTyped with
        | scalar scalar => simp [NTy.scalarType?] at scalar
        | unit => rfl
  | .bool, _, _, .scalar leftScalar, .scalar rightScalar
  | .int _ _, _, _, .scalar leftScalar, .scalar rightScalar
  | .num, _, _, .scalar leftScalar, .scalar rightScalar
  | .address, _, _, .scalar leftScalar, .scalar rightScalar
  | .signer, _, _, .scalar leftScalar, .scalar rightScalar
  | .string, _, _, .scalar leftScalar, .scalar rightScalar
  | .bytes, _, _, .scalar leftScalar, .scalar rightScalar => by
      rw [leftScalar] at rightScalar; exact Option.some.inj rightScalar
  | .tuple elements, _, _, leftTyped, rightTyped => by
      cases leftTyped with
      | scalar scalar => simp [NTy.scalarType?] at scalar
      | tuple leftRow => cases rightTyped with
        | scalar scalar => simp [NTy.scalarType?] at scalar
        | tuple rightRow => rw [NRow.readsUnique elements leftRow rightRow]
  | .vector element, _, _, leftTyped, rightTyped => by
      cases leftTyped with
      | scalar scalar => simp [NTy.scalarType?] at scalar
      | vector leftElement => cases rightTyped with
        | scalar scalar => simp [NTy.scalarType?] at scalar
        | vector rightElement => rw [NTy.readsUnique element leftElement rightElement]
  | .table key value, _, _, leftTyped, rightTyped => by
      cases leftTyped with
      | scalar scalar => simp [NTy.scalarType?] at scalar
      | table leftKey leftValue => cases rightTyped with
        | scalar scalar => simp [NTy.scalarType?] at scalar
        | table rightKey rightValue =>
            rw [NTy.readsUnique key leftKey rightKey, NTy.readsUnique value leftValue rightValue]
  | .ref referent, _, _, leftTyped, rightTyped => by
      cases leftTyped with
      | scalar scalar => simp [NTy.scalarType?] at scalar
      | ref leftReferent => cases rightTyped with
        | scalar scalar => simp [NTy.scalarType?] at scalar
        | ref rightReferent => rw [NTy.readsUnique referent leftReferent rightReferent]
  | .ghost slot, _, _, leftTyped, rightTyped => by
      cases leftTyped with
      | scalar scalar => simp [NTy.scalarType?] at scalar
      | ghost leftSlot => cases rightTyped with
        | scalar scalar => simp [NTy.scalarType?] at scalar
        | ghost rightSlot => rw [NTy.readsUnique slot leftSlot rightSlot]
  | .param _, _, _, leftTyped, rightTyped => by
      cases leftTyped with
      | scalar scalar => simp [NTy.scalarType?] at scalar
      | param => cases rightTyped with
        | scalar scalar => simp [NTy.scalarType?] at scalar
        | param => rfl
  | .function parameters _ returned, _, _, leftTyped, rightTyped => by
      cases leftTyped with
      | scalar scalar => simp [NTy.scalarType?] at scalar
      | function leftParameters leftResults leftPacked => cases rightTyped with
        | scalar scalar => simp [NTy.scalarType?] at scalar
        | function rightParameters rightResults rightPacked =>
            rw [NRow.parametersUnique parameters leftParameters rightParameters, leftPacked,
              rightPacked, NRow.readsUnique returned leftResults rightResults]
  | .struct _ nativeArguments _, _, _, leftTyped, rightTyped => by
      cases leftTyped with
      | scalar scalar => simp [NTy.scalarType?] at scalar
      | struct leftName _ leftArguments _ _ _ =>
        cases rightTyped with
        | scalar scalar => simp [NTy.scalarType?] at scalar
        | struct rightName _ rightArguments _ _ _ =>
            rw [leftName, Option.some.injEq] at rightName
            rw [rightName, NRow.argumentsUnique nativeArguments leftArguments rightArguments]
        | abstract rightName _ rightArguments _ _ _ _ =>
            rw [leftName, Option.some.injEq] at rightName
            rw [rightName, NRow.argumentsUnique nativeArguments leftArguments rightArguments]
      | abstract leftName _ leftArguments _ _ _ _ =>
        cases rightTyped with
        | scalar scalar => simp [NTy.scalarType?] at scalar
        | struct rightName _ rightArguments _ _ _ =>
            rw [leftName, Option.some.injEq] at rightName
            rw [rightName, NRow.argumentsUnique nativeArguments leftArguments rightArguments]
        | abstract rightName _ rightArguments _ _ _ _ =>
            rw [leftName, Option.some.injEq] at rightName
            rw [rightName, NRow.argumentsUnique nativeArguments leftArguments rightArguments]
  | .enum _ nativeArguments _ _ _, _, _, leftTyped, rightTyped => by
      cases leftTyped with
      | scalar scalar => simp [NTy.scalarType?] at scalar
      | enum leftName _ leftArguments _ _ _ =>
        cases rightTyped with
        | scalar scalar => simp [NTy.scalarType?] at scalar
        | enum rightName _ rightArguments _ _ _ =>
            rw [leftName, Option.some.injEq] at rightName
            rw [rightName, NRow.argumentsUnique nativeArguments leftArguments rightArguments]

/-- A row reads as at most one row of semantic types. -/
theorem Denote.NRow.readsUnique {unit : ValidatedUnit} :
    (row : NRow) → {left right : List SemTy} →
    row.TypedAs unit left → row.TypedAs unit right → left = right
  | .nil, _, _, .nil, .nil => rfl
  | .cons τ rest, _, _, .cons leftHead leftTail, .cons rightHead rightTail => by
      rw [NTy.readsUnique τ leftHead rightHead, NRow.readsUnique rest leftTail rightTail]

/-- A row reads as at most one row of type arguments. -/
theorem Denote.NRow.argumentsUnique {unit : ValidatedUnit} :
    (row : NRow) → {left right : List SemArg} →
    row.ArgumentsTypedAs unit left → row.ArgumentsTypedAs unit right → left = right
  | .nil, _, _, .nil, .nil => rfl
  | .cons τ rest, _, _, .cons leftHead leftTail, .cons rightHead rightTail => by
      rw [NTy.readsUnique τ leftHead rightHead, NRow.argumentsUnique rest leftTail rightTail]

theorem Denote.NRow.parametersUnique {unit : ValidatedUnit} : (row : NRow) →
    {shared : List Bool} → {left right : List SemTy} → row.ParametersTypedAs unit shared left →
      row.ParametersTypedAs unit shared right → left = right
  | .nil, _, _, _, .nil, .nil => rfl
  | .cons τ rest, _, _, _, .value leftTyped _ leftLater, .value rightTyped _ rightLater => by
      rw [NTy.readsUnique τ leftTyped rightTyped,
        NRow.parametersUnique rest leftLater rightLater]
  | .cons τ rest, _, _, _, .shared leftTyped _ leftLater, .shared rightTyped _ rightLater => by
      rw [NTy.readsUnique τ leftTyped rightTyped,
        NRow.parametersUnique rest leftLater rightLater]
end

/-- The sharing a function's declared parameters have, as a function type
marks it. -/
def parameterSharing (unit : ValidatedUnit) (function : FunctionHandle)
    (declaration : FunctionDecl Validation.FunctionBody) : List Bool :=
  declaration.signature.parameters.toList.map fun parameter =>
    sharedReference (unitTypes unit) function.namespaceId parameter.typeUse.typeId

/-- Whether a closure target's signature reads alike natively and
semantically, over its own environment. -/
def signatureAgrees (unit : ValidatedUnit) (function : FunctionHandle) : Bool :=
  match unit.namespaces[function.namespaceId.index]? with
  | none => true
  | some targetNs => match targetNs.functions[function.functionId.index]? with
    | none => true
    | some declaration => match closureSignature? unit function with
      | none => true
      | some (parameters, results) =>
          match StaticTyping.signatureTypes? targetNs declaration
              (StaticTyping.staticEnv declaration.signature.generics) with
          | some (parameterTypes, resultTypes) =>
              NRow.parametersCheck unit (NRow.ofList parameters)
                  (parameterSharing unit function declaration) parameterTypes &&
                NRow.typedAsCheck unit (NRow.ofList results) resultTypes
          | none => false

/-- Whether a unit's native and semantic readings agree: every closed type
identifier stating no reference reads natively as its semantic type, and
every closure target's signature reads alike. Decided by evaluation over the
unit. -/
def typesAgreeCheck (unit : ValidatedUnit) : Bool :=
  (List.range unit.namespaces.size).all fun index =>
      match unit.namespaces[index]? with
      | none => true
      | some ns =>
          (List.range ns.tables.types.size).all (fun typeIndex =>
            match ntyOf unit ⟨index⟩ ⟨typeIndex⟩,
                StaticTyping.resolveIn ns #[] ⟨typeIndex⟩ with
            | some τ, some type => !type.referenceFree || τ.typedAsCheck unit type
            | _, _ => true) &&
          (List.range ns.functions.size).all fun functionIndex =>
            signatureAgrees unit ⟨⟨index⟩, ⟨functionIndex⟩⟩

/-- A unit's native and semantic readings agree (`typesAgreeCheck`). -/
structure TypesAgree (unit : ValidatedUnit) : Prop where
  types : ∀ namespaceId typeId ns τ type, unit.namespaces[namespaceId.index]? = some ns →
    ntyOf unit namespaceId typeId = some τ → StaticTyping.resolveIn ns #[] typeId = some type →
    type.referenceFree = true → τ.TypedAs unit type
  signatures : ∀ function, signatureAgrees unit function = true

theorem TypesAgree.ofCheck {unit : ValidatedUnit} (check : typesAgreeCheck unit = true) :
    TypesAgree unit := by
  simp only [typesAgreeCheck, List.all_eq_true, List.mem_range] at check
  have namespaces := check
  refine ⟨fun namespaceId typeId ns τ type namespace_eq native resolved free => ?_,
    fun function => ?_⟩
  · have inNamespaces : namespaceId.index < unit.namespaces.size := by
      rcases Nat.lt_or_ge namespaceId.index unit.namespaces.size with inside | beyond
      · exact inside
      · simp [Array.getElem?_eq_none beyond] at namespace_eq
    have atNamespace := namespaces namespaceId.index inNamespaces
    rw [namespace_eq] at atNamespace
    simp only [Bool.and_eq_true, List.all_eq_true, List.mem_range] at atNamespace
    have inTable : typeId.index < ns.tables.types.size := by
      rcases Nat.lt_or_ge typeId.index ns.tables.types.size with inside | beyond
      · exact inside
      · simp [StaticTyping.resolveIn, SemTy.resolveFuel, Array.getElem?_eq_none beyond] at resolved
    have atType := atNamespace.1 typeId.index inTable
    have identifier : (⟨namespaceId.index⟩ : NamespaceId) = namespaceId := rfl
    have typeIdentifier : (⟨typeId.index⟩ : TypeId) = typeId := rfl
    rw [identifier, typeIdentifier, native, resolved] at atType
    simp only [free, Bool.not_true, Bool.false_or] at atType
    exact NTy.typedAs_of_check atType
  · cases namespace_eq : unit.namespaces[function.namespaceId.index]? with
    | none => simp [signatureAgrees, namespace_eq]
    | some ns =>
        have inNamespaces : function.namespaceId.index < unit.namespaces.size := by
          rcases Nat.lt_or_ge function.namespaceId.index unit.namespaces.size with inside | beyond
          · exact inside
          · simp [Array.getElem?_eq_none beyond] at namespace_eq
        have atNamespace := namespaces function.namespaceId.index inNamespaces
        rw [namespace_eq] at atNamespace
        simp only [Bool.and_eq_true, List.all_eq_true, List.mem_range] at atNamespace
        cases function_eq : ns.functions[function.functionId.index]? with
        | none => simp [signatureAgrees, namespace_eq, function_eq]
        | some declaration =>
            have inFunctions : function.functionId.index < ns.functions.size := by
              rcases Nat.lt_or_ge function.functionId.index ns.functions.size with inside | beyond
              · exact inside
              · simp [Array.getElem?_eq_none beyond] at function_eq
            exact atNamespace.2 function.functionId.index inFunctions

mutual
/-- Whether a type mentions no function type. -/
def Denote.NTy.closureFree : NTy → Bool
  | .function _ _ _ => false
  | .tuple elements => Denote.NRow.closureFree elements
  | .struct _ _ fields => Denote.NRow.closureFree fields
  | .enum _ _ _ rows _ => Denote.NRows.closureFree rows
  | .vector element => Denote.NTy.closureFree element
  | .table key value => Denote.NTy.closureFree key && Denote.NTy.closureFree value
  | .ref referent => Denote.NTy.closureFree referent
  | .ghost slot => Denote.NTy.closureFree slot
  | .unit | .bool | .int _ _ | .num | .address | .signer | .string | .bytes | .param _ => true

def Denote.NRow.closureFree : NRow → Bool
  | .nil => true
  | .cons τ rest => Denote.NTy.closureFree τ && Denote.NRow.closureFree rest

def Denote.NRows.closureFree : NRows → Bool
  | .nil => true
  | .cons fields rest => Denote.NRow.closureFree fields && Denote.NRows.closureFree rest
end

/-- Every resource type of a unit corresponds to its key's type: the
semantic type a type identifier resolves to is one its native type reads
as, and the native type is closed and holds no reference; where it states a
function type, the unit's readings agree. -/
def ResourcesTyped (unit : ValidatedUnit) : Prop :=
  ∀ namespaceId typeId resource, runtimeResourceOf unit namespaceId typeId = some resource →
    ∃ ns type, unit.namespaces[namespaceId.index]? = some ns ∧
      Resolves ns.tables #[] typeId type ∧ resource.type.TypedAs unit type ∧
      resource.type.paramFree = true ∧ resource.type.refFree = true ∧
      (resource.type.closureFree = true ∨ TypesAgree unit)

/-- Whether one type identifier's resource type corresponds to its type,
given whether the unit's readings agree. -/
def resourceTypedCheck (unit : ValidatedUnit) (agree : Bool) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace) (typeId : TypeId) : Bool :=
  match runtimeResourceOf unit namespaceId typeId with
  | none => true
  | some resource =>
      resource.type.paramFree && resource.type.refFree && (resource.type.closureFree || agree) &&
        match StaticTyping.resolveIn ns #[] typeId with
        | some type => resource.type.typedAsCheck unit type
        | none => false

/-- Whether every resource type of a unit corresponds to its type, decided
by evaluation over its type tables; the readings' agreement is evaluated
only for a resource type stating a function type. -/
def resourcesTypedCheck (unit : ValidatedUnit) : Bool :=
  (List.range unit.namespaces.size).all fun index =>
    match unit.namespaces[index]? with
    | none => true
    | some ns => (List.range ns.tables.types.size).all fun typeIndex =>
        resourceTypedCheck unit (typesAgreeCheck unit) ⟨index⟩ ns ⟨typeIndex⟩

/-- A type identifier outside its namespace's table denotes no resource. -/
theorem runtimeResourceOf_beyond {unit : ValidatedUnit} {namespaceId : NamespaceId} {ns : ValidatedNamespace}
    (namespace_eq : unit.namespaces[namespaceId.index]? = some ns) {typeId : TypeId}
    (beyond : ns.tables.types.size ≤ typeId.index) : runtimeResourceOf unit namespaceId typeId = none := by
  simp [runtimeResourceOf, resourceOf, unitTypes, namespace_eq, Array.getElem?_eq_none beyond]

theorem ResourcesTyped.ofCheck {unit : ValidatedUnit} (check : resourcesTypedCheck unit = true) :
    ResourcesTyped unit := by
  intro ⟨namespaceIndex⟩ ⟨typeIndex⟩ resource named
  have namespaceAt : ∃ ns, unit.namespaces[namespaceIndex]? = some ns := by
    cases namespace_eq : unit.namespaces[namespaceIndex]? with
    | some ns => exact ⟨ns, rfl⟩
    | none => simp [runtimeResourceOf, resourceOf, unitTypes, namespace_eq] at named
  obtain ⟨ns, namespace_eq⟩ := namespaceAt
  have inNamespaces : namespaceIndex < unit.namespaces.size := by
    rcases Nat.lt_or_ge namespaceIndex unit.namespaces.size with inside | beyond
    · exact inside
    · simp [Array.getElem?_eq_none beyond] at namespace_eq
  have inTable : typeIndex < ns.tables.types.size := by
    rcases Nat.lt_or_ge typeIndex ns.tables.types.size with inside | beyond
    · exact inside
    · rw [runtimeResourceOf_beyond (namespaceId := ⟨namespaceIndex⟩) namespace_eq
        (typeId := ⟨typeIndex⟩) beyond] at named
      cases named
  have checked := List.all_eq_true.mp check namespaceIndex (List.mem_range.mpr inNamespaces)
  simp only [namespace_eq] at checked
  have typed := List.all_eq_true.mp checked typeIndex (List.mem_range.mpr inTable)
  unfold resourceTypedCheck at typed
  rw [named] at typed
  simp only [Bool.and_eq_true] at typed
  obtain ⟨⟨⟨closed, plain⟩, closures⟩, typed⟩ := typed
  have closures : resource.type.closureFree = true ∨ TypesAgree unit := by
    rcases (Bool.or_eq_true _ _).mp closures with closureFree | agree
    · exact .inl closureFree
    · exact .inr (TypesAgree.ofCheck agree)
  split at typed
  · next type resolved =>
      exact ⟨ns, type, namespace_eq, ⟨_, resolved⟩, NTy.typedAs_of_check typed, closed, plain,
        closures⟩
  · cases typed

/-- Declared types that resolve by evaluation. -/
theorem ResolvesAll.ofResolveIn {ns : ValidatedNamespace} {typeIds : List TypeId}
    {types : List SemTy}
    (resolved : typeIds.mapM (StaticTyping.resolveIn ns #[]) = some types) :
    ResolvesAll ns.tables #[] typeIds types :=
  ResolvesAll.of_mapM resolved

/-- A certified integer fits its type on every target. -/
theorem holdsAt_val (pointerWidth : Option Nat) {width : Nat} {signed : Bool}
    (value : SpecInt (.bits width) signed) :
    StaticTyping.holdsAt pointerWidth (.bits width) signed value.val = true := by
  have fits := value.fits
  simp only [IntegerValueFits] at fits
  simp [StaticTyping.holdsAt, StaticTyping.targetWidth, fits]

section
variable {unit : Validation.ValidatedUnit} [Skolems unit]

/-- Every runtime value a closed native type holding no reference admits
(`NTy.admits`) has a typed runtime representation: a runtime value
normalizing to it (`SemanticOperations.normalize`), typed at the semantic
type the native type reads as, a function type only in a unit whose readings
agree. A value of an intrinsic map owner is abstract, and its representation
is a value of the owner's Move type holding its entries, slots, and handle;
this is assumed of a unit as its natives' contracts are
(`designs/intrinsic-maps.md`, "Abstract representation"). -/
def OwnersRepresentable (unit : ValidatedUnit) : Prop :=
  ∀ (τ : NTy) (type : SemTy) (value : RuntimeValue), τ.TypedAs unit type → τ.paramFree = true →
    τ.refFree = true → (τ.closureFree = true ∨ TypesAgree unit) → NTy.admits unit τ value = true →
    ∃ representation, SemanticOperations.normalize unit representation = value ∧
      HasType unit (fun _ => none) representation type

/-- `OwnersRepresentable` along a row. -/
theorem NRow.represented_of_admits {unit : ValidatedUnit} (representable : OwnersRepresentable unit)
    (agree : TypesAgree unit) :
    (values : List RuntimeValue) → (row : NRow) → {types : List SemTy} →
      NRow.admits unit row values = true → row.TypedAs unit types → row.paramFree = true →
      row.refFree = true →
      ∃ represented : List RuntimeValue, represented.map (SemanticOperations.normalize unit) = values ∧
        HasTypes unit (fun _ => none) represented types
  | [], .nil, _, _, .nil, _, _ => ⟨[], rfl, .nil⟩
  | value :: values, .cons τ rest, _, admitted, .cons head tail, closed, plain => by
      simp only [NRow.admits, Bool.and_eq_true] at admitted
      simp only [NRow.paramFree, Bool.and_eq_true] at closed
      simp only [NRow.refFree, Bool.and_eq_true] at plain
      obtain ⟨representation, normalized, typed⟩ :=
        representable τ _ value head closed.1 plain.1 (.inr agree) admitted.1
      obtain ⟨represented, normalizedRest, typedRest⟩ :=
        NRow.represented_of_admits representable agree values rest admitted.2 tail closed.2 plain.2
      exact ⟨representation :: represented, by simp [normalized, normalizedRest],
        .cons typed typedRest⟩
  | [], .cons _ _, _, admitted, _, _, _ | _ :: _, .nil, _, admitted, _, _, _ => by
      simp [NRow.admits] at admitted

end

end VerexIR.Proofs
