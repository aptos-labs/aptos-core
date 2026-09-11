-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR
import LeanerLang.Builtins
import LeanerLang.Diagnostic
import LeanerLang.Frame
import Lean.Util.SCC

/-!
# Leaner source lowering

This module is a frontend consumer of the public raw LIR API.  It does not
extend or mutate `LeanerIR`: source nodes are resolved, typed, interned, and
then emitted as an ordinary `LeanerIR.Import.RawUnit` for shared validation.
-/

namespace LeanerLang

open LeanerIR
open LeanerIR.Import

def ProfileName.toLIR : ProfileName → LeanerIR.Profile
  | .move => .move
  | .rust => .rust

private def BehaviorOperation.toLIR : BehaviorOperation → LeanerIR.BehaviorKind
  | .requiresOf => .requiresOf
  | .abortsOf => .abortsOf
  | .ensuresOf => .ensuresOf
  | .resultOf => .resultOf
  | .unchangedOf => .unchangedOf
  | .foldsOf => .foldsOf
  | .writeOf index => .writeOf index

/-- Checked defaults selected by the compact `using move` and `using rust`
headers. Canonical printing accepts only these configurations until profile
options receive an explicit source form. -/
def ProfileName.config : ProfileName → ProfileConfig
  | .move => { profile := .move, name := "move", version := 1 }
  | .rust => {
      profile := .rust
      name := "rust"
      version := 2
      options := #[
        ("panic", "abort"),
        ("unsafe", "reject"),
        ("target_pointer_width", "64")] }

private def QuantifierKind.toLIR : QuantifierKind → LeanerIR.QuantifierKind
  | .forall => .forall
  | .exists => .exists

private def Ability.toLIR : Ability → LeanerIR.Ability
  | .copy => .copy
  | .drop => .drop
  | .store => .store
  | .key => .key

private def BinderKind.toLIR : BinderKind → LeanerIR.BinderKind
  | .type => .typeArg
  | .const => .const
  | .lifetime => .lifetime
  | .evidence => .evidence

private def ThrowKind.toLIR : ThrowKind → LeanerIR.ThrowKind
  | .abort => .abort
  | .panic => .panic
  | .moveVectorError => .profile { profile := .move, tag := "runtime.vector_error" }

private def arrayIndex? [BEq α] (values : Array α) (needle : α) : Option Nat :=
  (Array.range values.size).find? fun index => values[index]? == some needle

private def namespaceId? (tables : Tables) (segments : Array String) : Option NamespaceId :=
  (arrayIndex? tables.namespaces { segments }).map (fun index => ⟨index⟩)

private def nameId? (tables : Tables) (namespaceId : NamespaceId)
    (name : String) : Option NameId :=
  (arrayIndex? tables.names { namespaceId, name }).map (fun index => ⟨index⟩)

private def internName (tables : Tables) (namespaceId : NamespaceId)
    (name : String) : Tables × NameId :=
  match nameId? tables namespaceId name with
  | some id => (tables, id)
  | none =>
      let id : NameId := ⟨tables.names.size⟩
      ({ tables with names := tables.names.push { namespaceId, name } }, id)

private def internNamespace (tables : Tables) (segments : Array String) : Tables × NamespaceId :=
  match namespaceId? tables segments with
  | some id => (tables, id)
  | none =>
      let id : NamespaceId := ⟨tables.namespaces.size⟩
      ({ tables with namespaces := tables.namespaces.push { segments } }, id)

/-- Expand either an imported symbol (`use a::b::f`; `f`) or an imported
namespace (`use a::b`; `b::f`). Exactly one matching final segment is required
so an ambiguous use never changes name resolution silently. -/
private def expandedUsePath? (sourceNs : Namespace)
    (segments : Array String) : Option (Array String) := do
  let first ← segments[0]?
  let candidates := sourceNs.uses.filter fun path => path.back? == some first
  let [path] := candidates.toList | none
  if segments.size == 1 then pure path else pure (path ++ segments.drop 1)

private partial def sourceTypeIsVector : Ty → Bool
  | .reference _ referent _ => sourceTypeIsVector referent.value
  | .vector .. => true
  | _ => false

/-- Resolve receiver syntax for a standard function through either a module
`use` (`use 0x1::std::vector`) or a function `use`
(`use 0x1::std::vector::push_back`). Ambiguous imports are rejected by
returning `none`; ordinary local call resolution then reports the useful
source error. -/
private def standardReceiverTarget? (sourceNs : Namespace) (functionName : String) :
    Option (Array String × StandardReceiverMode) := do
  let candidates := sourceNs.uses.filterMap fun path =>
    let owner := if path.back? == some functionName then path.pop else path
    let namespaceRef : NamespaceRef := { segments := owner }
    (standardCallReceiverMode? (some sourceNs.profile.toLIR) namespaceRef functionName).map
      fun mode => (owner.push functionName, mode)
  let first ← candidates[0]?
  guard <| candidates.all (· == first)
  pure first

private def predeclareItemNames (tables : Tables) (namespaceId : NamespaceId)
    (item : Item) : Tables :=
  let add (tables : Tables) (name : String) := (internName tables namespaceId name).1
  match item with
  | .constant declaration => add tables declaration.name
  | .function declaration => add tables declaration.name
  | .specFunction declaration => add tables declaration.name
  | .namespaceInvariants _ => tables
  | .struct declaration =>
      declaration.fields.foldl (fun tables field => add tables field.name)
        (add tables declaration.name)
  | .enum declaration =>
      declaration.variants.foldl (fun tables variant =>
        variant.fields.foldl (fun tables field => add tables field.name)
          (add tables variant.name)) (add tables declaration.name)

private def initialTables (unit : CompilationUnit) : Result Tables := do
  if unit.sourceName.isEmpty then
    throw #[.error "LEANER-SOURCE-NAME" "a Leaner compilation unit must name its source file"]
  if unit.namespaces.isEmpty then
    throw #[.error "LEANER-NAMESPACE-EMPTY" "a Leaner compilation unit must own a namespace"]
  let namespaceRefs := unit.namespaces.map fun sourceNs =>
    ({ segments := sourceNs.path } : NamespaceRef)
  if namespaceRefs.zipIdx.any fun (namespaceRef, index) =>
      namespaceRefs.take index |>.contains namespaceRef then
    throw #[.error "LEANER-NAMESPACE-DUPLICATE"
      "a compilation unit cannot declare the same namespace twice"]
  let base : Tables := {
    files := #[{ name := unit.sourceName }]
    namespaces := namespaceRefs }
  return unit.namespaces.zipIdx.foldl (fun tables (sourceNs, index) =>
    sourceNs.items.foldl (predeclareItemNames · ⟨index⟩ ·) tables) base

private structure TypeContext where
  binders : Array (String × Nat) := #[]
  lifetimeBinders : Array (String × Nat) := #[]
  /-- The namespace that wrote the types this context lowers. A declaration
  spells its own nominals the way its own source did — unqualified, or through
  its own `use` paths — so a signature borrowed from another namespace must
  resolve its names there, not in the namespace reading it. `none` means the
  namespace currently being lowered. -/
  owner : Option Namespace := none

private structure ScopedLocal where
  span : Span
  name : String
  id : LocalId
  typeId : TypeId

private structure ExprContext where
  types : TypeContext := {}
  generics : Array LeanerIR.GenericBinder := #[]
  locals : Array (String × LocalId × TypeId) := #[]
  declarations : Array ScopedLocal := #[]
  returnType : TypeId
  resultType : Option TypeId := none
  specification : Bool := false
  /-- Only a genuine function-tail return may become a fallthrough value.
  A nested block's tail can still exit enclosing statements or loops. -/
  functionTail : Bool := false
  loopResults : Array TypeId := #[]
  loopLabels : Array (Option String) := #[]
  /-- Executable pattern payload names are zero-cost aliases for field
  selections on the matched local. Keeping them symbolic avoids both
  administrative copies for by-value matches and nested runtime loans for
  reference matches. -/
  patternAliases : Array (String × Expr) := #[]

private structure BuildState where
  source : CompilationUnit
  sourceNamespace : Namespace
  namespaceId : NamespaceId
  tables : Tables
  output : RawNamespace
  /-- Base index and collected declarations of locals synthesized while
  lowering the current function's executable body.  Probing lowerings that
  discard their output restore these fields alongside it. -/
  temporaryLocalBase : Nat := 0
  temporaryLocals : Array LeanerIR.LocalDecl := #[]

private abbrev LowerM := StateT BuildState Result

private def failAt (code message : String) (span : Option Span := none) : LowerM α :=
  throw #[.error code message span]

/-- Canonical string-array profile payload, the format the LeanerLang printer
reads back (`Print.stringArrayPayload`). -/
private def packStringPayload (fields : Array String) : String :=
  (Lean.Json.arr (fields.map Lean.Json.str)).compress

private def addLoc (span : Span) : LowerM LocId := do
  if span.endByte < span.startByte then
    failAt "LEANER-SOURCE-RANGE" "a source range ends before it starts" (some span)
  let state ← get
  let id : LocId := ⟨state.tables.locations.size⟩
  let location : Location := {
    primary := some {
      file := ⟨0⟩
      startByte := span.startByte
      endByte := span.endByte } }
  set { state with tables := {
    state.tables with locations := state.tables.locations.push location } }
  return id

/-- Type identity is structural: the source location a type argument was
written at is provenance of that mention, not part of the interned type. -/
private def genericArgumentEqUpToLocation :
    LeanerIR.GenericArgument → LeanerIR.GenericArgument → Bool
  | .typeArg left, .typeArg right => left.typeId == right.typeId
  | left, right => left == right

private def typeEqUpToLocation : LeanerIR.Ty → LeanerIR.Ty → Bool
  | .nominal leftName leftArguments, .nominal rightName rightArguments =>
      leftName == rightName && leftArguments.size == rightArguments.size &&
        (leftArguments.zip rightArguments).all fun (left, right) =>
          genericArgumentEqUpToLocation left right
  | left, right => left == right

private def internType (type : LeanerIR.Ty) : LowerM TypeId := do
  let state ← get
  match state.tables.types.findIdx? (typeEqUpToLocation · type) with
  | some index => return ⟨index⟩
  | none =>
      let id : TypeId := ⟨state.tables.types.size⟩
      set { state with tables := {
        state.tables with types := state.tables.types.push type } }
      return id

private def pushExpression (loc : LocId) (typeId : TypeId)
    (kind : ExprKind) : LowerM ExprId := do
  let state ← get
  let id : ExprId := ⟨state.output.expressions.size⟩
  set { state with output := { state.output with
    expressions := state.output.expressions.push { loc, typeId, kind } } }
  return id

private def markReceiverSurface (id : ExprId) (span : Span) : LowerM Unit := do
  let state ← get
  let some expression := state.output.expressions[id.index]?
    | failAt "LEANER-RECEIVER-ID" "a receiver call produced no expression" (some span)
  let kind ← match expression.kind with
    | .operation operation instantiations arguments _ =>
        pure <| ExprKind.operation operation instantiations arguments (some .receiverCall)
    | _ => failAt "LEANER-RECEIVER-CALL" "receiver notation must lower to a function call" (some span)
  set { state with output := { state.output with
    expressions := state.output.expressions.set! id.index { expression with kind } } }

private def pushPattern (pattern : Pattern) : LowerM PatternId := do
  let state ← get
  let id : PatternId := ⟨state.output.patterns.size⟩
  set { state with output := { state.output with patterns := state.output.patterns.push pattern } }
  return id

/-- Declare a body-synthesized local of the function currently being lowered.
Synthesized locals index past every declared local; the function driver
appends them to the local table once its body lowering completes. -/
private def pushTemporaryLocal (typeId : TypeId) (loc : LocId) : LowerM LocalId := do
  let state ← get
  let id : LocalId := ⟨state.temporaryLocalBase + state.temporaryLocals.size⟩
  let declaration : LeanerIR.LocalDecl := {
    id, name := s!"$t{state.temporaryLocals.size}", type := { typeId, loc }
    mutable := false, loc }
  set { state with temporaryLocals := state.temporaryLocals.push declaration }
  return id

/-- Keep a computed mutation target at rest for the duration of the write.
The holder gives the reference a typed local identity, so mutation updates
that borrow instead of searching arbitrary global contents for a consumed
temporary. Existing local targets need no additional binding. -/
private def pushReferenceMutation (loc : LocId) (unitType referenceType : TypeId)
    (reference value : ExprId) : LowerM ExprId := do
  if let some { kind := .localVar _, .. } := (← get).output.expressions[reference.index]? then
    return ← pushExpression loc unitType <| .operation
      (.reference .mutate) #[] #[reference, value]
  let holder ← pushTemporaryLocal referenceType loc
  let pattern ← pushPattern { loc, typeId := referenceType, kind := .variable holder }
  let resting ← pushExpression loc referenceType (.localVar holder)
  let mutation ← pushExpression loc unitType <| .operation
    (.reference .mutate) #[] #[resting, value]
  pushExpression loc unitType (.letDecl pattern (some reference) mutation)

private def pushPlace (place : LeanerIR.Place) : LowerM PlaceId := do
  let state ← get
  let id : PlaceId := ⟨state.output.places.size⟩
  set { state with output := { state.output with places := state.output.places.push place } }
  return id

private def lookupTypeBinder? (context : TypeContext) (name : String) : Option Nat :=
  (context.binders.find? (·.1 == name)).map (·.2)

private def lowerLifetime (context : TypeContext) (source : SourceLifetime)
    (span : Span) : LowerM LifetimeId := do
  let loc ← addLoc span
  let (kind, name) ← match source with
    | .static => pure (LifetimeKind.static, some "'static")
    | .inference => pure (.inference, none)
    | .local name => pure (.local, some name)
    | .parameter name =>
        let some index := (context.lifetimeBinders.find? (·.1 == name)).map (·.2)
          | failAt "LEANER-LIFETIME-NAME" s!"unknown lifetime `{name}`" (some span)
        pure (.parameter index, some name)
  let state ← get
  let id : LifetimeId := ⟨state.tables.lifetimes.size⟩
  set { state with tables := { state.tables with
    lifetimes := state.tables.lifetimes.push { kind, loc, name } } }
  pure id

private partial def lowerTypeId (context : TypeContext) (source : Ty) : LowerM TypeId := do
  match source with
  | .unit => internType .unit
  | .never => internType .never
  | .bool => internType .bool
  | .char => internType .character
  | .string => internType .string
  | .bytes => internType .bytes
  | .address => internType .address
  | .signer => internType .signer
  | .uint width =>
      if width == 0 then failAt "LEANER-INTEGER-WIDTH" "`UInt<0>` is not a valid type"
      else internType (.integer (.bits width) false)
  | .sint width =>
      if width == 0 then failAt "LEANER-INTEGER-WIDTH" "`SInt<0>` is not a valid type"
      else internType (.integer (.bits width) true)
  | .uptr => internType (.integer .pointer false)
  | .iptr => internType (.integer .pointer true)
  | .nat => internType (.integer .unbounded false)
  | .int => internType (.integer .unbounded true)
  | .range => internType .range
  | .tuple elements =>
      let elements ← elements.mapM fun element => lowerTypeId context element.value
      internType (.tuple elements)
  | .vector element length =>
      if let some length := length then
        if length < 0 then
          failAt "LEANER-VECTOR-LENGTH" "a fixed vector length cannot be negative"
            (some element.span)
      let element ← lowerTypeId context element.value
      internType (.vector element (length.map .integer))
  | .function arguments result abilities =>
      let arguments ← arguments.mapM fun argument => lowerTypeId context argument.value
      let result ← lowerTypeId context result.value
      internType (.function arguments result (abilities.map Ability.toLIR))
  | .reference mutable referent lifetime =>
      let referentType ← lowerTypeId context referent.value
      let state ← get
      let lifetime ← lowerLifetime context lifetime referent.span
      let profile := state.sourceNamespace.profile.toLIR
      let kind := if mutable then ReferenceKind.mutable else .shared
      internType (.reference { profile, kind, referent := referentType, lifetime })
  | .named segments arguments =>
      if segments.size == 1 && arguments.isEmpty then
        if let some index := lookupTypeBinder? context segments[0]! then
          return ← internType (.typeParameter index)
      let state ← get
      let declaring := context.owner.getD state.sourceNamespace
      let segments := (expandedUsePath? declaring segments).getD segments
      let (ownerSegments, localName) :=
        if segments.size <= 1 then
          (declaring.path, segments[0]?.getD "")
        else
          (segments.extract 0 (segments.size - 1), segments.back!)
      let (tables, owner) := internNamespace state.tables ownerSegments
      set { state with tables }
      let current ← get
      let name ← match nameId? current.tables owner localName with
        | some name => pure name
        | none =>
            if owner == current.namespaceId then
              failAt "LEANER-TYPE-NAME" s!"unknown nominal type `{localName}`"
            else
              let (tables, name) := internName current.tables owner localName
              set { current with tables }
              pure name
      let arguments ← arguments.mapM fun argument => match argument with
        | .type argument => do
            let loc ← addLoc {}
            return LeanerIR.GenericArgument.typeArg {
              typeId := ← lowerTypeId context argument, loc }
        | .constInteger value => pure (.const (.integer value))
        | .constBool value => pure (.const (.bool value))
        | .lifetime value => do
            pure (.lifetime (← lowerLifetime context value {}))
      internType (.nominal name arguments)

private def lowerTypeUse (context : TypeContext) (source : Located Ty) : LowerM TypeUse := do
  let loc ← addLoc source.span
  let typeId ← lowerTypeId context source.value
  return { typeId, loc }

/-- Project a direct Move specification value. References expose their
referent and directly callable integer positions become mathematical `Int`.
Vectors, nominal data, and function values remain representation-bearing: a
function value carries its own signature, which the specification functions
that pass one around spell physically, so projecting it at a call boundary
would make the value disagree with the binding that holds it. A tuple is
projected, being a direct multi-value boundary rather than a value. -/
private partial def projectSpecTypeIdFuel (id : TypeId) (fuel : Nat) : LowerM TypeId := do
  if fuel == 0 then return id
  let state ← get
  match state.tables.types[id.index]? with
  | some (.integer _ _) => internType (.integer .unbounded true)
  | some (.reference reference) => projectSpecTypeIdFuel reference.referent (fuel - 1)
  | some (.tuple elements) =>
      internType (.tuple (← elements.mapM (projectSpecTypeIdFuel · (fuel - 1))))
  | _ => return id

private def projectSpecTypeId (id : TypeId) : LowerM TypeId := do
  projectSpecTypeIdFuel id ((← get).tables.types.size + 1)

private def specificationExprContext (context : ExprContext) : LowerM ExprContext := do
  let locals ← context.locals.mapM fun (name, id, typeId) => do
    pure (name, id, ← projectSpecTypeId typeId)
  let declarations ← context.declarations.mapM fun declaration => do
    pure { declaration with typeId := ← projectSpecTypeId declaration.typeId }
  pure {
    context with
    locals
    declarations
    returnType := ← projectSpecTypeId context.returnType
    resultType := ← context.resultType.mapM projectSpecTypeId
    specification := true
    loopResults := ← context.loopResults.mapM projectSpecTypeId }

private def lowerBinder (binder : GenericBinder) : LowerM LeanerIR.GenericBinder := do
  let loc ← addLoc binder.span
  let type ← if binder.kind == .const then
      some <$> lowerTypeUse {} (binder.type.getD { value := .uptr, span := binder.span })
    else pure none
  -- The phantom marker is a profile predicate: Move's `phantom` parameter.
  let predicates : Array LeanerIR.GenericPredicate := if binder.phantom then
      #[.profile { profile := .move, tag := "typeParameter.phantom" }] else #[]
  return {
    name := binder.name
    kind := binder.kind.toLIR
    type
    abilities := binder.abilities.map Ability.toLIR
    predicates
    loc }

private def typeContext (binders : Array GenericBinder)
    (owner : Option Namespace := none) : Result TypeContext := do
  let duplicate := binders.zipIdx.find? fun (binder, index) =>
    binders.take index |>.any (·.name == binder.name)
  if let some (binder, _) := duplicate then
    throw #[.error "LEANER-GENERIC-DUPLICATE"
      s!"generic binder `{binder.name}` is declared more than once" (some binder.span)]
  return {
    owner
    binders := binders.zipIdx.filterMap fun (binder, index) =>
      if binder.kind == .type then some (binder.name, index) else none
    lifetimeBinders := binders.zipIdx.filterMap fun (binder, index) =>
      if binder.kind == .lifetime then some (binder.name, index) else none }

private def lookupLocal? (context : ExprContext)
    (name : String) : Option (LocalId × TypeId) :=
  (context.locals.find? (·.1 == name)).map fun entry => (entry.2.1, entry.2.2)

private def withBoundLocals (context : ExprContext)
    (locals : Array (String × LocalId × TypeId)) : ExprContext :=
  { context with
    locals := locals ++ context.locals
    patternAliases := context.patternAliases.filter fun alias =>
      !locals.any (·.1 == alias.1) }

private def declaredLocal? (context : ExprContext) (span : Span) (name : String) :
    Option ScopedLocal :=
  context.declarations.find? fun declaration =>
    declaration.span == span && declaration.name == name

private structure SourceBinding where
  span : Span
  mutable : Bool
  pattern : BindingPattern
  type : Option (Located Ty)
  value : Expr

private def forLowerName (span : Span) : String :=
  s!"$for_lb_{span.startByte}"

private def forUpperName (span : Span) : String :=
  s!"$for_ub_{span.startByte}"

mutual
  private partial def sourceBindings (source : Expr) : Array SourceBinding :=
    match source with
    | .primitive _ arguments _ | .typedPrimitive _ _ arguments _ |
        .call _ arguments _ | .construct _ arguments _ |
        .appliedConstruct _ _ arguments _ |
        .throw_ _ arguments _ => arguments.flatMap sourceBindings
    | .namedConstruct _ _ fields _ => fields.flatMap (sourceBindings ·.2)
    | .closure _ _ captures _ => captures.flatMap sourceBindings
    | .invoke callable arguments _ =>
        sourceBindings callable ++ arguments.flatMap sourceBindings
    | .genericCall _ _ arguments _ | .typedCall _ _ arguments _ |
        .typedGenericCall _ _ _ arguments _ =>
        arguments.flatMap sourceBindings
    | .methodCall _ _ _ receiver arguments _ =>
        sourceBindings receiver ++ arguments.flatMap sourceBindings
    | .global _ _ arguments _ => arguments.flatMap sourceBindings
    | .select _ _ value _ | .field value _ _ | .placeOperation _ value _ |
        .variantTest value _ _ |
        .selectVariants _ _ value _ |
        .testVariants _ _ value _ | .discriminant _ _ value _ | .borrowValue _ value _ |
        .rawBorrowValue _ value _ |
        .freezeReference _ value _ | .dereference value _ | .return_ value _ =>
        sourceBindings value
    | .storageIndex _ index _ => sourceBindings index
    | .index value index _ | .membership value index _ =>
        sourceBindings value ++ sourceBindings index
    | .mutateReference reference value _ => sourceBindings reference ++ sourceBindings value
    | .quantifier _ binders body _ =>
        binders.flatMap (sourceBindings ·.2) ++ sourceBindings body
    | .specification _ _ arguments _ => arguments.flatMap sourceBindings
    | .specBlock conditions _ => conditions.flatMap fun (kind, value) =>
        let nested := sourceBindings value
        match kind with
        | .let_ name => nested.push {
            span := value.span
            mutable := false
            pattern := .variable name value.span
            type := none
            value }
        | _ => nested
    | .forRange iterator lower upper body span =>
        let lowerName := forLowerName span
        let upperName := forUpperName span
        sourceBindings lower ++
          #[{ span, mutable := false, pattern := .variable lowerName span,
              type := none, value := lower },
            { span, mutable := false, pattern := .variable iterator span,
              type := none, value := .local lowerName span }] ++
          sourceBindings upper ++
          #[{ span, mutable := false, pattern := .variable upperName span,
              type := none, value := upper }] ++
          sourceBindings body
    | .loop body _ _ => sourceBindings body
    | .break_ value _ _ => value.toArray.flatMap sourceBindings
    | .assign _ value _ | .assignPattern _ _ value _ => sourceBindings value
    | .assignExpression target value _ | .rawAssignExpression target value _ =>
        sourceBindings target ++ sourceBindings value
    | .block statements result _ =>
        statements.flatMap sourceStatementBindings ++ result.toArray.flatMap sourceBindings
    | .ifElse condition thenBranch elseBranch _ =>
        sourceBindings condition ++ sourceBindings thenBranch ++
          elseBranch.toArray.flatMap sourceBindings
    | .match_ scrutinee arms _ =>
        sourceBindings scrutinee ++ arms.flatMap fun (_, guard, body) =>
          guard.toArray.flatMap sourceBindings ++ sourceBindings body
    | .unit .. | .bool .. | .char .. | .integer .. | .typedInteger .. |
        .address .. | .string .. | .bytes .. |
        .local .. | .borrowPlace .. | .dropPlace .. | .continue_ .. => #[]

  /-- Immediate subexpressions, for checks that need no per-constructor
  meaning of their own. -/
  private partial def childExpressions : Expr → Array Expr
    | .primitive _ arguments _ | .typedPrimitive _ _ arguments _ |
        .call _ arguments _ | .construct _ arguments _ |
        .appliedConstruct _ _ arguments _ | .throw_ _ arguments _ |
        .genericCall _ _ arguments _ | .typedCall _ _ arguments _ |
        .typedGenericCall _ _ _ arguments _ | .global _ _ arguments _ |
        .specification _ _ arguments _ => arguments
    | .namedConstruct _ _ fields _ => fields.map (·.2)
    | .closure _ _ captures _ => captures
    | .invoke callable arguments _ => #[callable] ++ arguments
    | .methodCall _ _ _ receiver arguments _ => #[receiver] ++ arguments
    | .select _ _ value _ | .field value _ _ | .placeOperation _ value _ |
        .variantTest value _ _ | .selectVariants _ _ value _ |
        .testVariants _ _ value _ | .discriminant _ _ value _ |
        .borrowValue _ value _ | .rawBorrowValue _ value _ | .freezeReference _ value _ |
        .dereference value _ | .return_ value _ | .storageIndex _ value _ |
        .assign _ value _ | .assignPattern _ _ value _ | .loop value _ _ => #[value]
    | .index value index _ | .membership value index _ |
        .mutateReference value index _ | .assignExpression value index _ |
        .rawAssignExpression value index _ => #[value, index]
    | .quantifier _ binders body _ => binders.map (·.2) ++ #[body]
    | .specBlock conditions _ => conditions.map (·.2)
    | .forRange _ lower upper body _ => #[lower, upper, body]
    | .break_ value _ _ => value.toArray
    | .block statements result _ =>
        statements.map (fun statement => match statement with
          | .expression value => value
          | .letDecl _ _ _ value _ => value) ++ result.toArray
    | .ifElse condition thenBranch elseBranch _ =>
        #[condition, thenBranch] ++ elseBranch.toArray
    | .match_ scrutinee arms _ =>
        #[scrutinee] ++ arms.flatMap fun (_, guard, body) => guard.toArray ++ #[body]
    | .unit .. | .bool .. | .char .. | .integer .. | .typedInteger .. |
        .address .. | .string .. | .bytes .. | .local .. | .borrowPlace .. |
        .dropPlace .. | .continue_ .. => #[]

  /-- Bindings of this expression, each paired with the bindings lexically in
  scope where it occurs. A nested block's declarations are visible to what
  follows them inside that block, and to nothing outside it. -/
  private partial def scopedBindings (scope : Array SourceBinding) (source : Expr) :
      Array (SourceBinding × Array SourceBinding) :=
    match source with
    | .block statements result _ =>
        let (collected, scope) := statements.foldl (init := (#[], scope))
          fun (collected, scope) statement =>
            match statement with
            | .expression value => (collected ++ scopedBindings scope value, scope)
            | .letDecl mutable pattern type value span =>
                let binding : SourceBinding := { span, mutable, pattern, type, value }
                (collected ++ scopedBindings scope value ++ #[(binding, scope)],
                  scope.push binding)
        collected ++ (result.map (scopedBindings scope)).getD #[]
    | .specBlock conditions _ =>
        (conditions.foldl (init := (#[], scope)) fun (collected, scope) (kind, value) =>
          let nested := scopedBindings scope value
          match kind with
          | .let_ name =>
              let binding : SourceBinding := {
                span := value.span, mutable := false
                pattern := .variable name value.span, type := none, value }
              (collected ++ nested ++ #[(binding, scope)], scope.push binding)
          | _ => (collected ++ nested, scope)).1
    | .forRange iterator lower upper body span =>
        let lowerName := forLowerName span
        let upperName := forUpperName span
        let lowerBinding : SourceBinding := {
          span, mutable := false, pattern := .variable lowerName span
          type := none, value := lower }
        let iteratorBinding : SourceBinding := {
          span, mutable := false, pattern := .variable iterator span
          type := none, value := .local lowerName span }
        let upperBinding : SourceBinding := {
          span, mutable := false, pattern := .variable upperName span
          type := none, value := upper }
        let inner := scope.push lowerBinding |>.push iteratorBinding |>.push upperBinding
        scopedBindings scope lower ++
          #[(lowerBinding, scope), (iteratorBinding, scope.push lowerBinding)] ++
          scopedBindings scope upper ++ #[(upperBinding, scope)] ++
          scopedBindings inner body
    | .match_ scrutinee arms _ =>
        scopedBindings scope scrutinee ++ arms.flatMap fun (pattern, guard, body) =>
          let binding : SourceBinding := {
            span := pattern.span
            mutable := false
            pattern
            type := none
            value := scrutinee }
          let armScope := scope.push binding
          #[(binding, scope)] ++
            guard.toArray.flatMap (scopedBindings armScope) ++
            scopedBindings armScope body
    | source => (childExpressions source).flatMap (scopedBindings scope)

  /-- Does control leave this expression through a `return` written somewhere
  other than its own tail? A tail `return` is the expression's value, but an
  early one is a jump the specification projection cannot express. -/
  private partial def hasEarlyReturn (tail : Bool) (source : Expr) : Bool :=
    match source with
    | .return_ value _ => !tail || hasEarlyReturn false value
    | .block statements result _ =>
        statements.any (fun statement => match statement with
          | .expression value => hasEarlyReturn false value
          | .letDecl _ _ _ value _ => hasEarlyReturn false value) ||
          (result.map (hasEarlyReturn tail)).getD false
    | .ifElse condition thenBranch elseBranch _ =>
        hasEarlyReturn false condition || hasEarlyReturn tail thenBranch ||
          (elseBranch.map (hasEarlyReturn tail)).getD false
    | .match_ scrutinee arms _ =>
        hasEarlyReturn false scrutinee ||
          arms.any fun (_, guard, body) =>
            (guard.map (hasEarlyReturn false)).getD false || hasEarlyReturn tail body
    | source => (childExpressions source).any (hasEarlyReturn false)

  /-- Does this expression iterate? A loop's meaning is a fixed point, which
  the specification projection has no expression shape for, the same way it
  has none for a jump out of the body. -/
  private partial def hasLoop (source : Expr) : Bool :=
    match source with
    | .loop .. | .forRange .. => true
    | source => (childExpressions source).any hasLoop

  private partial def sourceStatementBindings : Statement → Array SourceBinding
    | .expression value => sourceBindings value
    | .letDecl mutable pattern type value span =>
        -- Nested declarations in an initializer must be predeclared before
        -- probing the enclosing initializer's inferred type.
        sourceBindings value ++ #[{ span, mutable, pattern, type, value }]
end

private structure SourceQuantifierBinding where
  pattern : BindingPattern
  domain : Expr

mutual
  private partial def sourceQuantifierBindings (source : Expr) :
      Array SourceQuantifierBinding :=
    match source with
    | .quantifier _ binders body _ =>
        binders.map (fun (pattern, domain) => { pattern, domain }) ++
          binders.flatMap (sourceQuantifierBindings ·.2) ++ sourceQuantifierBindings body
    | .specification _ _ arguments _ => arguments.flatMap sourceQuantifierBindings
    | .specBlock conditions _ => conditions.flatMap (sourceQuantifierBindings ·.2)
    | .forRange _ lower upper body _ =>
        sourceQuantifierBindings lower ++ sourceQuantifierBindings upper ++
          sourceQuantifierBindings body
    | .loop body _ _ => sourceQuantifierBindings body
    | .break_ value _ _ => value.toArray.flatMap sourceQuantifierBindings
    | .assign _ value _ | .assignPattern _ _ value _ => sourceQuantifierBindings value
    | .assignExpression target value _ | .rawAssignExpression target value _ =>
        sourceQuantifierBindings target ++ sourceQuantifierBindings value
    | .primitive _ arguments _ | .typedPrimitive _ _ arguments _ |
        .call _ arguments _ | .construct _ arguments _ |
        .appliedConstruct _ _ arguments _ |
        .throw_ _ arguments _ => arguments.flatMap sourceQuantifierBindings
    | .namedConstruct _ _ fields _ => fields.flatMap (sourceQuantifierBindings ·.2)
    | .closure _ _ captures _ => captures.flatMap sourceQuantifierBindings
    | .invoke callable arguments _ =>
        sourceQuantifierBindings callable ++ arguments.flatMap sourceQuantifierBindings
    | .genericCall _ _ arguments _ | .typedCall _ _ arguments _ |
        .typedGenericCall _ _ _ arguments _ =>
        arguments.flatMap sourceQuantifierBindings
    | .methodCall _ _ _ receiver arguments _ =>
        sourceQuantifierBindings receiver ++ arguments.flatMap sourceQuantifierBindings
    | .global _ _ arguments _ => arguments.flatMap sourceQuantifierBindings
    | .select _ _ value _ | .field value _ _ | .placeOperation _ value _ |
        .variantTest value _ _ |
        .selectVariants _ _ value _ |
        .testVariants _ _ value _ | .discriminant _ _ value _ | .borrowValue _ value _ |
        .rawBorrowValue _ value _ |
        .freezeReference _ value _ | .dereference value _ | .return_ value _ =>
        sourceQuantifierBindings value
    | .storageIndex _ index _ => sourceQuantifierBindings index
    | .index value index _ | .membership value index _ =>
        sourceQuantifierBindings value ++ sourceQuantifierBindings index
    | .mutateReference reference value _ =>
        sourceQuantifierBindings reference ++ sourceQuantifierBindings value
    | .block statements result _ =>
        statements.flatMap sourceStatementQuantifierBindings ++
          result.toArray.flatMap sourceQuantifierBindings
    | .ifElse condition thenBranch elseBranch _ =>
        sourceQuantifierBindings condition ++ sourceQuantifierBindings thenBranch ++
          elseBranch.toArray.flatMap sourceQuantifierBindings
    | .match_ scrutinee arms _ =>
        sourceQuantifierBindings scrutinee ++ arms.flatMap fun (_, guard, body) =>
          guard.toArray.flatMap sourceQuantifierBindings ++ sourceQuantifierBindings body
    | .unit .. | .bool .. | .char .. | .integer .. | .typedInteger .. |
        .address .. | .string .. | .bytes .. |
        .local .. | .borrowPlace .. | .dropPlace .. | .continue_ .. => #[]

  private partial def sourceStatementQuantifierBindings : Statement →
      Array SourceQuantifierBinding
    | .expression value => sourceQuantifierBindings value
    | .letDecl _ _ _ value _ => sourceQuantifierBindings value
end

private structure SourceMatchBinding where
  pattern : BindingPattern
  scrutinee : Expr

mutual
  private partial def sourceMatchBindings (source : Expr) : Array SourceMatchBinding :=
    match source with
    | .match_ scrutinee arms _ =>
        arms.map (fun (pattern, _, _) => { pattern, scrutinee }) ++
          sourceMatchBindings scrutinee ++ arms.flatMap fun (_, guard, body) =>
            guard.toArray.flatMap sourceMatchBindings ++ sourceMatchBindings body
    | .quantifier _ binders body _ =>
        binders.flatMap (sourceMatchBindings ·.2) ++ sourceMatchBindings body
    | .specification _ _ arguments _ => arguments.flatMap sourceMatchBindings
    | .specBlock conditions _ => conditions.flatMap (sourceMatchBindings ·.2)
    | .forRange _ lower upper body _ =>
        sourceMatchBindings lower ++ sourceMatchBindings upper ++ sourceMatchBindings body
    | .loop body _ _ => sourceMatchBindings body
    | .break_ value _ _ => value.toArray.flatMap sourceMatchBindings
    | .assign _ value _ | .assignPattern _ _ value _ => sourceMatchBindings value
    | .assignExpression target value _ | .rawAssignExpression target value _ =>
        sourceMatchBindings target ++ sourceMatchBindings value
    | .primitive _ arguments _ | .typedPrimitive _ _ arguments _ |
        .call _ arguments _ | .construct _ arguments _ |
        .appliedConstruct _ _ arguments _ |
        .throw_ _ arguments _ => arguments.flatMap sourceMatchBindings
    | .namedConstruct _ _ fields _ => fields.flatMap (sourceMatchBindings ·.2)
    | .closure _ _ captures _ => captures.flatMap sourceMatchBindings
    | .invoke callable arguments _ =>
        sourceMatchBindings callable ++ arguments.flatMap sourceMatchBindings
    | .genericCall _ _ arguments _ | .typedCall _ _ arguments _ |
        .typedGenericCall _ _ _ arguments _ =>
        arguments.flatMap sourceMatchBindings
    | .methodCall _ _ _ receiver arguments _ =>
        sourceMatchBindings receiver ++ arguments.flatMap sourceMatchBindings
    | .global _ _ arguments _ => arguments.flatMap sourceMatchBindings
    | .select _ _ value _ | .field value _ _ | .placeOperation _ value _ |
        .variantTest value _ _ |
        .selectVariants _ _ value _ |
        .testVariants _ _ value _ | .discriminant _ _ value _ | .borrowValue _ value _ |
        .rawBorrowValue _ value _ |
        .freezeReference _ value _ | .dereference value _ | .return_ value _ =>
        sourceMatchBindings value
    | .storageIndex _ index _ => sourceMatchBindings index
    | .index value index _ | .membership value index _ =>
        sourceMatchBindings value ++ sourceMatchBindings index
    | .mutateReference reference value _ =>
        sourceMatchBindings reference ++ sourceMatchBindings value
    | .block statements result _ =>
        statements.flatMap sourceStatementMatchBindings ++
          result.toArray.flatMap sourceMatchBindings
    | .ifElse condition thenBranch elseBranch _ =>
        sourceMatchBindings condition ++ sourceMatchBindings thenBranch ++
          elseBranch.toArray.flatMap sourceMatchBindings
    | .unit .. | .bool .. | .char .. | .integer .. | .typedInteger .. |
        .address .. | .string .. | .bytes .. |
        .local .. | .borrowPlace .. | .dropPlace .. | .continue_ .. => #[]

  private partial def sourceStatementMatchBindings : Statement → Array SourceMatchBinding
    | .expression value => sourceMatchBindings value
    | .letDecl _ _ _ value _ => sourceMatchBindings value
end

private def typeNode? (id : TypeId) : LowerM (Option LeanerIR.Ty) := do
  return (← get).tables.types[id.index]?

private def inferredReferenceType (kind : ReferenceKind) (referent : TypeId)
    (loc : LocId) : LowerM TypeId := do
  let state ← get
  let lifetime : LifetimeId := ⟨state.tables.lifetimes.size⟩
  let profile := state.sourceNamespace.profile.toLIR
  set { state with tables := { state.tables with
    lifetimes := state.tables.lifetimes.push { kind := .inference, loc } } }
  internType (.reference { profile, kind, referent, lifetime })

private def sourceMatchPatternType (context : ExprContext)
    (pattern : BindingPattern) (scrutinee : Expr) : LowerM TypeId := do
  let rec infer : Expr → LowerM (Option TypeId)
    | .local name span => do
        let type? := (lookupLocal? context name).map (·.2) |>.orElse fun _ =>
          (context.declarations.reverse.find? (·.name == name)).map (·.typeId)
        let some typeId := type?
          | failAt "LEANER-MATCH-SCRUTINEE"
              s!"unknown match scrutinee local `{name}`" (some span)
        pure (some typeId)
    | .discriminant _ result _ _ => do
        pure (some (← lowerTypeUse context.types result).typeId)
    | .specification .old _ #[value] _ => infer value
    | .dereference value _ => infer value
    | _ => pure none
  let typeId? ← infer scrutinee
  let typeId ← match typeId? with
    | some typeId => pure typeId
    | none => match pattern with
        | .constructor owner _ _ _ => pure (← lowerTypeUse context.types owner).typeId
        | _ => do
            failAt "LEANER-MATCH-SCRUTINEE"
              ("match-pattern predeclaration currently requires a local scrutinee " ++
                "or a constructor pattern") (some scrutinee.span)
  if context.specification then projectSpecTypeId typeId else pure typeId

private def requireExpected (expected : Option TypeId) (span : Span)
    (description : String) : LowerM TypeId :=
  match expected with
  | some type => pure type
  | none => failAt "LEANER-TYPE-INFERENCE"
      s!"cannot infer the type of {description}; add a result type or typed context" (some span)

private partial def compatibleType (expected actual : TypeId) (fuel : Nat) : LowerM Bool := do
  if expected == actual then return true
  if fuel == 0 then return false
  let state ← get
  match state.tables.types[expected.index]?, state.tables.types[actual.index]? with
  | some (.reference expectedRef), some (.reference actualRef) =>
      let expectedLifetime := state.tables.lifetimes[expectedRef.lifetime.index]?
      let actualLifetime := state.tables.lifetimes[actualRef.lifetime.index]?
      let compatibleLifetime := expectedRef.lifetime == actualRef.lifetime ||
        (expectedLifetime.any (fun value => value.kind == .inference) &&
          actualLifetime.any (fun value => value.kind == .inference))
      if expectedRef.profile != actualRef.profile || expectedRef.kind != actualRef.kind ||
          !compatibleLifetime then
        return false
      compatibleType expectedRef.referent actualRef.referent (fuel - 1)
  | some (.tuple expectedElements), some (.tuple actualElements) =>
      if expectedElements.size != actualElements.size then return false
      (expectedElements.zip actualElements).allM fun (expected, actual) =>
        compatibleType expected actual (fuel - 1)
  | some (.vector expectedElement expectedLength), some (.vector actualElement actualLength) =>
      if expectedLength != actualLength then return false
      compatibleType expectedElement actualElement (fuel - 1)
  | some (.function expectedArguments expectedResult expectedAbilities),
      some (.function actualArguments actualResult actualAbilities) =>
      if expectedAbilities != actualAbilities ||
          expectedArguments.size != actualArguments.size then
        return false
      if !(← (expectedArguments.zip actualArguments).allM fun (expected, actual) =>
          compatibleType expected actual (fuel - 1)) then
        return false
      compatibleType expectedResult actualResult (fuel - 1)
  | some (.nominal expectedName expectedArguments),
      some (.nominal actualName actualArguments) =>
      if expectedName != actualName || expectedArguments.size != actualArguments.size then
        return false
      (expectedArguments.zip actualArguments).allM fun (expected, actual) =>
        match expected, actual with
        | .typeArg expected, .typeArg actual =>
            compatibleType expected.typeId actual.typeId (fuel - 1)
        | .const expected, .const actual => pure (expected == actual)
        | .lifetime expected, .lifetime actual => pure (expected == actual)
        | .evidence expected, .evidence actual => pure (expected == actual)
        | _, _ => pure false
  | _, _ => return false

/-- The qualified source name behind a nominal type identity. A raw `NameId`
index says nothing about which declaration a mismatch names, and two same-named
nominals from different namespaces are exactly the interesting case. -/
private def nominalDescription? (typeId : TypeId) : LowerM (Option String) := do
  let some (.nominal name _) ← typeNode? typeId | return none
  let tables := (← get).tables
  let some qualified := tables.names[name.index]? | return none
  let some owner := tables.namespaces[qualified.namespaceId.index]? | return none
  return some <| "::".intercalate (owner.segments.push qualified.name).toList

private def ensureType (expected actual : TypeId) (span : Span) : LowerM Unit := do
  if ← compatibleType expected actual ((← get).tables.types.size + 1) then return
  let never ← internType .never
  if actual == never then return
  let tables := (← get).tables
  let expectedType ← typeNode? expected
  let actualType ← typeNode? actual
  let childType := fun typeId => tables.types[typeId.index]?
  let expectedNominal ← nominalDescription? expected
  let actualNominal ← nominalDescription? actual
  let named : Option String → String := fun description =>
    match description with
    | some description => s!" (`{description}`)"
    | none => ""
  let expectedDetail := match expectedType with
    | some (.vector element _) => s!" whose element is {repr (childType element)}"
    | _ => named expectedNominal
  let actualDetail := match actualType with
    | some (.vector element _) => s!" whose element is {repr (childType element)}"
    | _ => named actualNominal
  failAt "LEANER-TYPE-MISMATCH"
    s!"expression has type {repr actualType}{actualDetail} (arena entry {actual.index}), expected \
      {repr expectedType}{expectedDetail} (arena entry {expected.index})" (some span)

private def primitiveArity : Primitive → Nat
  | .repeatVector _ | .length | .destroyEmptyVector | .bitwiseNot | .logicalNot | .negate |
      .checkedNegate _ | .cast | .checkedCast _ | .profileNegate | .profileCast |
      .copyValue | .moveValue => 1
  | .slice | .swapVector | .reverseSliceVector | .insertVector => 3
  | .tuple | .vector => 0
  | _ => 2

private def primitiveLIR (specification : Bool) : Primitive → PrimitiveOperation
  | .tuple => .tuple
  | .vector => .vector
  | .repeatVector _ => .repeatVector
  | .pushVector => .pushVector
  | .concatVector => .concatVector
  | .insertVector => .insertVector
  | .removeVector => .removeVector
  | .swapVector => .swapVector
  | .reverseSliceVector => .reverseSliceVector
  | .destroyEmptyVector => .destroyEmptyVector
  | .containsVector => .containsVector
  | .indexOfVector => .indexOfVector
  | .checkVectorIndex failure => .checkVectorIndex failure.toLIR
  | .length => .length
  | .index => .index
  | .slice => .slice
  | .range => .range
  | .add => .add
  | .checkedAdd failure => if specification then .add else .checkedAdd failure.toLIR
  | .subtract => .subtract
  | .checkedSubtract failure => if specification then .subtract else .checkedSubtract failure.toLIR
  | .multiply => .multiply
  | .checkedMultiply failure => if specification then .multiply else .checkedMultiply failure.toLIR
  | .overflowingAdd => .overflowingAdd
  | .overflowingSubtract => .overflowingSubtract
  | .overflowingMultiply => .overflowingMultiply
  | .divide => .divide
  | .checkedDivide failure => if specification then .divide else .checkedDivide failure.toLIR
  | .modulo => .modulo
  | .checkedModulo failure => if specification then .modulo else .checkedModulo failure.toLIR
  | .bitwiseOr => .bitwiseOr
  | .bitwiseAnd => .bitwiseAnd
  | .bitwiseXor => .bitwiseXor
  | .bitwiseNot => .bitwiseNot
  | .shiftLeft => .shiftLeft
  | .checkedShiftLeft failure => if specification then .shiftLeft else .checkedShiftLeft failure.toLIR
  | .shiftRight => .shiftRight
  | .checkedShiftRight failure => if specification then .shiftRight else .checkedShiftRight failure.toLIR
  | .logicalAnd | .eagerLogicalAnd => .logicalAnd
  | .logicalOr | .eagerLogicalOr => .logicalOr
  | .logicalNot => .logicalNot
  | .equal => .equal
  | .notEqual => .notEqual
  | .less => .less
  | .greater => .greater
  | .lessEqual => .lessEqual
  | .greaterEqual => .greaterEqual
  | .negate => .negate
  | .checkedNegate failure => if specification then .negate else .checkedNegate failure.toLIR
  | .cast => .cast
  | .checkedCast failure => if specification then .cast else .checkedCast failure.toLIR
  | .implies => .implies
  | .equivalent => .equivalent
  | .identical => .identical
  | .copyValue => .copyValue
  | .moveValue => .moveValue
  | .profileAdd => .add
  | .profileSubtract => .subtract
  | .profileMultiply => .multiply
  | .profileDivide => .divide
  | .profileModulo => .modulo
  | .profileShiftLeft => .shiftLeft
  | .profileShiftRight => .shiftRight
  | .profileNegate => .negate
  | .profileCast => .cast

private def profilePrimitiveLIR (profile : ProfileName) (specification : Bool) :
    Primitive → PrimitiveOperation
  | .profileAdd => if specification then .add else match profile with
      | .move => .checkedAdd .abort | .rust => .add
  | .profileSubtract => if specification then .subtract else match profile with
      | .move => .checkedSubtract .abort | .rust => .subtract
  | .profileMultiply => if specification then .multiply else match profile with
      | .move => .checkedMultiply .abort | .rust => .multiply
  | .profileDivide => if specification then .divide else match profile with
      | .move => .checkedDivide .abort | .rust => .divide
  | .profileModulo => if specification then .modulo else match profile with
      | .move => .checkedModulo .abort | .rust => .modulo
  | .profileShiftLeft => if specification then .shiftLeft else match profile with
      | .move => .checkedShiftLeft .abort | .rust => .shiftLeft
  | .profileShiftRight => if specification then .shiftRight else match profile with
      | .move => .checkedShiftRight .abort | .rust => .shiftRight
  | .profileNegate => if specification then .negate else match profile with
      | .move => .checkedNegate .abort | .rust => .negate
  | .profileCast => if specification then .cast else match profile with
      | .move => .checkedCast .abort | .rust => .cast
  | operation => primitiveLIR specification operation

private def isBooleanPrimitive : Primitive → Bool
  | .logicalAnd | .logicalOr | .eagerLogicalAnd | .eagerLogicalOr |
      .logicalNot | .equal | .notEqual | .less |
      .greater | .lessEqual | .greaterEqual | .implies | .equivalent |
      .identical => true
  | _ => false

private def sourceFunction? (sourceNs : Namespace) (name : String) : Option FunctionDecl :=
  sourceNs.items.findSome? fun
    | .function declaration => if declaration.name == name then some declaration else none
    | _ => none

private def sourceConstant? (sourceNs : Namespace) (name : String) : Option ConstantDecl :=
  sourceNs.items.findSome? fun
    | .constant declaration => if declaration.name == name then some declaration else none
    | _ => none

private def sourceSpecFunction? (sourceNs : Namespace) (name : String) : Option SpecFunctionDecl :=
  sourceNs.items.findSome? fun
    | .specFunction declaration => if declaration.name == name then some declaration else none
    | _ => none

/-- The specification function a call path names, in whichever namespace of
this compilation unit declares it. A `use` alias may point at another
namespace, so the current one is not the only place to look. -/
private def specFunctionForPath? (state : BuildState) (segments : Array String) :
    Option SpecFunctionDecl := do
  let segments := (expandedUsePath? state.sourceNamespace segments).getD segments
  let localName ← segments.back?
  let owner := if segments.size == 1 then state.sourceNamespace.path else segments.pop
  let sourceNs ← state.source.namespaces.find? (·.path == owner)
  guard (sourceNs.profile == state.sourceNamespace.profile)
  sourceSpecFunction? sourceNs localName

private structure SourceNominal where
  name : String
  generics : Array GenericBinder := #[]
  fields : Array FieldDecl := #[]
  variants : Array VariantDecl := #[]
  abilities : Array Ability := #[]
  /-- A nominal another compilation unit declares. This unit knows its
  identity, which is what naming and constructing it need; its fields and
  variants belong to the declaring unit, so operations that read them report
  that boundary rather than guessing a shape. -/
  external : Bool := false
  /-- The namespace that declared this nominal. Its field and variant types are
  spelled in that namespace's own naming context. -/
  owner : Option Namespace := none

private def sourceNominal? (sourceNs : Namespace) (name : String) : Option SourceNominal :=
  sourceNs.items.findSome? fun
    | .struct declaration => if declaration.name == name then some {
        name, generics := declaration.generics, fields := declaration.fields
        abilities := declaration.abilities, owner := some sourceNs }
      else none
    | .enum declaration => if declaration.name == name then some {
        name, generics := declaration.generics, variants := declaration.variants
        abilities := declaration.abilities, owner := some sourceNs }
      else none
    | _ => none

private def resolveLocalNominal (segments : Array String) :
    LowerM (QualifiedRef × SourceNominal) := do
  if segments.isEmpty then
    failAt "LEANER-NOMINAL-NAME" "a nominal operation must name its target"
  let state ← get
  let segments := (expandedUsePath? state.sourceNamespace segments).getD segments
  let localName := segments.back!
  let owner := if segments.size == 1 then state.sourceNamespace.path else segments.pop
  let some sourceNs := state.source.namespaces.find? (·.path == owner)
    | -- The namespace belongs to a dependency: intern its identity the way an
      -- imported type name is interned, and leave its shape to its own unit.
      let (tables, namespaceId) := internNamespace state.tables owner
      let (tables, name) := internName tables namespaceId localName
      set { state with tables }
      return ({ namespaceId, name }, { name := localName, external := true })
  unless sourceNs.profile == state.sourceNamespace.profile do
    failAt "LEANER-CROSS-PROFILE"
      "cross-profile nominal operations require a validated boundary adapter"
  let some declaration := sourceNominal? sourceNs localName
    | failAt "LEANER-NOMINAL-NAME" s!"unknown nominal type `{localName}`"
  let some namespaceId := namespaceId? state.tables owner
    | failAt "LEANER-NOMINAL-NAME" s!"nominal namespace `{"::".intercalate owner.toList}` was not interned"
  let some name := nameId? state.tables namespaceId localName
    | failAt "LEANER-NOMINAL-NAME" s!"nominal type `{localName}` was not interned"
  return ({ namespaceId, name }, declaration)

private def resolveLocalConstructor (segments : Array String) :
    LowerM (QualifiedRef × SourceNominal × Option String) := do
  if segments.isEmpty then
    failAt "LEANER-CONSTRUCTOR-NAME" "a constructor expression must name its target"
  let state ← get
  let directPrefix := segments.extract 0 (segments.size - 1)
  if (segments.size == 1 || directPrefix == state.sourceNamespace.path) &&
      (sourceNominal? state.sourceNamespace segments.back!).isSome then
    let (reference, declaration) ← resolveLocalNominal segments
    return (reference, declaration, none)
  if segments.size < 2 then
    failAt "LEANER-CONSTRUCTOR-NAME" s!"unknown constructor `{segments.back!}`"
  let variant := segments.back!
  let owner := segments.extract 0 (segments.size - 1)
  let (reference, declaration) ← resolveLocalNominal owner
  unless declaration.external || declaration.variants.any (·.name == variant) do
    failAt "LEANER-CONSTRUCTOR-NAME"
      s!"enum `{declaration.name}` has no variant `{variant}`"
  return (reference, declaration, some variant)

private def constructorFields? (declaration : SourceNominal)
    (variant : Option String) : Option (Array FieldDecl) :=
  -- A dependency's declaration lists no fields here. Its payload-free
  -- constructors are complete as written; the rest need the shape its own
  -- unit holds.
  if declaration.external then some #[] else
  match variant with
  | none => if declaration.variants.isEmpty then some declaration.fields else none
  | some name => (declaration.variants.find? (·.name == name)).map (·.fields)

/-- Recover a constructor-pattern owner when the nominal type parser consumed
the final `::Variant` segment.  This is the binding-pattern counterpart of
the constructor-expression normalization below. -/
private def resolvedPatternConstructor (owner : Located Ty) (variant : Option String) :
    LowerM (Located Ty × Option String) := do
  let .named ownerSegments arguments := owner.value
    | failAt "LEANER-PATTERN-TYPE"
        "a constructor pattern must name a nominal owner" (some owner.span)
  let constructorSegments := variant.map ownerSegments.push |>.getD ownerSegments
  let (_, _, resolvedVariant) ← resolveLocalConstructor constructorSegments
  let resultOwner := if variant.isNone && resolvedVariant.isSome then
      { owner with value := .named ownerSegments.pop arguments }
    else owner
  pure (resultOwner, resolvedVariant)

private def fieldsNamed (declaration : SourceNominal) (field : String) : Array FieldDecl :=
  if declaration.variants.isEmpty then declaration.fields.filter (·.name == field)
  else declaration.variants.filterMap fun variant => variant.fields.find? (·.name == field)

private partial def instantiateTypeIdFuel (instantiations : Array GenericArgument)
    (id : TypeId) (fuel : Nat) (span : Span) : LowerM TypeId := do
  if fuel == 0 then
    failAt "LEANER-GENERIC-CYCLE" "cyclic type while instantiating nominal fields" (some span)
  let some type := (← get).tables.types[id.index]?
    | failAt "LEANER-TYPE-ID" s!"missing type arena entry {id.index}" (some span)
  match type with
  | .typeParameter index => do
      match instantiations[index]? with
      | some (.typeArg value) => pure value.typeId
      | _ =>
          failAt "LEANER-GENERIC-ARGUMENT"
            s!"type parameter {index} has no type instantiation" (some span)
  | .tuple elements =>
      internType (.tuple (← elements.mapM (instantiateTypeIdFuel instantiations · (fuel - 1) span)))
  | .vector element length =>
      internType (.vector (← instantiateTypeIdFuel instantiations element (fuel - 1) span) length)
  | .typeDomain nested =>
      internType (.typeDomain (← instantiateTypeIdFuel instantiations nested (fuel - 1) span))
  | .resourceDomain resource arguments =>
      let arguments ← arguments.mapM fun values =>
        values.mapM (instantiateTypeIdFuel instantiations · (fuel - 1) span)
      internType (.resourceDomain resource arguments)
  | .nominal name arguments =>
      let arguments ← arguments.mapM fun argument => match argument with
        | .typeArg value => do
            let typeId ← instantiateTypeIdFuel instantiations value.typeId (fuel - 1) span
            pure (.typeArg { value with typeId })
        | value => pure value
      internType (.nominal name arguments)
  | .function arguments result abilities =>
      let arguments ← arguments.mapM (instantiateTypeIdFuel instantiations · (fuel - 1) span)
      let result ← instantiateTypeIdFuel instantiations result (fuel - 1) span
      internType (.function arguments result abilities)
  | .reference reference =>
      let referent ← instantiateTypeIdFuel instantiations reference.referent (fuel - 1) span
      let lifetime := match (← get).tables.lifetimes[reference.lifetime.index]? with
        | some { kind := .parameter index, .. } =>
            match instantiations[index]? with
            | some (.lifetime lifetime) => lifetime
            | _ => reference.lifetime
        | _ => reference.lifetime
      internType (.reference { reference with referent, lifetime })
  | type => internType type

private def instantiateTypeId (instantiations : Array GenericArgument)
    (id : TypeId) (span : Span) : LowerM TypeId := do
  instantiateTypeIdFuel instantiations id ((← get).tables.types.size + 1) span

/-- Infer specification-function type arguments through the full parameter
shape. Move specs routinely mention a binder below a nominal constructor (for
example `Option<Element>`), so treating only a bare `Element` parameter as
inferable rejects ordinary stdlib contracts. -/
private partial def inferTypeArgumentsFuel (specification : Bool)
    (inferred : Array (Option TypeId)) (pattern actual : TypeId) (fuel : Nat)
    (span : Span) : LowerM (Array (Option TypeId)) := do
  if fuel == 0 then
    failAt "LEANER-SPEC-CALL-INFERENCE" "cyclic type while inferring specification arguments"
      (some span)
  let patternNode ← typeNode? pattern
  let actualNode ← typeNode? actual
  match patternNode, actualNode with
  | some (.typeParameter index), _ =>
      let some slot := inferred[index]?
        | failAt "LEANER-SPEC-CALL-INFERENCE"
            s!"type parameter {index} is outside the specification function's binders"
              (some span)
      if let some previous := slot then
        if specification then
          let projectedPrevious ← projectSpecTypeId previous
          let projectedActual ← projectSpecTypeId actual
          ensureType projectedPrevious projectedActual span
          -- A direct scalar gives only its logical type, while a later
          -- nominal/vector occurrence fixes the physical representation.
          -- Refine `Int` to that representation before instantiating the
          -- full parameters; otherwise argument order invents Choice<Int>
          -- for a call whose actual choice is Choice<u64>.
          if previous == projectedActual && actual != projectedActual then
            return inferred.set! index (some actual)
        else ensureType previous actual span
        pure inferred
      else
        pure (inferred.set! index (some actual))
  | some (.tuple patterns), some (.tuple actuals) =>
      if patterns.size != actuals.size then pure inferred else
      (patterns.zip actuals).foldlM (init := inferred) fun inferred (pattern, actual) =>
        inferTypeArgumentsFuel specification inferred pattern actual (fuel - 1) span
  | some (.vector pattern _), some (.vector actual _) =>
      inferTypeArgumentsFuel specification inferred pattern actual (fuel - 1) span
  | some (.nominal patternName patterns), some (.nominal actualName actuals) =>
      if patternName != actualName || patterns.size != actuals.size then pure inferred else
      (patterns.zip actuals).foldlM (init := inferred) fun inferred pair =>
        match pair with
        | (.typeArg pattern, .typeArg actual) =>
            inferTypeArgumentsFuel specification inferred pattern.typeId actual.typeId
              (fuel - 1) span
        | _ => pure inferred
  | some (.reference pattern), some (.reference actual) =>
      inferTypeArgumentsFuel specification inferred pattern.referent actual.referent
        (fuel - 1) span
  | some (.function patternArguments patternResult _),
      some (.function actualArguments actualResult _) =>
      if patternArguments.size != actualArguments.size then pure inferred else
      let inferred ← (patternArguments.zip actualArguments).foldlM (init := inferred)
        fun inferred (pattern, actual) =>
          inferTypeArgumentsFuel specification inferred pattern actual (fuel - 1) span
      inferTypeArgumentsFuel specification inferred patternResult actualResult (fuel - 1) span
  | _, _ => pure inferred

private def inferTypeArguments (specification : Bool) (inferred : Array (Option TypeId))
    (pattern actual : TypeId) (span : Span) : LowerM (Array (Option TypeId)) := do
  -- Projection is shape-sensitive: direct/function positions become logical,
  -- while representation-bearing containers retain their element types.
  -- Project each complete argument once, then recurse without projecting a
  -- newly inferred type parameter again after descending through a vector or
  -- nominal type. Repeated occurrences are still compared logically, so a
  -- representation-bearing `Vector<u64>` element agrees with a direct `Int`
  -- value at a specification call boundary.
  let pattern ← if specification then projectSpecTypeId pattern else pure pattern
  let actual ← if specification then projectSpecTypeId actual else pure actual
  inferTypeArgumentsFuel specification inferred pattern actual
    ((← get).tables.types.size + 1) span

/-- Does this nominal type declare variants? Selecting a field of one reads
through every variant that has it, which is a different LIR operation. -/
private def nominalHasVariants (baseType : TypeId) : LowerM Bool := do
  match ← typeNode? baseType with
  | some (.nominal owner _) =>
      let state ← get
      match state.tables.names[owner.index]? with
      | some ownerName =>
          pure ((sourceNominal? state.sourceNamespace ownerName.name).any
            fun declaration => !declaration.variants.isEmpty)
      | none => pure false
  | _ => pure false

private def resolveNominalField (baseType : TypeId) (field : String) (span : Span) :
    LowerM (QualifiedRef × NameId × TypeId) := do
  let some (.nominal owner instantiations) ← typeNode? baseType
    | failAt "LEANER-PLACE-FIELD" "a field place must have a nominal base" (some span)
  let state ← get
  let some ownerName := state.tables.names[owner.index]?
    | failAt "LEANER-PLACE-FIELD" "a field place references a missing nominal name" (some span)
  -- The field belongs to the namespace that declared the nominal, which any
  -- namespace this unit compiles may be — a dependency interface included.
  let ownerPath := (state.tables.namespaces[ownerName.namespaceId.index]?).map (·.segments)
  let some ownerNs := state.source.namespaces.find? (some ·.path == ownerPath)
    | failAt "LEANER-PLACE-FIELD"
        s!"field `{field}` needs the declaration of `{ownerName.name}`, which this \
          compilation unit does not hold" (some span)
  unless ownerNs.profile == state.sourceNamespace.profile do
    failAt "LEANER-CROSS-PROFILE"
      "cross-profile field places require a validated boundary adapter" (some span)
  let some declaration := sourceNominal? ownerNs ownerName.name
    | failAt "LEANER-PLACE-FIELD" s!"unknown nominal type `{ownerName.name}`" (some span)
  let selected := fieldsNamed declaration field
  if selected.isEmpty then
    failAt "LEANER-PLACE-FIELD"
      s!"nominal type `{declaration.name}` has no field `{field}`" (some span)
  let declarationTypes ← liftM <| typeContext declaration.generics declaration.owner
  let selectedTypes ← selected.mapM fun sourceField => do
    let declared := (← lowerTypeUse declarationTypes sourceField.type).typeId
    instantiateTypeId instantiations declared span
  let typeId := selectedTypes[0]!
  unless selectedTypes.all (· == typeId) do
    failAt "LEANER-PLACE-FIELD"
      s!"field `{field}` has different types across enum variants" (some span)
  let current ← get
  let fieldName ← match nameId? current.tables ownerName.namespaceId field with
    | some name => pure name
    | none =>
        let (tables, name) := internName current.tables ownerName.namespaceId field
        set { current with tables }
        pure name
  return ({ namespaceId := ownerName.namespaceId, name := owner }, fieldName, typeId)

private def nominalPatternDeclaration (typeId : TypeId) (span : Span) :
    LowerM (SourceNominal × Array GenericArgument) := do
  let some (.nominal owner instantiations) ← typeNode? typeId
    | failAt "LEANER-PATTERN-TYPE" "a constructor pattern must be nominal" (some span)
  let state ← get
  let some qualified := state.tables.names[owner.index]?
    | failAt "LEANER-PATTERN-TYPE" "a constructor pattern has a missing owner" (some span)
  -- The owner may be declared by any namespace this unit compiles, not only
  -- the one being lowered; a dependency's declaration is not among them.
  let ownerPath := (state.tables.namespaces[qualified.namespaceId.index]?).map (·.segments)
  let ownerNamespace? := state.source.namespaces.find? fun candidate =>
    some candidate.path == ownerPath
  let some declaration := ownerNamespace?.bind (sourceNominal? · qualified.name)
    | if qualified.namespaceId == state.namespaceId then
        failAt "LEANER-PATTERN-TYPE" "a constructor pattern owner does not resolve" (some span)
      else
        failAt "LEANER-NOMINAL-EXTERNAL"
          s!"`{qualified.name}` is declared by another compilation unit, whose fields and \
            variants are not part of this one, so a constructor pattern over it needs the \
            declarations dependency interfaces do not carry yet" (some span)
  pure (declaration, instantiations)

private def nominalPatternFields (typeId : TypeId) (variant : Option String)
    (span : Span) : LowerM (SourceNominal × Array GenericArgument × Array FieldDecl) := do
  let (declaration, instantiations) ← nominalPatternDeclaration typeId span
  let fields ← match variant with
    | none => do
        unless declaration.variants.isEmpty do
          failAt "LEANER-PATTERN-VARIANT"
            "an enum constructor pattern must name a variant" (some span)
        pure declaration.fields
    | some variant => do
        let some variantDecl := declaration.variants.find? (·.name == variant)
          | failAt "LEANER-PATTERN-VARIANT"
              s!"unknown enum variant `{variant}`" (some span)
        pure variantDecl.fields
  return (declaration, instantiations, fields)

private def nominalPatternFieldType (declaration : SourceNominal)
    (instantiations : Array GenericArgument) (field : FieldDecl) (span : Span) :
    LowerM TypeId := do
  let declarationTypes ← liftM <| typeContext declaration.generics declaration.owner
  let declared := (← lowerTypeUse declarationTypes field.type).typeId
  instantiateTypeId instantiations declared span

/-- Unqualified field aliases cannot distinguish differently typed payloads
with the same name. Such by-value matches must retain their typed patterns. -/
private def uniformEnumFieldTypes (declaration : SourceNominal) : LowerM Bool := do
  -- Match the printer's declaration-level criterion even when a particular
  -- instantiation happens to make distinct generic payload types equal.
  let declarationTypes ← liftM <| typeContext declaration.generics declaration.owner
  let mut seen : Std.HashMap String TypeId := {}
  for variant in declaration.variants do
    for field in variant.fields do
      let typeId := (← lowerTypeUse declarationTypes field.type).typeId
      if let some previous := seen[field.name]? then
        if previous != typeId then return false
      else
        seen := seen.insert field.name typeId
  return true

private def constructorPatternOwnerType (typeId : TypeId) (span : Span) :
    LowerM (TypeId × Option ReferenceType) := do
  return ← match ← typeNode? typeId with
  | some (.nominal ..) => pure (typeId, none)
  | some (.reference reference) =>
      let some (.nominal ..) ← typeNode? reference.referent
        | failAt "LEANER-PATTERN-TYPE"
            "a referenced constructor pattern must refer to a nominal type" (some span)
      pure (reference.referent, some reference)
  | _ => do
      failAt "LEANER-PATTERN-TYPE"
        "a constructor pattern must have a nominal or nominal-reference type" (some span)

private def constructorPatternFieldType (declaration : SourceNominal)
    (instantiations : Array GenericArgument) (reference : Option ReferenceType)
    (field : FieldDecl) (span : Span) : LowerM TypeId := do
  let fieldType ← nominalPatternFieldType declaration instantiations field span
  match reference with
  | none => pure fieldType
  | some reference => internType (.reference { reference with referent := fieldType })

private partial def predeclareBindingPattern (types : TypeContext) (mutable : Bool)
    (pattern : BindingPattern) (typeId : TypeId) (nextLocal : Nat) :
    LowerM (Array (ScopedLocal × LocalDecl)) := do
  match pattern with
  | .wildcard _ => pure #[]
  | .literal _ _ => pure #[]
  | .variable name span bindingMutable =>
      let loc ← addLoc span
      let id : LocalId := ⟨nextLocal⟩
      let localInfo : ScopedLocal := { span, name, id, typeId }
      let declaration : LocalDecl := {
        id, name, type := { typeId, loc },
        mutable := mutable || bindingMutable, loc }
      pure #[(localInfo, declaration)]
  | .tuple elements span =>
      let some (.tuple tupleTypes) ← typeNode? typeId
        | failAt "LEANER-PATTERN-TYPE" "a tuple pattern must have a tuple type" (some span)
      unless elements.size == tupleTypes.size do
        failAt "LEANER-PATTERN-ARITY" "a tuple pattern has the wrong arity" (some span)
      let mut declarations := #[]
      for (element, elementType) in elements.zip tupleTypes do
        let nested ← predeclareBindingPattern types mutable element elementType
          (nextLocal + declarations.size)
        declarations := declarations ++ nested
      pure declarations
  | .constructor owner variant fields span =>
      let (owner, variant) ← resolvedPatternConstructor owner variant
      let ownerType := (← lowerTypeUse types owner).typeId
      let (patternOwnerType, reference) ← constructorPatternOwnerType typeId span
      ensureType patternOwnerType ownerType span
      let (declaration, instantiations, declaredFields) ←
        nominalPatternFields patternOwnerType variant span
      unless fields.size == declaredFields.size && declaredFields.all fun declared =>
          fields.any (·.1 == declared.name) do
        failAt "LEANER-PATTERN-FIELDS"
          "a constructor pattern must cover every field exactly once" (some span)
      let mut declarations := #[]
      for declared in declaredFields do
        let some child := fields.find? (·.1 == declared.name) | unreachable!
        let fieldType ← constructorPatternFieldType declaration instantiations reference
          declared child.2.span
        let nested ← predeclareBindingPattern types mutable child.2 fieldType
          (nextLocal + declarations.size)
        declarations := declarations ++ nested
      pure declarations

private partial def bindingPatternLocals (context : ExprContext)
    (pattern : BindingPattern) : LowerM (Array (String × LocalId × TypeId)) := do
  match pattern with
  | .wildcard _ => pure #[]
  | .literal _ _ => pure #[]
  | .variable name span _ =>
      let some declaration := declaredLocal? context span name
        | failAt "LEANER-LOCAL-DECLARATION"
            s!"pattern local `{name}` was not predeclared" (some span)
      pure #[(name, declaration.id, declaration.typeId)]
  | .tuple elements _ =>
      elements.foldlM (init := #[]) fun locals child => do
        pure (locals ++ (← bindingPatternLocals context child))
  | .constructor _ _ fields _ =>
      fields.foldlM (init := #[]) fun locals (_, child) => do
        pure (locals ++ (← bindingPatternLocals context child))

private partial def lowerBindingPattern (context : ExprContext)
    (pattern : BindingPattern) (typeId : TypeId) : LowerM PatternId := do
  let loc ← addLoc pattern.span
  match pattern with
  | .wildcard _ => pushPattern { loc, typeId, kind := .wildcard }
  | .literal literal span =>
      let (literalType, value) ← match literal with
        | .bool value => pure (← internType .bool, ConstValue.bool value)
        | .char value => pure (← internType .character, ConstValue.character value)
        | .integer value =>
            unless (← typeNode? typeId).any fun | .integer _ _ => true | _ => false do
              failAt "LEANER-PATTERN-TYPE" "an integer pattern requires an integer scrutinee"
                (some span)
            pure (typeId, ConstValue.integer value)
      ensureType typeId literalType span
      pushPattern { loc, typeId, kind := .literal value }
  | .variable name span _ =>
      let some declaration := declaredLocal? context span name
        | failAt "LEANER-LOCAL-DECLARATION"
            s!"pattern local `{name}` was not predeclared" (some span)
      ensureType typeId declaration.typeId span
      pushPattern { loc, typeId, kind := .variable declaration.id }
  | .tuple elements span =>
      let some (.tuple types) ← typeNode? typeId
        | failAt "LEANER-PATTERN-TYPE" "a tuple pattern must have a tuple type" (some span)
      unless elements.size == types.size do
        failAt "LEANER-PATTERN-ARITY" "a tuple pattern has the wrong arity" (some span)
      let children ← (elements.zip types).mapM fun (element, elementType) => do
        let elementType ← if context.specification then projectSpecTypeId elementType
          else pure elementType
        lowerBindingPattern context element elementType
      pushPattern { loc, typeId, kind := .tuple children }
  | .constructor owner variant fields span =>
      let (owner, variant) ← resolvedPatternConstructor owner variant
      let ownerType := (← lowerTypeUse context.types owner).typeId
      let (patternOwnerType, reference) ← constructorPatternOwnerType typeId span
      ensureType patternOwnerType ownerType span
      let some (.nominal ownerName _) ← typeNode? patternOwnerType
        | failAt "LEANER-PATTERN-TYPE" "a constructor pattern must be nominal" (some span)
      let (declaration, instantiations, declaredFields) ←
        nominalPatternFields patternOwnerType variant span
      unless fields.size == declaredFields.size && declaredFields.all fun declared =>
          fields.any (·.1 == declared.name) do
        failAt "LEANER-PATTERN-FIELDS"
          "a constructor pattern must cover every field exactly once" (some span)
      let mut children := #[]
      for declared in declaredFields do
        let some child := fields.find? (·.1 == declared.name) | unreachable!
        let fieldType ← constructorPatternFieldType declaration instantiations reference
          declared child.2.span
        let fieldType ← if context.specification then projectSpecTypeId fieldType
          else pure fieldType
        children := children.push (← lowerBindingPattern context child.2 fieldType)
      pushPattern { loc, typeId, kind :=
        .constructor ownerName instantiations variant children }

/-- Lower a pattern which writes existing locals. Unlike a binding pattern,
its variables resolve in the active scope and do not introduce declarations. -/
private partial def lowerAssignmentPattern (context : ExprContext)
    (pattern : BindingPattern) (typeId : TypeId) : LowerM PatternId := do
  let loc ← addLoc pattern.span
  match pattern with
  | .wildcard _ => pushPattern { loc, typeId, kind := .wildcard }
  | .literal _ span =>
      failAt "LEANER-ASSIGNMENT-PATTERN"
        "a literal pattern cannot be used as an assignment target" (some span)
  | .variable name span _ =>
      let some (localId, localType) := lookupLocal? context name
        | failAt "LEANER-LOCAL-NAME" s!"unknown assignment local `{name}`" (some span)
      ensureType typeId localType span
      pushPattern { loc, typeId, kind := .variable localId }
  | .tuple elements span =>
      let some (.tuple types) ← typeNode? typeId
        | failAt "LEANER-PATTERN-TYPE"
            "a tuple assignment pattern must have a tuple type" (some span)
      unless elements.size == types.size do
        failAt "LEANER-PATTERN-ARITY"
          "a tuple assignment pattern has the wrong arity" (some span)
      let children ← (elements.zip types).mapM fun (element, elementType) => do
        let elementType ← if context.specification then projectSpecTypeId elementType
          else pure elementType
        lowerAssignmentPattern context element elementType
      pushPattern { loc, typeId, kind := .tuple children }
  | .constructor owner variant fields span =>
      let ownerType := (← lowerTypeUse context.types owner).typeId
      ensureType typeId ownerType span
      let some (.nominal ownerName instantiations) ← typeNode? typeId
        | failAt "LEANER-PATTERN-TYPE"
            "a constructor assignment pattern must be nominal" (some span)
      let state ← get
      let some qualified := state.tables.names[ownerName.index]?
        | failAt "LEANER-PATTERN-TYPE"
            "a constructor assignment pattern has a missing owner" (some span)
      let some declaration := sourceNominal? state.sourceNamespace qualified.name
        | failAt "LEANER-PATTERN-TYPE"
            "a constructor assignment pattern owner does not resolve" (some span)
      let declaredFields : Array FieldDecl ← match variant with
        | none => do
            unless declaration.variants.isEmpty do
              failAt "LEANER-PATTERN-VARIANT"
                "an enum constructor assignment pattern must name a variant" (some span)
            pure declaration.fields
        | some variant => match declaration.variants.find? (·.name == variant) with
            | some declaration => pure declaration.fields
            | none =>
                failAt "LEANER-PATTERN-VARIANT" s!"unknown enum variant `{variant}`" (some span)
      unless fields.size == declaredFields.size && declaredFields.all fun declared =>
          fields.any (·.1 == declared.name) do
        failAt "LEANER-PATTERN-FIELDS"
          "a constructor assignment pattern must cover every field exactly once" (some span)
      let mut children := #[]
      for declared in declaredFields do
        let some child := fields.find? (·.1 == declared.name) | unreachable!
        let (_, _, fieldType) ← resolveNominalField typeId declared.name child.2.span
        let fieldType ← if context.specification then projectSpecTypeId fieldType
          else pure fieldType
        children := children.push
          (← lowerAssignmentPattern context child.2 fieldType)
      pushPattern { loc, typeId, kind :=
        .constructor ownerName instantiations variant children }

/-- Lower an authored storage path and recover its static type.  This is kept
separate from expression lowering because dereference/field projections are
arena `Place` nodes in LIR, not value-producing operations. -/
private partial def lowerPlace (context : ExprContext) (source : Place) :
    LowerM (PlaceId × TypeId) := do
  let span := source.span
  match source with
  | .local name _ =>
      let some (localId, typeId) := lookupLocal? context name
        | failAt "LEANER-PLACE-LOCAL" s!"unknown local `{name}`" (some span)
      return (← pushPlace (.localVar localId), typeId)
  | .deref base _ =>
      let (base, baseType) ← lowerPlace context base
      let some (.reference reference) ← typeNode? baseType
        | failAt "LEANER-PLACE-DEREF" "a dereferenced place must contain a reference"
            (some span)
      return (← pushPlace (.deref base), reference.referent)
  | .field base field _ =>
      let (base, baseType) ← lowerPlace context base
      let (base, baseType) ← match ← typeNode? baseType with
        | some (.reference reference) =>
            pure (← pushPlace (.deref base), reference.referent)
        | _ => pure (base, baseType)
      let (owner, fieldName, typeId) ← resolveNominalField baseType field span
      return (← pushPlace (.field base owner fieldName), typeId)

private partial def lowerSpecificationPlaceValue (context : ExprContext) (source : Place) :
    LowerM (ExprId × TypeId) := do
  let span := source.span
  match source with
  | .local name _ =>
      let some (localId, typeId) := lookupLocal? context name
        | failAt "LEANER-PLACE-LOCAL" s!"unknown local `{name}`" (some span)
      let loc ← addLoc span
      return (← pushExpression loc typeId (.localVar localId), typeId)
  | .deref base _ => lowerSpecificationPlaceValue context base
  | .field base field _ =>
      let base ← lowerSpecificationPlaceValue context base
      let (reference, _, selectedType) ← resolveNominalField base.2 field span
      let selectedType ← projectSpecTypeId selectedType
      let loc ← addLoc span
      let id ← pushExpression loc selectedType <| .operation
        (.data (.select reference field)) #[] #[base.1]
      return (id, selectedType)

private partial def expressionPlace? : Expr → Option Place
  | .local name span => some (.local name span)
  | .field value field span => do
      some (.field (← expressionPlace? value) field span)
  | .dereference value span => do
      some (.deref (← expressionPlace? value) span)
  | _ => none

private def literalIndex? : Expr → Option Nat
  | .integer value _ | .typedInteger value _ _ =>
      if value < 0 then none else some value.toNat
  | _ => none

private def repeatVectorExpectedElement (expected : Option TypeId) (length : Nat)
    (span : Span) : LowerM (Option TypeId) := do
  let some expected := expected | return none
  let expectedNode ← typeNode? expected
  return ← match expectedNode with
  | some (.vector element (some (.integer expectedLength))) =>
      if expectedLength == Int.ofNat length then pure (some element)
      else
        failAt "LEANER-REPEAT-VECTOR-LENGTH"
          (s!"a repeated vector of length {length} has expected length {expectedLength}")
          (some span)
  | _ =>
      failAt "LEANER-REPEAT-VECTOR-CONTEXT"
        "a repeated vector literal has a non-fixed-vector expected type" (some span)

private def resolveLocalRef (segments : Array String) (span : Span) :
    LowerM (QualifiedRef × FunctionDecl) := do
  if segments.isEmpty then
    failAt "LEANER-CALL-NAME" "a function call must name a target" (some span)
  let state ← get
  let segments := (expandedUsePath? state.sourceNamespace segments).getD segments
  let localName := segments.back!
  let owner := if segments.size == 1 then state.sourceNamespace.path else segments.pop
  let some sourceNs := state.source.namespaces.find? (·.path == owner)
    | failAt "LEANER-CALL-NAME" s!"unknown function namespace `{"::".intercalate owner.toList}`"
        (some span)
  unless sourceNs.profile == state.sourceNamespace.profile do
    failAt "LEANER-CROSS-PROFILE"
      "cross-profile calls require a validated boundary adapter" (some span)
  let some declaration := sourceFunction? sourceNs localName
    | failAt "LEANER-CALL-NAME" s!"unknown function `{localName}`" (some span)
  let some namespaceId := namespaceId? state.tables owner
    | failAt "LEANER-CALL-NAME" s!"function namespace `{"::".intercalate owner.toList}` was not interned"
        (some span)
  let some name := nameId? state.tables namespaceId localName
    | failAt "LEANER-CALL-NAME" s!"function `{localName}` was not interned" (some span)
  return ({ namespaceId, name }, declaration)

/-- The source namespace a resolved reference points into, when this unit
holds it. A callee's signature is written in its own naming context, so
lowering that signature needs the namespace that declared it. -/
private def namespaceOfRef? (reference : QualifiedRef) : LowerM (Option Namespace) := do
  let state ← get
  let some path := state.tables.namespaces[reference.namespaceId.index]?.map (·.segments)
    | return none
  return state.source.namespaces.find? (·.path == path)

private def isExternalPath (segments : Array String) : LowerM Bool := do
  let sourceNs := (← get).sourceNamespace
  let segments := (expandedUsePath? sourceNs segments).getD segments
  if segments.size <= 1 then return false
  return segments.extract 0 (segments.size - 1) != sourceNs.path

private def resolveExternalRef (segments : Array String) : LowerM QualifiedRef := do
  let sourceNs := (← get).sourceNamespace
  let segments := (expandedUsePath? sourceNs segments).getD segments
  if segments.size <= 1 then
    failAt "LEANER-CALL-NAME" "an external call must carry a qualified name"
  let owner := segments.extract 0 (segments.size - 1)
  let localName := segments.back!
  let state ← get
  let (tables, namespaceId) := internNamespace state.tables owner
  let (tables, name) := internName tables namespaceId localName
  set { state with tables }
  return { namespaceId, name }

private def resolveLocalSpecRef (segments : Array String) :
    LowerM (QualifiedRef × SpecFunctionDecl) := do
  if segments.isEmpty then
    failAt "LEANER-SPEC-CALL-NAME" "a specification-function call must name a target"
  let state ← get
  let segments := (expandedUsePath? state.sourceNamespace segments).getD segments
  let localName := segments.back!
  let owner := if segments.size == 1 then state.sourceNamespace.path else segments.pop
  let some sourceNs := state.source.namespaces.find? (·.path == owner)
    | failAt "LEANER-SPEC-CALL-NAME"
        s!"unknown specification-function namespace `{"::".intercalate owner.toList}`"
  unless sourceNs.profile == state.sourceNamespace.profile do
    failAt "LEANER-CROSS-PROFILE"
      "cross-profile specification calls require a validated boundary adapter"
  let some declaration := sourceSpecFunction? sourceNs localName
    | failAt "LEANER-SPEC-CALL-NAME" s!"unknown specification function `{localName}`"
  let some namespaceId := namespaceId? state.tables owner
    | failAt "LEANER-SPEC-CALL-NAME"
        s!"specification-function namespace `{"::".intercalate owner.toList}` was not interned"
  let some name := nameId? state.tables namespaceId localName
    | failAt "LEANER-SPEC-CALL-NAME"
        s!"specification function `{localName}` was not interned"
  return ({ namespaceId, name }, declaration)

private def addOriginAndAlignment (loc : LocId) : LowerM (OriginId × AlignmentId) := do
  let state ← get
  let origin : OriginId := ⟨state.tables.origins.size⟩
  let alignment : AlignmentId := ⟨state.tables.alignments.size⟩
  let sourceIdentity := some state.source.sourceName
  set { state with tables := {
    state.tables with
    origins := state.tables.origins.push {
      kind := .leanerSource
      location := loc
      sourceIdentity
      description := "authored Leaner source" }
    alignments := state.tables.alignments.push {
      source := origin
      trust := .authored
      description := "direct Leaner source elaboration" } } }
  return (origin, alignment)

private def addArbitrarySpecValue (context : ExprContext) (typeId : TypeId)
    (span : Span) : LowerM (ExprId × TypeId) := do
  let loc ← addLoc span
  let state ← get
  let generated := s!"__leaner_arbitrary_{state.output.specFunctions.size}_{state.output.expressions.size}"
  let (tables, name) := internName state.tables state.namespaceId generated
  set { state with tables }
  let (origin, _) ← addOriginAndAlignment loc
  let result : TypeUse := { typeId, loc }
  let instantiations ← context.generics.zipIdx.mapM fun (binder, index) => do
    return ← match binder.kind with
    | .typeArg =>
        let parameterType ← internType (.typeParameter index)
        pure <| GenericArgument.typeArg { typeId := parameterType, loc }
    | _ => do
        failAt "LEANER-ARBITRARY-GENERICS"
          "arbitrary specification values currently require type-only generic binders" (some span)
  let declaration : LeanerIR.SpecFunctionDecl := {
    loc
    name
    profile := state.sourceNamespace.profile.toLIR
    signature := {
      generics := context.generics
      results := if (← typeNode? typeId) == some .unit then #[] else #[result] }
    origin }
  modify fun state => { state with output := { state.output with
    specFunctions := state.output.specFunctions.push declaration } }
  let expression ← pushExpression loc typeId <| .operation
    (.specification (.functionCall { namespaceId := state.namespaceId, name } {}))
      instantiations #[]
  return (expression, typeId)

private def runtimeIndexType : LowerM TypeId := do
  match (← get).sourceNamespace.profile with
  | .move => internType (.integer (.bits 64) false)
  | .rust => internType (.integer .pointer false)

/-- Does this bare name denote a value here? A specification's `result` is a
value the surface spells as a name without binding it, so a projection of it
belongs to that value and not to a Move resource family of the same name. -/
private def namesValue (context : ExprContext) (name : String) : Bool :=
  (lookupLocal? context name).isSome || (context.specification && name == "result")

/-- The Move 2 grammar deliberately leaves `head[index]` context-sensitive.
An uninstantiated one-segment head denotes a vector local when that local is
in scope; every other type-shaped head denotes a global resource family. -/
private def storageIndexLocalBase? (context : ExprContext) (head : Located Ty) :
    Option Expr := do
  let .named segments arguments := head.value | none
  if !arguments.isEmpty then none else
  let [source] := segments.toList | none
  let names := source.splitOn "."
  let some name := names.head? | none
  let rest := names.drop 1
  if !namesValue context name then none else
  some <| rest.foldl (fun base field => .field base field head.span)
    (.local name head.span)

private def storageIndexLocalName? (context : ExprContext) (head : Located Ty) :
    Option String := do
  let base ← storageIndexLocalBase? context head
  let rec rootName? : Expr → Option String
    | .local name _ => some name
    | .field base _ _ => rootName? base
    | _ => none
  rootName? base

private partial def storageProjection? (context : ExprContext) : Expr →
    Option (Located Ty × Expr × Array String)
  | .storageIndex head index _ => some (head, index, #[])
  | .index (.local name headSpan) index _ => do
      if namesValue context name then none else
      some ({ value := .named #[name] #[], span := headSpan }, index, #[])
  | .field base field _ => do
      let (head, index, fields) ← storageProjection? context base
      some (head, index, fields.push field)
  | _ => none

private partial def localStoragePlace? (context : ExprContext) : Expr → Option Expr
  | source@(.local name _) =>
      if (lookupLocal? context name).isSome then some source else none
  | .dereference base span => do
      some (.dereference (← localStoragePlace? context base) span)
  | .field base field span => do
      some (.field (← localStoragePlace? context base) field span)
  | .index base index span => do
      some (.index (← localStoragePlace? context base) index span)
  | .storageIndex head index span => do
      let base ← storageIndexLocalBase? context head
      some (.index base index span)
  | _ => none

/-- Lower expression-shaped Move 2 places. Index expressions are lowered by
the caller so this helper can remain outside the mutually recursive source
expression lowering function. Field and index projection through a reference
insert the implicit dereference prescribed by Move's place syntax. -/
private partial def lowerExpressionPlaceWith
    (lowerIndex : Expr → LowerM (ExprId × TypeId)) (context : ExprContext)
    (source : Expr) (checkBounds : Bool := true) (captureLiteralIndexes : Bool := false) :
    LowerM (PlaceId × TypeId × Array (PatternId × ExprId) × LowerM ExprId) := do
  let sourceSpan := source.span
  let loc ← addLoc sourceSpan
  match source with
  | .local name _ =>
      let some (localId, typeId) := lookupLocal? context name
        | failAt "LEANER-PLACE-LOCAL" s!"unknown local `{name}`" (some sourceSpan)
      let place ← pushPlace (.localVar localId)
      let kind ← match ← typeNode? typeId with
        | some (.reference ..) => pure (ExprKind.localVar localId)
        | _ => pure (.operation (.read place) #[] #[])
      return (place, typeId, #[],
        pushExpression loc typeId kind)
  | .dereference base _ =>
      let (base, baseType, bindings, observe) ← lowerExpressionPlaceWith
        lowerIndex context base checkBounds captureLiteralIndexes
      let some (.reference reference) ← typeNode? baseType
        | failAt "LEANER-PLACE-DEREF"
            "a dereferenced place must contain a reference" (some sourceSpan)
      return (← pushPlace (.deref base), reference.referent, bindings, do
        pushExpression loc reference.referent
          (.operation (.reference .dereference) #[] #[← observe]))
  | .field base field _ =>
      let (base, baseType, bindings, observe) ← lowerExpressionPlaceWith
        lowerIndex context base checkBounds captureLiteralIndexes
      let (base, baseType, observe) ← match ← typeNode? baseType with
        | some (.reference reference) =>
            pure (← pushPlace (.deref base), reference.referent, do
              pushExpression loc reference.referent
                (.operation (.reference .dereference) #[] #[← observe]))
        | _ => pure (base, baseType, observe)
      let (owner, fieldName, typeId) ← resolveNominalField baseType field sourceSpan
      return (← pushPlace (.field base owner fieldName), typeId, bindings, do
        pushExpression loc typeId (.operation (.data (.select owner field)) #[] #[← observe]))
  | .index base indexSource _ =>
      let (base, baseType, bindings, observe) ← lowerExpressionPlaceWith
        lowerIndex context base checkBounds captureLiteralIndexes
      let (base, baseType, observe) ← match ← typeNode? baseType with
        | some (.reference reference) =>
            pure (← pushPlace (.deref base), reference.referent, do
              pushExpression loc reference.referent
                (.operation (.reference .dereference) #[] #[← observe]))
        | _ => pure (base, baseType, observe)
      let typeId ← match ← typeNode? baseType with
        | some (.vector elementType _) => pure elementType
        | some (.tuple elements) =>
            let some position := literalIndex? indexSource
              | failAt "LEANER-INDEX-TYPE"
                  "tuple indexing requires a nonnegative literal" (some sourceSpan)
            let some elementType := elements[position]?
              | failAt "LEANER-INDEX-BOUNDS"
                  s!"tuple index {position} is out of bounds for {elements.size} elements"
                    (some sourceSpan)
            pure elementType
        | _ => do
            failAt "LEANER-INDEX-TYPE"
              "an indexed place requires a vector or tuple base" (some sourceSpan)
      let index ← lowerIndex indexSource
      let output := (← get).output
      -- Indexed assignments use the same typed local-index path for literal
      -- and computed vector indexes. Tuple projection stays a literal since
      -- its index selects a statically different element type.
      let captureLiteral := captureLiteralIndexes &&
        ((← typeNode? baseType).any fun | .vector .. => true | _ => false)
      let simple := match output.expressions[index.1.index]?.map (·.kind) with
        | some (.value ..) => !captureLiteral
        | some (.localVar ..) => true
        | _ => false
      let loc ← addLoc indexSource.span
      -- Sequence a computed index once, before checking or resolving the
      -- place. Failures in its computation must precede the bounds failure.
      let (indexId, bindings) ← if simple || context.specification then
          pure (index.1, bindings)
        else do
          let slot ← pushTemporaryLocal index.2 loc
          let pattern ← pushPattern { loc, typeId := index.2, kind := .variable slot }
          let read ← pushExpression loc index.2 (.localVar slot)
          pure (read, bindings.push (pattern, index.1))
      let bindings ← if checkBounds && !context.specification &&
          (← get).sourceNamespace.profile == .move &&
          ((← typeNode? baseType).any fun | .vector .. => true | _ => false) then do
          let unitType ← internType .unit
          -- This is a metadata observation, not a Copy of the elements.
          let collection ← observe
          let checked ← pushExpression loc unitType (.operation
            (.primitive (.checkVectorIndex ThrowKind.moveVectorError.toLIR)) #[]
            #[collection, indexId])
          let pattern ← pushPattern { loc, typeId := unitType, kind := .wildcard }
          pure (bindings.push (pattern, checked))
        else pure bindings
      let place ← pushPlace (.index base indexId)
      let observe := match ← typeNode? baseType with
        | some (.vector ..) => do
            pushExpression loc typeId
              (.operation (.primitive .index) #[] #[← observe, indexId])
        | _ => pushExpression loc typeId (.operation (.read place) #[] #[])
      return (place, typeId, bindings, observe)
  | .storageIndex head indexSource _ =>
      let some base := storageIndexLocalBase? context head
        | failAt "LEANER-PLACE-EXPRESSION"
            "a resource-storage index is not a local assignable place" (some sourceSpan)
      lowerExpressionPlaceWith lowerIndex context
        (.index base indexSource sourceSpan) checkBounds captureLiteralIndexes
  | _ => do
      failAt "LEANER-PLACE-EXPRESSION"
        "move, copy, read, borrow, and assignment require an assignable place"
          (some sourceSpan)

private def bindPlaceIndices (loc : LocId) (typeId : TypeId)
    (bindings : Array (PatternId × ExprId)) (body : ExprId) : LowerM ExprId :=
  bindings.foldrM (fun (pattern, value) body =>
    pushExpression loc typeId (.letDecl pattern (some value) body)) body

/-- Move evaluates the assignment value before constructing its destination.
Freeze a nonliteral value before effectful indexes and bounds checks; literals
need no temporary because moving them across those effects is unobservable. -/
private def pushIndexedAssignment (context : ExprContext) (loc : LocId)
    (unitType : TypeId) (place : PlaceId) (value : ExprId) (valueType : TypeId)
    (bindings : Array (PatternId × ExprId)) : LowerM ExprId := do
  let literal := ((← get).output.expressions[value.index]?).any fun expression =>
    match expression.kind with | .value .. => true | _ => false
  let expressions := (← get).output.expressions
  let observesDestination := bindings.any fun (_, initializer) =>
    !(expressions[initializer.index]?).any fun expression =>
      match expression.kind with | .value .. => true | _ => false
  if !context.specification && (← get).sourceNamespace.profile == .move &&
      observesDestination && !literal then
    let slot ← pushTemporaryLocal valueType loc
    let pattern ← pushPattern { loc, typeId := valueType, kind := .variable slot }
    let source ← pushPlace (.localVar slot)
    let read ← pushExpression loc valueType (.operation (.move source) #[] #[])
    let assignment ← pushExpression loc unitType (.assign place read)
    let body ← bindPlaceIndices loc unitType bindings assignment
    pushExpression loc unitType (.letDecl pattern (some value) body)
  else
    let assignment ← pushExpression loc unitType (.assign place value)
    bindPlaceIndices loc unitType bindings assignment

private def pushForIncrement (_iteratorPattern : PatternId) (iterator : LocalId)
    (iteratorType : TypeId) (loc : LocId) (specification : Bool) : LowerM ExprId := do
  let unitType ← internType .unit
  let iteratorRead ← pushExpression loc iteratorType (.localVar iterator)
  let one ← pushExpression loc iteratorType <| .value (.integer 1)
  let operation := profilePrimitiveLIR
    (← get).sourceNamespace.profile specification .profileAdd
  let value ← pushExpression loc iteratorType <|
    .operation (.primitive operation) #[] #[iteratorRead, one]
  let place ← pushPlace (.localVar iterator)
  pushExpression loc unitType <| .assign place value

/-- A surface range-loop `continue` must execute the implicit increment before
retesting the bound. Insert that increment at every current-loop continue and
stop at nested core loops, whose continues target the nested loop instead. The
source backend recognizes and hides this administrative block when recovering
canonical `for` syntax. -/
private partial def incrementForContinues (root : ExprId)
    (iteratorPattern : PatternId) (iterator : LocalId) (iteratorType : TypeId)
    (specification : Bool) (fuel : Nat) : LowerM ExprId := do
  if fuel == 0 then
    failAt "LEANER-FOR-CONTINUE-CYCLE"
      "a cyclic expression was reached while lowering `for` continues"
  let state ← get
  let some expression := state.output.expressions[root.index]?
    | failAt "LEANER-FOR-CONTINUE-ID"
        s!"a `for` body references missing expression {root.index}"
  let recur child := incrementForContinues child iteratorPattern iterator iteratorType
    specification (fuel - 1)
  let rebuild kind := pushExpression expression.loc expression.typeId kind
  match expression.kind with
  | .continue_ 0 => do
      let increment ← pushForIncrement iteratorPattern iterator iteratorType
        expression.loc specification
      rebuild (.block #[increment] (some root))
  | .loop .. => pure root
  | .operation operation instantiations arguments surface =>
      rebuild (.operation operation instantiations (← arguments.mapM recur) surface)
  | .block statements result =>
      rebuild (.block (← statements.mapM recur) (← result.mapM recur))
  | .letDecl pattern value body =>
      rebuild (.letDecl pattern (← value.mapM recur) (← recur body))
  | .ifElse condition thenBranch elseBranch =>
      rebuild (.ifElse (← recur condition) (← recur thenBranch) (← elseBranch.mapM recur))
  | .match_ scrutinee arms => do
      let arms ← arms.mapM fun arm => do
        pure { arm with guard := ← arm.guard.mapM recur, body := ← recur arm.body }
      rebuild (.match_ (← recur scrutinee) arms)
  | .break_ nest value => rebuild (.break_ nest (← value.mapM recur))
  | .return_ values => rebuild (.return_ (← values.mapM recur))
  | .throw_ kind arguments => rebuild (.throw_ kind (← arguments.mapM recur))
  | .assign place value => rebuild (.assign place (← recur value))
  | .assignPattern pattern value => rebuild (.assignPattern pattern (← recur value))
  | .value .. | .constant .. | .localVar .. | .continue_ .. |
      .quantifier .. | .spec .. => pure root

/-- Type of one of a function's results in its specification. The Move profile
has no tuple values, so a tuple-typed result is the packed multi-result row and
`result i` names its `i`-th component. -/
private def specificationResultType (context : ExprContext) (index : Nat)
    (span : Span) : LowerM TypeId := do
  let some resultType := context.resultType
    | failAt "LEANER-SPEC-RESULT" "`result` is unavailable in this specification" (some span)
  let packed := (← get).sourceNamespace.profile == .move
  match ← typeNode? resultType with
  | some (.tuple components) =>
      if packed then
        let some component := components[index]?
          | failAt "LEANER-SPEC-RESULT"
              s!"specification result {index} is out of range; the function has {components.size} results"
              (some span)
        pure component
      else if index == 0 then pure resultType
      else failAt "LEANER-SPEC-RESULT" s!"the function has one result, not {index + 1}" (some span)
  | _ =>
      if index == 0 then pure resultType
      else failAt "LEANER-SPEC-RESULT" s!"the function has one result, not {index + 1}" (some span)

/- A scalar call is logical in specifications even when its result is placed
inside a representation-bearing vector or nominal field. Preserve the call's
logical signature and explicitly inject its result, just as for spec locals.
Executable calls never acquire this specification-only conversion. -/
private def finishCall (context : ExprContext) (expected : Option TypeId)
    (id : ExprId) (actual : TypeId) (loc : LocId) (span : Span) :
    LowerM (ExprId × TypeId) := do
  if let some expected := expected then
    if context.specification && expected != actual then
      match ← typeNode? actual, ← typeNode? expected with
      | some (.integer .unbounded true), some (.integer (.bits _) _)
      | some (.integer .unbounded true), some (.integer .pointer _) =>
          let converted ← pushExpression loc expected <|
            .operation (.specification .intToBitVector) #[] #[id]
          return (converted, expected)
      | _, _ => pure ()
    ensureType expected actual span
  return (id, actual)

private partial def lowerExpr (context : ExprContext) (expected : Option TypeId)
    (source : Expr) : LowerM (ExprId × TypeId) := do
  let functionTail := context.functionTail
  let context := { context with functionTail := false }
  let span := source.span
  let loc ← addLoc span
  match source with
  | .unit _ =>
      let typeId ← internType .unit
      if let some expected := expected then ensureType expected typeId span
      return (← pushExpression loc typeId (.value .unit), typeId)
  | .bool value _ =>
      let typeId ← internType .bool
      if let some expected := expected then ensureType expected typeId span
      return (← pushExpression loc typeId (.value (.bool value)), typeId)
  | .char value _ =>
      let typeId ← internType .character
      if let some expected := expected then ensureType expected typeId span
      return (← pushExpression loc typeId (.value (.character value)), typeId)
  | .integer value _ =>
      let typeId ← match expected with
        | some expected => do
            match (← typeNode? expected) with
            | some (.integer _ _) => pure expected
            | _ => failAt "LEANER-INTEGER-CONTEXT" "an integer literal appears in a non-integer context" (some span)
        | none =>
            if context.specification then internType (.integer .unbounded true)
            else match (← get).sourceNamespace.profile with
              | .move => internType (.integer (.bits 64) false)
              | .rust => internType (.integer (.bits 32) true)
      return (← pushExpression loc typeId (.value (.integer value)), typeId)
  | .typedInteger value type _ =>
      let typeId := (← lowerTypeUse context.types type).typeId
      unless (← typeNode? typeId).any fun | .integer _ _ => true | _ => false do
        failAt "LEANER-INTEGER-TYPE" "an integer suffix must name a fixed integer type" (some type.span)
      let id ← pushExpression loc typeId (.value (.integer value))
      if let some expected := expected then
        if expected == typeId then return (id, typeId)
        let fixedInput := (← typeNode? typeId).any fun
          | .integer (.bits _) _ | .integer .pointer _ => true
          | _ => false
        let logicalExpected := (← typeNode? expected) ==
          some (.integer .unbounded true)
        if context.specification && fixedInput && logicalExpected then
          let converted ← pushExpression loc expected <|
            .operation (.specification .bitVectorToInt) #[] #[id]
          return (converted, expected)
        ensureType expected typeId span
      return (id, typeId)
  | .address value _ =>
      let typeId ← internType .address
      if let some expected := expected then ensureType expected typeId span
      return (← pushExpression loc typeId (.value (.address value)), typeId)
  | .string value _ =>
      let typeId ← internType .string
      if let some expected := expected then ensureType expected typeId span
      return (← pushExpression loc typeId (.value (.string value)), typeId)
  | .bytes value _ =>
      -- A Move byte-string literal in a `vector<u8>` context is the vector
      -- constant itself, matching the exchange encoding, so printed sugar
      -- round-trips to the identical value.
      if let some expectedType := expected then
        if let some (.vector element _) ← typeNode? expectedType then
          if (← typeNode? element) == some (.integer (.bits 8) false) then
            let elements := value.map fun byte =>
              ConstValue.integer (Int.ofNat byte.toNat)
            let id ← pushExpression loc expectedType (.value (.vector elements))
            return (id, expectedType)
      let typeId ← internType .bytes
      if let some expected := expected then ensureType expected typeId span
      return (← pushExpression loc typeId (.value (.bytes value)), typeId)
  | .local name _ =>
      if let some (_, alias) := context.patternAliases.find? (·.1 == name) then
        let some (_, typeId) := lookupLocal? context name
          | failAt "LEANER-MATCH-BINDER"
              s!"pattern alias `{name}` has no declared local" (some span)
        let aliasContext := { context with
          patternAliases := context.patternAliases.filter (·.1 != name) }
        let lowered ← lowerExpr aliasContext (some typeId) alias
        if let some expected := expected then ensureType expected lowered.2 span
        return lowered
      -- A local named `result` shadows the specification's implicit result,
      -- exactly as an ordinary binding shadows any other name.
      if context.specification && name == "result" && (lookupLocal? context name).isNone then
        let typeId ← specificationResultType context 0 span
        if let some expected := expected then ensureType expected typeId span
        let id ← pushExpression loc typeId <| .operation
          (.specification (.result 0)) #[] #[]
        return (id, typeId)
      let some (localId, typeId) := lookupLocal? context name | do
        let state ← get
        let some declaration := sourceConstant? state.sourceNamespace name
          | failAt "LEANER-LOCAL-NAME" s!"unknown local or constant `{name}`" (some span)
        let physicalType := (← lowerTypeUse context.types declaration.type).typeId
        let some constantName := nameId? state.tables state.namespaceId name
          | failAt "LEANER-CONSTANT-NAME" s!"constant `{name}` was not interned" (some span)
        let constant ← pushExpression loc physicalType <|
          .constant { namespaceId := state.namespaceId, name := constantName }
        let value ← if context.specification then do
            let projectedType ← projectSpecTypeId physicalType
            if projectedType == physicalType then pure (constant, physicalType)
            else match ← typeNode? physicalType, ← typeNode? projectedType with
              | some (.integer _ _), some (.integer .unbounded true) => do
                  let projected ← pushExpression loc projectedType <|
                    .operation (.specification .bitVectorToInt) #[] #[constant]
                  pure (projected, projectedType)
              | _, _ =>
                  failAt "LEANER-CONSTANT-PROJECTION"
                    s!"constant `{name}` has no specification projection" (some span)
          else pure (constant, physicalType)
        if let some expected := expected then ensureType expected value.2 span
        return value
      let moveTypeParameter := !context.specification &&
        match ← typeNode? typeId with
        | some (.typeParameter index) =>
            !context.generics[index]?.any (·.abilities.contains .copy)
        | _ => false
      let localExpr ← if moveTypeParameter then do
          let place ← pushPlace (.localVar localId)
          pushExpression loc typeId <| .operation (.move place) #[] #[]
        else
          pushExpression loc typeId (.localVar localId)
      if let some expected := expected then
        if expected == typeId then return (localExpr, typeId)
        match ← typeNode? expected, ← typeNode? typeId with
        | some (.reference expectedReference), some (.reference actualReference) =>
            if expectedReference.profile == actualReference.profile &&
                expectedReference.kind == .shared && actualReference.kind == .mutable &&
                expectedReference.referent == actualReference.referent then
              let frozen ← pushExpression loc expected <|
                .operation (.reference (.freeze false)) #[] #[localExpr]
              return (frozen, expected)
        | _, _ => pure ()
        let fixedInput := (← typeNode? typeId).any fun
          | .integer (.bits _) _ | .integer .pointer _ => true
          | _ => false
        let logicalExpected := (← typeNode? expected) ==
          some (.integer .unbounded true)
        if context.specification && fixedInput && logicalExpected then
          let converted ← pushExpression loc expected <|
            .operation (.specification .bitVectorToInt) #[] #[localExpr]
          return (converted, expected)
        let logicalInput := (← typeNode? typeId) == some (.integer .unbounded true)
        let fixedResult := (← typeNode? expected).any fun
          | .integer (.bits _) _ | .integer .pointer _ => true
          | _ => false
        if context.specification && logicalInput && fixedResult then
          let converted ← pushExpression loc expected <|
            .operation (.specification .intToBitVector) #[] #[localExpr]
          return (converted, expected)
        ensureType expected typeId span
      return (localExpr, typeId)
  | .typedPrimitive operation result arguments _ =>
      if operation == .vector then
        let physicalResult := (← lowerTypeUse context.types result).typeId
        let resultType ← if context.specification then projectSpecTypeId physicalResult
          else pure physicalResult
        let some (.vector elementType none) ← typeNode? resultType
          | failAt "LEANER-TYPED-VECTOR"
              "a typed vector literal requires an unfixed vector result type" (some span)
        let lowered ← arguments.mapM (lowerExpr context (some elementType))
        if let some expected := expected then ensureType expected resultType span
        let id ← pushExpression loc resultType <| .operation
          (.primitive .vector) #[] (lowered.map (·.1))
        return (id, resultType)
      unless operation == .cast || operation == .profileCast ||
          (match operation with | .checkedCast _ => true | _ => false) do
        failAt "LEANER-TYPED-PRIMITIVE"
          "only integer casts have an explicit primitive result type" (some span)
      unless arguments.size == 1 do
        failAt "LEANER-PRIMITIVE-ARITY"
          s!"a typed cast expects one operand, got {arguments.size}" (some span)
      let resultType := (← lowerTypeUse context.types result).typeId
      let value ← lowerExpr context none arguments[0]!
      let id ← pushExpression loc resultType <| .operation
        (.primitive (profilePrimitiveLIR (← get).sourceNamespace.profile
          context.specification operation)) #[] #[value.1]
      if let some expected := expected then
        if expected == resultType then return (id, resultType)
        let fixedResult := (← typeNode? resultType).any fun
          | .integer (.bits _) _ | .integer .pointer _ => true
          | _ => false
        let logicalExpected := (← typeNode? expected) == some (.integer .unbounded true)
        if context.specification && fixedResult && logicalExpected then
          let converted ← pushExpression loc expected <|
            .operation (.specification .bitVectorToInt) #[] #[id]
          return (converted, expected)
        ensureType expected resultType span
      return (id, resultType)
  | .primitive operation arguments _ =>
      let arity := primitiveArity operation
      let lirOperation := profilePrimitiveLIR (← get).sourceNamespace.profile
        context.specification operation
      if operation != .tuple && operation != .vector && arguments.size != arity then
        failAt "LEANER-PRIMITIVE-ARITY"
          s!"primitive `{repr operation}` expects {arity} argument(s), got {arguments.size}" (some span)
      if !context.specification && (operation == .logicalAnd || operation == .logicalOr) then
        let boolType ← internType .bool
        if let some expected := expected then ensureType expected boolType span
        let left ← lowerExpr context (some boolType) arguments[0]!
        let right ← lowerExpr context (some boolType) arguments[1]!
        let constant ← pushExpression loc boolType (.value (.bool (operation == .logicalOr)))
        let branch := if operation == .logicalAnd then
            ExprKind.ifElse left.1 right.1 (some constant)
          else .ifElse left.1 constant (some right.1)
        return (← pushExpression loc boolType branch, boolType)
      if operation == .tuple then
        let expectedElements : Option (Array TypeId) ← match expected with
          | some expected => match ← typeNode? expected with
            | some (.tuple elements) => do
                if elements.size == arguments.size then
                  pure (some elements)
                else
                  failAt "LEANER-TUPLE-ARITY"
                    s!"a tuple context expects {elements.size} element(s), got {arguments.size}"
                      (some span)
            | _ => pure none
          | none => pure none
        let lowered ← arguments.zipIdx.mapM fun (argument, index) =>
          lowerExpr context (expectedElements.bind (·[index]?)) argument
        let typeId ← match expectedElements, expected with
          | some _, some expected => pure expected
          | _, _ => internType (.tuple (lowered.map (·.2)))
        if let some expected := expected then ensureType expected typeId span
        let id ← pushExpression loc typeId <| .operation
          (.primitive .tuple) #[] (lowered.map (·.1))
        return (id, typeId)
      if operation == .vector then
        let expectedElement ← match expected with
          | some expected => match ← typeNode? expected with
            | some (.vector element _) => pure (some element)
            | _ => failAt "LEANER-VECTOR-CONTEXT" "a vector literal has a non-vector expected type" (some span)
          | none => pure none
        let mut lowered : Array (ExprId × TypeId) := #[]
        let mut elementType := expectedElement
        for argument in arguments do
          let value ← lowerExpr context elementType argument
          if let some expected := elementType then ensureType expected value.2 argument.span
          else elementType := some value.2
          lowered := lowered.push value
        let resolvedElementType ← match elementType with
          | some type => pure type
          | none => failAt "LEANER-VECTOR-EMPTY" "an empty vector needs an expected vector type" (some span)
        let typeId ← match expected with
          | some expected => pure expected
          | none => internType (.vector resolvedElementType none)
        let id ← pushExpression loc typeId <| .operation
          (.primitive .vector) #[] (lowered.map (·.1))
        return (id, typeId)
      if let .repeatVector length := operation then
        let expectedElement ← repeatVectorExpectedElement expected length span
        let value ← lowerExpr context expectedElement arguments[0]!
        let typeId ← match expected with
          | some expected => pure expected
          | none => internType (.vector value.2 (some (.integer (Int.ofNat length))))
        let id ← pushExpression loc typeId <| .operation
          (.primitive .repeatVector) #[] #[value.1]
        return (id, typeId)
      if operation == .length then
        let value ← lowerExpr context none arguments[0]!
        match ← typeNode? value.2 with
        | some (.vector ..) | some .string | some .bytes => pure ()
        | _ => failAt "LEANER-LENGTH-TYPE" "`length` needs a vector, string, or bytes" (some span)
        let typeId ← if context.specification then internType (.integer .unbounded true)
          else runtimeIndexType
        if let some expected := expected then ensureType expected typeId span
        let id ← pushExpression loc typeId <| .operation
          (.primitive .length) #[] #[value.1]
        return (id, typeId)
      if operation == .swapVector || operation == .reverseSliceVector then
        -- The operands are not homogeneous: the vector keeps the result type
        -- and the two indexes are runtime indexes.
        let collection ← lowerExpr context expected arguments[0]!
        let some (.vector ..) ← typeNode? collection.2
          | failAt "LEANER-VECTOR-ACCESS-TYPE"
              "`swapVector` requires a vector operand" (some span)
        let indexType ← if context.specification then
            internType (.integer .unbounded true)
          else runtimeIndexType
        let indexes ← arguments.extract 1 arguments.size |>.mapM
          (lowerExpr context (some indexType))
        if let some expected := expected then ensureType expected collection.2 span
        let id ← pushExpression loc collection.2 <| .operation
          (.primitive lirOperation) #[] (#[collection.1] ++ indexes.map (·.1))
        return (id, collection.2)
      if operation == .insertVector || operation == .removeVector then
        let collection ← lowerExpr context (if operation == .insertVector then expected else none)
          arguments[0]!
        let some (.vector elementType _) ← typeNode? collection.2
          | failAt "LEANER-VECTOR-ACCESS-TYPE"
              "vector insertion/removal requires a vector operand" (some span)
        let indexType ← if context.specification then internType (.integer .unbounded true)
          else runtimeIndexType
        let index ← lowerExpr context (some indexType) arguments[1]!
        let (typeId, operands) ← if operation == .insertVector then do
            let value ← lowerExpr context (some elementType) arguments[2]!
            ensureType elementType value.2 arguments[2]!.span
            pure (collection.2, #[collection.1, index.1, value.1])
          else do
            let removedType ← if context.specification then projectSpecTypeId elementType
              else pure elementType
            let result ← internType (.tuple #[removedType, collection.2])
            pure (result, #[collection.1, index.1])
        if let some expected := expected then ensureType expected typeId span
        let id ← pushExpression loc typeId <| .operation (.primitive lirOperation) #[] operands
        return (id, typeId)
      if let .checkVectorIndex failure := operation then
        let collection ← lowerExpr context none arguments[0]!
        let some (.vector ..) ← typeNode? collection.2
          | failAt "LEANER-VECTOR-ACCESS-TYPE" "index check requires a vector" (some span)
        let indexType ← runtimeIndexType
        let index ← lowerExpr context (some indexType) arguments[1]!
        let resultType ← internType .unit
        if let some expected := expected then ensureType expected resultType span
        let id ← pushExpression loc resultType <| .operation
          (.primitive (.checkVectorIndex failure.toLIR)) #[] #[collection.1, index.1]
        return (id, resultType)
      if operation == .destroyEmptyVector || operation == .containsVector ||
          operation == .indexOfVector then
        let collection ← lowerExpr context none arguments[0]!
        let some (.vector elementType _) ← typeNode? collection.2
          | failAt "LEANER-VECTOR-ACCESS-TYPE" "operation requires a vector operand" (some span)
        let (resultType, operands) ← if operation == .destroyEmptyVector then
            pure (← internType .unit, #[collection.1])
          else do
            let needleType ← if context.specification then projectSpecTypeId elementType
              else pure elementType
            let needle ← lowerExpr context (some needleType) arguments[1]!
            let boolType ← internType .bool
            let resultType ← if operation == .containsVector then pure boolType
              else do
                let indexType ← if context.specification then
                    internType (.integer .unbounded true) else runtimeIndexType
                internType (.tuple #[boolType, indexType])
            pure (resultType, #[collection.1, needle.1])
        if let some expected := expected then ensureType expected resultType span
        let id ← pushExpression loc resultType <|
          .operation (.primitive lirOperation) #[] operands
        return (id, resultType)
      if operation == .concatVector then
        let left ← lowerExpr context expected arguments[0]!
        let some (.vector _ none) ← typeNode? left.2
          | failAt "LEANER-VECTOR-ACCESS-TYPE"
              "`concatVector` requires variable-length vector operands" (some span)
        let right ← lowerExpr context (some left.2) arguments[1]!
        ensureType left.2 right.2 arguments[1]!.span
        if let some expected := expected then ensureType expected left.2 span
        let id ← pushExpression loc left.2 <| .operation
          (.primitive .concatVector) #[] #[left.1, right.1]
        return (id, left.2)
      if operation == .pushVector then
        -- The operands are not homogeneous: the vector keeps the result type
        -- and the pushed value carries its element type.
        let collection ← lowerExpr context expected arguments[0]!
        let some (.vector elementType _) ← typeNode? collection.2
          | failAt "LEANER-VECTOR-ACCESS-TYPE"
              "`pushVector` requires a vector operand" (some span)
        let value ← lowerExpr context (some elementType) arguments[1]!
        ensureType elementType value.2 arguments[1]!.span
        if let some expected := expected then ensureType expected collection.2 span
        let id ← pushExpression loc collection.2 <| .operation
          (.primitive .pushVector) #[] #[collection.1, value.1]
        return (id, collection.2)
      if operation == .index || operation == .slice then
        let collection ← lowerExpr context none arguments[0]!
        let some (.vector elementType _) ← typeNode? collection.2
          | failAt "LEANER-VECTOR-ACCESS-TYPE"
              "vector indexing and slicing require a vector operand" (some span)
        let indexType ← if context.specification then
            internType (.integer .unbounded true)
          else runtimeIndexType
        let bounds ← arguments.extract 1 arguments.size |>.mapM
          (lowerExpr context (some indexType))
        let typeId := if operation == .index then elementType else collection.2
        if let some expected := expected then ensureType expected typeId span
        let id ← pushExpression loc typeId <| .operation
          (.primitive lirOperation) #[]
            (#[collection.1] ++ bounds.map (·.1))
        return (id, typeId)
      if operation == .range then
        -- Infer from the non-literal bound when exactly one side is a
        -- literal. Move specifications retain fixed-width parameter types,
        -- so `range(0, i)` must type `0` from `i` rather than defaulting it
        -- to logical `Int`.
        let (lower, upper) ← match arguments[0]!, arguments[1]! with
          | .integer .., upperSource@(.integer ..) => do
              let lower ← lowerExpr context none arguments[0]!
              let upper ← lowerExpr context (some lower.2) upperSource
              pure (lower, upper)
          | .integer .., upperSource => do
              let upper ← lowerExpr context none upperSource
              let lower ← lowerExpr context (some upper.2) arguments[0]!
              pure (lower, upper)
          | lowerSource, upperSource => do
              let lower ← lowerExpr context none lowerSource
              let upper ← lowerExpr context (some lower.2) upperSource
              pure (lower, upper)
        ensureType lower.2 upper.2 span
        let typeId ← internType .range
        if let some expected := expected then ensureType expected typeId span
        let id ← pushExpression loc typeId <| .operation
          (.primitive .range) #[] #[lower.1, upper.1]
        return (id, typeId)
      if operation == .overflowingAdd || operation == .overflowingSubtract ||
          operation == .overflowingMultiply then
        let (resultType, lowered) ← match expected with
          | some resultType => do
              let some (.tuple resultElements) ← typeNode? resultType
                | failAt "LEANER-OVERFLOWING-TYPE"
                    "overflow-reporting arithmetic requires a tuple result" (some span)
              unless resultElements.size == 2 do
                failAt "LEANER-OVERFLOWING-TYPE"
                  "overflow-reporting arithmetic returns exactly two values" (some span)
              let valueType := resultElements[0]!
              unless (← typeNode? resultElements[1]!) == some .bool do
                failAt "LEANER-OVERFLOWING-TYPE"
                  "the overflow-reporting flag must be Boolean" (some span)
              pure (resultType, ← arguments.mapM (lowerExpr context (some valueType)))
          | none => do
              let first ← lowerExpr context none arguments[0]!
              unless (← typeNode? first.2).any fun | .integer _ _ => true | _ => false do
                failAt "LEANER-OVERFLOWING-TYPE"
                  "overflow-reporting arithmetic requires integer operands" (some span)
              let mut lowered := #[first]
              for argument in arguments.extract 1 arguments.size do
                lowered := lowered.push (← lowerExpr context (some first.2) argument)
              let boolType ← internType .bool
              let resultType ← internType (.tuple #[first.2, boolType])
              pure (resultType, lowered)
        let id ← pushExpression loc resultType <| .operation
          (.primitive lirOperation) #[] (lowered.map (·.1))
        return (id, resultType)
      if operation == .copyValue || operation == .moveValue then
        let some argument := arguments[0]?
          | failAt "LEANER-PRIMITIVE-ARITY" "a value copy or move expects one operand" (some span)
        let value ← lowerExpr context expected argument
        let id ← pushExpression loc value.2 <|
          .operation (.primitive lirOperation) #[] #[value.1]
        return (id, value.2)
      if operation == .cast || operation matches .checkedCast _ then
        let resultType ← requireExpected expected span "an integer cast"
        unless (← typeNode? resultType).any fun | .integer _ _ => true | _ => false do
          failAt "LEANER-CAST-TYPE" "an integer cast requires an integer result type" (some span)
        let operand ← lowerExpr context (if context.specification then some resultType else none)
          arguments[0]!
        unless (← typeNode? operand.2).any fun | .integer _ _ => true | _ => false do
          failAt "LEANER-CAST-TYPE" "an integer cast requires an integer operand" (some span)
        let id ← pushExpression loc resultType <| .operation
          (.primitive lirOperation) #[] #[operand.1]
        return (id, resultType)
      let boolType ← internType .bool
      let resultExpected := if isBooleanPrimitive operation then some boolType else expected
      let firstExpected ←
        if operation == .logicalAnd || operation == .logicalOr ||
            operation == .eagerLogicalAnd || operation == .eagerLogicalOr || operation == .logicalNot ||
            operation == .implies || operation == .equivalent then pure (some boolType)
        else if isBooleanPrimitive operation then pure none
        else pure expected
      let first ← lowerExpr context firstExpected arguments[0]!
      let argumentExpected :=
        if operation == .shiftLeft || operation == .profileShiftLeft ||
            operation matches .checkedShiftLeft _ ||
            operation == .shiftRight || operation == .profileShiftRight ||
            operation matches .checkedShiftRight _ then none
        else if isBooleanPrimitive operation &&
            !(operation == .logicalAnd || operation == .logicalOr ||
              operation == .eagerLogicalAnd || operation == .eagerLogicalOr || operation == .logicalNot ||
              operation == .implies || operation == .equivalent) then some first.2
        else firstExpected.orElse fun _ => some first.2
      let mut lowered := #[first]
      for argument in arguments.extract 1 arguments.size do
        lowered := lowered.push (← lowerExpr context argumentExpected argument)
      let typeId ← match resultExpected with
        | some type => pure type
        | none => pure first.2
      if let some expected := expected then ensureType expected typeId span
      let id ← pushExpression loc typeId <| .operation
        (.primitive lirOperation) #[] (lowered.map (·.1))
      return (id, typeId)
  | .construct segments arguments _ =>
      let (reference, declaration, variant) ← resolveLocalConstructor segments
      let resultType ← match expected with
        | some resultType => pure resultType
        | none => do
            unless declaration.generics.isEmpty do
              failAt "LEANER-TYPE-INFERENCE"
                "cannot infer generic arguments for a nominal constructor" (some span)
            internType (.nominal reference.name #[])
      let (owner, instantiations) ← match ← typeNode? resultType with
        | some (.nominal owner instantiations) => pure (owner, instantiations)
        | _ => do
            failAt "LEANER-CONSTRUCTOR-CONTEXT"
              "a nominal constructor requires a nominal expected type" (some span)
      unless owner == reference.name do
        failAt "LEANER-CONSTRUCTOR-CONTEXT"
          s!"constructor `{declaration.name}` does not match its expected nominal type" (some span)
      unless declaration.generics.size == instantiations.size &&
          declaration.generics.all (·.kind == .type) &&
          instantiations.all fun | .typeArg _ => true | _ => false do
        failAt "LEANER-CONSTRUCTOR-GENERICS"
          "constructor generic arguments do not match its type binders" (some span)
      let some fields := constructorFields? declaration variant
        | failAt "LEANER-CONSTRUCTOR-SHAPE"
            s!"constructor `{declaration.name}` does not select a valid shape" (some span)
      if arguments.size != fields.size then
        failAt "LEANER-CONSTRUCTOR-ARITY"
          s!"constructor expects {fields.size} field(s), got {arguments.size}" (some span)
      let declarationTypes ← liftM <| typeContext declaration.generics declaration.owner
      let fieldTypes ← fields.mapM fun field => do
        let declared := (← lowerTypeUse declarationTypes field.type).typeId
        let instantiated ← instantiateTypeId instantiations declared span
        if context.specification then projectSpecTypeId instantiated else pure instantiated
      let lowered ← arguments.zip fieldTypes |>.mapM fun (argument, type) =>
        lowerExpr context (some type) argument
      let id ← pushExpression loc resultType <| .operation
        (.call (.constructor reference variant)) instantiations (lowered.map (·.1))
      return (id, resultType)
  | .appliedConstruct ownerType variant arguments _ =>
      let .named ownerSegments _ := ownerType.value
        | failAt "LEANER-CONSTRUCTOR-TYPE"
            "an applied constructor requires a nominal owner type" (some ownerType.span)
      let constructorSegments := variant.map ownerSegments.push |>.getD ownerSegments
      let (reference, declaration, resolvedVariant) ←
        resolveLocalConstructor constructorSegments
      -- The nominal type parser greedily accepts `Type::Variant`; when the
      -- optional constructor suffix therefore has no separate syntax node,
      -- `resolveLocalConstructor` still identifies the final segment as the
      -- variant.  Remove it from the owner type before interning the result.
      let resultOwner := if variant.isNone && resolvedVariant.isSome then
          match ownerType.value with
          | .named segments arguments =>
              { ownerType with value := .named segments.pop arguments }
          | _ => ownerType
        else ownerType
      let resultType := (← lowerTypeUse context.types resultOwner).typeId
      if let some expected := expected then ensureType expected resultType span
      let (owner, instantiations) ← match ← typeNode? resultType with
        | some (.nominal owner instantiations) => pure (owner, instantiations)
        | _ => do
            failAt "LEANER-CONSTRUCTOR-TYPE"
              "an applied constructor did not lower to a nominal type" (some ownerType.span)
      unless owner == reference.name do
        failAt "LEANER-CONSTRUCTOR-CONTEXT"
          s!"constructor `{declaration.name}` does not match its named owner type" (some span)
      unless declaration.generics.size == instantiations.size &&
          declaration.generics.all (·.kind == .type) &&
          instantiations.all fun | .typeArg _ => true | _ => false do
        failAt "LEANER-CONSTRUCTOR-GENERICS"
          "constructor generic arguments do not match its type binders" (some span)
      let some fields := constructorFields? declaration resolvedVariant
        | failAt "LEANER-CONSTRUCTOR-SHAPE"
            s!"constructor `{declaration.name}` does not select a valid shape" (some span)
      if arguments.size != fields.size then
        failAt "LEANER-CONSTRUCTOR-ARITY"
          s!"constructor expects {fields.size} field(s), got {arguments.size}" (some span)
      let declarationTypes ← liftM <| typeContext declaration.generics declaration.owner
      let fieldTypes ← fields.mapM fun field => do
        let declared := (← lowerTypeUse declarationTypes field.type).typeId
        let instantiated ← instantiateTypeId instantiations declared span
        if context.specification then projectSpecTypeId instantiated else pure instantiated
      let lowered ← arguments.zip fieldTypes |>.mapM fun (argument, type) =>
        lowerExpr context (some type) argument
      let id ← pushExpression loc resultType <| .operation
        (.call (.constructor reference resolvedVariant)) instantiations (lowered.map (·.1))
      return (id, resultType)
  | .namedConstruct ownerType variant fields _ =>
      let .named ownerSegments _ := ownerType.value
        | failAt "LEANER-CONSTRUCTOR-TYPE"
            "a named constructor requires a nominal owner type" (some ownerType.span)
      let constructorSegments := variant.map ownerSegments.push |>.getD ownerSegments
      let (reference, declaration, resolvedVariant) ←
        resolveLocalConstructor constructorSegments
      let resultOwner := if variant.isNone && resolvedVariant.isSome then
          match ownerType.value with
          | .named segments arguments =>
              { ownerType with value := .named segments.pop arguments }
          | _ => ownerType
        else ownerType
      let resultType := (← lowerTypeUse context.types resultOwner).typeId
      if let some expected := expected then ensureType expected resultType span
      let (owner, instantiations) ← match ← typeNode? resultType with
        | some (.nominal owner instantiations) => pure (owner, instantiations)
        | _ => do
            failAt "LEANER-CONSTRUCTOR-TYPE"
              "a named constructor did not lower to a nominal type" (some ownerType.span)
      unless owner == reference.name do
        failAt "LEANER-CONSTRUCTOR-CONTEXT"
          s!"constructor `{declaration.name}` does not match its named owner type" (some span)
      unless declaration.generics.size == instantiations.size &&
          declaration.generics.all (·.kind == .type) &&
          instantiations.all fun | .typeArg _ => true | _ => false do
        failAt "LEANER-CONSTRUCTOR-GENERICS"
          "constructor generic arguments do not match its type binders" (some span)
      let some declaredFields := constructorFields? declaration resolvedVariant
        | failAt "LEANER-CONSTRUCTOR-SHAPE"
            s!"constructor `{declaration.name}` does not select a valid shape" (some span)
      unless fields.size == declaredFields.size do
        if declaration.external then
          failAt "LEANER-CONSTRUCTOR-FIELD"
            s!"constructor `{declaration.name}` carries fields whose order belongs to the \
              declaring unit; a dependency's nominal can only be constructed without fields \
              until dependency interfaces carry declarations" (some span)
        failAt "LEANER-CONSTRUCTOR-ARITY"
          s!"constructor expects {declaredFields.size} field(s), got {fields.size}" (some span)
      let declarationTypes ← liftM <| typeContext declaration.generics declaration.owner
      let mut lowered := #[]
      for declaredField in declaredFields do
        let provided := fields.filter (·.1 == declaredField.name)
        unless provided.size == 1 do
          failAt "LEANER-CONSTRUCTOR-FIELD"
            s!"constructor field `{declaredField.name}` must occur exactly once" (some span)
        let declared := (← lowerTypeUse declarationTypes declaredField.type).typeId
        let instantiated ← instantiateTypeId instantiations declared span
        let fieldType ← if context.specification then projectSpecTypeId instantiated
          else pure instantiated
        lowered := lowered.push (← lowerExpr context (some fieldType) provided[0]!.2).1
      let id ← pushExpression loc resultType <| .operation
        (.call (.constructor reference resolvedVariant)) instantiations lowered
      return (id, resultType)
  | .select ownerType field value _ =>
      let .named ownerSegments _ := ownerType.value
        | failAt "LEANER-SELECT-TYPE"
            "field selection requires a nominal owner type" (some ownerType.span)
      let (reference, declaration) ← resolveLocalNominal ownerSegments
      if declaration.external then
        failAt "LEANER-NOMINAL-EXTERNAL"
          s!"`{declaration.name}` is declared by another compilation unit, whose fields and \
            variants are not part of this one; only its identity and payload-free constructors \
            are available until dependency interfaces carry declarations" (some span)
      let ownerTypeId := (← lowerTypeUse context.types ownerType).typeId
      let instantiations ← match ← typeNode? ownerTypeId with
        | some (.nominal owner instantiations) => do
            unless owner == reference.name do
              failAt "LEANER-SELECT-TYPE"
                "field-selection owner resolves to a different nominal declaration"
                  (some ownerType.span)
            pure instantiations
        | _ => do
            failAt "LEANER-SELECT-TYPE"
              "field-selection owner did not lower to a nominal type" (some ownerType.span)
      let fields := fieldsNamed declaration field
      if fields.isEmpty then
        failAt "LEANER-SELECT-FIELD"
          s!"nominal type `{declaration.name}` has no field `{field}`" (some span)
      let declarationTypes ← liftM <| typeContext declaration.generics declaration.owner
      let fieldTypes ← fields.mapM fun sourceField => do
        let declared := (← lowerTypeUse declarationTypes sourceField.type).typeId
        let instantiated ← instantiateTypeId instantiations declared span
        if context.specification then projectSpecTypeId instantiated else pure instantiated
      let resultType := fieldTypes[0]!
      unless fieldTypes.all (· == resultType) do
        failAt "LEANER-SELECT-FIELD"
          s!"field `{field}` has different types across enum variants" (some span)
      let value ← lowerExpr context none value
      let resultType ← match ← typeNode? value.2 with
        | some (.reference sourceReference) => do
            unless sourceReference.referent == ownerTypeId && declaration.variants.isEmpty do
              failAt "LEANER-SELECT-TYPE"
                "referenced field selection requires a reference to a struct owner" (some span)
            internType (.reference { sourceReference with referent := resultType })
        | _ => do
            ensureType ownerTypeId value.2 span
            pure resultType
      if let some expected := expected then ensureType expected resultType span
      let id ← pushExpression loc resultType <| .operation
        (.data (.select reference field)) #[] #[value.1]
      return (id, resultType)
  | .field source field _ =>
      -- Rust source leaves scalar `Copy` loads implicit. Preserve their place
      -- projection in LIR so selecting a scalar field does not become a
      -- non-consuming read of the whole (possibly non-`Copy`) owner.
      if !context.specification && (← get).sourceNamespace.profile == .rust &&
          field != "length" then
        if let some sourcePlace := expressionPlace? (.field source field span) then
          let (place, placeType) ← lowerPlace context sourcePlace
          let intrinsicCopy := match ← typeNode? placeType with
            | some .unit | some .bool | some .character | some .string | some .bytes |
                some .address | some (.integer ..) => true
            | _ => false
          if intrinsicCopy then
            if let some expected := expected then ensureType expected placeType span
            let id ← pushExpression loc placeType <| .operation (.copy place) #[] #[]
            return (id, placeType)
      let value ← lowerExpr context none source
      let valueType ← typeNode? value.2
      let vectorElement? ← match valueType with
        | some (.vector element _) => pure (some element)
        | some (.reference reference) => match ← typeNode? reference.referent with
            | some (.vector element _) => pure (some element)
            | _ => pure none
        | _ => pure none
      if field == "length" then
        if let some elementType := vectorElement? then
          let value ← if context.specification then pure value else
            match valueType with
            -- The length primitive takes its vector by value in either
            -- profile, so a reference receiver is read through and a value
            -- receiver is already what it needs.
            | some (.reference reference) => do
                let id ← pushExpression loc reference.referent <| .operation
                  (.reference .dereference) #[] #[value.1]
                pure (id, reference.referent)
            | _ => pure value
          let instantiations := #[GenericArgument.typeArg { typeId := elementType, loc }]
          let (operation, resultType) ← if context.specification then do
              pure (Operation.specification .lengthVector,
                ← internType (.integer .unbounded true))
            else match (← get).sourceNamespace.profile with
              -- Move's `vector::length` is a `native fun` with no body: it is
              -- an instruction, and LIR owns that meaning as a primitive. The
              -- Move frontend lowers the native the same way, so the two
              -- paths into LIR agree and canonical source round trips.
              | .move =>
                  pure (Operation.primitive .length,
                    ← internType (.integer (.bits 64) false))
              | .rust =>
                  pure (Operation.primitive .length,
                    ← internType (.integer .pointer false))
          if let some expected := expected then ensureType expected resultType span
          let id ← pushExpression loc resultType <|
            .operation operation instantiations #[value.1]
          return (id, resultType)
        let primitiveLengthType? ← match valueType with
          | some .string | some .bytes => pure (some value.2)
          | some (.reference reference) => match ← typeNode? reference.referent with
              | some .string | some .bytes => pure (some reference.referent)
              | _ => pure none
          | _ => pure none
        if let some collectionType := primitiveLengthType? then
          let value ← match valueType with
            | some (.reference _) => do
                let id ← pushExpression loc collectionType <| .operation
                  (.reference .dereference) #[] #[value.1]
                pure (id, collectionType)
            | _ => pure value
          let resultType ← if context.specification then internType (.integer .unbounded true)
            else runtimeIndexType
          if let some expected := expected then ensureType expected resultType span
          let id ← pushExpression loc resultType <| .operation
            (.primitive .length) #[] #[value.1]
          return (id, resultType)
      let (baseType, sourceReference) ← match ← typeNode? value.2 with
        | some (.reference reference) => pure (reference.referent, some reference)
        | _ => pure (value.2, none)
      let (reference, _, fieldType) ← resolveNominalField baseType field span
      match sourceReference with
      | none =>
          let resultType ← if context.specification then projectSpecTypeId fieldType
            else pure fieldType
          if let some expected := expected then ensureType expected resultType span
          -- An enum's field lives in each variant that shares it, which
          -- `selectVariants` records and `select` does not; the operand being
          -- a value rather than a reference does not change that.
          let selection : LeanerIR.DataOperation ←
            if ← nominalHasVariants baseType then pure (.selectVariants reference #[field])
            else pure (.select reference field)
          let id ← pushExpression loc resultType <| .operation
            (.data selection) #[] #[value.1]
          return (id, resultType)
      | some sourceReference =>
          let selectedType ← internType
            (.reference { sourceReference with referent := fieldType })
          -- A field read through a reference names variants when the nominal
          -- type has them: an enum's field lives in each variant that shares
          -- it, which `selectVariants` records and `select` does not.
          let selection : LeanerIR.DataOperation ←
            if ← nominalHasVariants baseType then pure (.selectVariants reference #[field])
            else pure (.select reference field)
          let selected ← pushExpression loc selectedType <| .operation
            (.data selection) #[] #[value.1]
          let expectedIsReference ← match expected with
            | some expected => pure ((← typeNode? expected).any fun
                | .reference _ => true
                | _ => false)
            | none => pure false
          if expectedIsReference then
            if let some expected := expected then ensureType expected selectedType span
            return (selected, selectedType)
          let resultType ← if context.specification then projectSpecTypeId fieldType
            else pure fieldType
          if let some expected := expected then ensureType expected resultType span
          let id ← pushExpression loc resultType <| .operation
            (.reference .dereference) #[] #[selected]
          return (id, resultType)
  | .storageIndex head indexSource _ =>
      if let some localBase := storageIndexLocalBase? context head then
        return ← lowerExpr context expected
          (.index localBase indexSource span)
      unless (← get).sourceNamespace.profile == .move do
        failAt "LEANER-STORAGE-PROFILE"
          "resource index notation is available only in the Move profile" (some span)
      let resource ← lowerTypeUse context.types head
      let key ← lowerExpr context none indexSource
      if context.specification then
        let resultType ← projectSpecTypeId resource.typeId
        if let some expected := expected then ensureType expected resultType span
        let id ← pushExpression loc resultType <| .operation
          (.specification (.global none)) #[.typeArg resource] #[key.1]
        return (id, resultType)
      let referenceType ← inferredReferenceType .shared resource.typeId loc
      let borrowed ← pushExpression loc referenceType <| .operation
        (.global (.borrow .immutable)) #[.typeArg resource] #[key.1]
      if let some expected := expected then ensureType expected resource.typeId span
      let id ← pushExpression loc resource.typeId <| .operation
        (.reference .dereference) #[] #[borrowed]
      return (id, resource.typeId)
  | .index valueSource indexSource _ =>
      if (← get).sourceNamespace.profile == .move then
        if let some (head, index, fields) := storageProjection? context source then
          if fields.isEmpty then
            return ← lowerExpr context expected (.storageIndex head index span)
      let value ← lowerExpr context none valueSource
      let value ← match ← typeNode? value.2 with
        | some (.reference reference) => match ← typeNode? reference.referent with
            | some (.vector ..) => do
                let id ← pushExpression loc reference.referent <| .operation
                  (.reference .dereference) #[] #[value.1]
                pure (id, reference.referent)
            | _ => pure value
        | _ => pure value
      if let some (.tuple elements) ← typeNode? value.2 then
        let some position := literalIndex? indexSource
          | failAt "LEANER-INDEX-TYPE" "tuple indexing requires a nonnegative literal"
              (some span)
        let some elementType := elements[position]?
          | failAt "LEANER-INDEX-BOUNDS"
              s!"tuple index {position} is out of bounds for {elements.size} elements" (some span)
        let resultType ← if context.specification then projectSpecTypeId elementType
          else pure elementType
        let some sourcePlace := expressionPlace? valueSource
          | failAt "LEANER-INDEX-PLACE"
              "tuple indexing currently requires an assignable source value" (some span)
        let (base, baseType) ← lowerPlace context sourcePlace
        ensureType value.2 baseType span
        let index ← lowerExpr context none indexSource
        let place ← pushPlace (.index base index.1)
        if let some expected := expected then ensureType expected resultType span
        let id ← pushExpression loc resultType <|
          .operation (.read place) #[] #[]
        return (id, resultType)
      let some (.vector elementType _) ← typeNode? value.2
        | failAt "LEANER-INDEX-TYPE" "indexing requires a vector or tuple value" (some span)
      let index ← if context.specification then do
          if indexSource matches .primitive .range _ _ then
            lowerExpr context none indexSource
          else
            let indexType ← internType (.integer .unbounded true)
            lowerExpr context (some indexType) indexSource
        else do
          let indexType ← runtimeIndexType
          lowerExpr context (some indexType) indexSource
      let operationAndResult : LowerM (Operation × TypeId) :=
        if context.specification then do
          let indexNode ← typeNode? index.2
          match indexNode with
          | some .range => return (Operation.specification .sliceVector, value.2)
          | some (.integer .unbounded true) => do
              let projected ← projectSpecTypeId elementType
              -- Compiler-v2 specifications sometimes explicitly cast back
              -- into the vector's physical bit-vector element type. Honor
              -- that expected type; otherwise use the ordinary logical
              -- projection used by authored specification indexing.
              let resultType := if expected == some elementType then elementType
                else projected
              return (Operation.specification .indexVector, resultType)
          | _ =>
              failAt "LEANER-INDEX-TYPE"
                "specification indexing requires a logical integer or range" (some span)
        else pure (Operation.primitive .index, elementType)
      let (operation, resultType) ← operationAndResult
      if let some expected := expected then ensureType expected resultType span
      let instantiations := #[GenericArgument.typeArg { typeId := elementType, loc }]
      let id ← pushExpression loc resultType <|
        .operation operation instantiations #[value.1, index.1]
      return (id, resultType)
  | .membership element collection _ =>
      unless context.specification do
        failAt "LEANER-MEMBERSHIP-CONTEXT"
          "vector membership is available only in specification expressions" (some span)
      let collection ← lowerExpr context none collection
      let some (.vector elementType _) ← typeNode? collection.2
        | failAt "LEANER-MEMBERSHIP-TYPE" "membership requires a vector collection"
            (some span)
      let element ← lowerExpr context (some elementType) element
      let boolType ← internType .bool
      if let some expected := expected then ensureType expected boolType span
      let instantiations := #[GenericArgument.typeArg { typeId := elementType, loc }]
      let id ← pushExpression loc boolType <| .operation
        (.specification .containsVector) instantiations #[collection.1, element.1]
      return (id, boolType)
  | .variantTest value variants _ =>
      let value ← lowerExpr context none value
      -- Move 2 performs the same implicit receiver dereference for enum
      -- variant tests that it does for field and method receiver syntax.
      -- Preserve the explicit dereference in LIR, but accept the canonical
      -- source spelling `self is Variant` when `self` is a reference.
      let value ← match (← get).sourceNamespace.profile, ← typeNode? value.2 with
        | .move, some (.reference reference) =>
            match ← typeNode? reference.referent with
            | some (.nominal ..) => do
                let id ← pushExpression loc reference.referent <|
                  .operation (.reference .dereference) #[] #[value.1]
                pure (id, reference.referent)
            | _ => pure value
        | _, _ => pure value
      let some (.nominal owner _) ← typeNode? value.2
        | failAt "LEANER-VARIANT-TEST-TYPE"
            "variant testing requires a nominal value" (some span)
      let state ← get
      let some ownerName := state.tables.names[owner.index]?
        | failAt "LEANER-VARIANT-TEST-TYPE"
            "variant testing references a missing nominal name" (some span)
      -- The variants belong to the namespace that declared the enum, which a
      -- dependency interface makes available like any other declaration.
      let reference : QualifiedRef := { namespaceId := ownerName.namespaceId, name := owner }
      let some ownerNs ← namespaceOfRef? reference
        | failAt "LEANER-VARIANT-TEST-TYPE"
            s!"variant testing needs the declaration of `{ownerName.name}`, which this \
              compilation unit does not hold" (some span)
      unless ownerNs.profile == state.sourceNamespace.profile do
        failAt "LEANER-CROSS-PROFILE"
          "cross-profile variant testing requires a validated boundary adapter" (some span)
      let some declaration := sourceNominal? ownerNs ownerName.name
        | failAt "LEANER-VARIANT-TEST-TYPE"
            s!"unknown nominal type `{ownerName.name}`" (some span)
      for variant in variants do
        unless declaration.variants.any (·.name == variant) do
          failAt "LEANER-VARIANT-TEST-NAME"
            s!"enum `{declaration.name}` has no variant `{variant}`" (some span)
      let boolType ← internType .bool
      if let some expected := expected then ensureType expected boolType span
      let id ← pushExpression loc boolType <| .operation
        (.data (.testVariants reference variants)) #[] #[value.1]
      return (id, boolType)
  | .selectVariants ownerType fields value _ =>
      let .named ownerSegments _ := ownerType.value
        | failAt "LEANER-VARIANT-SELECT-TYPE"
            "variant-field selection requires a nominal owner type" (some ownerType.span)
      let (reference, declaration) ← resolveLocalNominal ownerSegments
      if declaration.external then
        failAt "LEANER-NOMINAL-EXTERNAL"
          s!"`{declaration.name}` is declared by another compilation unit, whose fields and \
            variants are not part of this one; only its identity and payload-free constructors \
            are available until dependency interfaces carry declarations" (some span)
      unless !fields.isEmpty do
        failAt "LEANER-VARIANT-SELECT-FIELDS"
          "variant-field selection must name at least one enum field" (some span)
      let ownerTypeId := (← lowerTypeUse context.types ownerType).typeId
      let instantiations ← match ← typeNode? ownerTypeId with
        | some (.nominal owner instantiations) => do
            unless owner == reference.name do
              failAt "LEANER-VARIANT-SELECT-TYPE"
                "variant-field owner resolves to a different nominal declaration"
                  (some ownerType.span)
            pure instantiations
        | _ => do
            failAt "LEANER-VARIANT-SELECT-TYPE"
              "variant-field owner did not lower to a nominal type" (some ownerType.span)
      let mut selectedFields := #[]
      for field in fields do
        let candidates := declaration.variants.filterMap fun variant =>
          variant.fields.find? (·.name == field)
        unless !candidates.isEmpty do
          failAt "LEANER-VARIANT-SELECT-FIELDS"
            s!"enum `{declaration.name}` has no variant field `{field}`" (some span)
        selectedFields := selectedFields ++ candidates
      let fieldTypes ← selectedFields.mapM fun field => do
        let instantiated ← nominalPatternFieldType declaration instantiations field span
        if context.specification then projectSpecTypeId instantiated else pure instantiated
      let resultType := fieldTypes[0]!
      let typeFuel := (← get).tables.types.size + 1
      unless ← fieldTypes.allM fun fieldType =>
          compatibleType resultType fieldType typeFuel do
        failAt "LEANER-VARIANT-SELECT-FIELDS"
          "selected variant fields have different types" (some span)
      if let some expected := expected then ensureType expected resultType span
      let value ← lowerExpr context (some ownerTypeId) value
      let id ← pushExpression loc resultType <| .operation
        (.data (.selectVariants reference fields)) #[] #[value.1]
      return (id, resultType)
  | .testVariants ownerType variants value _ =>
      let .named ownerSegments _ := ownerType.value
        | failAt "LEANER-VARIANT-TEST-TYPE"
            "variant testing requires a nominal owner type" (some ownerType.span)
      let (reference, declaration) ← resolveLocalNominal ownerSegments
      if declaration.external then
        failAt "LEANER-NOMINAL-EXTERNAL"
          s!"`{declaration.name}` is declared by another compilation unit, whose fields and \
            variants are not part of this one; only its identity and payload-free constructors \
            are available until dependency interfaces carry declarations" (some span)
      for variant in variants do
        unless declaration.variants.any (·.name == variant) do
          failAt "LEANER-VARIANT-TEST-NAME"
            s!"enum `{declaration.name}` has no variant `{variant}`" (some span)
      let ownerTypeId := (← lowerTypeUse context.types ownerType).typeId
      match ← typeNode? ownerTypeId with
      | some (.nominal owner _) => unless owner == reference.name do
          failAt "LEANER-VARIANT-TEST-TYPE"
            "variant-test owner resolves to a different nominal declaration"
              (some ownerType.span)
      | _ => do
          failAt "LEANER-VARIANT-TEST-TYPE"
            "variant-test owner did not lower to a nominal type" (some ownerType.span)
      let value ← lowerExpr context (some ownerTypeId) value
      let boolType ← internType .bool
      if let some expected := expected then ensureType expected boolType span
      let id ← pushExpression loc boolType <| .operation
        (.data (.testVariants reference variants)) #[] #[value.1]
      return (id, boolType)
  | .discriminant ownerType resultType value _ =>
      let .named ownerSegments _ := ownerType.value
        | failAt "LEANER-DISCRIMINANT-TYPE"
            "a discriminant expression requires a nominal enum type" (some ownerType.span)
      let (reference, declaration) ← resolveLocalNominal ownerSegments
      if declaration.external then
        failAt "LEANER-NOMINAL-EXTERNAL"
          s!"`{declaration.name}` is declared by another compilation unit, whose fields and \
            variants are not part of this one; only its identity and payload-free constructors \
            are available until dependency interfaces carry declarations" (some span)
      unless !declaration.variants.isEmpty &&
          declaration.variants.all (·.discriminant.isSome) do
        failAt "LEANER-DISCRIMINANT-TYPE"
          "a discriminant expression requires an enum with explicit discriminants" (some span)
      let ownerTypeId := (← lowerTypeUse context.types ownerType).typeId
      let instantiations ← match ← typeNode? ownerTypeId with
        | some (.nominal owner instantiations) => do
            unless owner == reference.name do
              failAt "LEANER-DISCRIMINANT-TYPE"
                "the discriminant owner resolves to a different declaration" (some span)
            pure instantiations
        | _ => do
            failAt "LEANER-DISCRIMINANT-TYPE"
              "the discriminant owner did not lower to a nominal type" (some span)
      let value ← lowerExpr context (some ownerTypeId) value
      let resultTypeId := (← lowerTypeUse context.types resultType).typeId
      unless (← typeNode? resultTypeId).any fun | .integer _ _ => true | _ => false do
        failAt "LEANER-DISCRIMINANT-TYPE"
          "a discriminant result must be an integer" (some resultType.span)
      if let some expected := expected then ensureType expected resultTypeId span
      let id ← pushExpression loc resultTypeId <| .operation
        (.data (.discriminant reference)) instantiations #[value.1]
      return (id, resultTypeId)
  | .placeOperation operation sourcePlace _ =>
      if context.specification then
        return ← lowerExpr context expected sourcePlace
      let (place, placeType, bindings, _) ← lowerExpressionPlaceWith
        (lowerExpr context none) context sourcePlace
      if let some expected := expected then ensureType expected placeType span
      let operation := match operation with
        | .move => LeanerIR.Operation.move place
        | .copy => LeanerIR.Operation.copy place
        | .read => LeanerIR.Operation.read place
      let id ← pushExpression loc placeType <| .operation operation #[] #[]
      return (← bindPlaceIndices loc placeType bindings id, placeType)
  | .borrowPlace mutable place _ =>
      if context.specification then
        let value ← lowerSpecificationPlaceValue context place
        if let some expected := expected then ensureType expected value.2 span
        return value
      let (place, placeType) ← lowerPlace context place
      let resultType ← match expected with
        | some resultType => pure resultType
        | none => inferredReferenceType (if mutable then ReferenceKind.mutable else
            ReferenceKind.shared) placeType loc
      let some (.reference reference) ← typeNode? resultType
        | failAt "LEANER-BORROW-TYPE" "a place borrow requires a reference expected type"
            (some span)
      let expectedKind := if mutable then ReferenceKind.mutable else .shared
      unless reference.kind == expectedKind && reference.referent == placeType do
        let expectedText := repr (← typeNode? reference.referent)
        let actualText := repr (← typeNode? placeType)
        failAt "LEANER-BORROW-TYPE" s!"place-borrow kind or referent differs from its expected reference type: borrowing {actualText} (arena entry {placeType.index}) where {repr reference.kind} {expectedText} (arena entry {reference.referent.index}) is expected" (some span)
      let kind := if mutable then BorrowKind.mutable else .immutable
      let id ← pushExpression loc resultType <| .operation (.borrow kind place) #[] #[]
      return (id, resultType)
  | .dropPlace place _ =>
      let unitType ← internType .unit
      if let some expected := expected then ensureType expected unitType span
      if context.specification then
        return (← pushExpression loc unitType (.value .unit), unitType)
      let (place, _) ← lowerPlace context place
      let id ← pushExpression loc unitType <|
        .operation (.drop place) #[] #[]
      return (id, unitType)
  | .borrowValue mutable value _ | .rawBorrowValue mutable value _ =>
      if context.specification then
        return ← lowerExpr context expected value
      let expectedKind := if mutable then ReferenceKind.mutable else .shared
      if (← get).sourceNamespace.profile == .move then
        if let some (head, index, fields) := storageProjection? context value then
          if (storageIndexLocalName? context head).isNone then
            let resource ← lowerTypeUse context.types head
            let key ← lowerExpr context none index
            let borrowKind := if mutable then BorrowKind.mutable else .immutable
            let mut referent := resource.typeId
            let mut referenceType ← inferredReferenceType expectedKind referent loc
            let borrowed ← pushExpression loc referenceType <| .operation
              (.global (.borrow borrowKind)) #[.typeArg resource] #[key.1]
            if mutable && !fields.isEmpty then
              -- A field-focused mutable storage borrow reborrows the
              -- projected place through a synthesized holder local: place
              -- resolution is the runtime meaning of a projection, and a
              -- value-level selection of a mutable reference has none.
              let holder ← pushTemporaryLocal referenceType loc
              let pattern ← pushPattern
                { loc, typeId := referenceType, kind := .variable holder }
              let mut place ← pushPlace (.deref (← pushPlace (.localVar holder)))
              for field in fields do
                let (owner, fieldName, fieldType) ←
                  resolveNominalField referent field span
                referent := fieldType
                place ← pushPlace (.field place owner fieldName)
              referenceType ← inferredReferenceType expectedKind referent loc
              let focused ← pushExpression loc referenceType <| .operation
                (.borrow .mutable place) #[] #[]
              let id ← pushExpression loc referenceType <|
                .letDecl pattern (some borrowed) focused
              if let some expected := expected then ensureType expected referenceType span
              return (id, referenceType)
            let mut selected := borrowed
            for field in fields do
              let (owner, _, fieldType) ← resolveNominalField referent field span
              referent := fieldType
              referenceType ← inferredReferenceType expectedKind referent loc
              selected ← pushExpression loc referenceType <| .operation
                (.data (.select owner field)) #[] #[selected]
            if let some expected := expected then ensureType expected referenceType span
            return (selected, referenceType)
      if let some placeSource := localStoragePlace? context value then
        let indexType ← runtimeIndexType
        let (place, placeType, bindings, _) ← lowerExpressionPlaceWith
          (lowerExpr context (some indexType)) context placeSource
          (!(source matches .rawBorrowValue ..))
        let resultType ← match expected with
          | some resultType => pure resultType
          | none => inferredReferenceType expectedKind placeType loc
        let some (.reference reference) ← typeNode? resultType
          | failAt "LEANER-BORROW-TYPE" "a place borrow requires a reference expected type"
              (some span)
        unless reference.kind == expectedKind && reference.referent == placeType do
          failAt "LEANER-BORROW-TYPE"
            "place-borrow kind or referent differs from its expected reference type" (some span)
        let kind := if mutable then BorrowKind.mutable else .immutable
        let id ← pushExpression loc resultType <| .operation (.borrow kind place) #[] #[]
        return (← bindPlaceIndices loc resultType bindings id, resultType)
      let (value, resultType) ← match expected with
        | some resultType => do
            let some (.reference reference) ← typeNode? resultType
              | failAt "LEANER-BORROW-TYPE"
                  "a value borrow requires a reference expected type" (some span)
            unless reference.kind == expectedKind do
              failAt "LEANER-BORROW-TYPE"
                "value-borrow kind differs from its expected reference type" (some span)
            let value ← lowerExpr context (some reference.referent) value
            pure (value, resultType)
        | none => do
            let value ← lowerExpr context none value
            let resultType ← inferredReferenceType expectedKind value.2 loc
            pure (value, resultType)
      let kind := if mutable then BorrowKind.mutable else .immutable
      let id ← pushExpression loc resultType <| .operation
        (.reference (.borrow kind)) #[] #[value.1]
      return (id, resultType)
  | .freezeReference explicit value _ =>
      if context.specification then
        return ← lowerExpr context expected value
      let value ← lowerExpr context none value
      let some (.reference sourceReference) ← typeNode? value.2
        | failAt "LEANER-FREEZE-TYPE" "a reference freeze operand must be a reference"
            (some span)
      let resultType ← match expected with
        | some resultType => pure resultType
        | none => inferredReferenceType .shared sourceReference.referent loc
      let some (.reference resultReference) ← typeNode? resultType
        | failAt "LEANER-FREEZE-TYPE" "a reference freeze requires a reference result"
            (some span)
      unless resultReference.kind == .shared do
        failAt "LEANER-FREEZE-TYPE" "a reference freeze result must be shared" (some span)
      unless sourceReference.profile == resultReference.profile &&
          sourceReference.referent == resultReference.referent do
        failAt "LEANER-FREEZE-TYPE" "a reference freeze changes its profile or referent"
          (some span)
      let id ← pushExpression loc resultType <| .operation
        (.reference (.freeze explicit)) #[] #[value.1]
      return (id, resultType)
  | .dereference value _ =>
      let value ← lowerExpr context none value
      if context.specification then
        match ← typeNode? value.2 with
        | some (.reference reference) =>
            if let some expected := expected then ensureType expected reference.referent span
            let id ← pushExpression loc reference.referent <| .operation
              (.reference .dereference) #[] #[value.1]
            return (id, reference.referent)
        | _ =>
            if let some expected := expected then ensureType expected value.2 span
            return value
      else
        let some (.reference reference) ← typeNode? value.2
          | failAt "LEANER-DEREFERENCE-TYPE"
              "core.ref.dereference requires a reference operand" (some span)
        if let some expected := expected then ensureType expected reference.referent span
        let id ← pushExpression loc reference.referent <| .operation
          (.reference .dereference) #[] #[value.1]
        return (id, reference.referent)
  | .mutateReference reference value _ =>
      let unitType ← internType .unit
      if let some expected := expected then ensureType expected unitType span
      if context.specification then
        let reference ← lowerExpr context none reference
        let valueType ← match ← typeNode? reference.2 with
          | some (.reference referenceType) => pure referenceType.referent
          | _ => pure reference.2
        let value ← lowerExpr context (some valueType) value
        ensureType valueType value.2 span
        return (← pushExpression loc unitType (.value .unit), unitType)
      let reference ← lowerExpr context none reference
      let some (.reference referenceType) ← typeNode? reference.2
        | failAt "LEANER-MUTATE-TYPE" "reference mutation requires a reference operand"
            (some span)
      unless referenceType.kind == .mutable do
        failAt "LEANER-MUTATE-TYPE" "reference mutation requires a mutable reference" (some span)
      let value ← lowerExpr context (some referenceType.referent) value
      let id ← pushReferenceMutation loc unitType reference.2 reference.1 value.1
      return (id, unitType)
  | .quantifier kind binders body _ =>
      unless context.specification do
        failAt "LEANER-QUANTIFIER-CONTEXT"
          "quantifiers are only available in specification expressions" (some span)
      let mut active := context
      let mut loweredBinders := #[]
      for (pattern, domain) in binders do
        let domainValue ← lowerExpr active none domain
        let patternType ← match ← typeNode? domainValue.2 with
          -- A vector keeps its element representation in the specification
          -- domain, so a binder ranging over one binds that element type; a
          -- use of it in a logical position widens implicitly.
          | some (.vector element _) => pure element
          | some .range => internType (.integer .unbounded true)
          -- `x : T` ranges over every value of a type.
          | some (.typeDomain element) => projectSpecTypeId element
          | _ => failAt "LEANER-QUANTIFIER-DOMAIN" "a quantifier domain must be a vector, logical range, or type" (some domain.span)
        let loweredPattern ← lowerBindingPattern active pattern patternType
        let boundLocals ← bindingPatternLocals active pattern
        active := withBoundLocals active boundLocals
        loweredBinders := loweredBinders.push {
          pattern := loweredPattern, domain := domainValue.1 }
      let boolType ← internType .bool
      let body ← lowerExpr active (some boolType) body
      if let some expected := expected then ensureType expected boolType span
      let id ← pushExpression loc boolType <| .quantifier kind.toLIR
        loweredBinders #[] none body.1
      return (id, boolType)
  | .specification operation types arguments _ =>
      unless context.specification do
        failAt "LEANER-SPEC-OPERATION"
          "specification operations are only available in specification expressions" (some span)
      match operation with
      | .behavior kind range =>
          unless types.isEmpty do
            failAt "LEANER-SPEC-GENERICS"
              "behavior predicates do not accept type arguments" (some span)
          let some targetSource := arguments[0]?
            | failAt "LEANER-SPEC-ARITY"
                "a behavior predicate requires a function-value target" (some span)
          let target ← lowerExpr context none targetSource
          let some (.function parameterTypes functionResult _) ← typeNode? target.2
            | failAt "LEANER-SPEC-BEHAVIOR-TARGET"
                "a behavior predicate target must have function type" (some targetSource.span)
          -- A `&mut` parameter is both an input and an output of the call, so
          -- what the target ensures relates its pre-state value, the results,
          -- and its post-state value.
          let mutatedParameters ← parameterTypes.filterMapM fun typeId => do
            match ← typeNode? typeId with
            | some (.reference reference) =>
                if reference.kind == .mutable then
                  pure (some (← projectSpecTypeId typeId))
                else pure none
            | _ => pure none
          let parameterTypes ← parameterTypes.mapM projectSpecTypeId
          let functionResult ← projectSpecTypeId functionResult
          let sourceValues := arguments.drop 1
          let resultSlots ← match ← typeNode? functionResult with
            | some .unit => pure #[]
            | some (.tuple elements) => pure elements
            | _ => pure #[functionResult]
          let expectedTypes := match kind with
            | .ensuresOf => parameterTypes ++ resultSlots ++ mutatedParameters
            | .foldsOf => #[]
            | .requiresOf | .abortsOf | .resultOf | .unchangedOf | .writeOf _ =>
                parameterTypes
          if kind == .foldsOf then
            unless sourceValues.size == 2 do
              failAt "LEANER-SPEC-ARITY"
                s!"folds_of expects two value arguments, got {sourceValues.size}" (some span)
          else unless sourceValues.size == expectedTypes.size do
            failAt "LEANER-SPEC-ARITY"
              s!"behavior predicate expects {expectedTypes.size} value argument(s), got {sourceValues.size}"
              (some span)
          let loweredValues ← if kind == .foldsOf then
              sourceValues.mapM (lowerExpr context none)
            else
              (sourceValues.zip expectedTypes).mapM fun (argument, expectedType) =>
                lowerExpr context (some expectedType) argument
          let resultType ← match kind with
            | .resultOf =>
                if resultSlots.isEmpty then
                  failAt "LEANER-SPEC-BEHAVIOR-RESULT"
                    "result_of cannot summarize a function with no return value" (some span)
                pure functionResult
            | .writeOf _ => requireExpected expected span "write_of result"
            | .requiresOf | .abortsOf | .ensuresOf | .unchangedOf | .foldsOf =>
                internType .bool
          if let some expected := expected then ensureType expected resultType span
          let lirRange : LeanerIR.MemoryRange := { pre := range.pre, post := range.post }
          let id ← pushExpression loc resultType <| .operation
            (.specification (.behavior kind.toLIR lirRange)) #[]
            (#[target.1] ++ loweredValues.map (·.1))
          return (id, resultType)
      | .saveStateAnchor label | .foldsCaptureAnchor label =>
          unless types.isEmpty do
            failAt "LEANER-SPEC-GENERICS"
              "specification state anchors do not accept type arguments" (some span)
          unless arguments.isEmpty do
            failAt "LEANER-SPEC-ARITY"
              s!"specification state-anchor marker expects no arguments, got {arguments.size}"
              (some span)
          let boolType ← internType .bool
          if let some expected := expected then ensureType expected boolType span
          let operation := match operation with
            | .saveStateAnchor _ => SpecOperation.saveStateAnchor label
            | .foldsCaptureAnchor _ => SpecOperation.foldsCaptureAnchor label
            | .behavior .. | .withStateAnchor _ | .old | .global | .typeDomain |
                .result _ | .inlineCallSummary | .emptyVector | .singletonVector |
                .updateVector | .concatVector | .indexOfVector | .containsVector |
                .lengthVector | .indexVector | .sliceVector | .bitVectorToInt |
                .intToBitVector |
                .inRange | .inVectorRange | .vectorRange => unreachable!
          let id ← pushExpression loc boolType <|
            .operation (.specification operation) #[] #[]
          return (id, boolType)
      | .withStateAnchor label =>
          unless types.isEmpty do
            failAt "LEANER-SPEC-GENERICS"
              "spec.withStateAnchor does not accept type arguments" (some span)
          unless arguments.size == 1 do
            failAt "LEANER-SPEC-ARITY"
              s!"spec.withStateAnchor expects one argument, got {arguments.size}" (some span)
          let value ← lowerExpr context expected arguments[0]!
          let id ← pushExpression loc value.2 <|
            .operation (.specification (.withStateAnchor label)) #[] #[value.1]
          return (id, value.2)
      | .result index =>
          unless types.isEmpty && arguments.isEmpty do
            failAt "LEANER-SPEC-ARITY" "a specification result takes no arguments" (some span)
          let typeId ← specificationResultType context index span
          if let some expected := expected then ensureType expected typeId span
          let id ← pushExpression loc typeId <|
            .operation (.specification (.result index)) #[] #[]
          return (id, typeId)
      | .inlineCallSummary =>
          unless types.isEmpty do
            failAt "LEANER-SPEC-GENERICS"
              "spec.inlineCallSummary does not accept type arguments" (some span)
          unless arguments.size == 2 do
            failAt "LEANER-SPEC-ARITY"
              s!"spec.inlineCallSummary expects two arguments, got {arguments.size}" (some span)
          let result ← lowerExpr context none arguments[0]!
          let aborts ← lowerExpr context (some (← internType .bool)) arguments[1]!
          let boolType ← internType .bool
          if let some expected := expected then ensureType expected boolType span
          let id ← pushExpression loc boolType <|
            .operation (.specification .inlineCallSummary) #[] #[result.1, aborts.1]
          return (id, boolType)
      | .typeDomain =>
          unless types.size == 1 do
            failAt "LEANER-SPEC-GENERICS" "spec.typeDomain requires one type argument" (some span)
          let element ← lowerTypeUse context.types types[0]!
          unless arguments.isEmpty do
            failAt "LEANER-SPEC-ARITY" "spec.typeDomain takes no arguments" (some span)
          let resultType ← internType (.typeDomain element.typeId)
          if let some expected := expected then ensureType expected resultType span
          let id ← pushExpression loc resultType <|
            .operation (.specification .typeDomain) #[.typeArg element] #[]
          return (id, resultType)
      | .old =>
          unless types.isEmpty do
            failAt "LEANER-SPEC-GENERICS" "spec.old does not accept type arguments" (some span)
          unless arguments.size == 1 do
            failAt "LEANER-SPEC-ARITY"
              s!"spec.old expects one argument, got {arguments.size}" (some span)
          let value ← lowerExpr context expected arguments[0]!
          let typeArgument : TypeUse := { typeId := value.2, loc }
          let id ← pushExpression loc value.2 <|
            .operation (.specification .old) #[.typeArg typeArgument] #[value.1]
          return (id, value.2)
      | .global =>
          let resource ← match types.toList with
            | [resource] => lowerTypeUse context.types resource
            | _ => failAt "LEANER-SPEC-GENERICS" "spec.global requires exactly one resource type" (some span)
          unless arguments.size == 1 do
            failAt "LEANER-SPEC-ARITY"
              s!"spec.global expects one key, got {arguments.size}" (some span)
          let key ← lowerExpr context none arguments[0]!
          if let some expected := expected then ensureType expected resource.typeId span
          let id ← pushExpression loc resource.typeId <|
            .operation (.specification (.global none)) #[.typeArg resource] #[key.1]
          return (id, resource.typeId)
      | .emptyVector | .singletonVector | .updateVector | .concatVector |
          .indexOfVector | .containsVector | .lengthVector | .indexVector |
          .sliceVector | .inVectorRange | .vectorRange =>
          let explicitElement ← match types.toList with
            | [element] => pure (some (← lowerTypeUse context.types element))
            | [] => pure none
            | _ => failAt "LEANER-SPEC-GENERICS" "a specification vector operation requires exactly one element type" (some span)
          let elementTypeId ← match explicitElement with
            | some element => pure element.typeId
            | none => do
                -- The element type may be inferred, matching imported units
                -- whose specification vector nodes carry no instantiation:
                -- from the expected vector type, else from the first
                -- vector-typed operand.
                let fromExpected ← match expected with
                  | some expectedType => do
                      match ← typeNode? expectedType with
                      | some (.vector element _) => pure (some element)
                      | _ => pure none
                  | none => pure none
                match fromExpected with
                | some element => pure element
                | none => do
                    let some first := arguments[0]?
                      | failAt "LEANER-SPEC-GENERICS"
                          "a specification vector operation without operands requires an element type"
                          (some span)
                    let before ← get
                    let probed ← lowerExpr context none first
                    let after ← get
                    set { after with
                      output := before.output
                      temporaryLocalBase := before.temporaryLocalBase
                      temporaryLocals := before.temporaryLocals }
                    match (← typeNode? probed.2) with
                    | some (LeanerIR.Ty.vector element _) => pure element
                    | _ =>
                        failAt "LEANER-SPEC-GENERICS" "a specification vector operation requires an element type" (some span)
          let vectorType ← internType (.vector elementTypeId none)
          let integerType ← internType (.integer .unbounded true)
          let boolType ← internType .bool
          let rangeType ← internType .range
          let (lirOperation, parameterTypes, resultType) := match operation with
            | .emptyVector => (SpecOperation.emptyVector, #[], vectorType)
            | .singletonVector => (.singletonVector, #[elementTypeId], vectorType)
            | .updateVector =>
                (.updateVector, #[vectorType, integerType, elementTypeId], vectorType)
            | .concatVector => (.concatVector, #[vectorType, vectorType], vectorType)
            | .indexOfVector => (.indexOfVector, #[vectorType, elementTypeId], integerType)
            | .containsVector => (.containsVector, #[vectorType, elementTypeId], boolType)
            | .lengthVector => (.lengthVector, #[vectorType], integerType)
            | .indexVector => (.indexVector, #[vectorType, integerType], elementTypeId)
            | .sliceVector => (.sliceVector, #[vectorType, rangeType], vectorType)
            | .inVectorRange => (.inVectorRange, #[vectorType, integerType], boolType)
            | .vectorRange => (.vectorRange, #[vectorType], rangeType)
            | .behavior .. | .old | .saveStateAnchor _ | .withStateAnchor _ |
                .foldsCaptureAnchor _ |
                .global | .typeDomain | .result _ | .inlineCallSummary |
                .bitVectorToInt | .intToBitVector | .inRange => unreachable!
          unless arguments.size == parameterTypes.size do
            failAt "LEANER-SPEC-ARITY"
              s!"specification vector operation expects {parameterTypes.size} argument(s), got {arguments.size}"
              (some span)
          let lowered ← (arguments.zip parameterTypes).mapM fun (argument, parameterType) =>
            lowerExpr context (some parameterType) argument
          if let some expected := expected then ensureType expected resultType span
          let instantiations : Array LeanerIR.GenericArgument :=
            match explicitElement with
            | some element => #[.typeArg element]
            | none => #[]
          let id ← pushExpression loc resultType <| .operation (.specification lirOperation)
            instantiations (lowered.map (·.1))
          return (id, resultType)
      | .bitVectorToInt =>
          unless types.isEmpty do
            failAt "LEANER-SPEC-GENERICS"
              "spec.bitVectorToInt does not accept type arguments" (some span)
          unless arguments.size == 1 do
            failAt "LEANER-SPEC-ARITY"
              s!"spec.bitVectorToInt expects one argument, got {arguments.size}" (some span)
          let value ← lowerExpr context none arguments[0]!
          unless (← typeNode? value.2).any fun
              | .integer (.bits _) _ | .integer .pointer _ => true
              | _ => false do
            failAt "LEANER-SPEC-BITVECTOR"
              "spec.bitVectorToInt requires a fixed-width integer" (some span)
          let resultType ← internType (.integer .unbounded true)
          if let some expected := expected then ensureType expected resultType span
          let id ← pushExpression loc resultType <|
            .operation (.specification .bitVectorToInt) #[] #[value.1]
          return (id, resultType)
      | .intToBitVector =>
          let resultType ← match types.toList with
            | [result] => pure (← lowerTypeUse context.types result).typeId
            | [] =>
                -- The width-less spelling keeps the exchange's deferred
                -- bit-vector width in the projected specification domain.
                internType (.integer .unbounded true)
            | _ => failAt "LEANER-SPEC-GENERICS" "spec.intToBitVector requires exactly one fixed-width result type" (some span)
          unless (← typeNode? resultType).any fun
              | .integer (.bits _) _ | .integer .pointer _
              | .integer .unbounded true => true
              | _ => false do
            failAt "LEANER-SPEC-BITVECTOR"
              "spec.intToBitVector requires a fixed-width integer result" (some span)
          unless arguments.size == 1 do
            failAt "LEANER-SPEC-ARITY"
              s!"spec.intToBitVector expects one argument, got {arguments.size}" (some span)
          let integerType ← internType (.integer .unbounded true)
          let value ← lowerExpr context (some integerType) arguments[0]!
          if let some expected := expected then ensureType expected resultType span
          let id ← pushExpression loc resultType <|
            .operation (.specification .intToBitVector) #[] #[value.1]
          return (id, resultType)
      | .inRange =>
          unless types.isEmpty do
            failAt "LEANER-SPEC-GENERICS" "spec.inRange does not accept type arguments"
              (some span)
          unless arguments.size == 2 do
            failAt "LEANER-SPEC-ARITY"
              s!"spec.inRange expects two arguments, got {arguments.size}" (some span)
          let rangeType ← internType .range
          let integerType ← internType (.integer .unbounded true)
          let boolType ← internType .bool
          let range ← lowerExpr context (some rangeType) arguments[0]!
          let index ← lowerExpr context (some integerType) arguments[1]!
          if let some expected := expected then ensureType expected boolType span
          let id ← pushExpression loc boolType <| .operation (.specification .inRange) #[]
            #[range.1, index.1]
          return (id, boolType)
  | .specBlock conditions _ =>
      let boolType ← internType .bool
      let unitType ← internType .unit
      if let some expected := expected then ensureType expected unitType span
      let specContext ← specificationExprContext context
      let mut active := specContext
      let mut loweredConditions := #[]
      for (kind, condition) in conditions do
        let conditionLoc ← addLoc condition.span
        let expression ← lowerExpr active
          (if kind matches .let_ _ then none else some boolType) condition
        loweredConditions := loweredConditions.push ({
          loc := conditionLoc
          kind := match kind with
            | .let_ name => ConditionKind.letPre name
            | .assertion => ConditionKind.assertion
            | .assumption => ConditionKind.assumption
            | .loopInvariant => ConditionKind.loopInvariant
          expression := expression.1 } : LeanerIR.Condition)
        if let .let_ name := kind then
          let some declaration := declaredLocal? active condition.span name
            | failAt "LEANER-SPEC-LET"
                s!"specification binding `{name}` was not predeclared" (some condition.span)
          active := { active with
            locals := #[(name, declaration.id, declaration.typeId)] ++ active.locals }
      let id ← pushExpression loc unitType <| .spec { loc, conditions := loweredConditions }
      return (id, unitType)
  | .global operation resource arguments _ =>
      let resource ← lowerTypeUse context.types resource
      let arity := match operation with
        | .publish => 2
        | _ => 1
      unless arguments.size == arity do
        failAt "LEANER-GLOBAL-ARITY"
          s!"global operation expects {arity} argument(s), got {arguments.size}" (some span)
      let key ← lowerExpr context none arguments[0]!
      let mut loweredArguments := #[key.1]
      let (lirOperation, resultType) ← match operation with
        | .contains => pure (GlobalKind.contains, ← internType .bool)
        | .take => pure (.take, resource.typeId)
        | .publish => do
            let value ← lowerExpr context (some resource.typeId) arguments[1]!
            loweredArguments := loweredArguments.push value.1
            pure (.publish, ← internType .unit)
        | .borrow mutable => do
            let borrowKind := if mutable then BorrowKind.mutable else .immutable
            let referenceKind := if mutable then ReferenceKind.mutable else .shared
            let resultType ← inferredReferenceType referenceKind resource.typeId loc
            pure (.borrow borrowKind, resultType)
      if let some expected := expected then ensureType expected resultType span
      let id ← pushExpression loc resultType <| .operation (.global lirOperation)
        #[.typeArg resource] loweredArguments
      return (id, resultType)
  | .methodCall name result types receiver arguments _ =>
      -- Receiver functions are ordinary functions whose first source
      -- parameter is named `self`. Move/Rust dot notation auto-borrows a
      -- value receiver when that parameter is a reference. Until dependency
      -- interfaces carry authoritative signatures, the shared builtin table
      -- provides the same information for the standard vector module.
      let state ← get
      let localDeclaration := sourceFunction? state.sourceNamespace name
      let standardTarget := standardReceiverTarget? state.sourceNamespace name
      let before ← get
      let receiverProbe ← lowerExpr context none receiver
      let receiverProbeType ← typeNode? receiverProbe.2
      let (receiverBaseType, receiverWasReference) := match receiverProbeType with
        | some (.reference reference) => (reference.referent, true)
        | _ => (receiverProbe.2, false)
      let receiverIsVector := (← typeNode? receiverBaseType).any fun
        | .vector .. => true
        | _ => false
      set before
      let localVectorReceiver := localDeclaration.any fun declaration =>
        declaration.parameters[0]?.any fun parameter =>
          sourceTypeIsVector parameter.type.value
      let useStandard := receiverIsVector && standardTarget.isSome &&
        (!localDeclaration.isSome || !localVectorReceiver)
      if useStandard then
        let some (path, receiverMode) := standardTarget
          | failAt "LEANER-RECEIVER-NAME"
              s!"standard receiver `{name}` has no imported target" (some span)
        unless types.isEmpty do
          failAt "LEANER-RECEIVER-GENERICS"
            "standard receiver calls infer their type argument from `self`" (some span)
        let receiverSource ← if context.specification then pure receiver else
          match receiverMode with
          | .byValue =>
              if receiverWasReference then
                failAt "LEANER-RECEIVER-TYPE"
                  s!"standard receiver `{name}` consumes its vector by value" (some span)
              else pure receiver
          | .shared | .mutable =>
              if receiverWasReference then pure receiver
              else
                let mutable := receiverMode == .mutable
                pure <| match expressionPlace? receiver with
                  | some place => .borrowPlace mutable place receiver.span
                  | none => .borrowValue mutable receiver receiver.span
        let loweredReceiver ← lowerExpr context none receiverSource
        let loweredArguments ← arguments.mapM (lowerExpr context none)
        let allArguments := #[loweredReceiver] ++ loweredArguments
        let some (.vector element _) ← typeNode? receiverBaseType
          | failAt "LEANER-RECEIVER-TYPE"
              "a standard vector receiver has no vector element type" (some span)
        let resultType ← match name with
          | "length" => do
              if context.specification then internType (.integer .unbounded true)
              else runtimeIndexType
          | "is_empty" | "contains" => internType .bool
          | "index_of" => do
              let boolType ← internType .bool
              let indexType ← if context.specification then
                  internType (.integer .unbounded true) else runtimeIndexType
              internType (.tuple #[boolType, indexType])
          | "borrow" | "last" => do
              if context.specification then projectSpecTypeId element
              else inferredReferenceType .shared element loc
          | "borrow_mut" | "last_mut" => do
              if context.specification then projectSpecTypeId element
              else inferredReferenceType .mutable element loc
          | "pop_back" | "remove" | "swap_remove" | "replace" =>
              if context.specification then projectSpecTypeId element else pure element
          | "trim" | "trim_reverse" | "remove_value" => pure receiverBaseType
          | "rotate" | "rotate_slice" =>
              if context.specification then internType (.integer .unbounded true)
              else runtimeIndexType
          | "push_back" | "destroy_empty" | "swap" | "reverse" |
              "reverse_slice" | "reverse_append" | "append" | "insert" => internType .unit
          | _ => do
              failAt "LEANER-RECEIVER-NAME"
                (s!"standard receiver `{name}` has no result signature") (some span)
        if let some result := result then
          let physical := (← lowerTypeUse context.types result).typeId
          let annotated ← if context.specification then projectSpecTypeId physical
            else pure physical
          ensureType resultType annotated result.span
        if let some expected := expected then ensureType expected resultType span
        let reference ← resolveExternalRef path
        let current ← get
        let some namespaceRef := current.tables.namespaces[reference.namespaceId.index]?
          | failAt "LEANER-RECEIVER-NAME"
              "a standard receiver references a missing namespace" (some span)
        let instantiationTypes := standardCallInferredTypes? current.tables
          (some current.sourceNamespace.profile.toLIR) namespaceRef name
          (allArguments.map (·.2))
        let some instantiationTypes := instantiationTypes
          | failAt "LEANER-RECEIVER-GENERICS"
              s!"cannot infer the type argument of standard receiver `{name}`" (some span)
        let instantiations := instantiationTypes.map fun typeId =>
          GenericArgument.typeArg { typeId, loc }
        let operation := if context.specification then
            Operation.specification (.functionCall reference {})
          else Operation.call (.function reference)
        let id ← pushExpression loc resultType <| .operation
          operation instantiations (allArguments.map (·.1))
            (some .receiverCall)
        return (id, resultType)
      let receiver ← if !context.specification then
          match localDeclaration with
          | some declaration => match declaration.parameters[0]? with
              | some parameter => match parameter.type.value with
                  | .reference mutable _ _ =>
                      pure <| if receiverWasReference then receiver
                        else .borrowValue mutable receiver receiver.span
                  | _ => pure receiver
              | none => pure receiver
          | none => pure receiver
        else pure receiver
      let arguments := #[receiver] ++ arguments
      let source := match result, types.isEmpty with
        | none, true => Expr.call #[name] arguments span
        | none, false => Expr.genericCall #[name] types arguments span
        | some result, true => Expr.typedCall #[name] result arguments span
        | some result, false => Expr.typedGenericCall #[name] result types arguments span
      let lowered ← lowerExpr context expected source
      markReceiverSurface lowered.1 span
      return lowered
  | .closure segments result captures _ =>
      let (reference, declaration) ← resolveLocalRef segments span
      unless declaration.generics.isEmpty do
        failAt "LEANER-CLOSURE-GENERICS"
          "generic closure targets require explicit generic arguments" (some span)
      let declarationTypes ← liftM <|
        typeContext declaration.generics (← namespaceOfRef? reference)
      let parameterTypes ← declaration.parameters.mapM fun parameter => do
        let declared := (← lowerTypeUse declarationTypes parameter.type).typeId
        if context.specification then projectSpecTypeId declared else pure declared
      unless captures.size ≤ parameterTypes.size do
        failAt "LEANER-CLOSURE-ARITY"
          "a closure captures more values than its target accepts" (some span)
      let captures ← (captures.zip (parameterTypes.take captures.size)).mapM
        fun (capture, type) => lowerExpr context (some type) capture
      let physicalResultType := (← lowerTypeUse context.types result).typeId
      let resultType ← if context.specification then projectSpecTypeId physicalResultType
        else pure physicalResultType
      if let some expected := expected then ensureType expected resultType span
      let id ← pushExpression loc resultType <| .operation
        (.call (.closure reference)) #[] (captures.map (·.1))
      return (id, resultType)
  | .invoke callable arguments _ =>
      let callable ← lowerExpr context none callable
      let some (.function parameterTypes resultType _) ← typeNode? callable.2
        | failAt "LEANER-INVOKE-TYPE" "core.invoke requires a function-valued operand"
            (some span)
      -- A function value keeps its physical signature; applying it produces a
      -- value, which the specification domain reads logically.
      let parameterTypes ← if context.specification then
          parameterTypes.mapM projectSpecTypeId
        else pure parameterTypes
      let resultType ← if context.specification then projectSpecTypeId resultType
        else pure resultType
      unless arguments.size == parameterTypes.size do
        failAt "LEANER-INVOKE-ARITY"
          s!"core.invoke expects {parameterTypes.size} argument(s), got {arguments.size}"
          (some span)
      let arguments ← (arguments.zip parameterTypes).mapM fun (argument, parameterType) =>
        lowerExpr context (some parameterType) argument
      let id ← pushExpression loc resultType <| .operation (.call .invoke) #[]
        (#[callable.1] ++ arguments.map (·.1))
      finishCall context expected id resultType loc span
  | .genericCall segments typeArguments arguments _ =>
      if context.specification then
        if let some declaration := specFunctionForPath? (← get) segments then
          let (reference, _) ← resolveLocalSpecRef segments
          unless declaration.generics.size == typeArguments.size &&
              declaration.generics.all (·.kind == .type) do
            failAt "LEANER-SPEC-CALL-GENERICS"
              "generic call arguments do not match the specification function's type binders"
              (some span)
          let instantiations ← typeArguments.mapM fun argument => do
            pure <| GenericArgument.typeArg (← lowerTypeUse context.types argument)
          let declarationTypes ← liftM <|
            typeContext declaration.generics (← namespaceOfRef? reference)
          let parameterTypes ← declaration.parameters.mapM fun parameter => do
            let declared := (← lowerTypeUse declarationTypes parameter.type).typeId
            projectSpecTypeId (← instantiateTypeId instantiations declared span)
          if arguments.size != parameterTypes.size then
            failAt "LEANER-SPEC-CALL-ARITY"
              s!"specification function `{declaration.name}` expects {parameterTypes.size} argument(s), got {arguments.size}"
                (some span)
          let lowered ← arguments.zip parameterTypes |>.mapM fun (argument, type) =>
            lowerExpr context (some type) argument
          let declaredResult := (← lowerTypeUse declarationTypes declaration.result).typeId
          let resultType ← projectSpecTypeId
            (← instantiateTypeId instantiations declaredResult span)
          let id ← pushExpression loc resultType <|
            .operation (.specification (.functionCall reference {})) instantiations
              (lowered.map (·.1))
          return ← finishCall context expected id resultType loc span
      let (reference, declaration) ← resolveLocalRef segments span
      unless declaration.generics.size == typeArguments.size &&
          declaration.generics.all (·.kind == .type) do
        failAt "LEANER-CALL-GENERICS"
          "generic call arguments do not match the function's type binders" (some span)
      let instantiations ← typeArguments.mapM fun argument => do
        pure <| GenericArgument.typeArg (← lowerTypeUse context.types argument)
      let declarationTypes ← liftM <|
        typeContext declaration.generics (← namespaceOfRef? reference)
      let parameterTypes ← declaration.parameters.mapM fun parameter => do
        let declared := (← lowerTypeUse declarationTypes parameter.type).typeId
        let instantiated ← instantiateTypeId instantiations declared span
        if context.specification then projectSpecTypeId instantiated else pure instantiated
      if arguments.size != parameterTypes.size then
        failAt "LEANER-CALL-ARITY"
          s!"function `{declaration.name}` expects {parameterTypes.size} argument(s), got {arguments.size}"
            (some span)
      let lowered ← arguments.zip parameterTypes |>.mapM fun (argument, type) =>
        lowerExpr context (some type) argument
      let declaredResult := (← lowerTypeUse declarationTypes declaration.result).typeId
      let instantiatedResult ← instantiateTypeId instantiations declaredResult span
      let resultType ← if context.specification then projectSpecTypeId instantiatedResult
        else pure instantiatedResult
      let operation := if context.specification then
          Operation.specification (.functionCall reference {})
        else Operation.call (.function reference)
      let id ← pushExpression loc resultType <|
        .operation operation instantiations (lowered.map (·.1))
      finishCall context expected id resultType loc span
  | .typedCall segments result arguments _ =>
      unless ← isExternalPath segments do
        failAt "LEANER-TYPED-CALL"
          "explicit call result types are reserved for external functions" (some span)
      let reference ← resolveExternalRef segments
      let resultType := (← lowerTypeUse context.types result).typeId
      let resultType ← if context.specification then projectSpecTypeId resultType
        else pure resultType
      let lowered ← arguments.mapM (lowerExpr context none)
      let state ← get
      let namespaceRef := state.tables.namespaces[reference.namespaceId.index]?
      let functionName := state.tables.names[reference.name.index]?.map (·.name)
      let instantiations := match namespaceRef, functionName with
        | some namespaceRef, some functionName =>
            (standardCallInferredTypes? state.tables
              (some state.sourceNamespace.profile.toLIR) namespaceRef functionName
              (lowered.map (·.2))).map (·.map fun typeId =>
                GenericArgument.typeArg { typeId, loc }) |>.getD #[]
        | _, _ => #[]
      let operation := if context.specification then
          Operation.specification (.functionCall reference {})
        else Operation.call (.function reference)
      let id ← pushExpression loc resultType <|
        .operation operation instantiations (lowered.map (·.1))
      finishCall context expected id resultType loc span
  | .typedGenericCall segments result typeArguments arguments _ =>
      unless ← isExternalPath segments do
        failAt "LEANER-TYPED-CALL"
          "explicit call result types are reserved for external functions" (some span)
      let reference ← resolveExternalRef segments
      let instantiations ← typeArguments.mapM fun argument => do
        pure <| GenericArgument.typeArg (← lowerTypeUse context.types argument)
      let resultType := (← lowerTypeUse context.types result).typeId
      let resultType ← if context.specification then projectSpecTypeId resultType
        else pure resultType
      let lowered ← arguments.mapM (lowerExpr context none)
      let operation := if context.specification then
          Operation.specification (.functionCall reference {})
        else Operation.call (.function reference)
      let id ← pushExpression loc resultType <|
        .operation operation instantiations (lowered.map (·.1))
      finishCall context expected id resultType loc span
  | .call segments arguments _ =>
      let localName := segments.back?.getD ""
      -- `!` is accepted as part of a source identifier, so the legacy Move
      -- anchor spellings may arrive through the ordinary call node instead
      -- of the dedicated parser alternative.  They remain intrinsic
      -- specification operations in LIR.
      if context.specification && segments.size == 1 then
        if localName == "save_state_anchor!" ||
            localName == "folds_capture_anchor!" then
          let #[.integer label _] := arguments
            | failAt "LEANER-SPEC-ANCHOR"
                s!"`{localName}` expects one nonnegative numeric label" (some span)
          if label < 0 then
            failAt "LEANER-SPEC-ANCHOR"
              s!"`{localName}` expects a nonnegative numeric label" (some span)
          let boolType ← internType .bool
          if let some expected := expected then ensureType expected boolType span
          let operation := if localName == "save_state_anchor!" then
              SpecOperation.saveStateAnchor label.toNat
            else SpecOperation.foldsCaptureAnchor label.toNat
          let id ← pushExpression loc boolType <|
            .operation (.specification operation) #[] #[]
          return (id, boolType)
        if localName == "with_state_anchor!" then
          let #[.integer label _, value] := arguments
            | failAt "LEANER-SPEC-ANCHOR"
                "`with_state_anchor!` expects a nonnegative numeric label and a value"
                (some span)
          if label < 0 then
            failAt "LEANER-SPEC-ANCHOR"
              "`with_state_anchor!` expects a nonnegative numeric label" (some span)
          let value ← lowerExpr context expected value
          let id ← pushExpression loc value.2 <|
            .operation (.specification (.withStateAnchor label.toNat)) #[] #[value.1]
          return (id, value.2)
      let state ← get
      let hasLocalTarget :=
        (sourceFunction? state.sourceNamespace localName).isSome ||
          (sourceSpecFunction? state.sourceNamespace localName).isSome
      let specificationBuiltin? : Option SpecOperation :=
        if context.specification && segments.size == 1 && !hasLocalTarget then
          match localName with
          | "vec" => some .singletonVector
          | "update" => some .updateVector
          | "concat" => some .concatVector
          | "index_of" => some .indexOfVector
          | "contains" => some .containsVector
          | "in_range" => some .inVectorRange
          | "range" => some .vectorRange
          | "len" => some .lengthVector
          | _ => none
        else none
      if let some operation := specificationBuiltin? then
        let integerType ← internType (.integer .unbounded true)
        let boolType ← internType .bool
        let rangeType ← internType .range
        let expectedVectorElement : Option TypeId ← match expected with
          | some expected => match ← typeNode? expected with
            | some (.vector element _) => pure (some element)
            | _ => pure none
          | none => pure none
        if operation == .singletonVector then
          unless arguments.size == 1 do
            failAt "LEANER-SPEC-ARITY" "`vec` expects one operand" (some span)
          let element ← lowerExpr context expectedVectorElement arguments[0]!
          let vectorType ← match expected, expectedVectorElement with
            | some expected, some _ => pure expected
            | _, _ => internType (.vector element.2 none)
          if let some expected := expected then ensureType expected vectorType span
          let instantiation := GenericArgument.typeArg { typeId := element.2, loc }
          let id ← pushExpression loc vectorType <| .operation
            (.specification .singletonVector) #[instantiation] #[element.1]
          return (id, vectorType)
        let expectedArity := match operation with
          | .updateVector => 3
          | .concatVector | .indexOfVector | .containsVector | .inVectorRange => 2
          | .vectorRange | .lengthVector => 1
          | _ => 0
        unless arguments.size == expectedArity do
          failAt "LEANER-SPEC-ARITY"
            s!"specification builtin `{localName}` expects {expectedArity} operand(s)"
              (some span)
        let collection ← lowerExpr context none arguments[0]!
        let some (.vector elementType _) ← typeNode? collection.2
          | failAt "LEANER-SPEC-VECTOR"
              s!"specification builtin `{localName}` requires a vector operand" (some span)
        let elementUse := GenericArgument.typeArg { typeId := elementType, loc }
        let mut lowered := #[collection.1]
        let resultType ← match operation with
          | .updateVector => do
              let index ← lowerExpr context (some integerType) arguments[1]!
              let element ← lowerExpr context (some elementType) arguments[2]!
              lowered := lowered.push index.1 |>.push element.1
              pure collection.2
          | .concatVector => do
              let other ← lowerExpr context (some collection.2) arguments[1]!
              lowered := lowered.push other.1
              pure collection.2
          | .indexOfVector | .containsVector => do
              let element ← lowerExpr context (some elementType) arguments[1]!
              lowered := lowered.push element.1
              pure <| if operation == .containsVector then boolType else integerType
          | .inVectorRange => do
              let index ← lowerExpr context (some integerType) arguments[1]!
              lowered := lowered.push index.1
              pure boolType
          | .vectorRange => pure rangeType
          | .lengthVector => pure integerType
          | _ => unreachable!
        if let some expected := expected then ensureType expected resultType span
        let id ← pushExpression loc resultType <| .operation
          (.specification operation) #[elementUse] lowered
        return (id, resultType)
      let explicitSpec := if context.specification then
          specFunctionForPath? state segments
        else none
      match explicitSpec with
      | some _ =>
          let (reference, declaration) ← resolveLocalSpecRef segments
          let sourceTypes ← typeContext declaration.generics (← namespaceOfRef? reference)
          if arguments.size != declaration.parameters.size then
            failAt "LEANER-SPEC-CALL-ARITY"
              s!"specification function `{declaration.name}` expects {declaration.parameters.size} argument(s), got {arguments.size}"
                (some span)
          unless declaration.generics.all (·.kind == .type) do
            failAt "LEANER-SPEC-CALL-GENERICS"
              "only inferred type arguments are supported on specification-function calls"
              (some span)
          let physicalParameterTypes ← declaration.parameters.mapM fun parameter =>
            (lowerTypeUse sourceTypes parameter.type).map (·.typeId)
          let parameterTypes ← physicalParameterTypes.mapM projectSpecTypeId
          let mut inferred : Array (Option TypeId) :=
            Array.replicate declaration.generics.size none
          let mut lowered := #[]
          for (argument, parameterType) in arguments.zip parameterTypes do
            let value ← lowerExpr context
              (if declaration.generics.isEmpty then some parameterType else none) argument
            inferred ← inferTypeArguments context.specification inferred
              parameterType value.2 argument.span
            lowered := lowered.push value
          let rawResult := (← lowerTypeUse sourceTypes declaration.result).typeId
          let projectedResult ← projectSpecTypeId rawResult
          if let some expected := expected then
            inferred ← inferTypeArguments context.specification inferred
              projectedResult expected span
          unless inferred.all (·.isSome) do
            failAt "LEANER-SPEC-CALL-INFERENCE"
              s!"cannot infer every type argument of specification function `{declaration.name}`"
              (some span)
          let instantiationLoc ← addLoc span
          let instantiations := inferred.map fun type =>
            GenericArgument.typeArg { typeId := type.get!, loc := instantiationLoc }
          -- A specification function's declared types are already logical;
          -- what instantiation substitutes into them is not, so the argument
          -- a type parameter stands for is projected after substitution, the
          -- same way the result is.
          let finalLowered ← (arguments.zip physicalParameterTypes).mapM
            fun (argument, parameterType) => do
              lowerExpr context
                (some (← projectSpecTypeId
                  (← instantiateTypeId instantiations parameterType span))) argument
          let resultType ← projectSpecTypeId
            (← instantiateTypeId instantiations rawResult span)
          let operation := Operation.specification (.functionCall reference {})
          let id ← pushExpression loc resultType <|
            .operation operation instantiations (finalLowered.map (·.1))
          finishCall context expected id resultType loc span
      | none =>
          let (reference, declaration) ← resolveLocalRef segments span
          unless declaration.generics.all (·.kind == .type) do
            failAt "LEANER-CALL-GENERICS"
              "only inferred type arguments are supported on direct function calls" (some span)
          let sourceTypes ← typeContext declaration.generics (← namespaceOfRef? reference)
          let physicalParameterTypes ← declaration.parameters.mapM fun parameter =>
            (lowerTypeUse sourceTypes parameter.type).map (·.typeId)
          if arguments.size != physicalParameterTypes.size then
            failAt "LEANER-CALL-ARITY"
              s!"function `{declaration.name}` expects {physicalParameterTypes.size} argument(s), got {arguments.size}"
                (some span)
          let parameterTypes ← if context.specification then
              physicalParameterTypes.mapM projectSpecTypeId
            else pure physicalParameterTypes
          let mut inferred : Array (Option TypeId) :=
            Array.replicate declaration.generics.size none
          let mut lowered := #[]
          for (argument, parameterType) in arguments.zip parameterTypes do
            let value ← lowerExpr context
              (if declaration.generics.isEmpty then some parameterType else none) argument
            inferred ← inferTypeArguments context.specification inferred
              parameterType value.2 argument.span
            lowered := lowered.push value
          let physicalResult := (← lowerTypeUse sourceTypes declaration.result).typeId
          let projectedResult ← if context.specification then projectSpecTypeId physicalResult
            else pure physicalResult
          if let some expected := expected then
            inferred ← inferTypeArguments context.specification inferred
              projectedResult expected span
          unless inferred.all (·.isSome) do
            failAt "LEANER-CALL-INFERENCE"
              s!"cannot infer every type argument of function `{declaration.name}`" (some span)
          let instantiationLoc ← addLoc span
          let instantiations := inferred.map fun type =>
            GenericArgument.typeArg { typeId := type.get!, loc := instantiationLoc }
          -- Projection does not commute with instantiation: a type parameter
          -- projects to itself, so projecting `V` and then substituting `u64`
          -- keeps a physical type where the call denotes a logical value.
          -- Substitute first and project the concrete type.
          let finalLowered ← (arguments.zip physicalParameterTypes).mapM
            fun (argument, parameterType) => do
              let instantiated ← instantiateTypeId instantiations parameterType span
              let expectedType ← if context.specification then
                  projectSpecTypeId instantiated
                else pure instantiated
              lowerExpr context (some expectedType) argument
          let instantiatedResult ← instantiateTypeId instantiations physicalResult span
          let resultType ← if context.specification then
              projectSpecTypeId instantiatedResult
            else pure instantiatedResult
          let operation := if context.specification then
              Operation.specification (.functionCall reference {})
            else Operation.call (.function reference)
          let id ← pushExpression loc resultType <|
            .operation operation instantiations (finalLowered.map (·.1))
          finishCall context expected id resultType loc span
  | .block statements result _ =>
      let unitType ← internType .unit
      -- Canonical source attaches a loop specification using a trailing
      -- `where` region. LIR keeps the verifier-oriented order, with logical
      -- bindings and invariant assumptions before the loop. Accept the
      -- source order here and restore that semantic order while lowering, so
      -- render/parse/render remains lossless.
      let isLoopSpecification (conditions : Array (SpecificationConditionKind × Expr)) : Bool :=
        !conditions.isEmpty &&
          conditions.any (fun condition => condition.1 == .loopInvariant) &&
          conditions.all fun condition => match condition.1 with
            | .let_ _ | .loopInvariant => true
            | _ => false
      -- A final specification is parsed as the block result. Include it
      -- in the same normalization as statement-position annotations.
      let (statements, result) := match result with
        | some spec@(.specBlock conditions _) =>
            if isLoopSpecification conditions then
              (statements.push (.expression spec), none)
            else (statements, result)
        | _ => (statements, result)
      let rec normalizeLoopSpecifications : List Statement → List Statement
        | .expression loopExpr@(.loop ..) ::
            .expression specExpr@(.specBlock conditions _) :: rest =>
            if isLoopSpecification conditions then
              .expression specExpr :: .expression loopExpr ::
                normalizeLoopSpecifications rest
            else
              .expression loopExpr :: normalizeLoopSpecifications
                (.expression specExpr :: rest)
        | statement :: rest => statement :: normalizeLoopSpecifications rest
        | [] => []
      let rec lowerTail (active : ExprContext) (remaining : List Statement) :
          LowerM (ExprId × TypeId) := do
        match remaining with
        | [] => match result with
            | some returned@(.return_ sourceResult _) =>
                if functionTail then lowerExpr active expected sourceResult
                else lowerExpr active expected returned
            | some sourceResult =>
                if expected == some unitType then
                  let value ← lowerExpr { active with functionTail } none sourceResult
                  let neverType ← internType .never
                  if value.2 == neverType || sourceResult matches .unit _ then
                    return value
                  let discarded ← pushExpression loc unitType (.block #[value.1] none)
                  return (discarded, unitType)
                else
                  lowerExpr { active with functionTail } expected sourceResult
            | none =>
                if let some expected := expected then ensureType expected unitType span
                let empty ← pushExpression loc unitType (.block #[] none)
                return (empty, unitType)
        | .expression statement :: rest =>
            -- A semicolon discards its expression's value; it does not
            -- require the expression itself to produce Unit. Move commonly
            -- discards values returned by mutating vector operations.
            let statement ← lowerExpr active none statement
            let neverType ← internType .never
            let terminalReturn := match remaining with
              | [.expression (.return_ ..)] => true
              | _ => false
            if result.isNone && (statement.2 == neverType || terminalReturn) then
              return (← pushExpression loc neverType (.block #[statement.1] none), neverType)
            let tail ← lowerTail active rest
            -- Consecutive offside entries are one semantic sequence.  The
            -- recursive lowering above is convenient for declarations and
            -- their scopes, but leaving an expression-only tail as nested
            -- singleton blocks makes `print -> elaborate -> print` insert an
            -- otherwise meaningless inner `do`.  Flatten that administrative
            -- tail to the same block shape used by compiler-v2 XAST.
            let state ← get
            let block ← match state.output.expressions[tail.1.index]? with
              | some { kind := .block tailStatements tailResult, .. } =>
                  pushExpression loc tail.2 <|
                    .block (#[statement.1] ++ tailStatements) tailResult
              | _ =>
                  pushExpression loc tail.2 (.block #[statement.1] (some tail.1))
            return (block, tail.2)
        | .letDecl _ pattern annotatedType value declarationSpan :: rest =>
            let annotatedType ← annotatedType.mapM fun annotatedType => do
              let typeId := (← lowerTypeUse active.types annotatedType).typeId
              if active.specification then projectSpecTypeId typeId else pure typeId
            -- Predeclaration already inferred every binding's scoped type.
            -- Feed that type back into an unannotated scalar binding so a
            -- derived specification body can insert the same physical-to-
            -- logical projection used by its local declaration.
            let inferredType? := match pattern with
              | .variable name patternSpan _ =>
                  (declaredLocal? active patternSpan name).map (·.typeId)
              | _ => none
            let initializer ← lowerExpr active (annotatedType.orElse fun _ => inferredType?) value
            let patternType := annotatedType.getD initializer.2
            let declarationLoc ← addLoc declarationSpan
            let loweredPattern ← lowerBindingPattern active pattern patternType
            let boundLocals ← bindingPatternLocals active pattern
            let extended := withBoundLocals active boundLocals
            let tail ← lowerTail extended rest
            let binding ← pushExpression declarationLoc tail.2 <|
              .letDecl loweredPattern (some initializer.1) tail.1
            return (binding, tail.2)
      lowerTail context (normalizeLoopSpecifications statements.toList)
  | .ifElse condition thenBranch elseBranch _ =>
      let boolType ← internType .bool
      let condition ← lowerExpr context (some boolType) condition
      let thenBranch ← lowerExpr { context with functionTail } expected thenBranch
      let neverType ← internType .never
      let branchExpected := if thenBranch.2 == neverType then expected else some thenBranch.2
      let elseBranch ← elseBranch.mapM (lowerExpr { context with functionTail } branchExpected)
      let typeId ← match elseBranch with
        | some branch =>
            if thenBranch.2 == neverType then pure branch.2
            else if branch.2 == neverType then pure thenBranch.2
            else ensureType thenBranch.2 branch.2 span *> pure thenBranch.2
        | none =>
            let unitType ← internType .unit
            -- An `if` used for an early return has no normal value from its
            -- taken branch.  As in the two-branch case above, `never`
            -- coerces to the surrounding Unit statement rather than forcing
            -- source to add the administrative `else ()`.
            unless thenBranch.2 == neverType do
              ensureType unitType thenBranch.2 span
            pure unitType
      let id ← pushExpression loc typeId <| .ifElse condition.1 thenBranch.1 (elseBranch.map (·.1))
      return (id, typeId)
  | .match_ scrutineeSource arms _ =>
      if arms.isEmpty then
        failAt "LEANER-MATCH-ARMS" "a match expression must contain at least one arm" (some span)
      let scrutinee ← lowerExpr context none scrutineeSource
      let boolType ← internType .bool
      let neverType ← internType .never
      -- Closed Boolean (possibly tuple) patterns form a finite decision
      -- tree. Evaluate the scrutinee once, even when it contains a call.
      -- Coverage counts distinct literals, without enumerating the domain.
      let rec boolCardinality (typeId : TypeId) : LowerM (Option Nat) := do
        match ← typeNode? typeId with
        | some .bool => pure (some 2)
        | some (.tuple children) => do
            let mut count := 1
            for child in children do
              let some childCount ← boolCardinality child | return none
              count := min (arms.size + 1) (count * childCount)
            pure (some count)
        | _ => pure none
      let rec closedLiteral (pattern : BindingPattern) : Option ConstValue := do
        match pattern with
        | .literal (.bool value) _ => some (.bool value)
        | .tuple children _ => some (.tuple (← children.mapM closedLiteral))
        | _ => none
      if !context.specification && arms.all (fun (pattern, guard, _) =>
          guard.isNone && (pattern matches .wildcard .. || (closedLiteral pattern).isSome)) then
        if let some cardinality ← boolCardinality scrutinee.2 then
          let mut literals : Array ConstValue := #[]
          let mut wildcard := false
          for (pattern, _, _) in arms do
            let _ ← lowerBindingPattern context pattern scrutinee.2
            match closedLiteral pattern with
            | some literal => unless literals.contains literal do literals := literals.push literal
            | none => wildcard := true
          unless wildcard || literals.size == cardinality do
            failAt "LEANER-BOOL-MATCH-COVERAGE"
              "a Boolean match must cover every value or include a wildcard" (some span)
          let slot ← pushTemporaryLocal scrutinee.2 loc
          let binding ← pushPattern { loc, typeId := scrutinee.2, kind := .variable slot }
          let read ← pushExpression loc scrutinee.2 (.localVar slot)
          let mut resultType? := expected
          let mut branches : Array (Option ExprId × ExprId) := #[]
          for (pattern, _, body) in arms do
            let condition ← match closedLiteral pattern with
              | none => pure none
              | some literal => do
                  let value ← pushExpression loc scrutinee.2 (.value literal)
                  some <$> pushExpression loc boolType
                    (.operation (.primitive .equal) #[] #[read, value])
            let body ← lowerExpr { context with functionTail } resultType? body
            if let some resultType := resultType? then ensureType resultType body.2 span
            else unless body.2 == neverType do resultType? := some body.2
            branches := branches.push (condition, body.1)
          let typeId := resultType?.getD neverType
          let mut result := branches.back!.2
          for (condition, body) in branches.pop.reverse do
            result ← match condition with
              | none => pure body
              | some condition => pushExpression loc typeId (.ifElse condition body (some result))
          return (← pushExpression loc typeId (.letDecl binding (some scrutinee.1) result), typeId)
      let lowerOrdinary : LowerM (ExprId × TypeId) := do
        let mut resultType? := expected
        let mut loweredArms : Array MatchArm := #[]
        for (pattern, guard, body) in arms do
          let loweredPattern ← lowerBindingPattern context pattern scrutinee.2
          let boundLocals ← bindingPatternLocals context pattern
          let armContext := withBoundLocals context boundLocals
          let loweredGuard ← guard.mapM (lowerExpr armContext (some boolType))
          let loweredBody ← lowerExpr { armContext with functionTail } resultType? body
          match resultType? with
          | some resultType => ensureType resultType loweredBody.2 body.span
          | none => unless loweredBody.2 == neverType do resultType? := some loweredBody.2
          loweredArms := loweredArms.push {
            pattern := loweredPattern
            guard := loweredGuard.map (·.1)
            body := loweredBody.1 }
        let typeId := resultType?.getD neverType
        let id ← pushExpression loc typeId <| .match_ scrutinee.1 loweredArms
        return (id, typeId)
      match context.specification, scrutineeSource, ← typeNode? scrutinee.2 with
      | false, .local _ _, some (.nominal _ _) =>
          /- A by-value match on a copyable enum uses the same closed
          decision tree as a reference match.  Variant tests inspect the
          original local and payload variables remain symbolic field
          selections, so no match interpreter or administrative payload
          locals enter the native denotation.  A move-only enum retains the
          ordinary match node until V1 has a consuming match combinator. -/
          let (declaration, _) ← nominalPatternDeclaration scrutinee.2 span
          if declaration.variants.isEmpty ||
              !declaration.abilities.contains .copy ||
              !(← uniformEnumFieldTypes declaration) then
            lowerOrdinary
          else do
            let mut coveredVariants : Array String := #[]
            let mut hasWildcard := false
            for ((pattern, _, _), index) in arms.zipIdx do
              match pattern with
              | .wildcard patternSpan =>
                  if index + 1 < arms.size then
                    failAt "LEANER-VALUE-MATCH-COVERAGE"
                      "a wildcard arm in a by-value match must be last"
                      (some patternSpan)
                  hasWildcard := true
              | .constructor owner variant _ patternSpan =>
                  let (owner, variant) ← resolvedPatternConstructor owner variant
                  let some variant := variant
                    | failAt "LEANER-VALUE-MATCH-PATTERN"
                        "a by-value enum pattern must name a variant"
                        (some patternSpan)
                  let ownerType := (← lowerTypeUse context.types owner).typeId
                  ensureType scrutinee.2 ownerType patternSpan
                  coveredVariants := coveredVariants.push variant
              | _ =>
                  failAt "LEANER-VALUE-MATCH-PATTERN"
                    "a by-value enum match requires constructor or wildcard arms"
                    (some pattern.span)
            unless hasWildcard || declaration.variants.all
                (fun variant => coveredVariants.contains variant.name) do
              let missing := declaration.variants.filterMap fun variant =>
                if coveredVariants.contains variant.name then none else some variant.name
              failAt "LEANER-VALUE-MATCH-COVERAGE"
                s!"a by-value enum match must cover every variant (missing: \
                  {String.intercalate ", " missing.toList})"
                (some span)
            let mut resultType? := expected
            let mut lowered : Array (Option ExprId × ExprId) := #[]
            for (pattern, guard, body) in arms do
              unless guard.isNone do
                failAt "LEANER-VALUE-MATCH-GUARD"
                  "guards on by-value enum matches are not supported yet"
                  (some pattern.span)
              let boundLocals ← bindingPatternLocals context pattern
              let mut aliases : Array (String × Expr) := #[]
              let condition? ← match pattern with
                | .wildcard _ => pure none
                | .constructor owner variant fields patternSpan => do
                    let (owner, variant) ← resolvedPatternConstructor owner variant
                    let some variant := variant
                      | failAt "LEANER-VALUE-MATCH-PATTERN"
                          "a by-value enum pattern must name a variant"
                          (some patternSpan)
                    let ownerType := (← lowerTypeUse context.types owner).typeId
                    ensureType scrutinee.2 ownerType patternSpan
                    let (_, _, declaredFields) ←
                      nominalPatternFields ownerType (some variant) patternSpan
                    unless fields.size == declaredFields.size && declaredFields.all fun declared =>
                        fields.any (·.1 == declared.name) do
                      failAt "LEANER-PATTERN-FIELDS"
                        "a constructor pattern must cover every field exactly once"
                        (some patternSpan)
                    for declared in declaredFields do
                      let some child := fields.find? (·.1 == declared.name) | unreachable!
                      match child.2 with
                      | .wildcard _ => pure ()
                      | .variable name childSpan _ =>
                          let some localDecl := declaredLocal? context childSpan name
                            | failAt "LEANER-LOCAL-DECLARATION"
                                s!"pattern local `{name}` was not predeclared"
                                (some childSpan)
                          let (_, _, fieldType) ←
                            resolveNominalField scrutinee.2 declared.name childSpan
                          ensureType localDecl.typeId fieldType childSpan
                          aliases := aliases.push
                            (name, .field scrutineeSource declared.name childSpan)
                      | _ =>
                          failAt "LEANER-VALUE-MATCH-PATTERN"
                            "by-value enum payloads currently bind variables or wildcards"
                            (some child.2.span)
                    let condition ← lowerExpr context (some boolType) <|
                      .variantTest scrutineeSource #[variant] patternSpan
                    pure (some condition.1)
                | _ =>
                    failAt "LEANER-VALUE-MATCH-PATTERN"
                      "a by-value enum match requires constructor or wildcard arms"
                      (some pattern.span)
              let armContext := withBoundLocals context boundLocals
              let armContext := { armContext with
                patternAliases := aliases ++ armContext.patternAliases }
              let loweredBody ← lowerExpr { armContext with functionTail } resultType? body
              match resultType? with
              | some resultType => ensureType resultType loweredBody.2 body.span
              | none => unless loweredBody.2 == neverType do resultType? := some loweredBody.2
              lowered := lowered.push (condition?, loweredBody.1)
            let typeId := resultType?.getD neverType
            let mut result? : Option ExprId := none
            for (condition?, armBody) in lowered.reverse do
              result? ← match result?, condition? with
                | none, _ => pure (some armBody)
                | some _, none => pure (some armBody)
                | some fallback, some condition => do
                    let branch ← pushExpression loc typeId <|
                      .ifElse condition armBody (some fallback)
                    pure (some branch)
            let some result := result? | unreachable!
            return (result, typeId)
      | false, .local _ _, some (.reference reference) =>
          -- A constructor match through a reference binds references to the
          -- selected payloads. Lower it to the already-native decision tree:
          -- one closed variant test per arm. Shared payloads stay aliases;
          -- mutable payloads get one stable focused reborrow each, so reads
          -- and writes use the same loan rather than opening overlapping loans.
          let (declaration, _) ← nominalPatternDeclaration reference.referent span
          if declaration.variants.isEmpty then
            failAt "LEANER-REFERENCE-MATCH-PATTERN"
              "a match through a reference requires an enum scrutinee" (some span)
          let mut coveredVariants : Array String := #[]
          let mut hasWildcard := false
          for ((pattern, _, _), index) in arms.zipIdx do
            match pattern with
            | .wildcard patternSpan =>
                if index + 1 < arms.size then
                  failAt "LEANER-REFERENCE-MATCH-COVERAGE"
                    "a wildcard arm in a reference match must be last"
                    (some patternSpan)
                hasWildcard := true
            | .constructor owner variant _ patternSpan =>
                let (owner, variant) ← resolvedPatternConstructor owner variant
                let some variant := variant
                  | failAt "LEANER-REFERENCE-MATCH-PATTERN"
                      "a reference-pattern match must name an enum variant"
                      (some patternSpan)
                let ownerType := (← lowerTypeUse context.types owner).typeId
                ensureType reference.referent ownerType patternSpan
                coveredVariants := coveredVariants.push variant
            | _ =>
                failAt "LEANER-REFERENCE-MATCH-PATTERN"
                  "a reference match requires enum-constructor or wildcard arms"
                  (some pattern.span)
          unless hasWildcard || declaration.variants.all
              (fun variant => coveredVariants.contains variant.name) do
            let missing := declaration.variants.filterMap fun variant =>
              if coveredVariants.contains variant.name then none else some variant.name
            failAt "LEANER-REFERENCE-MATCH-COVERAGE"
              s!"a match through a reference must cover every variant (missing: \
                {String.intercalate ", " missing.toList})"
              (some span)
          let mut resultType? := expected
          let mut lowered : Array (Option ExprId × ExprId) := #[]
          for (pattern, guard, body) in arms do
            unless guard.isNone do
              failAt "LEANER-REFERENCE-MATCH-GUARD"
                "guards on reference-pattern matches are not supported yet"
                (some pattern.span)
            let boundLocals ← bindingPatternLocals context pattern
            let mut aliases : Array (String × Expr) := #[]
            let mut payloadBindings : Array (PatternId × ExprId) := #[]
            let condition? ← match pattern with
              | .wildcard _ => pure none
              | .constructor owner variant fields patternSpan => do
                  let (owner, variant) ← resolvedPatternConstructor owner variant
                  let some variant := variant
                    | failAt "LEANER-REFERENCE-MATCH-PATTERN"
                        "a reference-pattern match must name an enum variant"
                        (some patternSpan)
                  let ownerType := (← lowerTypeUse context.types owner).typeId
                  ensureType reference.referent ownerType patternSpan
                  let (_, _, declaredFields) ←
                    nominalPatternFields ownerType (some variant) patternSpan
                  unless fields.size == declaredFields.size && declaredFields.all fun declared =>
                      fields.any (·.1 == declared.name) do
                    failAt "LEANER-PATTERN-FIELDS"
                      "a constructor pattern must cover every field exactly once"
                      (some patternSpan)
                  for declared in declaredFields do
                    let some child := fields.find? (·.1 == declared.name) | unreachable!
                    match child.2 with
                    | .wildcard _ => pure ()
                    | .variable name childSpan _ =>
                        let some localDecl := declaredLocal? context childSpan name
                          | failAt "LEANER-LOCAL-DECLARATION"
                              s!"pattern local `{name}` was not predeclared"
                              (some childSpan)
                        let (_, _, fieldType) ←
                          resolveNominalField reference.referent declared.name childSpan
                        let some (.reference localReference) ←
                            typeNode? localDecl.typeId
                          | failAt "LEANER-REFERENCE-MATCH-BINDER"
                              "a reference-pattern payload must have reference type"
                              (some childSpan)
                        ensureType localReference.referent fieldType childSpan
                        unless localReference.kind == reference.kind do
                          failAt "LEANER-REFERENCE-MATCH-BINDER"
                            "a payload reference changes the scrutinee's reference kind"
                            (some childSpan)
                        if reference.kind == .mutable then
                          let selected ← lowerExpr context (some localDecl.typeId) <|
                            .borrowValue true
                              (.field (.dereference scrutineeSource childSpan)
                                declared.name childSpan) childSpan
                          let binding ← lowerBindingPattern context child.2 localDecl.typeId
                          payloadBindings := payloadBindings.push (binding, selected.1)
                        else
                          aliases := aliases.push
                            (name, .field scrutineeSource declared.name childSpan)
                    | _ =>
                        failAt "LEANER-REFERENCE-MATCH-PATTERN"
                          "reference payloads currently bind variables or wildcards"
                          (some child.2.span)
                  let condition ← lowerExpr context (some boolType) <|
                    .variantTest scrutineeSource #[variant] patternSpan
                  pure (some condition.1)
              | _ =>
                  failAt "LEANER-REFERENCE-MATCH-PATTERN"
                    "a reference match requires enum-constructor or wildcard arms"
                    (some pattern.span)
            let armContext := withBoundLocals context boundLocals
            let armContext := { armContext with
              patternAliases := aliases ++ armContext.patternAliases }
            let loweredBody ← lowerExpr { armContext with functionTail } resultType? body
            let mut armBody := loweredBody.1
            for (binding, value) in payloadBindings.reverse do
              armBody ← pushExpression loc loweredBody.2 <| .letDecl binding (some value) armBody
            match resultType? with
            | some resultType => ensureType resultType loweredBody.2 body.span
            | none => unless loweredBody.2 == neverType do resultType? := some loweredBody.2
            lowered := lowered.push (condition?, armBody)
          let typeId := resultType?.getD neverType
          let mut result? : Option ExprId := none
          for (condition?, armBody) in lowered.reverse do
            result? ← match result?, condition? with
              | none, _ => pure (some armBody)
              | some _, none => pure (some armBody)
              | some fallback, some condition => do
                  let branch ← pushExpression loc typeId <|
                    .ifElse condition armBody (some fallback)
                  pure (some branch)
          let some result := result? | unreachable!
          return (result, typeId)
      | _, _, _ => lowerOrdinary
  | .forRange iterator lower upper body _ =>
      let unitType ← internType .unit
      if let some expected := expected then ensureType expected unitType span
      let lowerName := forLowerName span
      let upperName := forUpperName span
      let lowerPatternSource := BindingPattern.variable lowerName span
      let iteratorPatternSource := BindingPattern.variable iterator span
      let upperPatternSource := BindingPattern.variable upperName span

      -- Preserve Move/Rust range semantics: evaluate each bound exactly once,
      -- then introduce the iterator and lower the surface loop to ordinary
      -- core let/loop/assignment nodes.
      let some lowerDecl := declaredLocal? context span lowerName
        | failAt "LEANER-FOR-LOWER" "a `for` lower bound was not predeclared" (some span)
      let lowerValue ← lowerExpr context (some lowerDecl.typeId) lower
      unless lowerDecl.typeId == lowerValue.2 do
        let declaredType ← typeNode? lowerDecl.typeId
        let loweredType ← typeNode? lowerValue.2
        failAt "LEANER-FOR-LOWER-TYPE"
          s!"predeclared lower bound type {repr declaredType} differs from lowered type {repr loweredType}"
          (some span)
      let lowerPattern ← lowerBindingPattern context lowerPatternSource lowerValue.2
      let lowerLocals ← bindingPatternLocals context lowerPatternSource
      let lowerContext := withBoundLocals context lowerLocals
      let iteratorValue ← lowerExpr lowerContext (some lowerValue.2) (.local lowerName span)
      let some iteratorDecl := declaredLocal? lowerContext span iterator
        | failAt "LEANER-FOR-ITERATOR" "a `for` iterator was not predeclared" (some span)
      unless iteratorDecl.typeId == lowerValue.2 do
        failAt "LEANER-FOR-ITERATOR-TYPE"
          s!"predeclared iterator type {iteratorDecl.typeId.index} differs from bound type {lowerValue.2.index}"
          (some span)
      let iteratorPattern ←
        lowerBindingPattern lowerContext iteratorPatternSource lowerValue.2
      let iteratorLocals ← bindingPatternLocals lowerContext iteratorPatternSource
      let iteratorContext := withBoundLocals lowerContext iteratorLocals
      let upperValue ← lowerExpr iteratorContext (some lowerValue.2) upper
      let some upperDecl := declaredLocal? iteratorContext span upperName
        | failAt "LEANER-FOR-UPPER" "a `for` upper bound was not predeclared" (some span)
      unless upperDecl.typeId == upperValue.2 do
        failAt "LEANER-FOR-UPPER-TYPE"
          s!"predeclared upper bound type {upperDecl.typeId.index} differs from lowered type {upperValue.2.index}"
          (some span)
      let upperPattern ← lowerBindingPattern iteratorContext upperPatternSource upperValue.2
      let upperLocals ← bindingPatternLocals iteratorContext upperPatternSource
      let loopContext := { (withBoundLocals iteratorContext upperLocals) with
        loopResults := #[unitType] ++ iteratorContext.loopResults,
        loopLabels := #[none] ++ iteratorContext.loopLabels }

      let body ← lowerExpr loopContext (some unitType) body
      let neverType ← internType .never
      unless body.2 == unitType || body.2 == neverType do
        failAt "LEANER-FOR-BODY" "a `for` body must produce Unit or never return"
          (some span)
      let bodyId ← incrementForContinues body.1 iteratorPattern iteratorDecl.id
        lowerValue.2 context.specification ((← get).output.expressions.size + 1)
      let body := (bodyId, body.2)
      let boolType ← internType .bool
      let conditionIterator ← pushExpression loc lowerValue.2 <|
        .localVar iteratorDecl.id
      let conditionUpper ← pushExpression loc upperValue.2 <| .localVar upperDecl.id
      let condition ← pushExpression loc boolType <|
        .operation (.primitive .less) #[] #[conditionIterator, conditionUpper]
      let increment ← pushForIncrement iteratorPattern iteratorDecl.id lowerValue.2 loc
        context.specification
      let iteration ← if body.2 == neverType then pure body.1 else
        pushExpression loc unitType <| .block #[body.1] (some increment)
      let stop ← pushExpression loc neverType <| .break_ 0 none
      let guarded ← pushExpression loc unitType <|
        .ifElse condition iteration (some stop)
      let loop ← pushExpression loc unitType <| .loop none guarded
      -- Keep the zero-trip path outside the invariant boundary, preserving
      -- the entry state when the range is empty. Bounds are still evaluated
      -- once, and validation still checks the complete loop body.
      let empty ← pushExpression loc unitType (.value .unit)
      let entry ← pushExpression loc unitType (.ifElse condition loop (some empty))
      let upperBinding ← pushExpression loc unitType <|
        .letDecl upperPattern (some upperValue.1) entry
      let iteratorBinding ← pushExpression loc unitType <|
        .letDecl iteratorPattern (some iteratorValue.1) upperBinding
      let lowerBinding ← pushExpression loc unitType <|
        .letDecl lowerPattern (some lowerValue.1) iteratorBinding
      return (lowerBinding, unitType)
  | .loop body _ label =>
      if (← get).sourceNamespace.profile == .move then
        if let some name := label then
          if context.loopLabels.contains (some name) then
            failAt "LEANER-LOOP-LABEL-DUPLICATE"
              s!"loop label `{name}` is already used by an outer loop" (some span)
      let unitType ← internType .unit
      let resultType := expected.getD unitType
      let loopContext := { context with
        loopResults := #[resultType] ++ context.loopResults,
        loopLabels := #[label] ++ context.loopLabels }
      let body ← lowerExpr loopContext (some unitType) body
      let neverType ← internType .never
      unless body.2 == unitType || body.2 == neverType do
        failAt "LEANER-LOOP-BODY" "a loop body must produce Unit or never return" (some span)
      let id ← pushExpression loc resultType <| .loop label body.1
      return (id, resultType)
  | .break_ value _ label =>
      let nest ← match label with
        | none => pure 0
        | some label => match context.loopLabels.findIdx? (· == some label) with
          | some nest => pure nest
          | none => failAt "LEANER-LOOP-LABEL" s!"unknown loop label `{label}`" (some span)
      let some resultType := context.loopResults[nest]?
        | failAt "LEANER-BREAK-CONTEXT" "`break` appears outside a loop" (some span)
      let unitType ← internType .unit
      let value ← match value with
        | some value => some <$> lowerExpr context (some resultType) value
        | none => do
            ensureType unitType resultType span
            pure none
      let neverType ← internType .never
      let id ← pushExpression loc neverType <| .break_ nest (value.map (·.1))
      return (id, neverType)
  | .continue_ _ label =>
      let nest ← match label with
        | none => pure 0
        | some label => match context.loopLabels.findIdx? (· == some label) with
          | some nest => pure nest
          | none => failAt "LEANER-LOOP-LABEL" s!"unknown loop label `{label}`" (some span)
      unless !context.loopResults.isEmpty do
        failAt "LEANER-CONTINUE-CONTEXT" "`continue` appears outside a loop" (some span)
      let neverType ← internType .never
      let id ← pushExpression loc neverType <| .continue_ nest
      return (id, neverType)
  | .assign place value _ =>
      let unitType ← internType .unit
      if let some expected := expected then ensureType expected unitType span
      if (← get).sourceNamespace.profile == .move then
        if let .field (.local baseName _) field _ := place then
          if let some (baseLocal, baseType) := lookupLocal? context baseName then
            if let some (.reference reference) ← typeNode? baseType then
              let base ← pushExpression loc baseType (.localVar baseLocal)
              let (owner, _, fieldType) ←
                resolveNominalField reference.referent field span
              let referenceType ← inferredReferenceType .mutable fieldType loc
              let selection : LeanerIR.DataOperation ←
                if ← nominalHasVariants reference.referent then
                  pure (.selectVariants owner #[field])
                else pure (.select owner field)
              let selected ← pushExpression loc referenceType <| .operation
                (.data selection) #[] #[base]
              let value ← lowerExpr context (some fieldType) value
              let id ← pushReferenceMutation loc unitType referenceType selected value.1
              return (id, unitType)
        if let .deref (.local baseName _) _ := place then
          if let some (baseLocal, baseType) := lookupLocal? context baseName then
            if let some (.reference reference) ← typeNode? baseType then
              let base ← pushExpression loc baseType (.localVar baseLocal)
              let value ← lowerExpr context (some reference.referent) value
              let id ← pushReferenceMutation loc unitType baseType base value.1
              return (id, unitType)
      let (place, placeType) ← lowerPlace context place
      -- The specification re-lowering of an executable body reads in the
      -- projected domain, so the stored value is expected there too.
      let placeType ← if context.specification then projectSpecTypeId placeType
        else pure placeType
      let value ← lowerExpr context (some placeType) value
      let id ← pushExpression loc unitType <| .assign place value.1
      return (id, unitType)
  | .rawAssignExpression target value _ =>
      let unitType ← internType .unit
      if let some expected := expected then ensureType expected unitType span
      let some placeSource := localStoragePlace? context target
        | failAt "LEANER-PLACE-EXPRESSION" "core.assignPlace requires a local-rooted place"
            (some span)
      let indexType ← if context.specification then internType (.integer .unbounded true)
        else runtimeIndexType
      let (place, placeType, bindings, _) ← lowerExpressionPlaceWith
        (lowerExpr context (some indexType)) context placeSource false true
      let placeType ← if context.specification then projectSpecTypeId placeType
        else pure placeType
      let value ← lowerExpr context (some placeType) value
      let id ← pushIndexedAssignment context loc unitType place value.1 placeType bindings
      return (id, unitType)
  | .assignExpression target value _ =>
      let unitType ← internType .unit
      if let some expected := expected then ensureType expected unitType span
      if (← get).sourceNamespace.profile == .move then
        if let some (head, index, fields) := storageProjection? context target then
          if (storageIndexLocalName? context head).isNone then
            if context.specification then
              -- The derived pure reading erases a global write, but still
              -- checks both operands. Local value assignments below retain
              -- their semantics; they are not discarded with global effects.
              let target ← lowerExpr context none target
              let value ← lowerExpr context (some target.2) value
              ensureType target.2 value.2 span
              return (← pushExpression loc unitType (.value .unit), unitType)
            let resource ← lowerTypeUse context.types head
            let key ← lowerExpr context none index
            let mut referent := resource.typeId
            let mut referenceType ← inferredReferenceType .mutable referent loc
            let mut selected ← pushExpression loc referenceType <| .operation
              (.global (.borrow .mutable)) #[.typeArg resource] #[key.1]
            for field in fields do
              let (owner, _, fieldType) ← resolveNominalField referent field span
              let selection : LeanerIR.DataOperation ←
                if ← nominalHasVariants referent then
                  pure (.selectVariants owner #[field])
                else
                  pure (.select owner field)
              referent := fieldType
              referenceType ← inferredReferenceType .mutable referent loc
              selected ← pushExpression loc referenceType <| .operation
                (.data selection) #[] #[selected]
            let value ← lowerExpr context (some referent) value
            let id ← pushReferenceMutation loc unitType referenceType selected value.1
            return (id, unitType)
      -- Move assigns through a reference a call returns. The target's base is
      -- then an expression rather than a place, so the field selection is
      -- itself the reference the assignment mutates.
      if (← get).sourceNamespace.profile == .move then
        if let .field base field _ := target then
          let baseValue ← lowerExpr context none base
          if let some (.reference reference) ← typeNode? baseValue.2 then
            let (owner, _, fieldType) ← resolveNominalField reference.referent field span
            let referenceType ← inferredReferenceType .mutable fieldType loc
            let selection : LeanerIR.DataOperation ←
              if ← nominalHasVariants reference.referent then
                pure (.selectVariants owner #[field])
              else pure (.select owner field)
            let selected ← pushExpression loc referenceType <| .operation
              (.data selection) #[] #[baseValue.1]
            let value ← lowerExpr context (some fieldType) value
            let id ← pushReferenceMutation loc unitType referenceType selected value.1
            return (id, unitType)
        if let .dereference base _ := target then
          let baseValue ← lowerExpr context none base
          if let some (.reference reference) ← typeNode? baseValue.2 then
            let value ← lowerExpr context (some reference.referent) value
            let id ← pushReferenceMutation loc unitType baseValue.2 baseValue.1 value.1
            return (id, unitType)
      let some placeSource := localStoragePlace? context target
        | failAt "LEANER-PLACE-EXPRESSION"
            "an assignment target must be a local, field, vector index, or global resource"
              (some span)
      let indexType ← if context.specification then internType (.integer .unbounded true)
        else runtimeIndexType
      let (place, placeType, bindings, _) ← lowerExpressionPlaceWith
        (lowerExpr context (some indexType)) context placeSource true true
      let placeType ← if context.specification then projectSpecTypeId placeType
        else pure placeType
      let value ← lowerExpr context (some placeType) value
      let id ← pushIndexedAssignment context loc unitType place value.1 placeType bindings
      return (id, unitType)
  | .assignPattern pattern annotatedType value _ =>
      let unitType ← internType .unit
      if let some expected := expected then ensureType expected unitType span
      let patternType := (← lowerTypeUse context.types annotatedType).typeId
      let patternType ← if context.specification then projectSpecTypeId patternType
        else pure patternType
      let value ← lowerExpr context (some patternType) value
      let pattern ← lowerAssignmentPattern context pattern patternType
      let id ← pushExpression loc unitType <| .assignPattern pattern value.1
      return (id, unitType)
  | .return_ value _ =>
      if context.specification || functionTail then
        lowerExpr context (some context.returnType) value
      else
        let value ← lowerExpr context (some context.returnType) value
        let never ← internType .never
        let unitType ← internType .unit
        if context.returnType == unitType then
          let returned ← pushExpression loc never (.return_ #[])
          if (← get).output.expressions[value.1.index]!.kind matches .value .unit then
            return (returned, never)
          return (← pushExpression loc never (.block #[value.1] (some returned)), never)
        return (← pushExpression loc never (.return_ #[value.1]), never)
  | .throw_ kind arguments _ =>
      if context.specification then
        -- An aborting path in a value position takes the surrounding
        -- expected type; in statement position it has no value, so the
        -- specification reading is Unit.
        let typeId ← match expected with
          | some typeId => pure typeId
          | none => internType .unit
        addArbitrarySpecValue context typeId span
      else
        let arguments ← arguments.mapM (lowerExpr context none)
        let never ← internType .never
        return (← pushExpression loc never (.throw_ kind.toLIR (arguments.map (·.1))), never)

private def sourceQuantifierPatternType (context : ExprContext) (domain : Expr) :
    LowerM TypeId := do
  -- Domains routinely contain field selection and specification calls (for
  -- example `self.list`). Infer their type through the ordinary lowering
  -- rules, retaining newly interned table entries but discarding the probe's
  -- temporary namespace expressions.
  let before ← get
  let inferred ← lowerExpr context none domain
  let after ← get
  set { after with
    output := before.output
    temporaryLocalBase := before.temporaryLocalBase
    temporaryLocals := before.temporaryLocals }
  let domainType := inferred.2
  let patternType ← match ← typeNode? domainType with
    | some (.vector element _) => pure element
    | some .range => internType (.integer .unbounded true)
    | some (.typeDomain element) => projectSpecTypeId element
    | _ => do
        failAt "LEANER-QUANTIFIER-DOMAIN"
          "a quantifier domain must be a vector, logical range, or type" (some domain.span)
  return patternType

private def modifierAttributes (modifiers : FunctionModifiers) : Array Attribute :=
  let visibility := match modifiers.visibility with
    | .private_ => "private"
    | .public_ => "public"
    | .package => "package"
    | .friend => "friend"
  let attributes := #[Attribute.assign "visibility" (.qualifiedName visibility)]
  let attributes := if modifiers.isEntry then attributes.push (.call "entry" #[]) else attributes
  let attributes := if modifiers.isNative then attributes.push (.call "native" #[]) else attributes
  let attributes := if modifiers.isDeprecated then attributes.push (.call "deprecated" #[]) else attributes
  let attributes := if modifiers.isView then attributes.push (.call "view" #[]) else attributes
  if modifiers.isOpaque then attributes.push (.call "opaque" #[]) else attributes

private def clauseKind? : ContractClause → Option ConditionKind
  | .letPre name .. => some (.letPre name)
  | .letPost name .. => some (.letPost name)
  | .requires .. => some .requires
  | .ensures .. => some .ensures
  | .abortsIf .. => some .abortsIf
  | .invariant .. => some .structInvariant
  | .modifies .. | .modifiesAll .. | .reads .. | .readsAll .. => none

private def clauseExpression? : ContractClause → Option Expr
  | .letPre _ expression .. | .letPost _ expression .. |
      .requires expression .. | .ensures expression .. | .abortsIf expression .. |
      .invariant expression .. | .modifies expression .. => some expression
  | .modifiesAll .. | .reads .. | .readsAll .. => none

private def clauseAuxiliary (context : ExprContext) : ContractClause →
    LowerM (Array (String × ExprId))
  | .abortsIf _ code _ _ => code.toArray.mapM fun code => do
      let code ← lowerExpr context none code
      pure ("abortCode", code.1)
  | _ => pure #[]

private def clauseProperties : ContractClause → Array String
  | .letPre _ _ properties _ | .letPost _ _ properties _ |
      .requires _ properties _ | .ensures _ properties _ | .abortsIf _ _ properties _ |
      .invariant _ properties _ => properties
  | .modifies .. | .modifiesAll .. | .reads .. | .readsAll .. => #[]

private partial def pragmaConstValue (value : Expr) : LowerM ConstValue :=
  match value with
  | .unit .. => pure .unit
  | .bool value .. => pure (.bool value)
  | .char value .. => pure (.character value)
  | .integer value .. => pure (.integer value)
  | .typedInteger value .. => pure (.integer value)
  | .address value .. => pure (.address value)
  | .string value .. => pure (.string value)
  | .bytes value .. =>
      pure (.vector (value.map fun byte => .integer (Int.ofNat byte.toNat)))
  | .primitive .tuple values _ => .tuple <$> values.mapM pragmaConstValue
  | .primitive .vector values _ => .vector <$> values.mapM pragmaConstValue
  | value => failAt "LEANER-PRAGMA-VALUE"
      "a pragma value must be a constant literal" (some value.span)

private def pragmaAttribute (pragma : Pragma) : LowerM Attribute := do
  match pragma.value with
  | .local value _ =>
      -- A bare-name pragma value is a declaration reference (for example
      -- `pragma intrinsic = map` or a role target), not a constant.
      pure <| .assign pragma.name (.name none value)
  | value => pure <| .assign pragma.name (.constant (← pragmaConstValue value))

/-- Intrinsic role names are reserved by model prefix. Other annotations,
including annotations naming an intrinsic owner, remain ordinary metadata. -/
private def isIntrinsicAttribute (source : Namespace) (attr : SourceAttribute) : Bool :=
  attr.name.startsWith "intrinsic_" || attr.name.startsWith "map_" ||
    source.items.any fun item =>
      let attributes := match item with
        | .struct declaration => declaration.attributes
        | .enum declaration => declaration.attributes
        | _ => #[]
      attributes.any fun owner => owner.name.startsWith "intrinsic_" &&
        attr.name.startsWith ((owner.name.drop "intrinsic_".length).toString ++ "_")

private partial def lowerSourceAttribute : SourceAttribute → LowerM Attribute
  | .call name arguments span => do
      return .call name (← arguments.mapM lowerSourceAttribute) (some (← addLoc span))
  | .assign name value span => do
      let value := match value with
        | .number value => AttributeValue.constant (.integer value)
        | .string value => AttributeValue.constant (.string value)
        | .name value => AttributeValue.name none value
      return .assign name value (some (← addLoc span))

private def sourceAttributes (attributes : Array SourceAttribute) : LowerM (Array Attribute) := do
  let source := (← get).sourceNamespace
  (attributes.filter fun attr => !isIntrinsicAttribute source attr).mapM lowerSourceAttribute

private def lowerContract (context : ExprContext) (clauses : Array ContractClause)
    (pragmas : Array Attribute) (implicitRustNoPanic : Bool) (nextLocalId : Nat) :
    LowerM (FunctionContract × Array LocalDecl) := do
  let boolType ← internType .bool
  let mut pragmas := pragmas
  let mut conditions := #[]
  let mut modifies := #[]
  let mut reads := #[]
  let mut modifiesAll := false
  let mut readsAll := false
  let mut hasFrame := false
  let mut bindingLocals := #[]
  let mut context := context
  for clause in clauses do
    let loc ← addLoc clause.span
    match clause with
    | .modifies expression _ loose =>
        hasFrame := true
        modifies := modifies.push (← lowerExpr context none expression).1
        if loose && !(hasLooseFrame { pragmas }) then
          pragmas := pragmas.push (.assign "leaner_loose_frame" (.constant (.bool true)))
        continue
    | .modifiesAll _ =>
        hasFrame := true
        modifiesAll := true
        continue
    | .reads type _ =>
        hasFrame := true
        reads := reads.push (← lowerTypeUse context.types type)
        continue
    | .readsAll _ =>
        hasFrame := true
        readsAll := true
        continue
    | _ => pure ()
    let isBinding := match clause with
      | .letPre .. | .letPost .. => true
      | _ => false
    let expression ← lowerExpr context (if isBinding then none else some boolType)
      (clauseExpression? clause).get!
    let auxiliary ← clauseAuxiliary context clause
    let properties := clauseProperties clause |>.map fun name =>
      Attribute.assign name (.constant (.bool true))
    conditions := conditions.push {
      loc, kind := (clauseKind? clause).get!, properties,
      expression := expression.1, auxiliary }
    let bindingName? := match clause with
      | .letPre name .. | .letPost name .. => some name
      | _ => none
    if let some name := bindingName? then
      if (lookupLocal? context name).isSome then
        failAt "LEANER-CONTRACT-BINDING-DUPLICATE"
          s!"contract binding `{name}` shadows an existing local" (some clause.span)
      let id : LocalId := ⟨nextLocalId + bindingLocals.size⟩
      let localDecl : LocalDecl := {
        id, name, type := { typeId := expression.2, loc }, mutable := false, loc }
      bindingLocals := bindingLocals.push localDecl
      context := { context with locals := context.locals.push (name, id, expression.2) }
  if implicitRustNoPanic then
    let loc ← addLoc {}
    let expression ← pushExpression loc boolType (.value (.bool false))
    conditions := conditions.push { loc, kind := .abortsIf, expression }
  return ({
    loc := if conditions.isEmpty then none else conditions[0]?.map (·.loc)
    conditions
    modifies
    reads
    hasFrame
    modifiesAll
    readsAll
    pragmas }, bindingLocals)

private def lowerFunction (declaration : FunctionDecl) : LowerM Unit := do
  if declaration.modifiers.isNative && declaration.modifiers.isOpaque then
    failAt "LEANER-FUNCTION-MODIFIER"
      "a function cannot be both `native` and `opaque`" (some declaration.span)
  let bodyAbsent := declaration.modifiers.isNative || declaration.modifiers.isOpaque
  if bodyAbsent == declaration.body.isSome then
    failAt "LEANER-FUNCTION-BODY"
      (if bodyAbsent then "a native or opaque function cannot have a body"
       else "a regular function must have a body") (some declaration.span)
  let loc ← addLoc declaration.span
  let state ← get
  let some name := nameId? state.tables state.namespaceId declaration.name
    | failAt "LEANER-FUNCTION-NAME" s!"function `{declaration.name}` was not interned"
  let typeContext ← liftM <| typeContext declaration.generics
  let generics ← declaration.generics.mapM lowerBinder
  let mut parameters := #[]
  let mut locals := #[]
  let mut localTypes := #[]
  for (parameter, index) in declaration.parameters.zipIdx do
    if declaration.parameters.take index |>.any (·.name == parameter.name) then
      failAt "LEANER-PARAMETER-DUPLICATE"
        s!"parameter `{parameter.name}` is declared more than once" (some parameter.span)
    let typeUse ← lowerTypeUse typeContext parameter.type
    let parameterLoc ← addLoc parameter.span
    parameters := parameters.push {
      name := parameter.name, typeUse, mutable := parameter.mutable }
    locals := locals.push {
      id := ⟨index⟩, name := parameter.name, type := typeUse
      mutable := parameter.mutable, loc := parameterLoc }
    localTypes := localTypes.push (parameter.name, ⟨index⟩, typeUse.typeId)
  let mut declarations := #[]
  -- Quantifier binders of specification clauses are predeclared in the
  -- logical domain already; projecting their types again would move a vector
  -- element out of the representation its container keeps.
  let mut logicalLocals : Array LocalId := #[]
  -- One row per predeclared local, keyed by the declaration it came from, so
  -- each binding's type is inferred in its own lexical scope.
  let mut predeclared : Array (Span × String × LocalId × TypeId) := #[]
  -- Probing lowerings must read `return` the way the body does, so they use
  -- the function's own result type.
  let predeclareReturnType := (← lowerTypeUse typeContext declaration.result).typeId
  for (binding, scope) in scopedBindings #[] (declaration.body.getD (.unit {})) do
    let scopeLocals := scope.reverse.flatMap fun entry =>
      predeclared.filterMap fun (span, name, id, typeId) =>
        if span == entry.span then some (name, id, typeId) else none
    let inferenceLocals := scopeLocals ++ localTypes
    -- A quantifier inside this initializer binds locals the inference probe
    -- below must already know, so predeclare those binders first.
    for quantifierBinding in sourceQuantifierBindings binding.value do
      unless declarations.any (·.span == quantifierBinding.pattern.span) do
        -- The binders of this initializer's own declarations are already
        -- predeclared, so the domain sees every local it can mention.
        let visible := predeclared.reverse.map
          (fun (_, name, id, typeId) => (name, id, typeId)) ++ localTypes
        let quantifierScope ← visible.mapM fun (name, id, typeId) => do
          pure (name, id, ← projectSpecTypeId typeId)
        let predeclareContext : ExprContext := {
          types := typeContext
          locals := quantifierScope
          declarations
          returnType := predeclareReturnType
          specification := true }
        let patternType ← sourceQuantifierPatternType predeclareContext
          quantifierBinding.domain
        let patternDeclarations ← predeclareBindingPattern typeContext false
          quantifierBinding.pattern patternType locals.size
        for (localInfo, localDecl) in patternDeclarations do
          locals := locals.push localDecl
          declarations := declarations.push localInfo
          logicalLocals := logicalLocals.push localInfo.id
    let typeId ← match binding.type with
      | some type => pure (← lowerTypeUse typeContext type).typeId
      | none => do
          let before ← get
          let inferenceContext : ExprContext := {
            types := typeContext
            locals := inferenceLocals
            declarations
            returnType := predeclareReturnType }
          let inferred ← lowerExpr inferenceContext none binding.value
          let after ← get
          set { after with
            output := before.output
            temporaryLocalBase := before.temporaryLocalBase
            temporaryLocals := before.temporaryLocals }
          pure inferred.2
    let patternDeclarations ← predeclareBindingPattern typeContext binding.mutable
      binding.pattern typeId locals.size
    for (localInfo, localDecl) in patternDeclarations do
      locals := locals.push localDecl
      declarations := declarations.push localInfo
      predeclared := predeclared.push
        (binding.span, localInfo.name, localInfo.id, localInfo.typeId)
  let predeclareResult ← lowerTypeUse typeContext declaration.result
  let predeclareResultType ← projectSpecTypeId predeclareResult.typeId
  -- Quantifier domains may mention any body local, so their predeclaration
  -- scope is every local declared above.
  let bodyLocals := predeclared.map fun (_, name, id, typeId) => (name, id, typeId)
  let mut quantifierLocals ← (bodyLocals.reverse ++ localTypes).mapM fun (name, id, typeId) => do
    pure (name, id, ← projectSpecTypeId typeId)
  let quantifierRoots := declaration.body.toArray ++
    declaration.contract.filterMap clauseExpression?
  for binding in quantifierRoots.flatMap sourceQuantifierBindings do
    if declarations.any (·.span == binding.pattern.span) then continue
    let predeclareContext : ExprContext := {
      types := typeContext
      locals := quantifierLocals
      declarations
      returnType := predeclareReturnType
      resultType := some predeclareResultType
      specification := true }
    let patternType ← sourceQuantifierPatternType predeclareContext binding.domain
    let patternDeclarations ← predeclareBindingPattern typeContext false
      binding.pattern patternType locals.size
    for (localInfo, localDecl) in patternDeclarations do
      locals := locals.push localDecl
      localTypes := localTypes.push (localInfo.name, localInfo.id, localInfo.typeId)
      quantifierLocals := #[(localInfo.name, localInfo.id, localInfo.typeId)] ++
        quantifierLocals
      declarations := declarations.push localInfo
      logicalLocals := logicalLocals.push localInfo.id
  -- Specification clauses may also match: their arm patterns bind locals in
  -- the projected domain, predeclared like the body's match patterns.
  for binding in (declaration.contract.filterMap clauseExpression?).flatMap
      sourceMatchBindings do
    let predeclareContext : ExprContext := {
      types := typeContext
      locals := quantifierLocals
      declarations
      returnType := predeclareReturnType
      resultType := some predeclareResultType
      specification := true }
    let patternType ← sourceMatchPatternType predeclareContext
      binding.pattern binding.scrutinee
    let patternDeclarations ← predeclareBindingPattern typeContext false
      binding.pattern patternType locals.size
    for (localInfo, localDecl) in patternDeclarations do
      locals := locals.push localDecl
      declarations := declarations.push localInfo
  let result ← lowerTypeUse typeContext declaration.result
  let unitType ← internType .unit
  let results := if result.typeId == unitType then #[] else #[result]
  let signature : Signature := { generics, parameters, results }
  let (origin, alignment) ← addOriginAndAlignment loc
  let executableContext : ExprContext := {
    types := typeContext
    generics
    locals := localTypes
    declarations
    returnType := result.typeId }
  modify fun state => { state with
    temporaryLocalBase := locals.size, temporaryLocals := #[] }
  let body ← match declaration.body with
    | none => pure RawBody.absent
    | some source =>
        let lowered ← lowerExpr { executableContext with functionTail := true } (some result.typeId) source
        ensureType result.typeId lowered.2 source.span
        pure (.structured lowered.1)
  let executableLocals := locals ++ (← get).temporaryLocals
  modify fun state => { state with temporaryLocals := #[] }
  let specLocals ← localTypes.mapM fun (name, id, type) => do
    let type ← if logicalLocals.contains id then pure type else projectSpecTypeId type
    return (name, id, type)
  let resultSpecType ← projectSpecTypeId result.typeId
  let specDeclarations ← declarations.mapM fun declaration => do
    if logicalLocals.contains declaration.id then return declaration
    return { declaration with typeId := ← projectSpecTypeId declaration.typeId }
  let specContext : ExprContext := {
    types := typeContext
    generics
    locals := specLocals
    declarations := specDeclarations
    returnType := resultSpecType
    resultType := some resultSpecType
    specification := true }
  let contractPragmas ← declaration.pragmas.mapM pragmaAttribute
  let inheritedPragmas ← state.sourceNamespace.pragmas.mapM pragmaAttribute
  -- A pragma the contract sets shadows the namespace's setting of the same
  -- name, whatever value each one carries.
  let contractPragmaNames := declaration.pragmas.map (·.name)
  let (contract, contractLocals) ← lowerContract specContext declaration.contract contractPragmas
    (state.sourceNamespace.profile == .rust) executableLocals.size
  let pragmas := contract.pragmas ++
    (state.sourceNamespace.pragmas.zip inheritedPragmas).filterMap fun (inherited, lowered) =>
      if contractPragmaNames.contains inherited.name ||
          (hasLooseFrame contract && inherited.name == "leaner_loose_frame") then none
      else some lowered
  let function : LeanerIR.FunctionDecl RawBody := {
    loc
    name
    profile := state.sourceNamespace.profile.toLIR
    signature
    body
    origin
    alignment
    locals := executableLocals ++ contractLocals
    contract
    pragmas
    attributes := modifierAttributes
      (if declaration.attributes.any (fun attr => match attr with
          | .call "move_public" #[] _ => true | _ => false) then
        { declaration.modifiers with visibility := .public_ } else declaration.modifiers) ++
      (← sourceAttributes (declaration.attributes.filter fun attr => match attr with
        | .call "move_public" #[] _ => false | _ => true)) }
  modify fun state => { state with output := { state.output with
    functions := state.output.functions.push function } }
  if let some sourceBody := declaration.body then
    let specParameters ← parameters.mapM fun parameter => do
      let typeId ← projectSpecTypeId parameter.typeUse.typeId
      return { parameter with typeUse := { parameter.typeUse with typeId } }
    let specResults := if resultSpecType == unitType then #[] else
      #[{ result with typeId := resultSpecType }]
    -- An intrinsic or opaque contract supplies the function's logical model
    -- externally. Deriving another body from executable control flow would
    -- both duplicate that model and misinterpret early returns as ordinary
    -- expressions.
    let externallyModeled := declaration.pragmas.any fun pragma =>
      pragma.name == "intrinsic" ||
        (pragma.name == "opaque" && match pragma.value with | .bool true _ => true | _ => false)
    -- A body that jumps out through an early `return`, or iterates, has no
    -- expression-shaped projection either: deriving one would read the jump as
    -- this function's value and the loop as a value it does not have. Such a
    -- function keeps a declared logical symbol with no derived body until the
    -- projection models control flow.
    let externallyModeled := externallyModeled || hasEarlyReturn true sourceBody ||
      hasLoop sourceBody
    let specBody ← if externallyModeled then pure none else do
      let specBody ← lowerExpr { specContext with functionTail := true } (some resultSpecType) sourceBody
      ensureType resultSpecType specBody.2 sourceBody.span
      pure (some specBody.1)
    let specLocalDecls ← locals.mapM fun localDecl => do
      let typeId ← projectSpecTypeId localDecl.type.typeId
      return { localDecl with type := { localDecl.type with typeId } }
    let specFunction : LeanerIR.SpecFunctionDecl := {
      loc
      name
      profile := state.sourceNamespace.profile.toLIR
      signature := { generics, parameters := specParameters, results := specResults }
      body := specBody
      origin
      locals := specLocalDecls }
    modify fun state => { state with output := { state.output with
      specFunctions := state.output.specFunctions.push specFunction } }

private def lowerSpecFunction (declaration : SpecFunctionDecl) : LowerM Unit := do
  if declaration.isOpaque == declaration.body.isSome then
    failAt "LEANER-SPEC-FUNCTION-BODY"
      (if declaration.isOpaque then
        "an opaque specification function cannot have a body"
       else "a non-opaque specification function must have a body")
      (some declaration.span)
  let loc ← addLoc declaration.span
  let state ← get
  let some name := nameId? state.tables state.namespaceId declaration.name
    | failAt "LEANER-SPEC-FUNCTION-NAME"
        s!"specification function `{declaration.name}` was not interned"
  let typeContext ← liftM <| typeContext declaration.generics
  let generics ← declaration.generics.mapM lowerBinder
  let mut parameters := #[]
  let mut locals := #[]
  let mut localTypes := #[]
  for (parameter, index) in declaration.parameters.zipIdx do
    if declaration.parameters.take index |>.any (·.name == parameter.name) then
      failAt "LEANER-SPEC-PARAMETER-DUPLICATE"
        s!"parameter `{parameter.name}` is declared more than once" (some parameter.span)
    if parameter.mutable then
      failAt "LEANER-SPEC-PARAMETER-MUTABLE"
        "a specification-function parameter cannot be mutable" (some parameter.span)
    let typeUse ← lowerTypeUse typeContext parameter.type
    let parameterLoc ← addLoc parameter.span
    parameters := parameters.push { name := parameter.name, typeUse }
    locals := locals.push {
      id := ⟨index⟩, name := parameter.name, type := typeUse, loc := parameterLoc }
    localTypes := localTypes.push (parameter.name, ⟨index⟩, typeUse.typeId)
  let parameterLocalTypes := localTypes
  let mut declarations := #[]
  let mut inferenceLocals := localTypes
  let predeclareReturnType ← internType .unit
  for binding in declaration.body.toArray.flatMap sourceBindings do
    let typeId ← match binding.type with
      | some type => pure (← lowerTypeUse typeContext type).typeId
      | none => do
          let before ← get
          let inferenceContext : ExprContext := {
            types := typeContext
            locals := inferenceLocals
            declarations
            returnType := predeclareReturnType
            specification := true }
          let inferred ← lowerExpr inferenceContext none binding.value
          let after ← get
          set { after with
            output := before.output
            temporaryLocalBase := before.temporaryLocalBase
            temporaryLocals := before.temporaryLocals }
          pure inferred.2
    let patternDeclarations ← predeclareBindingPattern typeContext binding.mutable
      binding.pattern typeId locals.size
    for (localInfo, localDecl) in patternDeclarations do
      locals := locals.push localDecl
      declarations := declarations.push localInfo
      inferenceLocals := #[(localInfo.name, localInfo.id, localInfo.typeId)] ++ inferenceLocals
  let mut quantifierLocals ← inferenceLocals.mapM fun (name, id, typeId) => do
    pure (name, id, ← projectSpecTypeId typeId)
  for binding in declaration.body.toArray.flatMap sourceQuantifierBindings do
    let predeclareContext : ExprContext := {
      types := typeContext
      locals := quantifierLocals
      declarations
      returnType := predeclareReturnType
      specification := true }
    let patternType ← sourceQuantifierPatternType predeclareContext binding.domain
    let patternDeclarations ← predeclareBindingPattern typeContext false
      binding.pattern patternType locals.size
    for (localInfo, localDecl) in patternDeclarations do
      locals := locals.push localDecl
      localTypes := localTypes.push (localInfo.name, localInfo.id, localInfo.typeId)
      quantifierLocals := #[(localInfo.name, localInfo.id, localInfo.typeId)] ++
        quantifierLocals
      declarations := declarations.push localInfo
  for binding in declaration.body.toArray.flatMap sourceMatchBindings do
    let predeclareContext : ExprContext := {
      types := typeContext
      locals := localTypes
      declarations
      returnType := predeclareReturnType
      specification := true }
    let patternType ← sourceMatchPatternType predeclareContext
      binding.pattern binding.scrutinee
    let patternDeclarations ← predeclareBindingPattern typeContext false
      binding.pattern patternType locals.size
    for (localInfo, localDecl) in patternDeclarations do
      locals := locals.push localDecl
      localTypes := localTypes.push (localInfo.name, localInfo.id, localInfo.typeId)
      declarations := declarations.push localInfo
  let result ← lowerTypeUse typeContext declaration.result
  let unitType ← internType .unit
  let results := if result.typeId == unitType then #[] else #[result]
  let (origin, _) ← addOriginAndAlignment loc
  let context : ExprContext := {
    types := typeContext
    generics
    locals := parameterLocalTypes
    declarations
    returnType := result.typeId
    specification := true }
  let body ← declaration.body.mapM fun source => do
    let lowered ← lowerExpr context (some result.typeId) source
    ensureType result.typeId lowered.2 source.span
    pure lowered.1
  let function : LeanerIR.SpecFunctionDecl := {
    loc
    name
    profile := state.sourceNamespace.profile.toLIR
    signature := { generics, parameters, results }
    body
    origin
    locals
    -- Specification declarations use the existing unified annotation slot;
    -- the source printer restores these pragmas to declaration attributes.
    contract := { pragmas := ← sourceAttributes declaration.attributes } }
  modify fun state => { state with output := { state.output with
    specFunctions := state.output.specFunctions.push function } }

private def lowerConstant (declaration : ConstantDecl) : LowerM Unit := do
  let loc ← addLoc declaration.span
  let state ← get
  let some name := nameId? state.tables state.namespaceId declaration.name
    | failAt "LEANER-CONSTANT-NAME" s!"constant `{declaration.name}` was not interned"
  let type ← lowerTypeUse {} declaration.type
  let context : ExprContext := { returnType := type.typeId }
  let value ← lowerExpr context (some type.typeId) declaration.value
  ensureType type.typeId value.2 declaration.value.span
  let constant : LeanerIR.ConstantDecl := { loc, name, type, value := value.1 }
  modify fun state => { state with output := { state.output with
    constants := state.output.constants.push constant } }

private def lowerField (context : TypeContext) (field : FieldDecl) : LowerM LeanerIR.FieldDecl := do
  let loc ← addLoc field.span
  let state ← get
  let some name := nameId? state.tables state.namespaceId field.name
    | failAt "LEANER-FIELD-NAME" s!"field `{field.name}` was not interned"
  let type ← lowerTypeUse context field.type
  return { loc, name, type }

private def lowerStruct (declaration : StructDecl) : LowerM Unit := do
  let loc ← addLoc declaration.span
  let state ← get
  let some name := nameId? state.tables state.namespaceId declaration.name
    | failAt "LEANER-STRUCT-NAME" s!"struct `{declaration.name}` was not interned"
  let context ← liftM <| typeContext declaration.generics
  let generics ← declaration.generics.mapM lowerBinder
  let fields ← declaration.fields.mapM (lowerField context)
  let boolType ← internType .bool
  let mut locals := #[]
  let mut localTypes := #[]
  for ((source, field), index) in (declaration.fields.zip fields).zipIdx do
    let typeId ← projectSpecTypeId field.type.typeId
    let type := { field.type with typeId }
    locals := locals.push {
      id := ⟨index⟩, name := source.name, type, mutable := false, loc := field.loc }
    localTypes := localTypes.push (source.name, ⟨index⟩, typeId)
  let mut declarations := #[]
  for binding in declaration.contract.filterMap clauseExpression? |>.flatMap
      sourceQuantifierBindings do
    let predeclareContext : ExprContext := {
      types := context
      locals := localTypes
      declarations
      returnType := boolType
      specification := true }
    let patternType ← sourceQuantifierPatternType predeclareContext binding.domain
    let patternDeclarations ← predeclareBindingPattern context false
      binding.pattern patternType locals.size
    for (localInfo, localDecl) in patternDeclarations do
      locals := locals.push localDecl
      localTypes := localTypes.push (localInfo.name, localInfo.id, localInfo.typeId)
      declarations := declarations.push localInfo
  for binding in declaration.contract.filterMap clauseExpression? |>.flatMap
      sourceMatchBindings do
    let predeclareContext : ExprContext := {
      types := context
      locals := localTypes
      declarations
      returnType := boolType
      specification := true }
    let patternType ← sourceMatchPatternType predeclareContext
      binding.pattern binding.scrutinee
    let patternDeclarations ← predeclareBindingPattern context false
      binding.pattern patternType locals.size
    for (localInfo, localDecl) in patternDeclarations do
      let typeId ← projectSpecTypeId localInfo.typeId
      locals := locals.push {
        localDecl with type := { localDecl.type with typeId } }
      declarations := declarations.push { localInfo with typeId }
  let pragmas ← declaration.pragmas.mapM pragmaAttribute
  let (contract, contractLocals) ← lowerContract {
    types := context
    locals := localTypes
    declarations
    returnType := boolType
    specification := true } declaration.contract pragmas false locals.size
  let struct : LeanerIR.StructDecl := {
    loc, name, generics, fields, abilities := declaration.abilities.map Ability.toLIR
    locals := locals ++ contractLocals, contract
    attributes := ← sourceAttributes declaration.attributes }
  modify fun state => { state with output := { state.output with
    structs := state.output.structs.push struct } }

private def lowerEnum (declaration : EnumDecl) : LowerM Unit := do
  let loc ← addLoc declaration.span
  let state ← get
  let some name := nameId? state.tables state.namespaceId declaration.name
    | failAt "LEANER-ENUM-NAME" s!"enum `{declaration.name}` was not interned"
  let context ← liftM <| typeContext declaration.generics
  let generics ← declaration.generics.mapM lowerBinder
  let variants ← declaration.variants.mapM fun variant => do
    let loc ← addLoc variant.span
    let current ← get
    let some name := nameId? current.tables current.namespaceId variant.name
      | failAt "LEANER-VARIANT-NAME" s!"variant `{variant.name}` was not interned"
    let fields ← variant.fields.mapM (lowerField context)
    return ({ loc, name, fields, discriminant := variant.discriminant } : LeanerIR.VariantDecl)
  let boolType ← internType .bool
  let mut locals := #[]
  let mut localTypes := #[]
  let mut declarations := #[]
  /- Unlike a structure invariant, whose fields are declaration locals, an
  enum invariant must inspect the whole tagged value.  Give its specification
  the conventional `this` local.  It is logical-only and therefore never
  appears in the runtime field row. -/
  if declaration.contract.any (fun clause => match clause with
      | .invariant .. => true
      | _ => false) then
    unless declaration.generics.isEmpty do
      failAt "LEANER-ENUM-INVARIANT-GENERIC"
        "generic enum invariants are not supported yet" (some declaration.span)
    let thisType ← internType (.nominal name #[])
    locals := locals.push {
      id := ⟨0⟩, name := "this", type := { typeId := thisType, loc }
      mutable := false, loc }
    localTypes := localTypes.push ("this", ⟨0⟩, thisType)
  for binding in declaration.contract.filterMap clauseExpression? |>.flatMap
      sourceQuantifierBindings do
    let predeclareContext : ExprContext := {
      types := context
      locals := localTypes
      declarations
      returnType := boolType
      specification := true }
    let patternType ← sourceQuantifierPatternType predeclareContext binding.domain
    let patternDeclarations ← predeclareBindingPattern context false
      binding.pattern patternType locals.size
    for (localInfo, localDecl) in patternDeclarations do
      locals := locals.push localDecl
      localTypes := localTypes.push (localInfo.name, localInfo.id, localInfo.typeId)
      declarations := declarations.push localInfo
  for binding in declaration.contract.filterMap clauseExpression? |>.flatMap
      sourceMatchBindings do
    let predeclareContext : ExprContext := {
      types := context
      locals := localTypes
      declarations
      returnType := boolType
      specification := true }
    let patternType ← sourceMatchPatternType predeclareContext
      binding.pattern binding.scrutinee
    let patternDeclarations ← predeclareBindingPattern context false
      binding.pattern patternType locals.size
    for (localInfo, localDecl) in patternDeclarations do
      let typeId ← projectSpecTypeId localInfo.typeId
      locals := locals.push {
        localDecl with type := { localDecl.type with typeId } }
      declarations := declarations.push { localInfo with typeId }
  let pragmas ← declaration.pragmas.mapM pragmaAttribute
  let (contract, contractLocals) ← lowerContract {
    types := context
    locals := localTypes
    declarations
    returnType := boolType
    specification := true } declaration.contract pragmas false locals.size
  let properties := if state.sourceNamespace.profile == .move then #[{
      profile := Profile.move, tag := "struct.variants" }]
    else #[]
  let enum : LeanerIR.StructDecl := {
    loc, name, generics, variants, abilities := declaration.abilities.map Ability.toLIR
    properties, locals := locals ++ contractLocals, contract
    attributes := ← sourceAttributes declaration.attributes }
  modify fun state => { state with output := { state.output with
    structs := state.output.structs.push enum } }

private def lowerNamespaceInvariants
    (declarations : Array NamespaceInvariantDecl) : LowerM Unit := do
  let boolType ← internType .bool
  for declaration in declarations do
    let loc ← addLoc declaration.span
    let typeContext ← liftM <| typeContext #[]
    let mut locals := #[]
    let mut localTypes := #[]
    let mut scopedDeclarations := #[]
    for binding in sourceQuantifierBindings declaration.expression do
      let predeclareContext : ExprContext := {
        types := typeContext
        locals := localTypes
        declarations := scopedDeclarations
        returnType := boolType
        specification := true }
      let patternType ← sourceQuantifierPatternType predeclareContext binding.domain
      let patternDeclarations ← predeclareBindingPattern typeContext false
        binding.pattern patternType locals.size
      for (localInfo, localDecl) in patternDeclarations do
        locals := locals.push localDecl
        localTypes := localTypes.push (localInfo.name, localInfo.id, localInfo.typeId)
        scopedDeclarations := scopedDeclarations.push localInfo
    let context : ExprContext := {
      types := typeContext
      locals := localTypes
      declarations := scopedDeclarations
      returnType := boolType
      specification := true }
    let expression ← lowerExpr context (some boolType) declaration.expression
    let isUpdate := declaration.properties.contains "update"
    let properties := (declaration.properties.filter (· != "update")).map fun name =>
      Attribute.assign name (.constant (.bool true))
    let invariant : LeanerIR.NamespaceInvariant := {
      loc
      condition := {
        loc
        kind := if isUpdate then .globalInvariantUpdate else .globalInvariant
        properties
        expression := expression.1 }
      locals }
    modify fun state => { state with output := { state.output with
      invariants := state.output.invariants.push invariant } }

private def lowerItem : Item → LowerM Unit
  | .constant declaration => lowerConstant declaration
  | .struct declaration => lowerStruct declaration
  | .enum declaration => lowerEnum declaration
  | .function declaration => lowerFunction declaration
  | .specFunction declaration => lowerSpecFunction declaration
  | .namespaceInvariants declarations => lowerNamespaceInvariants declarations

/-- Reconstruct namespace intrinsic declarations from the inverted source
attributes: `@[intrinsic_<model>]` opens the declaration on its owner
nominal, and `@[<role> (<Owner>)]` on a function or specification function
adds one role binding. The reconstructed graphs flow through the same
shared validation as frontend-imported ones. -/
private def lowerIntrinsics (source : Namespace) : LowerM Unit := do
  let ownedName (name : String) (span : Span) : LowerM LeanerIR.NameId := do
    let state ← get
    let some id := nameId? state.tables state.namespaceId name
      | failAt "LEANER-ATTRIBUTE" s!"declaration `{name}` was not interned" (some span)
    pure id
  let mut owners : Array (String × String × LeanerIR.NameId × LeanerIR.LocId) := #[]
  for item in source.items do
    let ownerAttributes := match item with
      | .struct declaration => #[(declaration.name, declaration.attributes)]
      | .enum declaration => #[(declaration.name, declaration.attributes)]
      | _ => #[]
    for (declarationName, attributes) in ownerAttributes do
      for sourceAttribute in attributes do
        if sourceAttribute.name.startsWith "intrinsic_" then
          unless (match sourceAttribute with | .call _ #[] _ => true | _ => false) do
            failAt "LEANER-ATTRIBUTE"
              "an intrinsic owner attribute takes no arguments" (some sourceAttribute.span)
          let owner ← ownedName declarationName sourceAttribute.span
          owners := owners.push (declarationName,
            (sourceAttribute.name.drop "intrinsic_".length).toString, owner,
            ← addLoc sourceAttribute.span)
        else if isIntrinsicAttribute source sourceAttribute then
          failAt "LEANER-ATTRIBUTE"
            "an intrinsic role attribute must annotate a function or specification function"
            (some sourceAttribute.span)
  let bindingOf (declarationName : String)
      (sourceAttribute : SourceAttribute) : LowerM (String × LeanerIR.IntrinsicBinding) := do
    let .call _ #[.call ownerName #[] _] _ := sourceAttribute
      | failAt "LEANER-ATTRIBUTE"
          s!"attribute `{sourceAttribute.name}` must name exactly one intrinsic owner"
          (some sourceAttribute.span)
    unless owners.any (·.1 == ownerName) do
      failAt "LEANER-ATTRIBUTE"
        s!"attribute `{sourceAttribute.name}` names `{ownerName}`, which has no intrinsic marker"
        (some sourceAttribute.span)
    let target ← ownedName declarationName sourceAttribute.span
    let state ← get
    pure (ownerName, {
      loc := ← addLoc sourceAttribute.span
      role := sourceAttribute.name
      target := { namespaceId := state.namespaceId, name := target } })
  let mut executable : Array (String × LeanerIR.IntrinsicBinding) := #[]
  let mut specification : Array (String × LeanerIR.IntrinsicBinding) := #[]
  for item in source.items do
    match item with
    | .function declaration =>
        for sourceAttribute in declaration.attributes do
          if isIntrinsicAttribute source sourceAttribute then
            executable := executable.push (← bindingOf declaration.name sourceAttribute)
    | .specFunction declaration =>
        for sourceAttribute in declaration.attributes do
          if isIntrinsicAttribute source sourceAttribute then
            specification := specification.push (← bindingOf declaration.name sourceAttribute)
    | _ => pure ()
  for (ownerName, model, owner, loc) in owners do
    let declaration : LeanerIR.IntrinsicDecl := {
      loc, model, owner
      profile := source.profile.toLIR
      executableBindings := executable.filterMap fun (name, binding) =>
        if name == ownerName then some binding else none
      specBindings := specification.filterMap fun (name, binding) =>
        if name == ownerName then some binding else none }
    modify fun state => { state with output := { state.output with
      intrinsics := state.output.intrinsics.push declaration } }

private def lowerNamespace (unit : CompilationUnit) (source : Namespace)
    (namespaceId : NamespaceId) (tables : Tables) : Result (Tables × RawNamespace) := do
  if source.path.isEmpty then
    throw #[.error "LEANER-NAMESPACE-PATH"
      "a namespace path must contain at least one segment" (some source.span)]
  let initial : BuildState := {
    source := unit
    sourceNamespace := source
    namespaceId
    tables
    output := {
      loc := ⟨0⟩
      identity := namespaceId
      profile := some source.profile.toLIR
      doc := source.doc } }
  let action : LowerM Unit := do
    let loc ← addLoc source.span
    let pragmas ← source.pragmas.mapM pragmaAttribute
    modify fun state => { state with output := { state.output with loc, pragmas } }
    for comment in source.comments do
      let loc ← addLoc comment.span
      modify fun state => { state with output := { state.output with
        comments := state.output.comments.push {
          loc, text := comment.text, isDoc := comment.isDoc,
          ownLine := comment.ownLine } } }
    -- Uses are source-level aliases rather than semantic dependency claims.
    -- Intern their targets so a print/parse/print cycle retains the same
    -- inferred declarations even when a surface builtin consumes the only
    -- call site.
    for path in source.uses do
      let owner := path.pop
      let localName := path.back!
      let state ← get
      let (tables, namespaceId) := internNamespace state.tables owner
      let (tables, _) := internName tables namespaceId localName
      set { state with tables }
    -- `friend` grants another namespace privileged access. Move records the
    -- relation as namespace profile metadata, whose payload names the friend
    -- module by address, optional address alias, and name.
    for declaration in source.friends do
      unless source.profile == .move do
        failAt "LEANER-FRIEND-PROFILE"
          "friend declarations are a Move-profile concept" (some declaration.span)
      unless declaration.path.size == 2 do
        failAt "LEANER-FRIEND-PATH"
          "a friend declaration names `0xADDRESS::module_name` or `alias::module_name`"
          (some declaration.span)
      let head := declaration.path[0]!
      let name := declaration.path[1]!
      -- A friend module lives at its declaring module's address, so an alias
      -- in the address position denotes that same address.
      let (address, alias) :=
        if head.startsWith "0x" then (head, none) else (source.path[0]!, some head)
      let payload := packStringPayload
        #[address, packStringPayload (alias.map (#[·]) |>.getD #[]), name]
      modify fun state => { state with output := { state.output with
        profileMetadata := state.output.profileMetadata.push
          { profile := .move, tag := "metadata.friend", payload } } }
    for item in source.items do lowerItem item
    lowerIntrinsics source
  let (_, state) ← action.run initial
  return (state.tables, state.output)

private def configs (unit : CompilationUnit) : Array ProfileConfig :=
  unit.namespaces.foldl (fun configs sourceNs =>
    let profile := sourceNs.profile.toLIR
    if configs.any (·.profile == profile) then configs
    else configs.push sourceNs.profile.config) #[]

private structure InstantiationCall where
  caller : NameId
  callee : NameId
  arguments : Array GenericArgument

private def instantiationTypeChildren : LeanerIR.Ty → Array TypeId
  | .tuple elements => elements
  | .vector element _ | .typeDomain element => #[element]
  | .resourceDomain _ arguments => arguments.getD #[]
  | .nominal _ arguments => arguments.filterMap fun
      | .typeArg value => some value.typeId
      | _ => none
  | .function arguments result _ => arguments.push result
  | .reference reference => #[reference.referent]
  | _ => #[]

/-- Complete only the type instantiations demanded by executable operations
and their transitive callees. In particular a resource used only in a generic
body must have an interned key at each concrete invocation. Bodies remain
shared; this is a worklist over types, not monomorphization of functions. -/
private def completeInvocationTypes (namespaces : Array RawNamespace)
    (span : Span) : LowerM Unit := do
  let mut calls : Array InstantiationCall := #[]
  let mut pending : Array (Nat × TypeId) := #[]
  let mut known : Std.HashSet (Nat × Nat) := {}
  for ns in namespaces do
    for function in ns.functions do
      let .structured root := function.body | continue
      let mut stack := #[root]
      let mut visited : Std.HashSet Nat := {}
      while !stack.isEmpty do
        let id := stack.back!
        stack := stack.pop
        if visited.contains id.index then continue
        visited := visited.insert id.index
        let some expression := ns.expressions[id.index]? | continue
        stack := stack ++ Validation.expressionChildren expression.kind
        let .operation operation arguments _ _ := expression.kind | continue
        for argument in arguments do
          let .typeArg value := argument | continue
          let key := (function.name.index, value.typeId.index)
          unless known.contains key do
            known := known.insert key
            pending := pending.push (function.name.index, value.typeId)
        match operation with
        | .call (.function target) | .call (.closure target) =>
            calls := calls.push { caller := function.name, callee := target.name, arguments }
        | _ => pure ()
  if calls.isEmpty then return
  /- Match the VM's instantiation-loop rule: a cycle of parameter flow is
  legal only when every edge is an identity substitution. An occurrence
  underneath a type constructor is a positive (growing) edge. Reject those
  SCCs before the closure, which would otherwise produce infinitely many
  resource keys. Constants and parameter permutations are not rejected. -/
  let tables := (← get).tables
  let mut edges : Std.HashMap (Nat × Nat) (List (Nat × Nat)) := {}
  let mut vertices : Std.HashSet (Nat × Nat) := {}
  let mut positive : Array ((Nat × Nat) × (Nat × Nat)) := #[]
  for call in calls do
    for (argument, parameter) in call.arguments.zipIdx do
      let .typeArg value := argument | continue
      let target := (call.callee.index, parameter)
      let mut stack := #[(value.typeId, false)]
      let mut visited : Std.HashSet (Nat × Bool) := {}
      while !stack.isEmpty do
        let (id, nested) := stack.back!
        stack := stack.pop
        if visited.contains (id.index, nested) then continue
        visited := visited.insert (id.index, nested)
        let some type := tables.types[id.index]? | continue
        if let .typeParameter index := type then
          let source := (call.caller.index, index)
          vertices := (vertices.insert source).insert target
          edges := edges.insert source (target :: edges.getD source [])
          if nested then positive := positive.push (source, target)
        else
          stack := stack ++ (instantiationTypeChildren type).map (·, true)
  if !positive.isEmpty then
    let components := Lean.SCC.scc vertices.toList (edges.getD · [])
    let mut membership : Std.HashMap (Nat × Nat) Nat := {}
    for (component, index) in components.toArray.zipIdx do
      for vertex in component do membership := membership.insert vertex index
    for (source, target) in positive do
      if membership.get? source == membership.get? target then
        failAt "LEANER-INSTANTIATION-LOOP"
          "a recursive call cycle grows a generic type argument" (some span)
  let mut callers : Std.HashMap Nat (Array InstantiationCall) := {}
  for call in calls do
    callers := callers.insert call.callee.index
      ((callers.getD call.callee.index #[]).push call)
  let mut cursor := 0
  while cursor < pending.size do
    let (callee, required) := pending[cursor]!
    cursor := cursor + 1
    for call in callers.getD callee #[] do
      let instantiated ← instantiateTypeId call.arguments required span
      let key := (call.caller.index, instantiated.index)
      unless known.contains key do
        known := known.insert key
        pending := pending.push (call.caller.index, instantiated)

/-- Lower a resolved Leaner source AST to the only public frontend boundary.
The returned raw unit still requires ordinary shared LIR validation. -/
def lower (unit : CompilationUnit) : Result RawUnit := do
  let mut tables ← initialTables unit
  let mut namespaces := #[]
  for (sourceNs, index) in unit.namespaces.zipIdx do
    let (nextTables, lowered) ← lowerNamespace unit sourceNs ⟨index⟩ tables
    tables := nextTables
    namespaces := namespaces.push lowered
  if let some sourceNamespace := unit.namespaces[0]? then
    let (_, completed) ← (completeInvocationTypes namespaces sourceNamespace.span).run {
      source := unit, sourceNamespace, namespaceId := ⟨0⟩, tables,
      output := namespaces[0]! }
    tables := completed.tables
  return {
    tables
    profiles := configs unit
    namespaces
    evidence := #[{
      producer := "LeanerLang"
      description := "direct lowering from authored Leaner source"
      trusted := true }] }

end LeanerLang
