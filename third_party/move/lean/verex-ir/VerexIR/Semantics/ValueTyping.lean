-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Validation.StaticTyping
import VerexIR.Semantics.Typing
import VerexIR.Semantics.Focus

/-!
# Typing values at semantic types

A runtime value inhabits a semantic type (`SemanticTypes.lean`). Loans are
typed by an environment: a live mutable loan's hole and its borrow are typed
at the loan's referent type, so writing a borrow's current value back into
its hole keeps every position typed.
-/

namespace VerexIR


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

/-- The semantic types of the fields a nominal value holds beyond its declared
ones: a table owner's entries, pairs of its key and value type arguments
(`doc/designs/intrinsic-maps.md`, "Tables in the runtime semantics"). -/
def contentsTypes (unit : Validation.ValidatedUnit) (source : StructHandle)
    (variant : Option String) (arguments : List SemArg) : List SemTy :=
  match variant, Validation.tableLayout? unit source.namespaceId source.structId, arguments with
  | none, some _, [.type key, .type value] => [.vector (.tuple [key, value]) none]
  | _, _, _ => []

/-- A table owner's contents read at substituted type arguments are its
contents' types substituted. -/
theorem contentsTypes_substList (unit : Validation.ValidatedUnit) (source : StructHandle)
    (variant : Option String) (substitution : Array SemArg) (arguments : List SemArg) :
    contentsTypes unit source variant (SemArg.substList substitution arguments) =
      (contentsTypes unit source variant arguments).map (·.subst substitution) := by
  unfold contentsTypes
  cases variant with
  | some _ => rfl
  | none =>
      cases Validation.tableLayout? unit source.namespaceId source.structId with
      | none => rfl
      | some _ =>
          rcases arguments with _ | ⟨first, _ | ⟨second, _ | ⟨third, rest⟩⟩⟩
          · rfl
          · cases first <;> rfl
          · cases first <;> cases second <;>
              simp [SemArg.substList, SemArg.subst, SemTy.subst, SemTy.substList]
          · cases first <;> cases second <;> rfl

/-- The declared types of a nominal value's ghost fields, whose slots follow
its declared fields (`doc/designs/ghost-fields.md`). -/
def ghostTypeIds (unit : Validation.ValidatedUnit) (source : StructHandle) : List TypeId :=
  match unit.namespaces[source.namespaceId.index]? with
  | some ns => (SemanticOperations.nominalGhosts unit source).toList.map (ns.typeId ·.type.typeId)
  | none => []

theorem ghostTypeIds_of {unit : Validation.ValidatedUnit} {source : StructHandle}
    {ns : Validation.ValidatedNamespace}
    (namespace_eq : unit.namespaces[source.namespaceId.index]? = some ns) :
    ghostTypeIds unit source =
      (SemanticOperations.nominalGhosts unit source).toList.map (ns.typeId ·.type.typeId) := by
  simp [ghostTypeIds, namespace_eq]

@[simp] theorem ghostTypeIds_length (unit : Validation.ValidatedUnit) (source : StructHandle) :
    (ghostTypeIds unit source).length = (SemanticOperations.nominalGhosts unit source).size := by
  unfold ghostTypeIds
  split
  · simp
  · next none_eq => simp [SemanticOperations.nominalGhosts, none_eq]

/-- Read/write-back views of a storage type. Fixed storage may be observed
as a slice or written back to its growable-vector lender. This relation does
not authorize exposing a mutable growable-vector reference. -/
inductive StorageView : SemTy → SemTy → Prop where
  | same (type : SemTy) : StorageView type type
  | slice (element : SemTy) (extent : Nat) :
      StorageView (.vector element (some (.integer extent))) (.slice element)
  | vector (element : SemTy) (extent : Nat) :
      StorageView (.vector element (some (.integer extent))) (.vector element none)

theorem StorageView.trans {first second third : SemTy}
    (left : StorageView first second) (right : StorageView second third) :
    StorageView first third := by
  cases left with
  | same => exact right
  | slice => cases right; exact .slice _ _
  | vector => cases right; exact .vector _ _

theorem StorageView.inhabited {storage view : SemTy} (relation : StorageView storage view)
    (inhabited : storage ≠ .never) : view ≠ .never := by
  cases relation with
  | same => exact inhabited
  | slice | vector => simp

/-- A mutable borrow either retains a sized referent, or exposes fixed-array
storage as a slice. In particular a slice is never an ordinary unconstrained
loan, and fixed storage cannot be exposed as a growable-vector reference. -/
inductive BorrowView : SemTy → SemTy → Prop where
  | same (type : SemTy) (sized : type.sized = true) : BorrowView type type
  | slice (element : SemTy) (extent : Nat) :
      BorrowView (.vector element (some (.integer extent))) (.slice element)

theorem BorrowView.storageView {storage view : SemTy} (relation : BorrowView storage view) :
    StorageView storage view := by
  cases relation with
  | same => exact .same _
  | slice => exact .slice _ _

theorem BorrowView.sized {storage view : SemTy} (relation : BorrowView storage view)
    (sized : view.sized = true) : storage = view := by
  cases relation with
  | same => rfl
  | slice => cases sized

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
    /-- The observed contents of an unsized slice. A live mutable slice loan
    additionally fixes its extent in the loan's storage type. -/
    | slice (values : Array RuntimeValue) (element : SemTy)
        (elements : HasTypeEach unit loans values.toList element) :
        HasType unit loans (.vector values) (.slice element)
    /-- A nominal value inhabits its declaration's type at generic arguments
    under which each field inhabits its declared type, followed by its ghost
    slots at their declared types and a table owner's entries at their key and
    value types. -/
    | nominal (source : StructHandle) (variant : Option String) (fields : Array RuntimeValue)
        (name : QualifiedName) (arguments : List SemArg)
        (declaringNamespace : Validation.ValidatedNamespace) (declared : Array FieldDecl)
        (fieldTypes ghostTypes : List SemTy)
        (name_eq : SemanticOperations.structName? unit source = some name)
        (declaration_eq : SemanticOperations.handleFields? unit source variant =
          some (declaringNamespace, declared))
        (fieldTypes_resolve : ResolvesAll declaringNamespace.tables arguments.toArray
          (declared.toList.map (declaringNamespace.typeId ·.type.typeId)) fieldTypes)
        (ghostTypes_resolve : ResolvesAll declaringNamespace.tables arguments.toArray
          (ghostTypeIds unit source) ghostTypes)
        (fields_typed : HasTypes unit loans fields.toList
          (fieldTypes ++ (ghostTypes.map .ghost ++ contentsTypes unit source variant arguments))) :
        HasType unit loans (.nominal source variant fields) (.nominal name arguments)
    /-- A ghost field's slot, whose contents the runtime ignores: the
    verification model alone gives them meaning (`doc/designs/ghost-fields.md`);
    they hold no loan. -/
    | ghost (contents : RuntimeValue) (slot : SemTy) (plain : SemanticOperations.Plain contents) :
        HasType unit loans (.ghost contents) (.ghost slot)
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
        (sized_arguments : SizedArguments arguments)
        (mask_bound : mask < 2 ^ declaration.signature.parameters.size)
        (captures_typed : HasTypes unit loans captures.toList
          (ClosureMask.extract mask true allParameters))
        (parameters_eq : parameters = ClosureMask.extract mask false allParameters)
        (packs : Validation.StaticTyping.packs results result = true) :
        HasType unit loans (.closure function mask instantiation captures)
          (.function parameters result)
    /-- A live mutable loan's borrow holds a value of the loan's referent type. -/
    | borrow (loan : Nat) (current : RuntimeValue) (referent : SemTy) {storage : SemTy}
        (loan_eq : loans loan = some storage)
        (current_typed : HasType unit loans current storage)
        (view : BorrowView storage referent) :
        HasType unit loans (.borrow loan current) (.reference .mutable referent)
    /-- A shared reference is the value it observes. -/
    | shared (value : RuntimeValue) (referent : SemTy)
        (value_typed : HasType unit loans value referent) :
        HasType unit loans value (.reference .shared referent)
    /-- A shared view of a mutable reference holds what it refers to
    (`RuntimeValue.observe`), no borrow or hole itself: only a shared
    dereference reads it, and a mutable one is stuck on it. -/
    | observed (value : RuntimeValue) (referent : SemTy) (settled : value.live = false)
        (value_typed : HasType unit loans value referent) :
        HasType unit loans value (.reference .mutable referent)
    /-- What a settled borrow leaves, at the reference's type only. -/
    | dead (referent : SemTy) : HasType unit loans .retired (.reference .mutable referent)
    /-- A hole stands where a live mutable loan took its value from, at the
    loan's referent type, observed through shared references or not. -/
    | hole (loan : Nat) (referent type : SemTy) (loan_eq : loans loan = some referent)
        (same : StorageView referent.unshared type.unshared) (inhabited : referent.unshared ≠ .never) :
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
  | .slice values element elements =>
      .slice values element (HasTypeEach.weaken extends_ elements)
  | .nominal source variant fields name arguments declaringNamespace declared fieldTypes
      ghostTypes name_eq declaration_eq fieldTypes_resolve ghostTypes_resolve fields_typed =>
      .nominal source variant fields name arguments declaringNamespace declared fieldTypes
        ghostTypes name_eq declaration_eq fieldTypes_resolve ghostTypes_resolve
        (HasTypes.weaken extends_ fields_typed)
  | .closure function mask instantiation captures parameters result targetNs declaration
      arguments allParameters results namespace_eq declaration_eq signature_eq faithful sized_arguments
      mask_bound captures_typed parameters_eq packs =>
      .closure function mask instantiation captures parameters result targetNs declaration
        arguments allParameters results namespace_eq declaration_eq signature_eq faithful sized_arguments
        mask_bound (HasTypes.weaken extends_ captures_typed) parameters_eq packs
  | .borrow loan current referent loan_eq current_typed view =>
      .borrow loan current referent (extends_ loan _ loan_eq)
        (HasType.weaken extends_ current_typed) view
  | .shared value referent value_typed => .shared value referent (HasType.weaken extends_ value_typed)
  | .observed value referent settled value_typed =>
      .observed value referent settled (HasType.weaken extends_ value_typed)
  | .dead referent => .dead referent
  | .hole loan referent type loan_eq same inhabited =>
      .hole loan referent type (extends_ loan referent loan_eq) same inhabited
  | .unitEmptyTuple => .unitEmptyTuple
  | .emptyTupleUnit => .emptyTupleUnit
  | .ghost contents slot plain => .ghost contents slot plain

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
  | _, .hole _ _ _ _ same inhabited => same.inhabited inhabited
  | _, .unit | _, .bool _ | _, .character _ _ | _, .string _ | _, .bytes _
  | _, .integer _ _ _ _ | _, .address _ | _, .signer _ | _, .tuple _ _ _
  | _, .vector _ _ _ _ _ | _, .slice .. | _, .nominal .. | _, .closure .. | _, .borrow .. | _, .dead _
  | _, .observed .. | _, .unitEmptyTuple | _, .emptyTupleUnit | _, .ghost _ _ _ => by
      simp [SemTy.unshared]

/-- A value retains its type when observed through a permitted storage view. -/
theorem HasType.storageView {unit : Validation.ValidatedUnit} {loans : LoanTypes}
    {value : RuntimeValue} {storage view : SemTy}
    (typed : HasType unit loans value storage) (relation : StorageView storage view) :
    HasType unit loans value view := by
  cases relation with
  | same => exact typed
  | slice element extent =>
      cases typed with
      | vector _ _ _ _ elements => exact .slice _ _ elements
      | hole loan referent type loan_eq previous inhabited =>
          exact .hole loan referent _ loan_eq (previous.trans (.slice element extent)) inhabited
  | vector element extent =>
      cases typed with
      | vector _ _ _ _ elements => exact .vector _ _ _ trivial elements
      | hole loan referent type loan_eq previous inhabited =>
          exact .hole loan referent _ loan_eq (previous.trans (.vector element extent)) inhabited

private theorem observeList_of_plain : (values : List RuntimeValue) →
    (∀ value ∈ values, value.observe = value) → RuntimeValue.observeList values = values
  | [], _ => by rw [RuntimeValue.observeList]
  | head :: tail, plain => by
      rw [RuntimeValue.observeList, plain head (by simp),
        observeList_of_plain tail fun value member => plain value (by simp [member])]

/-- A plain value holds no borrow or hole: observing it is the identity. -/
theorem SemanticOperations.Plain.observe_eq {value : RuntimeValue} :
    SemanticOperations.Plain value → value.observe = value
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _ | .bytes _ => by
      rw [RuntimeValue.observe]
  | .vector elements plain => by
      rw [RuntimeValue.observe, observeList_of_plain _ fun value member =>
        Plain.observe_eq (plain value (by simpa using member))]
  | .tuple elements plain => by
      rw [RuntimeValue.observe, observeList_of_plain _ fun value member =>
        Plain.observe_eq (plain value (by simpa using member))]
  | .nominal _ _ fields plain => by
      rw [RuntimeValue.observe, observeList_of_plain _ fun value member =>
        Plain.observe_eq (plain value (by simpa using member))]
  | .closure _ _ _ captures plain => by
      rw [RuntimeValue.observe, observeList_of_plain _ fun value member =>
        Plain.observe_eq (plain value (by simpa using member))]
  | .ghost _ plain => by
      rw [RuntimeValue.observe, plain.observe_eq]
  | .retired => by rw [RuntimeValue.observe]

/-- An observation at a mutable reference type: what it refers to, or what a retired
borrow left. -/
theorem HasType.observedAt {unit : Validation.ValidatedUnit} {loans : LoanTypes}
    {value : RuntimeValue} {referent : SemTy} (typed : HasType unit loans value.observe referent) :
    HasType unit loans value.observe (.reference .mutable referent) := by
  rcases value.observe_live with settled | retired
  · exact .observed _ referent settled typed
  · rw [retired]
    exact .dead referent

mutual
/-- What a shared view holds of a typed value without a hole is typed the same. -/
theorem HasType.observe {unit : Validation.ValidatedUnit} {loans : LoanTypes}
    {value : RuntimeValue} {type : SemTy} (typed : HasType unit loans value type)
    (holeless : value.holeless = true) : HasType unit loans value.observe type := by
  match typed with
  | .unit => rw [RuntimeValue.observe]; exact .unit
  | .bool value => rw [RuntimeValue.observe]; exact .bool value
  | .character value valid => rw [RuntimeValue.observe]; exact .character value valid
  | .string value => rw [RuntimeValue.observe]; exact .string value
  | .bytes value => rw [RuntimeValue.observe]; exact .bytes value
  | .integer value width signed fits =>
      rw [RuntimeValue.observe]; exact .integer value width signed fits
  | .address value => rw [RuntimeValue.observe]; exact .address value
  | .signer value => rw [RuntimeValue.observe]; exact .signer value
  | .tuple values types elements =>
      rw [RuntimeValue.holeless] at holeless
      rw [RuntimeValue.observe]
      exact .tuple _ types (by simpa using HasTypes.observe elements holeless)
  | .slice values element elements =>
      rw [RuntimeValue.holeless] at holeless
      rw [RuntimeValue.observe]
      exact .slice _ element (by simpa using HasTypeEach.observe elements holeless)
  | .vector values element length length_matches elements =>
      rw [RuntimeValue.holeless] at holeless
      rw [RuntimeValue.observe]
      refine .vector _ element length ?_ (by simpa using HasTypeEach.observe elements holeless)
      simpa [RuntimeValue.observeList_length] using length_matches
  | .nominal source variant fields name arguments declaringNamespace declared fieldTypes
      ghostTypes name_eq declaration_eq fieldTypes_resolve ghostTypes_resolve fields_typed =>
      rw [RuntimeValue.holeless] at holeless
      rw [RuntimeValue.observe]
      exact .nominal source variant _ name arguments declaringNamespace declared fieldTypes
        ghostTypes name_eq declaration_eq fieldTypes_resolve ghostTypes_resolve
        (by simpa using HasTypes.observe fields_typed holeless)
  | .closure function mask instantiation captures parameters result targetNs declaration
      arguments allParameters results namespace_eq declaration_eq signature_eq faithful sized_arguments
      mask_bound captures_typed parameters_eq packs =>
      rw [RuntimeValue.holeless] at holeless
      rw [RuntimeValue.observe]
      exact .closure function mask instantiation _ parameters result targetNs declaration
        arguments allParameters results namespace_eq declaration_eq signature_eq faithful sized_arguments
        mask_bound (by simpa using HasTypes.observe captures_typed holeless) parameters_eq packs
  | .borrow _ current referent _ current_typed view =>
      rw [RuntimeValue.holeless] at holeless
      rw [RuntimeValue.observe]
      exact ((HasType.observe current_typed holeless).storageView view.storageView).observedAt
  | .shared _ referent value_typed => exact .shared _ referent (HasType.observe value_typed holeless)
  | .observed value referent _ value_typed =>
      exact (HasType.observe value_typed holeless).observedAt
  | .dead referent =>
      rw [RuntimeValue.observe]
      exact .dead referent
  | .hole .. => simp [RuntimeValue.holeless] at holeless
  | .unitEmptyTuple => rw [RuntimeValue.observe]; exact .unitEmptyTuple
  | .emptyTupleUnit =>
      rw [RuntimeValue.observe]
      simpa [RuntimeValue.observeList] using (HasType.emptyTupleUnit : HasType unit loans _ _)
  | .ghost contents slot plain =>
      rw [RuntimeValue.observe]
      exact .ghost _ slot (by rw [plain.observe_eq]; exact plain)

theorem HasTypes.observe {unit : Validation.ValidatedUnit} {loans : LoanTypes}
    {values : List RuntimeValue} {types : List SemTy} (typed : HasTypes unit loans values types)
    (holeless : RuntimeValue.holelessList values = true) :
    HasTypes unit loans (RuntimeValue.observeList values) types := by
  match typed with
  | .nil =>
      rw [RuntimeValue.observeList]
      exact .nil
  | .cons head tail =>
      rw [RuntimeValue.holelessList, Bool.and_eq_true] at holeless
      rw [RuntimeValue.observeList]
      exact .cons (HasType.observe head holeless.1) (HasTypes.observe tail holeless.2)

theorem HasTypeEach.observe {unit : Validation.ValidatedUnit} {loans : LoanTypes}
    {values : List RuntimeValue} {type : SemTy} (typed : HasTypeEach unit loans values type)
    (holeless : RuntimeValue.holelessList values = true) :
    HasTypeEach unit loans (RuntimeValue.observeList values) type := by
  match typed with
  | .nil =>
      rw [RuntimeValue.observeList]
      exact .nil
  | .cons head tail =>
      rw [RuntimeValue.holelessList, Bool.and_eq_true] at holeless
      rw [RuntimeValue.observeList]
      exact .cons (HasType.observe head holeless.1) (HasTypeEach.observe tail holeless.2)
end

/-- A shared borrow observes a value, optionally exposing a sequence as a slice. -/
theorem HasType.sharedBorrow {unit : Validation.ValidatedUnit} {loans : LoanTypes}
    {value : RuntimeValue} {source result : SemTy}
    (typed : HasType unit loans value source)
    (checked : Validation.StaticTyping.Context.sharedBorrowResult source result = true)
    (holeless : value.holeless = true) : HasType unit loans value.observe result := by
  simp only [Validation.StaticTyping.Context.sharedBorrowResult, Bool.or_eq_true,
    beq_iff_eq] at checked
  rcases checked with rfl | viewed
  · exact .shared _ _ (typed.observe holeless)
  · split at viewed
    · rename_i element length viewed
      simp only [beq_iff_eq] at viewed
      subst viewed
      apply HasType.shared
      cases typed with
      | vector values element length lengthMatches elements =>
          rw [RuntimeValue.holeless] at holeless
          rw [RuntimeValue.observe]
          exact .slice _ _ (by simpa using HasTypeEach.observe elements holeless)
      | hole => simp [RuntimeValue.holeless] at holeless
    · cases viewed

end VerexIR
