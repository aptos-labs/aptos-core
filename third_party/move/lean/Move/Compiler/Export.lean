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

/-- Defines a Lean namespace and exports it as a Move module with the same
name. `fun` declarations are private Move functions, while ordinary `def`s
remain Lean-only helpers. `@[entry]` is the block-scoped spelling of
`@[move_entry]`. Compilation remains deferred until end of input. Each child
command establishes an indentation boundary; this is essential because a
subsequent command-level `fun` is also valid as the start of a Lean term. -/
@[command_parser] def moveModuleCommand := leading_parser
  Lean.Parser.withPosition
    ("move_module " >> ident >> " where" >>
      Lean.Parser.many1 (Lean.Parser.ppLine >> Lean.Parser.checkColGt >>
        Lean.Parser.withPosition Lean.Parser.commandParser))

/-- A Move function declaration. This deliberately occupies command position,
where Lean's term-level `fun` keyword is otherwise unavailable. It is intended
for use inside `move_module`. The parser mirrors Lean's `def` signature and
body grammar, including equation clauses and recursion modifiers. -/
@[command_parser] def moveFunctionCommand := leading_parser
  Lean.Parser.Command.declModifiers false >>
  Lean.Parser.withPosition
    ("fun " >> Lean.Parser.Command.declId >>
      Lean.Parser.ppIndent
        (Lean.Parser.Command.optDeclSig >> Lean.Parser.Command.declVal) >>
      Lean.Parser.Command.optDefDeriving)

private partial def hasMoveDeclarationAttribute (stx : Syntax) : Bool :=
  if stx.isIdent then
    let name := stx.getId
    name == `move_fun || name == `move_public || name == `move_entry
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
in every generic Move function signature. -/
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
    expanded := expanded.push binderRaw
    let binder : TSyntax ``Lean.Parser.Term.bracketedBinder := ⟨binderRaw⟩
    let typeNames : Array (TSyntax `ident) ← match binder with
      | `(bracketedBinder| {$names:ident* : Type}) => pure names
      | `(bracketedBinder| ($names:ident* : Type)) => pure names
      | _ => pure #[]
    for name in typeNames do
      unless inhabitedNames.contains name.getId do
        let instanceBinder ← `(bracketedBinder| [Inhabited $name])
        expanded := expanded.push instanceBinder.raw
  pure ⟨signature.raw.setArg 0 (mkNullNode expanded)⟩

macro_rules
  | `($modifiers:declModifiers fun $declName:declId
        $signature:optDeclSig $value:declVal) => do
      let signature ← addMoveTypeInhabitants signature
      let modifiers : TSyntax ``Lean.Parser.Command.declModifiers :=
        ⟨expandLeanerAttributeAliases modifiers.raw⟩
      let modifiers ← if hasMoveDeclarationAttribute modifiers.raw then
        pure modifiers
      else do
        let attributes ← if modifiers.raw[1].isNone then
          `(attributes|@[move_fun])
        else
          let existing : TSyntax ``Lean.Parser.Term.attributes :=
            ⟨modifiers.raw[1][0]⟩
          let `(attributes|@[$attrs,*]) := existing
            | Macro.throwErrorAt existing.raw "invalid declaration attributes"
          `(attributes|@[move_fun, $attrs,*])
        pure ⟨modifiers.raw.setArg 1 (mkNullNode #[attributes.raw])⟩
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

macro_rules
  | `(move_module $moduleName:ident where $commands:command*) => do
      let exportName : TSyntax `str := ⟨Syntax.mkStrLit moduleName.getId.toString⟩
      let exportCommand ← `(#export_leaner $exportName)
      let namespaceCommand ← `(namespace $moduleName)
      let address : TSyntax `str := ⟨Syntax.mkStrLit "0x0"⟩
      let identityCommand ← `(#register_move_module_identity $address $exportName)
      let openLeanerCommand ← `(open Move)
      let openLeanerScopeCommand ← `(open scoped Move)
      let endCommand ← `(end $moduleName)
      let body ← commands.foldlM (init := #[]) fun result command => do
        let command := expandLeanerCommandAliases command.raw
        return result.push (← preserveLeanHelperBoundaries command)
      return mkNullNode <|
        #[namespaceCommand, identityCommand, exportCommand, openLeanerCommand,
          openLeanerScopeCommand] ++
          body ++ #[endCommand]

@[command_elab registerMoveModuleIdentity]
def elabRegisterMoveModuleIdentity : CommandElab := fun stx => do
  let some address := stx[1].isStrLit?
    | throwErrorAt stx[1] "expected a module address string"
  let some name := stx[2].isStrLit?
    | throwErrorAt stx[2] "expected a module name string"
  let leanNamespace ← getCurrNamespace
  modifyEnv fun env => Move.registerModuleNamespace env leanNamespace { address, name }

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
