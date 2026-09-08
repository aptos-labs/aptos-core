-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Data.Json
import LeanerLang.Builtins
import LeanerLang.Profile
import LeanerLang.Print.Layout

/-!
# Canonical LeanerLang source backend

This backend prints the executable, profile-neutral subset currently accepted
by the LeanerLang elaborator.  It consumes only validated LIR and fails at an
unsupported node rather than approximating it.  In particular, it recognizes
the single-result forwarding block produced by rustc MIR structurization and
prints the underlying source expression.
-/

namespace LeanerLang.Print

open LeanerIR
open LeanerIR.Validation

/-- A backend rejection with the authored LIR range responsible for it when
the frontend supplied one. Baseline drivers can turn the byte range into the
source language's ordinary line-and-column diagnostic spelling. -/
structure Error where
  message : String
  location : Option (SourceFile × SourceRange) := none
  deriving Repr, Inhabited

instance : ToString Error where
  toString error := match error.location with
    | some (source, range) =>
        s!"{source.name}:[{range.startByte}, {range.endByte}): {error.message}"
    | none => error.message

private partial def primaryRange? (tables : Tables) (id : LocId) (fuel : Nat) :
    Option SourceRange := do
  if fuel == 0 then none else
    let location ← tables.locations[id.index]?
    match location.primary with
    | some range => some range
    | none => location.parent.bind fun parent => primaryRange? tables parent (fuel - 1)

private def errorAt (unit : ValidatedUnit) (loc : LocId) (message : String) : Error :=
  let location := do
    let range ← primaryRange? unit.tables loc (unit.tables.locations.size + 1)
    let source ← unit.tables.files[range.file.index]?
    some (source, range)
  { message, location }

private def atLocation (unit : ValidatedUnit) (loc : LocId) (result : Except String α) :
    Except Error α :=
  result.mapError (errorAt unit loc)

private def commaSep (values : Array String) : String :=
  ", ".intercalate values.toList

/-- Close angle-bracketed type arguments without letting Lean's lexer combine
nested closing delimiters into the shift token `>>`. -/
private def typeArguments (value : String) : String :=
  "<" ++ value ++ (if value.endsWith ">" then " >" else ">")

private def lines (values : Array String) : String :=
  "\n".intercalate values.toList

private def indent (value : String) : String :=
  "\n".intercalate <| value.splitOn "\n" |>.map fun line =>
    if line.isEmpty then "" else "  " ++ line

/-- Render a sequence of block entries.

A block whose single entry binds nothing *is* that entry: the surface language
draws no distinction, and both the layout stage and the parser collapse it. So
must this renderer. Emitting the `do` wrapper anyway made the semantic form of
a unit depend on whether its frontend happened to introduce a block, which is
invisible in the printed source — the printer's canonical fixed point then held
only when the layout of the two forms coincided, and broke as soon as an
unrelated width changed. A single `let` entry keeps its block, because a
binding needs the scope the block provides. -/
private def blockText (entries : Array String) : String :=
  let wrapped := s!"do\n{indent (lines entries)}"
  match entries with
  | #[entry] =>
      -- The statement terminator belongs to the block, not to the statement:
      -- outside one the entry stands alone as an expression.
      if entry.startsWith "let " || entry.contains '\n' then wrapped
      else if entry.endsWith ";" then entry.dropRight 1
      else entry
  | _ => wrapped

private def fitsIndentedLine (value : String) : Bool :=
  -- Top-level declarations are indented once by the namespace command.
  value.length + 2 <= 80

private def sourceReservedIdentifier (value : String) : Bool :=
  value ∈ [
    "move", "rust", "Unit", "Never", "Bool", "Char", "string", "Bytes",
    "Address", "Signer", "UInt", "SInt", "UPtr", "IPtr", "Nat", "Int",
    "u8", "u16", "u32", "u64", "u128", "u256",
    "i8", "i16", "i32", "i64", "i128", "i256", "usize", "isize",
    "Range", "Vector", "Fn", "const", "type", "lifetime", "evidence", "mut",
    "private", "public", "package", "friend", "entry", "native", "opaque", "deprecated", "view",
    "pragma", "let_pre", "let_post", "requires", "ensures", "aborts_if", "assert", "assume",
    "invariant", "spec", "fun", "module", "namespace", "using", "where", "struct",
    "enum", "has", "Copy", "Drop", "Store", "Key", "true", "false",
    "abort", "panic", "do", "let", "loop", "while", "for", "break", "continue", "forall",
    "exists", "in", "immutable", "if", "then", "else", "return", "old",
    "copy", "drop", "discriminant", "invoke", "function",
    "as", "match", "with", "use"
  ]

private def sourceIdentifier (value : String) : String :=
  let value := if value.startsWith "$" then
      "_" ++ (value.drop 1).toString
    else if value.startsWith "'" then
      (value.drop 1).toString
    else value
  if !value.isEmpty && value.all Char.isDigit then value
  else (Lean.Name.mkSimple value).toStringWithToken
    (isToken := sourceReservedIdentifier)

/-- Compiler-v2 records the authored receiver head (for example
`self.list.contains`) in the call name for some Move 2 calls. The semantic
callee is still the final identifier; paths before it belong to the receiver
expression and must never be escaped as one giant LeanerLang identifier. -/
private def surfaceReferenceName (value : String) : String :=
  (value.splitOn ".").getLast?.getD value

private def executableReferenceName (value : String) : String :=
  let value := surfaceReferenceName value
  if value.startsWith "$" then value.drop 1 |>.toString else value

/-- A namespace can be imported by its final segment when that segment is
unambiguous and does not collide with a declaration in the current namespace.
This is the fallback for symbols such as `mem::replace` whose unqualified name
would shadow a local declaration. -/
private def importableNamespace (unit : ValidatedUnit) (current target : NamespaceId) : Bool :=
  if target == current then false else
  match unit.tables.namespaces[target.index]?.bind (·.segments.back?) with
  | none => false
  | some alias =>
      let shadowed := unit.tables.names.any (fun candidate =>
        candidate.namespaceId == current && executableReferenceName candidate.name == alias)
      let unique := unit.tables.namespaces.zipIdx.all fun (candidate, index) =>
        candidate.segments.back? != some alias || index == target.index
      !alias.isEmpty && !shadowed && unique

private def nameAt (unit : ValidatedUnit) (id : NameId) : Except String String := do
  let some name := unit.tables.names[id.index]?
    | throw s!"LeanerLang source references missing name {id.index}"
  pure (sourceIdentifier name.name)

private def namespacePath (unit : ValidatedUnit) (id : NamespaceId) : Except String String := do
  let some path := unit.tables.namespaces[id.index]?
    | throw s!"LeanerLang source references missing namespace {id.index}"
  if path.segments.isEmpty then throw "LeanerLang cannot print an empty namespace path"
  pure ("::".intercalate (path.segments.map sourceIdentifier).toList)

private def pushReferencedName (names : Array NameId) (name : NameId) : Array NameId :=
  if names.contains name then names else names.push name

private partial def referencedTypeNames (unit : ValidatedUnit) (fuel : Nat)
    (typeId : TypeId) (initial : Array NameId) : Array NameId :=
  if fuel == 0 then initial else
  match unit.tables.types[typeId.index]? with
  | none => initial
  | some type =>
    let visitTypes (types : Array TypeId) (names : Array NameId) :=
      types.foldl (fun names type => referencedTypeNames unit (fuel - 1) type names) names
    let visitArguments (arguments : Array GenericArgument) (names : Array NameId) :=
      arguments.foldl (fun names argument => match argument with
        | .typeArg type => referencedTypeNames unit (fuel - 1) type.typeId names
        | _ => names) names
    match type with
    | .tuple elements => visitTypes elements initial
    | .vector element _ | .typeDomain element =>
        referencedTypeNames unit (fuel - 1) element initial
    | .resourceDomain resource arguments =>
        visitTypes (arguments.getD #[]) (pushReferencedName initial resource)
    | .nominal name arguments => visitArguments arguments (pushReferencedName initial name)
    | .function arguments result _ =>
        referencedTypeNames unit (fuel - 1) result (visitTypes arguments initial)
    | .reference reference => referencedTypeNames unit (fuel - 1) reference.referent initial
    | _ => initial

private def operationReferencedNames (operation : Operation) : Array NameId :=
  let refs : Array QualifiedRef := match operation with
    | .call (.function reference) | .call (.constructor reference _) |
        .call (.destructor reference _) | .call (.closure reference) => #[reference]
    | .call (.extension _ targets) | .profile _ targets => targets
    | .data (.select reference _) | .data (.selectVariants reference _) |
        .data (.testVariants reference _) | .data (.discriminant reference) |
        .data (.updateField reference _) => #[reference]
    | .specification (.functionCall reference _) => #[reference]
    | _ => #[]
  refs.foldl (fun names reference => pushReferencedName names reference.name) #[]

/-- A specification function whose meaning the source derives again from the
declaration it belongs to: Move compiler-v2's hidden `$name` companion of an
executable function, the LeanerLang `spec f` contract of a function of the same
name, and the arbitrary-value symbols LeanerLang invents while lowering. The
printer emits none of them, so nothing about them reaches the source. -/
private def isDerivedSpecFunction (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : LeanerIR.SpecFunctionDecl) : Bool :=
  match unit.tables.names[declaration.name.index]? with
  | none => false
  | some specName =>
    let leanerArbitrary := specName.name.startsWith "__leaner_arbitrary_" &&
      declaration.body.isNone && declaration.signature.parameters.isEmpty
    leanerArbitrary || ns.functions.any fun function =>
      match unit.tables.names[function.name.index]? with
      | none => false
      | some functionName =>
          let sameLeanerName := declaration.name == function.name
          let moveFunctionProperty : ProfileValue := {
            profile := .move, tag := "specFunction.moveFunction" }
          let moveCompanion := declaration.profile == .move &&
            specName.namespaceId == functionName.namespaceId &&
            specName.name == "$" ++ functionName.name &&
            declaration.profileData.contains moveFunctionProperty
          declaration.profile == function.profile && (sameLeanerName || moveCompanion)

/-- Names reachable from one owned namespace's declaration roots. Namespace
arenas can retain dead nodes after inlining and normalization; scanning every
entry would make unrelated dependency references induce bogus `use` clauses. -/
private def referencedNames (unit : ValidatedUnit)
    (ns : ValidatedNamespace) : Array NameId := Id.run do
  let fuel := unit.tables.types.size + 1
  let visitType (names : Array NameId) (typeId : TypeId) :=
    referencedTypeNames unit fuel typeId names
  let visitTypeUse (names : Array NameId) (type : TypeUse) := visitType names type.typeId
  let visitArguments (names : Array NameId) (arguments : Array GenericArgument) :=
    arguments.foldl (fun names argument => match argument with
      | .typeArg type => visitTypeUse names type
      | _ => names) names
  let visitBinders (names : Array NameId) (binders : Array LeanerIR.GenericBinder) :=
    binders.foldl (fun names binder => binder.type.map (visitTypeUse names) |>.getD names) names
  let visitSignature (names : Array NameId) (signature : Signature) :=
    let names := visitBinders names signature.generics
    let names := signature.parameters.foldl (fun names parameter =>
      visitTypeUse names parameter.typeUse) names
    signature.results.foldl visitTypeUse names
  let visitLocals (names : Array NameId) (locals : Array LocalDecl) :=
    locals.foldl (fun names declaration => visitTypeUse names declaration.type) names
  let mut names := #[]
  let mut expressions : Array ExprId := #[]
  let mut patterns : Array PatternId := #[]
  let mut places : Array PlaceId := #[]
  let mut seenExpressions : Array ExprId := #[]
  let mut seenPatterns : Array PatternId := #[]
  let mut seenPlaces : Array PlaceId := #[]
  let enqueueCondition (roots : Array ExprId) (condition : Condition) :=
    (roots.push condition.expression) ++ condition.auxiliary.map (·.2)
  let enqueueContract (roots : Array ExprId) (contract : FunctionContract) :=
    let roots := contract.conditions.foldl enqueueCondition roots
    roots ++ contract.modifies
  for declaration in ns.constants do
    names := visitTypeUse names declaration.type
    expressions := expressions.push declaration.value
  for declaration in ns.structs do
    names := visitBinders names declaration.generics
    for field in declaration.fields do names := visitTypeUse names field.type
    for variant in declaration.variants do
      for field in variant.fields do names := visitTypeUse names field.type
    names := visitLocals names declaration.locals
    for type in declaration.contract.reads do names := visitTypeUse names type
    expressions := enqueueContract expressions declaration.contract
  for declaration in ns.functions do
    names := visitSignature names declaration.signature
    names := visitLocals names declaration.locals
    for type in declaration.contract.reads do names := visitTypeUse names type
    expressions := enqueueContract expressions declaration.contract
    match declaration.body with
    | .absent => pure ()
    | .structured root => expressions := expressions.push root
  for declaration in ns.specFunctions do
    -- A derived specification function has no printed text, so a symbol only
    -- its body mentions is not part of this namespace's import surface. The
    -- source derives it again from the declaration it belongs to.
    unless isDerivedSpecFunction unit ns declaration do
      names := visitSignature names declaration.signature
      names := visitLocals names declaration.locals
      for type in declaration.contract.reads do names := visitTypeUse names type
      expressions := enqueueContract expressions declaration.contract
      if let some root := declaration.body then expressions := expressions.push root
  for declaration in ns.specVars do
    names := visitBinders names declaration.generics
    names := visitTypeUse names declaration.type
    names := visitLocals names declaration.locals
    if let some root := declaration.init then expressions := expressions.push root
  for declaration in ns.invariants do
    names := visitLocals names declaration.locals
    expressions := enqueueCondition expressions declaration.condition

  let mut expressionIndex := 0
  let mut patternIndex := 0
  let mut placeIndex := 0
  while expressionIndex < expressions.size || patternIndex < patterns.size ||
      placeIndex < places.size do
    if expressionIndex < expressions.size then
      let id := expressions[expressionIndex]!
      expressionIndex := expressionIndex + 1
      unless seenExpressions.contains id do
        seenExpressions := seenExpressions.push id
        if let some expression := ns.expressions[id.index]? then
          names := visitType names expression.typeId
          match expression.kind with
          | .value .. | .localVar .. | .continue_ .. => pure ()
          | .constant reference => names := pushReferencedName names reference.name
          | .operation operation instantiations arguments _ =>
              for name in operationReferencedNames operation do
                names := pushReferencedName names name
              names := visitArguments names instantiations
              expressions := expressions ++ arguments
              match operation with
              | .move place | .copy place | .borrow _ place | .read place |
                  .write place | .drop place => places := places.push place
              | _ => pure ()
          | .block statements result =>
              expressions := expressions ++ statements
              if let some result := result then expressions := expressions.push result
          | .letDecl pattern value body =>
              patterns := patterns.push pattern
              if let some value := value then expressions := expressions.push value
              expressions := expressions.push body
          | .ifElse condition thenBranch elseBranch =>
              expressions := expressions.push condition |>.push thenBranch
              if let some elseBranch := elseBranch then
                expressions := expressions.push elseBranch
          | .match_ scrutinee arms =>
              expressions := expressions.push scrutinee
              for arm in arms do
                patterns := patterns.push arm.pattern
                if let some guard := arm.guard then expressions := expressions.push guard
                expressions := expressions.push arm.body
          | .loop _ body => expressions := expressions.push body
          | .break_ _ value =>
              if let some value := value then expressions := expressions.push value
          | .return_ values | .throw_ _ values =>
              expressions := expressions ++ values
          | .assign place value =>
              places := places.push place
              expressions := expressions.push value
          | .assignPattern pattern value =>
              patterns := patterns.push pattern
              expressions := expressions.push value
          | .quantifier _ binders triggers condition body =>
              for binder in binders do
                patterns := patterns.push binder.pattern
                expressions := expressions.push binder.domain
              for trigger in triggers do expressions := expressions ++ trigger
              if let some condition := condition then expressions := expressions.push condition
              expressions := expressions.push body
          | .spec block =>
              for condition in block.conditions do
                expressions := enqueueCondition expressions condition
              if let some frame := block.frame then
                expressions := expressions ++ frame.modifies
                for type in frame.reads do names := visitTypeUse names type
    else if patternIndex < patterns.size then
      let id := patterns[patternIndex]!
      patternIndex := patternIndex + 1
      unless seenPatterns.contains id do
        seenPatterns := seenPatterns.push id
        if let some pattern := ns.patterns[id.index]? then
          names := visitType names pattern.typeId
          match pattern.kind with
          | .tuple children => patterns := patterns ++ children
          | .constructor name instantiations _ children =>
              names := pushReferencedName names name
              names := visitArguments names instantiations
              patterns := patterns ++ children
          | _ => pure ()
    else
      let id := places[placeIndex]!
      placeIndex := placeIndex + 1
      unless seenPlaces.contains id do
        seenPlaces := seenPlaces.push id
        if let some place := ns.places[id.index]? then
          match place with
          | .localVar .. => pure ()
          | .deref base | .subslice base .. => places := places.push base
          | .field base .. | .downcast base _ => places := places.push base
          | .index base index =>
              places := places.push base
              expressions := expressions.push index
  return names

/-- Only declarations owned by the namespace being printed can collide with
an imported symbol. Dependency declarations may or may not be present in a
validated unit, so they must not influence canonical source. -/
private def ownedDeclarationHasName (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (value : String) : Bool :=
  let hasName (id : NameId) := unit.tables.names[id.index]?.any fun entry =>
    executableReferenceName entry.name == value
  ns.constants.any (hasName ·.name) ||
    ns.structs.any (fun declaration =>
      hasName declaration.name || declaration.variants.any (hasName ·.name)) ||
    ns.functions.any (hasName ·.name) ||
    ns.specFunctions.any (hasName ·.name) ||
    ns.specVars.any (hasName ·.name)

/-- Calls which canonical LeanerLang spells as intrinsic syntax do not need a
source import even though their imported Move LIR retains the standard-library
callee identity.  Keeping this list narrower than the receiver inventory is
important: ordinary methods such as `push_back` still need `use std::vector`
so lowering can resolve their receiver notation. -/
private def importFreeSurfaceName (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (id : NameId) : Bool :=
  match unit.tables.names[id.index]? with
  | none => false
  | some target =>
      unit.tables.namespaces[target.namespaceId.index]?.any fun namespaceRef =>
        isMoveStdModule ns.profile namespaceRef "vector" &&
          executableReferenceName target.name ∈ ["borrow", "borrow_mut", "length"]

/-- A referenced symbol can be imported without an alias when its surface
name is unambiguous among this namespace's actual references and does not
collide with an owned declaration. Looking only at those semantic roots keeps
the result independent of which dependency namespaces happen to be loaded. -/
private def importableName (unit : ValidatedUnit) (current : NamespaceId)
    (id : NameId) : Bool :=
  match unit.tables.names[id.index]?, unit.namespaces.find? (·.identity == current) with
  | some target, some ns =>
      let targetName := executableReferenceName target.name
      let surfaceBuiltin := unit.tables.namespaces[target.namespaceId.index]?.any
        fun namespaceRef =>
          (standardCallReceiverMode? ns.profile namespaceRef targetName).isSome
      let references := referencedNames unit ns
      target.namespaceId != current && !targetName.isEmpty &&
        !targetName.startsWith "$" && !surfaceBuiltin &&
        !ownedDeclarationHasName unit ns targetName &&
        references.all fun candidateId =>
          -- A symbol the surface spells only as its own syntax is never
          -- written as a name, so it cannot be taken for this alias.
          importFreeSurfaceName unit ns candidateId ||
          match unit.tables.names[candidateId.index]? with
          | none => false
          | some candidate =>
              executableReferenceName candidate.name != targetName ||
                candidate.namespaceId == target.namespaceId
  | _, _ => false

private def qualifiedNameAt (unit : ValidatedUnit) (current : NamespaceId)
    (id : NameId) : Except String String := do
  let some name := unit.tables.names[id.index]?
    | throw s!"LeanerLang source references missing name {id.index}"
  let localNameText := sourceIdentifier (surfaceReferenceName name.name)
  if name.namespaceId == current then pure localNameText
  else if importableName unit current id then pure localNameText
  else if importableNamespace unit current name.namespaceId then
    let some alias := unit.tables.namespaces[name.namespaceId.index]?.bind (·.segments.back?)
      | throw s!"LeanerLang source references an empty namespace {name.namespaceId.index}"
    pure s!"{sourceIdentifier alias}::{localNameText}"
  else pure s!"{← namespacePath unit name.namespaceId}::{localNameText}"

private def inferredUsePathsFor (unit : ValidatedUnit) (ns : ValidatedNamespace) :
    Except String (Array String) := do
  let current := ns.identity
  let mut paths := #[]
  for id in referencedNames unit ns do
    let some name := unit.tables.names[id.index]?
      | throw s!"LeanerLang source references missing name {id.index}"
    if importFreeSurfaceName unit ns id then
      pure ()
    else if importableName unit current id then
      let path := s!"{← namespacePath unit name.namespaceId}::{
        sourceIdentifier (executableReferenceName name.name)}"
      unless paths.contains path do paths := paths.push path
    else if importableNamespace unit current name.namespaceId then
      let path ← namespacePath unit name.namespaceId
      unless paths.contains path do paths := paths.push path
  pure <| paths.qsort (· < ·)

private def inferredUsePaths (unit : ValidatedUnit) (current : NamespaceId) :
    Except String (Array String) := do
  let some ns := unit.namespaces.find? (·.identity == current)
    | throw s!"LeanerLang source references missing owned namespace {current.index}"
  inferredUsePathsFor unit ns

private def binderName (binders : Array LeanerIR.GenericBinder) (index : Nat) : Except String String := do
  let some binder := binders[index]?
    | throw s!"LeanerLang source references missing generic binder {index}"
  pure (sourceIdentifier binder.name)

private def abilityText : LeanerIR.Ability → String
  | .copy => "Copy"
  | .drop => "Drop"
  | .store => "Store"
  | .key => "Key"

private def abilitiesText (abilities : Array LeanerIR.Ability) : String :=
  if abilities.isEmpty then "" else
    " has " ++ commaSep (abilities.map abilityText)

private def binderText (unit : ValidatedUnit) (binder : LeanerIR.GenericBinder) :
    Except String String := do
  -- A phantom type parameter is a predicate the surface spells in the binder
  -- itself; every other predicate has no LeanerLang spelling yet.
  let phantom := binder.predicates.any fun
    | .profile { profile := .move, tag := "typeParameter.phantom", payload } =>
        payload.isEmpty
    | _ => false
  let remaining := binder.predicates.size - (if phantom then 1 else 0)
  unless remaining == 0 do
    throw s!"generic binder `{binder.name}` has predicates outside the current LeanerLang printer"
  if phantom && binder.kind != .typeArg then
    throw s!"non-type binder `{binder.name}` is marked phantom"
  if binder.kind != .typeArg && !binder.abilities.isEmpty then
    throw s!"non-type binder `{binder.name}` carries abilities"
  let annotation ← match binder.kind, binder.type with
  | .const, some type =>
      let type ← match unit.tables.types[type.typeId.index]? with
        | some .bool => pure "Bool"
        | some (.integer (.bits width) false) => pure s!"u{width}"
        | some (.integer (.bits width) true) => pure s!"i{width}"
        | some (.integer .pointer false) => pure "usize"
        | some (.integer .pointer true) => pure "isize"
        | _ => throw s!"const binder `{binder.name}` has an unsupported declared type"
      pure s!" : const {type}"
  | .const, none => throw s!"const binder `{binder.name}` has no declared type"
  | _, some _ => throw s!"non-const binder `{binder.name}` carries a const type"
  | .typeArg, none => pure ""
  | .lifetime, none => pure " : lifetime"
  | .evidence, none => pure " : evidence"
  let annotation := if phantom then " : phantom type" else annotation
  pure ("{" ++ sourceIdentifier binder.name ++ annotation ++
    abilitiesText binder.abilities ++ "}")

mutual
  private partial def typeTextFuel (unit : ValidatedUnit)
      (ns : ValidatedNamespace) (binders : Array LeanerIR.GenericBinder)
      (id : TypeId) (fuel : Nat) : Except String String := do
    if fuel == 0 then throw "cyclic type reached the LeanerLang source backend"
    let some type := unit.tables.types[id.index]?
      | throw s!"LeanerLang source references missing type {id.index}"
    match type with
    | .unit => pure "Unit"
    | .never => pure "Never"
    | .bool => pure "Bool"
    | .character => pure "Char"
    | .string => pure "string"
    | .bytes => pure "Bytes"
    | .address => pure "Address"
    | .signer => pure "Signer"
    | .integer (.bits width) false => pure s!"u{width}"
    | .integer (.bits width) true => pure s!"i{width}"
    | .integer .pointer false => pure "usize"
    | .integer .pointer true => pure "isize"
    | .integer .unbounded false => pure "Nat"
    | .integer .unbounded true => pure "Int"
    | .range => pure "Range"
    | .tuple elements =>
        if elements.isEmpty then pure "Unit" else
          pure s!"({commaSep (← elements.mapM (typeTextFuel unit ns binders · (fuel - 1)))})"
    | .vector element none =>
        pure <| "Vector" ++ typeArguments (← typeTextFuel unit ns binders element (fuel - 1))
    | .vector element (some (.integer length)) =>
        if length < 0 then throw "a fixed vector has a negative length"
        pure <| "Vector" ++ typeArguments
          s!"{← typeTextFuel unit ns binders element (fuel - 1)}, const {length}"
    | .vector _ (some _) => throw "a fixed vector length is not an integer constant"
    | .function arguments result abilities =>
        let arguments ← arguments.mapM (typeTextFuel unit ns binders · (fuel - 1))
        let result ← typeTextFuel unit ns binders result (fuel - 1)
        pure s!"Fn({commaSep arguments}) -> {result}{abilitiesText abilities}"
    | .reference reference =>
        unless ns.profile == some reference.profile do
          throw "a reference type uses a profile different from its namespace"
        let some lifetime := unit.tables.lifetimes[reference.lifetime.index]?
          | throw s!"LeanerLang source references missing lifetime {reference.lifetime.index}"
        let lifetime ← match lifetime.kind with
          | .inference => pure ""
          | .static => pure "[static]"
          | .parameter index => pure s!"[{← binderName binders index}]"
          | .local =>
              let name := lifetime.name.getD s!"local{reference.lifetime.index}"
              pure s!"[{sourceIdentifier name}]"
        let mutability := if reference.kind == .mutable then "mut " else ""
        let referent ← typeTextFuel unit ns binders reference.referent (fuel - 1)
        let separator := if !lifetime.isEmpty then " "
          else if mutability.isEmpty && referent.startsWith "&" then " " else ""
        pure s!"&{lifetime}{mutability}{separator}{referent}"
    | .typeParameter index => binderName binders index
    | .nominal name arguments =>
        let name ← qualifiedNameAt unit ns.identity name
        if arguments.isEmpty then pure name else
          let arguments ← arguments.mapM fun argument => match argument with
            | .typeArg value => typeTextFuel unit ns binders value.typeId (fuel - 1)
            | .const (.integer value) => pure s!"const {value}"
            | .const (.bool value) => pure s!"const {value}"
            | .const _ => throw "this const nominal argument has no LeanerLang spelling"
            | .lifetime lifetimeId => do
                let some lifetime := unit.tables.lifetimes[lifetimeId.index]?
                  | throw s!"nominal type references missing lifetime {lifetimeId.index}"
                let value ← match lifetime.kind with
                  | .inference => pure "_"
                  | .static => pure "static"
                  | .parameter index => binderName binders index
                  | .local => pure (sourceIdentifier
                      (lifetime.name.getD s!"local{lifetimeId.index}"))
                pure s!"lifetime {value}"
            | .evidence _ => throw "evidence nominal arguments are outside the current parser"
          pure <| name ++ typeArguments (commaSep arguments)
    | .eventStore | .typeDomain _ | .resourceDomain .. | .stateDomain | .profile _ =>
        throw s!"validated type {id.index} is outside the current LeanerLang parser"

  private partial def typeText (unit : ValidatedUnit) (ns : ValidatedNamespace)
      (binders : Array LeanerIR.GenericBinder) (id : TypeId) : Except String String :=
    typeTextFuel unit ns binders id (unit.tables.types.size + 1)
end

private def byteVectorText? (values : Array ConstValue) : Option String := do
  let bytes ← values.mapM fun
    | .integer value => if 0 <= value && value <= 255 then some value.toNat else none
    | _ => none
  guard <| bytes.all fun value =>
    value >= 0x20 && value < 0x7f && value != 0x22 && value != 0x5c
  pure s!"b{repr (String.ofList (bytes.toList.map Char.ofNat))}"

private def constText : ConstValue → Except String String
  | .unit => pure "()"
  | .bool true => pure "true"
  | .bool false => pure "false"
  | .integer value => pure (toString value)
  | .address value => pure s!"@{value}"
  | .string value => pure s!"{repr value}"
  | .bytes values => pure s!"b[{commaSep (values.map (toString ·))}]"
  | .tuple values => do pure s!"({commaSep (← values.mapM constText)})"
  | .vector values => match byteVectorText? values with
      | some text => pure text
      | none => do pure s!"#[{commaSep (← values.mapM constText)}]"
  | .character value => pure s!"{repr (Char.ofNat value)}"
  | .profile _ =>
      throw "constant is outside the current LeanerLang literal parser"

/-- Use the source profile's default integer type without a suffix, matching
Move and Rust source. Every other physical integer keeps an explicit suffix so
type inference can reconstruct the LIR type after local annotations are
omitted. Specification integers are mathematical and never carry a suffix. -/
private def integerLiteralText (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (specification : Bool) (typeId : TypeId) (value : Int) : Except String String := do
  let literal := toString value
  if specification then return literal
  let some type := unit.tables.types[typeId.index]?
    | throw s!"integer literal references missing type {typeId.index}"
  let suffix? : Option String ← match type, ns.profile with
    | .integer (.bits 64) false, some .move => pure none
    | .integer (.bits 32) true, some .rust => pure none
    | .integer (.bits width) false, _ => pure (some s!"u{width}")
    | .integer (.bits width) true, _ => pure (some s!"i{width}")
    | .integer .pointer false, _ => pure (some "usize")
    | .integer .pointer true, _ => pure (some "isize")
    | .integer .unbounded _, _ => pure none
    | _, _ => throw "an integer constant has a non-integer expression type"
  pure <| literal ++ suffix?.getD ""

private def failureText : LeanerIR.ThrowKind → Except String String
  | .abort => pure "abort"
  | .panic => pure "panic"
  | .profile value => throw s!"profile throw `{value.tag}` has no core LeanerLang spelling"

private def checkedPrimitiveText (name : String) (failure : LeanerIR.ThrowKind) :
    Except String String := do
  let suffix ← match failure with
    | .abort => pure "Abort"
    | .panic => pure "Panic"
    | .profile value => throw s!"profile throw `{value.tag}` has no core LeanerLang spelling"
  pure s!"{name}{suffix}"

private def primitiveText : PrimitiveOperation → Except String String
  | .tuple => pure "tuple"
  | .vector => pure "vector"
  | .repeatVector => pure "repeatVector"
  | .pushVector => pure "pushVector"
  | .swapVector => pure "swapVector"
  | .length => pure "length"
  | .index => pure "index"
  | .slice => pure "slice"
  | .range => pure "range"
  | .add => pure "add"
  | .checkedAdd failure => checkedPrimitiveText "checkedAdd" failure
  | .subtract => pure "subtract"
  | .checkedSubtract failure => checkedPrimitiveText "checkedSubtract" failure
  | .multiply => pure "multiply"
  | .checkedMultiply failure => checkedPrimitiveText "checkedMultiply" failure
  | .overflowingAdd => pure "overflowingAdd"
  | .overflowingSubtract => pure "overflowingSubtract"
  | .overflowingMultiply => pure "overflowingMultiply"
  | .divide => pure "divide"
  | .checkedDivide failure => checkedPrimitiveText "checkedDivide" failure
  | .modulo => pure "modulo"
  | .checkedModulo failure => checkedPrimitiveText "checkedModulo" failure
  | .bitwiseOr => pure "bitwiseOr"
  | .bitwiseAnd => pure "bitwiseAnd"
  | .bitwiseXor => pure "bitwiseXor"
  | .bitwiseNot => pure "bitwiseNot"
  | .shiftLeft => pure "shiftLeft"
  | .checkedShiftLeft failure => checkedPrimitiveText "checkedShiftLeft" failure
  | .shiftRight => pure "shiftRight"
  | .checkedShiftRight failure => checkedPrimitiveText "checkedShiftRight" failure
  | .logicalAnd => pure "logicalAnd"
  | .logicalOr => pure "logicalOr"
  | .logicalNot => pure "logicalNot"
  | .equal => pure "equal"
  | .notEqual => pure "notEqual"
  | .less => pure "less"
  | .greater => pure "greater"
  | .lessEqual => pure "lessEqual"
  | .greaterEqual => pure "greaterEqual"
  | .negate => pure "negate"
  | .checkedNegate failure => checkedPrimitiveText "checkedNegate" failure
  | .cast => pure "cast"
  | .checkedCast failure => checkedPrimitiveText "checkedCast" failure
  | .implies => pure "implies"
  | .equivalent => pure "equivalent"
  | .identical => pure "identical"
  | .copyValue => pure "copyValue"
  | .moveValue => pure "moveValue"

/-- Control leaves the block at this expression: nothing after it is reachable
and nothing after it has a source spelling. A conditional whose branches all
leave is one too, which its `never` type records. -/
private def isDivergingExpression (ns : ValidatedNamespace) (id : ExprId) : Bool :=
  match ns.expressions[id.index]? with
  | some { kind := .return_ .., .. } | some { kind := .break_ .., .. }
  | some { kind := .continue_ .., .. } | some { kind := .throw_ .., .. } => true
  | some expression => ns.tables.types[expression.typeId.index]? == some .never
  | none => false

/-- A control-flow expression used as an operand needs delimiters: operators,
casts, and indexing all bind tighter than `if`, `match`, or a block. -/
private def parenthesizedOperand (ns : ValidatedNamespace) (id : ExprId)
    (text : String) : String :=
  match ns.expressions[id.index]? with
  | some { kind := .ifElse .., .. } | some { kind := .match_ .., .. }
  | some { kind := .block .., .. } | some { kind := .letDecl .., .. }
  | some { kind := .loop .., .. } | some { kind := .return_ .., .. }
  | some { kind := .break_ .., .. } | some { kind := .continue_ .., .. } =>
      s!"({text})"
  | _ => text

private def specificationVectorName? : SpecOperation → Option String
  | .emptyVector => some "emptyVector"
  | .singletonVector => some "singletonVector"
  | .updateVector => some "updateVector"
  | .concatVector => some "concatVector"
  | .indexOfVector => some "indexOfVector"
  | .containsVector => some "containsVector"
  | .lengthVector => some "lengthVector"
  | .indexVector => some "indexVector"
  | .sliceVector => some "sliceVector"
  | .inVectorRange => some "inVectorRange"
  | .vectorRange => some "vectorRange"
  | _ => none

private def behaviorOperationName : BehaviorKind → String
  | .requiresOf => "requires_of"
  | .abortsOf => "aborts_of"
  | .ensuresOf => "ensures_of"
  | .resultOf => "result_of"
  | .unchangedOf => "unchanged_of"
  | .foldsOf => "folds_of"
  | .writeOf index => s!"write_of[{index}]"

private def behaviorRangePrefix (kind : BehaviorKind) (range : MemoryRange) : String :=
  match range.pre, range.post with
  | none, none => ""
  | some pre, none =>
      if kind == .requiresOf || kind == .abortsOf then s!"@{pre} |~ "
      else s!"@{pre}.. |~ "
  | none, some post => s!"..@{post} |~ "
  | some pre, some post => s!"@{pre}..@{post} |~ "

private partial def patternLocalIds (ns : ValidatedNamespace) (id : PatternId)
    (fuel : Nat) : Array LocalId :=
  if fuel == 0 then #[] else
  match ns.patterns[id.index]? with
  | none => #[]
  | some pattern =>
      match pattern.kind with
      | .variable localId => #[localId]
      | .tuple elements => elements.flatMap (patternLocalIds ns · (fuel - 1))
      | .constructor _ _ _ fields => fields.flatMap (patternLocalIds ns · (fuel - 1))
      | .wildcard | .literal _ | .range .. => #[]

private partial def placeLocal? (ns : ValidatedNamespace) (id : PlaceId) (fuel : Nat) :
    Option LocalId :=
  if fuel == 0 then none else
  match ns.places[id.index]? with
  | some (.localVar localId) => some localId
  | some (.deref base) | some (.field base ..) | some (.index base _) |
    some (.subslice base ..) | some (.downcast base _) => placeLocal? ns base (fuel - 1)
  | none => none

/-- Names a declaration chain binds for the rest of the block that prints it.
The chain stops where the printer stops flattening: a loop or conditional
prints its own indented block, whose declarations stay inside it. -/
private partial def letChainBindings (ns : ValidatedNamespace) (locals : Array LocalDecl)
    (scope : Array (String × LocalId)) (id : ExprId) (fuel : Nat) :
    Array (String × LocalId) :=
  if fuel == 0 then scope else
  match ns.expressions[id.index]? with
  | none => scope
  | some expression =>
      match expression.kind with
      | .letDecl pattern _ body =>
          let scope := (patternLocalIds ns pattern (ns.patterns.size + 1)).foldl
            (fun scope localId =>
              match locals[localId.index]?.map (·.name) with
              | none => scope
              | some name => scope.push (name, localId)) scope
          letChainBindings ns locals scope body (fuel - 1)
      | .block statements result =>
          -- A nested block prints its entries into the same indentation, so
          -- the declarations it contains bind on beyond it as well.
          let scope := statements.foldl (fun scope statement =>
            letChainBindings ns locals scope statement (fuel - 1)) scope
          match result with
          | some result => letChainBindings ns locals scope result (fuel - 1)
          | none => scope
      | _ => scope

/-- Locals a source name cannot reach: a later binding of the same name stands
between the mention and the local it means. LIR identifies a local by id and
lets inlining reuse a name freely, while the surface binds by name, so the
mention would read — or, worse, assign — the shadowing local instead. The
walk follows the printed binding structure and marks every binding that stands
in the way of a mention of an outer local of its own name. -/
private partial def shadowingLocals (ns : ValidatedNamespace) (locals : Array LocalDecl)
    (id : ExprId) (scope : Array (String × LocalId)) (marked : Array LocalId)
    (fuel : Nat) : Array LocalId := Id.run do
  if fuel == 0 then return marked
  let localName? := fun (localId : LocalId) => locals[localId.index]?.map (·.name)
  -- A mention resolves to the innermost binding of its name, so every binding
  -- of that name inside the mentioned local's own binding hides it. A local
  -- the printed structure never binds — the frontend's `let x := x` reuses one
  -- local for both sides — has no binding to be hidden from.
  let mention := fun (marked : Array LocalId) (localId : LocalId) =>
    match localName? localId with
    | none => marked
    | some name =>
        let sameName := scope.filter (·.1 == name)
        -- Bindings after the local's own innermost binding are what hide it;
        -- a frontend that rebinds one local under its own name (`let x := x`)
        -- puts it in scope more than once and hides nothing.
        let hidden := match sameName.reverse.findIdx? (·.2 == localId) with
          | some fromEnd => sameName.extract (sameName.size - fromEnd) sameName.size
          | none => #[]
        hidden.foldl (fun marked entry =>
          if marked.contains entry.2 then marked else marked.push entry.2) marked
  let mut marked := marked
  let bind := fun (scope : Array (String × LocalId)) (pattern : PatternId) =>
    (patternLocalIds ns pattern (ns.patterns.size + 1)).foldl (fun scope localId =>
      match localName? localId with
      | none => scope
      | some name => scope.push (name, localId)) scope
  let rec placeMentions := fun (marked : Array LocalId) (place : PlaceId) =>
    match placeLocal? ns place (ns.places.size + 1) with
    | none => marked
    | some localId => mention marked localId
  let child := fun (marked : Array LocalId) (scope : Array (String × LocalId))
      (child : ExprId) => shadowingLocals ns locals child scope marked (fuel - 1)
  let some expression := ns.expressions[id.index]? | return marked
  match expression.kind with
  | .value .. | .constant .. | .continue_ .. => return marked
  | .localVar localId => return mention marked localId
  | .operation operation _ arguments _ =>
      match operation with
      | .move place | .copy place | .borrow _ place | .read place |
          .write place | .drop place => marked := placeMentions marked place
      | _ => pure ()
      return arguments.foldl (child · scope) marked
  | .block statements result =>
      -- The printer flattens a declaration chain into the block it belongs to,
      -- so a `let` statement binds its name for the rest of that block, not
      -- only for the sub-expression LIR nests under it.
      let mut blockScope := scope
      for statement in statements do
        marked := child marked blockScope statement
        blockScope := letChainBindings ns locals blockScope statement
          (ns.expressions.size + 1)
      return match result with
        | some result => child marked blockScope result
        | none => marked
  | .letDecl pattern value body =>
      if let some value := value then marked := child marked scope value
      return child marked (bind scope pattern) body
  | .ifElse condition thenBranch elseBranch =>
      marked := child marked scope condition
      marked := child marked scope thenBranch
      return match elseBranch with
        | some elseBranch => child marked scope elseBranch
        | none => marked
  | .match_ scrutinee arms =>
      marked := child marked scope scrutinee
      return arms.foldl (fun marked arm =>
        let armScope := bind scope arm.pattern
        let marked := match arm.guard with
          | some guard => child marked armScope guard
          | none => marked
        child marked armScope arm.body) marked
  | .loop _ body => return child marked scope body
  | .break_ _ value =>
      return match value with
        | some value => child marked scope value
        | none => marked
  | .return_ values | .throw_ _ values => return values.foldl (child · scope) marked
  | .assign place value =>
      marked := placeMentions marked place
      return child marked scope value
  | .assignPattern pattern value =>
      marked := (patternLocalIds ns pattern (ns.patterns.size + 1)).foldl mention marked
      return child marked scope value
  | .quantifier _ binders triggers condition body =>
      let mut binderScope := scope
      for binder in binders do
        marked := child marked binderScope binder.domain
        binderScope := bind binderScope binder.pattern
      for trigger in triggers do
        marked := trigger.foldl (child · binderScope) marked
      if let some condition := condition then
        marked := child marked binderScope condition
      return child marked binderScope body
  | .spec block =>
      for condition in block.conditions do
        marked := child marked scope condition.expression
        marked := condition.auxiliary.foldl (fun marked entry =>
          child marked scope entry.2) marked
      if let some frame := block.frame then
        marked := frame.modifies.foldl (child · scope) marked
      return marked

/-- Source names for one declaration's locals, keyed by local index. A binding
that hides a local the source still mentions is primed, which is also how the
Move printer that preceded this one spelled a shadowing binding; every other
local keeps the name its declaration carries. -/
private def sourceLocalNames (locals : Array LocalDecl) (shadowing : Array LocalId) :
    Array String := Id.run do
  let mut names : Array String := #[]
  for declaration in locals do
    let mut name := declaration.name
    if shadowing.contains declaration.id then
      while names.contains name || locals.any (·.name == name) do
        name := name ++ "'"
    names := names.push name
  return names

private def functionBodyRoot? : FunctionBody → Option ExprId
  | .structured root => some root
  | .absent => none

/-- Local source names for a declaration whose body is `root`. Parameters are
the outermost bindings; every other local is bound by the body itself. -/
private def declarationLocalNames (ns : ValidatedNamespace) (locals : Array LocalDecl)
    (parameters : Nat) (root : Option ExprId) : Array String :=
  match root with
  | none => locals.map (·.name)
  | some root =>
      let scope := (locals.extract 0 parameters).map fun declaration =>
        (declaration.name, declaration.id)
      sourceLocalNames locals
        (shadowingLocals ns locals root scope #[] (ns.expressions.size + 1))

private structure Context where
  unit : ValidatedUnit
  ns : ValidatedNamespace
  locals : Array LocalDecl
  /-- Source spelling of each local, disambiguating shadowed mutable locals.
  Empty falls back to the declared names. -/
  localNames : Array String := #[]
  binders : Array LeanerIR.GenericBinder := #[]
  nominalOwner : Option NameId := none
  logicalLocals : Array LocalId := #[]
  specification : Bool := false
  constantLocals : Array (LocalId × String) := #[]
  temporaryLocals : Array LocalId := #[]
  /-- Iterator whose canonical `for` body is currently being rendered.  It
  lets the source backend hide the increment inserted before a current-level
  `continue` by the surface lowerer. -/
  forIterator : Option LocalId := none
  /-- Bindings of the declaration being printed whose mutability the pattern
  itself must carry, as in `let (mut head, tail) := ...`. -/
  mutableBindings : Array LocalId := #[]

/-- Source specification contexts expose direct references by value and use
mathematical integers at direct value boundaries. Mirror the frontend's
projection when a semantic result type must be written explicitly. -/
private partial def contextTypeTextFuel (context : Context) (id : TypeId)
    (fuel : Nat) : Except String String := do
  if !context.specification then
    return ← typeTextFuel context.unit context.ns context.binders id fuel
  if fuel == 0 then throw "cyclic specification type reached the LeanerLang source backend"
  let some type := context.unit.tables.types[id.index]?
    | throw s!"LeanerLang source references missing type {id.index}"
  match type with
  | .integer .. => pure "Int"
  | .reference reference => contextTypeTextFuel context reference.referent (fuel - 1)
  | .tuple elements =>
      if elements.isEmpty then pure "Unit" else
        pure s!"({commaSep (← elements.mapM
          (contextTypeTextFuel context · (fuel - 1)))})"
  | .function arguments result abilities =>
      let arguments ← arguments.mapM (contextTypeTextFuel context · (fuel - 1))
      let result ← contextTypeTextFuel context result (fuel - 1)
      pure s!"Fn({commaSep arguments}) -> {result}{abilitiesText abilities}"
  | _ => typeTextFuel context.unit context.ns context.binders id fuel

private def contextTypeText (context : Context) (id : TypeId) : Except String String :=
  contextTypeTextFuel context id (context.unit.tables.types.size + 1)

/-- Rust spells loads of intrinsically `Copy` scalar places as ordinary
expressions. Keep the ownership operation in LIR, but do not expose it in the
source-like backend. Nominal and aggregate copyability is deliberately not
guessed here; those values retain explicit `copy` until checked ability facts
are available to the printer. -/
private def rustImplicitCopyType (context : Context) (id : TypeId) : Bool :=
  match context.unit.tables.types[id.index]? with
  | some .unit | some .bool | some .character | some .string | some .bytes |
      some .address | some (.integer ..) => true
  | _ => false

private def operatorInfo? (context : Context) (primitive : PrimitiveOperation) :
    Option LeanerLang.OperatorInfo :=
  let operation : Option LeanerLang.CoreOperator := match primitive with
    | .add => if context.specification || context.ns.profile == some .rust then
        some LeanerLang.CoreOperator.add else none
    | .checkedAdd .abort => if context.ns.profile == some .move then
        some LeanerLang.CoreOperator.add else none
    | .subtract => if context.specification || context.ns.profile == some .rust then
        some LeanerLang.CoreOperator.subtract else none
    | .checkedSubtract .abort =>
        if context.ns.profile == some .move then
          some LeanerLang.CoreOperator.subtract else none
    | .multiply => if context.specification || context.ns.profile == some .rust then
        some LeanerLang.CoreOperator.multiply else none
    | .checkedMultiply .abort =>
        if context.ns.profile == some .move then
          some LeanerLang.CoreOperator.multiply else none
    | .divide => if context.specification || context.ns.profile == some .rust then
        some LeanerLang.CoreOperator.divide else none
    | .checkedDivide .abort =>
        if context.ns.profile == some .move then
          some LeanerLang.CoreOperator.divide else none
    | .modulo => if context.specification || context.ns.profile == some .rust then
        some LeanerLang.CoreOperator.modulo else none
    | .checkedModulo .abort =>
        if context.ns.profile == some .move then
          some LeanerLang.CoreOperator.modulo else none
    | .shiftLeft => if context.specification || context.ns.profile == some .rust then
        some LeanerLang.CoreOperator.shiftLeft else none
    | .checkedShiftLeft .abort =>
        if context.ns.profile == some .move then
          some LeanerLang.CoreOperator.shiftLeft else none
    | .shiftRight => if context.specification || context.ns.profile == some .rust then
        some LeanerLang.CoreOperator.shiftRight else none
    | .checkedShiftRight .abort =>
        if context.ns.profile == some .move then
          some LeanerLang.CoreOperator.shiftRight else none
    | .negate => if context.specification || context.ns.profile == some .rust then
        some LeanerLang.CoreOperator.negate else none
    | .checkedNegate .abort =>
        if context.ns.profile == some .move then
          some LeanerLang.CoreOperator.negate else none
    | other => LeanerLang.Operators.ofPrimitiveOperation other
  operation.bind LeanerLang.Operators.info?

/-- Move compiler v2 represents the specification companion of an executable
function `f` as `$f`.  LeanerLang deliberately derives that companion in a
specification context by calling `f`, so the compiler-private name must never
escape into canonical source.  Preserve qualification because companions of
curated modules such as `std::vector` are external to the printed namespace. -/
private def specificationCallName (context : Context) (reference : QualifiedRef) :
    Except String (String × Bool) := do
  let some qualified := context.unit.tables.names[reference.name.index]?
    | throw s!"LeanerLang source references missing name {reference.name.index}"
  unless qualified.namespaceId == reference.namespaceId do
    throw "specification-function reference has a mismatched namespace"
  let moveCompanion := context.ns.profile == some .move && qualified.name.startsWith "$"
  let localName := if moveCompanion then qualified.name.drop 1 |>.toString
    else qualified.name
  let localName := surfaceReferenceName localName
  unless !localName.isEmpty do
    throw "a Move specification companion has an empty executable-function name"
  let localName := sourceIdentifier localName
  let imported ← if reference.namespaceId == context.ns.identity then pure false else do
    let expected := s!"{← namespacePath context.unit reference.namespaceId}::{localName}"
    pure ((← inferredUsePaths context.unit context.ns.identity).contains expected)
  let name ← if reference.namespaceId == context.ns.identity then pure localName
    else if imported then pure localName
    else pure s!"{← namespacePath context.unit reference.namespaceId}::{localName}"
  pure (name, moveCompanion)

/-- Aborting specification branches have an unspecified value in LIR.  The
lowerer represents each such site with a private opaque specification
function; recover the source-level `abort()` spelling instead of exposing its
generated name. -/
private def isArbitrarySpecificationCall (context : Context)
    (reference : QualifiedRef) : Bool :=
  reference.namespaceId == context.ns.identity &&
    (context.unit.tables.names[reference.name.index]?.map fun qualified =>
      qualified.namespaceId == reference.namespaceId &&
        qualified.name.startsWith "__leaner_arbitrary_").getD false &&
    context.ns.specFunctions.any fun declaration =>
      declaration.name == reference.name && declaration.body.isNone &&
        declaration.signature.parameters.isEmpty

private def localName (context : Context) (id : LocalId) : Except String String := do
  let some localDecl := context.locals[id.index]?
    | throw s!"LeanerLang source references missing local {id.index}"
  pure (sourceIdentifier (context.localNames[id.index]?.getD localDecl.name))

private def localValueText (context : Context) (id : LocalId) : Except String String := do
  if let some (_, value) := context.constantLocals.find? (·.1 == id) then
    pure value
  else
    let some declaration := context.locals[id.index]?
      | throw s!"LeanerLang source references missing local {id.index}"
    if declaration.name == "_0" &&
        context.unit.tables.types[declaration.type.typeId.index]? == some .unit then
      pure "()"
    else localName context id

private def numericTemporaryLocal? (context : Context) (id : LocalId) : Bool :=
  match context.locals[id.index]?.map (·.name.toList) with
  | some ('_' :: digits) =>
      !digits.isEmpty && digits.all fun digit => '0' ≤ digit && digit ≤ '9'
  | _ => false

private def compilerTemporaryLocal? (context : Context) (id : LocalId) : Bool :=
  context.temporaryLocals.contains id || numericTemporaryLocal? context id

/-- Does the operand of a field selection need delimiters? Field access binds
tighter than every control-flow form, and Move prints a selection through a
reference without its dereference, so the operand whose spelling the selection
follows is the referent, not the dereference around it. -/
private def selectionNeedsParentheses (context : Context) (value : LeanerIR.Expr) : Bool :=
  let operand := match value.kind with
    | .operation (.reference .dereference) #[] #[referent] _ =>
        if context.ns.profile == some Profile.move then
          (context.ns.expressions[referent.index]?).getD value
        else value
    | _ => value
  match operand.kind with
  | .block .. | .letDecl .. | .ifElse .. | .match_ .. | .loop .. |
      .break_ .. | .continue_ .. | .return_ .. | .throw_ .. | .assign .. |
      .assignPattern .. | .quantifier .. | .spec .. => true
  | .operation (.primitive operation) _ _ _ =>
      (operatorInfo? context operation).isSome
  | .operation (.reference .dereference) _ _ _ =>
      context.ns.profile != some Profile.move
  | _ => false

/-- A compiler temporary subsequently used as the base of a place projection
must remain storage. Substituting its value textually would turn, for example,
`copy(*temporary)` into the invalid place `copy(*copy(value.field))`.
Constants and temporaries consumed only as values remain safe to unfold. -/
private def requiresTemporaryStorage (context : Context) (localId : LocalId) : Bool :=
  context.ns.places.any fun place =>
    let base? := match place with
      | .deref base | .field base .. | .index base _ | .subslice base .. |
          .downcast base _ => some base
      | .localVar _ => none
    base?.any fun base => context.ns.places[base.index]? == some (.localVar localId)

private def temporaryConstant? (context : Context) (pattern : PatternId)
    (value : ExprId) : Except String (Option (LocalId × String)) := do
  let some patternNode := context.ns.patterns[pattern.index]? | return none
  let .variable localId := patternNode.kind | return none
  let some localDecl := context.locals[localId.index]? | return none
  unless localDecl.name.startsWith "$" do return none
  let some valueNode := context.ns.expressions[value.index]? | return none
  match valueNode.kind with
  | .value constant _ =>
      let text ← match constant with
        | .integer value =>
            integerLiteralText context.unit context.ns context.specification valueNode.typeId value
        | _ => constText constant
      pure (some (localId, text))
  | kind =>
      let sourceLocal? := match kind with
        | .localVar source => some source
        | .operation operation _ _ _ =>
            let place? := match operation with
              | .move place | .copy place | .read place => some place
              | _ => none
            place?.bind fun place => do
              let .localVar source ← context.ns.places[place.index]? | none
              some source
        | _ => none
      let some source := sourceLocal? | return none
      let some sourceDecl := context.locals[source.index]? | return none
      if sourceDecl.mutable then return none
      let scalar := match context.unit.tables.types[valueNode.typeId.index]? with
        | some .unit | some .bool | some .character | some .address |
            some .signer | some (.integer ..) => true
        | _ => false
      unless scalar do return none
      pure (some (localId, ← localValueText context source))

/-- A hidden holder bound to a storage borrow.  A field-focused mutable
borrow of a resource reborrows through such a holder, because a place is
rooted at a local; the surface spells the projection through the resource
itself, so the holder's borrow text substitutes for its name and the `*&`
pair cancels inside the place. -/
private def hiddenStorageBorrow? (context : Context) (pattern : PatternId)
    (value : ExprId) : Option LocalId := do
  let patternNode ← context.ns.patterns[pattern.index]?
  let .variable localId := patternNode.kind | none
  let localDecl ← context.locals[localId.index]?
  if !localDecl.name.startsWith "$" then none else
  let valueNode ← context.ns.expressions[value.index]?
  let .operation (.global (.borrow _)) _ _ _ := valueNode.kind | none
  some localId

/-- The referent a borrow's printed text names, when the text is a borrow.
Cancelling a source-level `*&` pair keeps compiler scaffolding out of the
generated source and, in a place, keeps the target a place. -/
private def borrowedReferentText? (value : String) : Option String :=
  if value.startsWith "(&mut " && value.endsWith ")" then
    some ((value.drop 6).dropEnd 1).toString
  else if value.startsWith "(&" && value.endsWith ")" then
    some ((value.drop 2).dropEnd 1).toString
  else if value.startsWith "&mut " then some (value.drop 5).toString
  else if value.startsWith "&" then some (value.drop 1).toString
  else none

private partial def placeText (context : Context)
    (indexText : ExprId → Except String String) (id : PlaceId)
    (fuel : Nat := 0) : Except String String := do
  let fuel := if fuel == 0 then context.ns.places.size + 1 else fuel
  if fuel == 0 then throw "cyclic place reached the LeanerLang source backend"
  let some place := context.ns.places[id.index]?
    | throw s!"LeanerLang source references missing place {id.index}"
  match place with
  | .localVar localId => localValueText context localId
  | .deref base =>
      let baseText ← placeText context indexText base (fuel - 1)
      if context.specification then return baseText
      -- MIR commonly stores a borrow in a compiler temporary and immediately
      -- dereferences it.  Once the temporary is unfolded, cancel the
      -- source-level `*&` pair instead of exposing compiler scaffolding.
      if let some referent := borrowedReferentText? baseText then return referent
      let baseText := match context.ns.places[base.index]? with
        | some (.localVar _) => baseText
        | _ => s!"({baseText})"
      pure s!"*{baseText}"
  | .field base _ field =>
      -- Move reads a field through a reference implicitly, so the dereference
      -- has no spelling; a borrow underneath it cancels with it, which is what
      -- keeps the result a place rather than a borrow expression.
      let baseText ← match context.ns.profile, context.ns.places[base.index]? with
        | some .move, some (.deref reference) =>
            placeText context indexText reference (fuel - 1)
        | _, _ => placeText context indexText base (fuel - 1)
      -- An unfolded compiler temporary can put a borrow where the base of the
      -- projection stands. Move spells a field of a borrowed value without the
      -- borrow, and only that spelling is a place.
      let baseText := if context.ns.profile == some .move then
          (borrowedReferentText? baseText).getD baseText
        else baseText
      let baseText := match context.ns.profile, context.ns.places[base.index]? with
        | some .move, some (.deref _) => baseText
        | _, some (.deref _) => s!"({baseText})"
        | _, _ => baseText
      pure s!"{baseText}.{← nameAt context.unit field}"
  | .index base index =>
      let baseText ← match context.ns.places[base.index]? with
        | some (.deref reference) => placeText context indexText reference (fuel - 1)
        | _ => placeText context indexText base (fuel - 1)
      let baseText := if context.ns.profile == some .move then
          (borrowedReferentText? baseText).getD baseText
        else baseText
      pure s!"{baseText}[{← indexText index}]"
  | .downcast base _ =>
      -- A downcast only refines which enum payload is active. Structured MIR
      -- already guards the corresponding arm by the discriminant, while the
      -- field name carries the source-visible projection.
      placeText context indexText base (fuel - 1)
  | .subslice base start stop fromEnd =>
      let baseText ← placeText context indexText base (fuel - 1)
      let receiverText ← match context.ns.places[base.index]? with
        | some (.deref reference) => placeText context indexText reference (fuel - 1)
        | _ => pure baseText
      let baseText := match context.ns.places[base.index]? with
        | some (.localVar _) => baseText
        | _ => s!"({baseText})"
      let lower := s!"{start}usize"
      let upper := if fromEnd then
          if stop == 0 then s!"{receiverText}.length"
          else s!"{receiverText}.length - {stop}usize"
        else s!"{stop}usize"
      pure s!"core.prim.slice({baseText}, {lower}, {upper})"

private def cancelBorrowedPlaceText (value : String) : String :=
  if value.startsWith "*&mut " then value.drop 6 |>.toString
  else if value.startsWith "*&" then value.drop 2 |>.toString
  else value

/-- Does this expression select a field through a reference, possibly after a
freeze? Move reads such a selection implicitly, so its dereference and the
freeze have no spelling of their own. -/
private partial def selectsFieldThroughReference (context : Context) (id : ExprId) : Bool :=
  match context.ns.expressions[id.index]? with
  | some { kind := .operation (.data (.select ..)) _ _ _, .. }
  | some { kind := .operation (.data (.selectVariants ..)) _ _ _, .. } => true
  | some { kind := .operation (.reference (.freeze _)) _ #[operand] _, .. } =>
      selectsFieldThroughReference context operand
  | _ => false

private def implicitDerefReceiverText (value : String) : String :=
  if value.startsWith "*(" && value.endsWith ")" then
    (value.drop 2).dropEnd 1 |>.toString
  else if value.startsWith "*" then value.drop 1 |>.toString
  else value

private def localPlace? (ns : ValidatedNamespace) (id : PlaceId) : Option LocalId := do
  let .localVar localId ← ns.places[id.index]? | none
  pure localId

/-- The reborrow `&mut *local` of a local that holds a mutable reference:
how validation spells a mutable reference copied into a call, which the
source spells as the bare local. -/
private def reborrowedLocal? (context : Context) (id : ExprId) : Option LocalId := do
  let node ← context.ns.expressions[id.index]?
  let .operation (.borrow .mutable place) _ _ _ := node.kind | none
  let .deref base ← context.ns.places[place.index]? | none
  let .localVar localId ← context.ns.places[base.index]? | none
  let declaration ← context.locals[localId.index]?
  let .reference reference ← context.unit.tables.types[declaration.type.typeId.index]? | none
  if reference.kind == .mutable then some localId else none

/-- Recover the declaration-local read by the two equivalent local-access
forms admitted by LIR. Compiler-v2 commonly uses a place load where authored
LeanerLang uses a local expression, including for quantified specification
variables. -/
private def accessedLocal? (ns : ValidatedNamespace) (expression : LeanerIR.Expr) :
    Option LocalId :=
  match expression.kind with
  | .localVar localId => some localId
  | .operation (.move place) _ _ _ => localPlace? ns place
  | .operation (.copy place) _ _ _ => localPlace? ns place
  | .operation (.read place) _ _ _ => localPlace? ns place
  | _ => none

private def variablePatternLocal? (ns : ValidatedNamespace)
    (pattern : PatternId) : Option LocalId := do
  let pattern ← ns.patterns[pattern.index]?
  let .variable localId := pattern.kind | none
  pure localId

private structure ForRangeShape where
  iterator : LocalId
  lower : ExprId
  upper : ExprId
  bodyStatements : Array ExprId

private def incrementsLocalByOne? (ns : ValidatedNamespace) (iterator : LocalId)
    (id : ExprId) : Bool :=
  (do
    let incrementNode ← ns.expressions[id.index]?
    let .assignPattern incrementPattern incrementValue := incrementNode.kind | none
    let incrementLocal ← variablePatternLocal? ns incrementPattern
    guard (incrementLocal == iterator)
    let incrementValueNode ← ns.expressions[incrementValue.index]?
    let .operation (.primitive operation) _ #[incrementRead, oneId] _ :=
      incrementValueNode.kind | none
    guard (operation == PrimitiveOperation.add ||
      operation == PrimitiveOperation.checkedAdd .abort)
    let incrementReadNode ← ns.expressions[incrementRead.index]?
    let incrementSource ← accessedLocal? ns incrementReadNode
    guard (incrementSource == iterator)
    let oneNode ← ns.expressions[oneId.index]?
    let .value (.integer 1) _ := oneNode.kind | none
    pure true).getD false

private def incrementedContinue? (ns : ValidatedNamespace) (iterator : LocalId)
    (id : ExprId) : Bool :=
  (do
    let node ← ns.expressions[id.index]?
    let .block #[increment] (some continueId) := node.kind | none
    guard (incrementsLocalByOne? ns iterator increment)
    let continueNode ← ns.expressions[continueId.index]?
    let .continue_ 0 := continueNode.kind | none
    pure true).getD false

/-- Recognize the stable compiler-v2 expansion of a half-open range loop:
`let $lb = lower; let i = $lb; let $ub = upper; while i < $ub { body; i += 1 }`.
The hidden bound locals ensure both bounds retain their one-time evaluation
semantics when the canonical surface form is recovered. -/
private def forRangeShape? (context : Context) (root : ExprId) : Option ForRangeShape := do
  let rootNode ← context.ns.expressions[root.index]?
  let .letDecl lowerPattern (some lower) iteratorBinding := rootNode.kind | none
  let lowerLocal ← variablePatternLocal? context.ns lowerPattern
  let lowerDecl ← context.locals[lowerLocal.index]?
  if !lowerDecl.name.startsWith "$" then none else
  let iteratorBindingNode ← context.ns.expressions[iteratorBinding.index]?
  let .letDecl iteratorPattern (some iteratorStart) upperBinding :=
    iteratorBindingNode.kind | none
  let iterator ← variablePatternLocal? context.ns iteratorPattern
  if iterator == lowerLocal then none else
  let iteratorStartNode ← context.ns.expressions[iteratorStart.index]?
  let some startLocal := accessedLocal? context.ns iteratorStartNode | none
  if startLocal != lowerLocal then none else
  let upperBindingNode ← context.ns.expressions[upperBinding.index]?
  let .letDecl upperPattern (some upper) loopId := upperBindingNode.kind | none
  let upperLocal ← variablePatternLocal? context.ns upperPattern
  let upperDecl ← context.locals[upperLocal.index]?
  if !upperDecl.name.startsWith "$" || upperLocal == iterator || upperLocal == lowerLocal then
    none
  else
  let loopNode ← context.ns.expressions[loopId.index]?
  let .loop none guardedId := loopNode.kind | none
  let guardedNode ← context.ns.expressions[guardedId.index]?
  let .ifElse conditionId iterationId (some stopId) := guardedNode.kind | none
  let stopNode ← context.ns.expressions[stopId.index]?
  let .break_ 0 none := stopNode.kind | none
  let conditionNode ← context.ns.expressions[conditionId.index]?
  let .operation (.primitive .less) _ #[iteratorRead, upperRead] _ :=
    conditionNode.kind | none
  let iteratorReadNode ← context.ns.expressions[iteratorRead.index]?
  let upperReadNode ← context.ns.expressions[upperRead.index]?
  let some conditionIterator := accessedLocal? context.ns iteratorReadNode | none
  let some conditionUpper := accessedLocal? context.ns upperReadNode | none
  if conditionIterator != iterator || conditionUpper != upperLocal then none else
  let iterationNode ← context.ns.expressions[iterationId.index]?
  let .block bodyStatements (some incrementId) := iterationNode.kind | none
  guard (incrementsLocalByOne? context.ns iterator incrementId)
  pure { iterator, lower, upper, bodyStatements }

private def loadedPlace? (ns : ValidatedNamespace) (id : ExprId) : Option PlaceId := do
  let expression ← ns.expressions[id.index]?
  let .operation operation _ _ _ := expression.kind | none
  match operation with
  | .move place | .copy place | .read place => some place
  | _ => none

private def reversedComparison? : PrimitiveOperation → Option PrimitiveOperation
  | .less => some .greater
  | .lessEqual => some .greaterEqual
  | .greater => some .less
  | .greaterEqual => some .lessEqual
  | _ => none

/-- Follow structurization-only blocks to the local read by a return. -/
private partial def returnedLocal? (ns : ValidatedNamespace) (id : ExprId)
    (fuel : Nat) : Option LocalId := do
  if fuel == 0 then none else
  let expression ← ns.expressions[id.index]?
  match expression.kind with
  | .return_ #[returned] => localPlace? ns (← loadedPlace? ns returned)
  | .block _ (some result) => returnedLocal? ns result (fuel - 1)
  | .letDecl _ _ body => returnedLocal? ns body (fuel - 1)
  | _ => none

private def forwardedValue? (ns : ValidatedNamespace) (statements : Array ExprId)
    (result : Option ExprId) : Option ExprId := do
  let [statementId] := statements.toList | none
  let statement ← ns.expressions[statementId.index]?
  let .assign target value := statement.kind | none
  let resultId ← result
  let result ← ns.expressions[resultId.index]?
  let .return_ values := result.kind | none
  let [returned] := values.toList | none
  let returnedPlace ← loadedPlace? ns returned
  let returnedLocal ← localPlace? ns returnedPlace
  let targetLocal ← localPlace? ns target
  if returnedLocal == targetLocal then some value else none

/-- One rustc checked-arithmetic operation is represented in optimized MIR as
an overflowing tuple, a test of tuple field `1`, and a panic branch returning
tuple field `0` on success.  Keep that administrative shape out of canonical
LeanerLang while retaining its overflow-as-panic semantics. -/
private structure CheckedArithmeticShape where
  operation : PrimitiveOperation
  arguments : Array ExprId
  priorAssignments : Array (LocalId × ExprId)

private def loadedIndexedLocal? (ns : ValidatedNamespace) (id : ExprId) :
    Option (LocalId × Int) := do
  let place ← loadedPlace? ns id
  let .index base index ← ns.places[place.index]? | none
  let .localVar localId ← ns.places[base.index]? | none
  let indexExpr ← ns.expressions[index.index]?
  let .value (.integer value) _ := indexExpr.kind | none
  pure (localId, value)

private partial def returnedValue? (ns : ValidatedNamespace) (id : ExprId)
    (fuel : Nat) : Option ExprId := do
  if fuel == 0 then none else
  let expression ← ns.expressions[id.index]?
  match expression.kind with
  | .return_ #[value] => some value
  | .block statements result =>
      (forwardedValue? ns statements result).orElse fun _ => do
        guard statements.isEmpty
        returnedValue? ns (← result) (fuel - 1)
  | .letDecl _ _ body => returnedValue? ns body (fuel - 1)
  | _ => none

private def checkedArithmeticShape? (context : Context) (statements : Array ExprId)
    (result : Option ExprId) : Option CheckedArithmeticShape := do
  guard (context.ns.profile == some .rust)
  let resultExpr ← context.ns.expressions[(← result).index]?
  let .ifElse condition failure (some success) := resultExpr.kind | none
  let failureExpr ← context.ns.expressions[failure.index]?
  let .throw_ .panic #[] := failureExpr.kind | none
  let (overflowLocal, overflowIndex) ← loadedIndexedLocal? context.ns condition
  guard (overflowIndex == 1)
  let successValue ← returnedValue? context.ns success (context.ns.expressions.size + 1)
  let (valueLocal, valueIndex) ← loadedIndexedLocal? context.ns successValue
  guard (valueLocal == overflowLocal && valueIndex == 0)
  let assignmentId ← statements.back?
  let assignment ← context.ns.expressions[assignmentId.index]?
  let .assign target value := assignment.kind | none
  guard (localPlace? context.ns target == some overflowLocal)
  let valueExpr ← context.ns.expressions[value.index]?
  let .operation (.primitive overflowing) #[] arguments _ := valueExpr.kind | none
  let operation ← match overflowing with
    | .overflowingAdd => some (.checkedAdd .panic)
    | .overflowingSubtract => some (.checkedSubtract .panic)
    | .overflowingMultiply => some (.checkedMultiply .panic)
    | _ => none
  let preceding := statements.extract 0 (statements.size - 1)
  let priorAssignments ← preceding.mapM fun statement => do
    let statementExpr ← context.ns.expressions[statement.index]?
    let .assign priorTarget priorValue := statementExpr.kind | none
    let priorLocal ← localPlace? context.ns priorTarget
    guard (compilerTemporaryLocal? context priorLocal)
    pure (priorLocal, priorValue)
  pure { operation, arguments, priorAssignments }

mutual
  private partial def assignedValueToLocal? (ns : ValidatedNamespace) (node : ExprId)
      (targetLocal : LocalId) (fuel : Nat) : Option ExprId := do
    if fuel == 0 then none else
    let expression ← ns.expressions[node.index]?
    match expression.kind with
    | .assign target value =>
        if localPlace? ns target == some targetLocal then some value else none
    | .block statements result =>
        (result.bind fun result =>
          assignedValueToLocal? ns result targetLocal (fuel - 1)).orElse fun _ =>
            assignedValueInStatements? ns statements targetLocal (fuel - 1)
    | _ => none

  /-- Find the last assignment to a local within a sequential region, then
  chase an SSA-style compiler temporary assigned earlier in that same region.
  This reconstructs enum payload reads before a forwarded MIR return. -/
  private partial def assignedValueInStatements? (ns : ValidatedNamespace)
      (statements : Array ExprId) (targetLocal : LocalId) (fuel : Nat) : Option ExprId := do
    if fuel == 0 then none else
    let value ← statements.reverse.findSome? fun statement =>
      assignedValueToLocal? ns statement targetLocal (fuel - 1)
    match (loadedPlace? ns value).bind (localPlace? ns) with
    | none => some value
    | some sourceLocal =>
        if sourceLocal == targetLocal then some value else
          (assignedValueInStatements? ns statements sourceLocal (fuel - 1)).orElse
            fun _ => some value
end

/-- Whether a structured region assigns the given local on at least one path.
This is deliberately weaker than `assignedValueToLocal?`: guarded MIR arms can
assign the return place through a nested conditional, in which case retaining
the structured arm is preferable to inventing another source temporary. -/
private partial def writesLocal? (ns : ValidatedNamespace) (node : ExprId)
    (targetLocal : LocalId) (fuel : Nat) : Bool :=
  if fuel == 0 then false else
  match ns.expressions[node.index]? with
  | none => false
  | some expression => match expression.kind with
    | .assign target _ => localPlace? ns target == some targetLocal
    | .block statements result =>
        statements.any (writesLocal? ns · targetLocal (fuel - 1)) ||
          result.any (writesLocal? ns · targetLocal (fuel - 1))
    | .letDecl _ _ body => writesLocal? ns body targetLocal (fuel - 1)
    | .ifElse _ thenBranch elseBranch =>
        writesLocal? ns thenBranch targetLocal (fuel - 1) ||
          elseBranch.any (writesLocal? ns · targetLocal (fuel - 1))
    | .match_ _ arms =>
        arms.any fun arm => writesLocal? ns arm.body targetLocal (fuel - 1)
    | .loop _ body => writesLocal? ns body targetLocal (fuel - 1)
    | _ => false

/-- Collect assignments which precede the assignment of `targetLocal` in a
sequential structured region.  The source backend uses these to inline MIR
projection temporaries while recovering a value-producing conditional. -/
private partial def assignmentsBeforeLocal? (ns : ValidatedNamespace)
    (node : ExprId) (targetLocal : LocalId) (fuel : Nat) :
    Option (Array (LocalId × ExprId)) := do
  if fuel == 0 then none else
  let expression ← ns.expressions[node.index]?
  match expression.kind with
  | .assign target _ =>
      if localPlace? ns target == some targetLocal then some #[] else none
  | .block statements result =>
      let rec scan (remaining : List ExprId) (prior : Array (LocalId × ExprId)) := do
        match remaining with
        | [] =>
            let result ← result
            let nested ← assignmentsBeforeLocal? ns result targetLocal (fuel - 1)
            pure (prior ++ nested)
        | statement :: rest =>
            if let some nested := assignmentsBeforeLocal? ns statement targetLocal (fuel - 1) then
              pure (prior ++ nested)
            else
              let prior := match ns.expressions[statement.index]? with
                | some { kind := .assign target value, .. } =>
                    match localPlace? ns target with
                    | some localId => prior.push (localId, value)
                    | none => prior
                | _ => prior
              scan rest prior
      scan statements.toList #[]
  | .letDecl _ _ body => assignmentsBeforeLocal? ns body targetLocal (fuel - 1)
  | _ => none

/-- rustc's return place is assigned in both sides of a branch before the
continuation returns it.  Recover the source conditional instead of leaking
the undeclared MIR return local into LeanerLang. -/
private def forwardedConditional? (ns : ValidatedNamespace)
    (statements : Array ExprId) (result : Option ExprId) :
    Option (ExprId × ExprId × Array (LocalId × ExprId) ×
      ExprId × Array (LocalId × ExprId)) := do
  let [statementId] := statements.toList | none
  let statement ← ns.expressions[statementId.index]?
  let .ifElse condition thenBranch (some elseBranch) := statement.kind | none
  let resultId ← result
  let result ← ns.expressions[resultId.index]?
  let .return_ #[returned] := result.kind | none
  let returnedPlace ← loadedPlace? ns returned
  let returnedLocal ← localPlace? ns returnedPlace
  let fuel := ns.expressions.size + 1
  let thenValue ← assignedValueToLocal? ns thenBranch returnedLocal fuel
  let elseValue ← assignedValueToLocal? ns elseBranch returnedLocal fuel
  let thenAssignments := (assignmentsBeforeLocal? ns thenBranch returnedLocal fuel).getD #[]
  let elseAssignments := (assignmentsBeforeLocal? ns elseBranch returnedLocal fuel).getD #[]
  pure (condition, thenValue, thenAssignments, elseValue, elseAssignments)

/-- rustc lowers a value-producing `match` to a unit-valued switch whose arms
assign the MIR return place, followed by a read of that place. Recover the
value-producing source match so compiler-private `_0` never escapes. -/
private def forwardedMatch? (ns : ValidatedNamespace)
    (statements : Array ExprId) (result : Option ExprId) :
    Option (ExprId × Array (MatchArm × ExprId)) := do
  let [statementId] := statements.toList | none
  let statement ← ns.expressions[statementId.index]?
  let .match_ scrutinee arms := statement.kind | none
  let resultId ← result
  let fuel := ns.expressions.size + 1
  let returnedLocal ← returnedLocal? ns resultId fuel
  -- A switch arm may fall through to a shared MIR continuation which assigns
  -- the return place.  That continuation is the value of every arm which
  -- does not assign the place itself (notably a wildcard/default arm).
  let continuationValue? := assignedValueToLocal? ns resultId returnedLocal fuel
  let arms ← arms.mapM fun arm => do
    let value ← (assignedValueToLocal? ns arm.body returnedLocal fuel).orElse fun _ =>
      if writesLocal? ns arm.body returnedLocal fuel then some arm.body
      else continuationValue?
    pure (arm, value)
  pure (scrutinee, arms)

/-- rustc lowers a source `while condition { body }` whose continuation
returns a value as an infinite natural loop: the false edge assigns the return
place and returns from inside the loop, while the true edge ends in
`continue`. Recover the source-shaped condition, body, and trailing return.
The optional leading assignment is rustc's Boolean switch temporary. -/
private def returningWhile? (ns : ValidatedNamespace) (id : ExprId) :
    Option (ExprId × ExprId × ExprId) := do
  let loopNode ← ns.expressions[id.index]?
  let .loop none loopBody := loopNode.kind | none
  let bodyNode ← ns.expressions[loopBody.index]?
  let .block setupStatements (some branchId) := bodyNode.kind | none
  let branch ← ns.expressions[branchId.index]?
  let .ifElse rawCondition thenBranch (some elseBranch) := branch.kind | none
  let condition ← match setupStatements with
    | #[] => some rawCondition
    | #[statementId] => do
        let statement ← ns.expressions[statementId.index]?
        let .assign target value := statement.kind | none
        let targetLocal ← localPlace? ns target
        let conditionPlace ← loadedPlace? ns rawCondition
        let conditionLocal ← localPlace? ns conditionPlace
        guard (targetLocal == conditionLocal)
        some value
    | _ => none
  let elseNode ← ns.expressions[elseBranch.index]?
  let .block statements result := elseNode.kind | none
  let returned ← forwardedValue? ns statements result
  pure (condition, thenBranch, returned)

private def inferredPrimitiveArgument (context : Context) (expression : LeanerIR.Expr)
    (arguments : Array ExprId) : GenericArgument → Bool
  | .typeArg value =>
      value.typeId == expression.typeId ||
      (match context.unit.tables.types[expression.typeId.index]? with
      | some (.vector element _) => element == value.typeId
      | some (.reference reference) => reference.referent == value.typeId
      | _ => false) || arguments.any fun id =>
        match context.ns.expressions[id.index]? with
        | some argument =>
            argument.typeId == value.typeId ||
              (match context.unit.tables.types[argument.typeId.index]? with
              | some (.vector element _) => element == value.typeId
              | some (.reference reference) => reference.referent == value.typeId
              | _ => false)
        | none => false
  | _ => false

private def typeInstantiationIds? (arguments : Array GenericArgument) : Option (Array TypeId) :=
  arguments.mapM fun
    | .typeArg value => some value.typeId
    | _ => none

private def inferredDataInstantiations (unit : ValidatedUnit) (operandType : TypeId)
    (ownerArguments instantiations : Array GenericArgument) : Bool :=
  instantiations.isEmpty ||
    typeInstantiationIds? instantiations == typeInstantiationIds? ownerArguments ||
    typeInstantiationIds? instantiations == some #[operandType] ||
    (match typeInstantiationIds? instantiations with
    | some #[inferred] => match unit.tables.types[inferred.index]? with
        | some (.reference reference) => reference.referent == operandType
        | _ => false
    | _ => false)

private def nominalAt? (unit : ValidatedUnit) (name : NameId) :
    Option LeanerIR.StructDecl := do
  let qualified ← unit.tables.names[name.index]?
  -- Owned namespaces are a list, not an array keyed by namespace identity.
  let ns ← unit.namespaces.find? (·.identity == qualified.namespaceId)
  ns.structs.find? (·.name == name)

private def standardVectorFunction? (context : Context) (reference : QualifiedRef)
    (functionName : String) : Bool :=
  context.ns.profile == some .move &&
  match context.unit.tables.names[reference.name.index]?,
      context.unit.tables.namespaces[reference.namespaceId.index]? with
  | some qualified, some namespaceRef =>
      qualified.namespaceId == reference.namespaceId &&
        executableReferenceName qualified.name == functionName &&
        namespaceRef.segments.size >= 2 &&
        namespaceRef.segments[namespaceRef.segments.size - 2]? == some "std" &&
        namespaceRef.segments.back? == some "vector"
  | _, _ => false

private def standardReceiverFunction? (context : Context)
    (reference : QualifiedRef) : Bool :=
  match context.unit.tables.names[reference.name.index]?,
      context.unit.tables.namespaces[reference.namespaceId.index]? with
  | some qualified, some namespaceRef =>
      qualified.namespaceId == reference.namespaceId &&
        (standardCallReceiverMode? context.ns.profile namespaceRef
          (executableReferenceName qualified.name)).isSome
  | _, _ => false

private def localReceiverFunction? (context : Context) (reference : QualifiedRef) : Bool :=
  reference.namespaceId == context.ns.identity &&
    (context.unit.tables.names[reference.name.index]?.any fun target =>
      context.ns.functions.any fun declaration =>
        (context.unit.tables.names[declaration.name.index]?.any fun candidate =>
          executableReferenceName candidate.name == executableReferenceName target.name) &&
          declaration.signature.parameters[0]?.any (·.name == "self"))

private partial def typeContainsParameter (unit : ValidatedUnit) (typeId : TypeId)
    (parameter : Nat) (fuel : Nat) : Bool :=
  if fuel == 0 then false else
  match unit.tables.types[typeId.index]? with
  | some (.typeParameter index) => index == parameter
  | some (.tuple elements) =>
      elements.any (typeContainsParameter unit · parameter (fuel - 1))
  | some (.vector element _) | some (.typeDomain element) =>
      typeContainsParameter unit element parameter (fuel - 1)
  | some (.resourceDomain _ (some arguments)) =>
      arguments.any (typeContainsParameter unit · parameter (fuel - 1))
  | some (.nominal _ arguments) => arguments.any fun
      | .typeArg value => typeContainsParameter unit value.typeId parameter (fuel - 1)
      | _ => false
  | some (.function arguments result _) =>
      arguments.any (typeContainsParameter unit · parameter (fuel - 1)) ||
        typeContainsParameter unit result parameter (fuel - 1)
  | some (.reference reference) =>
      typeContainsParameter unit reference.referent parameter (fuel - 1)
  | _ => false

private def signatureTypeArgumentsInferable (unit : ValidatedUnit)
    (signature : Signature) (instantiations : Array GenericArgument) : Bool :=
  signature.generics.size == instantiations.size &&
    signature.generics.all (·.kind == BinderKind.typeArg) &&
    instantiations.all (fun | .typeArg _ => true | _ => false) &&
    signature.generics.zipIdx.all fun (_, index) =>
      signature.parameters.any (fun parameter =>
        typeContainsParameter unit parameter.typeUse.typeId index
          (unit.tables.types.size + 1))

/-- The signature this unit holds for a reference: from an owned namespace, or
from the dependency interface that declares it. -/
private def functionSignatureFor? (unit : ValidatedUnit) (reference : QualifiedRef) :
    Option Signature :=
  let owned := do
    let targetNs ← unit.namespaces.find? fun (candidate : ValidatedNamespace) =>
      candidate.identity == reference.namespaceId
    let declaration ← targetNs.functions.find? fun
      (candidate : LeanerIR.FunctionDecl FunctionBody) => candidate.name == reference.name
    pure declaration.signature
  owned <|> do
    let interface ← unit.dependencies.find? (·.namespaceId == reference.namespaceId)
    let declaration ← interface.functions.find? fun
      (candidate : LeanerIR.FunctionDecl FunctionBody) => candidate.name == reference.name
    pure declaration.signature

/-- The specification signature this unit holds for a reference, preferring a
specification function and falling back to the executable declaration. -/
private def specificationSignatureFor? (unit : ValidatedUnit) (reference : QualifiedRef) :
    Option Signature :=
  let specification :=
    (do
      let targetNs ← unit.namespaces.find? fun (candidate : ValidatedNamespace) =>
        candidate.identity == reference.namespaceId
      let declaration ← targetNs.specFunctions.find? fun
        (candidate : LeanerIR.SpecFunctionDecl) => candidate.name == reference.name
      pure declaration.signature) <|>
    (do
      let interface ← unit.dependencies.find? (·.namespaceId == reference.namespaceId)
      let declaration ← interface.specFunctions.find? fun
        (candidate : LeanerIR.SpecFunctionDecl) => candidate.name == reference.name
      pure declaration.signature)
  specification <|> functionSignatureFor? unit reference

private def standardCallTypeArgumentsInferable (context : Context)
    (reference : QualifiedRef) (functionName : String)
    (instantiations : Array GenericArgument) (arguments : Array ExprId) : Bool :=
  match context.unit.tables.namespaces[reference.namespaceId.index]?,
      typeInstantiationIds? instantiations,
      arguments.mapM (fun argument : ExprId =>
        context.ns.expressions[argument.index]?.map (·.typeId)) with
  | some namespaceRef, some instantiations, some argumentTypes =>
      standardCallInferredTypes? context.unit.tables context.ns.profile namespaceRef
        functionName argumentTypes == some instantiations
  | _, _, _ => false

private def callTypeArgumentsInferable (context : Context) (reference : QualifiedRef)
    (instantiations : Array GenericArgument) (arguments : Array ExprId) : Bool :=
  let fromDeclaration := (functionSignatureFor? context.unit reference).map fun signature =>
    signatureTypeArgumentsInferable context.unit signature instantiations
  if fromDeclaration.getD false then true else
    match context.unit.tables.names[reference.name.index]? with
    | some functionName => standardCallTypeArgumentsInferable context reference
        functionName.name instantiations arguments
    | none => false

private def specificationCallTypeArgumentsInferable (context : Context)
    (reference : QualifiedRef) (surfaceName : String)
    (instantiations : Array GenericArgument) (arguments : Array ExprId) : Bool :=
  let fromDeclaration := (specificationSignatureFor? context.unit reference).map
    fun signature => signatureTypeArgumentsInferable context.unit signature instantiations
  fromDeclaration.getD false ||
    standardCallTypeArgumentsInferable context reference surfaceName instantiations arguments

private partial def inferPrintedTypeArgumentsFuel (unit : ValidatedUnit)
    (inferred : Array (Option TypeId)) (pattern actual : TypeId) (fuel : Nat) :
    Option (Array (Option TypeId)) := do
  guard (fuel > 0)
  let patternNode ← unit.tables.types[pattern.index]?
  let actualNode ← unit.tables.types[actual.index]?
  match patternNode, actualNode with
  | .typeParameter index, _ =>
      let slot ← inferred[index]?
      match slot with
      | some previous =>
          guard (previous == actual)
          pure inferred
      | none => pure (inferred.set! index (some actual))
  | .tuple patterns, .tuple actuals =>
      guard (patterns.size == actuals.size)
      (patterns.zip actuals).foldlM (init := inferred) fun inferred (pattern, actual) =>
        inferPrintedTypeArgumentsFuel unit inferred pattern actual (fuel - 1)
  | .vector pattern _, .vector actual _
  | .typeDomain pattern, .typeDomain actual =>
      inferPrintedTypeArgumentsFuel unit inferred pattern actual (fuel - 1)
  | .resourceDomain patternName (some patterns),
      .resourceDomain actualName (some actuals) =>
      guard (patternName == actualName && patterns.size == actuals.size)
      (patterns.zip actuals).foldlM (init := inferred) fun inferred (pattern, actual) =>
        inferPrintedTypeArgumentsFuel unit inferred pattern actual (fuel - 1)
  | .nominal patternName patterns, .nominal actualName actuals =>
      guard (patternName == actualName && patterns.size == actuals.size)
      (patterns.zip actuals).foldlM (init := inferred) fun inferred pair =>
        match pair with
        | (.typeArg pattern, .typeArg actual) =>
            inferPrintedTypeArgumentsFuel unit inferred pattern.typeId actual.typeId (fuel - 1)
        | (pattern, actual) => do
            guard (pattern == actual)
            pure inferred
  | .reference pattern, .reference actual =>
      inferPrintedTypeArgumentsFuel unit inferred pattern.referent actual.referent (fuel - 1)
  | .function patternArguments patternResult _,
      .function actualArguments actualResult _ =>
      guard (patternArguments.size == actualArguments.size)
      let inferred ← (patternArguments.zip actualArguments).foldlM (init := inferred)
        fun inferred (pattern, actual) =>
          inferPrintedTypeArgumentsFuel unit inferred pattern actual (fuel - 1)
      inferPrintedTypeArgumentsFuel unit inferred patternResult actualResult (fuel - 1)
  | _, _ => pure inferred

private def missingCallTypeArguments? (context : Context) (reference : QualifiedRef)
    (expression : LeanerIR.Expr) (arguments : Array ExprId) : Option (Array TypeId) := do
  guard (reference.namespaceId == context.ns.identity)
  let declaration ← context.ns.functions.find? (·.name == reference.name)
  guard (!declaration.signature.generics.isEmpty)
  guard (declaration.signature.generics.all (·.kind == .typeArg))
  guard (declaration.signature.parameters.size == arguments.size)
  let fuel := context.unit.tables.types.size + 1
  let needsExplicit := declaration.signature.generics.zipIdx.any fun (_, index) =>
    !declaration.signature.parameters.any (fun parameter =>
      typeContainsParameter context.unit parameter.typeUse.typeId index fuel)
  guard needsExplicit
  let mut inferred : Array (Option TypeId) :=
    Array.replicate declaration.signature.generics.size none
  for (parameter, argument) in declaration.signature.parameters.zip arguments do
    let argument ← context.ns.expressions[argument.index]?
    inferred ← inferPrintedTypeArgumentsFuel context.unit inferred
      parameter.typeUse.typeId argument.typeId fuel
  match declaration.signature.results with
  | #[] => none
  | #[result] =>
      inferred ← inferPrintedTypeArgumentsFuel context.unit inferred
        result.typeId expression.typeId fuel
  | results =>
      let .tuple actuals ← context.unit.tables.types[expression.typeId.index]?
        | none
      guard (results.size == actuals.size)
      for (result, actual) in results.zip actuals do
        inferred ← inferPrintedTypeArgumentsFuel context.unit inferred result.typeId actual fuel
  guard (inferred.all (·.isSome))
  pure (inferred.map (·.get!))

private partial def bindingPatternLocalIdsFuel (context : Context) (id : PatternId)
    (fuel : Nat) : Except String (Array LocalId) := do
  if fuel == 0 then throw "cyclic pattern reached the LeanerLang source backend"
  let some pattern := context.ns.patterns[id.index]?
    | throw s!"LeanerLang source references missing pattern {id.index}"
  match pattern.kind with
  | .wildcard => pure #[]
  | .variable localId => pure #[localId]
  | .tuple children | .constructor _ _ _ children =>
      children.foldlM (init := #[]) fun locals child => do
        pure (locals ++ (← bindingPatternLocalIdsFuel context child (fuel - 1)))
  | .literal _ | .range .. => pure #[]

private def bindingPatternLocalIds (context : Context) (id : PatternId) :
    Except String (Array LocalId) :=
  bindingPatternLocalIdsFuel context id (context.ns.patterns.size + 1)

private partial def bindingPatternTextFuel (context : Context) (id : PatternId)
    (fuel : Nat) : Except String String := do
  if fuel == 0 then throw "cyclic pattern reached the LeanerLang source backend"
  let some pattern := context.ns.patterns[id.index]?
    | throw s!"LeanerLang source references missing pattern {id.index}"
  match pattern.kind with
  | .wildcard => pure "_"
  | .variable localId =>
      let name ← localName context localId
      pure (if context.mutableBindings.contains localId then s!"mut {name}" else name)
  | .literal value => constText value
  | .constructor name instantiations variant children =>
      let patternType ← match context.unit.tables.types[pattern.typeId.index]? with
        | some (.nominal ..) => pure pattern.typeId
        | some (.reference reference) => pure reference.referent
        | _ => throw "a constructor pattern does not have a nominal or nominal-reference type"
      let some (.nominal owner ownerArguments) :=
          context.unit.tables.types[patternType.index]?
        | throw "a referenced constructor pattern does not refer to a nominal type"
      unless owner == name &&
          typeInstantiationIds? instantiations == typeInstantiationIds? ownerArguments do
        throw "constructor-pattern instantiations disagree with its nominal type"
      let some declaration := nominalAt? context.unit name
        | throw "a constructor pattern references a missing nominal declaration"
      let fields ← match variant with
        | none =>
            if declaration.variants.isEmpty then pure declaration.fields
            else throw "an enum constructor pattern does not name a variant"
        | some variant =>
            let some variantDecl := declaration.variants.find? fun candidate =>
                (context.unit.tables.names[candidate.name.index]?.map (·.name)).getD "" == variant
              | throw s!"a constructor pattern references missing variant `{variant}`"
            pure variantDecl.fields
      unless children.size == fields.size do
        throw "a constructor pattern has the wrong number of fields"
      let entries ← (fields.zip children).mapM fun (field, child) => do
        pure s!"{← nameAt context.unit field.name} := \
          {← bindingPatternTextFuel context child (fuel - 1)}"
      let owner ← typeText context.unit context.ns context.binders patternType
      let constructor := variant.map (fun variant =>
        owner ++ "::" ++ sourceIdentifier variant) |>.getD owner
      pure (constructor ++ " { " ++ commaSep entries ++ " }")
  | .tuple children =>
      let entries ← children.mapM (bindingPatternTextFuel context · (fuel - 1))
      let suffix := if entries.size == 1 then "," else ""
      pure s!"({commaSep entries}{suffix})"
  | .range .. =>
      throw "this pattern is outside local-declaration LeanerLang syntax"

private def bindingPatternText (context : Context) (id : PatternId) : Except String String :=
  bindingPatternTextFuel context id (context.ns.patterns.size + 1)

/-- Does this statement's text end inside a block of its own? A separator
there would attach to that block's last entry rather than to the statement
around it. It is the last thing printed that decides: a conditional whose
final branch is one entry ends with that entry and takes the separator, which
is also what keeps it a statement when the text is read back. A `return` never
ends inside a block: its value is delimited when it spans lines. -/
private partial def endsInsideBlock (ns : ValidatedNamespace) (id : ExprId)
    (fuel : Nat) : Bool :=
  if fuel == 0 then false else
  match ns.expressions[id.index]? with
  | none => false
  | some expression => match expression.kind with
      | .block .. | .letDecl .. | .loop .. | .spec .. => true
      | .ifElse _ thenBranch elseBranch =>
          endsInsideBlock ns (elseBranch.getD thenBranch) (fuel - 1)
      | .match_ _ arms => (arms.back?).any fun arm => endsInsideBlock ns arm.body (fuel - 1)
      | .assign _ value | .assignPattern _ value => endsInsideBlock ns value (fuel - 1)
      | _ => false

/-- Separator after one statement of a block. -/
private def statementTerminator (ns : ValidatedNamespace) (id : ExprId)
    (text : String) : String :=
  if text.contains '\n' && endsInsideBlock ns id (ns.expressions.size + 1) then "" else ";"

private partial def expressionText (context : Context) (id : ExprId)
    (fuel : Nat) (tailPosition : Bool := false) (statementPosition : Bool := false) :
    Except String String := do
  if fuel == 0 then throw "cyclic expression reached the LeanerLang source backend"
  let some expression := context.ns.expressions[id.index]?
    | throw s!"LeanerLang source references missing expression {id.index}"
  -- A call argument: a mutable reference copied into the call is the
  -- reborrow of the local that holds it, which the source spells as the
  -- bare local; anything else prints as the expression it is.
  let callArgumentText (argument : ExprId) (fuel : Nat) : Except String String :=
    match reborrowedLocal? context argument with
    | some localId => localValueText context localId
    | none => expressionText context argument fuel
  if let some iterator := context.forIterator then
    if incrementedContinue? context.ns iterator id then return "continue"
  if let some range := forRangeShape? context id then
    let iterator ← localName context range.iterator
    let lower ← expressionText context range.lower (fuel - 1)
    let upper ← expressionText context range.upper (fuel - 1)
    let bodyContext := { context with forIterator := some range.iterator }
    let rendered ← range.bodyStatements.mapM fun statement => do
      let statementText ← expressionText bodyContext statement (fuel - 1) false true
      pure (statementText ++ statementTerminator context.ns statement statementText)
    let body := if rendered.isEmpty then "()" else if rendered.size == 1 &&
        rendered[0]!.startsWith "do\n" then
      rendered[0]!.drop 3 |>.toString
    else lines rendered
    return s!"for {iterator} in {lower}..{upper} do\n{indent body}"
  match expression.kind with
  | .value _ (some sourceConstant) =>
      pure (sourceIdentifier sourceConstant)
  | .value (.integer value) none =>
      integerLiteralText context.unit context.ns context.specification expression.typeId value
  | .value value none => constText value
  | .constant reference =>
      qualifiedNameAt context.unit context.ns.identity reference.name
  | .localVar localId => do
      if let some (_, value) := context.constantLocals.find? (·.1 == localId) then
        return value
      let some localDecl := context.locals[localId.index]?
        | throw s!"LeanerLang source references missing local {localId.index}"
      let name := sourceIdentifier (context.localNames[localId.index]?.getD localDecl.name)
      if context.logicalLocals.contains localId ||
          expression.typeId == localDecl.type.typeId then
        pure name
      else
        match context.unit.tables.types[expression.typeId.index]?,
            context.unit.tables.types[localDecl.type.typeId.index]? with
        | some (.integer .unbounded true),
            some (.integer (.bits _) _) | some (.integer .unbounded true),
            some (.integer .pointer _) =>
            pure s!"spec.bitVectorToInt({name})"
        | _, _ => pure name
  | .operation operation instantiations arguments _surface =>
      match operation with
      | .move place =>
          unless instantiations.isEmpty do
            throw "place loads cannot print explicit generic arguments"
          if let some localId := localPlace? context.ns place then
            return ← localValueText context localId
          let place ← placeText context
            (fun index => expressionText context index (fuel - 1)) place
          pure s!"move({cancelBorrowedPlaceText place})"
      | .copy place =>
          unless instantiations.isEmpty do
            throw "place loads cannot print explicit generic arguments"
          if let some localId := localPlace? context.ns place then
            return ← localValueText context localId
          let place ← placeText context
            (fun index => expressionText context index (fuel - 1)) place
          if context.ns.profile == some .move ||
              (context.ns.profile == some .rust && rustImplicitCopyType context expression.typeId) then
            return cancelBorrowedPlaceText place
          pure s!"copy({cancelBorrowedPlaceText place})"
      | .read place =>
          unless instantiations.isEmpty do
            throw "place loads cannot print explicit generic arguments"
          if let some localId := localPlace? context.ns place then
            return ← localValueText context localId
          let place ← placeText context
            (fun index => expressionText context index (fuel - 1)) place
          -- Re-imported Rust indexing and tuple projection uses the neutral
          -- LIR `read` operation where rustc's MIR used `copy`.  Both spell an
          -- ordinary source expression for intrinsically Copy scalars, so the
          -- canonical form must not oscillate between `value[index]` and
          -- `core.read(value[index])`.
          if context.ns.profile == some .rust &&
              rustImplicitCopyType context expression.typeId then
            return cancelBorrowedPlaceText place
          pure s!"core.read({cancelBorrowedPlaceText place})"
      | .drop place =>
          unless instantiations.isEmpty && arguments.isEmpty do
            throw "core.drop has unexpected arguments"
          pure s!"drop({← placeText context
            (fun index => expressionText context index (fuel - 1)) place})"
      | .borrow kind place =>
          unless instantiations.isEmpty && arguments.isEmpty do
            throw "a place borrow has unexpected arguments"
          let kind ← match kind with
            | .immutable => pure "immutable"
            | .mutable => pure "mut"
            | .profile value => throw s!"profile borrow `{value.tag}` has no core spelling"
          pure s!"({if kind == "mut" then "&mut " else "&"}{
            ← placeText context (fun index => expressionText context index (fuel - 1)) place})"
      | .reference (.borrow kind) =>
          unless instantiations.isEmpty && arguments.size == 1 do
            throw "a value borrow must have exactly one operand"
          let kind ← match kind with
            | .immutable => pure "immutable"
            | .mutable => pure "mut"
            | .profile value => throw s!"profile borrow `{value.tag}` has no core spelling"
          pure s!"({if kind == "mut" then "&mut " else "&"}{
            ← expressionText context arguments[0]! (fuel - 1)})"
      | .reference (.freeze explicit) =>
          unless instantiations.isEmpty && arguments.size == 1 do
            throw "a reference freeze must have exactly one operand"
          if !explicit && context.ns.profile == some .move then
            return ← expressionText context arguments[0]! (fuel - 1)
          let operation := if explicit then "core.ref.freezeExplicit" else "core.ref.freeze"
          pure s!"{operation}({← expressionText context arguments[0]! (fuel - 1)})"
      | .reference .dereference =>
          unless instantiations.isEmpty && arguments.size == 1 do
            throw "core.ref.dereference must have exactly one operand"
          let argument ← expressionText context arguments[0]! (fuel - 1)
          if context.specification then return argument
          if let some referent := borrowedReferentText? argument then return referent
          if context.ns.profile == some .move then
            -- Move field selection through a reference yields a reference the
            -- surface reads implicitly, with or without an intervening
            -- freeze, so the dereference has no source spelling of its own.
            if selectsFieldThroughReference context arguments[0]! then return argument
          pure s!"*({argument})"
      | .reference .mutate =>
          unless instantiations.isEmpty && arguments.size == 2 do
            throw "a reference mutation must have exactly two operands"
          let reference ← expressionText context arguments[0]! (fuel - 1)
          let value ← expressionText context arguments[1]! (fuel - 1)
          if let some referent := borrowedReferentText? reference then
            return s!"{referent} := {value}"
          if context.ns.profile == some .move then
            if selectsFieldThroughReference context arguments[0]! then
              return s!"{reference} := {value}"
          pure s!"*({reference}) := {value}"
      | .global global =>
          let some typeIds := typeInstantiationIds? instantiations
            | throw "a global operation requires one resource type argument"
          unless typeIds.size == 1 do
            throw "a global operation requires exactly one resource type argument"
          let resource ← typeText context.unit context.ns context.binders typeIds[0]!
          if context.ns.profile == some .move then
            match global, arguments.toList with
            | .borrow kind, [address] =>
                let address ← expressionText context address (fuel - 1)
                let borrowPrefix ← match kind with
                  | .immutable => pure "&"
                  | .mutable => pure "&mut "
                  | .profile value =>
                      throw s!"profile global borrow `{value.tag}` has no core spelling"
                return s!"{borrowPrefix}{resource}[{address}]"
            | _, _ => pure ()
          let (name, arity) ← match global with
            | .contains => pure ("exists", 1)
            | .take => pure ("move_from", 1)
            | .publish => pure ("move_to", 2)
            | .borrow .immutable => pure ("borrow_global", 1)
            | .borrow .mutable => pure ("borrow_global_mut", 1)
            | .borrow (.profile value) =>
              throw s!"profile global borrow `{value.tag}` has no core spelling"
          unless arguments.size == arity do
            throw s!"{name} expects {arity} operand(s)"
          let arguments ← arguments.mapM (expressionText context · (fuel - 1))
          pure <| name ++ typeArguments resource ++ s!"({commaSep arguments})"
      | .primitive primitive =>
          if primitive == .tuple && instantiations.isEmpty && arguments.isEmpty then
            return "()"
          if primitive == .tuple && instantiations.isEmpty && arguments.size == 1 then
            return s!"({← expressionText context arguments[0]! (fuel - 1)},)"
          -- Importers may retain the inferred operand/result type as a
          -- redundant generic argument. LeanerLang reconstructs it from the
          -- typed expression and has no explicit spelling for it.
          let inferred := instantiations[0]?.any
            (inferredPrimitiveArgument context expression arguments)
          unless instantiations.isEmpty || (instantiations.size == 1 && inferred) do
            throw s!"primitive `{repr primitive}` has non-inferable generic operation arguments"
          if primitive == .vector && context.ns.profile == some .move then
            let some (.vector element none) :=
                context.unit.tables.types[expression.typeId.index]?
              | throw "a Move vector operation requires an unfixed vector result type"
            let element ← typeText context.unit context.ns context.binders element
            let arguments ← arguments.mapM (expressionText context · (fuel - 1))
            return "vector" ++ typeArguments element ++ s!"[{commaSep arguments}]"
          -- Prefer `collection.length > bound` over rustc's equivalent
          -- `bound < collection.length`.  Besides matching ordinary source
          -- style, this lets the logical projection infer the bound from the
          -- specification-side integer returned by `length`.
          let originalArguments := arguments
          let arguments ← arguments.zipIdx.mapM fun (argument, index) =>
            if primitive == .length && index == 0 then
              match context.ns.expressions[argument.index]? with
              | some argumentNode => match argumentNode.kind with
                  | .operation (.reference .dereference) #[] #[referent] _ =>
                      expressionText context referent (fuel - 1)
                  | .operation (.read place) #[] #[] _
                  | .operation (.copy place) #[] #[] _ => do
                      -- Rust MIR obtains slice metadata by reading the
                      -- dereferenced slice place. At source level `.length`
                      -- borrows and auto-dereferences its receiver, so retain
                      -- the read in LIR without exposing `core.read(*values)`.
                      let receiver ← placeText context
                        (fun index => expressionText context index (fuel - 1)) place
                      pure <| implicitDerefReceiverText
                        (cancelBorrowedPlaceText receiver)
                  | _ => expressionText context argument (fuel - 1)
              | none => expressionText context argument (fuel - 1)
            else if primitive == .index && index == 0 then
              -- Move and Rust both auto-dereference a referenced collection
              -- used as the receiver of index syntax.  Retain the dereference
              -- in LIR without printing `*(values)[index]`, whose surface
              -- precedence would misleadingly dereference the element.
              match context.ns.expressions[argument.index]? with
              | some argumentNode => match argumentNode.kind with
                  | .operation (.reference .dereference) #[] #[referent] _ =>
                      expressionText context referent (fuel - 1)
                  | _ => expressionText context argument (fuel - 1)
              | none => expressionText context argument (fuel - 1)
            else expressionText context argument (fuel - 1)
          -- A control-flow operand binds looser than every operator, cast, and
          -- index the surface can wrap it in.
          let arguments := (arguments.zip originalArguments).map fun (text, argument) =>
            parenthesizedOperand context.ns argument text
          let (primitive, arguments) :=
            if arguments.size == 2 && arguments[1]!.endsWith ".length" then
              match reversedComparison? primitive with
              | some reversed => (reversed, #[arguments[1]!, arguments[0]!])
              | none => (primitive, arguments)
            else (primitive, arguments)
          if primitive == .repeatVector then
            let [argument] := arguments.toList
              | throw "repeated vector construction expects one operand"
            let some (.vector _ (some (.integer length))) :=
                context.unit.tables.types[expression.typeId.index]?
              | throw "repeated vector construction requires a fixed-vector result type"
            unless length >= 0 do
              throw "repeated vector construction has a negative result length"
            return s!"#[{argument}; {length}]"
          if primitive == .slice then
            unless arguments.size == 3 do
              throw "slice expects a value, start, and end operand"
            return s!"slice({commaSep arguments})"
          let overflowingName? := match primitive with
            | .overflowingAdd => some "overflowing_add"
            | .overflowingSubtract => some "overflowing_subtract"
            | .overflowingMultiply => some "overflowing_multiply"
            | _ => none
          if let some name := overflowingName? then
            unless arguments.size == 2 do
              throw s!"{name} expects two operands"
            return s!"{name}({commaSep arguments})"
          if let some operator := operatorInfo? context primitive then
            match operator.fixity, arguments with
            | .prefix, #[argument] => pure s!"({operator.symbol}{argument})"
            | .infix, #[left, right] => pure s!"({left} {operator.symbol} {right})"
            | _, _ => throw s!"operator `{operator.symbol}` has the wrong number of operands"
          else match primitive with
          | .length =>
              let [argument] := arguments.toList
                | throw "length expects one operand"
              let argument := (borrowedReferentText? argument).getD argument
              let argument := implicitDerefReceiverText argument
              pure s!"{argument}.length"
          | .index =>
              let [value, index] := arguments.toList
                | throw "vector indexing expects two operands"
              pure s!"{value}[{index}]"
          | .cast =>
              let [argument] := arguments.toList
                | throw "integer cast expects one operand"
              let result ← contextTypeText context expression.typeId
              if context.specification && result == "Int" then return argument
              pure s!"({argument} as {result})"
          | .checkedCast .abort =>
              let [argument] := arguments.toList
                | throw "checked integer cast expects one operand"
              let result ← contextTypeText context expression.typeId
              if context.specification && result == "Int" then pure argument
              else if context.ns.profile == some .move then
                pure s!"({argument} as {result})"
              else
                pure s!"core.prim.checkedCast[abort, {result}]({argument})"
          | .checkedCast failure =>
              let result ← contextTypeText context expression.typeId
              pure s!"core.prim.checkedCast[{← failureText failure}, {result}]({commaSep arguments})"
          | _ => pure s!"core.prim.{← primitiveText primitive}({commaSep arguments})"
      | .call (.function reference) =>
          if standardVectorFunction? context reference "borrow" ||
              standardVectorFunction? context reference "borrow_mut" then
            let [collection, index] := arguments.toList
              | throw "std::vector element borrow expects two operands"
            let collection ← callArgumentText collection (fuel - 1)
            let collection := (borrowedReferentText? collection).getD collection
            let index ← expressionText context index (fuel - 1)
            let borrowPrefix := if standardVectorFunction? context reference "borrow_mut" then
                "&mut " else "&"
            return s!"{borrowPrefix}{collection}[{index}]"
          if standardVectorFunction? context reference "length" then
            let [argument] := arguments.toList
              | throw "std::vector::length expects one operand"
            let argument ← callArgumentText argument (fuel - 1)
            let argument := (borrowedReferentText? argument).getD argument
            return s!"{argument}.length"
          let missingTypeArguments := if instantiations.isEmpty then
              missingCallTypeArguments? context reference expression arguments
            else none
          let inferableTypeArguments :=
            callTypeArgumentsInferable context reference instantiations arguments
          let explicitTypeIds ← if let some typeIds := missingTypeArguments then
              pure typeIds
            else if instantiations.isEmpty || inferableTypeArguments then pure #[]
            else
              let some typeIds := typeInstantiationIds? instantiations
                | throw "generic function calls contain non-type arguments"
              pure typeIds
          let explicitTypes ← explicitTypeIds.mapM
            (typeText context.unit context.ns context.binders)
          let typeSuffix := if explicitTypes.isEmpty then "" else
            "::" ++ typeArguments (commaSep explicitTypes)
          let renderedArguments ← arguments.mapM (callArgumentText · (fuel - 1))
          let standardReceiver := standardReceiverFunction? context reference
          let receiverPreferred :=
            localReceiverFunction? context reference || standardReceiver
          let call ← if receiverPreferred then do
              let some receiver := renderedArguments[0]?
                | throw "a receiver call has no receiver operand"
              let receiver := (borrowedReferentText? receiver).getD receiver
              let some qualified := context.unit.tables.names[reference.name.index]?
                | throw s!"a receiver call references missing name {reference.name.index}"
              let method := sourceIdentifier (surfaceReferenceName qualified.name)
              pure s!"{receiver}.{method}{typeSuffix}({
                commaSep (renderedArguments.drop 1)})"
            else do
              let name ← qualifiedNameAt context.unit context.ns.identity reference.name
              pure s!"{name}{typeSuffix}({commaSep renderedArguments})"
          -- A signature this unit holds — its own, or one a dependency
          -- interface carries — is what the reader needs to recover the
          -- result type, so only a call without one is annotated.
          let hasDeclaration := context.unit.namespaces.any (fun candidate =>
              candidate.identity == reference.namespaceId &&
                candidate.functions.any (·.name == reference.name)) ||
            context.unit.dependencies.any fun interface =>
              interface.namespaceId == reference.namespaceId &&
                interface.functions.any (·.name == reference.name)
          if reference.namespaceId == context.ns.identity || standardReceiver || hasDeclaration then
            pure call
          else
            let result ← contextTypeText context expression.typeId
            pure s!"({call} : {result})"
      | .call (.closure reference) =>
          unless instantiations.isEmpty do
            throw "generic closure construction is outside the current LeanerLang parser"
          let name ← qualifiedNameAt context.unit context.ns.identity reference.name
          let result ← contextTypeText context expression.typeId
          let captures ← arguments.mapM (expressionText context · (fuel - 1))
          let suffix := if captures.isEmpty then "" else ", " ++ commaSep captures
          pure s!"function[{result}]({name}{suffix})"
      | .call .invoke =>
          unless instantiations.isEmpty && !arguments.isEmpty do
            throw "core.invoke requires a callable operand and no generic arguments"
          pure s!"invoke({commaSep (← arguments.mapM
            (expressionText context · (fuel - 1)))})"
      | .call (.constructor reference variant) =>
          let some (.nominal owner resultArguments) :=
              context.unit.tables.types[expression.typeId.index]?
            | throw "a constructor result is not nominal"
          unless owner == reference.name &&
              typeInstantiationIds? instantiations == typeInstantiationIds? resultArguments do
            throw "constructor instantiations disagree with its nominal result type"
          -- A dependency's nominal declaration is not part of this unit. Its
          -- field names arrive with dependency-interface enrichment; until
          -- then only a field-less constructor has a faithful spelling.
          let fields ← match nominalAt? context.unit reference.name with
            | none =>
                if arguments.isEmpty then pure #[]
                else throw s!"a constructor of nominal \
                  `{← nameAt context.unit reference.name}` needs the field names of a \
                  declaration this unit does not own"
            | some declaration =>
              match variant with
              | none =>
                  if declaration.variants.isEmpty then pure declaration.fields
                  else throw "an enum constructor does not name a variant"
              | some variant =>
                  let mut found := none
                  for candidate in declaration.variants do
                    if (← nameAt context.unit candidate.name) == variant then
                      found := some candidate.fields
                  let some fields := found
                    | throw s!"enum constructor references missing variant `{variant}`"
                  pure fields
          unless fields.size == arguments.size do
            throw "a constructor has the wrong number of field operands"
          let owner ← typeText context.unit context.ns context.binders expression.typeId
          let constructor := variant.map (s!"{owner}::{·}") |>.getD owner
          let entries ← (fields.zip arguments).mapM fun (field, argument) => do
            let fieldName ← nameAt context.unit field.name
            let value ← expressionText context argument (fuel - 1)
            pure <| if value == fieldName then fieldName
              else s!"{fieldName} := {value}"
          pure <| "new " ++ constructor ++ " { " ++ commaSep entries ++ " }"
      | .data (.select reference field) =>
          if arguments.isEmpty then
            unless context.nominalOwner == some reference.name do
              throw "an implicit field selection occurs outside its nominal contract"
            return sourceIdentifier field
          let [valueId] := arguments.toList
            | throw "field selection expects zero implicit or one explicit operand"
          let some value := context.ns.expressions[valueId.index]?
            | throw "field selection references a missing operand"
          let (ownerTypeId, owner, ownerArguments) ←
            match context.unit.tables.types[value.typeId.index]? with
            | some (.nominal owner ownerArguments) =>
                pure (value.typeId, owner, ownerArguments)
            | some (.reference sourceReference) =>
                let some (.nominal owner ownerArguments) :=
                    context.unit.tables.types[sourceReference.referent.index]?
                  | throw "field selection reference does not refer to a nominal type"
                let some (.reference resultReference) :=
                    context.unit.tables.types[expression.typeId.index]?
                  | throw "referenced field selection does not produce a reference"
                unless sourceReference.profile == resultReference.profile &&
                    sourceReference.kind == resultReference.kind do
                  throw "referenced field selection changes reference profile or mutability"
                pure (sourceReference.referent, owner, ownerArguments)
            | _ => throw "field selection operand is not nominal or a nominal reference"
          unless owner == reference.name &&
              inferredDataInstantiations context.unit ownerTypeId ownerArguments instantiations do
            throw "field-selection instantiations disagree with its nominal operand type"
          let valueText ←
            if context.ns.profile == some .move then
              match value.kind with
              | .operation (.reference .dereference) #[] #[referent] _ =>
                  let referent ← expressionText context referent (fuel - 1)
                  pure ((borrowedReferentText? referent).getD referent)
              | .operation (.borrow _ place) _ _ _ =>
                  -- Move selects a field of a borrowed place without the
                  -- borrow, and only that spelling is itself a place, so an
                  -- assignment through the selection stays assignable.
                  placeText context (expressionText context · (fuel - 1)) place
              | _ => expressionText context valueId (fuel - 1)
            else expressionText context valueId (fuel - 1)
          let needsParentheses := selectionNeedsParentheses context value
          let valueText := if needsParentheses then s!"({valueText})" else valueText
          pure s!"{valueText}.{sourceIdentifier field}"
      | .data (.selectVariants reference fields) =>
          let [valueId] := arguments.toList
            | throw "variant-field selection expects exactly one operand"
          let some value := context.ns.expressions[valueId.index]?
            | throw "variant-field selection references a missing operand"
          -- A mutable variant-field selection reads through the operand
          -- reference; the surface spelling is the same field access.
          let operandType ← match context.unit.tables.types[value.typeId.index]? with
            | some (.reference reference) => pure reference.referent
            | _ => pure value.typeId
          let some (.nominal owner ownerArguments) :=
              context.unit.tables.types[operandType.index]?
            | throw "variant-field selection operand is not nominal"
          unless owner == reference.name &&
              inferredDataInstantiations context.unit operandType ownerArguments instantiations do
            throw "variant-field instantiations disagree with its nominal operand type"
          unless !fields.isEmpty do throw "variant-field selection names no fields"
          if fields.size != 1 then
            throw "variant-field selection with multiple source names has no mixin spelling"
          -- Move reads a variant field through a reference implicitly, so its
          -- dereference is spelled by the field access itself; every other
          -- operand keeps the delimiters field access needs.
          let valueText ←
            if context.ns.profile == some .move then
              match value.kind with
              | .operation (.reference .dereference) #[] #[referent] _ =>
                  let referent ← expressionText context referent (fuel - 1)
                  pure ((borrowedReferentText? referent).getD referent)
              | .operation (.borrow _ place) _ _ _ =>
                  -- Move selects a field of a borrowed place without the
                  -- borrow, and only that spelling is itself a place, so an
                  -- assignment through the selection stays assignable.
                  placeText context (expressionText context · (fuel - 1)) place
              | _ => expressionText context valueId (fuel - 1)
            else expressionText context valueId (fuel - 1)
          let needsParentheses := selectionNeedsParentheses context value
          let valueText := if needsParentheses then s!"({valueText})" else valueText
          pure s!"{valueText}.{sourceIdentifier fields[0]!}"
      | .data (.testVariants reference variants) =>
          let [valueId] := arguments.toList
            | throw "variant testing expects exactly one operand"
          let some value := context.ns.expressions[valueId.index]?
            | throw "variant testing references a missing operand"
          let some (.nominal owner ownerArguments) :=
              context.unit.tables.types[value.typeId.index]?
            | throw "variant-test operand is not nominal"
          unless owner == reference.name &&
              inferredDataInstantiations context.unit value.typeId ownerArguments instantiations do
            throw "variant-test instantiations disagree with its nominal operand type"
          unless !variants.isEmpty do throw "variant testing names no variants"
          let variants := " | ".intercalate (variants.map sourceIdentifier).toList
          let value ← expressionText context valueId (fuel - 1)
          let value := if context.ns.profile == some .move then
              implicitDerefReceiverText value
            else value
          pure s!"({value} is {variants})"
      | .data (.discriminant reference) =>
          let [valueId] := arguments.toList
            | throw "discriminant expects exactly one operand"
          let some value := context.ns.expressions[valueId.index]?
            | throw "discriminant references a missing operand"
          let ownerTypeId ← match context.unit.tables.types[value.typeId.index]? with
            | some (.nominal owner _) =>
                if owner == reference.name then pure value.typeId
                else throw "discriminant operand has a different nominal owner"
            | _ => throw "discriminant operand is not nominal"
          let owner ← typeText context.unit context.ns context.binders ownerTypeId
          let result ← contextTypeText context expression.typeId
          pure s!"discriminant[{owner}, {result}]({
            ← expressionText context valueId (fuel - 1)})"
      | .specification (.behavior kind range) =>
          unless instantiations.isEmpty do
            throw "behavior predicates have no generic type arguments"
          let some targetId := arguments[0]?
            | throw "a behavior predicate requires a function-value target"
          let target ← expressionText context targetId (fuel - 1)
          let values ← (arguments.drop 1).mapM (expressionText context · (fuel - 1))
          pure s!"{behaviorRangePrefix kind range}{behaviorOperationName kind}<{target}>({
            commaSep values})"
      | .specification (.functionCall reference range) =>
          unless instantiations.all fun | .typeArg _ => true | _ => false do
            throw "non-type specification-function arguments are outside the current LeanerLang parser"
          unless range == {} do
            throw "state-ranged specification-function calls are outside the current LeanerLang parser"
          if isArbitrarySpecificationCall context reference then
            unless arguments.isEmpty do
              throw "an arbitrary specification value unexpectedly has arguments"
            return "abort()"
          -- Move compiler v2 retains calls to the generated `$length`
          -- specification companion of `std::vector::length`.  Its result is
          -- already the mathematical integer used by specification
          -- expressions, so canonical source uses the ordinary collection
          -- projection and lets that widening remain implicit.
          if standardVectorFunction? context reference "length" then
            let [argument] := arguments.toList
              | throw "std::vector specification length expects one operand"
            let argument ← expressionText context argument (fuel - 1)
            let argument := (borrowedReferentText? argument).getD argument
            let argument := implicitDerefReceiverText argument
            return s!"{argument}.length"
          let (name, moveCompanion) ← specificationCallName context reference
          let surfaceName := match context.unit.tables.names[reference.name.index]? with
            | some qualified =>
                if moveCompanion then qualified.name.drop 1 |>.toString else qualified.name
            | none => ""
          let inferableTypeArguments := specificationCallTypeArgumentsInferable context
            reference surfaceName instantiations arguments
          let arguments ← arguments.mapM (expressionText context · (fuel - 1))
          let typeSuffix ← if instantiations.isEmpty || inferableTypeArguments then
              pure ""
            else do
              let some typeIds := typeInstantiationIds? instantiations
                | throw "specification-function calls contain non-type arguments"
              let argumentTexts ← typeIds.mapM
                (typeText context.unit context.ns context.binders)
              pure <| "::" ++ typeArguments (commaSep argumentTexts)
          let call ← if localReceiverFunction? context reference then do
              let some receiver := arguments[0]?
                | throw "a receiver specification call has no receiver operand"
              let receiver := (borrowedReferentText? receiver).getD receiver
              let some qualified := context.unit.tables.names[reference.name.index]?
                | throw s!"a receiver call references missing name {reference.name.index}"
              let rawMethod := if moveCompanion then qualified.name.drop 1 |>.toString
                else qualified.name
              let method := sourceIdentifier (surfaceReferenceName rawMethod)
              pure s!"{receiver}.{method}{typeSuffix}({commaSep (arguments.drop 1)})"
            else
              pure s!"{name}{typeSuffix}({commaSep arguments})"
          -- A signature this unit holds — its own, or one a dependency
          -- interface carries — gives the reader the result type, so only a
          -- call without one is annotated.
          let hasDeclaration := context.unit.namespaces.any (fun candidate =>
              candidate.identity == reference.namespaceId &&
                (candidate.specFunctions.any (·.name == reference.name) ||
                  candidate.functions.any (·.name == reference.name))) ||
            context.unit.dependencies.any fun interface =>
              interface.namespaceId == reference.namespaceId &&
                (interface.specFunctions.any (·.name == reference.name) ||
                  interface.functions.any (·.name == reference.name))
          if reference.namespaceId == context.ns.identity || hasDeclaration then pure call
          else
            let result ← contextTypeText context expression.typeId
            pure s!"({call} : {result})"
      | .specification (.result index) =>
          unless instantiations.isEmpty && arguments.isEmpty do
            throw "the specification result operation has unexpected arguments"
          -- A multi-result signature names its later results by index; Move
          -- packs them into one tuple-typed row.
          pure (if index == 0 then "result" else s!"spec.result[{index}]")
      | .specification .old =>
          let some typeIds := typeInstantiationIds? instantiations
            | throw "spec.old requires an inferred type argument"
          unless typeIds.size == 1 && arguments.size == 1 &&
              typeIds[0]! == expression.typeId do
            throw s!"spec.old must have one matching type argument and one operand"
          let argument ← expressionText context arguments[0]! (fuel - 1)
          pure s!"old({argument})"
      | .specification (.global label) =>
          unless label.isNone && arguments.size == 1 do
            throw "spec.global currently requires one key and no state label"
          let some typeIds := typeInstantiationIds? instantiations
            | throw "spec.global requires one resource type argument"
          unless typeIds.size == 1 do
            throw "spec.global requires exactly one resource type argument"
          let resource ← typeText context.unit context.ns context.binders typeIds[0]!
          pure <| "global" ++ typeArguments resource ++
            s!"({← expressionText context arguments[0]! (fuel - 1)})"
      | .specification (.saveStateAnchor label) =>
          unless instantiations.isEmpty && arguments.isEmpty do
            throw "spec.saveStateAnchor has unexpected arguments"
          pure s!"save_state_anchor!({label})"
      | .specification (.foldsCaptureAnchor label) =>
          unless instantiations.isEmpty && arguments.isEmpty do
            throw "spec.foldsCaptureAnchor has unexpected arguments"
          pure s!"folds_capture_anchor!({label})"
      | .specification (.withStateAnchor label) =>
          unless instantiations.isEmpty && arguments.size == 1 do
            throw "spec.withStateAnchor must have exactly one argument"
          let argument ← expressionText context arguments[0]! (fuel - 1)
          pure s!"with_state_anchor!({label}, {argument})"
      | .specification .inRange =>
          unless instantiations.isEmpty && arguments.size == 2 do
            throw "spec.inRange requires two operands and no generic arguments"
          pure s!"in_range({commaSep (← arguments.mapM
            (expressionText context · (fuel - 1)))})"
      | .specification .bitVectorToInt =>
          unless instantiations.isEmpty && arguments.size == 1 do
            throw "spec.bitVectorToInt requires one operand and no generic arguments"
          let some argumentNode := context.ns.expressions[arguments[0]!.index]?
            | throw "spec.bitVectorToInt references a missing operand"
          let argument ← expressionText context arguments[0]! (fuel - 1)
          if argumentNode.kind matches .constant _ | .value _ (some _) then
            -- Fixed-width named constants widen implicitly in specification
            -- expressions, just like authored numeric literals.
            return argument
          if let some localId := accessedLocal? context.ns argumentNode then
            if context.logicalLocals.contains localId then return argument
          match context.unit.tables.types[argumentNode.typeId.index]? with
          | some (.integer (.bits _) _) | some (.integer .pointer _) =>
              -- Specification expressions implicitly widen fixed-width
              -- integers to Int. Keep the conversion in typed LIR, but do
              -- not expose it in canonical source.
              pure argument
          | some (.integer .unbounded true) =>
              -- Compiler-v2 can retain a bv-to-int coercion after its operand
              -- has already entered the logical integer domain. It is then
              -- the identity conversion, as in the legacy transpiler.
              unless argumentNode.typeId == expression.typeId do
                throw "a projected bit-vector conversion changes its logical integer type"
              pure argument
          | _ => throw "spec.bitVectorToInt requires a fixed-width or projected integer"
      | .specification .intToBitVector =>
          let instantiationMatches := instantiations.isEmpty ||
            (typeInstantiationIds? instantiations).any fun ids =>
              ids.size == 1 && ids[0]? == some expression.typeId
          unless instantiationMatches && arguments.size == 1 do
            throw "spec.intToBitVector requires one operand and its result-type instantiation"
          match context.unit.tables.types[expression.typeId.index]? with
          | some (.integer (.bits _) _) | some (.integer .pointer _) =>
              let resultType ← typeText context.unit context.ns context.binders expression.typeId
              let some argumentNode := context.ns.expressions[arguments[0]!.index]?
                | throw "spec.intToBitVector references a missing operand"
              let argument ← expressionText context arguments[0]! (fuel - 1)
              if argumentNode.typeId == expression.typeId then pure argument
              else pure s!"int_to_bit_vector[{resultType}]({argument})"
          | some (.integer .unbounded true) =>
              -- The exchange defers the bit-vector width in the projected
              -- specification domain; the width-less spelling round-trips
              -- that deferral.
              pure s!"int_to_bit_vector({← expressionText context arguments[0]! (fuel - 1)})"
          | _ => throw "spec.intToBitVector requires a fixed-width integer result"
      | .specification .inlineCallSummary =>
          -- A derivation summary for an expanded inline call: its symbolic
          -- result paired with the condition under which it aborts.
          let [result, aborts] := arguments.toList
            | throw "spec.inlineCallSummary expects two operands"
          pure s!"spec.inlineCallSummary({← expressionText context result (fuel - 1)}, {
            ← expressionText context aborts (fuel - 1)})"
      | .specification operation =>
          if operation == .lengthVector then
            let [argument] := arguments.toList
              | throw "spec.lengthVector expects one operand"
            return s!"{← expressionText context argument (fuel - 1)}.length"
          if operation == .indexVector then
            let [value, index] := arguments.toList
              | throw "spec.indexVector expects two operands"
            return s!"{← expressionText context value (fuel - 1)}[{
              ← expressionText context index (fuel - 1)}]"
          if operation == .sliceVector then
            let [value, range] := arguments.toList
              | throw "spec.sliceVector expects two operands"
            return s!"{← expressionText context value (fuel - 1)}[{
              ← expressionText context range (fuel - 1)}]"
          if operation == .containsVector then
            let [collection, element] := arguments.toList
              | throw "spec.containsVector expects two operands"
            return s!"({← expressionText context element (fuel - 1)} ∈ {
              ← expressionText context collection (fuel - 1)})"
          let some name := specificationVectorName? operation
            | throw s!"operation `{repr operation}` is outside the current LeanerLang parser"
          if instantiations.isEmpty then
            -- Imported nodes may defer the element type; the lowering
            -- infers it from operands on re-import.
            let arguments ← arguments.mapM (expressionText context · (fuel - 1))
            return s!"spec.{name}({commaSep arguments})"
          let some typeIds := typeInstantiationIds? instantiations
            | throw s!"spec.{name} requires one type argument"
          unless typeIds.size == 1 do
            throw s!"spec.{name} requires one type argument"
          let elementType ← typeText context.unit context.ns context.binders typeIds[0]!
          let arguments ← arguments.mapM (expressionText context · (fuel - 1))
          pure <| s!"spec.{name}::" ++ typeArguments elementType ++ s!"({commaSep arguments})"
      | _ => throw s!"operation `{repr operation}` is outside the current LeanerLang parser"
  | .block statements result => do
      let (statements, result) := if tailPosition && result.isNone then
          match statements.back? with
          | some trailing => match context.ns.expressions[trailing.index]? with
            | some { kind := .return_ #[value], .. } =>
                (statements.extract 0 (statements.size - 1), some value)
            | some trailingNode =>
                let unitResult := match context.unit.tables.types[expression.typeId.index]? with
                  | some .unit => true
                  | some (.tuple elements) => elements.isEmpty
                  | _ => false
                if !unitResult && trailingNode.typeId == expression.typeId then
                  (statements.extract 0 (statements.size - 1), some trailing)
                else (statements, result)
            | _ => (statements, result)
          | none => (statements, result)
        else (statements, result)
      if statements.isEmpty then
        if let some result := result then
          return ← expressionText context result (fuel - 1) tailPosition
        else return "()"
      else if let some (condition, loopBody, returned) :=
          if tailPosition && result.isNone && statements.size == 1 then
            returningWhile? context.ns statements[0]!
          else none then
        let condition ← expressionText context condition (fuel - 1)
        let loopBodyId := loopBody
        let loopBody ← expressionText context loopBodyId (fuel - 1) false true
        let head := s!"while {condition} do"
        let whileText := if loopBody.startsWith "do\n" then
            s!"{head}\n{loopBody.drop 3}"
          else s!"{head}\n{indent s!"{loopBody}{statementTerminator context.ns loopBodyId loopBody}"}"
        let returned ← expressionText context returned (fuel - 1) true
        return s!"do\n{indent (lines #[whileText, s!"return {returned}"])}"
      else if let some checked := checkedArithmeticShape? context statements result then
        let active ← checked.priorAssignments.foldlM (init := context)
          fun active (localId, valueId) => do
            let value ← expressionText active valueId (fuel - 1)
            let value := parenthesizedOperand active.ns valueId value
            pure { active with
              constantLocals := active.constantLocals.push (localId, value) }
        let arguments ← checked.arguments.mapM
          (expressionText active · (fuel - 1))
        return s!"core.prim.{← primitiveText checked.operation}({commaSep arguments})"
      else if let some value := forwardedValue? context.ns statements result then
        return ← expressionText context value (fuel - 1) true
      else if let some (condition, thenValue, thenAssignments,
          elseValue, elseAssignments) :=
          forwardedConditional? context.ns statements result then
        let renderForwarded (value : ExprId) (assignments : Array (LocalId × ExprId)) := do
          let active ← assignments.foldlM (init := context) fun active (localId, valueId) => do
            if compilerTemporaryLocal? active localId then
              let value ← expressionText active valueId (fuel - 1)
              let value := parenthesizedOperand active.ns valueId value
              pure { active with
                constantLocals := active.constantLocals.push (localId, value) }
            else pure active
          expressionText active value (fuel - 1) true
        let condition ← expressionText context condition (fuel - 1)
        let thenValue ← renderForwarded thenValue thenAssignments
        let elseValue ← renderForwarded elseValue elseAssignments
        return s!"if {condition} then {thenValue} else {elseValue}"
      else if let some (scrutinee, arms) := forwardedMatch? context.ns statements result then
        let scrutinee ← expressionText context scrutinee (fuel - 1)
        let arms ← arms.mapM fun (arm, value) => do
          let guard ← match arm.guard with
            | none => pure ""
            | some guard => pure s!" if {← expressionText context guard (fuel - 1)}"
          let value ← expressionText context value (fuel - 1) true
          pure s!"| {← bindingPatternText context arm.pattern}{guard} => ({value})"
        return s!"match {scrutinee} with\n{indent (lines arms)}"
      else
        let originalStatements := statements
        let (active, statements) ← statements.foldlM
          (init := (context, (#[] : Array String)))
          fun (active, rendered) statement => do
            let temporary? : Option (LocalId × ExprId) := do
              let statementNode ← active.ns.expressions[statement.index]?
              let .assign place value := statementNode.kind | none
              let targetLocal ← localPlace? active.ns place
              if compilerTemporaryLocal? active targetLocal then some (targetLocal, value) else none
            match temporary? with
            | some (targetLocal, valueId) =>
                let requiresStorage := requiresTemporaryStorage active targetLocal
                let value ← expressionText active valueId (fuel - 1)
                if active.temporaryLocals.contains targetLocal &&
                    (!numericTemporaryLocal? active targetLocal ||
                      requiresStorage) then
                  let name ← localName active targetLocal
                  pure ({ active with temporaryLocals :=
                    active.temporaryLocals.filter (· != targetLocal) },
                    rendered.push s!"let {name} := {value};")
                else
                  -- The text takes the temporary's place wherever it is read,
                  -- including under a field selection or an operator, so a
                  -- control-flow value carries the delimiters those need.
                  let value := parenthesizedOperand active.ns valueId value
                  pure ({ active with
                    constantLocals := active.constantLocals.push (targetLocal, value) },
                    rendered)
            | none =>
                let statementText ← expressionText active statement (fuel - 1) false true
                pure (active, rendered.push s!"{statementText}{
                  statementTerminator active.ns statement statementText}")
        -- Control leaves the block at a diverging statement: a result behind
        -- one is unreachable and has no source spelling.
        let result := if originalStatements.back?.any (isDivergingExpression context.ns)
          then none else result
        let entries ← if let some result := result then do
            let some resultNode := active.ns.expressions[result.index]?
              | throw s!"block result references missing expression {result.index}"
            let unitResult := match active.unit.tables.types[expression.typeId.index]? with
              | some .unit => true
              | some (.tuple elements) => elements.isEmpty
              | _ => false
            if tailPosition && unitResult &&
                resultNode.kind matches .loop .. then
              let resultText ← expressionText active result (fuel - 1) false
              let terminator := statementTerminator active.ns result resultText
              pure ((statements.push s!"{resultText}{terminator}").push
                "return ()")
            else if unitResult then
              if resultNode.kind matches .value .unit _ ||
                  resultNode.kind matches .operation (.primitive .tuple) _ #[] _ ||
                  resultNode.kind matches .block #[] none then
                pure statements
              else
                let resultText ← expressionText active result (fuel - 1) false statementPosition
                let terminator := statementTerminator active.ns result resultText
                pure (statements.push s!"{resultText}{terminator}")
            else
              -- A discarded block passes that position on to its own result,
              -- whose `return` then leaves the function rather than supplying
              -- a value nothing reads.
              -- Control does not come back from a result that never returns,
              -- so nothing consumes its value and it is written as it stands.
              let diverging := isDivergingExpression active.ns result
              let result ← expressionText active result (fuel - 1) tailPosition statementPosition
              let terminal := diverging ||
                resultNode.kind matches .return_ .. ||
                resultNode.kind matches .break_ .. ||
                resultNode.kind matches .continue_ .. ||
                resultNode.kind matches .throw_ ..
              let exitTerminator : String :=
                if statementPosition && resultNode.kind matches .return_ .. then ";" else ""
              pure (statements.push <|
                if terminal then s!"{result}{exitTerminator}" else s!"return {result}")
          else pure statements
        return blockText entries
  | .letDecl pattern none body =>
      -- Structurization introduces locals before the assignment which gives
      -- them their first value.  Keep those identities available to the
      -- sequential-block substitution below so neither rustc's return place
      -- nor source-level immutable `let` temporaries leak as undeclared names.
      let temporaryLocals ← bindingPatternLocalIds context pattern
      expressionText { context with
        temporaryLocals := context.temporaryLocals ++ temporaryLocals }
        body (fuel - 1) tailPosition
  | .letDecl pattern (some value) body =>
      if let some substitution ← temporaryConstant? context pattern value then
        return ← expressionText
          { context with constantLocals := context.constantLocals.push substitution }
          body (fuel - 1) tailPosition
      if let some holder := hiddenStorageBorrow? context pattern value then
        let borrowText ← expressionText context value (fuel - 1)
        return ← expressionText
          { context with constantLocals := context.constantLocals.push (holder, borrowText) }
          body (fuel - 1) tailPosition
      let rec declarationText (pattern : PatternId) (value : ExprId)
          (active : Context) (fuel : Nat) : Except String String := do
        let some patternNode := active.ns.patterns[pattern.index]?
          | throw "a local declaration references a missing pattern"
        if let .tuple #[child] := patternNode.kind then
          if let some { kind := .operation (.primitive .tuple) _ #[argument] _, .. } :=
              active.ns.expressions[value.index]? then
            return ← declarationText child argument active (fuel - 1)
        let localIds ← bindingPatternLocalIds active pattern
        let localMutabilities ← localIds.mapM fun localId => do
          let some localDecl := active.locals[localId.index]?
            | throw s!"a local declaration references missing local {localId.index}"
          pure localDecl.mutable
        let mutable := localMutabilities[0]?.getD false
        -- Bindings that disagree carry their own `mut`, as Rust spells it.
        let uniform := localMutabilities.all (· == mutable)
        let mutability := if uniform && mutable then "mut " else ""
        let mutableIds := (localIds.zip localMutabilities).filterMap fun entry =>
          if entry.2 then some entry.1 else none
        let active := if uniform then active else { active with mutableBindings := mutableIds }
        let valueText ← expressionText active value (fuel - 1)
        -- A declaration whose value ends with an indented block needs no
        -- separator: a semicolon there would attach to that block's own last
        -- entry rather than to the declaration around it.
        let terminator := if valueText.contains '\n' then "" else ";"
        pure s!"let {mutability}{← bindingPatternText active pattern} : \
          {← typeText active.unit active.ns active.binders patternNode.typeId} := \
          {valueText}{terminator}"
      let rec collectTail (active : Context) (node : ExprId) (fuel : Nat) :
          Except String (Array String × Option (ExprId × String)) := do
        if fuel == 0 then throw "cyclic local declaration reached the LeanerLang source backend"
        let some expression := active.ns.expressions[node.index]?
          | throw "a local declaration body references a missing expression"
        if (forRangeShape? active node).isSome then
          return (#[], some (node, ← expressionText active node (fuel - 1) tailPosition))
        match expression.kind with
        | .letDecl pattern (some value) body =>
            if let some substitution ← temporaryConstant? active pattern value then
              collectTail { active with
                constantLocals := active.constantLocals.push substitution } body (fuel - 1)
            else if let some holder := hiddenStorageBorrow? active pattern value then
              let borrowText ← expressionText active value (fuel - 1)
              collectTail { active with
                constantLocals := active.constantLocals.push (holder, borrowText) }
                body (fuel - 1)
            else
              let declaration ← declarationText pattern value active fuel
              let (entries, result) ← collectTail active body (fuel - 1)
              pure (#[declaration] ++ entries, result)
        | .letDecl _ none body => collectTail active body (fuel - 1)
        | .block statements result =>
            let rendered ← statements.mapM fun statement => do
              let statementText ← expressionText active statement (fuel - 1) false true
              pure s!"{statementText}{statementTerminator active.ns statement statementText}"
            -- Control leaves the block at a diverging statement: the block
            -- result behind it is unreachable and has no source spelling.
            if statements.back?.any (isDivergingExpression active.ns) then
              pure (rendered, none) else
            match result with
            | none => pure (rendered, none)
            | some result =>
                let (entries, value) ← collectTail active result (fuel - 1)
                pure (rendered ++ entries, value)
        | _ => pure (#[], some (node, ← expressionText active node (fuel - 1) tailPosition))
      let declaration ← declarationText pattern value context fuel
      let (entries, result) ← collectTail context body (fuel - 1)
      let entries := #[declaration] ++ entries
      let unitResult := match context.unit.tables.types[expression.typeId.index]? with
        | some .unit => true
        | some (.tuple elements) => elements.isEmpty
        | _ => false
      -- A `return` closing a chain that does not itself yield that value
      -- leaves the function, and its semicolon says so.
      let exitTerminator (node : ExprId) : String :=
        if statementPosition && (context.ns.expressions[node.index]?).any
            (fun candidate => candidate.kind matches .return_ ..)
          then ";" else ""
      let entries := match result with
        -- A `let` chain is itself an expression in LIR.  When its result is
        -- Unit, the source sequence already has the required result: retain a
        -- final effect, but do not manufacture a source-level `return ()`.
        | some (node, result) =>
            if tailPosition && result == "()" then entries
            else if unitResult then
              if result == "()" then entries
              else
                -- The chain's last entry is a statement of the block, not its
                -- value: the separator is what says so when the text is read
                -- back, and a `return` keeps the one that leaves the function.
                let separator := if (context.ns.expressions[node.index]?).any
                    (fun candidate => candidate.kind matches .return_ ..) then
                    exitTerminator node
                  else statementTerminator context.ns node result
                entries.push s!"{result}{separator}"
            else if isDivergingExpression context.ns node then
              entries.push s!"{result}{exitTerminator node}"
            else entries.push s!"return {result}"
        | none => entries
      if entries.isEmpty then pure "()" else
        pure (blockText entries)
  | .ifElse condition thenBranch elseBranch =>
      let condition ← expressionText context condition (fuel - 1)
      -- A block-valued condition spans lines; only a parenthesized form keeps
      -- the following `then` attached to this `if`.
      let condition := if condition.contains '\n' then s!"({condition})" else condition
      if elseBranch.isNone then
        let thenBranch ← expressionText context thenBranch (fuel - 1) tailPosition statementPosition
        let thenBranch := if thenBranch.contains '\n' then s!"({thenBranch})" else thenBranch
        return s!"if {condition} then {thenBranch}"
      let elseBranch := elseBranch.get!
      if condition == "true" then
        return ← expressionText context thenBranch (fuel - 1) tailPosition
      if condition == "false" then
        return ← expressionText context elseBranch (fuel - 1) tailPosition
      let thenBranch ← expressionText context thenBranch (fuel - 1) tailPosition statementPosition
      let elseBranch ← expressionText context elseBranch (fuel - 1) tailPosition statementPosition
      -- A branch that leaves the function is a statement of its own, not this
      -- `if`'s value. Spelling it as a block keeps the semicolon that tells an
      -- exit from a branch value, which a branch position cannot carry. An
      -- empty counterpart needs none of this: the branch is then the whole
      -- statement, as an `if` without an `else`.
      let exitBranch (text sibling : String) : String :=
        if !tailPosition && sibling != "()" && text.startsWith "return " &&
            !text.contains '\n' then
          s!"do\n{indent s!"{text};"}"
        else text
      let thenText := exitBranch thenBranch elseBranch
      let elseBranch := exitBranch elseBranch thenBranch
      let thenBranch := thenText
      let groupMultiline (value : String) :=
        if value.contains '\n' then s!"({value})" else value
      pure s!"if {condition} then \
        {groupMultiline thenBranch} else {groupMultiline elseBranch}"
  | .match_ scrutinee arms =>
      unless !arms.isEmpty do
        throw "a match expression has no arms"
      let scrutinee ← expressionText context scrutinee (fuel - 1)
      let arms ← arms.mapM fun arm => do
        let guard ← match arm.guard with
          | none => pure ""
          | some guard => pure s!" if {← expressionText context guard (fuel - 1)}"
        let body ← expressionText context arm.body (fuel - 1) tailPosition
        pure s!"| {← bindingPatternText context arm.pattern}{guard} => {body}"
      pure s!"match {scrutinee} with\n{indent (lines arms)}"
  | .loop label body =>
      unless label.isNone do
        throw "labeled loops are outside the current LeanerLang parser"
      let rec isLoopExit (node : ExprId) (fuel : Nat) : Bool :=
        if fuel == 0 then false else
          match context.ns.expressions[node.index]? with
          | some { kind := .break_ 0 none, .. } => true
          | some { kind := .block #[] (some result), .. }
          | some { kind := .block #[result] none, .. } =>
              isLoopExit result (fuel - 1)
          | _ => false
      let rec guardedLoop? (node : ExprId) (fuel : Nat) : Option (ExprId × ExprId) := do
        guard (fuel > 0)
        let bodyNode ← context.ns.expressions[node.index]?
        match bodyNode.kind with
        | .ifElse condition thenBranch (some elseBranch) =>
            if isLoopExit elseBranch (fuel - 1) then
              some (condition, thenBranch)
            else none
        | .block #[] (some result)
        | .block #[result] none => guardedLoop? result (fuel - 1)
        | _ => none
      let whileShape := guardedLoop? body (context.ns.expressions.size + 1)
      match whileShape with
      | some (condition, loopBody) =>
        let (invariants, condition) := match context.ns.expressions[condition.index]? with
          | some { kind := .block statements (some result), .. } =>
              let onlySpecifications := statements.all fun statement =>
                match context.ns.expressions[statement.index]? with
                | some { kind := .spec .., .. } => true
                | _ => false
              if onlySpecifications then (statements, result) else (#[], condition)
          | _ => (#[], condition)
        let invariantTexts ← invariants.mapM fun invariant =>
          expressionText context invariant (fuel - 1) false
        let condition ← expressionText context condition (fuel - 1)
        let loopBodyId := loopBody
        let loopBody ← expressionText context loopBodyId (fuel - 1)
        let head := s!"while {condition} do"
        let whileText := if loopBody.startsWith "do\n" then
            s!"{head}\n{loopBody.drop 3}"
          else s!"{head}\n{indent s!"{loopBody}{statementTerminator context.ns loopBodyId loopBody}"}"
        if invariantTexts.isEmpty then pure whileText
        else if tailPosition then
          pure s!"do\n{indent (lines ((#[whileText] ++ invariantTexts).push "return ()"))}"
        else
          pure (lines (#[whileText] ++ invariantTexts))
      | none => pure s!"loop {← expressionText context body (fuel - 1)}"
  | .break_ nest value =>
      unless nest == 0 do
        throw "nonlocal breaks are outside the current LeanerLang parser"
      match value with
      | none => pure "break"
      | some value => pure s!"break {← expressionText context value (fuel - 1)}"
  | .continue_ nest =>
      unless nest == 0 do
        throw "nonlocal continues are outside the current LeanerLang parser"
      pure "continue"
  | .assign place value =>
      pure s!"{← placeText context (fun index => expressionText context index (fuel - 1)) place} := \
        {← expressionText context value (fuel - 1)}"
  | .assignPattern pattern value => do
      let some patternNode := context.ns.patterns[pattern.index]?
        | throw s!"pattern assignment references missing pattern {pattern.index}"
      if let .variable localId := patternNode.kind then
        return s!"{← localName context localId} := {
          ← expressionText context value (fuel - 1)}"
      let patternType ← typeText context.unit context.ns context.binders patternNode.typeId
      pure s!"assign_pattern[{patternType}]({← bindingPatternText context pattern}, \
        {← expressionText context value (fuel - 1)})"
  | .quantifier kind binders _triggers condition body =>
      unless !binders.isEmpty do
        throw "a quantifier must contain at least one binder"
      let kind ← match kind with
        | .forall => pure "∀"
        | .exists => pure "∃"
        | .choose | .chooseMin | .profile _ =>
            throw "this quantifier kind is outside the current LeanerLang parser"
      let logicalLocals ← binders.foldlM (init := context.logicalLocals)
        fun locals binder => do
          pure (locals ++ (← bindingPatternLocalIds context binder.pattern))
      let binderTexts ← binders.mapM fun binder => do
        -- A binder over a whole type's domain prints as the annotated
        -- binder `x : T`, the legacy Move spelling for `forall x: T`.
        let typeBinder? ← match context.ns.expressions[binder.domain.index]? with
          | some node =>
              match node.kind with
              | .operation (.specification .typeDomain) #[.typeArg element] #[] _ =>
                  pure (some (← typeText context.unit context.ns context.binders element.typeId))
              | _ => pure none
          | none => pure none
        match typeBinder? with
        | some element =>
            pure s!"{← bindingPatternText context binder.pattern} : {element}"
        | none =>
            pure s!"{← bindingPatternText context binder.pattern} in \
              {← expressionText context binder.domain (fuel - 1)}"
      let bodyContext := { context with logicalLocals }
      let body ← expressionText bodyContext body (fuel - 1)
      let body ← match condition with
        | none => pure body
        | some condition => do
            let condition ← expressionText bodyContext condition (fuel - 1)
            pure <| if kind == "∀" then s!"({condition} ==> {body})"
              else s!"({condition} && {body})"
      -- Match the legacy Move printer: triggers guide the prover but do not
      -- alter the source proposition, so canonical LeanerLang omits them.
      pure s!"{kind} ({"; ".intercalate binderTexts.toList}), {body}"
  | .spec block =>
      unless block.pragmas.isEmpty && block.frame.isNone do
        throw "in-body specification pragmas and frames are outside the current LeanerLang parser"
      -- Source-level specification blocks project all enclosing integer
      -- locals into the mathematical integer domain.  Preserve that surface
      -- convention: fixed-width suffixes and the LIR's explicit projection
      -- operations are implementation detail here.
      let specContext := {
        context with
        specification := true
        logicalLocals := context.locals.map (fun declaration => declaration.id) }
      let conditions ← block.conditions.mapM fun condition => do
        unless condition.properties.isEmpty && condition.auxiliary.isEmpty do
          throw s!"in-body specification condition `{repr condition.kind}` with \
            {condition.properties.size} properties and {condition.auxiliary.size} auxiliary \
            expressions is outside the current LeanerLang parser"
        let memberPrefix ← match condition.kind with
          | .letPre name | .letPost name =>
              pure s!"let {sourceIdentifier name} := "
          | .assertion => pure "assert "
          | .assumption => pure "assume "
          | .loopInvariant => pure "invariant "
          | kind => throw s!"in-body specification condition `{repr kind}` is outside the \
              current LeanerLang parser"
        pure s!"{memberPrefix}{← expressionText specContext condition.expression (fuel - 1)};"
      pure s!"spec do\n{indent (lines conditions)}"
  | .return_ values =>
      let value ← match values.toList with
        | [value] => expressionText context value (fuel - 1)
        | [] => pure "()"
        | _ => pure s!"({commaSep (← values.mapM (expressionText context · (fuel - 1)))})"
      if tailPosition then pure value else
        -- A returned value that spans lines is delimited: the statement
        -- separator after it must belong to this `return`, not to the last
        -- entry of a block the value happens to end with.
        let value := if value.contains '\n' then s!"({value})" else value
        pure s!"return {value}"
  | .throw_ kind values =>
      if context.specification then return "abort()"
      let kind ← failureText kind
      pure s!"{kind}({commaSep (← values.mapM (expressionText context · (fuel - 1)))})"

private def flagAttributeName : Attribute → Except String String
  | .assign name (.constant (.bool true)) _ => pure name
  | .call name #[] _ => pure name
  | attr => throw s!"attribute `{repr attr}` is not a boolean flag"

private def pragmaText : Attribute → Except String String
  | .assign name (.constant (.bool true)) _ => pure name
  | .assign name (.constant (.bool false)) _ => pure s!"{name} = false"
  | .assign name (.constant value) _ => do pure s!"{name} = {← constText value}"
  | .assign name (.name none value) _ => pure s!"{name} = {sourceIdentifier value}"
  | .call name #[] _ => pure name
  | attr => throw s!"pragma attribute `{repr attr}` has no canonical LeanerLang spelling"

private def pragmaName : Attribute → String
  | .assign name .. | .call name .. => name

private def conditionText (context : Context) (condition : Condition) : Except String String := do
  let (keyword, bindingName?) ← match condition.kind with
    | .letPre name => pure ("let_pre", some name)
    | .letPost name => pure ("let_post", some name)
    | .requires => pure ("requires", none)
    | .ensures => pure ("ensures", none)
    | .abortsIf => pure ("aborts_if", none)
    | .structInvariant => pure ("invariant", none)
    | kind => throw s!"condition kind `{repr kind}` is outside the current LeanerLang parser"
  let properties ← condition.properties.mapM flagAttributeName
  let properties := if properties.isEmpty then "" else s!"[{commaSep properties}] "
  let expression ← expressionText context condition.expression
    (context.ns.expressions.size + 1)
  let auxiliary ← match condition.kind, condition.auxiliary.toList with
    | .abortsIf, [] => pure ""
    | .abortsIf, [("abortCode", code)] =>
        pure s!" with {← expressionText context code (context.ns.expressions.size + 1)}"
    | _, [] => pure ""
    | _, _ => throw "condition auxiliary expressions are outside the current LeanerLang parser"
  let bindingName := bindingName?.map (sourceIdentifier · ++ " := ") |>.getD ""
  pure s!"{keyword} {properties}{bindingName}{expression}{auxiliary};"

/-- Print ordered contract conditions while extending the logical scope after
each contract binding. Imported Move LIR retains the binding's physical local
type, but every following specification expression observes its projected
logical type. -/
private def conditionTexts (initial : Context) (conditions : Array Condition) :
    Except String (Array String) := do
  let mut context := initial
  let mut entries := #[]
  for condition in conditions do
    entries := entries.push (← conditionText context condition)
    let bindingName? := match condition.kind with
      | .letPre name | .letPost name => some name
      | _ => none
    if let some name := bindingName? then
      let localId? := context.locals.foldl (init := none) fun found localDecl =>
        if localDecl.name == name then some localDecl.id else found
      let some localId := localId?
        | throw s!"contract binding `{name}` has no declaration local"
      context := { context with logicalLocals := context.logicalLocals.push localId }
  pure entries

private def frameTexts (context : Context) (contract : FunctionContract) :
    Except String (Array String) := do
  let mut entries := #[]
  for expression in contract.modifies do
    entries := entries.push s!"modifies {← expressionText context expression
      (context.ns.expressions.size + 1)};"
  for type in contract.reads do
    entries := entries.push s!"reads {← typeText context.unit context.ns context.binders type.typeId};"
  if contract.modifiesAll then entries := entries.push "modifies *;"
  if contract.readsAll then entries := entries.push "reads *;"
  pure entries

private def isMoveVariantsProperty : ProfileValue → Bool
  | { profile := .move, tag := "struct.variants", payload := "" } => true
  | _ => false

private def nominalContractText? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : LeanerIR.StructDecl) (name : String) :
    Except String (Option String) := do
  let contract := declaration.contract
  unless !contract.hasFrame && contract.modifies.isEmpty && contract.reads.isEmpty &&
      !contract.modifiesAll && !contract.readsAll do
    throw "nominal frames are outside the current LeanerLang printer"
  unless contract.conditions.all (·.kind == .structInvariant) do
    throw "non-invariant nominal conditions are outside the current LeanerLang printer"
  if contract.conditions.isEmpty && contract.pragmas.isEmpty then return none
  let context : Context := {
    unit, ns, locals := declaration.locals
    localNames := declarationLocalNames ns declaration.locals 0 none
    binders := declaration.generics
    nominalOwner := some declaration.name, specification := true }
  let entries ← conditionTexts context contract.conditions
  let pragmas ← contract.pragmas.mapM pragmaText
  let entries := entries ++ pragmas.map fun pragma => s!"pragma {pragma};"
  pure <| some s!"spec {name} where\n{indent (lines entries)}"

private def fieldText (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (binders : Array LeanerIR.GenericBinder)
    (field : LeanerIR.FieldDecl) : Except String String := do
  pure s!"{← nameAt unit field.name} : {← typeText unit ns binders field.type.typeId}"

private def nominalText (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : LeanerIR.StructDecl) :
    Except String String := do
  unless declaration.properties.all isMoveVariantsProperty do
    throw "nominal profile properties are outside the current LeanerLang printer"
  unless declaration.attributes.isEmpty do
    throw "nominal attributes are outside the current LeanerLang printer"
  let name ← nameAt unit declaration.name
  let binders := " ".intercalate (← declaration.generics.mapM (binderText unit)).toList
  let binders := if binders.isEmpty then "" else " " ++ binders
  let abilities := abilitiesText declaration.abilities
  let isEnum := !declaration.variants.isEmpty || declaration.properties.any isMoveVariantsProperty
  let declarationText ← if !isEnum then
    let fields ← declaration.fields.mapM (fieldText unit ns declaration.generics)
    pure s!"struct {name}{binders}{abilities} where\n{indent (lines fields)}"
  else
    unless declaration.fields.isEmpty do
      throw s!"nominal `{name}` has both top-level fields and variants"
    let variants ← declaration.variants.mapM fun variant => do
      let fields ← variant.fields.mapM (fieldText unit ns declaration.generics)
      let fields := if fields.isEmpty then "" else s!" ({commaSep fields})"
      let discriminant := variant.discriminant.map (s!" = {·}") |>.getD ""
      pure s!"| {← nameAt unit variant.name}{fields}{discriminant}"
    pure s!"enum {name}{binders}{abilities} where\n{indent (lines variants)}"
  match ← nominalContractText? unit ns declaration name with
  | none => pure declarationText
  | some contract => pure s!"{declarationText}\n\n{contract}"

private structure PrintedModifiers where
  visibility : LeanerLang.Visibility := .private_
  isDeprecated : Bool := false
  isView : Bool := false
  isEntry : Bool := false
  isNative : Bool := false
  isOpaque : Bool := false

private def visibilityOf (value : String) : Except String LeanerLang.Visibility :=
  match value with
  | "private" | "visibility.private" => pure .private_
  | "public" | "visibility.public" => pure .public_
  | "package" | "visibility.package" => pure .package
  | "friend" | "visibility.friend" => pure .friend
  | _ => throw s!"unknown function visibility `{value}`"

private def mergeVisibility (current : Option LeanerLang.Visibility)
    (next : LeanerLang.Visibility) : Except String (Option LeanerLang.Visibility) :=
  match current with
  | none => pure (some next)
  | some previous =>
      if previous == next then pure current
      else throw "function carries conflicting visibility metadata"

private def functionModifiers
    (declaration : LeanerIR.FunctionDecl FunctionBody) : Except String PrintedModifiers := do
  let mut result : PrintedModifiers := {}
  let mut visibility : Option LeanerLang.Visibility := none
  for value in declaration.profileData do
    unless value.profile == declaration.profile do
      throw s!"function profile metadata `{value.tag}` belongs to a different profile"
    if value.tag.startsWith "visibility." then
      visibility ← mergeVisibility visibility (← visibilityOf value.tag)
    else match value.tag with
      | "function.regular" => pure ()
      | "function.entry" => result := { result with isEntry := true }
      | "function.native" => result := { result with isNative := true }
      -- Move classifies both tags as frontend-only provenance. Canonical
      -- LeanerLang retains the ordinary function and prefix-call semantics.
      | "function.inlineRetained" | "function.receiver" => pure ()
      | tag => throw s!"function profile property `{tag}` has no canonical LeanerLang spelling"
  for attr in declaration.attributes do
    match attr with
    | .assign "visibility" (.qualifiedName value) _ =>
        visibility ← mergeVisibility visibility (← visibilityOf value)
    | .call "entry" arguments _ =>
        unless arguments.isEmpty do throw "the `entry` modifier attribute has arguments"
        result := { result with isEntry := true }
    | .call "native" arguments _ =>
        unless arguments.isEmpty do throw "the `native` modifier attribute has arguments"
        result := { result with isNative := true }
    | .call "opaque" arguments _ =>
        unless arguments.isEmpty do throw "the `opaque` modifier attribute has arguments"
        result := { result with isOpaque := true }
    | .call "deprecated" arguments _ =>
        unless arguments.isEmpty do throw "the `deprecated` modifier attribute has arguments"
        result := { result with isDeprecated := true }
    | .call "view" arguments _ =>
        unless arguments.isEmpty do throw "the `view` modifier attribute has arguments"
        result := { result with isView := true }
    | .call "bytecode_instruction" arguments _ =>
        unless arguments.isEmpty do
          throw "the `bytecode_instruction` provenance attribute has arguments"
    | attr => throw s!"function attribute `{repr attr}` has no canonical LeanerLang spelling yet"
  result := { result with visibility := visibility.getD .private_ }
  if result.isNative && result.isOpaque then
    throw "a function cannot be both native and opaque"
  match declaration.body with
  | .structured _ =>
      if result.isNative || result.isOpaque then
        throw "a native or opaque function unexpectedly has an executable body"
      pure result
  | .absent =>
      pure <| if result.isNative then result else { result with isOpaque := true }

private def modifiersText (modifiers : PrintedModifiers) : String :=
  let values := #[]
  let values := match modifiers.visibility with
    | .private_ => values
    | .public_ => values.push "public"
    | .package => values.push "package"
    | .friend => values.push "friend"
  let values := if modifiers.isDeprecated then values.push "deprecated" else values
  let values := if modifiers.isView then values.push "view" else values
  let values := if modifiers.isEntry then values.push "entry" else values
  let values := if modifiers.isNative then values.push "native" else values
  let values := if modifiers.isOpaque then values.push "opaque" else values
  if values.isEmpty then "" else " ".intercalate values.toList ++ " "

private def isFalseExpression (ns : ValidatedNamespace) (id : ExprId) : Bool :=
  (ns.expressions[id.index]?).any fun expression => expression.kind == .value (.bool false)

private def contractText? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl FunctionBody) (name : String) :
    Except String (Option String) := do
  let contract := declaration.contract
  let declarationPragmas ← declaration.pragmas.mapM pragmaText
  let contractPragmas ← contract.pragmas.mapM pragmaText
  let namespacePragmas ← ns.pragmas.mapM pragmaText
  -- A pragma the contract sets shadows the namespace's setting of the same
  -- name, whatever value each one carries.
  let contractNames := contract.pragmas.map pragmaName
  let resolvedPragmas := contractPragmas ++
    (ns.pragmas.zip namespacePragmas).filterMap fun (inherited, text) =>
      if contractNames.contains (pragmaName inherited) then none else some text
  unless declarationPragmas == resolvedPragmas do
    throw s!"function pragmas are not the resolved contract and namespace pragmas: \
      the declaration carries {declarationPragmas.toList}, the contract and namespace resolve \
      to {resolvedPragmas.toList}"
  let mut conditions := contract.conditions
  if declaration.profile == .rust && conditions.size == 1 then
    let condition := contract.conditions[0]!
    if condition.kind == .abortsIf && condition.properties.isEmpty &&
        condition.auxiliary.isEmpty && isFalseExpression ns condition.expression then
      conditions := #[]
  if conditions.isEmpty && contractPragmas.isEmpty then return none
  let context : Context := {
    unit, ns, locals := declaration.locals
    localNames := declarationLocalNames ns declaration.locals
      declaration.signature.parameters.size (functionBodyRoot? declaration.body)
    binders := declaration.signature.generics
    specification := true
    logicalLocals := declaration.locals.map (fun localDecl => localDecl.id) }
  let entries ← conditionTexts context conditions
  let entries := entries ++ (← frameTexts context contract)
  let entries := entries ++ contractPragmas.map fun pragma => s!"pragma {pragma};"
  pure <| some s!"spec {name} where\n{indent (lines entries)}"

private def constantText (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : LeanerIR.ConstantDecl) : Except String String := do
  unless declaration.profileData.isEmpty && declaration.attributes.isEmpty do
    throw "constant metadata has no canonical LeanerLang spelling yet"
  let some expression := ns.expressions[declaration.value.index]?
    | throw "constant initializer is missing from the expression arena"
  let .value value _ := expression.kind
    | throw "non-literal constant initializers are outside the current LeanerLang printer"
  let value ← match value with
    | .integer value => integerLiteralText unit ns false declaration.type.typeId value
    | value => constText value
  pure s!"const {← nameAt unit declaration.name} : {← typeText unit ns #[] declaration.type.typeId} := \
    {value}"

/-- The Move exchange frontend reports the complete named-address environment,
including standard aliases that the module never mentions. Names are already
resolved in checked LIR, so these frontend-only entries are not part of the
canonical program projection. A module's own address alias is likewise already
spelled by the middle segment of its structural `address::alias::module` path;
accept it only when the redundant metadata agrees with that path. Metadata
that can affect a declaration remains an explicit capability error. -/
private def isResolvedMoveEnvironment (unit : ValidatedUnit) (ns : ValidatedNamespace) :
    ProfileValue → Bool
  | { profile := .move, tag := "metadata.namedAddress", .. } => true
  | { profile := .move, tag := "metadata.addressAlias", payload } =>
      match unit.tables.namespaces[ns.identity.index]? with
      | some { segments := #[_, alias, _] } => !alias.isEmpty && payload == alias
      | _ => false
  | _ => false

private def stringArrayPayload (payload : String) : Except String (Array String) := do
  let json ← Lean.Json.parse payload
  match json with
  | .arr fields => fields.mapM fun
      | .str value => pure value
      | _ => throw "profile payload field is not a string"
  | _ => throw "profile payload is not an array"

/-- The `friend` module a Move namespace grants privileged access to, spelled
the way the source declared its address: by alias when one was recorded. -/
private def friendPath? (owner : Array String) : ProfileValue → Except String (Option String)
  | { profile := .move, tag := "metadata.friend", payload } => do
      let #[address, aliasPayload, name] ← stringArrayPayload payload
        | throw "friend metadata must contain an address, alias, and name"
      let alias? ← match ← stringArrayPayload aliasPayload with
        | #[] => pure none
        | #[alias] => pure (some alias)
        | _ => throw "friend metadata carries more than one address alias"
      -- Move friends live at the declaring module's own address, so an alias
      -- there names that address and re-imports to the same reference.
      match alias? with
      | some alias => pure (some s!"{sourceIdentifier alias}::{sourceIdentifier name}")
      | none =>
          unless owner[0]? == some address do
            throw "a friend module at another address has no LeanerLang spelling"
          pure (some s!"{address}::{sourceIdentifier name}")
  | _ => pure none

/-- A frontend omission cannot be reconstructed from LIR, but it must not be
silently erased from generated source. Keep it visible as valid Lean comments
while printing all declarations that did cross the frontend boundary. -/
private def skippedDeclarationComment? : ProfileValue → Except String (Option String)
  | { profile := .move, tag := "metadata.skipped", payload } => do
      let #[name, reason] ← stringArrayPayload payload
        | throw "skipped-declaration metadata must contain a name and reason"
      let message := s!"unsupported Move declaration `{name}`: {reason}"
      pure <| some <| lines <| message.splitOn "\n" |>.toArray.map ("-- " ++ ·)
  | _ => pure none

/-- Move's `native` and `uninterpreted` flags are frontend provenance for a
bodyless logical symbol. The profile-general `opaque spec fun` form preserves
the semantic distinction that matters in canonical LeanerLang source. -/
private def isOpaqueSpecMetadata : ProfileValue → Bool
  | { profile := .move, tag := "specFunction.native", .. }
  | { profile := .move, tag := "specFunction.uninterpreted", .. } => true
  | _ => false

/-- Move's `uses_old` flag records that a specification function, or one of its
callees, reads a pre-state — a fact the Move backend's specification rewriter
derives again from `old(...)` and `&mut` parameters. Derived provenance needs
no source spelling; it is recomputed, not erased. -/
private def isDerivedSpecMetadata : ProfileValue → Bool
  | { profile := .move, tag := "specFunction.usesOld", payload } => payload.isEmpty
  | _ => false

private def canonicalProfileConfig : Profile → Option ProfileConfig
  | .move => some ProfileName.move.config
  | .rust => some ProfileName.rust.config
  | .extension _ => none

private def sourceOrderKey (unit : ValidatedUnit) (loc : LocId) (fallback : Nat) :
    Nat × Nat × Nat :=
  match unit.tables.locations[loc.index]?.bind (·.primary) with
  | some range => (range.file.index, range.startByte, fallback)
  | none => (Nat.succ unit.tables.files.size, fallback, fallback)

private def documentationLines (documentation : String) : List String :=
  let rawLines := documentation.trimAscii.toString.splitOn "\n"
  let leadingWhitespace (line : String) :=
    line.toList.takeWhile (fun character => character == ' ' || character == '\t') |>.length
  let continuation := rawLines.drop 1
  let margins := continuation.filterMap fun line =>
    if line.trimAscii.isEmpty then none else some (leadingWhitespace line)
  let margin := margins.foldl Nat.min (margins.head?.getD 0)
  match rawLines with
  | [] => []
  | first :: rest => first.trimAscii.toString :: rest.map fun line =>
      if line.trimAscii.isEmpty then ""
      else String.ofList (line.toList.drop margin) |>.trimAsciiEnd.toString

private def documentationText (documentation : String) : String :=
  if documentation.trimAscii.isEmpty then "" else
    match documentationLines documentation with
    | [line] => s!"/-- {line} -/"
    | documentation => "/--\n" ++ "\n".intercalate documentation ++ "\n-/"

private def sourceCommentText (comment : LeanerIR.Comment) : String :=
  let text := comment.text
  if comment.isDoc then
    let body :=
      if text.startsWith "--/" then text.drop 3
      else if text.startsWith "/--" || text.startsWith "/-!" then
        (text.drop 3).dropEnd 2
      else text
    documentationText body.toString
  else if text.startsWith "//" then "--" ++ text.drop 2
  else if text.startsWith "--" then text
  else if text.startsWith "/-" && text.endsWith "-/" then
    lines <| (((text.drop 2).dropEnd 2).toString.splitOn "\n").toArray.map fun line =>
      "-- " ++ line.trimAscii.toString
  else if text.startsWith "/*" && text.endsWith "*/" then
    lines <| (((text.drop 2).dropEnd 2).toString.splitOn "\n").toArray.map fun line =>
      "-- " ++ line.trimAscii.toString
  else "-- " ++ text

private def documented (documentation body : String) : String :=
  let documentation := documentationText documentation
  if documentation.isEmpty then body else documentation ++ "\n" ++ body

private def nominalDocumentation (declaration : LeanerIR.StructDecl) : String :=
  let docs := #[declaration.doc] ++ declaration.fields.map (fun field => field.doc) ++
    declaration.variants.flatMap fun variant =>
      variant.fields.map (fun field => field.doc)
  "\n".intercalate <| docs.filter (fun doc => !doc.trimAscii.isEmpty) |>.toList

private def functionText (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : LeanerIR.FunctionDecl FunctionBody) : Except String String := do
  unless ns.profile == some declaration.profile do
    throw "function profile differs from its namespace profile"
  unless declaration.signature.predicates.isEmpty do
    throw "function predicates are outside the current LeanerLang printer"
  let modifiers ← functionModifiers declaration
  let name ← nameAt unit declaration.name
  let binders := " ".intercalate
    (← declaration.signature.generics.mapM (binderText unit)).toList
  let binders := if binders.isEmpty then "" else " " ++ binders
  let parameters ← declaration.signature.parameters.mapM fun parameter => do
    let mutability := if parameter.mutable then "mut " else ""
    pure s!"{mutability}{sourceIdentifier parameter.name} : \
      {← typeText unit ns declaration.signature.generics parameter.typeUse.typeId}"
  let result ← match declaration.signature.results.toList with
    | [] => pure "Unit"
    | [result] => typeText unit ns declaration.signature.generics result.typeId
    | results => pure s!"({commaSep (← results.toArray.mapM fun result =>
        typeText unit ns declaration.signature.generics result.typeId)})"
  let declarationPrefix := s!"{modifiersText modifiers}fun {name}{binders}"
  let compactSignature := s!"{declarationPrefix} ({commaSep parameters}) -> {result}"
  let signature :=
    if fitsIndentedLine compactSignature then
      compactSignature
    else if parameters.isEmpty then
      s!"{declarationPrefix} () ->\n{indent result}"
    else
      declarationPrefix ++ " (\n" ++ indent (",\n".intercalate parameters.toList) ++
        s!"\n) -> {result}"
  let functionText ← match declaration.body with
    | .absent => pure signature
    | .structured root =>
        let context : Context := {
          unit, ns, locals := declaration.locals
          localNames := declarationLocalNames ns declaration.locals
            declaration.signature.parameters.size (some root)
          binders := declaration.signature.generics }
        let body ← expressionText context root (ns.expressions.size + 1) true
        let compact := s!"{signature} := {body}"
        if !signature.contains '\n' && body.startsWith "do\n" &&
            fitsIndentedLine s!"{signature} := do" then
          pure compact
        else if !signature.contains '\n' && fitsIndentedLine compact then
          pure compact
        else
          pure s!"{signature} :=\n{indent body}"
  match ← contractText? unit ns declaration name with
  | none => pure functionText
  | some contract => pure s!"{functionText}\n\n{contract}"

private def specFunctionText (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : LeanerIR.SpecFunctionDecl) : Except String String := do
  unless ns.profile == some declaration.profile do
    throw "specification-function profile differs from its namespace profile"
  unless declaration.signature.predicates.isEmpty do
    throw "specification-function predicates are outside the current LeanerLang printer"
  unless declaration.signature.parameters.all fun parameter => !parameter.mutable do
    throw "mutable specification-function parameters have no LeanerLang spelling"
  let contract := declaration.contract
  unless contract.conditions.isEmpty && contract.modifies.isEmpty && contract.reads.isEmpty &&
      !contract.hasFrame && !contract.modifiesAll && !contract.readsAll &&
      contract.pragmas.isEmpty do
    throw "specification-function contracts are outside the current LeanerLang printer"
  match declaration.body with
  | some _ => unless declaration.profileData.all isDerivedSpecMetadata do
      throw s!"a defined specification function carries profile metadata with no canonical \
        spelling: {(declaration.profileData.map (·.tag)).toList}"
  | none => unless declaration.profileData.all fun value =>
      isOpaqueSpecMetadata value || isDerivedSpecMetadata value do
      throw s!"an opaque specification function carries profile metadata with no canonical \
        spelling: {(declaration.profileData.map (·.tag)).toList}"
  let name ← nameAt unit declaration.name
  let binders := " ".intercalate
    (← declaration.signature.generics.mapM (binderText unit)).toList
  let binders := if binders.isEmpty then "" else " " ++ binders
  let parameters ← declaration.signature.parameters.mapM fun parameter => do
    pure s!"{sourceIdentifier parameter.name} : \
      {← typeText unit ns declaration.signature.generics parameter.typeUse.typeId}"
  let result ← match declaration.signature.results.toList with
    | [] => pure "Unit"
    | [result] => typeText unit ns declaration.signature.generics result.typeId
    | _ => throw "multiple specification-function results are outside the current LeanerLang parser"
  let declarationPrefix := if declaration.body.isNone then "opaque spec fun" else "spec fun"
  let signature := s!"{declarationPrefix} {name}{binders} ({commaSep parameters}) : {result}"
  match declaration.body with
  | none => pure signature
  | some root =>
      let context : Context := {
        unit, ns, locals := declaration.locals
        localNames := declarationLocalNames ns declaration.locals
          declaration.signature.parameters.size (some root)
        binders := declaration.signature.generics
        specification := true }
      let body ← expressionText context root (ns.expressions.size + 1) true
      let compact := s!"{signature} := {body}"
      if fitsIndentedLine compact then pure compact
      else pure s!"{signature} :=\n{indent body}"

private structure SourceOrderedText where
  file : Nat
  start : Nat
  /-- Documentation at the same source start belongs before its declaration. -/
  priority : Nat
  fallback : Nat
  text : String

/-- Recover a semantic LeanerLang spelling before canonical document layout.
This stage is deliberately private: callers must not observe a second,
unformatted source representation. -/
private def renderSemanticNamespace (unit : ValidatedUnit)
    (identity : NamespaceId) : Except Error String := do
  let some ns := unit.namespaces.find? (·.identity == identity)
    | throw { message := s!"LeanerLang source references missing owned namespace {identity.index}" }
  let (profile, profileName) ← match ns.profile with
    | some .move => pure (.move, "move")
    | some .rust => pure (.rust, "rust")
    | some (.extension _) =>
        throw <| errorAt unit ns.loc "extension-profile namespaces are outside the current printer"
    | none => throw <| errorAt unit ns.loc "profileless namespaces are outside the current printer"
  unless canonicalProfileConfig profile == unit.profiles[0]? && unit.profiles.size == 1 do
    throw <| errorAt unit ns.loc
      "profile configuration has no lossless spelling in the current LeanerLang header"
  unless ns.imports.isEmpty do
    throw <| errorAt unit ns.loc "namespace imports are outside the current LeanerLang printer"
  let owner := (unit.tables.namespaces[ns.identity.index]?.map (·.segments)).getD #[]
  let mut skippedComments := #[]
  let mut friends := #[]
  for value in ns.profileMetadata do
    unless isResolvedMoveEnvironment unit ns value do
      match ← atLocation unit ns.loc (friendPath? owner value) with
      | some path => friends := friends.push path
      | none =>
        match ← atLocation unit ns.loc (skippedDeclarationComment? value) with
        | some comment => skippedComments := skippedComments.push comment
        | none => throw (errorAt unit ns.loc
            s!"namespace profile metadata `{value.tag}` is outside the current LeanerLang printer")
  unless ns.attributes.isEmpty do
    throw <| errorAt unit ns.loc "namespace attributes are outside the current LeanerLang printer"
  let pragmas ← atLocation unit ns.loc <| ns.pragmas.mapM pragmaText
  let uses ← atLocation unit ns.loc <| inferredUsePaths unit ns.identity
  unless ns.traits.isEmpty && ns.implementations.isEmpty do
    throw <| errorAt unit ns.loc
      "namespace traits and implementations are outside the current LeanerLang printer"
  unless ns.specVars.isEmpty do
    throw <| errorAt unit ns.loc
      "namespace specification variables are outside the current LeanerLang printer"
  unless ns.invariants.isEmpty do
    throw <| errorAt unit ns.loc "namespace invariants are outside the current LeanerLang printer"
  -- Intrinsic declarations print inverted, as source attributes on their
  -- owner and target declarations (`@[intrinsic_map]` on the owner,
  -- `@[map_new (Owner)]` on each bound function or specification function),
  -- following the attribute design the legacy transpiler settled.
  let mut intrinsicAttributes : Array (Nat × String) := #[]
  for declaration in ns.intrinsics do
    let some owner := ns.tables.names[declaration.owner.index]?
      | throw <| errorAt unit declaration.loc "an intrinsic owner name is out of range"
    let ownerText := sourceIdentifier owner.name
    intrinsicAttributes := intrinsicAttributes.push
      (declaration.owner.index, s!"intrinsic_{declaration.model}")
    for binding in declaration.executableBindings ++ declaration.specBindings do
      unless binding.target.namespaceId == ns.identity do
        throw <| errorAt unit binding.loc
          "an intrinsic binding outside its owner's namespace is outside the current LeanerLang printer"
      intrinsicAttributes := intrinsicAttributes.push
        (binding.target.name.index, s!"{binding.role} ({ownerText})")
  let withAttributes (name : NameId) (body : String) : String :=
    let values := intrinsicAttributes.filterMap fun (index, value) =>
      if index == name.index then some value else none
    if values.isEmpty then body
    else s!"@[{", ".intercalate values.toList}]\n{body}"
  let mut ordered : Array SourceOrderedText := #[]
  let mut fallback := 0
  for declaration in ns.constants do
    let (file, start, _) := sourceOrderKey unit declaration.loc fallback
    ordered := ordered.push {
      file, start, priority := 1, fallback
      text := documented declaration.doc
        (← atLocation unit declaration.loc (constantText unit ns declaration)) }
    fallback := fallback + 1
  for declaration in ns.structs do
    let (file, start, _) := sourceOrderKey unit declaration.loc fallback
    ordered := ordered.push {
      file, start, priority := 1, fallback
      text := documented (nominalDocumentation declaration)
        (withAttributes declaration.name
          (← atLocation unit declaration.loc (nominalText unit ns declaration))) }
    fallback := fallback + 1
  for declaration in ns.specFunctions do
    unless isDerivedSpecFunction unit ns declaration do
      let (file, start, _) := sourceOrderKey unit declaration.loc fallback
      ordered := ordered.push {
        file, start, priority := 1, fallback
        text := documented declaration.doc
          (withAttributes declaration.name
            (← atLocation unit declaration.loc (specFunctionText unit ns declaration))) }
      fallback := fallback + 1
  for declaration in ns.functions do
    let (file, start, _) := sourceOrderKey unit declaration.loc fallback
    ordered := ordered.push {
      file, start, priority := 1, fallback
      text := documented declaration.doc
        (withAttributes declaration.name
          (← atLocation unit declaration.loc (functionText unit ns declaration))) }
    fallback := fallback + 1
  -- Compiler-v2 can retain comments from dependency files whose inline
  -- functions were expanded into this module. Those declarations do not
  -- survive as Leaner items, so their comments have no valid attachment here
  -- (the legacy Move printer dropped the same unmatched pool entries).
  let declarationLocations := #[ns.loc] ++ ns.constants.map (·.loc) ++
    ns.structs.map (·.loc) ++ ns.specFunctions.map (·.loc) ++ ns.functions.map (·.loc)
  let declarationFiles := declarationLocations.filterMap fun loc =>
    (primaryRange? unit.tables loc (unit.tables.locations.size + 1)).map (·.file.index)
  for comment in ns.comments do
    let commentFile? :=
      (primaryRange? unit.tables comment.loc (unit.tables.locations.size + 1)).map (·.file.index)
    if commentFile?.all declarationFiles.contains then
      let (file, start, _) := sourceOrderKey unit comment.loc fallback
      ordered := ordered.push {
        file, start, priority := if comment.isDoc then 0 else 1, fallback
        text := sourceCommentText comment }
      fallback := fallback + 1
  let sorted := ordered.qsort fun left right =>
    left.file < right.file || (left.file == right.file &&
      (left.start < right.start || (left.start == right.start &&
        (left.priority < right.priority || (left.priority == right.priority &&
          left.fallback < right.fallback)))))
  let declarations := sorted.map (·.text)
  let declarations := if skippedComments.isEmpty then declarations
    else #[lines skippedComments] ++ declarations
  let declarations := if pragmas.isEmpty then declarations
    else #[lines (pragmas.map fun pragma => s!"pragma {pragma};")] ++ declarations
  let declarations := if friends.isEmpty then declarations
    else #[lines (friends.map fun path => s!"friend {path};")] ++ declarations
  let declarations := if uses.isEmpty then declarations
    else #[lines (uses.map fun path => s!"use {path}")] ++ declarations
  let path ← atLocation unit ns.loc (namespacePath unit ns.identity)
  pure <| "-- Copyright © Aptos Foundation\n" ++
    "-- SPDX-License-Identifier: Apache-2.0\n\n" ++
    "import LeanerLang\n\n" ++
    (if ns.doc.trimAscii.isEmpty then "" else documentationText ns.doc ++ "\n") ++
    s!"leaner namespace {path} using {profileName} where\n" ++
    indent ("\n\n".intercalate declarations.toList) ++ "\n"

private def renderWithLeadingCommentsAt (environment : Lean.Environment)
    (unit : ValidatedUnit) (identity : NamespaceId) (width : Nat)
    (leadingComments : Array String) : Except Error String := do
  let source ← renderSemanticNamespace unit identity
  Layout.formatSource environment source width "<generated>" leadingComments
    |>.mapError fun message =>
    { message := s!"{message}\n--- semantic source ---\n{source}" }

/-- Render one validated Move- or Rust-profile namespace as formatted,
self-contained canonical LeanerLang source. -/
def render (environment : Lean.Environment) (unit : ValidatedUnit)
    (width : Nat := 80) : Except Error String := do
  let [ns] := unit.namespaces.toList
    | throw { message := "LeanerLang source currently requires exactly one owned namespace" }
  renderWithLeadingCommentsAt environment unit ns.identity width #[]

/-- Render one selected namespace while retaining the compilation unit's
other owned namespaces as authoritative declaration context for imports. -/
def renderNamespace (environment : Lean.Environment) (unit : ValidatedUnit)
    (identity : NamespaceId) (width : Nat := 80) : Except Error String :=
  renderWithLeadingCommentsAt environment unit identity width #[]

private def compileErrorText : CompileError → String
  | .frontend diagnostics =>
      String.intercalate "\n" (diagnostics.toList.map Diagnostic.render)
  | .lir diagnostics =>
      String.intercalate "\n" (diagnostics.toList.map fun diagnostic =>
        s!"{diagnostic.code}: {diagnostic.message}")

/-- Render frontend byte ranges with a human-readable position while
canonical source is being recompiled. Frontend spans are relative to the
namespace command (the prelude and module documentation have already been
removed by `namespaceStart`). -/
private def compileErrorTextAt (command : String) : CompileError → String
  | .frontend diagnostics =>
      let fileMap := Lean.FileMap.ofString command
      String.intercalate "\n" <| diagnostics.toList.map fun diagnostic =>
        let position := diagnostic.span.map fun span =>
          fileMap.toPosition ⟨span.startByte⟩
        let location := position.map (fun position =>
          s!" at {position.line}:{position.column + 1}") |>.getD ""
        s!"{diagnostic.render}{location}"
  | error => compileErrorText error

private def parseSourceUnit (environment : Lean.Environment) (source sourceName : String) :
    Except Error CompilationUnit := do
  let (command, comments, namespaceDoc) ← Layout.namespaceStart source
    |>.mapError fun message => { message }
  let parsedSyntax ← Lean.Parser.runParserCategory environment `command command sourceName
    |>.mapError fun message => { message }
  compilationUnitOfSyntax parsedSyntax sourceName comments namespaceDoc
    |>.mapError fun (_, message) => { message }

/-- Declaration-only source view used while checking a namespace against the
other namespaces in its compilation unit. Bodies and contracts are deliberately
absent: imported call typing depends on authoritative declarations and
signatures, not on re-elaborating dependency implementations. -/
private def dependencyInterfaceItem? : Item → Option Item
  | .function declaration => some <| .function {
      declaration with
      modifiers := { declaration.modifiers with isNative := true, isOpaque := false }
      body := none
      contract := #[]
      pragmas := #[] }
  | .specFunction declaration => some <| .specFunction {
      declaration with isOpaque := true, body := none }
  | .struct declaration => some <| .struct {
      declaration with contract := #[], pragmas := #[] }
  | .enum declaration => some <| .enum {
      declaration with contract := #[], pragmas := #[] }
  | .constant _ => none

private def dependencyInterfaceNamespace (source : Namespace) : Namespace := {
  source with
  pragmas := #[]
  comments := #[]
  items := source.items.filterMap dependencyInterfaceItem? }

/-- The Move function whose logical meaning this specification function is,
if it is one. An interface records the companion's signature whether or not
the declaring unit also gave it a body. -/
private def moveCompanionName? (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : LeanerIR.SpecFunctionDecl) : Option String := do
  guard <| declaration.profile == .move
  let qualified ← unit.tables.names[declaration.name.index]?
  guard <| qualified.name.startsWith "$"
  let name := qualified.name.drop 1 |>.toString
  let property : ProfileValue := { profile := .move, tag := "specFunction.moveFunction" }
  guard <| declaration.profileData.contains property
  guard <| ns.functions.any fun function =>
    unit.tables.names[function.name.index]?.any (·.name == name)
  pure name

/-- Move compiler-v2 represents an executable function's logical meaning as
a hidden `$name` spec function. It is suppressed in ordinary source, but a
dependency interface must retain that signature so calls in specifications
resolve to the logical overload rather than the executable declaration. -/
private def moveCompanionInterfaceText (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (declaration : LeanerIR.SpecFunctionDecl)
    (name : String) : Except String String := do
  let binders := " ".intercalate
    (← declaration.signature.generics.mapM (binderText unit)).toList
  let binders := if binders.isEmpty then "" else " " ++ binders
  let parameters ← declaration.signature.parameters.mapM fun parameter => do
    pure s!"{sourceIdentifier parameter.name} : {
      ← typeText unit ns declaration.signature.generics parameter.typeUse.typeId}"
  let result ← match declaration.signature.results.toList with
    | [] => pure "Unit"
    | [result] => typeText unit ns declaration.signature.generics result.typeId
    | _ => throw "multiple specification-function results are outside dependency interfaces"
  pure s!"opaque spec fun {sourceIdentifier name}{binders}({commaSep parameters}) : {result}"

/-- Canonically format LeanerLang source by parsing it, compiling it through
the public LIR boundary, and invoking the ordinary LIR source printer. -/
def formatSource (environment : Lean.Environment) (source : String) (width : Nat := 80)
    (sourceName : String := "<source>") : Except Error String := do
  let (command, _, _) ← Layout.namespaceStart source |>.mapError fun message => { message }
  let sourceUnit ← parseSourceUnit environment source sourceName
  let unit ← compile sourceUnit |>.mapError fun error => {
    message := compileErrorTextAt command error }
  let [ns] := unit.namespaces.toList
    | throw { message := "formatted LeanerLang source must own exactly one namespace" }
  renderWithLeadingCommentsAt environment unit ns.identity width #[]

/-- Render a dependency's public nominal declarations as an interface
namespace. A unit that constructs, selects, or matches a value of an imported
type needs that type's shape, and this is the source form of the shape its
dependency interface carries. -/
private def dependencyInterfaceText (unit : ValidatedUnit)
    (interface : LeanerIR.Validation.ValidatedNamespaceInterface) :
    Except Error String := do
  let profileName ← match interface.profile with
    | some .move => pure "move"
    | some .rust => pure "rust"
    | some (.extension _) | none =>
        throw { message := s!"dependency namespace {interface.namespaceId.index} has no printable profile" }
  let some anchorLoc := unit.namespaces[0]?.map (·.loc)
    | throw { message := "a compilation unit has no namespace to anchor a dependency interface" }
  let ns : ValidatedNamespace := {
    loc := anchorLoc
    identity := interface.namespaceId
    profile := interface.profile
    structs := interface.structs
    functions := interface.functions
    specFunctions := interface.specFunctions
    tables := unit.tables }
  let path ← (namespacePath unit interface.namespaceId).mapError fun message => { message }
  let nominals ← interface.structs.mapM fun declaration =>
    (nominalText unit ns declaration).mapError fun message => { message }
  let functions ← interface.functions.mapM fun declaration =>
    (functionText unit ns declaration).mapError fun message => { message }
  let specFunctions ← interface.specFunctions.filterMap
      (fun declaration => if isDerivedSpecFunction unit ns declaration then none
        else some declaration)
    |>.mapM fun declaration =>
      (specFunctionText unit ns declaration).mapError fun message => { message }
  -- A Move function's logical meaning travels as a hidden `$name` companion.
  -- Ordinary source suppresses it, but an interface must retain the signature
  -- so specification calls resolve to the logical overload.
  let companions ← interface.specFunctions.filterMap
      (fun declaration => (moveCompanionName? unit ns declaration).map ((declaration, ·)))
    |>.mapM fun (declaration, name) =>
      (moveCompanionInterfaceText unit ns declaration name).mapError fun message =>
        ({ message } : Error)
  -- The interface's own references print with the same aliases as ordinary
  -- source, so it carries the `use` paths that give those aliases a meaning.
  let uses ← (inferredUsePathsFor unit ns).mapError fun message => ({ message } : Error)
  let declarations := (if uses.isEmpty then #[] else
      #[lines (uses.map fun path => s!"use {path}")]) ++
    nominals ++ functions ++ specFunctions ++ companions
  pure <| "-- Copyright © Aptos Foundation\n" ++
    "-- SPDX-License-Identifier: Apache-2.0\n\n" ++
    "import LeanerLang\n\n" ++
    s!"leaner namespace {path} using {profileName} where\n" ++
    indent ("\n\n".intercalate declarations.toList) ++ "\n"

/-- Canonically recompile one source namespace against all declaration
signatures in an existing validated compilation unit, returning the freshly
validated unit. This is the round-trip entry behind
[`formatSourceInContext`](formatSourceInContext): parsing remains
context-free, while elaboration resolves imports and checks calls against
dependency signatures. -/
def reimportSourceInContext (environment : Lean.Environment) (source : String)
    (context : ValidatedUnit) (identity : NamespaceId)
    (sourceName : String := "<source>") : Except Error ValidatedUnit := do
  let (command, _, _) ← Layout.namespaceStart source |>.mapError fun message => { message }
  let sourceUnit ← parseSourceUnit environment source sourceName
  let [sourceNs] := sourceUnit.namespaces.toList
    | throw { message := "formatted LeanerLang source must own exactly one namespace" }
  let some contextNs := context.namespaces.find? (·.identity == identity)
    | throw { message := s!"missing owned namespace {identity.index} in compilation context" }
  let some storedPath := context.tables.namespaces[identity.index]?.map (·.segments)
    | throw { message := s!"missing source namespace {identity.index} in compilation context" }
  -- Move's exchange AST retains a named-address alias as a middle segment
  -- (`0x1::std::vector`). A module declaration has the canonical Move shape
  -- `address::module`, while qualified references may still use the alias.
  let expectedPath := if contextNs.profile == some .move && storedPath.size > 1 then
    #[storedPath[0]!, storedPath.back!]
  else storedPath
  unless sourceNs.path == expectedPath do
    throw { message := s!"source namespace `{"::".intercalate sourceNs.path.toList}` does not match compilation-context namespace `{"::".intercalate expectedPath.toList}`" }
  let mut dependencies := #[]
  for dependency in context.namespaces do
    if dependency.identity != identity then
      let some dependencyPath := context.tables.namespaces[dependency.identity.index]?.map
          (·.segments)
        | throw { message := s!"missing dependency namespace {dependency.identity.index} in compilation context" }
      let semantic ← renderSemanticNamespace context dependency.identity
      let mut companions := #[]
      for declaration in dependency.specFunctions do
        if let some name := moveCompanionName? context dependency declaration then
          companions := companions.push
            (← moveCompanionInterfaceText context dependency declaration name
              |>.mapError fun message => { message })
      let semantic := if companions.isEmpty then semantic else
        semantic.trimAsciiEnd.toString ++ "\n\n" ++
          indent ("\n\n".intercalate companions.toList) ++ "\n"
      let parsed ← parseSourceUnit environment semantic
        s!"<dependency {dependency.identity.index}>"
      let [parsedNs] := parsed.namespaces.toList
        | throw { message := s!"dependency {dependency.identity.index} did not render as one namespace" }
      -- A Move module header intentionally drops compiler-owned package aliases
      -- (`0x1::std::vector` becomes `0x1::vector`). Qualified `use` paths retain
      -- those aliases. Restore the authoritative context path on this private
      -- declaration interface so imported calls resolve to their signatures.
      dependencies := dependencies.push {
        dependencyInterfaceNamespace parsedNs with path := dependencyPath }
  for interface in context.dependencies do
    let semantic ← dependencyInterfaceText context interface
    let parsed ← parseSourceUnit environment semantic
      s!"<dependency interface {interface.namespaceId.index}>"
    let [parsedNs] := parsed.namespaces.toList
      | throw { message :=
          s!"dependency interface {interface.namespaceId.index} did not render as one namespace" }
    let some dependencyPath := context.tables.namespaces[interface.namespaceId.index]?.map
        (·.segments)
      | throw { message :=
          s!"missing dependency namespace {interface.namespaceId.index} in compilation context" }
    dependencies := dependencies.push {
      dependencyInterfaceNamespace parsedNs with path := dependencyPath }
  -- The Move module header drops the package alias its stored path keeps
  -- (`0x1::std::vector` is written `0x1::vector`). Compiling under the
  -- authoritative path leaves one namespace per module, so a reference to an
  -- imported name reads the same here as it does in the unit being printed.
  let combined : CompilationUnit := {
    sourceName
    namespaces := #[{ sourceNs with path := storedPath }] ++ dependencies }
  compile combined |>.mapError fun error => {
    message := compileErrorTextAt command error }

/-- Re-elaborate a whole validated unit from its rendered namespaces,
returning a unit whose dependencies carry full bodies again: the behavioral
round-trip counterpart of [`reimportSourceInContext`](reimportSourceInContext),
which checks one namespace against dependency interfaces only. -/
def reimportUnit (environment : Lean.Environment) (unit : ValidatedUnit)
    (width : Nat := 80) : Except Error ValidatedUnit := do
  let mut namespaces := #[]
  for original in unit.namespaces do
    let source ← renderNamespace environment unit original.identity width
    let parsed ← parseSourceUnit environment source
      s!"<reimport {original.identity.index}>"
    let [parsedNs] := parsed.namespaces.toList
      | throw
          { message :=
            s!"reimported namespace {original.identity.index} did not render as one namespace" }
    -- Restore the authoritative table path on the parsed namespace so
    -- qualified references resolve across the combined unit regardless of
    -- package-alias forms the renderer drops from module headers.
    let some storedPath := unit.tables.namespaces[original.identity.index]?.map (·.segments)
      | throw
          { message :=
            s!"missing namespace {original.identity.index} in compilation tables" }
    namespaces := namespaces.push { parsedNs with path := storedPath }
  compile { sourceName := "<reimport>", namespaces := namespaces } |>.mapError
    fun error =>
      { message := s!"round-trip recompilation failed: {compileErrorText error}" : Error }

/-- Canonically recompile one source namespace against all declaration
signatures in an existing validated compilation unit. This is the ordinary
package/compiler mode: parsing remains context-free, while elaboration resolves
imports and checks calls against dependency signatures. -/
def formatSourceInContext (environment : Lean.Environment) (source : String)
    (context : ValidatedUnit) (identity : NamespaceId) (width : Nat := 80)
    (sourceName : String := "<source>") : Except Error String := do
  let unit ← reimportSourceInContext environment source context identity sourceName
  renderWithLeadingCommentsAt environment unit ⟨0⟩ width #[]

/-- Load the parser environment for runtime generators which do not already
carry Lean's elaboration environment. -/
unsafe def loadEnvironment : IO Lean.Environment := do
  Lean.enableInitializersExecution
  Lean.initSearchPath (← Lean.findSysroot)
  Lean.importModules #[{ module := `LeanerLang.Elab }] {} (loadExts := true)

/-- Runtime printer entry point for generators without an existing Lean
environment. -/
unsafe def renderIO (unit : ValidatedUnit) (width : Nat := 80) : IO String := do
  let environment ← loadEnvironment
  match render environment unit width with
  | .ok source => pure source
  | .error error => throw <| IO.userError (toString error)

end LeanerLang.Print
