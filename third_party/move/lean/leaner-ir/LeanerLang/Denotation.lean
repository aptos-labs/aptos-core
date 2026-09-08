-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Quote
import LeanerIR.Proofs.Denotation
import LeanerIR.Proofs.Recursion

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

deriving instance ToExpr for LeanerIR.StructHandle
deriving instance ToExpr for LeanerIR.Proofs.Denotation.ResourceLocation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.BorrowLocation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.GlobalLocationOperation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.LocalLocation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.LocalLocationOperation
deriving instance ToExpr for LeanerIR.SemanticOperations.NominalFieldStep
deriving instance ToExpr for LeanerIR.Proofs.Denotation.DerefLocalBorrowOperation
deriving instance ToExpr for LeanerIR.Proofs.Denotation.NominalConstructor
deriving instance ToExpr for LeanerIR.Proofs.Denotation.NominalFieldLocation
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
            (genericArgumentMentionsParameter unit parameter) then
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
  | field (location : LeanerIR.Proofs.Denotation.NominalFieldLocation)
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
      let some index := handleFieldIndex? unit source none fieldName
        | throw s!"native projected borrow could not resolve field `{fieldName}`"
      return (localId, fields ++ [{ source, variant := none, index }])
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

private def lowerPrimitive (ns : ValidatedNamespace) (resultType : TypeId)
    (operation : PrimitiveOperation) : Except String LoweredOperation := do
  let some resolved := ns.tables.types[resultType.index]?
    | throw s!"primitive result type {resultType.index} is out of range"
  if let .integer .pointer _ := resolved then
    throw "V1 native lowering requires a source-independent integer width"
  let lowered := match operation with
    | .tuple => some .tuple
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
      lowerDerefLocalBorrow unit ns resultType site kind place
  | .call _ => .error s!"call form at expression {site.index} is not in V1"
  | .assert => .error s!"assert at expression {site.index} is not in V1"
  | .write _ | .drop _ =>
      .error s!"place operation at expression {site.index} is not in V1"
  | .profile .. => .error s!"profile operation at expression {site.index} is not in V1"
  | .specification _ =>
      .error s!"logical operation reached executable expression {site.index}"

/-- The runtime value of a constant whose initializer is a literal: the
big-step rule evaluates the initializer in an empty frame, which for a
literal is that value, so the constant denotes as `value`. -/
private def literalConstant? (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (reference : QualifiedRef) : Option RuntimeValue := do
  let handle ← resolveConstant? unit namespaceId reference
  let targetNs ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← targetNs.constants[handle.constantId]?
  let initializer ← targetNs.expressions[declaration.value.index]?
  match initializer.kind with
  | .value literal _ => constValue? literal
  | _ => none

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
      unless (literalConstant? unit namespaceId reference).isSome do
        throw s!"constant expression {id.index} has no literal initializer; only literal constants are in V1"
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
      let some (.localVar _) := ns.places[place.index]?
        | throw s!"assignment at expression {id.index} is not to a closed local"
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

private partial def emitExpr (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (namespaceId : NamespaceId)
    (namespaceTerm executable : Lean.Expr)
    (callees : Array CalleeDenotation) (id : ExprId) : MetaM Lean.Expr := do
  let some expression := ns.expressions[id.index]?
    | throwError "expression {id.index} is out of range during denotation generation"
  match expression.kind with
  | .value literal _ =>
      let some runtimeValue := constValue? literal
        | throwError "literal at expression {id.index} has no runtime representation"
      mkAppM ``LeanerIR.Proofs.Denotation.value #[toExpr runtimeValue]
  | .localVar localId =>
      mkAppM ``LeanerIR.Proofs.Denotation.localVar #[toExpr localId]
  | .constant reference =>
      let some runtimeValue := literalConstant? unit namespaceId reference
        | throwError "constant expression {id.index} has no literal initializer"
      mkAppM ``LeanerIR.Proofs.Denotation.value #[toExpr runtimeValue]
  | .operation operation instantiations arguments _ =>
      let operands ←
        emitValues unit ns namespaceId namespaceTerm executable callees arguments
      let lowered ← match lowerOperation unit namespaceId ns expression.typeId id
          operation instantiations with
        | .ok lowered => pure lowered
        | .error reason => throwError reason
      match lowered with
      | .primitive operation =>
          mkAppM ``LeanerIR.Proofs.Denotation.nativePrimitiveOperation
            #[toExpr operation, operands]
      | .global operation =>
          mkAppM ``LeanerIR.Proofs.Denotation.nativeGlobalOperation
            #[toExpr operation, operands]
      | .local operation =>
          mkAppM ``LeanerIR.Proofs.Denotation.nativeLocalOperation
            #[toExpr operation, operands]
      | .derefLocalBorrow operation =>
          mkAppM ``LeanerIR.Proofs.Denotation.nativeDerefLocalBorrowOperation
            #[toExpr operation, operands]
      | .field location =>
          let evaluate ← mkAppM
            ``LeanerIR.Proofs.Denotation.NominalFieldLocation.evaluateSelect?
            #[toExpr location]
          mkAppM ``LeanerIR.Proofs.Denotation.nativeOperation #[evaluate, operands]
      | .reference operation =>
          mkAppM ``LeanerIR.Proofs.Denotation.nativeReferenceOperation
            #[toExpr operation, operands]
      | .constructor constructor =>
          let evaluate ← mkAppM
            ``LeanerIR.Proofs.Denotation.NominalConstructor.evaluate?
            #[toExpr constructor]
          mkAppM ``LeanerIR.Proofs.Denotation.nativeOperation #[evaluate, operands]
      | .function reference =>
          let some handle := resolveFunction? unit namespaceId reference
            | throwError "direct call escaped denotation resolution"
          let some callee := callees.find? (fun candidate => candidate.handle == handle)
            | throwError "direct call escaped denotation dependency generation"
          let lexical := certificateLoanId? unit namespaceId id
          mkAppM ``LeanerIR.Proofs.Denotation.nativeCall
            #[toExpr handle, toExpr lexical, ← callee.relationTerm executable, operands]
  | .ifElse condition thenBranch elseBranch =>
      let conditionTerm ←
        emitExpr unit ns namespaceId namespaceTerm executable callees condition
      let thenTerm ←
        emitExpr unit ns namespaceId namespaceTerm executable callees thenBranch
      match elseBranch with
      | none =>
          let elseTerm ← mkAppOptM ``Option.none
            #[some (mkConst ``LeanerIR.Proofs.Denotation.ExprDenotation)]
          mkAppM ``LeanerIR.Proofs.Denotation.nativeBranch
            #[conditionTerm, thenTerm, elseTerm]
      | some elseBranch =>
          let elseTerm ←
            emitExpr unit ns namespaceId namespaceTerm executable callees elseBranch
          let elseTerm ← mkAppM ``Option.some #[elseTerm]
          mkAppM ``LeanerIR.Proofs.Denotation.nativeBranch
            #[conditionTerm, thenTerm, elseTerm]
  | .return_ values =>
      mkAppM ``LeanerIR.Proofs.Denotation.nativeReturn
        #[← emitValues unit ns namespaceId namespaceTerm executable callees values]
  | .throw_ kind arguments =>
      mkAppM ``LeanerIR.Proofs.Denotation.nativeThrow
        #[toExpr kind,
          ← emitValues unit ns namespaceId namespaceTerm executable callees arguments]
  | .loop _ body =>
      mkAppM ``LeanerIR.Proofs.Denotation.nativeLoop
        #[toExpr id,
          ← emitExpr unit ns namespaceId namespaceTerm executable callees body]
  | .break_ nest none =>
      mkAppM ``LeanerIR.Proofs.Denotation.nativeBreak
        #[toExpr nest, ← mkAppOptM ``Option.none
          #[some (mkConst ``LeanerIR.Proofs.Denotation.ExprDenotation)]]
  | .continue_ nest =>
      mkAppM ``LeanerIR.Proofs.Denotation.nativeContinue #[toExpr nest]
  | .assign place child =>
      let some (.localVar localId) := ns.places[place.index]?
        | throwError "assignment escaped closed-local native lowering"
      mkAppM ``LeanerIR.Proofs.Denotation.nativeAssignLocal
        #[toExpr localId,
          ← emitExpr unit ns namespaceId namespaceTerm executable callees child]
  | .spec _ =>
      pure (mkConst ``LeanerIR.Proofs.Denotation.nativeSpec)
  | .block statements result =>
      let statementTerm ←
        emitStatements unit ns namespaceId namespaceTerm executable callees statements
      match result with
      | none => mkAppM ``LeanerIR.Proofs.Denotation.blockUnit #[statementTerm]
      | some result =>
          mkAppM ``LeanerIR.Proofs.Denotation.blockResult
            #[statementTerm,
              ← emitExpr unit ns namespaceId namespaceTerm executable callees result]
  | .letDecl pattern initializer body =>
      let body ← emitExpr unit ns namespaceId namespaceTerm executable callees body
      match initializer with
      | none => mkAppM ``LeanerIR.Proofs.Denotation.letNoValue #[body]
      | some initializer =>
          let some binder := lowerPattern? unit ns pattern
            | throwError "pattern {pattern.index} escaped native lowering"
          mkAppM ``LeanerIR.Proofs.Denotation.letNativeValue
            #[toExpr binder,
              ← emitExpr unit ns namespaceId namespaceTerm executable callees initializer,
              body]
  | _ => throwError "unsupported expression escaped the denotation precheck"
where
  emitValues (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (namespaceId : NamespaceId)
      (namespaceTerm executable : Lean.Expr)
      (callees : Array CalleeDenotation) (ids : Array ExprId) :
      MetaM Lean.Expr := do
    let mut tail := mkConst ``LeanerIR.Proofs.Denotation.valuesNil
    for id in ids.reverse do
      tail ← mkAppM ``LeanerIR.Proofs.Denotation.valuesCons
        #[← emitExpr unit ns namespaceId namespaceTerm executable callees id, tail]
    return tail
  emitStatements (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (namespaceId : NamespaceId)
      (namespaceTerm executable : Lean.Expr)
      (callees : Array CalleeDenotation) (ids : Array ExprId) :
      MetaM Lean.Expr := do
    let mut tail := mkConst ``LeanerIR.Proofs.Denotation.statementsNil
    for id in ids.reverse do
      tail ← mkAppM ``LeanerIR.Proofs.Denotation.statementsCons
        #[← emitExpr unit ns namespaceId namespaceTerm executable callees id, tail]
    return tail

/-! ## Uniform agreement proof -/

/-- Close a generated body-agreement goal compositionally.  Every branch is
one application of a checked core lemma; `first` backtracks failed shapes,
and recursion follows only the native child terms already present in the
goal. -/
syntax "leaner_denotation_agree " ident ident : tactic

/-- Reuse the exact agreement theorem attached to a named generated callee
relation.  Generic V1 relations are instantiated with one abstract inhabited
carrier here; no representation-specific fact enters the proof. -/
syntax "leaner_denotation_reuse " ident : tactic

/-- Build the proof-only static path certificate for a generated local
reborrow.  Each constructor closes one literal arena lookup; recursion ends
at `deref (localVar _)`. -/
syntax "leaner_deref_local_path" : tactic

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
              intro actualVariant
              cases actualVariant <;> rfl))

elab_rules : tactic
  | `(tactic| leaner_denotation_reuse $unitEq:ident) => do
      let goal ← Lean.Elab.Tactic.getMainGoal
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

elab_rules : tactic
  | `(tactic| leaner_denotation_agree $namespaceEq:ident $unitEq:ident) => do
      /- A well-formed rewrite rule, not a bare identifier coerced to one:
      `rw` reads the rule's node kind. -/
      let unitRule ← `(Lean.Parser.Tactic.rwRule| $unitEq:ident)
      let unitSimp : TSyntax ``Lean.Parser.Tactic.simpLemma := ⟨unitEq.raw⟩
      let goal ← Lean.Elab.Tactic.getMainGoal
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
        /- A literal, or a constant whose initializer is one. -/
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            first
            | (apply LeanerIR.Proofs.Denotation.value_agrees
                (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
                first
                | rfl
                | simp [LeanerIR.SemanticOperations.constValue?])
            | (apply LeanerIR.Proofs.Denotation.constant_agrees
                (by first | exact $namespaceEq | (rw [$unitRule]; rfl))
               /- By name, and the names spliced without macro scopes so
               they meet the goal tags: the equations assign the lemma's
               data, and a blanket pass over every goal would also visit
               the assigned data goals, where its last alternative fails. -/
               case $(tag "expression_eq"):ident => rfl
               case $(tag "kind_eq"):ident => rfl
               case $(tag "resolve_eq"):ident => rw [$unitRule]; rfl
               case $(tag "target_namespace_eq"):ident => rw [$unitRule]; rfl
               case $(tag "declaration_eq"):ident => rfl
               case $(tag "initializer_eq"):ident => rfl
               case $(tag "initializer_kind_eq"):ident => rfl
               case $(tag "value_eq"):ident =>
                 first | rfl | simp [LeanerIR.SemanticOperations.constValue?])))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.localVar then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.local_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;> rfl)))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeReturn then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeReturn_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl))
              (by rfl) (by rfl)
              (by leaner_denotation_agree $namespaceEq $unitEq))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeThrow then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeThrow_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl))
              (by rfl) (by rfl)
              (by leaner_denotation_agree $namespaceEq $unitEq))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeLoop then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeLoop_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl))
              (by rfl) (by rfl)
              (by leaner_denotation_agree $namespaceEq $unitEq))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeBreak then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeBreakNone_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl))
              (by rfl) (by rfl))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeContinue then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeContinue_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl))
              (by rfl) (by rfl))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeAssignLocal then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeAssignLocal_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl))
              (by rfl) (by rfl) (by rfl)
              (by leaner_denotation_agree $namespaceEq $unitEq))))
        return
      if denotationHead? == some ``LeanerIR.Proofs.Denotation.nativeSpec then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeSpec_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl))
              (by rfl) (by rfl))))
        return
      let isNativeCall ← goal.withContext do
        let target ← Lean.instantiateMVars (← goal.getType)
        let some denotation := target.getAppArgs.back? | return false
        return denotation.getAppFn.isConstOf
          ``LeanerIR.Proofs.Denotation.nativeCall
      if isNativeCall then
        Lean.Elab.Tactic.evalTactic
          (← `(tactic|
            (apply LeanerIR.Proofs.Denotation.nativeCall_agrees
              (by exact $namespaceEq) (by rfl) (by rfl)
              (by leaner_denotation_agree $namespaceEq $unitEq)
              (by assumption)
              (by assumption)
              (by
                have unitEquality := $unitEq
                rw [unitEquality]
                rfl))))
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
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl))
              (by rfl) (by rfl) (by rfl)
              (by first
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.add_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.tuple_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.copyValue_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.moveValue_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedAdd_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.subtract_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedSubtract_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.multiply_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedMultiply_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.greater_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.lessEqual_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.greaterEqual_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.equal_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.notEqual_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.divide_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedDivide_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.modulo_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedModulo_evaluator_eq
                    <;> rfl)
                | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.less_evaluator_eq
                    <;> rfl))
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
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl))
              (by rfl) (by rfl) (by rfl)
              (by
                apply LeanerIR.Proofs.Denotation.DerefLocalBorrowOperation.evaluator_eq_path_of_unit
                  $unitEq (path := by leaner_deref_local_path)
                · rfl
                · decide
                · rfl
                · funext frame state place
                  first
                  | exact
                      (LeanerIR.SemanticOperations.borrowRuntimePlace?_immutable_at
                        _ _ _ _ _ _ _ _).symm
                  | exact
                      (LeanerIR.SemanticOperations.borrowRuntimePlace?_mutable_at
                        _ _ _ _ _ _ _ _
                        (by rfl)).symm)
              (by leaner_denotation_agree $namespaceEq $unitEq))))
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
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;> rfl)
        | (apply LeanerIR.Proofs.Denotation.local_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;> rfl)
        | (apply LeanerIR.Proofs.Denotation.nativePrimitiveOperation_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first
            | rfl
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.add_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.copyValue_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.moveValue_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedAdd_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.subtract_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedSubtract_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.multiply_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedMultiply_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.greater_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.lessEqual_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.greaterEqual_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.equal_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.notEqual_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.divide_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedDivide_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.modulo_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.checkedModulo_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.less_evaluator_eq
                <;> rfl)
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeGlobal_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first
            | rfl
            | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.borrow_evaluator_eq_of_unit
                  $unitEq <;> first
                | rfl
                | (funext frame state place
                   first
                   | exact
                       (LeanerIR.SemanticOperations.borrowRuntimePlace?_immutable_at
                         _ _ _ _ _ _ _ _).symm
                   | exact
                       (LeanerIR.SemanticOperations.borrowRuntimePlace?_mutable_at
                         _ _ _ _ _ _ _ _ (by rfl)).symm))
            | (simp only [$unitSimp]
               first
               | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.contains_evaluator_eq
                   <;> rfl)
               | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.borrow_evaluator_eq
                   <;> first
                   | rfl
                   | (funext frame state place
                      first
                      | exact
                          (LeanerIR.SemanticOperations.borrowRuntimePlace?_immutable_at
                            _ _ _ _ _ _ _ _).symm
                      | exact
                          (LeanerIR.SemanticOperations.borrowRuntimePlace?_mutable_at
                            _ _ _ _ _ _ _ _ (by rfl)).symm)
                   | (rw [$unitRule]; rfl))
               | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.take_evaluator_eq
                   <;> rfl)
               | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.publish_evaluator_eq
                   <;> rfl))
            | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.contains_evaluator_eq
                <;> first | rfl | (rw [$unitRule]; rfl))
            | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.borrow_evaluator_eq
                <;> first
                | rfl
                | (rw [$unitRule]; rfl)
                | (funext frame state place
                   first
                   | exact
                       (LeanerIR.SemanticOperations.borrowRuntimePlace?_immutable_at
                         _ _ _ _ _ _ _ _).symm
                   | exact
                       (LeanerIR.SemanticOperations.borrowRuntimePlace?_mutable_at
                         _ _ _ _ _ _ _ _ (by rw [$unitRule]; rfl)).symm))
            | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.take_evaluator_eq
                <;> first | rfl | (rw [$unitRule]; rfl))
            | (apply LeanerIR.Proofs.Denotation.GlobalLocationOperation.publish_evaluator_eq
                <;> first | rfl | (rw [$unitRule]; rfl))
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeLocal_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first
            | rfl
            | (apply LeanerIR.Proofs.Denotation.LocalLocationOperation.read_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.LocalLocationOperation.copy_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.LocalLocationOperation.move_evaluator_eq
                <;> rfl)
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeData_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first
            | rfl
            | (apply LeanerIR.Proofs.Denotation.NominalFieldLocation.select_evaluator_eq_of_unit
                 $unitEq <;> first
                 | rfl
                 | (intro actualVariant; cases actualVariant <;> rfl))
            | (apply LeanerIR.Proofs.Denotation.NominalFieldLocation.select_evaluator_eq
                <;> first
                | rfl
                | (rw [$unitRule]; rfl)
                | (intro actualVariant; cases actualVariant <;>
                    rw [$unitRule] <;> rfl))
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeReference_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first
            | rfl
            | (apply LeanerIR.Proofs.Denotation.ReferenceLocationOperation.dereference_evaluator_eq)
            | (apply LeanerIR.Proofs.Denotation.ReferenceLocationOperation.mutate_evaluator_eq)
            | (apply LeanerIR.Proofs.Denotation.ReferenceLocationOperation.freeze_evaluator_eq
                <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.ReferenceLocationOperation.endLoan_evaluator_eq)
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeConstructor_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first
            | rfl
            | (apply LeanerIR.Proofs.Denotation.NominalConstructor.evaluator_eq_of_unit
                 $unitEq <;> rfl)
            | (apply LeanerIR.Proofs.Denotation.NominalConstructor.evaluator_eq
                <;> first | rfl | (rw [$unitRule]; rfl))
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.primitive_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first | rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.global_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first | rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.data_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first | rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.reference_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first | rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.constructor_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first | rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeBranchUnit_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first | rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeBranchElse_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first | rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeReturn_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first | rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeThrow_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first | rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.blockUnit_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first | rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.blockResult_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first | rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.letNoValue_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first | rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.letNativeValue_agrees_of_unit $unitEq
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first
            | rfl
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.letValue_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl)) <;>
            first | rfl | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.call_agrees
              (by first | exact $namespaceEq | (rw [$unitRule]; rfl))
              (by rfl) (by rfl) (by rw [$unitRule]; rfl)
              (loan_eq := by
                have unitEquality := $unitEq
                rw [unitEquality]
                rfl)
            · leaner_denotation_agree $namespaceEq $unitEq
            · assumption)
        | (apply LeanerIR.Proofs.Denotation.functionRelation_agrees <;>
            first
            | rfl
            | (rw [$unitRule]; rfl)
            | leaner_denotation_agree $namespaceEq $unitEq)
        | (apply LeanerIR.Proofs.Denotation.nativeFunctionRelation_agrees <;>
            first
            | rfl
            | (rw [$unitRule]; rfl)
            | leaner_denotation_agree $namespaceEq $unitEq)
        | fail "no denotation agreement rule applies")
      evalTactic script

/-! ## Declaration materialization -/

private def addAbbrev (name : Name) (type value : Lean.Expr) : TermElabM Unit := do
  addDecl (.defnDecl {
    name, levelParams := [], type, value, hints := .abbrev, safety := .safe })
  enableRealizationsForConst name

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
      relationTerm := fun executable => mkAppM generated.relation #[executable]
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
      addAbbrev namespaceDef (mkConst ``ValidatedNamespace) (toExpr ns)
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
  let env ← getEnv
  if !env.contains relation then
    liftTermElabM do
      withLocalDeclD `executable (mkConst ``ExecutableUnit) fun executable => do
        let nativeBody ← mkAppM body #[executable]
        let value ← mkAppM ``LeanerIR.Proofs.Denotation.nativeFunctionRelation
          #[executable, mkConst shapeConst, nativeBody]
        let value ← mkLambdaFVars #[executable] value
        addAbbrev relation (← inferType value) value
  let env ← getEnv
  if !env.contains denotation then
    liftTermElabM do
      let argumentsType ← mkAppM ``Array #[mkConst ``RuntimeValue]
      withLocalDeclD `executable (mkConst ``ExecutableUnit) fun executable =>
        withLocalDeclD `arguments argumentsType fun arguments => do
          let nativeBody ← mkAppM body #[executable]
          let value ← mkAppM ``LeanerIR.Proofs.Denotation.nativeFunction
            #[executable, mkConst shapeConst, nativeBody, arguments]
          let value ← mkLambdaFVars #[executable, arguments] value
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
        `(term| $theoremName ($carrierIdent := fun _ => PUnit) (by assumption))
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
          simp only [$(mkIdent relation):term]
          apply LeanerIR.Proofs.Denotation.nativeFunctionRelation_agrees
            namespace_eq (by rfl) (by rfl) (by rfl)
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
            (unit_eq : executable.unit = $(mkIdent unitDef)) :
            LeanerIR.Proofs.Denotation.FunctionDenotation.Agrees executable
              ⟨⟨$namespaceIndexSyntax⟩, ⟨$functionIndexSyntax⟩⟩
              ($(mkIdent relation) executable) := by
          have namespace_eq : executable.unit.namespaces[$namespaceIndexSyntax]? =
              some $(mkIdent namespaceDef) := by
            rw [unit_eq]
            rfl
          $calleeAgreementSetup
          simp only [$(mkIdent relation):term]
          apply LeanerIR.Proofs.Denotation.nativeFunctionRelation_agrees
            namespace_eq (by rfl) (by rfl) (by rfl)
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
          simpa only [$(mkIdent denotation):term, $(mkIdent relation):term] using
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
                ($(mkIdent denotation) executable arguments)
                (LeanerIR.Proofs.functionSpec executable
                  ⟨⟨$namespaceIndexSyntax⟩,
                    ⟨$functionIndexSyntax⟩⟩ arguments) := by
          intro arguments
          simpa only [$(mkIdent denotation):term, $(mkIdent relation):term] using
            (LeanerIR.Proofs.Denotation.nativeFunction_agrees_of_relation
              ($(mkIdent relationAgreement)
                ($carrierIdent := $carrierIdent) unit_eq) arguments))
    elabCommand command
  return .generated {
    unitDef, namespaceDef, declaration := declarationConst, shape := shapeConst, body
    relation, relationAgreement, denotation, agreement, typeParameterCount
    requiresUnitEquality := true
    openBody := if selfCall?.isSome then some openBody else none }

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
