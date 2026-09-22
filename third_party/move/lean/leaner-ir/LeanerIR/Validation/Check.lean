-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Import.Structurize
import LeanerIR.Validation.Diagnostic
import LeanerIR.Validation.Validated
import LeanerIR.Validation.Profiles
import LeanerIR.Validation.Capability
import LeanerIR.Validation.NominalCycles

/-!
# Shared checked-construction boundary

The checker owns backend-independent structural validation.  Profile packages
register closed checks for their tagged values; frontends cannot bypass those
checks or construct `ValidatedUnit` directly.
-/

namespace LeanerIR.Validation

open LeanerIR.Import

private def invalidId (kind : String) (index size : Nat) (loc : Option LocId := none) : Diagnostic :=
  .error "LIR-ID-BOUNDS" s!"invalid {kind} id {index}; table has {size} entries" loc

private def checkIndex (kind : String) (index size : Nat) (loc : Option LocId := none) : Array Diagnostic :=
  if index < size then #[] else #[invalidId kind index size loc]

private def checkLoc (tables : Tables) (id : LocId) : Array Diagnostic :=
  checkIndex "location" id.index tables.locations.size

private def checkName (tables : Tables) (id : NameId) (loc : Option LocId := none) : Array Diagnostic :=
  checkIndex "name" id.index tables.names.size loc

private def checkDeclarationNames (tables : Tables) (owner : NamespaceId) (kind : String)
    (declarations : Array (NameId × LocId)) : Array Diagnostic :=
  declarations.zipIdx.foldl (init := #[]) fun ds (declaration, index) =>
    match tables.names[declaration.1.index]? with
    | none => ds
    | some name =>
        let ds := if name.namespaceId == owner then ds else ds.push <| .at
          "LIR-DECLARATION-NAME-OWNER"
          s!"{kind} declaration `{name.name}` is interned in namespace {name.namespaceId.index}, expected {owner.index}"
          declaration.2
        if declarations.take index |>.any fun previous =>
            tables.names[previous.1.index]? == some name then
          ds.push <| .at "LIR-DECLARATION-NAME-DUPLICATE"
            s!"duplicate {kind} declaration `{name.name}`" declaration.2
        else ds

private def checkTypeId (tables : Tables) (id : TypeId) (loc : Option LocId := none) : Array Diagnostic :=
  checkIndex "type" id.index tables.types.size loc

private def checkLifetimeId (tables : Tables) (id : LifetimeId)
    (loc : Option LocId := none) : Array Diagnostic :=
  checkIndex "lifetime" id.index tables.lifetimes.size loc

private def checkExprId {β : Type} (ns : Namespace β) (id : ExprId) (loc : Option LocId := none) : Array Diagnostic :=
  checkIndex "expression" id.index ns.expressions.size loc

private def checkPatternId {β : Type} (ns : Namespace β) (id : PatternId) (loc : Option LocId := none) : Array Diagnostic :=
  checkIndex "pattern" id.index ns.patterns.size loc

private def checkPlaceId {β : Type} (ns : Namespace β) (id : PlaceId) (loc : Option LocId := none) : Array Diagnostic :=
  checkIndex "place" id.index ns.places.size loc

private def checkProfileId (unit : RawUnit) (id : ProfileId) (loc : Option LocId := none) : Array Diagnostic :=
  checkIndex "profile" id.index unit.profiles.size loc

private def checkProfile (unit : RawUnit) (profile : Profile)
    (loc : Option LocId := none) : Array Diagnostic :=
  let bounds := match profile with
    | .extension id => checkProfileId unit id loc
    | .move | .rust => #[]
  if (profileConfig? unit.profiles profile).isSome then bounds
  else bounds.push <| .error "LIR-PROFILE-CONFIG"
    s!"profile {repr profile} has no matching configuration" loc

private def checkNamespaceId (unit : RawUnit) (id : NamespaceId) (loc : Option LocId := none) : Array Diagnostic :=
  checkIndex "namespace" id.index unit.tables.namespaces.size loc

private def checkProfileValue (registry : ProfileRegistry) (unit : RawUnit)
    (value : ProfileValue) (check : ProfileSchema → ProfileValue → Array Diagnostic) : Array Diagnostic :=
  let bounds := checkProfile unit value.profile
  match profileSchema? registry value.profile with
  | none => bounds.push <| .error "LIR-PROFILE-UNREGISTERED"
      s!"profile {repr value.profile} has no registered schema"
  | some schema =>
      if let some config := profileConfig? unit.profiles value.profile then
        if config.name != schema.name || config.version != schema.version then
          bounds.push <| .error "LIR-PROFILE-VERSION"
            s!"profile configuration {config.name}@{config.version} does not match registered {schema.name}@{schema.version}"
        else
          bounds ++ check schema value
      else
        bounds

private def checkTypeUse (tables : Tables) (use : TypeUse) : Array Diagnostic :=
  checkLoc tables use.loc ++ checkTypeId tables use.typeId (some use.loc)

private def checkQualifiedRef (_unit : RawUnit) (tables : Tables) (ref : QualifiedRef)
    (loc : Option LocId := none) : Array Diagnostic :=
  let bounds := checkIndex "namespace reference" ref.namespaceId.index tables.namespaces.size loc ++
    checkName tables ref.name loc
  match tables.names[ref.name.index]? with
  | some name =>
      if name.namespaceId == ref.namespaceId then bounds
      else bounds.push <| .error "LIR-QUALIFIED-REF"
        "qualified reference namespace disagrees with its interned name" loc
  | none => bounds

private def checkReferenceType (registry : ProfileRegistry) (unit : RawUnit)
    (tables : Tables) (reference : ReferenceType) : Array Diagnostic :=
  let bounds := checkProfile unit reference.profile ++
    checkTypeId tables reference.referent ++ checkLifetimeId tables reference.lifetime
  match profileSchema? registry reference.profile, profileConfig? unit.profiles reference.profile with
  | some schema, some config =>
      if config.name != schema.name || config.version != schema.version then
        bounds.push <| .error "LIR-PROFILE-VERSION"
          s!"reference profile configuration {config.name}@{config.version} does not match registered {schema.name}@{schema.version}"
      else
        bounds ++ schema.checkReference reference
  | none, _ => bounds.push <| .error "LIR-PROFILE-UNREGISTERED"
      s!"reference profile {repr reference.profile} has no registered schema"
  | _, none => bounds

private def checkAssociatedItemRef (unit : RawUnit) (trait : TraitRef)
    (item : AssociatedItemId) : Array Diagnostic :=
  match unit.namespaces.find? (fun ns => ns.identity == trait.trait.namespaceId) with
  | some target => checkIndex "associated item" item.index target.associatedItems.size
  | none => #[]

mutual
  private partial def checkConstValue (registry : ProfileRegistry) (unit : RawUnit)
      (tables : Tables) : ConstValue → Array Diagnostic
    | .vector elements | .tuple elements =>
        elements.foldl (fun ds element => ds ++ checkConstValue registry unit tables element) #[]
    | .character value => if isUnicodeScalar value then #[] else
        #[.error "LIR-CONSTANT-WELL-FORMED"
          s!"character constant {value} is not a Unicode scalar value"]
    | .profile value => checkProfileValue registry unit value ProfileSchema.checkProperty
    | _ => #[]

  private partial def checkGenericArgument (registry : ProfileRegistry) (unit : RawUnit)
      (tables : Tables) : GenericArgument → Array Diagnostic
    | .typeArg value => checkTypeUse tables value
    | .const value => checkConstValue registry unit tables value
    | .lifetime lifetime => checkLifetimeId tables lifetime
    | .evidence _ => #[]

  private partial def checkTraitRef (registry : ProfileRegistry) (unit : RawUnit)
      (tables : Tables) (trait : TraitRef) : Array Diagnostic :=
    checkQualifiedRef unit tables trait.trait ++ trait.arguments.foldl (fun ds argument =>
      ds ++ checkGenericArgument registry unit tables argument) #[]

  private partial def checkGenericPredicate (registry : ProfileRegistry) (unit : RawUnit)
      (tables : Tables) (_associatedItemCount : Nat) : GenericPredicate → Array Diagnostic
    | .ability type _ => checkTypeId tables type
    | .implements type trait =>
        checkTypeId tables type ++ checkTraitRef registry unit tables trait
    | .associatedTypeEq trait item value =>
        checkTraitRef registry unit tables trait ++
          checkAssociatedItemRef unit trait item ++ checkTypeId tables value
    | .associatedConstEq trait item value =>
        checkTraitRef registry unit tables trait ++
          checkAssociatedItemRef unit trait item ++
          checkConstValue registry unit tables value
    | .lifetimeOutlives longer shorter =>
        checkLifetimeId tables longer ++ checkLifetimeId tables shorter
    | .constEq left right =>
        checkConstValue registry unit tables left ++ checkConstValue registry unit tables right
    | .profile value => checkProfileValue registry unit value ProfileSchema.checkProperty
end

private def checkType (registry : ProfileRegistry) (unit : RawUnit) (tables : Tables) : Ty → Array Diagnostic
  | .tuple elements => elements.foldl (fun ds id => ds ++ checkTypeId tables id) #[]
  | .vector element length => checkTypeId tables element ++ match length with
      | some value => checkConstValue registry unit tables value
      | none => #[]
  | .typeDomain type => checkTypeId tables type
  | .resourceDomain resource arguments =>
      checkName tables resource ++ arguments.toArray.flatten.foldl
        (fun ds type => ds ++ checkTypeId tables type) #[]
  | .nominal name arguments =>
      checkName tables name ++ arguments.foldl (fun ds argument =>
        ds ++ checkGenericArgument registry unit tables argument) #[]
  | .function arguments result _ =>
      arguments.foldl (fun ds id => ds ++ checkTypeId tables id) (checkTypeId tables result)
  | .reference reference => checkReferenceType registry unit tables reference
  | .profile value => checkProfileValue registry unit value ProfileSchema.checkType
  | _ => #[]

private def checkAbilityList (description : String) (abilities : Array Ability)
    (loc : Option LocId := none) : Array Diagnostic :=
  abilities.zipIdx.foldl (init := #[]) fun diagnostics (ability, index) =>
    if abilities.take index |>.contains ability then
      diagnostics.push <| .error "LIR-ABILITY-DUPLICATE"
        s!"{description} lists {repr ability} more than once" loc
    else diagnostics

/-- Intrinsic well-formedness of a core type-table entry. These invariants do
not depend on a use site or semantic profile and therefore belong at the raw
unit boundary rather than in executable preparation. -/
private def checkTypeWellFormed (index : Nat) : Ty → Array Diagnostic
  | .integer (.bits 0) _ => #[.error "LIR-TYPE-WELL-FORMED"
      s!"type table entry {index} has a zero-width fixed integer"]
  | .vector _ (some (.integer length)) =>
      if length < 0 then #[.error "LIR-TYPE-WELL-FORMED"
        s!"type table entry {index} has negative fixed-vector length {length}"]
      else #[]
  | .vector _ (some _) => #[.error "LIR-TYPE-WELL-FORMED"
      s!"type table entry {index} has a non-integer fixed-vector length"]
  | .function _ _ abilities => checkAbilityList s!"function type table entry {index}" abilities
  | _ => #[]

private def binderKinds (binders : Array GenericBinder) : Array BinderKind :=
  binders.map (·.kind)

private def checkGenericBinderWellFormed (tables : Tables)
    (binder : GenericBinder) : Array Diagnostic :=
  let duplicateErrors := checkAbilityList s!"generic binder `{binder.name}`"
    binder.abilities (some binder.loc)
  let abilityErrors := if binder.kind != .typeArg && !binder.abilities.isEmpty then
    duplicateErrors.push <| .at "LIR-GENERIC-ABILITY-KIND"
      s!"generic binder `{binder.name}` has kind {repr binder.kind}; only type binders may carry abilities"
      binder.loc
  else duplicateErrors
  match binder.kind, binder.type with
  | .const, some type => abilityErrors ++ checkTypeUse tables type
  | .const, none => abilityErrors.push <| .at "LIR-GENERIC-CONST-TYPE"
      s!"const generic binder `{binder.name}` has no declared type" binder.loc
  | _, some _ => abilityErrors.push <| .at "LIR-GENERIC-CONST-TYPE"
      s!"non-const generic binder `{binder.name}` carries a const parameter type" binder.loc
  | _, none => abilityErrors

private def checkGenericBinderNames (binders : Array GenericBinder) : Array Diagnostic :=
  binders.zipIdx.foldl (init := #[]) fun diagnostics (binder, index) =>
    let diagnostics := if binder.name.isEmpty then diagnostics.push <| .at
        "LIR-GENERIC-NAME" "generic binder name must not be empty" binder.loc
      else diagnostics
    if !binder.name.isEmpty && (binders.take index).any (·.name == binder.name) then
      diagnostics.push <| .at "LIR-GENERIC-NAME-DUPLICATE"
        s!"generic binder `{binder.name}` occurs more than once in its declaration"
        binder.loc
    else diagnostics

private def checkScopedBinder (binders : Array BinderKind) (expected : BinderKind)
    (index : Nat) (loc : Option LocId := none) : Array Diagnostic :=
  match binders[index]? with
  | none => #[.error "LIR-GENERIC-SCOPE"
      s!"generic parameter {index} is out of scope; declaration has {binders.size} binders" loc]
  | some actual => if actual == expected then #[] else #[.error "LIR-GENERIC-SCOPE"
      s!"generic parameter {index} has kind {repr actual}, expected {repr expected}" loc]

mutual
  private partial def checkScopedTypeFuel (registry : ProfileRegistry) (unit : RawUnit)
      (tables : Tables) (binders : Array BinderKind) (loc : Option LocId)
      (root : TypeId) (fuel : Nat) : Array Diagnostic :=
    match fuel, tables.types[root.index]? with
    | 0, _ | _, none => #[]
    | fuel + 1, some ty => match ty with
      | .typeParameter index => checkScopedBinder binders .typeArg index loc
      | .tuple elements => elements.foldl (fun ds element =>
          ds ++ checkScopedTypeFuel registry unit tables binders loc element fuel) #[]
      | .vector element _ | .typeDomain element =>
          checkScopedTypeFuel registry unit tables binders loc element fuel
      | .resourceDomain _ arguments => arguments.toArray.flatten.foldl
          (fun ds element => ds ++
            checkScopedTypeFuel registry unit tables binders loc element fuel) #[]
      | .nominal _ arguments => arguments.foldl (fun ds argument =>
          ds ++ checkScopedGenericArgument registry unit tables binders loc argument fuel) #[]
      | .function arguments result _ =>
          arguments.foldl (fun ds argument => ds ++
            checkScopedTypeFuel registry unit tables binders loc argument fuel)
            (checkScopedTypeFuel registry unit tables binders loc result fuel)
      | .reference reference =>
          checkScopedTypeFuel registry unit tables binders loc reference.referent fuel ++
            checkScopedLifetime tables binders loc reference.lifetime
      | _ => #[]

  private partial def checkScopedLifetime (tables : Tables) (binders : Array BinderKind)
      (loc : Option LocId) (id : LifetimeId) : Array Diagnostic :=
    match tables.lifetimes[id.index]? with
    | some { kind := .parameter index, .. } =>
        checkScopedBinder binders .lifetime index loc
    | _ => #[]

  private partial def checkScopedGenericArgument (registry : ProfileRegistry)
      (unit : RawUnit) (tables : Tables) (binders : Array BinderKind)
      (loc : Option LocId) (argument : GenericArgument) (fuel : Nat) : Array Diagnostic :=
    match argument with
    | .typeArg value =>
        checkScopedTypeFuel registry unit tables binders (some value.loc) value.typeId fuel
    | .lifetime lifetime => checkScopedLifetime tables binders loc lifetime
    | .const _ | .evidence _ => #[]

  private partial def checkScopedTraitRef (registry : ProfileRegistry) (unit : RawUnit)
      (tables : Tables) (binders : Array BinderKind) (loc : Option LocId)
      (trait : TraitRef) : Array Diagnostic :=
    trait.arguments.foldl (fun ds argument =>
      ds ++ checkScopedGenericArgument registry unit tables binders loc argument
        (tables.types.size + 1)) #[]

  private partial def checkScopedPredicate (registry : ProfileRegistry) (unit : RawUnit)
      (tables : Tables) (binders : Array BinderKind) (loc : Option LocId) :
      GenericPredicate → Array Diagnostic
    | .ability type _ => checkScopedTypeFuel registry unit tables binders loc type
        (tables.types.size + 1)
    | .implements type trait =>
        checkScopedTypeFuel registry unit tables binders loc type (tables.types.size + 1) ++
          checkScopedTraitRef registry unit tables binders loc trait
    | .associatedTypeEq trait _ value =>
        checkScopedTraitRef registry unit tables binders loc trait ++
          checkScopedTypeFuel registry unit tables binders loc value (tables.types.size + 1)
    | .associatedConstEq trait _ _ =>
        checkScopedTraitRef registry unit tables binders loc trait
    | .lifetimeOutlives longer shorter =>
        checkScopedLifetime tables binders loc longer ++
          checkScopedLifetime tables binders loc shorter
    | .constEq .. | .profile _ => #[]
end

private def checkScopedType (registry : ProfileRegistry) (unit : RawUnit)
    (tables : Tables) (binders : Array BinderKind) (loc : Option LocId)
    (root : TypeId) : Array Diagnostic :=
  checkScopedTypeFuel registry unit tables binders loc root (tables.types.size + 1)

private def checkScopedTypeUse (registry : ProfileRegistry) (unit : RawUnit)
    (tables : Tables) (binders : Array BinderKind) (use : TypeUse) : Array Diagnostic :=
  checkScopedType registry unit tables binders (some use.loc) use.typeId

private def checkTables (registry : ProfileRegistry) (unit : RawUnit) (tables : Tables) : Array Diagnostic :=
  let checkRange (range : SourceRange) :=
    let ds := checkIndex "file" range.file.index tables.files.size
    if range.startByte <= range.endByte then ds
    else ds.push <| .error "LIR-LOC-RANGE" "source range starts after it ends"
  let fileErrors := tables.locations.foldl (fun ds loc =>
    let ds := match loc.primary with | some range => ds ++ checkRange range | none => ds
    let ds := loc.related.foldl (fun ds range => ds ++ checkRange range) ds
    let ds := loc.expansion.foldl (fun ds range => ds ++ checkRange range) ds
    match loc.parent with | some parent => ds ++ checkLoc tables parent | none => ds) #[]
  let originErrors := tables.origins.foldl (fun ds origin =>
    ds ++ checkLoc tables origin.location) #[]
  let alignmentErrors := tables.alignments.foldl (fun ds alignment =>
    ds ++ checkIndex "origin" alignment.source.index tables.origins.size) #[]
  let lifetimeErrors := tables.lifetimes.foldl (fun ds lifetime =>
    ds ++ checkLoc tables lifetime.loc) #[]
  let namespaceErrors := tables.namespaces.zipIdx.foldl (init := #[])
      fun ds (namespaceRef, index) =>
    if tables.namespaces.take index |>.contains namespaceRef then
      ds.push <| .error "LIR-NAMESPACE-DUPLICATE"
        s!"duplicate canonical namespace path `{String.intercalate "::" namespaceRef.segments.toList}`"
        ((unit.namespaces[index]?).map (·.loc))
    else ds
  let nameErrors := tables.names.foldl (fun ds name =>
    ds ++ checkIndex "namespace reference" name.namespaceId.index tables.namespaces.size) #[]
  let typeErrors := tables.types.zipIdx.foldl (init := #[]) fun ds (ty, index) =>
    ds ++ checkType registry unit tables ty ++ checkTypeWellFormed index ty
  fileErrors ++ originErrors ++ alignmentErrors ++ lifetimeErrors ++ namespaceErrors ++
    nameErrors ++ typeErrors

private def placeChildren : Place → Array PlaceId
  | .localVar _ => #[]
  | .deref base | .field base .. | .index base _ | .subslice base .. |
      .downcast base _ => #[base]

private def genericArgumentTypeChildren : GenericArgument → Array TypeId
  | .typeArg value => #[value.typeId]
  | _ => #[]

private def typeChildren : Ty → Array TypeId
  | .tuple elements => elements
  | .vector element _ | .typeDomain element => #[element]
  | .resourceDomain _ arguments => arguments.getD #[]
  | .nominal _ arguments => arguments.filterMap fun
      | .typeArg value => some value.typeId
      | _ => none
  | .function arguments result _ => arguments.push result
  | .reference reference => #[reference.referent]
  | _ => #[]

private def patternChildren : PatternKind → Array PatternId
  | .tuple elements => elements
  | .constructor _ _ _ fields => fields
  | _ => #[]

private def checkAcyclic (kind : String) (size : Nat) (children : Nat → Array Nat) : Array Diagnostic :=
  let rec visit (root current : Nat) (path : List Nat) (fuel : Nat) : Array Diagnostic :=
    match fuel with
    | 0 => #[.error "LIR-ARENA-CYCLE" s!"{kind} arena contains a cycle reachable from {root}"]
    | fuel + 1 =>
        if path.contains current then
          #[.error "LIR-ARENA-CYCLE" s!"{kind} arena contains a cycle reachable from {root}"]
        else
          (children current).foldl (fun ds child =>
            if child < size then ds ++ visit root child (current :: path) fuel else ds) #[]
  (Array.range size).foldl (fun ds root => ds ++ visit root root [] (size + 1)) #[]

private def checkOperation {β : Type} (registry : ProfileRegistry) (unit : RawUnit) (ns : Namespace β)
    (loc : LocId) : Operation → Array Diagnostic
  | .move place | .copy place | .read place | .write place | .drop place =>
      checkPlaceId ns place (some loc)
  | .borrow kind place =>
      let base := checkPlaceId ns place (some loc)
      match kind with
      | .profile value => base ++ checkProfileValue registry unit value ProfileSchema.checkOperation
      | _ => base
  | .call kind => match kind with
      | .function callee | .constructor callee _ | .destructor callee _ | .closure callee =>
          checkQualifiedRef unit unit.tables callee (some loc)
      | .invoke => #[]
      | .extension value targets =>
          checkProfileValue registry unit value ProfileSchema.checkOperation ++
            targets.foldl (fun ds target =>
              ds ++ checkQualifiedRef unit unit.tables target (some loc)) #[]
  | .profile value targets =>
      checkProfileValue registry unit value ProfileSchema.checkOperation ++
        targets.foldl (fun ds target =>
          ds ++ checkQualifiedRef unit unit.tables target (some loc)) #[]
  | .primitive (.checkedAdd (.profile value)) |
      .primitive (.checkedSubtract (.profile value)) |
      .primitive (.checkedMultiply (.profile value)) |
      .primitive (.checkedModulo (.profile value)) |
      .primitive (.checkedDivide (.profile value)) |
      .primitive (.checkedShiftLeft (.profile value)) |
      .primitive (.checkedShiftRight (.profile value)) |
      .primitive (.checkedCast (.profile value)) |
      .primitive (.checkedNegate (.profile value)) =>
      checkProfileValue registry unit value ProfileSchema.checkOperation
  | .reference (.borrow (.profile value)) =>
      checkProfileValue registry unit value ProfileSchema.checkOperation
  | .reference (.endLoan _) =>
      #[.error "LIR-SEMANTIC-LOAN-MARKER"
        "endLoan is synthesized by validation and may not appear in frontend input" (some loc)]
  | .data (.select reference _) | .data (.selectVariants reference _) |
      .data (.testVariants reference _) | .data (.discriminant reference) |
      .data (.updateField reference _) =>
      checkQualifiedRef unit unit.tables reference (some loc)
  | .specification (.functionCall reference _) =>
      checkQualifiedRef unit unit.tables reference (some loc)
  | _ => #[]

private partial def checkAttribute (registry : ProfileRegistry) (unit : RawUnit)
    (tables : Tables) : Attribute → Array Diagnostic
  | .call _ arguments loc =>
      checkOptional loc ++ arguments.foldl (fun ds argument =>
        ds ++ checkAttribute registry unit tables argument) #[]
  | .assign _ value loc => checkOptional loc ++ match value with
      | .constant value => checkConstValue registry unit tables value
      | .name (some namespaceId) _ =>
          checkIndex "namespace reference" namespaceId.index tables.namespaces.size loc
      | .name none _ | .qualifiedName _ => #[]
where
  checkOptional : Option LocId → Array Diagnostic
    | some loc => checkLoc tables loc
    | none => #[]

private def checkAttributes (registry : ProfileRegistry) (unit : RawUnit)
    (tables : Tables) (attributes : Array Attribute) : Array Diagnostic :=
  attributes.foldl (fun ds attr =>
    ds ++ checkAttribute registry unit tables attr) #[]

private def checkExprKind {β : Type} (registry : ProfileRegistry) (unit : RawUnit) (ns : Namespace β)
    (expr : Expr) : Array Diagnostic :=
  let childErrors := (expressionChildren expr.kind).foldl
    (fun ds id => ds ++ checkExprId ns id (some expr.loc)) #[]
  let nodeErrors := match expr.kind with
    | .value value _ => checkConstValue registry unit unit.tables value
    | .constant ref => checkQualifiedRef unit unit.tables ref (some expr.loc)
    | .operation op instantiations _ surface =>
        let surfaceErrors := match surface with
          | some (.extension value) =>
              checkProfileValue registry unit value ProfileSchema.checkSurface
          | _ => #[]
        checkOperation registry unit ns expr.loc op ++ surfaceErrors ++
          instantiations.foldl (fun ds argument =>
            ds ++ checkGenericArgument registry unit unit.tables argument) #[]
    | .letDecl pattern _ _ => checkPatternId ns pattern (some expr.loc)
    | .match_ _ arms => arms.foldl (fun ds arm =>
        ds ++ checkPatternId ns arm.pattern (some expr.loc)) #[]
    | .assign place _ => checkPlaceId ns place (some expr.loc)
    | .assignPattern pattern _ => checkPatternId ns pattern (some expr.loc)
    | .quantifier kind binders _ _ _ =>
        let ds := binders.foldl
          (fun ds binder => ds ++ checkPatternId ns binder.pattern (some expr.loc)) #[]
        match kind with
        | .profile value => ds ++ checkProfileValue registry unit value ProfileSchema.checkProperty
        | _ => ds
    | .throw_ (.profile value) _ =>
        checkProfileValue registry unit value ProfileSchema.checkOperation
    | .spec block =>
        let conditionErrors := checkOptional block.sourceLoc ++ block.conditions.foldl (fun ds condition =>
          ds ++ checkLoc unit.tables condition.loc ++
            checkAttributes registry unit unit.tables condition.properties ++
            condition.auxiliary.foldl (fun ds auxiliary =>
              ds ++ checkExprId ns auxiliary.2 (some condition.loc)) #[]) #[]
        let frameErrors := match block.frame with
          | some frame => frame.reads.foldl
              (fun ds ty => ds ++ checkTypeUse unit.tables ty) #[]
          | none => #[]
        conditionErrors ++ frameErrors ++ checkAttributes registry unit unit.tables block.pragmas
    | _ => #[]
  childErrors ++ nodeErrors
where
  checkOptional : Option LocId → Array Diagnostic
    | some loc => checkLoc unit.tables loc
    | none => #[]

private def checkSignature (registry : ProfileRegistry) (unit : RawUnit) (tables : Tables)
    (associatedItemCount : Nat) (signature : Signature)
    (outerBinders : Array BinderKind := #[]) : Array Diagnostic :=
  let scopedBinders := outerBinders ++ binderKinds signature.generics
  let genericErrors := signature.generics.foldl (fun ds binder =>
    let ds := ds ++ checkLoc tables binder.loc ++ checkGenericBinderWellFormed tables binder ++
      (match binder.type with
       | some type => checkScopedTypeUse registry unit tables scopedBinders type
       | none => #[])
    binder.predicates.foldl (fun ds predicate =>
      ds ++ checkGenericPredicate registry unit tables associatedItemCount predicate ++
        checkScopedPredicate registry unit tables scopedBinders (some binder.loc) predicate) ds) #[]
  let parameterErrors := signature.parameters.foldl
    (fun ds parameter => ds ++ checkTypeUse tables parameter.typeUse ++
      checkScopedTypeUse registry unit tables scopedBinders parameter.typeUse) #[]
  let resultErrors := signature.results.foldl
    (fun ds result => ds ++ checkTypeUse tables result ++
      checkScopedTypeUse registry unit tables scopedBinders result) #[]
  let predicateErrors := signature.predicates.foldl (fun ds predicate =>
    ds ++ checkGenericPredicate registry unit tables associatedItemCount predicate ++
      checkScopedPredicate registry unit tables scopedBinders none predicate) #[]
  checkGenericBinderNames signature.generics ++ genericErrors ++ parameterErrors ++
    resultErrors ++ predicateErrors

private def checkOptionalLoc (tables : Tables) (loc : Option LocId) : Array Diagnostic :=
  match loc with | some loc => checkLoc tables loc | none => #[]

private def checkCondition (registry : ProfileRegistry) (unit : RawUnit) (ns : RawNamespace)
    (condition : Condition) : Array Diagnostic :=
  checkLoc unit.tables condition.loc ++
    checkAttributes registry unit unit.tables condition.properties ++
    checkExprId ns condition.expression (some condition.loc) ++
    condition.auxiliary.foldl (fun ds auxiliary =>
      ds ++ checkExprId ns auxiliary.2 (some condition.loc)) #[]

private def checkContract (registry : ProfileRegistry) (unit : RawUnit) (ns : RawNamespace)
    (contract : FunctionContract) (binders : Array BinderKind := #[]) : Array Diagnostic :=
  checkOptionalLoc unit.tables contract.loc ++ contract.conditions.foldl
      (fun ds condition => ds ++ checkCondition registry unit ns condition) #[] ++
    contract.modifies.foldl (fun ds expr => ds ++ checkExprId ns expr) #[] ++
    contract.reads.foldl (fun ds ty => ds ++ checkTypeUse unit.tables ty ++
      checkScopedTypeUse registry unit unit.tables binders ty) #[] ++
    checkAttributes registry unit unit.tables contract.pragmas

private def checkGenericBinders (registry : ProfileRegistry) (unit : RawUnit) (tables : Tables)
    (associatedItemCount : Nat) (binders : Array GenericBinder) : Array Diagnostic :=
  let scopedBinders := binderKinds binders
  checkGenericBinderNames binders ++ binders.foldl (fun ds binder =>
    let ds := ds ++ checkLoc tables binder.loc ++ checkGenericBinderWellFormed tables binder ++
      (match binder.type with
       | some type => checkScopedTypeUse registry unit tables scopedBinders type
       | none => #[])
    binder.predicates.foldl (fun ds predicate =>
      ds ++ checkGenericPredicate registry unit tables associatedItemCount predicate ++
        checkScopedPredicate registry unit tables scopedBinders (some binder.loc) predicate)
      ds) #[]

private def checkField (tables : Tables) (field : FieldDecl) : Array Diagnostic :=
  checkLoc tables field.loc ++ checkName tables field.name (some field.loc) ++
    checkTypeUse tables field.type

private def checkPredicates (registry : ProfileRegistry) (unit : RawUnit)
    (ns : RawNamespace) (loc : LocId) (binders : Array BinderKind)
    (predicates : Array GenericPredicate) : Array Diagnostic :=
  predicates.foldl (fun ds predicate =>
    ds ++ checkGenericPredicate registry unit unit.tables ns.associatedItems.size predicate ++
      checkScopedPredicate registry unit unit.tables binders (some loc) predicate) #[]

private def checkAssociatedItemKind (registry : ProfileRegistry) (unit : RawUnit)
    (ns : RawNamespace) (loc : LocId) (outerBinders : Array BinderKind) :
    AssociatedItemKind → Array Diagnostic
  | .type bounds default =>
      checkPredicates registry unit ns loc outerBinders bounds ++
        (match default with
         | some value => checkTypeUse unit.tables value ++
             checkScopedTypeUse registry unit unit.tables outerBinders value
         | none => #[])
  | .constant type default =>
      checkTypeUse unit.tables type ++
        checkScopedTypeUse registry unit unit.tables outerBinders type ++
        (match default with | some value => checkExprId ns value (some loc) | none => #[])
  | .method signature defaultImplementation =>
      checkSignature registry unit unit.tables ns.associatedItems.size signature outerBinders ++
        (match defaultImplementation with
         | some value => checkQualifiedRef unit unit.tables value (some loc)
         | none => #[])

private def checkAssociatedItemValue (unit : RawUnit) (ns : RawNamespace)
    (loc : LocId) (registry : ProfileRegistry) (binders : Array BinderKind) :
    AssociatedItemValue → Array Diagnostic
  | .type value => checkTypeUse unit.tables value ++
      checkScopedTypeUse registry unit unit.tables binders value
  | .constant value => checkExprId ns value (some loc)
  | .method value => checkQualifiedRef unit unit.tables value (some loc)

private def associatedBindingKindMatches (kind : AssociatedItemKind)
    (value : AssociatedItemValue) : Bool :=
  match kind, value with
  | .type .., .type _ | .constant .., .constant _ | .method .., .method _ => true
  | _, _ => false

/-- Associated types occupy a separate name namespace from associated
constants and methods. Constants and methods share the value namespace. -/
private def associatedItemKindsShareNamespace
    (left right : AssociatedItemKind) : Bool :=
  match left, right with
  | .type .., .type .. => true
  | .type .., _ | _, .type .. => false
  | _, _ => true

private def checkLocals (tables : Tables) (locals : Array LocalDecl) : Array Diagnostic :=
  locals.zipIdx.foldl (init := #[]) fun ds (localDecl, index) =>
    ds ++ checkLoc tables localDecl.loc ++ checkTypeUse tables localDecl.type ++
      (if localDecl.id.index == index then #[] else
        #[.at "LIR-LOCAL-IDENTITY"
          s!"local id {localDecl.id.index} does not match table position {index}" localDecl.loc])

private def checkParametersMatchLocals (parameters : Array Parameter)
    (locals : Array LocalDecl) : Array Diagnostic :=
  let countErrors := if parameters.size <= locals.size then #[] else
    #[.error "LIR-PARAMETER-LOCAL"
      s!"function has {parameters.size} parameters but only {locals.size} local declarations"]
  parameters.zipIdx.foldl (init := countErrors) fun ds (parameter, index) =>
    match locals[index]? with
    | none => ds
    | some localDecl =>
        let ds := if parameter.typeUse.typeId == localDecl.type.typeId then ds else
            ds.push <| .at "LIR-PARAMETER-LOCAL"
              s!"parameter {index} type does not match leading local declaration" parameter.typeUse.loc
        if parameter.mutable == localDecl.mutable then ds else
          ds.push <| .at "LIR-PARAMETER-LOCAL"
            s!"parameter {index} mutability does not match leading local declaration"
            parameter.typeUse.loc

private def checkRawBody (registry : ProfileRegistry) (unit : RawUnit) (ns : RawNamespace)
    (localCount : Nat) (loc : LocId) : RawBody → Array Diagnostic
  | .absent => #[]
  | .structured root => checkExprId ns root (some loc)
  | .cfg graph =>
      let entryErrors := checkIndex "basic block" graph.entry.index graph.blocks.size (some loc)
      let blockErrors := graph.blocks.foldl (fun ds block =>
        let ds := ds ++ checkLoc unit.tables block.loc
        let checkBlock (id : BlockId) := checkIndex "basic block" id.index graph.blocks.size (some block.loc)
        let checkUnwind : RawUnwindAction → Array Diagnostic
          | .cleanup target => checkBlock target
          | .continue_ | .unreachable | .terminate _ => #[]
        let checkStatement : RawStatement → Array Diagnostic
          | .execute expression => checkExprId ns expression (some block.loc)
          | .storageLive localId | .storageDead localId =>
              checkIndex "local" localId.index localCount (some block.loc)
          | .deinit place | .retag place | .placeMention place =>
              checkPlaceId ns place (some block.loc)
          | .setDiscriminant place variant =>
              checkPlaceId ns place (some block.loc) ++
                checkName unit.tables variant (some block.loc)
          | .ascribeUserType place type =>
              checkPlaceId ns place (some block.loc) ++ checkTypeUse unit.tables type
          | .profile value => checkProfileValue registry unit value ProfileSchema.checkOperation
        let ds := block.statements.foldl (fun ds statement => ds ++ checkStatement statement) ds
        ds ++ match block.terminator with
          | .goto target => checkBlock target
          | .branch condition thenTarget elseTarget =>
              checkExprId ns condition (some block.loc) ++ checkBlock thenTarget ++ checkBlock elseTarget
          | .switch scrutinee cases defaultTarget =>
              checkExprId ns scrutinee (some block.loc) ++
                cases.foldl (fun ds case => ds ++ checkBlock case.2) (checkBlock defaultTarget)
          | .call call destination unwind =>
              let destinationErrors := match destination with
                | none => #[]
                | some destination =>
                    checkPlaceId ns destination.place (some block.loc) ++
                      checkBlock destination.target
              checkExprId ns call (some block.loc) ++ destinationErrors ++ checkUnwind unwind
          | .drop place target unwind =>
              checkPlaceId ns place (some block.loc) ++ checkBlock target ++ checkUnwind unwind
          | .assert condition _ kind target unwind =>
              let kindErrors := match kind with
                | .profile value =>
                    checkProfileValue registry unit value ProfileSchema.checkOperation
                | _ => #[]
              checkExprId ns condition (some block.loc) ++ kindErrors ++
                checkBlock target ++ checkUnwind unwind
          | .return_ values => values.foldl
              (fun ds value => ds ++ checkExprId ns value (some block.loc)) #[]
          | .throw_ _ arguments => arguments.foldl
              (fun ds argument => ds ++ checkExprId ns argument (some block.loc)) #[]
          | .unreachable | .resume | .abort => #[]) #[]
      entryErrors ++ blockErrors

private def duplicateIntrinsicDiagnostic (code message : String)
    (firstLoc duplicateLoc : LocId) : Diagnostic :=
  { code
    message
    primary := some duplicateLoc
    related := #[{ loc := firstLoc, message := "first occurrence is here" }] }

private def checkIntrinsicBindingList (unit : RawUnit) (ns : RawNamespace)
    (kind : String) (targets : Array NameId)
    (bindings : Array IntrinsicBinding) : Array Diagnostic :=
  bindings.zipIdx.foldl (init := #[]) fun ds (binding, index) =>
    let ds := ds ++ checkLoc unit.tables binding.loc ++
      checkQualifiedRef unit unit.tables binding.target (some binding.loc)
    let ds := match bindings.take index |>.find? (fun previous => previous.role == binding.role) with
      | some previous => ds.push <| duplicateIntrinsicDiagnostic
          "LIR-INTRINSIC-ROLE-DUPLICATE"
          s!"duplicate {kind} intrinsic role `{binding.role}`" previous.loc binding.loc
      | none => ds
    match unit.tables.names[binding.target.name.index]? with
    | some targetName =>
        if targetName.namespaceId != binding.target.namespaceId then ds
        else if binding.target.namespaceId != ns.identity then
          ds.push <| .at "LIR-INTRINSIC-TARGET-NAMESPACE"
            s!"{kind} intrinsic target `{targetName.name}` is outside the owner's namespace"
            binding.loc
        else
          if targets.contains binding.target.name then ds else ds.push <|
            .at "LIR-INTRINSIC-TARGET-KIND"
            s!"{kind} intrinsic target `{targetName.name}` does not resolve to a {kind} function"
            binding.loc
    | none => ds

/-- Validate the profile-independent shape of intrinsic role graphs. Model
vocabulary, role dependencies, and exact signatures remain profile-owned,
but every profile shares these declaration-identity invariants. -/
private def checkIntrinsics (registry : ProfileRegistry) (unit : RawUnit)
    (ns : RawNamespace) : Array Diagnostic :=
  let declarationErrors := ns.intrinsics.zipIdx.foldl (init := #[])
      fun ds (intrinsic, index) =>
    let ds := ds ++ checkLoc unit.tables intrinsic.loc ++
      checkName unit.tables intrinsic.owner (some intrinsic.loc) ++
      checkProfile unit intrinsic.profile (some intrinsic.loc) ++
      checkIntrinsicBindingList unit ns "executable" (ns.functions.map (·.name))
        intrinsic.executableBindings ++
      checkIntrinsicBindingList unit ns "specification" (ns.specFunctions.map (·.name))
        intrinsic.specBindings
    let ds := match ns.profile with
      | some profile => if profile == intrinsic.profile then ds else ds.push <|
          .at "LIR-PROFILE-MISMATCH"
            "intrinsic profile differs from its namespace profile" intrinsic.loc
      | none => ds
    let ds := match profileSchema? registry intrinsic.profile,
        profileConfig? unit.profiles intrinsic.profile with
      | some schema, some config =>
          if config.name == schema.name && config.version == schema.version then
            ds ++ schema.checkIntrinsic unit ns intrinsic
          else ds
      | _, _ => ds
    let ds := match unit.tables.names[intrinsic.owner.index]? with
      | some ownerName =>
          if ownerName.namespaceId != ns.identity ||
              !ns.structs.any (fun declaration => declaration.name == intrinsic.owner) then
            ds.push <| .at "LIR-INTRINSIC-OWNER"
              s!"intrinsic owner `{ownerName.name}` does not resolve to a struct or enum in this namespace"
              intrinsic.loc
          else ds
      | none => ds
    match ns.intrinsics.take index |>.find? (fun previous => previous.owner == intrinsic.owner) with
    | some previous => ds.push <| duplicateIntrinsicDiagnostic
        "LIR-INTRINSIC-OWNER-DUPLICATE"
        "a struct or enum has more than one intrinsic declaration" previous.loc intrinsic.loc
    | none => ds
  let bindings := ns.intrinsics.foldl (fun all intrinsic =>
    all ++ intrinsic.executableBindings ++ intrinsic.specBindings) #[]
  let sharedTargetErrors := bindings.zipIdx.foldl (init := #[]) fun ds (binding, index) =>
    match bindings.take index |>.find? (fun previous => previous.target == binding.target) with
    | some previous => ds.push <| duplicateIntrinsicDiagnostic
        "LIR-INTRINSIC-TARGET-SHARED"
        "an intrinsic target is assigned to more than one role" previous.loc binding.loc
    | none => ds
  declarationErrors ++ sharedTargetErrors

/-- Reject cycles among traits owned by this unit. Dependency interfaces do
not yet carry trait declarations, so external supertraits remain leaves. -/
private def checkTraitInheritanceAcyclic (ns : RawNamespace) : Array Diagnostic :=
  let children (index : Nat) : Array Nat :=
    match ns.traits[index]? with
    | none => #[]
    | some trait => trait.superTraits.filterMap fun parent =>
        if parent.trait.namespaceId != ns.identity then none
        else ns.traits.findIdx? (·.name == parent.trait.name)
  let rec visit (root current : Nat) (path : List Nat) (fuel : Nat) : Array Diagnostic :=
    match fuel with
    | 0 => match ns.traits[root]? with
        | some trait => #[.at "LIR-TRAIT-CYCLE"
            "trait inheritance contains a cycle" trait.loc]
        | none => #[]
    | fuel + 1 =>
        if path.contains current then match ns.traits[root]? with
          | some trait => #[.at "LIR-TRAIT-CYCLE"
              "trait inheritance contains a cycle" trait.loc]
          | none => #[]
        else (children current).foldl (fun ds child =>
          ds ++ visit root child (current :: path) fuel) #[]
  (Array.range ns.traits.size).foldl (fun ds root =>
    ds ++ visit root root [] (ns.traits.size + 1)) #[]

private def checkNamespace (registry : ProfileRegistry) (unit : RawUnit)
    (expectedId : Nat) (ns : RawNamespace) : Array Diagnostic :=
  let importErrors := ns.imports.zipIdx.foldl (init := #[]) fun ds (id, index) =>
    let ds := ds ++ checkNamespaceId unit id (some ns.loc)
    let ds := if id == ns.identity then
      ds.push <| .at "LIR-IMPORT-SELF" "namespace imports itself" ns.loc
    else ds
    let ds := if ns.imports.take index |>.contains id then
      ds.push <| .at "LIR-IMPORT-DUPLICATE"
        s!"namespace {id.index} is imported more than once" ns.loc
    else ds
    if unit.namespaces.size <= id.index &&
        !unit.dependencies.any (·.namespaceId == id) then
      ds.push <| .at "LIR-IMPORT-INTERFACE"
        s!"external namespace {id.index} has no dependency interface" ns.loc
    else ds
  let headerErrors :=
    checkLoc unit.tables ns.loc ++
    (if ns.identity.index == expectedId then #[] else
      #[.at "LIR-NAMESPACE-IDENTITY"
        s!"namespace identity {ns.identity.index} does not match unit position {expectedId}" ns.loc]) ++
    (match ns.profile with | some profile => checkProfile unit profile (some ns.loc) | none => #[]) ++
    importErrors
  let declarationNameErrors :=
    checkDeclarationNames unit.tables ns.identity "function"
      (ns.functions.map fun declaration => (declaration.name, declaration.loc)) ++
    checkDeclarationNames unit.tables ns.identity "constant"
      (ns.constants.map fun declaration => (declaration.name, declaration.loc)) ++
    checkDeclarationNames unit.tables ns.identity "struct or enum"
      (ns.structs.map fun declaration => (declaration.name, declaration.loc)) ++
    checkDeclarationNames unit.tables ns.identity "trait"
      (ns.traits.map fun declaration => (declaration.name, declaration.loc)) ++
    checkDeclarationNames unit.tables ns.identity "specification function"
      (ns.specFunctions.map fun declaration => (declaration.name, declaration.loc)) ++
    checkDeclarationNames unit.tables ns.identity "specification variable"
      (ns.specVars.map fun declaration => (declaration.name, declaration.loc))
  let exprErrors := ns.expressions.foldl (fun ds expr =>
    ds ++ checkLoc unit.tables expr.loc ++ checkTypeId unit.tables expr.typeId (some expr.loc) ++
      checkExprKind registry unit ns expr) #[]
  let patternErrors := ns.patterns.foldl (fun ds pattern =>
    let ds := ds ++ checkLoc unit.tables pattern.loc ++ checkTypeId unit.tables pattern.typeId (some pattern.loc)
    let ds := (patternChildren pattern.kind).foldl
      (fun ds id => ds ++ checkPatternId ns id (some pattern.loc)) ds
    match pattern.kind with
    | .constructor name instantiations _ _ =>
        instantiations.foldl (fun ds argument =>
          ds ++ checkGenericArgument registry unit unit.tables argument)
          (ds ++ checkName unit.tables name (some pattern.loc))
    | _ => ds) #[]
  let placeErrors := ns.places.foldl (fun ds place =>
    let ds := (placeChildren place).foldl (fun ds id => ds ++ checkPlaceId ns id) ds
    match place with
    | .field _ _ name | .downcast _ name => ds ++ checkName unit.tables name
    | .index _ expr => ds ++ checkExprId ns expr
    | .subslice _ start stop false => if start <= stop then ds else
        ds.push <| .error "LIR-PLACE-SUBSLICE"
          s!"subslice start {start} exceeds its end {stop}"
    | _ => ds) #[]
  let arenaErrors :=
    checkAcyclic "expression" ns.expressions.size fun index =>
      (ns.expressions[index]?).map (expressionChildren ·.kind |>.map (·.index)) |>.getD #[] ++
    checkAcyclic "pattern" ns.patterns.size fun index =>
      (ns.patterns[index]?).map (patternChildren ·.kind |>.map (·.index)) |>.getD #[] ++
    checkAcyclic "place" ns.places.size fun index =>
      (ns.places[index]?).map (placeChildren · |>.map (·.index)) |>.getD #[]
  let declErrors := ns.functions.foldl (fun ds function =>
    let functionBinders := binderKinds function.signature.generics
    ds ++ checkLoc unit.tables function.loc ++ checkName unit.tables function.name (some function.loc) ++
      checkProfile unit function.profile (some function.loc) ++
      checkSignature registry unit unit.tables ns.associatedItems.size function.signature ++
      checkIndex "origin" function.origin.index unit.tables.origins.size (some function.loc) ++
      checkIndex "alignment" function.alignment.index unit.tables.alignments.size (some function.loc) ++
      checkContract registry unit ns function.contract functionBinders ++
      checkLocals unit.tables function.locals ++
      checkParametersMatchLocals function.signature.parameters function.locals ++
      checkAttributes registry unit unit.tables function.pragmas ++
      function.profileData.foldl (fun ds value =>
        ds ++ checkProfileValue registry unit value ProfileSchema.checkProperty) #[] ++
      checkAttributes registry unit unit.tables function.attributes ++
      checkRawBody registry unit ns function.locals.size function.loc function.body ++
      (match ns.profile with
       | some profile => if profile == function.profile then #[] else
           #[.at "LIR-PROFILE-MISMATCH" "function profile differs from its namespace profile" function.loc]
       | none => #[])) #[]
  let constantErrors := ns.constants.foldl (fun ds constant =>
    ds ++ checkLoc unit.tables constant.loc ++ checkName unit.tables constant.name (some constant.loc) ++
      checkTypeUse unit.tables constant.type ++ checkExprId ns constant.value (some constant.loc) ++
      constant.profileData.foldl (fun ds value =>
        ds ++ checkProfileValue registry unit value ProfileSchema.checkProperty) #[] ++
      checkAttributes registry unit unit.tables constant.attributes) #[]
  let structErrors := ns.structs.foldl (fun ds struct =>
    let structBinders := binderKinds struct.generics
    let ds := ds ++ checkLoc unit.tables struct.loc ++ checkName unit.tables struct.name (some struct.loc) ++
      checkGenericBinders registry unit unit.tables ns.associatedItems.size struct.generics ++
      checkAbilityList "nominal declaration" struct.abilities (some struct.loc) ++
      checkDeclarationNames unit.tables ns.identity "struct field"
        (struct.fields.map fun field => (field.name, field.loc)) ++
      struct.fields.foldl (fun ds field => ds ++ checkField unit.tables field ++
        checkScopedTypeUse registry unit unit.tables structBinders field.type) #[] ++
      struct.properties.foldl (fun ds property =>
        ds ++ checkProfileValue registry unit property ProfileSchema.checkProperty) #[] ++
      checkLocals unit.tables struct.locals ++
      checkContract registry unit ns struct.contract structBinders ++
      checkAttributes registry unit unit.tables struct.attributes
    let ds := if !struct.fields.isEmpty && !struct.variants.isEmpty then
        ds.push <| .at "LIR-NOMINAL-SHAPE"
          "a nominal declaration cannot contain both struct fields and enum variants" struct.loc
      else ds
    let ds := ds ++ checkDeclarationNames unit.tables ns.identity "enum variant"
      (struct.variants.map fun variant => (variant.name, variant.loc))
    struct.variants.foldl (fun ds variant =>
      ds ++ checkLoc unit.tables variant.loc ++ checkName unit.tables variant.name (some variant.loc) ++
        checkDeclarationNames unit.tables ns.identity "variant field"
          (variant.fields.map fun field => (field.name, field.loc)) ++
        variant.fields.foldl (fun ds field => ds ++ checkField unit.tables field ++
          checkScopedTypeUse registry unit unit.tables structBinders field.type) #[]) ds) #[]
  let associatedItemErrors := ns.associatedItems.zipIdx.foldl (init := #[])
      fun ds (item, index) =>
    let ownerBinders := (ns.traits[item.owner.index]?).map
      (binderKinds ·.generics) |>.getD #[]
    let membershipErrors := match ns.traits[item.owner.index]? with
      | some owner => if owner.associatedItems.contains item.id then #[] else
          #[.at "LIR-ASSOCIATED-ITEM-MEMBERSHIP"
            s!"associated item {item.id.index} is not listed by its owner trait {item.owner.index}"
            item.loc]
      | none => #[]
    ds ++ checkLoc unit.tables item.loc ++ checkName unit.tables item.name (some item.loc) ++
      (if item.id.index == index then #[] else
        #[.at "LIR-ASSOCIATED-ITEM-IDENTITY"
          s!"associated item id {item.id.index} does not match table position {index}" item.loc]) ++
      checkIndex "trait" item.owner.index ns.traits.size (some item.loc) ++
      membershipErrors ++
      checkAssociatedItemKind registry unit ns item.loc ownerBinders item.kind ++
      checkAttributes registry unit unit.tables item.attributes
  let traitErrors := ns.traits.zipIdx.foldl (init := #[]) fun ds (trait, index) =>
    let traitBinders := binderKinds trait.generics
    let ds := ds ++ checkLoc unit.tables trait.loc ++ checkName unit.tables trait.name (some trait.loc) ++
      (if trait.id.index == index then #[] else
        #[.at "LIR-TRAIT-IDENTITY"
          s!"trait id {trait.id.index} does not match table position {index}" trait.loc]) ++
      checkGenericBinders registry unit unit.tables ns.associatedItems.size trait.generics ++
      trait.superTraits.foldl (fun ds parent =>
        ds ++ checkTraitRef registry unit unit.tables parent ++
          checkScopedTraitRef registry unit unit.tables traitBinders (some trait.loc) parent) #[] ++
      checkPredicates registry unit ns trait.loc traitBinders trait.predicates ++
      checkAttributes registry unit unit.tables trait.attributes
    trait.associatedItems.zipIdx.foldl (fun ds (itemId, itemIndex) =>
      let ds := ds ++ checkIndex "associated item" itemId.index ns.associatedItems.size (some trait.loc)
      let ds := if trait.associatedItems.take itemIndex |>.contains itemId then
          ds.push <| .at "LIR-ASSOCIATED-ITEM-DUPLICATE"
            s!"trait lists associated item {itemId.index} more than once" trait.loc
        else ds
      let ds := match ns.associatedItems[itemId.index]? with
        | some item =>
            let duplicateName := trait.associatedItems.take itemIndex |>.any fun previousId =>
              match ns.associatedItems[previousId.index]? with
              | some previous =>
                  associatedItemKindsShareNamespace previous.kind item.kind &&
                    unit.tables.names[previous.name.index]? ==
                      unit.tables.names[item.name.index]?
              | none => false
            if duplicateName then ds.push <| .at "LIR-ASSOCIATED-ITEM-NAME-DUPLICATE"
              "trait has more than one associated item with the same name" item.loc
            else ds
        | none => ds
      match ns.associatedItems[itemId.index]? with
      | some item => if item.owner.index == index then ds else ds.push <|
          .at "LIR-ASSOCIATED-ITEM-OWNER"
            s!"associated item {itemId.index} names trait {item.owner.index}, expected {index}" trait.loc
      | none => ds) ds
  let implementationErrors := ns.implementations.zipIdx.foldl (init := #[])
      fun ds (implementation, index) =>
    let implementationBinders := binderKinds implementation.generics
    let implementedTrait : Option (RawNamespace × TraitDecl) := do
      let targetNs ← unit.namespaces.find? fun candidate =>
        candidate.identity == implementation.trait.trait.namespaceId
      let trait ← targetNs.traits.find? fun candidate =>
        candidate.name == implementation.trait.trait.name
      some (targetNs, trait)
    let completenessErrors := match implementedTrait with
      | none => #[]
      | some (targetNs, trait) => trait.associatedItems.foldl (fun errors itemId =>
          match targetNs.associatedItems[itemId.index]? with
          | some item =>
              let required := match item.kind with
                | .type _ none | .constant _ none | .method _ none => true
                | _ => false
              if required && !implementation.bindings.any (·.item == itemId) then
                errors.push <| .at "LIR-ASSOCIATED-BINDING-MISSING"
                  s!"implementation does not bind required associated item {itemId.index}"
                  implementation.loc
              else errors
          | none => errors) #[]
    let ds := ds ++ checkLoc unit.tables implementation.loc ++
      (if implementation.id.index == index then #[] else
        #[.at "LIR-IMPL-IDENTITY"
          s!"implementation id {implementation.id.index} does not match table position {index}"
          implementation.loc]) ++
      checkGenericBinders registry unit unit.tables ns.associatedItems.size implementation.generics ++
      checkTraitRef registry unit unit.tables implementation.trait ++
      checkScopedTraitRef registry unit unit.tables implementationBinders
        (some implementation.loc) implementation.trait ++
      checkTypeUse unit.tables implementation.target ++
      checkScopedTypeUse registry unit unit.tables implementationBinders implementation.target ++
      checkPredicates registry unit ns implementation.loc implementationBinders
        implementation.predicates ++
      checkAttributes registry unit unit.tables implementation.attributes ++
      completenessErrors
    implementation.bindings.zipIdx.foldl (init := ds) fun ds (binding, bindingIndex) =>
      let ds := ds ++ checkLoc unit.tables binding.loc ++
        (match implementedTrait with
          | some (targetNs, _) => checkIndex "associated item" binding.item.index
              targetNs.associatedItems.size (some binding.loc)
          | none => #[]) ++
        checkAssociatedItemValue unit ns binding.loc registry implementationBinders binding.value
      let ds := if implementation.bindings.take bindingIndex |>.any (·.item == binding.item) then
          ds.push <| .at "LIR-ASSOCIATED-BINDING-DUPLICATE"
            s!"associated item {binding.item.index} is bound more than once" binding.loc
        else ds
      match implementedTrait with
      | some (targetNs, trait) => match targetNs.associatedItems[binding.item.index]? with
        | some item =>
          let ds := if item.owner == trait.id then ds else ds.push <|
                .at "LIR-ASSOCIATED-BINDING-OWNER"
                  s!"binding item {binding.item.index} belongs to trait {item.owner.index}, not implemented trait {trait.id.index}"
                  binding.loc
          if associatedBindingKindMatches item.kind binding.value then ds else ds.push <|
            .at "LIR-ASSOCIATED-BINDING-KIND"
              s!"binding for associated item {binding.item.index} has the wrong kind" binding.loc
        | none => ds
      | none => ds
  let specFunctionErrors := ns.specFunctions.foldl (fun ds function =>
    let functionBinders := binderKinds function.signature.generics
    let ds := ds ++ checkLoc unit.tables function.loc ++ checkName unit.tables function.name (some function.loc) ++
      checkProfile unit function.profile (some function.loc) ++
      checkSignature registry unit unit.tables ns.associatedItems.size function.signature ++
      checkIndex "origin" function.origin.index unit.tables.origins.size (some function.loc) ++
      checkLocals unit.tables function.locals ++
      checkParametersMatchLocals function.signature.parameters function.locals ++
      checkContract registry unit ns function.contract functionBinders ++
      function.profileData.foldl (fun ds value =>
        ds ++ checkProfileValue registry unit value ProfileSchema.checkProperty) #[] ++
      (match ns.profile with
       | some profile => if profile == function.profile then #[] else
           #[.at "LIR-PROFILE-MISMATCH"
             "specification function profile differs from its namespace profile" function.loc]
       | none => #[])
    match function.body with
    | some body => ds ++ checkExprId ns body (some function.loc)
    | none => ds) #[]
  let specVarErrors := ns.specVars.foldl (fun ds specVar =>
    let specVarBinders := binderKinds specVar.generics
    ds ++ checkLoc unit.tables specVar.loc ++ checkName unit.tables specVar.name (some specVar.loc) ++
      checkGenericBinders registry unit unit.tables ns.associatedItems.size specVar.generics ++
      checkTypeUse unit.tables specVar.type ++
      checkScopedTypeUse registry unit unit.tables specVarBinders specVar.type ++
      checkProfile unit specVar.profile (some specVar.loc) ++
      (match specVar.init with | some init => checkExprId ns init (some specVar.loc) | none => #[]) ++
      checkLocals unit.tables specVar.locals ++
      specVar.profileData.foldl (fun ds value =>
        ds ++ checkProfileValue registry unit value ProfileSchema.checkProperty) #[] ++
      (match ns.profile with
       | some profile => if profile == specVar.profile then #[] else
           #[.at "LIR-PROFILE-MISMATCH"
             "specification variable profile differs from its namespace profile" specVar.loc]
       | none => #[])) #[]
  let invariantErrors := ns.invariants.foldl (fun ds invariant =>
    ds ++ checkLoc unit.tables invariant.loc ++ checkCondition registry unit ns invariant.condition ++
      checkLocals unit.tables invariant.locals) #[]
  let intrinsicErrors := checkIntrinsics registry unit ns
  let traitCycleErrors := checkTraitInheritanceAcyclic ns
  let metadataErrors := ns.profileMetadata.foldl (fun ds value =>
    ds ++ checkProfileValue registry unit value ProfileSchema.checkProperty) #[]
  let surfaceErrors := checkAttributes registry unit unit.tables ns.attributes ++
    checkAttributes registry unit unit.tables ns.pragmas ++
    ns.comments.foldl (fun ds comment => ds ++ checkLoc unit.tables comment.loc) #[]
  headerErrors ++ declarationNameErrors ++ exprErrors ++ patternErrors ++ placeErrors ++ arenaErrors ++
    declErrors ++ constantErrors ++ structErrors ++ associatedItemErrors ++ traitErrors ++
    implementationErrors ++ specFunctionErrors ++ specVarErrors ++ invariantErrors ++
    intrinsicErrors ++ traitCycleErrors ++ metadataErrors ++ surfaceErrors

/-- Reuse an authored place node when possible and otherwise append it without
disturbing any producer-authored place identity. -/
private def internNormalizedPlace (places : Array Place) (place : Place) : PlaceId × Array Place :=
  match places.findIdx? (· == place) with
  | some index => (⟨index⟩, places)
  | none => (⟨places.size⟩, places.push place)

/-- Recover the storage path represented by the value-shaped trees emitted by
legacy frontends. In particular, Move's `borrow_field` arrives as
`borrow(select(referenceLocal))`; the reference-local base denotes a
dereferenced place before its field projection. -/
private partial def normalizedExpressionPlace? (tables : Tables) (ns : RawNamespace)
    (id : ExprId) (places : Array Place) (fuel : Nat := 0) : Option (PlaceId × Array Place) := do
  let fuel := if fuel == 0 then ns.expressions.size + 1 else fuel
  if fuel == 0 then none else
    let expression ← ns.expressions[id.index]?
    match expression.kind with
    | .localVar localId => some (internNormalizedPlace places (.localVar localId))
    | .operation (.reference .dereference) instantiations arguments _ => do
        if !instantiations.isEmpty then none else
        let [baseExpression] := arguments.toList | none
        let (base, places) ←
          normalizedExpressionPlace? tables ns baseExpression places (fuel - 1)
        -- Dereferencing a reference-typed field select cancels: the select's
        -- normalized place already denotes the referent's storage path.
        let selectReference := (ns.expressions[baseExpression.index]?).any fun node =>
          (node.kind matches .operation (.data _) _ _ _) &&
            ((tables.types[node.typeId.index]?).any fun ty => ty matches .reference _)
        if selectReference then some (base, places)
        else some (internNormalizedPlace places (.deref base))
    | .operation (.data (.select reference field)) _ arguments _ => do
        let [baseExpression] := arguments.toList | none
        let (base, places) ← normalizedExpressionPlace? tables ns baseExpression places (fuel - 1)
        let baseNode ← ns.expressions[baseExpression.index]?
        let (base, places) := match tables.types[baseNode.typeId.index]? with
          | some (.reference _) => internNormalizedPlace places (.deref base)
          | _ => (base, places)
        let fieldIndex ← tables.names.findIdx? fun name =>
          name.namespaceId == reference.namespaceId && name.name == field
        some (internNormalizedPlace places (.field base reference ⟨fieldIndex⟩))
    | .operation (.data (.selectVariants reference fields)) _ arguments _ => do
        let field ← fields[0]?
        if !fields.all (· == field) then none else
        let [baseExpression] := arguments.toList | none
        let (base, places) ← normalizedExpressionPlace? tables ns baseExpression places (fuel - 1)
        let baseNode ← ns.expressions[baseExpression.index]?
        let (base, places) := match tables.types[baseNode.typeId.index]? with
          | some (.reference _) => internNormalizedPlace places (.deref base)
          | _ => (base, places)
        let fieldIndex ← tables.names.findIdx? fun name =>
          name.namespaceId == reference.namespaceId && name.name == field
        some (internNormalizedPlace places (.field base reference ⟨fieldIndex⟩))
    | .operation (.primitive .index) _ arguments _ => do
        let [baseExpression, index] := arguments.toList | none
        let (base, places) ← normalizedExpressionPlace? tables ns baseExpression places (fuel - 1)
        let baseNode ← ns.expressions[baseExpression.index]?
        let (base, places) := match tables.types[baseNode.typeId.index]? with
          | some (.reference _) => internNormalizedPlace places (.deref base)
          | _ => (base, places)
        some (internNormalizedPlace places (.index base index))
    | _ => none

/-- Rewrite lossless value-level borrows of recoverable storage paths to the
stronger place-based core operation. Other computed-value borrows stay
explicit and receive the existing unsupported-capability diagnostic during
preparation. -/
private def normalizeValueBorrows (tables : Tables) (ns : RawNamespace) : RawNamespace :=
  let (expressions, places) := ns.expressions.foldl (init := (#[], ns.places))
      fun (expressions, places) expression =>
    match expression.kind with
    | .operation (.reference (.borrow kind)) instantiations arguments surface =>
        match instantiations.isEmpty, arguments.toList with
        | true, [argument] => match normalizedExpressionPlace? tables ns argument places with
            | some (place, places) =>
                let (place, places) :=
                  match ns.expressions[argument.index]?, tables.types[expression.typeId.index]? with
                  | some argumentExpression, some (.reference resultReference) =>
                      match argumentExpression.kind,
                          tables.types[argumentExpression.typeId.index]? with
                      -- A reference-typed field select already normalizes to
                      -- the referent's storage path; only a reference-typed
                      -- value (a reborrow) dereferences its holder.
                      | .operation (.data _) _ _ _, some (.reference _) => (place, places)
                      | _, some (.reference argumentReference) =>
                          if resultReference.referent == argumentReference.referent then
                            internNormalizedPlace places (.deref place)
                          else (place, places)
                      | _, _ => (place, places)
                  | _, _ => (place, places)
                (expressions.push { expression with
                    kind := .operation (.borrow kind place) #[] #[] surface }, places)
            | _ => (expressions.push expression, places)
        | _, _ => (expressions.push expression, places)
    | _ => (expressions.push expression, places)
  { ns with expressions, places }

/-- Namespace after structurization, paired with the data needed to check
exactly the content that pass added: the pre-structurization arena sizes and
which functions arrived as raw graphs. -/
private structure StructurizedNamespace where
  ns : Namespace FunctionBody
  witnesses : Array StructurizationWitness
  expressionBase : Nat
  patternBase : Nat
  convertedFromCfg : Array Bool

/-- Check exactly the content added by structurization.  The raw pass has
already certified everything else, and structurization only appends expression
and pattern nodes and rewrites `.cfg` bodies to `.structured` roots, so this
appendix checks the appended nodes, re-establishes arena acyclicity over the
grown arenas, and checks each converted function's root bounds; semantic
typing of every body, including converted ones, is the typing pass's
responsibility. -/
private def checkStructurizedAppendix (registry : ProfileRegistry) (unit : RawUnit)
    (structurized : StructurizedNamespace) : Array Diagnostic :=
  let ns := structurized.ns
  let exprErrors := (Array.range (ns.expressions.size - structurized.expressionBase)).foldl
    (init := #[]) fun ds offset =>
      match ns.expressions[structurized.expressionBase + offset]? with
      | some expr => ds ++ checkLoc unit.tables expr.loc ++
          checkTypeId unit.tables expr.typeId (some expr.loc) ++
          checkExprKind registry unit ns expr
      | none => ds
  let patternErrors := (Array.range (ns.patterns.size - structurized.patternBase)).foldl
    (init := #[]) fun ds offset =>
      match ns.patterns[structurized.patternBase + offset]? with
      | some pattern =>
          let ds := ds ++ checkLoc unit.tables pattern.loc ++
            checkTypeId unit.tables pattern.typeId (some pattern.loc)
          let ds := (patternChildren pattern.kind).foldl
            (fun ds id => ds ++ checkPatternId ns id (some pattern.loc)) ds
          match pattern.kind with
          | .constructor name instantiations _ _ =>
              instantiations.foldl (fun ds argument =>
                ds ++ checkGenericArgument registry unit unit.tables argument)
                (ds ++ checkName unit.tables name (some pattern.loc))
          | _ => ds
      | none => ds
  let arenaErrors :=
    checkAcyclic "expression" ns.expressions.size fun index =>
      (ns.expressions[index]?).map (expressionChildren ·.kind |>.map (·.index)) |>.getD #[] ++
    checkAcyclic "pattern" ns.patterns.size fun index =>
      (ns.patterns[index]?).map (patternChildren ·.kind |>.map (·.index)) |>.getD #[] ++
    checkAcyclic "place" ns.places.size fun index =>
      (ns.places[index]?).map (placeChildren · |>.map (·.index)) |>.getD #[]
  let functionErrors := (ns.functions.zip structurized.convertedFromCfg).foldl
    (init := #[]) fun ds (function, fromCfg) =>
      if fromCfg then
        match function.body with
        | .structured root => ds ++ checkExprId ns root (some function.loc)
        | .absent => ds
      else ds
  exprErrors ++ patternErrors ++ arenaErrors ++ functionErrors

private def checkProfiles (registry : ProfileRegistry) (unit : RawUnit) : Array Diagnostic :=
  let positionErrors := (Array.range unit.profiles.size).foldl (fun ds index =>
    match unit.profiles[index]? with
    | some config => match config.profile with
      | .extension id =>
          if id.index == index then ds else ds.push <| .error "LIR-PROFILE-IDENTITY"
            s!"extension profile id {id.index} does not identify its position {index}"
      | .move | .rust => ds
    | none => ds) #[]
  let duplicateErrors := (Array.range unit.profiles.size).foldl (fun ds index =>
    match unit.profiles[index]? with
    | some config =>
        if unit.profiles.take index |>.any (·.profile == config.profile) then
          ds.push <| .error "LIR-PROFILE-DUPLICATE"
            s!"profile {repr config.profile} has more than one configuration"
        else ds
    | none => ds) #[]
  let registrationErrors := unit.profiles.foldl (fun ds config =>
    match profileSchema? registry config.profile with
    | none => ds.push <| .error "LIR-PROFILE-UNREGISTERED"
        s!"profile {repr config.profile} ({config.name}) has no registered schema"
    | some schema =>
        if schema.name == config.name && schema.version == config.version then
          let optionErrors := config.options.zipIdx.foldl (init := #[])
              fun ds (option, index) =>
            if config.options.take index |>.any (·.1 == option.1) then
              ds.push <| .error "LIR-PROFILE-OPTION-DUPLICATE"
                s!"profile option `{option.1}` occurs more than once"
            else ds
          ds ++ optionErrors ++ schema.checkConfig config
        else ds.push <| .error "LIR-PROFILE-VERSION"
          s!"profile configuration {config.name}@{config.version} does not match registered {schema.name}@{schema.version}") #[]
  let registryDuplicateErrors := (Array.range registry.size).foldl (fun ds index =>
    match registry[index]? with
    | some schema =>
        if registry.take index |>.any (·.profile == schema.profile) then
          ds.push <| .error "LIR-PROFILE-SCHEMA-DUPLICATE"
            s!"profile {repr schema.profile} has more than one registered schema"
        else ds
    | none => ds) #[]
  positionErrors ++ duplicateErrors ++ registryDuplicateErrors ++ registrationErrors

private def checkVersion (version : Version) : Array Diagnostic :=
  if version.major == 1 && version.minor == 1 then #[] else
    #[.error "LIR-SCHEMA-VERSION"
      s!"unsupported raw LIR schema version {version.major}.{version.minor}; expected 1.1"]

/-- Check one nominal declaration a dependency interface exports. An interface
carries the declaration's shape — its binders, abilities, fields, and variants —
and nothing that belongs to a body: contracts and locals stay with the
declaring unit. -/
private def checkInterfaceStruct (registry : ProfileRegistry) (unit : RawUnit)
    (owner : NamespaceId) (struct : StructDecl) : Array Diagnostic :=
  let binders := binderKinds struct.generics
  let ds := checkLoc unit.tables struct.loc ++
    checkName unit.tables struct.name (some struct.loc) ++
    checkGenericBinders registry unit unit.tables 0 struct.generics ++
    checkAbilityList "nominal declaration" struct.abilities (some struct.loc) ++
    checkDeclarationNames unit.tables owner "struct field"
      (struct.fields.map fun field => (field.name, field.loc)) ++
    struct.fields.foldl (fun ds field => ds ++ checkField unit.tables field ++
      checkScopedTypeUse registry unit unit.tables binders field.type) #[] ++
    struct.properties.foldl (fun ds property =>
      ds ++ checkProfileValue registry unit property ProfileSchema.checkProperty) #[] ++
    checkDeclarationNames unit.tables owner "enum variant"
      (struct.variants.map fun variant => (variant.name, variant.loc)) ++
    struct.variants.foldl (fun ds variant =>
      ds ++ checkLoc unit.tables variant.loc ++
        checkName unit.tables variant.name (some variant.loc) ++
        checkDeclarationNames unit.tables owner "variant field"
          (variant.fields.map fun field => (field.name, field.loc)) ++
        variant.fields.foldl (fun ds field => ds ++ checkField unit.tables field ++
          checkScopedTypeUse registry unit unit.tables binders field.type) #[]) #[]
  let ds := if !struct.fields.isEmpty && !struct.variants.isEmpty then
      ds.push <| .at "LIR-NOMINAL-SHAPE"
        "a nominal declaration cannot contain both struct fields and enum variants" struct.loc
    else ds
  let ds := if struct.contract == ({} : FunctionContract) && struct.locals.isEmpty then ds else
    ds.push <| .at "LIR-DEPENDENCY-INTERFACE"
      "a dependency interface declaration carries no contract or locals" struct.loc
  match unit.tables.names[struct.name.index]? with
  | some name => if name.namespaceId == owner then ds else
      ds.push <| .at "LIR-DEPENDENCY-NAME"
        s!"dependency namespace {owner.index} declares a nominal owned by namespace {name.namespaceId.index}"
        struct.loc
  | none => ds

private def checkDependencies (registry : ProfileRegistry) (unit : RawUnit) : Array Diagnostic :=
  unit.dependencies.zipIdx.foldl (init := #[]) fun ds (dependency, index) =>
    let ds := ds ++ checkNamespaceId unit dependency.namespaceId
    let ds := if dependency.namespaceId.index < unit.namespaces.size then
      ds.push <| .error "LIR-DEPENDENCY-OWNED"
        s!"dependency namespace {dependency.namespaceId.index} is owned by this unit"
    else ds
    let ds := if unit.dependencies.take index |>.any
        (·.namespaceId == dependency.namespaceId) then
      ds.push <| .error "LIR-DEPENDENCY-DUPLICATE"
        s!"namespace {dependency.namespaceId.index} has more than one dependency interface"
    else ds
    let ds := match dependency.profile with
      | some profile => ds ++ checkProfile unit profile
      | none => ds
    let ds := dependency.functions.foldl (init := ds) fun ds function =>
      let ds := ds ++ checkLoc unit.tables function.loc ++
        checkName unit.tables function.name (some function.loc) ++
        checkProfile unit function.profile (some function.loc) ++
        checkSignature registry unit unit.tables 0 function.signature ++
        checkIndex "origin" function.origin.index unit.tables.origins.size (some function.loc) ++
        checkIndex "alignment" function.alignment.index unit.tables.alignments.size
          (some function.loc) ++
        checkAttributes registry unit unit.tables function.attributes
      let ds := if function.body matches .absent && function.contract == ({} : FunctionContract) &&
          function.locals.isEmpty then ds else
        ds.push <| .at "LIR-DEPENDENCY-INTERFACE"
          "a dependency interface function carries a signature only" function.loc
      match unit.tables.names[function.name.index]? with
      | some name => if name.namespaceId == dependency.namespaceId then ds else
          ds.push <| .at "LIR-DEPENDENCY-NAME"
            s!"dependency namespace {dependency.namespaceId.index} declares a function owned by namespace {name.namespaceId.index}"
            function.loc
      | none => ds
    let ds := dependency.specFunctions.foldl (init := ds) fun ds function =>
      let ds := ds ++ checkLoc unit.tables function.loc ++
        checkName unit.tables function.name (some function.loc) ++
        checkProfile unit function.profile (some function.loc) ++
        checkSignature registry unit unit.tables 0 function.signature ++
        checkIndex "origin" function.origin.index unit.tables.origins.size (some function.loc)
      let ds := if function.body.isNone && function.locals.isEmpty then ds else
        ds.push <| .at "LIR-DEPENDENCY-INTERFACE"
          "a dependency interface specification function carries a signature only" function.loc
      match unit.tables.names[function.name.index]? with
      | some name => if name.namespaceId == dependency.namespaceId then ds else
          ds.push <| .at "LIR-DEPENDENCY-NAME"
            s!"dependency namespace {dependency.namespaceId.index} declares a specification function owned by namespace {name.namespaceId.index}"
            function.loc
      | none => ds
    let ds := dependency.structs.zipIdx.foldl (init := ds) fun ds (struct, structIndex) =>
      let ds := ds ++ checkInterfaceStruct registry unit dependency.namespaceId struct
      if dependency.structs.take structIndex |>.any (·.name == struct.name) then
        ds.push <| .at "LIR-DEPENDENCY-NAME-DUPLICATE"
          s!"dependency namespace {dependency.namespaceId.index} declares nominal {struct.name.index} more than once"
          struct.loc
      else ds
    dependency.exportedNames.zipIdx.foldl (init := ds) fun ds (nameId, nameIndex) =>
      let ds := ds ++ checkName unit.tables nameId
      let ds := if dependency.exportedNames.take nameIndex |>.contains nameId then
        ds.push <| .error "LIR-DEPENDENCY-NAME-DUPLICATE"
          s!"dependency namespace {dependency.namespaceId.index} exports name {nameId.index} more than once"
      else ds
      match unit.tables.names[nameId.index]? with
      | some name => if name.namespaceId == dependency.namespaceId then ds else
          ds.push <| .error "LIR-DEPENDENCY-NAME"
            s!"dependency namespace {dependency.namespaceId.index} exports a name owned by namespace {name.namespaceId.index}"
      | none => ds

private def checkUnit (registry : ProfileRegistry) (unit : RawUnit) : Array Diagnostic :=
  (Array.range unit.namespaces.size).foldl (fun diagnostics index =>
    match unit.namespaces[index]? with
    | some ns => diagnostics ++ checkNamespace registry unit index ns
    | none => diagnostics)
    (checkVersion unit.version ++ checkProfiles registry unit ++
      unit.evidence.foldl (fun diagnostics evidence =>
        diagnostics ++
          (if evidence.producer.isEmpty then
            #[.error "LIR-EVIDENCE-PRODUCER" "import evidence must name its producer"]
          else #[]) ++
          (if evidence.description.isEmpty then
            #[.error "LIR-EVIDENCE-DESCRIPTION" "import evidence must describe its claim"]
          else #[])) #[] ++
      checkTables registry unit unit.tables ++ checkDependencies registry unit ++
      checkAcyclic "type" unit.tables.types.size (fun index =>
        (unit.tables.types[index]?).map (typeChildren · |>.map (·.index)) |>.getD #[]) ++
      checkMoveNominalCycles unit ++
      (if unit.namespaces.size <= unit.tables.namespaces.size then #[] else
        #[.error "LIR-NAMESPACE-TABLE"
          s!"unit has {unit.namespaces.size} owned namespaces but its table has only {unit.tables.namespaces.size} entries"]))

/-- A mutable reference held in a local and passed by name to a mutable
reference parameter is copied into the call, and in the prophecy model a
copy of a mutable reference is a reborrow: the argument becomes
`&mut *local`, a fresh loan whose hole rests in the caller's frame and whose
death the borrow analysis records, so the callee's write-back lands where
the caller reads and exports.  Frontends spell such an argument as the
bare local, the way their sources do, and the printer spells it back that
way; the rewrite keeps the argument's arena slot, so no ordering changes. -/
private def normalizeReferenceArguments (unit : RawUnit) : RawUnit :=
  let parameterTypes (reference : QualifiedRef) : Option (Array TypeId) :=
    let signature? :=
      if reference.namespaceId.index < unit.namespaces.size then
        unit.namespaces[reference.namespaceId.index]?.bind fun ns =>
          (ns.functions.find? (·.name == reference.name)).map (·.signature)
      else
        (unit.dependencies.find? (·.namespaceId == reference.namespaceId)).bind fun interface =>
          (interface.functions.find? (·.name == reference.name)).map (·.signature)
    signature?.map fun signature => signature.parameters.map (·.typeUse.typeId)
  let mutableReference? (tables : Tables) (typeId : TypeId) : Option ReferenceType :=
    match tables.types[typeId.index]? with
    | some (.reference reference) =>
        if reference.kind == .mutable then some reference else none
    | _ => none
  let (tables, namespaces) := unit.namespaces.foldl (init := (unit.tables, #[]))
    fun (tables, namespaces) ns =>
      let (tables, expressions, places) :=
        ns.expressions.foldl (init := (tables, ns.expressions, ns.places))
          fun (tables, expressions, places) node =>
            match node.kind with
            | .operation (.call (.function reference)) _ arguments _ =>
                match parameterTypes reference with
                | none => (tables, expressions, places)
                | some parameters =>
                    (arguments.zip parameters).foldl (init := (tables, expressions, places))
                      fun (tables, expressions, places) (argument, parameter) =>
                        match expressions[argument.index]? with
                        | some argumentNode =>
                            match argumentNode.kind, mutableReference? tables argumentNode.typeId,
                                mutableReference? tables parameter with
                            | .localVar localId, some reference, some _ =>
                                let lifetime : LifetimeId := ⟨tables.lifetimes.size⟩
                                let inferred : Lifetime := { kind := .inference, loc := argumentNode.loc }
                                let tables := { tables with lifetimes := tables.lifetimes.push inferred }
                                let typeId : TypeId := ⟨tables.types.size⟩
                                let reborrowed : Ty := .reference { reference with lifetime }
                                let tables := { tables with types := tables.types.push reborrowed }
                                let localPlace : PlaceId := ⟨places.size⟩
                                let places := places.push (.localVar localId)
                                let derefPlace : PlaceId := ⟨places.size⟩
                                let places := places.push (.deref localPlace)
                                let expressions := expressions.set! argument.index
                                  { argumentNode with
                                    typeId
                                    kind := .operation (.borrow .mutable derefPlace) #[] #[] none }
                                (tables, expressions, places)
                            | _, _, _ => (tables, expressions, places)
                        | none => (tables, expressions, places)
            | _ => (tables, expressions, places)
      (tables, namespaces.push { ns with expressions, places })
  { unit with tables, namespaces }

/-- Validate a raw compilation unit.  This is the only public constructor of
`ValidatedUnit`; all backends should accept that checked type.  Each check
family runs exactly once: the raw pass certifies the frontend-supplied unit,
structurization converts every graph body while recording a checked witness,
the appendix pass checks only the content structurization added, and the
resolution pass resolves every authoritative name use through the checked
`ResolutionIndex` retained by the validated unit. -/
def validate (registry : ProfileRegistry) (rawUnit : RawUnit) : Except (Array Diagnostic) ValidatedUnit := do
  let rawUnit := { rawUnit with
    namespaces := rawUnit.namespaces.map (normalizeValueBorrows rawUnit.tables) }
  let rawUnit := normalizeReferenceArguments rawUnit
  let rawDiagnostics := checkUnit registry rawUnit
  if rawDiagnostics.any (·.severity == .error) then
    throw (dedupDiagnostics rawDiagnostics)
  let structurized ← rawUnit.namespaces.zipIdx.mapM fun (ns, index) => do
    let (structuredNs, witnesses) ←
      Import.Structurize.structurizeNamespace rawUnit.tables ⟨index⟩ ns
    return { ns := structuredNs
             witnesses
             expressionBase := ns.expressions.size
             patternBase := ns.patterns.size
             convertedFromCfg := ns.functions.map fun function =>
               match function.body with
               | .cfg _ => true
               | _ => false
             : StructurizedNamespace }
  let appendixDiagnostics := structurized.foldl (init := #[]) fun ds s =>
    ds ++ checkStructurizedAppendix registry rawUnit s
  if appendixDiagnostics.any (·.severity == .error) then
    throw (dedupDiagnostics appendixDiagnostics)
  let structuredNamespaces := structurized.map (·.ns)
  let resolution := ResolutionIndex.build rawUnit.tables structuredNamespaces
    rawUnit.dependencies
  let resolutionDiagnostics := resolveUseSites rawUnit.tables resolution
    rawUnit.namespaces.size structuredNamespaces
  if resolutionDiagnostics.any (·.severity == .error) then
    throw (dedupDiagnostics resolutionDiagnostics)
  let namespaces := structurized.map fun s =>
    ({ toNamespace := s.ns, tables := rawUnit.tables } : ValidatedNamespace)
  let unit := Internal.mkValidatedUnit rawUnit.tables rawUnit.profiles namespaces
    (rawUnit.dependencies.map fun dep => {
      namespaceId := dep.namespaceId
      profile := dep.profile
      exportedNames := dep.exportedNames
      structs := dep.structs
      -- An interface declares signatures only, so every body is absent on
      -- both sides of this conversion.
      functions := dep.functions.map fun declaration => declaration.withBody .absent
      specFunctions := dep.specFunctions })
    (rawUnit.evidence.map fun evidence => {
      producer := evidence.producer
      description := evidence.description
      trusted := evidence.trusted })
    { namespaceCount := namespaces.size
      functionCounts := namespaces.map (·.functions.size) }
    (structurized.flatMap (·.witnesses))
    resolution
  let typingErrors := typingDiagnostics unit
  if typingErrors.any (·.severity == .error) then
    throw (dedupDiagnostics typingErrors)
  -- Definite initialization and the borrow analysis run exactly once here;
  -- preparation copies the certificates instead of re-running the analyses.
  -- Initialization failures are validate errors. Borrow-analysis rejections
  -- are recorded and replayed by preparation: the analysis still lacks
  -- non-lexical loan death and region solving, so making them validate
  -- errors would reject legitimate source programs.
  let (initializationErrors, initializationCertificates, borrowCertificates,
      borrowDiagnostics) :=
    unit.namespaces.foldl (init := (#[], #[], #[], #[])) fun acc ns =>
      ns.functions.zipIdx.foldl (init := acc)
        fun (initDs, inits, borrows, borrowDs) (function, index) =>
          let (functionInitDs, initCert) :=
            initializationOutcome ns.identity ⟨index⟩ ns function
          let (functionBorrowDs, borrowCert) :=
            borrowOutcome ns.identity ⟨index⟩ unit ns function
          (initDs ++ functionInitDs,
            match initCert with | some c => inits.push c | none => inits,
            match borrowCert with | some c => borrows.push c | none => borrows,
            borrowDs ++ functionBorrowDs)
  if initializationErrors.any (·.severity == .error) then
    throw (dedupDiagnostics initializationErrors)
  return Internal.mkValidatedUnit unit.tables unit.profiles unit.namespaces
    unit.dependencies unit.evidence unit.indexes unit.structurizationWitnesses
    unit.resolution initializationCertificates borrowCertificates borrowDiagnostics

def ValidatedUnit.namespace? (unit : ValidatedUnit) (id : NamespaceId) : Option ValidatedNamespace :=
  unit.namespaces[id.index]?

def ValidatedNamespace.expression? (ns : ValidatedNamespace) (id : ExprId) : Option Expr :=
  ns.expressions[id.index]?

def ValidatedNamespace.pattern? (ns : ValidatedNamespace) (id : PatternId) : Option Pattern :=
  ns.patterns[id.index]?

def ValidatedNamespace.place? (ns : ValidatedNamespace) (id : PlaceId) : Option Place :=
  ns.places[id.index]?

end LeanerIR.Validation
