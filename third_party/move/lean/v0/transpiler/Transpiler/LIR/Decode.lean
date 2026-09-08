-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove
import Transpiler.Effects
import Transpiler.LIR.Codec
import Transpiler.LIR.MoveNames

/-!
# Validated LIR to the Leaner backend view

The established Leaner printer operates on a typed Move view.  This module
derives that view solely from validated LIR.  It is a backend projection, not
retained frontend state, and therefore also acts as a losslessness oracle for
the transitional XAST adapter.
-/

namespace Transpiler.LIR.Decode

open Transpiler Xast Effects

private def fail {α : Type} (message : String) : Except String α := .error message

private def requireSome (value : Option α) (message : String) : Except String α :=
  match value with | some value => .ok value | none => .error message

private partial def location (ns : LeanerIR.Validation.ValidatedNamespace)
    (id : LeanerIR.LocId) (fuel : Nat := 0) : Except String Loc := do
  let fuel := if fuel == 0 then ns.tables.locations.size + 1 else fuel
  if fuel == 0 then fail s!"cyclic validated location parent chain at {id.index}"
  let loc ← requireSome ns.tables.locations[id.index]? s!"invalid validated location {id.index}"
  if let some range := loc.primary then
    return { file := range.file.index, start := range.startByte, stop := range.endByte }
  if let some parent := loc.parent then
    return ← location ns parent (fuel - 1)
  fail s!"location {id.index} has no Move source range or ranged parent"

private def moveAbility : LeanerIR.Ability → Except String Ability
  | .copy => pure .copy
  | .drop => pure .drop
  | .store => pure .store
  | .key => pure .key

private partial def ty (ns : LeanerIR.Validation.ValidatedNamespace) (id : LeanerIR.TypeId) : Except String Ty := do
  let value ← requireSome ns.tables.types[id.index]? s!"invalid validated type {id.index}"
  match value with
  | .unit => return .tuple []
  | .never => fail "Move profile contains neutral never type"
  | .bool => return .bool
  | .character => fail "character type cannot be projected as Move XAST"
  | .string => fail "string type cannot be projected as Move XAST"
  | .bytes => fail "bytes type cannot be projected as Move XAST"
  | .integer (.bits 8) false => return .u8 | .integer (.bits 16) false => return .u16
  | .integer (.bits 32) false => return .u32 | .integer (.bits 64) false => return .u64
  | .integer (.bits 128) false => return .u128 | .integer (.bits 256) false => return .u256
  | .integer (.bits 8) true => return .i8 | .integer (.bits 16) true => return .i16
  | .integer (.bits 32) true => return .i32 | .integer (.bits 64) true => return .i64
  | .integer (.bits 128) true => return .i128 | .integer (.bits 256) true => return .i256
  | .integer .unbounded _ => return .num
  | .integer _ _ => fail "integer width is not part of the Move profile"
  | .address => return .address
  | .signer => return .signer
  | .tuple elements => return .tuple (← elements.toList.mapM (ty ns))
  | .vector element none => return .vector (← ty ns element)
  | .vector _ (some _) => fail "fixed-length vector type cannot be projected as Move XAST"
  | .range => return .range
  | .eventStore => return .eventStore
  | .typeDomain type => return .typeDomain (← ty ns type)
  | .resourceDomain name arguments =>
      return .resourceDomain (← MoveNames.qualifiedName ns.tables name) (← arguments.mapM fun values =>
        values.toList.mapM (ty ns))
  | .stateDomain => return .stateDomain
  | .nominal name arguments => do
      let arguments ← arguments.toList.mapM fun
        | .typeArg value => ty ns value.typeId
        | _ => fail "Move nominal type has a non-type generic argument"
      return .struct (← MoveNames.qualifiedName ns.tables name) arguments
  | .function arguments result abilities =>
      return .function (.tuple (← arguments.toList.mapM (ty ns))) (← ty ns result)
        (← abilities.toList.mapM moveAbility)
  | .typeParameter index => return .typeParam index
  | .reference reference =>
      if reference.profile != .move then
        fail s!"reference type belongs to non-Move profile {repr reference.profile}"
      return .reference (reference.kind == .mutable) (← ty ns reference.referent)
  | .profile value => decodeProfileType value
where
  decodeProfileType (value : LeanerIR.ProfileValue) : Except String Ty := do
    match value.tag with
    | tag => fail s!"unsupported Move profile type `{tag}`"

private def typeUse (ns : LeanerIR.Validation.ValidatedNamespace) (value : LeanerIR.TypeUse) : Except String Ty :=
  ty ns value.typeId

private def surfaceSyntax : Option LeanerIR.SurfaceSyntax → Except String (Option Xast.SurfaceSyntax)
  | none => pure none
  | some .receiverCall => pure (some .receiverCall)
  | some .indexNotation => pure (some .indexNotation)
  | some (.extension value) =>
      fail s!"surface extension `{value.tag}` cannot be projected as Move XAST"

private partial def moveValue : LeanerIR.ConstValue → Except String Value
  | .bool value => return .bool value
  | .integer value => return .number value
  | .vector elements => return .vector (← elements.toList.mapM moveValue)
  | .tuple elements => return .tuple (← elements.toList.mapM moveValue)
  | .address value => return .address value
  | .profile profile => fail s!"unsupported Move value tag `{profile.tag}`"
  | _ => fail "constant value is not part of the Move profile"

private def pragma (ns : LeanerIR.Validation.ValidatedNamespace)
    (value : LeanerIR.Attribute) : Except String Pragma := do
  match value with
  | .assign name (.constant value) _ => return { name, value := .value (← moveValue value) }
  | .assign name (.name none value) _ => return { name, value := .name value }
  | .assign name (.qualifiedName value) _ => return { name, value := .qualifiedName value }
  | .assign _ (.name (some namespaceId) _) _ =>
      fail s!"namespace-qualified attribute value from {(← MoveNames.moduleRef ns.tables namespaceId).name} cannot be a Move pragma"
  | .call name _ _ => fail s!"call-style attribute `{name}` cannot be projected as a Move pragma"

private partial def decodeAttribute (ns : LeanerIR.Validation.ValidatedNamespace) :
    LeanerIR.Attribute → Except String Attribute
  | .call name arguments _ =>
      return .apply name (← arguments.toList.mapM (decodeAttribute ns))
  | .assign name (.constant value) _ =>
      return .assign name (.value (← moveValue value))
  | .assign name (.name namespaceId value) _ =>
      return .assign name (.name (← namespaceId.mapM (MoveNames.moduleRef ns.tables)) value)
  | .assign name (.qualifiedName value) _ =>
      return .assign name (.name none value)

private def typeParam (_ns : LeanerIR.Validation.ValidatedNamespace)
    (binder : LeanerIR.GenericBinder) : Except String TypeParam := do
  if binder.kind != .typeArg then fail "Move generic binder is not a type parameter"
  return {
    name := binder.name
    abilities := ← binder.abilities.toList.mapM moveAbility
    isPhantom := binder.predicates.any fun
      | .profile value => value.tag == "typeParameter.phantom"
      | _ => false }

structure ExprContext where
  ns : LeanerIR.Validation.ValidatedNamespace
  locals : Array LeanerIR.LocalDecl
  parameters : Array LeanerIR.Parameter

private def localNode (context : ExprContext) (id : LeanerIR.LocalId) : Except String ExpNode := do
  if id.index < context.parameters.size then return .param id.index
  let localDecl ← requireSome context.locals[id.index]? s!"invalid local {id.index}"
  return .«local» localDecl.name

private def localExpression (context : ExprContext) (sourceLoc : Loc)
    (id : LeanerIR.LocalId) : Except String Exp := do
  let localDecl ← requireSome context.locals[id.index]? s!"invalid local {id.index}"
  return .mk (← typeUse context.ns localDecl.type) sourceLoc (← localNode context id)

private def genericTypeArguments (ns : LeanerIR.Validation.ValidatedNamespace)
    (arguments : Array LeanerIR.GenericArgument) : Except String (List Ty) :=
  arguments.toList.mapM fun
    | .typeArg value => typeUse ns value
    | .const _ => fail "const generic argument cannot be projected as Move XAST"
    | .lifetime _ => fail "lifetime generic argument cannot be projected as Move XAST"
    | .evidence _ => fail "evidence generic argument cannot be projected as Move XAST"

mutual
  /-- Reconstruct the value-shaped XAST tree denoted by a validated LIR place.
  Place nodes intentionally carry no source location or result type of their
  own, so the enclosing borrow supplies the final referent type and generated
  projection nodes inherit its source range. Intermediate local, dereference,
  field, and vector-index types are recovered from the validated tables. -/
  private partial def placeExpression (context : ExprContext) (sourceLoc : Loc)
      (id : LeanerIR.PlaceId) (expected : Option LeanerIR.TypeId := none) :
      Except String (Exp × LeanerIR.TypeId) := do
    let place ← requireSome context.ns.places[id.index]?
      s!"invalid validated place {id.index}"
    match place with
    | .localVar localId => do
        let declaration ← requireSome context.locals[localId.index]?
          s!"invalid place local {localId.index}"
        let localExp ← localExpression context sourceLoc localId
        if let some expected := expected then
          if expected == declaration.type.typeId then
            return (localExp, declaration.type.typeId)
          if let some (.reference reference) :=
              context.ns.tables.types[declaration.type.typeId.index]? then
            if reference.referent == expected then
              return (.mk (← ty context.ns expected) sourceLoc
                (.call .deref [] [localExp] none), expected)
          fail <| s!"validated local place {localId.index} (`{declaration.name}`) has type " ++
            s!"{declaration.type.typeId.index} ({repr context.ns.tables.types[declaration.type.typeId.index]?}), " ++
            s!"but the borrowed referent has type {expected.index} " ++
            s!"({repr context.ns.tables.types[expected.index]?}) at " ++
            s!"{(context.ns.tables.files[sourceLoc.file]?).map (·.name) |>.getD s!"source file {sourceLoc.file}"}, " ++
            s!"bytes {sourceLoc.start}-{sourceLoc.stop}"
        return (localExp, declaration.type.typeId)
    | .deref base => do
        let (base, baseType) ← placeExpression context sourceLoc base
        let .reference reference ← requireSome context.ns.tables.types[baseType.index]?
            s!"invalid dereference-place type {baseType.index}"
          | fail "validated dereference place has a non-reference base"
        if let some expected := expected then
          unless expected == reference.referent do
            fail "validated dereference-place type differs from the borrowed referent"
        return (.mk (← ty context.ns reference.referent) sourceLoc
          (.call .deref [] [base] none), reference.referent)
    | .field base field => do
        let (base, baseType) ← projectionBaseExpression context sourceLoc base
        let .nominal owner instantiations ← requireSome
            context.ns.tables.types[baseType.index]?
            s!"invalid field-place base type {baseType.index}"
          | fail "validated field place has a non-nominal base"
        let fieldName ← requireSome context.ns.tables.names[field.index]?
          s!"invalid field-place name {field.index}"
        let resultType ← match expected with
          | some expected => pure expected
          | none => do
              let declaration ← requireSome
                (context.ns.structs.find? fun declaration => declaration.name == owner)
                "cannot infer an intermediate dependency field-place type"
              let fields := declaration.fields ++
                declaration.variants.flatMap (·.fields)
              let fieldTypes := fields.filterMap fun declaration => do
                let name ← context.ns.tables.names[declaration.name.index]?
                if name.name == fieldName.name then some declaration.type else none
              requireSome
                (LeanerIR.Validation.instantiatedCommonPlaceTypeId?
                  context.ns instantiations fieldTypes)
                "cannot infer an intermediate generic field-place type"
        let ownerName ← MoveNames.qualifiedName context.ns.tables owner
        let genericTypes ← genericTypeArguments context.ns instantiations
        return (.mk (← ty context.ns resultType) sourceLoc
          (.call (.select ownerName fieldName.name) genericTypes [base] none), resultType)
    | .index base index => do
        let (base, baseType) ← projectionBaseExpression context sourceLoc base
        let resultType ← match expected with
          | some expected => pure expected
          | none => match context.ns.tables.types[baseType.index]? with
              | some (.vector element _) => pure element
              | _ => fail "cannot infer an intermediate non-vector index-place type"
        let index ← expression context index
        return (.mk (← ty context.ns resultType) sourceLoc
          (.call .index [] [base, index] (some .indexNotation)), resultType)
    | .subslice .. =>
        fail "subslice place borrows cannot be projected as Move XAST"
    | .downcast .. =>
        fail "enum-downcast place borrows cannot be projected as Move XAST"

  /-- Move XAST field and index operations implicitly dereference a reference
  operand. Place normalization makes that step explicit; erase exactly this
  projection-local dereference when recovering the original value tree. -/
  private partial def projectionBaseExpression (context : ExprContext)
      (sourceLoc : Loc) (id : LeanerIR.PlaceId) :
      Except String (Exp × LeanerIR.TypeId) := do
    let place ← requireSome context.ns.places[id.index]?
      s!"invalid validated projection base place {id.index}"
    match place with
    | .deref referencePlace => do
        let (referenceExpression, referenceType) ←
          placeExpression context sourceLoc referencePlace
        let .reference reference ← requireSome
            context.ns.tables.types[referenceType.index]?
            s!"invalid projection reference type {referenceType.index}"
          | fail "validated projection dereference has a non-reference base"
        return (referenceExpression, reference.referent)
    | _ => placeExpression context sourceLoc id

  private partial def expression (context : ExprContext) (id : LeanerIR.ExprId) : Except String Exp := do
    let node ← requireSome context.ns.expressions[id.index]? s!"invalid expression {id.index}"
    let sourceLoc ← location context.ns node.loc
    let sourceTy ← ty context.ns node.typeId
    let expNode : ExpNode ← match node.kind with
      | .value constant sourceConstant =>
          pure <| Xast.ExpNode.value (← moveValue constant) sourceConstant
      | .constant _ => fail "standalone neutral constant reference is not a Move XAST expression"
      | .localVar localIdValue => localNode context localIdValue
      | .operation operation instantiations argumentIds surface => do
          let arguments ← argumentIds.toList.mapM (expression context)
          match operation with
          | .borrow kind place =>
              let .reference reference ← requireSome
                  context.ns.tables.types[node.typeId.index]?
                  s!"invalid validated borrow result type {node.typeId.index}"
                | fail "validated borrow operation has a non-reference result"
              let (target, _) ← placeExpression context sourceLoc place
                (some reference.referent)
              let operation ← match kind with
                | .immutable => pure <| Xast.Operation.borrow .immutable
                | .mutable => pure <| Xast.Operation.borrow .mutable
                | .profile value =>
                    fail s!"extension borrow `{value.tag}` cannot be projected as Move XAST"
              pure <| Xast.ExpNode.call operation
                (← genericTypeArguments context.ns instantiations)
                [target]
                (← surfaceSyntax surface)
          | .call (.function callee) =>
              pure <| Xast.ExpNode.call (.moveFunction (← MoveNames.qualifiedRef context.ns.tables callee))
                (← genericTypeArguments context.ns instantiations) arguments
                (← surfaceSyntax surface)
          | .call (.constructor constructor variant) =>
              pure <| Xast.ExpNode.call (.pack (← MoveNames.qualifiedRef context.ns.tables constructor) variant)
                (← genericTypeArguments context.ns instantiations) arguments
                (← surfaceSyntax surface)
          | .call .invoke =>
              match arguments with
              | function :: arguments => pure <| Xast.ExpNode.invoke function arguments
              | [] => fail "invoke call has no callable operand"
          | .call (.destructor ..) =>
              fail "constructor destruction cannot be projected as a Move XAST expression"
          | .call (.closure ..) =>
              fail "closure construction cannot be projected as Move XAST"
          | .global kind =>
              let operation ← match kind with
                | .contains => pure <| Xast.Operation.exists none
                | .borrow .immutable => pure <| Xast.Operation.borrowGlobal .immutable
                | .borrow .mutable => pure <| Xast.Operation.borrowGlobal .mutable
                | .borrow (.profile value) =>
                    fail s!"extension borrow `{value.tag}` cannot be projected as Move XAST"
                | .take => pure Xast.Operation.moveFrom
                | .publish => pure Xast.Operation.moveTo
              pure <| Xast.ExpNode.call operation
                (← genericTypeArguments context.ns instantiations) arguments
                (← surfaceSyntax surface)
          | .primitive primitive =>
              pure <| Xast.ExpNode.call (← Codec.primitiveXastOperation primitive)
                (← genericTypeArguments context.ns instantiations) arguments
                (← surfaceSyntax surface)
          | .reference referenceOperation =>
              match referenceOperation with
              | .mutate =>
                  match arguments with
                  | [target, value] => pure <| Xast.ExpNode.mutate target value
                  | _ => fail "mutate operation does not have two operands"
              | referenceOperation =>
                  let operation ← match referenceOperation with
                    | .borrow .immutable => pure <| Xast.Operation.borrow .immutable
                    | .borrow .mutable => pure <| Xast.Operation.borrow .mutable
                    | .borrow (.profile value) =>
                        fail s!"extension borrow `{value.tag}` cannot be projected as Move XAST"
                    | .dereference => pure Xast.Operation.deref
                    | .freeze explicit => pure <| Xast.Operation.freeze explicit
                    | .mutate => unreachable!
                  pure <| Xast.ExpNode.call operation
                    (← genericTypeArguments context.ns instantiations) arguments
                    (← surfaceSyntax surface)
          | .data dataOperation =>
              let elideMoveAutoDeref := match dataOperation with
                | .select .. => true
                | .selectVariants .. => true
                | .testVariants .. => true
                | _ => false
              let arguments ← match elideMoveAutoDeref, argumentIds.toList, arguments with
                | true, [baseId], [_] =>
                    match context.ns.expressions[baseId.index]? with
                    | some { kind := .operation (.reference .dereference) #[] #[reference] _, .. } =>
                        pure [← expression context reference]
                    | _ => pure arguments
                | _, _, _ => pure arguments
              let operation ← match dataOperation with
                | .select type field =>
                    let target ← MoveNames.qualifiedRef context.ns.tables type
                    pure <| Xast.Operation.select target field
                | .selectVariants type fields =>
                    let target ← MoveNames.qualifiedRef context.ns.tables type
                    pure <| Xast.Operation.selectVariants target fields.toList
                | .testVariants type variants =>
                    let target ← MoveNames.qualifiedRef context.ns.tables type
                    pure <| Xast.Operation.testVariants target variants.toList
                | .discriminant _ =>
                    fail "enum discriminants cannot be projected as a Move XAST expression"
                | .updateField type field =>
                    let target ← MoveNames.qualifiedRef context.ns.tables type
                    pure <| Xast.Operation.updateField target field
              pure <| Xast.ExpNode.call operation
                (← genericTypeArguments context.ns instantiations) arguments
                (← surfaceSyntax surface)
          | .specification specification =>
              let operation ← match specification with
                | .functionCall function range =>
                    let target ← MoveNames.qualifiedRef context.ns.tables function
                    pure <| Xast.Operation.specFunction target
                      { pre := range.pre, post := range.post }
                | specification => Codec.specXastOperation specification
              pure <| Xast.ExpNode.call operation
                (← genericTypeArguments context.ns instantiations) arguments
                (← surfaceSyntax surface)
          | .call (.extension profile targets) | .profile profile targets =>
              if profile.tag == "invoke" then
                match arguments with
                | function :: arguments => pure <| Xast.ExpNode.invoke function arguments
                | [] => fail "invoke operation has no function operand"
              else
                let targets ← targets.mapM (MoveNames.qualifiedRef context.ns.tables)
                pure <| Xast.ExpNode.call (← Codec.decodeOperation profile.tag profile.payload targets)
                  (← genericTypeArguments context.ns instantiations) arguments
                  (← surfaceSyntax surface)
          | _ => fail "neutral primitive operation cannot be projected as Move XAST"
      | .block statements result =>
          pure <| Xast.ExpNode.sequence
            (← (statements ++ result.toArray).toList.mapM (expression context))
      | .letDecl patternId binding body =>
          pure <| Xast.ExpNode.block (← pattern context patternId)
            (← binding.mapM (expression context))
            (← expression context body)
      | .ifElse condition thenBranch elseBranch =>
          pure <| Xast.ExpNode.ite (← expression context condition) (← expression context thenBranch)
            (← expression context (← requireSome elseBranch "Move conditional has no else branch"))
      | .match_ scrutinee arms =>
          pure <| Xast.ExpNode.match (← expression context scrutinee) (← arms.toList.mapM fun arm =>
            return Xast.MatchArm.mk sourceLoc (← pattern context arm.pattern)
              (← arm.guard.mapM (expression context)) (← expression context arm.body))
      | .loop _ body => pure <| Xast.ExpNode.loop (← expression context body)
      | .break_ nest _ => pure <| Xast.ExpNode.loopCont nest false
      | .continue_ nest => pure <| Xast.ExpNode.loopCont nest true
      | .return_ values =>
          match values with
          | #[result] => pure <| Xast.ExpNode.return (← expression context result)
          | _ => fail "Move return does not have exactly one value expression"
      | .throw_ .abort arguments =>
          match (← arguments.toList.mapM (expression context)) with
          | [code] => pure <| Xast.ExpNode.call (.abort .code) [] [code] none
          | _ => fail "Move XAST can project an abort with exactly one code argument"
      | .throw_ .panic _ => fail "Rust panic cannot be projected as Move XAST"
      | .throw_ (.profile value) _ =>
          fail s!"extension throw `{value.tag}` cannot be projected as Move XAST"
      | .assign _ _ => fail "place assignment was not produced by the Move XAST adapter"
      | .assignPattern patternId assigned =>
          pure <| Xast.ExpNode.assign (← pattern context patternId) (← expression context assigned)
      | .quantifier kind binders triggers condition body => do
          let kind ← match kind with
            | .forall => pure QuantKind.forall | .exists => pure .exists
            | .choose => pure .choose | .chooseMin => pure .chooseMin
            | .profile value => match value.tag with
                | tag => fail s!"unknown extension quantifier tag `{tag}`"
          let ranges ← binders.toList.mapM fun binder =>
            return .mk (← pattern context binder.pattern) (← expression context binder.domain)
          pure <| Xast.ExpNode.quant kind ranges (← triggers.toList.mapM fun trigger =>
            trigger.toList.mapM (expression context)) (← condition.mapM (expression context))
            (← expression context body)
      | .spec block => pure <| Xast.ExpNode.specBlock (← specBlock context block)
    return .mk sourceTy sourceLoc expNode

  private partial def pattern (context : ExprContext) (id : LeanerIR.PatternId) : Except String Pattern := do
    let node ← requireSome context.ns.patterns[id.index]? s!"invalid pattern {id.index}"
    let sourceLoc ← location context.ns node.loc
    let sourceTy ← ty context.ns node.typeId
    let patternNode : PatternNode ← match node.kind with
      | .wildcard => pure Xast.PatternNode.wildcard
      | .variable localIdValue => do
          let decl ← requireSome context.locals[localIdValue.index]?
            s!"invalid pattern local {localIdValue.index}"
          pure <| Xast.PatternNode.var decl.name
      | .tuple elements => Xast.PatternNode.tuple <$> elements.toList.mapM (pattern context)
      | .constructor name instantiations variant fields =>
          pure <| Xast.PatternNode.struct (← MoveNames.qualifiedName context.ns.tables name)
            (← genericTypeArguments context.ns instantiations) variant
            (← fields.toList.mapM (pattern context))
      | .literal constant => pure <| Xast.PatternNode.literal (← moveValue constant)
      | .range lower upper inclusive =>
          pure <| Xast.PatternNode.range (← lower.mapM moveValue) (← upper.mapM moveValue) inclusive
    return .mk sourceTy sourceLoc patternNode

  private partial def condition (context : ExprContext)
      (condition : LeanerIR.Condition) : Except String Xast.Condition := do
    let findOne (name : String) : Except String (Option Exp) :=
      condition.auxiliary.find? (·.1 == name) |>.mapM fun value => expression context value.2
    let additionalCodes ← condition.auxiliary.toList.filterMap (fun value =>
      if value.1 == "additionalCode" then some value.2 else none) |>.mapM (expression context)
    return .mk (Codec.xastConditionKind condition.kind)
      (← location context.ns condition.loc) (← condition.properties.toList.mapM (pragma context.ns))
      (← expression context condition.expression) (← findOne "abortCode") additionalCodes
      (← findOne "emitsHandle") (← findOne "emitsCondition") (← findOne "updateTarget")

  private partial def specBlock (context : ExprContext)
      (block : LeanerIR.SpecBlock) : Except String Spec := do
    let sourceLoc ← block.sourceLoc.mapM (location context.ns)
    let frame ← block.frame.mapM fun frame =>
      return .mk (← frame.modifies.toList.mapM (expression context))
        (← frame.reads.toList.mapM (typeUse context.ns)) frame.modifiesAll frame.readsAll
    return .mk sourceLoc (← block.pragmas.toList.mapM (pragma context.ns))
      (← block.conditions.toList.mapM (condition context)) frame
end

private def contract (context : ExprContext) (value : LeanerIR.FunctionContract) : Except String Spec := do
  let sourceLoc ← value.loc.mapM (location context.ns)
  let frame : Option Frame ← if value.hasFrame then do
    pure <| some <| Frame.mk (← value.modifies.toList.mapM (expression context))
      (← value.reads.toList.mapM (typeUse context.ns)) value.modifiesAll value.readsAll
  else
    pure none
  return .mk sourceLoc (← value.pragmas.toList.mapM (pragma context.ns))
    (← value.conditions.toList.mapM (condition context)) frame

private def has (values : Array LeanerIR.ProfileValue) (tag : String) : Bool :=
  values.any (·.tag == tag)

private def visibility (values : Array LeanerIR.ProfileValue) : Except String Visibility :=
  if has values "visibility.public" then pure .public
  else if has values "visibility.friend" then pure .friend
  else if has values "visibility.package" then pure .package
  else if has values "visibility.private" then pure .«private»
  else fail "function has no Move visibility"

private def functionKind (values : Array LeanerIR.ProfileValue) : Except String FunctionKind :=
  if has values "function.native" then pure .native
  else if has values "function.inlineRetained" then pure .inlineRetained
  else if has values "function.regular" then pure .regular
  else fail "function has no Move function kind"

private def parameters (ns : LeanerIR.Validation.ValidatedNamespace)
    (signature : LeanerIR.Signature) : Except String (List Param) :=
  signature.parameters.toList.mapM fun parameter =>
    return { name := parameter.name, ty := ← typeUse ns parameter.typeUse }

private def resultType (ns : LeanerIR.Validation.ValidatedNamespace)
    (signature : LeanerIR.Signature) : Except String Ty :=
  match signature.results with
  | #[result] => typeUse ns result
  | _ => fail "Move function signature does not have one result type"

private def module (ns : LeanerIR.Validation.ValidatedNamespace) : Except String Xast.Module := do
  let self ← MoveNames.moduleRef ns.tables ns.identity
  let moduleLoc ← location ns ns.loc
  let mut constants : List Constant := []
  for declaration in ns.constants do
    let valueExp ← requireSome ns.expressions[declaration.value.index]? "invalid constant value expression"
    let constant ← match valueExp.kind with
      | .value constantValue _ => moveValue constantValue
      | _ => fail "Move constant declaration does not contain a constant value"
    let name := (← MoveNames.qualifiedName ns.tables declaration.name).name
    let declarationLoc ← location ns declaration.loc
    let declarationTy ← typeUse ns declaration.type
    constants := constants ++ [{
      name
      doc := declaration.doc
      loc := declarationLoc
      ty := declarationTy
      value := constant }]
  let mut structs : List Xast.Struct := []
  for declaration in ns.structs do
    let context := ExprContext.mk ns declaration.locals #[]
    let decodeField (field : LeanerIR.FieldDecl) : Except String Field := do
      let name := (← MoveNames.qualifiedName ns.tables field.name).name
      return { name, doc := field.doc, ty := ← typeUse ns field.type }
    let fields ← declaration.fields.toList.mapM decodeField
    let variantValues ← declaration.variants.toList.mapM fun variant => do
      let name := (← MoveNames.qualifiedName ns.tables variant.name).name
      return {
        name
        loc := ← location ns variant.loc
        fields := ← variant.fields.toList.mapM decodeField }
    let intrinsic ← ns.intrinsics.find? (·.owner == declaration.name) |>.mapM fun intrinsic =>
      return {
        name := intrinsic.model
        moveFunctions := ← intrinsic.executableBindings.toList.mapM fun binding => do
          let target ← MoveNames.qualifiedRef ns.tables binding.target
          return { role := binding.role, target }
        specFunctions := ← intrinsic.specBindings.toList.mapM fun binding => do
          let target ← MoveNames.qualifiedRef ns.tables binding.target
          return { role := binding.role, target } }
    structs := structs ++ [{
      name := (← MoveNames.qualifiedName ns.tables declaration.name).name
      doc := declaration.doc
      loc := ← location ns declaration.loc
      abilities := ← declaration.abilities.toList.mapM moveAbility
      typeParams := ← declaration.generics.toList.mapM (typeParam ns)
      attributes := ← declaration.attributes.toList.mapM (decodeAttribute ns)
      isNative := has declaration.properties "struct.native"
      fields
      variants := if has declaration.properties "struct.variants" then some variantValues else none
      spec := ← contract context declaration.contract
      intrinsic }]
  let mut functions : List Function := []
  for declaration in ns.functions do
    let context := ExprContext.mk ns declaration.locals declaration.signature.parameters
    let body ← match declaration.body with
      | .absent => pure none | .structured root => some <$> expression context root
    functions := functions ++ [{
      name := (← MoveNames.qualifiedName ns.tables declaration.name).name
      doc := declaration.doc
      loc := ← location ns declaration.loc
      visibility := ← visibility declaration.profileData
      isEntry := has declaration.profileData "function.entry"
      kind := ← functionKind declaration.profileData
      isReceiver := has declaration.profileData "function.receiver"
      attributes := ← declaration.attributes.toList.mapM (decodeAttribute ns)
      typeParams := ← declaration.signature.generics.toList.mapM (typeParam ns)
      params := ← parameters ns declaration.signature
      result := ← resultType ns declaration.signature
      pragmas := ← declaration.pragmas.toList.mapM (pragma ns)
      spec := ← contract context declaration.contract
      body }]
  let mut specFuns : List SpecFun := []
  for declaration in ns.specFunctions do
    let context := ExprContext.mk ns declaration.locals declaration.signature.parameters
    specFuns := specFuns ++ [{
      name := (← MoveNames.qualifiedName ns.tables declaration.name).name
      doc := declaration.doc
      loc := ← location ns declaration.loc
      typeParams := ← declaration.signature.generics.toList.mapM (typeParam ns)
      params := ← parameters ns declaration.signature
      result := ← resultType ns declaration.signature
      uninterpreted := has declaration.profileData "specFunction.uninterpreted"
      isNative := has declaration.profileData "specFunction.native"
      isMoveFun := has declaration.profileData "specFunction.moveFunction"
      usesOld := has declaration.profileData "specFunction.usesOld"
      body := ← declaration.body.mapM (expression context)
      spec := ← contract context declaration.contract }]
  let mut specVars := []
  for declaration in ns.specVars do
    let context := ExprContext.mk ns declaration.locals #[]
    specVars := specVars ++ [{
      name := (← MoveNames.qualifiedName ns.tables declaration.name).name
      loc := ← location ns declaration.loc
      typeParams := ← declaration.generics.toList.mapM (typeParam ns)
      ty := ← typeUse ns declaration.type
      init := ← declaration.init.mapM (expression context) }]
  let mut invariants := []
  for declaration in ns.invariants do
    let context := ExprContext.mk ns declaration.locals #[]
    let (kind, typeParams) ← match declaration.condition.kind with
      | .globalInvariant parameters => pure (InvariantKind.global, parameters.toList)
      | .globalInvariantUpdate parameters => pure (.globalUpdate, parameters.toList)
      | .axiom_ parameters => pure (.«axiom», parameters.toList)
      | kind => fail s!"condition {repr kind} cannot be projected as a namespace invariant"
    invariants := invariants ++ [{
      kind
      loc := ← location ns declaration.loc
      typeParams
      properties := ← declaration.condition.properties.toList.mapM (pragma ns)
      exp := ← expression context declaration.condition.expression }]
  let metadata (tag : String) := ns.profileMetadata.toList.filter (·.tag == tag)
  let alias := (metadata "metadata.addressAlias").head?.map (·.payload)
  let namedAddresses ← (metadata "metadata.namedAddress").mapM fun value =>
    Codec.decodeNamedAddress value.payload
  let friends ← (metadata "metadata.friend").mapM fun value => Codec.decodeModuleRef value.payload
  let skipped ← (metadata "metadata.skipped").mapM fun value => do
    match ← Codec.unpack value.payload with
    | #[name, reason] => return { name, reason }
    | _ => fail "invalid skipped-declaration metadata"
  return {
    address := self.address
    addressAlias := alias
    name := self.name
    doc := ns.doc
    loc := moduleLoc
    namedAddresses
    friends
    pragmas := ← ns.pragmas.toList.mapM (pragma ns)
    constants
    structs
    functions
    specFuns
    specVars
    invariants
    skipped
    sources := ns.tables.files.toList.map (·.name)
    comments := ← ns.comments.toList.mapM fun comment =>
      return { loc := ← location ns comment.loc, text := comment.text, ownLine := comment.ownLine } }

/-- Project a validated Move LIR unit into the established backend's typed
Move package view. -/
def package (unit : LeanerIR.Validation.ValidatedUnit) : Except String Package := do
  return { modules := (← unit.namespaces.toList.mapM module) }

end Transpiler.LIR.Decode
