-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Meaning
import LeanerIR.Proofs.SimpAttrs

/-!
# Native shallow denotations for structured LIR

The definitions in this file are the proof-facing control semantics.  A
frontend-generated function definition is a tree of these combinators; it
does not inspect an expression arena.  `BigStep` remains authoritative, and
the agreement lemmas below connect each native combinator to the
corresponding deep rule.

V1 deliberately covers straight-line code.  Loops, matches, and calls within
one recursive SCC remain on the existing calculated-WP route until their
explicit fixed-point combinators land.
-/

namespace LeanerIR.Proofs
namespace Denotation

open LeanerIR.Validation
open LeanerIR.BigStep
open LeanerIR.SemanticOperations

/-- Native relation denoted by one expression.  The expression arena and its
node identifier do not occur in this type. -/
abbrev ExprDenotation := RuntimeFrame → RuntimeState →
  RuntimeFrame → RuntimeState → Control → Prop

/-- Native relation denoted by a left-to-right operand row. -/
abbrev ValuesDenotation := RuntimeFrame → RuntimeState → ValuesResult → Prop

/-- Native relation denoted by a left-to-right statement row. -/
abbrev StatementsDenotation :=
  RuntimeFrame → RuntimeState → StatementsResult → Prop

/-- Native relation denoted by a whole function.  Unlike `Spec`, this keeps
the final state of a throwing invocation because a caller must apply pending
write-backs before propagating the throw. -/
abbrev FunctionDenotation := RuntimeState → Array RuntimeValue →
  RuntimeState → Outcome → Prop

/-- Exact agreement of a native expression relation with one deep node,
under a callee oracle: what a body means before its call graph is tied.
The closed notions below fix the oracle to the closed semantics; a
recursive function's body is denoted against the oracle that answers its
own handle with the body under construction (`Proofs.Recursion`). -/
def ExprDenotation.AgreesWith (unit : ExecutableUnit) (callee : CalleeRelation)
    (namespaceId : NamespaceId) (exprId : ExprId) (denotation : ExprDenotation) :
    Prop :=
  ∀ frame state finalFrame finalState control,
    denotation frame state finalFrame finalState control ↔
      EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState
        control

/-- Exact agreement of a native operand relation with a deep operand row,
under a callee oracle. -/
def ValuesDenotation.AgreesWith (unit : ExecutableUnit) (callee : CalleeRelation)
    (namespaceId : NamespaceId) (expressions : List ExprId)
    (denotation : ValuesDenotation) : Prop :=
  ∀ frame state result,
    denotation frame state result ↔
      EvalValuesWith unit callee namespaceId frame state expressions result

/-- Exact agreement of a native statement relation with a deep statement
row, under a callee oracle. -/
def StatementsDenotation.AgreesWith (unit : ExecutableUnit) (callee : CalleeRelation)
    (namespaceId : NamespaceId) (statements : List ExprId)
    (denotation : StatementsDenotation) : Prop :=
  ∀ frame state result,
    denotation frame state result ↔
      EvalStatementsWith unit callee namespaceId frame state statements result

/-- Exact agreement of a native whole-function relation with what the
oracle answers for one handle. -/
def FunctionDenotation.AgreesWith (callee : CalleeRelation) (handle : FunctionHandle)
    (typeInstantiation : Array (TypeId × TypeId))
    (denotation : FunctionDenotation) : Prop :=
  ∀ initial arguments final outcome,
    denotation initial arguments final outcome ↔
      callee handle typeInstantiation initial arguments final outcome

/-- Transport a function agreement across a computed invocation
instantiation.  Call agreement uses this to discharge non-generic calls
without changing the callee relation. -/
theorem FunctionDenotation.AgreesWith.of_typeInstantiation_eq
    {callee : CalleeRelation} {handle : FunctionHandle}
    {actual expected : Array (TypeId × TypeId)}
    {denotation : FunctionDenotation}
    (eq : actual = expected)
    (agree : FunctionDenotation.AgreesWith callee handle expected denotation) :
    FunctionDenotation.AgreesWith callee handle actual denotation := by
  subst actual
  exact agree

/-- Exact agreement of a native expression relation with one deep node. -/
abbrev ExprDenotation.Agrees (unit : ExecutableUnit) (namespaceId : NamespaceId)
    (exprId : ExprId) (denotation : ExprDenotation) : Prop :=
  ExprDenotation.AgreesWith unit (EvalFunction unit) namespaceId exprId denotation

/-- Exact agreement of a native operand relation with a deep operand row. -/
abbrev ValuesDenotation.Agrees (unit : ExecutableUnit) (namespaceId : NamespaceId)
    (expressions : List ExprId) (denotation : ValuesDenotation) : Prop :=
  ValuesDenotation.AgreesWith unit (EvalFunction unit) namespaceId expressions
    denotation

/-- Exact agreement of a native statement relation with a deep statement
row. -/
abbrev StatementsDenotation.Agrees (unit : ExecutableUnit)
    (namespaceId : NamespaceId) (statements : List ExprId)
    (denotation : StatementsDenotation) : Prop :=
  StatementsDenotation.AgreesWith unit (EvalFunction unit) namespaceId statements
    denotation

/-- Exact agreement of a native whole-function relation with one deep
function handle. -/
abbrev FunctionDenotation.Agrees (unit : ExecutableUnit)
    (handle : FunctionHandle) (denotation : FunctionDenotation) : Prop :=
  FunctionDenotation.AgreesWith (EvalFunction unit) handle #[] denotation

/-- Exact agreement at an explicitly specialized invocation. -/
abbrev FunctionDenotation.AgreesAt (unit : ExecutableUnit)
    (handle : FunctionHandle) (typeInstantiation : Array (TypeId × TypeId))
    (denotation : FunctionDenotation) : Prop :=
  FunctionDenotation.AgreesWith (EvalFunction unit) handle typeInstantiation denotation

/-! ## Leaf and sequencing combinators -/

/-- A literal whose runtime representation was fixed while generating the
native term. -/
def value (runtimeValue : RuntimeValue) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    finalFrame = frame ∧ finalState = state ∧ control = .value runtimeValue

/-- Read one local from the native runtime frame. -/
def localVar (localId : LocalId) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    ∃ runtimeValue,
      readLocal? frame localId = some runtimeValue ∧
      finalFrame = frame ∧ finalState = state ∧ control = .value runtimeValue

/-- Empty operand row. -/
def valuesNil : ValuesDenotation :=
  fun frame state result => result = .values state frame []

/-- Left-to-right operand sequencing, with abrupt control propagated. -/
def valuesCons (head : ExprDenotation) (tail : ValuesDenotation) :
    ValuesDenotation :=
  fun frame state result =>
    (∃ finalFrame finalState control,
      head frame state finalFrame finalState control ∧
      Abrupt control ∧ result = .control finalState finalFrame control) ∨
    (∃ headFrame headState runtimeValue finalFrame finalState values,
      head frame state headFrame headState (.value runtimeValue) ∧
      tail headFrame headState (.values finalState finalFrame values) ∧
      result = .values finalState finalFrame (runtimeValue :: values)) ∨
    (∃ headFrame headState runtimeValue finalFrame finalState control,
      head frame state headFrame headState (.value runtimeValue) ∧
      tail headFrame headState (.control finalState finalFrame control) ∧
      result = .control finalState finalFrame control)

/-- Empty statement row. -/
def statementsNil : StatementsDenotation :=
  fun frame state result => result = .done state frame

/-- Left-to-right statement sequencing.  Ordinary expression values are
discarded; abrupt control stops the row. -/
def statementsCons (head : ExprDenotation) (tail : StatementsDenotation) :
    StatementsDenotation :=
  fun frame state result =>
    (∃ finalFrame finalState control,
      head frame state finalFrame finalState control ∧
      Abrupt control ∧ result = .control finalState finalFrame control) ∨
    (∃ headFrame headState runtimeValue,
      head frame state headFrame headState (.value runtimeValue) ∧
      tail headFrame headState result)

/-! ## Operation combinators -/

/-! Lowering selects every static location before it emits a denotation.
The following descriptors are the residual, native operation vocabulary:
they contain no `ValidatedUnit`, namespace table, expression id, qualified
reference, generic-instantiation row, or arena id.  Dynamic reads remain
dynamic—e.g. whether a resource is present at an address—but the identity of
the location being read is already fixed. -/

/-- A native global-storage family.  Applying it to a runtime storage key
selects one global slot without resolving a type instantiation. -/
structure ResourceLocation where
  namespaceId : NamespaceId
  typeId : TypeId
  deriving Repr, BEq, Inhabited

/-- A checked mutable-borrow site after certificate lookup. -/
structure BorrowLocation where
  resource : ResourceLocation
  referenceType : ReferenceType
  kind : BorrowKind
  lexicalLoan : Nat
  deriving Repr, BEq, Inhabited

/-- Native global actions. -/
inductive GlobalLocationOperation where
  | contains (resource : ResourceLocation)
  | borrow (site : BorrowLocation)
  | take (resource : ResourceLocation)
  | publish (resource : ResourceLocation)
  deriving Repr, BEq, Inhabited

/-- A parameter or local slot selected before proof generation.  `LocalId`
is the runtime slot identity; the source `PlaceId` is deliberately absent. -/
structure LocalLocation where
  localId : LocalId
  deriving Repr, BEq, Inhabited

/-- Proof-facing short name for the native nominal step shared with the
semantic lowering certificate. -/
abbrev NominalFieldStep := SemanticOperations.NominalFieldStep

/-- A reborrow through a mutable reference stored in a known local slot.
The source place is `deref (localVar localId)` followed by zero or more
field steps. Lowering has already selected every nominal owner and field
position, the result reference type, and the borrow certificate's lexical
loan; proof-time execution never walks the source place arena. -/
structure DerefLocalBorrowOperation where
  location : LocalLocation
  fields : List NominalFieldStep := []
  referenceType : ReferenceType
  kind : BorrowKind
  lexicalLoan : Nat
  deriving Repr, BEq, Inhabited

/-- A borrow of a literal-indexed vector or tuple element rooted at a known
local.  `dereference` distinguishes an owned local aggregate from one held
through a mutable-reference parameter. -/
structure IndexedLocalBorrowOperation where
  location : LocalLocation
  dereference : Bool
  index : Nat
  referenceType : ReferenceType
  kind : BorrowKind
  lexicalLoan : Nat
  indexLocal : Option LocalId := none
  deriving Repr, BEq, Inhabited

/-- A borrow of one statically selected nominal field below a literal
index into an owned local vector or tuple. -/
structure IndexedLocalFieldBorrowOperation where
  location : LocalLocation
  index : Nat
  field : NominalFieldStep
  referenceType : ReferenceType
  kind : BorrowKind
  lexicalLoan : Nat
  deriving Repr, BEq, Inhabited

/-- Read-like operations on an already selected local slot.  The variants
remain distinct so the lowering certificate records the exact source
operation even though their runtime action is identical. -/
inductive LocalLocationOperation where
  | read (location : LocalLocation)
  | copy (location : LocalLocation)
  | move (location : LocalLocation)
  | borrow (location : LocalLocation) (referenceType : ReferenceType)
      (kind : BorrowKind) (lexicalLoan : Nat)
  deriving Repr, BEq, Inhabited

/-- Recover only the source operation tag for an agreement statement.  The
source place is an argument to the certificate, never part of the native
denotation. -/
def LocalLocationOperation.sourceOperation
    (operation : LocalLocationOperation) (place : PlaceId) : Operation :=
  match operation with
  | .read _ => .read place
  | .copy _ => .copy place
  | .move _ => .move place
  | .borrow _ _ kind _ => .borrow kind place

/-- A nominal constructor after declaration and arity resolution. -/
structure NominalConstructor where
  source : StructHandle
  variant : Option String
  arity : Nat
  deriving Repr, BEq, Inhabited

/-- A nominal field after owner, variant, and field-position resolution.
V1 admits this form for a statically selected payload (ordinary structures,
and fields after a statically known enum downcast). -/
structure NominalFieldLocation where
  source : StructHandle
  variant : Option String
  index : Nat
  deriving Repr, BEq, Inhabited

/-- A variant payload selection after its per-variant offsets are resolved. -/
structure NominalVariantFieldLocation where
  source : StructHandle
  choices : Array (String × Nat)
  deriving Repr, BEq, Inhabited

/-- A nominal variant predicate after owner resolution. -/
structure NominalVariantTest where
  source : StructHandle
  variants : Array String
  deriving Repr, BEq, Inhabited

/-- The primitive subset whose result type has already been resolved. -/
inductive PrimitiveLocationOperation where
  | tuple
  | vector
  | pushVector
  | concatVector
  | slice
  | insertVector
  | removeVector
  | swapVector
  | reverseSliceVector
  | destroyEmptyVector
  | containsVector
  | indexOfVector (indexType : Ty)
  | checkVectorIndex (failure : ThrowKind)
  | length (resultType : Ty)
  | logicalNot
  | index (resultType : Ty)
  | copyValue (resultType : Ty)
  | moveValue (resultType : Ty)
  | add (resultType : Ty)
  | checkedAdd (failure : ThrowKind) (resultType : Ty)
  | subtract (resultType : Ty)
  | checkedSubtract (failure : ThrowKind) (resultType : Ty)
  | multiply (resultType : Ty)
  | checkedMultiply (failure : ThrowKind) (resultType : Ty)
  | less (resultType : Ty)
  | greater (resultType : Ty)
  | lessEqual (resultType : Ty)
  | greaterEqual (resultType : Ty)
  | equal (resultType : Ty)
  | notEqual (resultType : Ty)
  | divide (resultType : Ty)
  | checkedDivide (failure : ThrowKind) (resultType : Ty)
  | modulo (resultType : Ty)
  | checkedModulo (failure : ThrowKind) (resultType : Ty)
  | bitwiseOr (resultType : Ty)
  | bitwiseAnd (resultType : Ty)
  | bitwiseXor (resultType : Ty)
  | bitwiseNot (resultType : Ty)
  | shiftLeft (resultType : Ty)
  | checkedShiftLeft (failure : ThrowKind) (resultType : Ty)
  | shiftRight (resultType : Ty)
  | checkedShiftRight (failure : ThrowKind) (resultType : Ty)
  | cast (resultType : Ty)
  | checkedCast (failure : ThrowKind) (resultType : Ty)
  | logicalAnd
  | logicalOr
  deriving Repr, BEq, Inhabited

/-- Reference operations after result-type resolution. -/
inductive ReferenceLocationOperation where
  | dereference
  | mutate
  | freeze (resultType : ReferenceType)
  | endLoan (loans : Array LoanId)
  deriving Repr, BEq, Inhabited

/-- A lowered operation has one uniform stateful result shape. -/
abbrev NativeEvaluator := Array RuntimeValue → RuntimeFrame → RuntimeState →
  Option GlobalOperationResult

/-- Embed a pure evaluator into the uniform lowered operation result. -/
def liftPrimitiveEvaluator
    (evaluate : Array RuntimeValue →
      Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue)) :
    NativeEvaluator := fun arguments frame state => do
  match ← evaluate arguments with
  | .ok value => some (.value frame state value)
  | .error (kind, thrown) => some (.throw_ frame state kind thrown)

/-- Embed an existing place evaluator into the uniform lowered result. -/
def liftPlaceEvaluator
    (evaluate : Array RuntimeValue → RuntimeFrame → RuntimeState →
      Option (RuntimeFrame × RuntimeState × RuntimeValue)) :
    NativeEvaluator := fun arguments frame state => do
  let (finalFrame, finalState, value) ← evaluate arguments frame state
  some (.value finalFrame finalState value)

/-- Embed a pure nominal constructor into the uniform lowered result. -/
def liftConstructorEvaluator
    (evaluate : Array RuntimeValue → Option RuntimeValue) : NativeEvaluator :=
  fun arguments frame state => do
    let value ← evaluate arguments
    some (.value frame state value)

/-- Evaluate a global operation at its already selected resource location. -/
def GlobalLocationOperation.evaluate? :
    GlobalLocationOperation → NativeEvaluator
  | .contains resource => fun arguments frame state =>
      containsGlobalAt? resource.namespaceId
        (instantiatedTypeId frame.typeInstantiation resource.typeId)
        arguments frame state
  | .borrow site =>
      fun arguments frame state =>
        borrowGlobalAt? site.resource.namespaceId
          (instantiatedTypeId frame.typeInstantiation site.resource.typeId)
          site.referenceType site.lexicalLoan site.kind arguments frame state
  | .take resource => fun arguments frame state =>
      takeGlobalAt? resource.namespaceId
        (instantiatedTypeId frame.typeInstantiation resource.typeId)
        arguments frame state
  | .publish resource => fun arguments frame state =>
      publishGlobalAt? resource.namespaceId
        (instantiatedTypeId frame.typeInstantiation resource.typeId)
        arguments frame state

/-- Read an already selected local slot.  This performs only the dynamic
initialization check; it contains no namespace or place-arena lookup. -/
def LocalLocationOperation.evaluate? :
    LocalLocationOperation → NativeEvaluator
  | .read location | .copy location => fun arguments frame state => do
      if !arguments.isEmpty then none else
      let resolved ←
        if location.localId.index < frame.locals.size then
          some { root := RuntimePlaceRoot.local location.localId }
        else none
      let value ← readRuntimePlace? frame state resolved
      some (.value frame state value)
  | .move location => fun arguments frame state => do
      if !arguments.isEmpty then none else
      let resolved ←
        if location.localId.index < frame.locals.size then
          some { root := RuntimePlaceRoot.local location.localId }
        else none
      let value ← readRuntimePlace? frame state resolved
      let frame := { frame with
        locals := frame.locals.set! location.localId.index none }
      some (.value frame state value)
  | .borrow location referenceType kind lexicalLoan =>
      liftPlaceEvaluator fun arguments frame state => do
        if !arguments.isEmpty then none else
        if location.localId.index < frame.locals.size then
          borrowRuntimePlaceAt? lexicalLoan referenceType kind frame state
            { root := .local location.localId }
        else none

/-- Resolve the native runtime place of a local reborrow. Only slot bounds,
the dynamic borrow tag, and nominal shape checks remain. -/
def DerefLocalBorrowOperation.resolve? (operation : DerefLocalBorrowOperation)
    (frame : RuntimeFrame) (state : RuntimeState) : Option RuntimePlace :=
  resolveDerefLocalFieldPath? operation.location.localId operation.fields
    frame state

/-- Reborrow the referent of a mutable reference in one selected local.
Neither the place arena nor the borrow certificate is consulted here. -/
def DerefLocalBorrowOperation.evaluate? (operation : DerefLocalBorrowOperation) :
    NativeEvaluator :=
  liftPlaceEvaluator fun arguments frame state => do
    if !arguments.isEmpty then none else
    let resolved ← operation.resolve? frame state
    borrowRuntimePlaceAt? operation.lexicalLoan operation.referenceType
      operation.kind frame state resolved

/-- Resolve and borrow a statically indexed local aggregate. -/
def IndexedLocalBorrowOperation.resolve?
    (operation : IndexedLocalBorrowOperation)
    (frame : RuntimeFrame) (state : RuntimeState) : Option RuntimePlace :=
  match operation.indexLocal with
  | none => resolveLocalLiteralIndex? operation.location.localId operation.dereference
      operation.index frame state
  | some index =>
      if operation.dereference then
        resolveLocalDynamicIndex? operation.location.localId index true frame state
      else resolveLocalIndex? operation.location.localId index frame

def IndexedLocalBorrowOperation.evaluate?
    (operation : IndexedLocalBorrowOperation) : NativeEvaluator :=
  liftPlaceEvaluator fun arguments frame state => do
    if !arguments.isEmpty then none else
    let resolved ← operation.resolve? frame state
    borrowRuntimePlaceAt? operation.lexicalLoan operation.referenceType
      operation.kind frame state resolved

/-- Resolve and borrow a statically selected field below an owned local
aggregate's literal-indexed element. -/
def IndexedLocalFieldBorrowOperation.resolve?
    (operation : IndexedLocalFieldBorrowOperation)
    (frame : RuntimeFrame) (state : RuntimeState) : Option RuntimePlace :=
  resolveLocalLiteralIndexField? operation.location.localId operation.index
    operation.field frame state

def IndexedLocalFieldBorrowOperation.evaluate?
    (operation : IndexedLocalFieldBorrowOperation) : NativeEvaluator :=
  liftPlaceEvaluator fun arguments frame state => do
    if !arguments.isEmpty then none else
    let resolved ← operation.resolve? frame state
    borrowRuntimePlaceAt? operation.lexicalLoan operation.referenceType
      operation.kind frame state resolved

/-- Evaluate a fixed-width primitive without a type-table lookup. -/
def PrimitiveLocationOperation.evaluate? :
    PrimitiveLocationOperation → NativeEvaluator
  | .tuple, arguments, frame, state =>
      some (.value frame state (.tuple arguments))
  | .vector, arguments, frame, state =>
      some (.value frame state (.vector arguments))
  | .pushVector, arguments, frame, state =>
      liftPrimitiveEvaluator (fun arguments =>
        match arguments.toList with
        | [.vector elements, value] =>
            some (.ok (.vector (elements.push value)))
        | _ => none) arguments frame state
  | .insertVector, arguments, frame, state =>
      liftPrimitiveEvaluator insertVector? arguments frame state
  | .concatVector, arguments, frame, state =>
      liftPrimitiveEvaluator concatVector? arguments frame state
  | .slice, arguments, frame, state =>
      liftPrimitiveEvaluator sliceVector? arguments frame state
  | .removeVector, arguments, frame, state =>
      liftPrimitiveEvaluator removeVector? arguments frame state
  | .swapVector, arguments, frame, state =>
      liftPrimitiveEvaluator swapVector? arguments frame state
  | .reverseSliceVector, arguments, frame, state =>
      liftPrimitiveEvaluator reverseSliceVector? arguments frame state
  | .destroyEmptyVector, arguments, frame, state =>
      liftPrimitiveEvaluator destroyEmptyVector? arguments frame state
  | .containsVector, arguments, frame, state =>
      liftPrimitiveEvaluator containsVector? arguments frame state
  | .indexOfVector indexType, arguments, frame, state =>
      liftPrimitiveEvaluator (indexOfVector? indexType) arguments frame state
  | .checkVectorIndex failure, arguments, frame, state =>
      liftPrimitiveEvaluator (checkVectorIndex? failure) arguments frame state
  | .length resultType, arguments, frame, state =>
      liftPrimitiveEvaluator (fun arguments =>
        let lengthValue (length : Nat) := match resultType with
          | .integer .unbounded _ => some (.integer (Int.ofNat length))
          | _ => modularInteger resultType length
        match arguments.toList with
        | [.vector elements] => .ok <$> lengthValue elements.size
        | [.string value] => .ok <$> lengthValue value.utf8ByteSize
        | [.bytes values] => .ok <$> lengthValue values.size
        | _ => none) arguments frame state
  | .logicalNot, arguments, frame, state =>
      liftPrimitiveEvaluator (fun arguments =>
        match arguments.toList with
        | [.bool operand] => some (.ok (.bool (!operand)))
        | _ => none) arguments frame state
  | .index _, arguments, frame, state =>
      liftPrimitiveEvaluator (fun values => match values.toList with
        | [RuntimeValue.vector elements, RuntimeValue.integer position] =>
            if position < 0 then some (.error (.abort, #[.integer position]))
            else match elements[position.toNat]? with
              | some element => some (.ok element)
              | none => some (.error (.abort, #[.integer position]))
        | _ => none) arguments frame state
  | .copyValue _, arguments, frame, state
  | .moveValue _, arguments, frame, state =>
      match arguments.toList with
      | [value] => some (.value frame state value)
      | _ => none
  | .add resultType, arguments, frame, state => do
      let value ← modularBinaryInteger resultType arguments (fun left right => left + right)
      match value with
      | .ok value => some (.value frame state value)
      | .error (kind, thrown) => some (.throw_ frame state kind thrown)
  | .checkedAdd failure resultType, arguments, frame, state => do
      let value ← checkedBinaryInteger failure resultType arguments
        (fun left right => left + right)
      match value with
      | .ok value => some (.value frame state value)
      | .error (kind, thrown) => some (.throw_ frame state kind thrown)
  | .subtract resultType, arguments, frame, state => do
      let value ← modularBinaryInteger resultType arguments (fun left right => left - right)
      match value with
      | .ok value => some (.value frame state value)
      | .error (kind, thrown) => some (.throw_ frame state kind thrown)
  | .checkedSubtract failure resultType, arguments, frame, state => do
      let value ← checkedBinaryInteger failure resultType arguments
        (fun left right => left - right)
      match value with
      | .ok value => some (.value frame state value)
      | .error (kind, thrown) => some (.throw_ frame state kind thrown)
  | .multiply resultType, arguments, frame, state => do
      let value ← modularBinaryInteger resultType arguments (fun left right => left * right)
      match value with
      | .ok value => some (.value frame state value)
      | .error (kind, thrown) => some (.throw_ frame state kind thrown)
  | .checkedMultiply failure resultType, arguments, frame, state => do
      let value ← checkedBinaryInteger failure resultType arguments
        (fun left right => left * right)
      match value with
      | .ok value => some (.value frame state value)
      | .error (kind, thrown) => some (.throw_ frame state kind thrown)
  | .less _, arguments, frame, state =>
      liftPrimitiveEvaluator (fun values =>
        compareOrdered values (fun left right => left < right) booleanLess
          (fun left right => left < right)) arguments frame state
  | .greater _, arguments, frame, state =>
      liftPrimitiveEvaluator (fun values =>
        compareOrdered values (fun left right => left > right) booleanGreater
          (fun left right => left > right)) arguments frame state
  | .lessEqual _, arguments, frame, state =>
      liftPrimitiveEvaluator (fun values =>
        compareOrdered values (fun left right => left <= right) booleanLessEqual
          (fun left right => left <= right)) arguments frame state
  | .greaterEqual _, arguments, frame, state =>
      liftPrimitiveEvaluator (fun values =>
        compareOrdered values (fun left right => left >= right) booleanGreaterEqual
          (fun left right => left >= right)) arguments frame state
  | .equal _, arguments, frame, state =>
      liftPrimitiveEvaluator equalValues? arguments frame state
  | .notEqual _, arguments, frame, state =>
      liftPrimitiveEvaluator notEqualValues? arguments frame state
  | .divide resultType, arguments, frame, state =>
      liftPrimitiveEvaluator (divideIntegers? resultType) arguments frame state
  | .checkedDivide failure resultType, arguments, frame, state =>
      liftPrimitiveEvaluator (checkedDivideIntegers? failure resultType) arguments frame state
  | .modulo resultType, arguments, frame, state =>
      liftPrimitiveEvaluator (moduloIntegers? resultType) arguments frame state
  | .checkedModulo failure resultType, arguments, frame, state =>
      liftPrimitiveEvaluator (checkedModuloIntegers? failure resultType) arguments frame state
  | .bitwiseOr resultType, arguments, frame, state =>
      liftPrimitiveEvaluator
        (fun arguments => bitwiseBinary resultType arguments
          (fun left right => left ||| right) booleanOr) arguments frame state
  | .bitwiseAnd resultType, arguments, frame, state =>
      liftPrimitiveEvaluator
        (fun arguments => bitwiseBinary resultType arguments
          (fun left right => left &&& right) booleanAnd) arguments frame state
  | .bitwiseXor resultType, arguments, frame, state =>
      liftPrimitiveEvaluator
        (fun arguments => bitwiseBinary resultType arguments
          (fun left right => left ^^^ right) booleanXor) arguments frame state
  | .bitwiseNot resultType, arguments, frame, state =>
      liftPrimitiveEvaluator (bitwiseNotInteger resultType) arguments frame state
  | .shiftLeft resultType, arguments, frame, state =>
      liftPrimitiveEvaluator (shiftInteger true resultType) arguments frame state
  | .checkedShiftLeft failure resultType, arguments, frame, state =>
      liftPrimitiveEvaluator
        (checkedShiftInteger failure true resultType) arguments frame state
  | .shiftRight resultType, arguments, frame, state =>
      liftPrimitiveEvaluator (shiftInteger false resultType) arguments frame state
  | .checkedShiftRight failure resultType, arguments, frame, state =>
      liftPrimitiveEvaluator
        (checkedShiftInteger failure false resultType) arguments frame state
  | .cast resultType, arguments, frame, state =>
      liftPrimitiveEvaluator (fun arguments => match arguments.toList with
        | [.integer value] => match resultType with
            | .character =>
              if value < 0 || !isUnicodeScalar value.toNat then none
              else some (.ok (.character value.toNat))
            | _ => .ok <$> modularInteger resultType value
        | [.character value] => .ok <$> modularInteger resultType (Int.ofNat value)
        | _ => none) arguments frame state
  | .checkedCast failure resultType, arguments, frame, state =>
      liftPrimitiveEvaluator (fun arguments => match arguments.toList with
        | [.integer value] => match resultType with
            | .character =>
              if value < 0 || !isUnicodeScalar value.toNat then
                some (.error (failure, #[.integer value]))
              else some (.ok (.character value.toNat))
            | _ => some <| checkedInteger failure resultType value
        | [.character value] =>
            some <| checkedInteger failure resultType (Int.ofNat value)
        | _ => none) arguments frame state
  | .logicalAnd, arguments, frame, state =>
      liftPrimitiveEvaluator (fun arguments => match arguments.toList with
        | [.bool left, .bool right] => some (.ok (.bool (left && right)))
        | _ => none) arguments frame state
  | .logicalOr, arguments, frame, state =>
      liftPrimitiveEvaluator (fun arguments => match arguments.toList with
        | [.bool left, .bool right] => some (.ok (.bool (left || right)))
        | _ => none) arguments frame state

/-- Evaluate a constructor with no declaration/name/arity lookup. -/
def NominalConstructor.evaluate? (constructor : NominalConstructor) :
    NativeEvaluator := fun arguments frame state =>
  if constructor.arity != arguments.size then none
  else some (.value frame state
    (.nominal constructor.source constructor.variant arguments))

/-- Select one already resolved nominal field. -/
def NominalFieldLocation.evaluateSelect? (field : NominalFieldLocation) :
    NativeEvaluator := liftConstructorEvaluator
  (selectNominalFieldAt? field.source field.variant field.index)

def NominalVariantFieldLocation.evaluateSelect?
    (field : NominalVariantFieldLocation) : NativeEvaluator :=
  liftConstructorEvaluator
    (selectNominalVariantFieldAt? field.source field.choices)

def NominalVariantTest.evaluate? (test : NominalVariantTest) :
    NativeEvaluator := liftConstructorEvaluator
  (testNominalVariants? test.source test.variants)

/-- Evaluate reference-value operations after lowering has removed every
static result-type lookup. -/
def ReferenceLocationOperation.evaluate? :
    ReferenceLocationOperation → NativeEvaluator
  | .dereference => liftPlaceEvaluator dereferenceBorrow?
  | .mutate => liftPlaceEvaluator mutateBorrow?
  | .freeze resultType => liftPlaceEvaluator (freezeBorrow? resultType)
  | .endLoan loans => liftPlaceEvaluator (endLoans? loans)

/-- Execute one lowered operation.  Only its dynamic operands/frame/state
remain; all semantic selection happened in lowering. -/
def nativeOperation (evaluate : NativeEvaluator)
    (operands : ValuesDenotation) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (∃ operandFrame operandState propagated,
      operands frame state (.control operandState operandFrame propagated) ∧
      finalFrame = operandFrame ∧ finalState = operandState ∧ control = propagated) ∨
    (∃ operandFrame operandState values,
      operands frame state (.values operandState operandFrame values) ∧
      ((∃ runtimeValue,
          evaluate values.toArray operandFrame operandState =
            some (.value finalFrame finalState runtimeValue) ∧
          control = .value runtimeValue) ∨
       (∃ throwKind thrown,
          evaluate values.toArray operandFrame operandState =
            some (.throw_ finalFrame finalState throwKind thrown) ∧
          control = .throw_ throwKind thrown)))

/-- Execute a lowered primitive while retaining its descriptor in the
denotation term.  The descriptor-preserving head lets the WP select the
primitive rule directly instead of recovering its branches from an opaque
evaluator equation. -/
def nativePrimitiveOperation (operation : PrimitiveLocationOperation)
    (operands : ValuesDenotation) : ExprDenotation :=
  nativeOperation operation.evaluate? operands

/-- Execute a lowered global operation without erasing its keyed resource
location.  Verification can therefore reason about the selected slot
directly; it never has to recover the resource family from an evaluator. -/
def nativeGlobalOperation (operation : GlobalLocationOperation)
    (operands : ValuesDenotation) : ExprDenotation :=
  nativeOperation operation.evaluate? operands

/-- Execute a lowered parameter/local access while retaining its native slot
descriptor at the head of the denotation term. -/
def nativeLocalOperation (operation : LocalLocationOperation)
    (operands : ValuesDenotation) : ExprDenotation :=
  nativeOperation operation.evaluate? operands

/-- Execute a reborrow through a pre-resolved local-reference location. -/
def nativeDerefLocalBorrowOperation (operation : DerefLocalBorrowOperation)
    (operands : ValuesDenotation) : ExprDenotation :=
  nativeOperation operation.evaluate? operands

/-- Execute a borrow through a pre-resolved literal local index. -/
def nativeIndexedLocalBorrowOperation (operation : IndexedLocalBorrowOperation)
    (operands : ValuesDenotation) : ExprDenotation :=
  nativeOperation operation.evaluate? operands

/-- Execute a borrow through a literal local index and one resolved
nominal field. -/
def nativeIndexedLocalFieldBorrowOperation
    (operation : IndexedLocalFieldBorrowOperation)
    (operands : ValuesDenotation) : ExprDenotation :=
  nativeOperation operation.evaluate? operands

/-- Execute a lowered reference operation without erasing its resolved
descriptor.  In particular, mutation WP can expose the one dynamic
write-back choice without existentially recovering the evaluator result. -/
def nativeReferenceOperation (operation : ReferenceLocationOperation)
    (operands : ValuesDenotation) : ExprDenotation :=
  nativeOperation operation.evaluate? operands

/-! ## Lowering certificates

These lemmas are intentionally stated in terms of small, closed lookup
facts.  A lowering pass computes the descriptor and emits those facts once;
the proof below is uniform and all dynamic operands remain abstract. -/

theorem GlobalLocationOperation.contains_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId}
    {instantiations : Array GenericArgument} {resourceType : TypeUse}
    (resource : ResourceLocation)
    (identity_eq : ns.identity = resource.namespaceId)
    (instantiations_eq : instantiations = #[.typeArg resourceType])
    (resource_type_eq : resourceType.typeId = resource.typeId) :
    (GlobalLocationOperation.contains resource).evaluate? =
      fun arguments frame state => evaluateGlobalOperation? unit ns resultType
        site .contains instantiations arguments frame state := by
  subst instantiations
  funext arguments frame state
  simp only [GlobalLocationOperation.evaluate?]
  rw [evaluateGlobalOperation?_typeArg]
  rw [identity_eq, resource_type_eq]
  rfl

theorem LocalLocationOperation.read_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {place : PlaceId}
    (location : LocalLocation)
    (place_eq : ns.places[place.index]? =
      some (.localVar location.localId)) :
    (LocalLocationOperation.read location).evaluate? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? unit ns resultType site (.read place)
          arguments frame state := by
  funext arguments frame state
  by_cases arguments_eq : arguments = #[]
  · subst arguments
    by_cases in_bounds : location.localId.index < frame.locals.size <;>
      simp [LocalLocationOperation.evaluate?, liftPlaceEvaluator,
        evaluatePlaceOperation?, resolvePlace?_localVar place_eq,
        readRuntimePlace?, readRoot?, readProjections?, in_bounds]
    all_goals
      cases local_eq : readLocal? frame location.localId <;> simp
  · simp [LocalLocationOperation.evaluate?, liftPlaceEvaluator,
      evaluatePlaceOperation?, arguments_eq]

theorem LocalLocationOperation.copy_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {place : PlaceId}
    (location : LocalLocation)
    (place_eq : ns.places[place.index]? =
      some (.localVar location.localId)) :
    (LocalLocationOperation.copy location).evaluate? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? unit ns resultType site (.copy place)
          arguments frame state := by
  funext arguments frame state
  by_cases arguments_eq : arguments = #[]
  · subst arguments
    by_cases in_bounds : location.localId.index < frame.locals.size <;>
      simp [LocalLocationOperation.evaluate?, liftPlaceEvaluator,
        evaluatePlaceOperation?, resolvePlace?_localVar place_eq,
        readRuntimePlace?, readRoot?, readProjections?, in_bounds]
    all_goals
      cases local_eq : readLocal? frame location.localId <;> simp
  · simp [LocalLocationOperation.evaluate?, liftPlaceEvaluator,
      evaluatePlaceOperation?, arguments_eq]

theorem LocalLocationOperation.move_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {place : PlaceId}
    (location : LocalLocation)
    (place_eq : ns.places[place.index]? =
      some (.localVar location.localId)) :
    (LocalLocationOperation.move location).evaluate? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? unit ns resultType site (.move place)
          arguments frame state := by
  funext arguments frame state
  by_cases arguments_eq : arguments = #[]
  · subst arguments
    by_cases in_bounds : location.localId.index < frame.locals.size <;>
      simp [LocalLocationOperation.evaluate?, liftPlaceEvaluator,
        evaluatePlaceOperation?, resolvePlace?_localVar place_eq,
        readRuntimePlace?, readRoot?, readProjections?, in_bounds]
    all_goals
      cases local_eq : readLocal? frame location.localId <;> simp
  · simp [LocalLocationOperation.evaluate?, liftPlaceEvaluator,
      evaluatePlaceOperation?, arguments_eq]

theorem LocalLocationOperation.borrow_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {place : PlaceId}
    (location : LocalLocation) (referenceType : ReferenceType)
    (kind : BorrowKind) (lexicalLoan : Nat)
    (place_eq : ns.places[place.index]? =
      some (.localVar location.localId))
    (result_type_eq : ns.tables.types[resultType.index]? =
      some (.reference referenceType))
    (borrower_eq :
      (fun frame state place => borrowRuntimePlaceAt? lexicalLoan
        referenceType kind frame state place) =
      (fun frame state place => borrowRuntimePlace? unit ns site
        referenceType kind frame state place)) :
    (LocalLocationOperation.borrow location referenceType kind lexicalLoan).evaluate? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? unit ns resultType site (.borrow kind place)
          arguments frame state := by
  funext arguments frame state
  by_cases arguments_eq : arguments = #[]
  · subst arguments
    by_cases in_bounds : location.localId.index < frame.locals.size <;>
      simp [LocalLocationOperation.evaluate?, liftPlaceEvaluator,
        evaluatePlaceOperation?, result_type_eq,
        resolvePlace?_localVar place_eq, borrower_eq, in_bounds]
  · simp [LocalLocationOperation.evaluate?, liftPlaceEvaluator,
      evaluatePlaceOperation?, arguments_eq]

/-- A whole-referent local reborrow resolves through its closed local slot
without reducing the surrounding prepared unit. -/
theorem DerefLocalBorrowOperation.resolve_eq_nil_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {place base : PlaceId}
    (unit_eq : actual = prepared)
    (operation : DerefLocalBorrowOperation)
    (place_eq : ns.places[place.index]? = some (.deref base))
    (base_eq : ns.places[base.index]? =
      some (.localVar operation.location.localId))
    (fields_eq : operation.fields = []) :
    operation.resolve? =
      fun frame state => resolvePlace? actual ns frame state place := by
  subst actual
  let path : DerefLocalFieldPath prepared ns place [] :=
    .deref place_eq base_eq
  have enough : ([] : List NominalFieldStep).length + 2 ≤
      2 * ns.places.size + 3 := by
    simp only [List.length_nil]
    omega
  funext frame state
  rw [resolvePlace?_of_derefLocalFieldPath path enough]
  change resolveDerefLocalFieldPath? operation.location.localId
      operation.fields frame state =
    resolveDerefLocalFieldPath? operation.location.localId [] frame state
  rw [fields_eq]

/-- An arbitrary statically certified field chain resolves through the
compact native descriptor.  The path certificate is proof-only; the
denotation retains only its local slot and field row. -/
theorem DerefLocalBorrowOperation.resolve_eq_path_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {place : PlaceId}
    (unit_eq : actual = prepared)
    (operation : DerefLocalBorrowOperation)
    (path : DerefLocalFieldPath prepared ns place operation.fields.reverse)
    (local_eq : path.localId = operation.location.localId)
    (enough : operation.fields.length + 2 ≤ 2 * ns.places.size + 3) :
    operation.resolve? =
      fun frame state => resolvePlace? actual ns frame state place := by
  subst actual
  funext frame state
  rw [resolvePlace?_of_derefLocalFieldPath path (by simpa using enough)]
  simp only [DerefLocalBorrowOperation.resolve?]
  rw [local_eq, List.reverse_reverse]

/-- Agreement certificate for a native local reborrow. Lowering supplies one
closed equality for the statically resolved place chain and one for the
lexical-loan borrower; the executable evaluator retains only the native
descriptor. -/
theorem DerefLocalBorrowOperation.evaluator_eq_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {place : PlaceId}
    (unit_eq : actual = prepared)
    (operation : DerefLocalBorrowOperation)
    (result_type_eq : ns.tables.types[resultType.index]? =
      some (.reference operation.referenceType))
    (resolver_eq :
      operation.resolve? =
        fun frame state => resolvePlace? prepared ns frame state place)
    (borrower_eq :
      (fun frame state place => borrowRuntimePlaceAt? operation.lexicalLoan
        operation.referenceType operation.kind frame state place) =
      (fun frame state place => borrowRuntimePlace? prepared ns site
        operation.referenceType operation.kind frame state place)) :
    operation.evaluate? = liftPlaceEvaluator fun arguments frame state =>
      evaluatePlaceOperation? actual ns resultType site
        (.borrow operation.kind place) arguments frame state := by
  subst actual
  funext arguments frame state
  simp [DerefLocalBorrowOperation.evaluate?, liftPlaceEvaluator,
    evaluatePlaceOperation?, result_type_eq, resolver_eq, borrower_eq]

/-- Whole-referent specialization used by the generated agreement proof.
Keeping the source-place equations explicit prevents elaboration from
trying to reduce the enclosing validated unit. -/
theorem DerefLocalBorrowOperation.evaluator_eq_nil_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {place base : PlaceId}
    (unit_eq : actual = prepared)
    (operation : DerefLocalBorrowOperation)
    (place_eq : ns.places[place.index]? = some (.deref base))
    (base_eq : ns.places[base.index]? =
      some (.localVar operation.location.localId))
    (result_type_eq : ns.tables.types[resultType.index]? =
      some (.reference operation.referenceType))
    (fields_eq : operation.fields = [])
    (borrower_eq :
      (fun frame state place => borrowRuntimePlaceAt? operation.lexicalLoan
        operation.referenceType operation.kind frame state place) =
      (fun frame state place => borrowRuntimePlace? prepared ns site
        operation.referenceType operation.kind frame state place)) :
    operation.evaluate? = liftPlaceEvaluator fun arguments frame state =>
      evaluatePlaceOperation? actual ns resultType site
        (.borrow operation.kind place) arguments frame state := by
  subst actual
  apply operation.evaluator_eq_of_unit rfl result_type_eq
  · exact operation.resolve_eq_nil_of_unit rfl place_eq base_eq fields_eq
  · exact borrower_eq

/-- Arbitrary-field-chain specialization consumed by generated agreement
proofs.  Every source lookup is a constructor premise of `path`, so this
theorem never reduces the enclosing prepared unit. -/
theorem DerefLocalBorrowOperation.evaluator_eq_path_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {place : PlaceId}
    (unit_eq : actual = prepared)
    (operation : DerefLocalBorrowOperation)
    (path : DerefLocalFieldPath prepared ns place operation.fields.reverse)
    (local_eq : path.localId = operation.location.localId)
    (enough : operation.fields.length + 2 ≤ 2 * ns.places.size + 3)
    (result_type_eq : ns.tables.types[resultType.index]? =
      some (.reference operation.referenceType))
    (borrower_eq :
      (fun frame state place => borrowRuntimePlaceAt? operation.lexicalLoan
        operation.referenceType operation.kind frame state place) =
      (fun frame state place => borrowRuntimePlace? prepared ns site
        operation.referenceType operation.kind frame state place)) :
    operation.evaluate? = liftPlaceEvaluator fun arguments frame state =>
      evaluatePlaceOperation? actual ns resultType site
        (.borrow operation.kind place) arguments frame state := by
  subst actual
  apply operation.evaluator_eq_of_unit rfl result_type_eq
  · exact operation.resolve_eq_path_of_unit rfl path local_eq enough
  · exact borrower_eq

/-- Agreement of the compact indexed resolver with a source index rooted
directly at an owned local. -/
theorem IndexedLocalBorrowOperation.resolve_eq_local_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {place basePlace : PlaceId} {indexExpr : ExprId}
    (unit_eq : actual = prepared)
    (operation : IndexedLocalBorrowOperation)
    (place_eq : ns.places[place.index]? = some (.index basePlace indexExpr))
    (base_eq : ns.places[basePlace.index]? =
      some (.localVar operation.location.localId))
    (dereference_eq : operation.dereference = false)
    (index_local_eq : operation.indexLocal = none)
    (index_eq : placeIndexForm? ns indexExpr =
      some (.literal (operation.index : Int))) :
    operation.resolve? =
      fun frame state => resolvePlace? prepared ns frame state place := by
  subst actual
  funext frame state
  simp only [IndexedLocalBorrowOperation.resolve?, dereference_eq, index_local_eq]
  exact (resolvePlace?_localLiteralIndex place_eq base_eq index_eq).symm

/-- Agreement of the compact indexed resolver with an index through a
mutable-reference local. -/
theorem IndexedLocalBorrowOperation.resolve_eq_deref_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {place basePlace localPlace : PlaceId} {indexExpr : ExprId}
    (unit_eq : actual = prepared)
    (operation : IndexedLocalBorrowOperation)
    (place_eq : ns.places[place.index]? = some (.index basePlace indexExpr))
    (base_eq : ns.places[basePlace.index]? = some (.deref localPlace))
    (local_eq : ns.places[localPlace.index]? =
      some (.localVar operation.location.localId))
    (dereference_eq : operation.dereference = true)
    (index_local_eq : operation.indexLocal = none)
    (index_eq : placeIndexForm? ns indexExpr =
      some (.literal (operation.index : Int))) :
    operation.resolve? =
      fun frame state => resolvePlace? prepared ns frame state place := by
  subst actual
  funext frame state
  simp only [IndexedLocalBorrowOperation.resolve?, dereference_eq, index_local_eq]
  exact (resolvePlace?_derefLocalLiteralIndex place_eq base_eq local_eq index_eq).symm

theorem IndexedLocalBorrowOperation.resolve_eq_dynamic_local_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {place basePlace : PlaceId} {indexExpr : ExprId} {index : LocalId}
    (unit_eq : actual = prepared) (operation : IndexedLocalBorrowOperation)
    (place_eq : ns.places[place.index]? = some (.index basePlace indexExpr))
    (base_eq : ns.places[basePlace.index]? = some (.localVar operation.location.localId))
    (dereference_eq : operation.dereference = false)
    (index_local_eq : operation.indexLocal = some index)
    (index_eq : placeIndexForm? ns indexExpr = some (.local index) ∨
      placeIndexForm? ns indexExpr = some (.copyLocal index)) :
    operation.resolve? = fun frame state => resolvePlace? prepared ns frame state place := by
  subst actual
  funext frame state
  simp only [IndexedLocalBorrowOperation.resolve?, dereference_eq, index_local_eq, Bool.false_eq_true, ↓reduceIte]
  exact (resolvePlace?_localIndex place_eq base_eq index_eq).symm

theorem IndexedLocalBorrowOperation.resolve_eq_dynamic_deref_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {place basePlace localPlace : PlaceId} {indexExpr : ExprId} {index : LocalId}
    (unit_eq : actual = prepared) (operation : IndexedLocalBorrowOperation)
    (place_eq : ns.places[place.index]? = some (.index basePlace indexExpr))
    (base_eq : ns.places[basePlace.index]? = some (.deref localPlace))
    (local_eq : ns.places[localPlace.index]? = some (.localVar operation.location.localId))
    (dereference_eq : operation.dereference = true)
    (index_local_eq : operation.indexLocal = some index)
    (index_eq : placeIndexForm? ns indexExpr = some (.local index) ∨
      placeIndexForm? ns indexExpr = some (.copyLocal index)) :
    operation.resolve? = fun frame state => resolvePlace? prepared ns frame state place := by
  subst actual
  funext frame state
  simp only [IndexedLocalBorrowOperation.resolve?, dereference_eq, index_local_eq, ↓reduceIte]
  exact (resolvePlace?_derefLocalDynamicIndex place_eq base_eq local_eq index_eq).symm

theorem IndexedLocalBorrowOperation.evaluator_eq_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {place : PlaceId}
    (unit_eq : actual = prepared)
    (operation : IndexedLocalBorrowOperation)
    (result_type_eq : ns.tables.types[resultType.index]? =
      some (.reference operation.referenceType))
    (resolver_eq : operation.resolve? =
      fun frame state => resolvePlace? prepared ns frame state place)
    (borrower_eq :
      (fun frame state place => borrowRuntimePlaceAt? operation.lexicalLoan
        operation.referenceType operation.kind frame state place) =
      (fun frame state place => borrowRuntimePlace? prepared ns site
        operation.referenceType operation.kind frame state place)) :
    operation.evaluate? = liftPlaceEvaluator fun arguments frame state =>
      evaluatePlaceOperation? actual ns resultType site
        (.borrow operation.kind place) arguments frame state := by
  subst actual
  funext arguments frame state
  simp [IndexedLocalBorrowOperation.evaluate?, liftPlaceEvaluator,
    evaluatePlaceOperation?, result_type_eq, resolver_eq, borrower_eq]

/-- Agreement of the compact owned index/field resolver with its source
place.  All arena and declaration lookups are closed premises emitted by
the generator. -/
theorem IndexedLocalFieldBorrowOperation.evaluator_eq_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {place : PlaceId}
    (unit_eq : actual = prepared)
    (operation : IndexedLocalFieldBorrowOperation)
    (result_type_eq : ns.tables.types[resultType.index]? =
      some (.reference operation.referenceType))
    (resolver_eq : operation.resolve? =
      fun frame state => resolvePlace? prepared ns frame state place)
    (borrower_eq :
      (fun frame state place => borrowRuntimePlaceAt? operation.lexicalLoan
        operation.referenceType operation.kind frame state place) =
      (fun frame state place => borrowRuntimePlace? prepared ns site
        operation.referenceType operation.kind frame state place)) :
    operation.evaluate? = liftPlaceEvaluator fun arguments frame state =>
      evaluatePlaceOperation? actual ns resultType site
        (.borrow operation.kind place) arguments frame state := by
  subst actual
  funext arguments frame state
  simp [IndexedLocalFieldBorrowOperation.evaluate?, liftPlaceEvaluator,
    evaluatePlaceOperation?, result_type_eq, resolver_eq, borrower_eq]

theorem GlobalLocationOperation.borrow_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId}
    {instantiations : Array GenericArgument} {resourceType : TypeUse}
    (borrow : BorrowLocation)
    (identity_eq : ns.identity = borrow.resource.namespaceId)
    (instantiations_eq : instantiations = #[.typeArg resourceType])
    (resource_type_eq : resourceType.typeId = borrow.resource.typeId)
    (result_type_eq : ns.tables.types[resultType.index]? =
      some (.reference borrow.referenceType))
    (borrower_eq :
      (fun frame state place => borrowRuntimePlaceAt? borrow.lexicalLoan
        borrow.referenceType borrow.kind frame state place) =
      (fun frame state place => borrowRuntimePlace? unit ns site
        borrow.referenceType borrow.kind frame state place)) :
    (GlobalLocationOperation.borrow borrow).evaluate? =
      fun arguments frame state => evaluateGlobalOperation? unit ns resultType
        site (.borrow borrow.kind) instantiations arguments frame state := by
  subst instantiations
  funext arguments frame state
  simp only [GlobalLocationOperation.evaluate?, borrowGlobalAt?]
  rw [evaluateGlobalOperation?_typeArg]
  rw [identity_eq, resource_type_eq, result_type_eq]
  rw [borrower_eq]
  exact borrowGlobalUsing?_unfold borrow.resource.namespaceId
    (instantiatedTypeId frame.typeInstantiation borrow.resource.typeId)
    (fun frame state place => borrowRuntimePlace? unit ns site
      borrow.referenceType borrow.kind frame state place)
    arguments frame state

/-- As with nominal constructors, mutable-loan resolution is certified
against the closed prepared unit, then transported to the executable view.
This prevents agreement assembly from searching a symbolic unit's
certificate table. -/
theorem GlobalLocationOperation.borrow_evaluator_eq_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId}
    {instantiations : Array GenericArgument} {resourceType : TypeUse}
    (unit_eq : actual = prepared) (borrow : BorrowLocation)
    (identity_eq : ns.identity = borrow.resource.namespaceId)
    (instantiations_eq : instantiations = #[.typeArg resourceType])
    (resource_type_eq : resourceType.typeId = borrow.resource.typeId)
    (result_type_eq : ns.tables.types[resultType.index]? =
      some (.reference borrow.referenceType))
    (borrower_eq :
      (fun frame state place => borrowRuntimePlaceAt? borrow.lexicalLoan
        borrow.referenceType borrow.kind frame state place) =
      (fun frame state place => borrowRuntimePlace? prepared ns site
        borrow.referenceType borrow.kind frame state place)) :
    (GlobalLocationOperation.borrow borrow).evaluate? =
      fun arguments frame state => evaluateGlobalOperation? actual ns resultType
        site (.borrow borrow.kind) instantiations arguments frame state := by
  subst actual
  exact GlobalLocationOperation.borrow_evaluator_eq borrow identity_eq
    instantiations_eq resource_type_eq result_type_eq borrower_eq

theorem GlobalLocationOperation.take_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId}
    {instantiations : Array GenericArgument} {resourceType : TypeUse}
    (resource : ResourceLocation)
    (identity_eq : ns.identity = resource.namespaceId)
    (instantiations_eq : instantiations = #[.typeArg resourceType])
    (resource_type_eq : resourceType.typeId = resource.typeId) :
    (GlobalLocationOperation.take resource).evaluate? =
      fun arguments frame state => evaluateGlobalOperation? unit ns resultType
        site .take instantiations arguments frame state := by
  subst instantiations
  funext arguments frame state
  simp only [GlobalLocationOperation.evaluate?]
  rw [evaluateGlobalOperation?_typeArg]
  rw [identity_eq, resource_type_eq]
  rfl

theorem GlobalLocationOperation.publish_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId}
    {instantiations : Array GenericArgument} {resourceType : TypeUse}
    (resource : ResourceLocation)
    (identity_eq : ns.identity = resource.namespaceId)
    (instantiations_eq : instantiations = #[.typeArg resourceType])
    (resource_type_eq : resourceType.typeId = resource.typeId) :
    (GlobalLocationOperation.publish resource).evaluate? =
      fun arguments frame state => evaluateGlobalOperation? unit ns resultType
        site .publish instantiations arguments frame state := by
  subst instantiations
  funext arguments frame state
  simp only [GlobalLocationOperation.evaluate?]
  rw [evaluateGlobalOperation?_typeArg]
  rw [identity_eq, resource_type_eq]
  rfl

theorem PrimitiveLocationOperation.add_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.add resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .add arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.copyValue_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.copyValue resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .copyValue arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  generalize args_eq : arguments.toList = args
  cases args with
  | nil => simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
      evaluatePrimitiveOperation?, type_eq, args_eq]
  | cons value tail =>
      cases tail <;>
        simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
          evaluatePrimitiveOperation?, type_eq, args_eq]

theorem PrimitiveLocationOperation.tuple_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.tuple.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .tuple arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.vector_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.vector.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .vector arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.pushVector_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.pushVector.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .pushVector arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]
  rfl

theorem PrimitiveLocationOperation.insertVector_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.insertVector.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .insertVector arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.removeVector_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.removeVector.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .removeVector arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.swapVector_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.swapVector.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .swapVector arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.concatVector_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.concatVector.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .concatVector arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.slice_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.slice.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .slice arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.reverseSliceVector_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.reverseSliceVector.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .reverseSliceVector arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.destroyEmptyVector_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.destroyEmptyVector.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .destroyEmptyVector arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.containsVector_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.containsVector.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .containsVector arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.indexOfVector_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType found index : TypeId}
    (indexType : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some (.tuple #[found, index]))
    (index_eq : (ns.tables.types[index.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some indexType) :
    (PrimitiveLocationOperation.indexOfVector indexType).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .indexOfVector arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  change (ns.tables.types[index.index]?.bind
    (resolveTargetIntegerType? unit.targetPointerWidth)) = some indexType at index_eq
  simp [PrimitiveLocationOperation.evaluate?, evaluatePrimitiveOperation?, type_eq, index_eq]

theorem PrimitiveLocationOperation.checkVectorIndex_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty} (failure : ThrowKind)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.checkVectorIndex failure).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType (.checkVectorIndex failure) arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.index_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.index resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .index arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, evaluatePrimitiveOperation?, type_eq]
  rfl

theorem PrimitiveLocationOperation.length_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.length resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .length arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, evaluatePrimitiveOperation?, type_eq]
  rfl

theorem PrimitiveLocationOperation.logicalNot_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.logicalNot.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .logicalNot arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, evaluatePrimitiveOperation?, type_eq]
  rfl

theorem PrimitiveLocationOperation.moveValue_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.moveValue resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .moveValue arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  generalize args_eq : arguments.toList = args
  cases args with
  | nil => simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
      evaluatePrimitiveOperation?, type_eq, args_eq]
  | cons value tail =>
      cases tail <;>
        simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
          evaluatePrimitiveOperation?, type_eq, args_eq]

theorem PrimitiveLocationOperation.checkedAdd_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (failure : ThrowKind) (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.checkedAdd failure resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType (.checkedAdd failure) arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.subtract_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.subtract resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .subtract arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.checkedSubtract_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (failure : ThrowKind) (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.checkedSubtract failure resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments => evaluatePrimitiveOperation? ns
        resultType (.checkedSubtract failure) arguments unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.multiply_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.multiply resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .multiply arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.checkedMultiply_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (failure : ThrowKind) (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.checkedMultiply failure resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments => evaluatePrimitiveOperation? ns
        resultType (.checkedMultiply failure) arguments unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.less_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.less resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .less arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.greater_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.greater resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .greater arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.lessEqual_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.lessEqual resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .lessEqual arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.greaterEqual_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.greaterEqual resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .greaterEqual arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.equal_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.equal resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .equal arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.notEqual_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.notEqual resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .notEqual arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.divide_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.divide resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .divide arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.checkedDivide_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (failure : ThrowKind) (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.checkedDivide failure resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType (.checkedDivide failure) arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.modulo_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.modulo resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .modulo arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.checkedModulo_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (failure : ThrowKind) (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.checkedModulo failure resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType (.checkedModulo failure) arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.bitwiseOr_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.bitwiseOr resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .bitwiseOr arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.bitwiseAnd_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.bitwiseAnd resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .bitwiseAnd arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.bitwiseXor_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.bitwiseXor resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .bitwiseXor arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.bitwiseNot_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.bitwiseNot resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .bitwiseNot arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.shiftLeft_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.shiftLeft resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .shiftLeft arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.checkedShiftLeft_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (failure : ThrowKind) (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.checkedShiftLeft failure resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType (.checkedShiftLeft failure) arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.shiftRight_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.shiftRight resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .shiftRight arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.checkedShiftRight_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (failure : ThrowKind) (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.checkedShiftRight failure resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType (.checkedShiftRight failure) arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]

theorem PrimitiveLocationOperation.cast_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.cast resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .cast arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]
  rfl

theorem PrimitiveLocationOperation.checkedCast_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    (failure : ThrowKind) (resolved : Ty)
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    (PrimitiveLocationOperation.checkedCast failure resolved).evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType (.checkedCast failure) arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]
  rfl

theorem PrimitiveLocationOperation.logicalAnd_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.logicalAnd.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .logicalAnd arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]
  rfl

theorem PrimitiveLocationOperation.logicalOr_evaluator_eq
    {unit : ExecutableUnit} {ns : ValidatedNamespace} {resultType : TypeId}
    {resolved : Ty}
    (type_eq : (ns.tables.types[resultType.index]? >>= fun ty =>
      resolveTargetIntegerType? unit.targetPointerWidth ty) = some resolved) :
    PrimitiveLocationOperation.logicalOr.evaluate? =
      liftPrimitiveEvaluator fun arguments =>
        evaluatePrimitiveOperation? ns resultType .logicalOr arguments
          unit.targetPointerWidth := by
  funext arguments frame state
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    evaluatePrimitiveOperation?, type_eq]
  rfl

theorem NominalConstructor.evaluator_eq
    {unit : ValidatedUnit} {namespaceId : NamespaceId}
    {reference : QualifiedRef} {variant : Option String}
    (constructor : NominalConstructor)
    (variant_eq : constructor.variant = variant)
    (resolve_eq : resolveStruct? unit namespaceId reference =
      some constructor.source)
    (fields_eq : (constructorFields? unit constructor.source variant).map
      Array.size = some constructor.arity) :
    constructor.evaluate? = liftConstructorEvaluator fun values =>
      constructNominal? unit namespaceId reference variant values := by
  funext arguments frame state
  simp only [NominalConstructor.evaluate?, liftConstructorEvaluator,
    constructNominal?, resolve_eq, Option.bind_some]
  cases payload_eq : constructorFields? unit constructor.source variant with
  | none => simp [payload_eq] at fields_eq
  | some fields =>
      simp only [payload_eq, Option.map_some, Option.some.injEq] at fields_eq
      simp only [payload_eq, Option.bind_some, Option.pure_def, bind,
        variant_eq, ← fields_eq]
      by_cases same : fields.size = arguments.size <;> simp [same]

/-- Transport a constructor certificate across the executable unit equality
once, before resolving any source identities.  This keeps certificate
elaboration closed: lowering supplies facts about `prepared`, never lookup
goals containing the symbolic executable unit. -/
theorem NominalConstructor.evaluator_eq_of_unit
    {actual prepared : ValidatedUnit} {namespaceId : NamespaceId}
    {reference : QualifiedRef} {variant : Option String}
    (unit_eq : actual = prepared) (constructor : NominalConstructor)
    (variant_eq : constructor.variant = variant)
    (resolve_eq : resolveStruct? prepared namespaceId reference =
      some constructor.source)
    (fields_eq : (constructorFields? prepared constructor.source variant).map
      Array.size = some constructor.arity) :
    constructor.evaluate? = liftConstructorEvaluator fun values =>
      constructNominal? actual namespaceId reference variant values := by
  subst actual
  exact constructor.evaluator_eq variant_eq resolve_eq fields_eq

theorem NominalFieldLocation.select_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId}
    {reference : QualifiedRef} {fieldName : String}
    (field : NominalFieldLocation)
    (resolved_eq : resolveStruct? unit ns.identity reference =
      some field.source)
    (index_eq : ∀ actualVariant,
      handleFieldIndex? unit field.source actualVariant fieldName =
        if actualVariant == field.variant then some field.index else none) :
    field.evaluateSelect? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? unit ns resultType site
          (.data (.select reference fieldName)) arguments frame state := by
  funext arguments frame state
  have data_eq : evaluateDataOperation? unit ns.identity (.select reference fieldName)
      arguments = selectNominalFieldAt? field.source field.variant field.index
        arguments := evaluateDataOperation?_select_at resolved_eq index_eq arguments
  cases selected_eq : selectNominalFieldAt? field.source field.variant field.index
      arguments <;>
    simp [NominalFieldLocation.evaluateSelect?, liftConstructorEvaluator,
      liftPlaceEvaluator, evaluatePlaceOperation?, data_eq, selected_eq]

/-- Resolve field ownership and position against the closed prepared unit,
then transport the resulting evaluator equality to the executable view. -/
theorem NominalFieldLocation.select_evaluator_eq_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId}
    {reference : QualifiedRef} {fieldName : String}
    (unit_eq : actual = prepared) (field : NominalFieldLocation)
    (resolved_eq : resolveStruct? prepared ns.identity reference =
      some field.source)
    (index_eq : ∀ actualVariant,
      handleFieldIndex? prepared field.source actualVariant fieldName =
        if actualVariant == field.variant then some field.index else none) :
    field.evaluateSelect? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? actual ns resultType site
          (.data (.select reference fieldName)) arguments frame state := by
  subst actual
  exact field.select_evaluator_eq resolved_eq index_eq

theorem NominalVariantFieldLocation.select_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId}
    {reference : QualifiedRef} {fieldNames : Array String}
    (field : NominalVariantFieldLocation)
    (resolved_eq : resolveStruct? unit ns.identity reference =
      some field.source)
    (choices_eq : variantFieldChoices? unit field.source fieldNames =
      some field.choices) :
    field.evaluateSelect? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? unit ns resultType site
          (.data (.selectVariants reference fieldNames)) arguments frame state := by
  funext arguments frame state
  have data_eq : evaluateDataOperation? unit ns.identity
      (.selectVariants reference fieldNames) arguments =
      selectNominalVariantFieldAt? field.source field.choices arguments :=
    evaluateDataOperation?_selectVariants_at resolved_eq choices_eq arguments
  cases selected_eq : selectNominalVariantFieldAt? field.source field.choices
      arguments <;>
    simp [NominalVariantFieldLocation.evaluateSelect?, liftConstructorEvaluator,
      liftPlaceEvaluator, evaluatePlaceOperation?, data_eq, selected_eq]

theorem NominalVariantFieldLocation.select_evaluator_eq_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId}
    {reference : QualifiedRef} {fieldNames : Array String}
    (unit_eq : actual = prepared) (field : NominalVariantFieldLocation)
    (resolved_eq : resolveStruct? prepared ns.identity reference =
      some field.source)
    (choices_eq : variantFieldChoices? prepared field.source fieldNames =
      some field.choices) :
    field.evaluateSelect? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? actual ns resultType site
          (.data (.selectVariants reference fieldNames)) arguments frame state := by
  subst actual
  exact field.select_evaluator_eq resolved_eq choices_eq

theorem NominalVariantTest.evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {reference : QualifiedRef}
    (test : NominalVariantTest)
    (resolved_eq : resolveStruct? unit ns.identity reference = some test.source) :
    test.evaluate? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? unit ns resultType site
          (.data (.testVariants reference test.variants)) arguments frame state := by
  funext arguments frame state
  have data_eq := evaluateDataOperation?_testVariants_at
    (variants := test.variants) resolved_eq arguments
  cases tested_eq : testNominalVariants? test.source test.variants arguments <;>
    simp [NominalVariantTest.evaluate?, liftConstructorEvaluator,
      liftPlaceEvaluator, evaluatePlaceOperation?, data_eq, tested_eq]

theorem NominalVariantTest.evaluator_eq_of_unit
    {actual prepared : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {reference : QualifiedRef}
    (unit_eq : actual = prepared) (test : NominalVariantTest)
    (resolved_eq : resolveStruct? prepared ns.identity reference = some test.source) :
    test.evaluate? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? actual ns resultType site
          (.data (.testVariants reference test.variants)) arguments frame state := by
  subst actual
  exact test.evaluator_eq resolved_eq

theorem ReferenceLocationOperation.dereference_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} :
    ReferenceLocationOperation.dereference.evaluate? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? unit ns resultType site
          (.reference .dereference) arguments frame state := by
  funext arguments frame state
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    evaluatePlaceOperation?]

theorem ReferenceLocationOperation.mutate_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} :
    ReferenceLocationOperation.mutate.evaluate? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? unit ns resultType site
          (.reference .mutate) arguments frame state := by
  funext arguments frame state
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    evaluatePlaceOperation?]

theorem ReferenceLocationOperation.freeze_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {explicit : Bool}
    (referenceType : ReferenceType)
    (result_type_eq : ns.tables.types[resultType.index]? =
      some (.reference referenceType)) :
    (ReferenceLocationOperation.freeze referenceType).evaluate? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? unit ns resultType site
          (.reference (.freeze explicit)) arguments frame state := by
  funext arguments frame state
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    evaluatePlaceOperation?, result_type_eq]

theorem ReferenceLocationOperation.endLoan_evaluator_eq
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} (loans : Array LoanId) :
    (ReferenceLocationOperation.endLoan loans).evaluate? =
      liftPlaceEvaluator fun arguments frame state =>
        evaluatePlaceOperation? unit ns resultType site
          (.reference (.endLoan loans)) arguments frame state := by
  funext arguments frame state
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    evaluatePlaceOperation?]

/-- A profile-independent primitive operation. -/
def primitive (ns : ValidatedNamespace) (resultType : TypeId)
    (operation : PrimitiveOperation) (operands : ValuesDenotation)
    (pointerWidth : Option Nat) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (∃ operandFrame operandState propagated,
      operands frame state (.control operandState operandFrame propagated) ∧
      finalFrame = operandFrame ∧ finalState = operandState ∧ control = propagated) ∨
    (∃ operandFrame operandState values,
      operands frame state (.values operandState operandFrame values) ∧
      ((∃ runtimeValue,
          evaluatePrimitiveOperation? ns resultType operation values.toArray pointerWidth =
            some (.ok runtimeValue) ∧
          finalFrame = operandFrame ∧ finalState = operandState ∧
            control = .value runtimeValue) ∨
       (∃ kind thrown,
          evaluatePrimitiveOperation? ns resultType operation values.toArray pointerWidth =
            some (.error (kind, thrown)) ∧
          finalFrame = operandFrame ∧ finalState = operandState ∧
            control = .throw_ kind thrown)))

/-- One keyed global-storage operation. -/
def global (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (resultType : TypeId) (site : ExprId) (kind : GlobalKind)
    (instantiations : Array GenericArgument) (operands : ValuesDenotation) :
    ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (∃ operandFrame operandState propagated,
      operands frame state (.control operandState operandFrame propagated) ∧
      finalFrame = operandFrame ∧ finalState = operandState ∧ control = propagated) ∨
    (∃ operandFrame operandState values,
      operands frame state (.values operandState operandFrame values) ∧
      ((∃ runtimeValue,
          evaluateGlobalOperation? unit ns resultType site kind instantiations
              values.toArray operandFrame operandState =
            some (.value finalFrame finalState runtimeValue) ∧
          control = .value runtimeValue) ∨
       (∃ throwKind thrown,
          evaluateGlobalOperation? unit ns resultType site kind instantiations
              values.toArray operandFrame operandState =
            some (.throw_ finalFrame finalState throwKind thrown) ∧
          control = .throw_ throwKind thrown)))

/-- A non-call, non-global operation implemented by native place semantics
(field selection, dereference, mutation, and their V1 peers). -/
def placeOperation (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (resultType : TypeId) (site : ExprId) (operation : Operation)
    (operands : ValuesDenotation) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (∃ operandFrame operandState propagated,
      operands frame state (.control operandState operandFrame propagated) ∧
      finalFrame = operandFrame ∧ finalState = operandState ∧ control = propagated) ∨
    (∃ operandFrame operandState values runtimeValue,
      operands frame state (.values operandState operandFrame values) ∧
      evaluatePlaceOperation? unit ns resultType site operation values.toArray
          operandFrame operandState = some (finalFrame, finalState, runtimeValue) ∧
      control = .value runtimeValue)

/-- Construction of a nominal value is a native combinator rather than an
arena-dispatched call. -/
def constructor (unit : ValidatedUnit) (namespaceId : NamespaceId)
    (reference : QualifiedRef) (variant : Option String)
    (operands : ValuesDenotation) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (∃ operandFrame operandState propagated,
      operands frame state (.control operandState operandFrame propagated) ∧
      finalFrame = operandFrame ∧ finalState = operandState ∧ control = propagated) ∨
    (∃ operandFrame operandState values runtimeValue,
      operands frame state (.values operandState operandFrame values) ∧
      constructNominal? unit namespaceId reference variant values.toArray =
        some runtimeValue ∧
      finalFrame = operandFrame ∧ finalState = operandState ∧
        control = .value runtimeValue)

/-- Lift a completed function outcome into the control value observed by its
caller. -/
def callControl : Outcome → Control
  | .returned results => .value (packResults results)
  | .threw kind thrown => .throw_ kind thrown

/-- Caller frame after a call. Only a returned outcome can introduce the
checked call-produced lexical loan. -/
def callFrame (lexical : Option Nat) : Outcome → RuntimeFrame → RuntimeFrame
  | .returned results, frame => registerReturnedLoan lexical results frame
  | .threw .., frame => frame

@[simp] theorem callFrame_returned (lexical : Option Nat)
    (results : Array RuntimeValue) (frame : RuntimeFrame) :
    callFrame lexical (.returned results) frame =
      registerReturnedLoan lexical results frame := rfl

@[simp] theorem callFrame_threw (lexical : Option Nat) (kind : ThrowKind)
    (arguments : Array RuntimeValue) (frame : RuntimeFrame) :
    callFrame lexical (.threw kind arguments) frame = frame := rfl

/-- An acyclic direct call.  The callee is supplied as a native relation, so
the term is independent of the expression arena while still retaining the
callee state needed by `applyPending`.  Recursive SCCs require a separate
fixed-point family and are deliberately not represented by this combinator. -/
def call (lexical : Option Nat) (callee : FunctionDenotation)
    (operands : ValuesDenotation) :
    ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (∃ operandFrame operandState propagated,
      operands frame state (.control operandState operandFrame propagated) ∧
      finalFrame = operandFrame ∧ finalState = operandState ∧
        control = propagated) ∨
    ∃ operandFrame operandState values calleeState outcome,
      operands frame state (.values operandState operandFrame values) ∧
      callee operandState values.toArray calleeState outcome ∧
      finalFrame = callFrame lexical outcome
        (applyPendingFrom operandState.pending operandFrame calleeState).1 ∧
      finalState = (applyPendingFrom operandState.pending operandFrame calleeState).2 ∧
      control = callControl outcome

/-- A direct call whose callee relation is selected using the invocation
substitution carried by the caller frame. -/
def callAt (lexical : Option Nat)
    (callee : Array (TypeId × TypeId) → FunctionDenotation)
    (operands : ValuesDenotation) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (∃ operandFrame operandState propagated,
      operands frame state (.control operandState operandFrame propagated) ∧
      finalFrame = operandFrame ∧ finalState = operandState ∧
        control = propagated) ∨
    ∃ operandFrame operandState values calleeState outcome,
      operands frame state (.values operandState operandFrame values) ∧
      callee operandFrame.typeInstantiation operandState values.toArray calleeState outcome ∧
      finalFrame = callFrame lexical outcome
        (applyPendingFrom operandState.pending operandFrame calleeState).1 ∧
      finalState = (applyPendingFrom operandState.pending operandFrame calleeState).2 ∧
      control = callControl outcome

/-- A lowered direct call.  `handle` is the result of name resolution and is
retained natively so agreement never has to rediscover it from the source
namespace.  Runtime behavior is still entirely determined by the supplied
callee relation. -/
def nativeCall (_handle : FunctionHandle) (lexical : Option Nat)
    (callee : FunctionDenotation)
    (operands : ValuesDenotation) : ExprDenotation :=
  call lexical callee operands

/-- Lowered invocation-aware direct call. -/
def nativeCallAt (_handle : FunctionHandle) (lexical : Option Nat)
    (callee : Array (TypeId × TypeId) → FunctionDenotation)
    (operands : ValuesDenotation) : ExprDenotation :=
  callAt lexical callee operands

/-! ## Function control combinators -/

/-- Evaluate a native row of return values and turn its successful result
into explicit function-return control.  Abrupt control raised while evaluating
the row propagates unchanged. -/
def nativeReturn (values : ValuesDenotation) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (∃ runtimeValues,
      values frame state (.values finalState finalFrame runtimeValues) ∧
      control = .return_ runtimeValues.toArray) ∨
    values frame state (.control finalState finalFrame control)

/-- Evaluate a native row of throw arguments and turn its successful result
into explicit throw control.  Abrupt control raised while evaluating the row
propagates unchanged. -/
def nativeThrow (kind : ThrowKind) (arguments : ValuesDenotation) :
    ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (∃ runtimeValues,
      arguments frame state (.values finalState finalFrame runtimeValues) ∧
      control = .throw_ kind runtimeValues.toArray) ∨
    arguments frame state (.control finalState finalFrame control)

/-- Raise a loop break after optionally evaluating its result. -/
def nativeBreak (nest : Nat) (value : Option ExprDenotation) : ExprDenotation :=
  match value with
  | none => fun frame state finalFrame finalState control =>
      finalFrame = frame ∧ finalState = state ∧ control = .break_ nest none
  | some value => fun frame state finalFrame finalState control =>
      (∃ runtimeValue,
        value frame state finalFrame finalState (.value runtimeValue) ∧
        control = .break_ nest (some runtimeValue)) ∨
      (value frame state finalFrame finalState control ∧ Abrupt control)

/-- Raise a loop continue without consulting an expression arena. -/
def nativeContinue (nest : Nat) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    finalFrame = frame ∧ finalState = state ∧ control = .continue_ nest

/-- Runtime erasure of an embedded specification block. -/
def nativeSpec : ExprDenotation := value .unit

/-- Assign a successfully evaluated value to one closed local slot. -/
def nativeAssignLocal (localId : LocalId) (value : ExprDenotation) :
    ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (value frame state finalFrame finalState control ∧ Abrupt control) ∨
    ∃ valueFrame valueState runtimeValue,
      value frame state valueFrame valueState (.value runtimeValue) ∧
      localId.index < valueFrame.locals.size ∧
      finalFrame = { valueFrame with
        locals := valueFrame.locals.set! localId.index (some runtimeValue) } ∧
      finalState = valueState ∧ control = .value .unit

/-- Assign through a checked index of a vector or tuple held in a local.
The source place and index expression have been erased to their two local
slots; the compact resolver performs only the dynamic shape and bounds work. -/
def nativeAssignLocalIndex (base index : LocalId) (value : ExprDenotation) :
    ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (value frame state finalFrame finalState control ∧ Abrupt control) ∨
    ∃ valueFrame valueState runtimeValue resolved,
      value frame state valueFrame valueState (.value runtimeValue) ∧
      resolveLocalIndex? base index valueFrame = some resolved ∧
      writeRuntimePlace? valueFrame valueState resolved runtimeValue =
        some (finalFrame, finalState) ∧
      control = .value .unit

/-- Least finite native loop relation.  Recursive occurrences are explicit
Lean constructors, so verification reasons by the stated invariant and never
symbolically unrolls an expression id. -/
inductive NativeLoop (body : ExprDenotation) : ExprDenotation where
  | repeatValue (value : RuntimeValue) (control : Control)
      (body_step : body frame state bodyFrame bodyState (.value value))
      (repeat_step : NativeLoop body bodyFrame bodyState finalFrame finalState control) :
      NativeLoop body frame state finalFrame finalState control
  | repeatContinue (control : Control)
      (body_step : body frame state bodyFrame bodyState (.continue_ 0))
      (repeat_step : NativeLoop body bodyFrame bodyState finalFrame finalState control) :
      NativeLoop body frame state finalFrame finalState control
  | outerContinue (nest : Nat)
      (body_step : body frame state finalFrame finalState (.continue_ (nest + 1))) :
      NativeLoop body frame state finalFrame finalState (.continue_ nest)
  | break_ (breakValue : Option RuntimeValue)
      (body_step : body frame state finalFrame finalState (.break_ 0 breakValue)) :
      NativeLoop body frame state finalFrame finalState
        (.value (breakValue.getD .unit))
  | outerBreak (nest : Nat) (breakValue : Option RuntimeValue)
      (body_step : body frame state finalFrame finalState
        (.break_ (nest + 1) breakValue)) :
      NativeLoop body frame state finalFrame finalState (.break_ nest breakValue)
  | return_ (values : Array RuntimeValue)
      (body_step : body frame state finalFrame finalState (.return_ values)) :
      NativeLoop body frame state finalFrame finalState (.return_ values)
  | throw_ (kind : ThrowKind) (arguments : Array RuntimeValue)
      (body_step : body frame state finalFrame finalState (.throw_ kind arguments)) :
      NativeLoop body frame state finalFrame finalState (.throw_ kind arguments)

/-- Native loop combinator. `site` is a proof-only static tag used to select
the source-authored invariant; evaluation never looks it up in an arena. -/
def nativeLoop (_site : ExprId) (body : ExprDenotation) : ExprDenotation :=
  NativeLoop body

/-- Opaque motive used to invoke the mutual big-step recursor for the reverse
half of loop agreement.  Keeping it folded lets Lean recognize the eliminator
motive instead of reducing the target before induction starts. -/
def LoopConvertible (unit : ExecutableUnit) (callee : CalleeRelation)
    (targetNamespace : NamespaceId)
    (target bodyId : ExprId) (body : ExprDenotation)
    {namespaceId : NamespaceId}
    {frame finalFrame : RuntimeFrame} {state finalState : RuntimeState}
    {exprId : ExprId} {control : Control}
    (_step : EvalExprWith unit callee namespaceId frame state exprId finalFrame
      finalState control) :
    Prop :=
  namespaceId = targetNamespace → exprId = target →
    ExprDenotation.AgreesWith unit callee targetNamespace bodyId body →
    NativeLoop body frame state finalFrame finalState control

attribute [irreducible] LoopConvertible

/-! ## Straight-line control combinators -/

/-- A block without a result expression. -/
def blockUnit (statements : StatementsDenotation) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    statements frame state (.control finalState finalFrame control) ∨
    (statements frame state (.done finalState finalFrame) ∧ control = .value .unit)

/-- A block whose result is evaluated after all statements complete. -/
def blockResult (statements : StatementsDenotation) (result : ExprDenotation) :
    ExprDenotation :=
  fun frame state finalFrame finalState control =>
    statements frame state (.control finalState finalFrame control) ∨
    ∃ statementFrame statementState,
      statements frame state (.done statementState statementFrame) ∧
      result statementFrame statementState finalFrame finalState control

/-- A branch.  The condition runs first: abrupt control propagates, and
otherwise the boolean it produced selects a branch.  A branch with no else
arm produces unit on the false side, which is the shape a statement-position
`if` takes. -/
def nativeBranch (condition thenBranch : ExprDenotation)
    (elseBranch : Option ExprDenotation) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (condition frame state finalFrame finalState control ∧ Abrupt control) ∨
    (∃ conditionFrame conditionState,
      condition frame state conditionFrame conditionState (.value (.bool true)) ∧
      thenBranch conditionFrame conditionState finalFrame finalState control) ∨
    (∃ conditionFrame conditionState,
      condition frame state conditionFrame conditionState (.value (.bool false)) ∧
      match elseBranch with
      | some elseBranch =>
          elseBranch conditionFrame conditionState finalFrame finalState control
      | none =>
          finalFrame = conditionFrame ∧ finalState = conditionState ∧
            control = .value .unit)

/-- A declaration without an initializer is just its body. -/
def letNoValue (body : ExprDenotation) : ExprDenotation := body

/-- Bind through a closed native pattern.  The binder contains direct local
slots and constructor spellings; neither this term nor its WP consults the
namespace's pattern arena. -/
def letNativeValue (binder : NativePatternBinder)
    (initializer body : ExprDenotation) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (initializer frame state finalFrame finalState control ∧ Abrupt control) ∨
    ∃ initializedFrame initializedState runtimeValue boundFrame,
      initializer frame state initializedFrame initializedState (.value runtimeValue) ∧
      binder.bind initializedFrame runtimeValue = some boundFrame ∧
      body boundFrame initializedState finalFrame finalState control

/-- Bind the value produced by an initializer, or propagate its abrupt
control, before entering the native body term. -/
def letValue (unit : ValidatedUnit) (ns : ValidatedNamespace) (pattern : PatternId)
    (initializer body : ExprDenotation) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    (initializer frame state finalFrame finalState control ∧ Abrupt control) ∨
    ∃ initializedFrame initializedState runtimeValue boundFrame,
      initializer frame state initializedFrame initializedState (.value runtimeValue) ∧
      bindPattern unit ns initializedFrame pattern runtimeValue = some boundFrame ∧
      body boundFrame initializedState finalFrame finalState control

theorem letNativeValue_eq_letValue {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {pattern : PatternId} {binder : NativePatternBinder}
    (lower_eq : lowerPattern? unit ns pattern = some binder)
    (initializer body : ExprDenotation) :
    letNativeValue binder initializer body =
      letValue unit ns pattern initializer body := by
  funext frame state finalFrame finalState control
  apply propext
  simpa only [letNativeValue, letValue, lowerPattern?_bind_eq lower_eq]

/-! ## Function boundary -/

/-- Turn a native body into the state-retaining whole-function relation used
by direct callers. -/
def functionRelation (unit : ExecutableUnit)
    (declaration : FunctionDecl FunctionBody) (body : ExprDenotation) :
    FunctionDenotation :=
  fun initial arguments final outcome =>
    ∃ frame finalFrame evaluatedState control,
      initialFrame? declaration arguments = some frame ∧
      body frame initial finalFrame evaluatedState control ∧
      finishControl? declaration.signature.results.size control = some outcome ∧
      finalizeFunctionState unit declaration.profile initial evaluatedState
          finalFrame outcome = final

/-- Turn a native body relation into the ordinary function `Spec`.  The
wrapper retains exactly the reference function-boundary behavior, including
loan finalization and rollback-visible failure outcomes. -/
def function (unit : ExecutableUnit) (declaration : FunctionDecl FunctionBody)
    (body : ExprDenotation) (arguments : Array RuntimeValue) :
    Spec RuntimeState Failure (Array RuntimeValue) where
  ok := fun initial results final =>
    ∃ frame finalFrame evaluatedState control,
      initialFrame? declaration arguments = some frame ∧
      body frame initial finalFrame evaluatedState control ∧
      finishControl? declaration.signature.results.size control =
        some (.returned results) ∧
      finalizeFunctionState unit declaration.profile initial evaluatedState
          finalFrame (.returned results) = final
  aborts := fun initial failure =>
    ∃ frame finalFrame evaluatedState control final,
      initialFrame? declaration arguments = some frame ∧
      body frame initial finalFrame evaluatedState control ∧
      finishControl? declaration.signature.results.size control =
        some (.threw failure.1 failure.2) ∧
      finalizeFunctionState unit declaration.profile initial evaluatedState
          finalFrame (.threw failure.1 failure.2) = final

/-- State-retaining function boundary over the runtime projection produced by
lowering.  The generated relation contains no declaration lookup or projection. -/
def nativeFunctionRelationAt (unit : ExecutableUnit) (shape : FunctionShape)
    (typeInstantiation : Array (TypeId × TypeId))
    (body : ExprDenotation) : FunctionDenotation :=
  fun initial arguments final outcome =>
    ∃ frame finalFrame evaluatedState control,
      nativeInitialFrame? shape arguments typeInstantiation = some frame ∧
      body frame initial finalFrame evaluatedState control ∧
      finishControl? shape.resultCount control = some outcome ∧
      finalizeFunctionState unit shape.profile initial evaluatedState
          finalFrame outcome = final

/-- The ordinary invocation has the identity type substitution. -/
def nativeFunctionRelation (unit : ExecutableUnit) (shape : FunctionShape)
    (body : ExprDenotation) : FunctionDenotation :=
  nativeFunctionRelationAt unit shape #[] body

/-- Public `Spec` boundary over a lowered function shape and an explicit
invocation type substitution. -/
def nativeFunctionAt (unit : ExecutableUnit) (shape : FunctionShape)
    (typeInstantiation : Array (TypeId × TypeId))
    (body : ExprDenotation) (arguments : Array RuntimeValue) :
    Spec RuntimeState Failure (Array RuntimeValue) where
  ok := fun initial results final =>
    ∃ frame finalFrame evaluatedState control,
      nativeInitialFrame? shape arguments typeInstantiation = some frame ∧
      body frame initial finalFrame evaluatedState control ∧
      finishControl? shape.resultCount control = some (.returned results) ∧
      finalizeFunctionState unit shape.profile initial evaluatedState
          finalFrame (.returned results) = final
  aborts := fun initial failure =>
    ∃ frame finalFrame evaluatedState control final,
      nativeInitialFrame? shape arguments typeInstantiation = some frame ∧
      body frame initial finalFrame evaluatedState control ∧
      finishControl? shape.resultCount control =
        some (.threw failure.1 failure.2) ∧
      finalizeFunctionState unit shape.profile initial evaluatedState
          finalFrame (.threw failure.1 failure.2) = final

/-- Public invocation with the identity type substitution. -/
def nativeFunction (unit : ExecutableUnit) (shape : FunctionShape)
    (body : ExprDenotation) (arguments : Array RuntimeValue) :
    Spec RuntimeState Failure (Array RuntimeValue) :=
  nativeFunctionAt unit shape #[] body arguments

theorem nativeFunctionRelation_ofDeclaration (unit : ExecutableUnit)
    (declaration : FunctionDecl FunctionBody) (body : ExprDenotation) :
    nativeFunctionRelation unit (.ofDeclaration declaration) body =
      functionRelation unit declaration body := by
  rfl

theorem nativeFunction_ofDeclaration (unit : ExecutableUnit)
    (declaration : FunctionDecl FunctionBody) (body : ExprDenotation)
    (arguments : Array RuntimeValue) :
    nativeFunction unit (.ofDeclaration declaration) body arguments =
      function unit declaration body arguments := by
  rfl

/-! ## Agreement library

Every lemma is stated under an arbitrary callee oracle `callee`; the closed
notions are the instance at the closed semantics. -/

variable {callee : CalleeRelation}

private theorem evaluatePlaceOperation?_primitive
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId}
    {operation : PrimitiveOperation} {arguments : Array RuntimeValue}
    {frame : RuntimeFrame} {state : RuntimeState} :
    evaluatePlaceOperation? unit ns resultType site (.primitive operation)
      arguments frame state = none := rfl

private theorem evaluatePlaceOperation?_global
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {kind : GlobalKind}
    {arguments : Array RuntimeValue} {frame : RuntimeFrame}
    {state : RuntimeState} :
    evaluatePlaceOperation? unit ns resultType site (.global kind)
      arguments frame state = none := rfl

private theorem evaluatePlaceOperation?_call
    {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {resultType : TypeId} {site : ExprId} {kind : CallKind}
    {arguments : Array RuntimeValue} {frame : RuntimeFrame}
    {state : RuntimeState} :
    evaluatePlaceOperation? unit ns resultType site (.call kind)
      arguments frame state = none := rfl

theorem value_agrees {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {exprId : ExprId} {ns : ValidatedNamespace} {expression : Expr}
    {literal : ConstValue} {source : Option String}
    {runtimeValue : RuntimeValue}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .value literal source)
    (value_eq : constValue? literal = some runtimeValue) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId (value runtimeValue) := by
  intro frame state finalFrame finalState control
  constructor
  · rintro ⟨finalFrame_eq, finalState_eq, control_eq⟩
    subst finalFrame
    subst finalState
    subst control
    exact .value namespaceId frame state exprId ns expression literal source runtimeValue
      namespace_eq expression_eq kind_eq value_eq
  · intro step
    cases step <;> simp_all [value]

theorem local_agrees {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {exprId : ExprId} {ns : ValidatedNamespace} {expression : Expr}
    {localId : LocalId}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .localVar localId) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId (localVar localId) := by
  intro frame state finalFrame finalState control
  constructor
  · rintro ⟨runtimeValue, local_eq, finalFrame_eq, finalState_eq, control_eq⟩
    subst finalFrame
    subst finalState
    subst control
    exact .localVar namespaceId frame state exprId ns expression localId runtimeValue
      namespace_eq expression_eq kind_eq local_eq
  · intro step
    cases step <;> simp_all [localVar]

/-- A constant whose initializer is a literal denotes as that literal's
value: the big-step rule evaluates the initializer in an empty frame and
keeps the caller's frame, which for a literal is exactly `value`. -/
theorem constant_agrees {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {exprId : ExprId} {ns : ValidatedNamespace} {expression : Expr}
    {reference : QualifiedRef} {handle : ConstantHandle}
    {targetNs : ValidatedNamespace} {declaration : ConstantDecl}
    {initializer : Expr} {literal : ConstValue} {source : Option String}
    {runtimeValue : RuntimeValue}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .constant reference)
    (resolve_eq : resolveConstant? unit.unit namespaceId reference = some handle)
    (target_namespace_eq : unit.unit.namespaces[handle.namespaceId.index]? = some targetNs)
    (declaration_eq : targetNs.constants[handle.constantId]? = some declaration)
    (initializer_eq : targetNs.expressions[declaration.value.index]? = some initializer)
    (initializer_kind_eq : initializer.kind = .value literal source)
    (value_eq : constValue? literal = some runtimeValue) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId (value runtimeValue) := by
  intro frame state finalFrame finalState control
  constructor
  · rintro ⟨finalFrame_eq, finalState_eq, control_eq⟩
    subst finalFrame
    subst finalState
    subst control
    exact .constantValue namespaceId frame state exprId ns expression reference handle
      targetNs declaration { locals := #[] } state runtimeValue
      namespace_eq expression_eq kind_eq resolve_eq target_namespace_eq declaration_eq
      (.value handle.namespaceId { locals := #[] } state declaration.value targetNs
        initializer literal source runtimeValue target_namespace_eq initializer_eq
        initializer_kind_eq value_eq)
  · intro step
    cases step <;> simp_all [value]
    all_goals first
      | (rename_i initializerStep
         cases initializerStep <;> simp_all [value])
      | (rename_i initializerStep abrupt
         cases initializerStep <;> simp_all [value] <;> cases abrupt)

/-- Constant folding preserves a constant's initializer semantics in the
empty frame, and retains the caller's frame. -/
theorem constant_computed_agrees {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {exprId : ExprId} {ns : ValidatedNamespace} {expression : Expr}
    {reference : QualifiedRef} {handle : ConstantHandle}
    {targetNs : ValidatedNamespace} {declaration : ConstantDecl}
    {runtimeValue : RuntimeValue}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .constant reference)
    (resolve_eq : resolveConstant? unit.unit namespaceId reference = some handle)
    (target_namespace_eq : unit.unit.namespaces[handle.namespaceId.index]? = some targetNs)
    (declaration_eq : targetNs.constants[handle.constantId]? = some declaration)
    (initializer_agrees : ExprDenotation.AgreesWith unit callee handle.namespaceId
      declaration.value (value runtimeValue)) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId (value runtimeValue) := by
  intro frame state finalFrame finalState control
  constructor
  · rintro ⟨frame_eq, state_eq, control_eq⟩
    subst finalFrame
    subst finalState
    subst control
    exact .constantValue namespaceId frame state exprId ns expression reference handle
      targetNs declaration { locals := #[] } state runtimeValue
      namespace_eq expression_eq kind_eq resolve_eq target_namespace_eq declaration_eq
      ((initializer_agrees _ _ _ _ _).mp ⟨rfl, rfl, rfl⟩)
  · intro step
    cases step <;> simp_all
    all_goals subst_vars
    all_goals
      have evaluated := (initializer_agrees _ _ _ _ _).mpr (by assumption)
      simp_all [value]

/-- A closed row of constant values. This occurs only in folding certificates. -/
def literalValues (values : List RuntimeValue) : ValuesDenotation :=
  fun frame state result => result = .values state frame values

theorem literalValues_nil_agrees {unit : ExecutableUnit} {namespaceId : NamespaceId} :
    ValuesDenotation.AgreesWith unit callee namespaceId [] (literalValues []) := by
  intro frame state result
  constructor
  · rintro rfl; exact .nil namespaceId frame state
  · intro step; cases step; rfl

theorem literalValues_cons_agrees {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {head : ExprId} {tail : List ExprId} {headValue : RuntimeValue}
    {tailValues : List RuntimeValue}
    (head_agrees : ExprDenotation.AgreesWith unit callee namespaceId head (value headValue))
    (tail_agrees : ValuesDenotation.AgreesWith unit callee namespaceId tail
      (literalValues tailValues)) :
    ValuesDenotation.AgreesWith unit callee namespaceId (head :: tail)
      (literalValues (headValue :: tailValues)) := by
  intro frame state result
  constructor
  · rintro rfl
    exact .tailValues namespaceId frame state head tail frame state headValue frame state
      tailValues ((head_agrees _ _ _ _ _).mp ⟨rfl, rfl, rfl⟩)
      ((tail_agrees _ _ _).mp rfl)
  · intro step
    cases step with
    | headControl _ _ _ _ _ _ _ _ ran abrupt =>
        obtain ⟨rfl, rfl, rfl⟩ := (head_agrees _ _ _ _ _).mpr ran
        cases abrupt
    | tailValues _ _ _ _ _ _ _ _ _ _ _ ran tailRan =>
        obtain ⟨rfl, rfl, equal⟩ := (head_agrees _ _ _ _ _).mpr ran
        cases equal
        have equal := (tail_agrees _ _ _).mpr tailRan
        cases equal
        rfl
    | tailControl _ _ _ _ _ _ _ _ _ _ _ _ tailRan =>
        have equal := (tail_agrees _ _ _).mpr tailRan
        cases equal

theorem valuesNil_agrees {unit : ExecutableUnit} {namespaceId : NamespaceId} :
    ValuesDenotation.AgreesWith unit callee namespaceId [] valuesNil := by
  intro frame state result
  constructor
  · rintro rfl
    exact .nil namespaceId frame state
  · intro step
    cases step
    rfl

theorem valuesCons_agrees {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {expression : ExprId} {expressions : List ExprId}
    {head : ExprDenotation} {tail : ValuesDenotation}
    (head_agrees : ExprDenotation.AgreesWith unit callee namespaceId expression head)
    (tail_agrees : ValuesDenotation.AgreesWith unit callee namespaceId expressions tail) :
    ValuesDenotation.AgreesWith unit callee namespaceId (expression :: expressions)
      (valuesCons head tail) := by
  intro frame state result
  constructor
  · intro step
    rcases step with ⟨finalFrame, finalState, control, headStep, abrupt, rfl⟩ |
      ⟨headFrame, headState, runtimeValue, finalFrame, finalState, values,
        headStep, tailStep, rfl⟩ |
      ⟨headFrame, headState, runtimeValue, finalFrame, finalState, control,
        headStep, tailStep, rfl⟩
    · exact .headControl namespaceId frame state expression expressions
        finalFrame finalState control
        ((head_agrees _ _ _ _ _).mp headStep) abrupt
    · exact .tailValues namespaceId frame state expression expressions
        headFrame headState runtimeValue finalFrame finalState values
        ((head_agrees _ _ _ _ _).mp headStep)
        ((tail_agrees _ _ _).mp tailStep)
    · exact .tailControl namespaceId frame state expression expressions
        headFrame headState runtimeValue finalFrame finalState control
        ((head_agrees _ _ _ _ _).mp headStep)
        ((tail_agrees _ _ _).mp tailStep)
  · intro step
    cases step with
    | headControl _ _ _ _ _ finalFrame finalState control headStep abrupt =>
        exact Or.inl ⟨finalFrame, finalState, control,
          (head_agrees _ _ _ _ _).mpr headStep, abrupt, rfl⟩
    | tailValues _ _ _ _ _ headFrame headState runtimeValue _ _ _ headStep tailStep =>
        exact Or.inr <| Or.inl ⟨headFrame, headState, runtimeValue, _, _, _,
          (head_agrees _ _ _ _ _).mpr headStep,
          (tail_agrees _ _ _).mpr tailStep, rfl⟩
    | tailControl _ _ _ _ _ headFrame headState runtimeValue _ _ _ headStep tailStep =>
        exact Or.inr <| Or.inr ⟨headFrame, headState, runtimeValue, _, _, _,
          (head_agrees _ _ _ _ _).mpr headStep,
          (tail_agrees _ _ _).mpr tailStep, rfl⟩

theorem statementsNil_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} :
    StatementsDenotation.AgreesWith unit callee namespaceId [] statementsNil := by
  intro frame state result
  constructor
  · rintro rfl
    exact .nil namespaceId frame state
  · intro step
    cases step
    rfl

theorem statementsCons_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {statement : ExprId}
    {statements : List ExprId} {head : ExprDenotation}
    {tail : StatementsDenotation}
    (head_agrees : ExprDenotation.AgreesWith unit callee namespaceId statement head)
    (tail_agrees : StatementsDenotation.AgreesWith unit callee namespaceId statements tail) :
    StatementsDenotation.AgreesWith unit callee namespaceId (statement :: statements)
      (statementsCons head tail) := by
  intro frame state result
  constructor
  · intro step
    rcases step with ⟨finalFrame, finalState, control, headStep, abrupt, rfl⟩ |
      ⟨headFrame, headState, runtimeValue, headStep, tailStep⟩
    · exact .headControl namespaceId frame state statement statements
        finalFrame finalState control
        ((head_agrees _ _ _ _ _).mp headStep) abrupt
    · exact .cons namespaceId frame state statement statements headFrame headState
        runtimeValue _ ((head_agrees _ _ _ _ _).mp headStep)
        ((tail_agrees _ _ _).mp tailStep)
  · intro step
    cases step with
    | headControl _ _ _ _ _ finalFrame finalState control headStep abrupt =>
        exact Or.inl ⟨finalFrame, finalState, control,
          (head_agrees _ _ _ _ _).mpr headStep, abrupt, rfl⟩
    | cons _ _ _ _ _ headFrame headState runtimeValue result headStep tailStep =>
        exact Or.inr ⟨headFrame, headState, runtimeValue,
          (head_agrees _ _ _ _ _).mpr headStep,
          (tail_agrees _ _ _).mpr tailStep⟩

theorem primitive_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId}
    {operation : PrimitiveOperation} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {operands : ValuesDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind =
      .operation (.primitive operation) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (primitive ns resultType operation operands unit.targetPointerWidth) := by
  subst resultType
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with ⟨operandFrame, operandState, propagated, operandsStep,
        finalFrame_eq, finalState_eq, control_eq⟩ |
      ⟨operandFrame, operandState, values, operandsStep,
        ⟨runtimeValue, evaluate_eq, finalFrame_eq, finalState_eq, control_eq⟩ |
          ⟨kind, thrown, evaluate_eq, finalFrame_eq, finalState_eq, control_eq⟩⟩
    all_goals subst finalFrame
    all_goals subst finalState
    all_goals subst control
    · exact .primitiveArgumentsControl namespaceId frame state exprId ns expression
        operation instantiations arguments surface operandState operandFrame propagated
        namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep)
    · exact .primitiveValue namespaceId frame state exprId ns expression operation
        instantiations arguments surface operandState operandFrame values runtimeValue
        namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep) evaluate_eq
    · exact .primitiveThrow namespaceId frame state exprId ns expression operation
        instantiations arguments surface operandState operandFrame values kind thrown
        namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep) evaluate_eq
  · intro step
    cases step <;>
      try (exfalso; simp_all [evaluatePlaceOperation?_primitive]; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [primitive, ValuesDenotation.AgreesWith,
      evaluatePlaceOperation?_primitive]
    all_goals first
      | exact Or.inl ⟨_, _, _, by assumption, rfl, rfl, rfl⟩
      | exact Or.inr ⟨_, _, _, by assumption,
          _, by assumption, rfl, rfl, rfl⟩
      | exact Or.inr ⟨_, _, _, by assumption,
          _, _, by assumption, rfl, rfl, ⟨rfl, rfl⟩⟩

/-- A pure primitive on already folded operands is a literal when its
existing semantic evaluator returns that literal successfully. -/
theorem primitive_computed_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {operation : PrimitiveOperation} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {values : List RuntimeValue} {runtimeValue : RuntimeValue}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind =
      .operation (.primitive operation) instantiations arguments surface)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList (literalValues values))
    (evaluated : evaluatePrimitiveOperation? ns expression.typeId operation values.toArray
      unit.targetPointerWidth = some (.ok runtimeValue)) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId (value runtimeValue) := by
  have agreement := primitive_agrees namespace_eq expression_eq kind_eq rfl operands_agree
  intro frame state finalFrame finalState control
  rw [← agreement]
  simp [primitive, literalValues, value, evaluated, eq_comm, and_assoc]

theorem global_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId}
    {kind : GlobalKind} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {operands : ValuesDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind =
      .operation (.global kind) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (global unit.unit ns resultType exprId kind instantiations operands) := by
  subst resultType
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with ⟨operandFrame, operandState, propagated, operandsStep,
        finalFrame_eq, finalState_eq, control_eq⟩ |
      ⟨operandFrame, operandState, values, operandsStep,
        ⟨runtimeValue, evaluate_eq, control_eq⟩ |
          ⟨throwKind, thrown, evaluate_eq, control_eq⟩⟩
    · subst finalFrame
      subst finalState
      subst control
      exact .globalArgumentsControl namespaceId frame state exprId ns expression
        kind instantiations arguments surface operandState operandFrame propagated
        namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep)
    · subst control
      exact .globalValue namespaceId frame state exprId ns expression kind
        instantiations arguments surface operandState operandFrame values finalFrame
        finalState runtimeValue namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep) evaluate_eq
    · subst control
      exact .globalThrow namespaceId frame state exprId ns expression kind
        instantiations arguments surface operandState operandFrame values finalFrame
        finalState throwKind thrown namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep) evaluate_eq
  · intro step
    cases step <;>
      try (exfalso; simp_all [evaluatePlaceOperation?_global]; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [global, ValuesDenotation.AgreesWith]
    all_goals first
      | exact Or.inl ⟨_, _, _, by assumption, rfl, rfl, rfl⟩
      | exact Or.inr ⟨_, _, _, by assumption,
          by assumption⟩

theorem data_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId}
    {operation : DataOperation} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {operands : ValuesDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind =
      .operation (.data operation) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (placeOperation unit.unit ns resultType exprId (.data operation) operands) := by
  subst resultType
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with ⟨operandFrame, operandState, propagated, operandsStep,
        finalFrame_eq, finalState_eq, control_eq⟩ |
      ⟨operandFrame, operandState, values, runtimeValue, operandsStep,
        evaluate_eq, control_eq⟩
    · subst finalFrame
      subst finalState
      subst control
      exact .operationArgumentsControl namespaceId frame state exprId ns expression
        (.data operation) instantiations arguments surface operandState operandFrame
        propagated namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep)
    · subst control
      exact .operationValue namespaceId frame state exprId ns expression
        (.data operation) instantiations arguments surface operandState operandFrame
        values finalFrame finalState runtimeValue namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep) evaluate_eq
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [placeOperation, ValuesDenotation.AgreesWith]
    all_goals first
      | exact Or.inl ⟨_, _, _, by assumption, rfl, rfl, rfl⟩
      | exact Or.inr ⟨_, _, _, by assumption, by assumption⟩

theorem reference_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId}
    {operation : ReferenceOperation} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {operands : ValuesDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind =
      .operation (.reference operation) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (placeOperation unit.unit ns resultType exprId
        (.reference operation) operands) := by
  subst resultType
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with ⟨operandFrame, operandState, propagated, operandsStep,
        finalFrame_eq, finalState_eq, control_eq⟩ |
      ⟨operandFrame, operandState, values, runtimeValue, operandsStep,
        evaluate_eq, control_eq⟩
    · subst finalFrame
      subst finalState
      subst control
      exact .operationArgumentsControl namespaceId frame state exprId ns expression
        (.reference operation) instantiations arguments surface operandState operandFrame
        propagated namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep)
    · subst control
      exact .operationValue namespaceId frame state exprId ns expression
        (.reference operation) instantiations arguments surface operandState operandFrame
        values finalFrame finalState runtimeValue namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep) evaluate_eq
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [placeOperation, ValuesDenotation.AgreesWith]
    all_goals first
      | exact Or.inl ⟨_, _, _, by assumption, rfl, rfl, rfl⟩
      | exact Or.inr ⟨_, _, _, by assumption, by assumption⟩

theorem borrowPlace_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId} {kind : BorrowKind} {place : PlaceId}
    {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {operands : ValuesDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.borrow kind place)
      instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (placeOperation unit.unit ns resultType exprId
        (.borrow kind place) operands) := by
  subst resultType
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with ⟨operandFrame, operandState, propagated, operandsStep,
        finalFrame_eq, finalState_eq, control_eq⟩ |
      ⟨operandFrame, operandState, values, runtimeValue, operandsStep,
        evaluate_eq, control_eq⟩
    · subst finalFrame
      subst finalState
      subst control
      exact .operationArgumentsControl namespaceId frame state exprId ns expression
        (.borrow kind place) instantiations arguments surface operandState operandFrame
        propagated namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep)
    · subst control
      exact .operationValue namespaceId frame state exprId ns expression
        (.borrow kind place) instantiations arguments surface operandState operandFrame
        values finalFrame finalState runtimeValue namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep) evaluate_eq
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [placeOperation, ValuesDenotation.AgreesWith]
    all_goals first
      | exact Or.inl ⟨_, _, _, by assumption, rfl, rfl, rfl⟩
      | exact Or.inr ⟨_, _, _, by assumption, by assumption⟩

theorem localPlace_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId} {operation : LocalLocationOperation}
    {place : PlaceId} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {operands : ValuesDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation
      (operation.sourceOperation place) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (placeOperation unit.unit ns resultType exprId
        (operation.sourceOperation place) operands) := by
  subst resultType
  cases operation <;>
    simp only [LocalLocationOperation.sourceOperation] at kind_eq ⊢
  all_goals
    intro frame state finalFrame finalState control
    constructor
    · intro step
      rcases step with ⟨operandFrame, operandState, propagated, operandsStep,
          finalFrame_eq, finalState_eq, control_eq⟩ |
        ⟨operandFrame, operandState, values, runtimeValue, operandsStep,
          evaluate_eq, control_eq⟩
      · subst finalFrame
        subst finalState
        subst control
        exact .operationArgumentsControl namespaceId frame state exprId ns expression
          _ instantiations arguments surface operandState operandFrame propagated
          namespace_eq expression_eq kind_eq
          ((operands_agree _ _ _).mp operandsStep)
      · subst control
        exact .operationValue namespaceId frame state exprId ns expression _
          instantiations arguments surface operandState operandFrame values
          finalFrame finalState runtimeValue namespace_eq expression_eq kind_eq
          ((operands_agree _ _ _).mp operandsStep) evaluate_eq
    · intro step
      cases step <;> try (exfalso; simp_all; done)
      all_goals simp_all
      all_goals subst_vars
      all_goals simp_all [placeOperation, ValuesDenotation.AgreesWith]
      all_goals first
        | exact Or.inl ⟨_, _, _, by assumption, rfl, rfl, rfl⟩
        | exact Or.inr ⟨_, _, _, by assumption, by assumption⟩

theorem constructor_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {reference : QualifiedRef} {variant : Option String}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {surface : Option SurfaceSyntax} {operands : ValuesDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation
      (.call (.constructor reference variant)) instantiations arguments surface)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (constructor unit.unit namespaceId reference variant operands) := by
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with ⟨operandFrame, operandState, propagated, operandsStep,
        finalFrame_eq, finalState_eq, control_eq⟩ |
      ⟨operandFrame, operandState, values, runtimeValue, operandsStep,
        construct_eq, finalFrame_eq, finalState_eq, control_eq⟩
    · subst finalFrame
      subst finalState
      subst control
      exact .constructorArgumentsControl namespaceId frame state exprId ns expression
        reference variant instantiations arguments surface operandState operandFrame
        propagated namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep)
    · subst finalFrame
      subst finalState
      subst control
      exact .constructorValue namespaceId frame state exprId ns expression reference
        variant instantiations arguments surface operandState operandFrame values
        runtimeValue namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep) construct_eq
  · intro step
    cases step <;>
      try (exfalso; simp_all [evaluatePlaceOperation?_call]; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [constructor, ValuesDenotation.AgreesWith]
    all_goals first
      | exact Or.inl ⟨_, _, _, by assumption, rfl, rfl, rfl⟩
      | exact Or.inr ⟨_, _, _, by assumption, _, by assumption,
          rfl, rfl, rfl⟩

/-! A lowering certificate is an equality between the emitted native
evaluator and the authoritative leaf evaluator.  These wrappers consume that
certificate once, while assembling `denotation_agrees`; verification sees
only `nativeOperation` and the closed descriptor. -/

theorem nativeGlobal_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId} {kind : GlobalKind}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {surface : Option SurfaceSyntax} {operands : ValuesDenotation}
    {evaluate : NativeEvaluator}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind =
      .operation (.global kind) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (evaluate_eq : evaluate = fun values frame state =>
      evaluateGlobalOperation? unit.unit ns resultType exprId kind instantiations
        values frame state)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeOperation evaluate operands) := by
  subst evaluate
  intro frame state finalFrame finalState control
  simpa [nativeOperation, global] using
    (global_agrees namespace_eq expression_eq kind_eq type_eq operands_agree
      frame state finalFrame finalState control)

private theorem nativeOperation_liftPrimitive
    (evaluate : Array RuntimeValue →
      Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue))
    (operands : ValuesDenotation) :
    nativeOperation (liftPrimitiveEvaluator evaluate) operands =
      fun frame state finalFrame finalState control =>
        (∃ operandFrame operandState propagated,
          operands frame state (.control operandState operandFrame propagated) ∧
          finalFrame = operandFrame ∧ finalState = operandState ∧
          control = propagated) ∨
        (∃ operandFrame operandState values,
          operands frame state (.values operandState operandFrame values) ∧
          ((∃ runtimeValue,
              evaluate values.toArray = some (.ok runtimeValue) ∧
              finalFrame = operandFrame ∧ finalState = operandState ∧
              control = .value runtimeValue) ∨
           (∃ kind thrown,
              evaluate values.toArray = some (.error (kind, thrown)) ∧
              finalFrame = operandFrame ∧ finalState = operandState ∧
              control = .throw_ kind thrown))) := by
  funext frame state finalFrame finalState control
  apply propext
  constructor
  · intro step
    rcases step with propagated |
      ⟨operandFrame, operandState, values, operandsStep,
        ⟨runtimeValue, evaluated, control_eq⟩ |
        ⟨throwKind, thrown, evaluated, control_eq⟩⟩
    · exact .inl propagated
    · cases source_eq : evaluate values.toArray with
      | none => simp [liftPrimitiveEvaluator, source_eq] at evaluated
      | some source =>
          cases source with
          | ok value =>
              simp [liftPrimitiveEvaluator, source_eq] at evaluated
              rcases evaluated with ⟨rfl, rfl, rfl⟩
              exact .inr ⟨operandFrame, operandState, values, operandsStep,
                .inl ⟨value, source_eq, rfl, rfl, control_eq⟩⟩
          | error failure =>
              simp [liftPrimitiveEvaluator, source_eq] at evaluated
    · cases source_eq : evaluate values.toArray with
      | none => simp [liftPrimitiveEvaluator, source_eq] at evaluated
      | some source =>
          cases source with
          | ok value => simp [liftPrimitiveEvaluator, source_eq] at evaluated
          | error failure =>
              rcases failure with ⟨kind, arguments⟩
              simp [liftPrimitiveEvaluator, source_eq] at evaluated
              rcases evaluated with ⟨rfl, rfl, rfl, rfl⟩
              exact .inr ⟨operandFrame, operandState, values, operandsStep,
                .inr ⟨kind, arguments, source_eq, rfl, rfl, control_eq⟩⟩
  · intro step
    rcases step with propagated |
      ⟨operandFrame, operandState, values, operandsStep,
        ⟨runtimeValue, source_eq, rfl, rfl, control_eq⟩ |
        ⟨kind, thrown, source_eq, rfl, rfl, control_eq⟩⟩
    · exact .inl propagated
    · exact .inr ⟨finalFrame, finalState, values, operandsStep,
        .inl ⟨runtimeValue, by simp [liftPrimitiveEvaluator, source_eq],
          control_eq⟩⟩
    · exact .inr ⟨finalFrame, finalState, values, operandsStep,
        .inr ⟨kind, thrown, by simp [liftPrimitiveEvaluator, source_eq],
          control_eq⟩⟩

theorem nativePrimitive_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId} {operation : PrimitiveOperation}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {surface : Option SurfaceSyntax}
    {operands : ValuesDenotation} {evaluate : NativeEvaluator}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind =
      .operation (.primitive operation) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (evaluate_eq : evaluate = liftPrimitiveEvaluator fun values =>
      evaluatePrimitiveOperation? ns resultType operation values
        unit.targetPointerWidth)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeOperation evaluate operands) := by
  subst evaluate
  rw [nativeOperation_liftPrimitive]
  intro frame state finalFrame finalState control
  simpa [primitive] using
    (primitive_agrees namespace_eq expression_eq kind_eq type_eq operands_agree
      frame state finalFrame finalState control)

/-- Agreement for the descriptor-preserving primitive head emitted by
lowering.  Keeping this wrapper in the conclusion lets agreement assembly
select the rule by the generated denotation's native head. -/
theorem nativePrimitiveOperation_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId} {source : PrimitiveOperation}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {surface : Option SurfaceSyntax}
    {operands : ValuesDenotation} {operation : PrimitiveLocationOperation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind =
      .operation (.primitive source) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (evaluate_eq : operation.evaluate? = liftPrimitiveEvaluator fun values =>
      evaluatePrimitiveOperation? ns resultType source values
        unit.targetPointerWidth)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativePrimitiveOperation operation operands) := by
  simpa [nativePrimitiveOperation] using
    nativePrimitive_agrees namespace_eq expression_eq kind_eq type_eq
      evaluate_eq operands_agree

private theorem nativeOperation_liftPlace
    (evaluate : Array RuntimeValue → RuntimeFrame → RuntimeState →
      Option (RuntimeFrame × RuntimeState × RuntimeValue))
    (operands : ValuesDenotation) :
    nativeOperation (liftPlaceEvaluator evaluate) operands =
      fun frame state finalFrame finalState control =>
        (∃ operandFrame operandState propagated,
          operands frame state (.control operandState operandFrame propagated) ∧
          finalFrame = operandFrame ∧ finalState = operandState ∧
          control = propagated) ∨
        (∃ operandFrame operandState values runtimeValue,
          operands frame state (.values operandState operandFrame values) ∧
          evaluate values.toArray operandFrame operandState =
            some (finalFrame, finalState, runtimeValue) ∧
          control = .value runtimeValue) := by
  funext frame state finalFrame finalState control
  apply propext
  constructor
  · intro step
    rcases step with propagated |
      ⟨operandFrame, operandState, values, operandsStep,
        ⟨runtimeValue, evaluated, control_eq⟩ |
        ⟨throwKind, thrown, evaluated, control_eq⟩⟩
    · exact .inl propagated
    · cases source_eq : evaluate values.toArray operandFrame operandState with
      | none => simp [liftPlaceEvaluator, source_eq] at evaluated
      | some source =>
          rcases source with ⟨sourceFrame, sourceState, sourceValue⟩
          simp [liftPlaceEvaluator, source_eq] at evaluated
          rcases evaluated with ⟨rfl, rfl, rfl⟩
          exact .inr ⟨operandFrame, operandState, values, sourceValue,
            operandsStep, source_eq, control_eq⟩
    · cases source_eq : evaluate values.toArray operandFrame operandState with
      | none => simp [liftPlaceEvaluator, source_eq] at evaluated
      | some source =>
          rcases source with ⟨sourceFrame, sourceState, sourceValue⟩
          simp [liftPlaceEvaluator, source_eq] at evaluated
  · intro step
    rcases step with propagated |
      ⟨operandFrame, operandState, values, runtimeValue, operandsStep,
        source_eq, control_eq⟩
    · exact .inl propagated
    · exact .inr ⟨operandFrame, operandState, values, operandsStep,
        .inl ⟨runtimeValue, by simp [liftPlaceEvaluator, source_eq],
          control_eq⟩⟩

private theorem nativeOperation_liftConstructor
    (evaluate : Array RuntimeValue → Option RuntimeValue)
    (operands : ValuesDenotation) :
    nativeOperation (liftConstructorEvaluator evaluate) operands =
      fun frame state finalFrame finalState control =>
        (∃ operandFrame operandState propagated,
          operands frame state (.control operandState operandFrame propagated) ∧
          finalFrame = operandFrame ∧ finalState = operandState ∧
          control = propagated) ∨
        (∃ operandFrame operandState values runtimeValue,
          operands frame state (.values operandState operandFrame values) ∧
          evaluate values.toArray = some runtimeValue ∧
          finalFrame = operandFrame ∧ finalState = operandState ∧
          control = .value runtimeValue) := by
  funext frame state finalFrame finalState control
  apply propext
  constructor
  · intro step
    rcases step with propagated |
      ⟨operandFrame, operandState, values, operandsStep,
        ⟨runtimeValue, evaluated, control_eq⟩ |
        ⟨throwKind, thrown, evaluated, control_eq⟩⟩
    · exact .inl propagated
    · cases source_eq : evaluate values.toArray with
      | none => simp [liftConstructorEvaluator, source_eq] at evaluated
      | some sourceValue =>
          simp [liftConstructorEvaluator, source_eq] at evaluated
          rcases evaluated with ⟨rfl, rfl, rfl⟩
          exact .inr ⟨operandFrame, operandState, values, sourceValue,
            operandsStep, source_eq, rfl, rfl, control_eq⟩
    · cases source_eq : evaluate values.toArray with
      | none => simp [liftConstructorEvaluator, source_eq] at evaluated
      | some sourceValue =>
          simp [liftConstructorEvaluator, source_eq] at evaluated
  · intro step
    rcases step with propagated |
      ⟨operandFrame, operandState, values, runtimeValue, operandsStep,
        source_eq, rfl, rfl, control_eq⟩
    · exact .inl propagated
    · exact .inr ⟨finalFrame, finalState, values, operandsStep,
        .inl ⟨runtimeValue, by simp [liftConstructorEvaluator, source_eq],
          control_eq⟩⟩

theorem nativeData_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId} {operation : DataOperation}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {surface : Option SurfaceSyntax} {operands : ValuesDenotation}
    {evaluate : NativeEvaluator}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind =
      .operation (.data operation) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (evaluate_eq : evaluate = liftPlaceEvaluator fun values frame state =>
      evaluatePlaceOperation? unit.unit ns resultType exprId (.data operation)
        values frame state)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeOperation evaluate operands) := by
  subst evaluate
  rw [nativeOperation_liftPlace]
  intro frame state finalFrame finalState control
  simpa [placeOperation] using
    (data_agrees namespace_eq expression_eq kind_eq type_eq operands_agree
      frame state finalFrame finalState control)

theorem nativeReference_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId} {operation : ReferenceOperation}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {surface : Option SurfaceSyntax} {operands : ValuesDenotation}
    {evaluate : NativeEvaluator}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind =
      .operation (.reference operation) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (evaluate_eq : evaluate = liftPlaceEvaluator fun values frame state =>
      evaluatePlaceOperation? unit.unit ns resultType exprId
        (.reference operation) values frame state)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeOperation evaluate operands) := by
  subst evaluate
  rw [nativeOperation_liftPlace]
  intro frame state finalFrame finalState control
  simpa [placeOperation] using
    (reference_agrees namespace_eq expression_eq kind_eq type_eq operands_agree
      frame state finalFrame finalState control)

theorem nativeLocal_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId} {operation : LocalLocationOperation}
    {place : PlaceId} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {operands : ValuesDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation
      (operation.sourceOperation place) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (evaluate_eq : operation.evaluate? = liftPlaceEvaluator fun values frame state =>
      evaluatePlaceOperation? unit.unit ns resultType exprId
        (operation.sourceOperation place) values frame state)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeLocalOperation operation operands) := by
  rw [nativeLocalOperation, evaluate_eq, nativeOperation_liftPlace]
  intro frame state finalFrame finalState control
  simpa [placeOperation] using
    (localPlace_agrees namespace_eq expression_eq kind_eq type_eq operands_agree
      frame state finalFrame finalState control)

theorem nativeDerefLocalBorrow_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId} {operation : DerefLocalBorrowOperation}
    {place : PlaceId} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {operands : ValuesDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation
      (.borrow operation.kind place) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (evaluate_eq : operation.evaluate? = liftPlaceEvaluator fun values frame state =>
      evaluatePlaceOperation? unit.unit ns resultType exprId
        (.borrow operation.kind place) values frame state)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeDerefLocalBorrowOperation operation operands) := by
  rw [nativeDerefLocalBorrowOperation, evaluate_eq,
    nativeOperation_liftPlace]
  intro frame state finalFrame finalState control
  simpa [placeOperation] using
    (borrowPlace_agrees namespace_eq expression_eq kind_eq type_eq
      operands_agree frame state finalFrame finalState control)

theorem nativeIndexedLocalBorrow_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId} {operation : IndexedLocalBorrowOperation}
    {place : PlaceId} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {operands : ValuesDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation
      (.borrow operation.kind place) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (evaluate_eq : operation.evaluate? = liftPlaceEvaluator fun values frame state =>
      evaluatePlaceOperation? unit.unit ns resultType exprId
        (.borrow operation.kind place) values frame state)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeIndexedLocalBorrowOperation operation operands) := by
  rw [nativeIndexedLocalBorrowOperation, evaluate_eq,
    nativeOperation_liftPlace]
  intro frame state finalFrame finalState control
  simpa [placeOperation] using
    (borrowPlace_agrees namespace_eq expression_eq kind_eq type_eq
      operands_agree frame state finalFrame finalState control)

theorem nativeIndexedLocalFieldBorrow_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {resultType : TypeId} {operation : IndexedLocalFieldBorrowOperation}
    {place : PlaceId} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {operands : ValuesDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation
      (.borrow operation.kind place) instantiations arguments surface)
    (type_eq : expression.typeId = resultType)
    (evaluate_eq : operation.evaluate? = liftPlaceEvaluator fun values frame state =>
      evaluatePlaceOperation? unit.unit ns resultType exprId
        (.borrow operation.kind place) values frame state)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeIndexedLocalFieldBorrowOperation operation operands) := by
  rw [nativeIndexedLocalFieldBorrowOperation, evaluate_eq,
    nativeOperation_liftPlace]
  intro frame state finalFrame finalState control
  simpa [placeOperation] using
    (borrowPlace_agrees namespace_eq expression_eq kind_eq type_eq
      operands_agree frame state finalFrame finalState control)

theorem nativeConstructor_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {reference : QualifiedRef} {variant : Option String}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {surface : Option SurfaceSyntax} {operands : ValuesDenotation}
    {evaluate : NativeEvaluator}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation
      (.call (.constructor reference variant)) instantiations arguments surface)
    (evaluate_eq : evaluate = liftConstructorEvaluator fun values =>
      constructNominal? unit.unit namespaceId reference variant values)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeOperation evaluate operands) := by
  subst evaluate
  rw [nativeOperation_liftConstructor]
  intro frame state finalFrame finalState control
  simpa [constructor] using
    (constructor_agrees namespace_eq expression_eq kind_eq operands_agree
      frame state finalFrame finalState control)

theorem call_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {reference : QualifiedRef} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {handle : FunctionHandle} {operands : ValuesDenotation}
    {calleeDenotation : FunctionDenotation} {lexical : Option Nat}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.call (.function reference))
      instantiations arguments surface)
    (resolve_eq : resolveFunction? unit.unit namespaceId reference = some handle)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands)
    (callee_agrees : ∀ typeInstantiation,
      FunctionDenotation.AgreesWith callee handle typeInstantiation calleeDenotation)
    (loan_eq : certificateLoanId? unit.unit namespaceId exprId = lexical) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId (call lexical calleeDenotation operands) := by
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with ⟨operandFrame, operandState, propagated, operandsStep,
        finalFrame_eq, finalState_eq, control_eq⟩ |
      ⟨operandFrame, operandState, values, calleeState, outcome,
        operandsStep, calleeStep, finalFrame_eq, finalState_eq, control_eq⟩
    · subst finalFrame
      subst finalState
      subst control
      exact .callArgumentsControl namespaceId frame state exprId ns expression
        reference instantiations arguments surface operandState operandFrame
        propagated namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep)
    · subst finalFrame
      subst finalState
      subst control
      cases outcome with
      | returned results =>
          rw [← loan_eq]
          exact .callReturned namespaceId frame state exprId ns expression
            reference instantiations arguments surface operandState operandFrame
            values handle calleeState results namespace_eq expression_eq kind_eq
            ((operands_agree _ _ _).mp operandsStep) resolve_eq
            ((callee_agrees _ _ _ _ _).mp calleeStep)
      | threw kind thrown =>
          exact .callThrew namespaceId frame state exprId ns expression
            reference instantiations arguments surface operandState operandFrame
            values handle calleeState kind thrown namespace_eq expression_eq kind_eq
            ((operands_agree _ _ _).mp operandsStep) resolve_eq
            ((callee_agrees _ _ _ _ _).mp calleeStep)
  · intro step
    cases step <;>
      try (exfalso; simp_all [evaluatePlaceOperation?_call]; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [call, callFrame, callControl, ValuesDenotation.AgreesWith,
      FunctionDenotation.AgreesWith]
    all_goals first
      | exact Or.inl ⟨_, _, _, by assumption, rfl, rfl, rfl⟩
      | exact Or.inr ⟨_, _, _, by assumption, _, .returned _,
          by exact (callee_agrees _ _ _ _ _).mpr (by assumption), rfl, rfl, rfl⟩
      | exact Or.inr ⟨_, _, _, by assumption, _, .threw _ _,
          by exact (callee_agrees _ _ _ _ _).mpr (by assumption), rfl, rfl, rfl⟩

theorem nativeCall_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {reference : QualifiedRef} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {handle : FunctionHandle} {operands : ValuesDenotation}
    {calleeDenotation : FunctionDenotation} {lexical : Option Nat}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.call (.function reference))
      instantiations arguments surface)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands)
    (callee_agrees : ∀ typeInstantiation,
      FunctionDenotation.AgreesWith callee handle typeInstantiation calleeDenotation)
    (resolve_eq : resolveFunction? unit.unit namespaceId reference = some handle)
    (loan_eq : certificateLoanId? unit.unit namespaceId exprId = lexical) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeCall handle lexical calleeDenotation operands) := by
  simpa [nativeCall] using
    call_agrees namespace_eq expression_eq kind_eq resolve_eq
      operands_agree callee_agrees loan_eq

theorem callAt_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {reference : QualifiedRef} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {handle : FunctionHandle} {operands : ValuesDenotation}
    {calleeDenotation : Array (TypeId × TypeId) → FunctionDenotation}
    {lexical : Option Nat}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.call (.function reference))
      instantiations arguments surface)
    (resolve_eq : resolveFunction? unit.unit namespaceId reference = some handle)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands)
    (callee_agrees : ∀ outer,
      FunctionDenotation.AgreesWith callee handle
        (callTypeInstantiation unit.unit handle outer instantiations)
        (calleeDenotation outer))
    (loan_eq : certificateLoanId? unit.unit namespaceId exprId = lexical) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (callAt lexical calleeDenotation operands) := by
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with ⟨operandFrame, operandState, propagated, operandsStep,
        finalFrame_eq, finalState_eq, control_eq⟩ |
      ⟨operandFrame, operandState, values, calleeState, outcome,
        operandsStep, calleeStep, finalFrame_eq, finalState_eq, control_eq⟩
    · subst finalFrame
      subst finalState
      subst control
      exact .callArgumentsControl namespaceId frame state exprId ns expression
        reference instantiations arguments surface operandState operandFrame
        propagated namespace_eq expression_eq kind_eq
        ((operands_agree _ _ _).mp operandsStep)
    · subst finalFrame
      subst finalState
      subst control
      cases outcome with
      | returned results =>
          rw [← loan_eq]
          exact .callReturned namespaceId frame state exprId ns expression
            reference instantiations arguments surface operandState operandFrame
            values handle calleeState results namespace_eq expression_eq kind_eq
            ((operands_agree _ _ _).mp operandsStep) resolve_eq
            ((callee_agrees operandFrame.typeInstantiation _ _ _ _).mp calleeStep)
      | threw kind thrown =>
          exact .callThrew namespaceId frame state exprId ns expression
            reference instantiations arguments surface operandState operandFrame
            values handle calleeState kind thrown namespace_eq expression_eq kind_eq
            ((operands_agree _ _ _).mp operandsStep) resolve_eq
            ((callee_agrees operandFrame.typeInstantiation _ _ _ _).mp calleeStep)
  · intro step
    cases step <;>
      try (exfalso; simp_all [evaluatePlaceOperation?_call]; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [callAt, callFrame, callControl,
      ValuesDenotation.AgreesWith, FunctionDenotation.AgreesWith]
    all_goals first
      | exact Or.inl ⟨_, _, _, by assumption, rfl, rfl, rfl⟩
      | exact Or.inr ⟨_, _, _, by assumption, _, .returned _,
          by assumption, rfl, rfl, rfl⟩
      | exact Or.inr ⟨_, _, _, by assumption, _, .threw _ _,
          by assumption, rfl, rfl, rfl⟩

theorem nativeCallAt_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {reference : QualifiedRef} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {handle : FunctionHandle} {operands : ValuesDenotation}
    {calleeDenotation : Array (TypeId × TypeId) → FunctionDenotation}
    {lexical : Option Nat}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.call (.function reference))
      instantiations arguments surface)
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands)
    (callee_agrees : ∀ outer,
      FunctionDenotation.AgreesWith callee handle
        (callTypeInstantiation unit.unit handle outer instantiations)
        (calleeDenotation outer))
    (resolve_eq : resolveFunction? unit.unit namespaceId reference = some handle)
    (loan_eq : certificateLoanId? unit.unit namespaceId exprId = lexical) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeCallAt handle lexical calleeDenotation operands) := by
  simpa [nativeCallAt] using
    callAt_agrees namespace_eq expression_eq kind_eq resolve_eq
      operands_agree callee_agrees loan_eq

/-- A call with no generic arguments uses the ordinary function relation.
Its invocation substitution is empty, including inside a generic caller. -/
theorem nativeCall_monomorphic_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {reference : QualifiedRef} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {handle : FunctionHandle} {operands : ValuesDenotation}
    {calleeDenotation : FunctionDenotation} {lexical : Option Nat}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.call (.function reference))
      instantiations arguments surface)
    (instantiations_eq : instantiations = #[])
    (operands_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList operands)
    (callee_agrees : FunctionDenotation.AgreesWith callee handle #[] calleeDenotation)
    (resolve_eq : resolveFunction? unit.unit namespaceId reference = some handle)
    (loan_eq : certificateLoanId? unit.unit namespaceId exprId = lexical) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeCall handle lexical calleeDenotation operands) := by
  subst instantiations
  exact nativeCallAt_agrees namespace_eq expression_eq kind_eq operands_agree
    (fun _ => callee_agrees) resolve_eq loan_eq

theorem nativeReturn_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {values : Array ExprId} {denotation : ValuesDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .return_ values)
    (values_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      values.toList denotation) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeReturn denotation) := by
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with ⟨runtimeValues, values_step, rfl⟩ | values_step
    · exact .returnValues namespaceId frame state exprId ns expression values
        finalFrame finalState runtimeValues namespace_eq expression_eq kind_eq
        ((values_agree _ _ _).mp values_step)
    · exact .returnControl namespaceId frame state exprId ns expression values
        finalFrame finalState control namespace_eq expression_eq kind_eq
        ((values_agree _ _ _).mp values_step)
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [nativeReturn, ValuesDenotation.AgreesWith]
    all_goals first
      | exact Or.inl ⟨_, by assumption, rfl⟩
      | exact Or.inr (by assumption)

theorem nativeThrow_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {kind : ThrowKind} {arguments : Array ExprId}
    {denotation : ValuesDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .throw_ kind arguments)
    (arguments_agree : ValuesDenotation.AgreesWith unit callee namespaceId
      arguments.toList denotation) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeThrow kind denotation) := by
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with ⟨runtimeValues, arguments_step, rfl⟩ | arguments_step
    · exact .throwValues namespaceId frame state exprId ns expression kind
        arguments finalFrame finalState runtimeValues namespace_eq expression_eq
        kind_eq ((arguments_agree _ _ _).mp arguments_step)
    · exact .throwControl namespaceId frame state exprId ns expression kind
        arguments finalFrame finalState control namespace_eq expression_eq kind_eq
        ((arguments_agree _ _ _).mp arguments_step)
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [nativeThrow, ValuesDenotation.AgreesWith]
    all_goals first
      | exact Or.inl ⟨_, by assumption, rfl⟩
      | exact Or.inr (by assumption)

theorem nativeContinue_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {nest : Nat}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .continue_ nest) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId (nativeContinue nest) := by
  intro frame state finalFrame finalState control
  constructor
  · rintro ⟨rfl, rfl, rfl⟩
    exact .continue_ namespaceId _ _ exprId ns expression nest
      namespace_eq expression_eq kind_eq
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    simp_all [nativeContinue]

theorem nativeBreakNone_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {nest : Nat}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .break_ nest none) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId (nativeBreak nest none) := by
  intro frame state finalFrame finalState control
  constructor
  · rintro ⟨rfl, rfl, rfl⟩
    exact .breakNone namespaceId _ _ exprId ns expression nest
      namespace_eq expression_eq kind_eq
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    simp_all [nativeBreak]

theorem nativeSpec_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {block : SpecBlock}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .spec block) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId nativeSpec := by
  intro frame state finalFrame finalState control
  constructor
  · rintro ⟨rfl, rfl, rfl⟩
    exact .spec namespaceId _ _ exprId ns expression block
      namespace_eq expression_eq kind_eq
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    simp_all [nativeSpec, value]

theorem nativeAssignLocal_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId child : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {place : PlaceId}
    {localId : LocalId} {denotation : ExprDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .assign place child)
    (place_eq : ns.places[place.index]? = some (.localVar localId))
    (child_agree : ExprDenotation.AgreesWith unit callee namespaceId child denotation) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeAssignLocal localId denotation) := by
  intro frame state finalFrame finalState control
  constructor
  · rintro (⟨child_step, abrupt⟩ |
      ⟨valueFrame, valueState, runtimeValue, child_step, in_bounds,
        finalFrame_eq, finalState_eq, control_eq⟩)
    · exact .assignControl namespaceId frame state exprId ns expression place child
        finalFrame finalState control namespace_eq expression_eq kind_eq
        ((child_agree _ _ _ _ _).mp child_step) abrupt
    · subst finalFrame
      subst finalState
      subst control
      apply EvalExpr.assignValue (namespace_eq := namespace_eq)
        (expression_eq := expression_eq) (kind_eq := kind_eq)
        (child_step := (child_agree _ _ _ _ _).mp child_step)
        (resolved := { root := .local localId })
      · rw [resolvePlace?_localVar place_eq]
        simp [in_bounds]
      · simp [writeRuntimePlace?, writeRoot?, in_bounds]
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    case assignControl =>
      rename_i ns2 expression2 place2 child2 kind_eq2 namespace_eq2
        expression_eq2 abrupt2 child_step2
      have ns_same : ns2 = ns := Option.some.inj
        (namespace_eq2.symm.trans namespace_eq)
      subst ns2
      have expression_same : expression2 = expression := Option.some.inj
        (expression_eq2.symm.trans expression_eq)
      subst expression2
      have kind_same : ExprKind.assign place2 child2 =
          ExprKind.assign place child := kind_eq2.symm.trans kind_eq
      injection kind_same with place_same child_same
      subst place2
      subst child2
      left
      constructor
      · exact (child_agree _ _ _ _ _).mpr child_step2
      · exact abrupt2
    case assignValue =>
      rename_i ns2 expression2 place2 child2 childFrame childState resolved
        runtimeValue kind_eq2 resolve_eq2 namespace_eq2 child_step2
        expression_eq2 write_eq2
      have ns_same : ns2 = ns := Option.some.inj
        (namespace_eq2.symm.trans namespace_eq)
      subst ns2
      have expression_same : expression2 = expression := Option.some.inj
        (expression_eq2.symm.trans expression_eq)
      subst expression2
      have kind_same : ExprKind.assign place2 child2 =
          ExprKind.assign place child := kind_eq2.symm.trans kind_eq
      injection kind_same with place_same child_same
      subst place2
      subst child2
      right
      rw [resolvePlace?_localVar place_eq] at resolve_eq2
      split at resolve_eq2
      · rename_i in_bounds
        simp at resolve_eq2
        subst resolved
        simp [writeRuntimePlace?, writeRoot?, in_bounds] at write_eq2
        obtain ⟨rfl, rfl⟩ := write_eq2
        refine ⟨_, _, _, (child_agree _ _ _ _ _).mpr child_step2,
          in_bounds, ?_, rfl, rfl⟩
        simp only [SemanticOperations.array_set!_eq_setIfInBounds]
      · simp_all

theorem nativeAssignLocalIndex_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId child indexExpr : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {place basePlace : PlaceId} {base index : LocalId}
    {denotation : ExprDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .assign place child)
    (place_eq : ns.places[place.index]? = some (.index basePlace indexExpr))
    (base_eq : ns.places[basePlace.index]? = some (.localVar base))
    (index_eq : placeIndexForm? ns indexExpr = some (.local index) ∨
      placeIndexForm? ns indexExpr = some (.copyLocal index))
    (child_agree : ExprDenotation.AgreesWith unit callee namespaceId child denotation) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeAssignLocalIndex base index denotation) := by
  intro frame state finalFrame finalState control
  constructor
  · rintro (⟨child_step, abrupt⟩ |
      ⟨valueFrame, valueState, runtimeValue, resolved, child_step,
        resolve_eq, write_eq, control_eq⟩)
    · exact .assignControl namespaceId frame state exprId ns expression place child
        finalFrame finalState control namespace_eq expression_eq kind_eq
        ((child_agree _ _ _ _ _).mp child_step) abrupt
    · subst control
      exact .assignValue namespaceId frame state exprId ns expression place child
        valueFrame valueState resolved finalFrame finalState runtimeValue
        namespace_eq expression_eq kind_eq
        ((child_agree _ _ _ _ _).mp child_step)
        ((resolvePlace?_localIndex place_eq base_eq index_eq).trans resolve_eq)
        write_eq
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    case assignControl =>
      rename_i ns2 expression2 place2 child2 kind_eq2 namespace_eq2
        expression_eq2 abrupt2 child_step2
      have ns_same : ns2 = ns := Option.some.inj
        (namespace_eq2.symm.trans namespace_eq)
      subst ns2
      have expression_same : expression2 = expression := Option.some.inj
        (expression_eq2.symm.trans expression_eq)
      subst expression2
      have kind_same : ExprKind.assign place2 child2 =
          ExprKind.assign place child := kind_eq2.symm.trans kind_eq
      injection kind_same with place_same child_same
      subst place2
      subst child2
      exact .inl ⟨(child_agree _ _ _ _ _).mpr child_step2, abrupt2⟩
    case assignValue =>
      rename_i ns2 expression2 place2 child2 childFrame childState resolved
        runtimeValue kind_eq2 resolve_eq2 namespace_eq2 child_step2
        expression_eq2 write_eq2
      have ns_same : ns2 = ns := Option.some.inj
        (namespace_eq2.symm.trans namespace_eq)
      subst ns2
      have expression_same : expression2 = expression := Option.some.inj
        (expression_eq2.symm.trans expression_eq)
      subst expression2
      have kind_same : ExprKind.assign place2 child2 =
          ExprKind.assign place child := kind_eq2.symm.trans kind_eq
      injection kind_same with place_same child_same
      subst place2
      subst child2
      refine .inr ⟨childFrame, childState, runtimeValue, resolved,
        (child_agree _ _ _ _ _).mpr child_step2, ?_, write_eq2, rfl⟩
      rw [resolvePlace?_localIndex place_eq base_eq index_eq] at resolve_eq2
      exact resolve_eq2

theorem nativeLoop_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId bodyId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {label : Option String}
    {body : ExprDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .loop label bodyId)
    (body_agree : ExprDenotation.AgreesWith unit callee namespaceId bodyId body) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId (nativeLoop exprId body) := by
  intro frame state finalFrame finalState control
  constructor
  · intro step
    change NativeLoop body frame state finalFrame finalState control at step
    induction step with
    | repeatValue value control body_step repeat_step ih =>
        exact .loopRepeatValue (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (body_step := (body_agree _ _ _ _ _).mp body_step)
          (repeat_step := ih)
    | repeatContinue control body_step repeat_step ih =>
        exact .loopRepeatContinue (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (body_step := (body_agree _ _ _ _ _).mp body_step)
          (repeat_step := ih)
    | outerContinue nest body_step =>
        exact .loopOuterContinue (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (body_step := (body_agree _ _ _ _ _).mp body_step)
    | break_ breakValue body_step =>
        exact .loopBreak (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (body_step := (body_agree _ _ _ _ _).mp body_step)
    | outerBreak nest breakValue body_step =>
        exact .loopOuterBreak (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (body_step := (body_agree _ _ _ _ _).mp body_step)
    | return_ values body_step =>
        exact .loopReturn (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (body_step := (body_agree _ _ _ _ _).mp body_step)
    | throw_ kind arguments body_step =>
        exact .loopThrow (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (body_step := (body_agree _ _ _ _ _).mp body_step)
  · intro step
    have converted : LoopConvertible unit callee namespaceId exprId bodyId body step := by
      apply EvalExprWith.rec
        (motive_1 := fun namespaceArg frame state id finalFrame finalState control step =>
          LoopConvertible unit callee namespaceId exprId bodyId body step)
        (motive_2 := fun _ _ _ _ _ _ => True)
        (motive_3 := fun _ _ _ _ _ _ => True)
        (motive_4 := fun _ _ _ _ _ _ _ _ _ _ => True)
      all_goals intros
      all_goals try exact True.intro
      all_goals unfold LoopConvertible
      all_goals intros namespace_same target_eq body_agreement
      all_goals subst_vars
      all_goals simp_all
      next =>
        apply NativeLoop.repeatValue
        · exact (body_agreement _ _ _ _ _).mpr (by assumption)
        · unfold LoopConvertible at *
          solve_by_elim
      next =>
        apply NativeLoop.repeatContinue
        · exact (body_agreement _ _ _ _ _).mpr (by assumption)
        · unfold LoopConvertible at *
          solve_by_elim
      next =>
        apply NativeLoop.break_
        exact (body_agreement _ _ _ _ _).mpr (by assumption)
      next =>
        apply NativeLoop.outerBreak
        exact (body_agreement _ _ _ _ _).mpr (by assumption)
      next =>
        apply NativeLoop.outerContinue
        exact (body_agreement _ _ _ _ _).mpr (by assumption)
      next =>
        apply NativeLoop.return_
        exact (body_agreement _ _ _ _ _).mpr (by assumption)
      next =>
        apply NativeLoop.throw_
        exact (body_agreement _ _ _ _ _).mpr (by assumption)
    unfold LoopConvertible at converted
    exact converted rfl rfl body_agree

theorem blockUnit_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {statements : Array ExprId} {denotation : StatementsDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .block statements none)
    (statements_agree : StatementsDenotation.AgreesWith unit callee namespaceId
      statements.toList denotation) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId (blockUnit denotation) := by
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with controlStep | ⟨doneStep, control_eq⟩
    · exact .blockControl namespaceId frame state exprId ns expression statements none
        finalState finalFrame control namespace_eq expression_eq kind_eq
        ((statements_agree _ _ _).mp controlStep)
    · subst control
      exact .blockUnit namespaceId frame state exprId ns expression statements
        finalState finalFrame namespace_eq expression_eq kind_eq
        ((statements_agree _ _ _).mp doneStep)
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [blockUnit, StatementsDenotation.AgreesWith]
    all_goals first
      | exact Or.inl (by assumption)
      | exact Or.inr ⟨by assumption, rfl⟩

theorem blockResult_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId resultId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {statements : Array ExprId} {statementDenotation : StatementsDenotation}
    {resultDenotation : ExprDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .block statements (some resultId))
    (statements_agree : StatementsDenotation.AgreesWith unit callee namespaceId
      statements.toList statementDenotation)
    (result_agrees : ExprDenotation.AgreesWith unit callee namespaceId resultId
      resultDenotation) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (blockResult statementDenotation resultDenotation) := by
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with controlStep |
      ⟨statementFrame, statementState, statementsStep, resultStep⟩
    · exact .blockControl namespaceId frame state exprId ns expression statements
        (some resultId) finalState finalFrame control namespace_eq expression_eq
        kind_eq ((statements_agree _ _ _).mp controlStep)
    · exact .blockResult namespaceId frame state exprId ns expression statements
        resultId statementState statementFrame finalState finalFrame control
        namespace_eq expression_eq kind_eq
        ((statements_agree _ _ _).mp statementsStep)
        ((result_agrees _ _ _ _ _).mp resultStep)
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [blockResult, StatementsDenotation.AgreesWith,
      ExprDenotation.AgreesWith]
    all_goals first
      | exact Or.inl (by assumption)
      | exact Or.inr ⟨_, _, by assumption, by assumption⟩

/-- Agreement for a branch with both arms. -/
theorem nativeBranchElse_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId conditionId thenId elseId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {conditionDenotation thenDenotation elseDenotation : ExprDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .ifElse conditionId thenId (some elseId))
    (condition_agrees : ExprDenotation.AgreesWith unit callee namespaceId conditionId
      conditionDenotation)
    (then_agrees : ExprDenotation.AgreesWith unit callee namespaceId thenId thenDenotation)
    (else_agrees : ExprDenotation.AgreesWith unit callee namespaceId elseId elseDenotation) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeBranch conditionDenotation thenDenotation (some elseDenotation)) := by
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with ⟨conditionStep, abrupt⟩ |
      ⟨conditionFrame, conditionState, conditionStep, thenStep⟩ |
      ⟨conditionFrame, conditionState, conditionStep, elseStep⟩
    · exact .ifControl namespaceId frame state exprId ns expression conditionId
        thenId (some elseId) finalFrame finalState control namespace_eq
        expression_eq kind_eq ((condition_agrees _ _ _ _ _).mp conditionStep) abrupt
    · exact .ifTrue namespaceId frame state exprId ns expression conditionId
        thenId (some elseId) conditionFrame conditionState finalFrame finalState
        control namespace_eq expression_eq kind_eq
        ((condition_agrees _ _ _ _ _).mp conditionStep)
        ((then_agrees _ _ _ _ _).mp thenStep)
    · exact .ifFalse namespaceId frame state exprId ns expression conditionId
        thenId elseId conditionFrame conditionState finalFrame finalState control
        namespace_eq expression_eq kind_eq
        ((condition_agrees _ _ _ _ _).mp conditionStep)
        ((else_agrees _ _ _ _ _).mp elseStep)
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [nativeBranch, ExprDenotation.AgreesWith]
    all_goals first
      | exact Or.inl ⟨by assumption, by assumption⟩
      | exact Or.inr (Or.inl ⟨_, _, by assumption, by assumption⟩)
      | exact Or.inr (Or.inr ⟨_, _, by assumption, by assumption⟩)

/-- Agreement for a branch whose false side produces unit. -/
theorem nativeBranchUnit_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId conditionId thenId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {conditionDenotation thenDenotation : ExprDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .ifElse conditionId thenId none)
    (condition_agrees : ExprDenotation.AgreesWith unit callee namespaceId conditionId
      conditionDenotation)
    (then_agrees : ExprDenotation.AgreesWith unit callee namespaceId thenId thenDenotation) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (nativeBranch conditionDenotation thenDenotation none) := by
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with ⟨conditionStep, abrupt⟩ |
      ⟨conditionFrame, conditionState, conditionStep, thenStep⟩ |
      ⟨conditionFrame, conditionState, conditionStep, frame_eq, state_eq, control_eq⟩
    · exact .ifControl namespaceId frame state exprId ns expression conditionId
        thenId none finalFrame finalState control namespace_eq expression_eq
        kind_eq ((condition_agrees _ _ _ _ _).mp conditionStep) abrupt
    · exact .ifTrue namespaceId frame state exprId ns expression conditionId
        thenId none conditionFrame conditionState finalFrame finalState control
        namespace_eq expression_eq kind_eq
        ((condition_agrees _ _ _ _ _).mp conditionStep)
        ((then_agrees _ _ _ _ _).mp thenStep)
    · subst frame_eq; subst state_eq; subst control_eq
      exact .ifFalseUnit namespaceId frame state exprId ns expression conditionId
        thenId finalFrame finalState namespace_eq expression_eq kind_eq
        ((condition_agrees _ _ _ _ _).mp conditionStep)
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [nativeBranch, ExprDenotation.AgreesWith]
    all_goals first
      | exact Or.inl ⟨by assumption, by assumption⟩
      | exact Or.inr (Or.inl ⟨_, _, by assumption, by assumption⟩)
      | exact Or.inr (Or.inr ⟨_, _, by assumption, ⟨rfl, rfl, rfl⟩⟩)

theorem letNoValue_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId bodyId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {pattern : PatternId}
    {body : ExprDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .letDecl pattern none bodyId)
    (body_agrees : ExprDenotation.AgreesWith unit callee namespaceId bodyId body) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId (letNoValue body) := by
  intro frame state finalFrame finalState control
  constructor
  · intro step
    exact .letNoValue namespaceId frame state exprId ns expression pattern bodyId
      finalFrame finalState control namespace_eq expression_eq kind_eq
      ((body_agrees _ _ _ _ _).mp step)
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals subst_vars
    exact (body_agrees _ _ _ _ _).mpr (by assumption)

theorem letValue_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId initializerId bodyId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {pattern : PatternId}
    {initializer body : ExprDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .letDecl pattern (some initializerId) bodyId)
    (initializer_agrees : ExprDenotation.AgreesWith unit callee namespaceId initializerId
      initializer)
    (body_agrees : ExprDenotation.AgreesWith unit callee namespaceId bodyId body) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (letValue unit.unit ns pattern initializer body) := by
  intro frame state finalFrame finalState control
  constructor
  · intro step
    rcases step with ⟨initializerStep, abrupt⟩ |
      ⟨initializedFrame, initializedState, runtimeValue, boundFrame,
        initializerStep, bind_eq, bodyStep⟩
    · exact .letValueControl namespaceId frame state exprId ns expression pattern
        initializerId bodyId finalFrame finalState control namespace_eq expression_eq
        kind_eq ((initializer_agrees _ _ _ _ _).mp initializerStep) abrupt
    · exact .letValue namespaceId frame state exprId ns expression pattern
        initializerId bodyId initializedFrame initializedState runtimeValue boundFrame
        finalFrame finalState control namespace_eq expression_eq kind_eq
        ((initializer_agrees _ _ _ _ _).mp initializerStep) bind_eq
        ((body_agrees _ _ _ _ _).mp bodyStep)
  · intro step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals subst_vars
    all_goals simp_all [letValue, ExprDenotation.AgreesWith]
    all_goals first
      | exact Or.inl ⟨by assumption, by assumption⟩
      | exact Or.inr ⟨_, _, _, by assumption,
          ⟨_, by assumption, by assumption⟩⟩

theorem letNativeValue_agrees {unit : ExecutableUnit}
    {namespaceId : NamespaceId} {exprId initializerId bodyId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {pattern : PatternId}
    {binder : NativePatternBinder} {initializer body : ExprDenotation}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .letDecl pattern (some initializerId) bodyId)
    (lower_eq : lowerPattern? unit.unit ns pattern = some binder)
    (initializer_agrees : ExprDenotation.AgreesWith unit callee namespaceId initializerId
      initializer)
    (body_agrees : ExprDenotation.AgreesWith unit callee namespaceId bodyId body) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (letNativeValue binder initializer body) := by
  rw [letNativeValue_eq_letValue lower_eq]
  exact letValue_agrees namespace_eq expression_eq kind_eq initializer_agrees
    body_agrees

/-- Resolve the native binder against the closed prepared unit, then
transport the agreement to the executable view.  Lowering supplies a fact
about `prepared`, so the premise closes by reduction instead of leaving a
lookup goal containing the symbolic executable unit. -/
theorem letNativeValue_agrees_of_unit {unit : ExecutableUnit}
    {prepared : ValidatedUnit}
    {namespaceId : NamespaceId} {exprId initializerId bodyId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {pattern : PatternId}
    {binder : NativePatternBinder} {initializer body : ExprDenotation}
    (unit_eq : unit.unit = prepared)
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .letDecl pattern (some initializerId) bodyId)
    (lower_eq : lowerPattern? prepared ns pattern = some binder)
    (initializer_agrees : ExprDenotation.AgreesWith unit callee namespaceId initializerId
      initializer)
    (body_agrees : ExprDenotation.AgreesWith unit callee namespaceId bodyId body) :
    ExprDenotation.AgreesWith unit callee namespaceId exprId
      (letNativeValue binder initializer body) := by
  subst prepared
  exact letNativeValue_agrees namespace_eq expression_eq kind_eq lower_eq
    initializer_agrees body_agrees

/-- Assemble body agreement at the state-retaining function boundary used by
native direct calls. -/
theorem functionRelation_agrees {unit : ExecutableUnit}
    {handle : FunctionHandle} {ns : ValidatedNamespace}
    {declaration : FunctionDecl FunctionBody} {root : ExprId}
    {body : ExprDenotation}
    (namespace_eq : unit.unit.namespaces[handle.namespaceId.index]? = some ns)
    (declaration_eq : ns.functions[handle.functionId.index]? = some declaration)
    (body_eq : declaration.body = .structured root)
    (body_agrees : ExprDenotation.Agrees unit handle.namespaceId root body) :
    FunctionDenotation.Agrees unit handle
      (functionRelation unit declaration body) := by
  intro initial arguments final outcome
  constructor
  · rintro ⟨frame, finalFrame, evaluatedState, control, frame_eq,
        body_step, outcome_eq, finalize_eq⟩
    exact .body handle #[] initial arguments ns declaration frame root finalFrame
      evaluatedState final control outcome namespace_eq declaration_eq frame_eq
      body_eq ((body_agrees _ _ _ _ _).mp body_step) outcome_eq finalize_eq
  · intro step
    cases step
    simp_all
    subst_vars
    exact ⟨_, _, _, _, by assumption,
      (body_agrees _ _ _ _ _).mpr (by assumption), by assumption, rfl⟩

theorem nativeFunctionRelation_agrees {unit : ExecutableUnit}
    {handle : FunctionHandle} {ns : ValidatedNamespace}
    {declaration : FunctionDecl FunctionBody} {root : ExprId}
    {shape : FunctionShape} {body : ExprDenotation}
    (namespace_eq : unit.unit.namespaces[handle.namespaceId.index]? = some ns)
    (declaration_eq : ns.functions[handle.functionId.index]? = some declaration)
    (body_eq : declaration.body = .structured root)
    (shape_eq : shape = .ofDeclaration declaration)
    (body_agrees : ExprDenotation.Agrees unit handle.namespaceId root body) :
    FunctionDenotation.Agrees unit handle
      (nativeFunctionRelation unit shape body) := by
  subst shape
  rw [nativeFunctionRelation_ofDeclaration]
  exact functionRelation_agrees namespace_eq declaration_eq body_eq body_agrees

theorem nativeFunctionRelationAt_agrees {unit : ExecutableUnit}
    {handle : FunctionHandle} {ns : ValidatedNamespace}
    {declaration : FunctionDecl FunctionBody} {root : ExprId}
    {shape : FunctionShape} {body : ExprDenotation}
    (typeInstantiation : Array (TypeId × TypeId))
    (namespace_eq : unit.unit.namespaces[handle.namespaceId.index]? = some ns)
    (declaration_eq : ns.functions[handle.functionId.index]? = some declaration)
    (body_eq : declaration.body = .structured root)
    (shape_eq : shape = .ofDeclaration declaration)
    (body_agrees : ExprDenotation.Agrees unit handle.namespaceId root body) :
    FunctionDenotation.AgreesAt unit handle typeInstantiation
      (nativeFunctionRelationAt unit shape typeInstantiation body) := by
  subst shape
  intro initial arguments final outcome
  constructor
  · rintro ⟨frame, finalFrame, evaluatedState, control, frame_eq,
        body_step, outcome_eq, finalize_eq⟩
    exact .body handle typeInstantiation initial arguments ns declaration frame root
      finalFrame evaluatedState final control outcome namespace_eq declaration_eq
      frame_eq body_eq ((body_agrees _ _ _ _ _).mp body_step) outcome_eq finalize_eq
  · intro step
    cases step
    simp_all
    subst_vars
    exact ⟨_, _, _, _, by assumption,
      (body_agrees _ _ _ _ _).mpr (by assumption), by assumption, rfl⟩

/-- Project exact agreement of the state-retaining native call relation to
the public function `Spec`.  Generated callers reuse the relation theorem;
the callee body therefore has one agreement proof regardless of how many
concrete generic instantiations call it. -/
theorem nativeFunction_agrees_of_relation {unit : ExecutableUnit}
    {handle : FunctionHandle} {shape : FunctionShape} {body : ExprDenotation}
    (relation_agrees : FunctionDenotation.Agrees unit handle
      (nativeFunctionRelation unit shape body)) :
    ∀ arguments, Spec.Equiv (nativeFunction unit shape body arguments)
      (functionSpec unit handle arguments) := by
  intro arguments
  refine ⟨?_, ?_, ?_⟩
  · intro initial results final
    change nativeFunctionRelation unit shape body initial arguments final
        (.returned results) ↔
      BigStep.EvalFunction unit handle #[] initial arguments final
        (.returned results)
    exact relation_agrees initial arguments final (.returned results)
  · intro initial failure
    constructor
    · rintro ⟨frame, finalFrame, evaluatedState, control, final, frame_eq,
          body_step, outcome_eq, finalize_eq⟩
      refine ⟨final, (relation_agrees initial arguments final
        (.threw failure.1 failure.2)).mp ?_⟩
      exact ⟨frame, finalFrame, evaluatedState, control, frame_eq, body_step,
        outcome_eq, finalize_eq⟩
    · rintro ⟨final, step⟩
      rcases (relation_agrees initial arguments final
        (.threw failure.1 failure.2)).mpr step with
        ⟨frame, finalFrame, evaluatedState, control, frame_eq, body_step,
          outcome_eq, finalize_eq⟩
      exact ⟨frame, finalFrame, evaluatedState, control, final, frame_eq,
        body_step, outcome_eq, finalize_eq⟩
  · intro initial
    rfl

/-- Assemble the per-node agreement proofs at the public `Spec` boundary.
This is the theorem a generated verification theorem exposes. -/
theorem function_agrees {unit : ExecutableUnit} {handle : FunctionHandle}
    {ns : ValidatedNamespace} {declaration : FunctionDecl FunctionBody}
    {root : ExprId} {body : ExprDenotation}
    (namespace_eq : unit.unit.namespaces[handle.namespaceId.index]? = some ns)
    (declaration_eq : ns.functions[handle.functionId.index]? = some declaration)
    (body_eq : declaration.body = .structured root)
    (body_agrees : ExprDenotation.Agrees unit handle.namespaceId root body) :
    ∀ arguments, Spec.Equiv (function unit declaration body arguments)
      (functionSpec unit handle arguments) := by
  intro arguments
  refine ⟨?_, ?_, ?_⟩
  · intro initial results final
    constructor
    · rintro ⟨frame, finalFrame, evaluatedState, control, frame_eq,
          body_step, outcome_eq, finalize_eq⟩
      exact .body handle #[] initial arguments ns declaration frame root finalFrame
        evaluatedState final control (.returned results) namespace_eq declaration_eq
        frame_eq body_eq ((body_agrees _ _ _ _ _).mp body_step) outcome_eq
        finalize_eq
    · intro step
      cases step
      simp_all
      subst_vars
      exact ⟨_, _, _, _, by assumption,
        (body_agrees _ _ _ _ _).mpr (by assumption), by assumption, rfl⟩
  · intro initial failure
    constructor
    · rintro ⟨frame, finalFrame, evaluatedState, control, final, frame_eq,
          body_step, outcome_eq, finalize_eq⟩
      exact ⟨final, .body handle #[] initial arguments ns declaration frame root finalFrame
        evaluatedState final control (.threw failure.1 failure.2) namespace_eq
        declaration_eq frame_eq body_eq ((body_agrees _ _ _ _ _).mp body_step)
        outcome_eq finalize_eq⟩
    · rintro ⟨final, step⟩
      cases step
      simp_all
      subst_vars
      exact ⟨_, _, _, _, _, by assumption,
        (body_agrees _ _ _ _ _).mpr (by assumption), by assumption, rfl⟩
  · intro initial
    constructor <;> intro impossible <;> exact impossible.elim

theorem nativeFunction_agrees {unit : ExecutableUnit}
    {handle : FunctionHandle} {ns : ValidatedNamespace}
    {declaration : FunctionDecl FunctionBody} {root : ExprId}
    {shape : FunctionShape} {body : ExprDenotation}
    (namespace_eq : unit.unit.namespaces[handle.namespaceId.index]? = some ns)
    (declaration_eq : ns.functions[handle.functionId.index]? = some declaration)
    (body_eq : declaration.body = .structured root)
    (shape_eq : shape = .ofDeclaration declaration)
    (body_agrees : ExprDenotation.Agrees unit handle.namespaceId root body) :
    ∀ arguments, Spec.Equiv (nativeFunction unit shape body arguments)
      (functionSpec unit handle arguments) := by
  subst shape
  intro arguments
  rw [nativeFunction_ofDeclaration]
  exact function_agrees namespace_eq declaration_eq body_eq body_agrees arguments

end Denotation
end LeanerIR.Proofs
