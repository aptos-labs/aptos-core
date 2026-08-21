-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Compiler.Elab
import Move.Verify.Syntax
import MoveModel.Frontend.XIR.Json

/-!
# Move module export

`#export_leaner_xir module to "path"` evaluates an already elaborated
`MModule` and writes deterministic schema-versioned JSON.  The command is an
explicit author action; ordinary declaration elaboration performs no writes.

`#export_leaner "Module"` is the source-facing compiler directive. It combines
`move_module%` with the compiler-owned XIR handoff, so deployable sources do
not need to name an intermediate `MModule` value. The directive records the
request and performs compilation at end of input, after all declarations have
been elaborated.
-/

namespace Move.Compiler

open Lean Elab Command Term
open Lean.Parser.Term
open MoveModel.Frontend.XIR
open scoped Move.Compiler

syntax (name := exportLeanerXIR)
  "#export_leaner_xir " term " to " str : command

/-- Low-level compatibility form which marks an existing `MModule` value as
the deployable module in a `.lean` compiler input. -/
syntax (name := emitLeanerXIR)
  "#emit_leaner_xir " term : command

/-- Compiles the attributed declarations in the current namespace and marks
the resulting module as this `.lean` compiler input's deployable module. The
optional selection has the same meaning as the corresponding `move_module%`
form. -/
syntax (name := exportLeaner)
  "#export_leaner " str
    (" structs " "[" ident,* "]" " functions " "[" ident,* "]")? : command

/-- Internal command emitted by `move_module` to persist the relationship
between its Lean namespace and on-chain Move identity. -/
syntax (name := registerMoveModuleIdentity)
  "#register_move_module_identity " str str : command

/-- One attribute or pragma instance: a head name applied to arguments. -/
def moveAttributeInstance := leading_parser
  Lean.Parser.ident >>
    Lean.Parser.many (Lean.Parser.categoryParser `moveAttrArg 0)

/-- A source attribute list, written before the leading keyword of a
`struct`, `enum`, or `fun` declaration inside a `move_module`. -/
def moveAttributes := leading_parser
  "@[" >>
    Lean.Parser.withoutPosition
      (Lean.Parser.sepBy1 moveAttributeInstance ", ") >>
    "]"

/-- Internal command emitted by `move_module` to persist the data invariant a
type certifies, so source translation knows where a value is created. -/
scoped syntax (name := registerMoveInvariant)
  "#register_move_invariant " ident ident : command

/-- Internal command: persist that a resource family carries a global
invariant with the named body predicate. -/
scoped syntax (name := registerMoveGlobalInvariant)
  "#register_move_global_invariant " (&"update")? ident ident ident* : command

/-- Internal command emitted by `move_module` to persist the user-provided
source attributes of one declaration. -/
@[command_parser] def registerMoveAttributes := leading_parser
  "#register_move_attributes " >> Lean.Parser.ident >> moveAttributes

/-- A Move function declaration. This deliberately occupies command position,
where Lean's term-level `fun` keyword is otherwise unavailable. It is intended
for use inside `move_module`. The parser mirrors Lean's `def` signature and
body grammar, including equation clauses and recursion modifiers.

`public fun` declares a public Move function; Lean's `public` modifier is
consumed by `declModifiers` and translated to `@[move_public]`. -/
@[command_parser] def moveFunctionCommand := leading_parser
  Lean.Parser.Command.declModifiers false >>
  Lean.Parser.withPosition
    ("fun " >> Lean.Parser.Command.declId >>
      Lean.Parser.ppIndent
        (Lean.Parser.Command.optDeclSig >> Lean.Parser.Command.declVal) >>
      Lean.Parser.Command.optDefDeriving)

/-! `struct`, `enum`, `entry fun`, and `friend fun` are module-scoped
keywords: their leading words stay ordinary identifiers everywhere else, so
they are parsed by `move_module`'s item parser rather than registered as
global command tokens. The `move_module` expander rewrites them to the
attributed core declarations.

Each keyword may be preceded by a doc comment and a source attribute list
`@[name arg ..., ...]`. In this position `@[...]` always uses the Move
attribute grammar: well-known internal names desugar to the persistent tag
attributes, every other instance is recorded as user-provided metadata. -/

/-- The leading doc comment and source attribute list accepted before a
module-scoped keyword. Lean declaration attributes cannot parse the open
attribute-argument grammar, so the list is parsed before `declModifiers`. -/
def moveItemPrefix :=
  Lean.Parser.optional (Lean.Parser.Command.docComment) >>
  Lean.Parser.optional moveAttributes >>
  Lean.Parser.Command.declModifiers false

/-- `struct Name ... where` declares a Move structure; the expander rewrites
it to `@[move_struct] structure`. -/
def moveStructItem := leading_parser
  Lean.Parser.atomic
    (moveItemPrefix >> Lean.Parser.nonReservedSymbol "struct ") >>
  Lean.Parser.Command.declId >> Lean.Parser.Command.optDeclSig >>
  " where " >> Lean.Parser.Command.structFields >>
  Lean.Parser.Command.optDeriving

/-- `enum Name ... where` declares a Move enum; the expander rewrites it to
`@[move_enum] inductive`. -/
def moveEnumItem := leading_parser
  Lean.Parser.atomic
    (moveItemPrefix >> Lean.Parser.nonReservedSymbol "enum ") >>
  Lean.Parser.Command.declId >> Lean.Parser.Command.optDeclSig >>
  " where " >> Lean.Parser.many Lean.Parser.Command.ctor >>
  Lean.Parser.Command.optDeriving

/-- `entry fun` declares a public Move entry function. -/
def moveEntryFunItem := leading_parser
  Lean.Parser.atomic
    (moveItemPrefix >> Lean.Parser.nonReservedSymbol "entry " >> "fun ") >>
  Lean.Parser.Command.declId >>
  Lean.Parser.ppIndent
    (Lean.Parser.Command.optDeclSig >> Lean.Parser.Command.declVal) >>
  Lean.Parser.Command.optDefDeriving

/-- `friend fun` declares a Move function with `public(friend)` visibility. -/
def moveFriendFunItem := leading_parser
  Lean.Parser.atomic
    (moveItemPrefix >> Lean.Parser.nonReservedSymbol "friend " >> "fun ") >>
  Lean.Parser.Command.declId >>
  Lean.Parser.ppIndent
    (Lean.Parser.Command.optDeclSig >> Lean.Parser.Command.declVal) >>
  Lean.Parser.Command.optDefDeriving

/-- A plain `fun` carrying a source attribute list. Attribute-less `fun`
declarations keep parsing through the global `moveFunctionCommand`. -/
def moveFunItem := leading_parser
  Lean.Parser.atomic
    (Lean.Parser.optional (Lean.Parser.Command.docComment) >>
      moveAttributes >>
      Lean.Parser.Command.declModifiers false >> "fun ") >>
  Lean.Parser.Command.declId >>
  Lean.Parser.ppIndent
    (Lean.Parser.Command.optDeclSig >> Lean.Parser.Command.declVal) >>
  Lean.Parser.Command.optDefDeriving

/-- Defines a Lean namespace and exports it as a Move module with the same
name. `fun` declarations are private Move functions, while ordinary `def`s
remain Lean-only helpers. Compilation remains deferred until end of input.
Each child item establishes an indentation boundary; this is essential
because a subsequent command-level `fun` is also valid as the start of a Lean
term. -/
@[command_parser] def moveModuleCommand := leading_parser
  Lean.Parser.withPosition
    ("move_module " >> ident >> " where" >>
      Lean.Parser.many1 (Lean.Parser.ppLine >> Lean.Parser.checkColGt >>
        Lean.Parser.withPosition
          (moveStructItem <|> moveEnumItem <|> moveEntryFunItem <|>
            moveFriendFunItem <|> moveFunItem <|> Lean.Parser.commandParser)))

private partial def hasMoveDeclarationAttribute (stx : Syntax) : Bool :=
  if stx.isIdent then
    let name := stx.getId
    name == `move_fun || name == `move_public || name == `move_friend ||
      name == `move_entry
  else
    stx.getArgs.any hasMoveDeclarationAttribute

private partial def expandLeanerAttributeAliases (stx : Syntax) : Syntax :=
  if stx.isOfKind ``Lean.Parser.Attr.simple && stx[0].isIdent then
    if stx[0].getId == `entry then
      stx.setArg 0 (mkIdentFrom stx[0] `move_entry)
    else
      stx
  else
    stx.modifyArgs (·.map expandLeanerAttributeAliases)

private def normalizeRetainedBody (source : String) : String :=
  let indentation := if source.startsWith "do" then 2 else 4
  let removeIndent (line : String) :=
    if indentation == 2 && line.startsWith "  " then (line.drop 2).toString
    else if indentation == 4 && line.startsWith "    " then (line.drop 4).toString
    else line
  match source.splitOn "\n" with
  | [] => source
  | first :: rest => String.intercalate "\n" (first :: rest.map removeIndent)

/-- Move type parameters are always inhabited.  Lean needs the corresponding
instance for the opaque implementations of operations such as vector indexing
and abort, but source authors should not have to repeat this compiler detail
in every generic Move function signature. A bare implicit binder `{T}` is a
Move type parameter, so its `: Type` ascription may be omitted. -/
private def addMoveTypeInhabitants
    (signature : TSyntax ``Lean.Parser.Command.optDeclSig) : MacroM
      (TSyntax ``Lean.Parser.Command.optDeclSig) := do
  let (binders, _) := Lean.Elab.expandOptDeclSig signature.raw
  let inhabitedNames := binders.getArgs.foldl (init := #[]) fun names binder =>
    let binder : TSyntax ``Lean.Parser.Term.bracketedBinder := ⟨binder⟩
    match binder with
    | `(bracketedBinder| [Inhabited $name:ident]) => names.push name.getId
    | _ => names
  let mut expanded := #[]
  for binderRaw in binders.getArgs do
    let binder : TSyntax ``Lean.Parser.Term.bracketedBinder := ⟨binderRaw⟩
    let (binderRaw, typeNames) ← match binder with
      | `(bracketedBinder| {$names:ident* : Type}) => pure (binderRaw, names)
      | `(bracketedBinder| {$names:ident*}) => do
          let typed ← `(bracketedBinder| {$names:ident* : Type})
          pure (typed.raw, names)
      | `(bracketedBinder| ($names:ident* : Type)) => pure (binderRaw, names)
      | _ => pure (binderRaw, #[])
    expanded := expanded.push binderRaw
    for name in typeNames do
      unless inhabitedNames.contains name.getId do
        let instanceBinder ← `(bracketedBinder| [Inhabited $name])
        expanded := expanded.push instanceBinder.raw
  pure ⟨signature.raw.setArg 0 (mkNullNode expanded)⟩

/-- In `struct` and `enum` headers every binder is a Move type parameter, so
`: Type` may be omitted in both `(T)` and `{T}` binders. The ascription is
inserted into the parsed binder in place, preserving its source info. -/
private def normalizeTypeParameterBinders
    (signature : Syntax) : MacroM Syntax := do
  let typeTerm ← `(Type)
  let ascription := mkNullNode #[mkAtom ":", typeTerm.raw]
  let binders := signature[0].getArgs.map fun binderRaw =>
    let bare := (binderRaw.isOfKind ``Lean.Parser.Term.explicitBinder ||
        binderRaw.isOfKind ``Lean.Parser.Term.implicitBinder) &&
      binderRaw.getNumArgs > 2 && binderRaw[2].isNone &&
      binderRaw[1].getArgs.all (·.isIdent)
    if bare then binderRaw.setArg 2 ascription else binderRaw
  return signature.setArg 0 (mkNullNode binders)

/-- Prepend one Move declaration attribute to parsed modifiers. The attribute
identifier is deliberately unhygienic so later passes can recognize it by
name. -/
private def prependDeclarationAttribute (attributeName : Name)
    (modifiers : TSyntax ``Lean.Parser.Command.declModifiers) :
    MacroM (TSyntax ``Lean.Parser.Command.declModifiers) := do
  let attributeIdent : TSyntax `ident := mkIdent attributeName
  let added ← `(attr| $attributeIdent:ident)
  let attributes ← if modifiers.raw[1].isNone then
    `(attributes|@[$added:attr])
  else
    let existing : TSyntax ``Lean.Parser.Term.attributes :=
      ⟨modifiers.raw[1][0]⟩
    let `(attributes|@[$attrs,*]) := existing
      | Macro.throwErrorAt existing.raw "invalid declaration attributes"
    `(attributes|@[$added:attr, $attrs,*])
  pure ⟨modifiers.raw.setArg 1 (mkNullNode #[attributes.raw])⟩

/-- Whether parsed modifiers carry Lean's `public` visibility keyword, which
`fun` interprets as Move `public` visibility. -/
private def hasPublicModifier
    (modifiers : TSyntax ``Lean.Parser.Command.declModifiers) : Bool :=
  modifiers.raw[2].getArgs.any fun visibility =>
    visibility.isOfKind `Lean.Parser.Command.public

/-- Remove Lean's `public` visibility keyword from parsed modifiers. -/
private def dropPublicModifier
    (modifiers : TSyntax ``Lean.Parser.Command.declModifiers) :
    TSyntax ``Lean.Parser.Command.declModifiers :=
  ⟨modifiers.raw.setArg 2 (mkNullNode)⟩

macro_rules
  | `($modifiers:declModifiers fun $declName:declId
        $signature:optDeclSig $value:declVal) => do
      let signature ← addMoveTypeInhabitants signature
      let modifiers : TSyntax ``Lean.Parser.Command.declModifiers :=
        ⟨expandLeanerAttributeAliases modifiers.raw⟩
      let modifiers ← if hasPublicModifier modifiers then
        prependDeclarationAttribute `move_public (dropPublicModifier modifiers)
      else if hasMoveDeclarationAttribute modifiers.raw then
        pure modifiers
      else
        prependDeclarationAttribute `move_fun modifiers
      let modifiers ← match value with
        | `(declVal| := $body:term) =>
            let (_, type?) := Lean.Elab.expandOptDeclSig signature.raw
            match type? with
            | some resultType =>
                let resultType : TSyntax `term := ⟨resultType⟩
                let sourceBody := body.raw.reprint.getD body.raw.prettyPrint.pretty
                let prettyBody := normalizeRetainedBody sourceBody
                let encodedBody := Syntax.mkStrLit <|
                  "(\n  " ++ prettyBody.replace "\n" "\n  " ++ "\n)\n"
                let encodedBody : TSyntax `str := ⟨encodedBody⟩
                let sourceAttribute ← `(Lean.Parser.Term.attrInstance|
                  move_source ($resultType, $encodedBody))
                let existing : TSyntax ``Lean.Parser.Term.attributes :=
                  ⟨modifiers.raw[1][0]⟩
                let `(attributes|@[$attrs,*]) := existing
                  | Macro.throwErrorAt existing.raw "invalid declaration attributes"
                let attributes ← `(attributes|@[$sourceAttribute, $attrs,*])
                pure ⟨modifiers.raw.setArg 1 (mkNullNode #[attributes.raw])⟩
            | none => pure modifiers
        | _ => pure modifiers
      let value ← match value with
        | `(declVal| := $body:term) =>
            match body with
            | `(do $seq:doSeq) =>
                let (_, type?) := Lean.Elab.expandOptDeclSig signature.raw
                let isAction :=
                  match type? with
                  | some resultType =>
                      let identName :=
                        if resultType.isIdent then some resultType.getId
                        else if resultType.isOfKind ``Lean.Parser.Term.app &&
                            resultType.getNumArgs > 0 && resultType[0]!.isIdent then
                          some resultType[0]!.getId
                        else none
                      identName == some `Action || identName == some ``Move.Action
                  | none => false
                if isAction = true then
                  pure value
                else
                  `(declVal| := Id.run do $seq)
            | _ => pure value
        | _ => pure value
      `($modifiers:declModifiers def $declName:declId
        $signature:optDeclSig $value:declVal)

private partial def expandLeanerCommandAliases (stx : Syntax) : Syntax :=
  if stx.isOfKind ``Lean.Parser.Command.declaration then
    stx.setArg 0 (expandLeanerAttributeAliases stx[0])
  else
    stx.modifyArgs (·.map expandLeanerCommandAliases)

/-- Preserve calls to Lean-only helpers until Move lowering. Otherwise Lean
may inline a helper into a `fun` and silently erase the source boundary which
is supposed to distinguish proof code from deployable code. -/
private partial def preserveLeanHelperBoundaries (stx : Syntax) : MacroM Syntax := do
  if stx.isOfKind ``Lean.Parser.Command.declaration &&
      stx[1].isOfKind ``Lean.Parser.Command.definition &&
      !hasMoveDeclarationAttribute stx[0] then
    let modifiers := stx[0]
    let attributes ← if modifiers[1].isNone then
      `(attributes|@[noinline])
    else
      let existing : TSyntax ``Lean.Parser.Term.attributes := ⟨modifiers[1][0]⟩
      let `(attributes|@[$attrs,*]) := existing
        | Macro.throwErrorAt existing.raw "invalid declaration attributes"
      `(attributes|@[noinline, $attrs,*])
    return stx.setArg 0 (modifiers.setArg 1 (mkNullNode #[attributes.raw]))
  let args ← stx.getArgs.mapM preserveLeanHelperBoundaries
  return stx.setArgs args

/-- The well-known internal declaration attributes accepted by name in a
source attribute list, mapped to their tag-attribute spelling. -/
private def wellKnownAttribute? (name : Name) : Option Name :=
  match name with
  | `move_fun | `move_public | `move_friend | `move_entry
  | `move_struct | `move_enum | `move_native => some name
  | `entry => some `move_entry
  | _ => none

/-- Split a parsed source attribute list into well-known internal attribute
names and user-provided attribute instances. -/
private def splitAttributeInstances (attrs : Syntax) :
    MacroM (Array Name × Array Syntax) := do
  let some attrs := if attrs.isOfKind ``moveAttributes then some attrs
    else attrs.getOptional?
    | return (#[], #[])
  let mut wellKnown := #[]
  let mut user := #[]
  for instanceStx in attrs[1].getArgs do
    unless instanceStx.isOfKind ``moveAttributeInstance do continue
    match wellKnownAttribute? instanceStx[0].getId with
    | some name =>
        unless instanceStx[1].getArgs.isEmpty do
          Macro.throwErrorAt instanceStx
            s!"internal attribute `{instanceStx[0].getId}` takes no arguments"
        wellKnown := wellKnown.push name
    | none => user := user.push instanceStx
  return (wellKnown, user)

/-- Attach a hoisted leading doc comment to parsed modifiers. -/
private def applyDocComment (doc : Syntax)
    (modifiers : TSyntax ``Lean.Parser.Command.declModifiers) :
    MacroM (TSyntax ``Lean.Parser.Command.declModifiers) := do
  if doc.isNone then return modifiers
  unless modifiers.raw[0].isNone do
    Macro.throwErrorAt doc "duplicate doc comment"
  return ⟨modifiers.raw.setArg 0 doc⟩

/-- Prepend distinct Move declaration attributes to parsed modifiers. -/
private def prependDeclarationAttributes (names : Array Name)
    (modifiers : TSyntax ``Lean.Parser.Command.declModifiers) :
    MacroM (TSyntax ``Lean.Parser.Command.declModifiers) := do
  let mut modifiers := modifiers
  let mut seen : Array Name := #[]
  for name in names do
    unless seen.contains name do
      seen := seen.push name
      modifiers ← prependDeclarationAttribute name modifiers
  return modifiers

/-- The registration command persisting user-provided attributes of the
declaration named by `declId`. -/
private def buildAttributeRegistration (declId : Syntax)
    (user : Array Syntax) : Syntax :=
  let separated := user.foldl (init := #[]) fun result instanceStx =>
    if result.isEmpty then #[instanceStx]
    else result ++ #[mkAtom ", ", instanceStx]
  mkNode ``registerMoveAttributes #[
    mkAtom "#register_move_attributes ", declId[0],
    mkNode ``moveAttributes
      #[mkAtom "@[", mkNullNode separated, mkAtom "]"]]

private def withAttributeRegistration (declaration declId : Syntax)
    (user : Array Syntax) : Array Syntax :=
  if user.isEmpty then #[declaration]
  else #[declaration, buildAttributeRegistration declId user]

/-- The field names a `structFields` node declares, in order. -/
private def structFieldNames (fields : Syntax) : Array (TSyntax `ident) :=
  let args := if fields.getNumArgs == 1 && fields[0].isOfKind nullKind then
      fields[0].getArgs
    else
      fields.getArgs
  args.filterMap fun field =>
    if field.isOfKind ``Lean.Parser.Command.structSimpleBinder then
      some ⟨field[1]⟩
    else
      none

/-- The names a binder introduces, so a generated declaration can apply the
type it belongs to. -/
private def binderNames (binder : Syntax) : Array (TSyntax `ident) :=
  if binder.isOfKind ``Lean.Parser.Term.explicitBinder ||
      binder.isOfKind ``Lean.Parser.Term.implicitBinder ||
      binder.isOfKind ``Lean.Parser.Term.strictImplicitBinder then
    binder[1].getArgs.filterMap fun argument =>
      if argument.isIdent then some ⟨argument⟩ else none
  else
    #[]

/-- Add the certifying field: the invariant of this very value, defaulted to
a tactic so an ordinary literal carries no proof text. -/
private def appendInvariantField (item : Syntax) (fields : Syntax)
    (invariantName : TSyntax `ident) : MacroM Syntax := do
  let fieldNames := structFieldNames fields
  let assignments ← fieldNames.mapM fun field =>
    `(Lean.Parser.Term.structInstField| $field:ident := $field:ident)
  let value ← `({ $assignments,* : _ })
  let invariantField := mkIdentFrom item[7] `invariant
  let field ← `(Lean.Parser.Command.structSimpleBinder|
    $invariantField:ident : $invariantName $value := by move_invariant)
  -- `structFields` wraps its binders in a single null node.
  let existing := if fields.getNumArgs == 1 && fields[0].isOfKind nullKind then
      fields[0].getArgs
    else
      fields.getArgs
  let updated := mkNullNode (existing.push field.raw)
  return if fields.getNumArgs == 1 && fields[0].isOfKind nullKind then
      fields.setArg 0 updated
    else
      updated

/-- Attach a declared data invariant to the struct that carries it: a
proof-free twin used to state the condition, and the condition itself.  The
certified structure gains the proof as a field, so a value carries its
invariant and only creating one owes a proof. -/
private def certifiedStructDeclarations (item : Syntax)
    (conditions : Array Syntax) :
    MacroM (Array Syntax × Array Syntax × TSyntax `ident) := do
  let declId := item[4]
  let name : TSyntax `ident := ⟨declId[0]⟩
  let rawName := mkIdentFrom name (name.getId ++ `Raw)
  let invariantName := mkIdentFrom name (name.getId ++ `Invariant)
  let signature ← normalizeTypeParameterBinders item[5]
  -- The condition's parameters are implicit, so it applies to a value
  -- without repeating the type arguments.
  let binders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ←
    signature[0].getArgs.mapM fun binder =>
      if binder.isOfKind ``Lean.Parser.Term.explicitBinder then do
        let names : Array (TSyntax `ident) := binder[1].getArgs.map (⟨·⟩)
        let type : TSyntax `term := ⟨binder[2][1]⟩
        `(bracketedBinder| {$names:ident* : $type})
      else
        pure ⟨binder⟩
  let parameters := signature[0].getArgs.flatMap binderNames
  let this := mkIdentFrom item[7] `this
  let bound := conditions.map fun condition =>
    (⟨Move.Spec.bindInvariantValue this condition⟩ : TSyntax `term)
  let mut condition := bound[0]!
  for index in [1:bound.size] do
    condition ← `($condition ∧ $(bound[index]!))
  let rawStructure := mkNode ``Lean.Parser.Command.structure #[
    mkNode ``Lean.Parser.Command.structureTk #[mkAtomFrom item[3] "structure"],
    mkNode ``Lean.Parser.Command.declId #[rawName.raw, mkNullNode],
    signature, mkNullNode,
    mkNullNode #[mkAtom "where", mkNullNode, item[7]],
    mkNullNode]
  let rawDeclaration := mkNode ``Lean.Parser.Command.declaration
    #[mkNullNode, rawStructure]
  let rawType : TSyntax `term ← if parameters.isEmpty then
      pure ⟨rawName.raw⟩
    else
      `($rawName $(parameters.map fun p => (⟨p.raw⟩ : TSyntax `term))*)
  let invariantCommand ←
    `(@[move_invariant_norm] def $invariantName $binders*
        ($this : $rawType) : Prop := $condition)
  let inhabitedSignature ← addMoveTypeInhabitants ⟨signature⟩
  let inhabitedBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) :=
    inhabitedSignature.raw[0].getArgs.map (⟨·⟩)
  let certifiedType : TSyntax `term ← if parameters.isEmpty then
      pure ⟨name.raw⟩
    else
      `($name $(parameters.map fun p => (⟨p.raw⟩ : TSyntax `term))*)
  let fieldNames := structFieldNames item[7]
  let defaults ← fieldNames.mapM fun field =>
    `(Lean.Parser.Term.structInstField| $field:ident := default)
  let inhabitedCommand ←
    `(instance $inhabitedBinders:bracketedBinder* :
        Inhabited $certifiedType :=
          ⟨({ $defaults,* : $certifiedType })⟩)
  let registration ←
    `(#register_move_invariant $name $invariantName)
  -- The twin and the condition precede the type; its inhabitant follows it.
  return (#[rawDeclaration, invariantCommand.raw],
    #[inhabitedCommand.raw, registration.raw], invariantName)

/-- The binders of a constructor's signature, with the names they bind. -/
private def constructorBinders (ctor : Syntax) : Syntax × Nat := Id.run do
  for index in [0:ctor.getNumArgs] do
    if ctor[index].isOfKind ``Lean.Parser.Command.optDeclSig then
      return (ctor[index], index)
  return (mkNullNode, ctor.getNumArgs)

private def constructorName (ctor : Syntax) : Option (TSyntax `ident) := Id.run do
  for index in [0:ctor.getNumArgs] do
    if ctor[index].isIdent then return some ⟨ctor[index]⟩
  return none

/-- Attach a declared data invariant to the enum that carries it: a
proof-free twin states the condition (so it can `match this`), and every
constructor of the certified enum gains a trailing proof argument whose
default discharges the obligation at construction.  Patterns of a certified
enum bind that argument with a trailing `_`. -/
private def certifiedEnumDeclarations (item : Syntax)
    (conditions : Array Syntax) :
    MacroM (Array Syntax × Array Syntax × Syntax × TSyntax `ident) := do
  let declId := item[4]
  let name : TSyntax `ident := ⟨declId[0]⟩
  let rawName := mkIdentFrom name (name.getId ++ `Raw)
  let invariantName := mkIdentFrom name (name.getId ++ `Invariant)
  let signature ← normalizeTypeParameterBinders item[5]
  let binders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ←
    signature[0].getArgs.mapM fun binder =>
      if binder.isOfKind ``Lean.Parser.Term.explicitBinder then do
        let names : Array (TSyntax `ident) := binder[1].getArgs.map (⟨·⟩)
        let type : TSyntax `term := ⟨binder[2][1]⟩
        `(bracketedBinder| {$names:ident* : $type})
      else
        pure ⟨binder⟩
  let parameters := signature[0].getArgs.flatMap binderNames
  let this := mkIdentFrom item[7] `this
  let bound := conditions.map fun condition =>
    (⟨Move.Spec.bindInvariantValue this condition⟩ : TSyntax `term)
  let mut condition := bound[0]!
  for index in [1:bound.size] do
    condition ← `($condition ∧ $(bound[index]!))
  -- The twin: the same constructors, no proof, never compiled.
  let rawInductive := mkNode ``Lean.Parser.Command.inductive #[
    mkAtomFrom item[3] "inductive",
    mkNode ``Lean.Parser.Command.declId #[rawName.raw, mkNullNode],
    signature, mkNullNode #[mkAtom "where"], item[7], mkNullNode, mkNullNode]
  let rawDeclaration := mkNode ``Lean.Parser.Command.declaration
    #[mkNullNode, rawInductive]
  let rawType : TSyntax `term ← if parameters.isEmpty then
      pure ⟨rawName.raw⟩
    else
      `($rawName $(parameters.map fun p => (⟨p.raw⟩ : TSyntax `term))*)
  let invariantCommand ←
    `(@[move_invariant_norm] def $invariantName $binders*
        ($this : $rawType) : Prop := $condition)
  -- The certified constructors: each carries the proof of its own variant.
  let invariantField := mkIdentFrom item[7] `invariant
  let mut certifiedCtors : Array Syntax := #[]
  let mut firstCtor : Option (TSyntax `ident × Nat) := none
  for ctor in item[7].getArgs do
    let some ctorName := constructorName ctor
      | Macro.throwErrorAt ctor "unsupported enum constructor shape"
    let (sig, sigIndex) := constructorBinders ctor
    let ctorBinders := sig[0].getArgs
    let fieldNames := ctorBinders.flatMap binderNames
    let rawCtor := mkIdentFrom ctorName (rawName.getId ++ ctorName.getId)
    let rawValue : TSyntax `term ← if fieldNames.isEmpty then
        pure ⟨rawCtor.raw⟩
      else
        `($rawCtor $(fieldNames.map fun f => (⟨f.raw⟩ : TSyntax `term))*)
    let proofBinder ← `(bracketedBinder|
      ($invariantField:ident : $invariantName $rawValue := by move_invariant))
    let newSig := sig.setArg 0 (mkNullNode (ctorBinders.push proofBinder.raw))
    certifiedCtors := certifiedCtors.push (ctor.setArg sigIndex newSig)
    if firstCtor.isNone then firstCtor := some (ctorName, fieldNames.size)
  let certifiedCtorsNode := item[7].setArgs certifiedCtors
  -- An inhabitant: the first constructor on default fields, its proof
  -- discharged like any other creation.
  let inhabitedSignature ← addMoveTypeInhabitants ⟨signature⟩
  let inhabitedBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) :=
    inhabitedSignature.raw[0].getArgs.map (⟨·⟩)
  let certifiedType : TSyntax `term ← if parameters.isEmpty then
      pure ⟨name.raw⟩
    else
      `($name $(parameters.map fun p => (⟨p.raw⟩ : TSyntax `term))*)
  let some (firstName, arity) := firstCtor
    | Macro.throwErrorAt item "a Move enum must declare at least one variant"
  let firstFull := mkIdentFrom firstName (name.getId ++ firstName.getId)
  let defaults ← (List.range arity).toArray.mapM fun _ => `(default)
  let witness : TSyntax `term ← if arity == 0 then pure ⟨firstFull.raw⟩
    else `($firstFull $defaults*)
  let inhabitedCommand ←
    `(instance $inhabitedBinders:bracketedBinder* :
        Inhabited $certifiedType := ⟨($witness : $certifiedType)⟩)
  let registration ← `(#register_move_invariant $name $invariantName)
  return (#[rawDeclaration, invariantCommand.raw],
    #[inhabitedCommand.raw, registration.raw], certifiedCtorsNode, invariantName)

/-- Rewrite one module-scoped keyword item to its attributed core
declaration, followed by a registration command when the item carries
user-provided attributes. Ordinary commands pass through unchanged. -/
private def desugarModuleItem (invariants : Array (Name × Array Syntax))
    (stx : Syntax) : MacroM (Array Syntax) := do
  if stx.isOfKind ``Move.Spec.dataInvariantSpec then
    -- Consumed by the type it names.
    return #[]
  if stx.isOfKind ``Move.Spec.globalInvariantSpec then do
    -- `spec global where invariant (all a: P); …`: each clause becomes a state
    -- predicate `∀ a, guard → body` over `get`/`contains` of the families it
    -- names, registered under EACH of them so a write to any re-checks it.  A
    -- regular invariant is a `State → Prop`; an `update` invariant a relation
    -- `State → State → Prop` between the pre- and post-state.
    let clauses := #[stx[4]] ++ stx[5].getArgs.map (·[2])
    let mut commands : Array Syntax := #[]
    for (clause, index) in clauses.zipIdx do
      let (isUpdate, families, addr, atBody) ← Move.Spec.elabGlobalInvariantClause clause
      let stateType := mkIdentFrom clause `_moveSpecS
      let state := mkIdentFrom clause `_moveSpecState
      let pre := mkIdentFrom clause `_moveSpecPre
      let post := mkIdentFrom clause `_moveSpecPost
      -- `{S} [ResourceStore S R]*` shared by every generated declaration.
      let mut storeBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) :=
        #[← `(bracketedBinder| {$stateType : Type})]
      for family in families do
        storeBinders := storeBinders.push
          (← `(bracketedBinder| [Move.Semantics.ResourceStore $stateType $family]))
      -- The state binders and arguments the invariant quantifies over.
      let stateBinder : TSyntax ``Lean.Parser.Term.bracketedBinder ←
        if isUpdate then `(bracketedBinder| ($pre $post : $stateType))
        else `(bracketedBinder| ($state : $stateType))
      let stateArgs : Array (TSyntax `term) :=
        if isUpdate then #[pre, post] else #[state]
      let suffix := if index == 0 then "" else s!"_{index}"
      let base := if isUpdate then "GlobalUpdate" else "GlobalInvariant"
      let firstFamily := families[0]!
      let name := mkIdentFrom firstFamily
        (Name.mkSimple s!"{base}_{firstFamily.getId.getString!}{suffix}")
      let atName := mkIdentFrom firstFamily
        (Name.mkSimple s!"{base}_{firstFamily.getId.getString!}{suffix}_at")
      -- Per-address predicate `guard → body`, and the invariant as its `∀`.
      -- The invariant is `irreducible` so the shared finisher never expands the
      -- quantifier (which would make `grind`/`simp` explode); the only way to
      -- discharge it is the `@[grind]` reestablishment lemmas below, which open
      -- it explicitly.
      let atCommand ← `(@[grind] def $atName
        $storeBinders* $stateBinder ($addr : Move.Address) : Prop := $atBody)
      let invCommand ← `(@[irreducible] def $name $storeBinders* $stateBinder : Prop :=
        ∀ $addr : Move.Address, $atName $stateArgs* $addr)
      commands := commands ++ #[atCommand.raw, invCommand.raw]
      -- A reestablishment lemma per family and per write shape (insert/erase):
      -- the invariant survives a change at `w` given its address-`w`
      -- obligation, framing every other address from the entry certificate
      -- (regular) or reflexivity (update).
      let s := mkIdentFrom clause `_moveSpecReS
      let w := mkIdentFrom clause `_moveSpecReW
      let v := mkIdentFrom clause `_moveSpecReV
      let a := mkIdentFrom clause `_moveSpecReA
      let h := mkIdentFrom clause `_moveSpecReH
      let hyp := mkIdentFrom clause `_moveSpecReHyp
      let framed := mkIdentFrom clause `_moveSpecReFramed
      let changed := mkIdentFrom clause `_moveSpecReChanged
      for family in families do
        -- Independence of every *other* named family from the written one, so
        -- their stored values frame across this write.
        let mut indepBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) := #[]
        for other in families do
          unless other.getId == family.getId do
            indepBinders := indepBinders.push
              (← `(bracketedBinder|
                [Move.Semantics.IndependentResourceStores $stateType $family $other]))
        for isErase in #[false, true] do
          let verb := if isErase then "erase" else "insert"
          let lemmaName := mkIdentFrom family (Name.mkSimple
            s!"{base}_{firstFamily.getId.getString!}{suffix}_{verb}_{family.getId.getString!}")
          let changedState : TSyntax `term ←
            if isErase then
              `(Move.Semantics.ResourceStore.erase (State := $stateType)
                (Value := $family) $s $w)
            else
              `(Move.Semantics.ResourceStore.insert (State := $stateType)
                (Value := $family) $s $w $v)
          let frameLemma ← if isErase then
              `(Lean.Parser.Tactic.simpLemma|
                Move.Semantics.ResourceStore.lookup_erase_other _ _ _ $h)
            else
              `(Lean.Parser.Tactic.simpLemma|
                Move.Semantics.ResourceStore.lookup_insert_other _ _ _ _ $h)
          let indepLemma ← if isErase then
              `(Lean.Parser.Tactic.simpLemma|
                Move.Semantics.IndependentResourceStores.lookup_right_after_left_erase)
            else
              `(Lean.Parser.Tactic.simpLemma|
                Move.Semantics.IndependentResourceStores.lookup_right_after_left_insert)
          let valueBinders : Array (TSyntax ``Lean.Parser.Term.bracketedBinder) ←
            if isErase then pure #[]
            else do pure #[← `(bracketedBinder| ($v : $family))]
          -- Regular: the entry invariant frames untouched addresses and is
          -- threaded into the changed address's obligation (`K_at s w →
          -- K_at (post) w`).  Update: reflexivity frames, and the changed
          -- obligation is the raw relation at `w` (no entry).
          let hypType : TSyntax `term ←
            if isUpdate then `(∀ $a : Move.Address, $atName $s $s $a)
            else `($name $s)
          let changedType : TSyntax `term ←
            if isUpdate then `($atName $s $changedState $w)
            else `($atName $s $w → $atName $changedState $w)
          let changedExact : TSyntax `term ←
            if isUpdate then `($changed) else `($changed ($hyp $w))
          let conclusionArgs : Array (TSyntax `term) :=
            if isUpdate then #[s, changedState] else #[changedState]
          let framedFrom : TSyntax `term ← `($hyp $a)
          -- Unfold the (irreducible) invariant to expose its `∀`.  Regular:
          -- also in the entry hypothesis so it can be instantiated at an
          -- address.  Update: the hypothesis is already the reflexivity `∀`.
          let unfoldTac : TSyntax `tactic ←
            if isUpdate then `(tactic| unfold $name:ident)
            else `(tactic| unfold $name:ident at $hyp:ident ⊢)
          let lemmaCommand ← `(@[grind ←] theorem $lemmaName
              $storeBinders* $indepBinders* ($s : $stateType) ($w : Move.Address)
              $valueBinders* ($hyp : $hypType)
              ($changed : $changedType) :
              $name $conclusionArgs* := by
            $unfoldTac:tactic
            intro $a:ident
            by_cases $h:ident : $a = $w <;>
              first
                | (subst $a:ident; exact $changedExact)
                | (have $framed:ident := $framedFrom;
                   simp only [$atName:ident, Move.Semantics.ResourceStore.contains,
                     Move.Semantics.ResourceStore.get, $frameLemma:simpLemma,
                     $indepLemma:simpLemma] at $framed:ident ⊢;
                   exact $framed))
          commands := commands.push lemmaCommand.raw
      for family in families do
        let registration ← if isUpdate then
            `(#register_move_global_invariant update $family $name $families*)
          else
            `(#register_move_global_invariant $family $name $families*)
        commands := commands.push registration.raw
    return commands
  if stx.isOfKind ``moveStructItem then
    let (wellKnown, user) ← splitAttributeInstances stx[1]
    let modifiers ← prependDeclarationAttributes (#[`move_struct] ++ wellKnown)
      (← applyDocComment stx[0] ⟨stx[2]⟩)
    let signature ← normalizeTypeParameterBinders stx[5]
    let declared := stx[4][0].getId
    let invariant? := invariants.find? (·.1 == declared) |>.map (·.2)
    let mut preface : Array Syntax := #[]
    let mut postface : Array Syntax := #[]
    let mut fields := stx[7]
    if let some conditions := invariant? then
      let (declarations, inhabitant, invariantName) ←
        certifiedStructDeclarations stx conditions
      preface := declarations
      postface := inhabitant
      fields ← appendInvariantField stx fields invariantName
    let structureNode := mkNode ``Lean.Parser.Command.structure #[
      mkNode ``Lean.Parser.Command.structureTk #[mkAtomFrom stx[3] "structure"],
      stx[4], signature,
      mkNullNode,
      mkNullNode #[mkAtom "where", mkNullNode, fields],
      stx[8]]
    let declaration := mkNode ``Lean.Parser.Command.declaration
      #[modifiers.raw, structureNode]
    return preface ++ withAttributeRegistration declaration stx[4] user ++
      postface
  if stx.isOfKind ``moveEnumItem then
    let (wellKnown, user) ← splitAttributeInstances stx[1]
    let modifiers ← prependDeclarationAttributes (#[`move_enum] ++ wellKnown)
      (← applyDocComment stx[0] ⟨stx[2]⟩)
    let signature ← normalizeTypeParameterBinders stx[5]
    let declared := stx[4][0].getId
    let invariant? := invariants.find? (·.1 == declared) |>.map (·.2)
    let mut preface : Array Syntax := #[]
    let mut postface : Array Syntax := #[]
    let mut ctors := stx[7]
    if let some conditions := invariant? then
      let (declarations, inhabitant, certifiedCtors, _) ←
        certifiedEnumDeclarations stx conditions
      preface := declarations
      postface := inhabitant
      ctors := certifiedCtors
    let inductiveNode := mkNode ``Lean.Parser.Command.inductive #[
      mkAtomFrom stx[3] "inductive", stx[4], signature,
      mkNullNode #[mkAtom "where"],
      ctors,
      mkNullNode,
      stx[8]]
    let declaration := mkNode ``Lean.Parser.Command.declaration
      #[modifiers.raw, inductiveNode]
    return preface ++ withAttributeRegistration declaration stx[4] user ++
      postface
  if stx.isOfKind ``moveEntryFunItem || stx.isOfKind ``moveFriendFunItem then
    let attributeName :=
      if stx.isOfKind ``moveEntryFunItem then `move_entry else `move_friend
    let (wellKnown, user) ← splitAttributeInstances stx[1]
    let modifiers ← prependDeclarationAttributes (#[attributeName] ++ wellKnown)
      (← applyDocComment stx[0] ⟨stx[2]⟩)
    let declaration := mkNode ``moveFunctionCommand
      #[modifiers.raw, mkAtomFrom stx[4] "fun ", stx[5], stx[6], stx[7], stx[8]]
    return withAttributeRegistration declaration stx[5] user
  if stx.isOfKind ``moveFunItem then
    let (wellKnown, user) ← splitAttributeInstances stx[1]
    let modifiers ← prependDeclarationAttributes wellKnown
      (← applyDocComment stx[0] ⟨stx[2]⟩)
    let declaration := mkNode ``moveFunctionCommand
      #[modifiers.raw, mkAtomFrom stx[3] "fun ", stx[4], stx[5], stx[6], stx[7]]
    return withAttributeRegistration declaration stx[4] user
  return #[stx]

@[macro moveModuleCommand] def expandMoveModuleCommand : Macro := fun stx => do
  unless stx.isOfKind ``moveModuleCommand do Macro.throwUnsupported
  let moduleName : TSyntax `ident := ⟨stx[1]⟩
  let exportName : TSyntax `str := ⟨Syntax.mkStrLit moduleName.getId.toString⟩
  let exportCommand ← `(#export_leaner $exportName)
  let namespaceCommand ← `(namespace $moduleName)
  let address : TSyntax `str := ⟨Syntax.mkStrLit "0x0"⟩
  let identityCommand ← `(#register_move_module_identity $address $exportName)
  let openLeanerCommand ← `(open Move)
  let openLeanerScopeCommand ← `(open scoped Move)
  let endCommand ← `(end $moduleName)
  let invariants := stx[3].getArgs.filterMap fun item =>
    if item.isOfKind ``Move.Spec.dataInvariantSpec then
      let extras := item[6].getArgs.map fun clause => clause[2]
      some (item[1].getId, #[item[5]] ++ extras)
    else
      none
  let body ← stx[3].getArgs.foldlM (init := #[]) fun result item => do
    (← desugarModuleItem invariants item).foldlM (init := result) fun result command => do
      let command := expandLeanerCommandAliases command
      return result.push (← preserveLeanHelperBoundaries command)
  return mkNullNode <|
    #[namespaceCommand.raw, identityCommand.raw, exportCommand.raw,
      openLeanerCommand.raw, openLeanerScopeCommand.raw] ++
      body ++ #[endCommand.raw]

@[command_elab registerMoveModuleIdentity]
def elabRegisterMoveModuleIdentity : CommandElab := fun stx => do
  let some address := stx[1].isStrLit?
    | throwErrorAt stx[1] "expected a module address string"
  let some name := stx[2].isStrLit?
    | throwErrorAt stx[2] "expected a module name string"
  let leanNamespace ← getCurrNamespace
  modifyEnv fun env => Move.registerModuleNamespace env leanNamespace { address, name }

@[command_elab registerMoveInvariant]
def elabRegisterMoveInvariant : CommandElab := fun stx => do
  let typeName ← resolveGlobalConstNoOverload stx[1]
  let invariantName ← resolveGlobalConstNoOverload stx[2]
  modifyEnv fun env => Move.registerDataInvariant env typeName invariantName

@[command_elab registerMoveGlobalInvariant]
def elabRegisterMoveGlobalInvariant : CommandElab := fun stx => do
  let isUpdate := !stx[1].getArgs.isEmpty
  let family ← resolveGlobalConstNoOverload stx[2]
  let bodyName ← resolveGlobalConstNoOverload stx[3]
  let mentioned ← stx[4].getArgs.toList.mapM resolveGlobalConstNoOverload
  modifyEnv fun env =>
    Move.registerGlobalInvariant env family isUpdate bodyName mentioned

@[command_elab registerMoveAttributes]
def elabRegisterMoveAttributes : CommandElab := fun stx => do
  let name := stx[1].getId
  let name ← if (`_root_).isPrefixOf name then
      pure (name.replacePrefix `_root_ .anonymous)
    else
      pure ((← getCurrNamespace) ++ name)
  let mut attributes := #[]
  for instanceStx in stx[2][1].getArgs do
    unless instanceStx.isOfKind ``moveAttributeInstance do continue
    match Move.decodeAttributeInstance instanceStx with
    | .ok decoded => attributes := attributes.push decoded
    | .error message => throwErrorAt instanceStx message
  modifyEnv fun env =>
    Move.registerUserAttributes env name attributes.toList

private structure PendingExport where
  marker : Syntax
  ns : Name
  deriving Inhabited

private initialize pendingExportExt : EnvExtension (Option PendingExport) ←
  registerEnvExtension (pure none)

private unsafe def elabExportLeanerXIRUnsafe (moduleTerm pathTerm : Syntax) :
    CommandElabM Unit := do
  let module ← liftTermElabM do
    evalTerm MModule (mkConst ``MModule) moduleTerm
  let some path := pathTerm.isStrLit?
    | throwErrorAt pathTerm "expected an output path string"
  let encoded ← match module.encodeJson with
    | .ok encoded => pure encoded
    | .error message => throwErrorAt moduleTerm message
  IO.FS.writeFile path encoded
  logInfo s!"wrote Leaner XIR to {path}"

@[implemented_by elabExportLeanerXIRUnsafe]
private opaque elabExportLeanerXIR (moduleTerm pathTerm : Syntax) :
    CommandElabM Unit

@[command_elab exportLeanerXIR]
def elabExportLeanerXIRCommand : CommandElab := fun stx =>
  elabExportLeanerXIR stx[1] stx[3]

private unsafe def elabEmitLeanerXIRUnsafe (moduleTerm : Syntax) :
    CommandElabM Unit := do
  let module ← liftTermElabM do
    evalTerm MModule (mkConst ``MModule) moduleTerm
  let encoded ← match module.encodeJson with
    | .ok encoded => pure encoded
    | .error message => throwErrorAt moduleTerm message
  if let some path ← IO.getEnv "LEANER_XIR_OUTPUT" then
    if ← (System.FilePath.mk path).pathExists then
      throwErrorAt moduleTerm "a `.lean` compiler input may contain only one Leaner export directive"
    IO.FS.writeFile path encoded
    logInfo s!"wrote Leaner XIR to {path}"

@[implemented_by elabEmitLeanerXIRUnsafe]
private opaque elabEmitLeanerXIR (moduleTerm : Syntax) : CommandElabM Unit

@[command_elab emitLeanerXIR]
def elabEmitLeanerXIRCommand : CommandElab := fun stx =>
  elabEmitLeanerXIR stx[1]

private unsafe def elabExportLeanerUnsafe (stx : Syntax) : CommandElabM Unit := do
  let moduleTerm ← liftTermElabM do
    let moduleName : TSyntax `str := ⟨stx[1]⟩
    if stx[2].isNone then
      `(move_module% $moduleName)
    else
      let selection := stx[2]
      let structIdents : Array (TSyntax `ident) :=
        (selection[2].getArgs.filter (·.isIdent)).map (⟨·⟩)
      let functionIdents : Array (TSyntax `ident) :=
        (selection[6].getArgs.filter (·.isIdent)).map (⟨·⟩)
      `(move_module% $moduleName structs [$[$structIdents],*]
          functions [$[$functionIdents],*])
  elabEmitLeanerXIRUnsafe moduleTerm

@[implemented_by elabExportLeanerUnsafe]
private opaque elabExportLeaner (stx : Syntax) : CommandElabM Unit

@[command_elab exportLeaner]
def elabExportLeanerCommand : CommandElab := fun stx => do
  let env ← getEnv
  if let some pending := pendingExportExt.getState env then
    throwErrorAt stx m!"only one Leaner export directive is allowed; the first is at {pending.marker.getPos?}"
  let ns ← getCurrNamespace
  modifyEnv fun env => pendingExportExt.setState env (some { marker := stx, ns })

@[command_elab Lean.Parser.Command.eoi]
def elabPendingLeanerExport : CommandElab := fun _ => do
  -- This elaborator becomes visible at the end of this defining module, before
  -- its environment-extension initializer can be evaluated.
  if (← getMainModule) == `Move.Compiler.Export then
    return
  let env ← getEnv
  if let some pending := pendingExportExt.getState env then
    modifyEnv fun env => pendingExportExt.setState env none
    Lean.Elab.Command.withNamespace pending.ns <| elabExportLeaner pending.marker

end Move.Compiler
