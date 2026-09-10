-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerRust.Profile

/-!
# Canonical Rust source backend

This backend consumes only shared `ValidatedUnit` values.  It deliberately
prints a conservative, explicit Rust program from structured LIR instead of
recovering producer source text.  Unsupported source forms fail at the exact
validated node; the backend never falls back to a Rust-frontend side table.
-/

namespace LeanerIR.Rust.Source

open LeanerIR Validation

/-- Half-open UTF-8 byte range in the generated Rust source. -/
structure GeneratedRange where
  startByte : Nat
  stopByte : Nat
  deriving Repr, BEq

/-- Stable validated-LIR node referenced by generated Rust source. -/
inductive GeneratedNode where
  | function (namespaceId : NamespaceId) (functionId : FunctionId)
  | functionBinder (namespaceId : NamespaceId) (functionId : FunctionId) (index : Nat)
  | nominal (namespaceId : NamespaceId) (nameId : NameId)
  | nominalBinder (namespaceId : NamespaceId) (owner : NameId) (index : Nat)
  | variant (namespaceId : NamespaceId) (owner : NameId) (name : NameId)
  | field (namespaceId : NamespaceId) (owner : NameId) (variant : Option NameId)
      (name : NameId)
  | typeUse (namespaceId : NamespaceId) (typeId : TypeId) (loc : LocId)
  | expression (namespaceId : NamespaceId) (expressionId : ExprId)
  | pattern (namespaceId : NamespaceId) (patternId : PatternId)
  | place (namespaceId : NamespaceId) (placeId : PlaceId)
  | local (namespaceId : NamespaceId) (functionId : FunctionId) (localId : LocalId)
  deriving Repr, BEq

/-- One generated range, its semantic LIR node, and checked import provenance
when the source schema attaches such provenance to the declaration. -/
structure GeneratedSourceMapEntry where
  range : GeneratedRange
  node : GeneratedNode
  originalLoc : LocId
  origin : Option OriginId
  alignment : Option AlignmentId
  deriving Repr, BEq

structure GeneratedSourceMap where
  entries : Array GeneratedSourceMapEntry
  deriving Repr, BEq

structure RenderedSource where
  text : String
  sourceMap : GeneratedSourceMap
  deriving Repr, BEq

private def markerStart := String.singleton (Char.ofNat 31)
private def markerStop := String.singleton (Char.ofNat 30)

private def nodeFields : GeneratedNode → Array String
  | .function namespaceId functionId =>
      #["function", toString namespaceId.index, toString functionId.index]
  | .functionBinder namespaceId functionId index =>
      #["function-binder", toString namespaceId.index, toString functionId.index, toString index]
  | .nominal namespaceId nameId =>
      #["nominal", toString namespaceId.index, toString nameId.index]
  | .nominalBinder namespaceId owner index =>
      #["nominal-binder", toString namespaceId.index, toString owner.index, toString index]
  | .variant namespaceId owner name =>
      #["variant", toString namespaceId.index, toString owner.index, toString name.index]
  | .field namespaceId owner variant name =>
      #["field", toString namespaceId.index, toString owner.index,
        variant.map (toString ∘ (·.index)) |>.getD "-", toString name.index]
  | .typeUse namespaceId typeId loc =>
      #["type-use", toString namespaceId.index, toString typeId.index, toString loc.index]
  | .expression namespaceId expressionId =>
      #["expression", toString namespaceId.index, toString expressionId.index]
  | .pattern namespaceId patternId =>
      #["pattern", toString namespaceId.index, toString patternId.index]
  | .place namespaceId placeId =>
      #["place", toString namespaceId.index, toString placeId.index]
  | .local namespaceId functionId localId =>
      #["local", toString namespaceId.index, toString functionId.index, toString localId.index]

private def optionalIndexText {α : Type} (index : Option α) (getIndex : α → Nat) : String :=
  index.map (toString ∘ getIndex) |>.getD "-"

private def markerWithProvenance (node : GeneratedNode) (loc : LocId)
    (origin : Option OriginId) (alignment : Option AlignmentId) (boundary : String) : String :=
  markerStart ++ "LIR|" ++ "|".intercalate (nodeFields node).toList ++
    "|" ++ toString loc.index ++ "|" ++ optionalIndexText origin (·.index) ++
    "|" ++ optionalIndexText alignment (·.index) ++ "|" ++ boundary ++ markerStop

private def mark (node : GeneratedNode) (loc : LocId) (origin : OriginId)
    (alignment : AlignmentId) (value : String) : String :=
  markerWithProvenance node loc (some origin) (some alignment) "start" ++ value ++
    markerWithProvenance node loc (some origin) (some alignment) "stop"

private def markWithoutImportProvenance (node : GeneratedNode) (loc : LocId)
    (value : String) : String :=
  markerWithProvenance node loc none none "start" ++ value ++
    markerWithProvenance node loc none none "stop"

private structure OpenMarker where
  node : GeneratedNode
  originalLoc : LocId
  origin : Option OriginId
  alignment : Option AlignmentId
  startByte : Nat

private def parseMarker (value : String) :
    Except String (GeneratedNode × LocId × Option OriginId × Option AlignmentId × Bool) := do
  let fields := value.splitOn "|"
  let parseIndex (description text : String) : Except String Nat :=
    match text.toNat? with
    | some index => pure index
    | none => throw s!"malformed marker {description} `{text}`"
  let (node, locText, originText, alignmentText, boundary) ← match fields with
    | ["LIR", "function-binder", namespaceText, functionText, indexText, locText,
        originText, alignmentText, boundary] => do
        let namespaceIndex ← parseIndex "namespace" namespaceText
        let functionIndex ← parseIndex "binder owner" functionText
        let binderIndex ← parseIndex "binder" indexText
        pure (.functionBinder ⟨namespaceIndex⟩ ⟨functionIndex⟩ binderIndex,
          locText, originText, alignmentText, boundary)
    | ["LIR", "type-use", namespaceText, typeText, sourceLocText, locText,
        originText, alignmentText, boundary] => do
        let namespaceIndex ← parseIndex "namespace" namespaceText
        let typeIndex ← parseIndex "type" typeText
        let sourceLocIndex ← parseIndex "type-use location" sourceLocText
        pure (.typeUse ⟨namespaceIndex⟩ ⟨typeIndex⟩ ⟨sourceLocIndex⟩,
          locText, originText, alignmentText, boundary)
    | ["LIR", "nominal-binder", namespaceText, ownerText, indexText, locText,
        originText, alignmentText, boundary] => do
        let namespaceIndex ← parseIndex "namespace" namespaceText
        let ownerIndex ← parseIndex "binder owner" ownerText
        let binderIndex ← parseIndex "binder" indexText
        pure (.nominalBinder ⟨namespaceIndex⟩ ⟨ownerIndex⟩ binderIndex,
          locText, originText, alignmentText, boundary)
    | ["LIR", "field", namespaceText, ownerText, variantText, fieldText, locText,
        originText, alignmentText, boundary] => do
        let namespaceIndex ← parseIndex "namespace" namespaceText
        let ownerIndex ← parseIndex "field owner" ownerText
        let variantIndex ← if variantText == "-" then pure none else
          pure <| some (← parseIndex "field variant" variantText)
        let fieldIndex ← parseIndex "field" fieldText
        pure (.field ⟨namespaceIndex⟩ ⟨ownerIndex⟩ (variantIndex.map (⟨·⟩)) ⟨fieldIndex⟩,
          locText, originText, alignmentText, boundary)
    | ["LIR", "variant", namespaceText, ownerText, variantText, locText,
        originText, alignmentText, boundary] => do
        let namespaceIndex ← parseIndex "namespace" namespaceText
        let ownerIndex ← parseIndex "variant owner" ownerText
        let variantIndex ← parseIndex "variant" variantText
        pure (.variant ⟨namespaceIndex⟩ ⟨ownerIndex⟩ ⟨variantIndex⟩,
          locText, originText, alignmentText, boundary)
    | ["LIR", "local", namespaceText, functionText, localText, locText,
        originText, alignmentText, boundary] => do
        let namespaceIndex ← parseIndex "namespace" namespaceText
        let functionIndex ← parseIndex "function" functionText
        let localIndex ← parseIndex "local" localText
        pure (.local ⟨namespaceIndex⟩ ⟨functionIndex⟩ ⟨localIndex⟩,
          locText, originText, alignmentText, boundary)
    | ["LIR", kind, namespaceText, idText, locText, originText, alignmentText, boundary] => do
        let namespaceIndex ← parseIndex "namespace" namespaceText
        let nodeIndex ← parseIndex "node" idText
        let node ← match kind with
          | "function" => pure <| GeneratedNode.function ⟨namespaceIndex⟩ ⟨nodeIndex⟩
          | "nominal" => pure <| GeneratedNode.nominal ⟨namespaceIndex⟩ ⟨nodeIndex⟩
          | "expression" => pure <| GeneratedNode.expression ⟨namespaceIndex⟩ ⟨nodeIndex⟩
          | "pattern" => pure <| GeneratedNode.pattern ⟨namespaceIndex⟩ ⟨nodeIndex⟩
          | "place" => pure <| GeneratedNode.place ⟨namespaceIndex⟩ ⟨nodeIndex⟩
          | _ => throw s!"unknown generated-source marker kind `{kind}`"
        pure (node, locText, originText, alignmentText, boundary)
    | _ => throw s!"malformed generated-source marker `{value}`"
  let some locIndex := locText.toNat? | throw s!"malformed marker location `{locText}`"
  let parseOptionalIndex (description text : String) : Except String (Option Nat) :=
    if text == "-" then pure none else
      match text.toNat? with
      | some index => pure (some index)
      | none => throw s!"malformed marker {description} `{text}`"
  let originIndex ← parseOptionalIndex "origin" originText
  let alignmentIndex ← parseOptionalIndex "alignment" alignmentText
  let isStart ← match boundary with
    | "start" => pure true
    | "stop" => pure false
    | _ => throw s!"unknown generated-source marker boundary `{boundary}`"
  pure (node, ⟨locIndex⟩, originIndex.map (⟨·⟩), alignmentIndex.map (⟨·⟩), isStart)

private def finishMarkedSource (value : String) : Except String RenderedSource := do
  let chunks := value.splitOn markerStart
  let mut text := chunks.head?.getD ""
  let mut openMarkers : List OpenMarker := []
  let mut entries : Array GeneratedSourceMapEntry := #[]
  for chunk in chunks.drop 1 do
    let pieces := chunk.splitOn markerStop
    let some command := pieces.head?
      | throw "generated-source marker is missing its command"
    unless pieces.length > 1 do
      throw s!"unterminated generated-source marker `{command}`"
    let remainder := markerStop.intercalate (pieces.drop 1)
    let (node, originalLoc, origin, alignment, isStart) ← parseMarker command
    let byte := text.toUTF8.size
    if isStart then
      openMarkers := { node, originalLoc, origin, alignment, startByte := byte } :: openMarkers
    else
      let some opened := openMarkers.head?
        | throw "generated-source marker closed without an opening marker"
      unless opened.node == node && opened.originalLoc == originalLoc &&
          opened.origin == origin && opened.alignment == alignment do
        throw "generated-source markers are not properly nested"
      openMarkers := openMarkers.tail
      entries := entries.push {
        range := { startByte := opened.startByte, stopByte := byte }
        node
        originalLoc
        origin
        alignment }
    text := text ++ remainder
  unless openMarkers.isEmpty do
    throw "generated-source marker was not closed"
  entries := entries.qsort fun left right =>
    left.range.startByte < right.range.startByte ||
      (left.range.startByte == right.range.startByte && left.range.stopByte > right.range.stopByte)
  pure { text, sourceMap := { entries } }

private def commaSep (values : Array String) : String :=
  ", ".intercalate values.toList

private def lineSep (values : Array String) : String :=
  "\n".intercalate values.toList

private def indent (value : String) : String :=
  "\n".intercalate <| value.splitOn "\n" |>.map ("    " ++ ·)

private def rustBlock (value : String) : String :=
  if value.startsWith "{" then value else "{ " ++ value ++ " }"

private def rustKeywords : Array String := #[
  "as", "break", "const", "continue", "crate", "else", "enum", "extern",
  "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
  "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct",
  "super", "trait", "true", "type", "unsafe", "use", "where", "while", "async",
  "await", "dyn", "abstract", "become", "box", "do", "final", "macro", "override",
  "priv", "typeof", "unsized", "virtual", "yield", "try"
]

private def rustIdentifier (name : String) : String :=
  if rustKeywords.contains name then "r#" ++ name else name

private def nameAt (tables : Tables) (id : NameId) : Except String String := do
  let some name := tables.names[id.index]?
    | throw s!"validated name {id.index} is missing"
  pure (rustIdentifier name.name)

private def constText : ConstValue → Except String String
  | .unit => pure "()"
  | .bool true => pure "true"
  | .bool false => pure "false"
  | .character value =>
      pure <| "'\\u{" ++ String.ofList (Nat.toDigits 16 value) ++ "}'"
  | .integer value => pure (toString value)
  | .string value => pure (Lean.Json.compress (.str value))
  | .bytes values => pure s!"[{commaSep <| values.map (fun value => s!"{value}_u8")}]"
  | .vector values => do
      let elements ← values.mapM constText
      pure s!"[{commaSep elements}]"
  | .tuple values => do
      let fields ← values.mapM constText
      pure <| if fields.size == 1 then s!"({fields[0]!},)" else s!"({commaSep fields})"
  | .address _ => throw "Rust source does not support the Move address constant"
  | .profile value => throw s!"Rust source does not support profile constant `{value.tag}`"

private structure Context where
  unit : ValidatedUnit
  ns : ValidatedNamespace
  functionId : FunctionId
  declaration : FunctionDecl FunctionBody
  fieldBindings : Array (PlaceId × String) := #[]

private def localIdentifier (declaration : LocalDecl) (id : LocalId) : String :=
  rustIdentifier declaration.name ++ s!"__lir_{id.index}"

private def localName (context : Context) (id : LocalId) : Except String String := do
  let some localDecl := context.declaration.locals[id.index]?
    | throw s!"validated local {id.index} is missing"
  pure <| mark (.local context.ns.identity context.functionId id) localDecl.loc
    context.declaration.origin context.declaration.alignment (localIdentifier localDecl id)

private def binderName (binders : Array GenericBinder) (index : Nat) : Except String String := do
  let some binder := binders[index]?
    | throw s!"validated generic binder {index} is missing"
  pure (rustIdentifier binder.name)

private def lifetimeName (tables : Tables) (binders : Array GenericBinder)
    (id : LifetimeId) : Except String String := do
  let some lifetime := tables.lifetimes[id.index]?
    | throw s!"validated lifetime {id.index} is missing"
  match lifetime.kind with
  | .static => pure "'static"
  | .parameter index =>
      let name ← binderName binders index
      pure <| if name.startsWith "'" then name else "'" ++ name
  | .inference | .local => pure "'_"

private def constGenericParameterTypeText (tables : Tables) (type : TypeUse) :
    Except String String := do
  let some ty := tables.types[type.typeId.index]?
    | throw s!"validated const generic type {type.typeId.index} is missing"
  match ty with
  | .bool => pure "bool"
  | .character => pure "char"
  | .integer (.bits width) signed =>
      if #[8, 16, 32, 64, 128].contains width then
        pure s!"{if signed then "i" else "u"}{width}"
      else throw s!"Rust has no const generic integer type with width {width}"
  | .integer .pointer signed => pure (if signed then "isize" else "usize")
  | _ => throw "const generic binder type has no supported standard Rust spelling"

private def genericParameterText (tables : Tables) (binder : GenericBinder) :
    Except String String := do
    match binder.kind with
    | .typeArg => pure (rustIdentifier binder.name)
    | .lifetime =>
        pure <| if binder.name.startsWith "'" then binder.name else "'" ++ binder.name
    | .const =>
        let some type := binder.type
          | throw "Rust const generic binder has no declared parameter type"
        pure s!"const {rustIdentifier binder.name}: {← constGenericParameterTypeText tables type}"
    | .evidence => throw "Rust evidence binders require checked implementation syntax"

private def functionGenericParameterText (tables : Tables) (binder : GenericBinder) :
    Except String String := do
  match binder.kind with
  | .typeArg => pure (rustIdentifier binder.name)
  | .lifetime =>
      pure <| if binder.name.startsWith "'" then binder.name else "'" ++ binder.name
  | .const =>
      let some type := binder.type
        | throw "Rust const generic binder has no declared parameter type"
      pure s!"const {rustIdentifier binder.name}: {← constGenericParameterTypeText tables type}"
  | .evidence => throw "Rust evidence binders require checked implementation syntax"

private def genericParametersText (tables : Tables) (namespaceId : NamespaceId) (owner : NameId)
    (binders : Array GenericBinder) : Except String String := do
  let parameters ← binders.zipIdx.mapM fun (binder, index) => do
    let rendered ← genericParameterText tables binder
    pure <| markWithoutImportProvenance
      (.nominalBinder namespaceId owner index) binder.loc rendered
  pure <| if parameters.isEmpty then "" else s!"<{commaSep parameters}>"

private def functionGenericParametersText (tables : Tables) (namespaceId : NamespaceId)
    (functionId : FunctionId)
    (declaration : FunctionDecl FunctionBody) : Except String String := do
  let parameters ← declaration.signature.generics.zipIdx.mapM fun (binder, index) => do
    let rendered ← functionGenericParameterText tables binder
    pure <| mark (.functionBinder namespaceId functionId index) binder.loc declaration.origin
      declaration.alignment rendered
  pure <| if parameters.isEmpty then "" else s!"<{commaSep parameters}>"

private def predicateLifetimeName (tables : Tables) (binders : Array GenericBinder)
    (id : LifetimeId) : Except String String := do
  let some lifetime := tables.lifetimes[id.index]?
    | throw s!"validated predicate lifetime {id.index} is missing"
  match lifetime.kind with
  | .static | .parameter _ => lifetimeName tables binders id
  | .inference | .local =>
      throw "inferred or local lifetimes have no Rust where-predicate spelling"

private def functionPredicateText (tables : Tables) (binders : Array GenericBinder) :
    GenericPredicate → Except String String
  | .lifetimeOutlives longer shorter => do
      pure s!"{← predicateLifetimeName tables binders longer}: {←
        predicateLifetimeName tables binders shorter}"
  | .ability typeId .copy => match tables.types[typeId.index]? with
      | some (.typeParameter index) => do
          pure s!"{← binderName binders index}: Copy"
      | _ => throw "Rust Copy predicates currently require a type parameter"
  | .ability _ .drop | .ability _ .store | .ability _ .key |
      .implements .. | .associatedTypeEq .. | .associatedConstEq .. |
      .constEq .. | .profile _ =>
      throw "generic Rust function predicate has no implemented standard-source reconstruction"

private def genericPredicatesText (tables : Tables) (binders : Array GenericBinder)
    (signaturePredicates : Array GenericPredicate := #[]) : Except String String := do
  let predicates := binders.foldl
    (fun predicates binder => predicates ++ binder.predicates)
    signaturePredicates
  let rendered ← predicates.foldlM (m := Except String) (init := #[])
    fun rendered predicate => do
      let constraint ← functionPredicateText tables binders predicate
      pure <| if rendered.contains constraint then rendered else rendered.push constraint
  let rendered ← binders.zipIdx.foldlM (m := Except String)
    (init := rendered) fun rendered pair =>
      pair.1.abilities.foldlM (m := Except String) (init := rendered) fun rendered ability =>
        match pair.1.kind, ability with
        | .typeArg, .copy => do
            let constraint := s!"{← binderName binders pair.2}: Copy"
            pure <| if rendered.contains constraint then rendered else rendered.push constraint
        | _, .copy => throw "Rust Copy ability can constrain only a type binder"
        | _, .drop | _, .store | _, .key =>
            throw "generic function ability has no implemented Rust source bound"
  pure <| if rendered.isEmpty then "" else s!" where {commaSep rendered}"

private def contractIsEmpty (contract : FunctionContract) : Bool :=
  contract.loc.isNone && contract.conditions.isEmpty && contract.modifies.isEmpty &&
    contract.reads.isEmpty && !contract.hasFrame && !contract.modifiesAll &&
    !contract.readsAll && contract.pragmas.isEmpty

mutual
  private partial def typeTextFuel (tables : Tables) (binders : Array GenericBinder)
      (id : TypeId) (fuel : Nat) : Except String String := do
    if fuel == 0 then throw "cyclic validated type reached the Rust source backend"
    let some ty := tables.types[id.index]?
      | throw s!"validated type {id.index} is missing"
    match ty with
    | .unit => pure "()"
    | .never => pure "!"
    | .bool => pure "bool"
    | .character => pure "char"
    | .string => pure "str"
    | .integer (.bits width) signed =>
        if #[8, 16, 32, 64, 128].contains width then
          pure s!"{if signed then "i" else "u"}{width}"
        else throw s!"Rust has no source integer type with width {width}"
    | .integer .pointer signed => pure (if signed then "isize" else "usize")
    | .tuple elements =>
        let fields ← elements.mapM (typeTextFuel tables binders · (fuel - 1))
        pure <| if fields.size == 1 then s!"({fields[0]!},)" else s!"({commaSep fields})"
    | .vector element length =>
        let element ← typeTextFuel tables binders element (fuel - 1)
        match length with
        | none => pure s!"[{element}]"
        | some (.integer length) =>
            if length < 0 then throw "validated fixed vector has a negative length"
            pure s!"[{element}; {length}]"
        | some _ => throw "Rust source requires an evaluated integer array length"
    | .nominal name arguments =>
        let name ← nameAt tables name
        if arguments.isEmpty then pure name else
          let arguments ← arguments.mapM (genericArgumentText tables binders · (fuel - 1))
          pure s!"{name}::<{commaSep arguments}>"
    | .function arguments result _ =>
        let arguments ← arguments.mapM (typeTextFuel tables binders · (fuel - 1))
        let result ← typeTextFuel tables binders result (fuel - 1)
        pure s!"fn({commaSep arguments}) -> {result}"
    | .typeParameter index => binderName binders index
    | .reference reference =>
        let referent ← typeTextFuel tables binders reference.referent (fuel - 1)
        let lifetime ← lifetimeName tables binders reference.lifetime
        let lifetime := if lifetime == "'_" then "" else lifetime ++ " "
        pure s!"&{lifetime}{if reference.kind == .mutable then "mut " else ""}{referent}"
    | .bytes | .integer .unbounded _ | .address | .signer | .range | .eventStore |
        .typeDomain _ | .resourceDomain .. | .stateDomain | .profile _ =>
        throw s!"validated type {id.index} has no standard Rust source spelling"

  private partial def genericArgumentText (tables : Tables) (binders : Array GenericBinder)
      (argument : GenericArgument) (fuel : Nat) : Except String String :=
    match argument with
    | .typeArg value => typeTextFuel tables binders value.typeId fuel
    | .const value => constText value
    | .lifetime lifetime => lifetimeName tables binders lifetime
    | .evidence _ => throw "Rust source cannot print unresolved implementation evidence"
end

private def typeText (tables : Tables) (binders : Array GenericBinder) (id : TypeId) :
    Except String String :=
  typeTextFuel tables binders id (tables.types.size + 1)

private def declarationTypeUseText (unit : ValidatedUnit) (ns : NamespaceId)
    (binders : Array GenericBinder) (typeUse : TypeUse) : Except String String := do
  let rendered ← typeText unit.tables binders typeUse.typeId
  pure <| markWithoutImportProvenance (.typeUse ns typeUse.typeId typeUse.loc)
    typeUse.loc rendered

private def functionTypeUseText (unit : ValidatedUnit) (ns : NamespaceId)
    (declaration : FunctionDecl FunctionBody) (typeUse : TypeUse) : Except String String := do
  let rendered ← typeText unit.tables declaration.signature.generics typeUse.typeId
  pure <| mark (.typeUse ns typeUse.typeId typeUse.loc) typeUse.loc declaration.origin
    declaration.alignment rendered

private def expressionTypeUseText (context : Context) (typeId : TypeId) (loc : LocId) :
    Except String String := do
  let rendered ← typeText context.unit.tables context.declaration.signature.generics typeId
  pure <| mark (.typeUse context.ns.identity typeId loc) loc context.declaration.origin
    context.declaration.alignment rendered

private def fieldSelector (tables : Tables) (id : NameId) : Except String String := do
  let field ← nameAt tables id
  pure <| if field.toNat?.isSome then field else rustIdentifier field

private def fieldsArePositional (tables : Tables) (fields : Array FieldDecl) : Bool :=
  fields.all fun field =>
    (tables.names[field.name.index]?.map (·.name.toNat?.isSome)).getD false

private def loopTarget (loops : List String) (nest : Nat) : Except String String := do
  let some label := loops[nest]?
    | throw s!"validated loop target {nest} is missing"
  pure label

private def constructorText (context : Context) (reference : QualifiedRef)
    (variantName : Option String) (values : Array String) : Except String String := do
  unless reference.namespaceId == context.ns.identity do
    throw "external constructors require emitted dependency modules"
  let some declaration := context.ns.structs.find? (fun declaration =>
      declaration.name == reference.name)
    | throw "validated constructor declaration is missing"
  let owner ← nameAt context.unit.tables declaration.name
  let (constructor, fields) ← match variantName with
    | none => pure (owner, declaration.fields)
    | some variantName =>
        let some variant := declaration.variants.find? fun variant =>
          context.unit.tables.names[variant.name.index]?.map (·.name) == some variantName
          | throw s!"validated enum variant `{variantName}` is missing"
        pure (owner ++ "::" ++ rustIdentifier variantName, variant.fields)
  unless fields.size == values.size do
    throw s!"validated constructor `{constructor}` has wrong source arity"
  if fields.isEmpty then pure constructor
  else if fieldsArePositional context.unit.tables fields then
    pure s!"{constructor}({commaSep values})"
  else
    let assignments ← (fields.zip values).mapM fun (field, value) => do
      pure s!"{← nameAt context.unit.tables field.name}: {value}"
    pure <| constructor ++ " { " ++ commaSep assignments ++ " }"

private partial def placeType? (context : Context) (id : PlaceId)
    (fuel : Nat := 0) : Option TypeId :=
  let fuel := if fuel == 0 then context.ns.places.size + 1 else fuel
  match fuel, context.ns.places[id.index]? with
  | 0, _ | _, none => none
  | _ + 1, some (.localVar localId) =>
      context.declaration.locals[localId.index]?.map (·.type.typeId)
  | fuel + 1, some (.deref base) => do
      let baseType ← placeType? context base fuel
      let .reference reference ← context.unit.tables.types[baseType.index]? | none
      some reference.referent
  | fuel + 1, some (.index base index) => do
      let baseType ← placeType? context base fuel
      match context.unit.tables.types[baseType.index]? with
      | some (.vector element _) => some element
      | some (.tuple elements) => do
          let expression ← context.ns.expressions[index.index]?
          let .value (.integer value) _ := expression.kind | none
          if value < 0 then none else elements[value.toNat]?
      | _ => none
  | fuel + 1, some (.subslice base ..) => placeType? context base fuel
  | fuel + 1, some (.downcast base _) => placeType? context base fuel
  | fuel + 1, some (.field base _ field) => do
      let baseType ← placeType? context base fuel
      let .nominal name _ ← context.unit.tables.types[baseType.index]? | none
      let declaration ← context.ns.structs.find? (·.name == name)
      (declaration.fields.find? (·.name == field)).map (·.type.typeId)

private def directLocalPlace? (ns : ValidatedNamespace) (id : PlaceId) : Option LocalId := do
  let .localVar localId ← ns.places[id.index]? | none
  some localId

private def loadedPlace? (ns : ValidatedNamespace) (id : ExprId) : Option PlaceId := do
  let expression ← ns.expressions[id.index]?
  match expression.kind with
  | .operation (.move place) _ arguments _
  | .operation (.copy place) _ arguments _
  | .operation (.read place) _ arguments _ =>
      if arguments.isEmpty then some place else none
  | _ => none

private def loadedLocal? (ns : ValidatedNamespace) (id : ExprId) : Option LocalId := do
  let expression ← ns.expressions[id.index]?
  match expression.kind with
  | .localVar localId => some localId
  | _ => (loadedPlace? ns id).bind (directLocalPlace? ns)

private partial def flattenedSourceBlock (ns : ValidatedNamespace)
    (statements : Array ExprId) (result : Option ExprId) (fuel : Nat) :
    Array ExprId × Option ExprId :=
  if fuel == 0 then (statements, result) else
    match result with
    | some resultId => match ns.expressions[resultId.index]? with
        | some { kind := .block nestedStatements nestedResult, .. } =>
            flattenedSourceBlock ns (statements ++ nestedStatements) nestedResult (fuel - 1)
        | _ => (statements, result)
    | none => (statements, result)

private def forwardedReturnBlock? (ns : ValidatedNamespace) (statements : Array ExprId)
    (result : Option ExprId) : Option (ExprId × PlaceId × ExprId × ExprId × ExprId) := do
  let [assignmentId] := statements.toList | none
  let assignment ← ns.expressions[assignmentId.index]?
  let .assign targetPlace valueId := assignment.kind | none
  let targetLocal ← directLocalPlace? ns targetPlace
  let resultId ← result
  let resultExpression ← ns.expressions[resultId.index]?
  let .return_ returned := resultExpression.kind | none
  let [returnedId] := returned.toList | none
  if loadedLocal? ns returnedId == some targetLocal &&
      loadedLocal? ns valueId != some targetLocal then
    some (assignmentId, targetPlace, valueId, resultId, returnedId)
  else none

private partial def markFlattenedResultBlocks (context : Context) (current final : Option ExprId)
    (value : String) (fuel : Nat) : String :=
  if fuel == 0 || current == final then value else
    match current with
    | some currentId => match context.ns.expressions[currentId.index]? with
        | some expression@{ kind := .block _ nestedResult, .. } =>
            mark (.expression context.ns.identity currentId) expression.loc
              context.declaration.origin context.declaration.alignment <|
                markFlattenedResultBlocks context nestedResult final value (fuel - 1)
        | _ => value
    | none => value

mutual
  /-- Attach overlapping source-map ranges when a source-level reconstruction
  replaces an administrative LIR subtree rather than printing each node
  separately. -/
  private partial def markExpressionTree (context : Context) (id : ExprId)
      (value : String) (fuel : Nat) : String :=
    if fuel == 0 then value else
    match context.ns.expressions[id.index]? with
    | none => value
    | some expression =>
        let visitExpression child value :=
          markExpressionTree context child value (fuel - 1)
        let visitPlace child value := markPlaceTree context child expression.loc value (fuel - 1)
        let visitPattern child value := markPatternTree context child value (fuel - 1)
        let value := match expression.kind with
          | .value .. | .constant _ | .localVar _ | .continue_ _ | .spec _ => value
          | .operation operation _ arguments _ =>
              let value := arguments.foldl (fun value child => visitExpression child value) value
              match operation with
              | .move place | .copy place | .borrow _ place | .read place | .write place |
                  .drop place => visitPlace place value
              | _ => value
          | .block statements result =>
              let value := statements.foldl (fun value child => visitExpression child value) value
              result.map (fun child => visitExpression child value) |>.getD value
          | .letDecl pattern initializer body =>
              let value := visitPattern pattern value
              let value := initializer.map (fun child => visitExpression child value) |>.getD value
              visitExpression body value
          | .ifElse condition thenBranch elseBranch =>
              let value := visitExpression condition value
              let value := visitExpression thenBranch value
              elseBranch.map (fun child => visitExpression child value) |>.getD value
          | .match_ scrutinee arms =>
              arms.foldl (fun value arm =>
                let value := visitPattern arm.pattern value
                let value := arm.guard.map (fun child => visitExpression child value) |>.getD value
                visitExpression arm.body value) (visitExpression scrutinee value)
          | .loop _ body => visitExpression body value
          | .break_ _ result =>
              result.map (fun child => visitExpression child value) |>.getD value
          | .return_ results | .throw_ _ results =>
              results.foldl (fun value child => visitExpression child value) value
          | .assign place assigned => visitExpression assigned (visitPlace place value)
          | .assignPattern pattern assigned =>
              visitExpression assigned (visitPattern pattern value)
          | .quantifier _ binders triggers condition body =>
              let value := binders.foldl (fun value binder =>
                visitExpression binder.domain (visitPattern binder.pattern value)) value
              let value := triggers.foldl (fun value trigger =>
                trigger.foldl (fun value child => visitExpression child value) value) value
              let value := condition.map (fun child => visitExpression child value) |>.getD value
              visitExpression body value
        mark (.expression context.ns.identity id) expression.loc context.declaration.origin
          context.declaration.alignment value

  private partial def markPlaceTree (context : Context) (id : PlaceId) (sourceLoc : LocId)
      (value : String) (fuel : Nat) : String :=
    if fuel == 0 then value else
    match context.ns.places[id.index]? with
    | none => value
    | some place =>
        let value := match place with
          | .localVar _ => value
          | .deref base | .field base .. | .subslice base .. | .downcast base _ =>
              markPlaceTree context base sourceLoc value (fuel - 1)
          | .index base index =>
              markExpressionTree context index
                (markPlaceTree context base sourceLoc value (fuel - 1)) (fuel - 1)
        mark (.place context.ns.identity id) sourceLoc context.declaration.origin
          context.declaration.alignment value

  private partial def markPatternTree (context : Context) (id : PatternId)
      (value : String) (fuel : Nat) : String :=
    if fuel == 0 then value else
    match context.ns.patterns[id.index]? with
    | none => value
    | some pattern =>
        let value := match pattern.kind with
          | .tuple elements | .constructor _ _ _ elements =>
              elements.foldl (fun value child =>
                markPatternTree context child value (fuel - 1)) value
          | .wildcard | .variable _ | .literal _ | .range .. => value
        mark (.pattern context.ns.identity id) pattern.loc context.declaration.origin
          context.declaration.alignment value
end

mutual
  private partial def downcastFieldText? (context : Context) (loops : List String)
      (id : PlaceId) (sourceLoc : LocId) (receiverPrefix : String)
      (dereferenceSelected : Bool) (fuel : Nat) :
      Except String (Option String) := do
    if fuel == 0 then throw "cyclic validated place reached the Rust source backend"
    if let some binding := context.fieldBindings.find? (·.1 == id) then
      return some <| mark (.place context.ns.identity id) sourceLoc
        context.declaration.origin context.declaration.alignment binding.2
    let some (.field downcast _ fieldName) := context.ns.places[id.index]?
      | return none
    let some (.downcast base variantName) := context.ns.places[downcast.index]?
      | return none
    let some baseType := placeType? context base
      | throw "validated enum projection has no reconstructible base type"
    let some (.nominal ownerName _) := context.unit.tables.types[baseType.index]?
      | throw "validated downcast base is not nominal"
    let some declaration := context.ns.structs.find? (·.name == ownerName)
      | throw "validated downcast declaration is missing"
    let some variant := declaration.variants.find? (·.name == variantName)
      | throw "validated downcast variant is missing"
    let some fieldIndex := variant.fields.findIdx? (·.name == fieldName)
      | throw "validated downcast field is missing"
    let owner ← nameAt context.unit.tables declaration.name
    let variantName ← nameAt context.unit.tables variant.name
    let base ← placeText context loops base sourceLoc (fuel - 1)
    let binder := s!"__lir_variant_field_{fieldIndex}"
    let receiver := receiverPrefix ++ base ++ ")"
    let selected := if dereferenceSelected then "*" ++ binder else binder
    let pattern ← if fieldsArePositional context.unit.tables variant.fields then
        let fields := variant.fields.mapIdx fun index _ =>
          if index == fieldIndex then binder else "_"
        pure <| owner ++ "::" ++ variantName ++ "(" ++ commaSep fields ++ ")"
      else
        let fields ← variant.fields.mapIdxM fun index field => do
          let fieldName ← nameAt context.unit.tables field.name
          pure <| fieldName ++ ": " ++ if index == fieldIndex then binder else "_"
        pure <| owner ++ "::" ++ variantName ++ " { " ++ commaSep fields ++ " }"
    let arms := pattern ++ " => " ++ selected ++ ",\n_ => loop {}"
    let rendered := "(match " ++ receiver ++ " {\n" ++ indent arms ++ "\n})"
    pure <| some <| mark (.place context.ns.identity id) sourceLoc
      context.declaration.origin context.declaration.alignment rendered

  private partial def placeText (context : Context) (loops : List String)
      (id : PlaceId) (sourceLoc : LocId) (fuel : Nat) : Except String String := do
    if fuel == 0 then throw "cyclic validated place reached the Rust source backend"
    let some place := context.ns.places[id.index]?
      | throw s!"validated place {id.index} is missing"
    let rendered ← match place with
    | .localVar localId => localName context localId
    | .deref base => pure s!"(*{← placeText context loops base sourceLoc (fuel - 1)})"
    | .field base _ field =>
        pure s!"({← placeText context loops base sourceLoc (fuel - 1)}).{← fieldSelector context.unit.tables field}"
    | .index base index =>
        let baseText ← placeText context loops base sourceLoc (fuel - 1)
        let indexText ← exprText context loops index (fuel - 1)
        match placeType? context base with
        | some baseType => match context.unit.tables.types[baseType.index]? with
            | some (.tuple _) => pure s!"({baseText}).{indexText}"
            | _ => pure s!"({baseText})[{indexText}]"
        | none => throw "validated indexed place has no reconstructible base type"
    | .subslice base start stop fromEnd =>
        let baseText ← placeText context loops base sourceLoc (fuel - 1)
        if fromEnd then
          let upper := if stop == 0 then "" else s!"({baseText}).len() - {stop}"
          pure s!"({baseText})[{start}..{upper}]"
        else
          pure s!"({baseText})[{start}..{stop}]"
    | .downcast _ _ => throw "enum-downcast places require pattern reconstruction"
    pure <| mark (.place context.ns.identity id) sourceLoc context.declaration.origin
      context.declaration.alignment rendered

  private partial def patternText (context : Context) (loops : List String)
      (id : PatternId) (fuel : Nat) : Except String String := do
    if fuel == 0 then throw "cyclic validated pattern reached the Rust source backend"
    let some pattern := context.ns.patterns[id.index]?
      | throw s!"validated pattern {id.index} is missing"
    let rendered ← match pattern.kind with
    | .wildcard => pure "_"
    | .variable localId => localName context localId
    | .tuple elements =>
        let elements ← elements.mapM (patternText context loops · (fuel - 1))
        pure <| if elements.size == 1 then s!"({elements[0]!},)" else s!"({commaSep elements})"
    | .literal value => constText value
    | .range lower upper inclusive =>
        let lower ← lower.mapM constText
        let upper ← upper.mapM constText
        pure s!"{lower.getD ""}{if inclusive then "..=" else ".."}{upper.getD ""}"
    | .constructor name _ variant fields =>
      let owner ← nameAt context.unit.tables name
      let fields ← fields.mapM (patternText context loops · (fuel - 1))
      let some declaration := context.ns.structs.find? (·.name == name)
        | throw "validated pattern constructor declaration is missing"
      let declarationFields ← match variant with
        | none => pure declaration.fields
        | some variantName =>
            let some variant := declaration.variants.find? fun candidate =>
              context.unit.tables.names[candidate.name.index]?.map (·.name) == some variantName
              | throw s!"validated pattern variant `{variantName}` is missing"
            pure variant.fields
      let constructor := variant.map (owner ++ "::" ++ rustIdentifier ·) |>.getD owner
      unless declarationFields.size == fields.size do
        throw s!"validated constructor pattern `{constructor}` has wrong source arity"
      if fields.isEmpty then pure constructor
      else if fieldsArePositional context.unit.tables declarationFields then
        pure s!"{constructor}({commaSep fields})"
      else
        let assignments ← (declarationFields.zip fields).mapM fun (field, value) => do
          pure s!"{← nameAt context.unit.tables field.name}: {value}"
        pure <| constructor ++ " { " ++ commaSep assignments ++ " }"
    pure <| mark (.pattern context.ns.identity id) pattern.loc
      context.declaration.origin context.declaration.alignment rendered

  /-- Rust range indexing introduces `RangeFrom`/`Index` dependency calls in
  MIR. A slice-rest pattern, however, lowers directly to the same subslice
  place that LIR already carries. Reconstruct such a pattern for from-end
  subslice borrows so canonical Rust re-import stays inside the dependency-free
  source subset. -/
  private partial def subsliceBorrowText? (context : Context) (loops : List String)
      (kind : BorrowKind) (id : PlaceId) (sourceLoc : LocId) (fuel : Nat) :
      Except String (Option String) := do
    if fuel == 0 then throw "cyclic validated place reached the Rust source backend"
    let some (.subslice base start stop true) := context.ns.places[id.index]?
      | return none
    let borrowPrefix ← match kind with
      | .immutable => pure "&"
      | .mutable => pure "&mut "
      | .profile _ => throw "profile borrow has no standard Rust source spelling"
    let baseText ← placeText context loops base sourceLoc (fuel - 1)
    let binder := "__lir_subslice"
    let prefixParts := Array.replicate start "_"
    let suffixParts := Array.replicate stop "_"
    let pattern := "[" ++ commaSep
      (prefixParts ++ #[binder ++ " @ .."] ++ suffixParts) ++ "]"
    let rendered := "(match " ++ borrowPrefix ++ "(" ++ baseText ++ ") {\n" ++
      indent (pattern ++ " => " ++ binder ++ ",\n_ => loop {},") ++ "\n})"
    pure <| some <| mark (.place context.ns.identity id) sourceLoc
      context.declaration.origin context.declaration.alignment rendered

  /-- Recover a guarded slice-rest binding as one Rust `let ... else`.
  Printing the already-structured outer guard plus a nested pattern match makes
  rustc retain a duplicate length guard. This reconstruction emits the one
  source construct whose MIR is the guarded subslice already present in LIR. -/
  private partial def guardedSubsliceBlockText? (context : Context) (loops : List String)
      (statements : Array ExprId) (result : Option ExprId) (fuel : Nat) :
      Except String (Option String) := do
    if fuel == 0 then return none
    let originalResult := result
    let (statements, result) := flattenedSourceBlock context.ns statements result fuel
    let [lengthAssignmentId, guardAssignmentId, conditionalId] := statements.toList
      | return none
    let resultId ← match result with | some result => pure result | none => return none
    let lengthAssignment ← match context.ns.expressions[lengthAssignmentId.index]? with
      | some expression => pure expression
      | none => return none
    let .assign lengthPlace lengthValueId := lengthAssignment.kind | return none
    let some lengthLocal := directLocalPlace? context.ns lengthPlace | return none
    let guardAssignment ← match context.ns.expressions[guardAssignmentId.index]? with
      | some expression => pure expression
      | none => return none
    let .assign guardPlace predicateId := guardAssignment.kind | return none
    let some guardLocal := directLocalPlace? context.ns guardPlace | return none
    let predicate ← match context.ns.expressions[predicateId.index]? with
      | some expression => pure expression
      | none => return none
    let .operation (.primitive .greaterEqual) _ predicateArguments _ := predicate.kind
      | return none
    let [loadedLengthId, minimumId] := predicateArguments.toList | return none
    unless loadedLocal? context.ns loadedLengthId == some lengthLocal do return none
    let minimum ← match context.ns.expressions[minimumId.index]? with
      | some { kind := .value (.integer minimum) _, .. } => pure minimum
      | _ => return none
    if minimum < 0 then return none
    let conditional ← match context.ns.expressions[conditionalId.index]? with
      | some expression => pure expression
      | none => return none
    let .ifElse conditionId thenId (some elseId) := conditional.kind | return none
    unless loadedLocal? context.ns conditionId == some guardLocal do return none
    let thenExpression ← match context.ns.expressions[thenId.index]? with
      | some expression => pure expression
      | none => return none
    let .block thenStatements none := thenExpression.kind | return none
    let some borrowAssignmentId := thenStatements[0]? | return none
    let borrowAssignment ← match context.ns.expressions[borrowAssignmentId.index]? with
      | some expression => pure expression
      | none => return none
    let .assign borrowTarget borrowValueId := borrowAssignment.kind | return none
    let some borrowTargetLocal := directLocalPlace? context.ns borrowTarget | return none
    let borrowValue ← match context.ns.expressions[borrowValueId.index]? with
      | some expression => pure expression
      | none => return none
    let .operation (.borrow borrowKind subslicePlace) _ borrowArguments _ := borrowValue.kind
      | return none
    unless borrowArguments.isEmpty do return none
    let some (.subslice base start stop true) := context.ns.places[subslicePlace.index]?
      | return none
    unless minimum.toNat == start + stop do return none
    let [thenResultAssignmentId] :=
      thenStatements.extract 1 thenStatements.size |>.toList | return none
    let thenResultAssignment ←
      match context.ns.expressions[thenResultAssignmentId.index]? with
      | some expression => pure expression
      | none => return none
    let .assign thenResultPlace thenResultValue := thenResultAssignment.kind | return none
    let some resultLocal := directLocalPlace? context.ns thenResultPlace | return none
    let elseExpression ← match context.ns.expressions[elseId.index]? with
      | some expression => pure expression
      | none => return none
    let .block elseStatements none := elseExpression.kind | return none
    let [elseResultAssignmentId] := elseStatements.toList | return none
    let elseResultAssignment ←
      match context.ns.expressions[elseResultAssignmentId.index]? with
      | some expression => pure expression
      | none => return none
    let .assign elseResultPlace elseResultValue := elseResultAssignment.kind | return none
    unless directLocalPlace? context.ns elseResultPlace == some resultLocal do return none
    let resultExpression ← match context.ns.expressions[resultId.index]? with
      | some expression => pure expression
      | none => return none
    let .return_ returned := resultExpression.kind | return none
    let [returnedValue] := returned.toList | return none
    unless loadedLocal? context.ns returnedValue == some resultLocal do return none
    let lengthValue ← match context.ns.expressions[lengthValueId.index]? with
      | some expression => pure expression
      | none => return none
    let .operation (.primitive .length) _ lengthArguments _ := lengthValue.kind | return none
    let [lengthArgument] := lengthArguments.toList | return none
    let lengthDereference ← match context.ns.expressions[lengthArgument.index]? with
      | some expression => pure expression
      | none => return none
    let .operation (.reference .dereference) _ dereferenceArguments _ :=
      lengthDereference.kind | return none
    let [lengthSource] := dereferenceArguments.toList | return none
    let some lengthRoot := loadedPlace? context.ns lengthSource | return none
    let some (.deref baseRoot) := context.ns.places[base.index]? | return none
    let some lengthRootLocal := directLocalPlace? context.ns lengthRoot | return none
    let some baseRootLocal := directLocalPlace? context.ns baseRoot | return none
    unless baseRootLocal == lengthRootLocal do return none
    let borrowPrefix ← match borrowKind with
      | .immutable => pure "&"
      | .mutable => pure "&mut "
      | .profile _ => return none
    let baseText ← placeText context loops base conditional.loc
      (context.ns.places.size + 1)
    let binder ← localName context borrowTargetLocal
    let prefixParts := Array.replicate start "_"
    let suffixParts := Array.replicate stop "_"
    let pattern := "[" ++ commaSep
      (prefixParts ++ #[binder ++ " @ .."] ++ suffixParts) ++ "]"
    let elseValueText ← exprText context loops elseResultValue (fuel - 1)
    let thenValueText ← exprText context loops thenResultValue (fuel - 1)
    let binding := s!"let {pattern} = {borrowPrefix}({baseText}) else " ++
      rustBlock ("return " ++ elseValueText) ++ ";"
    let rendered := "{\n" ++ indent
      (lineSep #[binding, "return " ++ thenValueText]) ++ "\n}"
    let rendered := markExpressionTree context lengthAssignmentId rendered fuel
    let rendered := markExpressionTree context guardAssignmentId rendered fuel
    let rendered := markExpressionTree context conditionalId rendered fuel
    let rendered := markExpressionTree context resultId rendered fuel
    let rendered := markFlattenedResultBlocks context originalResult result rendered fuel
    pure <| some rendered

  private partial def primitiveText (context : Context) (loops : List String)
      (resultType : TypeId) (resultLoc : LocId) (kind : PrimitiveOperation)
      (arguments : Array ExprId)
      (fuel : Nat) : Except String String := do
    let values ← arguments.mapM (exprText context loops · (fuel - 1))
    let unary (operator : String) : Except String String := match values.toList with
      | [value] => pure s!"({operator}({value}))"
      | _ => throw s!"validated primitive {repr kind} has wrong source arity"
    let binary (operator : String) : Except String String := match values.toList with
      | [left, right] => pure s!"(({left}) {operator} ({right}))"
      | _ => throw s!"validated primitive {repr kind} has wrong source arity"
    let method (name : String) : Except String String := match values.toList with
      | [left, right] => pure s!"({left}).{name}({right})"
      | _ => throw s!"validated primitive {repr kind} has wrong source arity"
    match kind with
    | .tuple => pure <| if values.size == 1 then s!"({values[0]!},)" else s!"({commaSep values})"
    | .vector => pure s!"[{commaSep values}]"
    | .repeatVector => match values.toList, context.ns.tables.types[resultType.index]? with
        | [value], some (.vector _ (some (.integer length))) => pure s!"[{value}; {length}]"
        | _, _ => throw "validated repeated vector has no fixed integer length"
    | .swapVector | .insertVector | .removeVector | .concatVector |
        .reverseSliceVector | .destroyEmptyVector =>
        -- Rust reshapes a `Vec` by statement, not by expression, and the Rust
        -- frontend never produces these; a Move unit reaching this backend is
        -- the real error.
        throw "validated vector reshaping has no Rust source form"
    | .pushVector =>
        -- Rust grows a `Vec` by statement, not by expression, so there is no
        -- source form to render here. The Rust frontend never produces this
        -- operation; a Move unit reaching the Rust backend is the real error.
        throw "validated vector push has no Rust source form"
    | .add => method "wrapping_add"
    | .subtract => method "wrapping_sub"
    | .multiply => method "wrapping_mul"
    | .overflowingAdd => method "overflowing_add"
    | .overflowingSubtract => method "overflowing_sub"
    | .overflowingMultiply => method "overflowing_mul"
    | .divide => binary "/"
    | .modulo => binary "%"
    | .bitwiseOr => binary "|"
    | .bitwiseAnd => binary "&"
    | .bitwiseXor => binary "^"
    | .logicalAnd => binary "&&"
    | .logicalOr => binary "||"
    | .equal | .identical => binary "=="
    | .notEqual => binary "!="
    | .less => binary "<"
    | .greater => binary ">"
    | .lessEqual => binary "<="
    | .greaterEqual => binary ">="
    | .logicalNot | .bitwiseNot => unary "!"
    | .negate => unary "-"
    | .copyValue | .moveValue => match values.toList with
        | [value] => pure value
        | _ => throw s!"validated primitive {repr kind} has wrong source arity"
    | .cast => match values.toList with
        | [value] => pure s!"(({value}) as {← expressionTypeUseText context resultType resultLoc})"
        | _ => throw "validated cast has wrong source arity"
    | .length => match values.toList with
        | [value] => pure s!"({value}).len()"
        | _ => throw "validated length has wrong source arity"
    | .index => match values.toList with
        | [value, index] => pure s!"({value})[{index}]"
        | _ => throw "validated index has wrong source arity"
    | .shiftLeft => match values.toList with
        | [value, amount] => pure s!"({value}).wrapping_shl(({amount}) as u32)"
        | _ => throw "validated shift has wrong source arity"
    | .shiftRight => match values.toList with
        | [value, amount] => pure s!"({value}).wrapping_shr(({amount}) as u32)"
        | _ => throw "validated shift has wrong source arity"
    | .checkedAdd _ | .checkedSubtract _ | .checkedMultiply _ | .checkedModulo _ |
        .checkedDivide _ | .checkedShiftLeft _ | .checkedShiftRight _ | .checkedNegate _ |
        .checkedCast _ | .slice | .range | .implies | .equivalent |
        .containsVector | .indexOfVector | .checkVectorIndex _ =>
        throw s!"primitive {repr kind} requires source-level control reconstruction"

  private partial def callText (context : Context) (loops : List String)
      (kind : CallKind) (arguments : Array ExprId) (fuel : Nat) : Except String String := do
    let values ← arguments.mapM (exprText context loops · (fuel - 1))
    match kind with
    | .function callee => pure s!"{← nameAt context.unit.tables callee.name}({commaSep values})"
    | .invoke => match values.toList with
        | callee :: arguments => pure s!"({callee})({", ".intercalate arguments})"
        | [] => throw "validated indirect call has no callee"
    | .closure function =>
        if values.isEmpty then pure (← nameAt context.unit.tables function.name)
        else throw "capturing closures require source-level environment reconstruction"
    | .constructor constructor variant =>
        constructorText context constructor variant values
    | .destructor .. => throw "destructor calls require the M4 drop source model"
    | .extension .. => throw "extension calls have no standard Rust source spelling"

  private partial def enumDiscriminantBlockText? (context : Context) (loops : List String)
      (statements : Array ExprId) (result : Option ExprId) (fuel : Nat) :
    Except String (Option String) := do
    let originalResult := result
    let (statements, result) := flattenedSourceBlock context.ns statements result fuel
    let [discriminantAssignmentId, matchId] := statements.toList | return none
    let discriminantAssignment ← match context.ns.expressions[discriminantAssignmentId.index]? with
      | some value => pure value
      | none => return none
    let .assign discriminantPlace discriminantValueId := discriminantAssignment.kind
      | return none
    let some discriminantLocal := directLocalPlace? context.ns discriminantPlace | return none
    let discriminantValue ← match context.ns.expressions[discriminantValueId.index]? with
      | some value => pure value
      | none => return none
    let .operation (.data (.discriminant reference)) _ discriminantArguments _ :=
        discriminantValue.kind | return none
    let [enumValueId] := discriminantArguments.toList | return none
    let some enumPlace := loadedPlace? context.ns enumValueId | return none
    let some enumLocal := directLocalPlace? context.ns enumPlace | return none
    unless reference.namespaceId == context.ns.identity do return none
    let some declaration := context.ns.structs.find? (·.name == reference.name) | return none
    if declaration.variants.any (·.fields.isEmpty) then return none
    let matchExpression ← match context.ns.expressions[matchId.index]? with
      | some value => pure value
      | none => return none
    let .match_ scrutinee arms := matchExpression.kind | return none
    let some scrutineePlace := loadedPlace? context.ns scrutinee | return none
    unless directLocalPlace? context.ns scrutineePlace == some discriminantLocal do return none
    if arms.any (·.guard.isSome) then return none
    unless arms.size == declaration.variants.size do return none
    unless arms.all fun arm =>
        match context.ns.patterns[arm.pattern.index]? with
        | some { kind := .literal (.integer discriminant), .. } =>
            declaration.variants.any (·.discriminant == some discriminant)
        | _ => false do return none
    unless declaration.variants.all fun variant =>
        (arms.filter (fun (arm : MatchArm) =>
          match context.ns.patterns[arm.pattern.index]? with
          | some { kind := .literal (.integer discriminant), .. } =>
              variant.discriminant == some discriminant
          | _ => false)).size == 1 do return none
    let resultId ← match result with | some value => pure value | none => return none
    let resultExpression ← match context.ns.expressions[resultId.index]? with
      | some value => pure value
      | none => return none
    let .return_ returned := resultExpression.kind | return none
    let [returnedValue] := returned.toList | return none
    let some resultLocal := loadedLocal? context.ns returnedValue | return none
    let enumValue ← exprText context loops enumValueId (fuel - 1)
    let renderedArms ← arms.mapIdxM fun armIndex arm => do
      let pattern ← match context.ns.patterns[arm.pattern.index]? with
        | some value => pure value
        | none => throw "validated enum discriminant pattern is missing"
      let .literal (.integer discriminant) := pattern.kind
        | throw "enum discriminant source reconstruction requires literal arms"
      let some variant := declaration.variants.find? (·.discriminant == some discriminant)
        | throw s!"enum discriminant arm `{discriminant}` has no matching variant"
      let armBody ← match context.ns.expressions[arm.body.index]? with
        | some value => pure value
        | none => throw "validated enum discriminant arm body is missing"
      let .block armStatements armResult := armBody.kind
        | throw "enum discriminant source reconstruction requires assignment arms"
      let (armStatements, armResult) :=
        flattenedSourceBlock context.ns armStatements armResult (fuel - 1)
      unless armResult.isNone do
        throw "enum discriminant arm has a value result"
      let (targetPlace, fieldValueId) ← match armStatements.toList with
        | [assignmentId] => do
            let assignment ← match context.ns.expressions[assignmentId.index]? with
              | some value => pure value
              | none => throw "validated enum field assignment is missing"
            let .assign targetPlace fieldValueId := assignment.kind
              | throw "enum discriminant arm is not an assignment"
            pure (targetPlace, fieldValueId)
        | [valueAssignmentId, forwardingId] => do
            let valueAssignment ← match context.ns.expressions[valueAssignmentId.index]? with
              | some value => pure value
              | none => throw "validated enum field temporary assignment is missing"
            let .assign temporaryPlace fieldValueId := valueAssignment.kind
              | throw "enum discriminant arm temporary is not an assignment"
            let some temporaryLocal := directLocalPlace? context.ns temporaryPlace
              | throw "enum discriminant arm temporary is not a local"
            let forwarding ← match context.ns.expressions[forwardingId.index]? with
              | some value => pure value
              | none => throw "validated enum field forwarding assignment is missing"
            let .assign targetPlace forwardedValue := forwarding.kind
              | throw "enum discriminant arm forwarding is not an assignment"
            unless loadedLocal? context.ns forwardedValue == some temporaryLocal do
              throw "enum discriminant arm forwards a different temporary"
            pure (targetPlace, fieldValueId)
        | _ => throw s!"enum discriminant arm requires one field assignment, found \
            {armStatements.size}"
      unless directLocalPlace? context.ns targetPlace == some resultLocal do
        throw "enum discriminant arms assign different result locals"
      let fieldValue ← match context.ns.expressions[fieldValueId.index]? with
        | some value => pure value
        | none => throw "validated enum field value is missing"
      let fieldPlace ← match fieldValue.kind with
        | .operation (.copy place) _ arguments _
        | .operation (.read place) _ arguments _ =>
            if arguments.isEmpty then pure place
            else throw "enum field load has source arguments"
        | _ => throw "enum discriminant arm requires a copied field"
      let some (.field downcast _ fieldName) := context.ns.places[fieldPlace.index]?
        | throw "enum discriminant arm does not select a field"
      let some (.downcast basePlace selectedVariant) := context.ns.places[downcast.index]?
        | throw "enum discriminant arm field has no downcast"
      unless directLocalPlace? context.ns basePlace == some enumLocal do
        throw "enum discriminant arm selects a different enum value"
      unless selectedVariant == variant.name do
        throw "enum discriminant arm selects the wrong variant"
      let some fieldIndex := variant.fields.findIdx? (·.name == fieldName)
        | throw "enum discriminant arm selects an unknown field"
      let binder := s!"__lir_match_field_{armIndex}_{fieldIndex}"
      let patternFields := variant.fields.mapIdx fun index _ =>
        if index == fieldIndex then binder else "_"
      let owner ← nameAt context.unit.tables declaration.name
      let variantName ← nameAt context.unit.tables variant.name
      let constructor ← if fieldsArePositional context.unit.tables variant.fields then
          pure s!"{owner}::{variantName}({commaSep patternFields})"
        else
          let fields : Array String ← variant.fields.mapIdxM fun index field => do
            let fieldName ← nameAt context.unit.tables field.name
            pure s!"{fieldName}: {patternFields[index]!}"
          pure <| owner ++ "::" ++ variantName ++ " { " ++ commaSep fields ++ " }"
      let constructor := mark (.pattern context.ns.identity arm.pattern) pattern.loc
        context.declaration.origin context.declaration.alignment constructor
      let constructor := mark (.place context.ns.identity basePlace) fieldValue.loc
        context.declaration.origin context.declaration.alignment constructor
      let armContext := { context with
        fieldBindings := #[(fieldPlace, binder)] ++ context.fieldBindings }
      pure s!"{constructor} => {← exprText armContext loops arm.body (fuel - 1)},"
    let directMatch := "match " ++ enumValue ++ " {\n" ++
      indent (lineSep renderedArms) ++ "\n}"
    let directMatch := mark (.place context.ns.identity discriminantPlace)
      discriminantAssignment.loc context.declaration.origin context.declaration.alignment directMatch
    let directMatch := mark (.expression context.ns.identity discriminantValueId)
      discriminantValue.loc context.declaration.origin context.declaration.alignment directMatch
    let directMatch := mark (.expression context.ns.identity discriminantAssignmentId)
      discriminantAssignment.loc context.declaration.origin context.declaration.alignment directMatch
    let scrutineeExpression := context.ns.expressions[scrutinee.index]!
    let directMatch := mark (.place context.ns.identity scrutineePlace) scrutineeExpression.loc
      context.declaration.origin context.declaration.alignment directMatch
    let directMatch := mark (.expression context.ns.identity scrutinee) scrutineeExpression.loc
      context.declaration.origin context.declaration.alignment directMatch
    let directMatch := mark (.expression context.ns.identity matchId) matchExpression.loc
      context.declaration.origin context.declaration.alignment directMatch
    let directMatch := markFlattenedResultBlocks context originalResult result directMatch fuel
    let returnedText ← exprText context loops resultId (fuel - 1)
    pure <| some <| "{\n" ++ indent (directMatch ++ ";\n" ++ returnedText) ++ "\n}"

  private partial def exprText (context : Context) (loops : List String)
      (id : ExprId) (fuel : Nat) : Except String String := do
    if fuel == 0 then throw "cyclic validated expression reached the Rust source backend"
    let some expression := context.ns.expressions[id.index]?
      | throw s!"validated expression {id.index} is missing"
    let rendered ← match expression.kind with
    | .value value _ => constText value
    | .constant constant => nameAt context.unit.tables constant.name
    | .localVar localId => localName context localId
    | .operation operation _ arguments _ => match operation with
        | .move place =>
            match ← downcastFieldText? context loops place expression.loc "(" false
                (context.ns.places.size + 1) with
            | some value => pure value
            | none => placeText context loops place expression.loc (context.ns.places.size + 1)
        | .copy place | .read place =>
            match ← downcastFieldText? context loops place expression.loc "&(" true
                (context.ns.places.size + 1) with
            | some value => pure value
            | none => placeText context loops place expression.loc (context.ns.places.size + 1)
        | .borrow kind place =>
            let borrowPrefix := match kind with
              | .immutable => "&"
              | .mutable => "&mut "
              | .profile _ => ""
            if borrowPrefix.isEmpty then throw "profile borrow has no standard Rust source spelling"
            match ← subsliceBorrowText? context loops kind place expression.loc
                (context.ns.places.size + 1) with
            | some value => pure value
            | none =>
                let receiverPrefix :=
                  if kind == .mutable then "&mut (" else "&("
                match ← downcastFieldText? context loops place expression.loc receiverPrefix false
                    (context.ns.places.size + 1) with
                | some value => pure value
                | none => pure s!"{borrowPrefix}{← placeText context loops place expression.loc
                    (context.ns.places.size + 1)}"
        | .write place => match arguments.toList with
            | [value] =>
                let place ← placeText context loops place expression.loc
                  (context.ns.places.size + 1)
                let value ← exprText context loops value (fuel - 1)
                pure ("{ " ++ place ++ " = " ++ value ++ "; () }")
            | _ => throw "validated place write has wrong source arity"
        | .drop place => pure s!"drop({← placeText context loops place expression.loc
            (context.ns.places.size + 1)})"
        | .call kind => callText context loops kind arguments fuel
        | .primitive kind =>
            primitiveText context loops expression.typeId expression.loc kind arguments fuel
        | .assert => match arguments.toList with
            | [condition] => pure s!"assert!({← exprText context loops condition (fuel - 1)})"
            | _ => throw "validated assertion has wrong source arity"
        | .data (.discriminant reference) => match arguments.toList with
            | [argument] =>
                unless reference.namespaceId == context.ns.identity do
                  throw "external enum discriminants require emitted dependency modules"
                let some declaration := context.ns.structs.find? (·.name == reference.name)
                  | throw "validated discriminant declaration is missing"
                unless !declaration.variants.isEmpty do
                  throw "validated discriminant target is not an enum"
                let owner ← nameAt context.unit.tables declaration.name
                let value ← exprText context loops argument (fuel - 1)
                let arms ← declaration.variants.mapM fun variant => do
                  let variantName ← nameAt context.unit.tables variant.name
                  let pattern := if variant.fields.isEmpty then ""
                    else if fieldsArePositional context.unit.tables variant.fields then "(..)"
                    else " { .. }"
                  let some discriminant := variant.discriminant
                    | throw s!"validated enum variant `{variantName}` has no integer discriminant"
                  pure s!"{owner}::{variantName}{pattern} => {discriminant},"
                pure <| "(match &(" ++ value ++ ") {\n" ++ indent (lineSep arms) ++ "\n})"
            | _ => throw "validated discriminant operation has wrong source arity"
        | .reference .dereference => match arguments.toList with
            | [argument] => pure s!"(*({← exprText context loops argument (fuel - 1)}))"
            | _ => throw "validated reference dereference has wrong source arity"
        | .reference _ | .data _ | .global _ | .specification _ | .profile _ _ =>
            throw s!"operation {repr operation} is outside the current standard Rust source subset"
    | .block statements result =>
        match ← guardedSubsliceBlockText? context loops statements result (fuel - 1) with
        | some rendered => pure rendered
        | none => match forwardedReturnBlock? context.ns statements result with
        | some (assignmentId, targetPlace, valueId, resultId, returnedId) =>
            let assignment := context.ns.expressions[assignmentId.index]!
            let resultExpression := context.ns.expressions[resultId.index]!
            let returnedExpression := context.ns.expressions[returnedId.index]!
            let value ← exprText context loops valueId (fuel - 1)
            let rendered := "return " ++ value
            let rendered := mark (.place context.ns.identity targetPlace) assignment.loc
              context.declaration.origin context.declaration.alignment rendered
            let rendered := mark (.expression context.ns.identity assignmentId) assignment.loc
              context.declaration.origin context.declaration.alignment rendered
            let rendered := match loadedPlace? context.ns returnedId with
              | some returnedPlace =>
                  mark (.place context.ns.identity returnedPlace) returnedExpression.loc
                    context.declaration.origin context.declaration.alignment rendered
              | none => rendered
            let rendered := mark (.expression context.ns.identity returnedId)
              returnedExpression.loc context.declaration.origin context.declaration.alignment rendered
            let rendered := mark (.expression context.ns.identity resultId) resultExpression.loc
              context.declaration.origin context.declaration.alignment rendered
            pure <| "{\n" ++ indent rendered ++ "\n}"
        | none =>
              match ← enumDiscriminantBlockText? context loops statements result
                  (fuel - 1) with
              | some rendered => pure rendered
              | none =>
                  let statements ← statements.mapM fun statement => do
                    pure s!"{← exprText context loops statement (fuel - 1)};"
                  let result ← result.mapM (exprText context loops · (fuel - 1))
                  let lines := statements ++ result.toArray
                  pure <| if lines.isEmpty then "{}" else
                    "{\n" ++ indent (lineSep lines) ++ "\n}"
    | .letDecl pattern value body =>
        let pattern ← patternText context loops pattern (fuel - 1)
        let body ← exprText context loops body (fuel - 1)
        match value with
        | none => pure body
        | some value =>
            let value ← exprText context loops value (fuel - 1)
            let binding := s!"{pattern} = {value};"
            pure <| "{\n" ++ indent binding ++ "\n" ++ indent body ++ "\n}"
    | .ifElse condition thenBranch elseBranch =>
        let condition ← exprText context loops condition (fuel - 1)
        let thenBranch ← exprText context loops thenBranch (fuel - 1)
        match elseBranch with
        | some elseBranch =>
            let elseBranch ← exprText context loops elseBranch (fuel - 1)
            pure s!"if {condition} {rustBlock thenBranch} else {rustBlock elseBranch}"
        | none => pure s!"if {condition} {rustBlock thenBranch}"
    | .match_ scrutinee arms =>
        let scrutinee ← exprText context loops scrutinee (fuel - 1)
        let hasCatchAll := arms.any fun arm =>
          match context.ns.patterns[arm.pattern.index]? with
          | some { kind := .wildcard, .. } | some { kind := .variable _, .. } => true
          | _ => false
        let arms ← arms.mapM fun arm => do
          let pattern ← patternText context loops arm.pattern (fuel - 1)
          let guard ← arm.guard.mapM (exprText context loops · (fuel - 1))
          let guard := guard.map (" if " ++ ·) |>.getD ""
          pure s!"{pattern}{guard} => {← exprText context loops arm.body (fuel - 1)},"
        let arms := if hasCatchAll then arms else arms.push "_ => loop {},"
        pure <| s!"match {scrutinee} " ++ "{\n" ++ indent (lineSep arms) ++ "\n}"
    | .loop _ body =>
        let label := s!"'lir_loop_{loops.length}"
        let body ← exprText context (label :: loops) body (fuel - 1)
        pure s!"{label}: loop {rustBlock body}"
    | .break_ nest value =>
        let label ← loopTarget loops nest
        match value with
        | some value => pure s!"break {label} {← exprText context loops value (fuel - 1)}"
        | none => pure s!"break {label}"
    | .continue_ nest => pure s!"continue {← loopTarget loops nest}"
    | .return_ values =>
        let values ← values.mapM (exprText context loops · (fuel - 1))
        match values.toList with
        | [] => pure "return"
        | [value] => pure s!"return {value}"
        | _ => pure s!"return ({commaSep values})"
    | .throw_ .panic values =>
        if values.isEmpty then pure "std::process::abort()"
        else throw "panic payloads have no faithful standard Rust source spelling"
    | .throw_ .abort _ =>
        throw "non-panic abort outcomes have no faithful standard Rust source spelling"
    | .throw_ (.profile _) _ => throw "profile throw has no standard Rust source spelling"
    | .assign place value =>
        pure s!"{← placeText context loops place expression.loc
          (context.ns.places.size + 1)} = {← exprText context loops value (fuel - 1)}"
    | .assignPattern pattern value =>
        pure s!"{← patternText context loops pattern (fuel - 1)} = {← exprText context loops value (fuel - 1)}"
    | .quantifier .. | .spec _ =>
        throw "logical expressions have no executable Rust source spelling"
    pure <| mark (.expression context.ns.identity id) expression.loc
      context.declaration.origin context.declaration.alignment rendered
end

private def structText (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (declaration : StructDecl) : Except String String := do
  unless declaration.abilities.isEmpty && declaration.properties.isEmpty &&
      declaration.locals.isEmpty && contractIsEmpty declaration.contract &&
      declaration.attributes.isEmpty do
    throw "Rust source reconstruction does not yet support nominal semantic metadata"
  let name ← nameAt unit.tables declaration.name
  let declarationName := name ++
    (← genericParametersText unit.tables ns.identity declaration.name declaration.generics)
  let predicates ← genericPredicatesText unit.tables declaration.generics
  let rendered ← if declaration.variants.isEmpty then
    if declaration.fields.isEmpty then pure s!"pub struct {declarationName}{predicates};" else
      if fieldsArePositional unit.tables declaration.fields then
        let fields ← declaration.fields.mapM fun field => do
          let rendered :=
            s!"pub {← declarationTypeUseText unit ns.identity declaration.generics field.type}"
          pure <| markWithoutImportProvenance
            (.field ns.identity declaration.name none field.name) field.loc rendered
        pure s!"pub struct {declarationName}({commaSep fields}){predicates};"
      else
        let fields ← declaration.fields.mapM fun field => do
          let rendered :=
            s!"pub {← nameAt unit.tables field.name}: {← declarationTypeUseText unit
              ns.identity declaration.generics field.type},"
          pure <| markWithoutImportProvenance
            (.field ns.identity declaration.name none field.name) field.loc rendered
        pure <| s!"pub struct {declarationName}{predicates} " ++
          "{\n" ++ indent (lineSep fields) ++ "\n}"
  else
    let variants ← declaration.variants.mapM fun variant => do
      let name ← nameAt unit.tables variant.name
      let some discriminant := variant.discriminant
        | throw s!"Rust enum variant `{name}` has no integer discriminant"
      let rendered ← if variant.fields.isEmpty then pure s!"{name} = {discriminant},"
      else if fieldsArePositional unit.tables variant.fields then
        let fields ← variant.fields.mapM fun field => do
          let rendered ← declarationTypeUseText unit ns.identity declaration.generics field.type
          pure <| markWithoutImportProvenance
            (.field ns.identity declaration.name (some variant.name) field.name)
            field.loc rendered
        pure s!"{name}({commaSep fields}) = {discriminant},"
      else
        let fields ← variant.fields.mapM fun field => do
          let rendered := s!"{← nameAt unit.tables field.name}: {← declarationTypeUseText unit
            ns.identity declaration.generics field.type},"
          pure <| markWithoutImportProvenance
            (.field ns.identity declaration.name (some variant.name) field.name)
            field.loc rendered
        pure <| s!"{name} " ++ "{ " ++ " ".intercalate fields.toList ++
          s!" }} = {discriminant},"
      pure <| markWithoutImportProvenance
        (.variant ns.identity declaration.name variant.name) variant.loc rendered
    pure <| s!"#[repr(isize)]\npub enum {declarationName}{predicates} " ++
      "{\n" ++ indent (lineSep variants) ++ "\n}"
  pure <| markWithoutImportProvenance (.nominal ns.identity declaration.name)
    declaration.loc rendered

private def functionText (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (functionId : FunctionId) (declaration : FunctionDecl FunctionBody) : Except String String := do
  unless declaration.profile == .rust do
    throw "standard Rust source backend received a non-Rust function"
  unless contractIsEmpty declaration.contract && declaration.pragmas.isEmpty &&
      declaration.profileData.isEmpty && declaration.attributes.isEmpty do
    throw "Rust source reconstruction does not yet support function semantic metadata"
  let name ← nameAt unit.tables declaration.name
  let genericParameters ← functionGenericParametersText unit.tables ns.identity functionId declaration
  let predicates ← genericPredicatesText unit.tables declaration.signature.generics
    declaration.signature.predicates
  let parameters ← declaration.signature.parameters.zipIdx.mapM fun (parameter, index) => do
    let localDecl ← match declaration.locals[index]? with
      | some localDecl => pure localDecl
      | none => throw s!"validated parameter local {index} is missing"
    let mutability := if parameter.mutable || localDecl.mutable then "mut " else ""
    let parameterType ← functionTypeUseText unit ns.identity declaration parameter.typeUse
    let id : LocalId := ⟨index⟩
    let markedLocal := mark (.local ns.identity functionId id) localDecl.loc declaration.origin
      declaration.alignment (localIdentifier localDecl id)
    pure s!"{mutability}{markedLocal}: {parameterType}"
  let result ← match declaration.signature.results.toList with
    | [] => pure "()"
    | [result] => functionTypeUseText unit ns.identity declaration result
    | results =>
        let results ← results.toArray.mapM fun result =>
          functionTypeUseText unit ns.identity declaration result
        pure s!"({commaSep results})"
  let localDeclarations ← (declaration.locals.drop declaration.signature.parameters.size).mapIdxM
      fun index localDecl => do
        match unit.tables.types[localDecl.type.typeId.index]? with
        | some (Ty.never) => pure none
        | _ =>
            let localType ← functionTypeUseText unit ns.identity declaration localDecl.type
            let id : LocalId := ⟨declaration.signature.parameters.size + index⟩
            let markedLocal := mark (.local ns.identity functionId id) localDecl.loc
              declaration.origin declaration.alignment (localIdentifier localDecl id)
            pure <| some s!"let mut {markedLocal}: {localType};"
  let localDeclarations := localDeclarations.filterMap id
  let context : Context := { unit, ns, functionId, declaration }
  let body ← match declaration.body with
    | .absent => throw s!"Rust function `{name}` has no body"
    | .structured root => exprText context [] root (ns.expressions.size + 1)
  let lines := localDeclarations.push body
  let rendered := s!"pub fn {name}{genericParameters}({commaSep parameters}) -> {result}{predicates} " ++
    "{\n" ++ indent (lineSep lines) ++ "\n}"
  pure <| mark (.function ns.identity functionId) declaration.loc declaration.origin
    declaration.alignment rendered

private def sourceOrderKey (unit : ValidatedUnit) (loc : LocId) (fallback : Nat) :
    Nat × Nat × Nat :=
  match unit.tables.locations[loc.index]?.bind (·.primary) with
  | some range => (range.file.index, range.startByte, fallback)
  | none => (Nat.succ unit.tables.files.size, fallback, fallback)

private def commentText (comment : Comment) : String :=
  let text := comment.text
  if text.startsWith "//" || (text.startsWith "/*" && text.endsWith "*/") then text
  else if text.startsWith "--" then "//" ++ text.drop 2
  else if text.startsWith "/-" && text.endsWith "-/" then
    "/*" ++ (text.drop 2).dropEnd 2 ++ "*/"
  else "// " ++ text

/-- Emit a canonical standard-Rust crate for the currently supported
target-independent structured subset. The input is necessarily a checked
shared LIR unit; unsupported validated features are returned as source-backend
errors instead of being approximated. -/
def renderWithSourceMap (unit : ValidatedUnit) : Except String RenderedSource := do
  let ns ← match unit.namespaces.toList with
    | [ns] => pure ns
    | _ => throw "standard Rust source currently requires exactly one owned namespace"
  unless ns.profile == some .rust do
    throw "standard Rust source backend requires a Rust-profile namespace"
  unless unit.dependencies.isEmpty && ns.imports.isEmpty do
    throw "standard Rust source backend does not yet reconstruct dependency imports"
  unless ns.profileMetadata.isEmpty && ns.attributes.isEmpty && ns.pragmas.isEmpty do
    throw "standard Rust source backend does not yet reconstruct namespace semantic metadata"
  unless ns.constants.isEmpty && ns.traits.isEmpty &&
      ns.implementations.isEmpty && ns.specFunctions.isEmpty &&
      ns.specVars.isEmpty && ns.invariants.isEmpty && ns.intrinsics.isEmpty do
    throw "standard Rust source backend received unsupported declarations"
  let mut items : Array (Nat × Nat × Nat × String) := #[]
  let mut fallback := 0
  for declaration in ns.structs do
    let (file, start, _) := sourceOrderKey unit declaration.loc fallback
    items := items.push (file, start, fallback, ← structText unit ns declaration)
    fallback := fallback + 1
  for (declaration, index) in ns.functions.zipIdx do
    let (file, start, _) := sourceOrderKey unit declaration.loc fallback
    items := items.push
      (file, start, fallback, ← functionText unit ns ⟨index⟩ declaration)
    fallback := fallback + 1
  for comment in ns.comments do
    let (file, start, _) := sourceOrderKey unit comment.loc fallback
    items := items.push (file, start, fallback, commentText comment)
    fallback := fallback + 1
  let sortedItems := items.qsort fun left right =>
    left.1 < right.1 || (left.1 == right.1 &&
      (left.2.1 < right.2.1 || (left.2.1 == right.2.1 && left.2.2.1 < right.2.2.1)))
  finishMarkedSource <| "\n\n".intercalate (sortedItems.map (·.2.2.2)).toList ++ "\n"

/-- Compatibility entry point for callers which only need canonical source text. -/
def render (unit : ValidatedUnit) : Except String String := do
  pure (← renderWithSourceMap unit).text

end LeanerIR.Rust.Source
