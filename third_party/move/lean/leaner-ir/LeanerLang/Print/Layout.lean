-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Elab
import LeanerLang.Operators
import LeanerLang.Comments

/-!
# LeanerLang printer layout engine

This is the internal layout stage of `LeanerLang.Print`. It formats the
frontend-owned AST with `Std.Format`; public callers should use the printer so
canonical source always crosses LIR before layout.
-/

namespace LeanerLang.Print.Layout

open Lean
open Std (Format)

private abbrev Doc := Format

private def text (value : String) : Doc := Format.text value
private def soft : Doc := Format.line
private def hard : Doc := text "\n"
private def vcat (values : Array Doc) : Doc := Format.joinSep values.toList hard
private def blankSep (values : Array Doc) : Doc :=
  Format.joinSep values.toList (hard ++ hard)

private def commaSep (values : Array Doc) : Doc :=
  Format.group <| Format.joinSep values.toList (text "," ++ soft)

private partial def flattenedWidth? : Doc → Option Nat
  | .nil => some 0
  | .line => some 1
  | .align _ => some 0
  | .text value => if value.contains '\n' then none else some value.length
  | .nest _ value | .group value _ | .tag _ value => flattenedWidth? value
  | .append left right => return (← flattenedWidth? left) + (← flattenedWidth? right)

/-- Pack the contents of a hanging list for the default 80-column layout.
The surrounding declaration/expression contributes at least four columns of
indentation, leaving a conservative 72 columns for entries. The outer group
still makes the final flat-versus-hanging decision; pre-packing avoids nested
`fill` groups independently breaking commas while leaving `(` and `)` flat. -/
private def commaFill (values : Array Doc) : Doc :=
  let limit := 72
  let (lines, current, _) := values.foldl
    (init := (#[], #[], 0)) fun (lines, current, currentWidth) value =>
      let width := (flattenedWidth? value).getD (limit + 1)
      let separatedWidth := if current.isEmpty then width else currentWidth + 2 + width
      if !current.isEmpty && separatedWidth > limit then
        (lines.push current, #[value], width)
      else
        (lines, current.push value, separatedWidth)
  let lines := if current.isEmpty then lines else lines.push current
  let lines := lines.map fun line => Format.joinSep line.toList (text ", ")
  Format.joinSep lines.toList (text "," ++ soft)

private def delimited (left right : String) (values : Array Doc) : Doc :=
  text left ++ Format.nest 2 (commaSep values) ++ text right

/-- A delimited list is either entirely flat or hangs two columns from the
surrounding expression. Once hanging, fill as many entries as fit on each line
and align the closing delimiter with the surrounding expression. `align`
flattens to no text, unlike `line`, so the flat spelling has no padding inside
its delimiters. -/
private def hangingDelimited (head : Doc) (left right : String)
    (values : Array Doc) (tail : Doc := .nil) : Doc :=
  if values.isEmpty then head ++ text left ++ text right ++ tail
  else Format.group <| head ++ text left ++
    Format.nest 2 (Format.align false ++ commaFill values) ++
    Format.align false ++ text right ++ tail

/-- Constructor fields use interior spaces in their flat spelling, but become
an ordinary indented brace block as one group when they exceed the width. -/
private def hangingBraced (head : Doc) (values : Array Doc) : Doc :=
  Format.group <| head ++ text " {" ++
    Format.nest 2 (soft ++ commaFill values) ++ soft ++ text "}"

private def call (head : Doc) (arguments : Array Doc) : Doc :=
  hangingDelimited head "(" ")" arguments

private def block (head : Doc) (entries : Array Doc) : Doc :=
  head ++ Format.nest 2 (hard ++ vcat entries)

private def trimLineEnd (value : String) : String :=
  String.ofList (value.toList.reverse.dropWhile (· == ' ') |>.reverse)

private def renderDoc (document : Doc) (width : Nat) : String :=
  "\n".intercalate <| (Format.pretty document width).splitOn "\n" |>.map trimLineEnd

private def byteLiteral (values : Array UInt8) : String :=
  if values.all fun byte =>
      let value := byte.toNat
      value >= 0x20 && value < 0x7f && value != 0x22 && value != 0x5c then
    s!"b{repr (String.ofList (values.toList.map fun byte => Char.ofNat byte.toNat))}"
  else
    "b[" ++ ", ".intercalate (values.toList.map (toString ·.toNat)) ++ "]"

private def reservedIdentifier (value : String) : Bool :=
  value ∈ [
    "move", "rust", "Unit", "Never", "Bool", "Char", "string", "Bytes",
    "Address", "Signer", "UInt", "SInt", "UPtr", "IPtr", "Nat", "Int",
    "u8", "u16", "u32", "u64", "u128", "u256",
    "i8", "i16", "i32", "i64", "i128", "i256", "usize", "isize",
    "Range", "Vector", "Fn", "const", "type", "lifetime", "evidence", "mut",
    "private", "public", "package", "friend", "entry", "native", "opaque",
    "deprecated", "view", "pragma", "let_pre", "let_post", "modifies", "reads",
    "requires", "ensures", "aborts_if", "assert", "assume",
    "invariant", "spec", "fun", "module", "namespace", "using", "where", "struct",
    "enum", "has", "Copy", "Drop", "Store", "Key", "true", "false",
    "abort", "panic", "do", "let", "loop", "while", "for", "break", "continue", "forall",
    "exists", "in", "immutable", "if", "then", "else", "return", "old",
    "copy", "drop", "discriminant", "invoke", "function",
    "as", "match", "with", "use"
  ]

private def identifier (value : String) : String :=
  let value := if value.startsWith "'" then (value.drop 1).toString else value
  if !value.isEmpty && value.all Char.isDigit then value
  else (Lean.Name.mkSimple value).toStringWithToken (isToken := reservedIdentifier)

private def pragmaName (value : String) : String :=
  if value == "opaque" then value else identifier value

private def isMoveAddress (value : String) : Bool :=
  let digits := value.toList.drop 2
  value.startsWith "0x" && !digits.isEmpty && digits.all fun character =>
    character.isDigit || ('a' <= character && character <= 'f') ||
      ('A' <= character && character <= 'F')

private def pathText (segments : Array String) : String :=
  "::".intercalate (segments.mapIdx fun index segment =>
    if index == 0 && isMoveAddress segment then segment else identifier segment).toList

private def abilityText : Ability → String
  | .copy => "Copy"
  | .drop => "Drop"
  | .store => "Store"
  | .key => "Key"

private def abilitiesText (abilities : Array Ability) : String :=
  if abilities.isEmpty then ""
  else " has " ++ ", ".intercalate (abilities.map abilityText).toList

private def closeAngles (value : String) : String :=
  value ++ (if value.endsWith ">" then " >" else ">")

private def lifetimeText : SourceLifetime → String
  | .static => "static"
  | .parameter name | .local name => identifier name
  | .inference => "_"

mutual
private partial def typeText : Ty → String
  | .unit => "Unit"
  | .never => "Never"
  | .bool => "Bool"
  | .char => "Char"
  | .string => "string"
  | .bytes => "Bytes"
  | .address => "Address"
  | .signer => "Signer"
  | .uint width => s!"u{width}"
  | .sint width => s!"i{width}"
  | .uptr => "usize"
  | .iptr => "isize"
  | .nat => "Nat"
  | .int => "Int"
  | .range => "Range"
  | .tuple elements =>
      if elements.isEmpty then "Unit"
      else "(" ++ ", ".intercalate (elements.map (typeText ·.value)).toList ++ ")"
  | .vector element none => "Vector<" ++ closeAngles (typeText element.value)
  | .vector element (some length) =>
      "Vector<" ++ closeAngles s!"{typeText element.value}, const {length}"
  | .function arguments result abilities =>
      "Fn(" ++ ", ".intercalate (arguments.map (typeText ·.value)).toList ++ ") -> " ++
        typeText result.value ++ abilitiesText abilities
  | .reference mutable referent lifetime =>
      let referent := typeText referent.value
      let lifetime := match lifetime with
        | .inference => ""
        | lifetime => s!"[{lifetimeText lifetime}]"
      let separator := if !lifetime.isEmpty then " "
        else if !mutable && referent.startsWith "&" then " " else ""
      "&" ++ lifetime ++ (if mutable then "mut " else "") ++ separator ++ referent
  | .named segments arguments =>
      let name := pathText segments
      if arguments.isEmpty then name
      else name ++ "<" ++ closeAngles (", ".intercalate
        (arguments.map typeArgumentText).toList)

private partial def typeArgumentText : TypeArgument → String
  | .type value => typeText value
  | .constInteger value => s!"const {value}"
  | .constBool value => s!"const {value}"
  | .lifetime value => s!"lifetime {lifetimeText value}"
end

private def storageHeadText (head : Ty) : String :=
  match head with
  | .named segments arguments =>
      if arguments.isEmpty && segments.size == 1 && segments[0]!.contains '.' then
        ".".intercalate (segments[0]!.splitOn "." |>.map identifier)
      else typeText head
  | _ => typeText head

private def binderText (binder : GenericBinder) : String :=
  let annotation := match binder.kind with
    | .type => if binder.phantom then " : phantom type" else ""
    | .const => " : const" ++ match binder.type with
        | some type => " " ++ typeText type.value
        | none => ""
    | .lifetime => " : lifetime"
    | .evidence => " : evidence"
  "{" ++ identifier binder.name ++ annotation ++ abilitiesText binder.abilities ++ "}"

private def bindersText (binders : Array GenericBinder) : String :=
  if binders.isEmpty then ""
  else " " ++ " ".intercalate (binders.map binderText).toList

private def bindersDoc (binders : Array GenericBinder) : Doc :=
  text (bindersText binders)

private def throwText : ThrowKind → String
  | .abort => "abort"
  | .panic => "panic"

private def primitiveText : Primitive → String
  | .tuple => "tuple"
  | .vector => "vector"
  | .repeatVector length => s!"repeatVector[{length}]"
  | .pushVector => "pushVector"
  | .swapVector => "swapVector"
  | .length => "length"
  | .index => "index"
  | .slice => "slice"
  | .range => "range"
  | .add => "add"
  | .checkedAdd .abort => "checkedAddAbort"
  | .checkedAdd .panic => "checkedAddPanic"
  | .subtract => "subtract"
  | .checkedSubtract .abort => "checkedSubtractAbort"
  | .checkedSubtract .panic => "checkedSubtractPanic"
  | .multiply => "multiply"
  | .checkedMultiply .abort => "checkedMultiplyAbort"
  | .checkedMultiply .panic => "checkedMultiplyPanic"
  | .overflowingAdd => "overflowingAdd"
  | .overflowingSubtract => "overflowingSubtract"
  | .overflowingMultiply => "overflowingMultiply"
  | .divide => "divide"
  | .checkedDivide .abort => "checkedDivideAbort"
  | .checkedDivide .panic => "checkedDividePanic"
  | .modulo => "modulo"
  | .checkedModulo .abort => "checkedModuloAbort"
  | .checkedModulo .panic => "checkedModuloPanic"
  | .bitwiseOr => "bitwiseOr"
  | .bitwiseAnd => "bitwiseAnd"
  | .bitwiseXor => "bitwiseXor"
  | .bitwiseNot => "bitwiseNot"
  | .shiftLeft => "shiftLeft"
  | .checkedShiftLeft .abort => "checkedShiftLeftAbort"
  | .checkedShiftLeft .panic => "checkedShiftLeftPanic"
  | .shiftRight => "shiftRight"
  | .checkedShiftRight .abort => "checkedShiftRightAbort"
  | .checkedShiftRight .panic => "checkedShiftRightPanic"
  | .logicalAnd => "logicalAnd"
  | .logicalOr => "logicalOr"
  | .logicalNot => "logicalNot"
  | .equal => "equal"
  | .notEqual => "notEqual"
  | .less => "less"
  | .greater => "greater"
  | .lessEqual => "lessEqual"
  | .greaterEqual => "greaterEqual"
  | .negate => "negate"
  | .checkedNegate .abort => "checkedNegateAbort"
  | .checkedNegate .panic => "checkedNegatePanic"
  | .cast => "cast"
  | .checkedCast failure => s!"checkedCast[{throwText failure}]"
  | .implies => "implies"
  | .equivalent => "equivalent"
  | .identical => "identical"
  | .copyValue => "copyValue"
  | .moveValue => "moveValue"
  | .profileAdd => "add"
  | .profileSubtract => "subtract"
  | .profileMultiply => "multiply"
  | .profileDivide => "divide"
  | .profileModulo => "modulo"
  | .profileShiftLeft => "shiftLeft"
  | .profileShiftRight => "shiftRight"
  | .profileNegate => "negate"
  | .profileCast => "cast"

private def behaviorText : BehaviorOperation → String
  | .requiresOf => "requires_of"
  | .abortsOf => "aborts_of"
  | .ensuresOf => "ensures_of"
  | .resultOf => "result_of"
  | .unchangedOf => "unchanged_of"
  | .foldsOf => "folds_of"
  | .writeOf index => s!"write_of[{index}]"

private def specificationText : SpecificationOperation → String
  | .behavior kind _ => behaviorText kind
  | .old => "old"
  | .saveStateAnchor label => s!"saveStateAnchor[{label}]"
  | .withStateAnchor label => s!"withStateAnchor[{label}]"
  | .foldsCaptureAnchor label => s!"foldsCaptureAnchor[{label}]"
  | .global => "global"
  | .typeDomain => "typeDomain"
  | .result index => s!"result[{index}]"
  | .inlineCallSummary => "inlineCallSummary"
  | .emptyVector => "emptyVector"
  | .singletonVector => "singletonVector"
  | .updateVector => "updateVector"
  | .concatVector => "concatVector"
  | .indexOfVector => "indexOfVector"
  | .containsVector => "containsVector"
  | .lengthVector => "lengthVector"
  | .indexVector => "indexVector"
  | .sliceVector => "sliceVector"
  | .bitVectorToInt => "bitVectorToInt"
  | .intToBitVector => "intToBitVector"
  | .inRange => "inRange"
  | .inVectorRange => "inVectorRange"
  | .vectorRange => "vectorRange"

private partial def placeDoc : Place → Doc
  | .local name _ => text (identifier name)
  | .deref base _ => text "*" ++ placeAtomDoc base
  | .field base name _ => placeAtomDoc base ++ text s!".{identifier name}"
where
  placeAtomDoc (place : Place) : Doc := match place with
    | .local .. => placeDoc place
    | _ => text "(" ++ placeDoc place ++ text ")"

private partial def patternDoc : BindingPattern → Doc
  | .wildcard _ => text "_"
  | .variable name _ mutable =>
      text ((if mutable then "mut " else "") ++ identifier name)
  | .literal (.bool value) _ => text (if value then "true" else "false")
  | .literal (.char value) _ => text s!"{repr (Char.ofNat value)}"
  | .literal (.integer value) _ => text (toString value)
  | .tuple elements _ =>
      if elements.size == 1 then text "(" ++ patternDoc elements[0]! ++ text ",)"
      else delimited "(" ")" (elements.map patternDoc)
  | .constructor owner variant fields _ =>
      let suffix := (variant.map fun value => "::" ++ identifier value).getD ""
      let name := typeText owner.value ++ suffix
      let fields := fields.map fun (field, pattern) =>
        text s!"{identifier field} := " ++ patternDoc pattern
      if fields.isEmpty then text name ++ text " {}"
      else text name ++ text " { " ++ commaSep fields ++ text " }"

private partial def isBlockExpression : Expr → Bool
  | .block #[] (some result) _ => isBlockExpression result
  | .block .. | .specBlock .. | .ifElse .. | .match_ .. | .forRange .. => true
  | _ => false

private partial def rendersAsDo : Expr → Bool
  | .block #[] (some result) _ => rendersAsDo result
  | .block #[.expression value] none _ => rendersAsDo value
  | .block .. => true
  | _ => false

private def isUnitEffect : Expr → Bool
  | .assign .. | .assignExpression .. | .assignPattern .. | .mutateReference .. => true
  | _ => false

private def isAbruptExpression : Expr → Bool
  | .break_ .. | .continue_ .. | .return_ .. | .throw_ .. => true
  | _ => false

private partial def isKnownUnitExpression : Expr → Bool
  | .unit _ | .specBlock .. | .forRange .. | .loop .. | .assign .. | .assignExpression .. |
      .assignPattern .. |
      .mutateReference .. | .dropPlace .. => true
  | .typedCall _ result _ _ | .typedGenericCall _ result _ _ _ =>
      result.value == .unit
  | .methodCall _ (some result) _ _ _ _ => result.value == .unit
  | .block _ none _ => true
  | .block _ (some result) _ => isKnownUnitExpression result
  | .ifElse _ thenBranch (some elseBranch) _ =>
      (isKnownUnitExpression thenBranch || isAbruptExpression thenBranch) &&
        (isKnownUnitExpression elseBranch || isAbruptExpression elseBranch)
  | .ifElse _ thenBranch none _ =>
      isKnownUnitExpression thenBranch || isAbruptExpression thenBranch
  | _ => false

/-- A block used as a statement discards its result. Flatten that administrative
scope while retaining every effect in source order; this is the form lowering
reconstructs after a source round trip. -/
private partial def flattenDiscardedBlock (statements : Array Statement)
    (result : Option Expr) : Array Statement :=
  let statements := statements.foldl (init := #[]) fun flattened statement =>
    match statement with
    | .expression (.block nestedStatements nestedResult _) =>
        flattened ++ flattenDiscardedBlock nestedStatements nestedResult
    | statement => flattened.push statement
  match result with
  | none | some (.unit _) => statements
  | some (.block nestedStatements nestedResult _) =>
      statements ++ flattenDiscardedBlock nestedStatements nestedResult
  | some result => statements.push (.expression result)

private def operatorInfoForExpr? : Expr → Option OperatorInfo
  | .primitive operation arguments _ => do
      let info ← Operators.infoForPrimitive? operation
      let arity := if info.fixity == .prefix then 1 else 2
      if arguments.size == arity then some info else none
  | _ => none

private def expressionPrecedence (expression : Expr) : Nat :=
  if let some info := operatorInfoForExpr? expression then info.precedence
  else match expression with
    | .quantifier .. | .specBlock .. | .block .. | .ifElse .. | .match_ .. |
        .forRange .. | .loop .. | .break_ .. | .continue_ .. | .assign .. |
        .assignExpression .. | .assignPattern .. |
        .return_ .. => 0
    | .field .. | .storageIndex .. | .index .. | .methodCall .. => 14
    | .typedPrimitive .profileCast .. => 12
    | .placeOperation .. | .borrowPlace .. | .borrowValue .. |
        .dereference .. | .dropPlace .. => 13
    | .membership .. | .variantTest .. => 5
    | _ => 1024

private def isLoopSpecification
    (conditions : Array (SpecificationConditionKind × Expr)) : Bool :=
  !conditions.isEmpty &&
    conditions.any (fun condition => condition.1 == .loopInvariant) &&
    conditions.all fun condition => match condition.1 with
      | .let_ _ | .loopInvariant => true
      | _ => false

mutual
  private partial def expressionDoc : Expr → Except String Doc
    | .unit _ => pure <| text "()"
    | .bool value _ => pure <| text (if value then "true" else "false")
    | .char value _ => pure <| text s!"{repr (Char.ofNat value)}"
    | .integer value _ => pure <| text (toString value)
    | .typedInteger value type _ =>
        pure <| text s!"{value}{typeText type.value}"
    | .address value _ => pure <| text s!"@{value}"
    | .string value _ => pure <| text s!"{repr value}"
    | .bytes values _ => pure <| text (byteLiteral values)
    | .local name _ => pure <| text (identifier name)
    | .primitive .tuple arguments _ => do
        let arguments ← arguments.mapM expressionDoc
        if arguments.isEmpty then pure <| text "()"
        else if arguments.size == 1 then
          pure <| text "(" ++ arguments[0]! ++ text ",)"
        else pure <| delimited "(" ")" arguments
    | .primitive .vector arguments _ =>
        (fun arguments => text "#[" ++ commaSep arguments ++ text "]")
          <$> arguments.mapM expressionDoc
    | .primitive (.repeatVector length) arguments _ => do
        let [argument] := arguments.toList
          | throw "a repeated vector literal expects one operand"
        pure <| text "#[" ++ (← expressionDoc argument) ++ text s!"; {length}]"
    | .primitive .slice arguments _ =>
        call (text "slice") <$> arguments.mapM expressionDoc
    | .primitive .overflowingAdd arguments _ =>
        call (text "overflowing_add") <$> arguments.mapM expressionDoc
    | .primitive .overflowingSubtract arguments _ =>
        call (text "overflowing_subtract") <$> arguments.mapM expressionDoc
    | .primitive .overflowingMultiply arguments _ =>
        call (text "overflowing_multiply") <$> arguments.mapM expressionDoc
    | .primitive operation arguments _ => do
        if operation == .length then
          let [argument] := arguments.toList
            | throw "length expects one operand"
          let argument := match argument with
            | .dereference value _ => value
            | _ => argument
          return (← expressionDocAt 14 argument) ++ text ".length"
        if operation == .index then
          let [value, index] := arguments.toList
            | throw "vector indexing expects two operands"
          return (← expressionDocAt 14 value) ++ text "[" ++
            (← expressionDoc index) ++ text "]!"
        if operation == .logicalNot then
          match arguments with
          | #[.primitive .logicalNot #[argument] _] => return (← expressionDoc argument)
          | _ => pure ()
        if let some info := Operators.infoForPrimitive? operation then
          match info.fixity, arguments with
          | .prefix, #[argument] => do
              let argumentDoc ← expressionDocAt (info.precedence + 1) argument
              let argumentDoc := match argument with
                | .integer value _ | .typedInteger value .. =>
                    if value < 0 then text "(" ++ argumentDoc ++ text ")"
                    else argumentDoc
                | _ => argumentDoc
              pure <| text info.symbol ++ argumentDoc
          | .infix, #[left, right] => do
              let (leftPrecedence, rightPrecedence) := match info.associativity with
                | .left => (info.precedence, info.precedence + 1)
                | .right => (info.precedence + 1, info.precedence)
                | .none => (info.precedence + 1, info.precedence + 1)
              let left ← expressionDocAt leftPrecedence left
              let right ← expressionDocAt rightPrecedence right
              pure <| Format.group <| left ++
                Format.nest 2 (soft ++ text s!"{info.symbol} " ++ right)
          | _, _ =>
              call (text s!"core.prim.{primitiveText operation}")
                <$> arguments.mapM expressionDoc
        else
          call (text s!"core.prim.{primitiveText operation}")
            <$> arguments.mapM expressionDoc
    | .typedPrimitive operation result arguments _ => do
        if operation == .vector then
          let .vector element none := result.value
            | throw "a typed vector literal requires an unfixed vector result type"
          let arguments ← arguments.mapM expressionDoc
          return text s!"vector<{closeAngles (typeText element.value)}[" ++
            commaSep arguments ++ text "]"
        if operation == .profileCast then
          let [argument] := arguments.toList
            | throw "a source cast expects one operand"
          return (← expressionDocAt 13 argument) ++ text s!" as {typeText result.value}"
        let head ← match operation with
          | .cast => pure s!"core.prim.cast[{typeText result.value}]"
          | .checkedCast failure =>
              pure s!"core.prim.checkedCast[{throwText failure}, {typeText result.value}]"
          | _ => throw s!"typed primitive `{primitiveText operation}` has no canonical spelling"
        call (text head) <$> arguments.mapM expressionDoc
    | .call name arguments _ => call (text (pathText name)) <$> arguments.mapM expressionDoc
    | .closure name result captures _ => do
        let captures ← captures.mapM expressionDoc
        let suffix := if captures.isEmpty then .nil
          else text ", " ++ commaSep captures
        pure <| text s!"function[{typeText result.value}]({pathText name}" ++
          suffix ++ text ")"
    | .invoke callable arguments _ =>
        call (text "invoke") <$> (#[callable] ++ arguments).mapM expressionDoc
    | .genericCall name types arguments _ => do
        let typeArguments := ", ".intercalate (types.map (typeText ·.value)).toList
        pure <| call (text <| pathText name ++ "::<" ++ closeAngles typeArguments)
          (← arguments.mapM expressionDoc)
    | .typedCall name result arguments _ =>
        (fun invocation => text "(" ++ invocation ++ text s!" : {typeText result.value})")
          <$> (call (text (pathText name)) <$> arguments.mapM expressionDoc)
    | .typedGenericCall name result types arguments _ => do
        let typeArguments := ", ".intercalate (types.map (typeText ·.value)).toList
        let invocation ← call (text s!"{pathText name}::<{closeAngles typeArguments}")
          <$> arguments.mapM expressionDoc
        pure <| text "(" ++ invocation ++ text s!" : {typeText result.value})"
    | .methodCall name result types receiver arguments _ => do
        let typeArguments := ", ".intercalate (types.map (typeText ·.value)).toList
        let suffix := if types.isEmpty then "" else "::<" ++ closeAngles typeArguments
        let head := (← expressionDocAt 14 receiver) ++ text s!".{identifier name}{suffix}"
        let invocation := call head (← arguments.mapM expressionDoc)
        pure <| match result with
        | none => invocation
        | some result => text "(" ++ invocation ++ text s!" : {typeText result.value})"
    | .global operation resource arguments _ => do
        let name := match operation with
          | .contains => "exists"
          | .borrow false => "borrow_global"
          | .borrow true => "borrow_global_mut"
          | .take => "move_from"
          | .publish => "move_to"
        let arguments ← arguments.mapM expressionDoc
        pure <| call (text s!"{name}<{closeAngles (typeText resource.value)}") arguments
    | .construct name arguments _ =>
        call (text s!"core.construct {pathText name}") <$> arguments.mapM expressionDoc
    | .appliedConstruct owner variant arguments _ =>
        let suffix := (variant.map fun value => "::" ++ identifier value).getD ""
        call (text s!"core.construct {typeText owner.value}{suffix}")
          <$> arguments.mapM expressionDoc
    | .namedConstruct owner variant fields _ => do
        let suffix := (variant.map fun value => "::" ++ identifier value).getD ""
        let fields ← fields.mapM fun (name, value) => do
          match value with
          | .local localName _ =>
              if localName == name then pure <| text (identifier name)
              else pure <| text s!"{identifier name} := " ++ (← expressionDoc value)
          | _ => pure <| text s!"{identifier name} := " ++ (← expressionDoc value)
        let constructor := text ("new " ++ typeText owner.value ++ suffix)
        if fields.isEmpty then pure <| constructor ++ text " {}"
        else pure <| hangingBraced constructor fields
    | .select _ field value _ => do
        pure <| (← expressionDocAt 14 value) ++ text s!".{identifier field}"
    | .field value field _ => do
        pure <| (← expressionDocAt 14 value) ++ text s!".{identifier field}"
    | .storageIndex head index _ => do
        pure <| text (storageHeadText head.value) ++ text "[" ++
          (← expressionDoc index) ++ text "]"
    | .index value index _ => do
        pure <| (← expressionDocAt 14 value) ++ text "[" ++
          (← expressionDoc index) ++ text "]"
    | .membership element collection _ => do
        pure <| Format.group <| (← expressionDocAt 6 element) ++ text " ∈" ++
          Format.nest 2 (soft ++ (← expressionDocAt 6 collection))
    | .variantTest value variants _ => do
        let variants := Format.joinSep (variants.map (text ∘ identifier)).toList (text " | ")
        pure <| Format.group <| (← expressionDocAt 6 value) ++ text " is " ++ variants
    | .selectVariants owner fields value _ =>
        let fields := ", ".intercalate (fields.map identifier).toList
        call (text s!"core.data.selectVariants[{typeText owner.value}, {fields}]")
          <$> #[value].mapM expressionDoc
    | .testVariants owner variants value _ =>
        let variants := ", ".intercalate (variants.map identifier).toList
        call (text s!"core.data.testVariants[{typeText owner.value}, {variants}]")
          <$> #[value].mapM expressionDoc
    | .discriminant owner result value _ =>
        call (text s!"discriminant[{typeText owner.value}, {typeText result.value}]")
          <$> #[value].mapM expressionDoc
    | .placeOperation operation place _ =>
        let name := match operation with
          | .move => "move"
          | .copy => "copy"
          | .read => "core.read"
        call (text name) <$> #[place].mapM expressionDoc
    | .borrowPlace mutable place _ =>
        pure <| text (if mutable then "&mut " else "&") ++ placeDoc place
    | .dropPlace place _ =>
        pure <| text "drop(" ++ placeDoc place ++ text ")"
    | .borrowValue mutable value _ => do
        pure <| text (if mutable then "&mut " else "&") ++
          (← expressionDocAt 13 value)
    | .freezeReference explicit value _ =>
        call (text <| if explicit then "core.ref.freezeExplicit" else "core.ref.freeze")
          <$> #[value].mapM expressionDoc
    | .dereference value _ => do
        pure <| text "*" ++ (← expressionDocAt 13 value)
    | .mutateReference reference value _ => do
        pure <| text "*" ++ (← expressionDocAt 13 reference) ++ text " := " ++
          (← expressionDoc value)
    | .quantifier kind binders body _ => do
        let keyword := match kind with | .forall => "∀" | .exists => "∃"
        let binders ← binders.mapM fun (pattern, domain) => do
          -- A binder over a whole type's domain prints as `x : T`.
          match domain with
          | .specification .typeDomain #[element] #[] _ =>
              pure <| patternDoc pattern ++ text " : " ++ text (typeText element.value)
          | _ => pure <| patternDoc pattern ++ text " in " ++ (← expressionDoc domain)
        let binders := Format.group <|
          Format.joinSep binders.toList (text ";" ++ soft)
        pure <| Format.group <| text s!"{keyword} (" ++ binders ++ text ")," ++
          Format.nest 2 (soft ++ (← expressionDoc body))
    | .specification operation types arguments _ => do
        if let .behavior kind range := operation then
          let some target := arguments[0]?
            | throw "a behavior predicate requires a function-value target"
          unless types.isEmpty do throw "behavior predicates have no type arguments"
          let target ← expressionDocAt 14 target
          let values ← (arguments.drop 1).mapM expressionDoc
          let invocation := call (text s!"{behaviorText kind}<" ++ target ++ text ">") values
          let statePrefix := match range.pre, range.post with
            | none, none => .nil
            | some pre, none => if kind == .requiresOf || kind == .abortsOf then
                text s!"@{pre} |~ "
              else text s!"@{pre}.. |~ "
            | none, some post => text s!"..@{post} |~ "
            | some pre, some post => text s!"@{pre}..@{post} |~ "
          return statePrefix ++ invocation
        if let .saveStateAnchor label := operation then
          unless types.isEmpty && arguments.isEmpty do
            throw "save_state_anchor! expects only its label"
          return text s!"save_state_anchor!({label})"
        if let .foldsCaptureAnchor label := operation then
          unless types.isEmpty && arguments.isEmpty do
            throw "folds_capture_anchor! expects only its label"
          return text s!"folds_capture_anchor!({label})"
        if let .withStateAnchor label := operation then
          let [argument] := arguments.toList
            | throw "with_state_anchor! expects one value operand"
          unless types.isEmpty do throw "with_state_anchor! has no type arguments"
          return Format.group <| text s!"with_state_anchor!({label}," ++
            Format.nest 2 (soft ++ (← expressionDoc argument)) ++ text ")"
        if let .result index := operation then
          unless types.isEmpty && arguments.isEmpty do
            throw "a specification result takes no arguments"
          return text (if index == 0 then "result" else s!"spec.result[{index}]")
        if operation == .old then
          let [argument] := arguments.toList
            | throw "old expects one operand"
          return call (text "old") #[← expressionDoc argument]
        if operation == .inRange then
          unless types.isEmpty do throw "in_range has no type arguments"
          return call (text "in_range") (← arguments.mapM expressionDoc)
        if operation == .bitVectorToInt then
          unless types.isEmpty do throw "bit_vector_to_int has no type arguments"
          let [argument] := arguments.toList
            | throw "bit_vector_to_int expects one operand"
          -- The specification surface widens bounded integers implicitly.
          -- The typed operation remains in LIR and is recovered by lowering.
          return ← expressionDoc argument
        if operation == .intToBitVector then
          match types.toList with
          | [result] =>
              return call (text s!"int_to_bit_vector[{typeText result.value}]")
                (← arguments.mapM expressionDoc)
          | [] =>
              -- The exchange defers the bit-vector width in the projected
              -- specification domain; the width-less spelling round-trips it.
              return call (text "int_to_bit_vector") (← arguments.mapM expressionDoc)
          | _ => throw "int_to_bit_vector expects one result type"
        if operation == .global then
          let [resource] := types.toList
            | throw "global expects one resource type"
          let [key] := arguments.toList
            | throw "global expects one key"
          return call (text s!"global<{closeAngles (typeText resource.value)}")
            #[← expressionDoc key]
        if operation == .lengthVector then
          let [argument] := arguments.toList
            | throw "spec.lengthVector expects one operand"
          return (← expressionDocAt 14 argument) ++ text ".length"
        if operation == .indexVector then
          let [value, index] := arguments.toList
            | throw "spec.indexVector expects two operands"
          return (← expressionDocAt 14 value) ++ text "[" ++
            (← expressionDoc index) ++ text "]"
        if operation == .sliceVector then
          let [value, range] := arguments.toList
            | throw "spec.sliceVector expects two operands"
          return (← expressionDocAt 14 value) ++ text "[" ++
            (← expressionDoc range) ++ text "]"
        if operation == .containsVector then
          let [collection, element] := arguments.toList
            | throw "spec.containsVector expects two operands"
          return Format.group <| (← expressionDocAt 6 element) ++ text " ∈" ++
            Format.nest 2 (soft ++ (← expressionDocAt 6 collection))
        if operation == .emptyVector then
          let [element] := types.toList
            | throw "spec.emptyVector expects one element type"
          unless arguments.isEmpty do throw "spec.emptyVector expects no operands"
          return text s!"vec::<{typeText element.value}>()"
        let builtin? := match operation with
          | .singletonVector => some "vec"
          | .updateVector => some "update"
          | .concatVector => some "concat"
          | .indexOfVector => some "index_of"
          | .inVectorRange => some "in_range"
          | .vectorRange => some "range"
          | _ => none
        if let some builtin := builtin? then
          return call (text builtin) (← arguments.mapM expressionDoc)
        let typeArguments := ", ".intercalate (types.map (typeText ·.value)).toList
        let types := if types.isEmpty then "" else match operation with
          | .intToBitVector => "[" ++ typeArguments ++ "]"
          | _ => "::<" ++ closeAngles typeArguments
        call (text s!"spec.{specificationText operation}{types}") <$> arguments.mapM expressionDoc
    | .specBlock conditions _ => do
        let members ← specificationMemberDocs conditions
        if members.size == 1 then pure <| text "spec " ++ members[0]!
        else pure <| block (text "spec do") members
    | .block statements result _ => do
        match statements, result with
        | #[], some result => expressionDoc result
        | #[.expression value], none =>
            if isKnownUnitExpression value then expressionDoc value
            else pure <| block (text "do") #[← expressionDoc value]
        | _, _ =>
            let entries ← blockEntriesDoc statements result
            if entries.isEmpty then pure <| text "()"
            else pure <| block (text "do") entries
    | .ifElse condition (.unit _) (some (.throw_ .abort #[code] _)) _ => do
        pure <| call (text "assert!") #[← expressionDoc condition, ← expressionDoc code]
    | .ifElse condition (.unit _) (some elseBranch) span =>
        expressionDoc (.ifElse (negate condition) elseBranch none span)
    | .ifElse condition thenBranch (some (.unit _)) span =>
        expressionDoc (.ifElse condition thenBranch none span)
    | .ifElse condition thenBranch elseBranch _ => do
        -- A block-valued condition renders across lines; parentheses are what
        -- keep the following `then` attached to this `if`.
        let conditionDoc ← if rendersAsDo condition then
            pure (text "(" ++ (← expressionDoc condition) ++ text ")")
          else expressionDoc condition
        let (thenDoc, structurallyIndented) ← ifBranchDoc thenBranch
        -- An `else`-less branch is a statement, and a `return` closing one
        -- keeps its semicolon: the branch may wrap onto its own line, where
        -- an unterminated `return` would read as the branch's value instead.
        let thenDoc := if elseBranch.isNone && thenBranch matches .return_ .. then
            thenDoc ++ text ";" else thenDoc
        let thenIndented := structurallyIndented ||
          (do
            let conditionWidth ← flattenedWidth? conditionDoc
            let branchWidth ← flattenedWidth? thenDoc
            pure (decide (3 + conditionWidth + 6 + branchWidth > 72))).getD true
        let thenBody := if thenIndented then Format.nest 2 (hard ++ thenDoc)
          else text " " ++ thenDoc
        if elseBranch.isNone then
          return Format.group <| text "if " ++ conditionDoc ++ text " then" ++
            thenBody
        let elseBranch := elseBranch.get!
        let elseSeparator := if thenIndented || isBlockExpression thenBranch then
            hard ++ text "else"
          else soft ++ text "else"
        let (elseDoc, structurallyIndented) ← ifBranchDoc elseBranch
        let elseIndented := structurallyIndented ||
          (flattenedWidth? elseDoc).any fun width => decide (5 + width > 72)
        let elseBody := if elseIndented then Format.nest 2 (hard ++ elseDoc)
          else text " " ++ elseDoc
        pure <| Format.group <| text "if " ++ conditionDoc ++ text " then" ++
          thenBody ++ elseSeparator ++ elseBody
    | .match_ scrutinee arms _ => do
        let arms ← arms.mapM fun (pattern, guard, body) => do
          let guard ← guard.mapM expressionDoc
          let body ← expressionDoc body
          pure <| text "| " ++ patternDoc pattern ++
            (guard.map fun value => text " if " ++ value).getD .nil ++
            text " => " ++ body
        pure <| block (text "match " ++ (← expressionDoc scrutinee) ++ text " with") arms
    | .forRange iterator lower upper body _ => do
        let rangeDoc := Format.group <|
          (← expressionDocAt 7 lower) ++ text ".." ++ (← expressionDocAt 7 upper)
        let head := Format.group <|
          text s!"for {identifier iterator} in " ++ rangeDoc ++ text " do"
        match body with
        | .block statements result _ => do
            -- A range-loop body has no value position: every entry is
            -- discarded before the implicit increment. Do not synthesize
            -- the `return ()` used to distinguish a discarded final value in
            -- an ordinary `do` block.
            let statements := flattenDiscardedBlock statements result
            let entries ← statementDocs statements
            if entries.isEmpty then pure <| head ++ Format.nest 2 (hard ++ text "()")
            else pure <| head ++ Format.nest 2 (hard ++ vcat entries)
        | _ =>
            pure <| head ++ Format.nest 2 (hard ++ (← expressionDoc body))
    | .loop (.ifElse condition body (some (.break_ none _)) _) _ => do
        let attached := do
          let .block statements (some actualCondition) _ := condition | none
          guard (!statements.isEmpty)
          let blocks ← statements.mapM fun (statement : Statement) => match statement with
            | .expression (.specBlock conditions _) => some conditions
            | _ => none
          let conditions := blocks.flatten
          guard (isLoopSpecification conditions)
          some (actualCondition, conditions)
        let (condition, specification) := match attached with
          | some value => (value.1, some value.2)
          | none => (condition, none)
        let conditionDoc ← expressionDoc condition
        let head := if isBlockExpression condition then
            text "while " ++ conditionDoc ++ hard ++ text "do"
          else
            Format.group <| text "while " ++ conditionDoc ++ text " do"
        let loopDoc ← match body with
        | .block statements result _ => do
            -- The loop context already requires Unit, so its final source
            -- expression is not an ordinary discarded block result.  In
            -- particular, a Unit-returning call such as `values.push_back(0)`
            -- must remain the last iteration action rather than acquiring an
            -- explicit `return ()`, which would return from the function on
            -- the first iteration.  This is the same contextual treatment as
            -- range-loop bodies above. Administrative blocks in statement
            -- position have the same discarded result, so flatten those too;
            -- otherwise a source sequence acquires a needless nested `do`.
            let statements := flattenDiscardedBlock statements result
            let entries ← statementDocs statements
            if entries.isEmpty then pure <| head ++ text " ()"
            else pure <| head ++ Format.nest 2 (hard ++ vcat entries)
        | _ =>
            pure <| head ++ Format.nest 2 (soft ++ (← expressionDoc body))
        match specification with
        | none => pure loopDoc
        | some conditions =>
            pure <| loopDoc ++ hard ++ block (text "where")
              (← specificationMemberDocs conditions)
    | .loop body _ => do
        pure <| text "loop " ++ (← expressionDoc body)
    | .break_ value _ => do
        let value ← value.mapM expressionDoc
        pure <| text "break" ++ (value.map fun doc => text " " ++ doc).getD .nil
    | .continue_ _ => pure <| text "continue"
    | .assign place value _ => do
        pure <| placeDoc place ++ text " := " ++ (← expressionDoc value)
    | .assignExpression target value _ => do
        pure <| (← expressionDoc target) ++ text " := " ++ (← expressionDoc value)
    | .assignPattern pattern type value _ => do
        let value ← expressionDoc value
        pure <| call (text s!"assign_pattern[{typeText type.value}]")
          #[patternDoc pattern, value]
    | .return_ value _ => do
        let valueDoc ← if isBlockExpression value then expressionDoc value
          else expressionDocAt 1 value
        -- A returned value that renders across lines is delimited, so a
        -- statement separator after it belongs to this `return`.
        let multiline := (flattenedWidth? valueDoc).isNone || isBlockExpression value
        let valueDoc := if multiline && !rendersAsDo value then
            text "(" ++ valueDoc ++ text ")" else valueDoc
        pure <| text "return " ++ valueDoc
    | .throw_ kind arguments _ =>
        call (text (throwText kind)) <$> arguments.mapM expressionDoc

  private partial def expressionDocAt (minimumPrecedence : Nat)
      (expression : Expr) : Except String Doc := do
    let document ← expressionDoc expression
    if expressionPrecedence expression < minimumPrecedence then
      pure <| text "(" ++ document ++ text ")"
    else
      pure document

  /-- An `if` branch is already delimited by indentation and the outer `else`.
  Render a semantic block as its entries instead of introducing a redundant
  `do`; non-block branches retain their ordinary expression layout. -/
  private partial def ifBranchDoc : Expr → Except String (Doc × Bool)
    | .block #[] (some (.return_ result _)) _ => ifBranchDoc result
    | .block #[] (some result) _ => ifBranchDoc result
    -- A branch that only leaves the function stays a statement block: its
    -- semicolon is what keeps the `return` from supplying the branch's value.
    | .block #[.expression value@(.return_ ..)] none _ => do
        pure ((← expressionDoc value) ++ text ";", true)
    | .block #[.expression value] none _ => ifBranchDoc value
    | .block statements result _ => do
        let entries ← blockEntriesDoc statements result
        if entries.isEmpty then pure (text "()", false)
        else pure (vcat entries, true)
    | expression => do
        let document ← expressionDoc expression
        -- A nested control-flow expression must start on the branch's
        -- indented line. Keeping `then if ...` on one line leaves the two
        -- `else` clauses at the same offside column and is not parseable.
        pure (document, isBlockExpression expression)

  private partial def specificationMemberDocs
      (conditions : Array (SpecificationConditionKind × Expr)) : Except String (Array Doc) :=
    conditions.mapM fun (kind, expression) => do
      let memberPrefix := match kind with
        | .let_ name => s!"let {identifier name} := "
        | .assertion => "assert "
        | .assumption => "assume "
        | .loopInvariant => "invariant "
      pure <| text memberPrefix ++ (← expressionDoc expression)

  private partial def negate : Expr → Expr
    | .primitive .logicalNot #[argument] _ => argument
    | .typedPrimitive .logicalNot _ #[argument] _ => argument
    | argument => .primitive .logicalNot #[argument] argument.span

  private partial def statementDoc : Statement → Except String Doc
    -- A `return` written as a statement leaves the function; its semicolon is
    -- what separates it from a trailing `return` supplying a block's value.
    | .expression value@(.return_ ..) => do pure ((← expressionDoc value) ++ text ";")
    | .expression value => expressionDoc value
    | .letDecl mutable pattern _ value _ => do
        let value ← expressionDoc value
        pure <| Format.group <| text "let " ++ (if mutable then text "mut " else .nil) ++
          patternDoc pattern ++ text " :=" ++
          Format.nest 2 (soft ++ value)

  private partial def statementDocs (statements : Array Statement) : Except String (Array Doc) := do
    let rec go (remaining : List Statement) (rendered : Array Doc) := do
      match remaining with
      | .expression (.specBlock conditions _) ::
          .expression loopExpr@(.loop ..) :: rest
      | .expression loopExpr@(.loop ..) ::
          .expression (.specBlock conditions _) :: rest =>
          if isLoopSpecification conditions then
            go rest (rendered.push ((← expressionDoc loopExpr) ++ hard ++
              block (text "where") (← specificationMemberDocs conditions)))
          else
            let first ← statementDoc remaining.head!
            go remaining.tail (rendered.push first)
      | statement :: rest => go rest (rendered.push (← statementDoc statement))
      | [] => pure rendered
    go statements.toList #[]

  /-- Flatten a tail block into its enclosing block and omit the inferred unit
  result. Both transformations preserve sequencing while removing CFG- and
  elaboration-shaped `return (do ...)` / trailing `return ()` noise. -/
  private partial def blockEntriesDoc (statements : Array Statement)
      (result : Option Expr) : Except String (Array Doc) := do
    let statements := statements.foldl (init := (#[] : Array Statement))
      fun flattened statement =>
      match statement with
      | .expression (.block nestedStatements nestedResult _) =>
          flattened ++ flattenDiscardedBlock nestedStatements nestedResult
      | statement => flattened.push statement
    if let some (.expression (.return_ (.block nestedStatements nestedResult _) _)) :=
        statements.back? then
      let leading ← statements.pop.mapM statementDoc
      return leading ++ (← blockEntriesDoc nestedStatements nestedResult)
    let trailing ← match result with
      | none | some (.unit _) => pure #[]
      | some (.block nestedStatements nestedResult _)
      | some (.return_ (.block nestedStatements nestedResult _) _) =>
          blockEntriesDoc nestedStatements nestedResult
      | some result =>
          if isKnownUnitExpression result || isAbruptExpression result then
            pure #[← expressionDoc result]
          else
            pure #[text "return " ++ (← expressionDoc result)]
    let entries ← statementDocs statements
    pure (entries ++ trailing)
end

private def parameterDoc (parameter : Parameter) : Doc :=
  text <| (if parameter.mutable then "mut " else "") ++ identifier parameter.name ++
    " : " ++ typeText parameter.type.value

private def modifiersText (modifiers : FunctionModifiers) : String :=
  let values := match modifiers.visibility with
    | .private_ => #[]
    | .public_ => #["public"]
    | .package => #["package"]
    | .friend => #["friend"]
  let values := if modifiers.isDeprecated then values.push "deprecated" else values
  let values := if modifiers.isView then values.push "view" else values
  let values := if modifiers.isEntry then values.push "entry" else values
  let values := if modifiers.isNative then values.push "native" else values
  let values := if modifiers.isOpaque then values.push "opaque" else values
  if values.isEmpty then "" else " ".intercalate values.toList ++ " "

private def clauseDoc : ContractClause → Except String Doc
  | .letPre name expression properties _ =>
      binding "let_pre" name expression properties
  | .letPost name expression properties _ =>
      binding "let_post" name expression properties
  | .requires expression properties _ => condition "requires" expression properties
  | .ensures expression properties _ => condition "ensures" expression properties
  | .abortsIf expression code properties _ => do
      let base ← condition "aborts_if" expression properties
      let code ← code.mapM expressionDoc
      pure <| base ++ (code.map fun value => text " with " ++ value).getD .nil
  | .invariant expression properties _ => condition "invariant" expression properties
  | .modifies expression _ => do
      pure <| text "modifies " ++ (← expressionDoc expression)
  | .modifiesAll _ => pure <| text "modifies *"
  | .reads type _ => pure <| text s!"reads {typeText type.value}"
  | .readsAll _ => pure <| text "reads *"
where
  propertyText (properties : Array String) := if properties.isEmpty then "" else
    "[" ++ ", ".intercalate (properties.map identifier).toList ++ "] "
  condition (keyword : String) (expression : Expr) (properties : Array String) := do
    let properties := if properties.isEmpty then "" else
      "[" ++ ", ".intercalate (properties.map identifier).toList ++ "] "
    let expression ← expressionDoc expression
    pure <| Format.group <| text s!"{keyword} {properties}" ++
      Format.nest 2 expression
  binding (keyword name : String) (expression : Expr) (properties : Array String) := do
    pure <| Format.group <| text s!"{keyword} {propertyText properties}{identifier name} := " ++
      Format.nest 2 (← expressionDoc expression)

private def pragmaDoc (pragma : Pragma) : Except String Doc := do
  let value ← expressionDoc pragma.value
  pure <| match pragma.value with
  | .bool true _ => text s!"pragma {pragmaName pragma.name}"
  | _ => text s!"pragma {pragmaName pragma.name} = " ++ value

private def contractDoc? (name : String) (clauses : Array ContractClause)
    (pragmas : Array Pragma) : Except String (Option Doc) := do
  if clauses.isEmpty && pragmas.isEmpty then return none
  let entries ← pragmas.mapM pragmaDoc
  let entries := entries ++ (← clauses.mapM clauseDoc)
  pure <| some <| block (text s!"spec {identifier name} where") entries

private def fieldDoc (field : FieldDecl) : Doc :=
  text s!"{identifier field.name} : {typeText field.type.value}"

/-- A function body has an implicit result position.  The semantic printer
can nevertheless receive a single explicit `return` after CFG
structurization.  Spell that value directly at the function boundary so the
first rendering is already the same canonical source that a re-import would
produce. Returns in statement position remain explicit. -/
private partial def normalizeFunctionBody (unitResult : Bool) : Expr → Expr
  | .block #[.expression (.return_ value _)] none _ =>
      normalizeFunctionBody unitResult value
  | .block #[] (some (.return_ value _)) _ =>
      normalizeFunctionBody unitResult value
  | .block #[] (some value) span =>
      if unitResult && !isKnownUnitExpression value then
        .block #[.expression value] none span
      else normalizeFunctionBody unitResult value
  | body@(.block #[.expression value] none _) =>
      -- Preserve a one-entry effect block around calls: the original LIR may
      -- not carry a result annotation while re-elaboration does, and using
      -- that incidental difference would break the canonical fixed point.
      -- Structured control flow is unambiguously Unit-valued and can still
      -- shed the administrative wrapper.
      if isUnitEffect value ||
          value matches .loop .. | .forRange .. | .ifElse .. | .match_ .. then
        normalizeFunctionBody unitResult value
      else body
  | body@(.block #[.expression value,
      .expression (.return_ (.unit _) _)] none _) =>
      if isKnownUnitExpression value then normalizeFunctionBody unitResult value else body
  | body@(.block #[.expression value,
      .expression (.return_ (.unit _) _)] (some (.unit _)) _) =>
      if isKnownUnitExpression value then normalizeFunctionBody unitResult value else body
  | body@(.block #[.expression value]
      (some (.return_ (.unit _) _)) _) =>
      if isKnownUnitExpression value then normalizeFunctionBody unitResult value else body
  | body@(.block #[.expression value] (some (.unit _)) _) =>
      if isKnownUnitExpression value then normalizeFunctionBody unitResult value else body
  | body => body

/-- A function body and each final control-flow arm are already result
positions. CFG structurization may nevertheless wrap those values in
`return`; remove only those redundant wrappers while retaining early returns
among block statements. -/
private partial def normalizeResultPosition : Expr → Expr
  | .return_ value _ => normalizeResultPosition value
  | .block statements result span =>
      .block statements (result.map normalizeResultPosition) span
  | .ifElse condition thenBranch elseBranch span =>
      .ifElse condition (normalizeResultPosition thenBranch)
        (elseBranch.map normalizeResultPosition) span
  | .match_ scrutinee arms span =>
      .match_ scrutinee (arms.map fun (pattern, guard, body) =>
        (pattern, guard, normalizeResultPosition body)) span
  | body => body

/-- Interpret the tail of a Unit-returning function in effect position. The
semantic printer and the offside parser cannot carry an expected type between
them, so a final ordinary expression may temporarily appear as a block result.
Move it back into the sequence, recursively through final control-flow arms;
an explicit non-Unit `return` remains untouched and therefore type-checked. -/
private partial def normalizeUnitTail : Expr → Expr
  | .return_ value span =>
      if isKnownUnitExpression value then normalizeUnitTail value
      else .return_ value span
  | .block statements result span =>
      match result with
      | some (.unit _) => .block statements none span
      | some result@(.return_ ..) =>
          let normalized := normalizeUnitTail result
          if normalized matches .unit _ then .block statements none span
          else if isKnownUnitExpression normalized then
            .block (statements.push (.expression normalized)) none span
          else .block statements (some result) span
      | some result =>
          .block (statements.push (.expression (normalizeUnitTail result))) none span
      | none =>
          match statements.back? with
          | some (.expression value) =>
              let value := normalizeUnitTail value
              if value matches .unit _ then .block statements.pop none span
              else .block (statements.pop.push (.expression value)) none span
          | _ => .block statements none span
  | .ifElse condition thenBranch elseBranch span =>
      .ifElse condition (normalizeUnitTail thenBranch)
        (elseBranch.map normalizeUnitTail) span
  | .match_ scrutinee arms span =>
      .match_ scrutinee (arms.map fun (pattern, guard, body) =>
        (pattern, guard, normalizeUnitTail body)) span
  | body => body

/-- A declaration's source attributes print on their own line above it. -/
private def attributesDoc (attributes : Array SourceAttribute) : Doc :=
  if attributes.isEmpty then .nil else
    let entries := attributes.map fun sourceAttribute =>
      let arguments := if sourceAttribute.arguments.isEmpty then "" else
        " (" ++ ", ".intercalate (sourceAttribute.arguments.map identifier).toList ++ ")"
      identifier sourceAttribute.name ++ arguments
    text ("@[" ++ ", ".intercalate entries.toList ++ "]") ++ hard

private def itemDocs : Item → Except String (Array Doc)
  | .constant declaration => do
      pure #[Format.group <| text s!"const {identifier declaration.name} : \
        {typeText declaration.type.value} :=" ++
        Format.nest 2 (soft ++ (← expressionDoc declaration.value))]
  | .struct declaration => do
      let head := text s!"struct {identifier declaration.name}" ++
        bindersDoc declaration.generics ++ text (abilitiesText declaration.abilities) ++ text " where"
      let declarationDoc := attributesDoc declaration.attributes ++
        block head (declaration.fields.map fieldDoc)
      pure <| match ← contractDoc? declaration.name declaration.contract declaration.pragmas with
        | none => #[declarationDoc]
        | some contract => #[declarationDoc, contract]
  | .enum declaration => do
      let head := text s!"enum {identifier declaration.name}" ++
        bindersDoc declaration.generics ++ text (abilitiesText declaration.abilities) ++ text " where"
      let variants := declaration.variants.map fun variant =>
        let fields := if variant.fields.isEmpty then .nil
          else text " " ++ delimited "(" ")" (variant.fields.map fieldDoc)
        let discriminant := variant.discriminant.map (text s!" = {·}") |>.getD .nil
        text s!"| {identifier variant.name}" ++ fields ++ discriminant
      let declarationDoc := attributesDoc declaration.attributes ++ block head variants
      pure <| match ← contractDoc? declaration.name declaration.contract declaration.pragmas with
        | none => #[declarationDoc]
        | some contract => #[declarationDoc, contract]
  | .function declaration => do
      let signatureHead := s!"{modifiersText declaration.modifiers}fun {identifier declaration.name}\
        {bindersText declaration.generics}"
      let signature := hangingDelimited (text signatureHead) "(" ")"
        (declaration.parameters.map parameterDoc)
        (text s!" -> {typeText declaration.result.value}")
      let declarationDoc ← match declaration.body with
        | none => pure signature
        | some body =>
            let unitResult := declaration.result.value == .unit
            let body := normalizeResultPosition body
            let body := if declaration.result.value == .unit then
                normalizeUnitTail body
              else body
            let body := normalizeFunctionBody unitResult body
            let body := if unitResult && !isKnownUnitExpression body &&
                !isBlockExpression body && !isAbruptExpression body then
              .block #[.expression body] none body.span
            else body
            let bodyDoc ← expressionDoc body
            if body matches .match_ .. | .ifElse .. then
              pure <| signature ++ text " :=" ++ Format.nest 2 (hard ++ bodyDoc)
            else if isBlockExpression body then pure <| signature ++ text " := " ++ bodyDoc
            else
              let signatureWidth := (flattenedWidth? signature).getD 79
              let bodyWidth := (flattenedWidth? bodyDoc).getD 79
              if signatureWidth + 4 + bodyWidth <= 78 then
                pure <| Format.group (signature ++ text " := " ++ bodyDoc)
              else if signatureWidth > 78 && 13 + bodyWidth <= 78 then
                pure <| signature ++ text " := " ++ bodyDoc
              else
                pure <| signature ++ text " :=" ++ Format.nest 2 (hard ++ bodyDoc)
      let declarationDoc := attributesDoc declaration.attributes ++ declarationDoc
      pure <| match ← contractDoc? declaration.name declaration.contract declaration.pragmas with
        | none => #[declarationDoc]
        | some contract => #[declarationDoc, contract]
  | .specFunction declaration => do
      let declarationPrefix := if declaration.isOpaque then "opaque spec fun" else "spec fun"
      let signatureHead := s!"{declarationPrefix} {identifier declaration.name}\
        {bindersText declaration.generics}"
      let signature := hangingDelimited (text signatureHead) "(" ")"
        (declaration.parameters.map parameterDoc)
        (text s!" : {typeText declaration.result.value}")
      let declarationDoc ← match declaration.body with
        | none => pure signature
        | some body =>
            let body := normalizeFunctionBody (declaration.result.value == .unit)
              (normalizeResultPosition body)
            let bodyDoc ← expressionDoc body
            if body matches .match_ .. | .ifElse .. then
              pure <| signature ++ text " :=" ++ Format.nest 2 (hard ++ bodyDoc)
            else if isBlockExpression body then pure <| signature ++ text " := " ++ bodyDoc
            else pure <| Format.group (signature ++ text " :=" ++
              Format.nest 2 (soft ++ bodyDoc))
      pure #[attributesDoc declaration.attributes ++ declarationDoc]

private def itemSpan : Item → Span
  | .constant declaration => declaration.span
  | .struct declaration => declaration.span
  | .enum declaration => declaration.span
  | .function declaration => declaration.span
  | .specFunction declaration => declaration.span

private def normalizedDocumentationLines (documentation : String) : List String :=
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

private def commentDoc (comment : Comment) : Doc :=
  if comment.isDoc then
    let text := comment.text
    let body :=
      if text.startsWith "--/" then text.drop 3
      else if text.startsWith "/--" || text.startsWith "/-!" then
        (text.drop 3).dropEnd 2
      else text
    vcat <| (#["/--"] ++ (normalizedDocumentationLines (body.toString)).toArray ++ #["-/"])
      |>.map Std.Format.text
  else
    let text := if comment.text.startsWith "--" || comment.text.startsWith "/-" then
        comment.text
      else s!"-- {comment.text}"
    vcat <| text.splitOn "\n" |>.toArray.map Std.Format.text

private def commentsDoc (comments : Array Comment) : Doc :=
  vcat (comments.map commentDoc)

private def documentationDoc (documentation : String) : Doc :=
  let lines := documentation.trimAscii.toString.splitOn "\n"
  match lines with
  | [line] => text s!"/-! {line.trimAscii.toString} -/"
  | lines => vcat <| #[text "/-!"] ++
      lines.toArray.map (fun line => text line.trimAsciiEnd.toString) ++ #[text "-/"]

/-- Pretty-print a parsed LeanerLang compilation unit. `leadingComments` are
top-level namespace comments discarded as trivia by Lean's parser; the source
backend uses these for explicit unsupported-declaration notices. -/
def format (unit : CompilationUnit) (width : Nat := 80)
    (leadingComments : Array String := #[]) : Except String String := do
  let namespaces ← unit.namespaces.mapM fun ns => do
    let uses := ns.uses.map fun path => text s!"use {pathText path}"
    let friends := ns.friends.map fun declaration =>
      text s!"friend {pathText declaration.path};"
    let pragmas ← ns.pragmas.mapM pragmaDoc
    let comments := ns.comments.qsort fun left right =>
      left.span.startByte < right.span.startByte
    let mut remainingComments := comments
    let mut items := #[]
    for item in ns.items do
      let (before, remaining) := remainingComments.partition fun comment =>
        comment.span.startByte <= (itemSpan item).startByte
      let itemDocuments ← itemDocs item
      if before.isEmpty || itemDocuments.isEmpty then
        items := items ++ itemDocuments
      else
        items := items.push (commentsDoc before ++ hard ++ itemDocuments[0]!) ++
          itemDocuments.drop 1
      remainingComments := remaining
    if !remainingComments.isEmpty then
      items := items.push (commentsDoc remainingComments)
    let leadingCommentDocs := if leadingComments.isEmpty then #[] else
      #[vcat (leadingComments.map fun comment => text s!"-- {comment}")]
    -- Imports form one declaration group. Keeping them adjacent makes a
    -- generated module read like an ordinary Move/Rust import prelude while
    -- `blankSep` still separates the prelude from pragmas and declarations.
    let useEntries := if uses.isEmpty then #[] else #[vcat uses]
    let friendEntries := if friends.isEmpty then #[] else #[vcat friends]
    let entries := useEntries ++ friendEntries ++ pragmas ++ leadingCommentDocs ++ items
    let (kind, path) ← match ns.profile with
      | .move => do
          let path := if ns.path.size == 3 then #[ns.path[0]!, ns.path[2]!]
            else ns.path
          unless path.size == 2 && path[0]?.any isMoveAddress do
            throw "a Move module path must be exactly `0xADDRESS::module_name`"
          pure ("module", path)
      | .rust => pure ("namespace", ns.path)
    let head := text s!"leaner {kind} {pathText path} where"
    let declaration := head ++ Format.nest 2 (hard ++ blankSep entries)
    if ns.doc.trimAscii.isEmpty then pure declaration
    else pure <| documentationDoc ns.doc ++ hard ++ declaration
  let document := text "-- Copyright © Aptos Foundation" ++ hard ++
    text "-- SPDX-License-Identifier: Apache-2.0" ++ hard ++ hard ++
    text "import LeanerLang" ++ hard ++ hard ++ blankSep namespaces ++ hard
  pure (renderDoc document width)

def namespaceStart (source : String) : Except String (String × Array Comment × String) := do
  let lines := source.splitOn "\n"
  let some index := lines.findIdx? fun line =>
      line.startsWith "leaner namespace " || line.startsWith "leaner module "
    | throw "generated source contains no `leaner namespace` or `leaner module` command"
  let prefixLines := lines.take index
  let prelude := "\n".intercalate prefixLines
  let commandStart := if prelude.isEmpty then 0 else prelude.utf8ByteSize + 1
  let commandLines := (lines.drop index).toArray
  let command := "\n".intercalate commandLines.toList
  pure (command, commentsOfSource command, documentationBefore source commandStart)

/-- Parse the semantic printer's source with the registered LeanerLang grammar,
convert it through the frontend AST boundary, and lay it out at `width`. -/
def formatSource (environment : Environment) (source : String) (width : Nat := 80)
    (sourceName : String := "<generated>")
    (additionalLeadingComments : Array String := #[]) : Except String String := do
  let (command, comments, namespaceDoc) ← namespaceStart source
  let parsedSyntax ← Lean.Parser.runParserCategory environment `command command sourceName
  let unit ← compilationUnitOfSyntax parsedSyntax sourceName comments namespaceDoc
    |>.mapError (·.2)
  format unit width additionalLeadingComments

end LeanerLang.Print.Layout
