-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove
import Transpiler.Effects
import Transpiler.LIR.Codec

/-!
# XAST to raw LIR

This is the transitional compiler-v2 frontend.  It interns XAST semantic data
bottom-up into the neutral arenas and retains only Move-specific leaf choices
as checked profile tags.  No XAST tree is kept beside the resulting unit.
-/

namespace Transpiler.LIR.Encode

open Transpiler Xast Effects

structure BuildState where
  sourceNamespaces : Array ModuleRef := #[]
  sourceNames : Array QualifiedName := #[]
  sourceTypes : Array Xast.Ty := #[]
  sourceLocals : Array (String × Xast.Ty × LeanerIR.LocalId) := #[]
  files : Array LeanerIR.SourceFile := #[]
  fileOffset : Nat := 0
  locations : Array LeanerIR.Location := #[]
  origins : Array LeanerIR.Origin := #[]
  alignments : Array LeanerIR.Alignment := #[]
  lifetimes : Array LeanerIR.Lifetime := #[]
  types : Array LeanerIR.Ty := #[]
  names : Array LeanerIR.QualifiedName := #[]
  expressions : Array LeanerIR.Expr := #[]
  patterns : Array LeanerIR.Pattern := #[]
  places : Array LeanerIR.Place := #[]
  locals : Array LeanerIR.LocalDecl := #[]

abbrev BuildM := StateT BuildState (Except String)

private def addLocation (loc : Loc) : BuildM LeanerIR.LocId := do
  let state ← get
  let id : LeanerIR.LocId := ⟨state.locations.size⟩
  let range : LeanerIR.SourceRange := {
    file := ⟨state.fileOffset + loc.file⟩, startByte := loc.start, endByte := loc.stop }
  set { state with locations := state.locations.push { primary := some range } }
  return id

/-- Record a semantic node synthesized by the compatibility adapter when XAST
v3 provides only the containing declaration's location. -/
private def addGeneratedLocation (parent : LeanerIR.LocId) : BuildM LeanerIR.LocId := do
  let state ← get
  let id : LeanerIR.LocId := ⟨state.locations.size⟩
  set { state with locations := state.locations.push {
    generatedBy := some "Transpiler.LIR.Encode"
    parent := some parent } }
  return id

private def addModuleRef (module : ModuleRef) : BuildM LeanerIR.NamespaceId := do
  let state ← get
  if let some index := state.sourceNamespaces.findIdx? (· == module) then
    return ⟨index⟩
  let id : LeanerIR.NamespaceId := ⟨state.sourceNamespaces.size⟩
  set { state with sourceNamespaces := state.sourceNamespaces.push module }
  return id

private def addName (name : QualifiedName) : BuildM LeanerIR.NameId := do
  let state ← get
  if let some index := state.sourceNames.findIdx? (· == name) then
    return ⟨index⟩
  let namespaceId ← addModuleRef name.module
  let state ← get
  let id : LeanerIR.NameId := ⟨state.sourceNames.size⟩
  set { state with
    sourceNames := state.sourceNames.push name
    names := state.names.push { namespaceId, name := name.name } }
  return id

private def addQualifiedRef (name : QualifiedName) : BuildM LeanerIR.QualifiedRef := do
  let nameId ← addName name
  let state ← get
  let sourceName := state.sourceNames[nameId.index]!
  let namespaceId ← addModuleRef sourceName.module
  return { namespaceId, name := nameId }

private def lirAbility : Ability → LeanerIR.Ability
  | .copy => .copy
  | .drop => .drop
  | .store => .store
  | .key => .key

private def lirBorrowKind : RefKind → LeanerIR.BorrowKind
  | .immutable => .immutable
  | .mutable => .mutable

private def addInferredLifetime (loc : LeanerIR.LocId) : BuildM LeanerIR.LifetimeId := do
  let state ← get
  let id : LeanerIR.LifetimeId := ⟨state.lifetimes.size⟩
  set { state with lifetimes := state.lifetimes.push { kind := .inference, loc } }
  return id

private partial def addType (loc : LeanerIR.LocId) (source : Xast.Ty) : BuildM LeanerIR.TypeId := do
  let state ← get
  unless source matches .reference .. do
    if let some index := state.sourceTypes.findIdx? (· == source) then
      return ⟨index⟩
  let ty ← match source with
    | .bool => pure .bool
    | .u8 => pure <| .integer (.bits 8) false
    | .u16 => pure <| .integer (.bits 16) false
    | .u32 => pure <| .integer (.bits 32) false
    | .u64 => pure <| .integer (.bits 64) false
    | .u128 => pure <| .integer (.bits 128) false
    | .u256 => pure <| .integer (.bits 256) false
    | .i8 => pure <| .integer (.bits 8) true
    | .i16 => pure <| .integer (.bits 16) true
    | .i32 => pure <| .integer (.bits 32) true
    | .i64 => pure <| .integer (.bits 64) true
    | .i128 => pure <| .integer (.bits 128) true
    | .i256 => pure <| .integer (.bits 256) true
    | .address => pure .address
    | .signer => pure .signer
    | .num => pure <| .integer .unbounded true
    | .range => pure .range
    | .eventStore => pure .eventStore
    | .tuple elements => .tuple <$> (elements.toArray.mapM (addType loc))
    | .vector element => do
        let element ← addType loc element
        pure <| .vector element
    | .struct name arguments => do
        let name ← addName name
        let arguments ← arguments.toArray.mapM fun argument => do
          let typeId ← addType loc argument
          pure <| LeanerIR.GenericArgument.typeArg { typeId, loc }
        pure <| .nominal name arguments
    | .function arguments result abilities => do
        let arguments ← match arguments with
          | .tuple elements => elements.toArray.mapM (addType loc)
          | argument => do
              let argument ← addType loc argument
              pure #[argument]
        let result ← addType loc result
        pure <| .function arguments result (abilities.toArray.map lirAbility)
    | .typeParam index => pure <| .typeParameter index
    | .reference mutable ty => do
        let ty ← addType loc ty
        let lifetime ← addInferredLifetime loc
        pure <| LeanerIR.Ty.reference {
          profile := .move
          kind := if mutable then .mutable else .shared
          referent := ty
          lifetime }
    | .typeDomain ty => do
      let ty ← addType loc ty
      pure <| .typeDomain ty
    | .resourceDomain name arguments => do
        let name ← addName name
        let arguments ← arguments.mapM fun arguments => arguments.toArray.mapM (addType loc)
        pure <| .resourceDomain name arguments
    | .stateDomain => pure .stateDomain
  let state ← get
  let id : LeanerIR.TypeId := ⟨state.types.size⟩
  set { state with sourceTypes := state.sourceTypes.push source, types := state.types.push ty }
  return id

private def typeUse (loc : LeanerIR.LocId) (ty : Xast.Ty) : BuildM LeanerIR.TypeUse := do
  return { typeId := ← addType loc ty, loc }

private def generatedTypeUse (parent : LeanerIR.LocId)
    (ty : Xast.Ty) : BuildM LeanerIR.TypeUse := do
  typeUse (← addGeneratedLocation parent) ty

private def addExprNode (loc : LeanerIR.LocId) (typeId : LeanerIR.TypeId)
    (kind : LeanerIR.ExprKind) : BuildM LeanerIR.ExprId := do
  let state ← get
  let id : LeanerIR.ExprId := ⟨state.expressions.size⟩
  set { state with expressions := state.expressions.push { loc, typeId, kind } }
  return id

private def addPatternNode (loc : LeanerIR.LocId) (typeId : LeanerIR.TypeId)
    (kind : LeanerIR.PatternKind) : BuildM LeanerIR.PatternId := do
  let state ← get
  let id : LeanerIR.PatternId := ⟨state.patterns.size⟩
  set { state with patterns := state.patterns.push { loc, typeId, kind } }
  return id

/-- Move specification functions erase one reference layer and interpret a
direct fixed-width integer parameter or result in MSL's mathematical integer
domain.  Nested integers remain part of their enclosing data representation. -/
private def specificationType : Xast.Ty → Xast.Ty
  | .reference _ ty => specificationType ty
  | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 |
      .i8 | .i16 | .i32 | .i64 | .i128 | .i256 | .num => .num
  | ty => ty

private def localId (name : String) (ty : Xast.Ty) (loc : LeanerIR.LocId) : BuildM LeanerIR.LocalId := do
  let state ← get
  -- Exact identity first: distinct executable locals must never unify, or a
  -- use site's annotated type contradicts the shared declaration.
  if let some (_, _, id) := state.sourceLocals.find? fun entry =>
      entry.1 == name && entry.2.1 == ty then
    return id
  -- A specification occurrence arrives already projected (dereferenced, with
  -- mathematical integers) and may refer to the executable local it names.
  -- Executable occurrences whose type is not projection-normal never take
  -- this fallback.
  if specificationType ty == ty then
    if let some (_, _, id) := state.sourceLocals.find? fun entry =>
        entry.1 == name && specificationType entry.2.1 == ty then
      return id
  let type ← typeUse loc ty
  let state ← get
  let id : LeanerIR.LocalId := ⟨state.locals.size⟩
  set { state with
    sourceLocals := state.sourceLocals.push (name, ty, id)
    locals := state.locals.push { id, name, type, loc } }
  return id

private def addParameter (parameter : Param) (loc : LeanerIR.LocId) : BuildM LeanerIR.Parameter := do
  let loc ← addGeneratedLocation loc
  let id ← localId parameter.name parameter.ty loc
  let state ← get
  let some declaration := state.locals[id.index]?
    | throw s!"parameter `{parameter.name}` has no local declaration"
  return { name := parameter.name, typeUse := declaration.type }

private def addSpecificationParameter (parameter : Param)
    (loc : LeanerIR.LocId) : BuildM LeanerIR.Parameter := do
  addParameter { parameter with ty := specificationType parameter.ty } loc

private def resetLocals : BuildM Unit := modify fun state => {
  state with sourceLocals := #[], locals := #[] }

private def lirOperation : Xast.Operation → BuildM LeanerIR.Operation
  | .moveFunction name => return .call (.function (← addQualifiedRef name))
  | .pack name variant => return .call (.constructor (← addQualifiedRef name) variant)
  | .exists _ => return .global .contains
  | .borrowGlobal kind => return .global (.borrow (lirBorrowKind kind))
  | .moveFrom => return .global .take
  | .moveTo => return .global .publish
  | .borrow kind => return .reference (.borrow (lirBorrowKind kind))
  | .deref => return .reference .dereference
  | .freeze explicit => return .reference (.freeze explicit)
  | .select name field => return .data (.select (← addQualifiedRef name) field)
  | .selectVariants name fields =>
      return .data (.selectVariants (← addQualifiedRef name) fields.toArray)
  | .testVariants name variants =>
      return .data (.testVariants (← addQualifiedRef name) variants.toArray)
  | .updateField name field => return .data (.updateField (← addQualifiedRef name) field)
  | .specFunction name range =>
      return .specification (.functionCall (← addQualifiedRef name) {
        pre := range.pre, post := range.post })
  | operation => do
      match Codec.primitiveOperation? operation with
      | some primitive => return .primitive primitive
      | none =>
          match Codec.specOperation? operation with
          | some specification => return .specification specification
          | none =>
              let encoded := Codec.encodeOperation operation
              let targets ← encoded.targets.mapM addQualifiedRef
              return .profile {
                profile := .move, tag := encoded.tag, payload := encoded.payload } targets

private def surfaceSyntax : Option Xast.SurfaceSyntax → Option LeanerIR.SurfaceSyntax
  | none => none
  | some .receiverCall => some .receiverCall
  | some .indexNotation => some .indexNotation

private partial def constValue : Value → LeanerIR.ConstValue
  | .address value => .address value
  | .number value => .integer value
  | .bool value => .bool value
  | .vector elements => .vector (elements.toArray.map constValue)
  | .tuple elements => .tuple (elements.toArray.map constValue)

private def toPragma (pragma : Xast.Pragma) : LeanerIR.Attribute :=
  .assign pragma.name (match pragma.value with
    | .value value => .constant (constValue value)
    | .name name => .name none name
    | .qualifiedName name => .qualifiedName name)

mutual
  private partial def addExpr : Exp → BuildM LeanerIR.ExprId
    | .mk ty sourceLoc node => do
        let loc ← addLocation sourceLoc
        let typeId ← addType loc ty
        let kind ← match node with
          | .value value constant => pure <| .value (constValue value) constant
          | .«local» name => .localVar <$> localId name ty loc
          | .param index => pure <| .localVar ⟨index⟩
          | .call operation inst arguments surface => do
              let inst := if inst.isEmpty then
                  match operation, ty, arguments with
                  | .slice, .vector element, _ => [element]
                  | .len, _, .mk (.vector element) _ _ :: _ => [element]
                  | .len, _, .mk (.reference _ (.vector element)) _ _ :: _ => [element]
                  | .index, _, .mk (.vector element) _ _ :: _ => [element]
                  | .index, _, .mk (.reference _ (.vector element)) _ _ :: _ => [element]
                  | _, _, _ => inst
                else inst
              let instantiations ← match operation with
                -- Compiler-v2 retains the physical Move type in `old`'s
                -- inferred node instantiation even when its specification
                -- expression has already been projected (for example `u64`
                -- to mathematical `num`). LIR defines this generic argument
                -- as the operation's value/result type, so normalize it at
                -- the source boundary.
                | .old => do
                    let instantiationLoc ← addGeneratedLocation loc
                    pure #[.typeArg { typeId, loc := instantiationLoc }]
                | _ => inst.toArray.mapM fun ty =>
                    return LeanerIR.GenericArgument.typeArg (← generatedTypeUse loc ty)
              let sourceArguments := arguments
              let arguments ← sourceArguments.toArray.mapM addExpr
              let dereferenceFirst (referent : Xast.Ty) (first : LeanerIR.ExprId) := do
                let dereferenceLoc ← addGeneratedLocation loc
                let referentType ← addType dereferenceLoc referent
                let dereference ← addExprNode dereferenceLoc referentType <|
                  .operation (.reference .dereference) #[] #[first]
                pure (arguments.set! 0 dereference)
              let arguments ← match operation, ty, sourceArguments, arguments[0]? with
                -- A Move field borrow is represented by compiler-v2 as a
                -- selection whose operand and result are references. Keep
                -- that reference intact so LIR can project the borrow onto
                -- the selected field. By-value selection still needs the
                -- historical implicit dereference normalization below.
                | .select .., .reference .., .mk (.reference ..) _ _ :: _, _ => pure arguments
                | .select .., _, .mk (.reference _ referent) _ _ :: _, some first =>
                    dereferenceFirst referent first
                | .selectVariants .., _, .mk (.reference _ referent) _ _ :: _, some first =>
                    dereferenceFirst referent first
                | .testVariants .., _, .mk (.reference _ referent) _ _ :: _, some first =>
                    dereferenceFirst referent first
                | _, _, _, _ => pure arguments
              match operation with
              | .abort _ => pure <| .throw_ .abort arguments
              | operation =>
                  pure <| .operation (← lirOperation operation) instantiations arguments
                    (surfaceSyntax surface)
          | .invoke function arguments => do
              let arguments ← (function :: arguments).toArray.mapM addExpr
              pure <| .operation (.call .invoke) #[] arguments
          | .block pattern binding body => do
              pure <| .letDecl (← addPattern pattern) (← binding.mapM addExpr) (← addExpr body)
          | .ite condition thenBranch elseBranch =>
              pure <| .ifElse (← addExpr condition) (← addExpr thenBranch) (some (← addExpr elseBranch))
          | .«match» scrutinee arms => do
              let scrutinee ← addExpr scrutinee
              let arms ← arms.toArray.mapM fun arm => match arm with
                | .mk _ pattern guard body => do
                    let pattern ← addPattern pattern
                    let guard ← guard.mapM addExpr
                    let body ← addExpr body
                    return { pattern, guard, body }
              pure <| .match_ scrutinee arms
          | .sequence expressions =>
              let expressions ← expressions.toArray.mapM addExpr
              match expressions.back? with
              | none => pure <| .block #[] none
              | some result => pure <| .block (expressions.pop) (some result)
          | .loop body => pure <| .loop none (← addExpr body)
          | .loopCont nest isContinue =>
              pure <| if isContinue then .continue_ nest else .break_ nest none
          | .«return» value => pure <| .return_ #[← addExpr value]
          | .assign pattern value => pure <| .assignPattern (← addPattern pattern) (← addExpr value)
          | .mutate target value =>
              pure <| .operation (.reference .mutate)
                #[] #[← addMutationTarget target, ← addExpr value]
          | .specBlock spec => .spec <$> addSpecBlock spec loc
          | .quant kind ranges triggers condition body => do
              let kind := match kind with
                | .forall => LeanerIR.QuantifierKind.forall
                | .exists => .exists
                | .choose => .choose
                | .chooseMin => .chooseMin
              let binders ← ranges.toArray.mapM fun range => match range with
                | .mk pattern domain => return { pattern := ← addPattern pattern, domain := ← addExpr domain }
              let triggers ← triggers.toArray.mapM fun trigger => trigger.toArray.mapM addExpr
              pure <| .quantifier kind binders triggers (← condition.mapM addExpr) (← addExpr body)
        addExprNode loc typeId kind

  /-- Compiler-v2 represents assignment targets as value-typed XAST place
  trees. LIR mutation instead consumes a concrete reference. Reconstruct that
  reference while the place tree is still available, preserving a global or
  explicit borrow at its root and projecting it through struct fields. -/
  private partial def addMutationTarget : Exp → BuildM LeanerIR.ExprId
    | .mk fieldType sourceLoc
        (.call (.select owner field) inst [base] surface) => do
        let loc ← addLocation sourceLoc
        let typeId ← addType loc (.reference true fieldType)
        let instantiations ← inst.toArray.mapM fun ty =>
          return LeanerIR.GenericArgument.typeArg (← generatedTypeUse loc ty)
        let base ← addMutationTarget base
        addExprNode loc typeId <| .operation
          (.data (.select (← addQualifiedRef owner) field)) instantiations #[base]
            (surfaceSyntax surface)
    | .mk _ _ (.call .deref _ [reference] _) => addExpr reference
    | source@(.mk ty sourceLoc (.«local» _))
    | source@(.mk ty sourceLoc (.param _)) => do
        match ty with
        | .reference .. => addExpr source
        | _ =>
            let loc ← addLocation sourceLoc
            let typeId ← addType loc (.reference true ty)
            let value ← addExpr source
            addExprNode loc typeId <| .operation
              (.reference (.borrow .mutable)) #[] #[value]
    | source => addExpr source

  private partial def addPattern : Pattern → BuildM LeanerIR.PatternId
    | .mk ty sourceLoc node => do
        let loc ← addLocation sourceLoc
        let typeId ← addType loc ty
        let kind ← match node with
          | .var name => .variable <$> localId name ty loc
          | .wildcard => pure .wildcard
          | .tuple elements => .tuple <$> elements.toArray.mapM addPattern
          | .struct name inst variant fields => do
              let name ← addName name
              let instantiations ← inst.toArray.mapM fun ty =>
                return LeanerIR.GenericArgument.typeArg (← generatedTypeUse loc ty)
              pure <| .constructor name instantiations variant (← fields.toArray.mapM addPattern)
          | .literal value => pure <| .literal (constValue value)
          | .range lower upper inclusive =>
              pure <| .range (lower.map constValue) (upper.map constValue) inclusive
        addPatternNode loc typeId kind

  private partial def addCondition (condition : Xast.Condition) : BuildM LeanerIR.Condition :=
    match condition with
    | .mk kind sourceLoc properties expression abortCode additionalCodes emitsHandle emitsCondition updateTarget => do
        let loc ← addLocation sourceLoc
        let mut auxiliary := #[]
        if let some value := abortCode then auxiliary := auxiliary.push ("abortCode", ← addExpr value)
        for value in additionalCodes do auxiliary := auxiliary.push ("additionalCode", ← addExpr value)
        if let some value := emitsHandle then auxiliary := auxiliary.push ("emitsHandle", ← addExpr value)
        if let some value := emitsCondition then auxiliary := auxiliary.push ("emitsCondition", ← addExpr value)
        if let some value := updateTarget then auxiliary := auxiliary.push ("updateTarget", ← addExpr value)
        return {
          loc
          kind := Codec.lirConditionKind kind
          properties := properties.toArray.map toPragma
          expression := ← addExpr expression
          auxiliary }

  private partial def addFrame (frame : Xast.Frame) (loc : LeanerIR.LocId) : BuildM LeanerIR.Frame :=
    match frame with
    | .mk modifies reads modifiesAll readsAll =>
        return {
          modifies := ← modifies.toArray.mapM addExpr
          reads := ← reads.toArray.mapM (generatedTypeUse loc)
          modifiesAll
          readsAll }

  private partial def addSpecBlock (spec : Xast.Spec) (fallback : LeanerIR.LocId) : BuildM LeanerIR.SpecBlock :=
    match spec with
    | .mk sourceLoc pragmas conditions frame => do
        let sourceLoc ← sourceLoc.mapM addLocation
        let loc := sourceLoc.getD fallback
        return {
          loc
          sourceLoc
          pragmas := pragmas.toArray.map toPragma
          conditions := ← conditions.toArray.mapM addCondition
          frame := ← frame.mapM (addFrame · loc) }
end

private partial def addAttribute : Xast.Attribute → BuildM LeanerIR.Attribute
  | .apply name arguments =>
      return .call name (← arguments.toArray.mapM addAttribute)
  | .assign name (.value value) =>
      return .assign name (.constant (constValue value))
  | .assign name (.name module nameValue) =>
      return .assign name (.name (← module.mapM addModuleRef) nameValue)

/-- These Move compiler attributes decide which source declarations enter a
build mode. Once compiler-v2 has included the declaration in XAST they have no
remaining runtime or specification meaning in LIR. -/
private def isResolvedBuildSelectionAttribute : Xast.Attribute → Bool
  | .apply name [] => name == "test" || name == "test_only" || name == "verify_only"
  | _ => false

private def addGenericBinder (parent : LeanerIR.LocId)
    (parameter : TypeParam) : BuildM LeanerIR.GenericBinder := do
  let loc ← addGeneratedLocation parent
  return {
    name := parameter.name
    kind := .typeArg
    abilities := parameter.abilities.toArray.map lirAbility
    predicates := if parameter.isPhantom then
      #[.profile (LeanerIR.Move.propertyValue "typeParameter.phantom")] else #[]
    loc }

private def functionProperty (tag : String) : LeanerIR.ProfileValue :=
  LeanerIR.Move.propertyValue tag

private def visibilityProperty : Visibility → LeanerIR.ProfileValue
  | .«private» => functionProperty "visibility.private"
  | .public => functionProperty "visibility.public"
  | .friend => functionProperty "visibility.friend"
  | .package => functionProperty "visibility.package"

private def functionKindProperty : FunctionKind → LeanerIR.ProfileValue
  | .regular => functionProperty "function.regular"
  | .inlineRetained => functionProperty "function.inlineRetained"
  | .native => functionProperty "function.native"

/-- A source declaration which can own ordinary comments. Retained inline
functions are valid XAST anchors even though compiler-v2 has already expanded
them and the LIR adapter intentionally omits their declarations. -/
private structure CommentAnchor where
  loc : Loc
  retained : Bool

private def commentAnchors (module : Module) (inlineFunctions : List String) :
    Array CommentAnchor :=
  let anchors :=
    module.constants.toArray.map (fun declaration => { loc := declaration.loc, retained := true }) ++
    module.structs.toArray.map (fun declaration => { loc := declaration.loc, retained := true }) ++
    module.functions.toArray.map (fun declaration =>
      { loc := declaration.loc, retained := declaration.kind != .inlineRetained }) ++
    module.specFuns.toArray.map (fun declaration =>
      { loc := declaration.loc,
        retained := !(declaration.isMoveFun && inlineFunctions.contains declaration.name) }) ++
    module.specVars.toArray.map (fun declaration => { loc := declaration.loc, retained := true }) ++
    module.invariants.toArray.map (fun declaration => { loc := declaration.loc, retained := true })
  anchors.qsort fun left right =>
    left.loc.file < right.loc.file ||
      (left.loc.file == right.loc.file && left.loc.start < right.loc.start)

private def commentOwner? (anchors : Array CommentAnchor) (comment : Comment) :
    Option CommentAnchor :=
  let containing := anchors.filter fun anchor =>
    anchor.loc.file == comment.loc.file && anchor.loc.start <= comment.loc.start &&
      comment.loc.stop <= anchor.loc.stop
  match containing[0]? with
  | some first => some <| containing.foldl (init := first) fun best candidate =>
      let bestSize := best.loc.stop - best.loc.start
      let candidateSize := candidate.loc.stop - candidate.loc.start
      if candidateSize < bestSize ||
          (candidateSize == bestSize && !candidate.retained) then candidate else best
  | none => anchors.find? fun anchor =>
      anchor.loc.file == comment.loc.file && comment.loc.stop <= anchor.loc.start

/-- Keep comments only when the declaration they belong to also survives the
XAST-to-LIR boundary. Without this ownership pass, comments from several
expanded inline functions accumulate before the next retained declaration and
look like duplicated module-level comments. Unanchored comments remain free
standing and are preserved. -/
private def retainedComments (module : Module) (inlineFunctions : List String) :
    List Comment :=
  let anchors := commentAnchors module inlineFunctions
  module.comments.filter fun comment =>
    (commentOwner? anchors comment).all (·.retained)

private def addContract (spec : Xast.Spec) (fallback : LeanerIR.LocId) : BuildM LeanerIR.FunctionContract := do
  let block ← addSpecBlock spec fallback
  return {
    loc := block.sourceLoc
    conditions := block.conditions
    modifies := block.frame.map (·.modifies) |>.getD #[]
    reads := block.frame.map (·.reads) |>.getD #[]
    hasFrame := block.frame.isSome
    modifiesAll := block.frame.any (·.modifiesAll)
    readsAll := block.frame.any (·.readsAll)
    pragmas := block.pragmas }

private def ownName (module : Xast.Module) (name : String) : QualifiedName :=
  { module := module.ref, name }

private def buildNamespace (unitIndex : Nat) (module : Xast.Module) :
    BuildM LeanerIR.Import.RawNamespace := do
    let state ← get
    let moduleFiles := if module.sources.isEmpty then #[{ name := "?" }] else
      module.sources.toArray.map fun source => { name := source }
    let fileOffset := state.files.size
    set { state with
      files := state.files ++ moduleFiles
      fileOffset
      expressions := #[]
      patterns := #[]
      places := #[]
      locals := #[] }
    let _ ← addModuleRef module.ref
    let loc ← addLocation module.loc
    let state ← get
    let origin : LeanerIR.OriginId := ⟨state.origins.size⟩
    let alignment : LeanerIR.AlignmentId := ⟨state.alignments.size⟩
    set { state with
      origins := state.origins.push {
        kind := .moveSource
        location := loc
        sourceIdentity := module.sources.head? }
      alignments := state.alignments.push {
        source := origin
        trust := .checked
        description := "compiler-v2 XAST v3" } }
    let mut constants : Array LeanerIR.ConstantDecl := #[]
    for constant in module.constants do
      let constantLoc ← addLocation constant.loc
      let typeUseValue ← generatedTypeUse constantLoc constant.ty
      let valueLoc ← addGeneratedLocation constantLoc
      let value ← addExprNode valueLoc typeUseValue.typeId (.value (constValue constant.value))
      constants := constants.push {
        loc := constantLoc
        name := ← addName (ownName module constant.name)
        type := typeUseValue
        value
        doc := constant.doc }
    let mut structs : Array LeanerIR.StructDecl := #[]
    let mut intrinsics : Array LeanerIR.IntrinsicDecl := #[]
    for structDecl in module.structs do
      resetLocals
      let structLoc ← addLocation structDecl.loc
      let fields ← structDecl.fields.toArray.mapM fun field => do
        let fieldLoc ← addGeneratedLocation structLoc
        let name ← addName (ownName module field.name)
        let fieldType ← typeUse fieldLoc field.ty
        return { loc := fieldLoc, name, type := fieldType, doc := field.doc }
      let variants ← (structDecl.variants.getD []).toArray.mapM fun variant => do
        let variantLoc ← addLocation variant.loc
        let fields ← variant.fields.toArray.mapM fun field => do
          let fieldLoc ← addGeneratedLocation variantLoc
          let name ← addName (ownName module field.name)
          let fieldType ← typeUse fieldLoc field.ty
          return { loc := fieldLoc, name, type := fieldType, doc := field.doc }
        let name ← addName (ownName module variant.name)
        return { loc := variantLoc, name, fields }
      let owner ← addName (ownName module structDecl.name)
      let structContract ← addContract structDecl.spec structLoc
      let locals := (← get).locals
      let attributes ← structDecl.attributes.toArray
        |>.filter (!isResolvedBuildSelectionAttribute ·) |>.mapM addAttribute
      structs := structs.push {
        loc := structLoc
        name := owner
        doc := structDecl.doc
        generics := ← structDecl.typeParams.toArray.mapM (addGenericBinder structLoc)
        fields
        variants
        abilities := structDecl.abilities.toArray.map lirAbility
        properties := (if structDecl.isNative then #[functionProperty "struct.native"] else #[]) ++
          (if structDecl.variants.isSome then #[functionProperty "struct.variants"] else #[])
        locals
        contract := structContract
        attributes }
      if let some intrinsic := structDecl.intrinsic then
        let intrinsicLoc ← addGeneratedLocation structLoc
        let executableBindings ← intrinsic.moveFunctions.toArray.mapM fun binding => do
          let bindingLoc ← addGeneratedLocation intrinsicLoc
          let target ← addQualifiedRef binding.target
          return { loc := bindingLoc, role := binding.role, target }
        let specBindings ← intrinsic.specFunctions.toArray.mapM fun binding => do
          let bindingLoc ← addGeneratedLocation intrinsicLoc
          let target ← addQualifiedRef binding.target
          return { loc := bindingLoc, role := binding.role, target }
        intrinsics := intrinsics.push {
          loc := intrinsicLoc, model := intrinsic.name, owner, profile := .move,
          executableBindings, specBindings }
    let inlineFunctions := module.functions.filterMap fun function =>
      if function.kind == .inlineRetained then some function.name else none
    let mut functions : Array (LeanerIR.FunctionDecl LeanerIR.Import.RawBody) := #[]
    for function in module.functions do
      if function.kind == .inlineRetained then continue
      resetLocals
      let functionLoc ← addLocation function.loc
      let parameters ← function.params.toArray.mapM (addParameter · functionLoc)
      let resultLoc ← addGeneratedLocation functionLoc
      let result ← typeUse resultLoc function.result
      let body ← function.body.mapM addExpr
      let functionContract ← addContract function.spec functionLoc
      let locals := (← get).locals
      let attributes ← function.attributes.toArray
        |>.filter (!isResolvedBuildSelectionAttribute ·) |>.mapM addAttribute
      let mut profileData := #[visibilityProperty function.visibility, functionKindProperty function.kind]
      if function.isEntry then profileData := profileData.push (functionProperty "function.entry")
      if function.isReceiver then profileData := profileData.push (functionProperty "function.receiver")
      functions := functions.push {
        loc := functionLoc
        name := ← addName (ownName module function.name)
        doc := function.doc
        profile := .move
        signature := {
          generics := ← function.typeParams.toArray.mapM (addGenericBinder functionLoc)
          parameters
          results := #[result] }
        body := body.map LeanerIR.Import.RawBody.structured |>.getD .absent
        origin
        alignment
        locals
        contract := functionContract
        pragmas := function.pragmas.toArray.map toPragma
        profileData
        attributes }
    let mut specFunctions : Array LeanerIR.SpecFunctionDecl := #[]
    for function in module.specFuns do
      if function.isMoveFun && inlineFunctions.contains function.name then continue
      resetLocals
      let functionLoc ← addLocation function.loc
      let parameters ← function.params.toArray.mapM (addSpecificationParameter · functionLoc)
      let resultLoc ← addGeneratedLocation functionLoc
      let result ← typeUse resultLoc (specificationType function.result)
      let body ← function.body.mapM addExpr
      let functionContract ← addContract function.spec functionLoc
      let locals := (← get).locals
      let mut profileData := #[]
      if function.uninterpreted then profileData := profileData.push (functionProperty "specFunction.uninterpreted")
      if function.isNative then profileData := profileData.push (functionProperty "specFunction.native")
      if function.isMoveFun then profileData := profileData.push (functionProperty "specFunction.moveFunction")
      if function.usesOld then profileData := profileData.push (functionProperty "specFunction.usesOld")
      specFunctions := specFunctions.push {
        loc := functionLoc
        name := ← addName (ownName module function.name)
        doc := function.doc
        profile := .move
        signature := {
          generics := ← function.typeParams.toArray.mapM (addGenericBinder functionLoc)
          parameters
          results := #[result] }
        body
        origin
        locals
        contract := functionContract
        profileData }
    let mut specVars : Array LeanerIR.SpecVarDecl := #[]
    for specVar in module.specVars do
      resetLocals
      let variableLoc ← addLocation specVar.loc
      let init ← specVar.init.mapM addExpr
      let locals := (← get).locals
      let typeLoc ← addGeneratedLocation variableLoc
      specVars := specVars.push {
        loc := variableLoc
        name := ← addName (ownName module specVar.name)
        generics := ← specVar.typeParams.toArray.mapM (addGenericBinder variableLoc)
        type := ← typeUse typeLoc specVar.ty
        profile := .move
        init
        locals }
    let mut invariants : Array LeanerIR.NamespaceInvariant := #[]
    for invariant in module.invariants do
      resetLocals
      let invariantLoc ← addLocation invariant.loc
      let kind := match invariant.kind with
        | .global => LeanerIR.ConditionKind.globalInvariant invariant.typeParams.toArray
        | .globalUpdate => .globalInvariantUpdate invariant.typeParams.toArray
        | .«axiom» => .axiom_ invariant.typeParams.toArray
      let expression ← addExpr invariant.exp
      let locals := (← get).locals
      invariants := invariants.push {
        loc := invariantLoc
        condition := {
          loc := invariantLoc
          kind
          properties := invariant.properties.toArray.map toPragma
          expression }
        locals }
    let mut profileMetadata := #[]
    if let some alias := module.addressAlias then
      profileMetadata := profileMetadata.push (LeanerIR.Move.propertyValue "metadata.addressAlias" alias)
    for address in module.namedAddresses do
      profileMetadata := profileMetadata.push
        (LeanerIR.Move.propertyValue "metadata.namedAddress" (Codec.encodeNamedAddress address))
    for friend in module.friends do
      profileMetadata := profileMetadata.push
        (LeanerIR.Move.propertyValue "metadata.friend" (Codec.encodeModuleRef friend))
    for skipped in module.skipped do
      -- Compiler-v2 has already expanded retained inline functions at their
      -- call sites. Their retained declarations and companion diagnostics are
      -- producer bookkeeping, not unsupported declarations in target LIR.
      let expandedInlineDiagnostic :=
        skipped.reason.startsWith s!"in function `{module.name}::{skipped.name}`:"
      if inlineFunctions.contains skipped.name || expandedInlineDiagnostic then continue
      profileMetadata := profileMetadata.push
        (LeanerIR.Move.propertyValue "metadata.skipped" (Codec.pack #[skipped.name, skipped.reason]))
    let comments : Array LeanerIR.Comment ←
        (retainedComments module inlineFunctions).toArray.mapM fun comment => do
      let commentLoc ← addLocation comment.loc
      return { loc := commentLoc, text := comment.text, ownLine := comment.ownLine }
    let state ← get
    return {
      loc
      identity := ⟨unitIndex⟩
      profile := some .move
      doc := module.doc
      expressions := state.expressions
      patterns := state.patterns
      places := state.places
      profileMetadata
      pragmas := module.pragmas.toArray.map toPragma
      constants
      structs
      functions
      specFunctions
      specVars
      invariants
      intrinsics
      comments }

/-- Translate a complete compiler-v2 package into the only public LIR input
boundary. -/
def package (package : Package) : Except String LeanerIR.Import.RawUnit := do
  let ownedNamespaces := package.modules.toArray.map (·.ref)
  let initial : BuildState := { sourceNamespaces := ownedNamespaces }
  let (namespaces, finalState) ← (package.modules.toArray.mapIdxM fun index module =>
    buildNamespace index module).run initial
  let namespaceRefs := finalState.sourceNamespaces.map fun ref =>
    let segments := match ref.addressAlias with
      | some alias => #[ref.address, alias, ref.name]
      | none => #[ref.address, ref.name]
    { segments }
  let evidence : Array LeanerIR.Import.ImportEvidence := #[{
    producer := "move exchange --format ast"
    description := "typed XAST v3 imported into neutral LIR" }]
  return {
    tables := {
      files := finalState.files
      locations := finalState.locations
      origins := finalState.origins
      alignments := finalState.alignments
      lifetimes := finalState.lifetimes
      types := finalState.types
      namespaces := namespaceRefs
      names := finalState.names }
    profiles := #[LeanerIR.Move.config]
    namespaces
    evidence }

end Transpiler.LIR.Encode
