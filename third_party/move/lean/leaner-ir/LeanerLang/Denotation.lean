-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Quote
import LeanerIR.Proofs.Denotation
import LeanerIR.Proofs.Recursion
import LeanerIR.Proofs.Tree

/-!
# Generation of native verification denotations

This is an intentionally small certifying generator for V1.  It accepts the
straight-line subset supported by `LeanerIR.Proofs.Denotation`, emits a Lean
combinator tree, and emits an agreement theorem assembled exclusively from
the reusable core lemmas.  Unsupported constructs return a reason so the
verification command can retain its deep-semantics fallback.
-/

namespace LeanerLang.Denotation

open Lean Meta Elab Tactic Command
open LeanerIR
open LeanerIR.Validation
open LeanerIR.SemanticOperations
open LeanerLang.Quote

initialize registerTraceClass `leaner.agreement

elab "leaner_agreement_tick " label:str : tactic => do
  trace[leaner.agreement] "{label.getString}: {← IO.getNumHeartbeats}"

deriving instance ToExpr for LeanerIR.StructHandle
deriving instance ToExpr for LeanerIR.Proofs.Denotation.ResourceLocation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.BorrowLocation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.GlobalLocationOperation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.LocalLocation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.LocalLocationOperation
deriving instance ToExpr for LeanerIR.SemanticOperations.NominalFieldStep
deriving instance ToExpr for LeanerIR.Proofs.Denotation.DerefLocalBorrowOperation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.IndexedLocalBorrowOperation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.IndexedLocalFieldBorrowOperation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.NominalConstructor
deriving instance ToExpr for LeanerIR.Proofs.Denotation.NominalFieldLocation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.NominalVariantFieldLocation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.NominalVariantTest
deriving instance ToExpr for LeanerIR.Proofs.Denotation.PrimitiveLocationOperation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.ReferenceLocationOperation
deriving instance ToExpr for LeanerIR.SemanticOperations.NativePattern
deriving instance ToExpr for LeanerIR.SemanticOperations.NativePatternBinder
deriving instance ToExpr for LeanerIR.SemanticOperations.FunctionShape

/-- Names materialized for one supported function. -/
structure Generated where
  unitDef : Name
  namespaceDef : Name
  declaration : Name
  shape : Name
  body : Name
  relation : Name
  relationAgreement : Name
  denotation : Name
  agreement : Name
  typeParameterCount : Nat
  requiresUnitEquality : Bool
  /-- For a recursive function, the body abstracted over the body of its
  recursive calls (`fun executable self => …`); `body` is its fixed point. -/
  openBody : Option Name := none
  /-- The body as a literal tree, and the reflexivity that it denotes the
  body (the open body, for a recursive function). -/
  tree : Name
  treeDenotes : Name

/-- Whether native generation succeeded.  Unsupported is deliberately data,
not an elaboration error: coexistence requires the caller to select the deep
route without weakening or changing the contract. -/
inductive Result where
  | generated (value : Generated)
  | unsupported (reason : String)

private def pathName (segments : Array String) : Name :=
  segments.foldl (fun name segment => Name.str name segment) .anonymous

def namespaceName (segments : Array String) : Name :=
  Name.str (pathName segments) "denotationNamespace"

def unitName (segments : Array String) : Name :=
  Name.str (pathName segments) "denotationUnit"

def declarationName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "denotationDeclaration"

def shapeName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "denotationShape"

def bodyName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "denotationBody"

def openBodyName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "denotationOpenBody"

def treeName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "denotationTree"

def treeDenotesName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "denotationTree_denotes"

def openBodyAgreementName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "denotationOpenBody_agrees"

def relationName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "denotationRelation"

def relationAgreementName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "denotationRelation_agrees"

def functionName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "denotation"

def agreementName (segments : Array String) (function : String) : Name :=
  Name.str (Name.str (pathName segments) function) "denotation_agrees"

/-! ## Supported-subset check -/

/-- Whether one type contains a particular function type parameter.  Type
nodes are interned as a graph, so the visited row protects this diagnostic
scan independently of validation's acyclicity checks. -/
private partial def typeMentionsParameter (unit : ValidatedUnit)
    (parameter : Nat) (seen : Array Nat) (id : TypeId) : Bool :=
  if seen.contains id.index then false else
  match unit.tables.types[id.index]? with
  | none => false
  | some (.typeParameter index) => index == parameter
  | some (.tuple elements) =>
      elements.any (typeMentionsParameter unit parameter (seen.push id.index))
  | some (.vector element _) | some (.typeDomain element) =>
      typeMentionsParameter unit parameter (seen.push id.index) element
  | some (.resourceDomain _ (some arguments)) =>
      arguments.any (typeMentionsParameter unit parameter (seen.push id.index))
  | some (.nominal _ arguments) =>
      arguments.any (genericArgumentMentionsParameter unit parameter
        (seen.push id.index))
  | some (.function arguments result _) =>
      arguments.any (typeMentionsParameter unit parameter (seen.push id.index)) ||
        typeMentionsParameter unit parameter (seen.push id.index) result
  | some (.reference reference) =>
      typeMentionsParameter unit parameter (seen.push id.index) reference.referent
  | some _ => false
where
  genericArgumentMentionsParameter (unit : ValidatedUnit) (parameter : Nat)
      (seen : Array Nat) : GenericArgument → Bool
    | .typeArg value => typeMentionsParameter unit parameter seen value.typeId
    | .const _ | .lifetime _ | .evidence _ => false

private def genericArgumentMentionsParameter (unit : ValidatedUnit)
    (parameter : Nat) (argument : GenericArgument) : Bool :=
  match argument with
  | .typeArg value => typeMentionsParameter unit parameter #[] value.typeId
  | .const _ | .lifetime _ | .evidence _ => false

/-- A resource whose head is the parameter itself has no statically known
family to index.  A known nominal head such as `Vault<T>` does: its inner
argument remains parametric while the generated family accessor supplies the
corresponding store member. -/
private def genericArgumentIsParameter (unit : ValidatedUnit)
    (parameter : Nat) : GenericArgument → Bool
  | .typeArg value =>
      match unit.tables.types[value.typeId.index]? with
      | some (.typeParameter index) => index == parameter
      | _ => false
  | .const _ | .lifetime _ | .evidence _ => false

/-- Determine whether a function parameter reaches the type component of a
global-storage key.  Ordinary appearances in values, locals, aggregates,
operations, and direct generic calls remain parametric: Move's executable
semantics does not inspect their concrete instantiation.  A direct call is
followed so storage-key dependence propagates through the call graph.

The active `(function, parameter)` row breaks diagnostic recursion.  The
denotation generator separately rejects recursive call SCCs until it has a
mutual fixed-point rule. -/
private partial def parameterStorageKeyUse? (unit : ValidatedUnit)
    (handle : FunctionHandle) (parameter : Nat)
    (active : Array (FunctionHandle × Nat)) : Option String := do
  if active.contains (handle, parameter) then none else
  let ns ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← ns.functions[handle.functionId.index]?
  let .structured root := declaration.body | none
  expressionStorageKeyUse? unit handle ns parameter
    (active.push (handle, parameter)) #[] root
where
  expressionStorageKeyUse? (unit : ValidatedUnit) (handle : FunctionHandle)
      (ns : ValidatedNamespace) (parameter : Nat)
      (active : Array (FunctionHandle × Nat)) (seen : Array Nat)
      (id : ExprId) : Option String := do
    if seen.contains id.index then none else
    let expression ← ns.expressions[id.index]?
    let descend child := expressionStorageKeyUse? unit handle ns parameter
      active (seen.push id.index) child
    match expression.kind with
    | .operation (.global _) instantiations _ _ =>
        if instantiations.any
            (genericArgumentIsParameter unit parameter) then
          some s!"global operation {id.index} uses it in the resource key"
        else
          (Validation.expressionChildren expression.kind).findSome? descend
    | .operation (.call (.function reference)) instantiations _ _ =>
        let throughCall := do
          let callee ← SemanticOperations.resolveFunction? unit
            handle.namespaceId reference
          instantiations.zipIdx.findSome? fun (argument, calleeParameter) =>
            if genericArgumentMentionsParameter unit parameter argument then
              (parameterStorageKeyUse? unit callee calleeParameter active).map
                fun reason => s!"call {id.index} passes it to {reason}"
            else none
        throughCall.orElse fun () =>
          (Validation.expressionChildren expression.kind).findSome? descend
    | kind => (Validation.expressionChildren kind).findSome? descend

private def checkGenericParameters (unit : ValidatedUnit)
    (handle : FunctionHandle)
    (declaration : FunctionDecl FunctionBody) : Except String Nat := do
  unless declaration.signature.generics.all (fun binder => binder.kind == .typeArg) do
    throw "V1 denotations support type parameters only; const, lifetime, and evidence binders require specialization"
  for (binder, index) in declaration.signature.generics.zipIdx do
    if let some use := parameterStorageKeyUse? unit handle index #[] then
      throw s!"generic type parameter `{binder.name}` determines a global resource key: {use}"
  return declaration.signature.generics.size

private inductive LoweredOperation where
  | primitive (operation : LeanerIR.Proofs.Denotation.PrimitiveLocationOperation)
  | global (operation : LeanerIR.Proofs.Denotation.GlobalLocationOperation)
  | local (operation : LeanerIR.Proofs.Denotation.LocalLocationOperation)
  | derefLocalBorrow
      (operation : LeanerIR.Proofs.Denotation.DerefLocalBorrowOperation)
  | indexedLocalBorrow
      (operation : LeanerIR.Proofs.Denotation.IndexedLocalBorrowOperation)
  | indexedLocalFieldBorrow
      (operation : LeanerIR.Proofs.Denotation.IndexedLocalFieldBorrowOperation)
  | field (location : LeanerIR.Proofs.Denotation.NominalFieldLocation)
  | variantField
      (location : LeanerIR.Proofs.Denotation.NominalVariantFieldLocation)
  | variantTest (test : LeanerIR.Proofs.Denotation.NominalVariantTest)
  | reference (operation : LeanerIR.Proofs.Denotation.ReferenceLocationOperation)
  | constructor (constructor : LeanerIR.Proofs.Denotation.NominalConstructor)
  | function (reference : QualifiedRef)

private def lowerLocal (ns : ValidatedNamespace) (operation : Operation)
    (place : PlaceId) : Except String LoweredOperation := do
  let some (.localVar localId) := ns.places[place.index]?
    | throw s!"place {place.index} is not a direct parameter/local slot"
  let location : LeanerIR.Proofs.Denotation.LocalLocation := { localId }
  match operation with
  | .read _ => return .local (.read location)
  | .copy _ => return .local (.copy location)
  | .move _ => return .local (.move location)
  | _ => throw "internal native-local lowering mismatch"

private def lowerLocalBorrow (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (resultType : TypeId) (site : ExprId) (kind : BorrowKind)
    (localId : LocalId) : Except String LoweredOperation := do
  let some (.reference referenceType) := ns.tables.types[resultType.index]?
    | throw "a native local borrow requires a resolved reference result type"
  let lexicalLoan ← match kind with
    | .immutable => pure 0
    | .mutable =>
        let some lexicalLoan := certificateLoanId? unit ns.identity site
          | throw s!"local borrow site {site.index} has no checked loan identity"
        pure lexicalLoan
    | .profile _ =>
        throw "profile-defined local borrow has no native V1 descriptor"
  return .local (.borrow { localId } referenceType kind lexicalLoan)

/-- Resolve a `deref (localVar ...) / field*` source place to the closed
native descriptor consumed by a local reborrow. Validation has already
established that the place arena is acyclic. -/
private partial def lowerDerefLocalPlace (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (place : PlaceId) :
    Except String (LocalId × List LeanerIR.Proofs.Denotation.NominalFieldStep) := do
  match ns.places[place.index]? with
  | some (.deref base) =>
      let some (.localVar localId) := ns.places[base.index]?
        | throw s!"borrow base {base.index} is not a direct parameter/local slot"
      return (localId, [])
  | some (.field base owner fieldNameId) =>
      let (localId, fields) ← lowerDerefLocalPlace unit ns base
      let some source := resolveStruct? unit ns.identity owner
        | throw "native projected borrow could not resolve its nominal owner"
      let some fieldName := sourceFieldName? ns fieldNameId
        | throw s!"native projected borrow could not resolve field name {fieldNameId.index}"
      let (variant, index) ← match handleFieldIndex? unit source none fieldName with
        | some index => pure (none, index)
        | none =>
          match variantFieldChoices? unit source #[fieldName] with
          | some #[(variant, index)] => pure (some variant, index)
          | _ => throw s!"native projected borrow requires a uniquely located field `{fieldName}`"
      return (localId, fields ++ [{ source, variant, index }])
  | _ =>
      throw s!"borrow place {place.index} is not a dereferenced local reference"

/-- Resolve `&kind *localReference` to a runtime-local descriptor.  The
source place graph, result reference type, and lexical loan lookup are all
closed here rather than retained as proof-time computations. -/
private def lowerDerefLocalBorrow (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (resultType : TypeId) (site : ExprId)
    (kind : BorrowKind) (place : PlaceId) : Except String LoweredOperation := do
  let (localId, fields) ← lowerDerefLocalPlace unit ns place
  let some (.reference referenceType) := ns.tables.types[resultType.index]?
    | throw "a native local reborrow requires a resolved reference result type"
  let lexicalLoan ← match kind with
    | .immutable => pure 0
    | .mutable =>
        let some lexicalLoan := certificateLoanId? unit ns.identity site
          | throw s!"local reborrow site {site.index} has no checked loan identity"
        pure lexicalLoan
    | .profile _ =>
        throw "profile-defined local reborrow has no native V1 descriptor"
  return .derefLocalBorrow {
    location := { localId }, fields, referenceType, kind, lexicalLoan }

/-- Lower literal and local-read indexes without retaining source arena
nodes in the native operation. -/
private def lowerIndexedLocalBorrow (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (resultType : TypeId) (site : ExprId)
    (kind : BorrowKind) (place : PlaceId) : Except String LoweredOperation := do
  let some (.index basePlace indexExpr) := ns.places[place.index]?
    | throw s!"borrow place {place.index} is not an indexed local"
  let (index, indexLocal) ← match placeIndexForm? ns indexExpr with
    | some (.literal value) =>
        if value < 0 then throw s!"indexed borrow place {place.index} has a negative literal index"
        else pure (value.toNat, none)
    | some (.local localId) | some (.copyLocal localId) => pure (0, some localId)
    | _ => throw s!"indexed borrow place {place.index} requires a literal or local index"
  let (localId, dereference) ← match ns.places[basePlace.index]? with
    | some (.localVar localId) => pure (localId, false)
    | some (.deref localPlace) =>
        let some (.localVar localId) := ns.places[localPlace.index]?
          | throw s!"indexed borrow base {basePlace.index} is not rooted at a local"
        pure (localId, true)
    | _ => throw s!"indexed borrow base {basePlace.index} is not rooted at a local"
  let some (.reference referenceType) := ns.tables.types[resultType.index]?
    | throw "a native indexed borrow requires a resolved reference result type"
  let lexicalLoan ← match kind with
    | .immutable => pure 0
    | .mutable =>
        let some lexicalLoan := certificateLoanId? unit ns.identity site
          | throw s!"indexed borrow site {site.index} has no checked loan identity"
        pure lexicalLoan
    | .profile _ =>
        throw "profile-defined indexed borrow has no native V1 descriptor"
  return .indexedLocalBorrow {
    location := { localId }, dereference, index, indexLocal,
    referenceType, kind, lexicalLoan }

/-- Lower `ownedLocal[literal].field` to a descriptor which retains no
place-arena or declaration lookup. -/
private def lowerIndexedLocalFieldBorrow (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (resultType : TypeId) (site : ExprId)
    (kind : BorrowKind) (place : PlaceId) : Except String LoweredOperation := do
  let some (.field indexPlace owner fieldNameId) := ns.places[place.index]?
    | throw s!"borrow place {place.index} is not an indexed local field"
  let some (.index basePlace indexExpr) := ns.places[indexPlace.index]?
    | throw s!"indexed-field borrow base {indexPlace.index} is not an index"
  let some (.literal value) := placeIndexForm? ns indexExpr
    | throw s!"indexed-field borrow place {place.index} does not use a literal index"
  if value < 0 then
    throw s!"indexed-field borrow place {place.index} has a negative literal index"
  let some (.localVar localId) := ns.places[basePlace.index]?
    | throw s!"indexed-field borrow base {basePlace.index} is not an owned local"
  let some source := resolveStruct? unit ns.identity owner
    | throw "native indexed-field borrow could not resolve its nominal owner"
  let some fieldName := sourceFieldName? ns fieldNameId
    | throw s!"native indexed-field borrow could not resolve field name {fieldNameId.index}"
  let some fieldIndex := handleFieldIndex? unit source none fieldName
    | throw s!"native indexed-field borrow could not resolve field `{fieldName}`"
  let some (.reference referenceType) := ns.tables.types[resultType.index]?
    | throw "a native indexed-field borrow requires a resolved reference result type"
  let lexicalLoan ← match kind with
    | .immutable => pure 0
    | .mutable =>
        let some lexicalLoan := certificateLoanId? unit ns.identity site
          | throw s!"indexed-field borrow site {site.index} has no checked loan identity"
        pure lexicalLoan
    | .profile _ =>
        throw "profile-defined indexed-field borrow has no native V1 descriptor"
  return .indexedLocalFieldBorrow {
    location := { localId }, index := value.toNat,
    field := { source, variant := none, index := fieldIndex },
    referenceType, kind, lexicalLoan }

private def lowerPrimitive (ns : ValidatedNamespace) (resultType : TypeId)
    (operation : PrimitiveOperation) : Except String LoweredOperation := do
  let some resolved := ns.tables.types[resultType.index]?
    | throw s!"primitive result type {resultType.index} is out of range"
  if let .integer .pointer _ := resolved then
    throw "V1 native lowering requires a source-independent integer width"
  let lowered := match operation with
    | .tuple => some .tuple
    | .vector => some .vector
    | .pushVector => some .pushVector
    | .concatVector => some .concatVector
    | .slice => some .slice
    | .insertVector => some .insertVector
    | .removeVector => some .removeVector
    | .swapVector => some .swapVector
    | .reverseSliceVector => some .reverseSliceVector
    | .destroyEmptyVector => some .destroyEmptyVector
    | .containsVector => some .containsVector
    | .checkVectorIndex failure => some (.checkVectorIndex failure)
    | .indexOfVector => match resolved with
        | .tuple #[_, index] => match ns.tables.types[index.index]? with
            | some (.integer .pointer _) | none => none
            | some indexType => some (.indexOfVector indexType)
        | _ => none
    | .length => some (.length resolved)
    | .logicalNot => some .logicalNot
    | .index => some (.index resolved)
    | .copyValue => some (.copyValue resolved)
    | .moveValue => some (.moveValue resolved)
    | .add => some (.add resolved)
    | .checkedAdd failure => some (.checkedAdd failure resolved)
    | .subtract => some (.subtract resolved)
    | .checkedSubtract failure => some (.checkedSubtract failure resolved)
    | .multiply => some (.multiply resolved)
    | .checkedMultiply failure => some (.checkedMultiply failure resolved)
    | .less => some (.less resolved)
    | .greater => some (.greater resolved)
    | .lessEqual => some (.lessEqual resolved)
    | .greaterEqual => some (.greaterEqual resolved)
    | .equal => some (.equal resolved)
    | .notEqual => some (.notEqual resolved)
    | .divide => some (.divide resolved)
    | .checkedDivide failure => some (.checkedDivide failure resolved)
    | .modulo => some (.modulo resolved)
    | .checkedModulo failure => some (.checkedModulo failure resolved)
    | .bitwiseOr => some (.bitwiseOr resolved)
    | .bitwiseAnd => some (.bitwiseAnd resolved)
    | .bitwiseXor => some (.bitwiseXor resolved)
    | .bitwiseNot => some (.bitwiseNot resolved)
    | .shiftLeft => some (.shiftLeft resolved)
    | .checkedShiftLeft failure => some (.checkedShiftLeft failure resolved)
    | .shiftRight => some (.shiftRight resolved)
    | .checkedShiftRight failure => some (.checkedShiftRight failure resolved)
    | .cast => some (.cast resolved)
    | .checkedCast failure => some (.checkedCast failure resolved)
    | .logicalAnd => some .logicalAnd
    | .logicalOr => some .logicalOr
    | _ => none
  let some lowered := lowered
    | throw s!"primitive operation `{repr operation}` has no native V1 descriptor"
  return .primitive lowered

private def lowerGlobal (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (resultType : TypeId) (site : ExprId) (kind : GlobalKind)
    (instantiations : Array GenericArgument) : Except String LoweredOperation := do
  let #[.typeArg resourceType] := instantiations
    | throw "a native global operation requires exactly one resolved resource type"
  let resource : LeanerIR.Proofs.Denotation.ResourceLocation :=
    { namespaceId := ns.identity, typeId := resourceType.typeId }
  match kind with
  | .contains => return .global (.contains resource)
  | .take => return .global (.take resource)
  | .publish => return .global (.publish resource)
  | .borrow kind =>
      let some (.reference referenceType) := ns.tables.types[resultType.index]?
        | throw "a native global borrow requires a resolved reference result type"
      let lexicalLoan ← match kind with
        | .immutable => pure 0
        | .mutable =>
            let some lexicalLoan := certificateLoanId? unit ns.identity site
              | throw s!"global borrow site {site.index} has no checked loan identity"
            pure lexicalLoan
        | .profile _ =>
            throw "profile-defined global borrow has no native V1 descriptor"
      return .global (.borrow {
        resource, referenceType, kind, lexicalLoan })

private def lowerData (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (operation : DataOperation) : Except String LoweredOperation := do
  match operation with
  | .select reference fieldName =>
      let some handle := resolveStruct? unit ns.identity reference
        | throw "native field selection could not resolve its nominal owner"
      let some declarationNamespace := unit.namespaces[handle.namespaceId.index]?
        | throw "native field selection resolved an invalid namespace"
      let some declaration := declarationNamespace.structs[handle.structId]?
        | throw "native field selection resolved an invalid declaration"
      unless declaration.variants.isEmpty do
        throw "V1 native field selection requires a statically known enum variant"
      let some index := handleFieldIndex? unit handle none fieldName
        | throw s!"native field selection could not resolve field `{fieldName}`"
      return .field { source := handle, variant := none, index }
  | .selectVariants reference fields =>
      let some handle := resolveStruct? unit ns.identity reference
        | throw "native variant field selection could not resolve its nominal owner"
      let some choices := variantFieldChoices? unit handle fields
        | throw "native variant field selection could not resolve its payload map"
      return .variantField { source := handle, choices }
  | .testVariants reference variants =>
      let some handle := resolveStruct? unit ns.identity reference
        | throw "native variant test could not resolve its nominal owner"
      return .variantTest { source := handle, variants }
  | _ => throw s!"data operation `{repr operation}` has no native V1 descriptor"

private def lowerReference (ns : ValidatedNamespace) (resultType : TypeId)
    (operation : ReferenceOperation) : Except String LoweredOperation := do
  match operation with
  | .dereference => return .reference .dereference
  | .mutate => return .reference .mutate
  | .freeze _ =>
      let some (.reference referenceType) := ns.tables.types[resultType.index]?
        | throw "a native freeze requires a resolved reference result type"
      return .reference (.freeze referenceType)
  | .endLoan loans => return .reference (.endLoan loans)
  | .borrow _ => throw "value-level reference borrow has no native V1 descriptor"

private def lowerConstructor (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (reference : QualifiedRef) (variant : Option String) :
    Except String LoweredOperation := do
  let some handle := resolveStruct? unit namespaceId reference
    | throw "native construction could not resolve its nominal declaration"
  let some fields := constructorFields? unit handle variant
    | throw "native construction could not resolve its constructor payload"
  return .constructor { source := handle, variant, arity := fields.size }

private def lowerOperation (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace) (resultType : TypeId) (site : ExprId)
    (operation : Operation) (instantiations : Array GenericArgument) :
    Except String LoweredOperation :=
  match operation with
  | .primitive operation => lowerPrimitive ns resultType operation
  | .global kind => lowerGlobal unit ns resultType site kind instantiations
  | .data operation => lowerData unit ns operation
  | .reference operation => lowerReference ns resultType operation
  | .call (.constructor reference variant) =>
      lowerConstructor unit namespaceId reference variant
  | .call (.function reference) => .ok (.function reference)
  | .read place => lowerLocal ns operation place
  | .copy place => lowerLocal ns operation place
  | .move place => lowerLocal ns operation place
  | .borrow kind place =>
      match ns.places[place.index]? with
      | some (.localVar localId) =>
          lowerLocalBorrow unit ns resultType site kind localId
      | some (.index ..) =>
          lowerIndexedLocalBorrow unit ns resultType site kind place
      | some (.field base ..) =>
          match ns.places[base.index]? with
          | some (.index ..) =>
              lowerIndexedLocalFieldBorrow unit ns resultType site kind place
          | _ => lowerDerefLocalBorrow unit ns resultType site kind place
      | _ => lowerDerefLocalBorrow unit ns resultType site kind place
  | .call _ => .error s!"call form at expression {site.index} is not in V1"
  | .assert => .error s!"assert at expression {site.index} is not in V1"
  | .write _ | .drop _ =>
      .error s!"place operation at expression {site.index} is not in V1"
  | .profile .. => .error s!"profile operation at expression {site.index} is not in V1"
  | .specification _ =>
      .error s!"logical operation reached executable expression {site.index}"

/-- Fold only closed, successful primitive expressions. Each folded node
is certified against the existing semantics by `leaner_constant_agree`.
Fuel bounds reference chains, including malformed cycles. -/
private def constantValueFuel? (unit : ValidatedUnit) (fuel : Nat)
    (namespaceId : NamespaceId) (id : ExprId) : Option RuntimeValue := do
  let fuel + 1 := fuel | none
  let ns ← unit.namespaces[namespaceId.index]?
  let expression ← ns.expressions[id.index]?
  match expression.kind with
  | .value literal _ => constValue? literal
  | .constant reference =>
      let handle ← resolveConstant? unit namespaceId reference
      let targetNs ← unit.namespaces[handle.namespaceId.index]?
      let declaration ← targetNs.constants[handle.constantId]?
      constantValueFuel? unit fuel handle.namespaceId declaration.value
  | .operation (.primitive operation) _ arguments _ =>
      let values ← arguments.mapM (constantValueFuel? unit fuel namespaceId)
      let .ok value ← evaluatePrimitiveOperation? ns expression.typeId operation values | none
      some value
  | _ => none

private def foldedConstant? (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (reference : QualifiedRef) : Option RuntimeValue := do
  let handle ← resolveConstant? unit namespaceId reference
  let targetNs ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← targetNs.constants[handle.constantId]?
  let fuel := unit.namespaces.foldl (fun count ns => count + ns.expressions.size) 1
  constantValueFuel? unit fuel handle.namespaceId declaration.value

/-- Classify an assignment through a vector/tuple held directly in a local,
with an index that preparation reduced to a direct or copied local read. -/
private def localIndexAssignment? (ns : ValidatedNamespace) (place : PlaceId) :
    Option (LocalId × LocalId) := do
  let .index basePlace indexExpr ← ns.places[place.index]? | none
  let .localVar base ← ns.places[basePlace.index]? | none
  let index ← match placeIndexForm? ns indexExpr with
    | some (.local index) | some (.copyLocal index) => some index
    | _ => none
  some (base, index)

private partial def checkExpr (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace) (seen : Array Nat)
    (id : ExprId) : Except String Unit := do
  if seen.contains id.index then
    throw s!"expression {id.index} is cyclic outside the structured loop form"
  let some expression := ns.expressions[id.index]?
    | throw s!"expression {id.index} is out of range"
  let descend := checkExpr unit namespaceId ns (seen.push id.index)
  match expression.kind with
  | .value literal _ =>
      unless (constValue? literal).isSome do
        throw s!"literal at expression {id.index} has no runtime representation"
  | .localVar _ => pure ()
  | .operation operation instantiations arguments _ =>
      let lowered ← lowerOperation unit namespaceId ns expression.typeId id
        operation instantiations
      if let .function reference := lowered then
        unless (resolveFunction? unit namespaceId reference).isSome do
          throw s!"direct call at expression {id.index} does not resolve"
      arguments.forM descend
  | .block statements result =>
      statements.forM descend
      result.forM descend
  | .letDecl pattern initializer body =>
      if initializer.isSome then
        unless (lowerPattern? unit ns pattern).isSome do
          throw s!"pattern {pattern.index} has no native V1 binder; literal and range patterns require specialized lowering"
      initializer.forM descend
      descend body
  | .constant reference =>
      unless (foldedConstant? unit namespaceId reference).isSome do
        throw s!"constant expression {id.index} is not a closed, successful primitive initializer"
  | .ifElse condition thenBranch elseBranch =>
      descend condition
      descend thenBranch
      elseBranch.forM descend
  | .match_ .. => throw s!"match expression {id.index} is not in V1"
  | .loop _ body => descend body
  | .break_ _ none => pure ()
  | .break_ _ (some _) =>
      throw s!"value-carrying break at expression {id.index} is not yet native"
  | .continue_ _ => pure ()
  | .return_ values => values.forM descend
  | .throw_ _ arguments => arguments.forM descend
  | .assign place child =>
      match ns.places[place.index]? with
      | some (.localVar _) => pure ()
      | _ => unless (localIndexAssignment? ns place).isSome do
          throw s!"assignment at expression {id.index} is not to a closed local or local index"
      descend child
  | .assignPattern .. =>
      throw s!"pattern assignment at expression {id.index} is not in V1"
  | .spec _ => pure ()
  | .quantifier .. =>
      throw s!"logical expression {id.index} reached an executable body"

/-- Check the local V1 denotation subset and classify generic parameters
without materializing declarations.  Tooling and regression tests use this
entry point to distinguish a storage-parametric carrier from a parameter
which determines a global resource key and must await monomorphization.
Recursive dependency closure is still checked by `ensureDefinitions`. -/
def checkFunction (unit : ValidatedUnit) (handle : FunctionHandle) :
    Except String Nat := do
  let some ns := unit.namespaces[handle.namespaceId.index]?
    | throw s!"function namespace {handle.namespaceId.index} is out of range"
  let some declaration := ns.functions[handle.functionId.index]?
    | throw s!"function {handle.functionId.index} is out of range"
  let .structured root := declaration.body
    | throw "the function has no structured body"
  checkExpr unit handle.namespaceId ns #[] root
  checkGenericParameters unit handle declaration

/-! ## Native-term emission -/

private structure CalleeDenotation where
  reference : QualifiedRef
  handle : FunctionHandle
  /-- The callee's relation at the executable unit.  A recursive call's is
  the relation of the body under construction, not a named constant. -/
  relationTerm : Lean.Expr → MetaM Lean.Expr
  /-- The callee's generated names; none for the recursive call. -/
  generated : Option Generated

/-- The constructors one reading of a body is built from: the relational
combinators, or the tree's constructors.  One traversal emits either. -/
structure Builder where
  value : Lean.Expr → MetaM Lean.Expr
  localVar : Lean.Expr → MetaM Lean.Expr
  primitive : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  global : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  local_ : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  derefLocalBorrow : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  indexedLocalBorrow : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  indexedLocalFieldBorrow : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  reference : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  field : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  variantField : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  variantTest : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  constructor : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  call : Lean.Expr → Lean.Expr → Lean.Expr → Lean.Expr → MetaM Lean.Expr
  callAt : Lean.Expr → Lean.Expr → Lean.Expr → Lean.Expr → MetaM Lean.Expr
  branch : Lean.Expr → Lean.Expr → Option Lean.Expr → MetaM Lean.Expr
  return_ : Lean.Expr → MetaM Lean.Expr
  throw_ : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  loop : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  break_ : Lean.Expr → MetaM Lean.Expr
  continue_ : Lean.Expr → MetaM Lean.Expr
  assignLocal : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  assignLocalIndex : Lean.Expr → Lean.Expr → Lean.Expr → MetaM Lean.Expr
  spec : MetaM Lean.Expr
  blockUnit : Lean.Expr → MetaM Lean.Expr
  blockResult : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  letNoValue : Lean.Expr → MetaM Lean.Expr
  letValue : Lean.Expr → Lean.Expr → Lean.Expr → MetaM Lean.Expr
  valuesNil : MetaM Lean.Expr
  valuesCons : Lean.Expr → Lean.Expr → MetaM Lean.Expr
  statementsNil : MetaM Lean.Expr
  statementsCons : Lean.Expr → Lean.Expr → MetaM Lean.Expr

/-- The relational combinators. -/
def relationalBuilder : Builder where
  value v := mkAppM ``LeanerIR.Proofs.Denotation.value #[v]
  localVar i := mkAppM ``LeanerIR.Proofs.Denotation.localVar #[i]
  primitive op os := mkAppM ``LeanerIR.Proofs.Denotation.nativePrimitiveOperation #[op, os]
  global op os := mkAppM ``LeanerIR.Proofs.Denotation.nativeGlobalOperation #[op, os]
  local_ op os := mkAppM ``LeanerIR.Proofs.Denotation.nativeLocalOperation #[op, os]
  derefLocalBorrow op os :=
    mkAppM ``LeanerIR.Proofs.Denotation.nativeDerefLocalBorrowOperation #[op, os]
  indexedLocalBorrow op os :=
    mkAppM ``LeanerIR.Proofs.Denotation.nativeIndexedLocalBorrowOperation #[op, os]
  indexedLocalFieldBorrow op os :=
    mkAppM ``LeanerIR.Proofs.Denotation.nativeIndexedLocalFieldBorrowOperation #[op, os]
  reference op os := mkAppM ``LeanerIR.Proofs.Denotation.nativeReferenceOperation #[op, os]
  field location os := do
    let evaluate ← mkAppM
      ``LeanerIR.Proofs.Denotation.NominalFieldLocation.evaluateSelect? #[location]
    mkAppM ``LeanerIR.Proofs.Denotation.nativeOperation #[evaluate, os]
  variantField location os := do
    let evaluate ← mkAppM
      ``LeanerIR.Proofs.Denotation.NominalVariantFieldLocation.evaluateSelect?
      #[location]
    mkAppM ``LeanerIR.Proofs.Denotation.nativeOperation #[evaluate, os]
  variantTest test os := do
    let evaluate ← mkAppM
      ``LeanerIR.Proofs.Denotation.NominalVariantTest.evaluate? #[test]
    mkAppM ``LeanerIR.Proofs.Denotation.nativeOperation #[evaluate, os]
  constructor c os := do
    let evaluate ← mkAppM ``LeanerIR.Proofs.Denotation.NominalConstructor.evaluate? #[c]
    mkAppM ``LeanerIR.Proofs.Denotation.nativeOperation #[evaluate, os]
  call handle lexical relation os :=
    mkAppM ``LeanerIR.Proofs.Denotation.nativeCall #[handle, lexical, relation, os]
  callAt handle lexical relation os :=
    mkAppM ``LeanerIR.Proofs.Denotation.nativeCallAt #[handle, lexical, relation, os]
  branch c t e := do
    let e ← match e with
      | none =>
          mkAppOptM ``Option.none
            #[some (mkConst ``LeanerIR.Proofs.Denotation.ExprDenotation)]
      | some e =>
          mkAppM ``Option.some #[e]
    mkAppM ``LeanerIR.Proofs.Denotation.nativeBranch #[c, t, e]
  return_ vs := mkAppM ``LeanerIR.Proofs.Denotation.nativeReturn #[vs]
  throw_ kind as := mkAppM ``LeanerIR.Proofs.Denotation.nativeThrow #[kind, as]
  loop site body := mkAppM ``LeanerIR.Proofs.Denotation.nativeLoop #[site, body]
  break_ nest := do
    mkAppM ``LeanerIR.Proofs.Denotation.nativeBreak
      #[nest, ← mkAppOptM ``Option.none
        #[some (mkConst ``LeanerIR.Proofs.Denotation.ExprDenotation)]]
  continue_ nest := mkAppM ``LeanerIR.Proofs.Denotation.nativeContinue #[nest]
  assignLocal i v := mkAppM ``LeanerIR.Proofs.Denotation.nativeAssignLocal #[i, v]
  assignLocalIndex base index value :=
    mkAppM ``LeanerIR.Proofs.Denotation.nativeAssignLocalIndex #[base, index, value]
  spec := pure (mkConst ``LeanerIR.Proofs.Denotation.nativeSpec)
  blockUnit ss := mkAppM ``LeanerIR.Proofs.Denotation.blockUnit #[ss]
  blockResult ss r := mkAppM ``LeanerIR.Proofs.Denotation.blockResult #[ss, r]
  letNoValue body := mkAppM ``LeanerIR.Proofs.Denotation.letNoValue #[body]
  letValue binder i body :=
    mkAppM ``LeanerIR.Proofs.Denotation.letNativeValue #[binder, i, body]
  valuesNil := pure (mkConst ``LeanerIR.Proofs.Denotation.valuesNil)
  valuesCons h t := mkAppM ``LeanerIR.Proofs.Denotation.valuesCons #[h, t]
  statementsNil := pure (mkConst ``LeanerIR.Proofs.Denotation.statementsNil)
  statementsCons h t := mkAppM ``LeanerIR.Proofs.Denotation.statementsCons #[h, t]

/-- The tree's constructors. -/
def treeBuilder : Builder where
  value v := mkAppM ``LeanerIR.Proofs.Denotation.Tree.value #[v]
  localVar i := mkAppM ``LeanerIR.Proofs.Denotation.Tree.localVar #[i]
  primitive op os := mkAppM ``LeanerIR.Proofs.Denotation.Tree.primitive #[op, os]
  global op os := mkAppM ``LeanerIR.Proofs.Denotation.Tree.global #[op, os]
  local_ op os := mkAppM ``LeanerIR.Proofs.Denotation.Tree.local_ #[op, os]
  derefLocalBorrow op os := mkAppM ``LeanerIR.Proofs.Denotation.Tree.derefLocalBorrow #[op, os]
  indexedLocalBorrow op os :=
    mkAppM ``LeanerIR.Proofs.Denotation.Tree.indexedLocalBorrow #[op, os]
  indexedLocalFieldBorrow op os :=
    mkAppM ``LeanerIR.Proofs.Denotation.Tree.indexedLocalFieldBorrow #[op, os]
  reference op os := mkAppM ``LeanerIR.Proofs.Denotation.Tree.reference #[op, os]
  field location os := mkAppM ``LeanerIR.Proofs.Denotation.Tree.field #[location, os]
  variantField location os :=
    mkAppM ``LeanerIR.Proofs.Denotation.Tree.variantField #[location, os]
  variantTest test os :=
    mkAppM ``LeanerIR.Proofs.Denotation.Tree.variantTest #[test, os]
  constructor c os := mkAppM ``LeanerIR.Proofs.Denotation.Tree.constructor #[c, os]
  call handle lexical relation os :=
    mkAppM ``LeanerIR.Proofs.Denotation.Tree.call #[handle, lexical, relation, os]
  callAt handle lexical relation os :=
    mkAppM ``LeanerIR.Proofs.Denotation.Tree.callAt #[handle, lexical, relation, os]
  branch c t e :=
    match e with
    | none => mkAppM ``LeanerIR.Proofs.Denotation.Tree.branchNone #[c, t]
    | some e => mkAppM ``LeanerIR.Proofs.Denotation.Tree.branchSome #[c, t, e]
  return_ vs := mkAppM ``LeanerIR.Proofs.Denotation.Tree.return_ #[vs]
  throw_ kind as := mkAppM ``LeanerIR.Proofs.Denotation.Tree.throw_ #[kind, as]
  loop site body := mkAppM ``LeanerIR.Proofs.Denotation.Tree.loop #[site, body]
  break_ nest := mkAppM ``LeanerIR.Proofs.Denotation.Tree.break_ #[nest]
  continue_ nest := mkAppM ``LeanerIR.Proofs.Denotation.Tree.continue_ #[nest]
  assignLocal i v := mkAppM ``LeanerIR.Proofs.Denotation.Tree.assignLocal #[i, v]
  assignLocalIndex base index value :=
    mkAppM ``LeanerIR.Proofs.Denotation.Tree.assignLocalIndex #[base, index, value]
  spec := pure (mkConst ``LeanerIR.Proofs.Denotation.Tree.spec)
  blockUnit ss := mkAppM ``LeanerIR.Proofs.Denotation.Tree.blockUnit #[ss]
  blockResult ss r := mkAppM ``LeanerIR.Proofs.Denotation.Tree.blockResult #[ss, r]
  letNoValue body := mkAppM ``LeanerIR.Proofs.Denotation.Tree.letNoValue #[body]
  letValue binder i body := mkAppM ``LeanerIR.Proofs.Denotation.Tree.letValue #[binder, i, body]
  valuesNil := pure (mkConst ``LeanerIR.Proofs.Denotation.Operands.nil)
  valuesCons h t := mkAppM ``LeanerIR.Proofs.Denotation.Operands.cons #[h, t]
  statementsNil := pure (mkConst ``LeanerIR.Proofs.Denotation.Statements.nil)
  statementsCons h t := mkAppM ``LeanerIR.Proofs.Denotation.Statements.cons #[h, t]

private partial def emitWith (b : Builder) (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (namespaceId : NamespaceId)
    (namespaceTerm executable : Lean.Expr)
    (callees : Array CalleeDenotation) (id : ExprId) : MetaM Lean.Expr := do
  let some expression := ns.expressions[id.index]?
    | throwError "expression {id.index} is out of range during denotation generation"
  let emit := emitWith b unit ns namespaceId namespaceTerm executable callees
  match expression.kind with
  | .value literal _ =>
      let some runtimeValue := constValue? literal
        | throwError "literal at expression {id.index} has no runtime representation"
      b.value (toExpr runtimeValue)
  | .localVar localId => b.localVar (toExpr localId)
  | .constant reference =>
      let some runtimeValue := foldedConstant? unit namespaceId reference
        | throwError "constant expression {id.index} could not be folded"
      b.value (toExpr runtimeValue)
  | .operation operation instantiations arguments _ =>
      let operands ← emitValues arguments
      let lowered ← match lowerOperation unit namespaceId ns expression.typeId id
          operation instantiations with
        | .ok lowered => pure lowered
        | .error reason => throwError reason
      match lowered with
      | .primitive operation => b.primitive (toExpr operation) operands
      | .global operation => b.global (toExpr operation) operands
      | .local operation => b.local_ (toExpr operation) operands
      | .derefLocalBorrow operation => b.derefLocalBorrow (toExpr operation) operands
      | .indexedLocalBorrow operation => b.indexedLocalBorrow (toExpr operation) operands
      | .indexedLocalFieldBorrow operation =>
          b.indexedLocalFieldBorrow (toExpr operation) operands
      | .field location => b.field (toExpr location) operands
      | .variantField location => b.variantField (toExpr location) operands
      | .variantTest test => b.variantTest (toExpr test) operands
      | .reference operation => b.reference (toExpr operation) operands
      | .constructor constructor => b.constructor (toExpr constructor) operands
      | .function reference =>
          let some handle := resolveFunction? unit namespaceId reference
            | throwError "direct call escaped denotation resolution"
          let some callee := callees.find? (fun candidate => candidate.handle == handle)
            | throwError "direct call escaped denotation dependency generation"
          let lexical := certificateLoanId? unit namespaceId id
          if instantiations.isEmpty then
            let relation ← callee.relationTerm executable
            let relation := if callee.generated.isNone then relation else
              (Lean.mkApp relation (toExpr (#[] : Array (TypeId × TypeId)))).headBeta
            return ← b.call (toExpr handle) (toExpr lexical) relation operands
          let pairType ← mkAppM ``Prod #[mkConst ``TypeId, mkConst ``TypeId]
          let mapType ← mkAppM ``Array #[pairType]
          withLocalDeclD `outerTypeInstantiation mapType fun outer => do
            let executableUnit ← mkAppM ``ExecutableUnit.unit #[executable]
            let calleeMap ← mkAppM ``callTypeInstantiation
              #[executableUnit, toExpr handle, outer, toExpr instantiations]
            let relation ← callee.relationTerm executable
            let relation := Lean.mkApp relation calleeMap
            let family ← mkLambdaFVars #[outer] relation
            b.callAt (toExpr handle) (toExpr lexical) family operands
  | .ifElse condition thenBranch elseBranch =>
      let conditionTerm ← emit condition
      let thenTerm ← emit thenBranch
      let elseTerm ← match elseBranch with
        | none => pure none
        | some elseBranch => some <$> emit elseBranch
      b.branch conditionTerm thenTerm elseTerm
  | .return_ values => b.return_ (← emitValues values)
  | .throw_ kind arguments => b.throw_ (toExpr kind) (← emitValues arguments)
  | .loop _ body => b.loop (toExpr id) (← emit body)
  | .break_ nest none => b.break_ (toExpr nest)
  | .continue_ nest => b.continue_ (toExpr nest)
  | .assign place child =>
      match ns.places[place.index]? with
      | some (.localVar localId) => b.assignLocal (toExpr localId) (← emit child)
      | _ =>
          let some (base, index) := localIndexAssignment? ns place
            | throwError "assignment escaped closed-local native lowering"
          b.assignLocalIndex (toExpr base) (toExpr index) (← emit child)
  | .spec _ => b.spec
  | .block statements result =>
      let statementTerm ← emitStatements statements
      match result with
      | none => b.blockUnit statementTerm
      | some result => b.blockResult statementTerm (← emit result)
  | .letDecl pattern initializer body =>
      let body ← emit body
      match initializer with
      | none => b.letNoValue body
      | some initializer =>
          let some binder := lowerPattern? unit ns pattern
            | throwError "pattern {pattern.index} escaped native lowering"
          b.letValue (toExpr binder) (← emit initializer) body
  | _ => throwError "unsupported expression escaped the denotation precheck"
where
  emitValues (ids : Array ExprId) : MetaM Lean.Expr := do
    let mut tail ← b.valuesNil
    for id in ids.reverse do
      tail ← b.valuesCons
        (← emitWith b unit ns namespaceId namespaceTerm executable callees id) tail
    return tail
  emitStatements (ids : Array ExprId) : MetaM Lean.Expr := do
    let mut tail ← b.statementsNil
    for id in ids.reverse do
      tail ← b.statementsCons
        (← emitWith b unit ns namespaceId namespaceTerm executable callees id) tail
    return tail

/-- The relational denotation of an expression. -/
private def emitExpr := emitWith relationalBuilder

/-- The tree of an expression. -/
private def emitTree := emitWith treeBuilder

/-! ## Uniform agreement proof -/

/-- Close a generated body-agreement goal compositionally.  Every branch is
one application of a checked core lemma; `first` backtracks failed shapes,
and recursion follows only the native child terms already present in the
goal. -/
syntax "leaner_denotation_agree " ident ident : tactic

/-- Closed arena lookups need only walk to the requested slot. Avoid the
array bounds check, which first traverses the complete literal arena. -/
elab "leaner_arena_rfl" : tactic => do
  let (arrayLookup, certificate?) ← withMainContext do
    let target ← instantiateMVars (← (← getMainGoal).getType)
    let some (_, lhs, _) := target.eq? | return (false, none)
    unless lhs.isAppOf ``getElem? && (lhs.getArg! 0).isAppOf ``Array do
      return (false, none)
    let arguments := lhs.getAppArgs
    let array := arguments[arguments.size - 2]!
    let namespace? := array.find? fun term => match term with
      | .const (.str _ "denotationNamespace") _ => true
      | _ => false
    if let some (.const namespaceName _) := namespace? then
      for field in ["expressions", "places"] do
        let name := Name.str namespaceName (field ++ "_index_eq")
        if !(← getEnv).contains name then continue
        let some (_, indexed, _) := (← inferType (mkConst name)).eq? | continue
        if ← withTransparency .reducible <| isDefEq array (indexed.getArg! 1) then
          return (true, some name)
    return (true, none)
  if let some certificate := certificate? then
    evalTactic (← `(tactic|
      (rw [LeanerIR.Validation.IndexedArena.get?_of_index_eq $(mkIdent certificate)]
       rfl)))
    return
  if arrayLookup then
    evalTactic (← `(tactic| rw [← Array.getElem?_toList]; rfl))
  else
    evalTactic (← `(tactic| rfl))

/-- Reuse the exact agreement theorem attached to a named generated callee
relation.  Generic V1 relations are instantiated with one abstract inhabited
carrier here; no representation-specific fact enters the proof. -/
syntax "leaner_denotation_reuse " ident : tactic

/-- Build one small semantic certificate per folded expression/operand node. -/
syntax "leaner_constant_agree " Lean.Parser.Tactic.rwRule : tactic

macro_rules
  | `(tactic| leaner_constant_agree $unitRule:rwRule) => `(tactic|
      first
      | (apply LeanerIR.Proofs.Denotation.value_agrees
          (by first | rfl | (rw [$unitRule]; rfl)) (by rfl) (by rfl)
          (by first | rfl | (simp [LeanerIR.SemanticOperations.constValue?] <;> rfl)))
      | (apply LeanerIR.Proofs.Denotation.primitive_computed_agrees
          (by first | rfl | (rw [$unitRule]; rfl)) (by rfl) (by rfl)
          (by leaner_constant_agree $unitRule)
          (by first | rfl | (simp [LeanerIR.SemanticOperations.evaluatePrimitiveOperation?,
            LeanerIR.SemanticOperations.resolveTargetIntegerType?,
            LeanerIR.SemanticOperations.checkedBinaryInteger,
            LeanerIR.SemanticOperations.checkedInteger, LeanerIR.Ty.integerBounds?] <;> rfl)))
      | (apply LeanerIR.Proofs.Denotation.constant_computed_agrees
          (by first | rfl | (rw [$unitRule]; rfl)) (by rfl) (by rfl)
          (by rw [$unitRule]; rfl) (by rw [$unitRule]; rfl) (by rfl)
          (by leaner_constant_agree $unitRule))
      | exact LeanerIR.Proofs.Denotation.literalValues_nil_agrees
      | (apply LeanerIR.Proofs.Denotation.literalValues_cons_agrees
          (by leaner_constant_agree $unitRule) (by leaner_constant_agree $unitRule)))

/-- Build the proof-only static path certificate for a generated local
reborrow.  Each constructor closes one literal arena lookup; recursion ends
at `deref (localVar _)`. -/
syntax "leaner_deref_local_path" : tactic

private def fieldVariantNames (unit : ValidatedUnit) (handle : StructHandle) :
    List String :=
  match unit.namespaces[handle.namespaceId.index]? with
  | none => []
  | some ns =>
    match ns.structs[handle.structId]? with
    | none => []
    | some declaration => declaration.variants.toList.filterMap fun variant =>
        ns.tables.names[variant.name.index]?.map (·.name)

/-- Discharge the open-string branch using only the declaration's finite
variant-name list, without simplifying an entire generated namespace. -/
private theorem fieldIndexAbsent (unit : ValidatedUnit) (handle : StructHandle)
    (name field : String) (absent : name ∉ fieldVariantNames unit handle) :
    handleFieldIndex? unit handle (some name) field = none := by
  unfold handleFieldIndex?
  suffices fields : handleFields? unit handle (some name) = none by simp [fields]
  cases hns : unit.namespaces[handle.namespaceId.index]? with
  | none => simp [handleFields?, hns]
  | some ns =>
    cases hd : ns.structs[handle.structId]? with
    | none => simp [handleFields?, hns, hd]
    | some declaration =>
      simp only [fieldVariantNames, hns, hd] at absent
      have missing : declaration.variants.toList.find? (fun candidate =>
          (ns.tables.names[candidate.name.index]?).any (·.name == name)) = none := by
        apply List.find?_eq_none.mpr
        intro candidate member
        cases hn : ns.tables.names[candidate.name.index]? with
        | none => simp
        | some entry =>
          have neq : entry.name ≠ name := by
            intro equal
            apply absent
            exact List.mem_filterMap.mpr ⟨candidate, member, by simp [hn, equal]⟩
          simp [neq]
      simp only [handleFields?, hns, hd, Option.bind_eq_bind,
        Option.bind_some, missing, Option.bind_none]

/-- A static table equation must not rewrite a closed namespace back to the
symbolic namespace from the surrounding execution-agreement hypotheses. -/
elab "leaner_static_field_index" : tactic => do
  liftMetaTactic fun goal => do
    let goal ← if (← goal.getType).isForall then do
        let (_, goal) ← goal.intro1P
        pure goal
      else pure goal
    let mut hypotheses := #[]
    for declaration in (← goal.getDecl).lctx do
      hypotheses := hypotheses.push declaration.fvarId
    return [← goal.tryClearMany hypotheses]
  let (actualVariant, unitTerm, handleTerm, fieldTerm, variants) ← withMainContext do
    let goal ← getMainGoal
    let target ← instantiateMVars (← goal.getType)
    let some (_, lhs, rhs) := target.eq?
      | throwError "expected a static field-index equation"
    unless lhs.isAppOfArity ``handleFieldIndex? 4 do
      throwError "expected a static handle field lookup"
    -- Compute the field name before simplification. It is often an arena
    -- projection from the generated namespace, not yet a string literal.
    let fieldName ← withTransparency .all <| reduce (lhs.getArg! 3)
    let handle ← withTransparency .all <| reduce (lhs.getArg! 1)
    let lhs := mkApp lhs.appFn! fieldName
    let normalized ← mkEq lhs rhs
    replaceMainGoal [← goal.change normalized]
    let names ← mkAppM ``fieldVariantNames #[lhs.getArg! 0, lhs.getArg! 1]
    let mut names ← withTransparency .all <| reduce names
    let mut variants := #[]
    while names.isAppOfArity ``List.cons 3 do
      let .lit (.strVal name) := names.getArg! 1
        | throwError "expected a literal variant name"
      variants := variants.push name
      names := names.getArg! 2
    unless names.isAppOfArity ``List.nil 1 do
      throwError "expected a closed variant table"
    return (← PrettyPrinter.delab (lhs.getArg! 2),
      ← PrettyPrinter.delab (lhs.getArg! 0), ← PrettyPrinter.delab handle,
      ← PrettyPrinter.delab fieldName, variants)
  let value := mkIdent (← mkFreshUserName `variant)
  let variantNames ← withMainContext <| PrettyPrinter.delab (toExpr variants.toList)
  let mut proof ← `(tactic|
    (have absent : ¬ $value ∈ $variantNames := by simp_all
     have missing := fieldIndexAbsent $unitTerm
       ($handleTerm : LeanerIR.StructHandle) $value $fieldTerm (by
       change ¬ $value ∈ $variantNames
       exact absent)
     rw [missing]
     simp_all))
  for variant in variants.reverse do
    let name := Syntax.mkStrLit variant
    let hypothesis := mkIdent (← mkFreshUserName `variant_eq)
    proof ← `(tactic|
      (by_cases $hypothesis : $value = $name
       · subst $value; rfl
       have := Ne.symm $hypothesis
       $proof))
  evalTactic (← `(tactic|
    cases $actualVariant:term with
    | none => rfl
    | some $value => $proof:tactic))

macro_rules
  | `(tactic| leaner_deref_local_path) =>
      `(tactic|
        first
        | exact LeanerIR.SemanticOperations.DerefLocalFieldPath.deref
            (place_eq := by rfl) (base_eq := by rfl)
        | exact LeanerIR.SemanticOperations.DerefLocalFieldPath.field
            (step := _)
            (place_eq := by rfl)
            (base_path := by leaner_deref_local_path)
            (resolved_eq := by rfl)
            (name_eq := by rfl)
            (index_eq := by
              first
              | (intro actualVariant; cases actualVariant <;> rfl)
              | leaner_static_field_index))

elab_rules : tactic
  | `(tactic| leaner_denotation_reuse $unitEq:ident) => do
      let goal ← Lean.Elab.Tactic.getMainGoal
      trace[leaner.agreement] "node: {← goal.withContext do pure ((← instantiateMVars (← goal.getType)).getAppArgs.back?.map (·.getAppFn))}, {← IO.getNumHeartbeats}"
      let relationName ← goal.withContext do
        let target ← Lean.instantiateMVars (← goal.getType)
        unless target.getAppFn.isConstOf
              ``LeanerIR.Proofs.Denotation.FunctionDenotation.Agrees ||
            target.getAppFn.isConstOf
              ``LeanerIR.Proofs.Denotation.FunctionDenotation.AgreesWith do
          Lean.Meta.throwTacticEx `leaner_denotation_reuse goal
            "the goal is not function-denotation agreement"
        let some relation := target.getAppArgs.back?
          | Lean.Meta.throwTacticEx `leaner_denotation_reuse goal
              "the agreement goal has no relation"
        let some relationName := relation.getAppFn.constName?
          | Lean.Meta.throwTacticEx `leaner_denotation_reuse goal
              "the callee relation is not a named generated definition"
        return relationName
      let agreement := Name.str relationName.getPrefix
        "denotationRelation_agrees"
      let agreementSyntax : Lean.TSyntax `term :=
        ⟨(mkIdent agreement).raw⟩
      let carrierIdent := mkIdent `Carrier
      Lean.Elab.Tactic.evalTactic
        (← `(tactic|
          first
          | exact $agreementSyntax $unitEq
          | exact $agreementSyntax
              ($carrierIdent := fun _ => PUnit) $unitEq))

/-- A case tag as the goal carries it: an identifier without macro scopes. -/
private def tag (name : String) : Lean.Ident :=
  Lean.mkIdent (Lean.Name.mkSimple name)

/-- Constructor arity is a closed boundary lookup. Array's well-founded
search is not definitionally reducible, so use its list equation rather
than relying on `rfl` for enum variants. -/
elab "leaner_constructor_fields" : tactic => do
  let goal ← Lean.Elab.Tactic.getMainGoal
  let definitions ← goal.withContext do
    let target ← Lean.instantiateMVars (← goal.getType)
    let some lookup := target.find? (·.isAppOfArity
        ``LeanerIR.SemanticOperations.constructorFields? 3)
      | Lean.Meta.throwTacticEx `leaner_constructor_fields goal "not a constructor-field lookup"
    let unit := lookup.getArg! 0
    unless unit.isConst do
      Lean.Meta.throwTacticEx `leaner_constructor_fields goal "constructor unit is not a closed definition"
    -- Preparation shares unchanged tables and declarations with preceding
    -- units. Expose that short chain before rewriting Array's opaque search,
    -- without unfolding the preparation algorithms themselves.
    let mut pending := [unit.constName!]
    let mut names : Array Name := #[]
    while !pending.isEmpty do
      let name := pending.head!
      pending := pending.tail!
      if names.contains name then continue
      let info ← getConstInfo name
      unless info.type.isConstOf ``LeanerIR.Validation.ValidatedUnit do continue
      let some value := info.value? | continue
      names := names.push name
      pending := value.getUsedConstants.toList ++ pending
    names.mapM fun name =>
      `(Lean.Parser.Tactic.simpLemma| $(mkCIdent name):ident)
  Lean.Elab.Tactic.evalTactic (← `(tactic|
    simp [LeanerIR.SemanticOperations.constructorFields?, $definitions,*,
      List.findIdx?_toArray, List.findIdx?, List.findIdx?.go]))

elab_rules : tactic
  | `(tactic| leaner_denotation_agree $namespaceEq:ident $unitEq:ident) => do
      /- A well-formed rewrite rule, not a bare identifier coerced to one:
      `rw` reads the rule's node kind. -/
      let unitRule ← `(Lean.Parser.Tactic.rwRule| $unitEq:ident)
      let unitSimp : TSyntax ``Lean.Parser.Tactic.simpLemma := ⟨unitEq.raw⟩
      let goal ← Lean.Elab.Tactic.getMainGoal
      trace[leaner.agreement] "agree: {← goal.withContext do pure ((← instantiateMVars (← goal.getType)).getAppArgs.back?.map (·.getAppFn))}, {← IO.getNumHeartbeats}"
      let isValuesAgreement ← goal.withContext do
        let target ← Lean.instantiateMVars (← goal.getType)
        return target.getAppFn.isConstOf
            ``LeanerIR.Proofs.Denotation.ValuesDenotation.Agrees ||
          target.getAppFn.isConstOf
            ``LeanerIR.Proofs.Denotation.ValuesDenotation.AgreesWith
      if isValuesAgreement then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (try simp only [Array.toList_push, Array.toList_empty, List.nil_append]
             first
             | exact LeanerIR.Proofs.Denotation.valuesNil_agrees
             | (apply LeanerIR.Proofs.Denotation.valuesCons_agrees <;>
                 leaner_denotation_agree $namespaceEq $unitEq))))
        return
      let isStatementsAgreement ← goal.withContext do
        let target ← Lean.instantiateMVars (← goal.getType)
        return target.getAppFn.isConstOf
            ``LeanerIR.Proofs.Denotation.StatementsDenotation.Agrees ||
          target.getAppFn.isConstOf
            ``LeanerIR.Proofs.Denotation.StatementsDenotation.AgreesWith
      if isStatementsAgreement then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (try simp only [Array.toList_push, Array.toList_empty, List.nil_append]
             first
             | exact LeanerIR.Proofs.Denotation.statementsNil_agrees
             | (apply LeanerIR.Proofs.Denotation.statementsCons_agrees <;>
                 leaner_denotation_agree $namespaceEq $unitEq))))
        return
      let denotationHead? ← goal.withContext do
        let target ← Lean.instantiateMVars (← goal.getType)
        return target.getAppArgs.back?.bind (·.getAppFn.constName?)
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.value then
        /- A literal, or a certified folded constant initializer. -/
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            first
            | (apply LeanerIR.Proofs.Denotation.value_agrees
                (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
                first
                | leaner_arena_rfl
                | simp [LeanerIR.SemanticOperations.constValue?])
            | leaner_constant_agree $unitRule))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.localVar then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.local_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;> leaner_arena_rfl)))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeBranch then
        /- Select the certificate from the generated constructor. Trying
        reflexivity or unrelated agreement lemmas on a branch unfolds its
        entire relational continuation before visiting its children. -/
        let hasElse ← goal.withContext do
          let target ← Lean.instantiateMVars (← goal.getType)
          return (target.getAppArgs.back!.getArg! 2).isAppOf ``Option.some
        let agreement := mkIdent <| if hasElse then
          ``LeanerIR.Proofs.Denotation.nativeBranchElse_agrees
        else
          ``LeanerIR.Proofs.Denotation.nativeBranchUnit_agrees
        Lean.Elab.Tactic.evalTactic (← `(tactic|
          (apply $agreement
            (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
            (by leaner_arena_rfl) (by leaner_arena_rfl) <;>
            leaner_denotation_agree $namespaceEq $unitEq)))
        return
      let dataEvaluator? ← goal.withContext do
        let target ← Lean.instantiateMVars (← goal.getType)
        let some denotation := target.getAppArgs.back? | return none
        unless denotation.getAppFn.isConstOf
            ``LeanerIR.Proofs.Denotation.nativeOperation do return none
        return (denotation.getArg! 0).getAppFn.constName?
      let dataAgreement? := match dataEvaluator? with
        | some ``LeanerIR.Proofs.Denotation.NominalVariantTest.evaluate? =>
            some ``LeanerIR.Proofs.Denotation.NominalVariantTest.evaluator_eq_of_unit
        | some ``LeanerIR.Proofs.Denotation.NominalVariantFieldLocation.evaluateSelect? =>
            some ``LeanerIR.Proofs.Denotation.NominalVariantFieldLocation.select_evaluator_eq_of_unit
        | _ => none
      if let some dataAgreement := dataAgreement? then
        let agreement := mkIdent dataAgreement
        Lean.Elab.Tactic.evalTactic (← `(tactic|
          apply LeanerIR.Proofs.Denotation.nativeData_agrees
            (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
            (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl)
            (by apply $agreement $unitEq <;> leaner_arena_rfl)
            (by leaner_denotation_agree $namespaceEq $unitEq)))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeReturn then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeReturn_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl)
              (by leaner_denotation_agree $namespaceEq $unitEq))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeThrow then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeThrow_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl)
              (by leaner_denotation_agree $namespaceEq $unitEq))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeLoop then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeLoop_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl)
              (by leaner_denotation_agree $namespaceEq $unitEq))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeBreak then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeBreakNone_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeContinue then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeContinue_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeAssignLocal then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeAssignLocal_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl)
              (by leaner_denotation_agree $namespaceEq $unitEq))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeAssignLocalIndex then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeAssignLocalIndex_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl)
              (by first | exact Or.inl rfl | exact Or.inr rfl)
              (by leaner_denotation_agree $namespaceEq $unitEq))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeSpec then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeSpec_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.blockResult then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.blockResult_agrees
                (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
             · leaner_arena_rfl
             · leaner_arena_rfl
             · leaner_denotation_agree $namespaceEq $unitEq
             · leaner_denotation_agree $namespaceEq $unitEq)))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.letNativeValue then
        /- Keep native lets on a deterministic agreement path.  In
        particular, owned-local loan scopes contain calls and constructors;
        letting the generic alternative chain backtrack across that whole
        subtree both hides the failing child and duplicates substantial
        elaboration work. -/
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.letNativeValue_agrees_of_unit
                $unitEq
                (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
             · leaner_arena_rfl
             · leaner_arena_rfl
             · leaner_arena_rfl
             · leaner_denotation_agree $namespaceEq $unitEq
             · leaner_denotation_agree $namespaceEq $unitEq)))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeLocalOperation then
        -- Local reads and borrows select their semantic certificate directly.
        -- Reflexivity on an evaluator equation unfolds the whole place
        -- resolver before the reusable certificate gets a chance to apply.
        Lean.Elab.Tactic.evalTactic (← `(tactic|
          apply LeanerIR.Proofs.Denotation.nativeLocal_agrees
            (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
            (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl)
            (by
              first
              | (apply LeanerIR.Proofs.Denotation.LocalLocationOperation.read_evaluator_eq
                  <;> leaner_arena_rfl)
              | (apply LeanerIR.Proofs.Denotation.LocalLocationOperation.copy_evaluator_eq
                  <;> leaner_arena_rfl)
              | (apply LeanerIR.Proofs.Denotation.LocalLocationOperation.move_evaluator_eq
                  <;> leaner_arena_rfl)
              | (apply LeanerIR.Proofs.Denotation.LocalLocationOperation.borrow_evaluator_eq
                 · leaner_arena_rfl
                 · leaner_arena_rfl
                 · funext frame state place
                   first
                   | exact
                       (LeanerIR.SemanticOperations.borrowRuntimePlace?_immutable_at
                         _ _ _ _ _ _ _ _).symm
                   | exact
                       (LeanerIR.SemanticOperations.borrowRuntimePlace?_mutable_at
                         _ _ _ _ _ _ _ _ (by rw [$unitRule]; leaner_arena_rfl)).symm))
            (by leaner_denotation_agree $namespaceEq $unitEq)))
        return
      let isNativeCall ← goal.withContext do
        let target ← Lean.instantiateMVars (← goal.getType)
        let some denotation := target.getAppArgs.back? | return false
        return denotation.getAppFn.isConstOf
          ``LeanerIR.Proofs.Denotation.nativeCall
      if isNativeCall then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeCall_monomorphic_agrees
              (by exact $namespaceEq) (by leaner_arena_rfl) (by leaner_arena_rfl)
              (by leaner_arena_rfl)
              (by leaner_denotation_agree $namespaceEq $unitEq)
              (by assumption)
              (by assumption)
              (by
                have unitEquality := $unitEq
                rw [unitEquality]
                leaner_arena_rfl))))
        return
      let isNativeCallAt ← goal.withContext do
        let target ← Lean.instantiateMVars (← goal.getType)
        let some denotation := target.getAppArgs.back? | return false
        return denotation.getAppFn.isConstOf
          ``LeanerIR.Proofs.Denotation.nativeCallAt
      if isNativeCallAt then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeCallAt_agrees
              (by exact $namespaceEq) (by leaner_arena_rfl) (by leaner_arena_rfl)
              (by leaner_denotation_agree $namespaceEq $unitEq)
              (by
                intro outer
                first
                | apply_assumption
                | (apply LeanerIR.Proofs.Denotation.FunctionDenotation.AgreesWith.of_typeInstantiation_eq
                      (expected := #[])
                   · leaner_arena_rfl
                   · assumption)
                | (have unitEquality := $unitEq
                   rw [unitEquality]
                   assumption))
              (by assumption)
              (by
                have unitEquality := $unitEq
                rw [unitEquality]
                leaner_arena_rfl))))
        return
      /- Nominal constructors are represented by the generic native-operation
      combinator.  Dispatch them before the fallback search so a constructor
      nested under a loan scope does not make the enclosing expression's
      entire agreement branch backtrack. -/
      let isNativeConstructor ← goal.withContext do
        let target ← Lean.instantiateMVars (← goal.getType)
        let some denotation := target.getAppArgs.back? | return false
        unless denotation.getAppFn.isConstOf
            ``LeanerIR.Proofs.Denotation.nativeOperation do
          return false
        return (denotation.getArg! 0).getAppFn.isConstOf
          ``LeanerIR.Proofs.Denotation.NominalConstructor.evaluate?
      if isNativeConstructor then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeConstructor_agrees
                (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
             · leaner_arena_rfl
             · leaner_arena_rfl
             · apply LeanerIR.Proofs.Denotation.NominalConstructor.evaluator_eq_of_unit
                 $unitEq <;> first | leaner_arena_rfl | leaner_constructor_fields
             · leaner_denotation_agree $namespaceEq $unitEq)))
        return
      let isNativePrimitive ← goal.withContext do
        let target ← Lean.instantiateMVars (← goal.getType)
        let some denotation := target.getAppArgs.back? | return false
        return denotation.getAppFn.isConstOf
          ``LeanerIR.Proofs.Denotation.nativePrimitiveOperation
      if isNativePrimitive then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativePrimitiveOperation_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl)
              (by first
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.add_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.tuple_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.vector_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.pushVector_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.concatVector_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.slice_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.insertVector_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.removeVector_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.swapVector_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.reverseSliceVector_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.destroyEmptyVector_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.containsVector_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.indexOfVector_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkVectorIndex_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.length_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.logicalNot_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.index_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.copyValue_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.moveValue_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedAdd_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.subtract_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedSubtract_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.multiply_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedMultiply_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.greater_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.lessEqual_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.greaterEqual_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.equal_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.notEqual_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.divide_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedDivide_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.modulo_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedModulo_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.bitwiseOr_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.bitwiseAnd_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.bitwiseXor_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.bitwiseNot_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.shiftLeft_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedShiftLeft_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.shiftRight_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedShiftRight_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.cast_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedCast_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.logicalAnd_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.logicalOr_evaluator_eq
                    <;> leaner_arena_rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.less_evaluator_eq
                    <;> leaner_arena_rfl))
              (by leaner_denotation_agree $namespaceEq $unitEq))))
        return
      if denotationHead? == some
          ``LeanerIR.Proofs.Denotation.nativeReferenceOperation then
        /- Reference operations commonly wrap calls when the lowered form
        retires a temporary reborrow.  Keep that composition on a direct
        path so agreement construction does not backtrack over the callee. -/
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeReference_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl)
              (by first
                | apply LeanerIR.Proofs.Denotation.ReferenceLocationOperation.dereference_evaluator_eq
                | apply LeanerIR.Proofs.Denotation.ReferenceLocationOperation.mutate_evaluator_eq
                | (apply LeanerIR.Proofs.Denotation.ReferenceLocationOperation.freeze_evaluator_eq
                    <;> leaner_arena_rfl)
                | apply LeanerIR.Proofs.Denotation.ReferenceLocationOperation.endLoan_evaluator_eq)
              (by leaner_denotation_agree $namespaceEq $unitEq))))
        return
      let isNativeDerefLocalBorrow ← goal.withContext do
        let target ← Lean.instantiateMVars (← goal.getType)
        let some denotation := target.getAppArgs.back? | return false
        return denotation.getAppFn.isConstOf
          ``LeanerIR.Proofs.Denotation.nativeDerefLocalBorrowOperation
      if isNativeDerefLocalBorrow then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeDerefLocalBorrow_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl)
              (by
                apply LeanerIR.Proofs.Denotation.DerefLocalBorrowOperation.evaluator_eq_path_of_unit
                  $unitEq (path := by leaner_deref_local_path)
                · leaner_arena_rfl
                · decide
                · leaner_arena_rfl
                · funext frame state place
                  first
                  | exact
                      (LeanerIR.SemanticOperations.borrowRuntimePlace?_immutable_at
                        _ _ _ _ _ _ _ _).symm
                  | exact
                      (LeanerIR.SemanticOperations.borrowRuntimePlace?_mutable_at
                        _ _ _ _ _ _ _ _
                        (by leaner_arena_rfl)).symm)
              (by leaner_denotation_agree $namespaceEq $unitEq))))
        return
      let indexedLocalFieldBorrowShape? ← goal.withContext do
        let target ← Lean.instantiateMVars (← goal.getType)
        let some denotation := target.getAppArgs.back? | return none
        unless denotation.getAppFn.isConstOf
            ``LeanerIR.Proofs.Denotation.nativeIndexedLocalFieldBorrowOperation do
          return none
        let operation := denotation.getArg! 0
        unless operation.getAppFn.isConstOf
            ``LeanerIR.Proofs.Denotation.IndexedLocalFieldBorrowOperation.mk do
          return none
        return some ((operation.getArg! 4).isConstOf ``BorrowKind.mutable)
      if let some mutable := indexedLocalFieldBorrowShape? then
        let borrower ← if mutable then
          `(tactic|
            exact
              (LeanerIR.SemanticOperations.borrowRuntimePlace?_mutable_at
                _ _ _ _ _ _ _ _ (by leaner_arena_rfl)).symm)
        else
          `(tactic|
            exact
              (LeanerIR.SemanticOperations.borrowRuntimePlace?_immutable_at
                _ _ _ _ _ _ _ _).symm)
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            apply LeanerIR.Proofs.Denotation.nativeIndexedLocalFieldBorrow_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl)))
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            apply LeanerIR.Proofs.Denotation.IndexedLocalFieldBorrowOperation.evaluator_eq_of_unit
              $unitEq))
        Lean.Elab.Tactic.evalTactic (← `(tactic| leaner_arena_rfl))
        Lean.Elab.Tactic.evalTactic
          (← `(tactic| funext frame state))
        Lean.Elab.Tactic.evalTactic
          (← `(tactic| simp only [LeanerIR.Proofs.Denotation.IndexedLocalFieldBorrowOperation.resolve?]))
        Lean.Elab.Tactic.evalTactic
          (← `(tactic| exact
            (LeanerIR.SemanticOperations.resolvePlace?_fieldOfLocalLiteralIndex
              (place_eq := by leaner_arena_rfl) (index_place_eq := by leaner_arena_rfl)
              (base_eq := by leaner_arena_rfl) (index_eq := by leaner_arena_rfl)
              (resolved_eq := by leaner_arena_rfl) (name_eq := by leaner_arena_rfl)
              (field_index_eq := by
                intro actualVariant
                cases actualVariant <;> leaner_arena_rfl)).symm))
        Lean.Elab.Tactic.evalTactic
          (← `(tactic| funext frame state place; $borrower:tactic))
        Lean.Elab.Tactic.evalTactic
          (← `(tactic| leaner_denotation_agree $namespaceEq $unitEq))
        return
      let indexedLocalBorrowShape? ← goal.withContext do
        let target ← Lean.instantiateMVars (← goal.getType)
        let some denotation := target.getAppArgs.back? | return none
        unless denotation.getAppFn.isConstOf
            ``LeanerIR.Proofs.Denotation.nativeIndexedLocalBorrowOperation do
          return none
        let operation := denotation.getArg! 0
        unless operation.isAppOfArity
            ``LeanerIR.Proofs.Denotation.IndexedLocalBorrowOperation.mk 7 do
          return none
        let dereference := (operation.getArg! 1).isConstOf ``Bool.true
        let mutable := (operation.getArg! 4).isConstOf ``BorrowKind.mutable
        let dynamic := (operation.getArg! 6).isAppOfArity ``Option.some 2
        return some (dereference, mutable, dynamic)
      if let some (dereference, mutable, dynamic) := indexedLocalBorrowShape? then
        let resolver ← if dynamic then
          if dereference then
            `(tactic| exact
              LeanerIR.Proofs.Denotation.IndexedLocalBorrowOperation.resolve_eq_dynamic_deref_of_unit
                $unitEq _ (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl)
                (by first | exact Or.inl rfl | exact Or.inr rfl))
          else
            `(tactic| exact
              LeanerIR.Proofs.Denotation.IndexedLocalBorrowOperation.resolve_eq_dynamic_local_of_unit
                $unitEq _ (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl)
                (by first | exact Or.inl rfl | exact Or.inr rfl))
        else if dereference then
          `(tactic|
            exact
              LeanerIR.Proofs.Denotation.IndexedLocalBorrowOperation.resolve_eq_deref_of_unit
                $unitEq _ (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl))
        else
          `(tactic|
            exact
              LeanerIR.Proofs.Denotation.IndexedLocalBorrowOperation.resolve_eq_local_of_unit
                $unitEq _ (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl))
        let borrower ← if mutable then
          `(tactic|
            exact
              (LeanerIR.SemanticOperations.borrowRuntimePlace?_mutable_at
                _ _ _ _ _ _ _ _ (by leaner_arena_rfl)).symm)
        else
          `(tactic|
            exact
              (LeanerIR.SemanticOperations.borrowRuntimePlace?_immutable_at
                _ _ _ _ _ _ _ _).symm)
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            apply LeanerIR.Proofs.Denotation.nativeIndexedLocalBorrow_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl) (by leaner_arena_rfl)))
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            apply LeanerIR.Proofs.Denotation.IndexedLocalBorrowOperation.evaluator_eq_of_unit
              $unitEq))
        Lean.Elab.Tactic.evalTactic (← `(tactic| leaner_arena_rfl))
        Lean.Elab.Tactic.evalTactic resolver
        Lean.Elab.Tactic.evalTactic
          (← `(tactic| funext frame state place; $borrower:tactic))
        Lean.Elab.Tactic.evalTactic
          (← `(tactic| leaner_denotation_agree $namespaceEq $unitEq))
        return
      let script ← `(tactic|
        first
        | assumption
        | leaner_denotation_reuse $unitEq
        | (simp only [Array.toList_push, Array.toList_empty, List.nil_append]
            <;> first
            | exact LeanerIR.Proofs.Denotation.valuesNil_agrees
            | exact LeanerIR.Proofs.Denotation.statementsNil_agrees
            | (apply LeanerIR.Proofs.Denotation.valuesCons_agrees <;>
                leaner_denotation_agree $namespaceEq $unitEq)
            | (apply LeanerIR.Proofs.Denotation.statementsCons_agrees <;>
                leaner_denotation_agree $namespaceEq $unitEq))
        | exact LeanerIR.Proofs.Denotation.valuesNil_agrees
        | exact LeanerIR.Proofs.Denotation.statementsNil_agrees
        | (apply LeanerIR.Proofs.Denotation.valuesCons_agrees <;>
            leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.statementsCons_agrees <;>
            leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.value_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;> leaner_arena_rfl)
        | (apply LeanerIR.Proofs.Denotation.local_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;> leaner_arena_rfl)
        | (apply LeanerIR.Proofs.Denotation.nativePrimitiveOperation_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first
            | leaner_arena_rfl
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.add_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.vector_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.pushVector_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.concatVector_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.slice_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.insertVector_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.removeVector_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.swapVector_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.reverseSliceVector_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.destroyEmptyVector_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.containsVector_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.indexOfVector_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkVectorIndex_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.length_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.logicalNot_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.index_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.copyValue_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.moveValue_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedAdd_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.subtract_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedSubtract_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.multiply_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedMultiply_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.greater_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.lessEqual_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.greaterEqual_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.equal_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.notEqual_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.divide_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedDivide_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.modulo_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedModulo_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.bitwiseOr_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.bitwiseAnd_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.bitwiseXor_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.bitwiseNot_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.shiftLeft_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedShiftLeft_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.shiftRight_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedShiftRight_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.cast_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedCast_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.logicalAnd_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.logicalOr_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.less_evaluator_eq
                <;> leaner_arena_rfl)
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeGlobal_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first
            | leaner_arena_rfl
            | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.borrow_evaluator_eq_of_unit
                  $unitEq <;> first
                | leaner_arena_rfl
                | (funext frame state place
                   first
                   | exact
                       (LeanerIR.SemanticOperations.borrowRuntimePlace?_immutable_at
                         _ _ _ _ _ _ _ _).symm
                   | exact
                       (LeanerIR.SemanticOperations.borrowRuntimePlace?_mutable_at
                         _ _ _ _ _ _ _ _ (by leaner_arena_rfl)).symm))
            | (simp only [$unitSimp]
               first
               | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.contains_evaluator_eq
                   <;> leaner_arena_rfl)
               | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.borrow_evaluator_eq
                   <;> first
                   | leaner_arena_rfl
                   | (funext frame state place
                      first
                      | exact
                          (LeanerIR.SemanticOperations.borrowRuntimePlace?_immutable_at
                            _ _ _ _ _ _ _ _).symm
                      | exact
                          (LeanerIR.SemanticOperations.borrowRuntimePlace?_mutable_at
                            _ _ _ _ _ _ _ _ (by leaner_arena_rfl)).symm)
                   | (rw [$unitRule]; leaner_arena_rfl))
               | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.take_evaluator_eq
                   <;> leaner_arena_rfl)
               | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.publish_evaluator_eq
                   <;> leaner_arena_rfl))
            | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.contains_evaluator_eq
                <;> first | leaner_arena_rfl | (rw [$unitRule]; leaner_arena_rfl))
            | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.borrow_evaluator_eq
                <;> first
                | leaner_arena_rfl
                | (rw [$unitRule]; leaner_arena_rfl)
                | (funext frame state place
                   first
                   | exact
                       (LeanerIR.SemanticOperations.borrowRuntimePlace?_immutable_at
                         _ _ _ _ _ _ _ _).symm
                   | exact
                       (LeanerIR.SemanticOperations.borrowRuntimePlace?_mutable_at
                         _ _ _ _ _ _ _ _ (by rw [$unitRule]; leaner_arena_rfl)).symm))
            | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.take_evaluator_eq
                <;> first | leaner_arena_rfl | (rw [$unitRule]; leaner_arena_rfl))
            | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.publish_evaluator_eq
                <;> first | leaner_arena_rfl | (rw [$unitRule]; leaner_arena_rfl))
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeLocal_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first
            | leaner_arena_rfl
            | (apply LeanerIR.Proofs.Denotation.LocalLocationOperation.read_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.LocalLocationOperation.copy_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.LocalLocationOperation.move_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.LocalLocationOperation.borrow_evaluator_eq
                <;> first
                | leaner_arena_rfl
                | (funext frame state place
                   first
                   | exact
                       (LeanerIR.SemanticOperations.borrowRuntimePlace?_immutable_at
                         _ _ _ _ _ _ _ _).symm
                   | exact
                       (LeanerIR.SemanticOperations.borrowRuntimePlace?_mutable_at
                         _ _ _ _ _ _ _ _ (by rw [$unitRule]; leaner_arena_rfl)).symm))
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeData_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first
            | leaner_arena_rfl
            | (apply LeanerIR.Proofs.Denotation.NominalFieldLocation.select_evaluator_eq_of_unit
                 $unitEq <;> first
                 | leaner_arena_rfl
                 | (intro actualVariant; cases actualVariant <;> leaner_arena_rfl))
            | (apply LeanerIR.Proofs.Denotation.NominalVariantFieldLocation.select_evaluator_eq_of_unit
                 $unitEq <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.NominalFieldLocation.select_evaluator_eq
                <;> first
                | leaner_arena_rfl
                | (rw [$unitRule]; leaner_arena_rfl)
                | (intro actualVariant; cases actualVariant <;>
                    rw [$unitRule] <;> leaner_arena_rfl))
            | (apply LeanerIR.Proofs.Denotation.NominalVariantFieldLocation.select_evaluator_eq
                <;> first | leaner_arena_rfl | (rw [$unitRule]; leaner_arena_rfl))
            | (apply LeanerIR.Proofs.Denotation.NominalVariantTest.evaluator_eq_of_unit
                 $unitEq <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.NominalVariantTest.evaluator_eq
                <;> first | leaner_arena_rfl | (rw [$unitRule]; leaner_arena_rfl))
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeReference_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first
            | leaner_arena_rfl
            | (apply LeanerIR.Proofs.Denotation.ReferenceLocationOperation.dereference_evaluator_eq)
            | (apply LeanerIR.Proofs.Denotation.ReferenceLocationOperation.mutate_evaluator_eq)
            | (apply LeanerIR.Proofs.Denotation.ReferenceLocationOperation.freeze_evaluator_eq
                <;> leaner_arena_rfl)
            | (apply LeanerIR.Proofs.Denotation.ReferenceLocationOperation.endLoan_evaluator_eq)
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeConstructor_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first
            | leaner_arena_rfl
            | (apply LeanerIR.Proofs.Denotation.NominalConstructor.evaluator_eq_of_unit
                 $unitEq <;> first | leaner_arena_rfl | leaner_constructor_fields)
            | (apply LeanerIR.Proofs.Denotation.NominalConstructor.evaluator_eq
                <;> first | leaner_arena_rfl | leaner_constructor_fields | (rw [$unitRule]; leaner_constructor_fields))
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.primitive_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first | leaner_arena_rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.global_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first | leaner_arena_rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.data_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first | leaner_arena_rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.reference_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first | leaner_arena_rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.constructor_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first | leaner_arena_rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeBranchUnit_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first | leaner_arena_rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeBranchElse_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first | leaner_arena_rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeReturn_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first | leaner_arena_rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeThrow_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first | leaner_arena_rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.blockUnit_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first | leaner_arena_rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.blockResult_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first | leaner_arena_rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.letNoValue_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first | leaner_arena_rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.letNativeValue_agrees_of_unit $unitEq
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first
            | leaner_arena_rfl
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.letValue_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl)) <;>
            first | leaner_arena_rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.call_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl) (by rw [$unitRule]; leaner_arena_rfl)
              (loan_eq := by
                have unitEquality := $unitEq
                rw [unitEquality]
                leaner_arena_rfl)
            · leaner_denotation_agree $namespaceEq $unitEq
            · assumption)
        | (apply LeanerIR.Proofs.Denotation.callAt_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; leaner_arena_rfl))
              (by leaner_arena_rfl) (by leaner_arena_rfl) (by rw [$unitRule]; leaner_arena_rfl)
              (loan_eq := by
                have unitEquality := $unitEq
                rw [unitEquality]
                leaner_arena_rfl)
            · leaner_denotation_agree $namespaceEq $unitEq
            · intro outer
              solve_by_elim)
        | (apply LeanerIR.Proofs.Denotation.functionRelation_agrees <;>
            first
            | leaner_arena_rfl
            | (rw [$unitRule]; leaner_arena_rfl)
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeFunctionRelation_agrees <;>
            first
            | leaner_arena_rfl
            | (rw [$unitRule]; leaner_arena_rfl)
            | leaner_denotation_agree $namespaceEq $unitEq)
        | fail "no denotation agreement rule applies")
      evalTactic script

/-! ## Declaration materialization -/

private def addAbbrev (name : Name) (type value : Lean.Expr) : TermElabM Unit := do
  addDecl (.defnDecl {
    name, levelParams := [], type, value, hints := .abbrev, safety := .safe })
  enableRealizationsForConst name

/-- Build each namespace's balanced view once. Per-function aliases share
its certificate, so agreement does not re-scan the source arena per node. -/
private def ensureArenaIndex {α : Type} [ToExpr α] (unitDef namespaceDef : Name)
    (namespaceIndex : Nat) (field : String) (arraySyntax : Term)
    (values : Array α) : TermElabM Unit := do
  let aliasName := Name.str namespaceDef (field ++ "_index_eq")
  if (← getEnv).contains aliasName then return
  let base := Name.str unitDef s!"arena{namespaceIndex}"
  let indexName := Name.str base field
  let certificateName := Name.str base (field ++ "_eq")
  let array ← Lean.Elab.Term.elabTerm arraySyntax none
  Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
  let array ← instantiateMVars array
  let indexed ← mkAppM ``IndexedArena.ofArray #[array]
  if !(← getEnv).contains indexName then
    addAbbrev indexName (← inferType indexed) (toExpr (IndexedArena.ofArray values))
  let equation ← mkEq indexed (mkConst indexName)
  if !(← getEnv).contains certificateName then
    addDecl (.thmDecl {
      name := certificateName, levelParams := [], type := equation
      value := ← mkEqRefl (mkConst indexName) })
  addDecl (.thmDecl {
    name := aliasName, levelParams := [], type := equation
    value := mkConst certificateName })

private structure DirectCall where
  reference : QualifiedRef
  handle : FunctionHandle

private def pushDirectCall (calls : Array DirectCall)
    (call : DirectCall) : Array DirectCall :=
  if calls.any fun candidate =>
      candidate.reference == call.reference && candidate.handle == call.handle then
    calls
  else
    calls.push call

/-- Direct callees reachable from one supported body. -/
private partial def collectCalls (unit : ValidatedUnit)
    (namespaceId : NamespaceId) (ns : ValidatedNamespace)
    (seen : Array Nat) (id : ExprId) : Except String (Array DirectCall) := do
  if seen.contains id.index then return #[]
  let some expression := ns.expressions[id.index]?
    | throw s!"expression {id.index} is out of range while collecting calls"
  let mut found := #[]
  if let .operation (.call (.function reference)) _ _ _ := expression.kind then
    let some handle := resolveFunction? unit namespaceId reference
      | throw s!"direct call at expression {id.index} does not resolve"
    found := found.push ⟨reference, handle⟩
  for child in Validation.expressionChildren expression.kind do
    let nested ← collectCalls unit namespaceId ns (seen.push id.index) child
    for call in nested do
      found := pushDirectCall found call
  return found

/-! Denotation artifacts are keyed by semantic function identity rather than
the verification entry point which first reaches them.  This is load-bearing
for parametric V1: direct verification and every concrete caller must reuse
one generic body and one agreement proof. -/
def functionSegments (root : Array String)
    (handle : FunctionHandle) : Array String :=
  root ++ #["_denotation_dependencies",
    s!"namespace{handle.namespaceId.index}",
    s!"function{handle.functionId.index}"]

private def functionSpelling? (unit : ValidatedUnit)
    (declaration : FunctionDecl FunctionBody) : Option String := do
  let qualified ← unit.tables.names[declaration.name.index]?
  return qualified.name

/-- Recursive worker.  The active handle row is exactly the current DFS
stack: encountering it diagnoses a same-SCC call before any declarations are
materialized. -/
private partial def ensureDefinitionsInternal (unit : ValidatedUnit)
    (unitDef : Name) (rootSegments definitionSegments : Array String)
    (function : String)
    (namespaceIndex functionIndex : Nat) (ns : ValidatedNamespace)
    (declaration : FunctionDecl FunctionBody)
    (stack : Array FunctionHandle) : CommandElabM Result := do
  let current : FunctionHandle :=
    { namespaceId := ⟨namespaceIndex⟩, functionId := ⟨functionIndex⟩ }
  if stack.contains current then
    return .unsupported
      s!"call to `{function}` remains inside one recursive SCC; V1 requires a mutual fixed point"
  let .structured root := declaration.body
    | return .unsupported "the function has no structured body"
  match checkExpr unit current.namespaceId ns #[] root with
  | .error reason => return .unsupported reason
  | .ok () => pure ()
  let typeParameterCount ← match checkGenericParameters unit current declaration with
    | .error reason => return .unsupported reason
    | .ok count => pure count
  let directCalls ← match collectCalls unit current.namespaceId ns #[] root with
    | .error reason => return .unsupported reason
    | .ok handles => pure handles
  /- A call of the function itself is the recursive call: the body is
  generated over the body of those calls and closed by its fixed point.
  A call into a function on the stack is mutual recursion, which has no
  generated family yet. -/
  let selfCall? := directCalls.find? (·.handle == current)
  if selfCall?.isSome && typeParameterCount != 0 then
    return .unsupported
      "a recursive generic function has no V1 denotation: the fixed point is generated for monomorphic functions"
  let mut callees : Array CalleeDenotation := #[]
  for directCall in directCalls do
    let handle := directCall.handle
    if handle == current then
      continue
    if stack.contains handle then
      return .unsupported
        "a direct call remains inside one mutually recursive SCC; V1 generates the fixed point of a single function only"
    let some calleeNs := unit.namespaces[handle.namespaceId.index]?
      | return .unsupported "a resolved callee namespace is out of range"
    let some calleeDeclaration := calleeNs.functions[handle.functionId.index]?
      | return .unsupported "a resolved callee declaration is out of range"
    let some calleeSpelling := functionSpelling? unit calleeDeclaration
      | return .unsupported "a resolved callee has no declared spelling"
    let generated ← match ← ensureDefinitionsInternal unit unitDef rootSegments
        (functionSegments rootSegments handle) calleeSpelling
        handle.namespaceId.index handle.functionId.index calleeNs
        calleeDeclaration (stack.push current) with
      | .unsupported reason =>
          return .unsupported s!"callee `{calleeSpelling}` has no V1 denotation: {reason}"
      | .generated generated => pure generated
    callees := callees.push {
      reference := directCall.reference, handle
      relationTerm := fun executable => do
        let relation ← mkAppM generated.relation #[executable]
        if generated.typeParameterCount != 0 then
          pure relation
        else
          let pairType ← mkAppM ``Prod #[mkConst ``TypeId, mkConst ``TypeId]
          let mapType ← mkAppM ``Array #[pairType]
          withLocalDeclD `typeInstantiation mapType fun typeInstantiation =>
            mkLambdaFVars #[typeInstantiation] relation
      generated := some generated }
  let namespaceDef := namespaceName definitionSegments
  let declarationConst := declarationName definitionSegments function
  let shapeConst := shapeName definitionSegments function
  let body := bodyName definitionSegments function
  let openBody := openBodyName definitionSegments function
  let openBodyAgreementDeclaration := openBodyAgreementName definitionSegments function
  let relation := relationName definitionSegments function
  let relationAgreementDeclaration :=
    relationAgreementName definitionSegments function
  let denotation := functionName definitionSegments function
  let agreementDeclaration := agreementName definitionSegments function
  /- Definitions added through `addDecl` use their exact names, while theorem
  commands are scoped by the surrounding Lean namespace.  Track both names:
  the declaration spelling for emitted commands and the actual environment
  name for idempotence and downstream references. -/
  let currentNamespace ← getCurrNamespace
  let relationAgreement := currentNamespace ++ relationAgreementDeclaration
  let openBodyAgreement := currentNamespace ++ openBodyAgreementDeclaration
  let agreement := currentNamespace ++ agreementDeclaration
  let env ← getEnv
  if !env.contains unitDef then
    liftTermElabM do
      addAbbrev unitDef (mkConst ``ValidatedUnit) (toExpr unit)
  let env ← getEnv
  if !env.contains namespaceDef then
    liftTermElabM do
      -- Share the namespace already quoted in the semantic unit. A second
      -- literal copy makes every function's namespace-agreement reflexivity
      -- compare every arena and declaration in the namespace again.
      let value ← Lean.Elab.Term.elabTerm
        (← `(($(mkIdent unitDef):term).namespaces[$(Syntax.mkNatLit namespaceIndex)]!))
        (some (mkConst ``ValidatedNamespace))
      Lean.Elab.Term.synthesizeSyntheticMVarsNoPostponing
      addAbbrev namespaceDef (mkConst ``ValidatedNamespace) (← instantiateMVars value)
  liftTermElabM do
    ensureArenaIndex unitDef namespaceDef namespaceIndex "expressions"
      (← `(($(mkIdent namespaceDef):term).expressions)) ns.expressions
    ensureArenaIndex unitDef namespaceDef namespaceIndex "places"
      (← `(($(mkIdent namespaceDef):term).places)) ns.places
  let env ← getEnv
  if !env.contains declarationConst then
    liftTermElabM do
      let declarationType ← inferType (toExpr declaration)
      addAbbrev declarationConst declarationType (toExpr declaration)
  let env ← getEnv
  if !env.contains shapeConst then
    liftTermElabM do
      let shape := FunctionShape.ofDeclaration declaration
      addAbbrev shapeConst (mkConst ``FunctionShape) (toExpr shape)
  let env ← getEnv
  if !env.contains body then
    liftTermElabM do
      match selfCall? with
      | none =>
          withLocalDeclD `executable (mkConst ``ExecutableUnit) fun executable => do
            let value ← emitExpr unit ns current.namespaceId
              (mkConst namespaceDef) executable callees root
            let value ← mkLambdaFVars #[executable] value
            addAbbrev body (← inferType value) value
      | some selfCall =>
          /- The open body: the recursive call denotes as a call of the
          relation of `self`, the body of those calls. -/
          withLocalDeclD `executable (mkConst ``ExecutableUnit) fun executable =>
            withLocalDeclD `self
                (mkConst ``LeanerIR.Proofs.Denotation.ExprDenotation) fun self => do
              let selfRelation ← mkAppM
                ``LeanerIR.Proofs.Denotation.nativeFunctionRelation
                #[executable, mkConst shapeConst, self]
              let value ← emitExpr unit ns current.namespaceId
                (mkConst namespaceDef) executable
                (callees.push {
                  reference := selfCall.reference, handle := current
                  relationTerm := fun _ => pure selfRelation
                  generated := none })
                root
              let value ← mkLambdaFVars #[executable, self] value
              addAbbrev openBody (← inferType value) value
          withLocalDeclD `executable (mkConst ``ExecutableUnit) fun executable => do
            let value ← mkAppM ``LeanerIR.Proofs.Denotation.fixBody
              #[← mkAppM openBody #[executable]]
            let value ← mkLambdaFVars #[executable] value
            addAbbrev body (← inferType value) value
  /- The body as a literal tree, beside the combinator term: the same
  traversal with the tree's constructors, and the reflexivity that its
  denotation is the body.  The normalization route computes over it. -/
  let tree := treeName definitionSegments function
  let treeDenotesDeclaration := treeDenotesName definitionSegments function
  let treeDenotes := currentNamespace ++ treeDenotesDeclaration
  let env ← getEnv
  if !env.contains tree then
    liftTermElabM do
      match selfCall? with
      | none =>
          withLocalDeclD `executable (mkConst ``ExecutableUnit) fun executable => do
            let value ← emitTree unit ns current.namespaceId
              (mkConst namespaceDef) executable callees root
            let value ← mkLambdaFVars #[executable] value
            addAbbrev tree (← inferType value) value
      | some selfCall =>
          withLocalDeclD `executable (mkConst ``ExecutableUnit) fun executable =>
            withLocalDeclD `self
                (mkConst ``LeanerIR.Proofs.Denotation.ExprDenotation) fun self => do
              let selfRelation ← mkAppM
                ``LeanerIR.Proofs.Denotation.nativeFunctionRelation
                #[executable, mkConst shapeConst, self]
              let value ← emitTree unit ns current.namespaceId
                (mkConst namespaceDef) executable
                (callees.push {
                  reference := selfCall.reference, handle := current
                  relationTerm := fun _ => pure selfRelation
                  generated := none })
                root
              let value ← mkLambdaFVars #[executable, self] value
              addAbbrev tree (← inferType value) value
  let env ← getEnv
  if !env.contains treeDenotes then
    let command ← match selfCall? with
      | none =>
          `(command|
            theorem $(mkIdent treeDenotesDeclaration)
                (executable : LeanerIR.Validation.ExecutableUnit) :
                ($(mkIdent tree) executable).denote = $(mkIdent body) executable := rfl)
      | some _ =>
          `(command|
            theorem $(mkIdent treeDenotesDeclaration)
                (executable : LeanerIR.Validation.ExecutableUnit)
                (self : LeanerIR.Proofs.Denotation.ExprDenotation) :
                ($(mkIdent tree) executable self).denote =
                  $(mkIdent openBody) executable self := rfl)
    elabCommand command
  let env ← getEnv
  if !env.contains relation then
    liftTermElabM do
      withLocalDeclD `executable (mkConst ``ExecutableUnit) fun executable => do
        let nativeBody ← mkAppM body #[executable]
        let value ← if typeParameterCount == 0 then
          mkAppM ``LeanerIR.Proofs.Denotation.nativeFunctionRelation
            #[executable, mkConst shapeConst, nativeBody]
        else do
          let pairType ← mkAppM ``Prod #[mkConst ``TypeId, mkConst ``TypeId]
          let mapType ← mkAppM ``Array #[pairType]
          withLocalDeclD `typeInstantiation mapType fun typeInstantiation => do
            let relation ← mkAppM
              ``LeanerIR.Proofs.Denotation.nativeFunctionRelationAt
              #[executable, mkConst shapeConst, typeInstantiation, nativeBody]
            mkLambdaFVars #[typeInstantiation] relation
        let value ← mkLambdaFVars #[executable] value
        addAbbrev relation (← inferType value) value
  let env ← getEnv
  if !env.contains denotation then
    liftTermElabM do
      let argumentsType ← mkAppM ``Array #[mkConst ``RuntimeValue]
      withLocalDeclD `executable (mkConst ``ExecutableUnit) fun executable => do
        let nativeBody ← mkAppM body #[executable]
        if typeParameterCount == 0 then
          withLocalDeclD `arguments argumentsType fun arguments => do
            let value ← mkAppM ``LeanerIR.Proofs.Denotation.nativeFunction
              #[executable, mkConst shapeConst, nativeBody, arguments]
            let value ← mkLambdaFVars #[executable, arguments] value
            addAbbrev denotation (← inferType value) value
        else do
          let pairType ← mkAppM ``Prod #[mkConst ``TypeId, mkConst ``TypeId]
          let mapType ← mkAppM ``Array #[pairType]
          withLocalDeclD `typeInstantiation mapType fun typeInstantiation =>
            withLocalDeclD `arguments argumentsType fun arguments => do
              let value ← mkAppM ``LeanerIR.Proofs.Denotation.nativeFunctionAt
                #[executable, mkConst shapeConst, typeInstantiation, nativeBody, arguments]
              let value ← mkLambdaFVars #[executable, typeInstantiation, arguments] value
              addAbbrev denotation (← inferType value) value
  let namespaceIndexSyntax := Syntax.mkNatLit namespaceIndex
  let functionIndexSyntax := Syntax.mkNatLit functionIndex
  let mut calleeAgreementSetup : Lean.TSyntax `tactic ← `(tactic| skip)
  /- The recursive call's agreement is `self_agrees`, stated by the
  theorem itself; its resolution fact is emitted like every callee's. -/
  let setupCallees := match selfCall? with
    | some selfCall => callees.push {
        reference := selfCall.reference, handle := current
        relationTerm := fun _ => throwError "the recursive call is not emitted here"
        generated := none }
    | none => callees
  for (callee, index) in setupCallees.zipIdx do
    let factName := mkIdent (Name.mkSimple s!"callee_agrees_{index}")
    let resolveFactName := mkIdent (Name.mkSimple s!"callee_resolves_{index}")
    if let some generated := callee.generated then
      let theoremName : Lean.TSyntax `term :=
        ⟨(mkIdent generated.relationAgreement).raw⟩
      let closedProof ← if generated.typeParameterCount == 0 then
        `(term| $theoremName (by assumption))
      else
        let carrierIdent := mkIdent `Carrier
        `(term| fun typeInstantiation =>
          $theoremName ($carrierIdent := fun _ => PUnit)
            typeInstantiation (by assumption))
      /- Under the recursive body's oracle, every other callee keeps its
      closed agreement. -/
      let proof ← if selfCall?.isSome then
        `(term| LeanerIR.Proofs.Denotation.FunctionDenotation.agreesWith_oracle_other
          (LeanerIR.Proofs.Denotation.nativeFunctionRelation executable
            $(mkIdent shapeConst) self)
          (handle := ⟨⟨$namespaceIndexSyntax⟩, ⟨$functionIndexSyntax⟩⟩)
          (by decide) $closedProof)
      else pure closedProof
      calleeAgreementSetup ←
        `(tactic| ($calleeAgreementSetup; have $factName := $proof))
    let sourceNamespaceSyntax := Syntax.mkNatLit namespaceIndex
    let referenceNamespaceSyntax :=
      Syntax.mkNatLit callee.reference.namespaceId.index
    let referenceNameSyntax := Syntax.mkNatLit callee.reference.name.index
    let handleNamespaceSyntax := Syntax.mkNatLit callee.handle.namespaceId.index
    let handleFunctionSyntax := Syntax.mkNatLit callee.handle.functionId.index
    calleeAgreementSetup ← `(tactic|
      ($calleeAgreementSetup;
       have $resolveFactName :
           LeanerIR.SemanticOperations.resolveFunction? executable.unit
              ⟨$sourceNamespaceSyntax⟩
              { namespaceId := ⟨$referenceNamespaceSyntax⟩,
                name := ⟨$referenceNameSyntax⟩ } =
            some { namespaceId := ⟨$handleNamespaceSyntax⟩,
                   functionId := ⟨$handleFunctionSyntax⟩ } := by
         rw [unit_eq]
         rfl))
  let env ← getEnv
  if selfCall?.isSome && !env.contains openBodyAgreement then
    /- The open body agrees with the open semantics under the oracle that
    answers this function with the relation of `self`, for every `self`:
    the ordinary combinator lemmas, with the recursive call agreeing by
    construction. -/
    let rootSyntax := Syntax.mkNatLit root.index
    let command ←
      `(command|
        set_option maxRecDepth 100000 in
        theorem $(mkIdent openBodyAgreementDeclaration)
            {executable : LeanerIR.Validation.ExecutableUnit}
            (unit_eq : executable.unit = $(mkIdent unitDef))
            (self : LeanerIR.Proofs.Denotation.ExprDenotation) :
            LeanerIR.Proofs.Denotation.ExprDenotation.AgreesWith executable
              (LeanerIR.Proofs.Denotation.oracle executable
                ⟨⟨$namespaceIndexSyntax⟩, ⟨$functionIndexSyntax⟩⟩
                (LeanerIR.Proofs.Denotation.nativeFunctionRelation executable
                  $(mkIdent shapeConst) self))
              ⟨$namespaceIndexSyntax⟩ ⟨$rootSyntax⟩
              ($(mkIdent openBody) executable self) := by
          have namespace_eq : executable.unit.namespaces[$namespaceIndexSyntax]? =
              some $(mkIdent namespaceDef) := by
            rw [unit_eq]
            rfl
          have self_agrees :=
            LeanerIR.Proofs.Denotation.FunctionDenotation.agreesWith_oracle_self
              executable ⟨⟨$namespaceIndexSyntax⟩, ⟨$functionIndexSyntax⟩⟩
              (LeanerIR.Proofs.Denotation.nativeFunctionRelation executable
                $(mkIdent shapeConst) self)
          $calleeAgreementSetup
          simp only [$(mkIdent openBody):term]
          leaner_denotation_agree namespace_eq unit_eq)
    elabCommand command
  let env ← getEnv
  if !env.contains relationAgreement then
    let command ← if selfCall?.isSome then
      `(command|
        set_option maxRecDepth 100000 in
        theorem $(mkIdent relationAgreementDeclaration)
            {executable : LeanerIR.Validation.ExecutableUnit}
            (unit_eq : executable.unit = $(mkIdent unitDef)) :
            LeanerIR.Proofs.Denotation.FunctionDenotation.Agrees executable
              ⟨⟨$namespaceIndexSyntax⟩, ⟨$functionIndexSyntax⟩⟩
              ($(mkIdent relation) executable) := by
          have namespace_eq : executable.unit.namespaces[$namespaceIndexSyntax]? =
              some $(mkIdent namespaceDef) := by
            rw [unit_eq]
            rfl
          simp only [$(mkIdent relation):term, $(mkIdent body):term]
          exact LeanerIR.Proofs.Denotation.fixBody_agrees namespace_eq (by rfl) (by rfl)
            (by rfl) (fun self => $(mkIdent openBodyAgreement) unit_eq self))
    else if typeParameterCount == 0 then
      `(command|
        set_option maxRecDepth 100000 in
        theorem $(mkIdent relationAgreementDeclaration)
            {executable : LeanerIR.Validation.ExecutableUnit}
            (unit_eq : executable.unit = $(mkIdent unitDef)) :
            LeanerIR.Proofs.Denotation.FunctionDenotation.Agrees executable
              ⟨⟨$namespaceIndexSyntax⟩, ⟨$functionIndexSyntax⟩⟩
              ($(mkIdent relation) executable) := by
          have namespace_eq : executable.unit.namespaces[$namespaceIndexSyntax]? =
              some $(mkIdent namespaceDef) := by
            rw [unit_eq]
            rfl
          $calleeAgreementSetup
          leaner_agreement_tick "relation start"
          simp only [$(mkIdent relation):term]
          leaner_agreement_tick "relation unfolded"
          apply LeanerIR.Proofs.Denotation.nativeFunctionRelation_agrees
            namespace_eq (by rfl) (by rfl) (by rfl)
          leaner_agreement_tick "function agreed"
          simp only [$(mkIdent body):term]
          leaner_denotation_agree namespace_eq unit_eq)
    else
      let countSyntax := Syntax.mkNatLit typeParameterCount
      let carrierIdent := mkIdent `Carrier
      let carrierInhabitedIdent := mkIdent `carrier_inhabited
      `(command|
        set_option maxRecDepth 100000 in
        theorem $(mkIdent relationAgreementDeclaration)
            {$carrierIdent : Fin $countSyntax → Type}
            [$carrierInhabitedIdent : ∀ index, Nonempty ($carrierIdent index)]
            {executable : LeanerIR.Validation.ExecutableUnit}
            (typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId))
            (unit_eq : executable.unit = $(mkIdent unitDef)) :
            LeanerIR.Proofs.Denotation.FunctionDenotation.AgreesAt executable
              ⟨⟨$namespaceIndexSyntax⟩, ⟨$functionIndexSyntax⟩⟩
              typeInstantiation
              ($(mkIdent relation) executable typeInstantiation) := by
          have namespace_eq : executable.unit.namespaces[$namespaceIndexSyntax]? =
              some $(mkIdent namespaceDef) := by
            rw [unit_eq]
            rfl
          $calleeAgreementSetup
          simp only [$(mkIdent relation):term]
          apply LeanerIR.Proofs.Denotation.nativeFunctionRelationAt_agrees
            typeInstantiation namespace_eq (by rfl) (by rfl) (by rfl)
          simp only [$(mkIdent body):term]
          leaner_denotation_agree namespace_eq unit_eq)
    elabCommand command
  let env ← getEnv
  if !env.contains agreement then
    let command ← if typeParameterCount == 0 then
      `(command|
        set_option maxRecDepth 100000 in
        theorem $(mkIdent agreementDeclaration)
            {executable : LeanerIR.Validation.ExecutableUnit}
            (unit_eq : executable.unit = $(mkIdent unitDef)) :
            ∀ arguments,
              LeanerIR.Proofs.Spec.Equiv
                ($(mkIdent denotation) executable arguments)
                (LeanerIR.Proofs.functionSpec executable
                  ⟨⟨$namespaceIndexSyntax⟩,
                    ⟨$functionIndexSyntax⟩⟩ arguments) := by
          intro arguments
          leaner_agreement_tick "spec transport"
          simpa only [$(mkIdent denotation):term, $(mkIdent relation):term,
            LeanerIR.Proofs.Denotation.nativeFunction] using
            (LeanerIR.Proofs.Denotation.nativeFunction_agrees_of_relation
              ($(mkIdent relationAgreement) unit_eq) arguments))
    else
      let countSyntax := Syntax.mkNatLit typeParameterCount
      let carrierIdent := mkIdent `Carrier
      let carrierInhabitedIdent := mkIdent `carrier_inhabited
      `(command|
        set_option maxRecDepth 100000 in
        theorem $(mkIdent agreementDeclaration)
            {$carrierIdent : Fin $countSyntax → Type}
            [$carrierInhabitedIdent : ∀ index, Nonempty ($carrierIdent index)]
            {executable : LeanerIR.Validation.ExecutableUnit}
            (unit_eq : executable.unit = $(mkIdent unitDef)) :
            ∀ arguments,
              LeanerIR.Proofs.Spec.Equiv
                ($(mkIdent denotation) executable #[] arguments)
                (LeanerIR.Proofs.functionSpec executable
                  ⟨⟨$namespaceIndexSyntax⟩,
                    ⟨$functionIndexSyntax⟩⟩ arguments) := by
          intro arguments
          simpa only [$(mkIdent denotation):term, $(mkIdent relation):term,
            LeanerIR.Proofs.Denotation.nativeFunction] using
            (LeanerIR.Proofs.Denotation.nativeFunction_agrees_of_relation
              ($(mkIdent relationAgreement)
                ($carrierIdent := $carrierIdent) #[] unit_eq) arguments))
    elabCommand command
  return .generated {
    unitDef, namespaceDef, declaration := declarationConst, shape := shapeConst, body
    relation, relationAgreement, denotation, agreement, typeParameterCount
    requiresUnitEquality := true
    openBody := if selfCall?.isSome then some openBody else none
    tree, treeDenotes }

/-- Generate the native denotation and exact agreement theorem for a V1
function, or return the reason the existing deep route must be used. -/
def ensureDefinitions (unit : ValidatedUnit) (namespaceSegments : Array String)
    (function : String)
    (namespaceIndex functionIndex : Nat) (ns : ValidatedNamespace)
    (declaration : FunctionDecl FunctionBody)
    (preparedUnitDef? : Option Name := none) : CommandElabM Result := do
  let unitDef := preparedUnitDef?.getD (unitName namespaceSegments)
  let handle : FunctionHandle :=
    { namespaceId := ⟨namespaceIndex⟩, functionId := ⟨functionIndex⟩ }
  ensureDefinitionsInternal unit unitDef namespaceSegments
    (functionSegments namespaceSegments handle) function namespaceIndex
    functionIndex ns declaration #[]

end LeanerLang.Denotation
