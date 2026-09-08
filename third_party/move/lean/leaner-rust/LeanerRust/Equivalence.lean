-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerRust.Profile

/-!
# Rust-profile semantic alpha-equivalence

This module projects the currently source-renderable Rust-profile LIR fragment
to a provenance-free, alpha-normal form. It follows only reachable structured
bodies, assigns local identities by semantic first use, erases authored local
and binder spellings, and contracts only the exact Copy-result forwarding,
shift-distance cast, block nesting, repeated panic-guard administration,
enum-switch lowering, and redundant checked from-end slice-index
administration introduced by the standard-Rust renderer and rustc MIR.
The projection is intentionally narrower than arbitrary future LIR: unsupported
declaration families return an explicit error instead of being ignored.
-/

namespace LeanerIR.Rust.Equivalence

open LeanerIR
open LeanerIR.Validation

structure SemanticNominal where
  name : String
  generics : Array String
  abilities : Array Ability
  fields : Array (String × String)
  variants : Array (String × Option Int × Array (String × String))
  deriving Repr, BEq, Inhabited

structure SemanticFunction where
  name : String
  profile : Profile
  generics : Array String
  predicates : Array String
  parameters : Array (Bool × String)
  results : Array String
  locals : Array (Bool × String)
  body : String
  deriving Repr, BEq, Inhabited

structure SemanticNamespace where
  profile : Option Profile
  imports : Array NamespaceId
  metadata : Array ProfileValue
  nominals : Array SemanticNominal
  functions : Array SemanticFunction
  deriving Repr, BEq, Inhabited

structure SemanticUnit where
  profiles : Array ProfileConfig
  dependencies : Array (NamespaceId × Option Profile × Array String)
  namespaces : Array SemanticNamespace
  deriving Repr, BEq, Inhabited

private def fail {α : Type} (message : String) : Except String α := .error message

private def requireSome (message : String) : Option α → Except String α
  | some value => .ok value
  | none => .error message

private def nameProjection (unit : ValidatedUnit) (name : NameId) : Except String String := do
  let entry ← requireSome s!"semantic projection references missing name {name.index}"
    unit.tables.names[name.index]?
  pure s!"n{entry.namespaceId.index}::{entry.name}"

private def qualifiedProjection (unit : ValidatedUnit) (reference : QualifiedRef) :
    Except String String := do
  let entry ← requireSome
    s!"semantic projection references missing name {reference.name.index}"
    unit.tables.names[reference.name.index]?
  if entry.namespaceId != reference.namespaceId then
    fail s!"qualified reference {reference.name.index} has inconsistent namespace"
  else pure s!"n{reference.namespaceId.index}::{entry.name}"

private def lifetimeProjection (unit : ValidatedUnit) (lifetime : LifetimeId) :
    Except String String := do
  let declaration ← requireSome
    s!"semantic projection references missing lifetime {lifetime.index}"
    unit.tables.lifetimes[lifetime.index]?
  pure <| match declaration.kind with
    | .static => "static"
    | .parameter index => s!"parameter:{index}"
    | .inference => "inference"
    | .local => "local"

mutual
  private partial def typeProjection (unit : ValidatedUnit) (typeId : TypeId)
      (fuel : Nat) : Except String String := do
    if fuel == 0 then fail "cyclic type in Rust semantic projection"
    let ty ← requireSome s!"semantic projection references missing type {typeId.index}"
      unit.tables.types[typeId.index]?
    match ty with
    | .unit => pure "unit"
    | .never => pure "never"
    | .bool => pure "bool"
    | .character => pure "char"
    | .string => pure "string"
    | .bytes => pure "bytes"
    | .address => pure "address"
    | .signer => pure "signer"
    | .integer width signed => pure s!"integer:{repr width}:{signed}"
    | .tuple elements =>
        pure s!"tuple:{repr (← elements.mapM (typeProjection unit · (fuel - 1)))}"
    | .vector element length =>
        pure s!"vector:{← typeProjection unit element (fuel - 1)}:{repr length}"
    | .range => pure "range"
    | .eventStore => pure "eventStore"
    | .typeDomain type => pure s!"typeDomain:{← typeProjection unit type (fuel - 1)}"
    | .resourceDomain resource arguments =>
        let arguments ← arguments.mapM fun values =>
          values.mapM (typeProjection unit · (fuel - 1))
        pure s!"resourceDomain:{← nameProjection unit resource}:{repr arguments}"
    | .stateDomain => pure "stateDomain"
    | .nominal name arguments =>
        pure s!"nominal:{← nameProjection unit name}:{repr (← arguments.mapM (genericArgumentProjection unit · (fuel - 1)))}"
    | .function arguments result abilities =>
        pure s!"function:{repr (← arguments.mapM (typeProjection unit · (fuel - 1)))}:{← typeProjection unit result (fuel - 1)}:{repr abilities}"
    | .typeParameter index => pure s!"typeParameter:{index}"
    | .reference reference =>
        pure s!"reference:{repr reference.profile}:{repr reference.kind}:{← lifetimeProjection unit reference.lifetime}:{← typeProjection unit reference.referent (fuel - 1)}"
    | .profile value => pure s!"profile:{repr value}"

  private partial def genericArgumentProjection (unit : ValidatedUnit)
      (argument : GenericArgument) (fuel : Nat) : Except String String :=
    match argument with
    | .typeArg value => typeProjection unit value.typeId fuel
    | .const value => pure s!"const:{repr value}"
    | .lifetime lifetime => do
        pure s!"lifetime:{← lifetimeProjection unit lifetime}"
    | .evidence evidence => pure s!"evidence:{evidence.index}"
end

private partial def predicateProjection (unit : ValidatedUnit)
    (predicate : GenericPredicate) (fuel : Nat) : Except String String := do
  if fuel == 0 then fail "cyclic predicate in Rust semantic projection"
  let traitProjection (trait : TraitRef) := do
    pure s!"{← qualifiedProjection unit trait.trait}:{repr (← trait.arguments.mapM (genericArgumentProjection unit · (fuel - 1)))}"
  match predicate with
  | .ability type ability =>
      pure s!"ability:{← typeProjection unit type fuel}:{repr ability}"
  | .implements type trait =>
      pure s!"implements:{← typeProjection unit type fuel}:{← traitProjection trait}"
  | .associatedTypeEq trait item value =>
      pure s!"associatedType:{← traitProjection trait}:{item.index}:{← typeProjection unit value fuel}"
  | .associatedConstEq trait item value =>
      pure s!"associatedConst:{← traitProjection trait}:{item.index}:{repr value}"
  | .lifetimeOutlives longer shorter =>
      pure s!"outlives:{← lifetimeProjection unit longer}:{← lifetimeProjection unit shorter}"
  | .constEq left right => pure s!"constEq:{repr left}:{repr right}"
  | .profile value => pure s!"profile:{repr value}"

private def binderProjection (unit : ValidatedUnit) (binder : GenericBinder) :
    Except String String := do
  let predicates ← binder.predicates.mapM fun predicate =>
    predicateProjection unit predicate (unit.tables.types.size + 1)
  let type ← match binder.type with
    | some type => some <$> typeProjection unit type.typeId (unit.tables.types.size + 1)
    | none => pure none
  pure s!"{repr binder.kind}:{repr type}:{repr binder.abilities}:{repr predicates}"

private partial def typeIsCopy (unit : ValidatedUnit) (typeId : TypeId) (fuel : Nat) : Bool :=
  if fuel == 0 then false else
  match unit.tables.types[typeId.index]? with
  | some .unit | some .never | some .bool | some .character | some .string |
      some .bytes | some .address |
      some (.integer ..) | some .range => true
  | some (.reference reference) => reference.kind == .shared
  | some (.tuple elements) => elements.all (typeIsCopy unit · (fuel - 1))
  | some (.vector element _) => typeIsCopy unit element (fuel - 1)
  | some (.function ..) => true
  | some (.nominal name _) => unit.namespaces.any fun ns =>
      (ns.structs.find? (·.name == name)).any (·.abilities.contains .copy)
  | _ => false

private structure LocalProjectionState where
  mappings : Array (LocalId × Nat) := #[]
  shiftCastSubstitutions : Array (LocalId × ExprId) := #[]
  valueSubstitutions : Array (LocalId × ExprId) := #[]
  localTypes : Array TypeId := #[]
  normalizePureAdministration : Bool := false
  next : Nat := 0
  deriving Inhabited

private abbrev ProjectionM := StateT LocalProjectionState (Except String)

private def localProjection (localId : LocalId) : ProjectionM Nat := do
  let state ← get
  match state.mappings.find? (·.1 == localId) with
  | some mapping => pure mapping.2
  | none =>
      let nextState : LocalProjectionState := { state with
        mappings := state.mappings.push (localId, state.next), next := state.next + 1 }
      set nextState
      pure state.next

private def liftProjection (result : Except String α) : ProjectionM α :=
  StateT.lift result

private def directLocalPlace? (ns : ValidatedNamespace) (place : PlaceId) : Option LocalId :=
  match ns.places[place.index]? with
  | some (.localVar localId) => some localId
  | _ => none

private def indexedDirectLocalPlace? (ns : ValidatedNamespace) (place : PlaceId) :
    Option (LocalId × ExprId) := do
  let .index base index ← ns.places[place.index]? | none
  some (← directLocalPlace? ns base, index)

private def loadedLocal? (ns : ValidatedNamespace) (expression : ExprId) : Option LocalId := do
  let expression ← ns.expressions[expression.index]?
  let .operation operation _ _ _ := expression.kind | none
  let place ← match operation with
    | .move place | .copy place | .read place => some place
    | _ => none
  directLocalPlace? ns place

private def singleLocalAssignment? (ns : ValidatedNamespace) (expressionId : ExprId) :
    Option (LocalId × ExprId) := do
  let expression ← ns.expressions[expressionId.index]?
  let .block statements none := expression.kind | none
  let [statementId] := statements.toList | none
  let statement ← ns.expressions[statementId.index]?
  let .assign target value := statement.kind | none
  some (← directLocalPlace? ns target, value)

private def conditionalForwarding? (ns : ValidatedNamespace)
    (statements : Array ExprId) (result : Option ExprId) :
    Option (ExprId × ExprId × ExprId) := do
  let [conditionalId] := statements.toList | none
  let conditional ← ns.expressions[conditionalId.index]?
  let .ifElse condition thenBranch (some elseBranch) := conditional.kind | none
  let (thenTarget, thenValue) ← singleLocalAssignment? ns thenBranch
  let (elseTarget, elseValue) ← singleLocalAssignment? ns elseBranch
  if thenTarget != elseTarget then none
  let resultId ← result
  let result ← ns.expressions[resultId.index]?
  let .return_ returned := result.kind | none
  let [returned] := returned.toList | none
  if loadedLocal? ns returned == some thenTarget then
    some (condition, thenValue, elseValue)
  else none

private def matchForwarding? (ns : ValidatedNamespace)
    (statements : Array ExprId) (result : Option ExprId) :
    Option (ExprId × Array (PatternId × ExprId)) := do
  let [matchId] := statements.toList | none
  let matchExpression ← ns.expressions[matchId.index]?
  let .match_ scrutinee arms := matchExpression.kind | none
  if arms.isEmpty || (loadedLocal? ns scrutinee).isNone then none
  let mut target : Option LocalId := none
  let mut forwarded := #[]
  for arm in arms do
    if arm.guard.isSome then none
    let (armTarget, value) ← singleLocalAssignment? ns arm.body
    match target with
    | none => target := some armTarget
    | some previous => if previous != armTarget then none
    forwarded := forwarded.push (arm.pattern, value)
  let resolvedTarget ← target
  let resultId ← result
  let result ← ns.expressions[resultId.index]?
  let .return_ returned := result.kind | none
  let [returned] := returned.toList | none
  if loadedLocal? ns returned == some resolvedTarget then some (scrutinee, forwarded)
  else none

private def forwardedReturnValue? (ns : ValidatedNamespace) (statements : Array ExprId)
    (result : Option ExprId) : Option ExprId := do
  let [statement] := statements.toList | none
  let statement ← ns.expressions[statement.index]?
  let .assign target value := statement.kind | none
  let target ← directLocalPlace? ns target
  if loadedLocal? ns value == some target then none else
  let result ← result
  let result ← ns.expressions[result.index]?
  let .return_ values := result.kind | none
  let [returned] := values.toList | none
  let returned ← loadedLocal? ns returned
  if returned == target then some value else none

private def forwardedReturnTail? (ns : ValidatedNamespace) (statements : Array ExprId)
    (result : Option ExprId) : Option (Array ExprId × ExprId × TypeId) := do
  let statementId ← statements.back?
  let statement ← ns.expressions[statementId.index]?
  let .assign target value := statement.kind | none
  let target ← directLocalPlace? ns target
  let source ← loadedLocal? ns value
  if target == source then none else
  let resultId ← result
  let result ← ns.expressions[resultId.index]?
  let .return_ values := result.kind | none
  let [returned] := values.toList | none
  let returned ← loadedLocal? ns returned
  if returned == target then
    some (statements.extract 0 (statements.size - 1), value, result.typeId)
  else none

private def forwardedAssignment? (ns : ValidatedNamespace) (statements : Array ExprId)
    (result : Option ExprId) : Option (PlaceId × ExprId × TypeId) := do
  let [statementId] := statements.toList | none
  let statement ← ns.expressions[statementId.index]?
  let .assign temporaryPlace value := statement.kind | none
  let temporary ← directLocalPlace? ns temporaryPlace
  let resultId ← result
  let result ← ns.expressions[resultId.index]?
  let .block forwarded none := result.kind | none
  let [forwardedId] := forwarded.toList | none
  let forwarded ← ns.expressions[forwardedId.index]?
  let .assign targetPlace loaded := forwarded.kind | none
  let loaded ← loadedLocal? ns loaded
  if loaded == temporary then some (targetPlace, value, forwarded.typeId) else none

private def isPanicGuardBlock? (ns : ValidatedNamespace) (expressionId : ExprId) : Bool :=
  match ns.expressions[expressionId.index]? with
  | some { kind := .block _ (some resultId), .. } =>
      match ns.expressions[resultId.index]? with
      | some { kind := .ifElse _ panicBranch (some _), .. } =>
          match ns.expressions[panicBranch.index]? with
          | some { kind := .throw_ .panic _, .. } => true
          | _ => false
      | _ => false
  | _ => false

private partial def flattenedBlock (ns : ValidatedNamespace) (statements : Array ExprId)
    (result : Option ExprId) (fuel : Nat) : Array ExprId × Option ExprId :=
  if fuel == 0 then (statements, result) else
  match result with
  | some resultId => match ns.expressions[resultId.index]? with
    | some { kind := .block nestedStatements nestedResult, .. } =>
        if isPanicGuardBlock? ns resultId then (statements, result)
        else flattenedBlock ns (statements ++ nestedStatements) nestedResult (fuel - 1)
    | _ => (statements, result)
  | _ => (statements, result)

/-- Flatten administrative block shells for a recognizer that subsequently
checks the complete guard shape itself. -/
private partial def flattenedAdminBlock (ns : ValidatedNamespace)
    (statements : Array ExprId) (result : Option ExprId) (fuel : Nat) :
    Array ExprId × Option ExprId :=
  if fuel == 0 then (statements, result) else
  match result with
  | some resultId => match ns.expressions[resultId.index]? with
    | some { kind := .block nestedStatements nestedResult, .. } =>
        flattenedAdminBlock ns (statements ++ nestedStatements) nestedResult (fuel - 1)
    | _ => (statements, result)
  | none => (statements, result)

private partial def trailingBoolAssignments (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (statements : Array ExprId) (count : Nat := 0) : Nat :=
  if count == statements.size then count else
  match statements[statements.size - count - 1]? with
  | some statementId => match ns.expressions[statementId.index]? with
    | some { kind := .assign _ value, .. } =>
        match ns.expressions[value.index]? with
        | some expression => match unit.tables.types[expression.typeId.index]? with
          | some .bool => trailingBoolAssignments unit ns statements (count + 1)
          | _ => count
        | _ => count
    | _ => count
  | _ => count

private def trailingPanicGuardSplit? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (statements : Array ExprId) (result : Option ExprId) :
    Option (Array ExprId × Array ExprId × ExprId × ExprId × ExprId × TypeId) := do
  let resultId ← result
  let resultExpression ← ns.expressions[resultId.index]?
  let .ifElse condition panicBranch (some continuation) := resultExpression.kind | none
  let panic ← ns.expressions[panicBranch.index]?
  let .throw_ .panic _ := panic.kind | none
  let count := trailingBoolAssignments unit ns statements
  if count == 0 || count == statements.size then none else
  let suffix := statements.extract (statements.size - count) statements.size
  let lastId ← suffix.back?
  let last ← ns.expressions[lastId.index]?
  let .assign target _ := last.kind | none
  let target ← directLocalPlace? ns target
  let conditionLocal ← loadedLocal? ns condition
  if conditionLocal != target then none else
  some (statements.extract 0 (statements.size - count), suffix, condition,
    panicBranch, continuation, resultExpression.typeId)

/-- rustc makes the `as u32` required by `wrapping_shl`/`wrapping_shr`
explicit in MIR. The LIR shift primitive accepts an integer distance directly,
so source rendering introduces this one administrative widening cast. -/
private def forwardedShiftCast? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (statements : Array ExprId) (result : Option ExprId) : Option (LocalId × ExprId × ExprId) := do
  let [statementId] := statements.toList | none
  let statement ← ns.expressions[statementId.index]?
  let .assign temporaryPlace castId := statement.kind | none
  let temporary ← directLocalPlace? ns temporaryPlace
  let cast ← ns.expressions[castId.index]?
  let .operation (.primitive .cast) _ castArguments _ := cast.kind | none
  let [source] := castArguments.toList | none
  let some (.integer (.bits 32) false) := unit.tables.types[cast.typeId.index]? | none
  let sourceExpression ← ns.expressions[source.index]?
  let some (.integer _ false) := unit.tables.types[sourceExpression.typeId.index]? | none
  let resultId ← result
  let resultExpression ← ns.expressions[resultId.index]?
  let .block shiftStatements _ := resultExpression.kind | none
  let shiftStatementId ← shiftStatements[0]?
  let shiftStatement ← ns.expressions[shiftStatementId.index]?
  let .assign _ shiftId := shiftStatement.kind | none
  let shift ← ns.expressions[shiftId.index]?
  let .operation (.primitive operation) _ shiftArguments _ := shift.kind | none
  if operation != .shiftLeft && operation != .shiftRight then none else
  let [_, distance] := shiftArguments.toList | none
  let distanceLocal ← loadedLocal? ns distance
  if distanceLocal == temporary then some (temporary, source, resultId) else none

private partial def pureInlineableExpression? (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (expressionId : ExprId) (fuel : Nat) : Bool :=
  match fuel, ns.expressions[expressionId.index]? with
  | 0, _ | _, none => false
  | _, some { kind := .value .., .. } | _, some { kind := .localVar _, .. } => true
  | fuel + 1, some { kind := .operation operation _ arguments _, .. } =>
      (operation matches .move _ || operation matches .copy _ || operation matches .read _ ||
        operation matches .primitive _) &&
        arguments.all (pureInlineableExpression? unit ns · fuel)
  | _, some { kind := .block statements result, .. } =>
      (forwardedShiftCast? unit ns statements result).isSome
  | _, _ => false

/-- Recognize rustc's pure temporary chain ending in a result or returned
temporary. The LeanerLang source backend inlines precisely this shape, so
executable semantic comparison must erase it as well. -/
private def pureAdministrativeResult? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (parameterMappings : Array (LocalId × Nat)) (statements : Array ExprId)
    (result : Option ExprId) : Option (Array (LocalId × ExprId) × ExprId) := do
  if statements.isEmpty then none
  let mut substitutions : Array (LocalId × ExprId) := #[]
  for statementId in statements do
    let statement ← ns.expressions[statementId.index]?
    let .assign target value := statement.kind | none
    let localId ← directLocalPlace? ns target
    if parameterMappings.any (·.1 == localId) || substitutions.any (·.1 == localId) ||
        !pureInlineableExpression? unit ns value ns.expressions.size.succ then
      none
    substitutions := substitutions.push (localId, value)
  let resultId ← result
  let result ← ns.expressions[resultId.index]?
  let value := match result.kind with
    | .return_ values => do
        let [returned] := values.toList | none
        let localId ← loadedLocal? ns returned
        (substitutions.find? (·.1 == localId)).map (·.2)
    | _ => some resultId
  some (substitutions, ← value)

private structure DuplicatePanicGuard where
  outerPredicate : ExprId
  innerPredicate : ExprId
  condition : ExprId
  panicBranch : ExprId
  finalBranch : ExprId
  resultType : TypeId

private def duplicatePanicGuard? (ns : ValidatedNamespace) (statements : Array ExprId)
    (result : Option ExprId) : Option DuplicatePanicGuard := do
  let [statementId] := statements.toList | none
  let statement ← ns.expressions[statementId.index]?
  let .assign outerPlace outerPredicate := statement.kind | none
  let outerGuard ← directLocalPlace? ns outerPlace
  let resultId ← result
  let resultExpression ← ns.expressions[resultId.index]?
  let .ifElse condition panicBranch (some nestedId) := resultExpression.kind | none
  let conditionGuard ← loadedLocal? ns condition
  if conditionGuard != outerGuard then none else
  let panicExpression ← ns.expressions[panicBranch.index]?
  let .throw_ .panic _ := panicExpression.kind | none
  let nested ← ns.expressions[nestedId.index]?
  let .block nestedStatements (some nestedResultId) := nested.kind | none
  let [nestedStatementId] := nestedStatements.toList | none
  let nestedStatement ← ns.expressions[nestedStatementId.index]?
  let .assign innerPlace innerPredicate := nestedStatement.kind | none
  let innerGuard ← directLocalPlace? ns innerPlace
  let nestedResult ← ns.expressions[nestedResultId.index]?
  let .ifElse innerCondition innerPanic (some finalBranch) := nestedResult.kind | none
  let innerConditionGuard ← loadedLocal? ns innerCondition
  if innerConditionGuard != innerGuard then none else
  let innerPanicExpression ← ns.expressions[innerPanic.index]?
  let .throw_ .panic _ := innerPanicExpression.kind | none
  some {
    outerPredicate := outerPredicate
    innerPredicate := innerPredicate
    condition := condition
    panicBranch := panicBranch
    finalBranch := finalBranch
    resultType := resultExpression.typeId }

private structure PanicGuardNode where
  expression : ExprId
  statements : Array ExprId
  condition : ExprId
  panicBranch : ExprId
  continuation : ExprId
  panicWhenTrue : Bool

private def panicGuardNode? (ns : ValidatedNamespace) (expressionId : ExprId) :
    Option PanicGuardNode := do
  let expression ← ns.expressions[expressionId.index]?
  let .block statements (some resultId) := expression.kind | none
  let result ← ns.expressions[resultId.index]?
  let .ifElse condition thenBranch (some elseBranch) := result.kind | none
  match ns.expressions[thenBranch.index]?, ns.expressions[elseBranch.index]? with
  | some { kind := .throw_ .panic _, .. }, _ => some {
      expression := expressionId
      statements
      condition
      panicBranch := thenBranch
      continuation := elseBranch
      panicWhenTrue := true }
  | _, some { kind := .throw_ .panic _, .. } => some {
      expression := expressionId
      statements
      condition
      panicBranch := elseBranch
      continuation := thenBranch
      panicWhenTrue := false }
  | _, _ => none

private partial def panicGuardChain (ns : ValidatedNamespace) (expressionId : ExprId)
    (fuel : Nat) : Array PanicGuardNode :=
  if fuel == 0 then #[] else
  match panicGuardNode? ns expressionId with
  | none => #[]
  | some node => #[node] ++ panicGuardChain ns node.continuation (fuel - 1)

private partial def duplicatePrefixLength? (values : Array String) (candidate : Nat := 1) :
    Option Nat :=
  if candidate > values.size / 2 then none
  else if values.extract 0 candidate == values.extract candidate (2 * candidate) then
    some candidate
  else duplicatePrefixLength? values (candidate + 1)

private def panicGuardValues? (ns : ValidatedNamespace) (node : PanicGuardNode) :
    Option (Array ExprId) := do
  let values ← node.statements.mapM fun statementId => do
    let statement ← ns.expressions[statementId.index]?
    let .assign target value := statement.kind | none
    let _ ← directLocalPlace? ns target
    some value
  some (values.push node.condition)

private structure FromEndLoadAdmin where
  root : LocalId
  target : PlaceId
  offset : Nat
  deriving Repr

private def assignedLocal? (ns : ValidatedNamespace) (expressionId : ExprId) :
    Option (LocalId × ExprId) := do
  let expression ← ns.expressions[expressionId.index]?
  let .assign target value := expression.kind | none
  some (← directLocalPlace? ns target, value)

private def loadedPlace? (ns : ValidatedNamespace) (expressionId : ExprId) : Option PlaceId := do
  let expression ← ns.expressions[expressionId.index]?
  let .operation operation _ arguments _ := expression.kind | none
  if !arguments.isEmpty then none
  match operation with
  | .move place | .copy place | .read place => some place
  | _ => none

private def dereferencedLocalPlace? (ns : ValidatedNamespace) (placeId : PlaceId) :
    Option LocalId := do
  let .deref base ← ns.places[placeId.index]? | none
  directLocalPlace? ns base

private def lengthOfDereferencedLocal? (ns : ValidatedNamespace) (expressionId : ExprId) :
    Option LocalId := do
  let expression ← ns.expressions[expressionId.index]?
  let .operation (.primitive .length) _ arguments _ := expression.kind | none
  let [argument] := arguments.toList | none
  let dereference ← ns.expressions[argument.index]?
  let .operation (.reference .dereference) _ dereferenceArguments _ :=
    dereference.kind | none
  let [source] := dereferenceArguments.toList | none
  loadedLocal? ns source

private def loadedDereferenceOf? (ns : ValidatedNamespace) (expressionId : ExprId)
    (expected : LocalId) : Bool :=
  (loadedPlace? ns expressionId).bind (dereferencedLocalPlace? ns) == some expected

private def directFromEndLoad? (ns : ValidatedNamespace) (expressionId : ExprId) :
    Option FromEndLoadAdmin := do
  let expression ← ns.expressions[expressionId.index]?
  let .block statements none := expression.kind | none
  let [borrowStatement, loadStatement] := statements.toList | none
  let (borrowLocal, borrowValue) ← assignedLocal? ns borrowStatement
  let borrow ← ns.expressions[borrowValue.index]?
  let .operation (.borrow .immutable indexPlace) _ arguments _ := borrow.kind | none
  if !arguments.isEmpty then none
  let .index base indexExpression ← ns.places[indexPlace.index]? | none
  let some (.fromEnd source offset) := Validation.placeIndexForm? ns indexExpression | none
  let root ← dereferencedLocalPlace? ns base
  if dereferencedLocalPlace? ns source != some root then none
  let load ← ns.expressions[loadStatement.index]?
  let .assign target loadValue := load.kind | none
  if !loadedDereferenceOf? ns loadValue borrowLocal then none
  some { root, target, offset }

private def checkedFromEndLoad? (ns : ValidatedNamespace) (expressionId : ExprId) :
    Option FromEndLoadAdmin := do
  let expression ← ns.expressions[expressionId.index]?
  let .block initialStatements initialResult := expression.kind | none
  let (statements, some resultId) :=
    flattenedAdminBlock ns initialStatements initialResult (ns.expressions.size + 1) | none
  let result ← ns.expressions[resultId.index]?
  let .ifElse condition success (some panic) := result.kind | none
  let conditionLocal ← loadedLocal? ns condition
  let assignments ← statements.mapM (assignedLocal? ns)
  let conditionValue ← (assignments.find? (·.1 == conditionLocal)).map (·.2)
  let conditionExpression ← ns.expressions[conditionValue.index]?
  let .operation (.primitive .less) _ conditionArguments _ := conditionExpression.kind | none
  let [loadedIndex, loadedBound] := conditionArguments.toList | none
  let indexLocal ← loadedLocal? ns loadedIndex
  let boundLocal ← loadedLocal? ns loadedBound
  let boundValue ← (assignments.find? (·.1 == boundLocal)).map (·.2)
  let root ← lengthOfDereferencedLocal? ns boundValue
  let indexValue ← (assignments.find? (·.1 == indexLocal)).map (·.2)
  let index ← ns.expressions[indexValue.index]?
  let .operation (.primitive .subtract) _ indexArguments _ := index.kind | none
  let [loadedLength, offsetId] := indexArguments.toList | none
  let lengthLocal ← loadedLocal? ns loadedLength
  let lengthValue ← (assignments.find? (·.1 == lengthLocal)).map (·.2)
  if lengthOfDereferencedLocal? ns lengthValue != some root then none
  let offsetExpression ← ns.expressions[offsetId.index]?
  let .value (.integer offset) _ := offsetExpression.kind | none
  if offset < 0 then none
  let panicExpression ← ns.expressions[panic.index]?
  let .throw_ .panic panicArguments := panicExpression.kind | none
  if !panicArguments.isEmpty then none
  let successExpression ← ns.expressions[success.index]?
  let .block successStatements none := successExpression.kind | none
  let [borrowStatement, loadStatement, forwardStatement] := successStatements.toList | none
  let (borrowLocal, borrowValue) ← assignedLocal? ns borrowStatement
  let borrow ← ns.expressions[borrowValue.index]?
  let .operation (.borrow .immutable indexPlace) _ borrowArguments _ := borrow.kind | none
  if !borrowArguments.isEmpty then none
  let .index base indexExpression ← ns.places[indexPlace.index]? | none
  if dereferencedLocalPlace? ns base != some root ||
      loadedLocal? ns indexExpression != some indexLocal then none
  let (valueLocal, loadValue) ← assignedLocal? ns loadStatement
  if !loadedDereferenceOf? ns loadValue borrowLocal then none
  let forward ← ns.expressions[forwardStatement.index]?
  let .assign target forwardedValue := forward.kind | none
  if loadedLocal? ns forwardedValue != some valueLocal then none
  some { root, target, offset := offset.toNat }

private def fromEndLoad? (ns : ValidatedNamespace) (expressionId : ExprId) :
    Option FromEndLoadAdmin :=
  directFromEndLoad? ns expressionId <|> checkedFromEndLoad? ns expressionId

private structure CheckedIndexLoad where
  success : ExprId
  substitution : Option (LocalId × ExprId)

private def assignedValue? (ns : ValidatedNamespace) (statements : Array ExprId)
    (localId : LocalId) : Option ExprId :=
  statements.findSome? fun statementId => do
    let (target, value) ← assignedLocal? ns statementId
    if target == localId then some value else none

private def resolveAssignedValue (ns : ValidatedNamespace) (statements : Array ExprId)
    (expressionId : ExprId) : ExprId :=
  match loadedLocal? ns expressionId with
  | some localId => (assignedValue? ns statements localId).getD expressionId
  | none => expressionId

private def directIndexedReturn? (ns : ValidatedNamespace) (expressionId : ExprId) :
    Option (PlaceId × ExprId) := do
  let expression ← ns.expressions[expressionId.index]?
  let .block initialStatements initialResult := expression.kind | none
  let (statements, some resultId) :=
    flattenedBlock ns initialStatements initialResult (ns.expressions.size + 1) | none
  let [statementId] := statements.toList | none
  let statement ← ns.expressions[statementId.index]?
  let .assign targetPlace valueId := statement.kind | none
  let target ← directLocalPlace? ns targetPlace
  let value ← ns.expressions[valueId.index]?
  let .operation operation _ arguments _ := value.kind | none
  if !arguments.isEmpty then none
  let indexedPlace ← match operation with
    | .move place | .copy place | .read place => some place
    | _ => none
  let place ← ns.places[indexedPlace.index]?
  let .index base index := place | none
  let result ← ns.expressions[resultId.index]?
  let .return_ returned := result.kind | none
  let [returned] := returned.toList | none
  if loadedLocal? ns returned == some target then some (base, index) else none

private def fixedVectorLength? (unit : ValidatedUnit) (state : LocalProjectionState)
    (ns : ValidatedNamespace) (placeId : PlaceId) : Option Nat := do
  let localId ← directLocalPlace? ns placeId
  let typeId ← state.localTypes[localId.index]?
  let .vector _ (some (.integer length)) ← unit.tables.types[typeId.index]? | none
  if length < 0 then none else some length.toNat

private def indexBoundMatches? (unit : ValidatedUnit) (state : LocalProjectionState)
    (ns : ValidatedNamespace) (statements : Array ExprId) (base : PlaceId)
    (boundId : ExprId) : Bool :=
  let boundId := resolveAssignedValue ns statements boundId
  match ns.expressions[boundId.index]? with
  | some { kind := .value (.integer bound) _, .. } =>
      bound >= 0 && fixedVectorLength? unit state ns base == some bound.toNat
  | _ => match lengthOfDereferencedLocal? ns boundId, dereferencedLocalPlace? ns base with
      | some lengthRoot, some baseRoot => lengthRoot == baseRoot
      | _, _ => false

private partial def checkedIndexSuccess? (unit : ValidatedUnit)
    (state : LocalProjectionState) (ns : ValidatedNamespace) (expressionId : ExprId)
    (fuel : Nat) : Option (PlaceId × LocalId × ExprId × Array ExprId × Nat) := do
  if fuel == 0 then none
  match directIndexedReturn? ns expressionId with
  | some (base, index) =>
      let indexLocal ← loadedLocal? ns index
      some (base, indexLocal, expressionId, #[], 0)
  | none =>
      let expression ← ns.expressions[expressionId.index]?
      let .block initialStatements initialResult := expression.kind | none
      let (statements, some resultId) :=
        flattenedBlock ns initialStatements initialResult fuel | none
      let result ← ns.expressions[resultId.index]?
      let .ifElse condition success (some panic) := result.kind | none
      let panicExpression ← ns.expressions[panic.index]?
      let .throw_ .panic panicArguments := panicExpression.kind | none
      if !panicArguments.isEmpty then none
      let (base, indexLocal, finalSuccess, nestedStatements, guardCount) ←
        checkedIndexSuccess? unit state ns success (fuel - 1)
      let predicateId := resolveAssignedValue ns statements condition
      let predicate ← ns.expressions[predicateId.index]?
      let .operation (.primitive .less) _ arguments _ := predicate.kind | none
      let [checkedIndex, checkedBound] := arguments.toList | none
      if loadedLocal? ns checkedIndex != some indexLocal ||
          !indexBoundMatches? unit state ns statements base checkedBound then none
      some (base, indexLocal, finalSuccess, statements ++ nestedStatements, guardCount + 1)

private def checkedIndexLoad? (unit : ValidatedUnit) (state : LocalProjectionState)
    (ns : ValidatedNamespace) (expressionId : ExprId) : Option CheckedIndexLoad := do
  let (_, indexLocal, success, statements, guardCount) ←
    checkedIndexSuccess? unit state ns expressionId (ns.expressions.size + 1)
  if guardCount == 0 then none
  let value ← assignedValue? ns statements indexLocal
  some { success, substitution := some (indexLocal, value) }

/-! The standard-Rust renderer spells an enum discriminant and a downcast-field
read as borrowed source `match` expressions. rustc lowers those expressions to
an exhaustive discriminant switch, temporary shared references, and (for a
field) a second switch whose non-selected arms diverge. The following small
symbolic domain recognizes only that exact administration. -/

private inductive EnumAdminPlace where
  | localVar (localId : LocalId)
  | field (base : EnumAdminPlace) (field : NameId)
  | downcast (base : EnumAdminPlace) (variant : NameId)
  deriving Repr, BEq

private inductive EnumAdminValue where
  | place (place : EnumAdminPlace)
  | borrow (place : EnumAdminPlace)
  | discriminant (type : QualifiedRef) (place : EnumAdminPlace)
  | integer (value : Int)
  | primitive (operation : PrimitiveOperation) (arguments : Array EnumAdminValue)
  | conditional (condition thenValue elseValue : EnumAdminValue)
  deriving Repr, BEq

private abbrev EnumAdminEnv := Array (LocalId × EnumAdminValue)

private def enumAdminLookup (environment : EnumAdminEnv) (localId : LocalId) :
    Option EnumAdminValue :=
  (environment.find? (·.1 == localId)).map (·.2)

private def enumAdminSet (environment : EnumAdminEnv) (localId : LocalId)
    (value : EnumAdminValue) : EnumAdminEnv :=
  (environment.filter (·.1 != localId)).push (localId, value)

private partial def enumAdminPlace? (ns : ValidatedNamespace) (environment : EnumAdminEnv)
    (placeId : PlaceId) (fuel : Nat) : Option EnumAdminPlace := do
  if fuel == 0 then none else
  let place ← ns.places[placeId.index]?
  match place with
  | .localVar localId => some (.localVar localId)
  | .deref base => do
      let localId ← directLocalPlace? ns base
      let .borrow place ← enumAdminLookup environment localId | none
      some place
  | .field base _ field =>
      some (.field (← enumAdminPlace? ns environment base (fuel - 1)) field)
  | .downcast base variant =>
      some (.downcast (← enumAdminPlace? ns environment base (fuel - 1)) variant)
  | .index .. | .subslice .. => none

private partial def enumAdminValue? (ns : ValidatedNamespace) (environment : EnumAdminEnv)
    (expressionId : ExprId) (fuel : Nat) : Option EnumAdminValue := do
  if fuel == 0 then none else
  let expression ← ns.expressions[expressionId.index]?
  match expression.kind with
  | .value (.integer value) _ => some (.integer value)
  | .operation operation _ arguments _ => match operation with
      | .move place | .copy place | .read place =>
          match directLocalPlace? ns place with
          | some localId => some <|
              (enumAdminLookup environment localId).getD (.place (.localVar localId))
          | none => some (.place (← enumAdminPlace? ns environment place (fuel - 1)))
      | .borrow .immutable place =>
          some (.borrow (← enumAdminPlace? ns environment place (fuel - 1)))
      | .data (.discriminant type) => do
          let [argument] := arguments.toList | none
          let .place place ← enumAdminValue? ns environment argument (fuel - 1) | none
          some (.discriminant type place)
      | .primitive operation =>
          some (.primitive operation (← arguments.mapM fun argument =>
            enumAdminValue? ns environment argument (fuel - 1)))
      | _ => none
  | _ => none

private def enumAdminAssignment? (ns : ValidatedNamespace) (environment : EnumAdminEnv)
    (expressionId : ExprId) (fuel : Nat) : Option (LocalId × EnumAdminValue) := do
  let expression ← ns.expressions[expressionId.index]?
  let .assign target value := expression.kind | none
  let localId ← directLocalPlace? ns target
  some (localId, ← enumAdminValue? ns environment value fuel)

private partial def enumAdminSingleIdentityAssignment? (ns : ValidatedNamespace)
    (expressionId : ExprId) (expected : Int) (fuel : Nat) : Option LocalId := do
  if fuel == 0 then none else
  let expression ← ns.expressions[expressionId.index]?
  match expression.kind with
  | .letDecl _ none body =>
      enumAdminSingleIdentityAssignment? ns body expected (fuel - 1)
  | .block statements none => do
      let [statement] := statements.toList | none
      let assignment ← ns.expressions[statement.index]?
      let .assign target value := assignment.kind | none
      let localId ← directLocalPlace? ns target
      let value ← ns.expressions[value.index]?
      let .value (.integer actual) _ := value.kind | none
      if actual == expected then some localId else none
  | _ => none

private def enumAdminIdentityMatch? (ns : ValidatedNamespace) (environment : EnumAdminEnv)
    (expressionId : ExprId) (fuel : Nat) : Option (LocalId × EnumAdminValue) := do
  let expression ← ns.expressions[expressionId.index]?
  let .match_ scrutinee arms := expression.kind | none
  let value@(.discriminant ..) ← enumAdminValue? ns environment scrutinee fuel | none
  if arms.isEmpty then none else
  let mut target : Option LocalId := none
  for arm in arms do
    if arm.guard.isSome then none
    let pattern ← ns.patterns[arm.pattern.index]?
    let .literal (.integer discriminant) := pattern.kind | none
    let armTarget ← enumAdminSingleIdentityAssignment? ns arm.body discriminant fuel
    match target with
    | none => target := some armTarget
    | some previous => if previous != armTarget then none
  some (← target, value)

private partial def enumAdminReturnedLocal? (ns : ValidatedNamespace)
    (expressionId : ExprId) (fuel : Nat) : Option LocalId := do
  if fuel == 0 then none else
  let expression ← ns.expressions[expressionId.index]?
  match expression.kind with
  | .letDecl _ none body => enumAdminReturnedLocal? ns body (fuel - 1)
  | .block statements (some result) =>
      if statements.isEmpty then enumAdminReturnedLocal? ns result (fuel - 1) else none
  | .return_ values => do
      let [value] := values.toList | none
      loadedLocal? ns value
  | _ => none

private partial def enumAdminApplySimpleAssignments? (ns : ValidatedNamespace)
    (expressionId : ExprId) (environment : EnumAdminEnv) (fuel : Nat) :
    Option EnumAdminEnv := do
  if fuel == 0 then none else
  let expression ← ns.expressions[expressionId.index]?
  match expression.kind with
  | .letDecl _ none body =>
      enumAdminApplySimpleAssignments? ns body environment (fuel - 1)
  | .block statements result => do
      let (statements, result) := flattenedBlock ns statements result fuel
      if result.isSome then none
      let mut environment := environment
      for statement in statements do
        let (localId, value) ← enumAdminAssignment? ns environment statement (fuel - 1)
        environment := enumAdminSet environment localId value
      some environment
  | .assign .. => do
      let (localId, value) ← enumAdminAssignment? ns environment expressionId (fuel - 1)
      some (enumAdminSet environment localId value)
  | _ => none

private partial def enumAdminBranchValue? (ns : ValidatedNamespace)
    (expressionId : ExprId) (environment : EnumAdminEnv) (discriminant : Int)
    (wanted : Option LocalId) (fuel : Nat) : Option EnumAdminValue := do
  if fuel == 0 then none else
  let expression ← ns.expressions[expressionId.index]?
  match expression.kind with
  | .letDecl _ none body =>
      enumAdminBranchValue? ns body environment discriminant wanted (fuel - 1)
  | .block statements result =>
      let (statements, result) := flattenedBlock ns statements result fuel
      let rec run (remaining : List ExprId) (environment : EnumAdminEnv) :
          Option EnumAdminValue := do
        match remaining with
        | statement :: remaining =>
            match enumAdminAssignment? ns environment statement (fuel - 1) with
            | some (localId, value) =>
                run remaining (enumAdminSet environment localId value)
            | none =>
                let statementExpression ← ns.expressions[statement.index]?
                let .ifElse condition thenBranch (some elseBranch) :=
                  statementExpression.kind | none
                let condition ← enumAdminValue? ns environment condition (fuel - 1)
                let pathValue (branch : ExprId) : Option EnumAdminValue :=
                  match enumAdminApplySimpleAssignments? ns branch environment (fuel - 1) with
                  | some environment => run remaining environment
                  | none => enumAdminBranchValue? ns branch environment discriminant wanted
                      (fuel - 1)
                some (.conditional condition (← pathValue thenBranch) (← pathValue elseBranch))
        | [] => match result with
          | some result =>
              enumAdminBranchValue? ns result environment discriminant wanted (fuel - 1)
          | none =>
              let wanted ← wanted
              some <| (enumAdminLookup environment wanted).getD (.place (.localVar wanted))
      run statements.toList environment
  | .return_ values => do
      let [value] := values.toList | none
      enumAdminValue? ns environment value (fuel - 1)
  | .match_ scrutinee arms => do
      let .discriminant _ _ ← enumAdminValue? ns environment scrutinee (fuel - 1) | none
      let arm ← arms.find? fun arm =>
        arm.guard.isNone && (ns.patterns[arm.pattern.index]?.any fun pattern =>
          pattern.kind == .literal (.integer discriminant))
      enumAdminBranchValue? ns arm.body environment discriminant wanted (fuel - 1)
  | .ifElse condition thenBranch (some elseBranch) => do
      let condition ← enumAdminValue? ns environment condition (fuel - 1)
      let thenValue ← enumAdminBranchValue? ns thenBranch environment discriminant wanted
        (fuel - 1)
      let elseValue ← enumAdminBranchValue? ns elseBranch environment discriminant wanted
        (fuel - 1)
      some (.conditional condition thenValue elseValue)
  | _ => none

private partial def enumAdminDiverges? (ns : ValidatedNamespace) (expressionId : ExprId)
    (fuel : Nat) : Bool :=
  if fuel == 0 then false else
  match ns.expressions[expressionId.index]? with
  | some { kind := .letDecl _ none body, .. } => enumAdminDiverges? ns body (fuel - 1)
  | some { kind := .block statements result, .. } =>
      match statements.toList, result with
      | [], some result => enumAdminDiverges? ns result (fuel - 1)
      | [statement], none => enumAdminDiverges? ns statement (fuel - 1)
      | _, _ => false
  | some { kind := .loop .., .. } => true
  | _ => false

private partial def enumAdminPlaceProjection (unit : ValidatedUnit)
    (place : EnumAdminPlace) : ProjectionM String := do
  match place with
  | .localVar localId => pure s!"local:{← localProjection localId}"
  | .field base field =>
      pure s!"field:{← enumAdminPlaceProjection unit base}:{← liftProjection <| nameProjection unit field}"
  | .downcast base variant =>
      pure s!"downcast:{← enumAdminPlaceProjection unit base}:{← liftProjection <| nameProjection unit variant}"

private partial def enumAdminValueProjection (unit : ValidatedUnit) (value : EnumAdminValue) :
    ProjectionM String := do
  match value with
  | .place place => pure s!"place:{← enumAdminPlaceProjection unit place}"
  | .integer value => pure s!"integer:{value}"
  | .primitive operation arguments =>
      pure s!"primitive:{repr operation}:{repr (← arguments.mapM
        (enumAdminValueProjection unit))}"
  | .conditional condition thenValue elseValue =>
      pure s!"if:{← enumAdminValueProjection unit condition}:\
        {← enumAdminValueProjection unit thenValue}:\
        {← enumAdminValueProjection unit elseValue}"
  | .borrow _ | .discriminant .. =>
      liftProjection <| fail "enum source administration returns a non-value"

private def enumSwitchProjection? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (expressionId : ExprId) (fuel : Nat) : ProjectionM (Option String) := do
  let some expression := ns.expressions[expressionId.index]? | return none
  let .block initialStatements initialResult := expression.kind | return none
  let (statements, result) := flattenedBlock ns initialStatements initialResult fuel
  let mut environment : EnumAdminEnv := #[]
  let mut outerMatch : Option ExprId := none
  let mut outerEnvironment : Option EnumAdminEnv := none
  let mut wanted : Option LocalId := none
  let resultMatch := result.bind fun result =>
    (ns.expressions[result.index]?).bind fun expression =>
      if expression.kind matches .match_ .. then some result else none
  match resultMatch with
  | some resultMatch =>
      for statement in statements do
        match enumAdminAssignment? ns environment statement fuel with
        | some (localId, value) => environment := enumAdminSet environment localId value
        | none => match enumAdminIdentityMatch? ns environment statement fuel with
          | some (localId, value) => environment := enumAdminSet environment localId value
          | none => return none
      outerMatch := some resultMatch
      outerEnvironment := some environment
  | none =>
      let some result := result | return none
      let some resultLocal := enumAdminReturnedLocal? ns result fuel | return none
      wanted := some resultLocal
      for statement in statements do
        match outerMatch with
        | none =>
            match enumAdminAssignment? ns environment statement fuel with
            | some (localId, value) => environment := enumAdminSet environment localId value
            | none => match enumAdminIdentityMatch? ns environment statement fuel with
              | some (localId, value) => environment := enumAdminSet environment localId value
              | none =>
                  let some statementExpression := ns.expressions[statement.index]? | return none
                  unless statementExpression.kind matches .match_ .. do return none
                  outerMatch := some statement
                  outerEnvironment := some environment
        | some _ =>
            let some (localId, value) :=
              enumAdminAssignment? ns environment statement fuel | return none
            match wanted, value with
            | some wantedLocal, .place (.localVar sourceLocal) =>
                if localId == wantedLocal then wanted := some sourceLocal
                else environment := enumAdminSet environment localId value
            | _, _ => environment := enumAdminSet environment localId value
  let some outerMatchId := outerMatch | return none
  let some capturedEnvironment := outerEnvironment | return none
  let some outerExpression := ns.expressions[outerMatchId.index]? | return none
  let .match_ scrutinee arms := outerExpression.kind | return none
  let some (.discriminant enumType base) :=
    enumAdminValue? ns capturedEnvironment scrutinee fuel | return none
  let mut projectedArms : Array String := #[]
  for arm in arms do
    if arm.guard.isSome then return none
    let some pattern := ns.patterns[arm.pattern.index]? | return none
    match pattern.kind with
    | .literal (.integer discriminant) =>
        let some result := enumAdminBranchValue? ns arm.body environment discriminant wanted fuel
          | return none
        projectedArms := projectedArms.push
          s!"{discriminant}:{← enumAdminValueProjection unit result}"
    | .wildcard => unless enumAdminDiverges? ns arm.body fuel do return none
    | _ => return none
  if projectedArms.isEmpty then return none
  let resultType ← liftProjection <|
    typeProjection unit expression.typeId unit.tables.types.size.succ
  let enumType ← liftProjection <| qualifiedProjection unit enumType
  let base ← enumAdminPlaceProjection unit base
  pure <| some s!"{resultType}:enumSwitch:{enumType}:{base}:{repr projectedArms}"

mutual
  private partial def placeProjection (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (placeId : PlaceId) (fuel : Nat) : ProjectionM String := do
    if fuel == 0 then liftProjection (fail "cyclic place in Rust semantic projection")
    let place ← liftProjection <| requireSome
      s!"semantic projection references missing place {placeId.index}"
      ns.places[placeId.index]?
    match place with
    | .localVar localId => pure s!"local:{← localProjection localId}"
    | .deref base => pure s!"deref:{← placeProjection unit ns base (fuel - 1)}"
    | .field base _ field =>
        pure s!"field:{← placeProjection unit ns base (fuel - 1)}:{← liftProjection <| nameProjection unit field}"
    | .index base index =>
        pure s!"index:{← placeProjection unit ns base (fuel - 1)}:{← expressionProjection unit ns index (fuel - 1)}"
    | .subslice base start stop fromEnd =>
        pure s!"subslice:{← placeProjection unit ns base (fuel - 1)}:{start}:{stop}:{fromEnd}"
    | .downcast base variant =>
        pure s!"downcast:{← placeProjection unit ns base (fuel - 1)}:{← liftProjection <| nameProjection unit variant}"

  private partial def patternProjection (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (patternId : PatternId) (fuel : Nat) : ProjectionM String := do
    if fuel == 0 then liftProjection (fail "cyclic pattern in Rust semantic projection")
    let pattern ← liftProjection <| requireSome
      s!"semantic projection references missing pattern {patternId.index}"
      ns.patterns[patternId.index]?
    let type ← liftProjection <| typeProjection unit pattern.typeId unit.tables.types.size.succ
    let kind ← match pattern.kind with
      | .wildcard => pure "wildcard"
      | .variable localId => pure s!"variable:{← localProjection localId}"
      | .tuple elements =>
          pure s!"tuple:{repr (← elements.mapM (patternProjection unit ns · (fuel - 1)))}"
      | .constructor name instantiations variant fields =>
          let instantiations ← liftProjection <| instantiations.mapM
            (genericArgumentProjection unit · unit.tables.types.size.succ)
          let fields ← fields.mapM (patternProjection unit ns · (fuel - 1))
          pure s!"constructor:{← liftProjection <| nameProjection unit name}:{repr instantiations}:{repr variant}:{repr fields}"
      | .literal value => pure s!"literal:{repr value}"
      | .range lower upper inclusive =>
          pure s!"range:{repr lower}:{repr upper}:{inclusive}"
    pure s!"{type}:{kind}"

  private partial def fromEndLoadProjection? (unit : ValidatedUnit)
      (ns : ValidatedNamespace) (expressionId : ExprId) : ProjectionM (Option String) := do
    let some admin := fromEndLoad? ns expressionId | return none
    let expression ← liftProjection <| requireSome
      "missing from-end load expression" ns.expressions[expressionId.index]?
    let type ← liftProjection <|
      typeProjection unit expression.typeId unit.tables.types.size.succ
    let target ← placeProjection unit ns admin.target ns.places.size.succ
    let root ← localProjection admin.root
    pure <| some s!"{type}:fromEndLoad:{target}:deref:local:{root}:{admin.offset}"

  private partial def expressionProjection (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (expressionId : ExprId) (fuel : Nat) : ProjectionM String := do
    if fuel == 0 then liftProjection (fail "cyclic expression in Rust semantic projection")
    let expression ← liftProjection <| requireSome
      s!"semantic projection references missing expression {expressionId.index}"
      ns.expressions[expressionId.index]?
    let type ← liftProjection <| typeProjection unit expression.typeId unit.tables.types.size.succ
    if let some localId := loadedLocal? ns expressionId then
      if let some substitution := (← get).valueSubstitutions.find? (·.1 == localId) then
        return ← expressionProjection unit ns substitution.2 (fuel - 1)
    if expression.kind matches .block .. then
      if (← get).normalizePureAdministration then
        if let .block statements result := expression.kind then
          if let some (scrutineeId, arms) := matchForwarding? ns statements result then
            let scrutinee ← expressionProjection unit ns scrutineeId (fuel - 1)
            let rec armsProjection : List (PatternId × ExprId) → ProjectionM String
              | [] => liftProjection <| fail "a forwarded match has no fallback arm"
              | (patternId, valueId) :: rest => do
                  let pattern ← liftProjection <| requireSome
                    "a forwarded match references a missing pattern"
                    ns.patterns[patternId.index]?
                  let value ← expressionProjection unit ns valueId (fuel - 1)
                  return ← match pattern.kind with
                  | .wildcard =>
                      unless rest.isEmpty do
                        liftProjection <| fail "a forwarded match has arms after its wildcard"
                      pure value
                  | .literal literal =>
                      let fallback ← armsProjection rest
                      let patternType ← liftProjection <|
                        typeProjection unit pattern.typeId unit.tables.types.size.succ
                      let literal := s!"{patternType}:value:{repr literal}"
                      let condition := s!"bool:operation:primitive:{repr PrimitiveOperation.equal}:#[]:{repr #[scrutinee, literal]}"
                      pure s!"{type}:if:{condition}:{value}:some {repr fallback}"
                  | _ => liftProjection <| fail "a forwarded match uses a non-literal pattern"
            return ← armsProjection arms.toList
          if let some (condition, thenValue, elseValue) :=
              conditionalForwarding? ns statements result then
            let condition ← expressionProjection unit ns condition (fuel - 1)
            let thenValue ← expressionProjection unit ns thenValue (fuel - 1)
            let elseValue ← expressionProjection unit ns elseValue (fuel - 1)
            return s!"{type}:if:{condition}:{thenValue}:some {repr elseValue}"
      match ← fromEndLoadProjection? unit ns expressionId with
      | some normalized => return normalized
      | none => pure ()
      if let some checked := checkedIndexLoad? unit (← get) ns expressionId then
        let previous ← get
        let substitutions := match checked.substitution with
          | some substitution => #[substitution] ++ previous.valueSubstitutions
          | none => previous.valueSubstitutions
        set { previous with valueSubstitutions := substitutions }
        let projected ← expressionProjection unit ns checked.success (fuel - 1)
        let current ← get
        set { previous with mappings := current.mappings, next := current.next }
        return projected
      match ← enumSwitchProjection? unit ns expressionId fuel with
      | some normalized => return normalized
      | none => pure ()
      match ← duplicatePanicGuardChainProjection? unit ns expressionId fuel with
      | some normalized => return normalized
      | none => pure ()
      if (← get).normalizePureAdministration then
        if let .block statements result := expression.kind then
          let previous ← get
          if let some (substitutions, value) :=
              pureAdministrativeResult? unit ns previous.mappings statements result then
            set { previous with
              valueSubstitutions := substitutions ++ previous.valueSubstitutions }
            let projected ← expressionProjection unit ns value (fuel - 1)
            let current ← get
            set { previous with mappings := current.mappings, next := current.next }
            return projected
    let children (expressions : Array ExprId) :=
      expressions.mapM (expressionProjection unit ns · (fuel - 1))
    let kind ← match expression.kind with
      | .value value _ => pure s!"value:{repr value}"
      | .constant constant =>
          pure s!"constant:{← liftProjection <| qualifiedProjection unit constant}"
      | .localVar localId =>
          let action := if typeIsCopy unit expression.typeId unit.tables.types.size.succ then
            "load" else "move"
          pure s!"operation:{action}:local:{← localProjection localId}:#[]:#[]"
      | .operation operation instantiations arguments _ => do
          let rec indexValue (indexId : ExprId) (indexFuel : Nat) : ProjectionM String := do
            if indexFuel == 0 then
              return ← expressionProjection unit ns indexId (fuel - 1)
            match ns.expressions[indexId.index]? with
            | some { kind := .value (.integer value) _, .. } => pure s!"literal:{value}"
            | _ =>
                let state ← get
                match loadedLocal? ns indexId with
                | some localId => match state.valueSubstitutions.find? (·.1 == localId) with
                    | some substitution => indexValue substitution.2 (indexFuel - 1)
                    | none => expressionProjection unit ns indexId (fuel - 1)
                | none => expressionProjection unit ns indexId (fuel - 1)
          let rec selectedPlace? (selectedId : ExprId) (selectedFuel : Nat) :
              ProjectionM (Option String) := do
            if selectedFuel == 0 then return none
            let some selected := ns.expressions[selectedId.index]? | return none
            match selected.kind with
            | .localVar localId => pure <| some s!"local:{← localProjection localId}"
            | .operation selectedOperation _ selectedArguments _ =>
                match selectedOperation, selectedArguments.toList with
                | .move place, [] =>
                    let some localId := directLocalPlace? ns place | return none
                    pure <| some s!"local:{← localProjection localId}"
                | .copy place, [] =>
                    let some localId := directLocalPlace? ns place | return none
                    pure <| some s!"local:{← localProjection localId}"
                | .read place, [] =>
                    let some localId := directLocalPlace? ns place | return none
                    pure <| some s!"local:{← localProjection localId}"
                | .data (.select reference field), [base] =>
                    let some (fieldEntry, fieldIndex) := unit.tables.names.zipIdx.find? fun
                        (entry, _) => entry.namespaceId == reference.namespaceId &&
                          entry.name == field
                      | return none
                    let qualified ← liftProjection <| qualifiedProjection unit {
                      namespaceId := fieldEntry.namespaceId, name := ⟨fieldIndex⟩ }
                    let some base ← selectedPlace? base (selectedFuel - 1) | return none
                    pure <| some s!"field:{base}:{qualified}"
                | _, _ => pure none
            | _ => pure none
          if (← get).normalizePureAdministration then
            match operation, arguments.toList with
            | .primitive .index, [collection, index] =>
                let collection ← expressionProjection unit ns collection (fuel - 1)
                let index ← indexValue index ns.expressions.size.succ
                return s!"operation:index:{repr #[collection, index]}"
            | .data (.select _ _), [_] =>
                if let some place ← selectedPlace? expressionId ns.expressions.size.succ then
                  let action := if typeIsCopy unit expression.typeId unit.tables.types.size.succ then
                    "load" else "move"
                  return s!"{type}:operation:{action}:{place}:#[]:#[]"
            | operation, [] =>
                let place := match operation with
                  | .move place | .copy place | .read place => some place
                  | _ => none
                if let some place := place then
                  if let some (localId, index) := indexedDirectLocalPlace? ns place then
                    let state ← get
                    let localTypeId ← liftProjection <| requireSome
                      "an indexed local has no declared type" state.localTypes[localId.index]?
                    let localType ← liftProjection <|
                      typeProjection unit localTypeId unit.tables.types.size.succ
                    let action := if typeIsCopy unit localTypeId unit.tables.types.size.succ then
                      "load" else "move"
                    let collection := s!"{localType}:operation:{action}:local:{← localProjection localId}:#[]:#[]"
                    let index ← indexValue index ns.expressions.size.succ
                    return s!"operation:index:{repr #[collection, index]}"
            | _, _ => pure ()
          let instantiations ← liftProjection <| instantiations.mapM
            (genericArgumentProjection unit · unit.tables.types.size.succ)
          let shiftArguments (value distance : ExprId) : ProjectionM (Array String) := do
            let value ← expressionProjection unit ns value (fuel - 1)
            let state ← get
            let distance ← match loadedLocal? ns distance with
              | some localId => match state.shiftCastSubstitutions.find? (·.1 == localId) with
                  | some substitution =>
                      expressionProjection unit ns substitution.2 (fuel - 1)
                  | none => expressionProjection unit ns distance (fuel - 1)
              | none => expressionProjection unit ns distance (fuel - 1)
            pure #[value, distance]
          let arguments ← match operation, arguments.toList with
            | .primitive .shiftLeft, [value, distance] => shiftArguments value distance
            | .primitive .shiftRight, [value, distance] => shiftArguments value distance
            | _, _ => children arguments
          let operation ← operationProjection unit ns expression.typeId operation fuel
          pure s!"operation:{operation}:{repr instantiations}:{repr arguments}"
      | .block statements result =>
          match forwardedShiftCast? unit ns statements result with
          | some (temporary, source, forwardedResult) =>
              let previous ← get
              set { previous with
                shiftCastSubstitutions := #[(temporary, source)] ++ previous.shiftCastSubstitutions }
              let projected ← expressionProjection unit ns forwardedResult (fuel - 1)
              let current ← get
              set { previous with mappings := current.mappings, next := current.next }
              pure projected
          | none => match forwardedReturnValue? ns statements result with
          | some value =>
              let value ← expressionProjection unit ns value (fuel - 1)
              pure s!"return:{repr #[value]}"
          | none => match forwardedAssignment? ns statements result with
              | some (target, value, assignmentType) =>
                  let assignmentType ← liftProjection <|
                    typeProjection unit assignmentType unit.tables.types.size.succ
                  let assignment := s!"{assignmentType}:assign:{← placeProjection unit ns target fuel}:{← expressionProjection unit ns value (fuel - 1)}"
                  pure s!"block:{repr #[assignment]}:none"
              | none => match duplicatePanicGuard? ns statements result with
                  | some guard =>
                      let outerPredicate ← expressionProjection unit ns guard.outerPredicate
                        (fuel - 1)
                      let innerPredicate ← expressionProjection unit ns guard.innerPredicate
                        (fuel - 1)
                      if outerPredicate != innerPredicate then
                        let (statements, result) := flattenedBlock ns statements result fuel
                        let statements ← children statements
                        let result ← result.mapM (expressionProjection unit ns · (fuel - 1))
                        pure s!"block:{repr statements}:{repr result}"
                      else
                        let statements ← children statements
                        let resultType ← liftProjection <| typeProjection unit guard.resultType
                          unit.tables.types.size.succ
                        let normalized := s!"{resultType}:if:{← expressionProjection unit ns guard.condition (fuel - 1)}:{← expressionProjection unit ns guard.panicBranch (fuel - 1)}:some {repr (← expressionProjection unit ns guard.finalBranch (fuel - 1))}"
                        pure s!"block:{repr statements}:some {repr normalized}"
                  | none =>
                      let (statements, result) := flattenedBlock ns statements result fuel
                      match trailingPanicGuardSplit? unit ns statements result with
                      | some (leadingStatements, guardStatements, condition, panic, continuation,
                          conditionalType) =>
                          let leadingStatements ← children leadingStatements
                          let virtualGuard : PanicGuardNode := {
                            expression := expressionId
                            statements := guardStatements
                            condition
                            panicBranch := panic
                            continuation
                            panicWhenTrue := true }
                          let nodes := #[virtualGuard] ++
                            panicGuardChain ns continuation (fuel - 1)
                          let guard ← match ←
                              duplicatePanicGuardNodesProjection? unit ns nodes fuel with
                            | some normalized => pure normalized
                            | none =>
                                let guardStatements ← children guardStatements
                                let condition ← expressionProjection unit ns condition (fuel - 1)
                                let panic ← expressionProjection unit ns panic (fuel - 1)
                                let continuation ← expressionProjection unit ns continuation
                                  (fuel - 1)
                                let conditionalType ← liftProjection <| typeProjection unit
                                  conditionalType unit.tables.types.size.succ
                                let conditional :=
                                  s!"{conditionalType}:if:{condition}:{panic}:some {repr continuation}"
                                pure s!"{type}:block:{repr guardStatements}:some {repr conditional}"
                          pure s!"block:{repr leadingStatements}:some {repr guard}"
                      | none => match forwardedReturnTail? ns statements result with
                        | some (statements, value, returnType) =>
                            let statements ← children statements
                            let value ← expressionProjection unit ns value (fuel - 1)
                            let returnType ← liftProjection <|
                              typeProjection unit returnType unit.tables.types.size.succ
                            pure s!"block:{repr statements}:some {repr s!"{returnType}:return:{repr #[value]}"}"
                        | none =>
                            let statements ← children statements
                            let result ← result.mapM (expressionProjection unit ns · (fuel - 1))
                            pure s!"block:{repr statements}:{repr result}"
      | .letDecl pattern value body =>
          match value with
          | none => expressionProjection unit ns body (fuel - 1)
          | some value =>
              let pattern ← patternProjection unit ns pattern (fuel - 1)
              pure s!"let:{pattern}:{← expressionProjection unit ns value (fuel - 1)}:{← expressionProjection unit ns body (fuel - 1)}"
      | .ifElse condition thenBranch elseBranch =>
          let elseBranch ← elseBranch.mapM (expressionProjection unit ns · (fuel - 1))
          pure s!"if:{← expressionProjection unit ns condition (fuel - 1)}:{← expressionProjection unit ns thenBranch (fuel - 1)}:{repr elseBranch}"
      | .match_ scrutinee arms =>
          let arms ← arms.mapM fun arm => do
            let guard ← arm.guard.mapM (expressionProjection unit ns · (fuel - 1))
            pure s!"{← patternProjection unit ns arm.pattern (fuel - 1)}:{repr guard}:{← expressionProjection unit ns arm.body (fuel - 1)}"
          pure s!"match:{← expressionProjection unit ns scrutinee (fuel - 1)}:{repr arms}"
      | .loop _ body =>
          pure s!"loop:{← expressionProjection unit ns body (fuel - 1)}"
      | .break_ nest value =>
          let value ← value.mapM (expressionProjection unit ns · (fuel - 1))
          pure s!"break:{nest}:{repr value}"
      | .continue_ nest => pure s!"continue:{nest}"
      | .return_ values => pure s!"return:{repr (← children values)}"
      | .throw_ kind arguments => pure s!"throw:{repr kind}:{repr (← children arguments)}"
      | .assign place value =>
          pure s!"assign:{← placeProjection unit ns place fuel}:{← expressionProjection unit ns value (fuel - 1)}"
      | .assignPattern pattern value =>
          pure s!"assignPattern:{← patternProjection unit ns pattern (fuel - 1)}:{← expressionProjection unit ns value (fuel - 1)}"
      | .quantifier .. | .spec _ =>
          liftProjection <| fail "specification expressions are outside Rust source alpha-equivalence"
    let erasesWrapper := expression.kind matches .letDecl _ none _ ||
      match expression.kind with
      | .block statements result => (forwardedShiftCast? unit ns statements result).isSome
      | _ => false
    if erasesWrapper then pure kind
    else
      let type := if (← get).normalizePureAdministration && expression.kind matches .throw_ ..
        then "never" else type
      pure s!"{type}:{kind}"

  private partial def operationProjection (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (resultType : TypeId) (operation : Operation) (fuel : Nat) : ProjectionM String := do
    match operation with
    | .move place | .copy place =>
        let action := if typeIsCopy unit resultType unit.tables.types.size.succ then "load" else
          if operation matches .move _ then "move" else "copy"
        pure s!"{action}:{← placeProjection unit ns place fuel}"
    | .borrow kind place => pure s!"borrow:{repr kind}:{← placeProjection unit ns place fuel}"
    | .read place => pure s!"read:{← placeProjection unit ns place fuel}"
    | .write place => pure s!"write:{← placeProjection unit ns place fuel}"
    | .drop place => pure s!"drop:{← placeProjection unit ns place fuel}"
    | .call kind => match kind with
        | .function reference =>
            pure s!"call:{← liftProjection <| qualifiedProjection unit reference}"
        | .constructor reference variant =>
            pure s!"constructor:{← liftProjection <| qualifiedProjection unit reference}:{repr variant}"
        | .destructor reference variant =>
            pure s!"destructor:{← liftProjection <| qualifiedProjection unit reference}:{repr variant}"
        | .closure reference =>
            pure s!"closure:{← liftProjection <| qualifiedProjection unit reference}"
        | .invoke => pure "invoke"
        | .extension value targets =>
            let targets ← liftProjection <| targets.mapM (qualifiedProjection unit)
            pure s!"callExtension:{repr value}:{repr targets}"
    | .global kind => pure s!"global:{repr kind}"
    | .primitive kind => pure s!"primitive:{repr kind}"
    | .reference kind => pure s!"reference:{repr kind}"
    | .data kind => match kind with
        | .select type field =>
            pure s!"select:{← liftProjection <| qualifiedProjection unit type}:{field}"
        | .selectVariants type fields =>
            pure s!"selectVariants:{← liftProjection <| qualifiedProjection unit type}:{repr fields}"
        | .testVariants type variants =>
            pure s!"testVariants:{← liftProjection <| qualifiedProjection unit type}:{repr variants}"
        | .discriminant type =>
            pure s!"discriminant:{← liftProjection <| qualifiedProjection unit type}"
        | .updateField type field =>
            pure s!"updateField:{← liftProjection <| qualifiedProjection unit type}:{field}"
    | .specification _ =>
        liftProjection <| fail "specification operations are outside Rust source alpha-equivalence"
    | .assert => pure "assert"
    | .profile value targets =>
        let targets ← liftProjection <| targets.mapM (qualifiedProjection unit)
        pure s!"profile:{repr value}:{repr targets}"

  private partial def duplicatePanicGuardChainProjection? (unit : ValidatedUnit)
      (ns : ValidatedNamespace) (expressionId : ExprId) (fuel : Nat) :
      ProjectionM (Option String) := do
    let nodes := panicGuardChain ns expressionId fuel
    duplicatePanicGuardNodesProjection? unit ns nodes fuel

  private partial def duplicatePanicGuardNodesProjection? (unit : ValidatedUnit)
      (ns : ValidatedNamespace) (nodes : Array PanicGuardNode) (fuel : Nat) :
      ProjectionM (Option String) := do
    if nodes.size < 2 then return none
    let base ← get
    let fingerprints ← nodes.mapM fun node => do
      let some values := panicGuardValues? ns node | return ""
      let computation : ProjectionM (Array String) :=
        values.mapM (expressionProjection unit ns · (fuel - 1))
      let (values, _) ← liftProjection <| computation.run base
      pure s!"{node.panicWhenTrue}:{repr values}"
    if fingerprints.any (·.isEmpty) then return none
    let some prefixLength := duplicatePrefixLength? fingerprints | return none
    let duplicateEnd ← liftProjection <| requireSome
      "invalid duplicate panic-guard prefix" nodes[2 * prefixLength - 1]?
    let rec projectPrefix (index : Nat) : ProjectionM String := do
      if index == prefixLength then
        expressionProjection unit ns duplicateEnd.continuation (fuel - 1)
      else
        let node ← liftProjection <| requireSome
          "invalid panic-guard prefix" nodes[index]?
        let block ← liftProjection <| requireSome
          "missing panic-guard block" ns.expressions[node.expression.index]?
        let .block _ (some resultId) := block.kind |
          liftProjection (fail "malformed panic-guard block")
        let result ← liftProjection <| requireSome
          "missing panic-guard conditional" ns.expressions[resultId.index]?
        let blockType ← liftProjection <|
          typeProjection unit block.typeId unit.tables.types.size.succ
        let resultType ← liftProjection <|
          typeProjection unit result.typeId unit.tables.types.size.succ
        let statements ← node.statements.mapM
          (expressionProjection unit ns · (fuel - 1))
        let condition ← expressionProjection unit ns node.condition (fuel - 1)
        let panic ← expressionProjection unit ns node.panicBranch (fuel - 1)
        let continuation ← projectPrefix (index + 1)
        let conditional := if node.panicWhenTrue then
          s!"{resultType}:if:{condition}:{panic}:some {repr continuation}"
        else
          s!"{resultType}:if:{condition}:{continuation}:some {repr panic}"
        pure s!"{blockType}:block:{repr statements}:some {repr conditional}"
    return some (← projectPrefix 0)
end

private def nominalProjection (unit : ValidatedUnit) (declaration : StructDecl) :
    Except String SemanticNominal := do
  if !declaration.properties.isEmpty || !declaration.locals.isEmpty ||
      !declaration.contract.conditions.isEmpty || declaration.contract.hasFrame ||
      declaration.contract.modifiesAll || declaration.contract.readsAll ||
      !declaration.contract.modifies.isEmpty || !declaration.contract.reads.isEmpty ||
      !declaration.contract.pragmas.isEmpty || !declaration.attributes.isEmpty then
    fail "nominal extensions and contracts are outside Rust source alpha-equivalence"
  let fieldProjection (field : FieldDecl) := do
    pure (← nameProjection unit field.name,
      ← typeProjection unit field.type.typeId unit.tables.types.size.succ)
  let variants ← declaration.variants.mapM fun variant => do
    pure (← nameProjection unit variant.name, variant.discriminant,
      ← variant.fields.mapM fieldProjection)
  pure {
    name := ← nameProjection unit declaration.name
    generics := ← declaration.generics.mapM (binderProjection unit)
    abilities := declaration.abilities
    fields := ← declaration.fields.mapM fieldProjection
    variants }

private partial def executableResultRoot? (ns : ValidatedNamespace) (expressionId : ExprId)
    (fuel : Nat) : Option ExprId :=
  match fuel with
  | 0 => none
  | fuel + 1 => do
      let expression ← ns.expressions[expressionId.index]?
      match expression.kind with
      | .letDecl _ none body => executableResultRoot? ns body fuel
      | .block statements result => match forwardedReturnValue? ns statements result with
          | some value => some value
          | none => match statements.isEmpty, result with
              | true, some result => executableResultRoot? ns result fuel
              | _, _ => none
      | .return_ values => match values.toList with
          | [value] => some value
          | _ => none
      | _ => some expressionId

private def functionProjection (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : FunctionDecl FunctionBody) (executableOnly : Bool := false) :
    Except String SemanticFunction := do
  if !executableOnly && (!declaration.contract.conditions.isEmpty || declaration.contract.hasFrame ||
      declaration.contract.modifiesAll || declaration.contract.readsAll ||
      !declaration.contract.modifies.isEmpty || !declaration.contract.reads.isEmpty ||
      !declaration.contract.pragmas.isEmpty || !declaration.pragmas.isEmpty ||
      !declaration.profileData.isEmpty || !declaration.attributes.isEmpty) then
    fail "function contracts are outside Rust source alpha-equivalence"
  let parameterCount := declaration.signature.parameters.size
  let initial : LocalProjectionState := {
    mappings := (Array.range parameterCount).map fun index => (⟨index⟩, index)
    localTypes := declaration.locals.map (·.type.typeId)
    normalizePureAdministration := executableOnly
    next := parameterCount }
  let (body, state) ← match declaration.body with
    | .absent => pure ("absent", initial)
    | .structured root =>
        let root := if executableOnly then
            executableResultRoot? ns root ns.expressions.size.succ |>.getD root
          else root
        (expressionProjection unit ns root ns.expressions.size.succ).run initial
  let locals ← state.mappings.drop parameterCount |>.mapM fun mapping => do
    let localDecl ← requireSome
      s!"semantic projection references missing local {mapping.1.index}"
      declaration.locals[mapping.1.index]?
    pure (localDecl.mutable,
      ← typeProjection unit localDecl.type.typeId unit.tables.types.size.succ)
  let parameters ← declaration.signature.parameters.mapM fun parameter => do
    pure (parameter.mutable,
      ← typeProjection unit parameter.typeUse.typeId unit.tables.types.size.succ)
  pure {
    name := ← nameProjection unit declaration.name
    profile := declaration.profile
    generics := ← declaration.signature.generics.mapM (binderProjection unit)
    predicates := ← declaration.signature.predicates.mapM fun predicate =>
      predicateProjection unit predicate unit.tables.types.size.succ
    parameters
    results := ← declaration.signature.results.mapM fun result =>
      typeProjection unit result.typeId unit.tables.types.size.succ
    locals
    body }

private def semanticProjectionWith (unit : ValidatedUnit) (executableOnly : Bool) :
    Except String SemanticUnit := do
  let namespaces ← unit.namespaces.mapM fun ns => do
    if !ns.constants.isEmpty || !ns.associatedItems.isEmpty || !ns.traits.isEmpty ||
        !ns.implementations.isEmpty || (!executableOnly && !ns.specFunctions.isEmpty) ||
        !ns.specVars.isEmpty ||
        !ns.invariants.isEmpty || !ns.intrinsics.isEmpty || !ns.attributes.isEmpty ||
        !ns.pragmas.isEmpty then
      fail "unsupported declaration family in Rust source alpha-equivalence"
    pure {
      profile := ns.profile
      imports := ns.imports
      metadata := ns.profileMetadata
      nominals := ← ns.structs.mapM (nominalProjection unit)
      functions := ← ns.functions.mapM (functionProjection unit ns · executableOnly) }
  pure {
    profiles := if executableOnly then unit.profiles.map fun profile =>
        { profile with version := 0, options := #[] }
      else unit.profiles
    dependencies := ← unit.dependencies.mapM fun dependency => do
      pure (dependency.namespaceId, dependency.profile,
        ← dependency.exportedNames.mapM (nameProjection unit))
    namespaces }

/-- Produce the alpha-normal semantic view used by the standard-Rust
round-trip gate. Provenance, source spelling, comments, and unreachable arena
entries are intentionally absent. -/
def semanticProjection (unit : ValidatedUnit) : Except String SemanticUnit :=
  semanticProjectionWith unit false

/-- Produce the executable alpha-normal view used after a source frontend has
derived specification mirrors, contracts, or source-policy attributes. Those
verification-only declarations are omitted; executable signatures and bodies
remain fully projected. -/
def executableSemanticProjection (unit : ValidatedUnit) : Except String SemanticUnit :=
  semanticProjectionWith unit true

/-- Decide semantic alpha-equivalence for the currently supported
standard-Rust source fragment. -/
def semanticallyAlphaEquivalent (left right : ValidatedUnit) : Except String Bool := do
  pure ((← semanticProjection left) == (← semanticProjection right))

end LeanerIR.Rust.Equivalence
